// Benchmark 09 — Allocation stress: 500 000 struct push/read cycles  (Rust)
// rustc -C opt-level=2 -o alloc_rs alloc.rs && ./alloc_rs

struct Payload { a: i64, b: i64, c: i64 }

fn main() {
    const N: usize = 500_000;
    let mut items: Vec<Payload> = Vec::with_capacity(N);

    for i in 0..N as i64 {
        items.push(Payload { a: i, b: i * 2, c: i * 3 });
    }

    let sum: i64 = items.iter().map(|p| p.a + p.c).sum();
    // `items` is dropped here (Rust RAII — same deterministic drop as vāṇī).
    println!("{}", sum);
}
