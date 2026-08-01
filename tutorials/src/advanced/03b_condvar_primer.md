# Advanced 3b -- Condition variables: wait-until-predicate (primer)

> **Learning goal**: understand why condition variables exist,
> what the wait-loop idiom guarantees, and how `Condvar` pairs
> with `Mutex<T>` in vāṇी.
> Reading order: [Advanced 3 -- Concurrency](03_concurrency.md)
> -> here -> [Advanced 4a -- Embedded primer](04a_embedded_primer.md).

This chapter has **no compiler code**. Pure intuition, then the
one-page API reference.

---

## The problem: "wait until something is true"

A `Mutex<T>` lets threads *share* a value safely. But what if
one thread needs to **wait** until another thread changes that
value to meet a condition?

```
Producer thread:  fill the buffer, then signal "buffer ready"
Consumer thread:  wait until "buffer ready", then drain it
```

You could busy-spin:

```
while !buffer_ready { /* spin */ }
```

But that wastes CPU cycles and makes the scheduler's job harder.
What you want is: *park the thread* (zero CPU) until a signal
arrives, then wake up and check.

That's exactly what a **condition variable** does.

---

## The mental model: a ticket queue

Think of the condition variable as a **waiting room** outside a
locked office (the mutex):

```
Thread 1 (waiter):
  1. Enter the office (lock the mutex).
  2. Check: is the work ready?  -> No.
  3. Step into the waiting room (release mutex + park thread).
     <- zero CPU; thread is suspended
  4. Door opens: notification arrives, thread wakes.
  5. Re-enter the office (re-acquire mutex).
  6. Check again: is the work ready?  -> Yes.
  7. Do the work, leave the office.

Thread 2 (producer):
  1. Enter the office, do work, set the flag.
  2. Ring the bell (notify_one / notify_all).
  3. Leave the office (unlock mutex).
```

The critical design point: **wait atomically releases the mutex
and parks the thread**. There is no window between "release lock"
and "park" where a notification could be missed.

---

## Spurious wakeups -- why you always use a loop

On some OS implementations, a thread can wake up from a condvar
wait for *no reason* -- this is called a **spurious wakeup**. It's
a real phenomenon, not just a theoretical concern.

This is why the wait-loop idiom is always:

```
while !predicate {
    condvar_wait(ref cv, mut ref guard);
}
```

<img class="manas" src="../images/mascot/manas_mascot_caution.png" title="this code needs extra care"/>

Never `if !predicate { condvar_wait(...); }`. The `while` re-checks
after every wakeup (spurious or real) and only proceeds when the
predicate is actually true.

---

## The vāṇī API

### Types

| Type | Role |
|------|------|
| `Condvar` | The waiting-room object. Not tied to a specific mutex type. |
| `Mutex<T>` | The lock protecting the shared predicate state. |
| `Guard<T>` | The live lock token returned by `mutex_lock`. |

### Builtins

| Builtin | Signature | Description |
|---------|-----------|-------------|
| `condvar_new` | `() -> Condvar` | Create a condvar |
| `condvar_wait` | `(ref cv, mut ref guard: Guard<T>)` | Release mutex, park thread, re-acquire on wake |
| `condvar_wait_timeout` | `(ref cv, mut ref guard: Guard<T>, ms: i64) -> bool` | Same, but wakes after `ms` milliseconds if no signal; returns `false` on timeout |
| `condvar_notify_one` | `(ref cv)` | Wake exactly one waiter (if any) |
| `condvar_notify_all` | `(ref cv)` | Wake all waiters |

### Pattern

```vani
let cv:  Condvar   = condvar_new();
let mx:  Mutex<i64> = mutex_new(0);   // 0 = not ready

// --- Waiter thread ---
// (`mutex_lock` takes `ref`, not `mut ref` -- unlike rwlock_read/write,
// which DO need `mut ref`; a Guard's value is read/written through
// guard_get/guard_set, never mutex_get/mutex_set, which don't exist.)
{
    let g: Guard<i64> = mutex_lock(ref mx);
    while guard_get(ref g) == 0 {          // "not ready"
        condvar_wait(ref cv, mut ref g);
    }
    // predicate is now true
    let value: i64 = guard_get(ref g);
}   // Guard drops here -> mutex released automatically (RAII, no manual unlock)

// --- Producer thread ---
{
    let g2: Guard<i64> = mutex_lock(ref mx);
    let _ = guard_set(mut ref g2, 42);     // set value
}   // Guard drops here -> mutex released automatically
let _ = condvar_notify_one(ref cv);        // wake the waiter
```

### Notify after unlock

The example above unlocks before notifying. Some implementations
allow notify while holding the lock (it's not wrong), but
unlocking first avoids a priority-inversion where the waiter
wakes up, tries to acquire the mutex, and immediately blocks again
because the notifier still holds it. Unlock-then-notify is the
safer habit.

---

## `wait_timeout` -- bounded waiting

```vani
let cv: Condvar   = condvar_new();
let mx: Mutex<i64> = mutex_new(0);
{
    let g:  Guard<i64> = mutex_lock(ref mx);

    // Wait at most 200 ms for the predicate
    let signaled: bool = condvar_wait_timeout(ref cv, mut ref g, 200);
    if !signaled {
        print "timeout -- no event in 200 ms";
    }
}   // Guard drops here -> mutex released automatically
```

(Verified end to end against the real compiler, both backends --
`vanic run --backend=c` runs this through a real `cc` invocation, not
just the checker.)

`condvar_wait_timeout` returns `true` if a notification arrived
before the timeout, `false` if the deadline elapsed.

---

## When NOT to use a condvar

| Situation | Better tool |
|-----------|------------|
| One thread produces, one consumes, fixed-size buffer | `Channel<T, N>` (simpler API) |
| Just counting pending tasks | `Atomic<i64>` + busy-check in low-contention cases |
| Thread-pool work dispatch | `Channel` queue is cleaner |
| Single notify at shutdown | `Atomic<bool>` + polling loop in practice |

Use `Condvar` when: the predicate is complex (multiple fields), the waiting thread needs to inspect the shared state while holding the lock, or you need `notify_all` to wake every waiter simultaneously.

---

## Summary

- A **condvar** lets a thread park (zero CPU) until another
  thread signals that a predicate may now be true.
- **Always use a `while` loop** around `condvar_wait` -- spurious
  wakeups are real.
- `condvar_wait` **atomically** releases the mutex and parks;
  re-acquires before returning.
- `condvar_notify_one` wakes one waiter; `condvar_notify_all`
  wakes all.
- `condvar_wait_timeout` adds a deadline; returns `false` on timeout.

## Cross-reference

- [Advanced 3 -- `task`/`join` + atomics/mutexes/channels](03_concurrency.md) -- the full concurrency worked-example chapter
- [Advanced 2a -- Parallelism primer](02a_parallelism_primer.md) -- race-freedom model
- [`examples/language/english/condvar.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/english/condvar.vani) -- runnable condvar example


---

**Previous**: [Sec.3 -- task / join + atomics / mutexes / channels ->](03_concurrency.md)
**Next**: [Sec.4a -- Embedded, unsafe, and regions primer ->](04a_embedded_primer.md)

