# Advanced 3 -- `task` / `join` + atomics / mutexes / channels / barriers / rwlocks

> **Learning goal**: spawn an explicit OS thread with `task`,
> join it back, and pick the right concurrency primitive
> (`Atomic<T>` / `Mutex<T>` / `Channel<T>` / `Condvar` /
> `Barrier` / `RwLock<T>`) for your synchronization need.

Imagine two chefs sharing one kitchen at the same time (true
parallelism -- two threads). If they both reach for the salt
shaker simultaneously, one has to wait. Concurrency primitives
are the rules that prevent chaos:
- **Atomic** -- a single number with a padlock so small and fast
  that the two chefs can each update it without ever noticing
  the other. Good for simple counters.
- **Mutex** -- a door key: only the chef holding the key can
  enter the walk-in fridge. One at a time; everyone else waits.
- **Channel** -- a pass-through hatch: chef A prepares dishes
  and slides them through; chef B picks them up on the other
  side. No shared space, no collision.
- **Condvar** -- a buzzer: one chef waits in the break room and
  the other buzzes them when the oven is free.
- **Barrier** -- a starting gate: all N chefs must arrive before
  any of them can enter the kitchen together.
- **RwLock** -- a whiteboard with a usage log: many chefs can
  read it at once, but only one can write and only when no one
  is reading.

`task` spawns the second chef; `join` waits for them to finish
cleaning up before you leave.

## The `task` / `join` shape

```vani
fn worker(n: i64) -> i64 {
  return n * 7;
}

fn main() -> i64 {
  let t: Task<i64> = task worker(6);   // spawns an OS thread
  // ...do other work concurrently...
  let r: i64 = join t;                 // blocks until worker exits
  print "result =", r;
  return 0;
}
```

- **`task EXPR`** spawns an OS thread (via `pthread_create` on
  Linux/macOS, `CreateThread` on Windows -- the backend picks)
  and runs the expression in it. Returns a `Task<R>` handle.
- **`join t`** blocks the caller until the task finishes, then
  returns the task's value.
- **`Task<R>` is affine**: each handle can be joined exactly
  once. The compiler catches double-join at compile time.

## The six concurrency primitives

vāṇी ships six primitives in the prelude -- pick by the
synchronization shape you need:

### `Atomic<T>` -- lock-free counters and flags

```vani
let counter: Atomic<i64> = atomic_new(0);
let old: i64 = atomic_fetch_add(ref counter, 5);
let cur: i64 = atomic_load(ref counter);
let _ = atomic_store(ref counter, 0);
let _ = atomic_compare_exchange(ref counter, 0, 1);
```

- `T` ranges over `i8..i64`, `u8..u64`, `bool`, and `f64`.
- `atomic_fetch_add` is supported for integer types only; use a
  `Mutex<f64>` for floating-point accumulation.
- All operations are sequentially consistent (`seq_cst`) -- no
  weaker memory orderings in v1.
- Best for hot counters, flags, and single-cell lock-free
  publication.

### `Mutex<T>` -- guarded mutation of a payload

```vani
// Simple scalar payload
let m: Mutex<i64> = mutex_new(0);
{
  let g: Guard<i64> = mutex_lock(ref m);
  guard_set(mut ref g, 42);
  let v: i64 = guard_get(ref g);
}   // Guard's scope exit unlocks

// Any element type works (v0.1.1+)
struct Config { limit: i64, debug: bool }
let cfg: Mutex<Config> = mutex_new(Config { limit: 100, debug: false });
{
  let g: Guard<Config> = mutex_lock(ref cfg);
  let c: Config = guard_get(ref g);
  print "limit =", c.limit;
}
```

- `T` can be any type: scalars, `bool`, structs, enums (parametric since v0.1.1).
- The `Guard<T>` returned by `mutex_lock` is affine -- exactly one
  thread can hold a guard for a given mutex at a time, enforced
  at compile time + runtime.
- The unlock happens at the guard's scope exit (Rust-style RAII).

### `Channel<T, N>` -- bounded MPMC queue

```vani
// Scalar element
let ch: Channel<i64, 16> = channel_new();
let _ = channel_send(ref ch, 42);     // blocks if full
let v: i64 = channel_recv(ref ch);    // blocks if empty

// Struct element (v0.1.1+)
struct Msg { id: i64, value: i64 }
let ch2: Channel<Msg, 8> = channel_new();
let _ = channel_send(ref ch2, Msg { id: 1, value: 99 });
let m: Msg = channel_recv(ref ch2);
print "got msg id =", m.id;
```

- `T` can be any type: scalars, `bool`, structs, enums (parametric since v0.1.1). `N` is the bounded capacity.
- Backed by a ring buffer with a futex/WaitOnAddress wait
  protocol so blocked threads don't spin.

### `Condvar` -- signaling for non-trivial wait conditions

```vani
let cv: Condvar = condvar_new();
let m: Mutex<i64> = mutex_new(0);

let g: Guard<i64> = mutex_lock(ref m);
// pseudo: wait until predicate holds
let _ = condvar_wait(ref cv, mut ref g);
let _ = condvar_signal_one(ref cv);
let _ = condvar_signal_all(ref cv);
```

- Used together with `Mutex` for "wait until X" patterns.
- v1's predicate is just `Guard<i64>` payload comparison; full
  user-predicate support is a follow-up.

### `Barrier` -- N-thread rendezvous

A Barrier makes all N threads wait at a checkpoint until every
one of them has arrived. Only then does every thread proceed --
like a starting gun at a race.

```vani
fn stage_one(n: i64, b: mut ref Barrier) -> i64 {
  // ...do first-stage work...
  let is_last: bool = barrier_wait(mut ref b);
  // All N threads have now finished stage one.
  // is_last is true for exactly the last thread to arrive.
  return 0;
}

fn main() -> i64 {
  let b: Barrier = barrier_new(3);   // rendezvous of 3 threads
  let t1: Task<i64> = task stage_one(1, mut ref b);
  let t2: Task<i64> = task stage_one(2, mut ref b);
  let _ = stage_one(3, mut ref b);   // main thread is the third
  let _ = join t1;
  let _ = join t2;
  return 0;
}
```

- `barrier_new(n)` creates an affine Barrier for `n` threads.
- `barrier_wait(mut ref b)` blocks until all n threads have
  called it; returns `true` for the last thread to arrive.
- Uses a generation counter to prevent ABA races -- safe to
  reuse in a loop.

### `RwLock<T>` -- shared reads, exclusive writes

A RwLock lets many threads read simultaneously but requires
exclusive access to write. Use it when reads vastly outnumber
writes and you want to avoid Mutex contention.

```vani
fn main() -> i64 {
  let rw: RwLock<i64> = rwlock_new(0);

  // Shared read -- many threads can hold a ReadGuard at once
  let r: ReadGuard<i64> = rwlock_read(ref rw);
  let v: i64 = read_guard_get(ref r);
  print "current value =", v;
  // ReadGuard drops here -> read lock released

  // Exclusive write -- blocks until all readers have released
  let w: WriteGuard<i64> = rwlock_write(mut ref rw);
  let _ = write_guard_set(mut ref w, v + 1);
  // WriteGuard drops here -> write lock released

  return 0;
}
```

- State encoding: `0` = unlocked, `N > 0` = N concurrent
  readers, `-1` = write-locked.
- `ReadGuard<T>` and `WriteGuard<T>` are both affine; their
  Drop calls the appropriate unlock automatically (RAII).
- Parametric over any element type `T` (v0.1.1+).

## Worked examples in the repo

For runnable end-to-end programs, see:

- `examples/language/english/atomics.vani` -- atomic counter
  and the five builtin operations.
- `examples/language/english/concurrency.vani` -- `task` + `join`
  with multiple workers.
- `examples/language/english/condvar.vani` -- `Condvar` waiting
  on a `Mutex<i64>` payload.

## Picking the right primitive

| You want | Use |
|---|---|
| A counter that two threads bump | `Atomic<i64>` |
| Guarded mutation of a value | `Mutex<T>` |
| Read-heavy shared state (many readers, rare writes) | `RwLock<T>` |
| Producer-consumer queue | `Channel<T, N>` |
| Wait until a non-trivial condition | `Condvar` + `Mutex` |
| All N threads reach a checkpoint before proceeding | `Barrier` |
| Spawn-and-forget threads | `task` + immediate drop |
| Spawn-and-collect-result | `task` + `join` |

If your design needs a primitive that's not in the list, look
in `examples/language/english/parallel.vani` -- `parallel for`
+ `reduce` covers a lot of the embarrassingly-parallel space
without needing explicit synchronization.

## Challenge

Spawn 4 worker tasks, each summing a slice of a `Vec<i64>`,
joining all four, and combining their results into one
final total. Use `Atomic<i64>` for the running total.

---

**Previous**: [Sec.2 -- `parallel for` + reductions + race-freedom ->](02_parallel.md)
**Next**: [Sec.3b -- Condition variables primer ->](03b_condvar_primer.md)
