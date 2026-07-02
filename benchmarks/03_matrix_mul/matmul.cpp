// Benchmark 03 — 256×256 i64 matrix multiply  (C++)
// g++ -O2 -std=c++17 -o matmul_cpp matmul.cpp && ./matmul_cpp
#include <cstdio>
#include <vector>

int main() {
    const int N = 256;
    std::vector<long> a(N * N), b(N * N), c(N * N, 0);

    for (int i = 0; i < N * N; i++) a[i] = i % 97 + 1;
    for (int i = 0; i < N * N; i++) b[i] = i % 53 + 1;

    for (int row = 0; row < N; row++)
        for (int col = 0; col < N; col++) {
            long sum = 0;
            for (int k = 0; k < N; k++)
                sum += a[row * N + k] * b[k * N + col];
            c[row * N + col] = sum;
        }

    std::printf("%ld\n", c[0] + c[N * N - 1]);
}
