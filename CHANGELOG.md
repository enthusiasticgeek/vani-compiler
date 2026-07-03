# Changelog

All notable changes to vāṇी (vanic) are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Versions follow [Semantic Versioning](https://semver.org/).

---

## [0.1.3] — 2026-06-19

### Added — Installation docs

- **System requirements table** in `INSTALL.md` — minimum tool versions
  (Rust 1.75+, z3 4.8+, LLVM 14–22, gcc/clang 9+, Python 3.8+ optional).
- **Tested OS matrix** — explicit per-row verification status across Ubuntu
  20.04 / 22.04 / 24.04, Debian 10 / 12, Arch, Fedora, Windows 11 (GNU),
  WSL2; macOS marked ⚠️ unverified pending hardware.
- **Older Linux (Debian 10 Buster) subsection** — step-by-step guide to
  install z3 4.8.17 from the GitHub pre-built binary (glibc 2.27, compatible
  with Buster's glibc 2.28) when the apt repo only ships z3 4.4.1 (too old).
  Includes fallback `--backend=c` note to avoid the older LLVM 7 in Buster's apt.

### Changed — Docs

- `INSTALL.md` test counts updated to **2421+** throughout (was 2089).
- Windows status note updated to 2026-06-19.

---

## [0.1.2] — 2026-06-19

### Added — Language / Parser

- **SOV fn / struct / enum declarations** — name-first top-level shapes now supported
  (`add(a, b) -> i64 fn { … }`, `Point struct { … }`, `Dir enum { … }`).
  Parser rewrites token stream to canonical order; all downstream passes are unchanged.
  Wired in top-level and module-body dispatchers with `parse_match_arms_block` refactor.
- **Devanagari aliases for `extern` / `type` / `intent` / `invariant`** verified and
  tested: `बाह्य` / `प्रकार` / `उद्देश्य` / `अपरिवर्तनीय`.
- **`intentc` deprecation warning** — startup prints a migration notice toward `vanic`.

### Added — Platform ABI

- **Win64 struct-return classifier** — `is_ffi_safe_struct_win64`: size ∈ {1, 2, 4, 8}
  bytes only; platform-specific rejection hint.
- **AArch64 struct classifier** — `is_ffi_safe_struct_aarch64`: HFA (1–4 identical
  f32/f64 fields) OR all-scalar ≤ 16 bytes.
- `is_ffi_safe_struct` dispatches per target at compile time (SysV / Win64 / AArch64).

### Added — Dialect purity

- Sub-dialect gate (`spelling_supports_dialect`) verified across all 45 Devanagari
  structure-keyword aliases; stale doc comment in `enforce_language_purity` corrected.
- 2 new dialect-rejection tests: Marathi-pragma rejects Sanskrit-only `अन्यथा`; Hindi-pragma
  rejects Marathi-only `थांब`.

### Added — Tutorials

- `tutorials/src/advanced/02b_barrier_primer.md` — Barrier intuition, API, worked 3-thread example.
- `tutorials/src/advanced/02c_rwlock_primer.md` — RwLock state encoding, RAII guards, writer-starvation caveat.
- `tutorials/src/intermediate/04d_default_methods_primer.md` — default methods and blanket impls.
- All three added to `tutorials/src/SUMMARY.md`.

### Added — Tooling

- **`tools/vani_translate.py` v2** — auto-detect source language from pragma; `--verify`
  round-trip flag; `--list-keywords` markdown table; `--batch` directory mode;
  `--inplace` with `.bak` backup; UTF-8 stdout fix for Windows.

### Changed — Docs / Examples

- All Devanagari examples organised under `examples/language/{sanskrit,hindi,marathi}/`
  (14 Sanskrit, 12 Hindi, 12 Marathi); each carries a `// श्री।` header.
- `STATUS.md` / `TODO.md` condensed; pre-Arc-8 history moved to `*_ARCHIVE.md` files.
- `docs/v1_limitations.md`: L13 updated (SOV fn/struct/enum now supported; match-as-stmt
  stays keyword-first). L15/L16/L17 marked resolved.

### Distribution

- `v0.1.2` tagged and published to crates.io (`cargo install vanic`).

---

## [0.1.1] — 2026-06-18

### Added — Language

- **`Barrier`** — N-thread rendezvous primitive (`barrier_new(n)` / `barrier_wait(mut ref b) -> bool`).
  Stack-by-value, affine. Uses a generation counter to prevent ABA races under futex/WaitOnAddress.
  Both C and LLVM backends with inline IR lowering.
- **`RwLock<T>` / `ReadGuard<T>` / `WriteGuard<T>`** — readers-writer lock parametric over any value
  type T. `rwlock_read` acquires a shared read guard; `rwlock_write` acquires an exclusive write guard.
  RAII drop releases the lock. State encoding: 0=unlocked, N>0=N concurrent readers, -1=write-locked.
  Per-T C struct bundles + LLVM preamble types. Both backends.
- **Parametric `Mutex<T>` / `Guard<T>`** — previously i64-only; now any element type (integers,
  bool, struct, enum). Per-T C bundles via `collect_mutex_specs` + `emit_mutex_bundle`.
- **Parametric `Channel<T, N>`** — struct and enum element types now accepted in addition to
  integer widths and bool. C backend uses `c_element_storage` + `memset` zero-init; LLVM backend
  uses `channel_slot_llvm_string` for aggregate slots. Naming is consistent across both backends.
- **Traits phase 2** — default methods in interface declarations; blanket impls (`implement<T> Iface for Wrapper<T> where T is Iface`).
  Satisfiability checking with bounded generics.

### Added — Package manager (kosh)

- Runtime download URL configurable via `config.json`; custom CA certificate file support for
  private registries via `cafile` field.

---

## [0.1.0] — 2026-06-18

First public release. vāṇी compiles, verifies, and runs programs written
in a readable, proof-annotated language with affine types, closures,
generics, async/await, and a package manager.

### Added — Language

- **Generics** — `struct Foo<T>`, `enum Option<T>`, `fn id<T>(x: T) -> T`.
  Methods blocks and interface implementations on generic instantiations
  (`methods on Pair<i64>`, `implement Sumable for Pair<i64>`). Bounded
  generics via `where T is Cmp`. Full monomorphization to concrete types.
- **First-class closures** — `let f = fn(x: i64) -> i64 { x * 2 };`
  By-value captures and `[ref xs]` reference-capture syntax. `Closure(T) -> R`
  fat-pointer type for higher-order functions. Built-in HOF:
  `vec_map`, `vec_filter`, `vec_fold`.
- **`match` enum exhaustiveness** — payload exhaustiveness checking; rejects
  bindings on variants that carry no payload.
- **`forall` quantifiers** — `prove forall x: i64, x + 0 == x;` in proof
  positions. SMT layer emits `(forall ((x Int)) ...)` for Z3 discharge.
- **`break value` / labeled loops** — `let x = loop { break 42; };`
  and `'outer: while … { break 'outer; }`.
- **`volatile_read` / `volatile_write`** — MMIO builtins for embedded targets,
  gated by `INTENT_TARGET_EMBEDDED=1`.
- **`unsafe(reason = "…") { … }`** — explicit unsafe blocks with mandatory
  justification string. Raw pointer types `*const T` / `*mut T`,
  `Pool<T>` / `Handle<T>` generational-handle allocator, `Tainted<T>` wrapper.
- **`try` / `?` operator** — desugars for `Option<T>` and `Result<T, E>`
  return types. Postfix `?` form.
- **`pub use` re-exports and glob imports** — `pub use module::item;`,
  `use module::*;` resolved through facade modules.

### Added — Async

- **v3.1 async/await** — `async fn`, `await`, task structs synthesized per
  async function, poll-based execution model, `CancelToken` auto-plumbing,
  multi-task scheduling, `io_recv_async` / `io_send_async` / `io_accept_async`.
- **epoll / WSAPoll** — non-blocking TCP, `epoll_new/add_read/wait_one/close`,
  cooperative echo server in a single OS thread.
- **Windows async TCP** — full IOCP → WSAPoll/select rewrite for Windows
  compatibility; WSAECONNRESET handled.

### Added — Package manager (Kosh)

- **`vani.toml` manifest** — `[package]` (name, version, entry) and `[deps]`
  with semver constraints (`^1.0`, `~1.2`, `>=1.0`, exact).
- **`vani.lock`** — lockfile with SHA-256 checksums; verified at install time.
- **`vanic add <name>[@constraint]`** — resolves from registry, downloads
  tarball, extracts to `vendor/`, updates manifest and lockfile.
- **`vanic publish`** — builds tarball, checks publisher authorization,
  creates GitHub Release, appends to sparse index.
- **`vanic vendor`** / **`vanic remove`** / **`vanic search`** / **`vanic update`**.
- **Publisher governance** — `vanic apply-publisher` (accept agreement + open
  GitHub issue), `vanic registry-approve`, `vanic registry-blacklist`.
- **Registry** live at `https://enthusiasticgeek.github.io/kosh-index`.

### Added — Diagnostics

- **Elaboration on 597 diagnostic sites** — every compiler error includes a
  WHAT / WHY / HOW explanation. 20+ families: `type_mismatch`,
  `duplicate_declaration`, `duplicate_parameter`, `match_wrong_pattern_type`,
  `iface_missing_method`, `pure_fn_calls_non_pure`, `builtin_wrong_arg_type`,
  and more.

### Added — Internationalization

- **62 non-English keyword dialects** — Sanskrit, Hindi, Marathi, Bengali,
  Gujarati, Punjabi, Tamil, Telugu, Kannada, Malayalam, Odia, Assamese,
  Sinhala, Nepali, Urdu, Sindhi, Punjabi-Shahmukhi, Persian, Pashto,
  Spanish, French, Russian, German, Italian, Portuguese, Dutch, Swedish,
  Norwegian, Danish, Finnish, Hebrew, Armenian, Georgian, Japanese, Mandarin,
  Korean, Arabic, Amharic, Tibetan, Mongolian, Cherokee, Lao, Vietnamese,
  Hausa, Yoruba, Indonesian, Malay, Swahili, Filipino, and more.
  Activated via `// vani-lang: <name>` pragma.
- **LSP dialect-aware completion** — keyword autocomplete respects the active
  dialect pragma.

### Added — Tooling

- **LSP (`intent-lsp`)** — hover types, go-to-definition, diagnostics,
  semantic-token highlighting, completion. Works on broken documents.
- **Big-O annotation** — `--big-o[=auto|force|off]` flag annotates each
  function's asymptotic complexity in compiler output.
- **`install.sh` / `install.ps1`** — one-line installers for Linux/macOS
  and Windows that download the correct release binary.
- **GitHub Actions release workflow** — tag push builds 5 target triples
  (Linux x86_64/aarch64, macOS x86_64/aarch64, Windows x86_64) and
  attaches archives to the GitHub release.

### Fixed

- LLVM: `fcmp une` for `!=` so NaN != NaN evaluates to true (IEEE 754).
- LLVM: `Vec<Box<dyn Iface>>` element slot size (8 → 16 bytes for fat pointer).
- LLVM: per-element drop in `Vec<Box<T>>__free`.
- SSA/LLVM: `select` for aggregate captures instead of self-bitcast.
- SSA backends: runtime bounds guards for `InstrKind::Index { checked }`.
- Checker: `as` int-to-int cast wraps at compile time rather than rejecting.
- Checker: `MethodCall` receiver not consumed by `skip_call_args`.
- Checker: generic struct names resolved at `StructLit` lookup sites.
- Checker: `try`-desugar accepts `Option<T>` / `Result<T,E>` return types.
- Checker: glob import resolves `pub use` re-exports.
- Big-O: sort-outside-loop correctness (O(n log n) not O(n)).
- Windows: full end-to-end test parity on Windows 11 (GNU toolchain).
- Windows: WSAPoll/select replaces IOCP shim; stale-fd close on disconnect.

---

## [Unreleased] — 0.1.4-dev

Active development. See [RELEASING.md](RELEASING.md) for the roadmap and
[docs/TODO_CURRENT.md](docs/TODO_CURRENT.md) for the current work queue.

### Performance — SSA LLVM backend optimisations (v0.6, 2026-07-03)

Three `alwaysinline` changes that let LLVM's LICM and ConstraintElimination
passes work across function-call boundaries:

- **`@__intent_bounds_check` always-inline + `@llvm.assume`**: bounds check
  is expanded inline at every `xs[i]` site. GVN can now eliminate duplicate
  checks in the same block; ConstraintElimination eliminates checks where the
  loop condition already implies `idx < len` (BFS outer loop: `while head <
  queue.len { curr = queue[head] }`).

- **`set_mut` always-inline**: `set(mut ref xs, i, v)` expands to an inline
  GEP + store. LLVM LICM then hoists the data-pointer load out of enclosing
  while-loops, giving the sieve inner loop register-resident base address —
  matching C's direct array-index throughput.

- **`push_mut` always-inline**: `push(mut ref xs, v)` expands inline. LLVM
  sees the grow-path branch as unlikely and keeps Vec fields in registers
  across BFS queue iterations.

Results vs v0.5:

| Benchmark | v0.5 | v0.6 | Δ |
|-----------|------|------|---|
| Sieve | 66.8 ms | 51.4 ms | −23 % |
| BFS | 56.1 ms | 43.5 ms | −22 % |
| HashMap | 65.2 ms | 50.8 ms | −22 % |
| Array stats | 106.2 ms | 82.0 ms | −19 % |
| Parallel sum | 556.1 ms | 474.3 ms | −15 % |
| Fibonacci | 1028 ms | 875.9 ms | −15 % |

---

### Performance — thread-local reduction accumulation (v0.5, 2026-07-01)

Replaced per-element `atomicrmw seq_cst` ops in `parallel for … reduce`
regions with **per-thread stack-local accumulators**. The parallel body now
accumulates into a non-atomic local; a single `atomicrmw` (or CAS loop for
`*`) per thread combines the result at the parallel region's exit.

Results vs v0.4:

| Benchmark | v0.4 | v0.5 | Δ |
|-----------|------|------|---|
| Parallel sum (50 M elems) | 1300 ms | 556 ms | −57 % |
| Array statistics (10 M elems) | 499.7 ms | 106.2 ms | −79 % |
