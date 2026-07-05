// Benchmark 10 — Mean + variance of 10 000 000 values  (C++)
// g++ -O3 -march=native -std=c++17 -o stats_cpp stats.cpp && ./stats_cpp
#include <cinttypes>
#include <cstdint>
#include <cstdio>
#include <vector>

int main() {
    const int64_t N = 10000000LL;
    std::vector<int64_t> data((size_t)N);
    for (int64_t i = 0; i < N; i++) data[(size_t)i] = (i * 7 + 13) % 1000;

    int64_t sum = 0;
    for (int64_t i = 0; i < N; i++) sum += data[(size_t)i];
    int64_t mean = sum / N;

    int64_t var_sum = 0;
    for (int64_t i = 0; i < N; i++) {
        int64_t diff = data[(size_t)i] - mean;
        var_sum += diff * diff;
    }
    int64_t variance = var_sum / N;

    std::printf("%" PRId64 "\n%" PRId64 "\n", mean, variance);
}
