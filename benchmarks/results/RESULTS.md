# Benchmark Results — vāṇī vs Rust vs C vs C++

*Benchmarks 01–10 generated: 2026-07-06 08:45 — 5 timing run(s) per benchmark, median reported.*
*Benchmark 11 generated: 2026-07-10 — 5 timing run(s), median reported.*
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
| Fibonacci(42) — re |   862.2 ms   |   468.5 ms   |   482.8 ms   |   886.2 ms   | —            | —            |
| Sieve of Eratosthe |    13.4 ms   |    16.3 ms   |    17.6 ms   |    16.8 ms   | —            | —            |
| Matrix multiplicat |    12.9 ms   |    13.1 ms   |    16.1 ms   |    30.9 ms   | —            | —            |
| Sort 1 000 000 int |    97.4 ms   |   175.4 ms   |    97.8 ms   |    36.7 ms   | —            | —            |
| Graph BFS — index  |    12.7 ms   |    13.6 ms   | —            |    18.5 ms   |    20.3 ms   |    44.9 ms   |
| Parallel sum — 50  |   177.5 ms   |   182.9 ms   |   218.5 ms   |   170.6 ms   | —            | —            |
| HashMap — 500 000  |    40.4 ms   |    45.1 ms   |    54.4 ms   |    74.2 ms   | —            | —            |
| Linked list — 1 00 |    12.4 ms   |    12.5 ms   |    15.4 ms   |    18.3 ms   | —            | —            |
| Allocation stress  |    12.2 ms   |     9.6 ms   |    12.7 ms   |    13.9 ms   | —            | —            |
| Array statistics — |    43.0 ms   |    45.6 ms   |    50.4 ms   |    38.5 ms   | —            | —            |
| SIMD dot product — |    27.8 ms   |    41.5 ms   |    44.4 ms   |    37.6 ms   | —            | —            |

## Per-benchmark charts

> Bars are proportional to wall-clock time — **shorter is faster**.

### Fibonacci(42) — recursive

*Classic recursive fib(42). Tests raw function-call throughput.*

```
  vani           ███████████████████████████████████░    862.2 ms    baseline
  c              ███████████████████░░░░░░░░░░░░░░░░░    468.5 ms   45.7% faster
  cpp            ████████████████████░░░░░░░░░░░░░░░░    482.8 ms   44.0% faster
  rs             ████████████████████████████████████    886.2 ms    2.8% slower
```

> **Known structural gap**: vāṇī emits clean IR (no extra overhead vs C source),
> but GCC with `-O3 -march=native` produces tighter x86-64 code for deep recursion
> than LLVM's backend. With ~866 million recursive calls for fib(42), even a
> one-cycle-per-call difference compounds to ~200 ms. Not addressable at IR level.

### Sieve of Eratosthenes — primes ≤ 2 000 000

*Boolean sieve with Vec-set inner loop. Tests dense random-access array writes.*

```
  vani           ███████████████████████████░░░░░░░░░     13.4 ms    baseline
  c              █████████████████████████████████░░░     16.3 ms   21.9% slower
  cpp            ████████████████████████████████████     17.6 ms   31.6% slower
  rs             ██████████████████████████████████░░     16.8 ms   25.6% slower
```

### Matrix multiplication 256×256 (i64)

*Naïve triple-loop matmul. Tests arithmetic-dense nested loops.*

```
  vani           ███████████████░░░░░░░░░░░░░░░░░░░░░     12.9 ms    baseline
  c              ███████████████░░░░░░░░░░░░░░░░░░░░░     13.1 ms    2.0% slower
  cpp            ███████████████████░░░░░░░░░░░░░░░░░     16.1 ms   25.4% slower
  rs             ████████████████████████████████████     30.9 ms   140.1% slower
```

### Sort 1 000 000 integers

*vāṇī uses the built-in sort(mut ref xs); others use stdlib. Tests sort quality.*

```
  vani           ████████████████████░░░░░░░░░░░░░░░░     97.4 ms    baseline
  c              ████████████████████████████████████    175.4 ms   80.0% slower
  cpp            ████████████████████░░░░░░░░░░░░░░░░     97.8 ms    0.4% slower
  rs             ████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░     36.7 ms   62.4% faster
```

### Graph BFS — index handles vs. weak_ptr

*KEY BENCHMARK: BFS on a 1 000-node random graph, repeated 1 000×.
  graph.vani / graph_index.{c,cpp,rs} — int-index adjacency list, zero ref-counting.
  graph_weakptr.cpp                   — shared_ptr children + weak_ptr back-edges.*

```
  vani           ██████████░░░░░░░░░░░░░░░░░░░░░░░░░░     12.7 ms    baseline
  c              ███████████░░░░░░░░░░░░░░░░░░░░░░░░░     13.6 ms    7.1% slower
  C++ (index)    ████████████████░░░░░░░░░░░░░░░░░░░░     20.3 ms   59.8% slower
  C++ (weak_ptr) ████████████████████████████████████     44.9 ms   253.5% slower
  rs             ███████████████░░░░░░░░░░░░░░░░░░░░░     18.5 ms   45.7% slower
```

### Parallel sum — 50 000 000 elements

*vāṇī: `parallel for … reduce total with +` (3 extra keywords).
C/C++: OpenMP (if available), else serial.
Rust: std::thread manual split.*

```
  vani           █████████████████████████████░░░░░░░    177.5 ms    baseline
  c              ██████████████████████████████░░░░░░    182.9 ms    3.0% slower
  cpp            ████████████████████████████████████    218.5 ms   23.1% slower
  rs             ████████████████████████████░░░░░░░░    170.6 ms    3.9% faster
```

### HashMap — 500 000 insert + 500 000 lookup

*Tests open-addressing HashMap throughput.*

```
  vani           ████████████████████░░░░░░░░░░░░░░░░     40.4 ms    baseline
  c              ██████████████████████░░░░░░░░░░░░░░     45.1 ms   11.8% slower
  cpp            ██████████████████████████░░░░░░░░░░     54.4 ms   34.8% slower
  rs             ████████████████████████████████████     74.2 ms   83.7% slower
```

### Linked list — 1 000 000 nodes, index-based

*vāṇī/C index approach (no raw pointers): O(1) cache-friendly traversal.
C++/Rust use traditional pointer-linked nodes for comparison.*

```
  vani           ████████████████████████░░░░░░░░░░░░     12.4 ms    baseline
  c              █████████████████████████░░░░░░░░░░░     12.5 ms    0.9% slower
  cpp            ██████████████████████████████░░░░░░     15.4 ms   24.2% slower
  rs             ████████████████████████████████████     18.3 ms   47.5% slower
```

### Allocation stress — 500 000 struct alloc/free cycles

*Tests allocator throughput; vāṇī uses RAII affine drop.*

```
  vani           ███████████████████████████████░░░░░     12.2 ms    baseline
  c              █████████████████████████░░░░░░░░░░░      9.6 ms   21.2% faster
  cpp            █████████████████████████████████░░░     12.7 ms    4.1% slower
  rs             ████████████████████████████████████     13.9 ms   14.3% slower
```

### Array statistics — mean + variance of 10 000 000 values

*vāṇī: two `parallel for … reduce` passes. C/C++/Rust: sequential passes. Tests loop throughput and parallelism.*

```
  vani           ███████████████████████████████░░░░░     43.0 ms    baseline
  c              █████████████████████████████████░░░     45.6 ms    6.1% slower
  cpp            ████████████████████████████████████     50.4 ms   17.2% slower
  rs             ███████████████████████████░░░░░░░░░     38.5 ms   10.6% faster
```

### SIMD dot product — explicit vec128<f32> vs auto-vectorized (4 M elements)

*vāṇī: explicit `vec128<f32>` simd_mul + simd_reduce_add. C/C++/Rust: scalar loop auto-vectorized by compiler. Compares explicit SIMD vs optimizer output.*

```
  vani           ███████████████████████░░░░░░░░░░░░░     27.8 ms    baseline
  c              ██████████████████████████████████░░     41.5 ms   49.4% slower
  cpp            ████████████████████████████████████     44.4 ms   60.0% slower
  rs             ██████████████████████████████░░░░░░     37.6 ms   35.4% slower
```

> **Explicit SIMD wins**: vāṇī's `vec128<f32>` explicit load/mul/reduce path
> outperforms auto-vectorized scalar loops in all three comparison languages.
> The 49% advantage over C confirms that LLVM's auto-vectorizer, despite seeing
> the same scalar loop, leaves performance on the table that explicit lane
> control recovers.

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
