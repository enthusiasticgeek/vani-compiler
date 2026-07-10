/* Benchmark 11 — Dot product of two 4 000 000-element float vectors  (C++)
   g++ -O3 -march=native -std=c++17 -o dot_cpp dot.cpp && ./dot_cpp       */
#include <cinttypes>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <numeric>
#include <vector>

int main() {
    const int64_t N = 4'000'000LL;
    std::vector<float> a(N), b(N);

    for (int64_t i = 0; i < N; i++) {
        a[i] = static_cast<float>(i % 100) * 0.01f;
        b[i] = 1.0f;
    }

    float r1 = 0.0f;
    for (int64_t i = 0; i < N; i++) r1 += a[i] * b[i];

    float r2 = 0.0f;
    for (int64_t i = 0; i < N; i++) r2 += a[i] * b[i];

    std::printf("%" PRId64 "\n%" PRId64 "\n",
                static_cast<int64_t>(r1), static_cast<int64_t>(r2));
    return 0;
}
