// Benchmark 07 — HashMap: 500 000 insert + 500 000 lookup  (C++)
// g++ -O2 -std=c++17 -o hash_cpp hash.cpp && ./hash_cpp
#include <cstdio>
#include <unordered_map>

int main() {
    const long N = 500000L;
    std::unordered_map<long, long> m;
    m.reserve(N * 2); // avoid rehashing

    for (long i = 0; i < N; i++)
        m[i] = i * i;

    long sum = 0;
    for (long j = 0; j < N; j++)
        sum += m[j];

    std::printf("%ld\n%ld\n", (long)m.size(), sum);
}
