# Safety-Critical Standards

vāṇī ships a structured safety layer that aligns with four industry
standards: **MISRA C 2012**, **ISO 26262 ASIL-D**, **DO-178C Level A**
(avionics), and **IEC 62304 Class C** (medical devices). This chapter
explains what each composite tag enforces, how to read the audit
artifacts, and how to integrate the safety CLI subcommands into a CI
pipeline.

---

## Why this chapter exists

Most software bugs are an inconvenience: a crashed app, a wrong
number on a screen, a page that needs a refresh. **Safety-critical
software runs where a bug can kill someone or destroy something
expensive and irreplaceable** -- the code steering a car, controlling
an aircraft's flight surfaces, dosing a patient's medication pump, or
running an industrial robot arm next to a human worker. "Ship it and
patch the bug next week" isn't an option when the software is already
airborne, already implanted, or already braking a car at highway
speed.

Because the stakes are that high, several industries don't just
*hope* engineers write careful code -- they require **documented
proof**, checked by an independent certifying body, that specific
classes of bugs are structurally impossible before the software is
allowed to ship. That's what the four standards below are: each one
is a checklist of properties a regulator or certifying body demands
evidence for, in a specific industry.

- **MISRA C 2012** -- a C coding-style rulebook originally written for
  the automotive industry by the Motor Industry Software Reliability
  Association (UK, 1998), now used far beyond cars (aerospace,
  medical, industrial control). Its rules exist because plain C makes
  it easy to write code whose behavior isn't fully pinned down by the
  language spec -- multiple exit points that hide which cleanup code
  actually ran, expressions whose evaluation order silently differs
  between compilers, branches a reviewer assumes are reachable but
  aren't. In a 2013 product-liability trial (*Bookout v. Toyota*),
  independent embedded-systems experts examined Toyota's throttle-
  control firmware and testified to finding MISRA C violations,
  thousands of global variables, and a stack-overflow risk in code
  that was supposed to be safety-critical -- a widely cited case study
  in why "it compiled and passed testing" isn't the same as "this
  code's behavior is fully understood."
- **ISO 26262 (ASIL)** -- the international functional-safety
  standard for road vehicles. It grades every safety-relevant function
  by an **ASIL** (Automotive Safety Integrity Level), from A (lowest)
  to **D** (highest), based on how severe the harm would be, how
  likely the situation is, and how much control a driver realistically
  has if it goes wrong. Braking, steering, and airbag deployment are
  textbook ASIL-D: failure there can be immediately life-threatening
  and the driver has little to no chance to compensate.
- **DO-178C Level A** -- the standard commercial aircraft software
  must satisfy to be certified airworthy by the FAA (US) or EASA
  (Europe). Its five levels (A, most severe, through E, no safety
  effect) grade software by the worst plausible outcome of it failing;
  Level A means the failure condition is "catastrophic" -- potential
  loss of the aircraft. Aviation's zero-tolerance culture around
  software exists because software failure in flight has no safe
  fallback the way a car can pull over. The 1996 loss of Ariane 5
  Flight 501 -- not itself an FAA-certified aircraft, but the
  textbook cautionary tale this whole category of standard exists to
  prevent -- is a stark illustration: a guidance-software module
  reused from Ariane 4 hit a floating-point-to-integer conversion it
  had never been tested against under Ariane 5's faster flight
  profile, overflowed, and triggered the rocket's self-destruct 37
  seconds after launch.
- **IEC 62304 Class C** -- the medical-device software lifecycle
  standard, adopted by the FDA (US) and EU medical-device regulators.
  Class C is its highest severity tier: software whose failure could
  cause death or serious injury. The case most safety engineers learn
  this standard's history from is the **Therac-25** (1985-1987), a
  radiation-therapy machine whose control software had a race
  condition: a specific fast sequence of operator keystrokes could
  slip past a safety interlock and deliver a radiation dose roughly
  100x the intended amount. Several patients died or were severely
  injured before the root cause -- a software bug, not a hardware
  fault -- was found. It remains one of the most-cited case studies in
  software-safety engineering precisely because nothing about the
  *hardware* was unusual; the danger was entirely in code nobody had
  proven correct.

The common thread: every one of these disasters involved code that
compiled, passed its tests, and shipped -- the gap was between "we
tested the cases we thought of" and "we can *prove* the dangerous
cases are unreachable." That's the gap vāṇी's safety layer targets.
The composite tags below aren't paperwork bolted on after the fact --
each one tells the compiler to refuse to produce a binary at all until
it can demonstrate, mechanically, that the property the standard cares
about (no unbounded stack growth, no unbounded execution time, no
hidden dynamic allocation, no unreachable-looking-but-actually-live
branches, ...) genuinely holds for that function.

---

## Composite standard tags

Place one composite tag on a function to opt it into a full constraint
set. Primitive attributes (`#[no_heap]`, `#[no_float]`, etc.) can
stack on top for extra tightening.

```vani
// ISO 26262 ASIL-D — most stringent automotive level.
// Implies: no_heap + no_recursion + no_float + no_nan + deterministic_timing.
// Requires: #[bounded_stack(bytes=N)] and #[wcet(cycles=N)] to be
// declared explicitly (the budgets are yours to choose).
#[asil_d]
#[bounded_stack(bytes = 2048)]
#[wcet(cycles = 5000)]
fn compute_brake_torque(speed: i64, pedal: i64) -> i64 {
  return speed * pedal / 100;
}
```

```vani
// DO-178C Level A — avionics. Same expansion as asil_d.
#[do178c_level_a]
#[bounded_stack(bytes = 1024)]
#[wcet(cycles = 3000)]
fn aileron_deflection(angle: i64) -> i64 {
  return angle * 17 / 100;
}
```

```vani
// MISRA C 2012 — implies no_heap + no_recursion.
// Rule 13.5 (short-circuit side effects), Rule 14.1 (dead branches),
// Rule 15.5 (single exit), and complexity ceiling are all enforced --
// note the single-exit style below: Rule 15.5 applies to every
// composite-tagged function, not just #[misra_c_2012] (confirmed by
// testing; an earlier version of this example had two `return`
// statements and did NOT actually pass `vanic check`).
#[misra_c_2012]
fn filter_sensor(raw: i64) -> i64 {
  let result: i64 = raw;
  if raw < 0 {
    result = 0;
  }
  return result;
}
```

```vani
// IEC 62304 Class C — medical devices. Implies no_heap + no_recursion,
// and (like every composite tag) Rule 15.5 single exit -- hence the
// single trailing `return` here too (confirmed by testing).
#[iec_62304_class_c]
fn dose_clamp(dose: i64, max: i64) -> i64 {
  let result: i64 = dose;
  if dose > max {
    result = max;
  }
  return result;
}
```

### Expansion matrix

| Tag | `no_heap` | `no_recursion` | `no_float` | `no_nan` | `deterministic_timing` | `bounded_stack` required | `wcet` required |
|---|---|---|---|---|---|---|---|
| `misra_c_2012` | ✅ | ✅ | | | | | |
| `iec_62304_class_c` | ✅ | ✅ | | | | | |
| `asil_d` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `do178c_level_a` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

The compiler actually recognizes three more composite tags beyond
the four this chapter focuses on (confirmed by grep in
`src/parser.rs`): `#[iec_61508_sil3]` and `#[iec_61508_sil4]`
(industrial functional safety, same expansion as `asil_d`/
`do178c_level_a`) and `#[autosar_ap]` (no_heap + no_recursion +
deterministic_timing, but float is permitted, and bounded_stack +
wcet must still be declared explicitly). Everything in this chapter
-- the CLI subcommands, MISRA rules, deviation tracking -- applies
to all seven the same way; only the four most commonly requested
ones get dedicated example code here.

---

## MISRA rules enforced

| Rule | Description | Trigger |
|---|---|---|
| **13.5** | No side effects in `&&` / `\|\|` right-hand operand | Any function marked `pure` or with a composite tag |
| **14.1** | No unreachable / always-dead branches (`if true`, `while false`) | Any function with a composite tag |
| **15.5** | Single point of exit (≤ 1 `return`) | Any function with a composite tag |
| **2.1** | No dead code after `return` / `break` / `continue` | All functions (Required rule) |
| **18.x** | Cyclomatic complexity ceiling (advisory) | Compile-time enforcement is opt-in via `INTENT_CHECK_COMPLEXITY=1` (or `INTENT_MAX_COMPLEXITY=<N>`); when enabled it flags **any** function over the threshold, tagged or not -- there's no `#[misra_c_2012]`-specific behavior, and this compiler has no warning/error severity split (confirmed by testing; an earlier version of this row was wrong on both counts). `vanic complexity` (below) always reports scores regardless of the env var. |

---

## Unsafe deviations

Every `unsafe` block must carry a reason string:

<img class="manas" src="../images/mascot/manas_mascot_caution.png" title="this code needs extra care"/>

```vani
unsafe(reason = "MMIO: direct peripheral register write per HAL spec §3.2") {
  mmio_write_u32(0x4001_0000, value);
}
```

The reason string must start with a recognised prefix:
`MMIO`, `FFI`, `DMA`, `transmute`, `vendor-SDK`. Anything else is
classified as `"other"`.

### Extracting deviation records

```sh
# Human-readable (default)
vanic deviations src/firmware.vani

# Machine-readable for CI dashboards
vanic deviations src/firmware.vani --format=json

# Fail CI if any deviation has an unrecognised prefix
vanic deviations src/firmware.vani --strict

# Write to file for the sign-off package
vanic deviations src/firmware.vani --format=csv --out=deviations.csv
```

---

## Stack depth analysis

`vanic stack-depth` reports the estimated worst-case stack usage from
each program entry point, following every call chain.

```sh
vanic stack-depth src/firmware.vani

# Fail if any entry point exceeds 8 KiB
vanic stack-depth src/firmware.vani --max=8192

# JSON for audit dashboards
vanic stack-depth src/firmware.vani --format=json
```

Example output (real, from running `vanic stack-depth` against the
`compute_brake_torque` / `aileron_deflection` / `filter_sensor` /
`dose_clamp` functions shown above -- confirmed by testing; an
earlier version of this page showed a fabricated `entry main — max
depth …` / `Functions with unbounded recursion:` format that this
subcommand doesn't actually produce):

```
Per-function frame sizes:
  compute_brake_torque             56 bytes (locals: 24, prologue: 32)
  aileron_deflection               48 bytes (locals: 16, prologue: 32)
  filter_sensor                    64 bytes (locals: 32, prologue: 32)
  dose_clamp                       72 bytes (locals: 40, prologue: 32)
  main                             40 bytes (locals: 8, prologue: 32)

Per-entry-point max stack depths:
  compute_brake_torque             56 bytes  via compute_brake_torque
  aileron_deflection               48 bytes  via aileron_deflection
  filter_sensor                    64 bytes  via filter_sensor
  dose_clamp                       72 bytes  via dose_clamp
  main                             112 bytes  via main -> dose_clamp
```

With `--max=<N>`, any entry point whose depth exceeds `N` gets an
extra `EXCEEDS --max=<N> by <K> bytes` line and the command exits 1.

For functions with `#[bounded(N)]`, the estimator models N+1 stack
frames (worst case just before the runtime guard trips).

---

## Call-graph acyclicity

Mutual recursion makes WCET analysis impossible. `vanic acyclicity`
runs Tarjan's SCC algorithm to detect it:

```sh
vanic acyclicity src/firmware.vani

# Fail CI on any cycle
vanic acyclicity src/firmware.vani --format=json
```

Functions with `#[bounded(N)]` are exempt from the self-call rule
(their depth guard gives a finite bound).

---

## WCET annotation

`#[wcet(cycles=N)]` caps the worst-case execution time. The compiler
walks the function body with a static cost model and errors if the
estimated cycle count exceeds `N`.

```vani
#[do178c_level_a]
#[bounded_stack(bytes = 512)]
#[wcet(cycles = 200)]
fn read_sensor_array(data: [i64; 8]) -> i64 {
  let total: i64 = 0;
  for i from 0 to 8 {          // index-based: 8 iterations × body cost
    total = total + data[i];
  }
  return total;
}
```

This is confirmed by testing -- an earlier version of this example
used `for x in &data`, which is invalid syntax twice over: this
language has no `&` borrow operator (it's `ref`), and even the
correctly-spelled collection form (`for x in ref data`) is rejected
here specifically *because* `#[do178c_level_a]` implies
`#[deterministic_timing]`, which rejects every `for ... in
<collection>` iterator outright regardless of whether the collection
is a fixed-size array -- it wants an index-based `for i from 0 to N`
loop instead. Drop `#[do178c_level_a]` (keep bare `#[wcet]`) if you
want the collection-iteration form; see the note in the cycle model
below.

The cycle model (conservative):
- ALU op / load / store: 2 cycles
- Function call: 10 cycles (or the callee's declared `wcet` if annotated)
- `print` / `eprint`: 50 cycles (syscall baseline)
- `for i from 0 to N` with literal N: body cycles × N
- `for x in ref arr` over `[T; N]`: body cycles × N (S-12 improvement)
  -- **only** under a bare `#[wcet(...)]`; rejected under
  `#[deterministic_timing]` / `#[asil_d]` / `#[do178c_level_a]`
  (confirmed by testing), which require the index-based form above
- Unbounded loops, Vec iteration, calls to unannotated functions: UNBOUNDED (error)

---

## Safety attributes report

Get a machine-readable summary of every function's safety annotations:

```sh
# Human-readable table
vanic safety-attrs src/firmware.vani

# CSV for spreadsheet review
vanic safety-attrs src/firmware.vani --format=csv

# JSON for audit tooling
vanic safety-attrs src/firmware.vani --format=json
```

---

## Safety-attribute coverage gate

Every check above trusts that `#[bounded_stack]`/`#[wcet]` were declared in
the first place -- nothing stopped a function that *could* carry one from
shipping without it. `vanic audit-safety` closes that gap: it reuses the
same stack-depth/WCET estimators the enforcement passes use, but runs them
**unconditionally** (not gated on the attribute already being present) to
determine whether each function is *eligible*, then flags any
eligible-but-missing case.

```sh
vanic audit-safety src/firmware.vani

# Machine-readable, for CI dashboards
vanic audit-safety src/firmware.vani --format=json
```

Coverage means "the attribute exists wherever it's computable" -- not
blanket 100% attribute presence. A function with a fn-pointer parameter
can't have a computable `#[bounded_stack]` (an indirect call's frame cost
is unknowable to the checker), and a function with an unbounded loop or
unannotated recursion can't have a computable `#[wcet]`. Both are
legitimately exempt and never flagged.

```
$ vanic audit-safety src/firmware.vani
audit-safety: 1 of 12 function(s) missing an attribute they're eligible for (0 vendored fn(s) excluded):

  clamp_score (src/firmware.vani:41:1)
    missing #[bounded_stack(bytes = 32)] -- computed worst-case is 32 bytes

Add the attribute with the exact value shown (vanic will re-verify it), or
if this function genuinely shouldn't carry one, that's a bug in this
checker's eligibility rules -- please report it.
```

The reported value is exact -- copy it in verbatim and the normal
`#[bounded_stack]`/`#[wcet]` enforcement (already covered above) re-verifies
it on the next `vanic check`. Kosh package publishing (`vanic publish`)
runs this same check against the package entry and hard-blocks on any gap,
with `--allow-partial-safety-coverage` as an explicit escape hatch -- see
[Sec.16 -- Kosh Packages](../intermediate/16_packages.md#safety-coverage-gate).

---

## Cyclomatic complexity

```sh
# Report with default threshold (15)
vanic complexity src/firmware.vani

# Fail if any function exceeds complexity 10
vanic complexity src/firmware.vani --max=10

# Machine-readable
vanic complexity src/firmware.vani --format=json
```

---

## Full audit pack

<img class="manas" src="../images/mascot/manas_mascot_awesome.png" title="a good habit worth adopting"/>

`vanic audit-pack` runs all six checks (deviations, stack-depth,
acyclicity, complexity, safety-attrs, HashMap shapes) and bundles
results into a single Markdown document:

```sh
vanic audit-pack src/firmware.vani \
  --max-stack=8192 \
  --max-complexity=10 \
  --out=audit_report.md
```

This is the **reviewer-facing artifact** — one file with a summary
table followed by six per-fact sections. Suitable for attaching to a
DO-178C Software Accomplishment Summary or ISO 26262 Work Product.

---

## Real-world example: medical device sensor loop

```vani
// Pulse oximeter SpO2 sample accumulator.
// IEC 62304 Class C — patient-safety-critical path.
// No dynamic allocation. No recursion. Single exit per function
// (confirmed by testing -- an earlier version of this example had
// two `return` statements and a `for s in &samples` collection
// loop, neither of which actually passes `vanic check` here: MISRA
// 15.5 rejects the second `return`, and `&` isn't this language's
// borrow syntax -- it's `ref`).
#[iec_62304_class_c]
fn accumulate_spo2(samples: [i64; 16]) -> i64 {
  let total: i64 = 0;
  let count: i64 = 0;
  for s in ref samples {
    if s >= 0 {
      total = total + s;
      count = count + 1;
    }
  }
  let result: i64 = -1;    // sentinel: no valid samples
  if count > 0 {
    result = total / count;
  }
  return result;
}

fn main() -> i64 {
  let data: [i64; 16] = [98, 97, 99, 98, 97, 99, 98, 97,
                          99, 98, 97, 99, 98, 97, 99, 98];
  let avg: i64 = accumulate_spo2(data);
  print avg;
  return 0;
}
```

```sh
# Verify the audit package
vanic audit-pack spo2.vani --max-stack=4096 --max-complexity=5
vanic deviations spo2.vani --strict   # no unsafe blocks → exits 0
vanic acyclicity spo2.vani            # no cycles → exits 0
```

---

## Adversarial test coverage

vāṇī ships `tests/safety_adversarial.rs` — 31 integration tests that
probe the safety passes with non-obvious inputs. They cover:

| Group | Tests |
|---|---|
| ASIL-D / DO-178C constraints | Missing `wcet`, missing `bounded_stack`, direct float use, transitive float via helper, heap allocation, direct and indirect recursion |
| MISRA C 2012 rules | Multiple returns in flat bodies, returns buried in nested if/else, returns inside for-loop bodies, always-true branches, `while false` loops, eval-order violations |
| WCET model | Budget exceeded by arithmetic, unbounded while loop, unbounded path in one branch |
| Bounded stack | Exceeded by local bindings, exceeded via call chain |
| Lock-order (S-19) | Two functions with opposite orderings, both orderings in a single function's branches |
| ISR priority (S-20) | Two ISRs at different priorities sharing a mutex name |
| Inline stack (S-14) | Inline callee locals folded into caller frame; non-inline sub-callees still get own frames |
| No false positives | Consistent lock order, single MISRA exit, valid ASIL-D program |
| Documented gaps | Three acceptance tests that pin known limitations (see below) |

---

## Previously documented gaps — now fully fixed

Three analysis limitations were discovered via the adversarial test suite
in the previous release. All three have been fixed as of 2026-07-12.
Their `gap_*` tests in `tests/safety_adversarial.rs` now assert
**rejection** (not acceptance), confirming the fixes are in place.

### Gap 1 (L20) — S-19: transitive lock-order detection ✅ Fixed

**Was.** `enforce_lock_order` only walked each function's own body to
collect `mutex_lock` calls. If `fn_a` locked `m_x` then called a helper
that locked `m_y`, the cross-function ordering `m_x→m_y` was invisible.

**Fix.** The analysis now uses a held-set approach (`build_lock_edges` /
`build_lock_edges_expr`). When a user-defined callee is encountered, its
body is walked with a clone of the caller's current held-lock set.
Callee-acquired locks are released on return (clone discarded), so
independent sequential calls do not create spurious ordering constraints.
Only locks truly held by the caller at the call site constrain the callee's
first lock.

### Gap 2 (L21) — S-20: ISR mutex acquisition through a helper ✅ Fixed

**Was.** `collect_locked_mutexes` only walked the ISR's own body. A
mutex acquired by a helper called from the ISR was invisible to the
priority-inversion check.

**Fix.** `collect_locked_mutexes` / `collect_locked_mutexes_stmts` /
`collect_locked_mutexes_expr` now accept `fn_map` and `visiting`
parameters and recursively follow calls into user-defined functions,
building the full transitive mutex set for each ISR.

### Gap 3 (L22) — MISRA 13.2: non-adjacent duplicate arguments ✅ Fixed

**Was.** The MISRA 13.2 eval-order check only fired when the same variable
appeared in **consecutive** arg positions (positions `k` and `k+1`).
`foo(x, y, x)` was not flagged.

**Fix.** `check_eval_order_expr` now uses `seen.remove()` instead of
`seen.get()`. Any second occurrence of a variable in the same call's
arg list fires the diagnostic, regardless of the positions' distance.

---

## Safety certification coverage

All previously partial checks are now complete. The full compliance matrix:

| Objective | Tool coverage |
|---|---|
| No dynamic allocation | `#[no_heap]` + transitive fixpoint — **complete** |
| No recursion | `#[no_recursion]` + BFS call graph — **complete** |
| Stack bound | `#[bounded_stack]` + full call-chain depth analysis — **complete** |
| Execution time bound | `#[wcet]` + static cycle estimator — **complete** (conservative) |
| No floating point | `#[no_float]` + transitive fixpoint — **complete** |
| No NaN-contract builtins | `#[no_nan]` rejects `f64_nan()` and `vec_kth_smallest` on `Vec<f64>` — **complete** |
| Deterministic timing | `#[deterministic_timing]` branch-balance check — **complete** |
| Deviation tracking | `vanic deviations --strict` — **complete** |
| Call-graph acyclicity | `vanic acyclicity` (Tarjan SCC) — **complete** |
| MC/DC coverage points | `vanic coverage` — **complete** (runtime counters deferred) |
| MISRA single exit | Rule 15.5 recursive walk — **complete** |
| MISRA dead branches | Rule 14.1 literal-condition check — **complete** |
| Lock-order deadlocks | S-19 held-set transitive analysis — **complete** |
| ISR priority inversion | S-20 transitive mutex collection — **complete** |
| MISRA eval order | Rule 13.2 any-distance duplicate detection — **complete** |
| Bounded_stack/WCET coverage | `vanic audit-safety` (eligible-but-missing detection) + `vanic publish` hard gate — **complete** |

Safety standards (ISO 26262, DO-178C, IEC 62304) still require Tool
Qualification Documentation (TQD) describing the analysis scope, but no
partial-coverage disclosures are needed for L20–L22 as of this release.

---

## CI integration

Recommended `.github/workflows/safety.yml` gates:

```yaml
- name: Safety audit
  run: |
    vanic acyclicity src/firmware.vani
    vanic stack-depth src/firmware.vani --max=16384
    vanic complexity src/firmware.vani --max=15
    vanic deviations src/firmware.vani --strict
    vanic audit-safety src/firmware.vani
    vanic coverage src/firmware.vani --format=json --out=mcdc_map.json
```


---

**Previous**: [Sec.11 -- Using vani with an LLM ->](11_llm_workflows.md)

**Next**: [Sec.13 -- A world tour: vāṇी in your language ->](13_global_showcase.md)

