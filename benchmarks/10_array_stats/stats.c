/* Benchmark 10 — Mean + variance of 10 000 000 values  (C)
   gcc -O3 -march=native -o stats_c stats.c && ./stats_c          */
#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>

int main(void) {
    const int64_t N = 10000000LL;
    int64_t *data = (int64_t *)malloc((size_t)N * sizeof(int64_t));

    for (int64_t i = 0; i < N; i++)
        data[i] = (i * 7 + 13) % 1000;

    /* Pass 1: sum */
    int64_t sum = 0;
    for (int64_t i = 0; i < N; i++) sum += data[i];
    int64_t mean = sum / N;

    /* Pass 2: variance */
    int64_t var_sum = 0;
    for (int64_t i = 0; i < N; i++) {
        int64_t diff = data[i] - mean;
        var_sum += diff * diff;
    }
    int64_t variance = var_sum / N;

    printf("%" PRId64 "\n%" PRId64 "\n", mean, variance);
    free(data);
    return 0;
}
