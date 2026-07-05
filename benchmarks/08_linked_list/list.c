/* Benchmark 08 — Index-based linked list, 1 000 000 nodes  (C)
   Same index approach as list.vani: parallel arrays, no per-node malloc.
   gcc -O3 -march=native -o list_c list.c && ./list_c             */
#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>

int main(void) {
    const int64_t N = 1000000LL;
    int64_t *values = (int64_t *)malloc((size_t)N * sizeof(int64_t));
    int64_t *next   = (int64_t *)malloc((size_t)N * sizeof(int64_t));

    for (int64_t i = 0; i < N; i++) {
        values[i] = i % 1000;
        next[i]   = i + 1;
    }
    next[N - 1] = -1;

    int64_t sum = 0;
    for (int64_t curr = 0; curr != -1; curr = next[curr])
        sum += values[curr];

    printf("%" PRId64 "\n", sum);
    free(values);
    free(next);
    return 0;
}
