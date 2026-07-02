// Benchmark 06 — Parallel sum of 50 000 000 elements  (C++ + OpenMP)
// g++ -O2 -std=c++17 -fopenmp -o parsum_cpp parsum.cpp && ./parsum_cpp
#include <cstdio>
#include <vector>

int main() {
    const long N = 50000000L;
    std::vector<long> data(N);
    for (long i = 0; i < N; i++) data[i] = i % 1000;

    long total = 0;
#ifdef _OPENMP
#pragma omp parallel for reduction(+:total)
#endif
    for (long i = 0; i < N; i++)
        total += data[i];

    std::printf("%ld\n", total);
}
