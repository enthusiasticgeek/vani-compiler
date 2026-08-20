# Intermediate 6a -- Closures and lambda lifting (intuition primer)

> **Learning goal**: build a mental model of "closure" -- a
> function that REMEMBERS values from where it was created.
> Why this is different from a regular function, and what the
> compiler does to make it work. Reading order: this is
> standalone foundation; read it before
> [Intermediate 6 closures + iterator combinators](06_closures.md).

This chapter is mostly intuition, with real closure code once the
analogy lands.

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

**A quick terminology note, since it trips people up coming from other
languages**: "closure" and "lambda" are not two different features --
they're two names people use for overlapping ideas. "Lambda" (or
"anonymous function") just means "a function written inline, without
giving it a top-level name" -- the `fn(x: i64) -> i64 { return x + x;
}` written directly where you need it, or the shorthand `|x| x + x`.
"Closure" specifically means such a function that ALSO captures
something from its surroundings, like `make_greeter`'s `inc` above.
Every anonymous `fn`/`|...|` literal in vāṇी is a lambda in that
general sense; whether a *particular* one also counts as a closure
depends on whether it actually captures anything -- see the capture
rules later in this chapter for exactly when that does and doesn't
happen.

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
  let inc = fn(x: i64) -> i64 { return start + x; };
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

When you write (confirmed by testing, both backends -- note the
two-step shape: bind the closure expression to a `let` first,
*then* `return` the name. Returning `fn(...) {...}` directly
inline, without the intermediate `let`, doesn't get lifted --
the compiler's capture-detection is keyed on exactly this
`let NAME = fn(...) {...}` pattern):

```vani
fn make_greeter(name: OwnedStr) -> Closure(i64) -> i64 {
  let greet = fn(x: i64) -> i64 { print "hello,", name, x; return 0; };
  return greet;
}
```

The compiler internally rewrites it to something like (illustrative
pseudocode for intuition -- `Closure { call: ..., env_ptr: ... }`
struct-literal syntax isn't real, constructible user syntax; `Closure`
is a builtin two-pointer type, not a user struct you can build a
literal of):

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
fn make_greeter(name: OwnedStr) -> Closure(i64) -> i64 {
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
let doubled: Vec<i64> = vec_map(ref xs, |x| x * 2);
```

`vec_map` is a higher-order function -- it takes a function and
applies it to each element. The `|x| x * 2` shorthand is a tiny
anonymous function with no captures.

**A real v1 restriction worth knowing here**: `vec_map` (and the
other `vec_*` iterator builtins: `vec_filter`, `vec_fold`, ...)
specifically require a plain `fn(T) -> R` function pointer for
their callback argument -- confirmed by testing: passing a
`Closure(T) -> R` (a capturing closure) is rejected with "mapper
must be `fn(i64) -> i64`, got Closure(i64) -> i64". The `|x| ...`
shorthand always produces a plain `fn`, never a `Closure` (it can't
capture anything, which is exactly why it fits here), so this isn't
a limitation you'll usually notice -- but it does mean you can't
close over a local value directly inside a `vec_map` call the way
the closure sections above and below this one do:

```vani
// This does NOT work -- vec_map's callback can't capture:
// let factor: i64 = 10;
// let scaled: Vec<i64> = vec_map(ref xs, |x| x * factor);  // rejected

// Call the capturing closure yourself instead -- a plain loop:
let factor: i64 = 10;
let scale = fn(x: i64) -> i64 { return x * factor; };
let scaled: Vec<i64> = vec();
let i: i64 = 0;
while i < (len(xs) as i64) {
  push(mut ref scaled, scale(xs[i]));
  i = i + 1;
}
```

The closure `scale` still carries `factor` as part of its
environment -- it's just called directly rather than handed to
`vec_map`.

### 2. As returned values (factory patterns)

```vani
fn make_validator(min: i64, max: i64) -> Closure(i64) -> bool {
  let in_range = fn(x: i64) -> bool { return min <= x && x <= max; };
  return in_range;
}
```

`make_validator(0, 100)` returns a closure that checks if a
number is in the 0-100 range. The min/max are baked in.

### 3. Stored in data structures

```vani
struct Handler { cb: Closure(i64) -> i64 }

fn main() -> i64 {
  let base: i64 = 100;
  let cb = fn(extra: i64) -> i64 { return base + extra; };
  let h: Handler = Handler { cb: cb };
  let f: Closure(i64) -> i64 = h.cb;
  print f(5);   // 105
  return 0;
}
```

A closure whose captures are all Copy (like `base: i64` above)
moves into a struct field and reads back out cleanly, verified
end-to-end on both backends.

<img class="manas" src="../images/mascot/manas_mascot_error.png" title="this code does not compile!"/>

**Current limit**: a closure that captures a heap-owning value
(a `Vec`, `OwnedStr`, or similar) BY MOVE cannot yet be stored
in a struct field:

```vani
struct Handler { cb: Closure(i64) -> i64 }

fn main() -> i64 {
  let data: Vec<i64> = vec(1, 2, 3, 4);
  let cb = fn(extra: i64) -> i64 { return data[0] + extra; };
  let h: Handler = Handler { cb: cb };   // rejected
  return 0;
}
```

```
error: closure 'cb' captures a heap-owning value by move --
       storing it in struct field 'cb' is not yet supported in v1
```

v1 doesn't yet track a heap-owning closure environment's
lifetime once it crosses a struct-field boundary, so this is a
clean compile-time rejection rather than an attempt to make the
pattern work. Workarounds: capture by `[ref name]` instead of by
move if the captured value outlives the struct (subject to the
`[ref name]` escape limits above), or restructure so the closure
only captures Copy values and pass the heap-owning value as a
call argument instead of capturing it.

### 4. A closure that "sees" later changes

Every example so far captured a value that never changed after the
closure was created. Capturing **by reference** (`[ref name]`) is
different: because the closure holds a reference to the ORIGINAL
storage, not a snapshot of it, it sees whatever that storage holds
each time it's called -- including changes made *after* the closure
was created.

```vani
intent "A closure that sees later pushes to the Vec it captured.";

fn main() -> i64 {
  let cart: Vec<i64> = vec(500, 250);   // prices in cents
  let cart_total = fn() -> i64 [ref cart] {
    return vec_sum(cart);
  };

  print "total after 2 items:", cart_total();   // 750

  push(mut ref cart, 100);   // add a third item -- AFTER the closure exists

  print "total after 3 items:", cart_total();   // 850 -- same closure, new answer
  return 0;
}
```

`cart_total` doesn't get called again with new information passed
in -- it takes no arguments at all. It sees the third item because it
was never given a *copy* of the cart; it was given a reference to the
one true cart, and that cart changed. This is the closure equivalent
of "Aisle 3, Shelf 5, Position 12, whatever's there right now" versus
a photograph of what was there when you looked.

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
// Overkill -- name it. (`|a, b| a + b` has no captures, so its
// real type is the plain function-pointer `fn(i64, i64) -> i64`,
// not `Closure(i64, i64) -> i64` -- there's no implicit
// fn-to-Closure coercion, confirmed by testing.)
let add: fn(i64, i64) -> i64 = |a, b| a + b;
let r: i64 = add(3, 5);

// Better.
fn add2(a: i64, b: i64) -> i64 { return a + b; }
let r2: i64 = add2(3, 5);
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
- vāṇī captures by value (for owning types), by copy (for Copy
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

