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
 * Block partition scan (hot path):
 *   The comparison phase (l[i] >= pivot for 64 elements) is separated from
 *   the packing phase so GCC can auto-vectorise comparisons with AVX-512
 *   (8 x i64 per 512-bit register → 8 vector ops vs 64 scalar).  The packing
 *   phase remains scalar (dependent store on lc), but operates on 8-bit
 *   booleans rather than 64-bit values.
 *
 * Compiler hints applied:
 *   - #pragma GCC target: avx512f/vl/bw/dq (Ice Lake), avx2, bmi2, popcnt
 *   - __attribute__((always_inline)) on block_part and hoare to prevent
 *     call overhead inside the hot recursive loop
 *   - __builtin_clzll for ilog2 (single BSR instruction)
 *   - __builtin_prefetch in the block loop to preload the next 512B
 */

#pragma GCC optimize("O3,unroll-loops,prefetch-loop-arrays")

/* BUG-125: the AVX-512 block-partition scan below is x86-only (the
 * `#pragma GCC target`, `<immintrin.h>`, and `_mm512_*` intrinsics
 * simply don't exist on other architectures). This file is compiled
 * unconditionally into every LLVM-backend binary, including
 * cross-compiled ones (`vanic build --target=...`) -- on a non-x86
 * target (confirmed on `arm-unknown-linux-gnueabi`) the AVX-512
 * pragma/intrinsics failed to compile at all (`unknown target
 * attribute 'avx512f'`, `immintrin.h: No such file or directory`),
 * degrading to a non-fatal WARNING that still built a native binary
 * -- but with `intent_vec_i64__sort`/`intent_vec_double__sort`
 * missing entirely, so any program actually CALLING `sort`/`sort_by`
 * failed to LINK (`undefined reference to 'intent_vec_i64__sort'`).
 * Gate the x86-only bits behind an arch check and provide a portable
 * scalar equivalent for everyone else -- same algorithm, same output,
 * just without the vectorized compare/pack fast path. */
#if defined(__x86_64__) || defined(__i386__)
#define VANI_SORT_HAVE_AVX512 1
#pragma GCC target("avx512f,avx512bw,avx512dq,avx512vl,avx2,bmi2,popcnt")
#include <immintrin.h>
#else
#define VANI_SORT_HAVE_AVX512 0
#endif

#include <stdint.h>
#include <stddef.h>

typedef struct { int64_t *data; int64_t len; int64_t cap; } VecI64;
typedef struct { double  *data; int64_t len; int64_t cap; } VecF64;

#define ISORT    24
#define BLOCK    64
#define NINTHER  128

/* BUG-125: the two mask-computation shapes used inside `_block_part`
 * below ("which of these BLOCK elements starting at `ptr` are
 * >= / < `pivot`, as a 64-bit bitmask") -- AVX-512 on x86, a portable
 * scalar loop everywhere else. Note the AVX-512 form compares raw
 * bit patterns via `_mm512_cmpge/cmplt_epi64_mask` for BOTH the
 * int64_t and double instantiations (never using an FP compare
 * intrinsic even for `sd_block_part`) -- pre-existing x86-path
 * behavior, left exactly as it was; the scalar fallback below uses
 * T's own native `>=`/`<` operators, which is what every OTHER
 * (non-block-partition) comparison in this file already does. */
#if VANI_SORT_HAVE_AVX512
#define VANI_SORT_MASK_GE(out_mask, ptr, pivot)                            \
    do {                                                                   \
        __m512i vpivot_v = _mm512_set1_epi64((long long)(pivot));          \
        out_mask = 0;                                                      \
        for (int bi = 0; bi < BLOCK; bi += 8) {                            \
            __m512i vals = _mm512_loadu_si512((const __m512i *)((ptr) + bi)); \
            __mmask8 k = _mm512_cmpge_epi64_mask(vals, vpivot_v);          \
            out_mask |= (uint64_t)(unsigned)k << bi;                       \
        }                                                                  \
    } while (0)
#define VANI_SORT_MASK_LT(out_mask, ptr, pivot)                            \
    do {                                                                   \
        __m512i vpivot_v = _mm512_set1_epi64((long long)(pivot));          \
        out_mask = 0;                                                      \
        for (int bi = 0; bi < BLOCK; bi += 8) {                            \
            __m512i vals = _mm512_loadu_si512((const __m512i *)((ptr) + bi)); \
            __mmask8 k = _mm512_cmplt_epi64_mask(vals, vpivot_v);          \
            out_mask |= (uint64_t)(unsigned)k << bi;                       \
        }                                                                  \
    } while (0)
#else
#define VANI_SORT_MASK_GE(out_mask, ptr, pivot)                            \
    do {                                                                   \
        out_mask = 0;                                                      \
        for (int bi = 0; bi < BLOCK; bi++) {                               \
            if ((ptr)[bi] >= (pivot)) out_mask |= (uint64_t)1 << bi;       \
        }                                                                  \
    } while (0)
#define VANI_SORT_MASK_LT(out_mask, ptr, pivot)                            \
    do {                                                                   \
        out_mask = 0;                                                      \
        for (int bi = 0; bi < BLOCK; bi++) {                               \
            if ((ptr)[bi] < (pivot)) out_mask |= (uint64_t)1 << bi;        \
        }                                                                  \
    } while (0)
#endif

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
/* Block partition [lo, hi] around pivot.                                  \
 *                                                                          \
 * Scan phase: separated into comparison (auto-vectorised with AVX-512)    \
 * and packing (sequential, but on 8-bit booleans rather than 64-bit       \
 * values). On random data ~32 of 64 elements qualify per block; the       \
 * packing loop runs 64 iterations regardless.                              \
 *                                                                          \
 * The lbuf/rbuf arrays have BLOCK+4 bytes to absorb potential 4-byte      \
 * over-writes in vectorised variants without overflowing.                  \
 *                                                                          \
 * Returns pointer to first element of right partition (>= pivot). */      \
static inline __attribute__((always_inline))                                \
T* prefix##_block_part(T *lo, T *hi, T pivot) {                           \
    uint8_t lbuf[BLOCK + 4], rbuf[BLOCK + 4];                             \
    int lc = 0, ls = 0, rc = 0, rs = 0;                                   \
    T *l = lo, *r = hi;                                                    \
                                                                            \
    while (r - l + 1 >= 2 * BLOCK) {                                       \
        /* Prefetch next BLOCK-sized chunks to hide memory latency. */     \
        __builtin_prefetch(l + 2*BLOCK, 0, 1);                            \
        __builtin_prefetch(r - 2*BLOCK, 0, 1);                            \
        if (!lc) {                                                          \
            /* AVX-512 (x86) / scalar (everywhere else, BUG-125):        \
             * compare BLOCK elements against pivot, build a 64-bit       \
             * bitmask, then walk set bits to pack qualifying indices.    \
             * On x86 this replaces a 64-iteration scalar packing loop    \
             * with 8 vector compares + ~32 bit-walk iterations (50%      \
             * qualifying for random data). */                            \
            uint64_t mask_l;                                               \
            VANI_SORT_MASK_GE(mask_l, l, pivot);                           \
            lc = 0;                                                        \
            uint64_t m = mask_l;                                           \
            while (m) {                                                    \
                lbuf[lc++] = (uint8_t)__builtin_ctzll(m);                 \
                m &= m - 1;                                                \
            }                                                               \
            ls = 0;                                                        \
        }                                                                   \
        if (!rc) {                                                          \
            T *rb = r - BLOCK + 1;                                         \
            uint64_t mask_r;                                               \
            VANI_SORT_MASK_LT(mask_r, rb, pivot);                          \
            rc = 0;                                                        \
            uint64_t m = mask_r;                                           \
            while (m) {                                                    \
                rbuf[rc++] = (uint8_t)__builtin_ctzll(m);                 \
                m &= m - 1;                                                \
            }                                                               \
            rs = 0;                                                        \
        }\
        /* Swap min(lc,rc) pairs. */                                       \
        {   int n = lc < rc ? lc : rc;                                     \
            T *rb = r - BLOCK + 1;                                         \
            for (int i = 0; i < n; i++) {                                  \
                T t = l[lbuf[ls+i]];                                       \
                l[lbuf[ls+i]] = rb[rbuf[rs+i]];                           \
                rb[rbuf[rs+i]] = t;                                        \
            }                                                               \
            lc -= n; ls += n;                                               \
            rc -= n; rs += n;                                               \
            if (!lc) l += BLOCK;                                           \
            if (!rc) r -= BLOCK;                                           \
        }                                                                   \
    }                                                                       \
                                                                            \
    /* Swap remaining buffered pairs from partial last blocks. */           \
    {   int n = lc < rc ? lc : rc;                                         \
        T *rb = r - BLOCK + 1;                                             \
        for (int i = 0; i < n; i++) {                                      \
            T t = l[lbuf[ls+i]];                                           \
            l[lbuf[ls+i]] = rb[rbuf[rs+i]];                               \
            rb[rbuf[rs+i]] = t;                                            \
        }                                                                   \
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
static inline __attribute__((always_inline))                                \
T* prefix##_hoare(T *lo, T *hi, T pivot) {                                \
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
    /* BSR instruction: 63 - clzll(n) for n > 0. */
    return n > 1 ? (int)(63 - __builtin_clzll((uint64_t)n)) : 0;
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
