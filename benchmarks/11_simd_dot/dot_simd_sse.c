/* Benchmark 11 — Dot product: explicit SSE2/SSE4 intrinsics  (C)
   Fair explicit-vs-explicit comparison with vāṇī's vec128<f32> path.
   dot.c uses auto-vectorized scalar; this file uses _mm_* intrinsics directly.

   gcc -O3 -march=native -o dot_c_sse dot_simd_sse.c && ./dot_c_sse

   Pass 1: explicit __m128 (4×f32 SSE), matching vāṇī vec128<f32>
   Pass 2: auto-vectorized scalar (same as dot.c baseline)
   Output: two i64 checksums — must be equal. */

#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>
#include <xmmintrin.h>   /* SSE: _mm_add_ps, _mm_mul_ps, _mm_loadu_ps      */
#include <pmmintrin.h>   /* SSE3: _mm_hadd_ps (horizontal add)              */

int main(void) {
    const int64_t N = 4000000LL;

    /* malloc returns at least 8-byte aligned; loadu handles unaligned reads */
    float *a = (float *)malloc((size_t)N * sizeof(float));
    float *b = (float *)malloc((size_t)N * sizeof(float));
    if (!a || !b) { free(a); free(b); return 1; }

    for (int64_t i = 0; i < N; i++) {
        a[i] = (float)(i % 100) * 0.01f;
        b[i] = 1.0f;
    }

    /* Pass 1 — explicit SSE 128-bit (4 lanes × f32)
       Mirrors vāṇī: simd_load + simd_mul + simd_add + simd_reduce_add     */
    __m128 acc = _mm_setzero_ps();
    int64_t i = 0;
    for (; i + 4 <= N; i += 4) {
        __m128 ai = _mm_loadu_ps(&a[i]);
        __m128 bi = _mm_loadu_ps(&b[i]);
        acc = _mm_add_ps(acc, _mm_mul_ps(ai, bi));
    }
    /* Horizontal reduction: sum 4 lanes → scalar */
    acc = _mm_hadd_ps(acc, acc);   /* [a+b, c+d, a+b, c+d] */
    acc = _mm_hadd_ps(acc, acc);   /* [a+b+c+d, ...] */
    float r1 = _mm_cvtss_f32(acc);
    for (; i < N; i++) r1 += a[i] * b[i];   /* scalar tail */

    /* Pass 2 — auto-vectorized scalar (GCC baseline, same as dot.c) */
    float r2 = 0.0f;
    for (int64_t j = 0; j < N; j++) r2 += a[j] * b[j];

    printf("%" PRId64 "\n%" PRId64 "\n", (int64_t)r1, (int64_t)r2);
    free(a);
    free(b);
    return 0;
}
