/* Benchmark 02 — Sieve of Eratosthenes ≤ 2 000 000  (C)
   gcc -O2 -o sieve_c sieve.c && ./sieve_c
   Expected: 148933                                         */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main(void) {
    const int LIMIT = 2000000;
    char *sieve = (char *)malloc(LIMIT + 1);
    memset(sieve, 1, LIMIT + 1);
    sieve[0] = sieve[1] = 0;

    for (int i = 2; (long)i * i <= LIMIT; i++) {
        if (sieve[i]) {
            for (int j = i * i; j <= LIMIT; j += i)
                sieve[j] = 0;
        }
    }

    int count = 0;
    for (int i = 2; i <= LIMIT; i++)
        if (sieve[i]) count++;

    printf("%d\n", count);
    free(sieve);
    return 0;
}
