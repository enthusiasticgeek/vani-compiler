# Benchmark 12 — SIMD-256 dot product (f32)

> **STATUS: FUTURE / NOT YET RUNNABLE**
>
> This benchmark requires `vec256<f32>`, `vec128<f32>`, `Vec<f32>`, and
> the `simd256_*` / `simd_*` builtin intrinsics — none of which are
> implemented in the compiler yet. Do not attempt to run it; it will fail
> at compilation. It is kept here as a specification for when those
> features land.

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

## Prerequisites (not yet implemented)

- `Vec<f32>` collection type
- `vec256<f32>` SIMD vector type (8-lane 32-bit float)
- `vec128<f32>` SIMD vector type (4-lane 32-bit float)
- `simd256_splat`, `simd256_load`, `simd256_add`, `simd256_mul`, `simd256_reduce_add`
- `simd_splat`, `simd_load`, `simd_add`, `simd_mul`, `simd_reduce_add`
- `vec_fill(n, v: f32) -> Vec<f32>`

## Expected result (once implemented)

On x86-64 with AVX2 (e.g. Intel Ice Lake, Haswell+):
- `dot_vec256` should finish in roughly **half** the time of `dot_vec128`
  because each loop iteration processes twice as many elements.
- `dot_scalar` may approach `dot_vec128` speed if LLVM's auto-vectorizer
  produces 128-bit SSE instructions for the scalar loop; it should be
  clearly slower than `dot_vec256`.

All three checksums (truncated `f32 → i64`) should be 1000000 ± 1
due to floating-point accumulation-order variance.

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
