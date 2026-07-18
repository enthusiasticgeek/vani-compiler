# vāṇी — Current Work Queue

Actionable items fully within our control, ordered by effort.
Blocked items (macOS hardware, grammar consultant, IOCP) are at the bottom.

Last updated: 2026-07-17

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
  Stretch: `vec512<T>` for AVX-512 / future SVE-512 (same pattern, N×2).

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

- [x] **XL2. `vanic test` — built-in test runner** ✅ done 2026-07-16
  `#[test]` attribute in ast/ir/parser/checker sets is_test flag. `vanic test file.vani`
  collects is_test fns, synthesises a harness main (each fn called in order; pass = print
  "ok", fail = assert aborts with message), compiles+runs via CC. `resolve_combined_source()`
  public API in lib.rs for multi-file imports. 4 lib tests pass.

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
