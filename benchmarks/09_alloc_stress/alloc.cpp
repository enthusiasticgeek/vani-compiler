// Benchmark 09 — Allocation stress: 500 000 struct push/read cycles  (C++)
// g++ -O3 -march=native -std=c++17 -o alloc_cpp alloc.cpp && ./alloc_cpp
#include <cinttypes>
#include <cstdint>
#include <cstdio>
#include <vector>

struct Payload { int64_t a, b, c; };

int main() {
    const int64_t N = 500000LL;
    std::vector<Payload> items;
    items.reserve(N);

    for (int64_t i = 0; i < N; i++)
        items.push_back({i, i * 2, i * 3});

    int64_t sum = 0;
    for (int64_t j = 0; j < N; j++)
        sum += items[j].a + items[j].c;

    std::printf("%" PRId64 "\n", sum);
}
