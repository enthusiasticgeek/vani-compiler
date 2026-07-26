# Intermediate 6a -- Closures and lambda lifting (intuition primer)

> **Learning goal**: build a mental model of "closure" -- a
> function that REMEMBERS values from where it was created.
> Why this is different from a regular function, and what the
> compiler does to make it work. Reading order: this is
> standalone foundation; read it before
> [Intermediate 6 closures + iterator combinators](06_closures.md).

This chapter has **no compiler code**. Pure intuition.

## A function that "remembers"

Look at this hypothetical scenario.

You write a "make-greeter" tool. Given someone's name, it
returns a tiny custom function that greets that specific
person.

```vani
let say_hi_to_alice = make_greeter("alice");
let say_hi_to_bob   = make_greeter("bob");

say_hi_to_alice();   // prints "hello, alice"
say_hi_to_bob();     // prints "hello, bob"
```

These two functions came from the same factory (`make_greeter`)
but **remember different names**. `say_hi_to_alice` knows about
the string "alice"; `say_hi_to_bob` knows about "bob". Neither
takes a name as a parameter -- the name is *baked into the
function*.

That's a **closure**: a function that has *closed over* some
values from the surrounding context.

## The post-it-note analogy

Imagine functions as little robots. A regular function-robot
just runs the same program every time you press its button:

```vani
fn say_hello() -> i64 { print "hello"; return 0; }
```

A **closure**-robot has POST-IT NOTES stuck to it that say
things like `name = "alice"`, `count = 5`, etc. When you press
its button, it runs its program AND consults the notes to fill
in the blanks.

```vani
let say_hi_to_alice = make_greeter("alice");
```

The robot returned by `make_greeter("alice")` has a post-it
that says `name = "alice"` stuck on it. When called, it reads
the note to know who to greet.

These post-its are called **captured variables** -- values
captured from the surrounding scope at the moment the closure
was created.

## What makes this different from a function?

A regular function only knows about its parameters and global
items. It can't reach into the surrounding scope.

A closure can.

```vani
fn make_counter() -> ??? {
  let start: i64 = 0;
  let inc = |x: i64| start + x;
                  // ^ this closure CAPTURES `start`
                  // even though `start` is local to make_counter
  return inc;
}
```

When the closure `inc` runs, it can reference `start` even
though `start`'s declaration is back in `make_counter`. The
post-it stuck to `inc` keeps `start`'s value alive.

## How the compiler makes closures work

Two challenges:

### Challenge 1: where do the captured values LIVE?

When `make_greeter` returns, the function's stack frame is
gone. The local variables that lived on that frame are gone.
But the closure SHOULD still know about them. Where do they
live now?

**Solution**: the captured values are stored in a small data
structure (often called the closure's *environment*) that
lives on the heap. The closure itself is a pair: (function
code, pointer to environment).

```
The closure (say_hi_to_alice):
+--------------------------------+
| pointer to the greeter code    | <- stack/static
+--------------------------------+
| pointer to the environment     | <- heap
+--------------+-----------------+
               v
The environment (on the heap):
+--------------------------------+
| name = "alice"                 |
+--------------------------------+
```

Sound familiar? It's the same two-pointer shape as
`dyn Iface` (chapter 04a). Different bookkeeping, same idea:
small handle on stack/register, real data on heap.

### Challenge 2: how does the call site know what code to run?

Different closures (created from different factories) point at
different code. When you `say_hi_to_alice()`, the runtime has
to follow the closure's *first* pointer to find the right code.

This is exactly the same dispatch shape as a function pointer,
plus the environment pointer that gets passed in implicitly.

## "Lambda lifting" -- what the compiler ACTUALLY does

The phrase "lambda lifting" describes the compile-time
transformation that handles closures. The compiler does this
under the hood; you don't write any of it. But understanding
it helps the mental model.

When you write:

```vani
fn make_greeter(name: OwnedStr) -> Closure {
  return |x: i64| print "hello,", name, x;
}
```

The compiler internally rewrites it to something like:

```vani
// Step 1: lift the closure body into a top-level function
fn __anon_fn_0(env: ref Env_0, x: i64) -> i64 {
  print "hello,", env.name, x;
  return 0;
}

// Step 2: define a struct holding the captured values
struct Env_0 { name: OwnedStr }

// Step 3: make_greeter builds the env + bundles it with the
// fn pointer
fn make_greeter(name: OwnedStr) -> Closure {
  let env: Env_0 = Env_0 { name: name };  // heap-allocated
  return Closure { call: __anon_fn_0, env_ptr: ref env };
}
```

The closure expression becomes a top-level function (the
"lifted" lambda) plus a small env-struct. The runtime closure
is a two-pointer bundle. None of this is visible in your
source -- the compiler handles it.

## Where closures show up in practice

### 1. As function arguments (the iterator pattern)

```vani
let xs: Vec<i64> = vec(1, 2, 3, 4, 5);
let doubled: Vec<i64> = vec_map(xs, |x| x * 2);
```

`vec_map` is a higher-order function -- it takes a function (or
closure) and applies it to each element. The closure `|x| x * 2`
is a tiny anonymous function with no captures.

When you add captures:

```vani
let factor: i64 = 10;
let scaled: Vec<i64> = vec_map(xs, |x| x * factor);
                                       // ^ captures `factor`
```

The closure now carries `factor` as part of its environment.

### 2. As returned values (factory patterns)

```vani
fn make_validator(min: i64, max: i64) -> Closure {
  return |x| min <= x && x <= max;
}
```

`make_validator(0, 100)` returns a closure that checks if a
number is in the 0-100 range. The min/max are baked in.

### 3. Stored in data structures

```vani
struct EventHandler {
  on_click: Closure,
  on_hover: Closure,
}
```

Each handler is a closure with whatever captures it needs to
do its job.

## The capture rules

When a closure references a variable from the surrounding
scope, the compiler decides HOW to capture it:

- **By value (move)** -- for owning types (Vec, OwnedStr, Box).
  The closure takes ownership; the surrounding scope can no
  longer use the variable.
- **By copy** -- for Copy types (i64, bool, etc.). The closure
  gets a copy; the surrounding scope's binding is unchanged.
- **By reference** (`[ref name]`, 2026-07-25) -- an explicit
  capture-list, written between the closure's return type and
  its body, names exactly which free variables to borrow instead
  of move/copy: `fn(x: i64) -> f64 [ref data] { ... data[x] ... }`.
  Useful for a non-Copy value (like a `Vec`) the closure only
  needs to *read*, where moving would take ownership away from
  the surrounding scope unnecessarily. The captured reference's
  lifetime is checked the same way any other `ref` is: the
  closure (and anything built from it) can't outlive what it
  borrowed -- see the next paragraph for exactly what that does
  and doesn't allow yet.

**Current limits on `[ref name]`, worth knowing going in**: the
closure can be called directly by name in the same scope, or
passed as an argument to another function, right away -- both
work today. What's *not* yet allowed is treating it as a value
that outlives the scope where it captured the reference: return
it from the enclosing function, store it in a struct field or a
`Vec`, or otherwise let it escape past its captured data's
lifetime -- the compiler rejects all of those with a dangling-
reference diagnostic, the same class of check that guards a
plain `ref` local. If you need a closure to outlive its creating
scope while still borrowing, that's not supported (see
`vani-compiler/docs/ref_capturing_closures_design.md` for the
full design writeup, including why -- short version: it needs
real lifetime-variable tracking, which v1 deliberately doesn't
have; a non-escaping `[ref name]` closure gets you most of the
practical value without needing that).

## When NOT to use closures

A common over-use: turning a 3-line function body into a
closure when a regular function would do.

```vani
// Overkill -- name it.
let add: Closure = |a, b| a + b;
let r = add(3, 5);

// Better.
fn add(a: i64, b: i64) -> i64 { return a + b; }
let r = add(3, 5);
```

Closures are for cases where:
- You need to capture local context (the closure remembers
  something).
- You're passing the function as an argument to a higher-order
  function (`vec_map`, `vec_fold`).
- You're returning a function from a function (factory patterns).

If none of those apply, write a plain `fn`.

## A summary you can carry

- A **closure** is a function PLUS some captured values from
  where it was created. Two-pointer bundle: (code pointer,
  environment pointer).
- The captured values live on the heap in a synthesized env
  struct.
- **Lambda lifting** is the compile-time transformation that
  turns the closure expression into a top-level function +
  env struct.
- vāṇी captures by value (for owning types), by copy (for Copy
  types), or by reference via an explicit `[ref name]` list. A
  `[ref name]`-capturing closure can be called directly or
  passed as an argument today, but can't yet escape its creating
  scope as a stored/returned value.
- Use closures for: capturing local context, passing to
  higher-order functions, returning from factories. For
  everything else, plain `fn` is better.

The next chapter ([Intermediate 6](06_closures.md)) shows the
syntax + `vec_map` / `vec_fold` worked examples.

## Cross-reference

- [Intermediate 6 -- Closures + iterator combinators](06_closures.md)
  -- syntax + iterator pattern
- [Beginner 6c -- Ownership and move primer](../beginner/06c_ownership_primer.md)
  -- why "capture by value" is the default for owning types
- [Intermediate 4a -- dyn Iface primer](04a_dyn_iface_primer.md)
  -- closures and dyn share the same two-pointer shape; the
  bookkeeping differs but the dispatch idea is the same
- [Intermediate 4c -- Generics + monomorphization](04c_generics_primer.md)
  -- closures interact with generics: `vec_map<T, R>` takes a
  closure `T -> R` and monomorphizes per (T, R) pair


---

**Previous**: [Sec.5 -- Dynamic dispatch: dyn Iface ->](05_dyn.md)
**Next**: [Sec.6b -- Iterators and combinators primer ->](06b_iterators_primer.md)

