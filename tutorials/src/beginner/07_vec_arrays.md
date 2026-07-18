# Beginner 7 -- Arrays and `Vec<T>` basics

> **Learning goal**: declare fixed-size arrays and heap-allocated
> `Vec<T>`s, iterate them with `while` + indexing, and pass them
> to functions by reference.

> **New to this?** Read [Beginner 7a -- Tuples and destructuring primer](07a_tuples_primer.md)
> and [Beginner 6b -- Heap and stack primer](06b_heap_vs_stack_primer.md) first.

An array is like a fixed-size egg carton: you declare it holds
exactly 4 eggs, and it always holds exactly 4 slots. A `Vec<T>`
is like a resizable shopping bag: you can push items in and it
grows as needed. Both hold items in order and let you access
any item by its position number (starting from 0). Arrays live
entirely on the stack; Vec keeps a small handle on the stack and
stores the actual items on the heap, which is how it grows.

## The program

Save this in `~/lesson7.vani`:

```vani
intent "Lesson 7 worked example -- arrays + Vec basics.";

fn sum_array(xs: ref [i64; 4]) -> i64 {
  let total: i64 = 0;
  for i from 0 to 4 {
    total = total + xs[i];
  }
  return total;
}

fn sum_vec(xs: ref Vec<i64>) -> i64 {
  let total: i64 = 0;
  let i: u64 = 0;
  while i < len(xs) {
    total = total + xs[i];
    i = i + 1;
  }
  return total;
}

fn count_positive(xs: ref Vec<i64>) -> i64 {
  let count: i64 = 0;
  let i: u64 = 0;
  while i < len(xs) {
    if xs[i] > 0 {
      count = count + 1;
    }
    i = i + 1;
  }
  return count;
}

fn main() -> i64 {
  let arr: [i64; 4] = [10, 20, 30, 40];
  print "sum_array =", sum_array(ref arr);

  let v: Vec<i64> = vec(1, 2, 3, 4, 5);
  print "sum_vec =", sum_vec(ref v);
  print "len(v) =", len(ref v);

  let mixed: Vec<i64> = vec(0 - 2, 5, 0 - 1, 7, 0);
  print "count_positive =", count_positive(ref mixed);
  return 0;
}
```

## Compile + run

```bash
vanic run ~/lesson7.vani
```

Expected output:

```
sum_array = 100
sum_vec = 15
len(v) = 5
count_positive = 2
```

## Why it works that way

- **Arrays have a fixed length in the type**. `[i64; 4]` is "an
  array of four `i64`s." The length is part of the type; passing
  an `[i64; 3]` where `[i64; 4]` is expected is a type error.
- **`Vec<T>` is heap-allocated and grows**. It's the right
  default for variable-length collections. Construct one with
  `vec(...)` (varargs literal) or `Vec::new()` (empty). Both
  emit the same `intent_vec_int64_t` runtime bundle in C.
- **Pass by reference with `ref`**. `xs: ref Vec<i64>` is a
  read-only borrow -- the function reads but doesn't free the
  Vec. If you passed by value, ownership would transfer and
  the caller couldn't use `xs` after the call (affine
  ownership -- Intermediate Sec.3). `ref` is what you want most of
  the time.
- **Index with `[i]`**. Both arrays and `Vec<T>` support `[i]`.
  The compiler proves at compile time that the index is in
  bounds when it can (SMT pass); otherwise a runtime bounds
  check fires. Out-of-bounds is a clean abort, never undefined
  behavior.
- **`len(arr)` is a compile-time constant**, `len(vec)` is a
  runtime value. Both have type `u64`. Mixing `u64` and `i64`
  needs an explicit cast -- that's the most common beginner
  speed-bump.
- **Iterating with `while`** is the most explicit form. The
  Intermediate track shows `for x in xs { ... }` once you're
  comfortable.

## Challenge

Write a `max_in_vec(xs: ref Vec<i64>) -> i64` that returns the
maximum element. Assume the Vec is non-empty (you'll add a
`requires` clause for this in Sec.9).

<details>
<summary>Solution</summary>

```vani
fn max_in_vec(xs: ref Vec<i64>) -> i64 {
  let best: i64 = xs[0];
  let i: u64 = 1;
  while i < len(xs) {
    if xs[i] > best {
      best = xs[i];
    }
    i = i + 1;
  }
  return best;
}
```

</details>

---

**Previous**: [Sec.7a -- Tuples and destructuring primer ->](07a_tuples_primer.md)
**Next**: [Sec.8a -- Pattern matching primer ->](08a_pattern_match_primer.md)
