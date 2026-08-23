# Beginner 7b -- Type aliases: `type X = Y;` (primer)

> **Learning goal**: give an existing type a second, more meaningful
> name -- a nickname that documents intent without creating a new
> type underneath. Reading order: [Beginner 7 -- Arrays and
> `Vec<T>` basics](07_vec_arrays.md) -> here -> [Beginner 8a --
> Pattern matching primer](08a_pattern_match_primer.md).

## A nickname, not a new type

You probably know someone whose full name is "Alexander" but everyone
calls them "Alex." It's not a different person, not a different legal
name on paper -- just a shorter, friendlier label everyone agrees to
use in conversation. `type Alias = RealType;` is exactly that for a
type: after the declaration, `Alias` and `RealType` mean the *exact
same thing* to the compiler -- same size, same layout, same every
operation that works on one works on the other -- but `Alias` can
carry meaning in your code that `RealType` alone doesn't.

```vani
type Score = i64;

fn add_one(s: Score) -> Score {
  return s + 1;
}

fn main() -> i64 {
  let final_score: Score = add_one(41);
  print final_score;
  return 0;
}
```

`Score` is not a new, distinct type with its own rules -- it's `i64`,
full stop, everywhere. `add_one`'s signature could just as legally
read `fn add_one(s: i64) -> i64`. The only thing `type Score = i64;`
buys you is that a reader (including future-you, six months from now)
sees `fn add_one(s: Score) -> Score` and immediately knows *what kind
of number* this function is about, instead of "some i64, meaning
unclear" -- the same benefit a good variable name gives you, applied
to a type instead.

## Why bother, if it's "just" i64?

Three real reasons programmers reach for this:

1. **Self-documenting signatures.** `fn distance(a: Coord, b: Coord)
   -> Meters` reads its own contract; `fn distance(a: (i64, i64), b:
   (i64, i64)) -> i64` makes you go read the function body (or its
   comments) to find out what the return value even represents.
2. **One place to change your mind.** If `Score` needs to become
   `f64` later (say, fractional bonus points), you edit one `type`
   line -- every `Score`-typed signature in the program picks up the
   new underlying type automatically, instead of a find-and-replace
   across every `i64` that happened to mean "score."
3. **Naming something that doesn't have its own name yet.** A tuple
   like `(i64, i64)` is anonymous -- `type Coord = (i64, i64);` gives
   it an identity worth referring to, without the ceremony of
   declaring a full `struct` if a plain tuple is all the shape you
   need.

## What you can alias

Any type: a primitive, a tuple, an enum, a `Vec<T>`, or a `struct`.

```vani
type Coord = (i64, i64);              // a tuple
enum Color { Red, Green, Blue }
type Hue = Color;                      // an enum
type IntList = Vec<i64>;               // a generic instantiation
```

```vani
fn first(c: Coord) -> i64 { return c.0; }

fn pick(h: Hue) -> i64 {
  return match h {
    Color.Red then 1,
    Color.Green then 2,
    Color.Blue then 3,
  };
}

fn main() -> i64 {
  let p: Coord = (3, 4);
  print first(p);
  print pick(Color.Green);

  let xs: IntList = vec(1, 2, 3);
  print xs[0];
  return 0;
}
```

Notice `pick` matches on `Color.Red`/`Color.Green`/`Color.Blue`, not
`Hue.Red` -- the alias never creates new variant names or a new
enum identity, only a second name for the type `Hue` already IS,
which is `Color`.

### The one sharp edge: struct literals need the real name

An alias works everywhere a type is *named* -- parameter types,
return types, `let` annotations -- but **not** for constructing a
struct literal. The literal must spell the struct's real name:

```vani
struct Point { x: i64, y: i64 }
type Position = Point;

fn origin() -> Position {
  return Point { x: 0, y: 0 };   // <- must say `Point { ... }`, not `Position { ... }`
}

fn main() -> i64 {
  let p: Position = origin();     // <- but the TYPE annotation can say `Position`
  print p.x;
  print p.y;
  return 0;
}
```

`Position { x: 0, y: 0 }` is rejected -- struct-literal construction
looks up a real, declared struct, and an alias doesn't register as
one. Every other position (parameters, return types, `let`
annotations, generic arguments) accepts the alias freely. Once you
know the rule it's not a hardship -- write the alias in signatures
and variable declarations, and the real struct name at the one spot
you're building the value.

## What aliasing does NOT do

- **No new methods, no new identity.** `type Score = i64;` doesn't
  let you attach `methods on Score { ... }` that `i64` itself doesn't
  have -- it's not a wrapper, there's nothing to hang extra behavior
  off. If you need that, you want a `struct Score { value: i64 }`
  (a real new type, one field) instead of an alias.
- **No type-checking benefit between two different aliases of the
  same underlying type.** `type Score = i64; type Age = i64;` -- a
  function expecting `Score` will happily accept a value that started
  life as an `Age`, because to the checker they're both just `i64`.
  If you need the compiler to catch "don't pass an Age where a Score
  is expected," you need distinct `struct`s, not aliases -- aliasing
  is a *readability* tool, not a type-safety boundary.

## Rules the compiler enforces

- **No cycles.** `type A = B; type B = A;` is rejected --
  `recursive type alias 'A' is not allowed in v1` -- an alias must
  bottom out at a real, concrete type.
- **No duplicate names**, same as any other top-level declaration --
  `type X = i64; type X = f64;` is a compile-time "already declared"
  error.
- **Chains resolve fully.** `type Inner = i64; type Middle = Inner;
  type Outer = Middle;` -- `Outer` fully resolves to `i64` through
  the whole chain; nothing about using `Outer` in a signature looks
  or behaves any different from writing `i64` directly.

## Try it yourself

Give a tuple-based `(f64, f64)` coordinate pair a name (`type
Point2D = (f64, f64);`), write a function that computes the distance
between two of them, and confirm the signature reads better than the
raw tuple type would. Then try writing a two-step cycle (`type A =
B; type B = A;`) and read the compiler's rejection.

## Summary

- `type Alias = RealType;` is a second name for an existing type --
  identical size, layout, and behavior, purely for readability and a
  single point of change.
- Works for primitives, tuples, enums, `Vec<T>`, and structs.
- Struct literals must use the real struct name; every other
  position (params, returns, `let` annotations) accepts the alias.
- Not a type-safety boundary -- two aliases of the same underlying
  type are freely interchangeable to the checker. Reach for a real
  `struct` instead if you need the compiler to keep two "kinds" of
  `i64` from being mixed up.
- Cycles and duplicate names are compile-time errors.

---

**Previous**: [Sec.7 -- Arrays and `Vec<T>` basics ->](07_vec_arrays.md)
**Next**: [Sec.8a -- Pattern matching primer ->](08a_pattern_match_primer.md)
