/* vani_sort_runtime.c — pdqsort for vāṇī's LLVM backend.
 *
 * Compiled with GCC -O3 -march=native and linked into every LLVM-backend
 * binary.  Function names match `declare` statements in backend_llvm.rs.
 *
 * Algorithm: pattern-defeating quicksort (pdqsort) inspired by Orson Peters
 * (2015), adapted to C:
 *   - Insertion sort for n <= ISORT (24)
 *   - Heapsort fallback at depth > 2*log2(n)     → O(n log n) worst case
 *   - Tukey ninther pivot for n >= NINTHER (128)  → better pivot quality
 *   - Branchless block partition for n >= 2*BLOCK → eliminates branch
 *     mispredictions in the hot inner loop (the key pdqsort innovation)
 *   - Hoare partition for smaller sub-arrays
 *
 * Pattern-detection passes (these handle structured inputs in O(n)):
 *   - Ascending-sorted check in _recurse: if [lo,hi] is already sorted,
 *     return immediately without pivot selection or partitioning.  Costs
 *     ~2 comparisons on random data (scan terminates at first out-of-order
 *     pair, expected at position 1 for uniform random).
 *   - Reverse-sorted check at entry: if the full array is non-ascending,
 *     reverse it in O(n) and return.  Handles reverse-sorted input without
 *     entering the sort loop at all.
 *
 * The branchless block partition scans 64 elements at a time from each end,
 * collecting swap candidates into offset arrays WITHOUT branches:
 *   offs[cnt] = i;  cnt += (a[i] >= pivot);   // branchless conditional store
 * GCC -O3 lowers this to CMOV / SETCC instructions, eliminating ~50% of
 * branch mispredictions that dominate the cost of classic Hoare partition
 * on random data.
 */
#include <stdint.h>
#include <stddef.h>

typedef struct { int64_t *data; int64_t len; int64_t cap; } VecI64;
typedef struct { double  *data; int64_t len; int64_t cap; } VecF64;

#define ISORT    24
#define BLOCK    64
#define NINTHER  128

/* ================================================================
 * Generic helpers emitted twice: once for i64, once for double.
 * We use a macro to avoid code duplication.
 * ================================================================ */

#define DEFINE_SORT(T, prefix)                                              \
                                                                            \
static inline void prefix##_swap(T *a, T *b)                               \
    { T t = *a; *a = *b; *b = t; }                                         \
                                                                            \
static void prefix##_insort(T *lo, T *hi) {                                \
    for (T *i = lo + 1; i <= hi; i++) {                                     \
        T v = *i; T *j = i;                                                 \
        for (; j > lo && *(j-1) > v; j--) *j = *(j-1);                    \
        *j = v;                                                             \
    }                                                                       \
}                                                                           \
                                                                            \
static void prefix##_sift(T *a, ptrdiff_t i, ptrdiff_t n) {               \
    T v = a[i];                                                             \
    for (;;) {                                                              \
        ptrdiff_t c = 2*i+1;                                               \
        if (c+1 < n && a[c] < a[c+1]) c++;                                \
        if (c >= n || a[c] <= v) break;                                    \
        a[i] = a[c]; i = c;                                                \
    }                                                                       \
    a[i] = v;                                                               \
}                                                                           \
                                                                            \
static void prefix##_heapsort(T *a, ptrdiff_t n) {                        \
    for (ptrdiff_t i = n/2-1; i >= 0; i--) prefix##_sift(a, i, n);       \
    for (ptrdiff_t i = n-1; i > 0; i--) {                                  \
        prefix##_swap(&a[0], &a[i]);                                        \
        prefix##_sift(a, 0, i);                                             \
    }                                                                       \
}                                                                           \
                                                                            \
/* Sort a, b, c in-place; median ends at *b. */                            \
static inline void prefix##_med3(T *a, T *b, T *c) {                      \
    if (*a > *b) prefix##_swap(a, b);                                       \
    if (*b > *c) { prefix##_swap(b, c); if (*a > *b) prefix##_swap(a, b); } \
}                                                                           \
                                                                            \
/* Pivot = median-of-3 or Tukey ninther; placed at *(lo + n/2). */        \
static inline T prefix##_pivot(T *lo, ptrdiff_t n) {                      \
    T *mid = lo + n/2, *hi = lo + n - 1;                                   \
    if (n >= NINTHER) {                                                     \
        ptrdiff_t s = n/8;                                                  \
        prefix##_med3(lo,    lo+s,   lo+2*s);                              \
        prefix##_med3(mid-s, mid,    mid+s);                                \
        prefix##_med3(hi-2*s,hi-s,   hi);                                  \
        prefix##_med3(lo+s,  mid,    hi-s);                                 \
    } else {                                                                \
        prefix##_med3(lo, mid, hi);                                         \
    }                                                                       \
    return *mid;                                                            \
}                                                                           \
                                                                            \
/* Branchless block partition [lo, hi] around pivot.                       \
 * Fills offset arrays without conditional branches:                        \
 *   offs[cnt] = i;  cnt += (a[i] >= pivot);                               \
 * GCC -O3 emits CMOV/SETCC for the += expression — zero branches.         \
 * Returns pointer to first element of right partition (>= pivot). */      \
static T* prefix##_block_part(T *lo, T *hi, T pivot) {                    \
    uint8_t lbuf[BLOCK], rbuf[BLOCK];                                      \
    int lc = 0, ls = 0, rc = 0, rs = 0;                                   \
    T *l = lo, *r = hi;                                                    \
                                                                            \
    while (r - l + 1 >= 2 * BLOCK) {                                       \
        if (!lc) {                                                          \
            ls = 0; lc = 0;                                                 \
            for (int i = 0; i < BLOCK; i++) {                              \
                lbuf[lc] = (uint8_t)i;                                     \
                lc += (l[i] >= pivot);   /* branchless */                  \
            }                                                               \
        }                                                                   \
        if (!rc) {                                                          \
            rs = 0; rc = 0;                                                 \
            for (int i = 0; i < BLOCK; i++) {                              \
                rbuf[rc] = (uint8_t)i;                                     \
                rc += (r[-i] < pivot);   /* branchless */                  \
            }                                                               \
        }                                                                   \
        int n = lc < rc ? lc : rc;                                         \
        for (int i = 0; i < n; i++) {                                      \
            T t = l[lbuf[ls+i]];                                           \
            l[lbuf[ls+i]] = r[-rbuf[rs+i]];                               \
            r[-rbuf[rs+i]] = t;                                            \
        }                                                                   \
        lc -= n; ls += n;                                                   \
        rc -= n; rs += n;                                                   \
        if (!lc) l += BLOCK;                                               \
        if (!rc) r -= BLOCK;                                               \
    }                                                                       \
                                                                            \
    /* Swap remaining buffered pairs from partial last blocks. */           \
    { int n = lc < rc ? lc : rc;                                           \
      for (int i = 0; i < n; i++) {                                        \
          T t = l[lbuf[ls+i]];                                             \
          l[lbuf[ls+i]] = r[-rbuf[rs+i]];                                 \
          r[-rbuf[rs+i]] = t;                                              \
      }                                                                     \
    }                                                                       \
                                                                            \
    /* Hoare tail sweep on remaining [l, r]. */                            \
    if (l > r) return l;                                                    \
    T *i = l - 1, *j = r + 1;                                             \
    for (;;) {                                                              \
        do { i++; } while (*i < pivot);                                    \
        do { j--; } while (*j > pivot);                                    \
        if (i >= j) return j + 1;                                          \
        T t = *i; *i = *j; *j = t;                                        \
    }                                                                       \
}                                                                           \
                                                                            \
/* Hoare partition for sub-arrays smaller than 2*BLOCK. */                 \
static T* prefix##_hoare(T *lo, T *hi, T pivot) {                         \
    T *i = lo - 1, *j = hi + 1;                                           \
    for (;;) {                                                              \
        do { i++; } while (*i < pivot);                                    \
        do { j--; } while (*j > pivot);                                    \
        if (i >= j) return j + 1;                                          \
        T t = *i; *i = *j; *j = t;                                        \
    }                                                                       \
}                                                                           \
                                                                            \
static void prefix##_recurse(T *lo, T *hi, int lim) {                     \
    for (;;) {                                                              \
        ptrdiff_t n = hi - lo + 1;                                         \
        if (n <= ISORT)  { prefix##_insort(lo, hi); return; }             \
        if (lim == 0)    { prefix##_heapsort(lo, n); return; }            \
        lim--;                                                              \
        /* Pattern detection: if [lo,hi] is already ascending, done.      \
         * On random data this scan stops at the first out-of-order pair   \
         * (expected position 1), costing ~2 comparisons.  On a sorted     \
         * sub-array it returns in O(n) without any pivot work. */         \
        { T *s = lo;                                                        \
          while (s < hi && *s <= *(s+1)) s++;                              \
          if (s == hi) return;                                              \
        }                                                                   \
        T pv = prefix##_pivot(lo, n);                                      \
        T *cut = (n >= 2 * BLOCK)                                          \
                 ? prefix##_block_part(lo, hi, pv)                        \
                 : prefix##_hoare(lo, hi, pv);                             \
        /* Tail-recurse into smaller partition. */                          \
        if (cut - lo <= hi - cut + 1) {                                    \
            prefix##_recurse(lo, cut - 1, lim);                            \
            lo = cut;                                                       \
        } else {                                                            \
            prefix##_recurse(cut, hi, lim);                                \
            hi = cut - 1;                                                   \
        }                                                                   \
    }                                                                       \
}

/* Instantiate for i64 and double. */
DEFINE_SORT(int64_t, si)
DEFINE_SORT(double,  sd)

static int ilog2_n(int64_t n) {
    int k = 0; while (n > 1) { n >>= 1; k++; } return k;
}

/* Public entry points — names match `declare` in backend_llvm.rs. */

int64_t intent_vec_i64__sort(VecI64 *xs) {
    int64_t n = xs->len;
    if (n < 2) return 0;
    int64_t *a = xs->data;
    /* Pattern detection: reverse-sorted → reverse in O(n) and return.
     * Uses >= so that equal-element runs are treated as non-ascending
     * (reversing a run of equal elements is a no-op, so this is safe).
     * For random data the scan stops at the first ascending pair
     * (expected at position 1), costing 1 comparison. */
    { int64_t *p = a;
      while (p + 1 < a + n && *p >= *(p+1)) p++;
      if (p + 1 == a + n) {
          int64_t *l = a, *r = a + n - 1;
          while (l < r) { int64_t t = *l; *l++ = *r; *r-- = t; }
          return 0;
      }
    }
    si_recurse(a, a + n - 1, 2 * ilog2_n(n));
    return 0;
}

int64_t intent_vec_double__sort(VecF64 *xs) {
    int64_t n = xs->len;
    if (n < 2) return 0;
    double *a = xs->data;
    /* Pattern detection: reverse-sorted → reverse in O(n) and return.
     * NaN: *p >= *(p+1) is false when either is NaN, so the scan stops
     * early and we fall through to the general sort (correct behavior). */
    { double *p = a;
      while (p + 1 < a + n && *p >= *(p+1)) p++;
      if (p + 1 == a + n) {
          double *l = a, *r = a + n - 1;
          while (l < r) { double t = *l; *l++ = *r; *r-- = t; }
          return 0;
      }
    }
    sd_recurse(a, a + n - 1, 2 * ilog2_n(n));
    return 0;
}
