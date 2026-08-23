//! Feature-combination coverage fingerprinting.
//!
//! Extracts a set of `{shape}#{operation}` fingerprints from a typed
//! program -- a canonical, order-independent record of which
//! type-shape/codegen-operation combinations the program actually
//! exercises. This exists because every real bug found in the
//! 2026-08-21/22 audit rounds (BUG-216/217/218) was a gap at exactly
//! this granularity: a per-element-type dispatch table (Vec bundle
//! `__free`/`clear`/`set`/`set_mut`, `emit_index_assign`,
//! `vec_element_size_expr`, `mutex_new`'s Copy check, ...) missing an
//! arm for one specific (shape, operation) pair, while every OTHER
//! pair for the same shape (or the same operation on a different
//! shape) worked fine. A fingerprint is precisely "this dispatch
//! table, at this entry" -- the exact unit a missing-arm bug lives at.
//!
//! Used two ways:
//! 1. `vanic check <file> --dump-fingerprints` prints a program's own
//!    fingerprint set (one per line, sorted) -- both for manual
//!    inspection and as the raw input `tools/gen_coverage_db.py` feeds
//!    into the corpus-wide "known good" database.
//! 2. `vanic check <file> --coverage` cross-references that same set
//!    against the baked-in database (built from every corpus example/
//!    test that passes on both backends AND is leak-clean) and prints
//!    a score plus the specific fingerprints with no known coverage.
//!
//! Kept deliberately independent of any specific backend -- this
//! walks `ir::TypedProgram`, the shared typed IR both `backend_c.rs`
//! and `backend_llvm.rs` consume, not either backend's own codegen
//! structures.

use crate::ast::Type;
use crate::ir::{TypedExpr, TypedExprKind, TypedFunction, TypedProgram, TypedStmt};
use std::collections::BTreeSet;
use std::fmt;

/// How deep into a nested generic type (`Vec<Box<Mutex<...>>>`) the
/// canonical shape recurses before collapsing the remainder to `…`.
/// Bounds the fingerprint space -- real bugs in this codebase have
/// never been more than 2-3 levels of nesting deep (`Vec<Box<T>>`,
/// `Mutex<Vec<T>>`), and an unbounded depth would let a single
/// pathological program (or an adversarial one) blow up the
/// fingerprint set arbitrarily.
const MAX_SHAPE_DEPTH: u32 = 4;

/// Collapse a `Type` to its canonical shape string. Scalars/OwnedStr/
/// Str collapse to a small fixed alphabet (the actual WIDTH has never
/// been where a bug lived -- BUG-218's `Mutex<i32>` bug was about the
/// SHAPE "a non-i64 scalar", not i32 specifically, and the fix
/// treats every non-i64 scalar identically). Structs/enums split on
/// `is_copy()` since that's the exact axis BUG-218's soundness fix
/// cares about. Every named container/RAII type recurses through its
/// generic parameters so nesting (the actual bug-bearing dimension)
/// is preserved.
pub fn canonical_shape(ty: &Type, depth: u32) -> String {
    if depth == 0 {
        return "…".to_string();
    }
    let rec = |t: &Type| canonical_shape(t, depth - 1);
    match ty {
        Type::I8
        | Type::I16
        | Type::I32
        | Type::I64
        | Type::U8
        | Type::U16
        | Type::U32
        | Type::U64
        | Type::F32
        | Type::F64
        | Type::Bool => "Scalar".to_string(),
        Type::Str => "Str".to_string(),
        Type::OwnedStr => "OwnedStr".to_string(),
        Type::Array { element, length } => format!("Array<{},{}>", rec(element), length),
        Type::Vec(inner) => format!("Vec<{}>", rec(inner)),
        Type::Vec128(inner) => format!("Vec128<{}>", rec(inner)),
        Type::Vec256(inner) => format!("Vec256<{}>", rec(inner)),
        Type::Vec512(inner) => format!("Vec512<{}>", rec(inner)),
        Type::Tuple(elements) => {
            let parts: Vec<String> = elements.iter().map(rec).collect();
            format!("Tuple<{}>", parts.join(","))
        }
        Type::Struct(name) => {
            if ty.is_copy() {
                "Copy-Struct".to_string()
            } else {
                let _ = name;
                "NonCopy-Struct".to_string()
            }
        }
        Type::Enum(name) => {
            if ty.is_copy() {
                "Copy-Enum".to_string()
            } else {
                let _ = name;
                "NonCopy-Enum".to_string()
            }
        }
        // Monomorphized away before codegen; only seen pre-check.
        // Render structurally so a fingerprint extracted (in theory)
        // before monomorphization still means something.
        Type::Apply { name, args } => {
            let parts: Vec<String> = args.iter().map(rec).collect();
            format!("{}<{}>", name, parts.join(","))
        }
        Type::Param(name) => format!("Param({})", name),
        // A `ref`/`mut ref` wrapper is an annotation, not a real
        // nesting level the way `Vec<T>`/`Box<T>` are -- don't spend
        // depth budget on it, or `&mut Vec<Box<Vec<T>>>` (one extra
        // wrapper on top of 3 real container levels) would collapse
        // its innermost, most bug-relevant level to `…` a step
        // earlier than the un-referenced `Vec<Box<Vec<T>>>` would.
        Type::Ref(inner) => format!("&{}", canonical_shape(inner, depth)),
        Type::RefMut(inner) => format!("&mut {}", canonical_shape(inner, depth)),
        Type::Task => "Task".to_string(),
        Type::TaskR(inner) => format!("TaskR<{}>", rec(inner)),
        Type::Atomic(inner) => format!("Atomic<{}>", rec(inner)),
        Type::Channel(inner, capacity) => format!("Channel<{},{}>", rec(inner), capacity),
        Type::Mutex(inner) => format!("Mutex<{}>", rec(inner)),
        Type::Guard(inner) => format!("Guard<{}>", rec(inner)),
        Type::FnPtr(params, ret) => {
            let parts: Vec<String> = params.iter().map(rec).collect();
            format!("FnPtr({})->{}", parts.join(","), rec(ret))
        }
        Type::Closure(params, ret) => {
            let parts: Vec<String> = params.iter().map(rec).collect();
            format!("Closure({})->{}", parts.join(","), rec(ret))
        }
        Type::Object(iface) => format!("dyn {}", iface),
        Type::Box(inner) => format!("Box<{}>", rec(inner)),
        Type::Condvar => "Condvar".to_string(),
        Type::Barrier => "Barrier".to_string(),
        Type::FileHandle => "FileHandle".to_string(),
        Type::RwLock(inner) => format!("RwLock<{}>", rec(inner)),
        Type::ReadGuard(inner) => format!("ReadGuard<{}>", rec(inner)),
        Type::WriteGuard(inner) => format!("WriteGuard<{}>", rec(inner)),
        Type::Deque(inner) => format!("Deque<{}>", rec(inner)),
        Type::HashSet(inner) => format!("HashSet<{}>", rec(inner)),
        Type::HashMap(k, v) => format!("HashMap<{},{}>", rec(k), rec(v)),
        Type::BTreeSet(inner) => format!("BTreeSet<{}>", rec(inner)),
        Type::BTreeMap(k, v) => format!("BTreeMap<{},{}>", rec(k), rec(v)),
        Type::UnionFind => "UnionFind".to_string(),
        Type::BinaryHeap(inner) => format!("BinaryHeap<{}>", rec(inner)),
        Type::BloomFilter => "BloomFilter".to_string(),
        Type::Bst(inner) => format!("Bst<{}>", rec(inner)),
        Type::Graph => "Graph".to_string(),
        Type::Trie => "Trie".to_string(),
        Type::SkipList => "SkipList".to_string(),
        Type::Ptr(inner) => format!("Ptr<{}>", rec(inner)),
        Type::PtrMut(inner) => format!("PtrMut<{}>", rec(inner)),
        Type::Pool(inner) => format!("Pool<{}>", rec(inner)),
        Type::Handle(inner) => format!("Handle<{}>", rec(inner)),
        Type::Tainted(inner) => format!("Tainted<{}>", rec(inner)),
        Type::BoundedPtr(inner) => format!("BoundedPtr<{}>", rec(inner)),
        Type::Region => "Region".to_string(),
        Type::ArenaRef(inner) => format!("ArenaRef<{}>", rec(inner)),
    }
}

/// Whether a shape is "interesting" for coverage-scoring purposes --
/// involves at least one RAII/affine/heap-owning component somewhere
/// in the nest. Every real bug this session found lived on this axis
/// (a `Vec<i64>`/`Copy-Struct` combination has never been the site of
/// one of these bugs); scoring restricts to this subset so the score
/// isn't diluted by the overwhelming majority of scalar-only code a
/// typical program contains. Extraction itself does NOT filter --
/// `--dump-fingerprints` prints everything, so the DB-generation
/// script (which also wants the non-interesting shapes, to positively
/// confirm they're fine) sees the full set; only `--coverage`
/// scoring applies this filter.
pub fn shape_is_interesting(shape: &str) -> bool {
    const LEAF_ONLY: &[&str] = &["Scalar", "Str", "Copy-Struct", "Copy-Enum"];
    // A shape built ONLY from Copy leaves (no container/wrapper name
    // at all) is uninteresting. Cheapest correct test: does the
    // shape string exactly match one of the known Copy-leaf spellings?
    // Anything else has a container/wrapper name (even a Copy one
    // like `Array<Scalar,4>`) wrapping something, which is exactly
    // where the historical bugs lived (an Array-of-non-Copy leaked
    // the same way a Vec-of-non-Copy did).
    !LEAF_ONLY.contains(&shape)
}

/// One `{shape}#{operation}` fingerprint. `Ord`/`Hash` so a whole
/// program's fingerprints collapse into a `BTreeSet` for free (same
/// shape+operation touched at 50 call sites is one fingerprint, not
/// 50 -- coverage is about "has this combination ever been
/// exercised", not "how often").
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Fingerprint {
    pub shape: String,
    pub operation: String,
}

impl Fingerprint {
    pub fn new(shape: String, operation: impl Into<String>) -> Self {
        Fingerprint { shape, operation: operation.into() }
    }
}

impl fmt::Display for Fingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}#{}", self.shape, self.operation)
    }
}

/// Extract every fingerprint touched anywhere in a typed program:
/// every function's params/return type (as `bind`), and every
/// statement/expression in every function body.
///
/// `Call` nodes are only recorded as an operation-tagged fingerprint
/// when the callee is a BUILTIN, not one of the program's own
/// user-defined functions -- coverage is specifically about the
/// COMPILER's per-element-type codegen dispatch tables, which only
/// ever fire for builtins (`push`/`guard_get`/`mutex_new`/...); a
/// call to a user's own `overwrite(...)` helper isn't a "feature
/// combination" in that sense, it's just a function call, so tagging
/// it with the user's own function name as an "operation" would
/// pollute the fingerprint set with names that can never appear in
/// the DB (every corpus program's own helper names are different).
pub fn extract_program_fingerprints(program: &TypedProgram) -> BTreeSet<Fingerprint> {
    let user_fns: BTreeSet<String> = program.functions.iter().map(|f| f.name.clone()).collect();
    let mut out = BTreeSet::new();
    for f in &program.functions {
        extract_function_fingerprints(f, &user_fns, &mut out);
    }
    out
}

fn extract_function_fingerprints(
    f: &TypedFunction,
    user_fns: &BTreeSet<String>,
    out: &mut BTreeSet<Fingerprint>,
) {
    for param in &f.params {
        out.insert(Fingerprint::new(
            canonical_shape(&param.ty, MAX_SHAPE_DEPTH),
            "param",
        ));
    }
    out.insert(Fingerprint::new(
        canonical_shape(&f.return_type, MAX_SHAPE_DEPTH),
        "return",
    ));
    for stmt in &f.body {
        extract_stmt_fingerprints(stmt, user_fns, out);
    }
}

fn extract_stmt_fingerprints(
    stmt: &TypedStmt,
    user_fns: &BTreeSet<String>,
    out: &mut BTreeSet<Fingerprint>,
) {
    match stmt {
        TypedStmt::Let { ty, expr, .. } => {
            out.insert(Fingerprint::new(canonical_shape(ty, MAX_SHAPE_DEPTH), "bind"));
            extract_expr_fingerprints(expr, user_fns, out);
        }
        TypedStmt::Reassign { ty, expr, .. } => {
            out.insert(Fingerprint::new(canonical_shape(ty, MAX_SHAPE_DEPTH), "reassign"));
            extract_expr_fingerprints(expr, user_fns, out);
        }
        TypedStmt::Drop { ty, .. } => {
            out.insert(Fingerprint::new(canonical_shape(ty, MAX_SHAPE_DEPTH), "drop"));
        }
        TypedStmt::Discard { expr }
        | TypedStmt::Return { expr }
        | TypedStmt::Assert { expr, .. }
        | TypedStmt::Prove { expr } => extract_expr_fingerprints(expr, user_fns, out),
        TypedStmt::Print { items } | TypedStmt::EPrint { items } => {
            for item in items {
                if let crate::ir::TypedPrintItem::Expr(e, _) = item {
                    extract_expr_fingerprints(e, user_fns, out);
                }
            }
        }
        TypedStmt::If { cond, then_body, else_body } => {
            extract_expr_fingerprints(cond, user_fns, out);
            for s in then_body {
                extract_stmt_fingerprints(s, user_fns, out);
            }
            for s in else_body {
                extract_stmt_fingerprints(s, user_fns, out);
            }
        }
        TypedStmt::While { cond, body, .. } => {
            extract_expr_fingerprints(cond, user_fns, out);
            for s in body {
                extract_stmt_fingerprints(s, user_fns, out);
            }
        }
        TypedStmt::Break { .. } | TypedStmt::Continue { .. } => {}
        TypedStmt::IndexAssign { base_ty, index, field_path, value, .. } => {
            let op = if field_path.is_empty() { "index_assign" } else { "field_path_write" };
            out.insert(Fingerprint::new(canonical_shape(base_ty, MAX_SHAPE_DEPTH), op));
            extract_expr_fingerprints(index, user_fns, out);
            extract_expr_fingerprints(value, user_fns, out);
        }
        TypedStmt::FieldAssign { object, value, .. } => {
            out.insert(Fingerprint::new(
                canonical_shape(&object.ty, MAX_SHAPE_DEPTH),
                "field_assign",
            ));
            extract_expr_fingerprints(object, user_fns, out);
            extract_expr_fingerprints(value, user_fns, out);
        }
        TypedStmt::For { start, end, body, .. } => {
            extract_expr_fingerprints(start, user_fns, out);
            extract_expr_fingerprints(end, user_fns, out);
            for s in body {
                extract_stmt_fingerprints(s, user_fns, out);
            }
        }
        TypedStmt::ForIter { collection_ty, consumes, body, .. } => {
            let op = if *consumes { "for_iter_consume" } else { "for_iter_borrow" };
            out.insert(Fingerprint::new(canonical_shape(collection_ty, MAX_SHAPE_DEPTH), op));
            for s in body {
                extract_stmt_fingerprints(s, user_fns, out);
            }
        }
        TypedStmt::TaskSpawn { body, captures, .. } => {
            for (_, ty) in captures {
                out.insert(Fingerprint::new(canonical_shape(ty, MAX_SHAPE_DEPTH), "task_capture"));
            }
            for s in body {
                extract_stmt_fingerprints(s, user_fns, out);
            }
        }
        TypedStmt::TaskJoin { .. }
        | TypedStmt::Detach { .. }
        | TypedStmt::Cancel { .. }
        | TypedStmt::ForIterShallowFree { .. } => {}
        TypedStmt::UnsafeBlock { body, .. } => {
            for s in body {
                extract_stmt_fingerprints(s, user_fns, out);
            }
        }
    }
}

fn extract_expr_fingerprints(
    expr: &TypedExpr,
    user_fns: &BTreeSet<String>,
    out: &mut BTreeSet<Fingerprint>,
) {
    match &expr.kind {
        TypedExprKind::Int(_)
        | TypedExprKind::Float(_)
        | TypedExprKind::Bool(_)
        | TypedExprKind::Str(_)
        | TypedExprKind::Var(_)
        | TypedExprKind::Ref { .. }
        | TypedExprKind::RefMut { .. }
        | TypedExprKind::FnRef { .. } => {}
        TypedExprKind::Unary { expr, .. } | TypedExprKind::Cast { expr, .. } => {
            extract_expr_fingerprints(expr, user_fns, out);
        }
        TypedExprKind::Binary { left, right, .. } => {
            extract_expr_fingerprints(left, user_fns, out);
            extract_expr_fingerprints(right, user_fns, out);
        }
        TypedExprKind::Call { name, args, .. } => {
            // The (shape, operation) pair for a BUILTIN call only --
            // see this function's parent doc comment for why
            // user-defined function names are excluded. Prefer the
            // call's OWN result type (covers constructors --
            // `mutex_new`/`push`/`vec_fill`/`union_find_new`/... all
            // return the container type directly), falling back to
            // the first argument's type (unwrapping one layer of
            // Ref/RefMut, since accessor/mutator builtins take the
            // container BY REFERENCE -- `guard_get(ref g)`,
            // `push(v, x)` NOT through a ref but still arg[0]) when
            // the return type is a plain scalar/bool (e.g. `len`,
            // `guard_get` on a Copy T, `mutex_lock` returns Guard<T>
            // so that case is already covered by the return-type
            // path). Recording via BOTH when they differ and are
            // both "interesting" costs nothing (BTreeSet dedups) and
            // avoids picking the wrong one.
            if !user_fns.contains(name) {
                let ret_shape = canonical_shape(&expr.ty, MAX_SHAPE_DEPTH);
                if shape_is_interesting(&ret_shape) {
                    out.insert(Fingerprint::new(ret_shape, name.clone()));
                }
                if let Some(first) = args.first() {
                    let arg_ty = unwrap_ref(&first.ty);
                    let arg_shape = canonical_shape(arg_ty, MAX_SHAPE_DEPTH);
                    if shape_is_interesting(&arg_shape) {
                        out.insert(Fingerprint::new(arg_shape, name.clone()));
                    }
                }
            }
            for a in args {
                extract_expr_fingerprints(a, user_fns, out);
            }
        }
        TypedExprKind::ArrayLit { elements } => {
            for e in elements {
                extract_expr_fingerprints(e, user_fns, out);
            }
        }
        TypedExprKind::Index { array, index, .. } => {
            out.insert(Fingerprint::new(
                canonical_shape(&array.ty, MAX_SHAPE_DEPTH),
                "index_read",
            ));
            extract_expr_fingerprints(array, user_fns, out);
            extract_expr_fingerprints(index, user_fns, out);
        }
        TypedExprKind::Len { array, .. } => extract_expr_fingerprints(array, user_fns, out),
        TypedExprKind::RefField { object_ty, .. } => {
            out.insert(Fingerprint::new(
                canonical_shape(object_ty, MAX_SHAPE_DEPTH),
                "ref_field",
            ));
        }
        TypedExprKind::RefMutField { object_ty, .. } => {
            out.insert(Fingerprint::new(
                canonical_shape(object_ty, MAX_SHAPE_DEPTH),
                "ref_mut_field",
            ));
        }
        TypedExprKind::RefMutIndex { vec_ty, index, .. } => {
            out.insert(Fingerprint::new(
                canonical_shape(vec_ty, MAX_SHAPE_DEPTH),
                "ref_mut_index",
            ));
            extract_expr_fingerprints(index, user_fns, out);
        }
        TypedExprKind::CallIndirect { callee, args } => {
            extract_expr_fingerprints(callee, user_fns, out);
            for a in args {
                extract_expr_fingerprints(a, user_fns, out);
            }
        }
        TypedExprKind::Tuple { elements } => {
            for e in elements {
                extract_expr_fingerprints(e, user_fns, out);
            }
        }
        TypedExprKind::TupleAccess { tuple, .. } => extract_expr_fingerprints(tuple, user_fns, out),
        TypedExprKind::StructLit { fields, .. } => {
            for (_, e) in fields {
                extract_expr_fingerprints(e, user_fns, out);
            }
        }
        TypedExprKind::FieldAccess { object, .. } => {
            extract_expr_fingerprints(object, user_fns, out)
        }
        TypedExprKind::EnumVariant { .. } => {}
        TypedExprKind::EnumVariantWithPayload { payload, .. } => {
            extract_expr_fingerprints(payload, user_fns, out);
        }
        TypedExprKind::IfExpr { cond, then_value, else_value } => {
            extract_expr_fingerprints(cond, user_fns, out);
            extract_expr_fingerprints(then_value, user_fns, out);
            extract_expr_fingerprints(else_value, user_fns, out);
        }
        TypedExprKind::Match { scrutinee, arms } => {
            extract_expr_fingerprints(scrutinee, user_fns, out);
            for arm in arms {
                extract_expr_fingerprints(&arm.body, user_fns, out);
            }
        }
        TypedExprKind::Block { stmts, tail } => {
            for s in stmts {
                extract_stmt_fingerprints(s, user_fns, out);
            }
            extract_expr_fingerprints(tail, user_fns, out);
        }
        TypedExprKind::Forall { .. } => {}
        TypedExprKind::TaskSpawnCall { args, .. } => {
            for a in args {
                extract_expr_fingerprints(a, user_fns, out);
            }
        }
        TypedExprKind::TaskJoinExpr { .. } => {}
        TypedExprKind::DynDispatch { receiver, args, .. } => {
            extract_expr_fingerprints(receiver, user_fns, out);
            for a in args {
                extract_expr_fingerprints(a, user_fns, out);
            }
        }
        TypedExprKind::DynCoerce { value, .. } => extract_expr_fingerprints(value, user_fns, out),
    }
}

fn unwrap_ref(ty: &Type) -> &Type {
    match ty {
        Type::Ref(inner) | Type::RefMut(inner) => inner.as_ref(),
        other => other,
    }
}

// ---------------------------------------------------------------------
// Baked-in coverage database + scoring
// ---------------------------------------------------------------------

/// The corpus-wide "known good" fingerprint database, generated by
/// `tools/gen_coverage_db.py` and baked into the binary at compile
/// time -- `vanic` never fetches or generates this at check-time, so
/// `vanic check --coverage` works fully offline. See that script's
/// doc comment for exactly what "known good" means (accepted by the
/// checker AND leak/bug-clean per `tools/leak_sweep.py`'s ASan sweep).
const COVERAGE_DB_JSON: &str = include_str!("../coverage_fingerprints.json");

struct CoverageDb {
    generated_utc: String,
    verified_clean_files: usize,
    known: BTreeSet<String>,
}

/// Hand-parsed via `serde_json::Value` (rather than a `#[derive(Deserialize)]`
/// struct) since this crate depends on plain `serde`/`serde_json`
/// without the `derive` feature enabled.
fn coverage_db() -> &'static CoverageDb {
    static DB: std::sync::OnceLock<CoverageDb> = std::sync::OnceLock::new();
    DB.get_or_init(|| {
        let empty = || CoverageDb {
            generated_utc: String::new(),
            verified_clean_files: 0,
            known: BTreeSet::new(),
        };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(COVERAGE_DB_JSON) else {
            return empty();
        };
        let generated_utc =
            v.get("generated_utc").and_then(|x| x.as_str()).unwrap_or("").to_string();
        let verified_clean_files =
            v.get("verified_clean_files").and_then(|x| x.as_u64()).unwrap_or(0) as usize;
        let known = v
            .get("fingerprints")
            .and_then(|x| x.as_array())
            .map(|arr| arr.iter().filter_map(|e| e.as_str().map(str::to_string)).collect())
            .unwrap_or_default();
        CoverageDb { generated_utc, verified_clean_files, known }
    })
}

/// A program's coverage score against the baked-in database.
///
/// Scoring is restricted to "interesting" fingerprints
/// (`shape_is_interesting`) -- a scalar/Copy-struct/Copy-enum shape
/// touching `bind`/`return`/etc. is never where a missing-dispatch-
/// arm bug lives (there's no per-element-type dispatch table for a
/// plain `i64`), so scoring every fingerprint would dilute the score
/// with combinations that were never at risk and make a 100%-affine
/// program look artificially worse than an equally-tested all-scalar
/// one.
pub struct CoverageReport {
    /// 0..=100. 100 when every interesting fingerprint the program
    /// touches is present in the database (including the vacuous
    /// case of a program with no interesting fingerprints at all).
    pub score: u32,
    pub total_interesting: usize,
    pub known_count: usize,
    /// Interesting fingerprints this program touches that the
    /// database has no record of -- each is a real gap: either a
    /// legitimately new feature combination worth adding regression/
    /// library test coverage for, or (less likely, but possible) a
    /// combination the corpus happens not to exercise despite being
    /// fine. Either way, `vanic` cannot itself tell those apart --
    /// only a human (or a new corpus test) can.
    pub unknown: Vec<Fingerprint>,
    pub db_generated_utc: String,
    pub db_verified_clean_files: usize,
}

pub fn score_program(program: &TypedProgram) -> CoverageReport {
    let fps = extract_program_fingerprints(program);
    let db = coverage_db();
    let interesting: Vec<Fingerprint> =
        fps.into_iter().filter(|f| shape_is_interesting(&f.shape)).collect();
    let total = interesting.len();
    let (known, unknown): (Vec<Fingerprint>, Vec<Fingerprint>) =
        interesting.into_iter().partition(|f| db.known.contains(&f.to_string()));
    let known_count = known.len();
    let score = if total == 0 { 100 } else { ((known_count * 100) / total) as u32 };
    CoverageReport {
        score,
        total_interesting: total,
        known_count,
        unknown,
        db_generated_utc: db.generated_utc.clone(),
        db_verified_clean_files: db.verified_clean_files,
    }
}

/// Draft (never files) a GitHub issue markdown body reporting the
/// untested feature combinations in `report`, for the exact source
/// file `source_path` (whose full contents are embedded in the draft
/// as the reproduction case -- reasonable ONLY because this file
/// stays local until the user themselves decides to run the printed
/// `gh issue create` command; `vanic` never uploads or transmits
/// anything on its own). Returns `None` if there's nothing to report
/// (`report.unknown` is empty).
pub fn draft_coverage_issue(
    report: &CoverageReport,
    source_path: &std::path::Path,
    source: &str,
    vanic_version: &str,
) -> Option<String> {
    if report.unknown.is_empty() {
        return None;
    }
    let mut body = String::new();
    body.push_str(&format!(
        "## Untested feature combination{} found by `vanic check --coverage`\n\n",
        if report.unknown.len() == 1 { "" } else { "s" }
    ));
    body.push_str(&format!(
        "Coverage score: **{}/100** ({}/{} known combinations).\n\n",
        report.score, report.known_count, report.total_interesting
    ));
    body.push_str(&format!(
        "Coverage database: generated `{}`, from {} verified-clean example file(s).\n\n",
        report.db_generated_utc, report.db_verified_clean_files
    ));
    body.push_str("### Fingerprints with no known regression/library test coverage\n\n");
    for fp in &report.unknown {
        body.push_str(&format!("- `{}`\n", fp));
    }
    body.push_str(&format!(
        "\n### Reproduction (`{}`, compiled with {})\n\n```vani\n{}\n```\n",
        source_path.display(),
        vanic_version,
        source.trim_end(),
    ));
    body.push_str(
        "\n---\n*Drafted locally by `vanic check --emit-coverage-issue`. Nothing has been \
         sent anywhere -- this file is only submitted if you run the `gh issue create` \
         command it prints, or paste it in yourself.*\n",
    );
    Some(body)
}

// ---------------------------------------------------------------------
// Coverage GAP enumeration (`vanic coverage-gaps`)
// ---------------------------------------------------------------------
//
// `--coverage` needs a candidate program to score -- it can only ever
// confirm a gap you've already written a repro for. This is the
// inverse: mine the (container family, operation) vocabulary directly
// out of the baked-in database itself (e.g. seeing `Vec<Scalar>#push`
// tells us the family "Vec" has an operation "push"), then cross each
// family against every "filler" element shape ever seen anywhere in
// the database (the 7 hardcoded leaf shapes `canonical_shape` can
// produce, plus every non-parameterized "atomic" shape like `Graph`/
// `UnionFind`/`Barrier`/...) to build hypothesis fingerprints, and
// report the ones the database has NO record of. This is a mechanical
// version of the exact manual sweep that found BUG-216/217/218 --
// "this operation is proven safe for element type A, has it ever been
// exercised for element type B?"
//
// Deliberately scoped to depth-1, single-type-param families only
// (`Vec<Scalar>`, not `Vec<Box<Vec<Scalar>>>`; not `HashMap<K,V>` or
// `Array<T,N>`, which have more than one type/const parameter) --
// going deeper multiplies the candidate space combinatorially for
// diminishing real-bug-finding value, since every historical bug this
// approach was modeled on (BUG-216/217/218) was exactly one level of
// nesting.
//
// This is a HEURISTIC generator, not a proof of anything: many
// candidate fingerprints will be rejected by the checker outright
// (e.g. a builtin that only ever accepts a Copy element type), and
// that's expected -- the list is a set of hypotheses worth a human
// (or an automated fuzzer) spending a few minutes writing a real
// repro for, exactly like trying `Vec<Graph>` was worth trying.

/// The leaf shapes `canonical_shape` can produce that are NEVER
/// themselves recorded as a fingerprint in the database (filtered out
/// by `shape_is_interesting` at extraction time, since a plain scalar
/// touching `bind`/`return` was never where a dispatch-table bug
/// lived) -- so unlike the "atomic" shapes below, these have to be
/// hardcoded rather than mined.
const BASE_FILLERS: &[&str] = &[
    "Scalar",
    "Str",
    "OwnedStr",
    "Copy-Struct",
    "NonCopy-Struct",
    "Copy-Enum",
    "NonCopy-Enum",
];

/// If `shape` is exactly `Family<Inner>` for a single, simple
/// (non-nested, comma-free) `Inner`, return `(Family, Inner)`.
/// Deliberately excludes ref-wrapped (`&...`), depth-truncated (`…`),
/// and multi-parameter (`Array<T,N>`, `HashMap<K,V>`, `Channel<T,N>`)
/// shapes -- see this section's doc comment for why.
fn parse_family_leaf(shape: &str) -> Option<(&str, &str)> {
    if shape.starts_with('&') || shape.contains('…') {
        return None;
    }
    let open = shape.find('<')?;
    if !shape.ends_with('>') {
        return None;
    }
    let family = &shape[..open];
    let inner = &shape[open + 1..shape.len() - 1];
    if family.is_empty() || !family.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }
    if inner.is_empty() || inner.contains(['<', '>', ',']) {
        return None;
    }
    Some((family, inner))
}

/// A "filler"-eligible atomic shape: a bare identifier (optionally
/// hyphenated, e.g. `Copy-Struct`) with no generic parameters --
/// either one of the 7 `BASE_FILLERS`, or a first-class type like
/// `Graph`/`UnionFind`/`Barrier` that showed up as its own fingerprint
/// shape in the database.
fn is_atomic_filler_shape(shape: &str) -> bool {
    !shape.is_empty()
        && !shape.starts_with('&')
        && !shape.contains('…')
        && shape.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

pub fn enumerate_coverage_gaps() -> Vec<Fingerprint> {
    let db = coverage_db();
    let mut ops_by_family: std::collections::BTreeMap<String, BTreeSet<String>> =
        std::collections::BTreeMap::new();
    let mut fillers: BTreeSet<String> = BASE_FILLERS.iter().map(|s| s.to_string()).collect();

    for fp in &db.known {
        let Some((shape, op)) = fp.rsplit_once('#') else {
            continue;
        };
        if let Some((family, _inner)) = parse_family_leaf(shape) {
            ops_by_family
                .entry(family.to_string())
                .or_default()
                .insert(op.to_string());
        } else if is_atomic_filler_shape(shape) {
            fillers.insert(shape.to_string());
        }
    }

    let mut gaps: BTreeSet<Fingerprint> = BTreeSet::new();
    for (family, ops) in &ops_by_family {
        for op in ops {
            for filler in &fillers {
                let candidate_shape = format!("{family}<{filler}>");
                let candidate = format!("{candidate_shape}#{op}");
                if !db.known.contains(&candidate) {
                    gaps.insert(Fingerprint::new(candidate_shape, op.clone()));
                }
            }
        }
    }
    gaps.into_iter().collect()
}
