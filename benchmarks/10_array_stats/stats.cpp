// Benchmark 10 — Mean + variance of 10 000 000 values  (C++)
// g++ -O2 -std=c++17 -o stats_cpp stats.cpp && ./stats_cpp
#include <cstdio>
#include <vector>

int main() {
    const long N = 10000000L;
    std::vector<long> data(N);
    for (long i = 0; i < N; i++) data[i] = (i * 7 + 13) % 1000;

    long sum = 0;
    for (long i = 0; i < N; i++) sum += data[i];
    long mean = sum / N;

    long var_sum = 0;
    for (long i = 0; i < N; i++) {
        long diff = data[i] - mean;
        var_sum += diff * diff;
    }
    long variance = var_sum / N;

    std::printf("%ld\n%ld\n", mean, variance);
}
