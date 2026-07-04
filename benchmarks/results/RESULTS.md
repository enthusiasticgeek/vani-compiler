# Benchmark Results — vāṇī vs Rust vs C vs C++

*Generated: 2026-07-03 23:47 — 3 timing run(s) per benchmark, median reported.*

## System
```
OS       : Windows 11 AMD64
Python   : 3.14.5
vanic    : C:\Users\upaas\vani-compiler\target\release/vanic.EXE
CC       : C:\msys64\mingw64\bin/gcc.EXE
CXX      : C:\msys64\mingw64\bin/g++.EXE
rustc    : C:\Users\upaas\.cargo\bin/rustc.EXE
```

## Summary

| Benchmark | vani         | c            | cpp          | rs           | cpp_idx      | cpp_weak     |
|--------------------|--------------|--------------|--------------|--------------|--------------|--------------|
| Fibonacci(42) — re |   842.7 ms   |   495.1 ms   |   482.9 ms   |   829.0 ms   | —            | —            |
| Sieve of Eratosthe |    48.9 ms   |    18.0 ms   |    20.5 ms   |    23.8 ms   | —            | —            |
| Matrix multiplicat |    18.2 ms   |    17.5 ms   |    23.1 ms   |    33.2 ms   | —            | —            |
| Sort 1 000 000 int |    97.2 ms   |   158.4 ms   |    97.4 ms   |    40.6 ms   | —            | —            |
| Graph BFS — index  |    33.6 ms   |    18.7 ms   | —            |    19.7 ms   |    20.9 ms   |    51.5 ms   |
| Parallel sum — 50  |   202.9 ms   |    96.8 ms   |   101.0 ms   |   144.9 ms   | —            | —            |
| HashMap — 500 000  |    54.8 ms   |    41.2 ms   |    60.8 ms   |    70.1 ms   | —            | —            |
| Linked list — 1 00 |    24.3 ms   |    15.0 ms   |    21.5 ms   |    22.0 ms   | —            | —            |
| Allocation stress  |    18.0 ms   |    12.8 ms   |    15.5 ms   |    17.9 ms   | —            | —            |
| Array statistics — |    46.0 ms   |    31.7 ms   |    38.3 ms   |    48.1 ms   | —            | —            |

## Per-benchmark charts

> Bars are proportional to wall-clock time — **shorter is faster**.

### Fibonacci(42) — recursive

*Classic recursive fib(42). Tests raw function-call throughput.*

```
  vani           ████████████████████████████████████    842.7 ms    baseline
  c              █████████████████████░░░░░░░░░░░░░░░    495.1 ms   41.2% faster
  cpp            █████████████████████░░░░░░░░░░░░░░░    482.9 ms   42.7% faster
  rs             ███████████████████████████████████░    829.0 ms    1.6% faster
```

### Sieve of Eratosthenes — primes ≤ 2 000 000

*Boolean sieve with Vec-set inner loop. Tests dense random-access array writes.*

```
  vani           ████████████████████████████████████     48.9 ms    baseline
  c              █████████████░░░░░░░░░░░░░░░░░░░░░░░     18.0 ms   63.2% faster
  cpp            ███████████████░░░░░░░░░░░░░░░░░░░░░     20.5 ms   58.1% faster
  rs             ██████████████████░░░░░░░░░░░░░░░░░░     23.8 ms   51.4% faster
```

### Matrix multiplication 256×256 (i64)

*Naïve triple-loop matmul. Tests arithmetic-dense nested loops.*

```
  vani           ████████████████████░░░░░░░░░░░░░░░░     18.2 ms    baseline
  c              ███████████████████░░░░░░░░░░░░░░░░░     17.5 ms    3.7% faster
  cpp            █████████████████████████░░░░░░░░░░░     23.1 ms   26.6% slower
  rs             ████████████████████████████████████     33.2 ms   82.5% slower
```

### Sort 1 000 000 integers

*vāṇī uses the built-in sort(mut ref xs); others use stdlib. Tests sort quality.*

```
  vani           ██████████████████████░░░░░░░░░░░░░░     97.2 ms    baseline
  c              ████████████████████████████████████    158.4 ms   63.0% slower
  cpp            ██████████████████████░░░░░░░░░░░░░░     97.4 ms    0.2% slower
  rs             █████████░░░░░░░░░░░░░░░░░░░░░░░░░░░     40.6 ms   58.3% faster
```

### Graph BFS — index handles vs. weak_ptr

*KEY BENCHMARK: BFS on a 1 000-node random graph, repeated 1 000×.
  graph.vani / graph_index.{c,cpp,rs} — int-index adjacency list, zero ref-counting.
  graph_weakptr.cpp                   — shared_ptr children + weak_ptr back-edges.*

```
  vani           ███████████████████████░░░░░░░░░░░░░     33.6 ms    baseline
  c              █████████████░░░░░░░░░░░░░░░░░░░░░░░     18.7 ms   44.3% faster
  C++ (index)    ███████████████░░░░░░░░░░░░░░░░░░░░░     20.9 ms   37.8% faster
  C++ (weak_ptr) ████████████████████████████████████     51.5 ms   53.4% slower
  rs             ██████████████░░░░░░░░░░░░░░░░░░░░░░     19.7 ms   41.2% faster
```

### Parallel sum — 50 000 000 elements

*vāṇī: `parallel for … reduce total with +` (3 extra keywords).
C/C++: OpenMP (if available), else serial.
Rust: std::thread manual split.*

```
  vani           ████████████████████████████████████    202.9 ms    baseline
  c              █████████████████░░░░░░░░░░░░░░░░░░░     96.8 ms   52.3% faster
  cpp            ██████████████████░░░░░░░░░░░░░░░░░░    101.0 ms   50.2% faster
  rs             ██████████████████████████░░░░░░░░░░    144.9 ms   28.6% faster
```

### HashMap — 500 000 insert + 500 000 lookup

*Tests open-addressing HashMap throughput.*

```
  vani           ████████████████████████████░░░░░░░░     54.8 ms    baseline
  c              █████████████████████░░░░░░░░░░░░░░░     41.2 ms   24.8% faster
  cpp            ███████████████████████████████░░░░░     60.8 ms   11.0% slower
  rs             ████████████████████████████████████     70.1 ms   27.9% slower
```

### Linked list — 1 000 000 nodes, index-based

*vāṇī/C index approach (no raw pointers): O(1) cache-friendly traversal.
C++/Rust use traditional pointer-linked nodes for comparison.*

```
  vani           ████████████████████████████████████     24.3 ms    baseline
  c              ██████████████████████░░░░░░░░░░░░░░     15.0 ms   38.0% faster
  cpp            ████████████████████████████████░░░░     21.5 ms   11.6% faster
  rs             █████████████████████████████████░░░     22.0 ms    9.6% faster
```

### Allocation stress — 500 000 struct alloc/free cycles

*Tests allocator throughput; vāṇī uses RAII affine drop.*

```
  vani           ████████████████████████████████████     18.0 ms    baseline
  c              ██████████████████████████░░░░░░░░░░     12.8 ms   28.9% faster
  cpp            ███████████████████████████████░░░░░     15.5 ms   13.8% faster
  rs             ████████████████████████████████████     17.9 ms    0.2% faster
```

### Array statistics — mean + variance of 10 000 000 values

*Two sequential passes; tests plain arithmetic loop throughput.*

```
  vani           ██████████████████████████████████░░     46.0 ms    baseline
  c              ████████████████████████░░░░░░░░░░░░     31.7 ms   31.1% faster
  cpp            █████████████████████████████░░░░░░░     38.3 ms   16.8% faster
  rs             ████████████████████████████████████     48.1 ms    4.4% slower
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

