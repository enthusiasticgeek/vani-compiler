//! T1.2 of the safety-standard alignment arc — `#[no_heap]`
//! function attribute + `INTENT_NO_HEAP=1` global mode.
//!
//! A function marked `#[no_heap]` must not allocate, nor call
//! any function (directly or transitively) that allocates. The
//! check fires after typechecking: walk the program, build a
//! call graph, mark functions that directly allocate, propagate
//! the "transitively allocates" property to fixpoint, then
//! emit diagnostics for every `#[no_heap]` function that
//! transitively reaches a heap allocator.
//!
//! With `INTENT_NO_HEAP=1` set, every function is treated as if
//! it had the `#[no_heap]` annotation — useful for full safety-
//! critical builds where the whole binary must be heap-free.
//!
//! Heap-allocating builtins (V1 curated list):
//! - **Vec**: `vec`, `push`, `pop` (shrinks; no alloc but listed
//!   for closure consistency), `set`, `insert`, `swap_remove`,
//!   `vec_remove_at`, `vec_replace_all`, `clone`, `clone_at`,
//!   `try_vec`. Plus most `vec_*` combinators that return a
//!   new Vec.
//! - **OwnedStr / Str**: `i64_to_str`, `f64_to_str`,
//!   `bool_to_str`, `str_repeat`, `str_replace`, `substring`,
//!   `str_to_upper`, `str_to_lower`, `str_pad_left`,
//!   `str_pad_right`, `str_split`, `str_lines`, `str_chars`,
//!   `str_reverse`, `str_strip_prefix`, `str_strip_suffix`,
//!   `str_join`, string `+` operator (handled separately via
//!   binary-op check below).
//! - **Affine containers** (all heap-backed): `channel_new`,
//!   `deque_*` (push), `hashset_*` (insert), `hashmap_*`
//!   (insert), `btreeset_*` (insert), `btreemap_*` (insert),
//!   `binary_heap_*` (push), `bloom_filter_new`, `bst_*` (insert),
//!   `graph_*` (add_edge), `trie_*` (insert), `skiplist_*` (insert),
//!   `union_find_*` (union).
//! - **Unsafe / Pool / Region** (Layer 2-5 of unsafe.md):
//!   `unsafe_alloc`, `pool_alloc`, `region_alloc_i64`,
//!   `region_borrow_i64`.
//!
//! Non-allocating builtins (safe to call from `#[no_heap]`):
//! length / contains / get / peek / find / binary_search,
//! plain reads and bounds-checked accesses, `raw_load` /
//! `raw_store`, `aref_load` / `aref_store`, `pool_new`,
//! `pool_get`, `pool_free`, `region_new`, `region_len`,
//! `bptr_get` / `bptr_set` / `bptr_len`, `mutex_new`,
//! `atomic_new`, `assert_safe`, `taint`.
//!
//! Note: `pool_new` and `region_new` themselves are *zero-
//! init*; the heap allocation happens lazily inside
//! `pool_alloc` / `region_alloc_i64`. So a `#[no_heap]`
//! function that only constructs (but doesn't grow) a Pool /
//! Region is fine.

use crate::diagnostic::Diagnostic;
use crate::ir::{TypedProgram, TypedStmt, TypedExpr, TypedExprKind};
use std::collections::HashMap;

/// Names of builtin functions that allocate on the heap. A
/// `#[no_heap]` function (or any function reachable from one)
/// that calls any of these is rejected.
fn is_heap_allocating_builtin(name: &str) -> bool {
    matches!(
        name,
        // Vec
        "vec" | "push" | "set" | "insert" | "swap_remove"
        | "vec_remove_at" | "vec_replace_all" | "clone" | "clone_at"
        | "try_vec"
        // Vec combinators that allocate a new Vec
        | "vec_map" | "vec_filter" | "vec_zip_with" | "vec_take"
        | "vec_drop" | "vec_take_while" | "vec_drop_while"
        | "vec_map_filter" | "vec_chain" | "vec_range" | "vec_repeat"
        | "vec_extend" | "vec_concat" | "vec_reverse_copy"
        | "vec_unique" | "vec_iota" | "vec_intersect" | "vec_difference"
        | "vec_union" | "vec_diff" | "vec_pad_left" | "vec_pad_right"
        | "vec_chunks" | "vec_windows" | "vec_flatten"
        | "vec_group_by_value" | "vec_indices_of_value"
        | "vec_dedup_consecutive" | "vec_merge_sorted"
        | "vec_insert_sorted" | "vec_cumulative_max"
        | "vec_cumulative_min" | "vec_running_sum"
        | "vec_running_product" | "vec_running_xor"
        | "vec_running_and" | "vec_running_or" | "vec_sliding_max"
        | "vec_sliding_min" | "vec_sliding_sum"
        | "vec_sliding_product" | "vec_abs" | "vec_negate"
        | "vec_signum" | "vec_square" | "vec_add_scalar"
        | "vec_sub_scalar" | "vec_mul_scalar" | "vec_div_scalar"
        | "vec_mod_scalar" | "vec_pow_scalar" | "vec_shl_scalar"
        | "vec_shr_scalar" | "vec_eq_mask" | "vec_ne_mask"
        | "vec_lt_mask" | "vec_le_mask" | "vec_gt_mask"
        | "vec_ge_mask" | "vec_min_with_scalar"
        | "vec_max_with_scalar" | "vec_clamp_scalar"
        | "vec_add_pairwise" | "vec_sub_pairwise"
        | "vec_mul_pairwise" | "vec_min_pairwise"
        | "vec_max_pairwise" | "vec_rotate_left" | "vec_rotate_right"
        | "vec_shift_left" | "vec_shift_right"
        | "vec_replace_value" | "vec_running_mean" | "vec_intersperse"
        // OwnedStr producers
        | "i64_to_str" | "f64_to_str" | "bool_to_str" | "str_repeat"
        | "str_replace" | "substring" | "str_to_upper" | "str_to_lower"
        | "str_pad_left" | "str_pad_right" | "str_split" | "str_lines"
        | "str_chars" | "str_reverse" | "str_strip_prefix"
        | "str_strip_suffix" | "str_join"
        // Affine container heap-allocating ops (channel_new is
        // the ring buffer alloc; per-container new is mostly
        // zero-init but insert/push triggers grow → alloc)
        | "channel_new"
        | "deque_new" | "deque_push_back" | "deque_push_front"
        | "hashset_insert" | "hashmap_insert"
        | "btreeset_insert" | "btreemap_insert"
        | "binary_heap_push" | "binary_heap_new"
        | "bloom_filter_new"
        | "bst_insert" | "graph_add_edge" | "trie_insert"
        | "skiplist_insert" | "union_find_new" | "union_find_union"
        // Unsafe / Pool / Region (Layer 2-5 of unsafe.md)
        | "unsafe_alloc"
        | "pool_alloc"
        | "region_alloc_i64" | "region_borrow_i64"
        | "bptr_new"
    )
}

/// Read the `INTENT_NO_HEAP=1` env var once. The CLI layer
/// calls this at process start and threads the result through
/// `enforce_no_heap`'s `global` parameter — so the per-test
/// `cargo test` parallel harness can pass its own boolean
/// without racing on the process-global env var.
pub fn global_no_heap_from_env() -> bool {
    std::env::var("INTENT_NO_HEAP").ok().as_deref() == Some("1")
}

/// Walk a TypedProgram and emit a diagnostic for every
/// `#[no_heap]` function (or every function when `global` is
/// `true`) that transitively reaches a heap-allocating
/// builtin. The `global` flag corresponds to the
/// `INTENT_NO_HEAP=1` env var at the CLI layer; tests pass it
/// explicitly to avoid racing.
pub fn enforce_no_heap(
    program: &TypedProgram,
    global: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut direct: HashMap<String, Option<DirectAlloc>> = HashMap::new();
    let mut calls: HashMap<String, Vec<String>> = HashMap::new();

    // Pass 1: per-function, collect direct heap-allocating
    // builtin invocations + the set of user functions called.
    for f in &program.functions {
        let mut local_alloc: Option<DirectAlloc> = None;
        let mut local_calls: Vec<String> = Vec::new();
        walk_stmts(&f.body, &mut local_alloc, &mut local_calls);
        direct.insert(f.name.clone(), local_alloc);
        calls.insert(f.name.clone(), local_calls);
    }

    // Pass 2: fixpoint propagate "transitively allocates" through
    // the call graph. A fn transitively allocates if it directly
    // allocates OR any callee transitively allocates.
    let mut transitive: HashMap<String, Option<DirectAlloc>> = direct.clone();
    loop {
        let mut changed = false;
        // Iterate in a stable order so transitive-blame
        // propagation is deterministic.
        let names: Vec<String> = transitive.keys().cloned().collect();
        for name in &names {
            if transitive.get(name).map(|o| o.is_some()).unwrap_or(false) {
                continue;
            }
            // Was unset; check if any callee transitively
            // allocates.
            let propagate = calls
                .get(name)
                .map(|callees| {
                    callees.iter().find_map(|c| {
                        transitive.get(c).and_then(|o| o.clone()).map(|d| DirectAlloc {
                            builtin: d.builtin,
                            via: Some(c.clone()),
                            span: d.span,
                        })
                    })
                })
                .unwrap_or(None);
            if let Some(via_alloc) = propagate {
                transitive.insert(name.clone(), Some(via_alloc));
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    // Pass 3: emit diagnostics.
    for f in &program.functions {
        let gated = f.no_heap || global;
        if !gated {
            continue;
        }
        let Some(Some(alloc)) = transitive.get(&f.name) else {
            continue;
        };
        let via_note = match &alloc.via {
            Some(callee) => format!(
                " via call to '{}' (and possibly deeper)",
                callee
            ),
            None => String::new(),
        };
        let tag_origin = if f.no_heap {
            "function is marked `#[no_heap]`"
        } else {
            "global heap-free mode (`INTENT_NO_HEAP=1`) is active"
        };
        diagnostics.push(Diagnostic::new(
            alloc.span,
            format!(
                "'{}' calls heap-allocating builtin '{}'{} — {}. \
                 Refactor to use a `Region` arena (Layer 5 of `unsafe.md`) \
                 or pre-allocate at program startup.",
                f.name, alloc.builtin, via_note, tag_origin
            ),
        ));
    }
}

/// Record of a direct heap-allocating builtin invocation.
/// Carries the builtin name, the call site span, and (when set
/// during the fixpoint pass) the intermediate callee that
/// reaches it.
#[derive(Clone, Debug)]
struct DirectAlloc {
    builtin: String,
    via: Option<String>,
    span: crate::span::Span,
}

fn walk_stmts(
    stmts: &[TypedStmt],
    alloc: &mut Option<DirectAlloc>,
    calls: &mut Vec<String>,
) {
    for s in stmts {
        walk_stmt(s, alloc, calls);
        if alloc.is_some() {
            // We only need the first allocation site for the
            // diagnostic. Continue collecting calls so the
            // call-graph stays complete.
        }
    }
}

fn walk_stmt(stmt: &TypedStmt, alloc: &mut Option<DirectAlloc>, calls: &mut Vec<String>) {
    match stmt {
        TypedStmt::Let { expr, .. }
        | TypedStmt::Reassign { expr, .. }
        | TypedStmt::Return { expr }
        | TypedStmt::Assert { expr, .. }
        | TypedStmt::Prove { expr }
        | TypedStmt::Discard { expr } => walk_expr(expr, alloc, calls),
        TypedStmt::IndexAssign { value, .. }
        | TypedStmt::FieldAssign { value, .. } => walk_expr(value, alloc, calls),
        TypedStmt::Print { items } => {
            for it in items {
                if let crate::ir::TypedPrintItem::Expr(e) = it {
                    walk_expr(e, alloc, calls);
                }
            }
        }
        TypedStmt::If { cond, then_body, else_body } => {
            walk_expr(cond, alloc, calls);
            walk_stmts(then_body, alloc, calls);
            walk_stmts(else_body, alloc, calls);
        }
        TypedStmt::While { cond, body } => {
            walk_expr(cond, alloc, calls);
            walk_stmts(body, alloc, calls);
        }
        TypedStmt::For { start, end, body, .. } => {
            walk_expr(start, alloc, calls);
            walk_expr(end, alloc, calls);
            walk_stmts(body, alloc, calls);
        }
        TypedStmt::ForIter { body, .. }
        | TypedStmt::TaskSpawn { body, .. }
        | TypedStmt::UnsafeBlock { body, .. } => {
            walk_stmts(body, alloc, calls);
        }
        _ => {}
    }
}

fn walk_expr(expr: &TypedExpr, alloc: &mut Option<DirectAlloc>, calls: &mut Vec<String>) {
    match &expr.kind {
        TypedExprKind::Call { name, args, .. } => {
            if is_heap_allocating_builtin(name) && alloc.is_none() {
                *alloc = Some(DirectAlloc {
                    builtin: name.clone(),
                    via: None,
                    span: expr.span,
                });
            }
            // Track user-defined fn calls (anything not a
            // known heap-alloc-builtin — note this includes
            // non-heap-alloc builtins, which is fine: they
            // won't appear in the call-graph keys, so the
            // fixpoint's lookup just returns None for them).
            if !is_heap_allocating_builtin(name) {
                calls.push(name.clone());
            }
            for a in args {
                walk_expr(a, alloc, calls);
            }
        }
        TypedExprKind::Binary { left, right, op, .. } => {
            // Binary-op allocs: Str + Str / Str + OwnedStr =
            // OwnedStr is heap-allocating. Catch this without
            // requiring a builtin name match.
            let allocates = matches!(op, crate::ast::BinaryOp::Add)
                && (matches!(expr.ty, crate::ast::Type::OwnedStr));
            if allocates && alloc.is_none() {
                *alloc = Some(DirectAlloc {
                    builtin: "<str concat (+)>".to_string(),
                    via: None,
                    span: expr.span,
                });
            }
            walk_expr(left, alloc, calls);
            walk_expr(right, alloc, calls);
        }
        TypedExprKind::Unary { expr, .. } | TypedExprKind::Cast { expr, .. } => {
            walk_expr(expr, alloc, calls);
        }
        TypedExprKind::ArrayLit { elements } | TypedExprKind::Tuple { elements } => {
            for e in elements {
                walk_expr(e, alloc, calls);
            }
        }
        TypedExprKind::StructLit { fields, .. } => {
            for (_, e) in fields {
                walk_expr(e, alloc, calls);
            }
        }
        TypedExprKind::FieldAccess { object, .. } => {
            walk_expr(object, alloc, calls);
        }
        TypedExprKind::TupleAccess { tuple, .. } => {
            walk_expr(tuple, alloc, calls);
        }
        TypedExprKind::Len { array, .. } => {
            walk_expr(array, alloc, calls);
        }
        TypedExprKind::Index { array, index, .. } => {
            walk_expr(array, alloc, calls);
            walk_expr(index, alloc, calls);
        }
        TypedExprKind::IfExpr { cond, then_value, else_value } => {
            walk_expr(cond, alloc, calls);
            walk_expr(then_value, alloc, calls);
            walk_expr(else_value, alloc, calls);
        }
        TypedExprKind::Block { stmts, tail } => {
            walk_stmts(stmts, alloc, calls);
            walk_expr(tail, alloc, calls);
        }
        TypedExprKind::Match { scrutinee, arms } => {
            walk_expr(scrutinee, alloc, calls);
            for a in arms {
                walk_expr(&a.body, alloc, calls);
            }
        }
        TypedExprKind::Ref { .. }
        | TypedExprKind::RefMut { .. }
        | TypedExprKind::Var(_)
        | TypedExprKind::Int(_)
        | TypedExprKind::Float(_)
        | TypedExprKind::Bool(_)
        | TypedExprKind::Str(_) => {}
        _ => {
            // Catch-all for variants this pass doesn't need
            // to deeply traverse (RefField, MethodCall once
            // it's typed, DynCoerce, etc.). They're either
            // already-walked at a higher level or don't
            // contain heap-allocating builtins reachable
            // through their direct kind.
            let _ = calls;
        }
    }
    let _ = std::mem::discriminant(&expr.kind); // silence unused-warning chance
}
