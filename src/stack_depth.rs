//! T1.3 of the safety-standard alignment arc — per-function
//! frame-size estimates + call-graph max-depth reporting.
//!
//! For ASIL-D / DO-178C / IEC 62304 certification, the verifier
//! must produce a bound on worst-case stack usage. This module
//! walks the typed program, estimates a frame size per
//! function (sum of local-binding sizes + ~32-byte prologue
//! conservative overhead), then traces the call graph from
//! each entry-point to compute the max stack depth reachable.
//!
//! Recursion handling:
//! - Direct recursion with `#[bounded(N)]`: depth includes
//!   `(N+1)` copies of the frame. (N is the runtime guard
//!   threshold; allowing N+1 covers the worst case where the
//!   counter just barely doesn't trip.)
//! - Recursion without `#[bounded]`: unbounded — the function
//!   is flagged in the report and (when `--max` is set) makes
//!   the build fail.
//! - Mutual recursion via cycle detection: same treatment as
//!   direct unbounded.
//!
//! V1 is a conservative estimator, not an exact analysis. The
//! frame size is computed via a simple type → byte-size table
//! that matches the C / LLVM lowering on 64-bit targets.
//! Reg-allocator hoisting and spill packing are ignored —
//! over-estimation is safe.
//!
//! Output formats (`--format`):
//! - **text** (default): human-readable report with per-fn frame
//!   sizes + per-entry call-chain breakdown.
//! - **json**: structured records for CI integration.
//! - **csv**: tabular for spreadsheet review.
//!
//! `--max=N`: fail the run when any entry-point exceeds N bytes.

use crate::ast::Type;
use crate::ir::{TypedProgram, TypedStmt, TypedExpr, TypedExprKind};
use std::collections::{HashMap, HashSet};

/// Conservative caller-saved + return-addr + alignment
/// overhead per stack frame. ~32 bytes covers x86-64 SysV /
/// ARM AAPCS reasonably; over-estimate is safe.
const FRAME_OVERHEAD_BYTES: u64 = 32;

/// Per-function summary.
#[derive(Clone, Debug)]
pub struct FrameReport {
    pub name: String,
    pub local_bytes: u64,
    pub frame_bytes: u64, // local_bytes + FRAME_OVERHEAD_BYTES
    pub bounded_recursion: Option<u64>, // from #[bounded(N)]
    pub direct_recursion: bool,
    pub callees: Vec<String>,
}

/// Per-entry-point summary.
#[derive(Clone, Debug)]
pub struct EntryReport {
    pub name: String,
    /// Total stack depth in bytes. `None` indicates unbounded
    /// (recursion without #[bounded(N)] or via a cycle).
    pub max_depth_bytes: Option<u64>,
    /// Call chain that produces the max depth. Last element
    /// is the deepest leaf.
    pub chain: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct StackReport {
    pub frames: Vec<FrameReport>,
    pub entries: Vec<EntryReport>,
}

/// Walk the program, build per-function frame summaries +
/// per-entry-point depth bounds. Entry-points are every
/// non-extern, non-recursive top-level function (or every
/// top-level function if `entry_filter` is None).
pub fn compute_stack_depths(program: &TypedProgram, entry_filter: Option<&str>) -> StackReport {
    let mut frames: Vec<FrameReport> = Vec::new();
    for f in &program.functions {
        if f.is_extern {
            continue;
        }
        let mut local_bytes = 0u64;
        for p in &f.params {
            local_bytes += type_size(&p.ty);
        }
        for s in &f.body {
            local_bytes += stmt_local_bytes(s);
        }
        let frame_bytes = local_bytes + FRAME_OVERHEAD_BYTES;
        let mut callees: Vec<String> = Vec::new();
        for s in &f.body {
            stmt_callees(s, &mut callees);
        }
        callees.sort();
        callees.dedup();
        let direct_recursion = callees.iter().any(|c| c == &f.name);
        frames.push(FrameReport {
            name: f.name.clone(),
            local_bytes,
            frame_bytes,
            bounded_recursion: f.recursion_bound,
            direct_recursion,
            callees,
        });
    }
    let mut entries: Vec<EntryReport> = Vec::new();
    let frame_map: HashMap<String, FrameReport> = frames
        .iter()
        .map(|f| (f.name.clone(), f.clone()))
        .collect();
    for f in &frames {
        if let Some(filter) = entry_filter {
            if filter != f.name {
                continue;
            }
        }
        let mut visiting: HashSet<String> = HashSet::new();
        let mut chain: Vec<String> = Vec::new();
        let result = traverse_depth(&f.name, &frame_map, &mut visiting, &mut chain);
        entries.push(EntryReport {
            name: f.name.clone(),
            max_depth_bytes: result.depth,
            chain: result.chain,
        });
    }
    StackReport { frames, entries }
}

struct TraverseResult {
    depth: Option<u64>,
    chain: Vec<String>,
}

fn traverse_depth(
    name: &str,
    frames: &HashMap<String, FrameReport>,
    visiting: &mut HashSet<String>,
    chain: &mut Vec<String>,
) -> TraverseResult {
    let Some(frame) = frames.get(name) else {
        return TraverseResult { depth: Some(0), chain: vec![] };
    };
    if visiting.contains(name) {
        // Cycle — recursion. Unbounded unless a #[bounded(N)]
        // entry on the current frame is going to consume it.
        if let Some(n) = frame.bounded_recursion {
            // Cap at N additional frames.
            return TraverseResult {
                depth: Some(frame.frame_bytes.saturating_mul(n + 1)),
                chain: vec![name.to_string()],
            };
        }
        return TraverseResult { depth: None, chain: vec![format!("[recursion in {}]", name)] };
    }
    visiting.insert(name.to_string());
    chain.push(name.to_string());
    let mut best: Option<u64> = Some(0);
    let mut best_chain: Vec<String> = vec![name.to_string()];
    for callee in &frame.callees {
        if callee == name {
            // Self-call — handled via bounded_recursion below
            // OR flagged unbounded.
            if let Some(n) = frame.bounded_recursion {
                let extra = frame.frame_bytes.saturating_mul(n);
                if best.is_some() && extra > best.unwrap_or(0) {
                    best = Some(extra);
                    best_chain = vec![name.to_string(), format!("[bounded {} self-recursion]", n)];
                }
            } else {
                best = None; // unbounded
                best_chain = vec![name.to_string(), format!("[unbounded self-recursion]")];
            }
            continue;
        }
        let sub = traverse_depth(callee, frames, visiting, chain);
        let combined = match (best, sub.depth) {
            (Some(a), Some(b)) => Some(a.max(b)),
            _ => None,
        };
        if combined.is_none() || sub.depth.is_none() {
            best = None;
            best_chain = {
                let mut c = vec![name.to_string()];
                c.extend(sub.chain.clone());
                c
            };
        } else if let Some(b) = sub.depth {
            if b > best.unwrap_or(0) {
                best = Some(b);
                best_chain = {
                    let mut c = vec![name.to_string()];
                    c.extend(sub.chain.clone());
                    c
                };
            }
        }
    }
    visiting.remove(name);
    chain.pop();
    let result = TraverseResult {
        depth: best.map(|b| b + frame.frame_bytes),
        chain: best_chain,
    };
    result
}

fn type_size(ty: &Type) -> u64 {
    use Type::*;
    match ty {
        I8 | U8 | Bool => 1,
        I16 | U16 => 2,
        I32 | U32 | F32 => 4,
        I64 | U64 | F64 => 8,
        Str | OwnedStr => 8, // char*
        Ref(_) | RefMut(_) => 8,
        Ptr(_) | PtrMut(_) | ArenaRef(_) => 8,
        Handle(_) => 8,
        FnPtr(_, _) => 8,
        Closure(_, _) => 16, // Arc 5c: env-ptr + call-ptr
        Task => 16,
        Condvar => 8,
        Atomic(inner) | Mutex(inner) | Guard(inner) => type_size(inner).max(8),
        Channel(_, _) => 32,
        Deque(_) | HashSet(_) | HashMap(_, _) | BTreeSet(_) | BTreeMap(_, _) => 32,
        UnionFind | BinaryHeap(_) | BloomFilter | Bst(_) | Graph | Trie | SkipList => 48,
        Pool(_) => 48,
        Region => 24,
        BoundedPtr(_) => 24,
        Object(_) => 16,
        Vec(_) => 24, // {data, len, cap}
        Array { element, length } => type_size(element).saturating_mul(*length),
        Tuple(elements) => elements.iter().map(type_size).sum::<u64>().max(8),
        Struct(_) => 32, // estimate; structs lower to nested struct types
        Enum(_) => 16, // 4-byte tag + max-payload pad; estimate
        Apply { .. } | Param(_) => 16,
        Tainted(inner) => type_size(inner),
    }
}

fn stmt_local_bytes(stmt: &TypedStmt) -> u64 {
    match stmt {
        TypedStmt::Let { ty, .. } | TypedStmt::Reassign { ty, .. } => type_size(ty),
        TypedStmt::If { then_body, else_body, .. } => {
            // Branches share the stack frame; take the max.
            let t: u64 = then_body.iter().map(stmt_local_bytes).sum();
            let e: u64 = else_body.iter().map(stmt_local_bytes).sum();
            t.max(e)
        }
        TypedStmt::While { body, .. }
        | TypedStmt::For { body, .. }
        | TypedStmt::ForIter { body, .. }
        | TypedStmt::TaskSpawn { body, .. }
        | TypedStmt::UnsafeBlock { body, .. } => {
            body.iter().map(stmt_local_bytes).sum()
        }
        _ => 0,
    }
}

pub(crate) fn stmt_callees(stmt: &TypedStmt, out: &mut Vec<String>) {
    match stmt {
        TypedStmt::Let { expr, .. }
        | TypedStmt::Reassign { expr, .. }
        | TypedStmt::Return { expr }
        | TypedStmt::Assert { expr, .. }
        | TypedStmt::Prove { expr }
        | TypedStmt::Discard { expr } => expr_callees(expr, out),
        TypedStmt::IndexAssign { value, .. } | TypedStmt::FieldAssign { value, .. } => {
            expr_callees(value, out);
        }
        TypedStmt::Print { items } => {
            for it in items {
                if let crate::ir::TypedPrintItem::Expr(e) = it {
                    expr_callees(e, out);
                }
            }
        }
        TypedStmt::If { cond, then_body, else_body } => {
            expr_callees(cond, out);
            for s in then_body { stmt_callees(s, out); }
            for s in else_body { stmt_callees(s, out); }
        }
        TypedStmt::While { cond, body } => {
            expr_callees(cond, out);
            for s in body { stmt_callees(s, out); }
        }
        TypedStmt::For { start, end, body, .. } => {
            expr_callees(start, out);
            expr_callees(end, out);
            for s in body { stmt_callees(s, out); }
        }
        TypedStmt::ForIter { body, .. }
        | TypedStmt::TaskSpawn { body, .. }
        | TypedStmt::UnsafeBlock { body, .. } => {
            for s in body { stmt_callees(s, out); }
        }
        _ => {}
    }
}

fn expr_callees(expr: &TypedExpr, out: &mut Vec<String>) {
    match &expr.kind {
        TypedExprKind::Call { name, args, .. } => {
            // Exclude built-in calls (heap or otherwise); only
            // user-defined fn names contribute to the call graph.
            // We can't perfectly distinguish at this layer, but
            // names that are also builtins won't appear as
            // FrameReport entries and will just be skipped
            // during traversal.
            out.push(name.clone());
            for a in args { expr_callees(a, out); }
        }
        TypedExprKind::Binary { left, right, .. } => {
            expr_callees(left, out);
            expr_callees(right, out);
        }
        TypedExprKind::Unary { expr, .. } | TypedExprKind::Cast { expr, .. } => {
            expr_callees(expr, out);
        }
        TypedExprKind::Index { array, index, .. } => {
            expr_callees(array, out);
            expr_callees(index, out);
        }
        TypedExprKind::ArrayLit { elements } | TypedExprKind::Tuple { elements } => {
            for e in elements { expr_callees(e, out); }
        }
        TypedExprKind::StructLit { fields, .. } => {
            for (_, e) in fields { expr_callees(e, out); }
        }
        TypedExprKind::IfExpr { cond, then_value, else_value } => {
            expr_callees(cond, out);
            expr_callees(then_value, out);
            expr_callees(else_value, out);
        }
        TypedExprKind::Block { stmts, tail } => {
            for s in stmts { stmt_callees(s, out); }
            expr_callees(tail, out);
        }
        TypedExprKind::Match { scrutinee, arms } => {
            expr_callees(scrutinee, out);
            for a in arms { expr_callees(&a.body, out); }
        }
        TypedExprKind::FieldAccess { object, .. } => expr_callees(object, out),
        TypedExprKind::TupleAccess { tuple, .. } => expr_callees(tuple, out),
        TypedExprKind::Len { array, .. } => expr_callees(array, out),
        _ => {}
    }
}

// ---- Output formats ------------------------------------------------

pub fn format_text(report: &StackReport, max_bytes: Option<u64>) -> (String, bool) {
    let mut out = String::new();
    let mut failure = false;
    out.push_str("Per-function frame sizes:\n");
    for f in &report.frames {
        out.push_str(&format!(
            "  {:<32} {} bytes (locals: {}, prologue: {})\n",
            f.name, f.frame_bytes, f.local_bytes, FRAME_OVERHEAD_BYTES,
        ));
        if let Some(n) = f.bounded_recursion {
            out.push_str(&format!(
                "    bounded recursion depth: {}\n",
                n
            ));
        }
        if f.direct_recursion && f.bounded_recursion.is_none() {
            out.push_str("    UNBOUNDED self-recursion\n");
        }
    }
    out.push_str("\nPer-entry-point max stack depths:\n");
    for e in &report.entries {
        match e.max_depth_bytes {
            Some(d) => {
                out.push_str(&format!(
                    "  {:<32} {} bytes  via {}\n",
                    e.name,
                    d,
                    e.chain.join(" -> "),
                ));
                if let Some(max) = max_bytes {
                    if d > max {
                        failure = true;
                        out.push_str(&format!(
                            "    EXCEEDS --max={} by {} bytes\n",
                            max, d - max,
                        ));
                    }
                }
            }
            None => {
                failure = max_bytes.is_some() || failure;
                out.push_str(&format!(
                    "  {:<32} UNBOUNDED  via {}\n",
                    e.name,
                    e.chain.join(" -> "),
                ));
            }
        }
    }
    (out, failure)
}

pub fn format_json(report: &StackReport) -> String {
    let mut out = String::new();
    out.push_str("{\"frames\":[");
    for (i, f) in report.frames.iter().enumerate() {
        if i > 0 { out.push(','); }
        out.push_str(&format!(
            "{{\"name\":\"{}\",\"frame_bytes\":{},\"local_bytes\":{},\"bounded_recursion\":{},\"direct_recursion\":{}}}",
            f.name,
            f.frame_bytes,
            f.local_bytes,
            f.bounded_recursion.map(|n| n.to_string()).unwrap_or_else(|| "null".to_string()),
            f.direct_recursion,
        ));
    }
    out.push_str("],\"entries\":[");
    for (i, e) in report.entries.iter().enumerate() {
        if i > 0 { out.push(','); }
        out.push_str(&format!(
            "{{\"name\":\"{}\",\"max_depth_bytes\":{},\"chain\":[",
            e.name,
            e.max_depth_bytes.map(|d| d.to_string()).unwrap_or_else(|| "null".to_string()),
        ));
        for (j, c) in e.chain.iter().enumerate() {
            if j > 0 { out.push(','); }
            out.push_str(&format!("\"{}\"", c));
        }
        out.push_str("]}");
    }
    out.push_str("]}\n");
    out
}

pub fn format_csv(report: &StackReport) -> String {
    let mut out = String::new();
    out.push_str("entry,max_depth_bytes,chain\n");
    for e in &report.entries {
        let depth = e
            .max_depth_bytes
            .map(|d| d.to_string())
            .unwrap_or_else(|| "UNBOUNDED".to_string());
        out.push_str(&format!(
            "{},{},{}\n",
            e.name,
            depth,
            e.chain.join(" -> "),
        ));
    }
    out
}
