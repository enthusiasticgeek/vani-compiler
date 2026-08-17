# Advanced 3 -- `task` / `join` / `detach` / `cancel` + atomics / mutexes / channels / barriers / rwlocks

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
cleaning up before you leave. `detach` is the third option: send
the chef off to run their own errand and don't wait around for
them at all -- useful for background work (a heartbeat, a log
flusher) that should keep running independently of whatever the
caller does next.

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

- **`task <fn>(args…)`** spawns an OS thread (via
  `pthread_create` on Linux/macOS, `CreateThread` on Windows --
  the backend picks) that calls `<fn>` with `args`. Returns a
  `Task<R>` handle, where `R` is `<fn>`'s return type.
- **`join t`** (in expression position, e.g. `let r = join t;`)
  blocks the caller until the task finishes, then returns the
  task's value. `join t;` as a bare statement also works and
  just discards the result.
- **`Task<R>` is affine**: each handle can be joined exactly
  once. The compiler catches double-join, and unjoined handles,
  at compile time.
- The callee doesn't need to be `pure fn` -- unlike the
  block-form `task { .. }` below (whose inline body implicitly
  captures the *outer* function's bindings and so must stay
  side-effect-free to avoid racing the caller), a call-form
  callee only ever touches its own explicit arguments. It's free
  to call blocking/synchronizing builtins (`barrier_wait`,
  `mutex_lock`, ...) -- see the `Barrier` section below for an
  example. The one restriction: every argument's type must be
  Copy (the value is duplicated into the spawned thread's heap
  context) -- pass `mut ref x` for a shared primitive like
  `Barrier`/`Mutex<T>` rather than moving it.
- There's also a block form, `task <name> { <body> }` / bare
  `join <name>;`, for spawning an inline body instead of a named
  function. It has no return-value payload (`Task`, not
  `Task<R>`) and its body must be pure-with-Copy-captures, since
  it implicitly captures the enclosing function's bindings.

## `detach` -- fire-and-forget

```vani
fn heartbeat() -> i64 {
  let i: i64 = 0;
  while i < 5 {
    print "[heartbeat] tick", i;
    sleep_ms(50);
    i = i + 1;
  }
  return 0;
}

fn main() -> i64 {
  let hb: Task<i64> = task heartbeat();
  detach hb;              // don't wait -- keep going immediately
  // ...main's own work runs concurrently with the heartbeat...
  print "main is done";
  return 0;
}
```

- **`detach <name>;`** is the other way to consume a `Task`/
  `Task<R>` handle -- instead of blocking until the thread exits
  (`join`), it lets the thread keep running independently and
  returns immediately. There's no result to retrieve (a detached
  `Task<R>`'s return value is simply discarded whenever the
  thread does finish).
- **Mutually exclusive with `join`, and exactly-once**: a spawned
  handle must be consumed by exactly one of `join <name>;` or
  `detach <name>;` -- never both, never neither, never twice. The
  compiler's affine checker enforces this the same way it already
  enforced join-exactly-once.
- **`detach` is rejected inside `pure fn` bodies.** `join` waits
  for the spawned work to finish before the caller proceeds, so a
  pure caller can still reason about "no observable side effects
  outlive this call." `detach` explicitly gives up that guarantee
  -- the detached thread's side effects (prints, mutex writes,
  ...) can keep happening after the pure function has already
  returned, so it isn't allowed there.
- Why not just `join` and throw away the result? Because `join`
  *blocks the caller* until the thread exits -- for background
  work with no natural end point relative to the caller (a
  heartbeat, a periodic flush) that defeats the purpose of
  spawning it in the first place. `detach` is the "actually don't
  wait" primitive.
- See `examples/language/english/detach_heartbeat.vani` for a
  fuller worked example (a background heartbeat detached while
  `main` runs an independent, deterministic computation and
  checks the result once it's done).
- **`vanic run`-only caveat ([L31 in
  `docs/v1_limitations.md`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/docs/v1_limitations.md)):**
  if
  a `detach`'d task is still actively running when `main` returns,
  running the program with plain `vanic run` (the default LLVM path,
  which JIT-executes via the external `lli` tool) can segfault --
  `lli`'s own JIT engine appears to tear down its compiled machine
  code when the JIT'd `main` returns without waiting for OS-level
  pthreads (like a detached task's) that may still be executing
  through it. Root-caused to `lli` itself, not vāṇी's codegen: the
  identical program compiled with `vanic build` (AOT, no `lli`
  involved) or run via `vanic run --backend=c` completes correctly
  every time. If a detached task's runtime might outlive `main`
  (a genuine background heartbeat, not one that's guaranteed to
  finish first), prefer `vanic build` or `--backend=c` over the
  default `vanic run`.

## `cancel` -- interrupting a thread stuck in a blocking call {#cancel}

`detach` solves "I don't want to wait for this thread." It does
NOT solve a different problem: a thread genuinely stuck inside a
blocking syscall (`tcp_accept`, `tcp_recv`) with no data ever
coming. `cancel <name>;` (shipped 2026-08-14) forces that call to
return promptly instead of waiting forever.

```vani
fn main() -> i64 {
  let server: i64 = tcp_listen(0);

  task blocked {
    let fd: i64 = tcp_accept(server);
    // -2 is the "cancelled while blocked" sentinel -- distinct
    // from -1 (a real socket error) and any valid fd.
    assert fd == 0 - 2;
  }

  let _ = sleep_ms(100);  // let the thread actually enter accept()
  cancel blocked;
  join blocked;           // returns almost immediately

  print "cancelled thread returned -- no hang";
  let _ = tcp_close(server);
  return 0;
}
```

- **`cancel <name>;` does NOT consume the handle.** Unlike
  `join`/`detach`, it's not one of the two ways to satisfy the
  affine "every spawned task is consumed exactly once" rule -- a
  `join` or `detach` is still required afterward. Think of `cancel`
  as "please stop soon," not "I'm done with this task."
- **Cancelling is idempotent.** Calling `cancel` more than once on
  the same still-live task is harmless. Cancelling an
  already-`join`ed or already-`detach`ed task is rejected
  ("nothing left to cancel").
- **Only `tcp_accept`/`tcp_recv` are cancel-aware today.**
  `stdin_read_line`/`file_read_line` cancellation is still open
  (buffered stdio's `EINTR` interaction needs its own design pass).
  A CPU-bound task (no blocking call at all) still sees the
  cancellation if it cooperatively checks -- but nothing forces a
  tight compute loop to notice on its own.
- **`cancel` is rejected inside `pure fn` bodies**, same reasoning
  as `detach`: signaling another thread is a side effect outside
  the pure call's own sequential model.
- **Platform coverage**: verified via `strace` on Linux (both
  backends) that the interrupted syscall genuinely returns `EINTR`
  rather than silently auto-restarting. macOS shares the same POSIX
  signal path (untested on real hardware). Windows uses
  `CancelSynchronousIo` on the task's thread handle instead
  (untested on real hardware -- no Windows host available when this
  shipped).
- See `examples/language/english/cancel_blocking_task.vani` for the
  full worked example above, runnable on both backends. For `cancel`
  combined with the `Pollable`/`Executor` pattern and a `vanic test`
  harness, see [Advanced 3e's job scheduler
  capstone](03e_job_scheduler_capstone.md).

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
let _ = condvar_notify_one(ref cv);   // wake exactly one waiter
let _ = condvar_notify_all(ref cv);   // wake every waiter
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
  let is_last: bool = barrier_wait(b);   // `b` is already `mut ref Barrier`
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
  let v: i64 = 0;

  // Shared read -- many threads can hold a ReadGuard at once.
  // Both rwlock_read AND rwlock_write take `mut ref` -- acquiring
  // even a read lock mutates the lock's internal reader-count.
  // Scoped in its own block so the ReadGuard drops (releasing the
  // read lock) BEFORE the write-lock acquisition below -- holding
  // a live guard across another acquisition on the same thread
  // deadlocks (there's no other thread left to release it).
  {
    let r: ReadGuard<i64> = rwlock_read(mut ref rw);
    v = read_guard_get(ref r);
    print "current value =", v;
  }   // ReadGuard drops here -> read lock released

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
- `examples/language/english/concurrency.vani` -- `Channel<T>` +
  `Mutex<T>`/`Guard<T>` producer/consumer and protected-update
  patterns.
- `examples/language/english/task_result_multi.vani` -- `Task<R>`:
  concurrent spawns with a multi-arg callee, `join` with and
  without capturing the result.
- `examples/language/english/condvar.vani` -- `Condvar` waiting
  on a `Mutex<i64>` payload.
- `examples/language/english/task_parallel_chunk_sum.vani` -- a
  real-world (not toy) parallel-reduce: a dataset is split into
  chunks, each summed on its own thread (3 background tasks +
  main as the 4th "worker"), then joined and combined.
- `examples/language/english/detach_heartbeat.vani` -- a
  real-world `detach` example: a background heartbeat runs
  detached while `main` performs an independent, deterministic
  computation and checks its result.
- `examples/language/english/cancel_blocking_task.vani` -- a
  real-world `cancel` example: interrupts a thread genuinely
  blocked inside `tcp_accept()`, verified via `strace` that the
  syscall returns `EINTR` rather than auto-restarting.
- `examples/language/english/barrier_sensor_rendezvous.vani` --
  a real-world `Barrier` example; see [Advanced 2b -- Barrier
  primer](02b_barrier_primer.md#a-real-world-example) for the
  walkthrough.

## Picking the right primitive

| You want | Use |
|---|---|
| A counter that two threads bump | `Atomic<i64>` |
| Guarded mutation of a value | `Mutex<T>` |
| Read-heavy shared state (many readers, rare writes) | `RwLock<T>` |
| Producer-consumer queue | `Channel<T, N>` |
| Wait until a non-trivial condition | `Condvar` + `Mutex` |
| All N threads reach a checkpoint before proceeding | `Barrier` |
| Spawn-and-forget background work | `task` + `detach` |
| Spawn-and-collect-result | `task` + `join` |
| Interrupt a thread stuck in a blocking `tcp_accept`/`tcp_recv` | `task` + `cancel` + `join` |

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
**Next**: [Sec.3c -- Capstone: timed tic-tac-toe (stdin_ready_within_ms) ->](03c_timed_tic_tac_toe_capstone.md)
