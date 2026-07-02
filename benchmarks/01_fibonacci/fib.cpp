// Benchmark 01 — Fibonacci(42) recursive  (C++)
// g++ -O2 -std=c++17 -o fib_cpp fib.cpp && ./fib_cpp
// Expected: 267914296
#include <cstdio>

static long fib(long n) {
    if (n <= 1) return n;
    return fib(n - 1) + fib(n - 2);
}

int main() {
    std::printf("%ld\n", fib(42));
}
