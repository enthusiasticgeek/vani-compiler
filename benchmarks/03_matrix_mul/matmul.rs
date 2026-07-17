// Benchmark 03 — 256×256 i64 matrix multiply  (Rust, baseline i-j-k)
//
// Build:
//   rustc -C opt-level=3 -C target-cpu=native -o matmul_rs matmul.rs && ./matmul_rs
//
// WHY THIS IS ~2× SLOWER THAN C / vāṇī (~33 ms vs ~15 ms):
//
//   Issue 1 — LOOP ORDER (i-j-k, not i-k-j):
//     The inner loop (k) steps through b[k*N+col] with stride N.
//     That is column-major access — a cache miss on every inner iteration for N=256.
//     vāṇī and matmul.c both use i-k-j (inner col loop is sequential) which is
//     vectorisable as a SAXPY and avoids the stride-N miss pattern.
//
//   Issue 2 — BOUNDS CHECKS:
//     Every a[row*N+k], b[k*N+col], c[row*N+col] inserts a compare+branch.
//     Even if LLVM can prove some are unreachable, the extra IR prevents SIMD
//     pattern-matching from firing reliably.
//
// SEE ALSO: matmul_ikj.rs (this dir) — i-k-j + unsafe::get_unchecked → ~15 ms.

fn main() {
    const N: usize = 256;
    let mut a = vec![0i64; N * N];
    let mut b = vec![0i64; N * N];
    let mut c = vec![0i64; N * N];

    for i in 0..N * N { a[i] = (i % 97 + 1) as i64; }
    for i in 0..N * N { b[i] = (i % 53 + 1) as i64; }

    for row in 0..N {
        for col in 0..N {
            let mut sum = 0i64;
            for k in 0..N {
                sum += a[row * N + k] * b[k * N + col];
            }
            c[row * N + col] = sum;
        }
    }

    println!("{}", c[0] + c[N * N - 1]);
}
