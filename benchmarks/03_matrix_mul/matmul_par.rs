// Benchmark 03 — 256×256 matrix multiply  (Rust + Rayon)
// Parallel outer-row loop via Rayon — fair parallel comparison.
//
// Requires Rayon. With bare rustc use matmul.rs (sequential).
// cargo add rayon && cargo run --release

fn main() {
    use rayon::prelude::*;

    let n: usize = 256;
    let sz = n * n;

    let a: Vec<i64> = (0..sz as i64).map(|i| i % 97 + 1).collect();
    let b: Vec<i64> = (0..sz as i64).map(|i| i % 53 + 1).collect();
    let mut c: Vec<i64> = vec![0i64; sz];

    // Parallel outer row loop: split c into row-chunks, one per thread.
    c.par_chunks_mut(n)
        .enumerate()
        .for_each(|(row, c_row)| {
            for col in 0..n {
                let mut sum = 0i64;
                for m in 0..n {
                    sum += a[row * n + m] * b[m * n + col];
                }
                c_row[col] = sum;
            }
        });

    println!("{}", c[0] + c[sz - 1]);
}
