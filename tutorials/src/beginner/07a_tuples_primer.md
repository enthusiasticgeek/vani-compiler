# Beginner 7a — Tuples and destructuring (intuition primer)

> **Learning goal**: build a mental model of "tuple" — the
> simplest way to group a few values without inventing a
> struct — and "destructuring" — pulling the pieces back out
> at use sites. Reading order: this is short + foundational;
> read it any time after [Beginner 7 Vec + arrays](07_vec_arrays.md).

This chapter has **no compiler code**. Pure intuition.

## When a struct feels like overkill

You have a function that returns TWO values. The minimum.
Let's say "the quotient AND the remainder from dividing":

```
fn divmod(a: i64, b: i64) -> ???
```

Three ways to handle the "two return values":

### Option 1: a struct

```rust
struct DivMod { q: i64, r: i64 }

fn divmod(a: i64, b: i64) -> DivMod {
  return DivMod { q: a / b, r: a % b };
}
```

Works. But you had to *invent a type name* (`DivMod`) just to
hold a pair of i64s, and you have to remember it at every call
site. For a one-off return, this feels like ceremony.

### Option 2: two out-params (older C style)

```rust
fn divmod(a: i64, b: i64, out_q: mut ref i64, out_r: mut ref i64) -> i64 { ... }
```

Awful. Caller has to declare two variables BEFORE the call,
then read them after. Ugly + error-prone.

### Option 3: a tuple

```rust
fn divmod(a: i64, b: i64) -> (i64, i64) {
  return (a / b, a % b);
}

fn main() -> i64 {
  let (q, r) = divmod(17, 5);   // ← destructure the tuple
  print "q =", q, "r =", r;
  return 0;
}
```

The return type is `(i64, i64)` — an anonymous pair. No name
needed; the type is its shape. The call site uses
**destructuring** to pull the two pieces into individual
variables in one step.

For "I just need to return two/three values without inventing
a type", this is the sweet spot.

## What a tuple IS

A tuple is an anonymous, fixed-size, heterogeneous collection.
Three properties:

- **Anonymous**: no name. The type IS the shape `(i64, i64)`.
- **Fixed-size**: a `(i64, bool)` is exactly 2 elements. Not
  3, not 1. The size is part of the type.
- **Heterogeneous**: components can be different types.
  `(i64, OwnedStr, bool)` mixes three types.

Compare:
- A struct is *named* and heterogeneous. Same shape, but
  you've committed to a name.
- An array `[T; N]` is anonymous but *homogeneous* (all same
  type).
- A Vec is anonymous, dynamic-size, and homogeneous.

Tuples fill the "anonymous + fixed-size + heterogeneous" slot.

## When to use a tuple vs a struct

The deciding question: **will more than one piece of code use
this shape, and is the meaning of each component
self-evident?**

If yes → struct. Naming the type AND the fields adds
documentation. `struct Point { x: i64, y: i64 }` reads better
than `(i64, i64)` because future-you knows which slot is x.

If no (one-off return, the meaning is obvious from context)
→ tuple. `(quotient, remainder)` is clear enough.

A practical rule of thumb: if the tuple has more than 3
components OR if any component has a non-obvious meaning,
upgrade to a struct.

## Destructuring — pulling the pieces out

You've seen `let (q, r) = divmod(17, 5);`. This is
**destructuring binding**: declare multiple variables in one
let by matching the tuple's shape.

The same pattern shows up in function arguments AND match
arms:

### As function arguments

```rust
fn distance(p: (i64, i64), q: (i64, i64)) -> i64 {
  let (px, py) = p;
  let (qx, qy) = q;
  return (px - qx) * (px - qx) + (py - qy) * (py - qy);
}
```

Two tuple arguments, each destructured into two locals. The
function body then operates on the components.

### In match arms

```rust
match position {
  (0, 0) then "origin",
  (0, _) then "y-axis",
  (_, 0) then "x-axis",
  (x, y) then "general point",
}
```

The match arms each destructure differently. Wildcards `_` say
"don't bind this slot." The last arm `(x, y)` catches
everything else and binds the components.

### Indexed access — `.0` / `.1` / `.2`

You can also access individual slots without destructuring:

```rust
let pair: (i64, OwnedStr) = (42, "answer");
let n: i64 = pair.0;          // = 42
let s: OwnedStr = pair.1;     // = "answer" (moved out)
```

Useful when you only want one field. Less common than full
destructuring in idiomatic code.

## Tuples + ownership

Tuples follow the same ownership rules as any other type:

- A tuple of Copy types is itself Copy. `(i64, bool)` copies
  on assignment.
- A tuple containing a non-Copy type (like `OwnedStr`) is
  non-Copy. It moves on assignment.
- Partial moves work field-by-field, same as structs.

```rust
let pair: (i64, OwnedStr) = (42, "hi" + "!");
let answer: i64 = pair.0;      // copy — pair.0 is i64
let msg: OwnedStr = pair.1;    // move — pair.1 is OwnedStr,
                                // now pair.1 is moved
```

After the move, `pair.0` is still readable; `pair.1` is not.

## Common shapes in practice

- **Two-value return**: `(i64, i64)` for `divmod`,
  `(bool, OwnedStr)` for "did-it-work + reason".
- **Coordinate pair**: `(i64, i64)` for a point on a grid.
  (Many real programs upgrade this to a `Point` struct once
  it gets used widely.)
- **Map iteration**: iterating a HashMap yields `(key, value)`
  tuples per entry.
- **Reduction "carry"**: a fold with multiple accumulators
  uses a tuple for the state. `fold((0, 0), |(sum, count),
  x| (sum + x, count + 1))`.

## When NOT to use a tuple

- **More than 3 fields**. Becomes hard to remember which slot
  is which. Use a struct.
- **Self-similar slots that aren't obviously different**.
  `(i64, i64, i64, i64)` for "rectangle bounds" loses the
  meaning of each slot.
- **Anything part of a public API**. A user reading
  `fn lookup(k: K) -> (V, i64)` has to guess what the `i64`
  represents. Name it.

## A summary you can carry

- A **tuple** is an anonymous, fixed-size, heterogeneous
  group of values: `(i64, OwnedStr, bool)`.
- **Destructuring** (`let (a, b) = tuple;`) pulls the
  components into individual bindings in one step. Works in
  lets, fn params, and match arms.
- Slot access via `.0` / `.1` / `.2` for picking just one
  component without full destructuring.
- Tuples follow the same ownership rules as other types:
  Copy if all components Copy, move-by-default otherwise,
  partial-moves field-by-field.
- Default to a **struct** when more than a couple components
  OR when the meaning of components needs naming. Use tuples
  for quick one-off groupings.

That's tuples. The intermediate-track chapter ([Intermediate 7](../intermediate/07_tuples.md))
covers the actual syntax + a few worked examples.

## Cross-reference

- [Intermediate 7 — Tuples and tuple destructure](../intermediate/07_tuples.md)
  — syntax + worked examples
- [Beginner 8a — Pattern matching primer](08a_pattern_match_primer.md)
  — destructuring extends naturally to match patterns
- [Beginner 6c — Ownership primer](06c_ownership_primer.md)
  — tuples follow the same move-vs-copy rules as other
  types; partial moves apply field-by-field
