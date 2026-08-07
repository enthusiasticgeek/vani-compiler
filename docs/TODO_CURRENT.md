# vāṇी — Current Work Queue

Actionable items fully within our control, ordered by effort.
Blocked items (macOS hardware, grammar consultant, IOCP) are at the bottom.

Last updated: 2026-07-31

---

## Immediate (< 1 h)

- [x] **23. Block comments `/* ... */`** — Multi-line, nestable; unterminated → clean diagnostic. ✅ done 2026-06-23 (lexer `skip_block_comment` with depth counter; both backends)

- [x] **24. Print block `print { ... }`** — Group multiple print lines under one `print`; each `;`-group → separate line. ✅ done 2026-06-23 (parser `Stmt::PrintBlock`; checker desugars to `TypedStmt::Print`; C + LLVM; format.rs + zero_stmts updated)

- [x] **25. Positional break `break inner/middle/outer`** — Exit a specific enclosing loop by position without naming it. ✅ done 2026-06-23 (parser 2-token lookahead; checker assigns `__vani_pos_N` synthetic labels; SSA lowerer + LLVM backend search by label; tree-C emits `goto __vani_break_name`)

- [x] **26. Tutorial coverage expansion (v0.1.7)** — 10 new pages: CLI ref, FnPtr primer, file I/O primer + worked, math deep-dive, vec stats, condvar primer, cross-compile primer, attributes reference, advanced collections. ✅ done 2026-06-21

- [ ] **1. Publish to crates.io** — `cargo publish`. All required fields present in
  `Cargo.toml`. Gives `cargo install vanic` to Rust users. See
  [docs/decisions.md](decisions.md) for rationale.
  **BLOCKED**: needs crates.io API token (`cargo login <TOKEN>` or `$env:CARGO_REGISTRY_TOKEN`).
  v0.1.2 is tagged and ready; run `cargo publish` from repo root once token is available.

- [x] **2. Update RELEASING.md** — Point at `0.1.2-dev`; document `RELEASE_NOTES/`
  workflow and `body_path` release step. ✅ done 2026-06-19

- [x] **3. Remove `intentc` legacy binary** — Delete `[[bin]] name = "intentc"` from
  `Cargo.toml` at next release boundary (v0.1.x → v0.2 or when the release cycle
  ends). Add a compiler warning to `main.rs` when invoked as `intentc`. ✅ done 2026-06-19
  (deprecation warning added to run(); [[bin]] intentc removal deferred to v0.2.0 boundary)

---

## Short (2–4 h each)

- [x] **4. Add 4 missing Devanagari aliases to lexer** — `extern` / `type` / `intent`
  / `invariant` are shown in the README table but may not be wired in `lexer.rs`.
  Verify + add if missing; add lib tests. ✅ done 2026-06-19 (all 4 already wired; added tests for प्रकार + बाह्य)

- [x] **5. Groom `docs/v1_limitations.md`** — Mark limitations resolved since
  2026-06-09 ✅; add entries for parametric `Mutex<T>` (no longer i64-only),
  `Barrier`, `RwLock<T>/ReadGuard/WriteGuard`. ✅ done 2026-06-19 (L15/L16/L17)

- [x] **6. Tutorial: Barrier primer** — `tutorials/src/advanced/02b_barrier_primer.md`.
  Same format as `02a_parallelism_primer.md`. Add to `SUMMARY.md`. ✅ done 2026-06-19

- [x] **7. Tutorial: RwLock primer** — `tutorials/src/advanced/02c_rwlock_primer.md`.
  Add to `SUMMARY.md`. ✅ done 2026-06-19

- [x] **8. Tutorial: default methods + blanket impls primer** —
  `tutorials/src/intermediate/04d_default_methods_primer.md`. Add to `SUMMARY.md`. ✅ done 2026-06-19

- [x] **9. Update `tutorials/src/SUMMARY.md`** — Add the three new primer entries
  above to the book index. ✅ done 2026-06-19

---

## Medium (4–8 h each)

- [x] **10. Condense `STATUS.md` / `TODO.md`** — Both are 500 KB+. Extract
  pre-Arc-8 shipped history to `STATUS_ARCHIVE.md` / `TODO_ARCHIVE.md`. Keep main
  files as current-state ledgers. ✅ done 2026-06-19 (STATUS.md: 11741→306 lines; TODO.md: 10585→40 lines)

- [x] **11. A.2 Examples reorganization** — Verify all Devanagari examples live under
  `examples/language/{sanskrit,hindi,marathi}/`; add `// श्री।` header to each.
  Move any English examples not yet under `examples/language/english/`. ✅ done 2026-06-19
  (14 Sanskrit + 12 Hindi + 12 Marathi — all have // श्री। header; moved path_c_ref_returns.vani
  and vec_of_ref.vani from examples/ root to examples/language/english/)

- [x] **12. Arc 7 Win64 / AArch64 ABI** — Complete float-class + mixed struct
  Win64 struct-return classifier (~6–8 h). Code work only; CI wiring is separate.
  ✅ done 2026-06-19 (is_ffi_safe_struct_win64: size∈{1,2,4,8}; is_ffi_safe_struct_aarch64:
  HFA + scalar≤16; platform-dispatching is_ffi_safe_struct; platform-specific error hints;
  7 new lib tests gated by cfg(target_arch/os))

- [x] **13. Finer Sanskrit / Hindi / Marathi purity gate** — Tighten the
  `// vani-lang:` pragma in `lexer.rs` to distinguish the three dialects (currently
  only English vs Devanagari at script level). ✅ done 2026-06-19
  (gate already implemented via `spelling_supports_dialect`; updated stale doc comment
  in `enforce_language_purity`; added 2 new dialect-rejection tests:
  `dialect_gate_marathi_pragma_rejects_sanskrit_only_keyword` +
  `dialect_gate_hindi_pragma_rejects_marathi_only_keyword`)

---

## Bare-metal / OS (high priority — L19)

These five items together unlock vāṇी as the primary language for a
custom OS or bare-metal board firmware. See
[L19 in docs/v1_limitations.md](v1_limitations.md) for full context,
workarounds, and the exact design goal for each.

- [x] **18. `--target <triple>` cross-compilation flag** (G1 — highest impact) ✅ done 2026-06-21
  `vanic build --target=<triple>` passes `--mtriple=<triple>` to `llc`;
  selects cross-linker via `$CROSS_CC` or `<triple>-gcc`; bare-metal
  triples suppress libc/OpenMP/pthread link flags and auto-activate no-std.
  `vanic run --target=<triple>` errors helpfully for bare-metal; for Linux
  cross-targets it builds an ELF and runs via QEMU user-mode.
  `is_bare_metal_triple` + `cross_cc_for_triple` helper fns in `src/main.rs`.
  4 new tests: 2 lib (bare-metal LLVM IR + no-std C), 2 binary (triple
  detection + CC derivation). L19 fully resolved in `docs/v1_limitations.md`.

- [x] **19. `--no-std` mode — omit libc prelude in C backend** (G2) ✅ done 2026-06-21
  `--no-std` flag on `vanic emit --backend=c` / `vanic emit-c` suppresses
  all `#include <std*.h>` and emits a minimal bare-metal typedef block.
  Auto-activates when `--target` triple is bare-metal. `NO_STD_MODE`
  thread-local in `src/backend_c.rs`; `emit_c_no_std()` public API in
  `src/lib.rs`.

- [x] **20. `#[link_section = "..."]` attribute** (G3) ✅ done 2026-06-21
  Emits `__attribute__((section("...")))` in C and `section "..."` on the
  LLVM IR `define` line. Parser in `src/parser.rs`; `link_section:
  Option<String>` on `Function` in `src/ast.rs`/`src/ir.rs`.

- [x] **21. `#[no_mangle]` attribute — suppress symbol name mangling** (G4) ✅ done 2026-06-21
  `fn` declarations with `#[no_mangle]` emit the bare vāṇी name in both
  C and LLVM backends. `NO_MANGLE_FN_REGISTRY` / `LLVM_NO_MANGLE_FN_REGISTRY`
  thread-locals track which functions are bare so call-sites use the right
  symbol.

- [x] **22. MMIO 8-bit and 16-bit variants** (G5) ✅ done 2026-06-21
  `mmio_read_u8`, `mmio_read_u16`, `mmio_write_u8`, `mmio_write_u16` now
  ship. Lowers to `*(volatile uint8_t*)` / `*(volatile uint16_t*)` in C;
  volatile `i8`/`i16` load/store with `zext`/`trunc` in LLVM IR.

---

## Larger (dedicated session)

- [ ] **14. Homebrew formula** — `homebrew-vanic` tap repo. **Gate**: wait until
  macOS is empirically verified on a Darwin host.

- [x] **17. Native file I/O — eliminate FFI workaround for flat files + stdin** ✅ done 2026-06-21
  ([L18 resolved in docs/v1_limitations.md](v1_limitations.md)).
  Ships: `FileHandle` (affine RAII, auto-fclose at scope exit), `file_open`, `file_is_ok`,
  `file_read_line`, `file_write`, `file_close`, `file_flush`, `stdin_read_line`,
  `flush_stdout`, `eprint` statement — both C and LLVM backends, 5 lib tests.
  Device I/O (UART/I2C/SPI/RS485) stays FFI + C-shim by design (kernel ioctl
  ABI is platform-specific and aggregate-by-value).

- [x] **15. B.1 Cross-language `.vani` translator CLI** — `tools/vani_translate.py`
  already has `ALIASES`; build a proper round-trip CLI (~4–6 h). ✅ done 2026-06-19
  (auto-detect source lang from pragma; --verify round-trip flag; --list-keywords markdown
  table; --batch directory mode; --inplace with .bak backup; UTF-8 stdout fix for Windows;
  tested: 166/166 english→marathi batch, verify english↔hindi↔english, english↔sanskrit↔english)

- [x] **16. C.x SOV completion (mechanical parser side)** — Verb-at-end shapes for
  `let` / `fn` / `if` / `while` / `match` / `struct` / `enum` (~10–15 h). Grammar
  consultant review is separate; this is just the parser work. ✅ done 2026-06-19
  (looks_like_sov_fn/struct/enum detectors; parse_sov_fn token-rewrite + parse_function reuse;
  parse_sov_struct_decl + parse_sov_enum_decl with optional generics;
  parse_match_arms_block refactor; SOV match at stmt pos → helpful error;
  wired in top-level + module-body dispatchers; 3 new lib tests pass)

- [ ] **27. Inline `print`-item format specs (Rust `{:03}` / `{:.2}` syntax)** —
  `print` currently takes a flat comma-separated list of string-literal-or-expr
  items (`parser.rs::parse_print_item`, `PrintItem::{Str,Expr}`) — there's no
  template-string / placeholder mini-language. `f64_to_str_fixed(x, decimals)`
  (item 27.1 below) plus `str_pad_left(i64_to_str(n), w, "0")` already cover the
  *capability* (fixed-decimal floats, zero-padded ints) as ordinary function
  calls; this item is specifically about adding literal `{:03}`-shaped syntax
  at a print call site.

  **Design question, still open**: this would be the first "magic syntax
  inside/beside a print item" feature in the language — worth a deliberate
  decision (does it fit vāṇी's explicit-over-implicit posture, e.g. mandatory
  `unsafe(reason=...)`, no operator-overloading magic?) rather than adding it
  as a side effect of wanting decimal padding, which the cheap path already
  solves. Gate on: is there real demand for the syntax itself, not just the
  formatting capability. **Not decided; scoped below on the assumption it's
  picked up, so implementation can start the moment it is.**

  **Proposed grammar** (scoped 2026-07-25, not implemented): a postfix spec
  after the expr, `print x:03;` / `print y:.2;` / `print z:08.3;`, grammar
  `'0'? WIDTH? ('.' PRECISION)?`, hand-parsed as raw tokens right after `:`
  rather than reusing `parse_expr()`/number-literal lexing (`.2` isn't
  otherwise a lexable float literal in vāṇी, and `:` is never consumed by
  general expression parsing — checked every `TokenKind::Colon` site in
  `parser.rs`; all are struct-field/type-annotation/label contexts — so this
  is grammatically unambiguous to add). Semantics: WIDTH pads any numeric
  type (zero or space per the `0` flag); PRECISION only on `f32`/`f64`,
  fixed notation, same logic `f64_to_str_fixed` already implements; a spec
  on `bool`/`Str`/`OwnedStr`/aggregate types is a checker diagnostic, not a
  silent no-op or a crash; width/precision are compile-time integer literals
  only (no `x:0{n}` with a runtime `n`) — deliberately, since it keeps every
  codegen site building a literal printf format string instead of one
  assembled at runtime.

  **Scope, if picked up** (traced all ~160 `PrintItem`/`TypedPrintItem` match
  sites across the 13 files on 2026-07-25; most are mechanical, the real
  work concentrates in five places):
  - **Grammar** (`parser.rs`): new `parse_format_spec()` call after the expr
    in `parse_print_item`. Small, self-contained. ~2-3h together with the
    `vanic fmt` item below.
  - **Pretty-printer** (`format.rs`): 2 real sites (~L951, ~L972) re-emit
    `:spec` when present; the other 2 matches in the same file (~L1774,
    ~L1783) are span-zeroing for diffing and don't need to change.
  - **Type-checking** (`checker.rs`, ~4-6h): **6 duplicated construction
    blocks**, not the 2 you'd expect — `Print`/`EPrint` and `PrintBlock` are
    each type-checked twice, once around L10886-10973 and again around
    L17608-17775 (a second verification pass). Each needs the new
    width/precision-vs-type validation and a new diagnostic for
    incompatible combinations; worth factoring into one shared helper while
    doing this rather than copy-pasting the check 6 times. The other ~55
    matches in this file (purity/effects/mentions-var walkers) are
    mechanical — they walk the inner expr and don't care about the spec.
  - **Codegen, tree backends** (`backend_c.rs`, `backend_llvm.rs`, ~3-4h):
    both already dispatch the printf format string purely on `expr.ty` in
    one function each (`emit_print_expr_no_newline` and its LLVM twin) —
    threading a spec through just adds a parameter and a few
    `format!("%0{w}lld")`-style branches. The easy half, as originally
    guessed.
  - **Codegen, SSA backends** (`ssa.rs`, `ssa_backend_c.rs`,
    `ssa_backend_llvm.rs`, ~4-6h — **the fiddly half, highest design
    risk**): SSA lowers every print item to a generic `Call { name:
    "intent_print_item", args: [op] }`, and both SSA backends special-case
    that exact name string to inline-dispatch a printf format string on the
    operand's static type at the call site — there's no spec slot in that
    instruction today. Recommended mechanism: when a spec is present,
    `ssa.rs` mangles the call name instead of adding args (e.g.
    `intent_print_item$w3z0`), and both backends' existing `if name ==
    "intent_print_item"` blocks gain a sibling branch decoding the mangled
    suffix. This keeps the untouched-common-case path byte-identical —
    same zero-regression-risk lesson BUG-5 / L25 just taught for the
    Windows printf shims. Needs one shared encode/decode helper used by all
    3 sites so the mangling scheme can't drift between them.
  - **Mechanical updates** (~1-2h, ~120 sites, no design risk): `safety.rs`
    (13), `lsp.rs` (5), `stack_depth.rs`, `deviations.rs`,
    `hashmap_bundle.rs`, `main.rs`'s SSA-fallback-gate walkers (9),
    `ssa.rs`'s non-lowering matches, `lib.rs` — all just walk the
    expression inside a print item (purity, effects, hover, hashing,
    hardware-gate checks) and don't care about the new field; adding it
    breaks their exhaustive match, fixed with `, _` / `, spec` at each site.
    `cargo build` walks you to every one.
  - **Tests** (~3-4h): parser (spec parses / rejects malformed spec),
    checker (valid spec+type combos accepted, invalid ones rejected with a
    diagnostic), all 4 backends compiling+running a formatted print and
    checking the exact output string (mirror the
    `lli_runs_print_f64_scientific_notation_matches_c_backend_exponent_width`
    pattern from BUG-5 / L25), `vanic fmt` round-trip.
  - **Docs** (~1-2h): `06_strings.md` + this item's closure writeup.

  **Total estimate: ~3-4 focused days** (revises the earlier flat "multi-day"
  guess into where the days actually go — checker duplication and the
  SSA-backend name-mangling plumbing are the two real risk/effort centers,
  not parsing or the tree backends).

  **27.1 (done 2026-07-23)**: `f64_to_str_fixed(x, decimals) -> OwnedStr` —
  the cheap half of this ask, shipped as an ordinary builtin. Checker
  (`check_str_builtin`), both tree backends (`intent_f64_to_str_fixed` via
  two-pass `snprintf(NULL,0,...)` + malloc, in both `backend_c.rs` and
  `backend_llvm.rs`), 4 tests (typecheck+compile, helper-emission, wrong-arity
  reject, both in `lib.rs`). **Non-obvious gotcha found + fixed**: `vanic
  run`/`vanic build` don't call the tree backends directly — `main.rs`'s
  `emit_llvm_via_ssa`/`emit_c_via_ssa` try the SSA pipeline
  (`ssa_backend_llvm.rs`/`ssa_backend_c.rs`) first, and neither SSA backend's
  `Call` lowering has an error path for an unrecognized builtin name — it
  silently assumes "must be a user function" and mangles the callee to
  `fn_f64_to_str_fixed`, producing a program that fails at LLVM-verify/link
  time (undefined symbol) instead of at compile time. The existing
  `lib.rs` test suite (`compile_to_c`/`compile_to_llvm`) calls the tree
  backends directly and wouldn't have caught this. Fixed by adding
  `stmt_calls_f64_to_str_fixed`/`expr_calls_f64_to_str_fixed` (exhaustive
  `TypedStmt`/`TypedExpr` walkers, `main.rs`) and wiring them into both
  `ssa_llvm_extra_reject` and `ssa_c_extra_reject` so programs using this
  builtin fall back to the tree backends (which do implement it) — the same
  established pattern already used for payloaded enums / `Vec<Atomic|Channel>`.
  Added a dedicated regression test
  (`f64_to_str_fixed_falls_back_to_tree_backends_from_ssa_dispatch` in
  `main.rs`) asserting `emit_llvm_via_ssa`/`emit_c_via_ssa` output actually
  contains `intent_f64_to_str_fixed`, not `fn_f64_to_str_fixed` — this is the
  only test in the suite that exercises the SSA-dispatch layer for this
  builtin; **any future SSA-LLVM/SSA-C work that adds real support for this
  builtin should keep (or replace with an equivalent) that assertion**, since
  removing the reject-gate without adding real SSA support would silently
  reintroduce the bug. Verified end-to-end on both backends via `vanic run`
  and `vanic emit --backend=c` + `cc`. Documented in
  `tutorials/src/beginner/06_strings.md`'s string-builtins table.

  **Known caveats** (documented in the tutorial, recorded here for
  anyone touching this builtin later):
  - **Whole-program SSA fallback, not per-function.** `ssa_path_supports`
    gates the *entire* `TypedProgram` on one boolean — one function
    anywhere in the program calling `f64_to_str_fixed` forces
    `emit_llvm_via_ssa`/`emit_c_via_ssa` to tree-codegen the whole file,
    not just that function. Same blast radius as the existing payloaded-enum
    and `Vec<Atomic|Channel>` gates; output is correct either way, but
    worth knowing if you're diffing generated C/LLVM output and a program
    unexpectedly looks tree-shaped.
  - **NaN/Infinity spelling is toolchain-dependent.** Verified on this
    machine (MinGW/MSVCRT): `f64_to_str_fixed(f64_nan(), 2)` → `"1.#R"`,
    `f64_to_str_fixed(f64_inf(), 2)` → `"1.#J"` — legacy MSVCRT strings,
    not C99 `"nan"`/`"inf"`. Confirmed this is inherited from `snprintf`
    itself and not new: `f64_to_str(f64_nan())` already gives
    `"1.#QNAN"` on the same toolchain, so this isn't a regression, just
    something to be aware of before writing a test that asserts an exact
    NaN/Inf string.
  - **Rounding ties away from zero** (`f64_to_str_fixed(0.125, 2)` →
    `"0.13"`), standard C `printf` behavior but not guaranteed to match
    Rust's `{:.2}` bit-for-bit at every halfway case (Rust's formatter
    doesn't call into libc).

---

---

## ARM / AArch64 / NEON work queue

Added 2026-07-06. Full status document: [`docs/arm_neon_status.md`](arm_neon_status.md).

### Thursday evening execution order

| Step | Item | Est. | Gate |
|------|------|------|------|
| 1 | ARM-2 — `--cpu=` flag | ~1 h | none — pure flag wiring |
| 2 | ARM-1 — target-aware `vectorize.width` | ~2 h | none — ARM-2 not required |
| 3 | ARM-5 — bare-metal parallel-for docs | ~1 h | none — docs only, good wind-down task |
| 4 | ARM-3 — AArch64 benchmark run | ~2 h | **needs ARM64 hardware** (Pi 4 / Graviton / M-series) |
| 5 | ARM-4 — SVE / SVE2 opt-in | ~4 h | stretch; ARM-2 required first |
| 6 | ARM-6 — AArch64 CI runner | — | **BLOCKED** — CI budget |

Steps 1–3 fit in a ~4 h evening and require nothing but this laptop.
Step 4 needs hardware — skip or SSH into a remote box.
Steps 5–6 are next session / blocked.

---

### ARM-2 — `--cpu=` flag for llc tuning · **P1 · ~1 h**

- [x] **ARM-2. Add `--cpu=<name>` flag to `vanic build`** ✅ done 2026-07-06
  Today `llc` is invoked with `-mcpu=native` (host builds) or no `-mcpu`
  (cross builds). Cross targets like `aarch64-unknown-linux-gnu` would
  benefit from `-mcpu=cortex-a72` (Pi 4) or `-mcpu=neoverse-n2` (Graviton 3).
  Add `--cpu=<cpu-name>` flag; forward as `-mcpu=<name>` to `llc`.
  Ref: `src/main.rs:3003` (`opt_mcpu` logic in `build_program_llvm`).

### ARM-1 — Target-aware `vectorize.width` hint · **P2 · ~2 h**

- [x] **ARM-1. Emit target-aware loop vectorize width** ✅ done 2026-07-06
  Currently every reduction loop emits `vectorize.width = 4` regardless
  of target. On AArch64 NEON a 128-bit register holds **2×i64**, so width 4
  forces two registers and may confuse the vectorizer. Fix: read the target
  triple in the parallel-for emitter; emit `width = 2` for i64 on AArch64,
  `width = 4` for i32 on AArch64 / i64 on x86-64 AVX2, `width = 8` for i32
  on AVX-512.  Also applies to the `vec_fill` fill loop and the `set` loop.
  Ref: `src/backend_llvm.rs` — `!llvm.loop.vectorize.width` emission sites.

### ARM-5 — Bare-metal parallel-for workaround documentation · **P3 · ~1 h**

- [x] **ARM-5. Document bare-metal parallel-for limitation + FreeRTOS FFI pattern** ✅ done 2026-07-06
  `parallel for … reduce` emits `pthread_create` which doesn't exist on
  `arm-none-eabi`. Add a note to `tutorials/src/advanced/02_parallel.md`
  and `04_embedded.md` with the recommended alternative: manual loop split
  + FreeRTOS `xTaskCreate` via FFI, or single-threaded DMA-offload pattern.

### ARM-3 — AArch64 benchmark run · **P4 · ~2 h + hardware**

- [ ] **ARM-3. Run all 10 benchmarks on AArch64; add to RESULTS.md**
  All current numbers are x86-64 (Windows 11 AMD64). Run on a Graviton 3 /
  Raspberry Pi 4 / Apple Silicon (via cross-build + SSH or native vanic).
  Add an `AArch64` column to `benchmarks/results/RESULTS.md`.
  This validates NEON auto-vectorization quality on a real ARM64 target.

### ARM-4 — SVE / SVE2 opt-in · **P5 · ~4 h (next session)**

- [x] **ARM-4. Pass `+sve` / `+sve2` feature to llc for capable targets** ✅ done 2026-07-06
  Neoverse N2, Graviton 3, and Apple M4 support SVE (scalable vectors).
  Add `--sve` / `--sve2` flags (or auto-detect from `--cpu=neoverse-n2`).
  Forward as `-mattr=+sve` / `-mattr=+sve2` to `llc`.
  Ref: `src/main.rs` MTE path at line 3157 as a model for ISA-extension flags.

### ARM-6 — AArch64 CI runner · **P6 · BLOCKED**

- [x] **ARM-6. Add AArch64 GitHub Actions runner** ✅ done 2026-07-06
  `.github/workflows/ci.yml` added with two jobs: full suite on x86-64
  and lib unit tests on AArch64 via QEMU user-mode (`qemu-aarch64-static`).
  Integration tests excluded from AArch64 job (they spawn cc/llc which are
  host x86-64 binaries). Full end-to-end AArch64 coverage still requires
  real hardware (tracked in ARM-3).

---

---

## SIMD / QEMU follow-up (added 2026-07-10)

Items surfaced during the SIMD hardening + edge-case audit session.
Full context: `STATUS.md` handoff "2026-07-10 (SIMD hardening)".

### Quick wins (< 2 h each)

- [x] **SIMD-1. `mix_simd_struct_field` — vec128 in struct confirmed** ✅ done 2026-07-10
  `ast.rs:1257` — `Type::Vec128(_)` falls through to `_ => true` in `is_copy()`;
  vec128 IS Copy and struct fields are allowed. Added `mix_simd_struct_field.vani`
  (SIMD + STRT) and updated TEST_MATRIX.md SIMD×STRT cell.

- [x] **SIMD-2. `simd_store` return type — KEEP Vec<T>** ✅ decided 2026-07-10
  `src/lib.rs:47604` `simd_store_chained_compiles` test uses
  `simd_store(simd_store(data, 0, a), 4, b)` — chaining REQUIRES the Vec<T> return.
  Decision: Vec<T> return is load-bearing; no code change. TEST_MATRIX documented.

- [x] **SIMD-3. Add QEMU section to `04b_cross_compile_primer.md`** ✅ done 2026-07-10
  Expanded the existing minimal QEMU paragraph into a full section covering:
  SVE/RVV CPU flags (`-cpu max`, `-cpu rv64,v=true,vlen=256`), vec128 NEON note,
  what-QEMU-validates table, and ARM-6 CI snippet.

- [x] **SIMD-4. `ENM + BOX` — `Option<Box<i64>>` COMPILES** ✅ done 2026-07-10
  Tested `Option.Some(box(42))` — exits 0 on both backends. Not an xfail.
  Added `mix_enum_option_box.vani` and updated TEST_MATRIX.md ENM×BOX cell.

- [x] **SIMD-5. `CLO + VEC` fn-ptr passing** ✅ done 2026-07-10
  Added `mix_closure_ref_vec_capture.vani`: closure capturing Copy scalar derived
  from Vec, passed as a `fn(i64)->i64` parameter to another function.
  True `ref Vec` capture in closures (non-Copy borrow) remains a documented
  open gap in TEST_MATRIX.md (gap #4).

### Medium (2–8 h each)

- [x] **SIMD-6. RISC-V QEMU CI (equivalent of ARM-6)** ✅ done 2026-07-10
  Added `test-riscv64-qemu` job to `.github/workflows/ci.yml`:
  `cargo test --lib --target riscv64gc-unknown-linux-gnu` under
  `CARGO_TARGET_RISCV64GC_UNKNOWN_LINUX_GNU_RUNNER=qemu-riscv64-static`.
  Cross-linker: `riscv64-linux-gnu-gcc`. Packages: `gcc-riscv64-linux-gnu qemu-user-static`.

- [x] **SIMD-7. AArch64 lib tests via QEMU** ✅ done (CI already present)
  vanic has no native LLVM dependency — it shells out to `lli`/`llc`. The
  binary is pure Rust and cross-compiles to AArch64 with no extra steps.
  `test-aarch64-qemu` CI job (ci.yml lines 38-58) already runs
  `cargo test --lib --target aarch64-unknown-linux-gnu` under
  `CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUNNER=qemu-aarch64-static`.
  Cross-linker: `aarch64-linux-gnu-gcc`. This validates parser, type-checker,
  SSA lowerer, SIMD lowering, and both backends on emulated ARM64.
  Note: `cargo test --test edge_cases` is excluded — those tests spawn the
  vanic binary which forks `cc`/`lli` (x86-64 host binaries); that requires
  real AArch64 hardware or a full VM (tracked: ARM-3).

- [x] **SIMD-8. `ARR` bucket confirmed and pinned** ✅ done 2026-07-10
  `parser.rs` has `Type::Array { element, length }` + `ExprKind::ArrayLit`.
  Syntax: `[T; N]` type, `[e0, e1, …]` literal, `a[i]` indexing — all live.
  Added `mix_arr_indexing.vani` (ARR+SCAL) and `mix_arr_struct_elems.vani`
  (ARR+STRT). TEST_MATRIX.md ARR row filled; pin raised 87→89.

- [x] **SIMD-9. `vec256<T>` + `simd256_*` builtins** ✅ done 2026-07-10
  Added `Type::Vec256(Box<Type>)` in 8 files. 7 builtins: `simd256_splat`,
  `simd256_load`, `simd256_store`, `simd256_add`, `simd256_sub`, `simd256_mul`,
  `simd256_reduce_add`. LLVM: `<N x T>` where N = 256/bits(T), align 32.
  C: `T __attribute__((vector_size(32)))`. 2 lib tests + 3 edge-case files.
  Stretch (`vec512<T>` for AVX-512 / SVE-512 / RVV VLEN=512, same pattern
  N×2) ✅ **done as M4, v0.5.0, 2026-07-15** — see `CHANGELOG.md` /
  `RELEASE_NOTES/v0.5.0.md`. Docs updated 2026-07-21: `docs/arm_neon_status.md`
  (new §4), `docs/simd_ffi_shims.md` (Native SIMD types section).
  `tutorials/src/advanced/05_simd.md` already had a vec512 section (Layer 5)
  from the M4 release itself, so no tutorial gap there.

- [x] **SIMD-10. QEMU system-mode bare-metal integration** ✅ done 2026-07-10
  `vanic run --target=arm-none-eabi --qemu-machine=lm3s6965evb` now works.
  Added `--qemu-machine=<board>` / `--qemu-machine <board>` flag to
  `parse_run_args`; `board_to_qemu_cmd` maps (arch, board) → `(binary, args)`;
  `run_bare_metal_qemu_system` builds ELF then invokes `qemu-system-<arch>`.
  Covered: arm/thumb (semihosting), aarch64 (semihosting), riscv32/riscv64
  (bios=none). Env-var override: `QEMU_SYSTEM_<ARCH>`. 6 new unit tests in
  `src/main.rs`. Updated bare-metal error message to suggest `--qemu-machine`.

### Blocked on hardware

- [ ] **ARM-3 (existing). AArch64 benchmark run — all 11 benchmarks**
  All RESULTS.md numbers are x86-64. Run on Graviton 3 / Pi 4 / Apple Silicon.
  Add `AArch64` column. Validates NEON auto-vectorization quality on real silicon.

- [ ] **RVV-bench. RISC-V Vector benchmark run**
  Run benchmark 11 (SIMD dot product) on real RISC-V hardware with the V
  extension (SiFive X280, Milk-V Pioneer, StarFive VisionFive 2).
  Add `RISC-V (RVV)` column to RESULTS.md. This is the first cross-ISA SIMD
  comparison point.

---

---

## Vec\<f64\> builtin parity (added 2026-07-13)

These items directly unblock `vani-probability` and `vani-calculus` packages.
Root cause: all vec builtins that aggregate or reorder elements restrict to `Vec<i64>` in v1
via a hard `!matches!(element_type, Type::I64)` guard in `check_vec_reduction_builtin`
([checker.rs:23387](../src/checker.rs)) and sibling functions. The for-in-ref loop,
`xs[i]` indexing, and `set(mut ref xs, i, v)` already work on `Vec<f64>`.

Priority order matches what the packages need first.

- [x] **F64-1. Extend `sort` / `sort_by` to `Vec<f64>`** (~1 h · P1) ✅ done 2026-07-13 (commit b5c7ec5)
  checker.rs gate changed to `Type::I64 | Type::F64`; sort_by comparator becomes `fn(f64,f64)->i64`.
  C backend: parameterised sort helpers using `{ct}=c_element` for element-typed variables.
  LLVM backend: elt/ep/cmp_gt/cmp_lt locals parameterise all IR strings.
  3 lib tests added; all 25 sort tests pass.
  **Unblocks:** `median`, `quantile`, `iqr`, `spearman_r` in vani-probability.

- [x] **F64-2. Extend `vec_sum`, `vec_mean`, `vec_min`, `vec_max`, `vec_argmin`,
  `vec_argmax`, `vec_median`, `vec_kth_smallest` to `Vec<f64>`** ✅ done 2026-07-13
  checker.rs gate opened for `Type::F64`; C backend uses `double` helpers;
  LLVM backend uses `fsub`/`fcmp`/`uitofp` equivalents. f64 variants of all
  8 builtins confirmed via lib tests.

- [x] **F64-3. Extend `vec_fold`, `vec_map`, `vec_filter` to `Vec<f64>`** ✅ done 2026-07-13
  `check_vec_map_fold_builtin` opened for `Type::F64`; mapper/combiner/predicate
  types updated; both backends emit f64-typed helpers. lib tests added.

- [x] **F64-4. Extend `vec_swap` to `Vec<f64>`** ✅ done 2026-07-13
  `check_vec_swap_builtin` opened for `Type::F64`; C backend: `double tmp` swap;
  LLVM backend: parametric `double*` swap. 2 lib tests added.

- [x] **F64-5. Extend `vec_dot` to `Vec<f64>`** ✅ done 2026-07-13
  Return type `f64`. C backend: `__dot` helper with `double` accumulator;
  LLVM backend: `@intent_vec_double__dot` with `fmul`/`fadd` loop. 2 lib tests added.

---

## Implementable gaps and bugs (added 2026-07-14)

Sourced from `docs/missing_features.md` audit. All items are within our control —
no hardware, no external tokens, no grammar consultants required.

---

### Bugs (< 1 h each)

- [x] **B1. C-backend bounds-check `.len` vs `->len` on `ref Vec<T>` params** ✅ done 2026-07-14
  `while_bounds_hints` now tracks `is_ref` per vec name via `BTreeMap<String,bool>`;
  emits `xs->len` for `ref Vec<T>` params (C pointer) instead of `xs.len`.

- [x] **B2. `pub use foo::bar as baz` rename re-export** ✅ done 2026-07-14
  Implementation was already correct; added `top_level_use_of_pub_use_as_rename`
  regression test to lock the closure-#254 + closure-#245 interaction.

- [x] **B3. Anonymous fn called inline from Vec slot (`fs[0](10)`)** ✅ done 2026-07-14
  Added `ExprKind::IndirectCall { callee, args }` AST node. Parser emits it for
  non-Var callees; checker type-checks callee as `FnPtr`/`Closure` and lowers to
  existing `TypedExprKind::CallIndirect`. Updated all exhaustive `ExprKind` walkers
  (10 in checker.rs, format.rs, smt.rs).

---

### Short (2–4 h each)

- [x] **M1. `if let` / `while let`** ✅ done 2026-07-15 (commit `e210b13`)
  Parser desugars to `match expr { Opt.Some(v) then { … } _ then {} }`;
  checker handles the resulting match arms.

- [x] **M2. Or-patterns in match arms** ✅ done 2026-07-15 (commit `bb2d562`)
  Parser accepts `|`-separated patterns in a single arm; expands to synthetic
  arms sharing the same body before type-checking.

- [x] **M3. Pattern guards** ✅ done 2026-07-15 (commit `c0a1fd3`)
  Parser extends match arm with optional `if <expr>`; guarded + unguarded arms
  for the same variant merge into one switch case with if/else inside.

- [x] **M4. `vec512<T>` + `simd512_*` builtins** ✅ done 2026-07-15 (commit `316d419`)
  `Type::Vec512(Box<Type>)` + 7 builtins (splat/load/store/add/sub/mul/reduce_add).
  LLVM: `<N x T>` with align 64; C: `__attribute__((vector_size(64)))`.
  Targets AVX-512 zmm / SVE-512 / RVV VLEN=512.

- [x] **M5. `OwnedStr` payload bound in match arm returned as `OwnedStr`** ✅ done 2026-07-15 (commits `72df520`, `2102688`)
  Scrutinee Drop suppressed only on direct move-out (arm body = Var(binding));
  view-only / no-binding arms retain the Drop. Fixes double-free exit-116 crash.

- [x] **M6. Generic type inference from user-defined Apply constructors** ✅ done 2026-07-15 (commit `81ead74`)
  `unify_param_to_arg` now handles `Apply { name, [Param(T)] }` vs mangled
  `Struct("name__suffix")` / `Enum("name__suffix")` — strips prefix, un-mangles
  scalar suffix to recover T. Also adds Apply/Apply arm for pre-mono cases.

---

### Medium (4–8 h each)

- [x] **L1. Slice patterns** ✅ done 2026-07-15 (commit `27ecb84`)
  `[first, .., last]` destructuring on `Vec<T>` / `[T; N]`. Parser extension to
  recognise `[pat, .., pat]` in match position; checker binds `first` and `last`
  to the element type; both backends emit index + length checks; `..` matches
  zero or more middle elements (no binding in v1).

- [x] **L2. `#[repr(C)]` / `#[repr(packed)]`** ✅ done (pre-existing; confirmed + tests 2026-07-16)
  `ReprAttr` enum in ast.rs; parser handles `#[repr(C)]` and `#[repr(packed)]`
  at struct declaration sites; C backend emits `__attribute__((packed))` for
  packed; LLVM backend emits `<{ ... }>` packed-struct syntax. 4 lib tests pass
  (`repr_c_struct_parses_and_compiles_to_c`, `repr_packed_struct_emits_packed_attr_in_c`,
  `repr_packed_struct_emits_packed_type_in_llvm`, `repr_unknown_variant_is_rejected`).

- [x] **L3. `select!` over multiple futures** (~6 h) — DONE 2026-07-16
  Syntax: `select { await <poll_call> then <binding> { body } … }`.
  Desugars in `check_one_stmt` to `TypedStmt::While { cond: true }` with nested
  `TypedStmt::If { cond: __sel_rN != -2 }` arms (one per poll). First arm that
  returns non-(−2) executes its body and breaks. Tests: `select_single_arm_compiles`,
  `select_two_arms_compiles`, `select_lowers_to_while_true_in_c`,
  `select_wildcard_binding_compiles`, `select_non_i64_poll_is_rejected`.

- [x] **L4. Runtime integer overflow guards** ✅ done 2026-07-16
  Guards (`__builtin_add/sub/mul_overflow` in C; `llvm.sadd/ssub/smul.with.overflow` in
  LLVM) emitted for every signed `+`, `-`, `*` site via `checked: bool` in `TypedExprKind::Binary`.
  SMT elision extended in `try_elide_bounds_in_typed_expr` with monotonicity goals for
  Add/Sub and sign-consistency goals for Mul — elides guard when `requires` bounds prove safety.
  4 tests: `smt_elides_add/sub/mul_overflow_*`, `overflow_guard_retained_when_operands_unbounded`.

- [x] **L5. Closure capturing non-Copy (affine) bindings** (~6 h) — DONE 2026-07-15 (commit 76a9aea)
  FnOnce semantics: heap-malloc env; env-nulled after call; scope-exit Drop;
  moved-callee guard rejects double-call. Tests in lib.rs: `aff_closure_*`.

---

### Large (dedicated session, 6–12 h each)

- [x] **XL1. `Vec<bool>` packed type** ✅ done 2026-07-16
  No new Type variant — `Type::Vec(Box::new(Type::Bool))` throughout. C backend
  emits `uint64_t[]` bit-array (`intent_vec_bool`) with `(data[i/64] >> (i%64)) & 1`
  read and bitwise-set/clear write. LLVM backend uses `i64*` data field with
  udiv/urem/lshr/and extraction; `emit_vec_bool_helpers_llvm` for push/pop/free/clone/set_mut.
  `vec_struct_tag(Type::Bool)` = `"bool"` so struct name = `%intent_vec_bool`. 6/6 tests pass.

- [x] **XL2. `vanic test` — built-in test runner** ✅ attribute-parsing done 2026-07-16,
  harness actually wired up 2026-07-28 (see BUG-30 below — this entry's original text
  described the intended design; the `main.rs` "test" command never consumed the flag
  until BUG-30 closed the gap).
  `#[test]` attribute in ast/ir/parser/checker sets is_test flag. `vanic test file.vani`
  on a file with no top-level `main` now collects is_test fns and runs each in its own
  synthesized-`main` process (pass = exit 0, fail = assert aborts that process only).
  `resolve_combined_source()` public API in lib.rs for multi-file imports. 4 lib tests
  (attribute-parsing) + 3 new end-to-end tests (actual harness behavior) pass.

- [x] **XL3. `for await x in expr { body }` syntax** (parser sugar only)
  Desugars at parse time to `while let Option.Some(x) = expr { body }`.
  No new AST/IR node. 4 lib tests pass.

- [x] **XL4. Nested monomorphization (multi-pass)** (~10 h)
  Replaced single-pass monomorphizer with worklist-based approach that scans
  each newly-generated specialization for more generic calls and iterates until
  stable. Two-level (`wrap <- double_wrap`) and three-level (`f<-g<-h`) chains
  now compile. `nested_generic_call_pins_current_behavior` and
  `nested_generic_three_level_chain_fails` both succeed in the `Ok(_)` branch.

---

---

## Performance Engineering (added 2026-07-17)

Work done improving benchmark results vs Rust/C. **All changes maintain full safety and
correctness.**  No undefined behaviour introduced; all correctness verified by benchmark
output match and edge-case tests before commit.

### Completed

- [x] **PERF-1. `getelementptr inbounds` on all Vec/Array GEPs** ✅ done 2026-07-17
  After `@__intent_bounds_check` returns (only when `idx < len`), the subsequent GEP
  is provably in-bounds. `inbounds` lets LLVM enable aggressive alias analysis and
  vectorisation. **Effect:** sieve benchmark 15.4 ms → 12.6 ms (−18%), now fastest
  vs C/C++/Rust.

- [x] **PERF-2. pdqsort in `src/sort_runtime.c`** ✅ done 2026-07-17
  Replaced naive introsort with pdqsort: Tukey ninther pivot, 64-element branchless
  block partition, heapsort fallback (depth > 2·log₂n), insertion sort (n ≤ 24),
  pattern detection (pre-scan for already-sorted / reverse-sorted in O(n)).
  **Effect:** sort 97 ms → ~67 ms vs Rust's 37.9 ms (ipnsort).

- [x] **PERF-3. AVX-512 bitmask scan in block partition** ✅ done 2026-07-17
  Replaced two-phase scalar packing loop with `_mm512_cmpge_epi64_mask` producing a
  64-bit mask + BLSR `__builtin_ctzll` bit-walk (~32 iters vs 64 scalar).  Right-side
  scan changed from backward `r[-i]` to forward `rb[i]` so prefetcher tracks both sides.
  `__builtin_clzll` replaces loop in `ilog2_n`.  **Effect:** ~5% additional improvement.
  Commit: `55f7e3c`.

- [x] **PERF-4. Persistent pthreads pool for `parallel for`** ✅ done 2026-07-17
  `src/parallel_runtime.c` — 4 persistent workers (condvar wakeup); `intent_pool_run(fn, ctx, nth)`
  replaces per-invocation `CreateThread` / `GOMP_parallel`.  LLVM backend emits
  `@intent_pool_run` declarations instead of OpenMP calls.
  **Effect:** parallel sum 197.2 ms → 125.8 ms; now 4.5% faster than Rust `std::thread`.
  Commit: `d6022ce`.

### Open gaps (structural — not closeable by micro-optimisation)

| Benchmark | vāṇī | Competitor | Gap | Root cause |
|-----------|------|------------|-----|------------|
| Sort | ~67 ms | Rust 37.9 ms | 2× | ipnsort (Rust 1.81+) vs pdqsort; LLVM vs GCC inner-loop codegen |
| Fibonacci | 943 ms | C 486 ms | 2× | GCC restructures recursive call tree; vāṇī L4 overflow guard per `+` |
| Graph BFS vs C | 16.2 ms | C 10.9 ms | 33% | L4 guards on index arithmetic; same data structure |

**Sort**: Rust's ipnsort is a fundamentally different algorithm compiled by LLVM.
Closing the gap requires porting `sort_runtime` from GCC C to Rust/LLVM or using
the vāṇī LLVM JIT to emit the inner loop directly.  Out of scope for current sprint.

**Fibonacci**: GCC `-O3` loop-restructures the two recursive calls into a single
indirect branch; LLVM does not.  Additionally each `fib(n-1)+fib(n-2)` site emits
an `llvm.sadd.with.overflow` guard.  Eliding the guard (SMT-prove safe) is possible
but requires a chain-of-calls SMT proof, not just a single-site proof.

**Graph BFS vs C**: Same index-based data structure; gap is pure L4 overhead on 3
index-arithmetic sites per BFS step.  SMT elision would close most of it, but
proving `idx < capacity` from loop invariants is a bounded-model-checking problem
(not in current SMT pass scope).

---

---

## Implementable gaps (from missing_features.md audit, added 2026-07-17)

Features still absent or partial that are within our control — no hardware, no
external tokens, no grammar consultants required.

- [x] **G1. Generic trait bounds direct syntax** (`fn f<T: Iface>(x: T)`) ✅ done 2026-07-17
  Currently the compiler silently ignores bounds; iface methods called on `T` surface
  at instantiation but there's no syntactic bound expression.
  Add `T: Iface` parse in generic param lists; check that every instantiation site
  has a `implement Iface for ConcreteType` in scope (or fail with a clear diagnostic).
  **Impact:** makes generic APIs self-documenting and catches missing impl at call site.

- [x] **G2. `Vec<non-Copy-tuple>` end-to-end verification** ✅ done 2026-07-17
  Tuples with non-Copy elements compile since v0.1.4, but `Vec<(i64, OwnedStr)>`
  has not been verified end-to-end (push, index, drop). Add an edge-case test and
  fix any issue found.

- [x] **G3. `Atomic<T>` for non-i64 payloads** — done (2026-07-17)
  Added `f64` to `is_supported_atomic_element`; `atomic_storage_llvm` maps to
  `"double"`, `atomic_align` returns 8. `atomic_fetch_add` on `f64` is rejected
  with a clear diagnostic (hardware CAS loop not emitted; use a mutex). Bool and
  all integer widths (I8–U64) were already supported. Tests: `atomic_f64_new_load_store_work`,
  `atomic_fetch_add_rejects_f64_element`.

## Blocked (not in our control)

| Item | Blocker |
|---|---|
| macOS empirical verification | Darwin hardware needed |
| Grammar consultant pass | External native-speaker review |
| Windows IOCP async-TCP (`tcp_echo_epoll` etc.) | Readiness-vs-completion model mismatch (R8 in decisions.md) |
| Arc 7 Win64 / AArch64 CI wiring | CI runner setup |
| crates.io publish (item 1) — v0.1.2 tagged and ready | crates.io API token needed (`cargo login`) |
| ARM-3 AArch64 benchmarks | Real AArch64 hardware needed (QEMU perf numbers not meaningful) |
| RVV-bench RISC-V benchmarks | Real RISC-V hardware with V extension needed |

---

---

## Static memory reporting — `vanic mem-report` (added 2026-07-18)

Pre-run heap + stack breakdown per function, target-architecture-aware.
Full design rationale in conversation 2026-07-18.

| ID | Task | Effort | Depends on |
|----|------|--------|-----------|
| MEM-1 | Target-aware pointer sizing — thread `pointer_width` from `--target` triple through `type_size()` in `stack_depth.rs` | ~1 day | nothing |
| MEM-2 | Exact struct layout — walk `TypedStructDecl` fields with ABI alignment/padding instead of flat 32-byte estimate | ~3 days | MEM-1 (needs arch-aware field sizes) |
| MEM-3 | Heap call classifier — reuse `safety.rs` walker to report per-function heap-allocating builtins and types (no error, just report) | ~2 days | nothing |
| MEM-4 | Static heap floor — extract literal capacities from `pool_new(N)`, `Vec` literals, `OwnedStr` literals; sum per entry-point | ~3 days | MEM-3 |
| MEM-5 | `vanic mem-report` subcommand — new CLI command bundling stack (MEM-1/2) + heap (MEM-3/4) reports; text/csv/json output | ~3 days | MEM-1, MEM-2, MEM-3, MEM-4 |
| MEM-6 | `audit-pack` integration — add mem-report as 7th report section | ~1 day | MEM-5 |
| MEM-7 | Tutorial: `advanced/13a_mem_report_primer.md` | ~0.5 day | MEM-5 |
| MEM-8 | Tutorial: `advanced/13_mem_report.md` (full reference) | ~1 day | MEM-5 |
| MEM-9 | Tutorial: `advanced/13b_target_aware_sizing_primer.md` | ~0.5 day | MEM-1 |
| MEM-10 | Update `beginner/00_cli_reference.md` — add `mem-report` to command table | ~0.5 day | MEM-5 |
| MEM-11 | Update `advanced/04_embedded.md` — cross-reference `mem-report` alongside `#[no_heap]` / `#[bounded_stack]` | ~0.5 day | MEM-5 |
| MEM-12 | Update `advanced/04c_attributes_reference.md` — note `mem-report` gives actual frame estimate to compare against `#[bounded_stack(bytes=N)]` | ~0.5 day | MEM-5 |
| MEM-13 | Update `advanced/12_safety_standards.md` — add `mem-report` to ASIL-D / DO-178C certification workflow | ~0.5 day | MEM-5 |
| MEM-14 | Update `tutorials/src/SUMMARY.md` — insert three new primer files into nav sequence after `12_safety_standards.md` | ~0.5 day | MEM-7, MEM-8, MEM-9 |

**Out of scope (not statically decidable):**
- Static heap *upper* bound — requires abstract interpretation / user-supplied loop bounds
- Runtime heap tracing — needs malloc wrapper injection in C backend (~2–3 week separate effort)

---

## Kosh math-library ecosystem (added 2026-07-20)

Package-level roadmap (which new Kosh packages should exist for broad math library
coverage -- vani-complex, vani-optimize, vani-geometry, vani-signal, vani-tensor,
vani-pde, a matrix v0.2 eigen/QR/SVD extension, and an optional symbolic tier) lives in
[kosh-index/ROADMAP.md](https://github.com/enthusiasticgeek/kosh-index/blob/main/ROADMAP.md).
Most of it needs zero compiler changes: structs and `Vec<struct>` already work (confirmed
via `echo_p3d_vec_struct.vani` and friends), so Complex/Tensor packages can use them
directly, and the flat-`Vec<f64>`-plus-shape encoding vani-matrix already established
generalizes to N-D tensors without new type-system support. Arbitrary-precision
arithmetic (vani-bignum, for the symbolic tier) is likewise implementable in pure
vāṇी via digit-array carry/borrow arithmetic -- no native bignum type needed.

Two narrow compiler items surfaced during that planning pass:

| ID | Task | Effort | Depends on |
|----|------|--------|-----------|
| ~~MATH-1~~ ✅ fixed 2026-07-24 | Fixed `vanic run`'s JIT session missing `intent_vec_double__sort`. Plain `sort()` on `Vec<f64>` crashed under `vanic run` ("Symbols not found: intent_vec_double__sort") but worked correctly under `vanic build` (AOT). **Corrected root cause** (the original diagnosis above was wrong on one point): `intent_vec_i64__sort` was never actually present under the JIT either -- reproduced directly, both `Vec<i64>.sort()` and `Vec<f64>.sort()` failed identically with "Symbols not found" before this fix. `run_program_llvm`/`run_program_llvm_capture` (backing `vanic run` / `vanic test`) never linked `sort_runtime.c` at all; only `build_program_llvm` (AOT) did, by compiling it to a `.o` and linking it into the binary. Fixed by adding `sort_runtime_shared_lib()` (`src/main.rs`), which compiles `sort_runtime.c` into a host shared library once per process (`OnceLock`-cached, since `vanic test` calls the JIT path once per file in a loop) and `-load`s it into `lli`, mirroring the existing `add_libgomp_load_flags` pattern. | ~1-2 h | nothing |
| ~~MATH-2~~ ✅ fixed 2026-07-24 | Generalized `sort_by` (not plain `sort`) beyond `Vec<i64>`/`Vec<f64>` to arbitrary Copy `Vec<T>` via the existing `fn(T,T)->i64` comparator shape. Scope note: plain `sort()`/`sort_desc()` (no comparator) stay i64/f64-only by design -- there's no derivable ascending order for a struct, so widening those would need a different feature (e.g. an ordering trait), not just codegen work. `sort_by` needed no such thing: the caller already supplies the order, so `sort_with`'s IR/C emission just needed `elt`/`ep`/`epp` (LLVM) and `{ct}` (C) driven off `vec_element_value_str`/`c_element` -- both already generic and already used by `push`/`pop`/`reverse` for the exact same element types, per vani-complex's and vani-geometry's existing struct-Vec usage. Verified end-to-end: a `Vec<Point>.sort_by(cmp_by_x)` sorts correctly under `vanic run` (LLVM JIT), `vanic build` (LLVM AOT), and `--backend=c`; non-Copy element types (`Vec<OwnedStr>`) are still correctly rejected with an accurate diagnostic; fixed-size-array `sort`/`sort_by` (separate, i64-only codegen path) and plain `sort()`/`sort_desc()` are unchanged. Full `cargo test --lib` run before/after: 2551 passed / 3 failed both times (the 3 are pre-existing, unrelated Win64 FFI-struct-ABI test failures -- confirmed identical on `main` without this change). | ~1 day | nothing |
| ~~MATH-3~~ ✅ fixed 2026-07-20 | `vanic run`'s JIT reported a **failed `assert`** as a native stack overflow (Windows `STATUS_STACK_BUFFER_OVERRUN`) instead of a clean non-zero exit, once any `mut ref` Vec operation was live in a caller frame. Root cause: assert-failure lowering called `abort()`, whose SIGABRT triggered LLVM's own crash/backtrace signal handler inside `lli` -- that handler's stack walk could itself fault under the JIT. Fixed by lowering assert failure to `exit(3)` instead (both the SSA-LLVM and tree-LLVM backends), bypassing signal handling entirely and matching the exit code `vanic build`'s AOT binary already produced for the same failure. | ~2-4 h | nothing |

**Out of scope for the compiler**: everything else in the math roadmap is pure
kosh-package work using language features that already exist.

---

## Device I/O + Big-O doc audit (added 2026-07-21)

Sourced from user questions about hardware I/O (PCIe/NVMe/I2C/SPI/UART/CAN/
RS485/Ethernet), Big-O cross-function propagation, and whether `use "path";`
is still required for `[deps]`-declared packages. Findings, in order:

- [x] **DOC-1. Fix stale `big_o.rs` module comment** ✅ done 2026-07-21 (commit
  `41cca6d`). The comment claimed cross-fn analysis was "out of scope
  (future)" — false. `annotate_program` (what every `--big-o` CLI entry
  point actually calls) already walks the whole program's call graph in
  topological order and threads callee complexity into the caller,
  including across `use`-merged files. Confirmed by direct test: a fn
  calling an `O(n)` helper inside a 10-iteration loop correctly reports
  `O(n²)`. Only the unused `analyze_function`/`walk_body`/`walk_stmt` path
  (dead code per the compiler's own warnings) treats calls as O(1).

- [x] **DOC-2. Extend `v1_limitations.md` L18 device-I/O note past UART** ✅
  done 2026-07-21. Added worked `extern "C"` + C-shim examples for I2C and
  SPI (previously only UART had one) plus a PCIe/NVMe clarification: no
  native or shim-specific surface exists or is planned for either — PCIe
  config-space / NVMe go through the same `extern "C"` FFI pattern against
  an OS driver or vendor SDK (hosted), or `volatile_read`/`volatile_write`
  against a memory-mapped BAR (bare-metal), same design call as UART/I2C/SPI.
  This was a documentation gap, not a compiler gap.

- [x] **DOC-3. `use "path";` redundant for `[deps]` entries — already true,
  no compiler change** ✅ confirmed 2026-07-21, no code change needed.
  `compile_path`/`resolve_combined_source` (`src/lib.rs`) already walk
  `vani.toml`'s `[deps]` and prepend every dependency's entry source
  automatically (`manifest::load_manifest` + `resolve_uses` per dep),
  independent of whatever `use` statements the entry file has. Verified
  directly: a `[deps]`-declared package's functions were callable with
  *zero* `use` statement anywhere. This means every existing kosh package's
  `use "../vendor/<dep>/src/lib.vani";` line is already redundant — see the
  kosh-index-side cleanup tracked in `kosh-index/ROADMAP.md`. Not a
  compiler-level TODO; no change needed here.

**Correction on device I/O** (was asked as "does file I/O support raw/
unbuffered access to PCIe/NVMe/I2C/SPI/UART/CAN/RS485/Ethernet"): `file_open`
is always buffered libc `fopen`/`fwrite`/`fread` — confirmed via
`src/backend_c.rs`. TCP/UDP networking (the Ethernet/application-layer case)
is a real native builtin already (`tcp_listen`/`tcp_connect_local`/
`tcp_send_str`/`tcp_recv`/... plus non-blocking `epoll`-backed variants) —
not a gap. Everything else (I2C/SPI/UART/CAN/RS485/PCIe/NVMe) is device- or
kernel-level and stays FFI + C-shim (hosted) or `volatile_read`/
`volatile_write` MMIO (bare-metal) **by design**, per L18 above — this was
already a made decision, not an open gap, so "implement raw device I/O
support" was the wrong framing. The one genuinely open, narrowly-scoped
candidate feature surfaced by this audit:

- [x] **IO-1. Unbuffered / raw flat-file I/O mode** ✅ done 2026-07-21 —
  `file_open(path: Str, mode: Str, buffered: bool)`, third arg now
  **required** (breaking change to arity — old 2-arg calls are a compile
  error). `buffered: false` calls `setvbuf(f, NULL, _IONBF, 0)` right
  after `fopen` so writes reach the OS immediately. C backend: new
  `intent_file_open` runtime helper (gated into `emit_intent_file_io_helpers_c`
  alongside the existing `intent_file_read_line`/`intent_stdin_read_line`
  helpers). LLVM backend: inlined as `@fopen` + a conditional branch to a
  block calling a newly-`declare`d `@setvbuf` — deliberately NOT a custom
  `@intent_file_open` symbol, since that shape (call a custom `@intent_*`
  function with no `declare`/runtime linkage) is exactly what's broken for
  `@intent_file_read_line` (see BUG-1 below); inlining raw libc calls
  avoids the same trap. `_IONBF`'s value is host-only-gated
  (`host_ionbf_value()` in `backend_llvm.rs`), same limitation as
  `host_is_windows()` et al. Updated: `examples/language/english/file_io.vani`,
  3 existing `src/lib.rs` tests, 2 new `src/lib.rs` tests (arity rejection +
  LLVM `@setvbuf` emission), `docs/language_manual.md`,
  `docs/v1_limitations.md` L18, `tutorials/src/intermediate/09b_file_io_primer.md`
  + `09c_file_io.md`. Verified end-to-end (not just type-checked): a
  program that writes unbuffered and reads back *without ever calling
  `file_flush`* got the correct content, on both C and LLVM backends,
  both `vanic run` and `vanic build`.

- [x] **BUG-1. `file_read_line`/`stdin_read_line` completely broken on the
  LLVM backend** ✅ fixed 2026-07-24 (both `vanic run` and `vanic build`) —
  discovered while verifying IO-1. `backend_llvm.rs` emitted
  `call i8* @intent_file_read_line(...)` / `@intent_stdin_read_line()` but
  neither had a `declare` nor any definition reachable from the LLVM path.
  Fixed exactly as sketched: both are now defined directly as ordinary
  LLVM IR functions in the preamble (`emit_llvm`, `backend_llvm.rs`),
  built from already-declared libc externs (`malloc`/`realloc`/`free`/
  `fgetc`/new `getchar` declare) — same "inline the loop instead of a
  custom runtime symbol" approach IO-1 used for `file_open`'s `setvbuf`
  call. `intent_stdin_read_line` uses `getchar()` directly rather than
  hunting for a portable `stdin` `FILE*` global (glibc/MSVCRT disagree on
  that symbol) — avoids the problem entirely instead of solving it.
  Unconditionally emitted, matching the existing Windows
  `@snprintf`/`@dprintf` shim precedent in the same preamble.

  **Second bug found and fixed along the way**: neither SSA backend
  (`ssa_backend_llvm.rs` / `ssa_backend_c.rs`) implements these builtins
  either, and — same class of gap as `f64_to_str_fixed` (item 27.1) —
  their `Call` lowering has no error path for an unrecognized name, so it
  silently mangled `stdin_read_line()` to `@fn_stdin_read_line` /
  `fn_stdin_read_line` (assuming a user function), failing at LLVM-verify
  or link time instead of compile time. Reproduced directly: a program
  calling only `stdin_read_line()` (no `FileHandle`, so nothing else
  forced tree-backend fallback) failed with `use of undefined value
  '@fn_stdin_read_line'` even after the primary fix above. Fixed by
  adding `stmt_calls_file_line_read`/`expr_calls_file_line_read`
  (`main.rs`, mirroring `stmt_calls_f64_to_str_fixed`) and wiring into
  both `ssa_llvm_extra_reject` and `ssa_c_extra_reject`. New regression
  test `stdin_read_line_falls_back_to_tree_backends_from_ssa_dispatch`
  (`main.rs`) locks in both SSA dispatch paths, mirroring the existing
  `f64_to_str_fixed_falls_back_to_tree_backends_from_ssa_dispatch` test.

  Verified end-to-end (not just type-checked): `examples/language/english/file_io.vani`
  and a dedicated multi-line-with-realloc-growth repro pass under
  `vanic run` (LLVM JIT), `vanic build` (LLVM AOT), and `--backend=c`;
  `stdin_read_line()` alone (the SSA-path trigger) verified under all
  three the same way. Full `cargo test --lib --bin vanic`: no regressions
  (see commit for exact before/after counts).

- [x] **BUG-2. `#[wcet]` estimator doesn't recurse into struct-literal field
  expressions** ✅ fixed 2026-07-24 — discovered 2026-07-21 while backfilling
  `#[wcet]` across kosh-index packages (see `kosh-index/ROADMAP.md` MAINT-1).
  `wcet_expr` in `src/safety.rs` had explicit arms for `Binary`/`Call`/
  `Index`/etc. but `StructLit` fell into the catch-all `_ => Some(5)` — a
  flat cost regardless of how expensive the field expressions actually are.
  Reproduced exactly as described: a fn `fn f(z: Complex) -> Complex {
  return Complex { re: log(complex_abs(z)), im: complex_arg(z) }; }` got a
  real enforced `#[wcet]` budget of only 10 cycles despite calling three
  real functions (`log`, `complex_abs`, `complex_arg`) inside the literal —
  `vanic check` accepted `#[wcet(cycles=10)]` on it before this fix. Fixed
  exactly as sketched: gave `StructLit { fields, .. }` its own arm in
  `wcet_expr` that sums `wcet_expr` over every field's value expression,
  mirroring `ArrayLit`'s existing arm just above the catch-all. Verified:
  the repro fn now correctly reports 68 cycles and `vanic check` rejects
  `#[wcet(cycles=10)]` on it (accepts `#[wcet(cycles=68)]`). Full
  `cargo test --lib`: 2551 passed / 3 failed, same pre-existing unrelated
  Win64 FFI-struct-ABI failures as baseline — no regressions.

---

## Kosh publish safety-coverage gate (added 2026-07-21)

Sourced from a direct user question: "is there a sanity check to ensure
things like wcet, bounded stack for all functions before even accepting a
package in kosh-index?" — there wasn't. MAINT-1 (`kosh-index/ROADMAP.md`)
audited the 12 existing math packages by hand, but nothing stopped the next
`vanic publish` from skipping that step.

- [x] **GATE-1. `vanic audit-safety` + `vanic publish` hard gate** ✅ done
  2026-07-21 (`vani-compiler` commit `d845cc9`). New `vanic audit-safety
  <path> [--format=text|json]` reuses the existing `wcet_body`/
  `compute_stack_depths` analyses (`src/safety.rs`) **unconditionally** —
  not gated on the attribute already being declared — to determine
  per-function whether `#[bounded_stack]`/`#[wcet]` is *computable*, then
  flags any eligible-but-missing case. Coverage means "declared wherever
  computable", not blanket 100% attribute presence: a fn-pointer parameter
  makes `#[bounded_stack]` uncomputable (indirect calls' frame cost is
  unknowable), and an unbounded loop or unannotated recursion makes
  `#[wcet]` uncomputable, so both are legitimately exempt and never
  flagged. Vendored `[deps]` functions are excluded via `FileMap` path
  lookup. `vanic publish` now runs this audit before building the tarball
  and refuses to publish on any gap, with `--allow-partial-safety-coverage`
  as an explicit escape hatch.

  Package entries (`src/lib.vani`) have no `fn main()`, which the existing
  `compile_path`/`compile` unconditionally required (`validate_main` in
  `checker.rs`) — added `checker::check_library` (skips `validate_main`,
  otherwise identical to `check`) and `compile_library`/
  `compile_library_path` (`src/lib.rs`) so the audit can run directly
  against a package's entry file. These are strict supersets of the
  existing `compile`/`compile_path` (same checks, main just isn't
  required), so they're safe to use for auditing an ordinary program too.

  Running the new tool against all 12 already-published MAINT-1 packages
  validated the manual audit — and found 4 real gaps it missed:
  vani-discrete's `_disc_has_edge`/`_disc_transpose` (missing
  `#[bounded_stack]`), vani-optimize's `penalty_value` (fn-pointer params
  make it correctly `#[bounded_stack]`-exempt, but indirect calls get a
  flat 10-cycle WCET charge — NOT unbounded like bounded_stack — so it's
  still `#[wcet]`-eligible), and vani-probability's
  `markov_is_absorbing_state` (1 of 106 functions). All four fixed and
  republished (discrete 0.1.2, optimize 0.1.3, probability 0.4.5); see
  `kosh-index/ROADMAP.md` MAINT-4 for the full writeup. Documented in
  `docs/kosh_design.md` (new "Safety-coverage gate" section),
  `tutorials/src/advanced/12_safety_standards.md` (new "Safety-attribute
  coverage gate" section + CI example + compliance matrix row),
  `tutorials/src/intermediate/16_packages.md` (publish walkthrough +
  command table), and `tutorials/src/beginner/00_cli_reference.md`.

---

## Kosh package namespacing + dependency-graph resolution (added 2026-07-21)

Full design: [`docs/kosh_namespacing_design.md`](kosh_namespacing_design.md).
Sourced from a direct user question ("what happens if a kosh-index
package has the same function name as a vāṇी built-in?") that led to
hands-on testing and surfaced two real bugs: (1) Kosh packages share one
flat global function namespace with vāṇी builtins and each other — any
name collision is an unrecoverable compile error, and package authors
have no way to control other packages' names; (2) transitive
dependencies (a dependency of a dependency) were only resolved one level
deep, so a "diamond" — two packages each vendoring their own copy of a
shared dependency — silently produced missing-function errors instead
of a working shared dependency.

- [x] **NS-1 (Phase 1). Real transitive dependency graph** ✅ done
  2026-07-21. `manifest::resolve_transitive_deps` (`src/manifest.rs`)
  recursively walks `[deps]` through the full graph (not just the
  top-level manifest), deduplicating by `(name, resolved_version)` —
  not file path, since a diamond-shared package legitimately lives at
  two different vendored paths on disk. Same-name-different-version is
  a hard error in v1 (no per-edge resolution à la Cargo — not needed at
  current ecosystem scale). A `visiting`-set DFS guard prevents infinite
  recursion on a cycle (plain error for now; NS-2 upgrades this to a
  full cycle-chain diagnostic). Wired into all three compilation entry
  points (`compile_path`, `compile_library_path`,
  `resolve_combined_source` in `src/lib.rs`); `vanic vendor`'s on-disk
  vendoring behavior is untouched by design.

  Verified against a real diamond (`vani-probability` + `vani-optimize`,
  both vendoring `vani-matrix` independently): the old missing-function
  bug is gone, replaced by an accurate, previously-invisible diagnostic
  — **the two published packages have actually drifted to different
  matrix versions** (probability: 0.1.0, optimize: 0.2.0). This is a
  real bug in the published ecosystem, tracked for fix during NS-6
  (not fixed standalone — no point re-pinning versions twice across two
  migrations). Manually aligning both to the same version in a scratch
  test confirmed the diamond then resolves cleanly: single compile, no
  duplicate-definition error. Regression-swept clean against all 12 real
  kosh packages via `vanic audit-safety` plus two `vanic check` test-file
  spot checks (full SMT verification path).

- [x] **NS-2 (Phase 2). Circular dependency detection** ✅ done
  2026-07-21. `manifest::check_dependency_cycles` reuses the exact
  Tarjan SCC implementation that backs `vanic acyclicity`'s
  function-call-graph analysis (`src/acyclicity.rs::tarjan_scc`, made
  `pub(crate)` — already generic, no algorithm changes needed) against
  the package graph. Upgrades NS-1's plain "circular dependency
  detected" error into a full `pkg_a -> pkg_b -> pkg_a` cycle-chain
  diagnostic, checked before any compilation is attempted.

  Found and fixed two real pre-existing bugs along the way (neither
  introduced by NS-1 — just never triggered before, since nobody had a
  circular Kosh dependency to test with): (1) `load_manifest`'s own
  internal recursion (which loads each dep's manifest just to read its
  entry path) had zero cycle protection and crashed/hung on a genuine
  cycle — fixed with a `visiting`-set DFS-path guard in a new
  `load_manifest_impl`. (2) Because of (1), `check_dependency_cycles`
  can't be built as a wrapper around `load_manifest` — every `lib.rs`
  caller's `if let Ok(m) = load_manifest(...)` pattern silently
  *swallows* a cycle-caused error rather than surfacing it, producing a
  false "ok" instead of a diagnostic (verified directly: the first
  working attempt at this feature reported success on a deliberately
  circular 3-package fixture). Fixed by having `check_dependency_cycles`
  build its graph straight from the non-recursive `parse_toml_minimal`
  parse instead, and adding `check_cycles_before_load` to guard the
  *root* manifest's `load_manifest` call specifically, since that's the
  one call site that must be checked before the fact, not after. Full
  writeup with the exact repro and error output in
  `docs/kosh_namespacing_design.md`. Regression-swept clean against all
  12 real kosh packages; the NS-1 diamond fixture (aligned versions)
  still resolves correctly with no false cycle report.

- [x] **NS-3 (Phase 3). Automatic per-package namespacing** ✅ done
  2026-07-21 — the actual fix for the name-collision bug. Each resolved
  dependency (NS-1's flattened graph) gets wrapped in a synthetic
  `module <pkg_name> { ... }` textually, in `wrap_deps_into_combined`
  (`src/lib.rs`) — pushes the `module <name> {`/`}` wrapper directly
  into the same buffer `resolve_uses` appends into, so existing
  span-tracking stays correct with no other changes needed. Visibility
  default is force-flipped to `pub` for every item in a wrapped module
  (`mark_kosh_boundary_modules_pub`) regardless of what the source
  declares — existing packages have zero `pub` annotations anywhere, so
  respecting real module privacy defaults would make every dependency
  function invisible; this is a deliberate v1 simplification, no more
  permissive than today's flat-namespace status quo. Package names are
  validated as legal identifiers before wrapping
  (`is_valid_vani_identifier`); `vanic publish`-time validation of the
  same is still open, folded into NS-5.

  Found and fixed a real, unrelated parser gap along the way: module
  bodies had no dispatch branch for `#[attr]`-prefixed items at all
  (`#[bounded_stack(...)]`/`#[wcet(...)]` etc., ubiquitous in every real
  kosh package) — nobody had ever wrapped heavily-attributed code in a
  `module { }` block before. Fixed with one added branch in
  `parser.rs`'s module-body item dispatch, reusing the exact
  `parse_attributed_fn` top-level items already use.

  Verified directly: (1) the original motivating question — a
  dependency defining `fn abs(...)` colliding with the vāṇी builtin
  `abs` — now compiles and runs correctly, both `abs(-7)` and
  `mypkg::abs(-7)` resolve and return `7`; (2) an unqualified call to a
  dependency function now correctly fails ("unknown function"),
  confirming real isolation, not just that qualified calls happen to
  also work; (3) a 4-package diamond fixture (two packages sharing a
  third, plus the root project also calling the shared package
  directly) all produced correct results in one run, combining NS-1 +
  NS-3 correctly. **Correction to the original design doc**: it had
  `pub`/`pub(kosh)` backwards — the documented `ast.rs` semantics are
  `pub(kosh)` = visible within-kosh but NOT across the kosh boundary,
  plain `pub` = the one that crosses it. Corrected in
  `docs/kosh_namespacing_design.md`.

  **Breaking change, confirmed exactly as anticipated**: 8 of the 12
  published kosh packages (`vectorcalc`, `algebra`, `pde`, `interval`,
  `tensor`, `signal`, `optimize`, `probability` — every one with
  `[deps]`) now fail `vanic audit-safety` with "unknown function" at
  their own internal calls into their dependencies, since none use
  qualified `pkgname::item` syntax yet. Not a bug — this is Phase 6's
  job to fix. The 4 self-contained packages (`complex`, `discrete`,
  `sparse`, `geometry`) are unaffected.

- [x] **NS-4 (Phase 4). `vani.lock` becomes a real lockfile** ✅ done
  2026-07-21. `write_lockfile` now calls `resolve_transitive_deps`
  instead of walking only direct `manifest.deps` — every package
  reachable through the graph gets a `[[package]]` entry. Direct deps
  keep `path`/`version-req` plus a new `direct = true` marker;
  transitive-only deps get `direct = false` and a canonicalized
  absolute `root-path` instead (no single well-defined
  path-relative-to-root exists for a package vendored at different
  depths by different dependents). Verified against a 3-package
  transitive fixture.

- [x] **NS-5 (Phase 5). Migration UX + docs** ✅ done 2026-07-21.
  `checker.rs`'s unknown-function path now calls
  `module_suggestion_for`, which scans the signature table for a
  `<module>__<name>` mangled match and suggests `module::name` when
  found — verified against the real (Phase-3-broken) `vani-pde`
  package, every failure site got the exact correct fix suggestion.

  Found a second real bug via this same work, in `vanic add` itself
  (not the namespacing logic): `registry_add` wrote the raw registry
  package name as the `[deps]` key verbatim, and the real published
  `hello-kosh` package (`[package].name = "hello-kosh"`, verified in
  `kosh-index`) has a hyphen — meaning the default, documented
  `vanic add hello-kosh` workflow generated a `vani.toml` that failed
  to compile, with no fix short of hand-editing. Fixed with
  `sanitize_dep_key` (`manifest.rs`): non-identifier characters become
  `_`; applied to the `[deps]` key only, vendored directory and
  registry lookups keep the real name. `vanic add` now prints a note
  when it sanitizes. Verified end-to-end against the real package.

  Docs updated: `docs/kosh_design.md` (calling convention + transitive
  resolution), `docs/namespaces_design.md` (corrected the
  `pub`/`pub(kosh)` mixup from NS-3), `tutorials/src/intermediate/16_packages.md`
  (rewrote the dependency-usage section with accurate syntax + the
  `hello-kosh`/`hello_kosh` sanitization note + NS-4's new lockfile
  fields). The DOC-3 claim below needed no correction — NS-1 already
  made it accurate again; NS-3 changes call *syntax*, not whether
  `use` is needed.

- [x] **NS-6 (Phase 6). Migrate + republish the ecosystem** ✅ done
  2026-07-21 — **the full 6-phase Kosh namespacing arc is now
  complete.** All 8 affected packages (every one with `[deps]`)
  migrated to qualified `pkgname::item` call syntax and republished:
  `vectorcalc` 0.1.3, `algebra` 0.1.3, `pde` 0.1.3, `interval` 0.1.3,
  `tensor` 0.1.3, `signal` 0.1.3, `optimize` 0.1.4, `probability`
  0.4.6. `vani-signal`'s migration also had to qualify type references
  (`vani-complex` exports a `Complex` struct, not just functions) —
  verified `pkgname::TypeName` works the same way `pkgname::function`
  does before relying on it. `probability`'s vendored `matrix` upgraded
  0.1.0 → 0.2.0 (aligning with `optimize`) after confirming via `diff`
  that 0.2.0 is purely additive (nothing removed/renamed). Every
  package individually verified: `vanic audit-safety` on its lib.vani,
  every test file and example via `vanic check --no-verify`, at least
  one real `vanic run` (exit 0) before publishing.

  **Final verification**: a fresh scratch project depending on the
  real, newly-published `probability` + `optimize` (both now on
  `matrix` 0.2.0) compiles clean with zero version conflict, zero
  missing functions, zero namespace collision — the exact diamond
  scenario that started this whole arc. Full sweep of all 12 published
  kosh packages via `vanic audit-safety` confirmed every one passes.
  Full writeup in `docs/kosh_namespacing_design.md`.

**Non-goals (v1)**: multiple coexisting versions of the same package in
one graph (Cargo-style per-edge resolution); semver-range-based version
*selection* across the graph. Neither needed at current ecosystem scale
(~12-15 first-party packages, no external contributors yet).

---

## C backend bug found building vani-bignum (added 2026-07-24)

- [x] **BUG-3. C backend's `while`-loop Vec-bounds optimizer hint aborts
  on a correct, safe access pattern** ✅ fixed 2026-07-24 — discovered while building
  `vani-bignum`'s `_bn_mag_add` (base-1e9 digit-array add of two
  `Vec<i64>` magnitudes of possibly-different lengths). `while_bounds_hints`
  (`src/backend_c.rs:12521`) emits a pre-loop `abort()` guard asserting
  `upper <= vec.len` for every Vec indexed anywhere in a `while var (<|<=)
  upper` loop body (recursing into `if` blocks via `collect_vec_idx_names`),
  intended as an optimizer hint so gcc can prove per-element bounds checks
  redundant. The assumption is false whenever the loop's upper bound is
  the *max* of two Vecs' lengths and each Vec is separately guarded by its
  own `if i < len` check inside the loop (the standard "zip two
  different-length Vecs" pattern) — the shorter Vec's real length is
  legitimately less than `upper`, and the inner `if` guard already makes
  every access safe, but the hint doesn't account for that nesting and
  aborts unconditionally before the loop even runs.

  Minimal repro (fails under `--backend=c`, passes under the default LLVM
  backend):
  ```vani
  fn f(a: ref Vec<i64>, b: ref Vec<i64>) -> i64 {
      let na: i64 = len(a) as i64;
      let nb: i64 = len(b) as i64;
      let n: i64 = na;
      if nb > n { n = nb; }
      let sum: i64 = 0;
      let i: i64 = 0;
      while i < n {
          let av: i64 = 0;
          if i < na { av = a[i]; }
          let bv: i64 = 0;
          if i < nb { bv = b[i]; }
          sum = sum + av + bv;
          i = i + 1;
      }
      return sum;
  }
  ```
  Calling `f` with `a`/`b` of different lengths aborts with `loop bound out
  of vec range` under `--backend=c`. Oddly, this exact shape alone did
  *not* reproduce in isolation during triage (same code, no abort) — it
  only reproduced once embedded in `_bn_mag_add`'s full body (which also
  `push`es into a third `Vec<i64>` each iteration and has a carry/`if`
  chain after the two guarded reads); the minimal standalone repro above
  needed re-verification against the real trigger conditions before
  fixing.

  Fixed exactly per the first likely-fix shape sketched above: stopped
  collecting Vec names from inside `if` *bodies* in `collect_vec_idx_names`
  (`src/backend_c.rs`) — only the `if` *condition* is still walked, since
  that's evaluated unconditionally every iteration and safe to trust.
  Accesses inside `then`/`else` bodies are no longer assumed safe, which
  is always correct: this hint is purely an optimizer aid (gcc VRP), every
  indexed access still goes through the real per-element
  `intent_check_bounds` regardless of whether the hint fires, so being
  conservative here has no correctness downside, only a (rare, narrow)
  missed-optimization one. Verified: the standalone minimal repro above
  now passes under `--backend=c` (confirmed it reproduced pre-fix on the
  same binary), and `vani-bignum`'s full test suite + example (30!,
  large-number gcd) now passes cleanly under `--backend=c` for the first
  time — previously only LLVM (default) passed. New regression fixture
  `examples/edge_cases/edge_vec_zip_different_lengths_guarded.vani`
  (zip-two-different-length-Vecs + `push` into a third Vec, matching the
  original trigger shape) added to the standard edge-case harness
  (`tests/edge_cases.rs`), passing on both backends.

- [x] **BUG-4. `implement <Iface> for T { ... }` blocks reject
  `#[attr]`-prefixed methods entirely** ✅ fixed 2026-07-24 — discovered publishing
  `vani-bignum`: `vanic audit-safety` (and therefore `vanic publish`'s
  pre-publish gate, GATE-1) correctly identifies `BigInt_eq` (the `eq`
  method inside `implement Eq for BigInt`) as eligible for
  `#[bounded_stack(bytes = 257)]` and reports the exact number — but
  there is no syntax position to actually write that attribute. Placing
  `#[bounded_stack(...)]` directly above `fn eq(...)` inside the
  `implement` block is a parse error (`expected 'fn'`), confirmed by
  direct test. This is the same class of gap Phase 3 of the kosh
  namespacing arc already fixed for `module` blocks ("module bodies had
  no support for `#[attr]`-prefixed items") — `implement` blocks never
  got the equivalent fix. Net effect: any non-Copy struct's `Eq`/other
  interface impl with a real (non-trivial) method body can never reach
  100% attribute coverage, only `vanic publish
  --allow-partial-safety-coverage` (the escape hatch GATE-1 added for
  legitimately-uncomputable cases, not for "the checker computed a real
  number but the parser has nowhere to put it"). `vani-bignum` published
  with the escape hatch for this one function, documented in its module
  header.

  Fixed almost verbatim as sketched: `parse_impl_decl`'s method loop
  (`src/parser.rs`) now dispatches to `parse_attributed_fn` (the same
  function `parse_module_decl`'s Phase-3 fix and top-level items already
  use) when a method starts with `#`, instead of always calling
  `parse_function` directly. **Found and fixed the identical gap in a
  second, unrelated place while at it**: `methods on Type { }` blocks
  (`parse_methods_block`, inherent methods, distinct from `implement`)
  had the exact same missing-dispatch shape — same one-line-pattern fix
  applied there too. Verified: `implement Eq for BigInt`'s `eq` method
  now accepts `#[bounded_stack(bytes = 257)]` and `vanic audit-safety`
  reports full coverage with no escape hatch needed. Two new `lib.rs`
  regression tests (`implement_block_accepts_attributed_method`,
  `methods_on_block_accepts_attributed_method`) lock in both fixes,
  using the checker's own real computed budgets (99 and 72 bytes
  respectively) rather than guessed placeholders.

## Windows backend-parity bug found auditing tutorial docs (added 2026-07-24)

- [x] **BUG-5 / L25. `print`/`f64_to_str` scientific-notation exponent
  width differs between the C and LLVM backends on Windows** ✅ fixed
  2026-07-25 — discovered auditing `beginner/02_variables.md`/
  `06_strings.md`'s claims about `print`'s `f64` formatting. Also tracked
  as L25 in [`docs/v1_limitations.md`](v1_limitations.md).

  Verified directly across several magnitudes (`1000000.0`, `12345678.9`,
  `123456789.123456`, `0.0000001234`) on a Windows host: any `f64`
  value large/small enough that `%g`'s default (6-significant-digit)
  formatting switches to scientific notation printed with a **2-digit**
  exponent on the C backend (`1e+06`) and a **3-digit** exponent on the
  LLVM backend (`1e+006`) — same program, same value, different text.
  Both `print x;` (for an `f64` `x`) and the `f64_to_str` builtin hit this
  identically, since `print` routes through the same formatting path.

  **Root cause, precisely pinned down** (the original writeup's "MinGW/
  UCRT" guess was half right): MinGW's `<stdio.h>` macro-redirects
  `printf`/`vsnprintf` in *C source* to its own ANSI/C99-compliant
  `__mingw_printf`/`__mingw_vsnprintf` (statically linked from
  `libmingwex.a`) — that redirect is a preprocessor-level trick that
  only exists when compiling actual C source. Hand-emitted LLVM IR has
  no preprocessor, so `backend_llvm.rs`'s `declare i32 @printf(...)` /
  `@vsnprintf(...)` linked straight to msvcrt.dll's raw, legacy,
  non-C99 formatter instead (confirmed via `objdump -p` on the AOT
  binary: it imported `_vsnprintf` from `msvcrt.dll`, not the ANSI
  version). Reproduced identically under `vanic run` (JIT) *and*
  `vanic build` (AOT) — not JIT-specific, contrary to the earlier
  guess that AOT's `cc`-linked binary might already be fine.

  **Fix**: both LLVM backends' Windows-only preamble shims
  (`backend_llvm.rs`, `ssa_backend_llvm.rs`) now declare and route
  `printf`/`snprintf`/`dprintf` through `__mingw_vprintf`/
  `__mingw_vsnprintf` instead of the raw externs — a new `printf` shim
  (mirroring the existing `dprintf` va_list-forwarding pattern) was
  added alongside the existing `snprintf`/`dprintf` shims, whose target
  symbol was simply renamed. This alone fixed `vanic build` (AOT link
  resolves `__mingw_*` from `libmingwex.a` normally) but broke `vanic
  run` (JIT): `lli`'s symbol resolver only sees symbols exported from a
  *loaded DLL*, and `__mingw_vsnprintf`/`__mingw_vprintf` live in a
  static archive, never loaded as one. Fixed the same way MATH-1 fixed
  the equivalent JIT/AOT split for `sort_runtime.c`: two new files
  (`mingw_ansi_stdio_shim.c` + a `.def` file listing the two symbols)
  force-link and re-export them from an actual DLL, compiled once per
  process (`mingw_ansi_stdio_shared_lib()`, `main.rs`, `OnceLock`-cached)
  and `-load`ed into `lli` alongside the existing libgomp/sort-runtime
  loads. **Non-obvious part**: getting a DLL to export a symbol name
  that's merely referenced from a static archive (not defined in your
  own source) needs an explicit `.def` `EXPORTS` list — `__declspec
  (dllexport)` on a bare prototype alone does not force it (verified
  empirically: produced a DLL with an empty export table).

  **Regression found and fixed along the way**: every module either
  LLVM backend emits unconditionally `define`s the `printf`/`snprintf`/
  `dprintf` shims (not just programs that call `print`), so *any*
  Windows `lli` invocation needs the two `__mingw_*` symbols resolved,
  not just print-related ones. This broke 20 of the library's existing
  `lli_*` end-to-end tests (`backend_llvm.rs`) the moment the shim
  rename landed, since their direct `lli` invocations didn't pass a
  `-load=` for the new DLL. Added a test-local mirror
  (`add_mingw_ansi_stdio_load_flag_for_tests`, same pattern as the
  pre-existing `add_libgomp_load_flags_for_tests`) and wired it into
  all three of the module's direct-`lli` call sites.

  Regression tests: `lli_runs_print_f64_scientific_notation_matches_c_
  backend_exponent_width` (end-to-end via `lli`, not Windows-gated —
  passes trivially on Linux/macOS via the no-op stub) plus a structural
  IR-content check in each backend
  (`windows_llvm_printf_shims_use_mingw_ansi_stdio_not_raw_msvcrt` in
  `lib.rs`, `windows_ssa_llvm_printf_shims_use_mingw_ansi_stdio_not_raw_
  msvcrt` in `ssa_backend_llvm.rs`, both `#[cfg(windows)]`). Verified
  end-to-end across all three magnitudes above under `vanic run`,
  `vanic run --backend=c`, and `vanic build` — byte-identical output on
  all three paths. `cargo test --release --lib` on the affected test
  filters (`lli_`, `mingw`, `windows_llvm`, `windows_ssa_llvm`, plus
  `eprint`/`stdin`/`file_read_line`/`ssa_backend_llvm`): no regressions.

  **Workaround note** (still valid, kept in the tutorial): `f64_to_str_
  fixed(x, decimals)` never hits `%g`'s scientific-notation path at all,
  so it was never affected by this bug either way.

## Bug found building vani-ml (added 2026-07-25)

- [x] **BUG-6. LLVM backend panics on a unary-minus float literal used as
  a standalone call argument** ✅ fixed 2026-07-25 — discovered building
  `vani-ml`'s logistic-regression test (`vani-ml/tests/test_logreg.vani`),
  writing `vec(-3.0, -2.0, -1.0, -0.5, 0.5, 1.0, 2.0, 3.0)` as a
  training-data literal. `vanic check` (with or without `--no-verify`)
  accepted the file cleanly; `vanic test`/`vanic run` (default LLVM
  backend) crashed the compiler itself with an internal panic, not a
  program-level error:

  ```
  thread '<unnamed>' panicked at src\backend_llvm.rs:17093:17:
  internal error: entered unreachable code: backend: TypedExprKind not
  lowered as standalone expression: Unary { op: Neg, expr: TypedExpr {
  kind: Float(3.0), ty: F64, constant: Some(Float(3.0)), ... } }
  ```

  **Root cause**: `emit_expr`'s `TypedExprKind::Unary { op: Neg, .. }`
  match arm (`backend_llvm.rs`, then line 5221) was guarded
  `if is_int_or_bool(&expr.ty)` — there was no arm at all for the float
  case, so any standalone `Unary{Neg}` over an `f64`/`f32` operand fell
  through to the function's catch-all `unreachable!()`. This is
  consistent with — and narrower than — the existing documented guidance
  that vāṇी source in the wild never uses unary minus (this repo's own
  convention is `0.0 - x`, see `docs/reference_vani_language_notes.md`-
  style notes in downstream projects); the gap went unnoticed because the
  one existing regression test for this shape,
  `float_negation_via_unary_minus_compiles` (`lib.rs`), only calls
  `compile()` — the type-checker — never actual codegen, so it could not
  have caught a backend-only crash. Confirmed the sibling `ssa_backend_llvm.rs`
  (`InstrKind::Unary{Neg}`, around line 3113) already dispatches on
  `instr.ty.is_float()` and emits `fsub` for floats / `sub` for ints — this
  file was simply missing the equivalent split.

  **Fix**: added a second, unguarded `TypedExprKind::Unary { op: Neg, expr }`
  arm immediately after the existing int/bool-guarded one, emitting
  `fsub <ty> 0.0, %v` — mirroring `ssa_backend_llvm.rs`'s existing choice
  (plain `sub` rejects float operands outright: "integer constant must
  have integer type"). Verified: the original `vani-ml` repro now runs
  correctly on both `vanic run` and `vanic run --backend=c`; all four
  `vani-ml` test files (`test_linreg`/`test_logreg`/`test_kmeans`/
  `test_split_metrics`) still pass after rebuilding. New regression test
  `lli_runs_standalone_unary_minus_float_literal_as_call_arg`
  (`backend_llvm.rs`, actually executes the emitted IR via `lli`, unlike
  the pre-existing type-check-only test) plus a targeted
  `cargo test --release --lib -- unary neg float_arithmetic
  float_negation` spot-check (23 tests, all passing, no regressions) —
  full `cargo test --lib` not run.

## Soundness bug found scoping ref-capturing closures (added 2026-07-25)

- [x] **BUG-7. Scope-escape analyzer misses a struct-with-ref-field escape
  routed through an intermediate `let` binding — confirmed real dangling
  reference at runtime, not just a rejected-too-late diagnostic gap.**
  ✅ fixed 2026-07-25. Found while investigating whether ref-capturing closures
  (docs/missing_features.md's "Lifetime variables", path-D) could lean on
  the existing L4 Phase-3/4 scope-escape analyzer rather than needing full
  lifetime variables — see the new `docs/ref_capturing_closures_design.md`
  for the full context this was found under.

  **The documented-working case** (`docs/v1_limitations.md` L4 Phase 3):
  ```vani
  struct Holder { v: ref Vec<f64> }
  fn make() -> Holder {
      let v: Vec<f64> = vec(1.0, 2.0, 3.0);
      return Holder { v: ref v };   // correctly REJECTED
  }
  ```
  This is correctly rejected: `"ref to local binding 'v' escapes the
  function via return -- the binding drops when the function exits,
  leaving a dangling reference."` The `Stmt::Return` handler
  (`checker.rs:10692`) calls `collect_ref_sources_in_expr` (walks the
  return expression's own literal shape) and `collect_var_ref_aliases`
  (`checker.rs:13569`, added L4 Phase 4 2026-06-09 specifically to chase
  `let r = foo(ref local); return r;` through a **ref-typed** intermediate
  binding).

  **The confirmed bypass** — construct the exact same escaping struct one
  statement earlier, then return the already-built (owned-struct-typed,
  not ref-typed) local:
  ```vani
  struct Holder { v: ref Vec<f64> }
  fn make() -> Holder {
      let v: Vec<f64> = vec(1.0, 2.0, 3.0);
      let h: Holder = Holder { v: ref v };
      return h;                     // vanic check: ok  <-- should be rejected
  }
  fn main() -> i64 {
      let h: Holder = make();
      print h.v[0];                 // prints 1.2655e-311, not 1 -- confirmed
      return 0;                     //  live use-after-free via `vanic run`
  }
  ```
  `vanic check` accepts this cleanly; `vanic run` prints garbage read from
  freed memory. **Root cause**: `collect_var_ref_aliases`'s `ExprKind::Var`
  case (`checker.rs:13575`) only chases `env.lookup(name).info.ref_aliases`
  — a set populated by `compute_ref_aliases_from_let_rhs`
  (`checker.rs:13613`), which only fires for a `let` binding whose **own
  declared type is `ref T`/`mut ref T`**. `h: Holder` is an owned
  struct-typed binding whose *field* happens to hold a ref — nothing
  populates or chases `ref_aliases` for that shape, so the escape is
  invisible to both the Phase-3 inline-literal check (the return
  expression is just `Var("h")`, no struct literal to walk) and the
  Phase-4 alias-chase (h was never registered as a ref-alias source).

  **Why this matters beyond the specific repro**: any future
  ref-capturing-closure design that reuses this analyzer (a closure's
  synthesized env-struct is structurally identical to `Holder` — an owned
  struct with a `ref T` field) inherits this exact hole for free. This
  needed fixing before — or as part of — building on top of the
  scope-escape analyzer, not after.

  **Fix**: `compute_ref_aliases_from_let_rhs` (`checker.rs:13613`) gained a
  `StructLit { fields, .. }` arm — walks each field's value expression via
  the existing `collect_ref_sources_in_expr` (the same struct-literal-aware
  walker the Phase-3 inline-return check already uses) and returns the
  collected source names as this binding's `ref_aliases`. The `Stmt::Let`
  handler's guard (`checker.rs:10481`, previously `if matches!(var_ty,
  Type::Ref(_) | Type::RefMut(_))`) was relaxed to call
  `compute_ref_aliases_from_let_rhs` unconditionally — safe for every other
  binding shape, since the function only returns non-empty for the RHS
  shapes it actually recognizes. No change needed on the consuming side
  (`collect_var_ref_aliases` already reads `ref_aliases` generically,
  however it was populated) — a two-line-ish surgical fix once the right
  spot was found. **Bonus**: this also correctly rejects a two-hop variant
  (`let h = Holder{v:ref v}; let h2 = h; return h2;`) for free, via the
  Var-inheritance arm already present in `compute_ref_aliases_from_let_rhs`.
  Verified: the confirmed-bypass repro above is now rejected with the same
  diagnostic as the inline case; the pre-existing L4 Phase 3/4 test suite
  (16 tests) plus a broader struct-field/Vec-indexing spot-check (32 tests)
  all still pass; all four `vani-ml` test files still pass after rebuild.

- [x] **BUG-8. `h.v[i]` reads garbage for `struct Holder { v: ref Vec<T>
  }` under the LLVM backend — a `ref`-typed Vec field silently
  misreads its own struct's `data` pointer as the requested element.**
  ✅ fixed 2026-07-25. Found immediately after BUG-7, while writing that
  fix's positive-control test (a *legitimate*, non-escaping use of a
  `ref`-typed struct field — the exact shape `docs/v1_limitations.md`'s L4
  Phase 3 and its own regression test,
  `l4_b_phase3_user_struct_ref_field_now_accepted` (`lib.rs:9709`), claim
  is "shipped"). That test only ever calls `compile()`/`compile_to_llvm()`
  — never actually executes the emitted IR — so it could not have caught
  a codegen-only value bug, the same class of gap BUG-6 exposed.

  ```vani
  struct Holder { v: ref Vec<f64> }
  fn main() -> i64 {
      let v: Vec<f64> = vec(1.0, 2.0, 3.0);
      let h: Holder = Holder { v: ref v };
      print h.v[0];   // prints 1.2655e-311 on the LLVM backend, not 1.0
      return 0;       // --backend=c prints the correct 1 -- LLVM-only
  }
  ```

  **Root cause**, confirmed by inspecting the actual emitted `.ll` (`vanic
  emit --backend=llvm`), not just reasoning about the Rust source:
  `backend_llvm.rs`'s `Index` arm, "Vec-typed struct field" branch (under
  `TypedExprKind::FieldAccess`, the `array.ty.deref()` match), used
  `emit_lvalue_addr(array, ...)`'s result directly as the Vec struct's own
  address. That's correct when the field's own declared type IS `Vec<T>`
  (owned, embedded — `emit_lvalue_addr` already gives the Vec's address).
  But when the field's own type is `ref Vec<T>` (a pointer *field*, not an
  embedded struct), `emit_lvalue_addr` gives the address *of the pointer
  slot* — one level too shallow. The emitted IR then GEP'd into that
  address as if it directly addressed a `%intent_vec_double` struct,
  landing on the Vec struct's own first field (its internal `data`
  pointer) and loading THAT 8-byte pointer value, bit-reinterpreted as an
  `f64` — a pointer's bit pattern misread as a tiny denormalized double.
  No LLVM parse/verify error (the mismatch is semantic, not a type-name
  mismatch caught by the textual IR), no crash — just silently wrong data,
  every time, on every `ref`-typed Vec field read.

  **Fix**: when `array.ty` (the field's own declared type, *not*
  `.deref()`'d) is `Ref`/`RefMut`, insert one `load` to dereference the
  field-slot address before using it as the Vec struct's address; the
  plain-owned-field case is unchanged. Verified: both the BUG-7 repro's
  positive control and a same-scope minimal repro now read the correct
  value on the LLVM backend; new regression test
  `lli_runs_indexing_through_a_ref_typed_struct_field` (`backend_llvm.rs`,
  actually executes via `lli`) plus the same 16+32-test spot-check used
  for BUG-7 (all still passing, confirming the owned-Vec-field case is
  untouched) and all four `vani-ml` test files after rebuild.

  **Scope note**: only the specific shape found (a `ref`/`mut ref
  Vec<T>`-typed struct field, indexed) was fixed and tested. Whether the
  same "field-slot address used one level too shallow" class of bug
  affects other ref-typed-field access patterns (e.g. `Len`/`.len()` on a
  ref-typed Vec field, or a ref-typed field of a non-Vec aggregate type
  like an `Array`) was not exhaustively checked — worth a follow-up sweep
  if a similar shape is hit again.

## Ref-capturing closures v1 (added 2026-07-25)

- [x] **Ref-capturing closures can now be real `Closure` values** ✅ done
  2026-07-25 — see `docs/ref_capturing_closures_design.md` for the full
  phase breakdown (v-fix ✅, v1 ✅, v2 not started, v3 not started).
  `lambda_lift_program`'s Arc-5c Closure-value synthesis (`checker.rs`)
  used to skip entirely when a closure had any `[ref name]` capture
  ("only by-value Copy captures supported in the Closure-value path") —
  such a closure could only be called directly by name in the same
  scope, never passed as a value. Now it synthesizes a real env-struct +
  `Closure` value for ref-captures too, with `Ref<T>` env-struct fields.
  Verified: a closure capturing `ref Vec<f64>` can now be passed to a
  `Closure(...)->...`-typed higher-order function parameter and produces
  the correct result, on **both** backends.

  The C backend needed its own fix along the way: the closure-registry
  codegen rendered capture types via `c_leaf_type` (a `&'static str`
  lookup for simple types only), which produced the placeholder `/* ref
  */` for a `Ref<Vec<T>>` capture — invalid C. Fixed by switching to
  `format_declarator` (the same function real `ref T` parameters already
  render through, so the spelling — including the `const` qualifier —
  matches the hoisted function's own separately-emitted declaration
  exactly). No LLVM backend changes needed; its trampoline/constructor
  codegen was already fully generic over the capture type.

  New tests: `lli_runs_ref_capturing_closure_passed_as_higher_order_fn_arg`
  (`backend_llvm.rs`, actually executes via `lli`) and
  `ref_capturing_closure_as_value_passes_to_higher_order_fn` (`lib.rs`,
  pins both backends' generated shape via `compile_to_c`/`compile_to_llvm`).
  74-test regression spot-check (closures, L4/escape, struct-field/
  Vec-indexing) clean; all four `vani-ml` test files + its example
  re-verified passing on both backends after rebuild.

  **Update 2026-07-25 (same day)**: v2 (non-escape enforcement) is now
  done too — see the next entry.

- [x] **Ref-capturing closures v2: non-escape enforcement** ✅ done
  2026-07-25 — see `docs/ref_capturing_closures_design.md`. A
  ref-capturing `Closure` value can no longer be returned or pushed into
  an outer-scope `Vec<Closure(...)->...>` without a compile error.

  Turned out smaller than expected: `compute_ref_aliases_from_let_rhs`
  (`checker.rs`) gained a `Call`-to-magic-make-closure arm (mirroring
  BUG-7's `StructLit` arm) so a `Closure`-typed binding gets correct
  `ref_aliases` at its construction site; the *existing* Return and
  FieldAssign escape checks then reject it escaping with **zero further
  changes** (neither has a restrictive type guard — they already walk
  `ref_aliases` generically). The `push` escape check *did* need a
  one-line guard widening (previously only fired for `Vec<ref T>`
  elements; now also covers `Vec<Closure(...)->...>`). Legitimate uses
  (pass as a call argument, call directly) are unaffected — Call
  arguments were already a "consuming position" per the original L4
  Phase 3 design, so nothing needed to change there. Two new negative
  tests (`ref_capturing_closure_returned_directly_is_rejected`,
  `ref_capturing_closure_pushed_into_outer_scope_vec_is_rejected`,
  `lib.rs`), 97-test regression spot-check clean, all four `vani-ml`
  tests + example re-verified on both backends.

  **Found and filed BUG-9 along the way — fixed 2026-07-26** — see below.

- [x] **BUG-9. FieldAssign scope-escape check can be fooled when the
  target is reached through a `ref`/`mut ref` parameter, not an owned
  local.** ✅ fixed 2026-07-26. Pre-existing (predates the 2026-07-25
  session, confirmed to reproduce with a plain `ref`-field struct, no
  closures involved) — found while testing ref-capturing-closures v2's
  FieldAssign coverage, but it's a general L4 Phase 2/3 (2026-06-08) gap,
  not specific to closures.

  ```vani
  struct Holder { v: ref Vec<f64> }
  fn fill(h: mut ref Holder) -> i64 {
      let v: Vec<f64> = vec(1.0, 2.0, 3.0);
      h.v = ref v;   // was: vanic check ok -- now correctly rejected
      return 0;
  }
  ```
  `h`'s `Holder` lives in the *caller's* frame; `fill`'s local `v` drops
  at return, leaving `h.v` dangling in the caller. Was not caught.

  **Root cause**: the FieldAssign check compared `env.lookup_depth
  (obj_name)` (the object's lexical depth *within the current function*)
  against the ref source's depth. This conflated a `ref`/`mut ref`
  **parameter**'s depth (which says nothing about the actual, longer,
  caller-side lifetime of what it points to) with an **owned local**
  binding's depth (which correctly bounds the object's lifetime to the
  current function). Parameters and top-level function-body locals
  appear to share the same depth number, so a same-depth local wasn't
  flagged as "deeper" even though it's categorically shorter-lived.

  **Fix**: took option (a) from the original filing — when the
  assignment target is reached through a `mut ref` (`through_mut_ref`,
  already computed by the existing type-check above this point), skip
  the depth comparison entirely and instead require the ref source to be
  one of the *current function's own parameters* — matching `Return`'s
  existing, simpler, already-sound rule. A ref sourced from a parameter
  is safe (its referent also lives in the caller's frame, same as the
  mut-ref target); a ref sourced from any local is rejected outright,
  regardless of depth. Verified: the repro above is now rejected with a
  clear diagnostic; assigning a ref sourced from one of `fill`'s own
  parameters through the same `mut ref` target still compiles and runs
  correctly (positive control). Two new tests in `lib.rs`
  (`bug9_fieldassign_through_mut_ref_param_with_local_source_is_rejected`,
  `..._with_param_source_still_accepted`); 112-test regression spot-check
  clean; `vani-ml` and `vani-optimize`'s full suites re-verified on both
  backends.

  **Known follow-up, not fixed here (filed as BUG-12)**: the sibling
  `push(mut ref xs, ref X)` scope-escape check has the exact same
  `lookup_depth`-based flaw when `xs` is itself reached through a `mut
  ref Vec<...>` parameter — confirmed via direct test that it currently
  accepts an equally-unsound case. Not fixed in this pass: unlike the
  FieldAssign fix (which had `function: &Function`, and therefore
  `function.params`, already in scope in `check_one_stmt`), the push
  check lives in `check_push_builtin`, called from `check_call` — neither
  currently receives `function`, so the equivalent fix needs threading a
  "does this name refer to a parameter of the current function" signal
  through a much more widely-called part of the checker, a bigger and
  riskier change than this one. See `docs/ref_capturing_closures_design.md`'s
  "BUG-9 / BUG-12" section.

- [x] **Ref-capturing closures v3: _closure variants added to
  vani-optimize** DONE 2026-07-25 -- gradient_descent_fixed_closure,
  armijo_line_search_closure, gradient_descent_backtracking_closure
  (vani-optimize/src/lib.vani, v0.1.5). Additive only -- the original
  fn-typed functions are untouched, every existing caller keeps
  compiling unchanged (confirmed: no implicit coercion exists between
  fn(...)->... and Closure(...)->..., so an in-place signature change
  would have broken every current caller, including vani-optimize's own
  tests/examples -- additive variants were chosen specifically to avoid
  that). New test vani-optimize/tests/test_closure_variants.vani
  minimizes a data-parameterized objective (target captured by ref, not
  hand-coded into a top-level function) via both variants, on both
  backends. See docs/ref_capturing_closures_design.md's v3 row.

  Found two more real bugs while validating v3 (both fixed same day):

- [x] **BUG-10. A function that merely takes a Closure(...)->...-typed
  parameter fails to compile if no closure literal anywhere in the
  program happens to construct that exact shape.** DONE 2026-07-25.
  Both backends' closure-struct-typedef emission were driven entirely by
  CLOSURE_MAKE_REGISTRY, populated only when a closure literal is
  lifted somewhere in the compiled program -- a library function's own
  signature referencing a Closure type was never scanned. Confirmed via
  minimal repro (fn apply(f: Closure(i64) -> f64, x: i64) -> f64 with
  zero closure literals anywhere) failing on both backends: LLVM
  verifier "invalid type for function argument"; cc "has no member
  named 'call'". This is exactly what broke vani-optimize's other,
  pre-existing test files the moment the v0.1.5 _closure functions were
  added to its shared lib.vani -- none of those files construct a
  matching closure literal, so v3 without this fix would have broken
  every existing vani-optimize consumer, not just been inert for them.
  Fixed: both backend_llvm.rs and backend_c.rs's closure-struct
  typedef loops now also scan every function's params/return type (and
  struct fields) for Type::Closure occurrences and union those shapes
  in -- the trampoline/constructor loop stays keyed on the real registry
  (only meaningful for an actually-constructed literal); only the struct
  type declaration needed broadening.

- [x] **BUG-11 (C backend only). A closure shape referencing a Vec<T>
  (e.g. Closure(ref Vec<f64>, i64) -> f64) could have its struct
  typedef emitted before Vec<T>'s own typedef, if nothing else in the
  program happened to trigger Vec<T>'s bundle earlier.** DONE
  2026-07-25. Found immediately while fixing BUG-10: with the missing
  typedef restored, the C backend's generated code hit "cc: unknown type
  name 'intent_vec_double'" at the closure typedef line, in files with no
  local Vec<f64> variable of their own. Root cause: the existing
  early-Vec-bundle pass only scans struct fields + enum payloads for
  needed Vec<T> element types, not function signatures generally, and
  closures were a new place Vec<T> could hide that this pass never
  anticipated. Fixed: before emitting any closure struct typedef, walk
  every closure shape (via the existing collect_vec_elements, which
  already recurses through Ref/RefMut) and eagerly emit any
  not-yet-emitted primitive Vec<T> bundle first, reusing the same
  emitted_vec_bundles tracking set the existing early pass uses. LLVM
  backend was never affected (Vec types there are plain named LLVM
  struct types with no forward-declaration ordering requirement the way
  C typedefs have).

  New regression tests: closure_typed_param_with_no_matching_literal_
  in_program_compiles, closure_shape_referencing_vec_with_no_prior_
  vec_usage_compiles_on_c (lib.rs, compile-only), lli_runs_closure_
  typed_fn_defined_separately_from_its_matching_literal (backend_llvm.rs,
  actually executes). 105-test regression spot-check clean. Full
  vani-optimize suite (4 test files) re-verified passing on both
  backends; vani-ml's 4 test files + example re-verified passing on both
  backends too.

  Not published: vani-optimize v0.1.5 changes are committed and pushed
  to its own repo, but vanic publish was not run -- stopping for an
  explicit go-ahead before touching the Kosh registry.

## BUG-12, found fixing BUG-9 (added 2026-07-26, fixed same day)

- [x] **BUG-12. `push`'s scope-escape check has the same
  `lookup_depth`-through-a-`mut ref`-parameter flaw BUG-9 had for
  FieldAssign.** ✅ fixed 2026-07-26. `push(mut ref xs, ref X)`'s L4
  Phase 4 check (`checker.rs`, guards `Vec<ref T>` / `Vec<Closure(...)->
  ...>` element types) compared `env.lookup_depth(vec_name)` against the
  pushed ref's depth, the exact pattern BUG-9 fixed for FieldAssign. When
  `xs` is itself a `mut ref Vec<...>` parameter (so the real Vec lives in
  the caller's frame), the same conflation applied: a same-depth local
  ref source wasn't flagged as unsafe even though it's shorter-lived than
  the parameter's real referent.

  ```vani
  fn fill(xs: mut ref Vec<ref Vec<f64>>) -> i64 {
      let v: Vec<f64> = vec(1.0, 2.0, 3.0);
      push(xs, ref v);   // was: vanic check ok -- now correctly rejected
      return 0;
  }
  ```

  **Fix required more than mirroring BUG-9's fix directly.** The push
  check's `check_push_builtin` (called from `check_call`) doesn't have
  `function: &Function` in scope the way `check_one_stmt` (FieldAssign's
  home) does, and `check_call` is called from 8+ places throughout the
  checker — threading a new parameter through all of them would have
  been a much bigger, higher-blast-radius change. Used a thread-local
  instead, matching this file's own existing pattern for exactly this
  kind of ambient per-compile context (`CLOSURE_MAKE_REGISTRY` etc.):
  new `CURRENT_FN_PARAMS: RefCell<HashSet<String>>` in `ast.rs`, set once
  by `check_function` right before it checks that function's body (a
  plain overwrite is sufficient — functions are never checked
  concurrently within one compile), read by the push check in place of
  `function.params`.

  **A real regression was caught and fixed before landing this**: the
  first attempt reused `in_place` (derived from `push`'s first-argument
  *call-site expression* type) to decide when to apply the
  parameter-only rule. That's wrong — `push(mut ref xs, ...)` produces a
  `RefMut` call-site type even when `xs` itself is an ordinary *owned
  local* Vec (the `mut ref` there is just the in-place-push syntax,
  unrelated to whether `xs`'s own binding is a ref). Using `in_place`
  directly rejected a legitimate same-function, same-scope push that the
  pre-existing depth check already handled correctly — caught by testing
  the positive control before considering the fix done, not by a
  pre-existing regression test. Fixed by checking the *binding*'s own
  declared type (`env.lookup(vec_name)`) instead of the call-site
  expression's type: only when `xs` is *itself* declared `ref`/`mut ref`
  (a parameter, or a ref local) does the parameter-only rule apply.

  New tests: `bug12_push_through_mut_ref_vec_param_with_local_source_is_
  rejected`, `..._with_param_source_still_accepted`, and
  `bug12_regression_owned_local_vec_pushed_via_mut_ref_syntax_still_
  accepted` (the regression guard, `lib.rs`). 114-test regression
  spot-check clean (including the pre-existing `vec_of_ref_phase4_push_
  accepts_same_scope_source` and `vec_ref_push_after_source_borrow_ends_
  compiles`, which would have caught the `in_place` regression had it
  shipped); `vani-ml` and `vani-optimize`'s full suites re-verified on
  both backends. Also fixes the analogous hole in ref-capturing closures
  v2's `Vec<Closure(...)->...>` push protection, same as BUG-9 fixed it
  for v2's FieldAssign protection.

  **All bugs found this session (BUG-6 through BUG-12) are now fixed.
  None remain open.**

---

## Bugs found auditing tutorial mascot examples (added 2026-07-27)

Sourced from a doc task: adding compiler-verified `manas_mascot_error`/
`success`/`caution` examples to the tutorial book. Every new/modified
code claim was checked against `vanic.exe` (not just trusted from
prose) before being marked, which surfaced several places where the
compiler's actual behavior no longer matches — or never matched — what
the docs claim. **Update 2026-07-27 (effort pass):** traced each item
to its actual root cause in source before estimating. Two of the six
original findings (BUG-16, BUG-17 below) turned out on inspection to
be **doc bugs, not compiler bugs** — the compiler's real behavior is
correct/intentional and the tutorial text was simply wrong; downgraded
and moved out of the bug list accordingly. The remaining four are real,
now root-caused, with grounded effort estimates.

- [x] **BUG-13. `parallel for`'s purity gate rejects EVERY indexed
  write unconditionally, with no allowance for the safe "write only to
  `xs[i]` where `i` is the loop's own index" pattern — contradicting an
  existing "accepted" example already shipped in the tutorial.**
  ✅ fixed 2026-07-27. New `strip_safe_same_index_writes` pass, scoped
  ONLY to `verify_pure_body_with_reductions` (`pure fn`/`task` bodies
  untouched, confirmed by a dedicated regression test). Safe requires
  BOTH the write index being exactly the loop var AND the value not
  reading the same array at a different index — the estimate below
  under-scoped this second condition; caught it via the exact
  `xs[i] = xs[i-1] + 1` repro, which has a safe write index but a real
  cross-iteration read race. 5 new lib.rs tests; 58
  parallel_for/reduce/indirect-call tests swept clean.
  `tutorials/src/advanced/02_parallel.md` updated: `double_all` marked
  verified-working, allowed/rejected table and "why it works" bullets
  corrected. ~4–6 h · Medium. Root cause confirmed in `verify_pure_body`
  (`checker.rs:33166`, the `TypedStmt::IndexAssign` arm): it
  unconditionally pushes a diagnostic for *any* indexed write, with no
  carve-out for the same-index case. `tutorials/src/advanced/02_parallel.md`'s
  pre-existing "Mapping a Vec without a reduce" example
  (`xs[i] = xs[i] * 2;`) has apparently never actually compiled under
  this check. Fix must NOT loosen `verify_pure_body` itself — it's also
  used for `pure fn` (`checker.rs:9320`) and `task` bodies
  (`checker.rs:12654`), where no per-iteration index isolation exists
  and the blanket rejection is correct. The safe carve-out belongs
  specifically in `verify_pure_body_with_reductions`
  (`checker.rs:32752`, the wrapper already used only by `parallel for`),
  which needs the loop's index-variable name threaded in so it can
  allow `IndexAssign` when (a) the index expression is syntactically
  exactly the loop var (never `i-1`/`i+1`/a different var) and (b) the
  written-to Vec isn't also read at a *different* index in the same
  iteration in a way that could alias another thread's write. Needs new
  tests confirming `xs[i] = ...` is accepted while `xs[i-1] = ...`,
  `xs[j] = ...`, and writes gated behind an `if` on a different
  condition are still rejected.

- [x] **BUG-14. Declaring an `RwLock<T>` local crashes the LLVM backend
  instead of compiling.** ✅ fixed 2026-07-27, exactly the one-match-arm
  fix anticipated below. **Bigger finding while verifying against the
  tutorial's own worked example**: even after this fix, RwLock was
  still completely unusable for any realistic (more than one
  acquisition) program — `rwlock_read`/`rwlock_write` (acquire) were
  fully implemented, but NEITHER `ReadGuard` nor `WriteGuard` had ANY
  scope-exit release logic at all (no code path ever decremented the
  reader count or reset the writer flag), so a second acquisition on
  the same lock hung forever. Fixed too (same session, same class of
  bug — a type-dispatch match missing variants — this time in the
  `TypedStmt::Drop` handler in `backend_llvm.rs`, mirroring `Type::
  Guard`'s existing Mutex-unlock wake-one pattern but simpler: wake-all
  on release, since `rwlock_read`/`rwlock_write` already retry their
  whole CAS/load unconditionally on wake). Verified end-to-end: a
  previously-hanging read/write/read sequence now completes instantly
  with correct output, on both `vanic run` and `vanic build`. 3 new
  LLVM-backend regression tests (RwLock had C-backend-only test
  coverage before this — zero LLVM coverage, same class of gap as
  every bug below). `advanced/02c_rwlock_primer.md` rewritten with a
  fully-verified scalar-payload worked example (struct payloads hit a
  SEPARATE bug, see BUG-19 below; `task`/`join` hit ANOTHER separate
  bug, see BUG-21 below — both newly found while trying to keep the
  original doc's richer example, both deferred, not fixed).
  ~30 min – 1 h · Trivial, low risk. Root cause
  found precisely: `is_scalar()` (`backend_llvm.rs:45165`) has an
  explicit match arm listing every concurrency-primitive struct type
  that gets the uniform "single alloca" Let-codegen path —
  `Type::Channel | Type::Mutex | Type::Guard | Type::Condvar |
  Type::Barrier | ...` (line 45180) — and `Type::RwLock` /
  `Type::ReadGuard` / `Type::WriteGuard` are simply missing from that
  list, so they fall through to the `unreachable!()` at line 2591.
  `llvm_type()` (line 45405-45407) already maps all three to real named
  structs (`%intent_rwlock_i64` etc.) and the `rwlock_new`/`rwlock_read`/
  `rwlock_write` builtins already emit working codegen elsewhere
  (line 6937+) — this looks like a straightforward three-variant
  omission from one match arm, not a deeper design gap. Fix: add
  `Type::RwLock(_) | Type::ReadGuard(_) | Type::WriteGuard(_)` to the
  `is_scalar()` match arm at line 45180; verify against
  `advanced/02c_rwlock_primer.md`'s own worked example plus new lib
  tests; sweep for the same three variants in any other
  `is_scalar`-shaped exhaustive match elsewhere in the LLVM backend
  (e.g. drop/clone emission) in case the omission repeats there too.

- [x] **BUG-15. A blanket `implement<T> Iface for Wrap<T>` crashes the
  LLVM backend even completely on its own, with no concrete-impl
  conflict involved at all** (broader than first reported — reproduces
  with *just* the blanket impl, no override). ✅ fixed 2026-07-27, via
  fix (b) from the two options below — surgical, low-risk: skip
  hoisting methods from any impl with non-empty `type_params` (the
  template) in `hoist_impls_into_functions`'s per-impl loop, letting
  only the already-monomorphized concrete expansions through. Verified
  both the lone-blanket-impl case and the blanket+concrete-override
  case (concrete correctly wins, confirmed by output, not just
  compilation). 2 new LLVM-backend regression tests (existing
  blanket-impl tests were C-backend-only, same recurring gap). Also
  fixed `intermediate/04d_default_methods_primer.md`'s "Conflict rule"
  claim exactly as anticipated below — rewrote as "Overlap rule"
  describing the real (silent-concrete-wins, no diagnostic) behavior,
  with a verified example. Spot-checked 30 blanket/implement/interface/
  generic + 23 dyn/vtable/drop tests: no regressions. ~3–6 h · Medium. Root
  cause traced: `hoist_impls_into_functions` (`checker.rs:5658`, called
  early at `checker.rs:599`) hoists every impl's methods — including
  blanket ones — into ordinary top-level functions immediately, while
  the blanket impl's `T` is still an unresolved `Type::Param("T")`.
  `expand_blanket_impls` (`checker.rs:7857`, called much later at
  `checker.rs:7832`) is what's supposed to generate the real
  concrete-substituted copies (e.g. `Wrap__i64_label`), but nothing
  removes the original hoisted-too-early generic-bodied function before
  codegen. The existing `program.functions.retain(|f|
  f.type_params.is_empty())` filter (`checker.rs:6823`, meant to drop
  unmonomorphized generic-function templates) doesn't catch it, because
  the genericness here lives on the *impl block* (`imp.type_params`),
  not on the hoisted `Function`'s own `type_params` field — so the
  filter sees it as an ordinary non-generic function and keeps it.
  Reaches codegen with `Wrap<T>` still literally `Apply { name: "Wrap",
  args: [Param("T")] }`, hence the `llvm_type` unreachable panic. Two
  viable fixes: (a) have `hoist_impls_into_functions` skip impls with
  non-empty `type_params` and only hoist `expand_blanket_impls`'s
  synthesized concrete copies (requires reordering or a second hoist
  pass after line 7832), or (b) tag hoisted functions with their
  originating impl's blanket-ness and extend the line-6823-style retain
  filter to also drop those. (a) is more correct; (b) is more surgical
  and lower-risk against the rest of the early pipeline that already
  depends on hoisting happening at line 599. This also settles what
  `intermediate/04d_default_methods_primer.md`'s doc-fix should say:
  there is no ambiguity *detection* today at all (concrete-impl
  precedence is just accidental — `expand_blanket_impls` already skips
  generating a conflicting expansion when a concrete impl exists, per
  `checker.rs:7912-7919` — the crash reproduces with or without a
  conflicting concrete impl present).

- [x] **BUG-18. `match` on a slice/array pattern with no wildcard/rest
  arm silently falls back to a synthetic default instead of being
  rejected as non-exhaustive, unlike every other scrutinee kind.**
  ✅ fixed 2026-07-27 — took two passes, not one. First pass (a literal
  "`_` required" check) was too strict: it broke the tutorial's own
  pre-existing `describe_vec` pattern (`[]`, `[x]`, `[first, ..]`),
  which the docs correctly claim is exhaustive via complete length
  coverage with no `_` at all. Added coverage tracking (unconditional
  exact-length arms + an unconditional has_rest arm together proving
  every length is covered) so that shape is accepted. That in turn
  exposed a second, previously-unreachable bug: the synthetic
  "unreachable" default body was hardcoded `Int(0)` regardless of the
  match's real result type — harmless while every no-wildcard match
  was rejected (dead code), but produces invalid LLVM IR (`phi i8*` fed
  an untyped `0`) for a `Str`-returning match the instant the
  exhaustive-without-wildcard path became reachable. Fixed by reusing
  the last arm's already-correctly-typed body as the placeholder
  instead. 3 new regression tests; 136 match_-prefixed + pre-existing
  slice-pattern tests swept clean. **Second bug found, NOT fixed,
  logged as BUG-20 below**: verifying the guarded `classify_scores`
  example for the tutorial revealed `check_match_slice` type-checks
  pattern guards but never incorporates them into the dispatch
  condition at all — a guarded slice-match arm always behaves as if
  its guard were `true`, silently returning wrong results. Deferred;
  `intermediate/02b_match_enhancements.md` updated with a caution note
  + workaround, plus the new non-exhaustive error example.
  ~1–2 h · Short, low risk. Root cause found in `check_match_slice`
  (`checker.rs:15048`): every other dispatch kind (string
  `checker.rs:14722`, float `checker.rs:14972`, int/bool
  `checker.rs:17601-17644`, enum via `seen_variants`) pushes a
  `non-exhaustive match` diagnostic when no wildcard covers the
  remainder. `check_match_slice` has no equivalent check — it just does
  `wildcard_body.unwrap_or_else(|| ...Int(0)...)` (`checker.rs:15287`)
  and silently uses that fabricated default in the generated if/else
  chain when no wildcard arm was present. Fix: after the arm loop, if
  `!wildcard_seen`, push the same-style non-exhaustive diagnostic used
  by the string/float paths instead of falling back to the synthetic
  default (mirror `diagnostic_elaborations::match_not_exhaustive`).
  **Needs a second, separate look**: an agent verifying this also
  reported that *running* an accepted-but-incomplete slice match
  (before this fix lands) hits an unrelated LLVM codegen crash rather
  than any clean error — that crash should reproduce independently and
  get its own root-cause pass; it may or may not resolve automatically
  once the exhaustiveness check makes the accepting case unreachable.

**Reclassified — doc bugs, not compiler bugs** (moved out of the bug
list; still need a tutorial-text fix, tracked here so the correction
isn't lost):

- **~~BUG-16~~ → DOC-4.** `intermediate/03b_affine_deeper_primer.md`'s
  `maybe_consume` example claims the compiler should reject
  `return xs[0];` after a conditional move, but it doesn't — and it's
  *right* not to. Reproduced directly: the move only happens in the
  `if do_it { ... return other[0]; }` branch, which always returns: the
  post-`if` `return xs[0];` is only reachable when `do_it` was false, in
  which case `xs` genuinely was never moved. `checker.rs:11150-11204`'s
  branch-merge logic is correctly flow- and termination-sensitive here
  — confirmed the *actually* unsafe variant (same shape, but the moving
  branch does NOT return) is correctly rejected. ~15–30 min: fix the
  primer's own example to something that's genuinely unsafe (e.g. drop
  the early `return` from the moving branch, matching the "real bug"
  repro used to confirm this), or clearly relabel the existing
  `maybe_consume` as "already safe, and here's why" instead of "should
  be rejected."
- **~~BUG-17~~ → DOC-5.** `beginner/02_variables.md`'s Challenge section
  comment claims `narrow * a` (`i32 * i64`) is a type error — it isn't,
  and it's not a bug: `common_integer_type` (`checker.rs:32056`)
  deliberately widens same-signedness integer operands to the larger
  width (`(true, true) => signed_type(left_bits.max(right_bits))`);
  mismatched-signedness combinations are correctly still rejected
  (confirmed `u64`/`i64` fails). ~10–15 min: fix the comment to
  describe an actually-rejected mismatch (e.g. keep the existing
  `bool`/`Str` cases already in that section) instead of the
  same-signedness widening case.

**All 4 real bugs (BUG-13/14/15/18) are now fixed** ✅ (2026-07-27,
same session as this estimate — the ~9-15h bundled estimate above
turned out reasonably close; DOC-4/DOC-5 confirmed fixed 2026-08-02 —
both tutorial files already carried the corrected examples from a
prior pass, this was just a stale bookkeeping note; verified directly
against the compiler). Fixing them (mostly the tutorial-verification
work that came with each) surfaced 3 MORE bugs, none fixed, logged
below.

---

## More bugs found while fixing BUG-13/14/15/18 (added 2026-07-27)

None of these three are fixed. All were found incidentally while
verifying the four fixes above against their tutorial worked examples
— not through a dedicated audit of these specific features.

- [x] **BUG-19. `RwLock<T>`/`Mutex<T>` LLVM backend codegen is
  hardcoded for `i64` payloads only — any struct or enum `T` crashes,
  contrary to both the type system and the docs' "T can be any type:
  i64, bool, a struct, an enum, Vec<T>" claim.** ✅ fixed 2026-07-27.
  Added parametric struct-name helpers (`llvm_rwlock_struct`,
  `llvm_mutex_struct`, `llvm_guard_struct`, `llvm_read_guard_struct`,
  `llvm_write_guard_struct` — same `element_tag`-based naming
  `llvm_channel_struct` already used for `Channel<T,N>`), added the 5
  missing arms to `llvm_type_string`, replaced the preamble's
  unconditional single-i64 struct emission with a per-distinct-T scan
  (reusing `backend_c::collect_mutex_specs`/`collect_rwlock_specs`,
  mirroring the existing Channel<T,N> scan), and threaded the real
  element type through every RwLock/Mutex/Guard builtin's codegen
  (~10 call sites) plus the BUG-14 guard-release Drop logic. Verified
  end-to-end: struct, enum, `Vec<T>`, and plain `i64` payloads all
  compile and run correctly on the LLVM backend (`vanic run`,
  default), including the full acquire/read/release/write/release
  cycle. 4 new regression tests. Spot-checked 53
  mutex/guard/rwlock/channel tests + full lib suite (2596 passed, same
  3 pre-existing unrelated failures): no regressions.
  `advanced/02c_rwlock_primer.md` updated: "T can be any type" is now
  actually true on the default backend.
  **Found and fixed along the way, logged as BUG-22 below**: the SAME
  class of bug independently existed in the C backend too (missing
  parametric arms in `c_type_name`), affecting even the plain `i64`
  case — fixed. A separate C-backend-only struct-definition-ordering
  bug (struct/enum payloads specifically) was found, a fix attempted
  and found to regress the working i64 case, and reverted — logged as
  open in BUG-22, not fixed. **Stale as of the next day**: BUG-22's
  own entry below records this struct/enum-ordering gap as fully
  fixed 2026-07-28, one day after this note was written — re-verified
  directly 2026-08-02 (`RwLock<Point>`/`Mutex<Point>` struct payloads
  both compile and run correctly on `--backend=c`, matching LLVM); no
  further action needed, this note was just never updated.

- [x] **BUG-20. `check_match_slice` type-checks pattern guards on
  slice/array match arms but never incorporates them into the
  generated dispatch condition at all — a guarded slice-match arm
  always behaves as if its guard were `true`.** ✅ fixed 2026-07-27.
  Type-checks the guard in the same per-arm scope as the head/tail
  bindings, then combines it into the dispatch condition as `if
  length_cond { <bindings>; guard } else { false }` — deliberately
  NOT a plain `BinaryOp::And` (which lowers to an eager, non-short-
  circuiting LLVM `and`; the guard's bindings index the scrutinee at
  offsets only valid once the length check has already passed, so a
  plain AND would read out of bounds when length doesn't match).
  Verified directly with `[a, b] if a == b` against empty/one-element
  inputs: no crash, correct fall-through. New example
  (`examples/language/english/slice_pattern_guards.vani`) + end-to-end
  integration test asserting exact stdout on both backends (this is a
  "compiles fine, wrong answer" bug class — a compile-only test
  wouldn't have caught the original bug). 140 match_-prefixed tests +
  a 27-test slice/guard sweep: no regressions.
  `intermediate/02b_match_enhancements.md` updated: the caution note +
  workaround replaced with the real guarded example, now verified
  working.

- [x] **BUG-21. `Task`/`task`/`join` tutorial content described a
  `Task<R>` generic-return-value model that didn't match v1's actual
  implementation.** ✅ fully fixed 2026-07-28 — Path B implemented
  (real `Task<R>` with cross-thread return-value capture), not just
  a docs rewrite.
  - **New syntax**: `task <fn>(args…)` in EXPRESSION position spawns
    a real OS thread (`pthread_create`/`CreateThread`) that calls a
    named function with the given (Copy-typed) argument values,
    producing a `Type::TaskR(Box<Type>)` handle — `let t: Task<i64> =
    task worker(6);`. `join <name>` in expression position
    (`let r: i64 = join t;`) blocks until the thread finishes and
    yields the return value; bare `join <name>;` (statement form)
    still works too and just discards the result. The pre-existing
    payload-free block form (`task <name> { body }` / statement-only
    `join <name>;`) is completely unchanged — `Type::Task` (no
    payload) and `Type::TaskR(Box<Type>)` are separate types, and the
    two spawn forms are parsed via entirely different code paths
    (`parse_stmt` dispatches `task`/`join` at statement-start;
    the new expression forms are reachable only via
    `parse_primary_expr`), so there is no grammar ambiguity.
  - **Design decision that changed mid-implementation**: the callee
    does NOT need to be `pure fn` (an earlier draft required it, but
    that broke the concurrency chapter's own `stage_one`/`Barrier`
    example, which calls the inherently-blocking `barrier_wait`).
    Unlike the block form (whose inline body implicitly captures the
    *outer* function's bindings and so must stay pure-with-Copy-
    captures to avoid racing the caller), a call-form callee only
    ever touches its own explicit arguments — no implicit capture,
    nothing to race on from the caller's frame. The only requirement
    is that every argument's type is Copy (references are Copy, so
    `mut ref` to a shared primitive like `Barrier`/`Mutex<T>` is the
    normal way to share state with a spawned callee).
  - **LLVM backend** (`emit_task_spawn_call` in `backend_llvm.rs`):
    extends the existing `emit_task_via_pthread` ctx-malloc +
    pthread_create/CreateThread + outlined-trampoline pattern. The
    ctx struct is `{ result_ty, arg1_ty, arg2_ty, … }` — result
    FIRST. The trampoline calls the real `@fn_<mangled>` function
    (or bare `@<name>` for extern/no_mangle callees, mirroring the
    regular-call path) and stores its return value into the ctx's
    result field before returning. `join`'s codegen reads the result
    back by bitcasting the opaque ctx `i8*` directly to `result_ty*`
    — valid because a struct's first member is always at offset 0
    regardless of what (or how many) fields follow, which is what
    lets `TypedExprKind::TaskJoinExpr` get away with carrying only
    `result_ty` (not the arg types/count that shaped the rest of the
    struct at the spawn site). The existing `%intent_task_handle =
    type { i64, i8* }` handle struct is reused as-is for `Task<R>`
    (`llvm_type_string`/`is_scalar` both route it there) since the
    result lives in the heap ctx, not the handle.
  - **C backend** (`emit_task_spawn_call`/`emit_task_join_expr` in
    `backend_c.rs`): same design, adapted to the tree-C codegen
    shape. Since `emit_expr` here is a pure expr-to-C-source-string
    function (no `out: &mut String` side channel like the LLVM
    backend's `FnCtx`), the whole spawn sequence is wrapped in a GNU
    statement-expression `({ … })` — an established pattern already
    used elsewhere in this file for other multi-statement
    expressions. The outlined trampoline + ctx typedef go into the
    same `TASK_OUTLINES` module-scope thread-local the block form
    already uses. `join`'s result read is `*(result_ty*)ctx` — same
    offset-0 reasoning as the LLVM side.
  - **Checker**: `check_task_spawn_call`/`check_task_join_expr` in
    `checker.rs` mirror `check_call`'s signature-lookup/arity/
    per-arg-coercion shape. `verify_task_affine` (the same-block
    spawn/join affine-discipline pass) and `emit_current_scope_drops`
    were extended to recognize the expression forms (reached via
    `TypedStmt::Let`'s RHS, or `TypedStmt::Discard`'s for a
    discarded `let _ = join t;`) alongside the pre-existing
    statement-form nodes.
  - **Verified**: `examples/language/english/task_result.vani`
    (single spawn) and `task_result_multi.vani` (two concurrent
    spawns with a multi-arg callee, join-with-capture and
    join-without-capture) both produce correct output on both
    backends — `tests/run_end_to_end.rs`'s
    `task_result_multi_example_produces_correct_output_on_both_backends`
    runs the real binary end-to-end (not just a compile check) since
    a mis-sized ctx struct or wrong field offset would still compile
    and link, just read back garbage nondeterministically. Error
    paths spot-checked by hand: non-pure-*capture* still rejected for
    the block form, non-Copy task-spawn-call arg rejected, double-join
    rejected, joining a plain `Task` in expression position rejected
    with a clear "use the statement form" message. Full `cargo test
    --lib`: 2596 passed, same 3 pre-existing unrelated Win64 FFI-ABI
    failures, no regressions.
  - **Docs**: `advanced/03_concurrency.md`'s `worker(6)` example and
    `stage_one`/`Barrier` example both now compile and run as
    literally written (the latter also had a pre-existing, never-
    actually-tested `barrier_wait(mut ref b)` double-ref bug, fixed
    to `barrier_wait(b)` since `b` is already `mut ref Barrier`
    inside `stage_one`). `advanced/02c_rwlock_primer.md`'s caution
    note (added when only the sequential workaround existed) replaced
    with a real concurrent `task`/`join` snippet, verified compiling
    and running on both backends.

- [x] **BUG-22. C backend has its own, independent version of BUG-19's
  bug, PLUS a separate struct-definition-ordering bug — both found
  while verifying BUG-19's LLVM fix against `--backend=c`.** ✅ fully
  fixed 2026-07-28 (the `i64` case was fixed 2026-07-27; the struct/
  enum case — including as a function PARAMETER type, a third
  independent gap found while fixing this — landed the next day).
  - **Fixed (2026-07-27)**: `c_type_name` (`backend_c.rs` — "called by
    emit_prototype + emit_function for the return type and (mostly) by
    Let stmts for binding storage") was missing the same 5 arms
    (Mutex/Guard/RwLock/ReadGuard/WriteGuard) `llvm_type_string` was
    missing before BUG-19, and fell through to `c_leaf_type`'s
    hardcoded `intent_mutex_i64`/`intent_rwlock_i64` spelling — which
    doesn't match the REAL per-T bundle names
    (`intent_mutex_int64_t` etc) `emit_mutex_bundle`/`emit_rwlock_bundle`
    already generate correctly. `cc` rejected every program using
    `Mutex<i64>`/`RwLock<i64>` with "unknown type name
    'intent_rwlock_i64'; did you mean 'intent_rwlock_int64_t'?" — this
    reproduced for the PLAIN i64 case, not just struct payloads, and
    is fully independent of BUG-19 (never touched `backend_llvm.rs`).
    Fixed by adding the 5 missing arms to `c_type_name`, mirroring the
    LLVM-side fix exactly.
  - **Fixed (2026-07-28) — struct-definition-ordering**: a struct or
    enum `RwLock<T>`/`Mutex<T>` payload still failed to compile on the
    C backend — `cc` error "unknown type name 'Struct_Point'" inside
    the generated bundle typedef. Root cause: `emit_concurrency_
    runtime_helpers` was called (writing into `out`) BEFORE
    `out.push_str(&body)` — `body` holds every user struct's FULL
    field definition (a topological-sort loop earlier in `emit_c`),
    while only a forward declaration existed at the point the bundle
    was written; the bundle embeds `T` BY VALUE, which needs the
    complete type. Fixed using the previously-estimated approach: the
    same "separate buffer" pattern `emit_c` already used for splicing
    `TASK_OUTLINES` between prototypes and function bodies (comment:
    "so the task-outlining side-effect ... can be spliced between the
    prototypes and the bodies") was extended one step further —
    prototypes now build into their own `prototypes: String` (mirrors
    the pre-existing `function_bodies: String`), and
    `emit_concurrency_runtime_helpers` now writes into `body` right
    after struct/enum/vec-bundle emission finishes and BEFORE the
    prototype loop, using `format!("{}{}", prototypes, function_bodies)`
    (built first, from the newly-separated buffers) for its
    `.contains("intent_condvar")`/`.contains("intent_task_handle")`
    gating checks — so gating still sees real usage sites even though
    the bundle's own output now lands earlier than those buffers do.
    Assembly order is now: struct/enum/vec defs -> concurrency bundle
    -> prototypes -> dyn-iface vtables -> task outlines -> function
    bodies -> `main()`.
  - **Fixed (2026-07-28) — third independent gap, found verifying the
    struct-ordering fix**: even after the above, a function taking
    `mut ref RwLock<Config>` as a PARAMETER still emitted the wrong
    prototype type (`intent_rwlock_i64*` instead of
    `intent_rwlock_Struct_Config*`). Root cause: `format_declarator`
    (used specifically for parameter/pointer declarators, a code path
    entirely separate from `c_type_name`) has its OWN independent
    per-type match with the SAME 5-arm gap, duplicated in THREE
    places (bare, `Type::Ref`, `Type::RefMut`) — `c_type_name`'s fix
    never touched this function at all. Fixed by adding the same 5
    arms to all three match blocks in `format_declarator`.
  - Verified end-to-end (real `cc` invocation, not just type-checking):
    `Mutex<i64>`/`RwLock<i64>`, `RwLock<Point>`/`Mutex<Point>` (struct
    payload, standalone), a full `RwLock<Config>` read/write/release
    cycle threaded through TWO functions taking it as a `mut ref`
    parameter, and `Channel<Point, 4>` (same by-value-embedding shape,
    confirmed fixed as a side effect of the ordering fix) all compile
    AND run correctly on `--backend=c`, matching the already-working
    LLVM backend. 1 lib.rs regression test (the i64 case) + 1 new
    end-to-end integration test with a real `cc` build
    (`examples/language/english/rwlock_struct_payload.vani` +
    `rwlock_struct_payload_example_compiles_and_runs_on_both_backends`
    in `tests/run_end_to_end.rs`) — deliberately an execution test,
    not a `compile_to_c` substring check, since "doesn't compile with
    a real C compiler" is exactly the class of bug a substring check
    on generated-but-unverified C source can't catch. Full
    `cargo test --lib` swept clean (2596 passed, same 3 pre-existing
    unrelated Win64 FFI-ABI failures) both after the ordering reorder
    and after the `format_declarator` fix.
  - The default LLVM backend (BUG-19's fix) never had this limitation;
    both backends now handle arbitrary `T` for `RwLock`/`Mutex`/
    `Channel` identically. `advanced/02c_rwlock_primer.md` was already
    updated (2026-07-27) documenting the C-backend gap as a known
    limitation with a scalar-only workaround — that caution note is
    now stale and should be removed/updated to reflect this fix.

- [x] **BUG-23. C backend's `while_bounds_hints` optimizer-aid macro
  referenced a Vec `let`-declared fresh inside the very loop body it
  was scanning, producing an undeclared-variable `cc` error. Fixed
  2026-07-27.** Found while investigating why vani-algebra's
  `algebra_newton_system_fd` (real, shipped library code — a
  finite-difference Jacobian loop building a fresh perturbed copy `xp`
  of `x` on every iteration, then writing/reading `xp[j]`) failed to
  compile on `--backend=c` with `'v_xp' undeclared; did you mean
  'v_x'?`. Root cause: `while_bounds_hints` emits a pre-loop assertion
  macro (`if (upper > vec.len) abort();`) for every Vec indexed by the
  loop variable inside the body, as a GCC-VRP optimizer aid (real
  per-element bounds checks still happen regardless — this hint is
  purely advisory, never load-bearing for correctness). Its helper
  `collect_vec_idx_names` walked the body's statements to find such
  Vecs but never tracked which names the body itself introduces via
  `let` — so a pattern like `while j < n { let xp: Vec<f64> = ...;
  set(mut ref xp, j, xp[j] + h); }` collected `xp` (found via the
  `set(mut ref xp, j, ...)` call, since `j` is the loop var) and
  emitted `if (n > v_xp.len) ...` BEFORE the `while` statement even
  starts — where `v_xp` doesn't exist yet in the generated C (it's
  declared fresh inside the loop body, once per iteration). Same root
  category as BUG-3 (documented in `collect_vec_idx_names`'s own
  comment: this hint must never fire for an access that isn't
  unconditionally safe at the point the hint is emitted) but a
  different specific gap. Fixed: `collect_vec_idx_names` now tracks
  each loop body's own top-level `let`-declared names and strips them
  from the collected set before returning, regardless of which branch
  first notices the index access. Verified: the original
  `algebra_newton_system_fd` repro and vani-algebra's full test suite
  now compile and run correctly on `--backend=c` (previously blocked
  entirely); 1 new regression test
  (`run_backend_c_vec_declared_fresh_inside_while_loop_body` in
  `tests/run_end_to_end.rs`), plus a full `cargo test --release
  backend_c` sweep confirmed no regressions.

- [x] **BUG-24. LLVM backend's task-spawn ctx-size estimator
  undercounted any aggregate type wider than 8 bytes, causing a real
  heap buffer overflow. Fixed 2026-07-28.** Found while auditing the
  codebase (at user request, after BUG-21 Path B shipped) for other
  instances of the BUG-19/22 "parallel dispatch functions must
  independently be kept in sync" pattern. `backend_llvm.rs`'s private
  `type_byte_size` helper — used by both `compute_ctx_size` (the
  pre-existing block-form `task { .. }`'s capture ctx) and
  `task_spawn_call_ctx_size` (BUG-21 Path B's `Task<R>` arg/result
  ctx, which inherited the bug on day one since it reused the same
  helper) — had a blanket `_ => 8` fallback covering structs, tuples,
  arrays, payloaded enums, and closures; only a short explicit list
  (scalars, `Str`, refs, `FnPtr`, `Task`, the SIMD vector types) got a
  real size. Since Copy structs are explicitly permitted as task
  captures / `Task<R>` args+results, a struct wider than 8 bytes
  triggered a real malloc undersizing. Confirmed via generated IR, not
  just reasoning: `let t: Task<Big> = task make_big(100);` where `Big`
  has 4 `i64` fields emitted `call i8* @malloc(i64 16)` for a ctx typed
  `{ %Struct_Big, i64 }` that actually needs 40 bytes — a 24-byte heap
  overflow on the trampoline's `store %Struct_Big %call_result, ...`
  on every call. The repro still printed correct output before the
  fix (allocator slack, not correctness) — classic silent-until-it-
  isn't heap corruption. Fixed by deleting `type_byte_size` and
  routing both callers through `llvm_byte_size` instead — a function
  already in this file, already correct, already used for exactly
  this kind of sizing problem (enum-payload buffer allocation), which
  recurses into `LLVM_STRUCT_FIELDS_REGISTRY`/
  `LLVM_ENUM_VARIANT_PAYLOADS_REGISTRY` rather than guessing. The C
  backend was never affected — it sizes ctx structs with a real
  `sizeof()` on a generated typedef, which is always correct
  regardless of field count. Verified: re-ran the `Task<Big>` repro
  post-fix, confirmed `malloc(i64 40)` in the emitted IR; a second
  repro for the block-form capture path (`Wide`, 5 `i64` fields, plus
  a scalar capture) confirmed `malloc(i64 48)` (was `16` before the
  fix). New example + end-to-end execution test on both backends
  (`examples/language/english/task_struct_ctx_sizing.vani` +
  `task_struct_ctx_sizing_example_produces_correct_output_on_both_backends`
  in `tests/run_end_to_end.rs`) — deliberately an execution test since
  a too-small `malloc` still compiles and links; it corrupts memory
  silently. Full `cargo test --lib` swept clean (2597 passed, same 3
  pre-existing unrelated Win64 FFI-ABI failures).

- [x] **BUG-25. `stack_depth.rs`'s per-local byte-size estimator
  (feeds the `#[bounded_stack]`/recursion stack-overflow-safety
  verifier) flat-guessed struct/enum sizes regardless of actual
  declared fields — unsound relative to the module's own documented
  "over-estimation is safe" invariant. Fixed 2026-07-28.** Found in
  the same audit pass as BUG-24 (same root-cause family: a byte-size
  table with a fallback that quietly stops being conservative once a
  type exceeds the size the table's author had in mind).
  `type_size`'s `Struct(_) => 32` / `Enum(_) => 16` arms were flat
  constants, not derived from the type's real fields — a struct with
  more than 4 `i64`-sized fields (or an enum with a payload bigger
  than 12 bytes) was UNDERestimated. Since this estimator directly
  feeds the `#[bounded_stack(N)]` verifier's "does this function's
  worst-case call chain fit in N bytes" proof, an undercount is the
  dangerous direction: a function that genuinely overflows the stack
  at runtime could pass compile-time verification. Fixed by threading
  a new `SizeCtx` (built once per `compute_stack_depths` call from
  `TypedProgram.structs`/`.enums`, which already carry real field/
  payload types — no new registry needed, unlike BUG-24's fix) through
  `type_size`/`stmt_local_bytes`, and recursing into real field types
  for `Struct`/`Enum`, mirroring `llvm_byte_size`'s already-correct
  approach in `backend_llvm.rs`. Verified: `vanic stack-depth` on a
  function with one `Wide` local (8 `i64` fields = 64 bytes) now
  reports `local_bytes: 72` (was `32` before the fix — the flat
  guess); new unit test `stack_depth_struct_local_uses_real_field_size`
  asserts `local_bytes >= 64`. None of the pre-existing `stack_depth_*`
  tests use struct/enum locals, so none needed updating. Full `cargo
  test --lib` swept clean (2597 passed, same 3 pre-existing unrelated
  Win64 FFI-ABI failures).

- [x] **BUG-26. The "3 pre-existing unrelated Win64 FFI-ABI test
  failures" cited throughout this whole session's `cargo test --lib`
  runs (BUG-22, BUG-24, BUG-25 entries above) were never actually
  diagnosed — one turned out to be a real compiler bug, the other two
  were wrong test expectations. Fixed 2026-07-28.** User asked "how
  to fix?" after seeing the same 3 failures reported yet again; rather
  than continuing to write them off as environmental noise, diagnosed
  each on its merits.
  - **`extern_12byte_struct_rejected_on_win64` — real bug.**
    `extern_return_rejection_hint` (checker.rs)'s struct/enum arm
    built its diagnostic text as "...only all-scalar structs within
    the platform size limit pass by value — see
    extern_param_rejection_hint for details" — a literal dangling
    cross-reference to another function's *name*, not its actual
    per-platform ABI text (`extern_param_rejection_hint` itself
    builds a real "Win64 ABI: ..." / "AArch64 AAPCS64: ..." /
    "SysV x86-64: ..." string). A reader gets a compiler error
    message, not the source code — "see X for details" without X's
    actual details told them nothing, and the test's `.contains("Win64
    ABI")` assertion correctly caught the gap. Fixed by extracting the
    shared rule text into `ffi_struct_abi_rule_text()` and having both
    the param and return rejection-hint builders call it.
  - **`extern_struct_with_float_field_accepted` /
    `extern_struct_return_with_float_field_accepted` — wrong test
    expectations, not compiler bugs.** `Mixed { x: i32, y: f64 }` is
    16 bytes after alignment padding — genuinely outside Win64's
    `{1,2,4,8}` pass-by-value size set (`is_ffi_safe_struct_win64`),
    even though it's fine under SysV x86-64 and AArch64 (both accept
    scalar-only structs ≤ 16 bytes). The tests' own doc comments
    already say "LLVM's SysV calling-convention classifier" — they
    were SysV/AArch64-specific by design but were never `#[cfg]`-gated
    to match, unlike their sibling Win64-specific tests
    (`extern_12byte_struct_rejected_on_win64`,
    `extern_8byte_struct_accepted_on_win64`) which already had the
    right pattern. Fixed by gating both to
    `#[cfg(not(all(target_arch = "x86_64", target_os = "windows")))]`
    and adding Win64 counterparts
    (`extern_struct_with_float_field_rejected_on_win64` /
    `..._return_..._rejected_on_win64`) asserting the correct
    rejection — closing the coverage gap instead of just silencing it.
  - Verified: all 20 `extern_*` tests pass; full `cargo test --lib`
    is **2600 passed, 0 failed** — the first fully clean run this
    session (every prior run in this session's BUG-19 through BUG-25
    work reported these same 3 failures and moved on, assuming they
    were pre-existing/environmental without checking).
  - **Lesson**: "pre-existing, unrelated, known failures" is a claim
    that needs re-verifying occasionally, not a permanent label —
    especially when the failures are platform-specific and the dev
    environment IS that platform (this session ran natively on
    Windows the whole time, so "Win64 FFI-ABI failures" were never
    going to be a CI-only quirk; they were reachable and actionable
    the whole session).

- [x] **BUG-27. C backend: raw pointer (`*const T` / `*mut T`) struct
  FIELDS emitted an unusable placeholder comment instead of a real C
  declarator — `cc` rejected any struct with a raw-pointer field.
  Fixed 2026-07-28.** Found while writing a worked example for
  `tutorials/src/intermediate/03d_cyclic_references_primer.md`'s
  "self-deregistering observer via `unsafe`" pattern (user asked for
  a runnable example of a sentence the primer had been naming but
  never actually demonstrating in code) — the shape needs exactly a
  raw pointer stored as a struct field
  (`struct SelfDeregisteringObserver { id: i64, subject_ptr: *mut i64
  }`), which turned out to have never been exercised before. Root
  cause: same bug FAMILY as BUG-19/22/24/26 (a type gets a correct
  codegen path in one place but is missing from a PARALLEL
  type-dispatch function) — `c_element_storage` (used specifically
  for struct-field / Vec-element storage spelling, a THIRD parallel
  function alongside `c_type_name` and `format_declarator`, all three
  of which have independently needed the same class of fix this
  session) had no arms for `Type::Ptr`/`Type::PtrMut` and fell
  through to `c_leaf_type`'s placeholder-comment fallback
  (`/* *mut T */ subject_ptr;` — not valid C). `cc` rejected with
  "expected specifier-qualifier-list before 'subject_ptr'". The LLVM
  backend was never affected — it doesn't route struct fields through
  this function; the example compiled and ran correctly on LLVM on
  the first attempt. Fixed by adding the same `const {}*`/`{}*`
  declarator-form arms `c_type_name` already has (its own comment
  explains why: "raw pointer storage uses the full declarator form,
  not the leaf-comment placeholder"). Verified: the worked example
  (concurrent-thread-free — a `Subject { active_count: i64 }` an
  observer decrements via a raw pointer to that one field, inside
  `unsafe(reason = "self-deregistering observer needs raw subject
  pointer")`, requires `INTENT_TARGET_EMBEDDED=1` since raw pointer
  types are gated to embedded targets in v1) now compiles and runs
  identically on both backends. New example
  (`examples/language/english/design_patterns/behavioral/observer_
  self_deregistering.vani`) + end-to-end execution test on both
  backends
  (`observer_self_deregistering_example_produces_correct_output_on_both_backends`
  in `tests/run_end_to_end.rs`), plus the worked example itself slotted
  into the primer (replacing a sentence that named the pattern without
  ever showing it). Full `cargo test --lib`: 2600 passed, 0 failed —
  no regressions.

- [x] **BUG-28. Chained guarded match arms (3+ arms sharing the same
  dispatch tag, e.g. multiple guarded `_` wildcards, or a guarded
  enum variant repeated) dispatched incorrectly — every arm before
  the last two silently ignored its own guard. Fixed 2026-07-28.**
  Found while auditing the tutorial book for claims/patterns never
  actually demonstrated in code (user request, following the
  self-deregistering-observer finding above) — writing a verified
  example for `beginner/08a_pattern_match_primer.md`'s "range
  patterns" section (which needs a chain of guarded `_` arms) turned
  up wrong runtime output. Root cause: guarded match arms are
  represented internally as a placeholder `Block { stmts:
  [Assert(guard)], tail }` (crash if the guard is false) when first
  type-checked, on the expectation that a LATER arm sharing the same
  dispatch tag folds it into a real `if guard { tail } else { .. }`
  conditional before it's ever reached at runtime ("M3" in
  `checker.rs`). The fold only ever checked ONE arm back
  (`typed_arms.last_mut()`), so a chain of 3+ guarded arms only
  partially resolved: the last two folded together, but every EARLIER
  guarded arm stayed as an independent `typed_arms` entry sharing the
  same dispatch tag as a later (already-resolved) entry — and since
  the backend's tag-based dispatch only ever reaches the FIRST entry
  for a given tag, those earlier arms' Asserts became unreachable dead
  code. Confirmed via a minimal repro: `match n { _ if n < 10 then
  "small", _ if n < 100 then "medium", _ then "big" }` returned
  "small" for every input (5, 50, 500) before the fix — the second and
  third arms' logic never ran at all, but the program still compiled
  cleanly. Fixed by replacing the one-step merge with a new
  `fold_guard_chain` helper that recurses down the rightmost `else`
  spine of the previous same-tag arm's body to find whichever guard is
  still unresolved (however many prior merges deep), so an
  arbitrary-length chain folds correctly regardless of how many arms
  precede it. Verified: 3-arm and 5-arm wildcard-guard chains, a
  5-arm chain mixing an unguarded int-literal arm with wildcard
  guards, and a guarded-enum-variant chain (`Status.Active if secs >
  3600 then .., Status.Active if secs > 60 then .., Status.Active then
  ..`) — all now dispatch correctly on both backends. New example +
  end-to-end execution test on both backends
  (`examples/language/english/match_guard_chain.vani` +
  `match_guard_chain_example_produces_correct_output_on_both_backends`
  in `tests/run_end_to_end.rs`) — deliberately an execution test since
  the bug compiled cleanly and only produced wrong output at runtime.
  **Adjacent gaps found but NOT fixed** (out of scope for this pass,
  noted for later): `check_match_str` and `check_match_float` (string-
  and float-scrutinee matches) never type-check or wire in `arm.guard`
  at all — a guard on a string/float match arm is silently accepted by
  the parser and then completely ignored; `check_match_slice`'s
  `Pattern::Wildcard` arm also never even reads `arm.guard` (unlike
  its `Pattern::Slice` arm, which BUG-20 already fixed), so a guarded
  wildcard arm in a slice/Vec match silently drops the guard entirely
  (not even the "crash if false" placeholder — just ignored). Full
  `cargo test --lib`: 2600 passed, 0 failed.

- [x] **BUG-29. LLVM backend: a payload-less variant of an enum whose
  OTHER variant carries a `Str` payload (not `OwnedStr`) crashed —
  `lli` rejected the emitted IR. Fixed 2026-07-28.** Found in the same
  audit pass as BUG-28, writing a "two flat matches instead of one
  nested pattern" worked example (vāṇी has no nested variant-in-
  variant match patterns — see the `beginner/08a_pattern_match_
  primer.md` fixes below) that needed an enum with a `Str` payload.
  Root cause: the zero-init placeholder for a payload-less variant's
  unused payload slot (`payload_zero` in `backend_llvm.rs`) had an arm
  for `Type::OwnedStr => "null"` but not `Type::Str` — both lower to
  `i8*` at the LLVM level and both need the `null` literal, but `Str`
  fell through to the generic `_ => "0"` default, emitting `insertvalue
  %Enum_X %s0, i8* 0, 1` — an integer literal where LLVM expects a
  pointer. `lli` rejected it with "integer constant must have integer
  type". Same bug FAMILY as BUG-19/22/24/26/27 (a type's zero/default
  representation handled correctly for one variant of a type family but
  not a sibling variant — here `OwnedStr` but not `Str`, both `i8*`).
  Fixed by adding `Type::Str` alongside `Type::OwnedStr` in the same
  arm. C backend was never affected. New example + end-to-end
  execution test on both backends
  (`examples/language/english/nested_enum_str_payload.vani` +
  `nested_enum_str_payload_example_produces_correct_output_on_both_backends`
  in `tests/run_end_to_end.rs`). Full `cargo test --lib`: 2600 passed,
  0 failed.

- [x] **Beginner pattern-match primer (`08a_pattern_match_primer.md`)
  had SIX broken/nonexistent-syntax code sections, not just the one
  originally flagged. Fixed 2026-07-28.** The initial audit flagged
  only the "range patterns" section (`1..99 then ..`, hedged with
  "check the formal chapter for the exact spelling"); range patterns
  were indeed never implemented (`ast::Pattern` has no Range variant —
  confirmed via source, not just a failed compile). But verifying the
  REST of the file's code blocks by hand (rather than trusting them
  because ONE was already caught) turned up five more: **Pattern 2**
  showed Rust-style tuple-destructuring match patterns (`match (x, y)
  { (0, 0) then .. }`) — vāṇी has no tuple pattern at all, confirmed
  via the same `Pattern` enum audit. **Pattern 3** showed a bare
  binding pattern with a guard (`n if n < 0 then ..`) — vāṇी match
  patterns can't introduce a fresh "catch and bind" name outside enum-
  variant/slice patterns; the parser expects a bare identifier pattern
  to be followed by `.` (interpreting it as `EnumName.Variant`) and
  rejects anything else. **"Default with a name"** showed the same
  bare-binding shape (`other then handle_unknown(other)`) for an
  identical reason. **"Match on a structured Result"** showed a nested
  variant-in-variant pattern (`Ok(Command.Echo(s))`) — `VariantWith
  Binding` takes exactly one plain binding name, never another
  pattern; confirmed via a parse error ("expected ')' (variant payload
  binding close)"). The exhaustiveness section's intentionally-broken
  example also used `match c { .. }` as a bare STATEMENT (no `return`)
  — since `match` is expression-only in vāṇी, this hits "expected
  statement" instead of the intended "not exhaustive" diagnostic the
  prose promises, teaching the wrong lesson about why the example
  fails. All six rewritten with genuinely verified-working code (the
  real idioms: `_ if <condition on the outer scrutinee variable>` for
  guards/defaults — since only the scrutinee's own name is available,
  never a pattern-bound one; two flat matches instead of nesting; `if`/
  `else` instead of tuple patterns; chained `_ if ..` arms — now
  correctly dispatching end-to-end thanks to BUG-28 — for range-style
  dispatch). Also found and fixed in the same pass: `print` items
  can't be bare binary-comparison expressions (`print "x:", a > b;`
  is a parse error — extract to a `let` first, then print the bool);
  this was already latent in the file's own pre-existing "Quick
  example" in a SIBLING file (`beginner/06_strings.md`, see below) and
  is now avoided in both. Full `cargo test --lib` + `mdbook build`:
  clean, no regressions.

- [x] **`beginner/06_strings.md`'s "Quick example" never demonstrated
  `f64_to_str_fixed` despite the file spending a full subsection on
  its behavioral guarantees, one of which was stated backwards. Fixed
  2026-07-28.** `f64_to_str_fixed` got extensive prose (rounding
  behavior, platform-dependent NaN/Infinity spelling) but was never
  actually invoked in the file's own runnable example. Compiling the
  claims by hand also found the rounding claim was simply wrong:
  the file asserted `f64_to_str_fixed(0.125, 2)` "rounds ties away
  from zero," giving `"0.13"` — the real, verified output is
  `"0.12"` (round-to-even; 0.125 is exactly representable in `f64`,
  so this isn't a precision artifact). Also found and fixed the
  same `print`-item-comparison parse restriction BUG-28's entry
  above mentions (`print "upper > lower:", upper > lower;` doesn't
  parse) — pre-existing in this file's own example, extracted to a
  `let` — and a wrong expected-output value for that same comparison
  (claimed `"upper > lower: true"`; the real, verified value is
  `false`, since uppercase precedes lowercase in ASCII). Verified
  every line of the corrected example end-to-end on both backends.

## `vanic test` `#[test]` harness mode was documented but never wired up (found continuing the tutorial-example audit, added+fixed 2026-07-28)

- [x] **BUG-30. `vanic test`'s "`#[test]` attribute mode" — documented in
  `tutorials/src/beginner/00_cli_reference.md`, in the `is_test` field's
  own doc comment (`ast.rs`), and marked done in this file's XL2 entry
  above — never actually ran.** `is_test` has parsed onto `Function`
  since 2026-07-16 (XL2), but nothing downstream ever consumed the
  flag: the `"test"` match arm in `main.rs` unconditionally called
  `run_program_llvm_capture` on the file as-is, which requires a real
  `fn main`. A `#[test]`-only file (the CLI reference's own example —
  two `#[test]` fns, no `main`) failed with "program must define fn
  main() -> i64" instead of the documented `running 2 tests` /
  `test addition_works ... ok` / `test result: ok. 2 passed; 0 failed`
  output. Found while extending the tutorial audit from "claims left
  undemonstrated" to "mechanically compile every example," starting
  with the CLI reference. Fixed by implementing the feature for real
  in `main.rs`: `detect_harness_test_fns(path)` lexes+parses the file
  (without requiring a checked/typed program) and returns the
  `#[test]` fn names iff the file has no top-level `fn main` with zero
  params; when non-empty, `run_test_function(path, name)` writes a
  sibling temp file (same directory, so relative `use` imports still
  resolve) containing the original source plus a synthesized
  `fn main() -> i64 { return <name>(); }`, and runs it through the
  existing `run_program_llvm_capture` — each test its own process, so
  one failing `assert` doesn't abort the rest of the suite. A file
  that already defines `main` keeps using legacy mode unchanged
  (`#[test]` fns are not combined with an existing `main` — documented
  as a real restriction, not silently ignored). Type-correctness of
  each test fn (no params, `i64` return) falls out of the ordinary
  checker running on the synthesized `main` — no separate validation
  needed. `--json` mode emits one result object per test
  (`"path":"<file>::<fn>"`). Verified against the tutorial's exact
  example (output now matches verbatim) plus a deliberate-failure
  case (one test fails, the other still runs and reports). New tests:
  `intentc_test_harness_mode_runs_each_test_fn_in_isolation`,
  `intentc_test_harness_mode_reports_one_failure_without_killing_the_rest`,
  `intentc_test_legacy_mode_unaffected_by_harness_detection` in
  `tests/run_end_to_end.rs`. Corrected `00_cli_reference.md`'s prose
  to describe the real per-test-process design (not a single combined
  harness `main`, which would have the same one-abort-kills-the-suite
  problem legacy mode already has).

- [x] **`beginner/02_variables.md` overclaimed integer/float mixing
  strictness -- three false claims in one file, fixed 2026-07-28.**
  Found continuing the mechanical example-compiling audit past the
  CLI reference. The file asserted "mixing [integer] widths without
  an explicit cast is a type error" and "don't multiply an `f64` by
  an `i64` without casting" — both false: `checker.rs`'s
  `promoted_numeric_type`/`common_integer_type` deliberately auto-
  widens same-signedness integer pairs to the larger width (`i32 *
  i64` → `i64`, no cast needed) and auto-promotes any int/float
  mix to the float type (`f64 * i64` → `f64`, no cast needed) — by
  design, not a gap (this is a real, intentional, safe-direction-
  only promotion scheme; verified both backends compute the
  promoted values correctly). What genuinely requires a cast is a
  narrower case than documented: same-width mixed signedness
  (`i32 * u32`) or a signed type no wider than its unsigned
  partner (`i8 + u32`) — confirmed via `check`, real diagnostic
  "no safe implicit integer promotion for X and Y; use an explicit
  cast". The file's Challenge exercise asked the reader to "note
  the type error" on exactly a case that now compiles fine
  (`i32 * i64`); rewritten to use the genuinely-erroring same-width
  case instead. Also found the bitwise-operators table listed a
  unary `~` (bitwise NOT) that doesn't exist in the language at
  all — lexer has no `Tilde` token, so `~0` is a lex error, not a
  language feature gap this session chose to fill (would touch 11
  files: lexer/parser/checker/smt/both tree backends/both SSA
  backends/big_o/safety/format — XL-tier scope, not a quick add).
  Replaced with the verified-correct `n ^ -1` idiom (XOR against
  all-ones == two's-complement bitwise-NOT). Verified every
  corrected example end-to-end on both backends, including the
  named bitwise builtins (`i64_set_bit` etc.) the file also lists,
  which do exist and work as documented.

- [x] **`beginner/04_if_else.md` claimed integer literal unary minus
  ("`-1`") doesn't parse in v1 -- stale, fixed 2026-07-28.** Verified
  `-1`, `-x` (on a variable), and `-1` inside a larger expression all
  parse and evaluate correctly on both backends (`print -5 + 3;` →
  `-2`, correct). This matches BUG-6 (fixed 2026-07-25, unrelated
  session): that bug was specifically a *float*-literal codegen
  crash, already fixed; this tutorial's blanket "no unary minus on
  literals" claim was simply never true for integers and appears to
  be leftover from early drafting. Simplified `sign(n)`'s negative
  branch and its caller from `0 - 1` / `sign(0 - 3)` to `-1` /
  `sign(-3)` for both correctness and readability.

- [x] **BUG-31. C backend: a self-referential struct (`struct Node {
  children: Vec<Node> }` -- the shape every tree / recursive-data
  example needs) was silently never emitted, breaking every
  downstream use with a confusing "incomplete type" error.**
  Found auditing `beginner/05a_recursion_primer.md`'s tree-walk
  example (Shape 1), which used this exact pattern. Root cause,
  in `backend_c.rs`'s unified struct/Vec-bundle topological emit
  loop in `emit_c`: `struct_deps_in_ty` treated a `Vec<X>` field
  the same as a by-value `X` field -- recursing in and requiring
  X's FULL struct body before the outer struct could be emitted.
  For `Node { children: Vec<Node> }` this makes Node depend on
  Node (a false self-dependency: `Vec<X>`'s C spelling is a
  fixed-size pointer-indirected handle, `{ X* data; len; cap; }`,
  which never needs X complete -- same as `Ref`/`Box`/`Ptr`, which
  already got the correct no-propagate treatment right next to
  the wrong `Vec` arm). Worse, even after fixing that, a SECOND
  false cycle remained: the struct-emission loop's separate `vok`
  check required the Vec bundle's full *helper functions* (needs
  `sizeof(X)`, genuinely needs X complete) to already be emitted
  before the struct itself -- but the bundle's functions
  themselves wait on the struct being complete, so Node and its
  own bundle deadlocked either way. The iterate-to-fixpoint loop
  has no cycle detection -- it just silently stops making
  progress and moves on, so `struct Struct_Node { ... }`'s body
  never appeared ANYWHERE in the generated C, with no diagnostic
  pointing at why. Fixed with three changes: (1) `struct_deps_in_ty`'s
  `Type::Vec` arm now no-ops (joins `Ref`/`RefMut`/`Box`/`Ptr`/`PtrMut`)
  since Vec is always pointer-indirected in C. (2) Split
  `emit_vec_bundle` into `emit_vec_bundle_typedef` (just the
  handle typedef -- needs only a forward-declared element type)
  and `emit_vec_bundle_functions` (the `sizeof`-dependent
  helpers); `emit_c` now emits every struct-field/enum-payload Vec
  bundle's typedef eagerly, upfront, before the topo loop runs at
  all, so a struct's own `vok` check (now against a new
  `emitted_vec_typedefs` set, always satisfied) never has to wait
  on the bundle's functions -- breaking the cycle at its root. The
  functions themselves still emit later, through the ordinary topo
  loop, once their element type is complete (unchanged, correct
  requirement). (3) A third, narrower bug surfaced once the above
  two fixes let `Struct_Node` actually get emitted: within a
  single bundle, `__clear`'s per-slot drop for a self-referential
  element recurses into this SAME bundle's own `__free` (to
  release each child's nested `Vec<Node>`), but `__clear` is
  emitted earlier in `emit_vec_bundle_functions` than `__free`'s
  definition -- `cc` implicitly declared `__free` as `int(...)` at
  the call site, then rejected the real `static void` definition
  as a conflicting redeclaration. Fixed by forward-declaring
  `__free` at the top of `emit_vec_bundle_functions`, before any
  other helper. Verified end-to-end: prints "1\n2\n3\n" (root then
  its two children) for the exact tree-walk shape the tutorial
  needed. Full `cargo test --lib`: 2600/2600 clean, no regressions
  from touching the shared topo-sort logic (also applies the same
  fix to the parallel enum-payload topo loop, closing the same
  class of bug for a hypothetical `enum X { A(Vec<X>) }`, untested
  but same reasoning). New example:
  `examples/language/english/self_referential_struct_vec.vani`.
  New test: `self_referential_struct_vec_example_produces_correct_output_on_c_backend`
  in `tests/run_end_to_end.rs`.
  **Not fixed -- LLVM backend counterpart, same repro program**:
  `vanic build` succeeds and produces a native binary, but running
  it (or running via `vanic run`, or via `lli` directly on the
  emitted `.ll`) produces zero stdout/stderr and a nonzero exit
  (116 via `vanic run`'s wrapper, 127 running the built binary
  directly under Git Bash) -- crashes before the very first
  `print` in `visit` would fire, so the failure is somewhere in
  `main`'s construction of the tree or very early in `visit`'s
  entry, not in the recursion itself. The generated LLVM struct
  type (`%Struct_Node = type { i64, %intent_vec_Struct_Node }`)
  looks correct -- LLVM doesn't have C's forward-declaration
  problem for named struct types, so this is very likely a
  genuinely different root cause than BUG-31's C-backend one
  (candidate: a self-referential-struct byte-size computation used
  somewhere in `backend_llvm.rs`'s malloc/copy codegen, in the
  spirit of BUG-24's `llvm_byte_size` undercounting -- not yet
  confirmed). Tutorial and example both recommend `--backend=c`
  for this pattern until the LLVM side is root-caused; not
  attempted this session given the scope already spent on the
  C-backend half.

  **UPDATE (2026-08-05): LLVM side re-verified as already working,
  no further root-causing needed.** Re-checked while auditing
  `docs/LOCALFUZZ_HANDOFF_2026-08-05.md` section 3's "not new work"
  items (one of which was this exact repro, re-surfaced by
  localfuzz as `20260804-204024-backend-divergence-ffadfdc1f9`) --
  `vanic run` (LLVM/`lli`) now prints `1\n2\n3\n` correctly, and
  `vanic build`'s emitted `.ll` run directly through `lli` does too.
  The originally-described symptom (silent crash, zero output, exit
  116/127) no longer reproduces on current `main`. Most likely
  explanation: this was fixed as an unintended side effect of
  BUG-108/109/110's extensive `backend_llvm.rs`/`ssa_backend_llvm.rs`
  Vec-handling changes (BUG-108 in particular touched Vec
  index/bounds-check codegen broadly), not by any change targeting
  this bug specifically -- consistent with this doc's own candidate
  root-cause guess above (a Vec-adjacent byte-size/codegen path).
  `vanic build` (native binary) initially still failed, but for an
  UNRELATED reason discovered during re-verification: see BUG-112
  below (missing `-lm` on the host link line) -- once that's fixed,
  `vanic build`'s output also runs correctly. Example's header
  comment, `tutorials/src/beginner/05a_recursion_primer.md`, and
  `tests/run_end_to_end.rs`'s test for this example all updated to
  drop the `--backend=c`-only caveat and cover both backends.

- [x] **BUG-32. LLVM backend: an `eprint` string-literal item that
  never also appears in a `print` statement anywhere in the
  program is silently dropped from output -- not even an empty
  placeholder.** Found auditing `beginner/05b_print_block_primer.md`'s
  `eprint { ... }` example (which surfaced two other issues: see
  below). Repro: `eprint "message";` alone printed nothing; a
  mixed `eprint "label:", value;` dropped only the label, leaving
  just the auto-inserted separator space before the value (so
  stderr showed a leading space then the value, e.g. ` /tmp/x`
  instead of `  path = /tmp/x`). `print` was unaffected, and the C
  backend was unaffected on both. Root cause: `print`/`eprint`
  string literals share one module-level constant pool
  (`@.print_str.<idx>`, both `emit_print_items_llvm` and
  `emit_eprint_items_llvm` GEP into the same naming scheme), which
  is populated once, upfront, by `collect_print_strings` walking
  every statement in the program -- but that function's dedicated
  string-collection arm only matched `TypedStmt::Print`, never
  `TypedStmt::EPrint`, falling through to the catch-all `_ => {}`.
  So an eprint-only literal never got interned. The lookup at the
  eprint call site (`ctx.print_str_indices.get(text)`) had no
  fallback on a miss — the whole `if let Some(...)` block is
  simply skipped, so the item silently vanishes rather than
  erroring or falling back to some other emission path. Fixed by
  changing `TypedStmt::Print { items } =>` to
  `TypedStmt::Print { items } | TypedStmt::EPrint { items } =>` in
  `collect_print_strings` — both statement kinds now feed the same
  shared pool, matching how their *emission* already assumed one
  shared pool. Verified end-to-end on both backends. New example:
  `examples/language/english/eprint_string_literal.vani`. New
  test: `eprint_string_literal_example_produces_correct_output_on_both_backends`
  in `tests/run_end_to_end.rs`.

- [x] **`beginner/05b_print_block_primer.md` had three more issues
  besides the above, all fixed 2026-07-29.** (1) Claimed `eprint`
  "supports the same block form" as `print` — false: `eprint { ... }`
  is a parse error (`parse_eprint_stmt` only ever implemented the
  flat comma-list form; `print`'s block form is a distinct,
  dedicated parser path with its own desugar in the checker,
  confirmed nowhere near eprint's parser code). Fixing this for
  real would mean adding a new `Stmt`/`TypedStmt` variant threaded
  through ~19 separate statement-walking passes in `checker.rs`
  (recursion/heap/stack-depth/WCET/purity/... analyses each have
  their own `Stmt::PrintBlock` arm) — logged as a real gap, out of
  scope for a doc-audit pass; corrected the tutorial to show
  `eprint` used as repeated flat statements instead. (2) Every
  "Expected output" block in the file that padded a label with a
  trailing space before its comma-separated value (`"  a     = ", a`)
  was wrong by one space per item — `print`/`eprint` always insert
  their own single separating space on top of whatever's already
  in the string, so the real output was double-spaced
  (`a     =  3`) against the file's own claimed single-spaced
  output (`a     = 3`). Fixed by dropping the manual trailing space
  from each label literal (letting the auto-inserted space do the
  job alone), which restores the originally-intended alignment
  exactly rather than just editing the claimed output text to
  match a worse-looking real result. (3) The `label, ":";` group
  had the same problem in an uneditable form (`label` is a
  variable, not a literal, so there's no space to trim) — switched
  to `label + ":";` (string concatenation) so the colon abuts the
  label with no compiler-inserted gap, matching the file's own
  claimed `test:` output.

- [x] **`beginner/05c_loop_labels_primer.md` had two more issues,
  fixed 2026-07-29.** (1) Same double-space class as the print-block
  file: `print i, " ", j;` (a manual `" "` separator item) double-
  spaces against `print`'s own auto-inserted separator, so real
  output was `0   0` (three spaces) against the file's claimed
  `0 0` — fixed in both places it appeared (the `break label` and
  `continue label` examples) by dropping the manual `" "` item
  entirely and letting `print`'s automatic single space do the
  separating. (2) The challenge solution's claimed output
  (`found: 6 * 7 = 42`) was simply wrong for the algorithm as
  written: `j` runs fully for each `i` before `i` advances, so the
  actual first pair hit in iteration order is `(3, 14)` — `search`
  breaks out before `i` ever reaches `6`. Verified by running the
  exact solution code; corrected the claimed output and added a
  one-line explanation of why `(3, 14)` and not `(6, 7)`.

- [x] **`beginner/06_strings.md`'s builtins-reference table had two
  wrong signatures, found doing a full re-pass of the file (past
  the "Quick example" already fixed 2026-07-28), fixed 2026-07-29.**
  `str_index_of(s, sub)` was documented as returning plain `i64`
  ("-1 if absent") — the real return type is `Option<i64>`
  (`Option.None` if absent, not a sentinel `-1`); confirmed via
  `check`, which rejects printing the result directly ("cannot
  print an enum directly") and requires `option_unwrap_or` first.
  `str_join(v, sep)` was documented as taking `Vec<OwnedStr>` by
  value — it actually requires `ref Vec<OwnedStr>` (confirmed via
  `check`: "str_join() arg 0 must be `ref Vec<OwnedStr>`, got
  Vec<OwnedStr>"). Every other entry in the table (13 of 15
  builtins) was verified correct by direct call.

- [x] **`beginner/06a_pointers_refs_primer.md` recommended `unsafe`
  raw pointers for FFI, contradicting the FFI primer's own
  (verified) guidance — fixed 2026-07-29.** Twice in the file
  ("Does vāṇी have pointers?" and "When do you actually need
  unsafe"), the "when you need unsafe raw pointers" list included
  "FFI with a C library that passes raw pointers." This directly
  contradicts `intermediate/09a_ffi_primer.md` (fixed earlier this
  same tutorial-audit arc, 2026-07-28): raw pointer types never
  cross the `extern fn` boundary at all, unsafe or not — the
  checker rejects them outright. The correct, unsafe-free idiom
  for FFI pointer parameters is `ref T` / `mut ref T`, which
  already compile down to plain pointers at the ABI level. Fixed
  both mentions to point at the real guidance instead of inventing
  a plausible-sounding but wrong one; this file has no compiler
  examples of its own (by design — it's a pre-code intuition
  primer), so the fix was prose-only, cross-checked against the
  other file's own confirmed compiler behavior rather than a fresh
  `check` run.

- [x] **`beginner/07a_tuples_primer.md` had four confirmed-wrong
  code claims, despite opening with "this chapter has no compiler
  code" (it has many `vani` blocks) — fixed 2026-07-29.** (1)
  Claimed `match` supports tuple patterns (`match position { (0, 0)
  then "origin", ... }`) — false, parse error; `match` only
  supports variant/literal/wildcard/slice patterns (same
  restriction already documented in `08a_pattern_match_primer.md`
  from an earlier pass this same audit arc). Replaced with the
  real idiom: destructure into named locals, then `if`/`else`.
  (2) Claimed direct `.1` access works on a non-Copy tuple slot
  (`let s: OwnedStr = pair.1;`) — false: the checker rejects it
  outright ("tuple element 1 has non-Copy type OwnedStr — direct
  `.1` access would alias the tuple's heap data. Use tuple
  destructuring..."), in two separate sections (indexed-access and
  the ownership example). Fixed both to use
  `let (_, v) = tuple;` instead. (3) Claimed one-shot nested
  destructuring works (`let ((sx, sy), (ex, ey)) = line;`) — false,
  parse error; `let` patterns are one level deep only, so staged
  destructuring (outer first, then each inner `let`) is the only
  way. (4) Claimed "partial moves work field-by-field, same as
  structs" — false and worth calling out explicitly: a struct's
  `p.field` access is a true partial move (other fields stay
  usable), confirmed by direct test; a tuple has no equivalent —
  `.N` only works on Copy slots at all, and the destructuring
  required for a non-Copy slot consumes the *entire* tuple,
  including already-`.N`-read Copy slots. Rewrote the bullet to
  state the real, more limited rule and point at structs for
  genuine partial-move needs. The other ~10 code examples in the
  file (divmod via tuple return, fn-arg destructuring, nested
  tuples via staged `let`s, tuple-of-structs, tuple+Box, tuple+Vec,
  tuple-as-struct-field, Vec-of-tuples via `for`, 4-way mixed-type
  tuple) were all verified correct end-to-end.

- [x] **`beginner/08b_errors_primer.md`'s two primary worked
  examples, and its exhaustiveness-error illustration, all used a
  statement-form `match` with `print` calls inside the arms —
  parse error, since `match` is expression-only in v1 (correctly
  documented one file earlier, in `08_match.md`, but violated
  here) — fixed 2026-07-29.** All three code blocks wrote
  `match result { Option.Some(v) then print ..., Option.None then
  print ..., }` as a bare statement; `vanic check` rejects it with
  "expected statement" before ever reaching the logic the examples
  were meant to demonstrate. Fixed the two worked examples (the
  `safe_div(10, 2)` and `safe_div(10, 0)` cases) by building an
  `OwnedStr` message inside the match expression and calling
  `print` once, outside it — verified both produce the file's
  exact claimed output ("answer = 5" / "division by zero"). Fixed
  the exhaustiveness-illustration block similarly (wrapped in
  `let v: i64 = match result { ... };` so the checker actually
  reaches the exhaustiveness check instead of dying at parse time
  first) — verified the real diagnostic is "non-exhaustive match:
  missing arm for 'Option__i64.None'", matching the block's own
  claimed error in spirit.

- [x] **BUG-33 (found, fixed 2026-08-01 — high value, needed careful work in
  the SMT proof engine). `ensures` clauses fail to resolve a
  `let`-bound return value, always with the same nonsensical
  counterexample — a soundness-adjacent false-negative in a
  headline "verifiable language" feature, affecting an extremely
  common code shape.** Found writing the `ensures` addendum for
  `beginner/09_smt_intro.md`. Repro (any of these three, all fail
  identically):
  ```vani
  fn double(n: i64) -> i64
  requires n >= 0;
  ensures _return == n * 2;   // or even the trivial `ensures _return >= 0;`
  {
    let r: i64 = n * 2;       // or even `let r: i64 = n;` for the trivial case
    return r;
  }
  ```
  Every variant rejects with `error: function 'double' ensures
  clause does not hold at this return [counterexample: n = 0, r =
  -1]` — a **nonsensical** counterexample, since `r` is defined as
  `n * 2` (or `n`) two lines above and can never independently be
  `-1` while `n = 0`. `return n * 2;` (no intermediate `let`)
  compiles cleanly with the identical `ensures` clause — confirmed
  the bug is specifically about the `let`-then-`return` shape, not
  the arithmetic or the postcondition. Root cause, diagnosed but
  not fixed: `verify_ensures_at_return` (`checker.rs`) substitutes
  `_return` in the `ensures` expression with the literal AST of the
  `return` statement's expression via `substitute_expr` — for
  `return r;` that's just `Var("r")`, with no knowledge of what
  `r` equals. It then calls `prove_with_calls(substituted,
  smt_facts, ...)`, but `smt_facts` (built incrementally as each
  `Stmt::Let` is checked) has NO general case that records a plain
  scalar `let name = expr;` as an equality fact — the existing
  fact-recorders (`record_vec_builtin_facts`,
  `record_array_element_facts`, `record_ensures_facts`) only cover
  Vec-builtin calls, array/Vec literals, and calls to functions
  that themselves have `ensures` — never an ordinary scalar
  arithmetic/boolean RHS. So `r` reaches the prover as a genuinely
  free variable, and the solver correctly (from its point of view)
  finds `r = -1` disproves `r >= 0`. Contrast: `assert` *inside* the
  same function discharges fine using `r`'s definition (confirmed:
  `assert r >= n;` works in the exact same function) — that must go
  through a separate, later mechanism (an SMT-elision pass over the
  full typed body, per the "assert is free at runtime when SMT
  discharges it" behavior another part of this same tutorial
  documents) that isn't reused here. Also confirmed a tempting-
  looking "just add `assert r == n * 2;` before the `return`"
  workaround does NOT help — `Stmt::Assert`'s handler doesn't push
  its condition into `smt_facts` either, so the fact still never
  reaches `verify_ensures_at_return`.
  **Why not fixed this session**: this is core SMT-proof-engine
  code, the highest-risk part of the compiler to touch under time
  pressure — a wrong fix risks the opposite, far worse failure mode
  (soundness: an `ensures` clause appearing to hold when it
  doesn't), and this session already had one real regression
  (BUG-31's follow-up commit) caught only by CI. Two candidate fix
  directions, neither attempted: (a) add a general scalar-`let`
  equality fact to `smt_facts` (architecturally consistent with the
  existing Vec/array-literal fact-recorders, but touches a hot path
  every contract-bearing function's every `let` runs through — wide
  blast radius); (b) narrower — before substituting into `ensures`,
  recursively resolve any `Var` in the return expression back to
  its most recent same-function `let` RHS (mirroring whatever the
  assert-elision pass already does), scoped only to
  `verify_ensures_at_return`'s own substitution step. (b) is
  probably the safer starting point. Whoever picks this up should
  find and read whatever pass discharges `assert r >= n;` first —
  it already solves the "resolve a local through its let-chain"
  problem correctly for a sibling contract keyword.
  **Workaround documented in the tutorial**: return the expression
  directly (no intermediate `let`) in any function carrying an
  `ensures` clause on that return value.
  ✅ Fixed 2026-08-01, in the "fix documented TODO bugs" pass, using
  candidate direction (a) from this entry: `smt_facts` now records
  a `name == expr` fact for any scalar `let` whose RHS is a pure
  arithmetic/boolean shape (`is_smt_arithmetic_shape`). The fact is
  true by construction, so it can only let the solver prove MORE
  things, never accept something unsound. This DID surface the
  "wide blast radius" risk this entry itself flagged — two real
  regressions in loop-invariant preservation checking, found and
  fixed in the same pass: (1) a stale pre-loop fact leaking into
  preservation checking pinned a loop variable to its initial value
  instead of letting it range over the havoc'd invariant-satisfying
  states, defeating the whole point of preservation checking; (2) a
  scrub attempt for (1) collided with a user-written invariant of
  the identical `Var == expr` shape and deleted the invariant
  ASSUMPTION itself. Final fix wraps the new fact in a dedicated
  `__smt_scalar_let_eq` marker Call (mirroring the existing
  `__smt_array_eq` pattern, with a matching smt.rs encoder arm) so
  it can never again be confused with real user code. New tests: 3
  checker-level (original repro, a soundness check confirming a
  genuinely wrong `ensures` is still rejected, and a regression
  guard for the exact combination that broke both prior attempts)
  plus a real end-to-end test asserting the correct runtime value
  on both backends. Full `cargo test --release --workspace`: 13/13
  test binaries clean, 0 failed, run four times across the fix's
  iterations. Commit `304c922`.

- [x] **`beginner/09a_modules_primer.md` through `13a_big_o_primer.md`
  (rest of the beginner track) audited 2026-07-29 — one confirmed
  doc bug found.** `13a_big_o_primer.md`'s "ships annotated as
  `O(n^2)`" and its "Reading vāṇी's annotation output" code block
  used a caret (`O(n^2)`) for what's presented as literal
  `--big-o` terminal output — the real output uses the Unicode
  superscript digit (`O(n²)`), confirmed via raw byte inspection
  (`c2 b2` = U+00B2). Fixed both spots; left the *other* seven
  `O(n^2)` mentions in the file alone since those are genuinely
  general math notation (discussing the complexity class in prose,
  a table, or a summary bullet), not transcriptions of compiler
  output. `09a_modules_primer.md`, `10_modules.md`,
  `11_challenges.md`, `12_devanagari.md` (including the Devanagari-
  numeral output `१२`) all verified fully correct end-to-end,
  including exact error-message text in three cases.

- [x] **Started intermediate-track audit 2026-07-29.
  `intermediate/01_struct_methods.md` fully clean.
  `intermediate/02_enums_payloads.md` overclaimed "`Box<T>` is
  unsupported" — false and misleading; `Box<T>` works fine in
  general, including a self-referential *struct*
  (`struct Node { next: Option<Box<Node>> }` compiles and runs) —
  confirmed by direct test, and already independently demonstrated
  working in `beginner/07a_tuples_primer.md`'s "Tuple containing a
  Box" section. What's actually unsupported is narrower: boxing an
  *enum* value (`box()` rejects non-struct/non-scalar element
  types) and enum payloads that are non-Copy structs (so a struct
  holding `Box<T>` fields can't be used as an enum payload either)
  — meaning a recursive *enum* genuinely needs the arena-index
  workaround the file already shows, but a recursive *struct*
  doesn't. Rewrote the bullet to state the real, narrower
  restriction instead of the blanket wrong one.** All other
  examples in both files (struct methods worked example + missing-
  field error + challenge; enum worked example + let-destructure
  rejection + arena-index tree + Color/brightness challenge)
  verified correct end-to-end.

- [x] **BUG-34 (found, fixed 2026-08-01 — real compiler bug, root cause
  identified but the fix touches generic-enum monomorphization
  timing). `if let`/`while let` reject a direct call to a function
  returning the builtin generic `Option<T>` (or presumably
  `Result<T,E>`) as their scrutinee, even though the identical call
  works fine as a `match` scrutinee or when pre-bound to an
  explicitly-typed variable.** Found writing
  `intermediate/02b_match_enhancements.md`'s slice/while-let
  examples. Repro:
  ```vani
  if let Option.Some(v) = parse_int("42") { print v; }
  ```
  fails with `error: enum 'Option__i64' not declared`. Both of
  these succeed on the identical logic:
  ```vani
  let v: i64 = match parse_int("42") {
    Option.Some(n) then n, Option.None then -1,
  };                                          // match: fine

  let r: Option<i64> = parse_int("42");
  while let Option.Some(v) = r { ... }         // pre-bound var: fine
  ```
  Root cause, diagnosed but not fixed: `check_iflet_stmt` (and its
  `while let` counterpart, `checker.rs`) type-checks the scrutinee
  via the ordinary `check_expr`, which correctly resolves to
  `Type::Enum("Option__i64")`, then does its own direct
  `env.lookup_enum("Option__i64")` — this lookup fails because
  nothing has registered the monomorphized `Option__i64` enum into
  `env` at that point. A pre-bound `let r: Option<i64> = ...;`
  works because the *explicit type annotation* is what triggers
  registration (via the program-wide `monomorphize_type_decls_in_program`
  pass, which walks `Type::Apply` occurrences). Regular `match`
  somehow resolves the same call-expression scrutinee successfully
  with no explicit annotation anywhere in scope — so `match`'s path
  must trigger monomorphization (or reach a registry `if let`/
  `while let` don't) somewhere between its own `check_expr` call and
  its own enum lookup; tracing exactly where match's checker differs
  from `check_iflet_stmt`/`check_whilelet_stmt` is the next step for
  whoever picks this up. **Not fixed this session** — generic
  monomorphization ordering is shared, sensitive infrastructure
  (touches every generic type in the language), and this session
  already had one real regression from touching adjacent shared
  logic (BUG-31's follow-up commit). Only ever affects the
  **builtin generic enums** (`Option<T>`, presumably `Result<T,E>`)
  — user-defined non-generic enums (`enum Opt { None, Some(i64) }`)
  are completely unaffected, confirmed by extensive testing, which
  is why `02b_match_enhancements.md`'s examples route around this
  entirely by using the tutorial's own hand-rolled `Opt` enum
  instead of the builtin `Option<T>` for every `if let`/`while let`
  example (a real, verified, zero-cost workaround — not a
  compromise, since a hand-rolled enum is often clearer in tutorial
  code anyway).
  ✅ Fixed 2026-08-01, in the "fix documented TODO bugs" pass. The
  real root cause was different from this entry's own guess (which
  suspected monomorphization ORDERING): instrumented both the
  if-let and match code paths directly and compared `env.enums`'s
  actual contents at the failure point. Found `walk_stmt_kids`
  (checker.rs) — the statement walker the `Option<T>`
  auto-registration pre-pass uses to find builtins like `parse_int`/
  `find`/`pool_get` that return `Option<T>` even with no explicit
  type annotation anywhere — had `If`/`While`/`For`/`ForIter`/
  `UnsafeBlock` arms but was simply MISSING `IfLet`/`WhileLet`
  arms entirely, silently falling through to the catch-all `_ =>
  {}`. `match` never hit this because `Stmt::Let { expr: <match> }`
  walks fine on its own, and `walk_expr_kids` already handles
  `ExprKind::Match` correctly. Fixed by adding the two missing arms
  (walking the scrutinee plus both branches, matching the shape of
  the existing `If`/`While` arms) — this was a self-contained,
  low-risk missing-arm fix, not the "shared, sensitive
  infrastructure" this entry originally feared touching. New
  tests: 2 checker-level (`src/lib.rs`) plus a real end-to-end test
  (`tests/run_end_to_end.rs`) asserting correct runtime output on
  both backends. Full `cargo test --release --workspace`: 13/13
  test binaries clean, 0 failed. Commit `2ffbd85`.

- [x] **`intermediate/02b_match_enhancements.md` — the single most
  broken file found this entire tutorial-audit arc: at least 9
  distinct confirmed bugs across a 519-line file, essentially every
  code example needed a fix. Found+fixed 2026-07-29.** In order of
  appearance: (1) shared-setup `try_parse` used `len(s)` where
  `len` returns `u64` and the function needs `i64` — missing `as
  i64` cast. (2) The `if let` "desugars to" illustration used
  statement-form `match` (parse error — `match` is expression-only,
  see the `08b_errors_primer.md` fix earlier this arc) — reframed
  as an explicitly-illustrative, non-runnable block. (3)
  `drain_all` called a nonexistent `vec_len` (real name: `len`) and
  used an `i64` loop index where Vec indexing needs `u64`. (4)
  `sum_stack` assumed a `vec_pop` builtin that returns `Option<i64>`
  — doesn't exist; the real `pop(mut ref xs)` returns a bare `i64`
  and *aborts* on an empty Vec — rewritten to guard on `len(xs) >
  0` instead of (fictional) `while let`-driven draining. (5)+(6)
  The `Dir`/`Msg` or-pattern illustrations were statement-form
  `match` with `print` inside arms (same class of bug as #2) —
  both wrapped in real functions returning a value, `print`ed
  outside. (7) All three of `describe_vec`/`first_and_last`/
  `rgb_to_hex`'s slice-match functions took `xs: ref Vec<i64>` and
  matched via `match ref xs` (double-ref — "cannot create a
  reference to a reference") or bare `match xs` on a ref parameter
  ("match scrutinee must be an enum, integer, or bool type, got
  Vec<i64>") — slice-pattern matching requires the `Vec` **by
  value**; only the file's *fourth* slice example (`classify_scores`,
  evidently modeled on the real, already-tested
  `examples/language/english/slice_pattern_guards.vani`) had this
  right. Fixed all three to take `Vec<i64>` by value. (8)
  `rgb_to_hex` additionally called a completely fabricated
  `int_to_hex` builtin (no hex-conversion builtin exists in the
  language at all) and used `return` inside a match-arm block (the
  same expression-vs-statement restriction as #2/#5/#6) — simplified
  to `rgb_to_packed` returning the packed `i64` directly, with the
  arm's block ending in a tail expression instead of a `return`.
  (9) The combined example declared `enum Task { ... }` — `Task` is
  a **reserved built-in type name** (the `task <fn>(...)`
  concurrency primitive), rejected outright with "enum name 'Task'
  is a reserved built-in type" — renamed to `Job` throughout, and
  fixed a second, independent bug in the same example: indexing a
  non-Copy enum element by value (`tasks[i]`) or ref-borrowing
  through an index (`ref tasks[i]`) are both rejected ("would alias
  the owner's slot and double-free" / "`ref` can only borrow a
  named variable or a struct field") — fixed via `clone_at(tasks,
  i)`, plus the same `i64`-index/`vec_len` issues as `drain_all`.
  (10) The challenge's `enum Shape { ..., Triangle(i64, i64) }` hit
  the *same* single-payload-per-variant restriction found earlier
  this arc in `02_enums_payloads.md` ("only single-field payloads
  supported in v1") — wrapped the two sides in a new `TriSides`
  struct payload, matching that file's own documented workaround —
  and its `total_perimeter` used the same fictional `vec_pop`
  Option-returning pop as #4, redesigned as an index-scan with
  `clone_at` (matching the fix in #9) rather than a stack-drain.
  Verified the ENTIRE corrected file's example set end-to-end in
  one combined program on both backends (identical output on each)
  before writing anything back into the doc — given how many
  independent things were wrong, no single fix was trusted in
  isolation.

- [x] **BUG-35. LLVM backend crashed on `Option<Box<Node>>` — the
  canonical recursive-struct shape the `Box<T>`/RAII tutorial
  itself teaches — with "integer constant must have integer
  type". Same bug class as BUG-29 (2026-07-28), found auditing
  `intermediate/03a_box_raii_primer.md`'s own examples.** The
  `payload_zero` zero-init placeholder for a payload-less enum
  variant (`Option.None` here) treats pointer-shaped payload
  types specially (needs LLVM's `null` literal, not the integer
  `0`) — but only listed `Type::Str`/`Type::OwnedStr`, the exact
  gap BUG-29 fixed. `Box<T>` (and raw pointers) also lower to a
  bare pointer and fell through to the same `_ => "0"` default,
  so any `struct Node { ..., next: Option<Box<Node>> }` — a
  linked-list/tree node, about as common a shape as exists —
  crashed the LLVM backend the instant a value with a `None` was
  constructed (works fine on the C backend the whole time,
  confirmed). Fixed by adding `Type::Box(_)` /`Type::Ptr(_)` /
  `Type::PtrMut(_)` to the exact same match arm BUG-29 already
  fixed for `Str`/`OwnedStr`. Verified end-to-end on both
  backends; full `cargo test --workspace`: clean (only the
  already-known pre-existing Windows-local `lli`/`cc` link-flag
  gaps in `vtables_phase3.rs`/`ssa_backend_llvm_crosscheck.rs`/
  `user_drop_by_ref.rs`'s own test helpers, confirmed passing on
  Linux CI both before and after this session's changes). New
  example: `examples/language/english/option_box_recursive_struct.vani`.
  New test:
  `option_box_recursive_struct_example_produces_correct_output_on_both_backends`
  in `tests/run_end_to_end.rs`.

- [x] **`intermediate/03a_box_raii_primer.md` claimed two `Box<T>`
  shapes work that are explicitly rejected in v1 — fixed
  2026-07-29.** "`Box` of a tuple" (`box((42, "answer"))` for
  `Box<(i64, OwnedStr)>`) and "`Box<Box<T>>`" (`box(inner)` where
  `inner: Box<i64>`) both fail with the identical diagnostic:
  "box() v1 supports Copy + sized element types (primitives, Copy
  structs), `dyn Iface`, `Vec<T>`, and `OwnedStr`... Other owning
  inner types (`Box<Box<T>>`, `Box<HashMap<…>>`, etc.) remain a
  follow-up" — the error message ITSELF names `Box<Box<T>>` as a
  known future gap, directly contradicting the tutorial's own
  "the compiler accepts it" / "two heap allocations" framing of
  the identical pattern. Rewrote both sections into one "Two
  things `Box<T>` does NOT support yet" section stating the real
  boundary. All other examples in the file (move, borrow,
  `Box<Vec<T>>`, `Box<OwnedStr>`, `Box<dyn Iface>`, `Option<Box<T>>`
  type declaration) verified correct.

- [x] **BUG-36 -- STATUS CORRECTION (2026-08-04): this entry
  originally read "found, NOT fixed" (below, preserved for
  history/context) and was deliberately left open through several
  later passes as too risky to rush ("a substantial new checker
  subsystem... could itself introduce false rejections across a
  huge amount of existing working code"). It was, in fact, FIXED
  2026-08-02 -- a separate pass landed a deliberately narrow,
  lexical-scope (not full NLL), named-`let`-binding-only
  enforcement, validated against a full test run PLUS a byte-
  identical before/after `vanic check` diff of all 1034 `.vani`
  files under `examples/` (zero new rejections). This doc simply
  never got reconciled after that fix landed, so the entry sat
  contradicting itself two ways in this same file. Full writeup:
  search "BUG-36 (fixed 2026-08-02" below. Re-verified directly
  2026-08-04 against the exact repro in this entry -- correctly
  rejected on both backends with a clear diagnostic
  ("cannot use 'xs' while it is mutably borrowed by 'r'").
  Original "found, NOT fixed" writeup, preserved for context:
  The "single mutable borrow" exclusivity rule (`ref`
  can multiply, `mut ref` must be exclusive — the rule that's
  supposed to make aliased mutation, and therefore data races, a
  compile error) is not enforced by the checker in any tested
  shape. Found auditing
  `intermediate/03b_affine_deeper_primer.md`'s own worked
  example, which explicitly claimed this compiles-error:
  ```vani
  let xs: Vec<i64> = vec(1, 2, 3);
  let r: mut ref Vec<i64> = mut ref xs;
  push(r, 4);
  print xs[0];   // claimed: REJECTED -- xs is mutably borrowed
  ```
  It doesn't reject — compiles and runs cleanly on both backends,
  no diagnostic at all. Confirmed not a narrow gap in one shape:
  identical result whether the `mut ref` is stored in a `let` or
  passed inline (`push(mut ref xs, 4); print xs[0];`), and
  whether the conflicting access on the other alias is a read or
  a write (`set(r, 0, 99); print xs[0];` also compiles clean).
  [Note (2026-08-04): the fix below deliberately does NOT cover
  the inline-call-argument shape -- it's structurally untracked by
  design, matching the tutorials' own pre-existing "borrow ends at
  call return" model. Only the NAMED-`let`-binding shape shown in
  the repro above is enforced.]
  Only the **affine move rule** was enforced (a value can't be
  read after being *moved*) — there was no pass tracking "an
  outstanding `mut ref` alias makes the original binding
  temporarily unreadable."

- [x] **`03b_affine_deeper_primer.md`'s "conditional moves" flagship
  example also didn't demonstrate what it claimed — fixed
  2026-07-29.** The "broken" example (meant to show the compiler
  rejecting a value read after only one `if`-branch moves it) had
  the moving branch `return` early — which means the compiler's
  flow analysis correctly proves the post-`if` code is only ever
  reached via the non-moving branch, so it's genuinely safe and
  the checker accepts it (confirmed: `vanic check` says "ok",
  contradicting the tutorial's claimed rejection). This is a
  real, positive finding about the checker being *more precise*
  than the tutorial gave it credit for, not a compiler bug —
  early-return branches are correctly excluded from the
  "possibly moved" conservative join. Replaced with a version
  where the moving branch does NOT return early (assigns instead)
  — confirmed genuinely rejected — so the example actually
  demonstrates the join-point problem it's named after. The
  file's own "fixed" version was already correct as written (it's
  exactly the early-return shape that makes the original "broken"
  example not actually broken).

- [x] **`intermediate/03c_shared_ownership_primer.md`'s channel and
  Mutex patterns (4 and 5) both had multiple broken claims — fixed
  2026-07-29.** (1) `Channel<Vec<i64>, 8>` — the doc's own example
  — is rejected: "Channel element type must be an integer width or
  bool, got Vec<i64>." `Channel<T, N>`'s `T` is scalar-only in v1;
  you send a computed result/handle, not an owning value like a
  `Vec` directly. (2) Both patterns used the block form
  `task name { ...captures an outer non-Copy binding... }` —
  rejected outright ("task body captures non-Copy binding...
  Captures must be Copy types"), confirmed for `Channel<i64, 8>`
  AND `Mutex<T>` alike (not narrow to one type). The fix in both
  cases: define an ordinary function taking the shared value by
  `ref`, and spawn it with the expression form
  `task fn_name(ref x)` instead. (3) Task bodies can only call
  `pure fn`s ("task body cannot call non-pure function") — the
  doc's `process`/`expensive_compute` calls inside `task { }`
  blocks would have hit this too even setting aside the capture
  issue. (4) The Mutex pattern's `g.value = g.value + 1;` direct
  field-assignment on a `Guard<T>` is rejected — "only structs
  support field assignment ... Guard<T>` isn't one; the real API
  is `guard_get(ref g)` / `guard_set(mut ref g, new_value)`,
  confirmed working for both a struct-typed and (separately) a
  scalar-typed `Mutex`. Rewrote both patterns as fully verified,
  runnable programs (previously undefined-helper fragments) using
  the real expression-form `task`/`Task<R>`/`join`/`guard_get`/
  `guard_set` APIs throughout.

- [x] **BUG-38 (found, fixed 2026-08-01 — internal compiler error, not a
  clean rejection). `clone_at()` on a `Vec<Box<dyn Iface>>` element
  panics the compiler itself** (`internal error: entered unreachable
  code: clone_at on element type Box(Object("Observer")) not yet
  supported in tree-LLVM`, `src/backend_llvm.rs:9992`) **instead of
  emitting a normal diagnostic.** Found while working out the
  correct v1-compatible design for
  `intermediate/03d_cyclic_references_primer.md`'s observer-pattern
  example (originally written with `Vec<Box<dyn Observer>>`, which
  needs `clone_at` for indexed dispatch since a `Box<dyn Iface>`
  element isn't Copy). Worked around in the doc by switching to
  `Vec<dyn Observer>` (unboxed `dyn Iface` is a Copy fat pointer, so
  direct indexing `w.observers[idx as u64]` works with no
  `clone_at` needed at all) — confirmed correct on both backends.
  Separately, `examples/edge_cases/mix_vec_of_box_dyn.vani`'s own
  comment claims "v1 C-codegen has a known issue with `Vec<dyn
  Iface>` as a struct field," which is why that file and
  `design_patterns/behavioral/observer.vani` both use `Vec<dyn
  Observer>` as a function *parameter* rather than a struct field —
  but a direct test this session (`World { observers: Vec<dyn
  Observer> }`, built + populated + dispatched through) worked
  cleanly on both backends, so that comment may already be stale;
  not chased further. **Not fixed this session**: the ICE itself is
  low-severity (there's always a working alternative — use unboxed
  `dyn Iface` — so it doesn't block real code), and turning it into
  a clean checker-time rejection is a small but separate fix from
  BUG-37's double-free; logged here rather than bundled in.
  **✅ Fixed 2026-08-01**, in the "fix documented TODO bugs" pass.
  Turned out to be a bigger deal than the ICE alone suggested:
  confirmed by direct testing that the C backend doesn't panic for
  the same `Vec<Box<T>>` input (ANY `Box<T>`, not just `Box<dyn
  Iface>` — also reproduced with plain `Box<i64>`) but instead
  **silently double-frees at runtime** (`free(): double free
  detected in tcache 2`) — a real memory-safety bug the original
  report never surfaced, since it only looked at the LLVM side.
  Fixed with the checker-time rejection this entry already
  recommended: `check_clone_at_builtin` (checker.rs) now rejects any
  `clone_at` element type that isn't one of the types its codegen
  actually supports (Copy, `Vec<T>`, `OwnedStr`, struct, enum,
  tuple) — covering `Box<T>` and, by the same reasoning, any other
  affine type (`Mutex<T>`, `HashMap<K,V>`, etc.) nested in a `Vec`,
  not just the originally-reported `Box<dyn Iface>` shape. New
  tests: 2 checker-level (`Box<i64>` and `Box<dyn Iface>`) in
  `src/lib.rs`, plus a real end-to-end test in
  `tests/run_end_to_end.rs` that runs both backends via the actual
  CLI and asserts neither a Rust panic nor a double-free occurs.
  Full `cargo test --release --workspace`: 13/13 test binaries
  clean, 0 failed. Commit `0a74b62`.

- [x] **BUG-37. LLVM backend: `clone_at()` on a `Vec<Struct>` element
  double-freed when the struct had a nested non-Copy `Vec<T>` field
  — exit 116, heap corruption. Found auditing
  `intermediate/03d_cyclic_references_primer.md`'s tree-building
  example.** Building a tree (`struct Node { value: i64, children:
  Vec<i64> }`, `struct Tree { nodes: Vec<Node> }`) the natural way —
  since `mut ref t.nodes[i].children` is rejected (two-hop `mut ref`,
  see the standing `ref`/`mut ref` single-level-place restriction) —
  requires the `clone_at(t.nodes, i)` / mutate the clone / `set(mut
  ref t.nodes, i, clone)` idiom. `clone_at`ing a node crashed the
  LLVM backend the instant the clone (or the original tree) went out
  of scope; the C backend produced correct output the whole time.
  Root cause: TWO independent LLVM codegen sites build a per-field
  deep clone of a struct element and both only special-cased
  `Type::OwnedStr` as needing a real clone call, falling through to
  a bare `extractvalue`/`insertvalue` shallow copy for every other
  field type — including a nested `Vec<T>`. The "cloned" struct's
  `children` field ended up pointing at the exact same heap buffer
  as the original still sitting in the source `Vec`; both later got
  freed independently (once via the clone's own scope exit, once via
  the tree's), corrupting the allocator. The two sites: (1)
  `emit_vec_bundle_functions`'s `__clone` bundle function (used by
  the general `Vec::clone()` builtin), and (2) `clone_at`'s own,
  entirely separate inline struct-clone codegen — confirmed via IR
  inspection that fixing site (1) alone changed that bundle
  function's IR correctly but did NOT fix the end-to-end crash,
  because `clone_at()` compiles through site (2), not the bundle
  function. Both are the same "parallel dispatch function" bug class
  as BUG-29/BUG-35: a per-type dispatch match gets the right handling
  added in one place while a structurally identical sibling dispatch
  elsewhere keeps the old incomplete fallback. Fixed by adding a
  `Type::Vec(inner) => { ... call @intent_vec_{tag}__clone on the
  field ... }` arm to both dispatch sites' `match fty` (mirroring the
  `Type::Vec(inner)` handling that already existed correctly for the
  direct, non-struct-field Vec-element case in `clone_at`). Verified:
  minimal repro and the fuller tree-building example both now
  produce correct, matching output on LLVM and C backends (previously
  LLVM crashed, C was already correct); full `cargo test --workspace`
  clean (only the already-known pre-existing Windows-local
  `ssa_backend_c_crosscheck.rs` `-lsynchronization` link-flag gap,
  confirmed passing on Linux CI). New example:
  `examples/language/english/clone_at_struct_with_nested_vec_field.vani`.
  New test:
  `clone_at_struct_with_nested_vec_field_example_produces_correct_output_on_both_backends`
  in `tests/run_end_to_end.rs`. Also checked the adjacent
  `Type::Enum` per-payload clone dispatch in `clone_at` right after
  this Struct arm: it has a real-looking gap of its own (only
  `Type::OwnedStr` payloads get a deep-clone branch; any other
  payload type with `payload_tags` non-empty falls into the
  "tag-only" round-trip, which never inserts a payload into `dest`
  at all) — but a minimal probe (`enum Item { Empty, Full(Vec<i64>)
  }`, construct + `match` on `Item.Full(...)`, no `clone_at`
  involved) already fails identically on BOTH backends (exit 9, no
  output) before `clone_at` even enters the picture, so this is a
  separate, pre-existing gap in `enum`-with-non-scalar-payload
  support generally, not part of the BUG-37 double-free pattern and
  not caused by either of this session's fixes. Not investigated
  further this session (out of scope for BUG-37) — worth a
  dedicated follow-up.

- [x] **`intermediate/04c_generics_primer.md`'s central worked
  examples described a `Comparable` bound that doesn't exist and
  can't be expressed in v1 — fixed 2026-07-29.** Found continuing
  the sequential audit past `03_affine.md`. The chapter's "cookie
  cutter" example, its "What the compiler ACTUALLY does"
  monomorphization walkthrough, and its later "Generic with
  bounds" section all used `fn max<T>(a: T, b: T) -> T where T is
  Comparable { if a > b { ... } }`, called with `i64`/`f64`/`u32`
  literals directly. None of it compiles: (1) v1 has no built-in
  `Comparable`/`Ord`-style interface — `where T is Comparable`
  requires a real `implement Comparable for X` block, none shown;
  (2) primitive types can't `implement` any interface at all in
  v1 (`implement Cmp for i64` is rejected outright: "requires a
  struct or enum type"), confirmed directly; (3) even with a
  struct-typed bound, generic-bound method calls dispatch through
  an explicit method (`a.cmp(b)`), never a bare operator like `>`
  on the type parameter. This is the same real pattern already
  used correctly elsewhere (`examples/language/english/bounded_generics.vani`,
  and `04_generics_iface.md`'s own Challenge section) — 04c just
  never matched it. Rewrote all three sections to use a real,
  verified `Cmp` interface + `.cmp()` method + two independent
  `Score`/`Money` structs (confirmed on both backends: two
  distinct monomorphized specializations, correct output), and
  added an explicit callout on the primitives-can't-implement-
  interfaces boundary. Also caught and noted a related, non-
  obvious constraint while verifying the rewrite: v1's generic-
  call type inference only reads the concrete type off a literal
  or a `let`-annotated variable at the `T` position — passing a
  struct-literal expression directly (`max(Score { value: 3 },
  ...)`) is rejected ("v1 generic-call inference supports literal
  arguments ... or Var ... More complex argument expressions need
  full type-checking context"); documented this too. Also fixed a
  pre-existing bare `List<String> vs List<Integer>?` (no
  backticks) causing an `mdbook build` warning in the same file.

- [x] **BUG-39. Calling an interface's default method on a type
  that INHERITS it (doesn't override it) was rejected outright —
  checker-level, both backends. Found auditing
  `intermediate/04d_default_methods_primer.md`'s own worked
  example** (`interface Describable { fn name(self: Self) -> Str;
  fn describe(self: Self) -> Str { return "I am something."; } }`,
  `struct Dog` implementing only `name`, relying on the default
  `describe`). `d.describe()` failed: "argument 1 to 'Dog_describe'
  must be assignable to Self, got Dog". Root cause: in
  `hoist_impls_into_functions` (checker.rs), when an `implement`
  block is missing a method that has a default body, the checker
  injects a synthesized function using the interface's OWN declared
  `params`/`return_type` verbatim (`iface_method.params.clone()`,
  `iface_method.return_type.clone()`) — but those still say `Self`
  literally (`Type::Struct("Self")`), because v1's normal impl-
  validation path never needs an actual Self-substitution step: a
  user-written `implement Describable for Dog { fn name(self: Dog)
  ... }` already spells out the concrete type itself, so the
  checker only does positional shape-matching, never literal
  substitution. A default method has no user-written impl body at
  all — nothing ever performed that substitution — so the injected
  function's signature said `Self` while every real call site
  passed the concrete type, and the checker's own argument-
  assignability check then rejected it. Overriding a default method
  always worked fine (the override's body is user-written, already
  using the concrete type); only pure inheritance was broken — which
  is the central, most basic use case the whole "default methods"
  feature exists for. Fixed by adding `substitute_self_type` (a
  small recursive `Type` walker covering `Ref`/`RefMut`/`Box`/`Ptr`/
  `PtrMut`/`Vec`/`Vec128/256/512`/`Array`/`Tuple`/`Apply`/`FnPtr`
  wrappers around a `Self`) and applying it to both the injected
  function's params and return type, substituting `Self ->
  imp.for_type`. Verified: the tutorial's own Dog/Cat example and
  the file's separate blanket-vs-concrete-impl override example
  both now produce correct output on both backends; full `cargo
  test --workspace` clean (only the known pre-existing Windows-local
  `ssa_backend_c_crosscheck.rs` link-flag gap). New example:
  `examples/language/english/default_method_inherited_self_type.vani`.
  New test:
  `default_method_inherited_self_type_example_produces_correct_output_on_both_backends`
  in `tests/run_end_to_end.rs`.

- [x] **`intermediate/06b_iterators_primer.md` had THREE separate,
  significant false claims about iterator-combinator semantics —
  fixed 2026-07-29.** Found continuing the sequential audit past
  05_dyn.md. (1) The flagship chain example
  (`xs.filter(...).map(...).fold(...)`, written as one fluent
  expression) doesn't parse: v1's method-call sugar only rewrites a
  receiver that's a plain named `Var` (confirmed in checker.rs's own
  comment: "Non-Var receivers (`f().map(...)`) need an explicit
  named intermediate in v1 — same rule as `ref` borrowing only named
  places"); every chained call in the file needed splitting into
  separate `let` steps. (2) The entire "Lazy evaluation" section was
  fabricated — confirmed directly against the compiler's own doc
  comment on `check_vec_map_fold_builtin`: "v1 is eager — `vec_map`/
  `vec_filter` materialize fresh Vecs. Loop fusion at monomorphization
  time is queued as a follow-up" (not shipped). `.take(3)` does NOT
  short-circuit upstream work — it slices an already-fully-computed
  Vec. (3) The "Fusion" section claimed the compiler auto-fuses
  arbitrary adjacent combinators into one loop — also contradicted by
  the same comment ("queued as a follow-up"). What v1 actually ships:
  a small set of hand-written, pre-fused combined builtins
  (`map_fold`/`filter_fold`/`map_filter`/`map_filter_fold`) covering
  specific 2-3-step shapes, verified working. **A fourth, even more
  surprising issue found while verifying the "closure connection"
  section's example**: `vec_map`/`vec_filter`/`vec_fold`'s closure
  argument is typed as a **plain non-capturing function pointer**
  (`fn(i64) -> i64` etc., confirmed against `06_closures.md`'s own
  accurate signature table) — a closure literal referencing ANY
  outer-scope variable, even a `Copy` `i64`, is rejected with "unknown
  variable" the instant it's passed to `.filter(...)`/`vec_filter(...)`,
  because a bare `fn` pointer has no environment slot to hold a
  capture in (unlike the real, capture-carrying `Closure` type
  chapter 06a describes). Confirmed this is NOT specific to
  method-call sugar — the free-function `vec_filter(ref xs, |x| x >
  threshold)` form fails identically. There is currently no working
  way to filter/map/fold a Vec using a captured value through these
  builtins in v1 — an explicit `for` loop is the only way. Rewrote
  all four sections with verified-accurate replacements (both
  backends, where applicable) and fixed a resulting `mdbook build`
  warning (bare `` `Vec<i64>` `` inside a quoted diagnostic string,
  outside backticks).

- [x] **`intermediate/06_closures.md`: whole-number `f64` prints
  without a trailing `.0` — fixed 2026-07-29.** The ref-capturing-
  closure example's inline comments claimed `apply(lookup, 2)`
  prints `"3.0"` and `data[0]` prints `"1.0"`; confirmed directly
  the real output is `"3"` and `"1"` on both backends. Corrected
  the comments; no compiler change (this is just how `print`
  formats a whole-number `f64` in this compiler, consistent
  elsewhere).

- [x] **BUG-40. `vanic run` (the `lli` JIT path) failed on EVERY
  `parallel for` program with "Symbols not found: [
  intent_pool_run ]" — `vanic build` + running the binary always
  worked fine. Found auditing `intermediate/06c_fnptr_primer.md`'s
  own "this compiles" `parallel for` example, which failed the
  instant it was actually run.** Root cause: `intent_pool_run` (the
  pthreads thread-pool runtime `parallel for` lowers into, defined
  in `parallel_runtime.c`) is compiled as a static object and linked
  into AOT (`vanic build`) binaries by `build_program_llvm` — but
  `run_program_llvm`/`run_program_llvm_capture` (the `lli` JIT paths
  behind `vanic run` and `vanic test`) never got the equivalent
  `-load`able shared-library build the way `sort()`'s runtime
  already has (`sort_runtime_shared_lib`, added for a prior,
  unrelated bug). Every `parallel for` program — a large fraction of
  the concurrency tutorials' worked examples — was silently broken
  under the exact command (`vanic run`) those tutorials tell readers
  to use. Fixed by adding `parallel_runtime_shared_lib()` (mirrors
  `sort_runtime_shared_lib()` exactly: compiles `parallel_runtime.c`
  to a `.dll`/`.dylib`/`.so`, `-shared -fPIC` plus `-lpthread`/
  `-pthread` so the shared lib's own pthread calls resolve
  standalone) and wiring its `-load=` flag into both `lli`
  invocation sites. Verified: the tutorial's own minimal example and
  the full, much larger `examples/language/english/parallel.vani`
  (covers `+`/`*`/`min`/`max`/`&`/`|`/`^`/`&&`/`||` reductions plus
  outer-scalar and outer-array captures) both now run correctly via
  `vanic run` on both backends, matching `vanic build`'s
  already-correct output exactly. New example:
  `examples/language/english/parallel_for_jit_run.vani`. New test:
  `parallel_for_jit_run_example_produces_correct_output_on_both_backends`
  in `tests/run_end_to_end.rs`.

- [x] **BUG-41. LLVM backend: `parallel for ... reduce x with *;`
  (multiplicative reduction) crashed `lli`/`llc` with "atomic load
  must have explicit non-zero alignment" — found testing
  `examples/language/english/parallel.vani` while diagnosing
  BUG-40.** `atomicrmw` has no `mul` variant, so the Mul reduction
  is the one operator lowered via a `cmpxchg` retry loop instead of
  a plain `atomicrmw`; its initial atomic load
  (`backend_llvm.rs`, the outlined-worker Mul-reduction codegen) read
  `load atomic i64, i64* %cap_N monotonic` with no `, align 8` —
  the one atomic-load call site in the entire file missing an
  alignment (every sibling site already specifies one; grepped to
  confirm). Modern LLVM requires an explicit alignment on every
  atomic load. Fixed by adding `, align 8`. Verified: both the
  minimal repro and the full `parallel.vani` example's `*`
  reduction now produce correct output (`24` and `240000`
  respectively) on both backends, cross-checked several of
  `parallel.vani`'s other reduction outputs by hand
  (`acc_or`/`acc_xor` match `10|20|30|40=62` /
  `10^20^30^40=40`). New example:
  `examples/language/english/parallel_for_mul_reduction.vani`. New
  test:
  `parallel_for_mul_reduction_example_produces_correct_output_on_both_backends`
  in `tests/run_end_to_end.rs`.

- [x] **`intermediate/09a_ffi_primer.md`'s FFI worked example used
  `sqrt` as the `extern "C"` function name, which collides with
  vāṇी's own built-in `sqrt` — fixed 2026-07-29.** `extern "C" fn
  sqrt(x: f64) -> f64;` is rejected outright: "function 'sqrt' is
  a built-in name and cannot be redefined" (vāṇी ships `sqrt` as
  one of its many bare-named math builtins). Replaced with `hypot`
  (confirmed not a builtin collision, real libm function). Also
  corrected the surrounding claim that `vanic build --link-with=m`
  is needed to resolve libm symbols — verified directly that the
  `hypot` example runs correctly via both `vanic run` and a plain
  `vanic build` with no `--link-with` flag at all (libm is folded
  into libc on this toolchain); reworded to note `--link-with` is
  for when a symbol genuinely doesn't resolve on its own, not
  something every libm call needs. Also confirmed the file's
  "vendor_sort" callback-declaration snippet (already explicitly
  labeled illustrative/hypothetical) type-checks correctly as
  written.

- [x] **BUG-42. CLI: `vanic run --backend=c file.vani` (flag BEFORE
  the file path) silently ignored `--backend=c` and ran the LLVM
  backend instead — no error, no warning. Found auditing
  `intermediate/09_ffi.md`'s `strlen` Challenge example, which
  crashed only when the flag was placed after the file
  (`vanic run file.vani --backend=c`) worked. This turned out to
  affect a large fraction of THIS SESSION'S OWN verification
  methodology — every `vanic.exe run --backend=c "$file"` spot-check
  performed with the flag before the path (the pattern used
  throughout most of this session) was silently re-running the LLVM
  backend a second time instead of actually exercising the C
  backend.** Root cause: `required_file_at` (shared by `run` and
  `build`) correctly LOCATES the positional file argument even when
  preceded by flags (it skips over them during the scan) — but it
  only told the caller to resume flag-parsing from the index AFTER
  the file, discarding every flag it had skipped over to get there.
  `parse_run_args`/`parse_build_args` never saw `--backend=c`, `-o`,
  `--link-with`, etc. if they appeared before the file. Fixed by
  having `required_file_at` return every flag argument (both before
  and after the file position, file itself excluded, relative order
  preserved) instead of just a resume-index; callers now parse the
  full reconstructed flag list from index 0. Verified: `-o path`
  before the file for `build` (previously also silently dropped, now
  confirmed correct) and `--backend=c` before the file for `run`
  (confirmed via a canary that crashes loudly if it silently fell
  through to LLVM). New test: `backend_flag_before_file_path_is_honored`
  in `tests/run_end_to_end.rs`.
  **Consequence — this fix un-hid a real, separate, pre-existing bug
  (BUG-43 below) via `tests/run_end_to_end.rs`'s own
  `echo_loop_llvm_matches_c_on_windows` test, which had used the
  buggy flag-before-path ordering and could therefore never have
  failed, no matter what the LLVM backend actually did — it was
  comparing LLVM's stdout against itself.**
  **Retrospective note on this session's own methodology**: most of
  this session's "confirmed correct on both backends" spot-checks
  used `vanic.exe run --backend=c "$SCRATCH/file.vani"` (flag before
  path) via ad-hoc scratch-file testing, NOT the `tests/
  run_end_to_end.rs` pattern (which consistently puts the flag after
  the path and was therefore unaffected). This means many of those
  ad-hoc spot-checks' "C backend" runs were silently LLVM re-runs.
  The actual compiler-BUG findings from those checks (BUG-37, 39,
  40, 41, and the doc corrections) remain valid — their root causes
  were independently confirmed via source-code reading and/or IR
  inspection, not solely via the black-box comparison — but the
  blanket "verified identical on both backends" claims for the
  surrounding tutorial examples in this session's post-compaction
  work were not all independently double-checked against a real C
  backend run. Not practical to retroactively re-verify every prior
  claim; flagging this here as an honest record. **Going forward,
  always place `--backend=c` AFTER the file path in ad-hoc testing**
  (`vanic run file.vani --backend=c`), matching the CLI's own
  documented usage string and this fix's now-verified-correct
  before-path behavior as a safety net either way.

- [x] **BUG-43. LLVM backend: a `let`-declared local inside a `task
  NAME { ... }` block body (beyond the block's own captures) crashed
  with "use of undefined value '%N.name.addr'" — a pre-existing bug
  that BUG-42 above had kept permanently hidden from
  `echo_loop_llvm_matches_c_on_windows`'s own parity test.** Root
  cause: the outlined task function's `FnCtx` (in
  `emit_task_via_pthread`, `backend_llvm.rs`) never set
  `skip_alloca_hoisting = true` the way the sibling `parallel for`
  outlined-worker codegen already does (found the exact matching
  one-line precedent a few hundred lines down in the same file).
  Without it, a scalar `let`'s alloca gets pushed into
  `ctx.alloca_preamble` (normally flushed into the function's entry
  block by a second pass so `mem2reg` can promote it) — but outlined
  functions never get that second pass, so the alloca was silently
  dropped from the emitted IR entirely while the `store`/`load`
  referencing its address were still emitted, producing an
  LLVM-IR-verifier-rejected module. Same "parallel dispatch
  function" bug class as BUG-19/22/24/27/29/35/37 this session (and
  prior sessions) — a new kind of outlined-function context needed a
  fix a sibling context already had, and it was missed. Fixed with
  the matching one-line `outlined_ctx.skip_alloca_hoisting = true;`.
  Verified: `examples/language/english/echo_loop.vani` (the example
  that surfaced this) now produces byte-identical output on both
  backends; new minimal regression example
  `examples/language/english/task_block_local_variable.vani`; new
  test `task_block_local_variable_example_produces_correct_output_on_both_backends`
  in `tests/run_end_to_end.rs`. Also fixed
  `echo_loop_windows_byte_count_matches_c` (a second, adjacent test
  that started failing once BUG-42 made it meaningful too) — its
  failure was NOT a real IOCP/async-semantics issue despite its own
  comment blaming one; the actual cause was simply comparing raw
  stdout byte lengths without CRLF normalization (C backend's stdout
  is in Windows CRT text mode, `\r\n` per line; LLVM's isn't, `\n`)
  — a 2-byte-per-line artifact. Fixed by normalizing before
  comparing, matching every other cross-backend stdout comparison in
  the same test file. Full `cargo test --workspace`: clean (only the
  known pre-existing Windows-local `ssa_backend_c_crosscheck.rs`
  link-flag gap).

- [x] **BUG-44. `#[no_mangle]` silently did nothing for any program
  simple enough to hit the SSA fast path (the common case) — on
  BOTH backends. Found auditing `intermediate/09d_build_systems.md`'s
  "Calling vāṇी functions from C" example, and confirmed the bug
  ALSO already affected the already-shipped
  `examples/language/english/bare_metal.vani` (its own comment
  claims `#[no_mangle]` "makes Reset_Handler appear with its
  literal name in the ELF" — it didn't; still emitted as
  `fn_Reset_Handler`).** Root cause: both `emit_c_via_ssa` and
  `emit_llvm_via_ssa` (`main.rs`) try a newer SSA-driven codegen
  path first and only fall back to the older tree-based backends
  (`CBackend`/`LlvmBackend`) for programs using a feature the SSA
  path doesn't cover yet. The bare-symbol-emission machinery for
  `#[no_mangle]` (`NO_MANGLE_FN_REGISTRY` in `backend_c.rs`,
  `LLVM_NO_MANGLE_FN_REGISTRY` in `backend_llvm.rs`) only exists in
  those older tree backends — `ssa_backend_c.rs`/
  `ssa_backend_llvm.rs` never got the equivalent, so any `#[no_mangle]
  fn` simple enough to qualify for the SSA fast path (which is most
  real code, including every tutorial example that tried it) kept
  its normal mangled `fn_<name>` symbol with no error or warning —
  silently breaking the entire point of the attribute (external C
  code — a linker script's reset vector, a hand-written `extern`
  declaration — expecting the bare name never found it). Fixed with
  a minimal, low-risk change: added `f.no_mangle` to
  `ssa_path_supports`'s per-function rejection list, so any program
  containing a `#[no_mangle]` function is routed to the tree backend
  wholesale (matching every other "SSA doesn't support this yet"
  entry already in that same gate) rather than attempting to
  reimplement the registry machinery inside both SSA emitters.
  Verified: both `bare_metal.vani` (now correctly emits
  `Reset_Handler`/`fn_Reset_Handler`-free on both backends) and a
  new minimal regression example now produce the documented bare
  symbol name; full `cargo test --workspace` clean. Also fixed the
  tutorial's own example, which had TWO further problems on top of
  the underlying bug: (1) it wrote `pub fn add(...)` for the
  `#[no_mangle]` function — `pub` requires being inside a `module {
  ... }` block (the same rule `08_manifest.md` already documents
  correctly) and doesn't parse bare; the correct, already-
  established pattern (confirmed against `bare_metal.vani`) is a
  plain top-level `fn`, no `pub` at all; (2) the doc's own
  parenthetical "remove vāṇी's `main` or rename it" wasn't backed by
  any actual mechanism — every vāṇī file requires a `fn main()`, and
  `vanic emit`'s C output always lowers it to a literal, unmangled
  `int main(void)` (unlike every other function), so linking
  `c_helper.c` (which has its own `main`) straight against the vāṇी
  object genuinely fails with "multiple definition of `main`",
  confirmed directly. Replaced the unactionable claim with a
  verified, working fix: `objcopy -N main` on the vāṇी object before
  the final link, plus the matching CMake `add_custom_command`.
  Confirmed end-to-end: without the `objcopy` step the link fails
  exactly as described; with it, the link and run both succeed and
  print `7`. New example:
  `examples/language/english/no_mangle_ssa_fastpath.vani`. New
  tests: `no_mangle_ssa_fastpath_example_produces_correct_output_on_both_backends`
  and `no_mangle_ssa_fastpath_emits_bare_symbol_on_both_backends` in
  `tests/run_end_to_end.rs`.

- [x] **`intermediate/10a_result_try_primer.md`'s central "naive
  way" example (the manual match-then-bail idiom for propagating a
  `Result` before `try`/`?` land) used a fundamentally invalid
  pattern — fixed 2026-07-30.** `let n: i64 = match parsed {
  Result.Ok(v) then v, Result.Err(e) then return Result.Err(e), };`
  doesn't parse: "expected expression" at `return`. Root cause,
  confirmed directly: a `let x = match { ... }` puts every arm in
  **expression** position (each arm must produce a value of the
  match's result type), and `return` is a statement, not an
  expression — it can't appear there. A bare *statement-form*
  `match` (no `let`, no `return match`) also doesn't exist in v1
  ("expected statement" — matches the same finding already logged
  for `02b_match_enhancements.md` earlier this session). The
  correct idiom, confirmed working end-to-end on both backends: `if
  let Result.Ok(v) = parsed { n = v; } else if let Result.Err(e) =
  parsed { return Result.Err(e); }` — `if let`/`else if let` are
  statements, which is exactly why they're the right tool for
  "extract a value, or bail out of the enclosing function." Also
  fixed the example's function name (`parse_int`) — it collides
  with one of vāṇी's own built-in names ("function 'parse_int' is a
  built-in name and cannot be redefined"), confirmed directly;
  renamed to `parse_num`. Also fixed a pre-existing `mdbook build`
  warning in the same file (bare `Option<T>` in an H2 heading,
  outside backticks).

- [x] **BUG-45 (found 2026-07-30, confirmed no longer reproducible 2026-08-01 — deep, sensitive language-feature
  machinery; high risk to touch under time pressure, matching the
  BUG-33/34/36 precedent). A function with a heap-owning parameter
  (`OwnedStr`, presumably `Vec<T>`/other affine types too) crashes
  (exit 116, both backends) the instant a `try`/`?` INSIDE that same
  function actually takes its early-return path — even when the
  heap-owning parameter is completely unused by the `try`/`?` call
  itself.** Found while building a corrected, working example for
  `intermediate/10b_runtime_errors_primer.md`'s `?`-propagation
  section (the file's ORIGINAL example used `Result<T,E>` with `?`,
  already known broken per the 10a entry above — while fixing it
  with the real, working `Option<T>` + `?` pattern, hit this SEPARATE
  crash). Bisected extensively — confirmed the trigger is exactly
  "function has an `OwnedStr`-typed parameter" AND "a `?`/`try` in
  that function's body propagates `None`" (both conditions
  required; either alone is fine):
  - `fn f(s: OwnedStr) -> Option<i64> { let n = try parse_int(s); return Some(n); }` — fine (single `try`, no second early-return path).
  - `fn f(x: i64) -> Option<i64> { let n = try lookup(x); let r = safe_div(100, n)?; ... }` (no `OwnedStr` anywhere) — fine, even though `safe_div`'s `?` DOES propagate `None`.
  - `fn f(s: OwnedStr, x: i64) -> Option<i64> { let a = try lookup(x); let b = try lookup2(x); ... }` — fine when NEITHER `try` ever hits `None` (only ever takes the `Some` path).
  - `fn f(s: OwnedStr, x: i64) -> Option<i64> { let a = try parse_int(s); let b = safe_div(100, x)?; ... }` called such that the SECOND `?` hits `None` — **crashes**, exit 116, both backends. The `OwnedStr` parameter `s` is never even read after the first `try` extracts its scalar — its mere PRESENCE as a parameter is enough to trigger the crash once any later early-return fires.
  Strongly suggests the `try`/`?` desugar's synthesized early-return
  path doesn't correctly emit the drop/cleanup sequence for
  affine (heap-owning) parameters that are still in scope at that
  point — likely a double-free or a skipped free racing against the
  function's normal-path epilogue, matching the exit-116 signature
  this session has repeatedly traced back to LLVM heap corruption
  (BUG-37, others). **Not fixed this session** — the `try`/`?`
  desugar (T2.6 Phase 2) is new, sensitive, partially-shipped
  machinery; a wrong fix risks silently breaking the cases that DO
  work today. Worked around in the tutorial fix by keeping
  `?`/`try`-using functions to scalar parameters only, extracting any
  `OwnedStr` input via `if let` *before* calling into them —
  documented as an explicit callout in
  `10b_runtime_errors_primer.md` so readers don't independently
  rediscover this the hard way. Whoever picks this up next: start
  by checking whether the early-return codegen site (wherever T2.6
  phase 2's desugar emits the synthetic "if payload-less variant,
  return it" branch) walks the function's live affine bindings for
  a drop sequence the way the normal `return` path does, and
  whether parameters specifically are included in that walk.
  **✅ Confirmed fixed 2026-08-01**, in the "fix documented TODO
  bugs" pass — re-ran every bisected repro shape from this entry
  directly against the real CLI (single try/OwnedStr, two-try where
  the second propagates None, both `try EXPR` and `EXPR?` syntax,
  OwnedStr param used vs. completely unused) and none crash
  anymore, on either backend, across repeated runs. Not chased down
  to which of the ~14 checker/backend commits between when this was
  logged (`d5eabe0`) and now fixed it as a side effect of other
  work (candidates worth checking first if it ever regresses:
  BUG-37's clone_at/drop-sequence fixes, or the `try`-desugar
  commits `1eed9ff`/`0a32b2f` in that range) -- what matters is the
  current behavior is correct, and it's now locked in with a real
  test (`owned_str_param_with_propagating_try_does_not_crash` in
  `tests/run_end_to_end.rs`, 5 runs per backend) so a future
  regression is caught immediately instead of silently
  reintroducing this. Commit `d36db5d`.

- [x] **`intermediate/10_result_try.md`'s entire premise was
  outdated: "There's no built-in `Result<T, E>` in v1 — you
  declare your own per-function-family enum" is false — fixed
  2026-07-30.** Confirmed directly: `Result<T, E>` (and
  `Result.Ok(...)`/`Result.Err(...)`) work perfectly with ZERO
  custom declaration, on both backends — a real, working generic
  built-in enum, same as `Option<T>`. The chapter had readers
  hand-declare `enum Result { Ok(i64), Err(i64) }` (a fixed,
  non-generic, `i64`-only shape) as if that were necessary; it
  isn't. This also directly contradicted a separate claim later in
  the SAME file ("v1's enums don't carry type parameters yet") —
  also false, confirmed both `Option<T>` and `Result<T, E>` are
  genuine generic enums throughout this entire audit. Rewrote the
  worked example, the `try`-rejection example, and the "v1
  limitations" list to use the real `Result<T, E>` generic
  directly, dropped the false "no generics" limitation bullet, and
  corrected the "Why it works" explanation. Cross-checked the
  file's OTHER claims, which held up: `try`/`?` really is rejected
  for `Result<T, E>` specifically (confirmed exact diagnostic
  text), and `let Result.Ok(v) = r;` (destructuring directly in a
  `let`, not `match`/`if let`) really doesn't parse ("expected
  '='"), confirmed directly. All examples verified end-to-end on
  both backends after the fix; `mdbook build` clean.

- [x] **BUG-46 (found, fixed 2026-08-01 — deep checker/monomorphization
  machinery, high risk to touch under time pressure). ANY
  constructor call (`EnumName.Variant(...)`) for a built-in generic
  enum (`Result<T, E>`, and confirmed ALSO `Option<T>`) breaks — on
  EVERY call site in the program, not just the "extra" ones — the
  instant a second, differently-parameterized instantiation of that
  SAME generic enum exists anywhere else in the program.** Found
  auditing `intermediate/10c_error_patterns_primer.md`'s Pattern 1
  (a function calling two sub-functions that each return a
  different `Result<i64, E>`, unified into one error enum — the
  single most natural real-world use of `Result<T, E>`, now that
  it's a genuine generic per the `10_result_try.md` fix above).
  Minimal repro: `fn f() -> Option<i64> { return Option.Some(1); }`
  alone compiles fine; add ANY sibling `fn g() -> Option<OwnedStr> {
  return Option.Some("x" + ""); }` to the SAME file and BOTH
  constructor calls fail with "unknown variable 'Option'" / "cannot
  call method 'Some' on i64" (a parser/checker confusion, not a
  real name-resolution issue — the diagnostics are clearly
  downstream noise from whatever actually breaks). Same for
  `Result<i64, IoError>` + `Result<i64, ParseError>` in one file:
  ALL FOUR of `Ok`/`Err` construction sites across both functions
  fail, plus a third, later, differently-parameterized `Result<i64,
  ConfigError>` construction ALSO fails — i.e. this isn't "the
  first one wins" or "the last one wins," every constructor call
  for the affected generic breaks once 2+ instantiations coexist.
  **Crucially, only CONSTRUCTION breaks — PATTERN matching
  (`match`/`if let` destructuring an already-`let`-typed value)
  is unaffected**, confirmed directly: `main()`'s `if let
  Result.Ok(v) = outcome` (where `outcome`'s type is already known
  from its own `let` annotation) works fine throughout every repro
  above, even with 2+ instantiations present. This strongly
  suggests the break is in whatever resolves a bare
  `EnumName.Variant(payload)` CONSTRUCTOR expression's concrete
  monomorphization from context (return-type inference, `let`
  target-type inference) when multiple candidate instantiations of
  the same generic enum are registered — as opposed to pattern
  matching, which already has a concrete type to check against and
  never needs that inference step. **Not fixed this session** —
  this is core generic-enum monomorphization/constructor-resolution
  machinery; a wrong fix risks silently breaking the (already
  fragile, freshly-corrected) single-instantiation case that DOES
  work today. **Workaround, verified working**: when a program
  needs 2+ DIFFERENT instantiations of the same builtin generic
  enum with constructor calls at more than one of them, use
  hand-declared custom enums (`enum IoResult { Ok(i64), Err(i64) }`,
  etc.) for all but one of the instantiations instead — reserve the
  generic `Result<T, E>`/`Option<T>` for the single "outermost"
  unified error boundary. Worked around this way in the
  `10c_error_patterns_primer.md` fix below. Whoever picks this up
  next: the "unknown variable 'Result'"/"cannot call method on i64"
  diagnostic shape (treating `Result`/`Option` as if the identifier
  itself failed to resolve, then treating the constructed value as
  a bare `i64` fallback) is a strong hint about where in
  `checker.rs` to start looking — probably a per-generic-enum
  "resolve pending constructor calls once we know the concrete
  instantiation" pass that keys off name alone instead of a
  (name, call-site) pair, so a second registration for the same
  name clobbers the first's resolution state.
  ✅ Fixed 2026-08-01, in the "fix documented TODO bugs" pass.
  The real root cause was narrower and safer to fix than the
  above guess suggested: `resolve_enum_name`'s "exactly one
  candidate in the whole program" heuristic (a deliberate,
  documented v1 restriction, closure #281) is fine on its own —
  the actual gap is that construction never used the CONTEXT
  already available at a `return`/`let`-with-annotation site (the
  enclosing function's own return type, or the let's own
  annotation) to disambiguate, even though that context
  unambiguously names the concrete instantiation. Fixed with a new
  `resolve_bare_enum_ctors_in_stmt` step in
  `monomorphize_type_decls_in_program` that rewrites a bare
  `EnumName.Variant(...)` receiver to the already-monomorphized
  target name at exactly those two positions (return, let with
  annotation) — deliberately narrow, so anything else (e.g. a
  constructor passed directly as a function-call argument) keeps
  the exact pre-existing behavior, confirmed by testing. New
  tests: 3 checker-level (`src/lib.rs`) plus a real end-to-end
  test (`tests/run_end_to_end.rs`) asserting correct runtime
  values on both backends, not just successful compilation. Full
  `cargo test --release --workspace`: 13/13 test binaries clean,
  0 failed. Commit `fec2e1c`.
  **Follow-up gap found+fixed same day (commit `ae33c9c`)**, while
  updating `intermediate/10c_error_patterns_primer.md`'s own
  worked example to use the real generic instead of its
  hand-declared-enum workaround: the fix above only recursed into
  `If`/`While`/`For` bodies, not `IfLet`/`WhileLet` -- so a
  `return EnumName.Variant(...);` inside an `if let ... else if
  let ...` chain (the single most common place a union error type
  actually gets constructed) was still unresolved. Also found the
  payload-less-variant shape (`Option.None`, which parses as
  `FieldAccess` not `MethodCall`) was never handled at all, in any
  position. Both fixed together, with matching new tests.

- [x] **BUG-47 (found, fixed 2026-08-01 — C-backend-only type-emission bug).
  A function parameter typed `ref HashMap<OwnedStr, V>` gets the
  WRONG hardcoded C struct type in its emitted C signature
  (`intent_hashmap_<K2>_<V2>` from some OTHER HashMap instantiation
  used elsewhere in the same program) instead of the correct
  `intent_hashmap_owned_str_<V>`, causing real `cc` compile errors.
  LLVM backend is unaffected. Found auditing
  `intermediate/10c_error_patterns_primer.md`'s Pattern 4.**
  Minimal repro:
  ```vani
  fn lookup(map: ref HashMap<OwnedStr, i64>, key: OwnedStr) -> Option<i64> {
    return hashmap_get(map, key);
  }
  fn main() -> i64 {
    let m: HashMap<OwnedStr, i64> = hashmap_new();
    let _ = hashmap_insert(mut ref m, "a" + "", 1);
    let r: Option<i64> = lookup(ref m, "a" + "");
    if let Option.Some(v) = r { print v; }
    return 0;
  }
  ```
  Runs correctly (`1`) on the default LLVM backend. Under
  `--backend=c`, `cc` fails: `fn_lookup`'s emitted signature uses
  `const intent_hashmap_i64_i64* v_map` — a type name that isn't
  even declared anywhere in the emitted file, since no
  `HashMap<i64, i64>` exists in this program at all — while the
  function body correctly calls
  `intent_hashmap_owned_str_int64_t_get(v_map, ...)`, and the
  `main()` call site correctly passes an
  `intent_hashmap_owned_str_int64_t*`. So the parameter's DECLARED
  type and its ACTUAL use are inconsistent within the same
  generated function — strongly suggests the C backend's function
  SIGNATURE emission for a `ref HashMap<K,V>` parameter is resolving
  the monomorphized struct name from some global/first-seen-instantiation
  state rather than from the parameter's own declared type, while
  the body and call sites correctly use the parameter's real type.
  Confirmed specific to HashMap-as-parameter — the same
  `HashMap<OwnedStr, i64>` type used entirely within one function
  (no parameter passing) works fine on both backends.
  **✅ Fixed 2026-08-01**, in the "fix documented TODO bugs" pass.
  Root cause matched this entry's own hint exactly, but in
  `backend_c.rs`'s `format_declarator` (not `ssa_backend_c.rs`):
  its bare/`Ref`/`RefMut` match arms had every one of the
  BUG-22-class parametric types (`Mutex`, `Guard`, `RwLock`,
  `ReadGuard`, `WriteGuard`) explicitly handled, but no
  `Type::HashMap` arm at all in any of the three — so it fell
  through to `c_leaf_type`'s hardcoded `intent_hashmap_i64_i64`
  fallback, exactly the same missing-arm shape BUG-22 fixed for
  the other five types (HashMap just wasn't parametric yet when
  that fix landed). `c_type_name` (used for `let`/return-type
  spelling) already had the correct `hashmap_prefix_from_kv`-based
  arm, which is why the body/call sites were always right and only
  the parameter declarator was wrong. Fixed by adding the same
  `Type::HashMap(k, v) => hashmap_prefix_from_kv(k, v)`-based arm
  to all three of `format_declarator`'s matches (bare, `Ref`,
  `RefMut`) — also confirmed the bug applied to bare-by-value and
  `mut ref` parameters too, not just `ref`, none of which the
  original report tested. New tests: a checker-level string-content
  test (`src/lib.rs`) and a real end-to-end `cc`-invoking test
  (`tests/run_end_to_end.rs`) reproducing the exact cross-
  contamination trigger (a second `HashMap<i64, i64>` instantiation
  alongside the `HashMap<OwnedStr, i64>` parameter). Full
  `cargo test --release --workspace`: 2620 lib tests + 98
  end-to-end tests, 0 failed. Commit `ccbf771`.

---

## Async/await compiler crash found resuming the advanced tutorial-track audit (added+fixed 2026-07-31)

- [x] **BUG-48. Every `await(...)` call inside an `async fn` crashed the
  compiler (`vanic check`/`run`, both v3.1 state-machine paths) with a
  native stack overflow (`fatal runtime error: stack overflow`, abort,
  exit 134) — 100% reproducible, not input-dependent.** Found bisecting
  the crash recorded mid-investigation in the previous session's
  handoff notes while resuming the `tutorials/src/advanced/`
  audit.** ✅ fixed 2026-07-31.
  Minimal repro (previously crashed; now a clean diagnostic — see BUG-49):
  ```vani
  async fn handler1(fd: i64) -> i64 {
    let req: i64 = await(io_recv_async(fd, 64));
    return req;
  }
  fn main() -> i64 { return 0; }
  ```
  **Root cause** (confirmed with `gdb -batch -ex run -ex "thread apply
  all bt -50"`, showing `vani::parser::anf_lift_body` recursing into
  itself ~8860+ times with identical frame shapes): `await(inner)`
  parser-desugars to `match inner { Future.Ready(v) then v,
  Future.Pending then 0 }` (`synthesize_await_desugar`, `parser.rs`).
  Because `Future.Ready`/`Future.Pending` are Variant-shaped patterns,
  `try_desugar_let_match_with_suspends` routed this into
  `try_desugar_match_via_tag_extraction` (the Phase 2.3c/2.3d
  machinery, working as designed for its actual tested use case — see
  the `v31_phase23c_*`/`v31_phase23d_*` tests in `lib.rs`, all of
  which scrutinize a plain enum-returning helper fn with the suspend
  in an ARM BODY). Tag-extraction synthesizes `let __match_tag_X: i64
  = match SCRUT { <same Variant patterns>, ... };` — but for
  `await()`'s shape, `SCRUT` (`inner`, the `io_*_async` call) is
  *itself* the suspend, and the arm bodies are trivial (`v` / `0`), so
  the synthesized statement is structurally indistinguishable from the
  input that triggered tag-extraction in the first place. When
  `anf_lift_body` recursively re-processes its own synthesized output
  (`parser.rs` ~line 7588), the identical transform fires again,
  forever — this exact shape had never been exercised by the Phase
  2.3c/2.3d test suite, which only ever scrutinizes a separate,
  non-suspending helper-fn call, never the suspending call itself.
  **Fix**: `try_desugar_let_match_with_suspends` now only routes to
  tag-extraction when an ARM BODY (not just the scrutinee) contains a
  suspend (`arms_suspend` check, `parser.rs`) — i.e. only when
  per-arm state-splitting is actually needed, which is the sole case
  Phase 2.3c/2.3d ever implemented or tested. Scrutinee-only suspends
  (the `await()` shape) now fall through to the ALREADY-EXISTING
  `validate_v31_linear_body` diagnostic ("unsupported pattern shape"),
  converting a guaranteed compiler abort into an honest compile error
  — zero risk of newly-wrong codegen, since this input never compiled
  correctly before either way. Regression test:
  `bug48_await_scrutinee_only_suspend_does_not_crash_compiler`
  (`lib.rs`) — this test alone would previously have aborted the
  entire `cargo test` process. Full `cargo test --release --workspace`
  reverified clean (2599 lib tests + every integration-test binary, 0
  failed) after the fix.

- [x] **BUG-49 (found fixing BUG-48, fixed 2026-07-31 same day —
  `await()` now actually compiles and runs, not just fails cleanly).**
  BUG-48's fix made the compiler reject `await()`'s scrutinee-only-
  suspend shape gracefully; it didn't make it compile. Fixed by adding
  a dedicated case at the top of `try_desugar_let_match_with_suspends`
  (`parser.rs`) that recognizes the exact `synthesize_await_desugar`
  output shape end-to-end — scrutinee is directly an `io_recv_async`/
  `io_send_async`/`io_accept_async` call, arms are exactly
  `Future.Ready(v) then v` / `Future.Pending then 0` (guards none,
  arm bodies structurally verified, not just pattern-shape-checked) —
  and rewrites the whole `let X = await(io_*_async(..));` straight to
  `let X = io_*_async(..);` *before* the `has_variant` tag-extraction
  routing ever sees it. This sidesteps the earlier failed approach
  (hoisting the scrutinee via `anf_lift_expr`'s `Match` case, which
  stamped the hoisted local `Type::I64` per the generic `__anf_N`
  convention and lost the scrutinee's real shape) entirely — no hoist
  needed, since `io_*_async` builtins already check/codegen as a
  plain scalar suspend value identical to the pre-existing direct-let
  form (`check_epoll_builtin` in `checker.rs`; the alias rewrite to
  the nb variant at codegen). The `Future.Ready`/`Future.Pending`
  match was never semantically meaningful for this shape in the first
  place — v1's synchronous async desugar never actually boxes the
  suspend value in a `Future` at runtime — so eliminating the match
  outright is correct, not a workaround.
  Regression tests: `bug49_await_scrutinee_only_suspend_compiles_and_
  matches_direct_form` (`lib.rs`) checks the `Future`/`__await_v`
  match is structurally gone and the state-machine shape (suspend-state
  count, `req` reference count) matches the direct-suspend form
  1:1 — exact string equality against the direct form does NOT hold,
  since `await(..)`'s extra source characters shift every downstream
  span and this compiler's temp/return names are span-derived
  (`__intent_ret_63` etc.), so naively asserting byte-identical codegen
  is a false lead (tried, reverted after the assertion itself failed
  on span-derived names, not on any real structural difference).
  `bug49_await_builtin_example_compiles_and_runs_on_both_backends`
  (`tests/run_end_to_end.rs`) is the real end-to-end proof: a new
  example, `examples/language/english/bug49_await_builtin.vani`
  (mirrors `tcp_echo_async.vani`'s driver-loop pattern but wraps every
  suspend point in `await(..)`), does a real TCP round-trip through
  the compiler-synthesized Task/poll-fn state machine and asserts the
  awaited value is byte-correct (`"echoed bytes: 7"` for a real
  7-byte payload) on both backends — a compile-only check can't catch
  a wrong-value regression here, only actual execution can. Full
  `cargo test --release --workspace` clean (2601 passed, 0 failed, 1
  ignored) after this fix.
  **Still open, independent of this fix**: `01a_async_primer.md`'s
  tutorial text uses a bare-statement `await(io_send_async(fd,
  resp));` (no `let`), which still doesn't parse ("expected
  statement") — bare-statement-position `await()` was never
  implemented (only `let X = await(..);` is). Needs a doc fix (rewrite
  to `let _ = await(..);`) or a real bare-statement-await parser
  feature whenever the tutorial audit reaches this file.

## LLVM backend bug found continuing the advanced tutorial-track audit (added+fixed 2026-08-01)

- [x] **BUG-50. `barrier_wait`'s "last thread" wake path crashed the
  LLVM backend on `lli` with "floating point constant invalid for
  type" — 100% reproducible on every program that ever reaches the
  last-arriving thread's branch, i.e. every real use of `Barrier`
  (some thread is always last).** Found auditing
  `tutorials/src/advanced/02b_barrier_primer.md`'s worked example
  against the real compiler (LLVM backend; the C backend ran fine).
  Root cause: `backend_llvm.rs`'s barrier-wake codegen (in the
  `barrier_wait` match arm, the `else` branch for non-Win32 hosts)
  spliced the literal text `i32 0x7fffffff` directly into the
  generated `@syscall(...)` call's FUTEX_WAKE count argument — LLVM's
  textual IR only accepts hex literals for floating-point constants
  (with an exact required digit count per type); plain integer
  constants must be decimal. `lli` rejected the IR outright. The
  adjacent `condvar_notify_all` codegen a couple hundred lines up
  handles the identical "wake up to `INT_MAX` waiters" value
  correctly by interpolating a Rust `i64` (`{}`, which `Display`s as
  decimal) instead of splicing hex text into the format string
  literally — `barrier_wait`'s codegen just didn't follow that
  pattern. **Fixed**: replaced the literal `0x7fffffff` in the format
  string with the decimal `2147483647`. Grepped the rest of
  `backend_llvm.rs` for the same "raw hex text baked into an emitted
  IR string" shape — no other occurrence found; this was an isolated
  bug, not the systemic "parallel dispatch function" class this
  project has hit many times before.
  Regression tests: `barrier_wait_llvm_wake_uses_decimal_not_hex_literal`
  (`src/lib.rs`) checks the generated LLVM IR text directly (a
  `barrier_new(1)` program forces the only `barrier_wait` call to
  take the last-thread/wake-all branch, so no multi-threading is
  needed to reach the buggy code path) — asserts no `0x7fffffff`
  substring and the correct decimal literal is present.
  `barrier_two_threads_rendezvous_correctly_on_both_backends`
  (`tests/run_end_to_end.rs`) is the real proof: two real OS threads
  (one via `task fn(args)` expression-form spawn, one the main
  thread) race to a 2-participant barrier and each reports whether it
  was the last to arrive; asserts EXACTLY one of them saw `true` —
  this is the kind of thing a compile-only check can't catch (a
  logic bug could make both or neither see `true` while still
  producing syntactically valid, `lli`-acceptable IR). Full `cargo
  test --release --workspace` clean (2602 passed, 0 failed) after
  this fix.
  Also found and fixed in the same doc during this audit (docs-only,
  no compiler bugs): the worked example's `phase_one(id, b: mut ref
  Barrier)` called `barrier_wait(mut ref b)` — double-ref'ing a
  parameter that is already `mut ref Barrier`, the same double-ref
  mistake class as a prior session's `barrier_wait`/`stage_one` bug
  (see [[project_vani_compiler_status]]'s BUG-21 Path B section) —
  fixed to `barrier_wait(b)`. Also fixed doc bugs in
  `02a_parallelism_primer.md` (missing `;` after `reduce sum with +`;
  undocumented `pure fn` requirement for `parallel for` body calls;
  the `task`/`join` example wrapped I/O in an ordinary non-`pure`
  helper function, which is unconditionally rejected — `task { ... }`
  block bodies enforce the same purity rules as `parallel for`
  bodies, with only DIRECT calls to builtin I/O primitives exempted,
  confirmed by testing) and `01_async.md` (the `select { await ... }`
  example called a bare `async fn`, which returns `Future<T>` and is
  rejected — `select` polls a raw i64-returning nb-style call
  directly, unrelated to `Future`/`await()` despite the shared
  keyword; also a false claim that `try` works inside `async fn`
  bodies, confirmed rejected unconditionally since `async fn`'s
  return type desugars to `Future<R>` before `try`'s checker ever
  sees it) and `01a_async_primer.md` (bare-statement `await(...)`,
  which never parses, plus a call to an undefined `process` fn).

- [x] **BUG-51. Any program with two DIFFERENT functions each
  containing their own `parallel for` (or block-form `task { … }`, or
  expression-form `task fn(args)` spawn) crashed the LLVM backend with
  "invalid redefinition of function" — 100% reproducible, not
  input-dependent.** Found auditing `tutorials/src/advanced/
  02_parallel.md`'s `double_all`/`dot_product` pair (side by side in
  one file, each with its own `parallel for`) against the real
  compiler; the C backend was unaffected. Root cause:
  `backend_llvm.rs`'s `FnCtx.next_outline` counter (used by all three
  outlining sites — `emit_parallel_for_via_gomp`,
  `emit_task_via_pthread`, `emit_task_spawn_call` — to generate
  `@__intent_par_<N>` / `@intent_task_<N>` / `@intent_task_call_<N>`
  symbol names) is scoped to the CURRENT PARENT function only; it
  restarts at 0 for every new top-level function's `FnCtx`. LLVM
  symbol names are module-global, so two functions each with exactly
  one outlined construct both generated the identical `..._0` name —
  the compiler tried to `define` the same function twice. **Fixed**:
  added a new `FnCtx.outline_prefix` field, set to the enclosing
  top-level function's own name right after that `FnCtx` is
  constructed (and copied into the two nested outlined-fn `FnCtx`s so
  recursively-nested outlines stay qualified by the ORIGINAL top-level
  function rather than losing the qualifier), and spliced into all
  three symbol-name `format!` call sites
  (`__intent_par_<parent_fn>_<id>` etc.) — function names are already
  unique in a vāṇी program, so this guarantees global uniqueness
  without a new module-level counter. C backend untouched (its own
  naming, in `backend_c.rs`, wasn't affected by this bug — separate
  code, separate counter).
  Regression tests:
  `two_functions_each_with_parallel_for_get_distinct_llvm_outline_names`
  (`src/lib.rs`) asserts the LLVM IR defines exactly 2 outlined
  functions with DISTINCT names, not one followed by a silent
  duplicate. `two_functions_each_with_parallel_for_run_correctly_on_
  both_backends` (`tests/run_end_to_end.rs`) is the real proof: two
  functions, each multiplying every element of the SAME captured
  `Vec` by a different constant, called back to back — a compile-only
  check can't catch a wrong-value regression (e.g. the second
  outlined fn accidentally reusing the first's captures) the way an
  actual `lli`-executed run with real, distinguishable expected output
  (`xs = 6 12 18 24`) can. Updated one pre-existing test
  (`task_spawn_lowers_to_pthread_create_with_outlined_body`) whose
  LLVM assertion hardcoded the old unqualified `@intent_task_0` name
  to expect `@intent_task_main_0` instead (its `main`-only source
  program was never wrong, just written before this fix existed).
  Full `cargo test --release --workspace` clean (2603 passed, 0
  failed) after this fix.

- [x] **BUG-52. Any program calling `condvar_wait`/`condvar_wait_timeout`
  (not just `condvar_notify_one`/`condvar_notify_all`) failed a REAL
  `cc` compile on the C backend — "unknown type name
  'intent_guard_i64'; did you mean 'intent_guard_int64_t'".** Found
  auditing `tutorials/src/advanced/03b_condvar_primer.md`'s "Pattern"
  and `wait_timeout` sections against the real compiler. Root cause:
  `emit_intent_condvar_helpers_c`'s hardcoded C text for
  `intent_condvar_wait`/`intent_condvar_wait_timeout` referenced the
  STALE names `intent_guard_i64`/`intent_mutex_i64_lock`/
  `intent_guard_i64_unlock` — names that only exist via a separate
  "legacy alias" typedef block (`emit_intent_mutex_helpers_c`, whose
  own doc comment literally says "Used by the condvar helper (which
  still references intent_guard_i64)") that is called from
  `ssa_backend_c.rs` but NOT from the tree-C driver that actually
  calls `emit_intent_condvar_helpers_c` — so the alias was never
  actually in scope where it was needed. The pre-existing test
  (`condvar_emits_runtime_helpers_in_c`) only exercised
  `condvar_notify_all`, never `condvar_wait`/`condvar_wait_timeout`
  themselves, so this never got caught. **Fixed**: rewrote the
  hardcoded text to reference the REAL names `emit_mutex_bundle`
  generates for `Mutex<i64>`/`Guard<i64>` (`intent_guard_int64_t`,
  `intent_mutex_int64_t_lock`, `intent_guard_int64_t_unlock`) directly,
  removing the dependency on the legacy alias path entirely.
  Also confirmed (not a bug, but a real trap in the doc's own
  examples): `mutex_get`/`mutex_set`/`mutex_unlock` don't exist —
  Mutex's real API is `guard_get`/`guard_set` + RAII scope-exit
  (matching the pattern `03_concurrency.md`'s own Mutex section
  already documents correctly) — and `condvar_signal_one`/
  `condvar_signal_all` don't exist either (real names:
  `condvar_notify_one`/`condvar_notify_all`, same BUG-51-adjacent typo
  class already fixed in `03_concurrency.md` this same session). Fixed
  all three in `03b_condvar_primer.md`'s two worked snippets, plus
  scoped each `Guard` in its own block so it drops before the next
  acquisition (the doc's un-scoped version would leave two guards
  live on the same conceptual "thread" at once — the exact deadlock
  hazard `02c_rwlock_primer.md` already warns about for RwLock).
  Regression tests:
  `condvar_wait_uses_real_guard_type_name_not_stale_i64_alias`
  (`src/lib.rs`) pins the emitted C text. The real proof is
  `condvar_wait_and_wait_timeout_compile_and_run_with_real_cc`
  (`tests/run_end_to_end.rs`) — a substring check on emitted C text
  can prove the identifier got renamed but can't prove `cc` actually
  accepts the result; this test runs `vanic run --backend=c` end to
  end through a real `cc` invocation. Full `cargo test --release
  --workspace` clean (2604 passed, 0 failed) after this fix.

## Two more bugs found auditing 04b_cross_compile_primer.md (added+fixed 2026-08-01)

- [x] **BUG-53. `mmio_read_u8`/`mmio_read_u16`/`mmio_write_u8`/
  `mmio_write_u16` crashed `lli` ("use of undefined value") or failed
  a real `cc` compile ("implicit declaration of function") on ANY
  program calling them — 100% reproducible.** `mmio_read_u32`/
  `mmio_write_u32` were unaffected. Found auditing the MMIO builtins
  table in `04b_cross_compile_primer.md` against the real compiler.
  Root cause: these four builtins were implemented in the legacy
  tree-LLVM/tree-C backends (`backend_llvm.rs`/`backend_c.rs`) but
  never ported to the SSA fast path (`ssa_backend_llvm.rs`/
  `ssa_backend_c.rs`) that compiles by default — `mmio_read_u32`/
  `mmio_write_u32` WERE already correctly ported, so this was
  specific to the narrower widths. Unlike `#[no_mangle]` (BUG-44),
  nothing routed these four names to the tree-backend fallback, so
  they stayed on the unimplemented SSA path and fell through to
  ordinary (nonexistent) function-call codegen. **Fixed**: ported
  all four directly into both SSA backends, mirroring the exact
  pattern the already-correct `mmio_read_u32`/`mmio_write_u32` SSA
  implementations use (just narrower — i8/align 1, i16/align 2
  instead of i32/align 4). The pre-existing `mmio_read_u8_compiles_c`
  test only asserted `c.contains("uint8_t")`, which is present in
  every C file's boilerplate typedefs regardless of whether
  `mmio_read_u8` itself codegens correctly — too weak to have caught
  this; added 6 new tests asserting the actual volatile-access
  codegen (`load volatile i8`/`store volatile i16`/
  `volatile uint8_t*` etc.), mirroring the u32 tests' existing rigor.
- [x] **BUG-54. Printing an unsigned narrow type (`u8`/`u16`) whose
  high bit was set produced a NEGATIVE number on the LLVM backend
  while the C backend printed the correct value for the byte-identical
  program — a real backend-parity break, not just wrong in
  isolation.** Found immediately after fixing BUG-53, while building a
  real test program to exercise the mmio fix (`let a: u8 = 200; let b:
  u8 = 50; print a + b;` printed `-6` on LLVM, `250` on C). Root
  cause: `ssa_backend_llvm.rs`'s `print`-argument widening
  (`intent_print_item`'s generic integer arm) ALWAYS used `sext`
  (sign-extend) to widen a sub-i64 integer to i64 before printing,
  regardless of the value's actual signedness — `250` (200+50) as a
  signed i8 bit pattern is `-6`, so `sext i8 -6 to i64` correctly
  reproduces the BIT PATTERN but the WRONG mathematical value for an
  unsigned source type. The
  legacy tree-LLVM backend already had this right
  (`ty.is_unsigned_integer()` dispatches to a zext path via
  `widen_int_to_64(..., false)`); only the SSA fast path (the
  default) had the bug. **Fixed**: choose `zext` vs `sext` based on
  `is_signed_int(&aty)` (already defined in the same file for an
  unrelated reduction-widening use), matching the tree backend's
  existing logic exactly. Regression tests:
  `print_unsigned_narrow_int_zero_extends_not_sign_extends`
  (`src/lib.rs`) pins the emitted instruction.
  `unsigned_narrow_int_prints_same_value_on_both_backends`
  (`tests/run_end_to_end.rs`) is the real proof — a compile-only
  IR-text check can confirm the instruction changed but can't prove
  the two backends now agree on the actual printed value the way a
  real side-by-side execution comparison does. Full `cargo test
  --release --workspace` clean (2611 passed, 0 failed) after both
  fixes.

## Two more bugs found auditing 05_simd.md (added+fixed 2026-08-01)

- [x] **BUG-55. `vec_fill`/`vec_with_capacity` failed a real `cc`
  compile on `Vec<i64>` (and every other element type) — "unknown
  type name 'intent_vec_i64'; did you mean 'intent_vec_int64_t'".**
  Found running the chapter's own first example (`vec_fill(8, 1 as
  i64)`) through the real compiler. Root cause: both builtins'
  tree-C codegen (`backend_c.rs`) computed their Vec bundle's C
  struct name via `crate::backend_llvm::vec_struct_tag` — an
  **LLVM-backend** naming helper, reused by mistake inside C codegen
  — instead of the C backend's own `vec_c_struct`/`element_tag`
  (already used correctly at ~15 other call sites in the same file).
  For `i64` this produced the stale name `intent_vec_i64`, not the
  real `intent_vec_int64_t` that `emit_vec_bundle_typedef` actually
  emits. **Fixed**: both call sites now use `vec_c_struct(element)`,
  matching the file's own established convention. No prior test
  existed for either builtin at all — that's how this went
  uncaught. Added unit tests asserting the correct struct name
  appears (and the stale one doesn't) in the emitted C.
- [x] **BUG-56. `vec_with_capacity` failed a real `cc` compile via
  the actual CLI's default `--backend=c` path — "implicit
  declaration of function 'fn_vec_with_capacity'" — even after
  BUG-55 was fixed.** Root cause: `vec_with_capacity` is implemented
  in SSA-LLVM (`ssa_backend_llvm.rs`) but was never ported to SSA-C,
  and nothing routed it to the tree-C fallback the way `vec_fill`
  already was (see `main.rs`'s `expr_ssa_supported`, which explicitly
  rejects `vec_fill` for exactly this reason) — so SSA-C silently fell
  through to an ordinary (nonexistent) function call. Same root-cause
  shape as BUG-44/BUG-53 (a builtin ported to one backend's SSA path
  but not the other's, with nothing gating the gap). Since
  `vec_with_capacity` genuinely DOES work on SSA-LLVM, adding it to
  the *shared* `expr_ssa_supported` rejection list (like `vec_fill`)
  would have needlessly forced LLVM back to the tree path too —
  instead added a new `stmt_calls_vec_with_capacity`/
  `expr_calls_vec_with_capacity` pair (mirroring the existing
  `stmt_calls_file_line_read` pattern exactly) wired into
  `ssa_c_extra_reject` only, so just the C side falls back. **Caught
  a testing-methodology gap while writing the regression test**: the
  library's `compile_to_c` helper always calls the tree-C backend
  directly and can NEVER exercise this SSA-routing bug — only a real
  CLI invocation (which goes through `main.rs`'s SSA-first dispatch)
  can. Added `vec_with_capacity_compiles_and_runs_on_both_backends`
  (`tests/run_end_to_end.rs`) for that; the `src/lib.rs` unit tests
  cover BUG-55's struct-naming half only. Full `cargo test --release
  --workspace` clean (2613 passed, 0 failed) after both fixes.

## Three more bugs found continuing the 05_simd.md audit (added+fixed 2026-08-01)

- [x] **BUG-57. `let v: vec128<f32> = ...;` (and `vec256`/`vec512`)
  failed a real `cc` compile — "'v_v' undeclared" — on every local
  variable of these types, even though the `simd_*`/`simd256_*`/
  `simd512_*` builtin CALLS themselves already emitted correct C.**
  Found running the chapter's own SAXPY example. Root cause: same
  missing-arm shape as the BUG-22 fix — `c_type_name` (the function
  used for `let`-binding storage types) had no explicit arm for
  `Type::Vec128`/`Vec256`/`Vec512`, so it fell through to
  `c_leaf_type`'s per-T-unaware placeholder COMMENT (`/* vec128<T>
  */`) instead of the real GNU vector-extension type. The correct
  helpers (`c_vec128_type`/`c_vec256_type`/`c_vec512_type`) already
  existed and were already used correctly by the simd-builtin call
  sites — `c_type_name` just never routed through them. Fixed by
  adding the three explicit arms.
- [x] **BUG-58. `simd_store`/`simd256_store`/`simd512_store` caused a
  double-free the instant their (conventionally discarded) return
  value and the Vec they wrote through were both still live — on
  BOTH backends.** These three builtins mutate the target `Vec<T>`
  THROUGH its ref/pointer and return a byval copy of the struct
  header (the SAME `.data` buffer pointer) purely so the call can be
  chained — never a fresh allocation. Both backends' generic "free a
  discarded `Vec<T>`-returning call's result" codegen assumed every
  such discard owns a new buffer, so `let _ = simd_store(y, i, v);`
  freed `y`'s buffer immediately; the caller's own later scope-exit
  drop of `y` then freed the identical pointer again (glibc abort:
  "double free detected in tcache"). Fixed by special-casing these
  three builtin names in each backend's `Discard`-statement handling
  to just evaluate the call for its side effect, never free the
  result.
- [x] **BUG-59. `vec256<T>`/`vec512<T>` load/store crashed `lli`
  NON-DETERMINISTICALLY — the identical program, re-run several
  times with no code change, intermittently aborted.** Found
  stress-testing `dot256`/`dot512` by hand after BUG-57/58 stopped
  masking it (multiple back-to-back runs of the same binary
  sometimes succeeded, sometimes crashed). Root cause:
  `simd256_load`/`store` declared `align 32` and `simd512_load`/
  `store` declared `align 64` in the emitted LLVM IR, asserting the
  vector types' NATURAL alignment — but the underlying buffer always
  comes from a plain `malloc`, and glibc's malloc on x86-64 only
  guarantees 16-byte alignment, never 32 or 64. Declaring a stronger
  alignment than the pointer actually has is undefined behavior LLVM
  is free to exploit differently depending on the buffer's actual
  runtime address, which explains the non-determinism precisely (the
  128-bit `vec128<T>` path was already correct at `align 16`, which
  matches malloc's real guarantee exactly — only the wider two
  widths overclaimed). Fixed by changing all four sites (256/512 ×
  load/store) to `align 16`, matching what the allocator actually
  provides; LLVM emits the always-correct unaligned-safe instruction
  (`vmovups` instead of `vmovaps` etc.) regardless of the runtime
  pointer's actual alignment — this only forgoes an optimization,
  never correctness. The C backend was never affected (its
  `__attribute__((vector_size(N)))` GNU extension has no separate
  alignment annotation to overclaim).
  Regression tests: `vec128_let_binding_uses_real_vector_type_not_
  placeholder_comment` and `simd_store_discard_does_not_double_free_
  in_c` (`src/lib.rs`) cover BUG-57/58 via compile-only IR-text
  assertions. The real proof is in `tests/run_end_to_end.rs`:
  `saxpy_f32_example_runs_without_double_free_on_both_backends` (a
  compile-only check can't catch a runtime double-free) and
  `vec256_dot_product_runs_consistently_without_alignment_crash`,
  which runs the SAME binary 12 times in a loop — a single passing
  run proves nothing for a bug that's non-deterministic by nature.
  Also fixed one more doc-only bug in the same file's combined-
  layers example: `out: ref Vec<f32>` needed to be `mut ref` (the
  scalar tail loop index-assigns into it directly, which a plain
  `ref` rejects — only `simd_store`'s own aliasing write tolerates a
  plain `ref`). Full `cargo test --release --workspace` clean (2615
  passed, 0 failed) after all three fixes.

## Bug found auditing 06_smt_debug.md (added+fixed 2026-08-01)

- [x] **BUG-60. The compiler's own "proof failed" diagnostic hint told
  users to set `INTENT_TRACE_SMT=1` for a full SMT trace — an env var
  that is never actually checked anywhere in the source, confirmed by
  testing (setting it produces byte-identical output to setting
  nothing at all).** Found cross-checking `06_smt_debug.md`'s
  documented `VANIC_SMT_DEBUG=1` env var against the compiler's own
  in-diagnostic hint text — they didn't match, and only one of the two
  actually does anything (`smt.rs::smt_debug_enabled` checks
  `VANIC_SMT_DEBUG` plus the legacy alias `INTENTC_SMT_DEBUG`; the
  tutorial doc already had the right name). Fixed the two hint sites
  in `diagnostic_elaborations.rs` to say `VANIC_SMT_DEBUG=1`. Also
  confirmed (not a bug, a real and useful distinction the doc gets
  right): `prove` hard-rejects the build on an unprovable predicate
  (exit 1) despite its own message text saying "NOT a build failure"
  (that text describes the underlying runtime-check fallback
  mechanism, not the CLI's pass/fail outcome); a bare `assert` with
  the same unprovable predicate does NOT reject — it silently keeps a
  runtime check with no diagnostic at all. This is exactly the
  "prove is always compile-time, no fallback to runtime" distinction
  `06_smt_debug.md`'s "Tactics for hard proofs" section describes.
  Added a regression test asserting the hint names the real env var
  and never the dead one. Full `cargo test --release --workspace`
  clean (2616 passed, 0 failed) after this fix.

## Bug found sweeping docs/TESTING_MATRIX_TODO.md's priority list (added+fixed 2026-08-01)

- [x] **BUG-61 (found+fixed 2026-08-01 — two independent under-allocation
  /ordering bugs, one per backend, both specific to `Vec<Channel<T,N>>`
  — and by extension `Vec<Mutex<T>>`/`Vec<RwLock<T>>`).**
  Found running the feature x backend testing-matrix sweep (see
  `docs/TESTING_MATRIX_TODO.md`, priority item 2: `Channel<T,N>` had
  zero end-to-end coverage). Minimal repro:
  ```vani
  fn main() -> i64 {
    let ch_a: Channel<i64, 4> = channel_new();
    let ch_b: Channel<i64, 4> = channel_new();
    let chans: Vec<Channel<i64, 4>> = vec(ch_a, ch_b);
    let i: i64 = 0;
    let total: i64 = 0;
    while i < 2 {
      let _ = channel_send(mut ref chans[i], (i + 1) * 10);
      let v: i64 = channel_recv(mut ref chans[i]);
      total = total + v;
      i = i + 1;
    }
    print "total =", total;
    return 0;
  }
  ```
  **LLVM backend**: crashed with heap corruption (`free(): invalid
  next size (fast)` / `munmap_chunk(): invalid pointer`, sometimes
  inside LLVM's own JIT linker rather than user code). Root cause:
  both `vec()`-literal lowering sites (the generic expr-level handler
  and the dedicated `emit_vec_let_from_literal` for `let x: Vec<T> =
  vec(...)`) only routed element size through the correct runtime
  GEP-null `sizeof` trick (`vec_element_size_expr`) for `Struct`/
  `Tuple`/payloaded-`Enum` elements; every other element type fell
  through to `vec_element_byte_size`'s flat, **hardcoded 24-byte**
  hand-wave for `Channel`/`Mutex`/`Guard` (comment: "Vecs of these
  aren't allowed by the checker today so this is defensive" — false;
  the checker allows it and main.rs's SSA gating explicitly
  special-cases `Vec<Channel|Atomic>`). A `Channel<i64,4>`'s real
  LLVM struct is 80 bytes (`{ [4 x i64], [4 x i64], i64, i64 }`), so
  the 2-element buffer was malloc'd at 48 bytes instead of 160,
  corrupting the heap on the second element's store. Fixed by adding
  proper `vec_element_size_expr` arms for `Channel`/`Mutex`/`Guard`/
  `RwLock`/`ReadGuard`/`WriteGuard` (same GEP-null trick, using each
  type's existing `llvm_*_struct` name helper) and extending both
  `vec()`-literal call sites' struct/tuple/enum gating to also route
  these six types through it.
  **C backend** (SSA-C path is gated away from `mut ref vec[i]`
  shapes, so this exercised tree-C): `cc` failed outright —
  `error: unknown type name 'intent_channel_int64_t_4'` cascading
  into `expected 'const int *'` argument mismatches throughout the
  generated Vec-of-Channel helper functions. Root cause: tree-C's
  `emit_c` emitted the `Vec<Channel<T,N>>` bundle (whose typedef
  spells its `data` field as `intent_channel_<T>_<N>*`) from the
  `element_types` loop, but only emitted the `intent_channel_<T>_<N>`
  struct itself much later, inside `emit_concurrency_runtime_helpers`
  — after prototypes and function bodies, for its condvar/task/
  barrier text-scan gating — so the Vec bundle referenced a type name
  C hadn't seen yet. Fixed by splitting that function into
  `emit_concurrency_type_bundles` (Channel/Mutex/RwLock — no
  text-scan dependency, AST-derived specs only) which now runs right
  after every user struct is fully defined but before the
  `element_types` Vec-bundle loop, and `emit_concurrency_runtime_
  extras` (Condvar/Task/Barrier — genuinely needs the scanned text)
  which stays at the original late call site. The equivalent
  ordering bug in `ssa_backend_c.rs` (reachable when a `Vec<Channel<
  T,N>>` program doesn't use `mut ref vec[i]`, so takes the SSA-C
  path) was fixed the same way, reordering its channel-spec
  collection+emission before its Vec-bundle emission.
  New tests: `src/lib.rs` compile-time tests for all three symptoms
  (LLVM under-allocation, C ordering in tree-C, C ordering in
  SSA-C) plus a real end-to-end test (`tests/run_end_to_end.rs`)
  asserting the correct summed value on both backends, including a
  push-growth-beyond-initial-capacity variant. Also added the
  `Vec<Channel<T,N>>` worked example itself as a new automated sweep
  case so this class of bug can't silently regress.

  **Follow-up #1, found+fixed same day (2026-08-02)**, sweeping the
  new "container x concurrency-handle nesting" section added to
  `docs/TESTING_MATRIX_TODO.md`: a struct FIELD of type
  `Channel<T,N>` sitting alongside a `Vec<T>` field --
  `struct Worker { ch: Channel<i64,4>, buf: Vec<i64> }` -- hit the
  identical "unknown type name" failure as bare `Vec<Channel<T,N>>`
  under `--backend=c`, one level up: the channel bundle was emitted
  (per the fix above) right before the `element_types` Vec-bundle
  loop, which is itself AFTER the unified struct topo loop -- too
  late for a struct field that needs the channel struct at the
  struct's OWN declaration. Fixed by partitioning `channel_specs`/
  `mutex_specs`/`rwlock_specs` into "element needs no full struct
  body" (the overwhelmingly common case -- `Channel<i64,N>`, etc.)
  and "element needs a full struct/enum/tuple body" (e.g.
  `Channel<UserStruct,N>`) groups: the former now emits right after
  struct FORWARD declarations (before any struct BODY, including
  one with a Channel field), the latter still emits after the
  unified topo loop (since it needs full struct bodies itself). The
  shared futex-primitives emission was correspondingly guarded
  against double-emission across the two call sites.

  **Follow-up #2, found+fixed same day (2026-08-02)**, same sweep:
  `struct Counter { m: Mutex<i64>, history: Vec<i64> }` failed
  differently -- not an ordering bug, a NAMING bug. `c_element_
  storage` (the function struct-field declarators route through)
  had explicit arms for `Channel`/`Atomic`/`Vec`/`Struct`/etc. but
  NONE for `Mutex`/`Guard`/`RwLock`/`ReadGuard`/`WriteGuard`, so
  those fell through to `c_leaf_type`'s hardcoded placeholder
  spellings (`intent_mutex_i64`, `intent_guard_i64`,
  `intent_rwlock_i64`) -- names that don't match the REAL
  `element_tag`-based bundle names (`intent_mutex_int64_t` etc.) and
  were never actually emitted anywhere, so cc rejected the field
  declaration outright. Same bug class as the historical BUG-22/
  BUG-47/closures #208/#209 (a `c_leaf_type` caller "forgetting to
  special-case" a parametric handle type) -- just never closed for
  these five types in this specific function. Fixed by adding the
  five missing arms, delegating to the already-correct
  `c_mutex_storage`/`c_guard_storage`/`c_rwlock_storage`/
  `c_read_guard_storage`/`c_write_guard_storage` helpers (which
  existed and were already correct -- just unused by this caller).

  Both follow-ups: new tests added to `src/lib.rs` (2) and
  `tests/run_end_to_end.rs` (3, the third specifically confirming
  `Vec<Mutex<T>>`/`Vec<RwLock<T>>` -- covered by the original BUG-61
  fix's code paths but never actually run end-to-end until now).

## Bug found continuing the docs/TESTING_MATRIX_TODO.md nested-combinations sweep (added+fixed 2026-08-02)

- [x] **BUG-62 (found+fixed 2026-08-02 — FOUR independent bugs, three
  in tree-C and one in tree-LLVM, all specific to `Vec<[T; N]>`
  where `T` is a non-trivial (Struct) type).** Found sweeping the
  "multi-level container nesting"
  section for `Vec<Array<Struct,N>>`. Minimal repro:
  ```vani
  struct Point { x: i64, y: i64 }
  fn main() -> i64 {
    let a1: [Point; 2] = [Point { x: 1, y: 1 }, Point { x: 2, y: 2 }];
    let a2: [Point; 2] = [Point { x: 3, y: 3 }, Point { x: 4, y: 4 }];
    let vs: Vec<[Point; 2]> = vec(a1, a2);
    let total: i64 = 0;
    for arr in vs {
      total = total + arr[0].x + arr[0].y + arr[1].x + arr[1].y;
    }
    print total;
    return 0;
  }
  ```
  **tree-C, bug 1**: `array_c_typedef`'s helper for a `Vec<[T;N]>`
  typedef's INNER element spelling fell through to `c_leaf_type` for
  `Type::Struct`, which returns the bare placeholder comment `"/*
  struct */"` — producing the syntactically broken `typedef /*
  struct */ intent_arr2_Struct_Point[2];`, which made `cc` infer an
  implicit `int` element type and reject every real use downstream.
  Fixed by routing through `c_element_storage` (already has a real
  `Struct` arm) instead.
  **tree-C, bug 2**: once bug 1 was fixed, `vec(a1, a2)` itself still
  failed: the codegen for `Vec<[T;N]>` literals only special-cased
  an argument that is ITSELF an inline `[..]` array literal
  (stripping its cast so it could nest inside the outer compound
  literal's braces); any OTHER array-typed argument (a named `let`-
  bound variable, here `a1`/`a2`) fell through to a plain `emit_expr`
  call, emitting the bare variable name as an initializer-list item
  — C forbids using an array-typed EXPRESSION as an initializer-list
  item at all, which for Struct-element arrays silently produced
  malformed flattened-field assignments (`"make integer from
  pointer"`, `"invalid initializer"`) instead of a clean error.
  Fixed by rebuilding the whole `Vec<[T;N]>`-literal construction
  uniformly via `memcpy` (malloc the buffer, then `memcpy` each
  argument's bytes into its slot) — works identically for a literal
  or a named variable, since both decay to a pointer for memcpy.
  **tree-C, bug 3**: the `for x in xs` consuming-iteration lowering
  for a `Vec<[T;N]>` element also used a plain `=` to bind the loop
  variable (`intent_arr2_Struct_Point v_arr = v_vs.data[idx];`) —
  invalid C; arrays can't be assigned via `=` even through a typedef
  alias. Fixed by declaring the local bare and `memcpy`-ing the
  slot's bytes in for array-typed elements specifically.
  **tree-LLVM, bug 4** (a genuine 4th, independent bug, same repro):
  `vec_element_size_expr` had no arm for `Type::Array` at all, so it
  fell through to `vec_element_byte_size`'s byte-count fallback —
  which itself has no real understanding of `Struct` sizes either
  (its own fallback treats any unrecognized type as 8 bytes), so
  `[Point;2]` (32 real bytes: 2 × 2×i64) was sized at 16 bytes,
  under-allocating the `vec()` literal's malloc'd buffer by half and
  corrupting the heap (LLVM's own `lli` JIT crashed deep inside its
  register-allocation/bitcode-writing passes — malformed IR, not a
  clean runtime error). Fixed by adding a proper `Type::Array` arm
  using the same GEP-null `sizeof` trick already used for Struct/
  Tuple/Channel/etc, directly against the array's own LLVM type `[N
  x T]` (correct for any T without needing recursive byte-counting),
  and widening both `vec()`-literal call sites' "needs runtime
  sizeof" gating to route `Type::Array` elements through it.
  New tests: 2 `src/lib.rs` (one per tree-C sub-bug: typedef
  placeholder, memcpy construction) plus 1 real end-to-end test
  (`tests/run_end_to_end.rs`) asserting the correct summed value
  (20) on both backends, closing all four sub-bugs at once.

- [x] **BUG-63 (found+fixed 2026-08-02 — tree-C only).** Found
  continuing the same sweep, item `struct { items: Vec<(i64,
  OwnedStr)> }`. Minimal repro:
  ```vani
  struct Bag { items: Vec<(i64, OwnedStr)> }
  fn main() -> i64 {
    let items: Vec<(i64, OwnedStr)> = vec((1, "a" + ""), (2, "b" + ""));
    let bag: Bag = Bag { items: items };
    let pair: (i64, OwnedStr) = clone_at(ref bag.items, 0);
    let (num, s) = pair;
    print num;
    print s;
    return 0;
  }
  ```
  Runs correctly on LLVM. Under `--backend=c`, `cc` fails: "unknown
  type name `intent_tuple_int64_t_owned_str`". Root cause: a Tuple
  shape that ONLY ever appears inside a struct field (never in a
  function signature or body) was never fed into tree-C's
  `tuple_shapes` collection at all -- only function-level walks
  existed, mirroring the Vec-element collection but never extended
  to structs/enums the way that one was. Meanwhile the struct-field
  `Vec<(i64, OwnedStr)>` bundle WAS emitted (eagerly, by the "no
  user-struct dependency" fast path — `vec_element_has_user_struct`
  didn't recognize `Type::Tuple` at all, so it never deferred),
  referencing the tuple struct's name regardless of it never being
  declared anywhere in the file. Same root-cause SHAPE as BUG-61's
  struct-field follow-up, different type (Tuple, not Channel/Mutex/
  RwLock). Fixed by (1) collecting tuple shapes from struct fields
  and enum payloads too, and (2) partitioning them the same way as
  BUG-61's channel/mutex/rwlock specs: shapes whose OWN elements
  need no full struct/enum/tuple body (the common case — scalars,
  `OwnedStr`, refs) emit early, right where the early struct-field
  Vec-bundle pass needs them; shapes that DO need one (e.g.
  `(Point, i64)`) stay deferred to the existing later position.
  `vec_element_has_user_struct` now defers a Tuple element only
  when that specific tuple shape needs a full struct body — tuples
  that don't are safe to leave un-deferred since their bundle now
  exists early.
  New tests: 1 `src/lib.rs` (asserts the tuple typedef precedes the
  Vec-of-tuple bundle referencing it) plus 1 real end-to-end test
  (both backends, correct printed values).

- [x] **BUG-64 (found+fixed 2026-08-02 — silent double-free, both
  backends; a soundness gap, not just a codegen bug).** Found
  sweeping "container x concurrency-handle nesting" for
  `Channel<StructWithVecField, N>`. Minimal repro:
  ```vani
  struct Msg { id: i64, tags: Vec<i64> }
  fn main() -> i64 {
    let ch: Channel<Msg, 4> = channel_new();
    let tags: Vec<i64> = vec(10, 20, 30);
    let m: Msg = Msg { id: 7, tags: tags };
    let _ = channel_send(ref ch, m);
    let got: Msg = channel_recv(ref ch);
    print got.id;
    return 0;
  }
  ```
  Crashed with `free(): double free detected in tcache 2` on BOTH
  backends (LLVM: `lli` aborts; C: the compiled binary aborts) --
  not a compile error, a runtime memory-safety crash with no
  warning at all. Root cause: `is_supported_channel_element`
  accepted `Type::Struct(_)`/`Type::Enum(_)` unconditionally,
  with no check that the type is actually Copy. `channel_send`/
  `channel_recv` copy the payload BYTEWISE into/out of the ring
  buffer (both backends' runtime helpers) -- there is no move-
  out-of-sender or deep-clone-on-send machinery. For a non-Copy
  struct (one owning a `Vec`/`OwnedStr`/`Box` field), this
  bytewise copy duplicates the heap pointer into the channel's
  slot while the checker still treats the SENDER's original
  variable (`m`) as live and due a normal scope-exit drop -- so
  both `m`'s drop AND the later `got`'s drop free the SAME heap
  buffer. The doc's own worked examples (and the pre-existing
  passing test suite) only ever used Copy-only (all-`i64`)
  struct payloads, so this gap was never exercised. Fixed by
  requiring `ty.is_copy()` for a Struct/Enum Channel element (the
  existing `is_copy()` machinery already correctly walks a
  struct's fields via `STRUCT_NON_COPY_REGISTRY`, populated
  during the checker's struct-validation pass, so this needed no
  new Copy-detection logic -- just wiring `is_supported_channel_
  element` to actually use it). Also improved the rejection
  diagnostic to explain WHY (aliasing/double-free risk), not just
  restate the old "must be an integer width or bool" text that
  was already inaccurate before this fix (it never mentioned that
  Copy structs/enums were allowed at all).
  New tests: 2 `src/lib.rs` (the double-free repro is now cleanly
  rejected; the pre-existing Copy-only-struct case still compiles)
  plus 1 real end-to-end test (both backends: process exits
  non-zero with the new diagnostic text, and — the actually
  load-bearing assertion — stderr contains no "double free"/
  "free():" crash text at all).
  **Not chased further in this pass**: implementing real move-out-
  of-sender or deep-clone-on-send semantics so non-Copy payloads
  could be supported safely — noted as a genuine future feature,
  not a bug, since the current behavior (clean rejection) is sound.

- [x] **BUG-65 (found+fixed 2026-08-02 — tree-C, a self-inflicted
  regression from BUG-63's own fix, commit `9e24ce5`).** Sweeping
  the next checklist item,
  `Tuple<dyn Iface>`: `(dyn Shape, i64)` compiled and ran correctly
  on LLVM but failed `--backend=c` with "unknown type name
  `intent_dyn_Shape`". Root cause: BUG-63's new "early tuple bundle"
  partitioning reused `concurrency_element_needs_full_struct_def` to
  decide which tuple shapes are safe to emit right after struct
  forward-declarations — but that function didn't recognize
  `Type::Object` (a `dyn Iface` fat pointer), so a tuple containing
  one was wrongly treated as early-eligible and emitted before
  `emit_dyn_iface_typedefs` had run. Before BUG-63's fix this exact
  shape never failed, since EVERY tuple bundle emitted at the single
  late position (comfortably after dyn typedefs) — the bug only
  became reachable once an early-emission path existed at all.
  Fixed by adding `Type::Object(_) => true` to the shared
  "needs-deferral" check, alongside Struct/Enum/Tuple.
  New tests: 1 `src/lib.rs` + 1 real end-to-end test (both backends,
  correct printed values).

- [x] **BUG-66 (found+fixed 2026-08-02 — tree-C; PARTIALLY fixed,
  see the deferred gap below).** Found sweeping "closure capturing
  a Vec/Channel by move, stored in a struct field, called later" --
  the pattern `intermediate/06a_closures_primer.md`'s "Stored in
  data structures" section documents. Minimal repro (Copy-only
  capture):
  ```vani
  struct Handler { cb: Closure(i64) -> i64 }
  fn main() -> i64 {
    let base: i64 = 100;
    let cb = fn(extra: i64) -> i64 { return base + extra; };
    let h: Handler = Handler { cb: cb };
    let f: Closure(i64) -> i64 = h.cb;
    print f(5);
    return 0;
  }
  ```
  Ran correctly on LLVM. Under `--backend=c`, `cc` failed: "unknown
  type name `intent_closure_i64_i64`". Same root-cause SHAPE as
  BUG-61/63: the closure fat-pointer struct typedef (`{ uint64_t
  env; R (*call)(uint64_t, args); }` -- no dependency on any user
  struct's full body) was emitted much later, bundled together with
  the trampoline/constructor functions (which genuinely DO need
  full env-struct bodies, since they dereference captured fields)
  — after the unified struct topo loop had already emitted
  `Struct_Handler`'s body referencing the not-yet-declared typedef.
  Fixed by splitting the typedef-only half out to emit early (right
  after `emit_dyn_iface_typedefs`, alongside the Channel/Mutex/
  RwLock/Tuple early-emission blocks), leaving the trampoline/
  constructor half at its original late position.
  New tests: 1 `src/lib.rs` (typedef-before-struct-body ordering) +
  1 real end-to-end test (both backends, correct computed value) --
  both specifically for the Copy-only-capture case, which is now
  fully correct end-to-end.
  **Deferred gap found in the same sweep, NOT fixed**: the SAME
  pattern with a HEAP-owning capture (e.g. `let data: Vec<i64> =
  vec(1,2,3,4); let cb = fn(extra: i64) -> i64 { return data[0] +
  extra; };` moved into a struct field) crashes on BOTH backends --
  LLVM: `lli` rejects the emitted IR ("base element of getelementptr
  must be sized" -- the synthesized env struct is referenced as an
  opaque/unsized type at the point the closure is stored into and
  read back from the struct field); C: `free(): double free detected
  in tcache 2` at runtime. This is a materially different, deeper
  problem than the typedef-ordering bug above -- it's an affine-
  ownership/lifetime gap in how a closure's heap-owning env
  interacts with being moved across a struct-field boundary, not a
  missing typedef. The compiler already tracks which closures have
  an affine (non-Copy) env (`CLOSURE_AFF_ENV_SET`), which is a
  plausible foundation for a future clean-rejection fix (mirroring
  BUG-64's Channel-Copy-requirement pattern) -- not attempted in
  this pass given the scope (would need the CHECKER, not just
  codegen, to reject the pattern at the struct-field assignment
  site). Documented here rather than silently left broken; not
  chased further today.

## Bug found fixing intermediate/06a_closures_primer.md's Closure syntax examples (added+fixed 2026-08-02)

- [x] **BUG-67 (found+fixed 2026-08-02 — checker-level, silent
  use-after-free/double-free, both backends' generated code
  affected).** Found writing this tutorial's flagship "factory
  function returns a closure that captured a value" example
  (`make_greeter`) against the real compiler, using the actual
  captured value the tutorial had always shown -- an `OwnedStr`.
  Minimal repro:
  ```vani
  fn make_greeter(name: OwnedStr) -> Closure(i64) -> i64 {
    let g = fn(x: i64) -> i64 { print "hello,", name, x; return 0; };
    return g;
  }
  fn main() -> i64 {
    let say_hi: Closure(i64) -> i64 = make_greeter("alice" + "");
    say_hi(5);
    return 0;
  }
  ```
  Crashed with `free(): double free detected in tcache 2` under
  `--backend=c` (the LLVM backend's independently-implemented
  codegen happened not to trip over this specific case, but the
  underlying checker bug is backend-agnostic — see below). A
  Copy-only capture (e.g. an `i64`) masked the bug, since freeing an
  env struct with no heap fields is harmless — that's why the
  Copy-only case in BUG-66's own regression test never caught this.
  Root cause: the return-path code that decides which local
  affine-closure bindings to drop (`return v;` shouldn't drop `v`
  itself, only OTHER still-live locals) relies on
  `info.moved.is_none()` to detect "is this the variable being
  returned." That flag is set by `consume_if_moved_var`, which
  opens with `if checked.ty().is_copy() { return; }` — and
  `Type::Closure` has no explicit arm in `Type::is_copy()`, so it
  falls through to that function's `_ => true` catch-all (Closure
  is structurally a 2-pointer bundle, so treating it as Copy at the
  TYPE level is reasonable for many purposes, just not this one).
  So `consume_if_moved_var` always returned early for a returned
  closure, `info.moved` never got set, and the affine-closure drop
  pass fired for the returned variable exactly like any other
  still-live local — freeing the env (and its captured `OwnedStr`)
  that the SAME statement was about to return. Fixed narrowly (not
  by changing `Type::is_copy()` itself, which is load-bearing in
  many unrelated places across the checker and both backends) by
  explicitly excluding the returned variable's name from the
  affine-closure drop pass at its one call site, mirroring what
  correct `.moved` tracking would have done.
  New tests: 1 `src/lib.rs` (asserts the generated C no longer frees
  the returned closure's env inside the factory function) plus 1
  real end-to-end test (`tests/run_end_to_end.rs`, 5 runs per
  backend matching this session's "non-deterministic-looking crash"
  precedent) asserting the correct printed output on both backends.

## Bug found sweeping container x SMT contracts (added+fixed 2026-08-02)

- [x] **BUG-68 (found+fixed 2026-08-02 — silent soundness gap, no
  diagnostic at all, checker-level).** Found while testing
  `docs/TESTING_MATRIX_TODO.md`'s "`requires`/`ensures` referencing a
  `Vec<Struct>` element" row. `verify_ensures_at_return` (the
  return-site `ensures`-clause checker) treated ANY `ensures` clause
  the SMT encoder couldn't fully encode
  (`Verdict::Unknown`/`Unavailable`/`SkippedUnsupported`) as silently
  PROVEN — the code was a bare `{}` match arm with a comment
  describing a "fall back to constant-true check" that was never
  actually implemented. A deliberately FALSE `ensures` clause over a
  `ref` struct parameter's field (`ensures _return > r.hi;` on a
  function that returns `r.lo`) compiled clean with `vanic check`
  reporting `ok`. Every sibling contract-verification path (`prove`
  statements, loop invariants, both of which already push a real
  diagnostic on the same three verdicts) was NOT affected — only
  this one return-site `ensures` path had the hole. Root cause of
  *why* this was so easy to hit: the struct-field-to-SMT synthetic-
  var machinery (`p.x` → `Var("p__x")`) only ever covered a local
  bound to a struct LITERAL (`struct_literal_fields`); a plain
  struct-typed parameter (or a `ref`/`mut ref` to one) always hit the
  unconditional `FieldAccess` bail in `smt.rs`'s encoder
  ("structs not supported in SMT v1"), so ANY contract referencing a
  struct parameter's field — not just a `Vec<Struct>` element's field
  — silently fell into the unverified/accepted hole. Confirmed this
  wasn't hypothetical: `examples/edge_cases/mix_smt_struct_field.vani`
  and `mix_smt_pure_struct_ref.vani`, both pre-existing example files
  whose comments claim "ensures provable from requires," had NEVER
  actually been checked by the solver.
  Fixed two ways: (1) generalized the struct-field rewrite/declare
  machinery so ANY struct-typed binding — not just a literal-init
  local — gets a per-field SMT constant declared: literal-bound
  fields get the existing `name__field == literal-expr` equality
  fact, everything else (parameters, `ref`/`mut ref`, non-literal
  locals) gets a free/opaque constant with no defining equality, so
  `requires`/`ensures` can still relate fields to each other (e.g.
  `r.lo <= r.hi`) even with no concrete value known. This makes `ref`
  struct parameter field access in contracts *genuinely* verifiable
  now, not just gracefully rejected. (2) `verify_ensures_at_return`
  now pushes a real diagnostic ("cannot verify 'ensures' clause: ...")
  on Unknown/Unavailable/SkippedUnsupported, mirroring the pre-existing
  loop-invariant path exactly — closing the silent-accept hole for
  whatever's still genuinely unencodable (e.g. `Vec<Struct>` element
  field access via `Index`, which array theory still doesn't model —
  correctly and cleanly rejected now instead of silently passing).
  **Bonus finding**: re-running the now-real solver against
  `mix_smt_pure_struct_ref.vani` immediately caught an actual latent
  bug in that example's own contract — `r.hi - r.lo` can overflow
  i64 for extreme `lo`/`hi` values (counterexample:
  `r.lo = -9223372032559808512, r.hi = 9223372036854775806`),
  violating its `ensures _return >= 0;` — previously rubber-stamped
  as "verified," now fixed with overflow-bounding `requires` clauses
  (the same pattern `12a_smt_primer.md` already teaches for exactly
  this reason). New tests: 4 `src/lib.rs` (silent-accept regression
  now rejected, true-case discharges via the new opaque-field
  modeling, `Vec<Struct>` element access still cleanly rejected with
  an explicit diagnostic, plus the pre-existing struct-literal
  `prove` tests re-verified clean — 134/134 SMT/ensures/struct-field-
  filtered `src/lib.rs` tests passed before and after).
  **Follow-up gap found+fixed same day**, testing the testing-
  matrix's next row (`invariant` in a loop mutating a `Vec<Struct>`
  element): a plain (non-Vec) struct-typed loop accumulator whose
  field is mutated via `acc.total = acc.total + 10;` inside the loop
  body — the exact kind of "scalar loop state" case the row assumed
  was already covered — had its `invariant acc.total == i * 10;`
  incorrectly rejected as "not preserved by the loop body", because
  `walk_for_reassigns` (the loop-body-mutation summarizer that
  builds the preservation check's entry→exit substitution map) only
  handled `Stmt::Assign`/`Stmt::Let`, never `Stmt::FieldAssign` — so
  the field mutation was invisible to it and the solver kept using
  the field's original struct-literal value for the post-body state.
  Fixed by having `walk_for_reassigns` record a `"name__field"`-
  keyed substitution entry for `FieldAssign` (same synthetic-name
  convention as the BUG-68 fix above) and teaching `substitute_expr`'s
  `FieldAccess` arm to consult it. Confirmed both directions: the
  true invariant now discharges, and a deliberately false one is
  still correctly rejected (not a soundness regression in the other
  direction). `Vec<Struct>` element mutation via `mut ref cs[i]`
  alongside a scalar-only invariant compiles and runs correctly on
  both backends (new e2e test); an invariant that tries to reference
  the mutated Vec element's field directly is still cleanly rejected
  (array theory doesn't model struct elements — real v1 limitation,
  not a divergence bug). New tests: 2 `src/lib.rs` + 1
  `tests/run_end_to_end.rs` (both backends) — 204/204 SMT/ensures/
  struct-field/invariant-filtered `src/lib.rs` tests passed before
  and after.

## Bug found sweeping "ensures on Option<Vec<T>>/Result<Struct,E> return" (found+fixed 2026-08-02)

- [x] **BUG-69 (found+fixed 2026-08-02 — LLVM backend, invalid-IR
  crash, unrelated to what the sweep row was actually testing).**
  Found while testing the testing matrix's "`ensures` on a function
  returning `Option<Vec<T>>` or `Result<Struct, E>`" row. The SMT
  side of that row turned out to be a clean, already-covered-by-
  BUG-68's-fix rejection (enum-variant patterns aren't modeled by
  the SMT encoder — `ensures match _return { Option.Some(_) then
  true, ... };` correctly errors "method calls not supported in SMT
  v1" instead of silently passing). But exercising the row's
  *runtime* half — `Option<Vec<T>>`/`Result<Struct, E>` actually
  used, no SMT contract on the payload — crashed the LLVM backend
  entirely unrelated to Option/Result: `vec_fill` called anywhere
  textually AFTER a plain `if` statement in the SAME function
  produced `PHI node entries do not match predecessors!` at the LLVM
  verifier. Minimal repro (no Option/Result/generics involved at
  all):
  ```vani
  fn f(n: i64) -> Vec<i64> {
    let y: i64 = 0;
    if n < 0 { y = 1; }
    let xs: Vec<i64> = vec_fill(n, 7);
    return xs;
  }
  ```
  Root cause: `backend_llvm.rs`'s `emit_stmt` for `TypedStmt::If`
  never updated `ctx.current_block` after emitting the if's
  then/else/cont blocks — every OTHER multi-block construct in the
  tree emitter (`while`, `match`, `if let`, ...) already does this
  (confirmed by grep: ~25 other call sites assign
  `ctx.current_block` after their own block emission; plain `If` was
  the one exception). `vec_fill`'s fill loop is the one builtin that
  hand-rolls a raw SSA phi loop and reads `ctx.current_block` to name
  its loop-entry phi's predecessor (`{} = phi i64 [0, %{entry_blk}],
  [{}, %{body_lbl}]`) — everything else in the tree emitter uses
  alloca+load/store for locals, so nothing else actually depended on
  `ctx.current_block` being accurate, which is why this went
  undetected: `vec_fill` is only ever exercised as the first
  statement of a function in existing tutorials/benchmarks (e.g. the
  SIMD chapter's dot-product examples), where `ctx.current_block` is
  still correctly "entry" by coincidence. The instant `vec_fill`
  follows ANY prior `if`, the phi's declared predecessor (whatever
  `ctx.current_block` was before the if — stale) diverges from the
  real CFG predecessor (the if's own `cont` block) — invalid IR, LLVM
  verifier rejects it, `lli`/`llc` refuses to run it.
  Fixed by adding `ctx.current_block = cont_lbl;` after the if's
  block emission in `backend_llvm.rs`'s `TypedStmt::If` arm (in the
  same place `ctx.terminated` is already updated). C backend
  unaffected — its `if` codegen uses real C control flow, no
  SSA-phi/block-identity bookkeeping to go stale. New tests: 1
  `src/lib.rs` (asserts the emitted IR's `vec_fill` phi predecessor
  is not the stale `entry` block after an intervening if) + 1
  `tests/run_end_to_end.rs` (both backends, the actual
  `Option<Vec<T>>`/`Result<Struct, E>` + `vec_fill`-after-`if`
  scenario this bug was found in, correct output on both).

## Bug found sweeping "generic struct instantiated at 2+ different T" (found+fixed 2026-08-02)

- [x] **BUG-70 (found+fixed 2026-08-02 — same root cause and fix
  shape as BUG-46, just never re-checked for user-defined generic
  structs).** Found while testing the testing matrix's "generic
  struct `Box2<T>` instantiated at 2+ different T" row.
  `Env::resolve_struct_name` — the struct analog of the enum
  "exactly one candidate in the whole program" heuristic BUG-46 fixed
  — had the identical gap: `monomorphize_type_decls_in_program` drops
  the generic struct TEMPLATE once it's expanded into concrete
  monomorphizations (`Box2__i64`, `Box2__OwnedStr`, ...), so a bare
  `Box2 { items: ... }` `StructLit` can only be resolved back to a
  concrete name when EXACTLY ONE instantiation exists in the whole
  program. The instant a second instantiation exists anywhere, EVERY
  construction site for that struct breaks with "unknown struct type
  'Box2'" — even a construction whose concrete instantiation is
  perfectly clear from its own enclosing `let`'s type annotation.
  Minimal repro:
  ```vani
  struct Box2<T> { items: Vec<T> }
  fn main() -> i64 {
    let bi: Box2<i64> = Box2 { items: vec(1, 2, 3) };       // breaks
    let bs: Box2<OwnedStr> = Box2 { items: vec("a"+"") };   // breaks
    ...
  }
  ```
  A single instantiation alone (either one, alone in the program)
  compiles fine — confirmed matches BUG-46's exact symptom shape,
  just for `StructLit` construction instead of enum-variant
  construction.
  Fixed identically to BUG-46: added `resolve_bare_struct_lits_in_stmt`
  / `resolve_bare_struct_lit_receiver`, the struct analog of
  `resolve_bare_enum_ctors_in_stmt`/`resolve_bare_enum_ctor_receiver`
  — rewrites a bare `StructLit.type_name` to its already-known
  concrete monomorphized name at `let`/`return` sites, using the
  enclosing `let`'s own (already-monomorphized) annotation or the
  function's own (already-monomorphized) return type as the
  disambiguating context. `StructLit` carries its type name directly
  (no receiver-expression indirection like an enum constructor call
  has), so the rewrite is a straight field overwrite once the
  `let`/`return` context is known — simpler than the enum fix, no
  `MethodCall`-vs-`FieldAccess` payload/no-payload split to handle.
  Wired into the same three call sites the enum fix uses (plain
  functions, `implement` blocks, `methods on T` blocks). Both
  instantiations of `Box2<i64>`/`Box2<OwnedStr>` now construct and
  run correctly on both backends. New tests: 1 `src/lib.rs` + 1
  `tests/run_end_to_end.rs` (both backends).

## Bugs found sweeping "generic fn(ref Vec<T>) -> T over Struct and Tuple T" (found+fixed 2026-08-02)

- [x] **BUG-71 (found+fixed 2026-08-02 — generic-call inference,
  general bug, not container/generics-angle-specific).** Found while
  testing the testing matrix's "generic function `fn first<T>(xs:
  ref Vec<T>) -> T` monomorphized over a Struct T and a Tuple T" row.
  Narrowed immediately: a SCALAR-only repro (`T = i64` alone, no
  Struct/Tuple involved at all) already failed identically, so this
  isn't specific to the row's own Struct/Tuple angle — `ref Vec<T>`
  as a parameter shape was simply never exercised for ANY T before.
  Minimal repro:
  ```vani
  fn first<T>(xs: ref Vec<T>) -> T { return xs[0]; }
  fn main() -> i64 {
    let nums: Vec<i64> = vec(10, 20, 30);
    let n: i64 = first(ref nums);   // breaks
    ...
  }
  ```
  fails with `argument 1 to 'first__Vec_I64_' must be assignable to
  ref Vec<Vec<i64>>, got ref Vec<i64>` — i.e. T got bound to
  `Vec<i64>` (the whole referent type) instead of `i64` (its
  element). Root cause: `infer_concrete_type_for_call`'s handling of
  a `Ref{inner}`/`RefMut{inner}` call-argument expression (`ref
  nums`) resolved the referent's BARE scope type from `nums: Vec<T>`
  binding info, but never re-wrapped it in `Type::Ref`/`Type::RefMut`
  before handing it to `unify_param_to_arg` for structural
  unification against the parameter's declared type `Ref(Vec(Param(T)))`.
  Since the top-level shapes (`Ref` vs bare `Vec`) didn't match,
  `unify_param_to_arg` always returned `None` immediately, and the
  caller fell back to its "T = whole arg type" legacy path — wrong
  whenever there's a wrapper (`Vec`, `Box`, ...) between the `Ref` and
  the `Param(T)` slot in the parameter's declared type. This had gone
  undetected because a bare `ref T` param (nothing between Ref and
  Param) happens to get the right answer from that same fallback path
  by coincidence — confirmed both `ref T` and by-value `Vec<T>` (no
  ref) already worked correctly; only the `ref Vec<T>` combination
  was broken. Fixed by re-wrapping the resolved referent type in
  `Type::Ref`/`Type::RefMut` before unification, so
  `unify_param_to_arg`'s existing `(Ref(p), Ref(a))` arm can peel
  both the `Ref` AND the inner `Vec` (or `Box`, or any other
  single-arg wrapper) in lockstep, generalizing correctly for any
  nesting depth and any T.

- [x] **BUG-72 (found+fixed 2026-08-02 — name-mangling, LLVM-backend-
  only crash, found immediately after BUG-71 since fixing BUG-71 was
  needed to even reach this code path).** A generic fn specialized
  over a Tuple T (e.g. `first<T>` called with `T = (i64, i64)`)
  crashed the LLVM backend at `lli`: `expected '(' in call` on a line
  referencing `@fn_first__Tuple_[I64__I64]_` — not a valid bare LLVM
  identifier (literal `[`/`]` in the name). Root cause:
  `type_mangle`'s fallback arm for non-primitive/Struct/Enum types
  renders `format!("{:?}", ty)` (Rust's derived `Debug`) and replaces
  a fixed punctuation set with `_` — but `Type::Tuple(Vec<Type>)`'s
  derived Debug output renders the inner `Vec<Type>` with `[`/`]`
  (e.g. `Tuple([I64, I64])`), and `[`/`]` weren't in the replacement
  set, so they survived verbatim into the mangled function name. The
  C backend was unaffected in the exact repro that found this (its
  own code path for this case apparently doesn't hit the same
  string), but `[`/`]` aren't valid in a bare C identifier either, so
  the fix applies to both. Fixed by adding `'['`/`']'` to
  `type_mangle`'s replacement character set — a two-character, fully
  general fix (also covers `FnPtr`'s param-list Vec, and anything
  else whose Debug repr embeds a `Vec<Type>`).
  New tests (both bugs): 2 `src/lib.rs` + 1 `tests/run_end_to_end.rs`
  (both backends) — confirmed scalar, Struct, and Tuple T all
  correctly infer and specialize through the same generic
  `fn first<T>(xs: ref Vec<T>) -> T`.

## Bug found sweeping "Vec<GenericStruct<i64>> alongside Vec<GenericStruct<f64>>" (found+fixed 2026-08-02)

- [x] **BUG-73 (found+fixed 2026-08-02 — direct follow-up gap in
  BUG-70's own fix, found immediately by testing the next row).**
  BUG-70 fixed a bare generic-struct `StructLit` failing to resolve
  once 2+ instantiations exist, but only for the shape `let x:
  Box2<T> = Box2 { .. };` (the StructLit is the LET's own top-level
  RHS). The testing matrix's very next row —
  `Vec<GenericStruct<i64>>` alongside `Vec<GenericStruct<f64>>` —
  writes the natural container form instead: `let vi: Vec<Box2<i64>>
  = vec(Box2 { val: 100 }, Box2 { val: 200 });`. Here the LET's RHS
  is `Call { name: "vec", args: [StructLit, StructLit] }`, not a bare
  `StructLit` — `resolve_bare_struct_lits_in_stmt`'s match only
  checked `annotation: Some(Type::Struct(target))` and only rewrote
  when `expr` itself was a `StructLit`, so it never even looked
  inside the `vec(...)` call's argument list, and never fired at all
  for a `Vec<...>`-typed annotation. Both `Box2<i64>` and
  `Box2<OwnedStr>` broke identically with "unknown struct type
  'Box2'" the instant both instantiations coexisted — this is
  arguably the MORE common way real code would hit BUG-70's bug
  class in the first place, since `vec(literal, literal, ...)` is
  the standard construction idiom used throughout every example file
  in this repo. Fixed by (1) extending
  `resolve_bare_struct_lit_receiver` to recurse into a `vec(...)`
  call's every argument (each shares the Vec's declared element
  type), and (2) adding a `Stmt::Let { annotation: Some(Type::Vec(
  Struct(target))), .. }` match arm to
  `resolve_bare_struct_lits_in_stmt` alongside the existing bare
  `Type::Struct(target)` one. Deliberately narrow to the `vec`
  builtin specifically (not arbitrary function calls), matching the
  same "add resolving power, never remove it" philosophy BUG-46's
  original fix established. Both a Copy (`Box2<i64>`) and non-Copy
  (`Box2<OwnedStr>`) instantiation, each in its own `Vec`, now
  construct and run correctly on both backends — no further BUG-61-
  class element-size/free-helper bug found once construction itself
  worked (confirmed via a non-Copy `OwnedStr` instantiation
  specifically, the shape most likely to expose a wrong-element-size
  bug). New tests: 1 `src/lib.rs` + 1 `tests/run_end_to_end.rs` (both
  backends).

## Bug found sweeping "enum variant payload is Vec<Struct> or Tuple<Array>" (found+fixed 2026-08-02)

- [x] **BUG-74 (found+fixed 2026-08-02 — three layered bugs: one
  checker-level admission gap, two C-backend-only codegen gaps).**
  `Vec<Struct>` as an enum payload was already fine (checked, no
  bug). A `Tuple` containing an `Array` as an enum payload (e.g.
  `(i64, [i64; 3])`) was not:

  1. **Checker gate too conservative.** The enum-payload admission
     check computed `payload_ty.is_copy()` for the whole `Tuple`,
     and `Type::Array::is_copy()` returns `false` unconditionally
     (a deliberate design choice for reasons unrelated to payload
     safety — array-of-Copy-elements is handled via an ad-hoc
     `array_of_copy` special case everywhere it's needed instead,
     e.g. struct fields already have their own `[T;N] of Copy`
     arm). So `Type::Tuple([I64, Array{I64,3}]).is_copy()` — which
     recurses element-wise — returned `false`, and `(i64, [i64;3])`
     was rejected with "payload type ... is not admitted in v1"
     even though it's exactly as safe as any Copy payload (no heap
     pointers, all stack/inline data). Fixed with a
     `tuple_of_admitted` check that mirrors `array_of_copy` one
     level deeper: a Tuple whose every element is either `is_copy()`
     or itself an array-of-Copy-elements is admitted.
  2. **C backend: typedef never emitted (found immediately after
     relaxing the gate).** `emit_tuple_bundle`'s C emission spells
     an `Array` element via `intent_arr3_int64_t` (the wrapped-array
     typedef `c_element_storage` produces for aggregate contexts),
     but `emit_array_typedefs_for` — the pass responsible for
     actually declaring that typedef — never recursed into
     `Type::Tuple` elements, and its one call site only fed it the
     "Vec element" axis, never enum payloads or bare tuple shapes:
     `cc` rejected the file with "unknown type name
     'intent_arr3_int64_t'". Fixed by (a) adding a `Type::Tuple`
     recursion arm to `emit_array_typedefs_for`, and (b) moving the
     array-typedef emission pass earlier in `backend_c.rs`'s emit
     function — before the "early tuple bundle" emission loop that
     now needs it declared first — sharing the SAME `seen` set with
     the original later Vec-element-only call site so neither pass
     double-emits a shape the other already covered.
  3. **C backend: initializer syntax (found immediately after fixing
     #2).** Even with the typedef correctly declared, `cc` then
     rejected the generated compound literal with "array initialized
     from non-constant array expression" — `TypedExprKind::Tuple`'s
     C emission called `emit_expr` uniformly for every element,
     which for an inline `[1,2,3]` `ArrayLit` produces a CAST
     compound literal (`((int64_t[3]){1,2,3})`) — C forbids assigning
     that to a struct member of array type. `TypedExprKind::StructLit`
     already had the correct special case (a bare-brace `{1,2,3}`
     form) for exactly this situation; `Tuple`'s emission just never
     got the matching arm. Fixed by mirroring StructLit's special
     case.
  Bug #3 turned out to be entirely general, not enum-specific:
  confirmed a bare local `let x: (i64, [i64;3]) = (42, [1,2,3]);`
  (no enum anywhere) hit the identical C-backend crash before the
  fix — a pre-existing gap this sweep row happened to be the first
  thing to actually exercise. All three fixes together: `Tuple<Array>`
  enum payloads now construct, dispatch, and destructure correctly on
  both backends; the LLVM backend was unaffected throughout (it
  already handled this shape correctly once the checker gate
  allowed it through). New tests: 4 `src/lib.rs` + 3
  `tests/run_end_to_end.rs` (both backends). Full `cargo test
  --release --workspace`: 13/13 binaries clean, 0 failed (both
  before adding these new tests and after running them individually).

## Bug found sweeping "match over Vec<Enum> with 3+ variants, mixed Copy/non-Copy payloads" (found+fixed 2026-08-02)

- [x] **BUG-75 (found+fixed 2026-08-02 — real LLVM/C backend
  divergence, silent scalar-payload corruption, two layered root
  causes).** Found while testing the testing matrix's "match over
  `Vec<Enum>` with 3+ variants, mixed Copy/non-Copy payloads" row.
  Minimal repro:
  ```vani
  enum Item { Num(i64), Text(OwnedStr), Flag(bool), Nothing }
  fn main() -> i64 {
    let items: Vec<Item> = vec(Item.Num(7), Item.Flag(true));
    let it0: Item = clone_at(ref items, 0);   // becomes Num(0) on LLVM!
    let it1: Item = clone_at(ref items, 1);   // becomes Flag(false) on LLVM!
    ...
  }
  ```
  C backend: correct (7, true). LLVM backend: `it0` reads as `Num(0)`,
  `it1` reads as `Flag(false)` — every SCALAR payload silently
  zeroed/defaulted. Isolated with a narrowing sequence: no-Vec direct
  match (correct on both) → single clone_at, no loop (broken on
  LLVM) → confirmed the bug is in `clone_at`'s LLVM codegen for
  `Type::Enum` elements specifically.

  **Root cause #1**: `LLVM_ENUM_PAYLOAD_REGISTRY` (populated via
  `decl.payload_types.iter().find_map(|p| p.clone())`) stores the
  FIRST payload type found across a enum's variants, used by
  `clone_at`'s `heap_kind` detection (`Some(Type::OwnedStr) =>
  Some("owned_str"), _ => None`) to decide whether the enum needs
  deep-string-cloning. For `Item`, `Num`'s `i64` payload is declared
  FIRST, so `payload_ty = Some(Type::I64)`, `heap_kind = None` — even
  though `Item` genuinely has an OwnedStr-payloaded variant (`Text`).
  `clone_at` therefore always took the "tag-only" fallback path,
  which round-trips ONLY the tag via `insertvalue {} undef, i32 tag,
  0` and leaves the payload's `[N x i8]` byte-buffer field `undef` —
  discarding the payload for EVERY variant, not just the missed
  OwnedStr one.

  **Root cause #2** (found immediately after fixing #1 — the fix
  newly reached a second, previously-dormant code path): for a
  genuinely mixed-payload-type enum (3+ distinct payload types
  across variants), `%Enum_<Name>`'s field 1 is declared `[N x i8]`
  (a byte buffer sized to the largest variant, see
  `llvm_enum_has_mixed_payloads`/`llvm_enum_payload_buffer_size`) —
  NOT `i8*`. The pre-existing OwnedStr-deep-clone code
  (`extractvalue`/`insertvalue` at the SSA-value level, assuming
  field 1 was always `i8*`) had never actually been exercised for a
  TRULY mixed enum before (bug #1 always routed around it), so its
  type mismatch went undetected until now: `lli` rejected the IR
  with `'%tN' defined with type '[8 x i8]' but expected 'ptr'`.

  Fixed both:
  1. Compute the OwnedStr-tag set directly from the per-variant
     registry (`LLVM_ENUM_VARIANT_PAYLOADS_REGISTRY`, which already
     exists and is correctly per-variant — used elsewhere for enum
     drop dispatch) instead of the single-payload-type registry, so
     detection is correct regardless of variant declaration order,
     and so a scalar-payloaded tag can never be misrouted into the
     string-clone branch (which would have reinterpreted its raw
     bits as an `i8*` and handed that to `intent_str_concat` — an
     out-of-bounds read/crash, worse than the original bug).
  2. Rewrote the deep-clone-as-string path to operate entirely
     through pointers: `alloca` a destination slot, `store` the
     freshly-loaded source value into it whole (this alone already
     preserves every tag's raw payload bytes, scalar or not), then —
     only for a tag matching the OwnedStr set — `getelementptr` +
     `bitcast` BOTH the source and destination payload fields to
     `i8**` (using the correct field-1 type string, `[N x i8]` for a
     mixed enum or `i8*` for a uniform one, computed the same way
     the struct typedef itself is) and overwrite just the destination
     with a freshly-cloned string pointer. This works for both field-1
     representations and eliminates the SSA-value type-mismatch
     entirely, since bitcasts operate on pointers, not aggregate
     values.

  New tests: 1 `src/lib.rs` + 1 `tests/run_end_to_end.rs` (both
  backends, hand-computed expected sum: `7 + 1000 + 2000 + 3000 + 0 =
  6007` across all 4 variant shapes in one `Vec<Item>`). Full `cargo
  test --release --workspace`: 13/13 binaries clean, 0 failed.

## Bug found sweeping "nested if let 2 levels deep on a Vec element" (found+fixed 2026-08-02)

- [x] **BUG-76 (found+fixed 2026-08-02 — same bug class as BUG-29/
  BUG-35, LLVM-only, `Type::Enum` payload never covered).** Found
  while testing the testing matrix's "nested if let 2 levels deep on
  a Vec element" row. A genuinely nested pattern (`if let
  Option.Some(Result.Ok(v)) = ...`) is cleanly rejected at PARSE
  time on both backends, matching the already-documented v1 "no
  nested patterns" limitation — not a bug. Testing the flattened
  two-level form (the doc's own suggested workaround) combined with
  `clone_at` sourcing the outer binding from a `Vec<Option<UserEnum>>`
  found BUG-76, but the minimal repro had nothing to do with any of
  that:
  ```vani
  enum MyResult { Ok(i64), Err(i64) }
  fn main() -> i64 {
    let z: Option<MyResult> = Option.None;   // crashes LLVM
    return 0;
  }
  ```
  Crashed `lli` with `insertvalue %Enum_Option__MyResult %t,
  %Enum_MyResult 0, 1` — "integer constant must have integer type".
  Root cause: constructing a payload-less variant (`None`) of an
  enum whose SIBLING variant carries a payload (`Some(T)`) needs a
  correctly-typed LLVM zero-value placeholder for that unused
  payload slot — a per-`T`-type match already existed for exactly
  this purpose (with two prior fixes documented right there in the
  source: BUG-29 added `Str`, BUG-35 added `Box<T>`/raw pointers,
  both found the same way — a payload-less sibling variant of an
  enum whose OTHER variant carries that type crashed with this exact
  message), but `Type::Enum(_)` was never added to the
  `zeroinitializer` arm (alongside `Vec`/`Tuple`/`Struct`/`Array`/
  `Task`/`Mutex`/`Channel`), so it fell through to the numeric-only
  default (`"0"`) — invalid IR for an aggregate type. This is
  exactly the "not tested for this specific payload type yet"
  pattern BUG-29/35 already both hit, just never re-swept for `T` =
  a user-defined enum until now. Fixed by adding `Type::Enum(_)` to
  the existing `zeroinitializer` arm. New tests: 1 `src/lib.rs` + 1
  `tests/run_end_to_end.rs` (both backends, the full flattened-
  nesting scenario this row was actually testing). Full `cargo test
  --release --workspace`: 13/13 binaries clean, 0 failed.

## Bug found sweeping "extern C fn taking/returning a Struct BY VALUE" (found+fixed 2026-08-02)

- [x] **BUG-77 (found+fixed 2026-08-02 — LLVM-only, System V x86-64
  ABI lowering missing its mirror-image step on the return side).**
  Found while testing the testing matrix's "`extern \"C\" fn` taking
  or returning a Struct BY VALUE" row against a REAL linked C
  function (not just a compile-only declaration check — the existing
  test suite already covered that half). A small struct passed BY
  VALUE as a parameter worked correctly (Closure #288's ABI
  lowering). A small struct RETURNED by value crashed the LLVM
  backend the instant the function was actually called:
  ```vani
  struct Point { x: i32, y: i32 }
  extern "C" fn make_point(x: i32, y: i32) -> Point;
  fn main() -> i64 {
    let p: Point = make_point(3 as i32, 4 as i32);   // crashes LLVM
    ...
  }
  ```
  `llc`/`opt` rejected the IR: `'%t2' defined with type 'i64' but
  expected '%Struct_Point'`. Root cause: the call-site codegen
  correctly used the ABI-lowered return type (`i64`, matching cc's
  actual System V x86-64 calling convention for a small all-integer
  struct) for the `call` instruction itself — but the resulting SSA
  value was then returned to every downstream consumer (a `let`
  binding's `store`, a struct-field read, ...) unchanged, as if it
  already had the real `%Struct_Point` type. The param-passing side
  of this SAME ABI-lowering feature already had the matching "lower
  the value before the call" step (spill to alloca, bitcast to the
  lowered pointer type, load); the return side simply never got the
  mirror-image "un-lower the value after the call" step. C backend
  was unaffected throughout — its own ABI handling is per-value, not
  keyed to this LLVM-specific lowered-type SSA representation.
  Fixed by adding exactly that missing mirror step right after the
  `call` instruction: spill the lowered SSA value to an alloca,
  bitcast the pointer to the real struct type, and load — producing
  a correctly-typed struct value for every downstream use, the same
  way the param-passing side already does it in reverse.
  Confirmed against a REAL linked C shim (`vanic build --link-with`
  for LLVM, `vanic run --backend=c --link-with` for C) exercising
  BOTH directions in one program (a function taking a struct by
  value feeding into one returning a struct by value) — not just a
  compile-only check, which is what let this bug hide behind the
  pre-existing test suite's coverage gap (that suite only ever
  compiled a struct-returning extern declaration, never actually
  called one against a real linked symbol). New tests: 1
  `src/lib.rs` (LLVM IR shape assertion: the un-lowering bitcast
  sequence appears after the call) + 1 `tests/run_end_to_end.rs`
  (both backends, real linked C shim, correct runtime value `7` =
  `3+4`). Full `cargo test --release --workspace`: 13/13 binaries
  clean, 0 failed.

## Bug found sweeping "#[complexity(...)] fn on Vec<Struct>/Array<Tuple,N>" (found+fixed 2026-08-02)

- [x] **BUG-78 (found+fixed 2026-08-02 — C-backend-only declarator
  bug, same class as BUG-61's many follow-ups: a caller of
  `c_leaf_type` that should have routed through `c_element_storage`
  instead).** Found while testing the testing matrix's Big-O row —
  the `--big-o` analyzer itself handled `Vec<Struct>` loops correctly
  (not a bug), but writing a natural Big-O-relevant helper taking a
  fixed-size `Array<Tuple,N>` parameter crashed the C backend
  entirely unrelated to Big-O:
  ```vani
  fn sum_array_tuple(arr: [(i64, i64); 5]) -> i64 {
    return arr[0].0;
  }
  ```
  `cc` rejected the generated C: `unknown type name 'v_arr'` — the
  emitted parameter declaration was `/* tuple */ v_arr[5]`. Root
  cause: `format_declarator`'s `Type::Array { element, length }` arm
  called `c_leaf_type(element)` to spell the element type — but
  `c_leaf_type` is documented as a LEAF-only spelling table:
  Tuple/Struct/Vec/Channel/Mutex/etc. all deliberately return a
  placeholder comment there ("hitting this arm means a caller forgot
  to special-case X"), with `c_element_storage` existing specifically
  to give the correct per-shape spelling for exactly these types (and
  already used correctly elsewhere — struct/tuple fields, `Vec`
  bundle element storage, `emit_array_typedefs_for`'s inner spelling).
  The array-parameter/local declarator path was simply never updated
  to route through it. Same bug also present (found by inspection,
  same fix) in the sibling `Type::Ref(Array)` / `Type::RefMut(Array)`
  arms of the same function, for `ref [Tuple;N]` / `mut ref
  [Struct;N]` parameters. Fixed by changing all three arms to call
  `c_element_storage(element)` instead of `c_leaf_type(element)`.
  LLVM backend was unaffected throughout (its own array-parameter
  lowering already spells element types correctly). New tests: 1
  `src/lib.rs` (asserts no placeholder comment leaks into the
  declarator, and the real `intent_tuple_...`/`Struct_...` names
  appear) + 1 `tests/run_end_to_end.rs` (both backends, hand-computed
  sums over both an `Array<Tuple,N>` and an `Array<Struct,N>`
  parameter). Full `cargo test --release --workspace`: 13/13 binaries
  clean, 0 failed.

## Bug found sweeping "struct with a SIMD Vec128/Vec256 field AND a plain Vec field" (found+fixed 2026-08-02)

- [x] **BUG-79 (found+fixed 2026-08-02 — same "missing arm in
  `c_element_storage`" class as several earlier fixes this session:
  Closure, Channel, Mutex/Guard/RwLock, HashMap all got this exact
  fix before; `vec128`/`vec256`/`vec512` never did).** Found while
  testing the testing matrix's row about a struct holding both a
  SIMD `vec128`/`vec256` field and a plain `Vec` field — the row's
  own hypothesis (a helper-naming collision between the two Vec
  families) turned out not to be the actual bug:
  ```vani
  struct Combo { lane: vec128<f64>, xs: Vec<f64> }
  ```
  `cc` rejected the generated C: `expected specifier-qualifier-list
  before 'lane'` — the field declared itself as `/* vec128<T> */
  lane;`. Root cause: `c_element_storage` — the function that gives
  struct fields, Vec elements, and similar positions their REAL
  per-shape C type spelling, specifically so callers don't fall back
  to `c_leaf_type`'s deliberate placeholder-comment stubs for
  aggregate types ("hitting this arm means a caller forgot to
  special-case X") — simply never had arms for `Type::Vec128`,
  `Type::Vec256`, or `Type::Vec512`. The real spelling helpers
  (`c_vec128_type`/`c_vec256_type`/`c_vec512_type`, producing the
  GCC/Clang `__attribute__((vector_size(N)))` GNU vector-extension
  type) already existed and were already used correctly for the
  LOCAL-variable case (`let v: vec128<f64> = ...;`) — this was
  purely a missing arm in the struct-field/general "element storage"
  path, not a design gap. Fixed by adding the three missing arms.
  LLVM backend was unaffected throughout (its own field-type lowering
  already handled these types correctly). New tests: 1 `src/lib.rs`
  (placeholder-comment absence + real vector-extension type
  assertion) + 1 `tests/run_end_to_end.rs` (both backends,
  hand-computed SIMD `reduce_add` + plain `Vec` field values,
  vec128 AND vec256 in the same program). Full `cargo test --release
  --workspace`: 13/13 binaries clean, 0 failed.

## Bug found sweeping "Option<Array<T,N>>/Result<Tuple,E>" (found+fixed 2026-08-02)

- [x] **BUG-80 (found+fixed 2026-08-02 — two layered C-backend-only
  bugs: wrong match-arm binding type spelling, then a genuine "C
  arrays aren't assignable" language constraint).** Found while
  testing the testing matrix's `Option<Array<T,N>>`/`Result<Tuple,E>`
  row. `Result<(i64,i64), i64>` already worked correctly on both
  backends (Tuple's match-arm binding path was already correct).
  `Option<[i64; 3]>` crashed the C backend (LLVM was correct
  throughout):
  ```vani
  fn maybe_arr(has: bool) -> Option<[i64; 3]> {
    if has { return Option.Some([1, 2, 3]); }
    return Option.None;
  }
  fn main() -> i64 {
    let oa: Option<[i64; 3]> = maybe_arr(true);
    let total: i64 = match oa {
      Option.Some(arr) then arr[0] + arr[1] + arr[2],   // crashes C
      Option.None then 0 - 999,
    };
    ...
  }
  ```
  1. **Wrong type spelling for the match-arm local.** The codegen
     that declares a match arm's payload binding
     (`{TYPE} v_arr = __scr.payload;`) called `c_type_name(bty)` for
     `TYPE`. `c_type_name`'s `Type::Array` arm deliberately spells an
     array as the RETURN-POSITION wrapper struct
     (`intent_arr_ret_<N>_<T>`, Closure #239) — its own doc comment
     already says as much: "the Let path passes through
     `format_declarator` instead so the array declarator form keeps
     working for locals." This match-arm binding IS exactly such a
     Let-like local-binding position, but was never routed through
     the correct helper. Symptom: `unknown type name
     'intent_arr_ret_3_int64_t'` (the wrapper typedef isn't even
     emitted for this position), and even if it had been, it's a
     struct (`{ T data[N]; }`), so `v_arr[0]` wouldn't subscript
     correctly either way ("subscripted value is neither array nor
     pointer nor vector").
  2. **C arrays can't be copy-assigned (found immediately after fixing
     #1).** Switching to `c_element_storage` (the correct per-shape
     helper already used for every OTHER payload shape at this call
     site — Tuple, Struct, Vec, Closure, ...) fixed the type NAME
     (`intent_arr3_int64_t`) but exposed a second, more fundamental
     issue: C arrays — even through a raw-array typedef alias — can't
     be initialized via plain `=` assignment at all ("invalid
     initializer"; this is a real C-language constraint, not a naming
     bug). Fixed by declaring the binding as a POINTER to the element
     type instead (`int64_t* v_arr = __scr.payload;`) — the raw array
     struct member naturally array-decays to a pointer to its first
     element in this expression context (valid C), and `v_arr[0]`/
     etc. in the arm body keeps working completely unchanged (pointer
     subscripting uses the identical syntax to array subscripting).
  3. **Typedef-emission gap (a narrower repeat of BUG-74's class).**
     Even with both codegen fixes, `intent_arr3_int64_t` still wasn't
     declared anywhere in the file — BUG-74's fix walked `tuple_shapes`
     for nested `Array`-inside-`Tuple` payloads, but an enum payload
     that's DIRECTLY `Array<T,N>` (no Tuple wrapper) was never fed
     into the array-typedef pass at all. Fixed by walking
     `program.enums`'s payload types directly through the existing
     `emit_array_typedefs_for` pass (which already recurses correctly
     once fed the right root type).
  New tests: 1 `src/lib.rs` + 1 `tests/run_end_to_end.rs` (both
  backends, `Option<Array>` Some/None AND `Result<Tuple>` Ok/Err, all
  four paths, hand-computed values). Full `cargo test --release
  --workspace`: 13/13 binaries clean, 0 failed.

---

## Clearing the backlog of previously-deferred open bugs (2026-08-02)

User asked for a prioritized list of every open item still on the
books outside the BUG-68..80 sweep, approved items 1-5, ordered
cheapest/lowest-risk first. BUG-33 (SMT `let`-then-`return` proof gap)
was listed as still-open in that prioritization pass, deliberately
left for its own dedicated session given soundness risk — this was a
mistake in the audit that produced the list: BUG-33's own entry above
already carries a "✅ Fixed 2026-08-01" resolution (commit `304c922`),
predating this session entirely. Re-verified directly (2026-08-02):
the entry's exact repro (`let r: i64 = n * 2; return r;` with
`ensures _return == n * 2;`) proves cleanly, and a genuinely wrong
`ensures` on the same shape is still correctly rejected — no work
needed, BUG-33 was never actually open this session. BUG-36 (`mut ref`
exclusivity — missing checker subsystem) remains genuinely open and
was correctly deferred to its own dedicated session.

- [x] **DOC-4 / DOC-5 — already fixed, stale bookkeeping only.** Both
  tutorial files (`intermediate/03b_affine_deeper_primer.md`'s
  `maybe_consume` example, `beginner/02_variables.md`'s Challenge
  section) already carried the corrected examples from a prior pass —
  re-verified directly against the compiler (`vanic check`): the
  `maybe_consume` shape without an early return in the moving branch
  is correctly rejected; `u32 * i64` widens cleanly while `u32 * i32`
  (same width, mismatched sign) is correctly rejected. The "not yet
  fixed, still open" note near BUG-13/14/15/18 was simply never
  updated after the fix landed. Corrected that note in place.

- [x] **BUG-22 residual — already fixed, stale bookkeeping only.**
  The struct/enum RwLock/Mutex payload struct-definition-ordering gap
  noted as "attempted and reverted" in BUG-19's entry was, per BUG-22's
  own (later, more complete) entry, fully fixed the very next day
  (2026-07-28). Re-verified directly: `RwLock<Point>`/`Mutex<Point>`
  struct payloads both compile and run correctly on `--backend=c`,
  matching LLVM (read/write/release cycle, guard_get/guard_set).
  Corrected the stale note in place; no code change needed.

- [x] **BUG-20 residual — real bug, fixed.** Three adjacent gaps left
  over from BUG-20's original fix (which only wired `arm.guard` into
  `check_match_slice`'s `Slice` arm): (1) that same function's
  `Wildcard` arm never read `arm.guard` at all — a guarded slice/array
  wildcard always behaved as an unconditional catch-all; (2)
  `check_match_str` never type-checked or wired `arm.guard` into its
  generated dispatch at all; (3) `check_match_float` had the identical
  gap. All three are the "compiles fine, silently produces the wrong
  answer" class of bug.
  - Slice `Wildcard`: mirrors the M3 int/bool/enum dispatch precedent
    — a guarded wildcard does NOT close off later arms (`wildcard_seen`
    stays false) and its guard becomes an ordinary conditional entry in
    the dispatch chain (`typed_arms.push((guard_expr, body))`) instead
    of the unconditional fallback.
  - `check_match_str` / `check_match_float`: both fold arms right-to-
    left into a nested `IfExpr` chain; since string/float patterns bind
    no names (unlike slice patterns), the guard has no dependency on
    the pattern having matched, so it folds in with an eager `&&`
    (`cond = (scr == lit) && guard`) rather than needing the slice
    fix's "gate guard eval behind the length check first" structure.
  - 7 new `src/lib.rs` tests (compile-time acceptance + guard-must-be-
    Bool rejection for all three shapes) + 3 new `tests/run_end_to_end.rs`
    execution tests (real stdout on both backends — this class of bug
    compiles cleanly and only produces wrong output at runtime, so a
    compile-only test can't catch it). All pre-existing `match_`-
    filtered tests (137) and the two prior BUG-20/28 execution tests
    (`slice_pattern_guards_example_...`, `match_guard_chain_example_...`)
    still pass.

- [x] **BUG-66 residual — real bug, fixed as a clean rejection (not a
  "make it work" fix).** The deferred gap from BUG-66's original fix:
  a closure with a HEAP-OWNING capture (moved in, not `ref`-captured)
  stored into a struct field crashed both backends — LLVM: `lli`
  rejects the emitted IR ("base element of getelementptr must be
  sized" — the synthesized env struct is referenced as an opaque/
  unsized type at the point the closure is stored into and read back
  from the struct field); C: `free(): double free detected in tcache
  2` at runtime. This is an affine-ownership/lifetime gap (the
  closure's heap-owning env crossing a struct-field boundary), not a
  missing typedef — implementing real cross-struct-field lifetime
  tracking for an affine closure env is a substantial new feature, not
  a quick fix, so (matching the BUG-64 Channel-Copy-requirement
  precedent named in the original deferral) the fix is a clean
  checker-time rejection instead of an attempt to make the pattern
  sound.
  New `reject_affine_closure_into_struct_field` helper (`checker.rs`):
  narrow, name-based check using the pre-existing `CLOSURE_AFF_REGISTRY`
  (keyed by a closure literal's own bind name, populated when the
  literal itself is checked — `Type::Closure` has no Copy/affine
  distinction in the type system itself, so this out-of-band registry
  is the only place that information lives). If a struct-literal
  field's declared type is `Closure` and its RHS is a bare `Var(name)`
  found in the registry, reject with a diagnostic explaining the
  capture-by-move + struct-field-boundary problem and pointing at the
  `ref`-capture / pass-as-argument workarounds. Wired into both
  struct-literal field-checking and `Stmt::FieldAssign` (`obj.field =
  value;`) — the two places a closure value can be stored into a
  struct field. Deliberately narrow: only catches the direct
  `Struct { field: closure_var }` / `obj.field = closure_var;` shape
  (matching the original bug report and the tutorial's own worked
  example), not renamed/re-bound aliases of the closure — full alias
  tracking would be a broader mechanism than this targeted fix.
  Verified: the exact original repro (`Handler { cb: Vec-capturing
  closure }`) now gets a clean `vanic check` rejection on both
  backends instead of crashing; the pre-existing Copy-only-capture
  case (`struct_field_closure_with_copy_only_capture_...`) still
  compiles and runs correctly (105 = 100 + 5) — confirming the new
  check doesn't over-reject. 2 new `src/lib.rs` tests (struct-literal
  shape + field-assign shape) + 1 new `tests/run_end_to_end.rs` test
  (real `vanic check` invocation, confirms the diagnostic text). 71
  closure/struct-field-filtered `cargo test --lib` tests pass, 0
  regressions.

- [x] **BUG-36 (fixed 2026-08-02 -- the missing checker subsystem,
  attempted despite being flagged as highest-risk/dedicated-session
  material).** The "single mutable borrow" exclusivity rule (`ref`
  can multiply, `mut ref` must be exclusive) had NO enforcement at
  all -- only affine move tracking existed. `let r: mut ref Vec<i64>
  = mut ref xs; push(r, 4); print xs[0];` compiled and ran cleanly on
  both backends with no diagnostic.
  **Design, deliberately narrow to keep the false-rejection risk
  low**: a lexical (NOT full non-lexical-lifetime) approximation,
  scoped only to NAMED `let`-bound `ref`/`mut ref` bindings:
  - A tracked borrow's "lifetime" is exactly its owning binding's own
    lexical scope (`env.scopes` in `checker.rs` only contains
    bindings whose enclosing block hasn't exited yet, so this falls
    out for free from the existing scope-stack machinery -- no new
    "borrow end" bookkeeping needed).
  - An inline `foo(mut ref xs)` call argument is NEVER stored in
    `env` (confirmed by reading `check_ref_mut`: a `TypedExprKind::
    RefMut` value passed directly as a call argument doesn't go
    through `Stmt::Let`), so it's structurally invisible to this
    check -- exactly matching the tutorials' own pre-existing
    documented model ("the compiler doesn't track them across the
    call -- once the call returns, the borrow ends"). This is the
    single biggest reason the false-rejection risk stayed low: the
    dangerous, trackable shape (a persisted named binding) and the
    safe, untracked shape (an ephemeral call-argument borrow) are
    already syntactically distinct in the AST.
  - Reuses the pre-existing `ref_aliases` field on `VarInfo`
    (populated by `compute_ref_aliases_from_let_rhs`, built for the
    unrelated L4 scope-escape checks) rather than adding new state --
    a live `Ref`/`RefMut`-typed binding whose `ref_aliases` contains
    `target` IS the borrow. Two new helpers, `find_live_mut_borrow_of`
    / `find_live_borrow_of` (`checker.rs`), scan all open scopes for
    a conflicting live borrow.
  **Enforcement points** (four call sites, all reusing the two
  helpers above): (1) `check_expr`'s `ExprKind::Var` read arm --
  reading a binding directly while a `mut ref` of it is live is
  rejected (this alone also covers `Stmt::FieldAssign`'s writes for
  free, since that handler already routes its `object` expression
  through `check_expr`); (2) `Stmt::Assign` -- a direct `x = ...;`
  write is rejected the same way, gated on `!existing.ty.is_any_ref()`
  so writing THROUGH a ref binding itself is correctly exempt; (3)
  `Stmt::IndexAssign` -- `xs[i] = v;` rejected when `!through_ref`
  (mirrors (2)); (4) `check_ref` / `check_ref_mut`'s bare-`Var`
  branches -- creating a NEW `ref`/`mut ref` of an already-borrowed
  binding is rejected at the creation site itself, for an earlier,
  clearer diagnostic than waiting for the first conflicting access.
  **Soundness follow-up found and fixed in the same pass**:
  `Stmt::Assign` reassigning a ref-typed LOCAL to a new source
  (`r = mut ref ys;`) previously left `ref_aliases` stale (confirmed
  this shape already type-checked before this fix, so it's a real,
  reachable gap) -- would have both incorrectly kept the OLD target
  locked forever (usability bug, still sound) AND, the actual
  soundness gap, never recognized the NEW target as borrowed at all.
  Fixed by recomputing `ref_aliases` via the same
  `compute_ref_aliases_from_let_rhs` the `Let` path already uses,
  whenever the reassigned binding's own type is a ref.
  **Verification, given the explicit "could introduce false
  rejections across a huge amount of existing working code" risk
  flagged when this was first deferred**: (1) a full `cargo test
  --release --workspace` run, 0 failures; (2) an exhaustive
  before/after `vanic check` sweep of all 1034 `.vani` files under
  `examples/` (stashed the fix, built a baseline binary, ran `check`
  on every file, restored the fix, rebuilt, ran `check` on every file
  again, `diff`'d the two pass/fail sets) -- **byte-identical, zero
  differences** across the entire corpus; (3) directly extracted and
  re-verified the two tutorial code blocks using a NAMED `mut ref`
  binding found by grepping every tutorial for the pattern
  (`intermediate/03d_cyclic_references_primer.md`'s `register`/`sub`
  pattern -- confirmed unaffected, since nothing reads the borrowed
  binding directly after the named borrow is taken; and
  `intermediate/03b_affine_deeper_primer.md`'s own repro, which is
  the bug itself and is now correctly rejected).
  **Docs updated**: `03b_affine_deeper_primer.md`'s "Borrow scopes"
  section (previously documented the gap as "as of this writing, not
  enforced" -- now shows the real enforced diagnostic and explains
  the lexical/named-binding-only scope precisely), its "two-way
  trade" section and summary bullet (both previously downplayed the
  exclusivity rule as unenforced design-intent -- now state the real,
  current enforcement boundary).
  New tests: 10 checker-level (`src/lib.rs` -- read/write/reassign/
  index-assign violations rejected; second-borrow-while-live rejected
  both directions shared-then-mut and mut-then-shared; inline call-
  argument borrow correctly NOT tracked; multiple named shared refs
  coexist; scope-ended borrow correctly releases; reassigning a ref
  binding correctly retargets the tracked alias) + 2 end-to-end
  (`tests/run_end_to_end.rs` -- real `vanic check`/`run` invocations,
  one confirming the clean rejection, one confirming a legitimate
  scope-bounded usage still compiles and runs correctly on both
  backends). Full `cargo test --release --workspace` after the tests
  landed: 13/13 binaries clean, 0 failed (2688 lib tests, up from
  2678; 139 end-to-end tests, up from 137).
  **Known, deliberately-accepted residual gap** (documented in the
  new `find_live_mut_borrow_of` doc comment and in the updated
  tutorial section): this is a lexical approximation, not real
  liveness analysis -- a borrow is considered live for the REST of
  its declaring scope even past its actual last use, and only NAMED
  bindings are tracked at all. Anything beyond that (interprocedural
  aliasing, non-lexical patterns) remains unchecked exactly as
  before this fix -- a false negative, never a false positive, which
  is the required direction for a checker that must never reject
  sound code.

## Feature-combination gap audit sweep (2026-08-03)

Working through `docs/FEATURE_COMBINATION_GAPS_TODO.md` (49-row TODO
created 2026-08-02 for exactly this purpose). User authorized fully
autonomous operation ("proceed without any inputs from me... automate
and fix automatically without asking") for this pass.

- [x] **BUG-81 (found+fixed 2026-08-03 -- two independent bugs, one
  per backend, same "container element with its own codegen path
  forgot this case" class as BUG-61/79). `Vec<vec128<T>>` (a SIMD
  lane type as a Vec ELEMENT, not a struct field) crashed both
  backends.** Category 1, row 1 of the gap audit -- the top-priority
  row, flagged in advance as the closest unswept analog of BUG-61.
  - **C backend**: `element_tag` (`backend_c.rs`) -- the function
    that names Vec BUNDLE typedefs/helpers -- is a SEPARATE function
    from `c_element_storage` (which BUG-79 already fixed for the
    struct-FIELD case) and never got the matching arm. Its `_ =>
    c_leaf_type(element).replace(' ', "_")` fallback returned
    `c_leaf_type`'s placeholder comment `"/* vec128<T> */"`, and the
    space-replace turned it into `/*_vec128<T>_*/` -- embedded into
    every generated `intent_vec_<tag>__*` identifier, corrupting the
    entire bundle. `cc` rejected the output with a cascade of
    "expected '=', ',', ';'..." errors, one per corrupted identifier.
    Fixed by adding explicit `Vec128`/`Vec256`/`Vec512` arms
    (recursive composition, mirroring the Atomic/Channel/Box arms
    already in the same function).
  - **LLVM backend, layered**: (1) `vec_struct_tag` (the LLVM analog
    of `element_tag`) had the identical missing-arm gap, causing a
    Rust panic on compile ("llvm_type: use llvm_type_string for
    aggregate / ref type Vec128(F64)") -- fixed the same way. (2)
    Fixing that alone revealed a SECOND, more severe bug underneath:
    `vec_element_byte_size` (drives the Vec's malloc/realloc SIZE
    calculation for push/growth, a sibling of the already-correct
    `llvm_byte_size`) has a final fallback `element.bits().
    unwrap_or(64) / 8` -- `Type::bits()` returns `None` for SIMD lane
    types (they're not in its bits-classified match), so this
    silently computed 8 bytes for what's actually a 16-byte
    `vec128`/32-byte `vec256`/64-byte `vec512` register --
    under-allocating the buffer by half to 1/8th of what it needs.
    Confirmed via the exact failure signature every prior BUG-6x
    under-allocation fix describes: `realloc(): invalid next size` at
    runtime, corrupting the heap on the second element's `push`.
    Fixed by adding explicit 16/32/64-byte arms, matching
    `llvm_byte_size`'s already-correct constants.
  - **Verification**: beyond the usual both-backend stdout check, ran
    `valgrind --leak-check=full` against a native AOT LLVM build
    (`vanic build ... -lm`, since `--backend=c`/JIT paths don't
    produce a persistent native binary for LLVM the same way) --
    0 errors, all heap blocks freed, confirming the fix isn't just
    "happened not to crash this time" on a heap-layout-dependent bug.
  - Also swept the REST of category 1 (rows 2-8: `Array<vec128,N>`,
    `struct { Vec<vec256<T>> }`, `(vec128<T>, i64)` tuple, generic
    struct at `T=vec128<T>`, `Option<vec128<T>>`, `HashMap<i64,
    vec128<T>>`, `clone_at` on `Vec<Struct>` with a SIMD field) --
    all correct on both backends except `HashMap<i64, vec128<T>>`,
    which is a clean, consistent rejection matching the documented
    "hashmap_insert() supports scalar V in v1" restriction (not a
    bug). New tests: 8 `src/lib.rs` (one per row) + 2
    `tests/run_end_to_end.rs` (real stdout, both backends -- one for
    the bug itself, one covering the array/generic-wrapper rows).

- [x] **BUG-82 (found+fixed 2026-08-03, same sweep, LLVM-only). While
  testing category 1's `Option<vec128<f64>>`/`Result<vec128<f64>,
  E>` row, `Result<vec128<f64>, i64>` -- a MIXED-payload-type enum
  (unlike `Option<vec128<f64>>`, which has only ONE payloaded
  variant) -- segfaulted `lli` with no diagnostic, on both
  construction and match-arm extraction.** Root cause: mixed-payload
  enums store their payload in an `{i32, [N x i8]}` byte buffer;
  reading/writing the real payload type through it requires a
  `bitcast i8* ... to <payload_llvm_type>*` then load/store. Neither
  the construction site (`TypedExprKind::EnumVariantWithPayload`) nor
  the match-arm extraction site had an explicit `align` on that
  load/store -- LLVM defaults to the pointee type's ABI alignment
  (16 bytes for `<2 x double>`/vec128, more for vec256/512), but the
  buffer itself only guarantees 4-byte alignment (from the struct's
  leading `i32` tag) -- an ALIGNED SSE/AVX move against actually-
  unaligned memory is a hard segfault, not a compile error. `Option
  <T>` never hits this: single-payload-type enums use
  `insertvalue`/`extractvalue` directly on the SSA struct, never
  touching the byte buffer. Fixed by adding an explicit `align 1` to
  both sites -- unaligned move instructions are always correct
  regardless of the buffer's real alignment, for every payload type
  this path handles, not just SIMD ones (a strictly safer, simpler
  fix than trying to compute and thread through the real per-enum
  alignment requirement). Verified with `valgrind --leak-check=full`
  on a native AOT LLVM build covering BOTH variants (`Result.Ok` and
  `Result.Err`): 0 errors, all heap blocks freed. New tests: 1
  `src/lib.rs` + 1 `tests/run_end_to_end.rs` (real stdout, both
  variants exercised, both backends -- this is a RUNTIME crash a
  compile-only test can't catch, since LLVM compilation itself
  succeeded before the fix).

Full `cargo test --release --workspace` after both BUG-81 and BUG-82
landed: 13/13 binaries clean, 0 failed (2697 lib tests, up from 2688;
142 end-to-end tests, up from 139). Category 1 (all 8 rows) now fully
closed in `docs/FEATURE_COMBINATION_GAPS_TODO.md`.

- [x] **BUG-83 (found+fixed 2026-08-03 -- two layered bugs, plus a
  self-inflicted regression caught and fixed in the same pass).
  Category 2, row 1: `struct Cache<T> { lock: Mutex<T> }` -- a
  struct field holding a concurrency-handle type that's ONLY ever
  used through that field (never as a bare local/param elsewhere in
  the program) -- crashed both backends.** Root cause:
  `collect_mutex_specs`/`collect_rwlock_specs`/`collect_channel_specs`
  (the passes that discover which concrete T's need a bundle
  emitted) only ever recursed into `Vec`/`Atomic`/`Array`/`Ref`/
  `RefMut` -- never a nominal struct's OWN field types. The pre-
  existing `struct_field_mutex_alongside_vec_field_...` test never
  caught this: it ALSO declares a bare `let m: Mutex<i64> = ...;`
  elsewhere, which the existing `TypedStmt::Let` arm already
  discovers directly, masking the gap. Confirmed via `cc`: "implicit
  declaration of function 'intent_mutex_int64_t_new'" -- the bundle
  was simply never emitted. Fixed by adding a `Type::Struct(name)`
  arm to all three collector functions.
  **LLVM-specific second layer**: these three collectors live in
  `backend_c.rs` and are directly REUSED by `backend_llvm.rs` (no
  duplicate LLVM versions) -- but each backend populates its own
  independent struct-fields registry (`backend_c::
  STRUCT_FIELDS_REGISTRY` vs. `backend_llvm::
  LLVM_STRUCT_FIELDS_REGISTRY`), and only one is ever populated per
  compile run. The initial fix (reading only the C-side registry)
  made C correct while LLVM kept failing ("Cannot allocate unsized
  type" -- the mutex struct type was silently never emitted). Fixed
  with a new `lookup_struct_fields_any_backend` helper that checks
  both registries.
  **Self-inflicted regression, found verifying the fix and fixed in
  the same pass**: the `Type::Struct` recursion assumed a struct's
  field graph is always a DAG -- true for BY-VALUE nesting (can't be
  infinite-sized), false for `Vec<Self>` (legal and common --
  `struct Node { children: Vec<Node> }`, the shape every tree/
  recursive-walk example needs, since `Vec` is pointer-indirected).
  Without a cycle guard, `collect_mutex_specs(Struct(Node))` walked
  Node's `Vec<Node>` field straight back into itself, forever --
  confirmed as a REAL stack overflow: the pre-existing, pinned
  `self_referential_struct_vec_example_produces_correct_output_on_
  c_backend` end-to-end test (a regression test for a DIFFERENT,
  already-fixed bug, BUG-31) started crashing with "thread 'main' has
  overflowed its stack" the moment the naive fix landed. Fixed with a
  new `STRUCT_RECURSION_GUARD` thread-local tracking which struct
  names are currently being expanded on the current walk -- already-
  in-progress structs are skipped rather than re-entered.
  Also found a completely separate, general (not struct/generic-
  specific) bug while testing `Cache<bool>` (the second monomorphic
  instantiation) alongside `Cache<i64>` -- logged as BUG-84 below.
  Category 2 rows 2 and 4 (a generic function constructing a
  `Mutex<T>` from its own type parameter; the task-capture Copy
  check applying correctly PER MONOMORPHIZATION, accepting T=i64 and
  rejecting T=OwnedStr within the same program) both checked clean,
  no bug found. Row 3 (`Task<T>` as a generic function's return type)
  is blocked by the pre-existing, correctly-enforced "spawn and join
  must be in the same block" v1 architectural limitation -- applies
  identically regardless of generics, not a new finding.
  New tests: 6 `src/lib.rs` (struct-field discovery both directions,
  the recursion-guard regression, the generic-fn-constructs-lock row,
  both directions of the task-capture-Copy-check row) + 1
  `tests/run_end_to_end.rs` (real stdout, both `Cache<i64>`/
  `Cache<bool>` instantiations, both backends).

- [x] **BUG-84 (found+fixed 2026-08-03, same pass, LLVM-only).
  `Mutex<bool>` -- ANY `Mutex<bool>`, confirmed with a bare top-
  level one too, not just the struct-field/generic case that
  surfaced it -- crashed `lli` with a type-mismatch verifier error
  ("defined with type 'i8' but expected 'i1'").** Root cause:
  `Mutex<bool>`'s payload is stored as `i8` (the same `Atomic<bool>`
  shadow-storage trick `atomic_storage_llvm` already uses -- `i1`
  isn't byte-addressable), but `guard_get`'s codegen returned the
  raw loaded `i8` value directly instead of converting it back to
  `i1`; `guard_set` had the mirror gap on the write side. Fixed by
  mirroring `atomic_load`/`atomic_store`'s existing Bool handling
  (`icmp ne i8 X, 0` for the read direction -- not `trunc`, which
  would only look at the low bit; `zext i1 to i8` for the write
  direction).
  Verified (both BUG-83 and BUG-84) with `valgrind --leak-check=full`
  on a native AOT LLVM build: 0 errors, all heap blocks freed.

Full `cargo test --release --workspace` after BUG-83/84 and the
recursion-guard fix landed: 13/13 binaries clean, 0 failed (2703 lib
tests, up from 2697; 143 end-to-end tests, up from 142). Category 2
(all 4 rows) now fully closed in
`docs/FEATURE_COMBINATION_GAPS_TODO.md`.

- [x] **BUG-85 (found+fixed 2026-08-03, SSA-C only). Category 3, row 3
  ("invariant with task/Mutex in loop"): investigating this row
  required a BARE, SSA-eligible `Mutex<i64>` (no structs, no block
  expressions -- both of which force the tree backend) for the first
  time all session. It failed to compile on the C backend at all.**
  Root cause: `ssa_backend_c.rs` has its own, entirely SEPARATE
  `mutex_new`/`mutex_lock`/`guard_get`/`guard_set` implementation
  (and its own `c_declarator` type-spelling function) from the TREE
  emitter in `backend_c.rs` -- and this SSA-specific copy was
  hardcoded to the literal name `intent_mutex_i64`/`intent_guard_i64`,
  stale since BUG-19 (2026-07-27) made the preamble bundle ALWAYS use
  the parametric name (`intent_mutex_int64_t`, even for the plain i64
  case) on both backends. The tree emitter was updated at the time;
  this sibling SSA implementation never was -- nobody had run a
  purely-SSA-eligible Mutex program through the real C backend
  end-to-end before. `cc` rejected the output: "implicit declaration
  of function 'intent_mutex_i64_new'". Fixed by routing all four
  builtins plus the six `c_declarator` arms (bare/`&`/`&mut` x
  Mutex/Guard) through `c_mutex_storage`/`c_guard_storage`
  (extracting the real element type from `instr.ty` or the argument's
  type via `value_types`), mirroring the pattern
  `channel_new`/`channel_send`/`channel_recv` already used in the same
  file.

- [x] **BUG-86 (found+fixed 2026-08-03, same investigation, tree-C
  only -- a REAL SILENT DEADLOCK, not a compile failure). Once
  BUG-85's fix made the bare Mutex compile, testing TWO SEQUENTIAL
  (non-overlapping) lock/unlock cycles on the same mutex through a
  block-expression (the tutorial's own idiom, `let v: i64 = { let g =
  mutex_lock(ref m); guard_get(ref g) };`) hung FOREVER on the second
  `mutex_lock` call.** Root cause: the block-expression-specific
  `TypedStmt::Drop` emitter in `backend_c.rs` -- a completely separate,
  incomplete reimplementation of the correct TOP-LEVEL statement Drop
  emitter a few thousand lines earlier in the same file -- has
  explicit arms for OwnedStr/Vec/Struct/Enum but NONE for
  Guard/ReadGuard/WriteGuard; it silently fell through to a `_ => {}`
  no-op, so the guard's RAII unlock never fired. The first lock was
  never released, so the second `mutex_lock` spun forever waiting for
  a lock that (from the runtime's point of view) was still
  legitimately held -- no diagnostic, no crash, just a hang: the
  worst possible failure mode for a concurrency primitive. Confirmed
  this is TREE-C-only (LLVM's block-expression codegen handles this
  correctly already) and confirmed PRE-EXISTING, not a session
  regression: reproduced identically against commit `08f38c2` (the
  last commit before this gap-audit sweep even started), verified in
  an isolated `git worktree` so the main working tree was never at
  risk. Fixed by adding the three missing arms, mirroring the
  already-correct top-level statement Drop handler exactly.
  **Verification, given the severity (silent deadlock)**: real
  execution on both backends (not just compile success) for the
  tutorial-verbatim single-lock-cycle case AND a two-sequential-
  cycles case; `valgrind --leak-check=full` on BOTH a native AOT LLVM
  build and a directly-`cc`-compiled native binary of the generated C
  (0 errors, all heap blocks freed, on both). The new
  `tests/run_end_to_end.rs` regression test wraps the sequential-
  cycles invocation in the real `timeout` command specifically so a
  FUTURE regression of this exact bug fails the test (and CI) after
  20s instead of hanging the test suite itself forever.
  Also swept the rest of category 3 (SMT contracts x generics/
  concurrency/enums): a generic function's `requires`/`ensures`
  proves correctly per-monomorphization (scalar T) and cleanly
  rejects (non-scalar T, matching BUG-68's fix) rather than silently
  skipping; `--big-o` analysis and SMT verification coexist correctly
  on the same function with no interference in either direction; an
  `ensures`/`prove` referencing an enum's variant tag directly, or a
  `dyn Iface` method's return value, are both cleanly and
  consistently rejected on both backends (`Eq`-required / "method
  calls not supported in SMT v1" respectively) -- matching the
  documented v1 SMT boundary, not bugs.
  New tests: 2 `src/lib.rs` (SSA-C naming, block-expr guard-unlock
  presence) + 2 `tests/run_end_to_end.rs` (tutorial-verbatim single
  cycle; sequential two-cycle deadlock guard with the `timeout`
  wrapper). Full `cargo test --release --workspace`: 13/13 binaries
  clean, 0 failed (2705 lib tests, up from 2703; 145 end-to-end
  tests, up from 143). Category 3 (all 5 rows) now fully closed in
  `docs/FEATURE_COMBINATION_GAPS_TODO.md`.

- [x] **BUG-87 (found 2026-08-03, deferred, then FIXED 2026-08-04 --
  task #42; see the fix writeup right after this entry). Category 4,
  rows 1-2: `async fn` combined with generics or a built-in generic
  enum return type is broken -- two related symptoms, one root cause
  EACH (they turned out not to actually share a root cause -- see
  the fix).**
  - **Row 1, `async fn identity<T>(x: T) -> T`**: calling it directly
    inside `await(identity(42))` fails monomorphization outright
    ("generic function 'identity' is declared but never called with
    concrete types") -- the call-site scanner that discovers which
    concrete `T`'s a generic function is used at doesn't look inside
    `await(...)`'s argument expression. Pre-extracting to a `let f =
    identity(42);` first works around THIS half (monomorphization
    then succeeds, correctly producing `Future__i64`) but hits a
    SECOND bug: `await(f)`'s desugared match ("match scrutinee must
    be an enum, integer, or bool type, got Future__i64") --
    `Future__i64` prints identically to a real registered enum name
    but isn't actually resolving to `Type::Enum("Future__i64")` by
    the time the match-dispatch check runs, strongly suggesting
    `Future<T>`'s return type stays as an unresolved `Type::Apply`
    wrapper (built directly in `parser.rs`'s async desugar, `name:
    "Future".to_string()`) rather than going through the SAME
    generic-enum monomorphization pipeline `Option<T>`/`Result<T,E>`
    use (the one BUG-46 already fixed for bare-name resolution) --
    `Future<T>` appears to be a special-cased, parallel mechanism
    that was never wired into that pipeline at all.
  - **Row 2, `async fn maybe_get(n: i64) -> Option<i64>`**: same root
    cause, different surface -- "match arm body has type i64 but
    earlier arm produced Option__i64" when `await`'s own desugared
    match interacts with a user `match` over the awaited
    `Option<i64>` result.
  - **Row 3, `async fn` + `requires`/`ensures`**: NOT the same bug --
    a clean, SAFE rejection ("cannot verify 'ensures' clause: method
    calls not supported in SMT v1"), because `_return` for an async
    fn is the desugared `Future.Ready(expr)` constructor call, which
    reads as a method call to the SMT encoder. This is a real
    functional limitation (SMT contracts don't work on `async fn` at
    all today) but not a soundness bug -- matches BUG-68's "unverifiable
    means rejected, never silently accepted" fix.
  - **Rows 4-5, checked clean**: `async fn` taking a plain `fn(T) ->
    R` function-pointer parameter (called before AND after the
    `await`) computes correctly on both backends; `async fn` that
    internally spawns a `task`/`join`s it (a DIFFERENT concurrency
    primitive family, both inside the single synchronous-desugar
    function body) also computes correctly on both backends.
  - **Why not fixed this session**: properly fixing rows 1-2 requires
    either (a) routing `Future<T>`'s `Type::Apply` construction
    through the same monomorphization/enum-registration pipeline
    `Option<T>`/`Result<T,E>` already use, or (b) teaching the
    call-site scanner and match-dispatch-kind check to specially
    recognize `Type::Apply { name: "Future", .. }` and resolve it the
    way `Type::Enum` already is. Either touches the async desugar
    (`parser.rs`) and/or generic monomorphization (`checker.rs`)
    under time pressure, in code this session's own BUG-45 entry
    already flagged as "new, sensitive, partially-shipped machinery"
    where "a wrong fix risks silently breaking the cases that DO work
    today." Whoever picks this up next: start by checking whether
    `Future<T>`'s `Type::Apply` ever gets monomorphized into a real
    `Type::Enum(name)` ANYWHERE in the pipeline for a non-scalar `R`/
    generic `T` (the doc's own worked examples only ever use scalar
    `i64` returns, which may be why this was never caught before), and
    whether the fix should generalize `resolve_bare_enum_ctors_in_stmt`
    (BUG-46) or take a narrower, Future-specific path.

  **FIXED 2026-08-04 (task #42).** Investigated per this entry's own
  "whoever picks this up next" note. Turned out rows 1 and 2 do NOT
  share a root cause after all -- two independent findings:
  - **Row 1 was ALREADY FIXED**, as a side effect of an unrelated
    2026-08-03 gap-audit fix (documented above `collect_generic_
    calls_in_expr`'s `Match` arm in checker.rs) that added `ExprKind
    ::Match` handling to the fn-generics call-site scanner -- for a
    completely different bug (`try EXPR` calling a generic fn only
    through a desugared match). `await(...)` ALSO desugars to
    `ExprKind::Match` (`synthesize_await_desugar` in parser.rs), so
    that fix transitively started covering `await(identity(42))`
    too, without anyone realizing it at the time. Verified directly:
    `await(identity(42))` now monomorphizes and runs correctly on
    both backends with NO code changes needed for this row.
  - **Row 2 was a real, separate bug**, unrelated to Future<T>'s
    `Type::Apply` never being monomorphized (the original diagnosis's
    hypothesis) -- `Future<T>`'s Apply construction was a red
    herring. The actual cause: `synthesize_await_desugar` builds
    `await(expr)`'s own internal match with two arms -- `Future.Ready
    (v) then v` and `Future.Pending then 0` -- and that `0` is a
    HARDCODED integer literal, only type-correct when T happens to be
    i64. v1 ships purely synchronous `async fn` semantics (every
    `Future<T>` value is constructed via `Future.Ready(v)`), so the
    Pending arm is PROVABLY unreachable at runtime -- but the parser
    can't know T at desugar time to build a properly-typed
    placeholder there, and the checker's ordinary "every match arm
    must produce the same type" rule has no way to know this specific
    arm is dead code, so it correctly (if unhelpfully) reported a
    type mismatch for any T != i64.
    Fixed with a new `checked_expr_placeholder(ty, span, env)`
    function in checker.rs that constructs a well-typed placeholder
    value for a type: scalars/Bool directly (reusing the same shapes
    `CheckedExpr::fallback` already used for a different error-
    recovery purpose); Enum via its first variant (recursing into the
    payload, if any, with this same function); Struct via all
    fields (recursing); Tuple/fixed-length Array via all elements
    (recursing). Returns `None` -- falling back to the ordinary
    mismatch diagnostic -- for anything not on that list (Vec, Box,
    OwnedStr, Ref/RefMut, Mutex, dyn Iface, etc.), matching the
    project's "unverifiable means rejected, never silently accepted"
    convention (BUG-68). Bounded/terminating for any well-formed
    program: a Struct/Enum can only cycle back to itself through a
    `Box` indirection (direct by-value recursion is already rejected
    at struct-declaration time), and `Type::Box` is one of the
    rejected shapes.
    Wired in at the ONE match-arm type-mismatch site in `check_expr`'s
    `ExprKind::Match` handling: when a mismatch is found AND the
    mismatching arm's pattern is EXACTLY `Pattern::Variant{enum_name:
    "Future", variant: "Pending"}` -- a shape users can never write
    directly, since `Future<T>` is entirely parser-synthesized --
    try the placeholder builder against the expected (Ready arm's)
    type before falling back to the diagnostic. Every other match in
    the whole language (the overwhelming majority) is completely
    unaffected: the gate only fires for this one synthesized shape.
  Verified against both original repros (prints `42` and `5`
  respectively) plus the `Option.None` branch of row 2 (prints `-1`)
  and a STRUCT-returning async fn (exercises the Struct-placeholder
  recursion) on both backends; `valgrind --leak-check=full` clean (0
  errors) on the struct case. No regression on the existing async
  test surface (16 pre-existing async-related unit tests, including
  every v3.1 phase test) or the wider suite -- full `cargo test
  --release --workspace`: 0 failed.
  New tests: 2 `src/lib.rs` + 2 `tests/run_end_to_end.rs`.

---

## Bug found by the local-model differential-fuzzing harness (2026-08-03)

- [x] **BUG-88 (found+fixed 2026-08-03, LLVM-only, backend-divergence).
  A non-ASCII local variable name (e.g. Devanagari `थैला`) crashed the
  LLVM backend's `lli` JIT with a parser error; the C backend handled
  the identical program fine.** Found by `tools/localfuzz/`'s continuous
  differential-testing harness (mutating/generating `.vani` programs and
  comparing both backends -- see that tool's README) on a mutated Nepali
  tutorial example. Minimal repro: any `let <non-ascii-name>: T = ...;`
  reached at the LLVM path.
  ```
  lli: candidate.ll:3371:8: error: expected '=' after instruction name
    %t26.थैला.addr = alloca %Struct_Bag
  ```
  Root cause: local variable/register names (`%<name>.addr`,
  `%arg_<name>`, the parameter list in a function's `define` line, task-
  handle allocas, outlined-closure capture rehydration -- 12 call sites
  total in `backend_llvm.rs`) were built directly from the raw source
  identifier. LLVM's textual IR only allows *unquoted* local identifiers
  matching `[a-zA-Z$._][a-zA-Z$._0-9]*`; anything else needs `%"..."`
  quoting, which none of these sites did. Global function symbols
  (`@fn_<name>`) already had this handled correctly via an existing
  `llvm_mangle_ident` helper (non-ASCII byte -> `_uHHHH` hex escape,
  producing a valid *bare* identifier rather than using LLVM's quoted-
  string form) -- it just was never called from any of the local-variable
  binding sites. Fixed by routing all 12 through the same helper; since
  `ctx.locals` is keyed by the *original* source name (only the emitted
  IR string needed mangling), no lookup-side code needed to change.
  Confirmed the fix is general, not Nepali-specific: the checker's lexer
  explicitly supports arbitrary Unicode identifiers (`lex_unicode_ident`
  in `lexer.rs`), so verified against three unrelated scripts (Devanagari,
  Hangul, Cyrillic) -- see the new
  `non_ascii_local_variable_name_produces_correct_output_on_both_backends`
  test in `tests/run_end_to_end.rs`.

  **Sibling hardening, same session, but only partially verifiable --
  read carefully before assuming it's "done":** struct/enum *type* names
  (`%Struct_<Name>`, `%Enum_<Name>`) had the identical unmangled-
  identifier gap, at 33 call sites throughout `backend_llvm.rs` (4 type-
  definition sites plus 29 usage sites, all independently doing
  `format!("%Struct_{}"/"%Enum_{}", <name>)` -- no shared helper, unlike
  the 12 local-variable binding sites). All 33 now route through
  `llvm_mangle_ident`, applied uniformly rather than auditing each site's
  provenance individually -- safe to do that way specifically because
  the helper is a proven no-op on all-ASCII input (`if name.bytes().all(|b|
  b < 0x80) { return name.to_string(); }`), so this cannot change codegen
  for any existing ASCII-named program; confirmed by the full
  `cargo test --workspace --release` run staying at 2703/2703 lib tests
  and 144/144 end-to-end tests, 0 failures, identical to before this change.

  **However**, trying to actually exercise this end-to-end (a struct
  declared with a non-ASCII name, then constructed/used) found the real
  path is blocked EARLIER than the backend, by the parser: `struct
  Кошка { age: i64 }` alone parses/checks fine (`vanic check` -> `ok`),
  but referencing that name afterwards -- `let c: Кошка = ...` or the
  struct-literal expression `Кошка { age: 3 }` -- fails to parse
  (`error: expected ';'` right after the identifier), identically on
  both backends, before codegen is ever reached. So the declaration
  grammar accepts non-ASCII struct/enum names but the type-annotation
  and struct-literal-constructor grammar apparently doesn't (some
  dispatch/lookahead rule in `parser.rs` likely assumes an ASCII-only
  shape for a "known type/constructor name" at those two call sites).
  This is a DIFFERENT, NOT fixed bug, in the parser rather than the
  LLVM backend -- logged here, not fixed, since it's out of the scope
  this session actually verified (parser grammar dispatch is different,
  higher-risk surface than the mechanical `llvm_mangle_ident` wrapping
  done above). The `backend_llvm.rs` mangling is still worth keeping:
  correct, safe (proven no-op on existing programs), and removes one
  layer of work for whoever fixes the parser gap next -- they won't
  also need to redo this. Repro files used (not committed, in `/tmp`,
  easy to recreate): a `struct Кошка { age: i64 }` declaration, plus
  a `let`/struct-literal reference to it, both under
  `examples/language/{russian,any}/`-style naming if formalized.

Full `cargo test --release --workspace` after BUG-88: 2703 lib tests,
144 end-to-end tests (including the 3 new cross-script cases), 0 failed
across all 13 binaries -- clean.

---

## Feature-combination gap audit sweep (2026-08-03), continued -- category 5

- [x] **BUG-89 (found+fixed 2026-08-03). Category 5, row 1:
  `Vec<dyn Iface>` holding TWO DIFFERENT monomorphizations of the same
  blanket-impl'd generic struct (`Wrapper<Dog>` and `Wrapper<Cat>`,
  both implementing `Printable` via `implement<T> Printable for
  Wrapper<T> where T is Printable`) crashed BOTH backends.** Root
  cause: `expand_blanket_impls` (`checker.rs`) appends a concrete impl
  per monomorphization to `program.impls` but never removes the
  ORIGINAL blanket impl (`type_params` non-empty, `for_type:
  Type::Apply { name, [Type::Param(_)] }`) -- unlike the exactly
  analogous, already-established pattern for generic functions/
  structs/enums in the same function (`program.functions.retain(|f|
  f.type_params.is_empty())`, `program.structs.retain(...)`,
  `program.enums.retain(...)`, all in the same monomorphization
  driver), which all correctly drop the generic template after
  monomorphization -- impls were simply the one category that never
  got this cleanup. Whatever later builds the `dyn Printable` vtable/
  trampoline set iterates every impl of the interface in
  `program.impls` and doesn't filter out the still-present blanket
  template, so it generated a BOGUS THIRD trampoline for the literal
  unresolved template `Wrapper<Param(T)>` alongside the two real
  ones: LLVM rejected the emitted IR ("loading unsized types is not
  allowed" on `%Struct_Wrapper__Param__T__`); C failed to compile
  ("implicit declaration of function
  'fn_Wrapper__Param__T___print_it'", referencing a struct type that
  was never declared). Fixed with a single-line addition,
  `program.impls.retain(|imp| imp.type_params.is_empty())`, placed
  right after `expand_blanket_impls` runs -- mirroring the
  established convention exactly. Given how broadly `program.impls`
  is read throughout the rest of the checker (static dispatch
  resolution, satisfiability checks, method lookup), ran the full
  `cargo test --release --workspace` suite immediately after this
  change (before adding new tests) to confirm nothing else depended
  on the blanket template surviving past this point: 13/13 binaries
  clean, 0 failed. Verified with `valgrind --leak-check=full` on a
  native AOT LLVM build: 0 errors, all heap blocks freed.
  Also swept the rest of category 5 (dyn dispatch x generics): a
  generic function bounded `<T: Iface>` that ALSO takes a `dyn Iface`
  parameter of the same interface in a different slot (mixing static
  and dynamic dispatch for the same trait in one call), and a struct
  implementing TWO DIFFERENT interfaces with a single instance
  referenced through two SEPARATE `Vec<dyn IfaceA>`/`Vec<dyn IfaceB>`
  Vecs, both checked clean on both backends -- not bugs.
  New tests: 1 `src/lib.rs` + 1 `tests/run_end_to_end.rs` (real
  stdout, both backends). Full `cargo test --release --workspace`
  after the new tests landed: 13/13 binaries clean, 0 failed (2706
  lib tests, up from 2705; 147 end-to-end tests, up from 146 --
  counts relative to this session's own commit history, which may
  differ slightly from a concurrently-updated `main` given another
  process is independently landing fixes to this same repo in
  parallel this session). Category 5 (all 3 rows) now fully closed
  in `docs/FEATURE_COMBINATION_GAPS_TODO.md`.

---

## Feature-combination gap audit sweep (2026-08-03), continued -- category 6

- [x] **BUG-90 (found+fixed 2026-08-03). Category 6, rows 2 and 3:
  `try EXPR` calling a GENERIC function (`fn wrap<T>(x: T) ->
  Option<T>`) failed with "generic function 'wrap' is declared but
  never called with concrete types" even though the call is
  genuinely there; a related shape (`try` propagating through a
  nested `Option<Result<T,E>>`) failed with a baffling "expected
  Result<i64, i64>, got Result__i64__i64" -- same mangled name on
  both sides of the mismatch.** Root cause was FOUR compounding
  gaps, all the same "sibling walker never learned about a
  syntax-sugar shape" pattern this session kept hitting all the way
  back to BUG-83/85/86:
  1. `collect_generic_calls_in_expr` (`checker.rs`) had NO arm for
     `ExprKind::Match` or `ExprKind::Block` at all. `desugar_try_let_
     in_program` runs BEFORE fn-generics monomorphization, so by the
     time this scanner walks the program, every `try wrap(n)` has
     ALREADY been rewritten into `Match { scrutinee: Call(wrap, [n]),
     arms: [Some(..) then Block{...}, None then ...] }` -- the
     generic call sitting in a match scrutinee (or inside a
     desugared Some-arm's Block) was structurally invisible to this
     walker. (A narrower `ExprKind::Try` arm was added first but is
     dead code for this exact path, since `Try` nodes don't survive
     the desugar -- it may still matter for `try` occurrences the
     desugar doesn't reach, so it was kept.)
  2. `rewrite_generic_calls_in_expr`, the SIBLING pass that actually
     renames a resolved call site (`wrap` -> `wrap__i64`), had the
     identical missing-arm gap. Without it the call site was never
     renamed, so once the generic template got dropped post-
     monomorphization the bare name lingered in the AST, surfacing
     as "unknown function 'wrap'".
  3. `substitute_type_param`'s `Type::Apply` collapse (once a
     generic fn's substituted args become all-concrete) always
     produced `Type::Struct(mangled)`, never `Type::Enum(mangled)`
     -- so a generic fn returning `Option<T>`, once specialized,
     got a return type that Displayed as "Option__i64" but was
     actually a `Type::Struct`, not equal to the real `Type::Enum
     ("Option__i64")` used at the call site. Both print identically
     (Display doesn't distinguish struct/enum), so the mismatch
     surfaced as "expected Option__i64, got Option__i64". Fixed with
     a new `GENERIC_ENUM_TEMPLATE_NAMES` thread-local, populated
     once from `program.enums` at the very start of the checker
     pipeline (before any monomorphization pass touches them,
     alongside the pre-existing `enum_names_pre` computation), so
     the collapse can tell "this Apply name was originally an enum
     template" from "this was a struct template" and pick the right
     `Type` variant.
  4. `collect_apply_in_stmt`/`rewrite_apply_in_stmt` (used by
     `monomorphize_type_decls_in_program` to resolve `Type::Apply`
     annotations into concrete `Type::Enum`/`Type::Struct`) only
     ever looked at a `Stmt::Let`'s own `annotation` field and never
     recursed into `Return`/`Assign` exprs at all -- so a NESTED
     `Stmt::Let` the try-desugar synthesizes inside a `Block` inside
     a `Match` arm (e.g. `let r: Result<i64, i64> = __t;`, lifted
     from the user's `let r: Result<i64,i64> = try lookup(x);`)
     never had its annotation's `Type::Apply` resolved at all,
     again surfacing as "expected Result<i64, i64>, got
     Result__i64__i64" (the SAME name, one resolved, one not).
     Fixed by adding a `walk_expr_for_nested_lets`/
     `rewrite_apply_in_expr_nested_lets` helper pair that recurses
     through `Block`/`Match`/`IfExpr`/`Try` shapes to reach nested
     `Stmt::Let`s, called from `Stmt::Let`'s own expr, `Stmt::
     Return`, and `Stmt::Assign`.
  All four fixes compose and were verified together on both
  backends for: a plain `try wrap(n)` where `wrap<T>` is generic and
  `n` is a Var argument (not just a literal); two chained `try`s in
  the same function, one concrete (`lookup`) and one generic
  (`wrap`), exercising both the early-return and pass-through paths;
  and `try` propagating through a nested `Option<Result<i64, i64>>`
  in both the early-return (`Option.None`) and pass-through
  (`Option.Some(Result.Ok/Err(..))`) directions.
  Also swept category 6 row 1 (`try`/`?` inside a function holding a
  live LOCAL `Vec<Struct>` binding, read via `clone_at` after the
  `try`, across the early-return path): checked clean on both
  backends, not a bug -- verified with `valgrind --leak-check=full`
  on native AOT builds of both backends: 0 errors, all heap blocks
  freed, "All heap blocks were freed -- no leaks are possible".
  New tests: 4 `src/lib.rs` + 3 `tests/run_end_to_end.rs` (real
  stdout, both backends).
  Category 6 rows 1 and 3 fully closed. Row 2 partially closed: the
  `try`-specific failure mode is fixed (see above), but testing it
  surfaced a SEPARATE, deeper, pre-existing bug independent of `try`
  entirely -- see BUG-91 below, found but deferred.

- [x] **BUG-91 (found 2026-08-03, deferred, then FIXED 2026-08-04 --
  task #40; see the fix writeup right after this entry).** A bare
  call to a GENERIC function returning `Option<T>`/`Result<T,E>`,
  used DIRECTLY as a `match` scrutinee with no intermediate `let`
  binding (`match foo(7) { Option.Some(x) then x, Option.None then
  -1 }` where `fn foo<T>(a: T) -> Option<T>`), fails with `enum
  'Option__i64' is not declared` -- even though the exact same
  generic fn + call, when the result is first bound via an
  explicitly-annotated `let x: Option<i64> = foo(7);` and THEN
  matched on `x` as a separate statement, compiles and runs
  correctly. Reproduces with NO `try`/`?` anywhere in the program --
  it surfaced while testing category 6 row 2 (a generic function
  using `try` internally, whose caller in `main` naturally matches
  the generic call's result directly), but the root cause is
  independent of `try` and belongs to the generics/monomorphization
  pipeline generally.
  Root cause: `monomorphize_type_decls_in_program` (which
  materializes a concrete `EnumDecl`/`StructDecl` -- e.g. the actual
  `enum Option__i64 { ... }` declaration, not just the `Type::Enum`
  reference to it -- for every `Type::Apply` it finds anywhere in
  the program) runs BEFORE `monomorphize_generics_in_program` (the
  fn-level pass that infers `T=i64` for `foo(7)` and creates
  `foo__i64`). When the ONLY place a concrete `Option<i64>`
  instantiation is discoverable is through fn-generics' own type
  inference at a call site (no textual `Option<i64>` annotation
  appears anywhere else in the source for the earlier decl-mono
  pass to see), the enum-decl pass has already run and finished by
  the time that need becomes known -- and worse, right after it
  runs it unconditionally drops the generic template it monomorphized
  from (`program.enums.retain(|e| e.type_params.is_empty())` at
  checker.rs:8082, mirroring the same convention used for functions/
  structs/impls), so there is no template left to re-specialize from
  even if a second pass were added naively. `substitute_type_param`'s
  collapse (see BUG-90 fix #3 above) DOES produce a correct
  `Type::Enum("Option__i64")` reference for `foo__i64`'s return
  type, but a bare `Type::Enum` reference isn't the same thing as
  an actual `EnumDecl { name: "Option__i64", variants: [...] }`
  existing in `program.enums` -- nothing materializes the latter for
  this case, hence "enum 'Option__i64' is not declared".
  This is why every earlier BUG-90 repro happened to work: in each
  one, the ENCLOSING function's own return type was a concrete
  `Option<i64>`/`Result<i64,i64>` (a literal, textual annotation
  visible to the decl-mono pass before fn-generics ran), so the
  needed `EnumDecl` always already existed for an unrelated reason
  by the time the generic call's result needed it. BUG-91 is
  specifically the case where NO such textual concrete annotation
  exists anywhere in the source -- the concrete instantiation is
  discoverable ONLY via inference at a generic call site, consumed
  immediately (bare match scrutinee, no intermediate annotated
  `let`).
  Deferred rather than fixed in-session: a real fix needs either (a)
  interleaving `monomorphize_type_decls_in_program` and
  `monomorphize_generics_in_program` to a shared fixed point (each
  pass's output can create new work for the other -- fn-mono
  discovers new concrete Apply instantiations; decl-mono's freshly
  materialized decls could in principle reference further generics),
  or (b) retaining the original generic struct/enum templates (not
  dropping them at checker.rs:8081-8082) until AFTER fn-generics
  monomorphization has also stabilized, then running a final decl-
  materialization pass against whatever's left in `program.structs`/
  `program.enums`'s Type::Apply-in-signature surface at that point.
  Both directions touch a large, heavily-load-bearing shared
  pipeline stage with a wide blast radius (every generic struct/enum
  instantiation in the whole language goes through this path) --
  the same category of risk that led BUG-87 (async + generics) to
  be deferred rather than rushed. Whoever picks this up next: start
  by checking whether `monomorphize_generics_in_program` can, after
  specializing a generic fn, feed any newly-concrete `Type::Apply`
  it finds in the SPECIALIZED signature back into a queue that a
  final call to (a re-entrant, template-preserving version of)
  `monomorphize_type_decls_in_program` drains once fn-mono's own
  worklist (`needed`/`generated_keys`) reaches its fixed point.
  Repro (fails on both backends, no `try` involved):
  ```
  fn foo<T>(a: T) -> Option<T> {
    return Option.Some(a);
  }
  fn main() -> i64 {
    let r1: i64 = match foo(7) {
      Option.Some(x) then x,
      Option.None then -1,
    };
    print r1;
    return 0;
  }
  ```
  No regression test added (nothing to assert as passing); the repro
  above is preserved here for whoever fixes it. Category 6 row 2 is
  left checked in `docs/FEATURE_COMBINATION_GAPS_TODO.md` only for
  its `try`-specific aspect (fixed); this deferred finding is noted
  inline there too.

  **FIXED 2026-08-04 (task #40).** Took direction (a) from this
  entry's own "whoever picks this up next" note: fed newly-collapsed
  `Type::Apply` instantiations back into a queue that a final
  materialization pass drains once fn-generics' own worklist
  stabilizes -- reusing the `NEWLY_COLLAPSED_GENERIC_APPLIES` queue
  BUG-95 already introduced (that queue is populated by
  `substitute_type_param`'s collapse site itself, so it fires
  identically whether the collapse happens during
  `monomorphize_type_decls_in_program`'s own struct/enum field
  substitution -- BUG-95's case -- or during `monomorphize_generics_
  in_program`'s fn-signature substitution -- this bug's case; no
  changes needed at the collapse site at all).
  The one genuinely new piece direction (a) required: the original
  generic templates are gone (dropped) by the time fn-generics
  finishes, so `check_program` now snapshots `struct_templates`/
  `enum_templates` into local variables BEFORE calling
  `monomorphize_type_decls_in_program` (which still drops them from
  `program.structs`/`program.enums` exactly as before -- unchanged),
  and a new `materialize_late_discovered_type_decls` function runs
  right after `monomorphize_generics_in_program`: drains whatever the
  queue collected, and -- using the snapshots -- runs one more small,
  self-contained fixed-point round (mirroring `monomorphize_type_
  decls_in_program`'s own worklist loop's shape, but a fresh,
  parallel copy rather than a factored-out shared function, to avoid
  restructuring an 800+ line, heavily load-bearing pipeline stage
  this late -- yet another instance of the "duplicate walker" pattern
  already common in this file, e.g. `collect_apply_in_stmt` vs.
  `collect_apply_in_ty`) to materialize any still-missing decls. A
  no-op in the overwhelmingly common case where nothing new was
  discovered (checked via a cheap emptiness check on the drained
  queue before doing any other work).
  Verified against the exact repro above on both backends (prints
  `7`), plus a two-generic-function variant where two DIFFERENT
  generic fns each need a DIFFERENT concrete `Option<T>` instantiated
  purely through this late-discovery path in the same program
  (`Option__i64` and `Option__bool`), confirming the fixed-point loop
  correctly materializes more than one late-discovered decl. No
  regression on the full existing generics/monomorphization test
  surface (BUG-90/93/95's own tests, the recursive-generic-struct
  tests, etc.) -- full `cargo test --release --workspace`: 0 failed.
  Deferred, out-of-scope finding surfaced while probing edge cases,
  now tracked separately as BUG-98 below (a different bug from
  BUG-91: that one was about the DECL not existing; BUG-98 is about
  a working decl not being NAMEABLE from inside a still-generic
  function body).
  New tests: 1 `src/lib.rs` + 1 `tests/run_end_to_end.rs`. Full
  `cargo test --release --workspace`: 0 failed.

- [x] **BUG-98 (found+fixed 2026-08-04 -- task #41 -- a BUG-46-class
  ambiguity, but reaching a different site than BUG-46/95 cover). A
  bare enum constructor INSIDE a generic function's OWN body (e.g.
  `Option.Some(a)` in `fn foo<T>(a: T) -> Option<T> { return
  Option.Some(a); }`) fails to resolve ("unknown variable 'Option'")
  once 2+ DISTINCT concrete instantiations of `Option` exist
  anywhere in the whole program** -- regardless of whether they come
  from the SAME generic fn specialized twice (`foo(7)` and
  `foo(true)`) or from two DIFFERENT generic fns each specialized
  once (`foo1(7)` and `foo2(true)`, each internally constructing
  `Option.Some(a)`). A single instantiation program-wide works fine
  (this is exactly why BUG-91's own repro, `foo(7)` alone, never hit
  this: only one `Option` instantiation existed in that test).
  Root cause: `Option.Some(a)` inside a generic function's body stays
  a bare, unqualified `Var("Option")` receiver at parse time, same as
  everywhere else in the language. Resolving it to a concrete
  `Option__i64`/`Option__bool` normally happens one of two ways: (1)
  BUG-46/95's targeted, annotation-driven passes
  (`resolve_bare_enum_ctors_in_stmt`,
  `resolve_bare_enum_ctor_in_struct_lit`) which key off an
  already-KNOWN concrete type at the use site (a `let`'s own
  annotation, a function's own return type already being a concrete
  `Type::Enum`, a struct literal field's declared type) -- none of
  these apply to a still-generic function TEMPLATE's body, since the
  template's return type is `Type::Apply{Option,[Param(T)]}` (or
  similar), not yet a concrete `Type::Enum`, at the point these
  passes run (before `monomorphize_generics_in_program` specializes
  the body); or (2) `Env::resolve_enum_name`'s general "exactly one
  candidate starting with `{name}__`" fallback, consulted at ordinary
  type-check time (AFTER specialization, when the body is finally
  checked as part of `foo__i64`/`foo__bool`) -- this is what actually
  resolves it in the single-instantiation case, but is inherently
  ambiguous the moment 2+ candidates exist program-wide, and fails
  outright rather than picking one.
  What's missing: nothing re-resolves a generic function's bare enum
  constructors using the SPECIALIZED body's own now-concrete return
  type, once `monomorphize_generics_in_program` has produced it. The
  information needed (each specialized stub's own concrete return
  type, right after substitution) is exactly what BUG-91's fix
  (`materialize_late_discovered_type_decls`) already has on hand at
  the point it runs -- likely the natural place to also re-run BUG-46/
  95-style resolution against each newly-specialized function body,
  the same way the ORIGINAL (pre-generics) pass already does for
  ordinary concrete functions.
  **FIXED 2026-08-04 (task #41), same session as BUG-91.** Exactly
  the fix this entry's own "what's missing" paragraph anticipated:
  re-run `resolve_bare_struct_lits_in_stmt`/`resolve_bare_enum_ctors_
  in_stmt` over every function body in `check_program`, right after
  `monomorphize_generics_in_program` AND BUG-91's own `materialize_
  late_discovered_type_decls` have both finished -- at that point
  each specialized stub's own return type IS concrete, so the exact
  same BUG-46/95 resolution logic that already handles ordinary
  functions correctly resolves the generic body's bare receiver too.
  Uses the SAME template-name snapshots (`generic_struct_templates`/
  `generic_enum_templates`) BUG-91's fix already introduced. A no-op
  for already-resolved ordinary function bodies: their receivers, if
  any were bare, were already rewritten to a concrete mangled name by
  the FIRST (pre-fn-generics) pass, which no longer matches a generic
  template name on this second pass, so nothing double-rewrites.
  Verified against the repro below (prints `7` then `1`) plus the
  same-generic-fn-specialized-twice variant (`foo(7)` and `foo(true)`
  from the SAME `foo<T>`) on both backends, and confirmed no
  regression on BUG-91's own repro or the wider generics/
  monomorphization test surface. Full `cargo test --release
  --workspace`: 0 failed.
  Repro (previously failed on both backends, now passes):
  ```
  fn foo1<T>(a: T) -> Option<T> {
    return Option.Some(a);
  }
  fn foo2<T>(a: T) -> Option<T> {
    return Option.Some(a);
  }
  fn main() -> i64 {
    let r1: i64 = match foo1(7) {
      Option.Some(x) then x,
      Option.None then -1,
    };
    let r2: i64 = match foo2(true) {
      Option.Some(x) then 1,
      Option.None then -1,
    };
    print r1;
    print r2;
    return 0;
  }
  ```
  New tests: 1 `src/lib.rs` + 1 `tests/run_end_to_end.rs`.

---

## Feature-combination gap audit sweep (2026-08-03), continued -- category 7

- [x] **BUG-92 (found+fixed 2026-08-03). Category 7, row 3
  neighborhood: `examples/language/english/bare_metal.vani` -- the
  EXACT shipped example BUG-44 fixed -- crashes `opt`/`llc` with
  ill-typed IR the instant it's actually built or run, on the
  DEFAULT host target, no `--target`/`--no-std` needed at all.**
  Found while re-auditing the BUG-44 neighborhood (cross-compile +
  `no_std` + `#[no_mangle]` FFI export, three-way) per category 7
  row 3 -- but the root cause turned out to be independent of the
  three-way combination specifically: BUG-44's own fix was verified
  by grepping EMITTED TEXT for the bare symbol name, never by
  actually running the example through `opt`/`llc`/`lli`, so this
  bug was sitting undiscovered in an already-shipped tutorial
  example the whole time.
  Two compounding bugs, both in `backend_llvm.rs`'s tree-LLVM
  emitter (only reached when a program contains a `#[no_mangle]` fn
  anywhere, which routes the WHOLE program to tree-LLVM per BUG-44's
  own fix -- SSA-LLVM was never affected, and served as the
  reference for the correct convention):
  1. `mmio_read_u8`/`mmio_read_u16` internally did `zext i8`/`zext
     i16 ... to i64` after their `load volatile`, unconditionally
     widening the narrow hardware read to i64 -- contradicting the
     "narrow types stay narrow until an explicit cast" convention
     SSA-LLVM already follows for ordinary u8/u16 arithmetic
     (confirmed directly: `let y: u16 = x + 5;` emits `add i16
     %v_0, %v_1`, no widening, until a later `as i64` cast does an
     explicit `zext`). Storing that i64-typed result into a `let sr:
     u16 = mmio_read_u16(...);`'s `i16` alloca (the generic scalar
     Let codegen, which assumes `emit_expr`'s result already matches
     the destination width) produced ill-typed IR: "'%t2' defined
     with type 'i64' but expected 'i16'". Fixed by removing the
     internal zext entirely -- `mmio_read_u8`/`mmio_read_u16` now
     return the raw narrow-width loaded value directly, matching
     their own checker-declared `u8`/`u16` return type and mirroring
     `ssa_backend_llvm.rs`'s already-correct implementation exactly
     (confirmed by direct comparison).
  2. Fixing (1) exposed a SECOND, previously-masked bug in the same
     builtin family: `mmio_write_u8`/`mmio_write_u16` unconditionally
     emitted `trunc i64 {val} to i8`/`i16` before the volatile store,
     assuming `val` was always i64-typed. But the checker's own
     typing of these builtins (`coerce_checked(..., &Type::U8, ...)`
     / `&Type::U16`) means `args[1]`'s checker type is ALWAYS
     `u8`/`u16` already -- so `val` is ALREADY narrow whenever it
     comes from an already-narrow source (any `u8`/`u16` parameter or
     local -- confirmed directly: a plain `byte: u8` parameter loads
     as a native `i8`, `%t0 = load i8, i8* %byte.addr`, never i64).
     `trunc i64 %t0 to i8` when `%t0` is ALREADY `i8`-typed is itself
     ill-typed IR ("'%t0' defined with type 'i8' but expected
     'i64'") -- the OPPOSITE mismatch direction from bug (1). This
     was masked before bug (1)'s fix because nothing had exercised a
     write fed by an already-narrow value without ALSO hitting the
     read-side crash first. Fixed by removing the blind trunc
     entirely and storing `val` directly -- again mirroring `ssa_
     backend_llvm.rs`'s already-correct implementation exactly (it
     never truncates either).
  Verified: `examples/language/english/bare_metal.vani` now builds
  (`vanic build ... -lm`) and runs cleanly end-to-end on the default
  host target (previously crashed `opt`/`llc` before even reaching
  the link step); `valgrind --leak-check=full` on the resulting
  native binary for a minimal regression repro combining all four
  builtins under a `#[no_mangle]` fn: 0 errors, all heap blocks
  freed. New tests: 3 `src/lib.rs` + 1 `tests/run_end_to_end.rs`
  (the e2e test is the REAL regression guard here -- it actually
  builds+links+runs through `opt`/`llc`, which the `compile_to_llvm`
  lib.rs helper does not exercise at all, since it calls
  `LlvmBackend.emit` directly with no verifier pass).
  Also swept the rest of category 7 (collections beyond Vec/
  HashMap): (row 1) iterator-style Vec builtins chained directly in
  ONE expression are correctly rejected per docs (`tutorials/src/
  intermediate/06b_iterators_primer.md`'s own explicitly-documented
  "chain directly" restriction -- method-call sugar only rewrites a
  plain named-`Var` receiver); chaining via named `let`s between
  each step (the documented v1 pattern) verified correct on both
  backends. (row 2) `task`/`join` call-form with a genuinely multi-
  block callee body (nested if/else inside a while loop) verified
  correct on both backends by hand-computed expected values, not
  just cross-backend agreement. Also confirmed the block-form `task
  <name> { ... }`'s documented by-value/Copy capture semantics (a
  write to a captured outer variable inside the task body does NOT
  propagate back after `join` -- intentional, not a bug). (row 4)
  Graph/Bst/Trie/SkipList/UnionFind/BloomFilter all actually run
  end-to-end together (not just compile-checked), every value
  verified against `advanced/05b_advanced_collections.md`'s own
  documented expected output, on both backends. (row 5)
  `vec_with_capacity` under `--backend=c` specifically: pushing past
  the initial capacity (forcing a real realloc/growth) produces
  correct VALUES (not just correct length), verified with `valgrind
  --leak-check=full` on the native C-backend binary: 0 errors. (row
  6) `Deque<Struct>`/`BinaryHeap<Struct>` are both cleanly rejected
  as scalar-i64-only in v1, same restriction shape as HashMap's
  documented scalar-only-V boundary -- not a bug. (row 7) `Graph`/
  `Trie` with non-i64 payloads: more fundamental than a runtime
  restriction -- both are non-generic (no `Type::Apply` form at
  all), so the parser itself rejects `Graph<T>`/`Trie<T>` syntax
  outright ("expected '='"); the boundary can't even be expressed.
  New tests for rows 1/2/4/5/6/7: 6 `src/lib.rs` + 3 `tests/
  run_end_to_end.rs`. Category 7 (all 7 rows) fully closed in
  `docs/FEATURE_COMBINATION_GAPS_TODO.md`.

---

## Feature-combination gap audit sweep (2026-08-03), continued -- category 8

Category 8 (FFI x generics/containers/error-handling), all 3 rows
checked clean -- no bugs found, all three combinations either work
correctly or are cleanly rejected exactly as documented:

- (row 1) `extern "C"` fn taking/returning a MONOMORPHIZED GENERIC
  struct by value (BUG-77 only tested a concrete, non-generic
  struct). A small (<=16 byte, all-scalar-field) monomorphized
  generic struct (`Wrapper<i32>`, mangled `Wrapper__i32`) passes AND
  returns by value correctly against a real linked C shim, on both
  backends -- confirming the mangled name and the ABI-lowering path
  agree, exactly the concern this row was written to probe. An
  oversized monomorphized generic struct (`Triple<i64>`, 24 bytes)
  is cleanly rejected, with the diagnostic correctly naming the
  MANGLED monomorphized type (`Triple__i64`, not the generic
  template name) -- confirming FFI ABI validation runs after
  monomorphization and sees the concrete shape.
- (row 2) `extern "C"` fn signature using `Option<T>`/`Result<T,E>`
  directly in a parameter or return position: cleanly rejected on
  both backends with a specific, on-point diagnostic ("enum-by-value
  layout is not yet wired through FFI"), in both return position
  (`Option<i64>`) and parameter position (`Result<i64, i64>`).
- (row 3) Calling an `extern "C"` function inside a spawned `task`
  body: a plain (non-pure) `extern "C" fn` call hits the exact same
  "task body cannot call non-pure function" diagnostic as any other
  impure call -- and the diagnostic's own hint ("mark it `pure
  extern`") points at a REAL, documented escape hatch (`pure extern
  "C" fn`, `tutorials/src/intermediate/09_ffi.md`) that was verified
  to genuinely work end-to-end: a `pure extern "C" fn` called inside
  a `task` body, joined, against a real linked C shim, runs
  correctly on both backends. `valgrind --leak-check=full` on the
  resulting native binary: 0 errors, all heap blocks freed. Not a
  distinct, undocumented gap.
  New tests: 4 `src/lib.rs` + 2 `tests/run_end_to_end.rs` (the two
  e2e tests are the real verification here, each linking and running
  a genuine C shim). Category 8 (all 3 rows) fully closed in
  `docs/FEATURE_COMBINATION_GAPS_TODO.md`.

---

## Feature-combination gap audit sweep (2026-08-03), continued -- category 9

- [x] **BUG-93 (found+fixed 2026-08-03). Category 9, row 2: a
  recursive GENERIC struct (`struct Node<T> { value: T, next:
  Option<Box<Node<T>>> }`, self-referential AND generic at once)
  failed to compile at all -- even though the exact same pattern
  with a CONCRETE (non-generic) `Node` has been a working, shipped
  regression test since BUG-35.** Root cause was FIVE compounding
  gaps, the SAME "sibling walker never learned about a shape"
  pattern as BUG-90, but this time the missing shape was
  `Type::Box` (not `Match`/`Block`):
  1-4. `collect_apply_in_ty`, `rewrite_apply_in_ty`, the `rec`
  closure inside `collect_apply_in_stmt`, and `normalize_one` (FOUR
  independent copies of essentially the same "walk a Type looking
  for/rewriting nested `Type::Apply`" logic, all in `checker.rs`)
  each had an arm for `Vec`/`Ref`/`RefMut`/`Atomic`/`Mutex`/`Guard`
  but none for `Type::Box` -- so a generic instantiation nested
  inside a `Box<...>` (e.g. `Option<Box<Node<T>>>`'s inner
  `Node<T>`) was invisible to every one of these passes, surfacing
  as "enum payload must be assignable to Box<Node<i64>>, got
  Box<Node__i64>" (same underlying type, one resolved, one not --
  the exact same symptom shape BUG-90 kept producing). Fixed by
  adding the missing arm to all four, mirroring the existing
  sibling-type arms exactly.
  5. Even with (1-4) fixed, `monomorphize_type_decls_in_program`
  still only ran discovery+generation in a SINGLE pass: a freshly-
  monomorphized generic struct's OWN fields can introduce a FURTHER
  generic instantiation need (concretely: `Node__i64`'s own `next`
  field needs `Option<Box<Node__i64>>` registered, but nothing else
  in the program ever writes that type out literally for the
  single discovery pass to find). The code already computed this
  via `collect_apply_in_ty(&fld.ty, ..., &mut needed_structs.
  clone(), &mut needed_enums.clone())` -- but cloning BOTH output
  lists meant the discovery had nowhere real to go; the existing
  comment literally said "ignored copy". Fixed by converting the
  single-pass generation into a proper fixed-point worklist,
  mirroring the established "XL4 multi-pass" pattern
  `monomorphize_generics_in_program` already uses for the analogous
  fn-generics case: each round processes only newly-pending (name,
  args) pairs (tracked via a linear-scan `Vec` rather than a
  `HashSet` since `Type` isn't `Hash`) and feeds any further
  discovered needs into the next round, until a round adds nothing
  new.
  Verified with `valgrind --leak-check=full` on a native AOT LLVM
  build: 0 errors, all heap blocks freed. Given the blast radius (a
  refactor of the shared struct/enum monomorphization pipeline used
  by every generic type in the language), ran the full `cargo test
  --release --workspace` suite immediately after the refactor
  compiled (before adding new tests) to confirm no regressions:
  13/13 binaries clean, 0 failed.
  Also swept category 9 row 1 (`clone_at` on `Vec<GenericStruct<T>>`,
  the indexed-mutate-then-`set` idiom through a generic element
  type): checked clean on both backends, `valgrind --leak-check=
  full` clean -- `clone_at` correctly deep-clones the element
  (including a non-Copy `OwnedStr` field), `set` writes the mutated
  clone back to the right index, unrelated slots untouched.
  THREE separate, narrower findings surfaced while investigating
  row 2, all deferred rather than fixed in-session:
  - (a) A bare enum constructor written DIRECTLY inside a struct-
    literal field (`Node { value: 2, next: Option.Some(box(tail)) }`)
    is still ambiguous once 2+ instantiations of the same generic
    enum exist in the program -- BUG-46's existing fix
    (`resolve_bare_enum_ctors_in_stmt`) only covers a `Let`'s own
    top-level initializer or a `Return`, never an enum constructor
    nested inside a `StructLit` field. HAS A WORKING WORKAROUND:
    bind the constructor to its own `let` with an explicit enum
    annotation first, THEN use that binding as the struct field --
    this hits BUG-46's already-working `Let`-annotation path and
    compiles/runs correctly (verified, both backends; this is also
    the pattern the new regression tests use, since it's the
    idiomatic v1 way to write this today).
  - (b) Field access through a BARE `Box<T>` (`n1.value` where
    `n1: Box<Node<i64>>`) is rejected outright ("field access on
    non-struct type Box<...>") -- `Type::deref()` (`ast.rs`) only
    peels `Ref`/`RefMut`, never `Box`. Reproduces identically with
    a non-generic `Box<T>` too, so it's orthogonal to generics
    specifically -- confirmed the RAII tutorial's own `Box<Node>`
    linked-list example never actually demonstrates reading a
    field back through a `Box`, only building the chain. `Type::
    deref()` is used in 60+ call sites across `checker.rs`/
    `backend_c.rs`/`backend_llvm.rs`/`ssa.rs`/`smt.rs` -- extending
    it (or adding a parallel Box-aware variant) needs careful
    auditing of every call site's assumptions before it's safe to
    touch, the same category of risk as BUG-91's deferred fix.
  - (c) While valgrind-verifying (a)'s workaround, found a THIRD,
    genuinely separate PRE-EXISTING memory leak, independent of
    generics entirely: `Box<StructWithHeapOwningFields>`'s scope-
    exit Drop (`backend_c.rs`'s per-statement Drop AND its sibling
    `emit_struct_field_drops` helper both hit the exact same gap)
    only frees the Box's OWN heap slot -- it never recursively
    drops the BOXED struct's own heap-owning fields first.
    Reproduces on the ALREADY-SHIPPED, non-generic `examples/
    language/english/option_box_recursive_struct.vani` (BUG-35's
    own regression example): `valgrind --leak-check=full` on its
    C-backend build shows "48 (24 direct, 24 indirect) bytes ...
    definitely lost" (LLVM backend: 0 errors, unaffected -- LLVM's
    Box-drop codegen apparently already chains correctly, or never
    hit this specific gap; not investigated further given scope).
    Deferred rather than fixed in-session: unlike every other fix
    in this session, this ISN'T a simple missing-arm addition -- a
    `Box<Node>` field pointing into a chain of unknown-at-compile-
    time length needs a genuine RUNTIME recursive/iterative free
    routine (mirroring how `Vec<T>`'s own drop is a real per-
    element-type HELPER FUNCTION that loops at runtime, not inlined
    code), which is a substantive design task rather than a
    mirrored one-line arm addition. Whoever picks this up next:
    start from `emit_struct_field_drops`'s `Type::Box(box_inner) =>
    match &**box_inner { ... _ => free-only ... }` arm (backend_c.rs)
    and the analogous top-level scope-exit Drop arm for
    `Type::Box(inner)` -- both need a `Type::Struct(name)` case that
    emits a call to a NEW dedicated recursive free helper (one per
    boxed struct shape) rather than a bare `free()`.
  New tests: 2 `src/lib.rs` + 1 `tests/run_end_to_end.rs` (real
  stdout, both backends) for the fixed recursive-generic-struct
  case, using workaround (a)'s pattern (also the idiomatic v1 way
  to write this today). Full `cargo test --release --workspace`
  after the new tests landed: 13/13 binaries clean, 0 failed.
  Category 9 rows 1 and 2 closed in `docs/FEATURE_COMBINATION_GAPS_
  TODO.md`; rows 3-4 remain for a follow-up sweep pass.

Category 9 rows 3-4, swept immediately after -- both checked clean,
no bugs found:

- (row 3) `Box<T>` through a generic function boundary
  (`fn identity<T>(b: Box<T>) -> Box<T>`) -- flagged "worth probing,
  never observed broken" in `missing_features.md`'s own closing
  list. Verified for both a struct T and a scalar T; `valgrind
  --leak-check=full` clean on both backends -- ownership correctly
  passes through the generic boundary and back with no double-free
  or leak. (A second call site, `identity(box(42))` passing the
  `box(...)` call expression directly rather than through a named
  variable, hit v1's own DOCUMENTED and unrelated generic-inference
  restriction -- "supports literal arguments... Var, or Ref/RefMut
  (Var)... more complex argument expressions need full type-
  checking context" -- expected, not a finding.)
- (row 4) `parallel for` over a `Vec<Struct>` with an `OwnedStr`
  field -- flagged "worth probing" in the same list. Each iteration
  writes to a DISTINCT index via `clone_at` (a deep copy of a source
  element, no shared heap state) -- the compiler correctly ALLOWS
  this, since there is no actual race (no two iterations ever touch
  the same memory location); confirmed genuinely safe with `valgrind
  --leak-check=full` on the C-backend build: 0 errors, all heap
  blocks freed (the old `OwnedStr` previously occupying each written
  slot is correctly dropped as part of the per-iteration write). The
  LLVM-backend build showed small "definitely lost"/"possibly lost"
  counts under valgrind, but the backtrace traces entirely into
  `libgomp`'s own OpenMP thread-pool machinery (`GOMP_parallel` ->
  `pthread_create` -> `allocate_stack` -> `_dl_allocate_tls`) --
  well-known valgrind-vs-libgomp housekeeping noise, not vani-
  generated code; the C-backend run (identical program logic, no
  libgomp) being fully clean confirms the LOGIC itself is correct.
  Fresh heap allocation INSIDE the loop body (e.g. `"x" + ""`
  string concatenation) hits a SEPARATE, already-documented purity
  restriction ("'parallel for' body cannot use `+` on strings (heap
  allocation is impure)") -- expected v1 behavior, not this row's
  concern (the row asks about the ELEMENT TYPE's affine-ownership
  interaction with `parallel for`, which `clone_at` isolates
  cleanly from the separate "no impure heap alloc in the loop body"
  rule).
New tests: 2 `src/lib.rs` + 2 `tests/run_end_to_end.rs`. Full `cargo
test --release --workspace`: 0 failed. Category 9 (all 4 rows) now
fully closed in `docs/FEATURE_COMBINATION_GAPS_TODO.md`.

---

## Feature-combination gap audit sweep (2026-08-03), continued -- category 10

Category 10 (pattern matching depth x generics/enums), all 3 rows
checked clean -- no bugs found:

- (row 1) `match` with bindings on a DEEPLY nested built-in enum
  payload (`Result<Option<T>, E>`, matched in ONE `match`
  expression): confirmed to need the same documented "two flat
  matches" workaround user-declared nested enums already require
  (`tutorials/src/beginner/08a_pattern_match_primer.md`) -- nesting
  `Result.Ok(Option.Some(v))` in one pattern is a clean PARSER
  rejection ("expected ')' (variant payload binding close)"), and
  the two-flat-matches rewrite compiles and runs correctly on both
  backends (verified with a 3-way branch: `Ok(Some(_))`, `Ok(None)`,
  `Err(_)`).
- (row 2) A guarded slice-pattern arm (`[a, b] if cond then ...`)
  combined with a GENERIC function `fn classify<T>(xs: Vec<T>) ->
  ...`: checked clean on both backends, for both an `i64` and an
  `f64` instantiation of `T` -- the guard is correctly evaluated and
  the slice-length dispatch works through the monomorphized element
  type exactly as it does for the non-generic case.
- (row 3) Or-pattern-shaped guard conditions (`if n == 1 || n == 2`)
  on an enum variant match arm, combined with the variant's payload
  BINDING used inside the guard expression itself: checked clean on
  both backends, for two different variants (`Circle`/`Square`) each
  with their own guarded and unguarded arm.
New tests: 3 `src/lib.rs` + 3 `tests/run_end_to_end.rs`. Full `cargo
test --release --workspace`: 0 failed. Category 10 (all 3 rows) now
fully closed in `docs/FEATURE_COMBINATION_GAPS_TODO.md`.

---

## Feature-combination gap audit sweep (2026-08-03), continued -- category 11 (FINAL)

- [x] **BUG-94 (found+fixed 2026-08-03). Category 11, row 1:
  `HashMap<StructKey, V>` with `self: ref Self` in the `Hash`/`Eq`
  impls -- EXACTLY what the checker's own diagnostic suggests --
  crashed both backends outright.** Found while confirming this
  boundary case is "a clean, consistent rejection" per the row's own
  framing; instead of a rejection, the checker's own suggested fix
  triggered a genuine crash. The by-value `self: Self` form (no
  `ref`) already worked and still does (verified as a no-regression
  check).
  Two SEPARATE bugs, one per backend, same root cause: the
  `HashMap<StructKey, V>` bundle (ARC 1.7, both
  `emit_intent_hashmap_struct_pair_c_body` in `backend_c.rs` and
  `emit_intent_hashmap_struct_pair_llvm` in `backend_llvm.rs`)
  hard-coded an assumption about how the user's `Hash`/`Eq` impl
  methods take `self`, instead of matching however the user actually
  declared it:
  - C backend: always forward-declared `fn_Key_hash`/`fn_Key_eq` as
    taking the struct BY VALUE (`Struct_Key self`). When the user
    writes `self: ref Key`, the REAL hoisted function (emitted by the
    ordinary interface-method codegen) takes a POINTER (`const
    Struct_Key* v_self`) -- two conflicting C declarations of the
    same symbol: `error: conflicting types for 'fn_Key_hash'; have
    'int64_t(const Struct_Key *)' ... previous declaration ...
    'int64_t(Struct_Key)'`.
  - LLVM backend: always CALLED `fn_Key_hash`/`fn_Key_eq` with a
    bare-value argument (`{k} %k`). When the real function takes a
    pointer, this is an ill-typed `call` -- `lli` crashed the JIT
    outright (SIGSEGV in `llvm::orc::runAsMain`).
  Fixed by adding a registry per backend
  (`IMPL_METHOD_SELF_BY_REF_REGISTRY` in `backend_c.rs`,
  `LLVM_IMPL_METHOD_SELF_BY_REF_REGISTRY` in `backend_llvm.rs`),
  populated at the start of each backend's `emit_*` from
  `program.functions`, keyed by the hoisted method's own first-
  parameter type (`Type::Ref`/`Type::RefMut` -> by-ref; anything else
  -> by-value) -- mirroring the SAME "sibling walker never learned
  about a shape" root-cause pattern this whole sweep kept finding,
  just applied to a calling-convention mismatch instead of a missing
  type-walk arm. A subtlety specific to LLVM: the hoisted method's
  own name is stored WITHOUT the `fn_` emission prefix
  (`program.functions` entries are named e.g. "Key_hash"; `fn_` is
  added only at C/LLVM text-emission time), so the registry keys by
  the pre-mangled `fn_`-prefixed spelling to match how the HashMap
  bundle's own `hash_fn`/`eq_fn` strings are already constructed --
  missing this on the first attempt at the fix left the registry
  lookup silently falling back to "by-value" for every entry (a
  quiet, easy-to-miss failure mode worth flagging for future
  registry-based fixes of this shape).
  The HashMap bundle now matches whichever convention the impl
  actually uses: the C forward declaration picks pointer vs value
  param types accordingly; the LLVM bundle spills `%k` (the by-value
  HashMap-API parameter) into a fresh stack slot via `alloca`+`store`
  to get an address when by-ref is needed (an LLVM SSA value isn't
  otherwise addressable, unlike C locals/array elements), and reuses
  an already-addressable `getelementptr` result (`%kcell`, the
  table-slot side of an eq comparison) directly instead of
  redundantly loading-then-respilling it.
  Verified with a fuller round-trip exercising the whole API (3
  inserts, `get`, `contains_key` hit/miss, update-returns-old-value,
  remove-returns-old-value, `len`) on both backends; `valgrind
  --leak-check=full` on native AOT builds of both backends: 0 errors,
  all heap blocks freed. Given the blast radius (touches HashMap
  <StructKey, V> codegen in both backends), ran the full `cargo test
  --release --workspace` suite immediately after the fix compiled
  (before adding new tests): 13/13 binaries clean, 0 failed.
  Also swept the rest of category 11:
  - (row 2) `Atomic<Vec<T>>` / `Atomic<Struct>` (non-i64-width
    payload): checked clean -- cleanly rejected on both backends,
    matching the documented i64-width-only restriction.
  - (row 3) A `dyn Iface` method call held across an `.await` point:
    checked clean, but surfaced a DOCUMENTATION-ACCURACY finding --
    `docs/missing_features.md` documented this shape as unsupported
    ("dyn-method receivers can't be held across suspend points"), but
    it actually works correctly on both backends, verified with a
    `dyn` binding held across TWO separate `await` points (method
    called both before and after the second await) and two different
    concrete types behind the same binding, values matching hand-
    computation exactly (`87`, `143`). Corrected the stale entry in
    `docs/missing_features.md`.
  - (row 4) `Mutex<T>`/`RwLock<T>` where `T` is itself a `Mutex<U>`/
    `RwLock<U>` (nested locks): found a real gap -- this used to
    compile straight through the checker and crash the native
    toolchain (undefined `intent_mutex_intent_mutex_i64` bundle
    symbols never generated by either backend; full nested-lock
    codegen support was never implemented, a substantially larger
    task than a missing-arm fix). Per the row's own "either works
    correctly or is cleanly rejected; either is fine" framing, fixed
    with an explicit, clean rejection in `mutex_new`/`rwlock_new`'s
    own type-checking, covering all four nesting combinations
    (`Mutex<Mutex<T>>`, `Mutex<RwLock<T>>`, `RwLock<Mutex<T>>`,
    `RwLock<RwLock<T>>`) with a clear diagnostic pointing at the fix
    ("nested concurrency handles are not supported in v1 ... Use a
    single Mutex<T>/RwLock<T> around the innermost data").
  New tests: 4 `src/lib.rs` + 2 `tests/run_end_to_end.rs`. Full
  `cargo test --release --workspace` after all category 11 changes
  landed: 13/13 binaries clean, 0 failed. Category 11 (all 4 rows,
  the FINAL category of the 11-category feature-combination gap
  audit sweep) now fully closed in `docs/FEATURE_COMBINATION_GAPS_
  TODO.md`.

---

## Deferred-finding fixups (2026-08-03), post-sweep

- [x] **BUG-95 (found+fixed 2026-08-03). Deferred finding from BUG-93:
  a bare enum constructor written directly inside a struct-literal
  field (`Node { value: 2, next: Option.Some(box(tail)) }`, with no
  intermediate `let` to hold the value first) failed on both
  backends.** The BUG-93 writeup had diagnosed this as a "BUG-46-
  class ambiguity" (multiple generic-enum instantiations in scope,
  receiver resolution can't tell which one a bare constructor means)
  and shipped a workaround-pattern test instead of a fix. The real
  shape turned out to be THREE compounding bugs, only the first of
  which resembled the original diagnosis:
  1. BUG-46's existing fix (`resolve_bare_enum_ctors_in_stmt`) only
     ever looked at a `let`'s top-level initializer or a `return`
     value -- never inside a struct-literal's OWN fields. Fixed by
     adding `resolve_bare_enum_ctor_in_struct_lit`, which looks up
     the struct literal's (already-monomorphized) field types and
     resolves any enum-typed field's bare constructor the same way,
     recursing into nested struct literals.
  2. With (1) alone, the rewrite still silently failed: the pass that
     resolves a `StructLit`'s own `type_name` from the bare generic-
     template name ("Node") to the mangled monomorphized name
     ("Node__i64") (`resolve_bare_struct_lits_in_stmt`) ran AFTER the
     enum-ctor pass at all 3 call sites in
     `monomorphize_type_decls_in_program`, so (1)'s field-type lookup
     (keyed by the mangled name) always missed on a still-bare
     `type_name`. Fixed by reordering the two passes at all 3 sites,
     with a comment explaining why the order matters.
  3. Even with (1) and (2) fixed, the target enum this now correctly
     resolved TO didn't actually exist: `substitute_type_param`'s
     `Type::Apply` -> `Type::Enum`/`Type::Struct` collapse (BUG-90's
     fix) rewrites a struct field's type IN PLACE, eagerly, the
     moment its args become concrete -- discarding the `(name, args)`
     pair that produced it before `collect_apply_in_ty` (BUG-93's
     worklist-feeding fix, which walks a freshly-monomorphized
     struct's fields looking for `Type::Apply` nodes still needing
     generation) ever gets a chance to see it. Concretely:
     `Node__i64`'s `next` field correctly ends up typed
     `Type::Enum("Option__Box_Struct__Node__i64___")`, but
     `program.enums` never actually contained an `EnumDecl` by that
     name -- "enum '...' is not declared" the instant anything
     referenced it, including a completely unrelated-looking
     `Option.None` reference in a sibling `let` that had never
     touched the new code paths at all (traced via temporary debug
     `eprintln!`s dumping `program.enums`'s actual contents, since
     the failure mode gave no indication the missing piece was
     enum-generation rather than receiver-resolution). This exactly
     explains why the BUG-93 workaround (an explicit
     `let: Option<Box<Node<i64>>>` elsewhere in the same function)
     worked: that path keeps a genuine, literal `Type::Apply` node in
     the AST long enough for the normal discovery walk to find it,
     independent of the struct-field collapse. Fixed with a new
     thread-local queue (`NEWLY_COLLAPSED_GENERIC_APPLIES` in
     `checker.rs`) that the collapse site in `substitute_type_param`
     records `(name, args, is_enum)` into on its way to erasing the
     `Apply` shape; the same worklist round in
     `monomorphize_type_decls_in_program` that already drains
     `collect_apply_in_ty`'s discoveries into `discovered_structs`/
     `discovered_enums` now also drains this queue, right after both
     the struct-field and enum-variant-payload substitution passes.
     Same "sibling/parallel code path silently missing a case" root-
     cause family this whole sweep kept surfacing, just one level
     deeper than the original diagnosis reached.
  Verified with `valgrind --leak-check=full` on native AOT builds of
  both backends: LLVM 0 errors; the C backend reproduces the SAME
  pre-existing, already-separately-tracked leak in
  `Box<StructWithHeapOwningFields>` Drop (not a new regression --
  see the deferred finding in BUG-93's own writeup). No regression on
  the BUG-93 workaround-pattern test, which continues to pass
  unchanged.
  New tests: 1 `src/lib.rs` + 1 `tests/run_end_to_end.rs`. Full
  `cargo test --release --workspace`: 0 failed.

- [x] **BUG-96 (found+fixed 2026-08-03). Deferred finding from
  BUG-93: field access through a bare `Box<T>` (`n.value` where
  `n: Box<Node>`) was rejected outright ("field access on non-struct
  type Box<...>").** `Box<T>` (T != `dyn Iface`) lowers to a bare
  `T*` in both backends (confirmed in `c_type_name`/`llvm_byte_size`)
  -- bit-identical to Ref/RefMut's own runtime representation -- but
  the checker's FieldAccess resolution only ever unwrapped
  `Ref`/`RefMut` via `Type::deref()`, never `Box`, and both backends'
  FieldAccess codegen gated the pointer-load path on `is_any_ref()`,
  which is likewise Ref/RefMut-only.
  Fixed with two new, narrowly-scoped `Type` helpers in `ast.rs`:
  `is_field_access_indirect()` (true for Ref/RefMut and non-dyn Box)
  and `deref_through_box()` (like `deref()`, but also peels one
  non-dyn Box layer) -- used ONLY at the three FieldAccess
  resolution/codegen sites (`checker.rs`'s `ExprKind::FieldAccess`
  arm, `backend_c.rs`'s `TypedExprKind::FieldAccess` arm,
  `backend_llvm.rs`'s `TypedExprKind::FieldAccess` arm in
  `emit_expr`). Deliberately NOT folded into the general-purpose
  `is_any_ref()`/`deref()`: those are consulted by 60+ call sites
  across checker.rs/backend_c.rs/backend_llvm.rs/ssa.rs/smt.rs,
  many of which assume Ref/RefMut's specific *borrowed* semantics
  (borrow-checking, move analysis, drop) -- conflating an owned
  `Box` with a borrow there would be a much larger, riskier change
  than this fix warrants. `Box<dyn Iface>` is deliberately excluded
  and remains cleanly rejected: it lowers to the 16-byte fat-pointer
  struct itself (with an owning `.data` pointer), not a pointer to a
  field-bearing aggregate, so it genuinely has no fields to read;
  verified this exclusion holds (no regression) with a dedicated
  spot-check. The SSA fast-path emitters (`ssa_backend_c.rs`,
  `ssa_backend_llvm.rs`) don't implement `FieldAccess` at all (their
  instruction set doesn't cover struct field access), so no change
  was needed there.
  Also updated `tutorials/src/intermediate/03a_box_raii_primer.md`,
  which had documented this exact gap as "a real v1 boundary" after
  the BUG-93 investigation surfaced it -- replaced the caution note
  with a short confirmation that field access now works through
  `Box<T>` the same way it works through a ref.
  Verified on both backends with a struct having a direct `Box<T>`
  binding AND a `Box<T>`-typed struct field (covering the
  lvalue-chaining path, e.g. `n.next.x`).
  New tests: 1 `src/lib.rs` + 1 `tests/run_end_to_end.rs`. Full
  `cargo test --release --workspace`: 0 failed.

- [x] **BUG-97 (found+fixed 2026-08-03). Deferred finding from
  BUG-93/task #39: the canonical `Node { next: Option<Box<Node>> }`
  recursive-struct shape (the one the Box<T>/RAII tutorial itself
  demonstrates, `examples/language/english/
  option_box_recursive_struct.vani`) leaked on the C backend --
  root cause was much deeper than "Drop doesn't recurse into the
  Box".** `valgrind` on the shipped example showed 2 of 3 mallocs
  (48 of 72 bytes) never freed even at the OUTERMOST Box; the
  generated C for `fn_main` contained ZERO `free()` calls at all for
  any of the three `Node` locals.
  Root cause, found by dumping generated C and checking `is_copy()`
  by hand: `Type::Enum`'s non-Copy-payload registration (checker.rs)
  only ever checked a hardcoded `matches!(t, Type::OwnedStr |
  Type::Vec(_))` -- predating `Box<T>` and every other affine type
  added since -- so `Option<Box<Node>>` was silently treated as
  Copy. That made `Node`'s own field check (`!f.ty.is_copy()`) pass
  too, so `Node` itself registered as Copy, so the checker never
  emitted a scope-exit `TypedStmt::Drop` for a `Node` local AT ALL --
  not a "doesn't recurse" bug, a "never even starts" bug. Compounded
  by a second, independent gap: the struct-non-Copy pass and the
  enum-non-Copy pass ran as two separate ONE-SHOT passes (structs
  first, enums second), so even a corrected enum check would have
  been invisible to the struct pass on the same run -- a struct
  whose Copy-ness depends on an enum field, and an enum whose Copy-
  ness depends on a struct payload, are mutually dependent and need
  a SHARED fixed point, same "sibling/parallel walk needs a shared
  fixed point" root-cause family as the generics-monomorphization
  worklist bugs (BUG-90/93/95).
  Fixed in three layers:
  1. checker.rs: merged the struct and enum non-Copy registration
     into one fixed-point loop (was two separate one-shot passes),
     and switched the enum-payload check from the hardcoded
     `OwnedStr | Vec` match to `!payload.is_copy()` (self-updating
     as new affine types are added, matching how the struct-field
     check already worked).
  2. checker.rs: correctly-classified `Node` then hit TWO more
     gaps this had been masking: the struct-field-type allowlist had
     no `Type::Enum` arm (rejecting the field outright), and `box()`
     rejected any non-Copy struct argument outright (rejecting
     `box(tail)`). Both relaxed: `Type::Enum(_)` added to the field
     allowlist (Drop chains through it the same way a nested struct
     field already does); `box()`'s struct gate changed from
     requiring `is_copy()` to unconditional `true` (any struct
     reaching this point already passed the field-type allowlist, so
     its Drop is guaranteed to be a shape both backends know how to
     walk).
  3. backend_c.rs codegen, once the checker correctly asked for a
     `Node` Drop: `emit_struct_field_drops` had NO `Type::Enum` arm
     (silent no-op) and its `Type::Box` arm's fallback case for
     `Type::Struct` inner types just did `free(box)` -- the box's OWN
     slot, never the boxed struct's OWN owning fields. Same gap
     independently existed in the bare-local `TypedStmt::Drop`'s
     `Type::Box` arm AND in its `Type::Enum` arm's Box-payload case
     (three separate copies of "Box<T> Drop chaining," all missing
     the same `Type::Struct` case). Fixed by adding the `Type::Enum`
     arm (factored `emit_enum_value_drop` out of the bare-local
     handler so the tag-switch + per-variant-payload-free logic
     isn't tripled), and adding a `Type::Struct` case to all three
     `Type::Box` sites. For a NON-recursive `Box<Struct>` (no cycle
     back to itself), this inline-recurses via
     `emit_struct_field_drops`, same as an ordinary nested struct
     field -- safe, since a DAG always bottoms out. For a
     box-RECURSIVE struct (owns a `Box<Self>`, directly or through
     one layer of enum wrapping -- detected into a new
     `BOX_RECURSIVE_STRUCTS_REGISTRY` in ast.rs, populated by the
     checker), inline recursion would need infinitely much generated
     C text to unroll a cycle, so all three sites instead call one
     generated, ITERATIVE (heap-worklist-based, not native-call-
     recursive -- won't blow the C stack on a long chain) "deep drop"
     helper function per box-recursive struct type
     (`emit_box_recursive_deep_drop_helpers`), emitted once every
     struct/enum body is defined and before any function body that
     might drop one.
  The LLVM backend needed NO changes -- confirmed via `valgrind
  --leak-check=full` on native AOT builds that it already correctly
  drops every case this fix covers, both before and after (0 errors
  throughout), matching this bug's own original "LLVM backend is
  unaffected -- worth checking as a reference implementation" framing.
  Verified with `valgrind --leak-check=full --show-leak-kinds=all` on
  native AOT C builds of: the shipped 3-node example (0 errors, all
  heap blocks freed -- vs. "24 direct + 24 indirect bytes
  definitely/indirectly lost" before this fix); a 10-node chain where
  each node ALSO owns a plain `OwnedStr` field alongside the
  recursive `Box<Self>` edge (0 errors, 21/21 allocs freed --
  confirms the generated helper's non-recursive-field-drop pass and
  its worklist-push pass compose correctly); and the BUG-93/95
  generic `Node<T>` instantiation (0 errors -- this fix incidentally
  also closes THAT bug's own deferred C-backend leak finding, a nice
  two-for-one). No regression on any existing Box<T>/struct-field/
  enum-payload Drop test (166+31+... across the full suite, 0
  failed).
  Deferred, smaller finding along the way: a MIXED-payload enum
  (2+ variants with owning-but-different payload types) with a
  Box<Struct>-shaped variant payload still isn't handled in
  `emit_enum_value_drop`'s mixed-payload branch (only OwnedStr/Vec
  there) -- not needed by this bug's actual repro (`Option` is
  single-payload: only `Some` carries one), left as a documented gap
  rather than scope-creeping further.
  New tests: 1 `src/lib.rs` + 1 `tests/run_end_to_end.rs`. Full
  `cargo test --release --workspace`: 0 failed.

---

## Bug hunt (2026-08-04) -- user-requested, after BUG-91/95/98/87/36
## were all closed: "did you exhaust all feature combinations, think
## outside the box, are all bugs fixed"

All four found this pass share a thread: the fn-generics
monomorphization pipeline is walked by several independent, parallel
"find every use of X" scanners (call-site discovery, call-site
rewriting, bare-enum-ctor resolution), and each one turned out to
cover only a HANDFUL of the AST's `Stmt`/`ExprKind` shapes, not all
of them -- the exact same "duplicate walker silently missing a case"
root-cause family this whole multi-day session kept finding, just
four more instances of it, surfaced by deliberately probing MORE
exotic feature combinations than the original 11-category sweep
covered (nested generic-enum-of-generic-enum, generic calls inside
`if let`/`FieldAccess`, etc.).

- [x] **BUG-99 (found+fixed 2026-08-04). `collect_generic_calls_in_
  stmt`/`rewrite_generic_calls_in_stmt` -- the pair of walkers that
  discover a generic function's call sites and then mangle the
  call-site name once monomorphization knows the concrete type --
  only ever covered `Let`, `Assign`, `Return`, `Print`, `EPrint`,
  `PrintBlock`, `If`, `While`.** Every other `Stmt` variant
  (`LetTuple`, `Assert`, `Prove`, `Break`, `IndexAssign`,
  `FieldAssign`, `For`, `ForIter`, `TaskSpawn`, `UnsafeBlock`,
  `IfLet`, `WhileLet`, `Select`) silently fell through a catch-all
  `_ => {}`, so a generic call whose ONLY use site was one of them
  failed monomorphization outright ("generic function '...' is
  declared but never called with concrete types").
  Found via `if let Option.Some(y) = foo2(true) { ... }` where
  `foo2<T>` is generic -- `Stmt::IfLet`'s `scrutinee` was invisible
  to both walkers.
  Fixed by making both matches fully exhaustive (removed the
  catch-all entirely; Rust's own exhaustiveness check now guards
  against a future silent gap of this exact shape -- if a new `Stmt`
  variant is ever added without updating these two functions, the
  build fails instead of silently mis-compiling generic code).
  `ForIter`'s `collection` is a bare variable name (not an `Expr`),
  so its arm only needs to recurse into `body`; `Continue`/
  `TaskJoin` carry no expression at all.
  Repro (fails without the fix, on both backends):
  ```
  fn foo2<T>(a: T) -> Option<T> {
    return Option.Some(a);
  }
  fn main() -> i64 {
    if let Option.Some(y) = foo2(true) {
      print 1;
    }
    return 0;
  }
  ```
  New tests: 1 `src/lib.rs` + 1 `tests/run_end_to_end.rs`.

- [x] **BUG-100 (found+fixed 2026-08-04). Sibling gap to BUG-99, one
  layer down: `collect_generic_calls_in_expr`/`rewrite_generic_
  calls_in_expr` only ever covered `Call`, `Binary`, `Unary`, `Try`,
  `Match`, `Block`, `IfExpr`.** Every other `ExprKind` variant
  (`IndirectCall`, `MethodCall`, `Cast`, `ArrayLit`, `Index`, `Len`,
  `Ref`, `RefMut`, `Tuple`, `TupleAccess`, `StructLit`,
  `FieldAccess`, `WhileLoop`, `Forall`, `AnonFn`, `TaskSpawnCall`)
  silently fell through the same shape of catch-all.
  Found via `print make(3, 4).a;` where `make<T>` is a generic
  function returning a generic struct -- the call sits inside a
  `FieldAccess`, invisible to both walkers exactly the same way
  `IfLet`'s scrutinee was to BUG-99's.
  Fixed the same way: both matches now fully exhaustive.
  (`AnonFn`'s body is normally already lambda-lifted away -- replaced
  with a `Var` reference to a generated top-level fn -- before either
  pass runs; its new arm is defensive completeness, not load-bearing.)
  Repro (fails without the fix, on both backends):
  ```
  struct Pair<T> { a: T, b: T }
  fn make<T>(x: T, y: T) -> Pair<T> {
    return Pair { a: x, b: y };
  }
  fn main() -> i64 {
    print make(3, 4).a;
    return 0;
  }
  ```
  New tests: 1 `src/lib.rs` + 1 `tests/run_end_to_end.rs`.

- [x] **BUG-101 (found+fixed 2026-08-04). `substitute_type_param`'s
  `Type::Apply` collapse (the same eager-collapse logic BUG-90/95
  already touched) mangled a nested generic-enum type argument with
  the WRONG naming convention.** Two mangling conventions coexist in
  this codebase for a reason (per `type_mangle_for_decl`'s own doc
  comment): `type_mangle` is for FUNCTION specialization (`foo` ->
  `foo__i64`) and prefixes nominal types with "Enum_"/"Struct_"
  (`type_mangle(Type::Enum("Option__i64"))` -> `"Enum_Option__i64"`);
  `type_mangle_for_decl` is for STRUCT/ENUM DECL generation
  (`mangle_generic_decl`/`monomorphize_type_decls_in_program`'s
  worklist) and spells a nominal type's own bare name with NO prefix
  (`type_mangle_for_decl(Type::Enum("Option__i64"))` -> just
  `"Option__i64"`, so nested instantiations read as
  `Option__Option__i64` rather than `Option__Enum_Option__i64`). The
  two conventions agree for a scalar/struct T, but disagree the
  moment T is itself an already-resolved generic-enum instantiation
  -- exactly the shape `fn wrap<T>(x: T) -> Option<Option<T>>`
  produces once `T=i64` substitutes and the OUTER `Option<T>`
  collapses.
  `substitute_type_param`'s collapse used `type_mangle` (the wrong
  one for this purpose) -- `return Option.Some(...)`'s checked type
  ended up `Type::Enum("Option__Enum_Option__i64")`, a name the
  decl-generation worklist (which correctly uses
  `type_mangle_for_decl` throughout) never actually materializes a
  decl for. Surfaced as a genuine TYPE MISMATCH ("enum payload must
  be assignable to Option__i64, got i64"), not a missing-decl error,
  since the wrong name still looked like a plausible enum name --
  much harder to spot than BUG-91's "enum '...' is not declared"
  failure mode.
  One-line fix: use `type_mangle_for_decl` at the collapse site too.
  Repro (fails without the fix, on both backends):
  ```
  fn wrap<T>(x: T) -> Option<Option<T>> {
    return Option.Some(Option.Some(x));
  }
  fn main() -> i64 {
    let r: i64 = match wrap(9) {
      Option.Some(inner) then match inner {
        Option.Some(v) then v,
        Option.None then -1,
      },
      Option.None then -2,
    };
    print r;
    return 0;
  }
  ```
  New tests: 1 `src/lib.rs` + 1 `tests/run_end_to_end.rs`.

- [x] **BUG-102 (found+fixed 2026-08-04). One layer deeper than
  BUG-101, found immediately after fixing it: even with the correct
  mangled name, `resolve_bare_enum_ctor_receiver` (BUG-46's fix) only
  ever rewrote the OUTERMOST bare enum-constructor receiver -- it
  never recursed into a payloaded variant's own payload ARGUMENT to
  check whether THAT is itself a bare, unresolved constructor for a
  DIFFERENT generic instantiation.** For `Option.Some(Option.Some
  (x))`, the OUTER `Some`'s receiver correctly resolves to
  `Option__Option__i64` (once BUG-101 fixed the mangling), but the
  INNER `Option.Some(x)`'s own receiver -- which needs the DIFFERENT
  concrete name `Option__i64` -- stayed completely unresolved:
  "unknown variable 'Option'".
  Fixed by threading a new `enum_variant_payloads` map (`enum name ->
  (variant name, payload type) list`, built from the now-fully-
  monomorphized `program.enums` via a new `build_enum_variant_
  payloads` helper) through the whole `resolve_bare_enum_ctor_*`
  family (`resolve_bare_enum_ctor_receiver`, `resolve_bare_enum_ctor_
  in_struct_lit`, `resolve_bare_enum_ctors_in_stmt`, and all 4 of
  their top-level call sites in `check_program` +
  `monomorphize_type_decls_in_program`). `resolve_bare_enum_ctor_
  receiver` now recurses into a `MethodCall`'s single payload
  argument using the OUTER target enum's own declared payload type
  for the matched variant name -- correctly threading a DIFFERENT
  concrete instantiation down through each nesting level, however
  deep.
  Also verified through `await`, confirming the fix composes
  correctly with BUG-87's own fix (two different async generic fns,
  each specialized once, each internally constructing a bare
  `Option.Some(a)` needing a DIFFERENT concrete instantiation --
  `Option__i64` and `Option__bool`).
  Repro (fails without BUG-101+BUG-102, on both backends):
  ```
  fn wrap<T>(x: T) -> Option<Option<T>> {
    return Option.Some(Option.Some(x));
  }
  fn main() -> i64 {
    let r: i64 = match wrap(9) {
      Option.Some(inner) then match inner {
        Option.Some(v) then v,
        Option.None then -1,
      },
      Option.None then -2,
    };
    print r;
    return 0;
  }
  ```
  New tests: 1 `src/lib.rs` + 1 `tests/run_end_to_end.rs` (the
  triple-nested-via-async shape, exercising composition with BUG-87).
  **All four (BUG-99/100/101/102) verified together**: full `cargo
  test --release --workspace`, 0 failed, no regression on the
  existing generics/async/pattern-matching test surface built up
  over this whole session.
  **Two smaller findings from the same hunt, deferred to keep this
  batch reviewable**: a THIRD level of nesting
  (`Option<Option<Option<T>>>`) ran correctly on the LLVM backend
  but failed to COMPILE on the C backend -- **FIXED separately as
  BUG-103, see below**. Also: `Result<Option<T>, i64>` as a generic
  function's return type (a 2-argument builtin generic enum where
  only ONE arg depends on T) never resolved at all -- **FIXED
  separately as BUG-104, see below**.

- [x] **BUG-103 (found+fixed 2026-08-04, task #44). Three levels of
  nested generic enum (`Option<Option<Option<T>>>`) ran correctly on
  LLVM but failed to COMPILE on the C backend** -- `cc` error:
  "unknown type name 'Enum_Option__Option__i64'; did you mean
  'Enum_Option__i64'?".
  Root cause: `emit_c`'s "unified topological emit" loop (the system
  deciding emission order for structs/enums whose fields/payloads
  reference each other) has THREE parallel sub-loops -- Vec bundles,
  structs, and deferred payloaded enums -- each checking whether ITS
  OWN dependencies are already emitted before emitting itself. The
  STRUCT sub-loop correctly checks struct deps (`sdeps`/`sok`),
  Vec-bundle deps (`vdeps`/`vok`), AND enum deps (`edeps`/`eok`,
  added 2026-06-09 per its own comment). The DEFERRED-ENUM sub-loop
  -- which exists specifically to handle an enum whose OWN payload
  needs a full struct/enum/tuple definition, deferred from the
  earlier eager pre-emit pass -- computed `sdeps`/`vdeps` from its
  payload types but NEVER `edeps`, so an enum whose payload is
  ANOTHER deferred enum (exactly what 3+ levels of `Option<T>`
  nesting produces: the outer two levels both defer, since a
  `Type::Enum` payload needs a full def) could get emitted before
  the enum it depends on. Same "duplicate walker missing a case"
  root-cause family as BUG-99/100/101/102 (found in the SAME
  session) -- just living in backend_c.rs's own topological-emission
  system instead of the checker's generics-monomorphization
  pipeline.
  Fixed by adding the missing `edeps`/`eok` check to the deferred-
  enum sub-loop, mirroring the struct sub-loop's existing pattern
  exactly.
  Considered (and reverted after confirming it wasn't needed) an
  alternative checker-level fix that sorted `monomorphize_type_
  decls_in_program`'s freshly-generated decls into dependency order
  before appending them to `program.structs`/`program.enums` --
  verified working on its own, but proven UNNECESSARY once this
  backend_c.rs fix landed (the C backend's own topo-loop now
  correctly self-corrects regardless of input order; LLVM never
  needed correct input order in the first place, which is why it was
  never affected). Kept the smaller, more targeted fix rather than
  shipping the redundant one.
  Verified against the original 3-level repro AND a 4-level variant
  (`Option<Option<Option<Option<T>>>>`) on both backends, plus full
  `cargo test --release --workspace`: 0 failed.
  New tests: 1 `src/lib.rs` + 1 `tests/run_end_to_end.rs`.

- [x] **BUG-104 (found+fixed 2026-08-04, task #45, user-requested
  after BUG-99..103 landed: "fix it"). `Result<Option<T>, i64>` as a
  generic function's return type -- a 2-argument BUILTIN generic
  enum where only ONE arg depends on the function's own T -- never
  resolved at all.**
  Root cause: `substitute_type_param`'s own `Type::Apply` collapse
  (the same collapse BUG-101 already touched) was hard-gated on
  `args.len() == 1`, so a 2-arg Apply was silently SKIPPED and never
  collapsed to `Type::Enum`/`Type::Struct` in the first place -- the
  checker never even attempted to resolve it, so the diagnostic
  showed the raw, un-collapsed `Type::Apply`'s own Display output
  ("Result<Option__i64, i64>", not a mangled name) instead of a
  "not declared" or type-mismatch error.
  The gate's own original comment said it "matches the v1-supported
  `template.type_params.len() != 1` check at fn-mono" -- but that
  restriction is about USER-DEFINED GENERIC FUNCTIONS having at most
  one type parameter (a real, still-enforced v1 limit), which has
  nothing to do with how many args a BUILTIN generic enum/struct
  `Type::Apply` node this collapse fires on can have.
  `mangle_generic_decl` -- used by the decl-generation worklist for
  the exact same "build a mangled name from a template name +
  concrete args" purpose -- already looped over an arbitrary number
  of args correctly; this collapse just never matched it, both in
  the length gate AND (per BUG-101) in which mangling convention it
  used.
  Fixed by removing the length gate entirely and reusing `mangle_
  generic_decl` directly (instead of hand-building the mangled
  name), so 1-arg (`Option<T>`), 2-arg (`Result<T,E>`), and any
  future N-arg generic all collapse through the same unified code
  path.
  Verified against `Result<Option<T>, i64>` with a scalar-T `Ok`
  payload's `Some`/`None` and a genuine `Err` path (three distinct
  concrete tag combinations exercised in one program), plus a
  simpler `Result<T, i64>` (no nested `Option`) Ok/Err round trip, on
  both backends. No regression on BUG-101/102's own nested-`Option
  <Option<T>>` tests (the same collapse site, now handling both the
  1-arg and 2-arg shapes through one unified path instead of two).
  Repro (fails without the fix, on both backends):
  ```
  fn wrap<T>(x: T) -> Result<Option<T>, i64> {
    return Result.Ok(Option.Some(x));
  }
  fn main() -> i64 {
    let r: i64 = match wrap(6) {
      Result.Ok(opt) then match opt {
        Option.Some(v) then v,
        Option.None then -1,
      },
      Result.Err(e) then e,
    };
    print r;
    return 0;
  }
  ```
  New tests: 1 `src/lib.rs` + 1 `tests/run_end_to_end.rs`. Full
  `cargo test --release --workspace`: 0 failed.

This closes out the 2026-08-04 bug hunt: all six findings (BUG-99
through BUG-104) are now fixed and shipped, alongside the separate
BUG-36 documentation reconciliation from earlier the same day. No
further open items from this pass.

## localfuzz sweep (2026-08-04) -- FIXED, shipped commit 79dc457

`tools/localfuzz/` runs continuously in a separate worktree
(`/home/virgo/source/vani-compiler-localfuzz`, branch
`local-fuzz-findings`), mutating the shipped example programs and
diffing C-vs-LLVM behavior via a local Ollama qwen2.5-coder:1.5b
model. `scripts/localfuzz_status.py` clusters and dedupes its
findings for review. This pass reviewed the 2 findings clusters it
flagged as needing a real look; both were genuine, fixed below.

- **BUG-105 (found by tools/localfuzz, finding
  20260804-151851-backend-divergence-a0a31dce79). Two function
  parameters with DIFFERENT non-ASCII names collided into the SAME C
  identifier, failing to compile on the C backend while the LLVM
  backend compiled and ran fine.**
  Repro (Burmese, mutated from `examples/language/burmese/keywords.
  vani`): `fn ပေါင်း(က: i64, ခ: i64) -> i64 { return က + ခ; }`. C:
  `cc` error `redefinition of parameter 'v__'` (both params sanitized
  to the literal identifier `v__`). LLVM: ran fine, "OK 7".
  Root cause: `sanitize_ident` (backend_c.rs), used by both
  `function_name` (`fn_<sanitized>`) and `local_name`
  (`v_<sanitized>`) to turn a vāṇी identifier into a valid C
  identifier, mapped EVERY non-ASCII character to a single literal
  `_` regardless of which character it was -- so any two non-ASCII
  names of the same byte length (like two single-character Burmese
  parameter names) sanitized to the identical string. Purely a
  C-backend concern: the LLVM backend doesn't route source
  identifiers through this sanitizer.
  Fixed by encoding each non-ASCII character's Unicode codepoint as
  `_u<hex>_` instead of collapsing it to `_`, making the mangling
  collision-resistant while staying a valid C identifier.
  New tests: 1 `src/lib.rs` (`non_ascii_identifier_collision_
  compiles_to_c_lib`, checks the generated C for the specific
  duplicate-parameter shape) + 1 `tests/run_end_to_end.rs`
  (`non_ascii_identifier_collision_produces_correct_output_on_both_
  backends`, full subprocess run + output check on both backends).

- **BUG-106 (found by tools/localfuzz, finding
  20260804-135616-backend-divergence-d515a0fcc9, plus a follow-up
  hand-written bare-assert probe during triage). A failed `assert`
  diverged between the C and LLVM backends in two independent ways.**
  Repro A (Sinhala, mutated from `examples/language/sinhala/
  option_types.vani`, deliberately-wrong assertion): failing
  `assert x == 1, ...`-style (message-carrying) assert. C: exit 1,
  stderr `Assertion \`(v_x == 1)' failed.` (glibc `assert()` /
  `abort()` -> SIGABRT). LLVM: exit 3, `dprintf`-based message
  (already fixed for asserts back in MATH-3, 2026-07-20).
  Repro B (hand-written, to isolate the message-less case): `fn
  main() -> i64 { let x: i64 = 2; assert x == 1; return 0; }` run via
  `vanic run` (LLVM, default backend) crashed `lli` outright with a
  garbage stack-dump, rather than exiting cleanly at all.
  Root cause, Part A (exit code / message, message-carrying asserts):
  both the tree C backend (`backend_c.rs`) and the SSA fast-path C
  backend (`ssa_backend_c.rs`, tried FIRST by `vanic run`'s
  `emit_c_via_ssa`, tree is only the fallback) independently called
  `abort()` on a failed assert -- two separate copies of the same
  codegen, same "duplicate code path missed a fix" pattern as the
  BUG-99..104 hunt. SIGABRT termination makes `status.code()` return
  `None` in `main.rs`'s process driver, which then falls into
  `.unwrap_or(1)`. The LLVM backend already used `exit(3)` (MATH-3),
  precisely to avoid `vanic run`/`lli` misreporting a failed assert
  as an apparent native stack overflow.
  Fixed by switching both C codegen paths to `exit(3)`: added a
  `VANIC_ASSERT` macro to backend_c.rs's C preamble (used for the
  no-message arm, preserving glibc `assert()`'s own file/line/
  function/condition diagnostic format without letting it actually
  `abort()`), and switched the message-carrying arm's `abort();` to
  `exit(3);` in both backend_c.rs and ssa_backend_c.rs's
  `intent_assert_fail` handler. `#include <assert.h>` stays in the
  preamble -- two OTHER, unrelated native `assert()` call sites exist
  for `requires`-clause precondition codegen, and both backends
  already consistently `abort()` there (verified against
  backend_llvm.rs's own `requires`-clause codegen) -- not a
  divergence, deliberately left untouched.
  Root cause, Part B (true undefined behavior, message-LESS asserts,
  SSA fast path only -- the default for `vanic run`): the shared SSA
  lowering (`ssa.rs`, `TypedStmt::Assert`) skipped calling the
  runtime abort helper (`intent_assert_fail`) entirely when the
  assert had no message, terminating the fail block with
  `Terminator::Unreachable` on the assumption ("the SMT pass should
  already have proven it can't be reached") that isn't actually
  enforced anywhere -- there is no proof requirement gating an
  ordinary runtime `assert expr;` statement in the checker. The two
  SSA backends disagree on what `Terminator::Unreachable` means: the
  C SSA backend (`ssa_backend_c.rs`) lowers it to a safe `abort()`
  everywhere, which happened to mask this bug there; the LLVM SSA
  backend (`ssa_backend_llvm.rs`) lowers it to LLVM's actual
  `unreachable` instruction, which is genuine undefined behavior if
  control reaches it at runtime -- reproducing exactly as the `lli`
  JIT stack-dump crash in repro B.
  Fixed by having the message-less case in `ssa.rs` ALSO call
  `intent_assert_fail` (with an empty-string message, reusing the
  same call path as the message-carrying case), so the fail block is
  a real, well-defined `exit(3)` call on both backends instead of an
  unproven "unreachable" assumption. Output for a message-less
  failed assert is now `assertion failed: ` (empty trailing message)
  on the SSA path -- a minor, pre-existing-style cosmetic asymmetry
  vs. the tree LLVM backend's fully-silent bare-assert exit(3) (never
  printed anything for the no-message case either), not a new
  regression.
  Note: `ssa_backend_llvm.rs`'s own `intent_assert_fail` handler
  already correctly emitted `call void @exit(i32 3)` -- confirmed by
  reading it directly -- so no separate LLVM-side fix was needed
  there; the LLVM SSA path's bug was entirely that message-less
  asserts never reached that handler at all (Part B above).
  New tests: 1 `src/lib.rs` (`failed_assert_exit_code_and_message_
  match_across_backends_lib`, compiles both a message-carrying and a
  message-less failing assert on both backends) + 1
  `tests/run_end_to_end.rs`
  (`failed_assert_exit_code_and_message_match_across_backends`, full
  subprocess run checking actual process exit code == 3 and matching
  stderr content across backends for both shapes).
  Also updated 3 pre-existing `src/lib.rs` tests
  (`assert_without_message_still_uses_c_assert`,
  `assert_without_message_uses_c_assert_macro`,
  `assert_with_message_lowers_to_custom_abort`) that had hard-coded
  the OLD `assert(...)`/`abort();` C output shape from before this
  fix.
  regression.

## localfuzz handoff (2026-08-04) -- Issue 1 FIXED

Continuation of the sweep above. `docs/LOCALFUZZ_HANDOFF_2026-08-04.md`
handed off 4 confirmed-open findings to a fresh session. This entry
covers the first (most contained, per the handoff's own recommended
order); the other 3 are still open and tracked separately.

- **BUG-107 (found by tools/localfuzz, finding
  20260803-130927-backend-divergence-dc30074c7a). A struct field of
  type `Vec<Box<dyn Iface>>` failed to compile on the C backend while
  the LLVM backend ran it fine.**
  Repro: `struct Scene { name: OwnedStr, shapes: Vec<Box<dyn
  Drawable>>, ids: Vec<i64> }`, populated via `box(Circle { r: 5 } as
  dyn Drawable)` literals. C: `cc` rejected the generated output with
  a cascade of errors rooted in `error: unknown type name
  'intent_dyn_Drawable'` inside the `intent_vec_box_dyn_Drawable`
  bundle's helper functions (`__set`, `__set_mut`, `__clone`,
  `__free`, `__from`). LLVM: ran fine (SSA values have no textual
  forward-declaration ordering to get wrong).
  Root cause: `backend_c.rs`'s `vec_element_has_user_struct` helper
  decides whether a struct field's `Vec<T>` bundle must be DEFERRED
  to the unified topo-emission loop (which runs after
  `emit_dyn_iface_typedefs` has landed the per-Iface
  `intent_dyn_<Iface>` fat-pointer typedef) or can be emitted in the
  early, non-deferred pass. It already had a `Type::Object(_) => true`
  arm so a bare `Vec<dyn Iface>` struct field correctly deferred. It
  recursed into `Type::Vec`/`Type::Array` element types, but had NO
  arm for `Type::Box` -- so `Vec<Box<dyn Drawable>>`'s element type
  (`Type::Box(Type::Object("Drawable"))`) fell through to `_ =>
  false`, was wrongly classified as having no forward-reference
  dependency, and its bundle was emitted in the EARLY pass -- textually
  before `intent_dyn_Drawable`'s typedef, which only lands later via
  `emit_dyn_iface_typedefs`. (`Box<dyn Iface>`'s C storage is the
  16-byte fat-pointer struct BY VALUE, not a pointer to it -- see
  `c_element_storage`'s `Type::Box` arm -- so, unlike most `Box<T>`
  uses, this really does need the full typedef, not just a forward
  declaration, before the bundle referencing it.) Plain
  (non-struct-field) `let xs: Vec<Box<dyn Iface>> = ...;` locals were
  unaffected -- confirmed via manual repro -- because that path
  doesn't route through `vec_element_has_user_struct` at all.
  Fixed by adding a `Type::Box(inner) =>
  vec_element_has_user_struct(inner)` arm, mirroring the existing
  `Vec`/`Array` recursive arms, so the Box's inner type is walked and
  its `Type::Object(_) => true` arm catches the dyn-Iface dependency
  correctly.
  New tests: 1 `src/lib.rs`
  (`vec_of_box_dyn_iface_struct_field_compiles_to_c`, compiles the
  repro shape and asserts the `intent_dyn_Drawable` typedef's byte
  offset in the generated C precedes the
  `intent_vec_box_dyn_Drawable` bundle's) + 1 `tests/run_end_to_end.rs`
  (`vec_box_dyn_iface_struct_field_example_produces_correct_output_on_both_backends`,
  full subprocess run against a new example,
  `examples/language/english/vec_box_dyn_iface_struct_field.vani`,
  checking real `cc`-compiled output on both backends). Existing
  `Box<dyn Iface>` / `Vec<dyn Iface>` / `Vec<Box<dyn Iface>>` (LLVM,
  non-struct-field) tests all still pass -- confirmed no regression.

## localfuzz handoff (2026-08-04) -- Issue 2 FIXED (re-scoped, more severe than described)

Continuation of the handoff sweep (see BUG-107 above). The handoff
described this finding as a narrow `mut ref Vec<T>` write-back bug
("C's `topo_sort` out-param doesn't write back; LLVM handles it
fine"). Re-verifying against a fresh rebuild before trusting the
handoff's characterization (per its own section 0) turned up something
different and considerably worse.

- **BUG-108 (found by tools/localfuzz, finding
  20260803-144958-backend-divergence-2125e1a114 -- but NOT the bug
  that finding was originally attributed to). The tree-walking LLVM
  backend's Vec index read, index write, and mut-ref-element codegen
  had NO runtime bounds check at all.**
  Original repro (`examples/graph_algo2.vani`-derived, `graph_new(-1)`
  fuzzer-mutated): LLVM printed `astar dijkstra-like 0->4: -1` (and 3
  more astar calls, all wrongly `None`) followed by `topo_sort count:
  0` then `topo[ 0 ] = 94480208184997` (garbage) instead of the
  expected clean `index out of bounds` abort C produces for the same
  degenerate 0-node graph. The handoff's "C's out-param write-back is
  broken, LLVM is fine" read was backwards -- LLVM was ALSO wrong, in
  a different and more dangerous way (silently returning garbage
  instead of trapping, rather than cleanly aborting like C).
  Root cause has nothing to do with `Graph`/`astar`/`topo_sort`
  specifically -- `graph_new(-1)` was a red herring. Bisected with a
  minimal repro requiring only ANY struct-typed local declared before
  an out-of-range Vec index:
  ```
  struct Foo { a: i64, b: i64 }
  fn main() -> i64 {
    let g: Foo = Foo { a: 1, b: 2 };
    let order: Vec<i64> = vec();
    for i from 0 to 5 { print order[i]; }
    return 0;
  }
  ```
  `vanic run` on this (LLVM, default backend) silently read garbage
  memory and exited 0; the same construct WITHOUT the `Foo` struct
  local correctly aborted via `@__intent_bounds_check` (confirmed:
  removing only the struct declaration flips the behavior). The
  reason: `main.rs`'s `emit_llvm_via_ssa` routes a whole program
  through either SSA-LLVM (`ssa_backend_llvm.rs`, which DOES have
  `@__intent_bounds_check`) or, if `ssa_path_supports` rejects
  anything the program uses, falls back ENTIRELY to tree-LLVM
  (`backend_llvm.rs`). `expr_ssa_supported` unconditionally rejects
  `TypedExprKind::StructLit` and `TypedExprKind::FieldAccess` (`=>
  false`, no exceptions) -- so ANY struct literal or field access
  anywhere in the program forces the WHOLE program onto tree-LLVM.
  Tree-LLVM's `TypedExprKind::Index` (read), `TypedStmt::IndexAssign`
  (write), and `TypedExprKind::RefMutIndex` (`mut ref vec[i]`)
  codegen -- and the Vec<bool> packed-bit read path plus its
  `@intent_vec_bool__set_mut` write helper -- all did a raw
  GEP-then-load/store with no length check whatsoever. This is not a
  rare corner: struct literals and field access are basic, extremely
  common language features (also several dozen builtins are
  independently denylisted from SSA in `expr_ssa_supported`,
  including `graph_new`/`graph_astar`/`graph_topo_sort`, `push_mut`,
  most `str_*` builtins, etc.), so any realistic program combining
  "uses a struct" with "indexes a Vec with a non-provably-in-range
  index" was running with silently disabled bounds checking on the
  LLVM backend.
  Fixed by adding a `@__intent_bounds_check(i64 %idx, i64 %len)`
  helper to tree-LLVM's own preamble (`backend_llvm.rs`) -- same name
  and abort-on-oob shape as `ssa_backend_llvm.rs`'s existing one (the
  two backends never coexist in one emitted module, so sharing the
  symbol name is safe) -- and calling it before the element
  GEP/load/store on all 4 sites: the general-Vec `Index` read, the
  Vec<bool> packed-bit `Index` read, the general-Vec `IndexAssign`
  write, and `RefMutIndex`. `@intent_vec_bool__set_mut` (the one
  write helper backing the Vec<bool> `IndexAssign` path) got the
  check added directly inside its own body rather than at its single
  call site, since it already receives the full Vec struct pointer.
  `RefMutIndex`'s index value also wasn't being widened to i64 via
  `widen_index_to_64` before this fix (a separate pre-existing gap
  the new bounds-check call needed fixed to type-check against); now
  is, matching the other two sites.
  Scope note: this pass only audited and fixed the 3 codegen sites
  directly implicated by the reproducing finding (`Index`,
  `IndexAssign`, `RefMutIndex`, covering both the general-Vec and
  Vec<bool> shapes for each). `backend_llvm.rs` is large and it is
  plausible other Vec-touching call sites (e.g. inside builtin-helper
  codegen for things like `binary_search`, `swap_remove`, etc., that
  are ALSO tree-LLVM-only per `expr_ssa_supported`'s denylist) have
  the same missing-check pattern -- NOT audited here for time reasons;
  worth a dedicated follow-up sweep grepping tree-LLVM's Vec-adjacent
  codegen for GEP-without-a-preceding-`@__intent_bounds_check`-call.
  New tests: 2 `src/lib.rs`
  (`tree_llvm_vec_index_read_write_and_mut_ref_emit_bounds_checks`,
  asserts the helper is defined and called >=4 times for a program
  exercising all 4 sites;
  `tree_llvm_out_of_range_vec_index_aborts_instead_of_reading_garbage`,
  asserts the emitted IR's abort block is reachable) + 1
  `tests/run_end_to_end.rs`
  (`tree_llvm_out_of_range_vec_index_aborts_on_both_backends`, a real
  subprocess `lli`/`cc` run against the minimal struct-forces-tree-
  LLVM repro above, confirming the process now exits non-zero and
  never reaches the print statement after the OOB read -- this is a
  genuine runtime memory-safety bug that only an actual execution
  test catches). All 64 pre-existing Vec-index/bounds-check-related
  `src/lib.rs` tests (SMT elision, `set_mut`, `swap_remove`, SSA
  bounds-check tests, etc.) still pass -- confirmed no regression.
  Full `cargo test --release --workspace` (2749+ tests) clean on both
  BUG-107 and this fix.

## Issue 3B FIXED as BUG-109 (2026-08-04) -- the real echo_pool bug

Continuation of the localfuzz handoff sweep. `docs/LOCALFUZZ_HANDOFF_2026-08-04.md`'s
re-scoped Issue 3 section narrowed the still-open `echo_pool` "LLVM hangs" finding to
"the async fn/Task__handle/__poll_handle machinery" as the likely place to look. Found
it, and it isn't a scheduling/async bug at all.

- **BUG-109 (found by tools/localfuzz, finding
  20260803-050543-run-crash-6bd324cd8f -- effectively verbatim from the shipped
  `examples/language/english/echo_pool.vani`). `Vec<bool>` LITERALS
  (`vec(true, true, …)`) were silently broken on the tree-walking LLVM
  backend, reading back wrong values for any index past the first
  byte -- which looked exactly like an infinite scheduling hang in
  this repro.**
  Symptom: `vanic run examples/language/english/echo_pool.vani` (LLVM,
  default backend) hangs indefinitely; `--backend=c` completes
  correctly (`total bytes received across pool: 9`). No crash, no
  error -- just never terminates, matching the handoff's "Issue 3"
  framing exactly.
  Root cause: `backend_llvm.rs`'s `emit_vec_let_from_literal` (the
  tree-LLVM lowering for `let xs: Vec<T> = vec(a, b, …);`) has no
  special case for `Type::Bool` and falls through to the generic
  per-element path -- allocate `n * vec_element_byte_size(Bool)`
  bytes, bitcast to `i1*`, store each element via plain
  `getelementptr i1, i1* buf, i64 i` / `store i1`. That's a BYTE-
  addressed layout (8 bytes per bool, since `vec_element_byte_size`
  wasn't given a bool-specific case either). Every OTHER
  `intent_vec_bool` operation -- `Index` read, `IndexAssign` write
  (via `@intent_vec_bool__set_mut`), `push` -- uses the PACKED
  layout the type itself declares: `%intent_vec_bool = type { i64*,
  i64, i64 }` (data, len-in-BITS, cap-in-BITS), 64 bools per i64
  word, addressed via `idx/64` (word) + `idx%64` (bit shift/mask).
  The two layouts are totally incompatible: reading a `vec(true,
  true)` literal back through the packed accessor reinterprets the
  literal's raw byte-addressed storage as ONE i64 word. `xs[0]`
  happens to land on the right bit (bit 0 of byte 0) and reads
  correctly; `xs[1]` reads bit 1 of that SAME word -- which is
  whatever bit 1 of byte 0 (the literal's OWN storage for index 0)
  happened to be, not index 1's actual stored value at all. In
  practice: `vec(true, true)` reads back as `[true, false]`, always,
  regardless of what was written to index 1.
  Why this looked like a hang, not a wrong-value bug: `echo_pool.vani`
  uses exactly this shape -- `let alive: Vec<bool> = vec(true, true,
  true);` then a round-robin scheduler `if alive[j] { poll pool[j] }`.
  `alive[1]` and `alive[2]` read as `false` from the moment the
  literal is constructed (not from any later corruption, and not from
  anything to do with `Task__handle`/`__poll_handle`/`io_recv_async`/
  epoll at all -- all of which were fully correct and NOT the bug).
  With `alive[1]`/`alive[2]` wrongly `false`, `pool[1]`/`pool[2]` are
  simply never polled again after their first (skipped) visit --
  their peer connections' data sits ready in the kernel socket buffer
  forever, un-read, while the scheduler loop spins checking `alive[j]`
  (correctly, per its own now-corrupted data) and finding nothing to
  do. `total` never reaches 3 peers' worth of bytes, `done` never
  reaches 3, the loop never exits. Indistinguishable from a genuine
  infinite scheduling loop without bisecting the async machinery out
  entirely, which is what actually found this (see the handoff doc's
  updated Issue 3 section for the bisection trail: ruled out
  `epoll_wait_one`'s timeout truncation and `task{}`'s threading model
  before finding this).
  Only reproduces via tree-LLVM (a struct literal, OR -- as in this
  repro -- an `async fn` call's own `Task__handle` construction,
  which the parser's v3.1 desugar lowers via `StructLit`, forces the
  whole program off the SSA-LLVM fast path per
  `expr_ssa_supported`'s unconditional `StructLit`/`FieldAccess`
  rejection in `main.rs`; SSA-LLVM already had its own, separate,
  correct `Vec<bool>`-literal lowering).
  Fixed by adding `emit_vec_bool_let_from_literal`, a `Vec<bool>`-
  specific literal lowering that builds the correctly-shaped packed
  buffer (`ceil(n/64)` i64 words, zeroed via `memset`) and struct
  (`len`/`cap` in bits) up front, then reuses the already-correct,
  already bounds-checked (BUG-108) `@intent_vec_bool__set_mut` helper
  once per literal element -- rather than re-deriving the packed bit-
  twiddling (word/bit split, shift, mask) inline a second time.
  `emit_vec_let_from_literal` now dispatches to it for `Type::Bool`
  before falling into its generic per-element path.
  New tests: 1 `src/lib.rs`
  (`tree_llvm_vec_bool_literal_packs_elements_correctly`, forces
  tree-LLVM via a struct literal, checks all 5 literal elements of a
  `Vec<bool>` read back correctly and that the emitted IR uses
  `@intent_vec_bool__set_mut` rather than an `i1*` byte-addressed
  buffer) + 1 `tests/run_end_to_end.rs`
  (`echo_pool_example_produces_correct_output_on_both_backends`, a
  real subprocess `lli`/`cc` run against the shipped
  `examples/language/english/echo_pool.vani` example itself, wrapped
  in the `timeout` command per the established BUG-86-regression
  pattern so a future regression fails the test after 30s instead of
  hanging CI forever). All 71 pre-existing Vec<bool>/index/bounds-
  check `src/lib.rs` tests still pass -- confirmed no regression.
  Full `cargo test --release --workspace` (2750 tests) clean.

## Issue 4 FIXED as BUG-110 (2026-08-05) -- systemic gap, most impactful fix of the sweep

Final item from `docs/LOCALFUZZ_HANDOFF_2026-08-04.md`. The original framing ("neither
backend handles this input correctly... possibly related to parallel/sort library
loading") undersold it enormously -- this is not about parallel/sort libs at all (those
get loaded by every `lli` invocation regardless of use, a total red herring), and the
actual bug is not specific to this one repro's `factorial` function. It's a gap in the
runtime-safety-check machinery affecting essentially every ordinary vāṇी program.

- **BUG-110 (found by tools/localfuzz, finding 20260803-033452-run-crash-99db3e1928
  -- `examples/language/odia/keywords.vani`'s factorial, fuzzed from `n - 1` to
  `n - -1`). BOTH SSA backends (the DEFAULT/preferred codegen path for essentially
  every program without a struct literal, field access, or a handful of denylisted
  builtins) silently emit fully UNCHECKED arithmetic -- no overflow guard on Add/Sub/
  Mul, no divide-by-zero guard on Div/Rem, no range guard on Shl/Shr -- regardless of
  what the checker/SMT-elision pass determined was actually needed.**
  Symptom as originally reported: `vanic run <repro> --backend=c` hung indefinitely
  (100% CPU, no progress); the default LLVM backend crashed immediately with an
  `lli`-internal-looking stack dump. Root cause has nothing to do with the specific
  `factorial` shape, parallel/sort libraries, or task/async machinery: `n - -1` (i.e.
  `n + 1`) makes `factorial`'s recursion UNBOUNDED/INCREASING instead of bounded/
  decreasing -- a genuinely broken test program (any language's compiler will
  eventually crash on this), but the OBSERVABLE FAILURE MODE exposed a real, much
  bigger compiler bug: `TypedExprKind::Binary.checked` (set by `check_numeric_binary`/
  `check_integer_remainder` in `checker.rs`, doc'd as guarding "Add/Sub/Mul (integer)
  -> result does not overflow", "Div/Rem -> divisor != 0", "Shl/Shr -> 0 <= rhs <
  bits(lhs)", refined to `false` only by the SMT-discharge pass when provably safe) was
  UNCONDITIONALLY DROPPED when lowering into SSA form -- `ssa.rs`'s `InstrKind::Binary`
  had no field to carry it at all (`TypedExprKind::Binary { op, left, right, .. } =>`
  discarded it via `..`). Both `ssa_backend_c.rs` and `ssa_backend_llvm.rs` therefore
  emit plain `a - b` / `a / b` / `a << b` with zero runtime guard, no matter how
  dangerous the operation. The TREE backends (`backend_c.rs`, `backend_llvm.rs`) always
  respected `checked` correctly -- this was purely an SSA-path regression/gap, and since
  SSA is the FAST/PREFERRED path (`emit_c_via_ssa`/`emit_llvm_via_ssa` in `main.rs` only
  fall back to tree for struct literals, field access, or ~a hundred denylisted
  builtins), this meant MOST ordinary vāṇी programs ran with ZERO overflow/divide-by-
  zero/shift-range protection on EITHER backend -- directly contradicting the language's
  own stated safety guarantee (see `backend_c.rs`'s overflow-helper comment: "Both
  signed and unsigned overflow are trapped... in ASIL-D / DO-178C contexts" -- this is a
  real safety-critical-language guarantee, not a cosmetic nicety).
  Two concrete consequences, both confirmed via disassembly/direct testing, not just
  inferred:
  1. **C backend**: unchecked `int64_t` arithmetic is undefined behavior on signed
     overflow per the C standard. `cc -O2` on the generated code for this repro emitted
     the warning `iteration 9223372036854775802 invokes undefined behavior
     [-Waggressive-loop-optimizations]` and compiled `fn_factorial`'s recursive branch
     into a literal `jmp $` (self-jump) infinite spin loop -- gcc PROVED (correctly, per
     the C standard, given no overflow check exists) that the branch could never
     terminate without hitting UB, and therefore treated it as unreachable, deleting the
     real recursive call/multiply entirely. Confirmed via `objdump -d`: the whole
     `else` branch is `1146: jmp 1146`. This is why the process burned 100% CPU forever
     with a perfectly flat `VmStk` (no real recursion was happening at all -- just an
     infinite no-op spin) instead of ever crashing.
  2. **LLVM backend**: `lli`'s ugly "PLEASE submit a bug report" crash pattern was
     already the case pre-fix and remains post-fix for this SPECIFIC repro (see below)
     -- but for the more common case (e.g. plain division by zero on a non-constant
     divisor, confirmed with a minimal `fn divide(a,b) { return a/b; }` repro), the
     PRE-fix behavior let a raw hardware SIGFPE propagate straight into `lli`'s crash
     handler (which prints the scary LLVM-internal-bug-report message for what is
     actually a completely ordinary, expected-by-the-language trap condition) instead of
     a clean, single `abort()` -- and the C backend crashed with ZERO output at all
     (silent SIGFPE, no message whatsoever) instead of `backend_c.rs`'s intended
     "division by zero" diagnostic.
  Fixed in three files:
  - `ssa.rs`: added `checked: bool` to `InstrKind::Binary`, threaded through every
    construction site (the main `TypedExprKind::Binary` lowering propagates the
    checker's value; compiler-synthesized Binary instructions -- for-loop/for-iter
    counter comparisons and increments, string-comparison-vs-zero -- get `checked:
    false`, matching how the tree backends never guard their own synthesized loop
    counters either) and the Display impl. `ssa_pass.rs`'s constant-folding match arms
    updated to ignore the new field (folding a compile-time-constant pair is safe
    regardless of the flag -- `checked_add`/`checked_sub`/etc. already return `None`,
    leaving the original checked instruction in place, whenever a fold WOULD overflow).
  - `ssa_backend_c.rs`: `InstrKind::Binary`'s emission now mirrors `backend_c.rs`'s tree
    logic inline (no shared preamble helper needed -- SSA operands are already-
    evaluated `v_N` reads or constants, safe to reference twice, matching this file's
    own existing `InstrKind::Index` bounds-check convention): Add/Sub/Mul emit a
    `__builtin_{add,sub,mul}_overflow` guard + `fprintf`+`abort()` on overflow; Div/Rem
    emit a `== 0` guard before the division; Shl/Shr emit a range guard (`x < 0 ||
    (uint64_t)x >= bits` for signed, `(uint64_t)x >= bits` for unsigned) before the
    shift.
  - `ssa_backend_llvm.rs`: added a pre-scan (mirroring the existing Vec-element-
    collection pass in `emit()`) that walks every function's instructions for
    `checked: true` Binary ops, collecting exactly the `(type, op)` combinations
    actually used, and emits ONLY those as `alwaysinline` helper functions in the
    preamble (`@__intent_checked_{add,sub,mul}_{ty}` via `llvm.{s,u}{add,sub,mul}.
    with.overflow.<width>` + abort-on-overflow, matching `@__intent_bounds_check`'s
    existing shape from BUG-108; `@__intent_checked_divisor_{ty}` /
    `@__intent_checked_shift_{ty}` as simple validate-and-return-the-operand helpers).
    `emit_binary` calls these via plain `call` instructions (no new basic blocks at the
    call site -- deliberately NOT inline branching, since that's a proven-risky pattern
    for this file's block-sequential per-instruction emission; the existing
    `@__intent_bounds_check` precedent already established call-a-shared-helper as the
    safe approach here).
  Net effect on the actual repro: BOTH backends now genuinely recurse (the overflow
  check is a real, must-respect side effect gcc can no longer optimize away) and both
  now fail FAST and HONESTLY -- confirmed via direct `cc`-compiled-binary execution: a
  clean `Segmentation fault` (exit 139, genuine native stack overflow) on C instead of
  an eternal spin; `lli` still crashes with its characteristic noisy "PLEASE submit a
  bug report" pattern on LLVM (same as before -- this is `lli`'s own interpreter
  genuinely exhausting its stack under deep real recursion, an expected, not-further-
  fixable consequence of the input program's actual infinite/unbounded recursion, not a
  vāṇी-compiler bug; also matches how BUG-108's bounds-check aborts already look under
  `lli`, an established, accepted characteristic of this tool throughout this sweep).
  The original "C hangs, LLVM crashes" ASYMMETRY is resolved -- both now terminate
  quickly.
  A critical, sobering discovery made while root-causing this: `compile_to_c`/
  `compile_to_llvm` (the helpers essentially the ENTIRE existing overflow/divisor/shift
  test suite in `src/lib.rs` uses -- 72 pre-existing passing tests, checked during this
  fix) call `backend_c::CBackend.emit`/`backend_llvm::LlvmBackend.emit` DIRECTLY,
  completely bypassing `emit_c_via_ssa`/`emit_llvm_via_ssa` (what `vanic run` actually
  uses). Every one of those 72 tests was exercising ONLY the tree backends, which were
  never broken -- meaning the extensive-looking existing coverage provided ZERO
  protection against this bug in the path real users actually hit by default. New
  tests for this fix deliberately go through `lower_program` + `ssa_backend_c::emit` /
  `ssa_backend_llvm::emit` directly (or a subprocess `vanic run` with no `--backend`
  flag and no struct literal in the source) specifically BECAUSE of this gap.
  New tests: 2 `src/lib.rs`/module-inline unit tests
  (`ssa_backend_c::tests::checked_binary_emits_runtime_guards_on_ssa_c`,
  `ssa_backend_llvm::tests::checked_binary_emits_runtime_guards_on_ssa_llvm`, each
  compiling a small struct-literal-free program exercising all three guard categories
  and asserting the checked-helper names appear in the emitted C/LLVM IR) + 2
  `tests/run_end_to_end.rs` subprocess tests
  (`odia_factorial_unbounded_recursion_fails_fast_on_both_backends`, running the actual
  fuzzed repro shape wrapped in `timeout 15` on both backends and asserting neither
  hangs -- status code != 124 -- nor succeeds; and
  `odia_keywords_example_produces_correct_output_on_both_backends`, the REAL unmodified
  shipped example with correct/bounded `n - 1` recursion, confirming the new guards
  produce ZERO false positives on ordinary non-overflowing arithmetic). All 72
  pre-existing tree-backend overflow/divisor/shift/SMT-elision `src/lib.rs` tests still
  pass unchanged -- confirmed no regression (expected: this fix never touches
  `backend_c.rs`/`backend_llvm.rs`, only the SSA path). Full `cargo test --release
  --workspace` (2752 lib tests + 184 end-to-end subprocess tests) clean.
  Follow-up worth flagging for a future session (not done here, out of scope for a
  single-repro-driven fix): audit whether `ssa_pass.rs`'s SMT-elision-adjacent passes
  (or any future SSA optimization pass) could ever flip `checked` from `true` to
  `false` on a `Binary` instruction incorrectly -- this fix only makes the SSA backends
  RESPECT the flag they're given, it does not re-verify that the flag's VALUE (as
  already computed by the checker before SSA lowering ever sees it) is always sound.

## BUG-111 FIXED (2026-08-05) -- SSA-LLVM invalid IR for int-literal-to-float `let`

From `docs/LOCALFUZZ_HANDOFF_2026-08-05.md` (found via 4 localfuzz findings -- Korean,
Tibetan x2, plus hand-written minimal repros -- originally mis-clustered by the
digest's keyword matcher as a possible BUG-76 match purely because both mention
"integer constant must have integer type" for unrelated reasons).

- **BUG-111: `let x: f64 = <integer literal>;` (and the `f32` equivalent) made
  SSA-LLVM emit LLVM IR that `lli` rejects outright at parse time, before the program
  ever runs.** Minimal repro:
  ```vani
  fn main() -> i64 {
    let n: f64 = 0;
    print n;
    return 0;
  }
  ```
  `vanic run <repro>` (default LLVM/SSA path) failed with:
  ```
  lli: .../vanic-....ll:68:27: error: integer constant must have integer type
    %v_0 = fadd double 0.0, 0
  ```
  `vanic run <repro> --backend=c` worked fine (prints `0`), and adding any struct
  literal (forcing the program onto tree-LLVM instead of SSA-LLVM, same
  `expr_ssa_supported` mechanism as BUG-108/109/110) also worked fine -- confirming
  this was SSA-LLVM-specific, not a general LLVM-backend bug. `let x: f64 = 7;` failed
  identically (`fadd double 0.0, 7`), ruling out anything special about the literal
  `0`. Blast radius: `let total: f64 = 0;`-style accumulator initialization is an
  extremely common pattern, not a fuzzer edge case -- this affected a meaningful
  fraction of real f64-using programs on the default backend.
  Root cause: the checker desugars an implicit int-literal-to-float coercion (e.g. the
  RHS of `let n: f64 = 0;`) into a `TypedExprKind::Cast` node wrapping the
  still-integer-typed literal (`coerce_checked` -> `cast_expr` in `checker.rs`) rather
  than reinterpreting the literal in place -- correct and necessary, since the inner
  literal's own `.ty` (`I64`) needs to stay intact for the cast to mean anything.
  `ssa.rs`'s `lower_expr_to_operand` lowers that `Cast`'s inner literal via
  `lower_expr_to_operand`, which for a bare `TypedExprKind::Int` produces
  `Operand::Const(Const::Int(0))` -- a plain constant with NO `ValueId`, carrying no
  type annotation of its own at the SSA level. `ssa_backend_llvm.rs`'s `emit_instr` for
  `InstrKind::Cast` then tried to recover the cast's *source* type via
  `operand_type(x, value_types)` -- but that helper only knows how to look up a type
  for `Operand::Value(v)` (via the `value_types` map); for `Operand::Const(_)` it
  unconditionally returned `None`. The old fallback on that `None`,
  `.unwrap_or_else(|| to.clone())`, defaulted the cast's SOURCE type to its own TARGET
  type (`F64`) -- turning a genuine int-to-float cast into what looked, at the LLVM
  emission level, like a same-type "identity" cast. `emit_cast` took its
  `from_llvm == to_llvm` branch (both sides "F64"), which emits the identity-op
  pattern `fadd <T> 0.0, <operand text>` -- but `operand_str`/`const_str` render
  `Const::Int(0)` as the plain integer text `"0"` (correctly so in every OTHER
  context, e.g. a real `i64`-typed identity op), producing the invalid
  `fadd double 0.0, 0` instead of a real `sitofp i64 0 to double`. The C backend never
  hit this: `InstrKind::Cast` there just emits a C-style `(double)(0)` cast expression,
  which is valid C regardless of any int/float source-type bookkeeping, so this gap
  was invisible on that path -- exactly the "one codegen path tracks types precisely
  enough, the other doesn't" shape this project has repeatedly hit (see BUG-107 through
  BUG-110).
  Fixed in `ssa_backend_llvm.rs`: added `const_operand_natural_type(op: &Operand) ->
  Type`, which derives a `Const` operand's own true source type directly from its
  variant (`Const::Int(v)` -> `I64` unless `v` overflows `i64`'s range, then `U64`,
  matching `check_expr`'s own int-literal-widening rule in `checker.rs`;
  `Const::Float(_)` -> `F64`; `Const::Bool(_)` -> `Bool`) rather than guessing the
  cast's target type. The `InstrKind::Cast` handler's fallback now calls this instead
  of `to.clone()`: `operand_type(x, value_types).unwrap_or_else(|| const_operand_natural_type(x))`.
  A genuine int-to-float `Cast` on a bare constant now correctly resolves
  `from_llvm != to_llvm` and takes `emit_cast`'s real `sitofp`/`uitofp` path.
  Verified via the emitted IR directly (`sitofp i64 0 to double`, `sitofp i64 7 to
  float` for the `f64`/`f32` cases respectively) and via `vanic run` on both backends
  producing identical, correct output.
  New tests: 1 `src/lib.rs` unit test
  (`ssa_llvm_int_literal_to_float_let_emits_sitofp`, going through `lower_program` +
  `ssa_backend_llvm::emit` directly -- NOT `compile_to_llvm`, which calls the tree
  backend and wouldn't exercise this path at all, the same coverage gap BUG-110's
  writeup documents -- asserting the emitted IR contains `sitofp` and does NOT contain
  the invalid integer-spelled `fadd` identity op) + 1 `tests/run_end_to_end.rs`
  subprocess test (`int_literal_to_float_let_produces_correct_output_on_both_backends`,
  a real `vanic run` on both backends asserting matching correct output -- necessary
  because the bug is that `lli` refuses to even PARSE the generated IR, a failure mode
  a `compile_to_llvm` string-content assertion wouldn't catch on its own). Full `cargo
  test --release --workspace` (2753 lib tests + 185 end-to-end subprocess tests) clean,
  zero regressions.

## BUG-112 FIXED (2026-08-05) -- `vanic build` missing `-lm` on host POSIX link

Found while re-verifying BUG-31's LLVM-backend counterpart (see that entry's 2026-08-05
update above) -- not a localfuzz finding, discovered manually during that
investigation.

- **`vanic build` (LLVM AOT native-binary compile, `src/main.rs`'s `build`
  subcommand) omitted `-lm` from its host-POSIX link command, so ANY program could
  fail to link with `undefined reference to 'exp'` (or `erf`, `fmod`, ...) depending
  on the host's default `cc`/linker behavior.** Every vāṇी program's runtime
  unconditionally emits math-builtin helper functions (`intent_f64_normal_pdf`,
  `intent_f64_normal_cdf`, `intent_f64_wrap`, etc.) that reference libm symbols,
  regardless of whether the program actually calls any of them -- so this wasn't
  limited to math-heavy programs; it could hit literally any `vanic build` invocation
  on a host where the default `cc` link step doesn't already implicitly pull in libm.
  Root cause: `src/main.rs`'s LLVM-build link-command construction has three branches
  by target -- `bare_metal` (correctly no `-lm`, no libc at all), `is_cross`
  (correctly adds `-lm`), and the host-POSIX `else` branch, which only added
  `-lpthread` and never `-lm`. The near-identical host-C-backend link command a few
  hundred lines earlier in the same file (`src/main.rs`, "Closure #299" comment)
  already added `-lm` correctly for the exact same reason -- this was specifically a
  gap in the LLVM-build path's own, separate link-command construction, not a
  copy-paste of the already-correct C-backend one. `vanic run` (LLVM via `lli`) was
  never affected, because `lli` auto-resolves libc/libm symbols itself at JIT time
  (see this file's own "LLVM-JIT via lli auto-resolves" comment) -- only the AOT
  `vanic build` path, which shells out to `cc` for linking, needed the explicit flag.
  This stayed hidden because `vanic run` is the far more common/tested path, and
  because whether it manifests depends on the host's own `cc` defaults (some distros'
  gcc specs implicitly pull in libm even without `-lm`; this bug reproduced concretely
  on this project's own dev machine).
  Fixed by adding `link_cmd.arg("-lm");` to the host-POSIX branch, mirroring the
  `is_cross` branch immediately above it and the C-backend's own host link command.
  New test: `vanic_build_links_self_referential_struct_vec_example_without_manual_lm_flag`
  in `tests/run_end_to_end.rs` -- a real `vanic build` + execute-the-linked-native-
  binary subprocess test (not a string check, since the bug is specifically a linker
  failure that `emit`/`compile_to_llvm` assertions wouldn't exercise), reusing the
  self-referential-struct-Vec example (BUG-31's repro) since that's exactly the
  program that surfaced this while re-verifying that bug's LLVM side. Full `cargo test
  --release --workspace` clean, zero regressions.

## BUG-113/114/115/116 (2026-08-05) -- `requires`-clause bug-pattern audit sweep

Found while drafting `docs/BUG_PATTERN_AUDIT_TODO.md` (a new bug hunt organized by
root-cause *pattern* rather than feature-pair enumeration, since the two prior sweeps
exhausted the obvious combinations) and root-causing its top-priority item: whether
`requires`/`ensures` clauses have any real runtime enforcement. What started as one
confirmed bug (BUG-113) uncovered three more in the same investigation, escalating in
severity -- BUG-116 in particular is a genuine silent safety hole, not a cosmetic
crash-message issue.

- **BUG-113: a `requires`-clause violation on the tree-LLVM backend used a raw `call
  void @abort()`, making `lli` print its "PLEASE submit a bug report" internal-crash
  stack dump for a clean, expected precondition failure.** Repro: `fn safe_sqrt(x:
  f64) -> f64 requires sqrt(x) < 1000000.0; { return sqrt(x); }` called with `1.0e30`
  -- `sqrt` is on `expr_ssa_supported`'s denylist, forcing tree-LLVM. `--backend=c`
  already gave a clean assertion message (glibc `assert()` + SIGABRT via a real
  compiled binary isn't misleading the way `lli`'s JIT crash handler is); only LLVM
  crashed ugly. This is the exact class BUG-106 fixed for plain `assert` statements
  -- BUG-106's own writeup explicitly left the `requires`-clause `abort()` call sites
  "deliberately untouched" as "not a divergence" (both backends called `abort()`
  consistently there), but consistency doesn't make `lli`'s misleading crash report
  correct. Fixed by switching `backend_llvm.rs`'s `requires`-clause codegen (in
  `emit_function`, right after parameter stores) from `call void @abort()` to `call
  void @exit(i32 3)`, matching the assert fix. New test: updated the pre-existing
  `lli_aborts_on_violated_requires` (`src/backend_llvm.rs`), which had asserted the
  OLD abort-signal behavior (`exit == Some(134)` etc.) -- now asserts `exit ==
  Some(3)`.

- **BUG-114: `ssa_backend_c.rs`'s `c_const` used Rust's Display formatting (`{}`) for
  `Const::Float`, silently corrupting any f64 literal beyond ~1.8e19 in magnitude on
  the C backend.** Found by hand while verifying BUG-113's repro also worked on
  `--backend=c` -- `1.0e30` printed as `5.07694e+18` instead of `1e+30`, with no
  compiler error. Root cause: Display formatting for f64 omits BOTH the decimal point
  and any exponent notation for large whole-number magnitudes (`1.0e30` -> the
  33-digit string `"1000000000000000000000000000000"`, no `.`, no `e`) -- unlike
  Debug formatting (`{:?}`), which switches to `1e30`-style notation. C's lexer parses
  a bare digit sequence with neither as an unsuffixed INTEGER constant; once that
  exceeds `unsigned long long`'s range (~1.8e19), gcc/clang warn "integer constant is
  too large for its type" and silently truncate/wrap it before the implicit
  int-to-double conversion. Values below that threshold "accidentally" round-trip
  correctly (the digit string still parses as a valid, in-range integer constant that
  converts losslessly to the same double). `backend_c.rs`'s tree-emitter's
  `emit_float_literal` already correctly used `{:?}` -- this was specifically an
  `ssa_backend_c.rs` gap, invisible on any test using `compile_to_c` (bypasses SSA
  entirely, same coverage gap BUG-110's writeup documents) or any float literal below
  the threshold (essentially every prior test in the suite). Fixed by switching
  `c_const`'s `Const::Float` arm to `{:?}`. New tests: `src/lib.rs`
  (`ssa_backend_c::tests::large_float_literal_emits_c_float_syntax_not_bare_integer_digits`)
  + `tests/run_end_to_end.rs`
  (`large_float_literal_produces_correct_output_on_both_backends`).

- **BUG-115: the exact same misleading-`lli`-crash-report class as BUG-113 also
  affected the Vec bounds-check helper (both tree-LLVM and SSA-LLVM) and all three
  SSA-LLVM checked-arithmetic guards (overflow/divisor/shift, added earlier the same
  day by BUG-108/110) -- five more `call void @abort()` sites.** Confirmed
  independently of BUG-108/110's own repros (which involved genuine stack overflow,
  where their writeups' "an accepted, not-further-fixable characteristic of `lli`"
  framing legitimately applies) via a minimal, non-recursive out-of-bounds index
  (`v[10]` on a 3-element `Vec<i64>`, no recursion at all) -- still produced the ugly
  crash report pre-fix. All five sites are a simple, controlled single-branch trap
  (`br i1 %ok/%bad, label %cont, label %oob`), structurally identical to `assert`'s own
  guard shape, so the same fix applies directly. Fixed by switching all five
  `oob:`-block `call void @abort()` sites to `call void @exit(i32 3)`:
  `backend_llvm.rs`'s `@__intent_bounds_check` definition, and `ssa_backend_llvm.rs`'s
  `@__intent_bounds_check` plus the `checked_overflow_ops`/`checked_divisor_tys`/
  `checked_shift_tys` guard-emission loops. New tests: updated the pre-existing
  `tests::tree_llvm_out_of_range_vec_index_aborts_instead_of_reading_garbage`
  (`src/lib.rs`, hard-coded the old `abort()` IR text) + new
  `tests/run_end_to_end.rs` test
  `runtime_traps_exit_cleanly_instead_of_crashing_lli_on_llvm_backend`, which drives
  all five trap categories through a real subprocess `vanic run` and asserts both
  `exit(3)` and the absence of "PLEASE submit a bug report" in stderr.

- **BUG-116 (the most severe of the four): `requires` clauses were NEVER lowered
  into SSA at all -- `ssa.rs` never read `function.requires` anywhere -- and the
  checker uses an unprovable-at-call-site `requires` clause as a licensed ASSUMPTION
  to elide runtime `checked` guards on operations inside the function body that the
  precondition makes provably safe. Combined, this meant a violated precondition on
  the (default, most common) SSA path produced a completely unguarded raw operation
  -- a genuine hardware trap (SIGFPE for the repro below), not even the misleading-
  but-at-least-present `abort()` BUG-113/115 were about.** Repro: `fn safe_div(a: i64,
  b: i64) -> i64 requires b > 0; { return a / b; }`, called as `safe_div(10, id(0))`
  (the `id()` indirection hides the literal `0` from the SMT verifier, so the checker
  can't prove-or-disprove the precondition at the call site and silently accepts the
  program per `verify_call_args_in_expr`'s own documented "Unknown ... stay silent,
  the runtime `requires` check still fires" policy -- except on this path, nothing
  ever fires). Confirmed via direct SSA-LLVM IR inspection (not just black-box
  behavior): the emitted `fn_safe_div` was `%v_2 = sdiv i64 %v_0, %v_1 \n ret i64
  %v_2` -- a bare `sdiv`, zero guard calls anywhere in the module, because the
  checker's own SMT reasoning (correctly, GIVEN the precondition) proved `a / b`
  safe WITHIN `safe_div`'s body and set that Div's `checked` flag to `false`, and
  nothing anywhere enforces the precondition that safety argument depends on. This
  is a real violation of the language's own stated safety guarantee (`backend_c.rs`'s
  overflow-helper comment: "trapped... in ASIL-D / DO-178C contexts"), on the
  DEFAULT/preferred code path, for a completely ordinary defensive-programming
  pattern (guard a function with a `requires` clause instead of an in-body `assert`).
  Fixed in two places:
  - `ssa.rs`'s `lower_function`: lowers each `f.requires` clause into the same guard
    shape `TypedStmt::Assert` already uses (evaluate the condition, branch, on
    failure call `intent_assert_fail` with a synthesized message
    `"precondition violated in '<fn>'"` / `"... (requires #N)"` for 2+ clauses, then
    `Terminator::Unreachable`), emitted in the function's entry block right after
    parameter bindings and before the body -- so it reuses the exact same runtime
    helper (and therefore the exact same `exit(3)`-on-LLVM /
    `assert()`-macro-on-C semantics, per BUG-106) as an ordinary failed `assert`.
  - `main.rs`'s `ssa_path_supports`: now also requires every `f.requires` expression
    itself pass `expr_ssa_supported`, mirroring the existing body/param/return-type
    checks. Necessary because the new lowering reuses the body's own expression
    lowerer, which (correctly) has no idea how to handle a construct
    `expr_ssa_supported` would already reject in the body (e.g. a denylisted builtin
    like `sqrt`) -- without this gate, a `requires` clause using such a construct
    produced invalid IR (confirmed: a bare `sqrt(x) < N` clause with no `sqrt` call
    in the body previously compiled to a reference to a nonexistent `@fn_sqrt`,
    since nothing had ever exercised this combination before this fix started
    lowering `requires` at all). With the gate, such a function correctly falls back
    to the tree backend instead, which has always handled arbitrary `requires`
    expressions.
  New tests: `src/ssa.rs`
  (`tests::requires_clause_lowers_to_a_runtime_guard_at_function_entry`, lowers the
  `safe_div`/`id` repro directly via `lower_function` and asserts a `Branch`
  terminator + an `intent_assert_fail` call exist) + `tests/run_end_to_end.rs`
  (`requires_clause_violation_traps_and_valid_call_still_works_on_both_backends`,
  real subprocess runs confirming BOTH that a valid call still succeeds -- no false
  positives from the new guard -- and a violating call traps with `exit(3)` on both
  backends). Manually re-verified the `sqrt`-in-requires-not-in-body fallback case
  (no dedicated automated test; low risk, covered structurally by
  `expr_ssa_supported`'s existing extensive test coverage) produces correct output on
  both backends post-fix.

Follow-up not done here (logged in `docs/BUG_PATTERN_AUDIT_TODO.md` category A/B for
a future session): `ensures` clauses have no `TypedFunction` field at all (purely a
checker.rs/SMT-time concept, per BUG-68) and so have zero runtime backstop on ANY
backend -- unlike `requires`, there's no obvious single injection point (would need
to intercept every `return` site and substitute the return value), and no confirmed
live repro yet showing an actual wrong-answer consequence (only the theoretical gap).
Also: the `#[bounded(N)]` recursion-depth guard (`backend_c.rs`/`backend_llvm.rs`)
still uses raw `abort()` on the LLVM side, same BUG-115 class, left untouched --
out of the agreed scope for this pass (the 5 sites fixed were the ones flagged
before starting; this one was noticed afterward via a broader grep and deliberately
not opportunistically folded in).

Full `cargo test --release --workspace` (2755 lib tests + 189 end-to-end subprocess
tests) clean, zero regressions, across all four fixes.

## BUG-117 (2026-08-05) -- `#[bounded(N)]` recursion-depth guard also used raw abort()

Found via a broader grep for `call void @abort()` after BUG-115 landed (noted as a
follow-up in `docs/BUG_PATTERN_AUDIT_TODO.md` category B, deliberately not folded into
that pass; picked up separately as a low-risk, low-token-cost fix). Same class exactly:
both `backend_llvm.rs` (tree, `emit_function`'s bounded-fn entry sequence) and
`ssa_backend_llvm.rs` (SSA, its own bounded-fn entry sequence) branched to a
`__bd_abort` block that called raw `abort()` on a `#[bounded(N)]` violation, giving
`lli` its misleading "PLEASE submit a bug report" crash report for a clean, expected
trap. Fixed by switching both to `exit(3)`. Updated the pre-existing
`bounded_attribute_emits_depth_counter_on_llvm_backend` (`src/lib.rs`, hard-coded the
old `abort()` IR text) + new `tests/run_end_to_end.rs` test
`bounded_attribute_violation_exits_cleanly_instead_of_crashing_lli`. Full `cargo test
--release --workspace` clean.

## BUG-118 (2026-08-06) -- C backend's own `atoll` FFI example failed to compile via `cc`

Found by the `vani-localfuzz-ollama` harness's nightly digest as a `backend-divergence`
cluster (C backend fails, LLVM backend succeeds). Root-caused against the fuzzer's
`repro.vani`, which turned out to be an (essentially) unmutated copy of the repo's own
`examples/language/english/ffi.vani` -- i.e. this was already reproducible on plain
`main`, not a fuzzer-only artifact:

```
$ vanic run examples/language/english/ffi.vani --backend=c
cc failed while compiling ...c:149:16: error: conflicting types for 'atoll'; have
'int64_t(const char *)' {aka 'long int(const char *)'}
...
/usr/include/stdlib.h:493:1: note: previous definition of 'atoll' with type
'long long int(const char *)'
```

Root cause: both C backends (`backend_c.rs`'s `emit_prototype`/`emit_function` and
`ssa_backend_c.rs`'s `emit_function_prototype`) spell `i64` as `int64_t` when emitting
an `extern "C" fn`'s forward declaration. On this LP64 host `int64_t` is `long`, but
the C standard mandates `atoll` (and `strtoll`/`strtoull`/`llabs`/`lldiv`) return/take
`long long` -- same width, nominally different type, and `cc` treats redeclaring a
already-visible libc symbol with a different nominal type as a hard conflict even
though both backends' generated preamble already unconditionally `#include`s
`<stdlib.h>` (which declares `atoll` correctly on its own).

Fix: added `backend_c::is_known_libc_symbol`, a narrow allowlist covering exactly the
five C99-mandated `long long` libc symbols (`atoll`, `strtoll`, `strtoull`, `llabs`,
`lldiv`). When an `extern "C" fn`'s name matches, both backends now skip emitting
their own competing prototype and trust the declaration `<stdlib.h>` already provides;
call sites are unaffected (they already called by bare name). Deliberately did *not*
widen this to "every stdlib/string/stdio symbol" -- e.g. `atoi`'s `int` return matches
our `int32_t` mapping exactly, so suppressing its prototype too would have been an
unnecessary behavior change (and broke the pre-existing
`extern_c_fn_emits_bare_c_prototype_and_call` test, which asserts `atoi`'s prototype
*is* emitted). New test `extern_c_atoll_does_not_conflict_with_libc_prototype`
(`src/lib.rs`) covers both the tree-C and SSA-C backends. Full `cargo test --release`
(2756 lib tests + 190 end-to-end subprocess tests) clean, zero regressions.

## BUG-119 (2026-08-06) -- signed `MIN / -1` (and `MIN % -1`) had no runtime guard at all

Picked up from `docs/BUG_PATTERN_AUDIT_TODO.md` category B's own suggested next repro
("does `requires b != 0` correctly cover `a % b`'s *other* overflow case?"). The
`requires`-based repro (`requires b != 0` on `fn safe_rem(a: i64, b: i64)`, called with
`(i64::MIN, -1)`) crashed `vanic run` (default LLVM) with `lli`'s misleading "PLEASE
submit a bug report" banner -- a genuine hardware SIGFPE, not a clean vani trap.
Stripping the `requires` clause entirely still crashed the same way, proving this
isn't a `requires`-elision gap at all (unlike BUG-116): **no backend had ANY runtime
guard for this case, requires clause or not.** Both `--backend=c` and default LLVM hit
it for both `/` and `%`; the C backend happened not to crash (gcc's own codegen
sidesteps the hardware trap here) but LLVM's raw `sdiv`/`srem` reached actual hardware
overflow.

Root cause: all four backends' Div/Rem "checked" guard only ever validated the
divisor (`b == 0`) -- `intent_check_i64_divisor` (tree-C), the inline `(r) == 0` check
(SSA-C), and `@__intent_checked_divisor_i64` (SSA-LLVM) all take just the RHS operand,
so none of them could ever see the `a == i64::MIN` half of the `MIN / -1` overflow
condition; tree-LLVM's inline guard had the identical one-operand shape. Only signed
types have this case at all (no negative divisor for unsigned).

Fix: for signed integer Div/Rem specifically, replaced the divisor-only guard with a
combined helper/inline-check that validates BOTH operands and performs the operation
itself (same "return the checked result" shape the pre-existing Add/Sub/Mul overflow
helpers already use) -- `intent_checked_{ty}_div`/`_rem` (tree-C, new preamble
helpers), an added inline `(r) == -1 && (l) == {TY}_MIN` check after the existing
zero-check (SSA-C), a second `icmp`/`br`/`abort()` guard block after the existing
divisor-zero block (tree-LLVM, matching that function's existing raw-`abort()`
convention), and `@__intent_checked_div_{ty}`/`@__intent_checked_rem_{ty}` -- new
preamble helpers taking both operands, `exit(3)` on failure (matching SSA-LLVM's
existing clean-trap convention) -- replacing `@__intent_checked_divisor_{ty}` for
signed types only (unsigned Div/Rem keeps the old divisor-only helper unchanged, since
it's still correct there). Updated three pre-existing tests whose assertions named the
old divisor-only helper for a signed `i64` case (`divisor_check_remains_when_safety_is_
not_provable` in `src/lib.rs`, `checked_binary_emits_runtime_guards_on_ssa_llvm` in
`src/ssa_backend_llvm.rs`) to expect the new combined-helper name instead -- the
elision-based tests (`smt_elides_divisor_check_when_requires_proves_nonzero` and the
`safe_at` bounds+divisor test) needed no change, since `requires b > 0` already rules
out `b == -1` and the elision stays sound. New tests: `checked_signed_div_and_rem_
guard_against_min_by_neg_one_overflow` + `unsigned_div_still_uses_divisor_only_check_
not_combined_helper` (`src/lib.rs`, covering tree-C and SSA-C), `lli_aborts_on_signed_
div_min_by_neg_one_overflow` + `_rem_` variant (`src/backend_llvm.rs`, tree-LLVM via
`lli`). Full `cargo test --release` (2762 lib tests + 190 end-to-end subprocess tests)
clean, zero regressions.

Note: message text still differs cosmetically between tree-C ("i64") and SSA-C
("int64_t") for this new check, same as the pre-existing Add/Sub/Mul overflow
messages -- a known, already-accepted inconsistency (`docs/BUG_PATTERN_AUDIT_TODO.md`
category D), not something this fix changed or needs to fix.

## BUG-120 (2026-08-06) -- tree-LLVM's checked-arithmetic + Vec-mutator guards still used raw `abort()`

Found while building category D's ("trap exit-code/message consistency matrix") test
matrix -- specifically while forcing the tree-LLVM path (via an unused `sqrt()` call,
which the module-wide SSA-eligibility gate treats as disqualifying the whole program)
to test BUG-119's new MIN/-1 guard on that path. A plain signed-add overflow crashed
`lli` with its misleading "PLEASE submit a bug report" banner on the tree path even
though the identical program on the (default) SSA path exits cleanly with `exit(3)`.

Root cause: BUG-115's writeup is explicit that it fixed "the Vec bounds-check helper
(both tree-LLVM and SSA-LLVM) and all three SSA-LLVM checked-arithmetic guards" --
i.e. it never touched tree-LLVM's OWN checked-arithmetic guards (Add/Sub/Mul overflow,
Div/Rem zero-check, Shl/Shr range) at all, only SSA-LLVM's. These guards are inlined
directly per-call-site in `emit_binary` (not extracted into a named `alwaysinline`
helper function like `@__intent_bounds_check` is), which is likely why BUG-115's grep-
for-helper-functions sweep missed them. Separately, four Vec-mutator-builtin guards
(`pop_mut` generic, `Vec<bool>`'s dedicated packed `pop_mut`, `swap_remove`, `insert`)
had the same raw-`abort()` shape and no SSA-LLVM counterpart to catch the gap by
comparison, since `pop`/`swap_remove`/`insert` are all SSA-denylisted (tree-only).
My own BUG-119 fix earlier today added a new MIN/-1 guard to this same tree-LLVM
checked-arithmetic block and matched its (buggy) local `abort()` convention at the
time -- fixed here along with the rest.

Fix: switched all 9 `call void @abort()` sites in `backend_llvm.rs`'s checked-
arithmetic block (unsigned-Sub early-return, the shared Add/Mul/signed-Sub/Div/Rem
`fail` block, the MIN/-1 overflow block, Shl/Shr range) and the four Vec-mutator sites
to `call void @exit(i32 3)`, matching BUG-115's fix for the sibling helpers. Explicitly
did NOT touch: `match`'s no-wildcard exhaustiveness-fallback `abort()` (provably
unreachable given checker-enforced exhaustiveness -- a defensive compiler-bug catchall,
not a user-triggerable trap) or `unsafe_alloc`/`unsafe_free`'s heap-canary-corruption
`abort()`s (a different, deliberately-harder safety tier per `unsafe.md`, not one of
category D's listed trap types). Updated 3 pre-existing tests whose assertions expected
a signal (`lli_aborts_on_div_by_zero`, and my own `lli_aborts_on_signed_div_min_by_neg_
one_overflow` / `_rem_` variant from BUG-119 earlier today) to expect `Some(3)` instead.
New tests (`src/backend_llvm.rs`): `lli_exits_cleanly_on_signed_add_overflow_tree_path`,
`_unsigned_sub_overflow_tree_path`, `_shift_range_violation_tree_path`, `_pop_from_
empty_vec`, `_pop_from_empty_vec_bool`, `_swap_remove_out_of_bounds`, `_insert_out_of_
bounds`. Full `cargo test --release` clean, zero regressions.

Category D status: this closes the "does it trap at all + does it look the same"
question for the Add/Sub/Mul/Div/Rem/Shl/Shr and Vec-mutator cells specifically (now:
yes traps, exit(3) uniformly on LLVM, no signal-based crash-report divergence). The
tree-C-vs-SSA-C *message text* wording difference noted in BUG-119's entry above
remains open as a known, lower-severity, accepted inconsistency -- not addressed here.
`requires`/`ensures` and `#[bounded(N)]` cells were already covered by BUG-113/116/117.
Explicit `assert` was BUG-106's own template. Category D's matrix is now fully audited.

## BUG-121 (2026-08-06) -- `HashMap<K, bool>` produced invalid LLVM IR (`Option<bool>` payload)

Found while auditing category E's packed/special-layout candidates: a function that
directly `return`s a `vec(true, false, …)` literal, an `Array<bool, N>` literal, a
struct field of `Vec<bool>`, and a nested `Vec<Vec<bool>>` all round-tripped correctly
on both backends (added as regression tests, since the audit doc's own method says a
clean pass still closes a real coverage gap). `HashMap<K, bool>` was the one candidate
that broke -- not with BUG-109's runtime-corruption shape, but a **hard compile-time
LLVM IR verification failure**: `lli` rejected the module outright with
`insertvalue operand and field disagree in type: 'i8' instead of 'i1'` for any program
calling `hashmap_get`/`hashmap_insert`/`hashmap_remove` on a `HashMap<K, bool>`.

Root cause: `HashMap<K, bool>`'s internal value storage uses `i8` per slot (no
bit-packing for HashMap values -- a legitimate, different storage choice from
`Vec<bool>`'s packed layout, unrelated to BUG-109). But the generic enum emitter
declares `%Enum_Option__bool`'s payload field as `i1` (the true LLVM type for
`Type::Bool` everywhere else `Option<bool>` is constructed). `hashmap_get`'s codegen
loaded the real `i8`-typed value from storage and `insertvalue`d it straight into the
`i1`-typed struct field -- invalid for a named SSA register (unlike a bare `0`/`1`
literal token, which happened to work by accident: an untyped literal coerces to
whatever type context it's embedded in, so `hashmap_insert`'s call site with a literal
`true`/`false` argument never tripped this, masking the bug until a value was actually
*read back* through `get`/`insert`'s-previous-value/`remove`).

This root cause is replicated across **six** near-duplicate key-type-generic HashMap
codegen functions in `backend_llvm.rs` (`emit_intent_hashmap_pair_llvm` [i64 K],
`_f64k`, `_strk`, `_vec_i64k`, `_tuple_i64k`, `emit_intent_hashmap_struct_pair_llvm`),
each with 3 affected sites (`_get`'s "some" branch, `_insert`'s "previous value"
branch, `_remove`'s "removed value" branch) plus 3 matching "none"/placeholder sites
that also needed their literal `0`'s type string corrected from `{v}` to `i1`. The two
`_strk_strv`/`_i64k_strv` variants were NOT touched -- V is always `OwnedStr` there,
bool never reachable.

Fix: added a `trunc i8 to i1` conversion (via new Rust-level `is_bool_v`-gated
`vv_conv`/`old_v_conv`/`prev_v_conv` template placeholders) immediately before each
"value present" `insertvalue`, and swapped each "none" placeholder's literal-0 type
from `{v}` to a new `none_ty` placeholder (`i1` for bool, else unchanged). Two of the
six functions (`_strk`, `_vec_i64k`) had extra comment/free lines interrupting the
otherwise-identical template text, which an initial bulk `replace_all` missed --
fixed individually once the build's "unused formatting argument" errors pointed at
them directly (a nice side benefit: the fix couldn't silently miss a site without
Rust's own unused-arg lint catching it). New tests (`src/backend_llvm.rs`, all via
`lli` subprocess execution -- `compile_to_llvm()` alone wouldn't have caught this,
since it never asks `lli` to verify the module): `hashmap_bool_value_i64_key_
compiles_and_runs_on_llvm` + five sibling tests (`f64_key`, `owned_str_key`,
`vec_i64_key`, `tuple_i64_key`, `struct_key`) covering all six functions, plus
`hashmap_bool_value_insert_and_remove_option_round_trip_on_llvm` covering the
insert-previous-value and remove-removed-value sites specifically (not just get).
Full `cargo test --release` clean, zero regressions.

Category E status: `Array<bool, N>` literal and the direct-`return`-of-a-`vec(...)`-
literal path audited clean (regression tests added). `HashMap<K, bool>` (BUG-121),
nested `Vec<Vec<bool>>`, and a `Vec<bool>` struct-field READ all turned out to have
real bugs -- see BUG-122 below (found immediately after this paragraph was first
written, while adding the "clean pass" regression tests the audit method calls for;
running the tests is what caught it, not just writing them).

## BUG-122 (2026-08-06) -- two more `Vec<bool>` packed-vs-byte-addressed gaps on tree-LLVM

Found running (not just writing) the "clean pass" regression tests for category E's
`Vec<Vec<bool>>` and struct-field candidates: both looked fine when tested via `vanic
run`'s default path (which picks SSA-LLVM for these simple cases), but broke when
forced onto the tree-LLVM path (an unused `sqrt()` call disqualifies the whole module
from SSA eligibility, the same trick BUG-120 used) -- silently wrong output, not a
crash. Same root-cause class as BUG-109/121: something assumes byte-addressed `bool`
storage where the rest of the codebase uses `Vec<bool>`'s packed-bit layout.

**Bug 1 -- nested `Vec<Vec<bool>>` literal construction.** The "`vec(...)` as a nested
sub-expression" codegen in `backend_llvm.rs` (used for each ELEMENT literal of an
OUTER `vec(...)`, e.g. the inner `vec(true, …)` in `vec(vec(true, …), vec(false))`)
had no `Type::Bool` special case at all, unlike the direct `let x: Vec<bool> =
vec(...)` path (which already delegates to `emit_vec_bool_let_from_literal`). It fell
through to the generic path: allocate one BYTE per element, bitcast to `i1*`, store
each element as a separate byte -- then insert that `i1*` pointer straight into
`%intent_vec_bool`'s data field, which every reader (`Index`, `set_mut`, `clone`, …)
expects to be a packed `i64*` word buffer. Confirmed via direct IR inspection: the
outer literal's construction stored an unpacked `i1*` scratch pointer where a packed
`i64*` word pointer belonged.

Fix: added `emit_vec_bool_literal_value`, a value-returning counterpart of
`emit_vec_bool_let_from_literal` (same packed-word-buffer + `@intent_vec_bool__set_mut`
strategy, through a temporary scratch alloca instead of a `ctx.locals`-registered one,
so the final value can be `load`ed back out and returned as a plain SSA value). Wired
in as an early dispatch at the top of the nested-`vec(...)`-sub-expression handler.

**Bug 2 -- `Vec<bool>` struct-field READS.** The packed-bit `Index`-read special case
only fires when the indexed array is a bare `Var(name)` (looked up via `ctx.locals`)
-- `h.flags[i]`'s array is a `FieldAccess`, not a `Var`, so it fell through to a
SEPARATE generic "struct-field Vec read" branch that had no bool special case either,
misreading the packed `i64*` buffer as a byte-addressed `i1*` one (same wrong-type
mismatch as Bug 1, just on the read side of a differently-constructed value). The
WRITE side (`IndexAssign`) was already correct -- it only supports indexing a bare
local name at all (`h.flags[i] = v` isn't valid syntax; the reachable write path is
`set(mut ref h.flags, i, v)`), and that path's bool branch was already present and
correct, confirmed with a new regression test.

Fix: added the identical packed-bit read branch (word/bit `udiv`/`urem` math, bounds
check, `lshr`/`and`/`trunc`) to the struct-field `Vec<bool>` `Index`-read arm, keyed
off the field's own Vec-struct address instead of a `ctx.locals` alloca.

New/updated tests (`src/backend_llvm.rs`, all via `lli` subprocess execution on the
FORCED tree-LLVM path -- these are exactly the tests that would have silently passed
if only checked via `vanic run`'s default SSA-eligible path): `nested_vec_vec_bool_
literal_reads_back_correctly` and `struct_field_vec_bool_literal_reads_back_correctly`
now correctly pass under `run_lli` (tree-LLVM), plus new `struct_field_vec_bool_write_
via_set_then_read_back_correctly` covering the confirmed-clean write side. Full `cargo
test --release` clean, zero regressions.

Lesson for future audits: a "clean pass" test that only exercises the DEFAULT codegen
path (SSA-eligible, in this compiler) can miss a real tree-LLVM-only bug entirely --
worth deliberately forcing the non-default path (the `sqrt()`-dummy trick) for any
audit test that's specifically about codegen-path parity, not just adding the test and
trusting a pass.

## BUG-123 (2026-08-06) -- `print`ing a bare float literal misformatted on both SSA backends

Found auditing category G's own candidate list ("grep `operand_type(` call sites with
an `.unwrap_or`/`.unwrap_or_else` fallback and check each for BUG-111's exact silent-
wrong-default risk"). All 5 of the doc's own suggested repro shapes (int literal as an
`f64` function argument, as one element of a `Vec<f64>` literal, on one side of a float
comparison, as a struct field initializer, as the RHS of a compound-assignment -- the
last isn't even valid syntax, this language has no `+=`/`-=`) turned out already
correct. The actual gap was in a DIFFERENT `operand_type(...).unwrap_or(...)` call site
neither BUG-111 nor the doc's list mentioned: `intent_print_item`'s argument-type
dispatch, in BOTH `ssa_backend_c.rs` and `ssa_backend_llvm.rs` independently.

Repro: `fn main() -> i64 { print 5.5; return 0; }` -- a float literal passed directly
to `print`, no intermediate `let` to give it a `ValueId`. `vanic run` (default LLVM/SSA)
failed outright: `lli: ... error: floating point constant invalid for type` (`call i32
(i8*, ...) @printf(i8* %v_0.fmt, i64 5.5)` -- a float constant embedded where the
integer print branch expected an `i64` argument). `vanic run --backend=c` compiled and
ran, but silently printed `5` instead of `5.5` (`printf("%lld", (long long)(5.5))` --
the wrong format specifier AND a truncating cast).

Root cause: identical shape to BUG-111 -- `operand_type` returns `None` for a bare
`Operand::Const` (no `ValueId` to look up in `value_types`), and both backends'
`intent_print_item` handler independently defaulted the unresolved type to `Type::I64`
via `.unwrap_or(Type::I64)`, rather than deriving it from the constant's own variant.
For a bare integer literal this default happens to be correct (masking the bug for the
common case), but for a bare FLOAT literal it silently mis-selects the wrong printf
branch entirely.

Fix: `ssa_backend_llvm.rs` already has `const_operand_natural_type` (added for
BUG-111) -- reused directly, swapping the fallback to
`.unwrap_or_else(|| const_operand_natural_type(arg))`. `ssa_backend_c.rs` had no
equivalent helper; added the same three-variant match (`Const::Bool` -> `Bool`,
`Const::Float` -> `F64`, `Const::Int` -> `I64`/`U64` per range) inline at the one call
site. Swept both files for every other `Operand::Const(_) => None` site (3 more found,
all already using `.ok_or_else(...)` -> a proper `EmitError` rather than a silent wrong
default -- the safe pattern, not this bug's class, left unchanged). New test
(`src/lib.rs`): `bare_float_literal_print_item_infers_correct_type_on_both_ssa_backends`,
asserting the emitted SSA-LLVM IR never embeds a float constant on the integer branch
and the emitted SSA-C never takes the `%lld`/truncating-cast branch for this repro.
Full `cargo test --release` clean, zero regressions.

Category G status: the one real gap (`intent_print_item`) is now fixed on both
backends; the doc's own 5 suggested repro shapes were all already correct (regression
tests not added for those specifically, since they're negative results without a
distinguishing assertion beyond "compiles and runs" -- already covered by the existing
end-to-end test corpus's general float-literal-coercion coverage). Category G is now
fully audited.

## BUG-124 (2026-08-06) -- `vanic build --target=arm-*-linux-gnueabi*` misclassified as bare-metal

Found auditing category H's own 6-cell (bare_metal / cross / host-POSIX x C / LLVM)
link-flag-parity matrix. `is_bare_metal_triple` (`src/main.rs`) used substring checks
(`"none"` / `"eabi"` / `"-elf"`) as a proxy for "this is a freestanding microcontroller
target" -- but `"eabi"` also appears in a large family of REAL Linux userspace target
triples that use the EABI calling convention (soft/hard float) while having a full OS
and libc: `arm-unknown-linux-gnueabi`, `arm-unknown-linux-gnueabihf`,
`armv7-unknown-linux-gnueabihf`, ... (the Debian armel/armhf family -- Raspberry Pi OS
32-bit is exactly this). The existing unit test (`bare_metal_triple_detection`) never
covered this triple family at all, only `aarch64-unknown-linux-gnu`/`x86_64-unknown-
linux-musl` as "not bare metal" examples.

Verified against a REAL cross-toolchain (installed `arm-linux-gnueabi-gcc` +
`libc6-dev-armel-cross` for this investigation specifically, since the misclassification
otherwise silently "worked" in the sense of producing SOME broken output rather than a
loud, obviously-wrong error): `vanic build --target=arm-unknown-linux-gnueabi` on a
program calling `exp()` failed with `undefined reference to 'exp'`/`'erf'`/`'fmod'` --
BUG-112's exact class, since the misclassified triple took the bare-metal link branch
(which adds no `-lm` at all, on the theory that a freestanding target has no libc to
link against).

Fix: `is_bare_metal_triple` now checks for a real OS component (`"linux"`, `"darwin"`,
`"windows"`, `"freebsd"`, `"android"`) FIRST -- any triple naming an actual kernel/OS is
never bare-metal, regardless of its ABI suffix -- before falling through to the
existing freestanding-heuristic substrings. New unit tests in `src/main.rs` extend
`bare_metal_triple_detection` with the `gnueabi`/`gnueabihf` family (confirmed NOT
bare-metal) plus a genuinely-bare-metal EABI triple (confirmed the fix didn't weaken
detection there). New end-to-end test in `tests/run_end_to_end.rs`,
`vanic_build_cross_compiles_math_and_sort_program_for_real_arm_linux_target`, gated on
`arm-linux-gnueabi-gcc` being on `PATH` (skips gracefully otherwise, same pattern as
the `lli_available()`-gated LLVM tests -- this repo's own CI doesn't install this
specific cross-toolchain, only `aarch64-linux-gnu-gcc` for a separate lib-only QEMU
job) -- a real subprocess `vanic build --target=arm-unknown-linux-gnueabi`, checking
the link succeeds and the output is a genuine 32-bit ARM ELF binary.

## BUG-125 (2026-08-06) -- `sort`/`sort_by` failed to link on any non-x86 cross target

Found investigating BUG-124 on the same real ARM cross-toolchain: even after fixing the
bare-metal misclassification, a program calling `sort`/`sort_by` still failed --
`src/sort_runtime.c` (embedded into `vanic` via `include_str!`, unconditionally linked
into every LLVM-backend binary) starts with `#pragma GCC target("avx512f,avx512bw,
avx512dq,avx512vl,avx2,bmi2,popcnt")` and `#include <immintrin.h>` with NO architecture
guard at all. On `arm-linux-gnueabi-gcc` this failed to compile outright (`unknown
target attribute 'avx512f'`, `immintrin.h: No such file or directory`) -- `vanic build`
degraded this to a non-fatal WARNING and still produced a linked binary, but with
`intent_vec_i64__sort`/`intent_vec_double__sort` never defined, so any program actually
CALLING `sort`/`sort_by` failed at the FINAL link step with `undefined reference to
'intent_vec_i64__sort'`.

Root cause: the AVX-512 block-partition scan (`_block_part`, inside the `DEFINE_SORT`
macro, instantiated once for `int64_t` and once for `double`) uses `_mm512_cmpge_epi64_
mask`/`_mm512_cmplt_epi64_mask` to build a 64-bit "which of these BLOCK elements
qualify" bitmask, x86-only by construction.

Fix: gated the `#pragma GCC target`/`<immintrin.h>` behind `#if defined(__x86_64__) ||
defined(__i386__)` (setting a `VANI_SORT_HAVE_AVX512` flag), and extracted the two
mask-computation shapes (`>= pivot` / `< pivot`) into `VANI_SORT_MASK_GE`/
`VANI_SORT_MASK_LT` macros with two implementations: the existing AVX-512 one
(untouched, byte-for-byte identical logic, just relocated) on x86, and a portable
scalar loop (`for (bi = 0; bi < BLOCK; bi++) if (ptr[bi] >= pivot) mask |= 1 << bi;`,
using T's own native comparison operators) everywhere else -- same output shape, just
without the vectorized fast path. Verified the scalar fallback's correctness directly
(without needing `qemu-user`, which also isn't installed on this dev machine): patched
a copy of `sort_runtime.c` to force the non-x86 branch, compiled it standalone on this
native x86_64 host, and ran 2000 randomized `int64_t` + 500 randomized `double` sort
trials against `qsort`/insertion-sort references -- all matched exactly. The real cross
build (`arm-unknown-linux-gnueabi`, both math AND sort in one program) now compiles and
links cleanly with no warning, verified by the same new BUG-124 end-to-end test
(asserts the "sort runtime compilation failed" warning text is absent).

Aside (not fixed, out of scope): `#pragma GCC target("avx512f,...")` forces AVX-512
codegen for ANY x86 build regardless of the actual host CPU's capability (no runtime
CPUID dispatch) -- this dev machine's own CPU (Haswell) doesn't support AVX-512 and a
standalone test harness compiled with an explicit conflicting `-march=native` crashed
with an illegal instruction. The REAL `vanic build`/`vanic run` pipeline (which doesn't
pass `-march=native`) was unaffected -- confirmed executing a native (non-cross) build
of the same math+sort program on this exact machine, which ran correctly -- so this is
either already handled by the specific flags `vanic` passes, or a narrower, pre-existing
limitation unrelated to this session's fix; not chased further since it's orthogonal to
both this bug and the link-flag-parity theme of category H.

Full `cargo test --release` clean, zero regressions.

Category H status: the 6-cell matrix's bare-metal cell was fundamentally
misclassifying real Linux ARM targets (BUG-124, more severe than a single missing
flag), which also unmasked BUG-125 (an architecture-portability gap unrelated to link
flags specifically, found investigating the same real cross target). The audit's own
suggested `-lpthread` check was tested directly (a `Mutex`/task-using example
cross-compiled and linked cleanly without `-lpthread` on the same real toolchain --
modern glibc folds pthread symbols into libc itself, matching the existing `-lm`
helper's own comment about this) and found to be correct as-is, not a bug. Category H
is now fully audited.

## BUG-126 (2026-08-07) -- reassigning an Array binding broke C, reassigning a `ref T` binding corrupted memory on LLVM

Found picking up item A1 from `docs/UNRESOLVED_GAPS_TODO.md` (itself found chasing a
localfuzz backend-divergence finding). Two independent bugs sharing one root shape --
both live in `TypedStmt::Reassign` codegen, and both are reachable two ways: an
explicit `x = ...;` reassignment, or same-scope `let`-shadowing (`checker.rs`
deliberately desugars a same-scope `let` with a matching type into a `Reassign` node --
see its "Same-scope let -> Reassign" comment -- so shadowing and explicit reassignment
share the exact same codegen path).

**C backend, Array type**: `let xs: [i64; 5] = [1,2,3,4,5]; xs = [10,20,30,40,50];`
compiled to `v_xs = ((int64_t[5]){ 10, 20, 30, 40, 50 });` -- a hard `cc` error
(`assignment to expression with array type`), since C arrays aren't assignable via `=`.
`backend_c.rs`'s `Reassign` arm special-cases `Vec`/`OwnedStr`/`Struct`/`Enum` for
`drop_old` handling but had no `Type::Array` arm at all, so it fell into the generic
`name = expr;` catch-all -- the exact same catch-all the `Let` arm avoids by declaring
with a proper `T name[N] = {...};` initializer instead.

**LLVM backend, `ref T` type**: `let r: ref Point = shared(ref pt); r = shared(ref pt);
print area(r);` silently printed garbage (observed `1266547254094656` instead of `63`)
-- reading LLVM IR directly showed the smoking gun: `%t3 = call %Struct_Point*
@fn_shared(...)` (the first `let`'s result, no alloca -- see the L4(B) comment on the
Let arm: ref-typed lets skip the alloca and store the raw pointer VALUE itself in
`ctx.locals[name].1`), then `%t4 = call %Struct_Point* @fn_shared(...)` (the reassign's
new value), then `store %Struct_Point* %t4, %Struct_Point** %t3` -- reinterpreting
`%t3`, a `%Struct_Point*` VALUE (the address of `pt`), as though it were a
`%Struct_Point**` storage slot, and writing `%t4` into the first 8 bytes of `pt`
itself (its `x` field) as a raw pointer-bit-pattern. The later `call i64 @fn_area(
%Struct_Point* %t3)` then read `p.x` back as that garbage pointer value truncated to
an integer. `backend_llvm.rs`'s generic `Reassign` arm assumes `ctx.locals.get(name)`
always yields a real alloca address to `store` into; that assumption is simply false
for refs.

Fix: gave `Reassign` its own early carve-out in each affected backend, mirroring the
`Let` arm's existing special-casing instead of extending the generic path.
- `backend_c.rs`: `Type::Array` now writes into the EXISTING storage instead of
  declaring anything new -- per-element `v_xs[i] = ...;` stores for an `ArrayLit` RHS,
  `memcpy(v_xs, ..., sizeof(v_xs))` for any other RHS shape (including the
  struct-wrapper-unwrap case for a Call/Block/IfExpr/Match RHS), matching the `Let`
  arm's own two RHS shapes just without the leading declaration.
- `backend_llvm.rs`: `ty.is_any_ref()` now evaluates the RHS and rebinds
  `ctx.locals[name]` directly to the new pointer value -- no `store` at all, exactly
  mirroring the `Let` arm's own ref carve-out and matching true SSA rebind semantics
  (confirmed `ssa.rs`'s pure-SSA `Reassign` lowering already does the analogous
  "just remap the name to the new value" with zero store instructions -- this was
  purely a tree-LLVM gap, not present in the SSA path).

Checked all other let-shadowable types (struct, `Vec<i64>`, `Str`, tuple, enum,
`ref Vec<i64>`) via direct repros on both backends -- none showed the bug; it was
specifically Array-on-C and ref-on-LLVM. Root-caused via `vanic emit`'s raw C/LLVM IR
output rather than guessing -- the exact assignment-into-existing-storage vs.
declare-fresh-storage divergence was visible directly in the emitted C, and the
value-vs-address confusion was visible directly in the emitted LLVM IR.

Added `examples/language/english/reassign_array_and_ref.vani` plus an end-to-end test
(`reassign_array_and_ref_example_produces_correct_output_on_both_backends` in
`tests/run_end_to_end.rs`, real `vanic run` on both backends, checks actual stdout) and
two fast compile-only regression tests in `src/lib.rs`
(`bug126_array_reassign_uses_memcpy_not_invalid_c_assignment`,
`bug126_ref_reassign_rebinds_pointer_value_not_store_through_it_on_llvm`) pinning the
emitted C/LLVM text shape so a regression fails fast without needing a `cc`/`lli`
round-trip.

Full `cargo test --release` clean (2789 lib tests + all integration suites, 0 failed),
including the 3 new tests above.

## BUG-127 (2026-08-07) -- loop-carried checked-arithmetic elision used a stale pre-loop fact, LLVM hung instead of trapping on overflow

Found picking up item A2 from `docs/UNRESOLVED_GAPS_TODO.md` -- an EXPLICITLY
PREDICTED-BUT-NEVER-TESTED risk named by `docs/BUG_PATTERN_AUDIT_TODO.md` category B:
"could `ssa_pass.rs` ever flip `checked` from `true` to `false` on a Binary instruction
incorrectly? ... elision reasoning across a loop with a non-monotonic induction
variable." The prediction named the wrong file (turned out to be `checker.rs`, not
`ssa_pass.rs`) but the mechanism was exactly right.

Repro: `while n < 100 { if n == 5 { break; } n = n + -9223372036854775808; }` (n starts
at 0) -- on LLVM, `vanic run` HUNG FOREVER (confirmed via `timeout 5`, killed every
time); on C, correctly traps with `"integer overflow in int64_t add"` and exit 1.

Root cause: `checker.rs`'s overflow-elision pass (inside the badly-named
`try_elide_bounds_in_typed_expr`, which does BOTH Index-bounds elision AND
Add/Sub/Mul/Div/Rem/Shl/Shr elision) runs on the RHS of the loop's `n = n + ...;`
Reassign using whatever `smt_facts` are live at that point. Two existing call sites
(`if loops.is_empty() { drop_facts_mentioning(smt_facts, name); }`, both in
`check_one_stmt`) DELIBERATELY skip invalidating facts about a reassigned variable
when inside a loop -- by design, so a separate loop-invariant-preservation check can
still see the loop's ENTRY facts. But the elision pass runs on that SAME
not-yet-invalidated fact set: on the very first pass through the loop body, `n == 0`
(from the `let n: i64 = 0;` before the loop) was still live, and the "monotone result"
overflow-proof goal (`(b<=0 || a+b>=a) && (b>=0 || a+b<=a)`, evaluated over the SMT
wrapping-arithmetic model) DOES hold for `a=0, b=i64::MIN` (`0 + i64::MIN` wraps to
`i64::MIN`, which is `<= 0` -- no overflow). SMT proved the goal, `checked` flipped to
`false`, and the runtime guard was elided -- for a proof that's only valid on the FIRST
iteration. The checker doesn't do general loop-invariant inference, so it can't tell
that `n == 0` stops being true after the first iteration. On the SECOND iteration `n`
is actually `i64::MIN`, and `i64::MIN + i64::MIN` wraps (mod 2^64) to exactly `0` --
producing a genuine 2-cycle oscillation between `0` and `i64::MIN` that never equals 5
and never reaches 100, hence the infinite loop instead of a trap.

Confirmed directly by comparing LLVM IR before/after: `vanic emit --backend=llvm` on
the repro showed `%v_6 = add i64 %v_1, %v_5` (a bare, unchecked `add`) inside the loop's
body block, vs. the checked-add helper (`call i64 @__intent_checked_add_i64(...)`) that
the exact same source pattern produces OUTSIDE a loop, or for a genuinely
SMT-unprovable value (a plain function parameter).

Fix: added an `inside_loop: bool` parameter to `try_elide_bounds_in_typed_expr`
(threaded through its ~19 call sites, `!loops.is_empty()` at the 10 external
`check_one_stmt` sites, forwarded unchanged through its 9 internal recursive calls),
and skip the Div/Rem/Shl/Shr/Add/Sub/Mul-checked-elision arms entirely whenever
`inside_loop` is true -- deliberately conservative (loses some legitimate
would-have-elided optimization opportunities inside loops) rather than attempting
precise loop-invariant tracking, matching this codebase's established "sound over
clever" bias for this class of bug (see the SSA `Unreachable`-terminator memory, or
BUG-116's runtime-guard-fallback philosophy). The Index (array-bounds) elision arm in
the SAME function is intentionally left untouched by this guard: a `for i from 0 to
len(xs)` loop's own induction-variable facts (`i >= 0 && i < len(xs)`) are freshly and
soundly re-derived every iteration by construction (a different, already-correct
mechanism -- confirmed the existing `smt_elides_vec_bounds_in_for_loop_body` test still
passes unchanged). Verified none of the 8 existing `smt_elides_*` overflow/bounds/
divisor/shift elision tests are themselves inside a loop, so none regressed.

Added `examples/language/english/loop_carried_overflow_not_elided.vani` plus an
end-to-end test (`loop_carried_overflow_not_elided_example_traps_instead_of_hanging_
on_both_backends` in `tests/run_end_to_end.rs`, wrapped in the real `timeout` command
so a regression fails in 10s instead of hanging the suite/CI forever, matching the
BUG-109/echo_pool precedent) and a fast compile-only test in `src/lib.rs`
(`bug127_loop_carried_overflow_check_is_not_elided`, asserts `llvm.sadd.with.overflow`
survives in the tree-LLVM emission for the loop repro).

Full `cargo test --release` clean (2790 lib tests + all integration suites, 0 failed),
including the 3 new tests above.

## BUG-128 (2026-08-07) -- async fn's v3.1 state-machine transform hid a use-before-declare bug from both normal checks

Found picking up item A3 from `docs/UNRESOLVED_GAPS_TODO.md`. A plain (non-async)
function correctly rejects using a variable before its `let` declaration -- two
diagnostics fire: `"unknown variable 'n'"` and `"unreachable statement after a
control-flow exit"` (confirmed directly: `fn main() -> i64 { return n; let n: i64 =
5; }`). The identical shape inside an `async fn` whose body contains an `io_*_async`
call -- `return FetchResult.Ok(n); let n: i64 = io_recv_async(fd, size);` -- was
silently ACCEPTED (`vanic check` reported `ok`), and the full program hung on both
backends when actually run.

Root cause: `async fn` bodies with an `io_*_async` call go through the v3.1
state-machine transform in `parser.rs`'s `try_v31_transform` -- BEFORE `checker.rs`
ever sees the function. The transform splits the flat body into per-suspend-point
segments, wrapped as `while true { if state_tag==0 {...} if state_tag==1 {...} ...
}`, and promotes any local that crosses a suspend point into a field on a synthesized
`Task__<fn>` struct. Both of checker.rs's normal protections are defeated by this
BEFORE it gets to run on the ACTUAL bug:
- **Reachability**: the dead `let n = io_recv_async(...);` statement, originally
  "after an unconditional return in the same list," lands in its OWN `if
  state_tag==N` block after the split -- a syntactically distinct, genuinely
  reachable branch from the checker's point of view, not "after" anything anymore.
- **Use-before-declare**: `n` gets unconditionally promoted to a Task struct field
  (the promotion pass doesn't do reachability analysis -- it just collects every
  local that's read inside a segment other than the one that declares it), so every
  reference to `n` gets rewritten to `t.n`, a struct field access that's ALWAYS
  syntactically valid. There's no bare `Var("n")` left for "unknown variable" to
  ever fire on.

Confirmed via a real diff: the localfuzz finding that surfaced this
(`20260806-193817-run-crash-2767ef4c1c`) is the shipped
`examples/language/english/echo_p24_try_keyword.vani` example with exactly the
`let`/`return` pair swapped -- the shipped example itself is correct and unaffected.

Fix: added a narrow, syntax-only reachability check (`v31_reject_dead_code` +
`v31_stmt_terminates` in `parser.rs`) that runs on the flat, try-desugared async body
INSIDE `try_v31_transform`, right after `desugar_try_in_v31_body` and BEFORE the
ANF-lift + segment-splitting collector run -- i.e. on the body shape where "dead code
after a return, in the same list" is still syntactically true, before the transform
erases that fact. Deliberately mirrors only `checker.rs`'s reachability check (`Stmt`
kind shapes only, no scope/type info needed) rather than also re-implementing
use-before-declare detection in the parser -- the reachability half alone is
sufficient to catch and reject this bug pattern, since any use-before-declare shape
here necessarily has the "declare" half sitting in genuinely dead code. On rejection,
`try_v31_transform` returns its existing `Err(diag)` path, which the caller already
falls back from to the v1 synchronous async desugar -- that fallback re-runs the
FULL normal checker.rs pass on the (still correctly-ordered-in-source) body too, so
the rejected program actually surfaces BOTH the new reachability diagnostic and the
normal checker's own diagnostics (a bit more verbose than a single clean error, but
matches this codebase's existing behavior for every other v3.1-transform rejection
path, which already falls back the same way).

Verified the fix doesn't regress the legitimate case: `echo_p24_try_keyword.vani`
(correct ordering) still compiles and runs correctly on both backends, and all 79
existing `v31`-prefixed unit tests plus the `llvm_backend_run_produces_same_output_as_c`
end-to-end sweep (which runs dozens of async examples including this one) pass
unchanged.

Added `bug128_async_fn_use_before_declare_across_suspend_point_is_rejected` to
`src/lib.rs`, right next to the existing `v31_phase24_try_in_async_fn_accepted` test
it mirrors (same body, `let`/`return` swapped) -- asserts `compile()` now rejects with
either diagnostic.

Full `cargo test --release` clean (2792 lib tests + all integration suites, 0 failed).
