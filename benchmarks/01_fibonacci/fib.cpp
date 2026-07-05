// Benchmark 01 — Fibonacci(42) recursive  (C++)
// g++ -O3 -march=native -std=c++17 -o fib_cpp fib.cpp && ./fib_cpp
// Expected: 267914296
#include <cinttypes>
#include <cstdint>
#include <cstdio>

static int64_t fib(int64_t n) {
    if (n <= 1) return n;
    return fib(n - 1) + fib(n - 2);
}

int main() {
    std::printf("%" PRId64 "\n", fib(42));
}
