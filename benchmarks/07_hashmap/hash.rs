// Benchmark 07 — HashMap: 500 000 insert + 500 000 lookup  (Rust)
// rustc -C opt-level=2 -o hash_rs hash.rs && ./hash_rs

use std::collections::HashMap;

fn main() {
    const N: i64 = 500_000;
    let mut m: HashMap<i64, i64> = HashMap::with_capacity(N as usize * 2);

    for i in 0..N { m.insert(i, i * i); }

    let sum: i64 = (0..N).map(|j| m[&j]).sum();

    println!("{}", m.len());
    println!("{}", sum);
}
