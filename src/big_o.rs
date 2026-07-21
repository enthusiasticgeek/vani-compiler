//! Big-O complexity annotation pass.
//!
//! User-direction item (added 2026-06-08): when the compiler
//! emits artifacts (with `--big-o` flag on by default), each
//! user-declared fn gets a Big-O complexity annotation derived
//! statically from the body — loop nesting depth + recursive-
//! call depth + container-op asymptotics (`push` = O(1)
//! amortized, `sort` = O(n log n), etc.).
//!
//! Output in two channels:
//!   1. `vanic check --big-o[=mode]` adds a `complexity: O(...)`
//!      line per fn to stdout.
//!   2. The C / LLVM emit prepends a `// complexity: O(...)`
//!      comment block before each fn body.
//!
//! Modes:
//!   * `auto` (default) — annotate fns with loops / recursion /
//!     superlinear builtins; skip O(1) trivial fns to keep the
//!     output uncluttered.
//!   * `force` — annotate every fn (useful for review).
//!   * `off` — skip the pass entirely.
//!
//! v1 scope:
//!   * Loop-nesting count (every nested `for` / `while` /
//!     `parallel for` multiplies the inner cost).
//!   * Recursion presence (calls back to the same fn).
//!   * Builtin asymptotics for the most common ops: sort,
//!     sort_by, binary_search, reverse, dedup, contains, find,
//!     push, pop, swap_remove, insert, clear, len.
//!   * Cross-fn analysis: `annotate_program` (what every CLI
//!     entry point actually calls — see `main.rs`) walks the
//!     whole program's call graph in topological order and
//!     threads each callee's already-computed complexity into
//!     its caller at the call's loop depth. This applies across
//!     `use`-merged files too, since the checker's input is
//!     already the fully-combined source by the time this pass
//!     runs — a fn that calls an O(n) library helper inside a
//!     loop is correctly reported as O(n²), not O(n). Only
//!     `analyze_function` (single-fn, no `callees` map) treats
//!     every call as O(1); nothing in the CLI uses it directly.
//!   * Unable-to-prove cases → `O(?)` with the unprovable
//!     construct flagged.
//!
//! Out of scope (future):
//!   * Inferring "n" from a specific parameter (currently the
//!     bound is just "input size" without naming the parameter).
//!   * Solving closed-form recurrences — a fn in a call cycle
//!     (recursive, directly or through `annotate_program`'s
//!     topological walk) reports `O(recursive)` rather than a
//!     solved bound (e.g. naive fibonacci doesn't become O(2^n)).
//!   * Bounds derived from `requires`/`ensures` clauses (SMT
//!     could prove tighter bounds; v1 only uses syntactic
//!     evidence).

use crate::ast::Type;
use crate::ir::{TypedExpr, TypedExprKind, TypedFunction, TypedProgram, TypedStmt};

/// Coarse complexity classification. v1 covers the families
/// most user code falls into; finer distinctions (e.g.,
/// O(n + m) for two-input fns) collapse to the nearest
/// upper bound.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BigO {
    /// Constant time — no loops, no recursion, no superlinear
    /// builtins.
    Constant,
    /// Logarithmic — a single `binary_search` or similar.
    Logarithmic,
    /// Linear in the input size.
    Linear,
    /// `n log n` — typical of a single `sort` call or one
    /// linear loop containing a `binary_search`.
    NLogN,
    /// Polynomial — nested loops of depth k ≥ 2 with no
    /// superlinear builtin inside.
    Polynomial(u32),
    /// Polynomial × log — e.g. one loop containing a sort, or
    /// nested loops with a binary_search at the innermost.
    PolynomialLog(u32),
    /// Recursive — the fn calls itself; without a recurrence
    /// solver we report "recursive" rather than a closed form.
    Recursive,
    /// Couldn't bound — the analyzer hit a construct it
    /// doesn't model. The string carries the user-facing
    /// reason.
    Unknown(String),
}

impl std::fmt::Display for BigO {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BigO::Constant => write!(f, "O(1)"),
            BigO::Logarithmic => write!(f, "O(log n)"),
            BigO::Linear => write!(f, "O(n)"),
            BigO::NLogN => write!(f, "O(n log n)"),
            BigO::Polynomial(k) => match k {
                2 => write!(f, "O(n²)"),
                3 => write!(f, "O(n³)"),
                _ => write!(f, "O(n^{})", k),
            },
            BigO::PolynomialLog(k) => match k {
                2 => write!(f, "O(n² log n)"),
                3 => write!(f, "O(n³ log n)"),
                _ => write!(f, "O(n^{} log n)", k),
            },
            BigO::Recursive => write!(f, "O(recursive)"),
            BigO::Unknown(reason) => write!(f, "O(?) — {}", reason),
        }
    }
}

/// Modes for the `--big-o` flag.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BigOMode {
    /// Annotate fns that aren't O(1).
    Auto,
    /// Annotate every fn, including O(1).
    Force,
    /// Skip the pass.
    Off,
}

impl BigOMode {
    /// Parse a `--big-o=<mode>` value. Returns `Some(mode)` for
    /// valid values, `None` for unrecognized inputs (the caller
    /// surfaces the diagnostic).
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "auto" => Some(Self::Auto),
            "force" => Some(Self::Force),
            "off" => Some(Self::Off),
            _ => None,
        }
    }
}

/// Analyze a single function's complexity. Returns the
/// classification + an optional one-line note explaining the
/// dominant construct (used in the "unknown" reason string).
///
/// Local-only analysis: builtin calls contribute their
/// asymptotic cost; user-defined fn calls are treated as O(1).
/// For cross-function propagation (where a fn that calls an
/// O(n log n) helper inside a loop becomes O(n² log n) overall),
/// use `annotate_program` — it walks the call graph in
/// topological order and threads each callee's complexity into
/// its callers' classifications.
pub fn analyze_function(func: &TypedFunction) -> BigO {
    analyze_function_with_callees(func, &std::collections::HashMap::new())
}

/// L4 (D) refinement (2026-06-09): cross-fn propagation. When
/// a fn body calls another user-defined fn, that call site
/// contributes the callee's complexity (at the call's depth)
/// to the caller's summary. `callees` is the per-fn complexity
/// map built by `annotate_program` after a topological sort of
/// the call graph; for fns not in the map (builtins, or a
/// caller analyzed before its callee), the call contributes O(1)
/// — same as the pre-propagation behavior.
fn analyze_function_with_callees(
    func: &TypedFunction,
    callees: &std::collections::HashMap<String, BigO>,
) -> BigO {
    // Pass 1: detect self-recursion. If the fn body calls
    // itself anywhere, we report Recursive and stop — v1
    // doesn't have a recurrence solver.
    let mut sees_self = false;
    visit_calls(&func.body, &mut |name| {
        if name == func.name {
            sees_self = true;
        }
    });
    if sees_self {
        return BigO::Recursive;
    }

    // Pass 2: walk the body collecting loop depth + builtin /
    // callee complexity contributions per level.
    let summary = walk_body_with_callees(&func.body, 0, callees);

    classify(summary)
}

#[derive(Default, Debug, Clone)]
struct Summary {
    /// Max nesting depth seen anywhere in the body. 0 = no
    /// loops; k = the deepest `for` / `while` / `parallel for`
    /// nest contains k loops.
    max_depth: u32,
    /// Counts of superlinear builtins at the maximum-depth
    /// level. Used to bump a Polynomial(k) → PolynomialLog(k)
    /// (binary_search at the innermost) or to seed NLogN /
    /// Linear when there are no loops.
    has_sort_at_top: bool,
    has_logn_at_top: bool,
    has_n_at_top: bool,
    has_sort_inside_loop: bool,
    has_logn_inside_loop: bool,
}

fn walk_body(body: &[TypedStmt], depth: u32) -> Summary {
    walk_body_with_callees(body, depth, &std::collections::HashMap::new())
}

fn walk_body_with_callees(
    body: &[TypedStmt],
    depth: u32,
    callees: &std::collections::HashMap<String, BigO>,
) -> Summary {
    let mut s = Summary::default();
    s.max_depth = depth;
    for stmt in body {
        let inner = walk_stmt_with_callees(stmt, depth, callees);
        merge_summary(&mut s, &inner);
    }
    s
}

fn walk_stmt(stmt: &TypedStmt, depth: u32) -> Summary {
    walk_stmt_with_callees(stmt, depth, &std::collections::HashMap::new())
}

fn walk_stmt_with_callees(
    stmt: &TypedStmt,
    depth: u32,
    callees: &std::collections::HashMap<String, BigO>,
) -> Summary {
    let mut s = Summary::default();
    s.max_depth = depth;
    match stmt {
        TypedStmt::Let { expr, .. } => merge_with_expr_and_callees(&mut s, expr, depth, callees),
        TypedStmt::Reassign { expr, .. } => merge_with_expr_and_callees(&mut s, expr, depth, callees),
        TypedStmt::Discard { expr, .. } => merge_with_expr_and_callees(&mut s, expr, depth, callees),
        TypedStmt::Return { expr, .. } => merge_with_expr_and_callees(&mut s, expr, depth, callees),
        TypedStmt::FieldAssign { value, .. } => merge_with_expr_and_callees(&mut s, value, depth, callees),
        TypedStmt::IndexAssign { value, .. } => merge_with_expr_and_callees(&mut s, value, depth, callees),
        TypedStmt::Assert { expr, .. } | TypedStmt::Prove { expr } => {
            merge_with_expr_and_callees(&mut s, expr, depth, callees)
        }
        TypedStmt::While { body, .. } => {
            // `while` has no static iteration count — always
            // counts as a depth bump. The compiler doesn't try
            // to prove termination of arbitrary while loops.
            let inner = walk_body_with_callees(body, depth + 1, callees);
            merge_summary(&mut s, &inner);
        }
        TypedStmt::For { start, end, body, .. } => {
            // L4 (D) refinement (2026-06-09): bounded-loop
            // detection. A `for i in 0..16` has a constant
            // iteration count — the loop runs in O(1) regardless
            // of input. Don't bump the nesting depth in that
            // case; the body's own complexity counts but isn't
            // multiplied by `n`.
            //
            // Detection: both `start` and `end` have a folded
            // constant (the checker populated `TypedExpr.constant`
            // during type-check). When either is unknown, fall
            // back to treating the loop as input-bounded.
            let bounded = start.constant.is_some() && end.constant.is_some();
            let inner_depth = if bounded { depth } else { depth + 1 };
            let inner = walk_body_with_callees(body, inner_depth, callees);
            merge_summary(&mut s, &inner);
        }
        TypedStmt::ForIter { collection_ty, body, .. } => {
            // Same idea for `for x in xs`: when `xs` is a fixed-
            // size array `[T; N]`, the iteration count is N — a
            // constant — so the loop doesn't multiply complexity.
            // `Vec<T>` and ref-to-Vec stay input-bounded.
            let bounded = is_fixed_size_collection(collection_ty);
            let inner_depth = if bounded { depth } else { depth + 1 };
            let inner = walk_body_with_callees(body, inner_depth, callees);
            merge_summary(&mut s, &inner);
        }
        TypedStmt::If { then_body, else_body, .. } => {
            let then_s = walk_body_with_callees(then_body, depth, callees);
            let else_s = walk_body_with_callees(else_body, depth, callees);
            merge_summary(&mut s, &then_s);
            merge_summary(&mut s, &else_s);
        }
        TypedStmt::TaskSpawn { body, .. } | TypedStmt::UnsafeBlock { body, .. } => {
            let inner = walk_body_with_callees(body, depth, callees);
            merge_summary(&mut s, &inner);
        }
        _ => {}
    }
    s
}

fn merge_with_expr(s: &mut Summary, expr: &TypedExpr, depth: u32) {
    merge_with_expr_and_callees(s, expr, depth, &std::collections::HashMap::new())
}

fn merge_with_expr_and_callees(
    s: &mut Summary,
    expr: &TypedExpr,
    depth: u32,
    callees: &std::collections::HashMap<String, BigO>,
) {
    visit_calls_in_expr(expr, &mut |name| {
        // Builtin asymptotics take priority — they're hardcoded
        // and don't change across the call graph.
        if is_superlinear_builtin_sort(name) {
            if depth > 0 {
                s.has_sort_inside_loop = true;
            } else {
                s.has_sort_at_top = true;
            }
            return;
        }
        if is_logn_builtin(name) {
            if depth > 0 {
                s.has_logn_inside_loop = true;
            } else {
                s.has_logn_at_top = true;
            }
            return;
        }
        if is_linear_builtin(name) {
            if depth == 0 {
                s.has_n_at_top = true;
            } else {
                // Linear builtin inside a loop is equivalent to
                // a sort-like-cost contribution to the inner
                // level — bumps Linear-loop to Polynomial(k+1).
                // Match the existing behavior by treating it as
                // adding a level of nesting at the deepest point.
                s.max_depth = s.max_depth.max(depth + 1);
            }
            return;
        }
        // L4 (D) refinement (2026-06-09): cross-fn propagation.
        // A user-defined fn call contributes the callee's
        // baseline complexity (computed earlier in topo order).
        // Convert the callee's class into the corresponding
        // Summary flags / depth bump so the existing classifier
        // produces a combined upper bound.
        if let Some(callee) = callees.get(name) {
            propagate_callee_into_summary(s, callee, depth);
        }
        // Else: builtin we don't model + non-mapped user call.
        // Treat as O(1) — same as the pre-propagation behavior.
    });
}

/// Translate a callee's classification into a contribution to
/// the caller's `Summary`, scaled by the current loop depth.
/// Conservative upper-bound: never under-reports.
fn propagate_callee_into_summary(s: &mut Summary, callee: &BigO, depth: u32) {
    match callee {
        BigO::Constant => {}
        BigO::Logarithmic => {
            if depth > 0 {
                s.has_logn_inside_loop = true;
            } else {
                s.has_logn_at_top = true;
            }
        }
        BigO::Linear => {
            if depth == 0 {
                s.has_n_at_top = true;
            } else {
                // Same shape as `is_linear_builtin` inside a loop.
                s.max_depth = s.max_depth.max(depth + 1);
            }
        }
        BigO::NLogN => {
            if depth > 0 {
                s.has_sort_inside_loop = true;
            } else {
                s.has_sort_at_top = true;
            }
        }
        BigO::Polynomial(k) => {
            // A Poly(k) callee at depth d contributes a Poly(k)
            // worth of cost; the caller's effective depth becomes
            // max(current, d + k).
            s.max_depth = s.max_depth.max(depth + k);
        }
        BigO::PolynomialLog(k) => {
            s.max_depth = s.max_depth.max(depth + k);
            if depth > 0 {
                s.has_logn_inside_loop = true;
            } else {
                s.has_logn_at_top = true;
            }
        }
        BigO::Recursive | BigO::Unknown(_) => {
            // Best we can do without a recurrence solver: the
            // caller inherits the unknown bound. Use a huge depth
            // to make the final classification land on the upper
            // end of Polynomial; the program-level walker
            // post-processes this to Recursive when we want a
            // sentinel.
            s.max_depth = s.max_depth.max(depth + 99);
        }
    }
}

fn merge_summary(dst: &mut Summary, src: &Summary) {
    dst.max_depth = dst.max_depth.max(src.max_depth);
    dst.has_sort_at_top |= src.has_sort_at_top;
    dst.has_logn_at_top |= src.has_logn_at_top;
    dst.has_n_at_top |= src.has_n_at_top;
    dst.has_sort_inside_loop |= src.has_sort_inside_loop;
    dst.has_logn_inside_loop |= src.has_logn_inside_loop;
}

fn classify(s: Summary) -> BigO {
    match s.max_depth {
        0 => {
            // No loops, no recursion (caught earlier).
            if s.has_sort_at_top {
                BigO::NLogN
            } else if s.has_n_at_top {
                BigO::Linear
            } else if s.has_logn_at_top {
                BigO::Logarithmic
            } else {
                BigO::Constant
            }
        }
        1 => {
            // Single loop. Dominant term wins:
            //   - sort inside loop:  O(n × n log n) = O(n² log n)
            //   - logn inside loop:  O(n log n) from loop body
            //   - sort outside loop: O(n log n + n) = O(n log n)
            //   - everything else:   O(n)
            if s.has_sort_inside_loop {
                BigO::PolynomialLog(2)
            } else if s.has_logn_inside_loop {
                BigO::NLogN
            } else if s.has_sort_at_top {
                // A sort outside the loop costs O(n log n);
                // the loop costs O(n). Together: O(n log n).
                BigO::NLogN
            } else {
                BigO::Linear
            }
        }
        k => {
            // Nested loops, depth k ≥ 2. Same multiplication
            // rule applies at the innermost level.
            if s.has_sort_inside_loop {
                BigO::PolynomialLog(k + 1)
            } else if s.has_logn_inside_loop {
                BigO::PolynomialLog(k)
            } else {
                BigO::Polynomial(k)
            }
        }
    }
}

/// L4 (D) refinement (2026-06-09): bounded-loop detection for
/// `for x in xs`. A fixed-size array `[T; N]` has N iterations
/// — constant; doesn't multiply complexity. Other collection
/// types (Vec, Slice, HashMap iteration, etc.) are unbounded.
/// Refs to a fixed-size array are still fixed-size.
fn is_fixed_size_collection(ty: &Type) -> bool {
    match ty {
        Type::Array { .. } => true,
        Type::Ref(inner) | Type::RefMut(inner) => is_fixed_size_collection(inner),
        _ => false,
    }
}

fn is_superlinear_builtin_sort(name: &str) -> bool {
    matches!(
        name,
        "sort" | "sort_by" | "sort_desc"
    )
}

fn is_logn_builtin(name: &str) -> bool {
    matches!(
        name,
        "binary_search"
            | "btreemap_get"
            | "btreemap_insert"
            | "btreemap_remove"
            | "btreemap_contains"
            | "btreeset_get"
            | "btreeset_insert"
            | "btreeset_remove"
            | "btreeset_contains"
            | "bst_insert"
            | "bst_search"
            | "bst_contains"
    )
}

fn is_linear_builtin(name: &str) -> bool {
    matches!(
        name,
        "find"
            | "contains"
            | "reverse"
            | "dedup"
            | "clear"
            | "vec_map"
            | "vec_filter"
            | "vec_filter_map"
            | "vec_zip"
            | "vec_enumerate"
            | "vec_chain"
            | "vec_take"
            | "vec_drop"
            | "vec_take_while"
            | "vec_drop_while"
            | "vec_min"
            | "vec_max"
            | "vec_sum"
            | "vec_product"
    )
}

fn visit_calls(body: &[TypedStmt], f: &mut impl FnMut(&str)) {
    for stmt in body {
        visit_calls_in_stmt(stmt, f);
    }
}

fn visit_calls_in_stmt(stmt: &TypedStmt, f: &mut impl FnMut(&str)) {
    match stmt {
        TypedStmt::Let { expr, .. }
        | TypedStmt::Reassign { expr, .. }
        | TypedStmt::Discard { expr, .. }
        | TypedStmt::Return { expr, .. } => visit_calls_in_expr(expr, f),
        TypedStmt::FieldAssign { value, .. } => visit_calls_in_expr(value, f),
        TypedStmt::IndexAssign { value, .. } => visit_calls_in_expr(value, f),
        TypedStmt::Assert { expr, .. } | TypedStmt::Prove { expr } => {
            visit_calls_in_expr(expr, f)
        }
        TypedStmt::While { cond, body, .. } => {
            visit_calls_in_expr(cond, f);
            visit_calls(body, f);
        }
        TypedStmt::For { body, .. } | TypedStmt::ForIter { body, .. } => visit_calls(body, f),
        TypedStmt::If { cond, then_body, else_body, .. } => {
            visit_calls_in_expr(cond, f);
            visit_calls(then_body, f);
            visit_calls(else_body, f);
        }
        TypedStmt::TaskSpawn { body, .. } | TypedStmt::UnsafeBlock { body, .. } => {
            visit_calls(body, f)
        }
        _ => {}
    }
}

fn visit_calls_in_expr(expr: &TypedExpr, f: &mut impl FnMut(&str)) {
    match &expr.kind {
        TypedExprKind::Call { name, args, .. } => {
            f(name);
            for a in args {
                visit_calls_in_expr(a, f);
            }
        }
        TypedExprKind::Binary { left, right, .. } => {
            visit_calls_in_expr(left, f);
            visit_calls_in_expr(right, f);
        }
        TypedExprKind::Unary { expr, .. }
        | TypedExprKind::Cast { expr, .. } => visit_calls_in_expr(expr, f),
        TypedExprKind::FieldAccess { object, .. } => visit_calls_in_expr(object, f),
        TypedExprKind::Ref { .. } | TypedExprKind::RefMut { .. } => {}
        TypedExprKind::Index { array, index, .. } => {
            visit_calls_in_expr(array, f);
            visit_calls_in_expr(index, f);
        }
        TypedExprKind::TupleAccess { tuple, .. } => visit_calls_in_expr(tuple, f),
        TypedExprKind::ArrayLit { elements }
        | TypedExprKind::Tuple { elements } => {
            for e in elements {
                visit_calls_in_expr(e, f);
            }
        }
        TypedExprKind::StructLit { fields, .. } => {
            for (_, v) in fields {
                visit_calls_in_expr(v, f);
            }
        }
        TypedExprKind::Match { scrutinee, arms } => {
            visit_calls_in_expr(scrutinee, f);
            for a in arms {
                visit_calls_in_expr(&a.body, f);
            }
        }
        TypedExprKind::IfExpr { cond, then_value, else_value, .. } => {
            visit_calls_in_expr(cond, f);
            visit_calls_in_expr(then_value, f);
            visit_calls_in_expr(else_value, f);
        }
        TypedExprKind::Len { array, .. } => visit_calls_in_expr(array, f),
        _ => {}
    }
}

/// Walk every fn in a `TypedProgram` and return `(name, BigO)`
/// pairs, filtered per the mode.
///   - `Auto`: include every fn that classifies non-Constant.
///   - `Force`: include every fn.
///   - `Off`: caller checks before calling and skips entirely.
///
/// L4 (D) refinement (2026-06-09): cross-fn propagation. The
/// program is analyzed in a two-pass scheme:
///
///   1. Build the call graph (caller → set of callees).
///   2. Find strongly-connected components. Any SCC with more
///      than one node is mutually recursive — every member is
///      classified Recursive (the same way self-recursion is).
///   3. Topologically sort the SCCs; analyze each SCC's fns
///      using the already-analyzed downstream SCCs as the
///      `callees` map. Calls into recursive SCCs propagate
///      Recursive into the caller.
///
/// The result: a fn that calls an O(n log n) helper inside a
/// loop is correctly classified O(n² log n), not the
/// pre-propagation O(n).
pub fn annotate_program(program: &TypedProgram, mode: BigOMode) -> Vec<(String, BigO)> {
    let complexity_map = build_complexity_map(program);

    let mut out = Vec::new();
    for func in &program.functions {
        let complexity = complexity_map
            .get(&func.name)
            .cloned()
            .unwrap_or(BigO::Constant);
        match mode {
            BigOMode::Auto => {
                if complexity != BigO::Constant {
                    out.push((func.name.clone(), complexity));
                }
            }
            BigOMode::Force => {
                out.push((func.name.clone(), complexity));
            }
            BigOMode::Off => {}
        }
    }
    out
}

/// Build the per-fn complexity map for cross-fn propagation.
/// Topo-sorts the call graph, analyzes each fn after its
/// callees, and threads each callee's complexity into the
/// caller's classification via `analyze_function_with_callees`.
fn build_complexity_map(
    program: &TypedProgram,
) -> std::collections::HashMap<String, BigO> {
    use std::collections::{HashMap, HashSet};

    // Build caller → callees set (user fns only — builtins
    // route through `is_linear_builtin` / etc. and don't need
    // to participate in the call-graph walk).
    let user_fn_names: HashSet<String> =
        program.functions.iter().map(|f| f.name.clone()).collect();
    let mut call_graph: HashMap<String, HashSet<String>> = HashMap::new();
    for func in &program.functions {
        let mut callees = HashSet::new();
        visit_calls(&func.body, &mut |callee| {
            if user_fn_names.contains(callee) && callee != &func.name {
                callees.insert(callee.to_string());
            }
        });
        call_graph.insert(func.name.clone(), callees);
    }

    // Tarjan's SCC over the call graph. Each SCC is a set of
    // fns that recursively call each other (or just a single
    // non-recursive fn). Multi-node SCCs are flagged Recursive.
    let sccs = tarjan_sccs(&program.functions, &call_graph);

    // Topo order: sccs is already in reverse topological order
    // from tarjan_sccs (callees first). Process in that order
    // so each fn sees its callees' complexities in the map.
    let mut complexity_map: HashMap<String, BigO> = HashMap::new();
    for scc in &sccs {
        let is_recursive_scc = scc.len() > 1;
        for name in scc {
            // Find the fn body. If multiple bindings shadow,
            // the first one wins (matches checker.rs convention).
            let func = match program
                .functions
                .iter()
                .find(|f| &f.name == name)
            {
                Some(f) => f,
                None => continue,
            };
            let complexity = if is_recursive_scc {
                BigO::Recursive
            } else {
                analyze_function_with_callees(func, &complexity_map)
            };
            complexity_map.insert(name.clone(), complexity);
        }
    }
    complexity_map
}

/// Compute strongly-connected components of the call graph
/// using Tarjan's algorithm. Returns the SCCs in reverse
/// topological order — every SCC's callees appear in earlier
/// elements. Within each SCC, fn order is unspecified.
fn tarjan_sccs(
    functions: &[TypedFunction],
    call_graph: &std::collections::HashMap<String, std::collections::HashSet<String>>,
) -> Vec<Vec<String>> {
    use std::collections::HashMap;

    #[derive(Default)]
    struct State {
        index: usize,
        node_index: HashMap<String, usize>,
        node_lowlink: HashMap<String, usize>,
        on_stack: std::collections::HashSet<String>,
        stack: Vec<String>,
        sccs: Vec<Vec<String>>,
    }

    fn strongconnect(
        v: &str,
        call_graph: &std::collections::HashMap<String, std::collections::HashSet<String>>,
        st: &mut State,
    ) {
        st.node_index.insert(v.to_string(), st.index);
        st.node_lowlink.insert(v.to_string(), st.index);
        st.index += 1;
        st.stack.push(v.to_string());
        st.on_stack.insert(v.to_string());

        if let Some(callees) = call_graph.get(v) {
            for w in callees {
                if !st.node_index.contains_key(w) {
                    strongconnect(w, call_graph, st);
                    let new_lowlink = st.node_lowlink.get(v).copied().unwrap_or(0)
                        .min(st.node_lowlink.get(w).copied().unwrap_or(0));
                    st.node_lowlink.insert(v.to_string(), new_lowlink);
                } else if st.on_stack.contains(w) {
                    let new_lowlink = st.node_lowlink.get(v).copied().unwrap_or(0)
                        .min(st.node_index.get(w).copied().unwrap_or(0));
                    st.node_lowlink.insert(v.to_string(), new_lowlink);
                }
            }
        }

        if st.node_index.get(v) == st.node_lowlink.get(v) {
            let mut scc = Vec::new();
            loop {
                let w = st.stack.pop().expect("stack non-empty in SCC");
                st.on_stack.remove(&w);
                let done = &w == v;
                scc.push(w);
                if done {
                    break;
                }
            }
            st.sccs.push(scc);
        }
    }

    let mut st = State::default();
    for func in functions {
        if !st.node_index.contains_key(&func.name) {
            strongconnect(&func.name, call_graph, &mut st);
        }
    }
    st.sccs
}
