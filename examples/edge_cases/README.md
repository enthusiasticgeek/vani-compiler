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

Last verified 2026-06-09 against `vanic` HEAD; mixed-feature gap table updated 2026-06-20 for v0.1.1/v0.1.2/v0.1.3 changes.

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
| [`mix_box_enum_payload.vani`](mix_box_enum_payload.vani) | `Box<T>` and `Box<dyn Iface>` as enum variant payloads | **PREVIOUSLY REJECTED** (`Option<Box<T>>` not admitted); fixed 2026-06-20. Now compiles + runs on both backends; output `42`. C + LLVM Drop dispatch frees payload Box correctly. |
| [`mix_tuple_non_copy.vani`](mix_tuple_non_copy.vani) | `(OwnedStr, i64)` tuple — non-Copy element in tuple | **PREVIOUSLY REJECTED** (v1 tuples were Copy-only); fixed 2026-06-20. Now compiles + runs on both backends; output `42`. Scope-exit Drop walks each element; move-into-tuple marks source as moved; LetTuple destructuring marks source binding as moved. |

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
| `(Box<T>, U)` tuple element | ✅ **FIXED (v0.1.4, 2026-06-20)** — non-Copy elements allowed in tuples; tuple becomes non-Copy; scope-exit Drop frees heap elements; regression test: `mix_tuple_non_copy.vani` | No workaround needed |
| `(OwnedStr, U)` tuple element | ✅ **FIXED (v0.1.4, 2026-06-20)** — same fix; regression test: `mix_tuple_non_copy.vani` | No workaround needed |
| `Option<Box<T>>` enum payload | ✅ **FIXED (v0.1.4, 2026-06-20)** — checker now admits `Box<T>` in enum variant payloads; C + LLVM backends emit correct Drop; regression test: `mix_box_enum_payload.vani` | No workaround needed |
| `HashMap<K, V>` with non-scalar V | ✅ **FIXED (Arc 4, pre-v0.1.0)** — `hashmap_insert` now accepts `OwnedStr`, `Vec<i64>`, tuple, `f64`, and `Vec`-typed values; full K-V type matrix shipped | No workaround needed |
| `Mutex<Vec<T>>` | ✅ **FIXED (v0.1.1, 2026-06-18)** — `Mutex<T>` is now parametric over any element type including `Vec<T>` | No workaround needed |
| Closure capturing non-Copy binding | ⬜ Rejected with `closure_captures_affine` elaboration | Pre-extract scalar / pass as fn arg |

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

### Bug 5 — `return` inside `unsafe { ... }` (FIXED)

A function whose only `return` was inside an
`unsafe(reason = "...") { return X; }` block failed the
"function must return T" check. The unsafe-block handler in
the checker captured the inner-body's termination flag but
discarded it (`let _terminated = check_stmt_list(...)`),
always returning `false`. Fixed by propagating
`inner_terminated` as the unsafe-block's return value (also
skip the redundant scope-exit drop emission when inner
already returned). Regression:
`unsafe_block_with_return_satisfies_must_return`.

### Bug 6 — async fn returning `ref T` (FIXED, C backend)

An async fn `async fn pass(p: ref T) -> ref T` synthesizes
a payloaded enum `Future__Ref_Struct__T` whose payload is
`const Struct_T*`. The C backend's enum-typedef pre-emit
pass (which runs BEFORE struct typedefs) emitted this
typedef before `Struct_T` was declared, so cc rejected
with "unknown type name 'Struct_T'". LLVM compiled fine.
Fixed by deferring the pre-emit when an enum's payload
references a user struct (directly or via Vec / Array /
Ref / RefMut / Box / Tuple). The post-struct-typedef pass
already runs for these — letting it pick them up resolves
the ordering. Regression:
`async_fn_returning_ref_to_user_struct_compiles_on_c_backend`.

### Bug 4 — `Vec<Box<T>>` panicked both backends (C backend FIXED; LLVM backend still leaks)

`Vec<Box<T>>` previously panicked both backends with placeholder-
identifier emission (C: `intent_vec_/*_Box<T>_*/__from(...)`;
LLVM: `unreachable: use llvm_type_string for aggregate / ref
type Box(I64)`). Fixed for inner `i64` / `Vec<U>` / `OwnedStr` /
`dyn Iface`. **C backend** now per-element-drops Box elements
correctly (ASan-clean on the `Vec<Box<i64>>` shape). **LLVM
backend** still leaks Box-element heap allocations in the
Vec's `__free` (no Box arm in the per-element drop emit). The
LLVM `Vec<Box<dyn Iface>>` case additionally crashes at
scope-exit with "double free or corruption" — under
investigation; the per-element drop wiring is incomplete on
LLVM. Tracked as a follow-up.

Regression test: `vec_of_box_compiles_on_both_backends` (pins
the compile-succeeds case; runtime cleanup is a follow-up).

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
