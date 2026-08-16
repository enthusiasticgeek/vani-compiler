//! ARC 1.3 of the HashMap monomorphization arc — collect every
//! `Type::HashMap(K, V)` pair appearing in a typed program so
//! that the per-(K, V) bundle emitter (ARC 1.4 for C, ARC 1.5
//! for LLVM) has a deterministic, deduped list of pairs to
//! synthesize.
//!
//! This module is intentionally self-contained: the collector
//! walks types only (function signatures, function bodies, struct
//! fields, const types) and never emits code. ARC 1.4/1.5 will
//! consume the output, dispatch on K's category (built-in vs
//! user struct), and emit the corresponding helper bundle:
//!
//!   - K = i64 / u64 / i32 / etc.: compiler-provided hashing via
//!     a stable scalar hash; equality is `==`.
//!   - K = OwnedStr: FNV-1a / SipHash; equality is `strcmp`.
//!   - K = struct: call the user's `implement Hash for K`
//!     method via the mangled `fn_<K>__hash` symbol; equality
//!     is field-by-field via the existing struct-`==`
//!     machinery.
//!
//! Mangling: each (K, V) pair maps to a tag
//! `intent_hashmap_<K_tag>_<V_tag>` where `<X_tag>` is the type's
//! leaf C identifier (`int64_t`, `OwnedStr`, `Score`, etc.).
//! Nested types (`HashMap<HashMap<i64, i64>, i64>`) flatten via
//! the same scheme so the tag is unambiguous.

use crate::ast::Type;
use crate::ir::{TypedProgram, TypedStmt, TypedExpr, TypedExprKind};
use std::collections::BTreeSet;

/// One concrete (K, V) instantiation discovered in the program.
/// Order in the returned Vec is insertion order (= declaration
/// / appearance order in the typed program walk) so emission is
/// deterministic across builds. Duplicates are filtered by
/// mangled tag.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HashMapPair {
    pub key: Type,
    pub value: Type,
    /// Mangled C-identifier tag — unique per (K, V) pair.
    /// Backend bundle emitters key the typedef + helper names
    /// off this tag.
    pub tag: String,
}

/// Walk every type position in `program` and return the deduped
/// list of `Type::HashMap(K, V)` pairs. Nested HashMap types
/// (e.g. `HashMap<i64, HashMap<i64, i64>>`) are walked too.
pub fn collect_hashmap_pairs(program: &TypedProgram) -> Vec<HashMapPair> {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut out: Vec<HashMapPair> = Vec::new();
    for f in &program.functions {
        collect_in_type(&f.return_type, &mut seen, &mut out);
        for p in &f.params {
            collect_in_type(&p.ty, &mut seen, &mut out);
        }
        for s in &f.body {
            collect_in_stmt(s, &mut seen, &mut out);
        }
    }
    out
}

fn collect_in_type(ty: &Type, seen: &mut BTreeSet<String>, out: &mut Vec<HashMapPair>) {
    match ty {
        Type::HashMap(k, v) => {
            // Walk inner positions first so nested pairs are
            // registered before the outer composite. Mirrors
            // `collect_vec_elements`'s topological order.
            collect_in_type(k, seen, out);
            collect_in_type(v, seen, out);
            let tag = format!("intent_hashmap_{}_{}", type_tag(k), type_tag(v));
            if seen.insert(tag.clone()) {
                out.push(HashMapPair {
                    key: (**k).clone(),
                    value: (**v).clone(),
                    tag,
                });
            }
        }
        Type::Vec(inner)
        | Type::Ref(inner)
        | Type::RefMut(inner)
        | Type::Deque(inner)
        | Type::HashSet(inner)
        | Type::BTreeSet(inner)
        | Type::BinaryHeap(inner)
        | Type::Bst(inner)
        | Type::Atomic(inner)
        | Type::Mutex(inner)
        | Type::Guard(inner) => collect_in_type(inner, seen, out),
        Type::BTreeMap(k, v) => {
            collect_in_type(k, seen, out);
            collect_in_type(v, seen, out);
        }
        Type::Channel(inner, _) => collect_in_type(inner, seen, out),
        Type::Array { element, .. } => collect_in_type(element, seen, out),
        Type::Tuple(elements) => {
            for e in elements {
                collect_in_type(e, seen, out);
            }
        }
        _ => {}
    }
}

fn collect_in_stmt(stmt: &TypedStmt, seen: &mut BTreeSet<String>, out: &mut Vec<HashMapPair>) {
    use TypedStmt as S;
    match stmt {
        S::Let { ty, expr, .. } | S::Reassign { ty, expr, .. } => {
            collect_in_type(ty, seen, out);
            collect_in_expr(expr, seen, out);
        }
        S::Drop { ty, .. } => collect_in_type(ty, seen, out),
        S::Discard { expr } => collect_in_expr(expr, seen, out),
        S::Return { expr } | S::Assert { expr, .. } | S::Prove { expr } => {
            collect_in_expr(expr, seen, out)
        }
        S::Print { items } => {
            for it in items {
                if let crate::ir::TypedPrintItem::Expr(e, _) = it {
                    collect_in_expr(e, seen, out);
                }
            }
        }
        S::If { cond, then_body, else_body } => {
            collect_in_expr(cond, seen, out);
            for s in then_body {
                collect_in_stmt(s, seen, out);
            }
            for s in else_body {
                collect_in_stmt(s, seen, out);
            }
        }
        S::While { cond, body, .. } => {
            collect_in_expr(cond, seen, out);
            for s in body {
                collect_in_stmt(s, seen, out);
            }
        }
        S::For { start, end, body, .. } => {
            collect_in_expr(start, seen, out);
            collect_in_expr(end, seen, out);
            for s in body {
                collect_in_stmt(s, seen, out);
            }
        }
        S::ForIter { body, .. }
        | S::TaskSpawn { body, .. }
        | S::UnsafeBlock { body, .. } => {
            for s in body {
                collect_in_stmt(s, seen, out);
            }
        }
        S::IndexAssign { index, value, .. } => {
            collect_in_expr(index, seen, out);
            collect_in_expr(value, seen, out);
        }
        S::FieldAssign { object, value, .. } => {
            collect_in_expr(object, seen, out);
            collect_in_expr(value, seen, out);
        }
        _ => {}
    }
}

fn collect_in_expr(expr: &TypedExpr, seen: &mut BTreeSet<String>, out: &mut Vec<HashMapPair>) {
    collect_in_type(&expr.ty, seen, out);
    use TypedExprKind as E;
    match &expr.kind {
        E::Binary { left, right, .. } => {
            collect_in_expr(left, seen, out);
            collect_in_expr(right, seen, out);
        }
        E::Unary { expr: inner, .. } | E::Cast { expr: inner, .. } => {
            collect_in_expr(inner, seen, out)
        }
        E::Call { args, .. } | E::ArrayLit { elements: args } => {
            for a in args {
                collect_in_expr(a, seen, out);
            }
        }
        E::CallIndirect { callee, args } => {
            collect_in_expr(callee, seen, out);
            for a in args {
                collect_in_expr(a, seen, out);
            }
        }
        E::Index { array, index, .. } => {
            collect_in_expr(array, seen, out);
            collect_in_expr(index, seen, out);
        }
        E::Len { array, .. } => collect_in_expr(array, seen, out),
        _ => {}
    }
}

/// Produce a C-identifier-safe tag for a type. Mirrors (but is
/// simpler than) the backend's per-type mangling — sufficient to
/// uniquely key (K, V) pairs in a HashMap bundle.
fn type_tag(ty: &Type) -> String {
    match ty {
        Type::I8 => "int8_t".to_string(),
        Type::I16 => "int16_t".to_string(),
        Type::I32 => "int32_t".to_string(),
        Type::I64 => "int64_t".to_string(),
        Type::U8 => "uint8_t".to_string(),
        Type::U16 => "uint16_t".to_string(),
        Type::U32 => "uint32_t".to_string(),
        Type::U64 => "uint64_t".to_string(),
        Type::F32 => "float".to_string(),
        Type::F64 => "double".to_string(),
        Type::Bool => "bool".to_string(),
        Type::Str => "intent_str".to_string(),
        Type::OwnedStr => "intent_owned_str".to_string(),
        Type::Struct(name) | Type::Enum(name) => name.clone(),
        Type::Vec(inner) => format!("vec_{}", type_tag(inner)),
        Type::Array { element, length } => format!("arr_{}_{}", length, type_tag(element)),
        Type::HashMap(k, v) => format!("hm_{}_{}", type_tag(k), type_tag(v)),
        Type::BTreeMap(k, v) => format!("bm_{}_{}", type_tag(k), type_tag(v)),
        Type::HashSet(inner) => format!("hs_{}", type_tag(inner)),
        Type::Tuple(elements) => {
            let parts: Vec<String> = elements.iter().map(type_tag).collect();
            format!("tup_{}", parts.join("_"))
        }
        _ => "opaque".to_string(),
    }
}

// ---- Output formats for `intentc hashmap-usage` --------------

/// Human-readable text format. Default output of the CLI.
pub fn format_text(pairs: &[HashMapPair]) -> String {
    if pairs.is_empty() {
        return "no HashMap<K, V> instantiations found\n".to_string();
    }
    let mut out = String::new();
    for p in pairs {
        out.push_str(&format!(
            "HashMap<{}, {}>  →  {}\n",
            p.key, p.value, p.tag
        ));
    }
    out.push_str(&format!(
        "\n{} unique (K, V) pair{} total\n",
        pairs.len(),
        if pairs.len() == 1 { "" } else { "s" },
    ));
    out
}

/// Structured JSON. CI-friendly.
pub fn format_json(pairs: &[HashMapPair]) -> String {
    let mut out = String::from("{\"pairs\":[");
    for (i, p) in pairs.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            "{{\"key\":{},\"value\":{},\"tag\":{}}}",
            json_string(&format!("{}", p.key)),
            json_string(&format!("{}", p.value)),
            json_string(&p.tag),
        ));
    }
    out.push_str("]}\n");
    out
}

/// CSV. Spreadsheet-friendly review.
pub fn format_csv(pairs: &[HashMapPair]) -> String {
    let mut out = String::from("key,value,tag\n");
    for p in pairs {
        out.push_str(&format!(
            "{},{},{}\n",
            csv_escape(&format!("{}", p.key)),
            csv_escape(&format!("{}", p.value)),
            csv_escape(&p.tag),
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
