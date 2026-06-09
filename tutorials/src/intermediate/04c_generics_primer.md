# Intermediate 4c — Generics and monomorphization (intuition primer)

> **Learning goal**: build a mental model of "generic" — the
> `<T>` notation you've seen in `Vec<T>` or `fn id<T>(x: T)`.
> Why this exists, what the compiler does with it, and when
> to reach for it. Reading order: [04b interfaces primer](04b_interfaces_primer.md)
> → here → [Intermediate 4 generics+interfaces](04_generics_iface.md)
> for the formal syntax.

This chapter has **no compiler code**. Pure intuition.

## The problem: code that's the same except for the type

You wrote a function that returns the maximum of two i64s:

```rust
fn max_i64(a: i64, b: i64) -> i64 {
  if a > b { return a; }
  return b;
}
```

Now you need the same thing for f64. And for u32. And for any
type that has a `>` operator.

Without generics, you'd write `max_f64`, `max_u32`, `max_i32`,
... — N copies of the same algorithm, differing only in the
type name. Tedious, error-prone (a bug fix has to be applied
N times), and bloats your source.

**Generics** let you write the function ONCE with a placeholder
for the type, and the compiler generates the type-specific
versions for you.

## The cookie cutter analogy

A generic function is a **cookie cutter**. The shape is fixed —
"take two values, return whichever is larger" — but the
material isn't specified. When you press the cutter into
chocolate-chip dough, you get a chocolate-chip cookie. When you
press it into oatmeal dough, you get an oatmeal cookie.

```rust
fn max<T>(a: T, b: T) -> T where T is Comparable {
  if a > b { return a; }
  return b;
}
```

`<T>` is the cookie cutter's "material slot". `T` is a
placeholder — when someone calls `max(3, 7)`, the compiler
sees T = i64. When they call `max(3.14, 2.71)`, T = f64.

The `where T is Comparable` is the bound — only types
supporting `>` (the Comparable interface) can use this cookie
cutter. Without the bound, the compiler couldn't be sure the
`if a > b` step would work for arbitrary T.

## What the compiler ACTUALLY does

This is the load-bearing concept: the compiler doesn't keep
one generic version. It generates a **specialized copy per
concrete type** you call it with.

In your program:
```rust
let x: i64 = max(3, 7);
let y: f64 = max(3.14, 2.71);
let z: u32 = max(100 as u32, 50 as u32);
```

The compiler emits THREE functions:
- `max_i64(a: i64, b: i64) -> i64` (specialized to i64)
- `max_f64(a: f64, b: f64) -> f64` (specialized to f64)
- `max_u32(a: u32, b: u32) -> u32` (specialized to u32)

Each is fully optimized for its specific type. The i64 version
uses i64 comparison instructions; the f64 version uses f64
comparison instructions. No runtime type-dispatch, no
indirection.

This process is called **monomorphization** — "making
mono-typed versions". The Greek root: *mono* = one, *morph* =
form. Generic source → multiple mono-typed compiled forms.

## Why "monomorphization, not type erasure"?

Java (and some other languages) take a different approach
called **type erasure**: at runtime, generic types are
"erased" — your `List<Integer>` and `List<String>` are both
just `List` at runtime. They share ONE compiled implementation.

Pros of erasure:
- One compiled function per generic (smaller binary).
- Recompiling user code doesn't require recompiling the
  generic library.

Cons:
- Boxing: small types like `int` have to be wrapped in
  `Integer` objects to fit through the generic — slow.
- No type-specific optimization: the one compiled version has
  to work for everything.
- Lost type info at runtime: you can't ask "is this a
  List<String> vs List<Integer>?".

vāṇी (and Rust, and C++ templates) pick monomorphization
because the speed win is large for systems code. The cost is
a bigger compiled binary, but that's usually acceptable.

## Common generic shapes

### `Vec<T>` — generic container

You've seen this. `Vec<i64>`, `Vec<OwnedStr>`, `Vec<Bag>` —
each is its own type at runtime, with its own specialized
compiled helpers (push, pop, len, etc.).

### `Option<T>` / `Result<T, E>` — generic enum

```rust
enum Option<T> {
  Some(T),
  None,
}
```

Same shape (Some or None) regardless of T. The compiler
generates `Option__i64`, `Option__OwnedStr`, etc. per use.

### `id<T>` — generic identity function

```rust
fn id<T>(x: T) -> T { return x; }
```

Almost trivial — just returns its argument. But the type can
be anything. Useful as a building block in functional patterns.

### Generic with bounds

```rust
fn min<T>(a: T, b: T) -> T where T is Comparable {
  if a < b { return a; }
  return b;
}
```

The `where T is Comparable` bound tells the compiler what
operations you'll use inside the body (`<`). Without it, the
compiler can't verify the body — it'd reject the use of `<` on
an unknown type.

## Multi-parameter generics

```rust
enum Result<T, E> {
  Ok(T),
  Err(E),
}
```

Two placeholders. `Result<i64, OwnedStr>` is different from
`Result<f64, OwnedStr>` — both monomorphized separately.

## When NOT to use generics

If your function only ever runs on i64, don't make it generic.
The generic is overhead (in syntax + reader cognitive load + a
slight increase in compile time) for no benefit.

Generics shine when:
- The same algorithm genuinely works across types (containers,
  utility functions like `max` / `min` / `swap`).
- A library wants flexibility for its users (Vec, HashMap, etc.).

Generics misfire when:
- You're parameterizing over types that have nothing in common
  (use multiple specific functions instead).
- The bound (`where T is …`) effectively names a single type
  (just use that type).

## A summary you can carry

- A **generic** is a cookie cutter with a type-shaped slot.
- `<T>` declares the type placeholder.
- `where T is Iface` is a bound — restricts T to types
  implementing the interface, and lets the body call the
  interface's methods on T.
- The compiler does **monomorphization**: generates one
  compiled copy per concrete type used. Fast, no boxing, no
  runtime type info.
- The trade-off vs Java-style erasure: bigger binary, faster
  runtime, no boxing. vāṇी picks this trade-off.

That's generics. The pairing with interfaces (which 04b
covered) is the whole story: interfaces define contracts,
generics make functions that work over any type satisfying a
contract.

The next chapter ([Intermediate 4](04_generics_iface.md))
shows the syntax + a worked example with both pieces together.

## Cross-reference

- [Intermediate 4 — Generics and interfaces](04_generics_iface.md)
  — actual syntax + worked example
- [Intermediate 4a — `dyn Iface` primer](04a_dyn_iface_primer.md)
  — the dynamic-dispatch alternative to monomorphization
- [Intermediate 4b — Interfaces and static dispatch](04b_interfaces_primer.md)
  — the contract side of the generic+interface pairing
- [Beginner 6a — Pointers and references](../beginner/06a_pointers_refs_primer.md)
  — generics over reference types (`<T>` and `<ref T>`) compose
  cleanly
