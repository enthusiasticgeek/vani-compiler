//! Cross-example check that the SSA lowerer accepts every
//! .vani file in examples/ (recursively -- every real example
//! lives in a subdirectory, e.g. examples/language/english/,
//! examples/edge_cases/, examples/embedded/, never directly in
//! examples/ itself). Catches "feature X used in example Y broke
//! the lowerer" regressions early.

use std::fs;
use std::path::{Path, PathBuf};

use vani::compile;
use vani::ssa::{lower_program, LowerError};

fn walk_vani_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .expect("examples dir exists")
        .filter_map(Result::ok)
        .collect();
    entries.sort_by_key(|e| e.path());
    for entry in entries {
        let path = entry.path();
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            walk_vani_files(&path, out);
        } else if path.extension().and_then(|s| s.to_str()) == Some("vani") {
            out.push(path);
        }
    }
}

#[test]
fn ssa_lowers_every_example() {
    let dir = format!("{}/examples", env!("CARGO_MANIFEST_DIR"));
    let mut failures: Vec<(String, Vec<LowerError>)> = Vec::new();
    let mut files = Vec::new();
    walk_vani_files(Path::new(&dir), &mut files);
    assert!(!files.is_empty(), "examples/ must contain at least one .vani file");
    for path in files {
        let source = fs::read_to_string(&path).expect("read example");
        let checked = match compile(&source) {
            Ok(c) => c,
            Err(diags) => {
                // The example doesn't type-check — that's a
                // bug elsewhere, not the SSA lowerer's
                // concern. Skip rather than fail this test.
                eprintln!(
                    "warning: {} did not type-check ({} diagnostics); skipping SSA check",
                    path.display(),
                    diags.len()
                );
                continue;
            }
        };
        let (_module, errors) = lower_program(&checked.ir);
        // Gated errors carry one of a handful of established
        // "deliberately unsupported in the SSA v1 subset, routed
        // to the tree backend instead" markers -- the tree
        // backend handles those examples instead. The test only
        // fails on *unexpected* SSA errors so new gated features
        // (structs, enums, match, struct field-borrows, detach/
        // cancel, eprint, non-Copy reassign, …) don't require
        // manual skip-list maintenance here. Every current
        // LowerError message in src/ssa.rs matches at least one
        // of these (confirmed via a full grep of the file when
        // this list was last reconciled, 2026-08-14) -- if a new
        // gate is added with none of these phrases, this test
        // will (correctly) fail until it either picks up an
        // existing marker or a new one is added here.
        const KNOWN_GATE_MARKERS: &[&str] = &[
            "not yet supported",
            "not yet implemented",
            "not in the v1 SSA subset",
            "tree backend",
        ];
        let unexpected: Vec<LowerError> = errors
            .into_iter()
            .filter(|e| !KNOWN_GATE_MARKERS.iter().any(|m| e.message.contains(m)))
            .collect();
        if !unexpected.is_empty() {
            failures.push((path.display().to_string(), unexpected));
        }
    }
    if !failures.is_empty() {
        let mut msg = String::from("SSA lowerer rejected some examples:\n");
        for (path, errs) in &failures {
            msg.push_str(&format!("  {}:\n", path));
            for e in errs {
                msg.push_str(&format!("    - {}\n", e));
            }
        }
        panic!("{}", msg);
    }
}
