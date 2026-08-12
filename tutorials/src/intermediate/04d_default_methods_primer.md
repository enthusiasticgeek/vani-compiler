# Intermediate 4d -- Default methods and blanket implementations (primer)

> **Learning goal**: understand what *default methods* and *blanket
> implementations* are, why they exist, and when to reach for each.
> Reading order: [04b interfaces primer](04b_interfaces_primer.md)
> -> [04c generics primer](04c_generics_primer.md) -> here ->
> [Intermediate 4 generics and interfaces](04_generics_iface.md)
> (the code surface).

This chapter leads with intuition, then a one-page API reference
with real code.

## The government form with pre-filled boxes

Imagine you're filling out an official government form -- something
like a passport renewal or a tax form. Near the top there's a field
labeled "Country of residence," and it already has "USA" typed in,
pre-filled, with a small printed note next to it: "leave as-is if
this applies to you."

For the overwhelming majority of people filling out this form, USA
is exactly right. They read the pre-filled box, nod, and move on to
the next field without touching it. They didn't have to look up how
to spell "United States of America," didn't have to decide on
formatting -- the form already did that work for them. Submitted this
way, the form is still 100% official and 100% valid.

But suppose you live abroad. The pre-filled "USA" is wrong for you.
Nothing stops you: you cross it out, write "Canada" in its place,
and submit the form. It's still the exact same official form, still
fully valid, still accepted at the counter -- you just supplied your
own answer for that one box instead of the one that came
pre-printed.

Nobody had to design two different forms -- one for USA residents and
one for everyone else. Nobody had to reprint the whole form because
you needed a different country filled in. One form, one pre-filled
default, and an explicit, always-available option to override it
for the fields where the default doesn't apply. Notice, too, that
some boxes on the form -- like "Signature" -- are never pre-filled:
there's no way around filling those in yourself, because there's no
sensible default a stranger could write on your behalf.

Bridge to CS terms: the "pre-filled box you're free to leave as-is"
is a **default method** -- the interface itself supplies a working
method body, and any type implementing that interface may accept it
(by simply not writing the method) or override it with its own
version. The "Signature box with no default" is a **required
method** -- the interface provides no body, so every implementor
MUST supply one. Either way, the finished form -- like the finished
`implement` block -- is complete and valid.

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

**Overlap rule**: unlike most conflicting-declaration situations in
vāṇी (duplicate functions, duplicate struct fields), the compiler
does **not** reject an overlapping blanket + concrete impl pair --
there's no "ambiguous, pick one" diagnostic. If you have a blanket
impl for `Wrapper<T> where T is Printable` AND a concrete impl
`implement Printable for Wrapper<Dog>`, both are accepted, and the
**concrete impl silently wins** for `Wrapper<Dog>` specifically (the
blanket impl still covers every other `T`). This is a real, load-
bearing behavior -- not just an implementation gap -- since it's
exactly what lets you write a general-purpose blanket impl and then
carve out a faster or different hand-written override for one
specific type without touching the blanket impl at all. But because
there's no diagnostic, a *typo'd* concrete impl (wrong type param,
wrong module) won't warn you that it silently stopped shadowing
anything -- if you expected an override to take effect and it
doesn't seem to be running, double-check the concrete impl's `for`
type matches exactly.

<img class="manas" src="../images/mascot/manas_mascot_success.png" title="this is the correct, working version"/>

```vani
interface Printable {
  fn print_it(self: Self) -> i64;
}

struct Dog { name: i64 }
struct Wrapper<T> { inner: T }

implement<T> Printable for Wrapper<T> where T is Printable {
  fn print_it(self: Wrapper<T>) -> i64 { return 111; }   // generic path
}

implement Printable for Wrapper<Dog> {
  fn print_it(self: Wrapper<Dog>) -> i64 { return 222; } // Dog-specific override
}

implement Printable for Dog {
  fn print_it(self: Dog) -> i64 { return self.name; }
}

fn main() -> i64 {
  let w: Wrapper<Dog> = Wrapper { inner: Dog { name: 7 } };
  print w.print_it();   // prints 222, not 111 -- the override won
  return 0;
}
```

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

