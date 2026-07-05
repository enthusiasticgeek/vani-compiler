// Benchmark 02 — Sieve of Eratosthenes ≤ 2 000 000  (C++)
// g++ -O3 -march=native -std=c++17 -o sieve_cpp sieve.cpp && ./sieve_cpp
// Expected: 148933
#include <cstdio>
#include <vector>

int main() {
    const int LIMIT = 2000000;
    std::vector<bool> sieve(LIMIT + 1, true);
    sieve[0] = sieve[1] = false;

    for (int i = 2; (long)i * i <= LIMIT; i++) {
        if (sieve[i])
            for (int j = i * i; j <= LIMIT; j += i)
                sieve[j] = false;
    }

    int count = 0;
    for (int i = 2; i <= LIMIT; i++)
        if (sieve[i]) count++;

    std::printf("%d\n", count);
}
