//! T3.3 of the safety-standard alignment arc — call-graph
//! acyclicity proof. Catches mutual recursion in the program's
//! call graph; the existing `#[no_recursion]` attribute handles
//! single-function direct recursion at the type-checker level,
//! but mutual recursion (`a -> b -> a`) only surfaces when the
//! whole-program graph is built.
//!
//! Required by DO-178C Level A and ASIL-D timing analysis: every
//! call chain must terminate in bounded time, and unbounded
//! recursion violates that. Functions with `#[bounded(N)]` are
//! exempt from the direct-self-call rule (their runtime guard
//! caps the recursion depth at N).
//!
//! The pass implements Tarjan's strongly-connected-components
//! algorithm on the call graph. Any non-trivial SCC (size > 1)
//! is a mutual-recursion cycle. Self-loops (size-1 SCCs where
//! the function calls itself) are violations unless the function
//! is annotated `#[bounded(N)]`.
//!
//! Output formats (`--format`):
//! - **text** (default): human-readable report listing each cycle
//! - **json**: structured records for CI integration
//! - **csv**: tabular for spreadsheet review

use crate::ir::TypedProgram;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cycle {
    /// Members of the cycle in topological order (smallest name
    /// first so output is deterministic).
    pub members: Vec<String>,
    /// True if every member is annotated `#[bounded(N)]` (which
    /// makes the cycle acceptable). False means at least one
    /// participant is unbounded — a violation.
    pub all_bounded: bool,
}

#[derive(Clone, Debug)]
pub struct AcyclicityReport {
    /// All non-trivial cycles in the call graph + self-loops
    /// that aren't `#[bounded(N)]`-annotated.
    pub cycles: Vec<Cycle>,
}

impl AcyclicityReport {
    pub fn has_violations(&self) -> bool {
        self.cycles.iter().any(|c| !c.all_bounded)
    }
}

/// Build the call graph, run Tarjan's SCC, return all cycles.
pub fn check_acyclicity(program: &TypedProgram) -> AcyclicityReport {
    // Build adjacency: fn name -> list of callee fn names.
    let mut graph: HashMap<String, Vec<String>> = HashMap::new();
    let mut bounded: HashMap<String, bool> = HashMap::new();
    let mut names: Vec<String> = Vec::new();
    for f in &program.functions {
        if f.is_extern {
            continue;
        }
        let mut callees: Vec<String> = Vec::new();
        for s in &f.body {
            crate::stack_depth::stmt_callees(s, &mut callees);
        }
        callees.sort();
        callees.dedup();
        bounded.insert(f.name.clone(), f.recursion_bound.is_some());
        graph.insert(f.name.clone(), callees);
        names.push(f.name.clone());
    }
    let sccs = tarjan_scc(&names, &graph);
    let mut cycles: Vec<Cycle> = Vec::new();
    for scc in sccs {
        if scc.len() > 1 {
            // Non-trivial SCC — mutual recursion.
            let all_bounded = scc.iter().all(|n| *bounded.get(n).unwrap_or(&false));
            let mut members = scc.clone();
            members.sort();
            cycles.push(Cycle { members, all_bounded });
        } else if scc.len() == 1 {
            // Singleton — self-loop only if the fn calls itself.
            let f = &scc[0];
            if let Some(callees) = graph.get(f) {
                if callees.contains(f) {
                    let all_bounded = *bounded.get(f).unwrap_or(&false);
                    cycles.push(Cycle {
                        members: vec![f.clone()],
                        all_bounded,
                    });
                }
            }
        }
    }
    cycles.sort_by(|a, b| a.members.cmp(&b.members));
    AcyclicityReport { cycles }
}

/// Iterative Tarjan's SCC. Returns each SCC as a Vec of names.
///
/// `pub(crate)` since the Kosh namespacing arc's Phase 2
/// (`manifest::check_dependency_cycles`, `docs/kosh_namespacing_design.md`)
/// reuses this exact algorithm against the *package* dependency graph
/// instead of the function-call graph -- the implementation is already
/// generic over any `HashMap<String, Vec<String>>` adjacency, so no
/// changes were needed to make it reusable.
pub(crate) fn tarjan_scc(nodes: &[String], graph: &HashMap<String, Vec<String>>) -> Vec<Vec<String>> {
    let n = nodes.len();
    let mut index_of: HashMap<String, usize> = HashMap::new();
    for (i, name) in nodes.iter().enumerate() {
        index_of.insert(name.clone(), i);
    }
    let mut indices: Vec<Option<usize>> = vec![None; n];
    let mut lowlinks: Vec<usize> = vec![0; n];
    let mut on_stack: Vec<bool> = vec![false; n];
    let mut stack: Vec<usize> = Vec::new();
    let mut sccs: Vec<Vec<String>> = Vec::new();
    let mut index_counter: usize = 0;

    // Iterative DFS to avoid deep recursion. State per node:
    // (node_idx, iterator over its successors as Vec<usize>, position).
    for v_start in 0..n {
        if indices[v_start].is_some() {
            continue;
        }
        let mut call_stack: Vec<(usize, Vec<usize>, usize)> = Vec::new();
        let successors_of = |idx: usize| -> Vec<usize> {
            graph
                .get(&nodes[idx])
                .map(|v| v.iter().filter_map(|n| index_of.get(n).copied()).collect())
                .unwrap_or_default()
        };
        indices[v_start] = Some(index_counter);
        lowlinks[v_start] = index_counter;
        index_counter += 1;
        stack.push(v_start);
        on_stack[v_start] = true;
        call_stack.push((v_start, successors_of(v_start), 0));
        while let Some((v, succs, pos)) = call_stack.last_mut() {
            if *pos < succs.len() {
                let w = succs[*pos];
                *pos += 1;
                if indices[w].is_none() {
                    indices[w] = Some(index_counter);
                    lowlinks[w] = index_counter;
                    index_counter += 1;
                    stack.push(w);
                    on_stack[w] = true;
                    call_stack.push((w, successors_of(w), 0));
                } else if on_stack[w] {
                    let v_idx = *v;
                    lowlinks[v_idx] = lowlinks[v_idx].min(indices[w].unwrap());
                }
            } else {
                let v_idx = *v;
                if lowlinks[v_idx] == indices[v_idx].unwrap() {
                    let mut scc: Vec<String> = Vec::new();
                    while let Some(w) = stack.pop() {
                        on_stack[w] = false;
                        scc.push(nodes[w].clone());
                        if w == v_idx {
                            break;
                        }
                    }
                    sccs.push(scc);
                }
                call_stack.pop();
                if let Some((parent, _, _)) = call_stack.last() {
                    let p_idx = *parent;
                    lowlinks[p_idx] = lowlinks[p_idx].min(lowlinks[v_idx]);
                }
            }
        }
    }
    sccs
}

pub fn format_text(report: &AcyclicityReport) -> String {
    if report.cycles.is_empty() {
        return "no cycles in the call graph\n".to_string();
    }
    let mut out = String::new();
    for c in &report.cycles {
        let tag = if c.all_bounded {
            "[BOUNDED] "
        } else {
            "[UNBOUNDED] "
        };
        out.push_str(tag);
        if c.members.len() == 1 {
            out.push_str(&format!("self-loop: {}\n", c.members[0]));
        } else {
            out.push_str(&format!("cycle: {}\n", c.members.join(" -> ")));
        }
    }
    let violations = report.cycles.iter().filter(|c| !c.all_bounded).count();
    out.push_str(&format!(
        "\n{} cycle{} total, {} violation{}\n",
        report.cycles.len(),
        if report.cycles.len() == 1 { "" } else { "s" },
        violations,
        if violations == 1 { "" } else { "s" },
    ));
    out
}

pub fn format_json(report: &AcyclicityReport) -> String {
    let mut out = String::from("{\"cycles\":[");
    for (i, c) in report.cycles.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str("{\"members\":[");
        for (j, m) in c.members.iter().enumerate() {
            if j > 0 {
                out.push(',');
            }
            out.push_str(&json_string(m));
        }
        out.push_str(&format!(
            "],\"all_bounded\":{},\"violation\":{}}}",
            c.all_bounded, !c.all_bounded
        ));
    }
    out.push_str("]}\n");
    out
}

pub fn format_csv(report: &AcyclicityReport) -> String {
    let mut out = String::from("members,size,all_bounded,violation\n");
    for c in &report.cycles {
        out.push_str(&format!(
            "{},{},{},{}\n",
            csv_escape(&c.members.join("->")),
            c.members.len(),
            c.all_bounded,
            !c.all_bounded
        ));
    }
    out
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        let escaped = s.replace('"', "\"\"");
        format!("\"{}\"", escaped)
    } else {
        s.to_string()
    }
}

fn json_string(s: &str) -> String {
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
