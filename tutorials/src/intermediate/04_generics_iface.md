# Intermediate 4 — Generics and interfaces

> **Learning goal**: declare an `interface` (vāṇी's name for
> traits), `implement` it for concrete types, and write a
> generic function bounded by `where T is Iface`.

## The program

```vani
intent "Intermediate 4 worked example — interfaces + static dispatch.";

struct Circle { r: i64 }
struct Square { side: i64 }

interface Drawable {
  fn area(self: Circle) -> i64;
}

implement Drawable for Circle {
  fn area(self: Circle) -> i64 {
    return self.r * self.r;
  }
}

implement Drawable for Square {
  fn area(self: Square) -> i64 {
    return self.side * self.side;
  }
}

// Static-dispatch generic. `where T is Drawable` constrains
// the type parameter to any T with a Drawable implementation
// in scope. The compiler monomorphizes per concrete T.
fn print_area<T>(s: T) -> i64 where T is Drawable {
  let a: i64 = s.area();
  print "area =", a;
  return a;
}

fn main() -> i64 {
  let c: Circle = Circle { r: 5 };
  let s: Square = Square { side: 4 };
  print_area(c);
  print_area(s);
  return 0;
}
```

## Compile + run

```bash
vanic run ~/int4.vani
```

Output:

```
area = 25
area = 16
```

## Why it works that way

- **`interface Name { fn ...; }`** declares an interface (Rust
  calls these *traits*). Methods are listed with their full
  signature — no body in v1 (no default methods).
- **`implement Iface for Type { fn ...; }`** provides the
  concrete method bodies. Every interface method must be
  implemented; partial implementations are rejected.
- **The interface method's `self` is the concrete struct, not
  `Self`**. In v1 you write `fn area(self: Circle)` inside
  `implement Drawable for Circle`. The interface declaration
  uses one of the concrete types as the "anchor" (here
  `Circle`); each `implement` block writes its own.
- **`where T is Iface`** is the interface bound on a generic.
  Static dispatch: the compiler emits one specialization per
  concrete T, so the call `s.area()` has zero runtime
  overhead.
- **Dynamic dispatch** (`dyn Iface`) is a separate path —
  covered in [§5 — Dynamic dispatch](05_dyn.md).

## Choosing static vs dynamic dispatch

| Need | Use |
|---|---|
| Know the concrete type at compile time | Generic `<T> where T is Iface` (this lesson) |
| Heterogeneous collection (`Vec<…>` of mixed types) | `dyn Iface` (§5) |
| Tiny code size | `dyn Iface` (one fn body, runtime vtable) |
| Hot loop, minimal overhead | Generic (this lesson) |

## Challenge

Define a `Cmp` interface with `fn cmp(self: T, other: T) -> i64`
returning -1 / 0 / 1. Implement it for `i64` and write a
generic `fn smaller<T>(a: T, b: T) -> T where T is Cmp` that
returns whichever argument is smaller.

Hint: this is exactly the shape of
`examples/language/english/bounded_generics.vani` — peek there
if you get stuck.

---

**Next**: [§5 — Dynamic dispatch →](05_dyn.md)
