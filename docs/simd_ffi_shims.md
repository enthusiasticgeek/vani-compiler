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

## When to use `#[vectorize]` vs a shim

| Situation | Recommended approach |
|-----------|---------------------|
| Loop over array, LLVM can prove no aliasing | Auto-vectorization (already on by default) |
| Latency-bound loop, want software pipelining | `#[vectorize]` attribute on the function |
| Need a specific NEON / SSE intrinsic (e.g. `vsqrtq_f64`) | FFI shim |
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

## Future: native SIMD types

A future Arc will add `vec128<i32>`, `vec256<f64>`, and a builtin SIMD
surface (`simd_add`, `simd_load`, `simd_store`, etc.) directly to the vāṇī
type system. When that ships, FFI shims become the "exotic intrinsics" escape
hatch (e.g. AES-NI, SHA extensions, SVE gather-scatter) rather than the
primary SIMD mechanism.
