# BUG_PATTERN_AUDIT_TODO_13.md

Filed 2026-08-14, as part of task #186 ("Refresh localfuzz + audit v1
limitations, produce one prioritized fix list") -- the combined
successor to the localfuzz digest, `docs/BUG_PATTERN_AUDIT_TODO_9.md`'s
leftover candidates, and `docs/v1_limitations.md`'s open items, per
the user's explicit request to merge those three sources into ONE
prioritized list instead of three disconnected ones. This is item (b)
in the user's post-Phase-F plan; #187 ("fix triaged real bugs from
this audit") is the next task, blocked on nothing further -- this
document IS the triage.

**Process**: refreshed the localfuzz worktree (`refresh.sh`, merged 7
commits including Phase F, rebuilt --release), ran `digest.py --all`
for a full re-scan (341 findings -> 41 signatures), then did NOT trust
the digest's keyword-match heuristics -- re-ran a representative
sample from every signature cluster against the freshly-built binary
directly (both backends), reading full stdout+stderr, not just exit
codes. This is the same "always rebuild vs current main before
trusting localfuzz .vani repros" discipline as every prior round.

---

## Priority 1 (HIGH) -- LLVM backend silently drops the runtime-trap
## diagnostic message in some outlined/transformed codegen contexts

**Status: FIXED 2026-08-14 as BUG-192 -- see `docs/TODO_CURRENT.md`.**
Turned out to be TWO independent bugs sharing one symptom, not one
bug with two triggers: the "outlined/transformed codegen" framing
below was this document's own working hypothesis and did NOT hold up
under root-causing -- `TypedStmt::Assert`'s bug was backend-wide
(any tree-LLVM program with a message-less assert, `await` just
happened to be the first repro to force tree-LLVM), and `parallel
for`'s bug was a wholly separate, pre-existing trap site that was
simply never wired to a message in the first place. Left the
original write-up below unedited for the historical record of what
was actually investigated; see `docs/TODO_CURRENT.md`'s BUG-192
entry for the real root causes and the fix.

`assert`/overflow/bounds traps correctly exit(3) on the LLVM backend
(matching BUG-115's established convention -- see below), but in at
least two specific contexts the diagnostic MESSAGE that should
precede the exit is silently missing -- the process still exits 3
(so a caller checking the exit code alone sees nothing wrong), but
stdout AND stderr are both completely empty. The C backend prints
its message correctly in the same programs.

**Minimal repro (isolated from a 35-finding localfuzz cluster tagged
"C-ASSERT-FAIL" divergence, `findings/20260803-121804-*` and 34
siblings)**:

```vani
async fn delay(ms: i64, v: i64) -> i64 {
  sleep_ms(ms);
  return v;
}

fn main() -> i64 {
  let a: i64 = await(delay(5, 42));
  assert a == 99;   // fails -- a is 42
  return 0;
}
```

`vanic run` (LLVM): exits 3, **prints nothing**.
`vanic run --backend=c`: exits 3, prints
`assertion failed: (a == 99)` with file/line.

Confirmed this is specific to `await(...)` and not "any preceding
function call": a plain `let a = helper(41); assert a == 99;` (no
async) prints correctly on both backends. A single `await(...)` call
before the assert is sufficient to trigger the loss.

**A second, independent trigger for the same symptom**, found while
re-verifying a different cluster (`findings/20260807-083513-*`,
originally digest-tagged "LLVM-INTERNAL-CRASH" but no longer an
actual crash on the fresh binary):

```vani
fn main() -> i64 {
  let product: i64 = 1;
  let xs: [i64; 4] = [1, 2, 3, 4];
  parallel for i from -9223372036854775808 to 4
  reduce product with *;
  {
    product = product * xs[i];
  }
  print product;
  return 0;
}
```

(The `-9223372036854775808` is a fuzzer-injected `i64::MIN` boundary
literal -- see Priority 4 below on why this shape recurs so often --
but the RESULT is a genuine overflow trap inside `parallel for`'s
outlined reduction-worker codegen, and the LLVM backend prints
nothing for it, same as the async case.)

**Working hypothesis, not yet verified**: both triggers involve code
that runs inside a codegen path with its OWN synthesized `FnCtx`
(the async transform's generated `__poll_<fn>`/state-machine body;
`parallel for`'s outlined per-thread worker body) rather than the
caller's original function body directly. This project has hit the
same general bug SHAPE three times already this month in unrelated
features (see memory `project_vani_bug188_189_190_and_mutref_fix`:
"3rd instance this month of the ctx.current_block-not-updated LLVM
bug class") -- a child `FnCtx` created for an outlined/transformed
body not correctly propagating some piece of codegen state back (or
losing track of "which block is the trap's `dprintf` call actually
being emitted into") whenever that outlined body itself contains
another trap site. Worth checking `emit_task_via_pthread`/
`emit_parallel_for_via_gomp`'s trap-emission paths and the async
`__poll_<fn>` synthesizer for the same class of gap BEFORE assuming
it's a brand-new, unrelated bug -- but confirm with a debugger/IR
dump rather than assuming; this is a hypothesis, not a diagnosis.

**Suggested next steps for #187**: dump the LLVM IR for the minimal
async repro above (`vanic emit --backend=llvm`) and check whether the
`dprintf`/message-print call for the assert's failure block is
present in the IR at all, and if so, whether it's in a block that's
actually reachable from the `await`-transformed control flow. Add a
regression test once root-caused. Candidate bug number: **BUG-192**
(next free per `docs/TODO_CURRENT.md`).

---

## Priority 2 (MEDIUM) -- general `Str`-parameter `OwnedStr`
## auto-borrow leak (carried over from BUG_PATTERN_AUDIT_TODO_9.md)

**Status: confirmed real (2026-08-10), deliberately left unscoped,
still open. Not re-verified this round (no new evidence either way),
carried forward as-is.**

```vani
fn takes_str(s: Str) -> i64 { return len(s) as i64; }
fn main() -> i64 {
  let n: i64 = takes_str(i64_to_str(12345));   // leaks
  return n;
}
```

Any fresh, never-bound `OwnedStr` expression passed as a `Str`-typed
argument to an ordinary user-defined function leaks (the callee's
implicit borrow cast never frees the temporary). BUG-159/160/161
fixed this for specific builtin call sites (`hashmap_insert`/`_get`/
`_contains_key`/`_remove`, `trie_insert`/`_contains`/
`_starts_with`/`_delete`) but explicitly did NOT extend it to
ordinary function calls -- the general case touches every
function-call-argument codegen site in both backends, a much larger
blast radius that `docs/BUG_PATTERN_AUDIT_TODO_9.md` deferred pending
its own scoping discussion. Still true; this document does not
change that recommendation. If #187 picks this up, treat it as a
design/scoping task first (matching how the async/Executor and test-
framework work each got a dedicated `EnterPlanMode` pass this month),
not a quick pickup.

---

## Priority 3 (LOW, tooling) -- localfuzz digest can't distinguish
## "matched, already fixed" from "matched, explicitly accepted as
## non-bug"

**Status: not a compiler bug -- a `tools/localfuzz/digest.py`
usability gap, found while triaging this round's clusters.**

~103 of this round's 341 findings (48+35+10+7+2+1 signature
occurrences, all variants of "C backend `abort()`s with SIGABRT=134,
LLVM backend `exit(3)`s cleanly, for the identical overflow/division/
bounds-check message") are digest-tagged "possible match: BUG-177"
and flagged for re-verification. Re-verified a representative sample
directly against the freshly-rebuilt binary: the behavior is real,
reproducible, and UNCHANGED from before -- but it is not a live bug.
**`docs/TODO_CURRENT.md`'s own BUG-177 write-up (2026-08-11) already
triaged this EXACT pattern (15/18 findings in that round) and
explicitly decided to leave it as-is**: "Consistent across BOTH C
backends (tree-C and SSA-C), so treated as accepted/intentional
design, not a bug." (LLVM's own `abort()`-to-`exit(3)` conversion for
these traps was itself a deliberate fix, BUG-115 -- see
`src/ssa_backend_llvm.rs` around line 410 -- but never extended to
either C backend, and BUG-177's triage explicitly chose not to
extend it either, on the reasoning that both C backends already agree
with EACH OTHER even though they disagree with LLVM.)

Every future digest run will keep re-flagging this same ~30% of
findings as "needs re-verification" until either (a) someone reverses
the 2026-08-11 decision and actually unifies the exit convention, or
(b) `digest.py` gains a way to record "matched AND already explicitly
triaged as accepted, not a bug" separately from "matched, might be
fixed, please re-check" -- right now both cases render identically
("possible match: BUG-N"). Suggest (b): a small persistent
allow-list in `digest.py` (e.g. a `KNOWN_ACCEPTED` dict of signature
-> BUG-N-and-reason) that downgrades matching clusters to a single
collapsed "accepted, see BUG-177" line instead of spelling out every
finding directory, so future sessions' attention goes to the signal
instead of re-deriving this same conclusion. Cheap, not urgent --
doesn't affect compiler correctness, just triage ergonomics.

---

## Priority 4 (LOW, tooling) -- fuzzer mutator over-produces
## "correctly very slow" programs, not bugs

**Status: not a compiler bug -- a `tools/localfuzz/harness.py`
mutator-quality observation.**

The single largest signature cluster this round (83 of 341 findings,
~24%) is "both backends time out" with no stdout/stderr at all.
Sampled 6 of the 83 directly: every one has a fuzzer-mutated integer
literal at (or near) `i64::MIN`/`i64::MAX` sitting in a `sleep_ms(...)`
argument or a `for`/`parallel for` loop bound (`daga -9223372036854775808 zuwa n`,
`sleep_ms(9223372036854775807)`, etc.). These are not broken programs
-- they are asking for ~9.2 quintillion loop iterations or a
multi-million-year sleep, and correctly, deterministically take
"forever" on any backend. `RUN_TIMEOUT=20` (`harness.py`) simply can't
tell "genuinely hung" apart from "genuinely doing 2^63 units of
correctly-specified work" -- both look identical from outside.

One related, already-covered case worth noting explicitly: the same
`i64::MAX` loop-bound pattern, when it happens to land in a
CI-friendly position, produces a backend-SPEED divergence instead of
a double-timeout -- `findings/20260813-144531-*`'s `while i <
9223372036854775807 { i = i + 1; }` finishes almost instantly on the
C backend (gcc's `-O2` recognizes the closed-form trip count and
optimizes the loop away) but times out on `vanic run`'s LLVM path
(the JIT skips `opt`, so it actually executes all ~9.2e18 iterations
one at a time) -- this is `docs/v1_limitations.md`'s existing **L27**
("`vanic run`'s LLVM JIT path skips the `opt` optimizer"), not a new
gap, just a fuzzer input that happens to make L27 visible via a
timeout instead of a speed difference.

Suggest biasing the mutator away from inserting extreme-boundary
literals into `sleep_ms`'s argument position and loop-bound positions
specifically (or clamping mutated values used in those positions to
something that resolves within a few seconds even in the worst case)
-- this alone would likely cut the findings-needing-triage count by
roughly a quarter per run, with no loss of real bug-finding power
(extreme literals are still worth fuzzing in ARITHMETIC/comparison
positions, where they correctly stress overflow/bounds traps instead
of just burning wall-clock time).

---

## Confirmed FIXED / stale (no action needed) -- for completeness

Re-verified directly against the fresh binary, not just via keyword
match:

- **BUG-76 cluster** (`LLI-PARSE-ERROR: integer constant must have
  integer type`, 12+1 findings) -- both backends now succeed with
  matching output. Confirmed fixed.
- **BUG-88 cluster** (`LLI-PARSE-ERROR: expected '=' after
  instruction name`, 7 findings) -- both backends now succeed with
  matching output. Confirmed fixed.
- **The 14-finding `C-COMPILE-FAIL`/`llvm.rc=0` cluster** -- both
  backends now succeed with identical output (`2`). Confirmed fixed
  (stale repro, pre-dates whatever landed the fix; not re-derived
  further since it's already resolved).
- **The original 10-finding "both rc=0" cluster** -- re-ran the
  digest's own example: now diverges via the SAME accepted
  abort()-vs-exit(3) pattern as Priority 3 above (both backends now
  correctly detect and report "index out of bounds" where they
  previously silently produced output) -- i.e. this cluster's
  original bug (if it was ever a real wrong-answer bug and not
  already this same pattern) has been superseded by unrelated
  overflow/bounds-detection work landing since. Not actionable.
- **BUG_PATTERN_AUDIT_TODO_9.md's Category 2 and Category 3** --
  already fully resolved per that document's own status header, not
  re-derived here.

---

## `docs/v1_limitations.md` -- re-confirmed still-open items

Not re-derived from scratch (see `docs/TODO_NEXT_SESSIONS.md`'s own
prior-session analysis for the reasoning) -- spot-confirmed these are
still accurately described and none have silently become stale:

- **L5, L6, L13, L14** -- by-design, no action expected.
- **L10 (macOS), L24, L25 (Windows)** -- blocked on hardware not
  available in this environment; implemented from documented
  contracts, unverified on real hardware (same caveat this session's
  own `cancel <name>;` Windows/macOS paths carry -- see
  `docs/TODO_CURRENT.md`'s Phase D writeup).
- **L27** -- unchanged, and now has a second concrete real-world
  trigger (Priority 4 above) beyond its original discovery context.
- **L29** -- `step`/`by` (stride other than 1) still unimplemented;
  unchanged since 2026-08-13.
- **L30** -- new this session (Phase F), see
  `docs/TODO_CURRENT.md`'s Phase F write-up; not actionable within
  #187's scope without its own design pass (new builtin surface on
  both backends).

---

## Summary for #187

In priority order:

1. ~~**Fix** (or at minimum root-cause and file as its own tracked bug
   number, BUG-192): the LLVM silent-trap-message bug (Priority 1).~~
   **DONE 2026-08-14** -- see `docs/TODO_CURRENT.md`'s BUG-192 entry.
2. **Scope, don't quick-fix**: the general `Str`-param `OwnedStr` leak
   (Priority 2) -- needs its own `EnterPlanMode` pass given the
   blast radius, same as this document's own predecessor concluded.
3. **Optional, cheap**: the two `tools/localfuzz` tooling
   improvements (Priorities 3 and 4) -- neither touches the compiler,
   both reduce future triage noise; fine to defer to whoever next
   works on localfuzz specifically rather than blocking #187.
4. **No action**: everything under "Confirmed FIXED / stale" and
   "`v1_limitations.md` -- re-confirmed still-open items" above --
   listed for completeness so a future session doesn't re-derive the
   same conclusions from scratch.
