// Benchmark 10 — Mean + variance of 10 000 000 values  (Rust + Rayon parallel)
//
// REVIEWER CONCERN ADDRESSED:
//   The baseline comparison is vāṇī parallel vs C/Rust sequential.
//   That measures parallelism strategy, not language quality.
//   This file is the fair apples-to-apples counterpart:
//     vāṇī: `parallel for … reduce sum with +`   (OpenMP-backed)
//     Rust:  Rayon par_iter().sum()               (work-stealing thread pool)
//     C:     stats_omp.c with #pragma omp         (same thread count)
//
// ALGORITHM PARITY:
//   Two passes: (1) sum → mean, (2) sum of (x-mean)^2 → variance.
//   Same integer arithmetic, same data pattern as stats.vani / stats.c.
//   Same thread count: OMP_NUM_THREADS / Rayon both default to nCPU.
//
// Build (requires rayon in Cargo.toml):
//   cargo add rayon && cargo run --release --bin stats_rayon
//
// Or standalone (rustc with vendored rayon is complex; prefer cargo):
//   cargo build --release && ./target/release/stats_rayon

fn main() {
    use rayon::prelude::*;

    const N: i64 = 10_000_000;
    let n = N as usize;

    // Same deterministic data pattern as stats.vani / stats.c.
    let data: Vec<i64> = (0..N).map(|i| (i * 7 + 13) % 1000).collect();

    // Pass 1: parallel sum → mean.
    let sum: i64 = data.par_iter().copied().sum();
    let mean = sum / N;

    // Pass 2: parallel variance (integer arithmetic).
    let var_sum: i64 = data
        .par_iter()
        .map(|&x| {
            let d = x - mean;
            d * d
        })
        .sum();
    let variance = var_sum / N;

    println!("{}", mean);
    println!("{}", variance);
}
