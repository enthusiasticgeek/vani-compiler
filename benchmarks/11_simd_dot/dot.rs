// Benchmark 11 — Dot product of two 4 000 000-element f32 vectors  (Rust)
// rustc -C opt-level=3 -C target-cpu=native -o dot_rs dot.rs && ./dot_rs

fn main() {
    let n: usize = 4_000_000;
    let a: Vec<f32> = (0..n).map(|i| (i % 100) as f32 * 0.01).collect();
    let b: Vec<f32> = vec![1.0f32; n];

    let r1: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let r2: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();

    println!("{}", r1 as i64);
    println!("{}", r2 as i64);
}
