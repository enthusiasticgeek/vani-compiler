# Benchmark System Configuration

*For reproducibility — publish this alongside RESULTS.md.*

## Hardware

```
CPU model  : Intel(R) Core(TM) i5-1035G1 CPU @ 1.00GHz (Ice Lake, 10th gen)
Base clock : 1.0 GHz  (boost up to ~3.6 GHz under sustained single-thread load)
Cores      : 4 physical / 8 logical (Hyper-Threading)
L1 cache   : 32 KB I + 48 KB D per core
L2 cache   : 512 KB per core
L3 cache   : 6 MB shared (reported as 6144 KB by WMI)
RAM        : 8 GB DDR4 dual-channel
```

## Software

```
OS         : Windows 11 Home 10.0.26200
Shell      : PowerShell 5.1 / MSYS2 bash
gcc        : 16.1.0  (MSYS2 MinGW-w64, Rev5)
g++        : 16.1.0  (same)
rustc      : 1.96.0  (ac68faa20 2026-05-25)
vanic      : debug build (target/debug/vanic.exe); release build used for RESULTS.md
```

## Flags

```
C / C++    : -O3 -march=native  (+ -fopenmp for parallel variants)
Rust       : -C opt-level=3 -C target-cpu=native
vāṇī       : LLVM backend, opt -O3 --mcpu=native + llc -O3 -mcpu=native
```

## Notes

- All timings are median of 5 wall-clock runs via PowerShell Stopwatch.
- First run discarded (OS/DLL cold-start on Windows).
- Benchmark machine is a laptop; clock frequency varies with thermal state.
  Results within a session are internally consistent but may differ by ±10%
  across sessions or machines.
- i5-1035G1 supports AVX2 (256-bit YMM) and FMA3 — relevant for matmul
  and SIMD benchmarks.
- OpenMP on Windows (MSYS2 MinGW): uses pthreads-based OpenMP (libgomp).
  `OMP_NUM_THREADS` defaults to 8 (logical core count).
