# Advanced 2 — `parallel for` + reductions + race-freedom

> **Learning goal**: turn a sequential loop into a `parallel
> for`, declare a `reduce` accumulator, and understand how the
> affine type system proves race-freedom at compile time.

> **New to this?** Read [Advanced 2a — Parallelism and race-freedom primer](02a_parallelism_primer.md) first.

Imagine ten accountants each adding up a separate stack of
receipts simultaneously — each one works on their own stack,
never touching anyone else's, and at the end a supervisor
adds up all their sub-totals. No two accountants can possibly
interfere because the stacks are separate. That's a parallel
reduction: split the work into non-overlapping pieces, do them
simultaneously, combine the results. `parallel for ... reduce`
expresses this pattern; the compiler verifies at compile time
that loop iterations don't share writeable data (race-freedom
— no accountant steals from another's stack mid-tally).

## The program

```vani
intent "Advanced 2 worked example — parallel for + reduction.";

fn main() -> i64 {
  // Sum 1..100 sequentially first as the reference.
  let seq: i64 = 0;
  for i from 1 to 101 {
    seq = seq + i;
  }
  print "seq =", seq;

  // The same loop with parallel for + reduce. The compiler
  // proves the body has no inter-iteration data dependence
  // (affine ownership + reduce clause) and emits OpenMP
  // `reduction(+: total)` on the C backend, atomicrmw add on
  // LLVM.
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

## Why it works that way

- **Sequential `for i from lo to hi { ... }`** is the baseline.
  Each iteration runs in order; mutations to captured
  variables (`seq = seq + i`) commit before the next iteration.
- **`parallel for`** lifts the body to run on N threads. The
  compiler **statically rejects** loops whose body has
  iteration-to-iteration data dependencies — assignments to
  array slots, mutable captures without a `reduce` clause, etc.
  This is the safety boundary.
- **`reduce <var> with <op>;`** declares an accumulator variable
  that survives across iterations. The compiler emits OpenMP's
  `reduction(<op>: <var>)` clause on the C backend, or an
  `atomicrmw <op>` on LLVM. Per-thread partial sums combine to
  the final value at the loop's barrier.
- **Race-freedom is proved**, not asserted. The combination of
  affine ownership (each binding has one owner), the explicit
  `reduce` declaration, and the no-iteration-dependence check
  means a `parallel for` that compiles is guaranteed
  data-race-free.

## What's allowed in a `parallel for` body

| Allowed | Rejected |
|---|---|
| Reads of captured `let` bindings | Writes to non-reduce captures |
| Writes through `mut ref` parameters | Indexed assignment `xs[i] = …` on a captured Vec |
| Calls to pure fns | Calls to impure fns (FFI, IO) without `unsafe` |
| `reduce var with +`/`*`/`max`/`min` | Multiple `reduce` clauses on the same variable |

## Mapping a Vec without a reduce

If you want a transform-each-element pattern (no reduction),
the safe v1 form is a `mut ref Vec` + index-by-iteration:

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
where `i` ranges over the iteration variable's domain).

## Challenge

Write `dot_product(a: ref Vec<i64>, b: ref Vec<i64>) -> i64`
that returns Σ aᵢ·bᵢ. Use `parallel for` with `reduce sum with
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

**Next**: [§3 — `task` / `join` + atomics / mutexes / channels →](03_concurrency.md)
