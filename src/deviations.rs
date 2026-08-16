//! T1.1 of the safety-standard alignment arc — walks the
//! `TypedStmt::UnsafeBlock { reason, body, … }` nodes in a
//! typed program and emits a structured deviation-record
//! artifact. The artifact is the document ASIL-D / DO-178C /
//! IEC 62304 / MISRA C 2012 reviewers want to sign off on:
//! every escape from the safe surface, keyed by file:line,
//! categorized by reason prefix, and (once Tier 1.4 lands)
//! tagged with the standard target of the enclosing function.
//!
//! Output formats (selected via the CLI's `--format`):
//!   - **CSV**: `file,line,prefix,reason,target_standard`
//!     header + one row per deviation. Spreadsheet-friendly.
//!   - **JSON**: `{ "deviations": [{ … }] }` with one
//!     structured record per deviation. Easier downstream
//!     tooling integration (CI dashboards, audit pipelines).
//!   - **Text**: human-readable, one deviation per line
//!     formatted as `file:line [prefix] reason
//!     (in <fn>, standard=<tag>)`. Console-friendly.
//!
//! V1 (this commit): `target_standard` is always `"none"` —
//! the standard composite tags (`#[asil_d]`, `#[misra_c_2012]`,
//! …) land in subsequent commits (Tier 1.4+). When they do,
//! `extract_deviations` will read the enclosing function's
//! tag and populate the field.
//!
//! Reason-prefix derivation matches the convention
//! recommended in `unsafe.md` § "Reason-string rules (v1)":
//! `MMIO: …`, `FFI: …`, `DMA: …`, `transmute: …`,
//! `vendor-SDK: …`. If the reason doesn't start with one of
//! the recommended prefixes, `prefix = "other"`.

use crate::diagnostic::FileMap;
use crate::ir::{TypedProgram, TypedStmt};

/// One row of the deviation artifact. The fields map 1:1 to
/// CSV columns / JSON object keys / text-format substitutions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Deviation {
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub prefix: String,
    pub reason: String,
    pub function: String,
    pub target_standard: String,
}

/// Walk every `TypedStmt::UnsafeBlock` in the program and
/// build a `Deviation` for it. The enclosing function's
/// `safety_standard` tag (from a `#[asil_d]` / `#[misra_c_2012]`
/// / etc. composite annotation) populates the `target_standard`
/// column on the resulting record. Functions with no composite
/// tag get `target_standard = "none"`.
pub fn extract_deviations(program: &TypedProgram, map: &FileMap) -> Vec<Deviation> {
    let mut out = Vec::new();
    for f in &program.functions {
        let standard = f
            .safety_standard
            .clone()
            .unwrap_or_else(|| "none".to_string());
        walk_stmts(&f.body, &f.name, &standard, map, &mut out);
    }
    out
}

fn walk_stmts(
    stmts: &[TypedStmt],
    fn_name: &str,
    standard: &str,
    map: &FileMap,
    out: &mut Vec<Deviation>,
) {
    for stmt in stmts {
        match stmt {
            TypedStmt::UnsafeBlock { reason, body } => {
                let span = first_stmt_span(body);
                let (file, line, column) = resolve(span, map);
                out.push(Deviation {
                    file,
                    line,
                    column,
                    prefix: prefix_of(reason),
                    reason: reason.clone(),
                    function: fn_name.to_string(),
                    target_standard: standard.to_string(),
                });
                walk_stmts(body, fn_name, standard, map, out);
            }
            TypedStmt::If { then_body, else_body, .. } => {
                walk_stmts(then_body, fn_name, standard, map, out);
                walk_stmts(else_body, fn_name, standard, map, out);
            }
            TypedStmt::While { body, .. }
            | TypedStmt::For { body, .. }
            | TypedStmt::ForIter { body, .. }
            | TypedStmt::TaskSpawn { body, .. } => {
                walk_stmts(body, fn_name, standard, map, out);
            }
            _ => {}
        }
    }
}

/// Best-effort span for a body — first stmt's span when one
/// exists. Returns a zero span if the body is empty (which
/// shouldn't happen for a non-empty source-level unsafe block
/// but might if a future refactor admits empty bodies).
fn first_stmt_span(body: &[TypedStmt]) -> crate::span::Span {
    body.first()
        .and_then(stmt_span)
        .unwrap_or_default()
}

fn stmt_span(stmt: &TypedStmt) -> Option<crate::span::Span> {
    use TypedStmt as S;
    match stmt {
        S::Let { expr, .. } => Some(expr.span),
        S::Reassign { expr, .. } => Some(expr.span),
        S::Return { expr } => Some(expr.span),
        S::Assert { expr, .. } => Some(expr.span),
        S::Prove { expr } => Some(expr.span),
        S::Discard { expr } => Some(expr.span),
        S::Print { items } => items.iter().find_map(|it| match it {
            crate::ir::TypedPrintItem::Expr(e, _) => Some(e.span),
            _ => None,
        }),
        S::IndexAssign { value, .. } | S::FieldAssign { value, .. } => Some(value.span),
        _ => None,
    }
}

/// Map a global byte offset to `(file_path, line, column)`.
/// Lines and columns are 1-indexed (matches everything else
/// in `intentc` and most IDEs).
fn resolve(span: crate::span::Span, map: &FileMap) -> (String, u32, u32) {
    let Some((entry, local_offset)) = map.lookup(span.start) else {
        return ("<unknown>".to_string(), 0, 0);
    };
    let prefix_text = &entry.source[..local_offset.min(entry.source.len())];
    let line = (prefix_text.bytes().filter(|b| *b == b'\n').count() + 1) as u32;
    let column = match prefix_text.rfind('\n') {
        Some(nl) => (local_offset - nl) as u32,
        None => (local_offset + 1) as u32,
    };
    (entry.path.clone(), line, column)
}

/// Derive the recommended `unsafe.md` reason prefix from the
/// reason string. Conventional prefixes: `MMIO:`, `FFI:`,
/// `DMA:`, `transmute:`, `vendor-SDK:`. Anything else surfaces
/// as `other`.
fn prefix_of(reason: &str) -> String {
    let trimmed = reason.trim_start();
    for prefix in &["MMIO:", "FFI:", "DMA:", "transmute:", "vendor-SDK:"] {
        if trimmed.starts_with(prefix) {
            return prefix.trim_end_matches(':').to_string();
        }
    }
    "other".to_string()
}

// ---- Output formats ------------------------------------------------

pub fn format_csv(deviations: &[Deviation]) -> String {
    let mut out = String::new();
    out.push_str("file,line,column,prefix,reason,function,target_standard\n");
    for d in deviations {
        out.push_str(&format!(
            "{},{},{},{},{},{},{}\n",
            csv_escape(&d.file),
            d.line,
            d.column,
            csv_escape(&d.prefix),
            csv_escape(&d.reason),
            csv_escape(&d.function),
            csv_escape(&d.target_standard),
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

pub fn format_json(deviations: &[Deviation]) -> String {
    let mut out = String::new();
    out.push_str("{\"deviations\":[");
    for (i, d) in deviations.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            "{{\"file\":{},\"line\":{},\"column\":{},\"prefix\":{},\"reason\":{},\"function\":{},\"target_standard\":{}}}",
            json_string(&d.file),
            d.line,
            d.column,
            json_string(&d.prefix),
            json_string(&d.reason),
            json_string(&d.function),
            json_string(&d.target_standard),
        ));
    }
    out.push_str("]}\n");
    out
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

pub fn format_text(deviations: &[Deviation]) -> String {
    if deviations.is_empty() {
        return "no unsafe deviations found\n".to_string();
    }
    let mut out = String::new();
    for d in deviations {
        out.push_str(&format!(
            "{}:{}:{}: [{}] {} (in fn {}, standard={})\n",
            d.file, d.line, d.column, d.prefix, d.reason, d.function, d.target_standard,
        ));
    }
    out.push_str(&format!(
        "\n{} deviation{} total\n",
        deviations.len(),
        if deviations.len() == 1 { "" } else { "s" },
    ));
    out
}
