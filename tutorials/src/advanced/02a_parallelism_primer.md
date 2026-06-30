# Advanced 2a -- Parallelism and race-freedom (intuition primer)

> **Learning goal**: build a mental model of "parallel execution"
> and the specific bug class -- **data races** -- that vāṇī
> rejects at compile time. This is the second of vāṇी's two
> "compiler proves it can't go wrong" stories (SMT being the
> first; see [12a SMT primer](../intermediate/12a_smt_primer.md)).
> Reading order: [06c ownership primer](../beginner/06c_ownership_primer.md)
> -> here -> [Advanced 2 parallel for](02_parallel.md).

This chapter has **no compiler code**. Pure intuition.

## Why parallelism?

Modern CPUs have many cores. A 2026-era laptop has 8-16 cores;
a server has 64+. A sequential program uses ONE core. Run a
parallel program and you can multiply your speed by however
many cores you have -- for problems that decompose well.

Examples that parallelize naturally:
- Compute `f(x)` for each x in a million-element array.
- Sum a billion numbers.
- Resize 10,000 images.
- Run 1000 simulations with different seeds.

Examples that DON'T:
- Anything with a strict order ("step N depends on step N-1").
- Anything where shared state is mutated chaotically.

vāṇी helps you exploit the parallelizable shapes and rejects
the bug-prone non-parallelizable shapes at compile time.

## The bug class to fear: data races

A **data race** is what happens when two threads read AND
write the same memory location without coordination. Concrete
example:

```
Thread A: counter = counter + 1
Thread B: counter = counter + 1
```

You'd think `counter` ends up two higher. It might. But also it
might end up only one higher. Or worse, in some languages the
read-modify-write isn't atomic at the assembly level -- you
could get a partial update that's mathematically impossible.

The reason: `counter = counter + 1` is THREE operations:
1. Read counter (say, into a CPU register).
2. Add 1.
3. Write the result back.

If Thread A is partway through (1)-(3) and Thread B starts its
own (1)-(3), they're both reading the OLD value, adding 1, and
writing back. One write wins; the other is lost. Final result:
+1 instead of +2.

This is the *canonical* concurrency bug. It's hard to debug
because:
- It's TIMING-dependent -- runs fine 99% of the time.
- It varies by CPU, by load, by phase of the moon.
- It produces subtle wrong results, not crashes.

Languages handle data races in three broad ways:

1. **Hope** (C, C++ before C++11): the language doesn't help.
   You're on your own. Most production-quality C programs
   have data races somewhere; some are dormant for years.

2. **Lock everything** (Python's GIL, Java's `synchronized`
   convention): force serialization via locks. Easy to reason
   about, kills parallelism.

3. **Compile-time prevention** (vāṇी, Rust): the compiler
   rejects code that COULD race. Programs that compile can't
   data-race. Period.

vāṇी picks (3).

## How the compiler rejects races

The key insight: **a data race requires SHARED MUTABLE STATE
ACROSSED MULTIPLE THREADS**. Eliminate any of those three --
shared, mutable, multi-thread -- and races become impossible.

vāṇī's compiler tracks ownership (covered in the [ownership
primer](../beginner/06c_ownership_primer.md)). Each value has
exactly one owner; no two threads can both think they own the
same value. To share a value across threads, you must use a
specific concurrency primitive:

- `Atomic<T>` -- for small scalar values; the operations are
  hardware-atomic.
- `Mutex<T>` + `Guard<T>` -- for any T; the lock serializes
  access.
- `Channel<T, N>` -- for moving values between threads; no
  shared state at all.

If you try to share a plain Vec or struct between threads, the
compiler rejects with a clear "this type isn't thread-safe"
diagnostic. You either:
1. Wrap it in `Mutex<T>` to add the lock discipline, OR
2. Move it (not share) -- let only one thread own it at a time
   and pass ownership via a channel.

Either choice gives you race-free code.

## `parallel for` -- the easiest pattern

The simplest way to use parallelism in vāṇी is the parallel
for loop:

```vani
parallel for i from 0 to 1000 {
  let r: i64 = expensive_compute(i);
  results[i] = r;
}
```

The compiler:
1. Splits the iteration range across available CPU cores.
2. Runs the body for each `i` in parallel.
3. Verifies that each iteration doesn't read state written by
   another iteration.

The verification is the load-bearing piece. The compiler walks
the body and checks: "does iteration K read a memory location
that iteration M writes?" If yes -- REJECTED. If no -- safe to
parallelize.

For the example above, each iteration writes to `results[i]`
where `i` is unique to the iteration -- no two iterations write
the same slot. Safe.

If the body did `results[0] = ...` for every iteration -- REJECTED;
they all clobber the same slot, mathematically impossible to
parallelize correctly.

## Reductions -- combining results across iterations

What if you want to SUM all the values?

```vani
let sum: i64 = 0;
parallel for i from 0 to 1000 reduce sum with + {
  sum = sum + expensive_value(i);
}
```

`reduce sum with +` tells the compiler: each iteration
contributes to `sum`; combine all contributions using `+`.
The compiler then knows how to safely parallelize:

1. Each thread maintains its own LOCAL `sum`.
2. After all iterations complete, the locals are combined
   via `+`.

`+` is **associative** (`(a+b)+c = a+(b+c)`), so the order of
combination doesn't change the result. The compiler enforces
that the reduction operator is associative -- `+`, `*`, `min`,
`max`, `&`, `|`, `^`, `&&`, `||` are all supported.

## Thread-per-task -- `task` and `join`

For coarser-grained parallelism (running a few things in
parallel rather than thousands), use `task` + `join`:

```vani
task download_a {
  let data: OwnedStr = fetch("a");
  // do something with data
}
task download_b {
  let data: OwnedStr = fetch("b");
  // do something with data
}

// ... do other things in the main thread ...

join download_a;
join download_b;
```

Each `task` body runs in parallel on its own OS thread. `join`
waits for it to complete. Variables CAPTURED by the task body
follow the same ownership + race rules -- if both tasks try to
read AND write the same shared variable, the compiler rejects.

Tasks are vāṇी's parallel-for-loops counterpart: parallel for
is data-parallel (many uniform iterations); tasks are
task-parallel (a few distinct chunks of work).

## What the compiler CAN'T parallelize automatically

The compiler is conservative -- when in doubt, it rejects. It
can't:

- **Prove arbitrary loop iterations are independent**. Simple
  shapes (`results[i] = f(i)`) yes; tangled shapes (`results[i] =
  results[i-1] + 1`) no.
- **Reason about external side effects**. A loop that prints
  to stdout in each iteration WOULD parallelize fine but the
  output order would interleave; the compiler treats this as
  ambiguous and rejects.
- **Handle non-associative reductions**. `reduce x with -`
  (subtraction) is rejected because `(a-b)-c != a-(b-c)` --
  the result would depend on combination order.

When the compiler rejects, the diagnostic explains exactly
which iteration-pair could race and on which variable. You
then refactor (often into the reduction form) or accept that
this particular loop has to stay sequential.

## A summary you can carry

- **Data races** = the bug class where two threads concurrently
  read+write the same memory without coordination.
- vāṇी eliminates them at COMPILE TIME by tracking ownership
  + requiring explicit concurrency primitives (`Atomic`,
  `Mutex`, `Channel`) for shared mutable state.
- **`parallel for`** is the easiest parallelism -- the compiler
  proves iterations are independent and parallelizes
  automatically.
- **Reductions** (`reduce x with +/*/min/max/&/|/^/&&/||`)
  combine per-iteration contributions safely; reduction
  operators must be associative.
- **`task` + `join`** is for coarse-grained parallelism -- a
  few distinct chunks of work, each on its own OS thread.

This pairs with the SMT primer ([12a](../intermediate/12a_smt_primer.md))
as vāṇी's two "compiler proves it can't go wrong" stories:
SMT for arithmetic/logic correctness; ownership + concurrency
typing for memory + race safety. Together, the class of bugs
that survive compilation is dramatically narrower than in any
mainstream language.

The next chapter ([Advanced 2](02_parallel.md)) shows the
syntax + worked examples; [Advanced 3](03_concurrency.md)
covers atomics / mutexes / channels in detail.

## Cross-reference

- [Beginner 6c -- Ownership and move primer](../beginner/06c_ownership_primer.md)
  -- the foundation: one-owner-at-a-time is what makes
  race-freedom possible
- [Intermediate 12a -- SMT primer](../intermediate/12a_smt_primer.md)
  -- vāṇी's OTHER compile-time-proves-it story
- [Advanced 2 -- `parallel for` + reductions + race-freedom](02_parallel.md)
  -- syntax + worked examples
- [Advanced 3 -- task / join / atomics / mutexes / channels](03_concurrency.md)
  -- coarse-grained parallelism
