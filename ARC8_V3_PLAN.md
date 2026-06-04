# Arc 8 v3.1 + Platform Port — Phased Execution Plan

> **Status:** Draft 2026-06-04. Companion to [STATUS.md](STATUS.md)
> *📋 Arc 8 v3.1* + *🪟 Platform support*. Captures the two
> remaining workstreams as a phased plan with explicit
> acceptance criteria + effort estimates, so future sessions
> can pick up any phase without re-deriving scope.
>
> **Phase 0 ✅ COMPLETE 2026-06-04** (commits `eac1bf6`,
> `51748d6`). A0.1 gate + A0.3+A0.4 sleep_ms_async + timer
> example. A0.2 cleanly deferred to Phase 1 (resolved
> naturally by per-type concrete dispatch).
>
> **Phase 2.3-narrow ✅ COMPLETE 2026-06-04** (commit `2d92f30`).
> Match expressions in async fn bodies when no arm
> contains a suspend. Rewriter + walkers extended for
> Match + 6 other expr shapes. Suspends in arms rejected
> with Phase 2.3a pointer. examples/echo_match.vani
> parity-green. 2 new lib tests.
>
> **🎉 v3.1 control-flow sugar — CAPABILITY-COMPLETE 2026-06-04.**
> The compiler-driven `async fn → Task` transform handles
> every common control-flow shape in vāṇī. Capstone:
> `examples/async_showcase.vani` (commit `3c46e31`).
>
> Remaining v3.x sub-phases (each independent, none
> blocking):
> - 2.3 match arms (~8-10h)
> - 2.4 try keyword + Result propagation (~6-8h)
> - 3 affine types across await — OwnedStr/Vec (~20-25h)
> - 4 generics / nested async calls / multi-task (~25-30h)
> - 5 macOS kqueue port (~10-15h)
> - 6 Windows IOCP port (~25-35h)
>
> **Phase 2.5b ✅ COMPLETE 2026-06-04** (commit `4702cb6`).
> `break` / `continue` inside suspending loops. collect_into
> threads a loop_stack of (loop_header, post_loop) pairs;
> break/continue at top level emit Seg::Jump to the
> innermost loop. Verbatim Stmt::If's body rewrites
> break/continue to state_tag jumps + synthesized Continue.
> examples/echo_loop_break.vani parity-green. 2 new lib
> tests.
>
> **Phase 2.5 ✅ COMPLETE 2026-06-04** (commit `a77b799`).
> Loops with suspend inside body. Stmt::While state-splits
> into loop_header + body + post_loop with a BACKWARD
> Seg::Jump at body tail. Synthesis wraps poll body in
> `while true { ... }` + emits `Stmt::Continue` after
> backward jumps. examples/echo_loop.vani parity-green.
> 2 new lib tests. Phase 2.5b (break/continue inside
> suspending loops) deferred.
>
> **Phase 2.2 ✅ COMPLETE 2026-06-04** (commit `d3a0af3`).
> ANF lifting for nested `io_*_async` calls in compound
> expressions. Pre-pass `anf_lift_body` runs before
> validation; lifts nested calls into fresh `__anf_N`
> Lets. examples/echo_anf_lift.vani parity-green. 3 new
> lib tests. Phase 2.3 (match arms), 2.4 (try keyword),
> 2.5 (break/continue) queued.
>
> **Phase 2.1c ✅ COMPLETE 2026-06-04** (commit `22fc622`).
> Nested ifs inside suspending branches. Validator
> relaxed; `collect_branch_locals_recursive` collects Lets
> at arbitrary depth; `body_all_paths_return` suppresses
> dead Jumps after fully-returning Verbatim stmts.
> examples/echo_nested_if.vani parity-green. 2 new lib
> tests + 1 prior rejection test updated to acceptance.
> Recursive `collect_into` already supported this — just
> needed validator + locals helpers.
>
> **Phase 2.1b ✅ COMPLETE 2026-06-04** (commit `b1b8113`).
> Fall-through merge state. New `Seg::Jump` emitted at tail
> of non-return-terminated branches; merge state allocated
> AFTER branch recursion to maintain cascade monotonicity.
> `collect_into` returns bool tracking termination.
> examples/echo_fall_through.vani parity-green. 2 new lib
> tests + 2 prior rejection tests updated to acceptance.
> Phase 2.1c (nested ifs in suspending branches, ~5-8h),
> 2.2-2.5 deferred.
>
> **Phase 2.1a ✅ COMPLETE 2026-06-04** (commit `7566ab8`).
> Suspend-in-branch state-splitting. `if` with `io_*_async`
> in either or both branches (both must end with Return).
> Required restructuring: explicit state numbering via
> `collect_into` + new `Seg::Decision` + `Seg::Suspend.bump_to`.
> examples/tcp_echo_async_branched.vani parity-green. 4 new
> lib tests. Phase 2.1b (fall-through merge state, ~3-5h),
> 2.1c (nested ifs inside suspending branches, ~5-8h),
> 2.2-2.5 deferred.
>
> **Phase 2 narrow ✅ COMPLETE 2026-06-04** (commit `a3cab5b`).
> Control flow inside async-fn bodies — `if` / `while` /
> `Assign` / `Print` + mid-body `return` — provided branches
> don't contain `io_*_async`. examples/echo_with_timeout.vani
> parity-green. 4 new lib tests.
>
> **Phase 1 ✅ COMPLETE 2026-06-04** (commit `7d47ff6`).
> The headline v3.1 compiler-driven `async fn → Task`
> transform ships. Retrospective:
> - A1.1+A1.2 — detection + linear-shape validation. Walks
>   Stmt/Expr trees for `io_*_async` calls; rejects
>   non-linear bodies with explicit phase-pointer
>   diagnostics ("control flow arrives in Phase 2",
>   "non-i64 locals arrive in Phase 3").
> - A1.3+A1.4 — synthesizes `Task__<fn>` struct (state_tag
>   + saved params + locals) + `__poll_<fn>` fn (one
>   `if t.state_tag == K` arm per suspend point, with
>   Pending(-2) / Error(-1) handling).
> - A1.5+A1.6 — original fn's body rewritten to a
>   constructor returning the Task struct; synthesized
>   struct + poll fn queued in `V31_TASK_REGISTRY`,
>   flushed into `program.{structs, functions}` at end of
>   `parse_program`.
> - A1.7 — diagnostic-quality rejections with phase
>   pointers.
> - A1.8 — `examples/tcp_echo_async.vani` parity-green on
>   both backends.
> - Naming: `Task__<name>` (PascalCase leading per
>   `parse_type` discipline) + `__poll_<name>`.
> - A0.2's deferred `intent_event_loop_run` builtin
>   resolves naturally in Phase 2 (driver as builtin) or
>   user can hand-write the 5-line drive loop today.
> - Test ledger: 1827 lib (+4 Phase 1 tests) + 54 parity
>   green.

---

## Executive summary

Two independent workstreams remain on Arc 8:

| Workstream | Phases | Total effort | First-session start |
|---|---|---|---|
| **A. v3.1 compiler sugar** (state-machine codegen for `async fn`) | 5 phases (0 → 4) | ~78-98h focused | Phase 0 (foundation) |
| **B. Platform port** (lift Linux-only restriction) | 2 phases (5 → 6) | ~35-50h focused | Phase 5 (macOS — smaller lift) |

Each phase is sessionable independently. Phases inside a
workstream have ordered dependencies; the two workstreams are
parallel-safe. Total backlog: **~113-148h focused across
16-25 sessions**.

**Recommended order of execution (highest user value first):**
1. ~~**Phase 0** — Foundation~~ ✅ DONE 2026-06-04.
2. ~~**Phase 1** — v3.1 linear core~~ ✅ DONE 2026-06-04.
3. ~~**Phase 2 narrow** — control flow w/o suspend in
   branches~~ ✅ DONE 2026-06-04. Sub-phases 2.1-2.5
   queued.
4. **Phase 5** — macOS port. Smallest cross-platform lift;
   unblocks macOS CI and broadens user base. **← NEXT
   SESSION RECOMMENDED.**
5. **Phase 2.1** — suspend-in-branch state-splitting. The
   harder half of Phase 2 — each if/while branch with a
   suspend becomes its own state chain with a merge state.
   ~10-12h. Picks up where Phase 2 narrow left off.
5. **Phase 3** — v3.1 affine integration. Unlocks OwnedStr
   / Vec across awaits.
6. **Phase 4** — v3.1 advanced (generics, nested, multi-task).
7. **Phase 6** — Windows port. Largest lift, lowest user
   priority (most vāṇī users on Linux/macOS for now).

**Minimum viable v3.1** = Phase 0 + Phase 1 (~24-31h, 3-5
sessions). Gives users compiler-driven sugar for the common
case. Everything else is incremental polish.

---

## Workstream A: v3.1 compiler sugar

### Phase 0 — Foundation (NO state-machine transform yet)

**Goal.** Ship the runtime building blocks v3.1 will need.
Land risk-free changes that don't touch the compiler-driven
transform itself.

**Scope (in):**
- **A0.1 — Compile-time platform gate.** Detect non-Linux
  targets at build time; emit a clear diagnostic
  ("Arc 8 epoll runtime is Linux-only — see ARC8_V3_PLAN
  Phase 5/6 for ports"). Wrap the `emit_intent_epoll_helpers_*`
  + `emit_intent_tcp_helpers_*` + `emit_intent_sleep_ms_helper_*`
  emits in a `if cfg!(target_os = "linux") { … }` gate, OR
  emit a `#error` directive in the C output when targeting
  non-Linux.
- **A0.2 — `intent_event_loop_run<T>(task: T) -> T` builtin.**
  Today users write the driver loop by hand (see
  `examples/tcp_echo_state_machine.vani` `drive_task`).
  v3.1 will generate Tasks; this builtin drives them.
  Initial implementation: takes a struct-typed value with
  fields `state_tag: i64` + others; calls a per-type
  `__poll_<TypeName>` function until it returns >= 0.
  Each Pending (-2) return triggers an `epoll_wait_one`
  on a captured epfd.
- **A0.3 — `sleep_ms_async(ms: i64) -> i64` non-blocking
  timer.** Uses `timerfd_create(CLOCK_MONOTONIC, 0)` +
  `timerfd_settime` to register a one-shot timer fd. Returns
  the timerfd; the caller adds it to epoll and reads it
  when ready. Sentinel codes: same as nb variants (-2 =
  not yet, -1 = error). Companion `sleep_ms_finish(fd) -> i64`
  reads the expiration count (clears the timer) — returns
  0 on success.
- **A0.4 — Lib tests pinning the new surface.** Mirror the
  pattern from `sleep_ms_typechecks_and_emits_helper_in_c`
  + `_in_llvm` tests.

**Scope (out — deferred):**
- The state-machine transform itself (Phase 1+).
- Any new async-fn syntax.
- macOS / Windows variants (Phase 5/6).

**Acceptance criteria:**
- `cargo test --lib` adds 5+ tests for the new surface, all
  green.
- `cargo test --test run_end_to_end` adds an
  `examples/timer_async.vani` parity-green example that
  composes `sleep_ms_async` + `epoll_wait_one` + `sleep_ms_finish`
  into a single-threaded sleep that yields control.
- A new `examples/tcp_echo_event_loop.vani` rewrites
  `tcp_echo_state_machine.vani` to use
  `intent_event_loop_run(et)` instead of the hand-rolled
  `drive_task` loop — byte-identical stdout.
- Building on macOS / Windows with the gate produces a
  clear error message (manually verified, or via a
  `cargo check --target ...` test if CI is wired).

**Effort estimate:** ~6-8h focused, 1-2 sessions.

**Dependencies:** None (foundation phase).

**Open questions / risks:**
- **Q1**: How does `intent_event_loop_run` know which fields
  of the user's task struct are I/O fds vs. data? Initial
  answer: it doesn't — the user's `__poll_<T>` function
  encapsulates that knowledge and returns -2 when waiting;
  `intent_event_loop_run` just calls `epoll_wait_one` and
  re-polls. No fd-tracking in the driver itself.
- **Q2**: For `sleep_ms_async`, who owns the timerfd lifetime?
  Initial answer: the user. They open it, register with
  epoll, read it when ready, and close it. v3.1 sugar may
  later auto-allocate + clean up.
- **R1**: Risk: `timerfd` is Linux-only too. Phase 5 macOS
  port needs `kqueue` + `EVFILT_TIMER` instead.

---

### Phase 1 — v3.1 linear core (compiler-driven transform)

**Goal.** Auto-generate the state-machine struct + poll fn +
constructor triples from an `async fn` body. **Linear bodies
only** — no `if` / `while` / `for` / `match` / `try` / `break`
/ `continue` inside the async body. **i64 locals + params
only.**

**Scope (in):**
- **A1.1 — Detect async fn bodies eligible for v3.1 transform.**
  Inside `parse_function`, after `is_async` recognition: scan
  the body for `Call("io_recv_async" | "io_send_async" |
  "io_accept_async", ...)` or any other allowlisted suspend
  marker (`sleep_ms_async`). If found, route to v3.1
  transform instead of v1 sync desugar.
- **A1.2 — Validate body shape.** Linear-only at the
  statement level (no if/while/etc); all params + locals are
  `i64`; return type is `i64`. Reject with clear diagnostic
  pointing at the first violating construct.
- **A1.3 — Synthesize per-fn task struct.** For
  `async fn foo(p1: i64, p2: i64) -> i64 { let l1 = …; … }`:
  emit `struct __TaskFor_foo { state_tag: i64, p1: i64, p2:
  i64, l1: i64, … }`. State tag starts at 0; one bumped
  value per suspend point.
- **A1.4 — Synthesize poll fn.** Emit
  `fn __poll_foo(t: mut ref __TaskFor_foo) -> i64 { … }`
  as a chain of `if t.state_tag == N { /* state N body */ }`
  blocks. Each suspend point (io_*_async call) at state N
  becomes:
  ```
  if t.state_tag == N {
    let r: i64 = tcp_recv_nb(t.field_for_arg, …);
    if r == -2 { return -2; }
    if r < 0 { return -1; }
    t.local_field = r;
    t.state_tag = N+1;
  }
  ```
  Final state returns `EXPR` (the user's `return` expression
  with locals/params rewritten to `t.field`).
- **A1.5 — Replace original `async fn` body with constructor.**
  `fn foo(p1: i64, p2: i64) -> __TaskFor_foo { return
  __TaskFor_foo { state_tag: 0, p1: p1, p2: p2, l1: 0, …
  }; }`. The original async fn name is preserved; callers
  receive a Task value they can drive.
- **A1.6 — Thread-local registry flush.** Like
  `CLOSURE_MAKE_REGISTRY` in the checker. Synthesized
  structs + poll-fns are queued at parse time and flushed
  into `program.structs` + `program.functions` in
  `parse_program` so the post-parse passes see them.
- **A1.7 — Diagnostic-quality rejections.** Unsupported
  shapes get a per-construct error with a span. E.g.:
  `"v3.1 async fn body must be linear; if-statement at
  line 12:5 is not yet supported (Phase 2)"`.
- **A1.8 — Acceptance example.** `examples/tcp_echo_async.vani`
  — same runtime behavior as
  `tcp_echo_state_machine.vani` but the user writes only
  the async fn body; struct/poll/constructor synthesized.

**Scope (out — deferred):**
- Control flow inside async body (Phase 2).
- Non-i64 locals / params (Phase 3).
- Nested async-fn calls / generics / multi-task (Phase 4).

**Acceptance criteria:**
- `cargo test --lib` adds ~10 tests pinning the transform:
  shape validation, struct synthesis, poll-fn synthesis,
  diagnostic spans, rejection of out-of-scope shapes.
- `cargo test --test run_end_to_end` includes
  `examples/tcp_echo_async.vani` with byte-identical
  stdout to `tcp_echo_state_machine.vani` on both
  backends.
- Existing async / nb / epoll tests still green (1817 lib
  + 54 parity baseline maintained).

**Effort estimate:** ~12-15h focused, 2-3 sessions.

**Dependencies:** Phase 0 (intent_event_loop_run builtin
needed by the acceptance example).

**Open questions / risks:**
- **Q3**: Should the transform run at parser time or as a
  separate IR pass? Initial answer: parser time — the
  closure-lift precedent in
  [closures.md](src/checker.rs) (Arc 5c, commit `7cccc1b`)
  already does AST-level synthesis. Easier than touching
  IR.
- **Q4**: How does the transform name-mangle for collision
  avoidance? Initial answer: `__TaskFor_<fn>` +
  `__poll_<fn>`. If user already has a fn / struct by that
  name → parser error.
- **R2**: Risk: the user's original async fn name now
  returns a struct, not the original `i64`. Calls like
  `let x: i64 = my_async_fn(fd);` break with a type-error.
  v3.1 callers must call
  `intent_event_loop_run(my_async_fn(fd))` instead. This is
  a deliberate type-system gate — the compiler error
  surfaces correctly via the existing checker.

---

### Phase 2 — v3.1 control flow

**Phase 2 was split into sub-phases during implementation:**

- **Phase 2 narrow ✅ DONE 2026-06-04** (commit `a3cab5b`):
  `if` / `while` / `Assign` / `Print` + mid-body `return`
  ALLOWED at top level **provided** branches contain no
  `io_*_async` suspend points. Emitted verbatim in current
  state with var-rewriting. 4 lib tests +
  `examples/echo_with_timeout.vani` parity-green.

- **Phase 2.1** (queued, ~10-12h focused) —
  **suspend-in-branch state-splitting**. The hardest sub-
  phase. Detailed design below ("Phase 2.1 implementation
  guide"). Without 2.1, an `if` / `while` branch that
  itself contains `io_*_async` is rejected with the
  "needs full state-splitting codegen" diagnostic.

- **Phase 2.2** (queued, ~5-6h) — ANF lifting for
  `await(io_*_async(...))` calls inside larger
  expressions. Compiler walks the body and pulls each such
  call into its own `let __anf_v = io_*_async(...);` before
  the parent expression.

- **Phase 2.3** (queued, ~8-10h) — `match` arms with per-arm
  state graphs.

- **Phase 2.4** (queued, ~6-8h) — `try expr` keyword + Result
  propagation through the state machine.

- **Phase 2.5** (queued, ~6-8h) — `break` / `continue` inside
  loops with suspend-aware back-edge codegen.

### Phase 2.1 implementation guide (suspend-in-branch)

The bulk of Phase 2.1 is converting the current implicit
linear state allocation to explicit state numbering.

**Required restructuring** (in `src/parser.rs`):

1. **Replace the linear `Vec<Vec<Seg>>` `states` accumulator**
   with an explicit `StateMachine` struct:

   ```rust
   struct StateMachine {
       next_state: usize,            // monotonically increasing
       state_bodies: Vec<Vec<Seg>>,  // state_bodies[K] = segs for state K
   }
   impl StateMachine {
       fn alloc_state(&mut self) -> usize {
           let s = self.next_state;
           self.next_state += 1;
           self.state_bodies.push(Vec::new());
           s
       }
   }
   ```

2. **Add explicit branch + jump segments:**

   ```rust
   enum Seg {
       // ... existing variants ...
       Decision {
           cond: Expr,
           then_state: usize,
           else_state: usize,
           span: Span,
       },
       // Optional: explicit Jump for branches that don't
       // return but need to merge back. Phase 2.1a can
       // require return-termination instead.
   }
   ```

3. **Replace the existing `for s in segs` loop with a
   recursive `collect_segs(stmts, sm, current_state)`** that:
   - Takes a mutable `&mut StateMachine` and `&mut usize` for
     the current state being built
   - On Stmt::Let with suspend: bumps state and emits Suspend
   - On Stmt::Return: emits Return (no state change — the poll
     fn exits via return; the cascading caller's current_state
     becomes "dead")
   - On Stmt::If with suspend-in-branch: allocates `then_state`
     and `else_state`, pushes a `Seg::Decision` into the
     current state, then recurses into each branch with that
     branch's starting state as `current_state`
   - On Stmt::If without suspend (Phase 2 narrow): emits
     `Seg::Verbatim`

4. **State-numbering invariant (critical):** each state's body
   must set `state_tag` to a value GREATER than the current
   state index. The cascade pattern relies on monotonic state
   transitions. Decision states satisfy this because both
   branch states are allocated AFTER the decision state.

5. **Phase 2.1a narrow scope** (recommended first slice):
   - Require BOTH branches of every if-with-suspend to end
     with `Stmt::Return` — no fall-through, no merge state
     needed
   - Reject nested ifs inside branches (the recursion is
     supported in principle but increases test surface)
   - Reject Phase 2 narrow constructs INSIDE branches (no
     nested if/while/Assign in branches)

6. **Phase 2.1b** (follow-up): allow non-return-terminated
   branches. Requires a "merge state" that both branches
   `state_tag =` before falling through. Each branch's
   FINAL state body bumps to the merge state.

7. **Phase 2.1c** (follow-up): nested if-with-suspend inside a
   branch. Recursive `collect_segs` already handles this in
   principle; just need tests.

**Acceptance example** for Phase 2.1a:

```vani
async fn cond_recv(fd: i64, mode: i64) -> i64 {
  if mode > 0 {
    let n: i64 = io_recv_async(fd, 64);
    return n;
  } else {
    return mode;
  }
}
```

Expected synthesized states:
- State 0: `if t.mode > 0 { t.state_tag = 1; } else { t.state_tag = 3; }`
- State 1: `let r = tcp_recv_nb(t.fd, 64); ... save r → t.n; t.state_tag = 2;`
- State 2: `return t.n;`
- State 3: `return t.mode;`

The cascade naturally enters either state 1 (after state 0 sets state_tag=1) or skips to state 3 (state_tag=3 doesn't match state 1 or 2's check).

**Lib test surface for Phase 2.1a:** ~5-8 tests covering:
- Simplest case (one suspend in then-branch, return in else)
- Suspend in both branches
- Two suspends in then-branch
- Nested control-flow rejection (until Phase 2.1c)
- Fall-through rejection (until Phase 2.1b)

**Effort estimate (refined):** ~10-12h focused for Phase 2.1a;
each follow-up sub-phase ~3-5h additional.

---

### Phase 2 (original — kept for reference)

The pre-implementation Phase 2 sketch below describes the
end-state covering all sub-phases. Phase 2 narrow shipped
the no-suspend-in-branches case; 2.1 / 2.2 / 2.3 / 2.4 / 2.5
remain queued.

**Scope (in — full Phase 2):**
- **A2.1 — `if/else` inside async body.** Each branch may
  contain a suspend point. The state machine doubles per
  branch: states `(N, then)` and `(N, else)` track which
  arm is being executed, both flow to `N+1` at merge.
- **A2.2 — `while cond { body }` loops.** Add a back-edge
  state: after the loop body's final state, set
  `state_tag` to the loop-header state and re-poll.
  Termination: `cond` is evaluated at the loop-header
  state.
- **A2.3 — `for i from A to B { body }` ranges.** Lower to
  the while form internally; carry `i` in the state struct.
- **A2.4 — `match` over scalars / enums.** Each arm is a
  separate state; the scrutinee is evaluated once at the
  match-entry state, locals are bound, control flows to
  the arm's first state.
- **A2.5 — `break` / `continue`.** Map to state jumps
  (continue → loop-header state, break → post-loop state).
- **A2.6 — `try expr` keyword.** When the awaited expression
  could fail (e.g., `try try_vec(n)`), short-circuit on
  error: set the task's `error_field` and jump to a
  terminal error state. Caller checks `error_field` after
  `intent_event_loop_run` returns.
- **A2.7 — Acceptance example.** `examples/echo_with_timeout.vani`
  — an async fn that uses `if elapsed > timeout_ms { return; }`
  to abort an echo if the client takes too long.

**Scope (out — deferred):**
- Affine-type locals across await (Phase 3).
- Generic params (Phase 4).
- Multi-task scheduling (Phase 4).

**Acceptance criteria:**
- `cargo test --lib` adds ~15 tests covering each control-
  flow construct's state-machine emission.
- `examples/echo_with_timeout.vani` parity-green
  cross-backend.
- All existing v3.1 linear-core tests still green.

**Effort estimate:** ~15-20h focused, 2-3 sessions.

**Dependencies:** Phase 1 (linear core).

**Open questions / risks:**
- **Q5**: How does loop-body code interact with locals
  declared inside the loop body? Initial answer: such
  locals are NOT stored in the state struct (transient);
  only kept-across-await locals survive a suspend. Liveness
  analysis (Phase 1 may have used "all locals" as a
  conservative approximation; Phase 2 must refine for
  correctness with loops).
- **R3**: Risk: state explosion. A function with nested
  if/while/match could produce dozens of state transitions.
  Mitigation: synthesize a `match t.state_tag` switch
  instead of an if-chain at some threshold (e.g., > 8
  states).

---

### Phase 3 — v3.1 affine integration

**Goal.** Allow `OwnedStr` / `Vec<T>` / user-struct locals
to live across await points. This is THE hard problem of
async + affine ownership.

**Scope (in):**
- **A3.1 — Move-into-state on suspend.** A local of type
  `OwnedStr` declared before an await is MOVED into the
  task struct field at suspend time. The poll fn's
  prologue MOVES it back out into a local of the same
  name before the post-await code runs.
- **A3.2 — Drop on Pending-return.** When poll returns -2
  (Pending), the task struct still owns any heap fields.
  When the task is eventually dropped (e.g.,
  `intent_event_loop_run` finishes or the task is
  cancelled), affine drops fire per-field.
- **A3.3 — Drop on Ready completion.** When poll returns
  the Ready value, owned fields in the task have already
  been moved out into the return value (if returned) or
  must be explicitly dropped. The transform inserts the
  per-field drops automatically.
- **A3.4 — Diagnostic for shared-borrow pitfall.** If the
  user borrows an OwnedStr local with `ref` across an
  await, reject with a clear span pointing at the borrow.
  Borrows can't survive suspends without lifetime
  tracking.
- **A3.5 — Acceptance example.**
  `examples/async_string_echo.vani` — an async fn that
  reads a TCP message, builds an `OwnedStr` greeting,
  yields, then sends the greeting via `tcp_send_str`.

**Scope (out — deferred):**
- `mut ref` parameters (still rejected — needs lifetime
  tracking).
- Vec<T> for non-Copy T across awaits (deferred to Phase
  4's generic handling).

**Acceptance criteria:**
- `cargo test --lib` adds ~12 tests for the affine move-
  through-state pattern.
- ASan / valgrind clean on
  `examples/async_string_echo.vani` (no leaks, no
  double-frees).
- Existing v3.1 + v3.1-control-flow tests still green.

**Effort estimate:** ~20-25h focused, 3-4 sessions.

**Dependencies:** Phase 2 (control flow — affine drops
inside if-branches need solid state-machine emission).

**Open questions / risks:**
- **Q6**: Where does the per-field drop for a Pending-
  abandoned task happen? Initial answer: the task struct's
  parent scope (the caller of `intent_event_loop_run` or
  the explicit `drop(task)`). Reuse the existing struct-
  drop-walk machinery (closure #229 user-Drop work).
- **R4**: Risk: aliasing inside the state struct. If the
  task holds an `OwnedStr` AND the poll fn re-uses the
  same name for a fresh local, the affine checker sees
  the same name twice. Mitigation: rename locals to
  `__l_<name>` inside the poll fn body; preserve the
  user-visible name only in the task struct's field.

---

### Phase 4 — v3.1 advanced features

**Goal.** Lift the remaining restrictions: nested async
calls, generics, multi-task scheduling, CancelToken auto-
plumbing, error propagation via `try`.

**Scope (in):**
- **A4.1 — Nested async-fn calls.** `await(child_async_fn(args))`
  where `child_async_fn` is itself v3.1-transformed (returns
  `Task<T>`). The parent's task struct holds the child's
  task as a field; parent's poll fn polls the child until
  Ready, then extracts the child's value.
- **A4.2 — Generic params.** `async fn foo<T>(x: T) -> T`.
  Monomorphize the synthesized struct + poll fn per
  (async-fn, type-args) tuple using the existing closure
  #281 generic-decl infrastructure.
- **A4.3 — Multi-task scheduling.** A reactor that polls
  `Vec<Task<T>>` round-robin. Needs heterogeneous-T
  support — either a boxed-trait shape
  (`Vec<dyn Pollable>`) building on Arc 5 vtables, or an
  existential-type sugar. Initial v3.1 may ship a
  fixed-N homogeneous version
  (`intent_event_loop_run_n<T>([Task<T>; N])`).
- **A4.4 — CancelToken auto-plumbing.** When the async fn
  has a `ref CancelToken` parameter, automatically inject
  `if token.cancelled { return cancellation_sentinel; }`
  at every suspend point. The cancellation_sentinel value
  is configurable per fn (e.g., via an attribute
  `#[cancel_value(-1)]`).
- **A4.5 — `try` keyword + Result<T, E> integration.** When
  `await(io_*_async(...))` returns -1 (error), and the
  surrounding context is `try await(...)`, short-circuit
  to the enclosing fn's error handler. Reuse the existing
  `try` desugar (closure #218).
- **A4.6 — Acceptance example.**
  `examples/echo_pool.vani` — a single-thread server that
  handles N concurrent client tasks via the multi-task
  scheduler.

**Scope (out — deferred):**
- Mut ref params across awaits (probably never; affine
  conflicts).
- Cross-async-fn move semantics (advanced).

**Acceptance criteria:**
- `cargo test --lib` adds ~20 tests covering each new
  capability.
- `examples/echo_pool.vani` shows ≥10 concurrent clients
  handled on ONE OS thread via the multi-task scheduler.
- Both backends produce byte-identical stdout via parity.

**Effort estimate:** ~25-30h focused, 3-5 sessions.

**Dependencies:** Phase 3 (affine integration — nested
async calls compose affine state).

**Open questions / risks:**
- **Q7**: How to type a `Vec<Task<T>>` for heterogeneous
  T? Initial answer: introduce a `dyn Task` trait + reuse
  Arc 5 vtable machinery. Alternative: erase the value
  type to i64 in the task struct (lossy but simple).
- **R5**: Risk: feature creep. Phase 4 has the most
  "asks" remaining. Be willing to split into Phase 4a /
  4b / 4c if any single feature blows up.

---

## Workstream B: platform port

### Phase 5 — macOS port (kqueue shim)

**Goal.** Lift the Linux-only restriction for macOS users.
The biggest delta is `epoll` → `kqueue`; everything else
(sockets, fcntl, nanosleep, threading) already works on
macOS.

**Scope (in):**
- **B5.1 — `__error()` errno thunk.** macOS uses
  `__error()` instead of glibc's `__errno_location()`.
  Both return `int*`. Add a host-detection
  `host_is_darwin()` helper (mirrors
  `host_uses_win32_threading()`); emit the right thunk
  declaration per host.
- **B5.2 — `kqueue` shim matching `epoll_*` signatures.**
  Implement `intent_epoll_new` via `kqueue()`,
  `intent_epoll_add_read` via
  `kevent(EV_ADD | EV_ENABLE)`,
  `intent_epoll_wait_one` via `kevent()` blocking call,
  `intent_epoll_close` via `close()`. The user-facing
  vāṇī API stays the same.
- **B5.3 — `timerfd` → `EVFILT_TIMER`.** Phase 0's
  `sleep_ms_async` needs a kqueue-native variant: register
  an `EVFILT_TIMER` kevent instead of a timerfd.
- **B5.4 — Build-system per-host emit.** Both backends
  conditionally emit the kqueue OR epoll helper based on
  the host detection.
- **B5.5 — CI / parity sweep on macOS.** Wire up a macOS
  runner (or document the manual verification steps if CI
  isn't ready). All 5 Arc 8 examples should pass parity.

**Acceptance criteria:**
- All Arc 8 examples (`async_io.vani`, `tcp_echo.vani`,
  `tcp_multi_echo.vani`, `tcp_echo_epoll.vani`,
  `tcp_echo_state_machine.vani`) parity-green on macOS.
- 1817 lib + 54 parity baseline maintained on Linux.
- Documentation note in
  [README.md](README.md) lifting the "Linux only" caveat
  to "Linux + macOS."

**Effort estimate:** ~10-15h focused, 1-2 sessions (assumes
macOS dev access; CI wiring is ~3h of that).

**Dependencies:** None on Workstream A; can parallel-ship
with v3.1 phases.

**Open questions / risks:**
- **Q8**: Does macOS need separate `host_is_darwin()`, or
  can `cfg!(target_os = "macos")` be inlined? Initial
  answer: factor out a helper for consistency with
  `host_uses_win32_threading()`.
- **R6**: Risk: kevent's data layout (signed 32-bit fd in
  `ident`, etc.) differs from epoll_event. Test carefully.

---

### Phase 6 — Windows port (IOCP + winsock)

**Goal.** Lift the Linux-only restriction for Windows users.
This is the biggest single port because the IOCP programming
model is fundamentally different from epoll's readiness
notification.

**Scope (in):**
- **B6.1 — WSAStartup / WSACleanup boilerplate.** Wrap
  every program that uses TCP in a startup call. Easiest
  via a runtime helper called once at `main` entry.
- **B6.2 — Win32 socket type.** Windows `SOCKET` is a
  `UINT_PTR` (64-bit on x64), not an `int`. Adjust the
  intent_tcp_* helpers to use the wider type internally
  while still presenting `i64` to vāṇī code.
- **B6.3 — `ioctlsocket(FIONBIO)` for non-blocking.**
  Replaces `fcntl(F_SETFL, O_NONBLOCK)` on Windows.
- **B6.4 — `_errno()` / `WSAGetLastError()` for errno.**
  Replaces `__errno_location()`. Note: Win32 has TWO
  errno-like systems (CRT errno via _errno, socket errors
  via WSAGetLastError); pick the right one per call.
- **B6.5 — IOCP family** (`iocp_new`, `iocp_associate`,
  `iocp_wait_one`, `iocp_post`, `iocp_close`). NEW
  builtin family because the semantics don't match
  `epoll_*`. Document this as the Win32-native API; the
  cross-platform `epoll_*` family remains
  Linux/macOS-only.
- **B6.6 — `Sleep(ms)` for `sleep_ms`.** Replaces
  `nanosleep`. Easy 1:1 swap.
- **B6.7 — Conditional emit.** Both backends select
  POSIX / IOCP code paths based on
  `host_uses_win32_threading()`.
- **B6.8 — CI / parity on Windows.** Same gating as
  Arc 7 Win64 ABI work.

**Acceptance criteria:**
- Arc 8 examples adapted for Windows (e.g., a Win32-
  native `tcp_echo_iocp.vani` mirroring
  `tcp_echo_epoll.vani`'s structure) parity-green.
- Linux + macOS baselines maintained.
- README documents both portable (`epoll_*`) and
  Win32-native (`iocp_*`) API families.

**Effort estimate:** ~25-35h focused, 4-6 sessions.

**Dependencies:** None on Workstream A.

**Open questions / risks:**
- **Q9**: Should vāṇī's user-facing API be a single
  cross-platform abstraction (e.g., a `reactor_*`
  family that internally selects epoll/kqueue/IOCP) or
  two parallel APIs? Initial answer: ship the
  Win32-native `iocp_*` family first; a unifying
  `reactor_*` layer is a later polish step.
- **R7**: Risk: WSAStartup must be called before any
  socket call AND must be matched by WSACleanup. Easy
  to leak. Mitigation: emit a `__intent_winsock_init`
  runtime helper that runs once via a static initializer.
- **R8**: Risk: IOCP's completion-based semantics don't
  map cleanly onto the readiness-based v3.1 state-machine
  poll model. The v3.1 transform may need a
  Windows-specific code path that posts I/O operations
  + polls completions, rather than registering fds +
  polling readiness. Could push v3.1 + Windows port
  integration into a later phase.

---

## Phase scheduling at a glance

```
┌─────────────────────────────────────────────────────────┐
│  Workstream A (v3.1 sugar)                              │
│                                                          │
│  Phase 0 ───► Phase 1 ───► Phase 2 ───► Phase 3 ───►   │
│  Foundation   Linear core  Control     Affine          │
│  (6-8h)       (12-15h)     (15-20h)    (20-25h)       │
│                              │                          │
│                              ▼                          │
│                            Phase 4 (Advanced) 25-30h    │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│  Workstream B (platform port)                           │
│                                                          │
│  Phase 5 ─────────────────────► Phase 6                 │
│  macOS kqueue (10-15h)         Windows IOCP (25-35h)   │
│                                                          │
│  Either phase parallel-safe with all Workstream A phases │
└─────────────────────────────────────────────────────────┘

Total: ~113-148h, 16-25 sessions
```

---

## Recommended first session

**Phase 0 — Foundation.** Smallest, lowest-risk, unblocks
everything else.

Specific tasks for the next session (≤8h):
1. Add a `host_is_linux()` helper alongside
   `host_uses_win32_threading()` and use it to gate the
   `emit_intent_epoll_helpers_c` / `_llvm` +
   `emit_intent_tcp_helpers_c` / `_llvm` +
   `emit_intent_sleep_ms_helper_c` / `_llvm` emits. Emit
   a `#error` directive (C backend) /
   `; ERROR: Linux-only` comment (LLVM backend) + a
   diagnostic message when non-Linux.
2. Add `intent_event_loop_run<T>(task: T) -> T` builtin
   (Phase 0 scope A0.2). Signature stub + checker +
   both-backend codegen + lib test.
3. Add `sleep_ms_async(ms: i64) -> i64` + `sleep_ms_finish(fd) -> i64`
   builtins (Phase 0 scope A0.3). Use `timerfd_create` +
   `timerfd_settime` (Linux-only for now).
4. Add `examples/timer_async.vani` — composes
   `sleep_ms_async` + `epoll_wait_one` + `sleep_ms_finish`
   for a cooperative timer. Parity-green.
5. Add `examples/tcp_echo_event_loop.vani` — rewrites
   `tcp_echo_state_machine.vani`'s hand-rolled
   `drive_task` loop into a `intent_event_loop_run(et)`
   call. Identical stdout.
6. Update STATUS.md / TODO.md / ARCS.md / memory ledgers
   marking Phase 0 ✅ complete.

**Expected delta:** 1817 → ~1830 lib + 54 → 56 parity green.

After Phase 0 ships, the next session picks up Phase 1
(linear core) cold via the spec in this doc.
