// Benchmark 04 — Sort 1 000 000 integers  (Rust)
// rustc -C opt-level=2 -o sort_rs sort.rs && ./sort_rs

fn main() {
    const N: usize = 1_000_000;
    let mut xs = Vec::with_capacity(N);

    let mut seed: i64 = 12345678;
    let a: i64 = 1664525;
    let c: i64 = 1013904223;
    let mask: i64 = 2147483647;
    for _ in 0..N {
        seed = (a * seed + c) % mask;
        xs.push(seed);
    }

    xs.sort_unstable();
    println!("{}", xs[0] + xs[N - 1]);
}
