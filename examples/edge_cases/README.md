# Adversarial / edge-case examples

> Curated set of `.vani` programs designed to test
> compiler edge cases — deep nesting, large signatures,
> heavy SMT, pathological recursion, mutual references,
> nested generics. All compile cleanly today; this is the
> "did anything actually break?" check.
> Run them with `vanic run <file> --backend=c` (or
> `--backend=llvm`) to verify.

Last verified 2026-06-09 against `vanic` HEAD.

## Test inventory

| File | Tests | Verified behavior |
|---|---|---|
| [`edge_deep_nesting.vani`](edge_deep_nesting.vani) | Parser stack depth — 10 deep if/else chain | Compiles + runs; output `7` (n==7 branch) |
| [`edge_many_params.vani`](edge_many_params.vani) | 20-parameter function signature | Compiles + runs; output `210` (sum 1..20) |
| [`edge_huge_ensures.vani`](edge_huge_ensures.vani) | Heavy SMT discharge — 6-disjunct `ensures` + multi-`requires` | Verifier discharges; output `2` |
| [`edge_pathological_recursion.vani`](edge_pathological_recursion.vani) | Ackermann's function — multi-arg recursion + nested recursive calls | Compiles; Big-O = `O(recursive)`; runs to `9` for `ack(2,3)` |
| [`edge_smt_chain.vani`](edge_smt_chain.vani) | Cross-fn SMT fact propagation through 3-deep chain of `requires`/`ensures` | All clauses discharged; output `80` |
| [`edge_match_overflow.vani`](edge_match_overflow.vani) | 30-variant enum + exhaustive match | Compiles + runs; output `17` |
| [`edge_mutual_async.vani`](edge_mutual_async.vani) | Mutually recursive `async fn` (ping ↔ pong) | Compiles; Big-O = `O(recursive)` for both (mutual-recursion SCC) |

## Tests that fail by design (rejection is the success case)

| Scenario | Expected behavior |
|---|---|
| `Vec<Vec<i64>>[0][0]` index of non-Copy element | Rejected with `vec_index_non_copy_aliases` elaboration |
| `n * n` ensures positive without `n` bound | Rejected with SMT counterexample at `i64::MIN` |
| `fn foo() -> ref T` with no ref param | Rejected with path-C `ret_type_is_ref` elaboration |
| `fn foo(a: ref T, b: ref T) -> ref T` | Rejected with multi-ref-param elision diagnostic |
| `for x in xs; xs[i]` after consuming move | Rejected with `move_after_use` elaboration |
| `parallel for i in 0..n { print i; }` | Rejected — race-unsafe side effect |

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
