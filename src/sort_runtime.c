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
 *   the packing phase so the compare step can use AVX-512 intrinsics
 *   (8 x i64 per 512-bit register → 8 vector ops vs 64 scalar) on hosts that
 *   have it.  The packing phase remains scalar (dependent store on lc), but
 *   operates on 8-bit booleans rather than 64-bit values.
 *
 * Compiler hints applied:
 *   - BUG-131 (2026-08-07): the compare step's AVX-512 intrinsics live in
 *     their own `__attribute__((target("avx512f,avx512bw,avx512dq,
 *     avx512vl")))`-decorated functions, dispatched at RUNTIME via
 *     `__builtin_cpu_supports("avx512f")` against a portable
 *     `target("arch=x86-64")` scalar sibling -- NOT a file-wide `#pragma
 *     GCC target`, which used to let the auto-vectorizer emit AVX-512
 *     ANYWHERE in this file (confirmed: it crashed inside `si_recurse`,
 *     not the block-partition code) on any x86_64 host that doesn't
 *     actually have it.
 *   - __attribute__((always_inline)) on block_part and hoare to prevent
 *     call overhead inside the hot recursive loop
 *   - __builtin_clzll for ilog2 (single BSR instruction)
 *   - __builtin_prefetch in the block loop to preload the next 512B
 */

#pragma GCC optimize("O3,unroll-loops,prefetch-loop-arrays")

/* BUG-125: the AVX-512 block-partition scan below is x86-only (the
 * `<immintrin.h>` header and `_mm512_*` intrinsics simply don't exist
 * on other architectures). This file is compiled unconditionally into
 * every LLVM-backend binary, including cross-compiled ones (`vanic
 * build --target=...`) -- on a non-x86 target (confirmed on
 * `arm-unknown-linux-gnueabi`) the AVX-512 intrinsics failed to
 * compile at all (`immintrin.h: No such file or directory`),
 * degrading to a non-fatal WARNING that still built a native binary
 * -- but with `intent_vec_i64__sort`/`intent_vec_double__sort`
 * missing entirely, so any program actually CALLING `sort`/`sort_by`
 * failed to LINK (`undefined reference to 'intent_vec_i64__sort'`).
 * Gate the x86-only bits behind an arch check and provide a portable
 * scalar equivalent for everyone else -- same algorithm, same output,
 * just without the vectorized compare/pack fast path.
 *
 * BUG-131: this used to ALSO carry `#pragma GCC target("avx512f,...")`
 * here, applied file-wide -- meaning GCC's auto-vectorizer was free to
 * use AVX-512 instructions ANYWHERE in this file it judged profitable
 * (`si_recurse`'s pivot-selection loop, `si_heapsort`, etc.), not just
 * inside the two functions that actually intend to use AVX-512
 * intrinsics. Confirmed by the crash site: a 200-element shuffled sort
 * (large enough to enter `_block_part`, whose OWN AVX-512 use is now
 * correctly runtime-gated -- see `vani_sort_mask_*_avx512` below) hit
 * `SIGILL` inside `si_recurse` on this dev machine's own Haswell CPU
 * (no AVX-512 support), NOT inside the block-partition code at all.
 * `<immintrin.h>` doesn't need the pragma to be included or used --
 * each intrinsic function is individually declared with its own
 * `target("...")` attribute in the header, so a caller with a MATCHING
 * `__attribute__((target(...)))` can call it without any file-wide
 * pragma forcing every OTHER function in the file onto the same
 * assumption. Dropped the pragma entirely; the two explicitly-marked
 * AVX-512 functions below opt in on their own, and everything else in
 * the file now compiles under whatever `-march=`/`-mtune=` the actual
 * `cc` invocation passes (`-march=native` for a host build -- see
 * `main.rs`), which GCC won't use to emit AVX-512 on a CPU that
 * doesn't have it. */
#if defined(__x86_64__) || defined(__i386__)
#define VANI_SORT_HAVE_AVX512 1
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

/* The two mask-computation shapes used inside `_block_part` below
 * ("which of these BLOCK elements starting at `ptr` are >= / <
 * `pivot`, as a 64-bit bitmask") -- AVX-512 on x86 (runtime-gated,
 * see BUG-131 below), a portable scalar loop everywhere else. `T`
 * needs genuinely different comparison semantics per instantiation
 * (see BUG-131's `_f64` note), so this dispatches on `pivot`'s type
 * via C11 `_Generic` rather than being one macro shared verbatim
 * across both `DEFINE_SORT` instantiations. */
#if VANI_SORT_HAVE_AVX512
/* BUG-131 (2026-08-07), part 1 -- runtime CPU-capability dispatch.
 * A file-wide `#pragma GCC target(avx512...)` used to guarantee only
 * that the COMPILER could target AVX-512 -- it said nothing about
 * whether the CPU actually running the resulting binary has it, AND
 * (worse) let GCC's auto-vectorizer emit AVX-512 ANYWHERE in this
 * file it judged profitable, not just in the two functions that
 * intend to use it. Every x86_64 build unconditionally used AVX-512
 * with no runtime check, so `sort`/`sort_by` raised SIGILL on any
 * x86_64 host predating AVX-512 (confirmed on this dev machine's own
 * Haswell CPU, no `avx512*` flags in `/proc/cpuinfo`) -- and the
 * crash site wasn't even confined to the block-partition code: a
 * large enough sort hit SIGILL inside `si_recurse`'s own pivot-
 * selection loop, auto-vectorized by GCC under the ambient pragma.
 *
 * Fixed with real runtime CPUID dispatch, and the pragma dropped
 * entirely (see the file-top comment): the actual vector compare
 * lives in `vani_sort_mask_*_avx512`, each explicitly decorated with
 * `__attribute__((target(...)))` so GCC compiles THAT function with
 * AVX-512 enabled regardless of command-line flags -- nothing else in
 * the file gets that treatment anymore. A sibling `vani_sort_mask_*
 * _scalar` is decorated with `__attribute__((target("arch=x86-64")))`
 * to guarantee no AVX-512/AVX2/BMI2 auto-vectorization creeps in
 * there either (confirmed by disassembly, not just documentation).
 * `vani_sort_mask_ge`/`_lt` pick between the two via `__builtin_cpu_
 * supports("avx512f")` (a cheap read of a CPUID probe GCC's runtime
 * caches on first use, not a fresh CPUID instruction every call).
 * `target_clones` (GCC's usual IFUNC-based multiversioning attribute)
 * was tried first and rejected: it silently NO-OPs (a `-Wattributes`
 * warning, "ignoring attribute 'target_clones' because it conflicts
 * with attribute 'target'") when combined with an enclosing `#pragma
 * GCC target` -- confirmed empirically, back when this file still had
 * one file-wide, since getting that interaction wrong would have
 * silently reintroduced the exact crash this fix exists to prevent.
 * `_block_part` itself stays `always_inline` (unchanged) -- only the
 * mask computation, the one part that's actually CPU-feature-
 * dependent, goes through a real (non-inlined) function call, so the
 * dispatch resolves per-block at runtime instead of being baked into
 * the inlined caller at compile time.
 *
 * BUG-131, part 2 -- found chasing part 1: `int64_t`'s mask functions
 * correctly compare raw bit patterns (that IS int64_t's native
 * ordering -- no transform needed), but `double`'s ORIGINALLY did the
 * exact same raw-int64-bit-pattern compare (`_mm512_cmpge_epi64_mask`
 * on the double's bits reinterpreted as int64_t, a BUG-125-era
 * decision explicitly called out as deliberate: "never using an FP
 * compare intrinsic even for sd_block_part"). That's wrong: IEEE-754
 * negative doubles do NOT preserve their true ordering when compared
 * as raw signed int64_t -- e.g. -1000.0's bits (-4571364728013586432)
 * compare GREATER than -0.001's bits (-4661117527937406468) as raw
 * int64_t, backwards from -1000.0 < -0.001. This was invisible before
 * this session: any x86_64 host without AVX-512 crashed (part 1's
 * bug) before ever producing output, and the crash always fired
 * before a large-enough (>=128-element) `Vec<f64>` sort could return
 * a wrong answer. Fixing part 1's crash surfaced this for the first
 * time -- a 300-element shuffled negative-and-positive `Vec<f64>`
 * sort silently returned out-of-order output. Fixed by giving
 * `double` its own genuinely-floating-point mask functions
 * (`_mm512_cmp_pd_mask`/native `>=`/`<`) instead of routing through
 * int64_t's bit-pattern path -- `int64_t`'s own functions are
 * unaffected and unchanged. */
__attribute__((target("avx512f,avx512bw,avx512dq,avx512vl")))
static uint64_t vani_sort_mask_ge_i64_avx512(const int64_t *ptr, int64_t pivot) {
    __m512i vpivot_v = _mm512_set1_epi64((long long)pivot);
    uint64_t out_mask = 0;
    for (int bi = 0; bi < BLOCK; bi += 8) {
        __m512i vals = _mm512_loadu_si512((const __m512i *)(ptr + bi));
        __mmask8 k = _mm512_cmpge_epi64_mask(vals, vpivot_v);
        out_mask |= (uint64_t)(unsigned)k << bi;
    }
    return out_mask;
}
__attribute__((target("avx512f,avx512bw,avx512dq,avx512vl")))
static uint64_t vani_sort_mask_lt_i64_avx512(const int64_t *ptr, int64_t pivot) {
    __m512i vpivot_v = _mm512_set1_epi64((long long)pivot);
    uint64_t out_mask = 0;
    for (int bi = 0; bi < BLOCK; bi += 8) {
        __m512i vals = _mm512_loadu_si512((const __m512i *)(ptr + bi));
        __mmask8 k = _mm512_cmplt_epi64_mask(vals, vpivot_v);
        out_mask |= (uint64_t)(unsigned)k << bi;
    }
    return out_mask;
}
__attribute__((target("arch=x86-64")))
static uint64_t vani_sort_mask_ge_i64_scalar(const int64_t *ptr, int64_t pivot) {
    uint64_t out_mask = 0;
    for (int bi = 0; bi < BLOCK; bi++) {
        if (ptr[bi] >= pivot) out_mask |= (uint64_t)1 << bi;
    }
    return out_mask;
}
__attribute__((target("arch=x86-64")))
static uint64_t vani_sort_mask_lt_i64_scalar(const int64_t *ptr, int64_t pivot) {
    uint64_t out_mask = 0;
    for (int bi = 0; bi < BLOCK; bi++) {
        if (ptr[bi] < pivot) out_mask |= (uint64_t)1 << bi;
    }
    return out_mask;
}
static inline uint64_t vani_sort_mask_ge_i64(const int64_t *ptr, int64_t pivot) {
    return __builtin_cpu_supports("avx512f")
        ? vani_sort_mask_ge_i64_avx512(ptr, pivot)
        : vani_sort_mask_ge_i64_scalar(ptr, pivot);
}
static inline uint64_t vani_sort_mask_lt_i64(const int64_t *ptr, int64_t pivot) {
    return __builtin_cpu_supports("avx512f")
        ? vani_sort_mask_lt_i64_avx512(ptr, pivot)
        : vani_sort_mask_lt_i64_scalar(ptr, pivot);
}

__attribute__((target("avx512f,avx512bw,avx512dq,avx512vl")))
static uint64_t vani_sort_mask_ge_f64_avx512(const double *ptr, double pivot) {
    __m512d vpivot_v = _mm512_set1_pd(pivot);
    uint64_t out_mask = 0;
    for (int bi = 0; bi < BLOCK; bi += 8) {
        __m512d vals = _mm512_loadu_pd(ptr + bi);
        __mmask8 k = _mm512_cmp_pd_mask(vals, vpivot_v, _CMP_GE_OQ);
        out_mask |= (uint64_t)(unsigned)k << bi;
    }
    return out_mask;
}
__attribute__((target("avx512f,avx512bw,avx512dq,avx512vl")))
static uint64_t vani_sort_mask_lt_f64_avx512(const double *ptr, double pivot) {
    __m512d vpivot_v = _mm512_set1_pd(pivot);
    uint64_t out_mask = 0;
    for (int bi = 0; bi < BLOCK; bi += 8) {
        __m512d vals = _mm512_loadu_pd(ptr + bi);
        __mmask8 k = _mm512_cmp_pd_mask(vals, vpivot_v, _CMP_LT_OQ);
        out_mask |= (uint64_t)(unsigned)k << bi;
    }
    return out_mask;
}
__attribute__((target("arch=x86-64")))
static uint64_t vani_sort_mask_ge_f64_scalar(const double *ptr, double pivot) {
    uint64_t out_mask = 0;
    for (int bi = 0; bi < BLOCK; bi++) {
        if (ptr[bi] >= pivot) out_mask |= (uint64_t)1 << bi;
    }
    return out_mask;
}
__attribute__((target("arch=x86-64")))
static uint64_t vani_sort_mask_lt_f64_scalar(const double *ptr, double pivot) {
    uint64_t out_mask = 0;
    for (int bi = 0; bi < BLOCK; bi++) {
        if (ptr[bi] < pivot) out_mask |= (uint64_t)1 << bi;
    }
    return out_mask;
}
static inline uint64_t vani_sort_mask_ge_f64(const double *ptr, double pivot) {
    return __builtin_cpu_supports("avx512f")
        ? vani_sort_mask_ge_f64_avx512(ptr, pivot)
        : vani_sort_mask_ge_f64_scalar(ptr, pivot);
}
static inline uint64_t vani_sort_mask_lt_f64(const double *ptr, double pivot) {
    return __builtin_cpu_supports("avx512f")
        ? vani_sort_mask_lt_f64_avx512(ptr, pivot)
        : vani_sort_mask_lt_f64_scalar(ptr, pivot);
}

#define VANI_SORT_MASK_GE(out_mask, ptr, pivot)                            \
    ((out_mask) = _Generic((pivot),                                        \
        double: vani_sort_mask_ge_f64((const double *)(ptr), (double)(pivot)), \
        default: vani_sort_mask_ge_i64((const int64_t *)(ptr), (int64_t)(pivot))))
#define VANI_SORT_MASK_LT(out_mask, ptr, pivot)                            \
    ((out_mask) = _Generic((pivot),                                        \
        double: vani_sort_mask_lt_f64((const double *)(ptr), (double)(pivot)), \
        default: vani_sort_mask_lt_i64((const int64_t *)(ptr), (int64_t)(pivot))))
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
