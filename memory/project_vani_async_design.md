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

**Arc 8 runtime (8c + 8d + 8e non-blocking variants + 8h
cooperative fan-out) OPEN** — state-machine codegen,
epoll/kqueue event-loop runtime, real non-blocking I/O
futures (timerfd, O_NONBLOCK sockets), single-threaded
cooperative fan-out example. Picks up next session via
STATUS.md's "📋 NEXT SESSION" block. Estimated ~25–30h
focused. v1.5/v1.6 task-based fan-out + blocking I/O
remain useful alternative paths for users who don't need
single-thread cooperative scheduling.

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
