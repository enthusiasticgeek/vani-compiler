# vāṇी (वाणी) — Project Status

> The project was renamed from `future_compiler` to **VANI** (वाणी, Sanskrit for "speech")
> on 2026-05-21. "VANI" expands to *Verbose Alternative Natural Interface* — the
> design goal is code that reads like speech, not punctuation.

> Single-page snapshot of what the compiler does today, what's queued
> next, and known issues. Update this file whenever a feature lands,
> a TODO is added/closed, or an issue is resolved/discovered.
> Cross-reference [README.md](README.md) for the language tour and
> [TODO.md](TODO.md) for the canonical work list.

## 📋 NEXT SESSION HANDOFF — 2026-07-10 (SIMD hardening + edge-case audit)

**State**: SIMD correctness bugs fixed, adversarial test suite grown to 84 files, QEMU/RISC-V documented. Compiler version `v0.2.4` (Cargo.toml).

### Shipped this session (2026-07-10) — SIMD hardening + edge-case audit

| Item | What shipped |
|------|-------------|
| `simd_load`/`simd_store` accept `ref Vec<T>` | Checker recognises `Type::Ref(Vec(T))`/`RefMut`; LLVM backend emits `load %intent_vec_T` before GEP+extract; C backend uses `->data` instead of `.data`; both paths tested |
| LLVM intrinsic name fix (`f32`/`f64` suffix) | `simd_reduce_add` for floats now emits `@llvm.vector.reduce.fadd.v4f32` not `v4float`; was silently wrong on every float SIMD reduce |
| `type_byte_size` Vec128 fix | `Vec128(_) => 16` added before the `_ => 8` fallback; prevented `parallel for` ctx struct underallocation when a `vec128<T>` variable was captured |
| 23 new SIMD lib tests (29 total) | Categories A–F: all 8 lane types, ref Vec roundtrip, LLVM intrinsic names, C backend `->data`, error rejection, parallel-for capture; all pass |
| Benchmark 11 (`11_simd_dot`) real numbers | vāṇी 27.8 ms vs C 41.5 ms vs C++ 44.4 ms vs Rust 37.6 ms on this machine; RESULTS.md merged (runner would have wiped old results; fixed by `git show` + manual merge) |
| 20 new edge-case adversarial tests (84 total) | P1 SIMD (4 mix + 2 xfail), P2 GEN cross (3), P3 ENM+STRT match extraction (1), P4 CONC complex (3), P5 CLO rich (3), P7 SMT×VEC/STRT (2), P8 TUP cross (2); all pass C + LLVM backends |
| TEST_MATRIX.md: SIMD as bucket 17 | Coverage matrix expanded to 17×17; 20 new cells pinned; documented-gaps list revised; minimum count pin raised 37→80 |
| `docs/qemu_testing.md` (new) | Full QEMU setup guide: AArch64 NEON/SVE, RISC-V RVV, what QEMU can/cannot test, CI snippets, bare-metal system-mode manual invocation |
| `docs/arm_neon_status.md` RISC-V section | RVV feature table, `--cpu=sifive-x280` + QEMU v=true example, known gaps list; QEMU testing sub-section added |
| `docs/simd_ffi_shims.md` "Future" → "Shipped" | Stale "A future Arc will add vec128<T>" replaced with shipped-v0.2.4 instruction table (x86-64/AArch64/RISC-V); "When to use" table updated with `vec128<T>` and RVV rows |

### Key numbers (2026-07-10 session 2)
- **Lib tests**: 2466+ passing (29 SIMD tests, 23 new this session)
- **Edge-case files**: 84 (up from 64); all pass C + LLVM backends
- **Commits this session**: 3 (SIMD fixes+tests, benchmark results, edge-cases, QEMU docs)

### Bugs fixed this session

| Bug | Impact | Fix location |
|-----|--------|-------------|
| `simd_load`/`simd_store` rejected `ref Vec<T>` arg | All SAXPY / dot-product examples in README, tutorial, benchmark were false (unreachable) | `checker.rs`, `backend_llvm.rs`, `backend_c.rs` |
| `simd_reduce_add` f32/f64 emitted `v4float` not `v4f32` | Every float SIMD reduction would produce invalid LLVM IR; `lli`/`opt` would reject | `backend_llvm.rs` |
| `type_byte_size` returned 8 for `Vec128` | `parallel for` context struct undersized by 8 bytes when a `vec128<T>` was captured; potential silent memory corruption | `backend_llvm.rs` |

### Open TODOs from this session
See new section **"SIMD / QEMU follow-up"** in `docs/TODO_CURRENT.md`.

### Blocked (unchanged)

| # | Item | Blocker |
|---|------|---------|
| B1 | crates.io publish | Needs API token |
| B2 | macOS verification | No macOS hardware |
| B3 | Grammar consultant pass | External reviewer needed |
| B4 | Windows IOCP async-TCP | ~25–35 h; readiness-vs-completion mismatch |
| ARM-3 | ARM hardware benchmarks | Need physical AArch64 board |
| RVV-bench | RISC-V benchmark numbers | Need physical RISC-V board (SiFive, Milk-V, StarFive) |

---

## 📋 NEXT SESSION HANDOFF — 2026-07-10

**State**: ARM/NEON + SIMD work complete. Compiler version `v0.2.3` (Cargo.toml).

### Shipped this session (2026-07-10) — ARM/NEON + SIMD

| Item | What shipped |
|------|-------------|
| `--cpu=<name>` flag | Forwards `-mcpu=<cpu>` to both `opt` and `llc`; defaults to `native` on host builds, empty on cross-compile |
| Target-aware `vectorize.width` | AArch64 targets get width=2 (NEON 64-bit pairs); x86-64 keeps width=4 |
| `--sve` / `--sve2` flags | Forward `-mattr=+sve` / `-mattr=+sve2` to `llc`; AArch64-only with clear error otherwise |
| ARM-5 bare-metal parallel-for docs | `02_parallel.md` + `04_embedded.md` notes on FreeRTOS `xTaskCreate` workaround |
| ARM-6 QEMU CI | `.github/workflows/ci.yml` job: `cargo test --lib` under `qemu-aarch64-static` |
| `#[vectorize]` attribute | Adds `llvm.loop.interleave.count = 4` on all while-loops; 2 new lib tests |
| FFI SIMD shim docs | `docs/simd_ffi_shims.md` — NEON `vaddq_s64` + AVX2 `_mm256_add_epi64` examples |
| `vec128<T>` type + 7 `simd_*` builtins | `simd_splat`, `simd_load`, `simd_store`, `simd_add`, `simd_sub`, `simd_mul`, `simd_reduce_add`; lowers to LLVM vector IR + GNU vector_size in C backend; 6 lib tests all pass |
| Tutorial `advanced/05_simd.md` | Three-layer SIMD guide; SAXPY + dot-product examples; AArch64/NEON instruction table; decision flowchart |
| Benchmark 11 (`11_simd_dot`) | f32 dot product: explicit `vec128<f32>` vs auto-vectorized scalar — vāṇी / C / C++ / Rust |
| README SIMD section | `vec128<T>` overview + builtin table in Part IV |

### Key numbers (2026-07-10)
- **Lib tests**: 2442+ passing (6 new SIMD tests)
- **Blocked (unchanged)**: B1 crates.io token · B2 macOS hardware · B3 grammar consultant · B4 Windows IOCP

### Blocked

| # | Item | Blocker |
|---|------|---------|
| B1 | crates.io publish | Needs API token |
| B2 | macOS verification | No macOS hardware |
| B3 | Grammar consultant pass | External reviewer needed |
| B4 | Windows IOCP async-TCP | ~25–35 h; readiness-vs-completion mismatch |
| ARM-3 | ARM hardware benchmarks | Need physical AArch64 board |

---

## 📋 NEXT SESSION HANDOFF — 2026-06-23

**State**: `v0.1.7` + `v0.1.8` tagged. `0.1.8` is the live version. Three new language features shipped (block comments, print blocks, positional break). All prior L18/L19 gaps remain resolved.

### Shipped this session (2026-06-23) — v0.1.7 + v0.1.8

| Item | What shipped |
|------|-------------|
| Tutorial expansion (v0.1.7) | 10 new pages: CLI ref, FnPtr primer, file I/O primer + worked example, math deep-dive (special fns + ML activations + bit ops), vec statistics, condvar primer, cross-compile primer, attributes reference, advanced collections. No compiler changes. |
| Block comments `/* ... */` | Multi-line, nestable to any depth; empty `/**/` supported; unterminated comment → clean diagnostic (no panic). Lexer change only. |
| Print block `print { ... }` | Group multiple print lines under one `print` keyword; each `;`-terminated group → separate output line. Desugared in checker to `TypedStmt::Print`; works in loops; both C and LLVM backends. |
| Positional break | `break inner` (innermost), `break middle` (second-from-innermost), `break outer` (outermost enclosing loop). Checker assigns synthetic labels `__vani_pos_N`; both SSA-C and LLVM backends search by label. |
| 8 adversarial tests | `examples/edge_cases/` — deeply nested comments, empty `/**/`, unterminated comment (xfail), print block inside `for`, break outer single-loop, break middle two-loops, break inner 3-deep nest (count=399). All pass on C + LLVM backends. |
| `RELEASE_NOTES/v0.1.7.md` | Tutorial expansion release notes |
| `RELEASE_NOTES/v0.1.8.md` | Block comments, print blocks, positional break release notes |

### Key numbers (2026-06-23)
- **Lib tests**: 2434+ passing
- **E2e tests**: all pass (Linux + Windows)
- **Elaboration coverage**: 597/597 (100%)
- **Dialects**: 62 across 26 scripts
- **Cargo.toml version**: `0.1.8`

### Blocked (4 items, external dependencies)

| # | Item | Blocker |
|---|------|---------|
| B1 | crates.io publish | Needs API token |
| B2 | macOS verification | No macOS hardware |
| B3 | Grammar consultant pass | External reviewer needed |
| B4 | Windows IOCP async-TCP | ~25–35 h; readiness-vs-completion mismatch |

---

## 📋 NEXT SESSION HANDOFF — 2026-06-21

**State**: `v0.1.5` + `v0.1.6` tagged. `0.1.6` is the live version. All bare-metal
gaps (L19) and file I/O gap (L18) are resolved. Remaining open work is in the
Blocked table below.

### Shipped this session (2026-06-21) — v0.1.5 + v0.1.6

| Item | What shipped |
|------|-------------|
| `FileHandle` type | Affine RAII handle; auto-`fclose`d at scope exit. Both C and LLVM backends. |
| File I/O builtins | `file_open`, `file_is_ok`, `file_read_line`, `file_write`, `file_close`, `file_flush` |
| Stdin / stdout helpers | `stdin_read_line() -> OwnedStr`, `flush_stdout() -> i64` |
| `eprint` statement | Writes to stderr; same multi-item syntax as `print` |
| L18 resolved | `docs/v1_limitations.md` item L18 (native file I/O) marked shipped |
| `--target=<triple>` | Cross-compilation flag on `vanic build` / `vanic run`; bare-metal triples suppress libc/OpenMP/pthread |
| `--no-std` | Suppresses all `#include <std*.h>` in C backend; auto-activates for bare-metal triples |
| `#[link_section = "..."]` | `__attribute__((section(...)))` in C; `section "..."` in LLVM IR |
| `#[no_mangle]` | Suppresses `intent_` prefix and Unicode mangling in both backends |
| `mmio_read/write_u8` | 8-bit volatile MMIO builtins (both backends) |
| `mmio_read/write_u16` | 16-bit volatile MMIO builtins (both backends) |
| QEMU user-mode run | `vanic run --target=<linux-triple>` invokes `qemu-<arch>-static` |
| L19 fully resolved | All 5 bare-metal gaps closed in `docs/v1_limitations.md` |
| `RELEASE_NOTES/v0.1.5.md` | File I/O release notes |
| `RELEASE_NOTES/v0.1.6.md` | Bare-metal release notes |

### Key numbers (2026-06-21)
- **Lib tests**: 2434+ passing
- **E2e tests**: all pass (Linux + Windows)
- **Elaboration coverage**: 597/597 (100%)
- **Dialects**: 62 across 26 scripts
- **Cargo.toml version**: `0.1.6`

### Blocked (4 items, external dependencies)

| # | Item | Blocker |
|---|------|---------|
| B1 | crates.io publish | Needs API token |
| B2 | macOS verification | No macOS hardware |
| B3 | Grammar consultant pass | External reviewer needed |
| B4 | Windows IOCP async-TCP | ~25–35 h; readiness-vs-completion mismatch |

---

## 📋 NEXT SESSION HANDOFF — 2026-06-19

**State**: `v0.1.2` tagged + published. `0.1.3-dev` active. All TODO_CURRENT
items within our control are done. Remaining open work is in the Blocked table.

### Shipped this session (2026-06-19) — v0.1.2

| Item | What shipped |
|------|-------------|
| SOV fn / struct / enum | `Name(…) fn { … }` / `Name struct { … }` / `Name enum { … }` top-level shapes. Token-rewrite + `parse_function` reuse. 3 new lib tests. |
| Win64 / AArch64 ABI | `is_ffi_safe_struct_win64` (size ∈ {1,2,4,8}) + `is_ffi_safe_struct_aarch64` (HFA + scalar≤16). Platform-dispatch + hints. 7 new lib tests. |
| Dialect purity docs | `enforce_language_purity` doc-comment corrected. 2 new dialect-rejection tests. |
| Devanagari aliases | `बाह्य`/`प्रकार`/`उद्देश्य`/`अपरिवर्तनीय` verified wired; 2 new tests. |
| `intentc` deprecation | Startup deprecation warning; `[[bin]]` removal deferred to v0.2.0. |
| Tutorials | Barrier primer, RwLock primer, default-methods primer. All added to SUMMARY. |
| Examples reorg | 14 Sanskrit + 12 Hindi + 12 Marathi under `examples/language/`; `// श्री।` headers. |
| `tools/vani_translate.py` v2 | Auto-detect, `--verify`, `--list-keywords`, `--batch`, `--inplace`. Tested 166/166 batch. |
| `parse_match_arms_block` | Refactored arm parsing out of keyword-first match; SOV match → clear error. |
| v1_limitations.md | L13 updated (SOV fn/struct/enum resolved); L15/L16/L17 marked shipped. |
| STATUS / TODO condensed | Pre-Arc-8 history archived to `*_ARCHIVE.md` files. |
| CHANGELOG | v0.1.2 entry added. |
| crates.io | `v0.1.2` tagged; publish blocked on API token (see Blocked table). |

### Key numbers (2026-06-19)
- **Lib tests**: 2421+ passing
- **E2e tests**: all pass (Linux + Windows)
- **Elaboration coverage**: 597/597 (100%)
- **Dialects**: 62 across 26 scripts
- **Cargo.toml version**: `0.1.3-dev`

---

## 📋 PREV SESSION HANDOFF — 2026-06-18

**State**: All 0.1.0 gate items (G1/G2/G3) satisfied. Ready to tag `v0.1.0`.
`forall` quantifiers shipped (commit `13b93cd`).
`enum` payload exhaustiveness shipped (commit `3e1260c`).
Closures (captures, HOF) complete.
Generic functions + structs + methods + interface impls on generic instantiations complete (commit `c89cfb5`).
All v1 limitations (L1–L12) closed. All 62 dialects shipped.
`volatile_read`/`volatile_write` builtins shipped. Elaboration 100% (597/597).

### Active work queue (pick top-to-bottom)

| # | Item | Deps | Notes |
|---|------|------|-------|
| ~~1~~ | ~~`forall` quantifiers in invariants~~ | — | ✅ DONE commit `13b93cd` |
| ~~2~~ | ~~`enum` payload exhaustiveness checking~~ | — | ✅ DONE commit `3e1260c` |
| ~~3~~ | ~~Closures (captures, map/filter/fold HOF)~~ | — | ✅ DONE (already fully implemented) |
| ~~4~~ | ~~**Generics** (foundational)~~ | — | ✅ DONE commit `c89cfb5` — methods + iface impls on generic instantiations |
| ~~0~~ | ~~Cut `v0.1.0` release~~ | G1+G2+G3 | ✅ DONE — tagged `v0.1.0`; `v0.1.1` ships items 5–8 |
| ~~5~~ | ~~Traits/interfaces phase 2~~ | generics | ✅ DONE commit `e97ea6a` — default methods + blanket impls |
| ~~6~~ | ~~Parametric `Mutex<T>` / `Guard<T>`~~ | generics | ✅ DONE — checker infers T from args; tree-C emits per-T bundles via `collect_mutex_specs` + `emit_mutex_bundle` |
| ~~7~~ | ~~Parametric `Channel<T>`~~ | generics | ✅ DONE — checker allows struct/enum elements; tree-C uses `c_element_storage`+`memset`; LLVM uses `channel_slot_llvm_string` for aggregate slots |
| ~~8~~ | ~~`RwLock<T>` / `Barrier` / `CondVar`~~ | generics | ✅ DONE — Barrier commit `01bc0b3`; RwLock<T>/ReadGuard/WriteGuard commit `3365c07`; CondVar was already complete |

### Completed since 2026-06-13

#### ~~Phase 6 / 8a / 8b / 10 / 12 / 13 language batches~~ ✅ ALL 62 DIALECTS SHIPPED

All 62 non-English dialects are in `tests/run_end_to_end.rs` and pass
on both backends. Includes: Gujarati, Punjabi-Gurmukhi, Odia, Assamese,
Tamil, Telugu, Kannada, Malayalam, Sinhala, Urdu, Sindhi, Punjabi-Shahmukhi,
Persian, Pashto, Spanish, French, Russian, German, Italian, Portuguese,
Dutch, Swedish, Norwegian, Danish, Finnish, Hebrew, Armenian, Georgian,
Japanese, Mandarin, Korean, Arabic, Amharic, Tibetan, Mongolian, Cherokee,
Lao, and more.

#### ~~Phase 11 L3 — match-by-ref scrutinee~~ ✅ DONE (commit `c0a4620`)

`ref Option<Vec<i64>>` scrutinee now works — non-Copy payload bindings get
type `ref PayloadType` (borrow, not move). Both C and LLVM backends.
Example: `examples/language/english/match_ref_payload.vani`.

#### ~~Installers + GitHub release workflow~~ ✅ DONE

`install.sh`, `install.ps1`, `.github/workflows/release.yml` (matrix build
for 5 targets). All landed in the session that followed the 2026-06-13 handoff.

#### ~~GitHub Pages Playground fix~~ ✅ DONE

All 221 ` ```rust ` fences in tutorials renamed to ` ```vani `; `book.toml`
`runnable = false` added. mdBook no longer sends vāṇी code to
`play.rust-lang.org`.

#### ~~Big-O `--big-o` wired into `vanic run`~~ ✅ DONE (commit `5b73db7`)

All three subcommands (`check`, `emit`, `run`) now accept `--big-o[=auto|force|off]`.
`vanic run --big-o` prints per-fn complexity to stderr before executing.
2 new lib tests verify Auto (O(1) skipped) and Force (O(1) included) modes.

#### ~~Windows IOCP / async-TCP~~ ✅ DONE (commit `8193760`)

Root cause identified: `WSAECONNRESET` (10054) / `WSAECONNABORTED` (10053)
from a RST-close were mapped to error (-1), not EOF (0). The state machine
then yielded forever while WSAPoll kept returning "ready" on the error socket.
Fix: C and LLVM Windows `recv_nb` helpers now return 0 for both reset codes.
All 5 async-TCP examples (`async_showcase`, `echo_loop`, `echo_loop_break`,
`echo_match_stress`, `tcp_echo_epoll`) pass on both backends.
`echo_loop_windows_byte_count_matches_c` de-ignored.

#### ~~3 Windows edge tests~~ ✅ VERIFIED

`windows_brahmi_numeral_output_no_crt_reorder`,
`windows_tcp_echo_blocking_three_clients`,
`windows_snprintf_dprintf_shim_roundtrip` — all pass.

#### ~~`volatile_read` / `volatile_write` builtins~~ ✅ DONE (commit `2cea04a`)

Embedded MMIO access. `volatile_read(ref reg) -> T` emits `load volatile`
(LLVM) / `*(volatile T*)` (C). `volatile_write(mut ref reg, val: T)` emits
`store volatile`. Gated by `INTENT_TARGET_EMBEDDED=1`; hosted diagnostic
points to `Atomic<T>`. `examples/embedded/mmio_blink.vani` smoke example.
All 4 backends (tree-C, tree-LLVM, SSA-C, SSA-LLVM).

#### ~~Error-message elaboration~~ ✅ DONE 100% (commit `326ccad`)

597/597 diagnostic sites have elaboration. 20–30+ high-value families seeded
with step-by-step WHAT/WHY/HOW breakdowns. Integration tests for elaboration
JSON format ship in `tests/`.

### Pick up in this order

#### 1. Tutorials — analogy-first rewrite ✅ DONE (commits `bb9596d`, `6c2b6d8`)

Both passes complete. SUMMARY reordered, 65 chapters have analogy/orientation
openers. `unsafe(reason="...")` bare-`unsafe` bug in `04_embedded.md` fixed.

#### 2. Tutorials — missing feature coverage ✅ HIGH + MEDIUM DONE (commits `d176936`, `a8c2d68`, `b582d55`)

All HIGH and MEDIUM gaps filled:
- `intermediate/13_option.md` — `Option<T>` + all `option_*` builtins
- `intermediate/14_collections.md` — `HashMap<K,V>` + `HashSet<T>` full API
- `intermediate/15_math_rng.md` — math builtins, `seed_rng`/`rand_*`, `clone`
- `intermediate/06_closures.md` addendum — `vec_map`/`vec_filter`/`vec_sum`/`vec_any`/`vec_all`
- `beginner/06_strings.md` addendum — full string builtins reference table + sampler
- `beginner/02_variables.md` addendum — bitwise operators (&, |, ^, ~, <<, >>)
- `advanced/04_embedded.md` addendum — `#[no_heap]`/`#[bounded_stack]`/`#[deterministic_timing]`/`#[recursion_bound]`

**LOW (add only when asked):**
`BTreeMap`/`BTreeSet`/`Deque`, binary heap, union-find, trie, skiplist, bloom
filter, graph algorithms, hash utilities, `mmio_read_u32`/`mmio_write_u32`,
`aref_load`/`aref_store`, `bptr_*`.

#### 3. Deferred / long tail (touch only when asked)

- **Phase 9a** — ML-3 LoRA fine-tune (~25h + ~$200 GPU)
- **Phase 13** — Browser WASM playground (50h+)
- **macOS runtime verification** — no Darwin host available
- **Grammar consultant pass** on 62 dialects
- CI / GH-Actions deploy (Tier 4 — last)

---

### Key numbers (2026-06-16)
- **Lib tests**: 2108+ passing (2108 as of `c1cfb2e`; +elaboration + big-o tests since)
- **E2e tests**: all pass (Windows async-TCP fully unblocked as of `8193760`)
- **Elaboration coverage**: 597/597 (100%)
- **Dialects**: 62 across 26 scripts
- **Last commit**: `b582d55` — tutorials HIGH+MEDIUM gap chapters

---

## 📋 PREV SESSION HANDOFF — 2026-06-12

**State**: Edge tests + `volatile_read`/`volatile_write` shipped.
2108 lib tests + all e2e tests pass on Windows 11 GNU toolchain.

### Shipped this session (2026-06-12)

**`volatile_read`/`volatile_write` built-ins** (commit `2cea04a`):
- `volatile_read(ptr: ref i64) -> i64` — LLVM: `load volatile i64, i64*`; C: `*(volatile int64_t*)`. All 4 backends.
- `volatile_write(ptr: mut ref i64, val: i64) -> i64` — volatile store; returns 0. All 4 backends.
- Gated by `INTENT_TARGET_EMBEDDED=1`; hosted diagnostic points to `Atomic<T>`.
- `examples/embedded/mmio_blink.vani` smoke example.

**Edge test batch** (commit `826cf18`, 2100→2108 lib tests):
- `runtime_i64_max_plus_one_emits_no_overflow_guard` — verifies no `add nsw i64` / `__builtin_add_overflow`
- `runtime_i64_min_minus_one_emits_no_overflow_guard`
- `runtime_i64_min_times_neg_one_emits_no_overflow_guard`
- `runtime_u64_max_plus_one_compiles_both_backends`
- `vec_ref_push_after_source_borrow_ends_compiles`
- `struct_ref_field_survives_method_call_compiles`
- `windows_deep_recursion_no_stack_overflow` / `non_windows_deep_recursion_no_stack_overflow`
- `windows_llvm_print_uses_printf_not_putchar`

**Edge tests still pending** (low priority, require e2e infra):
- `windows_brahmi_numeral_output_no_crt_reorder` — needs binary execution
- `windows_tcp_echo_blocking_three_clients` — needs live TCP server
- `windows_snprintf_dprintf_shim_roundtrip` — needs binary execution

**Error-message elaboration** (commits `6cde30d`, `52004c6`, `29db5fa`):
- 9 new families: `struct_literal_missing_field`, `method_not_found`, `unknown_struct_type`,
  `assign_to_unknown_variable`, `for_over_non_iterable`, `iface_impl_missing_method`,
  `pure_fn_calls_non_pure` + 2 more wired from existing families.
- 48 checker.rs sites now have elaboration (was 32). 34 families total (was 25).
- 5 elaboration coverage tests added (13 total in the elaboration suite).
- 2111→2113 lib tests; all pass.

---

## 📋 PREV SESSION HANDOFF — 2026-06-11

**State**: All Tier 1 + Tier 2 items shipped. Windows full e2e parity achieved (commit `6255af8`).
2089 lib tests + all e2e tests pass on Windows 11 GNU toolchain.

### Pick up in this order

#### 1. Add edge test cases (low effort, high value — do first)

These are concrete gaps in test coverage identified 2026-06-11.
Each should be a `#[test]` in `src/lib.rs` unless noted.

**Windows CRT / JIT regression tests** — prevent regressions in the fixes just landed:
- `windows_brahmi_numeral_output_no_crt_reorder` — run `../sanskrit/sov_demo.vani` via LLVM backend on Windows; assert label text and Brahmi numerals interleave correctly (not all labels then all numerals). Currently covered by the parity test; add a standalone assertion for the specific ordering.
- `windows_snprintf_dprintf_shim_roundtrip` — emit a program that calls `snprintf`/`dprintf` via FFI and check output matches expected; ensures shim stays wired if symbol resolution changes.
- `windows_deep_recursion_no_stack_overflow` — compile + run a vāṇी program with ≥ 800 nested recursive calls; should complete without a stack overflow on Windows (64 MB thread). Use the `fib` or `ackermann` shape.
- `windows_tcp_echo_blocking_three_clients` — `tcp_echo.vani` runs to completion with 3 sequential clients on Windows via both backends (ws2_32 fix). Currently only covered by parity test; add an explicit Windows-only assertion.

**Integer overflow edge cases** (documents wrapping, prevents accidental "fixes" that break the known behavior):
- `i64_max_plus_one_wraps_to_min` — `9223372036854775807 + 1` at runtime → `-9223372036854775808`; both backends must agree.
- `i64_min_minus_one_wraps_to_max` — `-9223372036854775808 - 1` → `9223372036854775807`.
- `i64_min_times_neg_one_wraps_to_min` — `-9223372036854775808 * -1` → `-9223372036854775808` (wraps, not positive).
- `u64_max_plus_one_wraps_to_zero` — `18446744073709551615u64 + 1` → `0`.
- `const_overflow_is_a_type_error` — `const N: i64 = 9223372036854775807 + 1;` must be rejected at compile time (contrast with the runtime wrapping tests above).

**Generic monomorphization edge cases:**
- `nested_generic_three_level_chain_fails` — `h<T>` calls `g<T>` calls `f<T>`; only `h<i64>` called from non-generic; expect compile error mentioning `f` or `g` never specialized. (2-level already tested; add 3-level.)
- `nested_generic_nongeneric_bridge_works` — `h<T>` calls non-generic `bridge_i64()` which calls `f<i64>`; should compile and run, because `f<i64>` IS called from a non-generic site.
- `nested_generic_same_type_two_call_sites` — `f<i64>` called from both `main()` (non-generic) and `g<T>` (generic); should compile since the non-generic call site provides the specialization.

**OwnedStr / match arm edge cases:**
- `ownedstr_all_arms_must_produce_same_type` — variant with `String` payload: the "found" arm does `s + ""` (→ OwnedStr), the "not found" arm must also produce OwnedStr (`"" + ""` not `""`); mixing types across arms must be a type error.
- `ownedstr_nested_match_concat_workaround` — nested match where inner match produces OwnedStr; outer arms must all agree on OwnedStr, not mix Str literals.

**Ref / lifetime edge cases:**
- `ref_return_three_ref_params_rejects` — fn with three ref params trying to return a ref; the single-ref-param elision rule must reject (extend existing 2-param test).
- `vec_ref_push_after_source_borrow_ends` — borrow of `x` as `ref T`, push into vec, borrow ends; then re-borrow `x` as `mut ref T` and mutate; should compile since first borrow is over.
- `struct_field_ref_lifetime_survives_method_call` — struct holding `ref T` field; call a method that reads the field; struct must stay valid for the ref's scope.

**Async state machine (Windows mismatch — investigate before skipping forever):**
- `echo_loop_windows_byte_count_matches_c` — currently skipped on Windows with `#[cfg(not(target_os = "windows"))]`. Add a `#[cfg(target_os = "windows")]` variant that at minimum documents the mismatch: run both backends, print expected vs actual, then `#[ignore]` the assertion until IOCP is fixed. This prevents the mismatch from being silently forgotten.

#### 2. User-queued features (pick any one per session)

| Feature | Effort | Entry point |
|---|---|---|
| **`volatile_read` / `volatile_write` built-ins** | 4–6h | Embedded MMIO — access-level volatile, NOT a type qualifier. See TODO.md user-direction items for full design rationale. Entry: parse as built-in calls, checker enforces `unsafe {}` + embedded triple, LLVM backend emits `load/store volatile`, C backend emits `*(volatile T*)` cast. 3 lib tests + `examples/embedded/mmio_blink.vani`. |
| **Error-message elaboration** | 8–15h | `src/checker.rs` + `src/diagnostic.rs` — add elaboration vec to DiagnosticError; seed 20–30 most common families |
| **Big-O annotation** (`--big-o` flag) | 12–20h | New `src/big_o.rs` pass; hook into `vanic check` output; v1 scope: loop-nesting depth + known builtin asymptotics |
| **Tutorials rewrite for non-CS readers** | 20–40h | `tutorials/src/beginner/` + `tutorials/src/intermediate/` — add analogy chapters before formal definitions |

#### 3. Windows IOCP (larger arc — D.1 in TODO.md)

Currently skipped: `tcp_echo_epoll.vani`, `echo_loop.vani`, `echo_loop_break.vani`,
`async_showcase.vani`, `echo_match_stress.vani`.

Root cause: `epoll_wait_one` IOCP shim uses blocking `GetQueuedCompletionStatus` but the
sockets are opened in blocking mode — IOCP requires sockets to be opened with
`WSA_FLAG_OVERLAPPED` and all I/O submitted via `WSASend`/`WSARecv` with OVERLAPPED structs.
Entry: `src/backend_llvm.rs` `emit_intent_epoll_helpers_llvm_windows` +
`examples/tcp_echo_epoll.vani`.

#### 4. Tier 3 / deferred (touch only if asked)

- Grammar consultant pass — native-speaker review of 62 dialects
- macOS empirical verification — needs Darwin host
- Arc 9 Kosh package manager — pending registry choice
- CI / GH-Actions (Tier 4) — last

### Key numbers
- **Lib tests**: 2089 passing (Windows + Linux)
- **E2e tests**: all pass (5 async-TCP tests skipped on Windows)
- **Dialects**: 62 across 26 scripts
- **Last commit**: `6255af8` — Windows full e2e parity

---


---

> **Pre-Arc-8 session history archived** → [STATUS_ARCHIVE.md](STATUS_ARCHIVE.md)
> Contains all 🟢 Session logs from 2026-06-06 through 2026-06-08 and the
> pre-v0.1.0 remaining-work snapshot. The current work queue is in
> [docs/TODO_CURRENT.md](docs/TODO_CURRENT.md).

---

---

## Known issues

These are caveats present in the current implementation. Each links to
the TODO that would resolve it (or notes that the trade-off is
intentional). Resolved entries are deleted, not struck through —
TODO.md keeps the history.

### Backend / codegen
- **Full concurrency surface now flows through SSA on both backends.** `Atomic`, `Mutex`/`Guard`, `Channel`, `parallel for`, and `task`/`join` all use the SSA path; only multi-block task bodies and other shape-recognizer mismatches fall back via `EmitError`. (No active TODO in this row — kept here to document the milestone.)
- **No cross-compilation.** intentc bakes the host's `target_os` / `target_arch` into the emitted artifact (e.g., `SYS_futex` number, threading dispatch, link flags). A `--target=` flag is out of scope for v1; both C and LLVM backends emit code that only links on the same OS intentc was built for. *Trade-off, not a bug — flag separately if needed.*
- **Win32 parallel-for thread count reads `OMP_NUM_THREADS` at codegen time (default 4).** The fan-out N is resolved when intentc runs, avoiding `@getenv` linkage in the generated LLVM IR. Set `OMP_NUM_THREADS=N` before invoking intentc to control the worker count (1–256). *A future revision could switch to a runtime lookup if LLVM-version-independent IR is no longer required.*

### Verifier / SMT
- **Natural-exit `!cond` post-loop fact omitted when the body can `break`.** Would be unsound; the verifier conservatively drops the fact. *Working as intended.*
- **`prove foo(args) > 0;` only works if `foo` has `ensures`.** Calls to functions without ensures fall back to "unsupported" since the solver has no fact about the return value. *Working as intended — declare ensures.*
- **Bare `let inner = xs[i]` is rejected for `Vec<non-Copy>`.** Direct indexing would alias the owner's slot and double-free; the checker emits a clear hint pointing users at the new `clone_at(&xs, i)` builtin that returns an owned deep-clone of the slot. *Working as intended — clone_at is the explicit opt-in for non-Copy slot reads.*

### Language surface gaps
- **No mutable references to atomics-as-payloads.** Workaround: pre-extract scalars before spawning a task. *Tracked indirectly by future affine-rules work.*
- **References are second-class.** `&T` / `&mut T` only as function parameter types; not as returns, let-bindings, or aggregate elements. *Working as intended for v1 — Rust-style first-class references are explicitly out of scope.*
### Tooling
- **`INTENTC_NO_VERIFY=1` skips every SMT round-trip.** Useful for fast iteration; do not set in CI — a violated `ensures` won't surface. Runtime safety guards stay in place. *Working as intended.*

---

## Update protocol for this file

When you finish a unit of work, update STATUS.md in the same commit:

- **Feature added** → add a bullet to the matching subsection. Keep the wording terse; the README has the long form.
- **TODO closed** → delete it from the TODO list above; if it had a Known Issues entry, delete or rewrite that entry too.
- **TODO added** → insert at the priority position; cross-reference any related Known Issues entry.
- **Issue discovered** → add to Known Issues; if a fix is planned, also add a TODO and link them.
- **Issue resolved** → delete the entry; do not strike through (`~~`). TODO.md preserves the history if you need it.
- **Test totals shifted** → update the header line.
- **Date roll** → update `Last updated:` to today.
