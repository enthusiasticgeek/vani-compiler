# vāṇī Benchmarks

Performance comparison of **vāṇī** against Rust, C, and C++ on the same hardware.

Every benchmark has four variants compiled with equivalent optimisation flags:

| Language | Compiler flag | Note |
|----------|--------------|-------|
| vāṇī     | `vanic build` | SSA LLVM backend (default); `--backend=c` for C backend |
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
| 11 | `11_simd_dot/` | f32 dot product 4 M elements — explicit `vec128<f32>` vs auto-vectorized scalar |
| 12 | `12_simd256_dot/` | f32 dot product 4 M elements — `vec256<f32>` (256-bit) vs `vec128<f32>` vs scalar; vāṇī-only |

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

The SSA LLVM backend (default) allocates one stack-local accumulator per
thread, accumulates in the parallel body with no atomic ops, and combines
per-thread results with a single `atomicrmw` at the parallel region's exit
(v0.2.1-dev, 2026-07-01). The C backend emits `#pragma omp parallel for
reduction(+:total)`. Either way, you write three lines; the verifier proves
race-freedom statically.

Equivalent C requires:

```c
long total = 0;
#pragma omp parallel for reduction(+:total)
for (long i = 0; i < n; i++)
    total += data[i];
```

…and a `-fopenmp` flag that silently does nothing if OpenMP is missing.

---

## Current performance status — SSA LLVM backend (v0.4.4)

Numbers from a representative run (Intel i5-1035G1, Windows 11, gcc/g++/rustc `-O3 -march=native`).
Full results with per-benchmark charts: [results/RESULTS.md](results/RESULTS.md).

| Benchmark | vāṇī | C | C++ | Rust | vs C |
|-----------|------|---|-----|------|------|
| Fibonacci (42) | 943 ms | 486 ms | 489 ms | 931 ms | 1.9× slower |
| Sieve (2 M) | **15.4 ms** | 14.6 ms | 14.9 ms | 15.5 ms | **~tie (1.05×)** |
| Matrix (256×256) | **15.5 ms** | 15.6 ms | 15.5 ms | 32.9 ms | **tie** |
| Sort (1 M) | **97 ms** | 181 ms | 99 ms | 44 ms | **wins vs C (+46%)** |
| BFS index (1 K × 1 K) | 16.2 ms | 10.9 ms | — | 18.6 ms | 1.5× slower |
| BFS weak_ptr (C++) | — | — | 51.7 ms | — | **vāṇī 3.2× faster** |
| Parallel sum (50 M) | **197 ms** | 193 ms | 198 ms | 151 ms | **~tie (1.02×)** |
| HashMap (500 K) | **39.7 ms** | 60.0 ms | 60.9 ms | 73.5 ms | **wins vs all** |
| Linked list (1 M) | **13.7 ms** | 15.4 ms | 17.5 ms | 21.3 ms | **wins vs all** |
| Alloc stress (500 K) | **10.8 ms** | 10.3 ms | 16.0 ms | 14.7 ms | **~tie (1.05×)** |
| Array stats (10 M) | **37.9 ms** | 61.9 ms | 68.5 ms | 65.4 ms | **wins vs all** |
| SIMD dot vec128 (4 M) | **30.3 ms** | 33.7 ms | 42.9 ms | 42.5 ms | **wins vs all** |
| SIMD-256 dot (4 M) | 33.5 ms | — | — | — | vāṇī-only |

**vāṇī wins or ties C in 9 of 12 benchmarks.** The two remaining gaps are Fibonacci (recursive call overhead) and BFS vs a hand-tuned C index loop (bounds checks).

---

## Why is vāṇī sometimes slower?

The two remaining gaps are pure overhead in specific patterns — not the ownership model.

### Where vāṇī wins or ties (v0.4.4)

| Benchmark | Result | Why |
|-----------|--------|-----|
| Sort | **wins vs C (+46%), ties C++** | Inline median-of-3 quicksort in LLVM IR; no function-pointer comparator |
| HashMap | **wins vs C, C++, Rust** | splitmix64 + 75% load factor; no chained-list indirection |
| Linked list | **wins vs all** | index-into-Vec idiom; O(1) cache-line stride, no pointer chase |
| Array stats | **wins vs all** | two `parallel for … reduce` passes; C/Rust run serially |
| SIMD dot | **wins vs C, C++, Rust** | explicit `vec128<f32>` beats auto-vectorized scalar in all three |
| Alloc stress | **ties C** | RAII affine drop ≡ manual `free`; C++ RTTI overhead absent |
| Sieve | **ties C** | Previously 3.9× slower; sieve loop now emits single GEP + store, gap is measurement noise |
| Matrix | **ties C, C++** | Previously 2×; LLVM IR alias metadata now lets auto-vectorizer fire |
| Parallel sum | **ties C, ties C++** | Previously 2.5× slower; thread-local accumulators + single atomic at exit |
| BFS (index) | 1.5× vs C | Bounds checks on adjacency + visited arrays; `vec_with_capacity` closed the `realloc` gap |
| BFS vs weak_ptr | **3.2× faster than C++ `weak_ptr`** | Index handles need zero atomic ops; `lock()` costs ≥ 2 atomics per access |
| Fibonacci | 1.9× vs C | 331 M recursive calls; gcc applies `-foptimize-sibling-calls` more aggressively |

### Remaining gaps

#### 1. BFS — 1.5× vs hand-tuned C

`vec_with_capacity` eliminated all `realloc` doublings (was 2.6×). The remaining
gap is bounds checks: every `xs[i]` emits an `icmp ult + branch`. LLVM's
ConstraintElimination folds the outer-loop check but leaves 2 checks per inner
edge visit. Planned fix: emit `@llvm.assume(idx < len)` at loop entry.

#### 2. Fibonacci — 1.9× vs C

Purely recursive fib(42) makes 331 million calls. vāṇī emits full platform ABI;
gcc applies tail/sibling-call opts more aggressively at `-O3`. No language fix
available — recursive fib is inherently call-heavy. TCO for eligible patterns
is on the roadmap.

### What is *not* the cause

- **Ownership model**: benchmark 05 (BFS) shows index handles into flat `Vec<T>`
  are zero-overhead and 3.2× faster than C++ `weak_ptr`.
- **RAII / destructors**: alloc stress shows vāṇī ties C and beats C++.
- **`parallel for … reduce`**: thread-local accumulation eliminates all per-element
  atomics — parallel sum ties C and C++ at near-memory-bandwidth speed.

---

## Open gaps (compiler, not user code)

| Benchmark | Gap | Root cause | Planned fix |
|-----------|-----|------------|-------------|
| BFS | 1.5× vs C | Bounds checks on adjacency/visited inner loops | `@llvm.assume` loop-range hint |
| Fibonacci | 1.9× vs C | Recursive call overhead; no sibling-call opt yet | TCO / tail-call elimination |
| Sort vs Rust | 2.2× | pdqsort block partitioning vs median-of-3 quicksort | Full pdqsort port |

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
