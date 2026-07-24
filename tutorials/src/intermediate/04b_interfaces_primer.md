# Intermediate 4b -- Interfaces and static dispatch (intuition primer)

> **Learning goal**: build a mental model of "interface" (what
> vāṇी calls Rust's "trait" or Java's "interface") and "static
> dispatch" -- the non-`dyn` counterpart that pairs with
> [04a dyn Iface primer](04a_dyn_iface_primer.md). Reading
> order: 04a -> here -> [Intermediate 4 generics+interfaces](04_generics_iface.md)
> -> [Intermediate 5 dyn dispatch](05_dyn.md).

This chapter has **no compiler code**. Pure intuition.

## What an interface IS

An **interface** is a contract. It says: "any type that wants
to be a `Shape` must provide a method called `area` that takes
the shape and returns an `i64`."

That's it. Not "this is what a Shape looks like inside." Not
"this is what a Shape's data is." Just: **here is the set of
methods you must implement to call yourself a Shape.**

It's the LEGO-brick principle. A LEGO brick is anything with
the right studs on top + sockets on bottom. The studs are the
"interface"; what's inside the brick (color, material, hollow
or solid) is its own business.

## What "static dispatch" means

You wrote a function:

```vani
fn print_area(s: Shape) -> i64 {  // <- what does Shape mean here?
  print s.area();
  return 0;
}
```

There's a question hidden in this signature: **which `Shape`**?
A Circle? A Square? Triangles aren't even mentioned yet but
they could be.

vāṇी (and Rust) offer two answers:

### Answer 1: static dispatch

The compiler asks the call site:

> "You're calling `print_area`. What's the actual type of `s`?"

If the answer is "Circle", the compiler MAKES A COPY of
`print_area` with `Shape` replaced by `Circle` everywhere
inside. The body's `s.area()` calls the Circle-specific `area`
function directly -- no indirection, no vtable. If you call
`print_area(square)` later, it generates ANOTHER copy
specialized to Square.

This is called **monomorphization** -- "making mono-typed
versions". The compiler generates one version per concrete
type you call the generic function with.

Pros:
- The call to `area` is *inlinable* -- the compiler can paste
  the area code right where you called it, eliminating the
  function-call overhead entirely.
- Fastest possible runtime -- no indirection, no vtable.

Cons:
- More compiled binary code (one copy per type).
- All types must be known at compile time.

### Answer 2: dynamic dispatch (`dyn`)

The compiler keeps `print_area` as a SINGLE function. The
parameter is `dyn Shape` (the two-pointer package from chapter
04a). At each call, the function follows the vtable pointer to
find the right `area` function for whichever shape happens to
be inside.

Pros:
- One compiled function works for all types.
- Heterogeneous collections (`Vec<dyn Shape>`).
- New shape types can be plugged in without recompiling
  `print_area`.

Cons:
- One indirect call per `area` invocation -- small but real.
- No inlining.

## How vāṇी spells each

```vani
// Static dispatch -- compiler monomorphizes per call type
fn print_area<T>(s: T) -> i64 where T is Shape {
  print s.area();
  return 0;
}

// Dynamic dispatch -- one function for all types
fn print_area(s: dyn Shape) -> i64 {
  print s.area();
  return 0;
}
```

The first form uses **generics** (`<T>`) plus a **bound**
(`where T is Shape`). The generic says "T can be anything";
the bound says "but it must be a type that implements Shape."

The second form uses `dyn Shape` directly.

## Choosing between them

A practical rule of thumb:

| Situation | Use |
|---|---|
| Tight inner loop calling the method millions of times | Static (inlines) |
| Heterogeneous collection (`Vec<dyn Shape>`) | Dynamic |
| Library plugin point -- users provide their own impls | Dynamic |
| Function with a single Shape parameter that's a known type | Static |
| Function with a single Shape parameter that's "any Shape, decided later" | Either works |
| Need to call the same function on Circle and Square in one call site | Either; static makes two copies, dynamic makes one |

When in doubt: **start with static**. It's the default, faster,
more predictable. Switch to `dyn` when you hit the cases that
need it (heterogeneous collections; plugins).

## The "interface" as documentation

Interfaces serve a documentation purpose distinct from their
dispatch role.

If you see a function signature `fn foo<T>(x: T) -> i64 where
T is Hashable + Comparable`, you immediately know:
- `x` can be any type, BUT
- It must support hashing AND comparison.

The compiler enforces this -- you can't pass a type that
doesn't implement those interfaces. The function body can
freely call `x.hash()` and `x.cmp(...)` knowing they're
provided.

This is structural typing in disguise. The contract is
explicit; the compiler checks it.

## "implement Iface for T"

For a type `T` to satisfy interface `I`, you write:

```vani
implement Shape for Circle {
  fn area(self: Circle) -> i64 { return self.r * self.r; }
}
```

The block contains every method `Shape` requires. If you miss
one, the compiler rejects with "missing method `area` in
implement Shape for Circle". If you provide an extra one not
in the interface, it's just a regular method (not part of the
contract).

This is opt-in. You don't get an interface "automatically"
because your type happens to have an `area` method -- you have
to declare the implementation explicitly. The opt-in stops
accidental conformance (e.g., two types both with `clone`
methods accidentally treated as "Cloneable").

## Multi-interface types

A single type can implement many interfaces:

```vani
struct Point { x: i64, y: i64 }

implement Shape for Point { ... }
implement Cloneable for Point { ... }
implement Hashable for Point { ... }
```

Now `Point` works anywhere a `Shape` OR `Cloneable` OR
`Hashable` is expected.

When a function bounds on multiple interfaces (`where T is
Shape + Cloneable`), `T` must implement BOTH.

## No derive -- Eq (and other traits) are always hand-written

vāṇी has no `#[derive(Eq)]` / `#[derive(Debug)]` / `#[derive(Clone)]`.
Every interface implementation, including the ones that would be a
one-line attribute in many other languages, is a real `implement`
block you write yourself:

```vani
struct Point { x: i64, y: i64 }

interface Eq { fn eq(self: Point, other: Point) -> bool; }
implement Eq for Point {
  fn eq(self: Point, other: Point) -> bool {
    return self.x == other.x && self.y == other.y;
  }
}
```

For a non-Copy struct (one owning a `Vec`, `OwnedStr`, `Box<T>`, etc.)
the receiver has to be a reference instead, since the value can't be
taken by copy. Note there's no built-in `==` for `Vec<T>` either --
that gets hand-written too, element by element:

```vani
struct BigInt { limbs: Vec<i64>, sign: i64 }

interface Eq { fn eq(self: ref BigInt, other: ref BigInt) -> bool; }
implement Eq for BigInt {
  fn eq(self: ref BigInt, other: ref BigInt) -> bool {
    if self.sign != other.sign { return false; }
    if len(self.limbs) != len(other.limbs) { return false; }
    let i: i64 = 0;
    while i < len(self.limbs) as i64 {
      if self.limbs[i] != other.limbs[i] { return false; }
      i = i + 1;
    }
    return true;
  }
}
```

(This is the same shape as the real `implement Eq for BigInt` in the
`vani-bignum` Kosh package, which compares magnitude digit-by-digit
via its own `bn_cmp` rather than field equality directly.)

**Why no derive**: it's the same "opt-in only" philosophy this
chapter already covers for interfaces generally -- a type never gains
a capability just because its shape happens to match one. Generating
that opt-in block automatically from an attribute would be implicit
magic vāṇी deliberately avoids elsewhere too (no operator overloading
outside the fixed `Eq` hook, no implicit type coercion).

**Is this a real cost?** Yes, in raw lines -- but not in what's
*possible*: everything a derive macro would generate is directly
expressible by hand, just spelled out. The boilerplate scales with
struct count × trait count, which is worth knowing about if you're
designing a library with many small structs that all need `Eq`, but
it's not a blocker for anything. See
[`docs/missing_features.md`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/docs/missing_features.md#attribute-macros-derivedebug)
for the fuller writeup.

## A summary you can carry

- An **interface** is a contract: "any type calling itself X
  must provide these methods."
- **Static dispatch**: compiler generates a copy of the
  generic function per concrete type. Fast, inlinable. Use
  when the type is known at compile time.
- **Dynamic dispatch** (`dyn Iface`): one function, vtable
  lookup per call. Slower but supports heterogeneous
  collections + plugins.
- Default to static; switch to dynamic when you need it.
- Implementations are opt-in via explicit `implement Iface
  for Type` blocks.

That's interfaces. The next chapter ([Intermediate 4](04_generics_iface.md))
shows the syntax in code; [Intermediate 5](05_dyn.md)
shows the dynamic-dispatch variant.

## Cross-reference

- [Beginner 6a -- pointers/references](../beginner/06a_pointers_refs_primer.md)
- [Beginner 6c -- ownership/move](../beginner/06c_ownership_primer.md)
- [Intermediate 3a -- Box/RAII](03a_box_raii_primer.md)
- [Intermediate 4 -- Generics and interfaces](04_generics_iface.md)
  -- the actual syntax + worked example
- [Intermediate 4a -- `dyn Iface` primer](04a_dyn_iface_primer.md)
  -- the dynamic-dispatch counterpart
- [Intermediate 5 -- Dynamic dispatch](05_dyn.md) -- `Vec<dyn Shape>`
  worked example


---

**Previous**: [Sec.4a -- What's a dyn Iface? primer ->](04a_dyn_iface_primer.md)
**Next**: [Sec.4c -- Generics and monomorphization primer ->](04c_generics_primer.md)

