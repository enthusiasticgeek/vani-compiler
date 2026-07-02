/* Benchmark 10 — Mean + variance of 10 000 000 values  (C)
   gcc -O2 -o stats_c stats.c && ./stats_c                    */
#include <stdio.h>
#include <stdlib.h>

int main(void) {
    const long N = 10000000L;
    long *data = (long *)malloc(N * sizeof(long));

    for (long i = 0; i < N; i++)
        data[i] = (i * 7 + 13) % 1000;

    /* Pass 1: sum */
    long sum = 0;
    for (long i = 0; i < N; i++) sum += data[i];
    long mean = sum / N;

    /* Pass 2: variance */
    long var_sum = 0;
    for (long i = 0; i < N; i++) {
        long diff = data[i] - mean;
        var_sum += diff * diff;
    }
    long variance = var_sum / N;

    printf("%ld\n%ld\n", mean, variance);
    free(data);
    return 0;
}
