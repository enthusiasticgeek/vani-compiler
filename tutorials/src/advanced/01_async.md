# Advanced 1 -- Async / await and the `Task` transform

> **Learning goal**: declare `async fn`, call it with `await`,
> thread a `CancelToken` through long operations, and
> understand the v1 desugar from `async fn` to `Future<R>`.

> **New to this?** Read [Advanced 1a -- Async primer](01a_async_primer.md) first.

Imagine a restaurant kitchen with one chef. When an order comes
in for a pizza (a slow operation -- it takes 15 min to bake),
a synchronous chef would stand at the oven doing nothing until
it's done. An async chef puts the pizza in the oven, notes
"pizza for table 3 in the oven", and goes to make a salad
(another request). When the oven timer dings, they come back
(`await`), pull the pizza out, and finish the pizza order.
`async fn` in vāṇī turns a function into this "return to it
when the slow thing is done" style automatically. The compiler
transforms it into a state machine so the one thread can
juggle many in-progress tasks without blocking.

## The program

```vani
intent "Advanced 1 -- async fn + await + CancelToken.";

async fn fetch(n: i64) -> i64 {
  return n * 7;
}

async fn cancellable(n: i64, token: ref CancelToken) -> i64 {
  if token.cancelled {
    return 0 - 1;
  }
  return n + 100;
}

fn main() -> i64 {
  // Plain async fn + await round-trip.
  let r1: i64 = await(fetch(6));
  print "await fetch(6) =", r1;

  // CancelToken not cancelled -- produces value.
  let tok: CancelToken = CancelToken { cancelled: false };
  let r2: i64 = await(cancellable(7, ref tok));
  print "await cancellable(7, !tok) =", r2;

  // CancelToken cancelled -- produces sentinel.
  let tok2: CancelToken = CancelToken { cancelled: true };
  let r3: i64 = await(cancellable(7, ref tok2));
  print "await cancellable(7, tok)  =", r3;

  return 0;
}
```

## Compile + run

```bash
vanic run ~/adv1.vani
```

Output:

```
await fetch(6) = 42
await cancellable(7, !tok) = 107
await cancellable(7, tok)  = -1
```

## Why it works that way

- **`async fn foo() -> R { ... }`** desugars to
  `fn foo() -> Future<R> { ...; return Future.Ready(v); ... }`.
  **This chapter's worked example never actually suspends** --
  `fetch`/`cancellable` above run to completion synchronously,
  same as any ordinary function call, just spelled with
  `async`/`await`. A REAL suspend-point state machine (one that
  genuinely parks mid-function on an I/O wait and resumes later)
  is a separate, considerably more involved feature -- see
  "The real thing" below.
- **`await(expr)`** desugars to a `match` that extracts
  `Future.Ready`'s payload. For the synchronous desugar shown
  above, the `Pending` arm body is the literal `0` and is never
  actually reached. This shape lets you write code that *looks*
  asynchronous while the compiler treats it as straight-line.
- **`CancelToken`** is a prelude-defined struct:
  ```vani
  struct CancelToken { cancelled: bool }
  ```
  Thread it through async functions and check `.cancelled` at
  natural breakpoints.
- **The `try` keyword sugar** ([Intermediate Sec.10](../intermediate/10_result_try.md))
  is **NOT usable inside `async fn` bodies in v1** -- confirmed
  by testing, contrary to an earlier version of this page. `try`
  requires the *enclosing* function's return type to literally be
  a two-variant enum, but `async fn foo() -> Option<i64>`
  desugars its return type to `Future<Option<i64>>` before `try`'s
  checker ever sees it, so it's rejected outright ("`try` requires
  the enclosing function's return type to be an enum; got
  `Future<Option<i64>>`") regardless of what `R` is. Use an
  explicit `match` inside async fn bodies instead (the same
  manual pattern the compiler's own diagnostic suggests:
  `match opt { Opt.Some(v) then v, Opt.None then return Opt.None
  };`) until this integration lands.

## The real thing: suspend points over I/O

<img class="manas" src="../images/mascot/manas_mascot_caution.png" title="this code needs extra care"/>

The synchronous desugar above is the whole story for `fetch`/
`cancellable`-shaped async fns -- nothing in them can actually
wait for anything. A genuine suspend point needs an operation
that can report "not ready yet" without blocking the thread --
in v1 that means the `io_*_async` family (`io_recv_async`,
`io_send_async`, ...) layered on non-blocking sockets + `epoll`.
When an async fn's body calls one of those, the compiler
transforms the WHOLE function into a real state machine: a
`Task__<fn_name>` struct holding the suspended local state, a
generated `__poll_<fn_name>` function that advances it one step
and returns either the result or "still pending," and a
caller-side polling loop (typically `epoll_wait` between polls)
that drives it to completion.

This is a substantially bigger shape than the trivial `await`
round-trip above -- worth seeing written out, not just described.
[`examples/language/english/async_showcase.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/english/async_showcase.vani)
is the real thing: a single `async fn` with outer `let`s, a
top-level `if` with a mid-body `return`, a `while` loop with TWO
suspend points inside it, `break`/`continue` inside the
suspending loop, and an `if`/`else` where one branch suspends and
the other falls through -- plus the hand-written `drive(...)`
polling loop that calls the generated `__poll_showcase` in a loop
around `epoll_wait_one`. Run it with `--backend=c`; the LLVM
backend currently miscompiles this specific example on Windows
(`lli` rejects the emitted IR with an undefined-SSA-value error --
a known gap, not something introduced by reading this chapter).

That `drive(...)` loop's `ep` handle -- the thing `epoll_wait_one`
polls -- comes from a small set of setup/teardown builtins that
never shows up in the desugared snippets above, only in the full
example file:

| Builtin | Signature | Description |
|---------|-----------|-------------|
| `epoll_new` | `() -> i64` | create an epoll instance; returns its handle (`-1` on error) |
| `epoll_add_read` | `(epfd, fd: i64) -> i64` | register `fd` for readability events on `epfd` |
| `epoll_wait_one` | `(epfd, timeout_ms: i64) -> i64` | block until one registered fd is ready; returns that fd, `-2` on timeout, `-1` on error |
| `epoll_close` | `(epfd: i64) -> i64` | close the epoll instance |

The pattern every non-blocking example in this chapter follows:
`epoll_new()` once, `tcp_set_nonblocking` + `epoll_add_read` on every
socket you register, then loop on `epoll_wait_one` between poll
attempts, and `epoll_close` when done.

## You don't have to go async: the plain blocking socket API

Everything above -- suspend points, `Task`, `select`, `epoll` -- exists
because a real server juggles many connections on one thread. If
you're writing a short script that talks to exactly one peer at a
time, none of that machinery is required: vāṇī also ships an
ordinary **blocking** socket API, the same shape you'd reach for
first in Python or C.

```vani
intent "Plain blocking TCP -- no async, no epoll";

fn main() -> i64 {
  let server: i64 = tcp_listen(0);
  let port: i64 = tcp_socket_port(server);

  task client {
    let c: i64 = tcp_connect_local(port);
    let _ = tcp_send_str(c, "ping");
    let _ = tcp_close(c);
  }

  // Blocks the calling thread until a client connects -- fine for
  // a short-lived script, wrong for a server juggling many peers.
  let peer: i64 = tcp_accept(server);
  let n: i64 = tcp_recv(peer, 64);
  print "bytes received:", n;   // 4  ("ping")
  let _ = tcp_send_buf(peer, n);   // echo it back

  join client;
  let _ = tcp_close(peer);
  let _ = tcp_close(server);
  return 0;
}
```

| Builtin | Signature | Description |
|---------|-----------|-------------|
| `tcp_listen` | `(port: i64) -> i64` | bind a server socket; `port = 0` lets the OS pick one |
| `tcp_socket_port` | `(fd: i64) -> i64` | read back the bound port (useful after `tcp_listen(0)`) |
| `tcp_accept` | `(server_fd: i64) -> i64` | **block** until a client connects; return its fd |
| `tcp_connect_local` | `(port: i64) -> i64` | connect to `127.0.0.1:port` |
| `tcp_send_str` | `(fd: i64, s: Str) -> i64` | send a string's bytes |
| `tcp_recv` | `(fd: i64, max: i64) -> i64` | **block** until data arrives (or EOF); return byte count |
| `tcp_send_buf` | `(fd: i64, n: i64) -> i64` | send the first `n` bytes most recently read by `tcp_recv` |
| `tcp_close` | `(fd: i64) -> i64` | close a socket |

`tcp_recv` reads into a per-thread buffer (4KB, one buffer per OS
thread -- concurrent `task` bodies each get their own), which
`tcp_send_buf` echoes from directly, so a request/response echo is
just `tcp_recv` then `tcp_send_buf(fd, n)`. This is the same
"one-thread-per-connection" model `task`/`join` already gives you
elsewhere in vāṇī -- reach for it before reaching for `async`/
`epoll`, and only graduate to the suspend-point machinery above once
you actually need one thread serving many connections at once.
`tcp_accept_nb`/`tcp_recv_nb` (used earlier in this chapter) are the
non-blocking siblings of `tcp_accept`/`tcp_recv` -- same socket, same
data, the only difference is whether the call waits.

## What's coming and what's queued

| Today | Queued |
|---|---|
| `async fn` + `await` synchronous desugar (v1) AND real suspend-point state machine over `io_*_async` (v3.1) -- see "The real thing" above | -- |
| `CancelToken` cooperative cancellation AND **A4.4** auto-injected cancel guards at every suspend point | -- |
| `try EXPR` keyword AND postfix `EXPR?` operator in both sync + async bodies | -- |
| `Future<R>` for scalar R AND v3.1 Task<T> for all v3.1-allowed T | -- |
| **A4.3** dynamic-N multi-task scheduling via `mut ref pool[i]` over `Vec<Task__<fn>>` | -- |
| A built-in-shaped `Pollable`/`Executor` pattern (2026-08-14, below) -- drives heterogeneous `Task__<fn>` types without a hand-rolled driver loop | Auto-generating the `implement Pollable for Task__<fn>` boilerplate in the v3.1 synthesizer, so it's zero boilerplate instead of one block per async fn |
| Per-dialect spellings: `अतुल्यकालिक` / `异步` / `非同期` for `async`; `प्रतीक्षा` / `等候` / `待機` for `await` | -- |
| Cancellable BLOCKING `task` threads: `cancel <name>;` (2026-08-14, signal-based `EINTR` interruption for `tcp_accept`/`tcp_recv` on POSIX, `CancelSynchronousIo` on Windows) -- see [Advanced 3 -- Concurrency](03_concurrency.md#cancel) | `stdin_read_line`/`file_read_line` cancellation (buffered stdio's `EINTR` interaction needs its own design pass); auto-generating the `implement Pollable for Task__<fn>` boilerplate above |

## An executor, not a hand-rolled driver: `Pollable` + `Executor`

Every driver loop on this page so far (`drive(...)`, the `select`
example) is specific to ONE task at a time, hand-written per
program. For more than a couple of concurrent tasks -- especially
DIFFERENT `Task__<fn>` shapes -- that gets repetitive fast. The
`Pollable`/`Executor` pattern below is a small, reusable, fully
tested building block that solves this generically, built entirely
from machinery this chapter already covers (`dyn` interfaces,
`Box<dyn Iface>`, `epoll_wait_one`):

```vani
interface Pollable {
  fn poll(self: mut ref Self) -> i64;
}

struct Executor {
  ep: i64,
  tasks: Vec<Box<dyn Pollable>>,
}

fn executor_new(ep: i64) -> Executor {
  let empty: Vec<Box<dyn Pollable>> = vec();
  return Executor { ep: ep, tasks: empty };
}

fn executor_spawn(ex: mut ref Executor, t: Box<dyn Pollable>) -> i64 {
  push(mut ref ex.tasks, t);
  return 0;
}

fn executor_run_to_completion(ex: mut ref Executor) -> i64 {
  let done: i64 = 0;
  while len(ref ex.tasks) > 0 as u64 {
    let round_size: u64 = len(ref ex.tasks);
    let i: u64 = 0;
    let remaining: Vec<Box<dyn Pollable>> = vec();
    while i < round_size {
      let t: Box<dyn Pollable> = pop(mut ref ex.tasks);
      let r: i64 = t.poll();
      if r == 0 - 2 {
        push(mut ref remaining, t);
      } else {
        done = done + 1;
      }
      i = i + 1 as u64;
    }
    ex.tasks = remaining;
    if len(ref ex.tasks) > 0 as u64 {
      let _ = epoll_wait_one(ex.ep, 100);
    }
  }
  return done;
}
```

For each `async fn` you want to run through it, write ONE small
`implement Pollable` block that just forwards to the already-
generated `__poll_<fn>`:

```vani
implement Pollable for Task__fetch_a {
  fn poll(self: mut ref Task__fetch_a) -> i64 {
    return __poll_fetch_a(self);
  }
}
```

Then spawn heterogeneous tasks (different `Task__<fn>` shapes) into
ONE executor and drive them all with a single call:

```vani
let ex: Executor = executor_new(ep);
let _ = executor_spawn(mut ref ex, box(fetch_a(fd_a) as dyn Pollable));
let _ = executor_spawn(mut ref ex, box(fetch_b(fd_b) as dyn Pollable));
let done: i64 = executor_run_to_completion(mut ref ex);
```

`executor_run_to_completion` returns how many tasks were accounted
for (completed OR cancelled) -- not their individual results. A
heterogeneous `Vec<Box<dyn Pollable>>` can't carry distinct result
types back out through one uniform return value; have each task's
own `poll()` body surface its result directly (print it, write it
into a shared `Mutex`, etc.).

**Why this is a copy-paste pattern, not a compiler builtin.** An
earlier attempt injected `Pollable`/`Executor` into the compiler's
universal prelude, the same mechanism `CancelToken`/`Future`/`Poll`
already use. It broke SSA-backend compilation for EVERY program --
not just ones using `Executor` -- because `Box<dyn Pollable>` is an
SSA-unsupported shape and the prelude is injected unconditionally.
Copy the block above into your own program instead; nothing is
injected, so there's no blast radius on programs that don't use it.

**Leak safety**: cancelling a task mid-flight (after it's already
past its first suspend point, holding real heap-owned locals) and
handing it to the executor is ASan/LeakSanitizer-verified clean --
see [`async_executor.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/english/async_executor.vani)
for the full worked example, including this cancellation case.

**Combining it with real-thread `cancel <name>;` and `vanic test`**:
see [Advanced 3e](03e_job_scheduler_capstone.md), a capstone that runs
this Executor pattern on the main thread at the same time as a
separate OS thread blocked in a real `tcp_accept()`, bounded by
`cancel` -- plus a `vanic test` harness asserting the two genuinely
don't interfere with each other.

## Common patterns

**Sequential awaits**: write them as you would Rust. The
desugar lowers each to a `match`.

```vani
async fn pipeline(n: i64) -> i64 {
  let a: i64 = await(fetch(n));
  let b: i64 = await(fetch(a));
  return b;
}
```

**Conditional await**: standard `if` works inside async
bodies.

```vani
async fn maybe_fetch(use_cache: bool, key: i64) -> i64 {
  if use_cache {
    return key;
  }
  return await(fetch(key));
}
```

## Selecting over multiple non-blocking polls: `select { await }`

When you have two or more non-blocking operations and want to
proceed with whichever is ready first, use `select`. **This is
NOT `select` over `async fn`/`Future<T>` values** -- despite the
`await` keyword in its syntax, `select`'s arms poll a raw
i64-returning nb-style call directly (the same family as
`tcp_recv_nb`/`tcp_accept_nb`/`epoll_wait_one`'s `-2` = WOULDBLOCK
convention, or a Task's generated `__poll_<fn>` if the async fn
has a real suspend point -- see "The real thing" above). A plain
`async fn` call like `fast(10)` returns `Future<i64>`, which
`select` rejects outright ("select arm poll expression must be
i64, got Future__i64") -- there is no synchronous-Future
integration with `select` in v1.

```vani
fn main() -> i64 {
  let server1: i64 = tcp_listen(0);
  let _ = tcp_set_nonblocking(server1);
  let server2: i64 = tcp_listen(0);
  let _ = tcp_set_nonblocking(server2);
  let port2: i64 = tcp_socket_port(server2);

  // Connects to server2 only -- server2's branch should win the race.
  task client {
    let _ = sleep_ms(20);
    let c: i64 = tcp_connect_local(port2);
    let _ = sleep_ms(50);
    let _ = tcp_close(c);
  }

  select {
    await tcp_accept_nb(server1) then c1 {
      print "server1 accepted first";
    }
    await tcp_accept_nb(server2) then c2 {
      print "server2 accepted first, fd > 0:", c2 > 0;
    }
  }

  join client;
  let _ = tcp_close(server1);
  let _ = tcp_close(server2);
  return 0;
}
```

Output:

```
server2 accepted first, fd > 0: true
```

`select` desugars to a `while true` loop that calls EVERY arm's
poll expression once per iteration (source order), checking each
result against the `-2` WOULDBLOCK sentinel; the first arm whose
call returns something other than `-2` runs its body and breaks
out. Because it's a plain spin loop (no `epoll_wait_one` between
rounds), it busy-polls the CPU until something is ready -- fine
for a short race like the example above, but pair it with your
own `epoll_wait_one`-based backoff if you're selecting over a
long-lived wait. Remaining branches are simply not polled again
once one wins; if they wrap a `Task`, that Task is left mid-flight.

<img class="manas" src="../images/mascot/manas_mascot_caution.png" title="this code needs extra care"/>

**Key constraints in v1**:
- Every poll expression inside `select` must independently type as
  `i64` -- a raw nb call, or a Task's `__poll_<fn>`, not a bare
  `async fn` call.
- Branches are polled in source order; there is no randomized
  fairness. If the first branch is always ready, the others
  never run.
- `select` may only appear inside a function body, not inside
  a `parallel for` body.

**Use `select` for**:
- Racing two fetch paths (cache vs network).
- Implementing a timeout: one branch awaits the real operation,
  another awaits a `sleep`-style timer.
- Processing whichever of several channels has data first.

## Challenge

Write an `async fn batch(xs: ref Vec<i64>) -> i64` that calls
`fetch` on each element, sums the results, and returns the
total. Add a `CancelToken` parameter; return `0 - 1` immediately
if `cancelled`.

---

**Previous**: [Sec.1a -- Async / await primer ->](01a_async_primer.md)
**Next**: [Sec.2a -- Parallelism and race-freedom primer ->](02a_parallelism_primer.md)
