# Advanced 2c -- RwLock: shared reads, exclusive writes (primer)

> **Learning goal**: understand when you want *many readers or one
> writer* (not just one-at-a-time like `Mutex`), and how `RwLock<T>`
> gives you that with compile-time RAII safety.
> Reading order: [Advanced 3 concurrency](03_concurrency.md) (for
> the full primitive survey) or independently after the
> [02a parallelism primer](02a_parallelism_primer.md).

This chapter has **no compiler code**. Intuition first, then the
one-page API.

## The whiteboard problem

Picture a whiteboard in a shared office. Fifty engineers walk by
and glance at it throughout the day. Occasionally, one person
erases it and writes a new diagram.

Two rules make this work without chaos:

1. **Any number of people can READ at the same time** -- reading
   doesn't change anything, so there's no interference.
2. **Exactly one person can WRITE** -- and only when nobody else
   is reading. You can't draw a new box while someone is trying
   to read the board; they'd see half the old diagram and half
   the new one.

This is the **reader-writer lock** pattern: many shared readers
OR one exclusive writer, never both simultaneously.

A `Mutex` gives you the simpler version: one thread at a time,
period -- readers AND writers both block each other. For a
read-heavy workload (config table, routing table, metric cache),
a `Mutex` creates unnecessary contention. Threads that only want
to read are forced to queue up one-by-one even though they would
never interfere.

## State encoding

vāṇী's `RwLock<T>` uses a signed integer to encode three states:

| State | Counter value |
|---|---|
| Unlocked | `0` |
| N concurrent readers | positive integer N |
| One exclusive writer | `-1` |

Acquiring a read lock increments the counter (succeeds unless
the counter is `-1`). Acquiring a write lock sets it to `-1`
(succeeds only when the counter is `0`). This keeps the
implementation lock-free for the fast path (uncontended reads).

## RAII: guards release automatically

You never call `rwlock_unlock` directly. Instead:

- `rwlock_read` returns a `ReadGuard<T>` -- a scoped "I am reading"
  ticket.
- `rwlock_write` returns a `WriteGuard<T>` -- a scoped "I am
  writing" ticket.

When the guard goes out of scope, the RAII destructor calls the
appropriate unlock automatically. This prevents the classic
"forgot to unlock" bug -- if a function returns early or the code
panics, the lock is still released.

Both guard types are **affine** in vāṇī -- each guard has exactly
one owner. The compiler ensures you can't copy a guard and
accidentally hold the lock in two places at once.

## The API (one screen)

```vani
// Create an RwLock wrapping an initial value.
let rw: RwLock<i64> = rwlock_new(0);

// Acquire a shared read lock. Note: both rwlock_read AND
// rwlock_write take `mut ref` -- acquiring even a READ lock
// mutates the lock's internal reader-count, so the handle itself
// must be mutably borrowed even though you're only reading the
// payload.
// Many threads can hold a ReadGuard simultaneously.
let r: ReadGuard<i64> = rwlock_read(mut ref rw);
let v: i64 = read_guard_get(ref r);
// r goes out of scope here -> read lock released automatically

// Acquire an exclusive write lock.
// Blocks until all current readers have released.
let w: WriteGuard<i64> = rwlock_write(mut ref rw);
let _ = write_guard_set(mut ref w, v + 1);
// w goes out of scope here -> write lock released automatically
```

`T` can genuinely be any type, on **either** backend -- `i64`,
`bool`, other integer widths, a struct, an enum, `Vec<T>` all work
end to end (`rwlock_new`, read/write acquire, get/set, and the
automatic release on scope exit), including when the type appears as
a function parameter (`fn f(rw: mut ref RwLock<Config>) -> i64`).
Each element type gets its own correctly-sized generated struct, the
same way `Mutex<T>`, `Channel<T, N>`, and `Vec<T>` already do.

## When to use RwLock vs Mutex

| Shape | Use |
|---|---|
| Reads far outnumber writes AND reads don't block each other | `RwLock<T>` |
| Reads and writes are roughly equal OR the payload is tiny | `Mutex<T>` (simpler, same overhead for writes) |
| You only need atomic increment / flag flip | `Atomic<T>` (no lock at all) |

Common use cases for `RwLock`:
- **Config / routing table** -- loaded once, read millions of
  times, updated rarely.
- **Metric snapshot** -- many readers poll; one collector updates
  every second.
- **Cache** -- many cache hits (reads); occasional miss fills
  (writes).

## Writer starvation

One thing to watch: if readers arrive continuously, a writer
waiting to acquire the write lock might wait indefinitely
(readers never fully drain). vāṇī's v1 `RwLock` does NOT
implement writer-priority blocking. For write-heavy workloads,
use `Mutex<T>` instead.

## Worked example

<img class="manas" src="../images/mascot/manas_mascot_success.png" title="this is the correct, working version"/>

```vani
intent "RwLock primer -- shared timeout setting.";

fn read_timeout(rw: mut ref RwLock<i64>) -> i64 {
  let r: ReadGuard<i64> = rwlock_read(rw);
  let ms: i64 = read_guard_get(ref r);
  // r drops here (end of fn) -- read lock released automatically
  return ms;
}

fn update_timeout(rw: mut ref RwLock<i64>, new_ms: i64) -> i64 {
  let w: WriteGuard<i64> = rwlock_write(rw);
  let _ = write_guard_set(mut ref w, new_ms);
  // w drops here (end of fn) -- write lock released automatically
  return 0;
}

fn main() -> i64 {
  let rw: RwLock<i64> = rwlock_new(5000);

  // Two "readers" -- each acquires, reads, and releases before
  // the next runs. Because each ReadGuard is scoped to
  // read_timeout's body, the lock is free again by the time
  // update_timeout tries to acquire it for writing.
  let seen_by_reader_1: i64 = read_timeout(mut ref rw);
  let seen_by_reader_2: i64 = read_timeout(mut ref rw);

  let _ = update_timeout(mut ref rw, 10000);

  let seen_after_update: i64 = read_timeout(mut ref rw);

  print "reader 1 saw timeout_ms =", seen_by_reader_1;
  print "reader 2 saw timeout_ms =", seen_by_reader_2;
  print "after update, timeout_ms =", seen_after_update;
  return 0;
}
```

Expected output:

```
reader 1 saw timeout_ms = 5000
reader 2 saw timeout_ms = 5000
after update, timeout_ms = 10000
```

<img class="manas" src="../images/mascot/manas_mascot_caution.png" title="this code needs extra care"/>

If a `ReadGuard`/`WriteGuard` is still alive when you try to acquire
the lock again on the SAME thread (e.g. you keep both guards from
`read_timeout`-style calls alive at once, instead of letting each
drop at the end of its own scope before the next acquisition), the
second acquisition blocks forever -- there's no thread left to
release it. Scope each guard as tightly as this example does (one
guard per function call, dropped at that function's return) rather
than holding several open at once in the same function body.

The sequential version above keeps things simple for a first pass,
but `task`/`join` work here too -- `read_timeout` is Copy-in/Copy-out
(`mut ref RwLock<i64>` argument, `i64` result), so both reads can
genuinely run concurrently:

```vani
let seen_by_reader_1: Task<i64> = task read_timeout(mut ref rw);
let seen_by_reader_2: Task<i64> = task read_timeout(mut ref rw);
let r1: i64 = join seen_by_reader_1;
let r2: i64 = join seen_by_reader_2;
```

See [Advanced 3](03_concurrency.md) for the full `Task<R>` reference.

## Cross-reference

- [Advanced 3 -- concurrency primitives](03_concurrency.md)
  -- full API reference for all six primitives including RwLock
- [Advanced 2a -- Parallelism primer](02a_parallelism_primer.md)
  -- ownership model that makes all these primitives safe
- [Advanced 2b -- Barrier primer](02b_barrier_primer.md)
  -- the phase-rendezvous primitive: when you need all threads
  to reach the same point before any proceed


---

**Previous**: [Sec.2b -- Barrier: rendezvous synchronization primer ->](02b_barrier_primer.md)
**Next**: [Sec.2 -- parallel for + reductions + race-freedom ->](02_parallel.md)

