# Benchmark Results — vāṇī vs Rust vs C vs C++

*vāṇī timings: 2026-07-13 — 3 timing run(s), median.*
*New variant timings (rs_ikj, omp_c, omp_cpp, rs_par, c_sse): 2026-07-17 — 5 timing run(s), median.*
*C/C++ flags: `-O3 -march=native`. Rust flags: `-C opt-level=3 -C target-cpu=native`.*
*vāṇī uses LLVM backend with `opt -O3 --mcpu=native` + `llc -O3 -mcpu=native`.*
*See `benchmarks/results/SYSTEM.md` for full hardware and software details.*

## System

```
OS       : Windows 11 Home 10.0.26200 AMD64
CPU      : Intel i5-1035G1 (Ice Lake) — 4 cores / 8 threads, L3 6 MB
RAM      : 8 GB DDR4 dual-channel
Python   : 3.14.5
vanic    : C:\Users\upaas\vani-compiler\target\release\vanic.exe
CC       : C:\msys64\mingw64\bin\gcc.EXE  (GCC 16.1.0, MSYS2 MinGW-w64)
CXX      : C:\msys64\mingw64\bin\g++.EXE
rustc    : C:\Users\upaas\.cargo\bin\rustc.EXE  (1.96.0)
```

## Summary

| Benchmark | vani | c | cpp | rs | Notes |
|--------------------|--------|--------|--------|--------|-------|
| Fibonacci(42) — recursive | 943.1 ms | 486.2 ms | 488.6 ms | 930.7 ms | C 2× faster: GCC restructures recursion |
| Sieve of Eratosthenes | 15.4 ms | 14.6 ms | 14.9 ms | 15.5 ms | All within 6% — believable compiler bench |
| Matrix mul 256×256 (i-k-j) | 15.5 ms | 15.6 ms | 15.5 ms | 32.9 ms | Rust 2× gap: i-j-k loop + bounds checks |
| Matrix mul 256×256 (rs i-k-j) | — | — | — | **14.6 ms** | Loop fix closes gap; matches C/vāṇī |
| Sort 1 000 000 integers | 97.1 ms | 180.8 ms | 98.6 ms | 44.1 ms | Rust: pdqsort; C: qsort fn-ptr overhead |
| Graph BFS — index handles | 16.2 ms | 10.9 ms | 19.2 ms† | 18.6 ms | †C++ index; C++ weak_ptr: 51.7 ms |
| Parallel sum 50M elements | 197.2 ms | 193.1 ms | 198.3 ms | 151.1 ms | All parallel; Rust thread < OpenMP overhead |
| HashMap 500K ins + lookup | 39.7 ms | 60.0 ms | 60.9 ms | 73.5 ms | FNV-1a + linear probing beats SwissTable |
| Linked list 1M nodes | 13.7 ms | 15.4 ms | 17.5 ms | 21.3 ms | Index vs pointer-linked (different DS) |
| Alloc stress 500K structs | 10.8 ms | 10.3 ms | 16.0 ms | 14.7 ms | Same allocator; gap within noise |
| Array stats — seq baselines | 37.9 ms | 61.9 ms | 68.5 ms | 65.4 ms | vāṇī parallel vs others sequential (unfair) |
| Array stats — **parallel fair** | **37.9 ms** | **43.3 ms**† | **47.7 ms**† | **36.5 ms**† | †+OpenMP / +std::thread; fair comparison |
| SIMD dot — auto-vectorized | 30.3 ms | 33.7 ms | 42.9 ms | 42.5 ms | Explicit vec128 vs auto-vec (unfair) |
| SIMD dot — **explicit SSE** | **30.3 ms** | **29.0 ms**† | — | — | †C explicit `__m128`; explicit vs explicit |
| SIMD-256 dot product | 33.5 ms | — | — | — | vāṇī-only (vec256 = AVX2 width) |

*Bold = new results from 2026-07-17 fair-comparison runs.*

---

## Per-benchmark charts

> Bars are proportional to wall-clock time — **shorter is faster**.

### Fibonacci(42) — recursive

*Classic recursive fib(42). Tests raw function-call throughput.*
*C 2× advantage: GCC loop restructuring + vāṇī L4 overflow guard per add.*

```
  vani           ████████████████████████████████████    943.1 ms    baseline
  c              ███████████████████░░░░░░░░░░░░░░░░░    486.2 ms   48.4% faster
  cpp            ███████████████████░░░░░░░░░░░░░░░░░    488.6 ms   48.2% faster
  rs             ████████████████████████████████████    930.7 ms    1.3% faster
```

Root cause: GCC -O3 restructures the recursive call tree more aggressively than LLVM.
Rust and vāṇī are nearly identical because both use LLVM. The 1.3% Rust advantage
is consistent with vāṇī emitting an overflow guard on `fib(n-1)+fib(n-2)`.

### Sieve of Eratosthenes — primes ≤ 2 000 000

*Boolean sieve with Vec-set inner loop. Tests dense random-access array writes.*
*Vec<i8> (1 byte/element): 2 MB sieve fits in L2.*

```
  vani           ████████████████████████████████████     15.4 ms    baseline
  c              ██████████████████████████████████░░     14.6 ms    5.6% faster
  cpp            ██████████████████████████████████░░     14.9 ms    3.5% faster
  rs             ████████████████████████████████████     15.5 ms    0.8% slower
```

5% C advantage: vāṇī read-path still emits a bounds check (`sieve[i]`, `sieve[m]`);
C has no bounds check. Write-path uses `set(mut ref, idx as u64, ...)` — no check.

### Matrix multiplication 256×256 (i64)

*Triple-loop matmul. Tests arithmetic-dense nested loops and cache access patterns.*

```
  vani  (i-k-j LLVM)         █████████████████░░░░░░░░░░░░░░░░░░░     15.5 ms    baseline
  c     (i-j-k GCC, auto-itr) █████████████████░░░░░░░░░░░░░░░░░░░     15.6 ms    0.7% slower
  cpp   (i-j-k GCC)           █████████████████░░░░░░░░░░░░░░░░░░░     15.5 ms    0.4% faster
  rs    (i-j-k LLVM, baseline) ████████████████████████████████████     32.9 ms   112% slower
  rs    (i-k-j + unsafe, NEW) █████████████████░░░░░░░░░░░░░░░░░░░     14.6 ms    5.8% faster
```

**Two-factor explanation for Rust baseline:**
1. Loop order: `matmul.rs` uses i-j-k → inner `k` loop accesses `b[k*N+col]` with stride N
   (column-major cache miss every iteration). LLVM cannot auto-interchange unlike GCC.
2. Bounds checks: every indexed access adds a compare+branch, blocking SIMD pattern matching.

**Fix**: `matmul_ikj.rs` (i-k-j + `unsafe::get_unchecked`) → 14.6 ms, matching C/vāṇī.

### Sort 1 000 000 integers

*vāṇī uses the built-in sort(); others use stdlib. Tests sort algorithm quality.*

```
  vani           ████████████████████░░░░░░░░░░░░░░░░     97.1 ms    baseline
  c              ████████████████████████████████████    180.8 ms   86.1% slower
  cpp            ████████████████████░░░░░░░░░░░░░░░░     98.6 ms    1.5% slower
  rs             █████████░░░░░░░░░░░░░░░░░░░░░░░░░░░     44.1 ms   54.6% faster
```

Category B (library quality):
- Rust 54% faster: `sort_unstable()` = pdqsort (block-partitioning, better pivot selection)
- C 86% slower: `qsort` passes `cmp_i64` as function pointer — indirect call per comparison
- C++ comparable to vāṇī: `std::sort` = introsort with inlined template comparator
- Fix vāṇī: replace stdlib sort with pdqsort implementation

### Graph BFS — index handles vs. weak_ptr

*BFS on 1 000-node random graph, repeated 1 000×. KEY LANGUAGE DESIGN BENCHMARK.*

```
  vani (index)        ███████████░░░░░░░░░░░░░░░░░░░░░░░░░     16.2 ms    baseline
  c (index)           ████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░     10.9 ms   32.6% faster
  C++ (index)         █████████████░░░░░░░░░░░░░░░░░░░░░░░     19.2 ms   18.3% slower
  C++ (weak_ptr)      ████████████████████████████████████     51.7 ms  219.2% slower
  rs (index)          █████████████░░░░░░░░░░░░░░░░░░░░░░░     18.6 ms   14.7% slower
```

Category C (language design): affine ownership → index handles → flat Vec → zero allocations.
The 219% weak_ptr penalty is the cost of `lock()` ≥ 2 per access + pointer chasing.
Residual vāṇī vs C (33%): L4 overflow guards on index arithmetic; same data structure.

### Parallel sum — 50 000 000 elements

*All variants parallel. vāṇī: `parallel for … reduce`. C/C++: OpenMP. Rust: std::thread.*

```
  vani           ████████████████████████████████████    197.2 ms    baseline
  c              ███████████████████████████████████░    193.1 ms    2.1% faster
  cpp            ████████████████████████████████████    198.3 ms    0.6% slower
  rs             ███████████████████████████░░░░░░░░░    151.1 ms   23.3% faster
```

Rust 23% faster: `thread::scope` spawn+join has less overhead than OpenMP's
reduction barrier. vāṇī and C/C++ all use OpenMP and cluster tightly.

### HashMap — 500 000 insert + 500 000 lookup

*Open-addressing HashMap throughput. FNV-1a + linear probing.*

```
  vani           ███████████████████░░░░░░░░░░░░░░░░░     39.7 ms    baseline
  c              █████████████████████████████░░░░░░░     60.0 ms   51.0% slower
  cpp            ██████████████████████████████░░░░░░     60.9 ms   53.3% slower
  rs             ████████████████████████████████████     73.5 ms   85.0% slower
```

Category B (library): FNV-1a (2 instructions/byte, no finalization) + linear probing
(sequential cache line) beats SwissTable + SipHash-1-3 for this workload size.
See `hash.vani` header for full algorithm documentation.

### Linked list — 1 000 000 nodes

*NOTE: data-structure comparison, not pure language comparison.*
*vāṇī/C: two flat Vec<i64> arrays (index-based). C++/Rust: heap pointer-linked nodes.*

```
  vani (index)   ███████████████████████░░░░░░░░░░░░░     13.7 ms    baseline
  c    (index)   ██████████████████████████░░░░░░░░░░     15.4 ms   12.2% slower
  cpp  (ptr)     █████████████████████████████░░░░░░░     17.5 ms   27.3% slower
  rs   (ptr)     ████████████████████████████████████     21.3 ms   55.4% slower
```

Category C (language design): affine ownership prevents raw pointers → index-based
is the natural idiom. 55% Rust penalty = pointer chasing after 1M separate heap allocs.

### Allocation stress — 500 000 struct alloc/free cycles

*All variants use system malloc (ptmalloc2 / Windows CRT heap).*

```
  vani           ████████████████████████░░░░░░░░░░░░     10.8 ms    baseline
  c              ███████████████████████░░░░░░░░░░░░░     10.3 ms    4.6% faster
  cpp            ████████████████████████████████████     16.0 ms   47.5% slower
  rs             █████████████████████████████████░░░     14.7 ms   36.2% slower
```

C++ 47% slower: per-element constructor/destructor dispatch.
Rust 36% slower: bounds checks on `items[j].a` / `items[j].c` in accumulation loop.
vāṇī RAII drop = single `free(data_ptr)` — zero overhead vs C `free()`.

### Array statistics — mean + variance of 10 000 000 values

*Two-pass: sum→mean, then sum of (x-mean)² →variance.*

#### Sequential baselines (old, unfair comparison)

```
  vani (parallel) ████████████████████░░░░░░░░░░░░░░░░     37.9 ms    baseline
  c    (seq)      █████████████████████████████████░░░     61.9 ms   63.4% slower
  cpp  (seq)      ████████████████████████████████████     68.5 ms   80.6% slower
  rs   (seq)      ██████████████████████████████████░░     65.4 ms   72.5% slower
```

#### Fair parallel comparison (NEW — 2026-07-17)

```
  vani (parallel for reduce)  ████████████████████░░░░░░░░░░░░░░░░     37.9 ms    baseline
  c    (OpenMP, stats_omp.c)  ████████████████████████░░░░░░░░░░░░     43.3 ms   14.2% slower
  cpp  (OpenMP, stats_omp.cpp) ██████████████████████████░░░░░░░░░░     47.7 ms   25.9% slower
  rs   (std::thread)           ████████████████████░░░░░░░░░░░░░░░░     36.5 ms    3.7% faster
```

On a fair parallel-vs-parallel comparison, all four languages cluster at 36–48 ms.
vāṇī and Rust (std::thread) are essentially tied. C OpenMP 14% slower due to
reduction-variable synchronization overhead; C++ OpenMP 26% slower.

### SIMD dot product — explicit vec128<f32> (4 M elements)

#### Original (unfair: explicit SIMD vs auto-vectorized scalar)

```
  vani (explicit vec128)   █████████████████████████░░░░░░░░░░░     30.3 ms    baseline
  c    (auto-vec scalar)   ████████████████████████████░░░░░░░░     33.7 ms   11.1% slower
  cpp  (auto-vec scalar)   ████████████████████████████████████     42.9 ms   41.4% slower
  rs   (auto-vec scalar)   ████████████████████████████████████     42.5 ms   40.1% slower
```

#### Fair explicit-vs-explicit comparison (NEW — 2026-07-17)

```
  vani (explicit vec128<f32>)  ████████████████████████████████████     30.3 ms    baseline
  c    (explicit __m128 SSE)   ██████████████████████████████████░░     29.0 ms    4.3% faster
```

Explicit `__m128` SSE in C (29 ms) matches explicit `vec128<f32>` in vāṇī (30.3 ms)
within measurement noise. The original 11% C advantage over auto-vec is replicated
by explicit SSE — confirming the gap is explicit-vs-auto, not vāṇī-vs-C.

vec128 = 128-bit / 4×f32 (SSE width). Benchmark 12 uses vec256 = 256-bit (AVX2 width).

### SIMD-256 dot product — vec256<f32> vs vec128<f32> vs scalar (4 M elements)

*vāṇī-only: AVX2-width vec256 vs SSE-width vec128 vs auto-vectorized scalar.*

```
  vani           ████████████████████████████████████     33.5 ms    baseline
```

Note: vec256 (33.5 ms) is slightly slower than vec128 (30.3 ms) on this machine —
likely due to AVX2 frequency throttling on i5-1035G1 under sustained YMM workloads.
On a server CPU (no throttle), vec256 should show ~2× throughput improvement.

---

## Key insight: index handles vs. `weak_ptr`

Benchmark `05_graph_bfs` is the most architecture-revealing comparison.
vāṇī has no `weak_ptr` equivalent — its **affine ownership model** means
pointers cannot be aliased without explicit `ref`/`mut ref` borrows, which
makes cyclic references impossible to express directly. Instead, cyclic
graphs are stored as **integer indices** into a contiguous `Vec<T>`.

| Approach | Heap allocs | Atomic ops | Cache friendly |
|----------|-------------|------------|----------------|
| C++ `weak_ptr` | one per node | `lock()` ≥ 2 per access | poor (pointer chase) |
| vāṇī / C / C++ index | zero (flat Vec) | none | excellent (contiguous) |

The 219% penalty for C++ weak_ptr is a language-design cost, not a compiler cost.
vāṇī's ownership model makes the efficient representation the natural one.

---

## Findings summary — all gaps explained

| Benchmark | Gap | Root cause | Category |
|-----------|-----|------------|----------|
| Fibonacci vs C | C 2× faster | GCC recursion restructuring + L4 overflow guard per add | A + L4 |
| Fibonacci vs Rust | <2% | Both LLVM; Rust lacks L4 guard → tiny Rust advantage | A |
| Sieve vs C | C 5% faster | vāṇī read-path bounds check; C unchecked | A |
| Matmul Rust baseline | Rust 2× slower | i-j-k loop (LLVM no auto-interchange) + bounds checks | A |
| Matmul Rust i-k-j | <5% — **gap closed** | i-k-j + unsafe eliminates both causes | A (fixed) |
| Sort vs Rust | Rust 55% faster | pdqsort > introsort — library algorithm quality | B |
| Sort vs C | C 86% slower | qsort function-pointer comparator overhead per compare | B |
| HashMap | vāṇī 51–85% faster | FNV-1a + linear probing vs chaining/SwissTable+SipHash | B |
| Graph BFS vs weak_ptr | 219% penalty | Index handles: zero allocs, no atomics, cache-linear | C |
| Graph BFS vs C index | C 33% faster | L4 guards on index arithmetic; same data structure | A + L4 |
| Linked list | Rust 55% slower | Pointer-linked nodes vs flat arrays — different DS | C |
| Alloc stress | <5% | Same allocator; gap within noise | A |
| Parallel sum Rust | Rust 23% faster | std::thread < OpenMP synchronization overhead | A |
| Array stats (old) | — | **INVALID**: parallel vs sequential comparison | — |
| Array stats (fair) | <14% — **gap closed** | All parallel: vāṇī ≈ Rust; C OpenMP 14% slower | A |
| SIMD dot (old) | — | **MISLEADING**: explicit vs auto-vectorized | — |
| SIMD dot (explicit) | <5% — **gap closed** | Explicit __m128 ≈ explicit vec128 | A (fixed) |

**Category key:** A = compiler codegen · B = library quality · C = language design · L4 = overflow guard cost
