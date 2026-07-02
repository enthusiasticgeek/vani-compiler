// Benchmark 06 — Parallel sum of 50 000 000 elements  (Rust + Rayon)
// Idiomatic Rayon parallel reduction — fair comparison for vani's
// `parallel for ... reduce total with +`.
//
// Requires Rayon in Cargo.toml. If compiling standalone with rustc,
// use parsum.rs (std::thread) instead.
//
// cargo add rayon && cargo run --release

fn main() {
    use rayon::prelude::*;

    const N: usize = 50_000_000;
    let data: Vec<i64> = (0..N as i64).map(|i| i % 1000).collect();

    let total: i64 = data.par_iter().sum();

    println!("{}", total);
}
