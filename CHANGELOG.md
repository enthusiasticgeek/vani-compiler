# Changelog

All notable changes to vāṇी (vanic) are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Versions follow [Semantic Versioning](https://semver.org/).

---

## [v0.4.4] — 2026-07-13

### Changed

- **README.md** — benchmark catalogue and results links added to Key docs table.
- **`docs/language_manual.md`** — four correctness fixes:
  - `for i in 0 to n` corrected to `for i from 0 to n` (canonical range syntax).
  - `parallel for` reduce syntax fixed to `reduce sum with +;` form.
  - `Channel<T, N>` added to Concurrency section with example and primitive-selection table.
  - Editor integration (LSP) expanded: names `intent-lsp` binary, links to
    per-editor setup guide in `tutorials/src/installation.md`.

---

## [v0.4.3] — 2026-07-13

### Changed

- **README.md restructured** — 5 108 lines condensed to ~120. Removed internal
  arc ledger, duplicate language reference, and scattered sections. Now a clean
  landing page with one example, install snippet, and links to real docs.

### Added

- **`docs/language_manual.md`** — full language reference: types, ownership,
  control flow, closures, SMT, safety attributes, SIMD, file I/O, bare-metal,
  tooling, FFI, and glossary.
- **`docs/languages.md`** — all 57 human-language dialects with per-dialect
  native-speaker verification status. Section renamed from
  "Indian-subcontinent-first" to "Global Language Coverage".
- **`docs/philosophy.md`** — design rationale, affine ownership rationale,
  Rust/C++ comparisons, and why Rust for the compiler core.

---

## [v0.4.2] — 2026-07-13

### Added

- **`scripts/sync_version.py`**: single source of truth for the project version.
  Reads `Cargo.toml` and patches the "Current version" line in `RELEASING.md`
  and the version line in `TODO.md`. Run after every Cargo.toml bump.
- **`.githooks/pre-commit`**: blocks commits when any version-bearing doc is
  stale relative to `Cargo.toml`. Install once per clone:
  `git config core.hooksPath .githooks`.

---

## [v0.4.1] — 2026-07-13

### Added

- **`Vec<f64>` builtin parity (F64-2 – F64-5)**: `vec_sum`, `vec_mean`,
  `vec_min`, `vec_max`, `vec_argmin`, `vec_argmax`, `vec_median`,
  `vec_kth_smallest`, `vec_fold`, `vec_map`, `vec_filter`, `vec_swap`,
  and `vec_dot` now accept `Vec<f64>` arguments. Both the C and LLVM
  backends emit dedicated `double`-typed helpers. All operations that
  previously required a `Vec<i64>` now work identically on `Vec<f64>`.
- **`#[no_nan]` safety attribute (T2.4)**: new primitive that rejects
  calls to builtins with a documented NaN-as-error-sentinel contract:
  `f64_nan()` (explicit NaN constructor) and `vec_kth_smallest` on
  `Vec<f64>` (returns quiet NaN `0x7FF8000000000000` on out-of-bounds).
  Automatically implied by `#[asil_d]`, `#[do178c_level_a]`,
  `#[iec_61508_sil3]`, and `#[iec_61508_sil4]`. Composes with
  `#[no_float]`. 6 lib tests cover rejection, acceptance, nesting,
  and composite implication.

---

## [v0.3.1] — 2026-07-11

### Fixed — CI / Test Harness

- **AArch64 and RISC-V CI**: added `z3` to `apt-get install` in both QEMU
  jobs; all `smt::tests::*` and `*_invariant_keyword_compiles` tests now pass
  on AArch64 and RISC-V (previously failing with "no SMT solver available").
- **`intentc` deprecation warning**: the legacy `intentc` binary alias no longer
  emits its deprecation warning when stderr is not a TTY. CI, scripts, and
  integration tests now get clean stderr; interactive terminal users still see
  the hint.
- **`parser.rs` doctest**: grammar notation in a doc comment was being executed
  as a Rust doctest, failing with `E0762: unterminated character literal` on
  Devanagari single-quoted strings. Fenced the block as ` ```text `.
- **Amharic example files**: `keywords.vani` and `verified.vani` used `ለ`
  (the Amharic lexer keyword for `for`) as a parameter name, causing a parse
  error in the formatter. Renamed to `ሎ`.
- **RISC-V env-var race**: `complexity_pass_silent_without_opt_in` now holds
  `env_lock()` and clears `INTENT_MAX_COMPLEXITY` / `INTENT_CHECK_COMPLEXITY`
  before calling `enforce_complexity`, preventing leakage from
  `complexity_pass_runs_when_invoked_directly` under QEMU sequential scheduling.
- **Cargo.toml version**: aligned to `0.3.1` — the `v0.3.0` release tag was
  created without bumping `Cargo.toml` (remained at `0.2.3`); this release
  corrects the drift.

### Tests — Known Issues Tracked (marked `#[ignore]`)

Seven integration tests are now explicitly ignored pending underlying fixes:
- `run_parallel_example_proves_race_free_and_runs` — LLVM IR emits atomic load
  without alignment; `lli` rejects it.
- `windows_snprintf_dprintf_shim_roundtrip` — LLVM IR undefined value
  `%t3.c.addr` in `echo_p3b_str_local`'s snprintf path.
- `windows_tcp_echo_blocking_three_clients` — LLVM IR undefined value
  `%t3.fd.addr` for TCP fd locals in `tcp_multi_echo`.
- `intentc_test_expands_directory_arg_to_intent_files` and
  `intentc_test_passes_for_all_examples_and_fails_on_violated_assertion` —
  `echo_with_timeout.vani` LLVM IR has undefined value for async TCP locals.
- `intentc_check_accepts_directory_and_summarizes` — some example `.vani` files
  with `prove` statements fail z3 verification at `check` time.
- `llvm_backend_run_produces_same_output_as_c` — `tcp_echo.vani` LLVM IR has
  undefined values for socket locals.
- `fmt_roundtrips_every_example` — multiple translated example files have
  pre-existing formatter parse errors (keyword-as-identifier, `Box<Vec<T>>`
  type parsing). Amharic files fixed; remaining languages need systematic audit.

Two lib tests conditionally ignored on non-x86 targets:
- `mixed_place_assign_leaf_owned_str_emits_drop` and
  `mixed_place_assign_leaf_vec_emits_drop` — C backend drop emission for
  mixed-place index+field assign is architecture-dependent; `free()` call not
  emitted on AArch64 or RISC-V. Investigation tracked.

---

## [0.2.3] — 2026-07-05

### Performance — Builtins & Hash

- **`vec_fill(n, val)` builtin**: new `Vec<T>` bulk-initializer — allocates with
  `malloc` then fills with `memset` (for `i8` elements) or a phi-based fill loop
  (for wider types). Eliminates the 2M sequential `push()` calls in the sieve
  benchmark. Sieve gap vs C: 1.25× → **vāṇī 4% faster** (13.1ms vs 13.5ms).
  Available in both tree-LLVM and C backends. SSA-LLVM backend correctly routes
  programs using `vec_fill` or `set(mut ref ...)` to tree-LLVM.
- **Multiply-shift hash**: replaced the FNV-1a 8-step byte loop in all hashmap/
  hashset `__hash_key` functions (i64 and f64 key variants) with a 3-instruction
  PCG multiply + XOR-fold (`k * 6364136223846793005 ^ (result >> 33)`). Reduces
  per-hash cost from ~48 operations to 3; hashmap benchmark: 42ms → 41ms.
- **SSA-LLVM routing fix**: `set_mut` (in-place `set(mut ref ...)`) was not in
  the SSA-LLVM reject list, causing programs without `push_mut` to fall through
  to the SSA backend which emits undefined `@fn_set_mut`. Added `set_mut` and
  `vec_fill` to the reject list; affected programs now correctly use tree-LLVM.

---

## [0.2.2] — 2026-07-05

### Performance — Compiler & Benchmarks

- **`opt -O3` / `llc -O3`**: upgraded the LLVM pipeline from `-O2` to `-O3`,
  enabling more aggressive inlining, loop unrolling, and auto-vectorization.
  Fibonacci gap closed from 1.90× → 1.57×; matrix now at parity with C;
  parallel sum and array statistics now beat C (vāṇī faster).
- **Benchmark correctness fix**: all C/C++ benchmark files used bare `long`
  which is 32-bit on Windows/MinGW, giving wrong outputs and an unfair 2×
  SIMD advantage (32-bit fits 2× more elements per AVX2 register than 64-bit).
  Changed all 20 C/C++ files to `int64_t` / `PRId64` (`fib`, `sieve`,
  `matmul`, `sort`, `graph_index`, `graph_weakptr`, `parsum`, `hash`, `list`,
  `alloc`, `stats`). With a fair comparison, vāṇī beats C in 6/10 benchmarks.
- **Fibonacci gap analysis**: the remaining 1.57× gap vs GCC is due to GCC's
  O3 recursive tree-inlining pass (3–4 levels inline, ~25 labels / 337 ASM
  lines for 2 real call sites). LLVM has no equivalent pass; the gap is ~φ
  (≈ 1.618) as predicted. vāṇī already beats GCC with `noinline` (866ms vs
  1116ms), confirming LLVM's tail-call elimination gives a 1.29× advantage
  over naive recursion.
- **Benchmark runner**: fix stale `opt -O2` header; fix array statistics
  description to note vāṇī uses parallel reduces while C/C++/Rust are
  sequential.

---

## [0.2.1] — 2026-07-04

### Performance — Compiler & Benchmarks

- **Sieve Vec<i8>**: switched sieve benchmark from `Vec<i64>` to `Vec<i8>`
  (8× smaller working set fits L1/L2 instead of spilling to L3); sieve gap vs C
  closes from 3.35× to 1.03× (near parity).
- **`hashmap_with_capacity(n)`**: new builtin pre-allocates a power-of-two slot
  table ≥ 2n, eliminating incremental grow rehashing; hashmap gap closes from
  1.94× to ~1.0× parity with C. Wired in checker, LLVM backend, and C backend.
- **FNV-1a hash restored**: reverted a Fibonacci-hash experiment (`k * Φ`) that
  caused a 4× hashmap regression; FNV-1a iterative hash reinstated across all 6
  codegen sites (4 template + 2 concrete).
- **Runtime thread count**: parallel-for thread default now reads
  `std::thread::available_parallelism()` at compile time instead of a hardcoded
  4; `OMP_NUM_THREADS` env-var still overrides. Host builds scale automatically.
- **Alloca hoisting**: scalar `let` allocas moved to function entry block; local
  accumulators in outlined parallel-for functions hoisted similarly, enabling
  LICM and better register allocation.
- **Benchmark runner**: fix Unicode crash on Windows cp1252 console; auto-detect
  vanic from repo `target/release/`; upgrade C/C++ to `-O3 -march=native` and
  Rust to `-C opt-level=3 -C target-cpu=native` for a fair native comparison;
  deleted stale `RESULTS_SAMPLE.md`; regenerated `RESULTS.md` with 4-language
  (C / C++ / Rust / vāṇī) comparison table.

---

## [0.2.0] — 2026-07-01

### Added — Tooling

- **`vani_translate.py` v3 — 57-language translator** — expanded from 5 to 57
  supported languages; SOV word-order rewriting for 20 languages; `--llm` flag
  for AI-assisted translation of comments, strings, and identifiers via
  Anthropic / OpenAI / Ollama backends.
- No compiler or language changes.

---

## [0.1.9] — 2026-06-23

### Added — Language

- **Named loop labels** — `label: for/while` + `break label` + `continue label`;
  any nesting depth; undefined label produces a clear compile error. Both the
  SSA-C and LLVM backends emit correct labeled exits.
- Supersedes the positional break keywords (`break inner/middle/outer`)
  introduced in v0.1.8; plain `break` / `continue` still target the innermost
  loop.
- 3 new adversarial tests.

---

## [0.1.8] — 2026-06-23

### Added — Language

- **Block comments `/* ... */`** — multi-line, arbitrarily nested; unterminated
  comment produces a clean diagnostic (no panic).
- **Print block `print { ... }`** — groups multiple print lines under one
  `print` keyword; each `;`-terminated item becomes a separate output line.
- **Positional break** — `break inner` / `break middle` / `break outer` exit a
  loop by nesting position. (Superseded by named labels in v0.1.9.)
- Both C and LLVM backends; 8 new adversarial tests.

---

## [0.1.7] — 2026-06-21

### Added — Docs / Tutorials

- **10 new tutorial pages (~1 650 lines)** covering: function pointers as
  first-class values, native file I/O primer + worked examples, advanced math
  library deep-dive, Vec statistics and combinators, condition variables,
  cross-compilation primer, function attributes reference, advanced collections
  (Graph/BST/Trie/SkipList/UnionFind/BloomFilter/Deque), and full `vanic` CLI
  reference.
- No compiler changes; test count unchanged.

---

## [0.1.6] — 2026-06-21

### Added — Language / Tooling

- **`--target=<triple>`** — cross-compilation to arbitrary LLVM targets; passes
  `--mtriple` to `llc`; selects cross-linker via `$CROSS_CC` or `<triple>-gcc`;
  bare-metal triples (containing `none` or `eabi`) suppress libc/OpenMP/pthread
  linker flags.
- **`--no-std` (C backend)** — suppresses all `#include` lines; emits a minimal
  typedef preamble (`uint8_t`, `int64_t`, `size_t`, `uintptr_t`, `NULL`);
  auto-activates for bare-metal triples.
- **`#[link_section = "..."]`** — places function in the named linker section
  (C: `__attribute__((section(...)))`; LLVM IR: `section "..."`).
- **`#[no_mangle]`** — suppresses `intent_` prefix and Unicode mangling in both
  backends; linker scripts can reference the bare vāṇी function name.
- **`mmio_read_u8` / `mmio_write_u8`** — 8-bit volatile MMIO builtins.
- **`mmio_read_u16` / `mmio_write_u16`** — 16-bit volatile MMIO builtins.
- **QEMU user-mode run** — `vanic run --target=<linux-triple>` transparently
  invokes `qemu-<arch>-static`.
- Resolves L19 in `docs/v1_limitations.md` (all 5 bare-metal sub-gaps: target
  flag, no-std, no_mangle, link_section, narrow MMIO widths).

---

## [0.1.5] — 2026-06-21

### Added — Language

- **Native file I/O** — `FileHandle` affine RAII type; automatically `fclose`d
  at scope exit. New builtins: `file_open`, `file_is_ok`, `file_read_line`,
  `file_write`, `file_close`, `file_flush`, `stdin_read_line`, `flush_stdout`.
  Both C and LLVM backends.
- **`eprint` statement** — writes to stderr; syntax identical to `print`; both
  backends route through `fprintf(stderr, ...)`.
- Resolves L18 in `docs/v1_limitations.md` ("native file I/O").

---

## [0.1.4] — 2026-06-20

### Fixed — Compiler

- **Non-Copy elements in tuples** — `(OwnedStr, i64)` and similar tuples
  containing non-Copy types were previously rejected with "v1 tuples are
  Copy-only"; now fully supported across both backends. `ast.rs`,
  `checker.rs`, `backend_c.rs`, and `backend_llvm.rs` all updated.
- **`Box<T>` as enum variant payload** — `Option<Box<T>>` and any enum with a
  `Box<T>` payload were rejected by the checker; now accepted. Scope-exit and
  per-slot drop handlers added in both backends.

### Fixed — Docs

- Tutorial site broken-link sweep (19 files): wrong GitHub repo URLs, wrong
  relative paths, ellipsis placeholder URLs — all repaired.
- `SUMMARY.md` duplicate `installation.md` entry removed (caused mdBook to
  render the page starting from an anchor, dropping all content above it).

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

## [v0.3.0] — 2026-07-10

Active development. See [RELEASING.md](RELEASING.md) for the roadmap and
[docs/TODO_CURRENT.md](docs/TODO_CURRENT.md) for the current work queue.

### Feature — QEMU system-mode bare-metal integration (SIMD-10, 2026-07-10)

`vanic run` now supports QEMU system-mode via `--qemu-machine=<board>`. For
bare-metal triples (`arm-none-eabi`, `riscv32-unknown-none-elf`, etc.), adding
`--qemu-machine=lm3s6965evb` (or any supported board) builds an ELF and
launches `qemu-system-arm -machine lm3s6965evb -nographic -semihosting -kernel
<elf>` automatically. Supported boards: `lm3s6965evb`, `mps2-an385` (ARM),
`sifive_e`, `sifive_u` (RISC-V 32), `virt` (AArch64). Env-var override:
`QEMU_SYSTEM_<ARCH>`. Six new unit tests in `src/main.rs`.

### Feature — `vec256<T>` and `simd256_*` builtins (SIMD-9, 2026-07-10)

Added `vec256<T>` — a 256-bit SIMD register type — alongside seven
`simd256_*` builtins (`simd256_splat`, `simd256_load`, `simd256_store`,
`simd256_add`, `simd256_sub`, `simd256_mul`, `simd256_reduce_add`). Lane
counts are twice those of `vec128<T>` (e.g. `vec256<f32>` has 8 lanes). LLVM
IR type is `<N x T>`; on x86-64+AVX2 this lowers to `ymm` registers; on
AArch64 without SVE, to two 128-bit NEON registers. Eight files changed;
two lib tests; three edge-case files (`mix_simd256_basic.vani`,
`mix_simd256_i32_mul.vani`, `xfail_simd256_type_mismatch.vani`).

### CI — AArch64 and RISC-V QEMU lib-test jobs (SIMD-6, SIMD-7, 2026-07-10)

Added two CI jobs to `.github/workflows/ci.yml`:

- **`test-aarch64-qemu`**: `cargo test --lib --target aarch64-unknown-linux-gnu`
  under `qemu-aarch64-static`. Validates parser, type-checker, SSA lowerer, and
  both backends on emulated AArch64. vanic is pure Rust (no native LLVM
  dependency) so cross-compilation requires no special toolchain.
- **`test-riscv64-qemu`**: `cargo test --lib --target riscv64gc-unknown-linux-gnu`
  under `qemu-riscv64-static`. Validates correctness on emulated RV64GC.

Both jobs run on every push to `main` via `ubuntu-latest`.

### Test — `ARR` bucket confirmed live (SIMD-8, 2026-07-10)

Fixed-size arrays (`[T; N]` type, `[e0, …]` literal, `a[i]` indexing) are
confirmed working. Added `mix_arr_indexing.vani` (ARR+SCAL) and
`mix_arr_struct_elems.vani` (ARR+STRT) to `examples/edge_cases/`.
`TEST_MATRIX.md` ARR row is now filled. Pin raised 87 → 89.

### Performance — SSA LLVM backend alwaysinline optimisations (2026-07-03)

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

Results vs thread-local baseline (2026-07-01):

| Benchmark | before | after | Δ |
|-----------|--------|-------|---|
| Sieve | 66.8 ms | 51.4 ms | −23 % |
| BFS | 56.1 ms | 43.5 ms | −22 % |
| HashMap | 65.2 ms | 50.8 ms | −22 % |
| Array stats | 106.2 ms | 82.0 ms | −19 % |
| Parallel sum | 556.1 ms | 474.3 ms | −15 % |
| Fibonacci | 1028 ms | 875.9 ms | −15 % |

---

### Performance — thread-local reduction accumulation (2026-07-01)

Replaced per-element `atomicrmw seq_cst` ops in `parallel for … reduce`
regions with **per-thread stack-local accumulators**. The parallel body now
accumulates into a non-atomic local; a single `atomicrmw` (or CAS loop for
`*`) per thread combines the result at the parallel region's exit.

Results vs prior baseline:

| Benchmark | before | after | Δ |
|-----------|--------|-------|---|
| Parallel sum (50 M elems) | 1300 ms | 556 ms | −57 % |
| Array statistics (10 M elems) | 499.7 ms | 106.2 ms | −79 % |
