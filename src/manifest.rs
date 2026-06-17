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

// ── Kosh registry: fetch, semver resolution, vanic add ──────────────────────

/// Sparse-index URL for the default Kosh registry.
pub const DEFAULT_REGISTRY: &str = "https://enthusiasticgeek.github.io/kosh-index";

/// Download URL template for the default registry.
/// `{name}` and `{version}` are substituted at download time.
pub const DEFAULT_DL_TEMPLATE: &str =
    "https://github.com/enthusiasticgeek/kosh-index/releases/download/{name}-v{version}/{name}-{version}.tar.gz";

/// A single version entry from the Kosh sparse index (`index/<name>.json`).
#[derive(Debug, Clone)]
pub struct RegistryEntry {
    pub name: String,
    pub version: String,
    pub cksum: String,
    pub yanked: bool,
}

/// Return value from `registry_add`.
pub struct AddResult {
    pub name: String,
    pub version: String,
    pub vendor_path: PathBuf,
}

/// Parse "X.Y.Z" → `(major, minor, patch)`. Strips pre-release suffixes.
fn parse_version(s: &str) -> Option<(u64, u64, u64)> {
    let s = s.trim().split('-').next().unwrap_or(s.trim());
    let mut it = s.split('.');
    let major: u64 = it.next()?.parse().ok()?;
    let minor: u64 = it.next()?.parse().ok()?;
    let patch: u64 = it.next()?.parse().ok()?;
    if it.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

/// True if `ver` satisfies `constraint`.
/// Supported prefixes: `^`, `~`, `>=`, `>`, `=`, none (exact), `*`.
fn version_satisfies(ver: (u64, u64, u64), constraint: &str) -> bool {
    let c = constraint.trim();
    if c.is_empty() || c == "*" {
        return true;
    }
    if let Some(rest) = c.strip_prefix('^') {
        return parse_version(rest).map_or(false, |req| {
            if req.0 > 0 {
                ver.0 == req.0 && ver >= req
            } else if req.1 > 0 {
                ver.0 == 0 && ver.1 == req.1 && ver >= req
            } else {
                ver == req
            }
        });
    }
    if let Some(rest) = c.strip_prefix('~') {
        return parse_version(rest)
            .map_or(false, |req| ver.0 == req.0 && ver.1 == req.1 && ver >= req);
    }
    if let Some(rest) = c.strip_prefix(">=") {
        return parse_version(rest.trim()).map_or(false, |req| ver >= req);
    }
    if let Some(rest) = c.strip_prefix('>') {
        return parse_version(rest.trim()).map_or(false, |req| ver > req);
    }
    if let Some(rest) = c.strip_prefix('=') {
        return parse_version(rest.trim()).map_or(false, |req| ver == req);
    }
    parse_version(c).map_or(false, |req| ver == req)
}

/// Fetch the raw text body of a URL via `curl`.
fn http_get_text(url: &str) -> Result<String, String> {
    let out = std::process::Command::new("curl")
        .args(["-fsSL", url])
        .output()
        .map_err(|e| format!("curl not found: {e}. Install curl to use registry features."))?;
    if !out.status.success() {
        let msg = String::from_utf8_lossy(&out.stderr);
        return Err(format!("curl failed for '{}': {}", url, msg.trim()));
    }
    String::from_utf8(out.stdout)
        .map_err(|e| format!("registry response is not valid UTF-8: {e}"))
}

/// Download a URL to `dest` via `curl`.
fn http_get_file(url: &str, dest: &Path) -> Result<(), String> {
    let st = std::process::Command::new("curl")
        .args(["-fsSL", "--output", &dest.to_string_lossy().into_owned(), url])
        .status()
        .map_err(|e| format!("curl not found: {e}"))?;
    if !st.success() {
        return Err(format!("curl failed downloading '{url}'"));
    }
    Ok(())
}

/// Extract `tarball.tar.gz` into `dest_dir`, stripping the top-level
/// path component (`tar --strip-components=1`).
fn extract_tar_gz(tarball: &Path, dest_dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dest_dir)
        .map_err(|e| format!("create '{}': {e}", dest_dir.display()))?;
    let st = std::process::Command::new("tar")
        .args([
            "-xzf",
            &tarball.to_string_lossy().into_owned(),
            "-C",
            &dest_dir.to_string_lossy().into_owned(),
            "--strip-components=1",
        ])
        .status()
        .map_err(|e| format!("tar not found: {e}. Install tar to use registry features."))?;
    if !st.success() {
        return Err(format!("tar extraction failed for '{}'", tarball.display()));
    }
    Ok(())
}

/// Query the Kosh sparse index for the highest version of `name`
/// that satisfies `constraint` (or the latest if `None`).
pub fn fetch_best_version(
    registry: &str,
    name: &str,
    constraint: Option<&str>,
) -> Result<RegistryEntry, String> {
    let url = format!("{}/index/{}.json", registry.trim_end_matches('/'), name);
    let body = http_get_text(&url)?;
    let mut best: Option<((u64, u64, u64), RegistryEntry)> = None;
    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let v: serde_json::Value =
            serde_json::from_str(line).map_err(|e| format!("registry: bad JSON line: {e}"))?;
        if v["yanked"].as_bool().unwrap_or(false) {
            continue;
        }
        let ver_str = v["version"].as_str().unwrap_or("").to_string();
        let cksum = v["cksum"].as_str().unwrap_or("").to_string();
        let entry_name = v["name"].as_str().unwrap_or(name).to_string();
        if let Some(ver) = parse_version(&ver_str) {
            let ok = constraint.map_or(true, |c| version_satisfies(ver, c));
            if ok && best.as_ref().map_or(true, |(bv, _)| ver > *bv) {
                best = Some((
                    ver,
                    RegistryEntry { name: entry_name, version: ver_str, cksum, yanked: false },
                ));
            }
        }
    }
    best.map(|(_, e)| e).ok_or_else(|| match constraint {
        Some(c) => format!("no version of '{name}' satisfies '{c}'"),
        None => format!("no versions of '{name}' found in registry"),
    })
}

/// Add or update a `[deps]` entry in `vani.toml`.
///
/// - Updates the existing line if the dep name is already present.
/// - Appends to the `[deps]` section if it exists but doesn't have the dep.
/// - Appends a new `[deps]` section if one doesn't exist yet.
pub fn add_dep_to_manifest(
    manifest_path: &Path,
    name: &str,
    path_rel: &str,
    version_req: Option<&str>,
) -> Result<(), String> {
    let existing = std::fs::read_to_string(manifest_path)
        .map_err(|e| format!("read '{}': {e}", manifest_path.display()))?;

    let dep_line = match version_req {
        Some(req) => format!(r#"{name} = {{ path = "{path_rel}", version = "{req}" }}"#),
        None => format!(r#"{name} = {{ path = "{path_rel}" }}"#),
    };

    let mut out: Vec<String> = Vec::new();
    let mut in_deps = false;
    let mut dep_found = false;
    let mut deps_end_pos: Option<usize> = None;

    for raw in existing.lines() {
        let t = raw.trim();
        if t == "[deps]" {
            in_deps = true;
            out.push(raw.to_string());
            continue;
        }
        if t.starts_with('[') {
            if in_deps && deps_end_pos.is_none() {
                deps_end_pos = Some(out.len());
            }
            in_deps = false;
        }
        if in_deps && !dep_found {
            let key = t.split('=').next().unwrap_or("").trim();
            if key == name {
                out.push(dep_line.clone());
                dep_found = true;
                continue;
            }
        }
        out.push(raw.to_string());
    }

    if !dep_found {
        if in_deps {
            out.push(dep_line);
        } else if let Some(pos) = deps_end_pos {
            out.insert(pos, dep_line);
        } else {
            if out.last().map_or(false, |l| !l.trim().is_empty()) {
                out.push(String::new());
            }
            out.push("[deps]".to_string());
            out.push(dep_line);
        }
    }

    let mut content = out.join("\n");
    if !content.ends_with('\n') {
        content.push('\n');
    }
    std::fs::write(manifest_path, content)
        .map_err(|e| format!("write '{}': {e}", manifest_path.display()))
}

/// Full `vanic add` operation: fetch best version, download + extract
/// tarball to `vendor/<name>/`, update `vani.toml`, rewrite `vani.lock`.
pub fn registry_add<F: Fn(&str)>(
    manifest_path: &Path,
    pkg_name: &str,
    version_constraint: Option<&str>,
    on_status: F,
) -> Result<AddResult, String> {
    on_status(&format!("  fetching {pkg_name} from registry..."));
    let entry = fetch_best_version(DEFAULT_REGISTRY, pkg_name, version_constraint)?;
    on_status(&format!("  resolved {} v{}", entry.name, entry.version));

    let dl_url = DEFAULT_DL_TEMPLATE
        .replace("{name}", &entry.name)
        .replace("{version}", &entry.version);

    let tmp_dir =
        std::env::temp_dir().join(format!("vanic-add-{}-{}", entry.name, entry.version));
    let tarball = tmp_dir.join(format!("{}-{}.tar.gz", entry.name, entry.version));
    std::fs::create_dir_all(&tmp_dir).map_err(|e| format!("temp dir: {e}"))?;

    on_status(&format!("  downloading {dl_url}..."));
    http_get_file(&dl_url, &tarball)?;

    on_status("  verifying checksum...");
    let dl_cksum = sha256_file(&tarball)?;
    if dl_cksum != entry.cksum {
        let _ = std::fs::remove_dir_all(&tmp_dir);
        return Err(format!(
            "checksum mismatch for '{}' v{}: expected {} got {}",
            entry.name, entry.version, entry.cksum, dl_cksum
        ));
    }

    let root_dir = manifest_path.parent().unwrap_or(Path::new("."));
    let vendor_dest = root_dir.join("vendor").join(&entry.name);
    on_status(&format!("  extracting to {}...", vendor_dest.display()));
    extract_tar_gz(&tarball, &vendor_dest)?;

    let constraint_str = version_constraint
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("^{}", entry.version));
    let path_rel = format!("./vendor/{}", entry.name);

    add_dep_to_manifest(manifest_path, pkg_name, &path_rel, Some(&constraint_str))?;

    let updated = load_manifest(manifest_path).map_err(|e| e.to_string())?;
    write_lockfile(&updated).map_err(|e| e.to_string())?;

    let _ = std::fs::remove_dir_all(&tmp_dir);

    Ok(AddResult { name: entry.name, version: entry.version, vendor_path: vendor_dest })
}

// ── Kosh publish ─────────────────────────────────────────────────────────────

/// Result returned by `publish_package`.
pub struct PublishResult {
    pub name: String,
    pub version: String,
    pub cksum: String,
    pub release_url: String,
}

/// Standard base64 encoding (RFC 4648, with `=` padding).
fn base64_encode(data: &[u8]) -> String {
    const T: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::with_capacity((data.len() + 2) / 3 * 4);
    for c in data.chunks(3) {
        let n = match c.len() {
            1 => (c[0] as u32) << 16,
            2 => ((c[0] as u32) << 16) | ((c[1] as u32) << 8),
            _ => ((c[0] as u32) << 16) | ((c[1] as u32) << 8) | (c[2] as u32),
        };
        s.push(T[(n >> 18 & 63) as usize] as char);
        s.push(T[(n >> 12 & 63) as usize] as char);
        s.push(if c.len() > 1 { T[(n >> 6 & 63) as usize] as char } else { '=' });
        s.push(if c.len() > 2 { T[(n & 63) as usize] as char } else { '=' });
    }
    s
}

/// Compute the SHA-256 hex digest of a file.
/// Tries `sha256sum` first (Linux/macOS/Git-Bash), then `certutil` (Windows).
fn sha256_file(path: &Path) -> Result<String, String> {
    let p = path.to_string_lossy().into_owned();
    if let Ok(out) = std::process::Command::new("sha256sum").arg(&p).output() {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout);
            if let Some(h) = s.split_whitespace().next() {
                return Ok(h.to_string());
            }
        }
    }
    let out = std::process::Command::new("certutil")
        .args(["-hashfile", &p, "SHA256"])
        .output()
        .map_err(|e| format!("sha256sum / certutil not found: {e}"))?;
    if !out.status.success() {
        return Err(format!("certutil failed on '{}'", path.display()));
    }
    let s = String::from_utf8_lossy(&out.stdout);
    for line in s.lines().skip(1) {
        let h = line.trim().replace(' ', "");
        if h.len() == 64 && h.chars().all(|c| c.is_ascii_hexdigit()) {
            return Ok(h.to_lowercase());
        }
    }
    Err("could not parse SHA-256 from certutil output".to_string())
}

/// Build `<name>-<version>.tar.gz` from `src_dir` into `out_dir`.
/// The archive top-level directory is `<name>-<version>/`.
fn build_tarball(
    src_dir: &Path,
    name: &str,
    version: &str,
    out_dir: &Path,
) -> Result<PathBuf, String> {
    let stage_name = format!("{}-{}", name, version);
    let stage_dir = out_dir.join(&stage_name);
    copy_dir_vani(src_dir, &stage_dir)?;
    let tarball = out_dir.join(format!("{}.tar.gz", stage_name));
    let st = std::process::Command::new("tar")
        .args([
            "-czf",
            &tarball.to_string_lossy().into_owned(),
            "-C",
            &out_dir.to_string_lossy().into_owned(),
            &stage_name,
        ])
        .status()
        .map_err(|e| format!("tar not found: {e}"))?;
    if !st.success() {
        return Err(format!("tar failed creating '{}'", tarball.display()));
    }
    let _ = std::fs::remove_dir_all(&stage_dir);
    Ok(tarball)
}

/// Fetch the current `index/<name>.json` from kosh-index via GitHub API.
/// Returns `(raw_content, file_sha)` if it exists, `None` if not found.
fn fetch_index_file(name: &str) -> Result<Option<(String, String)>, String> {
    let api_path = format!(
        "repos/enthusiasticgeek/kosh-index/contents/index/{}.json",
        name
    );
    let out = std::process::Command::new("gh")
        .args(["api", &api_path])
        .output()
        .map_err(|e| format!("gh not found: {e}. Install GitHub CLI to publish."))?;
    if !out.status.success() {
        return Ok(None); // 404 → file does not exist yet
    }
    let meta: serde_json::Value =
        serde_json::from_slice(&out.stdout).map_err(|e| format!("gh api JSON: {e}"))?;
    let file_sha = meta["sha"].as_str().unwrap_or("").to_string();
    let download_url = meta["download_url"].as_str().unwrap_or("").to_string();
    if download_url.is_empty() {
        return Err("gh api: missing download_url in response".to_string());
    }
    let content = http_get_text(&download_url)?;
    Ok(Some((content, file_sha)))
}

/// Push updated `index/<name>.json` to kosh-index via the GitHub Contents API.
/// If `file_sha` is `Some`, this is an update; if `None`, a new file is created.
fn push_index_update(
    name: &str,
    new_content: &str,
    file_sha: Option<&str>,
    commit_msg: &str,
) -> Result<(), String> {
    let api_path = format!(
        "repos/enthusiasticgeek/kosh-index/contents/index/{}.json",
        name
    );
    let encoded = base64_encode(new_content.as_bytes());
    let mut body = serde_json::json!({
        "message": commit_msg,
        "content": encoded,
    });
    if let Some(sha) = file_sha {
        body["sha"] = serde_json::Value::String(sha.to_string());
    }
    let body_str = body.to_string();
    let mut child = std::process::Command::new("gh")
        .args(["api", &api_path, "--method", "PUT", "--input", "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("gh not found: {e}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin
            .write_all(body_str.as_bytes())
            .map_err(|e| format!("gh stdin: {e}"))?;
    }
    let result = child.wait_with_output().map_err(|e| format!("gh wait: {e}"))?;
    if !result.status.success() {
        let msg = String::from_utf8_lossy(&result.stderr);
        return Err(format!("gh api PUT failed: {}", msg.trim()));
    }
    Ok(())
}

/// Return the date as "YYYY-MM-DD" using the system clock (no external deps).
fn today_iso() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut days = secs / 86400;
    let mut y = 1970u32;
    loop {
        let dy = if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) { 366u64 } else { 365 };
        if days < dy { break; }
        days -= dy;
        y += 1;
    }
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let mdays: [u64; 12] = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut m = 1u32;
    for &md in &mdays {
        if days < md { break; }
        days -= md;
        m += 1;
    }
    format!("{:04}-{:02}-{:02}", y, m, days + 1)
}

/// Return the authenticated GitHub username via the `gh` CLI.
fn get_gh_username() -> Result<String, String> {
    let out = std::process::Command::new("gh")
        .args(["api", "user", "--jq", ".login"])
        .output()
        .map_err(|e| format!("gh not found: {e}. Run `gh auth login` first."))?;
    if !out.status.success() {
        return Err("gh: could not identify authenticated user. Run `gh auth login`.".to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Fetch `governance.json` from the registry via the GitHub Contents API.
/// Returns `(parsed_json, file_sha)`.
fn fetch_governance_with_sha(registry: &str) -> Result<(serde_json::Value, String), String> {
    // Derive the API path from the registry URL.
    // Default registry lives in enthusiasticgeek/kosh-index.
    let api_path = if registry == DEFAULT_REGISTRY {
        "repos/enthusiasticgeek/kosh-index/contents/governance.json".to_string()
    } else {
        return Err(format!("non-default registries are not yet supported: {registry}"));
    };
    let out = std::process::Command::new("gh")
        .args(["api", &api_path])
        .output()
        .map_err(|e| format!("gh not found: {e}"))?;
    if !out.status.success() {
        return Err("could not fetch governance.json from registry".to_string());
    }
    let meta: serde_json::Value =
        serde_json::from_slice(&out.stdout).map_err(|e| format!("gh api: {e}"))?;
    let file_sha = meta["sha"].as_str().unwrap_or("").to_string();
    let download_url = meta["download_url"].as_str().unwrap_or("").to_string();
    let content = http_get_text(&download_url)?;
    let gov: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("governance.json: {e}"))?;
    Ok((gov, file_sha))
}

/// Push an updated `governance.json` back to the registry via GitHub API.
fn push_governance_update(
    gov: &serde_json::Value,
    file_sha: &str,
    commit_msg: &str,
) -> Result<(), String> {
    let api_path = "repos/enthusiasticgeek/kosh-index/contents/governance.json";
    let pretty =
        serde_json::to_string_pretty(gov).map_err(|e| format!("serialize governance: {e}"))?;
    let encoded = base64_encode(format!("{pretty}\n").as_bytes());
    let body = serde_json::json!({ "message": commit_msg, "content": encoded, "sha": file_sha });
    let body_str = body.to_string();
    let mut child = std::process::Command::new("gh")
        .args(["api", api_path, "--method", "PUT", "--input", "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("gh: {e}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin.write_all(body_str.as_bytes()).map_err(|e| format!("gh stdin: {e}"))?;
    }
    let res = child.wait_with_output().map_err(|e| format!("gh wait: {e}"))?;
    if !res.status.success() {
        let msg = String::from_utf8_lossy(&res.stderr);
        return Err(format!("gh api PUT failed: {}", msg.trim()));
    }
    Ok(())
}

/// Gate `vanic publish`: verifies the authenticated `gh` user against
/// `governance.json → allowed_publishers` and checks the blacklist.
///
/// Governance lives entirely in `governance.json` — transferring to a
/// committee or a new registry URL requires no compiler change.
fn check_publish_auth(registry: &str) -> Result<(), String> {
    let (gov, _) = fetch_governance_with_sha(registry)?;
    let current = get_gh_username()?;

    // Check blacklist first.
    if let Some(bl) = gov["blacklisted"].as_array() {
        if let Some(entry) = bl.iter().find(|v| v["username"].as_str() == Some(&current)) {
            let reason = entry["reason"].as_str().unwrap_or("policy violation");
            let since = entry["since"].as_str().unwrap_or("unknown");
            let gov_url = gov["governance_url"].as_str().unwrap_or(registry);
            return Err(format!(
                "publish rejected: '{current}' is blacklisted from this registry.\n\
                 Reason: {reason}\n\
                 Since:  {since}\n\
                 To appeal, open an issue at {gov_url}"
            ));
        }
    }

    let allowed: Vec<String> = gov["allowed_publishers"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();

    if !allowed.contains(&current) {
        let is_pending = gov["pending_publishers"]
            .as_array()
            .map(|a| a.iter().any(|v| v["username"].as_str() == Some(&current)))
            .unwrap_or(false);
        let agreement_url = gov["agreement_url"].as_str().unwrap_or("");
        let gov_url = gov["governance_url"].as_str().unwrap_or(registry);

        if is_pending {
            return Err(format!(
                "publish rejected: '{current}' has applied but is awaiting operator approval.\n\
                 You will be notified via your GitHub issue when approved.\n\
                 See {gov_url}"
            ));
        }
        return Err(format!(
            "publish rejected: '{current}' is not an authorized publisher.\n\
             To apply:\n\
               1. Read the agreement: {agreement_url}\n\
               2. Run: vanic apply-publisher --accept-agreement\n\
             Authorized: {}",
            allowed.join(", ")
        ));
    }
    Ok(())
}

/// Show the Publisher Agreement (no flag) or submit a publisher application
/// (with `--accept-agreement`).
pub fn apply_publisher(registry: &str, accept_agreement: bool) -> Result<(), String> {
    let (gov, _) = fetch_governance_with_sha(registry)?;
    let agreement_url = gov["agreement_url"].as_str().unwrap_or("");
    let agreement_version = gov["agreement_version"].as_str().unwrap_or("1.0");
    let gov_url = gov["governance_url"].as_str().unwrap_or(registry);

    if !accept_agreement {
        // Print the agreement and instructions.
        if !agreement_url.is_empty() {
            let text = http_get_text(agreement_url)?;
            println!("{text}");
        }
        println!("\n─────────────────────────────────────────────────");
        println!("To submit your application, re-run with --accept-agreement:");
        println!("  vanic apply-publisher --accept-agreement");
        return Ok(());
    }

    let current = get_gh_username()?;

    // Already approved?
    let allowed: Vec<String> = gov["allowed_publishers"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();
    if allowed.contains(&current) {
        println!("'{current}' is already an authorized publisher. No action needed.");
        return Ok(());
    }

    // Blacklisted?
    if gov["blacklisted"]
        .as_array()
        .map(|a| a.iter().any(|v| v["username"].as_str() == Some(&current)))
        .unwrap_or(false)
    {
        return Err(format!(
            "'{current}' is blacklisted from this registry.\n\
             To appeal, open an issue at {gov_url}"
        ));
    }

    // Already pending?
    if gov["pending_publishers"]
        .as_array()
        .map(|a| a.iter().any(|v| v["username"].as_str() == Some(&current)))
        .unwrap_or(false)
    {
        println!("'{current}' already has a pending application. Wait for operator approval.");
        println!("See {gov_url}");
        return Ok(());
    }

    // Create a GitHub issue recording agreement acceptance.
    let issue_title = format!("Publisher application: {current}");
    let issue_body = format!(
        "## Publisher Application\n\n\
         **GitHub username**: `{current}`  \n\
         **Agreement version accepted**: {agreement_version}  \n\
         **Agreement URL**: {agreement_url}  \n\
         **Applied**: {}  \n\n\
         I have read and agree to the Kosh Publisher Agreement in full.  \n\
         I understand that publishing malware, harmful code, or content that \
         violates the agreement will result in immediate revocation, removal \
         of my packages, and possible legal action.\n\n\
         Please review my application and add me to `allowed_publishers` in \
         `governance.json`.",
        today_iso()
    );

    let out = std::process::Command::new("gh")
        .args([
            "issue", "create",
            "--repo", "enthusiasticgeek/kosh-index",
            "--title", &issue_title,
            "--body", &issue_body,
        ])
        .output()
        .map_err(|e| format!("gh not found: {e}"))?;
    if !out.status.success() {
        let msg = String::from_utf8_lossy(&out.stderr);
        return Err(format!("failed to create GitHub issue: {}", msg.trim()));
    }
    let issue_url = String::from_utf8_lossy(&out.stdout).trim().to_string();
    println!("Application submitted: {issue_url}");
    println!("The operator will review your application and notify you via the issue.");
    Ok(())
}

/// Admin: approve a pending publisher (only operator can call this).
pub fn registry_approve(registry: &str, username: &str) -> Result<(), String> {
    check_publish_auth(registry)?; // only existing publishers (= operator) may approve
    let (mut gov, sha) = fetch_governance_with_sha(registry)?;

    // Add to allowed_publishers if absent.
    if let Some(arr) = gov["allowed_publishers"].as_array_mut() {
        if !arr.iter().any(|v| v.as_str() == Some(username)) {
            arr.push(serde_json::Value::String(username.to_string()));
        }
    }
    // Remove from pending_publishers.
    if let Some(arr) = gov["pending_publishers"].as_array_mut() {
        arr.retain(|v| v["username"].as_str() != Some(username));
    }
    // Remove from blacklisted (handles unban via approve).
    if let Some(arr) = gov["blacklisted"].as_array_mut() {
        arr.retain(|v| v["username"].as_str() != Some(username));
    }

    let msg = format!("governance: approve publisher '{username}'");
    push_governance_update(&gov, &sha, &msg)?;
    println!("'{username}' approved as a publisher.");
    Ok(())
}

/// Admin: blacklist a publisher (only operator can call this).
/// Removes the user from allowed and pending lists, adds to blacklisted.
pub fn registry_blacklist(
    registry: &str,
    username: &str,
    reason: &str,
) -> Result<(), String> {
    check_publish_auth(registry)?;
    let (mut gov, sha) = fetch_governance_with_sha(registry)?;

    if let Some(arr) = gov["allowed_publishers"].as_array_mut() {
        arr.retain(|v| v.as_str() != Some(username));
    }
    if let Some(arr) = gov["pending_publishers"].as_array_mut() {
        arr.retain(|v| v["username"].as_str() != Some(username));
    }
    if let Some(arr) = gov["blacklisted"].as_array_mut() {
        if !arr.iter().any(|v| v["username"].as_str() == Some(username)) {
            arr.push(serde_json::json!({
                "username": username,
                "reason": reason,
                "since": today_iso(),
            }));
        }
    }

    let msg = format!("governance: blacklist '{username}'");
    push_governance_update(&gov, &sha, &msg)?;
    println!("'{username}' has been blacklisted. Reason: {reason}");
    Ok(())
}

/// Publish the current package to the Kosh registry.
///
/// 1. Checks `governance.allowed_publishers` in registry `config.json`.
/// 2. Builds a `<name>-<version>.tar.gz` tarball (*.vani + vani.toml only).
/// 3. Computes its SHA-256.
/// 4. Creates a GitHub Release in `kosh-index` with the tarball as asset.
/// 5. Appends a NDJSON line to `index/<name>.json` in `kosh-index`.
pub fn publish_package<F: Fn(&str)>(
    manifest_path: &Path,
    on_status: F,
) -> Result<PublishResult, String> {
    on_status("  checking publish authorization...");
    check_publish_auth(DEFAULT_REGISTRY)?;

    let manifest = load_manifest(manifest_path).map_err(|e| e.to_string())?;
    let version = manifest.package_version.clone().ok_or_else(|| {
        "vani.toml: [package].version is required for `vanic publish`".to_string()
    })?;
    let name = manifest.package_name.clone();

    on_status(&format!("  building tarball for {} v{}...", name, version));
    let tmp = std::env::temp_dir().join(format!("vanic-publish-{}-{}", name, version));
    std::fs::create_dir_all(&tmp).map_err(|e| format!("temp dir: {e}"))?;
    let tarball = build_tarball(&manifest.root_dir, &name, &version, &tmp)?;

    on_status("  computing SHA-256...");
    let cksum = sha256_file(&tarball)?;
    on_status(&format!("  cksum: {cksum}"));

    let tag = format!("{}-v{}", name, version);
    on_status(&format!("  creating GitHub release {tag}..."));
    let rel = std::process::Command::new("gh")
        .args([
            "release",
            "create",
            &tag,
            "--repo",
            "enthusiasticgeek/kosh-index",
            "--title",
            &format!("{} v{}", name, version),
            "--notes",
            "Published via `vanic publish`.",
            &tarball.to_string_lossy().into_owned(),
        ])
        .output()
        .map_err(|e| format!("gh not found: {e}"))?;
    if !rel.status.success() {
        let msg = String::from_utf8_lossy(&rel.stderr);
        return Err(format!("gh release create failed: {}", msg.trim()));
    }
    let release_url = String::from_utf8_lossy(&rel.stdout).trim().to_string();
    on_status(&format!("  release: {release_url}"));

    on_status("  updating registry index...");
    let new_line = serde_json::json!({
        "name": name,
        "version": version,
        "deps": [],
        "cksum": cksum,
        "yanked": false,
    })
    .to_string();

    let (new_content, file_sha) = match fetch_index_file(&name)? {
        Some((mut existing, sha)) => {
            if !existing.ends_with('\n') {
                existing.push('\n');
            }
            existing.push_str(&new_line);
            existing.push('\n');
            (existing, Some(sha))
        }
        None => (format!("{new_line}\n"), None),
    };

    let commit_msg = format!("index: add {} v{}", name, version);
    push_index_update(&name, &new_content, file_sha.as_deref(), &commit_msg)?;
    on_status("  index updated.");

    let _ = std::fs::remove_dir_all(&tmp);

    Ok(PublishResult { name, version, cksum, release_url })
}

// ── Kosh remove ──────────────────────────────────────────────────────────────

/// Remove a `[deps]` entry from `vani.toml` by name.
/// Returns `true` if found and removed, `false` if the dep was not present.
pub fn remove_dep_from_manifest(manifest_path: &Path, name: &str) -> Result<bool, String> {
    let existing = std::fs::read_to_string(manifest_path)
        .map_err(|e| format!("read '{}': {e}", manifest_path.display()))?;
    let mut out: Vec<String> = Vec::new();
    let mut in_deps = false;
    let mut found = false;
    for raw in existing.lines() {
        let t = raw.trim();
        if t == "[deps]" {
            in_deps = true;
            out.push(raw.to_string());
            continue;
        }
        if t.starts_with('[') {
            in_deps = false;
        }
        if in_deps {
            let key = t.split('=').next().unwrap_or("").trim();
            if key == name {
                found = true;
                continue;
            }
        }
        out.push(raw.to_string());
    }
    if !found {
        return Ok(false);
    }
    let mut content = out.join("\n");
    if !content.ends_with('\n') {
        content.push('\n');
    }
    std::fs::write(manifest_path, content)
        .map_err(|e| format!("write '{}': {e}", manifest_path.display()))?;
    Ok(true)
}

/// Full `vanic remove` operation: removes dep from `vani.toml`, deletes
/// `vendor/<name>/`, and rewrites `vani.lock`.
pub fn registry_remove<F: Fn(&str)>(
    manifest_path: &Path,
    pkg_name: &str,
    on_status: F,
) -> Result<(), String> {
    on_status(&format!("  removing {pkg_name} from vani.toml..."));
    let removed = remove_dep_from_manifest(manifest_path, pkg_name)?;
    if !removed {
        return Err(format!("'{pkg_name}' is not in [deps]"));
    }
    let root_dir = manifest_path.parent().unwrap_or(Path::new("."));
    let vendor_dir = root_dir.join("vendor").join(pkg_name);
    if vendor_dir.exists() {
        on_status(&format!("  removing vendor/{pkg_name}/..."));
        std::fs::remove_dir_all(&vendor_dir)
            .map_err(|e| format!("remove vendor/{pkg_name}: {e}"))?;
    }
    let updated = load_manifest(manifest_path).map_err(|e| e.to_string())?;
    write_lockfile(&updated)?;
    Ok(())
}

// ── Kosh search ──────────────────────────────────────────────────────────────

/// One row in the `vanic search` output.
pub struct SearchResult {
    pub name: String,
    pub latest_version: String,
    pub version_count: usize,
    pub yanked_count: usize,
}

/// List packages from the registry, optionally filtered by a substring query.
/// Uses the GitHub Contents API to enumerate `index/*.json` files, then
/// fetches each via `download_url` (avoids base64 decode).
pub fn registry_search(
    _registry: &str,
    query: Option<&str>,
) -> Result<Vec<SearchResult>, String> {
    let api_path = "repos/enthusiasticgeek/kosh-index/contents/index/";
    let out = std::process::Command::new("gh")
        .args(["api", api_path])
        .output()
        .map_err(|e| format!("gh not found: {e}. Install GitHub CLI to search registry."))?;
    if !out.status.success() {
        let msg = String::from_utf8_lossy(&out.stderr);
        return Err(format!("gh api failed: {}", msg.trim()));
    }
    let files: serde_json::Value =
        serde_json::from_slice(&out.stdout).map_err(|e| format!("gh api JSON: {e}"))?;
    let file_arr = files.as_array().ok_or("registry index/ response is not an array")?;

    let mut results: Vec<SearchResult> = Vec::new();
    for f in file_arr {
        let fname = f["name"].as_str().unwrap_or("");
        if !fname.ends_with(".json") {
            continue;
        }
        let pkg_name = fname.trim_end_matches(".json");
        if let Some(q) = query {
            if !pkg_name.contains(q) {
                continue;
            }
        }
        let download_url = f["download_url"].as_str().unwrap_or("");
        if download_url.is_empty() {
            continue;
        }
        let content = http_get_text(download_url)?;
        let mut best: Option<(u64, u64, u64)> = None;
        let mut best_version = String::new();
        let mut version_count = 0usize;
        let mut yanked_count = 0usize;
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let v: serde_json::Value = serde_json::from_str(line)
                .map_err(|e| format!("registry: bad JSON for '{pkg_name}': {e}"))?;
            if v["yanked"].as_bool().unwrap_or(false) {
                yanked_count += 1;
                continue;
            }
            version_count += 1;
            let ver_str = v["version"].as_str().unwrap_or("");
            if let Some(ver) = parse_version(ver_str) {
                if best.is_none() || ver > best.unwrap() {
                    best = Some(ver);
                    best_version = ver_str.to_string();
                }
            }
        }
        results.push(SearchResult {
            name: pkg_name.to_string(),
            latest_version: best_version,
            version_count,
            yanked_count,
        });
    }
    Ok(results)
}

// ── Kosh update ──────────────────────────────────────────────────────────────

/// Per-dep outcome from `vanic update`.
pub struct UpdateResult {
    pub name: String,
    pub old_version: String,
    pub new_version: String,
    /// `true` when the dep was actually re-downloaded + extracted.
    pub updated: bool,
}

/// Re-resolve all registry deps to their latest allowed version.
/// Only deps whose `path_rel` starts with `"./vendor/"` are treated as
/// registry deps; path-only local deps are left untouched.
pub fn registry_update<F: Fn(&str)>(
    manifest_path: &Path,
    on_status: F,
) -> Result<Vec<UpdateResult>, String> {
    let manifest = load_manifest(manifest_path).map_err(|e| e.to_string())?;
    let root_dir = manifest_path.parent().unwrap_or(Path::new("."));
    let mut results: Vec<UpdateResult> = Vec::new();

    for dep in &manifest.deps {
        if !dep.path_rel.starts_with("./vendor/") {
            continue;
        }
        on_status(&format!("  checking {} for updates...", dep.name));
        let entry = match fetch_best_version(
            DEFAULT_REGISTRY,
            &dep.name,
            dep.version_req.as_deref(),
        ) {
            Ok(e) => e,
            Err(e) => {
                on_status(&format!("  warning: could not fetch {}: {e}", dep.name));
                continue;
            }
        };

        let old_version =
            dep.resolved_version.clone().unwrap_or_else(|| "0.0.0".to_string());
        let old = parse_version(&old_version).unwrap_or((0, 0, 0));
        let new = parse_version(&entry.version).unwrap_or((0, 0, 0));

        if new <= old {
            on_status(&format!("  {} v{} is up-to-date", dep.name, old_version));
            results.push(UpdateResult {
                name: dep.name.clone(),
                old_version,
                new_version: entry.version,
                updated: false,
            });
            continue;
        }

        on_status(&format!(
            "  updating {} v{} → v{}...",
            dep.name, old_version, entry.version
        ));

        let dl_url = DEFAULT_DL_TEMPLATE
            .replace("{name}", &entry.name)
            .replace("{version}", &entry.version);
        let tmp_dir = std::env::temp_dir()
            .join(format!("vanic-update-{}-{}", entry.name, entry.version));
        let tarball =
            tmp_dir.join(format!("{}-{}.tar.gz", entry.name, entry.version));
        std::fs::create_dir_all(&tmp_dir).map_err(|e| format!("temp dir: {e}"))?;
        http_get_file(&dl_url, &tarball)?;

        on_status("  verifying checksum...");
        let dl_cksum = sha256_file(&tarball)?;
        if dl_cksum != entry.cksum {
            let _ = std::fs::remove_dir_all(&tmp_dir);
            return Err(format!(
                "checksum mismatch for '{}' v{}: expected {} got {}",
                dep.name, entry.version, entry.cksum, dl_cksum
            ));
        }

        let vendor_dest = root_dir.join("vendor").join(&entry.name);
        if vendor_dest.exists() {
            std::fs::remove_dir_all(&vendor_dest)
                .map_err(|e| format!("remove old vendor/{}: {e}", entry.name))?;
        }
        on_status(&format!("  extracting to {}...", vendor_dest.display()));
        extract_tar_gz(&tarball, &vendor_dest)?;

        let constraint_str = dep.version_req.clone()
            .unwrap_or_else(|| format!("^{}", entry.version));
        add_dep_to_manifest(manifest_path, &dep.name, &dep.path_rel, Some(&constraint_str))?;
        let _ = std::fs::remove_dir_all(&tmp_dir);

        results.push(UpdateResult {
            name: dep.name.clone(),
            old_version,
            new_version: entry.version,
            updated: true,
        });
    }

    let updated_manifest = load_manifest(manifest_path).map_err(|e| e.to_string())?;
    write_lockfile(&updated_manifest)?;
    Ok(results)
}

// ── end Kosh registry ────────────────────────────────────────────────────────

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
    fn parse_version_three_parts() {
        assert_eq!(parse_version("1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_version("0.0.0"), Some((0, 0, 0)));
        assert_eq!(parse_version("1.2.3-alpha"), Some((1, 2, 3)));
        assert_eq!(parse_version("1.2"), None);
        assert_eq!(parse_version("1.2.3.4"), None);
    }

    #[test]
    fn version_satisfies_caret() {
        assert!(version_satisfies((1, 2, 4), "^1.2.3"));
        assert!(version_satisfies((1, 9, 0), "^1.2.3"));
        assert!(!version_satisfies((2, 0, 0), "^1.2.3"));
        assert!(!version_satisfies((1, 2, 2), "^1.2.3"));
        // zero major: ^0.2.3 means >=0.2.3, <0.3.0
        assert!(version_satisfies((0, 2, 5), "^0.2.3"));
        assert!(!version_satisfies((0, 3, 0), "^0.2.3"));
    }

    #[test]
    fn version_satisfies_tilde() {
        assert!(version_satisfies((1, 2, 5), "~1.2.3"));
        assert!(!version_satisfies((1, 3, 0), "~1.2.3"));
        assert!(!version_satisfies((1, 2, 2), "~1.2.3"));
    }

    #[test]
    fn version_satisfies_exact_and_ge() {
        assert!(version_satisfies((1, 2, 3), "=1.2.3"));
        assert!(!version_satisfies((1, 2, 4), "=1.2.3"));
        assert!(version_satisfies((1, 2, 3), "1.2.3"));
        assert!(version_satisfies((2, 0, 0), ">=1.2.3"));
        assert!(version_satisfies((1, 2, 3), ">=1.2.3"));
        assert!(!version_satisfies((1, 2, 2), ">=1.2.3"));
        assert!(version_satisfies((1, 0, 0), "*"));
    }

    #[test]
    fn add_dep_creates_deps_section() {
        let dir = std::env::temp_dir().join(format!(
            "vani-adddep-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let mf = dir.join("vani.toml");
        std::fs::write(
            &mf,
            "[package]\nname = \"myapp\"\nversion = \"0.1.0\"\nentry = \"src/main.vani\"\n",
        )
        .unwrap();
        add_dep_to_manifest(&mf, "mathlib", "./vendor/mathlib", Some("^1.0")).unwrap();
        let content = std::fs::read_to_string(&mf).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        assert!(content.contains("[deps]"), "should have [deps] section");
        assert!(
            content.contains("mathlib = { path = \"./vendor/mathlib\", version = \"^1.0\" }"),
            "got: {content}"
        );
    }

    #[test]
    fn add_dep_updates_existing_entry() {
        let dir = std::env::temp_dir().join(format!(
            "vani-adddep-upd-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let mf = dir.join("vani.toml");
        std::fs::write(
            &mf,
            "[package]\nname = \"myapp\"\nentry = \"src/main.vani\"\n\n[deps]\nmathlib = { path = \"../old\" }\n",
        )
        .unwrap();
        add_dep_to_manifest(&mf, "mathlib", "./vendor/mathlib", Some("^1.2")).unwrap();
        let content = std::fs::read_to_string(&mf).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        let count = content.matches("mathlib =").count();
        assert_eq!(count, 1, "should have exactly one mathlib entry, got:\n{content}");
        assert!(
            content.contains("./vendor/mathlib"),
            "should have updated path, got:\n{content}"
        );
    }

    #[test]
    fn remove_dep_removes_from_deps_section() {
        let dir = std::env::temp_dir().join(format!(
            "vani-remove-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let mf = dir.join("vani.toml");
        std::fs::write(
            &mf,
            "[package]\nname = \"myapp\"\nentry = \"src/main.vani\"\n\n[deps]\nmathlib = { path = \"./vendor/mathlib\", version = \"^1.0\" }\nparser = { path = \"./vendor/parser\", version = \"^2.0\" }\n",
        )
        .unwrap();
        let found = remove_dep_from_manifest(&mf, "mathlib").unwrap();
        let content = std::fs::read_to_string(&mf).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        assert!(found, "should return true for found dep");
        assert!(!content.contains("mathlib"), "should not contain mathlib: {content}");
        assert!(content.contains("parser"), "should still contain parser: {content}");
    }

    #[test]
    fn remove_dep_returns_false_when_not_found() {
        let dir = std::env::temp_dir().join(format!(
            "vani-remove-nf-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let mf = dir.join("vani.toml");
        std::fs::write(
            &mf,
            "[package]\nname = \"myapp\"\nentry = \"src/main.vani\"\n",
        )
        .unwrap();
        let found = remove_dep_from_manifest(&mf, "nonexistent").unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        assert!(!found, "should return false for missing dep");
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
