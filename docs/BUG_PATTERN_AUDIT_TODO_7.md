# vāṇी — Bug-pattern audit, round 7

**STATUS (2026-08-09): CLOSED, clean sweep -- no new bugs found.**
Every item checked this round came back verified-safe, several via
`valgrind --leak-check=full` rather than just "didn't crash." This is
a legitimate, useful outcome (mirrors round 4's own systematic-sweep
closure) -- it retires several previously-flagged "worth checking"
items with real evidence instead of leaving them as open questions,
and rules out a handful of plausible-sounding bug classes that turned
out not to exist.

Sequel to `docs/BUG_PATTERN_AUDIT_TODO_6.md` (round 6, closed
2026-08-09 as BUG-152 -- `print(u64)` displayed large unsigned values
as negative on both SSA backends).

## What was checked, and why

Localfuzz first (per the established process): worktree had gone
stale again (third time in one day -- see the note at the end of this
doc), refreshed, no new candidates since the last triage.

**Round 6's explicit follow-up list** (from that doc's "Process"
section -- things flagged as *not yet empirically tested*, not
confirmed leads):
- **`eprint`/stderr print variant**: checked directly -- already
  correct on tree-C (`emit_eprint_expr_no_newline` in `backend_c.rs`
  already special-cases `Type::U8|U16|U32|U64` with `%llu`, identical
  to `print`'s own convention) and empirically prints a large u64
  correctly on both backends via the default (SSA) dispatch path too.
  Not a gap.
- **`i64_to_str` receiving an unsigned-sourced value via coercion**:
  not applicable -- there's no way to feed a `u64` into `i64_to_str`
  without an explicit `as i64` cast, and an explicit cast correctly
  reinterpreting the bit pattern as signed is the CORRECT, intended
  behavior for that cast, not a bug.
- **Float formatting edge cases**: checked `f32`, `f64::INFINITY`,
  `-INFINITY`, and `NaN` -- all print correctly and identically on
  both backends (`1.5`, `inf`, `-inf`, `nan`).

**Round 5 category B's two flagged-but-never-tested items**, now
verified with `valgrind --leak-check=full` (not just "the process
didn't crash," which is weak evidence for a leak specifically):
- **`Vec<Struct>` where the struct has its own heap-owning field,
  dropped naturally at end of scope** (not via `clone_at`, which
  round 4 already covered): 0 leaks, 0 errors, 4 allocs / 4 frees for
  a 2-element `Vec<Node>` with each `Node` owning a nested `Vec<i64>`.
- **Recursive function with a heap-holding local allocated fresh on
  every call, 50 levels deep**: 0 leaks, 0 errors, 51 allocs / 51
  frees (one per call, all correctly freed on unwind).

**Fresh checks, not carried over from any prior round's notes**:
- **Narrow-integer-width (`i8`/`u8`) overflow trapping**: confirmed
  both signed and unsigned narrow-width overflow are caught by the
  same checked-arithmetic convention i64 uses (`integer overflow in
  int8_t add` / `integer overflow in uint8_t add`, C exit 134 / LLVM
  exit 3) -- not a gap.
- **Cross-backend determinism for `hash_i64` and `seed_rng`/`rand_i64`**:
  a hash value and a two-value seeded-RNG sequence were byte-for-byte
  identical between C and LLVM. Not a gap -- and this incidentally
  re-confirmed BUG-152's fix on a real (non-literal) large `u64` hash
  value, not just the test-case literals used in that bug's own
  regression tests.
- **Enum payload move-tracking across `match` arms** (the natural
  "does BUG-150's shape recur for enums" question): turned out not to
  be applicable by design, not merely safe-by-luck -- a `match`
  arm's payload binding for a heap-owning variant (e.g. `OwnedStr`)
  is typed as the VIEW type (`Str`), not the owning type. No
  ownership transfer happens via match extraction at all, so there's
  no "conditionally moved on one arm" scenario to have a merge bug
  about in the first place. Confirmed by reading `checker.rs`'s `if
  let`/match-desugaring code (`Type::OwnedStr => Type::Str` view-type
  substitution) and by a compile error when trying to pass a payload
  binding somewhere an `OwnedStr` (not `Str`) is required.

## Round 5 category B: now fully closed

Both of category B's original items (`Vec<Struct>` natural drop,
recursive-function Drop) are now verified clean with real evidence.
The third item that doc mentioned in passing (whether an `abort()`-
triggering runtime trap ever risks running partial drop glue before
the process dies) wasn't separately tested this round -- `abort()`
doesn't unwind in C, so there's no drop glue to run at all after it
fires; this is true by construction of how the trap sites are
implemented (a direct `abort()` call, not a panic/unwind mechanism),
not something that needs a runtime check to confirm.

## Process (mirrors rounds 1 through 6's own process sections)

1. Check `docs/TODO_LOCAL_STAGING.md` (in the `vani-compiler-localfuzz`
   worktree) for anything landed since this file's creation
   (2026-08-09) -- re-verify against a freshly rebuilt `main` first.
   **The worktree went stale THREE separate times in a single day
   this session** (missing BUG-149/150/151 at different points,
   caught and refreshed each time before trusting anything). The
   nightly-only refresh cadence isn't tight enough during a high-
   velocity fixing session -- worth considering either a faster
   refresh cadence or making "refresh, then check for new candidates"
   a standing first step every single time (which is what this round
   and round 6 both already did by habit, but it's worth calling out
   explicitly as a recurring pattern rather than a one-off).
2. Round 7 is closed with no open leads. A future session should pick
   a genuinely new round-8 theme. Rounds 4-7 have now covered: bounds-
   check coverage (round 4), Drop/RAII merge-point double-free (round
   5), print/format-string signedness (round 6), and a broad-but-
   shallow validation sweep across several previously-flagged items
   (round 7, this doc) -- areas NOT yet touched by any round: generics/
   monomorphization edge cases, FFI/extern-boundary correctness,
   concurrency primitive edge cases beyond the one deadlock localfuzz
   already found and confirmed benign, and string (`Str`/`OwnedStr`)
   builtin correctness at UTF-8 boundaries (only ASCII-range test
   inputs have been used anywhere in this session's own testing).
3. N/A this round -- no fixes to test, since nothing was found broken.
4. N/A this round.
5. N/A this round -- no code changed, so no full-suite/check-examples
   run was needed before this doc's own push (docs-only commit).
6. CI/CodeQL polled green after the push.
