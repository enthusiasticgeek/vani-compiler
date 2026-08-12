# Intermediate 4c -- Generics and monomorphization (intuition primer)

> **Learning goal**: build a mental model of "generic" -- the
> `<T>` notation you've seen in `Vec<T>` or `fn id<T>(x: T)`.
> Why this exists, what the compiler does with it, and when
> to reach for it. Reading order: [04b interfaces primer](04b_interfaces_primer.md)
> -> here -> [Intermediate 4 generics+interfaces](04_generics_iface.md)
> for the formal syntax.

This chapter is mostly intuition, with real generic-function code
once the analogy lands.

## The problem: code that's the same except for the type

You wrote a function that returns the maximum of two i64s:

```vani
fn max_i64(a: i64, b: i64) -> i64 {
  if a > b { return a; }
  return b;
}
```

Now you need the same thing for f64. And for u32. And for any
type that has a `>` operator.

Without generics, you'd write `max_f64`, `max_u32`, `max_i32`,
... -- N copies of the same algorithm, differing only in the
type name. Tedious, error-prone (a bug fix has to be applied
N times), and bloats your source.

**Generics** let you write the function ONCE with a placeholder
for the type, and the compiler generates the type-specific
versions for you.

## The cookie cutter analogy

A generic function is a **cookie cutter**. The shape is fixed --
"take two values, return whichever is bigger" -- but the
material isn't specified. When you press the cutter into
chocolate-chip dough, you get a chocolate-chip cookie. When you
press it into oatmeal dough, you get an oatmeal cookie.

```vani
interface Cmp {
  fn cmp(self: Score, other: Score) -> i64;   // -1 / 0 / 1
}

struct Score { value: i64 }
implement Cmp for Score {
  fn cmp(self: Score, other: Score) -> i64 {
    if self.value < other.value { return -1; }
    if self.value > other.value { return 1; }
    return 0;
  }
}

fn max<T>(a: T, b: T) -> T where T is Cmp {
  if a.cmp(b) >= 0 { return a; }
  return b;
}
```

`<T>` is the cookie cutter's "material slot". `T` is a
placeholder -- when you call `max(score1, score2)`, the compiler
sees `T = Score`. Call it with a different `Cmp`-implementing
struct and you get a second, independently-compiled `max`.

The `where T is Cmp` is the bound -- only types implementing
`Cmp` (i.e. providing a `cmp` method) can use this cookie
cutter. Without the bound, the compiler couldn't be sure the
`a.cmp(b)` call would work for arbitrary T. **v1 doesn't ship a
built-in `Comparable`/`Ord`-style interface with native `<`/`>`
operator dispatch through a bound** -- and primitive types
(`i64`, `f64`, ...) can't `implement` a user interface at all in
v1 (`implement Cmp for i64` is rejected: "requires a struct or
enum type"). So a bounded generic always compares through an
explicit method like `cmp` on a struct/enum you define, never a
bare `a > b` on the type parameter directly. See the real worked
example in
[`examples/language/english/bounded_generics.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/english/bounded_generics.vani).

You can also write the bound inline, directly in the angle
brackets:

```vani
fn max<T: Cmp>(a: T, b: T) -> T {   // inline bound
  if a.cmp(b) >= 0 { return a; }
  return b;
}
```

`<T: Cmp>` and `<T> where T is Cmp` produce identical code --
pick whichever reads more naturally. Inline bounds are shorter
for single-constraint generics; `where` is clearer when several
constraints appear on different type params.

## What the compiler ACTUALLY does

This is the load-bearing concept: the compiler doesn't keep
one generic version. It generates a **specialized copy per
concrete type** you call it with.

In your program, given a second `Cmp`-implementing struct
alongside `Score`:
```vani
struct Money { cents: i64 }
implement Cmp for Money {
  fn cmp(self: Money, other: Money) -> i64 {
    if self.cents < other.cents { return -1; }
    if self.cents > other.cents { return 1; }
    return 0;
  }
}

let s1: Score = Score { value: 3 };
let s2: Score = Score { value: 7 };
let bigger_score: Score = max(s1, s2);

let m1: Money = Money { cents: 500 };
let m2: Money = Money { cents: 250 };
let bigger_money: Money = max(m1, m2);
```

(Note the `let`-bound intermediates -- v1's generic-call type
inference only reads the concrete type off a literal or a
named, `let`-annotated variable at the `T` position, not an
arbitrary expression like a struct literal. `max(Score { value:
3 }, ...)` inline is rejected; `let s1: Score = Score { value: 3
}; max(s1, ...)` works.)

The compiler emits TWO functions behind the one `max<T>` source:
- `max__Score(a: Score, b: Score) -> Score` (specialized to Score)
- `max__Money(a: Money, b: Money) -> Money` (specialized to Money)

Each is fully optimized for its specific type -- the `Score`
version's `a.cmp(b)` call resolves directly to `Score`'s
`implement Cmp` block at compile time; the `Money` version
resolves to `Money`'s. No runtime type-dispatch, no
indirection.

This process is called **monomorphization** -- "making
mono-typed versions". The Greek root: *mono* = one, *morph* =
form. Generic source -> multiple mono-typed compiled forms.

## Why "monomorphization, not type erasure"?

Java (and some other languages) take a different approach
called **type erasure**: at runtime, generic types are
"erased" -- your `List<Integer>` and `List<String>` are both
just `List` at runtime. They share ONE compiled implementation.

Pros of erasure:
- One compiled function per generic (smaller binary).
- Recompiling user code doesn't require recompiling the
  generic library.

Cons:
- Boxing: small types like `int` have to be wrapped in
  `Integer` objects to fit through the generic -- slow.
- No type-specific optimization: the one compiled version has
  to work for everything.
- Lost type info at runtime: you can't ask "is this a
  `List<String>` vs `List<Integer>`?".

vāṇी (and Rust, and C++ templates) pick monomorphization
because the speed win is large for systems code. The cost is
a bigger compiled binary, but that's usually acceptable.

## Coming from C++: `<T>` is a template parameter

If your mental model is C++, you've already used this feature --
`fn id<T>(x: T) -> T` is doing exactly what
`template <typename T> T id(T x)` does: both are compiled once
per concrete type actually used, and both are zero-cost (no
boxing, no vtable involved).

```cpp
// C++
template <typename T>
T id(T x) { return x; }

id(3);        // instantiates id<int>
id(3.0);      // instantiates id<double>
```

```vani
// vāṇी
fn id<T>(x: T) -> T { return x; }

id(3);        // instantiates id__i64
id(3.0);      // instantiates id__f64
```

The visible difference shows up once the body needs an operation
on `T` -- `max<T>` above uses `a.cmp(b)`, gated by `where T is
Cmp`. C++ templates are checked *lazily*, at each instantiation
site: historically an error inside the template body only
surfaced (with a wall of nested-instantiation text) once you
called it with a type that didn't support the operation; C++20
`concepts` fixed this by letting you write the constraint
(`requires std::totally_ordered<T>`) up front. vāṇी's `where T
is Iface` bound is the same idea, but mandatory and checked once
against the generic definition itself -- if the body uses an
operation the bound doesn't grant, it's a compile error on the
`fn` before anyone calls it with any type.
There's no equivalent to an unconstrained template (any type,
checked only at use) in vāṇी v1: every generic needs an explicit
bound for every operation its body performs.

The other difference: C++ templates support far more --
non-type template parameters, template template parameters,
partial/explicit specialization, SFINAE, variadic packs. vāṇी
generics are deliberately smaller: one type parameter per `fn`
in v1, a fixed set of bound kinds, no specialization. If you've
felt template-metaprogramming pain in C++, that's the pain vāṇी
is opting out of.

## Common generic shapes

### `Vec<T>` -- generic container

You've seen this. `Vec<i64>`, `Vec<OwnedStr>`, `Vec<Bag>` --
each is its own type at runtime, with its own specialized
compiled helpers (push, pop, len, etc.).

### `Option<T>` / `Result<T, E>` -- generic enum

```vani
enum Option<T> {
  Some(T),
  None,
}
```

Same shape (Some or None) regardless of T. The compiler
generates `Option__i64`, `Option__OwnedStr`, etc. per use.

### `id<T>` -- generic identity function

```vani
fn id<T>(x: T) -> T { return x; }
```

Almost trivial -- just returns its argument. But the type can
be anything. Useful as a building block in functional patterns.

### Generic with bounds

```vani
fn min<T>(a: T, b: T) -> T where T is Cmp {
  if a.cmp(b) <= 0 { return a; }
  return b;
}
```

The `where T is Cmp` bound tells the compiler what operations
you'll use inside the body (`a.cmp(b)`). Without it, the
compiler can't verify the body -- it'd reject the call to `cmp`
on an unknown type.

## Multi-parameter generics

```vani
enum Result<T, E> {
  Ok(T),
  Err(E),
}
```

Two placeholders. `Result<i64, OwnedStr>` is different from
`Result<f64, OwnedStr>` -- both monomorphized separately.

## When NOT to use generics

If your function only ever runs on i64, don't make it generic.
The generic is overhead (in syntax + reader cognitive load + a
slight increase in compile time) for no benefit.

Generics shine when:
- The same algorithm genuinely works across types (containers,
  utility functions like `max` / `min` / `swap`).
- A library wants flexibility for its users (Vec, HashMap, etc.).

Generics misfire when:
- You're parameterizing over types that have nothing in common
  (use multiple specific functions instead).
- The bound (`where T is ...`) effectively names a single type
  (just use that type).

## A summary you can carry

- A **generic** is a cookie cutter with a type-shaped slot.
- `<T>` declares the type placeholder.
- `where T is Iface` is a bound -- restricts T to types
  implementing the interface, and lets the body call the
  interface's methods on T.
- The compiler does **monomorphization**: generates one
  compiled copy per concrete type used. Fast, no boxing, no
  runtime type info.
- The trade-off vs Java-style erasure: bigger binary, faster
  runtime, no boxing. vāṇी picks this trade-off.

That's generics. The pairing with interfaces (which 04b
covered) is the whole story: interfaces define contracts,
generics make functions that work over any type satisfying a
contract.

The next chapter ([Intermediate 4](04_generics_iface.md))
shows the syntax + a worked example with both pieces together.

## Cross-reference

- [Intermediate 4 -- Generics and interfaces](04_generics_iface.md)
  -- actual syntax + worked example
- [Intermediate 4a -- `dyn Iface` primer](04a_dyn_iface_primer.md)
  -- the dynamic-dispatch alternative to monomorphization
- [Intermediate 4b -- Interfaces and static dispatch](04b_interfaces_primer.md)
  -- the contract side of the generic+interface pairing
- [Beginner 6a -- Pointers and references](../beginner/06a_pointers_refs_primer.md)
  -- generics over reference types (`<T>` and `<ref T>`) compose
  cleanly


---

**Previous**: [Sec.4b -- Interfaces and static dispatch primer ->](04b_interfaces_primer.md)
**Next**: [Sec.4d -- Default methods and blanket implementations primer ->](04d_default_methods_primer.md)

