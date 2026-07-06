/* Benchmark 07 — HashMap: 500 000 insert + 500 000 lookup  (C)
   Open-addressing hash table with FNV-1a hashing.
   gcc -O2 -o hash_c hash.c && ./hash_c                        */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <inttypes.h>

#define CAP_INIT  (1 << 20)   /* must be power-of-2 */

typedef struct { int64_t key; int64_t val; int used; } Slot;

static Slot  *table;
static size_t cap;
static size_t count;

static uint64_t fnv1a(int64_t key) {
    uint64_t h = 14695981039346656037ULL;
    unsigned char *p = (unsigned char *)&key;
    for (int i = 0; i < 8; i++) {
        h ^= p[i];
        h *= 1099511628211ULL;
    }
    return h;
}

static void ht_init(size_t initial_cap) {
    cap   = initial_cap;
    count = 0;
    table = (Slot *)calloc(cap, sizeof(Slot));
}

static void ht_insert(int64_t key, int64_t val) {
    size_t idx = (size_t)fnv1a(key) & (cap - 1);
    while (table[idx].used && table[idx].key != key)
        idx = (idx + 1) & (cap - 1);
    if (!table[idx].used) count++;
    table[idx].key  = key;
    table[idx].val  = val;
    table[idx].used = 1;
}

static int64_t ht_get(int64_t key, int64_t def) {
    size_t idx = (size_t)fnv1a(key) & (cap - 1);
    while (table[idx].used) {
        if (table[idx].key == key) return table[idx].val;
        idx = (idx + 1) & (cap - 1);
    }
    return def;
}

int main(void) {
    const int64_t N = 500000;
    ht_init(CAP_INIT);

    for (int64_t i = 0; i < N; i++)
        ht_insert(i, i * i);

    int64_t sum = 0;
    for (int64_t j = 0; j < N; j++)
        sum += ht_get(j, 0);

    printf("%zu\n%" PRId64 "\n", count, sum);
    free(table);
    return 0;
}
