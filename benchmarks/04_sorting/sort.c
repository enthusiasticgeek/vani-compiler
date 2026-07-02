/* Benchmark 04 — Sort 1 000 000 integers  (C)
   gcc -O2 -o sort_c sort.c && ./sort_c                   */
#include <stdio.h>
#include <stdlib.h>

static int cmp_long(const void *a, const void *b) {
    long x = *(const long *)a;
    long y = *(const long *)b;
    return (x > y) - (x < y);
}

int main(void) {
    const int N = 1000000;
    long *xs = (long *)malloc(N * sizeof(long));

    /* Same LCG as the vāṇī variant. */
    long seed = 12345678, a = 1664525, c_val = 1013904223, mask = 2147483647;
    for (int i = 0; i < N; i++) {
        seed = (a * seed + c_val) % mask;
        xs[i] = seed;
    }

    qsort(xs, N, sizeof(long), cmp_long);

    printf("%ld\n", xs[0] + xs[N - 1]);
    free(xs);
    return 0;
}
