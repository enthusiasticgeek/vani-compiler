# Benchmark Results — vāṇī vs Rust vs C vs C++

*Generated: 2026-07-04 08:02 — 3 timing run(s) per benchmark, median reported.*

## System
```
OS       : Windows 11 AMD64
Python   : 3.14.5
vanic    : C:\Users\upaas\vani-compiler\target\release\vanic.EXE
CC       : C:\msys64\mingw64\bin\gcc.EXE
CXX      : C:\msys64\mingw64\bin\g++.EXE
rustc    : C:\Users\upaas\.cargo\bin\rustc.EXE
```

## Summary

| Benchmark | vani         | c            | cpp          | rs           | cpp_idx      | cpp_weak     |
|--------------------|--------------|--------------|--------------|--------------|--------------|--------------|
| Fibonacci(42) — re |   860.6 ms   |   537.7 ms   |   523.5 ms   |   892.9 ms   | —            | —            |
| Sieve of Eratosthe |    47.2 ms   |    20.4 ms   |    25.4 ms   |    18.3 ms   | —            | —            |
| Matrix multiplicat |    17.3 ms   |    26.6 ms   |    21.0 ms   |    31.1 ms   | —            | —            |
| Sort 1 000 000 int |   100.8 ms   |   175.0 ms   |    99.8 ms   |    44.3 ms   | —            | —            |
| Graph BFS — index  |    16.1 ms   |    14.1 ms   | —            |    21.5 ms   |    42.0 ms   |    51.9 ms   |
| Parallel sum — 50  |   251.1 ms   |    93.1 ms   |   115.3 ms   |   255.8 ms   | —            | —            |
| HashMap — 500 000  |    58.3 ms   |    54.8 ms   |    55.9 ms   |    79.6 ms   | —            | —            |
| Linked list — 1 00 |    23.1 ms   |    15.2 ms   |    17.3 ms   |    19.4 ms   | —            | —            |
| Allocation stress  |    15.1 ms   |    14.5 ms   |    17.1 ms   |    17.7 ms   | —            | —            |
| Array statistics — |    52.1 ms   |    33.8 ms   |    39.2 ms   |    44.7 ms   | —            | —            |

## Per-benchmark charts

> Bars are proportional to wall-clock time — **shorter is faster**.

### Fibonacci(42) — recursive

*Classic recursive fib(42). Tests raw function-call throughput.*

```
  vani           ███████████████████████████████████░    860.6 ms    baseline
  c              ██████████████████████░░░░░░░░░░░░░░    537.7 ms   37.5% faster
  cpp            █████████████████████░░░░░░░░░░░░░░░    523.5 ms   39.2% faster
  rs             ████████████████████████████████████    892.9 ms    3.8% slower
```

### Sieve of Eratosthenes — primes ≤ 2 000 000

*Boolean sieve with Vec-set inner loop. Tests dense random-access array writes.*

```
  vani           ████████████████████████████████████     47.2 ms    baseline
  c              ████████████████░░░░░░░░░░░░░░░░░░░░     20.4 ms   56.8% faster
  cpp            ███████████████████░░░░░░░░░░░░░░░░░     25.4 ms   46.2% faster
  rs             ██████████████░░░░░░░░░░░░░░░░░░░░░░     18.3 ms   61.1% faster
```

### Matrix multiplication 256×256 (i64)

*Naïve triple-loop matmul. Tests arithmetic-dense nested loops.*

```
  vani           ████████████████████░░░░░░░░░░░░░░░░     17.3 ms    baseline
  c              ███████████████████████████████░░░░░     26.6 ms   53.7% slower
  cpp            ████████████████████████░░░░░░░░░░░░     21.0 ms   21.1% slower
  rs             ████████████████████████████████████     31.1 ms   79.3% slower
```

### Sort 1 000 000 integers

*vāṇī uses the built-in sort(mut ref xs); others use stdlib. Tests sort quality.*

```
  vani           █████████████████████░░░░░░░░░░░░░░░    100.8 ms    baseline
  c              ████████████████████████████████████    175.0 ms   73.6% slower
  cpp            █████████████████████░░░░░░░░░░░░░░░     99.8 ms    1.0% faster
  rs             █████████░░░░░░░░░░░░░░░░░░░░░░░░░░░     44.3 ms   56.0% faster
```

### Graph BFS — index handles vs. weak_ptr

*KEY BENCHMARK: BFS on a 1 000-node random graph, repeated 1 000×.
  graph.vani / graph_index.{c,cpp,rs} — int-index adjacency list, zero ref-counting.
  graph_weakptr.cpp                   — shared_ptr children + weak_ptr back-edges.*

```
  vani           ███████████░░░░░░░░░░░░░░░░░░░░░░░░░     16.1 ms    baseline
  c              ██████████░░░░░░░░░░░░░░░░░░░░░░░░░░     14.1 ms   12.3% faster
  C++ (index)    █████████████████████████████░░░░░░░     42.0 ms   160.9% slower
  C++ (weak_ptr) ████████████████████████████████████     51.9 ms   221.9% slower
  rs             ███████████████░░░░░░░░░░░░░░░░░░░░░     21.5 ms   33.7% slower
```

### Parallel sum — 50 000 000 elements

*vāṇī: `parallel for … reduce total with +` (3 extra keywords).
C/C++: OpenMP (if available), else serial.
Rust: std::thread manual split.*

```
  vani           ███████████████████████████████████░    251.1 ms    baseline
  c              █████████████░░░░░░░░░░░░░░░░░░░░░░░     93.1 ms   62.9% faster
  cpp            ████████████████░░░░░░░░░░░░░░░░░░░░    115.3 ms   54.1% faster
  rs             ████████████████████████████████████    255.8 ms    1.9% slower
```

### HashMap — 500 000 insert + 500 000 lookup

*Tests open-addressing HashMap throughput.*

```
  vani           ██████████████████████████░░░░░░░░░░     58.3 ms    baseline
  c              █████████████████████████░░░░░░░░░░░     54.8 ms    6.1% faster
  cpp            █████████████████████████░░░░░░░░░░░     55.9 ms    4.1% faster
  rs             ████████████████████████████████████     79.6 ms   36.5% slower
```

### Linked list — 1 000 000 nodes, index-based

*vāṇī/C index approach (no raw pointers): O(1) cache-friendly traversal.
C++/Rust use traditional pointer-linked nodes for comparison.*

```
  vani           ████████████████████████████████████     23.1 ms    baseline
  c              ████████████████████████░░░░░░░░░░░░     15.2 ms   34.3% faster
  cpp            ███████████████████████████░░░░░░░░░     17.3 ms   25.3% faster
  rs             ██████████████████████████████░░░░░░     19.4 ms   16.0% faster
```

### Allocation stress — 500 000 struct alloc/free cycles

*Tests allocator throughput; vāṇī uses RAII affine drop.*

```
  vani           ███████████████████████████████░░░░░     15.1 ms    baseline
  c              ██████████████████████████████░░░░░░     14.5 ms    4.0% faster
  cpp            ███████████████████████████████████░     17.1 ms   13.6% slower
  rs             ████████████████████████████████████     17.7 ms   17.1% slower
```

### Array statistics — mean + variance of 10 000 000 values

*Two sequential passes; tests plain arithmetic loop throughput.*

```
  vani           ████████████████████████████████████     52.1 ms    baseline
  c              ███████████████████████░░░░░░░░░░░░░     33.8 ms   35.2% faster
  cpp            ███████████████████████████░░░░░░░░░     39.2 ms   24.7% faster
  rs             ███████████████████████████████░░░░░     44.7 ms   14.2% faster
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

