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

// Acquire a shared read lock.
// Many threads can hold a ReadGuard simultaneously.
let r: ReadGuard<i64> = rwlock_read(ref rw);
let v: i64 = read_guard_get(ref r);
// r goes out of scope here -> read lock released automatically

// Acquire an exclusive write lock.
// Blocks until all current readers have released.
let w: WriteGuard<i64> = rwlock_write(mut ref rw);
let _ = write_guard_set(mut ref w, v + 1);
// w goes out of scope here -> write lock released automatically
```

`T` can be any type: `i64`, `bool`, a struct, an enum, `Vec<T>`.

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

```vani
intent "RwLock primer -- shared configuration table.";

struct Config { max_retries: i64, timeout_ms: i64 }

fn read_config(rw: ref RwLock<Config>) -> i64 {
  let r: ReadGuard<Config> = rwlock_read(ref rw);
  let cfg: Config = read_guard_get(ref r);
  // ReadGuard drops here -- read lock released
  return cfg.timeout_ms;
}

fn update_timeout(rw: mut ref RwLock<Config>, new_ms: i64) -> i64 {
  let w: WriteGuard<Config> = rwlock_write(mut ref rw);
  let old: Config = read_guard_get(ref w);   // read current value via write guard
  let _ = write_guard_set(mut ref w, Config { max_retries: old.max_retries, timeout_ms: new_ms });
  // WriteGuard drops here -- write lock released
  return 0;
}

fn main() -> i64 {
  let rw: RwLock<Config> = rwlock_new(Config { max_retries: 3, timeout_ms: 5000 });

  // Spawn two reader tasks
  let t1: Task<i64> = task read_config(ref rw);
  let t2: Task<i64> = task read_config(ref rw);

  // Update from the main thread while readers may be running
  let _ = update_timeout(mut ref rw, 10000);

  let ms1: i64 = join t1;
  let ms2: i64 = join t2;
  print "reader 1 saw timeout_ms =", ms1;
  print "reader 2 saw timeout_ms =", ms2;
  return 0;
}
```

## Cross-reference

- [Advanced 3 -- concurrency primitives](03_concurrency.md)
  -- full API reference for all six primitives including RwLock
- [Advanced 2a -- Parallelism primer](02a_parallelism_primer.md)
  -- ownership model that makes all these primitives safe
- [Advanced 2b -- Barrier primer](02b_barrier_primer.md)
  -- the phase-rendezvous primitive: when you need all threads
  to reach the same point before any proceed
