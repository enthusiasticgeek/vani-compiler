# Kosh security model

## Current trust model (v1)

Kosh uses three independent layers of integrity, applied in order:

| Layer | Mechanism | Who provides it |
|---|---|---|
| Transport confidentiality | HTTPS (TLS 1.2+) | GitHub's CA chain (Let's Encrypt / DigiCert) |
| Content integrity | SHA-256 digest in `vani.lock`; verified at `vanic add` download time | Kosh + `curl` |
| Publisher identity | GitHub OAuth token via `gh` CLI | GitHub |

### Transport (TLS)

All registry traffic goes to GitHub Pages (`enthusiasticgeek.github.io`) and
GitHub API/Releases endpoints. These use publicly-trusted certificates issued
by GitHub's CA chain. No configuration is required — `curl` and `gh` verify
against the OS cert store automatically.

**`curl` cert verification flags used:**

```
curl -fsSL <url>            # fetches index / governance / tarballs
curl -fsSL --output …       # downloads tarballs to vendor/
```

`-f` = fail on HTTP error, `-s` = silent, `-S` = show errors, `-L` = follow
redirects. **TLS certificate verification is ON by default**; curl will refuse
a connection to a host with an untrusted certificate.

### Content integrity

Every published package records its SHA-256 digest in the sparse index:

```json
{"name":"mathlib","version":"1.0.0","cksum":"sha256:<hex>","yanked":false}
```

`vanic add` computes the digest of the downloaded tarball and compares it to the
recorded value. Mismatch → hard error, tarball is discarded.  
This protects against a CDN compromise or tarball swap even if TLS is intact.

### Publisher identity

`vanic publish` requires a valid `gh auth login` session. The GitHub username is
extracted via `gh api user` and checked against `governance.json`. Only approved
publishers can append to the index.

---

## What is NOT protected (known gaps)

| Gap | Risk | Planned fix |
|---|---|---|
| No per-package signing | A registry compromise could swap index entries AND tarballs silently (SHA-256 only catches the tarball swap, not the index entry) | Sigstore / cosign — see below |
| No private-registry TLS config | `vanic add registry+https://internal.corp/...` against a server using a self-signed or internal-CA cert will fail because `curl` can't verify it | `VANI_HTTP_CAINFO` env var + `cafile` in `vani.toml` — see below |
| `gh` API calls use GitHub-hosted cert chain only | Any non-GitHub API endpoint is not reached via `gh` | Non-GitHub registries use `curl` only; same `cafile` fix applies |

---

## Deferred: private registry TLS (`cafile`) — v1.1 or later

When a user points Kosh at an internal registry over HTTPS with a self-signed
cert or an internal CA, `curl` will refuse the connection with
`SSL certificate problem: self-signed certificate`.

**Planned solution** (mirrors cargo's `[http] cainfo`):

1. Add `VANI_HTTP_CAINFO=/path/to/ca.pem` environment variable support.  
   `manifest.rs` passes `--cacert "$VANI_HTTP_CAINFO"` to every `curl` call
   when the variable is set.

2. Add `[registry]` section to `vani.toml`:

   ```toml
   [registry]
   url    = "https://internal.corp/kosh"
   cafile = "certs/internal-ca.pem"   # relative to vani.toml, or absolute
   ```

   `cafile` is forwarded as `--cacert <path>` to curl. `VANI_HTTP_CAINFO`
   overrides `cafile` if both are set.

**Where to store the CA cert:**

| Scenario | Recommended path |
|---|---|
| Single developer, personal machine | `~/.config/vani/ca.pem` — set `VANI_HTTP_CAINFO` in shell profile |
| Team with shared internal registry | Commit `certs/internal-ca.pem` to the project repo; reference it in `vani.toml`'s `cafile` field |
| CI / container | Mount cert at a known path; set `VANI_HTTP_CAINFO` in the pipeline env |
| OS-level trust (all tools) | Add the CA to the system store: `update-ca-certificates` (Debian/Ubuntu), `update-ca-trust` (RHEL/Fedora), Keychain Access (macOS), `certmgr` (Windows) — then no Kosh-specific config needed |

**Self-signed vs internal CA:**
- *Self-signed* = the server cert signs itself. Use `--cacert server.pem`.
  Only trusts that one cert; fragile (re-issue breaks it).
- *Internal CA* = you run a root CA that signs server certs. Use
  `--cacert rootCA.pem`. Trusts any cert your CA issues; survives server
  cert rotation. Prefer this for org-wide infrastructure.

---

## Deferred: package signing (Sigstore / cosign) — v3

The state-of-the-art is **Sigstore / `cosign`**:

- Author signs the tarball at publish time using a *short-lived OIDC
  certificate* (GitHub Actions identity, Google / GitHub account login).  
  No long-lived private key to leak, rotate, or store.
- The signature + certificate chain are appended to a **transparency log**
  (Rekor). The log is append-only; signatures cannot be removed or altered.
- Verifiers (`vanic add`) fetch the signature bundle from the log and check
  the cert chain against the OIDC issuer. No need to trust the registry host.

This is what `cargo` is moving toward (Crates.io Signing RFC #3721), what npm
adopted (npm audit signatures / provenance attestations), and what PyPI
implemented via sigstore-python.

Kosh will adopt this when there is enough of a publisher community to make the
friction worthwhile. Until then, SHA-256 + GitHub publisher gate is the
pragmatic v1 answer.

---

## See also

- [`docs/kosh_design.md`](docs/kosh_design.md) — architecture, governance, and full feature list
- [`src/manifest.rs`](src/manifest.rs) — all registry HTTP calls (`http_get_text`, `http_get_file`)
- [`kosh-index`](https://github.com/enthusiasticgeek/kosh-index) — live registry
