# Changelog

All notable changes to vāṇी (vanic) are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Versions follow [Semantic Versioning](https://semver.org/).

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

## [Unreleased]

Next planned release: `0.1.1` — patch fixes post-0.1.0, `forall` nice-to-haves,
Kosh `cafile` for private registries. See [RELEASING.md](RELEASING.md).
