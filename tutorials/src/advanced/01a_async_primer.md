# Advanced 1a — Async, await, and Task (intuition primer)

> **Learning goal**: build a mental model of "asynchronous"
> programming — what `async fn` actually IS, how `await`
> works, what a state machine has to do with any of this. The
> formal chapter ([Advanced 1](01_async.md)) is dense; this
> chapter sets up the intuition first. Reading order:
> [02a parallelism primer](02a_parallelism_primer.md) → here
> → [Advanced 1 async/await](01_async.md).

This chapter has **no compiler code**. Pure intuition.

## The problem: waiting

Imagine a program that talks to a website. The program sends a
request, then waits for the response. The wait might be 100ms
— time during which the CPU has NOTHING TO DO with this
request. It's like ordering coffee and just standing there
staring at the barista for two minutes.

A clever person, while waiting for coffee, takes out their
phone and reads email. When the coffee is ready, the barista
calls their name; they put down the phone and pick up the
coffee. They've USED the wait-time for something else.

That's what `async` is about. Instead of having your program
sit idle during the 100ms wait, it can be doing OTHER work —
serving another request, computing something else, anything
useful. When the network response arrives, the program
"comes back" to where it was waiting and resumes.

The result: ONE OS thread can handle hundreds or thousands of
"in-flight" requests at once. Memory and CPU usage stay tiny.
The alternative (one thread per request) needs a megabyte or
two of stack per request — at 10,000 requests, that's 10-20 GB
of RAM just for stacks. Async lets you do the same work in a
few MB.

## Why is this different from parallelism (chapter 02a)?

**Parallelism** = doing TWO things AT THE SAME TIME on multiple
cores. Two computations running side-by-side.

**Asynchrony** = ONE thread juggling many *waiting* tasks.
While one task is waiting (for network, file, timer, etc.),
the thread switches to another task. They take turns; only
one is RUNNING CODE at any moment, but many are *in
progress*.

You can combine them: an async runtime with N threads, each
juggling many tasks. But the basic concept is
single-threaded — one juggler, many balls.

## The juggler analogy

A juggler holding 5 balls. Each ball is a task. At any instant
only one ball is in the juggler's hand (executing). The
others are arcing through the air (waiting for I/O).

The juggler's job: when a ball is about to come down, catch
it and throw it back up. ("It's about to come down" =
"the network response just arrived"; "throw it back up" =
"continue the task from where it was waiting".)

When a task hits an `await`, it's like the juggler tossing
that ball up high — the task is now "in the air", waiting for
some external thing. The juggler immediately turns attention
to the next ball.

## What `async fn` actually IS

This is the part that surprises CS-experienced readers coming
from JavaScript / Python — and is the entire point of vāṇी's
async story.

```vani
async fn fetch(fd: i64) -> i64 {
  let n: i64 = io_recv_async(fd, 64);
  return n;
}
```

This function LOOKS like a regular function with one extra
keyword. But the compiler does something surprising with it:
it rewrites it into a **state machine**.

The state machine is a struct + a `poll` function:

```
struct Task__fetch {
  state_tag: i64,    // which step are we on?
  fd: i64,           // saved parameter
  n: i64,            // saved local
}

fn __poll_fetch(t: mut ref Task__fetch) -> i64 {
  match t.state_tag {
    0 => {
      // Step 0: call io_recv_async
      let r = io_recv_async(t.fd, 64);
      if r == -2 { return -2; }    // not ready yet — yield
      t.n = r;
      t.state_tag = 1;
      // fall through to state 1
    }
    1 => {
      // Step 1: return value
      return t.n;
    }
  }
}
```

The struct REMEMBERS where in the function we are AND what
local variables have been computed. The `poll` function
advances by one step each time it's called. When a step has
to wait (the `-2` Pending signal), it returns immediately —
the juggler can switch to another task.

When the waiting thing becomes ready, the juggler calls
`poll` again. The state_tag tells the function where to
resume; the saved locals are still there in the struct. It's
as if the function "continued where it left off".

This is **the compiler-generated state machine** — vāṇी's
Arc 8 v3.1 work, which other docs reference. The user
writes the natural `async fn` syntax; the compiler emits all
this boilerplate.

## What `await` does

`await(some_async_call)` is the user-side keyword that says
"this is a suspend point". Where you see `await`, the
compiler-generated state machine inserts a state-transition.
After the await, code that ran before the await is "earlier
in the state machine"; code after is "later".

```vani
async fn handler(fd: i64) -> i64 {
  let req: i64 = await(io_recv_async(fd, 64));   // suspend point 1
  let resp: i64 = process(req);
  await(io_send_async(fd, resp));                 // suspend point 2
  return resp;
}
```

The compiler splits this into three states:
- State 0: call `io_recv_async`. If it yields Pending,
  return. If it returned a value, save in `req` and advance
  to state 1.
- State 1: compute `resp = process(req)`. Then call
  `io_send_async`. If Pending, return. Otherwise advance to
  state 2.
- State 2: return `resp`.

Each `await` is a state boundary.

## The driver / event loop

You can't just declare an async fn and have it run. Something
has to actually poll the resulting Task in a loop, calling
`__poll_X` over and over, sleeping (via `epoll_wait_one`)
between rounds when no task has progress to make.

```vani
fn drive(ep: i64, t: mut ref Task__fetch) -> i64 {
  while true {
    let r: i64 = __poll_fetch(t);
    if r != -2 { return r; }          // Ready or Error → done
    let _ = epoll_wait_one(ep, 1000); // Pending → wait for I/O
  }
  return 0;
}
```

This is the "juggler" — the event loop that drives the state
machine forward. vāṇी doesn't ship a built-in runtime; users
write small drivers like this. (A runtime built into the
language is queued; the underlying capability already ships.)

## When async is the right tool

Use async when:
- You have many I/O-waiting operations to interleave.
- Memory is tight (you can't afford thousands of OS threads).
- Latency matters more than throughput.

Examples: web servers, network proxies, streaming pipelines,
GUI event loops.

DON'T use async when:
- Your tasks are CPU-bound (no waiting — you'd just be
  juggling without any I/O to wait for). Use `parallel for`
  or `task` instead.
- You have only a few concurrent operations. The async
  overhead isn't worth the complexity.

## The cost — what async DOES NOT give you for free

1. **Latency between events.** A task progresses one step per
   poll. If the driver is slow to call `poll` after the I/O
   becomes ready, that's latency. Most runtimes (vāṇी's
   driver loop included) try to poll promptly, but the
   abstraction adds some overhead vs sync code.

2. **Cognitive complexity.** Async-fn bodies follow special
   rules: each suspend point splits the state. Affine types
   crossing suspend points have to live in the Task struct
   (heap memory). The full chapter ([Advanced 1](01_async.md))
   covers the rules.

3. **Debugging.** Stepping through a state machine with a
   debugger is weirder than stepping through a synchronous
   call stack. The "control flow" jumps between states.

4. **Cancellation.** Stopping a task partway through requires
   careful design — vāṇी's `CancelToken` + A4.4 auto-plumbing
   handles the common case automatically.

## A summary you can carry

- **Async** = ONE thread juggling many waiting tasks. Switches
  between them whenever one is waiting for I/O.
- **`async fn`** is rewritten by the compiler into a
  **state machine** — a struct (remembering progress + locals)
  plus a `poll` function (advances one step at a time).
- **`await`** marks suspend points where the state machine
  yields if the awaited thing isn't ready yet.
- A **driver / event loop** polls the state machine in a loop,
  sleeping on `epoll_wait_one` between rounds.
- Use async for I/O-heavy work with many concurrent tasks;
  use `parallel for` or `task` for CPU-bound work.

That's async. The next chapter ([Advanced 1](01_async.md))
shows the actual code — `async fn` + `await` + the driver
loop, with full state-machine examples.

## Cross-reference

- [Advanced 2a — Parallelism primer](02a_parallelism_primer.md)
  — parallelism (many cores) vs asynchrony (one thread,
  many tasks); the comparison
- [Beginner 6c — Ownership primer](../beginner/06c_ownership_primer.md)
  — why affine types crossing await points live in the Task
  struct (the state machine takes ownership)
- [Intermediate 12a — SMT primer](../intermediate/12a_smt_primer.md)
  — compile-time guarantees apply to async fns too;
  contracts on `async fn` compose normally
- [Advanced 1 — Async / await / Task transform](01_async.md)
  — the formal chapter with full syntax and state-machine
  decomposition
