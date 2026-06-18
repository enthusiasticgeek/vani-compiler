# Kosh package manager — design notes

**Status**: ✅ MVP shipped 2026-06-17. vāṇी `0.1.0` released 2026-06-18.
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
| `governance.json` (separate publisher management file) | ✅ | kosh-index `ab3ffb3` |
| Publisher Agreement v1.0 + `apply-publisher` command | ✅ | `ab3ffb3` |
| `registry-approve` / `registry-blacklist` operator commands | ✅ | `ab3ffb3` |
| Checksum verification at `vanic add` time (SHA-256) | ✅ | 2026-06-17 |
| `vanic remove <name>` | ✅ | 2026-06-17 |
| `vanic search [<query>]` | ✅ | 2026-06-17 |
| `vanic update` | ✅ | 2026-06-17 |

---

## Architecture

```
enthusiasticgeek/kosh-index   (public GitHub repo)
  config.json                 ← dl template, api URL
  governance.json             ← publisher management (allowlist, blacklist, agreement)
  PUBLISHER_AGREEMENT.md      ← legal agreement all publishers must accept
  PUBLISHING.md               ← how-to guide for new publishers
  index/
    <name>.json               ← one NDJSON line per published version

GitHub Pages: https://enthusiasticgeek.github.io/kosh-index/
Tarballs:     GitHub Release assets inside kosh-index
```

### `config.json`

Technical registry configuration only — no governance data here.

```json
{
  "dl":  "https://github.com/enthusiasticgeek/kosh-index/releases/download/{name}-v{version}/{name}-{version}.tar.gz",
  "api": "https://enthusiasticgeek.github.io/kosh-index",
  "auth-required": false
}
```

### `governance.json`

Publisher management — lives separately so governance can be updated
(committee takeover, new domain) without any compiler change.

```json
{
  "version": 1,
  "agreement_version": "1.0",
  "agreement_url":   "https://enthusiasticgeek.github.io/kosh-index/PUBLISHER_AGREEMENT.md",
  "governance_url":  "https://github.com/enthusiasticgeek/kosh-index",
  "allowed_publishers": ["enthusiasticgeek"],
  "pending_publishers": [],
  "blacklisted": []
}
```

`blacklisted` entries carry `username`, `reason`, and `since` (ISO date).

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
| `vanic apply-publisher [--accept-agreement]` | fetch + display publisher agreement; with flag: submit GitHub issue to apply | ✅ |
| `vanic registry-approve <username>` | operator: approve a pending publisher (adds to `allowed_publishers`) | ✅ |
| `vanic registry-blacklist <username> --reason=<text>` | operator: blacklist a publisher (removes from allowed + blocks future publish) | ✅ |
| `vanic remove <name>` | remove from `[deps]`, delete `vendor/<name>/`, rewrite `vani.lock` | ✅ |
| `vanic search [<query>]` | list all packages in registry, or filter by name substring | ✅ |
| `vanic update` | re-resolve all registry deps to latest compatible version; verifies SHA-256 | ✅ |

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

Publishing is a two-step gated process:

**Step 1 — Publisher agreement.** Before applying, a prospective publisher
reads the [PUBLISHER_AGREEMENT.md](https://enthusiasticgeek.github.io/kosh-index/PUBLISHER_AGREEMENT.md)
(fetched by `vanic apply-publisher`) and formally accepts it:

```
vanic apply-publisher --accept-agreement
```

This creates a public GitHub issue in `kosh-index` recording the acceptance.

**Step 2 — Operator approval.** The registry operator (`enthusiasticgeek`)
reviews the application and either:
- Runs `vanic registry-approve <username>` → adds them to `allowed_publishers`.
- Closes the issue with an explanation.

**On publish** (`vanic publish`), the compiler fetches `governance.json` and
checks three states in order:

| State | Error message |
|-------|--------------|
| Blacklisted | `publish rejected: '<user>' has been blacklisted from this registry.\nReason: <reason>\nSince: <date>\nTo appeal, open an issue at https://github.com/enthusiasticgeek/kosh-index` |
| Pending approval | `publish rejected: '<user>' has applied but is awaiting operator approval.\nSee https://github.com/enthusiasticgeek/kosh-index for status.` |
| Not in allowlist | `publish rejected: '<user>' is not an authorized publisher for this registry.\nTo apply, run: vanic apply-publisher\nSee https://github.com/enthusiasticgeek/kosh-index` |

The blacklist is checked **before** the allowlist — a revoked publisher cannot
slip through a race window.

All publisher state lives **entirely in `governance.json`** in the registry
repo. When governance transfers to a committee, or the registry moves to a new
domain, only that file changes — no compiler update required.

### Future governance path

| Phase | Trigger | Action |
|---|---|---|
| v1 (now) | 0 external users | Sole authority: `enthusiasticgeek` |
| v2 | First external contributors | Add co-maintainers to `allowed_publishers` |
| v3 | Broader adoption | Move to `kosh.vani-lang.org`; committee CODEOWNERS; cryptographic signing |

### Tamper evidence

SHA-256 checksums are recorded in `vani.lock` and verified at `vanic add`
download time. Future (v3): cryptographic signing of index entries (see
governance roadmap above).

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

### Q4b. TLS / PKI strategy

Kosh has **three layers of integrity** at different maturity levels:

| Layer | v1 (now) | v3 (future) |
|---|---|---|
| Transport | HTTPS (GitHub Pages + GitHub Releases) — TLS provided by GitHub's CA chain | same |
| Index integrity | SHA-256 checksum in `vani.lock`; verified at `vanic add` download time | same + Merkle tree |
| Publisher identity | GitHub OAuth (`gh` CLI token) — GitHub is the identity provider | per-package signing key in `governance.json` |

**What TLS gives you**: encrypted transit; prevents passive eavesdropping and
trivial MitM. The server certificate is issued by a CA that the OS/browser
trusts. GitHub Pages uses Let's Encrypt + GitHub's own intermediate CA.

**What TLS does NOT give you**: it says nothing about whether the *content* at
that URL is what the author intended. A compromised registry host (or a
malicious GitHub Actions run) could silently replace a tarball.

That gap is why `vani.lock` stores SHA-256 digests and `vanic add` refuses a
tarball that doesn't match. This is the same model crates.io / npm use in v1.

**Future: per-package signing (v3)**  
The industry standard is Sigstore / `cosign` — the package author signs the
tarball with a short-lived OIDC-issued certificate (no long-lived key to
leak), and the signature + certificate chain are recorded in a transparency log
(Rekor). Verifiers check the signature without trusting the registry host.
This is what `cargo` is moving toward (Crates.io Signing RFC #3081) and what
npm is adopting via `npm audit signatures`. Kosh will follow the same path
once there is a publisher community that makes it worthwhile.

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

- [`SECURITY.md`](../SECURITY.md) — full TLS/PKI trust model, `cafile` private-registry design, Sigstore roadmap.
- [`src/manifest.rs`](../src/manifest.rs) — manifest parser + Kosh registry code.
- [`kosh-index`](https://github.com/enthusiasticgeek/kosh-index) — live registry.
- [TODO.md](../TODO.md) — Kosh arc status.
- [Closure #287 in STATUS.md](../STATUS.md) — manifest path-style v2.
