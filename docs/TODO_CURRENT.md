# vāṇी — Current Work Queue

Actionable items fully within our control, ordered by effort.
Blocked items (macOS hardware, grammar consultant, IOCP) are at the bottom.

Last updated: 2026-06-19

---

## Immediate (< 1 h)

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

- [ ] **18. `--target <triple>` cross-compilation flag** (G1 — highest impact)
  Add `--target arm-none-eabi` / `--target riscv32-unknown-none-elf` etc.
  to the LLVM backend path. Instead of running `lli` (JIT on host), invoke
  `llc -march=<arch>` to produce object code, then the appropriate cross
  linker. Also wire `--target` into `vanic build` / `vanic run` (run via
  QEMU if a target emulator is available).
  - Entry point: `src/main.rs` — add `--target` CLI flag; `src/backend_llvm.rs`
    — replace `lli` invocation with `llc` + cross linker path.
  - **Effort**: ~6–10 h. Unlocks the LLVM backend for real cross-compilation.

- [ ] **19. `--no-std` mode — omit libc prelude in C backend** (G2)
  A `--no-std` flag (or inferred from `--target` when it's a bare-metal
  triple) that makes `vanic emit-c` / `vanic build --backend=c` skip the
  `#include <stdio.h>` / `#include <stdlib.h>` / `#include <string.h>` etc.
  headers and instead emit only minimal forward declarations for what the
  program actually uses.
  - Entry point: `src/backend_c.rs` — the prelude-emit function; gate
    includes behind a `no_std: bool` flag threaded from `src/main.rs`.
  - **Effort**: ~3–4 h.

- [ ] **20. `#[link_section = "..."]` attribute** (G3)
  Allow `fn` declarations and top-level `let` bindings to be placed in
  a named linker section. Emits `__attribute__((section("...")))` in C,
  `section` metadata in LLVM IR.
  - Required for: interrupt vector tables, `.rodata` placement, boot code
    at the reset vector address.
  - Entry point: `src/ast.rs` — add `link_section: Option<String>` to
    `Function`; `src/parser.rs` — parse `#[link_section = "..."]`;
    `src/backend_c.rs` + `src/backend_llvm.rs` — emit the attribute.
  - **Effort**: ~4–6 h.

- [ ] **21. `#[no_mangle]` attribute — suppress symbol name mangling** (G4)
  Allow `fn` declarations to opt out of the `intent_` prefix and name
  mangling so the linker script can reference them by their literal vāṇी
  name (e.g. `Reset_Handler`, `_start`, `HardFault_Handler`).
  - Entry point: `src/ast.rs` — add `no_mangle: bool` to `Function`;
    `src/backend_c.rs` + `src/backend_llvm.rs` — skip `function_name()`
    mangling when `no_mangle` is set; `src/parser.rs` — parse `#[no_mangle]`.
  - **Effort**: ~2–3 h (mostly parser + two codegen paths).

- [ ] **22. MMIO 8-bit and 16-bit variants** (G5)
  Add `mmio_read_u8(addr: i64) -> u8`, `mmio_read_u16(addr: i64) -> u16`,
  `mmio_write_u8(addr: i64, val: u8)`, `mmio_write_u16(addr: i64, val: u16)`.
  Lower to `*(volatile uint8_t*)` / `*(volatile uint16_t*)` in C and to
  a volatile `i8`/`i16` `load`/`store` in LLVM IR.
  - Entry point: `src/checker.rs` — add to `check_mmio_builtin`;
    `src/backend_c.rs` + `src/backend_llvm.rs` — add codegen arms.
  - **Effort**: ~2–3 h (same pattern as existing `mmio_read_u32`).

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

## Blocked (not in our control)

| Item | Blocker |
|---|---|
| macOS empirical verification | Darwin hardware needed |
| Grammar consultant pass | External native-speaker review |
| Windows IOCP async-TCP (`tcp_echo_epoll` etc.) | Readiness-vs-completion model mismatch (R8 in decisions.md) |
| Arc 7 Win64 / AArch64 CI wiring | CI runner setup |
| crates.io publish (item 1) — v0.1.2 tagged and ready | crates.io API token needed (`cargo login`) |
