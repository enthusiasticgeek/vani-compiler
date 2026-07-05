/* Benchmark 04 — Sort 1 000 000 integers  (C)
   gcc -O3 -march=native -o sort_c sort.c && ./sort_c        */
#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>

static int cmp_i64(const void *a, const void *b) {
    int64_t x = *(const int64_t *)a;
    int64_t y = *(const int64_t *)b;
    return (x > y) - (x < y);
}

int main(void) {
    const int N = 1000000;
    int64_t *xs = (int64_t *)malloc(N * sizeof(int64_t));

    /* Same LCG as the vāṇī variant. */
    int64_t seed = 12345678, a = 1664525, c_val = 1013904223, mask = 2147483647;
    for (int i = 0; i < N; i++) {
        seed = (a * seed + c_val) % mask;
        xs[i] = seed;
    }

    qsort(xs, N, sizeof(int64_t), cmp_i64);

    printf("%" PRId64 "\n", xs[0] + xs[N - 1]);
    free(xs);
    return 0;
}
