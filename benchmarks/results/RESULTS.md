# Benchmark Results — vāṇī vs Rust vs C vs C++

*Generated: 2026-07-13 16:30 — 3 timing run(s) per benchmark, median reported.*
*C/C++ flags: `-O3 -march=native`. Rust flags: `-C opt-level=3 -C target-cpu=native`.*
*vāṇī uses LLVM backend with `opt -O3 --mcpu=native` + `llc -O3 -mcpu=native`.*

## System
```
OS       : Windows 11 AMD64
Python   : 3.14.5
vanic    : C:/Users/upaas/vani-compiler/target/release/vanic.exe
CC       : C:\msys64\mingw64\bin/gcc.EXE
CXX      : C:\msys64\mingw64\bin/g++.EXE
rustc    : C:\Users\upaas\.cargo\bin/rustc.EXE
```

## Summary

| Benchmark | vani         |
|--------------------|--------------|
| SIMD-256 dot produ |    28.2 ms   |

## Per-benchmark charts

> Bars are proportional to wall-clock time — **shorter is faster**.

### SIMD-256 dot product — vec256<f32> vs vec128<f32> vs scalar (4 M elements)

*vani-only: vec256 (ymm/SVE) vs vec128 (xmm/NEON) vs auto-vectorized scalar.*

```
  vani           ████████████████████████████████████     28.2 ms    baseline
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

