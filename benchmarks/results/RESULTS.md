# Benchmark Results — vāṇī vs Rust vs C vs C++

*Generated: 2026-07-03 08:17 — 3 timing run(s) per benchmark, median reported.*

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
| Fibonacci(42) — re |   1.028  s   |   586.3 ms   |   611.9 ms   |   1.016  s   | —            | —            |
| Sieve of Eratosthe |    66.8 ms   |    16.0 ms   |    18.2 ms   |    16.1 ms   | —            | —            |
| Matrix multiplicat |    21.6 ms   |    26.2 ms   |    18.7 ms   |    31.0 ms   | —            | —            |
| Sort 1 000 000 int |   220.1 ms   |   179.1 ms   |    96.4 ms   |    45.8 ms   | —            | —            |
| Graph BFS — index  |    56.1 ms   |    11.4 ms   | —            |    15.7 ms   |    23.1 ms   |    53.2 ms   |
| Parallel sum — 50  |   556.1 ms   |   116.8 ms   |   114.3 ms   |   204.7 ms   | —            | —            |
| HashMap — 500 000  |    65.2 ms   |    58.4 ms   |    63.6 ms   |    86.7 ms   | —            | —            |
| Linked list — 1 00 |    18.7 ms   |    13.4 ms   |    16.8 ms   |    21.0 ms   | —            | —            |
| Allocation stress  |    17.1 ms   |    11.7 ms   |    14.8 ms   |    15.9 ms   | —            | —            |
| Array statistics — |   106.2 ms   |    36.2 ms   |    40.4 ms   |    68.0 ms   | —            | —            |

## Per-benchmark charts

> Bars are proportional to wall-clock time — **shorter is faster**.

### Fibonacci(42) — recursive

*Classic recursive fib(42). Tests raw function-call throughput.*

```
  vani           ████████████████████████████████████    1.028  s    baseline
  c              █████████████████████░░░░░░░░░░░░░░░    586.3 ms   43.0% faster
  cpp            █████████████████████░░░░░░░░░░░░░░░    611.9 ms   40.5% faster
  rs             ████████████████████████████████████    1.016  s    1.2% faster
```

### Sieve of Eratosthenes — primes ≤ 2 000 000

*Boolean sieve with Vec-set inner loop. Tests dense random-access array writes.*

```
  vani           ████████████████████████████████████     66.8 ms    baseline
  c              █████████░░░░░░░░░░░░░░░░░░░░░░░░░░░     16.0 ms   76.0% faster
  cpp            ██████████░░░░░░░░░░░░░░░░░░░░░░░░░░     18.2 ms   72.7% faster
  rs             █████████░░░░░░░░░░░░░░░░░░░░░░░░░░░     16.1 ms   75.8% faster
```

### Matrix multiplication 256×256 (i64)

*Naïve triple-loop matmul. Tests arithmetic-dense nested loops.*

```
  vani           █████████████████████████░░░░░░░░░░░     21.6 ms    baseline
  c              ██████████████████████████████░░░░░░     26.2 ms   21.5% slower
  cpp            ██████████████████████░░░░░░░░░░░░░░     18.7 ms   13.5% faster
  rs             ████████████████████████████████████     31.0 ms   43.7% slower
```

### Sort 1 000 000 integers

*vāṇī uses the built-in sort(mut ref xs); others use stdlib. Tests sort quality.*

```
  vani           ████████████████████████████████████    220.1 ms    baseline
  c              █████████████████████████████░░░░░░░    179.1 ms   18.6% faster
  cpp            ████████████████░░░░░░░░░░░░░░░░░░░░     96.4 ms   56.2% faster
  rs             ███████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░     45.8 ms   79.2% faster
```

### Graph BFS — index handles vs. weak_ptr

*KEY BENCHMARK: BFS on a 1 000-node random graph, repeated 1 000×.
  graph.vani / graph_index.{c,cpp,rs} — int-index adjacency list, zero ref-counting.
  graph_weakptr.cpp                   — shared_ptr children + weak_ptr back-edges.*

```
  vani           ████████████████████████████████████     56.1 ms    baseline
  c              ███████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░     11.4 ms   79.6% faster
  C++ (index)    ███████████████░░░░░░░░░░░░░░░░░░░░░     23.1 ms   58.8% faster
  C++ (weak_ptr) ██████████████████████████████████░░     53.2 ms    5.2% faster
  rs             ██████████░░░░░░░░░░░░░░░░░░░░░░░░░░     15.7 ms   72.0% faster
```

### Parallel sum — 50 000 000 elements

*vāṇī: `parallel for … reduce total with +` (3 extra keywords).
C/C++: OpenMP (if available), else serial.
Rust: std::thread manual split.*

```
  vani           ████████████████████████████████████    556.1 ms    baseline
  c              ████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░    116.8 ms   79.0% faster
  cpp            ███████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░    114.3 ms   79.4% faster
  rs             █████████████░░░░░░░░░░░░░░░░░░░░░░░    204.7 ms   63.2% faster
```

### HashMap — 500 000 insert + 500 000 lookup

*Tests open-addressing HashMap throughput.*

```
  vani           ███████████████████████████░░░░░░░░░     65.2 ms    baseline
  c              ████████████████████████░░░░░░░░░░░░     58.4 ms   10.4% faster
  cpp            ██████████████████████████░░░░░░░░░░     63.6 ms    2.4% faster
  rs             ████████████████████████████████████     86.7 ms   33.0% slower
```

### Linked list — 1 000 000 nodes, index-based

*vāṇī/C index approach (no raw pointers): O(1) cache-friendly traversal.
C++/Rust use traditional pointer-linked nodes for comparison.*

```
  vani           ████████████████████████████████░░░░     18.7 ms    baseline
  c              ███████████████████████░░░░░░░░░░░░░     13.4 ms   28.0% faster
  cpp            █████████████████████████████░░░░░░░     16.8 ms   10.1% faster
  rs             ████████████████████████████████████     21.0 ms   12.2% slower
```

### Allocation stress — 500 000 struct alloc/free cycles

*Tests allocator throughput; vāṇī uses RAII affine drop.*

```
  vani           ████████████████████████████████████     17.1 ms    baseline
  c              █████████████████████████░░░░░░░░░░░     11.7 ms   31.8% faster
  cpp            ███████████████████████████████░░░░░     14.8 ms   13.8% faster
  rs             █████████████████████████████████░░░     15.9 ms    7.3% faster
```

### Array statistics — mean + variance of 10 000 000 values

*Two sequential passes; tests plain arithmetic loop throughput.*

```
  vani           ████████████████████████████████████    106.2 ms    baseline
  c              ████████████░░░░░░░░░░░░░░░░░░░░░░░░     36.2 ms   65.9% faster
  cpp            ██████████████░░░░░░░░░░░░░░░░░░░░░░     40.4 ms   62.0% faster
  rs             ███████████████████████░░░░░░░░░░░░░     68.0 ms   36.0% faster
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

