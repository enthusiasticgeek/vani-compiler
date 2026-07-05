// Benchmark 04 — Sort 1 000 000 integers  (C++)
// g++ -O3 -march=native -std=c++17 -o sort_cpp sort.cpp && ./sort_cpp
#include <algorithm>
#include <cinttypes>
#include <cstdint>
#include <cstdio>
#include <vector>

int main() {
    const int N = 1000000;
    std::vector<int64_t> xs(N);

    int64_t seed = 12345678, a = 1664525, c_val = 1013904223, mask = 2147483647;
    for (int i = 0; i < N; i++) {
        seed = (a * seed + c_val) % mask;
        xs[i] = seed;
    }

    std::sort(xs.begin(), xs.end());
    std::printf("%" PRId64 "\n", xs[0] + xs[N - 1]);
}
