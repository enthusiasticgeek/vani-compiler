// Benchmark 08 — Index-based linked list, 1 000 000 nodes  (Rust)
// Same approach as list.vani: two parallel Vecs, no heap alloc per node.
// Rust's borrow checker makes this natural for cyclic or
// self-referential structures (no Rc<RefCell<T>> needed).
//
// rustc -C opt-level=2 -o list_rs list.rs && ./list_rs

fn main() {
    const N: usize = 1_000_000;
    let values: Vec<i64>         = (0..N as i64).map(|i| i % 1000).collect();
    let mut next: Vec<Option<usize>> = (0..N).map(|i| Some(i + 1)).collect();
    next[N - 1] = None;

    let mut sum: i64 = 0;
    let mut curr = Some(0usize);
    while let Some(idx) = curr {
        sum += values[idx];
        curr = next[idx];
    }
    println!("{}", sum);
}
