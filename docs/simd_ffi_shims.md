# Using NEON / SSE / AVX intrinsics via FFI shims

> vāṇī v0.2.4+. Requires `--link-with=<shim.o>` and a C compiler with the
> target's SIMD headers.

vāṇī's auto-vectorizer (via LLVM) handles most numeric loops without any
user action. When you need **explicit control** over specific SIMD
instructions — cryptography, image processing, DSP, hand-tuned dot products —
the FFI shim pattern lets you call any platform intrinsic from vāṇī code
today, without waiting for native SIMD types (planned for a future Arc).

---

## How it works

1. Write a C shim file that wraps the intrinsic(s) you need.
2. Compile it to an object file with your C compiler.
3. Declare the shim function with `extern "C" fn` in your vāṇī source.
4. Link via `vanic build --link-with=simd_shim.o`.

The shim boundary is a plain C-ABI call — vāṇī passes scalars or pointers,
the shim does the SIMD work, and returns a scalar result. No special types
cross the boundary.

---

## AArch64 / NEON example — horizontal sum of i64 array

**`neon_shim.c`**
```c
#include <arm_neon.h>
#include <stdint.h>

// Sums n int64 values using NEON vaddq_s64.
// Called from vāṇī as: neon_sum(ref data, n)
int64_t neon_sum(const int64_t *data, int64_t n) {
    int64x2_t acc = vdupq_n_s64(0);
    int64_t i = 0;
    for (; i + 1 < n; i += 2) {
        int64x2_t v = vld1q_s64(data + i);
        acc = vaddq_s64(acc, v);
    }
    int64_t result = vgetq_lane_s64(acc, 0) + vgetq_lane_s64(acc, 1);
    // handle odd tail
    if (i < n) result += data[i];
    return result;
}
```

**Compile the shim** (on AArch64 or cross-compile):
```bash
aarch64-linux-gnu-gcc -O2 -march=armv8-a+simd -c neon_shim.c -o neon_shim.o
```

**`sum.vani`**
```vani
extern "C" fn neon_sum(data: ref Vec<i64>, n: i64) -> i64;

fn main() -> i64 {
    let data: Vec<i64> = vec_fill(1024, 1 as i64);
    let result: i64 = neon_sum(ref data, len(data) as i64);
    print result;
    return 0;
}
```

**Build and run:**
```bash
vanic build sum.vani \
    --target=aarch64-unknown-linux-gnu \
    --cpu=cortex-a72 \
    --link-with=neon_shim.o \
    -o sum
```

---

## x86-64 / AVX2 example — horizontal sum of i64 array

**`avx2_shim.c`**
```c
#include <immintrin.h>
#include <stdint.h>

int64_t avx2_sum(const int64_t *data, int64_t n) {
    __m256i acc = _mm256_setzero_si256();
    int64_t i = 0;
    for (; i + 3 < n; i += 4) {
        __m256i v = _mm256_loadu_si256((const __m256i *)(data + i));
        acc = _mm256_add_epi64(acc, v);
    }
    // horizontal reduce
    __m128i lo = _mm256_castsi256_si128(acc);
    __m128i hi = _mm256_extracti128_si256(acc, 1);
    __m128i sum128 = _mm_add_epi64(lo, hi);
    int64_t result = _mm_extract_epi64(sum128, 0) + _mm_extract_epi64(sum128, 1);
    // scalar tail
    for (; i < n; i++) result += data[i];
    return result;
}
```

**Compile:**
```bash
gcc -O2 -mavx2 -c avx2_shim.c -o avx2_shim.o
```

**`sum.vani`**
```vani
extern "C" fn avx2_sum(data: ref Vec<i64>, n: i64) -> i64;

fn main() -> i64 {
    let data: Vec<i64> = vec_fill(1024, 1 as i64);
    let result: i64 = avx2_sum(ref data, len(data) as i64);
    print result;
    return 0;
}
```

**Build:**
```bash
vanic build sum.vani --link-with=avx2_shim.o -o sum
```

---

## Passing data across the boundary

| vāṇī type | C type to receive it |
|-----------|---------------------|
| `ref Vec<i64>` | `const int64_t *` (pointer to data array) |
| `mut ref Vec<i64>` | `int64_t *` |
| `i64` / `u64` | `int64_t` / `uint64_t` |
| `i32` / `u32` | `int32_t` / `uint32_t` |
| `i8` / `u8` | `int8_t` / `uint8_t` |
| `f32` / `f64` | `float` / `double` |

> **Note:** vāṇī's `Vec<T>` is a fat pointer `{ data*, len, cap }`. When you
> pass `ref Vec<T>`, the C shim receives only the **data pointer** (the first
> field). The `len` must be passed separately as an `i64` parameter (as shown
> in the examples above).

---

## When to use `vec128<T>` vs a shim vs auto-vectorization

| Situation | Recommended approach |
|-----------|---------------------|
| Loop over array, LLVM can prove no aliasing | Auto-vectorization (already on by default) |
| Latency-bound loop, want software pipelining | `#[vectorize]` attribute on the function |
| Portable SIMD: splat / add / mul / reduce on f32 or i32 | `vec128<T>` / `vec256<T>` / `vec512<T>` builtins |
| Need a specific NEON intrinsic (e.g. `vsqrtq_f64`, `vrsqrteq_f32`) | FFI shim |
| Need a specific RVV intrinsic (e.g. `vfmacc_vv_f32m8`) | FFI shim with `<riscv_vector.h>` |
| AES-NI, SHA extensions, SVE gather-scatter | FFI shim |
| Hand-tuned crypto / image kernel | FFI shim |
| Multi-platform code, let LLVM choose | Auto-vectorization |

---

## Combining `#[vectorize]` with a shim

The two are orthogonal — use `#[vectorize]` on the vāṇī wrapper that
prepares/reduces data, and call the shim for the compute-intensive kernel:

```vani
extern "C" fn neon_matmul(a: ref Vec<i64>, b: ref Vec<i64>,
                           out: mut ref Vec<i64>, n: i64) -> i64;

#[vectorize]
fn prepare_and_multiply(a: ref Vec<i64>, b: ref Vec<i64>,
                         out: mut ref Vec<i64>) -> i64 {
    let n: i64 = len(a) as i64;
    // pre-processing loop — auto-vectorized + interleaved by #[vectorize]
    let i: i64 = 0;
    while i < n {
        let _ = set(out, i as u64, 0 as i64);
        i = i + 1;
    }
    // heavy kernel via NEON shim
    return neon_matmul(ref a, ref b, mut ref out, n);
}
```

---

## Native SIMD types (`vec128<T>` v0.2.4, `vec256<T>` 2026-07-10, `vec512<T>` v0.5.0)

`vec128<T>` (128-bit), `vec256<T>` (256-bit), and `vec512<T>` (512-bit) are
all live, each with the same seven-builtin set (`simd<N>_splat`, `_load`,
`_store`, `_add`, `_sub`, `_mul`, `_reduce_add` -- `simd_*` for vec128,
`simd256_*`/`simd512_*` for the wider types). FFI shims are now the
**exotic intrinsics escape hatch** rather than the primary mechanism, at
every width.

| Builtin (vec128 shown; vec256/vec512 identical shape) | x86-64 (i32 example) | AArch64 (i32) | RISC-V (i32, LLVM-lowered) |
|---------|----------------------|--------------|---------------------------|
| `simd_splat(x)` | `_mm_set1_epi32` | `dup v0.4s, w0` | `vmv.v.x` |
| `simd_add(a, b)` | `_mm_add_epi32` | `add v0.4s, v1.4s, v2.4s` | `vadd.vv` |
| `simd_mul(a, b)` | `_mm_mullo_epi32` | `mul v0.4s, v1.4s, v2.4s` | `vmul.vv` |
| `simd_sub(a, b)` | `_mm_sub_epi32` | `sub v0.4s, v1.4s, v2.4s` | `vsub.vv` |
| `simd_reduce_add(v)` | phased `_mm_hadd_*` | `addv s0, v1.4s` | `vredsum.vs` |
| `simd_load(vec, i)` | GEP + load | `ldr q0, [x0, x1, lsl #2]` | `vle32.v` |
| `simd_store(vec, i, d)` | GEP + store | `str q0, [x0, x1, lsl #2]` | `vse32.v` |

`vec256<T>`/`vec512<T>` lower to the same LLVM `<N x T>` shape with N
doubled/quadrupled -- on hardware narrower than the type (e.g. `vec512` on
plain NEON, or `vec256` on RVV with VLEN=128), LLVM legalises it into
multiple registers/vector-register-groups automatically; on hardware that
matches or exceeds the type's width (AVX-512 zmm, SVE-512, RVV VLEN>=512),
it's a single instruction. See `docs/arm_neon_status.md` and
`tutorials/src/advanced/05_simd.md` (Layer 5) for the full per-target
lowering tables.

Shims remain the escape hatch for: AES-NI, SHA extensions, Poly1305
acceleration, SVE scatter-gather, RVV widening multiply (`vwmul.vv`),
AVX-512 masking, and any intrinsic not yet in the builtin set -- this is
true at every width; `vec512<T>` does not add AVX-512 masking or SVE
gather-scatter support, only the four arithmetic ops + splat/load/store/
reduce.

For QEMU setup to test these on AArch64 and RISC-V targets without real
hardware, see `docs/qemu_testing.md`.
