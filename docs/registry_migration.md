# Kosh registry migration plan

The current registry lives in a personal GitHub repo (`enthusiasticgeek/kosh-index`).
This document records the trigger conditions for each migration phase and
exactly what changes — so if traction happens, the path is already mapped.

---

## What the registry is made of

```
Component            Where it lives                    Size growth
─────────────────────────────────────────────────────────────────
Sparse index         GitHub Pages (kosh-index repo)    ~200 B per published version
  index/<name>.json  NDJSON — one line per version     10k pkgs × 50 vers ≈ 100 MB total
  config.json        dl + api URL templates
  governance.json    publisher allowlist / blacklist

Tarballs             GitHub Release assets             ~100 KB – 5 MB per tarball
  (the actual code)  kosh-index repo, per-release      1k pkgs × 10 vers × 500 KB ≈ 5 GB
```

The index is tiny forever. **Tarballs are the risk** — they live as GitHub
Release assets and GitHub's per-repo storage is not publicly bounded but
practically soft-caps around 5 GB before friction begins.

---

## GitHub limits that apply

| Limit | Value | Notes |
|---|---|---|
| Repository (git objects) | 1 GB recommended, ~5 GB hard | Index NDJSON files are in git; tarballs are NOT (they are release assets, not committed) |
| Release asset (single file) | 2 GB | Not a concern — package tarballs are tiny |
| Release asset storage (total) | Not officially documented | Anecdotally several GB is fine on public repos; no per-repo cap enforced as of 2026 |
| GitHub Pages bandwidth | 100 GB / month | Only the index JSON is served via Pages; tarballs are served from `github.com/releases` |
| GitHub Pages repo size | 1 GB | Only the index files count; tarballs bypass this |

**Bottom line**: the git repo (index NDJSON) will never hit limits. Release
asset storage for tarballs is the only practical risk, and it only matters
if the registry has hundreds of active packages each publishing many versions.

---

## Phase 0 — current (personal repo)

**Trigger**: <~50 packages, sole maintainer.

No action needed. Personal repo is fine.

**URLs in play:**

| Constant | Value | File |
|---|---|---|
| `DEFAULT_REGISTRY` | `https://enthusiasticgeek.github.io/kosh-index` | `src/manifest.rs:266` |
| `DEFAULT_DL_TEMPLATE` | `https://github.com/enthusiasticgeek/kosh-index/releases/download/…` | `src/manifest.rs:270` |
| governance API path | `repos/enthusiasticgeek/kosh-index/contents/governance.json` | `src/manifest.rs:744` |

---

## Phase 1 — move to a GitHub Organization

**Trigger**: first external contributors, or when the registry needs shared
write access (multiple maintainers).  
**Effort**: ~1 hour.  
**Size benefit**: none — same GitHub infrastructure.  
**Governance benefit**: registry is no longer tied to one person's account.

### Steps

1. Create a GitHub org — e.g. `vani-lang`.
2. Transfer `kosh-index` repo to the org: `Settings → Transfer → vani-lang/kosh-index`.  
   GitHub automatically redirects the old URLs (`enthusiasticgeek.github.io/kosh-index`
   → `vani-lang.github.io/kosh-index`) for a grace period. Old tarballs remain
   accessible at the old release URLs permanently.
3. Update three places in `src/manifest.rs` in the same commit and release:

   ```rust
   // manifest.rs:266
   pub const DEFAULT_REGISTRY: &str = "https://vani-lang.github.io/kosh-index";

   // manifest.rs:270
   pub const DEFAULT_DL_TEMPLATE: &str =
       "https://github.com/vani-lang/kosh-index/releases/download/…";

   // manifest.rs:744 (inside fetch_governance_with_sha)
   "repos/vani-lang/kosh-index/contents/governance.json"
   ```

4. Update `governance.json` `governance_url` field in the transferred repo.
5. Tag a patch release of the compiler (`0.1.1` or `0.2.0`).

Old compiler builds continue to work during the GitHub redirect window.
Once the redirect expires, users on old builds get a fetch error pointing
them to upgrade — acceptable.

---

## Phase 2 — offload tarballs to a CDN

**Trigger**: release asset storage approaches ~2–3 GB (rough estimate: ~500
active packages × 5 versions × 500 KB). Or whenever GitHub Releases feels
unreliable.  
**Effort**: ~4 hours.  
**Index stays on GitHub Pages** (it's tiny). Only tarballs move.

### Recommended CDN: Cloudflare R2

Free tier: 10 GB storage, 10 M read requests/month, no egress fees (R2's
main selling point vs S3).  
Paid: $0.015/GB-month storage, $0.36/M requests beyond free tier.

### Steps

1. Create a Cloudflare R2 bucket, e.g. `kosh-packages`.
   Enable public access; bind a custom subdomain like `cdn.vani-lang.org`.

2. **Migrate existing tarballs** (one-time):
   ```bash
   # Download all release assets from kosh-index, re-upload to R2
   gh release list --repo vani-lang/kosh-index --limit 1000 | \
     awk '{print $1}' | while read tag; do
       gh release download "$tag" --repo vani-lang/kosh-index --dir /tmp/assets
   done
   # Upload to R2 via wrangler or rclone
   rclone sync /tmp/assets r2:kosh-packages/
   ```

3. Update `config.json` in the kosh-index repo:
   ```json
   {
     "dl":  "https://cdn.vani-lang.org/{name}-v{version}/{name}-{version}.tar.gz",
     "api": "https://vani-lang.github.io/kosh-index"
   }
   ```

4. Update `DEFAULT_DL_TEMPLATE` in `src/manifest.rs` and release a new compiler.

5. Keep old GitHub Release assets in place — permanent URLs never break
   for users on old compiler builds.

### Why not just use the `dl` field from `config.json` at runtime?

Currently `DEFAULT_DL_TEMPLATE` is a compile-time constant. A future
improvement (tracked below) would have the compiler fetch `config.json`
on first use and cache the `dl` URL — then changing `config.json` alone
updates all clients without a compiler release. Until then, the two-step
(update config.json + release new compiler) is acceptable.

---

## Phase 3 — custom domain + dedicated index host

**Trigger**: GitHub Pages bandwidth limit approached (100 GB/month), or
governance requires fully independent infrastructure.  
**Effort**: 1–2 days (domain + server setup + DNS propagation).

### Index options

| Option | Cost | Ops burden |
|---|---|---|
| GitHub Pages + CNAME (`index.vani-lang.org`) | Free | Zero — same infra, just a domain |
| Cloudflare Pages | Free tier generous | Zero |
| Fly.io or Railway small instance | ~$3–7/month | Low — stateless static file server |
| Self-hosted VPS (Hetzner, etc.) | ~$4–6/month | Medium |

**Recommended path**: CNAME `index.vani-lang.org` → `vani-lang.github.io`
first. Zero ops, professional URL, easy to move off later.

### Compiler changes at Phase 3

```rust
pub const DEFAULT_REGISTRY: &str = "https://index.vani-lang.org";
```

One constant. One compiler release. Done.

---

## Migration readiness today

The architecture was built for migration. The full change set to move from
Phase 0 to Phase 3 touches:

| File | What changes | Lines |
|---|---|---|
| `src/manifest.rs` | 3 URL constants | ~3 |
| `kosh-index/config.json` | `dl` + `api` fields | 2 |
| `kosh-index/governance.json` | `governance_url` | 1 |

No user `vani.toml` files change. No package tarballs are invalidated.
Existing `vani.lock` SHA-256 checksums remain valid regardless of where
the tarball is hosted.

---

## Deferred: read `dl` from `config.json` at runtime

Currently `DEFAULT_DL_TEMPLATE` is a hardcoded constant. If it were instead
fetched from `config.json`'s `dl` field at `vanic add` time (cached in
`vani.lock` or a local registry cache file), changing `config.json` would
instantly update all clients without a compiler release.

This is how Cargo works (`.cargo/config.toml` `replace-with` + `source` blocks
plus the Crates.io `dl` field in the sparse index config). Implement when
Phase 2 (CDN switch) makes it worthwhile.

---

## Size watch — suggested manual check

Until automated monitoring is set up, check release asset size periodically:

```bash
gh release list --repo enthusiasticgeek/kosh-index --limit 200 | \
  awk '{print $1}' | while read tag; do
    gh release view "$tag" --repo enthusiasticgeek/kosh-index --json assets \
      --jq '.assets[].size'
  done | awk '{sum += $1} END {printf "Total: %.1f MB\n", sum/1024/1024}'
```

Plan the CDN migration before reaching 1 GB total. At 1 package/week
averaging 500 KB per tarball, that is ~4 years away at current scale.

---

## See also

- [`docs/kosh_design.md`](kosh_design.md) — full architecture and governance
- [`SECURITY.md`](../SECURITY.md) — TLS/PKI trust model and `cafile` design
- [`src/manifest.rs`](../src/manifest.rs) — the three constants to update
- [`kosh-index`](https://github.com/enthusiasticgeek/kosh-index) — live registry
