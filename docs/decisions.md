# vāṇी — Architecture & Process Decisions

Running log of non-obvious decisions that aren't derivable from
the code. Newest entries at the top.

---

## 2026-06-18 — Package distribution strategy

**Decision**: Do not publish to GitHub Packages (npm / Docker)
now. Order of priority when ready:

1. **crates.io** (`cargo publish`) — simplest first step. No extra
   infra. Rust developers get `cargo install vanic`. Requires
   `Cargo.toml` already has `license`, `description`, `repository`,
   `readme` (all present). ~30 min effort.
2. **Homebrew tap** (`homebrew-vanic` repo with a Formula) —
   standard distribution path for developer CLI tools on macOS/Linux.
   Do this **after macOS is empirically verified** on a Darwin host.
3. **npm binary wrapper** — deferred indefinitely. npm is the
   JavaScript ecosystem's registry; signaling vāṇī as JS-adjacent
   is wrong for a systems language compiler. If community demand
   emerges, revisit.
4. **GitHub Container Registry (Docker)** — not a priority. Binary
   releases + install scripts already cover most users.

**Rationale**: the `install.sh` / `install.ps1` scripts + GitHub
Releases already serve the primary binary distribution need. Adding
package registries is polish, not gating. Crates.io is the
highest-value/lowest-effort next step because it's already a Rust
project.

---

## 2026-06-18 — Release notes infrastructure

**Decision**: per-release `RELEASE_NOTES/<tag>.md` files committed
to the repo. The `.github/workflows/release.yml` `Create release`
step resolves the tag, checks for `RELEASE_NOTES/<tag>.md`, and
uses `body_path` when present; falls back to `generate_release_notes:
true` for tags without a notes file.

**Rationale**: auto-generated GitHub release notes only show commit
messages (too noisy). Hand-written notes in the repo are versioned,
reviewable, and render well on the GitHub release page. The fallback
means future tags that skip writing notes still get something.

---

## 2026-06-18 — Stale docs refresh policy

**Decision**: which files to keep updated and at what cadence:

| File | Owner / cadence |
|---|---|
| `README.md` | Update per release: test count, feature ledger, new concurrency/generics features |
| `CHANGELOG.md` | Update per release (already done for v0.1.1) |
| `ARCS.md` | Update per session: mark items ✅ done, refresh open queue |
| `ARC8_V3_PLAN.md` | Amend per phase completion; use inline status blocks |
| `docs/missing_features.md` | Update when a "not in v1" limitation is lifted |
| `docs/next_session_design.md` | Archive when all items shipped; create a new file for the next work item |
| `ONBOARDING.md` | Update test count, env setup per release |
| `tutorials/src/advanced/03_concurrency.md` | Update when new sync primitives ship |
| `tutorials/src/intermediate/04_generics_iface.md` | Update when generics/traits features land |
| `STATUS.md` / `TODO.md` | Groom when they exceed ~500KB — extract shipped sections to `STATUS_ARCHIVE.md` |

**What NOT to document here**: code patterns, file paths, function
names — those are in the code. Decisions.md is for choices that
can't be recovered from reading the source (why we chose option A
over B, gating criteria, strategy).

---

## 2026-06-18 — Tutorial structure for concurrency (v0.1.1)

**Decision**: Barrier + RwLock coverage lives in the existing
`tutorials/src/advanced/03_concurrency.md`, not in new files.
The chapter is the concurrency reference; adding new sections there
is cheaper than maintaining separate primers for every new primitive.

**Exception**: if a primitive is pedagogically complex enough to
warrant a primer (like `02a_parallelism_primer.md`), add one in the
same tier. Barrier and RwLock are straightforward enough to not
need standalone primers for v0.1.1.

---

## 2026-06-17 — Kosh registry governance model

**Decision**: registry governance lives entirely in `governance.json`
on the registry side (`enthusiasticgeek/kosh-index`), not in the
compiler. The compiler reads it at `vanic publish` / `registry-approve`
time. Transferring stewardship of the registry = transferring the
GitHub repo; no compiler change needed.

**`config.json`** (new in v0.1.1) lets operators override the
registry URL and supply a CA cert for private registries, so
enterprise / airgapped deployments don't require a fork.

---

## 2026-06-11 — Windows verification complete; IOCP deferred

**Decision**: IOCP async-TCP (`tcp_echo_epoll`, `echo_loop`,
`async_showcase` on Windows) remains skipped in the test suite.
The shim uses completion-based IOCP semantics mapped onto a
readiness-shaped epoll API — the mismatch is documented in
`ARC8_V3_PLAN.md` Phase 6 as risk R8. Fully wiring it requires
either an overlapped-I/O rewrite or a native `iocp_*` builtin
family. Not gating v0.1.1.

---

## 2026-06-09 — Ref return via single-param lifetime elision (path-C)

**Decision**: functions can return `ref T` / `mut ref T` under the
single-ref-parameter elision rule. Zero or multi-ref-param returns
reject with a clear diagnostic. Multi-input distinct lifetimes (Rust
`'a` / `'b`) remain deferred indefinitely (path-D). Rationale: the
vast majority of practical "return a ref" patterns involve a single
source; adding explicit lifetime variables for the rare N-ref case
would add syntax complexity disproportionate to its use.

**Revisited 2026-07-25** (scoping pass, not implementation — see
`docs/ref_capturing_closures_design.md`): a real, repeated downstream need
(`vani-ml`'s `logreg_fit` needing to pass a ref-capturing closure to
`vani-optimize`'s generic gradient-descent driver) prompted checking
whether this decision should be reopened. Conclusion: **no** — general
multi-parameter lifetime variables (path-D) are still not warranted; the
actual gap turned out narrower than expected (see the design doc's
"Path B" — a bounded extension of the already-shipped Arc-5c closure
machinery plus the existing scope-escape analyzer, not new lifetime
inference) and a real, independent soundness bug (BUG-7,
`docs/TODO_CURRENT.md`) was found in that escape analyzer along the way.
This decision (defer path-D) stands; path-B is a separate, smaller,
not-yet-started proposal.

---

## 2026-06-06 — CLI rename: intentc → vanic

**Decision**: `vanic` is the canonical binary. `intentc` is kept as
a byte-identical binary alias for one release cycle (v0.1.x) so
existing scripts don't break. `default-run = "vanic"` in `Cargo.toml`.
Both binaries compile from the same `src/main.rs`. Cleanup tracked
by removing the `[[bin]] name = "intentc"` entry post-v0.1.x.

---

## 2026-06-06 — Platform async strategy

**Decision**: C backend uses `#ifdef __APPLE__` / `#if defined(_WIN32)`
branches in the helper-emit functions. LLVM backend uses
host-conditional inline IR. This means the compiler emits correct
platform code when run on the right host — it is NOT a
cross-compiler for async primitives. Linux epoll / macOS kqueue /
Windows IOCP are strictly host-detected, not target-specified.

**Rationale**: cross-compiling async code for a different OS would
require carrying all three platform runtimes in every emission —
unnecessary complexity for v1. Revisit at v2 if embedded/cross
targets become a priority.

---

## 2026-06-04 — v3.1 async transform: parser-time synthesis

**Decision**: the v3.1 `async fn → Task struct + __poll fn`
transform runs at parser time (inside `parse_function` →
`try_v31_transform`), not as a post-parse IR pass.

**Rationale**: follows the closure-lift precedent already in the
parser (`CLOSURE_MAKE_REGISTRY`). Parser-time synthesis means the
checker and backends see fully-concrete struct + function nodes
with no new IR forms. The cost is that `try_v31_transform` is a
long function; the benefit is zero new IR plumbing.

---

## 2026-06-03 — Kosh name

`kosh` (कोश) = Sanskrit/Hindi for "treasury / repository / store."
Used as: the package manager name, the visibility tier `pub(kosh)`,
and the registry subdomain. Keeps the project's Sanskrit/Hindi
naming convention consistent.
