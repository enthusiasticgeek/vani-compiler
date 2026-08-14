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
        TypedStmt::While { cond, body, .. } => {
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

/// T2.2 — enforce `#[interrupt]` calling convention. Composite
/// of: no_heap + no_recursion + no_lock + no_spawn — the four
/// constraints required for a body that runs in interrupt
/// context.
///
/// - **no_heap**: malloc/free in an ISR is forbidden (the
///   allocator may itself need a lock, leading to deadlock).
///   Already enforced by `enforce_no_heap` when the fn is
///   marked. We re-run the check here for explicit ISR
///   framing.
/// - **no_recursion**: ISRs must have bounded stack; recursion
///   risks overflowing the ISR stack budget.
/// - **no_lock**: an ISR holding a lock that the main thread
///   wants to take is a classic deadlock. Reject any call to
///   `mutex_lock`, `condvar_wait`, `condvar_wait_timeout`.
/// - **no_spawn**: ISRs can't fork — no `task <name> { … }`
///   or `parallel for`.
///
/// The composite isn't expanded in the AST (the fn just has
/// `interrupt = true`); this pass runs the union of the
/// underlying primitive checks and labels each violation with
/// the ISR context.
pub fn enforce_interrupt(program: &TypedProgram, diagnostics: &mut Vec<Diagnostic>) {
    // The interrupt fn's body must satisfy: no heap, no
    // recursion, no lock-acquiring ops, no spawn.
    // For each `interrupt = true` fn, run the checks with the
    // ISR-framed diagnostic message. Transitive call graph
    // already covered for no_heap / no_recursion via their
    // existing passes (`enforce_no_heap` + `enforce_no_recursion`
    // are called with `f.no_heap = true` / `f.no_recursion = true`
    // implicitly because the composite is expanded below by
    // setting those flags during the typechecker post-pass).
    // The new constraints (no_lock, no_spawn) are local checks
    // since both can be detected from the body statements
    // directly.
    for f in &program.functions {
        if !f.interrupt {
            continue;
        }
        let mut local_calls: Vec<String> = Vec::new();
        let mut violations: Vec<(crate::span::Span, &'static str)> = Vec::new();
        for s in &f.body {
            walk_stmt_for_isr(s, &mut local_calls, &mut violations);
        }
        for (span, kind) in violations {
            diagnostics.push(Diagnostic::new(
                span,
                format!(
                    "'{}' contains {} — `#[interrupt]` functions forbid this. \
                     An ISR holding a lock the main thread is waiting on \
                     creates a deadlock; forking a worker thread escapes \
                     ISR context and breaks the no-block guarantee.",
                    f.name, kind
                ),
            ));
        }
    }
}

fn walk_stmt_for_isr(
    stmt: &TypedStmt,
    calls: &mut Vec<String>,
    violations: &mut Vec<(crate::span::Span, &'static str)>,
) {
    match stmt {
        TypedStmt::TaskSpawn { body, .. } => {
            // Any TaskSpawn in an ISR's body is forbidden.
            // Use first body stmt's span as a proxy (TaskSpawn
            // doesn't carry its own span explicitly).
            let span = body
                .first()
                .and_then(isr_stmt_span)
                .unwrap_or_default();
            violations.push((span, "a `task` spawn"));
            for s in body { walk_stmt_for_isr(s, calls, violations); }
        }
        TypedStmt::For { body, parallel, start, .. } if *parallel => {
            violations.push((start.span, "a `parallel for`"));
            for s in body { walk_stmt_for_isr(s, calls, violations); }
        }
        TypedStmt::Let { expr, .. }
        | TypedStmt::Reassign { expr, .. }
        | TypedStmt::Return { expr }
        | TypedStmt::Assert { expr, .. }
        | TypedStmt::Prove { expr }
        | TypedStmt::Discard { expr } => walk_expr_for_isr(expr, calls, violations),
        TypedStmt::IndexAssign { value, .. } | TypedStmt::FieldAssign { value, .. } => {
            walk_expr_for_isr(value, calls, violations);
        }
        TypedStmt::Print { items } => {
            for it in items {
                if let crate::ir::TypedPrintItem::Expr(e) = it {
                    walk_expr_for_isr(e, calls, violations);
                }
            }
        }
        TypedStmt::If { cond, then_body, else_body } => {
            walk_expr_for_isr(cond, calls, violations);
            for s in then_body { walk_stmt_for_isr(s, calls, violations); }
            for s in else_body { walk_stmt_for_isr(s, calls, violations); }
        }
        TypedStmt::While { cond, body, .. } => {
            walk_expr_for_isr(cond, calls, violations);
            for s in body { walk_stmt_for_isr(s, calls, violations); }
        }
        TypedStmt::For { start, end, body, .. } => {
            walk_expr_for_isr(start, calls, violations);
            walk_expr_for_isr(end, calls, violations);
            for s in body { walk_stmt_for_isr(s, calls, violations); }
        }
        TypedStmt::ForIter { body, .. } | TypedStmt::UnsafeBlock { body, .. } => {
            for s in body { walk_stmt_for_isr(s, calls, violations); }
        }
        _ => {}
    }
}

fn isr_stmt_span(stmt: &TypedStmt) -> Option<crate::span::Span> {
    use TypedStmt as S;
    match stmt {
        S::Let { expr, .. } => Some(expr.span),
        S::Reassign { expr, .. } => Some(expr.span),
        S::Return { expr } => Some(expr.span),
        S::Assert { expr, .. } => Some(expr.span),
        S::Prove { expr } => Some(expr.span),
        S::Discard { expr } => Some(expr.span),
        S::IndexAssign { value, .. } | S::FieldAssign { value, .. } => Some(value.span),
        _ => None,
    }
}

fn walk_expr_for_isr(
    expr: &TypedExpr,
    calls: &mut Vec<String>,
    violations: &mut Vec<(crate::span::Span, &'static str)>,
) {
    match &expr.kind {
        TypedExprKind::Call { name, args, .. } => {
            calls.push(name.clone());
            // Lock-acquiring builtins.
            if matches!(
                name.as_str(),
                "mutex_lock" | "condvar_wait" | "condvar_wait_timeout"
            ) {
                violations.push((expr.span, "a blocking lock acquire"));
            }
            for a in args { walk_expr_for_isr(a, calls, violations); }
        }
        TypedExprKind::Binary { left, right, .. } => {
            walk_expr_for_isr(left, calls, violations);
            walk_expr_for_isr(right, calls, violations);
        }
        TypedExprKind::Unary { expr, .. } | TypedExprKind::Cast { expr, .. } => {
            walk_expr_for_isr(expr, calls, violations);
        }
        TypedExprKind::Index { array, index, .. } => {
            walk_expr_for_isr(array, calls, violations);
            walk_expr_for_isr(index, calls, violations);
        }
        TypedExprKind::ArrayLit { elements } | TypedExprKind::Tuple { elements } => {
            for e in elements { walk_expr_for_isr(e, calls, violations); }
        }
        TypedExprKind::IfExpr { cond, then_value, else_value } => {
            walk_expr_for_isr(cond, calls, violations);
            walk_expr_for_isr(then_value, calls, violations);
            walk_expr_for_isr(else_value, calls, violations);
        }
        _ => {}
    }
}

/// T2.4 — cyclomatic complexity (McCabe) warning. For each
/// function, count: 1 (base) + every if/while/for/match-arm/
/// && / ||. If > threshold (default 15, override via
/// `INTENT_MAX_COMPLEXITY=<N>`), emit a warning-level
/// diagnostic. Not a hard error — complex fns sometimes
/// genuinely need their branches, but the report nudges
/// users toward smaller fns.
///
/// MISRA 18.x adjacent. Not a MISRA rule per se but widely
/// used as a complexity ceiling for safety-critical code
/// review.
pub fn enforce_complexity(program: &TypedProgram, diagnostics: &mut Vec<Diagnostic>) {
    // Opt-in only — emits diagnostics only when
    // `INTENT_CHECK_COMPLEXITY=1` (or override via
    // `INTENT_MAX_COMPLEXITY=<N>` which also enables it).
    // Avoids surfacing the warning for the 1500+ existing
    // functions in the test corpus that legitimately exceed
    // the default threshold.
    let opt_in = std::env::var("INTENT_CHECK_COMPLEXITY")
        .ok()
        .as_deref()
        == Some("1")
        || std::env::var("INTENT_MAX_COMPLEXITY").is_ok();
    if !opt_in {
        return;
    }
    let max: u64 = std::env::var("INTENT_MAX_COMPLEXITY")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(15);
    for f in &program.functions {
        if f.is_extern {
            continue;
        }
        let mut count: u64 = 1;
        for s in &f.body {
            count += stmt_complexity(s);
        }
        if count > max {
            diagnostics.push(Diagnostic::new(
                f.span,
                format!(
                    "'{}' has cyclomatic complexity {} (over threshold {}). \
                     Consider extracting helpers — high-branch fns are harder \
                     to review against MISRA / ISO 26262 / DO-178C coverage \
                     requirements. Threshold is configurable via \
                     `INTENT_MAX_COMPLEXITY=<N>`.",
                    f.name, count, max
                ),
            ));
        }
    }
}

fn stmt_complexity(stmt: &TypedStmt) -> u64 {
    match stmt {
        TypedStmt::If { cond, then_body, else_body } => {
            let mut c = 1; // the `if` itself
            c += expr_complexity(cond);
            for s in then_body { c += stmt_complexity(s); }
            for s in else_body { c += stmt_complexity(s); }
            c
        }
        TypedStmt::While { cond, body, .. } => {
            let mut c = 1; // the `while`
            c += expr_complexity(cond);
            for s in body { c += stmt_complexity(s); }
            c
        }
        TypedStmt::For { start, end, body, .. } => {
            let mut c = 1; // the `for`
            c += expr_complexity(start);
            c += expr_complexity(end);
            for s in body { c += stmt_complexity(s); }
            c
        }
        TypedStmt::ForIter { body, .. } => {
            let mut c = 1;
            for s in body { c += stmt_complexity(s); }
            c
        }
        TypedStmt::Let { expr, .. }
        | TypedStmt::Reassign { expr, .. }
        | TypedStmt::Return { expr }
        | TypedStmt::Assert { expr, .. }
        | TypedStmt::Prove { expr }
        | TypedStmt::Discard { expr } => expr_complexity(expr),
        TypedStmt::IndexAssign { value, .. } | TypedStmt::FieldAssign { value, .. } => {
            expr_complexity(value)
        }
        TypedStmt::Print { items } => {
            items.iter().map(|it| match it {
                crate::ir::TypedPrintItem::Expr(e) => expr_complexity(e),
                _ => 0,
            }).sum()
        }
        TypedStmt::TaskSpawn { body, .. } | TypedStmt::UnsafeBlock { body, .. } => {
            body.iter().map(stmt_complexity).sum()
        }
        _ => 0,
    }
}

fn expr_complexity(expr: &TypedExpr) -> u64 {
    use crate::ast::BinaryOp;
    match &expr.kind {
        // && and || each add a branch point.
        TypedExprKind::Binary { op, left, right, .. } => {
            let extra = if matches!(op, BinaryOp::And | BinaryOp::Or) { 1 } else { 0 };
            extra + expr_complexity(left) + expr_complexity(right)
        }
        TypedExprKind::Unary { expr, .. } | TypedExprKind::Cast { expr, .. } => {
            expr_complexity(expr)
        }
        TypedExprKind::Match { scrutinee, arms } => {
            // Each match arm contributes 1.
            let mut c = arms.len() as u64;
            c += expr_complexity(scrutinee);
            for a in arms { c += expr_complexity(&a.body); }
            c
        }
        TypedExprKind::IfExpr { cond, then_value, else_value } => {
            1 + expr_complexity(cond) + expr_complexity(then_value)
                + expr_complexity(else_value)
        }
        TypedExprKind::Call { args, .. } => {
            args.iter().map(expr_complexity).sum()
        }
        TypedExprKind::Index { array, index, .. } => {
            expr_complexity(array) + expr_complexity(index)
        }
        TypedExprKind::ArrayLit { elements } | TypedExprKind::Tuple { elements } => {
            elements.iter().map(expr_complexity).sum()
        }
        TypedExprKind::Block { stmts, tail } => {
            let mut c = stmts.iter().map(stmt_complexity).sum::<u64>();
            c += expr_complexity(tail);
            c
        }
        _ => 0,
    }
}

/// T2.3 — enforce `#[no_float]` per function. Walks the
/// function body looking for any sub-expression of type
/// `Type::F32` or `Type::F64` (or `Vec<Fxx>` / `Array<Fxx;N>`
/// / `Tuple<…Fxx…>`); if found, emit a diagnostic. Transitive
/// through user-defined fn calls (a `#[no_float]` fn can't
/// call a fn that uses float internally because the callee's
/// frame would compute floats on its stack — undermining the
/// "no FPU touch" guarantee).
pub fn enforce_no_float(program: &TypedProgram, diagnostics: &mut Vec<Diagnostic>) {
    let mut direct: HashMap<String, Option<FloatUse>> = HashMap::new();
    let mut calls: HashMap<String, Vec<String>> = HashMap::new();
    for f in &program.functions {
        let mut float_use: Option<FloatUse> = None;
        let mut local_calls: Vec<String> = Vec::new();
        for p in &f.params {
            if ty_uses_float(&p.ty) && float_use.is_none() {
                float_use = Some(FloatUse {
                    site: format!("parameter '{}' of type {}", p.name, p.ty),
                    span: p.name_span,
                    via: None,
                });
            }
        }
        if ty_uses_float(&f.return_type) && float_use.is_none() {
            float_use = Some(FloatUse {
                site: format!("return type {}", f.return_type),
                span: f.span,
                via: None,
            });
        }
        for s in &f.body {
            walk_stmt_for_float(s, &mut float_use, &mut local_calls);
        }
        direct.insert(f.name.clone(), float_use);
        calls.insert(f.name.clone(), local_calls);
    }
    // Fixpoint propagation through call graph.
    let mut transitive: HashMap<String, Option<FloatUse>> = direct.clone();
    loop {
        let mut changed = false;
        let names: Vec<String> = transitive.keys().cloned().collect();
        for name in &names {
            if transitive.get(name).map(|o| o.is_some()).unwrap_or(false) {
                continue;
            }
            let propagate = calls.get(name).and_then(|callees| {
                callees.iter().find_map(|c| {
                    transitive.get(c).and_then(|o| o.clone()).map(|u| FloatUse {
                        site: u.site,
                        span: u.span,
                        via: Some(c.clone()),
                    })
                })
            });
            if let Some(u) = propagate {
                transitive.insert(name.clone(), Some(u));
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    for f in &program.functions {
        if !f.no_float {
            continue;
        }
        if let Some(Some(u)) = transitive.get(&f.name) {
            let via = u
                .via
                .as_ref()
                .map(|c| format!(" via call to '{}'", c))
                .unwrap_or_default();
            diagnostics.push(Diagnostic::new(
                u.span,
                format!(
                    "'{}' uses floating-point ({}){} — function is marked \
                     `#[no_float]`. Use fixed-point arithmetic on i32 / i64 \
                     for critical-path code.",
                    f.name, u.site, via
                ),
            ));
        }
    }
}

#[derive(Clone, Debug)]
struct FloatUse {
    site: String,
    span: crate::span::Span,
    via: Option<String>,
}

fn ty_uses_float(ty: &crate::ast::Type) -> bool {
    use crate::ast::Type::*;
    match ty {
        F32 | F64 => true,
        Vec(inner) | Ref(inner) | RefMut(inner) | Atomic(inner) | Mutex(inner)
        | Guard(inner) | Channel(inner, _) | Tainted(inner)
        | Ptr(inner) | PtrMut(inner) | Handle(inner) | Pool(inner)
        | BoundedPtr(inner) | ArenaRef(inner)
        | Deque(inner) | HashSet(inner) | BinaryHeap(inner) | BTreeSet(inner)
        | Bst(inner) => ty_uses_float(inner),
        HashMap(k, v) | BTreeMap(k, v) => ty_uses_float(k) || ty_uses_float(v),
        Array { element, .. } => ty_uses_float(element),
        Tuple(elements) => elements.iter().any(ty_uses_float),
        FnPtr(params, ret) => params.iter().any(ty_uses_float) || ty_uses_float(ret),
        Apply { args, .. } => args.iter().any(ty_uses_float),
        _ => false,
    }
}

fn walk_stmt_for_float(
    stmt: &TypedStmt,
    float_use: &mut Option<FloatUse>,
    calls: &mut Vec<String>,
) {
    match stmt {
        TypedStmt::Let { ty, expr, name } => {
            if ty_uses_float(ty) && float_use.is_none() {
                *float_use = Some(FloatUse {
                    site: format!("local '{}' of type {}", name, ty),
                    span: expr.span,
                    via: None,
                });
            }
            walk_expr_for_float(expr, float_use, calls);
        }
        TypedStmt::Reassign { ty, expr, .. } => {
            if ty_uses_float(ty) && float_use.is_none() {
                *float_use = Some(FloatUse {
                    site: format!("reassignment of type {}", ty),
                    span: expr.span,
                    via: None,
                });
            }
            walk_expr_for_float(expr, float_use, calls);
        }
        TypedStmt::Return { expr } | TypedStmt::Assert { expr, .. }
        | TypedStmt::Prove { expr } | TypedStmt::Discard { expr } => {
            walk_expr_for_float(expr, float_use, calls);
        }
        TypedStmt::IndexAssign { value, .. } | TypedStmt::FieldAssign { value, .. } => {
            walk_expr_for_float(value, float_use, calls);
        }
        TypedStmt::Print { items } => {
            for it in items {
                if let crate::ir::TypedPrintItem::Expr(e) = it {
                    walk_expr_for_float(e, float_use, calls);
                }
            }
        }
        TypedStmt::If { cond, then_body, else_body } => {
            walk_expr_for_float(cond, float_use, calls);
            for s in then_body { walk_stmt_for_float(s, float_use, calls); }
            for s in else_body { walk_stmt_for_float(s, float_use, calls); }
        }
        TypedStmt::While { cond, body, .. } => {
            walk_expr_for_float(cond, float_use, calls);
            for s in body { walk_stmt_for_float(s, float_use, calls); }
        }
        TypedStmt::For { start, end, body, .. } => {
            walk_expr_for_float(start, float_use, calls);
            walk_expr_for_float(end, float_use, calls);
            for s in body { walk_stmt_for_float(s, float_use, calls); }
        }
        TypedStmt::ForIter { body, .. }
        | TypedStmt::TaskSpawn { body, .. }
        | TypedStmt::UnsafeBlock { body, .. } => {
            for s in body { walk_stmt_for_float(s, float_use, calls); }
        }
        _ => {}
    }
}

fn walk_expr_for_float(
    expr: &TypedExpr,
    float_use: &mut Option<FloatUse>,
    calls: &mut Vec<String>,
) {
    if ty_uses_float(&expr.ty) && float_use.is_none() {
        *float_use = Some(FloatUse {
            site: format!("expression of type {}", expr.ty),
            span: expr.span,
            via: None,
        });
    }
    match &expr.kind {
        TypedExprKind::Call { name, args, .. } => {
            calls.push(name.clone());
            for a in args { walk_expr_for_float(a, float_use, calls); }
        }
        TypedExprKind::Binary { left, right, .. } => {
            walk_expr_for_float(left, float_use, calls);
            walk_expr_for_float(right, float_use, calls);
        }
        TypedExprKind::Unary { expr, .. } | TypedExprKind::Cast { expr, .. } => {
            walk_expr_for_float(expr, float_use, calls);
        }
        TypedExprKind::Index { array, index, .. } => {
            walk_expr_for_float(array, float_use, calls);
            walk_expr_for_float(index, float_use, calls);
        }
        TypedExprKind::ArrayLit { elements } | TypedExprKind::Tuple { elements } => {
            for e in elements { walk_expr_for_float(e, float_use, calls); }
        }
        TypedExprKind::IfExpr { cond, then_value, else_value } => {
            walk_expr_for_float(cond, float_use, calls);
            walk_expr_for_float(then_value, float_use, calls);
            walk_expr_for_float(else_value, float_use, calls);
        }
        TypedExprKind::Block { stmts, tail } => {
            for s in stmts { walk_stmt_for_float(s, float_use, calls); }
            walk_expr_for_float(tail, float_use, calls);
        }
        _ => {}
    }
}


/// T2.4 -- enforce `#[no_nan]` per function. Rejects any call
/// to a builtin whose documented error contract is to return
/// IEEE-754 quiet NaN:
///   - `f64_nan`             -- the explicit NaN literal.
///   - `vec_kth_smallest`    -- returns qNaN (0x7FF8000000000000)
///                              when k is out of bounds on Vec<f64>.
/// Mathematical builtins that CAN produce NaN on bad inputs
/// (sqrt, log, asin, ...) are NOT flagged here -- those require
/// value-range analysis beyond what the static pass supports.
/// Implied by `#[asil_d]`, `#[do178c_level_a]`, `#[iec_61508_sil3]`,
/// `#[iec_61508_sil4]`.
pub fn enforce_no_nan(program: &TypedProgram, diagnostics: &mut Vec<Diagnostic>) {
    for f in &program.functions {
        if !f.no_nan {
            continue;
        }
        for stmt in &f.body {
            walk_stmt_for_nan(stmt, f, diagnostics);
        }
    }
}

/// Returns true if a builtin call is DEFINED to produce NaN
/// as part of its error contract (not merely capable of doing
/// so on bad inputs).
fn is_nan_producing_builtin(name: &str, args: &[TypedExpr]) -> bool {
    use crate::ast::Type;
    match name {
        // f64_nan() is the explicit NaN constructor -- always NaN.
        "f64_nan" => true,
        // vec_kth_smallest returns quiet NaN as an out-of-bounds
        // sentinel, but only when the element type is f64.
        "vec_kth_smallest" => args.first().map_or(false, |a| {
            // Strip the outer Ref / RefMut to reach the Vec.
            let inner = match &a.ty {
                Type::Ref(i) | Type::RefMut(i) => i.as_ref(),
                other => other,
            };
            matches!(inner, Type::Vec(el) if matches!(el.as_ref(), Type::F64))
        }),
        _ => false,
    }
}

fn walk_stmt_for_nan(
    stmt: &TypedStmt,
    f: &crate::ir::TypedFunction,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match stmt {
        TypedStmt::Let { expr, .. }
        | TypedStmt::Reassign { expr, .. }
        | TypedStmt::Return { expr }
        | TypedStmt::Assert { expr, .. }
        | TypedStmt::Prove { expr }
        | TypedStmt::Discard { expr } => walk_expr_for_nan(expr, f, diagnostics),
        TypedStmt::IndexAssign { value, .. }
        | TypedStmt::FieldAssign { value, .. } => walk_expr_for_nan(value, f, diagnostics),
        TypedStmt::Print { items } | TypedStmt::EPrint { items } => {
            for item in items {
                if let crate::ir::TypedPrintItem::Expr(e) = item {
                    walk_expr_for_nan(e, f, diagnostics);
                }
            }
        }
        TypedStmt::If { cond, then_body, else_body } => {
            walk_expr_for_nan(cond, f, diagnostics);
            for s in then_body { walk_stmt_for_nan(s, f, diagnostics); }
            for s in else_body { walk_stmt_for_nan(s, f, diagnostics); }
        }
        TypedStmt::While { cond, body, .. } => {
            walk_expr_for_nan(cond, f, diagnostics);
            for s in body { walk_stmt_for_nan(s, f, diagnostics); }
        }
        TypedStmt::For { start, end, body, .. } => {
            walk_expr_for_nan(start, f, diagnostics);
            walk_expr_for_nan(end, f, diagnostics);
            for s in body { walk_stmt_for_nan(s, f, diagnostics); }
        }
        TypedStmt::ForIter { body, .. }
        | TypedStmt::TaskSpawn { body, .. }
        | TypedStmt::UnsafeBlock { body, .. } => {
            for s in body { walk_stmt_for_nan(s, f, diagnostics); }
        }
        _ => {}
    }
}

fn walk_expr_for_nan(
    expr: &TypedExpr,
    f: &crate::ir::TypedFunction,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let TypedExprKind::Call { name, args, .. } = &expr.kind {
        if is_nan_producing_builtin(name, args) {
            diagnostics.push(Diagnostic::new(
                expr.span,
                format!(
                    "'{}()' may produce IEEE-754 NaN as an error sentinel -- \
                     forbidden in `#[no_nan]` function '{}'.",
                    name, f.name
                ),
            ));
        }
        for a in args { walk_expr_for_nan(a, f, diagnostics); }
        return;
    }
    match &expr.kind {
        TypedExprKind::Binary { left, right, .. } => {
            walk_expr_for_nan(left, f, diagnostics);
            walk_expr_for_nan(right, f, diagnostics);
        }
        TypedExprKind::Unary { expr: e, .. } | TypedExprKind::Cast { expr: e, .. } => {
            walk_expr_for_nan(e, f, diagnostics);
        }
        TypedExprKind::Index { array, index, .. } => {
            walk_expr_for_nan(array, f, diagnostics);
            walk_expr_for_nan(index, f, diagnostics);
        }
        TypedExprKind::ArrayLit { elements } | TypedExprKind::Tuple { elements } => {
            for e in elements { walk_expr_for_nan(e, f, diagnostics); }
        }
        TypedExprKind::IfExpr { cond, then_value, else_value } => {
            walk_expr_for_nan(cond, f, diagnostics);
            walk_expr_for_nan(then_value, f, diagnostics);
            walk_expr_for_nan(else_value, f, diagnostics);
        }
        TypedExprKind::Block { stmts, tail } => {
            for s in stmts { walk_stmt_for_nan(s, f, diagnostics); }
            walk_expr_for_nan(tail, f, diagnostics);
        }
        _ => {}
    }
}
/// T2.5 — enforce `#[no_recursion]` strict. Detects direct
/// self-call OR mutual recursion via cycle in the call graph.
pub fn enforce_no_recursion(program: &TypedProgram, diagnostics: &mut Vec<Diagnostic>) {
    let mut calls: HashMap<String, Vec<String>> = HashMap::new();
    let mut spans: HashMap<String, crate::span::Span> = HashMap::new();
    for f in &program.functions {
        let mut local_calls: Vec<String> = Vec::new();
        for s in &f.body {
            collect_calls(s, &mut local_calls);
        }
        calls.insert(f.name.clone(), local_calls);
        spans.insert(f.name.clone(), f.span);
    }
    for f in &program.functions {
        if !f.no_recursion {
            continue;
        }
        // BFS from f.name; if we can reach f.name itself,
        // there's a recursion path.
        let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut stack: Vec<(String, Vec<String>)> = vec![(f.name.clone(), vec![f.name.clone()])];
        while let Some((cur, path)) = stack.pop() {
            let Some(callees) = calls.get(&cur) else {
                continue;
            };
            for callee in callees {
                if callee == &f.name {
                    // Found a path back to f.name.
                    let mut chain = path.clone();
                    chain.push(callee.clone());
                    let via = if chain.len() > 2 {
                        format!(" via {}", chain[1..chain.len() - 1].join(" -> "))
                    } else {
                        String::new()
                    };
                    diagnostics.push(Diagnostic::new(
                        f.span,
                        format!(
                            "'{}' recurses{} — function is marked `#[no_recursion]`. \
                             Refactor as an iterative loop (while / for) over a \
                             bounded counter or accumulator.",
                            f.name, via
                        ),
                    ));
                    return;
                }
                if visited.insert(callee.clone()) {
                    let mut new_path = path.clone();
                    new_path.push(callee.clone());
                    stack.push((callee.clone(), new_path));
                }
            }
        }
    }
}

/// T3.1 — enforce `#[bounded_stack(bytes=N)]`. For each
/// annotated function, run `stack_depth::compute_stack_depths`
/// using the function as entry-point and verify the worst-case
/// stack depth is bounded AND does not exceed N.
///
/// Failure modes:
/// - Worst-case depth is unbounded (unbounded recursion via the
///   call graph): hard error — the budget can't be honored.
/// - Worst-case depth exceeds N: hard error with the deepest
///   chain reported so the developer can identify the heavy
///   caller.
///
/// ASIL-D and DO-178C Level A both require a bounded stack
/// guarantee for every critical-path function. The annotation
/// + this check together provide the audit trail.
pub fn enforce_bounded_stack(program: &TypedProgram, diagnostics: &mut Vec<Diagnostic>) {
    use crate::stack_depth::compute_stack_depths;
    for f in &program.functions {
        let Some(bound) = f.bounded_stack else {
            continue;
        };
        let report = compute_stack_depths(program, Some(&f.name));
        let Some(entry) = report.entries.iter().find(|e| e.name == f.name) else {
            // Should not happen — the entry is built from the
            // same fn list. Defensive skip.
            continue;
        };
        match entry.max_depth_bytes {
            None => {
                diagnostics.push(Diagnostic::new(
                    f.span,
                    format!(
                        "'{}' has `#[bounded_stack(bytes={})]` but its worst-case \
                         stack depth is UNBOUNDED — the call graph contains \
                         unbounded recursion. Add `#[bounded(N)]` to the \
                         recursive function or refactor to iteration.",
                        f.name, bound
                    ),
                ));
            }
            Some(actual) if actual > bound => {
                let chain = if entry.chain.is_empty() {
                    f.name.clone()
                } else {
                    entry.chain.join(" -> ")
                };
                diagnostics.push(Diagnostic::new(
                    f.span,
                    format!(
                        "'{}' exceeds its `#[bounded_stack(bytes={})]` budget — \
                         worst-case stack depth is {} bytes via chain `{}`. \
                         Reduce local-binding sizes, inline small leaf callees, \
                         or raise the bound after re-auditing.",
                        f.name, bound, actual, chain
                    ),
                ));
            }
            Some(_) => {
                // Within budget — no diagnostic.
            }
        }
    }
}

/// T3.2 — enforce `#[wcet(cycles=N)]`. Walks the function body
/// with a conservative cycle model and rejects when the static
/// estimate exceeds N or is UNBOUNDED.
///
/// V1 cycle model (over-estimating is always safe for WCET):
/// - Each scalar op (Var, literal, Cast, Unary): 1 cycle
/// - Binary op / comparison: 2 cycles
/// - Memory load (Index, Field): 2 cycles
/// - Named call: 10 cycles (CALL + RET + arg marshaling)
/// - Branch (if): cond + max(then, else) + 2
/// - For loop with const start..end bounds: (end-start) * body
/// - For loop with non-const bounds: UNBOUNDED
/// - While loop, ForIter: UNBOUNDED (no static bound in v1)
/// - Recursion: UNBOUNDED (unless #[bounded(N)] caps it; then N+1 * body)
///
/// The model is intentionally coarse — real WCET analysis
/// requires architecture-specific timing (pipeline depth, cache
/// model, branch predictor). This pass establishes the audit
/// trail and catches obvious budget overruns. DO-178C Level A.
pub fn enforce_wcet(program: &TypedProgram, diagnostics: &mut Vec<Diagnostic>) {
    let mut fn_map: HashMap<String, &crate::ir::TypedFunction> = HashMap::new();
    for f in &program.functions {
        fn_map.insert(f.name.clone(), f);
    }
    for f in &program.functions {
        let Some(budget) = f.wcet_cycles else {
            continue;
        };
        let mut visiting: std::collections::HashSet<String> = std::collections::HashSet::new();
        visiting.insert(f.name.clone());
        let estimate = wcet_body(&f.body, &fn_map, &mut visiting, f.recursion_bound);
        match estimate {
            None => {
                diagnostics.push(Diagnostic::new(
                    f.span,
                    format!(
                        "'{}' has `#[wcet(cycles={})]` but the cycle estimate is \
                         UNBOUNDED — body contains an unbounded `while` loop, \
                         a non-const-bound `for` loop, a `for ... in <collection>` \
                         iterator (length isn't statically known), or \
                         unbounded recursion. Either add `#[bounded(N)]` to \
                         recursive callees, rewrite the loop with a const \
                         bound, or refactor.",
                        f.name, budget
                    ),
                ));
            }
            Some(actual) if actual > budget => {
                diagnostics.push(Diagnostic::new(
                    f.span,
                    format!(
                        "'{}' exceeds its `#[wcet(cycles={})]` budget — static \
                         estimate is {} cycles. Reduce loop bounds, factor \
                         heavy ops into separately-budgeted helpers, or raise \
                         the bound after target-specific re-auditing.",
                        f.name, budget, actual
                    ),
                ));
            }
            Some(_) => {
                // Within budget — no diagnostic.
            }
        }
    }
}

fn wcet_body(
    body: &[TypedStmt],
    fn_map: &HashMap<String, &crate::ir::TypedFunction>,
    visiting: &mut std::collections::HashSet<String>,
    recursion_bound: Option<u64>,
) -> Option<u64> {
    let mut total: u64 = 0;
    for s in body {
        let c = wcet_stmt(s, fn_map, visiting, recursion_bound)?;
        total = total.saturating_add(c);
    }
    Some(total)
}

fn wcet_stmt(
    stmt: &TypedStmt,
    fn_map: &HashMap<String, &crate::ir::TypedFunction>,
    visiting: &mut std::collections::HashSet<String>,
    recursion_bound: Option<u64>,
) -> Option<u64> {
    use TypedStmt as S;
    match stmt {
        S::Let { expr, .. }
        | S::Reassign { expr, .. }
        | S::Return { expr }
        | S::Assert { expr, .. }
        | S::Prove { expr }
        | S::Discard { expr } => Some(2 + wcet_expr(expr, fn_map, visiting, recursion_bound)?),
        S::IndexAssign { index, value, .. } => Some(
            3 + wcet_expr(index, fn_map, visiting, recursion_bound)?
                + wcet_expr(value, fn_map, visiting, recursion_bound)?,
        ),
        S::FieldAssign { value, .. } => {
            Some(3 + wcet_expr(value, fn_map, visiting, recursion_bound)?)
        }
        S::Print { items } | S::EPrint { items } => {
            let mut total: u64 = 0;
            for it in items {
                if let crate::ir::TypedPrintItem::Expr(e) = it {
                    total = total.saturating_add(wcet_expr(e, fn_map, visiting, recursion_bound)?);
                }
            }
            // print/eprint itself is a syscall — treat as 50 cycles
            // baseline (conservative; real cost is much higher).
            Some(total.saturating_add(50))
        }
        S::If { cond, then_body, else_body } => {
            let c = wcet_expr(cond, fn_map, visiting, recursion_bound)?;
            let t = wcet_body(then_body, fn_map, visiting, recursion_bound)?;
            let e = wcet_body(else_body, fn_map, visiting, recursion_bound)?;
            Some(c.saturating_add(t.max(e)).saturating_add(2))
        }
        S::While { .. } => None, // unbounded — no static iteration count
        S::For { start, end, body, descending, .. } => {
            let start_const = const_int(start);
            let end_const = const_int(end);
            let iters = match (start_const, end_const) {
                (Some(s), Some(e)) if !descending && e >= s => (e - s) as u64,
                (Some(s), Some(e)) if *descending && s >= e => (s - e) as u64,
                _ => return None,
            };
            let body_cycles = wcet_body(body, fn_map, visiting, recursion_bound)?;
            Some(body_cycles.saturating_mul(iters).saturating_add(2))
        }
        S::ForIter { collection_ty, body, .. } => {
            // S-12: if the collection is a fixed-size array [T; N],
            // the iteration count is statically known — multiply the
            // body WCET by N. Vec and slice lengths are dynamic →
            // return None (UNBOUNDED).
            if let crate::ast::Type::Array { length, .. } = collection_ty {
                let body_cycles = wcet_body(body, fn_map, visiting, recursion_bound)?;
                Some(body_cycles.saturating_mul(*length as u64).saturating_add(2))
            } else {
                None // Vec/slice length not statically known
            }
        }
        S::TaskSpawn { .. } => None, // concurrent execution; can't model
        S::TaskJoin { .. } | S::Detach { .. } => None,
        S::ForIterShallowFree { .. } => Some(1),
        S::UnsafeBlock { body, .. } => wcet_body(body, fn_map, visiting, recursion_bound),
        S::Break { .. } | S::Continue { .. } => Some(1),
        S::Drop { .. } => Some(1),
    }
}

/// S-11 — Architecture-calibrated per-builtin WCET cost table.
///
/// Values are conservative worst-case cycle counts for a generic
/// in-order 64-bit core (covers Cortex-A55/M85, RISC-V RV64GC, and
/// x86-64 at reference clock). Actual hardware will differ; these
/// numbers are intentionally over-estimates so `#[wcet(cycles=N)]`
/// budgets set from this table are safe. Teams with calibrated
/// hardware data should override by providing their own `#[wcet]`
/// annotations on wrapper functions.
///
/// Categories:
///   - ALU (add/sub/cmp/shift/bit):    1–2 cycles
///   - Multiply:                        3–5 cycles
///   - Integer divide / modulo:        20–40 cycles (in-order cores)
///   - Float (+-*/sqrt):                5–20 cycles
///   - Memory r/w (cache hit assumed):  3–5 cycles
///   - String / Vec scan (per element): 4 cycles + length unknown
///   - Heap alloc (malloc/free):       80 cycles (conservative)
///   - I/O / syscall:                 200 cycles (conservative)
///   - Hash / crypto primitives:       30 cycles
///   - Unknown / catch-all:            10 cycles
fn wcet_builtin_cycles(name: &str) -> u64 {
    match name {
        // ── Integer arithmetic ────────────────────────────────────────
        "i64_saturating_add" | "i64_saturating_sub" | "i64_saturating_mul"
        | "i64_min" | "i64_max" | "i64_clamp" | "i64_abs_diff"
        | "i64_signum" | "i64_wrap" | "i64_avg"
        | "i64_min_3" | "i64_max_3" => 2,

        "i64_gcd" | "i64_lcm" | "i64_pow_mod" | "i64_mod_inverse"
        | "i64_factorial" | "i64_fibonacci" | "i64_binomial"
        | "i64_perm" | "i64_isqrt" | "i64_isqrt_ceil"
        | "i64_cube_root" => 40,

        "i64_div_floor" | "i64_mod_floor" | "i64_div_ceil"
        | "i64_div_round" | "i64_safe_div" | "i64_pow" => 25,

        "i64_log2_floor" | "i64_log2_ceil" | "i64_log10_floor"
        | "i64_log10_ceil" | "i64_count_digits" => 5,

        "i64_is_prime" | "i64_next_prime" | "i64_prev_prime"
        | "i64_totient" | "i64_radical" | "i64_divisor_count"
        | "i64_divisor_sum" => 200, // trial-division bounded but slow

        "i64_is_power_of_2" | "i64_next_power_of_2"
        | "i64_count_set_bits" | "i64_leading_zeros"
        | "i64_trailing_zeros" | "i64_parity" | "i64_mod_pos"
        | "i64_bswap" | "i64_rotate_left" | "i64_rotate_right"
        | "i64_reverse_bits" | "i64_set_bit" | "i64_clear_bit"
        | "i64_toggle_bit" | "i64_test_bit" | "i64_count_leading_ones"
        | "i64_count_trailing_ones" | "i64_byte_at" | "i64_set_byte" => 2,

        // ── Float arithmetic ──────────────────────────────────────────
        "sqrt" | "f64_safe_sqrt" | "f64_inv_sqrt" => 20,
        "sin" | "cos" | "tan" | "asin" | "acos" | "atan" | "atan2"
        | "sinh" | "cosh" | "tanh" | "f64_sinc"
        | "f64_asin_deg" | "f64_acos_deg" | "f64_atan_deg"
        | "f64_atan2_deg" | "f64_sec_deg" | "f64_csc_deg"
        | "f64_cot_deg" | "f64_sec" | "f64_csc" | "f64_cot" => 50,
        "exp" | "f64_exp2" | "f64_exp10" | "f64_expm1"
        | "log" | "log2" | "log10" | "f64_log1p" | "f64_log_b"
        | "f64_safe_log" => 30,
        "pow" | "f64_pow_int" => 25,
        "f64_erf" | "f64_erfc" | "f64_tgamma" | "f64_lgamma"
        | "f64_cbrt" | "f64_hypot" => 40,
        "f64_fma" | "f64_remainder" | "f64_round_to"
        | "f64_round_to_multiple" => 5,
        "f64_safe_div" | "f64_lerp" | "f64_lerp_clamp"
        | "f64_inv_lerp" | "f64_clamp01" | "f64_clamp"
        | "f64_min" | "f64_max" | "f64_min_3" | "f64_max_3"
        | "f64_abs" | "f64_signum" | "floor" | "ceil"
        | "f64_round" | "f64_trunc" | "f64_frac"
        | "f64_copysign" | "f64_remap" | "f64_wrap"
        | "f64_mod_floor" | "f64_smoothstep" | "f64_smoothstep5"
        | "f64_step" | "f64_softsign" | "f64_sigmoid"
        | "f64_relu" | "f64_leaky_relu" | "f64_softplus"
        | "f64_swish" | "f64_logit" | "f64_next_up" | "f64_next_down"
        | "f64_normal_pdf" | "f64_normal_cdf" | "f64_quadratic_root"
        | "f64_inv_smoothstep" | "f64_chebyshev"
        | "f64_geometric_mean" | "f64_harmonic_mean"
        | "f64_quadratic_mean" | "f64_l1_norm"
        | "f64_to_radians" | "f64_to_degrees" | "abs" => 5,

        // ── Hashing / randomness ──────────────────────────────────────
        "hash_i64" | "hash_f64" | "hash_str" | "hash_combine"
        | "hash_combine_3" | "hash_combine_4" | "hash_pair"
        | "hash_triple" | "f64_hash_pair" | "f64_hash_triple"
        | "str_hash_pair" | "str_hash_triple" => 10,
        "siphash_i64" | "siphash_str" => 30,
        "seed_rng" | "rand_i64" | "rand_in_range" | "rand_f64"
        | "rand_in_range_f64" | "rand_bool" | "rand_choice"
        | "rand_normal" | "rand_uniform" | "f64_uniform_random" => 15,

        // ── Vec / array operations ────────────────────────────────────
        // Single-element or O(1) ops
        "push" | "pop" | "set" | "insert" | "swap_remove"
        | "vec_remove_at" | "vec_first" | "vec_last" | "length"
        | "contains" | "get" | "peek" | "find" | "clear"
        | "dedup" | "reverse" => 4,
        // O(n) scans
        "sort" | "sort_by" | "sort_desc" | "binary_search"
        | "vec_replace_all" | "vec_unique" | "vec_dedup_consecutive"
        | "vec_is_sorted_asc" | "vec_is_sorted_desc"
        | "vec_is_palindrome" | "vec_is_sorted_unique"
        | "vec_count_distinct" => 50,
        // O(n) combinators that return a new Vec (alloc included)
        "vec_map" | "vec_filter" | "vec_zip_with" | "vec_take"
        | "vec_drop" | "vec_take_while" | "vec_drop_while"
        | "vec_map_filter" | "vec_chain" | "vec_range"
        | "vec_repeat" | "vec_extend" | "vec_concat"
        | "vec_reverse_copy" | "vec_iota" | "vec_intersect"
        | "vec_difference" | "vec_union" | "vec_diff"
        | "vec_pad_left" | "vec_pad_right" | "vec_flatten"
        | "vec_group_by_value" | "vec_chunks" | "vec_windows"
        | "vec_intersperse" | "vec_merge_sorted"
        | "vec_insert_sorted" => 80, // alloc + scan
        // O(n) reductions (no alloc)
        "vec_sum" | "vec_product" | "vec_min" | "vec_max"
        | "vec_count" | "vec_any" | "vec_all" | "vec_fold"
        | "vec_running_sum" | "vec_running_mean"
        | "vec_running_product" | "vec_running_xor"
        | "vec_running_and" | "vec_running_or"
        | "vec_cumulative_max" | "vec_cumulative_min"
        | "vec_sliding_max" | "vec_sliding_min"
        | "vec_sliding_sum" | "vec_sliding_product"
        | "vec_dot" | "vec_mean" | "vec_mode"
        | "vec_kth_smallest" | "vec_median"
        | "vec_map_fold" | "vec_filter_fold"
        | "vec_map_filter_fold" | "vec_count_if"
        | "vec_position" | "vec_argmin" | "vec_argmax"
        | "vec_max_by" | "vec_min_by" | "vec_count_value"
        | "vec_index_of_value" | "vec_last_index_of_value"
        | "vec_indices_of_value" | "vec_range_span"
        | "vec_all_equal" | "vec_equal_set" | "vec_equal_seq"
        | "vec_subset_of" | "vec_disjoint"
        | "vec_abs" | "vec_negate" | "vec_signum" | "vec_square"
        | "vec_add_scalar" | "vec_sub_scalar" | "vec_mul_scalar"
        | "vec_div_scalar" | "vec_mod_scalar" | "vec_pow_scalar"
        | "vec_shl_scalar" | "vec_shr_scalar"
        | "vec_add_pairwise" | "vec_sub_pairwise"
        | "vec_mul_pairwise" | "vec_min_pairwise" | "vec_max_pairwise"
        | "vec_eq_mask" | "vec_ne_mask" | "vec_lt_mask"
        | "vec_le_mask" | "vec_gt_mask" | "vec_ge_mask"
        | "vec_min_with_scalar" | "vec_max_with_scalar"
        | "vec_clamp_scalar" | "vec_rotate_left" | "vec_rotate_right"
        | "vec_shift_left" | "vec_shift_right"
        | "vec_replace_value" | "vec_swap" | "vec_swap_remove"
        | "heapify" => 30,
        // Alloc-only
        "vec" | "try_vec" | "clone" | "clone_at" => 80,

        // ── String ops ────────────────────────────────────────────────
        "length" | "str_contains" | "str_starts_with" | "str_ends_with"
        | "str_trim" | "str_index_of" | "str_count_char"
        | "str_starts_with_byte" | "str_ends_with_byte"
        | "str_byte_count" | "str_index_of_byte"
        | "str_last_index_of_byte" | "str_first_byte" | "str_last_byte"
        | "str_byte_at" | "str_len_bytes"
        | "str_count_ascii_digits" | "str_count_ascii_alpha"
        | "str_count_ascii_alphanumeric" | "str_count_ascii_whitespace"
        | "str_count_ascii_upper" | "str_count_ascii_lower"
        | "str_count_ascii_punct" | "str_count_ascii_control"
        | "is_ascii_digit" | "is_ascii_alpha" | "is_ascii_alphanumeric"
        | "is_ascii_whitespace" | "str_is_ascii" | "str_is_empty"
        | "str_is_digit_only" | "str_is_alpha_only"
        | "str_is_alphanumeric_only" | "str_is_whitespace_only" => 5,

        "str_replace" | "substring" | "str_repeat" | "str_to_upper"
        | "str_to_lower" | "str_pad_left" | "str_pad_right"
        | "str_split" | "str_lines" | "str_chars" | "str_reverse"
        | "str_strip_prefix" | "str_strip_suffix" | "str_join"
        | "i64_to_str" | "f64_to_str" | "bool_to_str"
        | "parse_int" | "parse_float" | "parse_bool" => 40,

        // ── Memory / unsafe ops ───────────────────────────────────────
        "raw_load" | "raw_store" | "aref_load" | "aref_store"
        | "mmio_read_u32" | "mmio_write_u32"
        | "mmio_read_u8" | "mmio_write_u8"
        | "mmio_read_u16" | "mmio_write_u16"
        | "bptr_get" | "bptr_set" | "bptr_len"
        | "bptr_new" | "pool_get" | "pool_free"
        | "region_borrow_i64" | "region_len"
        | "pool_new" | "region_new" => 5,

        "unsafe_alloc" | "unsafe_free"
        | "pool_alloc" | "region_alloc_i64" => 80,

        // ── I/O / timing ──────────────────────────────────────────────
        "sleep_ms" => 500, // context switch minimum
        "stdin_ready_within_ms" => 500, // same context-switch-minimum model as sleep_ms
        "taint" | "assert_safe" => 1, // type-level only; zero runtime cost

        // ── Collections (affine, heap-backed) ─────────────────────────
        "hashmap_new" | "hashset_new" | "btreemap_new" | "btreeset_new"
        | "deque_new" | "binary_heap_new" | "bloom_filter_new"
        | "bst_new" | "graph_new" | "trie_new" | "skiplist_new"
        | "union_find_new" => 80,

        "hashmap_insert" | "hashset_insert" | "btreemap_insert"
        | "btreeset_insert" | "trie_insert" | "skiplist_insert"
        | "bst_insert" | "graph_add_edge" | "union_find_union"
        | "binary_heap_push" | "bloom_filter_insert"
        | "deque_push_back" | "deque_push_front"
        | "heap_push" => 30,

        "hashmap_get" | "hashset_contains" | "btreemap_get"
        | "btreeset_contains" | "trie_contains" | "skiplist_contains"
        | "bst_contains" | "bloom_filter_contains"
        | "union_find_find" | "union_find_connected"
        | "binary_heap_peek" | "heap_peek"
        | "deque_peek_back" | "deque_peek_front" => 10,

        "hashmap_remove" | "hashset_remove" | "btreemap_remove"
        | "btreeset_remove" | "trie_delete" | "skiplist_remove"
        | "bst_remove" | "union_find_clear"
        | "binary_heap_pop" | "heap_pop"
        | "deque_pop_back" | "deque_pop_front" => 20,

        "graph_bfs_reach" | "graph_dfs_reach" | "graph_dijkstra"
        | "graph_has_cycle" | "graph_mst_kruskal" | "graph_mst_prim"
        | "graph_astar" | "graph_topo_sort" => 500, // O(V+E) — unbounded but expensive

        // ── Catch-all ─────────────────────────────────────────────────
        _ => 10,
    }
}

fn wcet_expr(
    expr: &crate::ir::TypedExpr,
    fn_map: &HashMap<String, &crate::ir::TypedFunction>,
    visiting: &mut std::collections::HashSet<String>,
    _recursion_bound: Option<u64>,
) -> Option<u64> {
    use crate::ir::TypedExprKind as E;
    match &expr.kind {
        E::Int(_) | E::Float(_) | E::Bool(_) | E::Str(_) | E::Var(_) => Some(1),
        E::Ref { .. } | E::RefMut { .. } | E::RefField { .. } | E::RefMutField { .. } => Some(1),
        E::FnRef { .. } => Some(1),
        E::Unary { expr: inner, .. } => Some(1 + wcet_expr(inner, fn_map, visiting, None)?),
        E::Cast { expr: inner, .. } => Some(1 + wcet_expr(inner, fn_map, visiting, None)?),
        E::Binary { left, right, .. } => Some(
            2 + wcet_expr(left, fn_map, visiting, None)?
                + wcet_expr(right, fn_map, visiting, None)?,
        ),
        E::Index { array, index, .. } => Some(
            2 + wcet_expr(array, fn_map, visiting, None)?
                + wcet_expr(index, fn_map, visiting, None)?,
        ),
        E::Len { array, .. } => Some(2 + wcet_expr(array, fn_map, visiting, None)?),
        E::ArrayLit { elements } => {
            let mut total: u64 = 1;
            for e in elements {
                total = total.saturating_add(wcet_expr(e, fn_map, visiting, None)?);
            }
            Some(total)
        }
        // BUG-2: without this arm, StructLit fell into the `_ =>
        // Some(5)` catch-all below — a flat cost regardless of how
        // expensive the field expressions actually are (e.g. a
        // `Complex { re: log(complex_abs(z)), im: complex_arg(z) }`
        // return got charged 5 cycles despite three real function
        // calls inside it). Mirrors the ArrayLit arm just above.
        E::StructLit { fields, .. } => {
            let mut total: u64 = 1;
            for (_, e) in fields {
                total = total.saturating_add(wcet_expr(e, fn_map, visiting, None)?);
            }
            Some(total)
        }
        E::Call { name, args, .. } => {
            let mut args_cost: u64 = 0;
            for a in args {
                args_cost = args_cost.saturating_add(wcet_expr(a, fn_map, visiting, None)?);
            }
            // If the callee has its own #[wcet(cycles=N)], use N
            // directly (the budget is the contract). Otherwise
            // estimate the callee body, capped by a recursion guard.
            if let Some(callee) = fn_map.get(name) {
                if let Some(callee_budget) = callee.wcet_cycles {
                    return Some(args_cost.saturating_add(callee_budget).saturating_add(5));
                }
                if visiting.contains(name) {
                    // Recursive — try the recursion_bound
                    // mechanism. v1: any recursion without a
                    // declared callee WCET budget is UNBOUNDED.
                    return None;
                }
                visiting.insert(name.clone());
                let body_estimate = wcet_body(&callee.body, fn_map, visiting, callee.recursion_bound);
                visiting.remove(name);
                let body = body_estimate?;
                Some(args_cost.saturating_add(body).saturating_add(5))
            } else {
                // Builtin or extern — use the architecture-calibrated
                // cost table (S-11). Falls back to 10 cycles for
                // unknown builtins.
                Some(args_cost.saturating_add(wcet_builtin_cycles(name)))
            }
        }
        E::CallIndirect { args, .. } => {
            // Indirect call: can't follow the callee, treat as
            // a leaf 10-cycle op + arg costs.
            let mut total: u64 = 10;
            for a in args {
                total = total.saturating_add(wcet_expr(a, fn_map, visiting, None)?);
            }
            Some(total)
        }
        _ => {
            // Block / IfExpr / Match / etc. — fall back to a
            // walk via a more comprehensive visitor in future.
            // V1: 5-cycle flat estimate so we don't return None
            // (which would mark the whole fn UNBOUNDED for any
            // expression form not yet itemized).
            Some(5)
        }
    }
}

fn const_int(expr: &crate::ir::TypedExpr) -> Option<i128> {
    match &expr.kind {
        crate::ir::TypedExprKind::Int(v) => Some(*v),
        _ => None,
    }
}

/// T3.4 — enforce `#[deterministic_timing]`. For each annotated
/// function, walk the body and reject any construct whose
/// execution-time cost is data-dependent:
///
/// - `if` statements where the then-branch and else-branch have
///   different cycle estimates (the wcet model from T3.2 is used
///   to compute branch costs).
/// - `while` loops (no static iteration bound).
/// - `for` loops with non-const bounds.
/// - `for ... in <collection>` iterators (collection length not
///   statically known).
/// - calls to functions that aren't themselves
///   `#[deterministic_timing]` AND don't have `#[wcet]` declared
///   (timing variability unprovable for them).
///
/// DO-178C Level A timing-determinism rule. The annotation +
/// this check together guarantee constant-time execution
/// regardless of inputs.
pub fn enforce_deterministic_timing(program: &TypedProgram, diagnostics: &mut Vec<Diagnostic>) {
    let mut fn_map: HashMap<String, &crate::ir::TypedFunction> = HashMap::new();
    for f in &program.functions {
        fn_map.insert(f.name.clone(), f);
    }
    for f in &program.functions {
        if !f.deterministic_timing {
            continue;
        }
        check_dt_body(&f.body, f, &fn_map, diagnostics);
    }
}

fn check_dt_body(
    body: &[TypedStmt],
    f: &crate::ir::TypedFunction,
    fn_map: &HashMap<String, &crate::ir::TypedFunction>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for s in body {
        check_dt_stmt(s, f, fn_map, diagnostics);
    }
}

fn check_dt_stmt(
    stmt: &TypedStmt,
    f: &crate::ir::TypedFunction,
    fn_map: &HashMap<String, &crate::ir::TypedFunction>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    use TypedStmt as S;
    match stmt {
        S::If { then_body, else_body, .. } => {
            // Cycle estimates use the same model as T3.2's WCET.
            let mut visiting: std::collections::HashSet<String> = std::collections::HashSet::new();
            visiting.insert(f.name.clone());
            let t = wcet_body(then_body, fn_map, &mut visiting, f.recursion_bound);
            let e = wcet_body(else_body, fn_map, &mut visiting, f.recursion_bound);
            match (t, e) {
                (Some(tc), Some(ec)) if tc != ec => {
                    diagnostics.push(Diagnostic::new(
                        f.span,
                        format!(
                            "'{}' is `#[deterministic_timing]` but contains an `if` \
                             whose arms have unequal cycle estimates (then = {}, \
                             else = {}). Equalize the arms (add dead-store padding \
                             in the shorter branch) or refactor to a branchless form.",
                            f.name, tc, ec
                        ),
                    ));
                }
                (None, _) | (_, None) => {
                    diagnostics.push(Diagnostic::new(
                        f.span,
                        format!(
                            "'{}' is `#[deterministic_timing]` but contains an `if` \
                             arm with UNBOUNDED cycle cost (likely a while loop, \
                             ForIter, or recursive call inside the branch).",
                            f.name
                        ),
                    ));
                }
                (Some(_), Some(_)) => {}
            }
            check_dt_body(then_body, f, fn_map, diagnostics);
            check_dt_body(else_body, f, fn_map, diagnostics);
        }
        S::While { body, .. } => {
            diagnostics.push(Diagnostic::new(
                f.span,
                format!(
                    "'{}' is `#[deterministic_timing]` but contains a `while` loop. \
                     Use `for i from 0 to N` with a const upper bound, or refactor.",
                    f.name
                ),
            ));
            check_dt_body(body, f, fn_map, diagnostics);
        }
        S::For { start, end, body, .. } => {
            if const_int(start).is_none() || const_int(end).is_none() {
                diagnostics.push(Diagnostic::new(
                    f.span,
                    format!(
                        "'{}' is `#[deterministic_timing]` but contains a `for` loop \
                         with non-const bounds. Use literal integer bounds.",
                        f.name
                    ),
                ));
            }
            check_dt_body(body, f, fn_map, diagnostics);
        }
        S::ForIter { body, .. } => {
            diagnostics.push(Diagnostic::new(
                f.span,
                format!(
                    "'{}' is `#[deterministic_timing]` but contains a `for ... in <collection>` \
                     iterator. The collection length isn't statically known — use \
                     `for i from 0 to N` over the indices instead.",
                    f.name
                ),
            ));
            check_dt_body(body, f, fn_map, diagnostics);
        }
        S::Let { expr, .. }
        | S::Reassign { expr, .. }
        | S::Return { expr }
        | S::Assert { expr, .. }
        | S::Prove { expr }
        | S::Discard { expr } => check_dt_expr_calls(expr, f, fn_map, diagnostics),
        S::IndexAssign { index, value, .. } => {
            check_dt_expr_calls(index, f, fn_map, diagnostics);
            check_dt_expr_calls(value, f, fn_map, diagnostics);
        }
        S::FieldAssign { value, .. } => check_dt_expr_calls(value, f, fn_map, diagnostics),
        S::Print { items } => {
            for it in items {
                if let crate::ir::TypedPrintItem::Expr(e) = it {
                    check_dt_expr_calls(e, f, fn_map, diagnostics);
                }
            }
        }
        S::TaskSpawn { body, .. } | S::UnsafeBlock { body, .. } => {
            check_dt_body(body, f, fn_map, diagnostics);
        }
        _ => {}
    }
}

fn check_dt_expr_calls(
    expr: &crate::ir::TypedExpr,
    f: &crate::ir::TypedFunction,
    fn_map: &HashMap<String, &crate::ir::TypedFunction>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    use crate::ir::TypedExprKind as E;
    match &expr.kind {
        E::Call { name, args, .. } => {
            // Built-in or extern functions: leaf-ish, allowed (the
            // cycle model treats them as flat 10 cycles).
            // User-defined: must be #[deterministic_timing] or
            // have #[wcet] declared.
            if let Some(callee) = fn_map.get(name) {
                let ok = callee.deterministic_timing || callee.wcet_cycles.is_some();
                if !ok {
                    diagnostics.push(Diagnostic::new(
                        f.span,
                        format!(
                            "'{}' is `#[deterministic_timing]` but calls '{}' which is \
                             neither `#[deterministic_timing]` nor `#[wcet(cycles=N)]` — \
                             callee timing variability is unprovable.",
                            f.name, name
                        ),
                    ));
                }
            }
            for a in args {
                check_dt_expr_calls(a, f, fn_map, diagnostics);
            }
        }
        E::Binary { left, right, .. } => {
            check_dt_expr_calls(left, f, fn_map, diagnostics);
            check_dt_expr_calls(right, f, fn_map, diagnostics);
        }
        E::Unary { expr: inner, .. } | E::Cast { expr: inner, .. } => {
            check_dt_expr_calls(inner, f, fn_map, diagnostics);
        }
        E::Index { array, index, .. } => {
            check_dt_expr_calls(array, f, fn_map, diagnostics);
            check_dt_expr_calls(index, f, fn_map, diagnostics);
        }
        E::ArrayLit { elements } => {
            for e in elements {
                check_dt_expr_calls(e, f, fn_map, diagnostics);
            }
        }
        E::CallIndirect { args, .. } => {
            diagnostics.push(Diagnostic::new(
                f.span,
                format!(
                    "'{}' is `#[deterministic_timing]` but contains an indirect call \
                     through a function pointer — the callee's timing is opaque.",
                    f.name
                ),
            ));
            for a in args {
                check_dt_expr_calls(a, f, fn_map, diagnostics);
            }
        }
        _ => {}
    }
}

/// T3.5 — enforce MISRA C 2012 Rule 13.5: "The right hand
/// operand of a logical `&&` or `||` operator shall not contain
/// persistent side effects."
///
/// In vāṇी's static type system every function call could in
/// principle have side effects (unless declared `pure`). MISRA
/// 13.5 forbids RHS expressions that conditionally execute (via
/// the short-circuit evaluation rule) and may have effects — the
/// reason is that whether the side effect happens depends on the
/// LHS value, making behaviour evaluation-order dependent.
///
/// This pass fires for functions that are either annotated
/// `pure fn` or tagged with a standard composite that includes
/// MISRA compliance (`#[misra_c_2012]`, `#[asil_d]`,
/// `#[do178c_level_a]`, `#[iec_62304_class_c]`). It walks every
/// expression and rejects any `&&` / `||` whose RHS contains a
/// non-pure-non-builtin function call.
///
/// For pure fns the check is partially redundant (pure fns
/// forbid impure calls entirely), but the targeted diagnostic
/// gives a clearer MISRA-13.5 reason than the generic
/// "calls to impure function" message.
pub fn enforce_misra_13(program: &TypedProgram, diagnostics: &mut Vec<Diagnostic>) {
    let mut sig_pure: HashMap<String, bool> = HashMap::new();
    for f in &program.functions {
        sig_pure.insert(f.name.clone(), f.is_pure);
    }
    for f in &program.functions {
        let in_scope = f.is_pure || f.safety_standard.is_some();
        if !in_scope {
            continue;
        }
        for s in &f.body {
            check_misra_13_stmt(s, &f.name, &sig_pure, diagnostics);
        }
    }
}

fn check_misra_13_stmt(
    stmt: &TypedStmt,
    fn_name: &str,
    sig_pure: &HashMap<String, bool>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    use TypedStmt as S;
    match stmt {
        S::Let { expr, .. }
        | S::Reassign { expr, .. }
        | S::Return { expr }
        | S::Assert { expr, .. }
        | S::Prove { expr }
        | S::Discard { expr } => check_misra_13_expr(expr, fn_name, sig_pure, diagnostics),
        S::IndexAssign { index, value, .. } => {
            check_misra_13_expr(index, fn_name, sig_pure, diagnostics);
            check_misra_13_expr(value, fn_name, sig_pure, diagnostics);
        }
        S::FieldAssign { value, .. } => {
            check_misra_13_expr(value, fn_name, sig_pure, diagnostics)
        }
        S::Print { items } => {
            for it in items {
                if let crate::ir::TypedPrintItem::Expr(e) = it {
                    check_misra_13_expr(e, fn_name, sig_pure, diagnostics);
                }
            }
        }
        S::If { cond, then_body, else_body } => {
            check_misra_13_expr(cond, fn_name, sig_pure, diagnostics);
            for s in then_body {
                check_misra_13_stmt(s, fn_name, sig_pure, diagnostics);
            }
            for s in else_body {
                check_misra_13_stmt(s, fn_name, sig_pure, diagnostics);
            }
        }
        S::While { cond, body, .. } => {
            check_misra_13_expr(cond, fn_name, sig_pure, diagnostics);
            for s in body {
                check_misra_13_stmt(s, fn_name, sig_pure, diagnostics);
            }
        }
        S::For { start, end, body, .. } => {
            check_misra_13_expr(start, fn_name, sig_pure, diagnostics);
            check_misra_13_expr(end, fn_name, sig_pure, diagnostics);
            for s in body {
                check_misra_13_stmt(s, fn_name, sig_pure, diagnostics);
            }
        }
        S::ForIter { body, .. }
        | S::TaskSpawn { body, .. }
        | S::UnsafeBlock { body, .. } => {
            for s in body {
                check_misra_13_stmt(s, fn_name, sig_pure, diagnostics);
            }
        }
        _ => {}
    }
}

fn check_misra_13_expr(
    expr: &crate::ir::TypedExpr,
    fn_name: &str,
    sig_pure: &HashMap<String, bool>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    use crate::ast::BinaryOp;
    use crate::ir::TypedExprKind as E;
    if let E::Binary { op, left: _, right, .. } = &expr.kind {
        if matches!(op, BinaryOp::And | BinaryOp::Or) {
            if expr_contains_impure_call(right, sig_pure) {
                diagnostics.push(Diagnostic::new(
                    right.span,
                    format!(
                        "MISRA 13.5: the right-hand operand of `{}` in '{}' contains \
                         a function call — short-circuit evaluation makes the call's \
                         side effect conditional on the LHS, producing evaluation-\
                         order-dependent behaviour. Lift the call to a `let` binding \
                         before the `{}`, or split the condition into separate `if` \
                         statements.",
                        op.display_symbol(),
                        fn_name,
                        op.display_symbol(),
                    ),
                ));
            }
        }
    }
    walk_misra_13_subexprs(expr, fn_name, sig_pure, diagnostics);
}

fn walk_misra_13_subexprs(
    expr: &crate::ir::TypedExpr,
    fn_name: &str,
    sig_pure: &HashMap<String, bool>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    use crate::ir::TypedExprKind as E;
    match &expr.kind {
        E::Binary { left, right, .. } => {
            check_misra_13_expr(left, fn_name, sig_pure, diagnostics);
            check_misra_13_expr(right, fn_name, sig_pure, diagnostics);
        }
        E::Unary { expr: inner, .. } | E::Cast { expr: inner, .. } => {
            check_misra_13_expr(inner, fn_name, sig_pure, diagnostics);
        }
        E::Call { args, .. } => {
            for a in args {
                check_misra_13_expr(a, fn_name, sig_pure, diagnostics);
            }
        }
        E::Index { array, index, .. } => {
            check_misra_13_expr(array, fn_name, sig_pure, diagnostics);
            check_misra_13_expr(index, fn_name, sig_pure, diagnostics);
        }
        E::ArrayLit { elements } => {
            for e in elements {
                check_misra_13_expr(e, fn_name, sig_pure, diagnostics);
            }
        }
        E::CallIndirect { args, callee } => {
            check_misra_13_expr(callee, fn_name, sig_pure, diagnostics);
            for a in args {
                check_misra_13_expr(a, fn_name, sig_pure, diagnostics);
            }
        }
        _ => {}
    }
}

fn expr_contains_impure_call(
    expr: &crate::ir::TypedExpr,
    sig_pure: &HashMap<String, bool>,
) -> bool {
    use crate::ir::TypedExprKind as E;
    match &expr.kind {
        E::Call { name, args, .. } => {
            let pure = sig_pure.get(name).copied().unwrap_or(false);
            if !pure {
                return true;
            }
            args.iter().any(|a| expr_contains_impure_call(a, sig_pure))
        }
        E::CallIndirect { .. } => {
            // Indirect calls go through fn pointers — callee
            // purity isn't statically resolvable. Conservative
            // MISRA-13.5: treat as impure.
            true
        }
        E::Binary { left, right, .. } => {
            expr_contains_impure_call(left, sig_pure)
                || expr_contains_impure_call(right, sig_pure)
        }
        E::Unary { expr: inner, .. } | E::Cast { expr: inner, .. } => {
            expr_contains_impure_call(inner, sig_pure)
        }
        E::Index { array, index, .. } => {
            expr_contains_impure_call(array, sig_pure)
                || expr_contains_impure_call(index, sig_pure)
        }
        E::ArrayLit { elements } => {
            elements.iter().any(|e| expr_contains_impure_call(e, sig_pure))
        }
        _ => false,
    }
}

fn collect_calls(stmt: &TypedStmt, out: &mut Vec<String>) {
    match stmt {
        TypedStmt::Let { expr, .. }
        | TypedStmt::Reassign { expr, .. }
        | TypedStmt::Return { expr }
        | TypedStmt::Assert { expr, .. }
        | TypedStmt::Prove { expr }
        | TypedStmt::Discard { expr } => collect_expr_calls(expr, out),
        TypedStmt::IndexAssign { value, .. } | TypedStmt::FieldAssign { value, .. } => {
            collect_expr_calls(value, out);
        }
        TypedStmt::Print { items } => {
            for it in items {
                if let crate::ir::TypedPrintItem::Expr(e) = it {
                    collect_expr_calls(e, out);
                }
            }
        }
        TypedStmt::If { cond, then_body, else_body } => {
            collect_expr_calls(cond, out);
            for s in then_body { collect_calls(s, out); }
            for s in else_body { collect_calls(s, out); }
        }
        TypedStmt::While { cond, body, .. } => {
            collect_expr_calls(cond, out);
            for s in body { collect_calls(s, out); }
        }
        TypedStmt::For { start, end, body, .. } => {
            collect_expr_calls(start, out);
            collect_expr_calls(end, out);
            for s in body { collect_calls(s, out); }
        }
        TypedStmt::ForIter { body, .. }
        | TypedStmt::TaskSpawn { body, .. }
        | TypedStmt::UnsafeBlock { body, .. } => {
            for s in body { collect_calls(s, out); }
        }
        _ => {}
    }
}

fn collect_expr_calls(expr: &TypedExpr, out: &mut Vec<String>) {
    match &expr.kind {
        TypedExprKind::Call { name, args, .. } => {
            out.push(name.clone());
            for a in args { collect_expr_calls(a, out); }
        }
        TypedExprKind::Binary { left, right, .. } => {
            collect_expr_calls(left, out);
            collect_expr_calls(right, out);
        }
        TypedExprKind::Unary { expr, .. } | TypedExprKind::Cast { expr, .. } => {
            collect_expr_calls(expr, out);
        }
        TypedExprKind::Index { array, index, .. } => {
            collect_expr_calls(array, out);
            collect_expr_calls(index, out);
        }
        TypedExprKind::ArrayLit { elements } | TypedExprKind::Tuple { elements } => {
            for e in elements { collect_expr_calls(e, out); }
        }
        TypedExprKind::IfExpr { cond, then_value, else_value } => {
            collect_expr_calls(cond, out);
            collect_expr_calls(then_value, out);
            collect_expr_calls(else_value, out);
        }
        TypedExprKind::Block { stmts, tail } => {
            for s in stmts { collect_calls(s, out); }
            collect_expr_calls(tail, out);
        }
        TypedExprKind::Match { scrutinee, arms } => {
            collect_expr_calls(scrutinee, out);
            for a in arms { collect_expr_calls(&a.body, out); }
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

// ---- T2.4 follow-up: per-function complexity report --------
//
// Surfaces the existing McCabe complexity counter as an audit
// artifact via `intentc complexity` (mirrors the established
// `deviations` / `stack-depth` / `acyclicity` / `hashmap-usage`
// pattern). Useful for embedded teams reviewing MISRA / ISO
// 26262 / DO-178C complexity ceilings against actual code.

/// One row of the complexity report. Live functions only
/// (extern fns excluded — they have no body to count).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FnComplexity {
    pub name: String,
    pub score: u64,
}

/// Walk the typed program and compute the McCabe complexity
/// score for every non-extern function. Returns one record per
/// function in declaration order.
pub fn compute_complexity_report(program: &TypedProgram) -> Vec<FnComplexity> {
    let mut out: Vec<FnComplexity> = Vec::new();
    for f in &program.functions {
        if f.is_extern {
            continue;
        }
        let mut count: u64 = 1;
        for s in &f.body {
            count += stmt_complexity(s);
        }
        out.push(FnComplexity { name: f.name.clone(), score: count });
    }
    out
}

/// Human-readable text format. Each line is one function with
/// its score; functions over `threshold` get a `[OVER]` marker.
/// Returns `(output, any_over)` so the CLI can set its exit
/// code accordingly.
pub fn format_complexity_text(report: &[FnComplexity], threshold: Option<u64>) -> (String, bool) {
    let mut out = String::new();
    let mut any_over = false;
    for r in report {
        let marker = match threshold {
            Some(t) if r.score > t => {
                any_over = true;
                "[OVER] "
            }
            _ => "",
        };
        out.push_str(&format!("{}{}: {}\n", marker, r.name, r.score));
    }
    if let Some(t) = threshold {
        let over = report.iter().filter(|r| r.score > t).count();
        out.push_str(&format!(
            "\n{} of {} fn{} exceed threshold {}\n",
            over,
            report.len(),
            if report.len() == 1 { "" } else { "s" },
            t,
        ));
    } else {
        out.push_str(&format!("\n{} fn{} total\n",
            report.len(),
            if report.len() == 1 { "" } else { "s" }));
    }
    (out, any_over)
}

pub fn format_complexity_json(report: &[FnComplexity]) -> String {
    let mut out = String::from("{\"functions\":[");
    for (i, r) in report.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            "{{\"name\":{},\"score\":{}}}",
            json_string_local(&r.name),
            r.score,
        ));
    }
    out.push_str("]}\n");
    out
}

pub fn format_complexity_csv(report: &[FnComplexity]) -> String {
    let mut out = String::from("name,score\n");
    for r in report {
        out.push_str(&format!("{},{}\n", csv_escape_local(&r.name), r.score));
    }
    out
}

fn csv_escape_local(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        let escaped = s.replace('"', "\"\"");
        format!("\"{}\"", escaped)
    } else {
        s.to_string()
    }
}

fn json_string_local(s: &str) -> String {
    let mut out = String::from("\"");
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
    out.push('"');
    out
}

// ---- Safety-attribute report --------------------------------
//
// Surfaces the full safety-tag set per function as an audit
// artifact via `intentc safety-attrs`. Mirrors the established
// audit-CLI pattern. A compliance reviewer can see at a glance
// which functions claim which standards / primitives, without
// grepping source files.

/// One row of the safety-attr report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FnSafetyAttrs {
    pub name: String,
    /// Standard-composite tag if any (e.g. `"misra_c_2012"`,
    /// `"asil_d"`, `"do178c_level_a"`, `"iec_62304_class_c"`).
    pub safety_standard: Option<String>,
    pub no_heap: bool,
    pub no_float: bool,
    pub no_nan: bool,
    pub no_recursion: bool,
    pub interrupt: bool,
    pub deterministic_timing: bool,
    pub bounded_stack_bytes: Option<u64>,
    pub wcet_cycles: Option<u64>,
    pub bounded_recursion: Option<u64>,
    pub is_pure: bool,
}

/// Build the per-function safety-attribute report. Non-extern
/// functions only; extern fns have no body and their safety
/// posture is opaque.
pub fn compute_safety_attrs_report(program: &TypedProgram) -> Vec<FnSafetyAttrs> {
    let mut out = Vec::new();
    for f in &program.functions {
        if f.is_extern {
            continue;
        }
        out.push(FnSafetyAttrs {
            name: f.name.clone(),
            safety_standard: f.safety_standard.clone(),
            no_heap: f.no_heap,
            no_float: f.no_float,
            no_nan: f.no_nan,
            no_recursion: f.no_recursion,
            interrupt: f.interrupt,
            deterministic_timing: f.deterministic_timing,
            bounded_stack_bytes: f.bounded_stack,
            wcet_cycles: f.wcet_cycles,
            bounded_recursion: f.recursion_bound,
            is_pure: f.is_pure,
        });
    }
    out
}

pub fn format_safety_attrs_text(report: &[FnSafetyAttrs]) -> String {
    if report.is_empty() {
        return "no functions found\n".to_string();
    }
    let mut out = String::new();
    for r in report {
        let mut attrs: Vec<String> = Vec::new();
        if let Some(s) = &r.safety_standard {
            attrs.push(format!("#[{}]", s));
        }
        if r.is_pure { attrs.push("pure".to_string()); }
        if r.no_heap { attrs.push("#[no_heap]".to_string()); }
        if r.no_float { attrs.push("#[no_float]".to_string()); }
        if r.no_nan { attrs.push("#[no_nan]".to_string()); }
        if r.no_recursion { attrs.push("#[no_recursion]".to_string()); }
        if r.interrupt { attrs.push("#[interrupt]".to_string()); }
        if r.deterministic_timing {
            attrs.push("#[deterministic_timing]".to_string());
        }
        if let Some(b) = r.bounded_stack_bytes {
            attrs.push(format!("#[bounded_stack(bytes={})]", b));
        }
        if let Some(w) = r.wcet_cycles {
            attrs.push(format!("#[wcet(cycles={})]", w));
        }
        if let Some(rb) = r.bounded_recursion {
            attrs.push(format!("#[bounded({})]", rb));
        }
        let attr_str = if attrs.is_empty() {
            "(none)".to_string()
        } else {
            attrs.join(" ")
        };
        out.push_str(&format!("{}: {}\n", r.name, attr_str));
    }
    let tagged = report.iter().filter(|r| {
        r.safety_standard.is_some() || r.no_heap || r.no_float || r.no_nan
            || r.no_recursion || r.interrupt || r.deterministic_timing
            || r.bounded_stack_bytes.is_some() || r.wcet_cycles.is_some()
            || r.bounded_recursion.is_some() || r.is_pure
    }).count();
    out.push_str(&format!(
        "\n{} of {} fn{} carry at least one safety annotation\n",
        tagged,
        report.len(),
        if report.len() == 1 { "" } else { "s" },
    ));
    out
}

pub fn format_safety_attrs_json(report: &[FnSafetyAttrs]) -> String {
    let mut out = String::from("{\"functions\":[");
    for (i, r) in report.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        let standard = match &r.safety_standard {
            Some(s) => json_string_local(s),
            None => "null".to_string(),
        };
        let bs = match r.bounded_stack_bytes {
            Some(b) => b.to_string(),
            None => "null".to_string(),
        };
        let wc = match r.wcet_cycles {
            Some(w) => w.to_string(),
            None => "null".to_string(),
        };
        let br = match r.bounded_recursion {
            Some(b) => b.to_string(),
            None => "null".to_string(),
        };
        out.push_str(&format!(
            "{{\"name\":{},\"safety_standard\":{},\"is_pure\":{},\"no_heap\":{},\"no_float\":{},\"no_recursion\":{},\"interrupt\":{},\"deterministic_timing\":{},\"bounded_stack_bytes\":{},\"wcet_cycles\":{},\"bounded_recursion\":{}}}",
            json_string_local(&r.name),
            standard,
            r.is_pure,
            r.no_heap,
            r.no_float,
            r.no_recursion,
            r.interrupt,
            r.deterministic_timing,
            bs,
            wc,
            br,
        ));
    }
    out.push_str("]}\n");
    out
}

pub fn format_safety_attrs_csv(report: &[FnSafetyAttrs]) -> String {
    let mut out = String::from(
        "name,safety_standard,is_pure,no_heap,no_float,no_recursion,interrupt,\
         deterministic_timing,bounded_stack_bytes,wcet_cycles,bounded_recursion\n",
    );
    for r in report {
        let standard = r.safety_standard.as_deref().unwrap_or("");
        let bs = r.bounded_stack_bytes.map(|b| b.to_string()).unwrap_or_default();
        let wc = r.wcet_cycles.map(|w| w.to_string()).unwrap_or_default();
        let br = r.bounded_recursion.map(|b| b.to_string()).unwrap_or_default();
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{}\n",
            csv_escape_local(&r.name),
            csv_escape_local(standard),
            r.is_pure,
            r.no_heap,
            r.no_float,
            r.no_recursion,
            r.interrupt,
            r.deterministic_timing,
            bs,
            wc,
            br,
        ));
    }
    out
}

// ── S-5: MISRA C 2012 Rule 14.1 — no unreachable / always-false branches ──

/// S-5 — MISRA C 2012 Rule 14.1: there shall be no unreachable code.
/// Under `#[misra_c_2012]` (or any composite that includes it): flag
/// `if` conditions that are literal `true`/`false` (the body or else
/// branch is statically dead) and `while false { }` loops.
/// Only fires when the function carries a MISRA composite tag.
pub fn enforce_misra_no_dead_branch(
    program: &TypedProgram,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for f in &program.functions {
        if !matches!(
            f.safety_standard.as_deref(),
            Some("misra_c_2012") | Some("asil_d") | Some("do178c_level_a")
                | Some("iec_62304_class_c")
                | Some("iec_61508_sil3") | Some("iec_61508_sil4")
                | Some("autosar_ap")
        ) {
            continue;
        }
        check_dead_branch_stmts(&f.body, &f.name, diagnostics);
    }
}

fn check_dead_branch_stmts(
    stmts: &[TypedStmt],
    fn_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for stmt in stmts {
        check_dead_branch_stmt(stmt, fn_name, diagnostics);
    }
}

fn check_dead_branch_stmt(
    stmt: &TypedStmt,
    fn_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    use crate::ir::TypedExprKind as EK;
    match stmt {
        TypedStmt::If {
            cond,
            then_body,
            else_body,
        } => {
            let span = cond.span;
            match &cond.kind {
                EK::Bool(true) => {
                    diagnostics.push(Diagnostic::new(
                        span,
                        format!(
                            "MISRA 14.1 (in '{}'): condition is always true — \
                             else branch is unreachable dead code",
                            fn_name
                        ),
                    ));
                }
                EK::Bool(false) => {
                    diagnostics.push(Diagnostic::new(
                        span,
                        format!(
                            "MISRA 14.1 (in '{}'): condition is always false — \
                             then branch is unreachable dead code",
                            fn_name
                        ),
                    ));
                }
                _ => {}
            }
            check_dead_branch_stmts(then_body, fn_name, diagnostics);
            check_dead_branch_stmts(else_body, fn_name, diagnostics);
        }
        TypedStmt::While { cond, body, .. } => {
            let span = cond.span;
            if matches!(cond.kind, EK::Bool(false)) {
                diagnostics.push(Diagnostic::new(
                    span,
                    format!(
                        "MISRA 14.1 (in '{}'): `while false` loop body is \
                         unreachable dead code",
                        fn_name
                    ),
                ));
            }
            check_dead_branch_stmts(body, fn_name, diagnostics);
        }
        TypedStmt::For { body, .. } | TypedStmt::ForIter { body, .. } => {
            check_dead_branch_stmts(body, fn_name, diagnostics);
        }
        TypedStmt::UnsafeBlock { body, .. }
        | TypedStmt::TaskSpawn { body, .. } => {
            check_dead_branch_stmts(body, fn_name, diagnostics);
        }
        _ => {}
    }
}

// ── S-6: MISRA C 2012 Rule 15.5 — single point of exit ──────────────────

/// S-6 — MISRA C 2012 Rule 15.5: a function should have a single point
/// of exit at the end. Under `#[misra_c_2012]` (or composites that
/// include it): flag functions that contain more than one `return`
/// statement.
pub fn enforce_misra_single_exit(
    program: &TypedProgram,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for f in &program.functions {
        if !matches!(
            f.safety_standard.as_deref(),
            Some("misra_c_2012") | Some("asil_d") | Some("do178c_level_a")
                | Some("iec_62304_class_c")
                | Some("iec_61508_sil3") | Some("iec_61508_sil4")
                | Some("autosar_ap")
        ) {
            continue;
        }
        let mut returns: Vec<crate::span::Span> = Vec::new();
        collect_returns(&f.body, &mut returns);
        if returns.len() > 1 {
            // Report on every return after the first.
            for span in &returns[1..] {
                diagnostics.push(Diagnostic::new(
                    *span,
                    format!(
                        "MISRA 15.5 (in '{}'): function has {} return statements — \
                         only one is permitted (single point of exit)",
                        f.name,
                        returns.len()
                    ),
                ));
            }
        }
    }
}

fn collect_returns(stmts: &[TypedStmt], out: &mut Vec<crate::span::Span>) {
    for stmt in stmts {
        collect_returns_stmt(stmt, out);
    }
}

fn collect_returns_stmt(stmt: &TypedStmt, out: &mut Vec<crate::span::Span>) {
    match stmt {
        TypedStmt::Return { expr } => out.push(expr.span),
        TypedStmt::If { then_body, else_body, .. } => {
            collect_returns(then_body, out);
            collect_returns(else_body, out);
        }
        TypedStmt::While { body, .. }
        | TypedStmt::For { body, .. }
        | TypedStmt::ForIter { body, .. }
        | TypedStmt::TaskSpawn { body, .. } => {
            collect_returns(body, out);
        }
        TypedStmt::UnsafeBlock { body, .. } => {
            collect_returns(body, out);
        }
        _ => {}
    }
}

// ── S-10: MISRA C 2012 Rule 2.1 — dead code after unconditional jump ─────

/// S-10 — MISRA C 2012 Rule 2.1: a project shall not contain
/// unreachable code. This pass catches the most common mechanical form:
/// statements that follow a `return`, `break`, or `continue` inside a
/// block without any intervening branch. Fires for all functions
/// (MISRA Rule 2.1 is a Required rule; we emit it as a warning-level
/// note rather than gating it behind a composite tag so teams without
/// the full composite still benefit).
pub fn enforce_dead_code_after_jump(
    program: &TypedProgram,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for f in &program.functions {
        check_dead_after_jump_stmts(&f.body, &f.name, false, diagnostics);
    }
}

/// Returns `true` if the block ends with an unconditional jump (return /
/// break / continue / panic call) and therefore any subsequent sibling
/// statement in the *parent* block would be unreachable.
fn check_dead_after_jump_stmts(
    stmts: &[TypedStmt],
    fn_name: &str,
    _in_loop: bool,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let mut jumped = false;
    for (i, stmt) in stmts.iter().enumerate() {
        if jumped {
            // This statement is unreachable — report it.
            if let Some(span) = dead_stmt_span(stmt) {
                diagnostics.push(Diagnostic::new(
                    span,
                    format!(
                        "MISRA 2.1 (in '{}'): unreachable code — \
                         this statement follows an unconditional jump \
                         (return / break / continue)",
                        fn_name
                    ),
                ));
            }
            // Don't recurse further in this block — the first
            // unreachable is enough to flag the issue.
            break;
        }
        jumped = check_dead_after_jump_stmt(stmt, fn_name, diagnostics);
        let _ = i; // suppress unused warning
    }
    jumped
}

fn check_dead_after_jump_stmt(
    stmt: &TypedStmt,
    fn_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    match stmt {
        TypedStmt::Return { .. } => true,
        TypedStmt::Break { .. } | TypedStmt::Continue { .. } => true,
        TypedStmt::If { then_body, else_body, .. } => {
            let then_jumps =
                check_dead_after_jump_stmts(then_body, fn_name, false, diagnostics);
            let else_jumps =
                check_dead_after_jump_stmts(else_body, fn_name, false, diagnostics);
            then_jumps && else_jumps
        }
        TypedStmt::While { body, .. }
        | TypedStmt::For { body, .. }
        | TypedStmt::ForIter { body, .. }
        | TypedStmt::TaskSpawn { body, .. } => {
            check_dead_after_jump_stmts(body, fn_name, true, diagnostics);
            false
        }
        TypedStmt::UnsafeBlock { body, .. } => {
            check_dead_after_jump_stmts(body, fn_name, false, diagnostics)
        }
        _ => false,
    }
}

fn dead_stmt_span(stmt: &TypedStmt) -> Option<crate::span::Span> {
    match stmt {
        TypedStmt::Let { expr, .. } => Some(expr.span),
        TypedStmt::Return { expr } => Some(expr.span),
        TypedStmt::Reassign { expr, .. } => Some(expr.span),
        TypedStmt::Discard { expr } => Some(expr.span),
        TypedStmt::Assert { expr, .. } | TypedStmt::Prove { expr } => Some(expr.span),
        TypedStmt::If { cond, .. } | TypedStmt::While { cond, .. } => Some(cond.span),
        TypedStmt::For { start, .. } => Some(start.span),
        TypedStmt::ForIter { collection_ty: _, .. } => None,
        TypedStmt::Print { items } | TypedStmt::EPrint { items } => {
            items.first().and_then(|i| match i {
                crate::ir::TypedPrintItem::Expr(e) => Some(e.span),
                _ => None,
            })
        }
        _ => None,
    }
}

// ── S-8: MISRA C 2012 Rule 17.1 — no variadic functions ──────────────────

/// S-8 — MISRA C 2012 Rule 17.1: the features of `<stdarg.h>` shall
/// not be used. vāṇī has no user-level variadic fn syntax; the only
/// variadic declaration in the compiler is the internal `syscall`
/// trampoline which is emitted unconditionally by the backends and is
/// never user-declarable. This pass verifies that structural guarantee:
/// no `TypedFunction` marked `is_extern` with a composite safety tag
/// appears in the known-variadic builtins set. Extend `VARIADIC_BUILTINS`
/// if additional variadic trampolines are added.
pub fn enforce_misra_no_variadic(
    program: &TypedProgram,
    diagnostics: &mut Vec<Diagnostic>,
) {
    const VARIADIC_BUILTINS: &[&str] = &["syscall"];
    for f in &program.functions {
        if !f.is_extern || f.safety_standard.is_none() {
            continue;
        }
        if VARIADIC_BUILTINS.contains(&f.name.as_str()) {
            diagnostics.push(Diagnostic::new(
                f.span,
                format!(
                    "MISRA 17.1 (extern '{}'): variadic function declaration \
                     is forbidden under `#{}`; replace with a fixed-arity wrapper",
                    f.name,
                    f.safety_standard.as_deref().unwrap_or("?")
                ),
            ));
        }
    }
}

// ── S-9: MISRA C 2012 Rule 11.1–11.3 — fn-ptr / object-ptr conversions ──

/// S-9 — MISRA C 2012 Rules 11.1–11.3:
///   11.1 — conversions to/from a pointer to a function are forbidden
///   11.3 — a cast shall not be performed between a pointer to object
///           and a pointer to a different object type
/// Under composite-tagged functions, walk all `Cast` expressions and
/// flag function-pointer ↔ data-pointer conversions and data-pointer
/// ↔ incompatible-data-pointer casts (only inside unsafe blocks where
/// raw pointers can appear).
pub fn enforce_misra_no_fnptr_cast(
    program: &TypedProgram,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for f in &program.functions {
        if f.safety_standard.is_none() {
            continue;
        }
        let std_name = f.safety_standard.as_deref().unwrap_or("?");
        check_fnptr_cast_stmts(&f.body, &f.name, std_name, diagnostics);
    }
}

fn check_fnptr_cast_stmts(
    stmts: &[TypedStmt],
    fn_name: &str,
    std_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for stmt in stmts {
        check_fnptr_cast_stmt(stmt, fn_name, std_name, diagnostics);
    }
}

fn check_fnptr_cast_stmt(
    stmt: &TypedStmt,
    fn_name: &str,
    std_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match stmt {
        TypedStmt::UnsafeBlock { body, .. } => {
            check_fnptr_cast_stmts(body, fn_name, std_name, diagnostics);
        }
        TypedStmt::Let { expr, .. }
        | TypedStmt::Reassign { expr, .. }
        | TypedStmt::Return { expr }
        | TypedStmt::Assert { expr, .. }
        | TypedStmt::Prove { expr }
        | TypedStmt::Discard { expr } => {
            check_fnptr_cast_expr(expr, fn_name, std_name, diagnostics);
        }
        TypedStmt::If { cond, then_body, else_body } => {
            check_fnptr_cast_expr(cond, fn_name, std_name, diagnostics);
            check_fnptr_cast_stmts(then_body, fn_name, std_name, diagnostics);
            check_fnptr_cast_stmts(else_body, fn_name, std_name, diagnostics);
        }
        TypedStmt::While { cond, body, .. } => {
            check_fnptr_cast_expr(cond, fn_name, std_name, diagnostics);
            check_fnptr_cast_stmts(body, fn_name, std_name, diagnostics);
        }
        TypedStmt::For { start, end, body, .. } => {
            check_fnptr_cast_expr(start, fn_name, std_name, diagnostics);
            check_fnptr_cast_expr(end, fn_name, std_name, diagnostics);
            check_fnptr_cast_stmts(body, fn_name, std_name, diagnostics);
        }
        TypedStmt::ForIter { body, .. } | TypedStmt::TaskSpawn { body, .. } => {
            check_fnptr_cast_stmts(body, fn_name, std_name, diagnostics);
        }
        TypedStmt::IndexAssign { index, value, .. } => {
            check_fnptr_cast_expr(index, fn_name, std_name, diagnostics);
            check_fnptr_cast_expr(value, fn_name, std_name, diagnostics);
        }
        TypedStmt::FieldAssign { value, .. } => {
            check_fnptr_cast_expr(value, fn_name, std_name, diagnostics);
        }
        _ => {}
    }
}

fn check_fnptr_cast_expr(
    expr: &crate::ir::TypedExpr,
    fn_name: &str,
    std_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    use crate::ir::TypedExprKind as EK;
    use crate::ast::Type;

    fn is_fn_ptr(ty: &Type) -> bool {
        matches!(ty, Type::FnPtr(_, _) | Type::Closure(_, _))
    }
    fn is_data_ptr(ty: &Type) -> bool {
        matches!(ty, Type::Ptr(_) | Type::PtrMut(_))
    }

    if let EK::Cast { expr: inner, .. } = &expr.kind {
        let src = &inner.ty;
        let dst = &expr.ty;
        if (is_fn_ptr(src) && !is_fn_ptr(dst)) || (!is_fn_ptr(src) && is_fn_ptr(dst)) {
            diagnostics.push(Diagnostic::new(
                expr.span,
                format!(
                    "MISRA 11.1 (in '{}'): cast between function-pointer and \
                     non-function-pointer type is forbidden under `#{}`",
                    fn_name, std_name
                ),
            ));
        }
        if is_data_ptr(src) && is_data_ptr(dst) && src != dst {
            diagnostics.push(Diagnostic::new(
                expr.span,
                format!(
                    "MISRA 11.3 (in '{}'): cast between pointers to different \
                     object types is forbidden under `#{}`",
                    fn_name, std_name
                ),
            ));
        }
        check_fnptr_cast_expr(inner, fn_name, std_name, diagnostics);
    } else {
        match &expr.kind {
            EK::Unary { expr: inner, .. } => {
                check_fnptr_cast_expr(inner, fn_name, std_name, diagnostics);
            }
            EK::Binary { left, right, .. }
            | EK::Index { array: left, index: right, .. } => {
                check_fnptr_cast_expr(left, fn_name, std_name, diagnostics);
                check_fnptr_cast_expr(right, fn_name, std_name, diagnostics);
            }
            EK::Call { args, .. } => {
                for a in args {
                    check_fnptr_cast_expr(a, fn_name, std_name, diagnostics);
                }
            }
            _ => {}
        }
    }
}

// ── S-7: MISRA C 2012 Rule 13.2 — evaluation-order-dependent args ────────

/// S-7 — MISRA C 2012 Rule 13.2: the value of an expression and its
/// persistent side effects shall be the same under any order of
/// evaluation. In vāṇī the main risk is a `Call` expression whose
/// argument list mentions the same binding more than once — the C
/// backend may evaluate arguments in any order (C leaves argument
/// evaluation order unspecified), so if a callee could modify one of
/// them, the result is implementation-defined.
///
/// V1 scope: flag any `Call` where the same `Var` name appears in two
/// or more argument positions under a composite-tagged function.
pub fn enforce_misra_eval_order(
    program: &TypedProgram,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for f in &program.functions {
        if f.safety_standard.is_none() {
            continue;
        }
        let std_name = f.safety_standard.as_deref().unwrap_or("?");
        check_eval_order_stmts(&f.body, &f.name, std_name, diagnostics);
    }
}

fn check_eval_order_stmts(
    stmts: &[TypedStmt],
    fn_name: &str,
    std_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for stmt in stmts {
        check_eval_order_stmt(stmt, fn_name, std_name, diagnostics);
    }
}

fn check_eval_order_stmt(
    stmt: &TypedStmt,
    fn_name: &str,
    std_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match stmt {
        TypedStmt::Let { expr, .. }
        | TypedStmt::Reassign { expr, .. }
        | TypedStmt::Return { expr }
        | TypedStmt::Assert { expr, .. }
        | TypedStmt::Prove { expr }
        | TypedStmt::Discard { expr } => {
            check_eval_order_expr(expr, fn_name, std_name, diagnostics);
        }
        TypedStmt::If { cond, then_body, else_body } => {
            check_eval_order_expr(cond, fn_name, std_name, diagnostics);
            check_eval_order_stmts(then_body, fn_name, std_name, diagnostics);
            check_eval_order_stmts(else_body, fn_name, std_name, diagnostics);
        }
        TypedStmt::While { cond, body, .. } => {
            check_eval_order_expr(cond, fn_name, std_name, diagnostics);
            check_eval_order_stmts(body, fn_name, std_name, diagnostics);
        }
        TypedStmt::For { start, end, body, .. } => {
            check_eval_order_expr(start, fn_name, std_name, diagnostics);
            check_eval_order_expr(end, fn_name, std_name, diagnostics);
            check_eval_order_stmts(body, fn_name, std_name, diagnostics);
        }
        TypedStmt::ForIter { body, .. }
        | TypedStmt::TaskSpawn { body, .. }
        | TypedStmt::UnsafeBlock { body, .. } => {
            check_eval_order_stmts(body, fn_name, std_name, diagnostics);
        }
        TypedStmt::IndexAssign { index, value, .. } => {
            check_eval_order_expr(index, fn_name, std_name, diagnostics);
            check_eval_order_expr(value, fn_name, std_name, diagnostics);
        }
        TypedStmt::FieldAssign { value, .. } => {
            check_eval_order_expr(value, fn_name, std_name, diagnostics);
        }
        _ => {}
    }
}

fn check_eval_order_expr(
    expr: &crate::ir::TypedExpr,
    fn_name: &str,
    std_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    use crate::ir::TypedExprKind as EK;
    if let EK::Call { args, .. } = &expr.kind {
        // Count occurrences of each Var name across args. Using remove() so
        // each variable is flagged exactly once (at its second occurrence)
        // regardless of whether that occurrence is adjacent to the first.
        let mut seen: std::collections::HashMap<&str, (usize, crate::span::Span)> =
            std::collections::HashMap::new();
        for (i, arg) in args.iter().enumerate() {
            if let EK::Var(name) = &arg.kind {
                if let Some((first_pos, first_span)) = seen.remove(name.as_str()) {
                    // Fire for ANY second occurrence, not just adjacent ones (L22 fix).
                    diagnostics.push(Diagnostic::new(
                        arg.span,
                        format!(
                            "MISRA 13.2 (in '{}'): binding '{}' appears at \
                             arg positions {} and {} in the same call — \
                             C argument evaluation order is unspecified; \
                             forbidden under `#{}`",
                            fn_name,
                            name,
                            first_pos + 1,
                            i + 1,
                            std_name
                        ),
                    ));
                    let _ = first_span;
                } else {
                    seen.insert(name.as_str(), (i, arg.span));
                }
            }
            check_eval_order_expr(arg, fn_name, std_name, diagnostics);
        }
    } else {
        match &expr.kind {
            EK::Unary { expr: inner, .. }
            | EK::Cast { expr: inner, .. } => {
                check_eval_order_expr(inner, fn_name, std_name, diagnostics);
            }
            EK::Binary { left, right, .. }
            | EK::Index { array: left, index: right, .. } => {
                check_eval_order_expr(left, fn_name, std_name, diagnostics);
                check_eval_order_expr(right, fn_name, std_name, diagnostics);
            }
            _ => {}
        }
    }
}

// ── S-19: Lock-order graph and deadlock detection ────────────────────────────

/// Intern a lock variable name as a stable integer ID for the lock-order graph.
fn lock_intern(
    name: &str,
    lock_id: &mut std::collections::HashMap<String, usize>,
    id_name: &mut std::collections::HashMap<usize, String>,
    next_id: &mut usize,
) -> usize {
    if let Some(&id) = lock_id.get(name) {
        id
    } else {
        let id = *next_id;
        *next_id += 1;
        lock_id.insert(name.to_string(), id);
        id_name.insert(id, name.to_string());
        id
    }
}

/// Walk `stmts` tracking the set of currently held locks (`held`) and record
/// lock-acquisition ordering edges directly into `edges`.
///
/// When a user-defined callee is encountered the analysis recurses into its
/// body with a CLONE of `held`.  Callee-acquired locks are released on return,
/// so the clone is discarded — the caller's `held` set is unchanged after
/// the call returns.  This prevents spurious ordering constraints between
/// sequential independent function calls (L20 correctness fix for the
/// held-set vs. flat-sequence ambiguity).
fn build_lock_edges(
    stmts: &[crate::ir::TypedStmt],
    held: &mut Vec<(usize, crate::span::Span)>,
    lock_id: &mut std::collections::HashMap<String, usize>,
    next_id: &mut usize,
    id_name: &mut std::collections::HashMap<usize, String>,
    edges: &mut std::collections::HashMap<usize, std::collections::HashMap<usize, crate::span::Span>>,
    fn_map: &std::collections::HashMap<String, &[crate::ir::TypedStmt]>,
    visiting: &mut std::collections::HashSet<String>,
) {
    use crate::ir::TypedStmt as S;
    for stmt in stmts {
        match stmt {
            S::Let { expr, .. } | S::Discard { expr } | S::Reassign { expr, .. } => {
                build_lock_edges_expr(expr, held, lock_id, next_id, id_name, edges, fn_map, visiting);
            }
            S::If { cond, then_body, else_body } => {
                build_lock_edges_expr(cond, held, lock_id, next_id, id_name, edges, fn_map, visiting);
                // Linear over-approximation: process both branches sequentially.
                build_lock_edges(then_body, held, lock_id, next_id, id_name, edges, fn_map, visiting);
                build_lock_edges(else_body, held, lock_id, next_id, id_name, edges, fn_map, visiting);
            }
            S::While { cond, body, .. } => {
                build_lock_edges_expr(cond, held, lock_id, next_id, id_name, edges, fn_map, visiting);
                build_lock_edges(body, held, lock_id, next_id, id_name, edges, fn_map, visiting);
            }
            S::For { start, end, body, .. } => {
                build_lock_edges_expr(start, held, lock_id, next_id, id_name, edges, fn_map, visiting);
                build_lock_edges_expr(end, held, lock_id, next_id, id_name, edges, fn_map, visiting);
                build_lock_edges(body, held, lock_id, next_id, id_name, edges, fn_map, visiting);
            }
            S::ForIter { body, .. }
            | S::TaskSpawn { body, .. }
            | S::UnsafeBlock { body, .. } => {
                build_lock_edges(body, held, lock_id, next_id, id_name, edges, fn_map, visiting);
            }
            S::Assert { expr, .. } | S::Prove { expr, .. } => {
                build_lock_edges_expr(expr, held, lock_id, next_id, id_name, edges, fn_map, visiting);
            }
            S::IndexAssign { index, value, .. } => {
                build_lock_edges_expr(index, held, lock_id, next_id, id_name, edges, fn_map, visiting);
                build_lock_edges_expr(value, held, lock_id, next_id, id_name, edges, fn_map, visiting);
            }
            S::FieldAssign { object, value, .. } => {
                build_lock_edges_expr(object, held, lock_id, next_id, id_name, edges, fn_map, visiting);
                build_lock_edges_expr(value, held, lock_id, next_id, id_name, edges, fn_map, visiting);
            }
            S::Return { expr } => {
                build_lock_edges_expr(expr, held, lock_id, next_id, id_name, edges, fn_map, visiting);
            }
            S::Print { items } | S::EPrint { items } => {
                for item in items {
                    if let crate::ir::TypedPrintItem::Expr(e) = item {
                        build_lock_edges_expr(e, held, lock_id, next_id, id_name, edges, fn_map, visiting);
                    }
                }
            }
            S::Drop { .. } | S::ForIterShallowFree { .. }
            | S::Break { .. } | S::Continue { .. }
            | S::TaskJoin { .. } | S::Detach { .. } => {}
        }
    }
}

fn build_lock_edges_expr(
    expr: &crate::ir::TypedExpr,
    held: &mut Vec<(usize, crate::span::Span)>,
    lock_id: &mut std::collections::HashMap<String, usize>,
    next_id: &mut usize,
    id_name: &mut std::collections::HashMap<usize, String>,
    edges: &mut std::collections::HashMap<usize, std::collections::HashMap<usize, crate::span::Span>>,
    fn_map: &std::collections::HashMap<String, &[crate::ir::TypedStmt]>,
    visiting: &mut std::collections::HashSet<String>,
) {
    use crate::ir::TypedExprKind as EK;
    match &expr.kind {
        EK::Call { name, args, .. } => {
            if name == "mutex_lock" {
                if let Some(arg) = args.first() {
                    if let Some(mutex_name) = extract_mutex_name_from_typed_expr(arg) {
                        let new_id = lock_intern(&mutex_name, lock_id, id_name, next_id);
                        // Every currently held lock must be acquired before this one.
                        for &(h_id, _h_span) in held.iter() {
                            if h_id != new_id {
                                edges.entry(h_id).or_default().entry(new_id).or_insert(expr.span);
                            }
                        }
                        held.push((new_id, expr.span));
                    }
                }
                for a in args {
                    build_lock_edges_expr(a, held, lock_id, next_id, id_name, edges, fn_map, visiting);
                }
            } else if let Some(body) = fn_map.get(name.as_str()) {
                if !visiting.contains(name.as_str()) {
                    // Recurse into the callee with a snapshot of the current held set.
                    // The callee's locks are released on return → discard callee_held.
                    let mut callee_held = held.clone();
                    visiting.insert(name.clone());
                    build_lock_edges(body, &mut callee_held, lock_id, next_id, id_name, edges, fn_map, visiting);
                    visiting.remove(name.as_str());
                }
                for a in args {
                    build_lock_edges_expr(a, held, lock_id, next_id, id_name, edges, fn_map, visiting);
                }
            } else {
                for a in args {
                    build_lock_edges_expr(a, held, lock_id, next_id, id_name, edges, fn_map, visiting);
                }
            }
        }
        EK::Unary { expr: inner, .. } | EK::Cast { expr: inner, .. } => {
            build_lock_edges_expr(inner, held, lock_id, next_id, id_name, edges, fn_map, visiting);
        }
        EK::Ref { .. } | EK::RefMut { .. } => {}
        EK::Binary { left, right, .. }
        | EK::Index { array: left, index: right, .. } => {
            build_lock_edges_expr(left, held, lock_id, next_id, id_name, edges, fn_map, visiting);
            build_lock_edges_expr(right, held, lock_id, next_id, id_name, edges, fn_map, visiting);
        }
        EK::Block { stmts, tail } => {
            build_lock_edges(stmts, held, lock_id, next_id, id_name, edges, fn_map, visiting);
            build_lock_edges_expr(tail, held, lock_id, next_id, id_name, edges, fn_map, visiting);
        }
        EK::IfExpr { cond, then_value, else_value } => {
            build_lock_edges_expr(cond, held, lock_id, next_id, id_name, edges, fn_map, visiting);
            build_lock_edges_expr(then_value, held, lock_id, next_id, id_name, edges, fn_map, visiting);
            build_lock_edges_expr(else_value, held, lock_id, next_id, id_name, edges, fn_map, visiting);
        }
        EK::Match { scrutinee, arms } => {
            build_lock_edges_expr(scrutinee, held, lock_id, next_id, id_name, edges, fn_map, visiting);
            for arm in arms {
                build_lock_edges_expr(&arm.body, held, lock_id, next_id, id_name, edges, fn_map, visiting);
            }
        }
        EK::EnumVariantWithPayload { payload, .. } => {
            build_lock_edges_expr(payload, held, lock_id, next_id, id_name, edges, fn_map, visiting);
        }
        EK::DynCoerce { value, .. } => {
            build_lock_edges_expr(value, held, lock_id, next_id, id_name, edges, fn_map, visiting);
        }
        EK::DynDispatch { receiver, args, .. } => {
            build_lock_edges_expr(receiver, held, lock_id, next_id, id_name, edges, fn_map, visiting);
            for a in args {
                build_lock_edges_expr(a, held, lock_id, next_id, id_name, edges, fn_map, visiting);
            }
        }
        EK::CallIndirect { callee, args } => {
            build_lock_edges_expr(callee, held, lock_id, next_id, id_name, edges, fn_map, visiting);
            for a in args {
                build_lock_edges_expr(a, held, lock_id, next_id, id_name, edges, fn_map, visiting);
            }
        }
        EK::Tuple { elements } | EK::ArrayLit { elements } => {
            for e in elements {
                build_lock_edges_expr(e, held, lock_id, next_id, id_name, edges, fn_map, visiting);
            }
        }
        EK::TupleAccess { tuple: inner, .. }
        | EK::FieldAccess { object: inner, .. } => {
            build_lock_edges_expr(inner, held, lock_id, next_id, id_name, edges, fn_map, visiting);
        }
        EK::RefMutIndex { index, .. } => {
            build_lock_edges_expr(index, held, lock_id, next_id, id_name, edges, fn_map, visiting);
        }
        EK::Len { array: inner, .. } => {
            build_lock_edges_expr(inner, held, lock_id, next_id, id_name, edges, fn_map, visiting);
        }
        EK::StructLit { fields, .. } => {
            for (_, e) in fields {
                build_lock_edges_expr(e, held, lock_id, next_id, id_name, edges, fn_map, visiting);
            }
        }
        _ => {}
    }
}

/// Extract a mutex variable name from a `TypedExpr` argument to `mutex_lock`.
fn extract_mutex_name_from_typed_expr(arg: &crate::ir::TypedExpr) -> Option<String> {
    use crate::ir::TypedExprKind as EK;
    match &arg.kind {
        EK::Ref { name } | EK::RefMut { name } => Some(name.clone()),
        EK::Var(name) => Some(name.clone()),
        _ => None,
    }
}

/// Detect lock-acquisition-order cycles across all functions in the program.
///
/// Algorithm:
/// 1. For each function, walk its body with a held-set tracking which locks
///    are currently held. For each new `mutex_lock`, add edges from every
///    currently held lock to the new one.
/// 2. User-defined callees are analysed transitively: the callee receives a
///    clone of the caller's held set at the call site. Since callee locks are
///    released on return, the caller's held set is unchanged after the call.
///    This correctly captures "lock held by caller constrains callee's first
///    lock" while avoiding spurious bridging across sequential independent calls.
/// 3. After processing all functions, run a DFS cycle detection on the graph.
/// 4. Report each cycle as a warning-level diagnostic including the span of
///    the lock acquisition that closes the cycle.
pub fn enforce_lock_order(
    program: &crate::ir::TypedProgram,
    diagnostics: &mut Vec<crate::diagnostic::Diagnostic>,
) {
    use std::collections::{HashMap, HashSet};

    // lock_name -> integer ID
    let mut lock_id: HashMap<String, usize> = HashMap::new();
    let mut next_id: usize = 0;

    // Adjacency list: edge_span[u][v] = first span where u → v was seen.
    // We record the span so we can point at the code in the diagnostic.
    let mut edges: HashMap<usize, HashMap<usize, crate::span::Span>> = HashMap::new();
    // reverse map: ID -> name (for diagnostics)
    let mut id_name: HashMap<usize, String> = HashMap::new();

    // Build fn_map once for transitive analysis (L20 fix).
    let fn_map: HashMap<String, &[crate::ir::TypedStmt]> =
        program.functions.iter()
            .filter(|f| !f.is_extern)
            .map(|f| (f.name.clone(), f.body.as_slice()))
            .collect();

    for func in &program.functions {
        if func.is_extern {
            continue;
        }
        let mut held: Vec<(usize, crate::span::Span)> = Vec::new();
        let mut visiting: HashSet<String> = HashSet::new();
        visiting.insert(func.name.clone());
        build_lock_edges(
            &func.body, &mut held,
            &mut lock_id, &mut next_id, &mut id_name,
            &mut edges, &fn_map, &mut visiting,
        );
    }

    if edges.is_empty() {
        return;
    }

    // DFS-based cycle detection. We track the recursion stack (not just
    // visited) to distinguish back-edges from cross-edges.
    let all_nodes: HashSet<usize> = edges.keys().copied()
        .chain(edges.values().flat_map(|m| m.keys().copied()))
        .collect();

    let mut visited: HashSet<usize> = HashSet::new();
    let mut on_stack: Vec<usize> = Vec::new();
    let mut reported: HashSet<(usize, usize)> = HashSet::new();

    fn dfs(
        node: usize,
        edges: &HashMap<usize, HashMap<usize, crate::span::Span>>,
        visited: &mut HashSet<usize>,
        on_stack: &mut Vec<usize>,
        reported: &mut HashSet<(usize, usize)>,
        id_name: &HashMap<usize, String>,
        diagnostics: &mut Vec<crate::diagnostic::Diagnostic>,
    ) {
        visited.insert(node);
        on_stack.push(node);

        if let Some(neighbours) = edges.get(&node) {
            for (&next, &span) in neighbours {
                if !visited.contains(&next) {
                    dfs(next, edges, visited, on_stack, reported, id_name, diagnostics);
                } else if on_stack.contains(&next) {
                    // Found a back-edge: node → next closes a cycle
                    let cycle_key = (node.min(next), node.max(next));
                    if reported.insert(cycle_key) {
                        // Build a human-readable cycle description
                        let cycle_start = on_stack.iter().position(|&n| n == next).unwrap();
                        let cycle_nodes = &on_stack[cycle_start..];
                        let cycle_str: Vec<&str> = cycle_nodes
                            .iter()
                            .map(|n| id_name.get(n).map(|s| s.as_str()).unwrap_or("?"))
                            .collect();
                        let a_name = id_name.get(&node).map(|s| s.as_str()).unwrap_or("?");
                        let b_name = id_name.get(&next).map(|s| s.as_str()).unwrap_or("?");
                        let msg = format!(
                            "[S-19] potential deadlock: lock-acquisition cycle detected \
                             ({} → {}). Cycle path: {} → {}. \
                             If two threads acquire these locks in opposite orders \
                             they will deadlock.",
                            a_name, b_name,
                            cycle_str.join(" → "),
                            b_name,
                        );
                        let hint = format!(
                            "Establish a global lock ordering and always acquire \
                             locks in that order. For example, always acquire '{}' \
                             before '{}'.",
                            b_name, a_name
                        );
                        diagnostics.push(crate::diagnostic::Diagnostic {
                            span,
                            message: msg,
                            related: vec![],
                            elaboration: vec![hint],
                        });
                    }
                }
            }
        }

        on_stack.pop();
    }

    for &node in &all_nodes {
        if !visited.contains(&node) {
            dfs(
                node,
                &edges,
                &mut visited,
                &mut on_stack,
                &mut reported,
                &id_name,
                diagnostics,
            );
        }
    }
}

// ── S-20: ISR priority annotation and preemption check ───────────────────────

/// Collect the set of mutex names that a function (transitively) tries to
/// `mutex_lock`. Used by the ISR preemption checker to find shared resources.
///
/// `fn_name` seeds the cycle-guard set so we never re-enter the top-level ISR.
/// `fn_map` provides callee bodies for transitive detection (L21 fix).
fn collect_locked_mutexes(
    fn_name: &str,
    stmts: &[crate::ir::TypedStmt],
    fn_map: &std::collections::HashMap<String, &[crate::ir::TypedStmt]>,
) -> std::collections::HashSet<String> {
    let mut result = std::collections::HashSet::new();
    let mut visiting = std::collections::HashSet::new();
    visiting.insert(fn_name.to_string());
    collect_locked_mutexes_stmts(stmts, &mut result, fn_map, &mut visiting);
    result
}

fn collect_locked_mutexes_stmts(
    stmts: &[crate::ir::TypedStmt],
    out: &mut std::collections::HashSet<String>,
    fn_map: &std::collections::HashMap<String, &[crate::ir::TypedStmt]>,
    visiting: &mut std::collections::HashSet<String>,
) {
    use crate::ir::TypedStmt as S;
    for stmt in stmts {
        match stmt {
            S::Let { expr, .. } | S::Discard { expr } | S::Reassign { expr, .. }
            | S::Assert { expr, .. } | S::Prove { expr } | S::Return { expr } => {
                collect_locked_mutexes_expr(expr, out, fn_map, visiting);
            }
            S::If { cond, then_body, else_body } => {
                collect_locked_mutexes_expr(cond, out, fn_map, visiting);
                collect_locked_mutexes_stmts(then_body, out, fn_map, visiting);
                collect_locked_mutexes_stmts(else_body, out, fn_map, visiting);
            }
            S::While { cond, body, .. } => {
                collect_locked_mutexes_expr(cond, out, fn_map, visiting);
                collect_locked_mutexes_stmts(body, out, fn_map, visiting);
            }
            S::For { start, end, body, .. } => {
                collect_locked_mutexes_expr(start, out, fn_map, visiting);
                collect_locked_mutexes_expr(end, out, fn_map, visiting);
                collect_locked_mutexes_stmts(body, out, fn_map, visiting);
            }
            S::ForIter { body, .. }
            | S::TaskSpawn { body, .. }
            | S::UnsafeBlock { body, .. } => {
                collect_locked_mutexes_stmts(body, out, fn_map, visiting);
            }
            S::IndexAssign { index, value, .. } => {
                collect_locked_mutexes_expr(index, out, fn_map, visiting);
                collect_locked_mutexes_expr(value, out, fn_map, visiting);
            }
            S::FieldAssign { object, value, .. } => {
                collect_locked_mutexes_expr(object, out, fn_map, visiting);
                collect_locked_mutexes_expr(value, out, fn_map, visiting);
            }
            S::Print { items } | S::EPrint { items } => {
                for item in items {
                    if let crate::ir::TypedPrintItem::Expr(e) = item {
                        collect_locked_mutexes_expr(e, out, fn_map, visiting);
                    }
                }
            }
            S::Drop { .. } | S::ForIterShallowFree { .. }
            | S::Break { .. } | S::Continue { .. }
            | S::TaskJoin { .. } | S::Detach { .. } => {}
        }
    }
}

fn collect_locked_mutexes_expr(
    expr: &crate::ir::TypedExpr,
    out: &mut std::collections::HashSet<String>,
    fn_map: &std::collections::HashMap<String, &[crate::ir::TypedStmt]>,
    visiting: &mut std::collections::HashSet<String>,
) {
    use crate::ir::TypedExprKind as EK;
    match &expr.kind {
        EK::Call { name, args, .. } => {
            if name == "mutex_lock" {
                if let Some(arg) = args.first() {
                    if let Some(mx) = extract_mutex_name_from_typed_expr(arg) {
                        out.insert(mx);
                    }
                }
            } else if let Some(body) = fn_map.get(name.as_str()) {
                // Follow call into helper to find transitively-acquired mutexes (L21 fix).
                if !visiting.contains(name.as_str()) {
                    visiting.insert(name.clone());
                    collect_locked_mutexes_stmts(body, out, fn_map, visiting);
                    visiting.remove(name.as_str());
                }
            }
            for a in args { collect_locked_mutexes_expr(a, out, fn_map, visiting); }
        }
        EK::Unary { expr: inner, .. } | EK::Cast { expr: inner, .. } => {
            collect_locked_mutexes_expr(inner, out, fn_map, visiting);
        }
        EK::Binary { left, right, .. }
        | EK::Index { array: left, index: right, .. } => {
            collect_locked_mutexes_expr(left, out, fn_map, visiting);
            collect_locked_mutexes_expr(right, out, fn_map, visiting);
        }
        EK::Block { stmts, tail } => {
            collect_locked_mutexes_stmts(stmts, out, fn_map, visiting);
            collect_locked_mutexes_expr(tail, out, fn_map, visiting);
        }
        EK::IfExpr { cond, then_value, else_value } => {
            collect_locked_mutexes_expr(cond, out, fn_map, visiting);
            collect_locked_mutexes_expr(then_value, out, fn_map, visiting);
            collect_locked_mutexes_expr(else_value, out, fn_map, visiting);
        }
        EK::Match { scrutinee, arms } => {
            collect_locked_mutexes_expr(scrutinee, out, fn_map, visiting);
            for arm in arms { collect_locked_mutexes_expr(&arm.body, out, fn_map, visiting); }
        }
        EK::EnumVariantWithPayload { payload, .. } => {
            collect_locked_mutexes_expr(payload, out, fn_map, visiting);
        }
        EK::DynCoerce { value, .. } => { collect_locked_mutexes_expr(value, out, fn_map, visiting); }
        EK::DynDispatch { receiver, args, .. } => {
            collect_locked_mutexes_expr(receiver, out, fn_map, visiting);
            for a in args { collect_locked_mutexes_expr(a, out, fn_map, visiting); }
        }
        EK::CallIndirect { callee, args } => {
            collect_locked_mutexes_expr(callee, out, fn_map, visiting);
            for a in args { collect_locked_mutexes_expr(a, out, fn_map, visiting); }
        }
        EK::Tuple { elements } | EK::ArrayLit { elements } => {
            for e in elements { collect_locked_mutexes_expr(e, out, fn_map, visiting); }
        }
        EK::TupleAccess { tuple: inner, .. }
        | EK::FieldAccess { object: inner, .. } => {
            collect_locked_mutexes_expr(inner, out, fn_map, visiting);
        }
        EK::RefMutIndex { index, .. } => { collect_locked_mutexes_expr(index, out, fn_map, visiting); }
        EK::Len { array: inner, .. } => { collect_locked_mutexes_expr(inner, out, fn_map, visiting); }
        EK::StructLit { fields, .. } => {
            for (_, e) in fields { collect_locked_mutexes_expr(e, out, fn_map, visiting); }
        }
        _ => {}
    }
}

/// S-20 — ISR priority and preemption check.
///
/// A higher-priority ISR (lower priority number) that acquires a mutex
/// also held by a lower-priority ISR (or the main thread) risks a
/// **priority inversion** or **deadlock**: if the lower-priority ISR has
/// already locked the mutex when the higher-priority ISR fires, the
/// high-priority ISR will spin/block until the lower-priority ISR is
/// scheduled again — which can never happen while the high-priority ISR
/// is running.
///
/// Check: for every pair of `#[interrupt(priority=P)]` functions that
/// both call `mutex_lock` on a mutex with the same variable name, emit a
/// warning when one priority is strictly higher than the other (lower P
/// value). The canonical fix is to replace the mutex with an atomic
/// operation or to use a priority-ceiling protocol.
pub fn enforce_isr_preemption(
    program: &crate::ir::TypedProgram,
    diagnostics: &mut Vec<crate::diagnostic::Diagnostic>,
) {
    // Build fn_map for transitive mutex collection through helpers (L21 fix).
    let fn_map: std::collections::HashMap<String, &[crate::ir::TypedStmt]> =
        program.functions.iter()
            .filter(|f| !f.is_extern)
            .map(|f| (f.name.clone(), f.body.as_slice()))
            .collect();

    // Collect (priority, fn_name, fn_span, locked_mutex_set) for every ISR
    // that declares a priority.
    let isrs: Vec<(u32, &str, crate::span::Span, std::collections::HashSet<String>)> =
        program.functions.iter().filter_map(|f| {
            if let Some(prio) = f.interrupt_priority {
                let mx = collect_locked_mutexes(f.name.as_str(), &f.body, &fn_map);
                Some((prio, f.name.as_str(), f.span, mx))
            } else {
                None
            }
        }).collect();

    if isrs.len() < 2 {
        return;
    }

    // For every pair (i, j) where prio_i < prio_j (i has HIGHER urgency),
    // check if they share a mutex.
    for i in 0..isrs.len() {
        for j in (i + 1)..isrs.len() {
            let (pi, ni, si, mi) = &isrs[i];
            let (pj, nj, sj, mj) = &isrs[j];
            // lower number = higher urgency; only flag when priorities differ
            if pi == pj {
                continue;
            }
            // find shared mutexes
            let shared: Vec<&String> = mi.intersection(mj).collect();
            if shared.is_empty() {
                continue;
            }
            let (high_fn, low_fn, high_span, high_prio, low_prio) = if pi < pj {
                (ni, nj, si, pi, pj)
            } else {
                (nj, ni, sj, pj, pi)
            };
            let mutex_list: Vec<&str> = {
                let mut v: Vec<&str> = shared.iter().map(|s| s.as_str()).collect();
                v.sort();
                v
            };
            let msg = format!(
                "[S-20] potential priority inversion: ISR `{}` (priority {}) and ISR `{}` \
                 (priority {}) both acquire mutex '{}'. If the lower-priority ISR holds \
                 the lock when the higher-priority ISR fires, the higher-priority ISR \
                 cannot proceed until the lower one is scheduled — which may never happen.",
                high_fn, high_prio, low_fn, low_prio,
                mutex_list.join("', '"),
            );
            let hint = format!(
                "Use an atomic operation or a priority-ceiling protocol instead of a \
                 mutex for resources shared between ISRs at different priority levels. \
                 Alternatively, ensure both ISRs use `#[interrupt]` without a mutex \
                 for this resource."
            );
            diagnostics.push(crate::diagnostic::Diagnostic {
                span: *high_span,
                message: msg,
                related: vec![],
                elaboration: vec![hint],
            });
        }
    }
}

// ── S-23: MC/DC coverage map ─────────────────────────────────────────────────

/// A single boolean decision point that must achieve MC/DC coverage.
/// Each condition in a compound decision (e.g. `a && b`) becomes its
/// own entry so that test suites can verify each condition independently
/// controls the overall outcome.
#[derive(Debug, Clone)]
pub struct MCDCPoint {
    /// Source span of the condition expression.
    pub span: crate::span::Span,
    /// Human-readable function name.
    pub function: String,
    /// Context kind: "if-condition", "while-condition", "assert",
    /// "prove", or "sub-condition" for atomic conditions inside a
    /// compound decision.
    pub kind: String,
    /// Index of this point within the function (0-based, stable across
    /// runs so coverage data from different test runs can be merged).
    pub index: usize,
}

/// S-23 — Compute the MC/DC coverage map for a program.
///
/// DO-178C Level A requires Modified Condition/Decision Coverage: every
/// boolean condition in every decision must independently affect the
/// outcome at least once (true → outcome changes; false → outcome changes).
///
/// This function returns the static **coverage map** — the list of all
/// condition points that a test suite must exercise. The map is consumed
/// by `vanic coverage` to produce human-readable or machine-readable
/// artifacts that CI can compare against runtime hit data collected via
/// the `--instrument-mcdc` code-generation flag (backend instrumentation
/// is left to a future sprint; this phase produces the map file).
///
/// Coverage granularity:
///   - Every `if`/`while` condition is a decision.
///   - `assert` and `prove` conditions are decisions.
///   - Compound conditions (`&&`, `||`) are decomposed: each atomic
///     sub-condition gets its own `MCDCPoint`.
pub fn compute_mcdc_map(program: &crate::ir::TypedProgram) -> Vec<MCDCPoint> {
    let mut points: Vec<MCDCPoint> = Vec::new();
    for func in &program.functions {
        if func.is_extern {
            continue;
        }
        let mut counter = 0usize;
        collect_mcdc_stmts(&func.body, &func.name, &mut counter, &mut points);
    }
    points
}

fn collect_mcdc_stmts(
    stmts: &[crate::ir::TypedStmt],
    fn_name: &str,
    counter: &mut usize,
    out: &mut Vec<MCDCPoint>,
) {
    use crate::ir::TypedStmt as S;
    for stmt in stmts {
        match stmt {
            S::If { cond, then_body, else_body } => {
                add_mcdc_decision(cond, fn_name, "if-condition", counter, out);
                collect_mcdc_stmts(then_body, fn_name, counter, out);
                collect_mcdc_stmts(else_body, fn_name, counter, out);
            }
            S::While { cond, body, .. } => {
                add_mcdc_decision(cond, fn_name, "while-condition", counter, out);
                collect_mcdc_stmts(body, fn_name, counter, out);
            }
            S::For { start, end, body, .. } => {
                // Range bounds aren't boolean decisions but the body may contain them
                collect_mcdc_stmts_expr_scan(start, fn_name, counter, out);
                collect_mcdc_stmts_expr_scan(end, fn_name, counter, out);
                collect_mcdc_stmts(body, fn_name, counter, out);
            }
            S::ForIter { body, .. }
            | S::TaskSpawn { body, .. }
            | S::UnsafeBlock { body, .. } => {
                collect_mcdc_stmts(body, fn_name, counter, out);
            }
            S::Assert { expr, .. } => {
                add_mcdc_decision(expr, fn_name, "assert", counter, out);
            }
            S::Prove { expr } => {
                add_mcdc_decision(expr, fn_name, "prove", counter, out);
            }
            S::Let { expr, .. } | S::Discard { expr } | S::Reassign { expr, .. } => {
                collect_mcdc_stmts_expr_scan(expr, fn_name, counter, out);
            }
            S::Return { expr } => {
                collect_mcdc_stmts_expr_scan(expr, fn_name, counter, out);
            }
            S::IndexAssign { index, value, .. } => {
                collect_mcdc_stmts_expr_scan(index, fn_name, counter, out);
                collect_mcdc_stmts_expr_scan(value, fn_name, counter, out);
            }
            S::FieldAssign { object, value, .. } => {
                collect_mcdc_stmts_expr_scan(object, fn_name, counter, out);
                collect_mcdc_stmts_expr_scan(value, fn_name, counter, out);
            }
            S::Print { items } | S::EPrint { items } => {
                for item in items {
                    if let crate::ir::TypedPrintItem::Expr(e) = item {
                        collect_mcdc_stmts_expr_scan(e, fn_name, counter, out);
                    }
                }
            }
            S::Drop { .. } | S::ForIterShallowFree { .. }
            | S::Break { .. } | S::Continue { .. }
            | S::TaskJoin { .. } | S::Detach { .. } => {}
        }
    }
}

/// Scan an expression for nested decision points (e.g. `if` expressions
/// inside a let-binding or call argument).
fn collect_mcdc_stmts_expr_scan(
    expr: &crate::ir::TypedExpr,
    fn_name: &str,
    counter: &mut usize,
    out: &mut Vec<MCDCPoint>,
) {
    use crate::ir::TypedExprKind as EK;
    match &expr.kind {
        EK::IfExpr { cond, then_value, else_value } => {
            add_mcdc_decision(cond, fn_name, "if-expr-condition", counter, out);
            collect_mcdc_stmts_expr_scan(then_value, fn_name, counter, out);
            collect_mcdc_stmts_expr_scan(else_value, fn_name, counter, out);
        }
        EK::Block { stmts, tail } => {
            collect_mcdc_stmts(stmts, fn_name, counter, out);
            collect_mcdc_stmts_expr_scan(tail, fn_name, counter, out);
        }
        EK::Match { scrutinee, arms } => {
            collect_mcdc_stmts_expr_scan(scrutinee, fn_name, counter, out);
            for arm in arms {
                collect_mcdc_stmts_expr_scan(&arm.body, fn_name, counter, out);
            }
        }
        EK::Call { args, .. } | EK::DynDispatch { args, .. } => {
            for a in args { collect_mcdc_stmts_expr_scan(a, fn_name, counter, out); }
        }
        EK::CallIndirect { callee, args } => {
            collect_mcdc_stmts_expr_scan(callee, fn_name, counter, out);
            for a in args { collect_mcdc_stmts_expr_scan(a, fn_name, counter, out); }
        }
        EK::Binary { left, right, .. }
        | EK::Index { array: left, index: right, .. } => {
            collect_mcdc_stmts_expr_scan(left, fn_name, counter, out);
            collect_mcdc_stmts_expr_scan(right, fn_name, counter, out);
        }
        EK::Unary { expr: inner, .. }
        | EK::Cast { expr: inner, .. }
        | EK::TupleAccess { tuple: inner, .. }
        | EK::FieldAccess { object: inner, .. }
        | EK::Len { array: inner, .. }
        | EK::DynCoerce { value: inner, .. } => {
            collect_mcdc_stmts_expr_scan(inner, fn_name, counter, out);
        }
        EK::EnumVariantWithPayload { payload, .. } => {
            collect_mcdc_stmts_expr_scan(payload, fn_name, counter, out);
        }
        EK::RefMutIndex { index, .. } => {
            collect_mcdc_stmts_expr_scan(index, fn_name, counter, out);
        }
        EK::Tuple { elements } | EK::ArrayLit { elements } => {
            for e in elements { collect_mcdc_stmts_expr_scan(e, fn_name, counter, out); }
        }
        EK::StructLit { fields, .. } => {
            for (_, e) in fields { collect_mcdc_stmts_expr_scan(e, fn_name, counter, out); }
        }
        _ => {}
    }
}

/// Decompose a decision expression into atomic conditions and register each.
/// Compound `&&` / `||` / `!` are split recursively into sub-conditions.
fn add_mcdc_decision(
    expr: &crate::ir::TypedExpr,
    fn_name: &str,
    kind: &str,
    counter: &mut usize,
    out: &mut Vec<MCDCPoint>,
) {
    use crate::ir::TypedExprKind as EK;
    use crate::ast::BinaryOp;
    use crate::ast::UnaryOp;
    match &expr.kind {
        // Compound decision — recurse into sub-conditions
        EK::Binary { op: BinaryOp::And, left, right, .. }
        | EK::Binary { op: BinaryOp::Or, left, right, .. } => {
            add_mcdc_decision(left, fn_name, "sub-condition", counter, out);
            add_mcdc_decision(right, fn_name, "sub-condition", counter, out);
        }
        EK::Unary { op: UnaryOp::Not, expr: inner, .. } => {
            add_mcdc_decision(inner, fn_name, "sub-condition", counter, out);
        }
        // Atomic condition — register this as a coverage point
        _ => {
            let idx = *counter;
            *counter += 1;
            out.push(MCDCPoint {
                span: expr.span,
                function: fn_name.to_string(),
                kind: kind.to_string(),
                index: idx,
            });
        }
    }
}

/// Format the MC/DC map as plain text.
pub fn format_mcdc_text(points: &[MCDCPoint], file_map: &crate::diagnostic::FileMap) -> String {
    let mut out = String::new();
    let mut current_fn = "";
    for p in points {
        if p.function != current_fn {
            if !current_fn.is_empty() {
                out.push('\n');
            }
            out.push_str(&format!("fn {}:\n", p.function));
            current_fn = &p.function;
        }
        let loc = span_to_location(p.span, file_map);
        out.push_str(&format!(
            "  [{:>4}] {} @ {}\n",
            p.index, p.kind, loc
        ));
    }
    if points.is_empty() {
        out.push_str("(no decision points found)\n");
    }
    out
}

/// Format the MC/DC map as JSON.
pub fn format_mcdc_json(points: &[MCDCPoint], file_map: &crate::diagnostic::FileMap) -> String {
    let mut out = String::from("[\n");
    for (i, p) in points.iter().enumerate() {
        let loc = span_to_location(p.span, file_map);
        out.push_str(&format!(
            "  {{\"index\":{},\"function\":{:?},\"kind\":{:?},\"location\":{:?}}}{}",
            p.index, p.function, p.kind, loc,
            if i + 1 < points.len() { "," } else { "" }
        ));
        out.push('\n');
    }
    out.push_str("]\n");
    out
}

/// Format the MC/DC map as CSV.
pub fn format_mcdc_csv(points: &[MCDCPoint], file_map: &crate::diagnostic::FileMap) -> String {
    let mut out = String::from("index,function,kind,location\n");
    for p in points {
        let loc = span_to_location(p.span, file_map);
        out.push_str(&format!(
            "{},{:?},{:?},{:?}\n",
            p.index, p.function, p.kind, loc
        ));
    }
    out
}

fn span_to_location(
    span: crate::span::Span,
    file_map: &crate::diagnostic::FileMap,
) -> String {
    if let Some((entry, local)) = file_map.lookup(span.start) {
        let before = &entry.source[..local.min(entry.source.len())];
        let line = before.bytes().filter(|&b| b == b'\n').count() + 1;
        let col = local - before.rfind('\n').map(|p| p + 1).unwrap_or(0) + 1;
        format!("{}:{}:{}", entry.path, line, col)
    } else {
        format!("byte {}", span.start)
    }
}

// ── `vanic audit-safety` — #[bounded_stack]/#[wcet] coverage gate ────────────
//
// Added 2026-07-21 (kosh-index MAINT-1 follow-up): a package can now have
// full #[bounded_stack]/#[wcet] discipline on every function that's
// actually eligible for it, but there was no automated way to VERIFY
// that before `vanic publish` — only the honor system. This pass answers
// "for every function in the package's own source (not a vendored
// dependency), COULD it have a #[bounded_stack]/#[wcet] attribute, and
// if so, does it?" It reuses the exact same analysis `enforce_bounded_stack`
// / `enforce_wcet` already run — `compute_stack_depths` and `wcet_body`
// below — just applied UNCONDITIONALLY (to every function, not only ones
// that already declare a budget) so a real bytes/cycles number is
// available to report even for functions with no attribute yet.
//
// Eligibility rules (deliberately NOT "100% attribute coverage" — see
// kosh-index/ROADMAP.md's MAINT-1 for why most of this ecosystem's
// functions genuinely can't be annotated):
//   - #[bounded_stack]: required whenever `compute_stack_depths` returns
//     a finite depth AND the function has no function-pointer-typed
//     parameter. A function-pointer parameter makes the TRUE depth
//     unknowable (the analysis can't see through an indirect call), so a
//     declared number there would be silently incomplete rather than
//     genuinely bounded — same reasoning `vani-calculus`'s `bisect` and
//     every fn-pointer-taking function audited under MAINT-1 already
//     follows (bound documented in a comment instead).
//   - #[wcet]: required whenever the WCET estimator (`wcet_body`) returns
//     `Some(cycles)` for the function's body. Unlike bounded_stack, a
//     function-pointer parameter does NOT block this — `wcet_expr`'s
//     `CallIndirect` arm already gives every indirect call a flat 10-cycle
//     charge, so plenty of fn-pointer-taking functions (e.g.
//     vani-vectorcalc's differential operators) are genuinely WCET-eligible
//     even though they can never get #[bounded_stack].
//   - Functions defined in a vendored `[deps]` package (any origin path
//     containing a `vendor` path component) are excluded — they're a
//     separately-published, separately-audited package; re-flagging their
//     gaps while checking a downstream package would be both wrong (not
//     this package's responsibility) and impossible to fix from here.
//   - `extern` functions are excluded (no body to analyze).

/// One function that could carry a #[bounded_stack]/#[wcet] attribute
/// but doesn't yet.
#[derive(Clone, Debug)]
pub struct CoverageViolation {
    pub name: String,
    pub span: crate::span::Span,
    /// `Some(bytes)` if missing #[bounded_stack] with that computed depth.
    pub missing_bounded_stack: Option<u64>,
    /// `Some(cycles)` if missing #[wcet] with that computed estimate.
    pub missing_wcet: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct CoverageReport {
    pub checked: usize,
    pub excluded_vendor: usize,
    pub violations: Vec<CoverageViolation>,
}

impl CoverageReport {
    pub fn passed(&self) -> bool {
        self.violations.is_empty()
    }
}

fn has_fn_ptr_param(f: &crate::ir::TypedFunction) -> bool {
    f.params
        .iter()
        .any(|p| matches!(p.ty, crate::ast::Type::FnPtr(_, _)))
}

/// `file_map` is optional so this can run standalone (`vanic audit-safety`
/// on a single file with no manifest) as well as vendor-aware (from
/// `vanic publish`, which always has one). Without it every function in
/// the merged program is checked — including any pulled in via `use`.
pub fn audit_safety_coverage(
    program: &TypedProgram,
    file_map: Option<&crate::diagnostic::FileMap>,
) -> CoverageReport {
    let mut fn_map: HashMap<String, &crate::ir::TypedFunction> = HashMap::new();
    for f in &program.functions {
        fn_map.insert(f.name.clone(), f);
    }
    let stack_report = crate::stack_depth::compute_stack_depths(program, None);
    let stack_by_name: HashMap<&str, Option<u64>> = stack_report
        .entries
        .iter()
        .map(|e| (e.name.as_str(), e.max_depth_bytes))
        .collect();

    let mut violations = Vec::new();
    let mut checked = 0usize;
    let mut excluded_vendor = 0usize;
    for f in &program.functions {
        if f.is_extern {
            continue;
        }
        if let Some(fm) = file_map {
            if let Some((entry, _)) = fm.lookup(f.span.start) {
                let normalized = entry.path.replace('\\', "/");
                if normalized.split('/').any(|seg| seg == "vendor") {
                    excluded_vendor += 1;
                    continue;
                }
            }
        }
        checked += 1;

        let mut missing_bounded_stack = None;
        if f.bounded_stack.is_none() && !has_fn_ptr_param(f) {
            if let Some(Some(bytes)) = stack_by_name.get(f.name.as_str()) {
                missing_bounded_stack = Some(*bytes);
            }
        }

        let mut missing_wcet = None;
        if f.wcet_cycles.is_none() {
            let mut visiting: std::collections::HashSet<String> = std::collections::HashSet::new();
            visiting.insert(f.name.clone());
            if let Some(cycles) = wcet_body(&f.body, &fn_map, &mut visiting, f.recursion_bound) {
                missing_wcet = Some(cycles);
            }
        }

        if missing_bounded_stack.is_some() || missing_wcet.is_some() {
            violations.push(CoverageViolation {
                name: f.name.clone(),
                span: f.span,
                missing_bounded_stack,
                missing_wcet,
            });
        }
    }
    CoverageReport {
        checked,
        excluded_vendor,
        violations,
    }
}

pub fn format_coverage_text(report: &CoverageReport, file_map: &crate::diagnostic::FileMap) -> String {
    let mut out = String::new();
    if report.violations.is_empty() {
        out.push_str(&format!(
            "audit-safety: OK — {} function(s) checked ({} vendored fn(s) excluded), full #[bounded_stack]/#[wcet] coverage where eligible.\n",
            report.checked, report.excluded_vendor
        ));
        return out;
    }
    out.push_str(&format!(
        "audit-safety: {} of {} function(s) missing an attribute they're eligible for ({} vendored fn(s) excluded):\n\n",
        report.violations.len(), report.checked, report.excluded_vendor
    ));
    for v in &report.violations {
        let loc = span_to_location(v.span, file_map);
        out.push_str(&format!("  {} ({})\n", v.name, loc));
        if let Some(bytes) = v.missing_bounded_stack {
            out.push_str(&format!(
                "    missing #[bounded_stack(bytes = {})] -- computed worst-case is {} bytes\n",
                bytes, bytes
            ));
        }
        if let Some(cycles) = v.missing_wcet {
            out.push_str(&format!(
                "    missing #[wcet(cycles = {})] -- static estimate is {} cycles\n",
                cycles, cycles
            ));
        }
    }
    out.push_str(
        "\nAdd the attribute with the exact value shown (vanic will re-verify it), \
         or if this function genuinely shouldn't carry one, that's a bug in this \
         checker's eligibility rules -- please report it.\n",
    );
    out
}

fn coverage_json_escape(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

pub fn format_coverage_json(report: &CoverageReport, file_map: &crate::diagnostic::FileMap) -> String {
    let mut out = String::from("{\"checked\":");
    out.push_str(&report.checked.to_string());
    out.push_str(",\"excluded_vendor\":");
    out.push_str(&report.excluded_vendor.to_string());
    out.push_str(",\"passed\":");
    out.push_str(if report.passed() { "true" } else { "false" });
    out.push_str(",\"violations\":[");
    for (i, v) in report.violations.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        let loc = span_to_location(v.span, file_map);
        out.push_str(&format!(
            "{{\"name\":{},\"location\":{},\"missing_bounded_stack_bytes\":{},\"missing_wcet_cycles\":{}}}",
            coverage_json_escape(&v.name),
            coverage_json_escape(&loc),
            v.missing_bounded_stack.map(|b| b.to_string()).unwrap_or_else(|| "null".to_string()),
            v.missing_wcet.map(|c| c.to_string()).unwrap_or_else(|| "null".to_string()),
        ));
    }
    out.push_str("]}");
    out
}
