# vāṇी — Bug-pattern audit, round 5

**STATUS (2026-08-09): category A FIXED as BUG-150 (+ a second,
narrower gap found while verifying it, fixed as BUG-151). Category B
is still open/unstarted, deliberately left unpopulated (see its own
note below).**

Sequel to `docs/BUG_PATTERN_AUDIT_TODO_4.md` (round 4, closed
2026-08-09 as BUG-149 -- see that file's category A for the
bounds-check-parity story that led here). Round 4's fix (BUG-149)
revealed a useful methodology: when a codegen fix lands for one
"shape" of a construct (there, `Vec<T>` indexing), check whether a
structurally parallel shape (there, `Type::Array` indexing) got the
same treatment. Applying the same "what's the parallel shape nobody
re-checked" instinct to a completely different area -- Drop/RAII
codegen, untouched by rounds 1-4 -- immediately found something serious.

## A. Struct field double-free when a move happens on only one incoming control-flow path (🔴 CRITICAL) -- FIXED 2026-08-09, BUG-150 (+ BUG-151, a second gap found while verifying it)

**Update (2026-08-09)**: fixed as BUG-150. The `Stmt::If` merge in
`checker.rs` now reconciles per-FIELD move divergence the same way it
already reconciled whole-variable divergence -- a compensating single-
field `Drop` (reusing the existing `moved_fields` skip-list mechanism,
enumerated via `env.lookup_struct`) inserted into the non-moving
branch, and the merged env's `moved_fields` set to the union of both
branches. No new IR node or backend codegen changes were needed --
both backends' `Drop` handling already correctly respected a
`moved_fields` skip-list of any size; the bug was purely in the
checker never computing the right skip-list to begin with. Verified:
the repro below (and its reverse-direction and loop-nested variants)
no longer double-free; the still-live sibling field stays usable;
using the moved field again after the merge is correctly rejected.

**A second, narrower gap surfaced while verifying the fix, fixed as
BUG-151**: `ref t.field` / `mut ref t.field` never checked whether
that specific field had been moved (only the whole-binding flag,
which plain `t.field` already correctly guarded against) -- a real,
if narrow, use-after-free hole, now closed with the same
`moved_fields` check plain `FieldAccess` already had.

**A loop-safety complication found while verifying the fix**: the
same compensating-drop trick, applied naively to an `if`/`else`
NESTED INSIDE A LOOP BODY, is unsound on its own -- the checker
processes the loop body once, so the compensating drop re-fires every
iteration that takes the non-moving arm, double-freeing a second way.
Fixed by extending the pre-existing `validate_loop_balance` (which
already rejected the equivalent whole-variable pattern at compile
time) to also check `moved_fields` divergence, rather than trying to
make the runtime trick itself loop-safe (that would need real runtime
drop flags -- left as a documented possible future enhancement, not
attempted).

Full writeup, including the one deliberately-left limitation (structs
with a user-defined by-ref `drop`, a narrow combination), in
`docs/TODO_CURRENT.md`'s BUG-150 section.

<details>
<summary>Original writeup, kept for the reasoning trail</summary>

**Confirmed, unfixed, both backends:** moving a struct field out (by
passing it by value to a function) inside one arm of an `if`/`else`,
while only *borrowing* the same field in the other arm, produces a
genuine double-free once control flow rejoins after the `if`/`else`
-- but only when the move-arm is the one actually taken at runtime.
This is not a hypothetical or narrow edge case: "take ownership of a
field if some condition holds, otherwise just look at it" is an
extremely natural, common pattern.

Minimal repro:
```vani
struct Pair { a: Vec<i64>, b: Vec<i64> }
fn consume(v: Vec<i64>) -> i64 { return len(ref v) as i64; }
fn get_flag() -> bool { return true; }
fn main() -> i64 {
  let p: Pair = Pair { a: vec(1,2,3), b: vec(4,5,6) };
  let flag: bool = get_flag();
  if flag {
    let n: i64 = consume(p.a);   // moves p.a out
    print n;
  } else {
    print len(ref p.b) as i64;   // only borrows p.b, p.a untouched
  }
  return 0;                      // <-- p's drop glue lives here
}
```
- The compiler accepts this with **zero diagnostics** on either
  backend -- no move error, no partial-move warning.
- At runtime, when `flag` is `true` (the move-arm executes): both
  backends crash identically with `free(): double free detected in
  tcache 2`, C exit 134, LLVM exit 134 (glibc's malloc consistency
  check catches it and aborts -- this is real heap corruption, not a
  clean, intentional trap like BUG-147/148/149's `abort()` calls).
- When `flag` is `false` (the borrow-arm executes, confirmed by
  flipping `get_flag`'s return value): runs cleanly, prints `3`, exit
  0 -- no crash, because `p.a` was never moved on that path and the
  struct's own fields are genuinely both still owned at scope end.

This confirms the bug is exactly what it looks like: `p`'s end-of-
scope drop glue (emitted once, after the `if`/`else` merges back into
a single continuation) frees `p.a` unconditionally, without knowing
which branch actually ran -- so it double-frees whenever the
move-arm was taken.

**A closely related shape does NOT reproduce the bug** -- worth
recording since it sharpens the root cause. Moving the same field
right before an *early return from within the branch itself* (so
there's no shared merge point -- each return site gets its own
tailored drop sequence) works correctly:
```vani
  if get_flag() {
    let n: i64 = consume(p.a);
    print n;
    return 0;      // <-- this branch's own drop glue, computed for
  }                 //     THIS path specifically -- correct
  print len(ref p.b) as i64;
  return 0;
```
This runs cleanly on both backends regardless of which branch is
taken. That comparison narrows the root cause precisely: per-return-
site drop-glue computation is path-aware and correct; it's
specifically the **shared continuation after a control-flow merge**
(`if`/`else` with no early return, `while`/`for` loop bodies with
conditional moves inside them, `match` arms that rejoin, etc.) where
the compiler needs to know "was this field moved on the path that
actually got here" and currently doesn't track it -- it appears to
statically pick one path's moved-set (plausibly always the
`else`/fallthrough path's) rather than merging the two, or generating
a *runtime* drop flag the way real Rust does for exactly this pattern.

Root cause (from behavior, not yet read against the checker/codegen
source in depth -- next session should start here): the move-tracking
that computes "is this field still owned at this program point" is
evaluated per-branch correctly for return-early sites, but the
struct's `moved: HashSet<&String>` set (see
`emit_llvm_struct_field_drops` in `backend_llvm.rs`, and presumably a
parallel mechanism in `backend_c.rs`) that drives per-field drop
skipping at a MERGED continuation point is apparently computed
without regard to which incoming edge of the merge is live at
runtime. Two plausible fix shapes, in increasing order of
implementation cost:

1. **Reject at compile time** (matches this codebase's existing
   convention of preferring compile-time rejection over unsound
   runtime behavior -- see e.g. the `s.data[idx] = 99` /
   `mut ref s.v[idx]` / non-Copy nested-indexing restrictions, all
   discovered as "already rejected, not a gap" during this session's
   round-4 follow-up work). Diagnose "field '{name}' is moved on one
   branch but not another; move it on both, or move it before the
   branch" the same way the checker already diagnoses move-after-use
   elsewhere. Simplest, smallest, most conservative -- but rejects
   the common, previously-silently-accepted pattern above, likely
   breaking some already-written vani programs that "worked" only
   because they never happened to double-free at runtime (e.g. if
   the moved field's Drop is a no-op for empty collections, or if the
   move-arm and free just happened not to run in whatever testing
   happened).
2. **Real runtime drop flags** (matches real Rust's own solution to
   this exact problem: a hidden per-conditionally-moved-field boolean
   local, set to false at the move site, checked before the field's
   drop at the merge point). More permissive, more work, touches both
   backends' drop-glue codegen and the checker's move-tracking to
   thread the flag through.

</details>

## Process (mirrors rounds 1 through 4's own process sections)

1. Check `docs/TODO_LOCAL_STAGING.md` (in the `vani-compiler-localfuzz`
   worktree) for anything landed since this file's creation
   (2026-08-09) -- re-verify each against a freshly rebuilt `main`
   before trusting it (see `feedback_vani_localfuzz_staleness`; also
   see `docs/TODO_CURRENT.md`'s BUG-149 section for a fresh reminder
   of what happens when the nightly refresh silently stalls -- check
   `tools/localfuzz/refresh.log`'s tail for a recent "refresh done"
   line before trusting the harness's current findings at all).
2. ~~Work category A first~~ -- done, fixed as BUG-150 (+ BUG-151).
3. ~~Sweep the other control-flow-merge shapes~~ -- done as part of
   BUG-150's own verification: `match` arms were already safe (a
   pre-existing conservative "any-arm-moved counts as moved" strategy,
   confirmed by direct testing, not a gap); loop-nested `if`/`else`
   was NOT safe and is now correctly rejected at compile time (see
   the loop-safety note above) rather than silently reconciled.
   Nested `if`/`else` three-plus levels deep was not separately
   tested -- the fix is structurally recursive (each `Stmt::If`
   reconciles its own immediate branches independently), so it should
   compose correctly, but this wasn't empirically re-verified with a
   3-level-deep repro; worth a quick spot-check if this area gets
   touched again.
4. Every fix gets a `src/lib.rs` compile-check test (or, if category
   A's fix is a compile-time rejection, a test asserting the
   diagnostic fires) AND a `tests/run_end_to_end.rs` real-subprocess
   test on both backends -- established convention.
5. Full `cargo test --release` clean + `vanic check examples`
   compared against the current baseline (78 errors as of this
   writing -- re-check the live count, it drifts) before any push.
   Verify freshness (`git fetch origin && git log origin/main --oneline -3`)
   before every commit -- a concurrent localfuzz process also lands
   commits.
6. CI/CodeQL polled green after every push.

## B. (reserved) Other Drop/RAII edge cases, once category A is fixed

Category A alone is substantial enough to open this round with. Once
it's fixed, worth a fresh look (not pre-populated with leads the way
category A is, since finding category A ate this session's remaining
budget): recursive functions with heap-holding locals under deep
recursion, `Vec<Struct>` element-wise drop when the outer Vec itself
naturally goes out of scope (not via `clone_at`, which round 4 already
covered), and whether an `abort()`-triggering runtime trap (bounds
check, overflow, assert failure) mid-function ever risks running
partial drop glue before the process dies (plausibly not, since
`abort()` doesn't unwind -- worth a one-line confirmation, not a deep
dive, if category A doesn't already answer it in passing).
