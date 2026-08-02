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

## Container operations x intermediate/advanced feature nesting (added 2026-08-02)

The section above swept containers (Vec/Array/Tuple/Struct) against
*concurrency handles and dyn/closures*. BUG-61 through BUG-67 all
lived at exactly those intersections. This section is the same idea
applied to a DIFFERENT, so-far-unswept axis: containers (Vec, Array,
Tuple, Struct, **Enum** -- the previous pass barely touched Enum
payloads) crossed with the *rest* of the intermediate/advanced
feature surface -- SMT contracts, generics/monomorphization, pattern
matching depth, FFI, affine edge cases, Big-O annotations,
`parallel for`, SIMD, `Option`/`Result` wrapping. None of these
pairings were part of the sweep that found BUG-61-67, and the same
root-cause pattern (a per-shape codegen helper with an arm for the
common case but not this one; or ordering between two independently-
emitted pieces) is exactly as likely to recur here.

**Sweep method**: unchanged from above -- real `.vani` snippet, run
through both `--backend` values, output/exit-code checked against
hand-computed values. Prioritize pairings where a container's
element/field is itself something with its own emission machinery
(a generic instantiation, an enum payload, an FFI-boundary type),
mirroring exactly why `Vec<Channel<T,N>>` broke.

### Container x SMT contracts

- [x] `requires`/`ensures` referencing a `Vec<T>` parameter's
      contents (e.g. `requires v[0] > 0;`) combined with a `Struct`
      or `Tuple` element type, not just scalar `i64` elements --
      **found+fixed 2026-08-02, BUG-68 -- a much more severe silent
      soundness gap than the row title suggests.** `Vec<Struct>`
      element field access (`pts[0].x`) in a contract is genuinely
      unsupported (array theory only models scalar elements) and now
      correctly REJECTS with a clear diagnostic -- but investigating
      why it wasn't already rejected uncovered the real bug:
      `verify_ensures_at_return` treated ANY `ensures` clause the SMT
      encoder couldn't fully encode (`Verdict::Unknown` /
      `Unavailable` / `SkippedUnsupported`) as silently PROVEN, with
      zero diagnostic -- the comment above the empty match arm even
      described a "fall back to constant-true check" that was never
      implemented. Every sibling contract-verification path (`prove`,
      loop invariants) already hard-errors on the same verdicts; only
      return-site `ensures` had this hole. Worse, this wasn't
      Vec-specific: it affected the general case of a bare struct
      parameter's field in `ensures`/`requires`/`prove`, since the
      existing struct-field synthetic-SMT-var machinery only covered
      locals initialized from a struct literal, never parameters --
      so `examples/edge_cases/mix_smt_struct_field.vani` and
      `mix_smt_pure_struct_ref.vani`'s ensures clauses (referencing a
      `ref R` param's fields) had *never* actually been checked by
      the solver despite reading as "verified" examples. Fixed two
      ways: (1) extended the struct-field-to-SMT-var rewrite so ANY
      struct-typed binding (parameter, `ref`/`mut ref`, or non-
      literal local) gets a free/opaque per-field SMT constant
      declared, not just literal-initialized locals -- this makes
      `ref` struct param field access in contracts *genuinely*
      verifiable, not just gracefully rejected; (2) made
      `verify_ensures_at_return` push a real diagnostic on
      Unknown/Unavailable/SkippedUnsupported, mirroring the loop-
      invariant path. Re-running the solver on the now-real
      `mix_smt_pure_struct_ref.vani` example immediately caught an
      actual latent bug in that example's own contract (`r.hi - r.lo`
      can overflow i64 for extreme `lo`/`hi`, violating `_return >=
      0`) that had been silently rubber-stamped before -- fixed by
      adding overflow-bounding `requires` clauses, the same pattern
      `12a_smt_primer.md` already teaches. `Vec<Struct>`/`Tuple`
      element-field access in contracts remains a real, cleanly-
      rejected v1 gap (not modeled by array theory) -- confirmed
      consistent on both backends since SMT verification runs before
      backend selection. New tests: 4 `src/lib.rs` (silent-accept
      regression, true-case discharge, Vec<Struct>-rejected-not-
      silent, plus the pre-existing struct-literal tests re-verified
      clean).
- [x] `invariant` in a loop that mutates a `Vec<Struct>` element
      in place (via `mut ref vec[i]`-style access) -- the
      loop-invariant-preservation machinery (BUG-33's territory)
      has only been exercised with scalar loop state so far --
      **checked+fixed 2026-08-02, BUG-68 follow-up.** In-place
      `Vec<Struct>` element mutation via `mut ref cs[i]` alongside a
      scalar-only loop invariant compiles and runs correctly on both
      backends (new e2e test) -- an invariant that tries to
      reference the mutated element's field directly (`invariant
      cs[0].n >= 0;`) is cleanly rejected on both backends
      ("structs not supported in SMT v1"), same as the row above:
      array theory doesn't model struct/tuple elements, not a
      divergence bug. But testing the "scalar loop state" half of
      this row's own premise against a plain (non-Vec) struct-typed
      accumulator surfaced a real second bug: `walk_for_reassigns`
      (builds the body-entry→body-end substitution map for
      preservation checking) only recognized `Stmt::Assign`/
      `Stmt::Let`, never `Stmt::FieldAssign` -- so `invariant
      acc.total == i * 10;` with `acc.total = acc.total + 10;` in
      the body was incorrectly reported as "not preserved," because
      the substitution map had no entry for the mutated field and
      the solver fell back to the struct-literal's original value.
      Fixed by teaching `walk_for_reassigns` to record a
      `"name__field"`-keyed substitution for `FieldAssign` (mirroring
      the synthetic-var naming from the BUG-68 fix above) and
      teaching `substitute_expr`'s `FieldAccess` arm to consult that
      key. Confirmed both directions: the true invariant now
      verifies, and a deliberately false one (`i * 999` instead of
      `i * 10`) is still correctly rejected. New tests: 2
      `src/lib.rs` + 1 `tests/run_end_to_end.rs` (both backends).
- [x] `ensures` on a function returning `Option<Vec<T>>` or
      `Result<Struct, E>` -- contract checking through an enum
      wrapper around a container return type -- **checked+fixed
      2026-08-02.** An `ensures` clause that tries to inspect the
      returned enum's shape (`ensures option_is_some(_return);`
      doesn't even type-check -- the `option_*` combinator builtins
      are hardcoded `Option<i64>`/`Option<f64>` only, a real, separate
      v1 restriction unrelated to this row; a `match _return { ... }`
      form type-checks but hits "method calls not supported in SMT
      v1" -- cleanly rejected thanks to the BUG-68 fix above, not
      silently accepted) is a genuine, cleanly-rejected v1 gap
      (enum-variant patterns aren't SMT-modeled, per the SMT encoder's
      own existing comments). NOT a divergence bug. But testing the
      row's actual container/runtime angle (no SMT contract on the
      enum payload, just `Option<Vec<T>>`/`Result<Struct,E>` used for
      real) found **BUG-69**, unrelated to Option/Result/generics
      entirely: `vec_fill` called anywhere textually AFTER a plain
      `if` statement in the same function crashed the LLVM backend
      with "PHI node entries do not match predecessors!" --
      `TypedStmt::If`'s tree emitter never updated `ctx.current_block`
      to the merge/cont block (every other multi-block construct
      already did), so `vec_fill`'s hand-rolled SSA fill loop (the one
      builtin that reads `ctx.current_block` to name its own loop
      phi's predecessor) wired itself to a stale block name. Fixed by
      setting `ctx.current_block = cont_lbl` after the if. C backend
      unaffected (no phi/SSA bookkeeping there). New tests: 1
      `src/lib.rs` (textual phi-predecessor assertion) + 1
      `tests/run_end_to_end.rs` (both backends, the actual
      Option<Vec<T>>/Result<Struct,E> scenario end-to-end).

### Container x generics / monomorphization

- [x] A generic struct `struct Box2<T> { items: Vec<T> }`
      instantiated at 2+ different `T` in the same program (the
      exact shape that broke for the built-in `Option`/`Result`
      generics in BUG-46 -- never re-tested for a *user-defined*
      generic struct wrapping a container) -- **found+fixed
      2026-08-02, BUG-46-class bug, confirmed for structs too.**
      `Env::resolve_struct_name` (the struct analog of the enum
      "exactly one candidate in the whole program" heuristic BUG-46
      patched) had the identical gap: the instant a second
      instantiation of the same generic struct exists anywhere in
      the program, a bare `Box2 { items: ... }` literal can't be
      disambiguated and EVERY construction site for that struct
      breaks with "unknown struct type 'Box2'" -- confirmed via a
      minimal repro with `Box2<i64>` + `Box2<OwnedStr>` side by side;
      a single instantiation alone compiled fine. Fixed the same way
      BUG-46 was fixed for enums: added
      `resolve_bare_struct_lits_in_stmt`/
      `resolve_bare_struct_lit_receiver`, rewriting a bare
      `StructLit.type_name` to its already-known concrete
      monomorphization at `let`/`return` sites (mirroring
      `resolve_bare_enum_ctors_in_stmt` exactly, wired into the same
      three call sites: plain functions, `implement` blocks, `methods
      on T` blocks). Both instantiations now construct and run
      correctly on both backends. New tests: 1 `src/lib.rs` + 1
      `tests/run_end_to_end.rs` (both backends).
- [x] A generic function `fn first<T>(xs: ref Vec<T>) -> T`
      monomorphized over a `Struct` T and a `Tuple` T in the same
      program -- **found+fixed 2026-08-02, two independent bugs
      (BUG-71, BUG-72), both general rather than container/generics-
      angle-specific.** BUG-71: generic inference through a `ref
      Vec<T>` parameter bound T to the WHOLE Vec instead of its
      element -- confirmed with a scalar-only repro FIRST (T=i64
      alone already broken), so this was never actually about
      Struct/Tuple specifically, just never tested for `ref Vec<T>`
      shaped params at all (`ref T` and by-value `Vec<T>` both
      already worked). Root cause: `infer_concrete_type_for_call`
      resolved a `ref`/`mut ref` call argument's referent type from
      scope but never re-wrapped it in `Type::Ref`/`RefMut`, so
      `unify_param_to_arg`'s `(Ref(p), Ref(a))` peeling arm could
      never match against a `Ref(Vec(Param(T)))` parameter shape and
      silently fell back to "T = whole arg type". Fixed by re-
      wrapping the resolved type before unification. BUG-72 (found
      immediately after, fixing BUG-71 was needed to even reach it):
      `type_mangle`'s Debug-based fallback didn't escape `[`/`]`, so
      a generic fn specialized over a Tuple T (whose `Type::Tuple`
      derived-Debug repr uses `[...]` for the element list) mangled
      to a name with literal brackets, crashing the LLVM backend
      ("expected '(' in call" -- not a valid identifier); C backend
      unaffected in the exact repro that found it. Fixed by adding
      `[`/`]` to the mangler's replacement set. Confirmed correct on
      both backends for scalar, Struct, and Tuple T through the same
      generic fn. New tests: 2 `src/lib.rs` + 1
      `tests/run_end_to_end.rs` (both backends).
- [x] `Vec<GenericStruct<i64>>` alongside `Vec<GenericStruct<f64>>`
      -- two different monomorphizations of the same generic struct
      both used as Vec elements (compounds BUG-61's exact bug class
      with generics) -- **found+fixed 2026-08-02, BUG-73, a BUG-70
      follow-up gap.** BUG-70's fix (`resolve_bare_struct_lits_in_stmt`)
      only rewrote a bare `StructLit` when it was the LET's own
      top-level RHS expression -- it never recursed into a `vec(...)`
      call's argument list. So `let vi: Vec<Box2<i64>> = vec(Box2 {
      val: 100 }, Box2 { val: 200 });` -- the single most natural way
      to write "a Vec of a generic struct" (matches every other
      `vec(literal, literal, ...)` pattern used throughout this
      codebase) -- still hit "unknown struct type 'Box2'" once a
      second `Box2<...>` instantiation existed, exactly this row's
      scenario. Fixed by (1) teaching `resolve_bare_struct_lit_receiver`
      to recurse into a `vec(...)` call's arguments (every arg shares
      the Vec's element type), and (2) adding a `Stmt::Let {
      annotation: Some(Type::Vec(Struct(target))), .. }` match arm
      alongside the existing bare `Type::Struct(target)` one. Both a
      Copy (`Box2<i64>`) and non-Copy (`Box2<OwnedStr>`) instantiation,
      each in its own `Vec`, now construct and run correctly on both
      backends -- no BUG-61-class element-size/free-helper bug found
      once construction itself worked. New tests: 1 `src/lib.rs` + 1
      `tests/run_end_to_end.rs` (both backends).

### Container x pattern matching / enum payload depth

- [x] An enum variant whose payload is itself a `Vec<Struct>` or a
      `Tuple` containing an `Array` (multi-level payload nesting,
      not just `enum { Variant(Vec<i64>) }`) -- **`Vec<Struct>`
      payload checked 2026-08-02, not a bug** (construction, tag-
      match dispatch, drop all correct on both backends, same v1
      destructure-binding restriction `Vec<i64>` payloads already
      document). **`Tuple<Array>` payload found+fixed 2026-08-02,
      BUG-74 -- THREE layered bugs in sequence.** (1) Checker: the
      enum-payload admission gate computed `payload_ty.is_copy()` for
      the whole Tuple, and `Type::Array::is_copy()` is unconditionally
      `false` by design (unrelated to payload safety) -- so `(i64,
      [i64; 3])` was rejected as "not admitted in v1" even though
      every element is stack/inline data, exactly as safe as any
      other admitted Copy payload. Fixed with a `tuple_of_admitted`
      check mirroring the existing `array_of_copy` special case one
      level deeper. (2) C backend: `emit_array_typedefs_for` never
      recursed into `Type::Tuple` elements, and its one call site
      only walked the Vec-element axis (never enum payloads or tuple
      elements) -- so `intent_arr3_int64_t` was referenced by the
      tuple bundle before ever being declared ("unknown type name").
      Fixed by adding Tuple recursion and moving the array-typedef
      pass earlier (before early-tuple-bundle emission), sharing one
      `seen` set with the original pass to avoid double-emission.
      (3) C backend: even with the typedef declared,
      `TypedExprKind::Tuple`'s C emission didn't special-case an
      Array-typed element with an inline `ArrayLit` initializer the
      way `StructLit` already did -- gcc rejects assigning a cast
      compound-literal array to a struct member of array type. Fixed
      by mirroring StructLit's existing bare-brace special case for
      Tuple. Bug (3) is general, not enum-specific -- confirmed a
      bare local `let x: (i64, [i64;3]) = (42, [1,2,3]);` (no enum at
      all) hit the identical C-backend crash before the fix. New
      tests: 4 `src/lib.rs` + 3 `tests/run_end_to_end.rs` (both
      backends).
- [x] `match` over a `Vec<Enum>` where the enum has 3+ variants with
      different payload shapes (mixed Copy/non-Copy payloads in the
      same enum, iterated) -- **found+fixed 2026-08-02, BUG-75, a
      real backend DIVERGENCE (not just an LLVM-only crash).**
      `clone_at` on a `Vec<Enum>` element -- where the enum has 3+
      variants spanning genuinely different payload types (`Num(i64)`,
      `Text(OwnedStr)`, `Flag(bool)`, `Nothing`) -- silently corrupted
      every SCALAR payload on the LLVM backend: `Num(7)` cloned as
      `Num(0)`, `Flag(true)` cloned as `Flag(false)`. C was correct
      throughout. Two layered root causes: (1)
      `LLVM_ENUM_PAYLOAD_REGISTRY` stored the FIRST payload type found
      across variants (not "does any variant have OwnedStr"), so when
      a scalar-payloaded variant was declared before the OwnedStr one,
      `clone_at`'s OwnedStr-deep-clone detection missed it entirely
      and fell back to a "tag-only" path that discarded the payload
      for EVERY variant, not just the missed one. (2) Fixing #1
      newly reached a second, previously-dormant bug: for a genuinely
      mixed-payload-type enum the payload field's real LLVM type is a
      byte buffer `[N x i8]`, not `i8*` -- the pre-existing deep-
      clone-as-string code assumed `i8*` and hit a straight LLVM type
      mismatch the instant it was actually exercised. Fixed by (1)
      computing the OwnedStr-tag set from the per-variant payload
      registry instead of the single-type one, and (2) rewriting the
      deep-clone path to operate through pointers (GEP + bitcast to
      `i8**`) instead of SSA-value extract/insert -- works for both
      the uniform-`i8*` and mixed-`[N x i8]` representations and
      naturally preserves every other tag's raw payload bytes. New
      tests: 1 `src/lib.rs` + 1 `tests/run_end_to_end.rs` (both
      backends, hand-computed expected sum across all 4 variant
      shapes).
- [x] Nested `if let` destructuring two levels deep where the OUTER
      binding is a Vec element (`if let Option.Some(Result.Ok(v)) =
      clone_at(ref xs, i)` style) -- combines BUG-46's enum-ctor
      resolution area with container indexing -- **checked+fixed
      2026-08-02.** A genuinely nested pattern
      (`Option.Some(Result.Ok(v))` in one `if let`) is cleanly
      rejected at PARSE time ("expected ')' (pattern binding close)")
      on both backends, matching the already-documented v1 limitation
      (`intermediate/02_enums_payloads.md`: "No nested patterns...
      flatten with two match levels"). Not a divergence bug -- the
      parser rejects it before any backend split happens. The
      flattened two-level form (two separate `if let` statements, per
      that doc's own guidance) combined with the outer binding coming
      from a `Vec<Option<UserEnum>>` element via `clone_at` found
      **BUG-76**, but the minimal repro turned out to have NOTHING to
      do with Vec/clone_at/if-let nesting: `let z: Option<MyResult> =
      Option.None;` alone (`MyResult` a user-defined payloaded enum)
      already crashed the LLVM backend. Same bug class as BUG-29
      (`Str`) / BUG-35 (`Box<T>`/raw pointers), documented right next
      to this fix in the source: the payload-less `None` variant's
      zero-value placeholder (for the sibling `Some(T)` variant's
      payload type) is built from a per-type match that was simply
      missing an arm for `Type::Enum(_)`, falling through to the
      default `"0"` -- invalid LLVM IR for an aggregate type
      ("integer constant must have integer type"). Fixed by adding
      `Type::Enum(_)` to the existing `zeroinitializer` arm alongside
      Vec/Tuple/Struct/Array/Task/Mutex/Channel. New tests: 1
      `src/lib.rs` + 1 `tests/run_end_to_end.rs` (both backends, full
      flattened-nesting scenario).

### Container x FFI / extern boundary

- [x] `extern "C" fn` taking or returning a `Struct` BY VALUE
      (not just scalars/`Str`/`ref T` -- the FFI tutorials only show
      scalar marshaling) -- **found+fixed 2026-08-02, BUG-77.** A
      small struct passed BY VALUE as a PARAMETER to a real, linked
      extern C function already worked correctly on both backends
      (Closure #288's System V x86-64 ABI lowering: bitcast to a
      packed-register form at the call site). A small struct
      RETURNED by value from such a function crashed the LLVM
      backend the instant it was actually CALLED (declaring it alone
      was fine -- confirmed via the pre-existing
      `extern_small_struct_lowers_to_packed_integer_in_llvm` test,
      which only exercised the param side): `'%tN' defined with type
      'i64' but expected '%Struct_Point'`. Root cause: the `call`
      instruction correctly used the lowered return type (`i64`) to
      match the real C ABI, but the resulting SSA value was then
      handed to every downstream consumer (a `let` binding's
      `store`, a field read, ...) as if it already had the real
      `%Struct_X` type -- the param-passing side had a matching
      "lower before the call" step; the return side never got the
      mirror-image "un-lower after the call" step. C backend was
      unaffected throughout. Fixed by spilling the lowered value to
      an alloca, bitcasting to the real struct pointer type, and
      loading, right after the call returns. Confirmed against a
      REAL linked C shim (both directions: a function taking a
      struct by value AND one returning one) on both backends, not
      just a compile-only check. New tests: 1 `src/lib.rs` (LLVM IR
      shape assertion) + 1 `tests/run_end_to_end.rs` (both backends,
      real `--link-with` C shim, `vanic build` for LLVM + `vanic run
      --backend=c` for C).
- [x] A `#[no_mangle]` exported function whose signature includes a
      `Tuple` or fixed-size `Array` parameter (BUG-44's `#[no_mangle]`
      + SSA-gating fix was scalar-only; never re-tested with an
      aggregate param) -- **checked 2026-08-02, not a bug.** A
      `#[no_mangle]` fn taking a `(i64, i64)` Tuple and one taking a
      `[i64; 4]` Array both keep their bare symbol name (`sum_pair`/
      `sum_arr`, not `fn_sum_pair`/`fn_sum_arr`) on both backends
      (confirmed directly in the emitted IR/C) and compute correct
      values. New tests: 1 `src/lib.rs` (bare-symbol assertion on
      both backends' emitted output) + 1 `tests/run_end_to_end.rs`
      (both backends, real runtime values).

### Container x affine ownership edge cases

- [x] Partial move out of a struct field that's itself a `Vec<T>`,
      followed by `clone_at` on a DIFFERENT field of the same
      struct instance (interaction between partial-move tracking
      and `clone_at`'s own non-copy-element restrictions) --
      **checked 2026-08-02, not a bug.** Moving `h.xs` (a `Vec<i64>`
      field) out leaves `h.ys` (a `Vec<OwnedStr>` field) fully
      usable; `clone_at` on it computes correctly on both backends,
      with no double-free at scope exit (the moved-out field is
      correctly skipped by Drop, confirmed by the pre-existing
      `partial_move_field_extract_compiles` test's own C-output
      assertion). Reading `h.xs` again after the move is still
      correctly rejected ("field 'h.xs' was moved; cannot use after
      move"), confirming the tracking is real, not just permissive.
      New tests: 1 `src/lib.rs` + 1 `tests/run_end_to_end.rs` (both
      backends).
- [x] `clone_at` chained three levels deep:
      `clone_at(ref outer, i)` where `outer: Vec<Vec<Struct>>`,
      then `clone_at` again on the result -- **checked 2026-08-02,
      not a bug.** Both clone_at calls compute correctly on both
      backends, and `outer`'s own length stays unaffected by cloning
      out of it (confirms no aliasing between `outer`/`middle`). New
      tests: 1 `src/lib.rs` + 1 `tests/run_end_to_end.rs` (both
      backends).
- [x] A struct with BOTH a `Vec<T>` field and an `OwnedStr` field,
      partially moved (one field taken, the other read), then
      dropped -- confirm no leak/double-free on the un-moved field --
      **checked 2026-08-02, not a bug, verified with `valgrind
      --leak-check=full` (not just correct output).** Runs correctly
      on both backends; separately built native binaries from this
      exact program and ran them under valgrind memcheck: **0
      leaks, 0 errors, balanced alloc/free counts on both backends**
      (LLVM: 2 allocs/2 frees; C: 3 allocs/3 frees -- different
      internal representation, same "everything freed exactly once"
      outcome). This is the strongest verification method used in
      this sweep so far -- direct confirmation, not just inferred
      from correct printed values. New tests: 1 `src/lib.rs` + 1
      `tests/run_end_to_end.rs` (both backends; the valgrind run
      itself was a one-time manual check, not wired into CI).

### Container x Big-O / complexity annotations

- [x] A `#[complexity(...)]`-annotated function (or whatever the
      real attribute syntax is -- check `13a_big_o_primer.md`) whose
      body operates on `Vec<Struct>`/`Array<Tuple,N>`, not just
      `Vec<i64>` -- confirm the complexity analysis doesn't
      misclassify or crash on non-scalar element types --
      **checked+fixed 2026-08-02.** There's no `#[complexity(...)]`
      attribute in v1 -- `--big-o` is a whole-program `vanic check`
      flag, no per-function opt-in needed. The `--big-o` analyzer
      itself correctly classifies loops over `Vec<Struct>` (O(n) for
      a single loop, O(n²) for nested loops over the same Vec) with
      no crash or misclassification -- not a bug. But testing
      `Array<Tuple,N>` as a function PARAMETER (the natural way to
      write a Big-O-relevant helper taking a fixed-size aggregate
      array) found **BUG-78**, entirely unrelated to Big-O: ANY
      function taking `[Tuple; N]` or `[Struct; N]` BY VALUE crashed
      the C backend. `format_declarator`'s `Type::Array` arm (and its
      `ref`/`mut ref` siblings) called `c_leaf_type(element)`
      directly -- `c_leaf_type` is a LEAF-only spelling table that
      deliberately returns a placeholder comment ("/* tuple */", "/*
      struct */") for aggregate element types, documented there as
      "a caller forgot to route through the real per-shape name" (the
      same class of gap `c_element_storage` exists specifically to
      close, and which other declarator paths already use correctly).
      `fn sum_array_tuple(arr: [(i64, i64); 5])` declared itself as
      `/* tuple */ v_arr[5]` -- not valid C, "unknown type name
      'v_arr'". Fixed by routing all three arms (bare `Type::Array`,
      `Type::Ref(Array)`, `Type::RefMut(Array)`) through
      `c_element_storage` instead. LLVM backend was unaffected
      throughout. New tests: 1 `src/lib.rs` (placeholder-comment
      absence + real type-name assertion) + 1
      `tests/run_end_to_end.rs` (both backends, hand-computed sums
      over both an `Array<Tuple>` and an `Array<Struct>` parameter).

### Container x `parallel for` / SIMD

- [x] `parallel for` iterating a `Vec<Struct>` (not just
      `Vec<i64>`/`Vec<f64>`) with a `reduce` accumulating a struct
      field -- **checked 2026-08-02, not a bug.** Indexing a
      `Vec<Pt>` (Copy struct, scalar fields only) inside a `parallel
      for` body and reducing one of its fields with `+` compiles and
      computes correctly on both backends. New tests: 1 `src/lib.rs`
      + 1 `tests/run_end_to_end.rs` (both backends).
- [x] A struct with both a `Vec128<f64>`/`Vec256<f64>` SIMD field
      AND a plain `Vec<f64>` field in the same struct -- confirm
      the two very different Vec-family codegens (BUG-62's territory
      for one of them) don't collide on shared helper-naming --
      **found+fixed 2026-08-02, BUG-79, unrelated to the row's own
      naming-collision hypothesis.** `c_element_storage` (the
      function specifically responsible for giving struct fields/Vec
      elements/etc. their REAL per-shape C type, as opposed to
      `c_leaf_type`'s deliberate placeholder-comment fallback) simply
      never had arms for `Type::Vec128`/`Vec256`/`Vec512` at all --
      unlike Tuple/Struct/Closure/Channel/Mutex, which all already
      got this exact fix in earlier sweeps this session. A struct
      field of `vec128<f64>` type declared itself as `/* vec128<T>
      */ lane;` -- not valid C ("expected specifier-qualifier-list
      before 'lane'"). Fixed by adding the three missing arms,
      delegating to the already-correct `c_vec128_type`/
      `c_vec256_type`/`c_vec512_type` helpers (the same helpers the
      pre-existing LOCAL-variable case already used correctly). LLVM
      backend was unaffected throughout. No actual helper-naming
      collision found between the two Vec families once construction
      itself worked. New tests: 1 `src/lib.rs` (placeholder-comment
      absence + real GNU-vector-extension type assertion) + 1
      `tests/run_end_to_end.rs` (both backends, hand-computed SIMD
      reduce + plain Vec values).

### Container x `Option`/`Result` wrapping

- [x] `Option<Array<T,N>>` / `Result<Tuple<...>, E>` -- Option/
      Result generic instantiation (BUG-46's territory) has mostly
      been tested with scalar or single-struct payloads, not
      Array/Tuple payloads specifically -- **found+fixed 2026-08-02,
      BUG-80.** LLVM was correct throughout for both
      `Option<[i64;3]>` and `Result<(i64,i64), i64>`. `Result` over a
      Tuple payload already worked correctly on C too (Tuple's
      match-arm binding path was already correct). `Option` over an
      Array payload crashed the C backend specifically, two layered
      bugs: (1) the match-arm payload-binding codegen declared its
      local via `c_type_name(bty)`, whose `Type::Array` arm
      deliberately spells arrays as the RETURN-POSITION wrapper
      struct (`intent_arr_ret_<N>_<T>`) -- its own doc comment already
      warned "the Let path passes through `format_declarator`
      instead", but this match-arm binding (a Let-like position) was
      still calling `c_type_name` directly; the wrapper typedef was
      never even emitted for this position ("unknown type name"), and
      even if it had been, `v_arr[0]` wouldn't subscript a struct
      correctly anyway. (2) Switching to `c_element_storage` fixed the
      type name but exposed a SECOND issue: C arrays (even through a
      raw-array typedef) can't be copy-assigned via `=` at all
      ("invalid initializer"). Fixed by declaring the binding as a
      POINTER to the element type instead -- the raw array field
      naturally array-decays to a pointer to its first element in
      this expression context (valid C), and `v_arr[0]`/etc. in the
      arm body keeps working unchanged. Also needed emitting the
      `intent_arrN_T` typedef for an enum payload that's DIRECTLY
      `Array<T,N>` (not nested inside a Tuple, which BUG-74's fix
      already covered) -- walked `program.enums`'s payload types
      directly through the existing array-typedef pass. New tests: 1
      `src/lib.rs` + 1 `tests/run_end_to_end.rs` (both backends,
      Option<Array> Some/None AND Result<Tuple> Ok/Err all four
      paths, hand-computed values).
- [ ] A `Vec<Option<Struct>>` -- Option of a non-Copy struct, stored
      in a Vec (three-level: Vec -> Option -> Struct).

Cross off each row the same way as the section above: bug or no bug,
add a permanent regression test and note the outcome inline.

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
