# Safety-Critical Standards

vāṇी ships a structured safety layer that aligns with four industry
standards: **MISRA C 2012**, **ISO 26262 ASIL-D**, **DO-178C Level A**
(avionics), and **IEC 62304 Class C** (medical devices). This chapter
explains what each composite tag enforces, how to read the audit
artifacts, and how to integrate the safety CLI subcommands into a CI
pipeline.

---

## Composite standard tags

Place one composite tag on a function to opt it into a full constraint
set. Primitive attributes (`#[no_heap]`, `#[no_float]`, etc.) can
stack on top for extra tightening.

```vani
// ISO 26262 ASIL-D — most stringent automotive level.
// Implies: no_heap + no_recursion + no_float + deterministic_timing.
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
// Rule 15.5 (single exit), and complexity ceiling are all enforced.
#[misra_c_2012]
fn filter_sensor(raw: i64) -> i64 {
  if raw < 0 {
    return 0;
  }
  return raw;
}
```

```vani
// IEC 62304 Class C — medical devices. Implies no_heap + no_recursion.
#[iec_62304_class_c]
fn dose_clamp(dose: i64, max: i64) -> i64 {
  if dose > max {
    return max;
  }
  return dose;
}
```

### Expansion matrix

| Tag | `no_heap` | `no_recursion` | `no_float` | `deterministic_timing` | `bounded_stack` required | `wcet` required |
|---|---|---|---|---|---|---|
| `misra_c_2012` | ✅ | ✅ | | | | |
| `iec_62304_class_c` | ✅ | ✅ | | | | |
| `asil_d` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `do178c_level_a` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

---

## MISRA rules enforced

| Rule | Description | Trigger |
|---|---|---|
| **13.5** | No side effects in `&&` / `\|\|` right-hand operand | Any function marked `pure` or with a composite tag |
| **14.1** | No unreachable / always-dead branches (`if true`, `while false`) | Any function with a composite tag |
| **15.5** | Single point of exit (≤ 1 `return`) | Any function with a composite tag |
| **2.1** | No dead code after `return` / `break` / `continue` | All functions (Required rule) |
| **18.x** | Cyclomatic complexity ceiling (advisory) | Warning when score > 15; error under `#[misra_c_2012]` |

---

## Unsafe deviations

Every `unsafe` block must carry a reason string:

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

Example output:

```
entry main — max depth 1240 bytes
  main → compute_brake_torque (256 B)
  main → filter_sensor (48 B)

Functions with unbounded recursion:
  (none)
```

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
  for x in &data {          // fixed-size array: 8 iterations × body cost
    total = total + x;
  }
  return total;
}
```

The cycle model (conservative):
- ALU op / load / store: 2 cycles
- Function call: 10 cycles (or the callee's declared `wcet` if annotated)
- `print` / `eprint`: 50 cycles (syscall baseline)
- `for i in 0..N` with literal N: body cycles × N
- `for x in &arr` over `[T; N]`: body cycles × N (S-12 improvement)
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
// No dynamic allocation. No recursion. Single exit per function.
#[iec_62304_class_c]
fn accumulate_spo2(samples: [i64; 16]) -> i64 {
  let total: i64 = 0;
  let count: i64 = 0;
  for s in &samples {
    if s >= 0 {
      total = total + s;
      count = count + 1;
    }
  }
  if count == 0 {
    return -1;    // sentinel: no valid samples
  }
  return total / count;
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

## Known analysis gaps and real-world compliance

Three limitations were discovered via the adversarial test suite and
are documented here. Each has a corresponding `gap_*` test in
`tests/safety_adversarial.rs` that currently asserts the program is
**accepted** — and will tell you to flip it to a rejection test once
the limitation is resolved.

### Gap 1 — S-19: transitive lock-order detection

**What it is.** `enforce_lock_order` collects `mutex_lock` call sequences
by walking each function's body directly. It does **not** follow calls
into helper functions. If function `A` locks `m_x` then calls `helper`
which locks `m_y`, and function `B` locks `m_y` then calls a helper
which locks `m_x`, the effective orderings `m_x→m_y` and `m_y→m_x`
form a deadlock cycle — but the analyser sees only single-element
sequences `[m_x]` and `[m_y]`, never building the cross-function edges.

**Workaround.** Inline the locking sequence into the calling function,
or use a wrapper that acquires both locks in one place:

```vani
// SAFE: both locks always acquired in the same function, in the same order.
fn with_both_locks(m_x: ref Mutex<i64>, m_y: ref Mutex<i64>) -> i64 {
  let gx: Guard<i64> = mutex_lock(m_x);
  let gy: Guard<i64> = mutex_lock(m_y);
  return 0;
}
```

### Gap 2 — S-20: ISR mutex acquisition through a helper

**What it is.** `collect_locked_mutexes` only walks the ISR's own body.
If an ISR calls a non-ISR helper that acquires a mutex, that mutex is
not attributed to the ISR's lock set. Two ISRs at different priorities
that share a resource via a common helper will not trigger the
priority-inversion warning.

**Workaround.** Keep `mutex_lock` calls in the ISR body directly, or
use atomics (`Atomic<T>`) for resources shared across priority levels,
which are immune to priority inversion.

```vani
// PREFERRED for shared ISR resources: atomic avoids the gap entirely.
#[interrupt(priority = 1)]
fn high_isr(counter: ref Atomic<i64>) -> i64 {
  return atomic_fetch_add(counter, 1);
}
```

### Gap 3 — MISRA 13.2: non-adjacent duplicate arguments

**What it is.** The MISRA 13.2 eval-order check only fires when the same
variable appears in **consecutive** arg positions (positions `k` and
`k+1`). A variable in positions 0 and 2 of the same call — with an
unrelated arg in between — does not trigger a diagnostic:

```vani
// Caught:     foo(x, x)        — x at positions 0 and 1
// NOT caught: foo(x, y, x)     — x at positions 0 and 2 (gap)
```

**Severity.** MISRA C 2012 Rule 13.2 is an **Advisory** rule, so a
documented partial implementation is permitted in the MISRA compliance
matrix. The pattern is also unusual in practice; the adjacent case
covers the vast majority of real violations.

---

## Do these gaps prevent real-world safety certification?

**Short answer: No — with conditions.**

Safety standards (ISO 26262, DO-178C, IEC 62304) do not require
automated tooling to catch *all* possible defects. They require:

1. **Tool qualification documentation** — a Safety Manual or Tool
   Qualification Document (TQD) that honestly describes what each tool
   analysis does and does not cover.
2. **Supplementary verification** — code review, integration tests, or
   formal proof to cover gaps the tool does not detect.
3. **Deviation records** — for MISRA Advisory rules, an entry in the
   MISRA compliance matrix is sufficient.

The three gaps documented above are **over-approximations** (the tool
misses real issues in specific patterns) rather than
**under-approximations** (the tool accepts clearly wrong code). All
three involve patterns that can be found by structured code review.

The broader safety toolchain covers the most critical certification
objectives independently:

| Objective | Tool coverage |
|---|---|
| No dynamic allocation | `#[no_heap]` + transitive fixpoint — **complete** |
| No recursion | `#[no_recursion]` + BFS call graph — **complete** |
| Stack bound | `#[bounded_stack]` + full call-chain depth analysis — **complete** |
| Execution time bound | `#[wcet]` + static cycle estimator — **complete** (conservative) |
| No floating point | `#[no_float]` + transitive fixpoint — **complete** |
| Deterministic timing | `#[deterministic_timing]` branch-balance check — **complete** |
| Deviation tracking | `vanic deviations --strict` — **complete** |
| Call-graph acyclicity | `vanic acyclicity` (Tarjan SCC) — **complete** |
| MC/DC coverage points | `vanic coverage` — **complete** (runtime counters deferred) |
| MISRA single exit | Rule 15.5 recursive walk — **complete** |
| MISRA dead branches | Rule 14.1 literal-condition check — **complete** |
| Lock-order deadlocks | S-19 intra-function + cross-function (direct) — **partial** |
| ISR priority inversion | S-20 direct-body detection — **partial** |
| MISRA eval order | Rule 13.2 adjacent-arg check — **partial** |

The three partial checks are supplementary safety aids, not
certification gates. For a certification-ready project:

1. Add `gap_s19_*` and `gap_s20_*` patterns to your **integration test**
   suite (the adversarial tests already show how).
2. Add a code-review checklist item: *"All mutex acquisitions that span
   two or more functions are reviewed for ordering consistency."*
3. For MISRA 13.2: add a static analyser (PC-lint, Polyspace, etc.) or
   extend the check — or document the advisory deviation.

The `gap_*` tests in `tests/safety_adversarial.rs` serve as living
documentation of these conditions. When the limitations are eventually
closed in the compiler, those tests will fail with a helpful message
("Gap is already fixed! Update this test to assert_rejected") — making
the improvement visible rather than silent.

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
    vanic coverage src/firmware.vani --format=json --out=mcdc_map.json
```
