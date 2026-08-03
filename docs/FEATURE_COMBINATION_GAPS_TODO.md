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

- [ ] `struct Cache<T> { lock: Mutex<T> }`, instantiated at two
      different `T` in the same program (mirrors the already-tested
      "generic struct at 2+ different T" pattern, but with a
      concurrency handle as the generic field instead of a plain
      value).
- [ ] `fn spawn_with_lock<T>(initial: T) -> Mutex<T>` — a generic
      function that itself constructs a `Mutex<T>`/`RwLock<T>`/
      `Channel<T,N>` from its generic parameter.
- [ ] `Task<T>` where `T` is a generic parameter of the enclosing
      generic function (`fn run<T>(x: T) -> Task<T>`).
- [ ] A generic function bounded `<T: Iface>` that spawns a `task`
      capturing a `T`-typed value — does the "task captures must be
      Copy" check correctly evaluate `T`'s Copy-ness per
      monomorphization (Copy for one instantiation, non-Copy — should
      reject — for another)?

## 3. SMT contracts x generics / concurrency / enums (🟡–🔴 mixed — the closed sweep only tested SMT x Vec/Struct)

The closed "Container x SMT contracts" section has exactly two rows
(Vec/Struct field access in `ensures`, loop invariant on `Vec<Struct>`
element). Everything below is genuinely untested.

- [ ] 🔴 `requires`/`ensures` on a **generic function** `fn foo<T>(x:
      T) -> T requires ...; ensures ...;` where the contract only
      makes sense for the scalar instantiation (e.g. `T = i64`) —
      confirm it discharges correctly per-monomorphization, and
      confirm a SECOND instantiation at a non-scalar `T` (where the
      contract can't be encoded) gets a clean rejection, not a
      silent skip (this is exactly the BUG-68 failure mode: SMT
      verdicts other than Proven/Disproven silently treated as
      proven).
- [ ] 🔴 `#[complexity(...)]` (Big-O annotation) and `requires`/
      `ensures` on the SAME function — do the two independent
      analysis passes (Big-O walker, SMT verifier) interfere, e.g.
      does the Big-O walker choke on a function body shape the SMT
      elision pass rewrites, or vice versa? Never tested together at
      all per a direct grep of both tracking docs.
- [ ] 🟡 `invariant` in a loop that also spawns a `task` or touches a
      `Mutex`/`Channel` inside the loop body — does the SMT verifier
      correctly treat the loop as opaque/unverifiable across the
      concurrency call (the way it already does for ordinary
      function calls without `ensures`), or does it incorrectly try
      to reason through it?
- [ ] 🟡 `ensures _return == ...` on a function returning
      `Option<T>`/`Result<T,E>` where the postcondition needs to
      distinguish the `Some`/`Ok` vs `None`/`Err` case — e.g.
      `ensures _return != Option.None;` or similar tag-level
      assertions. (Different from the already-tested "ensures on
      Option<Vec<T>>/Result<Struct,E> RETURN TYPE" row — this is
      about the postcondition PREDICATE referencing the enum's own
      tag/variant, not just typing the return position.)
- [ ] 🟢 `prove` statement referencing a `dyn Iface` method's return
      value — confirm this is cleanly rejected (SMT can't reason
      through a vtable call) rather than silently treated as proven.

## 4. Async/await x everything else (🟡 — the async chapter's own roadmap flags several as "queued," worth confirming the boundary is a clean rejection, not a crash)

The closed sweep tested async x plain struct/Vec return (not a bug,
synchronous-desugar case only). Nothing below was tested.

- [ ] 🟡 `async fn` that is ALSO generic: `async fn foo<T>(x: T) ->
      T`.
- [ ] 🟡 `async fn` returning `Option<T>`/`Result<T,E>` specifically
      (the closed sweep tested plain struct/Vec return, not the
      built-in generic enums).
- [ ] 🟡 `async fn` with a `requires`/`ensures` contract.
- [ ] 🟢 `async fn` taking a `Closure(...)` parameter and calling it
      before vs. after an `.await` point.
- [ ] 🟢 `async fn` spawning a `task` internally (async x the OTHER
      concurrency primitive family) — do they compose or collide?

## 5. dyn dispatch x generics (🟡 — two different polymorphism mechanisms meeting)

- [ ] 🔴 `Vec<dyn Iface>` where the underlying concrete types are
      themselves DIFFERENT INSTANTIATIONS of the same generic struct
      (`Wrapper<Dog>` and `Wrapper<Cat>`, both implementing
      `Printable` via the blanket impl already tested, both pushed
      into the SAME `Vec<dyn Printable>`). This directly composes two
      patterns each individually tested this session (two-
      instantiation generics; `Vec<dyn Iface>` heterogeneous
      dispatch) but never together.
- [ ] 🟡 A generic function bounded `<T: Iface>` that also accepts a
      `dyn Iface` parameter of the SAME interface in a different
      parameter slot (mixing static and dynamic dispatch for the
      same trait in one call).
- [ ] 🟢 A struct implementing TWO DIFFERENT interfaces, with a
      SINGLE instance referenced through both `Vec<dyn IfaceA>` and
      `Vec<dyn IfaceB>` at once (two independent vtables over the
      same concrete data).

## 6. Error propagation (`try`/`?`) x containers/generics (🟡)

- [ ] 🟡 `try`/`?` inside a function whose body also indexes/mutates
      a `Vec<Struct>` or `HashMap` — does the early-return desugar's
      drop-sequence correctly account for a live container binding
      (mirrors BUG-45's now-fixed OwnedStr-parameter case, but for a
      LOCAL Vec/HashMap rather than a parameter).
- [ ] 🟡 `try`/`?` inside a GENERIC function `fn foo<T>(...) ->
      Option<T> { let x = try bar::<T>(...); ... }`.
- [ ] 🟢 Nested `Option<Result<T,E>>` or `Result<Option<T>,E>` — the
      built-in generic enums nested in EACH OTHER (not Vec/Array
      nested inside one of them, which the closed sweep covered) —
      confirm construction, `match`, and `try`/`?` propagation
      through both layers.

## 7. Collections beyond Vec/HashMap (🟡 — thin coverage, flagged but never closed in the original priority list)

Carried over verbatim from `TESTING_MATRIX_TODO.md`'s original
"Priority sweep list" items 4–8, which were never actually swept
(only items 1–3 on that list got a "done" marker):

- [ ] 🟡 Iterator-style `Vec` builtins CHAINED together
      (`vec_map`/`vec_fold`/`vec_filter`/`vec_zip_with`/...) in one
      expression — 73 `lib.rs` hits, only 1 real end-to-end
      execution test despite heavy tutorial reliance
      (`06b_iterators_primer.md`, `15_math_rng.md`).
- [ ] 🟡 `task`/`join` (the primitive itself, not `parallel for`) —
      explicitly called out in `main.rs` comments as having SSA-vs-
      tree edge cases for multi-block bodies; thin real-execution
      coverage.
- [ ] 🟡 `--target=` cross-compile combined with `no_std`/embedded
      AND `#[no_mangle]` FFI export, all three at once (BUG-44 was
      found exactly at this three-way intersection; the intersection
      itself was never re-swept afterward for OTHER bugs in the same
      neighborhood).
- [ ] 🟡 Graph/Trie/SkipList/UnionFind/BloomFilter/BST builtins
      actually RUN end-to-end (not just compiled) — 401 `lib.rs`
      hits, 8 e2e hits, despite `advanced/05b_advanced_collections.md`
      leaning heavily on them.
- [ ] 🟢 `vec_with_capacity` under `--backend=c` specifically — the
      one documented per-backend SSA-vs-tree-fallback gap in the
      coverage table; confirm the tree-C fallback computes correct
      VALUES, not just that it compiles.
- [ ] 🟡 `Deque<Struct>` / `BinaryHeap<Struct>` — do these collections
      support non-scalar (struct) elements at all, and if so, is it
      tested? (HashMap explicitly documents scalar-only V; confirm
      whether Deque/BinaryHeap share or differ from that
      restriction, and that whichever is true is actually exercised
      end-to-end.)
- [ ] 🟢 `Graph`/`Trie` with non-`i64` node/edge payloads (if
      supported at all) — confirm the boundary is real and tested,
      not just assumed from the collections chapter's i64-only
      examples.

## 8. FFI x generics/containers/error-handling (🟡)

- [ ] 🟡 `extern "C"` function taking or returning a MONOMORPHIZED
      GENERIC struct by value (BUG-77 tested a concrete, non-generic
      struct; never tested a generic one's monomorphized shape
      specifically, where the mangled name and the ABI-lowering path
      both have to agree).
- [ ] 🟢 `extern "C"` function signature using `Option<T>`/
      `Result<T,E>` directly in a parameter or return position —
      confirm this is a CLEAN rejection (enums almost certainly can't
      cross the FFI ABI boundary as-is) rather than emitting garbage
      or crashing at the `cc`/`lli` step.
- [ ] 🟢 Calling an `extern "C"` function from inside a spawned
      `task` body (task bodies are restricted to `pure fn` calls per
      the concurrency chapters — confirm an FFI call inside a task
      body hits that same restriction cleanly, or is a distinct,
      undocumented gap).

## 9. Affine/ownership x generics x containers (🟡 — three-way, the deepest layer the closed sweep reached was two-way)

- [ ] 🟡 `clone_at` on `Vec<GenericStruct<T>>` specifically (the
      closed sweep tested `Vec<GenericStruct<i64>>` ALONGSIDE
      `Vec<GenericStruct<f64>>` existing in the same program as a
      monomorphization-collision check — it did not specifically
      confirm `clone_at`'s indexed-mutate-then-`set` idiom works
      through a generic element type).
- [ ] 🟡 A recursive GENERIC struct: `struct Node<T> { value: T, next:
      Option<Box<Node<T>>> }` — self-referential AND generic at
      once. Recursive structs (non-generic) and `Box<T>`-in-enum-
      payload are each independently well-tested; nobody has
      combined them.
- [ ] 🟢 `Box<T>` through a generic function boundary: `fn identity<T>
      (b: Box<T>) -> Box<T>` — explicitly flagged as "worth probing,
      never observed broken" in `missing_features.md`'s own closing
      list; still unprobed.
- [ ] 🟢 `parallel for` over a `Vec<Struct>` where the struct has an
      `OwnedStr` field — explicitly flagged as "worth probing" in the
      same list; confirm the required-Copy-capture rule correctly
      rejects this (an `OwnedStr` field makes the struct non-Copy),
      rather than silently allowing a double-free across threads.

## 10. Pattern matching depth x generics/enums (🟢–🟡)

- [ ] 🟡 `match` with bindings on a DEEPLY nested built-in enum
      payload — `Result<Option<T>, E>` or similar, matched in ONE
      `match` expression if the language allows it, or confirmed to
      need the documented "two flat matches" workaround the same way
      user-declared nested enums already do.
- [ ] 🟢 A guarded slice-pattern arm (`[a, b] if cond then ...`,
      already fixed this session) combined with a GENERIC function
      `fn classify<T>(xs: Vec<T>) -> ...` where `T` is a Copy scalar
      type parameter, not a concrete `i64` — confirm slice patterns
      work through a monomorphized generic Vec element type.
- [ ] 🟢 Or-pattern-shaped guard conditions (`if n == 1 || n == 2`) on
      an enum variant match arm, combined with the variant's payload
      binding used inside the guard expression itself.

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
