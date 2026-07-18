# Intermediate 4d -- Default methods and blanket implementations (primer)

> **Learning goal**: understand what *default methods* and *blanket
> implementations* are, why they exist, and when to reach for each.
> Reading order: [04b interfaces primer](04b_interfaces_primer.md)
> -> [04c generics primer](04c_generics_primer.md) -> here ->
> [Intermediate 4 generics and interfaces](04_generics_iface.md)
> (the code surface).

This chapter has **no compiler code**. Intuition first, then the
one-page API.

## Default methods: the "built-in recipe"

Imagine a `Describable` interface that every type in your program
needs. For most types, the description is the same boilerplate --
"I am something." Only a few unusual types need a custom
description.

Without default methods, you'd have to copy-paste the same body
into every `implement Describable for Type` block -- tedious and
error-prone.

**Default methods** let the interface itself supply the body.
Implementors can **inherit** the default or **override** it with
their own version.

Think of it like a legal form with pre-filled fields. Most
signatories accept the defaults; a few cross out a field and
write their own value. The form is valid either way.

```vani
interface Describable {
  fn name(self: Self) -> Str;            // required -- no default

  fn describe(self: Self) -> Str {       // default body
    return "I am something.";
  }
}
```

A type that implements `Describable` MUST provide `name` (no
default) but may OMIT `describe` to inherit the default.

```vani
struct Dog { breed: Str }
struct Cat {}

implement Describable for Dog {
  fn name(self: Dog) -> Str { return self.breed; }
  // describe() NOT provided -> inherits default: "I am something."
}

implement Describable for Cat {
  fn name(self: Cat) -> Str { return "Cat"; }
  fn describe(self: Cat) -> Str { return "I am a cat."; }  // override
}
```

## The `Self` keyword

You'll notice `self: Self` in the interface signature above.
`Self` is a placeholder for "whatever concrete type is implementing
this interface." When `Dog` implements `Describable`, the compiler
substitutes `Self` -> `Dog` in the method signature.

This lets you write interface signatures that are generic without
declaring a type parameter. `fn name(self: Self) -> Str` becomes
`fn name(self: Dog) -> Str` inside `implement Describable for Dog`.

## Blanket implementations: "for any T that can already X, also Y"

A **blanket implementation** lets you implement an interface for
**any type `T` that already satisfies another bound**. You write
one impl block and it covers an infinite family of types
simultaneously.

The canonical example: a `Wrapper<T>` that forwards all method
calls to the inner `T`. If `T` can already print itself, then
`Wrapper<T>` should be able to print itself too -- by delegating:

```vani
interface Printable {
  fn print_it(self: Self) -> i64;
}

struct Wrapper<T> { inner: T }

implement<T> Printable for Wrapper<T> where T is Printable {
  fn print_it(self: Wrapper<T>) -> i64 {
    return self.inner.print_it();   // delegate to the inner T
  }
}
```

With this single blanket impl:
- `Wrapper<Dog>` gets `Printable` if `Dog` is `Printable`.
- `Wrapper<Cat>` gets `Printable` if `Cat` is `Printable`.
- `Wrapper<Wrapper<Dog>>` works too (doubly wrapped).

Without the blanket impl you'd need a separate
`implement Printable for Wrapper<Dog>`,
`implement Printable for Wrapper<Cat>`, ... one per concrete type.

## Blanket impl vs concrete impl

| Question | Concrete impl | Blanket impl |
|---|---|---|
| How many types does it cover? | One | Any T satisfying the bound |
| Syntax | `implement Iface for Type` | `implement<T> Iface for Container<T> where T is Bound` |
| Use when | You know the specific type | You're building a generic adapter/wrapper |

**Conflict rule**: the compiler rejects overlapping impls. If you
have a blanket impl for `Wrapper<T> where T is Printable` AND a
concrete impl `implement Printable for Wrapper<Dog>`, the compiler
reports ambiguity and rejects. Choose one path per concrete type.

## When to use each

**Default methods** -- reach for these when:
- Most implementors share the same body (saves boilerplate).
- You want to add a new method to an existing interface without
  breaking every existing `implement` block.
- The interface provides a sensible generic "fallback" behavior.

**Blanket implementations** -- reach for these when:
- You have a generic container/adapter type that should
  automatically inherit capabilities from its inner type.
- You want to derive interface X from interface Y automatically
  (e.g., "anything that is `Ord` should also be `Eq`").

## Quick syntax reference

**Default method** (in interface declaration):

```vani
interface MyIface {
  fn required_method(self: Self) -> i64;        // no body -> must implement

  fn optional_method(self: Self) -> Str {        // has body -> inheritable
    return "default";
  }
}
```

**Blanket implementation**:

```vani
implement<T> TargetIface for Container<T> where T is SourceBound {
  fn method(self: Container<T>) -> RetType {
    // implementation using self.inner (which is T) and T is SourceBound
  }
}
```

## Cross-reference

- [Intermediate 4b -- Interfaces primer](04b_interfaces_primer.md)
  -- what interfaces are and how `implement` blocks work
- [Intermediate 4c -- Generics primer](04c_generics_primer.md)
  -- type parameters, `where` bounds, and monomorphization
- [Intermediate 4 -- Generics and interfaces](04_generics_iface.md)
  -- worked code examples including default methods and blanket impls


---

**Previous**: [Sec.4c -- Generics and monomorphization primer ->](04c_generics_primer.md)
**Next**: [Sec.4 -- Generics and interfaces ->](04_generics_iface.md)

