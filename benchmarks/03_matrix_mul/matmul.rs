// Benchmark 03 — 256×256 i64 matrix multiply  (Rust)
// rustc -C opt-level=2 -o matmul_rs matmul.rs && ./matmul_rs

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
