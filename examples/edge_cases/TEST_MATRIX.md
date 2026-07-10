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
| **SIMD** | SIMD / vec128 | `vec128<T>`, `simd_splat`, `simd_add`, `simd_load`, `simd_store`, `simd_reduce_add` |

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

## Out-of-the-box adversarial shapes

Tests added during a "think outside the box" round. Each
probes a non-obvious feature interaction or boundary value.

| File | Probe | Verified |
|---|---|---|
| `mix_str_concat_with_i64.vani` | OwnedStr + i64 conversion + concat | runs; prints `n=42` |
| `mix_print_unicode.vani` | Unicode (Hindi + Chinese + Arabic + emoji) in print | runs; emits all glyphs |
| `mix_iface_method_chain.vani` | Iface method-chain through builder pattern | runs; sum = 6 |
| `mix_methods_on_enum.vani` | `methods on Enum` block | runs; returns 1 |
| `mix_match_in_struct_lit.vani` | `match` as a struct field initializer expression | runs |
| `mix_nested_if_expr.vani` | Nested if-as-expression | runs |
| `xfail_closure_in_block_expr.vani` | Closure inside a block expression (mistyped as `void*`) — KNOWN BUG | rejects at codegen |
| `xfail_vec_tree_empty_init.vani` | `vec()` of a recursive struct without type-flow inference | rejects with annotation diagnostic |
| `xfail_match_stmt_without_let.vani` | `match` as standalone statement (no `let` capturing) | rejects with parser diagnostic |

## SIMD tests (new bucket — added 2026-07-10)

| File | Combo | What it tests |
|---|---|---|
| `mix_simd_basic.vani` | SIMD | i32 splat + add + reduce_add; 4 lanes × 3 = 12 |
| `mix_simd_ref_vec.vani` | SIMD + REF + VEC | `simd_load` from `ref Vec<f32>`; hsum 4 elements |
| `mix_simd_parallel_capture.vani` | SIMD + CONC | `vec128<f32>` captured read-only in `parallel for`; pins ctx 16-byte alloc |
| `mix_simd_i32_mul.vani` | SIMD | `simd_mul` + `simd_reduce_add` on `vec128<i32>` |
| `xfail_simd_bool_splat.vani` | SIMD | `vec128<bool>` is not a valid element type — must reject |
| `xfail_simd_type_mismatch.vani` | SIMD | `simd_add(vec128<i32>, vec128<f32>)` — element type mismatch must reject |

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
| `mix_generic_ref_vec_param.vani` | GEN + REF + VEC | generic fn `len_of<T>(v: ref Vec<T>)` |
| `mix_generic_box_wrap.vani` | GEN + BOX + VEC | compose `wrap<T>` + `first<T>` generic fns |
| `mix_generic_option_return.vani` | GEN + ENM | generic fn returning `Option<T>` |
| `mix_enum_struct_match.vani` | ENM + STRT | match arm extracts struct payload fields |
| `mix_conc_parallel_struct_capture.vani` | CONC + STRT | struct field accessed via parallel-for capture |
| `mix_conc_mutex_struct.vani` | CONC + STRT | `Mutex<i64>` as a struct field; lock + guard_get |
| `mix_conc_channel_send_recv.vani` | CONC | `Channel<i64>` create + send + recv round-trip |
| `mix_closure_vec_len_capture.vani` | CLO + VEC | closure captures Vec length (Copy i64 from Vec) |
| `mix_closure_box_dyn_call.vani` | CLO + BOX + DYN | closure applied to result of DYN dispatch on Box |
| `mix_closure_chain.vani` | CLO | two closures composed in one function |
| `mix_smt_vec_param.vani` | SMT + VEC | `ref Vec<i64>` param alongside requires/ensures |
| `mix_smt_pure_struct_ref.vani` | SMT + STRT + REF | `pure fn` with struct ref; ensures provable from requires |
| `mix_tuple_struct_elem.vani` | TUP + STRT | tuple containing a struct; destructure to access fields |
| `mix_tuple_fn_return.vani` | TUP | function returning `(i64, i64)`; caller destructures |

## Coverage matrix — which two-bucket pairs are pinned?

Pairs explicitly exercised somewhere in the existing set:

|       | SCAL | STR | VEC | ARR | TUP | STRT | ENM | REF | BOX | DYN | ASNC | SMT | CONC | CLO | GEN | UNS | SIMD |
|-------|------|-----|-----|-----|-----|------|-----|-----|-----|-----|------|-----|------|-----|-----|-----|------|
| SCAL  |  ✓   |     |     |     |     |      |     |     |     |     |      |  ✓  |      |     |     |     |      |
| STR   |      |  ✓  |     |     |     |  ✓   |     |     |     |     |      |     |      |     |     |     |      |
| VEC   |      |  ✓  |  ✓  |     |     |  ✓   |     |     |  ✓  |  ✓  |      |  ✓  |  ✓   |  ✓  |  ✓  |     |      |
| ARR   |      |     |     |     |     |      |     |     |     |     |      |     |      |     |     |     |      |
| TUP   |      |  ✓  |     |     |  ✓  |  ✓   |     |     |     |     |      |     |      |     |     |     |      |
| STRT  |      |  ✓  |  ✓  |     |  ✓  |  ✓   |  ✓  |  ✓  |  ✓  |  ✓  |  ✓   |  ✓  |  ✓   |  ✓  |     |     |      |
| ENM   |      |     |     |     |     |  ✓   |  ✓  |     |     |     |      |     |      |     |  ✓  |     |      |
| REF   |      |     |  ✓  |     |     |  ✓   |     |  ✓  |     |     |  ✓   |  ✓  |      |     |  ✓  |     |  ✓   |
| BOX   |      |     |  ✓  |     |     |  ✓   |     |     |  ✓  |  ✓  |  ✓   |     |      |  ✓  |  ✓  |     |      |
| DYN   |      |     |  ✓  |     |     |  ✓   |     |     |  ✓  |  ✓  |  ✓   |     |      |  ✓  |     |     |      |
| ASNC  |      |     |     |     |     |  ✓   |     |  ✓  |  ✓  |  ✓  |  ✓   |     |      |     |     |     |      |
| SMT   |   ✓  |     |  ✓  |     |     |  ✓   |     |  ✓  |     |     |      |  ✓  |      |     |     |     |      |
| CONC  |      |     |  ✓  |     |     |  ✓   |     |     |     |     |      |     |  ✓   |     |     |     |  ✓   |
| CLO   |      |     |  ✓  |     |     |  ✓   |     |     |  ✓  |  ✓  |      |     |      |  ✓  |     |     |      |
| GEN   |      |     |  ✓  |     |     |      |  ✓  |  ✓  |  ✓  |     |      |     |      |     |  ✓  |     |      |
| UNS   |   ✓  |     |  ✓  |     |     |      |     |     |     |     |      |     |      |     |     |  ✓  |      |
| SIMD  |      |     |  ✓  |     |     |      |     |  ✓  |     |     |      |     |  ✓   |     |     |     |  ✓   |

✓ = pinned; ARR row remains empty (fixed-size array literals not yet confirmed in the compiler).

## Documented gaps (combinations not yet tested)

Gaps addressed in the 2026-07-10 audit round:
- SIMD bucket added (6 new files); SIMD × REF, SIMD × VEC, SIMD × CONC pinned
- GEN × REF, GEN × ENM, GEN × VEC cross-tests added
- ENM + STRT match-extraction added
- CONC × STRT, CONC × Channel pinned
- CLO × VEC, CLO × BOX + DYN pinned
- SMT × VEC, SMT × STRT × REF (`pure fn`) pinned
- TUP × STRT, TUP return-value pinned

### Remaining gaps

1. **ARR + any** — fixed-size array literal syntax not yet confirmed
   as a live compiler feature; all ARR cells remain empty.
2. **ASNC + VEC / Box** — async fn with complex param types across
   await points. `mix_async_box_dyn.vani` covers ASNC+BOX+DYN in
   compile-only mode; run-mode async integration is largely untested.
3. **GEN + STRT methods** — generic struct with `methods on T` block
   (monomorphization of method dispatch).
4. **CLO + REF + VEC** — closure capturing a `ref Vec` (as opposed
   to a Copy length scalar).
5. **ENM + BOX** — `Option<Box<T>>` is rejected; would be a good
   `xfail_enum_box_payload.vani` pin.
6. **REF + STR** — `ref OwnedStr` through function calls.
7. **UNS + BOX / DYN** — unsafe block touching heap-allocated types.

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
