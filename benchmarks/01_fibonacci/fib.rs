// Benchmark 01 — Fibonacci(42) recursive  (Rust)
// rustc -C opt-level=2 -o fib_rs fib.rs && ./fib_rs
// Expected: 267914296

fn fib(n: i64) -> i64 {
    if n <= 1 { return n; }
    fib(n - 1) + fib(n - 2)
}

fn main() {
    println!("{}", fib(42));
}
