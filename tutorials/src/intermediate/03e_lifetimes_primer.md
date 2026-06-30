# Intermediate 3e -- Lifetimes and reference returns (intuition primer)

> **Learning goal**: understand vāṇी's "implicit lifetimes"
> model -- how `fn foo(p: ref T) -> ref T` works without
> mentioning `'a` anywhere, when it accepts your code, and
> when it doesn't. Reading order: [Intermediate 3 -- Affine
> ownership](03_affine.md), [Intermediate 3b -- Affine
> deeper](03b_affine_deeper_primer.md), and [Intermediate 3d --
> Cyclic references](03d_cyclic_references_primer.md).

This chapter has **no compiler code**. Pure intuition.

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
split into two functions.

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
// vāṇी v1 closures don't capture refs that outlive the
// declaration scope.
```

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

Three shapes you'll see in real code:

### The accessor

```vani
fn name(person: ref Person) -> ref OwnedStr {
  return ref person.name;
}
```

Returns a borrow of a struct field. Caller can read the
string through the ref without owning it.

### The index lookup

```vani
fn nth(xs: ref Vec<i64>, i: u64) -> ref i64
  requires i < len(xs)
{
  return ref xs[i];
}
```

Returns a borrow into a Vec slot. Requires the index is
in-bounds (caller's contract).

### The map lookup (when you have HashMap)

```vani
fn lookup(map: ref HashMap, key: i64) -> Option<ref V>
```

Returns either a borrow into the map's value slot, or None.
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
  to one ref + values, or split into narrower fns.
- The compiler tracks the lifetime relationship through
  chained ref-returning calls -- `let r2 = shared(r1)`
  inherits r1's source.
- Multi-lifetime struct definitions and ref-capturing
  closures are deferred ("path D" -- explicit lifetime
  syntax). v1 doesn't ship them.

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
