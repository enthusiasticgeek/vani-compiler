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
