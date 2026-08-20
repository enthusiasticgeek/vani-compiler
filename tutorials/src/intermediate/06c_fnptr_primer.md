# Intermediate 6c -- Function pointers: `fn(A) -> R`

> **Learning goal**: store functions as values, pass them as
> arguments, return them from functions, and collect them in
> `Vec<fn(A)->R>`.

> **Prerequisites**: [Intermediate 6 -- Closures and iterator combinators](06_closures.md).

---

## The phone number on a sticky note

Imagine a friend asks you to arrange a plumber for their leaky sink
while they're out of town. There are two ways you could handle
this.

The first way: you personally call the plumber, explain the
problem, schedule the appointment, and report back once it's done.
You did the work yourself, start to finish, right then and there.

The second way: you don't call anyone at all. Instead, you write
the plumber's phone number on a sticky note and hand it to your
friend. "Here -- call this whenever you need to." Your friend now
has exactly what they need to reach the plumber themselves, at
whatever time suits them, without you being involved at all.
Critically, the sticky note itself is not a plumber -- it's just
seven digits. Nothing happens until someone actually dials it. And
whoever's holding the sticky note doesn't need to know anything
about who answers, how the plumber does the job, or what tools they
carry -- they just need the number.

That's the difference between doing the work yourself and handing
someone "the number to call." A phone number on a sticky note is
small, easy to pass along, easy to tuck into someone else's pocket,
easy to swap out for a different plumber's number later -- and it
*represents* a specific, callable thing without *being* that thing.

Bridge to CS terms: a function pointer (`fn(A) -> R`) is exactly
this sticky note. Instead of calling a function yourself and
handing back the result, or writing the function's body inline
wherever it's needed, you hand someone a reference to a specific
function -- "the number to call" -- which they can invoke themselves,
whenever they need to, without needing to know how that function is
implemented until the moment they actually call it.

## The concept

In vāṇī a function name can be used as a *value* -- not just called,
but stored in a variable or passed to another function. The type of
such a value is written `fn(A, B) -> R`, mirroring the parameter
and return types of the function itself.

```vani
pure fn double(x: i64) -> i64 { return x * 2; }

fn main() -> i64 {
  let f: fn(i64) -> i64 = double;   // store function as value
  let r: i64 = f(7);               // call through the variable
  print "double(7) via fn-ptr:", r; // 14
  return 0;
}
```

No `&` or `*` -- the syntax is the same as a named function type.

---

## Passing a function to another function

The canonical use-case: a *higher-order function* that receives
a callback.

```vani
intent "higher-order function";

pure fn double(x: i64) -> i64 { return x * 2; }
pure fn square(x: i64) -> i64 { return x * x; }

fn apply(f: fn(i64) -> i64, x: i64) -> i64 {
  return f(x);
}

fn main() -> i64 {
  print "apply(double, 7):", apply(double, 7);   // 14
  print "apply(square, 7):", apply(square, 7);   // 49
  return 0;
}
```

The `f: fn(i64) -> i64` parameter is just a typed variable -- `apply`
doesn't know at compile time which function it will call.

---

## Anonymous functions at the call site

You can write an inline `fn` literal directly at the call site
instead of naming a top-level function:

```vani
fn apply(f: fn(i64) -> i64, x: i64) -> i64 { return f(x); }

fn main() -> i64 {
  let r: i64 = apply(fn(x: i64) -> i64 { return x + 100; }, 5);
  print "anonymous fn result:", r;  // 105
  return 0;
}
```

This is the same lambda syntax used with `map` / `filter` / `fold`.

---

## Returning a function from a function

A function can return another function. The return type is a `fn`
type:

```vani
intent "function factory";

fn add(a: i64, b: i64) -> i64 { return a + b; }

fn picker(use_add: bool) -> fn(i64, i64) -> i64 {
  // (vāṇī v1: the false branch would need a second named fn;
  //  here we always return add for illustration)
  return add;
}

fn main() -> i64 {
  let op: fn(i64, i64) -> i64 = picker(true);
  let r: i64 = op(3, 5);
  print "op(3, 5):", r;   // 8
  return 0;
}
```

---

## `Vec<fn(A) -> R>` -- tables of functions

A `Vec` of function pointers is a *dispatch table*. Push named
functions in, iterate, call them all:

```vani
intent "dispatch table";

fn double(x: i64) -> i64 { return x * 2; }
fn square(x: i64) -> i64 { return x * x; }
fn negate(x: i64) -> i64 { return 0 - x; }

fn main() -> i64 {
  let ops: Vec<fn(i64) -> i64> = vec(double, square, negate);

  let i: i64 = 0;
  while i < len(ops) as i64 {
    let f: fn(i64) -> i64 = ops[i];
    print "op", i, "applied to 4:", f(4);
    i = i + 1;
  }
  // prints: 8, 16, -4
  return 0;
}
```

---

## A `Vec<fn(A) -> R>` as a pipeline, not just a dispatch table

The dispatch table above calls each function on the SAME starting
value, independently. A different, equally common shape: run a value
through every function IN ORDER, feeding each one's output into the
next -- like a photo editing app applying "brighten," then "increase
contrast," then "clamp to valid range," one filter after another.

```vani
intent "a photo-filter pipeline built from a Vec of function pointers";

fn brighten(x: i64) -> i64 { return x + 20; }
fn contrast(x: i64) -> i64 { return x * 2; }
fn clamp_255(x: i64) -> i64 { if x > 255 { return 255; } return x; }

fn run_pipeline(value: i64, steps: ref Vec<fn(i64) -> i64>) -> i64 {
  let result: i64 = value;
  let i: i64 = 0;
  while i < len(steps) as i64 {
    let step: fn(i64) -> i64 = steps[i];
    result = step(result);   // this step's output feeds the next step
    i = i + 1;
  }
  return result;
}

fn main() -> i64 {
  let filters: Vec<fn(i64) -> i64> = vec(brighten, contrast, clamp_255);
  let pixel: i64 = 100;
  print "raw pixel:", pixel;                                // 100
  print "after filter pipeline:", run_pipeline(pixel, ref filters);  // 240
  return 0;
}
```

Nothing here is a new language feature -- it's the exact same
`Vec<fn(A) -> R>` from the dispatch-table example, just walked
sequentially instead of independently. What makes it worth calling out
separately: the LIST OF FILTERS can be built at runtime (read from a
config file, chosen by a user toggling checkboxes, reordered), while
`run_pipeline` itself never changes -- it doesn't know or care which
filters are in the list, only that each one is a `fn(i64) -> i64`.
That decoupling -- "the function that runs the steps" doesn't need to
know "which steps" -- is the actual payoff of treating functions as
values.

---

## Type-checking rules

| Situation | Result |
|-----------|--------|
| Named function used as value | Inferred as `fn(<params>) -> <ret>` |
| Anonymous `fn` literal | Same type as its declared signature |
| Wrong arity or param type | **Compile error** -- the signatures must match exactly |
| `fn(i64) -> i64` != `fn(i64, i64) -> i64` | **Compile error** |

<img class="manas" src="../images/mascot/manas_mascot_error.png" title="this code does not compile!"/>

```vani
// This is a compile error -- wrong arity:
fn add(a: i64, b: i64) -> i64 { return a + b; }
fn apply(f: fn(i64) -> i64, x: i64) -> i64 { return f(x); }
fn bad() -> i64 {
  return apply(add, 1);   // ERROR: fn(i64,i64)->i64 != fn(i64)->i64
}
```

---

## Closures vs function pointers

| | Function pointer `fn(A)->R` | Closure (via `fn` literal capturing locals) |
|---|---|---|
| Captures outer variables? | No -- stateless | Yes -- captures by value |
| Can store in `Vec`? | Yes -- `Vec<fn(A)->R>` | Yes -- same syntax |
| Usable with `map`/`filter`/`fold`? | Yes | Yes |
| Passed to `parallel for` body? | No -- indirect calls rejected | No -- same restriction |

When you need to capture a local variable, write a closure
(see [Sec.6 -- Closures](06_closures.md)). When you want a pure
stateless transform (no captured state), a named function passed
as a value is cleaner.

### A gotcha: `parallel for` rejects indirect calls

<img class="manas" src="../images/mascot/manas_mascot_caution.png" title="this code needs extra care"/>

The table above says fn-pointers and closures are both rejected
as *indirect calls* inside a `parallel for` body -- the
race-freedom pass only sees through a **direct** call to a named
function. It's easy to reach for the fn-pointer-as-parameter
habit from earlier in this chapter and get bitten. This compiles:

```vani
pure fn double(x: i64) -> i64 { return x * 2; }

fn main() -> i64 {
  let total: i64 = 0;
  parallel for i from 0 to 10
  reduce total with +;
  {
    total = total + double(i);   // OK -- direct call to a named fn
  }
  print "total =", total;
  return 0;
}
```

Storing `double` in a `fn(i64) -> i64` variable first and calling
*that* inside the loop body -- `let f: fn(i64) -> i64 = double;`
then `total = total + f(i);` -- is rejected: `'parallel for' body
cannot use indirect calls (fn-ptr) -- the purity gate sees only
direct calls`. Same restriction applies to closures. Call the
named function directly in the loop body instead of routing
through a stored fn-pointer or closure value.

---

## Challenge

Write a function `compose(f: fn(i64)->i64, g: fn(i64)->i64) -> fn(i64)->i64`
that returns a new function equal to `f(g(x))`. Use it to compose
`double` and `square` and verify `compose(double, square)(3) == 18`.

*(Hint: vāṇī v1 closures can capture function-pointer locals.)*

---

**Previous**: [Sec.6 -- Closures and iterator combinators ->](06_closures.md)
**Next**: [Sec.7 -- Tuples and tuple destructure ->](07_tuples.md)
