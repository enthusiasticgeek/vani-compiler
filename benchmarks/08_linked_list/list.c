/* Benchmark 08 — Index-based linked list, 1 000 000 nodes  (C)
   Same index approach as list.vani: parallel arrays, no per-node malloc.
   gcc -O2 -o list_c list.c && ./list_c                         */
#include <stdio.h>
#include <stdlib.h>

int main(void) {
    const long N = 1000000L;
    long *values = (long *)malloc(N * sizeof(long));
    long *next   = (long *)malloc(N * sizeof(long));

    for (long i = 0; i < N; i++) {
        values[i] = i % 1000;
        next[i]   = i + 1;
    }
    next[N - 1] = -1;

    long sum = 0;
    for (long curr = 0; curr != -1; curr = next[curr])
        sum += values[curr];

    printf("%ld\n", sum);
    free(values);
    free(next);
    return 0;
}
