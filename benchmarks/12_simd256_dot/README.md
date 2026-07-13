# Benchmark 12 — SIMD-256 dot product (f32)

Measures dot product throughput on two 4 000 000-element `f32` vectors,
comparing three explicit SIMD widths within vāṇī itself:

| Variant | SIMD width | Lanes / iteration | x86-64+AVX2 registers |
|---------|-----------|-------------------|-----------------------|
| `dot_vec256` | `vec256<f32>` | 8 | `ymm` (256-bit) |
| `dot_vec128` | `vec128<f32>` | 4 | `xmm` (128-bit) |
| `dot_scalar` | scalar loop | 1 | auto-vectorized by LLVM |

The benchmark is **vāṇī-only** — it is not comparing against C/C++/Rust
because the question being answered is: *does the wider SIMD type actually
reduce wall-clock time on this machine?*

## Run

```bash
vanic build benchmarks/12_simd256_dot/dot_simd256.vani -o dot256
./dot256   # prints three i64 checksums; all should be 1000000 ± 1

# With native AVX2 target-feature tuning:
vanic build benchmarks/12_simd256_dot/dot_simd256.vani --cpu=native -o dot256_native
```

To register in the automated runner, see [run_benchmarks.py](../run_benchmarks.py)
(currently commented out pending `expected` value stabilisation across machines).

## Results — Intel Core i5-1035G1 (Ice Lake, AVX2, 4C/8T, 1.0 GHz base / 3.6 GHz boost)

**Date:** 2026-07-13 · **OS:** Windows 11 · **Backend:** LLVM (default)  
**Working set:** 2 × 4 M × 4 B = 32 MB (exceeds 6 MB L3 → memory-bandwidth bound)

| Variant | Median (ms) | Min (ms) | Checksum |
|---------|------------|---------|----------|
| `dot_vec256` (default) | 20 | 18 | 1000000 |
| `dot_vec128` (default) | 18 | 17 | 1000000 |
| `dot_scalar` (default) | 16 | 16 | 1000000 |
| `dot_vec256` (`--cpu=native`) | 17 | 16 | 1000000 |

**Interpretation:** All three variants converge at ~16–20 ms because the 32 MB
working set exceeds the L3 cache, making every run DRAM-bandwidth bound.
LLVM's auto-vectorizer already emits AVX2 for the scalar loop, which is why
`dot_scalar` matches or beats the explicit variants. To see explicit SIMD pull
ahead, use a dataset that fits in L2/L3 (e.g. `n = 200_000`).

## AArch64 notes

On AArch64 **without** SVE, `vec256<f32>` is legalised by LLVM as two
128-bit NEON registers. You will see two `fmla v0.4s` / `fmla v1.4s`
instructions per iteration — functional but no throughput gain vs
`dot_vec128`. To get a true single-register 256-bit result, build with
`--sve` on an SVE-capable CPU.

## RISC-V notes

On RISC-V with the Vector extension (VLEN ≥ 256), `vec256<f32>` maps to
`vsetvli t0, ..., e32, m1` with `vl = 8`. On SiFive X280 (VLEN = 256)
this is a single-pass operation.
