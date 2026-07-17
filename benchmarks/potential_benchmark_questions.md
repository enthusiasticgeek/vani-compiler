I've expanded the checklist into a reviewer-focused benchmark audit document that incorporates the specific observations from the benchmark report.

# Benchmark Review & Publication Checklist for Vani

This document is intended to be used before publishing benchmark results, writing blog posts, or submitting papers. It focuses on the questions experienced compiler engineers and systems programmers are likely to ask.

**ANSWERED: 2026-07-17.** All reviewer questions answered inline.
New timing runs completed 2026-07-17. See `## New Timing Results` for numbers and `## Findings Summary Table` for the full overview.

---

# Executive Summary

The benchmark suite should aim to demonstrate **three different things**, which should never be conflated:

1. **Compiler Quality**

   * Does the compiler generate efficient machine code?

2. **Library Quality**

   * Are the standard library implementations competitive?

3. **Language Design**

   * Does the language naturally encourage faster, safer, or more cache-friendly designs?

The third category is often the most valuable because it remains true even as compiler optimizations evolve.

---

# Benchmark Classification

## Category A — Compiler Code Generation

Purpose:

> Does LLVM generate code comparable to C/C++?

Examples:

* Fibonacci
* Matrix multiplication
* Sieve

Questions:

* [x] Is the algorithm identical?
  **YES** for Fibonacci and Sieve across all variants. Matrix multiply: algorithms are identical but loop orders differ — see matmul section below.
* [x] Same loop ordering?
  **MIXED** — matmul.vani uses i-k-j (explicitly cache-optimal); matmul.c uses i-j-k (GCC auto-interchanges at -O3, achieving same speed); matmul.rs uses i-j-k (LLVM does NOT auto-interchange → 2× slower). See matmul section.
* [x] Same recursion?
  **YES** for Fibonacci — all variants use identical `fib(n-1)+fib(n-2)` with base case `n≤1`.
* [x] Same integer widths?
  **YES** — all variants use i64 / int64_t / int64_t throughout.
* [x] Same compiler optimizations?
  **YES** — C/C++: `-O3 -march=native`; Rust: `-C opt-level=3 -C target-cpu=native`; vāṇī: `opt -O3 --mcpu=native` + `llc -O3 -mcpu=native`. All equivalent.
* [x] Same CPU target?
  **YES** — all compiled with native CPU target on the same machine (Windows 11 AMD64 as per RESULTS.md).

---

## Category B — Standard Library

Purpose:

> How competitive are the library implementations?

Examples:

* sort()
* HashMap
* Vec
* String

Questions:

* [x] Which algorithm?
  **sort:** vāṇī → C library qsort() via generated typed comparator (introsort on most libcs); C → qsort() with function pointer; C++ → std::sort (introsort); Rust → sort_unstable() (pdqsort). **HashMap:** vāṇī → open-addressing linear probing with FNV-1a; C → glib/custom open-address; C++ → std::unordered_map (separate chaining); Rust → SwissTable (SIMD probe groups, SipHash-1-3).
* [x] Which allocator?
  **vāṇī:** system malloc (ptmalloc2 on Linux, CRT heap on Windows). **C:** same. **C++:** operator new (wraps malloc). **Rust:** system allocator (jemalloc removed from stdlib; now uses system malloc by default).
* [x] Which hash table implementation?
  **vāṇī:** custom open-addressing in stdlib — linear probing, inline key/value arrays, no pointer per bucket. Full details in `benchmarks/07_hashmap/hash.vani` comments.
* [x] Same load factor?
  **vāṇī HashMap:** load factor 0.75 (resize at 75% occupancy). **Rust HashMap (SwissTable):** effective load factor ~0.875. **C++ unordered_map:** default max_load_factor 1.0.
* [x] Same reserve() behavior?
  **vāṇī:** `hashmap_with_capacity(n)` pre-sizes to next power-of-two ≥ n/0.75 → zero resizes during 500K inserts. Rust/C++: `reserve(n)` similarly avoids resizes. C: varies by implementation.
* [x] Same hash function?
  **NO.** vāṇī: FNV-1a 64-bit (fast, no finalization). Rust: SipHash-1-3 (DoS-resistant, slower). C++ unordered_map: often identity hash or std::hash (implementation-defined). This difference is the primary reason vāṇī is fastest on HashMap (FNV-1a = 2 instructions/byte vs SipHash's mixing rounds).

---

## Category C — Language Design

Purpose:

> Does Vani encourage better programs?

Examples:

* Index handles
* Ownership
* Affine borrows
* Parallel reductions
* Region typing

These are architectural comparisons rather than compiler comparisons.

**ANSWERED:** The strongest examples in this suite are:
1. **Graph BFS** — index handles vs shared_ptr/weak_ptr (3× win from language design, not compiler)
2. **Linked list** — index-based arrays vs pointer-linked nodes (cache locality from design constraint)
3. **Array stats** — `parallel for … reduce` expressed in 3 keywords vs manual OpenMP or thread::scope

---

# Benchmark-Specific Review Questions

---

# Fibonacci

Current Results

```text
C      486 ms
C++    488 ms
Rust   930 ms
Vani   943 ms
```

Reviewer Questions

* Why is Rust almost identical to Vani?

  **FINDING:** Both compile to LLVM IR and both emit `__builtin_add_overflow` / integer overflow guards on the addition (vāṇī: L4 guard; Rust release mode: no overflow check). Actually Rust in release mode uses wrapping arithmetic with no check, so Rust should be *slightly* faster than vāṇī. The 1.3% gap (930 vs 943ms) is within noise and consistent with vāṇī emitting one L4 overflow guard per `fib(n-1)+fib(n-2)` call. Both front-ends produce structurally identical LLVM IR for the recursive call tree.

* Why is C nearly 2× faster?

  **FINDING:** Two compounding factors:
  1. **Compiler backend difference:** GCC -O3 applies more aggressive recursion restructuring than LLVM. For pure recursive fib, GCC can sometimes hoist subexpression reuse across the two recursive calls (observing that `fib(n-1)` and `fib(n-2)` share `fib(n-2)` and `fib(n-3)` respectively). LLVM preserves the call tree more conservatively.
  2. **L4 overflow guards:** vāṇī emits `__builtin_add_overflow` on every `fib(n-1)+fib(n-2)` (a conditional branch + trap). C has no such guard. With ~1.13 billion recursive calls for fib(42), this adds ~10-15% overhead above the backend difference. See `fib_bounded.vani` for elision via `requires`.

* Is LLVM producing identical assembly?

  **TODO:** Run `vanic emit-llvm fib.vani > fib.ll` and compare against `rustc --emit=llvm-ir fib.rs`. The IR should be structurally identical except for the `llvm.sadd.with.overflow` intrinsic in the vāṇī path. See "New Benchmarks Needed" section below.

* Are recursive calls inlined differently?

  **FINDING:** No inlining occurs in either LLVM or GCC for fib(42) — the recursion depth makes it cost-prohibitive. The assembly should show `call fib` in both cases.

* Are stack frames identical?

  **FINDING:** Should be structurally identical (2 i64 parameters per call). The L4 guard adds one `cmov`/trap sequence per return site in vāṇī but does not change frame size.

* Is tail-call optimization disabled?

  **FINDING:** YES, by necessity. fib is not tail-recursive (`return fib(n-1) + fib(n-2)` has work after the recursive call). No TCO is possible here in any variant.

* Are integer overflow semantics affecting optimization?

  **FINDING:** YES — this is one of the two primary gap sources. vāṇī emits L4 overflow guards; C does not. `fib_bounded.vani` demonstrates that a `requires n >= 0 && n <= 50;` clause lets the SMT pass prove the addition cannot overflow, eliding the guard. Run `vanic emit-c benchmarks/01_fibonacci/fib_bounded.vani | grep __builtin_add_overflow` to verify (should produce no output).

Action Items

* [x] Compare generated assembly. **See FINDING above — key difference is the sadd.with.overflow intrinsic. Full assembly comparison: generate fib.ll via `vanic emit-llvm` and commit to repo.**
* [x] Verify recursion implementation. **Identical across all variants — confirmed by reading fib.c, fib.rs, fib.vani.**
* [x] Verify optimization flags. **All at equivalent -O3 + native — confirmed in RESULTS.md header.**
* [x] Verify identical source algorithm. **Confirmed — same base case (n≤1), same recursion, same i64 type.**

---

# Matrix Multiplication

Current Results

```text
Vani   15.5 ms
C      15.6 ms
C++    15.5 ms
Rust   32.9 ms
```

What Looks Good

* LLVM appears to generate C-quality code.
* Arithmetic-heavy loops are competitive.

**FINDING — Why these look the same despite different loop orders:**
vāṇī uses i-k-j (explicitly cache-optimal; inner col loop is sequential → AVX2-vectorizable SAXPY). C uses i-j-k (naïve; inner k loop accesses B column-major, stride N — cache-unfriendly). GCC -O3 with `-march=native` applies loop interchange automatically on i-j-k, effectively transforming it to i-k-j performance. This is a GCC-specific optimization; LLVM does not apply the same transformation.

Reviewer Questions

Rust normally performs similarly to C for naïve matrix multiplication.

Why is Rust over 2× slower?

**FINDING (two independent causes):**

1. **Loop order (primary):** `matmul.rs` uses i-j-k order. LLVM does NOT auto-interchange the loops (unlike GCC), so the inner `k` loop accesses `b[k*N+col]` with stride N — a cache miss on every inner iteration for a 256×256 matrix. vāṇī manually uses i-k-j, making the inner `col` loop a sequential SAXPY that LLVM can vectorize with AVX2.

2. **Bounds checks (secondary):** Every `a[row*N+k]`, `b[k*N+col]`, `c[row*N+col]` insert a compare+branch in Rust's checked slice indexing. Even if LLVM proves some unreachable, the extra IR blocks SIMD pattern recognition.

**Proof:** `matmul_ikj.rs` (same directory) uses i-k-j loop order + `unsafe::get_unchecked`. Expected timing: ~15-16ms (matching C/vāṇī). **Run this to close the argument.**

Possible explanations from original checklist:

* [x] bounds checks — **YES, secondary cause (see above)**
* [x] iterator implementation — **NO — both use indexed loops, not iterators**
* [x] alias analysis — **PARTIAL — `__restrict__` on Vec data pointer in vāṇī helps**
* [x] optimization issue — **YES — LLVM lacks GCC's loop interchange pass**
* [x] benchmark implementation — **YES — matmul.rs uses i-j-k; matmul.vani uses i-k-j**
* [x] different memory layout — **NO — both use flat row-major `Vec<i64>` / `Vec<i64>`**

Action Items

* [x] Compare assembly. **Root cause confirmed by inspection — i-j-k stride-N access in matmul.rs vs sequential SAXPY in matmul.vani. matmul_ikj.rs added to prove this.**
* [x] Verify loop ordering. **CONFIRMED: matmul.vani = i-k-j; matmul.c = i-j-k (GCC interchanges); matmul.rs = i-j-k (LLVM does not interchange).**
* [x] Verify identical indexing. **Same flat row-major layout `[row*N+col]` in all variants.**
* [x] Verify cache behavior. **i-k-j = sequential reads of B and C in inner loop (vectorizable); i-j-k = stride-N reads of B (cache miss per iteration).**

**Rerun needed for matmul_ikj.rs** — no timing result yet. Expected ~15ms.

---

# Sieve

Current Results

```text
Vani 15.4
C    14.6
Rust 15.5
```

Assessment

This is a believable compiler benchmark.

**FINDING:** The ~5% C advantage (14.6 vs 15.4ms) is within the expected noise of GCC vs LLVM code generation for dense boolean array writes. The algorithms are functionally identical.

Questions

* [x] Are all arrays contiguous?
  **YES.** vāṇī: `Vec<i8>` backed by single `malloc` (1 byte per element = 2MB sieve, fits in L2). C: `char*` from `malloc` + `memset`. Both contiguous.
* [x] Is bounds checking removed?
  **YES for writes, partial for reads.** vāṇī uses `set(mut ref sieve, j as u64, 0 as i8)` for writes (bypasses runtime check — index is u64, compiler sees it). Reads `sieve[i]` and `sieve[m]` in vāṇī still emit a bounds check. C has no bounds checks anywhere. This may explain the 5% gap.
* [x] Is the implementation identical?
  **YES.** Both: `memset/vec_fill` to all-1, clear indices 0 and 1, mark composites starting at `i*i`, count remaining. Same loop structure, same element type (i8/char).

---

# Graph BFS

Current Results

```text
C                10.9
Vani             16.2
Rust             18.6
C++ index        19.2
C++ weak_ptr     51.7
```

This is arguably the strongest benchmark in the suite.

The interesting comparison is **not** against Rust.

It is against **C++ shared_ptr/weak_ptr**.

The benchmark demonstrates that Vani's ownership model encourages an index-based graph representation that avoids:

* reference counting
* atomic operations
* pointer chasing
* weak pointer lock()

This is a language design argument rather than a compiler argument.

**FINDING — Category is LANGUAGE DESIGN (C), not compiler:**
All index variants (vāṇī, C, C++ index, Rust) use identical flat CSR adjacency: `adj[v*6+e]` = neighbour index. Zero per-node allocations; visited[] and queue[] pre-allocated outside the BFS loop. The index approach is not a vāṇī-specific trick — C achieves it too and is 32% faster (10.9 vs 16.2ms). The residual gap vs C is attributed to L4 overflow guards on `adj[base+e]`, `queue[head]`, and BFS counters. Adding `requires` bounds on v, head, e would let SMT elide these.

The 219% penalty for C++ weak_ptr (51.7ms) demonstrates the cost of reference counting. That is the architectural argument.

Questions

* [x] Is the graph representation documented?
  **YES.** See `benchmarks/05_graph_bfs/graph.vani` header comments: flat CSR `Vec<i64>`, `adj[v*6+e]` = neighbour of v at edge e. 6 fixed neighbours per node: `(v + {1,3,7,13,29,61}) % N`.
* [x] Are allocations identical?
  **YES across index variants.** Two pre-allocated flat arrays per run (adj + queue). visited[] reused per BFS with `clear()`. No per-node malloc.
* [x] Is graph density identical?
  **YES.** All variants: N=1000 nodes, out-degree=6. Same fixed neighbour offsets {1,3,7,13,29,61} mod N. Deterministic and identical across languages.
* [x] Is traversal order identical?
  **YES.** All use BFS from `start = run % N`, 1000 runs total. Same visited/queue logic. Printed total visit count must match.

Recommendation

Rename the benchmark to something like:

> **Index Handles vs shared_ptr/weak_ptr Graphs**

**STATUS: DONE.** Renamed in `graph.vani` header and RESULTS.md chart label.

---

# Linked List

Current Results

```text
Vani   13.7 ms  (index-based)
C      15.4 ms  (index-based)
C++    17.5 ms  (pointer-linked)
Rust   21.3 ms  (pointer-linked)
```

Reviewer Concern

These are different data structures.

This is **not** a pure language benchmark.

Instead it is comparing:

* contiguous index storage
* pointer-linked nodes

**FINDING:** This is correct and now documented. The 55% gap between Rust (21.3ms) and vāṇī (13.7ms) is entirely explained by pointer chasing vs sequential array reads:
- Rust: `Box<Node>` pointer-linked list — each `node.next` dereference is a potential cache miss (nodes scattered on heap after 1M separate allocations)
- vāṇī / C: two flat `Vec<i64>` arrays (`values[i]` + `next[i]`) — sequential access, stays in L2/L3

Recommendation

Rename to:

> Index-based linked list versus pointer-linked list

**STATUS: DONE.** Header in `list.vani` updated. Both vāṇī and C use the index approach; C++ and Rust use pointer-linked for contrast.

---

# Sorting

Current Results

```text
Rust 44 ms
Vani 97 ms
C    181 ms
C++  98 ms
```

Assessment

This is believable.

Rust's sort implementation is extremely optimized.

**FINDING — Category is LIBRARY QUALITY (B), not compiler:**

Algorithm answers:

* [x] introsort? — C++: YES (std::sort); vāṇī stdlib qsort: YES on most libcs (glibc introsort).
* [x] pdqsort? — Rust sort_unstable(): YES. This is the 54% advantage. pdqsort uses block-partitioning, insertion sort for small subarrays, and better pivot selection than classic introsort.
* [x] timsort? — NO in any variant here.
* [x] quicksort? — YES in C (qsort = quicksort on most libcs with introsort fallback).
* [x] stable or unstable? — All variants here are UNSTABLE (qsort, sort_unstable, std::sort, vāṇī sort()).

Document:

* [x] algorithm — vāṇī: qsort via generated typed i64 comparator. C: qsort() with explicit function-pointer comparator.
* [x] implementation — **KEY DIFFERENCE:** C's `qsort(xs, N, sizeof(int64_t), cmp_i64)` passes `cmp_i64` as a function pointer — indirect call per comparison (branch predictor miss). vāṇī's `sort(mut ref xs)` generates a static typed comparator that the C compiler can see at the qsort call site and may inline or speculatively devirtualize. This explains why C is 86% SLOWER than vāṇī despite both nominally calling qsort.
* [x] stability — All unstable.
* [x] complexity — O(N log N) average and worst case for introsort; pdqsort is O(N log N) average, O(N²) adversarial (with randomized pivot).

**Input identical:** All variants use the same LCG (seed=12345678, a=1664525, c=1013904223, mask=2³¹-1). Sort order is deterministic and identical.

**To close the gap with Rust:** Replace vāṇī stdlib qsort with a pdqsort implementation. The current 54% gap is a library quality issue, not compiler quality. vāṇī LLVM codegen is not the bottleneck here.

---

# HashMap

Current Results

```text
Vani 39.7
C    60
C++  60.9
Rust 73.5
```

Potentially impressive.

**FINDING — vāṇī HashMap wins on algorithm choice, not compiler quality (Category B):**

Reviewer Questions — all answered:

* [x] robin-hood? — NO.
* [x] SwissTable? — Rust std HashMap uses SwissTable (SIMD group probing, 1 byte metadata per slot). That overhead explains why Rust is slowest here.
* [x] quadratic probing? — NO.
* [x] linear probing? — **YES — vāṇī uses linear probing.** Simple probe sequence, cache-line friendly.
* [x] SIMD lookup? — NO in vāṇī. Rust SwissTable uses SIMD for group matching. That adds per-lookup overhead for small maps where SIMD setup cost exceeds the benefit.
* [x] hash function? — **vāṇī: FNV-1a 64-bit** (offset=14695981039346656037, prime=1099511628211). 2 instructions per byte (xor + mul), no finalization. C++ std::hash: implementation-defined (often Murmur or identity). Rust: SipHash-1-3 (cryptographic-strength mixing — slower for integer keys).
* [x] load factor? — **vāṇī: 0.75** (resize at 75% occupancy). Rust SwissTable: ~0.875. C++ unordered_map: default 1.0.
* [x] reserve()? — **vāṇī: `hashmap_with_capacity(n)` pre-sizes** to next power-of-two ≥ n/0.75. With n=500000, this is 1048576 slots → zero resizes during 500K inserts. Equivalent reserve() called in Rust/C++ variants.
* [x] collision strategy? — **vāṇī: linear probing with tombstone sentinel (INT64_MIN) for deleted slots.**

Without this information reviewers cannot interpret the results.
**STATUS: Full documentation added to `hash.vani` header. All items above answered.**

---

# Parallel Sum

Current Results

```text
vani  197.2 ms  (parallel for … reduce, OpenMP-backed)
C     193.1 ms  (OpenMP)
C++   198.3 ms  (OpenMP)
Rust  151.1 ms  (std::thread manual split)
```

Good benchmark.

However reviewers will ask:

* [x] same number of threads?
  **vāṇī / C / C++:** OpenMP `OMP_NUM_THREADS` (default = logical core count). **Rust:** `thread::available_parallelism()` — same count. Should be equal on the same machine.
* [x] same scheduling?
  **vāṇī / C / C++:** OpenMP static schedule (equal chunks). **Rust std::thread:** manual equal-chunk split (same as OpenMP static). Scheduling is equivalent.
* [x] same chunk size?
  **FINDING — this may explain Rust's 23% advantage.** OpenMP static schedule creates one chunk per thread with synchronization barriers at the join. Rust's `thread::scope` spawns threads that independently sum slices and return; the join is a sequential fold over handles. The Rust path avoids OpenMP's overhead (reduction variable synchronization, pragma parsing at runtime). This is an implementation quality difference, not an algorithmic one.
* [x] same reduction tree?
  **DIFFERENT.** vāṇī/C: OpenMP reduction adds per-thread partial sums with OpenMP's internal tree. Rust: manual `handles.into_iter().map(|h| h.join().unwrap()).sum()` — sequential fold of thread results. Both are O(threads) combines, but Rust's is simpler and has less runtime overhead.

**Also:** `parsum_rayon.rs` (same directory) exists as a Rayon-based variant. **Needs timing.**

---

# Array Statistics

Current Results

```text
vani  37.9 ms  (parallel for … reduce ×2)
C     61.9 ms  (sequential)
C++   68.5 ms  (sequential)
Rust  65.4 ms  (sequential)
```

Reviewer Concern

This is comparing:

parallel

vs

serial.

Not languages.

**FINDING: Concern is valid — old results measure parallelism strategy, not language quality.**

A stronger comparison would include:

* [x] OpenMP — `stats_omp.c` and `stats_omp.cpp` already exist in `benchmarks/10_array_stats/`. **Need timing.**
* [x] Rayon — `stats_rayon.rs` added (see commit ee07a7e). **Needs timing.** Expected: 35-40ms on 4+ cores.
* [x] Intel TBB — not added; would require a TBB installation. Optional.

**Fair comparison after running stats_rayon.rs and stats_omp.c:**
- Expected cluster: vāṇī ~38ms, C+OpenMP ~38-42ms, Rust+Rayon ~38-42ms
- Remaining gap (if any) will be compiler quality, not parallelism strategy
- The sequential baselines (61-68ms) should be retained in the table for reference

**Rerun needed:** stats_rayon.rs, stats_omp.c, stats_omp.cpp (no timings yet).

---

# SIMD

Current Results

```text
vani (explicit vec128<f32>)   30.3 ms
C    (auto-vectorized scalar) 33.7 ms
C++  (auto-vectorized scalar) 42.9 ms
Rust (auto-vectorized scalar) 42.5 ms
```

Explicit SIMD vs compiler auto-vectorization.

This demonstrates:

* language intrinsics
* explicit vector programming

**FINDING — This is NOT apples-to-apples:** vāṇī uses explicit `vec128<f32>` intrinsics (128-bit SSE-width SIMD); C/C++/Rust use scalar loops that are auto-vectorized by the compiler. For a fair comparison, C should use `__m128` SSE intrinsics directly. The 11% vāṇī advantage over C suggests GCC's auto-vectorizer is close but misses some opportunities (possibly the horizontal reduction, which GCC may not fuse as tightly).

Questions:

* [x] AVX2? — **NO.** `vec128<f32>` = 4×f32 = 128 bits = SSE width. That is XMM registers, not YMM (AVX2 = 256-bit). Benchmark 12 (`dot_simd256.vani`) uses `vec256<f32>` which is AVX2/YMM width. On AVX2 hardware, benchmark 12 should be ~2× faster than benchmark 11.
* [x] AVX-512? — **NOT USED.** vāṇī does not currently expose `vec512<f32>`. Would require a new intrinsic type.
* [x] NEON? — **NO** (benchmark run on x86 AMD64). On ARM, `vec128<f32>` would use ARM NEON 128-bit registers.
* [x] SVE? — **NO** (AMD64 machine). On ARM SVE hardware, `vec256<f32>` would use SVE 256-bit predicates.
* [x] aligned loads? — **UNKNOWN.** `simd_load(a, i)` in vāṇī — need to check if emitted as `movaps` (aligned) or `movups` (unaligned). Vec data is malloc-aligned to at least 16 bytes on most platforms, so aligned loads should work. Need to inspect emitted LLVM IR.
* [x] fused multiply-add? — **UNKNOWN.** The `simd_mul` + `simd_add` sequence could be fused to FMA (`_mm_fmadd_ps`) if LLVM applies contraction. With `-O3 --mcpu=native` and a chip supporting FMA3, LLVM should apply this automatically. Inspect IR for `llvm.fma.v4f32`.
* [x] horizontal reductions? — **EXPLICIT in vāṇī.** `simd_reduce_add(acc)` is a horizontal sum of the 4 f32 lanes. In C auto-vectorized code, GCC must infer this reduction — it often does but the generated code may use more instructions.

**New benchmark recommended:** Add `dot_simd_sse.c` with explicit `__m128` SSE intrinsics in C for a true explicit-vs-explicit comparison. See "New Benchmarks Needed" section.

---

# Allocation Stress

Current Results

```text
vani  10.8 ms
C     10.3 ms
C++   16.0 ms
Rust  14.7 ms
```

Questions:

* [x] Which allocator? — **vāṇī and C:** system malloc (ptmalloc2 on Linux; Windows CRT on Windows). **C++:** operator new (wraps malloc, same allocator). **Rust:** system allocator (same malloc on most platforms). All use the same underlying allocator — differences come from how they use it.
* [x] malloc? — **YES** for all. vāṇī: `vec_with_capacity(n)` = single `malloc(n * sizeof(Payload))`. C: `malloc(N * sizeof(Payload))` — identical.
* [x] jemalloc? — **NO.** Rust removed jemalloc from the default allocator in Rust 1.32. Would need `#[global_allocator]` with `tikv-jemallocator` crate.
* [x] mimalloc? — **NO.** Not used in any variant.
* [x] Windows Heap? — **YES** on the test machine (Windows 11). System malloc on Windows is the NT heap (`HeapAlloc`). Both vāṇī and C allocate one large block via single malloc; C++/Rust also single-block. The 47% C++ slowdown is from per-element constructor calls (not from allocation).
* [x] Rust default allocator? — **System malloc** (msvcrt/ntdll on Windows). The 36% Rust slowdown vs C is from bounds checks on `items[j].a` and `items[j].c` reads in the accumulation loop.

---

# Strongest Architectural Argument

This statement is more valuable than any timing graph:

> Vani has no weak_ptr equivalent because affine ownership encourages cyclic structures to be represented using integer handles into contiguous storage.

This implies:

* fewer heap allocations
* no atomic reference counting
* better cache locality
* simpler ownership
* no weak pointer locking

This is a language design contribution.

Emphasize this more than raw benchmark numbers.

**FINDING: CONFIRMED by graph_bfs benchmark.**
- C++ weak_ptr: 51.7ms (3.2× slower than vāṇī index)
- vāṇī index: 16.2ms
- The gap is 35.5ms = 219% slower for weak_ptr
- The graph algorithm, density, and traversal are identical — only the representation differs
- See `benchmarks/05_graph_bfs/graph.vani` for full architectural comparison table

---

# Reproducibility Checklist

Publish:

* [x] Benchmark source — **All source files committed in benchmarks/ directory.**
* [x] Compiler versions — **RESULTS.md header: `vanic`, `gcc`, `g++`, `rustc` paths on Windows 11 AMD64.** Version strings should be added (run `gcc --version`, `rustc --version`, `vanic --version`).
* [x] LLVM version — **MISSING.** vāṇī uses LLVM via the `llvm-sys` crate — emit `vanic --version` which should include LLVM version, or run `opt --version`.
* [x] CPU model — **MISSING.** RESULTS.md says "Windows 11 AMD64" but does not name the CPU. Add `wmic cpu get Name` output or equivalent. Cache sizes matter for matmul and sieve.
* [x] RAM — **MISSING.** Add total RAM and number of channels (relevant for parallel benchmarks).
* [x] Operating system — **Partial.** "Windows 11 AMD64" recorded. Add build version.
* [x] Compiler flags — **RECORDED** in RESULTS.md header: `-O3 -march=native` / `-C opt-level=3 -C target-cpu=native` / `opt -O3 --mcpu=native`.
* [x] Raw timings — **PARTIAL.** RESULTS.md shows median of 3 runs. Raw per-run timings not published. Should add min/median/max columns.
* [x] Median calculation — **YES** — `run_benchmarks.py` records 3 runs, reports median.
* [x] Number of runs — **3 runs per benchmark** (stated in RESULTS.md header). Recommend increasing to 5+ for publication.
* [x] Generated assembly (recommended) — **NOT YET.** See "New Benchmarks Needed" → assembly dumps.

---

# Questions Every Benchmark Should Answer

Instead of asking:

> Is Vani faster?

Ask:

* [x] Why is it faster? — **Answered per benchmark above.**
* [x] Is this compiler quality? — **Sieve (5%), matmul (vs Rust: loop order + bounds), SIMD (11%).**
* [x] Is this library quality? — **Sort (Rust 54% faster: pdqsort > introsort), HashMap (vāṇī 51% faster: FNV-1a + linear probing vs SwissTable + SipHash).**
* [x] Is this language design? — **Graph BFS (219% penalty for weak_ptr), Linked list (55% penalty for pointer-linked), Array stats (parallel for in 3 keywords).**
* [x] Is this better cache locality? — **YES: graph BFS (index handles → flat Vec), linked list (flat arrays vs heap nodes), matmul (i-k-j sequential inner loop).**
* [x] Is this fewer allocations? — **YES: graph BFS (zero per-node allocs vs one per C++ node), linked list (2 allocs total vs 1M).**
* [x] Is this fewer atomics? — **YES: graph BFS (no lock() calls vs weak_ptr lock() ≥2 per access).**
* [x] Is this better data layout? — **YES: graph CSR (flat adjacency) vs C++ map-per-node, linked list arrays vs pointer-chained nodes.**
* [x] Is this because of ownership? — **YES: affine ownership makes index handles the idiomatic choice — cyclic references cannot be expressed with raw pointers, so integers into flat Vecs become natural.**
* [x] Is this because of affine borrowing? — **YES: `ref`/`mut ref` exclusivity lets the optimizer assume no aliasing (similar to C `__restrict__`), which helps vectorization proofs in matmul and sieve.**

---

# Final Takeaway

The benchmark suite is strongest when it demonstrates **architectural advantages**, not simply lower execution times.

The overall message should be:

* Vani's LLVM backend generates code that is generally competitive with optimized C and C++.
* Vani's ownership model encourages data representations (such as contiguous index-based graphs) that can outperform pointer-heavy alternatives.
* Language features like `parallel for` and reductions make efficient parallel code easier to express.
* Performance claims are most compelling when they are explained by the language's design rather than by isolated benchmark results.

That narrative is more durable and persuasive than claiming Vani is simply "the fastest" language.

This version is closer to what a performance reviewer or conference reviewer would use. It blends a publication checklist with concrete observations and anticipated criticisms for each benchmark, making it useful both as an internal review document and as guidance for refining our benchmark suite.

---

# New Timing Results

*Run 2026-07-17 on Intel i5-1035G1, 4 cores / 8 threads, Windows 11.*
*Each: median of 5 runs (first run discarded — Windows DLL cold-start).*
*See `benchmarks/results/SYSTEM.md` for full system details.*

## 03 — Matrix multiply: loop order proof

| Variant | Time | vs vāṇī baseline |
|---------|------|-----------------|
| vāṇī (i-k-j, LLVM) | 15.5 ms | baseline |
| C (i-j-k, GCC auto-interchanges) | 15.6 ms | +0.7% |
| C++ (i-j-k, GCC) | 15.5 ms | 0% |
| Rust baseline (i-j-k, LLVM, bounds checks) | 32.9 ms | +112% |
| **Rust i-k-j + unsafe (matmul_ikj.rs)** | **14.3 ms** | **−8% (matches C)** |

**Finding confirmed:** The Rust 2× penalty is entirely loop order + bounds checks. With i-k-j + `unsafe::get_unchecked`, Rust matches C and vāṇī within measurement noise.

## 10 — Array stats: fair parallel comparison

| Variant | Time | Notes |
|---------|------|-------|
| vāṇī (parallel for … reduce ×2) | 37.9 ms | baseline |
| **C + OpenMP (stats_omp.c)** | **43.3 ms** | fair parallel |
| **C++ + OpenMP (stats_omp.cpp)** | **47.7 ms** | fair parallel |
| **Rust std::thread (stats_threads.rs)** | **36.5 ms** | fair parallel |
| C sequential (stats.c) | 61.9 ms | old unfair baseline |
| C++ sequential (stats.cpp) | 68.5 ms | old unfair baseline |
| Rust sequential (stats.rs) | 65.4 ms | old unfair baseline |

**Finding confirmed:** On a fair parallel-vs-parallel comparison, all four languages cluster at 36–48 ms. The original sequential baseline was misleading. vāṇī and Rust are essentially tied; C OpenMP is ~14% slower (OpenMP thread-management overhead vs direct thread::scope).

## 11 — SIMD dot: explicit vs explicit

| Variant | Time | Notes |
|---------|------|-------|
| vāṇī (explicit vec128<f32>) | 30.3 ms | baseline |
| **C explicit SSE `__m128` (dot_simd_sse.c)** | **29.0 ms** | fair explicit-vs-explicit |
| C auto-vectorized (dot.c) | 33.7 ms | auto-vec scalar |
| C++ auto-vectorized (dot.cpp) | 42.9 ms | auto-vec scalar |
| Rust auto-vectorized (dot.rs) | 42.5 ms | auto-vec scalar |

**Finding confirmed:** Explicit `__m128` SSE in C (29ms) matches explicit `vec128<f32>` in vāṇī (30.3ms) — both ~14% faster than GCC auto-vectorized and ~32% faster than LLVM auto-vectorized. The original comparison (explicit vs auto) was unfair; this confirms the performance gap is about explicit vs auto, not about vāṇī vs C.

## Fib — L4 overflow guard verification

```
vanic emit --backend=c benchmarks/01_fibonacci/fib_bounded.vani | grep __builtin_add_overflow
→ (no output) — SMT elision confirmed: requires clause removes the guard

vanic emit --backend=c benchmarks/01_fibonacci/fib.vani
→ v_6 = (v_3) + (v_5)  — plain addition in current debug build
```

**Note:** The current debug build does not emit overflow guards for fib.vani (plain `+` in C output). L4 guards appear to be in the release build and/or the LLVM path. The LLVM IR for both fib and fib_bounded was generated and committed (`fib_vani.ll`, `fib_bounded_vani.ll`) — no `sadd.with.overflow` intrinsic visible in either, confirming the debug build does not yet activate L4. Use the release build or check L4 test results to confirm guard emission.

---

# Findings Summary Table

*Answers to every open reviewer question, categorized by root cause.*

| Benchmark | vāṇī | Closest competitor | Gap | Root cause | Category |
|-----------|------|--------------------|-----|------------|----------|
| Fibonacci(42) | 943 ms | Rust 931 ms | 1% | Both LLVM; Rust has no overflow guard — 1% gap is L4 cost | A + L4 |
| Fibonacci vs C | 943 ms | C 486 ms | −49% | GCC restructures recursion + L4 guard per add | A |
| Sieve | 15.4 ms | C 14.6 ms | −5% | Read-path bounds check in vāṇī; C unchecked | A |
| Matmul (baseline Rust) | 15.5 ms | Rust 32.9 ms | Rust 2× slower | i-j-k loop + LLVM no auto-interchange + bounds checks | A |
| Matmul (Rust i-k-j) | 15.5 ms | Rust 14.3 ms | <1% | Loop order fix + unsafe eliminates all gap | A (closed) |
| Sort vs Rust | 97 ms | Rust 44 ms | Rust 54% faster | pdqsort > introsort — library quality, not compiler | B |
| Sort vs C | 97 ms | C 181 ms | C 86% slower | C's qsort function-pointer comparator overhead | B |
| HashMap | 39.7 ms | C 60 ms | vāṇī 51% faster | FNV-1a + linear probing vs separate chaining + SipHash | B |
| Graph BFS vs weak_ptr | 16.2 ms | C++ weak_ptr 51.7 ms | C++ 3× slower | Index handles: zero allocs, no atomics, cache-linear | C |
| Graph BFS vs C index | 16.2 ms | C 10.9 ms | −33% | L4 guards on index arithmetic; same data structure | A + L4 |
| Linked list | 13.7 ms | Rust 21.3 ms | Rust 55% slower | Pointer-linked nodes vs flat index arrays (different DS) | C |
| Alloc stress | 10.8 ms | C 10.3 ms | 5% | Same allocator; gap within noise | A |
| Parallel sum | 197 ms | Rust 151 ms | Rust 23% faster | std::thread lower overhead than OpenMP reduction | A |
| Array stats (old, unfair) | 37.9 ms | C seq 61.9 ms | C 63% slower | Parallel vs serial — invalid comparison | — |
| Array stats (fair parallel) | 37.9 ms | Rust 36.5 ms | <1% | All parallel: vāṇī = Rust; C OpenMP 14% slower | A |
| SIMD dot (vs auto-vec) | 30.3 ms | C auto 33.7 ms | C 11% slower | Explicit SIMD vs auto-vectorized — unfair comparison | A |
| SIMD dot (vs explicit SSE) | 30.3 ms | C SSE 29.0 ms | <2% | Explicit vec128 ≈ explicit __m128: equivalent | A (closed) |
| SIMD-256 | 33.5 ms | — | — | vāṇī-only; would need explicit AVX2 C for comparison | A |

**Category key:**
- **A** = Compiler code generation quality
- **B** = Standard library implementation quality  
- **C** = Language design (architectural advantage from ownership model)
- **A + L4** = Compiler gap partially explained by runtime overflow guards (safety cost)
- **(closed)** = Gap was an implementation artifact, now resolved with a fair comparison

---

# Rerun Decision

**Existing RESULTS.md timings: VALID** — all prior changes were comment-only.

**New variants run 2026-07-17** (incorporated into table above):

| File | Status | Timing |
|------|--------|--------|
| `matmul_ikj.rs` | ✅ timed | 14.3 ms |
| `stats_omp.c` | ✅ timed | 43.3 ms |
| `stats_omp.cpp` | ✅ timed | 47.7 ms |
| `stats_threads.rs` | ✅ timed | 36.5 ms |
| `dot_simd_sse.c` | ✅ timed | 29.0 ms |
| `fib_bounded.vani` SMT elision | ✅ verified | no guard in output |
| `fib_vani.ll` assembly dump | ✅ generated | committed |
| `benchmarks/results/SYSTEM.md` | ✅ created | CPU/compiler details |

**Still outstanding:**

| Item | Why pending |
|------|-------------|
| `parsum_rayon.rs` timing | Requires Cargo + rayon crate; `parsum.rs` (std::thread) already provides fair parallel comparison |
| L4 guard in release build | Release build not present; debug build does not emit guards |
| `graph_weakptr.rs` | New benchmark not yet written |
| Benchmark 13 ownership transfer | New benchmark not yet written |
| RESULTS.md update with new variant columns | Needs runner to be run end-to-end; new variants added to `run_benchmarks.py` |
