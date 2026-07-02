// Benchmark 03 — 256×256 matrix multiply  (C + OpenMP)
// Parallel outer-row loop — fair comparison for a hypothetical
// parallel vani matmul once row-slice parallelism is supported.
//
// gcc -O2 -fopenmp -o matmul_omp_c matmul_omp.c && ./matmul_omp_c

#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>

int main(void) {
    const int64_t n = 256;
    const int64_t sz = n * n;

    int64_t* a = (int64_t*)malloc(sz * sizeof(int64_t));
    int64_t* b = (int64_t*)malloc(sz * sizeof(int64_t));
    int64_t* c = (int64_t*)calloc(sz, sizeof(int64_t));
    if (!a || !b || !c) abort();

    for (int64_t i = 0; i < sz; i++) a[i] = i % 97 + 1;
    for (int64_t i = 0; i < sz; i++) b[i] = i % 53 + 1;

    // Parallel outer row loop; each thread writes to a distinct row of c.
    #pragma omp parallel for schedule(static)
    for (int64_t row = 0; row < n; row++) {
        for (int64_t col = 0; col < n; col++) {
            int64_t sum = 0;
            for (int64_t m = 0; m < n; m++)
                sum += a[row * n + m] * b[m * n + col];
            c[row * n + col] = sum;
        }
    }

    printf("%ld\n", (long)(c[0] + c[sz - 1]));
    free(a); free(b); free(c);
    return 0;
}
