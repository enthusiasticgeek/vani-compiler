# Intermediate 3e -- Lifetimes and reference returns (intuition primer)

> **Learning goal**: understand vāṇी's "implicit lifetimes"
> model -- how `fn foo(p: ref T) -> ref T` works without
> mentioning `'a` anywhere, when it accepts your code, and
> when it doesn't. Reading order: [Intermediate 3 -- Affine
> ownership](03_affine.md), [Intermediate 3b -- Affine
> deeper](03b_affine_deeper_primer.md), and [Intermediate 3d --
> Cyclic references](03d_cyclic_references_primer.md).

This chapter has **no compiler code**. Pure intuition.

## The borrowed ladder

Your neighbor owns a tall extension ladder. You're painting your
fence and need it for the afternoon, so you knock on their door and
ask to borrow it. They say yes, wheel it over, and you get to work.
While you're using it, one rule is obvious without anyone stating it
out loud: the ladder is still THEIRS. You didn't buy it, you can't
sell it, and your ability to keep using it is entirely tied to your
neighbor's continued ownership of it. If your neighbor still owns
the ladder and hasn't asked for it back, you're fine. The moment
they sell the ladder, or it burns up in a garage fire, or they
simply want it back -- your use of it ends too. You never had an
independent claim on the ladder; you had a claim that rides
piggyback on THEIRS.

Here's the situation that makes this interesting: suppose, halfway
through painting, a second neighbor leans over the fence and asks if
THEY can borrow the ladder from you "indefinitely, for as long as
they need it, no questions asked." Can you promise them that? No --
and not because you're being stingy. You genuinely don't have the
authority to make that promise. Your own right to the ladder is only
as solid as your neighbor's ownership of it, and you have no idea
how long that will last. The most honest thing you can say is "you
can use it as long as I still have it, and I can't guarantee how
long that is." Passing along a borrowed thing doesn't reset the
clock on it or grant it a fresh, independent lifetime -- it just
extends the SAME clock one link further, and that clock is still
ultimately set by whoever owns the ladder.

Now picture the failure case: imagine you tell the second neighbor
"sure, keep it as long as you like," and then, that same evening,
the FIRST neighbor sells the ladder to a scrap dealer who hauls it
away. The second neighbor shows up the next morning expecting a
ladder that no longer exists anywhere -- they're holding a promise
about a thing that's already gone. That's a dangling reference: a
"you can use this" claim that outlived the actual thing it pointed
to.

This is exactly the shape of a reference in vāṇी. A `ref T` is a
borrowed ladder -- temporary access to a value you don't own, valid
only as long as the actual owner still has it. A function that hands
back a `ref` is like you telling someone "here, use my neighbor's
ladder" -- and the compiler's whole job in this chapter is making
sure nobody ever gets handed a ladder-promise that outlives the
ladder. When the chapter talks about a reference's "lifetime" being
tied to its source, that's just: your right to the ladder can never
be longer than your neighbor's ownership of it. And when it talks
about a function rejected for "returning a reference with nothing to
borrow from" -- that's the neighbor who tries to lend out a ladder
they never actually had in the first place.

## What problem lifetimes solve

You learned that references are second-class -- they're
borrows, not owners, and the compiler tracks the scope of
every borrow so it can never outlive its source. So far,
all your examples kept refs as parameters: a function takes
a `ref T`, reads through it, and returns by value.

What if a function wants to **return a reference**?

```vani
struct Point { x: i64, y: i64 }

fn shared(p: ref Point) -> ref Point {
  return p;
}
```

The function gets `p` (a ref into the caller's Point), and
hands the same ref back. The caller does:

```vani
let pt: Point = Point { x: 3, y: 4 };
let r: ref Point = shared(ref pt);
print area(r);  // <- r points at pt; pt is still alive
```

Question: how does the compiler know that `r`'s lifetime is
bounded by `pt`'s? The signature `fn shared(p: ref Point)
-> ref Point` doesn't mention `pt` -- it's the caller's
variable, invisible to the function.

The answer is **lifetime elision**: the compiler INFERS the
relationship from the signature shape. The single ref input
is the "source"; the ref output borrows from it. Lifetimes
exist conceptually, but you never type `'a` in vāṇी.

## The elision rule

vāṇी's rule is the simplest possible:

> A function that returns `ref T` (or `mut ref T`) must
> have **exactly one** `ref`/`mut ref` parameter. The
> return ref's lifetime is elided to equal that parameter's.

That's it. One sentence. Three cases:

### Case 1: exactly one ref param -> ✅ accept

```vani
fn first(xs: ref Vec<i64>) -> ref i64 {
  return ref xs[0];
}
```

`xs` is the single ref parameter. The returned `ref i64`
borrows from xs's referent -- the call-site `let r =
first(ref my_vec);` makes `r`'s lifetime equal to `my_vec`'s.

### Case 2: zero ref params -> [x] reject

<img class="manas" src="../images/mascot/manas_mascot_error.png" title="this code does not compile!"/>

```vani
fn make() -> ref i64 {
  let x: i64 = 42;
  return ref x;   // <- x drops on return; the ref would dangle
}
```

Diagnostic:
```
function 'make' returns a reference but has no reference
parameter to borrow from -- the returned ref would dangle.
Add a `ref T` / `mut ref T` parameter, return by value
instead, or use Box<T> for owned heap allocation.
```

This is the classic "returning a pointer to a local"
mistake. C compiles it; vāṇी rejects it at the signature.

### Case 3: two or more ref params -> [x] reject

<img class="manas" src="../images/mascot/manas_mascot_error.png" title="this code does not compile!"/>

```vani
fn pick(a: ref Point, b: ref Point) -> ref Point {
  return a;   // <- a's lifetime? b's lifetime? Compiler can't tell.
}
```

Diagnostic:
```
function 'pick' returns a reference but has 2 reference
parameters ('a', 'b') -- the elision rule needs exactly one
ref parameter to borrow from. Either drop one borrow (make
it a value parameter or remove it), or split into two
narrower functions, one per borrow.
```

Some languages (Rust) handle this with **explicit lifetime
variables** (`fn pick<'a>(a: &'a Point, b: &'a Point) ->
&'a Point`). vāṇी doesn't expose that syntax -- the
designers decided the ergonomics weren't worth it for v1.
The workaround: refactor to use one ref + one value, or
split into two functions -- worked examples below.

## Returning more than one reference

Case 3 covers *two ref params, one ref return*. A related shape
comes up just as often: a return type that packages **more than
one reference together** -- a tuple of refs, or `Option<ref T>`.
Both are rejected too, for the same underlying reason: v1's
elision only understands a bare `ref T` / `mut ref T` return
type, not a reference nested inside another type.

<img class="manas" src="../images/mascot/manas_mascot_error.png" title="this code does not compile!"/>

```vani
struct Pair { x: i64, y: i64 }

fn two_fields(p: ref Pair) -> (ref i64, ref i64) {
  return (ref p.x, ref p.y);
}
```

Diagnostic (verified directly against the compiler):
```
function 'two_fields' returns `(ref i64, ref i64)`, which has
a reference nested inside a tuple/Vec/array/generic type --
vāṇी's v1 lifetime elision only understands a bare `ref T` /
`mut ref T` return type, not a reference nested inside another
type. Split this into separate accessor functions (one bare
`ref` return each), or return the referenced values by
value/clone instead.
```

The same rejection applies to `Option<ref T>` / `Result<ref T,
E>` -- a "maybe-a-ref" return is just as ambiguous to the
elision analyzer as a tuple of refs, even though only one ref
is ever live at a time.

### Workaround 1: split into narrower functions, select at the call site

This is the general-purpose fix, and it composes cleanly with
`if` used as an *expression* (the branches must agree on type):

```vani
struct Point { x: i64, y: i64 }

fn pick_a(a: ref Point) -> ref Point { return a; }
fn pick_b(b: ref Point) -> ref Point { return b; }

fn main() -> i64 {
  let p1: Point = Point { x: 1, y: 2 };
  let p2: Point = Point { x: 9, y: 8 };
  let want_a: bool = true;
  let chosen: ref Point =
    if want_a { pick_a(ref p1) } else { pick_b(ref p2) };
  print "chosen.x:", chosen.x;   // 1
  return 0;
}
```

Each function satisfies the single-ref-param rule on its own;
the `if`-expression at the call site picks which one runs, and
the result is still a zero-copy `ref Point` -- no cloning needed.
This is exactly the shape the Case 3 diagnostic's "split into two
narrower functions" advice means in practice.

### Workaround 2: one ref param over a collection, return by value

When the candidates already live in a `Vec`, pass the whole `Vec`
as the single ref param and return the selected element **by
value** instead of by ref (v1 can't return a ref into a Vec slot
-- see the next section):

```vani
struct Point { x: i64, y: i64 }

fn pick(pts: ref Vec<Point>, want_first: bool) -> Point {
  if want_first {
    return pts[0];
  }
  return pts[1];
}

fn main() -> i64 {
  let pts: Vec<Point> = vec(Point { x: 1, y: 2 }, Point { x: 9, y: 8 });
  let chosen: Point = pick(ref pts, false);
  print "chosen.x:", chosen.x;   // 9
  return 0;
}
```

Pick Workaround 1 when you want zero-copy and can name the
candidates as separate bindings/functions; pick Workaround 2 when
the candidates are already indexed and a small `Point`-sized copy
is cheap enough not to matter.

## A real v1 gap: `ref` to a bare scalar has no read-back path

Everything above works because the ref's *referent* is a
`struct` (or a `Vec`) -- reading through it is `.field` / `[i]`
access, which yields a plain, printable value. Verified directly:
a `ref` whose referent is a **bare scalar** (`ref i64`, `ref
f64`, `ref bool`, `ref OwnedStr`) is a dead end in v1 -- there is
no deref operator, and none of arithmetic, comparison, `print`,
plain assignment, or a `let` into an unref'd type will read
through it:

<img class="manas" src="../images/mascot/manas_mascot_error.png" title="this code does not compile!"/>

```vani
fn double(x: ref i64) -> i64 {
  return x * 2;     // error: left operand must be numeric, got ref i64
}
```

```vani
let r: ref i64 = ref some_i64;
print "r:", r;       // error: cannot print a reference directly
let v: i64 = r;       // error: let initializer must be assignable to i64, got ref i64
```

Writing through a `mut ref i64` doesn't work either -- plain
assignment to a `mut ref i64`-typed binding is rejected the same
way. The only thing you can legally do with a bare-scalar ref is
pass it *onward* as an argument to another function that also
declares a `ref i64` (or `mut ref i64`) parameter -- which just
moves the dead end, it doesn't resolve it.

**The practical rule**: only return/parameterize `ref`/`mut ref`
down to *struct* or *`Vec`* granularity, never down to a bare
scalar. If you need a scalar out, either return it **by value**
(cheap for `i64`/`f64`/`bool` -- they're `Copy`) or keep it behind
a struct-level ref and let the caller do the final `.field` read,
same as every working example in this chapter.

(The one place v1 *does* let you read/write through a scalar-level
reference is the dedicated `region_borrow_i64` / `aref_load` /
`aref_store` builtins over `ArenaRef<i64>` -- a different, purpose-
built mechanism, not the general `ref`/`mut ref` syntax. See
[Advanced 4 -- Embedded](../advanced/04_embedded.md).)

## How the lifetime threads through

The call site is where the magic happens. When you write:

```vani
let pt: Point = Point { x: 3, y: 4 };
let r: ref Point = shared(ref pt);
```

The compiler records that `r` borrows from `pt` -- internally
it stores a tiny "this ref aliases these source bindings"
list per binding. Subsequent uses of `r` are checked as if
they were uses of `ref pt`:

- Pushing `r` into a Vec? The Vec's scope must include `pt`.
- Storing `r` in a struct field? The struct's scope must
  include `pt`.
- Returning `r` from another function? That fn's signature
  must have a ref parameter whose source-binding outlives
  the return path.

The tracking is automatic. You write code that LOOKS like
"the lifetime is implicit" -- and the compiler's analyzer
makes the implicit explicit at every use site.

### Chaining ref-returning calls

```vani
fn shared(p: ref Point) -> ref Point { return p; }

let pt: Point = ...;
let r1: ref Point = shared(ref pt);   // r1 borrows from pt
let r2: ref Point = shared(r1);       // r2 borrows from pt (via r1)
let r3: ref Point = shared(r2);       // r3 borrows from pt
```

Each call propagates the source. r3 ultimately borrows from
pt. If pt drops, every dependent binding is invalidated --
the analyzer rejects subsequent uses with a "would dangle"
diagnostic.

### Field-ref returns

```vani
fn x_ref(p: ref Point) -> ref i64 {
  return ref p.x;
}
```

The return is `ref p.x` -- a field-borrow of the parameter.
Same rule: result's lifetime = `p`'s lifetime. The caller's
`let n: ref i64 = x_ref(ref my_point);` makes `n` borrow
from `my_point`.

## What you CAN'T do (path-D territory)

The single-param elision rule covers most common cases. A
few advanced shapes are deferred -- they'd need real
lifetime variables, which v1 doesn't ship:

### Multi-input distinct lifetimes

```vani
// Rust: fn pick<'a, 'b>(a: &'a P, b: &'b P) -> &'a P
// vāṇी: not allowed -- only single-ref-param elision.
```

You'd want this if the output borrows from ONE specific input
but the fn takes multiple refs. The vāṇी workaround:
restructure so the fn has only one ref param (pass the others
by value, or call multiple narrower fns).

### Struct fields holding refs in return types

```vani
// Rust: struct View<'a> { x: &'a T }, fn make<'a>(t: &'a T) -> View<'a>
// vāṇी: works for SIMPLE cases via the existing struct-field
// scope-escape analyzer, but explicit-lifetime-in-return-type
// declarations aren't supported.
```

You can write `struct View { x: ref T }` and `fn make(t: ref
T) -> View` -- that works under the elision rule (one ref
param + return shape uses that param's lifetime). What's
not supported is multi-lifetime structs.

### Closures capturing refs that outlive the closure

```vani
// Path D -- closures + lifetimes is genuinely complex.
// vāṇी closures CAN capture by reference (`[ref name]`, see
// Intermediate 6/6a) and can be passed around as real values --
// but a ref-capturing closure can't yet be returned, stored in a
// struct/Vec, or otherwise made to outlive the scope where it
// captured the reference. That specific escape shape is what's
// still rejected here, not by-ref capture itself.
```

This is narrower than it used to be. `[ref name]` closures were
extended in 2026-07-25 to work as genuine values (not just as a
same-scope call-by-name shorthand) with a real non-escape check —
see `docs/ref_capturing_closures_design.md` for the full design and
why the general case (letting one escape to a caller who keeps the
borrowed data alive) still needs real lifetime-variable tracking,
i.e. still needs path-D.

## Why this design

Rust ships explicit lifetime variables (`'a`, `'b`, etc.)
because they unlock advanced patterns. The cost: every
intermediate Rust user has war stories about lifetime
errors that took hours to untangle.

vāṇी's design choice: **elide the easy 90%, reject the
remaining 10% with a clear "use a different shape"
diagnostic.** The 10% includes some genuinely useful
patterns (multi-input distinct lifetimes), but the rejection
is loud and the workaround is mechanical.

The bet: most users don't need the 10%, and those who do
can write the workaround. Future versions may lift the
restriction once we see real-world demand for the missing
shapes.

## Reading vāṇी signatures

When you see:

```vani
fn foo(p: ref T) -> ref U
```

read it as: "the returned `ref U` borrows from the same
source as `p`." The lifetime is implicit but real.

When you see:

```vani
fn foo(p: ref T) -> U
```

read it as: "U is a value; no borrow relationship." The
caller owns whatever comes back.

When you see:

```vani
fn foo(p: T) -> T
```

no refs anywhere -- pure value semantics, ownership transfers
in and out.

## Common patterns

Three shapes you'll see in real code -- all keep the ref at
*struct* granularity, per the gap above, never down to a bare
scalar or an arbitrary Vec slot:

### The struct passthrough

```vani
struct Person { name: OwnedStr, age: i64 }

fn oldest(a: ref Person, b: ref Person) -> ref Person {
  if a.age > b.age {
    return a;
  }
  return b;
}

fn main() -> i64 {
  let alice: Person = Person { name: "Alice" + "", age: 30 };
  let bob: Person = Person { name: "Bob" + "", age: 42 };
  let older: ref Person = oldest(ref alice, ref bob);
  print "older.age:", older.age;   // 42
  return 0;
}
```

Wait -- `oldest` takes *two* ref params. Doesn't Case 3 reject
that? It would, except here the return itself is a value-typed
comparison-then-return of `a` or `b`, each independently a valid
single-source elision -- **this exact shape still needs the
Workaround 1 split** (`pick_a`/`pick_b`-style) to satisfy the
one-ref-param rule; `oldest` as written above is illustrative of
the *goal*, not something the checker accepts as one function. Do
it for real as:

<img class="manas" src="../images/mascot/manas_mascot_success.png" title="this is the correct, working version"/>

```vani
fn person_a(a: ref Person) -> ref Person { return a; }
fn person_b(b: ref Person) -> ref Person { return b; }

fn main() -> i64 {
  let alice: Person = Person { name: "Alice" + "", age: 30 };
  let bob: Person = Person { name: "Bob" + "", age: 42 };
  let older: ref Person =
    if alice.age > bob.age { person_a(ref alice) } else { person_b(ref bob) };
  print "older.age:", older.age;   // 42
  return 0;
}
```

The caller reads `older.age` / `older.name` directly -- struct
field access through a ref yields a plain, printable value (see
the gap above for why this only works at struct granularity, not
for a `ref OwnedStr` returned on its own).

### The index lookup -- returns an index, not a ref

v1 cannot return a ref into an arbitrary Vec slot at all (`ref`
only accepts a named variable or `t.field`, never `xs[i]` --
verified: `return ref xs[i];` is rejected with `'ref' can only
borrow a named variable or a struct field`, regardless of the
element type). The working pattern returns the **index** instead,
and lets the caller do the (cheap, in-bounds) indexing themselves:

<img class="manas" src="../images/mascot/manas_mascot_success.png" title="this is the correct, working version"/>

```vani
fn find_index(xs: ref Vec<i64>, target: i64) -> Option<i64> {
  let i: i64 = 0;
  while i < (xs.len() as i64) {
    if xs[i] == target {
      return Option.Some(i);
    }
    i = i + 1;
  }
  return Option.None;
}

fn main() -> i64 {
  let xs: Vec<i64> = vec(3, 7, 9);
  let found: Option<i64> = find_index(ref xs, 7);
  let result: i64 = match found {
    Option.Some(i) then xs[i],
    Option.None then 0 - 1,
  };
  print "result:", result;   // 7
  return 0;
}
```

### The map lookup (when you have HashMap)

Same idea as the index lookup, and for the same reason:
`hashmap_get(ref map, key)` already returns `Option<V>` (a
*copy* of the value, not `Option<ref V>`) -- there is no
ref-returning HashMap lookup in v1. If `V` is a struct you'd
rather not copy, store `HashMap<K, i64>` mapping to an index into
a parallel `Vec<V>`, and apply the index-lookup pattern above to
read the `V` through a ref.
Caller matches on the Option before dereferencing.

## A summary you can carry

- vāṇी uses **lifetime elision** -- references can be
  returned from functions, but lifetimes are inferred from
  signature shape, NEVER written as `'a` / `'b`.
- The rule: a function returning `ref T` must have
  **exactly one** ref/mut-ref parameter. The result borrows
  from that single source.
- Zero ref params + ref return -> reject ("nothing to
  borrow from").
- Two-or-more ref params + ref return -> reject
  ("ambiguous; v1 needs exactly one"). Workaround: refactor
  to one ref + values, or split into narrower fns (each
  returning a bare `ref`) and select at the call site with an
  `if`-expression -- zero-copy, verified working.
- **A ref return can only carry ONE reference** -- a tuple of
  refs or `Option<ref T>` is rejected the same way (a nested
  ref, not a bare `ref T` at the top level). Workaround: same
  as the two-ref-param case, split into narrower functions.
- **A `ref`/`mut ref` to a bare scalar (`ref i64`, `ref
  OwnedStr`, ...) is a dead end** -- no deref operator, so
  nothing reads or writes through it (arithmetic, comparison,
  `print`, and plain assignment are all rejected). Only
  struct/`Vec` granularity works, because `.field`/`[i]`
  access is how you actually read through a ref in v1.
- v1 also can't return a ref into an arbitrary Vec slot
  (`ref xs[i]` is rejected outright, any element type) --
  return the index instead and let the caller index.
- The compiler tracks the lifetime relationship through
  chained ref-returning calls -- `let r2 = shared(r1)`
  inherits r1's source.
- Multi-lifetime struct definitions are deferred ("path D" --
  explicit lifetime syntax); v1 doesn't ship them. Ref-capturing
  closures are narrower than that: they work as real values
  (pass as an argument, call directly) with a genuine non-escape
  check, just not yet for the "outlive the creating scope" shape
  -- see the "Closures capturing refs" section above.

The takeaway: **vāṇी has lifetimes -- they're just always
implicit.** The single-param elision rule + automatic
source-tracking covers the cases most user code needs;
advanced shapes that would require `'a` syntax are
explicitly rejected with workarounds.

## Cross-reference

- [Intermediate 3 -- Affine ownership](03_affine.md) -- the
  foundation; refs are second-class
- [Intermediate 3b -- Affine deeper](03b_affine_deeper_primer.md)
  -- borrow scopes; many-shared-XOR-one-mutable rule
- [Intermediate 3d -- Cyclic references](03d_cyclic_references_primer.md)
  -- refs in tree-shaped data using indices, not lifetime-
  parameterized pointers
- [Beginner 9 -- First contract](../beginner/09_smt_intro.md)
  -- `requires` clauses can complement lifetime rules (e.g.
  `requires i < len(xs)` for the index-lookup pattern)


---

**Previous**: [Sec.3d -- Cyclic references primer ->](03d_cyclic_references_primer.md)
**Next**: [Sec.3 -- Affine ownership: ref / mut ref ->](03_affine.md)

