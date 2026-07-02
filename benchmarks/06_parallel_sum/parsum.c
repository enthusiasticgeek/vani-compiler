/* Benchmark 06 — Parallel sum of 50 000 000 elements  (C + OpenMP)
   gcc -O2 -fopenmp -o parsum_c parsum.c && ./parsum_c
   (Falls back to serial if -fopenmp is unavailable.)              */
#include <stdio.h>
#include <stdlib.h>

int main(void) {
    const long N = 50000000L;
    long *data = (long *)malloc(N * sizeof(long));
    for (long i = 0; i < N; i++) data[i] = i % 1000;

    long total = 0;
#ifdef _OPENMP
#pragma omp parallel for reduction(+:total)
#endif
    for (long i = 0; i < N; i++)
        total += data[i];

    printf("%ld\n", total);
    free(data);
    return 0;
}
