# BUG_PATTERN_AUDIT_TODO_12.md

Filed 2026-08-12 at the user's suggestion, after BUG-181 and BUG-182
landed back-to-back in the same localfuzz triage session — both real,
previously-unknown SMT-elision soundness bugs found within one hour of
each other, both in the same general area (bounds-check elision
interacting with loops), both affecting both SSA-C and SSA-LLVM (and
their tree-backend siblings) identically since the bug lives in the
shared checker.rs pass both backends consume.

**Status update 2026-08-12, same day**: item 1 below ("there may be
more in match-arm merging") paid off immediately — the targeted audit
found and fixed **BUG-183**, the identical stale-fact shape at the
`if`/`else` merge, the `if let` merge, the `select`-statement
loop-desugar exit, and (a worse variant — no restore at all) `while
let`. It also surfaced a SECOND, independent leak source at two of
those four sites: `env`'s own `VarInfo.constant` bookkeeping, read
directly by `current_smt_facts` on every query, bypassing the
`smt_facts` Vec entirely — fixing the Vec alone was not sufficient at
the `if let` site until `clear_constants_for` was added there too.
Full writeup: `docs/TODO_CURRENT.md`'s BUG-183 entry. Items 2–4 below
are still open for a follow-up round.

**Status update 2026-08-16**: item 1's own loop/branch-merge restore
sites are confirmed fully closed (all 10 `*smt_facts = pre_facts`
sites in `checker.rs` carry a BUG-181/182/183 fix comment). Item 2's
follow-up audit found and fixed **BUG-198**: no code path ever
invalidated `smt_facts` for a Vec mutated via a `mut ref` builtin
argument (`pop(mut ref zs)`, `push(mut ref zs, v)`, ...) used as a
statement's RHS — confirmed exploitable (a stale post-push length
fact survived a `pop`, letting the elision pass prove a now-OOB index
safe, producing a real unguarded `zs.data[2]` on the C backend that
silently read stale-but-still-allocated memory instead of trapping).
Full writeup: `docs/TODO_CURRENT.md`'s BUG-198 entry. The "match-arm
merging" and `try`/`?` phrasing in item 1 above turned out to be N/A
(this language has no `match` statement, and `try`/`?` desugars to
plain `Stmt::If`/`Return`/`Let` before `check_one_stmt` ever runs, so
both already flow through the already-audited paths). Item 4 (Vec/str
length-specific facts) is now covered by BUG-198's fix. **Still open**:
async suspend-point (`__poll_*`) state-machine transforms — not yet
tested with a targeted repro combining a suspend point inside a loop
with a bounds-check-eligible access.

**Status update 2026-08-16, later same day**: the async suspend-point
item was picked up. Bounds/overflow elision inside an async loop with a
real suspend point (`echo_loop.vani`'s exact shape) is confirmed safely
conservative — the transform's synthesized `while true { if
state_tag==N {...} ... }` wrapper means `inside_loop` is true for the
whole poll-fn body, so BUG-127/181's existing blanket guard already
covers every state segment. While chasing an `(assert false)` anomaly
noticed in `--smt-debug` output for these repros, found and fixed a
DIFFERENT, more severe bug: **BUG-199**, `Stmt::FieldAssign` never
invalidated `struct_literal_fields`-derived SMT facts (the sibling gap
to BUG-198, for struct fields instead of Vec mutation) — confirmed to
let the checker `prove` a demonstrably FALSE claim about a field's
value right after reassignment (`c.n = 5; prove c.n == 0;` was silently
accepted). Full writeup: `docs/TODO_CURRENT.md`'s BUG-199 entry. The
`(assert false)` anomaly itself persists in every `__poll_*` function's
first-branch queries even after the BUG-199 fix — traced its span to
the function's own `fn_name_span` (used pervasively by
`try_v31_transform`'s synthesized code) but not fully root-caused;
empirically confirmed NOT exploitable across every case tested this
round (the contaminated query is independently, trivially true
regardless), flagged for a dedicated follow-up round with better
tooling rather than continued span-matching guesswork.

## The pattern, in one sentence

`checker.rs`'s SMT-based elision passes (overflow-guard elision AND
bounds-check elision) all read from `smt_facts: &mut Vec<Expr>`, a
running list of "facts assumed true at this program point" that gets
mutated as the checker walks the typed AST — and every time a NEW kind
of loop/branch-merge site has needed to reason about `smt_facts`
across the join point, it has needed its own hand-written logic for
"which facts survive, which get dropped, which get invalidated." Three
confirmed bugs so far, all from this exact class:

- **BUG-127** (2026-08-07): a stale fact from before a `while` loop
  (`n == 0`) survived INSIDE the loop body and let the
  arithmetic-overflow elision "prove" `n + i64::MIN` never overflows,
  when it does on the second iteration. Fixed with an
  `if inside_loop { return; }` guard on the Binary-arithmetic arm of
  `try_elide_bounds_in_typed_expr`.
- **BUG-181** (2026-08-12): the exact same stale-fact problem, but in
  the Index/bounds-elision arm of the SAME function, which a comment
  on the BUG-127 fix explicitly (and incorrectly) claimed was immune.
  Elided a `Vec` bounds check on a `while`-loop-mutated index,
  producing an unconditional out-of-bounds read — SIGSEGV on the C
  backend, silent unbounded OOB heap reads on LLVM. Fixed with the
  identical `if inside_loop { return; }` guard, now on the Index arm
  too.
- **BUG-182** (2026-08-12): found ~1 hour after BUG-181, in the SAME
  localfuzz batch. A DIFFERENT stale-fact bug in the SAME general
  area: after a loop exits, `*smt_facts = pre_facts;` restores the
  ENTIRE pre-loop fact snapshot wholesale, including facts about any
  variable the loop body reassigned (e.g. `len(xs) == 1` from before a
  loop that grows `xs` via `push`). Combined with the loop's own
  freshly-added, CORRECT post-loop facts (invariant + exit condition),
  this produces an internally CONTRADICTORY fact set, from which the
  SMT solver can "prove" anything — confirmed to elide a bounds check
  on a wildly out-of-range constant index (`xs[-1]`, `xs[i64::MAX]`)
  used AFTER a loop, not even inside one. Fixed by dropping every fact
  mentioning a loop-mutated variable right after the `pre_facts`
  restore, at all four sites sharing this pattern (`Stmt::While`,
  `Stmt::For`, `Stmt::ForIter`, `check_while_loop_as_let_init`).

Three bugs, one root theme: **`smt_facts` correctness across a loop
boundary is subtle, hand-maintained per call site, and has now been
wrong three separate times in three separate ways.** There is no
single source of truth for "is this fact still valid here" — every
new code path that touches `smt_facts` near a loop or branch merge is
its own hand-proof of soundness, and hand-proofs have had a
demonstrated ~33% failure rate here (2 bugs found out of roughly 6-8
`smt_facts`-mutating sites audited so far, informally).

## Where to look for round 12

Grep `smt_facts` in `src/checker.rs` (currently ~150+ call sites) and
specifically audit every site that does one of:

1. `*smt_facts = pre_facts` or `*smt_facts = pre_facts.clone()` (loop
   exits, if/else merges, select-loop desugars) — verify each one
   correctly scopes what survives vs. what needs dropping for the
   construct it belongs to. BUG-182 fixed 4 of these; there may be
   more in `match`-arm merging, `try`/`?` desugaring, or async
   suspend-point state-machine transforms that weren't audited this
   round.
2. `drop_facts_mentioning` call sites and their GUARDS (`if
   loops.is_empty() { ... }` and similar) — confirm the guard
   condition actually matches the soundness argument in its own
   comment. BUG-127's fix comment made an incorrect blanket claim
   about a sibling code path (Index elision) that went unverified for
   5 days until BUG-181 disproved it by direct construction — treat
   every "X is unaffected by this" comment as a hypothesis to
   re-verify with a real repro, not a fact.
3. `verify_loop_invariants_with_havoc` and the whole
   loop-invariant-preservation machinery — this is the ONE place that
   already does real per-variable havoc-and-reprove reasoning
   (correctly), so it's a good template for what the OTHER
   `smt_facts` call sites should probably be doing instead of ad hoc
   drop/restore logic.
4. Any elision pass that reasons about a Vec/array's LENGTH
   specifically (not just an index value) — BUG-182 was a length
   fact, not an index fact; there may be sibling bugs in `str` length
   facts, `Vec` capacity-adjacent reasoning, or `OwnedStr`-related
   length tracking that share the same "stale length fact from before
   a mutating loop" shape.

## Suggested method

Given BUG-181/182 were both found via ordinary localfuzz mutation
(not a targeted audit), a *targeted* pass — deliberately writing loops
that mutate a Vec/counter/length, sprinkling `--smt-debug` output
review at each provably-safe-looking elision site, and specifically
checking for CONTRADICTORY fact sets (not just "does this specific
goal look wrong") — is likely to surface more of these faster than
waiting for the fuzzer to stumble into the next one. A contradictory
fact set is detectable programmatically too: before trusting any
`Verdict::Proven` result from `try_elide_bounds_in_typed_expr` or its
siblings, a debug-only assertion that `smt_facts` itself is
satisfiable (not just that the negated goal is unsat) would catch this
entire bug class at the SOURCE rather than one elided check at a time
— worth prototyping as a `--smt-debug`-gated sanity check even if it's
too expensive to run unconditionally in every build.
