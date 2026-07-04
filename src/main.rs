use vani::ast::Type;
use vani::backend::Backend;
use vani::backend_c::CBackend;
use vani::backend_llvm::LlvmBackend;
use vani::ir::{TypedExpr, TypedExprKind, TypedPrintItem, TypedProgram, TypedStmt};
use vani::ssa::lower_program;
use vani::ssa_backend_c;
use vani::ssa_backend_llvm;

/// Module-wide gate: returns false if any function uses a
/// feature the SSA backends don't yet cover safely. Avoids
/// emitting broken IR that would silently produce wrong
/// output (the runtime error would only surface in tests).
/// Per-backend `extra_reject` lets callers add backend-
/// specific exclusions on top of the common set (e.g.,
/// SSA-LLVM still rejects parallel-for and tasks; SSA-C
/// now handles parallel-for so it sets `false` here).
fn ssa_path_supports(
    ir: &TypedProgram,
    extra_reject: impl Fn(&TypedStmt) -> bool,
) -> bool {
    for f in &ir.functions {
        for param in &f.params {
            if !ssa_type_supported(&param.ty) {
                return false;
            }
        }
        if !ssa_type_supported(&f.return_type) {
            return false;
        }
        if !stmts_ssa_supported(&f.body, &extra_reject) {
            return false;
        }
    }
    true
}

/// SSA-LLVM handles `parallel for` (full reduction op
/// table) and `task`/`join` (single-block bodies via
/// pthread_create/CreateThread outlining). Multi-block
/// task bodies and other unsupported shapes surface
/// `EmitError` from inside the SSA-LLVM emit → tree-LLVM
/// fallback.
fn ssa_llvm_extra_reject(stmt: &TypedStmt) -> bool {
    // Closure #212: `Vec<Atomic<T>>` / `Vec<Channel<T,N>>`
    // route through SSA-LLVM's vec literal emit which
    // expects the element to be a value-shaped LLVM type
    // (i32, i64, struct, …). SSA-LLVM represents Atomic
    // as the alloca *pointer* (so subsequent `&counter`
    // references reuse the same address), and Channel
    // similarly indirects through the struct. Storing a
    // pointer-shaped SSA value into an `i32` Vec slot
    // emits `store i32 %ptr, …` which fails the LLVM IR
    // verifier with a type mismatch. Tree-LLVM doesn't
    // have this issue (it goes through a different vec
    // emit path) — gate Vec<Atomic|Channel> out of SSA-
    // LLVM so it falls back to tree-LLVM. Also gates any
    // outer Vec containing Atomic/Channel at any nesting
    // depth.
    stmt_uses_vec_of_atomic_or_channel(stmt)
}

fn ty_contains_vec_of_atomic_or_channel(ty: &Type) -> bool {
    match ty {
        Type::Vec(inner) => matches!(
            &**inner,
            Type::Atomic(_) | Type::Channel(_, _)
        ) || ty_contains_vec_of_atomic_or_channel(inner),
        Type::Array { element, .. } => ty_contains_vec_of_atomic_or_channel(element),
        Type::Ref(inner) | Type::RefMut(inner) => {
            ty_contains_vec_of_atomic_or_channel(inner)
        }
        Type::Tuple(elements) => elements
            .iter()
            .any(ty_contains_vec_of_atomic_or_channel),
        Type::FnPtr(params, ret) => {
            params.iter().any(ty_contains_vec_of_atomic_or_channel)
                || ty_contains_vec_of_atomic_or_channel(ret)
        }
        _ => false,
    }
}

fn expr_uses_vec_of_atomic_or_channel(expr: &vani::ir::TypedExpr) -> bool {
    if ty_contains_vec_of_atomic_or_channel(&expr.ty) {
        return true;
    }
    use vani::ir::TypedExprKind as E;
    match &expr.kind {
        E::Unary { expr, .. } | E::Cast { expr, .. } | E::Len { array: expr, .. } => {
            expr_uses_vec_of_atomic_or_channel(expr)
        }
        E::Binary { left, right, .. } => {
            expr_uses_vec_of_atomic_or_channel(left)
                || expr_uses_vec_of_atomic_or_channel(right)
        }
        E::Call { args, .. }
        | E::ArrayLit { elements: args } => {
            args.iter().any(expr_uses_vec_of_atomic_or_channel)
        }
        E::CallIndirect { callee, args } => {
            expr_uses_vec_of_atomic_or_channel(callee)
                || args.iter().any(expr_uses_vec_of_atomic_or_channel)
        }
        E::Index { array, index, .. } => {
            expr_uses_vec_of_atomic_or_channel(array)
                || expr_uses_vec_of_atomic_or_channel(index)
        }
        E::Tuple { elements } => {
            elements.iter().any(expr_uses_vec_of_atomic_or_channel)
        }
        E::TupleAccess { tuple, .. } => expr_uses_vec_of_atomic_or_channel(tuple),
        E::StructLit { fields, .. } => fields
            .iter()
            .any(|(_, e)| expr_uses_vec_of_atomic_or_channel(e)),
        E::FieldAccess { object, .. } => expr_uses_vec_of_atomic_or_channel(object),
        E::EnumVariantWithPayload { payload, .. } => {
            expr_uses_vec_of_atomic_or_channel(payload)
        }
        E::IfExpr { cond, then_value, else_value } => {
            expr_uses_vec_of_atomic_or_channel(cond)
                || expr_uses_vec_of_atomic_or_channel(then_value)
                || expr_uses_vec_of_atomic_or_channel(else_value)
        }
        E::Match { scrutinee, arms } => {
            expr_uses_vec_of_atomic_or_channel(scrutinee)
                || arms.iter().any(|arm| expr_uses_vec_of_atomic_or_channel(&arm.body))
        }
        E::Block { stmts, tail } => {
            stmts.iter().any(stmt_uses_vec_of_atomic_or_channel)
                || expr_uses_vec_of_atomic_or_channel(tail)
        }
        _ => false,
    }
}

fn stmt_uses_vec_of_atomic_or_channel(stmt: &TypedStmt) -> bool {
    match stmt {
        TypedStmt::Let { ty, expr, .. } | TypedStmt::Reassign { ty, expr, .. } => {
            ty_contains_vec_of_atomic_or_channel(ty)
                || expr_uses_vec_of_atomic_or_channel(expr)
        }
        TypedStmt::Drop { ty, .. } => ty_contains_vec_of_atomic_or_channel(ty),
        TypedStmt::Discard { expr }
        | TypedStmt::Return { expr }
        | TypedStmt::Assert { expr, .. }
        | TypedStmt::Prove { expr } => expr_uses_vec_of_atomic_or_channel(expr),
        TypedStmt::Print { items } => items.iter().any(|i| match i {
            TypedPrintItem::Expr(e) => expr_uses_vec_of_atomic_or_channel(e),
            TypedPrintItem::Str(_) => false,
        }),
        TypedStmt::If { cond, then_body, else_body } => {
            expr_uses_vec_of_atomic_or_channel(cond)
                || then_body.iter().any(stmt_uses_vec_of_atomic_or_channel)
                || else_body.iter().any(stmt_uses_vec_of_atomic_or_channel)
        }
        TypedStmt::While { cond, body, .. } => {
            expr_uses_vec_of_atomic_or_channel(cond)
                || body.iter().any(stmt_uses_vec_of_atomic_or_channel)
        }
        TypedStmt::For { start, end, body, .. } => {
            expr_uses_vec_of_atomic_or_channel(start)
                || expr_uses_vec_of_atomic_or_channel(end)
                || body.iter().any(stmt_uses_vec_of_atomic_or_channel)
        }
        TypedStmt::ForIter { body, element_ty, collection_ty, .. } => {
            ty_contains_vec_of_atomic_or_channel(element_ty)
                || ty_contains_vec_of_atomic_or_channel(collection_ty)
                || body.iter().any(stmt_uses_vec_of_atomic_or_channel)
        }
        TypedStmt::IndexAssign { index, value, .. } => {
            expr_uses_vec_of_atomic_or_channel(index)
                || expr_uses_vec_of_atomic_or_channel(value)
        }
        TypedStmt::FieldAssign { object, value, .. } => {
            expr_uses_vec_of_atomic_or_channel(object)
                || expr_uses_vec_of_atomic_or_channel(value)
        }
        TypedStmt::TaskSpawn { body, .. } => {
            body.iter().any(stmt_uses_vec_of_atomic_or_channel)
        }
        _ => false,
    }
}

/// SSA-C now handles both `parallel for` (via OpenMP
/// pragmas + reduction clauses) and `task`/`join` (via the
/// `intent_thread_create`/`intent_thread_join` runtime
/// wrappers + outlined `static void* intent_task_<N>(void*)`
/// helpers). Multi-block task bodies and non-canonical
/// parallel-for carry shapes still surface `EmitError` →
/// tree-C fallback.
fn ssa_c_extra_reject(_stmt: &TypedStmt) -> bool {
    false
}

fn ssa_type_supported(ty: &Type) -> bool {
    // Every concurrency primitive now flows through SSA
    // (Atomic + Mutex/Guard + Channel) on both SSA-C and
    // SSA-LLVM. Anything an SSA backend can't yet handle
    // surfaces `EmitError` from inside its emit and falls
    // back per-backend in `emit_c_via_ssa` /
    // `emit_llvm_via_ssa`.
    //
    // Exception (closure #239): `[T; N]` in return position
    // routes through tree-LLVM. SSA-LLVM's array-return emit
    // returns a pointer to a stack-alloca'd array (the
    // pointer dangles after the fn returns); tree-C's
    // struct-wrap also lives in tree-side emit. Fix by
    // gating away from SSA when an array return appears
    // anywhere in the program.
    if matches!(ty, Type::Array { .. }) {
        return false;
    }
    true
}

fn stmts_ssa_supported(stmts: &[TypedStmt], extra_reject: &impl Fn(&TypedStmt) -> bool) -> bool {
    stmts.iter().all(|s| stmt_ssa_supported(s, extra_reject))
}

fn stmt_ssa_supported(stmt: &TypedStmt, extra_reject: &impl Fn(&TypedStmt) -> bool) -> bool {
    if extra_reject(stmt) {
        return false;
    }
    match stmt {
        TypedStmt::Print { items } => items.iter().all(|i| match i {
            TypedPrintItem::Expr(e) => expr_ssa_supported(e),
            TypedPrintItem::Str(_) => true,
        }),
        // eprint is not in the v1 SSA subset — always route to tree.
        TypedStmt::EPrint { .. } => false,
        TypedStmt::Let { ty, expr, .. } | TypedStmt::Reassign { ty, expr, .. } => {
            ssa_type_supported(ty) && expr_ssa_supported(expr)
        }
        TypedStmt::Drop { ty, .. } => ssa_type_supported(ty),
        TypedStmt::Discard { expr } => expr_ssa_supported(expr),
        TypedStmt::Return { expr } => expr_ssa_supported(expr),
        TypedStmt::Assert { expr, .. } | TypedStmt::Prove { expr } => {
            expr_ssa_supported(expr)
        }
        TypedStmt::If { cond, then_body, else_body } => {
            expr_ssa_supported(cond)
                && stmts_ssa_supported(then_body, extra_reject)
                && stmts_ssa_supported(else_body, extra_reject)
        }
        TypedStmt::While { cond, body, .. } => {
            expr_ssa_supported(cond) && stmts_ssa_supported(body, extra_reject)
        }
        TypedStmt::For { start, end, body, .. } => {
            expr_ssa_supported(start)
                && expr_ssa_supported(end)
                && stmts_ssa_supported(body, extra_reject)
        }
        TypedStmt::ForIter { collection_ty, consumes, element_ty, body, .. } => {
            // Consuming `for x in xs` over a Vec of non-Copy
            // elements: the SSA lowerer never emits a Drop for
            // the consumed collection, leaving the outer buffer
            // leaked (and there is no IR shape for "free the
            // outer buffer only, skip the per-element walk").
            // Route through tree-LLVM/tree-C which now handles
            // it directly via `emit_for_iter`. Closure #159.
            let consume_owned_vec = *consumes
                && matches!(collection_ty, Type::Vec(_))
                && !element_ty.is_copy();
            if consume_owned_vec {
                return false;
            }
            ssa_type_supported(collection_ty)
                && stmts_ssa_supported(body, extra_reject)
        }
        TypedStmt::TaskSpawn { body, .. } => stmts_ssa_supported(body, extra_reject),
        TypedStmt::TaskJoin { .. } => true,
        TypedStmt::ForIterShallowFree { .. } => true,
        // `unsafe(reason = "...")` blocks route through the tree
        // backends in v1 of Layer 1.1 — the tree backends emit the
        // reason as machine-readable deviation metadata, while the
        // SSA backends drop unknown HintKind variants silently
        // today. Wiring SSA emission for `HintKind::UnsafeBegin`
        // is a small follow-up; not needed for the v1 acceptance
        // tests (none of the Arc-or-Layer roadmap targets routes
        // unsafe-bearing functions through SSA today).
        TypedStmt::UnsafeBlock { .. } => false,
        TypedStmt::IndexAssign { base_ty, index, value, .. } => {
            ssa_type_supported(base_ty)
                && expr_ssa_supported(index)
                && expr_ssa_supported(value)
        }
        // FieldAssign currently has no SSA lowering — route
        // through the tree backend. T1.2 phase 2a follow-up.
        TypedStmt::FieldAssign { .. } => false,
        TypedStmt::Break { .. } | TypedStmt::Continue { .. } => true,
    }
}

fn expr_ssa_supported(expr: &TypedExpr) -> bool {
    if !ssa_type_supported(&expr.ty) {
        return false;
    }
    match &expr.kind {
        TypedExprKind::Binary { left, right, .. } => {
            expr_ssa_supported(left) && expr_ssa_supported(right)
        }
        TypedExprKind::Unary { expr: e, .. } => expr_ssa_supported(e),
        TypedExprKind::Cast { expr: e, .. } => expr_ssa_supported(e),
        TypedExprKind::Index { array, index, .. } => {
            expr_ssa_supported(array) && expr_ssa_supported(index)
        }
        TypedExprKind::Len { array, .. } => expr_ssa_supported(array),
        TypedExprKind::Call { name, args, .. } => {
            // `push_mut` (the in-place `push(mut ref xs, v)`
            // form), `pop` (in-place `pop(mut ref xs)`), and
            // `sort` / `sort_by` (in-place on `Vec<i64>`) all
            // operate through a Vec pointer and have no
            // SSA-backend lowering yet — route through the
            // tree backend.
            if name == "push_mut" || name == "push_unchecked" || name == "pop"
                || name == "sort" || name == "sort_by" || name == "sort_desc"
                || name == "vec_swap" || name == "vec_remove_at"
                || name == "vec_replace_all"
                || name == "reverse" || name == "dedup"
                || name == "find" || name == "contains"
                || name == "binary_search"
                || name == "swap_remove" || name == "insert"
                || name == "clear"
                || name == "str_contains" || name == "str_starts_with"
                || name == "str_ends_with" || name == "str_trim"
                || name == "str_replace" || name == "str_split"
                || name == "str_index_of"
                || name == "substring"
                || name == "str_repeat"
                || name == "str_to_upper" || name == "str_to_lower"
                || name == "parse_bool"
                || name == "str_join"
                || name == "str_pad_left" || name == "str_pad_right"
                || name == "str_lines"
                || name == "str_chars" || name == "str_reverse"
                || name == "str_strip_prefix" || name == "str_strip_suffix"
                || name == "str_count_char"
                || name == "i64_to_str"
                || name == "f64_to_str"
                || name == "bool_to_str"
                || name == "parse_int"
                || name == "parse_float"
                || name == "pow" || name == "sqrt"
                || name == "sin" || name == "cos" || name == "tan"
                || name == "floor" || name == "ceil" || name == "abs"
                || name == "log" || name == "log2" || name == "log10"
                || name == "exp" || name == "atan2"
                || name == "f64_is_nan" || name == "f64_is_inf"
                || name == "f64_is_finite"
                || name == "f64_pi" || name == "f64_e"
                || name == "f64_inf" || name == "f64_nan"
                || name == "f64_round" || name == "f64_trunc_to_i64"
                || name == "i64_gcd" || name == "i64_lcm" || name == "i64_pow"
                || name == "i64_abs_diff" || name == "i64_signum"
                || name == "f64_signum"
                || name == "is_ascii_digit" || name == "is_ascii_alpha"
                || name == "is_ascii_alphanumeric" || name == "is_ascii_whitespace"
                || name == "i64_count_set_bits"
                || name == "i64_leading_zeros"
                || name == "i64_trailing_zeros"
                || name == "i64_bswap"
                || name == "i64_rotate_left"
                || name == "i64_rotate_right"
                || name == "f64_to_bits" || name == "f64_from_bits"
                || name == "i64_min_value" || name == "i64_max_value"
                || name == "f64_max_finite"
                || name == "i64_div_floor" || name == "i64_mod_floor"
                || name == "f64_lerp" || name == "f64_clamp01"
                || name == "i64_log2_floor" || name == "i64_log2_ceil"
                || name == "i64_is_power_of_2" || name == "i64_next_power_of_2"
                || name == "i64_saturating_add"
                || name == "i64_saturating_sub"
                || name == "i64_saturating_mul"
                || name == "i64_min" || name == "i64_max" || name == "i64_clamp"
                || name == "f64_min" || name == "f64_max" || name == "f64_clamp"
                || name == "i64_isqrt"
                || name == "f64_hypot"
                || name == "f64_to_radians" || name == "f64_to_degrees"
                || name == "asin" || name == "acos" || name == "atan"
                || name == "sinh" || name == "cosh" || name == "tanh"
                || name == "f64_epsilon"
                || name == "f64_min_positive" || name == "f64_min_subnormal"
                || name == "f64_copysign" || name == "f64_fma"
                || name == "f64_remainder"
                || name == "f64_is_normal" || name == "f64_is_subnormal"
                || name == "f64_sign_bit"
                || name == "f64_next_up" || name == "f64_next_down"
                || name == "i64_div_ceil" || name == "i64_div_round"
                || name == "f64_trunc" || name == "f64_frac"
                || name == "i64_count_digits" || name == "i64_log10_floor"
                || name == "i64_log10_ceil" || name == "i64_pow_mod"
                || name == "i64_is_prime" || name == "i64_factorial"
                || name == "i64_fibonacci" || name == "i64_binomial"
                || name == "i64_perm" || name == "i64_avg"
                || name == "i64_wrap" || name == "f64_wrap"
                || name == "f64_mod_floor"
                || name == "i64_min_3" || name == "i64_max_3"
                || name == "f64_min_3" || name == "f64_max_3"
                || name == "f64_sigmoid" || name == "f64_softsign"
                || name == "f64_step" || name == "f64_smoothstep"
                || name == "f64_smoothstep5"
                || name == "f64_inv_lerp" || name == "f64_chebyshev"
                || name == "f64_l1_norm" || name == "i64_isqrt_ceil"
                || name == "i64_is_perfect_square"
                || name == "i64_divisor_count"
                || name == "i64_divisor_sum"
                || name == "i64_totient" || name == "i64_radical"
                || name == "i64_next_prime" || name == "i64_prev_prime"
                || name == "i64_mod_inverse"
                || name == "i64_set_bit" || name == "i64_clear_bit"
                || name == "i64_toggle_bit" || name == "i64_test_bit"
                || name == "i64_byte_at" || name == "i64_set_byte"
                || name == "i64_count_leading_ones"
                || name == "i64_count_trailing_ones"
                || name == "i64_parity" || name == "i64_mod_pos"
                || name == "i64_cube_root"
                || name == "f64_pow_int" || name == "f64_round_to_multiple"
                || name == "f64_quadratic_root"
                || name == "i64_reverse_bits"
                || name == "f64_relu" || name == "f64_leaky_relu"
                || name == "f64_softplus"
                || name == "f64_swish" || name == "f64_logit"
                || name == "f64_sinc" || name == "f64_safe_div"
                || name == "f64_safe_sqrt" || name == "i64_safe_div"
                || name == "f64_safe_log" || name == "f64_geometric_mean"
                || name == "f64_harmonic_mean" || name == "f64_quadratic_mean"
                || name == "f64_log_b"
                || name == "f64_erf" || name == "f64_erfc"
                || name == "f64_tgamma" || name == "f64_lgamma"
                || name == "f64_cbrt" || name == "f64_expm1"
                || name == "f64_log1p"
                || name == "f64_exp2" || name == "f64_exp10"
                || name == "f64_inv_sqrt" || name == "f64_round_to"
                || name == "f64_sec" || name == "f64_csc" || name == "f64_cot"
                || name == "f64_normal_pdf" || name == "f64_normal_cdf"
                || name == "f64_lerp_clamp"
                || name == "f64_atan2_deg" || name == "f64_uniform_random"
                || name == "f64_inv_smoothstep" || name == "f64_atan_deg"
                || name == "f64_asin_deg" || name == "f64_acos_deg"
                || name == "f64_sec_deg" || name == "f64_csc_deg"
                || name == "f64_cot_deg"
                || name == "str_is_ascii" || name == "str_is_digit_only"
                || name == "str_is_alpha_only" || name == "str_is_alphanumeric_only"
                || name == "str_is_whitespace_only" || name == "str_is_empty"
                || name == "f64_rgb_to_grayscale"
                || name == "i64_pack_rgb"
                || name == "i64_unpack_rgb_r"
                || name == "i64_unpack_rgb_g"
                || name == "i64_unpack_rgb_b"
                || name == "f64_remap"
                || name == "str_byte_at" || name == "str_len_bytes"
                || name == "str_starts_with_byte"
                || name == "str_ends_with_byte"
                || name == "str_byte_count"
                || name == "str_index_of_byte"
                || name == "str_last_index_of_byte"
                || name == "str_count_ascii_digits"
                || name == "str_count_ascii_alpha"
                || name == "str_count_ascii_alphanumeric"
                || name == "str_count_ascii_whitespace"
                || name == "str_count_ascii_upper"
                || name == "str_count_ascii_lower"
                || name == "str_count_ascii_punct"
                || name == "str_count_ascii_control"
                || name == "str_first_byte" || name == "str_last_byte"
                || name == "seed_rng" || name == "rand_i64"
                || name == "rand_in_range"
                || name == "rand_f64" || name == "rand_in_range_f64"
                || name == "rand_bool" || name == "rand_choice"
                || name == "rand_normal"
                || name == "sleep_ms"
                || name == "tcp_listen" || name == "tcp_socket_port"
                || name == "tcp_accept" || name == "tcp_connect_local"
                || name == "tcp_send_str" || name == "tcp_recv"
                || name == "tcp_send_buf" || name == "tcp_close"
                || name == "epoll_new" || name == "epoll_add_read"
                || name == "epoll_wait_one" || name == "epoll_close"
                || name == "tcp_set_nonblocking"
                || name == "tcp_accept_nb" || name == "tcp_recv_nb"
                || name == "io_recv_async" || name == "io_send_async"
                || name == "io_accept_async"
                || name == "sleep_ms_async" || name == "sleep_ms_finish"
                || name == "vec_chunks" || name == "vec_windows"
                || name == "vec_flatten" || name == "vec_group_by_value"
                || name == "vec_running_mean" || name == "vec_intersperse"
                || name == "hash_i64" || name == "hash_f64"
                || name == "hash_str" || name == "hash_combine"
                || name == "hash_combine_3"
                || name == "hash_combine_4"
                || name == "hash_pair" || name == "hash_triple"
                || name == "f64_hash_pair" || name == "f64_hash_triple"
                || name == "str_hash_pair" || name == "str_hash_triple"
                || name == "siphash_i64" || name == "siphash_str"
                || name == "heap_push" || name == "heap_pop"
                || name == "heap_peek" || name == "heapify"
                || name == "deque_new"
                || name == "deque_push_back" || name == "deque_push_front"
                || name == "deque_pop_back" || name == "deque_pop_front"
                || name == "deque_peek_back" || name == "deque_peek_front"
                || name == "deque_len" || name == "deque_clear"
                || name == "hashset_new" || name == "hashset_insert"
                || name == "hashset_contains" || name == "hashset_remove"
                || name == "hashset_len" || name == "hashset_clear"
                || name == "hashmap_new" || name == "hashmap_insert"
                || name == "hashmap_get" || name == "hashmap_contains_key"
                || name == "hashmap_remove"
                || name == "hashmap_len" || name == "hashmap_clear"
                || name == "btreeset_new" || name == "btreeset_insert"
                || name == "btreeset_contains" || name == "btreeset_remove"
                || name == "btreeset_len" || name == "btreeset_range"
                || name == "btreeset_min" || name == "btreeset_max"
                || name == "btreeset_clear"
                || name == "btreemap_new" || name == "btreemap_insert"
                || name == "btreemap_get" || name == "btreemap_contains_key"
                || name == "btreemap_remove" || name == "btreemap_len"
                || name == "btreemap_range_keys" || name == "btreemap_range_values"
                || name == "btreemap_min_key" || name == "btreemap_max_key"
                || name == "btreemap_clear"
                || name == "vec_map" || name == "vec_fold" || name == "vec_filter"
                || name == "vec_position"
                || name == "vec_count_if"
                || name == "vec_max_by" || name == "vec_min_by"
                || name == "vec_zip_with"
                || name == "vec_range" || name == "vec_repeat"
                || name == "vec_extend" || name == "vec_concat"
                || name == "vec_reverse_copy" || name == "vec_unique"
                || name == "vec_iota"
                || name == "vec_first" || name == "vec_last"
                || name == "vec_running_sum"
                || name == "vec_cumulative_max"
                || name == "vec_cumulative_min"
                || name == "vec_running_product"
                || name == "vec_running_xor"
                || name == "vec_running_and"
                || name == "vec_running_or"
                || name == "vec_all_equal"
                || name == "vec_is_sorted_asc"
                || name == "vec_is_sorted_desc"
                || name == "vec_is_palindrome"
                || name == "vec_sliding_max"
                || name == "vec_sliding_min"
                || name == "vec_sliding_sum"
                || name == "vec_sliding_product"
                || name == "vec_abs"
                || name == "vec_negate"
                || name == "vec_signum"
                || name == "vec_square"
                || name == "vec_add_scalar"
                || name == "vec_sub_scalar"
                || name == "vec_mul_scalar"
                || name == "vec_div_scalar"
                || name == "vec_eq_mask"
                || name == "vec_ne_mask"
                || name == "vec_lt_mask"
                || name == "vec_le_mask"
                || name == "vec_gt_mask"
                || name == "vec_ge_mask"
                || name == "vec_min_with_scalar"
                || name == "vec_max_with_scalar"
                || name == "vec_clamp_scalar"
                || name == "vec_add_pairwise"
                || name == "vec_sub_pairwise"
                || name == "vec_mul_pairwise"
                || name == "vec_min_pairwise"
                || name == "vec_max_pairwise"
                || name == "vec_mod_scalar"
                || name == "vec_pow_scalar"
                || name == "vec_shl_scalar"
                || name == "vec_shr_scalar"
                || name == "vec_rotate_left"
                || name == "vec_rotate_right"
                || name == "vec_shift_left"
                || name == "vec_shift_right"
                || name == "vec_subset_of"
                || name == "vec_disjoint"
                || name == "vec_equal_set"
                || name == "vec_equal_seq"
                || name == "vec_diff"
                || name == "vec_pad_left"
                || name == "vec_pad_right"
                || name == "vec_replace_value"
                || name == "vec_count_distinct"
                || name == "vec_indices_of_value"
                || name == "vec_dedup_consecutive"
                || name == "vec_mean"
                || name == "vec_merge_sorted"
                || name == "vec_insert_sorted"
                || name == "vec_is_sorted_unique"
                || name == "vec_range_span"
                || name == "vec_mode"
                || name == "vec_kth_smallest"
                || name == "vec_median"
                || name == "vec_dot"
                || name == "vec_intersect" || name == "vec_difference" || name == "vec_union"
                || name == "option_unwrap_or"
                || name == "option_is_some" || name == "option_is_none"
                || name == "option_map"
                || name == "option_filter" || name == "option_or"
                || name == "option_and_then"
                || name == "option_unwrap_or_f64"
                || name == "option_is_some_f64" || name == "option_is_none_f64"
                || name == "vec_take" || name == "vec_drop" || name == "vec_map_fold"
                || name == "vec_take_while" || name == "vec_drop_while"
                || name == "vec_filter_fold" || name == "vec_map_filter"
                || name == "vec_map_filter_fold"
                || name == "vec_sum" || name == "vec_product"
                || name == "vec_min" || name == "vec_max"
                || name == "vec_argmin" || name == "vec_argmax"
                || name == "vec_count_value"
                || name == "vec_index_of_value"
                || name == "vec_last_index_of_value"
                || name == "vec_count" || name == "vec_any" || name == "vec_all"
                || name == "vec_chain"
                || name == "union_find_new" || name == "union_find_union"
                || name == "union_find_find" || name == "union_find_connected"
                || name == "union_find_count" || name == "union_find_clear"
                || name == "binary_heap_new" || name == "binary_heap_push"
                || name == "binary_heap_pop" || name == "binary_heap_peek"
                || name == "binary_heap_len" || name == "binary_heap_clear"
                || name == "bloom_filter_new" || name == "bloom_filter_insert"
                || name == "bloom_filter_contains" || name == "bloom_filter_len"
                || name == "bloom_filter_count" || name == "bloom_filter_clear"
                || name == "bst_new" || name == "bst_insert"
                || name == "bst_contains" || name == "bst_remove"
                || name == "bst_len" || name == "bst_min" || name == "bst_max"
                || name == "bst_clear"
                || name == "graph_new" || name == "graph_add_edge"
                || name == "graph_num_nodes" || name == "graph_num_edges"
                || name == "graph_bfs_reach" || name == "graph_dfs_reach"
                || name == "graph_dijkstra"
                || name == "graph_has_cycle" || name == "graph_mst_kruskal"
                || name == "graph_mst_prim"
                || name == "graph_astar" || name == "graph_topo_sort"
                || name == "graph_clear"
                || name == "trie_new" || name == "trie_insert"
                || name == "trie_contains" || name == "trie_starts_with"
                || name == "trie_delete"
                || name == "trie_len" || name == "trie_node_count"
                || name == "trie_clear"
                || name == "skiplist_new" || name == "skiplist_insert"
                || name == "skiplist_contains" || name == "skiplist_remove"
                || name == "skiplist_len"
                || name == "skiplist_min" || name == "skiplist_max"
                || name == "skiplist_clear"
            {
                return false;
            }
            args.iter().all(expr_ssa_supported)
        }
        TypedExprKind::CallIndirect { callee, args } => {
            expr_ssa_supported(callee) && args.iter().all(expr_ssa_supported)
        }
        TypedExprKind::ArrayLit { elements } => {
            elements.iter().all(expr_ssa_supported)
        }
        TypedExprKind::Int(_)
        | TypedExprKind::Float(_)
        | TypedExprKind::Bool(_)
        | TypedExprKind::Str(_)
        | TypedExprKind::Var(_)
        | TypedExprKind::Ref { .. }
        | TypedExprKind::RefMut { .. }
        | TypedExprKind::FnRef { .. } => true,
        // Tuples flow through the tree backends; SSA lowering
        // surfaces LowerError which routes here. Mark
        // unsupported so the SSA gate falls back early. T1.1.
        TypedExprKind::Tuple { .. } | TypedExprKind::TupleAccess { .. } => false,
        // Structs likewise fall back to tree backends until
        // SSA support lands (T1.2 follow-up).
        TypedExprKind::StructLit { .. } | TypedExprKind::FieldAccess { .. } => false,
        // Enums + match also fall through to tree backends
        // for now. T1.3 follow-up.
        TypedExprKind::EnumVariant { .. }
        | TypedExprKind::EnumVariantWithPayload { .. }
        | TypedExprKind::Match { .. } => false,
        // If-expressions route through tree backends. T4.
        TypedExprKind::IfExpr { .. } => false,
        // Block expressions route through tree backends in
        // v1 (SSA lowering can be added in a follow-up).
        TypedExprKind::Block { .. } => false,
        // Struct field-borrow routes through tree backends —
        // SSA doesn't model field-paths yet. T1.2 phase 2b
        // follow-up.
        TypedExprKind::RefField { .. } | TypedExprKind::RefMutField { .. } => false,
        // `mut ref vec[i]` is tree-backend only (A4.3 v1 lift).
        TypedExprKind::RefMutIndex { .. } => false,
        // `dyn Iface` method dispatch / coercion route
        // through tree backends; SSA vtable lowering lands
        // with Phase 3.
        TypedExprKind::DynDispatch { .. } | TypedExprKind::DynCoerce { .. } => false,
        // Forall is proof-only; never reaches codegen.
        TypedExprKind::Forall { .. } => false,
    }
}

/// Try the SSA-driven C backend first; fall back to the
/// tree-based path if the SSA pipeline doesn't yet cover a
/// feature the program uses (Vec/Channel/FnPtr/Atomic/etc.).
/// Once SSA-C reaches feature parity, the fallback can go.
fn emit_c_via_ssa(ir: &TypedProgram) -> String {
    if ssa_path_supports(ir, ssa_c_extra_reject) {
        let (module, lower_errs) = lower_program(ir);
        if lower_errs.is_empty() {
            if let Ok(c) = ssa_backend_c::emit(&module) {
                return c;
            }
        }
    }
    CBackend.emit(ir)
}

/// Same dual-path strategy for the LLVM backend.
fn emit_llvm_via_ssa(ir: &TypedProgram) -> String {
    // T1.3 phase 2b: payloaded enums need tagged-union codegen.
    // Tree-LLVM now supports them (closure #90); SSA-LLVM
    // doesn't. Force the tree-LLVM path for payloaded
    // programs.
    let has_payloaded_enum = ir
        .enums
        .iter()
        .any(|e| e.payload_types.iter().any(|p| p.is_some()));
    if !has_payloaded_enum && ssa_path_supports(ir, ssa_llvm_extra_reject) {
        let (module, lower_errs) = lower_program(ir);
        if lower_errs.is_empty() {
            if let Ok(ll) = ssa_backend_llvm::emit(&module) {
                return ll;
            }
        }
    }
    LlvmBackend.emit(ir)
}
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::{SystemTime, UNIX_EPOCH};

const HELP: &str = "\
vanic — vāṇी language compiler driver
                  (legacy alias: `intentc`; identical binary)

USAGE:
    vanic <COMMAND> [ARGS]
        # or: intentc <COMMAND> [ARGS]  (legacy)

COMMANDS:
    check <path>... [--json] [--no-verify] [--smt-debug]
        [--big-o[=<auto|force|off>]]
                                          Type-check one or more sources.
                                          Paths may be files or directories
                                          (the latter expand recursively to
                                          *.vani descendants). With --json,
                                          a single combined diagnostics
                                          object on stdout collects all
                                          findings across every file. With
                                          --no-verify, the SMT verifier
                                          is skipped for fast iteration
                                          (runtime guards stay in place);
                                          same effect as VANIC_NO_VERIFY=1
                                          (or legacy INTENTC_NO_VERIFY=1).
                                          With --smt-debug, every SMT query
                                          and z3 response is dumped to
                                          stderr (also via VANIC_SMT_DEBUG=1
                                          or legacy INTENTC_SMT_DEBUG=1).
                                          With --big-o[=mode], print a Big-O
                                          complexity annotation per fn:
                                          auto (default) skips O(1); force
                                          includes every fn; off is no-op.
    emit <file.vani> [--backend=<c|llvm>] [-o out]
        [--big-o[=<auto|force|off>]]
                                          Emit lowered source for a program.
                                          --backend defaults to 'llvm'. Pass
                                          --backend=c for the legacy C output.
                                          With --big-o, a per-fn complexity
                                          comment block is prepended to the
                                          emitted artifact.
    emit-c <file.vani> [-o out.c]       Legacy alias for 'emit --backend=c'.
                                          Kept for back-compat.
    run <file.vani> [--backend=<c|llvm>]
        [--link-with PATH ...]            Compile and run a program. Default
        [-l<name> ...]                    backend is 'llvm' (emits LLVM IR
        [--big-o[=<auto|force|off>]]      and runs it via $LLI or `lli`).
        [--target=<triple>]               With --backend=c, invokes $CC or
                                          `cc` on the C output.
                                          --link-with / -l<name> require
                                          --backend=c (LLVM-JIT auto-resolves
                                          host symbols). --big-o prints per-fn
                                          complexity to stderr before running.
                                          --target=<triple> cross-targets:
                                          bare-metal triples (*-none-eabi /
                                          *-elf) produce an ELF and run via
                                          QEMU if available (QEMU_<ARCH> or
                                          qemu-<arch>-static on PATH).
    build <file.vani> [-o out]          AOT-compile to a native binary.
          [--link-with PATH ...]          Lowers via the LLVM backend, calls
          [-l<name> ...]                  $LLC (or `llc`) for object code,
          [--target=<triple>]             then $CC (or `cc`) to link with
          [--no-std]                      libc. Output defaults to the
                                          source file's stem in the cwd.
                                          --target=<triple> cross-compiles:
                                          passes --mtriple to llc and uses
                                          $CROSS_CC or <triple>-gcc as the
                                          cross linker. Bare-metal triples
                                          (*-none-eabi / *-elf) also
                                          suppress libc/libm/OpenMP flags
                                          and auto-activate --no-std.
                                          --link-with adds an extra object
                                          or source file to the link line
                                          (e.g. foo.o, foo.c) for `extern
                                          \"C\" fn` whose body lives in a
                                          separately-compiled translation
                                          unit. -l<name> forwards a system
                                          library flag (e.g. -lm) to cc.
    tokens <file.vani>                  Dump the token stream (debug).
    ast <file.vani>                     Dump the parsed AST (debug). Skips
                                          type checking.
    ir <file.vani>                      Dump the typed IR (debug). Runs the
                                          checker; what the backends see.
    deviations <file.vani> [--format=<csv|json|text>] [--out=<file>]
                                          Extract every `unsafe(reason = \"...\")`
                                          block as a structured deviation record.
                                          The audit artifact for ASIL-D / DO-178C /
                                          IEC 62304 / MISRA sign-off. Default
                                          --format=text writes a human-readable
                                          summary to stdout; --format=csv / json
                                          produce machine-readable formats. With
                                          --out, writes to that file instead.
    stack-depth <file.vani> [--format=<csv|json|text>] [--max=<bytes>] [--entry=<fn>]
                                          Per-function frame-size estimates +
                                          max stack depth reachable from each
                                          entry-point. Detects unbounded
                                          recursion (or recursion via cycle).
                                          With --max=<bytes>, exits 1 if any
                                          entry-point exceeds the budget (or
                                          is unbounded). With --entry=<fn>,
                                          only reports for that entry-point.
    acyclicity <file.vani> [--format=<csv|json|text>]
                                          Prove the call graph has no cycles
                                          (modulo `#[bounded(N)]`-annotated
                                          members). Catches mutual recursion
                                          that single-fn checks miss. Required
                                          by DO-178C / ASIL-D. Exit 1 on
                                          violation.
    audit-pack <file.vani> [--out=<file.md>] [--max-stack=<bytes>] [--max-complexity=<N>]
                                          Run all six audit reports
                                          (deviations / stack-depth /
                                          acyclicity / hashmap-usage /
                                          complexity / safety-attrs) and
                                          bundle them into a single Markdown
                                          document. With --out, writes the
                                          file; otherwise prints to stdout.
                                          Exit 1 if any hard-gate exceeds
                                          its bound (--max-stack /
                                          --max-complexity / acyclicity).
    safety-attrs <file.vani> [--format=<csv|json|text>]
                                          Per-function listing of every
                                          safety annotation (composite tags
                                          + primitives + budgets). Reviewer-
                                          friendly audit pack: one row per
                                          fn, columns for each #[...] /
                                          standard / budget. No exit-code
                                          gate; pair with `complexity` /
                                          `stack-depth` for full coverage.
    complexity <file.vani> [--max=<N>] [--format=<csv|json|text>]
                                          Per-function McCabe cyclomatic
                                          complexity. With --max=<N>, exits 1
                                          if any fn exceeds N. MISRA / ISO
                                          26262 / DO-178C reviews use this as
                                          the standard branch-depth ceiling.
    hashmap-usage <file.vani> [--format=<csv|json|text>]
                                          Surface every HashMap<K, V> pair the
                                          program uses, with the mangled bundle
                                          tag (`intent_hashmap_<K>_<V>`). Audit
                                          artifact for embedded teams reviewing
                                          HashMap shapes; consumed by per-
                                          (K, V) bundle emitters in ARC 1.4/1.5.
    fmt <path>... [--check|--in-place]
                                          Pretty-print canonical source.
                                          // comments are preserved. Paths
                                          may be files or directories (the
                                          latter expand recursively to
                                          *.vani descendants; dot-dirs
                                          skipped).
                                          Default writes to stdout (single-
                                          file only); --check exits 1 if
                                          any file is not canonical;
                                          --in-place rewrites each file
                                          (mtime stable when canonical).
    test <path>... [--json] [--smt-debug] Compile + run each path via the
                                          LLVM backend, treating exit 0 as
                                          pass. Paths may be files or
                                          directories (the latter expand
                                          recursively to *.vani
                                          descendants; dot-dirs skipped).
                                          Output per file plus a summary;
                                          exits 1 if any failed.
                                          With --json, a machine-readable
                                          results object is printed on
                                          stdout instead of human lines.
                                          With --smt-debug, every SMT query
                                          and z3 response is dumped to
                                          stderr (also via VANIC_SMT_DEBUG=1
                                          or legacy INTENTC_SMT_DEBUG=1).

    apply-publisher [--accept-agreement]   Apply to become a Kosh publisher.
                                          Without the flag: fetches and prints
                                          the Publisher Agreement.
                                          With --accept-agreement: reads your
                                          gh identity, opens a GitHub issue in
                                          kosh-index recording your acceptance,
                                          and waits for operator approval.
                                          Requires `gh` CLI + `gh auth login`.

    registry-approve <username>            [Operator only] Approve a pending
                                          publisher application. Moves the user
                                          from pending_publishers to
                                          allowed_publishers in governance.json.

    registry-blacklist <username>          [Operator only] Blacklist a publisher.
           [--reason=<text>]              Removes from allowed/pending lists,
                                          adds to blacklisted with date +
                                          reason. Default reason: policy
                                          violation.

    publish                                Publish the current package to the
                                          Kosh registry. Reads [package].name
                                          and [package].version from vani.toml,
                                          builds a tarball, creates a GitHub
                                          Release in kosh-index, and appends a
                                          line to the sparse index. Requires
                                          the GitHub CLI (`gh`) to be installed
                                          and authenticated.

    add <name>[@<version>]                 Add a package from the Kosh registry.
                                          Fetches the best matching version,
                                          verifies SHA-256, downloads + extracts
                                          to vendor/<name>/, then updates
                                          vani.toml and rewrites vani.lock.
                                          Examples:
                                            vanic add mathlib
                                            vanic add mathlib@^1.0
                                            vanic add mathlib@1.2.3

    remove <name>                          Remove a dependency: deletes the
                                          [deps] entry from vani.toml, removes
                                          vendor/<name>/, and rewrites vani.lock.

    search [<query>]                       Search the Kosh registry for packages.
                                          Without a query: lists all packages.
                                          With a query: filters by name substring.
                                          Shows name, latest version, and count.

    update                                 Re-resolve all registry deps to their
                                          latest compatible version. Only updates
                                          deps whose path is ./vendor/<name>.
                                          Verifies SHA-256 checksums. Rewrites
                                          vani.toml and vani.lock on change.

    vendor [--manifest=<path>]              Copy each dep's source tree into
                                          vendor/<name>/ under the project root
                                          and write/update vani.lock. No network
                                          access; path-deps only.

MANIFEST (vani.toml):
    For run / build / check / emit / ir / ast / tokens, if
    no source file is given on the command line, the driver
    walks up from the current directory looking for a
    `vani.toml` manifest. When found, its `[package].entry`
    key supplies the source file. `vani.lock` is written
    (or updated) automatically whenever the manifest is
    newer than the lock. Full format:

        [package]
        name = \"my_project\"
        version = \"0.1.0\"           # optional semver
        entry = \"src/main.vani\"

        [deps]
        mathlib = { path = \"../math\" }
        utils   = { path = \"../utils\", version = \"^1.0\" }

GLOBAL OPTIONS:
    -h, --help        Show this message
    -V, --version     Show version
";

fn main() -> ExitCode {
    // Windows default stack is 1 MB; Linux/macOS default is 8 MB.
    // The compiler's recursive descent through large programs can exceed
    // 1 MB, so on Windows we run the real work on a 64 MB thread.
    #[cfg(target_os = "windows")]
    {
        let result = std::thread::Builder::new()
            .stack_size(64 * 1024 * 1024)
            .spawn(run)
            .expect("failed to spawn worker thread")
            .join()
            .expect("worker thread panicked");
        return match result {
            Ok(code) => code,
            Err(message) => {
                eprintln!("{}", message);
                ExitCode::from(1)
            }
        };
    }
    #[cfg(not(target_os = "windows"))]
    match run() {
        Ok(code) => code,
        Err(message) => {
            eprintln!("{}", message);
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<ExitCode, String> {
    let args: Vec<String> = env::args().collect();

    // Deprecation warning when invoked as `intentc` (legacy alias).
    // The `intentc` [[bin]] entry will be removed at the v0.2.0
    // release boundary. See docs/decisions.md 2026-06-06 entry.
    if args
        .first()
        .and_then(|a| std::path::Path::new(a).file_stem())
        .and_then(|s| s.to_str())
        == Some("intentc")
    {
        eprintln!(
            "warning: `intentc` is deprecated and will be removed in v0.2.0. \
             Use `vanic` instead."
        );
    }

    if args.len() < 2 {
        return Err(HELP.to_string());
    }

    match args[1].as_str() {
        "-h" | "--help" => {
            println!("{}", HELP);
            Ok(ExitCode::SUCCESS)
        }
        "-V" | "--version" => {
            // Use argv[0] to report whichever name the user invoked
            // (`vanic` or the legacy `intentc`).
            let bin = std::path::Path::new(&args[0])
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "vanic".to_string());
            println!("{} {}", bin, env!("CARGO_PKG_VERSION"));
            Ok(ExitCode::SUCCESS)
        }
        "check" => {
            // Type-check one or more files. Paths may be files or
            // directories (the latter expand recursively to
            // `*.vani` descendants via `expand_intent_paths`).
            // Exit 1 if any file's check fails. For `--json`, all
            // diagnostics across all files are flattened into a
            // single `{"diagnostics": [...]}` object so the schema
            // matches the single-file form.
            let mut json = false;
            let mut big_o_mode: Option<vani::big_o::BigOMode> = None;
            let mut path_args: Vec<String> = Vec::new();
            for arg in args.iter().skip(2) {
                match arg.as_str() {
                    "--json" => json = true,
                    // User-direction item (2026-06-08): static
                    // Big-O annotation per fn. `--big-o` with no
                    // value defaults to Auto (annotate fns that
                    // aren't O(1)); `--big-o=force` annotates
                    // every fn; `--big-o=off` is no-op so users
                    // can override a config-file enable.
                    "--big-o" => big_o_mode = Some(vani::big_o::BigOMode::Auto),
                    s if s.starts_with("--big-o=") => {
                        let val = &s["--big-o=".len()..];
                        match vani::big_o::BigOMode::parse(val) {
                            Some(m) => big_o_mode = Some(m),
                            None => {
                                return Err(format!(
                                    "unknown --big-o mode '{}'; expected auto|force|off",
                                    val,
                                ));
                            }
                        }
                    }
                    "--no-verify" => {
                        // Same effect as VANIC_NO_VERIFY=1 (or the
                        // legacy INTENTC_NO_VERIFY=1) — sets the env
                        // var for the remainder of the process so
                        // the checker's gates fire.
                        std::env::set_var("VANIC_NO_VERIFY", "1");
                    }
                    "--smt-debug" => {
                        // Surface the existing VANIC_SMT_DEBUG=1
                        // (legacy: INTENTC_SMT_DEBUG=1) toggle as a
                        // CLI flag so users debugging a `prove`
                        // failure don't have to rediscover the env
                        // var. The verifier dumps each SMT query +
                        // z3 response to stderr.
                        std::env::set_var("VANIC_SMT_DEBUG", "1");
                    }
                    other if other.starts_with('-') => {
                        return Err(format!("unexpected argument '{}'", other));
                    }
                    other => path_args.push(other.to_string()),
                }
            }
            if path_args.is_empty() {
                return Err(format!("'check' requires a path argument\n\n{}", HELP));
            }
            let files = expand_intent_paths(&path_args)?;
            if files.is_empty() {
                return Err("no .vani files to check".into());
            }

            let mut failed = 0usize;
            // For --json multi-file: accumulate every file's
            // FileMap entries (shifted into a global frame) and
            // every diagnostic (shifted by the same amount) into a
            // single combined map/diags pair, then emit one JSON
            // object at the end.
            let mut combined_map = vani::diagnostic::FileMap::new();
            let mut combined_diags: Vec<vani::diagnostic::Diagnostic> = Vec::new();

            for file in &files {
                match vani::compile_path(file) {
                    Ok((checked, _map)) => {
                        if !json && files.len() > 1 {
                            println!("ok: {}", file.display());
                        }
                        // User-direction item (2026-06-08):
                        // emit Big-O annotations for this
                        // file's fns when the flag is set.
                        // Auto mode skips O(1); force mode
                        // includes every fn; off mode is
                        // already filtered above.
                        if let Some(mode) = big_o_mode {
                            if mode != vani::big_o::BigOMode::Off {
                                let annotations = vani::big_o::annotate_program(
                                    &checked.ir, mode,
                                );
                                if !annotations.is_empty() && !json {
                                    if files.len() > 1 {
                                        println!("complexity ({}):", file.display());
                                    }
                                    for (name, complexity) in &annotations {
                                        println!("  fn {}: {}", name, complexity);
                                    }
                                }
                            }
                        }
                    }
                    Err((map, diagnostics)) => {
                        failed += 1;
                        if json {
                            let shift = combined_map.extend_with(&map);
                            for d in diagnostics {
                                let mut shifted = d.clone();
                                shifted.span = vani::span::Span::new(
                                    d.span.start + shift,
                                    d.span.end + shift,
                                );
                                shifted.related = d
                                    .related
                                    .iter()
                                    .map(|(s, note)| {
                                        (
                                            vani::span::Span::new(
                                                s.start + shift,
                                                s.end + shift,
                                            ),
                                            note.clone(),
                                        )
                                    })
                                    .collect();
                                combined_diags.push(shifted);
                            }
                        } else if files.len() == 1 {
                            return Err(
                                vani::diagnostic::format_diagnostics_with_files(
                                    &map,
                                    &diagnostics,
                                ),
                            );
                        } else {
                            eprintln!(
                                "{}",
                                vani::diagnostic::format_diagnostics_with_files(
                                    &map,
                                    &diagnostics,
                                )
                            );
                        }
                    }
                }
            }

            if json {
                print!(
                    "{}",
                    vani::diagnostic::format_diagnostics_json_with_files(
                        &combined_map,
                        &combined_diags,
                    )
                );
                // The single-file `{"diagnostics":[]}` success case
                // also flows through here — combined_map is empty
                // and the formatter emits the right empty object.
            } else if failed == 0 {
                if files.len() == 1 {
                    println!("ok: {}", files[0].display());
                } else {
                    println!("ok: {} file(s)", files.len());
                }
            }
            if failed > 0 {
                Ok(ExitCode::from(1))
            } else {
                Ok(ExitCode::SUCCESS)
            }
        }
        "emit" | "emit-c" => {
            // `emit-c` is the legacy spelling kept for back-compat; the
            // new `emit` form takes an explicit `--backend=<c|llvm>` flag
            // so we can grow into LLVM (and beyond) without churning the
            // CLI. The legacy alias pins backend=c regardless of flags.
            let cmd_name = args[1].clone();
            let file = required_file(&args, 2, &cmd_name)?;
            // Pre-scan for --no-std and --target before parse_emit_args
            // (which handles the remaining positional flags).
            let no_std = args[3..].iter().any(|a| a == "--no-std")
                || args[3..].iter().any(|a| {
                    if let Some(t) = a.strip_prefix("--target=") {
                        t.contains("none") || t.contains("eabi") || t.contains("-elf")
                    } else {
                        false
                    }
                });
            // Filter out --no-std and --target= before parsing remaining flags.
            let filtered_args: Vec<String> = {
                let mut v = vec![args[0].clone(), args[1].clone(), args[2].clone()];
                v.extend(args[3..].iter().filter(|a| {
                    *a != "--no-std" && !a.starts_with("--target=")
                }).cloned());
                v
            };
            let (backend_kind, out, big_o_mode) = parse_emit_args(&filtered_args, 3, &cmd_name)?;
            let checked = compile_path_or_report(&file)?;
            let body = match backend_kind {
                BackendKind::C => {
                    if no_std {
                        vani::backend_c::emit_c_no_std(&checked.ir)
                    } else {
                        emit_c_via_ssa(&checked.ir)
                    }
                }
                BackendKind::Llvm => emit_llvm_via_ssa(&checked.ir),
            };
            // User-direction item (2026-06-08): Big-O comment
            // block prepended to the emitted artifact when the
            // flag is set. C uses `//` comments; LLVM IR uses
            // `;` comments. Auto mode skips O(1) fns; force
            // includes everything. `off` is a no-op (no
            // comment block).
            let text = if let Some(mode) = big_o_mode {
                if mode == vani::big_o::BigOMode::Off {
                    body
                } else {
                    let annotations =
                        vani::big_o::annotate_program(&checked.ir, mode);
                    if annotations.is_empty() {
                        body
                    } else {
                        let comment_lead = match backend_kind {
                            BackendKind::C => "//",
                            BackendKind::Llvm => ";",
                        };
                        let mut header = String::new();
                        header.push_str(&format!(
                            "{} ─── Big-O complexity annotations ({:?} mode) ───\n",
                            comment_lead, mode,
                        ));
                        for (name, complexity) in &annotations {
                            header.push_str(&format!(
                                "{} {}: {}\n",
                                comment_lead, name, complexity,
                            ));
                        }
                        header.push_str(&format!(
                            "{} ────────────────────────────────────────────────\n\n",
                            comment_lead,
                        ));
                        header.push_str(&body);
                        header
                    }
                }
            } else {
                body
            };
            match out {
                Some(path) => fs::write(&path, text)
                    .map_err(|error| format!("failed to write '{}': {}", path.display(), error))?,
                None => print!("{}", text),
            }
            Ok(ExitCode::SUCCESS)
        }
        "run" => {
            let (file, flag_start) = required_file_at(&args, 2, "run")?;
            let (backend_kind, link_args, big_o_mode, target) = parse_run_args(&args, flag_start)?;
            match backend_kind {
                BackendKind::C => run_program(&file, &link_args, big_o_mode),
                BackendKind::Llvm => {
                    if !link_args.is_empty() {
                        return Err(
                            "--link-with / -l<name> require --backend=c \
                             (LLVM-JIT via lli auto-resolves libc/libm symbols \
                             from the host process; use `vanic build … \
                             --link-with …` for AOT linking with custom code)"
                                .to_string(),
                        );
                    }
                    if let Some(triple) = &target {
                        if is_bare_metal_triple(triple) {
                            return Err(format!(
                                "bare-metal target '{}' cannot run via LLVM-JIT.\n\
                                 Use `vanic build --target={} -o out.elf` to \
                                 produce an ELF, then run it on your board or \
                                 via QEMU: qemu-system-<arch> -kernel out.elf",
                                triple, triple
                            ));
                        }
                        // Linux cross-targets: build an ELF and try QEMU user-mode.
                        return run_program_llvm_target(&file, big_o_mode, triple);
                    }
                    run_program_llvm(&file, big_o_mode)
                }
            }
        }
        "build" => {
            let (file, flag_start) = required_file_at(&args, 2, "build")?;
            let (out, link_args, target) = parse_build_args(&args, flag_start)?;
            build_program_llvm(&file, out.as_deref(), &link_args, target.as_deref())
        }
        "tokens" => {
            // Debug subcommand: dump the token stream to stdout.
            // Useful for parser/lexer development — see a token's
            // source span and kind without running the full
            // pipeline.
            let file = required_file(&args, 2, "tokens")?;
            let source = fs::read_to_string(&file)
                .map_err(|error| format!("failed to read '{}': {}", file.display(), error))?;
            match vani::lexer::lex(&source) {
                Ok(tokens) => {
                    for tok in &tokens {
                        println!("{:>5}..{:<5} {:?}", tok.span.start, tok.span.end, tok.kind);
                    }
                    Ok(ExitCode::SUCCESS)
                }
                Err(diag) => Err(vani::diagnostic::format_diagnostics(
                    file.to_str().unwrap_or("<input>"),
                    &source,
                    &[diag],
                )),
            }
        }
        "ast" => {
            // Debug subcommand: dump the parsed AST. Skips the
            // type checker — useful when you want to see what the
            // parser produced even if the checker would reject.
            let file = required_file(&args, 2, "ast")?;
            let source = fs::read_to_string(&file)
                .map_err(|error| format!("failed to read '{}': {}", file.display(), error))?;
            let tokens = vani::lexer::lex(&source).map_err(|diag| {
                vani::diagnostic::format_diagnostics(
                    file.to_str().unwrap_or("<input>"),
                    &source,
                    &[diag],
                )
            })?;
            let (program, diags) = vani::parser::parse(tokens);
            // Print whatever the parser produced, then surface any
            // parse diagnostics on stderr so partial parses are
            // still useful.
            println!("{:#?}", program);
            if !diags.is_empty() {
                eprintln!(
                    "{}",
                    vani::diagnostic::format_diagnostics(
                        file.to_str().unwrap_or("<input>"),
                        &source,
                        &diags
                    )
                );
            }
            Ok(ExitCode::SUCCESS)
        }
        "ir" => {
            // Debug subcommand: run the full pipeline through the
            // type checker and dump the resulting TypedProgram.
            // Useful for checker / IR work — see what the backends
            // are actually about to lower.
            let file = required_file(&args, 2, "ir")?;
            let checked = compile_path_or_report(&file)?;
            println!("{:#?}", checked.ir);
            Ok(ExitCode::SUCCESS)
        }
        "stack-depth" => {
            // T1.3 of safety-standard arc: per-function frame
            // size estimates + max stack depth per entry-point.
            // Usage: vanic stack-depth <path>
            //          [--format=csv|json|text]
            //          [--max=<bytes>]
            //          [--entry=<fn>]
            // Fails (exit 1) if --max is set and any entry-point
            // exceeds it (or is unbounded).
            let mut format = "text";
            let mut max_bytes: Option<u64> = None;
            let mut entry: Option<String> = None;
            let mut path_arg: Option<String> = None;
            let mut idx = 2;
            while idx < args.len() {
                let arg = &args[idx];
                if let Some(value) = arg.strip_prefix("--format=") {
                    format = match value {
                        "csv" => "csv",
                        "json" => "json",
                        "text" => "text",
                        other => {
                            return Err(format!(
                                "unsupported --format='{}'; choose csv | json | text",
                                other
                            ));
                        }
                    };
                } else if let Some(value) = arg.strip_prefix("--max=") {
                    max_bytes = Some(value.parse::<u64>().map_err(|_| {
                        format!("--max=<bytes> expects a non-negative integer, got '{}'", value)
                    })?);
                } else if let Some(value) = arg.strip_prefix("--entry=") {
                    entry = Some(value.to_string());
                } else if arg.starts_with('-') {
                    return Err(format!("unexpected argument '{}'", arg));
                } else if path_arg.is_none() {
                    path_arg = Some(arg.clone());
                } else {
                    return Err("'stack-depth' takes one path argument".into());
                }
                idx += 1;
            }
            let path = path_arg.ok_or_else(|| {
                "'stack-depth' requires a path argument".to_string()
            })?;
            let path = std::path::PathBuf::from(path);
            let (checked, _file_map) = vani::compile_path(&path)
                .map_err(|(map, diagnostics)| {
                    vani::diagnostic::format_diagnostics_with_files(&map, &diagnostics)
                })?;
            let report = vani::stack_depth::compute_stack_depths(
                &checked.ir,
                entry.as_deref(),
            );
            match format {
                "csv" => {
                    print!("{}", vani::stack_depth::format_csv(&report));
                    Ok(ExitCode::SUCCESS)
                }
                "json" => {
                    print!("{}", vani::stack_depth::format_json(&report));
                    Ok(ExitCode::SUCCESS)
                }
                _ => {
                    let (out, failure) =
                        vani::stack_depth::format_text(&report, max_bytes);
                    print!("{}", out);
                    if failure {
                        Ok(ExitCode::from(1))
                    } else {
                        Ok(ExitCode::SUCCESS)
                    }
                }
            }
        }
        "acyclicity" => {
            // T3.3 of the safety-standard alignment arc: prove the
            // program's call graph has no cycles (modulo
            // `#[bounded(N)]`-annotated members). Catches mutual
            // recursion that single-function checks would miss.
            //
            // Usage: vanic acyclicity <path> [--format=csv|json|text]
            // Exit 1 if any non-bounded cycle exists.
            let mut format = "text";
            let mut path_arg: Option<String> = None;
            let mut idx = 2;
            while idx < args.len() {
                let arg = &args[idx];
                if let Some(value) = arg.strip_prefix("--format=") {
                    format = match value {
                        "csv" => "csv",
                        "json" => "json",
                        "text" => "text",
                        other => {
                            return Err(format!(
                                "unsupported --format='{}'; choose csv | json | text",
                                other
                            ));
                        }
                    };
                } else if arg.starts_with('-') {
                    return Err(format!("unexpected argument '{}'", arg));
                } else if path_arg.is_none() {
                    path_arg = Some(arg.clone());
                } else {
                    return Err("'acyclicity' takes one path argument".into());
                }
                idx += 1;
            }
            let path = path_arg.ok_or_else(|| {
                "'acyclicity' requires a path argument".to_string()
            })?;
            let path = std::path::PathBuf::from(path);
            let (checked, _file_map) = vani::compile_path(&path)
                .map_err(|(map, diagnostics)| {
                    vani::diagnostic::format_diagnostics_with_files(&map, &diagnostics)
                })?;
            let report = vani::acyclicity::check_acyclicity(&checked.ir);
            match format {
                "csv" => print!("{}", vani::acyclicity::format_csv(&report)),
                "json" => print!("{}", vani::acyclicity::format_json(&report)),
                _ => print!("{}", vani::acyclicity::format_text(&report)),
            }
            if report.has_violations() {
                Ok(ExitCode::from(1))
            } else {
                Ok(ExitCode::SUCCESS)
            }
        }
        "audit-pack" => {
            // Meta audit: run all six per-fact CLIs (deviations,
            // stack-depth, acyclicity, hashmap-usage, complexity,
            // safety-attrs) against the program and bundle their
            // outputs into a single Markdown report. The
            // reviewer-facing audit-pack deliverable.
            //
            // Usage: vanic audit-pack <path> [--out=<file>]
            //          [--max-stack=<bytes>] [--max-complexity=<N>]
            // Exit 1 if any hard-gate exceeds its bound.
            let mut out_path: Option<String> = None;
            let mut max_stack: Option<u64> = None;
            let mut max_complexity: Option<u64> = None;
            let mut path_arg: Option<String> = None;
            let mut idx = 2;
            while idx < args.len() {
                let arg = &args[idx];
                if let Some(value) = arg.strip_prefix("--out=") {
                    out_path = Some(value.to_string());
                } else if let Some(value) = arg.strip_prefix("--max-stack=") {
                    max_stack = Some(value.parse::<u64>().map_err(|_| {
                        format!("--max-stack=<bytes> expects an integer, got '{}'", value)
                    })?);
                } else if let Some(value) = arg.strip_prefix("--max-complexity=") {
                    max_complexity = Some(value.parse::<u64>().map_err(|_| {
                        format!("--max-complexity=<N> expects an integer, got '{}'", value)
                    })?);
                } else if arg.starts_with('-') {
                    return Err(format!("unexpected argument '{}'", arg));
                } else if path_arg.is_none() {
                    path_arg = Some(arg.clone());
                } else {
                    return Err("'audit-pack' takes one path argument".into());
                }
                idx += 1;
            }
            let path = path_arg.ok_or_else(|| {
                "'audit-pack' requires a path argument".to_string()
            })?;
            let path_buf = std::path::PathBuf::from(&path);
            let (checked, file_map) = vani::compile_path(&path_buf)
                .map_err(|(map, diagnostics)| {
                    vani::diagnostic::format_diagnostics_with_files(&map, &diagnostics)
                })?;

            // Compute every report up front so the summary
            // section at the top can reference the aggregate
            // numbers without re-running.
            let devs = vani::deviations::extract_deviations(&checked.ir, &file_map);
            let stack_report = vani::stack_depth::compute_stack_depths(&checked.ir, None);
            let (stack_text, stack_over) =
                vani::stack_depth::format_text(&stack_report, max_stack);
            let acyclicity_report = vani::acyclicity::check_acyclicity(&checked.ir);
            let pairs = vani::hashmap_bundle::collect_hashmap_pairs(&checked.ir);
            let complexity_report =
                vani::safety::compute_complexity_report(&checked.ir);
            let (complexity_text, complexity_over) =
                vani::safety::format_complexity_text(&complexity_report, max_complexity);
            let safety_report =
                vani::safety::compute_safety_attrs_report(&checked.ir);

            // Summary aggregates for the at-a-glance table.
            let max_complexity_score: u64 = complexity_report
                .iter().map(|r| r.score).max().unwrap_or(0);
            let max_stack_bytes: Option<u64> = stack_report
                .entries.iter().filter_map(|e| e.max_depth_bytes).max();
            let any_unbounded_stack = stack_report
                .entries.iter().any(|e| e.max_depth_bytes.is_none());
            let acyclicity_violations = acyclicity_report
                .cycles.iter().filter(|c| !c.all_bounded).count();
            let tagged_fns = safety_report.iter().filter(|r| {
                r.safety_standard.is_some() || r.no_heap || r.no_float
                    || r.no_recursion || r.interrupt || r.deterministic_timing
                    || r.bounded_stack_bytes.is_some() || r.wcet_cycles.is_some()
                    || r.bounded_recursion.is_some() || r.is_pure
            }).count();

            let mut md = String::new();
            md.push_str(&format!("# Audit pack: `{}`\n\n", path));
            md.push_str("Generated by `vanic audit-pack`. At-a-glance summary followed by six per-fact reports.\n\n");

            // Summary table at the top.
            md.push_str("## Summary\n\n");
            md.push_str("| Metric | Value |\n");
            md.push_str("|---|---|\n");
            md.push_str(&format!("| Unsafe deviations | {} |\n", devs.len()));
            md.push_str(&format!(
                "| Max stack depth (bytes) | {} |\n",
                if any_unbounded_stack {
                    "UNBOUNDED".to_string()
                } else {
                    max_stack_bytes.map(|b| b.to_string()).unwrap_or_else(|| "0".to_string())
                }
            ));
            md.push_str(&format!(
                "| Call-graph cycles | {} ({} violation{}) |\n",
                acyclicity_report.cycles.len(),
                acyclicity_violations,
                if acyclicity_violations == 1 { "" } else { "s" },
            ));
            md.push_str(&format!("| HashMap<K, V> shapes | {} |\n", pairs.len()));
            md.push_str(&format!("| Max complexity (McCabe) | {} |\n", max_complexity_score));
            md.push_str(&format!(
                "| Functions with safety annotations | {} of {} |\n",
                tagged_fns,
                safety_report.len(),
            ));
            md.push_str("\n");

            md.push_str("## Deviations (`unsafe(reason = \"…\")` blocks)\n\n```\n");
            md.push_str(&vani::deviations::format_text(&devs));
            md.push_str("```\n\n");

            md.push_str("## Stack-depth\n\n```\n");
            md.push_str(&stack_text);
            md.push_str("```\n\n");

            md.push_str("## Call-graph acyclicity\n\n```\n");
            md.push_str(&vani::acyclicity::format_text(&acyclicity_report));
            md.push_str("```\n\n");

            md.push_str("## HashMap<K, V> instantiations\n\n```\n");
            md.push_str(&vani::hashmap_bundle::format_text(&pairs));
            md.push_str("```\n\n");

            md.push_str("## Cyclomatic complexity (McCabe)\n\n```\n");
            md.push_str(&complexity_text);
            md.push_str("```\n\n");

            md.push_str("## Safety attributes\n\n```\n");
            md.push_str(&vani::safety::format_safety_attrs_text(&safety_report));
            md.push_str("```\n");

            match out_path {
                Some(p) => {
                    std::fs::write(&p, &md).map_err(|e| {
                        format!("audit-pack: failed to write {}: {}", p, e)
                    })?;
                }
                None => print!("{}", md),
            }

            let any_violation = acyclicity_report.has_violations()
                || (max_stack.is_some() && stack_over)
                || (max_complexity.is_some() && complexity_over);
            if any_violation {
                Ok(ExitCode::from(1))
            } else {
                Ok(ExitCode::SUCCESS)
            }
        }
        "safety-attrs" => {
            // Cross-tier follow-up to the safety-standard arc:
            // per-function listing of all safety annotations.
            // Compliance reviewers can use this as the entry-
            // point audit document — every fn's tag set
            // (composite + primitives + budgets) in a single
            // machine-readable view, instead of grepping source.
            //
            // Usage: vanic safety-attrs <path> [--format=text|json|csv]
            let mut format = "text";
            let mut path_arg: Option<String> = None;
            let mut idx = 2;
            while idx < args.len() {
                let arg = &args[idx];
                if let Some(value) = arg.strip_prefix("--format=") {
                    format = match value {
                        "csv" => "csv",
                        "json" => "json",
                        "text" => "text",
                        other => {
                            return Err(format!(
                                "unsupported --format='{}'; choose csv | json | text",
                                other
                            ));
                        }
                    };
                } else if arg.starts_with('-') {
                    return Err(format!("unexpected argument '{}'", arg));
                } else if path_arg.is_none() {
                    path_arg = Some(arg.clone());
                } else {
                    return Err("'safety-attrs' takes one path argument".into());
                }
                idx += 1;
            }
            let path = path_arg.ok_or_else(|| {
                "'safety-attrs' requires a path argument".to_string()
            })?;
            let path = std::path::PathBuf::from(path);
            let (checked, _file_map) = vani::compile_path(&path)
                .map_err(|(map, diagnostics)| {
                    vani::diagnostic::format_diagnostics_with_files(&map, &diagnostics)
                })?;
            let report = vani::safety::compute_safety_attrs_report(&checked.ir);
            match format {
                "csv" => print!("{}", vani::safety::format_safety_attrs_csv(&report)),
                "json" => print!("{}", vani::safety::format_safety_attrs_json(&report)),
                _ => print!("{}", vani::safety::format_safety_attrs_text(&report)),
            }
            Ok(ExitCode::SUCCESS)
        }
        "complexity" => {
            // T2.4 follow-up: surface McCabe cyclomatic complexity
            // per function as an audit artifact. The same
            // counter that the `enforce_complexity` checker pass
            // uses (when opted in via env vars) is now also
            // queryable standalone via the CLI. Useful for MISRA /
            // ISO 26262 / DO-178C reviews against complexity
            // ceilings.
            //
            // Usage: vanic complexity <path>
            //          [--max=<N>] [--format=csv|json|text]
            // With --max, exits 1 if any fn's score exceeds N.
            let mut format = "text";
            let mut max_score: Option<u64> = None;
            let mut path_arg: Option<String> = None;
            let mut idx = 2;
            while idx < args.len() {
                let arg = &args[idx];
                if let Some(value) = arg.strip_prefix("--format=") {
                    format = match value {
                        "csv" => "csv",
                        "json" => "json",
                        "text" => "text",
                        other => {
                            return Err(format!(
                                "unsupported --format='{}'; choose csv | json | text",
                                other
                            ));
                        }
                    };
                } else if let Some(value) = arg.strip_prefix("--max=") {
                    max_score = Some(value.parse::<u64>().map_err(|_| {
                        format!("--max=<N> expects a non-negative integer, got '{}'", value)
                    })?);
                } else if arg.starts_with('-') {
                    return Err(format!("unexpected argument '{}'", arg));
                } else if path_arg.is_none() {
                    path_arg = Some(arg.clone());
                } else {
                    return Err("'complexity' takes one path argument".into());
                }
                idx += 1;
            }
            let path = path_arg.ok_or_else(|| {
                "'complexity' requires a path argument".to_string()
            })?;
            let path = std::path::PathBuf::from(path);
            let (checked, _file_map) = vani::compile_path(&path)
                .map_err(|(map, diagnostics)| {
                    vani::diagnostic::format_diagnostics_with_files(&map, &diagnostics)
                })?;
            let report = vani::safety::compute_complexity_report(&checked.ir);
            match format {
                "csv" => {
                    print!("{}", vani::safety::format_complexity_csv(&report));
                    Ok(ExitCode::SUCCESS)
                }
                "json" => {
                    print!("{}", vani::safety::format_complexity_json(&report));
                    Ok(ExitCode::SUCCESS)
                }
                _ => {
                    let (out, any_over) =
                        vani::safety::format_complexity_text(&report, max_score);
                    print!("{}", out);
                    if any_over && max_score.is_some() {
                        Ok(ExitCode::from(1))
                    } else {
                        Ok(ExitCode::SUCCESS)
                    }
                }
            }
        }
        "hashmap-usage" => {
            // ARC 1.3 follow-up: surface every HashMap<K, V> pair
            // appearing in the typed program as an audit artifact.
            // Each row carries the K type, V type, and mangled tag
            // used by the per-(K, V) bundle emitters in ARC 1.4/1.5.
            // Useful for embedded teams reviewing HashMap shapes
            // before sign-off, and for compilers shipping
            // per-(K, V) bundle emission to inspect what shapes a
            // program actually uses.
            //
            // Usage: vanic hashmap-usage <path> [--format=csv|json|text]
            let mut format = "text";
            let mut path_arg: Option<String> = None;
            let mut idx = 2;
            while idx < args.len() {
                let arg = &args[idx];
                if let Some(value) = arg.strip_prefix("--format=") {
                    format = match value {
                        "csv" => "csv",
                        "json" => "json",
                        "text" => "text",
                        other => {
                            return Err(format!(
                                "unsupported --format='{}'; choose csv | json | text",
                                other
                            ));
                        }
                    };
                } else if arg.starts_with('-') {
                    return Err(format!("unexpected argument '{}'", arg));
                } else if path_arg.is_none() {
                    path_arg = Some(arg.clone());
                } else {
                    return Err("'hashmap-usage' takes one path argument".into());
                }
                idx += 1;
            }
            let path = path_arg.ok_or_else(|| {
                "'hashmap-usage' requires a path argument".to_string()
            })?;
            let path = std::path::PathBuf::from(path);
            let (checked, _file_map) = vani::compile_path(&path)
                .map_err(|(map, diagnostics)| {
                    vani::diagnostic::format_diagnostics_with_files(&map, &diagnostics)
                })?;
            let pairs = vani::hashmap_bundle::collect_hashmap_pairs(&checked.ir);
            match format {
                "csv" => print!("{}", vani::hashmap_bundle::format_csv(&pairs)),
                "json" => print!("{}", vani::hashmap_bundle::format_json(&pairs)),
                _ => print!("{}", vani::hashmap_bundle::format_text(&pairs)),
            }
            Ok(ExitCode::SUCCESS)
        }
        "deviations" => {
            // T1.1 of the safety-standard alignment arc: extract
            // every `unsafe(reason = "…")` block as a structured
            // deviation record. The artifact reviewers need for
            // ASIL-D / DO-178C / IEC 62304 / MISRA sign-off.
            //
            // Usage: vanic deviations <path> [--format=csv|json|text] [--out=<file>]
            // Defaults: --format=text, --out=stdout.
            let mut format = "text";
            let mut out_path: Option<String> = None;
            let mut path_arg: Option<String> = None;
            let mut idx = 2;
            while idx < args.len() {
                let arg = &args[idx];
                if let Some(value) = arg.strip_prefix("--format=") {
                    format = match value {
                        "csv" => "csv",
                        "json" => "json",
                        "text" => "text",
                        other => {
                            return Err(format!(
                                "unsupported --format='{}'; choose csv | json | text",
                                other
                            ));
                        }
                    };
                } else if let Some(value) = arg.strip_prefix("--out=") {
                    out_path = Some(value.to_string());
                } else if arg.starts_with('-') {
                    return Err(format!("unexpected argument '{}'", arg));
                } else if path_arg.is_none() {
                    path_arg = Some(arg.clone());
                } else {
                    return Err("'deviations' takes one path argument".into());
                }
                idx += 1;
            }
            let path = path_arg.ok_or_else(|| {
                "'deviations' requires a path argument".to_string()
            })?;
            let path = std::path::PathBuf::from(path);
            let (checked, file_map) = vani::compile_path(&path)
                .map_err(|(map, diagnostics)| {
                    vani::diagnostic::format_diagnostics_with_files(&map, &diagnostics)
                })?;
            let deviations = vani::deviations::extract_deviations(
                &checked.ir,
                &file_map,
            );
            let output = match format {
                "csv" => vani::deviations::format_csv(&deviations),
                "json" => vani::deviations::format_json(&deviations),
                _ => vani::deviations::format_text(&deviations),
            };
            match out_path {
                Some(p) => {
                    fs::write(&p, &output).map_err(|e| {
                        format!("failed to write '{}': {}", p, e)
                    })?;
                }
                None => print!("{}", output),
            }
            Ok(ExitCode::SUCCESS)
        }
        "test" => {
            // Treat each path as a self-contained test case: compile +
            // run it through the LLVM backend, capturing stdout/stderr.
            // A test "passes" iff the program exits 0 (i.e. no `assert`
            // fired, no runtime guard tripped, no proof obligation
            // remained unsatisfied at runtime). Output per file plus a
            // summary line; exit 1 if any failed. A directory arg
            // expands to its `*.vani` children (non-recursive).
            if args.len() < 3 {
                return Err("test requires at least one source file\n\n".to_string() + HELP);
            }
            // Split flags from path args. Supported: --smt-debug
            // and --json. The JSON form is machine-readable for CI;
            // a single object on stdout, no per-file lines.
            let mut path_args: Vec<String> = Vec::new();
            let mut json = false;
            for arg in args.iter().skip(2) {
                match arg.as_str() {
                    "--smt-debug" => {
                        std::env::set_var("VANIC_SMT_DEBUG", "1");
                    }
                    "--json" => json = true,
                    other if other.starts_with('-') => {
                        return Err(format!(
                            "unknown flag for 'test': '{}' (expected --smt-debug, --json)",
                            other
                        ));
                    }
                    other => path_args.push(other.to_string()),
                }
            }
            let files = expand_intent_paths(&path_args)?;
            if files.is_empty() {
                return Err("no .vani files to test".into());
            }
            let mut passed = 0usize;
            let mut failed = 0usize;
            // For --json mode we collect per-file outcomes and emit
            // one object at the end. Each result records ok-ness,
            // elapsed ms, and (for failures) the exit code + a
            // brief reason. We deliberately do NOT include
            // stdout/stderr in the JSON to keep the payload small —
            // the human-readable form prints them on FAILED.
            let mut json_results: Vec<String> = Vec::new();
            for path in &files {
                let start = std::time::Instant::now();
                let result = run_program_llvm_capture(path);
                let elapsed = start.elapsed().as_millis();
                let path_str = json_escape(&path.display().to_string());
                match result {
                    Ok((0, _, _)) => {
                        if !json {
                            println!("{}: ok ({} ms)", path.display(), elapsed);
                        }
                        json_results.push(format!(
                            "{{\"path\":\"{}\",\"ok\":true,\"ms\":{}}}",
                            path_str, elapsed
                        ));
                        passed += 1;
                    }
                    Ok((code, stdout, stderr)) => {
                        if !json {
                            println!(
                                "{}: FAILED (exit {}, {} ms)",
                                path.display(),
                                code,
                                elapsed
                            );
                            if !stdout.is_empty() {
                                eprintln!("--- stdout ---\n{}", stdout);
                            }
                            let stderr = trim_lli_backtrace(&stderr);
                            if !stderr.is_empty() {
                                eprintln!("--- stderr ---\n{}", stderr);
                            }
                        }
                        json_results.push(format!(
                            "{{\"path\":\"{}\",\"ok\":false,\"ms\":{},\"exit\":{},\"reason\":\"runtime\"}}",
                            path_str, elapsed, code
                        ));
                        failed += 1;
                    }
                    Err(msg) => {
                        if !json {
                            println!("{}: FAILED (compile, {} ms)", path.display(), elapsed);
                            eprintln!("{}", msg);
                        }
                        json_results.push(format!(
                            "{{\"path\":\"{}\",\"ok\":false,\"ms\":{},\"reason\":\"compile\"}}",
                            path_str, elapsed
                        ));
                        failed += 1;
                    }
                }
            }
            if json {
                println!(
                    "{{\"results\":[{}],\"summary\":{{\"passed\":{},\"failed\":{}}}}}",
                    json_results.join(","),
                    passed,
                    failed
                );
            } else {
                println!();
                println!("{} passed; {} failed", passed, failed);
            }
            if failed > 0 {
                Ok(ExitCode::from(1))
            } else {
                Ok(ExitCode::SUCCESS)
            }
        }
        "fmt" => {
            // Pretty-print canonical source. `// …` comments are
            // preserved (best-effort: trailing same-line comments
            // are promoted to leading; blank lines between comment
            // groups are not preserved in v1).
            //
            // Modes (mutually exclusive):
            //   default:     print formatted source to stdout
            //                (single-file only)
            //   --check:     exit 1 (silent) if any file is not
            //                already canonical; useful for CI
            //   --in-place:  overwrite each file with canonical
            //                source
            //
            // Path args may be files or directories. Directories
            // expand to their `*.vani` children (non-recursive)
            // via the same helper used by `vanic test`.
            let mut check = false;
            let mut in_place = false;
            let mut path_args: Vec<String> = Vec::new();
            for arg in args.iter().skip(2) {
                match arg.as_str() {
                    "--check" => check = true,
                    "--in-place" | "-i" => in_place = true,
                    other if other.starts_with('-') => {
                        return Err(format!(
                            "unknown flag for 'fmt': '{}' (expected --check or --in-place)",
                            other
                        ));
                    }
                    other => path_args.push(other.to_string()),
                }
            }
            if check && in_place {
                return Err("--check and --in-place are mutually exclusive".into());
            }
            if path_args.is_empty() {
                return Err(format!("'fmt' requires a path argument\n\n{}", HELP));
            }
            let files = expand_intent_paths(&path_args)?;
            if files.is_empty() {
                return Err("no .vani files to format".into());
            }
            if files.len() > 1 && !check && !in_place {
                return Err(
                    "multiple files require --check or --in-place \
                     (stdout mode is single-file only)"
                        .into(),
                );
            }

            let mut not_canonical = 0usize;
            for file in &files {
                let source = fs::read_to_string(file).map_err(|error| {
                    format!("failed to read '{}': {}", file.display(), error)
                })?;
                let tokens = vani::lexer::lex(&source).map_err(|diag| {
                    vani::diagnostic::format_diagnostics(
                        file.to_str().unwrap_or("<input>"),
                        &source,
                        &[diag],
                    )
                })?;
                let comments = vani::lexer::extract_comments(&source);
                let (program, diags) = vani::parser::parse(tokens);
                if !diags.is_empty() {
                    return Err(vani::diagnostic::format_diagnostics(
                        file.to_str().unwrap_or("<input>"),
                        &source,
                        &diags,
                    ));
                }
                let formatted = vani::format::format_program_with_comments(
                    &program, &source, &comments,
                );

                if check {
                    if formatted != source {
                        eprintln!("{}: not canonically formatted", file.display());
                        not_canonical += 1;
                    }
                } else if in_place {
                    // Only write if content actually changes — keeps
                    // mtime stable for files already canonical,
                    // making `vanic fmt --in-place examples/`
                    // safe to run repeatedly.
                    if formatted != source {
                        fs::write(file, &formatted).map_err(|e| {
                            format!("failed to write '{}': {}", file.display(), e)
                        })?;
                    }
                } else {
                    print!("{}", formatted);
                }
            }
            if check && not_canonical > 0 {
                Ok(ExitCode::from(1))
            } else {
                Ok(ExitCode::SUCCESS)
            }
        }
        // vanic apply-publisher [--accept-agreement]
        "apply-publisher" => {
            let accept = args.iter().skip(2).any(|a| a == "--accept-agreement");
            vani::manifest::apply_publisher(vani::manifest::DEFAULT_REGISTRY, accept)?;
            Ok(ExitCode::SUCCESS)
        }

        // vanic registry-approve <username>  [operator only]
        "registry-approve" => {
            let username = args.get(2).ok_or_else(|| {
                "usage: vanic registry-approve <username>".to_string()
            })?;
            vani::manifest::registry_approve(vani::manifest::DEFAULT_REGISTRY, username)?;
            Ok(ExitCode::SUCCESS)
        }

        // vanic registry-blacklist <username> [--reason=<text>]  [operator only]
        "registry-blacklist" => {
            let username = args.get(2).ok_or_else(|| {
                "usage: vanic registry-blacklist <username> [--reason=<text>]".to_string()
            })?;
            let reason = args.iter().skip(3)
                .find_map(|a| a.strip_prefix("--reason="))
                .unwrap_or("policy violation");
            vani::manifest::registry_blacklist(
                vani::manifest::DEFAULT_REGISTRY,
                username,
                reason,
            )?;
            Ok(ExitCode::SUCCESS)
        }

        // vanic publish
        // Build tarball, create GH Release in kosh-index, push NDJSON index line.
        "publish" => {
            let cwd = std::env::current_dir()
                .map_err(|e| format!("failed to read cwd: {}", e))?;
            let manifest_path = vani::manifest::find_manifest(&cwd)
                .ok_or_else(|| {
                    "no vani.toml found in current directory or any parent".to_string()
                })?;
            let result = vani::manifest::publish_package(
                &manifest_path,
                |msg| eprintln!("{}", msg),
            )?;
            eprintln!(
                "publish: {} v{} → {}",
                result.name, result.version, result.release_url
            );
            Ok(ExitCode::SUCCESS)
        }

        // vanic add <name>[@<version_constraint>]
        // Fetch a package from the Kosh registry, extract to vendor/,
        // update vani.toml + vani.lock.
        "add" => {
            if args.len() < 3 {
                return Err(
                    "usage: vanic add <name>[@<version>]\n\n\
                     Examples:\n  vanic add mathlib\n  vanic add mathlib@^1.0"
                        .to_string(),
                );
            }
            let pkg_arg = &args[2];
            let (pkg_name, version_constraint) = match pkg_arg.find('@') {
                Some(at) => (&pkg_arg[..at], Some(&pkg_arg[at + 1..])),
                None => (pkg_arg.as_str(), None),
            };
            let cwd = std::env::current_dir()
                .map_err(|e| format!("failed to read cwd: {}", e))?;
            let manifest_path = vani::manifest::find_manifest(&cwd)
                .ok_or_else(|| {
                    "no vani.toml found in current directory or any parent".to_string()
                })?;
            let result = vani::manifest::registry_add(
                &manifest_path,
                pkg_name,
                version_constraint,
                |msg| eprintln!("{}", msg),
            )?;
            eprintln!(
                "add: {} v{} → {}",
                result.name,
                result.version,
                result.vendor_path.display()
            );
            Ok(ExitCode::SUCCESS)
        }

        // vanic remove <name>
        // Remove a dep from vani.toml, delete vendor/<name>/, rewrite vani.lock.
        "remove" => {
            let pkg_name = args.get(2).ok_or_else(|| {
                "usage: vanic remove <name>".to_string()
            })?;
            let cwd = std::env::current_dir()
                .map_err(|e| format!("failed to read cwd: {}", e))?;
            let manifest_path = vani::manifest::find_manifest(&cwd).ok_or_else(|| {
                "no vani.toml found in current directory or any parent".to_string()
            })?;
            vani::manifest::registry_remove(&manifest_path, pkg_name, |msg| eprintln!("{}", msg))?;
            eprintln!("remove: {} removed from project", pkg_name);
            Ok(ExitCode::SUCCESS)
        }

        // vanic search [<query>]
        // Search the Kosh registry by name substring.
        "search" => {
            let query = args.get(2).map(String::as_str);
            eprintln!("  searching registry{}...", query.map(|q| format!(" for '{q}'")).unwrap_or_default());
            let results = vani::manifest::registry_search(
                vani::manifest::DEFAULT_REGISTRY,
                query,
            )?;
            if results.is_empty() {
                println!("no packages found{}", query.map(|q| format!(" matching '{q}'")).unwrap_or_default());
            } else {
                println!("{:<30} {:<12} {}", "NAME", "LATEST", "VERSIONS");
                for r in &results {
                    println!(
                        "{:<30} {:<12} {}",
                        r.name,
                        if r.latest_version.is_empty() { "-" } else { &r.latest_version },
                        r.version_count,
                    );
                }
                println!("\n{} package(s) found", results.len());
            }
            Ok(ExitCode::SUCCESS)
        }

        // vanic update
        // Re-resolve all registry deps to their latest compatible versions.
        "update" => {
            let cwd = std::env::current_dir()
                .map_err(|e| format!("failed to read cwd: {}", e))?;
            let manifest_path = vani::manifest::find_manifest(&cwd).ok_or_else(|| {
                "no vani.toml found in current directory or any parent".to_string()
            })?;
            let results = vani::manifest::registry_update(
                &manifest_path,
                |msg| eprintln!("{}", msg),
            )?;
            if results.is_empty() {
                eprintln!("update: no registry deps to update");
            } else {
                let updated: Vec<_> = results.iter().filter(|r| r.updated).collect();
                let up_to_date = results.len() - updated.len();
                for r in &updated {
                    eprintln!("  updated {} v{} → v{}", r.name, r.old_version, r.new_version);
                }
                eprintln!(
                    "update: {} updated, {} already up-to-date",
                    updated.len(), up_to_date
                );
            }
            Ok(ExitCode::SUCCESS)
        }

        // vanic vendor [--manifest=<path>]
        // Copy each dep's source tree into vendor/<name>/ and
        // write/update vani.lock.
        "vendor" => {
            let mut manifest_override: Option<PathBuf> = None;
            for arg in args.iter().skip(2) {
                if let Some(val) = arg.strip_prefix("--manifest=") {
                    manifest_override = Some(PathBuf::from(val));
                } else {
                    return Err(format!("unknown flag '{}'\n\nvanic vendor [--manifest=<path>]", arg));
                }
            }
            let manifest_path = if let Some(p) = manifest_override {
                p
            } else {
                let cwd = std::env::current_dir()
                    .map_err(|e| format!("failed to read cwd: {}", e))?;
                vani::manifest::find_manifest(&cwd)
                    .ok_or_else(|| "no vani.toml found in current directory or any parent".to_string())?
            };
            let manifest = vani::manifest::load_manifest(&manifest_path)
                .map_err(|e| e.to_string())?;
            if manifest.deps.is_empty() {
                eprintln!("nothing to vendor: [deps] is empty");
                return Ok(ExitCode::SUCCESS);
            }
            let vendored = vani::manifest::vendor_deps(&manifest)
                .map_err(|e| e.to_string())?;
            for (name, dest) in &vendored {
                eprintln!("  vendored {} -> {}", name, dest.display());
            }
            vani::manifest::write_lockfile(&manifest).map_err(|e| e.to_string())?;
            eprintln!(
                "vendor: {} package{} copied, vani.lock updated",
                vendored.len(),
                if vendored.len() == 1 { "" } else { "s" }
            );
            Ok(ExitCode::SUCCESS)
        }
        other => Err(format!("unknown command '{}'\n\n{}", other, HELP)),
    }
}

fn required_file(args: &[String], index: usize, command: &str) -> Result<PathBuf, String> {
    let (file, _next_idx) = required_file_at(args, index, command)?;
    Ok(file)
}

/// Like `required_file` but also returns the next arg index
/// to scan from. When a positional file is present at `index`
/// the next index is `index + 1`; when the file comes from
/// `vani.toml` (no positional consumed) the next index is
/// `index` itself so flag parsing sees every remaining arg.
/// Closure #280.
fn required_file_at(
    args: &[String],
    index: usize,
    command: &str,
) -> Result<(PathBuf, usize), String> {
    // Look for the first positional (non-flag) arg, skipping
    // flag pairs `-o PATH` / `--out PATH` / `--link-with PATH`
    // / `--backend=...` etc. Without this, `vanic build -o
    // out` with an implicit manifest entry would mis-read
    // `out` as the source path.
    let mut idx = index;
    while let Some(arg) = args.get(idx) {
        if arg == "-o" || arg == "--out" || arg == "--link-with" {
            idx += 2;
            continue;
        }
        if arg.starts_with('-') {
            idx += 1;
            continue;
        }
        // Found the positional file at `idx`. The caller
        // should start flag parsing from `idx + 1` (the arg
        // just after).
        return Ok((PathBuf::from(arg), idx + 1));
    }
    // No positional — try manifest discovery. The caller
    // should start flag parsing from `index` (no arg
    // consumed for the source).
    let cwd = std::env::current_dir()
        .map_err(|e| format!("failed to read cwd: {}", e))?;
    if let Some(manifest_path) = vani::manifest::find_manifest(&cwd) {
        let manifest = vani::manifest::load_manifest(&manifest_path)
            .map_err(|e| e.to_string())?;
        // Write vani.lock when absent or stale (manifest newer than lock).
        if vani::manifest::lockfile_is_stale(&manifest_path) {
            // Non-fatal: warn but don't abort the build.
            if let Err(e) = vani::manifest::write_lockfile(&manifest) {
                eprintln!("warning: could not write vani.lock: {}", e);
            }
        }
        return Ok((manifest.entry_path, index));
    }
    Err(format!(
        "'{}' requires a source file argument (or a `vani.toml` \
         manifest with [package].entry in cwd / a parent directory)\n\n{}",
        command, HELP
    ))
}

#[derive(Clone, Copy, Debug)]
enum BackendKind {
    C,
    Llvm,
}

/// Parse `[--backend=<c|llvm>] [-o path | --out path]` for the
/// `emit` subcommand. The legacy `emit-c` alias forces backend=c
/// and rejects --backend to keep its semantics unambiguous.
// FFI follow-up: `vanic build` accepts extra inputs that flow
// straight to the system linker (`cc`). Two shapes:
//   --link-with PATH   add an object/source file (e.g. foo.o, foo.c).
//                      Repeatable. Useful for `extern "C" fn` whose
//                      implementation lives in a separately-compiled
//                      C/C++/Rust translation unit.
//   -l<name>           add a system library (e.g. -lm, -lcurl).
//                      Repeatable. Forwarded verbatim to cc.
// Both are appended after the vāṇी object file in the link line so
// usual link-order rules apply.
// Closure #274: `vanic run` accepts the same link flags as
// `vanic build` (only the C-backend path actually consumes
// them — LLVM-JIT runs through lli's host-symbol resolver and
// can't link extra translation units). Returning the same
// (backend, link_args) shape so the dispatch can validate the
// combination.
fn parse_run_args(
    args: &[String],
    from: usize,
) -> Result<(BackendKind, Vec<String>, Option<vani::big_o::BigOMode>, Option<String>), String> {
    let mut backend = BackendKind::Llvm;
    let mut link_args: Vec<String> = Vec::new();
    let mut big_o_mode: Option<vani::big_o::BigOMode> = None;
    let mut target: Option<String> = None;
    let mut idx = from;
    while let Some(arg) = args.get(idx) {
        if let Some(value) = arg.strip_prefix("--backend=") {
            backend = match value {
                "c" => BackendKind::C,
                "llvm" => BackendKind::Llvm,
                other => return Err(format!("unknown backend '{}': expected c|llvm", other)),
            };
            idx += 1;
        } else if arg == "--link-with" {
            let path = args
                .get(idx + 1)
                .ok_or_else(|| "expected a path after '--link-with'".to_string())?;
            link_args.push(path.clone());
            idx += 2;
        } else if let Some(value) = arg.strip_prefix("--link-with=") {
            link_args.push(value.to_string());
            idx += 1;
        } else if arg.starts_with("-l") && arg.len() > 2 {
            link_args.push(arg.clone());
            idx += 1;
        } else if arg == "-o" || arg == "--out" {
            // `-o` is meaningless for run but the legacy parser
            // accepted it; preserve back-compat by consuming the
            // path arg without using it.
            let _ = args
                .get(idx + 1)
                .ok_or_else(|| format!("expected a path after '{}'", arg))?;
            idx += 2;
        } else if arg == "--big-o" {
            big_o_mode = Some(vani::big_o::BigOMode::Auto);
            idx += 1;
        } else if let Some(value) = arg.strip_prefix("--big-o=") {
            match vani::big_o::BigOMode::parse(value) {
                Some(m) => {
                    big_o_mode = Some(m);
                    idx += 1;
                }
                None => {
                    return Err(format!(
                        "unknown --big-o mode '{}'; expected auto|force|off",
                        value,
                    ));
                }
            }
        } else if let Some(triple) = arg.strip_prefix("--target=") {
            target = Some(triple.to_string());
            idx += 1;
        } else if arg == "--target" {
            let triple = args
                .get(idx + 1)
                .ok_or_else(|| "expected a triple after '--target'".to_string())?;
            target = Some(triple.clone());
            idx += 2;
        } else {
            return Err(format!("unexpected argument '{}'", arg));
        }
    }
    Ok((backend, link_args, big_o_mode, target))
}

fn parse_build_args(
    args: &[String],
    from: usize,
) -> Result<(Option<PathBuf>, Vec<String>, Option<String>), String> {
    let mut out: Option<PathBuf> = None;
    let mut link_args: Vec<String> = Vec::new();
    let mut target: Option<String> = None;
    let mut idx = from;
    while let Some(arg) = args.get(idx) {
        if arg == "-o" || arg == "--out" {
            let path = args
                .get(idx + 1)
                .ok_or_else(|| format!("expected a path after '{}'", arg))?;
            out = Some(PathBuf::from(path));
            idx += 2;
        } else if arg == "--link-with" {
            let path = args
                .get(idx + 1)
                .ok_or_else(|| "expected a path after '--link-with'".to_string())?;
            link_args.push(path.clone());
            idx += 2;
        } else if let Some(value) = arg.strip_prefix("--link-with=") {
            link_args.push(value.to_string());
            idx += 1;
        } else if arg.starts_with("-l") && arg.len() > 2 {
            link_args.push(arg.clone());
            idx += 1;
        } else if let Some(triple) = arg.strip_prefix("--target=") {
            target = Some(triple.to_string());
            idx += 1;
        } else if arg == "--target" {
            let triple = args
                .get(idx + 1)
                .ok_or_else(|| "expected a triple after '--target'".to_string())?;
            target = Some(triple.clone());
            idx += 2;
        } else if arg == "--no-std" {
            // Accepted here; build_program_llvm auto-activates no-std for
            // bare-metal triples, but the explicit flag is also honoured.
            idx += 1;
        } else {
            return Err(format!("unexpected argument '{}'", arg));
        }
    }
    Ok((out, link_args, target))
}

fn parse_emit_args(
    args: &[String],
    from: usize,
    cmd_name: &str,
) -> Result<(BackendKind, Option<PathBuf>, Option<vani::big_o::BigOMode>), String> {
    // LLVM is now the default — the project's direction is to move
    // away from the C backend. The `emit-c` legacy alias forces C
    // regardless of this default.
    let mut backend = if cmd_name == "emit-c" {
        BackendKind::C
    } else {
        BackendKind::Llvm
    };
    let mut out: Option<PathBuf> = None;
    let mut big_o_mode: Option<vani::big_o::BigOMode> = None;
    let mut idx = from;
    while let Some(arg) = args.get(idx) {
        if let Some(value) = arg.strip_prefix("--backend=") {
            if cmd_name == "emit-c" {
                return Err(
                    "'emit-c' forces backend=c; use 'emit --backend=…' to choose"
                        .to_string(),
                );
            }
            backend = match value {
                "c" => BackendKind::C,
                "llvm" => BackendKind::Llvm,
                other => return Err(format!("unknown backend '{}': expected c|llvm", other)),
            };
            idx += 1;
        } else if arg == "-o" || arg == "--out" {
            let path = args
                .get(idx + 1)
                .ok_or_else(|| format!("expected a path after '{}'", arg))?;
            out = Some(PathBuf::from(path));
            idx += 2;
        } else if arg == "--big-o" {
            big_o_mode = Some(vani::big_o::BigOMode::Auto);
            idx += 1;
        } else if let Some(value) = arg.strip_prefix("--big-o=") {
            match vani::big_o::BigOMode::parse(value) {
                Some(m) => {
                    big_o_mode = Some(m);
                    idx += 1;
                }
                None => {
                    return Err(format!(
                        "unknown --big-o mode '{}'; expected auto|force|off",
                        value,
                    ));
                }
            }
        } else {
            return Err(format!("unexpected argument '{}'", arg));
        }
    }
    Ok((backend, out, big_o_mode))
}

fn compile_path_or_report(
    _path: &Path,
) -> Result<vani::checker::CheckedProgram, String> {
    vani::compile_path(_path)
        .map(|(c, _)| c)
        .map_err(|(map, diagnostics)| {
            vani::diagnostic::format_diagnostics_with_files(&map, &diagnostics)
        })
}

fn run_program(
    path: &Path,
    link_args: &[String],
    big_o_mode: Option<vani::big_o::BigOMode>,
) -> Result<ExitCode, String> {
    let checked = compile_path_or_report(path)?;
    if let Some(mode) = big_o_mode {
        if mode != vani::big_o::BigOMode::Off {
            for (name, complexity) in vani::big_o::annotate_program(&checked.ir, mode) {
                eprintln!("  fn {}: {}", name, complexity);
            }
        }
    }
    let c = emit_c_via_ssa(&checked.ir);
    let (c_path, bin_path) = temp_paths(path);

    fs::write(&c_path, c)
        .map_err(|error| format!("failed to write '{}': {}", c_path.display(), error))?;

    let cc = env::var("CC").unwrap_or_else(|_| "cc".to_string());
    // Probe once for `-fopenmp` support and add it when available
    // so `parallel for` loops in the source get actual parallelism.
    // Compilers without OpenMP issue an "unknown pragma" warning
    // and run sequentially — also correct (the verifier proved the
    // body is independent of iteration order).
    let openmp_ok = Command::new(&cc)
        .args(["-fopenmp", "-x", "c", "-E", "-"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    let mut cmd = Command::new(&cc);
    cmd.arg(&c_path)
        .arg("-std=c11")
        .arg("-O2")
        // Enable cross-function inlining (the part of -O3 relevant to
        // recursive/trivial-base-case functions like fib). With -O2 alone
        // gcc only inlines functions explicitly marked `inline`; this flag
        // lets it inline small callees based on cost heuristics, closing
        // the ~9% gap vs hand-written C on recursive benchmarks.
        .arg("-finline-functions")
        // Enable the auto-vectoriser (also part of -O3 but absent from -O2).
        // Works with the __restrict__ data pointers in every Vec struct and
        // the _Pragma("GCC ivdep") emitted before every loop: gcc can use
        // SSE2/AVX2/AVX-512 SIMD for Vec-element loops (matmul inner col-loop,
        // sieve mark-composites, stats accumulation) without the aliasing
        // ambiguity that blocks vectorisation in the absence of restrict.
        .arg("-ftree-vectorize")
        // Unlock CPU-native instruction set (AVX-512F/BW/DQ/VL + FMA3 on
        // Ice Lake i5-1035G1). Key wins:
        //   matmul SAXPY:  c += a*b → single vfmadd231epi64 (FMA, 1 insn vs 2)
        //   sieve inner:   8-wide AVX-512 i64 stores vs 4-wide AVX2
        //   general:       better uarch scheduling for the specific CPU model
        // Safe because benchmarks compile and run on the same machine.
        .arg("-march=native")
        // Free the frame-pointer register (rbp) for use as a general-purpose
        // register in tight loops. Saves 1 instruction per function entry/exit
        // and reduces register spills in register-pressure inner loops.
        .arg("-fomit-frame-pointer");
    // Layer 4.1 of `unsafe.md` — stack canaries. Opt-in via the
    // same embedded gate as Layer 1.1 / 1.2. `-fstack-protector-
    // strong` catches stack-smashing in any function with
    // buffers, allocas, or string operations at ~2 instructions
    // per frame. Free defense-in-depth when the build target is
    // embedded; held back from default hosted builds so we
    // don't perturb the existing parity / perf baseline.
    apply_embedded_cc_hardening(&mut cmd);
    // Link pthread on POSIX so the `task` lowering's
    // pthread_create / pthread_join references resolve.
    // glibc folds -lpthread into libc on modern systems;
    // -pthread is the portable spelling and is also a
    // no-op when libgomp already brings pthread in via
    // -fopenmp. On Windows the runtime uses CreateThread
    // (kernel32.lib is linked by default) and
    // WaitOnAddress / WakeByAddressSingle (kernel32 +
    // synchronization.lib).
    if !cfg!(target_os = "windows") {
        cmd.arg("-pthread");
        // Link libm so libm symbols emitted by the math
        // builtins (sqrt / sin / cos / pow / floor / ceil
        // / fabs) resolve at link time. glibc keeps the
        // math functions in libm; modern Apple SDKs / BSDs
        // ship the same set in libm. Windows has the math
        // functions in the C runtime (msvcrt) — no extra
        // flag needed. Closure #299.
        cmd.arg("-lm");
    } else {
        cmd.arg("-lsynchronization");
        // Winsock2 for socket / bind / listen / accept / recv / send
        // / closesocket. On Windows the pragma comment(lib,...) in
        // the emitted C is MSVC-only; MinGW/gcc needs an explicit flag.
        cmd.arg("-lws2_32");
    }
    if openmp_ok {
        cmd.arg("-fopenmp");
    }
    // Closure #274: user-supplied link inputs (`--link-with PATH`
    // / `-l<name>`) trail the vāṇी source so symbol resolution
    // sees vāṇी's `call abs(...)` first and then the providing
    // object / library.
    for extra in link_args {
        cmd.arg(extra);
    }
    let compile_out = cmd
        .arg("-o")
        .arg(&bin_path)
        .output()
        .map_err(|error| format!("failed to invoke {}: {}", cc, error))?;

    if !compile_out.status.success() {
        return Err(format!(
            "{} failed while compiling '{}' (left at this path for debugging):\n{}",
            cc,
            c_path.display(),
            String::from_utf8_lossy(&compile_out.stderr).trim_end()
        ));
    }

    let run_result = Command::new(&bin_path).status();
    let _ = fs::remove_file(&c_path);
    let _ = fs::remove_file(&bin_path);
    let status = run_result
        .map_err(|error| format!("failed to run '{}': {}", bin_path.display(), error))?;

    Ok(ExitCode::from(status.code().unwrap_or(1) as u8))
}

/// LLVM equivalent of `run_program`. Emits `.ll`, runs it through
/// `lli`, returns the program's exit code. `LLI` env var overrides
/// the default `lli` binary lookup, mirroring `CC` for the C path.
fn run_program_llvm(
    path: &Path,
    big_o_mode: Option<vani::big_o::BigOMode>,
) -> Result<ExitCode, String> {
    let checked = compile_path_or_report(path)?;
    if let Some(mode) = big_o_mode {
        if mode != vani::big_o::BigOMode::Off {
            for (name, complexity) in vani::big_o::annotate_program(&checked.ir, mode) {
                eprintln!("  fn {}: {}", name, complexity);
            }
        }
    }
    let ll = emit_llvm_via_ssa(&checked.ir);
    let stem = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("program");
    let pid = std::process::id();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let ll_path = env::temp_dir().join(format!("vanic-{}-{}-{}.ll", stem, pid, nanos));
    fs::write(&ll_path, ll)
        .map_err(|error| format!("failed to write '{}': {}", ll_path.display(), error))?;

    let lli = env::var("LLI").unwrap_or_else(|_| "lli".to_string());
    let mut cmd = Command::new(&lli);
    // The LLVM backend emits `parallel for` lowerings that call
    // GOMP_parallel / omp_get_thread_num / omp_get_num_threads
    // from libgomp.so. Probe the well-known soname; if present,
    // tell lli to load it so JIT calls resolve. When absent, the
    // OpenMP entries are unresolved but only get called by
    // `parallel for` sites — pure-sequential programs still run.
    add_libgomp_load_flags(&mut cmd);
    // lli's MCJIT isn't thread-safe for concurrent function
    // resolution; cap libgomp to a single thread when JITting so
    // `parallel for` runs serially under the JIT. AOT builds
    // (`vanic build`) leave the env alone and get real
    // parallelism. Users who want JIT'd parallelism can override.
    if env::var("OMP_NUM_THREADS").is_err() {
        cmd.env("OMP_NUM_THREADS", "1");
    }
    cmd.arg(&ll_path);
    let run_result = cmd.status();
    let _ = fs::remove_file(&ll_path);
    let status = run_result
        .map_err(|error| format!("failed to invoke {}: {}", lli, error))?;
    Ok(ExitCode::from(status.code().unwrap_or(1) as u8))
}

/// Probe known libgomp.so paths and add `-load=<path>` flags to
/// `cmd` for each one that exists. lli silently ignores duplicates
/// and unknown paths. Order matters only when symbols collide,
/// which they don't between libgomp versions.
fn add_libgomp_load_flags(cmd: &mut Command) {
    const CANDIDATES: &[&str] = &[
        "/usr/lib/x86_64-linux-gnu/libgomp.so.1",
        "/lib/x86_64-linux-gnu/libgomp.so.1",
        "/usr/lib64/libgomp.so.1",
        "/usr/lib/aarch64-linux-gnu/libgomp.so.1",
        // Mac (Homebrew clang's libomp.dylib also works because
        // lli on macOS can resolve both libgomp and libomp).
        "/opt/homebrew/opt/libomp/lib/libomp.dylib",
        "/usr/local/opt/libomp/lib/libomp.dylib",
    ];
    for path in CANDIDATES {
        if std::path::Path::new(path).exists() {
            cmd.arg(format!("-load={}", path));
            return;
        }
    }
    // `INTENT_LIBGOMP` env override for non-standard paths.
    if let Ok(p) = env::var("INTENT_LIBGOMP") {
        if std::path::Path::new(&p).exists() {
            cmd.arg(format!("-load={}", p));
        }
    }
}

/// Drop lli's signal-handler diagnostics from a captured stderr.
/// When an Intent program aborts (failed assert, divisor=0, etc.),
/// lli intercepts SIGABRT and dumps "PLEASE submit a bug report",
/// "Stack dump:", and a long native backtrace. None of that is
/// useful to an Intent user — the line that *is* useful (e.g.
/// `assertion failed: ...`) was printed earlier by the program
/// itself. Truncate at the first lli-internal marker.
/// Resolve a list of CLI args into a flat list of `.vani` files,
/// shared by `vanic test` and `vanic fmt`. Each arg is treated
/// as a file or a directory; a directory expands recursively to
/// every `*.vani` descendant, alphabetized. Dot-prefixed
/// directories (`.git`, `.cargo`, etc.) are skipped.
fn expand_intent_paths(args: &[String]) -> Result<Vec<PathBuf>, String> {
    let mut files: Vec<PathBuf> = Vec::new();
    for raw in args {
        let path = PathBuf::from(raw);
        if path.is_dir() {
            walk_intent_files(&path, &mut files).map_err(|e| {
                format!("failed to read directory '{}': {}", path.display(), e)
            })?;
        } else {
            files.push(path);
        }
    }
    Ok(files)
}

/// Append every `*.vani` file under `dir` to `out`, recursing
/// into subdirectories in alphabetical order. Skips entries whose
/// name starts with `.` so `vanic fmt --check .` doesn't drill
/// into `.git/`, `.cargo/`, etc.
fn walk_intent_files(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    let mut entries: Vec<_> = fs::read_dir(dir)?.filter_map(Result::ok).collect();
    entries.sort_by_key(|e| e.path());
    for entry in entries {
        let path = entry.path();
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            walk_intent_files(&path, out)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("vani") {
            out.push(path);
        }
    }
    Ok(())
}

/// Minimal JSON-string escaping for paths and short reason strings
/// embedded in `vanic test --json` output. Just the basics: `\"`,
/// `\\`, control chars escaped as `\uXXXX`. We don't pull in a
/// JSON-emitter crate for this — the entire `--json` payload is
/// hand-shaped, mirroring `format_diagnostics_json_with_files`.
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn trim_lli_backtrace(stderr: &str) -> String {
    const MARKERS: &[&str] = &["PLEASE submit a bug report", "Stack dump:"];
    let mut cut = stderr.len();
    for m in MARKERS {
        if let Some(idx) = stderr.find(m) {
            if idx < cut {
                cut = idx;
            }
        }
    }
    let trimmed = stderr[..cut].trim_end();
    if trimmed.is_empty() {
        String::new()
    } else {
        format!("{}\n", trimmed)
    }
}

/// Like `run_program_llvm` but captures stdout+stderr instead of
/// inheriting the parent's. Returns `(exit_code, stdout, stderr)` so
/// callers (notably `vanic test`) can decide whether to show output.
fn run_program_llvm_capture(path: &Path) -> Result<(i32, String, String), String> {
    let checked = compile_path_or_report(path)?;
    let ll = emit_llvm_via_ssa(&checked.ir);
    let stem = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("program");
    let pid = std::process::id();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let ll_path = env::temp_dir().join(format!("vanic-{}-{}-{}.ll", stem, pid, nanos));
    fs::write(&ll_path, ll)
        .map_err(|error| format!("failed to write '{}': {}", ll_path.display(), error))?;

    let lli = env::var("LLI").unwrap_or_else(|_| "lli".to_string());
    let mut cmd = Command::new(&lli);
    add_libgomp_load_flags(&mut cmd);
    if env::var("OMP_NUM_THREADS").is_err() {
        cmd.env("OMP_NUM_THREADS", "1");
    }
    cmd.arg(&ll_path);
    let output_result = cmd.output();
    let _ = fs::remove_file(&ll_path);
    let out = output_result
        .map_err(|error| format!("failed to invoke {}: {}", lli, error))?;
    Ok((
        out.status.code().unwrap_or(1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    ))
}

/// AOT-compile to a native binary via the LLVM backend.
/// Pipeline: emit `.ll` → `llc -filetype=obj` → `.o` → `cc -o` → binary.
/// `out_path` overrides the default (source-stem in the cwd).
/// `target` is an optional LLVM target triple (e.g. `arm-none-eabi`).
/// When set, `--mtriple=<triple>` is passed to `llc` and the appropriate
/// cross-linker (`$CROSS_CC` or `<triple>-gcc`) replaces the host `cc`.
fn build_program_llvm(
    path: &Path,
    out_path: Option<&Path>,
    link_args: &[String],
    target: Option<&str>,
) -> Result<ExitCode, String> {
    let checked = compile_path_or_report(path)?;
    let ll = emit_llvm_via_ssa(&checked.ir);
    let stem = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("program");
    let pid = std::process::id();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let ll_path = env::temp_dir().join(format!("vanic-{}-{}-{}.ll", stem, pid, nanos));
    let opt_path = env::temp_dir().join(format!("vanic-{}-{}-{}.opt.ll", stem, pid, nanos));
    let obj_path = env::temp_dir().join(format!("vanic-{}-{}-{}.o", stem, pid, nanos));
    fs::write(&ll_path, ll)
        .map_err(|error| format!("failed to write '{}': {}", ll_path.display(), error))?;
    if let Ok(keep) = env::var("VANIC_KEEP_IR") {
        let _ = fs::copy(&ll_path, &keep);
    }

    // Optional opt(1) pass: promotes our alloca-heavy locals into
    // SSA values (mem2reg), inlines small functions, and folds
    // constants before llc sees the IR. Skipped silently if `opt`
    // is not installed — the build still completes with llc's own
    // optimizer (the -O=2 below).
    let opt = env::var("OPT").unwrap_or_else(|_| "opt".to_string());
    // For host builds (no cross-compile target), unlock the native CPU's
    // full ISA so the loop vectorizer can emit AVX2/AVX-512 intrinsics
    // rather than being limited to SSE2. The flag is safe because `vanic
    // build` compiles and runs on the same machine.
    let opt_mcpu = if target.is_none() { Some("native") } else { None };
    let llc_input = match {
        let mut cmd = Command::new(&opt);
        cmd.arg("-O2");
        if let Some(cpu) = opt_mcpu {
            cmd.arg(format!("--mcpu={}", cpu));
        }
        cmd.arg("-S")
            .arg(&ll_path)
            .arg("-o")
            .arg(&opt_path)
            .output()
    }
    {
        Ok(o) if o.status.success() => {
            if let Ok(keep) = env::var("VANIC_KEEP_OPT_IR") {
                let _ = fs::copy(&opt_path, &keep);
            }
            opt_path.clone()
        }
        // `opt` exists but choked: emit the stderr and keep going
        // with the unoptimized IR so the user still gets a binary.
        Ok(o) => {
            eprintln!(
                "warning: {} failed (continuing with unoptimized IR):\n{}",
                opt,
                String::from_utf8_lossy(&o.stderr).trim_end()
            );
            ll_path.clone()
        }
        // Tool missing entirely (cargo + no LLVM dev tools) — same
        // fallback. Don't make `vanic build` require `opt`.
        Err(_) => ll_path.clone(),
    };

    let llc = env::var("LLC").unwrap_or_else(|_| "llc".to_string());
    let mut llc_cmd = Command::new(&llc);
    llc_cmd.arg("-filetype=obj");
    // For cross-compilation, pass the target triple so llc selects
    // the right instruction-set backend. Without this the host triple
    // is used (x86-64 on most dev machines).
    if let Some(triple) = target {
        llc_cmd.arg(format!("--mtriple={}", triple));
        // Bare-metal / noOS targets need position-independent code
        // disabled (PIE relocations don't exist on ELF-for-ROM).
        // For Linux cross-targets keep PIC — the dynamic linker needs it.
        if !is_bare_metal_triple(triple) {
            llc_cmd.arg("-relocation-model=pic");
        }
    } else {
        // Host build: use the native CPU to lower vector IR to the widest
        // available ISA (AVX2, AVX-512). Matches the -mcpu=native passed
        // to opt above so the IR and the lowering agree on feature set.
        llc_cmd.arg("-mcpu=native");
        llc_cmd.arg("-relocation-model=pic");
    }
    // Default to -O=2. The verifier proves safety upstream so
    // the optimizer is free to assume no UB on the proved paths.
    // Users can override the optimization level by setting LLC
    // to a wrapper script if they need a different level.
    llc_cmd.arg("-O=2");
    llc_cmd.arg("-o").arg(&obj_path).arg(&llc_input);
    let llc_out = llc_cmd
        .output()
        .map_err(|error| format!("failed to invoke {}: {}", llc, error))?;
    if !llc_out.status.success() {
        let _ = fs::remove_file(&opt_path);
        let _ = fs::remove_file(&ll_path);
        return Err(format!(
            "{} failed while lowering '{}' (left at this path for debugging):\n{}",
            llc,
            llc_input.display(),
            String::from_utf8_lossy(&llc_out.stderr).trim_end()
        ));
    }

    let bin_path: PathBuf = match out_path {
        Some(p) => p.to_path_buf(),
        None => PathBuf::from(stem),
    };
    // Select the linker. For cross-compilation:
    //   1. $CROSS_CC env var (explicit override)
    //   2. <triple>-gcc, stripping "unknown-" from the triple
    //      (arm-none-eabi-gcc, riscv32-elf-gcc, aarch64-linux-gnu-gcc …)
    //   3. $CC / cc for host builds
    let (cc, is_cross) = if let Some(triple) = target {
        (cross_cc_for_triple(triple), true)
    } else {
        (env::var("CC").unwrap_or_else(|_| "cc".to_string()), false)
    };
    let bare_metal = target.map(is_bare_metal_triple).unwrap_or(false);
    let mut link_cmd = Command::new(&cc);
    link_cmd.arg(&obj_path);
    if bare_metal {
        // Bare-metal: no libc, no libm, no OpenMP, no host thread libs.
        // The user supplies their own linker script via link_args if needed.
    } else if is_cross {
        // Linux cross-targets: keep libm; skip -fopenmp (cross libgomp
        // path is non-trivial — leave it to the user via link_args).
        link_cmd.arg("-lm");
    } else if cfg!(target_os = "windows") {
        // Host Windows build
        link_cmd.arg("-lsynchronization");
        link_cmd.arg("-lws2_32");
    } else {
        // Host POSIX build
        link_cmd.arg("-fopenmp");
    }
    // Layer 4.1 of `unsafe.md` — same toolchain hardening on
    // the LLVM-backend link path. See `apply_embedded_cc_hardening`.
    if !bare_metal {
        apply_embedded_cc_hardening(&mut link_cmd);
    }
    // FFI follow-up: user-supplied link inputs follow the vāṇī
    // object so symbol resolution sees vāṇī's `extern "C" fn` call
    // sites first and then the providing object/library.
    for extra in link_args {
        link_cmd.arg(extra);
    }
    let link_out = link_cmd
        .arg("-o")
        .arg(&bin_path)
        .output()
        .map_err(|error| format!("failed to invoke {}: {}", cc, error))?;
    let _ = fs::remove_file(&ll_path);
    let _ = fs::remove_file(&opt_path);
    let _ = fs::remove_file(&obj_path);
    if !link_out.status.success() {
        return Err(format!(
            "{} failed while linking:\n{}",
            cc,
            String::from_utf8_lossy(&link_out.stderr).trim_end()
        ));
    }
    Ok(ExitCode::SUCCESS)
}

/// Layer 4.1 of `unsafe.md` — add toolchain-level hardening
/// flags to a `cc` / `clang` invocation when the embedded gate
/// is on (`INTENT_TARGET_EMBEDDED=1`). The flags applied today:
///
/// - `-fstack-protector-strong` — emits a stack canary on every
///   function that has a buffer, alloca, or string operation,
///   catching the classic "smash the canary, overwrite the
///   return address" exploit class. Cost: ~2 instructions per
///   protected frame. The `-strong` variant balances coverage
///   vs perf (vs `-fstack-protector-all` which protects every
///   single frame).
///
/// Gated to embedded so hosted parity / perf baseline doesn't
/// shift. The proper `--target embedded` flag will eventually
/// replace the env-var gate.
///
/// Layer 4.2 (ARM MTE) is a separate flag added on a different
/// path (`-march=armv8.5-a+memtag`) since it's hardware-
/// gated; that's a follow-up.
fn apply_embedded_cc_hardening(cmd: &mut Command) {
    if env::var("INTENT_TARGET_EMBEDDED").ok().as_deref() != Some("1") {
        return;
    }
    cmd.arg("-fstack-protector-strong");
    // Layer 4.2 of `unsafe.md` — ARM MTE (Memory Tagging
    // Extension v8.5+). When the user opts in via
    // `INTENT_TARGET_MTE=1` AND the embedded gate is on, append
    // `-march=armv8.5-a+memtag` so hardware tags every pointer
    // with a 4-bit value and traps on mismatch. Catches
    // use-after-free + most buffer-overrun bugs at zero runtime
    // cost on the supported hardware (recent Cortex-A,
    // Apple Silicon).
    //
    // Held behind a second env var because the flag rejects on
    // non-ARM hosts (and most CI is x86-64) — accidentally
    // enabling MTE on x86 would fail every build. Once the
    // proper `--target <triple>` flag ships, MTE becomes
    // automatic for arm64 embedded targets and the env var
    // dance goes away.
    if env::var("INTENT_TARGET_MTE").ok().as_deref() == Some("1") {
        cmd.arg("-march=armv8.5-a+memtag");
    }
}

/// Returns true for target triples that target bare-metal / no-OS environments.
/// These triples have no C runtime, no kernel ABI, and cannot be JIT-run on the host.
fn is_bare_metal_triple(triple: &str) -> bool {
    triple.contains("none") || triple.contains("eabi") || triple.ends_with("-elf")
        || triple.contains("-unknown-elf")
}

/// Derive the cross-linker for a given LLVM target triple.
/// Priority: $CROSS_CC > <triple>-gcc (with "unknown-" stripped).
fn cross_cc_for_triple(triple: &str) -> String {
    if let Ok(cc) = env::var("CROSS_CC") {
        return cc;
    }
    // Strip "unknown-" which is conventional filler; most cross-toolchains
    // use the shorter form. e.g.:
    //   arm-unknown-none-eabi   → arm-none-eabi-gcc
    //   riscv32-unknown-none-elf → riscv32-none-elf-gcc (or riscv32-elf-gcc)
    //   aarch64-unknown-linux-gnu → aarch64-linux-gnu-gcc
    let prefix = triple.replace("-unknown-", "-");
    format!("{}-gcc", prefix)
}

/// Probe the system for a QEMU user-mode emulator suitable for `triple`.
/// Returns the binary name (e.g. "qemu-arm-static") if found on PATH,
/// or None if no emulator is available.
fn qemu_for_triple(triple: &str) -> Option<String> {
    // Extract the architecture prefix from the triple
    // arm-unknown-linux-gnueabihf → arm
    // aarch64-unknown-linux-gnu → aarch64
    // riscv64-unknown-linux-gnu → riscv64
    let arch = triple.split('-').next().unwrap_or("");
    // $QEMU_<ARCH> env var override
    let env_key = format!("QEMU_{}", arch.to_uppercase());
    if let Ok(q) = env::var(&env_key) {
        return Some(q);
    }
    // Common QEMU user-mode binary names
    let candidates = [
        format!("qemu-{}-static", arch),
        format!("qemu-{}", arch),
    ];
    for candidate in &candidates {
        if which_on_path(candidate) {
            return Some(candidate.clone());
        }
    }
    None
}

/// Returns true if `name` resolves to an executable on PATH.
fn which_on_path(name: &str) -> bool {
    env::var_os("PATH")
        .map(|path_os| {
            env::split_paths(&path_os).any(|dir| {
                let full = dir.join(name);
                full.is_file()
                    || {
                        // On Unix, executable is signalled by the +x bit;
                        // on Windows, by the extension. Check existence only —
                        // if the binary is found we assume it is executable.
                        cfg!(target_os = "windows")
                            && [".exe", ".cmd", ".bat"].iter().any(|ext| {
                                dir.join(format!("{}{}", name, ext)).is_file()
                            })
                    }
            })
        })
        .unwrap_or(false)
}

/// Cross-compile via the LLVM backend for `triple` and run via QEMU user-mode.
/// Used for `vanic run --target=<linux-cross-triple>`.
fn run_program_llvm_target(
    path: &Path,
    big_o_mode: Option<vani::big_o::BigOMode>,
    triple: &str,
) -> Result<ExitCode, String> {
    // Build a temporary ELF in a temp dir
    let stem = path
        .file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or("program");
    let pid = std::process::id();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let elf_path = env::temp_dir().join(format!("vanic-{}-{}-{}.elf", stem, pid, nanos));
    if let Some(mode) = big_o_mode {
        if mode != vani::big_o::BigOMode::Off {
            let checked = compile_path_or_report(path)?;
            for (name, complexity) in vani::big_o::annotate_program(&checked.ir, mode) {
                eprintln!("  fn {}: {}", name, complexity);
            }
        }
    }
    build_program_llvm(path, Some(&elf_path), &[], Some(triple))?;
    // Try QEMU user-mode
    match qemu_for_triple(triple) {
        Some(qemu) => {
            let status = Command::new(&qemu)
                .arg(&elf_path)
                .status()
                .map_err(|e| format!("failed to invoke {}: {}", qemu, e))?;
            let _ = fs::remove_file(&elf_path);
            Ok(ExitCode::from(status.code().unwrap_or(1) as u8))
        }
        None => {
            eprintln!(
                "note: ELF written to '{}' (no QEMU emulator found for '{}')",
                elf_path.display(),
                triple
            );
            eprintln!(
                "hint: install qemu-user-static and set QEMU_{} or add \
                 qemu-{}-static to PATH",
                triple.split('-').next().unwrap_or("ARCH").to_uppercase(),
                triple.split('-').next().unwrap_or("arch"),
            );
            Ok(ExitCode::from(1))
        }
    }
}

fn temp_paths(source_path: &Path) -> (PathBuf, PathBuf) {
    let stem = source_path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("program");
    let pid = std::process::id();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let unique = format!("{}-{}-{}", stem, pid, nanos);
    let c_path = env::temp_dir().join(format!("vanic-{}.c", unique));
    let bin_path = env::temp_dir().join(format!("vanic-{}", unique));
    (c_path, bin_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_metal_triple_detection() {
        assert!(is_bare_metal_triple("arm-none-eabi"),
            "arm-none-eabi is bare metal");
        assert!(is_bare_metal_triple("riscv32-unknown-none-elf"),
            "riscv32-unknown-none-elf is bare metal");
        assert!(is_bare_metal_triple("thumbv7em-none-eabihf"),
            "thumbv7em-none-eabihf is bare metal (eabi)");
        assert!(!is_bare_metal_triple("aarch64-unknown-linux-gnu"),
            "aarch64-linux is not bare metal");
        assert!(!is_bare_metal_triple("x86_64-unknown-linux-musl"),
            "x86_64-musl is not bare metal");
    }

    #[test]
    fn cross_cc_derivation() {
        // Ensure CROSS_CC is not set so we test the fallback path.
        std::env::remove_var("CROSS_CC");
        assert_eq!(
            cross_cc_for_triple("arm-none-eabi"),
            "arm-none-eabi-gcc"
        );
        assert_eq!(
            cross_cc_for_triple("aarch64-unknown-linux-gnu"),
            "aarch64-linux-gnu-gcc",
            "unknown- must be stripped from the toolchain prefix"
        );
        assert_eq!(
            cross_cc_for_triple("riscv32-unknown-none-elf"),
            "riscv32-none-elf-gcc"
        );
    }
}
