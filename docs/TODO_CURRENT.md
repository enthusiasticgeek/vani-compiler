# vāṇी — Current Work Queue

Actionable items fully within our control, ordered by effort.
Blocked items (macOS hardware, grammar consultant, IOCP) are at the bottom.

Last updated: 2026-07-21

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
  (item 26.1 below) plus `str_pad_left(i64_to_str(n), w, "0")` already cover the
  *capability* (fixed-decimal floats, zero-padded ints) as ordinary function
  calls; this item is specifically about adding literal `{:03}`-shaped syntax
  at a print call site, e.g. `print x:03;` or a template-string form.
  **Scope, if picked up**: `PrintItem`/`TypedPrintItem` has 132 match sites
  across 13 files (parser.rs, checker.rs, ssa.rs, backend_c.rs, backend_llvm.rs,
  lsp.rs, safety.rs, stack_depth.rs, deviations.rs, format.rs, hashmap_bundle.rs,
  main.rs, lib.rs) — adding a format-spec field touches most of them
  non-mechanically (effects/purity checking, `vanic fmt` pretty-printing, LSP
  hover, codegen in both C and LLVM backends). The formatting codegen itself is
  easy once the plumbing exists (a compile-time-constant width/precision is
  just a computed `"%03lld"` snprintf format string, same trick already used
  for `%llu`/`%g`) — nearly all the cost is parser/AST/multi-pass plumbing, not
  runtime logic. Multi-day effort, not a quick add.
  **Design question to settle first**: this would be the first "magic syntax
  inside/beside a print item" feature in the language — worth a deliberate
  decision (does it fit vāṇी's explicit-over-implicit posture, e.g. mandatory
  `unsafe(reason=...)`, no operator-overloading magic?) rather than adding it
  as a side effect of wanting decimal padding, which the cheap path already
  solves. Gate on: is there real demand for the syntax itself, not just the
  formatting capability.

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
