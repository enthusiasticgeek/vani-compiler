# vāṇī — Release Process & Version Roadmap

## Versioning scheme

vāṇī follows [Semantic Versioning](https://semver.org/) with a pre-release suffix
during active development:

```
MAJOR.MINOR.PATCH[-PRERELEASE]
```

| Segment | Meaning |
|---------|---------|
| `MAJOR` | Breaking language or compiler-API changes (syntax, IR, ABI) |
| `MINOR` | New language features, new backends, new built-ins (backwards-compatible) |
| `PATCH` | Bug fixes, diagnostic improvements, performance, docs |
| `-dev`  | Unreleased development build — no stability promise |

**Current version: `0.1.0-dev`** (Cargo.toml).  
No formal release has been cut yet. The `-dev` suffix makes this explicit.

---

## Planned milestones

### `0.1.0` — First public release

**Gate** (all three required before tagging):

| # | Item | Status |
|---|------|--------|
| G1 | Generics (`Vec<T>`, `Option<T>`, user-defined `struct Foo<T>`) | pending |
| G2 | `match` exhaustiveness checking for enum payloads | pending |
| G3 | First-class closures (captures, `map`/`filter`/`fold` as HOF) | pending |

**Nice-to-have for 0.1.0** (can slip to 0.1.1):
- `forall` quantifiers in invariants (SMT layer already speaks `forall`)

Once G1–G3 land and the test suite is green on all platforms, bump
`Cargo.toml` to `0.1.0`, tag `v0.1.0`, trigger the GitHub release workflow.

### `0.1.x` — Patch series after first release

- `0.1.1`: `forall` quantifiers, any 0.1.0 bug fixes
- `0.1.x`: Ongoing diagnostics improvements, new example programs,
  additional dialect keyword tables, doc fixes.

### `0.2.0` — Parametric sync primitives

Depends on generics landing in 0.1.0.

| Item | Notes |
|------|-------|
| `Mutex<T>` / `Guard<T>` | Replace the current `Mutex_i64`-style mono-morphs |
| `Channel<T>` | Parameterised over element type |
| `RwLock<T>` / `Barrier` / `CondVar` | Broader sync primitive set |
| Traits / interfaces phase 2 | Blanket impls, default methods |

### `1.0.0` — Language stability promise

Criteria (not time-boxed):
- Syntax frozen: no breaking keyword or operator changes without a
  deprecation cycle.
- Formal spec document published (BNF + type rules).
- At least one non-trivial real-world program (>1 KLOC) ships as a
  reference.
- All three compiler backends (C, LLVM, SSA) pass the full test suite
  on Linux x86-64, Linux aarch64, macOS arm64, and Windows x86-64.

---

## How to cut a release

1. **Verify gate**: all gate items checked off, `cargo test` green on
   Linux + Windows.
2. **Bump version**: edit `Cargo.toml` — remove `-dev`, set target version
   (e.g. `0.1.0`).
3. **Update CHANGELOG** (create if absent): one-liner per notable change,
   grouped by Added / Fixed / Changed.
4. **Commit**: `git commit -m "chore: bump version to X.Y.Z"`.
5. **Tag**: `git tag -a vX.Y.Z -m "Release X.Y.Z"`.
6. **Push tag**: `git push origin vX.Y.Z` — triggers
   `.github/workflows/release.yml` which builds binaries for all 5
   target triples and attaches them to the GitHub release.
7. **Post-release**: immediately bump Cargo.toml to `X.Y.(Z+1)-dev` so
   the main branch always shows a dev suffix between releases.

---

## Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-06-17 | Bumped `0.1.0` → `0.1.0-dev` | No formal release cut yet; `-dev` makes pre-release state explicit |
| 2026-06-17 | Set G1/G2/G3 gate for `0.1.0` | Generics + closures + exhaustiveness are the minimum for a language to feel complete to an external user |
