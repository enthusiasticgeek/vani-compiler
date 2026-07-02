// Benchmark 02 — Sieve of Eratosthenes ≤ 2 000 000  (Rust)
// rustc -C opt-level=2 -o sieve_rs sieve.rs && ./sieve_rs
// Expected: 148933

fn main() {
    let limit = 2_000_000usize;
    let mut sieve = vec![true; limit + 1];
    sieve[0] = false;
    sieve[1] = false;

    let mut i = 2usize;
    while i * i <= limit {
        if sieve[i] {
            let mut j = i * i;
            while j <= limit {
                sieve[j] = false;
                j += i;
            }
        }
        i += 1;
    }

    let count = sieve.iter().filter(|&&x| x).count();
    println!("{}", count);
}
