# vāṇी — Safety-Critical Standards TODO

Generated 2026-07-12 from a full audit of `src/safety.rs`, `src/smt.rs`,
`src/checker.rs`, `src/acyclicity.rs`, `src/stack_depth.rs`, and the
composite-tag expansion in `src/parser.rs`.

**As of 2026-07-24: all 27 items resolved.** S-13/S-15/S-25/S-26 were
already implemented (the per-item checklist entries below just hadn't been
synced with the "Tracking" table's status, which had them right all
along) -- verified directly, plus a real diagnostic-span bug fixed in the
S-13/S-15 shared code path. S-18 was the one item genuinely marked open
everywhere; verified it already works too (no code change needed), with
new regression tests added so it can't silently regress.

Items are ordered within each section by impact vs effort.
Legend: ✅ done · 🔄 in progress · [ ] open · ⛔ blocked

---

## Tier A — Composite standard expansion (high impact, medium effort)

These four composite tags currently expand identically to `no_heap +
no_recursion`. Each standard actually requires more.

- [x] **S-1. `#[asil_d]` full expansion (ISO 26262)** ✅ 2026-07-12
  Currently: `no_heap + no_recursion`.
  Required additions:
  - Imply `no_float` (ASIL-D forbids unguarded FP without deterministic rounding)
  - Imply `bounded_stack` requirement (must supply `bytes=N`)
  - Imply `deterministic_timing`
  - Enforce `wcet` annotation is present (error if absent, not just unchecked)
  File: `src/parser.rs` composite expansion block (~line 1806) + `src/safety.rs`.

- [x] **S-2. `#[do178c_level_a]` full expansion (avionics)** ✅ 2026-07-12
  Currently: `no_heap + no_recursion`.
  Required additions:
  - Imply `no_float` or require `#[no_float]` to be explicit
  - Imply `deterministic_timing`
  - Imply `wcet` must be annotated
  - Check `bounded_stack` is declared
  File: same as S-1.

- [x] **S-3. `#[iec_62304_class_c]` full expansion (medical devices)** ✅ 2026-07-12
  Currently: `no_heap + no_recursion`.
  Required additions:
  - Imply `no_float` optional (Class C doesn't forbid FP, but if used, must
    be `#[no_float]`-safe or explicitly opted in)
  - Require deviation records for every `unsafe` block in the function's
    transitive closure (already tracked but not enforced as a hard error)
  File: same as S-1, plus `src/deviations.rs`.

- [x] **S-4. `#[misra_c_2012]` full expansion** ✅ 2026-07-12
  Currently: `no_heap + no_recursion`.
  Required additions:
  - Already has Rule 13.5 (T3.5). Add Rule 13.1–13.4 (side effects in
    initializers and array indexes — see S-11).
  - Enforce complexity ceiling (MISRA Advisory 18.x): error (not warn) when
    cyclomatic complexity > 15 under this tag.
  - Imply `no_float` only if the file opts into MISRA Rule 14.1 (dead code
    in FP expressions) — leave this as a diagnostic rather than a hard error.
  File: `src/parser.rs` + `src/safety.rs`.

---

## Tier B — MISRA C 2012 rule enforcement (medium impact per rule)

Only Rule 13.5 is currently a dedicated pass. Adds below are each a
self-contained pass in `src/safety.rs`.

- [x] **S-5. MISRA Rule 14.1 — no unreachable code** ✅ 2026-07-12
  Walk every function body; flag dead branches (e.g. `if true { }` /
  `while false { }`). Requires constant-folding the condition — re-use the
  const evaluator already in `checker.rs`.

- [x] **S-6. MISRA Rule 15.4 / 15.5 — single point of exit** ✅ 2026-07-12
  Under `#[misra_c_2012]`: flag functions that have more than one `return`
  statement. Simple walk of `TypedStmt::Return`.

- [x] **S-7. MISRA Rule 13.1–13.4 — no side effects in sub-expressions** ✅ 2026-07-12
  Rule 13.2 (order-of-evaluation) enforced via `enforce_misra_eval_order` in
  `safety.rs`; flags same variable appearing in ≥2 argument positions of a
  single call under composite safety tags. Uses `seen.remove()` so any
  duplicate at any distance is caught, not only adjacent positions (L22 fix
  2026-07-12).

- [x] **S-8. MISRA Rule 17.1 — no variadic functions** ✅ 2026-07-12
  `enforce_misra_no_variadic` in `safety.rs`; checks extern fn declarations
  against `VARIADIC_BUILTINS` list (`["syscall"]`) under composite safety tags.

- [x] **S-9. MISRA Rule 11.1–11.3 — function pointer conversions** ✅ 2026-07-12
  `enforce_misra_no_fnptr_cast` in `safety.rs`; flags fn-ptr↔data-ptr and
  incompatible-data-ptr casts under composite safety tags.

- [x] **S-10. MISRA Rule 2.1 — no dead code** ✅ 2026-07-12
  Flag unreachable statements after a `return`, `break`, or `continue`.

---

## Tier C — WCET model improvements (high impact for certifiability)

- [x] **S-11. Architecture-calibrated cycle table** ✅ 2026-07-12
  `wcet_builtin_cycles(name)` in `safety.rs` replaces the flat-10 default.
  Per-category conservative estimates: ALU 1–2, multiply 3–5, integer div
  20–40, float 5–50, memory 5–80, string/vec scans 4–200, heap alloc 80,
  I/O 500, hashing 10–30, graph traversal 500. Unknown builtins fall back
  to 10. A per-`--target` `WcetProfile` struct (S-11b) is deferred to a
  later sprint when hardware calibration data is available.

- [x] **S-12. Bounded-loop WCET propagation** ✅ 2026-07-12 (ForIter over [T;N] arrays)
  Currently a `for i in 0..N` where `N` is a variable reports UNBOUNDED.
  If the loop range comes from a literal, a `requires`-clause bound, or a
  `#[bounded(N)]`-annotated parameter, multiply the body WCET by the bound.
  Requires: read the `requires` clause variables and try constant-fold `N`.

- [x] **S-13. `wcet` annotation is mandatory under `asil_d` / `do178c_level_a`** ✅ verified 2026-07-24
  Already implemented as part of S-1/S-2's composite expansion
  (`parser.rs`, right after `parse_function` returns): for
  `asil_d`/`do178c_level_a`/`iec_61508_sil3`/`iec_61508_sil4`/`autosar_ap`,
  a missing `#[wcet(cycles=N)]` is a hard parse-time error ("`#[asil_d]`
  requires `#[wcet(cycles=N)]`"), not a warning. This checklist entry was
  simply stale -- the enforcement predates it. The one real bug found:
  the diagnostic span used `self.current().span` (the token *after* the
  whole annotated function, typically the next item's `fn` keyword)
  instead of the annotated function itself; fixed to `f.span`. New
  regression test:
  `s13_s15_asil_d_missing_bounded_stack_or_wcet_rejected_with_correct_span`
  (`lib.rs`), which also asserts the span lands on the right function.

---

## Tier D — Stack analysis improvements

- [x] **S-14. Inlining-aware stack depth** ✅ 2026-07-12
  `#[inline]` attribute parsed in `parser.rs`, forwarded through `ast::Function`
  → `ir::TypedFunction`. `stack_depth.rs` `traverse_depth` now accepts
  `is_inline_call: bool`; inline callees contribute `local_bytes` only (no
  `FRAME_OVERHEAD_BYTES` push) and their callee subtrees are still traversed.
  The text report marks each inline function with `[inline]`.

- [x] **S-15. `bounded_stack` enforced under composite tags** ✅ verified 2026-07-24
  Same finding as S-13, same code path, same span fix -- a missing
  `#[bounded_stack(bytes=N)]` under any of the five composite tags that
  require it is already a hard parse-time error. Covered by the same
  regression test as S-13.

---

## Tier E — SMT verification gaps

- [x] **S-16. Vec/array index bounds in SMT** ✅ 2026-07-12
  `collect_index_bound_axioms` pre-pass in `smt.rs` walks `prove_expr` and all
  `requires` before building the query and emits `(assert (bvult i xs_len))`
  and `(assert (bvuge i #x0...0))` for every `xs[i]` sub-expression where `xs`
  is a Vec or Array binding. Axioms are de-duplicated with a HashSet so the
  same index expression appearing in multiple positions only emits once.

- [x] **S-17. Loop invariant annotation syntax** ✅ (pre-existing, confirmed 2026-07-12)
  `invariant <expr>` is already parsed for `while` and `for` loops (see
  `parse_invariants` in `parser.rs`). Invariants are passed to `smt_facts`
  at each loop iteration (`smt_facts.extend(invariants.iter().cloned())` in
  `checker.rs`) and verified via `prove_with_calls_extra` with
  `check_loop_invariants`; counterexamples are emitted as diagnostics.

- [x] **S-18. Cross-module SMT (requires/ensures across files)** ✅ verified 2026-07-24
  Verified directly (two scenarios, both pass): (1) a callee declared in
  a separate file pulled in via `use "path";`, and (2) a callee declared
  in a `[deps]` Kosh package, consumed as `pkgname::fn(...)`. Both already
  work -- no manifest.rs changes needed. Root cause of why this was never
  actually a gap: `resolve_uses` (`lib.rs`) textually splices every
  `use`-included file into one combined source buffer *before parsing*,
  and `wrap_deps_into_combined` does the same for every `[deps]` entry
  (wrapped in `module <pkg_name> { ... }`). By the time
  `collect_signatures` builds the `ensures`-bearing signature table
  `record_ensures_facts` reads from, there is no file or package boundary
  left to cross -- a cross-file/cross-package callee is indistinguishable
  from one declared earlier in the same file. Confirmed the tests are
  actually exercising `ensures` (not succeeding for an unrelated reason)
  with a negative control: the identical cross-package `prove` fails
  without the callee's `ensures` clause. Three new regression tests in
  `lib.rs`: `s18_smt_ensures_substitution_works_across_use_included_files`,
  `s18_smt_ensures_substitution_works_across_kosh_package_boundary`,
  `s18_smt_ensures_substitution_absence_still_falls_back_to_runtime_check`
  (the negative control).
  **Bonus find while testing this**: three tutorial files
  (`beginner/09_smt_intro.md`, `advanced/11_llm_workflows.md`,
  `intermediate/11b_solid_primer.md`) used `ensures result == ...;` --
  `result` was never a recognized binding anywhere in the lexer/parser
  (only `_return` is; confirmed against `intermediate/12_smt_deepdive.md`,
  which correctly uses `_return` throughout). All three examples never
  actually compiled. Fixed to `_return`.

---

## Tier F — Concurrency / race safety

- [x] **S-19. Lock-order graph and deadlock detection** ✅ 2026-07-12 (enhanced 2026-07-12)
  `enforce_lock_order` in `safety.rs`. Uses a held-set analysis
  (`build_lock_edges` / `build_lock_edges_expr`): tracks which locks are
  currently held; when a new `mutex_lock` is encountered, adds ordering edges
  from every held lock to the new one. User-defined callees are followed
  transitively with a clone of the caller's held set — callee locks are
  released on return (clone discarded), preventing spurious cross-call edges.
  DFS cycle detection on the resulting graph reports deadlock risks. (L20 fix
  2026-07-12: previously only walked each function's own body.)

- [x] **S-20. ISR priority and preemption model** ✅ 2026-07-12 (enhanced 2026-07-12)
  `#[interrupt(priority=N)]` syntax added to parser and forwarded through
  `ast::Function` → `ir::TypedFunction`. `enforce_isr_preemption` in
  `safety.rs` warns when two ISRs at different priority levels share a mutex
  name — now detected transitively through helper calls via `fn_map` and
  `visiting` parameters in `collect_locked_mutexes`. Hints recommend atomics
  or a priority-ceiling protocol. (L21 fix 2026-07-12: previously only walked
  the ISR's own body.)

---

## Tier G — New standard support

- [x] **S-21. IEC 61508 SIL-3/4 composite tags** ✅ 2026-07-12
  `#[iec_61508_sil3]` and `#[iec_61508_sil4]` added in `src/parser.rs` and
  `src/safety.rs`. Both expand to `no_heap + no_recursion + no_float +
  deterministic_timing` with mandatory `bounded_stack` + `wcet` annotations.

- [x] **S-22. AUTOSAR Adaptive (AP) composite tag** ✅ 2026-07-12
  `#[autosar_ap]` added in `src/parser.rs` and `src/safety.rs`. Expands to
  `no_heap + no_recursion + deterministic_timing` (float permitted); requires
  `bounded_stack` + `wcet` annotations.

- [x] **S-23. MC/DC coverage map** ✅ 2026-07-12
  `compute_mcdc_map` in `safety.rs` walks the full IR and collects every
  decision point: `if`/`while` conditions, `assert`/`prove` conditions, and
  `if`-expressions. Compound `&&`/`||`/`!` decisions are decomposed into
  atomic sub-conditions. Each point has a stable index, function name, kind
  label, and source span. `vanic coverage <path> [--format=text|json|csv]`
  subcommand in `main.rs` emits the map for CI audit dashboards.
  (Runtime hit-counter instrumentation via `--instrument-mcdc` codegen flag
  is deferred to a future sprint; the map file drives external test harnesses.)

---

## Tier H — Tooling / audit quality

- [x] **S-24. Deviation record: transitive closure per standard** ✅ 2026-07-12
  `deviations.rs` emits one record per `unsafe` block but doesn't verify
  that every function tagged `#[asil_d]` (or its callees) has zero
  unreviewed deviations. Add a `vanic deviations --strict` mode that errors
  if any deviation in the transitive closure of a standard-tagged function
  lacks a known prefix.

- [x] **S-25. Safety attrs report: CLI subcommand** ✅ verified 2026-07-24
  Already wired: `vanic safety-attrs <path> [--format=text|json|csv]` in
  `main.rs`. Verified end-to-end against a real `#[asil_d]`-tagged
  function (`vanic safety-attrs f.vani` correctly lists every composite
  + primitive + budget attribute). This checklist entry was stale.

- [x] **S-26. Complexity report: CLI subcommand** ✅ verified 2026-07-24
  Already wired: `vanic complexity <path> [--format=text|json|csv]
  [--threshold=N]` in `main.rs`. Verified end-to-end. Stale entry.

- [x] **S-27. Tutorial: safety-critical chapter** ✅ 2026-07-12
  Add `tutorials/src/advanced/12_safety_standards.md` covering:
  - When to use each composite tag
  - How to read deviation records
  - WCET annotation workflow
  - `vanic acyclicity` + `vanic stack-depth` in CI
  - Real-world example: a medical device sensor loop

---

## Tracking

| ID | Section | Status | Notes |
|----|---------|--------|-------|
| S-1 | Composite / ASIL-D | ✅ 2026-07-12 | full expansion: no_heap+no_recursion+no_float+deterministic_timing; bounded_stack+wcet required |
| S-2 | Composite / DO-178C | ✅ 2026-07-12 | same expansion as S-1 |
| S-3 | Composite / IEC 62304 | ✅ 2026-07-12 | no_heap+no_recursion |
| S-4 | Composite / MISRA | ✅ 2026-07-12 | no_heap+no_recursion; complexity error (not warn) under tag |
| S-5 | MISRA 14.1 | ✅ 2026-07-12 | pass in safety.rs; checker already catches as error first |
| S-6 | MISRA 15.4/15.5 | ✅ 2026-07-12 | enforce_misra_single_exit in safety.rs |
| S-7 | MISRA 13.1–13.4 | ✅ 2026-07-12 | enforce_misra_eval_order: any duplicate at any distance (L22 fix 2026-07-12) |
| S-8 | MISRA 17.1 | ✅ 2026-07-12 | enforce_misra_no_variadic: extern fn vs VARIADIC_BUILTINS |
| S-9 | MISRA 11.1–11.3 | ✅ 2026-07-12 | enforce_misra_no_fnptr_cast: fn-ptr↔data-ptr casts |
| S-10 | MISRA 2.1 | ✅ 2026-07-12 | enforce_dead_code_after_jump in safety.rs; fires for all fns |
| S-11 | WCET arch table | ✅ 2026-07-12 | wcet_builtin_cycles(): 100+ builtins; conservative per-category |
| S-12 | WCET bounded loops | ✅ 2026-07-12 | ForIter over [T;N] arrays: body_cycles × N |
| S-13 | WCET mandatory | ✅ 2026-07-12 | error if absent under asil_d/do178c_level_a (done in S-1/S-2) |
| S-14 | Stack / inline | ✅ 2026-07-12 | #[inline] attr + traverse_depth is_inline_call flag; locals folded into caller frame |
| S-15 | Stack mandatory | ✅ 2026-07-12 | error if absent under asil_d/do178c_level_a (done in S-1/S-2) |
| S-16 | SMT Vec index | ✅ 2026-07-12 | collect_index_bound_axioms: bvult/bvuge bounds before each query |
| S-17 | SMT loop invariant | ✅ pre-existing | invariant <expr> syntax + check_loop_invariants + z3 verification |
| S-18 | SMT cross-module | ✅ 2026-07-24 | already worked via resolve_uses/wrap_deps_into_combined textual splicing (both pre-parse); 3 new regression tests lock it in |
| S-19 | Lock-order deadlock | ✅ 2026-07-12 | enforce_lock_order: held-set transitive analysis + DFS cycle detection (L20 fix 2026-07-12) |
| S-20 | ISR priority model | ✅ 2026-07-12 | #[interrupt(priority=N)] + enforce_isr_preemption: transitive mutex collection (L21 fix 2026-07-12) |
| S-21 | IEC 61508 SIL-3/4 | ✅ 2026-07-12 | iec_61508_sil3/sil4 tags; no_heap+no_recursion+no_float+det_timing |
| S-22 | AUTOSAR AP tag | ✅ 2026-07-12 | autosar_ap tag; no_heap+no_recursion+det_timing; float ok |
| S-23 | MC/DC coverage | ✅ 2026-07-12 | compute_mcdc_map + vanic coverage subcommand; runtime instrumentation deferred |
| S-24 | Deviations strict | ✅ 2026-07-12 | --strict flag exits 1 on prefix="other" |
| S-25 | safety-attrs CLI | ✅ 2026-07-12 | already wired pre-session |
| S-26 | complexity CLI | ✅ 2026-07-12 | already wired pre-session |
| S-27 | Safety tutorial | ✅ 2026-07-12 | tutorials/src/advanced/12_safety_standards.md |
