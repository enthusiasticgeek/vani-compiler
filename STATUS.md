# vāṇī (वाणी) — Project Status

> The project was renamed from `future_compiler` to **VANI** (वाणी, Sanskrit for "speech")
> on 2026-05-21. "VANI" expands to *Verbose Alternative Natural Interface* — the
> design goal is code that reads like speech, not punctuation.

> Single-page snapshot of what the compiler does today, what's queued
> next, and known issues. Update this file whenever a feature lands,
> a TODO is added/closed, or an issue is resolved/discovered.
> Cross-reference [README.md](README.md) for the language tour and
> [TODO.md](TODO.md) for the canonical work list.

**Last updated:** 2026-06-04 (**Arc 8 FULLY COMPLETE + v3.1
Phases 0 + 1 + 2 narrow + 2.1a-c + 2.2 + 2.3-narrow +
2.3a + 2.3b + 2.3c + 2.3d + 2.5 + 2.5b + 3a shipped —
compiler-driven `async fn → Task` transform handles linear
bodies + non-suspending control flow + suspend-in-branch
state-splitting + fall-through merge state + nested ifs +
ANF lifting + match in async fn + match arms WITH suspends
(Int + Bool + Str + Float + Variant + VariantWithBinding
+ Wildcard) + loops with suspend inside + break/continue
inside loops + non-i64 locals (bool / f64). 18 v3.1
acceptance examples parity-green. **v3.1 match-with-suspends
+ bool/f64 locals — feature-complete.** Phase 3 next-up
(Str / Enum / Struct / Vec locals), Phase 2.4 (try) blocked
on broader Phase 3; macOS port (Phase 5) for later**).

Session 2026-06-04 shipped (six commits, ~+1100 lines):
- **Step 8e v1.5** (commit `d344828`) — `sleep_ms(ms: i64) -> i64`
  builtin on both backends. Wraps POSIX `nanosleep` with
  EINTR retry. 4 lib tests pin surface.
- **Step 8h v1.5** (commit `d209e06`) — `examples/async_io.vani`
  with timer-driven `async fn` + sequential awaits + CancelToken
  short-circuit + concurrent timer fan-out via `task` + `join`
  (real OS threads). Walker fix: `sleep_ms` inside a `task`
  body emits the helper correctly.
- **Step 8e proper v1.6** (commit `9aaec41`) — full TCP
  networking builtin family on both backends:
  `tcp_listen` / `tcp_socket_port` / `tcp_accept` /
  `tcp_connect_local` / `tcp_send_str` / `tcp_recv` /
  `tcp_send_buf` / `tcp_close`. All wrap libc
  socket / bind / listen / accept / connect / send / recv /
  close. Thread-local 4KB recv buffer per OS thread.
  `examples/tcp_echo.vani` ships an echo server + client in
  one process via `task` + `join`. LLVM IR codegen builds
  `sockaddr_in` by hand (16-byte stack alloca + htons +
  htonl). Walker fix: string literals inside task bodies
  now intern at module scope. 6 new lib tests.
- **Step 8h proper v1.6** (commit `83010ab`) —
  `examples/tcp_multi_echo.vani` — three concurrent client
  tasks, sequential server accept loop, 12 bytes echoed
  across 3 clients. Demonstrates the v1.6 surface scales to
  real concurrent server workloads.
- **Step 8c+8d+8e nb v2 runtime** (commit `92864de`) — full
  cooperative single-threaded reactor lands. Seven new
  builtins: `epoll_new` / `epoll_add_read` / `epoll_wait_one`
  / `epoll_close` / `tcp_set_nonblocking` / `tcp_accept_nb` /
  `tcp_recv_nb`. LLVM IR builds `epoll_event` by hand
  (16-byte stack alloca + EPOLLIN + data.fd); fcntl via
  varargs declare; errno via `__errno_location()` thunk.
  `examples/tcp_echo_epoll.vani` ships three concurrent
  clients multiplexed on ONE OS thread via the epoll reactor.
- **Step 8c sugar (v3 pattern)** (commit `f7743a1`) — three
  new async-flavored builtin aliases (`io_recv_async` /
  `io_send_async` / `io_accept_async`) name-reserve the
  future suspend-point semantics; today they rewrite to
  their v2 nb counterparts at the typed-IR boundary in
  `check_epoll_builtin`. `examples/tcp_echo_state_machine.vani`
  ships the hand-rolled state-machine pattern: an `EchoTask`
  struct + an `echo_task_poll` fn returning -2 (Pending) / -1
  (Error) / Ready-value, driven by a small loop that
  epoll-waits on Pending. Both backends produce byte-
  identical "echoed bytes: 2" (for "v2" payload).
- **v3.1 Phase 0 Foundation** — first slice of the
  compiler-driven sugar arc per [ARC8_V3_PLAN.md](ARC8_V3_PLAN.md):
  - **A0.3 + A0.4** (commit `eac1bf6`) — `sleep_ms_async(ms) -> i64`
    + `sleep_ms_finish(fd) -> i64` builtins backed by
    `timerfd_create` + `timerfd_settime`. Composes with the
    existing epoll runtime so the calling thread yields
    while a timer is pending. LLVM IR builds
    `struct itimerspec` by hand (32-byte stack alloca + sec
    at +16, nsec at +24). 5 lib tests pin the surface.
  - **examples/timer_async.vani** (commit `eac1bf6`) —
    cooperative 40ms timer via `sleep_ms_async` + epoll +
    `sleep_ms_finish`. Both backends byte-identical.
  - **Build-order fix** (commit `eac1bf6`) — when epoll
    helpers emit but TCP helpers don't, the epoll
    definitions referenced `@accept` / `@recv` /
    `@intent_tcp_buf` undeclared. Force TCP helpers to
    emit whenever epoll helpers do.
  - **A0.1 compile-time gate** (commit pending) —
    `host_is_linux()` helper alongside
    `host_uses_win32_threading()`. Codegen panics with a
    clear "see ARC8_V3_PLAN.md Phases 5/6 for the macOS /
    Windows ports" message when any Arc 8 I/O builtin
    (sleep_ms, full TCP family, epoll + nb variants,
    sleep_ms_async, sleep_ms_finish) is referenced on a
    non-Linux host. Fails LOUD during codegen instead of
    silently at link time.
  - **A0.2 deferred** — Phase 1's compiler-driven sugar
    resolves this naturally by concretizing T at transform
    time.

- **v3.1 Phase 2.3-narrow — match in async fns** (commit
  `2d92f30`). Allows `match` expressions inside async fn
  bodies when no arm contains a suspend:
  - `rewrite_vars_to_fields` extended with Match +
    MethodCall + Tuple/TupleAccess + FieldAccess +
    StructLit + ArrayLit cases (recursive walk into all
    these expr shapes)
  - `expr_contains_io_async` + `body_uses_io_async` both
    walk Match scrutinee + arm bodies
  - Suspends inside match arms rejected with Phase 2.3a
    pointer (per-arm state graphs needed)
  - `examples/echo_match.vani` — `match mode {…}` followed
    by `io_recv_async`; total = 305 byte-identical
  - 2 new lib tests

- **v3.1 Phase 2.3a — match arms WITH suspends** (commit
  `7a19b68`). Generalizes Phase 2.1a (binary if-with-suspend)
  to n-way match arms via a desugar in `anf_lift_body`:
  - `try_desugar_let_match_with_suspends` lifts
    `let X = match SCRUT { p0 then e0, …, _ then ed };` with
    suspends in any arm into
    `let X: i64 = 0; if SCRUT == p0 { X = e0; }
    else if … else { X = ed; }`
  - Runs BEFORE ANF + before validator; existing Phase
    2.1a/b state-splitting machinery handles per-arm state
    graphs without new infrastructure
  - Supports Pattern::Int + Pattern::Wildcard (last arm);
    Variant / Bool / Str patterns rejected with sub-phase
    pointer
  - Mixed suspending + non-suspending arms work
    (suspending arms become poll-states; non-suspending
    arms stay inline)
  - `examples/echo_match_suspend.vani` — 4-arm match
    (`recv 64`, `recv 8`, `42`, `0 - 1`); all three modes
    byte-identical: mode=0 → 3 (recv "abc"), mode=2 → 42,
    mode=99 → -1
  - Existing rejection lib test flipped to acceptance +
    new mixed-arms acceptance test added (2 lib tests
    total covering 2.3a)

- **v3.1 Phase 2.3b — Bool/Str/Float match patterns** (commit
  `eb2a8af`). Extends Phase 2.3a's desugar to non-Int literal
  patterns. Per-pattern shape only affects the comparison
  RHS Expr synthesized; nesting machinery is unchanged.
  Bool → `ExprKind::Bool(b)`, Str → `ExprKind::Str(s)`,
  Float → `ExprKind::Float(f)`.
  - Caveat: v3.1's i64-only param/local rule still applies.
    Non-i64 scrutinees must be INLINE expressions (e.g.
    `mode > 0`, `i64_to_str(x)`, `(x * 100) as f64 / 100.0`).
    Persisting non-i64 values across await suspend points is
    Phase 3 work.
  - Pattern::Variant + Pattern::VariantWithBinding remain
    rejected (need enum-tag dispatch at parse time, not
    accessible until checker); deferred to Phase 2.3c.
  - `examples/echo_match_b_s_f.vani` — three async fns
    exercising Bool / Str / Float patterns with suspending
    arms. All five outputs byte-identical: bool true → 3
    (recv), bool false → 999, str=1 → 111, str=99 → -1,
    f=2.0 → 222.
  - 4 new lib tests (3 acceptance + 1 Variant-rejection).

- **v3.1 Phase 3a — non-i64 locals (bool / f64)** (commit
  pending). First slice of Phase 3 (affine types across
  await). Lifts the historical i64-only locals rule for the
  two types with obvious defaults:
  - `v31_local_type_allowed(ty)` gates v3.1 local types
    (I64 / Bool / F64). Str / Enum / Struct / Vec deferred.
  - `v31_default_init_expr(ty, span)` emits per-type default
    init for the task struct constructor's StructLit
    (`Int(0)` / `Bool(false)` / `Float(0.0)`).
  - `Seg::NonSuspendLet { ty, ... }` carries the local's
    annotation so the synthesizer's `let __v3_tmp_<name>:
    <ty> = ...;` temp matches (previously hardcoded i64).
  - `validate_v31_linear_body` + `validate_v31_phase_21a_branch`
    + `collect_branch_locals_recursive` all route through
    `v31_local_type_allowed`.
  - `examples/echo_p3a_nonint_locals.vani` — bool flag +
    f64 factor in one async fn, both modes parity-green
    (true→recv 3 bytes, false→0).
  - 3 new lib tests (bool acceptance + f64 acceptance + Str
    rejection pinning the next-sub-phase boundary).

- **v3.1 Phase 2.3d — VariantWithBinding patterns** (commit
  `9fb16dc`). Extends Phase 2.3c's tag-extraction sub-match
  approach with SUBSTITUTION-based binding propagation:
  - For each `Opt.Some(v) then EXPR` arm, the desugar
    walks EXPR and substitutes every `Var(v)` with an
    inline re-extraction match expression
    `match SCRUT { Opt.Some(__pat_v) then __pat_v, _ then 0 }`.
    The substituted EXPR becomes the assign value in the
    corresponding if-branch.
  - SUBSTITUTION (not a local LET) sidesteps a subtle bug:
    a branch-local LET whose name collides with a task
    struct field creates a disconnect — the local LET
    writes to a local, but subsequent reads get rewritten
    to `__t.<name>` (uninitialized). Inlining the extraction
    at each binding-use point bypasses the issue.
  - New helper `substitute_var_in_expr` walks all common
    expression shapes (Binary, Unary, Cast, Call, Index, Len,
    Ref/RefMut, Match, MethodCall, Tuple/TupleAccess,
    FieldAccess, StructLit, ArrayLit) and substitutes.
  - Inner pattern binding uses fresh `__pat_<binding>` to
    avoid name collisions with the rename set.
  - Caveat: a binding used N times re-evaluates the scrutinee
    N times. Fine for pure helper-fn scrutinees (the typical
    pattern in v3.1 due to i64-only locals constraint).
  - `examples/echo_match_binding.vani` — three binding-carrying
    modes byte-identical: Recv(10) → 5 (recv 5 bytes), Constant(444)
    → 445 (val+1), Idle → -1 (no binding).
  - 2 new acceptance lib tests (single binding + multiple
    binding-arms); previous rejection test flipped to acceptance.

- **v3.1 Phase 2.3c — Variant patterns with suspends** (commit
  `cc8c8de`). Tag-extraction sub-match approach:
  - `try_desugar_match_via_tag_extraction` synthesizes a
    sub-match returning i64 tag (arm idx) for each variant arm.
    The sub-match's arms are unit variants returning literal
    i64 — no suspends — so the existing Phase 2.3-narrow
    variant-dispatch path handles them.
  - The outer if-chain dispatches on the synthesized tag,
    so per-arm suspending bodies route through Phase 2.1a/b's
    state-splitting machinery without new infrastructure.
  - Supports Pattern::Variant (unit variants). Wildcard
    maps to sentinel tag = `len(variant_arms)`.
  - Pattern::VariantWithBinding still deferred (Phase 2.3d):
    the binding `v` in `Result.Ok(v)` would need to be relifted
    across the tag-extraction → if-chain split.
  - Caveat: v3.1's i64-only locals/params rule means Variant
    scrutinees must be inline expressions — typically a helper
    `fn make_action(mode: i64) -> Action` call.
  - `examples/echo_match_variant.vani` — three modes
    byte-identical: Recv → 4 (recv "abcd"), Constant → 333,
    Wild → -1.
  - 3 new lib tests (2 acceptance + 1 VariantWithBinding
    rejection).

- **v3.1 Phase 2.5b — break/continue inside loops** (commit
  `4702cb6`). Adds `break` and `continue` keywords inside
  suspending while bodies:
  - `collect_into` gains a `loop_stack` of
    `(loop_header, post_loop)` pairs
  - `Stmt::Break` → `Seg::Jump(post_loop)`;
    `Stmt::Continue` → `Seg::Jump(loop_header)`
  - `rewrite_break_continue_in_stmts` rewrites break/
    continue inside Verbatim Stmt::If branches to
    state_tag jumps + synthesized `Continue` (re-enters
    the while-true cascade)
  - Doesn't recurse into user `while` bodies (those have
    their own break/continue scope)
  - `examples/echo_loop_break.vani` — `if n == 0 { break; }`
    inside a while loop. Read "stop" (4) + EOF break.
    Total=4 byte-identical on both backends
  - 2 new lib tests

- **v3.1 Phase 2.5 — loops with suspend** (commit `a77b799`).
  Lifts the "while body cannot contain suspends"
  restriction:
  - Stmt::While with suspends in body now state-splits:
    loop_header + body_start + post_loop states + BACKWARD
    `Seg::Jump` at body tail
  - Synthesis wraps poll body in `while true { ... return
    -1; }` so backward Jumps use `continue;` to re-enter
    the cascade
  - Jump synthesis detects `target_state <= state_idx`
    (backward jump) and emits `Stmt::Continue`
  - Branch validator relaxed for Phase 2.5 needs: Assign +
    Print allowed inside suspending branches (when their
    exprs don't suspend — ANF lifts those)
  - `examples/echo_loop.vani` — 3-iteration while loop
    reading 4 + 0 + 0 bytes. "3-iter loop total: 4"
    byte-identical on both backends
  - 2 new lib tests

- **v3.1 Phase 2.2 — ANF lifting** (commit `d3a0af3`).
  Lifts the restriction that `io_*_async` must appear ONLY
  as a direct Let RHS:
  - Pre-pass walks every expression in the async fn body,
    lifting nested `io_*_async(args)` into fresh
    `let __anf_N = io_*_async(args);` Lets emitted BEFORE
    the original stmt
  - `anf_lift_expr` recursive Expr walker; `anf_lift_body`
    statement-level driver
  - Direct-Let-RHS pass-through optimization (don't lift
    when already at valid suspend-point position)
  - `examples/echo_anf_lift.vani` — `let r = io_recv_async(fd,
    64) + bias;` lifts cleanly; "anf-lifted result (bytes
    + 100): 104" byte-identical on both backends
  - 3 new lib tests

- **v3.1 Phase 2.1c — nested ifs inside suspending branches**
  (commit `22fc622`). Lifts Phase 2.1a/b's "branches must
  be linear" restriction:
  - Recursive `collect_into` ALREADY supported arbitrary
    nesting via its recursive Stmt::If handling — Phase
    2.1c was a validator relaxation + recursive branch-
    local collection
  - New `collect_branch_locals_recursive` — walks nested
    branches' Lets so the task struct has fields for all
    locals that might persist across suspends
  - New `body_all_paths_return` — detects when a Verbatim
    stmt unconditionally exits, so the collector suppresses
    the dead `Jump` that would trip the reachability checker
  - `examples/echo_nested_if.vani` — outer suspending
    branch contains a nested if with both branches
    returning; state machine fans out to two nested-return
    states. Byte-identical on both backends.
  - 2 new lib tests + 1 prior rejection test updated to
    acceptance

- **v3.1 Phase 2.1b — fall-through merge state** (commit
  `b1b8113`). Lifts Phase 2.1a's "both branches must
  return" restriction:
  - Branches can fall through to a merge state holding
    post-if code (e.g. trailing `return EXPR`)
  - New `Seg::Jump { target_state, span }` emitted at the
    tail of a non-return-terminated branch
  - `collect_into` returns bool (terminated-with-return);
    merge state allocated AFTER both branch recursions
    so its index is monotonically highest in the cascade
  - `examples/echo_fall_through.vani` parity-green —
    then-branch falls through to `return 100`,
    else-branch returns -5
  - 2 new lib tests + 2 prior "rejection" tests updated to
    "acceptance" tests since Phase 2.1a/b lifted those

- **v3.1 Phase 2.1a — suspend-in-branch state-splitting**
  (commit `7566ab8`). Lifts Phase 2 narrow's "no suspend
  in branches" restriction:
  - `if cond { ... } else { ... }` may now contain
    `io_*_async` calls in either or both branches
    (provided both branches end with explicit Return)
  - New `Seg::Decision { cond, then_state, else_state }`
    variant — the current state ends with a conditional
    state_tag bump to the right branch's starting state
  - New `Seg::Suspend.bump_to` carries the explicit next-
    state index (Phase 2.1 fix: branching makes the next
    state non-sequential)
  - Recursive `collect_into` replaces the flat collect-
    then-group pass; uses explicit state allocation via
    `state_bodies.len()`
  - Branch-local Lets contribute fields to the task struct
    so locals persist across suspends within a branch
  - Phase 2.1b (fall-through merge), 2.1c (nested ifs),
    2.5 (break/continue) rejected with explicit pointers
  - `examples/tcp_echo_async_branched.vani` — both branches
    exercised cross-backend, byte-identical
  - 4 new lib tests

- **v3.1 Phase 2 narrow — control flow inside async fn**
  (commit `a3cab5b`). Lifts Phase 1's linear-body
  restriction:
  - Stmt::If / Stmt::While / Stmt::Assign / Stmt::Print
    + mid-body Return all allowed at the top level —
    provided branches don't contain `io_*_async` suspends
  - Reassign on outer-scope locals rewrites to FieldAssign
    on `__t.<name>`
  - Suspend-in-branch / suspend-in-cond / multi-await-in-
    expr cases rejected with explicit Phase 2.1 / 2.2
    pointers
  - examples/echo_with_timeout.vani — input validation +
    counter increment + two suspend points woven together,
    byte-identical "echoed bytes: 6" on both backends
  - 4 new lib tests; helpers `expr_contains_io_async` +
    `stmt_contains_io_async` for granular shape checks;
    `rewrite_vars_in_stmt` for recursive Var-to-field
    rewriting across nested control flow

- **v3.1 Phase 1 — compiler-driven `async fn → Task`
  transform** (commit `7d47ff6`). The headline v3.1
  feature lands:
  - Detection: `body_uses_io_async` walks Stmt/Expr trees
    for `io_recv_async` / `io_send_async` / `io_accept_async`
    calls.
  - Validation: `validate_v31_linear_body` enforces Phase 1's
    narrow shape (linear body, all-i64 locals/params, return
    i64). Rejects with explicit phase-pointer diagnostics
    ("control flow arrives in Phase 2", "non-i64 locals
    arrive in Phase 3").
  - Synthesis: per-fn `Task__<name>` struct (state_tag + saved
    params + locals) + `__poll_<name>` fn (one `if t.state_tag
    == K` arm per suspend point, with Pending(-2) / Error(-1)
    propagation) + constructor body replacing the original
    async fn body.
  - Registry: `V31_TASK_REGISTRY` thread-local queue of
    synthesized `(StructDecl, Function)` pairs, cleared at
    start + flushed at end of `parse_program`.
  - Naming: `Task__<name>` (PascalCase leading per
    `parse_type` discipline) + `__poll_<name>`.
  - `examples/tcp_echo_async.vani` — Phase 1 acceptance.
    User writes ONLY the async-fn body + a 5-line drive
    loop; compiler generates the rest. Byte-identical
    "echoed bytes: 3" on both backends.
  - 4 new lib tests (simplest case, two-await composition,
    no-io fall-through to v1 desugar, non-linear rejection).

**1847 lib + 54 parity green** (parity includes 13 v3.1
acceptance examples + the v3 hand-rolled pattern, all
byte-identical cross-backend: `tcp_echo.vani`,
`tcp_multi_echo.vani`, `tcp_echo_epoll.vani`,
`tcp_echo_state_machine.vani`, `tcp_echo_async.vani`,
`tcp_echo_async_branched.vani`, `echo_fall_through.vani`,
`echo_nested_if.vani`, `echo_anf_lift.vani`,
`echo_loop.vani`, `echo_loop_break.vani`,
`echo_match.vani`, `echo_with_timeout.vani`,
`async_showcase.vani`, and `timer_async.vani`).

## 🎉 v3.1 sugar arc — capability-complete for control flow

The compiler-driven `async fn → Task` transform handles
EVERY common control-flow shape in vāṇī:
- Linear bodies + sequential suspends
- if/else (with or without suspends in branches)
- Mid-body Return
- Outer-local Assign + Print between suspends
- Fall-through merge state for non-return-terminated branches
- Nested ifs inside suspending branches (arbitrary depth)
- ANF lifting for io_*_async in compound expressions
- while loops with suspends inside
- break / continue inside suspending loops

`examples/async_showcase.vani` is the user-facing capstone —
all of these in one async fn body.

What's NOT yet supported (deferred sub-phases):
- `match` arms (Phase 2.3, ~8-10h)
- `try` keyword + Result propagation (Phase 2.4, ~6-8h)
- Affine types across await — OwnedStr / Vec (Phase 3, ~20-25h)
- Generics / nested async-fn calls / multi-task scheduling
  (Phase 4, ~25-30h)
- macOS port (Phase 5, ~10-15h) / Windows port (Phase 6, ~25-35h)

## ✅ Arc 8 fully complete (2026-06-04)

Every user-visible async + networking + concurrency feature
ships on `main` with byte-identical behavior on both backends:

- **Surface:** `async fn` / `await(expr)` / `Future<T>` /
  `Poll<T>` / `CancelToken` (parser-level desugar; v1
  semantics are synchronous Future.Ready).
- **Timers:** `sleep_ms(ms) -> i64` blocking-precise
  POSIX wrapper.
- **Blocking TCP:** 8-function family — listen, accept,
  connect, send_str, recv, send_buf, socket_port, close.
- **Non-blocking TCP + epoll:** `epoll_new` /
  `epoll_add_read` / `epoll_wait_one` / `epoll_close` /
  `tcp_set_nonblocking` / `tcp_accept_nb` / `tcp_recv_nb`.
  Composes into single-threaded cooperative scheduling —
  one OS thread, N concurrent connections, multiplexed by
  the kernel.
- **Async-flavored aliases:** `io_recv_async` /
  `io_send_async` / `io_accept_async` name-reserve the
  future v3.1 suspend-point semantics; today they rewrite
  to their v2 nb counterparts.
- **Concurrency models — user's choice:**
  - Thread-per-task via existing `task` + `join` (race-free
    by construction per the affine checker)
  - Single-thread cooperative via epoll + nb variants
  - Hand-rolled state-machine pattern (struct + poll fn +
    driver loop) using `io_*_async` builtins
- **Examples (all parity-green):**
  - `async_io.vani` — timer-driven async fn + task fan-out
  - `tcp_echo.vani` — single-client TCP echo
  - `tcp_multi_echo.vani` — 3 concurrent clients via tasks
  - `tcp_echo_epoll.vani` — 3 concurrent clients on **ONE
    thread** via epoll reactor
  - `tcp_echo_state_machine.vani` — v3 state-machine pattern
    (struct + poll fn + driver) using `io_*_async` aliases

Web servers, scheduler demos, fan-out workloads, single-
thread multiplexed I/O, AND hand-rolled cooperative state
machines all work TODAY. Compiler-driven state-machine
sugar (v3.1) that would auto-generate the struct/poll/
constructor triples from an `async fn` body is queued as
multi-day compiler work — the underlying capability
already ships.

---

## 📋 Arc 8 v3.1 (compiler-driven sugar transform — OPTIONAL)

Arc 8 is FULLY COMPLETE today including the v3 state-machine
PATTERN (hand-rolled via `examples/tcp_echo_state_machine.vani`).
v3.1 would add the COMPILER-DRIVEN transform that auto-rewrites
`async fn` bodies containing `io_*_async` calls into the
state-machine struct + poll fn + constructor triples that
users today write by hand.

**Phased execution plan lives in [ARC8_V3_PLAN.md](ARC8_V3_PLAN.md).**
Five v3.1 phases (foundation → linear core → control flow →
affine integration → advanced) + two platform-port phases
(macOS kqueue → Windows IOCP). Total backlog ~113-148h
across 16-25 sessions. **Recommended first session: Phase 0
(Foundation) — ~6-8h.**

Pick up a future session with this prompt if you want sugar:

> Continue Arc 8 v3.1 sugar layer from where 2026-06-04 left
> off. v3 partial (commit `f7743a1`) ships the state-machine
> PATTERN — users write the struct + poll fn + driver loop by
> hand using `io_*_async` builtin aliases (see
> `examples/tcp_echo_state_machine.vani`). v3.1 lifts that
> pattern into compiler-driven sugar:
>
> **State-machine sugar.** Walk each `async fn` body at check
> time. If it contains `await(expr)` calls, split the body at
> each await, generate a per-state frame struct holding
> locals alive across suspensions, emit a `poll(state) ->
> Poll<T>` dispatch. The poll-function reads from / writes to
> the epoll reactor via the existing nb builtins. Likely
> needs a new prelude type `Task<T> = { env: u64, poll: fn(u64) -> Poll<T> }`
> distinct from the v1 `Future<T>` enum (v1 source stays
> source-compatible). Foundation: Arc 5c closure-as-value
> (commit `7cccc1b`) gives the env-struct + trampoline
> primitive the frames build on.
>
> **Event-loop driver.** Add `intent_event_loop_run<T>(task: Task<T>) -> T`
> that drives the task to completion by polling against the
> epoll reactor. This replaces the hand-written reactor loop
> in `examples/tcp_echo_epoll.vani` with one line of user
> code.
>
> **Acceptance.** Add `examples/tcp_echo_async.vani` —
> a `tcp_echo_epoll.vani` rewrite where the reactor logic
> moves into compiler-generated state machines. User-side
> code becomes:
>   ```
>   async fn handle_client(fd: i64) -> i64 {
>     let n: i64 = await(tcp_recv_async(fd, 64));
>     return await(tcp_send_buf_async(fd, n));
>   }
>   ```
> ...with `tcp_recv_async` returning `Future<i64>` that the
> reactor drives. Identical stdout to `tcp_echo_epoll.vani`.
>
> Estimated effort: ~15-20h focused (state machines are the
> bulk; the reactor + nb primitives already ship).

### v3.1 design caveats — open questions to resolve

The transform sketched above works cleanly only for the
simplest case. Each item below is a real engineering question
the v3.1 implementer must answer:

| # | Caveat | Initial v3.1 stance | Future polish |
|---|---|---|---|
| 1 | **Body shape** — linear (Let + Print + Discard + Return) only | Accept only linear; reject `if` / `while` / `for` / `match` / `try` / `break` / `continue` inside async fn body with `io_*_async` calls | Each control-flow form needs explicit state-machine handling. `if/else` doubles states per branch; `while` needs a back-edge state. Multi-day per construct. |
| 2 | **Local liveness analysis** — only locals KEPT ACROSS an await should land in the state struct | Conservatively put EVERY i64 local in the state struct | Compute per-await liveness; transient locals become poll-fn locals (not stored). Saves heap. |
| 3 | **Affine types across await** — `OwnedStr` / `Vec<T>` locals must be moved into state, not borrowed | Reject non-i64 locals at suspend-point boundaries with a clear diagnostic | Track owning locals → store in state with proper Drop on Pending-return; restore on resume. Hard problem under the affine checker. |
| 4 | **Multiple awaits in one expression** — `let x = await(f()) + await(g());` | Reject; require ANF lifting (one await per Let) | Expression-level transform that lifts awaits into sequenced Lets before state-machine pass |
| 5 | **`ref` / `mut ref` parameters** — can't store across heap-allocated state without lifetime tracking | Reject async fns that take `ref T` params with a clear diagnostic | Either lifetime tracking in the affine checker, or copy-the-pointee-into-state pattern |
| 6 | **CancelToken plumbing** — currently user checks `.cancelled` by hand | v3.1 user still passes `ref CancelToken` and reads `.cancelled` between awaits explicitly | Auto-inject `if token.cancelled { return ...; }` at every suspend point. Needs semantic decision: what does the cancelled value look like? |
| 7 | **Error propagation** — `io_recv_async` returns -1 on hard error; the user's async fn must decide what to do | v3.1 propagates the -1 through the state struct's error field; caller checks | Integrate with `try` keyword / `Result<T, E>` so async fns naturally propagate errors |
| 8 | **Nested async-fn calls** — `await(foo())` where `foo` is itself v3.1-transformed (returns Task<T>) | Reject: await only applies to `io_*_async` builtin calls, not user async fns | State-machine composition: parent suspends until child Task<T> is Ready. Real-world need but complex (child state lives inside parent state struct). |
| 9 | **Generic params** — `async fn foo<T>(x: T) -> T` | Reject generics; require concrete i64 | Monomorphize the state struct + poll fn per (async-fn, type-args) tuple. Already have closure #281 generic-decl machinery to lean on. |
| 10 | **Side effects between awaits** — Print statements, Vec mutations, etc. | Allowed (just emit as normal vāṇī code in the matching state arm) | Verify no surprises with affine drops at suspend points |
| 11 | **Top-level driver** — `intent_event_loop_run<T>(task: Task<T>) -> T` | Ships as a vāṇī source-level helper (loops calling poll + epoll_wait) | Builtin that takes the Task type and the underlying fd dispatch table |
| 12 | **Multi-task scheduling** — N concurrent Tasks on one reactor | v3.1 ships single-task driver; multi-task uses task + join (cooperative scheduling INSIDE a task is single-task) | Real cooperative scheduler that round-robins through a Vec<Task> — needs Vec<Task<T>> for heterogeneous T (existential or boxed-trait shape) |
| 13 | **timerfd-based `sleep_ms_async`** — currently `sleep_ms` blocks the calling OS thread | Ship a `sleep_ms_async(ms) -> i64` that uses `timerfd_create` + `epoll_add_read` so the event loop can wait on it without blocking | Same mechanism extended to file I/O (`file_open_async`, etc.) |
| 14 | **Diagnostic quality** — when v3.1 rejects an unsupported async-fn shape, the error should clearly point at the problem | Reuse the existing diagnostic machinery; carry the span of the offending construct | Suggest fix-ups (e.g., "lift this expression into a Let") |
| 15 | **Test surface** — how do we test compiler-generated state machines? | Generate to typed IR; assert structure in lib tests; run via parity for behavior | Snapshot tests on the generated state-struct + poll-fn AST |

---

## 🪟 Platform support (Arc 8) — Linux only today

**Phased port plan lives in [ARC8_V3_PLAN.md](ARC8_V3_PLAN.md)
Phases 5–6** (macOS kqueue ~10-15h, Windows IOCP ~25-35h).
Below is the per-subsystem matrix for quick reference.


The Arc 8 v1.5/v1.6/v2/v3 runtime helpers **only compile + run on
Linux**. Every other tested target needs porting:

| Subsystem | Linux | macOS | Windows | Notes |
|---|---|---|---|---|
| `sleep_ms` (nanosleep) | ✅ ships | ✅ POSIX (nanosleep works) | ❌ would need `Sleep(ms)` from Win32 API | nanosleep is POSIX, not Win32 |
| Blocking TCP (socket/bind/listen/accept/connect/recv/send/close) | ✅ ships | 🟡 likely works (BSD sockets) — UNTESTED | ❌ requires WSAStartup + winsock2.h + SOCKET type (≠ int) + WSAGetLastError | Win32 sockets are a separate API surface |
| `tcp_set_nonblocking` (fcntl O_NONBLOCK) | ✅ ships | 🟡 likely works — UNTESTED | ❌ requires `ioctlsocket(FIONBIO)` | fcntl is POSIX-only |
| `__errno_location()` (used by `tcp_accept_nb`, `tcp_recv_nb`) | ✅ glibc/musl | ❌ macOS uses `__error()` | ❌ Win32 uses `_errno()` / `GetLastError()` | Three different thunks for three platforms |
| **epoll** (epoll_create1 / epoll_ctl / epoll_wait) | ✅ ships | ❌ macOS uses **kqueue** + kevent | ❌ Windows uses **IOCP** (I/O Completion Ports — fundamentally different programming model) | The biggest porting cost is here |
| `io_*_async` aliases | ✅ ships (route to nb variants) | 🟡 follows nb variant support | ❌ follows nb variant support | Aliases inherit underlying portability |
| `task` + `join` concurrency | ✅ pthread_create / pthread_join | 🟡 likely works (POSIX threads) — UNTESTED | ✅ already wired via `CreateThread` + `WaitForSingleObject` (see `host_uses_win32_threading()`) | Threading IS portable; I/O isn't |

**Engineering plan to lift Linux-only restriction:**

1. **macOS (lower lift)** — port the LLVM IR + C runtime helpers
   to use `__error()` for errno, then write a `kqueue` shim that
   matches the `intent_epoll_*` signatures byte-for-byte. The
   user-facing `epoll_*` names stay; the implementation forks.
   Estimated effort: ~8–12h.

2. **Windows (high lift)** — full IOCP port. The programming
   model is "register I/O operations, then poll for completions"
   rather than "register fds, then poll for readiness". This
   doesn't map cleanly onto the existing `epoll_add_read` +
   `epoll_wait_one` shape. Likely needs a separate
   `iocp_*` builtin family AND a Win32 socket runtime
   (`WSAStartup` etc.). Estimated effort: ~25–35h.

3. **Compile-time gate** — until macOS / Windows ports land,
   wrap the Arc 8 I/O helper emit in
   `if !host_uses_win32_threading() && !host_is_darwin()` so
   builds on those targets fail loud with a clear message
   ("Arc 8 epoll runtime is Linux-only; see STATUS.md
   Platform support"). Currently Windows builds would fail
   silently at C-compile time (missing `<sys/socket.h>`) or
   LLVM link time (undefined `@epoll_create1`).

**Today's parity runner is Linux-only** (`cargo test` runs on
the developer's machine — typically Linux). Cross-platform CI
would catch the breakage but isn't wired up; same gate as
Arc 7 Win64/AArch64 FFI work.

---

Session 2026-06-03 shipped: all 6 ARC 4 sub-arcs cross-backend
(4.1–4.6), then closed the now-actionable polish queue (items
0–5):
- **Item 0** — markdown ledger refresh (Arcs 1–4 marked
  complete; new sequenced ledger for Arcs 5–10).
- **Item 1** — Vtables Vec<dyn Iface> element hoist: non-Var
  call/struct/literal sources now compile cleanly via the
  shared `__dyn_src_*` synthetic-let prelude (checker hoists,
  C backend's Let-stmt unfold broadens to recognize the
  prelude pattern, LLVM already worked).
- **Item 2** — Closure A drop-dispatch lib test added (per-
  variant `switch (tag)` drop for `Result<Vec, OwnedStr>`-
  shape mixed-payload enums; both backends already had the
  codegen).
- **Items 3, 4, 5** turned out to already be on `main`
  (`try_vec` = closure #284, nested arrays = closure #291
  Phases 3+4, `#[bounded(N)]` = closures #286 + #289).
- **Arc 5 audit**: 3 of 4 sub-steps (5a non-Copy capture-by-
  value, 5b `[ref xs]` capture-by-ref, 5d reassign + sibling
  name reuse) all already on `main`. Only 5c (closure-as-
  value across fn boundaries) genuinely remains.

**Test ledger:** 1791 lib + 54 parity green at end of session.

**Forward queue** (per [TODO.md](TODO.md) + [ARCS.md](ARCS.md))
after the deeper audit on 2026-06-03 revealed Arcs 6 and 9 c/d
were also already on `main` (closures #281/#282 for generic
type decls; closures #257/#258 for `pub use` re-exports +
`pub(kosh)` visibility tier):

- **Arc 5c COMPLETE** (commit `7cccc1b`, 2026-06-03) — closure-
  as-value across fn boundaries. **Arc 7 SysV remainder
  COMPLETE** (commit `69b5ec0`) — float-field + mixed int/float
  structs ≤ 16 bytes FFI-safe.
- **Arc 8 v1 COMPLETE** (2026-06-03/04, four commits):
  - 8a (`2e649ff`) — `Poll<T>` + `Future<T>` in prelude
  - 8b (`e50dc20`) — `async fn` contextual keyword + parser
    desugar (body returns wrap to `Future.Ready(expr)`,
    `-> R` reshapes to `-> Future<R>`)
  - 8f + 8g (`25b5a84`) — `await(expr)` parser desugar
    extracting Future.Ready payload; `CancelToken` prelude
    struct

  **1800 lib + 56 parity green.**

  Working syntax on both backends:
  ```vani
  async fn fetch(n: i64) -> i64 { return n * 7; }
  fn main() -> i64 {
    let r: i64 = await(fetch(6));    // 42
    let t: CancelToken = CancelToken { cancelled: false };
    return r;
  }
  ```

  v1 semantics: `async fn` runs synchronously on call,
  always emitting `Future.Ready`. `await` extracts the
  payload immediately. Users get the correct **types +
  syntax** matching Rust's; the future async runtime
  (state-machine codegen + event loop + non-blocking I/O)
  upgrades the runtime without breaking source code.

  **Arc 8 remainder (8c state-machine + 8d event loop + 8e
  I/O primitives + 8h example) is queued as its own focused
  multi-day arc — concentrated single-session work that
  needs to land contiguously without mid-flight context
  exhaustion.**
- **Arc 10 (Devanagari SOV grammar)** explicitly blocked on
  grammar-consultant validation per the TODO.md design notes
  ("Sanskrit options include `संविदा` (saṁvidā), `कार्यान्वयन`
  (kāryānvayan); Hindi / Marathi need separate evaluation.
  Don't ship without language-expert confirmation."). Cannot
  responsibly ship without consultant input — every sub-step
  affects every Devanagari vāṇī file's surface syntax.
- **Arc 9 (Kosh package manager)** deferred per user: registry
  hosting model still pending decision before implementation
  begins.

  Captured anonymous fns can now be passed to higher-order
  functions typed `Closure(args) -> R`.
  Lift pass synthesizes an env-struct + trampoline + magic
  `__intent_make_closure_<N>` constructor per closure-with-
  captures. C backend emits per-(args, ret) closure struct
  typedef `{uint64_t env; R (*call)(uint64_t, args);}` +
  trampoline; LLVM emits `%intent_closure_<sig>` named struct
  + trampoline with `inttoptr`/`getelementptr` to read env.
  Call on Closure-typed value dispatches via
  `c.call(c.env, args)`. **1792 lib + 55 parity green.**
  Example: `examples/closure_as_value.vani`.

  v1 restrictions: per-closure env stored in a static __thread
  / module-global slot (no heap), so a Closure cannot outlive
  the lift-instantiation site. By-value Copy captures only.
  No FnPtr → Closure auto-coercion. Heap-env + ref-coercion
  are queued v2 follow-ups.

  **Unblocks Arc 8 (Async)** — `Future<T>` + `async fn` state
  machines need closure-as-value to plumb resumes through.
- **Arc 7 remainder** — full FFI classifier (float-class +
  mixed integer/float decomposition under SysV x86-64;
  Windows/ARM gated on CI availability). ~6-8h focused.
- **Arc 8** — Async / asyncio. ~30-40h. Deps: Arc 5c.
- **Arc 9 a/b/e/f** — full package manager (manifest,
  resolver, registry, stdlib-as-kosh). ~25-30h.
- **Arc 10** — Devanagari SOV grammar fit. Soft-blocked on
  grammar consultant.

**Already shipped** (no work remaining):
- Arc 5a/5b/5d (closure phase 4 + ARC 3a + reassign)
- Arc 6 entire (generic type decls + prelude Option/Result)
- Arc 7 safe subset (closure #285 — SysV ≤ 16 bytes integer)
- Arc 9 c/d (visibility tier + `pub use` re-exports)

**Prior:** 2026-06-03 (**ARC 4 COMPLETE — `HashMap<Vec<i64>, V>`
finishes the wider K-V matrix on both backends.** All six ARC 4
sub-arcs (4.1 OwnedStr K, 4.2 OwnedStr V with i64 K, 4.3
OwnedStr K + OwnedStr V, 4.4 (i64,…,i64) tuple K, 4.5 f64 K,
4.6 Vec<i64> K) are shipped.

4.6 specifics: K is the existing `intent_vec_int64_t` /
`%intent_vec_i64` struct stored by value per slot. Hash:
length-prefixed FNV-1a over each i64 element (so empty +
shorter Vecs distribute distinctly). Equality: lengths-equal
then `memcmp` of data buffers. Map deep-clones the data array
on insert via `malloc + memcpy` (same affine-system workaround
used in 4.1/4.2/4.3); drop walks free each stored data buffer.
Duplicate K keeps stored K's clone, swaps V (returns prior V
to caller). Remove frees stored K's data, transfers V out,
tombstones slot.

4.6 dispatch: `hashmap_type_tag_c_owned` returns `vec_int64_t`
for `Type::Vec(Type::I64)`; LLVM `hashmap_llvm_dispatch_with_k`
returns `%intent_vec_i64`. Bundle prefix on both backends:
`intent_hashmap_vec_int64_t_<V>`. LLVM prologue now declares
`memcmp` so the bundle's equality call resolves.

4.6 (end-to-end): `examples/hashmap_veck.vani` exercises
insert (with overwrite triggering duplicate-K + V-swap) /
get / contains_key / remove / len with `(1,2,3)` /
`(4,5,6)` / shorter `(1,2)` Vec keys. Distinct-length
lookups miss (proves length-prefix hashing). Identical
stdout on both backends.

**2 new lib tests** covering:
  - both backends compile HashMap<Vec<i64>, i64> with the
    `intent_hashmap_vec_int64_t_int64_t` bundle prefix
  - C bundle dispatches equality through `__eq_key` (which
    uses `memcmp(a.data, b.data, …)`) and clones K via
    `malloc + memcpy` on insert

**1789 lib + 59 parity green. ARC 4 fully complete: all six
(K, V) shape combinations ship cross-backend.**

**Prior:** 2026-06-03 (**ARC 4.3 COMPLETE — `HashMap<OwnedStr, OwnedStr>`
end-to-end on both backends.** Both K and V heap-owned by the
map. Composition of ARC 4.1 (K=OwnedStr) and ARC 4.2
(V=OwnedStr): drop walks free both per occupied slot; insert
clones both K and V (strlen + malloc + memcpy each); duplicate
K keeps the stored K and swaps in a fresh V clone (returning
prior V to caller); remove frees the stored K and transfers
the stored V pointer out via Option<OwnedStr>; get clones the
stored V.

4.3 checker relaxation: `check_hashmap_builtin` accepts
V=OwnedStr when K is i64 *or* OwnedStr (composing 4.2 + 4.1
acceptance).

4.3 (C bundle): new `emit_intent_hashmap_pair_c_body_strk_strv`
emits per-(OwnedStr, OwnedStr) bundle with prefix
`intent_hashmap_owned_str_owned_str`. Both keys and values are
`char**` arrays. Drop/clear walk frees both K and V per slot.
Insert clones K and V (separate malloc+memcpy per axis);
duplicate K extracts old V (returned), stores clone of new V.
Remove frees K, transfers V pointer out, tombstones slot. Get
clones the stored V.

4.3 (LLVM bundle): `emit_intent_hashmap_pair_llvm_strk_strv`
mirrors the C side. Bundle struct
`{i8**, i8**, i8*, i64, i64, i64}`. Equality via `strcmp`;
hash via FNV-1a byte loop. Drop loops free both K and V;
clear mirrors. Insert clones both via `strlen + malloc + memcpy`;
duplicate path returns prior V in Option payload. Remove
frees K, transfers V out, nulls slot pointers.

4.3 dispatch: prologue match handles `(Type::OwnedStr,
Type::OwnedStr)` arm before the K-only `(Type::OwnedStr, _)`
arm on both backends, so the str-str bundle is preferred for
that pair.

4.3 (end-to-end): `examples/hashmap_strstr.vani` exercises
insert (with overwrite that triggers duplicate-K + V-swap) /
contains_key / remove / len. Identical stdout on both
backends; no leaks under default malloc. Added to parity
test list.

**2 new lib tests** covering:
  - both backends compile HashMap<OwnedStr, OwnedStr> with the
    `intent_hashmap_owned_str_owned_str` bundle prefix
  - C bundle emits per-slot `free(m->keys[i])` +
    `free(m->values[i])` (both-axes drop walk) and both
    `char* k_owned = malloc(nk+1)` + `char* v_owned = malloc(nv+1)`
    in insert

**1787 lib + 58 parity green. Only ARC 4.6 remains in the
wider K-V arc.**

**Prior:** 2026-06-03 (**ARC 4.2 COMPLETE — `HashMap<i64, OwnedStr>`
end-to-end on both backends.** i64 keys, OwnedStr values. Map
clones each V internally on insert (same affine-system
workaround as ARC 4.1 K=OwnedStr — strlen + malloc + memcpy);
drop/clear walks free each stored V. `_insert` on duplicate
returns the prior V pointer ownership back to the caller via
Option<OwnedStr>; `_remove` transfers stored V pointer ownership
to caller; `_get` returns a fresh clone so map + caller each
own one.

4.2 checker relaxation: `check_hashmap_builtin` accepts V=OwnedStr
when K=i64 (the v1 scope). Wider K + OwnedStr V is ARC 4.3.

4.2 (C bundle): new `emit_intent_hashmap_pair_c_body_i64k_strv`
emits per-(i64, OwnedStr) bundle with prefix
`intent_hashmap_int64_t_owned_str`. Values field is `char**`;
each slot owns its V pointer. Drop / clear walk all occupied
slots and `free` each. Insert clones via strlen-malloc-memcpy;
duplicate path extracts old V pointer (returned to caller),
stores clone of new V. Remove returns slot's V pointer
(transferred out), tombstones the slot. Get clones the stored V.

4.2 (LLVM bundle): `emit_intent_hashmap_pair_llvm_i64k_strv`
mirrors C side. Bundle struct: `{i64*, i8**, i8*, i64, i64,
i64}`. Drop loops free each occupied V; clear mirrors. Insert
clones via `strlen + malloc + memcpy`; duplicate returns prior
V pointer in Option payload. Remove transfers V pointer out
and nulls the slot. Get clones the stored V.

4.2 dispatch: prologue match restructured from `match &p.key`
to `match (&p.key, &p.value)` on both backends to allow the
(i64, OwnedStr) special-case alongside scalar-V/struct-K/etc.
LLVM `hashmap_llvm_dispatch` extended: V=OwnedStr → v_tag
"owned_str", v_llvm "i8*", v_mangle "OwnedStr"; produces prefix
`intent_hashmap_int64_t_owned_str`.

4.2 (end-to-end): `examples/hashmap_strv.vani` exercises
insert (with overwrite) / contains_key / remove / len —
exit clean, no double-free / leak, identical stdout on both
backends. Added to parity test list.

**2 new lib tests** covering:
  - both backends compile HashMap<i64, OwnedStr> with the
    `intent_hashmap_int64_t_owned_str` bundle prefix
  - C bundle emits `free(m->values[i])` (per-slot V drop walk)
    + `char* v_owned = (char*)malloc(n + 1)` (insert clone)

**1785 lib + 57 parity green.**

**Prior:** 2026-06-03 (**ARC 4.4 COMPLETE — `HashMap<(i64, …, i64), V>`
end-to-end on both backends.** Tuple K with FNV-1a per-element
hash_combine + pairwise field equality. Elements are Copy (i64)
so no drop walk needed. Initially scoped to any-arity tuples
of i64; wider element types (struct/OwnedStr) deferred.

4.4 checker relaxation: `check_hashmap_builtin` accepts K=Tuple
where every element is i64. Diagnostic enumerates all five
admissible K shapes (i64 / f64 / OwnedStr / (i64, …, i64) /
struct-with-Hash+Eq).

4.4 (C bundle): `emit_intent_hashmap_pair_c_body_tuple_i64k`
emits per-(Tuple, V) bundle with prefix
`intent_hashmap_tup_<arity>_i64_<V>`. Keys field is a contiguous
array of `intent_tuple_int64_t_…_int64_t` structs (reuses the
existing tuple typedef machinery). Hash function FNV-1a's each
i64 element's 8 bytes into the running hash. Equality is
pairwise `==` over `_0`, `_1`, … fields chained with `&&`.

4.4 (LLVM bundle): `emit_intent_hashmap_pair_llvm_tuple_i64k`
mirrors the C side. Key type is the anonymous-struct literal
`{i64, i64, …}`; keys field is a flat `{i64, …}*`. Hash uses
`extractvalue` + FNV-1a loops per element. Equality is a
helper `__eq_key` that extracts each field, icmp eq's them,
and chains via `and`. Dispatch helpers updated:
`hashmap_llvm_dispatch` produces `intent_hashmap_tup_<n>_i64_<V>`;
`_with_k` returns `{i64, i64, …}` as the LLVM key type.

4.4 (end-to-end): `examples/hashmap_tup.vani` exercises insert
/ get / contains_key / remove / len with `(i64, i64)` keys —
duplicate-insert overwrites value, distinct-bucket lookups
miss, removes shrink length. Identical stdout on both backends.
Added to the parity test list.

**2 new lib tests** covering:
  - both backends compile HashMap<(i64,i64), i64> with the
    `intent_hashmap_tup_2_i64_int64_t` bundle prefix
  - LLVM emits an internal `__eq_key` helper + `extractvalue`
    calls for the per-element hash + equality dispatch

**1783 lib + 56 parity green.**

**Prior:** 2026-06-03 (**ARC 4.1 COMPLETE — `HashMap<OwnedStr, V>`
end-to-end on both backends.** String keys via FNV-1a byte hash
+ `strcmp` equality. Bundle clones each key internally
(strlen + malloc + memcpy — C11-portable; no POSIX strdup
dependency) so the caller retains ownership of the OwnedStr
passed in. Matches Rust's `m.insert(key.clone(), v)`
ergonomics; sidesteps the affine-system gap where local-drop
isn't yet suppressed for OwnedStr moved into builtin args.

4.1 checker relaxation: `check_hashmap_builtin` accepts K=OwnedStr
alongside i64 / f64 / struct-with-Hash+Eq. Diagnostic
enumerates all four admissible K shapes.

4.1 (C bundle): `emit_intent_hashmap_pair_c_body_strk` emits
per-(OwnedStr, V) bundle with prefix
`intent_hashmap_owned_str_<V>`. Keys field is `char**`. Drop /
clear walk all occupied slots and `free` each stored copy.
`_insert` strlen-malloc-memcpy's the caller's pointer before
storing. Duplicate-key path just swaps in the new value (no
free of caller's k); `_remove` frees the stored key copy.

4.1 (LLVM bundle): `emit_intent_hashmap_pair_llvm_strk` mirrors
C bundle. Key type `i8*`; hash via byte-by-byte FNV-1a loop
that walks the null-terminated string; equality via
`call i32 @strcmp`. Insert clones via
`strlen + malloc + memcpy` (memcpy is an LLVM intrinsic, no
explicit declare needed). Dispatch helpers updated:
`hashmap_llvm_dispatch` returns `intent_hashmap_owned_str_<V>`
for OwnedStr K; `_with_k` returns `"i8*"` as the LLVM key type.

4.1 (end-to-end): `examples/hashmap_str.vani` exercises
insert / get / contains_key / remove / len with string keys
"1", "2", "42". Identical stdout on both backends. Added to
the parity test list.

**3 new lib tests** covering:
  - both backends compile HashMap<OwnedStr, i64> with the
    `intent_hashmap_owned_str_int64_t` bundle prefix
  - LLVM emits `call i32 @strcmp` for equality + `strlen` /
    `memcpy` for the insert clone
  - C bundle declares `char** keys` + `__hash_key(const char* k)`
    + `strcmp(m->keys[i], k)`

**1781 lib + 55 parity green.**

**Prior:** 2026-06-03 (**ARC 4.5 COMPLETE — `HashMap<f64, V>`
end-to-end on both backends.** f64 keys with built-in `==`
equality (NaN keys unrecoverable, documented in unsafe.md) and
reinterpret-bits FNV-1a hashing. Checker, C bundle, LLVM
bundle, dispatch helpers, parity example all in one atomic
landing.

4.5 checker relaxation: `check_hashmap_builtin` now also
accepts `HashMap<f64, V>` (alongside `HashMap<i64, V>` and
`HashMap<Struct, V>`). Diagnostic updated to mention all three
admissible K shapes.

4.5 (C bundle): new `emit_intent_hashmap_pair_c_body_f64k`
emits per-(f64, V) bundle with prefix
`intent_hashmap_double_<V>`. Hash function reinterprets f64
bits via `memcpy` to a uint64 then FNV-1a's the 8 bytes
(matches the i64-K hash for cross-key-shape distribution).
Equality uses native `==` (so NaN keys are unrecoverable —
inserted but never findable).

4.5 (LLVM bundle): `emit_intent_hashmap_pair_llvm_f64k` mirrors
the C side. Key type is `double`; hash function uses
`bitcast double to i64` then the same FNV-1a loop. Equality
uses `fcmp oeq` (ordered equality — NaN comparisons return
false, matching the IEEE 754 + C semantics). Dispatch helpers
extended: `hashmap_llvm_dispatch` returns
`intent_hashmap_double_<V>` for f64 K; `_with_k` returns
`"double"` as the LLVM key type.

4.5 (end-to-end): `examples/hashmap_f64.vani` exercises
new/insert/get/contains_key/remove/len with f64 keys
3.14/2.71/42.0, identical stdout on both backends, added to
parity test list.

**3 new lib tests** covering:
  - both backends compile HashMap<f64, i64> with the
    `intent_hashmap_double_int64_t` bundle prefix
  - LLVM emits `fcmp oeq double` for f64 K equality + `bitcast
    double to i64` in the hash function
  - C bundle declares `double* keys` and `__hash_key(double k)`

**1778 lib + 55 parity green.**

**Prior:** 2026-06-03 (**ARC 1.7 COMPLETE — `HashMap<UserStruct, V>`
end-to-end on both backends.** All four sub-steps (1.7a struct-K
hash dispatch / 1.7b struct-K equality / 1.7c LLVM mirror /
1.7d round-trip + cross-backend parity) shipped in one atomic
landing.

1.7 checker relaxation: `check_hashmap_builtin` now accepts
`HashMap<Struct, V>` when both `implement Hash for Struct` and
`implement Eq for Struct` are in scope. The Hash bound was
already enforced in ARC 1.2; Eq is the new requirement for
1.7's key-equality delegation. Improved diagnostic: "supports
`HashMap<i64, V>` for scalar V, or `HashMap<Struct, V>` when
Struct implements both `Hash` and `Eq`".

1.7a/b (C bundle): new `emit_intent_hashmap_struct_pair_c_body`
emits per-(Struct K, V) bundle with prefix
`intent_hashmap_Struct_<K>_<V>`. Hash function calls the user's
`fn_<K>_hash` (FNV-1a wrapped around the user's i64 hash so
single-field structs still distribute). Key equality uses
`fn_<K>_eq`. Struct K values are passed by value (vāṇी structs
are Copy).

1.7c (LLVM bundle): `emit_intent_hashmap_struct_pair_llvm`
mirrors the C side. Struct types lower to `%Struct_<name>` in
LLVM IR; `ptrtoint` on `getelementptr-of-null` computes the
struct's runtime size for malloc. emit_call dispatch helpers
extended to also return K's LLVM type for call-site
parameterization.

1.7d (end-to-end): `HashMap<Score, i64>` works on both
backends. `Score` defines `Hash` + `Eq` impls returning i64 +
bool. `hashmap_insert(mut ref m, Score{val:1}, 100)` works,
`hashmap_len(ref m)` returns 2 after two inserts. Both
backends return exit 2.

**4 new lib tests**:
  - type-checks with Hash + Eq impls
  - C emits per-pair struct-K bundle delegating to fn_<K>_hash
    + fn_<K>_eq
  - LLVM emits per-pair struct-K bundle with the same delegation
  - missing Eq impl rejected at the checker with a clear
    diagnostic mentioning both interfaces

**1775 lib + 54 parity green. ARC 1 COMPLETE** — HashMap
monomorphization for scalar V (i8/i16/i32/i64/u8/u16/u32/u64/bool)
+ user struct K with Hash + Eq impls all work end-to-end
cross-backend. Only ARC 4 (wider K-V — OwnedStr / Tuple / Vec /
f64 / non-Copy V) remains open.

**Prior:** 2026-06-03 (**ARC 1.4 + ARC 1.5 COMPLETE —
per-(K, V) HashMap bundle, both backends.** `HashMap<i64, V>`
for V ∈ {i8, i16, i32, i64, u8, u16, u32, u64, bool} now
compiles + runs end-to-end on **both** the C and LLVM backends
with cross-backend parity preserved. Apply of the ARC 2.3 success
pattern (mechanical translation with C as gold reference).

ARC 1.5a: new `emit_intent_hashmap_pair_llvm(out, v_llvm, v_mangle,
v_tag, v_size, has_option_v)` produces a per-(K=i64, V) LLVM
bundle parallel to the C-side 1.4c. Struct typedef
`%intent_hashmap_int64_t_<V_tag>` + new/drop/hash_key/insert_raw/
grow/contains_key/len/clear. Get/insert/remove gated on
has_option_v. K=i64 stays in v1 (hash function unchanged); V's
LLVM type, byte size, and Option<V> mangled name parameterize.

ARC 1.5b: LLVM prologue mirrors C-side 1.4d — walks
`collect_hashmap_pairs(program)`, emits per-pair bundle for each
non-(i64, i64) shape, skips (i64, i64) (legacy bundle covers it).

ARC 1.5c: LLVM `emit_call` hashmap_* arms dispatch via three
helpers (`hashmap_llvm_dispatch_from_kv` / `_from_ty` /
`_from_recv`) that derive `(prefix, v_llvm, opt_v)` from the call
site's type. `llvm_type_string` extended with HashMap arm so
binding declarations use the right struct typename. HashMap drop
emission updated similarly.

ARC 1.5d: LLVM SSA no-op — same `ssa_path_supports` rejection
applies (hashmap programs fall back to tree LLVM path).

ARC 1.5e: **4 new lib tests** pin cross-backend parity —
HashMap<i64, u32/u64/i32> produce per-pair LLVM bundles,
HashMap<i64, i64> keeps the legacy LLVM prefix. End-to-end
smoke: `HashMap<i64, u32>` returns exit 2 on both backends.
Cross-backend parity sweep (54 tests) still green.

**Sub-arc 1 status: HashMap monomorphization for scalar V is
COMPLETE on both backends.** Only ARC 1.7 (struct K) and ARC 4
(wider K, OwnedStr/Tuple/Vec V) remain. **1771 lib + 54 parity
green.**

**Prior:** 2026-06-03 (**ARC 1.4 — C-side per-(K, V)
HashMap bundle COMPLETE.** All six C-side sub-steps (1.4a Option<V>
mono / 1.4b checker relax / 1.4c parameterized bundle / 1.4d
prologue wiring / 1.4e tree emit_call dispatch / 1.4f SSA no-op
— hashmap programs fall back from SSA to tree path) shipped this
session. `HashMap<i64, V>` for V in {i8, i16, i32, i64, u8, u16,
u32, u64, bool} compiles + runs end-to-end on the C backend.
LLVM backend still uses the legacy single-shape bundle —
ARC 1.5a-e (~500-line LLVM IR mirror, comparable to ARC 2.3
trie work) remains as the next focused chunk. Cross-backend
parity still holds for the legacy `HashMap<i64, i64>` shape.
**1767 lib + 54 parity green.**

ARC 1.4f acknowledgment: the SSA C backend has no per-builtin
HashMap dispatch — `ssa_path_supports` (in main.rs) rejects any
program calling hashmap_* builtins, falling back to the tree
backend. So 1.4f is automatic; the 1.4e tree dispatch covers
both call paths.

**Prior:** 2026-06-03 (**ARC 1.4c + 1.4d + 1.4e shipped —
per-(K, V) C bundle, prologue wiring, tree emit_call dispatch.**
C backend now supports `HashMap<i64, V>` end-to-end for V in
{i8, i16, i32, i64, u8, u16, u32, u64, bool}.

1.4c: new `emit_intent_hashmap_pair_c_body(out, v_tag, v_ctype,
option_v_mangle, has_option_v)` emits a per-(K, V) bundle with
prefix `intent_hashmap_int64_t_<V_tag>`. The legacy
`intent_hashmap_i64_i64_*` bundle stays untouched for
backwards compat. Get/insert/remove emission gated on
`has_option_v` (the Option<V> enum's presence in
`ENUM_PAYLOAD_REGISTRY`).

1.4d: C backend prologue walks `collect_hashmap_pairs(program)`
and emits a per-pair bundle for each non-(i64, i64) shape. The
legacy (i64, i64) is skipped here — already covered by the
existing `program_uses_i64_i64_hashmap` branch.

1.4e: `emit_call` dispatches on (K, V) extracted from the
result-ty (`hashmap_new`) or args[0].ty (other ops). New
helpers `hashmap_prefix_from_kv` / `_from_ty` / `_from_recv`
produce the bundle prefix. `c_type_name` also extended with a
HashMap arm so the binding's declared C type matches the per-
pair struct. HashMap drop emission updated similarly.

**End-to-end verified:** `let m: HashMap<i64, u32> = hashmap_new();
hashmap_insert(mut ref m, 1, 100); return hashmap_len(ref m);`
compiles + runs to exit 2 on the C backend. LLVM backend still
on legacy path (1.5a-e pending).

ARC 1.4a (Option<V> mono) had a second walker `collect_apply_in_stmt`
that also needed the HashMap arm — patched. Also added a
payload-eligibility check (V must be scalar/bool/Str/OwnedStr) so
nested `HashMap<i64, HashMap<i64, i64>>` doesn't try to register
the invalid `Option<HashMap<...>>`.

**4 new lib tests** (HashMap<i64, u32/u64/i32> emit per-pair
bundles, HashMap<i64, i64> keeps legacy prefix). **1767 lib + 54
parity green.** Sub-arc 1.4 (C side) COMPLETE; 1.4f (SSA C
dispatch) and 1.5a-e (LLVM mirror) remain.

**Prior:** 2026-06-03 (**ARC 1.4a + 1.4b shipped —
Option<V> auto-monomorphization + checker relaxation.** First
two sub-steps of the per-(K, V) HashMap bundle arc.

1.4a: extended `collect_apply_in_ty` so any `Type::HashMap(K, V)`
or `Type::BTreeMap(K, V)` annotation registers `Option<V>` as a
needed monomorphic enum. Previously only `Option<i64>` was
auto-registered (via the hard-coded path in
`walk_expr_for_search_builtins`); now any V type used as a
HashMap V triggers its own `Option<V>` instantiation. Backwards
compatible — (i64, i64) was already registered.

1.4b: relaxed `check_hashmap_builtin` from `(i64, i64)`-only to
`HashMap<i64, V>` where V is any scalar (i8/i16/i32/i64/u8/u16/
u32/u64/bool). Key coercion now uses the actual k_ty (still
i64 in v1), value coercion uses v_ty, and the return type for
get/insert/remove is `Option<V>` mangled by the actual V.
Wider K types (u64, struct, OwnedStr) remain restricted —
arrive in ARC 1.7 / Arc 4. **4 new lib tests**
(HashMap<i64, u32>, HashMap<i64, i32>, HashMap<i64, bool>
type-check; HashMap<u64, i64> still rejected with the
upgraded diagnostic). One existing test was updated to reflect
the new checker behaviour. **1763 lib + 54 parity green.**
**Mid-arc state:** checker accepts the new shapes, but the
per-(K, V) backend bundle still needs to land (ARC 1.4c–1.5e)
before codegen succeeds for non-(i64, i64) shapes.)

**Prior:** 2026-06-03 (**Remaining-work breakdown +
sequenced ledger added to ARCS.md and TODO.md.** The open Arc
1.4–1.7 and Arc 4 work is now structured as 21 granular sub-
steps with explicit dependencies (Arc 1.4a → 1.4b → 1.4c → 1.4d
→ 1.4e → 1.4f → 1.5a-e → 1.7a-d → 4.1-4.6). Each sub-step lists
effort, depends-on, and acceptance criterion so future sessions
can pick up at any sub-step without re-planning. Total
remaining work: ~32h across ~50 commits if all pursued.
TODO.md's "Top-of-queue planning docs" reflects the current
state — safety-standard arc COMPLETE, Arcs 2/3 COMPLETE, Arc 1
partial, Arc 4 open. ARCS.md leads with a "Remaining work —
sequenced sub-step breakdown" section + a cross-dependency
graph. **1759 lib + 54 parity green.**)

**Prior:** 2026-06-03 (**`intentc audit-pack` extended
with at-a-glance Summary table.** Follow-up to the audit-pack
capstone. The Markdown output now leads with a Summary section
formatted as a 6-row table — unsafe deviations count, max stack
depth (or UNBOUNDED), cycle count + violation count, HashMap
shape count, max McCabe score, count of safety-tagged
functions. Computed up-front from each per-fact report, so the
reviewer sees the key numbers at the top without scrolling.
Rest of the document is unchanged (six sections, one per
per-fact report). Smoke-tested with a mixed-annotation program
— rows render correctly (`120 bytes`, `1` shape, `4` max
complexity, `2 of 4` tagged). **1759 lib + 54 parity green.**)

**Prior:** 2026-06-03 (**`intentc audit-pack` shipped —
meta CLI that bundles all 6 audit reports into a Markdown
document.** Capstone of the audit-CLI family. New CLI:
`intentc audit-pack <path> [--out=report.md] [--max-stack=<bytes>]
[--max-complexity=<N>]`. Runs all six per-fact CLIs internally
(`deviations`, `stack-depth`, `acyclicity`, `hashmap-usage`,
`complexity`, `safety-attrs`) and bundles their outputs as
Markdown sections in a single deliverable. With `--out`, writes
to the file; otherwise prints to stdout. The two hard-gate
flags (`--max-stack`, `--max-complexity`) plus the inherent
acyclicity gate (exits 1 on any non-bounded cycle) compose into
a CI-friendly all-in-one safety/audit run. One reviewer-facing
output: a single Markdown document with one section per
reviewable shape — pipe through standard markdown tooling for
PDF / HTML / git artifact pinning. Smoke-tested end-to-end with
a mixed-annotation program — produces a 61-line report with
proper section ordering, hard-gate exit codes verified
(`--max-complexity=2` exits 1, `--max-stack=50` exits 1,
lenient gates exit 0). **No new lib tests** (the underlying
report builders are already covered by their per-CLI test
suites; this is just an orchestrator). **1759 lib + 54 parity
green.**

`intentc` now has a **complete 7-CLI audit family**: 6 per-fact
CLIs + 1 capstone `audit-pack` that runs them all. Together
they cover the embedded / safety-critical review surface end-
to-end with a single command for the audit pack deliverable.)

**Prior:** 2026-06-03 (**`intentc safety-attrs` shipped —
per-function listing of every safety annotation.** New CLI:
`intentc safety-attrs <path> [--format=text|json|csv]`. One row
per function with columns for every safety primitive
(`#[no_heap]`, `#[no_float]`, `#[no_recursion]`, `#[interrupt]`,
`#[deterministic_timing]`), composite standard tag
(`#[misra_c_2012]` / `#[asil_d]` / `#[do178c_level_a]` /
`#[iec_62304_class_c]`), budget annotations (`bounded_stack_bytes`,
`wcet_cycles`, `bounded_recursion`), and the `is_pure` flag.
Composite tag expansion is REFLECTED — `#[asil_d]` shows up on a
function alongside its derived `#[no_heap]` + `#[no_recursion]`
primitives, so reviewers see both the declared standard and its
constraint set. Reviewer-facing audit pack — no exit-code gate
(it's an artifact, not a check). Pair with `intentc complexity`,
`intentc stack-depth`, `intentc deviations`, `intentc acyclicity`,
`intentc hashmap-usage` for full coverage. **5 new lib tests**
(composite expansion reflected, budgets carried, text format
lists every annotation, pure fn reflected, unannotated fn shown
as `(none)`). **1759 lib + 54 parity green.**

The `intentc` CLI now has **six audit-artifact subcommands**
following the established `--format=text|json|csv` pattern —
together they cover the embedded / safety-critical review
surface end-to-end:

| CLI | Artifact |
|---|---|
| `deviations` | `unsafe(reason = "…")` block records |
| `stack-depth` | Per-fn frame size + entry-point max depth |
| `acyclicity` | Call-graph cycles (mutual recursion catcher) |
| `hashmap-usage` | HashMap<K, V> pair instantiations |
| `complexity` | Per-fn McCabe cyclomatic scores |
| `safety-attrs` | Per-fn safety annotation set |)

**Prior:** 2026-06-03 (**`intentc complexity` standalone
audit CLI shipped — per-function McCabe scores as artifact.**
T2.4 follow-up. The same cyclomatic-complexity counter that the
existing `enforce_complexity` checker pass uses (opt-in via
`INTENT_CHECK_COMPLEXITY=1` env var) is now also queryable
standalone: `intentc complexity <path> [--max=N] [--format=text|json|csv]`.
Per-function scores in declaration order, three output formats
matching the established pattern. With `--max=N`, exits 1 if
any function exceeds N — CI-friendly hard gate. Useful for
MISRA / ISO 26262 / DO-178C reviews where complexity ceilings
(typically 10-15) are part of the standard rubric. **5 new lib
tests** (report walks all fns, text marks over-threshold, JSON
well-formed, CSV header + rows, no-threshold means no
violations). Smoke-tested end-to-end — `branchy()` scores 5 with
default `--max`, scores 5 with `--max=3` and triggers exit 1.
**1754 lib + 54 parity green.**)

**Prior:** 2026-06-03 (**`intentc hashmap-usage` subcommand
shipped — surfaces every HashMap<K, V> pair the program uses
as an audit artifact.** New CLI: `intentc hashmap-usage <path>
[--format=text|json|csv]`. Walks the typed program via the
ARC 1.3 `collect_hashmap_pairs` collector and emits each
deduped (K, V) pair with the mangled bundle tag
(`intent_hashmap_<K>_<V>`). Three output formats matching the
established pattern (text default for console, JSON for CI,
CSV for spreadsheet review). Useful for embedded teams
reviewing HashMap shapes before sign-off, and as the input
shape catalog that the eventual ARC 1.4/1.5 per-(K, V) bundle
emitters will consume. **4 new lib tests** (empty program text
format, single-pair text/JSON/CSV well-formed). Smoke-tested
all three formats end-to-end with `intentc hashmap-usage`.
**1749 lib + 54 parity green.**)

**Prior:** 2026-06-03 (**ARC 2.3 shipped — sparse trie
LLVM mirror. ARC 2 (Trie sparse) COMPLETE.** The LLVM trie
implementation now mirrors the C-side sparse representation:
11-field struct `{ i8**, i32**, i16*, i16*, i64*, i8*, i64×5 }`,
five new helper functions (`intent_trie__find_slot`,
`__lower_bound`, `__grow_node`, `__insert_pair`, `__remove_pair`),
and all 5 user-facing ops rewritten in LLVM IR using them.
Manual SSA + GEP threading throughout — careful per-function
label prefixes (`fs_`, `lb_`, `gn_`, `ip_`, `rp_`, `tn_`, `tnu_`,
`td_`, `ti_`, `tw_`, `tc_`, `tsw_`, `tde_`, `trc_`) to avoid label
collisions in LLVM's per-function namespace. Two existing lib
tests had pinned the legacy LLVM struct shape and 1024-byte
realloc pattern — updated to match the new sparse-shape
invariants. Trie example output is now byte-identical across
both backends with the sparse implementation. Cross-backend
parity sweep still green. **1745 lib + 54 parity green. ARC 2
COMPLETE — both backends use sparse children, ~30–100× memory
reduction for sparse alphabets vs the legacy dense 256-wide
storage.**)

**Prior:** 2026-06-03 (**ARC 2.1 + 2.2 shipped — sparse-
children trie on the C backend.** The per-node storage flipped
from a flat dense `int32_t children[256]` (1024 bytes/node fixed)
to struct-of-arrays sparse pairs: `node_keys` (u8 sorted array)
and `node_children` (parallel i32 array) per node, sized to the
actual fan-out. Memory usage drops from 1024 bytes/node fixed to
4N bytes/node (N = live children), with `O(log N)` lookup via
binary search. Freelist now uses a dedicated per-node
`free_next` array rather than reusing a child slot — cleaner and
removes the byte-0-special-case implicit in the dense layout.
**All 5 ops rewritten atomically in a single commit** as the
ARCS plan required (insert / walk / contains / starts_with /
delete + clear). `intent_trie_drop` walks every per-node buffer
to free, then frees the parent arrays. The 13 lib tests + the
trie.vani example produce **byte-identical output** to the
legacy dense implementation across both backends. **ARC 2.3
(LLVM mirror) deferred** — the LLVM helpers are ~3× the C
footprint (491 vs 168 lines) and the manual SSA / GEP / label
management makes a careful rewrite a focused effort better
suited to a dedicated session. Cross-backend parity holds today
via observably-equivalent semantics: C-backend = sparse, LLVM-
backend = dense, both produce identical user-visible output.
**1745 lib + 54 parity green.**)

**Prior:** 2026-06-03 (**ARC 2.4 shipped (standalone) —
trie depth + breadth confidence tests.** The full sparse-
representation rewrite (ARC 2.1-2.3) is a multi-hour cross-
backend atomic landing deferred to a focused fresh session;
this commit ships ARC 2.4's deliverable proactively against the
existing dense-children implementation. Three new lib tests:
(a) long-prefix round trip — 33-char `supercalifragilisticexpialidocious`
+ shorter prefixes, exercises walk depth on both backends;
(b) wide-branching round trip — 8 sibling words at depth 1
exercising the children-array per-node when populated heavily;
(c) shared-prefix delete behavior — `car` deleted from a trie
holding {car, cart, card} preserves cart/card and starts_with.
These tests now anchor the regression gate for the eventual
sparse-representation rewrite — when 2.1-2.3 land, this trio
plus the existing example-driven trie tests must continue to
pass byte-for-byte. **3 new lib tests.** **1745 lib + 54 parity
green.**)

**Prior:** 2026-06-03 (**ARC 1.6 shipped — HashMap<i64, i64>
regression sweep.** Documented as a single end-to-end lib test
that exercises every legacy bundle op (`hashmap_new` + `_insert`
+ `_len` + `_remove` + `_clear`) in one program, compiling to
both backends. The implicit regression coverage from running the
full lib suite (1742 tests) on every commit has been load-bearing
through ARC 1.1 / 1.2 / 1.3 — this new test names the gate
explicitly so future ARC 1.4 / 1.5 work (per-(K, V) bundle
emission) has a clean baseline to compare against. **1 new lib
test.** **1742 lib + 54 parity green.**)

**Prior:** 2026-06-03 (**ARC 3d shipped — `Vec<Tuple<...>>`
in closure bodies; LLVM `vec_struct_tag` bug fix.** Final
sub-track of the richer-closures arc. Smoke-testing closures that
capture `Vec<(i64, i64)>` by reference and index into the tuples
surfaced a real LLVM backend bug: `vec_struct_tag` (which derives
the per-(element-type) Vec helper name) had explicit arms for
Struct / Enum / FnPtr / Object but fell through `Type::Tuple`,
`Type::Vec`, and `Type::Array` to the panicking `llvm_type` arm.
Added recursive arms that compose the inner type's tag — Tuple
spells as `tuple_<e0>_<e1>_...`, nested Vec as `vec_<inner>`,
Array as `arr_<len>_<element>`. Mirrors the C backend's already-
correct tag derivation. **3 new lib tests** (Vec<(i64,i64)> index
+ .1 access, both-backends compile, tuple .0 field access).
End-to-end smoke: both backends return exit 20 from `pairs[1].1`
on `[(1,10),(2,20),(3,30)]`. **1741 lib + 54 parity green.** **ARC
3 (richer closures) COMPLETE** — all four sub-tracks (3a
capture-by-ref / 3b non-i64 elements / 3c .collect chain / 3d
Vec<Tuple>) shipped. ARC 2 (Trie sparse) is next.)

**Prior:** 2026-06-03 (**ARC 3c shipped — `.collect()`
chain syntax.** Third sub-track of the richer-closures arc. New
postfix `.collect()` is recognized as identity-on-Vec: vāṇी's
combinators are eager (`.map(f)` already returns a fresh
`Vec<T>`), so `xs.map(f).collect()` is sugar for `xs.map(f)`. The
sugar terminates iterator-chain expressions in the Rust-familiar
way without requiring a separate lazy-iterator runtime. Two
handler sites: (a) the bare-Var Vec method-sugar path catches
`xs.collect()` (identity); (b) the generic method-dispatch path
catches `xs.map(f).collect()` (chained-call receiver — checks
the receiver's typed type for `Vec<T>` and returns the checked
receiver unchanged). `.collect()` on a borrowed `Ref<Vec<T>>`
rejects with a clear diagnostic — would need an explicit clone
to produce an owned Vec, out of v1 scope. **4 new lib tests**
(map.collect, filter.collect, identity-on-Vec, borrowed-Vec
rejection). Smoke-tested both backends with `xs.map(double).
collect()` — both return exit 6 from index 2 of [2, 4, 6].
**1738 lib + 54 parity green.** ARC 3 closures track: 3a + 3b +
3c all shipped; 3d (Vec<Tuple<i64,i64>> closures) is the final
sub-track.)

**Prior:** 2026-06-03 (**ARC 3b verified shipped — non-i64
element types in closure contexts.** Originally planned as a
5-step landing for `Vec<Str>` / `Vec<f64>` etc. in closure
bodies, but inspection revealed the constraint no longer exists
at ship time: (a) Vec element-type parameterization landed in
closure #291 (Vec<T> for non-Copy T), and (b) ARC 3a's `[ref xs]`
capture mechanism unlocks the closure-side equation — non-Copy
elements flow through closure bodies as `Ref<Vec<T>>` and the
body's `xs[i]` / `len(xs)` / method calls all work via the
existing Ref-param conventions. **4 new lib tests** pin the
supported cases (f64 closure body, Vec<f64> ref-capture indexing,
Vec<OwnedStr> ref-capture + len, f64 closure result inferred).
**1734 lib + 54 parity green.** ARCS.md updated with the "already
shipped — unlocked by 3a" annotation. ARC 3c (`.collect` chain
syntax) is next in the closures arc.)

**Prior:** 2026-06-03 (**ARC 3a shipped — capture-by-ref
closures (all 5 sub-steps atomic).** First sub-track of the
richer-closures arc. New surface syntax:
`fn(p: T) -> R [ref name1, ref name2] { body }` — between the
return type and the body, an explicit capture list naming
variables to be captured by reference rather than by value. The
list is opt-in: omitting `[...]` keeps the v1 default (implicit
captures, by-value snapshot semantics via the existing free-var
detector). `[ref xs] {...}` lifts each ref-captured name's
type to `Type::Ref<T>` in the hoisted fn's param list, and the
call site passes `ref xs` instead of `xs`. Critically, this
unlocks **non-Copy captures** (`Vec<i64>`, `OwnedStr`, `HashMap`,
etc.) which previously couldn't be captured at all — the
existing `Ref<T>` codegen + drop machinery handles them
automatically across both backends, so 3a.3/3a.4/3a.5 (C / LLVM /
drop semantics) all came along for free. **5 new lib tests**
(Vec ref-capture typechecks, C emits `__cap_xs` param, default
by-value still works, multiple ref-captures compose,
bare-identifier in `[...]` requires `ref` keyword). Smoke-tested
end-to-end with both backends — `vec(10,20,30)` ref-captured
through closure, indexed, both backends return exit 20.
**1730 lib + 54 parity green.** ARC 3a complete; ARC 3b (non-i64
element types in closures) is next in the closures arc.)

**Prior:** 2026-06-03 (**ARC 1.3 shipped — HashMap (K, V)
pair collector.** Third sub-step of the HashMap monomorphization
arc. New `src/hashmap_bundle.rs` module with the
`collect_hashmap_pairs(program)` API. Walks every type position
in a typed program (function signatures, function bodies, struct
fields, nested types) and returns a deduped, topologically-
ordered list of `HashMapPair { key, value, tag }` — the
inner pair of `HashMap<i64, HashMap<i64, i64>>` lands before the
outer so future bundle emitters can rely on the dependency
ordering. Mangled tags follow the
`intent_hashmap_<K_tag>_<V_tag>` scheme; struct K spells as the
struct name verbatim, scalars use their C-leaf names, nested
HashMap collapses to `hm_<K>_<V>`. ARC 1.4 (C bundle emission)
and ARC 1.5 (LLVM bundle emission) will consume this list to
drive per-(K, V) helper synthesis. **4 new lib tests** (single-
pair finding, dedup across multiple uses, struct-key pair
discovery, inner-before-outer ordering for nested HashMap).
**1725 lib + 54 parity green.** ARC 1.4 / 1.5 (atomic pair —
per-(K, V) C + LLVM bundle emission) is next.)

**Prior:** 2026-06-03 (**ARC 1.2 shipped — Hash bound
enforcement on user struct keys.** Second sub-step of the
HashMap monomorphization arc. `validate_array_element_type` (the
shared type-validation helper called by Let annotations, function
params, etc.) extended: when the type is `HashMap<K, V>` and K is
a user-defined struct, the validator looks up
`iface_impl_exists("Hash", K_name)` and emits a clear diagnostic
when no `implement Hash for K` block is in scope. Built-in K
types (i64, u32, OwnedStr, etc.) get compiler-provided hashing
unchanged. The diagnostic names the exact `implement Hash for K
{ fn hash(self: ref Self) -> i64 { ... } }` template the user
should write. Same validator also restated the existing reference-
key/value ban. **4 new lib tests** (struct K without Hash impl
rejected, struct K with Hash impl passes the bound check, i64 K
unchanged, diagnostic walks user through the fix). **1721 lib +
54 parity green.** ARC 1.3 (type collector for per-(K, V) bundle
queue) is next.)

**Prior:** 2026-06-03 (**Devanagari `main()` entry-point
aliases + cross-markdown audit.** `fn मुख्य()` (mukhya — Sanskrit
/ Hindi / Marathi "main / primary"), `fn प्रमुख()` (pramukh —
"primary, foremost"), and `fn प्रधान()` (pradhan — "principal /
chief", same word used in प्रधानमंत्री / Prime Minister) all
canonicalize to `main` at parse time. Compiler emits the same
`@fn_main` symbol regardless of the source spelling; declaring
two of these forms in the same program is a duplicate-fn error.
Adds an entry-point row to the Devanagari alias table in
README.md. **4 new lib tests** plus an end-to-end smoke test
(`fn मुख्य()` compiles, type-checks, and emits the expected
LLVM `@fn_main` symbol). README.md / TODO.md / ARCS.md /
unsafe.md audited: stale "scheduled" claims on the safety-
standard alignment arc replaced with ✅ SHIPPED banners; Tier
1/2/3 + standard composites + MMIO + ARC 1.1 cross-referenced
into the historical plan record. **1717 lib + 54 parity green.**)

**Prior:** 2026-06-03 (**ARC 1.1 shipped — generic-context
`hashmap_new()` inference.** The first sub-step of the HashMap
monomorphization arc. New helper `try_elaborate_empty_hashmap`
mirrors the existing `try_elaborate_empty_vec` pattern: when
`let m: HashMap<K, V> = hashmap_new()` is seen, the let-binding's
type annotation drives the inferred return type instead of the
hardcoded `HashMap<i64, i64>` default. The default (no
annotation) is preserved for backwards compatibility. The other
bundle ops (`hashmap_insert`/`get`/`len`/etc.) still restrict to
(i64, i64) — that restriction is lifted in ARC 1.2-1.7 when the
per-(K, V) bundle emission lands. **3 new lib tests** (default
i64/i64 still works, annotated i64/i64 binds via the new path,
non-default V correctly propagates as the binding type with the
expected bundle-op restriction diagnostic). **1713 lib + 54
parity green.** Tier 3 + safety-standard alignment arc COMPLETE;
ARC work begun.)

**Prior:** 2026-06-03 (**T3.5 of safety-standard arc
shipped — MISRA C 2012 Rule 13.5 tightening. Tier 3 COMPLETE.**
MISRA 13.5 says: "The right hand operand of a logical `&&` or
`||` operator shall not contain persistent side effects." The
short-circuit evaluation rule makes the RHS conditional on the
LHS value, so a side-effecting RHS produces evaluation-order-
dependent behaviour. The new `enforce_misra_13` pass fires for
functions tagged with a standard composite (`#[misra_c_2012]` /
`#[asil_d]` / `#[do178c_level_a]` / `#[iec_62304_class_c]`) or
declared `pure fn`. It rejects any `&&` / `||` whose RHS contains
a call to a non-pure function (or any `CallIndirect` — opaque
callee). Diagnostic explicitly cites MISRA 13.5 and suggests
the lift-to-let workaround. **6 new lib tests** (impure || RHS
rejection, impure && RHS rejection, pure-fn RHS OK, lifted call
OK, unannotated fn unaffected, nested-branch fires inside if).
**1710 lib + 54 parity green.** **Tier 3 of safety-standard arc
COMPLETE.** All five tiers (T3.1-T3.5) plus the four composites
(misra_c_2012 / asil_d / do178c_level_a / iec_62304_class_c) are
shipped; the safety-standard alignment arc is complete. ARCs 1-4
(HashMap monomorph / Trie sparse / closures / wider K-V) next.)

**Prior:** 2026-06-03 (**T3.4 of safety-standard arc
shipped — `#[deterministic_timing]` attribute.** Function-level
opt-in to constant-time execution. The post-check pass walks the
body and rejects every construct whose cycle cost is data-
dependent: `if` arms with unequal cycle estimates (uses the T3.2
WCET model to compute branch costs), `while` loops, `for` loops
with non-const bounds, `for ... in <collection>` iterators
(collection length is opaque), indirect calls through fn-pointers,
and calls into user-defined functions that aren't themselves
`#[deterministic_timing]` AND don't have `#[wcet(cycles=N)]`
declared. DO-178C Level A timing-determinism rule. Composes with
all other primitives (no_heap, no_recursion, bounded_stack,
wcet). **7 new lib tests** (straight-line OK, while-loop reject,
unequal-arm reject, equal-arm OK, unannotated callee reject,
wcet-annotated callee OK, const-bounded for OK). **1704 lib +
54 parity green.** Tier 3: 4 of 5 done (T3.1/T3.2/T3.3/T3.4);
only T3.5 (pure fn → MISRA 13.x tightening) remains.)

**Prior:** 2026-06-03 (**T3.3 of safety-standard arc
shipped — `intentc acyclicity` subcommand + Tarjan's SCC pass.**
Whole-program call-graph cycle detection. New CLI:
`intentc acyclicity <path> [--format=text|json|csv]`. Builds the
call graph from every non-extern function, runs an iterative
(deep-recursion-safe) Tarjan's strongly-connected-components
algorithm, and reports every non-trivial SCC plus every self-loop
where the participating function lacks `#[bounded(N)]`. The exit
code is 1 if any non-bounded cycle is found — a CI-friendly gate.
Catches mutual recursion (`a -> b -> a`) that the single-fn
`#[no_recursion]` check can only see when the annotated fn is
inside the cycle. Required by DO-178C Level A and ASIL-D timing
analysis (every call chain must terminate in bounded time).
Output formats match the established three-format pattern
(text default, JSON for CI, CSV for spreadsheet review).
**8 new lib tests** (clean graph, direct self-call, bounded
self-call exempt, mutual recursion, three-way cycle, all three
output formats well-formed). **1697 lib + 54 parity green.**)

**Prior:** 2026-06-02 (**T3.2 of safety-standard arc
shipped — `#[wcet(cycles=N)]` attribute + cycle estimator.** Per-
function worst-case execution time budget. Surface form:
`#[wcet(cycles=N)]` with N a positive integer. The post-check
pass runs a coarse static cycle estimator on the function body:
scalar ops ~1 cycle, binary/comparison ~2, memory access ~2,
named call ~10 (or the callee's declared `#[wcet]` budget if
present), const-bounded for loops multiplied by (end-start),
print syscalls 50 baseline. Reports UNBOUNDED for: while loops,
non-const-bound for loops, ForIter (collection length not
statically known), TaskSpawn, recursion without a declared
callee WCET budget. The pass rejects when the estimate exceeds N
OR is UNBOUNDED. DO-178C Level A timing requirement. V1 is
intentionally a coarse over-estimator — real WCET analysis
requires architecture-specific timing models (pipeline depth,
cache, branch predictor), deferred. The annotation establishes
the audit trail and catches obvious overruns. **8 new lib tests**
(within-budget OK, unbounded while rejection, const for OK,
over-budget const for rejection, recursion-without-budget
UNBOUNDED, caller delegates to callee budget, cycles=0 parse
rejection, unknown-key rejection). **1689 lib + 54 parity green.**)

**Prior:** 2026-06-02 (**T3.1 of safety-standard arc
shipped — `#[bounded_stack(bytes=N)]` attribute.** Per-function
worst-case stack budget. Surface form: `#[bounded_stack(bytes=N)]`
where N is a positive integer literal. The post-check pass runs
the existing T1.3 `stack_depth::compute_stack_depths` analysis
using the annotated function as entry-point and rejects the
program when (a) the call graph has unbounded recursion from
that entry (depth = UNBOUNDED), or (b) the worst-case frame chain
exceeds N. The deepest call chain is included in the diagnostic
so the heavy caller is identifiable. ASIL-D and DO-178C Level A
both mandate a bounded stack guarantee for critical-path
functions; this attribute + the existing `intentc stack-depth`
CLI together provide the audit trail. Composes cleanly with
`#[bounded(N)]` (recursion bound caps frame copies) and the
other Tier 1/2 primitives. **8 new lib tests** (within-budget OK,
over-budget rejected, unbounded recursion rejected, bounded
recursion within budget OK, bytes=0 parse rejection, unknown key
rejection, stacks with #[no_heap], chain shown in diagnostic).
**1681 lib + 54 parity green.** Starting Tier 3 of safety-
standard arc.)

**Prior:** 2026-06-02 (**T2.1 of safety-standard arc
shipped — MMIO volatile load/store. Tier 2 COMPLETE.** Two new
builtins `mmio_read_u32(addr: i64) -> u32` and
`mmio_write_u32(addr: i64, v: u32) -> i64`. V1 covers u32-width
registers (most common width on ARM Cortex-M and similar
embedded targets); larger widths are a Tier 3 extension. Both
backends emit a `volatile` qualifier on the load/store — tree-C
spells the access as `(*((const volatile uint32_t*)((uintptr_t)addr)))`
(read) / `((*((volatile uint32_t*)((uintptr_t)addr))) = v,
(int64_t)0)` (write); tree-LLVM emits `inttoptr` from the
i64 address to `i32*`, then `load volatile i32, i32* ..., align 4`
/ `store volatile i32 v, i32* ..., align 4`. LLVM preserves
`volatile` across optimization passes — required for peripheral
registers where the act of reading clears interrupt-pending
flags and writes can latch state. Surrounded by an
`unsafe(reason = "MMIO: ...")` block, the deviations extractor
captures the access in the audit artifact with
`target_standard` populated from the enclosing fn's composite
tag. SSA backends (the path used by `intentc emit/run`) also
received the volatile arms in addition to the legacy tree
backends. **8 new lib tests** (typecheck for each builtin,
volatile in C output, volatile + inttoptr in LLVM output,
arg-count violations for both). **1673 lib + 54 parity green.**
**Tier 2 of safety-standard arc complete.** Next: Tier 3 (WCET
budgets, bounded_stack annotations, call-graph acyclicity proof,
deterministic_timing, `pure fn` → MISRA 13.x tightening).)

**Prior:** 2026-06-02 (**Standard composites shipped —
`#[misra_c_2012]` / `#[asil_d]` / `#[do178c_level_a]` /
`#[iec_62304_class_c]`.** Each is a hardcoded alias that expands
to the primitive constraint set the standard requires (v1: all
four map to `{no_heap, no_recursion}` — the additional
primitives `bounded_stack`, `wcet`, `deterministic_timing` are
Tier 3 work). The composite tag string is preserved on the
function and read by the deviations extractor to populate each
row's `target_standard` column. Composites + primitives compose
by union; two composites on the same fn rejected. **6 new lib
tests** (composite expansion verified for each, target_standard
populated, two-composite rejection, composite + primitive
stack). Tier 2 progress: 5 of 6 shipped (T2.2/T2.3/T2.4/T2.5/T2.6);
only T2.1 Mmio remains. **1665 lib + 54 parity green.**)

**Prior:** 2026-06-02 (**T1.3 of safety-standard arc
shipped — Tier 1 COMPLETE.** Stack-depth bound checker via new
`intentc stack-depth <path> [--max=<bytes>] [--entry=<fn>]
[--format=text|json|csv]` subcommand. Per-function frame size
estimate (sum of local-binding sizes + 32-byte prologue
overhead, using a curated type → byte-size table) + call-graph
traversal from each entry-point. Recursion handling:
`#[bounded(N)]` caps at (N+1) frame copies; unbounded recursion
(direct self-call or via cycle) reports `UNBOUNDED` and exits 1
under `--max`. ASIL-D mandatory bound. Outputs match the
established three-format pattern (text default, JSON for CI,
CSV for spreadsheet review). **7 new lib tests** plus end-to-
end smoke (multi-fn call chain, bounded vs unbounded recursion,
--max violation detection, --entry filter, JSON well-formed).
**1644 lib + 54 parity green.** **Tier 1 of safety-standard
arc complete.** Continuing into Tier 2 (Mmio + #[interrupt] +
#[no_float] + complexity warn + #[no_recursion] strict +
ptr-arith diagnostic) per the locked execution order.)

**Prior:** 2026-06-02 (**T1.2 of safety-standard arc
shipped — `#[no_heap]` attribute + `INTENT_NO_HEAP=1` global
mode.** Function attribute reject any code path (transitive
via call graph) that reaches a heap-allocating builtin: Vec
ops, OwnedStr producers, all affine containers, unsafe_alloc,
pool_alloc, region_alloc/borrow, bptr_new. MISRA C 2012
Rule 21.3 alignment. The transitive walker fixpoint-propagates
the "allocates" flag through user-defined fn calls; diagnostics
include the `via call to '<fn>'` chain so users can trace the
violation. Production CLI reads `INTENT_NO_HEAP=1` once at
process entry; library `compile()` skips the env-var read in
`#[cfg(test)]` builds to prevent racing with parallel tests
(1500+ tests would otherwise observe the global flip from
unrelated test threads). Test-only API `compile_with_global_no_heap(src, true)`
exercises the global path without env-var risk. **8 new lib
tests** (direct vec rejected, pure arithmetic accepted,
transitive alloc rejected, global mode rejects/accepts,
OwnedStr concat rejected, stacked attributes compile, unknown
attribute rejected). **1637 lib + 54 parity green.** Next:
T1.3 stack-depth bound checker.)

**Prior:** 2026-06-02 (**T1.1 of safety-standard arc
shipped — `intentc deviations` extractor.** New CLI subcommand
walks every `TypedStmt::UnsafeBlock { reason, ... }` and emits a
structured deviation record in CSV / JSON / human-readable text
formats. Each record: file, line, column, prefix (auto-derived
from reason: MMIO / FFI / DMA / transmute / vendor-SDK / other),
reason text, enclosing function name, target_standard (always
`"none"` in this commit; populated when standard composites
land in subsequent commits). The recursive walker covers nested
unsafe blocks inside if / while / for / task / unsafe scopes.
Usage: `intentc deviations <path> [--format=csv|json|text]
[--out=<file>]`. **7 new lib tests**: each format well-formed,
CSV escapes commas, JSON escapes control chars, walker recurses
into nested blocks, empty program emits sentinel, unknown
prefix falls back to "other". **1629 lib + 54 parity green.**
Next: T1.2 `#[no_heap]` attribute + INTENT_NO_HEAP=1 global mode.)

**Prior:** 2026-06-02 (**Safety-standard alignment arc
planned in TODO.md — implementation scheduled next, before ARCs.**
User-asked scope review (2026-06-02): "any MISRA / AUTOSAR /
other standards items for maximum embedded safety." Plan
landed at commit `bb5b78f`: two-tier attribute system with
orthogonal feature primitives (`#[no_heap]`, `#[no_float]`,
`#[no_recursion]`, `#[no_unsafe]`, `#[bounded_stack(bytes)]`,
`#[wcet(cycles)]`, `#[interrupt]`, `#[deterministic_timing]`)
+ hardcoded standard composites (`#[misra_c_2012]`,
`#[asil_d]`, `#[do178c_level_a]`, `#[iec_62304_class_c]`).
Composition by union (most restrictive wins); per-fn opt-in
plus global mode via `INTENT_NO_HEAP=1` and friends.
**Compile-with-and-without parity locked**: without any tag,
vāṇī behaves exactly as today; tags / env-vars are the only
strictness levers. Three tiers (Tier 1 ~10h, Tier 2 ~25h,
Tier 3 multi-day each). After Tier 1+2 land, vāṇī marketable
as MISRA C 2012-aligned + ASIL-D / DO-178C / IEC 62304-
feasible. **No code in this update — plan only.** Next: execute
Tier 1 (deviations extractor + `#[no_heap]` + stack-depth)
once user approves.)

**Prior:** 2026-06-02 (**Layer 5 `region <name> { ... }`
block syntax shipped — Layer 5 is now ergonomically complete.**
Added `region` (lowercase) as a structure keyword plus Devanagari
alias `क्षेत्र` (kṣetra). The block desugars at parse time to a
bare scope `{ let <name>: Region = region_new(); <body>; }` —
the affine drop emission on Region fires automatically at the
closing `}`, freeing the entire arena in one `free` call. No
new AST node, no new checker pass; the existing
scope-exit drop + ArenaRef no-escape dataflow handle everything.
Smoke test (region block with two `region_borrow_i64` slots +
`aref_store` + 2× `aref_load`) round-trips identically on both
backends. Escape attempt (return ArenaRef from inside the
block) rejected with the expected diagnostic. **4 new lib
tests**: block syntax works end-to-end, escape rejected, drop
emitted in C, Devanagari alias parses. **1622 lib + 54 parity
green.** **unsafe.md is now fully done** — every layer + every
sugar.)

**Prior:** 2026-06-02 (**Layer 5 lifetime-tagged
`ArenaRef<T>` shipped — unsafe.md plan is now FULLY COMPLETE.**
Added `Type::ArenaRef(Box<Type>)` plus three new builtins:
`region_borrow_i64(mut ref r, v) -> ArenaRef<i64>`, `aref_load(a)
-> i64`, `aref_store(a, v) -> i64`. The compile-time scope binding
IS the safety proof: no Tainted wrapping, no canary check, no
generation check, no bounds check — *zero runtime cost per
deref*. Layer 1.2's no-escape dataflow extended to cover
`Type::ArenaRef` — returning an ArenaRef tied to a local Region
is rejected at compile time with a `Handle<T>`/`Pool<T>`
recommendation; passing through a parameter Region works fine.
Taint propagation handles ArenaRef flowing through let-bindings.
Both backends emit `ArenaRef<i64>` as a plain `int64_t*` /
`i64*` machine word; safety is purely compile-time. Smoke test
(borrow two slots, store through one, load both) prints
identical output on both backends. Escape attempt (return an
ArenaRef tied to a local Region) is rejected with the right
diagnostic. **7 new lib tests**: borrow returns ArenaRef,
load returns plain i64 (not Tainted), escape via return
rejected, escape via let-binding rejected, pass-through from
param Region accepted, store emits `*p = v` in C, load emits
`load i64, i64*` in LLVM. **1618 lib + 54 parity green.**
**unsafe.md COMPLETE.** Layers 1.1, 1.2, 1.3, 2.1, 2.2, 3.1,
3.2, 4.1, 4.2, 5 (foundation + lifetime-tagged pointers) all
on `main`. Only the Layer 5 `region <name> { ... }` block-with-
explicit-lifetime-name syntax is parked as ergonomic-only
future work.)

**Prior:** 2026-06-02 (**Layer 5 v2 foundation shipped** —
`Region` bump-allocator arena. Affine type; scope-exit drop frees
the entire arena in a single `free` call. V1 scaffolding ships
the runtime story; the full Layer 5 design's `&'arena T`
lifetime-tagged pointer refinement is queued as a follow-up
(today, `region_alloc_i64` returns raw `*mut i64`).
Three builtins: `region_new()`, `region_alloc_i64(mut ref r, v)`,
`region_len(ref r)`. Bundle gated on program usage; trivial
main without Region produces no helpers. Smoke test (alloc 3
i64 slots in a region, raw_load each through assert_safe, drop
the arena at scope exit) round-trips identically on both
backends. Performance-critical pattern: deterministic O(1)-per-
region "free" amortized over the entire region — useful for
hot loops and safety-critical workloads that need predictable
timing. **7 new lib tests**: region_new returns Region,
region_alloc returns *mut i64, mut-ref required for alloc,
both backends emit the bundle, helpers not leaked, scope-exit
drop emitted in C. **1611 lib + 54 parity green.**)

**Prior:** 2026-06-02 (**Layer 3.2 BoundedPtr<i64> shipped** —
v1 unsafe surface is now complete. `Type::BoundedPtr(Box<Type>)`
plus four builtins (`bptr_new`, `bptr_get`, `bptr_set`,
`bptr_len`). Fat pointer struct `{ data: *mut i64, len: u64,
capacity: u64 }`, Copy (it's a view, not an owner). The user
holds the underlying `unsafe_alloc` allocation separately and
wraps it in a BoundedPtr for the bounds-checked indexed-access
path. Out-of-bounds reads return `Option::None`; out-of-bounds
writes return `false` with no store performed. Smoke test (raw
alloc + bptr_new + 3 bptr_sets including one OOB at i=9999 +
3 bptr_gets) round-trips identically on both backends —
`v[oob] = -1` proves the bounds check fires. **6 new lib
tests** (signature/return-type checks, mut-ref-required
diagnostic, both backends emit the helper bundle, helpers
gated by program usage). **1604 lib + 54 parity green.**
**v1 unsafe.md surface: COMPLETE.** Only Layer 5 (regions)
remains — intentionally v2.)

**Prior:** 2026-06-02 (**Layer 3.1 canary instrumentation
shipped** — `unsafe_alloc` / `unsafe_free` now route through
helper functions that bracket each allocation with two i64
magic words. Layout per allocation: `[size][prefix-canary
0xDEADBEEFCAFEBABE][user N×i64][suffix-canary 0xBAADF00DDEADC0DE]`.
The returned user pointer is `base + 16 bytes` — the canary
words live outside the user's addressable range, so `raw_load`
/ `raw_store` paths see zero added cost. `intent_unsafe_free`
verifies both canaries before reclaiming; mismatch aborts with
a clear stderr message (`buffer underrun?` / `buffer overrun?`).
Per-allocation overhead: 24 bytes + ~4 cycles at free time —
well within embedded budgets. Bundle gated on program usage
(`program_uses_unsafe_alloc`); a trivial main without
unsafe_alloc emits no helper code. **3 new lib tests**
(C bundle present + magic constants emitted, LLVM bundle
present + signed-i64 forms of magic constants emitted, helper
not leaked into unused programs). **1598 lib + 54 parity
green.** Layer 3.1 fully complete — Layer 3.2 (BoundedPtr<T>)
remains.)

**Prior:** 2026-06-02 (**Layer 3.1 heap allocator shipped** —
`unsafe_alloc(n: i64) -> *mut i64` allocates `n` zero-initialized
slots via `calloc(n, sizeof(i64))`; `unsafe_free(p: *mut i64)`
releases via `free(p)` and returns 0 (dummy). Both builtins go
through the existing raw-pointer hosted gate
(`INTENT_TARGET_EMBEDDED=1`). Smoke test (`unsafe_alloc(8)` →
`raw_store(p, 42)` → `raw_load(p)` → `assert_safe` → `unsafe_free`)
round-trips cleanly on both backends. Also fixed the hosted gate
to cover function *return types* (was only checking params, fields,
and cast targets) — caught by a regression test that now passes.
**5 new lib tests** plus the hosted-gate-fixes-return-type test.
Canary instrumentation around the allocation (debug-mode magic
words) is the next Layer 3 sub-step. **1595 lib + 54 parity
green.**)

**Prior:** 2026-06-02 (**Layer 1.3 producers shipped** —
`raw_load(p)` / `raw_store(p, v)` builtins complete the Tainted<T>
discipline with a real production path. `raw_load(p: *const T |
*mut T) -> Tainted<T>` reads through a raw pointer; result is
wrapped in Tainted so the user must `assert_safe()` before flowing
into safe-typed slots. `raw_store(p: *mut T, v: T) -> i64` writes
through a mutable pointer (rejects `*const T` for const-
correctness). Tree-C emits `(*(p))` / `(*(p)) = v`; tree-LLVM
emits `load i64, i64* %p` / `store i64 %v, i64* %p`. Smoke tests:
`raw_load` round-trip via both backends prints `loaded: 42`;
`raw_store` followed by `raw_load` prints `stored+loaded: 99`.
Also fixed `c_type_name` to render `Type::Ptr` / `Type::PtrMut`
as `const T*` / `T*` in let-binding declarators (was emitting
the `/* *const T */` leaf placeholder). **5 new lib tests**:
raw_load returns Tainted, raw_load accepts *mut, raw_store
rejects *const, C deref emission, LLVM load emission. **1590
lib + 54 parity green.** Layer 1.3 functionally complete —
the `*p` parser sugar is a pure ergonomic follow-up; semantics
ship today via `raw_load` / `raw_store`.)

**Prior:** 2026-06-02 (**Layer 1.3 wrapper machinery
shipped** — `Type::Tainted<T>` plus the `taint(v)` / `assert_safe(t)`
builtin pair. Storing a `Tainted<T>` value into a `T`-typed slot is
rejected by the type checker — users have to wrap with `assert_safe()`
to vouch for the invariant. V1 restricts T to i64 (matches Pool /
Handle scope). The wrapper is purely a type-level discipline; both
backends emit identity codegen (Tainted shares its inner T's machine
representation). The canonical producer of tainted values — `*p`
raw-pointer deref — is queued for a follow-up commit; today's only
path is the explicit `taint(v)` bootstrap builtin. 5 new lib tests:
taint+strip round-trip, assert_safe lowers to inner type, Tainted
+ i64 arithmetic rejected, assert_safe on safe i64 rejected, end-to-
end LLVM compile. **1585 lib + 54 parity green.**)

**Prior:** 2026-06-02 (**Layer 2.2 codified** — Handle<T> as
the blessed escape from unsafe. No new code; falls out of Layers
1.1 + 1.2 + 2. Two acceptance tests pin the standing position:
(a) a function can freely return Handle<i64> across the boundary
(Copy, generationally-checked), (b) a function still cannot
return a raw pointer to a stack local (Layer 1.2's no-escape
dataflow stays in force). Future refactors can't accidentally
erode the Handle-only-escape rule without these tests
re-failing. **1580 lib + 54 parity green.** Layer 2 fully closed.)

**Prior:** 2026-06-02 (**Layer 2.1c shipped — generational
handles fully operational end-to-end.** Tree-C bundle emits
`intent_pool_i64` / `intent_handle_i64` structs + 5 helpers
(`pool_i64_new`, `pool_i64_alloc`, `pool_i64_get`, `pool_i64_free`,
`pool_i64_drop`) gated on program usage. Tree-LLVM bundle mirrors
the C in IR (`%intent_pool_i64 = type { i64*, i32*, i32*, i64,
i64, i64 }`; `%intent_handle_i64 = type { i32, i32 }`) with
hand-written IR for each operation. `Drop` emission frees the
three heap arrays at Pool scope exit; Handle is Copy and has no
Drop. Both backends route `pool_get` through the auto-registered
`Enum_Option__i64` so users get `Some(value) / None` semantics
(None = use-after-free / double-free caught at the generation
check). Smoke test (`/tmp/pool_test.vani` via `intentc run`)
confirms the round trip works: alloc → get returns Some(42) →
free → get on stale handle returns None → re-alloc reuses the
slot with a bumped generation. **3 new lib tests** (C bundle
emitted + LLVM bundle emitted + UAF scenario typechecks).
**1578 lib + 54 parity green.** Layer 2 of `unsafe.md`
complete — the load-bearing safety net is live.)

**Prior:** 2026-06-02 (**Layer 2.1b shipped** —
generational-handle builtins. `pool_new()` returns
`Pool<i64>` (affine, owned), `pool_alloc(mut ref p, v)` returns
`Handle<i64>`, `pool_get(ref p, h)` returns `Option<i64>`
(`None` on generation mismatch = UAF / double-free), `pool_free(mut
ref p, h)` bumps the generation and pushes the slot onto the
free list. Mutating ops require `mut ref Pool<i64>`; `pool_get`
accepts either `ref` or `mut ref`. `Option<i64>` auto-registered
in the monomorphization pre-pass so users don't need to write
the type. 8 new lib tests (each op's signature + the
`mut ref` requirement + the Handle-second-arg requirement).
**Codegen still missing** — `compile_to_c` / `compile_to_llvm`
emit nonfunctional output for any program touching Pool/Handle.
Layer 2.1c wires up the per-T C struct + helpers and the LLVM
counterparts. **1575 lib + 54 parity green.**)

**Prior:** 2026-06-02 (**Layer 2.1a scaffolding shipped** —
type system support for `Pool<T>` and `Handle<T>`. AST variants
`Type::Pool(Box<Type>)` (affine; owns slot array + generations +
free list) and `Type::Handle(Box<Type>)` (Copy; `(slot_idx: u32,
generation: u32)` opaque key — 8 bytes). Parser accepts `Pool<T>`
and `Handle<T>` in any type position. is_copy / bits / min / max
plus the 4 backend visitor sites updated. **Not yet usable** —
builtins (`pool_new` / `pool_alloc` / `pool_get` / `pool_free`)
land in Layer 2.1b; tree-C + tree-LLVM codegen bundles land in
2.1c. 2 new lib tests (Pool/Handle parses in signature, Handle
echo function compiles to typed IR). **1568 lib + 54 parity
green.**)

**Prior:** 2026-06-02 (**Layer 4.2 of `unsafe.md` shipped** —
ARM MTE flag. `apply_embedded_cc_hardening` now also conditionally
appends `-march=armv8.5-a+memtag` when both `INTENT_TARGET_EMBEDDED=1`
and `INTENT_TARGET_MTE=1` are set. Double-gated because the flag
rejects on non-ARM hosts (most CI is x86-64) — once the proper
`--target <triple>` flag ships, MTE becomes automatic for arm64
embedded targets and the env-var dance goes away. v1 Layers 4.1 +
4.2 + 1.1 + 1.2 + raw pointers all green. Remaining v1 work: Layer
2 (Handle<T> / Pool<T> generational handles — the load-bearing
safety net), Layer 1.3 (Tainted<T>), Layer 3.1+3.2 (canaries +
BoundedPtr). 1566 lib + 54 parity green.)

**Prior:** 2026-06-02 (**Layer 4.1 of `unsafe.md` shipped** —
toolchain-level stack canaries. `apply_embedded_cc_hardening` adds
`-fstack-protector-strong` to both `cc` invocations (tree-C `run`
path and LLVM-backend link path) when `INTENT_TARGET_EMBEDDED=1`.
Gated to embedded so hosted parity / perf baseline doesn't shift.
The `-strong` variant balances coverage vs perf (protects frames
with buffers / allocas / string ops at ~2 instructions per
protected frame). Layer 4.2 (ARM MTE) and Layer 1.3 (Tainted<T>)
remain queued. 1566 lib + 54 parity green.)

**Prior:** 2026-06-02 (**Layer 1.2 of `unsafe.md` shipped** —
no-escape analysis on `&local`-derived raw pointers. Two-pass
intra-procedural dataflow: (pass 1) collect the set of let-binding
names that hold raw pointers traced back to a stack-local;
(pass 2) walk the body rejecting `Return`, `IndexAssign value`,
and `FieldAssign value` whose value reads a tainted binding or
expresses a direct `(ref local) as *const T` / `(mut ref local)
as *mut T` cast. Pointers derived from function parameters are
**not** tainted (already from outside the frame), so pass-through
of caller-supplied pointers compiles. The diagnostic explicitly
points users at `Handle<T>` / `Pool<T>` (Layer 2) for long-lived
references. **7 new lib tests**: rejects return of local ptr
(direct + via-binding), allows pass-through of param ptr,
rejects store into heap-backed slot, allows local-only use in
same frame, rejects *mut return, diagnostic recommends Handle.
**1566 lib + 54 parity green.**)

**Prior:** 2026-06-02 (**Raw pointer types shipped** —
prerequisite for Layer 1.2's no-escape dataflow. `Type::Ptr(Box<Type>)`
(`*const T`) and `Type::PtrMut(Box<Type>)` (`*mut T`) added to the
type system. Parser accepts `*const T` / `*mut T` in any type
position; the mutability marker is mandatory. The reference-to-raw
casts `Ref(T) → Ptr(T)`, `RefMut(T) → PtrMut(T)`, and the downgrade
`RefMut(T) → Ptr(T)` are permitted; the upgrade `Ptr → PtrMut` is
rejected (the cast direction that *adds* mutation capability is a
meaningful safety downgrade and should be explicit at the source).
Tree-C emits `const T*` / `T*`; tree-LLVM emits `T*` (LLVM's type
system has no `const`). Raw pointers are Copy (machine words, 8
bytes on 64-bit). Same **embedded-only gate** as `unsafe(reason)`:
hosted builds reject raw pointer types in function signatures,
struct fields, or cast targets; `INTENT_TARGET_EMBEDDED=1` opts in.
**7 new lib tests** (hosted-rejection on signature + cast,
embedded-compile success for both `*const` and `*mut`, LLVM
emission, RefMut→Ptr downgrade allowed, Ptr→PtrMut upgrade
rejected). **1559 lib + 54 parity green.**)

**Prior:** 2026-06-02 (**Layer 1.1 of `unsafe.md` shipped** —
`unsafe(reason = "...") { ... }` block now parses, checks, and
emits machine-readable deviation metadata in both tree backends.
Parser-enforced rules: reason is mandatory, non-empty, ≤256
chars, ASCII-printable, no newlines (Devanagari alias
`असुरक्षित`). Tree-C emits `/* UNSAFE-DEVIATION: <reason> */`
comments; tree-LLVM emits `; UNSAFE-DEVIATION: <reason>` IR
comments — both ahead of the block body. The construct is gated
**default-deny on hosted targets** (matches the standing position
recorded in [README.md](README.md) → *Embedded targets — current
position*); opt in with `INTENT_TARGET_EMBEDDED=1` until a proper
`--target embedded` flag ships. Checker walks the body in a fresh
inner scope, so bindings declared inside don't leak. No raw-pointer
types yet — Layer 1.1 lays the metadata frame so Layers 1.2
(no-escape on `&local`), 1.3 (`Tainted<T>`), and 2 (`Handle<T>` /
`Pool<T>` generational handles) plug into a stable boundary. **9
new lib tests** (parser rejections × 4, hosted-default gate,
both-backends emission, Devanagari alias, scope isolation, env-var
serialization via static `Mutex` so parallel test runs are
deterministic). **1552 lib + 54 parity green.**)

**Prior:** 2026-06-02 (planning-docs landing — bounded
primitive surface is now closed at closure #604; all subsequent work
flows through two project-root plan docs: **[ARCS.md](ARCS.md)** (4
multi-session arcs broken into atomic sub-steps with time budgets +
acceptance tests; suggested order Arc 2 → Arc 1 → Arc 4.1 → Arc 3a →
rest) and **[unsafe.md](unsafe.md)** (5-layer embedded-safety plan;
v1 = generational handles, v2 = region typing for safety-critical
certification). **Keyword form locked at `unsafe(reason = "...") { ... }`
with parser-enforced non-empty reason clause** — chosen 2026-06-02
to make the justification part of the syntax (not a comment), so
ASIL-D / DO-178C / IEC 62304 deviation records can be extracted
straight from the IR/DWARF metadata. The two plan docs are
independent and interleavable. **No code changes this commit**;
closure count, lib test count, and parity remain at 1543 lib + 54
parity green from the Arc 0 landing. Prior log preserved below.)

**Prior:** 2026-06-02 (closures #597-#604 — **Arc 0**, the final 8 small one-shot primitives landed together as a single bounded effort. **i64 scalar:** `i64_parity(x)` popcount&1; `i64_mod_pos(x, m)` always-non-negative modulo with `abs(m)` (m==0 → 0); `i64_cube_root(x)` libm cbrt seed + fix-up loop. **f64 scalar:** `f64_pow_int(base, k)` repeated multiply (k<0 → 1/result; mixed-arg, special checker case to avoid the default-f64 coercion); `f64_round_to_multiple(x, m)` rounds x to nearest k*m (m≤0 → x unchanged); `f64_quadratic_root(a, b, c)` returns `(-b + sqrt(b²-4ac))/2a`, NaN on a==0 or negative discriminant. **Vec<i64>:** `vec_running_mean(xs)` cumulative integer average per index; `vec_intersperse(xs, sep)` inserts sep between elements (output length 2n-1). All cross-backend byte-identical with seeded determinism for the LLVM-backend `cbrt` path. **Arc 0 closes the bounded-primitive surface.** 1 new lib test. 1543 lib + 54 parity green.)
**Test totals:** 1665 lib + 54 end-to-end + 11 vtables-phase3 + 2 user-drop-by-ref + 1 ssa-examples tests passing; the cross-backend parity runner covers all 90 examples under `examples/`. (Win32 LLVM dispatch adds 4 host-gated tests that fire on Windows hosts only — futex/WaitOnAddress, CreateThread for tasks, plus the CreateThread fan-out parallel-for tests in tree-LLVM and SSA-LLVM.)

**Standing language decisions (carry across sessions):**
- **Affine ownership** is the v1 model. Every container, algorithm,
  or API proposed in [TODO.md](TODO.md) must be flagged for affine
  compliance — ✅ AFFINE / ⚠️ AFFINE-TENSION / 🛑 NON-COMPLIANT.
  Non-compliant items name their affine-friendly substitute.
- **Affine-friendly substitute patterns** (used throughout the
  data-structures roadmap):
  - Linked structures → **index-based arenas** (`Vec<Node>` +
    `i32` child indices, `-1` = none).
  - Shared ownership → `Channel<T, N>` (cross-task) or `Mutex<T>` /
    `Atomic<T>` (cross-thread shared state).
  - Map / set lookup → `get(m, ref k) -> Option<ref V>` (borrowed
    view); `remove(m, ref k) -> Option<V>` is the move-out path;
    `insert(k, v)` consumes both.
  - Iteration → `for x in xs` (by Copy-value for Copy T; by-ref for
    non-Copy T); combinators are by-ref or consume-whole-Vec via
    `.fold` / `.collect`.

### Data structures + algorithms roadmap (added 2026-05-27)

Levels 1–4 sequenced by dependency, all flagged for affine
compliance. Full plan + per-item contracts in [TODO.md](TODO.md)
under *Data structures + algorithms roadmap*.

- **Level 1** (no closures, no new types — ✅ AFFINE):
  Vec.sort / sort_by(fn), reverse / dedup, find / contains /
  binary_search, pop / swap_remove / insert / clear, Array.sort,
  String split / contains / trim / replace, parse_int / parse_float,
  math (pow / abs / sqrt / sin / cos / tan / floor / ceil),
  RNG (seed_rng / rand_i64), Hash interface + FNV-1a / SipHash.
- **Level 2** (depends on generic decls #281 — ✅ Copy /
  ⚠️ AFFINE-TENSION owning): HashSet, **HashMap (⚠️ —
  `get -> Option<ref V>`, `insert` consumes, `remove` moves)**,
  BTreeSet, BTreeMap, Deque (ring buffer), BinaryHeap.
- **Level 3** (closures multi-session — ✅ / ⚠️): closures with
  captured state, `.map / .filter / .fold` loop-fused, sort_by /
  find_by lifted to closure.
- **Level 4** (advanced, mostly arena-based — ✅ AFFINE): BST /
  AVL / red-black via node arena, B-tree, Trie, graphs as
  `Vec<Node>` + `Vec<Vec<u32>>` adjacency + algorithms (BFS, DFS,
  Dijkstra, A*, topo, Kruskal, Prim), Union-Find, skip list,
  Bloom filter.
- **Deferred / 🛑 NON-COMPLIANT** (flagged with reasoning +
  substitute): doubly-linked list with raw prev / next pointers
  (substitute: Deque + index-based BST); Rc / Arc shared ownership
  (substitute: index-based graphs + Channel + Mutex); iterators
  yielding owned T (substitute: by-ref iteration / `.fold` /
  `.collect`); Pin / self-referential structs (substitute: arena
  pattern); GC (any flavor — defeats no-runtime promise).

### Data-structures roadmap Level 3 — Closures with captured environment (shipped 2026-05-28, closure #314)

✅ AFFINE under the v1 capture-by-value-of-Copy contract. A
closure that references an outer Copy binding now compiles via
a checker-side lambda-lift transform — no new runtime types,
no new IR nodes, no backend changes.

**Surface:**
```vani
fn main() -> i64 {
  let n: i64 = 10;
  let factor: i64 = 3;
  let f = fn(x: i64) -> i64 { return double(x) * factor + n; };
  return f(5) + f(10);
}
```

**Transform (pre-checker pass in `lambda_lift_program`):**
For each `Stmt::Let { name: f, expr: AnonFn { .. } }` at the
top level of a function's body:

1. Compute free vars in the AnonFn body — names referenced
   that aren't in the closure's own params, not declared by
   `let` inside the body, not in the top-level name set
   (fns + consts + builtins).
2. Each free var must be in the enclosing fn's annotated-let
   or fn-param scope. The captured type is read from that
   scope.
3. Hoist `fn __anon_fn_<N>(__cap_<v1>: T1, ..., user_params...) -> R
   { body with captures renamed }` to `program.functions`.
4. DELETE the `let f = ...` statement entirely — `f` is
   compile-time only.
5. Build a per-function closure-handle map: `f → (hoisted_name,
   [capture var names])`.
6. Walk the function's remaining statements. For every
   `Call { name: f }` where `f` is a closure handle,
   rewrite to `Call { name: hoisted_name, args: [Var(cap)
   for each cap..., ...original args] }`. The captured
   vars are read at call time (not snapshotted at the
   closure-creation point).

After the rewrite, no reference to the closure handle
survives. The hoisted top-level fn is type-checked, lowered,
and called like any other top-level fn — both backends emit
identical code to a direct call.

**v1 restrictions:**
- The closure may only be CALLED in the same function. Passing
  it to another function, storing it in a struct, returning it,
  or reassigning it is NOT supported.
- Captures are read at call time, so reassigning the captured
  binding between closure creation and call is visible to the
  call. Faithful snapshot semantics are deferred.
- Captured bindings must be Copy (i64 / bool / f64 / etc.).
  Non-Copy captures (Vec / OwnedStr / HashMap / ...) are
  queued — they need move/clone semantics decisions.
- Captured bindings must have an explicit type annotation
  or be a fn param so the lift pass can read their types.
- ~~The Let-of-AnonFn pattern is only recognized at the
  function's top-level statement list (not inside nested
  `if` / `while` / block bodies in v1).~~ **Closed in
  closure #315.** Nested closures inside `if` / `while` /
  `for` / `ForIter` / `TaskSpawn` bodies are now supported;
  the lift pass recurses into nested blocks. Closure names
  must still be unique within the function (vāṇी's
  shadowing rules forbid same-scope reuse).

**Codegen:** None new. The rewritten calls are regular
`Call` nodes that lower identically to direct calls.

**Tests:** 5 new lib tests (multi-capture; multi-call to the
same closure; capture + top-level fn ref in body; no-capture
path still works; the closure_lift-now-compiles test that
supersedes #308's "captures rejected" assertion). New
`examples/closures.vani` exercises single/multi captures,
multi-call patterns, and closures inside a while loop on
both backends.

**Pending follow-ups:** non-Copy captures with move semantics
(needs affine analysis of the captured binding); capture-by-
ref second-class closures; passing closures across function
boundaries (needs closure-as-value type — likely a struct
holding the env + fn-ptr pair); reassigning a closure
binding; nested closure declarations.

### Data-structures roadmap Level 3 — Array method-call sugar (shipped 2026-05-28, closure #321)

✅ AFFINE — pure surface desugar, mirrors closures #311 / #320.

Method-call syntax now works on `[T; N]` Array receivers,
parallel to Vec:

| Source                | Desugared form                       |
|-----------------------|--------------------------------------|
| `arr.sort()`          | `sort(mut ref arr)`                  |
| `arr.sort_by(cmp)`    | `sort_by(mut ref arr, cmp)`          |
| `arr.reverse()`       | `reverse(mut ref arr)`               |
| `arr.find(v)`         | `find(ref arr, v)`                   |
| `arr.contains(v)`     | `contains(ref arr, v)`               |
| `arr.binary_search(v)`| `binary_search(ref arr, v)`          |

Same `Var` receiver restriction as #311. Iterator combinators
(`map` / `filter` / `fold` / `take` / `drop`) are NOT exposed
on arrays since the underlying builtins are Vec-only in v1.
That's a queued follow-up alongside non-i64 element types.

**Codegen:** none new. Same builtins as the function-call
form; the existing array dispatch in both backends handles
everything.

**Tests:** 3 new lib tests (sort + reverse pattern; contains
+ binary_search pattern; sort_by with anon-fn comparator).

**Pending follow-ups:** iterator combinator sugar on arrays
(map / filter / fold) — requires array-specific runtime
helpers; non-i64 array element types in combinators.

### Data-structures roadmap Level 3 — Vec mutator method-call sugar (shipped 2026-05-28, closure #320)

✅ AFFINE — pure surface desugar, mirrors #311's pattern.

Completes the uniform `xs.method()` surface across all Vec
operations. The Vec method-call arm in the checker now
recognizes the full set of in-place mutators and read-only
search builtins:

| Source                | Desugared form                       |
|-----------------------|--------------------------------------|
| `xs.push(v)`          | `push(mut ref xs, v)`                |
| `xs.pop()`            | `pop(mut ref xs)`                    |
| `xs.reverse()`        | `reverse(mut ref xs)`                |
| `xs.dedup()`          | `dedup(mut ref xs)`                  |
| `xs.swap_remove(i)`   | `swap_remove(mut ref xs, i)`         |
| `xs.insert(i, v)`     | `insert(mut ref xs, i, v)`           |
| `xs.clear()`          | `clear(mut ref xs)`                  |
| `xs.find(v)`          | `find(ref xs, v)`                    |
| `xs.contains(v)`      | `contains(ref xs, v)`                |
| `xs.binary_search(v)` | `binary_search(ref xs, v)`           |

Same receiver restriction as #311: receiver must be a simple
`Var` binding (after stripping one level of borrow). The
by-value `push(xs, v) -> Vec<T>` and `set(xs, i, v) -> Vec<T>`
forms (consuming forms) intentionally have no method sugar —
they require reassignment (`xs = xs.push(v)`) which is
awkward for the method-call pattern. Users keep using the
function-call form for those.

**Codegen:** none new. The desugared calls flow through the
existing builtin pipeline identically to direct calls.

**Tests:** 3 new lib tests (push/pop/reverse pattern;
contains/find pattern; swap_remove/insert/clear pattern).

**Pending follow-ups:** by-value `set` / `push` method sugar
via implicit reassignment; method sugar for Array-typed
receivers (today: Vec only).

### Data-structures roadmap Level 3 — Auto-fusion phase 2 (shipped 2026-05-28, closure #319)

✅ AFFINE — extension of closure #318's pure AST rewrite.

The fusion peephole now recognizes **all four** producer +
consumer combinations:

| Producer    | Consumer | Fused builtin           |
|-------------|----------|-------------------------|
| `vec_map`   | `vec_fold`   | `vec_map_fold`      |
| `vec_filter`| `vec_fold`   | `vec_filter_fold`   |
| `vec_map`   | `vec_filter` | `vec_map_filter`    |
| `vec_map_filter` | `vec_fold` | `vec_map_filter_fold` |

The last two rows enable **3-stage `map → filter → fold`
chains** to fuse iteratively: the pass first rewrites
`map+filter` into `map_filter`, then the next iteration sees
`map_filter+fold` and rewrites that into `map_filter_fold`.
Net result: a 3-stage chain in user source compiles to a
single tight loop with zero intermediate Vec allocations.

**Refactor:** the fusion dispatcher now uses `ProducerKind` /
`ConsumerKind` enums to classify the RHS shape of the
producer Let and the consumer call, then a small match on
the pair selects the fused name + arg order. Cleaner than
the closure #318 one-pattern-at-a-time approach.

Same surface guarantees as #318: handles both fn-call form
and method-call form (and mixed); recurses into nested
blocks; conservative use-count check (no fusion when the
intermediate is referenced elsewhere); works in return
position.

**Tests:** 3 new lib tests verify each new pattern fuses
(filter+fold, map+filter, 3-stage). The 3-stage test also
asserts that NEITHER intermediate `__map` nor `__filter`
call sites remain — the iterative fusion fully collapsed
the chain.

**Pending follow-ups:** chains across non-adjacent
statements (e.g. `map → print(m) → fold`); auto-fusion for
`vec_filter + vec_map` (filter then map — different from
`map + filter`); cross-fn fusion (when a closure-bound
helper consumes a Vec produced from the caller); type-
changing fused stages.

### Data-structures roadmap Level 3 — Transparent loop fusion (shipped 2026-05-28, closure #318)

✅ AFFINE — pure AST rewrite; no new types, no runtime cost.

A new pre-check pass `fuse_combinator_chains_in_program` walks
each function's body looking for two-statement patterns:

```vani
let m: Vec<i64> = vec_map(ref xs, f);   // or xs.map(f)
let t: i64 = vec_fold(ref m, init, g);  // or m.fold(init, g)
```

When `m` has NO other references in the function, the pair
collapses to a single fused call:

```vani
let t: i64 = vec_map_fold(ref xs, init, f, g);
```

The intermediate Vec allocation is eliminated transparently —
users get the fused-combinator perf benefit even when writing
the unfused source pattern.

**Recognized variants:**
- Function-call form: `vec_map(...)` + `vec_fold(ref m, ...)`.
- Method-call form: `xs.map(f)` + `m.fold(init, g)`.
- Mixed forms (one fn-call, one method-call).
- Return-position fold: `let m = ...; return vec_fold(ref m, ...);`
  fuses to `return vec_map_fold(...);`.

**Conservative behavior:** when `m` is referenced anywhere
else in the function (e.g. a second fold, or passed to another
fn), fusion is skipped — the unfused `__map` + `__fold` calls
remain, preserving semantics.

**Implementation:** AST-level peephole that runs after
`lambda_lift_program` (so closures-in-RHS positions are
already lifted) and before any type-checking. Recurses into
nested blocks (`if`/`while`/`for` bodies) so chains inside
control flow also fuse.

**Tests:** 4 lib tests verifying fusion happens (fn-call form,
method-call form, return position) and correctly DOES NOT
happen (intermediate used twice). The tests inspect emitted
LLVM directly to confirm the helper name.

**Pending follow-ups:** auto-fusion of more pattern shapes —
`vec_filter` + `vec_fold` → `vec_filter_fold`; `vec_map` +
`vec_filter` → `vec_map_filter`; 3-stage `map` + `filter` +
`fold` → `vec_map_filter_fold`; chains across more than two
adjacent statements (e.g. `map → use_for_print → fold` skipping
the print).

### Data-structures roadmap Level 3 — Fused combinator family (shipped 2026-05-28, closure #317)

✅ AFFINE — completes the fused family started in #316.

**API:**
- `vec_filter_fold(ref xs, init, p, g) -> i64` — single-pass
  reduce over matching elements only.
- `vec_map_filter(ref xs, f, p) -> Vec<i64>` — two-pass
  (count after mapping, allocate exact, fill). One Vec alloc
  instead of two.
- `vec_map_filter_fold(ref xs, init, f, p, g) -> i64` — full
  single-pass map → filter → fold pipeline, no intermediate
  Vec materializes.

Method sugar: `xs.filter_fold(init, p, g)`,
`xs.map_filter(f, p)`, `xs.map_filter_fold(init, f, p, g)`.

**Codegen:** all three emit straightforward helpers inside the
existing Vec bundle. Tree-C reuses the existing `__pred_fn` /
`__cmp_fn` / `__map_fn` typedefs (with `__pred_fn` forward-
declared earlier in the bundle so the fused combinators above
can reference it before `vec_filter`'s emission). Tree-LLVM
emits matching `define`s on `%intent_vec_i64` with the same
control-flow shape as #316's `__map_fold`.

**v1 restrictions:** Vec<i64> only; locked signatures as in
the earlier combinators.

**Tests:** 5 new lib tests (each builtin's basic compile +
LLVM; the chained-method-sugar test; the wrong-predicate
rejection). `examples/iter_combinators.vani` extended with
filter+fold (sum of evens), map+filter (doubled > 5), and
map+filter+fold (sum of doubled > 5) patterns.

**Pending follow-ups:** auto-detection of fold-of-map and
fold-of-filter chains in user source via a peephole IR pass
with use-count analysis; type-changing map+fold (`Vec<i64> ->
R`); non-i64 element types in combinators.

### Data-structures roadmap Level 3 — Fused map+fold combinator (shipped 2026-05-28, closure #316)

✅ AFFINE — input borrowed read-only; mapper + combiner are Copy
fn-ptr args. Returns i64 (the accumulator).

The opt-in fused form for the common pattern
`vec_fold(ref vec_map(ref xs, f), init, g)`:

```vani
let total = xs.map_fold(0,
                        fn(x: i64) -> i64 { return x * x; },
                        fn(a: i64, b: i64) -> i64 { return a + b; });
// equivalent to:
//   let m = xs.map(fn(x: i64) -> i64 { return x * x; });
//   let total = m.fold(0, add);
// but no intermediate Vec materializes.
```

**API:**
- `vec_map_fold(ref xs: Vec<i64>, init: i64,
                f: fn(i64) -> i64,
                g: fn(i64, i64) -> i64) -> i64`
- Method sugar: `xs.map_fold(init, f, g)`.

**Codegen:**
- **Tree-C**: `intent_vec_int64_t__map_fold` helper emitted inside
  the existing Vec bundle alongside `__map` / `__fold` / `__filter`.
  Single tight loop: `acc = g(acc, f(xs->data[i]))`.
- **Tree-LLVM**: matching `define` on `%intent_vec_i64` —
  alloca for `acc` initialized to `init`; loop loads element,
  calls `%f(v)`, calls `%g(acc, mapped)`, stores back.
- **SSA**: routes through tree backends.

**v1 restrictions:** Vec<i64> only; mapper signature locked to
`fn(i64) -> i64` (no type-changing); combiner returns i64.
Auto-detection of `vec_fold(ref vec_map(...))` chains in user
source (so the fusion happens without the user opting in) is
queued — needs a peephole IR pass with use-count analysis.

**Tests:** 4 lib tests (basics + LLVM compile; method sugar;
mapper signature mismatch; `__map_fold` helper appears in
emitted C). `examples/iter_combinators.vani` extended with
sum-of-squares and max-of-doubled patterns.

**Pending follow-ups:** `vec_filter_fold`, `vec_map_filter`,
`vec_map_filter_fold` — the rest of the fused combinator
family; type-changing fused map+fold (`Vec<i64> -> R`);
auto-detection of fold-of-map / fold-of-filter chains.

### Data-structures roadmap Level 3 — Eager slicing combinators + `.len()` (shipped 2026-05-28, closure #313)

✅ AFFINE — input Vec borrowed read-only; output Vec is a fresh
heap allocation the caller owns and drops.

**API (2 builtins + 3 method-sugar entries):**
- `vec_take(ref xs: Vec<i64>, n: i64) -> Vec<i64>` — first
  `min(n, len)` elements. Negative `n` clamps to 0.
- `vec_drop(ref xs: Vec<i64>, n: i64) -> Vec<i64>` — elements
  after the first `min(n, len)`. Negative `n` clamps to 0
  (returns the whole Vec).
- Method-call sugar: `xs.take(n)` → `vec_take(ref xs, n)`;
  `xs.drop(n)` → `vec_drop(ref xs, n)`; `xs.len()` → lowers to
  `ExprKind::Len { array: xs }` (the surface `len(xs)` builtin).

The `.len()` method sugar is a special case in the Vec
method-sugar arm — instead of synthesizing a Call to a
`vec_len` builtin (which doesn't exist), it produces
`ExprKind::Len` directly so the existing `Len` codegen
handles it. Net effect: a uniform `obj.len()` syntax across
Vec / HashMap / HashSet / BTreeMap / BTreeSet / Deque.

**Codegen:**
- **Tree-C**: `intent_vec_int64_t__take` / `__drop` helpers
  emitted inside the Vec bundle. Each does a malloc + memcpy
  of the slice; empty result returns `data=null, len=0,
  capacity=0`.
- **Tree-LLVM**: matching `define`s on `%intent_vec_i64`.
  Negative-n clamping via `icmp slt` + `select`; result
  Vec built via `insertvalue` triplet. memmove for the copy.
- **SSA**: routes through tree backends.

**v1 restrictions:** Vec<i64> only (per the rest of the
combinator family). No method sugar for `.take(n)` /
`.drop(n)` on other types yet.

**Tests:** 5 lib tests (vec_take + vec_drop type-check on
both backends; method sugar for take/drop; `xs.len()` sugar;
wrong-count-type rejection). `examples/iter_combinators.vani`
extended to demonstrate the new sugared forms.

**Pending follow-ups:** non-i64 element types; `vec_zip` (needs
`Vec<(i64, i64)>` — tuple Vec element); `.collect()` (only
makes sense once lazy iterators land alongside loop fusion);
`.len()` sugar for `[T; N]` arrays.

### Data-structures roadmap Level 3 — Affine container method-call sugar (shipped 2026-05-28, closure #312)

✅ AFFINE — pure surface desugar; same proven pattern as #311.

A second arm in `check_expr`'s `MethodCall` handler rewrites
method-call syntax on the five affine container types to their
existing builtin form:

| Container  | Sugar                  | Desugared form                          |
|------------|------------------------|-----------------------------------------|
| HashMap    | `m.get(k)`             | `hashmap_get(ref m, k)`                 |
|            | `m.insert(k, v)`       | `hashmap_insert(mut ref m, k, v)`       |
|            | `m.contains_key(k)`    | `hashmap_contains_key(ref m, k)`        |
|            | `m.len()`              | `hashmap_len(ref m)`                    |
| HashSet    | `s.insert(v)`          | `hashset_insert(mut ref s, v)`          |
|            | `s.contains(v)`        | `hashset_contains(ref s, v)`            |
|            | `s.len()`              | `hashset_len(ref s)`                    |
| BTreeMap   | `m.get(k)`             | `btreemap_get(ref m, k)`                |
|            | `m.insert(k, v)`       | `btreemap_insert(mut ref m, k, v)`      |
|            | `m.contains_key(k)`    | `btreemap_contains_key(ref m, k)`       |
|            | `m.remove(k)`          | `btreemap_remove(mut ref m, k)`         |
|            | `m.len()`              | `btreemap_len(ref m)`                   |
| BTreeSet   | `s.insert(v)`          | `btreeset_insert(mut ref s, v)`         |
|            | `s.contains(v)`        | `btreeset_contains(ref s, v)`           |
|            | `s.remove(v)`          | `btreeset_remove(mut ref s, v)`         |
|            | `s.len()`              | `btreeset_len(ref s)`                   |
| Deque      | `d.push_back(v)`       | `deque_push_back(mut ref d, v)`         |
|            | `d.push_front(v)`      | `deque_push_front(mut ref d, v)`        |
|            | `d.pop_back()`         | `deque_pop_back(mut ref d)`             |
|            | `d.pop_front()`        | `deque_pop_front(mut ref d)`            |
|            | `d.peek_back()`        | `deque_peek_back(ref d)`                |
|            | `d.peek_front()`       | `deque_peek_front(ref d)`               |
|            | `d.len()`              | `deque_len(ref d)`                      |

Same restriction as #311: the receiver must be a simple `Var`
binding (after stripping one level of borrow). Non-Var
receivers (`make_map().get(k)`) fall through to the existing
user-method-dispatch path.

**Parser change:** `len` is a reserved token (`TokenKind::Len`,
used for the unary `len(xs)` builtin). To allow `.len()` method
position, the parser now accepts `Len` as a method name after
`.` and maps it to the synthetic ident `"len"`. Both
`obj.len()` (MethodCall) and `obj.len` (FieldAccess) parse
correctly.

**Bug fix bundled:** the SSA-LLVM backend reuses tree-LLVM's
`emit_vec_helpers`, which gates `heap_peek` / `heap_pop`
emission on `LLVM_ENUM_PAYLOAD_REGISTRY`. The registry was
cleared at the start of tree-LLVM's `emit_llvm` but NOT at the
start of SSA-LLVM's `emit`. When a process compiled
container_method_sugar.vani (which registers `Option__i64`)
followed by control_flow.vani (which goes through SSA-LLVM
because it has no payloaded enums), the SSA emit would pick
up the stale `Option__i64` registration and emit
`%Enum_Option__i64`-referencing helpers without the
corresponding typedef. The fix: SSA-LLVM's `emit` now clears
the enum payload, variant-payload, and tag registries at the
top alongside its own thread_locals.

**Tests:** 6 new lib tests (one per container + fall-through
behaviour). New `examples/container_method_sugar.vani` covers
all five containers' sugared methods on both backends.

**Pending follow-ups:** method-call sugar for Vec mutators
(push / pop / reverse / dedup / find / contains /
binary_search); non-Var receivers via implicit temporary
binding; uniform `.len()` method on Vec (which currently uses
the standalone `len(xs)` builtin form).

### Data-structures roadmap Level 3 — Vec method-call sugar (shipped 2026-05-28, closure #311)

✅ AFFINE — pure surface-level desugar; no new runtime cost.

A checker-only desugar pass converts method-call syntax on
`Vec<T>` receivers to the equivalent builtin call:

| Source                       | Desugared form                       |
|------------------------------|--------------------------------------|
| `xs.map(f)`                  | `vec_map(ref xs, f)`                 |
| `xs.filter(p)`               | `vec_filter(ref xs, p)`              |
| `xs.fold(init, g)`           | `vec_fold(ref xs, init, g)`          |
| `xs.sort()`                  | `sort(mut ref xs)`                   |
| `xs.sort_by(cmp)`            | `sort_by(mut ref xs, cmp)`           |

**Restrictions:** the receiver must be a simple `Var` binding;
a non-Var receiver (`f().map(...)`) keeps falling through to
the existing user-method-dispatch path. The receiver's
declared type must be `Vec<T>` (or borrowed `ref` / `mut ref`).
Method-name matches fail-soft — if `xs.foo(...)` doesn't match
any Vec builtin, the existing dispatch continues looking for a
user-declared `methods on Vec` method (which, in v1, doesn't
exist for Vec; the user gets the standard "no such method"
diagnostic).

**Implementation:** new arm in `check_expr`'s `MethodCall`
handler that synthesizes a `Call` expression with a leading
`ref` / `mut ref` of the receiver and re-enters `check_call`,
where the existing builtin validators handle everything from
arity to signature checking to codegen.

**Codegen:** none — the desugared call goes through the existing
builtin pipeline. Both backends emit identically to direct
calls.

**Tests:** 5 lib tests (map / filter / fold / sort_by / chained
through named intermediates). `examples/iter_combinators.vani`
extended to demonstrate the sugared API alongside the existing
direct-call examples.

**Pending follow-ups:** method-call sugar for other Vec
builtins (push / pop / reverse / dedup / find / contains /
binary_search); non-Var receivers via implicit temporary
binding; method-call sugar for the affine container types
(HashMap, BTreeMap, etc.); loop fusion across chains.

### Data-structures roadmap Level 3 — Iterator combinators on Vec<i64> (shipped 2026-05-28, closures #309 + #310)

✅ AFFINE — fn-ptr args are Copy; the input Vec is borrowed
read-only (`ref Vec<i64>`); `vec_map` / `vec_filter` return
owned Vecs the caller is responsible for dropping.

**API (3 builtins):**
- `vec_map(ref xs: Vec<i64>, f: fn(i64) -> i64) -> Vec<i64>` —
  eager. Allocates a fresh result Vec.
- `vec_filter(ref xs: Vec<i64>,
              p: fn(i64) -> bool) -> Vec<i64>` — eager. Two-
  pass: count matches, allocate exact, fill. Closure #310.
- `vec_fold(ref xs: Vec<i64>, init: i64,
           g: fn(i64, i64) -> i64) -> i64` — reduces with the
  user-supplied combiner.

Both pair with anonymous fn expressions from closure #308:
```vani
let doubled = vec_map(ref xs, fn(x: i64) -> i64 { return x + x; });
let sum     = vec_fold(ref xs, 0, fn(a: i64, b: i64) -> i64 { return a + b; });
```
or top-level fn-refs, since the `fn(...)` argument type is
satisfied by either.

**Codegen:**
- **Tree-C**: helpers `intent_vec_int64_t__map` /
  `intent_vec_int64_t__filter` / `intent_vec_int64_t__fold`
  emitted inside the existing Vec bundle for i64. map mallocs
  a result buffer the size of the input; filter is two-pass
  (count, allocate, fill); fold accumulates in a register.
  Reuses the existing `__cmp_fn` typedef for fold's combiner;
  adds a `__pred_fn` typedef for filter's predicate.
- **Tree-LLVM**: matching `define`s for `__map` / `__filter` /
  `__fold` on `%intent_vec_i64`. map / filter call `@malloc`;
  fold uses an alloca + i-counter loop. filter follows the
  same two-pass shape as C.
- **SSA**: routes through tree backends via the
  `ssa_path_supports` reject list.

**v1 restrictions:**
- Vec<i64> only. `Vec<f64>` / `Vec<Str>` deferred — needs
  per-element-type helpers.
- Mapper signature locked to `fn(i64) -> i64` (no type-changing
  map yet — would require generic monomorphization over the
  output element type).
- No loop fusion. A chain `vec_fold(ref m, 0, g)` where
  `m = vec_map(ref xs, f)` materializes an intermediate Vec
  in `m`. The user must introduce a named let between stages —
  `ref vec_map(...)` is rejected (`ref` borrows named places).
  Fusion at monomorphization time queued as a follow-up.
- `vec_take`, `vec_zip`, `.collect()` deferred.

**Tests:** 10 lib tests across closures #309 and #310 — basics
+ LLVM compile for each builtin; inline anon-fn use; signature-
mismatch rejection per builtin; `__map` / `__filter` helpers
appear in emitted C. `examples/iter_combinators.vani`
exercises both backends via the parity runner with top-level
fn-refs AND inline anon fns AND a map → filter → fold pipeline.

**Pending follow-ups:** loop fusion at monomorphization time;
`vec_take` / `vec_zip` / `.collect()`; non-i64 element types;
type-changing map (`Vec<i64> -> Vec<Str>`); method-call sugar
(`xs.map(f)` / `xs.filter(p)` / `xs.fold(init, g)`).

### Data-structures roadmap Level 3 — Anonymous fn expressions (shipped 2026-05-28, closure #308)

✅ AFFINE in v1 (no captured environment, so no aliasing of
outer bindings is possible). Foundation for Level 3 — the
follow-up closure adds captured environments (capture-by-
value moves; capture-by-ref produces second-class closures).

**Syntax:**
```vani
let f: fn(i64) -> i64 = fn(x: i64) -> i64 { return x + x; };
let result = apply(fn(x: i64) -> i64 { return x * 3; }, 7);
sort_by(mut ref xs, fn(a: i64, b: i64) -> i64 { return a - b; });
```

The body is parsed identically to a top-level fn body: `{ stmt*
return EXPR; }`. The unit-return shorthand `fn(x: i64) { print
x; }` is accepted (auto-appends `return 0`).

**Strategy — lambda-lift:** A new pre-checker pass
`lambda_lift_program` walks each top-level fn's body / requires
/ ensures and replaces every `fn(...) -> R { body }` expression
with a `Var(name)` referencing a freshly generated top-level
fn `__anon_fn_<N>`. The lifted fn is appended to
`program.functions`, so signature collection and the existing
fn-pointer infrastructure (FnRef → CallIndirect, both backends'
indirect-call lowerings) handle everything downstream with zero
new codegen.

**Codegen:** None new. Both backends already emitted lowerings
for `TypedExprKind::FnRef` and `TypedExprKind::CallIndirect`
(closure #213). The hoisted fn appears identical to a user-
written top-level fn.

**v1 restrictions:**
- No captured environment. The body is type-checked in a fresh
  scope containing only its own params + top-level fns +
  builtins; references to outer let-bound vars surface as the
  existing `unknown variable 'X'` diagnostic.
- Statement-style body only. Expression-body sugar (`fn(x) =>
  x + x`) is deferred.
- Generic anon fns deferred (would require monomorphization
  to see the typed call site).

**Tests:** 6 lib tests (let-binding; inline call argument;
outer-capture rejection; signature-mismatch rejection;
`sort_by` comparator; `__anon_fn_0` appears in emitted C).
New `examples/anon_fn.vani` exercises both backends via the
parity runner.

**Pending follow-ups:** captured environment (next closure);
expression-body shorthand; generic anon fns.

### Data-structures roadmap Level 2 — BTreeMap<i64, i64> (shipped 2026-05-28, closure #307)

✅ AFFINE under v1 Copy-V scoping. ⚠️ AFFINE-TENSION when V
goes non-Copy — `get` shifts from `Option<V>` (by-value Copy)
to `Option<ref V>` (borrowed view). v1 closes the Level 2
ladder; node-arena B-tree lands at Level 4.

**Layout:** `{ keys: i64*, values: i64*, len: u64, capacity:
u64 }`. Parallel sorted arrays; binary-search `lower_bound`
finds the slot. Insert/remove memmove both keys + values
tails in lockstep. Grow doubles capacity (start 4) via
`realloc`.

**API (6 builtins):**
- `btreemap_new() -> BTreeMap<i64, i64>`
- `btreemap_insert(mut ref m, k, v) -> Option<i64>` — Some(prev)
  on overwrite, None on first insert
- `btreemap_get(ref m, k) -> Option<i64>` — Some(v) on hit, None
  on miss
- `btreemap_contains_key(ref m, k) -> bool`
- `btreemap_remove(mut ref m, k) -> Option<i64>` — Some(prev)
  on removal, None when key was absent
- `btreemap_len(ref m) -> i64`

**Codegen:**
- **Tree-C**: `intent_btreemap_i64_i64` struct + helpers in
  body via `program_uses_i64_i64_btreemap` walker. get /
  insert / remove gated on Option__i64 registry. memmove
  shifts keys + values in lockstep.
- **Tree-LLVM**: `%intent_btreemap_i64_i64 = type { i64*, i64*,
  i64, i64 }` preamble typedef. Module-level `define`s for
  new / drop / __lower_bound (internal) / contains_key / len /
  get / insert / remove. Insert path: lower_bound → equal-key
  update short-circuit → grow if full (realloc keys + values)
  → memmove tails → store.
- **SSA**: routes through tree backends via the
  `ssa_path_supports` reject list.
- **Drop**: both backends free `keys` and `values` at scope
  exit.

**v1 restrictions:**
- (K, V) = (i64, i64) only. Wider K/V (`BTreeMap<Str, V>`,
  `BTreeMap<i64, OwnedStr>`) deferred.
- Sorted-Vec backing (O(log n) lookup, O(n) insert/remove).
  Real B-tree node arena queued for Level 4.
- No range / iteration builtin yet (lands with Level 3
  closures; will reuse the natural sorted order).

**Tests:** 6 lib tests (basics + LLVM compile; insert returns
prev via Option<i64>; insert rejects ref (non-mut); `BTreeMap`
name reserved against user structs; C runtime emitted; LLVM
typedef + insert/remove defines emitted). `examples/btreemap.vani`
covers insert returning prev (None then Some on overwrite),
get hit/miss, contains_key, bulk insert triggering grow,
remove of present-vs-absent keys, and a drain loop.
Cross-backend parity green.

**Pending follow-ups:** node-arena B-tree (Level 4); range
queries via Level 3 closures; non-i64 K/V via user `Ord` /
non-Copy V via `Option<ref V>` (the AFFINE-TENSION shift);
iteration via closures.

### Data-structures roadmap Level 2 — BTreeSet<i64> (shipped 2026-05-28, closure #306)

✅ AFFINE — new affine handle type `BTreeSet<T>` (v1 i64 only).
Ordered set backed by a sorted heap-allocated `i64*` buffer; v1
keeps the API minimal so we can layer in arena-backed B-tree
nodes at Level 4 without re-touching call sites.

**Layout:** `{ keys: i64*, len: u64, capacity: u64 }`. Keys held
in ascending order; binary-search `lower_bound` finds the slot;
inserts memmove the tail right; removes memmove the tail left.
Grow doubles capacity (start 4) via `realloc`.

**API (5 builtins):**
- `btreeset_new() -> BTreeSet<i64>` — empty
- `btreeset_insert(mut ref s, k) -> bool` — true iff newly
  inserted; duplicates return false
- `btreeset_contains(ref s, k) -> bool`
- `btreeset_remove(mut ref s, k) -> bool` — true iff was present
- `btreeset_len(ref s) -> i64`

**Codegen:**
- **Tree-C**: `intent_btreeset_i64` struct + helpers in body via
  `program_uses_i64_btreeset` walker. C-level `lower_bound` is
  a static inline using `<` on i64.
- **Tree-LLVM**: `%intent_btreeset_i64 = type { i64*, i64, i64 }`
  preamble typedef + module-level `define`s for new / drop /
  __lower_bound (internal) / contains / len / insert / remove.
  Insert path: lower_bound → equal-key short-circuit → grow if
  full → memmove tail → store. Remove path: lower_bound →
  equal-key check → memmove tail → decrement len.
- **SSA**: routes through tree backends via the `ssa_path_supports`
  reject list.
- **Drop**: both backends free `keys` at scope exit.

**v1 restrictions:**
- T = i64 only. Wider widths (`BTreeSet<Str>`,
  `BTreeSet<MyEnum>` via a user `Ord` trait) deferred.
- Sorted-Vec backing (O(log n) lookup, O(n) insert/remove).
  Real B-tree node arena queued for Level 4.
- No range / iteration builtin yet (lands with Level 3 closures
  alongside `for k in btreeset` and `btreeset_range`).

**Tests:** 6 lib tests (basics + LLVM compile; insert rejects
ref (non-mut); remove rejects ref (non-mut); `BTreeSet` name
reserved against user structs; C runtime emitted; LLVM typedef
+ helpers emitted). `examples/btreeset.vani` covers first-time
inserts vs duplicates, contains hit/miss, bulk insert triggering
grow, remove of present-vs-absent keys, and a drain loop.
Cross-backend parity green.

**Pending follow-ups:** node-arena B-tree (Level 4); range
queries; non-i64 keys via user `Ord`; iteration via closures
(Level 3).

### Data-structures roadmap Level 2 — HashMap<i64, i64> (shipped 2026-05-28, closure #305)

✅ AFFINE under v1 Copy-V scoping. ⚠️ AFFINE-TENSION when V
goes non-Copy — that's the v2 expansion where `get` must
shift from `Option<V>` (by-value Copy) to `Option<ref V>`
(borrowed view, map retains ownership).

**Layout:** `{ keys: i64*, values: i64*, occ: u8*, len: u64,
capacity: u64, tombstones: u64 }` (6 fields, tombstones added
in closure #343). Parallel arrays for cache-friendly probing.
Open-addressing linear probing; grow doubles capacity when
`(len + tombstones) * 2 >= capacity` and rehashes (which
clears tombstones to 0). Same FNV-1a hash function as
`hash_i64`. `occ` is tri-state: 0=empty, 1=occupied,
2=tombstone.

**API (6 builtins):**
- `hashmap_new() -> HashMap<i64, i64>`
- `hashmap_insert(mut ref m, k, v) -> Option<i64>` — returns
  the previous value at key k (Some), or None if the key was
  not present. First-tombstone-or-empty placement reuses
  vacated slots.
- `hashmap_get(ref m, k) -> Option<i64>` — Some(v) on hit,
  None on miss. Probes past tombstones.
- `hashmap_contains_key(ref m, k) -> bool` — probes past
  tombstones.
- `hashmap_remove(mut ref m, k) -> Option<i64>` — closure
  #343. Tombstone-aware: marks the slot 2, --len, ++tombstones.
  Returns Some(prev_value) if removed, None if absent.
- `hashmap_len(ref m) -> i64`

**Codegen:**
- **Tree-C**: `intent_hashmap_i64_i64` struct + helpers in
  body via `program_uses_i64_i64_hashmap` walker. get /
  insert / remove gated on Option__i64 registry.
- **Tree-LLVM**: `%intent_hashmap_i64_i64 = type { i64*, i64*,
  i8*, i64, i64, i64 }` typedef (6 fields after #343).
  Module-level `define`s; linear probing via alloca +
  capacity mask.
- **SSA**: routes through tree.
- **Drop**: both backends free keys, values, occ at scope
  exit.

**v1 restrictions:**
- (K, V) = (i64, i64) only.
- No iteration builtin yet.

**Tests:** 6 lib tests (basics + LLVM compile; insert returns
prev via Option<i64>; insert rejects ref (non-mut); `HashMap`
name reserved against user structs; C runtime emitted; LLVM
typedef + insert define emitted). `examples/hashmap.vani`
covers insert returning prev (None first, then Some on
overwrite), get hit/miss, contains_key, bulk insert
triggering grow + rehash. Cross-backend parity green.

**Pending follow-ups:** non-Copy V via `Option<ref V>` (the
AFFINE-TENSION shift); wider K widths (`HashMap<Str, V>`,
`HashMap<i64, Str>`); user `Hash` trait for struct keys;
iteration via Level 3 closures. `hashmap_remove` shipped in
closure #343 (tombstone-based, mirrors hashset_remove).

### Data-structures roadmap Level 2 — HashSet<i64> (shipped 2026-05-28, closure #304)

✅ AFFINE — new affine handle type `HashSet<T>` (v1 i64 only).
Heap-backed open-addressing hash set with linear probing.

**Layout:** `{ keys: i64*, occ: u8*, len: u64, capacity: u64 }`.
Empty(0) / occupied(1) slot tags via parallel u8 array. Grow
doubles capacity at 50% load factor and rehashes. Hash via
inlined FNV-1a (matches the `hash_i64` builtin's algorithm so
external `hash_i64(k)` and internal bucket selection see the
same values).

**API (4 builtins):**
- `hashset_new() -> HashSet<i64>` — empty
- `hashset_insert(mut ref s, v: i64) -> bool` — true if newly
  inserted, false if already present
- `hashset_contains(ref s, v: i64) -> bool`
- `hashset_len(ref s) -> i64`

**Codegen:**
- **Tree-C**: `intent_hashset_i64` struct + 7 helpers (inc.
  internal `__hash_key`, `__insert_into`, `__grow`). Emitted
  in body via the new `program_uses_i64_hashset` walker.
- **Tree-LLVM**: `%intent_hashset_i64 = type { i64*, i8*, i64,
  i64 }` typedef + module-level `define`s. Linear probing via
  alloca-based index + capacity mask (capacity is always a
  power of two from the grow doubling).
- **SSA**: routes through tree.
- **Drop**: both backends call `intent_hashset_i64_drop` at
  scope exit, freeing both the keys and occ buffers.

**v1 restrictions:**
- i64 element only.
- `hashset_remove` deferred (needs tombstone-and-rebuild or
  backshift-deletion logic; queued as a follow-up).
- No iteration builtin yet — will arrive with Level 3 closures.

**Tests:** 5 lib tests (basics + LLVM compile; rejects non-mut-
ref insert; `HashSet` name reserved against user structs; C
runtime emitted; LLVM typedef + helpers emitted).
`examples/hashset.vani` covers insert/contains/len, duplicate
rejection, bulk insert triggering grow + rehash, and
duplicate counting across overlapping ranges. Cross-backend
parity green.

**Pending follow-ups:** `hashset_remove`; wider element widths
(`HashSet<Str>` next, then arbitrary `Hash + Eq` user types);
iteration via Level 3 closures.

### Data-structures roadmap Level 2 — Deque<i64> (shipped 2026-05-28, closure #303)

✅ AFFINE — new affine handle type `Deque<T>` (v1 i64 only).
Heap-backed ring buffer; scope-exit Drop frees the data
buffer.

**Layout (both backends):** `{ data: i64*, front: u64, len:
u64, capacity: u64 }`. Mod-capacity arithmetic implements
ring wrap-around. Grow doubles capacity and unwraps the ring
so subsequent ops see a contiguous prefix.

**API (8 builtins):**
- `deque_new() -> Deque<i64>` — empty (zero capacity)
- `deque_push_back(mut ref d, v: i64) -> i64` — new len
- `deque_push_front(mut ref d, v: i64) -> i64` — new len
- `deque_pop_back(mut ref d) -> Option<i64>` — Some / None
- `deque_pop_front(mut ref d) -> Option<i64>` — Some / None
- `deque_peek_back(ref d) -> Option<i64>` — Some / None
- `deque_peek_front(ref d) -> Option<i64>` — Some / None
- `deque_len(ref d) -> i64`

**Codegen:**
- **Tree-C**: `typedef struct { … } intent_deque_i64` +
  helpers emitted in body (so the `Enum_Option__i64` typedef
  is in scope when pop / peek helpers are defined). pop /
  peek further gated on Option__i64 registry.
- **Tree-LLVM**: `%intent_deque_i64 = type { i64*, i64, i64,
  i64 }` declared in the preamble; `define` helpers gated via
  the new `program_uses_i64_deque` walker.
- **SSA**: routes through tree.
- **Drop**: both backends call `intent_deque_i64_drop` at
  scope exit, which frees the data buffer.

**v1 restrictions:**
- i64 element only (extending to wider widths needs
  parameterized runtime helpers).
- `deque_clear` not yet shipped (use `pop_front` in a loop
  for now).

**Tests:** 7 lib tests (basics + pop/peek typecheck; pop with
mut-ref dispatch; non-i64 element rejected; `Deque` name
reserved against user structs; C runtime emitted; LLVM
typedef + helpers emitted). New `examples/deque.vani` covers
mixed-end pushes, FIFO drain, LIFO from back, empty pop, and
ring-stress test triggering capacity grow at both ends.
Cross-backend parity green.

**Pending follow-ups:** wider element widths; `deque_clear`
sugar; index access (`deque_get(ref d, i)`); iteration via
closures (Level 3).

### Data-structures roadmap Level 2 — BinaryHeap on Vec (shipped 2026-05-28, closure #302)

✅ AFFINE — v1 design: a `Vec<i64>` plus four builtins gives a
min-heap priority queue. A dedicated `BinaryHeap<T>` wrapper
type stays queued as a v2 ergonomic layer.

**API:**
- `heap_push(mut ref xs: Vec<i64>, v: i64) -> i64` — sift-up;
  returns the new length.
- `heap_pop(mut ref xs: Vec<i64>) -> Option<i64>` — extract
  min + sift-down (or None on empty).
- `heap_peek(ref xs: Vec<i64>) -> Option<i64>` — view min
  without modifying.
- `heapify(mut ref xs: Vec<i64>) -> i64` — Floyd's O(n)
  bottom-up algorithm; transforms an arbitrary Vec into a
  valid min-heap.

**Codegen:**
- **Tree-C**: helpers emitted in `emit_vec_bundle` alongside
  sort/dedup (i64-gated). `heap_pop` / `heap_peek` further
  gated on Option__i64 in the enum-payload registry
  (forward-reference protection).
- **Tree-LLVM**: module-level `@intent_vec_i64__heap_*` defs
  with internal `__heap_sift_up` / `__heap_sift_down` helpers.
  Same Option__i64 gate.
- **SSA**: routes through tree via `ssa_path_supports`.

**v1 restrictions:**
- Min-heap only (max-heap via inversion: push `-v`, pop and
  negate; or use sort + reverse).
- i64 element only.
- Dedicated `BinaryHeap<T>` wrapper type deferred.

**Tests:** 6 lib tests (push + pop + peek typecheck; heapify
typecheck; rejects non-mut-ref; rejects non-i64 element; C
emits the heap runtime; LLVM emits the helper defines). New
`examples/heap.vani` covers push-then-drain, peek, empty pop,
heapify-then-drain. Cross-backend parity green.

### Data-structures roadmap Level 1 — Hash builtins (shipped 2026-05-28, closure #301)

✅ AFFINE — pure, deterministic, no heap.

**API:**
- `hash_i64(x: i64) -> u64` — FNV-1a unrolled over 8 bytes.
- `hash_str(s: Str) -> u64` — FNV-1a until NUL.
- `hash_combine(a: u64, b: u64) -> u64` — boost::hash_combine
  with the golden-ratio constant 0x9e3779b97f4a7c15. Useful for
  composite keys (tuples, structs) once Level 2 lands.

**Constants:** FNV-1a offset basis 0xcbf29ce484222325, prime
0x100000001b3 — standard. Cross-backend output is byte-for-byte
identical for the same input.

**Codegen:**
- **Tree-C**: 3 static helpers emitted in the preamble when
  any hash builtin is referenced (body-substring gate).
- **Tree-LLVM**: 3 module-level `define i64 @intent_hash_*`
  blocks emitted via the new `program_uses_hash` walker over
  the typed IR.
- **SSA**: routes through tree via `ssa_path_supports`.

**Tests:** 6 lib tests (typecheck + LLVM compile; determinism;
hash_str rejects non-Str; hash_combine rejects wrong arity; C
runtime present; LLVM define present). `examples/hash.vani`
covers determinism, distinctness, combine order-sensitivity,
empty string. Cross-backend parity green.

**Pending follow-ups:** SipHash (adversarial-resistant);
hash_f64; hash interface (`Hash` trait) for user-defined
struct keys. These unlock the full `HashSet<T>` / `HashMap<K,
V>` Level 2 design — the FNV-1a builtins ship today as the
v1 primitive.

**🎉 Level 1 of the data-structures roadmap is COMPLETE.**
Shipped closures: #293 (Vec.sort + sort_by) · #294 (Vec.reverse
+ Vec.dedup) · #295 (Vec.find / contains / binary_search) ·
#296 (Vec.swap_remove / insert / clear) · #297 (Array ops on
`[i64; N]`) · #298 (str_contains / starts_with / ends_with +
parse_int / parse_float) · #299 (math: pow / sqrt / trig /
floor / ceil + overloaded abs) · #300 (RNG) · #301 (hash).
Level 2 begins next: HashSet / HashMap (⚠️ AFFINE-TENSION) /
BTreeSet / BTreeMap / Deque / BinaryHeap.

### Data-structures roadmap Level 1 — RNG (shipped 2026-05-28, closure #300)

✅ AFFINE — thread-local xorshift64 PRNG state.

**API:**
- `seed_rng(seed: u64) -> i64` — sets the thread-local state.
  Seed 0 falls back to a fixed nonzero default (xorshift's
  zero-state trap).
- `rand_i64() -> i64` — returns the next full-width signed
  value. Cast to u64 / smaller widths at the call site if
  needed.
- `rand_in_range(lo: i64, hi: i64) -> i64` — uniform integer
  in `[lo, hi)`. `lo >= hi` returns `lo` as a safe clamp.

**Codegen:**
- **Tree-C**: `_Thread_local uint64_t intent_rng_state` global
  + 3 static helpers. Gated on the program actually calling
  any RNG builtin via the body substring check.
- **Tree-LLVM**: `@intent_rng_state = thread_local global i64`
  + outlined `define i64 @intent_rng_seed(...)` / `_next` /
  `_in_range` helpers emitted once at module scope. Same
  body-walk gate.
- **SSA**: routes through tree via `ssa_path_supports`.

**Determinism:** same seed + same call sequence ⇒ identical
output. Cross-backend parity passes byte-for-byte on
`examples/rng.vani`.

**Thread-locality:** each `task` body inherits its own
independent stream (no cross-task interference). A separate
RNG instance per `Mutex<u64>` is a possible v2 layer if users
want shared seedable state across threads.

**Tests:** 6 lib tests (typecheck + LLVM compile; seed_rng
type check; arity rejection; C thread-local emit; LLVM
thread_local global + helpers emit; determinism stub).
`examples/rng.vani` exercises die rolls, coin flips, reseed
reproducibility. Cross-backend parity green.

### Data-structures roadmap Level 1 — Math ops (shipped 2026-05-28, closure #299)

✅ AFFINE — all scalar pure, libm-backed.

**API:**
- `pow(base: f64, exp: f64) -> f64`
- `sqrt(x: f64) -> f64`
- `sin(x: f64) -> f64` / `cos(x: f64) -> f64` / `tan(x: f64) -> f64`
- `floor(x: f64) -> f64` / `ceil(x: f64) -> f64`
- `abs(x: i64) -> i64` (signed int → llabs)
- `abs(x: f64) -> f64` (float → fabs) — overloaded on arg type

**Codegen:**
- **Tree-C**: `<math.h>` added to the preamble; call-sites emit
  the libm symbol directly (`sqrt(x)`, `pow(b, e)`, `llabs(x)`,
  `fabs(x)`).
- **Tree-LLVM**: `declare double @sqrt(double)` etc. emitted in
  the preamble; call-sites emit `call double @sqrt(...)`.
- **Linker**: `-lm` added to the cc args on POSIX (libm holds
  the math symbols on glibc / macOS / BSD). Windows ships
  them in the C runtime — no extra flag.
- **SSA**: routes through tree via `ssa_path_supports` gate.

**Example rename:** `abs` is now a builtin, so the existing
`examples/ffi.vani` (which used `extern "C" fn abs`) was
renamed to use `atoi` / `atoll`; sort.vani's helper renamed to
`my_abs`; sanskrit_keywords.vani's helper likewise. Six lib
tests covering extern-`abs` were updated to use `atoi`.

**Tests:** 6 lib tests (pow/sqrt/sin compile; abs overload;
floor + ceil; rejects wrong arity; C libm symbols emitted;
LLVM libm declares emitted). New `examples/math_ops.vani`
covers each builtin + a composed distance(x, y) example.
Cross-backend parity green.

### Data-structures roadmap Level 1 — String ops + parsers (shipped 2026-05-28, closure #298)

✅ AFFINE — all read-only, no heap allocation:
- `str_contains(s: Str, needle: Str) -> bool` — libc strstr.
- `str_starts_with(s: Str, prefix: Str) -> bool` — libc strncmp + strlen.
- `str_ends_with(s: Str, suffix: Str) -> bool` — strlen + strcmp on tail.
- `parse_int(s: Str) -> Option<i64>` — libc strtoll with whole-string consumption check.
- `parse_float(s: Str) -> Option<f64>` — libc strtod, same consumption check.

**Multi-instantiation Option fix:** match-arm pattern resolution
now consults the scrutinee's mangled enum name FIRST (before
falling back to the unmangled base-name prefix lookup). Lets
`Option<i64>` and `Option<f64>` coexist in the same program —
previously the prefix-match returned `None` when both were
registered, breaking `Option.Some(v)` binding extraction.

**Codegen:** tree-C uses GCC statement-expressions for the
multi-step builtins (caching `strlen` results, structuring the
parse result). Tree-LLVM emits inline IR per call site —
multi-block bodies with phi for the option result. SSA routes
both through tree.

**Tests:** 6 lib tests (each builtin typecheck + LLVM compile;
multi-instantiation Option<i64>+Option<f64> coexistence; C
output references the libc primitives). New
`examples/string_ops.vani` covers all five builtins; added to
the parity runner.

**Pending follow-ups (queued):** heap-allocating string ops —
`str_split(s, sep) -> Vec<OwnedStr>`, `str_trim(s) -> OwnedStr`,
`str_replace(s, from, to) -> OwnedStr`. These need explicit
affine wiring for `Vec<OwnedStr>` and per-call OwnedStr Drop
semantics, hence the separation.

### Data-structures roadmap Level 1 — Array ops on `[i64; N]` (shipped 2026-05-28, closure #297)

✅ AFFINE — `sort` / `sort_by` / `reverse` / `find` / `contains` /
`binary_search` extended to accept `mut ref [i64; N]` (sort /
sort_by / reverse) or `ref [i64; N]` (find / contains /
binary_search). Same surface names as the Vec variants;
dispatch is by argument type. v1: i64 element only.

**Codegen:**
- **Tree-C**: shared `intent_array_int64_t__<op>(int64_t* a,
  uint64_t n, ...)` runtime helpers — one set per program,
  pointer + length signature so a single helper covers every
  `[i64; N]` shape. Emitted in `body` after enum typedefs so
  `find` / `binary_search` (returning `Enum_Option__i64`)
  resolve. Gated on the program actually using `[i64; N]` via
  `program_uses_i64_array`. find / binary_search additionally
  gated on `Option__i64` being in the enum-payload registry.
- **Tree-LLVM**: shared `@intent_array_i64__<op>(i64* %a,
  i64 %n, ...)` module-level defs. Call-site emits a `getelementptr`
  to extract the data pointer from the `[N x i64]*` argument
  then calls the helper with `(data_ptr, N)`. Same Option__i64
  gating as tree-C.
- **SSA**: arrays already fall through to tree via the existing
  `Type::Array` rule in `ssa_type_supported` — no new gate.

**v1 restrictions:**
- i64 element only.
- `dedup` / `swap_remove` / `insert` / `clear` stay Vec-only
  (can't shrink a fixed-size array).

**Tests:** 6 lib tests (array sort + reverse + find +
contains + binary_search typecheck/compile; runtime helpers
emitted). `examples/sort.vani` extended with array demos.
Cross-backend parity green across all sections.

### Data-structures roadmap Level 1 — Vec mutators (swap_remove + insert + clear) (shipped 2026-05-28, closure #296)

✅ AFFINE — all three are in-place over `mut ref Vec<T>`.

**API:**
- `swap_remove(mut ref xs, i) -> T` — O(1) remove by swapping
  slot `i` with the last slot and decrementing len. Order NOT
  preserved. Returns the removed element (by-move for non-Copy
  T). Works for any non-array element.
- `insert(mut ref xs, i, v) -> i64` — shift slots `i..` right
  by one (memmove), place `v` at slot `i`, return the new
  length. Consumes `v`. Works for any non-array element.
- `clear(mut ref xs) -> i64` — drop each element (when non-Copy)
  and reset `len = 0`. Capacity preserved (no realloc).

**Codegen:**
- **Tree-C**: `intent_vec_<T>__swap_remove` / `__insert` /
  `__clear` runtime helpers emitted in `emit_vec_bundle`
  alongside push_mut / pop_mut. insert uses `memmove` for the
  shift; clear's drop walk reuses the per-element drop spell
  from `c_element_drop_old` (the same helper the existing
  `__set` and `__free` paths use).
- **Tree-LLVM**: module-level `@intent_vec_<T>__swap_remove`
  / `__insert` / `__clear` defs emitted in `emit_vec_helpers`
  alongside push_mut / pop_mut. `@memmove` now declared in
  both LLVM preambles (tree + SSA). insert grows capacity if
  needed and reloads the data pointer post-realloc; clear's
  drop walk emits per-element type dispatch (Vec<U> →
  `__free`, OwnedStr → `@free`; other non-Copy shapes deferred
  to a follow-up).
- **SSA-C / SSA-LLVM**: route through tree via
  `ssa_path_supports` gate (same pattern as push_mut / pop).

**v1 restrictions:**
- Array element types (`Vec<[T; N]>`) rejected by the checker
  for all three (matches `pop`'s existing gate).
- clear of `Vec<Struct{owning_field}>` and `Vec<Enum{payload}>`
  doesn't drop per-element owning fields yet (just sets
  len=0); leaks queued as a follow-up.

**Tests:** 6 lib tests (typecheck for all three; rejects
by-value Vec; rejects wrong value type for insert; runtime
helpers emitted in C). `examples/sort.vani` extended with
mutator demos. Cross-backend parity green across all
sections.

### Data-structures roadmap Level 1 — Vec.find + Vec.contains + Vec.binary_search (shipped 2026-05-28, closure #295)

✅ AFFINE — all three are read-only over `ref Vec<i64>`; no
elements moved (the affine-friendly substitute pattern from
the container API memo).

**API:**
- `find(ref xs: Vec<i64>, needle: i64) -> Option<i64>` —
  linear scan; `Some(i)` of first match or `None`.
- `contains(ref xs: Vec<i64>, needle: i64) -> bool` — same
  scan, returns the boolean.
- `binary_search(ref xs: Vec<i64>, needle: i64) -> Option<i64>`
  — assumes `xs` is sorted ascending. Caller responsibility
  to pre-sort.

**Auto-monomorphization:** the checker's
`monomorphize_type_decls_in_program` pass now walks Call
expressions for the search builtin names and synthetically
registers `Option<i64>` so `Option__i64` is materialized
without the user having to write `let _: Option<i64> = …`
explicitly. Result returns still need annotation (the let-
type unification path), but the enum decl is guaranteed in
scope.

**Codegen:**
- **Tree-C**: inline stmt-expr at the call site, building the
  `Enum_Option__i64` struct directly (single-payload layout —
  `{ int32_t tag; int64_t payload; }`).
- **Tree-LLVM**: module-level helper fns in `emit_vec_helpers`
  (`@intent_vec_i64__find` / `__contains` / `__binary_search`).
  find / binary_search emission gated on `Option__i64` being
  registered (avoids forward-referencing a missing type).
- **SSA-C / SSA-LLVM**: route through tree via
  `ssa_path_supports` gate (mirrors push_mut / pop / sort
  treatment for in-place + read-only Vec ops).

**v1 restriction:** `Vec<i64>` only.

**Tests:** 6 lib tests (typecheck + LLVM compile for find,
binary_search; bool return for contains; rejects by-value
Vec; rejects non-i64 element; LLVM emits the Option<i64>
typedef + find helper). `examples/sort.vani` extended with
find / contains / binary_search demos. Cross-backend parity
green.

### Data-structures roadmap Level 1 — Vec.reverse + Vec.dedup (shipped 2026-05-28, closure #294)

✅ AFFINE — both in-place over `mut ref Vec<T>`.

**`reverse(mut ref xs: Vec<T>) -> i64`** — two-pointer swap;
works for any Copy element type. Returns 0.

**`dedup(mut ref xs: Vec<i64>) -> i64`** — drops consecutive
duplicates (sort first for set-like behavior); returns the
post-dedup length. v1: `Vec<i64>` only (needs `==` on the
element; will widen with Hash/Eq interfaces).

**Codegen:**
- **Tree-C**: reverse uses two-pointer swap via temp; dedup is
  the canonical read-cursor / write-cursor loop.
- **Tree-LLVM**: inline IR using alloca-based loop counters
  (no phi). Reverse emitted for any Copy element type; dedup
  alongside sort (i64-only gate).
- **SSA**: routes through tree via `ssa_path_supports`.

**Tests:** 5 lib tests (reverse + dedup typecheck; reverse
rejects by-value Vec; dedup rejects non-i64 element; runtime
helpers present in C output). `examples/sort.vani` extended
with reverse / dedup / sort+dedup combo. Cross-backend parity
green.

### Data-structures roadmap Level 1 — Vec.sort + Vec.sort_by (shipped 2026-05-28, closure #293)

✅ AFFINE — `sort(mut ref xs)` / `sort_by(mut ref xs, cmp)` on
`Vec<i64>`. Vec borrowed by `mut ref` (in-place); returns `i64 0`
so it composes with `let _ = sort(mut ref xs);`. Comparator
signature `fn(i64, i64) -> i64` (i64 is Copy — by-value works).
strcmp convention (negative / zero / positive).

**Codegen:**
- **Tree-C**: Hoare-partition quicksort with insertion-sort
  small-N cutoff (N < 16), tail-recursing on the larger
  partition to bound stack depth. Helpers emitted in
  `emit_vec_bundle` per-element (gated on `Type::I64`).
- **Tree-LLVM**: insertion sort in inline LLVM IR (~70 lines).
  Default ascending comparator emitted as `@intent_vec_int64__cmp_asc`;
  `sort` and `sort_by` wrap a shared `__sort_with` core that
  takes a `i64 (i64, i64)*` comparator.
- **SSA-C / SSA-LLVM**: route through the tree backends via
  `ssa_path_supports` gate (in-place Vec ops go through tree).

**v1 restriction:** `Vec<i64>` only. Wider element widths
follow when the runtime helpers are parameterized (matches the
existing `Mutex<i64>` v1 scope).

**Tests:** 6 lib tests (typecheck for sort + sort_by; rejects
non-mut-ref arg, non-i64 element, wrong comparator signature;
C runtime helpers emitted). 1 example (`examples/sort.vani`)
covering ascending, descending, sort by absolute value, empty
Vec, singleton Vec. Cross-backend parity green.

**Pending follow-ups:**
- Wider element widths (`Vec<i32>`, `Vec<u64>`, etc.).
- Quicksort in LLVM (today insertion sort O(n²)).
- Stable sort variant.
- `sort_by_key` ergonomics layer.

### Condition variables — concurrency primitive (shipped 2026-05-28, closure #292)

✅ AFFINE — `Condvar` is a new affine builtin type (stack-by-value,
mirrors `Mutex` / `Guard`). Pairs with existing `Mutex<T>` +
`Guard<T>` to fill the "wait until predicate becomes true" gap.

**API (5 builtins):**
- `condvar_new() -> Condvar`
- `condvar_wait(ref cv, mut ref g: Guard<i64>) -> i64` — atomic
  release + park + re-acquire; guard stays mut-borrowed
- `condvar_wait_timeout(ref cv, mut ref g, timeout_ms) -> bool`
  — `false` on timeout, `true` on notify
- `condvar_notify_one(ref cv) -> i64`
- `condvar_notify_all(ref cv) -> i64`

**Codegen (both backends):**
- **Tree-C** + **SSA-C**: futex (Linux) / WaitOnAddress (Win) /
  spin-yield fallback (other Unix). Runtime helpers emitted in
  the preamble alongside Mutex helpers; substring-gated.
- **Tree-LLVM**: inline LLVM IR per call site. condvar_wait
  inlines the unlock + park + Drepper re-acquire sequence;
  notify ops emit `atomicrmw add` + `@syscall` /
  `@WakeByAddress*`. `%intent_condvar = type { i32 }` typedef
  in module preamble.
- **SSA-LLVM**: surfaces `EmitError` and falls back to tree-LLVM
  (single-block tree path covers it).

**Tests:** 5 lib tests (basic API typecheck; checker rejects
non-Guard second arg; checker rejects non-Condvar arg on notify;
C runtime helpers present; LLVM typedef present). 1 example
(`examples/condvar.vani`) covering single-thread API surface +
wait_timeout returning `false`. Cross-backend parity:
`examples/condvar.vani` produces identical stdout on both
backends.

**Pending follow-ups:**
- Cross-task wait/notify pattern requires the affine task-capture
  rule to admit `mut ref Mutex<T>` / `ref Condvar` (today tasks
  capture Copy-only). Queued separately as a partial-move
  expansion item.
- SSA-LLVM direct support (currently falls back to tree-LLVM).
- v1 pairs with `Mutex<i64>` only — wider widths wait on the
  parametric Mutex runtime.

### Async / asyncio — concurrency arc (queued 2026-05-27)

⚠️ AFFINE-TENSION (compiler-lowered state machines on an arena)
/ 🛑 NON-COMPLIANT (Rust-style `Pin<&mut Self>` self-references).

Canonical path: the compiler lowers each `async fn` body to an
enum-of-frames stored in `Vec<StateMachine>`; frames never hold
raw pointers into other frames. Single-threaded event-loop driver
(`intent_async_run`) polls until completion; non-blocking I/O
primitives (file / socket / timer) under epoll / kqueue / IOCP;
`Channel<T, N>` is the cooperative coordination primitive.

Dependency chain (L-tier multi-session arc): closures w/ captured
state (Level 3 #17) → `Future<T>` generic enum (uses #281 generic
decls + #283 mixed-payload lift) → `async fn` parser + checker →
state-machine codegen on both backends → event-loop C runtime →
non-blocking I/O stdlib → `await` sugar → cancellation
(`CancelToken` by-ref) → `examples/async_io.vani` parity.

NOT shipping: Rust-style `Pin` self-references, panic-based
cancellation, stackful coroutines / fibers, async inside
`parallel for` bodies. Full design + reasoning in [TODO.md](TODO.md)
under *Async / asyncio — concurrency arc*.

### Session updates 2026-05-26 → 2026-05-27 (closures #269–#291)

- **FFI v1–v8** — `extern "C" fn` declarations (#269), `--link-with`
  / `-l<name>` flags (#270), call-site checker (#271), codegen with
  mangled symbols (#272), struct-by-value rejection w/ `ref T` hint
  (#273), linker-discovery polish (#274), FFI callbacks via
  `Type::FnPtr` (#279), System V x86-64 small-struct return lowering
  (#288). Net: `qsort`-style callbacks + libc string / math interop.
- **vani.toml manifest** — v1 (#280) + v2 `[deps]` inline-table
  (#287).
- **Generic struct + enum declarations** — `enum Result<T, E>` etc.,
  `Type::Apply { name, args }`, mangled names like
  `Result__Vec_I64___AllocError` (#281). Prelude injected at AST
  level — `Option<T>`, `Result<T, E>`, `AllocError` (#282).
- **Mixed-payload enums** — C tagged-union, LLVM `[N x i8]` + bitcast
  (#283).
- **`try_vec(n) -> Result<Vec<i64>, AllocError>`** — fallible alloc
  builtin (#284).
- **FFI param/return rejection hints** (#285).
- **Attribute syntax + `#[bounded(N)]`** — first attribute in the
  language; `#` token; LLVM thread-local + per-Return decrement
  (#286, #289, #290); C uses GCC `__attribute__((cleanup))`.
- **Nested arrays** — `[[T; N]; M]` / `[Vec<T>; N]` end-to-end,
  including per-slot per-field drops for arrays of structs (#291
  Phases 1–4).
- **Other closures** — match on f64 (#278); `let _ = make()`
  discard of fresh struct value frees heap fields (#277); DynCoerce
  non-Var hoist via synthetic Block (#276); parallel-for purity
  hole in reduction RHS (#275).



---

## Current feature set

### Language surface
- **Scalar types:** `i8/16/32/64`, `u8/16/32/64`, `f32/64`, `bool` (all `Copy`).
- **Aggregates:** `[T; N]` (stack, affine), `Vec<T>` (heap, affine, monomorphized struct) with `vec`/`push`/`set`/`clone`/`len`/index/`clone_at`. `Vec<T>` accepts non-`Copy` elements (`Vec<Vec<i64>>`, `Vec<[i64; N]>`); reading inner non-Copy slots into a binding goes through `clone_at(&xs, i)` (returns an owned deep-clone) since bare indexing would alias.
- **Strings:** `Str` (borrowed C-string, `Copy`, `==`/`!=`/`<`/`<=`/`>`/`>=` via strcmp, `len` via strlen), `OwnedStr` (heap, affine, produced by `+` concat).
- **References:** second-class `&T` and `&mut T` (params only, no returns, no let-bindings, no nested refs). Call-site aliasing rejected. Auto-deref for indexing.
- **First-class fn-pointers** `fn(T1, ...) -> R` (FnRef + indirect call). Pure / parallel-for / lock-graph passes reject indirect calls conservatively.
- **Control flow:** `if/else`, `else if`, `while`, `for i in lo..hi`, `for x in &xs`, `for x in xs` (consuming), `break`, `continue`. Lexical scoping (nested scopes, shadowing).
- **Constructs:** `let`, `let _ =`, plain `name = expr;` reassignment, `assert cond[, "msg"]`, `prove`, `print` (multi-item), discarded `call();` / `receiver.method();` as a statement (sugar for `let _ = …`), block expressions `let r = { let a = …; let b = …; a + b };` (Let stmts then tail expr).
- `intent "…";` module header; multi-file with `use "path.intent";` (transitive resolve, cycle detection).

### Verification & contracts
- `requires` / `ensures` — call-site discharge, callee-side check. `ensures _return[k] == V` array facts propagate.
- `invariant` on `while`/`for` (entry + preservation with last-reassignment substitution + post-loop facts).
- Contradictory-requires detection.
- SMT (z3) with BitVec overflow semantics, FloatingPoint theory (NaN/±inf modeled), signed/unsigned compare split, casts via sign/zero-extend.
- Symbolic SMT arrays per Vec/Array binding with versioned store axioms.
- SMT-driven runtime-guard elision: bounds, divisor, shift checks dropped when discharged.
- Three-layer `prove`: constant fold → structural tautology → SMT.
- Dev opt-out `INTENTC_NO_VERIFY=1`.
- Affine-ownership with move-state reconciliation at if/while merges and break/continue jump points; per-scope auto-drops.

### Concurrency
- `parallel for` with reduction clauses (`+`, `*`, `&&`, `||`, `min`, `max`, bitwise `&`/`|`/`^`). Verifier proves race-freedom.
- `task <name> { … } / join <name>;` — affine handles, Copy-only captures, real-thread.
- `Atomic<T>` for i8..i64, u8..u64, bool — `atomic_new`/`load`/`store`/`fetch_add`/`compare_exchange`, seq_cst.
- `Channel<T, N>` — Vyukov MPSC ring buffer (per-slot seq counter, power-of-2 N, integer + bool elements).
- `Mutex<T>` + RAII `Guard<T>` — Drepper three-state futex on Linux, WaitOnAddress on Windows, sched_yield/SwitchToThread fallback elsewhere. Cross-function double-acquire detection via transitive `locks_params` propagation.

### Compilation pipeline
- Lexer → Parser (error recovery) → Type checker → SMT verifier → Typed IR → SSA IR → backend.
- Two backends:
  - **LLVM** (default for `emit` / `run` / `build`, AOT via `llc + cc`).
  - **C** (`--backend=c`, legacy; still authoritative for several patterns).
- `intentc` routes through the SSA pipeline first with graceful fallback to the tree backends when the SSA-path gate rejects the program (parallel-for, Tasks, `Channel`/`Atomic`/`Mutex`/`Guard` params).
- **SSA-C covers:** scalars, control flow, arrays, Vec, StrLit, RefOf, fn-pointers, multi-item print, strcmp/strlen, OwnedStr concat, assert-with-message, OwnedStr Drop.
- **SSA-LLVM covers:** same set + phi nodes for block params, vec_struct typedefs, `intent_str_concat` reused from tree-LLVM. `Hint(ParallelForBegin)` regions get **real `@GOMP_parallel` outlining** including:
  - **Captures** — body-block free variables get marshalled by-value through the ctx struct under the shared `%cap_<i>_p` / `%cap_<i>` naming convention; the outlined fn loads each capture and aliases it to the SSA `%v_<id>` so body instructions resolve without operand rewriting.
  - **All reduction ops** — `+` (`atomicrmw add`), `&`/`|`/`^` bitwise (`atomicrmw and/or/xor`), `min`/`max` (`atomicrmw min`/`max` for signed, `umin`/`umax` for unsigned), `*` via `cmpxchg` retry loop, `&&`/`||` on bool via an i8 shadow (parent allocates i8, zext-stores init; outlined fn `atomicrmw and/or` on i8 with `zext i1→i8` of the increment; final-load uses `icmp ne i8 _, 0`). Parent-side `alloca` accumulator carries the user's init value; the pointer flows through the ctx struct; after `@GOMP_parallel` the parent loads the final value and binds to the exit-block param.
  - **`Hint(TaskBegin)` tasks** — single-block task bodies get lifted into `@intent_task_<N>` outlined fns; the spawn site emits `@pthread_create` (POSIX) or `@CreateThread` (Win32) against the outlined fn with a heap-allocated ctx holding captures by-value; the matching `Hint(TaskJoin)` emits `@pthread_join` / `@WaitForSingleObject` + `@CloseHandle` plus `@free` of the ctx. Multi-block task bodies (with control flow inside the `task { … }`) surface `EmitError` → tree-LLVM fallback.
- **SSA-C handles `parallel for`** via the
  `emit_parallel_for_region` machinery for single-block
  bodies with the canonical {counter, …reductions} carry
  shape. Emits structured `for (v_<counter> = start; v_<counter> < end; v_<counter>++) { … }` + `_Pragma("omp parallel for [reduction(op: v_<carry>)…]")`.
  `min` / `max` are recognized as intrinsics and lower to
  inline ternaries (matches tree-C). Multi-block bodies and
  non-canonical carry shapes surface `EmitError` → tree-C
  fallback. `parallel.intent` runs end-to-end through SSA-C
  with the right outputs (100, 240000, 1, 10, 40, 0, 62, 40
  for the eight reductions).
- **SSA-C handles `task`/`join`** via outlined `static void*
  intent_task_<N>(void*)` functions emitted to a module-
  scope `TASK_OUTLINES` buffer (spliced between user-fn
  prototypes and bodies so task outlines can call into
  user-defined helpers). Single-block task bodies only —
  multi-block bodies surface `EmitError` → tree-C fallback.
  Spawn site uses `intent_thread_create` (POSIX
  `pthread_create` / Win32 `CreateThread` via the cross-
  platform wrapper); join uses `intent_thread_join` +
  `free` of the heap ctx. Cross-platform threading
  wrappers + the `intent_task_handle` typedef are now
  shared with tree-C via
  `backend_c::emit_intent_thread_wrappers_c`.
- **C runtime is Windows-portable:** `intent_thread_t` wrappers (`CreateThread` / `pthread_create`), mutex `WaitOnAddress` / futex arms; `intentc build` gates `-pthread` vs `-lsynchronization`.
- **LLVM runtime is Windows-portable** for both tree-LLVM and SSA-LLVM: `cfg!(target_os)`-gated threading declares + spawn-site (`@CreateThread` returning HANDLE, `ptrtoint`'d into the i64 handle slot) + join-site (`@WaitForSingleObject` + `@CloseHandle`) + mutex park (`@WaitOnAddress`) + Guard drop wake (`@WakeByAddressSingle`). `parallel for` open-codes a hardcoded-N=4 `@CreateThread` fan-out on Windows in lieu of libgomp; the outlined fn returns `i8*` (CreateThread's start-routine ABI) and reads `tid`/`nt` out of a per-thread `WinParArg { i8* ctx, i64 tid, i64 nt }` struct instead of calling `omp_get_*`. Helper `host_uses_win32_threading()` drives all the dispatch.

### Tooling
- `intentc check / emit / emit-c / run / build / test` subcommands. `--json` diagnostics for editors/CI.
- `intent-lsp` binary: didOpen/didChange/didClose, publishDiagnostics, hover, definition, references, rename, completion, code actions (single-char insert quickfix), semantic tokens (7-type legend, declaration/readonly modifiers).

---

## TODOs

The list below replaces the now-closed backend / verifier / Vec-of-non-Copy
queue. Canonical priority order — work top-down. Items 1–9 are explicit
language-surface gaps; items 10–11 are deferred multi-week build-outs.
Closed-out history lives in TODO.md.

### Design philosophy (read before reordering)

The compiler's reach should be **20 % of Rust/C++'s feature set covering
80 % of the programs an average programmer writes**, optimized for two
things above all else:

1. **Memory safety enforced at compile time.** No NULL dereferences, no
   use-after-free, no double-free, no data races, no resource leaks, no
   uninitialized reads. Errors that crash or corrupt at runtime in C++
   surface as type errors before the program ever runs.
2. **The programmer writes ownership once; the compiler schedules
   cleanup.** Affine ownership + automatic drop chains at scope exit
   mean the user declares WHAT owns a resource — the compiler decides
   WHEN to release it. No `free`, no `close`, no `delete`, no `try /
   finally` boilerplate in user code.

Concrete consequences that shape every choice below:

- **RAII is the only resource-management mechanism.** Affine types are
  auto-dropped at scope exit; struct fields drop in reverse-declaration
  order; user types opt into custom cleanup by implementing the `Drop`
  interface. The compiler already does this implicitly today for
  `Vec<T>`, `Mutex<T>`/`Guard<T>`, `OwnedStr`, and `Task` handles —
  Tier 1 promotes it from a built-in privilege to a user-extensible
  one without changing the underlying machinery.
- **No exceptions.** Hidden control flow breaks both the SMT verifier
  and the "memorize the language" goal. Failure is modelled with
  `Result<T, E>` + the `?` operator — every fallible call's failure
  edge is visible in the source.
- **Composition over inheritance.** Structs can hold other structs and
  implement multiple interfaces. There is no inheritance chain, no
  method override, no diamond resolution.
- **Generics WITHOUT trait bounds in v1.** Type parameters are opaque
  inside the function — only "move / pass / return" work generically.
  Anything that needs structural behaviour (compare, hash, equality,
  drop) goes through an **interface bound** (`<T: Cmp>`). This keeps
  the language tiny without losing collections.
- **Interfaces, not full traits.** No interface inheritance, no default
  methods, no associated types in v1. One layer of abstraction. `Drop`
  is the one interface the compiler treats specially (auto-invoked at
  scope exit).
- **Affine ownership is the only safety mechanism.** No GC, no
  reference-counting, no first-class lifetimes. Borrow-checking
  remains scoped to function parameters (`ref T` / `mut ref T`).

### Syntax conventions (keep keywords first; symbols only when math-universal)

Intent prefers spelled-out keywords over operator soup. The table
below is the canonical replacement list — every TODO item in the
tiers below uses it. The rule: a keyword wins unless the operator
form is also valid math notation (`<`, `==`, `&&`, `<T>` for type
parameters, `{ }` for blocks, etc.).

| Concept | Rust/C++ shape | Intent shape | Why |
| --- | --- | --- | --- |
| Borrow | `&x` | `ref x` | reads as "reference to" |
| Mutable borrow | `&mut x` | `mut ref x` | composes `mut` + `ref` |
| Path resolution | `Point::dist`, `Color::Red` | `Point.dist`, `Color.Red` | one operator for fields + namespace |
| Error propagation | `parse(s)?` | `try parse(s)` | explicit "try this" |
| Range (loop) | `for i in 0..10` | `for i from 0 to 10` | BASIC-style English |
| Range (slice) | `xs[lo..hi]` | `xs[lo to hi]` | same `to` |
| Return type | `fn foo() -> i64` | `fn foo() returns i64` | the arrow is jargon |
| Match arm | `Red => "red"` | `Red then "red"` | reads as "if Red then …" |
| Closure | `\|x\| x + 1` | `with x do x + 1` | no pipe-symbol |
| Method/field receiver | `&self`, `&mut self` | `self`, `mut self` | `self` is always borrowed; `mut` flips it |
| Interface decl | `trait Cmp` | `interface Cmp` | universally familiar |
| Interface impl | `impl Drop for File` | `implement Drop for File` | reads as a sentence |
| Methods block | `impl Point { … }` | `methods on Point { … }` | reads as a sentence |
| Generic bound | `fn min<T: Cmp>` | `fn min<T> where T is Cmp` | `where … is …` reads as English |

Things explicitly NOT replaced (math-universal symbols carry their
weight):

- `<T>` type parameters — replacing with `of T` makes `Map<K, V>`
  ambiguous.
- `:` type annotations (`let x: i64`) — Python / TypeScript shape,
  universally known.
- `==` `!=` `<` `<=` `>` `>=` `&&` `||` `!` — math/boolean basics.
- `{ }` blocks, `[ ]` indexing, `( )` grouping/call.

#### Retroactive updates to existing syntax

Today's source uses a few shapes that pre-date the keyword-first
convention. These get a single sweep when Tier 1 starts (or sooner
if surface is small enough to fold in). Tracked here so the rewrite
isn't accidentally piecemeal:

| Today's shape | New shape | Where it shows up |
| --- | --- | --- |
| `for i in 0..n` | `for i from 0 to n` | every `for`-counter loop |
| `for x in &xs` | `for x in ref xs` | borrowing iteration |
| `for x in &mut xs` | `for x in mut ref xs` | mutable iteration (when added) |
| `parallel for i in 0..n` | `parallel for i from 0 to n` | parallel loops |
| Param type `&T` / `&mut T` | `ref T` / `mut ref T` | fn signatures + ref expressions |
| Call site `foo(&x)` / `foo(&mut x)` | `foo(ref x)` / `foo(mut ref x)` | every borrow at a call |
| Range slice `xs[lo..hi]` | `xs[lo to hi]` | future slice ops (T3.10) |

Concurrency keywords that DON'T need rewriting (already keyword-shaped
and clear): `task`, `join`, `parallel`, `reduce <var> with <op>`,
`pure fn`, `requires`, `ensures`, `invariant`, `intent`, `use`, `as`,
`assert`, `prove`, `print`.

`with` is reused in two contexts: `reduce <var> with <op>` (operator
follows the keyword) and `with x do <body>` (identifier + `do`
follows). The parser disambiguates on the token immediately after
`with` — no genuine ambiguity, and keeping two separate keywords
would lose the reading "perform this reduction *with* the +
operator" / "evaluate this expression *with* x bound to …".

Concrete example combining the above — a struct with custom RAII,
a method block, a fallible constructor with `?`-propagation, an
interface implementation, and a generic function with a bound:

```intent
struct FileHandle { fd: i32, path: OwnedStr }

methods on FileHandle {
  fn open(path: Str) returns Result<FileHandle, Str> {
    let fd: i32 = try sys_open(path);
    return Ok(FileHandle { fd: fd, path: path });
  }

  fn read(self) returns Result<OwnedStr, Str> {
    return sys_read(self.fd);
  }
}

implement Drop for FileHandle {
  fn drop(mut self) {
    sys_close(self.fd);
  }
}

interface Cmp { fn cmp(self, other: ref Self) returns i64; }

fn min<T>(a: T, b: T) returns T where T is Cmp {
  if a.cmp(ref b) <= 0 { return a; } else { return b; }
}

enum Color { Red, Green, Blue }

fn describe(c: Color) returns Str {
  match c {
    Color.Red   then "warm",
    Color.Green then "cool",
    Color.Blue  then "cool",
  }
}

fn sum_each(xs: ref Vec<i64>) returns i64 {
  let total: i64 = 0;
  for x in ref xs { total = total + x; }
  return total;
}

fn main() returns i64 {
  let h: FileHandle = try FileHandle.open("/tmp/x.txt");
  let body: OwnedStr = try h.read();
  for i from 0 to 5 {
    print body;
  }
  return 0;
}
```

### Tier 0 — Syntax sweep (no new features; aligns existing source to the convention)

0. **Apply the keyword-first sweep** — *done 2026-05-20*. Lexer
   gained `Ref` / `From` / `To` keyword tokens. Parser rewired:
   type position accepts `ref T` / `mut ref T` (old `&T` / `&mut T`
   surfaces a guidance error pointing at the new keyword), unary
   borrow accepts `ref x` / `mut ref x` (likewise), for-loop range
   shape is now `for VAR from LO to HI` (old `for VAR in LO..HI`
   removed), for-iter borrow is `for VAR in ref XS`. Formatter
   updated to emit the new shapes for `Type::Ref` / `Type::RefMut`,
   `ExprKind::Ref` / `ExprKind::RefMut`, and both for-loop variants.
   `Type::Display` matches. All 27 example files swept, every test
   that pinned source text (4 lib tests + 2 SSA crosscheck files)
   migrated. Bitwise `&` / `|` / `^` operators remain available in
   binary position (reductions, expressions); `&` as a prefix borrow
   is rejected with a friendly hint. 456 lib + 47 e2e + 3
   integration tests green on the new syntax.

### Tier 1 — Composite types (foundation; rest of the list waits on this)

1. **Tuples** — *done 2026-05-20*. `(T1, T2, T3)` multi-return,
   light-weight grouping. v1 ships 2..=4 element tuples, Copy-only
   element types, destructure-only access. **AST:** new
   `Type::Tuple(Vec<Type>)`, `ExprKind::Tuple(Vec<Expr>)`,
   `ExprKind::TupleAccess { tuple, index }`, and a parse-only
   `Stmt::LetTuple { names, annotation, expr }` that the checker
   desugars. **IR:** `TypedExprKind::Tuple` + `TypedExprKind::TupleAccess`
   (no new `TypedStmt` variant — the checker lowers `LetTuple` to
   a sequence of `TypedStmt::Let`s reading from a synthetic temp).
   **Parser:** tuple type `(T1, …)` accepted in every type position;
   tuple expression `(e1, …)` disambiguated from grouped expression
   by the top-level comma; destructure-let
   `let (a, b) = expr;` produces `Stmt::LetTuple`. **Checker:**
   `check_expr` handles both new ExprKinds (enforces 2..=4 elements,
   Copy-only, in-bounds index); `check_one_stmt`'s LetTuple arm
   verifies arity matches names count, rejects duplicate names,
   and emits the desugared sequence. **Tree-C:** new
   `tuple_c_struct` / `emit_tuple_bundle` / `collect_tuple_shapes`
   pipeline emits per-shape `typedef struct { T1 _0; T2 _1; …; }
   intent_tuple_<tags>;` in the preamble before vec / array
   typedefs; tuple construction emits a designated-initializer
   compound literal; `.<index>` access emits `._<index>` field
   reads. **Tree-LLVM:** `llvm_type_string` returns the
   anonymous-struct literal `{ T1, T2, … }`; tuple construction
   emits an `insertvalue` chain; `TupleAccess` emits `extractvalue`;
   `is_scalar` includes Tuple so the existing scalar Let path's
   `alloca + store` shapes work uniformly. **SSA backends:** tuple
   lowering surfaces `LowerError` for now, routing programs
   through the tree-fallback. **Tests:** three new lib tests
   (`tuple_multi_return_and_destructure`,
   `tuple_arity_mismatch_rejected`,
   `tuple_non_copy_element_rejected`). 456 → 459 lib tests; 47
   e2e + 3 integration unchanged. **Follow-ups (later phases):**
   non-Copy elements (lifts the Copy gate; needs RAII drop chains
   like Vec phase 1), `.0` / `.1` direct field-access syntax in
   source (today's destructure-let desugars to that internally),
   nested tuples / tuples >4 elements, SSA backend support.
2. **Structs (records)** — *phase 1 done 2026-05-20*. Top-level
   `struct Point { x: i64, y: i64 }` decls, field-init literals
   `Point { x: 3, y: 4 }`, and field-access `p.x`. v1 caps at 1..=8
   fields, **Copy-only** fields, no `methods on` blocks yet, no
   RAII chains. Both backends emit a per-name struct typedef
   (C: `typedef struct { … } Struct_<Name>;`; LLVM: `%Struct_<Name>
   = type { … }`). New `TokenKind::Dot` enables the postfix
   `.<ident>` / `.<int>` access syntax (re-used for `t.0` tuple
   access from T1.1). Four new lib tests pin: working struct
   decl + literal + access, missing-field rejection, unknown-
   field rejection, non-Copy-field rejection. SSA backends fall
   back to tree via `LowerError` (parallel to tuples).
   **Phase 2a done 2026-05-20**: `methods on Point { fn
   dist(self: Point) -> i64 { … } }` block syntax +
   `p.dist()` method-call sugar work end-to-end. New
   `methods` lexer keyword; AST `MethodsBlock { for_type,
   methods }` on `Program`; new
   `ExprKind::MethodCall { receiver, method, args }` for
   the postfix `.<ident>(args)` shape (existing
   `.<ident>` stays as FieldAccess when no `(` follows so
   `p.x` still field-accesses). Parser disambiguates via
   lookahead on `(`. The checker hoists each method into
   the regular function table with mangled name
   `<TypeName>_<methodName>` (after enum + alias
   resolution so the type name is accurate), validates the
   methods-block target is a struct/enum, catches
   duplicate methods, and catches mangled-name collisions
   with existing functions. MethodCall expressions desugar
   at check time: the receiver's type yields the
   type-name, and the call becomes a regular
   `Call { name: "<T>_<method>", args: [receiver, …] }`
   consumed by the existing call-checking pipeline.
   **Auto-ref**: when the method's first param is
   `ref T` / `mut ref T` and the receiver is a plain
   value of `T`, the desugar wraps the receiver in
   `ExprKind::Ref` / `ExprKind::RefMut` so users can
   write `p.method()` whether the method binds `self`
   by value or by borrow — no manual `ref(p).method()`
   ceremony. **Field assignment** (`p.x = expr;` and
   `self.field = expr;` through a `mut ref T` receiver)
   now works end-to-end. New `Stmt::FieldAssign` +
   `TypedStmt::FieldAssign` carry the (object, field,
   value, through_mut_ref) shape; parser uses a
   `looks_like_field_assign` lookahead that walks
   `<ident>(.<ident>)+ =` and rejects `.<ident>(`
   patterns (which are method calls). The checker
   validates the place is an owned struct or a
   `mut ref` to one, requires the field name to exist,
   coerces the value type, and forbids field-assign
   through an immutable `ref`. Tree-C emits
   `obj.field = value;` / `obj->field = value;`;
   tree-LLVM emits the appropriate `getelementptr` +
   `store` (or `load` + `insertvalue` + `store` for
   owned structs). SSA path gates with a clear
   "field assignment is not yet supported"
   `LowerError` so it falls back to the tree backend.
   The effects-checker treats FieldAssign as a side
   effect so `parallel for` / `task` reject it
   correctly. Twelve new lib tests pin: basic method,
   method with extra args, missing-method rejection,
   primitive-receiver rejection, duplicate-method
   rejection, self-field access, auto-ref to `ref T`
   self, auto-ref to `mut ref T` self, owned-struct
   field-assign with `.x =` in emitted C, mut-ref
   field-assign via a counter-bump method, unknown-field
   rejection, immutable-ref field-assign rejection. Two new
   format round-trip tests pin the methods-block +
   method-call surface shapes. A new
   `examples/methods.intent` exercises four shapes
   (value-self, ref-self, ref-self reading consts,
   value-self returning a new instance) end-to-end
   and is included in the `intentc test` pass.
   **Nested affine struct fields + recursive Drop done
   2026-05-22**: `struct Outer { inner: Inner, … }` where
   Inner has heap fields now compiles. Both backends emit
   recursive Drop walks through nested struct types. The
   non-Copy registry uses fixed-point iteration so source
   order doesn't matter. Nested-path moves like
   `let v = o.inner.s;` are gated with a workaround hint
   (move the inner struct out first) — full path-level
   move tracking is deferred. See
   [examples/nested_struct_drop.intent](examples/nested_struct_drop.intent).

   **Mutex / Channel enum payloads done 2026-05-22**:
   `enum Locked { Held(Mutex<i64>), Free }` and analog
   for Channel now compile. Symmetric to closure #123's
   struct-field work — Mutex/Channel are inline layouts
   with no Drop concern; gate-lift + LLVM
   `zeroinitializer` extension cover everything.

   **Mutex / Channel struct fields done 2026-05-22**:
   `struct State { m: Mutex<i64> }` and the analog for
   `Channel<T, N>` now compile. Combined with the
   field-borrow work from closure #102,
   `mutex_lock(ref s.m)` flows cleanly. Both are inline
   struct layouts; per-field Drop is a no-op. Only
   `Guard<T>` remains rejected as a struct field — its
   RAII unlock is bespoke and needs more wiring.

   **Task + Atomic enum payloads done 2026-05-22**: closes
   the originally-listed affine enum payload types. Only
   Mutex / Guard / Channel payloads remain rejected. Both
   Task and Atomic have no Drop story (Task drops via
   join; Atomic is a primitive cell). LLVM
   `llvm_type_string` gained a Task arm
   (`%intent_task_handle`); the payload-less `zeroinitializer`
   list extended to include Task.

   **Const initializer arithmetic done 2026-05-22**:
   `const B: i64 = A + 1;` (and `*`, `-`, `/`, `%`) folds
   over previously-declared integer consts. Checker
   `literal_const_value` and parser `expr_as_int_literal`
   both walk Var / Binary nodes recursively with checked
   arithmetic. The resolved value flows into the `[T; N]`
   array-length resolver from closure #120 too.

   **`const N` as array length done 2026-05-22**: users
   can now declare `const SIZE: i64 = 8;` and reference
   SIZE in array types (`[i64; SIZE]`) across let
   annotations, fn params, struct fields, and array
   literals. The parser stashes integer-literal const
   values during `parse_const_decl` and resolves them in
   the array-length slot at parse time. Forward references
   and non-literal const initializers still error cleanly.

   **`[T; N]` enum payload done 2026-05-22**: arrays of
   Copy elements are now valid as enum payloads. No Drop
   needed (stack lifetime). C-side uses an inline `T name[N]`
   declarator (via `format_declarator`) and a bare-brace
   `{e1, e2, …}` initializer for the variant constructor.
   LLVM `zeroinitializer` extended to cover Array payloads
   for payload-less variants. See
   [examples/enum_arr_payload.intent](examples/enum_arr_payload.intent).

   **Vec<T> enum payload done 2026-05-22**: extends #113's
   OwnedStr work — enum variants can now hold `Vec<T>`
   payloads. Both backends emit a tag-conditional
   `intent_vec_<T>__free` at scope exit. C ordering pre-
   pass walks enum payload types alongside struct fields
   so the Vec typedef lands before the enum typedef. LLVM
   uses `zeroinitializer` (C uses `{0}`) for payload-less
   variants of aggregate-payloaded enums. See
   [examples/enum_vec_payload.intent](examples/enum_vec_payload.intent).

   **SSA bool-print parity done 2026-05-22**: bool prints
   through both SSA backends now render as "true"/"false"
   instead of "1"/"0". SSA-C uses `fputs(v ? "true" :
   "false", stdout)`; SSA-LLVM uses `select i1` over two
   private string globals + `printf("%s", …)`.

   **Empty struct + bare-block scope-stmt done 2026-05-22**:
   `struct E {}` is now accepted for marker/zero-sized
   types; struct-lit lookahead handles `Type {}`. Bare
   `{ stmts; }` as a free-standing statement desugars to
   `if true { stmts; }` at parse time, providing explicit
   scoping for nested bindings.

   **Unit-return functions done 2026-05-22**: `fn f() { … }`
   without `-> Type` is parser sugar for `-> i64` with an
   implicit `return 0;` appended. Callers invoke as bare
   statement or via `let _ = f();`. Idempotent synthesis —
   if the user already ends with `return`, no extra is added.
   See [examples/unit_return.intent](examples/unit_return.intent).

   **Type-associated functions done 2026-05-22**:
   `methods on T { fn helper(args) -> R { … } }` (without
   `self`) declares a type-associated function, callable
   as `T.helper(args)`. The checker hoists it to the same
   `<T>_<method>` mangled name; the MethodCall handler
   recognizes a Var receiver naming a struct/enum and
   dispatches directly to that mangled function (no
   self-receiver prefix). Co-exists with regular
   `recv.method()` dispatch in the same block. See
   [examples/type_associated_fn.intent](examples/type_associated_fn.intent).

   **Enum payloads admit OwnedStr done 2026-05-22**: enums
   like `enum Maybe { Some(OwnedStr), None }` are now valid
   in v1. The aggregate is affine; both backends emit a
   tag-conditional `free` at scope exit, only for variants
   that carry a heap payload. New `ENUM_NON_COPY_REGISTRY`
   in [src/ast.rs](src/ast.rs) parallels the struct one;
   `LLVM_/(C)_ENUM_PAYLOAD_TAGS_REGISTRY` thread-locals
   drive the per-variant dispatch. v1 limitation: matching
   without a binding only — destructure-binding patterns
   for non-Copy payloads are rejected (alias-vs-Drop
   tracking is deferred). See
   [examples/enum_owned_payload.intent](examples/enum_owned_payload.intent).

   **Deep field paths for `xs[i].a.b = v` done 2026-05-22**:
   the depth gate in the checker was lifted; the existing
   loop already validates each segment with per-step type
   descent and a Copy check on intermediates and the leaf.
   Backends already iterated over segments (closure #109).
   See updated
   [examples/mixed_place_assign.intent](examples/mixed_place_assign.intent).

   **Mixed-place leaf-OwnedStr / leaf-Vec done 2026-05-22**:
   the Copy gate on the leaf segment of `xs[i].a.b = v` was
   relaxed for heap-shaped types (`OwnedStr` and `Vec<T>`).
   Intermediate segments still require Copy. Both backends
   emit a free of the old slot before storing the new value:
   the C backend writes `free((void*)<lv>)` (OwnedStr) or
   `intent_vec_<T>__free(<lv>)` (Vec); LLVM loads the old
   pointer and calls `@free` or `@intent_vec_<tag>__free`.
   Closure #126 / F2. See updated
   [examples/mixed_place_assign.intent](examples/mixed_place_assign.intent).

   **LLVM Vec `__set` frees old element done 2026-05-23**:
   The per-shape Vec `__set` helper in LLVM only freed
   the old slot for `Type::Vec(inner)` element types —
   `set(Vec<OwnedStr>, …)`, `set(Vec<Struct{heap}>, …)`
   and `set(Vec<Enum{OwnedStr}>, …)` leaked the previous
   slot's heap. Closure #127 had extended the analogous
   tree-C `c_element_drop_old`; this closes the LLVM
   parallel by adding `Type::OwnedStr` (direct `@free`),
   `Type::Struct` (per-field `emit_vec_element_struct_drop`)
   and `Type::Enum` (extract tag/payload, OR-chain over
   payloaded tags, branch to free vs done block) arms.
   Closure #157.

   **README + examples + TODO refresh — smart pointers, FFI, build headers done 2026-05-27**:
   triple update in response to three user questions:

   1. **Smart-pointer / cycle-avoidance section** added to
      the README's *Memory safety & concurrency model*:
      vāṇी ships none of Rust's Box / Rc / Arc / RefCell /
      Weak or C++'s unique_ptr / shared_ptr / weak_ptr.
      Each use case is either covered by an existing
      primitive (Vec/OwnedStr replace Box; Channel /
      Atomic / Mutex through refs replace Arc + lock
      patterns) or **structurally avoided by the type
      system** (no shared ownership → no cycles → no
      Weak needed). Added a `Vec<Node>` + index pattern
      worked example for cyclic data structures.

   2. **Multi-file linking + cross-language FFI** section
      added under *Multi-file projects*. Documents the
      `intentc emit + llc -filetype=obj → .o` pipeline,
      function symbol naming (`fn_<name>` with C ABI),
      and how to declare vāṇी fns on the C / C++ / Rust
      side via `extern "C"` blocks. Calling-INTO vāṇी
      works today. Calling-OUT (`extern fn foo();`
      declarations in vāṇी source) is queued — TODO.md
      gets a new FFI section laying out the design
      (surface syntax, ExternFn IR shape, effects
      treatment as impure-by-default, toolchain
      threading).

   3. **All 62 example files** now have a 4-line build-and-
      run header at the top: `intentc run` (LLVM JIT),
      `intentc run --backend=c`, and `intentc build`
      (native binary) variants. Headers survive the
      formatter's round-trip (comments preserved).

   Test totals unchanged: 978 lib + 47 e2e + 11
   vtables-phase3 + 2 user-drop-by-ref + 1 ssa-examples.
   Closure #268.

   **FFI v1 — `extern "C" fn` declarations done 2026-05-27**:
   first-class foreign-function interface. Surface syntax
   `extern "C" fn name(params) -> R;` declares a C-ABI symbol
   that the linker resolves at build time. Touches every
   layer:

     • Lexer — new `Extern` token + structure-keyword
       registration so error-recovery treats `extern` as a
       top-level boundary.
     • Parser — `parse_extern_fn` accepts the body-less
       prototype, dispatches at the top level.
     • AST / IR / SSA — `is_extern: bool` field on Function /
       TypedFunction / SSA Function, defaulted false; carries
       the marker through every pass.
     • Checker — early-return on `is_extern`: registers the
       signature, skips the "must return" rule (empty body
       is correct for an FFI declaration).
     • SSA lowering — emits a stub Function with `is_extern
       = true` and an unreachable terminator; no body
       lowering.
     • Codegen — both backends emit a prototype, not a
       definition: LLVM `declare RET @name(...)` (no `fn_`
       prefix), C `extern RET name(...);` (no `static`).
       Call sites consult per-backend thread-local
       registries (`LLVM_EXTERN_FN_REGISTRY` /
       `C_EXTERN_FN_REGISTRY`) populated at module entry,
       and switch to the bare C-ABI name. Tree-C path's
       prototype emitter also short-circuits so we don't
       emit a `static fn_<name>(...)` ghost prototype.
     • Both tree and SSA emit paths updated in lock-step for
       both backends (4 paths total).

   Effects model: extern fns are conservatively treated as
   impure — the SMT engine can't reason across the FFI
   boundary, so `prove`/`assume` involving an extern call
   must rest on caller-side invariants.

   New tests: 3 lib tests (`extern_c_fn_parses_and_checks_
   without_body`, `extern_c_fn_emits_bare_c_prototype_and_
   call`, `extern_c_fn_emits_llvm_declare`). New example
   `examples/ffi.vani` (calls libm's `abs`) runs end-to-end
   on LLVM-JIT, AOT LLVM, tree-C, and SSA-C — all four
   paths emit `abs(-7) = 7`.

   Test totals: 981 lib + 47 e2e + 11 vtables-phase3 + 2
   user-drop-by-ref + 1 ssa-examples. Closure #269.

   **FFI v2 — `intentc build --link-with PATH` / `-l<name>` done 2026-05-27**:
   the linker side of the FFI story. `intentc build` learns
   two flag shapes that are forwarded to the system linker
   (`cc`):

     • `--link-with PATH` (repeatable) — pass an extra
       object or source file to cc. The common shape: a
       `helper.c` (or `.o`) carrying the body of an `extern
       "C" fn` declared in vāṇी. Both `--link-with PATH`
       and `--link-with=PATH` accepted.
     • `-l<name>` (repeatable) — pass a system library
       link flag to cc (e.g. `-lm` for libm, `-lcurl`).
       Forwarded verbatim.

   Both flag groups appear after the vāṇी object in the
   link command so usual link-order rules apply (vāṇी's
   `call @triple` discovers the providing symbol from
   the helper that follows it).

   Verified end-to-end via new
   `build_link_with_resolves_extern_c_symbol` test:
   spawns `intentc build … --link-with helper.c`, runs
   the binary, asserts stdout contains `triple(7) = 21`.

   Test totals: 981 lib + 48 e2e + 11 vtables-phase3 + 2
   user-drop-by-ref + 1 ssa-examples. Closure #270.

   **FFI v3 — `pure extern "C" fn` opt-in marker done 2026-05-27**:
   the purity side of FFI. `pure extern "C" fn name(...) -> R;`
   declares an FFI symbol that vāṇी's effect tracker treats as
   pure, so `pure fn` bodies (and parallel-for bodies, once
   those gain extern-call support) can call it. The caller
   asserts the foreign symbol is actually pure — no side
   effects, no shared state, deterministic output — vāṇी can't
   verify across the FFI boundary.

   Changes:
     • Parser — top-level dispatch peeks `Pure` followed by
       `Extern`, consumes the `Pure` token, calls
       `parse_extern_fn`, and sets `is_pure = true` on the
       returned Function.
     • Checker — `Signature` carries an `is_extern: bool`
       mirror of the function's flag so the impurity
       diagnostic can phrase the suggested marker as
       `pure extern` for extern callees, `pure fn`
       otherwise. The existing `sig.is_pure` check naturally
       admits `pure extern` callees without a separate code
       path.
     • Formatter — emit `pure ` before `extern "C" ` (not
       after), so round-trip preserves the canonical ordering.

   Verified: a `pure fn` body now compiles a call to a
   `pure extern "C" fn` (LLVM + C backends both). Without
   the `pure` marker, the same call surfaces the existing
   purity diagnostic with the corrected "mark it `pure extern`"
   hint.

   New tests: 2 lib tests (`pure_extern_c_fn_parses_and_a_pure_
   fn_can_call_it`, `impure_extern_rejected_from_pure_fn_with_
   pure_extern_hint`).

   Test totals: 983 lib + 48 e2e + 11 vtables-phase3 + 2
   user-drop-by-ref + 1 ssa-examples. Closure #271.

   **Auto-borrow audit closed 2026-05-27 (no code, closure #272)**:
   sweep of `check_binary_op` / `check_equality` /
   `auto_borrow` to identify gaps where binary ops still
   consume unnecessarily. Findings:

     • Str/OwnedStr comparisons + concat already covered.
     • Eq impl with `ref T` self auto-borrows Var operands
       (closure #228); non-Var operands surface a clear
       let-bind hint. Workaround is one line — not worth a
       feature ship.
     • Eq impl with `T` by-value self consumes by design.
     • Numeric / boolean / bitwise ops are Copy-only; no
       move surface.
     • No "Vec arithmetic" gap exists; the original TODO
       entry was speculative.

   Conclusion: the binary-op auto-borrow surface is tight.
   No code changes shipped under this closure; see TODO.md
   for the full audit notes. Test totals unchanged.

   **FFI v4 — reject non-FFI-safe types in extern signatures done 2026-05-27**:
   silent ABI corruption guard. While probing
   `extern "C" fn point_sum(p: Point) -> i32;` against a C
   helper, the resulting binary returned `3` instead of `7`
   for `Point { x: 3, y: 4 }`: vāṇी's LLVM emit produces
   `declare i32 @point_sum(%Struct_Point)` which doesn't
   match cc's System V x86-64 ABI lowering for small
   aggregates (packed-register layout). Same risk applies to
   tuples, arrays, enums by value.

   Solution: checker rejects unsupported FFI shapes at the
   extern declaration site with a `ref T` migration hint.

     • `extern_param_rejection_hint(ty)` and
       `extern_return_rejection_hint(ty)` classify safe
       vs unsafe FFI shapes:
         - **Safe**: scalars (i8..i64, u8..u64, f32/f64,
           bool), `Str` (`i8*`), and any `ref T` / `mut ref T`.
         - **Unsafe (by value)**: Struct, Tuple, Array, Enum
           — migration hint "write `ref T`".
         - **Forbidden**: `Vec<T>`, `OwnedStr`, exclusive
           handles (Atomic / Mutex / Channel / Guard /
           Task / Fn / Object). Cross-language heap
           semantics don't survive.
         - **Forbidden**: `Type::Param` (generic parameters
           on extern fns).
     • Return type validation slightly stricter: refuses
       struct/enum/tuple/array by value with the same
       `ref T` migration hint.
     • Also fixed: `is_pure` flag on the extern's
       TypedFunction was being hard-coded to false; now
       preserves `function.is_pure` (closure #271's value)
       for consistency.

   Verified: `extern "C" fn ... (p: ref Point)` still type-
   checks, and the AOT-linked binary returns the correct
   `sum = 7`. Direct by-value declaration now surfaces a
   clear diagnostic.

   New tests: 4 lib tests (struct-by-value rejected, Vec
   rejected, struct-by-ref accepted, struct return rejected).

   Test totals: 987 lib + 48 e2e + 11 vtables-phase3 + 2
   user-drop-by-ref + 1 ssa-examples. Closure #273.

   **FFI v5 — `intentc run --link-with` ergonomic parity done 2026-05-27**:
   `intentc run --backend=c` now accepts the same
   `--link-with PATH` / `-l<name>` flags as `intentc build`, so
   FFI iteration doesn't require switching to a separate build
   step. LLVM-JIT path keeps host-symbol resolution as-is and
   surfaces a clear "require --backend=c" diagnostic if a user
   tries `--link-with` on the LLVM path (lli can't link static
   translation units; the LLVM-JIT runtime auto-resolves
   libc/libm from the host process).

   New `parse_run_args` helper mirrors `parse_build_args`'s
   shape; `run_program` (C backend) takes the link_args slice
   and appends each flag after the vāṇी source on the cc
   command line.

   2 new e2e tests pin the shape: positive
   (`run_link_with_resolves_extern_c_symbol_in_run_mode`) and
   negative (`run_link_with_requires_backend_c`).

   Test totals: 987 lib + 50 e2e + 11 vtables-phase3 + 2
   user-drop-by-ref + 1 ssa-examples. Closure #274.

   **Parallel-for purity gate covers reduction RHS done 2026-05-27**:
   discovered a real silent-acceptance bug while validating
   the FFI v3 follow-up: `total = total + rand();` inside a
   `parallel for ... reduce total with +;` body was accepted
   even though `rand` is impure. Root cause:
   `strip_reduction_uses` replaced approved reduction
   reassigns with `Discard 0`, which swallowed the entire
   non-self subexpression — including any impure calls
   hidden inside it.

   Fix:
     • Refactored `validate_reduction_rhs(name, expr, op) -> bool`
       into `extract_reduction_other_side(name, expr, op) ->
       Option<TypedExpr>`. Returns the non-self subexpression
       (the X in `total + X` / `total * X` /
       `min(total, X)`, etc.) for valid shapes; None for
       invalid.
     • `strip_reduction_uses` now emits
       `Discard { expr: other_side }` instead of
       `Discard 0`, so the pure-body walker still sees X
       and surfaces any impurity inside it.

   2 new lib tests pin both shapes:
     - `pure_extern_in_parallel_for_body_accepted` (positive)
     - `impure_extern_in_reduction_rhs_rejected` (negative)

   Test totals: 989 lib + 50 e2e + 11 vtables-phase3 + 2
   user-drop-by-ref + 1 ssa-examples. Closure #275.

   **Vtables Phase 3 v1.1 — non-Var DynCoerce in let-RHS done 2026-05-27**:
   `let v: dyn Iface = make_thing(...);` now compiles
   (previously panicked at codegen with "non-Var source is
   pending"). The checker hoists the non-Var source into a
   synthetic let inside a `TypedExprKind::Block { stmts, tail
   }`; the C backend's stmt-level Let detects the
   synthetic shape (Block with DynCoerce tail) and unfolds
   the prelude stmts to the OUTER scope so the temp's
   storage survives the GCC stmt-expr's lifetime. LLVM emits
   the synthetic let through its normal Block emit path
   (alloca is function-scoped, so no special unfold needed).

   Changes:
     • Checker — new `make_dyn_coerce` helper that emits a
       direct `DynCoerce` for Var sources and a
       `Block { Let __dyn_src_<N> = src; DynCoerce(Var(...)) }`
       wrapper for non-Var sources. Process-wide
       `AtomicUsize` counter for synthetic names.
     • Tree-C — `TypedStmt::Let` detects the synthetic-shape
       Block-RHS pattern (Block whose tail is DynCoerce) and
       unfolds the prelude to the outer scope, preserving
       the temp's lvalue lifetime across the fat pointer's
       data slot read.

   Limitation: Vec literal elements with non-Var sources are
   still rejected (because the synthetic Block as a call-arg
   would die before vec(...) consumes it). The reject now
   surfaces a clear let-bind hint instead of silently
   producing wrong results — previously the codegen
   path-panicked. Fixing Vec-literal would need a higher-
   level statement-context hoist (deferred).

   2 new lib tests:
     - `dyn_coerce_from_call_result_in_let_rhs_compiles` —
       both backends must produce a compiling artifact for
       the Let-RHS case.
     - `dyn_coerce_in_vec_literal_rejects_non_var_with_letbind_hint` —
       Vec-literal still rejects, with the let-bind hint.

   Test totals: 991 lib + 50 e2e + 11 vtables-phase3 + 2
   user-drop-by-ref + 1 ssa-examples. Closure #276.

   **User-Drop fires on `let _ = make_struct()` discard done 2026-05-27**:
   probing the "remaining work" #5 from the work queue
   revealed the stale TODO entry (closures #102 / #103 /
   #105 had already shipped partial-move tracking, multi-
   field drop order, and Mutex/Guard struct fields). But
   one real correctness gap surfaced: when a struct with a
   user-declared `Drop` impl was discarded via `let _ =
   make_struct();`, both backends ran the per-field free
   chain but silently SKIPPED the user's drop method.
   End-of-scope drop already fired user-Drop correctly —
   only the discard path was broken.

   Fix: both backends' `TypedStmt::Discard { expr }` arms
   for `Type::Struct` now consult `USER_DROP_REGISTRY` /
   `LLVM_USER_DROP_REGISTRY` and dispatch to the user's
   drop function. Two shapes mirror the scope-exit Drop
   path:
     • By-value `fn drop(self: T)` with no owning fields
       — user-drop consumes the discarded value, per-field
       pass skipped.
     • By-ref `fn drop(self: mut ref T)` — spill the rvalue
       to a tmp alloca, call user-drop with the address,
       then per-field cleanup.

   2 new lib tests pin the fix (C and LLVM backends).

   Test totals: 993 lib + 50 e2e + 11 vtables-phase3 + 2
   user-drop-by-ref + 1 ssa-examples. Closure #277.

   **Match on f64 / f32 scrutinees done 2026-05-27**: new
   `Pattern::Float(f64)` AST variant; parser accepts float
   literal patterns (with optional `-` prefix); checker
   dispatches f32/f64 scrutinees to `check_match_float`
   (modeled on `check_match_str`) which desugars to a
   nested IfExpr chain keyed on `scrut == lit_arm`, with
   the wildcard body as the final else.

   Edge cases handled with clear diagnostics:
     • Missing wildcard: "non-exhaustive match: float
       scrutinees require a wildcard `_ then …` arm".
     • Duplicate float literal: "match arm for float
       pattern '…' appears twice" (compared via `f.to_bits()`
       so the dedupe handles negative zero correctly).
     • NaN literal in pattern: "NaN match pattern never
       fires (IEEE 754) — use a guard like `if x.is_nan()
       { … }` or fall through to the wildcard".
     • Wrong pattern type (float pattern on int scrutinee,
       int pattern on float scrutinee): clear cross-type
       diagnostic.

   v1 limitation documented in the AST docstring: NaN
   scrutinees never match any literal arm (IEEE 754 says
   `NaN != NaN`), so they fall through to the wildcard.
   This is the standard Rust / OCaml / Swift behavior.

   Files touched: [src/ast.rs](src/ast.rs) (new variant),
   [src/parser.rs](src/parser.rs) (Float literal in
   patterns + optional unary minus),
   [src/checker.rs](src/checker.rs) (early dispatch +
   `check_match_float` helper + non-float scrutinee
   arm),
   [src/format.rs](src/format.rs) (round-trip Float
   patterns with `.0` suffix when missing),
   [src/smt.rs](src/smt.rs) (Unsupported arm for SMT
   encoding).

   4 new lib tests:
     - `match_on_f64_classifies_literals_then_falls_through_to_wildcard`
     - `match_on_f64_without_wildcard_rejected`
     - `match_on_f64_with_nan_pattern_rejected` (covers
       duplicate-literal rejection — verified at the
       literal-equality level via bit-pattern comparison)
     - `match_on_f64_with_wrong_pattern_type_rejected`

   Test totals: 997 lib + 50 e2e + 11 vtables-phase3 + 2
   user-drop-by-ref + 1 ssa-examples. Closure #278.

   **FFI callbacks via fn-pointer extern params done 2026-05-27**:
   `extern "C" fn invoke_cmp(cmp: fn(i32, i32) -> i32, a: i32,
   b: i32) -> i32;` now compiles. The parser already accepted
   `fn(...) -> R` types in extern positions (#269); the
   checker's ABI gate (#273) was the blocker. Added
   `Type::FnPtr` to both `extern_param_rejection_hint` and
   `extern_return_rejection_hint` since function pointers
   are pointer-sized in both C ABI (function pointer) and
   LLVM (`R (T1, ...)*`), crossing the FFI boundary cleanly.

   Common shape — qsort-style callback:
     extern "C" fn qsort(base: ref u8, n: u64, sz: u64,
                          cmp: fn(ref u8, ref u8) -> i32);

   Verified end-to-end: vāṇी's `my_cmp` fn passed as
   callback to a separately-compiled `helper.c` returns the
   correct `cmp(5, 7) = -1` on both backends.

   Varargs (`...` in declarations) deferred: requires both
   the declaration shape AND variadic call-site syntax to be
   useful. Without variadic calls, declared `printf(...)`
   can't actually be called from vāṇी source. M+ tier.

   1 new lib test (`extern_fn_with_fn_pointer_param_accepted`)
   pins compile-time acceptance on both backends.

   Test totals: 998 lib + 50 e2e + 11 vtables-phase3 + 2
   user-drop-by-ref + 1 ssa-examples. Closure #279.

   **vani.toml manifest + auto-discovery done 2026-05-27**:
   foundational for multi-file projects and the future Kosh
   package manager. `intentc build|run|check|emit|ir|ast|
   tokens` invoked without a positional source file now walks
   up from cwd looking for a `vani.toml`. When found, its
   `[package].entry` key supplies the entry source.

   Minimal v1 manifest format:

       [package]
       name = "my_project"
       entry = "src/main.vani"

   New `src/manifest.rs` module: hand-rolled minimal TOML
   parser (no new dependency), `find_manifest(start)`
   parent-walk, `load_manifest(path) -> Manifest { name,
   entry_path, root_dir }`. 7 inline lib tests pin the
   parser shapes: minimal, comments+blanks, unknown section
   rejected, non-string value rejected, kv-outside-section
   rejected, missing-entry diagnostic, parent-walk
   discovery.

   Driver changes: new `required_file_at(args, idx, cmd) ->
   (PathBuf, next_idx)` that returns both the resolved
   source path AND the next index to scan from. When the
   positional file is present, returns `idx + 1`; when it
   came from manifest (no arg consumed), returns `idx` so
   flag parsing sees every remaining arg. Skips flag pairs
   (`-o PATH` / `--out PATH` / `--link-with PATH`) and
   standalone flags when looking for the positional.
   `run` / `build` dispatchers updated to use it.

   Future v2 additions queued (not in this closure):
   `[deps]` table for Kosh-registry packages, optional
   `[build]` knobs for backend default / opt level /
   `--link-with` defaults.

   2 new e2e tests
   (`manifest_discovery_resolves_entry_from_subdir`,
   `manifest_build_with_o_flag_finds_entry`) pin the
   end-to-end shape including parent-walk + flag
   interleaving.

   Test totals: 1005 lib + 52 e2e + 11 vtables-phase3 + 2
   user-drop-by-ref + 1 ssa-examples. Closure #280.

   **Generic struct/enum declarations done 2026-05-27**:
   `enum Option<T> { Some(T), None }` and `enum Result<T, E>
   { Ok(T), Err(E) }` now compile end-to-end on both
   backends. Mirrors closure #99's fn-generic
   monomorphization but for type declarations.

   Surface:

       enum Option<T> { Some(T), None }
       enum Result<T, E> { Ok(T), Err(E) }
       struct Pair<A, B> { first: A, second: B }

       fn main() -> i64 {
         let a: Option<i64> = Option.Some(42);
         return match a {
           Option.Some(v) then v,
           Option.None then 0,
         };
       }

   Pipeline:

     • AST — `StructDecl` and `EnumDecl` gain `type_params:
       Vec<String>` (empty for monomorphic decls). New
       `Type::Apply { name, args }` variant represents
       parse-time generic instantiations.
     • Parser — accepts `<T, E>` after struct/enum name;
       registers the params in `current_type_params` so
       field/variant payload types resolve as `Type::Param`.
       At type-position use-sites, `Name<T1, T2>` parses
       to `Type::Apply`.
     • Monomorphization pre-pass
       (`monomorphize_type_decls_in_program`) — runs BEFORE
       the fn-generic pass. Walks every type in the
       program, collects `Type::Apply` use-sites, generates
       one monomorphic `EnumDecl` / `StructDecl` per unique
       (template, args) tuple with a mangled name
       (`Result__i64__OwnedStr`), and rewrites every
       `Type::Apply` into the corresponding
       `Type::Struct(mangled)` / `Type::Enum(mangled)`.
     • Checker — `lookup_enum` and `resolve_enum_name`
       accept the unmangled base name (`Option`) when
       exactly ONE monomorphic instantiation exists in the
       program. So `Option.Some(42)` resolves to
       `Option__i64.Some(42)` without the user spelling out
       the mangled name. Multiple instantiations require
       the user to disambiguate (or future expected-type
       threading).
     • Match patterns — accept the unmangled base name
       similarly: `Result.Ok(v)` matches a scrutinee of
       type `Result__i64__OwnedStr` (prefix-match).
     • Codegen — backends never see `Type::Apply` (rewritten
       to mangled `Type::Struct` / `Type::Enum` by the
       pre-pass).

   v1 limitations documented:
     • Multiple monomorphic instantiations of the same
       generic in one program force the user to spell the
       mangled name explicitly (e.g. `Option__i64.Some(0)`
       vs `Option__OwnedStr.Some("x" + "")`). Future:
       expected-type threading through expression
       checking.

   3 new lib tests pin the shape:
     - `generic_option_with_i64_payload_compiles_both_backends`
     - `generic_result_two_type_params_compiles`
     - `generic_enum_with_mismatched_arg_count_rejected`

   Test totals: 1008 lib + 52 e2e + 11 vtables-phase3 + 2
   user-drop-by-ref + 1 ssa-examples. Closure #281.

   **Prelude — auto-import of Option / Result / AllocError done 2026-05-27**:
   piggybacking on the new generic-decl machinery (#281),
   every program now implicitly receives three enum
   declarations without the user needing to write them:

       enum Option<T> { Some(T), None }
       enum Result<T, E> { Ok(T), Err(E) }
       enum AllocError { OutOfMemory }

   Injected at the AST level (NOT as a source prepend) so
   user diagnostic spans / line numbers stay anchored to
   the user's actual source text. `inject_prelude` parses
   the prelude string into a Program, then merges its
   `enums` into the user's parsed Program — skipping any
   prelude enum the user has already declared by name, so
   user redeclarations override the prelude shape (with no
   "duplicate declaration" error).

   3 new lib tests:
     - `prelude_provides_option_without_user_declaration`
     - `prelude_provides_result_without_user_declaration`
     - `user_redeclaration_of_option_overrides_prelude`

   Test totals: 1011 lib + 52 e2e + 11 vtables-phase3 + 2
   user-drop-by-ref + 1 ssa-examples. Closure #282.

   **Mixed-payload-type enum lift (C backend) done 2026-05-27**:
   v1's restriction "all payload-bearing variants must share
   the payload type" is lifted on the C backend, unblocking
   `Result<T, E>` with T != E and (once LLVM is also lifted)
   #6 try_vec.

   Surface: any enum can now mix payload types across
   variants:

       enum R { Ok(i64), Err(OwnedStr) }      // works
       enum Result<T, E> { Ok(T), Err(E) }    // works for all T, E

   Pipeline (C backend):

     • New `ENUM_VARIANT_PAYLOADS_REGISTRY: HashMap<String,
       Vec<(variant_name, Option<Type>)>>` carries per-variant
       payload info. Existing single-type registry kept for
       back-compat with the legacy `{ tag; T payload; }`
       layout when all payload-bearing variants agree.
     • Mixed-payload enums emit the new layout:
         typedef struct {
             int32_t tag;
             union {
                 <Type0> v_<Variant0>;
                 <Type1> v_<Variant1>;
                 …
             } u;
         } Enum_<Name>;
     • Variant construction emits `(Enum_X){ .tag = T, .u = {
       .v_<variant> = <payload> } }` for mixed-payload, legacy
       `.payload = <payload>` for single-payload.
     • Match-extract emits `__scr.u.v_<variant>` for mixed,
       `__scr.payload` for single.
     • New `enum_has_mixed_payloads(decl)` helper routes
       between the two paths at every codegen site.

   Limitation queued: **LLVM backend** still uses the legacy
   `{ i32, T }` layout and panics with a clear "use
   --backend=c" message when a mixed-payload enum reaches
   it. Lifting requires byte-buffer `[N x i8]` payload + per-
   variant bitcast at every variant access (~15 sites).
   Tracked as the immediate follow-up to this closure.

   1 new lib test
   (`mixed_payload_enum_compiles_on_c_backend`) validates
   both the typedef shape and the variant-construction emit.

   Test totals: 1012 lib + 52 e2e + 11 vtables-phase3 + 2
   user-drop-by-ref + 1 ssa-examples. Closure #283.

   **Mixed-payload-type enum lift (LLVM backend) done 2026-05-27**:
   completing closure #283 — `Result<T, E>` with T != E now
   works on the LLVM backend too. Both backends fully
   support mixed-payload enums end-to-end.

   LLVM pipeline:

     • New `LLVM_ENUM_VARIANT_PAYLOADS_REGISTRY` (mirror of
       the C-side per-variant registry). New `llvm_byte_size(ty)`
       helper (target Linux x86-64 best-effort sizing) and
       `llvm_enum_payload_buffer_size(decl)` for max-payload
       sizing rounded to 8-byte alignment.
     • Mixed-payload enums emit
       `%Enum_<Name> = type { i32, [N x i8] }` (byte buffer
       payload) where N = max(payload_size).
     • Variant construction (EnumVariant + EnumVariantWithPayload):
       alloca the struct, GEP to tag + byte-buffer fields,
       bitcast `i8*` → `<PayloadTy>*`, store payload, load
       whole struct back.
     • Match-extract (VariantWithBinding arms): spill scrutinee
       to alloca, GEP into byte buffer, bitcast to variant's
       payload type, load.
     • Single-payload enums keep the legacy `{ i32, T }`
       layout for back-compat.

   Verified end-to-end: `enum R { Ok(i64), Err(OwnedStr) }`
   round-trips a value through R.Ok(42), match returns 42 on
   both backends.

   1 new lib test (`mixed_payload_enum_compiles_on_llvm_backend`)
   pins the LLVM emit shape. The defensive
   `panic!("use --backend=c")` is removed.

   Test totals: 1013 lib + 52 e2e + 11 vtables-phase3 + 2
   user-drop-by-ref + 1 ssa-examples. Closure #283 complete
   (both halves).

   **try_vec(n) -> Result<Vec<i64>, AllocError> done 2026-05-27 (C backend)**:
   first fallible allocation API. Builds on closures #281
   (generic decls), #282 (Result prelude), #283 (mixed-
   payload enums) — all the foundation needed for an
   idiomatic OOM-tolerant builtin.

   Surface:

       fn main() -> i64 {
         let r: Result<Vec<i64>, AllocError> = try_vec(10 as u64);
         return match r {
           Result.Ok then 0,    // alloc succeeded
           Result.Err then 1,   // alloc failed
           _ then 2,
         };
       }

   Pipeline:

     • Checker — new `check_try_vec_builtin`. Validates a
       single u64 arg, returns
       `Type::Enum("Result__Vec_I64___AllocError")` (the
       mangled monomorphic name produced by closure #281's
       pass).
     • C codegen — special-case `Call { name: "try_vec" }`
       to emit a GCC statement-expression doing
       malloc-with-null-check + Result construction. AllocError
       (payload-less enum) lowers to `int32_t`.
     • Scope-exit Drop for mixed-payload enums — closure
       #283's drop-dispatch follow-up landed alongside #284.
       For mixed-payload enum scope-exit Drop, the C
       backend now emits one `switch (tag)` case per
       owning variant, reading through the correct
       `.u.v_<variant>` member.

   LLVM (deferred): panics with a clear "use --backend=c"
   message at try_vec call sites. The codegen requires
   if/else basic blocks for the Result construction; queued
   as a follow-up.

   1 new lib test
   (`try_vec_returns_result_vec_on_c_backend`) pins the
   malloc + null-check + Result-construction emit.

   Test totals: 1014 lib + 52 e2e + 11 vtables-phase3 + 2
   user-drop-by-ref + 1 ssa-examples. Closure #284.

   **try_vec LLVM half done 2026-05-27**: completes closure
   #284 — try_vec(n) now works on both backends.

   LLVM pipeline:

     • Call emit detects `name == "try_vec"` and lowers to
       basic-block IR: alloca result struct → call malloc →
       null-check via `icmp eq i8* %raw, null` → branch to
       try_vec_ok / try_vec_err labels.
     • Ok block: build Vec<i64> {data, len=0, cap=n} via
       insertvalue, write tag=0 to result, bitcast result's
       `[N x i8]` payload buffer to `%intent_vec_i64*` and
       store the Vec.
     • Err block: write tag=1 to result; AllocError has no
       payload so nothing to store in the buffer.
     • Merge block: load result struct.
     • Vec typedef emission now walks enum-variant payload
       types + struct field types (in addition to the
       existing fn-walking) so `Vec<i64>` inside a
       `Result<Vec<i64>, AllocError>` triggers its typedef
       even when the user never spells `Vec<i64>` directly.
     • Scope-exit Drop for mixed-payload enums: LLVM now
       emits per-variant `icmp eq i32 tag, T` + branch +
       bitcast-on-buffer + free, matching the C-side closure
       #283 fix.

   1 new lib test (`try_vec_returns_result_vec_on_llvm_backend`)
   pins the LLVM emit shape. The defensive `panic!("use
   --backend=c")` for try_vec is removed.

   Test totals: 1015 lib + 52 e2e + 11 vtables-phase3 + 2
   user-drop-by-ref + 1 ssa-examples. Closure #284 complete
   (both backends).

   **FFI v6 — small struct-by-value on C backend done 2026-05-27**:
   the silent-corruption guard from closure #273 is lifted
   for all-integer-field structs ≤ 16 bytes. Common case:
   `struct Point { x: i32, y: i32 }` passed by value to a C
   helper now works correctly via cc's native System V
   x86-64 packed-register handling.

   New checker helpers:

     • `is_ffi_integer_class(ty)` — true for scalar integers,
       bool, refs, Str.
     • `ffi_byte_size(ty)` — best-effort byte size per type.
     • `is_ffi_safe_struct(name, structs)` — struct passes by
       value iff all fields are integer-class AND total size
       ≤ 16 bytes.
     • `extern_param_rejection_hint` and
       `extern_return_rejection_hint` now consult these to
       carve out the FFI-safe subset.

   Backend status:

     • **C backend** ✓ — cc handles ABI natively. Just emits
       the struct as a parameter type; the C compiler does
       packed-register passing automatically.
     • **LLVM backend** — panics with a clear "use
       --backend=c" message at extern struct param/return
       sites. Proper LLVM ABI lowering (emit `declare RET
       @name(i64)` for size-8 structs and `{i64, i64}` for
       size-9..16, with bitcast at call sites) is queued as
       a follow-up.

   2 updated lib tests + 1 new:
     - `extern_struct_by_value_param_rejected_with_ref_hint`
       now uses a mixed-type struct (i32 + f64) to test the
       rejection path.
     - `extern_struct_return_rejected_with_ref_hint` likewise.
     - `extern_small_integer_struct_by_value_accepted` (new)
       confirms the happy path for FFI-safe structs.

   Test totals: 1016 lib + 52 e2e + 11 vtables-phase3 + 2
   user-drop-by-ref + 1 ssa-examples. Closure #285.

   **#[bounded(N)] recursion-depth attribute done 2026-05-27 (C backend)**:
   first attribute syntax in vāṇी. Caps fn recursion depth
   at runtime — exceeding N aborts with a diagnostic.
   Caller's responsibility: pick a sane N.

   Surface:

       #[bounded(100)]
       fn factorial(n: i64) -> i64 {
         if n <= 1 { return 1; }
         return n * factorial(n - 1);
       }

   Pipeline:

     • Lexer — new `TokenKind::Hash` for `#`.
     • AST — `Function.recursion_bound: Option<u64>`. New
       `parse_attributed_fn` recognizes `#[bounded(N)]`
       before a fn decl. Unknown attribute names surface
       a "not recognized in v1" diagnostic.
     • IR / SSA — `recursion_bound` threaded through
       `TypedFunction` and SSA `Function`.
     • C backend (tree + SSA) — emits a thread-local depth
       counter `__intent_depth_<fn>` + GCC
       `__attribute__((cleanup))` helper that decrements on
       every exit path (return, fall-through). Bound check
       at entry: `if (++counter > N) abort()`.

   Backend status:

     • C ✓ — works on both tree-C and SSA-C paths via
       `intentc run --backend=c` and `intentc build` on the
       C output. Verified: bound exceeded aborts with exit
       134 (SIGABRT) + stderr diagnostic.
     • LLVM — panics with a clear "use --backend=c" message
       at fn-emit time on both tree-LLVM and SSA-LLVM (the
       latter returns an `EmitError` to take the tree-LLVM
       fallback path, which also panics — uniform user
       experience). Lifting LLVM requires either GCC-cleanup-
       like instrumentation (which LLVM lacks directly) or
       ret-instruction interception. Queued as a follow-up.

   2 new lib tests
   (`bounded_attribute_emits_depth_counter_on_c_backend`,
   `bounded_attribute_unknown_name_rejected`).

   Test totals: 1018 lib + 52 e2e + 11 vtables-phase3 + 2
   user-drop-by-ref + 1 ssa-examples. Closure #286.

   **vani.toml v2 — [deps] table with local paths done 2026-05-27**:
   foundation for future Kosh package management. Manifest
   v2 adds:

       [deps]
       mathlib = { path = "../math-lib" }
       other_lib = { path = "../other" }

   Local-path deps are resolved transitively at compile time:
   the driver reads each dep's `vani.toml`, finds its entry
   source, and prepends it (and any of its `use "..."`
   resolves) to the main entry's combined source before
   type-checking. Functions / types declared in the dep are
   directly callable from the main entry without a
   namespace prefix in v1.

   Pipeline:

     • `manifest::Manifest` gains `deps: Vec<Dependency>`.
     • `manifest::Dependency { name, entry_path }`.
     • Parser accepts the inline-table form `name = { path =
       "..." }`. Unknown keys (e.g. `version = "1.0"`)
       surface a "only `path` is recognized in v1" hint —
       reserves the syntax for future Kosh registry coords.
     • `compile_path` walks `find_manifest` from the entry's
       parent dir; for each resolved dep, calls
       `resolve_uses` to inline the dep's source into the
       combined buffer.
     • Recursive dep loading: a dep with its own `[deps]`
       cascades. Shared visited set prevents cycles /
       diamond duplication.

   2 new lib tests (`parses_deps_inline_table`,
   `rejects_unknown_key_in_inline_table`) and 1 e2e test
   (`manifest_deps_local_path_brings_lib_into_scope`).

   Future v3 follow-ups queued:
   - Kosh registry coords: `name = "1.0"` shorthand.
   - Namespacing: `mathlib::triple` per-dep prefix.
   - `[build]` knobs (default backend, opt level,
     `--link-with` defaults).

   Test totals: 1020 lib + 53 e2e + 11 vtables-phase3 + 2
   user-drop-by-ref + 1 ssa-examples. Closure #287.

   **FFI v7 — LLVM small-struct ABI lowering done 2026-05-27**:
   completes the LLVM half of closure #285. Small
   all-integer-field structs ≤ 16 bytes now cross the FFI
   boundary correctly via the LLVM backend, matching cc's
   System V x86-64 packed-register passing.

   Lowering:

     • Struct size ≤ 8 bytes → `i64`.
     • Struct size 9..=16 bytes → `{ i64, i64 }`.
     • Larger / mixed payloads → not yet supported (checker
       still rejects upstream).

   Pipeline:

     • `llvm_ffi_struct_lowered_ty(ty)` — maps a vāṇी
       struct type to the lowered LLVM form per the rules
       above.
     • `emit_function` (extern path) — declare emits the
       lowered form for struct params + return.
     • Call-site emit — for each struct arg to an extern,
       spill to alloca, bitcast to lowered ptr type, load,
       pass the loaded value.
     • Return-type lowering — call's result type uses the
       lowered form for extern returns.

   Verified end-to-end via `intentc build` of a vāṇी
   program calling `point_sum({x: 3, y: 4})` against a C
   helper — both backends now return 7.

   1 new lib test
   (`extern_small_struct_lowers_to_packed_integer_in_llvm`).
   The defensive `panic!("use --backend=c")` for struct FFI
   is removed.

   Test totals: 1021 lib + 53 e2e + 11 vtables-phase3 + 2
   user-drop-by-ref + 1 ssa-examples. Closure #288.

   **#[bounded(N)] LLVM lift done 2026-05-27 (SSA-LLVM)**:
   completes the LLVM half of closure #286 for the default
   `intentc run` / `build` path. Tree-LLVM continues to
   panic with a clear "SSA-LLVM supports this" message (the
   tree-LLVM lift is a smaller follow-up).

   SSA-LLVM pipeline:

     • Module-level `@__intent_depth_<fn> = thread_local
       global i32 0`.
     • Entry sequence (in the implicit entry block, before
       the first labeled block): load counter → +1 → store
       back → cmp > N → branch to `__bd_abort` or
       fall-through to the entry block.
     • `__bd_abort` block emits `call void @abort()` +
       `unreachable`.
     • Before each `Terminator::Return` emit, inject `load
       counter → -1 → store`. Per-block names use the
       block id to avoid LLVM register collisions when the
       function has multiple Return blocks.

   Verified end-to-end via new e2e test: `#[bounded(3)] fn
   deep(...)` called as `deep(10)` aborts with SIGABRT.
   Bound = 5 + `deep(3)` exits 3.

   1 new e2e test pins the abort behavior with both
   `code()` and Unix `signal()` checks for portability.

   Test totals: 1021 lib + 54 e2e + 11 vtables-phase3 + 2
   user-drop-by-ref + 1 ssa-examples. Closure #289.

   **#[bounded(N)] tree-LLVM follow-up done 2026-05-27**:
   tree-LLVM now emits the same depth-counter
   instrumentation as SSA-LLVM. Both LLVM paths handle
   `#[bounded(N)]` end-to-end.

   Tree-LLVM pipeline:

     • Module-level thread-local global emitted just before
       the fn's `define`.
     • Entry sequence at fn opening: load → +1 → store →
       cmp > N → branch to `__bd_abort` or fall-through to
       a fresh label that holds the body.
     • New `FnCtx.bounded_fn_name: Option<String>` carries
       the fn name into Return-statement codegen so the
       per-return decrement knows which counter to touch.
     • `TypedStmt::Return` emit checks `ctx.bounded_fn_name`
       and emits `load → -1 → store` before the actual
       `ret`.

   The defensive `panic!("SSA-LLVM supports this")` for
   tree-LLVM is removed. 1 lib test
   (`bounded_attribute_emits_depth_counter_on_llvm_backend`)
   pins the tree-LLVM emit shape — restored from a previous
   removal now that tree-LLVM works.

   Test totals: 1022 lib + 54 e2e + 11 vtables-phase3 + 2
   user-drop-by-ref + 1 ssa-examples. Closure #290.

   **Nested arrays — Copy-restriction lifted done 2026-05-27**:
   `[Vec<i64>; N]`, `[OwnedStr; N]`, and other non-Copy
   element arrays now type-check. References as element
   types remain rejected (dangling-pointer risk).

   Pipeline:

     • Checker — `validate_array_element_type` and the
       array-literal check no longer reject non-Copy
       elements. The "must be Copy" diagnostic is replaced
       by a "cannot be a reference" guard that catches the
       genuinely-unsafe case.
     • `clone_at` builtin — both backends extended to
       accept arrays alongside Vec. C lowers to `xs[i]`
       (with through-ref decay handling); LLVM emits
       `getelementptr [N x T], [N x T]*, i64 0, i64 idx`
       followed by a per-element-type clone (load for
       Copy elements; vec __clone for Vec elements).

   Verified end-to-end on C backend:
   `[Vec<i64>; 2] xs = [vec(1, 2), vec(3, 4)]; return
   len(clone_at(ref xs, 0));` returns 2.

   Remaining work queued:
   - **Bare `xs[i]` rejection for non-Copy element**:
     mirrors Vec's restriction. Today bare indexing on a
     non-Copy array slot would silently move from the
     array, leaving a dangling slot. Should reject with a
     `clone_at(ref xs, i)` hint.
   - **Per-slot drop at scope exit**: arrays of OwnedStr /
     Vec / nested-struct need each slot freed when the
     array binding goes out of scope. Today only Copy
     elements (no heap) are correct at scope exit.
   - **LLVM end-to-end pass for nested-array-of-Vec**: the
     LLVM IR for nested arrays runs but len() of the
     cloned slot returns 0 (likely a Vec field-access
     issue when the Vec comes from a nested-array
     `clone_at`). Tree-LLVM has the path; SSA-LLVM falls
     back. Debug + fix queued.

   2 new lib tests
   (`clone_at_accepts_array_argument`,
   `nested_array_of_vec_compiles_on_c_backend`). 1 test
   updated (`clone_at_rejected_on_non_vec_collection` →
   `clone_at_accepts_array_argument`).

   Test totals: 1023 lib + 54 e2e + 11 vtables-phase3 + 2
   user-drop-by-ref + 1 ssa-examples. Closure #291 (Phase 1).

   **Nested arrays Phase 2 + 3 done 2026-05-27**: end-to-end
   on both backends, including per-slot drop at scope exit.

   Phase 2 confirmed: the existing index check at
   `check_index` already rejects bare `xs[i]` for non-Copy
   element types uniformly across Vec and arrays — the
   diagnostic suggests `clone_at(&xs, i)`. No code change
   needed; just verified with a probe.

   Phase 3 — codegen:

     • **C backend** — `TypedStmt::Drop` for
       `Type::Array { element, length }` with non-Copy
       element emits per-slot free. Vec elements call the
       per-element `intent_vec_<T>__free` helper; OwnedStr
       elements free the buffer directly. Nested-struct
       slots are punted with a TODO for Phase 4.
     • **LLVM backend** — `TypedExprKind::Len` for a Vec
       rvalue (e.g. result of `clone_at(ref xs, i)` on a
       `[Vec<T>; N]`) now spills to alloca, GEPs to `.len`,
       loads. Previously fell through to the static
       `length` baked into Len { length }, which is 0 for
       Vec — silently producing wrong results.

   Verified end-to-end on both backends:
   `[Vec<i64>; 2] xs = [vec(1, 2), vec(3, 4)]; return
   len(clone_at(ref xs, 0));` returns 2.

   1 new lib test (`nested_array_of_vec_compiles_on_llvm_backend`).

   Test totals: 1024 lib + 54 e2e + 11 vtables-phase3 + 2
   user-drop-by-ref + 1 ssa-examples. Closure #291 complete
   (Phases 1+2+3).

   **Nested arrays Phase 4 — struct slot per-field drops done 2026-05-27**:
   arrays of structs whose fields own heap (OwnedStr, Vec)
   now correctly free per-slot per-field at scope exit on
   the C backend.

   `TypedStmt::Drop` for `Type::Array { element: Type::Struct,
   length }` walks each slot, looks up the struct's field
   list via `STRUCT_FIELDS_REGISTRY`, and calls
   `emit_struct_field_drops` with the slot's address
   (`v_bags[0]`, `v_bags[1]`, …).

   Verified on:
       struct Bag { name: OwnedStr, count: i64 }
       let bags: [Bag; 2] = [
         Bag { name: "first" + "", count: 1 },
         Bag { name: "second" + "", count: 2 },
       ];
   Each slot's `.name` OwnedStr is now freed at scope exit
   (`free((void*)v_bags[0].name)` + `free((void*)v_bags[1].name)`).

   1 new lib test
   (`array_of_struct_with_owning_fields_drops_each_slot_on_c`)
   pins the per-slot per-field emit shape.

   Test totals: 1025 lib + 54 e2e + 11 vtables-phase3 + 2
   user-drop-by-ref + 1 ssa-examples. Closure #291 fully
   complete (Phases 1-4).

   **Devanagari Sanskrit/Hindi/Marathi 3-way alias parity (Phase 2) done 2026-05-27**:
   pragmatic best-effort sweep of the lexer's alias table
   to give pure-Hindi / pure-Sanskrit / pure-Marathi
   programs full keyword coverage for the constructs that
   previously forced English fall-back. Added aliases:

     else        वरना (Hindi)
     mut         परिवर्तनीय (Sanskrit/Hindi)
     prove       प्रमाणित (Hindi/Marathi), दर्शाओ (Hindi),
                 दाखवा (Marathi)
     ensures     सुनिश्चयित (Sanskrit)
     true        सही (Hindi/Marathi colloquial)
     false       अशुद्ध (Hindi/Marathi colloquial)
     enum        गणन (Hindi/Marathi)
     const       नियत (Hindi/Marathi)
     continue    अग्रे (Sanskrit)
     parallel    समानांतर (single-word, all three)
     use         उपयोग
     module      खण्ड, मॉड्यूल
     pub         सार्वजनिक
     as          यथा
     interface   संकेत, अंतरापृष्ठ
     implement   कार्यान्वित
     methods     विधि
     where       जहाँ / यत्र / जिथे (Hindi/Sanskrit/Marathi)
     is          है / अस्ति / आहे (Hindi/Sanskrit/Marathi)
     try         प्रयास
     task        नियोग
     join        संयोजन

   Where a Sanskrit-root word is tatsama (loanword in
   Hindi/Marathi), it's documented as shared across the
   three languages rather than duplicated per-language.
   The picks are practical "best-effort, awaits consultant
   refinement" — the user can adjust specific verb
   choices later. Two new lib tests pin: a pure-Hindi
   program using `वरना` / `अग्रे` / `प्रमाणित` / `सही`,
   and a pure-Devanagari namespace declaration with
   `खण्ड` (module) / `सार्वजनिक` (pub) / `उपयोग` (use) /
   `यथा` (as). Test totals: 978 lib + 47 e2e + 11
   vtables-phase3 + 2 user-drop-by-ref + 1
   ssa-examples. Closure #267.

   **Devanagari SOV verb-at-end statements done 2026-05-26**:
   follows naturally from #265's SOV for-loop work. Hindi /
   Sanskrit / Marathi are verb-final ("my name is Ryan" →
   `मेरा नाम Ryan है`, verb at the end). Four verb-like
   statement forms now accept the verb-at-end shape:

     `X पुनरागम;`              (return X)
     `"x =", x लिखो;`          (print "x =", x)
     `cond सुनिश्चित;`        (assert cond)
     `cond, "msg" खात्री;`    (assert cond, "msg")
     `expr प्रमाण;`            (prove expr)

   A new helper `looks_like_sov_verb_at_end(&self) -> Option<TokenKind>`
   scans the current statement from `self.pos` to the next
   `;` at depth 0 (tracking parens / brackets / braces),
   and returns the verb-kind if the token immediately
   before `;` is one of Return / Print / Assert / Prove.
   `parse_stmt` runs the check BEFORE the assignment /
   discard branches so SOV statements take precedence.
   `parse_sov_verb_stmt(verb)` dispatches per-verb,
   re-using `parse_print_item` for the multi-item form
   and producing the same `Stmt::Return` / `::Print` /
   `::Assert` / `::Prove` AST as the English path.

   Five new lib tests cover each form individually plus a
   regression guard that English `return X;` /
   `print …;` etc. still compile (the SOV detector only
   fires when the leading token is NOT a verb-keyword).

   Test totals: 976 lib + 47 e2e + 11 vtables-phase3 + 2
   user-drop-by-ref + 1 ssa-examples. Closure #266.

   **Devanagari SOV word-order `for` loop done 2026-05-26**:
   the Phase-1 Devanagari aliases (`के लिए` = for,
   `से` = from, `तक` = to) already lexed correctly but
   the parser only accepted English word order — the
   awkward `के लिए i से 0 तक 5` (literally "for i
   from 0 to 5" with `से` PRECEDING its operand, which
   is grammatically wrong in Hindi/Sanskrit/Marathi).
   Natural Indo-Aryan grammar uses **postpositions**:
   noun first, then the marker. The compiler now ALSO
   accepts the natural shape:

     `i के लिए 0 से 5 तक { … }`     (range)
     `समान्तर प्रति i के लिए 0 से N तक संक्षेप X सह +; { … }`
                                     (parallel + reduce)

   Two new parser helpers detect the SOV variant:
   `looks_like_sov_for` (Ident immediately followed by
   For) and `looks_like_sov_parallel_for` (Parallel
   then Ident then For). When detected, `parse_stmt`
   routes to the new `parse_sov_for_stmt` which
   consumes:

       IDENT 'के लिए' START 'से' END 'तक'
       [invariants] [reductions] { body }

   AST shape produced is identical to the English form
   — the checker, SSA pass, and backends see no
   difference. Three new lib tests cover the SOV range
   form, regression-guard for the English form (still
   compiles), and the parallel-for SOV with reduce
   clause. English `for` users are unaffected.

   This closes the largest remaining Devanagari gap
   without a grammar consultant pass — the
   postpositional shape was always grammatically
   uncontroversial; the consultant pass for #29 is
   about verb-form parity across Sanskrit / Hindi /
   Marathi. Test totals: 971 lib + 47 e2e + 11
   vtables-phase3 + 2 user-drop-by-ref + 1
   ssa-examples. Closure #265.

   **SSA-LLVM multi-block parallel-for body emit done 2026-05-26**:
   the larger work item from the queue. SSA-LLVM now
   lowers `parallel for { if cond { acc = acc + i; } }`
   directly to atomicrmw in the outlined fn, without
   falling back to tree-LLVM. Three pieces landed
   together (closure #264):

   1. **Capture + body-defined analysis** extended to
      walk every block in `region.region_blocks` (not
      just `body_block`), including block-params. Without
      this, intermediate-block-local values would be
      misclassified as captures.

   2. **Phi-traceback reduction-update detection**: when
      the `update_v` value isn't a direct instruction in
      any region block, it's recognized as a Phi result
      on a region block's params; the new analysis walks
      predecessors of that Phi block (restricted to
      region_blocks), reads each predecessor's Jump/Branch
      arg at the relevant param-index, and if the
      contribution is itself a Binary/Call matching the
      reduction shape, records it as the update site. v1
      requires all sites in a region to use the same
      increment operand; divergent shapes surface a clear
      EmitError → tree-LLVM fallback as before.

   3. **Multi-block outlined-fn emit**: each region block
      becomes a labeled LLVM block `body_bb<N>:` with Phi
      nodes for params (predecessors restricted to
      region_blocks). Reduction-update instructions are
      intercepted at their actual production site (which
      may be in a conditional branch) and replaced with
      atomicrmw. The merge block's Jump-to-step
      terminator becomes `br label %body_end`. The Phi
      result at the merge block for the reduction's
      merged value is skipped (atomicrmw owns that
      value's effect — the Phi has no downstream user
      because the back-edge is replaced).

   Parent walk's `skip_blocks` extends to cover all
   region blocks so the parent fn doesn't emit orphan
   labels referencing the now-absorbed in-region
   blocks. Closure #252's fallback gate is gone —
   replaced by the new "ssa_llvm_multi_block_parallel_
   for_lowers_to_atomicrmw" lib test that asserts the
   outlined fn contains BOTH `body_bb<N>:` labels and
   `atomicrmw add i64*`.

   Test totals: 968 lib + 47 e2e + 11 vtables-phase3 +
   2 user-drop-by-ref + 1 ssa-examples — all green
   (the parity runner diffs both backends and they
   agree). Closure #264.

   **SSA-LLVM identity-cast uses `bitcast` for pointers 2026-05-26**:
   `emit_cast` previously emitted `add T 0, x` for any
   case where `from_llvm == to_llvm` (the "identity op"
   path used when two source-level types share a backing
   type, e.g. `i64` / `u64` both → `i64`). For pointer-
   typed identity (`OwnedStr → Str`, both `i8*`) LLVM
   rejected the IR with "integer constant must have
   integer type". Surfaced via a follow-up sanity sweep
   on top of #262 — passing OwnedStr to a `Str`-typed fn
   parameter is a common pattern that was silently
   broken on the SSA-LLVM path. Fix uses `bitcast T x
   to T` (a no-op) for pointer types (Str / OwnedStr /
   Vec / Ref / RefMut); integers and floats keep the
   `add 0` / `fadd 0.0` form. One lib test pins the
   bitcast shape AND adds a regression guard that no
   `add i8* 0` survives in the output. Test totals: 968
   lib + 47 e2e + 11 vtables-phase3 + 2 user-drop-by-
   ref + 1 ssa-examples. Closure #263.

   **Codegen fix — `len(ref OwnedStr)` 2026-05-26**:
   `len(ref s)` for `s: OwnedStr` had a 4-layer bug:
   (a) SSA lowerer routed it through the static-array
   path (which uses a `length: u64` field defaulting to
   0) instead of the strlen call, because the type
   match was on `array.ty` not `array.ty.deref()`;
   (b) SSA-LLVM intent_str_len emit passed the alloca
   address `i8**` straight to strlen which wants `i8*`,
   producing IR `lli` rejected; (c) SSA-C emitted
   `strlen(<ref expr>)` which compiled but read junk
   bytes (returned ≈ 6 on x86-64); (d) tree-C had the
   same junk-byte bug. The fix dereferences once when
   the operand's type is `Type::Ref(_) | Type::RefMut(_)`
   — SSA-LLVM emits `load i8*, i8** %x` first, SSA-C +
   tree-C wrap the operand with `(*<expr>)`. One lib
   test pins both shapes (`strlen((*…)` for C,
   `load i8*, i8** … call i64 @strlen` for LLVM).
   Surfaced by `examples/memory_safety.vani`; the
   example now uses `len(ref greeting)` cleanly and
   returns 12. Test totals: 967 lib + 47 e2e + 11
   vtables-phase3 + 2 user-drop-by-ref + 1
   ssa-examples. Closure #262.

   **examples/memory_safety.vani — canonical patterns demo done 2026-05-26**:
   added a single example that exercises the seven core
   memory-safety patterns end-to-end and is now wired into
   the cross-backend parity runner: (1) affine Vec ownership
   + move-into-callee, (2) explicit `clone(xs)` for the
   both-bindings-own-data case, (3) push/pop Stack pattern
   through `mut ref`, (4) `OwnedStr` automatic drop on scope
   exit, (5) user-defined `Drop` interface, (6) `parallel for`
   with `reduce total with +;`, (7) `task name { … }` +
   `join name;`. All seven compile + run on both backends
   and produce identical stdout. Surfaced one codegen gap
   along the way: `len(ref OwnedStr)` produces invalid LLVM
   IR (passes `i8**` where `strlen` wants `i8*`) — recorded
   in TODO.md under "Known codegen bugs" with workaround. The
   example complements the README's *Memory safety &
   concurrency model* section as the canonical entry point
   for newcomers. Test totals unchanged: 966 lib + 47 e2e +
   11 vtables-phase3 + 2 user-drop-by-ref + 1 ssa-examples.
   Closure #261.

   **Move-rejection diagnostic — type-aware fix hint done 2026-05-26**:
   the "value 'v' was moved; cannot use after move"
   diagnostic now carries a type-aware secondary note
   suggesting the user's best recovery. For `Vec<T>` /
   `OwnedStr` / affine structs / enums: "consider borrowing
   with `ref v` for read-only access, or call `clone(v)` if
   you need both bindings to own data". For exclusive
   single-owner handles (`Atomic` / `Mutex` / `Channel` /
   `Guard`): "share via `ref v` — exclusive single-owner
   handle and cannot be cloned (use Atomic<T> or Channel
   through a borrow if multiple threads need access)". For
   `[T;N]` arrays: ref-only hint (clone not supported in
   v1). Helper `move_recovery_hint(name, ty)` in
   `src/checker.rs` returns the right phrasing per Type
   variant. Three new lib tests pin the Vec, OwnedStr, and
   Atomic shapes; existing move-tracking tests untouched.
   Closes the first item in the *Move/clone polish* TODO
   sub-section. Test totals: 966 lib + 47 e2e + 11
   vtables-phase3 + 2 user-drop-by-ref + 1 ssa-examples.
   Closure #260.

   **Move / clone / copy story documented in README 2026-05-26**:
   added a *vāṇī vs Rust — ownership at a glance* subsection
   under the *Memory safety & concurrency model* section.
   Confirms vāṇī's model already matches Rust's
   move-by-default semantics: affine types (Vec, OwnedStr,
   Atomic, Mutex, Guard, Channel, Task, [T;N], structs/enums
   with affine fields) MOVE on assignment / fn call; Copy
   types (primitives, references, all-Copy aggregates) copy.
   **No implicit clone anywhere** in the language — the only
   deep-copy paths are explicit `.clone()` and `clone_at(ref
   xs, i)`. Auto-borrow handles `==` / Str-param contexts so
   the binding stays usable without consuming. No code
   changes — the model is already shipping. TODO.md gains a
   "Move / clone polish" subsection with small Q-of-L
   follow-ups (diagnostic hints, auto-borrow extension survey,
   `pop` builtin, fallible allocation API) — all
   future-polish, not correctness fixes.

   **Parallel-for implicit-reduction race check done 2026-05-26**:
   `parallel for { total = total + i; }` over a captured
   Copy-typed primitive (no `reduce` clause) previously
   compiled cleanly even though the runtime race produced
   non-deterministic results. The effects checker walked
   the body but only flagged *impure* operations (print,
   impure calls, indexed writes); naked Copy mutations of
   a captured binding slipped through. The new
   `check_for_captured_mutations` pass runs after the
   reduction-strip pass, walks the body tracking
   body-local `let` bindings, and emits a precise
   "mutates captured variable '<name>' without declaring
   it as a reduction; this races at runtime. Add
   `reduce <name> with <op>;` … or use `Atomic<T>` …"
   diagnostic on any non-local non-reduction Reassign.
   Body-local mutations remain free (per-iteration, not
   shared). Three new lib tests cover: the race shape
   errors with the precise hint, body-local lets pass,
   declared reductions still parse + compile. Closes
   the largest documented compile-time gap in the
   parallel-for safety story. Test totals: 963 lib + 47
   e2e + 11 vtables-phase3 + 2 user-drop-by-ref + 1
   ssa-examples. Closure #259.

   **Namespaces — `pub(kosh)` visibility tier done 2026-05-26**:
   modules now accept the qualifier `pub(kosh)` to mark
   an item as exported within the kosh but NOT through
   the kosh boundary into external dependents. Today
   vāṇī compiles a single kosh at a time, so `pub(kosh)`
   behaves identically to plain `pub` — the bit is
   preserved so future kosh-boundary enforcement
   activates on existing source without rewrites. The
   parser peeks for `(kosh)` after `pub` (using the
   2-token lookahead pattern from the `pub use` work);
   any other qualifier (`pub(super)`, `pub(crate)`, …)
   surfaces a clear "only `pub(kosh)` is supported"
   diagnostic and skips past the bad form to avoid
   cascading errors. `ModuleVisibility` grows parallel
   `*_kosh_only: Vec<bool>` arrays alongside each
   `*_pub` bitmap; the formatter checks the kosh-only
   bit and emits `pub(kosh)` when set. Three new lib
   tests pin: the syntax parses + compiles, the bit
   lands correctly in the AST, and bad qualifiers are
   rejected cleanly. Closes the last namespaces follow-
   up that lives entirely inside the existing compiler
   — the remaining kosh work (manifest, resolver,
   registry CLI) is package-manager scope. Test totals:
   960 lib + 47 e2e + 11 vtables-phase3 + 2 user-drop-
   by-ref + 1 ssa-examples. Closure #258.

   **Namespaces — re-exports `pub use foo::bar;` done 2026-05-26**:
   `pub use` inside a module body re-exports an item
   under the current module's namespace — external
   callers reach it as `<this_mod>::<local>` even though
   the actual implementation lives in another module.
   `UsePath` gains an `is_pub: bool` field; the parser
   peeks for `Pub` immediately followed by `Use` so the
   token is consumed only when the re-export form is
   intended (regular `pub fn`, `pub struct` etc. still
   work). The checker builds a global re-export map
   (`<this_mod>__<local> → <imported_mangled>`) during
   per-module flattening, then resolves transitively via
   fixed-point iteration so chained re-exports
   (`top::pub use middle::X` → `middle::pub use deepest::X`)
   collapse to a single hop. After resolution, a rewriting
   pass walks every top-level function, impl, methods
   block, and struct, substituting source-visible
   re-export names for their implementation names so the
   type checker only ever sees real items. Five new lib
   tests pin: single-hop re-export resolves, transitive
   chain collapses, duplicate local name diagnoses
   cleanly, `as`-rename disambiguates colliding
   re-exports, and a regression guard confirms plain
   (non-pub) `use` inside a module does NOT re-export.
   Formatter round-trips the `pub` prefix on use paths.
   Test totals: 957 lib + 47 e2e + 11 vtables-phase3 +
   2 user-drop-by-ref + 1 ssa-examples. Closure #257.

   **Namespaces — `use` inside `module { }` blocks done 2026-05-26**:
   modules now admit local `use foo::bar;` declarations
   alongside item definitions. The alias is scoped to
   that module's body — references inside the body
   resolve through the local map, but the alias does
   NOT leak outside the module nor into nested
   submodules (a child module needs its own `use`).
   `ModuleDecl` gains a `use_paths: Vec<UsePath>` field;
   the parser admits the same three forms it does at
   top level (single-item, brace-list, `as`-rename)
   alongside an explicit reject for glob `use foo::*;`
   inside modules — the post-flatten name set isn't
   available during per-module processing. The
   checker's per-module `qualify` map gains a fourth
   resolution case (after intra-module visibility,
   before nested-sibling lookup) that pulls in the
   local aliases. The formatter rounds-trips
   automatically — `format_module_decl` gained a
   `ModItem::Use` arm that mirrors the top-level emit
   shape. Four new lib tests pin: aliases resolve
   inside body, alias doesn't leak outside, brace-list
   + per-entry `as` rename works, and glob is rejected
   with a clear diagnostic. This is the prerequisite
   for re-exports (`pub use foo::bar;` builds on top
   of module-local `use`). Test totals: 952 lib + 47
   e2e + 11 vtables-phase3 + 2 user-drop-by-ref + 1
   ssa-examples. Closure #256.

   **Vāṇī terminology — "kosh" (कोश) adopted as the name for what Rust calls a crate 2026-05-26**:
   per user preference, vāṇī's compilation-unit /
   package concept is named **kosh** ("treasure /
   repository"). One kosh = one compilation unit
   shipping a public API; the future package registry
   is **Vāṇī-Kosh**. The rename is purely terminological
   today — the kosh concept itself hasn't been
   implemented yet (multi-file compile via
   `use "path";` works file-by-file; kosh adds a
   manifest + boundary on top). The naming is now
   reflected in `docs/namespaces_design.md`,
   TODO.md (the `pub(crate)` follow-up is renamed
   `pub(kosh)`), and a new section under
   "Deferred" outlines the full package-manager arc
   (manifest → resolver → `pub(kosh)` → re-exports
   → registry CLI → stdlib-as-kosh). The smallest
   beachhead is `pub(kosh)` + re-exports — both live
   entirely in the existing compiler, no registry
   needed.

   **English keyword aliases — `assign` for `let`, `give_back`/`give back` for `return` done 2026-05-26**:
   the lexer now recognizes `assign` as a single-token
   alias for `let`, and adds two more aliases for
   `return` alongside the existing `give`: `give_back`
   (snake-case multi-word) and the two-word `give back`
   (folded by a small post-lex pass
   `merge_give_back_ascii_alias`). The merger only fires
   when the preceding `Return` token's *source text* was
   exactly `give`, so canonical `return back;` (where
   `back` is a user variable) is unaffected — the SSA
   value of `back` still flows into the return expr.
   Three new lib tests pin each form + the regression
   guard. The aliases are pure surface — identical AST,
   no semantic divergence. Closure #255.

   **Namespaces — `use foo::bar as baz;` rename + collision diagnostic done 2026-05-26**:
   single-item and brace-list `use` entries now accept
   an optional `as <local>` suffix that overrides the
   bound name (`use a::item as a_item;` →  `a_item`
   resolves to `a__item`, the original `item` does NOT
   come into scope). The checker also tracks
   `alias_origin` and surfaces a precise diagnostic if
   the same local name is imported twice ("name `item`
   is already imported from `a::item`; give one a
   different local name with `use … as …;`") — closes
   a silent-last-wins footgun. The collision check
   covers explicit-vs-explicit, explicit-vs-glob, and
   glob-vs-glob. Three new lib tests pin: rename
   resolves, collision diagnostic fires, mixed-rename
   brace-list compiles. Formatter round-trips
   automatically via the new `up.alias` field. Test
   totals: 945 lib + 47 e2e + 11 vtables-phase3 + 2
   user-drop-by-ref + 1 ssa-examples. Closure #254.

   **Namespaces — glob `use foo::*;` import done 2026-05-26**:
   `use foo::*;` now expands to every direct public child
   of `foo`, bringing each into scope as an unprefixed
   alias. The parser accepts `*` as the leaf segment
   after `module::` (alongside the existing single-item
   and brace-list forms) and stores it as a sentinel
   `UsePath { item: "*" }`. The checker — after the
   flatten pass has already mangled module items into
   top-level names — scans `program.functions`,
   `structs`, `enums`, `interfaces`, `consts`, and
   `type_aliases` for entries matching `foo__<leaf>`;
   filters out private (`foo__priv__<leaf>`) and
   transitive (`foo__bar__<leaf>`) entries; and inserts
   each into the alias map. Matches Rust's
   non-transitive glob semantics — `use foo::*;` does
   NOT pull in `foo::bar::baz`, just the direct
   children. The formatter already round-trips
   correctly (`up.module + "::" + up.item` produces
   `foo::*`). Three new lib tests pin the expansion,
   private filtering, and non-transitivity. Test
   totals: 942 lib + 47 e2e + 11 vtables-phase3 + 2
   user-drop-by-ref + 1 ssa-examples. Closure #253.

   **SSA-LLVM — multi-block parallel-for fallback gate done 2026-05-26**:
   the SSA-LLVM `emit_parallel_for_region_llvm` now
   early-exits with a clear EmitError when the recognized
   region has more than one block — surfacing the
   intentional gate instead of silently failing deeper
   in the reduction-update analysis. The
   `emit_llvm_via_ssa` wrapper in main.rs catches the
   error and routes through tree-LLVM, which already
   handles multi-block bodies correctly via GOMP's
   reduction combine. The SSA-LLVM optimization path
   (atomicrmw against parent-side allocas) requires
   Phi-traceback to find where the actual `+`/`*` update
   physically lives — the back-edge arg in multi-block is
   a block-param (merge's Phi-equivalent), not the
   arithmetic op. Implementing that traceback is a
   follow-up; for now, the fallback is automatic and
   correct. One new lib test pins the fallback shape.
   Test totals: 939 lib + 47 e2e + 11 vtables-phase3 + 2
   user-drop-by-ref + 1 ssa-examples. Closure #252.

   **SSA Step 3b — emit half (SSA-C) done 2026-05-26**:
   `emit_parallel_for_region` in
   [`src/ssa_backend_c.rs`](src/ssa_backend_c.rs) now
   inlines every block in `ParallelRegion::region_blocks`
   inside the `#pragma omp parallel for` for-loop, not
   just `body_block.instructions`. Each in-region block
   gets a `bbN:` label + its standard `goto` terminator
   so the if/else (and other in-body control flow) the
   recognizer accepted via #241 now actually runs. The
   merge block (unique back-edge to step) is reordered
   to be last in `region_blocks` so its replaced
   terminator — the reduction-update rebind — falls
   through to the for-loop's closing `}` correctly.
   `skip_blocks` extends to cover all region blocks so
   the parent block walk no longer emits orphan
   `bbN:` labels after the loop. SSA-LLVM still routes
   multi-block bodies through tree-LLVM as before
   (its `emit_outlined_parallel_for_llvm` only walks
   `body_block`; capture analysis + reduction-update
   discovery across multi-block bodies is the next
   step). One new lib test exercises the end-to-end
   shape via `if`-inside-`parallel for`. Test totals:
   938 lib + 47 e2e + 11 vtables-phase3 + 2 user-drop-
   by-ref + 1 ssa-examples. Closure #251.

   **Namespaces — formatter support + round-trip done 2026-05-26**:
   `intentc fmt` now re-emits `module name { … }` blocks
   (with nested children and `pub` markers) and
   `use foo::bar;` path imports. Internal `__`-mangled
   type names (e.g. `Type::Struct("geo__Point")` from a
   `geo::Point` parse) are rendered back as `geo::Point`
   via a new `type_to_source` helper so the round-trip
   parses back to the same AST. `examples/modules.vani`
   demonstrates the full feature set (visibility,
   path imports, nested modules, orphan-respecting
   `implement`) and is now wired into the format
   round-trip / parity runners. All 937 lib + 47 e2e
   tests still green. Closure #242–#249 ship-complete.

   **Namespaces — implicit sibling-module references done 2026-05-26**:
   from inside `module outer`, references to a nested
   module's items work with just `inner::item` (no
   `outer::` prefix needed). The `qualify` function in
   the flattening pass now recognizes the first segment
   of an `__`-separated path; if it matches a child
   module's name, the parent path is prepended. Mirrors
   Rust's sibling-module lookup inside `mod outer { … }`.

   Test totals: 937 lib + 47 e2e + 11 vtables-phase3 + 2
   user-drop-by-ref + 1 ssa-examples passing. Closure
   #249.

   **Namespaces — nested modules done 2026-05-26**:
   `module outer { module inner { … } }` now parses and
   flattens. Items in the inner module mangle to
   `outer__inner__name`. Path expressions and types support
   arbitrary-depth `a::b::c::…` chains — the parser loops
   the `::` consumption.

   Implementation:
   - **AST**: `ModuleDecl` gains
     `modules: Vec<ModuleDecl>` (nested children) +
     `ModuleVisibility.modules_pub`. The parser drops its
     previous v1 "nested module rejection" and recurses into
     `parse_module_decl` when it sees `module` inside another
     module.
   - **Path parser**: deep-path consumption now loops on
     `::` in both expression and type position, plus in
     `use` declarations. `outer::inner::deep` → `outer__inner__deep`.
   - **Checker**: `flatten_modules_in_program` converted from
     a `for module in modules` loop to a worklist
     `Vec<(path_prefix, ModuleDecl)>`. Each iteration's
     full module name is `path_prefix__module.name` (or
     just `module.name` if prefix is empty). Nested
     children get pushed onto the worklist with the
     current path as their prefix.

   v1 requires explicit `outer::inner::item` paths from
   anywhere. Implicit `inner::item` from inside `outer`
   (Rust-style sibling-module lookup) is deferred. `use`
   statements work for any depth via the existing deep-path
   parser. Test totals: 936 lib + 47 e2e + 11 vtables-phase3
   + 2 user-drop-by-ref + 1 ssa-examples passing. Closure
   #248.

   **Namespaces Phase 3c (start) — multi-item `use foo::{a, b};` done 2026-05-26**:
   the brace-list form of module imports parses + applies.
   The parser detects `{` after `module::` and reads a
   comma-separated list of item idents (trailing comma
   allowed, empty list rejected), then expands each into a
   separate `UsePath` entry. The flattening pass already
   handles arbitrary numbers of aliases — no additional
   checker work required. The single-item `use foo::bar;`
   form remains unchanged.

   `module foo::*;` (glob) and nested modules are still
   queued; both have additional design + implementation
   considerations. Test totals: 935 lib + 47 e2e + 11
   vtables-phase3 + 2 user-drop-by-ref + 1 ssa-examples
   passing. Closure #247.

   **Namespaces Phase 3b — orphan rules for `implement` blocks done 2026-05-26**:
   `implement Iface for T` must now live in the same module
   as either the interface or the for-type (or all three at
   top level). Implementation:
   - `ImplDecl` gains `home_module: Option<String>`, set
     by the flattening pass when the impl was declared
     inside a `module { ... }` block.
   - `hoist_impls_into_functions` extracts the module of
     iface (`<mod>__name` → `<mod>`) and for-type, compares
     to the impl's home, and emits an `orphan impl` diagnostic
     naming the current and target modules when the placement
     is invalid.

   Diagnostic example:
   ```
   error: orphan impl: `implement Drawable for geo__Point`
   declared in module `rendering` but the interface lives
   in top-level and the type lives in module `geo`. Move
   the impl into one of those modules.
   ```

   Two lib tests cover the rejection + valid placement
   cases. Test totals: 934 lib + 47 e2e + 11 vtables-phase3
   + 2 user-drop-by-ref + 1 ssa-examples passing. Closure #246.

   **Namespaces Phase 3a — `use foo::bar;` single-item imports done 2026-05-26**:
   the third Rust-style namespace primitive lands.
   `use math::square;` at the top of a file introduces
   `square` as a bare alias for `math::square` in the
   surrounding file, so the user can call `square(x)`
   without the module prefix. Explicit `math::double(x)`
   continues to work.

   Implementation:
   - New `UsePath { module, item, span }` AST node alongside
     the existing `Use { path, span }` (file import). The
     parser detects which form by peeking at the token
     after `use`: a string literal stays as `Use`, an
     identifier becomes `UsePath`.
   - `Program` gains a `use_paths: Vec<UsePath>` field.
   - At the end of `flatten_modules_in_program`, build a
     `bar → foo__bar` alias map and walk top-level fn
     bodies rewriting bare references (Call / Var /
     StructLit / Match-pattern / Cast type). Module
     bodies aren't re-walked — they already received
     intra-module prefix handling. Top-level fns with `__`
     in their names (i.e. fns hoisted out of modules) are
     also skipped — they retain their explicit paths.

   v1 supports single-item imports only. Glob
   `use foo::*;` and multi-item `use foo::{a, b};` are
   deferred. Orphan rules for `implement` blocks +
   nested modules also queued.

   Test totals: 932 lib + 47 e2e + 11 vtables-phase3 + 2
   user-drop-by-ref + 1 ssa-examples passing. Closure #245.

   **Namespaces Phase 2.1 — struct / enum / const / type-alias visibility done 2026-05-26**:
   visibility enforcement now extends to every item kind
   inside a module, not just functions. The flattening
   pass's per-item-list loop converts to
   `.into_iter().enumerate()` for each kind, looking up
   `module.visibility.<kind>_pub.get(idx)` and registering
   private items in `PRIVATE_MODULE_ITEMS`. The
   unknown-struct-type diagnostic also consults the
   registry to surface the same "private to its module"
   message for struct references. Two new lib tests cover
   private struct rejection + public struct round-trip
   with intra-module bare-name use. Test totals: 931 lib +
   47 e2e + 11 vtables-phase3 + 2 user-drop-by-ref + 1
   ssa-examples passing. Closure #244.

   **Namespaces Phase 2 — visibility enforcement done 2026-05-26**:
   non-`pub` items inside a module are now unreachable from
   outside the module. Implementation uses **differentiated
   name mangling** rather than runtime checks: public items
   mangle to `<mod>__<name>` (matches what the parser
   produces for source `mod::name`); private items mangle
   to `<mod>__priv__<name>` (a form the parser CAN'T
   produce). Outside references to private items hit a
   lookup miss; intra-module references are rewritten by
   the flattening pass to the matching mangled form.

   The unknown-function diagnostic now consults a
   `PRIVATE_MODULE_ITEMS` registry: when the user wrote
   `mod::name` (parser-mangled to `mod__name`) and that
   doesn't exist but `mod__priv__name` does, surface
   `"function 'mod::name' is private to its module — mark
   it 'pub' to allow access from outside"`. Clear, names
   the source path, suggests the fix.

   New lib tests:
   - `module_private_item_accessible_from_inside` — public
     sibling calling a private fn inside the same module
     compiles (resolution stays in-module).
   - `module_private_item_blocked_from_outside_with_clear_diagnostic`
     — outside reference surfaces the new diagnostic.

   v1 enforces visibility for functions; struct / enum /
   const / type-alias visibility lands in a Phase 2.1
   follow-up (same mechanism, more enumerate-with-index
   patterns to wire). Test totals: 929 lib + 47 e2e + 11
   vtables-phase3 + 2 user-drop-by-ref + 1 ssa-examples
   passing. Closure #243.

   **Namespaces / modules — Phase 1 (parser + checker flatten) done 2026-05-26**:
   Rust-style `module foo { items… }` blocks now parse and
   type-check. Items inside are renamed to `<module>__<name>`
   at the AST level (the `__` separator is backend-safe; the
   source-form `::` gets mapped at parse time). Intra-module
   references to sibling items get the same prefix
   automatically — `quad` inside `module math` calling
   `double(x)` resolves to `math__double` without the
   user spelling the prefix.

   Pieces shipped in this phase:
   - **Lexer**: new `module`/`mod` keyword pair (English
     canonical + Rust shorthand alias); new `pub`/`public`
     keyword pair (Rust canonical + readable alias); `::`
     operator (already in the lexer; ColonColon).
   - **AST**: `ModuleDecl` carrying parallel lists of every
     item kind plus a `ModuleVisibility` bitmap; `Program`
     gains a `modules: Vec<ModuleDecl>`.
   - **Parser**: top-level `module IDENT { items… }` blocks;
     `pub` modifier on each item; rejects nested `module`
     in v1; path expressions `IDENT::IDENT(args)` parse to
     a single `Var("module__item")` so downstream code
     resolves naturally. Type-position paths `geo::Point`
     parse to `Type::Struct("geo__Point")`.
   - **Checker**: `flatten_modules_in_program` pre-pass
     renames items, rewrites intra-module references in
     bodies (Call/Var/StructLit/Match/Cast/types), then
     pushes the items onto Program's global lists. The rest
     of the checker sees a flat program with mangled
     names. Visibility (`pub`) is parsed but not yet
     enforced — v1 effectively makes every item public.
   - **Tests**: two lib tests
     (`modules_namespace_items_and_intra_module_refs_resolve`
     + `modules_with_struct_and_call_compile`) cover
     functions, structs, intra-module calls, and type-
     position paths.

   Test totals: 927 lib + 47 e2e + 11 vtables-phase3 + 2
   user-drop-by-ref + 1 ssa-examples passing. Closure
   #242. Phase 2 (visibility enforcement, `use` statements,
   orphan rules) is queued.

   **SSA Step 3b — recognizer multi-block body acceptance done 2026-05-26**:
   `recognize_parallel_region` in
   [`src/ssa_backend_c.rs`](src/ssa_backend_c.rs) now
   accepts multi-block parallel-for bodies. The recognizer
   walks the body sub-CFG from `body_block`, collecting all
   reachable blocks until reaching `step_block`. v1 still
   requires exactly one block in the region to terminate
   by jumping to step (the "merge block"); multiple
   back-edges or nested cycles surface clean `EmitError`s.
   The merge block's Jump args are used as reduction-
   update values (same shape as the single-block case).
   This is the FIRST half of Step 3b — the recognizer
   accepts the shape, but neither SSA-C nor SSA-LLVM's
   emit yet lowers multi-block bodies (they still iterate
   only `body_block.instructions`). Tree fallback handles
   multi-block correctness today, so the recognizer change
   is a foundation without behavior change. Test totals:
   925 lib + 47 e2e + 11 vtables-phase3 + 2 user-drop-
   by-ref + 1 ssa-examples passing. Closure #241.

   **Array-return follow-up: tree-LLVM array-from-call + SSA gate + example done 2026-05-26**:
   while wiring up the tree-LLVM array-return path, two
   issues surfaced:
   1. SSA-LLVM's array-return emit produces `define [N x T]*
      @fn_make(...)` returning a pointer to a stack-allocated
      array — the pointer dangles after the fn returns, so
      callers read garbage. Fix: `ssa_type_supported` now
      rejects `Type::Array { .. }` so array-returning
      programs route through tree-LLVM (which correctly
      returns `[N x T]` by value, handled by LLVM's
      built-in struct-return ABI).
   2. Tree-LLVM's Let-from-array-expression handler only
      knew about ArrayLit and Var sources; a Call returning
      an array fell through to `unreachable!`. Now the
      catch-all path emits the call (which yields the
      `[N x T]` SSA value), then stores it into the local
      alloca.

   New `examples/array_return.vani` demonstrates both the
   ArrayLit-return path and the Var-source-return path,
   wired into both check + cross-backend parity runners.
   Both backends now produce `make_doubles(5) = 5 10 20 40`
   and `shift_by(7) = 17 27 37`. Test totals unchanged:
   925 lib + 47 e2e + 11 vtables-phase3 + 2 user-drop-by-
   ref + 1 ssa-examples. Closure #240.

   **Array types in fn return position done 2026-05-26**:
   `fn make() -> [i64; 3] { return [10, 20, 30]; }` now
   compiles + runs on both backends. tree-LLVM already
   accepted `[N x T]` returns natively (the gate was the
   only blocker). tree-C needed a per-shape struct wrapper
   because C can't return values of bare array type; new
   `typedef struct { T data[N]; } intent_arr_ret_<N>_<T>;`
   gets emitted in the preamble (one per shape, only when
   the program actually uses array returns). Return-stmt
   wraps array values in the struct literal
   (`return (intent_arr_ret_3_int64_t){ .data = {10, 20,
   30} };`) — direct array-literal source path or via a
   memcpy stack temp for other expression shapes. Let
   stmts from array-returning calls unwrap through
   `_intent_ret_<name>.data` + memcpy into the local
   array. `c_type_name(Array)` returns the struct
   typedef name; locals + struct fields + Vec elements
   still use the bare-array declarator form. Test
   totals: 925 lib + 47 e2e + 11 vtables-phase3 + 2
   user-drop-by-ref + 1 ssa-examples passing. Closure
   #239.

   **`while` loop between `try` and `return` done 2026-05-26**:
   the try desugar's intermediate-stmt vocabulary gained
   `Stmt::While`, and Block-expr's checker now type-checks
   while-loops whose body uses only Assign / Print stmts.
   tree-C emits the while inline inside the GCC stmt-
   expression (`while ((cond)) { v_x = (...); ... }`).
   tree-LLVM forwards through the existing `emit_stmt`
   while handler — plus a small fix where `ctx.current_block`
   is now updated to the while's exit label so a
   surrounding match-arm's PHI captures the correct
   incoming predecessor (without the fix, the PHI named
   the arm's entry block but the actual predecessor was
   the while's exit; `opt -verify` rejected the IR).
   Closes the last actionable item under the `#5` try
   follow-up queue. Test totals: 925 lib + 47 e2e + 11
   vtables-phase3 + 2 user-drop-by-ref + 1 ssa-examples
   passing. Closure #238.

   **`write` alias for `print` + Devanagari `लिखो` done 2026-05-26**:
   user feedback flagged two issues with the print
   keyword. `print` is C/Python heritage; `write` is more
   versatile (matches `write(stdout, ...)` style). The
   Devanagari `छाप` (chāp = "imprint/stamp") felt
   unnatural for screen output; `लिख` / `लिखो` (likh /
   likho = "write") is the natural verb across Sanskrit /
   Hindi / Marathi. Lexer accepts `write` as English alias
   for `print`; Devanagari table swaps `छाप` for
   `लिख` / `लिखो`. Three Devanagari examples updated.
   Test totals: 924 lib + 47 e2e + 11 vtables-phase3 + 2
   user-drop-by-ref + 1 ssa-examples passing. Closure #237.

   *Word-order issue (queued)*: user flagged that the
   Devanagari aliases swap words but keep English's word
   ORDER, producing unnatural sentences (e.g. `के लिए i
   से 0 तक 5` instead of the natural Hindi `0 से 5 तक i
   के लिए`). Indo-Aryan languages are SOV with
   postpositions — `से` (from) follows the noun, not
   precedes it. Properly fixing requires a per-language
   parser mode that swaps to SOV grammar when the file is
   detected as Devanagari. Multi-session work; tracked in
   TODO under "Devanagari word order — SOV grammar fit".

   **Per-file language purity done 2026-05-26**: the lexer
   now rejects files that mix English structure keywords
   with Devanagari aliases in the same file. A new
   post-lex `enforce_language_purity` pass walks the
   token stream; the first English-vs-Devanagari mismatch
   surfaces as a clear "language mismatch" diagnostic
   naming the prior-keyword's span so the user can pick
   which script to keep. Type names (`i64`, `bool`, `Vec`,
   …), identifiers, `true`/`false`, and operators all
   stay neutral so a Hindi file can still write `फलन
   add(a: i64, b: i64) -> i64`. V1 enforces script-level
   purity (English vs Devanagari); finer-grained
   Sanskrit / Hindi / Marathi distinction within
   Devanagari is queued — the existing alias table has
   ambiguous entries (`यदि` is both Sanskrit and Hindi)
   so it needs grammar-consultant review before
   tightening. Three previously mixed-language examples
   (`hindi_keywords.vani`, `marathi_keywords.vani`,
   `sanskrit_keywords.vani`) updated to use the
   Devanagari `छाप` alias instead of English `print`.
   Test totals: 923 lib + 47 e2e + 11 vtables-phase3 + 2
   user-drop-by-ref + 1 ssa-examples passing. Closure
   #236.

   **File extension renamed `.intent` → `.vani` done 2026-05-26**:
   59 example files renamed via `git mv`; all tests, source,
   doc-comments, README references, LSP snippet, and CLI help
   text updated. Four extension-matcher sites in
   `tests/run_end_to_end.rs` + `tests/ssa_examples.rs` updated
   (`Some("intent")` → `Some("vani")`). CLI banner now reads
   "intentc — vāṇī language compiler driver". Language
   keyword `intent "..."` and the `program.intents` field
   stay unchanged — they're language constructs, not file
   extensions. **Decision rationale (user-confirmed):** single
   extension `.vani` for everything; vāṇī has no header/impl
   split, follows the Rust/Go/Swift/Python pattern. Test
   totals unchanged: 921 lib + 47 e2e + 11 vtables-phase3 + 2
   user-drop-by-ref + 1 ssa-examples. Closure #235.

   **English keyword aliases (Phase 1, conservative set) done 2026-05-26**:
   the lexer's keyword table now recognizes five high-value
   English aliases that don't collide with common
   user-identifier shapes:
   - `struct` / `record`
   - `interface` / `trait`
   - `implement` / `impl`
   - `return` / `give`
   - `->` / `returns` / `yields`

   Each alias group maps to the same `TokenKind` so the
   parser doesn't see the difference; the formatter picks
   a canonical spelling per file based on what the user
   first used. Riskier aliases like `def` / `function` /
   `bind` / `mutable` / `constant` / `otherwise` were
   deliberately NOT added — they'd silently break existing
   programs that use them as parameter / variable / field
   names (the `def` alias was added in a first attempt and
   broke an existing test's `def: i64` parameter). The
   per-file language-purity gate (queued in TODO) is the
   path to safely expanding the set. Test totals: 921 lib
   + 47 e2e + 11 vtables-phase3 + 2 user-drop-by-ref
   passing. Closure #234.

   **`examples/dyn_dispatch.intent` added 2026-05-25**: the
   first example exercising the vtables epic end-to-end. The
   program declares two impls (Circle + Square of Drawable),
   demonstrates owned-`dyn` dispatch, `ref dyn` dispatch, and
   heterogeneous `Vec<dyn>` iteration. Wired into both the
   `check_examples_all_succeed` and the cross-backend parity
   runner so any future regression in the vtable codegen path
   surfaces immediately. SSA example walker picks it up
   automatically via `read_dir`. Test totals: 920 lib + 47
   e2e + 11 vtables-phase3 + 2 user-drop-by-ref passing.
   Closure #233.

   **Guard-if between `try` and `return` done 2026-05-25**:
   `if cond { return X; }` early-return guards now compose
   with `try`. New AST-level pre-pass
   `rewrite_guard_ifs_in_stmt_list` runs BEFORE the try
   desugar — it finds the first guard-if of shape
   `if cond { return X; }` (no else, single Return in
   then) followed by remaining stmts ending in `return Y`,
   and rewrites the whole tail into
   `return match cond { true then X, false then { rest...; Y } };`.
   The result feeds the existing try desugar, which only
   sees a try-let and a final Return — no Block-expr stmt
   vocab extension needed. The pre-pass is gated on
   `body_contains_try` so non-try functions keep their
   direct if/else lowering (visible to LLVM test
   assertions on `then0:`/`else1:` block labels). Test
   totals: 920 lib + 47 e2e + 11 vtables-phase3 + 2
   user-drop-by-ref. Closure #232.

   **Reassignment between `try` and `return` done 2026-05-25**:
   the try desugar now admits assignment statements
   (e.g. `w = w + 1;`) between the try-let and the final
   return. The Block-expr stmt vocabulary gained an Assign
   arm — it looks up the binding in the surrounding scope,
   type-checks the RHS, and emits `TypedStmt::Reassign` into
   the typed_stmts list. Tree-C emits the store inline
   inside the GCC statement-expression; tree-LLVM forwards
   through the existing Reassign stmt emit which targets the
   binding's alloca address. Test totals: 920 lib + 47 e2e
   + 11 vtables-phase3 + 2 user-drop-by-ref passing.
   Closure #231.

   **`try EXPR(args)` call-precedence parsing done 2026-05-25**:
   `try maybe(5)` (and `try Type.helper(args)`) now parses
   correctly — previously the parser stopped at primary-expr
   precedence so `(5)` got reapplied to the `try maybe`
   expression and surfaced "only named functions can be
   called". `try EXPR` now binds at call-expr precedence, so
   the operand includes any trailing `(...)` chain.
   Binary operators (`+`, `*`, etc.) still bind above the
   try, keeping `try a + b` unambiguous. Closure #230.

   **Epic C: user-Drop for structs with heap fields done 2026-05-25**:
   structs that own `OwnedStr` / `Vec` / nested-struct fields
   can now declare a user-Drop without losing the per-field
   cleanup pass. The Drop impl signature now accepts both
   the original by-value form `fn drop(self: T) -> i64` (for
   structs without heap fields — consumes self, suppresses
   per-field drops to avoid double-free) and a new mut-ref
   form `fn drop(self: mut ref T) -> i64`. The mut-ref form
   runs FIRST at scope exit, then the per-field drop pass
   reclaims the heap. The user code can `print self.name`,
   `self.x = 0`, etc. — full access without ownership. A
   thread-local `USER_DROP_BY_REF` registry (populated in
   `hoist_impls_into_functions`) tells the backends which
   call shape to emit. Both tree-C and tree-LLVM updated.
   New e2e test `tests/user_drop_by_ref.rs` verifies the
   user-Drop runs (visible via stdout) AND the program's
   exit code matches `len(b.items)`. Test totals: 920 lib
   + 47 e2e + 11 vtables-phase3 + 2 user-drop-by-ref
   passing. Closure #229.

   **Vtables Phase 5 (auto-borrow in `==` desugar) done 2026-05-25**:
   the existing `a == b` → `<T>_eq(a, b)` desugar didn't
   look at the impl's parameter types and always passed both
   args by value. When `implement Eq for T` declares
   `other: ref T` (or `mut ref T`), the call was a type
   mismatch (cc / lli both rejected). The desugar now
   inspects each param's expected type and auto-wraps Var
   operands in `TypedExprKind::Ref` / `RefMut` when the impl
   wants a borrow. Non-Var operands fall through with a
   clear "let-bind both operands before comparing"
   diagnostic. Test totals: 920 lib + 47 e2e + 11 vtables-
   phase3 passing. Closure #228.

   **Vtables Phase 4c (`ref dyn Iface` borrows) done 2026-05-25**:
   functions can now take `d: ref dyn Iface` and dispatch
   through the borrow. Three small extensions: (1) the
   MethodCall checker accepts `Type::Ref(Type::Object)` /
   `Type::RefMut(Type::Object)` receivers in addition to the
   owned `Type::Object` form (same DynDispatch IR, same
   slot lookup). (2) `format_declarator` learned
   `Type::Ref(Object)` → `const intent_dyn_<Iface>*` and
   `Type::RefMut(Object)` → `intent_dyn_<Iface>*`. (3) The
   DynDispatch emit in both backends now detects a borrowed
   receiver and dereferences first — tree-C wraps in
   `(*recv)`; tree-LLVM emits an explicit `load` before the
   extractvalue ops. New e2e tests confirm `area_of(ref
   dyn_c)` returns 36 on both backends. Test totals: 920
   lib + 47 e2e + 9 vtables-phase3 passing. Closure #227.

   **Vtables Phase 4b (`Vec<dyn Iface>` heterogeneous collections) done 2026-05-25**:
   the canonical vtable use case — `vec(circle, square)` typed
   as `Vec<dyn Drawable>` — now compiles + runs identically on
   both backends, with a per-element trampoline dispatching to
   each impl's concrete `area`. Three pieces landed:

   1. **Checker**: `check_vec_builtin` no longer requires
      strict element homogeneity. When elements have different
      nominal types but all share exactly one common
      `implement Iface for T`, the result is
      `Vec<dyn Iface>` and each element is wrapped in a
      `TypedExprKind::DynCoerce`. Multiple shared interfaces
      → ambiguous (the existing "must be assignable to T"
      diagnostic surfaces). New `ast::ifaces_implemented_by`
      helper inverts the impl registry. `coerce_checked` also
      gained a `Vec<T> → Vec<dyn Iface>` arm that re-coerces a
      homogeneous vec literal's elements when the destination
      is annotated as `Vec<dyn Iface>` (handles the
      `vec(c)` single-element case where homogeneity inference
      still triggers).
   2. **Trampoline shape fix**: the per-(T, Iface) trampolines
      previously inherited the iface declaration's example
      self type (e.g. `Struct_Circle` for Drawable). For Square's
      `area` impl the cast was wrong. Both tree-C and
      tree-LLVM now cast `void* self` to `Struct_<type_name>`
      where type_name is the impl's actual `for_type`.
   3. **Sizing + tags**: tree-LLVM's `vec_struct_tag` and
      `vec_element_byte_size` gained `Type::Object` arms
      (`intent_dyn_<Iface>` tag, 16-byte fat-pointer width)
      so the Vec runtime layout matches what the C backend
      already produces.

   New e2e tests exercise `for x in xs { total = total + x.area(); }`
   over a `Vec<dyn Drawable>` containing both a Circle and a
   Square; both backends return 34 (= 3*3 + 5*5).
   Test totals: 920 lib + 47 e2e + 7 vtables-phase3 passing.
   Closure #226.

   **Vtables Phase 4a (struct field of `dyn Iface`) done 2026-05-25**:
   structs can now declare a field of type `dyn Iface` and
   the field works through both backends. Two small
   plumbing fixes: (1) `c_element_storage(Type::Object)`
   returns `intent_dyn_<Iface>` so struct field declarations
   spell correctly; (2) the per-Iface vtable typedef is now
   forward-declared in two stages — the tag + fat-pointer
   typedef BEFORE struct typedefs (so structs can carry
   `dyn Iface` fields), and the full `struct
   intent_vtbl_<Iface> { ... }` body AFTER struct typedefs
   (so the slot fn-ptrs can reference `Struct_<T>` arg
   types should the interface declare any). LLVM IR's
   named-type model handles forward refs natively; no
   reordering needed. New e2e tests in `vtables_phase3.rs`
   cover the Holder { d: dyn Drawable } pattern through
   both backends. Vec<dyn Iface> (Phase 4b) still pending:
   the checker needs vec-literal element coercion to the
   declared `Vec<dyn Iface>` element type. Test totals:
   920 lib + 47 e2e + 5 vtables-phase3 passing. Closure
   #225.

   **Vtables Phase 3b (tree-LLVM codegen) done 2026-05-25**: lli now
   runs dyn-dispatching programs identically to tree-C. New
   `%intent_vtbl_<Iface>` and `%intent_dyn_<Iface>` named struct
   types in the preamble; per-(T, Iface) trampolines defined as
   `internal` functions that bitcast `i8* self` to the declared
   self shape (load for by-value, raw cast for ref/mut-ref) and
   tail-call `@fn_<T>_<method>`; per-(T, Iface) global vtable
   constants `@intent_vtbl_<Iface>_<T>` initialized with the
   trampoline `@`-names. DynCoerce emits `bitcast %struct_addr to
   i8*` + `insertvalue` chain to build the fat pointer (v1
   restricts source to Var). DynDispatch emits `extractvalue` for
   both vtable and data pointers, `getelementptr` to the slot,
   `load` the fn-ptr, `call` indirect. Type-routing extended:
   `llvm_type_string(Type::Object)` returns `%intent_dyn_<Iface>`
   and `is_scalar` accepts Object so the by-value param path
   alloca-stores it. New e2e test exercises emit_llvm + opt
   -verify + lli + run; lli returns 25 for the same Circle
   program tree-C runs. `compile_to_llvm` added as a public
   API symmetric to `compile_to_c`. Test totals: 920 lib + 47
   e2e + 3 vtables-phase3 passing. Closure #224.

   **Vtables Phase 3a (tree-C codegen) done 2026-05-25**: dyn-using
   programs now produce running C. New `TypedExprKind::DynCoerce
   { value, iface_name, from_type_name, from_ty }` IR node inserted
   by `coerce_checked` whenever a `Struct`/`Enum` value flows into
   a `dyn Iface` slot. tree-C emits `intent_vtbl_<Iface>` typedef
   (struct of fn pointers in declaration order, each taking
   `void* self` as the first arg) + `intent_dyn_<Iface>` fat-
   pointer typedef, gated on whether the interface is actually
   used as `dyn Iface` somewhere (avoids generating bogus
   trampolines for interfaces that only use static dispatch).
   Per-(T, Iface) trampoline functions cast `void* self` to the
   declared self shape (by-value, ref, or mut-ref) and forward
   to the hoisted `fn_<T>_<method>`; the per-(T, Iface) static
   `intent_vtbl_<Iface>_<T>` populates the vtable slots in
   declaration order. `DynDispatch` lowers to
   `recv.vtable->m<slot>(recv.data, args...)`. The coercion
   site materializes `(intent_dyn_<Iface>){ .vtable = &..., .data
   = (void*)&v_<name> }`; v1 restricts source to a Var binding
   so the data address is a stable lvalue. Tree-LLVM intentionally
   panics with a clear "use --backend=c" message — Phase 3b
   (LLVM mirror) is queued. New e2e test `vtables_phase3.rs`
   exercises emit_c + cc + run for a Circle/Drawable program;
   the dispatched call returns 25 for `area(Circle { r: 5 })`.
   Test totals: 920 lib + 47 e2e + 2 vtables-phase3 passing.
   Closure #223.

   **Vtables Phase 2b: `obj.method(args)` dispatch on `dyn Iface` done 2026-05-25**:
   Continued epic A. The checker now recognizes method
   calls on a `dyn Iface` receiver, looks up the method in
   the interface declaration, validates arg arity and types
   against the interface's parameter shape (skipping the
   `self` slot), and emits a new
   `TypedExprKind::DynDispatch { receiver, iface_name,
   method, method_span, slot_index, args }` IR node. The
   `slot_index` is the declaration-order position of the
   method in its interface — Phase 3 codegen will use it to
   index the per-(T, Iface) vtable. A new
   `IFACE_METHOD_REGISTRY` thread-local in
   [`src/ast.rs`](src/ast.rs) holds iface method
   signatures (populated alongside the existing impl
   registry in `hoist_impls_into_functions`). SSA gate +
   `expr_ssa_supported` reject DynDispatch so programs
   that use `dyn Iface` route through the tree backends.
   tree-C / tree-LLVM panic with a clear "vtables Phase 3
   pending" message when they hit DynDispatch in
   `emit_expr`; the checker-only tests in this closure
   never reach the backend. Test totals: 920 lib + 47
   e2e passing. Closure #222.

   **Vtables Phase 2a: `T → dyn Iface` coercion validation done 2026-05-25**:
   Continued epic A. The checker now accepts implicit
   coercion from a concrete `T` to `dyn Iface` when an
   `implement Iface for T` is in scope; rejects with the
   standard "must be assignable to dyn Iface, got T"
   diagnostic when no impl is found. New
   `IFACE_IMPL_REGISTRY` thread-local in
   [`src/ast.rs`](src/ast.rs) populated by
   `hoist_impls_into_functions` BEFORE the impl drain;
   queried from `can_assign` in [`src/checker.rs`](src/checker.rs).
   Method dispatch through the fat pointer (Phase 2b) and
   the actual codegen emit (Phase 3) still pending — programs
   that use `dyn Iface` will type-check but won't link
   yet (tree-C's placeholder `intent_dyn` typedef from #220
   surfaces as a cc error). Test totals: 917 lib + 47 e2e
   passing. Closure #221.

   **Vtables Phase 1: `Type::Object` + `dyn Iface` parsing done 2026-05-25**:
   Started epic A (vtables) per user direction "use with
   original intent, no inheritance". Phase 1 adds the
   type-level recognition only — coercion and codegen are
   Phases 2-3. New `Type::Object(iface_name)` variant
   carries the interface name; parser recognizes
   `dyn IfaceName` contextually (no new lexer token). The
   checker treats `dyn Iface` as a distinct type; assigning
   an unrelated type to a `dyn Iface`-typed binding
   surfaces a clean `must be assignable to dyn Iface, got
   <other>` diagnostic. `is_copy()` returns true (fat
   pointer is two scalar fields). Every `match` arm on
   `Type` across the codebase (~30+ sites) extended to
   handle the new variant. Tree-C's `c_leaf_type` falls
   back to `"intent_dyn"` placeholder — Phase 3 will
   emit the actual per-Iface fat-pointer typedef. Test
   totals: 915 lib + 47 e2e passing. Closure #220.

   **`pop(mut ref xs) -> T` builtin done 2026-05-25**:
   Completes the Vec-as-stack story (push + pop). New
   builtin: `pop(mut ref xs)` aborts on empty Vec,
   otherwise decrements `len` and returns the last
   element. For non-Copy element types (OwnedStr / Vec /
   Struct with heap fields), the returned T carries
   ownership; the Vec's scope-exit `__free` walks
   elements via the post-pop `len` so the moved-out slot
   is not re-freed. tree-C + tree-LLVM helpers emitted;
   SSA gate rejects (routes through tree backends like
   `push_mut`). Array element types (`Vec<[T;N]>`) skip
   the helper — C can't return a bare array by value;
   defer to a follow-up. The design rationale (build
   stack from Vec rather than adding a separate Stack<T>
   type) lives in README's "Design Philosophy &
   Limitations". Test totals: 913 lib + 47 e2e passing.
   Closure #219.

   **Multiple `try`s in one block done 2026-05-25**:
   The desugar from closure #217 still only handled the
   FIRST `Let(try)` in a body — subsequent `try` stmts
   surfaced the "still in progress" diagnostic.
   Refactored to `try_rewrite_block_stmts` which finds
   the first try in a stmt-list, lifts the rest into a
   match's Some-arm Block-expr, and recurses on the
   inner stmts. N `try`s in one body now produce N
   nested matches with the innermost holding the final
   return-expr. Intermediate non-try `let` stmts
   between consecutive `try`s are preserved (e.g.
   `let x = try a; let doubled = x * 2; let y = try b;
    return Opt.Some(doubled + y);`). Test totals: 911
   lib + 47 e2e passing. Closure #218.

   **`try` in nested blocks done 2026-05-25**:
   The v1 `try`-let desugar previously only operated on
   the top-level function body — `body[0]` had to be the
   Let(try), `body[last]` the Return, intermediate stmts
   restricted to Let/Print. Nested `try` (inside an
   if/else/while/for body) surfaced the "still in
   progress" diagnostic. Refactored the desugar into
   `try_rewrite_stmt_list` which recursively walks
   if/else/while/for/for-iter/task bodies AND applies
   the top-level pattern-match at each level. The
   inner-first traversal ensures inner rewrites land
   before the outer pattern-match runs (so e.g. an
   if-body with `[Let(try), Return]` gets rewritten to
   a single `Return(match…)` before the outer fn body
   is inspected). Test totals: 910 lib + 47 e2e
   passing. Closure #217.

   **tree-C nested FnPtr return declarator done 2026-05-25**:
   `fn() -> fn(T) -> R` produced syntactically broken C
     `int64_t (*)(int64_t, int64_t) (*v_p)()`
   because `format_declarator` recursively formatted the
   inner fn-ptr return as a prefix — fn-ptr declarators
   can't appear prefix-only in C. Fix: when the FnPtr's
   return type is itself a FnPtr, emit `void*` for the
   return slot. All fn-ptrs are interchangeable at the
   C storage level (closures #214/#215), so the implicit
   conversion at use sites works (gcc accepts void*↔fn-ptr
   silently). Test totals: 908 lib + 47 e2e passing.
   Closure #216.

   **tree-LLVM Vec<FnPtr> tag fix done 2026-05-25**:
   Parallel to #214 on the LLVM side. tree-LLVM's
   `vec_struct_tag` for FnPtr fell through to
   `llvm_type(FnPtr)` which is `unreachable!` ("use
   llvm_type_string for fn-ptr type") → panic during
   emit. Added `Type::FnPtr(_, _)` arm returning
   `"fnptr"` — all fn-ptrs lower to the same
   `<ret> (<params>)*` LLVM type so one tag works
   regardless of signature. Test totals: 907 lib + 47
   e2e passing. Closure #215.

   **tree-C Vec<FnPtr> identifier-safe typedef done 2026-05-25**:
   `Vec<fn(T) -> R>` element-tag fell through to
   `c_leaf_type(FnPtr).replace(' ', '_')` which returned
   `"void*"` (the `*` survives the replace). The emitted
   typedef `intent_vec_void*` is not a valid C
   identifier; cc rejected with "expected '=', ',', ';',
   'asm' or '__attribute__' before '*' token". Fix: add
   `Type::FnPtr(_, _)` arm to `element_tag` that returns
   the identifier-safe spelling `"fnptr"`. All fn-ptrs
   share the same C representation (`void*` cast in/out
   for indirect calls), so one tag is correct regardless
   of param/return types. Test totals: 906 lib + 47 e2e
   passing. Closure #214.

   **CallIndirect arg move tracking done 2026-05-25**:
   `check_indirect_call` (the fn-ptr call path)
   checked + coerced each arg but never called
   `consume_if_moved_var` or `inject_branch_drops`.
   For a non-Copy arg like `OwnedStr`, the callee
   consumed the heap (freed at fn scope exit) AND the
   caller's scope-exit Drop fired on the same binding
   — ASan-detected double-free at runtime. The regular
   `check_call` already had the consume + inject pair;
   `check_indirect_call` was the outlier. Test totals:
   905 lib + 47 e2e passing. Closure #213.

   **SSA-LLVM gates out Vec<Atomic/Channel> → tree-LLVM done 2026-05-25**:
   SSA-LLVM represents `Atomic<T>` as the alloca *pointer*
   (so subsequent `&counter` references reuse the same
   address), and Channel similarly indirects through the
   struct. Storing a pointer-shaped SSA value into an
   `i32` Vec slot emitted `store i32 %ptr, …` which
   failed the LLVM IR verifier with a type mismatch (the
   element width is the underlying scalar `i32`, not
   `i32*`). Tree-LLVM doesn't have this issue — it goes
   through a different vec emit path. Closure #212 adds
   `stmt_uses_vec_of_atomic_or_channel` to
   `ssa_llvm_extra_reject` so any program containing
   `Vec<Atomic<T>>` / `Vec<Channel<T,N>>` (at any nesting
   depth) routes through tree-LLVM. SSA-C unaffected;
   its emit already handles these correctly. Test
   totals: 904 lib + 47 e2e passing. Closure #212.

   **tree-C Vec<Atomic/Channel> typedef collision done 2026-05-25**:
   `Vec<Atomic<T>>` element-tag fell through to
   `c_leaf_type(Atomic).replace(' ', '_')` which returned
   the hardcoded `_Atomic int64_t` regardless of T →
   typedef `intent_vec__Atomic_int64_t` for ANY
   Vec<Atomic<T>>. Two distinct `Vec<Atomic<T>>` in the
   same program collapsed to one typedef whose `data`
   field had the FIRST T's width → ASan stack-buffer-
   overflow on memcpy when widths differed (e.g. u32 vs
   u8). Same shape for `Vec<Channel<T, N>>` (different
   (T, N) would collide). Fix: add
   `Type::Atomic(element)` and `Type::Channel(element,
   capacity)` arms to `element_tag` so distinct shapes
   get distinct typedef names like
   `intent_vec_atomic_uint32_t` and
   `intent_vec_channel_int64_t_4`. Test totals: 904 lib
   + 47 e2e passing. Closure #211.

   **tree-C RefField const-strip for Mutex/Atomic/Channel done 2026-05-25**:
   When borrowing a struct via `ref T` and then field-
   borrowing a Mutex/Atomic/Channel field (`ref c.lock`),
   the C lowering took the address through a `const T*`
   pointer → `const Mutex*` operand. The runtime helper
   `intent_mutex_i64_lock` (and Atomic/Channel ops) takes
   a non-const pointer — atomic-style ops are inherently
   mutating even via a read-only borrow. gcc warned
   `-Wdiscarded-qualifiers`. Closure #176 already handled
   direct `ref Mutex/Channel/Atomic` params; #210 covers
   field-borrow through a `ref Struct`. Tree-C now emits
   `(intent_mutex_i64*)&v_c->lock` with an explicit const-
   strip cast. -Wdiscarded-qualifiers sweep over all 58
   examples clean. Test totals: 903 lib + 47 e2e passing.
   Closure #210.

   **tree-C Atomic<T> struct field element width fix done 2026-05-25**:
   Parallel to #208 for `Atomic<T>` as a struct field.
   The c_leaf_type fallback returned `_Atomic int64_t`
   for any Atomic; an `Atomic<u32>` field declared at
   i64 width would have wrong memory size / alignment /
   lock-free properties vs the declared type
   (functionally tolerated at runtime via implicit
   conversion, but type-incorrect). Fix: add an
   `Atomic(element)` arm to `c_element_storage` that
   calls `c_atomic_storage(element)` → `_Atomic
   <c_leaf_type(element)>`. Test totals: 902 lib + 47
   e2e passing. Closure #209.

   **tree-C Channel<T,N> struct field capacity fix done 2026-05-25**:
   `Channel<T, N>` as a struct field emitted with the
   hardcoded fallback type `intent_channel_int64_t_16`
   because `c_element_storage` fell through to
   `c_leaf_type` for Channel, and `c_leaf_type(Channel)`
   returns the 16-capacity fallback (the comment there
   explicitly notes callers must special-case Channel).
   A field of `Channel<i64, 4>` therefore didn't match
   the constructor's `intent_channel_int64_t_4_new()`
   return type and cc rejected with "incompatible types".
   Same shape for non-i64 Channel element types. Fix:
   add a `Channel(elt, cap)` arm to `c_element_storage`
   that calls `c_channel_storage(elt, cap)`. Test
   totals: 901 lib + 47 e2e passing. Closure #208.

   **tree-C Block-expr user-Drop for Copy structs done 2026-05-25**:
   tree-C's Block-expr Drop emit (inline arm for non-
   stmt-level Drops) had a Struct branch that walked
   per-field free chains but never checked
   USER_DROP_REGISTRY. For a Copy-but-user-Drop struct
   (e.g. `Resource { id: i64 }` plus `implement Drop`),
   the per-field walk emitted nothing and the user's
   drop method was silently skipped at Block-expr scope
   exit. The regular stmt-level Struct Drop handler at
   backend_c.rs:1965-1987 already had the user-Drop
   check; the Block-expr inline arm needed the same.
   Tree-LLVM unaffected (its Block-expr emit forwards
   Drop to emit_stmt which already handles user-Drop).
   Test totals: 900 lib + 47 e2e passing. Closure #207.

   **SSA-C parallel-for post-loop counter uses end-bound done 2026-05-25**:
   Per OpenMP, the iteration variable inside `omp parallel
   for` is implicitly private — reading its value AFTER
   the loop is undefined. SSA-C's
   `emit_parallel_for_region` propagated header→exit
   block-args literally, so a Phi capturing the post-loop
   counter value rendered as `v_3 = v_2` where v_2 is the
   (now-undefined) counter. gcc warned `v_2 is used
   uninitialized`. Fix: substitute the counter operand
   with the loop's `end` operand when emitting exit-arg
   assignments — the well-defined post-loop value is
   exactly the loop bound (parallel-for forbids `break`
   per closure #190). Test totals: 899 lib + 47 e2e
   passing. Closure #206.

   **tree-C match-on-bool cast for switch done 2026-05-25**:
   gcc warns `switch condition has boolean value`
   (-Wswitch-bool) when the dispatch expression is
   bool-typed. Tree-C's Match emit passed the bool
   scrutinee directly to `switch(…)` with `case 0` /
   `case 1` arms. Fix: cast bool scrutinees to int
   (`switch ((int)v_b)`) so the canonical 0/1 dispatch
   is unambiguous. Test totals: 898 lib + 47 e2e
   passing. Closure #205.

   **SSA-C omits unused block labels done 2026-05-25**:
   SSA-C emitted a `bbN:` label for EVERY block,
   including the entry block of a straight-line fn that
   no `goto` ever targets. gcc warned "label 'bb0'
   defined but not used" (-Wunused-label) and the noise
   hid real diagnostics. Fix: pre-scan all terminators
   (Jump, Branch) plus special-region targets (parallel-
   for exit, multi-block task end) to build a
   `referenced_blocks` set, then emit a label only for
   blocks in that set. -Wunused-label sweep over all 58
   examples now clean. Test totals: 897 lib + 47 e2e
   passing. Closure #204.

   **tree-C array-payload no-variant brace-init done 2026-05-25**:
   `.payload = 0` for an enum whose payload is an array
   type (e.g. `Window.Closed` when `Window` carries an
   `[i64; 4]` payload) was tripping `-Wmissing-braces`
   and is technically ill-formed C — an array can't be
   initialized from a bare integer (gcc accepts via the
   zero-fill extension; stricter compilers reject).
   Tree-C's payload-less variant emit had brace-init for
   Vec/Tuple/Struct but not Array. Added Array to the
   brace-init list — emits `.payload = {0}`. Test
   totals: 896 lib + 47 e2e passing. Closure #203.

   **SSA-C empty-param prototype `(void)` done 2026-05-25**:
   SSA-C emitted `fn_main()` with empty parens for no-
   arg functions. In C, empty parens mean "unspecified
   prototype" (K&R style), not "no args" — tripping
   `-Wstrict-prototypes` and breaking -Werror builds.
   Both `emit_function_prototype` and `emit_function`
   now write `(void)` when `f.params` is empty,
   mirroring what tree-C's `emit_params` already did.
   `-Wstrict-prototypes` sweep over all 58 examples now
   clean. Test totals: 895 lib + 47 e2e passing.
   Closure #202.

   **Block-expr Let RHS move tracking done 2026-05-25**:
   The Block-expr `Stmt::Let` arm (closure #129 MVP)
   never called `consume_if_moved_var(rhs, …)`, so
   `let n = b.name` (partial-move) or `let n = outer_var`
   (Var move) inside a Block-expr didn't propagate the
   move into the env. The struct's per-field free / outer
   Var's scope-exit free then double-freed the heap that
   the moved-out binding's drop ALSO freed — ASan ABORT.
   Fix: mirror the regular fn-body Let arm — call
   `consume_if_moved_var(rhs, &rhs_checked, env)` then
   `inject_branch_drops(&mut rhs_checked.expr)`. Test
   totals: 894 lib + 47 e2e passing. Closure #201.

   **Block-expr `let _ = …` Discard handling done 2026-05-25**:
   The Block-expr `check_expr` arm always called
   `env.insert_current(name)` and emitted
   `TypedStmt::Let`. For `name == "_"`, two consecutive
   discards collided on the synthetic name (`v__`
   redefined) and the fresh OwnedStr/Vec result leaked
   because Discard wasn't on the Block emit's accepted
   arm list. Fix: detect `name == "_"` in the Block-expr
   Let arm and emit `TypedStmt::Discard { expr }` —
   mirrors the regular fn-body Let path (closure #134).
   Tree-C Block emit grew a Discard arm covering
   OwnedStr/Vec/Struct/Enum with brace-scoped tmps;
   tree-LLVM Block emit now forwards Discard to
   `emit_stmt` alongside Print/Drop. Test totals: 893
   lib + 47 e2e passing. Closure #200.

   **Block-expr shadow-name false-move done 2026-05-25**:
   When the OUTER `consume_if_moved_var` walked into a
   Block-expr's tail (closure #174), the inner scope had
   already been popped. `env.lookup_mut(name)` then walked
   past the gone inner shadow and marked an outer-scope
   binding of the same name as moved — surfacing a
   spurious "value 'a' was moved" diagnostic on
   subsequent uses of the outer `a`. Closure #194's
   inner `consume_if_moved_var` already marked the inner
   binding before pop_scope; closure #199 plugs the
   outer recursion: skip the recursion when the Block's
   tail is a `Var(name)` and the same Block declares a
   `Let` with that name. Test totals: 892 lib + 47 e2e
   passing. Closure #199.

   **tree-C tuple-shape collection in control flow done 2026-05-25**:
   `collect_tuple_shapes_in_expr` handled Tuple/
   TupleAccess/Unary/Binary/Call/ArrayLit/Cast/Index/
   Len/CallIndirect but fell through `_ => {}` for
   Block/IfExpr/Match. A tuple type that only appeared
   inside a Block-expr inner Let (`let p: (i64, i64) =
   (1, 2)`) never had its `intent_tuple_<…>` typedef
   emitted and cc rejected with `unknown type name
   intent_tuple_<…>`. The Vec walker already handled
   Block/IfExpr/Match arms (closure #129); the tuple
   walker was the outlier. Mirrored the same three
   arms. Test totals: 891 lib + 47 e2e passing.
   Closure #198.

   **Block-expr inner type-alias resolution done 2026-05-25**:
   Parallel to closure #196 for the type-alias
   substitution pass. `sub_aliases_in_stmt` had the same
   pre-existing limitation — it never descended into a
   Stmt's `expr` field, so any `let p: AliasName = …;`
   inside a Block-expr kept the unsubstituted alias and
   the checker rejected with the unresolved-name
   diagnostic. New `sub_aliases_in_expr` walks every
   expression shape and recurses through nested Lets,
   mirroring the #196 enum walker. Test totals: 890 lib
   + 47 e2e passing. Closure #197.

   **Block-expr inner enum-let annotation resolution done 2026-05-24**:
   `resolve_enum_types_in_stmt` walked top-level fn
   bodies and the bodies of `if`/`while`/`for`/`for-iter`
   /task — but never descended into a Stmt's `expr`
   field, so any Let inside a Block-expr (e.g. `let r =
   { let a: Maybe = …; … }`) kept its annotation as
   `Type::Struct("Maybe")` instead of being resolved to
   `Type::Enum("Maybe")`. `coerce_checked` then got
   actual=Type::Enum, target=Type::Struct, both rendered
   as "Maybe", and rejected with "let initializer must
   be assignable to Maybe, got Maybe" — a confusing
   identical-text diagnostic. Fix: extend
   `resolve_enum_types_in_stmt` to call a new
   `resolve_enum_types_in_expr` for every expression
   field, and have the expr walker descend into Block,
   IfExpr, Match, Cast, Binary, Call, Tuple, StructLit,
   FieldAccess, Try, etc. Test totals: 889 lib + 47
   e2e passing. Closure #196.

   **inject_branch_drops skips inner Block decls done 2026-05-24**:
   With closure #194's tail-spill landing `let
   __block_tail_<span> = …` inside each Block-expr,
   the `if-expr cond { Block1 } else { Block2 }` shape
   broke at codegen because
   `collect_branch_var_leaves` was treating the inner
   spill Var as a "leaf" of the branch and
   inject_branch_drops then emitted `Drop
   __block_tail_<span>` in the OTHER branch — where
   that name isn't declared. cc rejected with
   `undeclared identifier v___block_tail_<n>`. Fix in
   `collect_branch_var_leaves`: when descending into
   `Block { stmts, tail }`, filter out any Var name
   that a `Let` inside the same Block introduces. The
   filter is symmetric for spill Vars (synthetic) and
   user-declared inner Vars. Test totals: 888 lib +
   47 e2e passing. Closure #195.

   **Block-expr sibling-let scope-exit drops done 2026-05-24**:
   `let r = { let a = …; let b = …; a };` was leaking
   b's heap. The Block-expr type-checker pushed and
   popped a scope but never called
   `emit_current_scope_drops`, so sibling lets that the
   tail neither consumed nor moved were never freed.
   Fix in `check_expr` for `ExprKind::Block`: after the
   tail is checked, call `consume_if_moved_var(tail,
   …)` to propagate tail-Var moves into the inner
   scope, then synthesize Drop stmts for the remaining
   non-moved non-Copy bindings. When drops exist, spill
   the tail into a synthetic `__block_tail_<span>` Let
   so the Drops fire AFTER the tail evaluates (avoids
   UAF for tails that borrow a sibling, e.g.
   `{ let a = …; len(a) }`). When the tail already
   consumes every sibling (binary concat, fn args), the
   drops list is empty and no spill is emitted. Both
   tree-C and tree-LLVM benefit since the Block emit
   was already wired for Drop stmts (closures #160,
   #192, #193). Test totals: 887 lib + 47 e2e passing.
   Closure #194.

   **tree-C Block Drop Enum: tag switch + payload free done 2026-05-24**:
   Parallel to closure #192's Struct arm. Block-expr
   Drop for a payloaded enum needed to switch on the
   active tag and free the heap payload (OwnedStr /
   Vec). Inject_branch_drops's branch-wrap left enum-
   typed Vars in the unchosen branch with their
   payload heap leaked. Added the Enum arm: emits a
   `switch (v_name.tag) { case T1: free_call; break;
   default: break; }` form (same shape as the Reassign
   Enum drop in closure #147). Test totals: 885 lib +
   47 e2e passing. Closure #193.

   **tree-C Block Drop Struct emits field chain done 2026-05-24**:
   tree-C's Block-expression emit (used by
   inject_branch_drops's branch-wrap from closures #179
   / #180) handled `Drop OwnedStr` and `Drop Vec` arms
   but fell through `_ => {}` for `Drop Struct` — leaking
   the unchosen branch's heap on if-expr / match Var
   branches with struct types. Added the Struct arm:
   walks the STRUCT_FIELDS_REGISTRY and emits the per-
   field free chain (mirrors `emit_struct_field_drops`
   used by Stmt::Drop). Test totals: 884 lib + 47 e2e
   passing. Closure #192.

   **Task body_blocks via CFG reachability done 2026-05-24**:
   Task body containing a for-loop with `continue` was
   failing SSA-C and SSA-LLVM emit. The task region
   collection used `(begin_id..=end_id)` for body_blocks
   — a contiguous range. Closures #185 / #187 step
   blocks plus if-then/else/merge blocks created later
   in the same body got BlockIds beyond end_block, so
   they fell outside the range. Parent (fn_main) emitted
   them with goto-targets pointing at skipped step
   blocks → undefined-label errors. Fix: walk the CFG
   from begin_block, collecting all reachable blocks
   without following end_block's successors. Mirrored
   in both SSA-C and SSA-LLVM. Test totals: 883 lib +
   47 e2e passing. Closure #191.

   **Parallel-for body rejects break done 2026-05-24**:
   `break` inside a `parallel for` body must be rejected
   — OpenMP's `parallel for` pragma forbids early exit
   from worker iterations. The C backend forwarded
   `break;` directly into the `_Pragma("omp parallel
   for")` loop, and gcc/clang rejected with "break
   statement used with OpenMP for loop". Tree-LLVM
   accepted it with ambiguous cross-thread semantics.
   Checker now diagnoses break inside a parallel-for
   body with a clear message pointing at the
   Mutex<bool>-guarded flag workaround. `continue` is
   still allowed (OpenMP accepts it; the #185-#189 fixes
   ensure correct increment). Test totals: 882 lib + 47
   e2e passing. Closure #190.

   **tree-LLVM parallel-for outlined fn continue done 2026-05-24**:
   tree-LLVM's outlined parallel-for (`@__intent_par_<N>`
   invoked via @GOMP_parallel / CreateThread) didn't push
   a LoopFrame onto its FnCtx — `continue` inside the
   body fell through to the "outside a loop" no-op
   branch, then continued past the if-merge into the rest
   of the loop body. `total = total + 1` ran on every
   iteration regardless of the continue, breaking
   reduction totals. Pre-existing bug; SSA-LLVM falls
   back to tree-LLVM for multi-block parallel-for bodies
   so the LLVM run hit this path. Fix mirrors closures
   #185–#188: push a LoopFrame with header=step, emit a
   step block that loads-bumps-stores i_addr then jumps
   to hdr. Both natural body-end and `continue` jump to
   step. Test totals: 881 lib + 47 e2e passing. Closure
   #189.

   **tree-LLVM for-range continue emits step block done 2026-05-24**:
   tree-LLVM's `TypedStmt::For` (range form) had the same
   continue-infinite-loop bug as the for-iter form
   (#186) and SSA paths (#185, #187). `continue` jumped
   straight to for_header with i_addr unchanged →
   infinite loop. Pre-existing bug since tree-LLVM range-
   for shipped. Now uses a `for_step` block between body-
   end and header for the increment; both continue and
   natural fallthrough jump to step. Test totals: 880
   lib + 47 e2e passing. Closure #188.

   **SSA for-range continue + parallel-for shape done 2026-05-24**:
   `for i from start to end` (range form, lowered via
   `lower_integer_for`) had the same continue-infinite-
   loop bug as the for-iter form fixed in #185.
   Restructured with the same `step` block shape: step
   bumps the counter, jumps to header. Body's natural
   end and `continue` both jump to step. LoopFrame's
   header is step (the continue target).
   `ParallelForShape` grew a `step_block` field so the
   SSA-C / SSA-LLVM parallel-for recognizers can absorb
   step into the OpenMP / outlined-fn region — they now
   skip step alongside header / body. Without this
   update, the C / LLVM emit referenced step as a
   free-standing basic block with `goto bb_step;` that
   no other block defined. Test totals: 879 lib + 47
   e2e passing. Closure #187.

   **tree-LLVM for-iter continue emits step block done 2026-05-24**:
   tree-LLVM had the same continue-infinite-loop bug as
   SSA (closed in #185). `continue` jumped straight to
   iter_header with i_addr unchanged → infinite loop.
   Pre-existing bug since tree-LLVM for-iter shipped.
   Fix mirrors #185: introduce `iter_step` block that
   bumps i_addr then jumps to header. LoopFrame's header
   points to step (continue target). Body's natural end
   also jumps to step. Tree-C is unaffected (uses C's
   native `for (i = 0; i < len; i++)` form). Test totals:
   878 lib + 47 e2e passing. Closure #186.

   **SSA for-iter continue increments counter done 2026-05-24**:
   `continue` inside an SSA for-iter was jumping straight
   to the header block with the OLD i_header value — the
   increment only happened on the natural body-
   fallthrough path. Every `continue` re-entered the
   same iteration → infinite loop (hang at runtime).
   Pre-existing bug from when SSA for-iter was added.
   Fix: introduce a `step` block between body-end and
   header. Step takes the carry params, increments idx,
   then jumps to header. Both the natural fallthrough
   and `continue` jump to step (with the OLD i_header)
   so the increment fires uniformly. LoopFrame.header
   now points to step (the continue target), with the
   step's carry params replacing the header's. Test
   totals: 877 lib + 47 e2e passing. Closure #185.

   **SSA consuming for-iter emits buffer Drop on normal exit done 2026-05-24**:
   `for x in xs` (consuming form, Vec of Copy elements)
   flowing through SSA wasn't emitting any Drop for the
   consumed buffer — the checker marks the source as
   moved and SSA's lower_for_iter ignored the consumes
   flag. On normal loop completion the outer buffer
   leaked. SSA's gate already routes Vec<non-Copy> to
   tree backends (closure #159), so SSA only sees
   Vec<Copy>; `intent_vec_<T>__free` IS the shallow free
   for Copy elements. Emit an InstrKind::Drop at the
   loop's exit block. Test totals: 876 lib + 47 e2e
   passing. Closure #184. Known remaining: early
   `return` from inside the body still skips this Drop
   (tracked in STATUS.md known-issues).

   **`is_fresh_owned_str` refined for if-expr Var branches done 2026-05-24**:
   `print if cond { a } else { b };` (a, b: OwnedStr
   Vars) was double-freeing. `is_fresh_owned_str` /
   `is_fresh_non_copy` used a kind-only whitelist that
   returned true for any IfExpr / Match / Block,
   regardless of contents. Print's "free fresh result
   after use" path then freed the Var's heap; scope-exit
   freed it again. Refined to recurse into branches: an
   if-expr / match / block is fresh only when EVERY
   leaf is a fresh non-Copy producer (Call or Binary).
   Var leaves disqualify. Test totals: 875 lib + 47
   e2e passing. Closure #183.

   **inject_branch_drops at push/set xs arg done 2026-05-24**:
   `push(if cond { xs1 } else { xs2 }, v)` and `set(if
   cond { xs1 } else { xs2 }, i, v)` were leaking the
   unchosen Vec. The builtin handlers had wired
   inject_branch_drops into the value arg (closure #180)
   but not the Vec arg. Symmetric fix. Test totals: 874
   lib + 47 e2e passing. Closure #182.

   **inject_branch_drops at Return stmt done 2026-05-24**:
   `return if cond { a } else { b };` (a, b non-Copy
   Vars) was still leaking. inject_branch_drops was
   wired into Let / Reassign / Index / Field / Call /
   Method / vec / push / set / enum payload (#179, #180)
   but the Return-stmt arm was missed. One-line addition
   right before `try_elide_bounds_in_typed_expr` on the
   return expression. Test totals: 873 lib + 47 e2e
   passing. Closure #181.

   **inject_branch_drops at remaining consume sites done 2026-05-24**:
   Extends closure #179's structural-rewrite to the rest
   of the consume_if_moved_var sites: named-function
   Call args, MethodCall args (via Type-associated and
   `obj.method()` paths), StructLit field values,
   EnumVariantWithPayload constructor arg, `vec(…)`
   element args, `push()` value, and `set()` value.
   Same wrap-each-branch-in-Block-with-Drops pattern as
   #179. Each site now adds the inject after consume,
   so `f(if cond { a } else { b })` and similar shapes
   no longer leak the unchosen alternative. Test
   totals: 872 lib + 47 e2e passing. Closure #180.

   **If-expr / match Var-branch unchosen leak fixed done 2026-05-24**:
   Closes the unchosen-alternative leak left behind by
   the conservative move-tracking from closures
   #172/#173. The checker now rewrites if-expr / match /
   block-tail typed expressions so each branch wraps its
   chosen value in a Block that drops the OTHER
   branches' Var leaves before yielding. C ternary form:
   `cond ? ({ free(v_b); v_a; }) : ({ free(v_a); v_b; })`.
   LLVM emits the equivalent through the Block emitter
   (closure #160 already wired Block Drop forwarding).
   The rewrite is wired into Let, Reassign, IndexAssign,
   and FieldAssign — the most common move contexts.
   inject_branch_drops walks IfExpr, Match, and Block
   recursively so nested patterns work too. Test totals:
   871 lib + 47 e2e passing. Closure #179.

   **`Enum.Some(v)` consumes Var payload done 2026-05-24**:
   `Maybe.Some(n)` where n is a Var of OwnedStr was
   double-freeing on scope exit. The
   EnumVariantWithPayload constructor transfers
   ownership of the payload into the tagged-union, but
   `check_call`'s enum-constructor branch never called
   `consume_if_moved_var` on the payload arg. Source
   Var's scope-exit Drop fired AFTER the constructor
   stored the payload pointer, and the enum's drop
   re-freed the same heap. Same family as vec / push /
   set (#171, #177). One-line addition. Both backends
   were affected (checker/IR-level bug). Test totals:
   870 lib + 47 e2e passing. Closure #178.

   **`vec(a, b, …)` consumes Var element args done 2026-05-24**:
   `let xs: Vec<OwnedStr> = vec(a, b);` (a, b: Var
   OwnedStr) was double-freeing on scope exit. The
   vec() builtin transfers each Var's heap into the
   new Vec's slot, but `check_vec_builtin` never called
   `consume_if_moved_var` on its element args — the
   source Var's scope-exit Drop fired AFTER vec()
   already stored the pointer, and the Vec's __free
   re-freed each slot. Same family as push / set
   (closure #171); one-line addition in the
   element-coerce loop. Both backends were affected
   (checker/IR-level bug). Test totals: 869 lib + 47
   e2e passing. Closure #177.

   **SSA-C `ref Channel<T,N>` param drops `const` done 2026-05-24**:
   `fn produce(ch: ref Channel<i64, 16>, v: i64)` was
   declared as `const intent_channel_int64_t_16*`. The
   shared `intent_channel_*_send` / `_recv` helpers take
   a NON-const pointer (they bump seq counters and idx
   atomically through the cell pointer). Every send /
   recv site raised -Wdiscarded-qualifiers. Atomic refs
   already dropped `const` (the closest analogue);
   Channel now mirrors that. Caught via `cc -Wall -Wextra
   -c` on the concurrency example. Test totals: 868 lib
   + 47 e2e passing. Closure #176.

   **SSA-C `OwnedStr` declared `char*` not `const char*` done 2026-05-24**:
   SSA-C lumped `Str` and `OwnedStr` together as `const
   char*`. The shared Vec helper bundle declares the
   data field as `char* data`, so storing an OwnedStr
   value into a slot raised -Wdiscarded-qualifiers on
   every IndexAssign / Reassign / push / set. Runtime
   was fine (const is purely a compile-time tag) but
   the noise hid real warnings. Split: `Str` keeps
   `const char*` (borrowed read-only), `OwnedStr`
   becomes `char*` (heap-owning, mutable — matches
   tree-C). Test totals: 867 lib + 47 e2e passing.
   Closure #175.

   **Block-expr Var tail consumes source Var done 2026-05-24**:
   `let b = { let _x = 1; a };` (a: OwnedStr Var) was
   double-freeing on scope exit. The Block's tail
   expression yields a's value into b, so b aliases a's
   heap; both Vars' scope-exit drops then fire. Same
   shape as closures #172/#173 — `consume_if_moved_var`
   covered Var, FieldAccess, IfExpr, and Match but
   Block fell through. Now the tail is recursively
   consumed. Closure #174. Test totals: 866 lib + 47
   e2e passing.

   **Match arms returning Var consume all arms done 2026-05-24**:
   Same shape as closure #172 but for match scrutinees
   that stay as `TypedExprKind::Match` (integer / enum /
   bool). `let chosen = match n { 1 then a, 2 then b, _
   then c };` was double-freeing because the codegen
   switch makes v_chosen alias one of the Vars and the
   scope-exit drops of every Var plus v_chosen all hit
   the same heap. `consume_if_moved_var` now recurses
   into every arm's body the same way it recurses into
   if-expr branches. Str scrutinees were already
   covered through check_match_str's IfExpr-chain
   desugar. Conservative: unchosen-arm Vars leak (same
   TODO as the if-expr case). Test totals: 865 lib +
   47 e2e passing. Closure #173.

   **If-expr Var branches consume both Vars done 2026-05-24**:
   `let chosen = if cond { a } else { b };` (a, b: Vars
   of OwnedStr) was double-freeing on scope exit. The
   codegen ternary `cond ? v_a : v_b` makes v_chosen
   alias the chosen Var's heap, so the scope-exit drops
   of v_a, v_b, AND v_chosen all hit the same heap.
   `consume_if_moved_var` only descended into bare Var
   and FieldAccess sources — IfExpr fell through `_ =>
   {}`. Now it recurses into both branches and marks
   each branch's Var moved. Conservative: the UNCHOSEN
   alternative leaks (its heap isn't freed since the Var
   is marked moved). Both backends were affected
   (checker/IR-level bug). Closure #172. Known
   remaining: unchosen-alternative leak (tracked in
   TODO.md). Test totals: 864 lib + 47 e2e passing.

   **`push(xs, v)` / `set(xs, i, v)` consume value Var done 2026-05-24**:
   `push(xs, v)` and `set(xs, i, v)` where `v` is a Var
   of OwnedStr (or any non-Copy heap-owner) were
   double-freeing on scope exit. The checker's builtin
   handlers consumed `args[0]` (the Vec) via
   `consume_if_moved_var` but never the value arg —
   so the source Var stayed "live" and its scope-exit
   drop fired AFTER push transferred ownership into the
   new Vec's slot, freeing the heap a second time when
   the Vec was later __free'd. ASan caught it on a
   chained `let xs2 = push(xs, v); let xs3 =
   push(xs2, w);`. Both backends were affected
   (checker/IR-level bug). Two-line fix: also call
   `consume_if_moved_var` on the value arg in push() and
   set(). Test totals: 863 lib + 47 e2e passing.
   Closure #171.

   **tree-LLVM nested FieldAssign drops old heap done 2026-05-24**:
   `o.inner = NewInner { name: "fresh" + "" };` (struct-
   typed field on a struct that already owns heap) was
   leaking the OLD nested struct's heap-owning fields.
   Tree-LLVM's FieldAssign had drop-old arms for OwnedStr
   and Vec (closure #132); Struct and Enum fell through
   `_ => {}`. Tree-C had the parallel arms via closure
   #148. Added Struct arm (walks the OLD nested struct's
   heap-owning fields via emit_llvm_struct_field_drops
   before overwriting) and a defensive Enum arm (mirrors
   the Reassign Enum drop in closure #169; the checker
   currently gates enum-as-struct-field but kept for
   parity). Test totals: 862 lib + 47 e2e passing.
   Closure #170.

   **tree-LLVM Reassign of struct/enum drops old heap done 2026-05-24**:
   `b = Box { name: "second" + "" };` (where `b: Box` has
   `name: OwnedStr`) and `m = Maybe.Some("second" + "");`
   (payloaded enum) were leaking the OLD value's heap.
   Tree-LLVM's Reassign drop_old match only had arms for
   Vec and OwnedStr; Struct / Enum fell through `_ => {}`.
   Tree-C had the parallel arms via closure #147. Added
   the Struct arm (walk OLD alloca's heap-owning fields
   via emit_llvm_struct_field_drops) and the Enum arm
   (load OLD tagged-union, branch on tag, free heap
   payload if active — mirrors the Drop handler's Enum
   arm). Test totals: 861 lib + 47 e2e passing. Closure
   #169.

   **tree-LLVM Discard of OwnedStr frees heap done 2026-05-24**:
   `let _ = s;` (s: OwnedStr) was leaking. The Discard
   handler's OwnedStr arm sat AFTER the `is_scalar(&expr.ty)`
   arm, but `is_scalar(Type::OwnedStr)` returns true, so the
   scalar arm consumed the branch — it just calls
   `emit_expr` and discards the SSA value, never freeing
   the heap. Same shape as the Struct fix (closure #145)
   that already moved its arm BEFORE `is_scalar`. Now
   OwnedStr is checked first too. Test totals: 860 lib +
   47 e2e passing. Closure #168.

   **tree-LLVM `xs[i] = v` drops old slot done 2026-05-24**:
   `emit_leaf_overwrite_drop` had an early-return on
   `field_path.is_empty()` so the bare-leaf IndexAssign
   on a `Vec<OwnedStr>` / `Vec<Vec<T>>` skipped freeing
   the old slot entirely — the previous element's heap
   leaked. The early-return was originally meant to gate
   the deep mixed-place path; turns out the OwnedStr /
   Vec arms work for both shapes since `p` is the slot
   pointer in either case. Removing the guard fixes the
   leak; Copy element types stay no-ops via the wildcard
   match arm. SSA-C handled this through its own
   `c_element_drop_old` call, tree-C through a separate
   IndexAssign path. Verified ASan-clean. Test totals:
   859 lib + 47 e2e passing. Closure #167.

   **FieldAssign marks RHS Var moved done 2026-05-24**:
   `self.name = n;` inside `fn set_name(self: mut ref T, n:
   OwnedStr)` was double-freeing the new heap. The C
   output ran `free(self->name)` (correct old-slot drop),
   stored `v_n` into the slot (correct), then on the
   method's scope exit ran `free(v_n)` — freeing the heap
   the field now owns. ASan caught it as heap-use-after-
   free on the next read of `b.name`. The checker's Let /
   Reassign / Call-arg arms already call
   `consume_if_moved_var` to mark the RHS Var moved when
   it owns non-Copy heap. FieldAssign was missing that
   call. One-line addition. Verified ASan-clean on both
   backends. Test totals: 858 lib + 47 e2e passing.
   Closure #166.

   **Field-borrow through ref-typed self done 2026-05-23**:
   `ref self.items` inside a method declared `self: ref T`
   was broken on both backends. Tree-C emitted
   `&v_self.items` — gcc rejected it with "request for
   member 'items' in something not a structure" since
   v_self is `Struct_T*`. Tree-LLVM emitted
   `getelementptr %Struct_T*, %Struct_T** %arg_self, …`
   which lli rejected because %arg_self IS %Struct_T*,
   not a pointer-to-pointer. Both bugs stemmed from
   RefField/RefMutField carrying only the binding's
   name — not its type — so backends couldn't tell
   whether the binding was owned (use `.`) or borrowed
   (use `->`). Fix: add `object_ty: Type` to RefField /
   RefMutField in the IR; tree-C picks `.` vs `->` from
   `object_ty.is_any_ref()`; tree-LLVM derefs object_ty
   before spelling the GEP source type so the
   indirection level matches. Test totals: 857 lib +
   47 e2e passing. Closure #165.

   **tree-C struct typedefs topologically sorted done 2026-05-23**:
   Source order was emitting `typedef struct {
   Struct_Inner inner; } Struct_Outer;` BEFORE
   `Struct_Inner` was declared. cc rejected the output
   with "unknown type name 'Struct_Inner'". The emit
   loop now does a DFS over the struct dependency graph
   (direct `Struct(S)` field or `[S; N]` field), so
   dependencies come first. Vec / Ref / Atomic / Mutex /
   Guard / Channel / Tuple all introduce pointer-shaped
   indirection through their own typedef bundles, so
   they don't drive struct dependencies. LLVM's IR
   forward-declares named types so tree-LLVM was
   unaffected. Test totals: 856 lib + 47 e2e passing.
   Closure #164.

   **tree-LLVM `t.items[i]` for Vec field done 2026-05-23**:
   `b.items[1]` (FieldAccess base, Vec element type) was
   panicking the tree-LLVM Index handler with
   `unreachable!("Index on unsupported base")`. The
   handler already had a FieldAccess arm for Array-typed
   fields; the parallel Vec arm now reuses
   emit_lvalue_addr to get the field-pointer (which is
   itself the Vec struct address), GEPs into .data, loads
   the element pointer, GEPs at the dynamic idx, and
   loads. Same shape is reachable whenever an OwnedStr
   concat or a clone_at sibling forces an SSA fallback.
   Test totals: 855 lib + 47 e2e passing. Closure #163.

   **tree-LLVM `len` on field forms done 2026-05-23**:
   Closure #161 fixed `len(ref xs)` (Ref/RefMut wrapping
   Var). This closes the other two shapes that also fell
   through to the static-length fallback in tree-LLVM:
   `len(ref t.items)` / `len(mut ref t.items)` (RefField /
   RefMutField) and `len(t.items)` (FieldAccess yielding a
   Vec). Both shapes hit lli's verifier with the `i64 0`
   operand the fallback emitted, crashing the program
   before it could run. Field-borrow forms reuse the
   field-pointer that `emit_expr` already materializes;
   bare field-access calls `emit_lvalue_addr` to get a
   pointer to the field. Both then GEP into the Vec's
   `.len` slot (field index 1) and load. Test totals:
   854 lib + 47 e2e passing. Closure #162.

   **tree-LLVM `len(ref Vec)` GEP+load fix done 2026-05-23**:
   `emit_expr` Len handler only matched `array.kind ==
   Var(name)`. When the source spelled the argument as
   `len(ref xs)`, the typed expression is `Len { array:
   Ref { name = "xs" } }`, so the Var arm was skipped and
   the handler fell through to a `format!("{}", length)`
   fallback. `length` carries the static array length —
   for Vec it's always 0, so any `len(ref xs)` on a Vec
   that landed on tree-LLVM (e.g. when a sibling
   expression forced an SSA fallback) returned 0 instead
   of the real length. Now the Ref/RefMut(name) case
   resolves to the same alloca address as Var(name) and
   takes the GEP-into-.len + load path. Test totals: 853
   lib + 47 e2e passing. Closure #161.

   **tree-LLVM Block-expr emits Drop stmts done 2026-05-23**:
   `match <fresh OwnedStr> { … }` was leaking the
   scrutinee's heap on tree-LLVM. The checker's
   `check_match_str` desugar wraps the if-chain in a
   Block { Let temp = scr; Let result = ifchain; Drop
   temp; result } (closure #137), so the temp gets
   released after the if-chain evaluates. Tree-C's
   Block emitter handled the Drop arm; tree-LLVM's
   emitter only routed `Let` and `Print` through
   `emit_stmt` — the Drop was silently discarded.
   Fix: extend the Block emitter to also forward
   `TypedStmt::Drop` to `emit_stmt`, which already
   knows how to free OwnedStr / Vec / Struct / Enum
   bindings registered in `ctx.locals` (each Let stmt
   above the Drop puts the binding's alloca address
   there). Verified clean under
   `-fsanitize=address,leak` for `match make_owned()
   { "abcdef" then 1, _ then 0 }`. Closure #160.

   **Consuming `for x in xs` on `Vec<non-Copy>` shallow-frees done 2026-05-23**:
   `for x in xs` (consuming form) over `Vec<OwnedStr>` /
   `Vec<Vec<T>>` / `Vec<Struct{heap}>` /
   `Vec<Enum{heap-payload}>` was double-freeing the
   per-element heap. Each iteration loaded the slot into
   `x` and freed it via `x`'s scope-exit drop; then the
   post-loop code called `intent_vec_<T>__free(xs)` which,
   after closure #127, walks every slot and frees its
   inner heap → second free of the same heap (ASan
   double-free). Fix: tree-C and tree-LLVM `emit_for_iter`
   now emit a shallow `free(xs.data)` (only the outer
   buffer) when the element type is non-Copy and the
   collection is owned. Copy-element collections still
   route through `intent_vec_<T>__free` (which is just
   `free(xs.data)` for Copy elements anyway). The SSA path
   never emitted a Drop for the consumed collection at
   all — silently leaking the outer buffer (no IR shape
   for "free outer buffer only"). Gated out via
   `stmt_ssa_supported`'s `ForIter` arm: programs with
   consuming for-iter over non-Copy Vec elements now fall
   back to tree-LLVM / tree-C. Verified clean under
   `-fsanitize=address,leak` for `Vec<OwnedStr>`,
   `Vec<Vec<i64>>`, and `Vec<Struct{OwnedStr,i64}>`.
   Closure #159.

   **SSA-LLVM vec set/push/clone arg type fix done 2026-05-23**:
   `emit_vec_call` in
   [src/ssa_backend_llvm.rs](src/ssa_backend_llvm.rs) was
   falling back to `element.clone()` whenever
   `operand_type(...)` returned `None` (which happens for
   every `Operand::Const`). That typed `set`'s i64 index
   slot as the element type — e.g. `set(Vec<OwnedStr>,
   0, v)` emitted `i8* 0` for the literal index, which
   the lli verifier warned about and the call site
   tolerated by accident. Fix: a per-builtin signature
   table (`sig_at(pos)`) returns the correct expected
   type per position: `push(Vec<T>, T)`, `set(Vec<T>,
   i64, T)`, `clone(Vec<T>)`. Const operands now type
   correctly. Closure #158.

   **`clone_at` Enum element done 2026-05-23**:
   tree-LLVM's `clone_at` panicked for Enum element
   types. Closures #154 / #155 added OwnedStr / Struct
   arms; #156 finishes Enum with an OR-chain over the
   payloaded tags, branching to a deep-clone block
   (`intent_str_concat` of the OwnedStr payload, then
   insertvalue into a new enum struct) vs a tag-only
   block (use the loaded slot as-is), phi-joined into
   `dest`. Tree-C was already correct via
   `c_element_deep_clone`'s Enum arm from #152.

   **`clone_at` Struct element done 2026-05-23**:
   tree-LLVM's `clone_at` panicked with "Struct(…) not
   yet supported" when the Vec element was a struct with
   heap fields. Closure #154 only added the OwnedStr arm;
   #155 finishes Struct: load the slot, extract each
   field, deep-clone OwnedStr fields via
   `intent_str_concat` with the empty literal, assemble
   via an insertvalue chain. Tree-C was already correct
   (closure #153 made `c_element_deep_clone` recurse
   through Struct fields). Closure #155.

   **`clone_at(ref xs, i)` for OwnedStr / Struct slots done 2026-05-23**:
   `clone_at(ref Vec<OwnedStr>, i)` was broken in two
   places: (1) SSA-C had no `clone_at` handler — fell
   through to the `fn_clone_at(...)` user-fn shape and
   failed at link time with an undefined-reference; (2)
   tree-LLVM's `clone_at` only handled Copy and `Vec<U>`
   element types — OwnedStr / Struct panicked with "not
   yet supported in tree-LLVM". Both backends now produce
   per-element deep clones: SSA-C routes through the
   existing `c_element_deep_clone` helper; tree-LLVM
   loads the i8* slot and calls `intent_str_concat` with
   the `@.empty_str_clone` empty literal. Closure #154.

   **`clone(Vec<Struct{heap-field}>)` deep-copies done 2026-05-23**:
   `clone(Vec<Tag>)` where `Tag` carries an OwnedStr field
   was shallow-copying the struct, sharing the field's
   heap pointer between source and clone — both __free
   sites then freed the same heap (ASan double-free; lli
   abort). C `c_element_deep_clone` adds a `Type::Struct`
   arm that reconstructs the struct with each owning
   field deep-cloned (recursive call) and Copy fields
   copied as-is. LLVM's per-shape Vec __clone gets a
   parallel Struct arm: extract each field, deep-clone
   (OwnedStr via the `@.empty_str_clone`-fed
   `intent_str_concat`), assemble the new struct via an
   insertvalue chain. Closure #153.

   **`clone(Vec<OwnedStr>)` / `clone(Vec<Enum>)` deep-copies done 2026-05-23**:
   the per-shape Vec `__clone` helper was shallow-copying
   the per-element heap pointers — `clone(Vec<OwnedStr>)`
   produced a new Vec whose i8* slots aliased the source's,
   so the source's free + the clone's free double-freed the
   shared heap. C `c_element_deep_clone` now deep-clones
   `OwnedStr` via `intent_str_concat(slot, 0, "", 0)` (round-
   trip through the concat helper with an empty literal,
   gives a strdup-like copy) and `Enum` via a tag-switched
   ternary that reconstructs the enum with a deep-cloned
   OwnedStr payload for payloaded variants. LLVM's per-
   shape `__clone` helper extended to loop over slots for
   ANY non-Copy element type (was only handling `Vec<U>`
   elements; `OwnedStr` / `Enum` payloads fell through to
   an uninitialized buffer, crashing lli with "free():
   invalid pointer"). LLVM emit also adds an
   `@.empty_str_clone` private constant. Closure #152.

   **`Vec<PayloadedEnum>` compiles + drops correctly done 2026-05-23**:
   `Vec<Msg>` where `Msg` is a payloaded enum was broken
   in four places: (1) C `element_tag` and (2)
   `c_element_storage` fell through to `c_leaf_type` →
   "int32_t" for enums, so the per-shape typedef tried to
   store `Enum_Msg` struct literals into i32 slots (cc
   rejected); (3) `c_element_drop_old` lacked an Enum arm,
   so the per-element drop body was empty (payloads leaked
   at `intent_vec_Enum_Msg__free` time); (4) LLVM vec
   literal used `vec_element_byte_size` for enums
   (returning 8 = i64), under-allocating the 16-byte
   tagged union and crashing lli with "free(): invalid
   pointer". All four sites now treat payloaded enums
   like structs/tuples: `Enum_<Name>` tagged-union
   typedef, GEP-null sizeof, tag-switched per-element
   payload free. Closure #151.

   **IndexAssign whole-element for OwnedStr/Vec elements done 2026-05-23**:
   `Vec<OwnedStr>[i] = "x" + ""` and `Vec<Vec<i64>>[i] =
   vec(…)` were leaking the OLD element's heap. Closure
   #149's IndexAssign whole-element extension only added
   the Struct/Enum element-type arms; OwnedStr and Vec
   element types fell through to a plain store. Tree-C
   `emit_index_assign` now also frees the OLD slot for
   `Type::OwnedStr` (`free((void*)<lv>)`) and `Type::Vec`
   (`intent_vec_<T>__free(<lv>)`) leaf cases. SSA-C's
   `InstrKind::IndexAssign` emitter extended in parallel
   (the `Vec<OwnedStr>` case routes through SSA, not
   tree-C). Closure #150.

   **IndexAssign of Struct/Enum element frees old heap done 2026-05-23**:
   `xs[i] = newStruct` for a `Vec<Struct{heap-field}>`
   element was leaking the OLD element's heap fields.
   The IndexAssign leaf-drop logic (closure #126) only
   fired when `field_path != []` (i.e. `xs[i].field =
   …`); whole-element overwrites (field_path empty +
   leaf == Struct/Enum) fell through to a plain store,
   losing the old heap. Tree-C's `emit_index_assign`
   now also handles the `field_path == []` case for
   leaf-Struct (walk per-field drops over the OLD
   element) and leaf-Enum (switch on the OLD tag to
   free the payload). Closure #149.

   **FieldAssign of Struct-typed field frees old heap done 2026-05-23**:
   `o.inner = newInner` where Inner has heap-shaped fields
   (OwnedStr / Vec) was leaking the previous Inner's
   heap. FieldAssign's heap-overwrite logic (from closure
   #132) only handled OwnedStr / Vec field types; Struct
   fell through to a plain assign. Tree-C now also walks
   the OLD struct field's per-field drops via
   `emit_struct_field_drops` before storing the new
   value. Enum-typed struct fields are still gated by
   the checker (not yet supported), so no fix needed
   there. Closure #148. Verified leak-free under
   `-fsanitize=address,leak`.

   **Reassign of Struct / Enum with heap fields done 2026-05-23**:
   `t = Tag { name: …}` and `m = Msg.Text(…)` for bindings
   with heap-shaped fields / payloads were leaking the
   previous heap. Tree-C, tree-LLVM, and SSA Reassign
   handlers only had Vec / OwnedStr drop-old cases — Struct
   / Enum fell through to plain assign. Tree-C now eval-
   into-tmp, walk the old binding's per-field drops (Struct)
   or switch-on-tag payload free (Enum), then move the tmp
   in. SSA's drop_old whitelist extended to admit non-Copy
   Struct / Enum; backends' `Drop` emit handlers already
   knew how to walk those. Closure #147. Verified leak-free
   under `-fsanitize=address,leak`.

   **`let _ = make_enum()` frees heap payload done 2026-05-23**:
   `let _ = make_enum();` for an enum with a heap-shaped
   payload (OwnedStr / Vec<T>) was leaking. Same shape
   as closure #145's struct discard fix — Tree-C, tree-
   LLVM, and SSA Discard handlers only matched
   OwnedStr / Vec / Struct; `Type::Enum(_)` fell
   through. Tree-C spills to `Enum_<Name> _intent_discard`,
   switches on the tag, and frees the payload for the
   payloaded variants. Tree-LLVM mirrors the scope-exit
   Drop logic for enums (extractvalue tag / extractvalue
   payload, OR-chain of `icmp eq` tags, conditional
   branch to the free block, `@free` or
   `@intent_vec_<tag>__free`). SSA Discard emits
   `InstrKind::Drop` for non-Copy enums. Closure #146.

   **`let _ = make_struct()` frees heap fields done 2026-05-23**:
   `let _ = make_struct();` for a struct with heap-shaped
   fields (OwnedStr, Vec<T>, nested struct) was silently
   leaking the per-field heap. Tree-C, tree-LLVM, and SSA
   Discard handlers all only matched `OwnedStr | Vec(_)`
   — `Type::Struct(_)` fell through to a `(void) expr`
   (tree-C) or bare `emit_expr` (LLVM / SSA), never
   freeing the struct's owning fields. Tree-C now spills
   to a brace-scoped `_intent_discard` local and walks
   the fields via `emit_struct_field_drops`; tree-LLVM
   spills to an alloca and walks via
   `emit_llvm_struct_field_drops` (the existing per-field
   helper used by scope-exit Drop). The tree-LLVM arm
   also had to be moved BEFORE the `is_scalar` check —
   `is_scalar(Type::Struct(_))` returns true since the
   alloca path treats structs like scalars; without the
   reorder the discard would skip the Struct arm. SSA
   Discard emits an `InstrKind::Drop` for non-Copy
   structs. Closure #145. Verified leak-free under
   `-fsanitize=address,leak` against a 100-iter loop
   (was leaking ~300 bytes pre-fix).

   **`intent_str_concat` l_owned flag fix done 2026-05-23**:
   `t.name + "-suffix"` where `t.name: OwnedStr` was
   DOUBLE-FREEING. The concat helper's `l_owned`/`r_owned`
   flag was set unconditionally for any OwnedStr-typed
   operand, so concat freed the struct field's heap. Then
   the struct's per-field scope-exit Drop also freed it
   (the partial-move tracking only kicks in for whole-
   binding `Stmt::Assign` moves, not for FieldAccess in
   binary-op operands). New
   `crate::ir::owned_str_consumed_at_concat(expr)` helper
   uses a refined rule: `l_owned=1` only when the operand
   is a Var (checker marks Var as moved by the binary op,
   so the binding's Drop is suppressed and concat MUST
   free) OR fresh (Call / Binary / Block / IfExpr /
   Match — no other owner). FieldAccess / TupleAccess /
   Ref keep `l_owned=0` so the binding's Drop owns the
   free. Closure #144. Verified leak-free + double-free-
   free under `-fsanitize=address,leak` against both the
   Var-Var concat (`g + "!"`) and the FieldAccess-Str
   concat (`t.name + "-suffix"`) shapes. See updated
   [examples/struct_owned_field.intent](examples/struct_owned_field.intent).

   **`clone(fresh_vec)` drops borrowed arg done 2026-05-22**:
   `clone(vec(1, 2, 3))` was silently leaking the fresh
   Vec passed in. The checker treats `clone(xs)` as
   borrow-semantics — xs continues to be readable after
   the call (useful for "deep copy without consuming the
   source") — but for a fresh-Vec argument there's no
   other binding to own the heap. SSA Call lowering now
   emits a `Drop` after the `clone` call for each fresh
   non-Copy argument. Var / FieldAccess args skip
   (binding owns). Other built-ins (`push`, `set`,
   `vec()`) either consume their args at the checker
   level or are construction-only. Closure #143.

   **`Index` of fresh Vec drops buffer done 2026-05-22**:
   `vec(1, 2, 3)[0]` (and other fresh-Vec index shapes)
   was silently leaking the Vec buffer — the `Index`
   instruction reads one element but doesn't free
   `.data`. Mirrors closure #141 for `Len`. SSA Index
   lowering emits a `Drop` after the `InstrKind::Index`
   when the operand is a fresh Vec. Tree-C `emit_index`
   for Vec wraps the index read in a brace-scoped tmp +
   `intent_vec_<T>__free` for fresh operands. Var /
   FieldAccess Vec operands keep the simple form.
   Closure #142.

   **`len` of fresh Vec drops buffer done 2026-05-22**:
   `len(vec(1, 2, 3))` was leaking the Vec buffer — the
   `Len` instruction reads `.len` from the struct but
   doesn't touch `.data`. Generalized
   `is_fresh_owned_str` to `is_fresh_non_copy` (matches
   both OwnedStr AND `Vec<T>` over the same
   Call/Binary/Block/IfExpr/Match kind whitelist). SSA
   `Len` lowering for non-Str arrays now emits a `Drop`
   after the Len instruction when the operand is fresh.
   Tree-C `emit_len` for Vec wraps the `.len` read in a
   brace-scoped tmp + `intent_vec_<T>__free` for fresh
   operands. Verified leak-free under
   `-fsanitize=address,leak` against a 1000-iteration
   loop (1000 × 5-element vecs previously left ~40KB
   of leaked buffers). Closure #141. See updated
   [examples/vectors.intent](examples/vectors.intent).

   **Unified fresh-OwnedStr helper + tree-C strcmp/strlen fixes done 2026-05-22**:
   The per-site `matches!(e.ty, OwnedStr) && matches!(e.kind,
   Call | Binary)` whitelist used by closures #135 (print),
   #137 (match scrutinee), #138 (strcmp), and #139 (strlen)
   is now a single `crate::ir::is_fresh_owned_str(expr)`
   helper. The unified helper also broadens the set to
   include `Block` / `IfExpr` / `Match` expressions
   returning OwnedStr — these escape an inner heap to the
   outer context (the inner Let bindings are never Drop'd
   by the Block emitters in v1, so the value's only owner
   is the outer use site). The leak surfaced as
   `len({ let s = make(); s })` slipping past closure
   #139's narrower whitelist. The tree-C emit_len and
   emit_binary-strcmp paths (previously untouched by #138
   / #139) now also free fresh operands via GCC statement-
   expression-wrapped temps. Closure #140. Verified
   leak-free under `-fsanitize=address,leak`.

   **`len` of fresh OwnedStr drops heap done 2026-05-22**:
   `len(make_owned_str())` was silently leaking —
   `intent_str_len` (strlen) doesn't consume its argument,
   so a fresh-OwnedStr operand (Call / Binary `+`) had no
   other binding to own the heap after the `len` call.
   Fixed in both the SSA lowering of `TypedExprKind::Len`
   for `Str/OwnedStr` (emits a `Drop` instruction after
   the `intent_str_len` call) and the tree-LLVM
   `TypedExprKind::Len` arm (emits `call void @free(i8* %v)`
   after the strlen). Var / FieldAccess operands skip
   the drop — same conservative whitelist as closures
   #135 / #137 / #138. Closure #139. Verified leak-free
   under `-fsanitize=address,leak`. See updated
   [examples/strings_concat.intent](examples/strings_concat.intent).

   **`strcmp` of fresh OwnedStr drops heap done 2026-05-22**:
   `make_owned_str() == "literal"` (and `!=`, `<`, `<=`,
   `>`, `>=`) was silently leaking — `intent_str_cmp` /
   `strcmp` doesn't consume its arguments, so a fresh
   OwnedStr operand (Call / Binary `+`) had no other owner
   after the comparison. Fixed in both the SSA lowering of
   string comparison (emits a `Drop` instruction after the
   strcmp call for each fresh operand) and the tree-LLVM
   Binary-strcmp branch (emits `call void @free(i8* %v)`
   after the compare). Var / FieldAccess operands skip
   the drop — same whitelist as closure #135's print
   handling and closure #137's match scrutinee, so the
   outer binding's scope-exit Drop owns the heap and
   nothing double-frees. Closure #138. Verified leak-free
   under `-fsanitize=address,leak`. See updated
   [examples/strings_concat.intent](examples/strings_concat.intent).

   **`match make_owned_str() { … }` drops temp scrutinee done 2026-05-22**:
   `match` on a fresh OwnedStr scrutinee (Call or `+`
   concat producing a heap string with no other owner)
   was silently leaking. `check_match_str` bound the
   scrutinee to a temp inside a synthetic Block but never
   emitted a Drop for the temp, so the heap escaped at
   Block exit. Restructured the synthetic Block to wrap
   the if-chain through a `__match_str_result_<n>` let,
   emit a `TypedStmt::Drop` for the temp after the
   if-chain runs, then yield the result var as the
   block's tail. Tree-C / tree-LLVM Block codegen also
   extended to emit Drop stmts inside the GCC
   statement-expression body. The fix uses the same
   conservative whitelist as closure #135 (Call / Binary
   only) so Var / FieldAccess scrutinees — which alias an
   outer-binding's heap — don't get spuriously dropped
   (would double-free at the outer scope's existing
   Drop). Closure #137. Verified leak-free under
   `-fsanitize=address,leak`. See updated
   [examples/match_str.intent](examples/match_str.intent).

   **`Vec<OwnedStr>` compiles to valid C done 2026-05-22**:
   the C backend's `element_tag` helper was leaking the
   `*` from `c_leaf_type(OwnedStr) = "char*"` into the
   per-shape Vec typedef name — `Vec<OwnedStr>` emitted
   `typedef … intent_vec_char*;` and the cc step failed
   with "expected ';'…before '*'". Added explicit arms
   for Type::Str (`str`) and Type::OwnedStr (`owned_str`)
   so the typedef becomes `intent_vec_owned_str`. LLVM
   was already sanitizing `*`→`p` via its own
   `vec_struct_tag`. No example exercised `Vec<OwnedStr>`
   before — `examples/strings_concat.intent` now does.
   Closure #136.

   **`print` of fresh OwnedStr expression frees heap done 2026-05-22**:
   `print make_owned_str();` was silently leaking the
   returned heap string. All three print emit paths —
   SSA (which routes both SSA-C and SSA-LLVM via
   `intent_print_item`), tree-C `emit_print_expr_no_newline`,
   and tree-LLVM `emit_print_expr_no_newline` — handled
   OwnedStr as a borrowed read (the right thing for
   `print s;` where `s: OwnedStr`) but never freed the
   heap when the printed value came from a fresh
   expression with no other owner. The fix uses a
   conservative whitelist: free after print only when the
   item's TypedExprKind is `Call { … }` or `Binary { … }`
   (the v1 OwnedStr heap-producers). Var / FieldAccess /
   TupleAccess and any other variant skip the free so the
   binding's scope-exit Drop still has the only handle —
   freeing eagerly there would double-free (e.g. the
   `print t.name` pattern in
   `examples/struct_owned_field.intent`). Closure #135.
   Verified leak-free under `-fsanitize=address,leak` on
   both the fresh-call and binding-owned shapes. See
   updated [examples/strings_concat.intent](examples/strings_concat.intent).

   **`let _ = …` discard of OwnedStr frees heap done 2026-05-22**:
   `let _ = make_owned_str();` (and the bare-call form
   `make();` that the parser sugars to it) was silently
   leaking the returned heap string. All three Discard
   emit paths — tree-C, tree-LLVM, and the SSA lowerer
   that fed both SSA-C and SSA-LLVM — handled `Vec<T>` but
   skipped `OwnedStr`, falling through to a `(void)`-style
   no-op. SSA Reassign lowering was also extended to lower
   `drop_old` reassigns for OwnedStr / Vec (was a hard
   reject — fell back to tree backends) so closure #133's
   `current = "step-" + ""` shape lowers through SSA too.
   Closure #134. Verified leak-free under
   `-fsanitize=address,leak`. See updated
   [examples/strings_concat.intent](examples/strings_concat.intent).

   **Reassign of OwnedStr frees previous heap done 2026-05-22**:
   `s = "b" + ""` for a non-Copy `OwnedStr` binding now
   frees the previous heap string before storing the new
   value. Was a real leak: the Reassign emit's drop-old
   path only handled `Type::Vec`; OwnedStr fell through to
   the plain-assign branch and the previous allocation was
   lost. C emits the same tmp-eval / free-old / move-tmp
   shape Vec uses; LLVM emits eval-first then free-old then
   store (the previous order — free-before-eval — was also
   incorrect for any non-consuming RHS that READS the
   binding, e.g. `s = s + ""`-ish patterns; the LLVM Vec
   path had the same latent issue and is also fixed here).
   Closure #133. Verified leak-free under
   `-fsanitize=address,leak`. See updated
   [examples/strings_concat.intent](examples/strings_concat.intent).

   **FieldAssign with heap-shaped field frees old slot done 2026-05-22**:
   `t.name = newstr` for an OwnedStr field (and
   `b.items = newvec` for a Vec field) now frees the
   previous slot's heap before storing the new value, both
   for plain owned-struct assigns and through-`mut ref`
   borrows. Mirrors the leaf-Drop logic from closure #126
   for mixed-place index-assigns (`xs[i].field = …`). Was
   a real leak: a struct with an OwnedStr field that gets
   overwritten leaked the old string until scope exit, and
   the scope-exit drop only freed the latest pointer. The
   bug was masked because no example exercised the
   field-overwrite pattern. Closure #132. C backend now
   emits `free((void*)<lvalue>)` or
   `intent_vec_<T>__free(<lvalue>)` before the assign;
   LLVM loads the old pointer/struct via the same GEP and
   calls `@free` or the matching vec `__free`. Verified
   leak-free under `-fsanitize=address,leak`. See updated
   [examples/struct_owned_field.intent](examples/struct_owned_field.intent).

   **Cross-backend parity runner covers all examples done 2026-05-22**:
   the `llvm_backend_run_produces_same_output_as_c` runner
   in [tests/run_end_to_end.rs](tests/run_end_to_end.rs)
   was missing 14 of the 57 examples — including
   `try_keyword.intent`, `block_expressions.intent`,
   `option_types.intent`, `option_error_propagation.intent`,
   `interfaces.intent`, `generic_functions.intent`,
   `composite_types.intent`, `fn_pointers.intent`,
   `methods.intent`, `assert_messages.intent`,
   `tracker.intent`, and the three Devanagari keyword
   examples (`hindi_keywords.intent`,
   `marathi_keywords.intent`, `sanskrit_keywords.intent`).
   Closure #130 surfaced this gap by hitting a pre-existing
   C codegen bug that the parity runner would have caught
   at landing time had `try_keyword.intent` been listed.
   All 57 examples now run identically on both backends.
   Closure #131.

   **`try` desugar admits intermediate `print` done 2026-05-22**:
   the `let v = try opt; …; return X;` desugar's
   intermediate-stmt check was relaxed from Let-only to
   Let + Print, riding on closure #129's extension of
   block expressions. Useful for tracing the happy path
   while the desugar still short-circuits the None case.
   This closure also fixed a pre-existing C-backend codegen
   bug surfaced by trying to emit the OFFICIAL example
   end-to-end: match expressions with a payloaded-enum
   result type were using `c_element_storage` (returns the
   bare `int32_t` tag for any enum) instead of `c_type_name`
   (returns `Enum_<Name>` for payloaded enums); the bug
   never showed up in unit tests because they stopped at
   `compile()` and didn't `emit`+`cc`. Closure #130. See
   updated [examples/try_keyword.intent](examples/try_keyword.intent).

   **Block expressions admit print stmts done 2026-05-22**:
   `let r = { let a = …; print "log", a; tail }` now
   compiles. The v1 Block MVP was Let-only; the relaxation
   keeps the same shape (Let prefix + tail expression) but
   also lets `print` stmts interleave for logging
   intermediate values. Control flow, reassignment, and
   other shapes still surface the existing diagnostic.
   Parser accepts a Let/Print prefix; checker pushes
   `TypedStmt::Print` into the block's stmts; tree-C and
   tree-LLVM Block emitters emit print stmts inline (via
   `emit_print_items` / the standard stmt emitter). SSA
   Block routing is unchanged (still falls back to tree
   backends). Closure #129. See updated
   [examples/block_expressions.intent](examples/block_expressions.intent).

   **OwnedStr enum payload destructure done 2026-05-22**:
   `match m { Msg.Text(s) then … }` now compiles when the
   variant payload is `OwnedStr`. The binding `s` is
   exposed to the arm body as a `Str` (Copy borrowed view),
   so the scrutinee retains ownership and its existing
   scope-exit Drop frees the heap exactly once. Other
   non-Copy payload types (`Vec<T>`, structs with owning
   fields, …) still need their own borrow-view wiring and
   stay rejected. Closure #128 / D3. Verified leak-free
   under `-fsanitize=address,leak`. See updated
   [examples/enum_owned_payload.intent](examples/enum_owned_payload.intent).

   **Vec element drops walk owning fields done 2026-05-22**:
   `intent_vec_<S>__free` now iterates every live element
   and drops its owning resources before freeing the buffer,
   for `S = OwnedStr`, `S = Struct{…}` with owning fields,
   and `S = Vec<U>` (was already handled). Closes a
   pre-existing leak where `Vec<Struct{OwnedStr…}>` and
   `Vec<OwnedStr>` left their element heaps unfreed at
   scope exit. C: `c_element_drop_old` extended with
   `OwnedStr` and `Struct` arms (the latter via
   `emit_struct_field_drops`); LLVM: per-element loop body
   emits per-field GEP + load + `@free` /
   `@intent_vec_<tag>__free` driven by a slim local
   counter. Closure #127. Verified leak-free under
   `-fsanitize=address,leak`. See updated
   [examples/struct_owned_field.intent](examples/struct_owned_field.intent).

   **Match on `Str` / `OwnedStr` scrutinee done 2026-05-22**:
   string-literal patterns desugar at the checker level to a
   nested if-expression chain over `==` on Str (strcmp-based).
   The scrutinee binds to a temp once so any side-effecting
   expression evaluates exactly once. Wildcard required.
   No backend changes — uses existing `==`/IfExpr/Block
   primitives. See
   [examples/match_str.intent](examples/match_str.intent).

   **Match on `bool` scrutinee done 2026-05-22**:
   `match b { true then …, false then …, _ then … }` works.
   Exhaustiveness requires both arms OR a wildcard. Bool
   patterns lower as int_value=0/1 so the existing
   integer-switch shape handles dispatch uniformly across
   both backends. New `Pattern::Bool` + `Pattern::Str`
   variants on the AST; Str still surfaces a "not yet
   supported" diagnostic (strcmp-dispatch is the natural
   follow-up). See
   [examples/match_bool.intent](examples/match_bool.intent).

   **`xs[i].field = v` mixed-place assign done 2026-05-22**:
   single-level field path on the indexed element. Parser
   builds an `IndexAssign` with a non-empty `field_path`;
   the checker validates the path against the struct decl;
   both backends GEP/access into the slot and store at the
   field offset. Copy-leaf only in v1 (avoids field-Drop on
   overwrite). See
   [examples/mixed_place_assign.intent](examples/mixed_place_assign.intent).

   **In-place `push(mut ref xs, v)` done 2026-05-22**: a
   second form of `push` that operates through a Vec
   pointer instead of consuming + returning the Vec.
   Useful for growing a Vec owned by a struct field
   without partial-move + write-back. Same realloc
   logic as the consuming form; returns `i64` (new len).
   See [examples/push_mut.intent](examples/push_mut.intent).

   **Tuple auto-equality done 2026-05-22**: tuples are
   anonymous so they can't have a user `Eq` impl, but the
   checker synthesizes an AND-chain of per-element
   comparisons: `(a, b) == (c, d)` → `a == c && b == d`.
   Primitive elements use built-in `==`; nominal element
   types (struct/enum) dispatch through the element's
   `<T>_eq` impl. See
   [examples/tuple_eq.intent](examples/tuple_eq.intent).

   **Enum `==` desugar + partial-then-whole-move done
   2026-05-22**: `check_equality` matches `(Enum, Enum)` of
   the same nominal type in addition to struct-struct, so
   `implement Eq for Color { fn eq(self: Color, other: Color)
   -> bool }` makes `a == b` work on Color bindings. The
   enum-type resolver (`resolve_enum_types_in_program`) now
   walks `program.impls` so the impl body's `self: Color`
   resolves to `Type::Enum` (was `Type::Struct`, blocking
   `self as i32`). Separately, moving a struct as a whole
   after a partial-field-move now emits a clean diagnostic
   ("cannot move 'b' — its field 'f' was previously moved
   out"). See
   [examples/enum_eq.intent](examples/enum_eq.intent).

   **Partial-move tracking done 2026-05-21**: `let taken =
   bag.contents;` moves a single field out of a struct
   without invalidating the rest of the struct. A new
   `VarInfo.moved_fields` map tracks which fields have been
   moved out; `TypedStmt::Drop` gained a `moved_fields:
   Vec<String>` list that both backends consult to skip the
   per-field free for moved-out fields. Reading a moved
   field again surfaces a use-after-move diagnostic. See
   [examples/partial_move.intent](examples/partial_move.intent).

   **User-Eq desugar for struct `==` done 2026-05-21**:
   `a == b` and `a != b` on two bindings of the same struct
   type desugar to `<T>_eq(a, b)` / `!<T>_eq(a, b)` whenever
   an `implement Eq for T { fn eq(self: T, other: T) -> bool }`
   is in scope. The hoisted method is the same statically-
   dispatched path used by the `recv.eq(...)` MethodCall
   form. Tuple / enum auto-equality can use the same recipe
   when needed. See
   [examples/struct_eq.intent](examples/struct_eq.intent).

   **Reverse-declaration field drop order done 2026-05-21**:
   struct Drop walks fields in reverse declaration order so
   destruction mirrors construction (Rust's RAII convention).
   Pure code-shape change in both backends' Drop emit.

   **Field-borrow expressions done 2026-05-21**: `ref t.f`
   and `mut ref t.f` now work, single-level. Two new
   `TypedExprKind` variants (`RefField` / `RefMutField`)
   carry the (object, field, field_index) triple; tree-C
   emits `&v_t.f`; tree-LLVM GEPs into the struct. The
   immediate unlock is atomic operations through a struct
   field (`atomic_store(mut ref c.hits, 42)`). Vec push
   through field still needs partial-move tracking. See
   [examples/struct_atomic_field.intent](examples/struct_atomic_field.intent).

   **T2.7 phase 2 — user-Drop auto-call at scope exit done 2026-05-21**:
   `implement Drop for T { fn drop(self: T) -> i64 }` now runs
   automatically at every scope exit where a non-moved binding
   of T goes out of scope. The auto-call is suppressed when
   T also has heap-shaped fields (OwnedStr / Vec) — those
   route through the per-field free pass; users invoke
   `t.drop()` explicitly for richer behavior. Two key wiring
   pieces: (1) the affine-aggregate registry now picks up
   any struct with a hoisted `<T>_drop` function (so the
   scope-exit pass doesn't short-circuit on `is_copy()=true`
   for Copy-only structs), and (2) `self` inside `<T>_drop`
   bodies gets `VarInfo.no_drop = true` to break the
   otherwise-infinite recursion (both the scope-exit pass and
   the Return-path cleanup consult this flag). See
   [examples/drop_interface.intent](examples/drop_interface.intent).

   **T1.2 phase 2b — affine struct fields expanded 2026-05-21**:
   Structs now accept `OwnedStr`, `Vec<T>`, `[T;N]` of Copy
   elements, `Task`, and `Atomic<T>` as fields. Both backends
   emit per-field `free` (heap fields) or no-op (stack-shaped
   fields) on struct Drop. Tree-LLVM gained a FieldAccess-as-
   Index-base arm so `t.data[i]` works. Tree-C reorders to
   emit Vec typedefs before struct typedefs for `struct {
   xs: Vec<T> }` to resolve at its declaration. Struct field
   `[T;N]` uses an inline `T name[N]` declarator and a bare-
   brace `{…}` initializer for the StructLit field path
   (C forbids compound-literal-array assignment into struct
   members). See
   [examples/struct_mixed_fields.intent](examples/struct_mixed_fields.intent).

   **T1.5 phase 2 — bounded generics done 2026-05-21**:
   `fn min<T>(a: T, b: T) -> T where T is Cmp` now monomorphizes
   when the call-site concrete type has a matching
   `implement Cmp for <T>` decl. The previous WIP gate in
   `monomorphize_generics_in_program` has been replaced by an
   impl-existence check that walks `program.impls` for each
   (template, concrete) pair and surfaces a clean diagnostic
   if no satisfying impl is in scope. Scope-aware first-arg
   inference (annotated `let` + fn params) means calls like
   `let m: Score = min(a, b);` resolve correctly. Vtables /
   dynamic dispatch still pending. See
   [examples/bounded_generics.intent](examples/bounded_generics.intent).

   **Phase 2b (OwnedStr fields) done 2026-05-21**: structs
   may now carry an `OwnedStr` field. The aggregate is
   automatically affine; both backends emit a `free` of the
   field at scope exit, and the checker treats struct-literal
   initialization from a `Var` as a move on the source
   binding so a heap string can flow `caller → struct field →
   drop` without a double-free. Implementation: new
   thread-local `STRUCT_NON_COPY_REGISTRY` in
   [src/ast.rs](src/ast.rs) consulted by `Type::is_copy()`;
   per-backend `STRUCT_FIELDS_REGISTRY` / `LLVM_STRUCT_FIELDS_REGISTRY`
   populated at emit start so the `TypedStmt::Drop` handler
   can free each owning field by name (C: `free((void*)v_t.<field>)`)
   or index (LLVM: GEP + load i8* + `@free`). The LLVM
   string-interning pre-pass (`collect_strings_in_expr` in
   [src/backend_llvm.rs](src/backend_llvm.rs)) was the
   blocker — it didn't recurse into `StructLit` / `Tuple` /
   `Match` / `IfExpr` / `Block` / etc., so string literals
   nested inside struct-field initializers fell back to
   `i8* null` and segfaulted at `strlen`. Now recurses into
   every sub-expression form. A new lib test
   `struct_owned_str_field_compiles_and_drops` and example
   `examples/struct_owned_field.intent` exercise the path
   end-to-end. **Phase 2b still pending**: other affine
   field types (`Vec<T>`, `[T;N]`, `Task`, `Atomic<T>`),
   auto-drop chains in reverse-declaration order across
   multiple owning fields, `methods on Type<T>` generic
   methods (depends on T1.4 phase 2), user-defined `Drop`
   auto-call at scope exit (T2.7 phase 2, depends on this
   work).
3. **Enums + `match`** — *phase 1 done 2026-05-20*. Payload-less
   `enum Color { Red, Green, Blue }` declarations, variant references
   `Color.Red`, and `match scrutinee { Variant then expr, … }`
   expressions with **exhaustiveness checking** (every variant must
   have an arm; missing variants are a compile-time error). New
   `enum`, `match`, `then` lexer keywords. The parser produces
   `Type::Struct(name)` for any uppercase-identifier nominal type;
   a new `resolve_enum_types_in_program` pass rewrites
   `Type::Struct(name)` → `Type::Enum(name)` for declared enums
   before signature collection so all downstream analysis sees the
   right Type variant. Enums lower to a 32-bit tag in both backends
   (no per-name typedef needed — they fit in `i32`/`int32_t`).
   Tree-C `match` uses a GCC statement-expression with `switch`;
   tree-LLVM uses an LLVM `switch` with per-arm basic blocks
   merged via `phi`. Three new lib tests pin: working enum + match
   round-trip, non-exhaustive rejection, unknown-variant rejection.
   **Phase 2a done 2026-05-20**: parser accepts payload
   syntax — `enum Maybe { Some(T), None }`, `enum Outcome
   { Ok(i64, i64), Err }`. `EnumVariant.payload: Vec<Type>`
   carries the declared payload types; empty `Vec` means
   payload-less (back-compat with phase 1). The checker has a
   phase-2b gate: any program that declares a payloaded
   variant surfaces a clear "T1.3 phase 2b: tagged-union
   codegen + pattern binding are still in progress" diagnostic
   so users learn the syntax parses but isn't yet executable.
   **Phase 2b/3 done 2026-05-21 (tree-C)**: single-Copy-payload
   enums now compile end-to-end via `--backend=c`. The
   compiler lays them out as `typedef struct { int32_t tag;
   T payload; } Enum_<Name>;` where T is the shared payload
   type. Constructors `Opt.Some(42)` build `(Enum_Opt){.tag
   = 0, .payload = 42}`; match dispatches on `__scr.tag` and
   destructure arms `Opt.Some(v) then …` extract `__scr
   .payload` into a local `v` in the arm body's scope. The
   `TypedEnumDecl` carries `payload_types: Vec<Option<Type>>`
   and `TypedMatchArm` carries `binding: Option<(String,
   Type)>`. LLVM driver currently rejects payloaded enums
   with a "use --backend=c" diagnostic; tree-LLVM
   tagged-union codegen is queued as a follow-up.
   **Still pending**: multi-field payloads (`Pair(i64,
   i64)`), non-Copy payloads (Vec/OwnedStr), mixed payload
   types across variants (would need a union representation
   in the C struct rather than a single field), nested
   destructure (`Outcome.Ok((a, b))`), guards (`Color.Red
   if cond then …`), and LLVM tagged-union codegen.
   **Wildcard `_` pattern done 2026-05-20**: `_ then …`
   arms are accepted by the parser (lexed as the
   identifier `_`), satisfy exhaustiveness without
   listing every variant, and lower cleanly. Tree-C emits
   a `default: __r = (body); break;` case inside the GCC
   stmt-expr switch; tree-LLVM uses the wildcard's basic
   block as the switch's default label so no
   `unreachable + abort` block is emitted. An arm after
   the wildcard is dead and surfaces an "unreachable arm"
   diagnostic. AST refactored:
   `MatchArm.enum_name + variant` fields replaced by
   `MatchArm.pattern: Pattern { Variant { enum_name,
   variant }, Wildcard }`; `TypedMatchArm` gained
   `is_wildcard: bool`. Three new lib tests pin the
   surface (`match_wildcard_covers_remaining_variants`,
   `match_wildcard_alone_is_exhaustive`,
   `match_wildcard_followed_by_arm_rejected`).
   **Integer-literal patterns done 2026-05-20**: match
   scrutinees can now be any integer type (i8/i16/…/u64)
   in addition to enums; arms accept literal-integer
   patterns (positive or negative) plus a required
   wildcard. `Pattern::Int(i128)` joins `Variant` and
   `Wildcard` in the Pattern enum; `TypedMatchArm` gained
   `int_value: Option<i128>` so backends emit
   `case <int_value>:` instead of the variant tag. The
   checker rejects duplicate integer values, out-of-type
   overflow, missing-wildcard non-exhaustiveness, and
   cross-kind patterns (variant-on-int + int-on-enum).
   Five new lib tests pin the surface.
4. **Simple generics** — *phase 1 started 2026-05-20.* **Done so
   far:** `Type::Param(String)` in AST; `Function.type_params` carries
   declared type parameters; parser accepts `fn name<T1, T2>(…)`
   syntax and resolves identifier-typed positions against the
   in-scope type-param set so a bare `T` parses as `Type::Param(T)`
   rather than `Type::Struct(T)`; `Type::Display` prints param names
   verbatim. The checker surfaces a clear "T1.4 phase 2:
   monomorphization is still in progress, specialize manually for
   now" diagnostic on any generic function declaration. One new lib
   test (`generic_function_syntax_parses_but_gated`) pins the gate
   shape. **Phase 2 pending**: call-site type-arg inference, body
   type-check with substituted T, monomorphization (one specialized
   `TypedFunction` per (fn × type-args) seen at call sites),
   name-mangling so backends emit distinct symbols per
   specialization, recursion-safe specialization queue.
5. **Interfaces + bounded generics** — *phase 1 done 2026-05-20.*
   `interface Cmp { fn cmp(self, other: ref Self) returns i64; }`
   plus `implement Cmp for Point { … }` and the bound form
   `fn min<T>(a: T, b: T) returns T where T is Cmp`. No interface
   inheritance, no default methods, no associated types in v1.
   **Done so far:** `InterfaceDecl`, `InterfaceMethod`, `ImplDecl`,
   `WhereClause` in AST; `Program.interfaces`, `Program.impls`,
   `Function.where_clauses` carry the surface declarations; lexer
   recognizes `interface`, `implement`, `where`, `is`; parser
   accepts top-level `interface Name { fn m(…) -> T; … }`,
   `implement Iface for Type { fn m(…) -> T { … } … }`, and
   `where T is C, U is D` clauses after the return type. The
   checker surfaces clear "T1.5 phase 2: dispatch / bounded-generic
   checking is still in progress, specialize manually" diagnostics on
   any interface decl, impl block, or `where` clause. Three new lib
   tests (`interface_decl_parses_but_gated`,
   `implement_for_parses_but_gated`, `where_bound_parses_but_gated`)
   pin the gate shape. **Phase 2 pending**: interface-method
   signature verification against impl methods, vtable layout +
   dispatch (static-monomorphized first, dynamic later if needed),
   `where T is C` constraint propagation through the
   monomorphization queue (depends on T1.4 phase 2), `Self` type
   inside interfaces, conflict detection on overlapping impls.

### Tier 2 — Error handling + safe absence + custom RAII (built on Tier 1)

6. **`Option<T>` + `Result<T, E>` + `try` keyword** —
   `enum Option<T> { Some(T), None }`,
   `enum Result<T, E> { Ok(T), Err(E) }` shipped as built-in enums.
   `try expr` unwraps `Ok` / `Some` or short-circuits the current
   function with `Err` / `None`. Every fallible call's failure edge
   stays explicit in the source — no exceptions, no `?` operator,
   no hidden control flow. Implementation depends on #3.
7. **User-defined `Drop` interface** —
   `interface Drop { fn drop(mut self); }` plus
   `implement Drop for FileHandle { fn drop(mut self) { close(self.fd); } }`.
   `Drop` is the single compiler-recognized "magic" interface: when
   a type implements it, the auto-drop pass at scope exit invokes
   the user's `drop` method (in addition to recursively dropping
   affine fields). Compile-time guarantees stay: each value's `drop`
   runs exactly once, and the value is unusable after the call (the
   existing affine-move bookkeeping carries through). Lets user types
   wrap file descriptors / sockets / raw FFI pointers with the same
   leak-free behavior `Vec<T>` / `Mutex<T>` already get. Depends on
   #2 + #5.

### Tier 3 — Collections (depend on generics + interfaces)

8. **`Map<K, V>`** — hashed key-value store. Requires `K: Hash + Eq`
   (two built-in interfaces shipped alongside). Keys are values (no
   reference keys).
9. **`Set<T>`** — sibling to `Map`; thin wrapper requiring `T: Hash + Eq`.
10. **Slice type `&[T]`** — first-class read-only view over
    `Vec<T>` / `[T; N]`. Unblocks generic helpers like
    `fn sum(xs: &[i64]) -> i64` and range subslicing (`&xs[lo..hi]`).

### Tier 4 — Ergonomics & expressiveness

11. **`format!("x = {}, y = {}", x, y)`** — compiler-recognized
    intrinsic (NOT a user-defined macro system). Lowers to a chain of
    string-concat builtins at parse time.
12. **Method-call syntax `xs.len()`, `p.dist()`** — parser sugar for
    `len(xs)` / `Point::dist(p)`. No new lookup machinery; resolves to
    the existing free-function table during type-check.
13. **Closures (Copy-capture only)** —
    `let inc = with x: i64 do x + 1;`. Captures by value only and
    only Copy types — same rule the existing `task` body uses.
    Required for collection methods (`xs.map(with x do x * 2)`).
14. **Block expressions** — `let r = { let t = compute(); t + 1 };`.
    Already half-supported by the parser; finish + lower properly so
    `match` arms / closures have a natural body shape.
15. **Type aliases + `const`** — `type Coord = (i64, i64);`,
    `const PI: f64 = 3.14159;`. Cheap aliasing for readability; v1
    rejects recursive aliases and non-Copy `const`s.
    *Type aliases done 2026-05-20.* `type Name = Target;`
    top-level declarations parse via new `type` keyword;
    AST `TypeAlias { name, target }` lives on
    `Program.type_aliases`. The checker resolves each
    alias's target (recursively unfolding alias chains
    `Outer → Middle → Inner → i64`) into a fully concrete
    type. A DFS-based cycle detector rejects recursive
    aliases (`type A = B; type B = A;`) with a clear
    "recursive type alias" diagnostic. After resolution,
    a substitution pass walks every Type position in the
    program (function signatures, struct fields, const
    types, let/return/etc. in bodies) and replaces
    `Type::Struct(alias_name)` with the alias's resolved
    target. This means backends never see the alias name
    — they get a clean concrete type tree. Aliases that
    point at enums resolve correctly because the alias
    pass runs *after* `resolve_enum_types_in_program`
    has rewritten `Struct(Color) → Enum(Color)` inside
    alias targets too. Seven new lib tests pin: primitive
    alias, tuple alias, enum alias, alias chain,
    recursive alias rejection, duplicate alias rejection,
    struct-collision rejection. One new format
    round-trip test. *`const` done 2026-05-20.* `const NAME: T = literal;`
    top-level declarations work end-to-end: lexer keyword
    `const`, AST `ConstDecl` on `Program.consts`, parser
    `parse_const_decl`, formatter emission via
    `format_const_decl`. The checker validates Copy-only
    scalar types (i64/i32/.../f64/bool), rejects non-literal
    initializers (arithmetic + calls land in a later phase),
    catches duplicate names + collisions with structs/enums/
    functions, and folds unary-minus over a literal so
    `const MIN: i64 = -100;` works. Const bindings get
    seeded into the function env's root scope with
    `is_const: true` and `constant: Some(TypedConst::…)`;
    Var-resolution substitutes the literal in-place so the
    C/LLVM backends never see an unbound `v_NAME` symbol.
    Function-scoped `let NAME` cleanly shadows the const
    (the local lives in a deeper scope; `is_const: false`).
    Eight new lib tests + one new format round-trip pin the
    feature. **Pending**: type aliases (`type Coord = (i64,
    i64);`), const initializers with simple arithmetic
    (`const TAU: f64 = 2.0 * 3.14;`), const string literals,
    const struct/tuple values.

### Tier 5 — Verifier precision (nice-to-have, not blocking)

16. **`forall` quantifiers in invariants** —
    `invariant forall i: 0 <= i < len(xs), xs[i] >= 0;`. Encodes
    array-wide properties without manual unrolling. SMT support is
    already in z3; the surface needs parser + checker plumbing.
17. **Opaque-call return refinement** — drop the "`prove foo(args) > 0`
    requires `ensures` on `foo`" caveat by tracking a richer return-
    fact set. Lower priority than user-visible language surface.

### Tier 6 — Concurrency widening (mostly parameterization, cheap after #4)

18. **Parametric `Mutex<T>` / `Guard<T>`** — drop the i64-payload
    restriction once generics land.
19. **Parametric `Channel<T>`** — currently integer + bool only.
20. **`RwLock<T>` / `Barrier` / `CondVar`** — broader sync primitives.

### Tier 7 — Backend / runtime polish

21. **Windows `OMP_NUM_THREADS` lookup** — current Win32 parallel-for
    hardcodes N=4 workers; plumb a runtime query through the existing
    `WinParArg` (the outlined fn already reads `nt` from it).
22. **`break value` / labeled `continue`** — loop-as-expression form.
    Memorable, no new types, finite parser/checker work.

### Tier 8 — Deferred (revisit after the language feature set is complete)

23. **`async` / `await`** — adds a second control-flow story on top of
    `task`/`join`. Multi-week; skip until v1 surface stabilizes.
24. **Cranelift backend** — fast JIT path independent of LLVM.
25. **Direct-asm targets (x86_64-linux first)** — teaching path +
    tiny-target option.

### Explicitly out of scope (do not propose without consensus)

- **Inheritance, method overriding, virtual dispatch.** Use composition
  + interfaces instead.
- **Exceptions, `try`/`catch`, panicking propagation.** Use `Result` + `?`.
- **C++-style templates / specialization.** Generics are
  parameter-only.
- **First-class lifetimes / borrow checker beyond function params.**
  Affine ownership is the safety mechanism.
- **Macro system.** `format!` is a single compiler intrinsic, not a
  user-extensible macro.
- **Operator overloading on user types** in v1 (might lift later).

## Design Q&A (frequently-asked rationale)

### Why vtables / dynamic dispatch? Don't we prefer composition?

Composition is the default — interfaces dispatch **statically**
today (monomorphized per call site, no runtime indirection).
Vtables (epic A) are for the one workflow static dispatch can't
serve: **heterogeneous collections that don't enumerate variants**
(e.g. `Vec<dyn Drawable>`). For the variants-known case, a tagged
enum + `match` is idiomatic vāṇī and ships today. Vtables are an
ergonomic addition for the open-set case, not a missing core
capability.

### Does `try` have `catch`?

No. vāṇī has no exceptions, no `catch`, no stack unwinding.
Errors are **values** carried in payloaded enums (Option-like,
Result-like). `try` is the Rust `?` operator: sugar for early-
return on the None / Err arm. To "catch" a possible-None value,
use `match`. Every control-flow path is statically visible; no
hidden unwind. `assert` triggers `abort()` (no recovery).

### Are stack / queue / set / map built-in or composed?

Composed from existing primitives — the language stays minimal:

| Structure | How to build it |
|-----------|-----------------|
| Stack | `Vec<T>` + `push`. A `pop` builtin is queued (small). |
| Queue (concurrent) | `Channel<T, N>` — already ships. |
| Queue (FIFO, single-thread) | `Vec<T>` front-shift, or use `Channel<T, N>` even single-threaded. `VecDeque` is unblocked-by-need. |
| Set / Map (small N) | `Vec<(K, V)>` with linear scan. |
| Set / Map (large N) | Reserved for a future stdlib module — needs hash/cmp interface design first. |
| Linked list / tree | Index-based via `Vec<T>` with `parent: i64` / `next: i64`. Pointer-based linked structures fight the affine ownership model. |

Principle: **add a new built-in only when no composition of
existing primitives gets within an order of magnitude of optimal**.

## Caveats / out-of-scope (intentionally not on the TODO list)

These are design decisions or working-as-intended trade-offs — not
gaps to close. They appear in Known Issues with the "*Working as
intended*" tag where applicable.

- **Cross-compilation.** `intentc` bakes the host's `target_os` /
  `target_arch` into the emitted artifact (e.g., `SYS_futex`
  number, threading dispatch, `-lsynchronization` link flag). A
  `--target=` flag is out of scope for v1 — single-host-target is
  the operating assumption.
- **`!cond` post-loop fact dropped when body can `break`.** Adding
  it would be unsound under a break; the verifier conservatively
  omits it. Working as intended.
- **References are second-class.** `&T` / `&mut T` only as function
  parameter types; not as returns, let-bindings, or aggregate
  elements. Rust-style first-class references are explicitly out
  of scope for v1.
- **`prove foo(args) > 0;` requires `ensures` on `foo`.** Calls to
  functions without ensures fall back to "unsupported" since the
  solver has no fact about the return value. Working as intended —
  declare `ensures` to enable inline-call proofs.
- **`INTENTC_NO_VERIFY=1` bypass.** Skips every SMT round-trip.
  Useful for fast dev iteration; do not set in CI (a violated
  `ensures` won't surface). Runtime safety guards stay in place.

---

## Known issues

These are caveats present in the current implementation. Each links to
the TODO that would resolve it (or notes that the trade-off is
intentional). Resolved entries are deleted, not struck through —
TODO.md keeps the history.

### Backend / codegen
- **Full concurrency surface now flows through SSA on both backends.** `Atomic`, `Mutex`/`Guard`, `Channel`, `parallel for`, and `task`/`join` all use the SSA path; only multi-block task bodies and other shape-recognizer mismatches fall back via `EmitError`. (No active TODO in this row — kept here to document the milestone.)
- **No cross-compilation.** intentc bakes the host's `target_os` / `target_arch` into the emitted artifact (e.g., `SYS_futex` number, threading dispatch, link flags). A `--target=` flag is out of scope for v1; both C and LLVM backends emit code that only links on the same OS intentc was built for. *Trade-off, not a bug — flag separately if needed.*
- **Parallel-for thread count is hardcoded N=4 on Windows.** The Win32 fan-out doesn't read `OMP_NUM_THREADS` (or query `GetSystemInfo`) — it always spawns 3 worker threads plus the calling thread. *Trade-off: keeps the LLVM IR portable across LLVM versions without depending on getenv linkage. A future revision can plug in a runtime lookup helper; the per-thread `WinParArg` struct already carries `nt` so the outlined fn would not change.*

### Verifier / SMT
- **Natural-exit `!cond` post-loop fact omitted when the body can `break`.** Would be unsound; the verifier conservatively drops the fact. *Working as intended.*
- **`prove foo(args) > 0;` only works if `foo` has `ensures`.** Calls to functions without ensures fall back to "unsupported" since the solver has no fact about the return value. *Working as intended — declare ensures.*
- **Bare `let inner = xs[i]` is rejected for `Vec<non-Copy>`.** Direct indexing would alias the owner's slot and double-free; the checker emits a clear hint pointing users at the new `clone_at(&xs, i)` builtin that returns an owned deep-clone of the slot. *Working as intended — clone_at is the explicit opt-in for non-Copy slot reads.*

### Language surface gaps
- **No mutable references to atomics-as-payloads.** Workaround: pre-extract scalars before spawning a task. *Tracked indirectly by future affine-rules work.*
- **References are second-class.** `&T` / `&mut T` only as function parameter types; not as returns, let-bindings, or aggregate elements. *Working as intended for v1 — Rust-style first-class references are explicitly out of scope.*
- **Early `return` from inside a consuming `for x in xs` body leaks the outer Vec buffer.** The post-loop shallow-free is emitted inline by the for-iter backend code and runs only on natural completion / break. `return` skips it. Workaround: use `break` to exit the loop, then return after. *Tracked as a structural-rewrite TODO: introduce a `TypedStmt::ForIterShallowFree` variant and emit it at every Return inside the consuming loop body.*

### Tooling
- **`INTENTC_NO_VERIFY=1` skips every SMT round-trip.** Useful for fast iteration; do not set in CI — a violated `ensures` won't surface. Runtime safety guards stay in place. *Working as intended.*

---

## Update protocol for this file

When you finish a unit of work, update STATUS.md in the same commit:

- **Feature added** → add a bullet to the matching subsection. Keep the wording terse; the README has the long form.
- **TODO closed** → delete it from the TODO list above; if it had a Known Issues entry, delete or rewrite that entry too.
- **TODO added** → insert at the priority position; cross-reference any related Known Issues entry.
- **Issue discovered** → add to Known Issues; if a fix is planned, also add a TODO and link them.
- **Issue resolved** → delete the entry; do not strike through (`~~`). TODO.md preserves the history if you need it.
- **Test totals shifted** → update the header line.
- **Date roll** → update `Last updated:` to today.
