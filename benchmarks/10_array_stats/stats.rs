// Benchmark 10 — Mean + variance of 10 000 000 values  (Rust)
// rustc -C opt-level=2 -o stats_rs stats.rs && ./stats_rs

fn main() {
    const N: i64 = 10_000_000;
    let data: Vec<i64> = (0..N).map(|i| (i * 7 + 13) % 1000).collect();

    let sum: i64 = data.iter().sum();
    let mean = sum / N;

    let variance: i64 = data.iter().map(|&x| { let d = x - mean; d * d }).sum::<i64>() / N;

    println!("{}", mean);
    println!("{}", variance);
}
