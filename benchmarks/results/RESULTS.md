# Benchmark Results — vāṇī vs Rust vs C vs C++

*Generated: 2026-07-03 10:08 — 3 timing run(s) per benchmark, median reported.*

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
| Fibonacci(42) — re |   875.9 ms   |   518.6 ms   |   523.7 ms   |   832.8 ms   | —            | —            |
| Sieve of Eratosthe |    51.4 ms   |    12.8 ms   |    27.9 ms   |    14.0 ms   | —            | —            |
| Matrix multiplicat |    24.1 ms   |    11.8 ms   |    18.2 ms   |    29.6 ms   | —            | —            |
| Sort 1 000 000 int |   195.6 ms   |   162.3 ms   |    96.5 ms   |    38.6 ms   | —            | —            |
| Graph BFS — index  |    43.5 ms   |    12.4 ms   | —            |    15.6 ms   |    26.2 ms   |    51.8 ms   |
| Parallel sum — 50  |   474.3 ms   |   106.6 ms   |   130.5 ms   |   195.3 ms   | —            | —            |
| HashMap — 500 000  |    50.8 ms   |    44.9 ms   |    55.8 ms   |    77.7 ms   | —            | —            |
| Linked list — 1 00 |    19.0 ms   |    12.5 ms   |    16.9 ms   |    17.5 ms   | —            | —            |
| Allocation stress  |    16.0 ms   |    20.3 ms   |    19.4 ms   |    14.9 ms   | —            | —            |
| Array statistics — |    82.0 ms   |    26.8 ms   |    30.5 ms   |    49.8 ms   | —            | —            |

## Per-benchmark charts

> Bars are proportional to wall-clock time — **shorter is faster**.

### Fibonacci(42) — recursive

*Classic recursive fib(42). Tests raw function-call throughput.*

```
  vani           ████████████████████████████████████    875.9 ms    baseline
  c              █████████████████████░░░░░░░░░░░░░░░    518.6 ms   40.8% faster
  cpp            ██████████████████████░░░░░░░░░░░░░░    523.7 ms   40.2% faster
  rs             ██████████████████████████████████░░    832.8 ms    4.9% faster
```

### Sieve of Eratosthenes — primes ≤ 2 000 000

*Boolean sieve with Vec-set inner loop. Tests dense random-access array writes.*

```
  vani           ████████████████████████████████████     51.4 ms    baseline
  c              █████████░░░░░░░░░░░░░░░░░░░░░░░░░░░     12.8 ms   75.0% faster
  cpp            ████████████████████░░░░░░░░░░░░░░░░     27.9 ms   45.7% faster
  rs             ██████████░░░░░░░░░░░░░░░░░░░░░░░░░░     14.0 ms   72.7% faster
```

### Matrix multiplication 256×256 (i64)

*Naïve triple-loop matmul. Tests arithmetic-dense nested loops.*

```
  vani           █████████████████████████████░░░░░░░     24.1 ms    baseline
  c              ██████████████░░░░░░░░░░░░░░░░░░░░░░     11.8 ms   51.3% faster
  cpp            ██████████████████████░░░░░░░░░░░░░░     18.2 ms   24.7% faster
  rs             ████████████████████████████████████     29.6 ms   22.6% slower
```

### Sort 1 000 000 integers

*vāṇī uses the built-in sort(mut ref xs); others use stdlib. Tests sort quality.*

```
  vani           ████████████████████████████████████    195.6 ms    baseline
  c              ██████████████████████████████░░░░░░    162.3 ms   17.0% faster
  cpp            ██████████████████░░░░░░░░░░░░░░░░░░     96.5 ms   50.7% faster
  rs             ███████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░     38.6 ms   80.3% faster
```

### Graph BFS — index handles vs. weak_ptr

*KEY BENCHMARK: BFS on a 1 000-node random graph, repeated 1 000×.
  graph.vani / graph_index.{c,cpp,rs} — int-index adjacency list, zero ref-counting.
  graph_weakptr.cpp                   — shared_ptr children + weak_ptr back-edges.*

```
  vani           ██████████████████████████████░░░░░░     43.5 ms    baseline
  c              █████████░░░░░░░░░░░░░░░░░░░░░░░░░░░     12.4 ms   71.4% faster
  C++ (index)    ██████████████████░░░░░░░░░░░░░░░░░░     26.2 ms   39.7% faster
  C++ (weak_ptr) ████████████████████████████████████     51.8 ms   19.1% slower
  rs             ███████████░░░░░░░░░░░░░░░░░░░░░░░░░     15.6 ms   64.1% faster
```

### Parallel sum — 50 000 000 elements

*vāṇī: `parallel for … reduce total with +` (3 extra keywords).
C/C++: OpenMP (if available), else serial.
Rust: std::thread manual split.*

```
  vani           ████████████████████████████████████    474.3 ms    baseline
  c              ████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░    106.6 ms   77.5% faster
  cpp            ██████████░░░░░░░░░░░░░░░░░░░░░░░░░░    130.5 ms   72.5% faster
  rs             ███████████████░░░░░░░░░░░░░░░░░░░░░    195.3 ms   58.8% faster
```

### HashMap — 500 000 insert + 500 000 lookup

*Tests open-addressing HashMap throughput.*

```
  vani           ████████████████████████░░░░░░░░░░░░     50.8 ms    baseline
  c              █████████████████████░░░░░░░░░░░░░░░     44.9 ms   11.7% faster
  cpp            ██████████████████████████░░░░░░░░░░     55.8 ms    9.9% slower
  rs             ████████████████████████████████████     77.7 ms   52.9% slower
```

### Linked list — 1 000 000 nodes, index-based

*vāṇī/C index approach (no raw pointers): O(1) cache-friendly traversal.
C++/Rust use traditional pointer-linked nodes for comparison.*

```
  vani           ████████████████████████████████████     19.0 ms    baseline
  c              ████████████████████████░░░░░░░░░░░░     12.5 ms   34.3% faster
  cpp            ████████████████████████████████░░░░     16.9 ms   10.8% faster
  rs             █████████████████████████████████░░░     17.5 ms    7.7% faster
```

### Allocation stress — 500 000 struct alloc/free cycles

*Tests allocator throughput; vāṇī uses RAII affine drop.*

```
  vani           ████████████████████████████░░░░░░░░     16.0 ms    baseline
  c              ████████████████████████████████████     20.3 ms   26.8% slower
  cpp            ██████████████████████████████████░░     19.4 ms   20.8% slower
  rs             ██████████████████████████░░░░░░░░░░     14.9 ms    7.0% faster
```

### Array statistics — mean + variance of 10 000 000 values

*Two sequential passes; tests plain arithmetic loop throughput.*

```
  vani           ████████████████████████████████████     82.0 ms    baseline
  c              ████████████░░░░░░░░░░░░░░░░░░░░░░░░     26.8 ms   67.3% faster
  cpp            █████████████░░░░░░░░░░░░░░░░░░░░░░░     30.5 ms   62.9% faster
  rs             ██████████████████████░░░░░░░░░░░░░░     49.8 ms   39.3% faster
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

