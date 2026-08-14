# Next-sessions plan (created 2026-08-14, not started)

> Saved before starting so the plan survives a session boundary.
> Task IDs below refer to the harness's own TaskCreate/TaskList
> tracker (not GitHub issues) -- re-create them from this file if
> the tracker state is ever lost. Status as of this writing:
> **everything below is pending, nothing in progress.**

## Context

Follows directly from the async/cancellation work shipped 2026-08-14
(see `TODO_CURRENT.md`'s "Arc 8 v3.2" and "Phase D" entries, and
memory `project_vani_async_executor_and_cancel_2026_08_14`). The
user's plan, in order:

(a) Start with Phase F and complete it.
(b) Identify any limitations in v1 and triage them for the next few
    sessions.
(c) Create a unit-testing/test harness for vani, useful for TDD.
(d) Fix localfuzz findings that haven't been fixed yet.
(e) Update documentation/tutorials/md files.

## Analysis performed before task creation (grounded, not guessed)

- **localfuzz worktree** (`/home/virgo/source/vani-compiler-localfuzz`)
  was **6 commits behind `origin/main`** at plan-creation time (does
  not yet include the async/cancel work) and has local-only,
  intentional modifications to `allowed_paths.conf`/
  `allowed_readonly_paths.conf` (do NOT blanket `git checkout --` in
  that worktree -- see memory `feedback_vani_localfuzz_worktree_checkout_pitfall`).
  Its latest digest (`tools/localfuzz/DIGEST_LATEST.md`, generated
  20260814-100618Z) showed **37 findings collapsing to 5 distinct
  signatures** -- 4 of those already keyword-match `BUG-177` in the
  docs (likely already fixed; needs confirming against a freshly
  rebuilt binary, not just a docs-grep match) and **1 is genuinely
  untriaged** (`assertion failed:`, 1 occurrence). So (d) is smaller
  and more bounded than "fix unfixed findings" sounds -- mostly
  *confirm*, plus one real unknown, as of this snapshot.
- **`docs/v1_limitations.md`** has **7 open items** (L5, L6,
  L10-macOS, L13, L14, L24, L25). Per the doc's own status table,
  most are **by-design** (L5, L6, L13, L14) or **blocked on hardware
  not available in this environment** (L10 macOS; L24, L25 Windows).
  So (b), read as "triage the existing catalog," will mostly
  re-confirm known state rather than surface a large new backlog --
  unless the intent is a fresh audit for genuinely NEW gaps (a
  different, open-ended activity, closer in spirit to the
  `BUG_PATTERN_AUDIT_TODO_N.md` series this project already runs
  periodically).

## Suggested improvements (discussed and adopted)

1. **(b) and (d) merged** into one audit task -- both are "find real
   compiler gaps"; running them separately risked two disconnected
   TODO docs and duplicate triage.
2. **(c) scope clarified with the user**: "vani-language test
   framework first, internal compiler-testing tooling second" (not
   instead of). These differ by roughly an order of magnitude in
   size -- a `#[test]`-style vani-language feature vs. improving the
   compiler's own test tooling on top of the existing 2966 lib
   tests / 259 e2e tests / 1050-file corpus check / localfuzz.
3. **(a) Phase F sequenced first**, not just because it was listed
   first -- it's already scoped from a prior session's plan, and
   every dogfooding pass this month (including the async/cancel work
   that immediately preceded this plan) found real compiler bugs by
   actually exercising a feature end-to-end. Expect it to feed real
   findings into the (b)+(d) audit rather than being independent of it.
4. **(e) is not a terminal phase.** Docs get updated per-item as each
   thing ships (matching this month's own successful pattern), plus
   one final consistency sweep at the very end -- batching everything
   into one doc pass after all code work tends to produce a worse,
   less accurate result.
5. **Validation discipline carried forward** from the async/cancel
   session: full `cargo test --release --workspace` + corpus check
   (diff the exact passing file SET, not just the count) + ASan/
   valgrind for anything touching drop/threading/codegen, and --
   the new lesson from that session -- `strace`-level verification
   for anything signal- or syscall-adjacent. "Compiles and the new
   test passes" was insufficient evidence three separate times in
   one session; don't regress to that bar.
6. **No subagents for vani work** (standing feedback from earlier
   sessions -- a fork stalled 15-16h on a small task once). All of
   this gets done directly.

## Task list (harness tracker IDs, current state: all pending)

1. **#185 -- Phase F: tic-tac-toe re-implementation using Executor + cancel.**
   Build a 2nd/3rd implementation of `tic_tac_toe_timed.vani` using
   the `Pollable`/`Executor` pattern and/or `cancel <name>;` for the
   per-turn timeout. Compare against the existing
   `stdin_ready_within_ms` non-blocking-poll version. Expect real
   compiler bugs to surface -- budget for bug-hunting, not just
   feature-writing. Full verification battery before done.

2. **#186 -- Refresh localfuzz + audit v1 limitations, produce one
   prioritized fix list.** (blocks #187)
   - Refresh the localfuzz worktree via `refresh.sh` so repros are
     trustworthy (it's currently stale relative to main).
   - Re-run `digest.py` against the refreshed binary; confirm the
     `BUG-177`-keyword-matched signatures are genuinely fixed on
     current main (not just docs-matched), isolate real unknowns.
   - Re-check `docs/BUG_PATTERN_AUDIT_TODO_9.md`'s not-started
     candidates (BUG-159-family leaks: `hashmap_get`/
     `contains_key`/`remove`, `Trie.insert`) and
     `docs/v1_limitations.md`'s still-open items (confirm L5/L6/L13/L14
     are still correctly by-design; decide whether to attempt
     best-effort work on L10/L24/L25 despite no hardware, or leave
     them explicitly scoped out as before).
   - Produce ONE combined, prioritized TODO covering all three
     sources instead of three disconnected lists.

3. **#187 -- Fix triaged real bugs from the audit.** (blocked by
   #186) Follow the established `BUG_PATTERN_AUDIT` round workflow:
   fix, add a regression test that would have caught it, verify
   against the full example corpus (exact file-set diff), full
   `cargo test`, mark closed in BOTH `docs/TODO_CURRENT.md` and the
   localfuzz worktree's `docs/TODO_LOCAL_STAGING.md`.

4. **#188 -- Scope + design a vani-LANGUAGE test framework (TDD for
   vani programs).** Confirmed scope: a `#[test]`-style feature for
   programs written IN vani -- test discovery, a `vanic test`
   runner, pass/fail reporting, likely building on existing
   `assert`/`prove` builtins. Ships FIRST (internal tooling is
   separate, see #191). Needs its own `EnterPlanMode` design pass
   before implementation -- new keyword/attribute surface, discovery
   mechanism, runner architecture, codegen for both C and LLVM
   backends -- matching how the async/Executor work got its own
   dedicated plan.

5. **#189 -- Implement the vani test harness per the agreed scope.**
   (blocked by #188) Ship with worked examples, tutorial coverage,
   full verification discipline.

6. **#191 -- Improve internal compiler-testing tooling.** (blocked
   by #189, i.e. sequenced AFTER the language-level framework)
   Property-based testing, codegen snapshot diffing, or similar, on
   top of the existing 2966 lib tests / 259 e2e tests / corpus check
   / localfuzz. No new vani-language surface.

7. **#190 -- Final documentation/tutorial consistency sweep.**
   (blocked by #185, #187, #189, #191) `docs/TODO_CURRENT.md`,
   `docs/v1_limitations.md`, relevant tutorial pages, cross-
   references, `SUMMARY.md` nav, glossary, and a check for any
   newly-stale claims introduced by the other four items. Also
   folds in per-item doc updates that should already have happened
   alongside 1-6, not been deferred to this step alone.

## Dependency graph

```
185 (Phase F)                              ─┐
186 (audit) ──> 187 (fix)                   │
188 (scope test fw) ──> 189 (build) ──> 191 ┤──> 190 (final docs sweep)
```

185 and 186 can start independently; 188 can also start independently
(it's a design/scope task, not blocked on the audit or Phase F).
