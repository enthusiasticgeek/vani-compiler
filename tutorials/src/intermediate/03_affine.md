# Intermediate 3 — Affine ownership: `ref` / `mut ref`

> **Learning goal**: borrow a struct by `ref` (read-only) or
> `mut ref` (exclusive read-write), and understand how vāṇी's
> affine ownership keeps borrows safe at compile time.

## The program

```vani
intent "Intermediate 3 worked example — affine ownership and borrows.";

struct Pair { a: i64, b: i64 }

// `ref` = read-only borrow. The caller keeps ownership; this
// function can read fields but not free the struct.
fn sum_pair(p: ref Pair) -> i64 {
  return p.a + p.b;
}

// `mut ref` = exclusive read-write borrow. The function can
// write fields through it; the caller can't access `p` while
// the borrow is live.
fn double_pair(p: mut ref Pair) -> i64 {
  p.a = p.a * 2;
  p.b = p.b * 2;
  return p.a + p.b;
}

fn main() -> i64 {
  let pair: Pair = Pair { a: 3, b: 4 };
  let s1: i64 = sum_pair(ref pair);
  print "sum =", s1;

  let p2: Pair = Pair { a: 5, b: 7 };
  let doubled_sum: i64 = double_pair(mut ref p2);
  print "doubled_sum =", doubled_sum;
  print "p2.a =", p2.a;
  print "p2.b =", p2.b;
  return 0;
}
```

## Compile + run

```bash
vanic run ~/int3.vani
```

Output:

```
sum = 7
doubled_sum = 24
p2.a = 10
p2.b = 14
```

## Why it works that way

- **`ref Type`** in a parameter says "I borrow this read-only."
  The call site uses `ref name` to opt into the borrow:
  `sum_pair(ref pair)`.
- **`mut ref Type`** is the read-write borrow. Call site form:
  `double_pair(mut ref p2)`.
- **Affine ownership** means each binding has exactly one
  owner. Passing by value (no `ref`) transfers ownership; the
  caller can't use the variable afterward. Passing by `ref` /
  `mut ref` is a non-owning borrow that keeps the caller's
  binding intact.
- **There is no `let mut` in v1** ([L5 in v1_limitations.md](https://github.com/anthropics/claude-code/blob/main/docs/v1_limitations.md)).
  A regular `let` binding is sufficient for an inner `mut ref`
  borrow to work, as shown above. This is one of the
  intentional surface-syntax simplifications relative to Rust.
- **No two `mut ref` borrows can coexist**. The exclusivity is
  enforced at compile time via the affine type system.

## When to use which

| Use case | Borrow form |
|---|---|
| Read fields | `ref T` |
| Write fields, then return | `mut ref T` |
| Move ownership out | pass by value (no `ref`) |
| Pass a `Vec<T>` to a function for reading | `ref Vec<T>` |

## Challenge

Write a `swap_pair(p: mut ref Pair)` that swaps `p.a` and `p.b`.
Call it on a `Pair { a: 1, b: 2 }` and verify the fields are
swapped after the call.

<details>
<summary>Solution</summary>

```vani
fn swap_pair(p: mut ref Pair) -> i64 {
  let tmp: i64 = p.a;
  p.a = p.b;
  p.b = tmp;
  return 0;
}

fn main() -> i64 {
  let p: Pair = Pair { a: 1, b: 2 };
  let _ = swap_pair(mut ref p);
  print "a =", p.a, "b =", p.b;  // a = 2 b = 1
  return 0;
}
```

</details>

---

**Next**: [§4 — Generics and interfaces →](04_generics_iface.md)
