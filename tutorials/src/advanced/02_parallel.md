# Advanced 2 -- `parallel for` + reductions + race-freedom

> **Learning goal**: turn a sequential loop into a `parallel
> for`, declare a `reduce` accumulator, and understand how the
> affine type system proves race-freedom at compile time.

> **New to this?** Read [Advanced 2a -- Parallelism and race-freedom primer](02a_parallelism_primer.md) first.

Imagine ten accountants each adding up a separate stack of
receipts simultaneously -- each one works on their own stack,
never touching anyone else's, and at the end a supervisor
adds up all their sub-totals. No two accountants can possibly
interfere because the stacks are separate. That's a parallel
reduction: split the work into non-overlapping pieces, do them
simultaneously, combine the results. `parallel for ... reduce`
expresses this pattern; the compiler verifies at compile time
that loop iterations don't share writeable data (race-freedom
-- no accountant steals from another's stack mid-tally).

## The program

<img class="manas" src="../images/mascot/manas_mascot_success.png" title="this is the correct, working version"/>

```vani
intent "Advanced 2 worked example -- parallel for + reduction.";

fn main() -> i64 {
  // Sum 1..100 sequentially first as the reference.
  let seq: i64 = 0;
  for i from 1 to 101 {
    seq = seq + i;
  }
  print "seq =", seq;

  // The same loop with parallel for + reduce. The compiler
  // proves the body has no inter-iteration data dependence
  // (affine ownership + reduce clause). The SSA LLVM backend
  // (default) uses per-thread local accumulators combined with
  // a single atomic at the end; the C backend emits OpenMP
  // `reduction(+: total)`.
  let par: i64 = 0;
  parallel for i from 1 to 101
  reduce par with +;
  {
    par = par + i;
  }
  print "par =", par;

  assert seq == par;
  return 0;
}
```

## Compile + run

```bash
vanic run ~/adv2.vani
```

Output:

```
seq = 5050
par = 5050
```

Both backends produce the same answer; `parallel for` adds
multi-threaded execution on hosted targets without changing
the semantics.

## A rejected loop body: cross-iteration dependency

For contrast, here's a loop body the compiler rejects. Each
iteration reads the slot the *previous* iteration wrote --
there is no order guarantee across threads, so the result
would be nondeterministic:

<img class="manas" src="../images/mascot/manas_mascot_error.png" title="this code does not compile!"/>

```vani
fn bad_prefix(xs: mut ref Vec<i64>, n: u64) -> i64 {
  parallel for i from 1 to n {
    xs[i] = xs[i - 1] + 1;   // wrong: reads a slot another iteration writes
  }
  return 0;
}
```

```
error: 'parallel for' body cannot mutate 'xs[i] = …' (indexed write is a side effect)
    xs[i] = xs[i - 1] + 1;   // wrong: reads a slot another iteration writes
       ^
  help: Pure functions in vāṇी are honored by both SMT (allowed inside
  `requires` / `ensures`) AND the parallel-for safety pass. They must be
  safe to call multiple times with the same arguments and produce the
  same result, with no externally visible mutation.
```

Indexed assignment on a captured `Vec` is exactly the "assignments to
array slots" case called out above -- the compiler can't prove the
slots don't alias across iterations without a `reduce` clause, so it
rejects the whole loop rather than risk a race.

## Why it works that way

- **Sequential `for i from lo to hi { ... }`** is the baseline.
  Each iteration runs in order; mutations to captured
  variables (`seq = seq + i`) commit before the next iteration.
- **`parallel for`** lifts the body to run on N threads. The
  compiler **statically rejects** loops whose body has
  iteration-to-iteration data dependencies -- mutable captures
  without a `reduce` clause, an array-slot write at any index
  other than the loop's own (`xs[j]`, `xs[i-1]`, ...), or a
  same-index write (`xs[i] = ...`) whose value reads that same
  array at a different index. A same-index write that only reads
  its own slot (or a different, untouched array entirely) is
  allowed -- each iteration owns a distinct slot, so there's
  nothing to race on. This is the safety boundary.
- **`reduce <var> with <op>;`** declares an accumulator variable
  that survives across iterations. The SSA LLVM backend (default)
  allocates one stack-local accumulator per thread, applies the
  reduction body with no atomic ops, then combines per-thread
  results with a single `atomicrmw` at the parallel region's exit.
  The C backend emits an OpenMP `reduction(<op>: <var>)` clause.
  Per-thread partial sums combine to the final value at the
  loop's barrier.
- **Race-freedom is proved**, not asserted. The combination of
  affine ownership (each binding has one owner), the explicit
  `reduce` declaration, and the no-iteration-dependence check
  means a `parallel for` that compiles is guaranteed
  data-race-free.

## What's allowed in a `parallel for` body

| Allowed | Rejected |
|---|---|
| Reads of captured `let` bindings | Writes to non-reduce captures |
| Writes through `mut ref` parameters | Indexed assignment `xs[j] = ...` where `j` isn't the loop's own index |
| `xs[i] = ...` where `i` IS the loop's own index, and the value doesn't read `xs` at any other index | `xs[i] = xs[i - 1] + ...` -- safe write index, but the read side aliases another iteration's write |
| Calls to pure fns | Calls to impure fns (FFI, IO) without `unsafe` |
| `reduce var with +`/`*`/`max`/`min` | Multiple `reduce` clauses on the same variable |

## Mapping a Vec without a reduce

If you want a transform-each-element pattern (no reduction),
the safe v1 form is a `mut ref Vec` + index-by-iteration:

<img class="manas" src="../images/mascot/manas_mascot_success.png" title="this is the correct, working version"/>

```vani
fn double_all(xs: mut ref Vec<i64>) -> i64 {
  let n: u64 = len(xs);
  parallel for i from 0 to n {
    xs[i] = xs[i] * 2;
  }
  return 0;
}
```

Each iteration writes a *distinct* slot in `xs`. The compiler
proves the slots don't alias (each iteration writes `xs[i]`
where `i` ranges over the iteration variable's domain) -- the
write index must be syntactically exactly the loop's own index
variable, and the value being written must not itself read `xs`
at any OTHER index (that would be the `bad_prefix` hazard above,
just hidden on the read side instead of the write side).

## Challenge

Write `dot_product(a: ref Vec<i64>, b: ref Vec<i64>) -> i64`
that returns Sigma ai*bi. Use `parallel for` with `reduce sum with
+`. Assume `len(a) == len(b)`.

<details>
<summary>Solution</summary>

```vani
fn dot_product(a: ref Vec<i64>, b: ref Vec<i64>) -> i64
requires len(a) == len(b);
{
  let n: u64 = len(a);
  let sum: i64 = 0;
  parallel for i from 0 to n
  reduce sum with +;
  {
    sum = sum + a[i] * b[i];
  }
  return sum;
}
```

</details>

---

## Bare-metal / RTOS note

<img class="manas" src="../images/mascot/manas_mascot_caution.png" title="this code needs extra care"/>

`parallel for … reduce` emits `pthread_create` (POSIX) or `CreateThread`
(Windows). Neither symbol exists on bare-metal targets such as
`arm-none-eabi` or `thumbv7em-none-eabihf`. Linking for those targets will
fail with an undefined-reference error.

**Workaround — manual work split via FreeRTOS:**

```vani
// Split the range by hand and create one task per slice.
// The FreeRTOS xTaskCreate symbol is available via FFI:
extern fn xTaskCreate(f: fn() -> i64, name: ref i8, stack: i64,
                      arg: i64, prio: i64, handle: i64) -> i64;

fn worker_0() -> i64 {
    // process a[0..n/2]
    return 0;
}
fn worker_1() -> i64 {
    // process a[n/2..n]
    return 0;
}
fn main() -> i64 {
    let _ = xTaskCreate(worker_0, ref "w0" as i8, 256, 0, 1, 0);
    let _ = xTaskCreate(worker_1, ref "w1" as i8, 256, 0, 1, 0);
    // ... vTaskStartScheduler() via FFI to begin execution
    return 0;
}
```

For single-core bare-metal (Cortex-M0/M0+), the entire premise of
parallel execution doesn't apply — run the loop body sequentially and
use DMA for true offload. `parallel for` is a POSIX/Win32 feature;
use it only when linking against a hosted OS.

## Windows thread-count note

<img class="manas" src="../images/mascot/manas_mascot_caution.png" title="this code needs extra care"/>

On Linux/macOS, `parallel for`'s worker-thread count is decided
by libgomp (GNU OpenMP) at **run time**, on whichever machine
executes the binary — it scales to that machine's actual core
count automatically, or honors `OMP_NUM_THREADS` if you set it
before running the binary.

On Windows, the LLVM backend hand-rolls its own `CreateThread`
dispatch instead of delegating to an OpenMP runtime, and that
path resolves the thread count once, **at `vanic build`/`vanic
run` time** — from `OMP_NUM_THREADS` if set at build time, else
from the *build machine's* core count — and bakes it into the
binary as a fixed constant. Build on an 8-core Windows laptop,
copy the binary to a 32-core Windows server, and it still only
ever spawns 8 worker threads.

**Workaround**: set `OMP_NUM_THREADS` to the *target* machine's
core count before running `vanic build`, or rebuild on (or for)
the machine the binary will actually run on. This is a v1
limitation, not a language guarantee — see
[L24 in v1_limitations.md](https://github.com/enthusiasticgeek/vani-compiler/blob/main/docs/v1_limitations.md)
for the fix path (a runtime `GetActiveProcessorCount` call would
close this the same way `GOMP_parallel` already does on POSIX).

---

**Previous**: [Sec.2c -- RwLock: shared reads, exclusive writes primer ->](02c_rwlock_primer.md)
**Next**: [Sec.3 -- `task` / `join` + atomics / mutexes / channels ->](03_concurrency.md)
