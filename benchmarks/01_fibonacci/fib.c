/* Benchmark 01 — Fibonacci(42) recursive  (C)
   gcc -O2 -o fib_c fib.c && ./fib_c
   Expected: 267914296                                      */
#include <stdio.h>

static long fib(long n) {
    if (n <= 1) return n;
    return fib(n - 1) + fib(n - 2);
}

int main(void) {
    printf("%ld\n", fib(42));
    return 0;
}
