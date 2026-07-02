/* Benchmark 03 — 256×256 i64 matrix multiply  (C)
   gcc -O2 -o matmul_c matmul.c && ./matmul_c            */
#include <stdio.h>
#include <stdlib.h>

#define N 256

int main(void) {
    long *a = (long *)malloc(N * N * sizeof(long));
    long *b = (long *)malloc(N * N * sizeof(long));
    long *c = (long *)calloc(N * N, sizeof(long));

    for (int i = 0; i < N * N; i++) a[i] = i % 97 + 1;
    for (int i = 0; i < N * N; i++) b[i] = i % 53 + 1;

    for (int row = 0; row < N; row++) {
        for (int col = 0; col < N; col++) {
            long sum = 0;
            for (int k = 0; k < N; k++)
                sum += a[row * N + k] * b[k * N + col];
            c[row * N + col] = sum;
        }
    }

    printf("%ld\n", c[0] + c[N * N - 1]);

    free(a); free(b); free(c);
    return 0;
}
