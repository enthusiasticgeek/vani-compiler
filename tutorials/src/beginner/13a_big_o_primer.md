# Beginner 13a — Big-O notation (intuition primer)

> **Learning goal**: read and write Big-O complexity annotations
> the way the `--big-o` flag emits them. Reading order: this is
> short + foundational; read it any time after [Beginner 7 — Vec
> and arrays](07_vec_arrays.md). Pairs with the compiler's
> `--big-o` flag — `vanic check foo.vani --big-o` prints a
> per-fn `O(...)` annotation that this chapter teaches you to
> read.

This chapter has **no compiler code**. Pure intuition.

## What problem Big-O solves

You wrote a function that searches a list for a value. Question:
how slow does it get as the list gets bigger?

Concrete numbers don't generalize. "0.3 seconds on my laptop"
tells you nothing about a server, or a Raspberry Pi, or a list
that's 10× bigger.

**Big-O** answers a different question: as the input size grows,
how does the running time GROW? Not "how long does it take" —
"how does the time grow when n doubles."

If `n` doubles and your time doubles → linear → `O(n)`.
If `n` doubles and your time quadruples → quadratic → `O(n²)`.
If `n` doubles and your time stays the same → constant →
`O(1)`.

The notation is a compact label for the **growth shape**, not
the actual milliseconds.

## The four shapes you'll see most

| Notation | Name | Growth when n doubles | When you see it |
|---|---|---|---|
| `O(1)` | constant | stays the same | hash lookup, array index, push/pop |
| `O(log n)` | logarithmic | grows by a constant | binary search, BTreeMap get/insert |
| `O(n)` | linear | doubles | scan a Vec, find/contains, linear search |
| `O(n log n)` | "n log n" | a bit more than doubles | sort, sort_by |
| `O(n²)` | quadratic | quadruples | nested loops over the same list |

There are slower shapes (`O(n³)`, `O(2ⁿ)`, `O(n!)`) and a few
weirder ones (`O(α(n))` for union-find), but 90% of code lives
in these five.

## Why "n doubles" matters

Start with `n = 100`. Imagine each step takes 1 microsecond.

| Shape | 100 steps cost | 1,000 cost | 10,000 cost | 1,000,000 cost |
|---|---|---|---|---|
| `O(1)` | 1 µs | 1 µs | 1 µs | 1 µs |
| `O(log n)` | 7 µs | 10 µs | 13 µs | 20 µs |
| `O(n)` | 100 µs | 1 ms | 10 ms | 1 sec |
| `O(n log n)` | 700 µs | 10 ms | 130 ms | 20 sec |
| `O(n²)` | 10 ms | 1 sec | 100 sec | ~12 days |

The `O(n²)` row is why beginners frequently write a program
that works fine on test data and times out in production.
"Fast enough on 100 items" is not "fast enough on a million
items" — the growth shape predicts the wall.

## How the compiler infers complexity

vāṇी's `--big-o` flag walks each function and looks at:

1. **Loop nesting depth.** Zero loops → `O(1)` baseline. One
   loop → `O(n)`. Two nested → `O(n²)`. K nested → `O(n^k)`.
2. **Builtin call asymptotics.** `sort(...)` injects `O(n log n)`
   into the body's cost. `binary_search` injects `O(log n)`.
   `find` injects `O(n)`. These multiply with the surrounding
   loop nesting.
3. **Self-recursion.** A function that calls itself gets
   `O(recursive)` — no recurrence solver in v1, so the
   compiler honestly reports "I can't bound this."

So a function like:

```rust
fn sort_then_search(xs: mut ref Vec<i64>, key: i64) -> i64 {
  let _ = sort(mut ref xs);
  return binary_search(ref xs, key);
}
```

ships annotated as `O(n log n)` — the sort dominates the
binary_search.

A function like:

```rust
fn all_pairs(xs: ref Vec<i64>) -> i64 {
  // ... two nested loops over xs ...
}
```

ships annotated as `O(n²)`.

## Reading vāṇी's annotation output

`vanic check foo.vani --big-o` prints lines like:

```
  fn one_loop: O(n)
  fn nested_loops: O(n²)
  fn just_sort: O(n log n)
  fn recursive: O(recursive)
```

`--big-o=auto` (the default) skips `O(1)` fns to keep output
focused on the interesting cases. `--big-o=force` prints
every fn including the trivial ones — useful when reviewing
to confirm "yes, this helper IS O(1) by design."

`vanic emit foo.vani --backend=c --big-o` prepends the same
table as a comment block to the emitted C source, so reviewers
reading the output also see the complexity contract.

## When the annotation is wrong (or the analyzer gives up)

The analyzer is conservative and local — it can over-report,
under-report, or honestly admit it can't bound the work.

### Cases the analyzer handles correctly (refined 2026-06-09)

- **Bounded loops.** `for i from 0 to 16` and `for x in ref
  arr` where `arr: [T; N]` stay `O(1)` — the iteration count
  is part of the type or a literal constant.
- **Cross-fn propagation.** A fn that calls an O(n log n)
  helper inside an O(n) loop correctly classifies as
  O(n² log n). Tarjan SCC + topo-walk; callees analyzed first.
- **Mutual recursion.** SCC of size > 1 → every member is
  Recursive.

### Cases the analyzer **cannot** compute

Some shapes are genuinely out of reach without a much heavier
analyzer. The compiler emits `O(?)` (the `BigO::Unknown`
variant) or falls back to the conservative `O(recursive)` /
`O(n^k)` upper bound. Read source intent when these appear:

1. **`while` loops with non-trivial termination.** `while
   tree[node].next != -1 { ... }` walks a linked list whose
   length the analyzer can't bound. The compiler treats every
   while loop as `O(n)`. If the loop genuinely runs a constant
   number of times (e.g. converging a fixed-point in 3
   iterations), the annotation will over-report.
2. **Recurrence-driven recursion.** Merge sort
   (`T(n) = 2T(n/2) + O(n) = O(n log n)`) is honestly
   `O(recursive)` in vāṇी today — no closed-form recurrence
   solver in v1.
3. **Indirect calls via `dyn Iface`.** A method call through
   a `dyn` trait object dispatches at runtime; the compiler
   sees the iface name but doesn't know which concrete
   implementation's complexity will run. Treated as `O(1)`
   conservatively — accurate when every impl is `O(1)`,
   under-reporting if any impl is heavier.
4. **Closures stored in a binding.** `fn(...) -> R` and
   `Closure<T1, T2>` fat pointers route the same way — the
   analyzer doesn't follow the closure's body across the call.
5. **`extern "C"` FFI calls.** Opaque to the analyzer.
   Treated as `O(1)`; the user's responsibility to know the
   C function's actual cost.
6. **HashMap / BTreeMap with user-defined `Hash` impls.**
   The builtin asymptotic table assumes `O(1)` hashing for
   HashMap and `O(log n)` for BTreeMap. If the user's `Hash
   for K` impl is itself non-constant (rare but possible —
   e.g. hashing a Vec by its full contents), the analyzer
   doesn't pull through. Slight under-report.
7. **Calls to user-defined helpers analyzed AFTER the caller.**
   Theoretically impossible given the topo-walk, but if a fn
   isn't in the program's function list (e.g. a synthesized
   v3.1 Task fn or an inlined helper), the lookup returns
   `BigO::Constant` as default.

### Cases the analyzer over-reports

- **Convergent while loops.** `while delta > epsilon { delta =
  delta / 2; }` converges in O(log(initial/epsilon)) iterations,
  not O(n). Analyzer says O(n).
- **Bounded recursion via SMT.** A function with `requires n
  < 10` recurses at most 10 times — bounded — but the
  analyzer doesn't read `requires` clauses to refine. Reports
  `O(recursive)`.

### Cases the analyzer under-reports

- **Mutating non-passed-in state.** A fn that writes to a
  global Vec at depth d still classifies by the local body's
  depth. There's no truly-global state in v1 so this is rare.
- **HashMap with adversarial hash collisions.** Builtin
  asymptotic is `O(1)` per op; pathological inputs degrade
  to `O(n)`. Analyzer doesn't model.

### When to ignore the annotation

The annotation is a **hint**, not a contract. Read the
source when the complexity matters for correctness:
- Any while loop whose termination depends on data shape.
- Any recursive function that doesn't fit a simple
  decrement-and-recurse pattern.
- Anything calling through `dyn Iface` or `Closure<...>`.
- Anything calling out via FFI.

For the cases the analyzer handles, the bound is correct
within v1's modeling — and the more code you write through
the bounded paths (arrays, fixed-size loops, simple
recursion), the more reliable the annotation becomes.

## A summary you can carry

- Big-O = how running time GROWS with input size, not how long
  it takes in milliseconds.
- The five shapes to know: `O(1)`, `O(log n)`, `O(n)`,
  `O(n log n)`, `O(n²)`.
- vāṇी's `--big-o` flag walks each fn statically and prints
  the inferred annotation. Three modes: `auto` (default, skip
  O(1)), `force` (every fn), `off`.
- Annotation lives in the check output AND as a comment
  prepended to the emitted artifact, so reviewers see the
  cost contract alongside the code.
- The analyzer is conservative + local — over-reports on
  bounded loops, under-reports across helpers. Always sanity-
  check the source when complexity matters.

The takeaway: **complexity is a growth shape, not a clock
reading.** Reading Big-O notation is the same skill as
reading types — once you see `O(n²)`, you know what shape of
work to expect.

## Cross-reference

- [Beginner 7 — Vec and arrays](07_vec_arrays.md) — the data
  structure most Big-O reasoning is about
- [Intermediate 12a — SMT primer](../intermediate/12a_smt_primer.md)
  — proving stronger bounds at compile time (the SMT layer
  can sometimes lift a runtime `O(?)` to a static `O(...)`
  via `requires` / `ensures` clauses)
- [Advanced 10 — Compiler internals tour](../advanced/10_internals.md)
  — the static-analysis pass that builds the annotation lives
  in `src/big_o.rs`
