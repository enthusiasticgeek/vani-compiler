# Kosh package manager — design notes

**Status**: ✅ MVP shipped 2026-06-17.
Registry live at `https://enthusiasticgeek.github.io/kosh-index/`.

`Kosh` = कोश ("treasure / repository"). The package manager for
vāṇी, implemented as Arc 9 in [TODO.md](../TODO.md).

---

## What ships (2026-06-17)

| Feature | Status | Commit |
|---|---|---|
| `vani.toml` manifest (`[package]` + `[deps]`) | ✅ | pre-2026-06-16 |
| `[package].version` field | ✅ | `4a225ee` |
| `[deps]` version constraints (`^1.0`, `~1.2`, `>=1.0`) | ✅ | `4a225ee` |
| `vani.lock` writer + staleness check | ✅ | `4a225ee` |
| `vanic vendor` | ✅ | `4a225ee` |
| `kosh-index` repo + GitHub Pages + `config.json` | ✅ | kosh-index `14e90e0` |
| `vanic add <name>[@constraint]` | ✅ | `4bf72c2` |
| `vanic publish` | ✅ | `6e0ac44` |
| Publish gate (`governance.allowed_publishers`) | ✅ | `3897371` |

---

## Architecture

```
enthusiasticgeek/kosh-index   (public GitHub repo)
  config.json                 ← dl template, api, governance block
  index/
    <name>.json               ← one NDJSON line per published version

GitHub Pages: https://enthusiasticgeek.github.io/kosh-index/
Tarballs:     GitHub Release assets inside kosh-index
```

### `config.json`

```json
{
  "dl":   "https://github.com/enthusiasticgeek/kosh-index/releases/download/{name}-v{version}/{name}-{version}.tar.gz",
  "api":  "https://enthusiasticgeek.github.io/kosh-index",
  "auth-required": false,
  "governance": {
    "allowed_publishers": ["enthusiasticgeek"],
    "governance_url":     "https://github.com/enthusiasticgeek/kosh-index",
    "note": "v1: enthusiasticgeek is the sole write authority. Registry URL and governance model will change when vani adoption warrants a committee-managed standalone domain."
  }
}
```

### Index entry format (`index/<name>.json`)

One JSON object per line (NDJSON). Each line is one published version:

```json
{"name":"mathlib","version":"1.0.0","deps":[],"cksum":"<sha256>","yanked":false}
```

---

## `vani.toml` format

```toml
[package]
name    = "myapp"
version = "0.1.0"        # required for publish; optional otherwise
entry   = "src/main.vani"

[deps]
# path dep (local development):
mathlib = { path = "../math-lib" }
# path dep + version pin (verifies dep's declared version):
utils   = { path = "../utils", version = "^1.0" }
# registry dep (added via vanic add):
parser  = { path = "./vendor/parser", version = "^2.1" }
```

---

## CLI surface

| Command | Effect | Shipped |
|---|---|---|
| `vanic build` / `run` / `check` | existing driver | ✅ |
| `vanic vendor` | copies path-dep source trees to `vendor/` | ✅ |
| `vanic add <name>[@constraint]` | fetches from registry → `vendor/` → updates `vani.toml` + `vani.lock` | ✅ |
| `vanic publish` | build tarball → auth gate → GH Release → index append | ✅ |
| `vanic remove <name>` | removes from `[deps]` + updates lockfile | future |
| `vanic search <q>` | queries registry | future |
| `vanic update` | re-resolves all registry deps | future |

---

## Semver constraints

| Syntax | Meaning |
|---|---|
| `"^1.2.3"` | `>=1.2.3, <2.0.0` (same major) |
| `"~1.2.3"` | `>=1.2.3, <1.3.0` (same minor) |
| `">=1.2.3"` | at least |
| `"=1.2.3"` | exact |
| `"1.2.3"` | exact (no prefix = same as `=`) |
| `"*"` | any (latest) |

---

## Governance & security

### Publish gate (2026-06-17)

`vanic publish` fetches `config.json` from the registry and reads
`governance.allowed_publishers`. The authenticated `gh` user must
appear in that list, otherwise publish is rejected with a clear error:

```
publish rejected: 'bob' is not an authorized publisher for this registry.
Authorized: enthusiasticgeek
See https://github.com/enthusiasticgeek/kosh-index for governance details.
```

The allowlist lives **entirely in the registry's `config.json`**. When
governance transfers to a committee, or the registry moves to a new
domain, only that file changes — no compiler update required.

### Future governance path

| Phase | Trigger | Action |
|---|---|---|
| v1 (now) | 0 external users | Sole authority: `enthusiasticgeek` |
| v2 | First external contributors | Add co-maintainers to `allowed_publishers` |
| v3 | Broader adoption | Move to `kosh.vani-lang.org`; committee CODEOWNERS; cryptographic signing |

### Tamper evidence

SHA-256 checksums are recorded in `vani.lock`. Future: verify
checksum at `vanic add` download time (not yet implemented).

---

## Registry design decisions

### Q1. Hosting model

**GitHub Pages sparse index** (Cargo RFC 2789 format). Chosen for:
- Zero cost, zero infrastructure.
- Migration: change one URL in `config.json`; format identical everywhere.

Rejected: git-only (Go modules style), hosted server (crates.io style — running cost), IPFS (immature tooling).

### Q2. Namespace authority

First-come first-served. `enthusiasticgeek` is sole write authority in v1.
SHA-256 checksums in `vani.lock` provide tamper evidence without signing.

### Q3. Audit flag

`"verified": true` in an index entry means all exported items carry
`requires` / `ensures` contracts that discharge under Z3. Informational only in v1.

### Q4. Mirror policy

The index is a public git repo — anyone can fork. `vani.toml` accepts
`registry+<url>` so private / mirror registries need no compiler change.

---

## Legal note

vāṇी keyword spellings that resemble Rust keywords (`unwrap`, `match`,
`struct`, `fn`, etc.) are **not** trademark-protected. The Rust Foundation
trademark covers the name "Rust", the Rust logo, "Cargo", and "crates.io" —
not keywords or stdlib function names. *Oracle v. Google* (2021) settled the
API-naming question for function names. No issue. (2026-06-16)

---

## See also

- [`src/manifest.rs`](../src/manifest.rs) — manifest parser + Kosh registry code.
- [`kosh-index`](https://github.com/enthusiasticgeek/kosh-index) — live registry.
- [TODO.md](../TODO.md) — Kosh arc status.
- [Closure #287 in STATUS.md](../STATUS.md) — manifest path-style v2.
