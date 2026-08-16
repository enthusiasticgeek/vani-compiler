# Advanced 3e -- Capstone: a job scheduler (Executor + cooperative cancel + real-thread cancel)

> **Learning goal**: see three concurrency mechanisms this book covers
> separately -- the `Pollable`/`Executor` pattern, cooperative
> `CancelToken` cancellation, and real-thread `cancel <name>;` -- work
> together in one program, and confirm with a `vanic test` harness
> that they genuinely don't interfere with each other. Reading order:
> [Advanced 1 -- async](01_async.md#an-executor-not-a-hand-rolled-driver-pollable--executor)
> introduces `Pollable`/`Executor` and cooperative `CancelToken`
> cancellation; [Advanced 3 -- concurrency](03_concurrency.md#cancel)
> introduces real-thread `cancel <name>;`; [Intermediate
> 16a](../intermediate/16a_testing_primer.md) introduces `vanic test`
> and `#[test]`. This chapter is a walking tour of what happens when a
> program genuinely needs all three at once, not a re-introduction of
> any of them.

This is a walking tour of
[`examples/language/english/job_scheduler_capstone.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/english/job_scheduler_capstone.vani)
and its companion test harness,
[`job_scheduler_capstone_test.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/english/job_scheduler_capstone_test.vani).

## The scenario

A job scheduler polls three worker connections for results at the
same time a separate watchdog thread waits for a health-check probe
that may never arrive. This is a genuinely common shape: an event
loop cooperatively juggling several in-flight jobs, alongside a
thread that has to make a *real* blocking call (accepting a
connection has no non-blocking, poll-driven equivalent that fits
every use case) and therefore needs a hard, external way to be told
"give up" rather than being trusted to time out on its own.

Two things are genuinely happening **at the same time**, on two
different threads, not as two sequential phases of the program:

- **The main thread** runs a single-threaded, cooperative
  `Pollable`/`Executor` loop driving three heterogeneous jobs:
  - a quick job that completes after one suspend point,
  - a normal job that completes after two,
  - a job that's cooperatively cancelled mid-flight via
    `CancelToken`, between its two suspend points.
- **A background OS thread** (`task watchdog`) sits inside a real,
  genuinely blocking `tcp_accept()` call, on a listener nobody is
  ever going to connect to -- until the main thread, once its own
  work is done, bounds that thread's worst case with `cancel
  watchdog;`.

Every connection in this file is local loopback
(`tcp_listen(0)` picks an OS-assigned ephemeral port), and every
peer's timing is controlled by this program's own `sleep_ms` calls --
the same pattern
[`async_executor.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/english/async_executor.vani)
and
[`cancel_blocking_task.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/english/cancel_blocking_task.vani)
each already use individually, and which this whole codebase's test
suite already relies on being deterministic -- confirmed directly
with 10 back-to-back runs on each backend during this chapter's own
verification, all with identical output.

## Build & run

```bash
vanic run examples/language/english/job_scheduler_capstone.vani                          # LLVM backend
vanic run examples/language/english/job_scheduler_capstone.vani --backend=c              # C backend
vanic build examples/language/english/job_scheduler_capstone.vani -o /tmp/sched && /tmp/sched
vanic test examples/language/english/job_scheduler_capstone_test.vani                    # the test harness
```

---

## Step 1: the Pollable/Executor pattern, unchanged

The `Pollable` interface and `Executor` struct are copied verbatim
from [Advanced 1](01_async.md#an-executor-not-a-hand-rolled-driver-pollable--executor)
-- this is deliberate. The pattern is
[documented as copy-paste, not a compiler builtin](01_async.md#why-this-is-a-copy-paste-pattern-not-a-compiler-builtin):
injecting `Pollable`/`Executor` into the compiler's universal prelude
once broke SSA-backend compilation for every program, not just ones
using `Executor`, since `Box<dyn Pollable>` is an SSA-unsupported
shape. Nothing new here -- if `executor_run_to_completion`'s
round-robin loop looks familiar, it's because it's the same function.

```vani
interface Pollable {
  fn poll(self: mut ref Self) -> i64;
}

struct Executor {
  ep: i64,
  tasks: Vec<Box<dyn Pollable>>,
}
```

## Step 2: three heterogeneous jobs

```vani
async fn collect_quick_result(fd: i64) -> i64 {
  let n: i64 = io_recv_async(fd, 64);
  return n * 10;
}

async fn collect_result(fd: i64) -> i64 {
  let n: i64 = io_recv_async(fd, 64);
  let m: i64 = io_recv_async(fd, 64);
  return n + m;
}

async fn collect_abandoned_result(fd: i64, token: ref CancelToken) -> i64 {
  let n: i64 = io_recv_async(fd, 64);
  let m: i64 = io_recv_async(fd, 64);
  return n + m;
}
```

Three different `async fn`s means three different `Task__<fn>`
state-machine shapes -- `collect_quick_result` has one suspend point,
`collect_result` and `collect_abandoned_result` each have two. Each
gets its own `implement Pollable for Task__<fn>` forwarding block (one
line, `return __poll_<fn>(self);`), and all three end up in the *same*
`Vec<Box<dyn Pollable>>`, driven by the same `executor_run_to_completion`
loop, via dynamic dispatch. This is the actual point of `Pollable`:
the Executor doesn't know or care that these are three unrelated
state-machine shapes.

## Step 3: cooperative cancellation, mid-flight

`collect_abandoned_result`'s peer sends its first chunk but never its
second -- so by design, this job never has a chance to complete on
its own. Instead of leaving it to hang forever inside the Executor's
polling loop, it's cancelled cooperatively, the same way
[`async_executor.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/english/async_executor.vani)
demonstrates it:

```vani
let tok: CancelToken = CancelToken { cancelled: false };
let mut_ta: Task__collect_abandoned_result = collect_abandoned_result(fd_abandoned, ref tok);
let ta: Task__collect_abandoned_result = mut_ta;
let _ = __poll_collect_abandoned_result(mut ref ta);
tok.cancelled = true;
let _ = executor_spawn(mut ref ex, box(ta as dyn Pollable));
```

The job is polled once by hand first (guaranteed past its first
suspend point by an earlier `sleep_ms`), so it's already holding
real, heap-owned local state when `tok.cancelled` flips to `true` and
it's handed to the Executor. This exact shape -- a cancelled task
with live heap state, reaped by the Executor -- is
[ASan/LeakSanitizer-verified clean](01_async.md#leak-safety), and this
chapter's own verification re-confirmed it under a fresh
`-fsanitize=address,undefined` build.

`executor_run_to_completion` doesn't distinguish "completed normally"
from "cancelled" in its return count -- both count as "accounted
for". If you need to know *which* happened for a specific job, that's
what each job's own `poll()` body is for (print it, write it into a
shared `Mutex`, check a flag your own code set) -- the same caveat
[Advanced 1](01_async.md) makes about the Executor's uniform return
value.

## Step 4: a real thread, blocked in a real syscall, running concurrently

Nothing about `Pollable`/`Executor`/`CancelToken` above touches
threads -- the whole point of the pattern is a *single*-threaded
cooperative loop. But `tcp_accept()` has no non-blocking, pollable
equivalent that fits every use case (waiting for a connection that
may simply never come, with no data to poll for in the meantime), so
this chapter also spawns a genuine OS thread that makes a genuine
blocking call -- started *before* the Executor section, so it's
already sitting inside `accept()` while the Executor above runs:

```vani
let watchdog_server: i64 = tcp_listen(0);
task watchdog {
  let fd: i64 = tcp_accept(watchdog_server);
  assert fd == 0 - 2;
}

// ... the entire Step 1-3 Executor section runs here, on the main
// thread, while `watchdog` is still blocked in accept() ...

cancel watchdog;
join watchdog;
```

`cancel watchdog;` is the mechanism [Advanced
3](03_concurrency.md#cancel) covers: it sets a shared flag and (on
POSIX) sends a reserved signal that forces the in-flight `tcp_accept`
to return `EINTR` instead of the kernel silently re-driving it.
`tcp_accept` is one of the small set of **cancel-aware** blocking
builtins -- it checks that flag on `EINTR` and returns `-2` (a
sentinel distinct from `-1`, a real socket error, and any
non-negative fd) instead of retrying. `cancel` doesn't consume the
task -- `join watchdog;` afterward is still required, same affine
discipline `task`/`join` always uses; `cancel` just makes that
eventual `join` return promptly instead of waiting for a connection
that was never coming.

**Why this couldn't be `CancelToken` instead.** `CancelToken`
cancellation (Step 3) is *cooperative*: the suspended `async fn` has
to reach its own next suspend point and check the flag itself before
anything happens. A thread genuinely blocked inside the kernel's
`accept()` implementation isn't running any of your code at all --
there's no suspend point for it to check a flag at, because it isn't
polling anything. That's exactly the gap `cancel <name>;` fills:
interrupting a syscall from *outside*, at the OS level, rather than
waiting for the blocked code to cooperate.

## Step 5: confirming it with `vanic test`, not just reading the printed output

The runnable file above proves the pattern *works*; it doesn't prove
the Executor and the concurrently-blocked `watchdog` thread
*genuinely* don't affect each other's results -- that claim needs an
assertion, not eyeballed stdout. That's what
[`job_scheduler_capstone_test.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/english/job_scheduler_capstone_test.vani)
is for -- five `#[test]` functions, no top-level `fn main` (see
[Intermediate 16a](../intermediate/16a_testing_primer.md) for why a
main-less file with `#[test]` fns runs in harness mode under `vanic
test` instead of failing to compile):

```bash
vanic test examples/language/english/job_scheduler_capstone_test.vani
```

```
running 5 tests (examples/language/english/job_scheduler_capstone_test.vani)
test executor_with_zero_tasks_returns_zero_immediately ... ok
test executor_drives_two_heterogeneous_jobs_to_completion ... ok
test executor_reaps_a_cooperatively_cancelled_job ... ok
test cancel_makes_blocked_accept_return_cancelled_sentinel ... ok
test executor_and_a_concurrently_blocked_cancel_thread_do_not_interfere ... ok

test result: ok. 5 passed; 0 failed
```

- **`executor_with_zero_tasks_returns_zero_immediately`** -- an edge
  case the runnable demo never exercises: an `Executor` that never
  gets a single job spawned into it must return `0` immediately,
  without ever calling `epoll_wait_one` at all. This is the outer
  `while len(ref ex.tasks) > 0 as u64` loop's zero-iteration path.
- **`executor_drives_two_heterogeneous_jobs_to_completion`** and
  **`executor_reaps_a_cooperatively_cancelled_job`** -- `assert_eq_i64`
  on the Executor's returned count, isolating Steps 2 and 3 above from
  each other and from the watchdog thread entirely.
- **`cancel_makes_blocked_accept_return_cancelled_sentinel`** --
  isolates Step 4 on its own, exactly matching
  `cancel_blocking_task.vani`'s own shape.
- **`executor_and_a_concurrently_blocked_cancel_thread_do_not_interfere`**
  -- the actual claim this chapter makes: runs the Executor to
  completion while a separate thread sits blocked in `tcp_accept()`
  at the same time, asserts the Executor's own result is exactly what
  it would be without the concurrent thread (`assert_eq_i64(done,
  1)`), and only then cancels and joins the watchdog. If a future
  change ever introduced real interference between the two -- a
  shared resource collision, an unexpected epoll wakeup from the
  wrong fd, anything -- this is the test that would catch it.

Each `#[test]` fn opens its own listener via `tcp_listen(0)` (an
OS-assigned ephemeral port), so they're safe to run in any order,
including in parallel under `vanic test`'s own parallel execution --
no fixed port for two concurrently-running tests to collide on.

## Try it yourself

Add a fourth job to the scheduler that's cancelled via `cancel
<name>;` instead of `CancelToken` -- i.e., give it its own `task`
thread that calls a **blocking** (not `async`) `tcp_recv`, and cancel
that thread the same way `watchdog` is cancelled, while the
`Pollable`/`Executor` loop for the other three jobs runs alongside
it. Write a `#[test]` asserting the mixed scheduler still reports the
right total job count across both cancellation mechanisms.
