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
(v0.5, 2026-07-01). The C backend emits `#pragma omp parallel for
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

## Current performance status — SSA LLVM backend (v0.6)

Numbers from a representative run (Intel i5-1035G1, Windows 11, gcc/rustc -O2):

| Benchmark | vāṇī | C | C++ | Rust | vs C |
|-----------|------|---|-----|------|------|
| Fibonacci (42) | 876 ms | 519 ms | 524 ms | 833 ms | 1.7× |
| Sieve (2 M) | 51 ms | 13 ms | 28 ms | 14 ms | 3.9× |
| Matrix (256×256) | 24 ms | 12 ms | 18 ms | 30 ms | 2.0× |
| Sort (1 M) | 196 ms | 162 ms | 97 ms | 39 ms | 1.2× |
| BFS (1 K nodes × 1 K) | 44 ms | 12 ms | 26 ms | 16 ms | 3.5× |
| Parallel sum (50 M) | 474 ms | 107 ms | 131 ms | 195 ms | 4.4× |
| HashMap (500 K) | 51 ms | 45 ms | 56 ms | 78 ms | 1.1× |
| Linked list (1 M) | 19 ms | 13 ms | 17 ms | 18 ms | 1.5× |
| Alloc stress (500 K) | 16 ms | 20 ms | 19 ms | 15 ms | **0.8× (wins)** |
| Array stats (10 M) | 82 ms | 27 ms | 31 ms | 50 ms | 3.1× |

---

## Why is vāṇī sometimes slower?

The short answer: **element size**, **bounds checking**, and **Vec construction
overhead** — not the ownership model.

### Where vāṇī wins or ties

| Benchmark | Result | Why |
|-----------|--------|-----|
| Alloc stress | **vāṇī wins** | RAII drop matches manual `free`; zero per-dealloc overhead |
| HashMap | within 10% of C | splitmix64 hash + 75% load factor matches hand-rolled C |
| Linked list | within 50% of C | index idiom same as C `int[]`; gap is while-loop overhead |
| Sort vs C | within 25% | both use `qsort` with function-pointer comparator |
| Fibonacci vs Rust | tie | pure recursion, same code after inlining |

### Where vāṇī lags and why

#### 1. Sieve — 3.9× vs C: element-size mismatch

vāṇī's `sieve` is `Vec<i64>` (8 bytes/element). C uses `char` (1 byte).
The inner marking loop (`while j <= limit { set(mut ref sieve, j, 0) }`)
moves **8× more data** through cache. With `set_mut` inlined (v0.6), the
inner loop itself is now a single GEP + store — the remaining gap is almost
entirely cache bandwidth, not code quality.

A `Vec<Bool>` type (1-bit packed) would close this gap to ~1.5×; it is on
the roadmap but not yet implemented.

#### 2. BFS — 3.5× vs C: bounds checks + while-loop overhead

Every `xs[i]` read emits an inline bounds check (`icmp ult idx, len` +
conditional branch). For BFS's inner loop over adjacency lists, that is 3
bounds checks per edge visit. LLVM's ConstraintElimination eliminates the
`queue[head]` check (same condition as the outer `while head < queue.len`),
but the adjacency-list and visited-array checks remain.

Additionally, the BFS queue starts empty and grows dynamically; `push_mut`
(now inlined, v0.6) still has a capacity-check branch every push.

#### 3. Parallel sum / Array stats — Vec construction

Both benchmarks build a `Vec<i64>` with 50 M / 10 M sequential `push` calls
before the parallel computation. Vec construction — initial alloc + repeated
`realloc` doubling + 50 M/10 M pointer writes — accounts for most of the
wall-clock gap. The actual parallel sum is fast; the construction is not.

A `vec_with_capacity(n)` builtin (pre-allocate at known size) would eliminate
the realloc doubling and reduce construction time by ~3×. Planned for a future
release.

#### 4. Fibonacci — 1.7× vs C: recursive call overhead

vāṇī emits calls with the platform ABI (arguments on stack, full calling
convention). At 331 million recursive `fib` calls for `fib(42)`, the overhead
per call adds up. C's gcc applies `-finline-functions` + `-foptimize-sibling-calls`
more aggressively. No language fix available; recursive fib is inherently
call-heavy.

#### 5. Matrix — 2× vs C: no SIMD

The inner `k` loop is auto-vectorised by gcc on the C/C++ variants but not yet
on the SSA LLVM path. `__restrict__` hints and `!alias.scope` metadata are not
yet emitted. Planned.

### What is *not* the cause

- **Ownership model**: index handles into flat `Vec<T>` are zero-overhead by
  design — no atomic reference count, no GC pause, no pointer-chase indirection.
  Benchmark 05 shows this directly: vāṇī index handles match or beat C++ index
  arrays and are 3-8× faster than C++ `weak_ptr`.

- **`parallel for … reduce`**: thread-local accumulation (v0.5) eliminates all
  per-element atomic ops. The parallel sum gap vs C is not the parallel part —
  it is the Vec construction before the loop starts.

- **RAII / destructors**: the alloc-stress benchmark (500 K alloc/free cycles)
  shows vāṇī *faster* than both C and C++ (v0.6 run).

---

## Open gaps (compiler, not user code)

| Benchmark | Gap | Root cause | Planned fix |
|-----------|-----|------------|-------------|
| Sieve | 3.9× | `Vec<i64>` vs `char` — 8× memory bandwidth | `Vec<Bool>` packed type |
| BFS | 3.5× | bounds checks on adjacency/visited arrays | Loop-range assertion (`@llvm.assume(upper < len)` before loop) |
| Parallel sum | 4.4× | Vec construction: 50 M sequential `push` | `vec_with_capacity(n)` builtin |
| Array stats | 3.1× | Vec construction + while-loop overhead | Same as parallel sum |
| Fibonacci | 1.7× | Recursive call overhead, no sibling-call opt | TCO / tail-call elimination |
| Matrix | 2× | No SIMD / `__restrict__` hints | LLVM alias metadata |
| Sort | 1.2× | qsort function-pointer comparator (same as C) | Inline introsort for SSA path |

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
