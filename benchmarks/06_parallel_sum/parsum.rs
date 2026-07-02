// Benchmark 06 — Parallel sum of 50 000 000 elements  (Rust)
// Uses std::thread with manual work-splitting (no Rayon dependency,
// so this compiles with bare rustc).
//
// rustc -C opt-level=2 -o parsum_rs parsum.rs && ./parsum_rs

use std::thread;

fn main() {
    const N: usize = 50_000_000;
    let data: Vec<i64> = (0..N as i64).map(|i| i % 1000).collect();

    let num_threads = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);

    let chunk = (N + num_threads - 1) / num_threads;
    let data_ref: &[i64] = &data;

    // SAFETY: We slice the data read-only and each thread gets a distinct
    // range, so there are no data races.
    let total: i64 = thread::scope(|s| {
        let handles: Vec<_> = (0..num_threads)
            .map(|t| {
                let start = t * chunk;
                let end   = (start + chunk).min(N);
                let slice = &data_ref[start..end];
                s.spawn(move || slice.iter().sum::<i64>())
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).sum()
    });

    println!("{}", total);
}
