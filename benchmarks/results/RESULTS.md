# Benchmark Results — vāṇī vs Rust vs C vs C++

*Generated: 2026-07-05 11:57 — 7 timing run(s) per benchmark, median reported.*
*C/C++ flags: `-O3 -march=native`. Rust flags: `-C opt-level=3 -C target-cpu=native`.*
*vāṇī uses LLVM backend with `opt -O2 --mcpu=native` + `llc -O2 -mcpu=native`.*

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
| Fibonacci(42) — re |   866.9 ms   |   505.8 ms   |   479.1 ms   |   902.9 ms   | —            | —            |
| Sieve of Eratosthe |    14.6 ms   |    12.0 ms   |    17.4 ms   |    14.5 ms   | —            | —            |
| Matrix multiplicat |    13.2 ms   |    13.6 ms   |    19.6 ms   |    30.4 ms   | —            | —            |
| Sort 1 000 000 int |    98.8 ms   |   177.0 ms   |   100.8 ms   |    39.1 ms   | —            | —            |
| Graph BFS — index  |    11.7 ms   |    12.7 ms   | —            |    17.1 ms   |    19.1 ms   |    43.9 ms   |
| Parallel sum — 50  |   168.5 ms   |   203.1 ms   |   197.6 ms   |   121.2 ms   | —            | —            |
| HashMap — 500 000  |    38.2 ms   |    34.2 ms   |    55.8 ms   |    69.7 ms   | —            | —            |
| Linked list — 1 00 |    13.5 ms   |    15.7 ms   |    17.9 ms   |    18.9 ms   | —            | —            |
| Allocation stress  |    15.4 ms   |    14.9 ms   |    17.1 ms   |    16.2 ms   | —            | —            |
| Array statistics — |    51.5 ms   |    53.9 ms   |    57.7 ms   |    51.2 ms   | —            | —            |

## Per-benchmark charts

> Bars are proportional to wall-clock time — **shorter is faster**.

### Fibonacci(42) — recursive

*Classic recursive fib(42). Tests raw function-call throughput.*

```
  vani           ███████████████████████████████████░    866.9 ms    baseline
  c              ████████████████████░░░░░░░░░░░░░░░░    505.8 ms   41.7% faster
  cpp            ███████████████████░░░░░░░░░░░░░░░░░    479.1 ms   44.7% faster
  rs             ████████████████████████████████████    902.9 ms    4.2% slower
```

### Sieve of Eratosthenes — primes ≤ 2 000 000

*Boolean sieve with Vec-set inner loop. Tests dense random-access array writes.*

```
  vani           ██████████████████████████████░░░░░░     14.6 ms    baseline
  c              █████████████████████████░░░░░░░░░░░     12.0 ms   17.6% faster
  cpp            ████████████████████████████████████     17.4 ms   19.1% slower
  rs             ██████████████████████████████░░░░░░     14.5 ms    0.7% faster
```

### Matrix multiplication 256×256 (i64)

*Naïve triple-loop matmul. Tests arithmetic-dense nested loops.*

```
  vani           ████████████████░░░░░░░░░░░░░░░░░░░░     13.2 ms    baseline
  c              ████████████████░░░░░░░░░░░░░░░░░░░░     13.6 ms    2.7% slower
  cpp            ███████████████████████░░░░░░░░░░░░░     19.6 ms   48.3% slower
  rs             ████████████████████████████████████     30.4 ms   129.6% slower
```

### Sort 1 000 000 integers

*vāṇī uses the built-in sort(mut ref xs); others use stdlib. Tests sort quality.*

```
  vani           ████████████████████░░░░░░░░░░░░░░░░     98.8 ms    baseline
  c              ████████████████████████████████████    177.0 ms   79.1% slower
  cpp            ████████████████████░░░░░░░░░░░░░░░░    100.8 ms    2.0% slower
  rs             ████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░     39.1 ms   60.4% faster
```

### Graph BFS — index handles vs. weak_ptr

*KEY BENCHMARK: BFS on a 1 000-node random graph, repeated 1 000×.
  graph.vani / graph_index.{c,cpp,rs} — int-index adjacency list, zero ref-counting.
  graph_weakptr.cpp                   — shared_ptr children + weak_ptr back-edges.*

```
  vani           ██████████░░░░░░░░░░░░░░░░░░░░░░░░░░     11.7 ms    baseline
  c              ██████████░░░░░░░░░░░░░░░░░░░░░░░░░░     12.7 ms    9.0% slower
  C++ (index)    ████████████████░░░░░░░░░░░░░░░░░░░░     19.1 ms   64.1% slower
  C++ (weak_ptr) ████████████████████████████████████     43.9 ms   276.3% slower
  rs             ██████████████░░░░░░░░░░░░░░░░░░░░░░     17.1 ms   46.3% slower
```

### Parallel sum — 50 000 000 elements

*vāṇī: `parallel for … reduce total with +` (3 extra keywords).
C/C++: OpenMP (if available), else serial.
Rust: std::thread manual split.*

```
  vani           ██████████████████████████████░░░░░░    168.5 ms    baseline
  c              ████████████████████████████████████    203.1 ms   20.5% slower
  cpp            ███████████████████████████████████░    197.6 ms   17.3% slower
  rs             █████████████████████░░░░░░░░░░░░░░░    121.2 ms   28.1% faster
```

### HashMap — 500 000 insert + 500 000 lookup

*Tests open-addressing HashMap throughput.*

```
  vani           ████████████████████░░░░░░░░░░░░░░░░     38.2 ms    baseline
  c              ██████████████████░░░░░░░░░░░░░░░░░░     34.2 ms   10.4% faster
  cpp            █████████████████████████████░░░░░░░     55.8 ms   45.9% slower
  rs             ████████████████████████████████████     69.7 ms   82.4% slower
```

### Linked list — 1 000 000 nodes, index-based

*vāṇī/C index approach (no raw pointers): O(1) cache-friendly traversal.
C++/Rust use traditional pointer-linked nodes for comparison.*

```
  vani           ██████████████████████████░░░░░░░░░░     13.5 ms    baseline
  c              ██████████████████████████████░░░░░░     15.7 ms   16.9% slower
  cpp            ██████████████████████████████████░░     17.9 ms   33.1% slower
  rs             ████████████████████████████████████     18.9 ms   40.2% slower
```

### Allocation stress — 500 000 struct alloc/free cycles

*Tests allocator throughput; vāṇī uses RAII affine drop.*

```
  vani           █████████████████████████████████░░░     15.4 ms    baseline
  c              ████████████████████████████████░░░░     14.9 ms    3.1% faster
  cpp            ████████████████████████████████████     17.1 ms   10.6% slower
  rs             ██████████████████████████████████░░     16.2 ms    5.2% slower
```

### Array statistics — mean + variance of 10 000 000 values

*Two sequential passes; tests plain arithmetic loop throughput.*

```
  vani           ████████████████████████████████░░░░     51.5 ms    baseline
  c              ██████████████████████████████████░░     53.9 ms    4.7% slower
  cpp            ████████████████████████████████████     57.7 ms   12.0% slower
  rs             ████████████████████████████████░░░░     51.2 ms    0.5% faster
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

