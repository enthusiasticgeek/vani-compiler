# Intermediate 3 -- Affine ownership: `ref` / `mut ref`

> **Learning goal**: borrow a struct by `ref` (read-only) or
> `mut ref` (exclusive read-write), and understand how vāṇी's
> affine ownership keeps borrows safe at compile time.

> **New to this?** Read [Beginner 6c -- Ownership and move](../beginner/06c_ownership_primer.md)
> first for the analogy, then [Intermediate 3b -- Affine deeper pass](03b_affine_deeper_primer.md)
> for the precise mechanics. This chapter is the code surface.

Imagine a hotel room. You have the key (ownership). You can lend
a friend a read-only pass (`ref` -- they can look, not move
furniture). Or you give a cleaner a full key (`mut ref` -- they can
rearrange), but only one cleaner can hold the key at a time so
they don't conflict. When done, the key returns to you. That's
the borrow model.

## The program

```vani
intent "Intermediate 3 worked example -- affine ownership and borrows.";

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
- **There is no `let mut` in v1** ([L5 in v1_limitations.md](https://github.com/enthusiasticgeek/vani-compiler/blob/main/docs/v1_limitations.md)).
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

## What the compiler catches

### Passing by value where `ref` is expected

If you forget the `ref` keyword at the call site, the compiler
rejects the call because you're trying to move a value into a
parameter slot that expects only a borrow:

<img class="manas" src="../images/mascot/manas_mascot_error.png" title="this code does not compile!"/>

```vani
fn sum_pair(p: ref Pair) -> i64 { return p.a + p.b; }

fn main() -> i64 {
  let pair: Pair = Pair { a: 3, b: 4 };
  let _ = sum_pair(pair);   // wrong: passes by value, not by ref
  return 0;
}
```

```
error: argument 1 to 'sum_pair' must be assignable to ref Pair, got Pair
  let _ = sum_pair(pair);
                   ^^^^
  help: The value has type `Pair`, but the slot expects `ref Pair`.
```

**Fix**: write `sum_pair(ref pair)`.

### Using a heap-owning value after it was moved

Structs that contain heap-owning fields (like `OwnedStr`,
`Vec<T>`, or another struct with such fields) are **affine**:
ownership transfers on each pass-by-value. Using the binding
after the move is a compile error:

<img class="manas" src="../images/mascot/manas_mascot_error.png" title="this code does not compile!"/>

```vani
struct Named { name: OwnedStr, val: i64 }

fn consume(n: Named) -> i64 { return n.val; }

fn main() -> i64 {
  let x: Named = Named { name: "alice" + "", val: 42 };
  let _ = consume(x);   // ownership of x transfers to consume()
  let _ = consume(x);   // error: x was already moved
  return 0;
}
```

```
error: value 'x' was moved; cannot use after move
  let _ = consume(x);   // error: x was already moved
                  ^
note: 'x' was moved here
  let _ = consume(x);   // ownership of x transfers to consume()
                  ^
  help: borrow with `ref x` for read-only access, or call `clone(x)` if the type supports it
```

**Fix**: use `consume(ref x)` if `consume` can accept a borrow,
or restructure so the second use happens before the move.

> **Note**: structs whose fields are all scalar (`i64`, `bool`,
> `Str`, etc.) are automatically **Copy** — they can be passed
> by value multiple times without a move error. The affine rule
> only applies to heap-owning types.

---

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

**Previous**: [Sec.3e -- Lifetimes and reference returns primer ->](03e_lifetimes_primer.md)
**Next**: [Sec.4a -- What's a `dyn Iface`? primer ->](04a_dyn_iface_primer.md)
