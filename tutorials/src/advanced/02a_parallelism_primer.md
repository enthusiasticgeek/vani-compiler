# Advanced 2a -- Parallelism and race-freedom (intuition primer)

> **Learning goal**: build a mental model of "parallel execution"
> and the specific bug class -- **data races** -- that vāṇī
> rejects at compile time. This is the second of vāṇी's two
> "compiler proves it can't go wrong" stories (SMT being the
> first; see [12a SMT primer](../intermediate/12a_smt_primer.md)).
> Reading order: [06c ownership primer](../beginner/06c_ownership_primer.md)
> -> here -> [Advanced 2 parallel for](02_parallel.md).

This chapter is mostly intuition, with real `parallel for` code
once the analogy lands.

## The restaurant kitchen

Picture a small restaurant kitchen with one cook and a stack of
dinner tickets. The cook works through the tickets one at a time:
chop vegetables for ticket 1, sear the meat for ticket 1, plate
ticket 1, then start ticket 2 from scratch. It works. Nothing ever
goes wrong, because there's only ever one pair of hands touching
anything -- the cutting board, the pan, the plate. But it's slow.
A rush of twenty tickets means twenty tickets' worth of waiting,
one after another, no matter how many empty stations sit unused
around the kitchen.

Now hire three more cooks. Give ticket 1 to cook A, ticket 2 to
cook B, ticket 3 to cook C, ticket 4 to cook D. If each cook has
their own cutting board, their own pan, and their own corner of the
counter, all four tickets get made at the same time and the whole
rush finishes in a quarter of the time. That's the entire appeal of
having more cooks: work that doesn't depend on each other can
happen at the same time instead of waiting in line.

But now suppose the kitchen only owns ONE good chef's knife, and
both cook A and cook C reach for it at the same instant to start
chopping. Whoever's hand gets there first wins -- except sometimes
neither notices the other already has it, and you get two cooks
mid-chop on the same board, elbows colliding, a customer's onions
ending up in someone else's dish. Nobody planned for this collision;
it just happens when two cooks touch the same shared thing at the
same moment with no rule for who goes first. The dish that comes out
of that collision is wrong, and it's wrong in a way that only
happens sometimes -- on a slow night the two cooks might never reach
for the knife at the exact same second, so the kitchen "seems fine"
right up until the rush when it isn't.

A well-run multi-cook kitchen avoids this one of two ways: either
every cook gets their OWN full set of tools so nobody ever needs to
reach for what someone else is using, or -- when a tool genuinely
has to be shared, like the one good knife, or the walk-in fridge --
the kitchen has a clear rule for who gets it and when (a hook by the
door: take the knife off the hook, use it, hang it back up before
the next cook can take it). Either solution works. What doesn't work
is four cooks and one knife with no rule at all.

Now map this onto code: a cook is a **thread** (an independent
stream of execution running on its own CPU core); working through
tickets one at a time is a **sequential program**; hiring more cooks
to work different tickets simultaneously is **parallelism**; two
cooks colliding over the same knife with no rule is a **data race**
(two threads reading and writing the same memory at the same moment
with no coordination); and giving each cook their own tools, or
putting a clear hand-off rule on the one shared tool, is
**synchronization** -- exactly the ownership rules and concurrency
primitives (`Atomic`, `Mutex`, `Channel`) this chapter is about to
walk through.

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
  let r: i64 = expensive_compute(i);   // must be `pure fn` -- see below
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
the same slot. Safe. (`expensive_compute` itself must be declared
`pure fn` -- a `parallel for` body can only call pure functions,
confirmed by testing; calling an ordinary function is rejected the
same way a `task` body rejects one, below.)

If the body did `results[0] = ...` for every iteration -- REJECTED;
they all clobber the same slot, mathematically impossible to
parallelize correctly.

## Reductions -- combining results across iterations

What if you want to SUM all the values?

```vani
let sum: i64 = 0;
parallel for i from 0 to 1000 reduce sum with +;
{
  sum = sum + expensive_value(i);
}
```

(the `;` after `reduce sum with +` is required -- confirmed by
testing, the loop doesn't parse without it.)

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
  let _ = sleep_ms(20);   // stand-in for a real download
}
task download_b {
  let _ = sleep_ms(10);   // stand-in for a real download
}

// ... do other things in the main thread ...

join download_a;
join download_b;
print "both downloads done";
```

Confirmed by testing -- and worth calling out, since it's easy to
get wrong by analogy with other languages: a `task { ... }` body is
checked with the SAME purity rules as a `parallel for` body (no
`print`, no calling a non-`pure` user-defined function -- even a
one-line wrapper around a builtin gets rejected). It can call
*builtin* I/O primitives directly (`sleep_ms`, the `tcp_*`/
`io_*_async` family, ...), which is what makes `task` useful for
real concurrent I/O at all -- but there is no way to route that I/O
through your own helper function; the builtin call has to be
written literally inside the `task` body. This is why the two tasks
above call `sleep_ms` inline rather than through a `fetch`-style
wrapper, and why `print` moved to after `join`.

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


---

**Previous**: [Sec.1 -- Async / await and the Task transform ->](01_async.md)
**Next**: [Sec.2b -- Barrier: rendezvous synchronization primer ->](02b_barrier_primer.md)

