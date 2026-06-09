# Edge-case test matrix

> What each `.vani` file under this directory covers, mapped to
> the language-feature pairs (and triples) it exercises. As the
> compiler's feature set grows, this matrix tells future
> contributors which combinations are pinned and which are
> still gaps. Keep updated when you add a new test file.

The matrix uses 16 feature buckets. A row marks which buckets a
single test touches; mixed-feature tests touch multiple. The
cell value is the test filename.

## Feature buckets

| Code | Bucket | Examples in the bucket |
|---|---|---|
| **SCAL** | Scalar / Copy primitives | `i64`, `bool`, `f64`, `u32`, `[T; N]` of Copy |
| **STR** | Strings | `Str`, `OwnedStr`, str-builtins, `+` concat |
| **VEC** | `Vec<T>` | `vec(...)`, `push`, `len`, indexing, drop |
| **ARR** | Arrays `[T; N]` | fixed-size array literals + indexing |
| **TUP** | Tuples | `(T1, T2)`, destructuring, `.0` / `.1` |
| **STRT** | Structs | named fields, literals, field access |
| **ENM** | Enums | with/without payload; match; Option/Result |
| **REF** | References | `ref T`, `mut ref T`, scope-escape, Path B/C |
| **BOX** | `Box<T>` | `box(...)`, `Box<dyn>`, recursive drop |
| **DYN** | `dyn Iface` / interfaces | iface decl + impl + dispatch |
| **ASNC** | Async / Task / Future | `async fn`, `await`, `Task__X`, state machines |
| **SMT** | SMT verification | `requires`, `ensures`, `invariant`, `prove` |
| **CONC** | Concurrency | `parallel for`, `task`, `Atomic`, `Mutex`, `Channel`, `Condvar` |
| **CLO** | Closures / fn ptr | anon fns, `fn(...) -> R`, capture |
| **GEN** | Generics | `fn f<T>(...)`, `struct G<T>`, mono |
| **UNS** | Unsafe / FFI | `unsafe(reason = "...")`, `extern "C"`, raw ptrs |

## Single-bucket stress (the foundation tests)

| File | Bucket | What it tests |
|---|---|---|
| `edge_deep_nesting.vani` | SCAL | 10-level if/else; parser depth |
| `edge_many_params.vani` | SCAL | 20-parameter fn signature |
| `edge_match_overflow.vani` | ENM | 30-variant exhaustive match |
| `edge_huge_ensures.vani` | SMT | Multi-clause requires + 6-disjunct ensures |
| `edge_smt_chain.vani` | SMT | 3-deep cross-fn contract propagation |
| `edge_pathological_recursion.vani` | SMT | Ackermann (multi-arg recursion + ensures) |
| `edge_mutual_async.vani` | ASNC | Mutually-recursive async fns |

## Two-feature combinations (the workhorse mid-tier)

| File | Combo | What it tests |
|---|---|---|
| `mix_vec_of_box.vani` | VEC + BOX | `Vec<Box<i64>>` end-to-end; element drop |
| `mix_vec_of_box_dyn.vani` | VEC + BOX + DYN | `Vec<Box<dyn Iface>>` heterogeneous |
| `mix_box_inline_dyn.vani` | BOX + DYN + STRT | `box(Foo { ... } as dyn Iface)` inline |
| `mix_nested_enum_payload.vani` | ENM + ENM | Enum-of-enum: `enum Outer { Wrap(Inner) }` |
| `mix_closure_in_iface_impl.vani` | CLO + DYN | Anon fn inside `implement Iface for T` |
| `mix_closure_in_methods_block.vani` | CLO + STRT | Anon fn inside `methods on T` |
| `mix_struct_in_struct_deep.vani` | STRT + STR + VEC | 3-deep struct nesting, owned/Vec fields |
| `mix_box_dyn_in_struct.vani` | STRT + VEC + BOX + DYN + STR | `Vec<Box<dyn Iface>>` as a struct field alongside `Vec<i64>` + `OwnedStr` |
| `mix_async_ref_return_to_struct.vani` | ASNC + REF + STRT | `async fn pass(p: ref T) -> ref T` |
| `mix_unsafe_return_in_block.vani` | UNS + SCAL | `unsafe { return X; }` as fn body |

## Coverage matrix — which two-bucket pairs are pinned?

Pairs explicitly exercised somewhere in the existing set:

|       | SCAL | STR | VEC | ARR | TUP | STRT | ENM | REF | BOX | DYN | ASNC | SMT | CONC | CLO | GEN | UNS |
|-------|------|-----|-----|-----|-----|------|-----|-----|-----|-----|------|-----|------|-----|-----|-----|
| SCAL  |  ✓   |     |     |     |     |      |     |     |     |     |      |  ✓  |      |     |     |     |
| STR   |      |  ✓  |     |     |     |  ✓   |     |     |     |     |      |     |      |     |     |     |
| VEC   |      |  ✓  |  ✓  |     |     |  ✓   |     |     |  ✓  |  ✓  |      |     |      |     |     |     |
| ARR   |      |     |     |     |     |      |     |     |     |     |      |     |      |     |     |     |
| TUP   |      |     |     |     |     |      |     |     |     |     |      |     |      |     |     |     |
| STRT  |      |  ✓  |  ✓  |     |     |  ✓   |     |  ✓  |  ✓  |  ✓  |  ✓   |     |      |  ✓  |     |     |
| ENM   |      |     |     |     |     |      |  ✓  |     |     |     |      |     |      |     |     |     |
| REF   |      |     |     |     |     |  ✓   |     |  ✓  |     |     |  ✓   |     |      |     |     |     |
| BOX   |      |     |  ✓  |     |     |  ✓   |     |     |  ✓  |  ✓  |      |     |      |     |     |     |
| DYN   |      |     |  ✓  |     |     |  ✓   |     |     |  ✓  |  ✓  |      |     |      |  ✓  |     |     |
| ASNC  |      |     |     |     |     |  ✓   |     |  ✓  |     |     |  ✓   |     |      |     |     |     |
| SMT   |   ✓  |     |     |     |     |      |     |     |     |     |      |  ✓  |      |     |     |     |
| CONC  |      |     |     |     |     |      |     |     |     |     |      |     |  ?   |     |     |     |
| CLO   |      |     |     |     |     |  ✓   |     |     |     |  ✓  |      |     |      |  ?  |     |     |
| GEN   |      |     |     |     |     |      |     |     |     |     |      |     |      |     |  ?  |     |
| UNS   |   ✓  |     |     |     |     |      |     |     |     |     |      |     |      |     |     |  ?  |

✓ = pinned; ? = single-feature only, no cross test in the edge set yet (gap).

## Documented gaps (combinations not yet tested)

The blank cells in the matrix correspond to feature pairs we
haven't constructed a focused mixed-feature test for. Each is a
candidate for a future adversarial test:

### High-priority gaps (likely surface area for bugs)

1. **CONC + STR / VEC / STRT / BOX / DYN** — multi-thread tasks
   passing complex types via channels.
2. **GEN + any other bucket** — generic fns / structs holding
   any non-Copy type. Generic monomorphization + complex types
   is fertile bug territory.
3. **CLO + capture-rich features** — closure capturing a Box,
   Vec, struct-with-OwnedStr, etc.
4. **ASNC + most other features** — async fn doing parallel for,
   async fn with Mutex, async fn returning Result.
5. **SMT + complex types** — `ensures` clause referring to
   struct field, Vec element, ref-deref through call.
6. **ARR + TUP** — `[(i64, OwnedStr); 4]` (rejected today —
   tuples are Copy-only; nice to pin the rejection).
7. **TUP + various** — tuples in any context outside `(i64,
   i64)` are under-tested.
8. **ENM + BOX** — `Option<Box<T>>` is rejected; document the
   rejection.
9. **ENM + STRT** — enum-with-struct-payload, struct-with-enum-
   field. Common; under-tested.
10. **REF + STR** — `ref OwnedStr`, `ref Str`. Likely safe but
    untested.
11. **UNS + each feature** — `unsafe` block containing a Box op
    / Vec op / etc.

### Three-feature combinations not pinned

Each is a candidate for a more aggressive test:

- ASNC + REF + VEC (async fn taking ref Vec<T> across await)
- ASNC + BOX + DYN (async fn returning a Box<dyn Iface>)
- GEN + BOX + dyn (generic over a Box<dyn Iface>)
- CONC + BOX + STRT (task capturing a Box<Struct>)
- SMT + REF + STRT (ensures over a ref-struct field)
- CLO + REF + VEC (closure capturing a ref Vec)
- TUP + STRT + VEC (tuple with struct + Vec fields)
- ENM + REF + VEC (Vec<Option<ref T>> — likely rejected today)
- UNS + any-non-trivial-feature pair (raw pointer + Vec, etc.)

## How to extend this set

Adding a new test:

1. Write the `.vani` file in `examples/edge_cases/`.
   Filename convention: `mix_<feature_a>_<feature_b>.vani` for
   should-pass combos. Future xfail (must-reject) shapes go
   under `xfail_<...>.vani`.
2. Add a row to the **Single-bucket stress** or **Two-feature
   combinations** table above.
3. Mark the relevant cell(s) in the coverage matrix.
4. The `tests/edge_cases.rs` integration test auto-discovers
   every `.vani` file in this dir; no Rust-side wiring needed.

Cleanups:

- Move the file to `xfail_<...>.vani` if it transitions to
  a "must-reject" case after a future feature deprecation.
- Delete (with a justification comment in this README) if the
  combination becomes impossible due to a deliberate language
  change.

Bug-tracking:

- When the audit finds a new compiler bug, add the minimal
  reproducer here BEFORE the fix; mark it `pre_fix_<bug>.vani`
  in the commit message. After the fix lands, rename to
  `mix_<feature>_<feature>.vani` so the regression stays
  pinned without indicating which session originally found it.
