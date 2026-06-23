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

**Current version: `0.1.8`** (Cargo.toml).  
`v0.1.0` tagged 2026-06-18. `v0.1.1` tagged 2026-06-18 (Barrier, RwLock<T>, parametric Mutex<T>/Channel<T>, Traits phase 2, kosh config). `v0.1.2` tagged 2026-06-19 (Win64/AArch64 ABI, dialect purity, first crates.io publish). `v0.1.3` tagged 2026-06-20 (patch series). `v0.1.4` tagged 2026-06-20 (non-Copy tuples, Box<T> enum payloads, tutorial link fixes). `v0.1.5` tagged 2026-06-21 (native file I/O: FileHandle, file_open/read/write/close, stdin_read_line, eprint). `v0.1.6` tagged 2026-06-21 (bare-metal: --target, --no-std, #[no_mangle], #[link_section], mmio u8/u16, QEMU run). `v0.1.7` tagged 2026-06-21 (tutorial coverage expansion: 10 new pages covering CLI ref, FnPtr, file I/O, math deep-dive, vec stats, condvar, cross-compile, attributes, advanced collections). `v0.1.8` tagged 2026-06-23 (three new language features: block comments `/* ... */` with nesting, print blocks `print { ... }`, positional break `break inner/middle/outer`).

---

## Planned milestones

### `0.1.0` — First public release

**Gate** (all three required before tagging):

| # | Item | Status |
|---|------|--------|
| G1 | Generics (`Vec<T>`, `Option<T>`, user-defined `struct Foo<T>`, methods + iface impls on generic instantiations) | ✅ done commit `c89cfb5` |
| G2 | `match` exhaustiveness checking for enum payloads | ✅ done commit `3e1260c` |
| G3 | First-class closures (captures, `map`/`filter`/`fold` as HOF) | ✅ done |

**Nice-to-have for 0.1.0** — also shipped:
- `forall` quantifiers in invariants — ✅ done commit `13b93cd`

Once G1–G3 land and the test suite is green on all platforms, bump
`Cargo.toml` to `0.1.0`, tag `v0.1.0`, trigger the GitHub release workflow.

### `0.1.1` — Concurrency + Traits phase 2 ✅ shipped 2026-06-18

| Item | Status |
|------|--------|
| `Barrier` (`barrier_new` / `barrier_wait`) | ✅ |
| `RwLock<T>` / `ReadGuard<T>` / `WriteGuard<T>` | ✅ |
| Parametric `Mutex<T>` / `Guard<T>` (any element type) | ✅ |
| Parametric `Channel<T, N>` (any element type) | ✅ |
| Traits phase 2: default methods + blanket impls | ✅ |
| kosh config: runtime registry URL + CA cert override | ✅ |

### `0.1.x` — Patch series

- `0.1.2` ✅ shipped 2026-06-19: Win64/AArch64 ABI classifiers, dialect purity docs,
  `intentc` deprecation warning, new tutorials (Barrier, RwLock, default methods),
  examples reorganisation, first crates.io publish.
- `0.1.3` ✅ shipped 2026-06-20: patch series fixes.
- `0.1.4` ✅ shipped 2026-06-20: non-Copy elements in tuples, `Box<T>` as enum variant
  payload, tutorial site broken-link sweep (19 files).
- `0.1.5` ✅ shipped 2026-06-21: native file I/O — `FileHandle` (affine RAII), `file_open` /
  `file_is_ok` / `file_read_line` / `file_write` / `file_close` / `file_flush`,
  `stdin_read_line`, `flush_stdout`, `eprint` statement. L18 resolved.
- `0.1.6` ✅ shipped 2026-06-21: bare-metal / cross-compilation — `--target=<triple>`,
  `--no-std`, `#[no_mangle]`, `#[link_section = "..."]`, `mmio_read/write_u8`,
  `mmio_read/write_u16`, QEMU user-mode run. L19 fully resolved (all 5 gaps).
- `0.1.7` ✅ shipped 2026-06-21: tutorial coverage expansion — 10 new pages covering the
  full CLI reference, function pointers, native file I/O, math deep-dive (special fns +
  ML activations + bit ops), vec statistics, condvar primer, cross-compile primer,
  function attributes reference, and advanced collections. No compiler changes.

### `0.2.0` — Package manager + cross-platform I/O

| Item | Notes |
|------|-------|
| Arc 9 kosh: `kosh.toml` manifest, resolver, lockfile, registry CLI, stdlib-as-kosh | Deferred pending registry-hosting choice |
| Arc 8 macOS port: kqueue shim for async I/O | Blocked on Darwin hardware |
| Arc 8 Windows IOCP: overlapped-I/O rewrite | ~25–35 h; readiness-vs-completion mismatch (R8 in decisions.md) |
| Arc 7 Win64/AArch64 ABI: float-class + mixed struct classifier | ~6–8 h; gated on CI runner |
| Arc 10 Devanagari SOV word order | Blocked on grammar consultant |

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
   (e.g. `0.1.1`).
3. **Write release notes**: create `RELEASE_NOTES/vX.Y.Z.md` with a
   human-readable summary of the release. The release workflow reads this
   file via `body_path`; if absent it falls back to auto-generated notes.
4. **Update CHANGELOG** (create if absent): one-liner per notable change,
   grouped by Added / Fixed / Changed.
5. **Commit**: `git commit -m "chore: bump version to X.Y.Z"`.
6. **Tag**: `git tag -a vX.Y.Z -m "Release X.Y.Z"`.
7. **Push tag**: `git push origin vX.Y.Z` — triggers
   `.github/workflows/release.yml` which builds binaries for all 5
   target triples and attaches them to the GitHub release.
8. **Publish to crates.io**: `cargo publish` — makes `cargo install vanic`
   work for Rust users. Run from the repo root after the tag is pushed.
9. **Post-release**: immediately bump Cargo.toml to `X.Y.(Z+1)-dev` so
   the main branch always shows a dev suffix between releases.

---

## Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-06-17 | Bumped `0.1.0` → `0.1.0-dev` | No formal release cut yet; `-dev` makes pre-release state explicit |
| 2026-06-17 | Set G1/G2/G3 gate for `0.1.0` | Generics + closures + exhaustiveness are the minimum for a language to feel complete to an external user |
| 2026-06-18 | All G1–G3 gates satisfied | G2/G3 landed in prior sessions; G1 completed today (methods + iface impls on generic struct instantiations). `forall` nice-to-have also shipped. Ready to tag `v0.1.0`. |
| 2026-06-18 | Tagged `v0.1.0` | First public release. |
| 2026-06-18 | Tagged `v0.1.1` | Shipped `Barrier`, `RwLock<T>/ReadGuard<T>/WriteGuard<T>`, parametric `Mutex<T>/Channel<T>`, Traits phase 2 (default methods + blanket impls), kosh config (runtime URL + CA cert). |
| 2026-06-18 | Bumped to `0.1.2-dev` | Active development continues post v0.1.1. |
| 2026-06-18 | Added `RELEASE_NOTES/` infrastructure | Per-release hand-written notes in `RELEASE_NOTES/<tag>.md`; release workflow uses `body_path` when present, auto-generates otherwise. |
| 2026-06-18 | crates.io publish as next distribution step | `cargo install vanic` for Rust users. Homebrew tap deferred until macOS empirically verified. See docs/decisions.md. |
| 2026-06-19 | Tagged `v0.1.2` | Win64/AArch64 ABI, dialect purity, tutorials, first crates.io publish. |
| 2026-06-19 | Bumped to `0.1.3-dev` | Active development continues post v0.1.2. |
| 2026-06-20 | Tagged `v0.1.3` | Patch series. |
| 2026-06-20 | Tagged `v0.1.4` | Non-Copy tuples, Box<T> enum payloads, tutorial link sweep. |
| 2026-06-21 | Tagged `v0.1.5` | Native file I/O: FileHandle, file_open/read/write/close, stdin_read_line, eprint. L18 resolved. |
| 2026-06-21 | Tagged `v0.1.6` | Bare-metal arc: --target, --no-std, #[no_mangle], #[link_section], mmio u8/u16, QEMU run. L19 fully resolved. |
| 2026-06-21 | Tagged `v0.1.7` | Tutorial coverage expansion: 10 new pages — CLI ref, FnPtr primer, file I/O primer+worked, math deep-dive, vec stats, condvar primer, cross-compile primer, attributes reference, advanced collections. No compiler changes. |
