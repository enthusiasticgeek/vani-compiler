# Advanced 3c -- Capstone: timed tic-tac-toe (`stdin_ready_within_ms`, non-blocking I/O)

> **Learning goal**: race a blocking action (reading a move from
> stdin) against a countdown, WITHOUT spawning a thread at all --
> using `stdin_ready_within_ms`, a genuinely non-blocking readiness
> poll. Understand why the more obvious `task`/`join`/`Atomic<T>`
> approach (see [Advanced 3](03_concurrency.md)) has a real,
> unavoidable limitation for this exact problem, and why `async`
> wouldn't have fixed it either.

This is a walking tour of
[`examples/language/english/tic_tac_toe_timed.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/english/tic_tac_toe_timed.vani),
a standalone variant of the tic-tac-toe game from
[Intermediate 17](../intermediate/17_tic_tac_toe_capstone.md): same
board, win-detection, and ANSI color, but every move now has a
15-second clock. Run out of time and you forfeit the game -- and the
game finds out and exits **immediately**, not after waiting for you
to eventually type something. It's a separate file, not a
modification of the original -- the board/color/win-detection code is
duplicated rather than shared, since this is a standalone example,
not a library.

## Build & run

```bash
vanic run examples/language/english/tic_tac_toe_timed.vani                          # LLVM backend, JIT via lli
vanic run examples/language/english/tic_tac_toe_timed.vani --backend=c              # C backend, gcc
vanic build examples/language/english/tic_tac_toe_timed.vani -o /tmp/ttt_timed && /tmp/ttt_timed
```

---

## Why not `task` + `Atomic<bool>`?

The obvious first design: spawn a `task` (a real OS thread) that does
the actual blocking `stdin_read_line()` + validation loop, while the
main thread polls a shared `Atomic<bool>` "done" flag on a short
cadence, counting elapsed time. Whichever finishes first -- the human
typing a valid move, or the clock -- decides the outcome. This file
used to work exactly that way, and it's a legitimate, useful pattern
covered in [Advanced 3](03_concurrency.md).

It has a real limitation for *this specific problem*, though, and
it's not a workaround-able edge case:

1. **There is no non-blocking or cancellable stdin read to give the
   worker thread.** `cancel <name>;` shipped 2026-08-14 (see
   [Advanced 3](03_concurrency.md#cancel)) and DOES make a thread
   stuck in a blocking `tcp_accept`/`tcp_recv` genuinely
   interruptible -- but `stdin_read_line`/`file_read_line` are
   deliberately NOT cancel-aware (buffered stdio's `EINTR`
   interaction is murkier than raw socket syscalls and needs its own
   design pass). So for THIS specific file's problem -- a blocking
   *stdin* read -- the limitation described below still holds today,
   even with `cancel` in the language. (There's no reason that
   thread *has* to be permanently stuck -- if the read it's doing
   could itself be told "give up after N ms," none of this would be
   a problem. That's exactly what this file's fix provides -- see
   below.)
2. **`Task<R>` is affine.** Every spawned task must be consumed
   exactly once, by `join` OR `detach` -- see
   [Advanced 3](03_concurrency.md#detach----fire-and-forget). At the
   time this file's design problem first came up, `detach` didn't
   exist yet, so a "fire and forget" worker genuinely wasn't
   possible at all -- the program couldn't exit without eventually
   joining the stuck thread. `detach` removes that specific
   constraint (a caller COULD `detach` the blocking-read worker and
   let `main` exit immediately, leaving the orphaned thread to be
   reclaimed at process exit) -- but (per point 1) `cancel` doesn't
   reach `stdin_read_line`, and a detached thread still holds stdin
   open and could still consume the human's next keystroke at the
   wrong moment (e.g. bleeding into a subsequent prompt). The real
   fix below is still the better one for THIS file: a genuinely
   non-blocking poll needs no thread, no `join`, no `detach`, and no
   `cancel` at all.

Put together (as things stood before `detach`/`cancel` existed):
once the timer wins the race, the worker thread is still sitting
inside a real, blocking `stdin_read_line()` call, it cannot be
killed, and the program cannot exit without eventually joining it --
so the timed-out player's opponent, who already knows they won, has
to sit and wait for that thread's pending read to actually return (a
stray keystroke, Ctrl-D, or the process being killed) before the
process can fully exit. Not a bug -- an honest consequence of "no
cancellable blocking stdin I/O" combined with (at the time) "no
fire-and-forget tasks" -- but a real, user-visible annoyance. **This
specific gap (stdin cancellation) is still open as of 2026-08-14**
even though blocking-socket cancellation now exists -- see
`docs/TODO_CURRENT.md`.

**Would `async`/`await` have fixed it?** No, and it's worth being
precise about why. v1's async surface still desugars synchronously
(see `docs/v1_limitations.md`) -- an `async fn` body runs straight
through on whatever thread calls it, the same as an ordinary
function. Wrapping the same blocking `stdin_read_line()` call inside
`async fn read_move() { ... }` doesn't make it non-blocking; it just
moves the identical block onto whatever runs the async body. The
actual fix isn't a different way to run a blocking call -- it's a
non-blocking call to run instead.

## The fix: `stdin_ready_within_ms`, a real non-blocking poll

```vani
fn read_move(board: ref Vec<i64>, player: i64, turn_timeout_ms: i64, poll_ms: i64) -> i64 {
  let elapsed_ms: i64 = 0;
  let prompted: bool = false;
  while true {
    if elapsed_ms >= turn_timeout_ms {
      return 0 - 1;
    }
    if !prompted {
      let prompt: OwnedStr = "Player " + player_label(player) + ", enter a position (1-9), or 'quit' to exit:";
      print prompt;
      flush_stdout();
      prompted = true;
    }
    let budget: i64 = turn_timeout_ms - elapsed_ms;
    let slice: i64 = poll_ms;
    if slice > budget {
      slice = budget;
    }
    let ready: bool = stdin_ready_within_ms(slice);
    elapsed_ms = elapsed_ms + slice;
    if !ready {
      continue;
    }
    let input: OwnedStr = stdin_read_line();
    // ... validate exactly as the non-timed game's read_move does ...
  }
  return 0 - 1;
}
```

`stdin_ready_within_ms(timeout_ms: i64) -> bool` is a genuinely
non-blocking readiness poll on stdin (POSIX `poll()` on fd 0 /
Windows `WaitForSingleObject` on the console input handle) -- it
answers "would a read block right now?" **without reading anything**.
`read_move` calls it in `poll_ms`-sized slices (200ms here), each one
clamped to whatever's left of the turn's budget so the very last
slice can never overrun the deadline. Only once a slice reports
`true` does the function call the blocking `stdin_read_line()` -- and
at that point it's guaranteed not to actually block, because
`stdin_ready_within_ms` already confirmed a full line is sitting
there waiting.

This is the whole fix. No `task`, no `Atomic`, no `join`, no second
thread at all -- one function, one loop, one deadline. When time runs
out, `read_move` returns `-1` immediately; nothing was ever left
half-blocked, because the blocking call was never made until it was
already known to be safe.

The `prompted` flag exists so the "enter a position" prompt prints
once per *attempt* (including retries after invalid input), not once
per 200ms poll slice -- without it you'd see the prompt spammed five
times a second while waiting.

## `main()` gets simpler, not more complex

```vani
let idx: i64 = read_move(ref board, player, turn_timeout_ms, poll_ms);

if idx == 0 - 1 {
  // timed out -- print the result and return immediately, no join,
  // no thread left running, nothing to wait for.
  let winner_by_time: i64 = 3 - player;
  print player_label(player) + "'s time ran out!";
  print player_label(winner_by_time) + " wins on time!";
  return 0;
}

if idx == 0 - 2 {
  print "Goodbye!";
  return 0;
}

board[idx as u64] = player;
// ... same win-detection as the non-timed game from here ...
```

Compare this to the old version's `task`/`Atomic`/poll-loop/`join`
dance, plus the "still waiting on that player's pending input" message
it had to print because the process genuinely couldn't exit yet. That
entire limitation is gone -- there's simply nothing left to wait for.

## A JIT-startup-latency gotcha (not a bug)

If you shrink `turn_timeout_ms` for faster manual testing (see [Try
it yourself](#try-it-yourself)), you may notice `vanic run` (the LLVM
JIT path) appears to need a bit more wall-clock time before its own
`elapsed_ms` clock starts counting than `--backend=c` does. This
isn't a bug in `stdin_ready_within_ms` -- it's `vanic run`'s LLVM JIT
having real startup latency (already documented as this book's
[**L27** limitation](https://github.com/enthusiasticgeek/vani-compiler/blob/main/docs/v1_limitations.md))
that eats into a tight budget before the program itself has run a
single instruction. The real game's 15-second per-move budget makes
this a non-issue in practice; a 2-3 second test budget can make it
visible.

## History: two real compiler bugs, found building the original version

<img class="manas" src="../images/mascot/manas_mascot_awesome.png" title="two real compiler bugs, found and fixed"/>

This file's *first* version (the `task`/`Atomic`/`join` design
described above, before the `stdin_ready_within_ms` rewrite) hit two
real compiler bugs while it was being built -- both bisected to a
minimal repro, both fixed the same day they were found. Both are
still worth knowing about even though the current version of this
file no longer uses `task`/`join` at all (BUG-186 in particular was
specific to that pattern) -- they're real project history, and the
bisection process is the useful part.

**BUG-185** -- an early draft's mode-selection-style prompt did
roughly `let flag: bool = ...; print flag;` followed later by
`vec_fill(...)`, and `vanic run` failed outright: `lli` rejected the
compiled module with `"PHI node entries do not match predecessors!"`.
Root cause: the LLVM backend's `print <bool>` codegen branches to two
blocks (print "true" / print "false") and merges them back together,
but never updated the codegen context's notion of "which block are we
in now" after the merge -- so *any* later control-flow-producing code
in the same function (an `if`, a `while`, `vec_fill`'s own internal
loop) computed its next block's predecessors against a stale,
pre-branch block. Fixed by one missing assignment; finding it took
bisecting a much bigger repro down to four lines with no string
builtins at all. (This bug is also covered in
[Intermediate 17](../intermediate/17_tic_tac_toe_capstone.md#a-simple-computer-opponent),
since it was found there first, building the non-timed game's
computer opponent.)

**BUG-186** -- once the original `task`/`Atomic` design (spawn a task
once per turn, inside the game's `while true` loop) was in place,
`vanic check` crashed outright, with no `requires`/`ensures`/
`invariant` clause anywhere in sight:

```
thread 'main' panicked at src/checker.rs:37069:13:
internal error: entered unreachable code: task/join cannot appear in a proof position (requires/ensures/invariant)
```

Root cause: the bounds-elision pass's per-loop-iteration reassignment
tracker (`walk_for_reassigns`, used for the compiler's *own* internal
loop-invariant reasoning about array-index safety, not anything the
user writes) walks every `let`/assign statement in a loop body and
tries to symbolically substitute its right-hand side -- including
`let t: Task<i64> = task read_move_worker(...);` and `let idx: i64 =
join t;`. The substitution function had a defensive `unreachable!()`
for task/join expressions, written on the assumption they could never
reach that code path. False: this ran on *every* loop, unconditionally,
regardless of whether the loop had an explicit `invariant` clause --
so any loop containing `task`/`join` at all crashed the compiler,
100% reproducible. Fixed by returning the expression unchanged instead
of panicking: a task/join result isn't something the SMT layer can
reason about symbolically anyway, so any later fact depending on one
just goes unprovable (the normal, non-crashing fallback for any
unprovable shape) -- exactly what should have happened in the first
place.

---

## Try it yourself

1. Shrink `turn_timeout_ms` to something short (2000-3000) for faster
   manual testing -- and notice the JIT-startup-latency gotcha above
   if you test against `vanic run` specifically with too tight a
   margin.
2. Print a live countdown (`"5 seconds left..."`, updating in place)
   instead of a silent poll loop -- you'll need `\r` (carriage
   return, already in the base escape table) to overwrite the
   previous line. `stdin_ready_within_ms`'s slice-by-slice structure
   makes this natural: update the countdown once per slice, right
   next to the `elapsed_ms` update.
3. Combine this with [Intermediate 17](../intermediate/17_tic_tac_toe_capstone.md)'s
   computer opponent: only time the human's turns (the computer's
   `ai_move` is synchronous and instant, so it never needs a clock at
   all).
4. *(Bigger)* Read [Advanced 3](03_concurrency.md) and rebuild this
   file's *original* `task`/`Atomic<bool>`/`join` design from
   scratch, to feel the difference directly -- then compare the two
   versions' behavior on a timeout with `time` (real wall-clock time
   to process exit), the way this rewrite's own testing did.

---

## Summary

- `stdin_ready_within_ms(timeout_ms) -> bool` is a genuinely
  non-blocking readiness poll on stdin (POSIX `poll()` / Windows
  `WaitForSingleObject`) -- it answers "would a read block?" without
  reading. Call the blocking `stdin_read_line()` only once it returns
  `true`, and a timeout never leaves anything blocked.
- This replaces `task`/`Atomic<bool>`/`join` entirely for "race a
  blocking action against a countdown" -- no thread needed, because
  the blocking call itself is never made until it's known to be safe.
- `async`/`await` would not have solved this in v1: the async surface
  still desugars synchronously, so a blocking call inside `async fn`
  is still blocking. The real fix is a non-blocking *primitive*, not
  a different way to schedule a blocking one.
- The old design's real limitation -- `Task<R>` is affine (must
  always be `join`ed) and there was no cancellable blocking I/O, so a
  timed-out player's opponent had to wait for that player's eventual
  keystroke before the process could exit -- is now gone completely
  for THIS file, not just documented as an accepted tradeoff. (As of
  2026-08-14, `cancel <name>;` also makes blocking `tcp_accept`/
  `tcp_recv` interruptible in general -- see
  [Advanced 3](03_concurrency.md#cancel) -- but not `stdin_
  read_line`, so a `task`+`cancel` rewrite of THIS specific game
  still couldn't fully replace the non-blocking-poll approach below.)
- Two real, previously-unknown compiler bugs (BUG-185, BUG-186) were
  found and fixed building this file's original version -- a small,
  honest reminder that "the compiler has a bug in a construct you're
  using" is a real possibility worth knowing how to bisect toward.

---

**Previous**: [Sec.3 -- `task` / `join` / `detach` / `cancel` + atomics / mutexes / channels ->](03_concurrency.md)
**Next**: [Sec.3d -- Capstone: a concurrent sensor-dashboard pipeline ->](03d_concurrent_pipeline_capstone.md)
