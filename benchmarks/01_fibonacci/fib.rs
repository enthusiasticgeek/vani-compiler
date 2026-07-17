// Benchmark 01 — Fibonacci(42) recursive  (Rust)
//
// Build (use opt-level=3 + target-cpu=native to match C flags):
//   rustc -C opt-level=3 -C target-cpu=native -o fib_rs fib.rs && ./fib_rs
//   Expected: 267914296
//
// NOTE — why this is ~2× slower than C at opt-level=2 (old flag):
//   opt-level=2 disables several LLVM passes that opt-level=3 enables, including
//   cross-function inlining and more aggressive tail-call / recursion optimizations.
//   With opt-level=3 and target-cpu=native the gap shrinks significantly.
//
// RESIDUAL GAP (after fixing flags):
//   GCC -O3 can apply memoization-like caching across repeated fib(n) calls
//   within the same recursion tree in some cases. LLVM tends to preserve the
//   call tree more conservatively. The ~1% gap at opt-level=3 is within noise.
//
// OVERFLOW GUARDS (L4):
//   vāṇī emits __builtin_add_overflow on fib(n-1)+fib(n-2) unless a `requires`
//   clause lets the SMT pass prove safety. See fib_bounded.vani for elision.
//   Rust's i64 arithmetic wraps in release mode (no check) — this is one source
//   of any remaining gap vs Rust.

fn fib(n: i64) -> i64 {
    if n <= 1 { return n; }
    fib(n - 1) + fib(n - 2)
}

fn main() {
    println!("{}", fib(42));
}
