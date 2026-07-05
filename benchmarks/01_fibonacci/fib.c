/* Benchmark 01 — Fibonacci(42) recursive  (C)
   gcc -O3 -march=native -o fib_c fib.c && ./fib_c
   Expected: 267914296                                      */
#include <inttypes.h>
#include <stdio.h>

static int64_t fib(int64_t n) {
    if (n <= 1) return n;
    return fib(n - 1) + fib(n - 2);
}

int main(void) {
    printf("%" PRId64 "\n", fib(42));
    return 0;
}
