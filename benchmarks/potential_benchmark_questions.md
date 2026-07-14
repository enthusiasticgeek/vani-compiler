# Benchmark Review Questions for Vani

Use this checklist before publishing benchmark results or comparing Vani against other languages.

---

# 1. Benchmark Fairness

## Algorithms

* [ ] Is every implementation using the exact same algorithm?
* [ ] Is the asymptotic complexity identical?
* [ ] Are any language-specific optimizations changing the algorithm?
* [ ] Is any benchmark comparing different data structures rather than different languages?
* [ ] Does every implementation produce identical output?

---

## Compiler Flags

* [ ] Are all languages compiled with equivalent optimization levels?
* [ ] Are CPU-specific optimizations enabled equally?
* [ ] Are link-time optimizations either enabled or disabled consistently?
* [ ] Are debug checks disabled in release builds?

---

## Runtime Environment

* [ ] Same operating system?
* [ ] Same compiler versions?
* [ ] Same LLVM version where applicable?
* [ ] Same CPU frequency?
* [ ] Hyper-threading status documented?
* [ ] CPU governor fixed?
* [ ] Turbo Boost documented?
* [ ] Same number of benchmark iterations?

---

# 2. Code Generation

Ask for every benchmark:

* [ ] Is LLVM generating similar assembly?
* [ ] Are vector instructions emitted?
* [ ] Are unnecessary bounds checks eliminated?
* [ ] Is loop unrolling occurring?
* [ ] Is dead-code elimination affecting results?
* [ ] Is inlining equivalent?

---

# 3. Library Comparisons

When comparing library performance:

* [ ] Is the comparison really about the language?
* [ ] Or is it comparing standard library implementations?
* [ ] Which sorting algorithm is used?
* [ ] Which hash table implementation is used?
* [ ] Which allocator is being used?
* [ ] Is memory preallocated equally?

---

# 4. Data Structure Fairness

Examples:

## Graphs

* [ ] Pointer graph?
* [ ] Index graph?
* [ ] CSR?
* [ ] Adjacency list?

Question:

> Am I comparing languages or graph representations?

---

## Linked Lists

* [ ] Pointer linked list?
* [ ] Index linked list?

Question:

> Is the benchmark demonstrating a language feature or a better data structure?

---

# 5. Parallel Benchmarks

* [ ] Are all languages using parallel implementations?
* [ ] Same number of worker threads?
* [ ] Same scheduling strategy?
* [ ] Same reduction algorithm?
* [ ] Same synchronization primitives?

Question:

> Am I comparing serial code against parallel code?

---

# 6. SIMD Benchmarks

* [ ] Auto-vectorization?
* [ ] Explicit SIMD?
* [ ] Same vector width?
* [ ] Same instruction set (AVX2, AVX-512, NEON, SVE)?
* [ ] Memory alignment documented?

---

# 7. Memory Allocation

Questions:

* [ ] Which allocator?
* [ ] malloc?
* [ ] jemalloc?
* [ ] mimalloc?
* [ ] tcmalloc?
* [ ] Language runtime allocator?

Also ask:

* [ ] Number of allocations?
* [ ] Allocation size?
* [ ] Object lifetime?

---

# 8. Cache Effects

Questions:

* [ ] Is the workload cache-friendly?
* [ ] Sequential access?
* [ ] Random access?
* [ ] Pointer chasing?
* [ ] NUMA effects?
* [ ] Working set size?

---

# 9. Safety Costs

Questions:

* [ ] Bounds checking?
* [ ] Overflow checking?
* [ ] Reference counting?
* [ ] Atomic operations?
* [ ] Borrow checking (compile-time only)?
* [ ] Runtime ownership costs?

---

# 10. What Is Actually Being Measured?

For every benchmark ask:

* [ ] Compiler quality?
* [ ] Standard library quality?
* [ ] Runtime quality?
* [ ] Memory allocator?
* [ ] Data structure?
* [ ] Language design?
* [ ] Programmer ergonomics?

---

# 11. Architecture Questions

These are often more valuable than raw timings.

Examples:

* Why does Vani encourage this representation?
* Does affine ownership eliminate runtime overhead?
* Can this benchmark exist without `unsafe`?
* Can users accidentally write a slower representation?
* Is the fast implementation the idiomatic one?

---

# 12. Reviewer Questions

Expect reviewers to ask:

* Why is Rust slower here?
* Why is C faster here?
* Are these equivalent algorithms?
* Can you show the generated assembly?
* Can you publish the benchmark source?
* Can the results be reproduced?
* Are warm-up runs discarded?
* Are medians reported?
* What is the variance?
* What hardware was used?
* Which compiler versions were used?

---

# 13. Reproducibility Checklist

* [ ] Publish benchmark source code.
* [ ] Publish compiler versions.
* [ ] Publish CPU model.
* [ ] Publish RAM configuration.
* [ ] Publish operating system.
* [ ] Publish compiler flags.
* [ ] Publish benchmark harness.
* [ ] Publish raw timing data.
* [ ] Publish generated assembly (optional but recommended).

---

# 14. Language Design Questions

Instead of asking:

> Is Vani faster?

Ask:

* Why is Vani fast?
* Which language features enable this?
* Which runtime costs are avoided?
* Which bugs are prevented?
* Which optimizations become possible?
* Which APIs become simpler?
* Which unsafe code disappears?

---

# 15. The Most Important Question

For every benchmark, ask yourself:

> **Am I demonstrating that Vani is a faster compiler, a better standard library, or a better language design?**

Keeping those three categories separate makes benchmark results easier to interpret and strengthens the credibility of any performance claims.
