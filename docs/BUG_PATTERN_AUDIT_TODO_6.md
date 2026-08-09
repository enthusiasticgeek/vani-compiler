# vāṇी — Bug-pattern audit, round 6

**STATUS (2026-08-09): category A CLOSED same-session as BUG-152.
Opened by first re-verifying round 5's fix generalizes correctly
(it does), then pivoting to a fresh area (integer print formatting)
which immediately turned up a real, previously-undiscovered
correctness bug on both SSA backends.**

Sequel to `docs/BUG_PATTERN_AUDIT_TODO_5.md` (round 5, closed
2026-08-09 as BUG-150 + BUG-151 -- struct field double-free on
conditional move, plus a `ref`-field move-diagnostic gap found while
verifying it). Round 5's own closing note flagged two things for a
follow-up: (1) whether the same double-free class extends to other
control-flow-merge shapes not individually tested, and (2) that
category B (other Drop/RAII edge cases) was left deliberately
unpopulated. This round starts by answering (1) directly, then -- once
that came back clean -- moves to a genuinely fresh theme rather than
resuming category B, since nothing about category B constituted a
confirmed lead worth chasing over a fresh area.

## Round 5 generalization check (closed, no new bugs -- BUG-150's fix holds)

Quick, targeted empirical checks before picking round 6's real theme:

- **Nested 3-level `if`/`else` with a divergent field move at the
  inner level**: compiles and runs correctly on both backends. The
  fix is structurally recursive (each `Stmt::If` reconciles its own
  immediate branches independently), and this confirms it composes.
- **`for` loop with the same conditional-field-move-inside-a-loop
  shape** that BUG-150's loop-safety extension rejects for `while`:
  correctly rejected too, with the same diagnostic -- confirms
  `validate_loop_balance`'s fix applies uniformly (it's a single
  shared function called from all loop-checking call sites).
- **Tuple elements**: not reachable. Direct `.0`/`.1` access on a
  non-Copy tuple element is already a compile error regardless of
  control flow ("would alias the tuple's heap data... use tuple
  destructuring instead") -- this class of bug can't occur for tuples
  because the language never lets you get into the aliased state to
  begin with.
- **Closures capturing a struct field by value across a branch**: not
  investigated deeply, but closures have their own, SEPARATE,
  pre-existing whole-variable Drop tracking (`Type::Closure` bindings
  go through the same `info.moved`-based mechanism BUG-150 already
  fixed for the whole-variable case) -- not the `moved_fields`
  per-field mechanism BUG-150 was specifically about, so the same gap
  doesn't directly apply. Not exhaustively proven safe, just a lower-
  priority combination given the mechanism difference.

No new bugs found in this pass -- treated as confirmation the BUG-150
fix is solid, not as a wasted check.

## A. `print` on unsigned integers displayed large values as negative (🔴 high) -- FIXED 2026-08-09, BUG-152

**Fixed same-session.** `print y;` where `y: u64` has the high bit
set (any value >= 2^63, including the very common `u64::MAX`) printed
as a negative signed number instead of its correct unsigned decimal
value -- on SSA-C and SSA-LLVM, the DEFAULT dispatch path for any
program simple enough not to need the tree backends. Confirmed
isolated purely to print formatting: arithmetic and comparisons on
the same u64 values were already correct throughout.

Minimal repro:
```vani
fn main() -> i64 {
  let y: u64 = 18446744073709551615;  // u64::MAX
  print y;
  return 0;
}
```
Before the fix: printed `-1` on both C and LLVM (SSA dispatch).
Tree-C was already correct (confirmed by forcing tree dispatch via a
`vec_remove_at` call in the same program).

Root cause: `ssa_backend_c.rs` and `ssa_backend_llvm.rs`'s
`intent_print_item` codegen both hardcoded `%lld` (signed decimal)
for every integer type reaching their fallback branch, never special-
casing unsigned types the way tree-C's `emit_print_expr_no_newline`
already did. A related, narrower, latent bug in the same area: tree-
LLVM's unsigned-print branch routed through the shared
`intent_print_int_<suffix>` dialect-digit-translation helper (used
for Devanagari/Bengali/Tamil/... pragma-selected scripts) whenever a
dialect was active -- that helper ALSO formats via `%lld` internally
on both its C and LLVM implementations, so a dialect-mode program
printing a large u64 would hit the same bug via a different, narrower
trigger (dialect pragma + large u64, vs. just "any large u64" for the
main bug). Tree-C never routed unsigned types through this helper at
all, so it never had this particular manifestation (though it also
means tree-C's unsigned prints never get dialect digit translation --
a separate, pre-existing, unrelated completeness gap, not touched).

Fixed at all three sites: `ssa_backend_c.rs`'s fallback format
selection gained an unsigned arm (`%llu`/`(unsigned long long)`), and
its dialect-suffix branch is now signed-only. `ssa_backend_llvm.rs`
mirrors this (dialect-suffix call gated on `is_signed_int`, unsigned
fallback format is `%llu`). `backend_llvm.rs`'s (tree-LLVM) unsigned
branch no longer attempts dialect dispatch at all, matching tree-C's
existing convention exactly.

Verified on all four codegen paths (SSA-C/SSA-LLVM via default
dispatch, tree-C/tree-LLVM via forced non-SSA dispatch), plus a
dialect-mode sanity check confirming signed dialect translation still
works while unsigned prints correctly fall back to plain ASCII with
the right value. Full writeup, including the example-corpus dialect
sweep and its one confirmed-unrelated finding, in
`docs/TODO_CURRENT.md`'s BUG-152 section.

5 regression tests added (3 `src/lib.rs` compile-checks across the
three fixed sites, 2 `tests/run_end_to_end.rs` real-subprocess tests
covering both dispatch paths on both backends).

## Process (mirrors rounds 1 through 5's own process sections)

1. Check `docs/TODO_LOCAL_STAGING.md` (in the `vani-compiler-localfuzz`
   worktree) for anything landed since this file's creation
   (2026-08-09) -- re-verify each against a freshly rebuilt `main`
   first; the worktree has gone stale mid-day more than once this
   week (see `docs/TODO_CURRENT.md`'s BUG-149 section for the first
   incident and its cause -- a dirty `Cargo.lock` silently stalling
   the nightly refresh -- and check `tools/localfuzz/refresh.log`'s
   tail for a recent "refresh done" line before trusting anything the
   harness currently reports).
2. Round 6 is closed -- category A fixed same-session, no category B
   populated. A future session should pick a genuinely new round-7
   theme rather than resume anything from here. Worth considering,
   NOT pre-populated as confirmed leads (found only by inference,
   never empirically tested): other decimal/string-formatting sites
   that might share BUG-152's "signed-by-default" shape --
   `i64_to_str` called on a value whose SOURCE type was actually
   unsigned via some coercion path, `EPrint` (the `eprint`/stderr
   variant -- has its own separate emission functions,
   `emit_eprint_items_llvm` et al., not audited this round), and
   whether `f64_to_str`/float formatting has any analogous
   width/signedness edge case (unlikely, floats don't have an
   unsigned variant, but worth a sanity check before assuming).
3. Every fix gets a `src/lib.rs` compile-check test AND a
   `tests/run_end_to_end.rs` real-subprocess test on both backends --
   established convention, upheld this round.
4. Full `cargo test --release` clean + `vanic check examples`
   compared against the current baseline (78 errors as of this
   writing) before any push. Verify freshness before every commit.
5. CI/CodeQL polled green after every push.
