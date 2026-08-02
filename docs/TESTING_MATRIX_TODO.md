# Feature x backend testing matrix (2026-08-01)

Working list for a systematic sweep across tutorial feature areas x
backend x codegen path, done to hunt for bugs the existing test
suite doesn't already cover. Companion to `TODO_CURRENT.md` (which
tracks specific diagnosed bugs) -- this file tracks *coverage gaps*,
i.e. combinations nobody has actually exercised end-to-end yet.

## How backend/path selection actually works

There is no user-facing "ssa" flag. `vanic run/build/emit` only
takes `--backend=<c|llvm>`. Within each backend, `main.rs` tries the
SSA emitter first (`emit_c_via_ssa` / `emit_llvm_via_ssa`) and
silently falls back to the tree emitter (`backend_c.rs` /
`backend_llvm.rs`) whenever the program uses a feature the SSA path
doesn't cover (`ssa_path_supports` in `src/main.rs`). So "SSA vs
tree" is not independently selectable -- it's a deterministic
function of *which language features a program uses*, per backend.

Forced to the **tree** backend on **both** backends when a function
uses any of: struct literals/field access, enum variants (payload
or not), tuples, `dyn` dispatch/coercion, `match` or `if` as an
*expression*, block expressions, `[T; N]` in return position,
`#[no_mangle]`, `eprint`, or most of the "container library"
builtins (`vec_map`/`vec_fold`/... , `hashmap_*`/`hashset_*`/
`btreemap_*`/`btreeset_*`, `union_find_*`, `binary_heap_*`,
`bloom_filter_*`, `bst_*`, `graph_*`, `trie_*`, `skiplist_*` -- see
the full list in `src/main.rs`'s `expr_ssa_supported`).

Attempted via **SSA** first on **both** backends: scalars, `Var`/
`Ref`/`RefMut`/`FnRef`, arithmetic, `if`/`while`/`for` as
*statements*, `Vec<T>` of scalars with basic ops, and (as of the
concurrency work) `Atomic`/`Mutex`/`Guard`/`RwLock`/`Channel`/
`parallel for`/single-block `task`+`join`.

Per-backend extra exceptions (forces tree on one backend only):

| Feature | LLVM | C |
|---|---|---|
| `f64_to_str_fixed`, `file_read_line`/`stdin_read_line` | tree | tree |
| `vec_with_capacity` | SSA (works) | tree (not ported) |
| `Vec<Atomic<T>>` / `Vec<Channel<T,N>>` (any nesting) | tree | SSA (works) |

Net effect: most of **intermediate** (structs, enums, dyn, generics
monomorphized into struct/enum shapes, closures with non-trivial
captures) and a good chunk of **advanced** (vtables, most container
builtins) always exercises the **tree** backends on both `--backend`
values -- the SSA emitters mostly only ever run for **beginner**-
level scalar/Vec-arithmetic code and the concurrency primitives.
That means "test both `--backend` values" already covers the
tree-vs-SSA axis for most subsystems; the exceptions above are the
only cells where backend x path actually gives 3+ distinct
codepaths worth testing separately (`vec_with_capacity`,
`Vec<Atomic|Channel>`).

## Existing coverage snapshot (grep heuristic, 2026-08-01)

Rough keyword-hit counts in `src/lib.rs` (in-process compile-only
tests) vs `tests/run_end_to_end.rs` (real `vanic` binary, both
backends, actual stdout/exit-code assertions -- the only tests that
can catch a backend-specific miscompile or runtime crash). Low e2e
counts are the actual gap; lib.rs count alone is not a substitute.

| Area | lib.rs hits | e2e hits | Read as |
|---|---:|---:|---|
| `dyn` dispatch | 74 | **0** | compile-checked only, never run |
| `vtable` (advanced/05_vtables.md) | 4 | **0** | compile-checked only, never run |
| `Channel<T,N>` | 127 | **0** | compile-checked only, never run |
| `fnptr` | 23 | **0** | compile-checked only, never run |
| `vec_map`/`vec_fold`/iterator-style builtins | 73 | **1** | mostly compile-checked only |
| `task_spawn`/`task_join` | 3 | 1 | thin |
| cross-compile / `--target=` | 4 | 1 | thin |
| `extern "C"` / FFI | 110 | 5 | thin relative to lib.rs weight |
| `union_find`/`binary_heap`/`bloom_filter`/`bst_`/`graph_`/`trie_`/`skiplist` | 401 | 8 | thin relative to weight |
| `big_o`/complexity annotations | 69 | 2 | thin |
| `hashmap`/`hashset`/`btree` | 644 | 26 | moderate |
| SIMD / `vec_with_capacity`/`vec_dot` | 217 | 22 | moderate |
| `no_mangle`/`no_std`/embedded | 142 | 16 | moderate |
| affine/`clone_at`/drop | 498 | 86 | good |
| SMT (`ensures`/`requires`/`invariant`/`prove`) | 1125 | 122 | good |
| `try`/propagation | 72 | 72 | good |

## Priority sweep list (this pass)

Ordered by (real-execution gap) x (blast radius if broken). Each
row gets a real `.vani` snippet run through `vanic run <f>` and
`vanic run <f> --backend=c`, output/exit-code compared against hand-
computed expected values, not just "does it compile."

1. ~~`dyn Iface` dispatch, `Vec<dyn Iface>`~~ -- **done 2026-08-01**:
   swept struct-field `dyn Iface` (two different Iface types on one
   struct, the historical L8 shape), `ref dyn Shape` params, and a
   3-way heterogeneous `Vec<dyn Shape>`. Both backends agree and
   match hand-computed values. No bug found; not yet promoted to a
   permanent e2e test (still 0 in the coverage table above) --
   worth adding one from this sweep's repro rather than leaving the
   subsystem at 0 permanent coverage.
2. ~~`Channel<T,N>` send/recv, including `Vec<Channel<T,N>>`~~ --
   **done 2026-08-01, found BUG-61**: bare scalar/struct-payload
   Channel send/recv was fine on both backends, but
   `Vec<Channel<T,N>>` accessed via `mut ref chans[i]` crashed LLVM
   (heap corruption -- hardcoded 24-byte/element malloc size vs. the
   real 80-byte struct) and failed to compile under C (channel
   struct typedef referenced before declaration). Fixed in commit
   `8551cac` with 5 new tests (2 lib.rs, 3 e2e) closing the e2e gap
   this row used to describe as 0.
3. ~~Function pointers (`fnptr`, `06c_fnptr_primer.md`) as values,
   params, and struct fields~~ -- **done 2026-08-02, not a bug**:
   `Vec<fn(i64)->i64>`, a struct field of fn-pointer type, and
   indirect calls through a local re-bound from a struct field all
   compute correctly on both backends. Fn pointers are Copy, so no
   affine-ownership interaction to trip over. Closes the 0-e2e gap.
4. Iterator-style Vec builtins (`vec_map`/`vec_fold`/`vec_filter`/
   `vec_zip_with`/...) chained together -- **1 e2e hit** despite
   heavy tutorial use (06b_iterators_primer.md, 15_math_rng.md).
5. `task`/`join` (concurrency task primitive, not `parallel for`) --
   thin coverage, and it's explicitly called out in `main.rs`
   comments as having SSA-vs-tree edge cases (multi-block bodies).
6. Cross-compile (`--target=`) combined with embedded/`no_std` +
   `#[no_mangle]` FFI export together (04b/04a/04c primers) -- thin,
   and BUG-44 was found exactly in this intersection.
7. Graph/trie/skiplist/union-find/bloom-filter/BST algorithmic
   builtins actually *run* end-to-end (not just compiled) -- thin
   relative to how much tutorial content leans on them
   (advanced/05b_advanced_collections.md).
8. `vec_with_capacity` under `--backend=c` specifically (the one
   documented per-backend SSA gap in the table above) -- confirm
   the tree-C fallback produces correct values, not just that it
   compiles.

## Nested / multi-level combinations (added 2026-08-02)

BUG-61 (item 2 above) wasn't a bug in `Channel<T,N>` alone, or in
`Vec<T>` alone -- it only existed at their *intersection*
(`Vec<Channel<T,N>>`). Every bug found and fixed in this file and in
`TODO_CURRENT.md` before this point was found by testing ONE feature
at a time; almost none of them tested two-or-three-feature nesting
deliberately. That's a real, systematic blind spot: per-backend
codegen for a container (`Vec`/`Array`/`Tuple`/`Struct`) frequently
special-cases "the element is a scalar" vs. "the element is a
Struct/Tuple/payloaded-enum" (see `vec_element_size_expr`'s own
match arms) and simply forgets any OTHER aggregate/handle-shaped
element type exists -- exactly the class of gap BUG-61 was. This
section tracks that axis explicitly: basic containers (Vec, Array,
Tuple, Struct fields) nested around intermediate/advanced handle
types (Channel, Mutex, RwLock, Atomic, Task, dyn Iface, closures),
and multi-level nesting of the containers themselves.

**Sweep method**: same as above (real `.vani` snippet, run through
both `--backend` values, output/exit-code checked against hand-
computed values) -- but each row is a *pairing*, not a single
feature. Prioritize pairings where one side is a "container whose
element-size/type-name logic is written per-shape" (Vec, Channel,
Array, struct fields) and the other is a handle/aggregate type,
since that's exactly where BUG-61 lived.

### Container x concurrency-handle nesting

- [x] `Tuple` containing a `Channel<T,N>` element (e.g.
      `(Channel<i64,4>, i64)`) -- **checked 2026-08-02, not a bug**:
      `ref pair.0` / `mut ref pair.0` (borrowing a TUPLE field, as
      opposed to a struct field) is cleanly and consistently
      rejected on both backends ("'ref' can only borrow a named
      variable or a struct field") -- a real v1 syntax gap
      (tuple-field ref targets aren't supported at all yet, unlike
      the diagnostic's own example text which only mentions struct
      fields), but NOT a backend-divergence bug since both backends
      agree. Not chased further; belongs in `docs/v1_limitations.md`
      as a language gap, not this bug-hunting file.
- [x] `struct { ch: Channel<T,N>, buf: Vec<i64> }` -- **found+fixed
      2026-08-02, BUG-61 follow-up #1**: identical "unknown type
      name" failure as bare `Vec<Channel<T,N>>`, one level up (the
      channel struct wasn't declared before the OWNING struct's own
      typedef). Fixed by emitting scalar-element channel/mutex/
      rwlock bundles right after struct forward declarations,
      before any struct body (including one with a Channel field).
- [x] `Array<Channel<T,N>, K>` (fixed-size array, not `Vec`) --
      **checked 2026-08-02, not a bug**: `mut ref arr[i]` on a
      `[Channel<i64,4>; 2]` is cleanly and consistently rejected on
      both backends ("requires 'arr' to be a Vec... got
      [Channel<i64,4>; 2]") -- index-borrow builtins are Vec-only in
      v1, arrays aren't supported as their target at all. Both
      backends agree; not a divergence bug.
- [x] `Vec<Mutex<T>>` / `Vec<RwLock<T>>` specifically -- **found+
      fixed 2026-08-02, BUG-61 follow-up #2**: NOT the same bug as
      Channel -- `c_element_storage` (struct-field declarators) had
      no arms at all for Mutex/Guard/RwLock/ReadGuard/WriteGuard,
      falling through to `c_leaf_type`'s hardcoded (and wrong)
      `intent_mutex_i64`-style placeholders. Fixed with 5 new arms
      delegating to the already-correct per-type storage helpers.
- [x] `Channel<StructWithVecField, N>` -- **found+fixed 2026-08-02,
      BUG-64**: a genuine soundness gap, not just a codegen bug --
      `is_supported_channel_element` accepted ANY struct/enum
      unconditionally, with no Copy check, so a non-Copy payload's
      heap pointer got bytewise-duplicated into the ring buffer
      while the sender's original variable was still considered
      live -- a real, silent double-free at runtime on BOTH
      backends. Fixed by requiring Copy for struct/enum Channel
      elements. This was the most severe finding of the whole
      sweep (crashed with no warning at all, not even a compile
      error, before the fix).
- [x] `Vec<Task>` / storing `Task<R>` handles in a container before
      `join`-ing them in a loop -- **checked 2026-08-02, not a
      bug**: the checker rejects this pattern outright, cleanly and
      identically on both backends ("Cross-block joins are not
      supported in v1 -- the spawn and join must appear in the same
      statement list"). A genuine v1 architectural limitation (the
      affine task-tracker requires spawn+join in one block), not a
      divergence bug.

### Container x dyn / closure nesting

- [x] `struct { shape: dyn Shape, tag: i64 }` inside a `Vec<...>`
      (a Vec of structs that themselves hold a `dyn Iface` field --
      two levels of indirection: Vec -> struct -> fat pointer) --
      **checked 2026-08-02, not a bug**: `dyn Iface`'s fat pointer
      is Copy (two plain pointers, no owned allocation of its own),
      so the struct is entirely Copy and `items[i].shape.area()`
      indexes directly with no clone_at/ref needed. Both backends
      agree and match hand-computed values.
- [x] `Tuple` containing a `dyn Iface` element -- **found+fixed
      2026-08-02, BUG-65**: a self-inflicted regression from
      BUG-63's own fix (the new early-tuple-bundle path didn't
      defer `dyn Iface`-containing tuples past `emit_dyn_iface_
      typedefs`). Fixed in the same sweep.
- [x] `Vec<FnPtr>` -- **checked 2026-08-02, not a bug**: correct on
      both backends. See priority item 3 above.
- [x] Closure capturing a `Vec<T>` by move, stored in a struct
      field, called later -- **split finding, 2026-08-02, BUG-66**:
      the Copy-only-capture case (found broken as a byproduct --
      typedef-ordering, same shape as BUG-61/63) is now fixed and
      fully correct on both backends. The actual "capture a `Vec<T>`
      by move" case is a DEEPER, unfixed gap: crashes on both
      backends (LLVM: unsized-type IR rejection; C: double-free) --
      an affine-ownership problem, not a codegen-ordering one.
      Documented as a known gap in TODO_CURRENT.md's BUG-66 entry,
      not chased further this pass (would need checker-level
      rejection, mirroring BUG-64's Channel-Copy-requirement fix).

### Multi-level container nesting (no concurrency/dyn involved)

- [x] `Vec<Vec<Struct>>` (three levels: Vec of Vec of user struct) --
      **checked 2026-08-02, not a bug**: both backends agree and
      match hand-computed values (accessed via `clone_at`, since
      `Vec<Struct>` elements are non-Copy).
- [x] `Array<Tuple<...>, N>` -- **checked 2026-08-02, not a bug**:
      `[(i64,i64); 3]` indexed directly (tuples of Copy scalars are
      themselves Copy) works correctly on both backends.
- [x] `Vec<Array<Struct, N>>` -- **found+fixed 2026-08-02, BUG-62**:
      FOUR independent bugs (3 tree-C, 1 tree-LLVM), all specific to
      a `Vec<[T;N]>` whose element `T` is non-trivial, some only
      triggered when the array element is built from named
      variables rather than inline literals. See TODO_CURRENT.md's
      BUG-62 entry for the full breakdown -- this was the richest
      single repro of the whole sweep.
- [x] `struct { items: Vec<(i64, OwnedStr)> }` -- **found+fixed
      2026-08-02, BUG-63**: tree-C only. A Tuple shape that ONLY
      ever appears inside a struct field was never collected into
      `tuple_shapes` at all (only function signatures/bodies were),
      so its bundle was never emitted, while the eagerly-emitted
      struct-field `Vec<Tuple>` bundle referenced it regardless.
      Fixed with the same early/late partition pattern as BUG-61.
- [x] `HashMap<i64, Vec<Struct>>` -- **checked 2026-08-02, not a
      bug**: `hashmap_insert`/`hashmap_get` cleanly and consistently
      reject any non-scalar `V` on both backends ("hashmap_insert()
      supports scalar V in v1") -- matches the documented v1
      limitation in `intermediate/14_collections.md`. Not a
      divergence bug.

### Async / task x container nesting

- [x] `async fn` returning a `Struct` or `Vec<T>` -- **checked
      2026-08-02, not a bug**: both a Copy struct return and a
      `Vec<i64>` return through `Future<R>` compute correctly on
      both backends, for the no-suspend-point (synchronous desugar)
      case my repro exercised. `advanced/01_async.md`'s roadmap
      table's "Future<R> for scalar R... [queued for] v3.1" line
      turns out to describe the REAL multi-block suspend-point
      state machine specifically, not the v1 synchronous-desugar
      path -- not re-tested with an actual `.await` mid-body given
      this file's own "Non-goals" section already excludes deep
      async/await re-litigation.
- [x] `Barrier` combined with a `Vec<Mutex<T>>` shared-state pattern
      -- **checked 2026-08-02, not a bug**: three tasks (main +2
      spawned) each lock their own indexed `mutexes[i]` slot, set a
      value, then `barrier_wait`; a deterministic post-join summed
      check (not print-ordering, which is inherently racy across
      threads) confirms the correct total on both backends.

This closes every row in this section -- see the header note above
for the summary and the one real deferred gap (BUG-66's heap-
capturing closure case).

Cross off each row with the commit that added its e2e test (bug or
no bug found -- a clean pairing still earns a permanent regression
test, since it's exactly the kind of coverage that was missing
before BUG-61 was found).

## Non-goals for this pass

- Re-testing SMT/`ensures`/`try`/affine -- already well covered
  (see table); revisit only if a sweep item above turns up a
  related regression.
- Async/await -- has a documented, already-known LLVM/Windows gap
  (advanced/01_async.md); not re-litigating that here.
- Exhaustively enumerating every tutorial code sample individually.
  This file tracks *subsystems*, not a line-by-line tutorial replay.

## Process

For each item found broken: diagnose root cause in the compiler (not
the test), fix it, add both a `src/lib.rs` compile-time test and a
`tests/run_end_to_end.rs` real-binary test (both backends), run the
full `cargo test --release --workspace` suite, record the bug in
`docs/TODO_CURRENT.md` with the fix commit, and update any tutorial
page whose text turns out to assume the broken behavior.
