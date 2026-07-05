# Benchmark Results — vāṇī vs Rust vs C vs C++

*Generated: 2026-07-05 12:13 — 7 timing run(s) per benchmark, median reported.*
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
| Fibonacci(42) — re |   894.2 ms   |   570.5 ms   |   592.3 ms   |   906.3 ms   | —            | —            |
| Sieve of Eratosthe |    15.5 ms   |    12.4 ms   |    15.3 ms   |    18.5 ms   | —            | —            |
| Matrix multiplicat |    15.8 ms   |    15.8 ms   |    21.2 ms   |    32.6 ms   | —            | —            |
| Sort 1 000 000 int |    95.9 ms   |   184.2 ms   |   103.9 ms   |    42.0 ms   | —            | —            |
| Graph BFS — index  |    14.7 ms   |    12.2 ms   | —            |    17.5 ms   |    18.6 ms   |    47.8 ms   |
| Parallel sum — 50  |   184.2 ms   |   202.8 ms   |   193.1 ms   |   143.9 ms   | —            | —            |
| HashMap — 500 000  |    38.1 ms   |    28.7 ms   |    54.1 ms   |    79.1 ms   | —            | —            |
| Linked list — 1 00 |    13.3 ms   |    14.5 ms   |    17.6 ms   |    19.7 ms   | —            | —            |
| Allocation stress  |    16.9 ms   |    11.4 ms   |    12.7 ms   |    14.8 ms   | —            | —            |
| Array statistics — |    36.1 ms   |    44.2 ms   |    51.4 ms   |    44.5 ms   | —            | —            |

## Per-benchmark charts

> Bars are proportional to wall-clock time — **shorter is faster**.

### Fibonacci(42) — recursive

*Classic recursive fib(42). Tests raw function-call throughput.*

```
  vani           ████████████████████████████████████    894.2 ms    baseline
  c              ███████████████████████░░░░░░░░░░░░░    570.5 ms   36.2% faster
  cpp            ████████████████████████░░░░░░░░░░░░    592.3 ms   33.8% faster
  rs             ████████████████████████████████████    906.3 ms    1.4% slower
```

### Sieve of Eratosthenes — primes ≤ 2 000 000

*Boolean sieve with Vec-set inner loop. Tests dense random-access array writes.*

```
  vani           ██████████████████████████████░░░░░░     15.5 ms    baseline
  c              ████████████████████████░░░░░░░░░░░░     12.4 ms   20.3% faster
  cpp            ██████████████████████████████░░░░░░     15.3 ms    1.4% faster
  rs             ████████████████████████████████████     18.5 ms   19.3% slower
```

### Matrix multiplication 256×256 (i64)

*Naïve triple-loop matmul. Tests arithmetic-dense nested loops.*

```
  vani           █████████████████░░░░░░░░░░░░░░░░░░░     15.8 ms    baseline
  c              █████████████████░░░░░░░░░░░░░░░░░░░     15.8 ms    0.2% slower
  cpp            ███████████████████████░░░░░░░░░░░░░     21.2 ms   34.2% slower
  rs             ████████████████████████████████████     32.6 ms   106.7% slower
```

### Sort 1 000 000 integers

*vāṇī uses the built-in sort(mut ref xs); others use stdlib. Tests sort quality.*

```
  vani           ███████████████████░░░░░░░░░░░░░░░░░     95.9 ms    baseline
  c              ████████████████████████████████████    184.2 ms   92.0% slower
  cpp            ████████████████████░░░░░░░░░░░░░░░░    103.9 ms    8.3% slower
  rs             ████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░     42.0 ms   56.3% faster
```

### Graph BFS — index handles vs. weak_ptr

*KEY BENCHMARK: BFS on a 1 000-node random graph, repeated 1 000×.
  graph.vani / graph_index.{c,cpp,rs} — int-index adjacency list, zero ref-counting.
  graph_weakptr.cpp                   — shared_ptr children + weak_ptr back-edges.*

```
  vani           ███████████░░░░░░░░░░░░░░░░░░░░░░░░░     14.7 ms    baseline
  c              █████████░░░░░░░░░░░░░░░░░░░░░░░░░░░     12.2 ms   16.9% faster
  C++ (index)    ██████████████░░░░░░░░░░░░░░░░░░░░░░     18.6 ms   27.2% slower
  C++ (weak_ptr) ████████████████████████████████████     47.8 ms   225.9% slower
  rs             █████████████░░░░░░░░░░░░░░░░░░░░░░░     17.5 ms   19.4% slower
```

### Parallel sum — 50 000 000 elements

*vāṇī: `parallel for … reduce total with +` (3 extra keywords).
C/C++: OpenMP (if available), else serial.
Rust: std::thread manual split.*

```
  vani           █████████████████████████████████░░░    184.2 ms    baseline
  c              ████████████████████████████████████    202.8 ms   10.1% slower
  cpp            ██████████████████████████████████░░    193.1 ms    4.8% slower
  rs             ██████████████████████████░░░░░░░░░░    143.9 ms   21.9% faster
```

### HashMap — 500 000 insert + 500 000 lookup

*Tests open-addressing HashMap throughput.*

```
  vani           █████████████████░░░░░░░░░░░░░░░░░░░     38.1 ms    baseline
  c              █████████████░░░░░░░░░░░░░░░░░░░░░░░     28.7 ms   24.6% faster
  cpp            █████████████████████████░░░░░░░░░░░     54.1 ms   42.1% slower
  rs             ████████████████████████████████████     79.1 ms   107.7% slower
```

### Linked list — 1 000 000 nodes, index-based

*vāṇī/C index approach (no raw pointers): O(1) cache-friendly traversal.
C++/Rust use traditional pointer-linked nodes for comparison.*

```
  vani           ████████████████████████░░░░░░░░░░░░     13.3 ms    baseline
  c              ██████████████████████████░░░░░░░░░░     14.5 ms    9.2% slower
  cpp            ████████████████████████████████░░░░     17.6 ms   32.5% slower
  rs             ████████████████████████████████████     19.7 ms   48.6% slower
```

### Allocation stress — 500 000 struct alloc/free cycles

*Tests allocator throughput; vāṇī uses RAII affine drop.*

```
  vani           ████████████████████████████████████     16.9 ms    baseline
  c              ████████████████████████░░░░░░░░░░░░     11.4 ms   33.0% faster
  cpp            ███████████████████████████░░░░░░░░░     12.7 ms   24.9% faster
  rs             ████████████████████████████████░░░░     14.8 ms   12.4% faster
```

### Array statistics — mean + variance of 10 000 000 values

*vāṇī: two `parallel for … reduce` passes. C/C++/Rust: sequential passes. Tests loop throughput and parallelism.*

```
  vani           █████████████████████████░░░░░░░░░░░     36.1 ms    baseline
  c              ███████████████████████████████░░░░░     44.2 ms   22.3% slower
  cpp            ████████████████████████████████████     51.4 ms   42.3% slower
  rs             ███████████████████████████████░░░░░     44.5 ms   23.2% slower
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

