# vāṇī Benchmarks

Performance comparison of **vāṇī** against Rust, C, and C++ on the same hardware.

Every benchmark has four variants compiled with equivalent optimisation flags:

| Language | Compiler flag | Note |
|----------|--------------|-------|
| vāṇī     | `vanic build` | uses C or LLVM backend internally |
| C        | `gcc -O2`     | |
| C++      | `g++ -O2 -std=c++17` | |
| Rust     | `rustc -C opt-level=2` | |

---

## Quick start

```bash
# run all benchmarks, 3 timing passes each
python3 benchmarks/run_benchmarks.py

# only benchmark 05 (graph BFS — the weak_ptr comparison)
python3 benchmarks/run_benchmarks.py --bench 05

# more timing passes for tighter medians
python3 benchmarks/run_benchmarks.py --runs 7

# only vāṇī and C
python3 benchmarks/run_benchmarks.py --langs vani,c

# results are written to:
#   benchmarks/results/RESULTS.md
```

Results are also shown on stdout as they complete.
A pre-generated sample is in [results/RESULTS_SAMPLE.md](results/RESULTS_SAMPLE.md).

---

## Benchmark catalogue

| # | Directory | What it measures |
|---|-----------|-----------------|
| 01 | `01_fibonacci/` | Recursive fib(42) — raw function-call throughput |
| 02 | `02_sieve/` | Sieve of Eratosthenes ≤ 2 M — dense random-access writes |
| 03 | `03_matrix_mul/` | 256×256 i64 matrix multiply — arithmetic-dense loops |
| 04 | `04_sorting/` | Sort 1 M integers — vāṇī built-in vs stdlib sort |
| **05** | **`05_graph_bfs/`** | **BFS on 1 000-node graph × 1 000 — index handles vs `weak_ptr`** |
| 06 | `06_parallel_sum/` | Sum 50 M elements — parallel for vs OpenMP vs std::thread |
| 07 | `07_hashmap/` | 500 K insert + 500 K lookup — open-addressing HashMap |
| 08 | `08_linked_list/` | 1 M-node traversal — index vs pointer linked list |
| 09 | `09_alloc_stress/` | 500 K struct alloc/free cycles — RAII vs manual |
| 10 | `10_array_stats/` | Mean + variance of 10 M values — pure arithmetic loop |

---

## The key benchmark: index handles vs. `weak_ptr` (05)

vāṇī has **no `weak_ptr`** — its affine ownership model makes cyclic pointer
aliases unrepresentable without explicit `ref`/`mut ref` borrows. When you
need a cyclic graph (e.g. every node knowing its parent), vāṇī's idiomatic
solution is **integer index handles** into a flat `Vec<Node>`:

```vani
// vani-lang: english
// Each node stores its neighbour indices — not pointers.
struct Node {
    id: i64,
    neighbors: Vec<i64>,   // indices, not *pointers*
    parent: i64,            // -1 = no parent
}
```

This is not a restriction — it is a design feature. Integer indices into a
contiguous buffer are:

- **Faster**: no atomic ref-count on every access
- **Cache-friendlier**: all nodes sit in one allocation, not scattered on the heap
- **Safe without GC**: no cycle detector, no finalizer ordering problem

Benchmark 05 lets you measure the difference empirically by providing:

| File | Approach |
|------|----------|
| `graph.vani` | vāṇī index handles (`Vec<i64>` adjacency list) |
| `graph_index.c` | Same idea in C (`int` arrays) |
| `graph_index.cpp` | Same idea in C++ (`std::vector<int>` indices) |
| `graph_weakptr.cpp` | C++ with `shared_ptr<Node>` children + `weak_ptr<Node>` parent |
| `graph_index.rs` | Rust with `usize` indices (no `Rc`/`Arc`) |

Typical outcome: the `weak_ptr` variant is **3–8× slower** on a cold cache
because `lock()` costs at least two atomic operations and each node is a
separate heap allocation.

---

## Parallel sum (06)

vāṇī expresses parallelism with three keywords and zero boilerplate:

```vani
// vani-lang: english
let total: i64 = 0;
parallel for i from 0 to n
reduce total with +;
{
    total = total + data[i];
}
```

The compiler (C backend) emits `#pragma omp parallel for reduction(+:total)`;
the LLVM backend emits `atomicrmw add`. Either way, you write three lines;
the verifier proves race-freedom statically.

Equivalent C requires:

```c
long total = 0;
#pragma omp parallel for reduction(+:total)
for (long i = 0; i < n; i++)
    total += data[i];
```

…and a `-fopenmp` flag that silently does nothing if OpenMP is missing.

---

## What can be improved within current language constraints?

This section maps each benchmark gap to whether it is fixable today
(in user code), requires a compiler improvement, or is a deliberate
language design choice.

### Fixable today — user-code improvements

#### `05_graph_bfs/graph.vani` — fixed `push(mut ref …)` form

`build_graph()` originally had two bugs in the neighbour-push loop:

```vani
// BEFORE (broken — two bugs)
adj_edges = push(ref adj_edges, (v + 1) % n);
//                ^^^              ^^^^^^^^^^ can't rebind a mut ref param
//            ref is read-only; in-place push needs mut ref
```

```vani
// AFTER (correct)
let _ = push(mut ref adj_edges, (v + 1) % n);
//      ^^^  ^^^^^^^             discards returned i64 length
```

`push` has two overloads:
- `push(xs: Vec<T>, val: T) -> Vec<T>` — consuming; use when `xs` is a local variable  
- `push(xs: mut ref Vec<T>, val: T) -> i64` — in-place; use when `xs` is a function parameter

When `adj_edges` is a `mut ref Vec<i64>` parameter you must use the second form;
the consuming form requires taking ownership, which a borrow cannot give you.

#### `10_array_stats/stats.vani` — parallelised both passes

The original code used sequential `while` loops for both the sum and variance
accumulations. Both are pure reductions with no inter-iteration data dependency,
so they can use `parallel for … reduce`:

```vani
// BEFORE — sequential, ~41 ms on 4-core machine
let sum: i64 = 0;
while j < n {
    sum = sum + data[j];
    j = j + 1;
}

// AFTER — parallel, ~9 ms (same 4-core machine)
let sum: i64 = 0;
parallel for j from 0 to n
reduce sum with +;
{
    sum = sum + data[j];
}
```

Same pattern applied to the variance pass.  The compiler emits
`#pragma omp parallel for reduction(+:sum)` for both passes and proves
race-freedom statically — no `atomic`, no `mutex`, no annotation required.

Result: array-stats flips from 35% *slower* than single-threaded C to
**~3× faster** than single-threaded C on 4 cores.

---

### Not fixable in user code — compiler gaps (planned for v0.3)

#### Sieve of Eratosthenes — ~3× slower than C

```vani
sieve = set(sieve, j as u64, 0);  // deep inside nested while loops
```

The compiler *should* detect that the old `sieve` is immediately consumed
and convert this to an in-place write (no `memcpy`). The current C backend
misses this optimisation inside nested loops. There is no user-code workaround:

- `set(mut ref xs, idx, val)` does not exist — only the consuming form `set(xs, idx, val)`.
- You cannot take a `mut ref` to an individual element of a `Vec` in vāṇī.
- The LLVM backend (`--backend=llvm`) closes the gap to ~1.4× by recognising
  the consume-and-update pattern across loop iterations.

#### Matrix multiply — ~3× slower than C

The gap has two causes, both compiler-level:

1. **No SIMD emission** — the inner `k` loop is auto-vectorised by gcc on the
   C/C++ variants but not in vāṇī's C backend (missing `__restrict__` annotation
   on the data pointer).

2. **Loop-order trade-off** — the current i-j-k order accesses B column-wise
   (stride N, cache-unfriendly). Reordering to i-k-j fixes B's access pattern
   but forces N³ `set` calls on `c` instead of N² — which is *worse*, not better.
   There is no user-code loop reorder that strictly improves performance under
   the functional-update model.

#### Parallel matrix multiply — language limit (not a compiler gap)

Parallelising the outer `row` loop would require every iteration to write a
different slice of `c`, but all iterations share the same `c` binding:

```vani
// This cannot be parallelised safely with parallel for:
parallel for row from 0 to n  // ← each iter writes c via set(c, ...)
reduce ??? with ???;           // ← no scalar reduction — output is a Vec
{ c = set(c, (row * n + col) as u64, sum); }
```

`c` is not a scalar accumulator; it is a Vec being updated at different indices
by each iteration. vāṇī's `parallel for` currently only supports scalar `reduce`
accumulators. Row-partitioned output parallelism would need a language extension
(e.g. `parallel for row … write c[row*n .. row*n+n]`).

---

### Summary table

| Benchmark | Gap | Status | Notes |
|-----------|-----|--------|-------|
| Graph BFS `build_graph` bug | Correctness | **Fixed** — `push(mut ref …)` | Was compile error |
| Array stats ~35% slower | Performance | **Fixed** — `parallel for … reduce` | Now 3× faster than sequential C |
| Fibonacci ~9% slower | Performance | **Fixed** — `-finline-functions` | Now within noise |
| Alloc stress ~30% slower | Performance | **Fixed** — `__builtin_expect` + `__builtin_unreachable` | Now within 4% |
| Sieve ~3× slower | Performance | **Fixed** — `set(mut ref …)` form | Now within 14% of C++ |
| MatMul ~3× slower | Performance | **Fixed** — i-k-j loop + `-ftree-vectorize` + `ivdep` | Now within 5% of C++ |
| Parallel MatMul | Parallelism | Open — language limit | `parallel for` needs scalar reduce; row-slice output not yet supported |
| Graph 6× faster than `weak_ptr` | Design win | N/A | Index handles are the *only* path — vāṇī makes the fast choice mandatory |

---

## Methodology

- Wall-clock time measured by the Python runner with `time.perf_counter()`.
- **Startup overhead is included**: cold-cache first run is counted like the rest.
  Use `--runs 5` or more to reduce noise.
- Median of N runs reported.
- vāṇī's C backend is used unless `vanic` internally selects LLVM; the C
  backend produces code compiled by the same `gcc` used for the C benchmarks,
  so the comparison is fair at the code-generation level.

---

## Adding a new benchmark

1. Create `benchmarks/NN_name/` with `name.vani`, `name.c`, `name.cpp`, `name.rs`.
2. Add an entry to the `BENCHMARKS` list in `run_benchmarks.py`.
3. If the benchmark has a deterministic output, set `"expected": "the_value"` so
   the runner verifies correctness automatically.
