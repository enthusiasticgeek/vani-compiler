//! `vani.toml` manifest parser + `vani.lock` writer.
//!
//! Kosh pre-registry arc (2026-06-16):
//!   - `[package].version` — optional semver string
//!   - `[deps]` inline tables accept `path` + optional `version`
//!     constraint (`"^1.0"`, `"~1.2.3"`, `">=1.0, <2.0"`)
//!   - `write_lockfile` — writes `vani.lock` next to `vani.toml`
//!   - `lockfile_is_stale` — true when lock is absent or older
//!     than the manifest
//!   - `vendor_deps` — copies dep source trees to `vendor/`
//!
//! Registry-dependent steps (resolver, `vanic add`, `vanic publish`)
//! ship once the Kosh index repo is created.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Parsed `vani.toml` manifest.
#[derive(Debug, Clone, PartialEq)]
pub struct Manifest {
    pub package_name: String,
    /// Optional semver version string from `[package].version`.
    pub package_version: Option<String>,
    /// Absolute path to the entry `.vani` file.
    pub entry_path: PathBuf,
    /// Directory containing `vani.toml`.
    pub root_dir: PathBuf,
    /// Absolute path to `vani.toml` itself (for lockfile staleness).
    pub manifest_path: PathBuf,
    pub deps: Vec<Dependency>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Dependency {
    /// Name as written in `[deps]`.
    pub name: String,
    /// Absolute path to the dep's entry `.vani` source.
    pub entry_path: PathBuf,
    /// Root directory of the dep package.
    pub root_dir: PathBuf,
    /// Relative path string as written in the manifest (`{ path = "..." }`).
    pub path_rel: String,
    /// Optional version constraint from `{ version = "^1.0" }`.
    pub version_req: Option<String>,
    /// Version advertised by the dep's own `[package].version` (may be None).
    pub resolved_version: Option<String>,
}

#[derive(Debug)]
pub enum ManifestError {
    Io(String),
    Parse { line: usize, message: String },
    MissingField { section: String, key: String },
    UnknownSection(String),
}

impl std::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ManifestError::Io(m) => write!(f, "vani.toml: {}", m),
            ManifestError::Parse { line, message } => {
                write!(f, "vani.toml:{}: {}", line, message)
            }
            ManifestError::MissingField { section, key } => write!(
                f,
                "vani.toml: missing required `{}` in [{}] section",
                key, section
            ),
            ManifestError::UnknownSection(name) => {
                write!(f, "vani.toml: unknown section [{}]", name)
            }
        }
    }
}

/// Locate the nearest `vani.toml` by walking up from `start`
/// until either found or the filesystem root is reached.
/// Returns `None` if no manifest is found.
pub fn find_manifest(start: &Path) -> Option<PathBuf> {
    let mut cur: Option<&Path> = if start.is_file() {
        start.parent()
    } else {
        Some(start)
    };
    while let Some(dir) = cur {
        let candidate = dir.join("vani.toml");
        if candidate.is_file() {
            return Some(candidate);
        }
        cur = dir.parent();
    }
    None
}

pub fn load_manifest(manifest_path: &Path) -> Result<Manifest, ManifestError> {
    let source = std::fs::read_to_string(manifest_path).map_err(|e| {
        ManifestError::Io(format!("read '{}': {}", manifest_path.display(), e))
    })?;
    let (sections, dep_entries) = parse_toml_minimal(&source)?;
    let pkg = sections.get("package").ok_or(ManifestError::MissingField {
        section: "package".into(),
        key: "package".into(),
    })?;
    let name = pkg
        .get("name")
        .ok_or_else(|| ManifestError::MissingField {
            section: "package".into(),
            key: "name".into(),
        })?
        .clone();
    let entry_rel = pkg
        .get("entry")
        .ok_or_else(|| ManifestError::MissingField {
            section: "package".into(),
            key: "entry".into(),
        })?
        .clone();
    let package_version = pkg.get("version").cloned();
    let root_dir = manifest_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    let entry_path = root_dir.join(entry_rel);
    let mut deps: Vec<Dependency> = Vec::new();
    for (dep_name, dep_path_rel, version_req) in &dep_entries {
        let dep_dir = root_dir.join(dep_path_rel);
        let dep_manifest_path = dep_dir.join("vani.toml");
        let dep_loaded = load_manifest(&dep_manifest_path).map_err(|e| {
            ManifestError::Io(format!(
                "loading dep '{}' from '{}': {}",
                dep_name, dep_manifest_path.display(), e
            ))
        })?;
        deps.push(Dependency {
            name: dep_name.clone(),
            entry_path: dep_loaded.entry_path,
            root_dir: dep_loaded.root_dir,
            path_rel: dep_path_rel.clone(),
            version_req: version_req.clone(),
            resolved_version: dep_loaded.package_version,
        });
    }
    Ok(Manifest {
        package_name: name,
        package_version,
        entry_path,
        root_dir,
        manifest_path: manifest_path.to_path_buf(),
        deps,
    })
}

/// Returns true when `vani.lock` is absent or older than `vani.toml`.
pub fn lockfile_is_stale(manifest_path: &Path) -> bool {
    let lock_path = manifest_path.with_file_name("vani.lock");
    if !lock_path.exists() {
        return true;
    }
    let mf_mtime = std::fs::metadata(manifest_path)
        .and_then(|m| m.modified())
        .ok();
    let lk_mtime = std::fs::metadata(&lock_path)
        .and_then(|m| m.modified())
        .ok();
    match (mf_mtime, lk_mtime) {
        (Some(mf), Some(lk)) => mf > lk,
        _ => true,
    }
}

/// Write (or overwrite) `vani.lock` next to `vani.toml`.
///
/// Format mirrors Cargo.lock enough that the same tooling habits
/// apply, but uses `vani.lock` as the filename.
pub fn write_lockfile(manifest: &Manifest) -> Result<(), String> {
    let lock_path = manifest.manifest_path.with_file_name("vani.lock");
    let mut out = String::new();
    out.push_str("# This file is @generated by vanic.\n");
    out.push_str("# It is not intended for manual editing.\n");
    out.push_str("version = 1\n");

    // Root package entry.
    out.push_str("\n[[package]]\n");
    out.push_str(&format!("name = \"{}\"\n", manifest.package_name));
    if let Some(v) = &manifest.package_version {
        out.push_str(&format!("version = \"{}\"\n", v));
    } else {
        out.push_str("version = \"0.0.0\"\n");
    }

    // One entry per path-dep.
    for dep in &manifest.deps {
        out.push_str("\n[[package]]\n");
        out.push_str(&format!("name = \"{}\"\n", dep.name));
        if let Some(v) = &dep.resolved_version {
            out.push_str(&format!("version = \"{}\"\n", v));
        } else {
            out.push_str("version = \"0.0.0\"\n");
        }
        out.push_str("source = \"local\"\n");
        out.push_str(&format!("path = \"{}\"\n", dep.path_rel));
        if let Some(req) = &dep.version_req {
            out.push_str(&format!("version-req = \"{}\"\n", req));
        }
    }

    std::fs::write(&lock_path, &out)
        .map_err(|e| format!("failed to write vani.lock: {}", e))
}

/// Copy each dep's source tree into `vendor/<dep-name>/` under
/// the manifest's root directory. Returns a list of (name, dest)
/// pairs for each dep that was vendored.
///
/// Skips copying if the dep's root_dir is already inside `vendor/`
/// (idempotent re-vendor).
pub fn vendor_deps(manifest: &Manifest) -> Result<Vec<(String, PathBuf)>, String> {
    let vendor_dir = manifest.root_dir.join("vendor");
    std::fs::create_dir_all(&vendor_dir)
        .map_err(|e| format!("failed to create vendor/: {}", e))?;

    let mut vendored: Vec<(String, PathBuf)> = Vec::new();
    for dep in &manifest.deps {
        let dest = vendor_dir.join(&dep.name);
        // Skip if the dep is already inside vendor/ (idempotent).
        if dep.root_dir.starts_with(&vendor_dir) {
            vendored.push((dep.name.clone(), dest));
            continue;
        }
        copy_dir_vani(&dep.root_dir, &dest)
            .map_err(|e| format!("vendoring '{}': {}", dep.name, e))?;
        vendored.push((dep.name.clone(), dest));
    }
    Ok(vendored)
}

/// Recursively copy `*.vani` files and `vani.toml` from `src` to `dst`.
fn copy_dir_vani(src: &Path, dst: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dst)
        .map_err(|e| format!("create '{}': {}", dst.display(), e))?;
    let entries = std::fs::read_dir(src)
        .map_err(|e| format!("read dir '{}': {}", src.display(), e))?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if path.is_dir() {
            // Skip hidden dirs and the vendor dir itself to avoid cycles.
            if name_str.starts_with('.') || name_str == "vendor" || name_str == "target" {
                continue;
            }
            copy_dir_vani(&path, &dst.join(&name))?;
        } else if name_str.ends_with(".vani") || name_str == "vani.toml" {
            let dest_file = dst.join(&name);
            std::fs::copy(&path, &dest_file)
                .map_err(|e| format!("copy '{}' -> '{}': {}", path.display(), dest_file.display(), e))?;
        }
    }
    Ok(())
}

/// Parse a TOML inline-table body (the part between `{` and `}`).
/// Handles comma-separated `key = "value"` pairs. Values must be
/// quoted strings. Returns a map of key → value.
fn parse_inline_table(
    inner: &str,
    line: usize,
) -> Result<HashMap<String, String>, ManifestError> {
    let mut map = HashMap::new();
    for part in inner.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let eq = part.find('=').ok_or_else(|| ManifestError::Parse {
            line,
            message: format!("inline table: expected `key = \"value\"`, got `{}`", part),
        })?;
        let k = part[..eq].trim().to_string();
        let v_raw = part[eq + 1..].trim();
        if !v_raw.starts_with('"') {
            return Err(ManifestError::Parse {
                line,
                message: format!("inline table key `{}`: value must be a quoted string", k),
            });
        }
        let after_open = &v_raw[1..];
        let qclose = after_open.find('"').ok_or_else(|| ManifestError::Parse {
            line,
            message: format!("inline table key `{}`: missing closing quote", k),
        })?;
        map.insert(k, after_open[..qclose].to_string());
    }
    Ok(map)
}

// Minimal TOML subset parser.
//
// Kosh pre-registry: now accepts multi-key inline tables in [deps]:
//   math = { path = "../math-lib", version = "^1.0" }
// and `[package].version = "0.1.0"`.
//
// Returns the regular section map plus a Vec of
// (dep_name, dep_path_rel, Option<version_req>) triples.
fn parse_toml_minimal(
    source: &str,
) -> Result<
    (
        HashMap<String, HashMap<String, String>>,
        Vec<(String, String, Option<String>)>,
    ),
    ManifestError,
> {
    let mut sections: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut deps: Vec<(String, String, Option<String>)> = Vec::new();
    let mut current_section: Option<String> = None;
    for (lineno_zero, raw) in source.lines().enumerate() {
        let line_no = lineno_zero + 1;
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('[') {
            let end = trimmed.find(']').ok_or(ManifestError::Parse {
                line: line_no,
                message: "section header missing closing `]`".into(),
            })?;
            let name = trimmed[1..end].trim();
            match name {
                "package" | "deps" => {}
                other => return Err(ManifestError::UnknownSection(other.into())),
            }
            current_section = Some(name.to_string());
            sections.entry(name.to_string()).or_default();
            continue;
        }
        let eq = trimmed.find('=').ok_or(ManifestError::Parse {
            line: line_no,
            message: "expected `key = …`".into(),
        })?;
        let key = trimmed[..eq].trim().to_string();
        let value_raw = trimmed[eq + 1..].trim();
        let section = current_section.as_deref().ok_or(ManifestError::Parse {
            line: line_no,
            message: "key/value outside any [section]".into(),
        })?;
        // [deps] entries: `name = { path = "...", version = "^1.0" }`
        if section == "deps" && value_raw.starts_with('{') {
            let close = value_raw.rfind('}').ok_or(ManifestError::Parse {
                line: line_no,
                message: "inline table missing closing `}`".into(),
            })?;
            let inner = &value_raw[1..close];
            let kv = parse_inline_table(inner, line_no)?;
            let path_val = kv.get("path").ok_or_else(|| ManifestError::Parse {
                line: line_no,
                message: format!("deps `{}`: inline table must contain `path = \"...\"`", key),
            })?.clone();
            // Reject unrecognised keys (future-proofing).
            for k in kv.keys() {
                if k != "path" && k != "version" {
                    return Err(ManifestError::Parse {
                        line: line_no,
                        message: format!(
                            "deps `{}`: unrecognised key `{}`; valid keys are `path`, `version`",
                            key, k
                        ),
                    });
                }
            }
            let version_req = kv.get("version").cloned();
            deps.push((key, path_val, version_req));
            continue;
        }
        if !value_raw.starts_with('"') {
            return Err(ManifestError::Parse {
                line: line_no,
                message: format!(
                    "value of `{}` must be a quoted string (or an inline \
                     table `{{ path = \"...\" }}` in [deps])",
                    key
                ),
            });
        }
        // Strip trailing comment after the closing quote.
        let after_open = &value_raw[1..];
        let close = after_open.find('"').ok_or(ManifestError::Parse {
            line: line_no,
            message: "string value missing closing `\"`".into(),
        })?;
        let value = after_open[..close].to_string();
        sections
            .entry(section.to_string())
            .or_default()
            .insert(key, value);
    }
    Ok((sections, deps))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_manifest() {
        let s = r#"
            [package]
            name = "my_project"
            entry = "src/main.vani"
        "#;
        let (sections, _deps) = parse_toml_minimal(s).expect("parses");
        assert_eq!(sections["package"]["name"], "my_project");
        assert_eq!(sections["package"]["entry"], "src/main.vani");
    }

    #[test]
    fn tolerates_comments_and_blank_lines() {
        let s = "\
            # leading comment\n\
            \n\
            [package]\n\
            # mid-section\n\
            name = \"x\"\n\
            entry = \"e.vani\"  # trailing comment\n\
        ";
        let (sections, _deps) = parse_toml_minimal(s).expect("parses");
        assert_eq!(sections["package"]["entry"], "e.vani");
    }

    #[test]
    fn rejects_unknown_section() {
        let s = "[unknown]\nfoo = \"bar\"\n";
        let err = parse_toml_minimal(s).expect_err("rejects");
        assert!(matches!(err, ManifestError::UnknownSection(ref n) if n == "unknown"));
    }

    #[test]
    fn rejects_non_string_value() {
        let s = "[package]\nname = 42\n";
        let err = parse_toml_minimal(s).expect_err("rejects");
        match err {
            ManifestError::Parse { message, .. } => {
                assert!(message.contains("quoted string"));
            }
            _ => panic!("expected Parse error, got {:?}", err),
        }
    }

    #[test]
    fn rejects_kv_outside_section() {
        let s = "name = \"x\"\n";
        let err = parse_toml_minimal(s).expect_err("rejects");
        match err {
            ManifestError::Parse { message, .. } => {
                assert!(message.contains("outside any"));
            }
            _ => panic!("expected Parse error, got {:?}", err),
        }
    }

    #[test]
    fn load_manifest_surfaces_missing_entry() {
        let dir = std::env::temp_dir().join(format!(
            "vani-manifest-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let mf = dir.join("vani.toml");
        std::fs::write(&mf, "[package]\nname = \"x\"\n").unwrap();
        let err = load_manifest(&mf).expect_err("missing entry");
        let _ = std::fs::remove_dir_all(&dir);
        assert!(matches!(
            err,
            ManifestError::MissingField { ref key, .. } if key == "entry"
        ));
    }

    #[test]
    fn parses_deps_inline_table() {
        let s = r#"
            [package]
            name = "x"
            entry = "e.vani"

            [deps]
            mathlib = { path = "../math" }
            other = { path = "../other-lib" }
        "#;
        let (_sections, deps) = parse_toml_minimal(s).expect("parses");
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].0, "mathlib");
        assert_eq!(deps[0].1, "../math");
        assert_eq!(deps[0].2, None);
        assert_eq!(deps[1].0, "other");
        assert_eq!(deps[1].1, "../other-lib");
        assert_eq!(deps[1].2, None);
    }

    #[test]
    fn parses_deps_with_version_constraint() {
        let s = r#"
            [package]
            name = "x"
            version = "0.2.0"
            entry = "e.vani"

            [deps]
            mathlib = { path = "../math", version = "^1.0" }
            other = { path = "../other-lib" }
        "#;
        let (sections, deps) = parse_toml_minimal(s).expect("parses");
        assert_eq!(sections["package"]["version"], "0.2.0");
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].0, "mathlib");
        assert_eq!(deps[0].1, "../math");
        assert_eq!(deps[0].2.as_deref(), Some("^1.0"));
        assert_eq!(deps[1].2, None);
    }

    #[test]
    fn rejects_unknown_key_in_inline_table() {
        let s = r#"
            [package]
            name = "x"
            entry = "e.vani"

            [deps]
            mathlib = { path = "../math", registry = "kosh" }
        "#;
        let err = parse_toml_minimal(s).expect_err("rejects unknown key");
        match err {
            ManifestError::Parse { message, .. } => {
                assert!(message.contains("unrecognised key"), "got: {}", message);
            }
            _ => panic!("expected Parse error, got {:?}", err),
        }
    }

    #[test]
    fn rejects_deps_without_path() {
        let s = r#"
            [package]
            name = "x"
            entry = "e.vani"

            [deps]
            mathlib = { version = "1.0" }
        "#;
        let err = parse_toml_minimal(s).expect_err("rejects missing path");
        match err {
            ManifestError::Parse { message, .. } => {
                assert!(message.contains("path"), "got: {}", message);
            }
            _ => panic!("expected Parse error, got {:?}", err),
        }
    }

    #[test]
    fn find_manifest_walks_up() {
        let dir = std::env::temp_dir().join(format!(
            "vani-find-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let sub = dir.join("nested/deep");
        std::fs::create_dir_all(&sub).unwrap();
        let mf = dir.join("vani.toml");
        std::fs::write(&mf, "[package]\nname = \"x\"\nentry = \"e.vani\"\n").unwrap();
        let found = find_manifest(&sub).expect("finds via parent walk");
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(found, mf);
    }
}
