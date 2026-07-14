# Benchmark Results — vāṇī vs Rust vs C vs C++

*Generated: 2026-07-13 20:17 — 3 timing run(s) per benchmark, median reported.*
*C/C++ flags: `-O3 -march=native`. Rust flags: `-C opt-level=3 -C target-cpu=native`.*
*vāṇī uses LLVM backend with `opt -O3 --mcpu=native` + `llc -O3 -mcpu=native`.*

## System
```
OS       : Windows 11 AMD64
Python   : 3.14.5
vanic    : C:\Users\upaas\vani-compiler\target\release\vanic.exe
CC       : C:\msys64\mingw64\bin\gcc.EXE
CXX      : C:\msys64\mingw64\bin\g++.EXE
rustc    : C:\Users\upaas\.cargo\bin\rustc.EXE
```

## Summary

| Benchmark | vani         | c            | cpp          | rs           | cpp_idx      | cpp_weak     |
|--------------------|--------------|--------------|--------------|--------------|--------------|--------------|
| Fibonacci(42) — re |   943.1 ms   |   486.2 ms   |   488.6 ms   |   930.7 ms   | —            | —            |
| Sieve of Eratosthe |    15.4 ms   |    14.6 ms   |    14.9 ms   |    15.5 ms   | —            | —            |
| Matrix multiplicat |    15.5 ms   |    15.6 ms   |    15.5 ms   |    32.9 ms   | —            | —            |
| Sort 1 000 000 int |    97.1 ms   |   180.8 ms   |    98.6 ms   |    44.1 ms   | —            | —            |
| Graph BFS — index  |    16.2 ms   |    10.9 ms   | —            |    18.6 ms   |    19.2 ms   |    51.7 ms   |
| Parallel sum — 50  |   197.2 ms   |   193.1 ms   |   198.3 ms   |   151.1 ms   | —            | —            |
| HashMap — 500 000  |    39.7 ms   |    60.0 ms   |    60.9 ms   |    73.5 ms   | —            | —            |
| Linked list — 1 00 |    13.7 ms   |    15.4 ms   |    17.5 ms   |    21.3 ms   | —            | —            |
| Allocation stress  |    10.8 ms   |    10.3 ms   |    16.0 ms   |    14.7 ms   | —            | —            |
| Array statistics — |    37.9 ms   |    61.9 ms   |    68.5 ms   |    65.4 ms   | —            | —            |
| SIMD dot product — |    30.3 ms   |    33.7 ms   |    42.9 ms   |    42.5 ms   | —            | —            |
| SIMD-256 dot produ |    33.5 ms   | —            | —            | —            | —            | —            |

## Per-benchmark charts

> Bars are proportional to wall-clock time — **shorter is faster**.

### Fibonacci(42) — recursive

*Classic recursive fib(42). Tests raw function-call throughput.*

```
  vani           ████████████████████████████████████    943.1 ms    baseline
  c              ███████████████████░░░░░░░░░░░░░░░░░    486.2 ms   48.4% faster
  cpp            ███████████████████░░░░░░░░░░░░░░░░░    488.6 ms   48.2% faster
  rs             ████████████████████████████████████    930.7 ms    1.3% faster
```

### Sieve of Eratosthenes — primes ≤ 2 000 000

*Boolean sieve with Vec-set inner loop. Tests dense random-access array writes.*

```
  vani           ████████████████████████████████████     15.4 ms    baseline
  c              ██████████████████████████████████░░     14.6 ms    5.6% faster
  cpp            ██████████████████████████████████░░     14.9 ms    3.5% faster
  rs             ████████████████████████████████████     15.5 ms    0.8% slower
```

### Matrix multiplication 256×256 (i64)

*Naïve triple-loop matmul. Tests arithmetic-dense nested loops.*

```
  vani           █████████████████░░░░░░░░░░░░░░░░░░░     15.5 ms    baseline
  c              █████████████████░░░░░░░░░░░░░░░░░░░     15.6 ms    0.7% slower
  cpp            █████████████████░░░░░░░░░░░░░░░░░░░     15.5 ms    0.4% faster
  rs             ████████████████████████████████████     32.9 ms   111.9% slower
```

### Sort 1 000 000 integers

*vāṇī uses the built-in sort(mut ref xs); others use stdlib. Tests sort quality.*

```
  vani           ███████████████████░░░░░░░░░░░░░░░░░     97.1 ms    baseline
  c              ████████████████████████████████████    180.8 ms   86.1% slower
  cpp            ████████████████████░░░░░░░░░░░░░░░░     98.6 ms    1.5% slower
  rs             █████████░░░░░░░░░░░░░░░░░░░░░░░░░░░     44.1 ms   54.6% faster
```

### Graph BFS — index handles vs. weak_ptr

*KEY BENCHMARK: BFS on a 1 000-node random graph, repeated 1 000×.
  graph.vani / graph_index.{c,cpp,rs} — int-index adjacency list, zero ref-counting.
  graph_weakptr.cpp                   — shared_ptr children + weak_ptr back-edges.*

```
  vani           ███████████░░░░░░░░░░░░░░░░░░░░░░░░░     16.2 ms    baseline
  c              ████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░     10.9 ms   32.6% faster
  C++ (index)    █████████████░░░░░░░░░░░░░░░░░░░░░░░     19.2 ms   18.3% slower
  C++ (weak_ptr) ████████████████████████████████████     51.7 ms   219.2% slower
  rs             █████████████░░░░░░░░░░░░░░░░░░░░░░░     18.6 ms   14.7% slower
```

### Parallel sum — 50 000 000 elements

*vāṇī: `parallel for … reduce total with +` (3 extra keywords).
C/C++: OpenMP (if available), else serial.
Rust: std::thread manual split.*

```
  vani           ████████████████████████████████████    197.2 ms    baseline
  c              ███████████████████████████████████░    193.1 ms    2.1% faster
  cpp            ████████████████████████████████████    198.3 ms    0.6% slower
  rs             ███████████████████████████░░░░░░░░░    151.1 ms   23.3% faster
```

### HashMap — 500 000 insert + 500 000 lookup

*Tests open-addressing HashMap throughput.*

```
  vani           ███████████████████░░░░░░░░░░░░░░░░░     39.7 ms    baseline
  c              █████████████████████████████░░░░░░░     60.0 ms   51.0% slower
  cpp            ██████████████████████████████░░░░░░     60.9 ms   53.3% slower
  rs             ████████████████████████████████████     73.5 ms   85.0% slower
```

### Linked list — 1 000 000 nodes, index-based

*vāṇī/C index approach (no raw pointers): O(1) cache-friendly traversal.
C++/Rust use traditional pointer-linked nodes for comparison.*

```
  vani           ███████████████████████░░░░░░░░░░░░░     13.7 ms    baseline
  c              ██████████████████████████░░░░░░░░░░     15.4 ms   12.2% slower
  cpp            █████████████████████████████░░░░░░░     17.5 ms   27.3% slower
  rs             ████████████████████████████████████     21.3 ms   55.4% slower
```

### Allocation stress — 500 000 struct alloc/free cycles

*Tests allocator throughput; vāṇī uses RAII affine drop.*

```
  vani           ████████████████████████░░░░░░░░░░░░     10.8 ms    baseline
  c              ███████████████████████░░░░░░░░░░░░░     10.3 ms    4.6% faster
  cpp            ████████████████████████████████████     16.0 ms   47.5% slower
  rs             █████████████████████████████████░░░     14.7 ms   36.2% slower
```

### Array statistics — mean + variance of 10 000 000 values

*vāṇī: two `parallel for … reduce` passes. C/C++/Rust: sequential passes. Tests loop throughput and parallelism.*

```
  vani           ████████████████████░░░░░░░░░░░░░░░░     37.9 ms    baseline
  c              █████████████████████████████████░░░     61.9 ms   63.4% slower
  cpp            ████████████████████████████████████     68.5 ms   80.6% slower
  rs             ██████████████████████████████████░░     65.4 ms   72.5% slower
```

### SIMD dot product — explicit vec128<f32> vs auto-vectorized (4 M elements)

*vāṇī: explicit vec128<f32> simd_mul + simd_reduce_add. C/C++/Rust: scalar loop auto-vectorized by compiler. Compares explicit SIMD vs optimizer output.*

```
  vani           █████████████████████████░░░░░░░░░░░     30.3 ms    baseline
  c              ████████████████████████████░░░░░░░░     33.7 ms   11.1% slower
  cpp            ████████████████████████████████████     42.9 ms   41.4% slower
  rs             ████████████████████████████████████     42.5 ms   40.1% slower
```

### SIMD-256 dot product — vec256<f32> vs vec128<f32> vs scalar (4 M elements)

*vani-only: vec256 (ymm/SVE) vs vec128 (xmm/NEON) vs auto-vectorized scalar.*

```
  vani           ████████████████████████████████████     33.5 ms    baseline
```

## Key insight: index handles vs. `weak_ptr`

Benchmark `05_graph_bfs` is the most architecture-revealing comparison.
vāṇī has no `weak_ptr` equivalent — its **affine ownership model** means
pointers cannot be aliased without explicit `ref`/`mut ref` borrows, which
makes cyclic references impossible to express directly. Instead, cyclic
graphs are stored as **integer indices** into a contiguous `Vec<T>`.

| Approach | Heap allocs | Atomic ops | Cache friendly |
|----------|-------------|------------|----------------|
| C++ `weak_ptr` | one per node | `lock()` ≥ 2 per access | poor (pointer chase) |
| vāṇī / C++ index | zero (flat Vec) | none | excellent (contiguous) |

