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

## Borrow checker: yes, but smaller than Rust's

If you're coming from Rust (or asking "why doesn't vāṇी have a
borrow checker like Rust?"), the premise is off: vāṇी has one.
`ref` / `mut ref` above IS the borrow checker -- it enforces the
same core rule Rust does, "many shared borrows XOR one exclusive
mutable borrow, never both," and it rejects use-after-move and
dangling references at compile time. It's built directly into
the affine type system rather than living in a separate named
pass, but the guarantee is the same class of guarantee.

What's smaller than Rust's version:

| | Rust | vāṇी v1 |
|---|---|---|
| Shared-XOR-mutable borrows | ✓ enforced | ✓ enforced |
| Use-after-move rejected | ✓ | ✓ |
| Partial moves (move one struct field) | ✓ | ✓ |
| Dangling-reference-return rejected | ✓ | ✓ |
| Lifetime syntax (`'a`) | explicit, user-written | **none -- always elided** |
| Fn returning `ref T` with 2+ ref params | ✓ (annotate which lifetime) | [x] rejected -- restructure to one ref param |
| Struct holding refs with independent lifetimes | ✓ | [x] rejected -- use one shared lifetime or restructure |
| Closures capturing a ref that outlives the closure | ✓ | [x] deferred to a later version |
| `Rc<T>` / `Weak<T>` for cyclic data (trees, doubly-linked lists) | ✓ | [x] no equivalent -- use index handles into a `Vec`/`Pool` instead |

**What you'd lose with no borrow checker at all** (i.e. plain
C-style pointers): silent use-after-free, double-free, data
races from aliased mutable pointers, and dangling references
returned from functions -- all of them runtime bugs that show up
far from their cause, sometimes only under load or with specific
inputs. vāṇी's borrow checker turns every one of those into a
compile error at the call site.

**What the smaller (elided-only) design costs you**, versus
Rust's explicit lifetimes: a few advanced shapes don't compile
as-written and need a mechanical workaround --

- A function taking two `ref` parameters can't return a `ref`
  derived from just one of them (Rust: annotate `<'a>`; vāṇी:
  split into narrower functions, or make the unused param a
  value instead of a ref). See [Intermediate 3e -- lifetimes
  primer](03e_lifetimes_primer.md).
- Cyclic data (a tree node with a parent pointer, a
  doubly-linked list, an observer pattern) can't use owning
  pointers in either direction -- there's no `Rc`/`Weak`. The
  fix is to store the structure as indices into a flat `Vec`
  instead of a graph of pointers. See [Intermediate 3d --
  cyclic references primer](03d_cyclic_references_primer.md)
  for the full rewrite, plus the rare cases (third-party plugin
  callbacks, DOM-like shared graphs) where an `unsafe(reason =
  "...")` escape hatch is the honest answer instead.

The design bet, stated in [Intermediate 3e](03e_lifetimes_primer.md#why-this-design):
elide the easy 90% for free, reject the remaining 10% with a
loud diagnostic and a mechanical workaround, rather than exposing
`'a` syntax to every user for the sake of the 10%.

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

<img class="manas" src="../images/mascot/manas_mascot_success.png" title="this is the correct, working version"/>

```vani
fn sum_pair(p: ref Pair) -> i64 { return p.a + p.b; }

fn main() -> i64 {
  let pair: Pair = Pair { a: 3, b: 4 };
  let _ = sum_pair(ref pair);
  return 0;
}
```

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

<img class="manas" src="../images/mascot/manas_mascot_success.png" title="this is the correct, working version"/>

```vani
struct Named { name: OwnedStr, val: i64 }

fn consume(n: ref Named) -> i64 { return n.val; }

fn main() -> i64 {
  let x: Named = Named { name: "alice" + "", val: 42 };
  let _ = consume(ref x);   // borrow, doesn't move
  let _ = consume(ref x);   // fine: x is still owned by main
  return 0;
}
```

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
