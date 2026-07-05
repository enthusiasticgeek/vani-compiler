// Benchmark 07 — HashMap: 500 000 insert + 500 000 lookup  (C++)
// g++ -O3 -march=native -std=c++17 -o hash_cpp hash.cpp && ./hash_cpp
#include <cinttypes>
#include <cstdint>
#include <cstdio>
#include <unordered_map>

int main() {
    const int64_t N = 500000LL;
    std::unordered_map<int64_t, int64_t> m;
    m.reserve(N * 2); // avoid rehashing

    for (int64_t i = 0; i < N; i++)
        m[i] = i * i;

    int64_t sum = 0;
    for (int64_t j = 0; j < N; j++)
        sum += m[j];

    std::printf("%" PRId64 "\n%" PRId64 "\n", (int64_t)m.size(), sum);
}
