# Benchmark Results — vāṇī vs Rust vs C vs C++

*Generated: 2026-07-06 08:15 — 3 timing run(s) per benchmark, median reported.*
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
| Fibonacci(42) — re |   842.5 ms   |   495.3 ms   |   511.2 ms   |   877.8 ms   | —            | —            |
| Sieve of Eratosthe |    14.8 ms   |    11.3 ms   |    15.3 ms   |    14.3 ms   | —            | —            |
| Matrix multiplicat |    15.8 ms   |    20.3 ms   |    19.6 ms   |    31.5 ms   | —            | —            |
| Sort 1 000 000 int |   129.3 ms   |   176.7 ms   |   108.1 ms   |    35.5 ms   | —            | —            |
| Graph BFS — index  |    15.0 ms   |    11.0 ms   | —            |    18.3 ms   |    16.7 ms   |    49.1 ms   |
| Parallel sum — 50  |   224.1 ms   |   164.1 ms   |   204.2 ms   |   138.9 ms   | —            | —            |
| HashMap — 500 000  |    51.0 ms   |    43.6 ms   |    53.3 ms   |    89.0 ms   | —            | —            |
| Linked list — 1 00 |    14.2 ms   |    15.5 ms   |    16.8 ms   |    16.5 ms   | —            | —            |
| Allocation stress  |    19.7 ms   |    15.3 ms   |    16.1 ms   |    13.4 ms   | —            | —            |
| Array statistics — |    36.1 ms   |    43.0 ms   |    45.3 ms   |    45.6 ms   | —            | —            |

## Per-benchmark charts

> Bars are proportional to wall-clock time — **shorter is faster**.

### Fibonacci(42) — recursive

*Classic recursive fib(42). Tests raw function-call throughput.*

```
  vani           ███████████████████████████████████░    842.5 ms    baseline
  c              ████████████████████░░░░░░░░░░░░░░░░    495.3 ms   41.2% faster
  cpp            █████████████████████░░░░░░░░░░░░░░░    511.2 ms   39.3% faster
  rs             ████████████████████████████████████    877.8 ms    4.2% slower
```

### Sieve of Eratosthenes — primes ≤ 2 000 000

*Boolean sieve with Vec-set inner loop. Tests dense random-access array writes.*

```
  vani           ███████████████████████████████████░     14.8 ms    baseline
  c              ███████████████████████████░░░░░░░░░     11.3 ms   23.7% faster
  cpp            ████████████████████████████████████     15.3 ms    2.8% slower
  rs             ██████████████████████████████████░░     14.3 ms    3.5% faster
```

### Matrix multiplication 256×256 (i64)

*Naïve triple-loop matmul. Tests arithmetic-dense nested loops.*

```
  vani           ██████████████████░░░░░░░░░░░░░░░░░░     15.8 ms    baseline
  c              ███████████████████████░░░░░░░░░░░░░     20.3 ms   28.1% slower
  cpp            ██████████████████████░░░░░░░░░░░░░░     19.6 ms   23.5% slower
  rs             ████████████████████████████████████     31.5 ms   98.7% slower
```

### Sort 1 000 000 integers

*vāṇī uses the built-in sort(mut ref xs); others use stdlib. Tests sort quality.*

```
  vani           ██████████████████████████░░░░░░░░░░    129.3 ms    baseline
  c              ████████████████████████████████████    176.7 ms   36.7% slower
  cpp            ██████████████████████░░░░░░░░░░░░░░    108.1 ms   16.4% faster
  rs             ███████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░     35.5 ms   72.6% faster
```

### Graph BFS — index handles vs. weak_ptr

*KEY BENCHMARK: BFS on a 1 000-node random graph, repeated 1 000×.
  graph.vani / graph_index.{c,cpp,rs} — int-index adjacency list, zero ref-counting.
  graph_weakptr.cpp                   — shared_ptr children + weak_ptr back-edges.*

```
  vani           ███████████░░░░░░░░░░░░░░░░░░░░░░░░░     15.0 ms    baseline
  c              ████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░     11.0 ms   26.9% faster
  C++ (index)    ████████████░░░░░░░░░░░░░░░░░░░░░░░░     16.7 ms   11.3% slower
  C++ (weak_ptr) ████████████████████████████████████     49.1 ms   226.9% slower
  rs             █████████████░░░░░░░░░░░░░░░░░░░░░░░     18.3 ms   21.8% slower
```

### Parallel sum — 50 000 000 elements

*vāṇī: `parallel for … reduce total with +` (3 extra keywords).
C/C++: OpenMP (if available), else serial.
Rust: std::thread manual split.*

```
  vani           ████████████████████████████████████    224.1 ms    baseline
  c              ██████████████████████████░░░░░░░░░░    164.1 ms   26.8% faster
  cpp            █████████████████████████████████░░░    204.2 ms    8.9% faster
  rs             ██████████████████████░░░░░░░░░░░░░░    138.9 ms   38.0% faster
```

### HashMap — 500 000 insert + 500 000 lookup

*Tests open-addressing HashMap throughput.*

```
  vani           █████████████████████░░░░░░░░░░░░░░░     51.0 ms    baseline
  c              ██████████████████░░░░░░░░░░░░░░░░░░     43.6 ms   14.6% faster
  cpp            ██████████████████████░░░░░░░░░░░░░░     53.3 ms    4.4% slower
  rs             ████████████████████████████████████     89.0 ms   74.5% slower
```

### Linked list — 1 000 000 nodes, index-based

*vāṇī/C index approach (no raw pointers): O(1) cache-friendly traversal.
C++/Rust use traditional pointer-linked nodes for comparison.*

```
  vani           ██████████████████████████████░░░░░░     14.2 ms    baseline
  c              █████████████████████████████████░░░     15.5 ms    9.7% slower
  cpp            ████████████████████████████████████     16.8 ms   18.3% slower
  rs             ███████████████████████████████████░     16.5 ms   16.4% slower
```

### Allocation stress — 500 000 struct alloc/free cycles

*Tests allocator throughput; vāṇī uses RAII affine drop.*

```
  vani           ████████████████████████████████████     19.7 ms    baseline
  c              ████████████████████████████░░░░░░░░     15.3 ms   22.2% faster
  cpp            █████████████████████████████░░░░░░░     16.1 ms   18.2% faster
  rs             █████████████████████████░░░░░░░░░░░     13.4 ms   31.8% faster
```

### Array statistics — mean + variance of 10 000 000 values

*vāṇī: two `parallel for … reduce` passes. C/C++/Rust: sequential passes. Tests loop throughput and parallelism.*

```
  vani           █████████████████████████████░░░░░░░     36.1 ms    baseline
  c              ██████████████████████████████████░░     43.0 ms   19.0% slower
  cpp            ████████████████████████████████████     45.3 ms   25.3% slower
  rs             ████████████████████████████████████     45.6 ms   26.2% slower
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

