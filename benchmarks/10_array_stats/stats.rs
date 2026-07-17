// Benchmark 10 — Mean + variance of 10 000 000 values  (Rust, sequential)
//
// Build:
//   rustc -C opt-level=3 -C target-cpu=native -o stats_rs stats.rs && ./stats_rs
//
// REVIEWER NOTE: This is a SEQUENTIAL baseline. The vāṇī variant uses two
// `parallel for … reduce` passes (OpenMP-backed). Comparing parallel vs sequential
// measures parallelism strategy, not language quality.
//
// For a fair parallel vs parallel comparison, see stats_rayon.rs (Rayon) and
// stats_omp.c (C + OpenMP). Those should all cluster at ~35-40 ms on 4+ cores.
//
// This sequential file (~65 ms) is kept as the baseline to show the raw
// single-core throughput before parallelism is applied.

fn main() {
    const N: i64 = 10_000_000;
    let data: Vec<i64> = (0..N).map(|i| (i * 7 + 13) % 1000).collect();

    let sum: i64 = data.iter().sum();
    let mean = sum / N;

    let variance: i64 = data.iter().map(|&x| { let d = x - mean; d * d }).sum::<i64>() / N;

    println!("{}", mean);
    println!("{}", variance);
}
