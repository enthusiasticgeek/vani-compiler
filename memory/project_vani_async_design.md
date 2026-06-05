---
name: project-vani-async-design
description: "Async / asyncio for vāṇī — compiler-lowered state machines on an arena; explicitly NOT Rust-style Pin self-references"
metadata:
  node_type: memory
  type: project
---

User asked on 2026-05-27 to add async / asyncio to the TODO. The
canonical design lives in `/home/ptambe/vani/TODO.md` under *Async
/ asyncio — concurrency arc*. Summary here:

## Current status (2026-06-04)

**Arc 8 v1 ✅ SHIPPED** — the user-visible surface lands at the
parser + prelude level with synchronous semantics:

- `async fn foo() -> R { … return v; … }` desugars to
  `fn foo() -> Future<R> { … return Future.Ready(v); … }`.
  Body runs to completion synchronously on call.
- `await(expr)` desugars to a `match` that extracts
  `Future.Ready`'s payload.
- `Future<T>` and `Poll<T>` are prelude generic enums.
- `CancelToken` is a prelude struct `{ cancelled: bool }`;
  user threads it through async fns as `ref CancelToken` and
  reads `.cancelled` at relevant points.
- Foundation commits: `2e649ff` (prelude), `e50dc20` (async fn
  desugar), `25b5a84` (await + CancelToken). Example:
  [examples/async_await.vani](../examples/async_await.vani).

**Arc 8 v1.5 + v1.6 ✅ SHIPPED 2026-06-04** — incremental
slices on top of v1:

- `sleep_ms(ms: i64) -> i64` builtin (commit `d344828`)
  wraps POSIX `nanosleep` (EINTR retry) and emits via both
  tree-LLVM and tree-C. Used inside `async fn` bodies today
  to give real timer-driven flow with synchronous v1
  semantics. Walker recurses into TaskSpawn bodies so
  `sleep_ms` in a `task` works.
- `examples/async_io.vani` (commit `d209e06`) — timer-driven
  `async fn` + sequential awaits + `CancelToken` short-
  circuit + **concurrent timer fan-out via `task` + `join`**
  (real OS threads, ~30ms wall-clock for three 30ms sleeps).
- **Full TCP networking builtin family** (commit `9aaec41`):
  `tcp_listen` / `tcp_socket_port` / `tcp_accept` /
  `tcp_connect_local` / `tcp_send_str` / `tcp_recv` /
  `tcp_send_buf` / `tcp_close`. All wrap libc socket /
  bind / listen / accept / connect / send / recv / close.
  Thread-local 4KB recv buffer per OS thread. LLVM IR
  builds `sockaddr_in` by hand (16-byte stack alloca +
  htons port + htonl loopback addr). C side `#include
  <sys/socket.h>` + friends. Walker fix: string literals
  inside `task` bodies now intern at module scope.
  `examples/tcp_echo.vani` ships end-to-end loopback echo
  server + client in one process via `task` + `join`.
  Both backends produce byte-identical stdout (port `> 0`
  printed instead of literal so kernel-assigned ports
  don't break parity).

**Arc 8 FULLY COMPLETE 2026-06-04 + v3.1 Phase 0 + Phase 1
+ Phase 2 narrow + 2.1a-c + 2.2 + 2.3-narrow + 2.3a + 2.3b
+ 2.3c + 2.3d + 2.5 + 2.5b shipped**: every user-visible
async + networking + concurrency feature ships AND the
compiler-driven `async fn → Task` transform now auto-
generates state-machine struct/poll/constructor triples
for: linear async-fn bodies; non-suspending control flow
(if/while/Assign/Print + mid-body return); suspend-in-
branch state-splitting (if + while bodies); fall-through
merge state; nested ifs in suspending branches; ANF
lifting of `io_*_async` from compound expressions; match
expressions with suspending arms — desugared via Phase
2.3a/b/c/d to cover all literal + enum pattern shapes
(Int / Bool / Str / Float / Variant / VariantWithBinding /
Wildcard) through if-else-assign chains, tag-extraction
sub-matches, and inline substitution; break/continue
inside suspending loops. **v3.1 match-with-suspends
feature-complete.** See `ARC8_V3_PLAN.md` for the phased
plan-of-record (canonical source; this memory is a brief
overview only).

Fourteen acceptance examples cross-backend parity-green:

- `async_io.vani` — timer-driven async fn + task fan-out
- `tcp_echo.vani` — single-client TCP echo
- `tcp_multi_echo.vani` — 3 concurrent clients via tasks
- `tcp_echo_epoll.vani` — 3 concurrent clients on ONE
  OS thread via the epoll reactor
- `tcp_echo_state_machine.vani` — v3 hand-rolled state-
  machine pattern (struct + poll fn + driver loop) using
  `io_*_async` builtin aliases
- `tcp_echo_async.vani` — v3.1 Phase 1 compiler-driven
  `async fn → Task` transform (user writes ONLY the async-
  fn body + 5-line drive loop; compiler synthesizes the
  rest)
- `echo_with_timeout.vani` — v3.1 Phase 2 narrow control
  flow (input validation + outer-local Assign + mid-body
  return + suspend points)
- `timer_async.vani` — v3.1 Phase 0 timerfd-based
  cooperative sleep

Four concurrency models supported, user's choice:
1. Thread-per-task via `task` + `join` (race-free by the
   affine checker)
2. Single-thread cooperative via epoll + nb I/O variants
   (kernel multiplexing, no per-task threads)
3. Hand-rolled state-machine pattern (struct + poll fn +
   driver) using `io_*_async` aliases (v3 pattern)
4. **Compiler-driven `async fn → Task` (v3.1 Phase 1)** —
   user writes async-fn body; compiler synthesizes the
   Task struct + poll fn + constructor

**Arc 8 v3.1 sugar (compiler-driven state-machine codegen)
OPTIONAL** — parser-level transform that scans `async fn`
bodies containing `io_*_async` calls and auto-generates the
struct + poll fn + constructor triples that users write by
hand today. Multi-day compiler work (~500 lines: body scan +
StructDecl synthesis + Function synthesis + thread-local
registry flush + tests + lib tests + parity example).
Doesn't honestly fit a single session.

If/when v3.1 sugar lands, it would add:
- A new prelude type `Task<T> = { state_tag: i64, /* per-fn fields */ }`
  or per-fn `__TaskFor_<name>` struct synthesized by the
  transform
- A parser-level pass walking `async fn` bodies, finding
  `io_*_async(args)` calls (suspend points), splitting at
  each, emitting state struct + poll fn
- `intent_event_loop_run<T>(task: Task<T>) -> T` driver
  over the existing epoll primitives
- `examples/tcp_echo_async.vani` acceptance — same byte-
  identical runtime behavior as `tcp_echo_state_machine.vani`
  but with the struct/poll/constructor triple hidden behind
  `async fn` + `io_*_async` calls

Estimated v3.1 effort: ~15–20h focused for the linear-body
case; many MORE multi-day items per design caveat below.

## v3.1 design caveats (open questions for the implementer)

**Phased execution plan: see [ARC8_V3_PLAN.md](../ARC8_V3_PLAN.md)**
(5 v3.1 phases + 2 platform-port phases, total ~113-148h
across 16-25 sessions, each phase has explicit
scope/acceptance/effort/dependencies). Below is the design-
caveat summary captured in STATUS.md "v3.1 design caveats"
table:

1. **Body shape**: linear (Let + Print + Discard + Return)
   only at first; `if` / `while` / `for` / `match` / `try` /
   `break` / `continue` each need explicit state-machine
   handling and add multi-day implementation cost per
   construct.
2. **Local liveness analysis**: storing every local in the
   state struct is wasteful; cross-await liveness analysis
   is the right answer but adds complexity.
3. **Affine types across await**: `OwnedStr` / `Vec<T>` /
   user-struct locals that live across a suspend point must
   be moved into state with proper Drop-on-Pending-return.
   Initial v3.1 rejects non-i64 locals at suspend points.
4. **Multiple awaits in one expression**: needs ANF lifting
   pass before state-machine transform. Initial v3.1 rejects.
5. **`ref` / `mut ref` params**: can't store across heap-
   allocated state without lifetime tracking. Initial v3.1
   rejects.
6. **CancelToken auto-plumbing**: today the user reads
   `.cancelled` explicitly. v3.1 ideally auto-injects checks
   at every suspend point.
7. **Error propagation**: `io_*_async(-1)` hard error must
   flow back; integrate with `try` keyword / `Result<T, E>`.
8. **Nested async-fn calls**: `await(foo())` where `foo` is
   itself v3.1-transformed (state-machine composition with
   child state inside parent state).
9. **Generics**: monomorphize per (async-fn, type-args). Lean
   on closure #281 generic-decl infrastructure.
10. **Side effects between awaits** preserved (Print, Vec
    mutation, etc.); verify no affine-drop surprises.
11. **`intent_event_loop_run<T>(task) -> T`** as a real builtin
    (today the v3 pattern has the user write the driver loop
    by hand).
12. **Multi-task scheduling** — running multiple Tasks on one
    reactor needs `Vec<Task<T>>` (heterogeneous T = existential
    type or boxed-trait shape).
13. **`sleep_ms_async`**: timerfd-based, registers with epoll,
    yields without blocking. Today's `sleep_ms` blocks the
    calling OS thread.
14. **Diagnostic quality** when rejecting unsupported async-fn
    shapes — clear spans + suggested fix-ups.
15. **Test surface** for compiler-generated state machines —
    snapshot the typed IR + parity-test the runtime behavior.

## Platform support (Arc 8 runtime)

**Linux only today** — every Arc 8 v1.5/v1.6/v2/v3 helper
assumes glibc/musl + epoll + POSIX socket headers.

| Subsystem | Linux | macOS | Windows |
|---|---|---|---|
| `sleep_ms` | ✅ | ✅ (nanosleep works) | ❌ needs `Sleep(ms)` |
| Blocking TCP | ✅ | 🟡 untested | ❌ needs WSAStartup + winsock2 |
| `tcp_set_nonblocking` | ✅ | 🟡 untested | ❌ needs `ioctlsocket(FIONBIO)` |
| `__errno_location()` | ✅ glibc/musl | ❌ macOS uses `__error()` | ❌ Win32 uses `_errno()` |
| `epoll_*` | ✅ | ❌ needs **kqueue** shim | ❌ needs **IOCP** port (different programming model) |
| `task` + `join` | ✅ pthread | 🟡 likely (POSIX) | ✅ already wired via CreateThread |

Threading IS portable; I/O isn't. macOS port is ~8–12h
(kqueue shim matching epoll_* signatures); Windows port is
~25–35h (full IOCP redesign). Compile-time gate to fail loud
on non-Linux targets is a small follow-up (~1h) until ports
land.

**Affine flag: ⚠️ AFFINE-TENSION (compiler-lowered state machines)
/ 🛑 NON-COMPLIANT (Rust-style `Pin<&mut Self>` self-references).**

**Why:** stackless coroutines need to capture locals across
`await` points. Rust uses Pin + self-referential structs — but
that's 🛑 NON-COMPLIANT per [[project-vani-affine-standing]].

## Canonical path

- Compiler lowers each `async fn` body to an enum-of-frames; each
  frame is an owned affine struct in `Vec<StateMachine>` arena.
- Frames never hold raw pointers into other frames; cross-frame
  data flows by index or by move on suspend / resume.
- Single-threaded event-loop driver: `intent_async_run(task)`
  polls the root state machine until completion.
- `Future<T>` generic enum with `Ready(T)` / `Pending` variants
  (uses #281 generic decls + #283 mixed-payload lift; lives in
  prelude alongside Option / Result).
- `await` is statement-or-expression sugar that the checker
  rewrites at the state-machine boundary.
- Non-blocking I/O primitives (file / socket / timer) lower to
  epoll / kqueue / IOCP under the hood; user sees `async fn` in
  stdlib.
- `Channel<T, N>` is the cooperative coordination primitive —
  async tasks `recv` / `send`; event loop parks on channel-state
  changes.
- Cancellation: explicit `CancelToken` passed by-ref; checked at
  each suspend point. NOT panic / unwind.

## Explicitly NOT shipping

- 🛑 `Pin<&mut Self>` self-references
- 🛑 Panic-based cancellation
- 🛑 Stackful coroutines / fibers
- 🛑 Async inside `parallel for` bodies (use `task` + `join` for
  parallelism; async-of-tasks if you need both)

## Dependency chain (L-tier multi-session arc)

1. Closures w/ captured state (Level 3 #17 in data-structures
   roadmap) — prerequisite.
2. `Future<T>` generic enum + Poll interface.
3. `async fn` parser + checker (state-machine transform at check
   time).
4. State-machine codegen on both backends (frame arena + `poll`
   dispatch).
5. Event-loop C runtime (epoll / kqueue / IOCP wrappers).
6. Non-blocking I/O primitives (file / socket / timer as
   `async fn` in stdlib).
7. `await` statement-or-expression sugar.
8. Cancellation via `CancelToken`.
9. `examples/async_io.vani` — timer fan-out + tiny TCP echo
   server; cross-backend parity.

## How to apply

- Async ships AFTER Level 3 closures. Until then, the README's
  *Memory & runtime model* says async is "queued."
- Reject any proposal to ship `Pin` / self-referential async;
  point at this memory + [[project-vani-affine-standing]].
- Condition variables ([[project-vani-condvar-design]]) are a
  useful building block for the eventual event-loop runtime —
  not a prerequisite, but they land first and naturally.

Cross-references: [[project-vani-affine-standing]],
[[project-vani-data-structures-roadmap]],
[[project-vani-container-affine-contract]],
[[project-vani-condvar-design]].
