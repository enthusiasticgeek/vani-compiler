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
    /// True when the function carries `#[inline]`. The stack analyser folds
    /// an inline callee's locals into the caller's frame (no separate push).
    pub inline: bool,
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
    let size_ctx = SizeCtx::new(program);
    // Task #245 (RTOS true-compliance sweep, 2026-09-18): real,
    // caller-declared per-extern-fn stack costs, closing BUG-233's own
    // explicitly-deferred remainder. Built once here from every
    // `extern "C" fn` carrying `#[stack_cost(bytes=N)]`
    // (`ast::Function::stack_cost_bytes`, forwarded through the
    // checker unchanged for extern fns -- see checker.rs's own extern
    // TypedFunction construction). `traverse_depth`'s unresolvable-
    // callee branch consults this before falling back to the flat
    // `FRAME_OVERHEAD_BYTES` conservative default BUG-233 introduced.
    let mut extern_costs: HashMap<String, u64> = HashMap::new();
    for f in &program.functions {
        if f.is_extern {
            if let Some(cost) = f.stack_cost_bytes {
                extern_costs.insert(f.name.clone(), cost);
            }
        }
    }
    let mut frames: Vec<FrameReport> = Vec::new();
    for f in &program.functions {
        if f.is_extern {
            continue;
        }
        let mut local_bytes = 0u64;
        for p in &f.params {
            local_bytes += type_size(&p.ty, &size_ctx);
        }
        for s in &f.body {
            local_bytes += stmt_local_bytes(s, &size_ctx);
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
            inline: f.inline,
            callees,
        });
    }
    let frame_map: HashMap<String, FrameReport> = frames
        .iter()
        .map(|f| (f.name.clone(), f.clone()))
        .collect();
    // PERFORMANCE FIX (2026-10-11, found live: a real DhruvaOS build on a
    // 45,613-line single-file kernel took 10m40s wall-clock for this one
    // analysis pass alone -- over half a ~20-minute total build). The
    // previous implementation was a plain top-down recursive DFS
    // (`traverse_depth`, removed) with NO memoization: every time a
    // widely-shared helper (e.g. a `uart_puts`/`buf_read_byte`-style leaf
    // called from thousands of sites) was reached via a DIFFERENT call
    // path, its entire subtree got re-walked from scratch. On a call
    // graph this size that's the textbook shape of exponential/
    // polynomial blowup in path count, even though the actual underlying
    // computation (each function's own max-depth-below-it) is a
    // path-independent property almost everywhere.
    //
    // Fixed by computing the call graph's strongly-connected components
    // once (`compute_sccs`, iterative Tarjan's, O(V+E)) and then doing a
    // SINGLE bottom-up memoized pass over the resulting DAG of SCCs
    // (`compute_all_fn_depths`) instead of re-deriving each function's
    // depth once per incoming call-graph edge. SCCs are the textbook-
    // correct way to make this sound: the OLD code's `visiting`-based
    // cycle detection only saw the CURRENT path's ancestors, so whether
    // a given back-edge counted as "a cycle" could (in a pathological
    // multi-function-mutual-recursion case) depend on which caller
    // reached a function first -- a real, inherent soundness gap, not
    // just a performance one. SCC membership is a global, path-
    // independent graph property, so this fix closes that gap as a
    // side effect of fixing the performance bug. True multi-function
    // cycles (SCC size > 1) are always reported unbounded, matching this
    // module's own documented contract above ("Mutual recursion via
    // cycle detection: same treatment as direct unbounded" -- #[bounded
    // (N)] is documented and used for DIRECT self-recursion only); a
    // single-function SCC with a self-loop keeps the exact existing
    // self-recursion-bounded formula. Both changes only ever make an
    // already-ambiguous case MORE conservative (bounded -> unbounded),
    // never the reverse, matching this checker's own stated "over-
    // estimation is safe" philosophy stated elsewhere in this module.
    let fn_depths = compute_all_fn_depths(&frame_map, &extern_costs);
    let mut entries: Vec<EntryReport> = Vec::new();
    for f in &frames {
        if let Some(filter) = entry_filter {
            if filter != f.name {
                continue;
            }
        }
        // Entry points always use this function's own full frame cost
        // (never folded as an inline callee), matching the previous
        // implementation's own hardcoded `is_inline_call=false` for
        // every top-level entry.
        let fd = &fn_depths[&f.name];
        let depth = fd.best.map(|b| b + f.frame_bytes);
        let mut chain = vec![f.name.clone()];
        chain.extend(fd.chain_tail.clone());
        entries.push(EntryReport {
            name: f.name.clone(),
            max_depth_bytes: depth,
            chain,
        });
    }
    StackReport { frames, entries }
}

/// Strongly-connected components of the function call graph, computed via
/// iterative (non-recursive, to stay safe on deep real-world call chains)
/// Tarjan's algorithm. Returns `name -> scc_id`. SCC ids are assigned in
/// Tarjan's own natural completion order, which guarantees: if SCC X calls
/// into a DIFFERENT SCC Y, then `id(Y) < id(X)` -- i.e., increasing scc_id
/// order is already bottom-up (callee SCCs complete, and get an id,
/// strictly before the SCCs of anything that calls into them).
fn compute_sccs(frame_map: &HashMap<String, FrameReport>) -> HashMap<String, usize> {
    let mut index_counter: usize = 0;
    let mut indices: HashMap<String, usize> = HashMap::new();
    let mut lowlink: HashMap<String, usize> = HashMap::new();
    let mut on_stack_set: HashSet<String> = HashSet::new();
    let mut tstack: Vec<String> = Vec::new();
    let mut scc_of: HashMap<String, usize> = HashMap::new();
    let mut next_scc_id: usize = 0;

    let all_names: Vec<String> = frame_map.keys().cloned().collect();

    for start_name in &all_names {
        if indices.contains_key(start_name) {
            continue;
        }
        // Explicit DFS stack standing in for the call stack a recursive
        // Tarjan's would use: (node name, index of the next callee of
        // `node` still to be examined).
        let mut work: Vec<(String, usize)> = vec![(start_name.clone(), 0)];

        while let Some(top) = work.last().cloned() {
            let (cur_name, cur_idx) = top;
            if !indices.contains_key(&cur_name) {
                indices.insert(cur_name.clone(), index_counter);
                lowlink.insert(cur_name.clone(), index_counter);
                index_counter += 1;
                tstack.push(cur_name.clone());
                on_stack_set.insert(cur_name.clone());
            }

            let callees: &[String] = frame_map
                .get(&cur_name)
                .map(|f| f.callees.as_slice())
                .unwrap_or(&[]);

            let mut i = cur_idx;
            let mut pushed_child: Option<String> = None;
            while i < callees.len() {
                let callee = &callees[i];
                // A callee not present in frame_map is an extern fn (or
                // otherwise outside the analyzed program) -- a leaf with
                // no further edges, not a graph node here.
                if !frame_map.contains_key(callee) {
                    i += 1;
                    continue;
                }
                if !indices.contains_key(callee) {
                    pushed_child = Some(callee.clone());
                    i += 1; // resume here once the child fully returns
                    break;
                }
                if on_stack_set.contains(callee) {
                    let callee_idx = indices[callee];
                    let cur_low = lowlink[&cur_name];
                    if callee_idx < cur_low {
                        lowlink.insert(cur_name.clone(), callee_idx);
                    }
                }
                i += 1;
            }

            if let Some(child) = pushed_child {
                work.last_mut().unwrap().1 = i;
                work.push((child, 0));
                continue;
            }

            // No unvisited children left -- cur_name is fully explored.
            work.pop();
            if let Some((parent_name, _)) = work.last() {
                let cur_low = lowlink[&cur_name];
                let parent_low = lowlink[parent_name];
                if cur_low < parent_low {
                    lowlink.insert(parent_name.clone(), cur_low);
                }
            }
            if lowlink[&cur_name] == indices[&cur_name] {
                // cur_name roots an SCC -- pop the Tarjan stack down to it.
                let scc_id = next_scc_id;
                next_scc_id += 1;
                loop {
                    let member = tstack.pop().expect("Tarjan stack underflow: unreachable, every pushed node is popped exactly once");
                    on_stack_set.remove(&member);
                    scc_of.insert(member.clone(), scc_id);
                    if member == cur_name {
                        break;
                    }
                }
            }
        }
    }

    scc_of
}

/// Per-function memoized result: `best` is the max depth contributed by
/// this function's OWN callees (i.e. NOT including this function's own
/// frame/local-byte contribution -- that depends on whether the CALLER
/// treats this function as inlined or not, so it's added back by each
/// caller separately, matching the previous implementation's own
/// `is_inline_call`-dependent `frame_contribution`). `chain_tail` is the
/// deepest call chain starting from (and including) the deepest callee,
/// NOT including this function's own name (each caller prepends its own
/// name when consuming this).
struct FnDepth {
    best: Option<u64>,
    chain_tail: Vec<String>,
}

fn leaf_depth(name: &str, extern_costs: &HashMap<String, u64>) -> (Option<u64>, Vec<String>) {
    // See the long comment on this exact branch in the previous
    // `traverse_depth` implementation (preserved verbatim in spirit):
    // an extern fn with no `#[stack_cost(bytes=N)]` is conservatively
    // charged the same flat `FRAME_OVERHEAD_BYTES` every ordinary
    // function uses, rather than silently treating it as zero-cost.
    if let Some(&real_cost) = extern_costs.get(name) {
        (Some(real_cost), vec![name.to_string()])
    } else {
        (Some(FRAME_OVERHEAD_BYTES), vec![name.to_string()])
    }
}

/// Single bottom-up pass over the call graph's SCCs (in increasing,
/// already-bottom-up scc_id order), computing each function's `FnDepth`
/// exactly once. Replaces the old per-call-path recursive re-derivation.
fn compute_all_fn_depths(
    frame_map: &HashMap<String, FrameReport>,
    extern_costs: &HashMap<String, u64>,
) -> HashMap<String, FnDepth> {
    let scc_of = compute_sccs(frame_map);
    let mut scc_count: usize = 0;
    for &id in scc_of.values() {
        if id + 1 > scc_count {
            scc_count = id + 1;
        }
    }
    let mut scc_members: Vec<Vec<String>> = vec![Vec::new(); scc_count];
    for (name, &id) in &scc_of {
        scc_members[id].push(name.clone());
    }

    let mut memo: HashMap<String, FnDepth> = HashMap::new();

    for members in &scc_members {
        if members.len() > 1 {
            // Genuine multi-function mutual recursion. Per this module's
            // own documented contract, always unbounded -- #[bounded(N)]
            // is for direct self-recursion only (handled in the
            // single-member branch below).
            for name in members {
                memo.insert(
                    name.clone(),
                    FnDepth { best: None, chain_tail: vec!["[unbounded mutual recursion]".to_string()] },
                );
            }
            continue;
        }

        let name = &members[0];
        let frame = &frame_map[name];
        let has_self_loop = frame.callees.iter().any(|c| c == name);

        let mut best: Option<u64> = Some(0);
        let mut chain_tail: Vec<String> = Vec::new();

        if has_self_loop {
            if let Some(n) = frame.bounded_recursion {
                let frame_contribution = if frame.inline { frame.local_bytes } else { frame.frame_bytes };
                let extra = frame_contribution.saturating_mul(n);
                if extra > best.unwrap_or(0) {
                    best = Some(extra);
                    chain_tail = vec![format!("[bounded {} self-recursion]", n)];
                }
            } else {
                best = None;
                chain_tail = vec!["[unbounded self-recursion]".to_string()];
            }
        }

        if best.is_some() {
            // PERFORMANCE (2026-10-11, found immediately after the first
            // version of this SCC-based rewrite still took minutes, not
            // the expected near-instant): this scan used to build the
            // FULL candidate chain (`c.extend(cd.chain_tail.clone())`)
            // for EVERY callee just to compare depths, even for the
            // losing candidates -- O(E * average-chain-depth) instead of
            // O(E), and on this call graph's own long self-test chains
            // that was still enough to dominate real time. Comparing
            // bare `Option<u64>` depths first (O(1) per callee) and only
            // building the winning chain ONCE, after the scan, drops
            // this back to genuinely O(E) for the comparison pass.
            let mut best_callee: Option<&str> = None;
            for callee in &frame.callees {
                if callee == name {
                    continue; // handled via has_self_loop above
                }
                let sub_depth: Option<u64> = if let Some(cf) = frame_map.get(callee) {
                    // Callee's own SCC is guaranteed already memoized:
                    // it necessarily has a smaller scc_id (processed
                    // earlier in this same bottom-up loop) unless it's
                    // in the SAME SCC as `name` -- impossible here since
                    // this branch is single-member-SCC-only and callee
                    // != name was just checked above.
                    let cd = &memo[callee];
                    let contribution = if cf.inline { cf.local_bytes } else { cf.frame_bytes };
                    cd.best.map(|b| b + contribution)
                } else if let Some(&c) = extern_costs.get(callee.as_str()) {
                    Some(c)
                } else {
                    Some(FRAME_OVERHEAD_BYTES)
                };

                match (best, sub_depth) {
                    (Some(a), Some(b)) => {
                        if b > a {
                            best = Some(b);
                            best_callee = Some(callee.as_str());
                        }
                    }
                    _ => {
                        best = None;
                        best_callee = Some(callee.as_str());
                        break; // unbounded is terminal -- no later callee can un-set it
                    }
                }
            }
            if let Some(bc) = best_callee {
                chain_tail = if let Some(_cf) = frame_map.get(bc) {
                    let cd = &memo[bc];
                    let mut c = vec![bc.to_string()];
                    c.extend(cd.chain_tail.clone());
                    c
                } else {
                    let (_, lc) = leaf_depth(bc, extern_costs);
                    lc
                };
            }
        }

        memo.insert(name.clone(), FnDepth { best, chain_tail });
    }

    memo
}

/// Struct/enum field-type lookup so `type_size` can compute real
/// aggregate sizes instead of a flat guess. Built once per
/// `compute_stack_depths` call from `TypedProgram.structs` /
/// `.enums` (mirrors the pattern `backend_llvm.rs`'s
/// `llvm_byte_size` already uses via `LLVM_STRUCT_FIELDS_REGISTRY`
/// / `LLVM_ENUM_VARIANT_PAYLOADS_REGISTRY` -- this module has no
/// access to those backend-only thread-locals, and doesn't need
/// its own registry since `TypedProgram` already carries the same
/// information directly).
struct SizeCtx<'a> {
    structs: HashMap<&'a str, &'a [(String, Type)]>,
    enum_payloads: HashMap<&'a str, &'a [Option<Type>]>,
}

impl<'a> SizeCtx<'a> {
    fn new(program: &'a TypedProgram) -> Self {
        SizeCtx {
            structs: program.structs.iter().map(|s| (s.name.as_str(), s.fields.as_slice())).collect(),
            enum_payloads: program.enums.iter().map(|e| (e.name.as_str(), e.payload_types.as_slice())).collect(),
        }
    }
}

/// Bug fix (2026-07-28): `Struct(_) => 32` and `Enum(_) => 16` used
/// to be flat guesses regardless of the actual declared fields --
/// unsound for the module's own documented "over-estimation is
/// safe" invariant (see the module doc comment above). A struct
/// with more than 4 `i64`-sized fields (or an enum with a payload
/// bigger than 12 bytes) was UNDERestimated, which could let a
/// `#[bounded_stack(N)]` function that actually overflows the
/// stack pass verification. Fixed by looking up real field types
/// from `TypedProgram.structs` / `.enums` (threaded in via
/// `SizeCtx`) and recursing, mirroring the already-correct
/// `llvm_byte_size` in `backend_llvm.rs`.
fn type_size(ty: &Type, ctx: &SizeCtx) -> u64 {
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
        TaskR(_) => 16,
        Condvar => 8,
        Barrier => 24, // { i64 count, i64 n, i32 gen, pad }
        FileHandle => 8, // int64_t wrapping FILE*
        RwLock(inner) => type_size(inner, ctx).max(8) + 8, // T + state + pad
        ReadGuard(_) | WriteGuard(_) => 8, // pointer to RwLock
        Box(_) => 8, // L2 Phase 1: Box<T> = T* (8 bytes on x86-64)
        Atomic(inner) | Mutex(inner) | Guard(inner) => type_size(inner, ctx).max(8),
        Channel(_, _) => 32,
        Deque(_) | HashSet(_) | HashMap(_, _) | BTreeSet(_) | BTreeMap(_, _) => 32,
        UnionFind | BinaryHeap(_) | BloomFilter | Bst(_) | Graph | Trie | SkipList => 48,
        Pool(_) => 48,
        Region => 24,
        BoundedPtr(_) => 24,
        Object(_) => 16,
        Vec(_) => 24, // {data, len, cap}
        Vec128(_) => 16, // 128-bit SIMD register value
        Vec256(_) => 32, // 256-bit SIMD register value
        Vec512(_) => 64, // 512-bit SIMD register value
        Array { element, length } => type_size(element, ctx).saturating_mul(*length),
        Tuple(elements) => elements.iter().map(|t| type_size(t, ctx)).sum::<u64>().max(8),
        Struct(name) => ctx
            .structs
            .get(name.as_str())
            .map(|fields| fields.iter().map(|(_, t)| type_size(t, ctx)).sum::<u64>().max(8))
            // Not found should be unreachable (every declared struct
            // is in TypedProgram.structs); keep the old flat guess
            // as a defensive fallback rather than panicking.
            .unwrap_or(32),
        Enum(name) => {
            let payload = ctx
                .enum_payloads
                .get(name.as_str())
                .map(|payloads| {
                    payloads
                        .iter()
                        .filter_map(|p| p.as_ref())
                        .map(|t| type_size(t, ctx))
                        .max()
                        .unwrap_or(0)
                })
                .unwrap_or(12); // defensive fallback, see Struct arm above
            4 + payload // 4-byte tag + max-payload
        }
        Apply { .. } | Param(_) => 16,
        Tainted(inner) => type_size(inner, ctx),
    }
}

fn stmt_local_bytes(stmt: &TypedStmt, ctx: &SizeCtx) -> u64 {
    match stmt {
        TypedStmt::Let { ty, .. } | TypedStmt::Reassign { ty, .. } => type_size(ty, ctx),
        TypedStmt::If { then_body, else_body, .. } => {
            // Branches share the stack frame; take the max.
            let t: u64 = then_body.iter().map(|s| stmt_local_bytes(s, ctx)).sum();
            let e: u64 = else_body.iter().map(|s| stmt_local_bytes(s, ctx)).sum();
            t.max(e)
        }
        TypedStmt::While { body, .. }
        | TypedStmt::For { body, .. }
        | TypedStmt::ForIter { body, .. }
        | TypedStmt::TaskSpawn { body, .. }
        | TypedStmt::UnsafeBlock { body, .. } => {
            body.iter().map(|s| stmt_local_bytes(s, ctx)).sum()
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
                if let crate::ir::TypedPrintItem::Expr(e, _) = it {
                    expr_callees(e, out);
                }
            }
        }
        TypedStmt::If { cond, then_body, else_body } => {
            expr_callees(cond, out);
            for s in then_body { stmt_callees(s, out); }
            for s in else_body { stmt_callees(s, out); }
        }
        TypedStmt::While { cond, body, .. } => {
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
        let inline_tag = if f.inline { " [inline]" } else { "" };
        out.push_str(&format!(
            "  {:<32} {} bytes (locals: {}, prologue: {}){}\n",
            f.name, f.frame_bytes, f.local_bytes, FRAME_OVERHEAD_BYTES, inline_tag,
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
