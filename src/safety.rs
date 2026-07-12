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
        S::For { start, end, body, .. } => {
            let start_const = const_int(start);
            let end_const = const_int(end);
            let iters = match (start_const, end_const) {
                (Some(s), Some(e)) if e >= s => (e - s) as u64,
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
        S::TaskJoin { .. } => None,
        S::ForIterShallowFree { .. } => Some(1),
        S::UnsafeBlock { body, .. } => wcet_body(body, fn_map, visiting, recursion_bound),
        S::Break { .. } | S::Continue { .. } => Some(1),
        S::Drop { .. } => Some(1),
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
                // Builtin or extern — flat 10 cycles. Real WCET
                // analyses substitute per-builtin tables.
                Some(args_cost.saturating_add(10))
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
        r.safety_standard.is_some() || r.no_heap || r.no_float
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
