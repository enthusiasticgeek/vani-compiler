// Benchmark 10 — Mean + variance of 10M values  (C++ + OpenMP)
// Parallel comparison for the vani parallel-for-reduce version.
//
// g++ -O2 -std=c++17 -fopenmp -o stats_omp_cpp stats_omp.cpp && ./stats_omp_cpp

#include <cstdio>
#include <cstdlib>
#include <cstdint>
#include <vector>

int main() {
    const int64_t n = 10'000'000;
    std::vector<int64_t> data(n);

    for (int64_t i = 0; i < n; i++)
        data[i] = (i * 7 + 13) % 1000;

    // Pass 1: parallel sum.
    int64_t sum = 0;
    #pragma omp parallel for reduction(+:sum)
    for (int64_t j = 0; j < n; j++)
        sum += data[j];
    int64_t mean = sum / n;

    // Pass 2: parallel variance.
    int64_t var_sum = 0;
    #pragma omp parallel for reduction(+:var_sum)
    for (int64_t k = 0; k < n; k++) {
        int64_t diff = data[k] - mean;
        var_sum += diff * diff;
    }
    int64_t variance = var_sum / n;

    printf("%ld\n%ld\n", (long)mean, (long)variance);
    return 0;
}
