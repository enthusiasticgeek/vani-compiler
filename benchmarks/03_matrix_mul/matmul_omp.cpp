// Benchmark 03 — 256×256 matrix multiply  (C++ + OpenMP)
// Parallel outer-row loop — fair parallel comparison for matmul.
//
// g++ -O2 -std=c++17 -fopenmp -o matmul_omp_cpp matmul_omp.cpp && ./matmul_omp_cpp

#include <cstdio>
#include <cstdlib>
#include <cstdint>
#include <vector>

int main() {
    const int64_t n = 256;
    const int64_t sz = n * n;

    std::vector<int64_t> a(sz), b(sz), c(sz, 0);

    for (int64_t i = 0; i < sz; i++) a[i] = i % 97 + 1;
    for (int64_t i = 0; i < sz; i++) b[i] = i % 53 + 1;

    // Parallel outer row loop; each thread writes a distinct row of c.
    #pragma omp parallel for schedule(static)
    for (int64_t row = 0; row < n; row++) {
        for (int64_t col = 0; col < n; col++) {
            int64_t sum = 0;
            for (int64_t m = 0; m < n; m++)
                sum += a[row * n + m] * b[m * n + col];
            c[row * n + col] = sum;
        }
    }

    printf("%ld\n", (long)(c[0] + c[sz - 1]));
    return 0;
}
