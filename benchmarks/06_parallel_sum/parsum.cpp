// Benchmark 06 — Parallel sum of 50 000 000 elements  (C++ + OpenMP)
// g++ -O3 -march=native -std=c++17 -fopenmp -o parsum_cpp parsum.cpp && ./parsum_cpp
#include <cinttypes>
#include <cstdint>
#include <cstdio>
#include <vector>

int main() {
    const int64_t N = 50000000LL;
    std::vector<int64_t> data((size_t)N);
    for (int64_t i = 0; i < N; i++) data[(size_t)i] = i % 1000;

    int64_t total = 0;
#ifdef _OPENMP
#pragma omp parallel for reduction(+:total)
#endif
    for (int64_t i = 0; i < N; i++)
        total += data[(size_t)i];

    std::printf("%" PRId64 "\n", total);
}
