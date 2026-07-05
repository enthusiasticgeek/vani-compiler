# Benchmark Results — vāṇī vs Rust vs C vs C++

*Generated: 2026-07-05 16:25 — 7 timing run(s) per benchmark, median reported.*
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
| Fibonacci(42) — re |   849.9 ms   |   491.6 ms   |   485.5 ms   |   855.1 ms   | —            | —            |
| Sieve of Eratosthe |    12.1 ms   |    12.5 ms   |    14.6 ms   |    14.1 ms   | —            | —            |
| Matrix multiplicat |    13.3 ms   |    13.2 ms   |    15.5 ms   |    26.4 ms   | —            | —            |
| Sort 1 000 000 int |    91.4 ms   |   165.8 ms   |   102.3 ms   |    36.8 ms   | —            | —            |
| Graph BFS — index  |    12.9 ms   |    13.9 ms   | —            |    16.3 ms   |    19.0 ms   |    45.9 ms   |
| Parallel sum — 50  |   157.5 ms   |   160.6 ms   |   161.8 ms   |   124.6 ms   | —            | —            |
| HashMap — 500 000  |    39.8 ms   |    32.1 ms   |    58.3 ms   |    69.9 ms   | —            | —            |
| Linked list — 1 00 |    11.9 ms   |    14.5 ms   |    14.4 ms   |    18.3 ms   | —            | —            |
| Allocation stress  |    16.2 ms   |    12.7 ms   |    13.6 ms   |    13.2 ms   | —            | —            |
| Array statistics — |    31.9 ms   |    40.6 ms   |    52.6 ms   |    40.8 ms   | —            | —            |

## Per-benchmark charts

> Bars are proportional to wall-clock time — **shorter is faster**.

### Fibonacci(42) — recursive

*Classic recursive fib(42). Tests raw function-call throughput.*

```
  vani           ████████████████████████████████████    849.9 ms    baseline
  c              █████████████████████░░░░░░░░░░░░░░░    491.6 ms   42.2% faster
  cpp            ████████████████████░░░░░░░░░░░░░░░░    485.5 ms   42.9% faster
  rs             ████████████████████████████████████    855.1 ms    0.6% slower
```

### Sieve of Eratosthenes — primes ≤ 2 000 000

*Boolean sieve with Vec-set inner loop. Tests dense random-access array writes.*

```
  vani           ██████████████████████████████░░░░░░     12.1 ms    baseline
  c              ███████████████████████████████░░░░░     12.5 ms    2.8% slower
  cpp            ████████████████████████████████████     14.6 ms   20.7% slower
  rs             ███████████████████████████████████░     14.1 ms   16.4% slower
```

### Matrix multiplication 256×256 (i64)

*Naïve triple-loop matmul. Tests arithmetic-dense nested loops.*

```
  vani           ██████████████████░░░░░░░░░░░░░░░░░░     13.3 ms    baseline
  c              ██████████████████░░░░░░░░░░░░░░░░░░     13.2 ms    1.2% faster
  cpp            █████████████████████░░░░░░░░░░░░░░░     15.5 ms   16.2% slower
  rs             ████████████████████████████████████     26.4 ms   98.4% slower
```

### Sort 1 000 000 integers

*vāṇī uses the built-in sort(mut ref xs); others use stdlib. Tests sort quality.*

```
  vani           ████████████████████░░░░░░░░░░░░░░░░     91.4 ms    baseline
  c              ████████████████████████████████████    165.8 ms   81.4% slower
  cpp            ██████████████████████░░░░░░░░░░░░░░    102.3 ms   12.0% slower
  rs             ████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░     36.8 ms   59.8% faster
```

### Graph BFS — index handles vs. weak_ptr

*KEY BENCHMARK: BFS on a 1 000-node random graph, repeated 1 000×.
  graph.vani / graph_index.{c,cpp,rs} — int-index adjacency list, zero ref-counting.
  graph_weakptr.cpp                   — shared_ptr children + weak_ptr back-edges.*

```
  vani           ██████████░░░░░░░░░░░░░░░░░░░░░░░░░░     12.9 ms    baseline
  c              ███████████░░░░░░░░░░░░░░░░░░░░░░░░░     13.9 ms    7.1% slower
  C++ (index)    ███████████████░░░░░░░░░░░░░░░░░░░░░     19.0 ms   47.0% slower
  C++ (weak_ptr) ████████████████████████████████████     45.9 ms   255.0% slower
  rs             █████████████░░░░░░░░░░░░░░░░░░░░░░░     16.3 ms   26.1% slower
```

### Parallel sum — 50 000 000 elements

*vāṇī: `parallel for … reduce total with +` (3 extra keywords).
C/C++: OpenMP (if available), else serial.
Rust: std::thread manual split.*

```
  vani           ███████████████████████████████████░    157.5 ms    baseline
  c              ████████████████████████████████████    160.6 ms    2.0% slower
  cpp            ████████████████████████████████████    161.8 ms    2.7% slower
  rs             ████████████████████████████░░░░░░░░    124.6 ms   20.9% faster
```

### HashMap — 500 000 insert + 500 000 lookup

*Tests open-addressing HashMap throughput.*

```
  vani           █████████████████████░░░░░░░░░░░░░░░     39.8 ms    baseline
  c              █████████████████░░░░░░░░░░░░░░░░░░░     32.1 ms   19.4% faster
  cpp            ██████████████████████████████░░░░░░     58.3 ms   46.4% slower
  rs             ████████████████████████████████████     69.9 ms   75.5% slower
```

### Linked list — 1 000 000 nodes, index-based

*vāṇī/C index approach (no raw pointers): O(1) cache-friendly traversal.
C++/Rust use traditional pointer-linked nodes for comparison.*

```
  vani           ███████████████████████░░░░░░░░░░░░░     11.9 ms    baseline
  c              █████████████████████████████░░░░░░░     14.5 ms   22.3% slower
  cpp            ████████████████████████████░░░░░░░░     14.4 ms   21.1% slower
  rs             ████████████████████████████████████     18.3 ms   53.7% slower
```

### Allocation stress — 500 000 struct alloc/free cycles

*Tests allocator throughput; vāṇī uses RAII affine drop.*

```
  vani           ████████████████████████████████████     16.2 ms    baseline
  c              ████████████████████████████░░░░░░░░     12.7 ms   21.9% faster
  cpp            ██████████████████████████████░░░░░░     13.6 ms   16.4% faster
  rs             █████████████████████████████░░░░░░░     13.2 ms   18.4% faster
```

### Array statistics — mean + variance of 10 000 000 values

*vāṇī: two `parallel for … reduce` passes. C/C++/Rust: sequential passes. Tests loop throughput and parallelism.*

```
  vani           ██████████████████████░░░░░░░░░░░░░░     31.9 ms    baseline
  c              ████████████████████████████░░░░░░░░     40.6 ms   27.1% slower
  cpp            ████████████████████████████████████     52.6 ms   64.6% slower
  rs             ████████████████████████████░░░░░░░░     40.8 ms   27.8% slower
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

