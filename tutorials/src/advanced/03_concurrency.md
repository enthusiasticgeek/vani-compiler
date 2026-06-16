# Advanced 3 — `task` / `join` + atomics / mutexes / channels

> **Learning goal**: spawn an explicit OS thread with `task`,
> join it back, and pick the right concurrency primitive
> (`Atomic<T>` / `Mutex<T>` / `Channel<T>` / `Condvar`) for
> your synchronization need.

Imagine two chefs sharing one kitchen at the same time (true
parallelism — two threads). If they both reach for the salt
shaker simultaneously, one has to wait. Concurrency primitives
are the rules that prevent chaos:
- **Atomic** — a single number with a padlock so small and fast
  that the two chefs can each update it without ever noticing
  the other. Good for simple counters.
- **Mutex** — a door key: only the chef holding the key can
  enter the walk-in fridge. One at a time; everyone else waits.
- **Channel** — a pass-through hatch: chef A prepares dishes
  and slides them through; chef B picks them up on the other
  side. No shared space, no collision.
- **Condvar** — a buzzer: one chef waits in the break room and
  the other buzzes them when the oven is free.

`task` spawns the second chef; `join` waits for them to finish
cleaning up before you leave.

## The `task` / `join` shape

```vani
fn worker(n: i64) -> i64 {
  return n * 7;
}

fn main() -> i64 {
  let t: Task<i64> = task worker(6);   // spawns an OS thread
  // …do other work concurrently…
  let r: i64 = join t;                 // blocks until worker exits
  print "result =", r;
  return 0;
}
```

- **`task EXPR`** spawns an OS thread (via `pthread_create` on
  Linux/macOS, `CreateThread` on Windows — the backend picks)
  and runs the expression in it. Returns a `Task<R>` handle.
- **`join t`** blocks the caller until the task finishes, then
  returns the task's value.
- **`Task<R>` is affine**: each handle can be joined exactly
  once. The compiler catches double-join at compile time.

## The four concurrency primitives

vāṇी ships four primitives in the prelude — pick by the
synchronization shape you need:

### `Atomic<T>` — lock-free counters and flags

```vani
let counter: Atomic<i64> = atomic_new(0);
let old: i64 = atomic_fetch_add(ref counter, 5);
let cur: i64 = atomic_load(ref counter);
let _ = atomic_store(ref counter, 0);
let _ = atomic_compare_exchange(ref counter, 0, 1);
```

- `T` ranges over `i8..i64`, `u8..u64`, and `bool`.
- All operations are sequentially consistent (`seq_cst`) — no
  weaker memory orderings in v1.
- Best for hot counters, flags, and single-cell lock-free
  publication.

### `Mutex<T>` — guarded mutation of a payload

```vani
let m: Mutex<i64> = mutex_new(0);
{
  let g: Guard<i64> = mutex_lock(ref m);
  guard_set(mut ref g, 42);
  let v: i64 = guard_get(ref g);
}   // Guard's scope exit unlocks
```

- `T` is `i64` only in v1.
- The `Guard<T>` returned by `lock` is affine — exactly one
  thread can hold a guard for a given mutex at a time, enforced
  at compile time + runtime.
- The unlock happens at the guard's scope exit (Rust-style
  RAII).

### `Channel<T, N>` — bounded MPMC queue

```vani
let ch: Channel<i64, 16> = channel_new();
let _ = channel_send(ref ch, 42);     // blocks if full
let v: i64 = channel_recv(ref ch);    // blocks if empty
```

- `T` is currently scalar; `N` is the bounded capacity.
- Backed by a ring buffer with a futex/WaitOnAddress wait
  protocol so blocked threads don't spin.

### `Condvar` — signaling for non-trivial wait conditions

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

## Worked examples in the repo

For runnable end-to-end programs, see:

- `examples/language/english/atomics.vani` — atomic counter
  and the five builtin operations.
- `examples/language/english/concurrency.vani` — `task` + `join`
  with multiple workers.
- `examples/language/english/condvar.vani` — `Condvar` waiting
  on a `Mutex<i64>` payload.

## Picking the right primitive

| You want | Use |
|---|---|
| A counter that two threads bump | `Atomic<i64>` |
| Guarded mutation of a value | `Mutex<T>` |
| Producer-consumer queue | `Channel<T, N>` |
| Wait until a non-trivial condition | `Condvar` + `Mutex` |
| Spawn-and-forget threads | `task` + immediate drop |
| Spawn-and-collect-result | `task` + `join` |

If your design needs a primitive that's not in the list, look
in `examples/language/english/parallel.vani` — `parallel for`
+ `reduce` covers a lot of the embarrassingly-parallel space
without needing explicit synchronization.

## Challenge

Spawn 4 worker tasks, each summing a slice of a `Vec<i64>`,
joining all four, and combining their results into one
final total. Use `Atomic<i64>` for the running total.

---

**Next**: [§4 — Embedded targets + `unsafe` + region typing →](04_embedded.md)
