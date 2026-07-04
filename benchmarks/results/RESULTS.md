# Benchmark Results — vāṇī vs C

*Updated: 2026-07-04 — 7 timing runs per benchmark, trimmed mean (drop min/max).
C compiled with `gcc -O3 -march=native` (plus `-fopenmp` for parallel benchmarks).
vāṇī uses LLVM backend with `-O3 -mcpu=native` and full optimization pipeline.*

## System
```
OS       : Windows 11 AMD64 (8 logical CPUs)
vanic    : C:\Users\upaas\vani-compiler\target\release\vanic.EXE
CC       : gcc (MinGW64) -O3 -march=native
```

## Summary

| Benchmark | vāṇī | C | ratio |
|-----------|-------|---|-------|
| Fibonacci(42) — recursive | 834 ms | 456 ms | 1.83× C faster |
| Sieve of Eratosthenes ≤ 2M | 47 ms | 15 ms | 3.08× C faster (Vec<i8> pending) |
| Matrix multiplication 256×256 | 13 ms | 12 ms | 1.12× C faster |
| Sort 1 000 000 integers | 94 ms | 159 ms | **0.59× vāṇī faster** |
| Graph BFS — 1000 nodes × 1000 runs | 13 ms | 12 ms | 1.10× C faster |
| Parallel sum — 50 000 000 elements | 160 ms | 61 ms | 2.61× C faster (4T vs OpenMP 8T) |
| HashMap — 500K insert + 500K lookup | 56 ms | 29 ms | 1.94× C faster |
| Linked list — 1 000 000 nodes | 12 ms | 11 ms | 1.11× C faster |
| Allocation stress — 500K alloc/free | 17 ms | 10 ms | 1.77× C faster |
| Array statistics — 10 000 000 values | 31 ms | 38 ms | **0.81× vāṇī faster** |

## Per-benchmark results

### Fibonacci(42) — recursive

*Classic recursive fib(42). Tests raw function-call throughput.
Gap is structural: fib(n-1)+fib(n-2) is not tail-callable; each call
allocates a stack frame. vāṇī adds recursion-depth bounds checking overhead.*

```
  vani           ████████████████████████████████████    834 ms    baseline
  c              ███████████████████░░░░░░░░░░░░░░░░░    456 ms   45% faster
```

### Sieve of Eratosthenes — primes ≤ 2 000 000

*Boolean sieve. Gap is structural: vāṇī uses `Vec<i64>` (8 bytes/element) where
C uses `char` (1 byte/element). The 8× larger working set causes 8× more cache
misses. Fix pending: add `Vec<i8>` type to the language.*

```
  vani           ████████████████████████████████████     47 ms    baseline
  c              ████████████░░░░░░░░░░░░░░░░░░░░░░░░     15 ms   68% faster
```

### Matrix multiplication 256×256 (i64)

*Naïve triple-loop matmul. Tests arithmetic-dense nested loops.
noalias on realloc + loop vectorization hints bring this to near-parity.*

```
  vani           ████████████████████████████████████     13 ms    baseline
  c              ████████████████████████████████░░░░     12 ms   12% faster
```

### Sort 1 000 000 integers

*vāṇī uses the built-in `sort(mut ref xs)`; C uses `qsort`. Tests sort quality.*

```
  vani           ████████████████████████████████████     94 ms    baseline
  c              ████████████████████████████████████████████████████████████   159 ms   69% slower
```

### Graph BFS — index handles vs. pointer-linked

*BFS on a 1 000-node random graph, repeated 1 000×.
Alloca hoisting (this session) promoted `head` and `count` to phi nodes,
eliminating redundant load/store cycles. Went from 1.80× to 1.10×.*

```
  vani           ████████████████████████████████████     13 ms    baseline
  c              ████████████████████████████████░░░░     12 ms   10% faster
```

### Parallel sum — 50 000 000 elements

*vāṇī: `parallel for … reduce total with +` (3 extra keywords).
C/C++: OpenMP (`#pragma omp parallel for reduction`).
vāṇī uses 4 threads (baked in via OMP_NUM_THREADS at compile time on Windows).
C uses OpenMP default thread count (~8T on this machine).
Alloca domination fix (this session) restored correctness; binary was previously
built from unoptimized IR because `opt` rejected the IR due to a dominance error.*

```
  vani (4T)      ████████████████████████████████████    160 ms    baseline
  c (OMP ~8T)    ██████████████░░░░░░░░░░░░░░░░░░░░░░     61 ms   62% faster
```

### HashMap — 500 000 insert + 500 000 lookup

*Open-addressing HashMap.*

```
  vani           ████████████████████████████████████     56 ms    baseline
  c              ████████████████████░░░░░░░░░░░░░░░░     29 ms   48% faster
```

### Linked list — 1 000 000 nodes, index-based

*Two parallel `Vec<i64>` for values+next pointers. Sequential traversal.
push_unchecked in init loop (this session) + alloca hoisting brought
this from 1.52× to 1.11× gap.*

```
  vani           ████████████████████████████████████     12 ms    baseline
  c              ████████████████████████████████░░░░     11 ms   11% faster
```

### Allocation stress — 500 000 struct alloc/free cycles

*Tests allocator throughput; vāṇī uses RAII affine drop.*

```
  vani           ████████████████████████████████████     17 ms    baseline
  c              ████████████████████░░░░░░░░░░░░░░░░     10 ms   41% faster
```

### Array statistics — mean + variance of 10 000 000 values

*Two parallel reduction passes (sum → mean, then variance).
vāṇī's `parallel for … reduce` with explicit vectorize metadata beats
C OpenMP for this workload size (80MB fits in L3 cache, vectorizer wins).*

```
  vani           ████████████████████████████████████     31 ms    baseline
  c (OMP)        █████████████████████████████████████████████     38 ms   23% slower
```

## Key optimizations applied (2026-07 session)

| Fix | Benchmark impact |
|-----|-----------------|
| Alloca hoisting (scalar `let` to entry block) | BFS: 1.80× → 1.10× |
| Alloca domination fix for parallel-for local accumulators | parsum: broken → 2.61×; stats: broken → 0.81× |
| `push_unchecked` in init loops | list: 1.52× → 1.11× |
| `!llvm.loop.vectorize.enable/width` on outlined loops | stats: contributes to 0.81× (faster than C) |

## Known remaining gaps

| Gap | Root cause | Fix |
|-----|-----------|-----|
| Sieve 3.08× | `Vec<i8>` not yet in language; uses 8× memory | Add `Vec<i8>` element type |
| Fibonacci 1.83× | Recursion depth check overhead per call | @llvm.assume or depth-elision |
| Parallel sum 2.61× | 4T vs C's ~8T; Windows CreateThread overhead | Runtime thread count; GOMP on Linux |
| HashMap 1.94× | BTreeMap-style internals | Pending investigation |
| Alloc 1.77× | malloc overhead per struct | Slab allocator |

## Key insight: index handles vs. `weak_ptr`

Benchmark `05_graph_bfs` shows vāṇī's ownership model in practice:
vāṇī has no `weak_ptr` — its affine ownership model means pointers
cannot be aliased without explicit borrows, making cyclic references
impossible to express directly. Instead, cyclic graphs are stored as
**integer indices** into a contiguous `Vec<T>`.

| Approach | Heap allocs | Atomic ops | Cache friendly |
|----------|-------------|------------|----------------|
| C++ `weak_ptr` | one per node | `lock()` ≥ 2 per access | poor (pointer chase) |
| vāṇī / C index | zero (flat Vec) | none | excellent (contiguous) |
