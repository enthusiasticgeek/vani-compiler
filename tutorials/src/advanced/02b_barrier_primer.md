# Advanced 2b -- Barrier: rendezvous synchronization (primer)

> **Learning goal**: understand *why* you sometimes need all threads
> to reach the same point before any of them can proceed, and how
> `Barrier` in vāṇī gives you that guarantee with no data races.
> Reading order: [02a parallelism primer](02a_parallelism_primer.md)
> -> here -> [Advanced 3 concurrency](03_concurrency.md).

This chapter leads with intuition, then a one-page API with real
code.

## The problem: phase-by-phase parallel work

Imagine you are directing a team of 4 chefs preparing a feast.
The feast has two distinct phases:

1. **Phase 1** -- each chef prepares their own dish independently
   (no coordination needed; fully parallel).
2. **Phase 2** -- all four dishes are plated together; the sous-chef
   can't start plating until every dish is ready.

If Chef A finishes in 2 minutes and runs straight to phase 2,
they'll try to plate three empty places -- wrong. Chef A must
*wait at the counter* until all four chefs finish phase 1, then
everyone can move to phase 2 together.

That waiting-at-the-counter checkpoint is exactly what a **Barrier**
implements.

```
Thread A --------------------- Phase 1 work ------- WAIT --+
Thread B ----------- Phase 1 work ---------------- WAIT ---+ ALL HERE
Thread C ---------------------------- Phase 1 work - WAIT -+  v
Thread D - Phase 1 work -------------------------- WAIT ---+  v
                                                        ALL PROCEED to Phase 2
```

Every thread blocks on `barrier_wait` until the last thread
arrives. The moment the last thread arrives, all threads
simultaneously unblock and continue.

## Why not just `join` all the tasks?

`join` collects a task's *return value* back into the main thread.
For phase-by-phase work, you want ALL threads to continue past
the barrier -- including the threads that were already fast. `join`
would force everything back to the main thread and destroy the
parallelism of phase 2.

A Barrier lets threads do:

```
Thread A:  phase_one() -> barrier_wait() -> phase_two()   <- stays alive
Thread B:  phase_one() -> barrier_wait() -> phase_two()   <- stays alive
Thread C:  phase_one() -> barrier_wait() -> phase_two()   <- stays alive
```

With just `join`, the flow would be:

```
Thread A:  phase_one() -> EXIT -> (main joins, starts new thread) -> phase_two()
Thread B:  phase_one() -> EXIT -> ...
```

Extra thread spawn/join overhead per phase, plus the parallelism
gap between teardown and re-spawn.

## The "last to arrive" signal

`barrier_wait` returns a `bool` -- `true` for the **last** thread
to arrive, `false` for all others. This lets you do a final
single-thread cleanup step before everyone proceeds:

```vani
let is_last: bool = barrier_wait(mut ref b);
if is_last {
  // Only one thread runs this -- perfect for writing a summary,
  // flipping a flag, or triggering the next pipeline stage.
  print "All threads finished phase 1";
}
// ALL threads continue here (including the one that ran the block above)
```

## Safety: the generation counter

A Barrier is safe to *reuse* in a loop without resetting it.
Internally it uses a generation counter. When the N-th thread
arrives:

1. The counter flips to the next generation.
2. All waiting threads are woken.

Any thread that calls `barrier_wait` again immediately goes to
the *new* generation's waiting list. There's no window where a
fast thread can re-enter the old generation and accidentally
unblock it twice. The technical name for this bug is an **ABA
race**; the generation counter prevents it.

## The API (one screen)

```vani
// Create a Barrier for n threads.
let b: Barrier = barrier_new(n);

// Each participating thread calls this.
// Blocks until all n threads have called barrier_wait on this barrier.
// Returns true for the last thread to arrive, false for all others.
let is_last: bool = barrier_wait(mut ref b);
```

`Barrier` is affine (one owner). Pass it around as `mut ref Barrier`
so every thread can call `barrier_wait` on the same object.

## Worked example

```vani
intent "Barrier primer -- two-phase parallel work.";

fn phase_one(id: i64, b: mut ref Barrier) -> i64 {
  // Simulate work -- in a real program this would be computation.
  print "thread", id, "finished phase 1";
  let is_last: bool = barrier_wait(b);   // b is already `mut ref Barrier` here
  if is_last {
    print "All threads at barrier -- starting phase 2";
  }
  // All threads proceed here.
  print "thread", id, "starting phase 2";
  return 0;
}

fn main() -> i64 {
  let b: Barrier = barrier_new(3);
  let t1: Task<i64> = task phase_one(1, mut ref b);
  let t2: Task<i64> = task phase_one(2, mut ref b);
  let _ = phase_one(3, mut ref b);   // main thread is the third participant
  let _ = join t1;
  let _ = join t2;
  return 0;
}
```

Sample output (ordering within phase 1 varies by schedule):

```
thread 2 finished phase 1
thread 1 finished phase 1
thread 3 finished phase 1
All threads at barrier -- starting phase 2
thread 3 starting phase 2
thread 1 starting phase 2
thread 2 starting phase 2
```

(Verified against the real compiler on both backends -- the barrier
semantics hold exactly: every "finished phase 1" line precedes "All
threads at barrier," and every "starting phase 2" line follows it,
on every run. The exact byte layout above can differ run to run, and
individual lines can even interleave mid-line, since `print` calls
from different threads aren't synchronized against each other --
only `barrier_wait` itself is.)

## When to use Barrier vs alternatives

| Need | Use |
|---|---|
| All N threads reach a checkpoint before any proceed | `Barrier` |
| One thread waits for another to finish entirely | `join` |
| Wait until a specific condition holds | `Condvar` + `Mutex` |
| One thread signals another event | `Channel` (send a token) |

Barrier is the right tool when you have **N equal participants**
and need a **synchronous rendezvous** at a phase boundary. If
the roles are asymmetric (one producer, many consumers), reach
for `Channel` or `Condvar` instead.

## Cross-reference

- [Advanced 2a -- Parallelism primer](02a_parallelism_primer.md)
  -- race-freedom background; why ownership prevents data races
- [Advanced 3 -- concurrency primitives](03_concurrency.md)
  -- full API reference for all six primitives including Barrier
- [Advanced 2 -- `parallel for` + reductions](02_parallel.md)
  -- data-parallel alternative (no explicit synchronization needed)


---

**Previous**: [Sec.2a -- Parallelism and race-freedom primer ->](02a_parallelism_primer.md)
**Next**: [Sec.2c -- RwLock: shared reads, exclusive writes primer ->](02c_rwlock_primer.md)

