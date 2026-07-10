# Benchmark 11 — SIMD dot product (f32)

Measures dot product throughput on two 4 000 000-element `f32` vectors.

**What's being compared:**

| Variant | Approach |
|---------|---------|
| vāṇī | Explicit `vec128<f32>` loads → `simd_mul` → `simd_add` → `simd_reduce_add`; falls back to scalar tail |
| C | Scalar loop — GCC auto-vectorizes with `-march=native` (SSE/AVX2/NEON) |
| C++ | Same as C via `std::inner_product`-style loop |
| Rust | Iterator chain — LLVM auto-vectorizes with `-C target-cpu=native` |

The comparison is **explicit SIMD (vāṇī vec128) vs. auto-vectorized
scalar** across languages. All four compilers have the same opportunity
to use SIMD — the question is whether the explicit vāṇī path matches
or beats what the optimizer produces automatically.

## Expected result

The benchmark prints two integer-truncated checksums (SIMD path, then
scalar path). Both should be the same or differ by at most 1 due to
floating-point accumulation order differences.

The automated runner checks **exit code 0 only** (`expected: None` in
`run_benchmarks.py`). Compare the two printed lines manually to verify
correctness; they should be in the approximate range `1980`–`1990`.

## Run

```bash
# All languages
python3 benchmarks/run_benchmarks.py --bench 11

# vāṇী only
python3 benchmarks/run_benchmarks.py --bench 11 --langs vani

# AArch64 cross-compile (requires qemu-aarch64 or real hardware)
vanic build benchmarks/11_simd_dot/dot_simd.vani \
      --target=aarch64-unknown-linux-gnu --cpu=cortex-a72 \
      -o dot_aarch64
```

## AArch64 / NEON note

On AArch64 targets, `vec128<f32>` lowers to NEON `v`-registers with
4 `float` lanes. The `simd_mul` builtin emits `fmul v0.4s, v1.4s, v2.4s`
and `simd_reduce_add` emits `faddp`/`faddv`. With `--cpu=cortex-a72`
the scheduler picks Cortex-A72 pipeline timings for better instruction
ordering.

Cross-compile and profile with QEMU (correctness only, not speed):

```bash
vanic build dot_simd.vani --target=aarch64-unknown-linux-gnu -o dot_aarch64
qemu-aarch64 -L /usr/aarch64-linux-gnu dot_aarch64
```
