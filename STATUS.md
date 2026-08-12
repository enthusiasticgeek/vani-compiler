# vāṇी (वाणी) — Project Status

> The project was renamed from `future_compiler` to **VANI** (वाणी, Sanskrit for "speech")
> on 2026-05-21. "VANI" expands to *Verbose Alternative Natural Interface* — the
> design goal is code that reads like speech, not punctuation.

> Single-page snapshot of what the compiler does today, what's queued
> next, and known issues. Update this file whenever a feature lands,
> a TODO is added/closed, or an issue is resolved/discovered.
> Cross-reference [README.md](README.md) for the language tour and
> [TODO.md](TODO.md) for the canonical work list.

## 📋 NEXT SESSION HANDOFF — 2026-08-12 (BUG-81 through BUG-184; v0.9.1, v0.9.2, v0.9.3 releases)

**State**: ten days, ~104 bugs (BUG-81 through BUG-184), and three patch
releases (`v0.9.1` 2026-08-07, `v0.9.2` 2026-08-11, `v0.9.3` 2026-08-12)
since the last handoff entry below. This entry summarizes the major
threads at session-cluster granularity rather than bug-by-bug — every
individual fix has its own full writeup in
[docs/TODO_CURRENT.md](docs/TODO_CURRENT.md) (search the `BUG-NNN`
number), and each release's `CHANGELOG.md`/`RELEASE_NOTES/vX.Y.Z.md`
entry groups its own range by theme. Compiler version `0.9.4-dev`
(`Cargo.toml`); last tagged release `v0.9.3`.

### Shipped across this window (2026-08-03 → 2026-08-12)

| Item | What shipped |
|------|-------------|
| Feature-combination gaps sweep (BUG-81–104) | 49-row sweep across 11 categories (SIMD × containers/generics, generics × concurrency handles, SMT contracts × generics/enums, async × everything, `dyn` × generics, `try`/`?` × containers, thin-coverage collections, FFI × generics, 3-way affine × generics × containers, pattern-match depth, boundary confirmations). ~15 bugs, nearly all the same root shape: a monomorphization/codegen "find every use of X" walker covering only some of the AST's shapes. |
| Localfuzz-found + bug-pattern-audit rounds (BUG-105–178) | Multiple audit rounds (round 2 through round 11) plus ongoing `tools/localfuzz` triage. Highlights: BUG-110 (both SSA backends silently emitted fully unchecked arithmetic — the single most impactful fix of this window), BUG-116 (SSA path never lowered `requires` at all), a systematic ASan/LeakSanitizer sweep (BUG-153/154 onward) that found a real leak/UAF class recurring at 5 independent call sites (closures, struct-field assignment, bare call args, `hashmap_*`, `Trie` ops) — CI gained a permanent ASan/LSan/UBSan corpus-sweep job as a result. BUG-166/167 found and fixed a real SMT-soundness bug (the identifier sanitizer aliased distinct non-ASCII variable names into one SMT symbol). BUG-169–172 were large localization sweeps: structure-keyword parity across all 60+ dialect functions, then a native-speaker linguistic review across ~40 dialects that also found a widespread `vec(0)`-is-not-empty bug in 46 example files. BUG-176 shipped a real language feature (positional `break inner`/`outer`/`middle`). |
| **v0.9.1** (2026-08-07) | Consolidated ~70 bug fixes (BUG-68–140) across 5 sweeps into one patch release — the version had been sitting at `0.9.1-dev` unreleased while that work accumulated. |
| **v0.9.2** (2026-08-11) | ~38 more bug fixes (BUG-141–178) + positional break/continue. Shipped with the release-notes/changelog scaffolding still unfilled (`TODO` placeholders) — caught and fixed retroactively on 2026-08-12, see below. |
| SMT bounds-elision soundness series (BUG-181/182/183) | Found via localfuzz, same root theme across 3 separate bugs: `checker.rs`'s `smt_facts` (a running "facts assumed true here" list) going stale across a loop or branch boundary and letting the bounds-check elision pass "prove" an out-of-range index in-bounds. BUG-181 was the worst in kind — an unconditional, always-reachable out-of-bounds memory access (SIGSEGV on the C backend). Fixed by disabling that specific elision inside loop bodies (a permanent, documented trade-off — `docs/v1_limitations.md` L26) and by dropping stale facts at 5 more merge points (`if`/`else`, `if let`, `while let`, `select`) BUG-183's targeted follow-up audit found. |
| **v0.9.3** (2026-08-12) | BUG-179–184 (the SMT series above, two tutorial-writing-time gaps, and a `--big-o` classification bug found while adding its own regression coverage). `scripts/release.py` gained a hard guard (`check_notes_not_stale`) refusing to tag a release while `RELEASE_NOTES`/`CHANGELOG.md` still hold unfilled scaffold placeholders — the exact gap that let v0.9.2 ship stale. |
| Tutorial sequential-readability audit | Verified the tutorials' "no CS background needed" claim by reading every chapter in the book's own `SUMMARY.md` order, tracking whether any concept is used before it's taught. Found and fixed real forward-references (struct/enum/interface/Vec used ahead of their own chapters) in 7 files, plus a separately-discovered, repo-wide false "this chapter has no compiler code" claim in 25 more primer chapters. |
| `docs/language_manual.md` accuracy audit | Found real drift against actual compiler behavior: 6 wrong/nonexistent builtin names in the Collections table (`vec_push`/`vec_len` don't exist), a Concurrency example using an invalid type + syntax, a Mutex example using pointer-deref syntax that doesn't exist in vāṇी, and an SMT example with 3 compile errors. Every corrected sample re-verified against the real compiler. |
| `tools/vani_translate.py` fully fixed | Found broken for ~24% of its claimed dialect coverage (untranslated keywords silently producing non-compiling output) plus 6 dialects missing from `--to` entirely — same "hand-copied table drifted from `lexer.rs`" shape as BUG-173's `src/lsp.rs` staleness. New `tools/regen_vani_translate_keywords.py` (fixed 42 wrong + 523 entirely-missing table cells + added the 6 missing dialects) and `tools/test_vani_translate.py` (permanent regression suite, now a CI job) close it for good. |
| Retroactive doc/release fixes (2026-08-12) | v0.9.2's `CHANGELOG.md`/`RELEASE_NOTES/v0.9.2.md` entries backfilled with real content (were shipped with `TODO` placeholders); the already-published GitHub release page for v0.9.2 also needed a separate `gh release edit` since it doesn't auto-sync with repo file changes. This `STATUS.md` entry, `TODO.md`'s "Current status" section, and `RELEASING.md`'s version-history gap (stopped at `v0.5.0`) were all found stale by the same pass and are being brought current alongside it. |

### Key numbers (2026-08-12)
- **Compiler version**: `0.9.4-dev` (last tagged release: `v0.9.3`, 2026-08-12)
- **Bugs fixed this window**: ~104 (BUG-81 through BUG-184)
- **Releases cut this window**: 3 (`v0.9.1`, `v0.9.2`, `v0.9.3`)
- **`cargo test`**: 2906 lib tests + 12 other suites, 0 failures (includes 2 new CI jobs added this window: the ASan/LSan/UBSan corpus sweep and the `vani_translate.py` regression suite)
- **`vanic check` example corpus**: 1022/1040 ok (18 non-ok: 17 known pre-existing xfail/embedded-gated files + 1 deliberately-malformed lex fixture)

---

## 📋 NEXT SESSION HANDOFF — 2026-08-02 (testing-matrix sweep: 13 bugs found+fixed, BUG-68–BUG-80)

**State**: completed a full, systematic sweep of `docs/TESTING_MATRIX_TODO.md`'s
"container operations x intermediate/advanced feature nesting" matrix — 19
rows covering SMT contracts, generics, enum/pattern matching, FFI, affine
ownership, Big-O, `parallel for`/SIMD, and `Option`/`Result`, each crossed
with a container type (`Vec`, `Array`, `Tuple`, generic struct). Every row
got a real `.vani` snippet run through both `--backend` values, checked
against hand-computed values. **13 real bugs found and fixed** (BUG-68
through BUG-80); 6 rows confirmed already-correct or a real, consistently-
enforced v1 limitation. Every row (bug or not) got a permanent
`src/lib.rs` compile-time test plus a `tests/run_end_to_end.rs` real-
binary test on both backends. 10 commits, each pushed and confirmed
CI-green before starting the next. No version bump this session (all
patch-level fixes; compiler version stays `0.9.1-dev`). Full technical
writeups: `docs/TODO_CURRENT.md` (search "BUG-68" through "BUG-80").
Per-row sweep notes + closing summary: `docs/TESTING_MATRIX_TODO.md`.

### Shipped this session (2026-08-02)

| Item | What shipped |
|------|-------------|
| **BUG-68** — SMT `ensures` silent-accept | `verify_ensures_at_return` treated ANY `ensures` clause the SMT encoder couldn't fully encode as silently PROVEN (empty non-`Proven` match arm — a "fall back to constant-true check" comment that was never implemented). A deliberately false `ensures` clause over a `ref` struct parameter's field compiled clean. Fixed by (1) generalizing struct-field-to-SMT-var modeling to ANY struct-typed binding (not just literal-init locals), making `ref` struct param field access in contracts genuinely verifiable, and (2) making `verify_ensures_at_return` push a real diagnostic on the unproven verdicts, mirroring the already-correct loop-invariant path. Re-running the now-real solver against a pre-existing example caught an actual latent overflow bug in its own contract. Same push: `walk_for_reassigns` didn't handle `Stmt::FieldAssign`, breaking loop-invariant preservation for struct-field accumulators mutated via `acc.field = ...;`. |
| **BUG-69** — `vec_fill` after `if` crashes LLVM | `TypedStmt::If`'s LLVM emitter never updated `ctx.current_block` after the if — the one builtin (`vec_fill`) that reads it for its own PHI predecessor got wired to a stale block whenever called after any prior `if` in the same function. "PHI node entries do not match predecessors!" |
| **BUG-70** — generic struct construction breaks at 2+ instantiations | Same root cause and fix shape as the earlier BUG-46 (which only covered enum-variant construction): `Env::resolve_struct_name`'s "exactly one candidate" fallback can't disambiguate a bare `StructLit` once 2+ instantiations of the same generic struct exist. Fixed via `resolve_bare_struct_lits_in_stmt`, mirroring `resolve_bare_enum_ctors_in_stmt`. |
| **BUG-71** — generic inference through `ref Vec<T>` | Bound T to the whole Vec instead of the element, for ANY T (confirmed with a scalar-only repro first — not container/generics-angle specific). `infer_concrete_type_for_call` never re-wrapped a `ref` argument's resolved type in `Type::Ref` before structural unification. |
| **BUG-72** — Tuple generic specialization crashes LLVM | `type_mangle`'s Debug-based fallback didn't escape `[`/`]` from `Type::Tuple`'s derived-Debug repr, producing an invalid LLVM identifier. |
| **BUG-73** — generic struct construction inside `vec(...)` | BUG-70's fix only handled a bare `StructLit` as a `let`'s top-level RHS, not nested inside a `vec(...)` call's args — the natural way to write "a Vec of a generic struct." |
| **BUG-74** — enum payload `Tuple<Array>` | Checker admission gate too conservative (`Type::Array::is_copy()` is unconditionally `false` by design, poisoning `Tuple`'s element-wise Copy check) + two C-backend codegen gaps (array-typedef emission ordering, and an invalid C array-assignment initializer). |
| **BUG-75** — `clone_at` on mixed-payload-type enum | Silently corrupted every SCALAR payload on LLVM (`Num(7)` cloned as `Num(0)`) — wrong OwnedStr-tag detection (picked the FIRST payload type, not "does any variant have OwnedStr"), then an LLVM type mismatch once that was fixed (mixed-payload enums store the payload as an opaque `[N x i8]` byte buffer, not `i8*`). Real backend divergence — C was correct throughout. |
| **BUG-76** — `Option<UserEnum>.None` crashes LLVM | Payload-less variant's zero-value placeholder match was missing a `Type::Enum(_)` arm — same class as the earlier BUG-29 (`Str`) / BUG-35 (`Box<T>`). |
| **BUG-77** — `extern "C" fn` returning a struct by value | Crashed LLVM at the call site — the System V x86-64 ABI-lowering had the param-passing-side "lower before the call" step but no return-side "un-lower after the call" mirror. Found by actually calling a real linked C function, not just declaring one (the pre-existing test suite only ever compiled a struct-returning declaration). |
| **BUG-78** — `Array<Tuple/Struct,N>` function parameters | Crashed the C backend — `format_declarator`'s `Type::Array` arm used the wrong leaf-only type-spelling helper (`c_leaf_type` instead of `c_element_storage`), leaking a placeholder comment into the declarator. |
| **BUG-79** — `vec128`/`vec256`/`vec512` struct fields | Crashed the C backend — `c_element_storage` simply never had arms for these three types (unlike Closure/Channel/Mutex, which got this exact fix in earlier sessions). |
| **BUG-80** — `Option<Array<T,N>>` | Crashed the C backend — match-arm payload-binding codegen used the wrong type spelling, then (once fixed) hit "C arrays can't be copy-assigned via `=`" (fixed via array-decay-to-pointer), plus a typedef-emission gap for enum payloads that are directly `Array<T,N>` (not nested in a Tuple, which an earlier fix in this same sweep already covered). |
| Doc sync | `CHANGELOG.md` gained a `v0.9.1` section for this batch; tutorials updated where a page demonstrated (or should now demonstrate) a capability this sweep verified for the first time — see the Documentation section of the changelog entry for the exact list. |

### Key numbers (2026-08-02)
- **Compiler version**: `0.9.1-dev` (no bump this session — patch-level fixes only)
- **Commits this session**: 10, each pushed individually and confirmed CI-green (`05031fd`, `8e905e4`, `2dd59e3`, `f9f2df5`, `87d9fa8`, `d542bc2`, `ebd2cd4`, `8f96a2f`, `73643fd`, `cab1803`)
- **Bugs found**: 13 (BUG-68 through BUG-80); **rows confirmed clean**: 6 of 19
- **New regression tests**: ~30 `src/lib.rs` compile-time tests + ~17 `tests/run_end_to_end.rs` real-binary tests (both backends), one per sweep row
- **`cargo test --release --workspace`**: run in full at every batch checkpoint this session (not just spot-checked) — 13/13 test binaries clean, 0 failures, every time

---

## 📋 NEXT SESSION HANDOFF — 2026-07-22 (Kosh audit-safety gate + v0.6.0 release + Kosh namespacing arc)

**State**: the single biggest-scope session in this log. Four largely-independent
deliverables landed back to back: (1) `vanic audit-safety` + a `vanic publish`
coverage gate, (2) a v0.6.0 feature release (14 language features + perf work
already queued from prior sessions, cut and shipped this session) plus a real
bug fix to the release-asset pipeline, (3) a tutorial mojibake fix, and (4) a
full 6-phase Kosh package **namespacing** arc — the largest single piece of
work — ending with every affected published kosh package migrated and
republished. Compiler version `v0.6.1-dev` (Cargo.toml); last tagged release
`v0.6.0`.

### Shipped this session (2026-07-21 -> 2026-07-22)

| Item | What shipped |
|------|-------------|
| **GATE-1** `vanic audit-safety` | New CLI command + `vanic publish` hard gate: verifies `#[bounded_stack]`/`#[wcet]` coverage wherever a function is *eligible* (not blanket 100%), reusing existing `wcet_body`/`compute_stack_depths`. `--allow-partial-safety-coverage` escape hatch. Needed `compile_library`/`compile_library_path` (checker::check_library) since packages have no `fn main()`. Found 4 real coverage gaps across published packages on first run; fixed + republished. |
| v0.6.0 release | Cut via `scripts/release.py --minor`; wrote real `RELEASE_NOTES/v0.6.0.md` + `CHANGELOG.md` entries by hand (not the auto-stub). 14 language/tooling features (generic trait bounds, slice patterns, `#[repr(C/packed)]`, async `select`, overflow guards, affine closures, `Vec<bool>`, `vanic test`, `for await`, multi-pass monomorphization, `Atomic<f64>`, `file_open` buffered arg, the audit-safety gate itself) + 5 perf wins (pdqsort, sort pattern detection, AVX-512 bitmask scan, persistent pthread pool, `getelementptr inbounds`). |
| Release workflow bug (found + fixed) | Every past tagged release (verified back to v0.4.0) shipped with **zero binary assets** despite reporting success — `actions/download-artifact@v4` nested each artifact into a same-named subdirectory, and the "flatten" step's `mv` moved each file back into its own parent (self-referential no-op). Fixed with `merge-multiple: true`; backfilled v0.6.0's assets manually. Added a pruning step: only the 3 most recent releases keep their binaries (release pages/notes untouched). |
| Tutorial mojibake fix | `08b_errors_primer.md` had "vāṇी" double-encoded (UTF-8 read as Windows-1252) into `vÄá¹‡Ä«` — same failure class as an earlier `parser.rs` mojibake bug. Fixed both occurrences + a stray BOM. |
| SIMD/NEON doc parity sweep | `vec512<T>` (shipped v0.5.0) had never been added to `docs/arm_neon_status.md` or `docs/simd_ffi_shims.md` (both stopped at vec128/vec256). No compiler code changes needed — vec512 already uses the same architecture-generic LLVM lowering. Also fixed a README.md inaccuracy (`simd128_add` isn't a real builtin; it's `simd_add`). |
| **Kosh namespacing arc, all 6 phases** ✅ | Full design + verification in `docs/kosh_namespacing_design.md`. Triggered by a user question ("what happens if a kosh package has the same function name as a builtin?") that led to hands-on testing and uncovered a second bug (diamond dependencies silently produced missing-function errors). NS-1: real transitive dependency graph, `(name,version)`-deduped. NS-2: circular-dependency detection (reused `vanic acyclicity`'s Tarjan SCC). NS-3: automatic per-package namespacing (`pkgname::item`) — the actual fix for the original question; found and fixed a real parser gap (module bodies had no `#[attr]` item support) along the way. NS-4: `vani.lock` records the full transitive graph. NS-5: "did you mean `pkgname::item`?" migration diagnostic; also fixed `vanic add` writing invalid (hyphenated) `[deps]` keys by default. NS-6: migrated + republished all 8 affected packages (`vectorcalc` 0.1.3, `algebra` 0.1.3, `pde` 0.1.3, `interval` 0.1.3, `tensor` 0.1.3, `signal` 0.1.3, `optimize` 0.1.4, `probability` 0.4.6); fixed a real `probability`/`optimize` `matrix`-version-drift bug NS-1 surfaced along the way. Final proof: a fresh project depending on the real, republished `probability`+`optimize` (the exact diamond that started this) compiles clean with zero conflicts. |
| `pub(kosh)` non-enforcement documented as **L23** | Found while correcting namespacing docs: `pub(kosh)` has never been enforced (verified: zero reads of the AST's `_kosh_only` bit anywhere in `checker.rs`; a `pub(kosh)` fn is callable from completely outside its module with no error). Predates this session; not introduced by the namespacing arc. Added to `docs/v1_limitations.md`, corrected in `tutorials/src/beginner/09a_modules_primer.md` (which had a fabricated "REJECTED" worked example) and `docs/missing_features.md`. |
| Misc doc fixes | `docs/missing_features.md`'s "Kosh package manager... queued, pending registry-hosting decision" was stale (live all session with 12+ published packages) -- fixed. README.md nav gained a Kosh Package Manager link. |

### Key numbers (2026-07-22)
- **Compiler version**: `v0.6.1-dev` (last tagged release: `v0.6.0`)
- **Commits this session**: ~20 across `vani-compiler`, plus 8 kosh package repos + `kosh-index`
- **`#[test]` functions in `src/lib.rs`**: 2378 (static count; full suite not re-run this session per standing "spot tests only" preference — spot-checked extensively instead: every touched package's `vanic audit-safety`, every test/example file via `vanic check`, and at least one real `vanic run` per package before publishing)
- **Kosh ecosystem**: 12/12 published packages pass `vanic audit-safety` cleanly; 8/12 required migration to qualified `pkgname::item` syntax

---

## 📋 NEXT SESSION HANDOFF — 2026-07-21 (file_open buffered arg + Big-O/device-I/O doc audit)

**State**: `file_open` gained a required third `buffered: bool` argument (breaking arity change) with a working unbuffered path on both backends; a stale Big-O doc comment was fixed; device-I/O docs extended past UART-only; one new bug discovered (not fixed) on the LLVM backend. Compiler version `v0.5.4-dev` (Cargo.toml).

### Shipped this session (2026-07-21)

| Item | What shipped |
|------|-------------|
| DOC-1 | Fixed stale `big_o.rs` module comment claiming no cross-fn analysis — `annotate_program` (what the CLI actually calls) already does it; confirmed via direct test (loop calling an O(n) helper correctly reports O(n²)) |
| DOC-2 | `docs/v1_limitations.md` L18 extended with I2C/SPI worked FFI-shim examples (previously UART-only) + PCIe/NVMe clarification (same FFI/MMIO pattern, no protocol-specific surface planned) |
| DOC-3 / kosh_design.md | Documented that `[deps]`-declared packages don't need an explicit `use "path";` — `compile_path`/`resolve_uses` already auto-include them via the manifest |
| **IO-1** | `file_open(path, mode, buffered: bool)` — see `docs/v1_limitations.md` L18 for the full writeup. C backend: new `intent_file_open` helper. LLVM backend: inlined `@fopen` + conditional `@setvbuf` branch (deliberately not a custom linked symbol). 5 lib tests (2 new), example + 2 tutorials updated. Verified end-to-end on both backends, both `run` and `build`. |
| BUG-1 (found, not fixed) | `file_read_line`/`stdin_read_line` completely broken on the LLVM backend (both `run` and `build`) — `@intent_file_read_line` has no `declare` or definition anywhere reachable from that path. `--backend=c` unaffected. Tracked in `docs/TODO_CURRENT.md`. |

### Key numbers (2026-07-21)
- **Compiler version**: `v0.5.4-dev`
- **Commits this session**: 3 (`41cca6d`, `c5695d1`, `0e236c8`) + IO-1 (pending commit)

---

## 📋 NEXT SESSION HANDOFF — 2026-07-13 (Vec<f64> parity + #[no_nan] + benchmark 12)

**State**: Vec<f64> builtin parity complete (F64-2–F64-5), #[no_nan] safety attribute added (T2.4), benchmark 12 SIMD-256 f32 dot product running, pub(kosh) tutorial example added. Released as **v0.4.1**. Compiler version `v0.4.1` (Cargo.toml).

### Shipped this session (2026-07-13)

| Item | What shipped |
|------|-------------|
| Vec<f64> stats builtins (F64-2) | `vec_sum/mean/min/max/argmin/argmax/median/kth_smallest` accept `Vec<f64>`; C + LLVM backends emit `double` helpers |
| Vec<f64> combinators (F64-3) | `vec_fold/map/filter` accept `Vec<f64>`; mapper/combiner/predicate types updated |
| `vec_swap` on Vec<f64> (F64-4) | `check_vec_swap_builtin` opened for F64; C: `double tmp`; LLVM: parametric |
| `vec_dot` on Vec<f64> (F64-5) | Returns `f64`; C: `double` accumulator; LLVM: `fmul`/`fadd` loop |
| `#[no_nan]` safety attribute (T2.4) | Rejects `f64_nan()` and `vec_kth_smallest<f64>`; implied by asil_d/do178c_level_a/sil3/sil4; 6 lib tests |
| Benchmark 12 (SIMD-256 f32 dot) | `vec256<f32>`, `vec128<f32>`, `Vec<f32>`, all `simd256_*`/`simd_*` builtins confirmed; results on i5-1035G1 documented |
| Tutorial: pub(kosh) example | All three visibility tiers shown side-by-side in Beginner 9a |
| Released v0.4.1 | CHANGELOG stamped, Cargo.toml bumped, tag pushed, GitHub release created |

### Key numbers (2026-07-13)
- **Lib tests**: 2466+ passing
- **Compiler version**: `v0.4.1`
- **Commits this session**: 8

---

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
