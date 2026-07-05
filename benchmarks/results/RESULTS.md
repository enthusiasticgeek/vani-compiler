# Benchmark Results — vāṇī vs Rust vs C vs C++

*Generated: 2026-07-04 21:00 — 7 timing run(s) per benchmark, median reported.*
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
| Fibonacci(42) — re |   1.310  s   |   776.4 ms   |   747.4 ms   |   1.199  s   | —            | —            |
| Sieve of Eratosthe |    15.3 ms   |    14.8 ms   |    20.3 ms   |    16.0 ms   | —            | —            |
| Matrix multiplicat |    17.0 ms   |    14.5 ms   |    16.0 ms   |    31.9 ms   | —            | —            |
| Sort 1 000 000 int |   123.0 ms   |   191.3 ms   |   114.4 ms   |    48.1 ms   | —            | —            |
| Graph BFS — index  |    13.9 ms   |    17.9 ms   | —            |    20.4 ms   |    22.3 ms   |    65.0 ms   |
| Parallel sum — 50  |   218.8 ms   |    81.1 ms   |    94.0 ms   |   151.6 ms   | —            | —            |
| HashMap — 500 000  |    50.6 ms   |    41.1 ms   |    67.1 ms   |    99.5 ms   | —            | —            |
| Linked list — 1 00 |    15.5 ms   |    14.8 ms   |    32.8 ms   |    33.8 ms   | —            | —            |
| Allocation stress  |    32.9 ms   |    27.9 ms   |    16.0 ms   |    25.9 ms   | —            | —            |
| Array statistics — |    59.1 ms   |    40.1 ms   |    43.2 ms   |    89.8 ms   | —            | —            |

## Per-benchmark charts

> Bars are proportional to wall-clock time — **shorter is faster**.

### Fibonacci(42) — recursive

*Classic recursive fib(42). Tests raw function-call throughput.*

```
  vani           ████████████████████████████████████    1.310  s    baseline
  c              █████████████████████░░░░░░░░░░░░░░░    776.4 ms   40.7% faster
  cpp            █████████████████████░░░░░░░░░░░░░░░    747.4 ms   42.9% faster
  rs             █████████████████████████████████░░░    1.199  s    8.4% faster
```

### Sieve of Eratosthenes — primes ≤ 2 000 000

*Boolean sieve with Vec-set inner loop. Tests dense random-access array writes.*

```
  vani           ███████████████████████████░░░░░░░░░     15.3 ms    baseline
  c              ██████████████████████████░░░░░░░░░░     14.8 ms    3.3% faster
  cpp            ████████████████████████████████████     20.3 ms   32.6% slower
  rs             ████████████████████████████░░░░░░░░     16.0 ms    4.4% slower
```

### Matrix multiplication 256×256 (i64)

*Naïve triple-loop matmul. Tests arithmetic-dense nested loops.*

```
  vani           ███████████████████░░░░░░░░░░░░░░░░░     17.0 ms    baseline
  c              ████████████████░░░░░░░░░░░░░░░░░░░░     14.5 ms   14.9% faster
  cpp            ██████████████████░░░░░░░░░░░░░░░░░░     16.0 ms    5.8% faster
  rs             ████████████████████████████████████     31.9 ms   87.7% slower
```

### Sort 1 000 000 integers

*vāṇī uses the built-in sort(mut ref xs); others use stdlib. Tests sort quality.*

```
  vani           ███████████████████████░░░░░░░░░░░░░    123.0 ms    baseline
  c              ████████████████████████████████████    191.3 ms   55.5% slower
  cpp            ██████████████████████░░░░░░░░░░░░░░    114.4 ms    7.0% faster
  rs             █████████░░░░░░░░░░░░░░░░░░░░░░░░░░░     48.1 ms   60.9% faster
```

### Graph BFS — index handles vs. weak_ptr

*KEY BENCHMARK: BFS on a 1 000-node random graph, repeated 1 000×.
  graph.vani / graph_index.{c,cpp,rs} — int-index adjacency list, zero ref-counting.
  graph_weakptr.cpp                   — shared_ptr children + weak_ptr back-edges.*

```
  vani           ████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░     13.9 ms    baseline
  c              ██████████░░░░░░░░░░░░░░░░░░░░░░░░░░     17.9 ms   29.2% slower
  C++ (index)    ████████████░░░░░░░░░░░░░░░░░░░░░░░░     22.3 ms   60.4% slower
  C++ (weak_ptr) ████████████████████████████████████     65.0 ms   368.6% slower
  rs             ███████████░░░░░░░░░░░░░░░░░░░░░░░░░     20.4 ms   47.1% slower
```

### Parallel sum — 50 000 000 elements

*vāṇī: `parallel for … reduce total with +` (3 extra keywords).
C/C++: OpenMP (if available), else serial.
Rust: std::thread manual split.*

```
  vani           ████████████████████████████████████    218.8 ms    baseline
  c              █████████████░░░░░░░░░░░░░░░░░░░░░░░     81.1 ms   62.9% faster
  cpp            ███████████████░░░░░░░░░░░░░░░░░░░░░     94.0 ms   57.0% faster
  rs             █████████████████████████░░░░░░░░░░░    151.6 ms   30.7% faster
```

### HashMap — 500 000 insert + 500 000 lookup

*Tests open-addressing HashMap throughput.*

```
  vani           ██████████████████░░░░░░░░░░░░░░░░░░     50.6 ms    baseline
  c              ███████████████░░░░░░░░░░░░░░░░░░░░░     41.1 ms   18.7% faster
  cpp            ████████████████████████░░░░░░░░░░░░     67.1 ms   32.7% slower
  rs             ████████████████████████████████████     99.5 ms   96.8% slower
```

### Linked list — 1 000 000 nodes, index-based

*vāṇī/C index approach (no raw pointers): O(1) cache-friendly traversal.
C++/Rust use traditional pointer-linked nodes for comparison.*

```
  vani           █████████████████░░░░░░░░░░░░░░░░░░░     15.5 ms    baseline
  c              ████████████████░░░░░░░░░░░░░░░░░░░░     14.8 ms    4.9% faster
  cpp            ███████████████████████████████████░     32.8 ms   111.1% slower
  rs             ████████████████████████████████████     33.8 ms   117.7% slower
```

### Allocation stress — 500 000 struct alloc/free cycles

*Tests allocator throughput; vāṇī uses RAII affine drop.*

```
  vani           ████████████████████████████████████     32.9 ms    baseline
  c              ███████████████████████████████░░░░░     27.9 ms   15.2% faster
  cpp            ██████████████████░░░░░░░░░░░░░░░░░░     16.0 ms   51.2% faster
  rs             ████████████████████████████░░░░░░░░     25.9 ms   21.1% faster
```

### Array statistics — mean + variance of 10 000 000 values

*Two sequential passes; tests plain arithmetic loop throughput.*

```
  vani           ████████████████████████░░░░░░░░░░░░     59.1 ms    baseline
  c              ████████████████░░░░░░░░░░░░░░░░░░░░     40.1 ms   32.1% faster
  cpp            █████████████████░░░░░░░░░░░░░░░░░░░     43.2 ms   26.8% faster
  rs             ████████████████████████████████████     89.8 ms   52.0% slower
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

