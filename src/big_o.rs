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
//!   * Unable-to-prove cases → `O(?)` with the unprovable
//!     construct flagged.
//!
//! Out of scope (future):
//!   * Inferring "n" from a specific parameter (currently the
//!     bound is just "input size" without naming the parameter).
//!   * Cross-fn analysis (a fn that calls another fn picks up
//!     the callee's complexity — v1 ignores call-site costs
//!     beyond builtins).
//!   * Bounds derived from `requires`/`ensures` clauses (SMT
//!     could prove tighter bounds; v1 only uses syntactic
//!     evidence).

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
pub fn analyze_function(func: &TypedFunction) -> BigO {
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

    // Pass 2: max loop nesting depth + tally of superlinear
    // builtins per level.
    let summary = walk_body(&func.body, 0);

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
    let mut s = Summary::default();
    s.max_depth = depth;
    for stmt in body {
        let inner = walk_stmt(stmt, depth);
        merge_summary(&mut s, &inner);
    }
    s
}

fn walk_stmt(stmt: &TypedStmt, depth: u32) -> Summary {
    let mut s = Summary::default();
    s.max_depth = depth;
    match stmt {
        TypedStmt::Let { expr, .. } => merge_with_expr(&mut s, expr, depth),
        TypedStmt::Reassign { expr, .. } => merge_with_expr(&mut s, expr, depth),
        TypedStmt::Discard { expr, .. } => merge_with_expr(&mut s, expr, depth),
        TypedStmt::Return { expr, .. } => merge_with_expr(&mut s, expr, depth),
        TypedStmt::FieldAssign { value, .. } => merge_with_expr(&mut s, value, depth),
        TypedStmt::IndexAssign { value, .. } => merge_with_expr(&mut s, value, depth),
        TypedStmt::Assert { expr, .. } | TypedStmt::Prove { expr } => {
            merge_with_expr(&mut s, expr, depth)
        }
        TypedStmt::While { body, .. } => {
            let inner = walk_body(body, depth + 1);
            merge_summary(&mut s, &inner);
        }
        TypedStmt::For { body, .. } | TypedStmt::ForIter { body, .. } => {
            let inner = walk_body(body, depth + 1);
            merge_summary(&mut s, &inner);
        }
        TypedStmt::If { then_body, else_body, .. } => {
            let then_s = walk_body(then_body, depth);
            let else_s = walk_body(else_body, depth);
            merge_summary(&mut s, &then_s);
            merge_summary(&mut s, &else_s);
        }
        TypedStmt::TaskSpawn { body, .. } | TypedStmt::UnsafeBlock { body, .. } => {
            let inner = walk_body(body, depth);
            merge_summary(&mut s, &inner);
        }
        _ => {}
    }
    s
}

fn merge_with_expr(s: &mut Summary, expr: &TypedExpr, depth: u32) {
    visit_calls_in_expr(expr, &mut |name| {
        if is_superlinear_builtin_sort(name) {
            if depth > 0 {
                s.has_sort_inside_loop = true;
            } else {
                s.has_sort_at_top = true;
            }
        } else if is_logn_builtin(name) {
            if depth > 0 {
                s.has_logn_inside_loop = true;
            } else {
                s.has_logn_at_top = true;
            }
        } else if is_linear_builtin(name) {
            if depth == 0 {
                s.has_n_at_top = true;
            }
        }
    });
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
            // Single loop. The body's content multiplies the
            // n iterations:
            //   - sort inside: O(n × n log n) = O(n² log n)
            //   - binary_search inside: O(n log n)
            //   - everything else: O(n)
            if s.has_sort_inside_loop {
                BigO::PolynomialLog(2)
            } else if s.has_logn_inside_loop {
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
pub fn annotate_program(program: &TypedProgram, mode: BigOMode) -> Vec<(String, BigO)> {
    let mut out = Vec::new();
    for func in &program.functions {
        let complexity = analyze_function(func);
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
