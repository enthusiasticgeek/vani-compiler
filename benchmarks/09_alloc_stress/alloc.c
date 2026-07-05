/* Benchmark 09 — Allocation stress: 500 000 struct push/read cycles  (C)
   gcc -O3 -march=native -o alloc_c alloc.c && ./alloc_c              */
#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>

typedef struct { int64_t a, b, c; } Payload;

int main(void) {
    const int64_t N = 500000LL;
    Payload *items = (Payload *)malloc((size_t)N * sizeof(Payload));

    for (int64_t i = 0; i < N; i++) {
        items[i].a = i;
        items[i].b = i * 2;
        items[i].c = i * 3;
    }

    int64_t sum = 0;
    for (int64_t j = 0; j < N; j++)
        sum += items[j].a + items[j].c;

    printf("%" PRId64 "\n", sum);
    free(items);
    return 0;
}
