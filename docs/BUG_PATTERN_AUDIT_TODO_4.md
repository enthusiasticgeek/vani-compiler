# vāṇी — Bug-pattern audit, round 4

**STATUS (2026-08-09): OPEN, one confirmed unfixed lead ready to go.**
Sequel to `docs/BUG_PATTERN_AUDIT_TODO_3.md` (round 3, closed 2026-08-08
as BUG-146). This round's localfuzz backlog triage (5 fresh candidates,
2026-08-08 through 2026-08-09) found only one genuine bug --
`clone_at()`'s missing bounds check, fixed same-session as **BUG-147**
-- and the other 4 were non-bugs (a correct i64::MIN overflow trap, two
source-level infinite loops from mutation stripping a loop's increment,
one more instance of the already-characterized `sleep_ms(i64::MAX)`
pattern). See `docs/TODO_CURRENT.md`'s BUG-147 section for the full
fix writeup.

Rather than derive round 4's theme from a shape localfuzz hasn't found
yet, this file follows round 3's other lesson: BUG-147 itself pointed
at a *class*, not just one call site. A short manual code audit
(grep + direct reproduction, not waiting on the fuzzer) done in the
same session already found a second, still-unfixed instance of the
exact same class. That's category A below, and it's a running head
start, not a cold lead -- the next session should be able to open with
a fix, the same way round 3 did for its two categories.

## A. Bounds-check coverage audit across indexed-access builtins (🔴 high) -- one CONFIRMED unfixed lead, rest needs a systematic sweep

**Confirmed, unfixed:** `vec_remove_at(mut ref xs, i)` never bounds-checks
`i` on either backend -- exactly the same gap BUG-147 fixed for
`clone_at`. Directly reproduced on current main (commit `a33cec3`,
includes the BUG-147 fix):

```vani
fn main() -> i64 {
  let xs: Vec<i64> = vec(1, 2, 3);
  let r: i64 = vec_remove_at(mut ref xs, 99);
  print r;
  return 0;
}
```

- LLVM: prints `4` (garbage read past the 3-element buffer), exits 0.
- C: prints `0` (different garbage), exits 0.

Neither backend traps. Compare the SAME shape against `swap_remove` and
`insert` -- both DO trap correctly and consistently (LLVM exit 3, C
exit 134 with `"swap_remove: index out of bounds"` / `"insert: index
out of bounds"` on stderr) -- confirming this is a real, narrow gap in
`vec_remove_at` specifically, not a general problem with the indexing
convention.

Root cause (from reading, not yet fixed): `vec_remove_at` is a special-
cased, monomorphized-to-`Vec<i64>` builtin (`backend_c.rs`'s
`"vec_remove_at" =>` arm at ~line 18386 emits a raw C statement-
expression indexing `intent_vec_int64_t`'s `.data[__vra_i]` directly;
`backend_llvm.rs`'s `if name == "vec_remove_at"` arm at ~line 7992
GEPs `%intent_vec_i64`'s `data` field directly) -- both were written
as one-off inline codegen rather than routed through the shared
`vec_helper(element, ...)` per-type-helper machinery that
`swap_remove`/`insert` use, and whoever wrote them didn't carry over
the bounds-check convention. `vec_remove_at` doesn't exist in either
SSA backend at all (`ssa_backend_c.rs`/`ssa_backend_llvm.rs` have no
match arm for it) -- `src/main.rs`'s `ssa_path_supports` already
excludes it by name (~line 675), forcing tree-backend dispatch
unconditionally, so there's no third/fourth site to fix, just these two.

Suggested fix shape: same as BUG-147 -- wrap the index in
`intent_check_bounds((int64_t)(i), (int64_t)xs->len)` in `backend_c.rs`,
and add a `call void @__intent_bounds_check(i64 %idx, i64 %len)` before
the GEP in `backend_llvm.rs` (load `len` the same way the existing
shift-loop's `len` load already does, a few lines above the unguarded
read). Add the mirror pair of regression tests BUG-147 added (a
`src/lib.rs` compile-check that the emitted code contains the bounds-
check call/macro, and a `tests/run_end_to_end.rs` real-subprocess test
asserting the OOB case now traps with the expected exit code/message
on both backends, plus an in-bounds sanity check that it still returns
the correct value and shifts elements correctly).

**Needs a systematic sweep (not yet done):** the two confirmed
instances (`clone_at`, `vec_remove_at`) share a specific shape --
special-cased/inline codegen that bypasses the shared per-type helper
machinery. The rest of the ~300-entry builtin surface hasn't been
checked. A worthwhile next step is grepping every `"\w+_at"` /
explicit-index-argument builtin across all four codegen files
(`backend_c.rs`, `backend_llvm.rs`, `ssa_backend_c.rs`,
`ssa_backend_llvm.rs`) and classifying each site into one of three
buckets -- confirmed via this session's spot-checks:

1. **Should be checked and isn't** (bug) -- `vec_remove_at` is the one
   confirmed instance so far.
2. **Intentionally caller-responsible, unsafe-by-design** (not a bug,
   possibly worth a doc/lint) -- e.g. `str_byte_at` in `backend_c.rs`
   (~line 20741) is explicitly commented "Caller is responsible for
   bounds — out-of-range reads are undefined behavior (matches the
   safety contract of pointer arithmetic)" -- deliberate, matches the
   `unsafe_*`/`raw_load`/`raw_store` family's own contract.
3. **Already safe by a different mechanism** (not a bug) --
   `i64_byte_at` clamps out-of-range to a defensive `0` return rather
   than trapping (a value-level operation, not a memory access, so
   there's nothing to corrupt); `bptr_get`/`bptr_set` (the
   `BoundedPtr` family, `backend_c.rs` ~line 3220) return
   `Option`/`bool` on out-of-range rather than trapping OR reading OOB
   memory -- a deliberately different, already-safe API shape, not a
   gap.

Builtins worth checking next (not yet done this session, listed as
starting points, not confirmed either way): `deque_pop_back`/
`deque_pop_front`/`deque_peek_*` (empty-deque case), `heap_pop`/
`heap_peek`/`binary_heap_pop`/`binary_heap_peek` (empty-heap case),
`bst_min`/`bst_max`/`skiplist_min`/`skiplist_max` (empty-collection
case), `pool_get`/`pool_free` (handle validity, a different notion
than index bounds -- may already be handle-generation-checked, unread),
`aref_load`/`aref_store` (`ArenaRef`, unread), and the SIMD
`simd*_load`/`simd*_store` family (BUG-138 already fixed their index
*width*-mismatch class -- worth confirming their bounds-check
*presence* separately, since width and presence are orthogonal bugs,
as this round's `clone_at`/`vec_remove_at` pair demonstrates: BUG-138
was a width bug, BUG-147 is a presence bug, on largely disjoint
builtin sets).

## Process (mirrors rounds 1 through 3's own process sections)

1. Check `docs/TODO_LOCAL_STAGING.md` (in the `vani-compiler-localfuzz`
   worktree) for anything landed since this file's creation
   (2026-08-09) -- re-verify each against a freshly rebuilt `main`
   before trusting it (see `feedback_vani_localfuzz_staleness`).
2. Work category A's confirmed lead first (`vec_remove_at`) -- it's a
   same-shape sequel to BUG-147, should be fast.
3. Do the systematic sweep second -- grep every index-taking builtin
   across all four codegen files, classify into the three buckets
   above, fix anything in bucket 1.
4. Every fix gets a `src/lib.rs` compile-check test (one per affected
   codegen path) AND a `tests/run_end_to_end.rs` real-subprocess test
   (OOB traps cleanly + in-bounds still works, both backends) --
   established convention, "a clean pass still gets a permanent
   regression test."
5. Full `cargo test --release` clean + `vanic check examples` compared
   against the current baseline before any push. Verify freshness
   (`git fetch origin && git log origin/main --oneline -3`) before
   every commit -- a concurrent localfuzz process also lands commits.
6. CI/CodeQL polled green after every push.
