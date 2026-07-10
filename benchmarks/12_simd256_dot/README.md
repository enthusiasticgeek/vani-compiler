# Benchmark 12 — SIMD-256 dot product (f32)

Measures dot product throughput on two 4 000 000-element `f32` vectors,
comparing three explicit SIMD widths within vāṇी itself:

| Variant | SIMD width | Lanes / iteration | x86-64+AVX2 registers |
|---------|-----------|-------------------|-----------------------|
| `dot_vec256` | `vec256<f32>` | 8 | `ymm` (256-bit) |
| `dot_vec128` | `vec128<f32>` | 4 | `xmm` (128-bit) |
| `dot_scalar` | scalar loop | 1 | auto-vectorized by LLVM |

The benchmark is **vāṇी-only** — it is not comparing against C/C++/Rust
because the question being answered is: *does the wider SIMD type actually
reduce wall-clock time on this machine?*

## Expected result

On x86-64 with AVX2 (e.g. Intel Ice Lake, Haswell+):
- `dot_vec256` should finish in roughly **half** the time of `dot_vec128`
  because each loop iteration processes twice as many elements.
- `dot_scalar` may approach `dot_vec128` speed if LLVM's auto-vectorizer
  produces 128-bit SSE instructions for the scalar loop; it should be
  clearly slower than `dot_vec256`.

All three checksums (truncated `f32 → i64`) should be the same value or
differ by at most 1 due to floating-point accumulation-order variance.

## Run

```bash
# vāṇी LLVM backend (default)
python3 benchmarks/run_benchmarks.py --bench 12

# vāṇī only, direct
vanic run benchmarks/12_simd256_dot/dot_simd256.vani
```

To see the assembly LLVM produces for the vec256 path on x86-64:

```bash
vanic build benchmarks/12_simd256_dot/dot_simd256.vani \
    --emit-llvm -o dot_simd256.ll
llc -O3 -mcpu=native -mattr=+avx2 dot_simd256.ll -o dot_simd256.s
grep -A8 "dot_vec256" dot_simd256.s | head -20
# expect: vmovups ymm, vfmadd231ps ymm, ...
```

## AArch64 notes

On AArch64 **without** SVE, `vec256<f32>` is legalised by LLVM as two
128-bit NEON registers. You will see two `fmla v0.4s` / `fmla v1.4s`
instructions per iteration — functional but no throughput gain vs
`dot_vec128`. To get a true single-register 256-bit result, build with
`--sve` on an SVE-capable CPU (Neoverse N2, Cortex-A715, Apple M-series
does not expose SVE via QEMU — use `--cpu neoverse-n2 --sve`).

```bash
# Cross-compile for AArch64 + SVE (correctness check via QEMU)
vanic build benchmarks/12_simd256_dot/dot_simd256.vani \
    --target=aarch64-unknown-linux-gnu \
    --cpu=neoverse-n2 \
    --sve \
    -o dot256_aarch64

qemu-aarch64-static -cpu max ./dot256_aarch64
```

## RISC-V notes

On RISC-V with the Vector extension (VLEN ≥ 256), `vec256<f32>` maps to
`vsetvli t0, ..., e32, m1` with `vl = 8`. On SiFive X280 (VLEN = 256) this
is a single-pass operation; on a minimal VLEN = 128 core, LLVM emits two
passes. Either way the result is correct.

```bash
vanic build benchmarks/12_simd256_dot/dot_simd256.vani \
    --target=riscv64-unknown-linux-gnu \
    --cpu=sifive-x280 \
    -o dot256_riscv

QEMU_RISCV64="qemu-riscv64-static -cpu rv64,v=true,vlen=256" \
  qemu-riscv64-static -cpu rv64,v=true,vlen=256 ./dot256_riscv
```

## Benchmark registration

To wire this into `run_benchmarks.py`:

```python
# Add to BENCHMARKS list in run_benchmarks.py:
{
    "id": 12,
    "name": "SIMD-256 dot product (4M f32 elements)",
    "dir": "12_simd256_dot",
    "vani": "dot_simd256.vani",
    "expected": None,   # compare printed lines manually
    "langs": ["vani"],
},
```
