# vāṇी — Current Work Queue

Actionable items fully within our control, ordered by effort.
Blocked items (macOS hardware, grammar consultant, IOCP) are at the bottom.

Last updated: 2026-07-06

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

- [ ] **ARM-6. Add AArch64 GitHub Actions runner**
  `.github/workflows/release.yml` only tests `ubuntu-latest` (x86-64).
  Add a `ubuntu-24.04-arm` or self-hosted Graviton job that runs the full
  `cargo test` suite. **BLOCKED**: needs CI runner budget / self-hosted Pi.

---

## Blocked (not in our control)

| Item | Blocker |
|---|---|
| macOS empirical verification | Darwin hardware needed |
| Grammar consultant pass | External native-speaker review |
| Windows IOCP async-TCP (`tcp_echo_epoll` etc.) | Readiness-vs-completion model mismatch (R8 in decisions.md) |
| Arc 7 Win64 / AArch64 CI wiring | CI runner setup |
| crates.io publish (item 1) — v0.1.2 tagged and ready | crates.io API token needed (`cargo login`) |
| ARM-6 AArch64 CI | CI runner budget / self-hosted hardware |
