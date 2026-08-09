# vāṇी — Bug-pattern audit, round 4

**STATUS (2026-08-09): category A CLOSED. Confirmed lead fixed as
BUG-148; the systematic sweep of the rest of the builtin surface
found no further bugs (clean negative result) -- round 4 is done.**
Sequel to `docs/BUG_PATTERN_AUDIT_TODO_3.md` (round 3, closed 2026-08-08
as BUG-146). This round's localfuzz backlog triage (5 fresh candidates,
2026-08-08 through 2026-08-09) found only one genuine bug --
`clone_at()`'s missing bounds check, fixed same-session as **BUG-147**
-- and the other 4 were non-bugs (a correct i64::MIN overflow trap, two
source-level infinite loops from mutation stripping a loop's increment,
one more instance of the already-characterized `sleep_ms(i64::MAX)`
pattern). See `docs/TODO_CURRENT.md`'s BUG-147 section for the full
fix writeup. `vec_remove_at`'s matching gap (below) was then fixed the
same day as **BUG-148** -- see `docs/TODO_CURRENT.md`'s BUG-148 section.

Rather than derive round 4's theme from a shape localfuzz hasn't found
yet, this file follows round 3's other lesson: BUG-147 itself pointed
at a *class*, not just one call site. A short manual code audit
(grep + direct reproduction, not waiting on the fuzzer) done in the
same session already found a second, still-unfixed instance of the
exact same class. That's category A below, and it's a running head
start, not a cold lead -- the next session should be able to open with
a fix, the same way round 3 did for its two categories.

## A. Bounds-check coverage audit across indexed-access builtins (🔴 high) -- CLOSED 2026-08-09, BUG-148 + systematic sweep, clean negative result

**Update (2026-08-09, sweep)**: the confirmed lead is fixed (BUG-148,
see below). The systematic sweep the writeup below left open is now
done, with a clean negative result -- no further bugs found. Method:

1. Grepped every `"..._at"` / `"..._nth"` string literal across all
   four codegen files (`backend_c.rs`, `backend_llvm.rs`,
   `ssa_backend_c.rs`, `ssa_backend_llvm.rs`) -- the exhaustive set is
   `clone_at` (fixed, BUG-147), `vec_remove_at` (fixed, BUG-148),
   `i64_byte_at`, `str_byte_at`. No other `_at`/`_nth`-suffixed
   builtin exists anywhere in the four files.
2. Walked the "Builtins worth checking next" list from the original
   writeup below, reading each implementation directly:
   - `deque_pop_back`/`deque_pop_front`/`deque_peek_back`/
     `deque_peek_front` -- **bucket 3**. All four return
     `Enum_Option__i64` with an explicit `if (d->len == 0) { r.tag = 1;
     ...; return r; }` guard (`backend_c.rs` ~line 2757-2785;
     LLVM preamble mirrors it, `backend_llvm.rs` ~line 40103-40181).
   - `heap_pop`/`heap_peek` (generic, per-type-helper path) and
     `binary_heap_pop`/`binary_heap_peek` (monomorphized i64 path) --
     **bucket 3**. Same `Option` + `if (xs->len == 0) {...None...}`
     shape (`backend_c.rs` ~line 12459-12480 for the generic path,
     ~line 5686-5696 for `binary_heap_i64`).
   - `bst_min`/`bst_max`, `skiplist_min`/`skiplist_max` -- **bucket
     3**. Same `Option`-on-empty shape (`backend_c.rs` ~line
     6138-6152, ~line 7483-7497).
   - `pool_get`/`pool_free` -- **bucket 3**, but via a different
     mechanism than the others: a handle carries `(slot_idx,
     generation)`; both C and LLVM (`backend_c.rs` ~line 3121-3140,
     `backend_llvm.rs` ~line 39518-39560) check `slot_idx >= len` OR
     stale `generation` and return `None` / no-op rather than
     touching memory. Confirmed identical on both backends.
   - `aref_load`/`aref_store` -- **bucket 2**, intentionally
     caller-responsible. The code comment right above them
     (`backend_c.rs` ~line 19572-19574) says explicitly: "same machine
     semantics as raw load/store but no Tainted wrapping (the
     compile-time scope binding is the safety proof)." No length is
     even passed in -- `args[0]` is a bare pointer, not a
     length-carrying collection, so there's nothing to bounds-check
     against; matches `str_byte_at`'s existing documented contract.
   - `simd_load`/`simd_store` (and the 256/512-bit variants) --
     **bucket 2**, intentionally caller-responsible, and explicitly
     documented as such in the tutorial
     (`tutorials/src/advanced/05_simd.md` line 175-176: "`simd_load`
     and `simd_store` access the **heap buffer** of a `Vec<T>`
     directly — no bounds checking, no copy of the fat pointer.").
     Distinct from BUG-138 (index *width* mismatch, already fixed);
     this is bounds-check *presence*, which this family deliberately
     omits by design.
3. Found one additional site not on the original list while grepping:
   `intent_vec_bool__get` (`backend_c.rs` ~line 11493, the packed-bit
   `Vec<bool>` accessor). The raw helper itself has no bounds check,
   but unlike `vec_remove_at`'s old bug, every call site
   (`emit_index`, `backend_c.rs` ~line 21230-21278) wraps the index in
   `intent_check_bounds(...)` before calling the helper when
   `checked` is true -- **bucket 3**, already safe, just via a
   caller-wraps-callee shape instead of callee-checks-itself.
4. None of these builtins have SSA-backend implementations
   (`ssa_backend_c.rs`/`ssa_backend_llvm.rs` have no match arms for
   any of them) -- confirmed via grep, only a stray comment mentions
   `heap_pop`/`heap_peek` in passing. Any program using these types
   falls back to the tree backend's `emit_c_via_ssa`/
   `emit_llvm_via_ssa` fallback path, so the sites audited above are
   the only sites that exist for this builtin family -- no third/
   fourth site was missed.

No code changes came out of this pass -- the sweep's value was
confirming (by reading, not assuming) that `clone_at`/`vec_remove_at`
were the only two gaps in this shape, and that the rest of the
indexed-access surface is either already safe or deliberately
unsafe-by-design and documented as such. Round 4 is closed; a future
round should pick a new theme rather than re-sweep this surface.

<details>
<summary>BUG-148 fix note (2026-08-09)</summary>

`vec_remove_at`'s index is now routed through `intent_check_bounds`
(`backend_c.rs`) / `@__intent_bounds_check` (`backend_llvm.rs`), the
same two sites the original writeup below pinpointed. Verified: the
OOB repro below now traps cleanly on both backends (C exit 134 with
`"index out of bounds: 99, len 3"`, LLVM exit 3) instead of returning
garbage; an in-bounds sanity check still shifts elements correctly.
Full writeup in `docs/TODO_CURRENT.md`'s BUG-148 section.

</details>

<details>
<summary>Original writeup, kept for the reasoning trail</summary>

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

</details>

## Process (mirrors rounds 1 through 3's own process sections)

1. Check `docs/TODO_LOCAL_STAGING.md` (in the `vani-compiler-localfuzz`
   worktree) for anything landed since this file's creation
   (2026-08-09) -- re-verify each against a freshly rebuilt `main`
   before trusting it (see `feedback_vani_localfuzz_staleness`).
2. ~~Work category A's confirmed lead first (`vec_remove_at`)~~ --
   done, fixed as BUG-148.
3. ~~Do the systematic sweep~~ -- done, 2026-08-09: every `_at`/
   `_nth` builtin plus the full "Builtins worth checking next" list
   classified, clean negative result (all bucket 2/3), no code
   changes needed. Category A is closed; see the sweep write-up above.
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

**Round 4 is closed.** A future session should pick a genuinely new
audit theme rather than re-sweep this builtin surface -- see round 3's
own closing note for the same guidance it followed.
