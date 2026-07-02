// Benchmark 09 — Allocation stress: 500 000 struct push/read cycles  (C++)
// g++ -O2 -std=c++17 -o alloc_cpp alloc.cpp && ./alloc_cpp
#include <cstdio>
#include <vector>

struct Payload { long a, b, c; };

int main() {
    const long N = 500000L;
    std::vector<Payload> items;
    items.reserve(N);

    for (long i = 0; i < N; i++)
        items.push_back({i, i * 2, i * 3});

    long sum = 0;
    for (long j = 0; j < N; j++)
        sum += items[j].a + items[j].c;

    std::printf("%ld\n", sum);
}
