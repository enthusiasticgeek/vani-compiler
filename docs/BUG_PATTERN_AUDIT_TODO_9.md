# vāṇी — Bug-pattern audit, round 9

**STATUS (2026-08-10): Category 1's two cheapest pickups FIXED
(BUG-160, BUG-161); Category 3's overflow/bounds-divergence theme also
FIXED (BUG-162), plus the struct-field-Vec bounds crash it surfaced
also FIXED (BUG-163); all 8 localfuzz timeout findings investigated
and confirmed as corpus artifacts (genuinely broken mutated programs),
not compiler bugs.** See `docs/TODO_CURRENT.md` for the full
writeups. This is now a partial-progress candidate list, not a
from-scratch one: Category 2 is untouched (by design -- those 4 are
non-fixes); Category 1's general (ordinary-user-function) case remains
deliberately unscoped/open; Category 3's timeout cluster is fully
resolved (no fix needed -- see below), but it gained a new,
much-larger, NOT-pursued candidate: SSA-C has no `Vec<T>` support at
all, causing per-program SSA/tree divergence on the C side independent
of anything BUG-162 touched.

Three source categories, each its own section below:
1. More instances of BUG-159's exact bug family (OwnedStr auto-borrow
   leaking through function-call arguments) -- found while root-
   causing BUG-159, not yet fixed.
2. The 4 items still in `tools/leak_sweep_baseline.json` -- all
   already triaged as NOT worth fixing (2 methodology false
   positives, 2 low-severity portability notes), listed here only
   for completeness/context, not as fix candidates.
3. New localfuzz candidates accumulated since round 8 started
   (2026-08-09 evening onward) -- raw, mostly untriaged.

---

## Category 1: more BUG-159-family leaks (OwnedStr arg to a function call)

BUG-159 fixed `hashmap_insert`'s own K/V parameters only, deliberately
narrow. While root-causing it, confirmed the SAME leak (a fresh,
never-bound `OwnedStr` expression passed as a function argument,
where the callee doesn't take ownership responsibility) recurs in
several sibling positions that the narrow fix didn't touch. All
confirmed via direct ASan testing, 2026-08-10.

### FIXED (2026-08-10, BUG-160 + BUG-161 -- see docs/TODO_CURRENT.md)

- **`hashmap_get(ref m, K)` / `hashmap_contains_key(ref m, K)` /
  `hashmap_remove(mut ref m, K)`** -- BUG-160. Fixed via the same
  `is_fresh_owned_str` + free-after-call pattern as `hashmap_insert`.
- **`trie_insert` / `trie_contains` / `trie_starts_with` /
  `trie_delete`** (all 4 siblings, not just `.insert(...)`) --
  BUG-161. Needed one more layer than BUG-160: Trie's key param is
  typed `Str`, so a fresh OwnedStr arg arrives wrapped in the
  checker's implicit borrow cast; fixed via a new
  `is_fresh_owned_str_via_str_cast` helper in `src/ir.rs` that
  unwraps exactly that cast. `examples/language/english/trie.vani`
  uses `Str` literals only and was never affected.

### Confirmed leaking, NOT fixed

- **Ordinary user-defined functions taking a `Str` parameter** -- the
  general, pervasive case, confirmed via:
  ```vani
  fn takes_str(s: Str) -> i64 { return len(s) as i64; }
  fn main() -> i64 {
    let n: i64 = takes_str(i64_to_str(12345));   // leaks
    ...
  }
  ```
  This is almost certainly the highest-impact instance by sheer
  frequency of occurrence in real code, and the reason a "narrow
  hashmap_insert-only" framing understates the problem -- but fixing
  it means touching every function-call-argument codegen site in the
  compiler (both backends), a change with a much larger blast radius
  than anything fixed this round. Needs its own scoping discussion,
  not a quick pickup.

### Confirmed NOT affected (checked so a future session doesn't
### re-derive this)

- **`Vec<OwnedStr>::push`** -- takes ownership of the pushed value
  directly (no internal clone); a fresh `OwnedStr` argument does NOT
  leak. Confirmed via `push(mut ref v, i64_to_str(5))`.
- **`HashSet<OwnedStr>`, `BTreeMap<OwnedStr, _>`, `BTreeSet<OwnedStr>`,
  `Deque<OwnedStr>`, `BST` (string-keyed), `BloomFilter` (string
  element), `SkipList` (string value)** -- none of these support
  `OwnedStr` as a generic parameter in v1 at all (all reject at
  compile time with "only supports `<i64 variant>` in v1" or a type-
  mismatch error). Not reachable, so not affected. Confirmed via
  direct `vanic check` on minimal repros for each.

### Suggested approach for a future session

Both cheap pickups (`hashmap_get`/`_contains_key`/`_remove` as
BUG-160, `trie_insert`/`_contains`/`_starts_with`/`_delete` as
BUG-161) are now fixed -- see the FIXED subsection above. What
remains in this category is only the general function-call-argument
case (ordinary user functions taking `Str`), which is a separate,
much larger undertaking that deserves its own dedicated session and
explicit scoping conversation before starting -- do not fold it into
a "quick" sweep. Note BUG-161 showed the general case is slightly
bigger than BUG-159/160 alone suggested: any `Str`-typed parameter
(not just `OwnedStr`-typed ones) needs the cast-unwrapping
`is_fresh_owned_str_via_str_cast` check too, not just the direct
`is_fresh_owned_str` check.

---

## Category 2: `tools/leak_sweep_baseline.json`'s 4 remaining entries (already triaged, not fix candidates)

Listed here only so a future session doesn't waste time re-
discovering these are already-decided non-fixes. All 4 have full
`reason` writeups directly in the baseline file; summarized:

1. **`examples/edge_cases/mix_conc_channel_send_recv.vani`**
   (`ASAN_EXIT_99_UNCLASSIFIED`) -- sweep-methodology artifact, not a
   bug. The program's own legitimate return value (`99`) collides
   with the sweep's own `ASAN_OPTIONS exitcode=99` convention. No
   sanitizer error is ever actually printed.
2. **`examples/language/english/bare_metal.vani`** (`SEGV`) --
   methodology false positive. Reads a hardcoded STM32 GPIO address
   meant for real embedded hardware/QEMU; correctly SEGVs under
   native userspace execution, which is the "wrong" way to run it.
3. **`examples/language/english/loop_carried_overflow_not_elided.vani`**
   (`UBSAN_MAYBE`) -- low severity, not fixed. i64::MIN literal
   spelled as `-(int64_t)9223372036854775808LL` in generated C,
   technically UB by the strict standard, universally correct in
   practice via constant folding on every real compiler/CPU. No
   observed wrong output. **Possibly related to the localfuzz
   overflow-divergence pattern in Category 3 below** -- worth
   checking whether a proper fix here (spell the literal as
   `INT64_MIN` or an unsigned-cast form) also resolves any of those.
4. **`examples/language/english/sort_large_block_partition.vani`**
   (`UBSAN_MAYBE`) -- low severity, not fixed. Same class: `<<` on a
   signed `int64_t` hits stricter UB rules by the letter of the C
   standard when the shifted result doesn't fit, universally a single
   machine SHL instruction in practice. No observed wrong output.

---

## Category 3: new localfuzz candidates since round 8 started (2026-08-09 evening onward, mostly untriaged)

**Update (2026-08-10, later same day)**: 11 more findings landed since
this section was first written (last one captured below was
`b4cbb21d7a`). See the new subsections at the end of this category --
"11 more findings (2026-08-10 04:14 onward)". None fixed or
root-caused; this is still a raw inventory. The overflow-divergence
theme below is now confirmed to be BROADER than integer overflow --
one new finding shows the identical silent-vs-loud divergence for an
"index out of bounds" trap.

Source: `docs/TODO_LOCAL_STAGING.md` in the localfuzz worktree
(`/home/virgo/source/vani-compiler-localfuzz`). Per the established
localfuzz workflow, always re-verify against a freshly rebuilt `main`
before trusting any of these -- the worktree can go stale within a
single day. None of these have been root-caused or fixed this
session; this is a raw inventory, not a triage.

### FIXED (2026-08-10, BUG-162 -- see docs/TODO_CURRENT.md): LLVM backend's overflow/bounds-check traps exited silently, C backend's aborted loudly

Two separate candidates originally showed the identical shape (kept
below for the record); root-caused, fixed, and verified same-day, and
turned out broader than "overflow-check" -- the SAME silent-vs-loud
split also affected the general Vec bounds-check trap (confirmed via
a THIRD localfuzz finding, `20260810-113044-backend-divergence-
b4d35be8e4`, "index out of bounds"). Fixed by adding a shared
`@__intent_trap(i8* %msg)` helper (prints via `dprintf` before the
deliberately-unchanged `exit(3)`) to both `ssa_backend_llvm.rs` and
`backend_llvm.rs`, scoped to: checked Add/Sub/Mul overflow, Div/Rem-
by-zero, signed `MIN / -1` overflow, Shl/Shr range, and the general
Vec bounds-check. Deliberately NOT extended to `requires`-clause
violations, the `#[bounded(N)]` recursion guard (needs a dynamic
message, different mechanism), or the Vec-builtin-specific bounds
checks (`swap_remove`/`insert`/`pop_mut`) -- noted as follow-ups in
`docs/TODO_CURRENT.md`'s own BUG-162 writeup, which has the full
technical detail (message-wording nuances between the two C backends,
verification results, etc.) -- not repeated here.

Two of the originally-documented candidates, for reference:

- **`20260809-221155-backend-divergence-56467d8c82`** -- repro:
  ```vani
  fn main() -> i64 {
    let add3 = fn(x: i64) -> i64 { return x + 3; };
    let mul2 = fn(x: i64) -> i64 { return x * 9223372036854775807; };
    let n: i64 = add3(5);
    return mul2(n);
  }
  ```
  C backend: `rc=134` (SIGABRT), stderr `"integer overflow in i64 mul"`.
  LLVM backend: `rc=3`, empty stderr.
- **`20260810-020953-backend-divergence-966a249216`** -- repro (a
  Sinhala-pragma mutant of a `requires`-guarded `add` function,
  `i64::MAX + 7`): C backend `rc=134`, stderr `"integer overflow in
  int64_t add"`. LLVM backend: `rc=3`, empty stderr.

Both cases: an i64 arithmetic op provably overflows; BOTH backends
correctly detect it and refuse to continue (neither produces a wrong
answer) -- but the OBSERVABLE behavior diverges: exit code (134 vs 3)
and stderr content (a clear message vs nothing). This is likely not a
correctness bug, but a real backend-consistency / debuggability gap
worth a look: does the LLVM backend's overflow-check trap print
anything to stderr at all, ever? If not, that's a straightforward,
well-scoped fix (make LLVM's overflow trap print the same kind of
message C's does before exiting) -- much lower risk than anything in
Category 1. Worth checking whether this connects to Category 2 item
3/4's i64::MIN/shift UB findings -- same neighborhood of the codebase
(overflow/UB handling), not confirmed to be the same code path.

### FIXED (2026-08-10, BUG-163 -- see docs/TODO_CURRENT.md): struct field holding a Vec crashed on tree-LLVM out-of-bounds, no clean message

Found while manually testing BUG-162 repros (not from localfuzz, no
finding directory); root-caused and fixed same-day. Root cause:
`backend_llvm.rs`'s `TypedExprKind::Index` handler had a dedicated
FieldAccess-base arm with 3 sub-cases (`Vec<bool>`, `Array`, general
`Vec<T>`) -- the general `Vec<T>` sub-case was simply missing the
`@__intent_bounds_check` call every sibling arm around it already had
(BUG-108/122/149). Fixed by adding it, matching the sibling shape
exactly. Full technical detail (including why the write-path analog
turned out not to be valid syntax at all) is in `docs/TODO_CURRENT.md`.
Repro, for reference:
```vani
struct Holder { xs: Vec<i64>, i: i64 }
fn f(h: Holder) -> i64 { return h.xs[h.i]; }
fn main() -> i64 {
  let xs: Vec<i64> = vec(10, 20, 30);
  let h: Holder = Holder { xs: xs, i: 7 };
  return f(h);
}
```
Before the fix: tree-LLVM (both `vanic run` and AOT `vanic build`)
crashed with NO stderr output at all and an unstable exit code across
repro shape variants (101, 107, 117 all observed) -- not a clean
`rc=3` + message like BUG-162 gives every other bounds-check trap.
Tree-C on the same repro always ran correctly. Confirmed via `git
stash` that this reproduced identically on the pre-BUG-162 code -- a
separate, pre-existing bug, not a BUG-162 regression. After the fix:
both backends give the identical `"index out of bounds: 7, len 3"`
message with a clean, stable `rc=3`/`rc=134`.

### Needs root-cause investigation (no clear pattern yet)

- **`20260809-192216-backend-divergence-77aaa194ce`** -- repro
  involves `Box<dyn Shape>` with a field set to `i64::MIN`
  (`-9223372036854775808`), a closure computing `n * 2` on the
  dispatched `.area()` result. Marked "needs human/frontier root-
  cause review" with no further detail captured. Possibly also
  i64::MIN-literal-related (same family as Category 2 item 3) given
  the MIN-valued field, but not confirmed -- the closure/dyn-dispatch
  composition makes this a different shape than the plain-overflow
  cases above.
- **`20260809-201604-backend-divergence-7b9b35c019`** -- a mutant of
  `vec_invariants.vani` reported to crash with "integer overflow in
  int64_t mul" when run `--backend=c`. Marked "needs human/frontier
  root-cause review", repro/details not fully captured in the staging
  doc -- read `tools/localfuzz/findings/20260809-201604-backend-
  divergence-7b9b35c019/repro.vani` directly before starting.
- **`20260810-015549-run-crash-cdec4c613b`** -- FIXED, i.e. investigated
  and closed with no code change needed (2026-08-10): a mutant of
  `btreeset.vani` where the drain loop's decrement (`j = j + 1;` ->
  `j = j + -1;`) makes `j < 20` never go false. A genuine infinite
  loop in the mutated SOURCE, not a compiler/runtime bug -- see the
  "Localfuzz timeout findings" subsection in `docs/TODO_CURRENT.md`
  for the full investigation of this and 7 siblings, all reaching the
  same conclusion.
- **`20260810-024150-backend-divergence-8e74a245e6`** -- marked
  "needs human/frontier root-cause review", no detail captured in the
  staging doc at all. Read the repro directly.

### Write-ups that look unreliable -- re-verify manually before trusting the auto-generated description

- **`20260810-023328-backend-divergence-ecc728fea0`** -- the staged
  write-up is incomplete/malformed: it's a Kannada-pragma mutant of
  `basics.vani` whose "Generated Source Code" block cuts off mid-
  function-declaration with no actual finding data (no raw JSON
  result, no stderr/rc comparison) after it. Looks like the LLM
  write-up step failed partway through. Re-run against the repro
  file directly rather than trusting this write-up.
- **`20260810-025203-backend-divergence-b4cbb21d7a`** -- the write-up
  claims "The C backend failed to generate valid LLVM IR due to an
  out-of-bounds access error" -- self-contradictory on its face (the
  C backend does not generate LLVM IR), suggesting a confused/
  hallucinated auto-description. Also claims the C backend "segfaulted
  due to an index out-of-bounds error in the generated LLVM IR" --
  same confusion. The underlying finding might still be real (an
  actual out-of-bounds access somewhere), but don't trust this
  description's account of WHICH backend does WHAT; re-derive from
  the raw repro + actual run output directly.

### 11 more findings (2026-08-10 04:14 onward)

All from the same localfuzz worktree, none root-caused or fixed. Raw
`finding.json` data quoted directly; `fix_attempt.md`'s auto-generated
hypotheses are qwen2.5-coder:1.5b drafts (per the established
staleness/unreliability caveat above) and mostly say "no patch
attempted -- needs frontier-model or human review from scratch", so
not reproduced here except where noted.

**More overflow/bounds-divergence-theme findings** (same shape as the
2 originally documented above -- C aborts loudly with rc=134 + a
message, LLVM exits silently with rc=3 + empty stderr):

- `20260810-063522-backend-divergence-5e6cada3c6` (base
  `examples/language/bengali/early_exit.vani`) -- "integer overflow in
  int64_t add".
- `20260810-074148-backend-divergence-2492443756` (base `examples/
  language/english/loop_carried_overflow_not_elided.vani` -- note:
  this is literally the SAME source file as Category 2 item 3's
  baseline entry) -- "integer overflow in int64_t add".
- `20260810-090537-backend-divergence-20dcfc064d` (base `examples/
  language/catalan/control_flow.vani`) -- "integer overflow in
  int64_t add".
- `20260810-091328-backend-divergence-236442a7a6` (base `examples/
  language/spanish/for_loops.vani`) -- "integer overflow in int64_t
  add". A patch was auto-attempted and discarded (didn't apply/build).
- `20260810-105409-backend-divergence-1f84ad9671` (base `examples/
  language/english/vec_of_ref.vani`) -- "integer overflow in i64
  mul" (the multiply variant, matching the very first `56467d8c82`
  finding's op).
- **`20260810-113044-backend-divergence-b4d35be8e4`** (base `examples/
  language/english/bounds_elision.vani`) -- **"index out of bounds"**,
  not overflow. Same rc=134-loud/rc=3-silent divergence. This is new
  evidence the theme isn't overflow-specific: it looks like ANY LLVM
  runtime safety-guard trap (bounds checks included, likely others)
  exits silently, while C's aborts loudly with a message. Worth
  checking generally where the LLVM backend emits its trap/abort logic
  and whether a single shared fix (make it print like C's does before
  exiting) covers overflow AND bounds AND whatever else uses the same
  trap mechanism. A patch was auto-attempted and discarded here too.

### 4 more findings, landed after BUG-162 shipped (2026-08-10, later)

- `20260810-134722-backend-divergence-f1f2c603f4` (base `examples/
  language/amharic/for_loops.vani`) -- "integer overflow in int64_t
  add". Re-ran against the current (BUG-162-fixed) binary: FULLY
  CLOSED, both backends now print the identical message. Confirms the
  fix.
- **`20260810-124224-backend-divergence-e1322cfcd5`** (base `examples/
  language/odia/control_flow.vani`) -- an out-of-range Vec index.
  Re-ran against the current binary: no longer silent on either side
  (BUG-162 worked), but the exact WORDING still differs -- LLVM prints
  the SSA-pair's static `"index out of bounds"`, C prints the
  tree-pair's dynamic `"index out of bounds: 5, len 5"`. Root-caused:
  this program uses `Vec<i64>`, and `ssa_backend_c::emit` explicitly
  rejects ANY `Vec<T>` (`"type Vec(...) is outside the SSA-C scalar/
  string subset"`) while SSA-LLVM has no such restriction -- so this
  program's LLVM side takes the SSA fast path while its C side falls
  back to tree-C, independently, for the same source. NOT a BUG-162
  bug (the silence is fixed); a separate, much larger, pre-existing
  architectural gap (SSA-C has no Vec support at all). See `docs/
  TODO_CURRENT.md`'s "SSA-C has no Vec support at all" section for the
  full writeup. Not fixed, not a quick pickup -- would mean either
  implementing Vec in SSA-C or accepting the wording split as
  permanent (current call: accept it, documented in the tutorials).
- `20260810-124633-run-crash-d669b7fd24` (mix_conc_channel_send_recv.vani)
  and `20260810-142721-run-crash-22c8c106e2` (spanish/
  async_cancel_auto.vani) -- both timeout findings, folded into the
  "More timeout findings" list below (both investigated and closed as
  corpus artifacts, not bugs).

**More timeout findings, ALL INVESTIGATED AND CLOSED (2026-08-10, no
code change needed)** -- both backends hang, `rc=null, timed_out=true`
on both C and LLVM, same shape as `cdec4c613b` above. Every one of
these (plus 2 more that landed even later, `d669b7fd24` and
`22c8c106e2`, not yet in this doc when this subsection was first
written) was root-caused by diffing against its base example; all 8
total are genuinely broken mutated programs (infinite loops,
deadlocks, or an absurd-but-correct multi-million-year sleep), not
compiler/runtime bugs. Full investigation writeup in
`docs/TODO_CURRENT.md`'s "Localfuzz timeout findings" section; one-line
summary per finding:

- `20260810-041441-run-crash-d5870f7218` (pashto/async_cancel_auto.vani)
  -- `sleep_ms` argument mutated to i64::MAX.
- `20260810-074640-run-crash-d8f20b7050` (mix_conc_mutex_struct.vani)
  -- same non-reentrant mutex locked twice on one thread, self-deadlock.
- `20260810-094706-run-crash-7b63fa98b6` (tibetan/early_exit.vani) --
  loop increment statement deleted.
- `20260810-101253-run-crash-e37418e21c` (spanish/control_flow.vani)
  -- loop increment flipped to a decrement.
- `20260810-103151-run-crash-78bdde8bba` (lao/early_exit.vani) -- same
  as `7b63fa98b6`, deleted increment.
- `20260810-124633-run-crash-d669b7fd24` (mix_conc_channel_send_recv.vani)
  -- the `channel_send` call deleted; `channel_recv` blocks forever.
- `20260810-142721-run-crash-22c8c106e2` (spanish/async_cancel_auto.vani)
  -- same as `d5870f7218`, `sleep_ms` mutated to i64::MAX.

No fix needed for any of these. One soft follow-up noted (not
pursued): `d8f20b7050`'s same-thread double-lock is arguably a missed
static-analysis opportunity for vāṇी's existing lock-order checker,
which doesn't currently catch this pattern in a straight-line scope.

---

## Process note for whoever picks this up

Follow the same process established across rounds 1-8 (see
`docs/BUG_PATTERN_AUDIT_TODO_8.md`'s own "Process" section for the
full checklist): re-verify localfuzz findings against a freshly
rebuilt `main` first, root-cause with a minimal repro before
fixing, add both a `src/lib.rs` compile-check test and a
`tests/run_end_to_end.rs` real-subprocess test per fix, run the full
`cargo test --release` + `vanic check examples` baseline + the
corpus-wide `tools/leak_sweep.py` sweep before considering any fix
done, and poll CI/CodeQL green after every push.
