# Adversarial / edge-case examples

> Curated set of `.vani` programs designed to test
> compiler edge cases — deep nesting, large signatures,
> heavy SMT, pathological recursion, mutual references,
> nested generics, **and mixed-feature combinations** (the
> shapes where two or three features interact in ways the
> per-feature tests didn't cover). All "should-pass" cases
> here compile cleanly today; the "should-reject" cases
> reject with clear diagnostics; one panic was found and
> fixed (see below). Run with `vanic run <file>
> --backend=c` (or `--backend=llvm`) to verify.

Last verified 2026-06-09 against `vanic` HEAD.

> **Why mixed-feature tests matter.** A feature working in
> isolation is necessary but not sufficient. `Box<T>` works.
> `dyn Iface` works. `Box<dyn Iface>` works. **`box(X { ...
> } as dyn Iface)` with an inline struct literal** — a
> combination of three features (Box, struct lit, dyn-coerce
> in box arg position) — **panicked both backends** until
> fixed in this session. Every feature interaction is a
> potential gap; combinatorial coverage matters more than
> any single feature's depth.

## Single-feature stress tests

| File | Tests | Verified behavior |
|---|---|---|
| [`edge_deep_nesting.vani`](edge_deep_nesting.vani) | Parser stack depth — 10 deep if/else chain | Compiles + runs; output `7` (n==7 branch) |
| [`edge_many_params.vani`](edge_many_params.vani) | 20-parameter function signature | Compiles + runs; output `210` (sum 1..20) |
| [`edge_huge_ensures.vani`](edge_huge_ensures.vani) | Heavy SMT discharge — 6-disjunct `ensures` + multi-`requires` | Verifier discharges; output `2` |
| [`edge_pathological_recursion.vani`](edge_pathological_recursion.vani) | Ackermann's function — multi-arg recursion + nested recursive calls | Compiles; Big-O = `O(recursive)`; runs to `9` for `ack(2,3)` |
| [`edge_smt_chain.vani`](edge_smt_chain.vani) | Cross-fn SMT fact propagation through 3-deep chain of `requires`/`ensures` | All clauses discharged; output `80` |
| [`edge_match_overflow.vani`](edge_match_overflow.vani) | 30-variant enum + exhaustive match | Compiles + runs; output `17` |
| [`edge_mutual_async.vani`](edge_mutual_async.vani) | Mutually recursive `async fn` (ping ↔ pong) | Compiles; Big-O = `O(recursive)` for both (mutual-recursion SCC) |

## Mixed-feature stress tests

These programs combine 2-3 features to test interaction
shapes the per-feature tests don't reach.

| File | Feature combo | Verified behavior |
|---|---|---|
| [`mix_box_inline_dyn.vani`](mix_box_inline_dyn.vani) | inline struct lit + `as dyn Iface` + `box(...)` | **PREVIOUSLY PANICKED**; fixed 2026-06-09. Now compiles + runs on both backends. The C and LLVM codegen now unwrap the Block-wrapped DynCoerce that the checker hoists. |
| [`mix_struct_in_struct_deep.vani`](mix_struct_in_struct_deep.vani) | 3-deep struct nesting + OwnedStr + Vec field at each level | Compiles + runs; output `42`. Affine drops chain correctly through all three layers. |
| [`mix_box_dyn_in_struct.vani`](mix_box_dyn_in_struct.vani) | Struct field is `Vec<Box<dyn Iface>>`; struct also has Vec\<i64\> + OwnedStr fields | Compiles + runs after the inline-box-dyn fix. |

## Tests that fail by design (rejection is the success case)

| Scenario | Expected behavior |
|---|---|
| `Vec<Vec<i64>>[0][0]` index of non-Copy element | Rejected with `vec_index_non_copy_aliases` elaboration |
| `n * n` ensures positive without `n` bound | Rejected with SMT counterexample at `i64::MIN` |
| `fn foo() -> ref T` with no ref param | Rejected with path-C `ret_type_is_ref` elaboration |
| `fn foo(a: ref T, b: ref T) -> ref T` | Rejected with multi-ref-param elision diagnostic |
| `for x in xs; xs[i]` after consuming move | Rejected with `move_after_use` elaboration |
| `parallel for i in 0..n { print i; }` | Rejected — race-unsafe side effect |

## Documented mixed-feature gaps (NOT yet supported)

These combinations are honestly rejected by the checker
today. The documentation calls them out so users know to
restructure rather than wonder if it's a bug.

| Combination | Status | Workaround |
|---|---|---|
| `(Box<T>, U)` tuple element | Rejected — v1 tuples are Copy-only | Wrap in a named struct; structs accept non-Copy fields |
| `(OwnedStr, U)` tuple element | Rejected — same reason | Same — named struct |
| `Option<Box<T>>` enum payload | Rejected — v1 admits Copy / OwnedStr / Vec / array / Task / Atomic / Mutex / Channel only | Use Vec<Box<T>> of length 0 or 1, or wrap in a struct field with custom None handling |
| `HashMap<K, V>` with non-scalar V | Rejected at use-site (`hashmap_insert`) | v1 HashMap is (i64, i64) only; index-into-Vec for non-scalar values |
| `Mutex<Vec<T>>` | Not supported | Channel-transfer ownership instead |
| Closure capturing non-Copy binding | Rejected with `closure_captures_affine` elaboration | Pre-extract scalar / pass as fn arg |

## What's been tried and works correctly

- **Parser depth.** 10+ levels of nested if/else parse and
  type-check without stack overflow.
- **Large signatures.** 20-parameter functions compile + run.
- **Heavy SMT.** Multi-clause `requires` + multi-disjunct
  `ensures` discharge in reasonable time (< 1 second on a
  modern laptop).
- **Pathological recursion.** Multi-arg recursion (Ackermann)
  + mutual recursion compile correctly; Big-O honestly
  classifies as `O(recursive)`.
- **Cross-fn SMT propagation.** A 3-deep chain of contracts
  carries facts through without losing them.
- **Many-variant enums.** 30-variant exhaustive match
  compiles cleanly.
- **Deeply nested structs.** 3-deep struct nesting with OwnedStr
  + Vec at each level compiles + runs; affine drops chain.
- **Vec<Box<dyn Iface>> in struct field.** Struct holds a Vec of
  boxed interface objects alongside Vec<i64> + OwnedStr.

## Notable findings during this round

Three compiler bugs found and fixed; several documented
limitations confirmed.

### Bug 1 — `box(X as dyn Iface)` inline form (FIXED)

`box(Foo { ... } as dyn Iface)` with an inline struct
literal previously panicked both the C and LLVM backends
with `Box<dyn Iface> __box_new expected a DynCoerce arg;
got Block { ... }`. The checker hoists the inline struct
into a synthetic let inside a Block; the codegen now
unwraps the Block before pulling out the DynCoerce.
Regression test: `box_dyn_iface_accepts_inline_struct_literal_source`.

### Bug 2 — Nested enum payloads (FIXED)

`enum Outer { Wrap(Inner) }` where Inner is itself an enum
previously rejected `Outer.Wrap(Inner.A(42))` with the
nonsensical "enum payload must be assignable to Inner, got
Inner" — same name on both sides. Root cause: the parser
stamps the variant's payload type as `Type::Struct("Inner")`
(parser doesn't know Inner is an enum yet); the
`resolve_enum_types_in_program` pass walked function
signatures, struct fields, type aliases, consts, methods
blocks, and impls — but **not enum variant payloads**.
Fixed by extending the pass.
Regression test: `nested_enum_payload_accepts_enum_construction`.

### Bug 3 — Closure inside iface impl / methods-block (FIXED)

Inline `let f = fn(...) -> R { ... };` inside an
`implement Iface for T { fn m(...) { ... } }` body or a
`methods on T { fn m(...) { ... } }` body previously
panicked the checker with "internal: anonymous fn expression
survived the lambda-lift pass. This is a vāṇी compiler bug
— please report." The `lambda_lift_program` walked only
`program.functions`; impls were hoisted INTO functions later
but never had their inline closures lifted. Fixed by lifting
closures inside `methods_blocks` + `impls` before the hoist
pass runs.
Regression tests: `closure_inside_iface_impl_method_lifts_correctly`
+ `closure_inside_methods_block_method_lifts_correctly`.

### Newly documented limitations

- **Integer overflow guards are NOT emitted** in v1 despite
  the README's claim. `i64::MAX + 1` silently wraps to
  `i64::MIN` on both backends. Real safety gap; documented
  in [`docs/missing_features.md`](../../docs/missing_features.md).
- **Generic-call inference** is limited to literal args / Var
  / (v3.1 only) Ref/RefMut(Var) at the T-position. Complex
  argument expressions reject with a documented diagnostic;
  no turbofish (`f::<i64>(...)`) syntax.
- **Tuples are Copy-only.** `(Box<T>, U)` / `(OwnedStr, U)`
  reject; workaround is a named struct.
- **OwnedStr enum payloads** are exposed in match arm
  bindings as `Str` (read-only borrow) not `OwnedStr` (owning).
  Match arm returning the bound `s` then types as `Str`,
  which mismatches if other arms return `OwnedStr`.
  Workaround: `s + ""` to copy into OwnedStr at the binding
  site.
- **`Option<Box<T>>` enum payload** rejects — v1 enum
  variant payloads admit Copy / OwnedStr / Vec / array /
  Task / Atomic / Mutex / Channel only.
- **Anonymous fn called inline from Vec slot** (`fs[0](10)`)
  rejects — only named functions can be called.

## What would actually break the compiler

The cases below would hit genuine compiler bugs / panics if
they triggered. **None of these have been observed in
testing.** Listed here as a record of where to look if a
panic ever surfaces:

- **Generic instantiation at very high depth** — could
  trip the monomorphization loop if a generic recursively
  instantiates itself.
- **SMT timeout under non-linear arithmetic** — Z3 may
  return `Unknown` rather than `Disproven`; the compiler
  handles this gracefully (emits the runtime guard).
- **Closure-in-closure capture chain** — the closure-tag
  generator might collide for deeply nested closures.
- **Devanagari identifier with rare combining marks** —
  the LLVM identifier-mangling layer hashes the source
  bytes; collision-free in practice but no formal proof.
- **Recursive structs with no termination** — the layout
  analyzer should reject; if it doesn't, the LLVM emit
  panics on the cyclic struct type.

## How to extend this set

When you find a new "this might break the compiler" scenario,
add a `.vani` file to this directory and a row to the
inventory table. Make sure the program either compiles +
runs cleanly OR rejects with a clear diagnostic — silent
compiler panics are the failure mode this set is meant to
catch.

Run all examples in this directory:

```bash
for f in examples/edge_cases/*.vani; do
  echo "== $f =="
  ./target/release/vanic check "$f" --big-o || true
done
```
