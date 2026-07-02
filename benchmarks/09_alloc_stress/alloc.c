/* Benchmark 09 — Allocation stress: 500 000 struct push/read cycles  (C)
   gcc -O2 -o alloc_c alloc.c && ./alloc_c                            */
#include <stdio.h>
#include <stdlib.h>

typedef struct { long a, b, c; } Payload;

int main(void) {
    const long N = 500000L;
    Payload *items = (Payload *)malloc(N * sizeof(Payload));

    for (long i = 0; i < N; i++) {
        items[i].a = i;
        items[i].b = i * 2;
        items[i].c = i * 3;
    }

    long sum = 0;
    for (long j = 0; j < N; j++)
        sum += items[j].a + items[j].c;

    printf("%ld\n", sum);
    free(items);
    return 0;
}
