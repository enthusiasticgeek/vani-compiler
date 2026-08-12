# Advanced 3c -- Capstone: timed tic-tac-toe (`task` / `join` / `Atomic<T>`)

> **Learning goal**: apply `task`, `join`, and `Atomic<T>` from
> [Advanced 3](03_concurrency.md) to a real problem -- "race a
> blocking action against a countdown" -- and learn exactly where
> that pattern's edges are in v1: `join` can't cross a block
> boundary, and there's no way to cancel a task blocked in a real
> blocking syscall. Both are real constraints this chapter hit while
> building the example, not hypotheticals.

This is a walking tour of
[`examples/language/english/tic_tac_toe_timed.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/english/tic_tac_toe_timed.vani),
a standalone variant of the tic-tac-toe game from
[Intermediate 17](../intermediate/17_tic_tac_toe_capstone.md): same
board, win-detection, and ANSI color, but every move now has a
15-second clock. Run out of time and you forfeit the game. It's a
separate file, not a modification of the original -- the board/color/
win-detection code is duplicated rather than shared, since this is a
standalone example, not a library.

## Build & run

```bash
vanic run examples/language/english/tic_tac_toe_timed.vani                          # LLVM backend, JIT via lli
vanic run examples/language/english/tic_tac_toe_timed.vani --backend=c              # C backend, gcc
vanic build examples/language/english/tic_tac_toe_timed.vani -o /tmp/ttt_timed && /tmp/ttt_timed
```

---

## The race: a task vs. a countdown

```vani
fn read_move_worker(board: ref Vec<i64>, player: i64, done: mut ref Atomic<bool>, result: mut ref Atomic<i64>) -> i64 {
  while true {
    let prompt: OwnedStr = "Player " + player_label(player) + ", enter a position (1-9), or 'quit' to exit:";
    print prompt;
    flush_stdout();
    let input: OwnedStr = stdin_read_line();
    let trimmed: OwnedStr = str_trim(input);
    if trimmed == "" {
      let _ = atomic_store(result, 0 - 1);
      let _ = atomic_store(done, true);
      return 0 - 1;
    }
    if trimmed == "quit" {
      let _ = atomic_store(result, 0 - 1);
      let _ = atomic_store(done, true);
      return 0 - 1;
    }
    if let Option.Some(n) = parse_int(trimmed) {
      if n >= 1 {
        if n <= 9 {
          let idx: i64 = n - 1;
          if board[idx as u64] == 0 {
            let _ = atomic_store(result, idx);
            let _ = atomic_store(done, true);
            return idx;
          } else {
            print "That cell is already taken -- try again.";
          }
        } else {
          print "Please enter a number from 1 to 9.";
        }
      } else {
        print "Please enter a number from 1 to 9.";
      }
    } else {
      print "That's not a number -- try again.";
    }
  }
  return 0 - 1;
}
```

This is nearly the same read-validate-retry loop as the non-timed
game's `read_move`, spawned as a `task` (a real OS thread) once per
turn. Two differences matter:

- The result is published through `Atomic<bool>` (`done`) and
  `Atomic<i64>` (`result`), not just returned. The main thread needs
  to ask "has this finished yet?" *from outside this thread, without
  blocking on it* -- a plain return value can't answer that question
  until the thread has already finished, which defeats the purpose.
- A bad line (non-numeric, out of range, occupied cell) retries
  *inside this same thread*, silently, without touching `done`. That
  means typos don't buy extra time -- the clock keeps running, same
  as a real turn-based game timer.

## The poll loop, and the one-join-per-block rule

```vani
let done: Atomic<bool> = atomic_new(false);
let result: Atomic<i64> = atomic_new(0 - 1);
let t: Task<i64> = task read_move_worker(ref board, player, mut ref done, mut ref result);

let elapsed_ms: i64 = 0;
let timed_out: bool = false;
while true {
  if atomic_load(ref done) {
    break;
  }
  if elapsed_ms >= turn_timeout_ms {
    timed_out = true;
    break;
  }
  sleep_ms(poll_ms);
  elapsed_ms = elapsed_ms + poll_ms;
}
```

The main thread polls `done` every 200ms (`poll_ms`), counting
elapsed time against `turn_timeout_ms` (15000). Whichever condition
is true first -- the human answered, or the clock ran out -- decides
`timed_out`.

<img class="manas" src="../images/mascot/manas_mascot_caution.png" title="join can't cross a block"/>

**A real constraint, found the hard way:** the first draft of this
loop tried to `join t` inside two different branches -- `if
timed_out { join it one way } else { join it the other way }`. That
doesn't compile:

```
error: join: task 't' was not spawned in this block (cross-block joins aren't supported in v1)
```

`task`/`join` must appear in the *same* block -- v1 doesn't support
joining a task from a different block than the one that spawned it,
even a nested `if`/`else` one level down. The fix turned out to be
the right design anyway: exactly **one** unconditional `let idx: i64
= join t;`, right after the poll loop, in the same block as the
spawn. `timed_out` (already computed by then) only decides what
*message* to print -- the join itself always happens exactly once,
unconditionally.

## The real limitation: you can't cancel a blocking read

<img class="manas" src="../images/mascot/manas_mascot_caution.png" title="no cancellable I/O in v1"/>

Here's the question that matters most: when the countdown wins the
race, what happens to the worker thread? It's still sitting inside a
real, blocking `stdin_read_line()` call. Two facts about v1 combine
to make this a real constraint, not just an implementation detail:

1. **There is no non-blocking or cancellable stdin read.** Checked
   directly: `tcp_set_nonblocking` / `tcp_recv_nb` exist for TCP
   sockets, nothing equivalent exists for stdin.
2. **`Task<R>` is affine.** The compiler requires every spawned task
   to eventually be `join`ed -- no "fire and forget," no dropping a
   handle. This is enforced at compile time, not a convention.

Put together: once the timer wins, the worker thread cannot be
killed, and the program cannot exit without eventually joining it.
The `join t;` shown above is not just cleanup -- when `timed_out` is
true, it genuinely **blocks for real** until that player's pending
input actually arrives (or the process is killed):

```vani
if timed_out {
  let timeout_msg: OwnedStr = player_label(player) + "'s time ran out!";
  print timeout_msg;
  print "(the game is over -- still waiting on that player's pending input so the program can fully exit; press Enter or Ctrl-D)";
}
let idx: i64 = join t;
```

Two ways to handle this were considered:

- **What's implemented: join at the end, document the wait.** The
  game declares the timeout outcome immediately (prints the forfeit
  message and who won), but the process itself doesn't fully exit
  until the abandoned worker's `stdin_read_line()` actually returns
  -- a stray keystroke, Ctrl-D, or the process being killed. No
  unsafe code, works identically on every platform. The honest
  tradeoff: a player who times out and then never touches the
  keyboard again leaves the process hanging around (though the
  *game's outcome* is already decided and printed).
- **Not implemented: force-unblock via FFI.** When the timer fires,
  call libc's `close(0)` through `extern "C"` FFI (see
  [Intermediate 9 -- FFI](../intermediate/09_ffi.md)) to forcibly
  close the stdin file descriptor. The pending `read()` inside
  `stdin_read_line()` unblocks immediately with an error/EOF, the
  worker thread finishes, and `join` returns right away -- a fully
  clean, instant exit. The costs: it needs an `unsafe(reason = "...")`
  block, it's POSIX-specific (`close()` on an arbitrary file
  descriptor doesn't behave the same way on Windows), and closing fd
  0 process-wide is a blunt instrument if anything else in the
  program still wants to read stdin afterward. Left as
  [Try it yourself](#try-it-yourself) item 2 below rather than
  shipped, since the added `unsafe`/platform-specific complexity
  isn't worth it for a demo whose main point is the `task`/`Atomic`
  pattern, not FFI.

## Two real compiler bugs, found building this

<img class="manas" src="../images/mascot/manas_mascot_awesome.png" title="two real compiler bugs, found and fixed"/>

Neither of these is hypothetical -- both blocked this file from
compiling or running at all, both were bisected to a minimal repro,
and both are fixed (separate commits, same day).

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

**BUG-186** -- once this file's actual design (spawn a task once per
turn, inside the game's `while true` loop) was in place, `vanic
check` crashed outright, with no `requires`/`ensures`/`invariant`
clause anywhere in sight:

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

## A JIT-startup-latency gotcha (not a bug)

While testing the timeout path with a tight real-time margin (a 3-
second delayed stdin write against a 2-second test timeout, before
settling on the real 15-second budget), `vanic run` appeared to
*miss* a timeout that `--backend=c` correctly caught. This looked
like a race condition at first. It wasn't: `vanic run`'s LLVM JIT
path has real startup latency (already documented as this book's
[**L27** limitation](https://github.com/enthusiasticgeek/vani-compiler/blob/main/docs/v1_limitations.md))
that ate into the tight 2-second budget before the program's own
`elapsed_ms` clock even started counting. Confirmed by widening the
margin (a 10-second delay against the same 2-second timeout): LLVM
correctly detected the timeout once the budget was generous enough
that JIT startup couldn't eat all of it. The real game's 15-second
per-move budget makes this a complete non-issue in practice -- but
if you shrink `turn_timeout_ms` for testing (see
[Try it yourself](#try-it-yourself) item 1), keep this in mind before
concluding you've found a bug.

---

## Try it yourself

1. Shrink `turn_timeout_ms` to something short (2000-3000) for faster
   manual testing -- and notice the JIT-startup-latency gotcha above
   if you test against `vanic run` specifically with too tight a
   margin.
2. *(Bigger)* Implement the FFI `close(0)` cancellation approach
   described above: on timeout, call libc's `close` on file
   descriptor 0 before `join`ing, and confirm the join now returns
   instantly instead of waiting for real input.
3. Print a live countdown (`"5 seconds left..."`, updating in place)
   instead of a silent poll loop -- you'll need `\r` (carriage
   return, already in the base escape table) to overwrite the
   previous line.
4. Combine this with [Intermediate 17](../intermediate/17_tic_tac_toe_capstone.md)'s
   computer opponent: only time the human's turns (the computer's
   `ai_move` is synchronous and instant, so it never needs a task or
   a clock at all).

---

## Summary

- `Atomic<bool>` / `Atomic<i64>` are how a polling main thread
  observes a spawned task's progress *without* blocking on it --
  `join` is for "wait until done," atomics are for "has it finished
  yet, right now, non-blocking."
- `task`/`join` must be spawned and joined in the same block -- no
  cross-block joins in v1. Design around this by keeping exactly one
  unconditional `join` per spawn, using already-computed flags to
  decide what to do with the result, not which `join` call to run.
- There's no cancellable or non-blocking stdin read in v1, and
  `Task<R>` is affine -- so a task blocked on real I/O can't be
  abandoned. The honest design accepts that a timed-out player's
  eventual (or Ctrl-D'd) input is what actually lets the process
  exit; the alternative (FFI `close()`) trades that for `unsafe`,
  platform-specific complexity.
- Two real, previously-unknown compiler bugs (BUG-185, BUG-186) were
  found and fixed building this file -- a small, honest reminder that
  "the compiler has a bug in a construct you're using" is a real
  possibility worth knowing how to bisect toward.

---

**Previous**: [Sec.3 -- `task` / `join` + atomics / mutexes / channels ->](03_concurrency.md)
**Next**: [Sec.3b -- Condition variables primer ->](03b_condvar_primer.md)
