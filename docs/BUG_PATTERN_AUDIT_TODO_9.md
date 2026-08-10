# vāṇी — Bug-pattern audit, round 9 (candidates, NOT started)

**STATUS (2026-08-10): NOT STARTED.** This is a candidate list compiled
at the end of round 8 (which is fully closed -- see
`docs/BUG_PATTERN_AUDIT_TODO_8.md`), at direct request, so a future
session can pick a theme without re-deriving this research from
scratch. Nothing in this file has been fixed. Some items are well-
root-caused already (cheap to pick up); others are raw localfuzz
findings that still need root-causing.

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

### Confirmed leaking, NOT fixed

- **`hashmap_get(ref m, K)`** -- fresh `OwnedStr` K leaks. Repro:
  ```vani
  let r: Option<OwnedStr> = hashmap_get(ref m, i64_to_str(1));
  ```
  Leaks 2 bytes / 1 object under ASan.
- **`hashmap_contains_key(ref m, K)`** -- same, fresh K leaks.
- **`hashmap_remove(mut ref m, K)`** -- same, fresh K leaks. (Note:
  unlike `_insert`, `_get`/`_contains_key`/`_remove` don't clone K
  into new storage at all -- they only use it for a `strcmp` lookup
  and then discard it. The exact "clone vs never-freed-source" shape
  BUG-159's writeup describes for `_insert` doesn't apply the same
  way here; still, root cause is the same category: nothing frees
  the fresh temporary after the call, because nothing owns it.)
  Combined repro (`contains_key` + `remove`) leaks 4 bytes / 2
  objects.
- **`trie_insert` / `Trie.insert(...)`** -- fresh `OwnedStr` key
  leaks identically to `hashmap_insert`'s pre-fix behavior. Repro:
  ```vani
  let t: Trie = trie_new();
  let _ = t.insert(i64_to_str(5));   // leaks
  ```
  Leaks 2 bytes / 1 object. NOT checked: `trie_contains`,
  `trie_starts_with`, `trie_delete` (all take a Str-shaped key
  argument too, per the same builtin family -- likely affected
  identically, not individually verified this session).
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

Given `hashmap_get`/`_contains_key`/`_remove` are the SAME file,
SAME function family, SAME fix pattern (`is_fresh_owned_str` +
free-after-call) as the already-fixed `hashmap_insert`, they're the
cheapest, lowest-risk pickup -- essentially finishing what BUG-159
started, not a new investigation. `trie_insert` (and its Str-key
siblings) is the next-cheapest, same pattern, different file. The
general function-call-argument case is a separate, much larger
undertaking that deserves its own dedicated session and explicit
scoping conversation before starting -- do not fold it into a
"quick" round-9 sweep.

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

Source: `docs/TODO_LOCAL_STAGING.md` in the localfuzz worktree
(`/home/virgo/source/vani-compiler-localfuzz`). Per the established
localfuzz workflow, always re-verify against a freshly rebuilt `main`
before trusting any of these -- the worktree can go stale within a
single day. None of these have been root-caused or fixed this
session; this is a raw inventory, not a triage.

### Likely ONE unified theme: LLVM backend's overflow-check exits silently, C backend's aborts loudly

Two separate candidates show the identical shape:

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
- **`20260810-015549-run-crash-cdec4c613b`** -- BOTH backends TIMED
  OUT on the same input (`rc=null, timed_out=true` for both C and
  LLVM). Needs investigation into whether this is a genuine compiler/
  runtime hang (a real bug) or simply a fuzzer-mutated program that
  legitimately contains an infinite loop (not a bug, a corpus
  artifact) -- check the repro's actual control flow first before
  assuming either.
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
