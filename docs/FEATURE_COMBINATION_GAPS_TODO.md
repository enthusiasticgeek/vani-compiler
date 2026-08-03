# Feature-combination gap audit (2026-08-02)

Critical inventory of vāṇी feature *combinations* that should work in
theory (every individual feature involved is shipped and documented)
but have never been exercised **together**, end-to-end, on both
backends. This is a fresh sweep layered on top of two already-closed
ones:

- `docs/TESTING_MATRIX_TODO.md`'s "Nested / multi-level combinations"
  and "Container operations x intermediate/advanced feature nesting"
  sections — **fully closed**, 38 rows, found BUG-61 through BUG-80.
  Read those first; this file deliberately does not re-list anything
  already covered there.
- `docs/missing_features.md`'s "Mixed-feature gaps" table — mostly
  stale (dated well before this session's fixes; most rows there are
  now marked ✅ Fixed). Its "worth probing" bullets at the bottom are
  folded into this file where still relevant.

**Why this file exists**: every bug found in the two closed sweeps
above lived at an *intersection*, not in a single feature tested
alone (BUG-61: `Vec<Channel<T,N>>`, not `Vec` or `Channel` alone;
BUG-68: `Vec<Struct>` field access in `ensures`, not `Vec` or
`ensures` alone). Single-feature coverage across the whole tutorial
set is now good. Combination depth — especially 3-feature nesting —
is where the remaining risk concentrates, per this session's own
repeated experience.

**Ground rule for what belongs here**: only combinations of features
that *individually already exist and are documented as supported*.
Things vāṇी doesn't have at all yet (const generics, `Rc<T>`, custom
`Drop`, HashMap with a struct key, macros — see
`docs/missing_features.md`'s main sections) are language gaps, not
test gaps, and are explicitly OUT of scope for this file.

**Sweep method** (unchanged from the prior two sweeps — reuse it):
for each row, write a real `.vani` snippet, run `vanic check`, then
`vanic run <f>` and `vanic run <f> --backend=c`, compare stdout/exit
code against a hand-computed expected value. Three outcomes: clean
pass (add a permanent `src/lib.rs` + `tests/run_end_to_end.rs` pair
either way), a clean/consistent rejection on both backends (not a
bug — document as a v1 limitation if not already documented), or a
crash / backend divergence / silent wrong answer (a real bug — root-
cause, fix, then add the regression tests). Every row gets checked
off with an outcome, per the established convention in the two prior
sweeps.

**Priority key**: 🔴 High (directly analogous to a bug class already
found this session — e.g. "container x element with its own codegen
path"), 🟡 Medium (plausible gap, moderate blast radius), 🟢 Low
(edge case, low blast radius, but still worth a clean-rejection
confirmation).

---

## 1. SIMD x containers/generics (🔴 highest priority — unswept analog of BUG-61)

The two closed sweeps thoroughly covered "container x concurrency-
handle" and "container x dyn/closure" nesting (where BUG-61's per-
element-type codegen dispatch bug lived) but **never touched SIMD as
a container element type**. `vec128<T>`/`vec256<T>`/`vec512<T>` have
their own element-size/storage logic (`c_vec128_type` etc., per this
session's own BUG-78/79 fixes to `c_element_storage`) — exactly the
shape of helper that forgot cases before.

- [x] `Vec<vec128<f64>>` — **found+fixed 2026-08-03, BUG-81 (two
      independent bugs, one per backend).** C: `element_tag` was
      missing the Vec128/256/512 arms (a separate function from
      `c_element_storage`, which BUG-79 already fixed), corrupting
      every generated bundle identifier. LLVM: `vec_struct_tag` had
      the identical gap (Rust panic), and fixing that alone revealed
      `vec_element_byte_size` under-counting a SIMD element's malloc/
      realloc size (`Type::bits()` returns `None` for SIMD types, so
      the fallback silently computed 8 bytes for a real 16/32/64-byte
      register) — heap corruption on push. Verified with valgrind on
      a native AOT LLVM build. See `docs/TODO_CURRENT.md`'s BUG-81
      entry for the full writeup.
- [x] `Array<vec128<f64>, N>` — **checked 2026-08-03, not a bug**:
      correct on both backends once BUG-81's fixes landed (shares the
      same underlying tag/size helpers).
- [x] `struct { lanes: Vec<vec256<f64>> }` — **checked 2026-08-03,
      not a bug**: correct on both backends.
- [x] `Tuple` containing a `vec128<T>` element, e.g. `(vec128<f64>,
      i64)` — **checked 2026-08-03, not a bug**: correct on both
      backends.
- [x] A generic struct `struct Wrapper<T> { v: T }` instantiated at
      `T = vec128<f64>` — **checked 2026-08-03, not a bug**:
      monomorphization accepts SIMD as a generic type argument and
      computes correctly on both backends.
- [x] `Option<vec128<f64>>` / `Result<vec128<f64>, E>` — **`Option`
      checked 2026-08-03, not a bug; `Result` found+fixed 2026-08-03,
      BUG-82, LLVM-only.** `Result<vec128<f64>, i64>` is a MIXED-
      payload-type enum (unlike `Option`, which has only one
      payloaded variant) — segfaulted `lli` on both construction and
      match-arm extraction due to a missing `align 1` on the byte-
      buffer bitcast load/store (LLVM assumed the SIMD payload's
      natural 16-byte ABI alignment against a buffer that only
      guarantees 4). Verified with valgrind covering both `Ok`/`Err`
      variants. See `docs/TODO_CURRENT.md`'s BUG-82 entry.
- [x] `HashMap<i64, vec128<f64>>` — **checked 2026-08-03, not a
      bug**: cleanly and consistently rejected on both backends,
      matching the documented "hashmap_insert() supports scalar V in
      v1" restriction.
- [x] SIMD field/element `clone_at` — `Vec<Struct>` where `Struct`
      has a `vec128<T>` field — **checked 2026-08-03, not a bug**:
      the `clone_at`/mutate/`set` idiom works correctly on both
      backends.

Category 1 fully closed. New tests: 10 `src/lib.rs` + 3
`tests/run_end_to_end.rs` (see `docs/TODO_CURRENT.md`'s BUG-81/82
entries). Full `cargo test --release --workspace`: 13/13 binaries
clean, 0 failed.

## 2. Generics x concurrency handles (🔴 high — untested direction of an already-fixed bug class)

BUG-19/22 fixed `RwLock<T>`/`Mutex<T>`/`Channel<T,N>` for arbitrary
CONCRETE `T` (struct, enum, i64) on both backends. Never tested: `T`
still being a **generic type parameter** at the point the handle is
declared (monomorphized later), as opposed to already being a
concrete struct.

- [x] `struct Cache<T> { lock: Mutex<T> }`, instantiated at two
      different `T` in the same program — **found+fixed 2026-08-03,
      BUG-83 (two layered bugs + a self-inflicted regression caught
      in the same pass) and BUG-84 (a separate, general `Mutex<bool>`
      bug surfaced by the T=bool instantiation).** A struct field
      holding a concurrency handle used ONLY through that field
      (never a bare local elsewhere) was never discovered by the
      bundle-collection passes on either backend; LLVM needed a
      second, cross-backend-registry fix on top. Fixing the
      discovery gap naively assumed struct field graphs are acyclic,
      which broke `Vec<Self>`-shaped structs (a real stack overflow,
      caught by a pre-existing pinned regression test) — fixed with a
      recursion guard. See `docs/TODO_CURRENT.md`'s BUG-83/84 entries
      for the full writeup.
- [x] `fn spawn_with_lock<T>(initial: T) -> Mutex<T>` — a generic
      function that itself constructs a `Mutex<T>`/`RwLock<T>`/
      `Channel<T,N>` from its generic parameter — **checked
      2026-08-03, not a bug**: correct on both backends.
- [x] `Task<T>` where `T` is a generic parameter of the enclosing
      generic function (`fn run<T>(x: T) -> Task<T>`) — **checked
      2026-08-03, blocked by a pre-existing, correctly-enforced v1
      limitation, not a new bug**: returning a spawned `Task` from a
      function and joining it in a DIFFERENT function/block hits the
      already-documented "spawn and join must be in the same
      statement list" restriction — applies identically with or
      without generics, so this row can't isolate a generics-specific
      behavior at all.
- [x] A generic function bounded `<T: Iface>` that spawns a `task`
      capturing a `T`-typed value — **checked 2026-08-03, not a
      bug**: the "task captures must be Copy" check correctly
      evaluates `T`'s Copy-ness PER MONOMORPHIZATION — the same
      generic function accepts a Copy instantiation (T=i64) and
      cleanly rejects a non-Copy one (T=OwnedStr) within the same
      program.

Category 2 fully closed. New tests: 6 `src/lib.rs` + 1
`tests/run_end_to_end.rs` (see `docs/TODO_CURRENT.md`'s BUG-83/84
entries). Full `cargo test --release --workspace`: 13/13 binaries
clean, 0 failed.

## 3. SMT contracts x generics / concurrency / enums (🟡–🔴 mixed — the closed sweep only tested SMT x Vec/Struct)

The closed "Container x SMT contracts" section has exactly two rows
(Vec/Struct field access in `ensures`, loop invariant on `Vec<Struct>`
element). Everything below is genuinely untested.

- [x] 🔴 `requires`/`ensures` on a **generic function** — **checked
      2026-08-03, not a bug**: discharges correctly per-
      monomorphization for a scalar instantiation (T=i64), and a
      second, non-scalar instantiation (T=Point) gets a clean,
      consistent rejection ("cannot verify 'ensures' clause: variable
      'x' has unsupported type Point for SMT"), not a silent skip —
      matches BUG-68's fix.
- [x] 🔴 `#[complexity(...)]` (Big-O analysis, via `vanic check
      --big-o`) and `requires`/`ensures` on the SAME function —
      **checked 2026-08-03, not a bug**: both analyses coexist
      correctly with no interference in either direction (Big-O
      still reports the right complexity class when SMT proves
      cleanly; a genuinely-violated `ensures` is still correctly
      rejected with `--big-o` enabled).
- [x] 🟡 `invariant` in a loop that also touches a `Mutex` inside the
      loop body — **found+fixed 2026-08-03, BUG-85 and BUG-86 (two
      severe, unrelated bugs found investigating this row, neither
      actually about SMT/invariants at all)**. The row's own premise
      (does the invariant checker interfere with concurrency
      primitives in the loop body) checked out clean once both bugs
      were fixed — but getting a working repro at all required a
      BARE, SSA-eligible Mutex (no structs/block-expressions forcing
      tree), which had never been tested end-to-end all session and
      turned out to be completely broken on the C backend: (1) BUG-85
      — the SSA-C emitter has its own, never-updated-since-BUG-19
      hardcoded Mutex/Guard implementation, so it failed to compile
      at all; (2) BUG-86 — once fixed, a program with two SEQUENTIAL
      lock/unlock cycles on the same mutex (through a block-
      expression, confirmed PRE-EXISTING via an isolated git
      worktree check against the pre-sweep commit) hung FOREVER — the
      tree-C block-expression Drop emitter has no arm for
      Guard/ReadGuard/WriteGuard at all, so the RAII unlock silently
      never fired. See `docs/TODO_CURRENT.md`'s BUG-85/86 entries.
- [x] 🟡 `ensures`/`prove` referencing an enum's variant tag directly
      (e.g. `ensures _return != Option.None;`) — **checked
      2026-08-03, not a bug**: cleanly and consistently rejected
      (enum `==`/`!=` requires an `Eq` impl at the language level;
      separately, the SMT layer correctly reports "method calls not
      supported in SMT v1" rather than silently skipping).
- [x] 🟢 `prove` statement referencing a `dyn Iface` method's return
      value — **checked 2026-08-03, not a bug**: cleanly rejected
      ("method calls not supported in SMT v1"), not silently treated
      as proven.

Category 3 fully closed. New tests: 2 `src/lib.rs` + 2
`tests/run_end_to_end.rs` (see `docs/TODO_CURRENT.md`'s BUG-85/86
entries — one test wraps its invocation in the real `timeout` command
so a future regression of the BUG-86 deadlock fails cleanly instead
of hanging the suite). Full `cargo test --release --workspace`:
13/13 binaries clean, 0 failed.

## 4. Async/await x everything else (🟡 — the async chapter's own roadmap flags several as "queued," worth confirming the boundary is a clean rejection, not a crash)

The closed sweep tested async x plain struct/Vec return (not a bug,
synchronous-desugar case only). Nothing below was tested.

- [x] 🟡 `async fn` that is ALSO generic: `async fn foo<T>(x: T) ->
      T` — **found 2026-08-03, BUG-87 (NOT fixed, deferred given
      async-internals risk).** Broken two ways: calling it directly
      inside `await(...)` fails monomorphization outright (the
      call-site scanner doesn't look inside `await`'s argument);
      pre-extracting to a `let` first gets past that but then hits a
      second bug in `await`'s own desugared match dispatch. See
      `docs/TODO_CURRENT.md`'s BUG-87 entry for the full root-cause
      writeup (likely: `Future<T>`'s `Type::Apply` construction in
      `parser.rs` was never wired into the same monomorphization
      pipeline `Option<T>`/`Result<T,E>` use).
- [x] 🟡 `async fn` returning `Option<T>`/`Result<T,E>` specifically —
      **found 2026-08-03, same BUG-87 root cause.** Fails with "match
      arm body has type i64 but earlier arm produced Option__i64"
      when a user `match` interacts with `await`'s own desugared
      match over the result.
- [x] 🟡 `async fn` with a `requires`/`ensures` contract — **checked
      2026-08-03, not a soundness bug**: a clean, SAFE rejection
      ("cannot verify 'ensures' clause: method calls not supported in
      SMT v1"), since `_return` for an async fn is the desugared
      `Future.Ready(expr)` constructor call, which reads as a method
      call to the SMT encoder. A real functional limitation (SMT
      contracts don't work on `async fn` at all), but matches BUG-68's
      "unverifiable means rejected, never silently accepted" fix —
      not unsound.
- [x] 🟢 `async fn` taking a `Closure(...)` parameter and calling it
      before vs. after an `.await` point — **checked 2026-08-03, not
      a bug**: correct on both backends (tested with a plain
      `fn(T) -> R` function pointer parameter specifically).
- [x] 🟢 `async fn` spawning a `task` internally (async x the OTHER
      concurrency primitive family) — **checked 2026-08-03, not a
      bug**: correct on both backends.

Category 4 closed with one real bug found and DELIBERATELY left
unfixed (BUG-87 — async internals are explicitly sensitive,
partially-shipped machinery per this session's own BUG-45 precedent;
see `docs/TODO_CURRENT.md` for the full reasoning and a starting
point for whoever picks it up). No regression tests added for BUG-87
since it documents currently-broken behavior, not a boundary to pin.

## 5. dyn dispatch x generics (🟡 — two different polymorphism mechanisms meeting)

- [x] 🔴 `Vec<dyn Iface>` where the underlying concrete types are
      themselves DIFFERENT INSTANTIATIONS of the same generic struct
      — **found+fixed 2026-08-03, BUG-89.** `expand_blanket_impls`
      appends a concrete impl per monomorphization to `program.impls`
      but never removes the original blanket-impl template, so
      whatever builds the `dyn Iface` vtable/trampoline set generated
      a bogus extra trampoline for the unresolved generic template —
      crashed both backends. Fixed with a one-line `retain`, mirroring
      the established pattern already used for generic functions/
      structs/enums. See `docs/TODO_CURRENT.md`'s BUG-89 entry.
- [x] 🟡 A generic function bounded `<T: Iface>` that also accepts a
      `dyn Iface` parameter of the SAME interface in a different
      parameter slot — **checked 2026-08-03, not a bug**: correct on
      both backends.
- [x] 🟢 A struct implementing TWO DIFFERENT interfaces, with a
      SINGLE instance referenced through both `Vec<dyn IfaceA>` and
      `Vec<dyn IfaceB>` at once — **checked 2026-08-03, not a bug**:
      correct on both backends.

Category 5 fully closed. New tests: 1 `src/lib.rs` + 1
`tests/run_end_to_end.rs` (see `docs/TODO_CURRENT.md`'s BUG-89 entry).
Full `cargo test --release --workspace`: 13/13 binaries clean, 0
failed.

## 6. Error propagation (`try`/`?`) x containers/generics (🟡) -- CLOSED 2026-08-03

- [x] 🟡 `try`/`?` inside a function whose body also indexes/mutates
      a `Vec<Struct>` or `HashMap` — does the early-return desugar's
      drop-sequence correctly account for a live container binding
      (mirrors BUG-45's now-fixed OwnedStr-parameter case, but for a
      LOCAL Vec/HashMap rather than a parameter). Checked clean on
      both backends (not a bug) -- `valgrind --leak-check=full`
      clean on native AOT builds of both backends. See BUG-90 in
      `docs/TODO_CURRENT.md`.
- [x] 🟡 `try`/`?` inside a GENERIC function `fn foo<T>(...) ->
      Option<T> { let x = try bar::<T>(...); ... }`. Found+fixed a
      real bug (BUG-90: four compounding missing-arm/collapse gaps
      in the generics-monomorphization pipeline, the try-desugar
      producing `Match`/`Block` shapes those walkers never
      anticipated). While testing this row also found a SEPARATE,
      deeper, pre-existing bug independent of `try` entirely (a bare
      generic call used directly as a `match` scrutinee, with no
      concrete `Option<T>`/`Result<T,E>` annotation anywhere else in
      the source to pre-register the needed enum decl) -- found but
      NOT fixed, deferred as BUG-91 in `docs/TODO_CURRENT.md` given
      its architectural blast radius (same category of risk as the
      deferred BUG-87 async+generics finding).
- [x] 🟢 Nested `Option<Result<T,E>>` or `Result<Option<T>,E>` — the
      built-in generic enums nested in EACH OTHER (not Vec/Array
      nested inside one of them, which the closed sweep covered) —
      confirm construction, `match`, and `try`/`?` propagation
      through both layers. Construction+match (no `try`) checked
      clean on both backends. `try` propagation through both layers
      needed the same BUG-90 fixes as row 2 above (specifically the
      nested-Let-annotation walk gap); confirmed working on both
      backends after the fix.

## 7. Collections beyond Vec/HashMap (🟡 — thin coverage, flagged but never closed in the original priority list) -- CLOSED 2026-08-03

Carried over verbatim from `TESTING_MATRIX_TODO.md`'s original
"Priority sweep list" items 4–8, which were never actually swept
(only items 1–3 on that list got a "done" marker):

- [x] 🟡 Iterator-style `Vec` builtins CHAINED together
      (`vec_map`/`vec_fold`/`vec_filter`/`vec_zip_with`/...) in one
      expression — 73 `lib.rs` hits, only 1 real end-to-end
      execution test despite heavy tutorial reliance
      (`06b_iterators_primer.md`, `15_math_rng.md`). Checked clean:
      one-expression direct chaining is correctly REJECTED per the
      tutorial's own explicitly documented restriction; chaining via
      named `let`s between steps (the v1-supported pattern) verified
      correct on both backends. See BUG-92 in `docs/TODO_CURRENT.md`.
- [x] 🟡 `task`/`join` (the primitive itself, not `parallel for`) —
      explicitly called out in `main.rs` comments as having SSA-vs-
      tree edge cases for multi-block bodies; thin real-execution
      coverage. Checked clean: call-form `task fn(args) -> Task<R>`
      with a genuinely multi-block callee body verified correct on
      both backends by hand-computed expected values.
- [x] 🟡 `--target=` cross-compile combined with `no_std`/embedded
      AND `#[no_mangle]` FFI export, all three at once (BUG-44 was
      found exactly at this three-way intersection; the intersection
      itself was never re-swept afterward for OTHER bugs in the same
      neighborhood). Found+fixed a real bug in this neighborhood
      (BUG-92): the ALREADY-SHIPPED `bare_metal.vani` example (BUG-
      44's own fix target) crashed `opt`/`llc` with ill-typed IR the
      moment it was actually built/run (on the DEFAULT host target,
      not even requiring the three-way combo) -- BUG-44's own
      verification only ever grepped emitted text, never ran the
      pipeline. Two compounding mmio_read/write_u8/u16 tree-LLVM
      bugs, fixed; see BUG-92.
- [x] 🟡 Graph/Trie/SkipList/UnionFind/BloomFilter/BST builtins
      actually RUN end-to-end (not just compiled) — 401 `lib.rs`
      hits, 8 e2e hits, despite `advanced/05b_advanced_collections.md`
      leaning heavily on them. Checked clean: all six run correctly
      together, every value verified against the tutorial's own
      documented expected output, on both backends.
- [x] 🟢 `vec_with_capacity` under `--backend=c` specifically — the
      one documented per-backend SSA-vs-tree-fallback gap in the
      coverage table; confirm the tree-C fallback computes correct
      VALUES, not just that it compiles. Checked clean: pushing past
      initial capacity (forcing real realloc/growth) produces
      correct values; `valgrind --leak-check=full` clean.
- [x] 🟡 `Deque<Struct>` / `BinaryHeap<Struct>` — do these collections
      support non-scalar (struct) elements at all, and if so, is it
      tested? (HashMap explicitly documents scalar-only V; confirm
      whether Deque/BinaryHeap share or differ from that
      restriction, and that whichever is true is actually exercised
      end-to-end.) Checked clean: both are scalar-i64-only, cleanly
      rejected, same restriction shape as HashMap -- not a bug.
- [x] 🟢 `Graph`/`Trie` with non-`i64` node/edge payloads (if
      supported at all) — confirm the boundary is real and tested,
      not just assumed from the collections chapter's i64-only
      examples. Checked clean: more fundamental than a runtime
      restriction -- both are non-generic types with no `Type::Apply`
      form at all; the parser rejects `Graph<T>`/`Trie<T>` syntax
      outright.

## 8. FFI x generics/containers/error-handling (🟡) -- CLOSED 2026-08-03

- [x] 🟡 `extern "C"` function taking or returning a MONOMORPHIZED
      GENERIC struct by value (BUG-77 tested a concrete, non-generic
      struct; never tested a generic one's monomorphized shape
      specifically, where the mangled name and the ABI-lowering path
      both have to agree). Checked clean on both backends: a small
      monomorphized generic struct passes/returns by value correctly
      against a real linked C shim; an oversized one is cleanly
      rejected, diagnostic correctly naming the MANGLED type.
- [x] 🟢 `extern "C"` function signature using `Option<T>`/
      `Result<T,E>` directly in a parameter or return position —
      confirm this is a CLEAN rejection (enums almost certainly can't
      cross the FFI ABI boundary as-is) rather than emitting garbage
      or crashing at the `cc`/`lli` step. Checked clean: cleanly
      rejected on both backends, in both parameter and return
      position, with a specific diagnostic.
- [x] 🟢 Calling an `extern "C"` function from inside a spawned
      `task` body (task bodies are restricted to `pure fn` calls per
      the concurrency chapters — confirm an FFI call inside a task
      body hits that same restriction cleanly, or is a distinct,
      undocumented gap). Checked clean: a plain extern call hits the
      same "task body cannot call non-pure function" rejection; the
      documented `pure extern "C" fn` escape hatch genuinely works
      end-to-end (verified against a real linked C shim, both
      backends, valgrind-clean).

## 9. Affine/ownership x generics x containers (🟡 — three-way, the deepest layer the closed sweep reached was two-way)

- [x] 🟡 `clone_at` on `Vec<GenericStruct<T>>` specifically (the
      closed sweep tested `Vec<GenericStruct<i64>>` ALONGSIDE
      `Vec<GenericStruct<f64>>` existing in the same program as a
      monomorphization-collision check — it did not specifically
      confirm `clone_at`'s indexed-mutate-then-`set` idiom works
      through a generic element type). Checked clean on both
      backends, valgrind-clean. See BUG-93 in `docs/TODO_CURRENT.md`.
- [x] 🟡 A recursive GENERIC struct: `struct Node<T> { value: T, next:
      Option<Box<Node<T>>> }` — self-referential AND generic at
      once. Recursive structs (non-generic) and `Box<T>`-in-enum-
      payload are each independently well-tested; nobody has
      combined them. Found+fixed a real bug (BUG-93: five compounding
      gaps -- four missing `Type::Box` arms across four copies of the
      generics-monomorphization type-walker, plus a single-pass
      generation loop that silently discarded newly-discovered needs
      from a freshly-monomorphized struct's own fields, fixed via a
      proper fixed-point worklist). Investigating this also surfaced
      THREE separate, narrower, deferred findings: bare enum
      constructors nested directly in struct-literal fields are still
      ambiguous once 2+ generic-enum instantiations exist (has a
      working workaround); field access through a bare `Box<T>`
      doesn't auto-deref at all (orthogonal to generics, wide blast
      radius to fix); and a pre-existing C-backend memory leak in
      `Box<StructWithHeapOwningFields>`'s scope-exit Drop, reproducing
      on the already-shipped BUG-35 example independent of generics.
      All documented in BUG-93's writeup in `docs/TODO_CURRENT.md`.
- [x] 🟢 `Box<T>` through a generic function boundary: `fn identity<T>
      (b: Box<T>) -> Box<T>` — explicitly flagged as "worth probing,
      never observed broken" in `missing_features.md`'s own closing
      list; still unprobed. Checked clean on both backends (struct T
      and scalar T), valgrind-clean.
- [x] 🟢 `parallel for` over a `Vec<Struct>` where the struct has an
      `OwnedStr` field — explicitly flagged as "worth probing" in the
      same list; confirm the required-Copy-capture rule correctly
      rejects this (an `OwnedStr` field makes the struct non-Copy),
      rather than silently allowing a double-free across threads.
      Checked clean: each iteration writing to a distinct index via
      `clone_at` is correctly ALLOWED (no actual race exists), and
      valgrind confirms it's genuinely memory-safe. Fresh heap
      allocation inside the loop body hits a separate, already-
      documented purity rule (not this row's concern).

## 10. Pattern matching depth x generics/enums (🟢–🟡) -- CLOSED 2026-08-03

- [x] 🟡 `match` with bindings on a DEEPLY nested built-in enum
      payload — `Result<Option<T>, E>` or similar, matched in ONE
      `match` expression if the language allows it, or confirmed to
      need the documented "two flat matches" workaround the same way
      user-declared nested enums already do. Confirmed: clean parser
      rejection when nested in one expression; the two-flat-matches
      rewrite compiles and runs correctly on both backends.
- [x] 🟢 A guarded slice-pattern arm (`[a, b] if cond then ...`,
      already fixed this session) combined with a GENERIC function
      `fn classify<T>(xs: Vec<T>) -> ...` where `T` is a Copy scalar
      type parameter, not a concrete `i64` — confirm slice patterns
      work through a monomorphized generic Vec element type. Checked
      clean on both backends, for both i64 and f64 instantiations.
- [x] 🟢 Or-pattern-shaped guard conditions (`if n == 1 || n == 2`) on
      an enum variant match arm, combined with the variant's payload
      binding used inside the guard expression itself. Checked clean
      on both backends.

## 11. Boundary confirmations (🟢 — expected-to-reject, but never actually confirmed; closes a documentation-accuracy gap even if not a "bug")

Not expected to reveal bugs, but nobody has actually run these to
confirm the assumed rejection is real, clean, and backend-consistent
rather than silently accepted or a raw panic:

- [ ] `HashMap<StructKey, V>` (non-scalar key) — confirm clean,
      consistent rejection on both backends with a real diagnostic.
- [ ] `Atomic<Vec<T>>` / `Atomic<Struct>` (non-i64-width payload) —
      confirm clean rejection matching the documented i64-width-only
      restriction.
- [ ] A `dyn Iface` method call held across an `.await` point inside
      an `async fn` — `missing_features.md` documents this as
      rejected in principle; confirm the actual diagnostic fires
      (this may already be covered — verify before re-testing).
- [ ] `Mutex<T>`/`RwLock<T>` where `T` is itself a `Mutex<U>` or
      `RwLock<U>` (nested locks) — confirm this either works
      correctly (composable locking, deadlock risk is the
      programmer's problem) or is cleanly rejected; either is fine,
      but nobody has checked which.

---

## Process (mirrors the two closed sweeps exactly)

For each row: write the `.vani` snippet, `vanic check` + `vanic run`
on both `--backend` values, compare against a hand-computed expected
value.
- **Clean pass, correct on both backends** → add a permanent
  `src/lib.rs` compile-time test AND a `tests/run_end_to_end.rs`
  real-binary test (both backends), even though no bug was found —
  this closes the coverage gap the row exists to close.
- **Clean, consistent rejection on both backends** → same as above,
  but the e2e test asserts the rejection + diagnostic text instead of
  successful output; if not already documented, add a line to
  `docs/v1_limitations.md`.
- **Crash, backend divergence, or silent wrong answer** → root-cause
  in the compiler (not the test), fix, add regression tests, log the
  bug in `docs/TODO_CURRENT.md` following the established BUG-NN
  writeup convention, run the full `cargo test --release --workspace`
  suite, commit + push, poll CI green before moving to the next row.

Batch fixes ~3 at a time before a full local test run + commit/push +
CI poll, per this session's established workflow — do not run the
full suite after every single row.
