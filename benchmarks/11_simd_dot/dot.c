/* Benchmark 11 — Dot product of two 4 000 000-element float vectors  (C)
   gcc -O3 -march=native -o dot_c dot.c && ./dot_c
   Expected output: two equal integers (truncated dot-product checksum).  */
#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>

int main(void) {
    const int64_t N = 4000000LL;
    float *a = (float *)malloc((size_t)N * sizeof(float));
    float *b = (float *)malloc((size_t)N * sizeof(float));

    for (int64_t i = 0; i < N; i++) {
        a[i] = (float)(i % 100) * 0.01f;
        b[i] = 1.0f;
    }

    /* Pass 1 — explicit SIMD via GCC auto-vectorisation (-march=native) */
    float r1 = 0.0f;
    for (int64_t i = 0; i < N; i++) r1 += a[i] * b[i];

    /* Pass 2 — same scalar loop; GCC will also vectorise this */
    float r2 = 0.0f;
    for (int64_t i = 0; i < N; i++) r2 += a[i] * b[i];

    printf("%" PRId64 "\n%" PRId64 "\n", (int64_t)r1, (int64_t)r2);
    free(a);
    free(b);
    return 0;
}
