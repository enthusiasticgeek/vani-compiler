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

## CI integration

Recommended `.github/workflows/safety.yml` gates:

```yaml
- name: Safety audit
  run: |
    vanic acyclicity src/firmware.vani
    vanic stack-depth src/firmware.vani --max=16384
    vanic complexity src/firmware.vani --max=15
    vanic deviations src/firmware.vani --strict
```
