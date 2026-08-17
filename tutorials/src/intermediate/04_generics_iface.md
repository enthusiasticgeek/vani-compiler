# Intermediate 4 -- Generics and interfaces

> **Learning goal**: declare an `interface` (vāṇī's name for
> traits), `implement` it for concrete types, and write a
> generic function bounded by `where T is Iface`.

> **New to this?** Read [Intermediate 4b -- Interfaces and static dispatch primer](04b_interfaces_primer.md)
> and [Intermediate 4c -- Generics primer](04c_generics_primer.md) first. This chapter
> is the code surface.

Think of an `interface` as a job description: "any employee who
fills this role must know how to do X, Y, Z." A `Circle` and a
`Square` both apply for the `Drawable` role by implementing it
-- you write their specific skills in an `implement` block. A
generic function that accepts `where T is Drawable` can then
work with any shape, just like a manager can assign tasks to
anyone who holds the right job title, without caring about the
specific person.

## The program

```vani
intent "Intermediate 4 worked example -- interfaces + static dispatch.";

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
  calls these *traits*). Methods may have full signatures only,
  or a **default body** -- see *Default methods* below.
- **`implement Iface for Type { fn ...; }`** provides the
  concrete method bodies. Methods with defaults can be omitted;
  methods without defaults must be implemented. Partial
  implementations (missing a required method) are rejected.
- **The interface method's `self` is the concrete struct, not
  `Self`**. In v1 you write `fn area(self: Circle)` inside
  `implement Drawable for Circle`. The interface declaration
  uses one of the concrete types as the "anchor" (here
  `Circle`); each `implement` block writes its own.
- **`where T is Iface`** is the interface bound on a generic.
  Since v0.5.3 you can also write the bound inline as
  **`<T: Iface>`** -- the two forms produce identical AST:
  ```vani
  fn print_area<T: Drawable>(s: T) -> i64 {  // inline form (v0.5.3+)
    return s.area();
  }
  ```
  The inline form is preferred for single-bound generics;
  `where` is clearer when multiple bounds appear.
- Static dispatch: the compiler emits one specialization per
  concrete T, so the call `s.area()` has zero runtime
  overhead.
- **Dynamic dispatch** (`dyn Iface`) is a separate path --
  covered in [Sec.5 -- Dynamic dispatch](05_dyn.md).

### A missing method is rejected

<img class="manas" src="../images/mascot/manas_mascot_error.png" title="this code does not compile!"/>

```vani
// This is a compile error -- Square's implement block omits `area`,
// which Drawable requires (no default body):
interface Drawable {
  fn area(self: Circle) -> i64;
}

struct Circle { r: i64 }
struct Square { side: i64 }

implement Drawable for Circle {
  fn area(self: Circle) -> i64 {
    return self.r * self.r;
  }
}

implement Drawable for Square {
  // ERROR: `area` is required by Drawable but never implemented here.
}
```

## Default methods (v0.1.1+)

An interface can provide a default body for a method. Types
that implement the interface can override it or inherit the
default.

```vani
interface Describable {
  fn name(self: Self) -> Str;          // required -- no default

  fn describe(self: Self) -> Str {     // default body
    return "I am something.";
  }
}

struct Dog { breed: Str }
struct Cat {}

implement Describable for Dog {
  fn name(self: Dog) -> Str { return self.breed; }
  // describe() is NOT overridden -- Dog inherits the default
}

implement Describable for Cat {
  fn name(self: Cat) -> Str { return "Cat"; }
  fn describe(self: Cat) -> Str { return "I am a cat."; }  // override
}

fn main() -> i64 {
  let d: Dog = Dog { breed: "Lab" };
  let c: Cat = Cat {};
  print d.describe();   // "I am something."  (default)
  print c.describe();   // "I am a cat."       (override)
  return 0;
}
```

## Blanket implementations (v0.1.1+)

A blanket impl lets you implement an interface for **any type
`T` that satisfies another bound** -- the `Wrapper<T>` example
below automatically gets `Printable` for every `T` that is
already `Printable`:

```vani
interface Printable {
  fn print_it(self: Self) -> i64;
}

struct Wrapper<T> { inner: T }

implement<T> Printable for Wrapper<T> where T is Printable {
  fn print_it(self: Wrapper<T>) -> i64 {
    return self.inner.print_it();
  }
}
```

The compiler checks that every method required by `Printable`
is present on `T` (satisfiability check) before accepting the
blanket impl.

### Two instantiations of the same generic struct

A generic struct isn't limited to one concrete `T` per program --
construct `Wrapper<Dog>` and `Wrapper<Cat>` side by side and each
gets its own independently-monomorphized copy:

```vani
struct Dog { name: i64 }
implement Printable for Dog {
  fn print_it(self: Dog) -> i64 { return 111; }
}

struct Cat { name: i64 }
implement Printable for Cat {
  fn print_it(self: Cat) -> i64 { return 222; }
}

fn main() -> i64 {
  let wd: Wrapper<Dog> = Wrapper { inner: Dog { name: 1 } };
  let wc: Wrapper<Cat> = Wrapper { inner: Cat { name: 2 } };
  print wd.print_it();   // 111 -- forwards to Dog's impl
  print wc.print_it();   // 222 -- forwards to Cat's impl
  return 0;
}
```

## Choosing static vs dynamic dispatch

| Need | Use |
|---|---|
| Know the concrete type at compile time | Generic `<T> where T is Iface` (this lesson) |
| Heterogeneous collection (`Vec<...>` of mixed types) | `dyn Iface` (Sec.5) |
| Tiny code size | `dyn Iface` (one fn body, runtime vtable) |
| Hot loop, minimal overhead | Generic (this lesson) |

## Challenge

Define a `Cmp` interface with `fn cmp(self: T, other: T) -> i64`
returning -1 / 0 / 1. Implement it for a small struct (e.g. a
`Score { value: i64 }` wrapper -- **primitive types like `i64`
can't `implement` any interface in v1**, `implement Cmp for i64`
is rejected outright with "requires a struct or enum type") and
write a generic `fn smaller<T>(a: T, b: T) -> T where T is Cmp`
that returns whichever argument is smaller.

Hint: this is exactly the shape of
`examples/language/english/bounded_generics.vani` -- peek there
if you get stuck.

---

**Previous**: [Sec.4d -- Default methods and blanket implementations primer ->](04d_default_methods_primer.md)
**Next**: [Sec.5 -- Dynamic dispatch ->](05_dyn.md)
