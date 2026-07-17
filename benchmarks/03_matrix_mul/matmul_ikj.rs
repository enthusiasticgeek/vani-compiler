// Benchmark 03 — 256×256 i64 matrix multiply  (Rust, cache-optimal i-k-j + unsafe)
//
// WHY BASELINE matmul.rs IS ~2× SLOWER THAN C / vāṇī:
//
//   Two compounding issues:
//
//   1. LOOP ORDER — baseline uses i-j-k:
//        for row in 0..N {
//          for col in 0..N {        ← outer col loop
//            for k in 0..N {        ← inner k: accesses b[k*N+col] with stride N
//
//      Stride-N access on b is column-major: L1/L2 miss on every iteration.
//      LLVM cannot auto-vectorise the inner loop because b and c are not
//      accessed contiguously. Result: scalar loop, cache-thrashing.
//
//   2. BOUNDS CHECKS — every a[row*N+k], b[k*N+col], c[row*N+col]
//      inserts a branch + potential panic. The extra branches prevent SIMD
//      pattern recognition even when loop order is fixed.
//
// THIS FILE FIXES BOTH:
//
//   Loop order: i-k-j (identical to matmul.vani and matmul.c):
//        for row in 0..N {
//          for k in 0..N {           ← middle k: a_val = a[row*N+k] (scalar, once)
//            for col in 0..N {       ← inner col: b[k*N+col] and c[row*N+col] sequential
//
//      Inner col loop is a SAXPY: c_row[col] += a_val * b_row[col].
//      Sequential reads of b and c enable AVX2/SSE2 auto-vectorisation.
//
//   Indexing: unsafe get_unchecked / get_unchecked_mut removes all bounds checks.
//   Justified: indices are always in [0, N*N) by construction of the loop bounds.
//
// EXPECTED RESULT: ~15-16 ms, matching C and vāṇī (vs ~33 ms for baseline).
//
// Build:
//   rustc -C opt-level=3 -C target-cpu=native -o matmul_rs_ikj matmul_ikj.rs && ./matmul_rs_ikj

fn main() {
    const N: usize = 256;
    let sz = N * N;

    // Same initialisation as matmul.rs / matmul.vani / matmul.c.
    let a: Vec<i64> = (0..sz as i64).map(|i| i % 97 + 1).collect();
    let b: Vec<i64> = (0..sz as i64).map(|i| i % 53 + 1).collect();
    let mut c = vec![0i64; sz];

    // i-k-j loop: broadcast a_val, then SAXPY across the row of B and C.
    for row in 0..N {
        let row_n = row * N;
        for k in 0..N {
            // SAFETY: row_n + k < N*N because row < N and k < N.
            let a_val = unsafe { *a.get_unchecked(row_n + k) };
            let k_n = k * N;
            for col in 0..N {
                // SAFETY: row_n + col < N*N and k_n + col < N*N by loop bounds.
                unsafe {
                    *c.get_unchecked_mut(row_n + col) +=
                        a_val * b.get_unchecked(k_n + col);
                }
            }
        }
    }

    // Same checksum as the other variants.
    println!("{}", c[0] + c[sz - 1]);
}
