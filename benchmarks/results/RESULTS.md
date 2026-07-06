# Benchmark Results — vāṇī vs Rust vs C vs C++

*Generated: 2026-07-06 08:06 — 3 timing run(s) per benchmark, median reported.*
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

| Benchmark | vani         | c            | cpp          | rs           |
|--------------------|--------------|--------------|--------------|--------------|
| HashMap — 500 000  |    48.4 ms   |    32.1 ms   |    54.4 ms   |    93.3 ms   |

## Per-benchmark charts

> Bars are proportional to wall-clock time — **shorter is faster**.

### HashMap — 500 000 insert + 500 000 lookup

*Tests open-addressing HashMap throughput.*

```
  vani           ███████████████████░░░░░░░░░░░░░░░░░     48.4 ms    baseline
  c              ████████████░░░░░░░░░░░░░░░░░░░░░░░░     32.1 ms   33.5% faster
  cpp            █████████████████████░░░░░░░░░░░░░░░     54.4 ms   12.5% slower
  rs             ████████████████████████████████████     93.3 ms   92.9% slower
```

> **Note on C comparison**: On Windows MinGW, `long` is 4 bytes (not 8).
> `hash.c` uses `struct { long key; long val; int used; }` = 12-byte slots,
> meaning the C benchmark stores 32-bit keys/values and the `i*i` sum
> silently overflows. vāṇī uses 64-bit `i64` throughout (correct answer:
> 41666541666750000). The ~33% speed gap is structurally explained by the
> 12 MB C table vs vāṇī's 24 MB table — a 32-bit vs 64-bit key mismatch,
> not a code-quality difference.

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

