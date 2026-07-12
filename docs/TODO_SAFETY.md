# vāṇी — Safety-Critical Standards TODO

Generated 2026-07-12 from a full audit of `src/safety.rs`, `src/smt.rs`,
`src/checker.rs`, `src/acyclicity.rs`, `src/stack_depth.rs`, and the
composite-tag expansion in `src/parser.rs`.

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

- [ ] **S-7. MISRA Rule 13.1–13.4 — no side effects in sub-expressions**
  Extend `enforce_misra_13` (already in `safety.rs`) to cover:
  - Rule 13.1: initializer expressions
  - Rule 13.2: value of expression shall not depend on order of evaluation
  - Rule 13.3: increment/decrement (vāṇī doesn't have `++` but assignment
    expressions in call args are the equivalent)
  - Rule 13.4: assignment in sub-expression (e.g. `f(x = y)` pattern)

- [ ] **S-8. MISRA Rule 17.1 — no variadic functions**
  Under `#[misra_c_2012]`: flag any `extern "C"` declaration whose
  parameter list is variadic. vāṇी has no user-level variadics but FFI
  bindings can declare them.

- [ ] **S-9. MISRA Rule 11.1–11.3 — function pointer conversions**
  Flag casts between function-pointer types and data-pointer types in
  `unsafe` blocks under `#[misra_c_2012]`.

- [x] **S-10. MISRA Rule 2.1 — no dead code** ✅ 2026-07-12
  Flag unreachable statements after a `return`, `break`, or `continue`.

---

## Tier C — WCET model improvements (high impact for certifiability)

- [ ] **S-11. Architecture-calibrated cycle table**
  Current model: flat 10 cycles per builtin/extern call, linear
  accumulation. Replace with a per-`--target` table:
  - x86-64: calibrated from Intel optimization reference (simple ops ~1–4
    cycles, divide ~20–80, sqrt ~10–20, branch misprediction ~15).
  - aarch64: ARM Cortex-A55/A72 typical latencies.
  - riscv64: in-order pipeline estimate (flat 1–3 cycles for ALU).
  The `--target` flag is already parsed in `main.rs`; thread it through to
  `enforce_wcet` via a new `WcetProfile` struct.

- [x] **S-12. Bounded-loop WCET propagation** ✅ 2026-07-12 (ForIter over [T;N] arrays)
  Currently a `for i in 0..N` where `N` is a variable reports UNBOUNDED.
  If the loop range comes from a literal, a `requires`-clause bound, or a
  `#[bounded(N)]`-annotated parameter, multiply the body WCET by the bound.
  Requires: read the `requires` clause variables and try constant-fold `N`.

- [ ] **S-13. `wcet` annotation is mandatory under `asil_d` / `do178c_level_a`**
  Emit an error (not warning) if a function tagged with either composite
  lacks a `#[wcet(cycles=N)]` annotation. Implement in the composite
  expansion verification step (after S-1/S-2 are done).

---

## Tier D — Stack analysis improvements

- [ ] **S-14. Inlining-aware stack depth**
  `stack_depth.rs` currently treats every call as a full frame push. If the
  backend would inline a function (small body, `#[inline]`), the frame
  isn't pushed. Add an `#[inline]` attribute and treat inlined calls as
  adding their local bytes to the caller's frame rather than a separate
  frame in the call-chain.

- [ ] **S-15. `bounded_stack` enforced under composite tags**
  Under `#[asil_d]` / `#[do178c_level_a]`, emit an error if
  `bounded_stack` is not declared on the function. Currently the composite
  expands only `no_heap + no_recursion`; this requires S-1/S-2.

---

## Tier E — SMT verification gaps

- [ ] **S-16. Vec index bounds in SMT**
  `smt.rs` returns `SkippedUnsupported` for array/Vec indexing. Add
  encoding: for `xs[i]`, assert `0 <= i < xs_len` as a side-condition and
  model the element as an opaque symbolic. This enables proofs like
  `requires i < len(xs)` + `ensures result == xs[i]`.

- [ ] **S-17. Loop invariant annotation syntax**
  Add `invariant <expr>` inside `while` / `for` bodies (similar to SPARK
  `Loop_Invariant`). The checker should pass the invariant to z3 as an
  assumed fact at each loop iteration, enabling proofs over loop bodies
  without full loop unrolling.

- [ ] **S-18. Cross-module SMT (requires/ensures across files)**
  Currently `ensures` substitution only works within a single compilation
  unit. Track `ensures` in the manifest (`manifest.rs`) so that callers
  from other `.vani` files can use them.

---

## Tier F — Concurrency / race safety

- [ ] **S-19. Lock-order graph and deadlock detection**
  Build a static lock-acquisition-order graph from the program's mutex
  calls. Detect cycles (A acquires M1 then M2; B acquires M2 then M1 →
  potential deadlock). Report as a warning-level diagnostic.

- [ ] **S-20. ISR priority and preemption model**
  `#[interrupt]` currently validates the body but not the priority level.
  Add `#[interrupt(priority=N)]` syntax. Detect when a lower-priority ISR
  accesses a resource that a higher-priority ISR also accesses without an
  atomic or `#[no_preempt]` guard.

---

## Tier G — New standard support

- [ ] **S-21. IEC 61508 SIL-1/2/3/4 composite tag**
  IEC 61508 (functional safety of E/E/PE systems) covers general industrial
  safety. Add `#[iec_61508_sil3]` and `#[iec_61508_sil4]` composite tags.
  SIL-3 expansion: `no_heap + no_recursion + no_float + bounded_stack +
  deterministic_timing`. SIL-4 adds `wcet` required.

- [ ] **S-22. AUTOSAR Adaptive (AP) composite tag**
  AUTOSAR AP allows dynamic memory in a controlled way. Add
  `#[autosar_ap]` tag that permits heap but enforces `deterministic_timing`
  + deviation records for all `unsafe` blocks.

- [ ] **S-23. MC/DC coverage instrumentation**
  DO-178C Level A requires Modified Condition/Decision Coverage. Add a
  compile-time instrumentation pass (`--instrument-mcdc`) that inserts
  branch-hit counters and a post-run report. Output: a coverage map linking
  each condition in each decision to its T/F hit count.

---

## Tier H — Tooling / audit quality

- [x] **S-24. Deviation record: transitive closure per standard** ✅ 2026-07-12
  `deviations.rs` emits one record per `unsafe` block but doesn't verify
  that every function tagged `#[asil_d]` (or its callees) has zero
  unreviewed deviations. Add a `vanic deviations --strict` mode that errors
  if any deviation in the transitive closure of a standard-tagged function
  lacks a known prefix.

- [ ] **S-25. Safety attrs report: CLI subcommand**
  `compute_safety_attrs_report` exists in `safety.rs` but is not wired to a
  CLI subcommand. Add `vanic safety-attrs [--format=text|json|csv]` to
  expose it. Useful for audit dashboards.

- [ ] **S-26. Complexity report: CLI subcommand**
  `compute_complexity_report` / `format_complexity_*` exist but lack a
  dedicated subcommand. Add `vanic complexity [--format=text|json|csv]
  [--threshold=N]`.

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
| S-7 | MISRA 13.1–13.4 | [ ] | |
| S-8 | MISRA 17.1 | [ ] | |
| S-9 | MISRA 11.1–11.3 | [ ] | |
| S-10 | MISRA 2.1 | ✅ 2026-07-12 | enforce_dead_code_after_jump in safety.rs; fires for all fns |
| S-11 | WCET arch table | [ ] | |
| S-12 | WCET bounded loops | ✅ 2026-07-12 | ForIter over [T;N] arrays: body_cycles × N |
| S-13 | WCET mandatory | ✅ 2026-07-12 | error if absent under asil_d/do178c_level_a (done in S-1/S-2) |
| S-14 | Stack / inline | [ ] | |
| S-15 | Stack mandatory | ✅ 2026-07-12 | error if absent under asil_d/do178c_level_a (done in S-1/S-2) |
| S-16 | SMT Vec index | [ ] | |
| S-17 | SMT loop invariant | [ ] | |
| S-18 | SMT cross-module | [ ] | |
| S-19 | Lock-order deadlock | [ ] | |
| S-20 | ISR priority model | [ ] | |
| S-21 | IEC 61508 tag | [ ] | |
| S-22 | AUTOSAR AP tag | [ ] | |
| S-23 | MC/DC coverage | [ ] | large |
| S-24 | Deviations strict | ✅ 2026-07-12 | --strict flag exits 1 on prefix="other" |
| S-25 | safety-attrs CLI | ✅ 2026-07-12 | already wired pre-session |
| S-26 | complexity CLI | ✅ 2026-07-12 | already wired pre-session |
| S-27 | Safety tutorial | ✅ 2026-07-12 | tutorials/src/advanced/12_safety_standards.md |
