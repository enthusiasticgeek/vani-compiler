# vāṇी — Bug-pattern audit, round 8

**STATUS (2026-08-10): 6 real bugs FIXED (BUG-153, BUG-154 -- general
and severe -- BUG-155/156/157, and BUG-158, all fixed in same-day-or-
next-day follow-up sessions at direct request), PLUS the async
`__poll_*` instance of BUG-157/158's own root issue, fixed in the
async transform itself (not a checker rejection this time -- see
"Async transform fix" section below). Every OwnedStr/Str
ownership-escape finding from this round is now closed except one
separate, still-undiagnosed leak (`hashmap_strstr.vani` /
`hashmap_strv.vani` -- NOT async-related, likely HashMap-internal,
mislabeled as part of the async cluster in round 8's first pass).
2 false positives and 2 low-severity UB findings triaged, not fixed
(see below for why).**

Directly requested: "pick a round-8 theme... my worry is always
memory and leaks - this is absolutely uncompromised requirement for
safety critical application. not sure if you have asan and/or
valgrind or similar tools check for all examples or libs for any
potential leaks." Confirmed no such systematic sweep existed --
`valgrind` had only ever been used manually, one program at a time,
explicitly "not wired into CI" per an earlier session's own note in
`docs/TESTING_MATRIX_TODO.md`. This round built one.

## Methodology

Every `.vani` program that passes `vanic check` was compiled to C
(`vanic emit-c`) and built with `gcc -fsanitize=address,leak,undefined
-fno-sanitize-recover=all -O1 -g -pthread -fopenmp` (matching the real
`-pthread`/`-lm`/`-fopenmp` flags `vanic run --backend=c` itself uses,
confirmed by reading `main.rs`'s compile invocation directly), then
run with a short timeout under `ASAN_OPTIONS=detect_leaks=1`. Two
program sources: all 1040 files under `examples/` (recursively), plus
108 unique inline `.vani` programs extracted via regex from
`tests/run_end_to_end.rs`'s `write_tmp_vani(...)` calls -- covering
both the standalone example corpus AND the compiler's own runtime
test suite, per the explicit ask to check "all around 2800+ test
cases" (most of `src/lib.rs`'s ~2655 `#[test]`s are pure compile-
checks that never execute a subprocess and so have nothing to leak;
the 242 in `run_end_to_end.rs` -- plus a handful more across other
`tests/*.rs` files that also spawn real subprocesses, all of which
run standalone `.vani` files already covered by the `examples/` sweep
-- are the ones that matter for this kind of check).

1148 total programs; 1008 compiled and ran clean, 12 flagged on the
first pass (before any fixes), down to 9 after BUG-153/154 were
fixed and the sweep re-run against the fixed binary. ASan can't
instrument already-emitted LLVM IR after the fact, so every flagged
finding's LLVM behavior was separately checked with `valgrind
--leak-check=full` on a native `vanic build` AOT binary of the exact
same repro.

Full sweep driver + raw results kept in the session scratchpad, not
committed (not durable project artifacts -- the point was the
findings, now written up here and in `docs/TODO_CURRENT.md`, not the
one-off tooling). Worth reconstructing as a real, checked-in,
periodically-run script in a future session -- see "Worth doing
next" below.

## Fixed this round

**BUG-153**: a mixed-payload enum (variants with different payload
types) never freed a `Box<T>`-payload variant at scope exit --
`emit_enum_value_drop`'s mixed-payload branch only ever handled
`OwnedStr`/`Vec<T>` payloads; a pre-existing code comment even
admitted the gap ("BUG-97 note: ... remain a deferred gap") without
it ever actually being closed. Confirmed a real 16-byte leak. Fixed
by factoring the ALREADY-CORRECT single-payload path's `Box<T>`
handling into a shared `box_payload_free_expr` helper and calling it
from both branches. LLVM confirmed unaffected for this repro shape
(valgrind clean before and after) -- the parallel gap visibly exists
in tree-LLVM's own mixed-payload Drop codegen on direct inspection,
but wasn't reachable by this specific repro; not chased further given
time, flagged below as a genuine open question.

**BUG-154**: far more general and severe. `Env::scopes` is
`Vec<BTreeMap<String, VarInfo>>` -- every scope-exit drop site
iterated bindings in ALPHABETICAL ORDER BY NAME, not reverse
declaration order, contradicting the "Rust RAII convention" this
codebase's comments claim everywhere (and correctly implement for
struct fields, which use an ordered `Vec`, not a name-keyed map).
Confirmed via a genuine, reproducible heap-use-after-free: a
`ReadGuard` borrowed from a `RwLock` stored in a `Vec` was used to
unlock AFTER the `Vec` (and the `RwLock`'s storage) had already been
freed, purely because `"locks" < "rg"` alphabetically. This is a
NAME-DEPENDENT bug -- only reproduces when a dependent binding's name
happens to sort before what it depends on -- exactly the kind of
thing a systematic sweep across many differently-named real programs
finds and a hand-written test suite is unlikely to stumble onto by
chance. Fixed by sorting every bulk drop-list by `VarInfo.decl_span`
(an existing field, reused directly -- no new field needed) before
emission, at all four sites found: `emit_current_scope_drops`, the
return-statement's two drop lists (regular bindings + affine
closures, sorted independently -- see the residual cross-group-
ordering limitation noted in `docs/TODO_CURRENT.md`), and
`emit_drops_through_loop` (break/continue).

Both fixes verified on both backends (ASan on C, valgrind on native
LLVM AOT builds), full regression tests added (2 `src/lib.rs`
compile-checks, 2 `tests/run_end_to_end.rs` real-subprocess tests on
both backends), full `cargo test --release` clean, `vanic check
examples` unchanged at the 78-error baseline. Full writeup in
`docs/TODO_CURRENT.md`'s BUG-153/154 sections.

## Triaged, not compiler bugs

- **`examples/language/english/bare_metal.vani`** -- SEGV under ASan.
  Reads a hardcoded STM32 Cortex-M GPIO peripheral address
  (`mmio_read_u32(0x40020014)`) in a `Reset_Handler` meant for real
  embedded hardware or QEMU, not native userspace execution.
  Correctly crashes when run the "wrong" way; not a bug, a
  methodology false positive of running a bare-metal example
  natively.
- **`examples/edge_cases/mix_conc_channel_send_recv.vani`** -- flagged
  `exit code 99`. The program's own, entirely legitimate return value
  (`channel_recv` returns the sent value, `99`) collided with the
  sweep harness's own `ASAN_OPTIONS exitcode=99` convention for
  signaling "ASan detected a real problem." A sweep-methodology
  artifact, not a compiler bug -- worth picking a less collision-
  prone sentinel exit code in any future incarnation of this sweep.

## Triaged, low severity, not fixed

- **`examples/language/english/loop_carried_overflow_not_elided.vani`**
  -- UBSan: "negation of -9223372036854775808 cannot be represented."
  The generated C represents the i64::MIN literal as
  `-(int64_t)9223372036854775808LL` -- technically UB by the strict
  C standard (both the huge-unsigned-to-int64_t conversion and the
  subsequent negation of an unrepresentable value), but every real
  compiler/CPU implements this identically to the intended two's-
  complement value via constant folding. The program's actual
  overflow-CHECK logic (a few lines later, `__builtin_add_overflow`
  + `abort()`) is present and correct -- this is purely about how the
  literal itself gets spelled in C source, not a missing runtime
  check.
- **`examples/language/english/sort_large_block_partition.vani`** --
  UBSan: "left shift of 108595223277980261 by 17 places cannot be
  represented." Same class -- vāṇी's `<<` is meant to be a raw,
  always-well-defined bitwise operation, but the C backend emits it
  as C's native `<<` on a signed `int64_t`, which has stricter
  (letter-of-the-standard) UB rules when the shifted result doesn't
  fit. Universally implemented as a single machine SHL instruction in
  practice; no observed wrong output.
- Neither produced any observably incorrect behavior in this sweep.
  A proper fix (emitting `INT64_MIN`/`unsigned`-cast-based literal
  and shift forms in the generated C) is worth doing for standards-
  conformance and portability to more exotic ASan/UBSan-style tooling
  or genuinely non-two's-complement hardware, but isn't urgent -- left
  undone this round given the two real, higher-severity bugs already
  found and fixed ate the round's time budget.

## Fixed in a same-day follow-up (BUG-155 + BUG-156)

**Closure returned from a factory function leaks its env struct** --
fixed. Turned out to be TWO bugs, not one:
```vani
fn make_greeter(name: OwnedStr) -> Closure(i64) -> i64 {
  let g = fn(x: i64) -> i64 { print "hello,", name, x; return 0; };
  return g;
}
fn main() -> i64 {
  let say_hi: Closure(i64) -> i64 = make_greeter("alice" + "");
  say_hi(5);
  return 0;
}
```
BUG-155: `say_hi` (bound via a function call, not a direct closure
literal) was never registered in `CLOSURE_AFF_REGISTRY` at all --
fixed with a new `FN_RETURN_VAR_NAME` registry, populated once
program-wide right after `lambda_lift_program`, propagating a
callee's own affine-closure registration to the caller's binding.
BUG-156, found while verifying BUG-155's fix didn't actually stop the
leak: an EVEN MORE GENERAL gap underneath -- `say_hi(5);` (a
discarded call, no `let`) never freed the env struct at the call site
AT ALL, for ANY affine closure regardless of provenance (confirmed
with a minimal repro: a closure constructed directly in `main`, never
returned from anywhere, called once, leaked identically).
`TypedStmt::Discard`'s C emission had no handling for calling an
affine closure -- that logic only ever existed in `TypedStmt::Let`'s
emission (`let r = f(args);`). Fixed by adding the missing intercept
to Discard's emission too. LLVM confirmed unaffected by either gap
(valgrind clean, no LLVM changes needed). Full writeup in
`docs/TODO_CURRENT.md`'s BUG-155/156 section; 4 regression tests
added, verified via ASan + valgrind on both backends.

## Fixed in a second same-day follow-up (BUG-157) -- general case; async cluster deliberately left open

**Not async-specific after all.** The four originally-flagged files
(`echo_p3_locals_stress.vani`, `echo_p3b_str_local.vani`,
`hashmap_strstr.vani`, `hashmap_strv.vani`) all happen to be async
examples, which is what round 8's first pass assumed was the common
thread. It isn't. Minimal, fully general, non-async repro:
```vani
fn main() -> i64 {
  let label: Str = i64_to_str(42);
  print label;
  return 0;
}
```
Root cause: `can_assign`'s `OwnedStr -> Str` auto-borrow comment says
"the OwnedStr binding stays live; its drop fires at the original
scope's end" -- true when the RHS is already a `Var` (confirmed
leak-free separately), false when it's a fresh, never-bound
`OwnedStr`-producing expression like `i64_to_str(42)`: nothing ever
owns that allocation, so `coerce_checked`'s generic `cast_expr`
fallback just relabeled it as `Str` with no free anywhere.

**Fixed** (general case only) by synthesizing a real sibling `let`
for the `OwnedStr` result at the one `checker.rs` call site
(`Stmt::Let`'s processing) that has access to both the output
statement list and `env` -- registered exactly like an ordinary
binding, so it gets a genuine scope-exit Drop, then the original name
becomes a `Str` view of that temp. Verified via ASan on the minimal
repro (clean) and via the full corpus-wide sweep re-run.

**The async cluster (all 4 originally-flagged files) is deliberately
LEFT LEAKING, not fixed** -- and this is the most important finding
of this follow-up, not a footnote. The first version of this fix
applied unconditionally and turned the leak into a confirmed
**heap-use-after-free** inside `__poll_*` functions (direct ASan
repro on both `echo_p3_locals_stress.vani` and
`echo_p3b_str_local.vani`). Cause: `parser.rs`'s `try_v31_transform`
(the async state-machine desugar) runs at PARSE time, before this
checker pass ever sees the function body, and hoists the user's own
named `let`s into a persistent per-coroutine state struct so values
survive across separate `poll()` calls (separate C function
invocations, each a fresh call frame). A synthetic temp introduced by
the checker AFTER that transform already ran is invisible to its
hoisting pass -- it stays an ordinary per-call local, freed at the
end of THIS `poll()` call, while the (correctly hoisted) `Str` view
gets read on a LATER call, after the buffer is already gone. The fix
now explicitly excludes any function whose name starts with
`__poll_`, keeping the original (leaky but safe) codegen there. A
real fix needs the async transform itself to learn about
checker-synthesized temps -- out of scope here. Confirmed via
`async_poll_owned_str_narrowing_keeps_original_leaky_but_safe_form`
in `src/lib.rs` that the exclusion guard holds.

This is exactly the risk flagged when this leak was first left
unfixed ("a rushed fix... risks being WORSE than the current narrow
leak, not better") -- and it materialized on the very first attempt.
Lesson for next time a fix touches a pervasive coercion/lowering
path: test the async/coroutine corpus specifically (not just the
common case) before calling any such fix done, even when the common
case looks obviously correct.

2 regression tests added to `src/lib.rs`, 1 to
`tests/run_end_to_end.rs` (real-subprocess, both backends). Full
writeup in `docs/TODO_CURRENT.md`'s BUG-157 section.

## IMPORTANT UPDATE (2026-08-09, same day): the "async transform" framing above was wrong -- this is a GENERAL, non-async, pre-existing heap-use-after-free

While updating the tutorial to explain the `__poll_` caveat, I tried
to describe a source-level workaround for users hitting it (split
`let label: Str = i64_to_str(mode);` into two `let`s, matching the
already-confirmed-safe pattern). **That "workaround" also
use-after-frees.** Testing it revealed the real bug is not in the
async transform at all -- it's a general gap in the checker's
`OwnedStr -> Str` auto-borrow that the async transform's own codegen
happens to trigger, but which is fully reachable from ordinary,
completely non-async code:

```vani
struct Holder { s: Str, n: i64 }
fn main() -> i64 {
  let h: Holder = Holder { s: "", n: 0 };
  {
    let owned: OwnedStr = i64_to_str(99);
    h.s = owned;      // FieldAssign: OwnedStr auto-borrowed to Str
  }                    // owned's scope ends HERE -- freed
  print h.s;           // heap-use-after-free: h.s aliases owned's freed buffer
  return 0;
}
```
```vani
struct Holder { s: Str, n: i64 }
fn make() -> Holder {
  let owned: OwnedStr = i64_to_str(77);
  return Holder { s: owned, n: 1 };   // StructLit field init, same coercion
}
fn main() -> i64 {
  let h: Holder = make();   // owned's scope (make's body) ends -- freed
  print h.s;                // heap-use-after-free
  return 0;
}
```
Both confirmed via direct ASan runs: `heap-use-after-free` on the
`print`/`fputs` read, `freed by` traced straight to the OwnedStr
local's own ordinary scope-exit Drop. Root cause is the SAME
`can_assign` assumption BUG-157 already found unsound for `Stmt::Let`
("the OwnedStr binding stays live; its drop fires at the original
scope's end") -- but here it's `Stmt::FieldAssign`'s own
`coerce_checked` call (checker.rs ~line 14935) that has the identical
gap: the source `OwnedStr` local's own (correctly-scoped, narrower)
lifetime ends before the struct/container it was written into (which
outlives that scope) is done reading the aliased `Str` view. The
async transform's `__poll_*` codegen just happens to synthesize
exactly this `FieldAssign` shape internally when hoisting a
cross-state `Str` local -- it isn't the root cause, just one caller
that reaches it. `StructLit`'s field-init coercion likely shares the
identical gap (confirmed above); other escape vectors (`Vec` push, a
struct passed by value into another escaping container, etc.) were
not individually checked and should be assumed similarly affected
until proven otherwise.

**FIXED next day, as BUG-158, via a dedicated follow-up pass at
direct request.** Rather than attempt real scope-depth escape
analysis under time pressure (rejected as too risky the day this was
found -- see the original reasoning, kept below for the record), the
fix rejects the coercion outright at both confirmed sites
(`Stmt::FieldAssign`, `StructLit` field init) plus a third, closely
related one found during implementation (`xs[i].field =
owned_expr;`, the mixed index+field-assign path -- same risk against
a `Vec` that typically outlives a narrower-scoped `OwnedStr`). A
clear diagnostic + 3-step elaboration points at the fix: declare the
field `OwnedStr` instead of `Str`, or supply an already-safe `Str`
value. Matches this codebase's established preference for compile-
time rejection over unsound runtime behavior -- simpler and safer
than attempting the ownership-transfer alternative also considered
(suppressing the source's own drop specifically at these escaping
positions), which would have needed more careful, riskier surgery to
avoid breaking the already-correct `Var`-to-already-live-binding
case.

Still EXCLUDED for `__poll_*` functions, same rationale as BUG-157:
rejecting there would break the already-shipped (leaky-but-safe, not
crashing) async Str-local feature wholesale. The async-transform
INSTANCE of this exact bug (confirmed via the two-`let` manual-
workaround repro above) was left open here -- **now FIXED, in the
async transform itself, same day. See "Async transform fix" below.**

Full writeup, verification (both repros now cleanly rejected, `vanic
check examples` baseline unchanged, corpus-wide sweep unchanged, full
`cargo test --release` clean, 5 new regression tests), in
`docs/TODO_CURRENT.md`'s BUG-158 section.

`tools/leak_sweep_baseline.json`'s existing entries did NOT need
updating for this fix -- neither of the two minimal repros went
through the sweep corpus (they're standalone repros, not files under
`examples/`), and the sweep's own re-run after the fix confirmed an
identical flagged-file set (the async cluster's leak-not-UAF
behavior is unchanged, as intended).

<details>
<summary>Original same-day reasoning for NOT fixing immediately (superseded by the FIXED note above, kept for the record)</summary>

This is more general and more severe than the narrow "hoist checker-
synthesized temps" framing originally used above for the `__poll_`
caveat, and a correct general fix needs real design work (either
reject the coercion at compile time when the source's scope is
provably narrower than the destination's -- a real escape-analysis
feature that doesn't exist yet in any form -- or make the coercion an
actual ownership transfer in these escaping-write positions
specifically, suppressing the source's own drop, which needs to be
done without breaking the already-correct, already-tested `Var`-to-
already-live-binding case). Attempting this at the tail of an already
very long session, right after the BUG-157 near-miss already
demonstrated how easy it is to turn a leak into a worse UAF in this
exact area, would repeat the same mistake at a larger scale. This
needs a dedicated session with full attention, not a rushed attempt.

</details>

## Async transform fix (2026-08-10) -- the __poll_* instance of BUG-157/158, FIXED at direct request

Root cause, precisely: `try_v31_transform`'s (parser.rs) liveness
analysis classifies each local purely by NAME -- "is this name read
in a different state than the one that declared it?" -- with no
notion that one local's VALUE might alias another's underlying heap
buffer. `let label: Str = i64_to_str(mode);` makes `label` cross-
state (hoisted into the Task struct as a `Str` field aliasing a
state-local `OwnedStr` temp), while the temp holding the ACTUAL
buffer stays a plain per-state stack local, freed at the end of ITS
OWN (narrower) state while the hoisted alias is read later.

**Fix**: when a cross-state local is declared `Str` but its value
provably comes from an `OwnedStr` source (a call to a small, explicit
allowlist of known OwnedStr-returning builtins --  `i64_to_str`,
`f64_to_str`, `f64_to_str_fixed`, `bool_to_str` -- or a plain `Var`
referring to another local/param already known `OwnedStr`-typed),
hoist it into the Task struct as an `OwnedStr` FIELD instead of a
`Str` alias. The EXISTING, already-correct generic struct-Drop
machinery then frees it whenever the Task struct itself is dropped --
exactly like any other struct with an `OwnedStr` field, no new drop
logic needed anywhere. The synthesized temp's type is updated to
match, so the FieldAssign that stores it becomes an exact-type,
ownership-transferring move instead of an untracked alias copy.

Can't run real type inference (the checker hasn't run yet at parse
time) -- deliberately conservative, verified against the two real
shipped examples that actually use this feature
(`echo_p3_locals_stress.vani`, `echo_p3b_str_local.vani`, both using
exactly the allowlisted `i64_to_str` shape). Anything the allowlist
doesn't recognize keeps the ORIGINAL leaky-but-safe behavior,
guarded by the existing checker-side `__poll_*` exclusion -- this
pass can only IMPROVE safety, never regress compilability.

**`hashmap_strstr.vani` / `hashmap_strv.vani` are NOT fixed by this**
-- confirmed during this fix that neither file has an `async fn` at
all; they were mislabeled as part of the "async cluster" in round 8's
first pass. Their leak is separate and still undiagnosed, most likely
in `HashMap<OwnedStr, OwnedStr>`'s own insert/remove semantics. Left
open, re-labeled correctly in `tools/leak_sweep_baseline.json`.

Verified: both real async examples run completely clean under ASan
(exit 0 -- confirmed via the generated C, which shows the Task
struct's field freed both on overwrite mid-poll and at the Task's
own scope exit in `main`). `vanic check examples` baseline unchanged.
Corpus-wide sweep flagged count dropped from 8 to 6 (both async files
no longer flagged; baseline updated with corrected reasons for the
remaining 6). Full `cargo test --release` clean. 3 regression tests
(1 updated `src/lib.rs` compile-check asserting the field's free
sites directly, the existing `__poll_*`-exclusion guard test kept,
1 real-subprocess `tests/run_end_to_end.rs` test running the actual
example pattern end-to-end on both backends). Full writeup in
`docs/TODO_CURRENT.md`'s "BUG-157/158 async instance" section.

## Done: wired into CI (2026-08-09, same day)

This sweep was ad hoc scratchpad tooling at first (a Python driver,
not committed). Given how directly and quickly it found real bugs --
BUG-154 general and severe, plus catching the BUG-157 near-miss UAF
before it ever reached `main` -- it's now a real, checked-in tool:
`tools/leak_sweep.py`, run as the `leak-sweep` job in
`.github/workflows/ci.yml` on every push/PR to `main` (ubuntu-latest,
~5-6 minutes: `cargo build --release --bin vanic` + the sweep itself).

Known/already-triaged findings (the 2 methodology false positives, 2
low-severity UBSan literal-spelling findings, and the BUG-157 async
`__poll_*` cluster left deliberately open) are tracked in
`tools/leak_sweep_baseline.json` with a `reason` field each, so CI
only fails on a genuinely NEW finding -- not on rediscovering an
already-documented one. The script also fails CI if a previously
baselined finding stops reproducing (either the underlying bug got
fixed -- good, remove the stale entry -- or the sweep methodology
regressed -- bad, needs a look), so the baseline can't silently drift
out of sync with reality. Full usage docs in `tools/README.md`.

This only covers the C backend (ASan can't instrument already-
emitted LLVM IR after the fact) and only `examples/` (not the
`run_end_to_end.rs`-extracted corpus this round's manual sweep also
covered) -- both reasonable scope cuts for a per-push CI gate;
extending either is a fine future follow-up but not required for the
CI job to be worth having.

## Process (mirrors rounds 1 through 7's own process sections)

1. Check `docs/TODO_LOCAL_STAGING.md` (localfuzz worktree) for
   anything new -- re-verify against a freshly rebuilt `main` first
   (the worktree has gone stale multiple times in single days this
   week; always refresh before trusting it).
2. Two confirmed, unfixed leaks are documented above with concrete
   starting points -- either is a reasonable place for a future
   session to open, alongside (or instead of) a fresh round-9 theme.
3. Every fix gets a `src/lib.rs` compile-check test AND a
   `tests/run_end_to_end.rs` real-subprocess test on both backends --
   upheld this round.
4. Full `cargo test --release` clean + `vanic check examples`
   compared against the baseline (78 errors as of this writing).
   Verify freshness before every commit.
5. CI/CodeQL polled green after every push.
