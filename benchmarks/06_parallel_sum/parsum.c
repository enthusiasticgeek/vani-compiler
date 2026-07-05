/* Benchmark 06 — Parallel sum of 50 000 000 elements  (C + OpenMP)
   gcc -O3 -march=native -fopenmp -o parsum_c parsum.c && ./parsum_c
   (Falls back to serial if -fopenmp is unavailable.)              */
#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>

int main(void) {
    const int64_t N = 50000000LL;
    int64_t *data = (int64_t *)malloc((size_t)N * sizeof(int64_t));
    for (int64_t i = 0; i < N; i++) data[i] = i % 1000;

    int64_t total = 0;
#ifdef _OPENMP
#pragma omp parallel for reduction(+:total)
#endif
    for (int64_t i = 0; i < N; i++)
        total += data[i];

    printf("%" PRId64 "\n", total);
    free(data);
    return 0;
}
