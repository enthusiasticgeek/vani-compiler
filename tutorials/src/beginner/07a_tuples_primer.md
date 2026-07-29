# Beginner 7a -- Tuples and destructuring (intuition primer)

> **Learning goal**: build a mental model of "tuple" -- the
> simplest way to group a few values without inventing a
> struct -- and "destructuring" -- pulling the pieces back out
> at use sites. Reading order: this is short + foundational;
> read it any time after [Beginner 7 Vec + arrays](07_vec_arrays.md).

This chapter has **no compiler code**. Pure intuition.

## The combo-meal tray

Picture a fast-food combo tray with three fixed molded slots:
burger goes in the left slot, fries in the middle dip, drink in
the round cutout on the right. Three things about that tray:

- **The slots are fixed in number.** You can't cram a fourth item
  in -- there's no fourth slot. If your order only has two items,
  one slot sits empty; the tray shape itself doesn't change.
- **The slots don't need labels.** Nobody printed "BURGER" under
  the left slot. Its *position* -- leftmost -- is what tells you
  what belongs there. You know a drink goes in the round cutout
  because of where it is, not because it says so.
- **Each slot can hold a different kind of thing.** The burger
  slot holds a burger-shaped item, the drink slot holds a
  cup-shaped item. It's not a tray of three identical fries
  boxes -- the slots are shaped differently on purpose.

A **tuple** is exactly that tray, but for values in your program: a
fixed number of slots, in a fixed order, where each slot can hold a
different type, and the slot's *position* -- not a name -- tells you
what it means. `(i64, i64)` is a two-slot tray where both slots
happen to hold the same kind of thing (like a two-drink tray);
`(OwnedStr, i64, bool)` is a three-slot tray holding three different
kinds of things.

The moment you'd want to print a label under a slot -- "this one's
specifically the quotient, that one's specifically the remainder" --
that's the sign you've outgrown the tray and want a **struct**
instead, where every slot gets an actual name.

## When a struct feels like overkill

You have a function that returns TWO values. The minimum.
Let's say "the quotient AND the remainder from dividing":

```
fn divmod(a: i64, b: i64) -> ???
```

Three ways to handle the "two return values":

### Option 1: a struct

```vani
struct DivMod { q: i64, r: i64 }

fn divmod(a: i64, b: i64) -> DivMod {
  return DivMod { q: a / b, r: a % b };
}
```

Works. But you had to *invent a type name* (`DivMod`) just to
hold a pair of i64s, and you have to remember it at every call
site. For a one-off return, this feels like ceremony.

### Option 2: two out-params (older C style)

```vani
fn divmod(a: i64, b: i64, out_q: mut ref i64, out_r: mut ref i64) -> i64 { ... }
```

Awful. Caller has to declare two variables BEFORE the call,
then read them after. Ugly + error-prone.

### Option 3: a tuple

```vani
fn divmod(a: i64, b: i64) -> (i64, i64) {
  return (a / b, a % b);
}

fn main() -> i64 {
  let (q, r) = divmod(17, 5);   // <- destructure the tuple
  print "q =", q, "r =", r;
  return 0;
}
```

The return type is `(i64, i64)` -- an anonymous pair. No name
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

If yes -> struct. Naming the type AND the fields adds
documentation. `struct Point { x: i64, y: i64 }` reads better
than `(i64, i64)` because future-you knows which slot is x.

If no (one-off return, the meaning is obvious from context)
-> tuple. `(quotient, remainder)` is clear enough.

A practical rule of thumb: if the tuple has more than 3
components OR if any component has a non-obvious meaning,
upgrade to a struct.

## Destructuring -- pulling the pieces out

You've seen `let (q, r) = divmod(17, 5);`. This is
**destructuring binding**: declare multiple variables in one
let by matching the tuple's shape.

The same pattern shows up in function arguments; `match` patterns
are a different story (see below).

### As function arguments

```vani
fn distance(p: (i64, i64), q: (i64, i64)) -> i64 {
  let (px, py) = p;
  let (qx, qy) = q;
  return (px - qx) * (px - qx) + (py - qy) * (py - qy);
}
```

Two tuple arguments, each destructured into two locals. The
function body then operates on the components.

### Not in match arms

vāṇी's `match` patterns don't include a tuple form -- `match
position { (0, 0) then "origin", ... }` is a parse error (`match`
only supports variant, literal, wildcard, and slice patterns; see
[Beginner 8a](08a_pattern_match_primer.md)). Destructure the tuple
into named locals first, then branch on those with `if`/`else`:

```vani
fn describe(position: (i64, i64)) -> Str {
  let (x, y) = position;
  if x == 0 && y == 0 {
    return "origin";
  }
  if x == 0 {
    return "y-axis";
  }
  if y == 0 {
    return "x-axis";
  }
  return "general point";
}
```

### Indexed access -- `.0` / `.1` / `.2` (Copy slots only)

You can access an individual slot without destructuring -- but
only when that slot's type is Copy (`i64`, `bool`, `f64`, ...).
A non-Copy slot (`OwnedStr`, a `Vec`, ...) rejects direct `.N`
access, because reading it out that way would alias the tuple's
heap data without transferring ownership:

```vani
let pair: (i64, OwnedStr) = (42, "answer" + "");
let n: i64 = pair.0;          // fine -- i64 is Copy, = 42
// let s: OwnedStr = pair.1;  // error: use destructuring instead
let (_, s) = pair;             // this is how you get the OwnedStr out
```

Useful when you only want one Copy field. For a non-Copy slot,
destructure (`let (_, v) = tuple;`) instead of reaching for `.N`.

## Tuples + ownership

Tuples follow the same ownership rules as any other type:

- A tuple of Copy types is itself Copy. `(i64, bool)` copies
  on assignment.
- A tuple containing a non-Copy type (like `OwnedStr`) is
  non-Copy. It moves on assignment.
- **Unlike structs, tuples don't support true partial moves.**
  A struct's `p.field` access moves just that one field out and
  leaves the struct's *other* fields still usable (`p.a` still
  reads fine after `let x = p.b;`). A tuple has no equivalent:
  `.N` direct access only works on Copy slots at all (a non-Copy
  slot rejects `.N` outright, see above), and destructuring
  (`let (_, v) = tuple;`) -- the only way to pull a non-Copy slot
  out -- consumes the *entire* tuple, including slots you'd
  already read via `.N`. Reach for a struct instead of a tuple if
  you need to move one field while continuing to use the others.

```vani
let pair: (i64, OwnedStr) = (42, "hi" + "!");
let answer: i64 = pair.0;      // copy -- fine via `.0`, doesn't
                                // consume `pair`; read pair.0 as
                                // many times as you like before
                                // the next line
let (_, msg) = pair;           // move -- pair.1 is OwnedStr, so it
                                // has to come out via destructuring,
                                // not `.1` (see the note above);
                                // this consumes ALL of `pair`,
                                // including the already-copied
                                // slot 0 -- `pair` (and `pair.0`)
                                // is unusable after this line
```

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

## Variations -- tuples aren't just `(i64, i64)`

The shape `(T1, T2, T3, ...)` works for ANY types, not just
primitives. A few non-trivial examples to fix the mental
model:

### Tuple of structs

```vani
struct Player { name: OwnedStr, score: i64 }
struct Enemy  { kind: i64, hp: i64 }

fn next_encounter() -> (Player, Enemy) {
  return (Player { name: "alice" + "", score: 0 },
          Enemy  { kind: 7, hp: 100 });
}

let (me, foe) = next_encounter();
print "vs", foe.kind, "at", foe.hp, "HP";
```

The tuple is `(Player, Enemy)`. Each slot is a struct;
destructuring binds each to its own local. Ownership rules
apply per-component: `me` owns the Player; `foe` owns the
Enemy.

### Nested tuples

```vani
let line: ((i64, i64), (i64, i64)) = ((0, 0), (10, 5));
let (start, end) = line;
let (sx, sy) = start;
let (ex, ey) = end;
```

A pair of pairs. Destructure in stages: outer first, then
each inner, as above. A single nested pattern in one `let` --
`let ((sx, sy), (ex, ey)) = line;` -- is *not* supported (`let`
destructuring patterns are one level deep only); staged
destructuring is the only way to unpack a nested tuple.

### Tuple containing a `Box`

```vani
struct BigData { ... }   // imagine this is 4 KB

fn make_pair() -> (i64, Box<BigData>) {
  let id: i64 = 42;
  let payload: Box<BigData> = box(BigData { ... });
  return (id, payload);
}
```

The i64 lives in the tuple directly (8 bytes). The
`Box<BigData>` is also in the tuple but is just an 8-byte
pointer -- the 4 KB lives separately on the heap. The whole
tuple's stack footprint is 16 bytes; the actual data is
out-of-band.

### Tuple containing a `Vec`

```vani
fn parse_line(s: Str) -> (Vec<i64>, OwnedStr) {
  let nums: Vec<i64> = vec();    // imagine parsing
  let leftover: OwnedStr = "rest" + "";
  return (nums, leftover);
}
```

Two heap-owning values returned as a single tuple. Each owns
its own heap allocation; when the destructured locals go out
of scope, BOTH are freed.

### Tuple as a struct field

```vani
struct Rectangle {
  top_left: (i64, i64),
  bottom_right: (i64, i64),
}
```

You can use tuples inside structs when you want a quick
multi-field group without inventing a sub-struct. Compare to
naming each: `top_left_x`, `top_left_y`, `bottom_right_x`,
`bottom_right_y` -- verbose.

### Tuple inside a Vec

```vani
let coords: Vec<(i64, i64)> = vec((0, 0), (3, 4), (10, 0));
for pt in ref coords {
  let (x, y) = pt;
  print x, y;
}
```

A Vec of pairs -- common pattern for storing many (key, value)
or (timestamp, sample) pairs. Each tuple is its own heap-free
two-i64 unit; the Vec's heap buffer holds Nx16 bytes
contiguously.

### Mixed types in the same tuple

```vani
fn snapshot() -> (OwnedStr, i64, bool, Vec<i64>) {
  return ("snapshot-1" + "", 1234, true, vec(10, 20, 30));
}
```

Four-component tuple mixing String + int + bool + Vec. Works
fine; destructuring + ownership rules apply per-slot. (Reaching
4 components is around the upper limit where "should I just
make a struct?" becomes the right question.)

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

- [Intermediate 7 -- Tuples and tuple destructure](../intermediate/07_tuples.md)
  -- syntax + worked examples
- [Beginner 8a -- Pattern matching primer](08a_pattern_match_primer.md)
  -- destructuring extends naturally to match patterns
- [Beginner 6c -- Ownership primer](06c_ownership_primer.md)
  -- tuples follow the same move-vs-copy rules as other
  types; partial moves apply field-by-field


---

**Previous**: [Sec.6d -- Program memory layout primer ->](06d_memory_sections_primer.md)
**Next**: [Sec.7 -- Arrays and `Vec<T>` basics ->](07_vec_arrays.md)
