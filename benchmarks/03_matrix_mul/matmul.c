/* Benchmark 03 — 256×256 i64 matrix multiply  (C)
   gcc -O3 -march=native -o matmul_c matmul.c && ./matmul_c  */
#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>

#define N 256

int main(void) {
    int64_t *a = (int64_t *)malloc(N * N * sizeof(int64_t));
    int64_t *b = (int64_t *)malloc(N * N * sizeof(int64_t));
    int64_t *c = (int64_t *)calloc(N * N, sizeof(int64_t));

    for (int i = 0; i < N * N; i++) a[i] = i % 97 + 1;
    for (int i = 0; i < N * N; i++) b[i] = i % 53 + 1;

    for (int row = 0; row < N; row++) {
        for (int col = 0; col < N; col++) {
            int64_t sum = 0;
            for (int k = 0; k < N; k++)
                sum += a[row * N + k] * b[k * N + col];
            c[row * N + col] = sum;
        }
    }

    printf("%" PRId64 "\n", c[0] + c[N * N - 1]);

    free(a); free(b); free(c);
    return 0;
}
