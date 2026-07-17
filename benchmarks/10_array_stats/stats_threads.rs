// Benchmark 10 — Mean + variance of 10 000 000 values  (Rust + std::thread, parallel)
//
// FAIR PARALLEL COMPARISON for vāṇī's `parallel for … reduce`.
// Uses std::thread::scope (stable since Rust 1.63, no Cargo/Rayon required).
// Thread count: thread::available_parallelism() = same default as OMP_NUM_THREADS.
//
// Build:
//   rustc -C opt-level=3 -C target-cpu=native -o stats_rs_par stats_threads.rs
//
// Same data pattern, same algorithm, same integer arithmetic as:
//   stats.vani (vāṇī parallel for … reduce ×2)
//   stats_omp.c (C + OpenMP #pragma reduction)
//   stats_omp.cpp (C++ + OpenMP)

use std::thread;

fn main() {
    const N: i64 = 10_000_000;
    let data: Vec<i64> = (0..N).map(|i| (i * 7 + 13) % 1000).collect();

    let num_threads = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    let chunk = (N as usize + num_threads - 1) / num_threads;

    // Pass 1: parallel sum → mean.
    let sum: i64 = thread::scope(|s| {
        let handles: Vec<_> = (0..num_threads)
            .map(|t| {
                let start = t * chunk;
                let end = (start + chunk).min(N as usize);
                let slice = &data[start..end];
                s.spawn(move || slice.iter().sum::<i64>())
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).sum()
    });
    let mean = sum / N;

    // Pass 2: parallel variance.
    let var_sum: i64 = thread::scope(|s| {
        let handles: Vec<_> = (0..num_threads)
            .map(|t| {
                let start = t * chunk;
                let end = (start + chunk).min(N as usize);
                let slice = &data[start..end];
                s.spawn(move || {
                    slice.iter().map(|&x| { let d = x - mean; d * d }).sum::<i64>()
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).sum()
    });
    let variance = var_sum / N;

    println!("{}", mean);
    println!("{}", variance);
}
