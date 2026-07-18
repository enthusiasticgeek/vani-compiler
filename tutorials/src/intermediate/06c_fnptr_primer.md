# Intermediate 6c -- Function pointers: `fn(A) -> R`

> **Learning goal**: store functions as values, pass them as
> arguments, return them from functions, and collect them in
> `Vec<fn(A)->R>`.

> **Prerequisites**: [Intermediate 6 -- Closures and iterator combinators](06_closures.md).

---

## The concept

In vāṇी a function name can be used as a *value* -- not just called,
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

## Type-checking rules

| Situation | Result |
|-----------|--------|
| Named function used as value | Inferred as `fn(<params>) -> <ret>` |
| Anonymous `fn` literal | Same type as its declared signature |
| Wrong arity or param type | **Compile error** -- the signatures must match exactly |
| `fn(i64) -> i64` != `fn(i64, i64) -> i64` | **Compile error** |

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

---

## Challenge

Write a function `compose(f: fn(i64)->i64, g: fn(i64)->i64) -> fn(i64)->i64`
that returns a new function equal to `f(g(x))`. Use it to compose
`double` and `square` and verify `compose(double, square)(3) == 18`.

*(Hint: vāṇी v1 closures can capture function-pointer locals.)*

---

**Previous**: [Sec.6 -- Closures and iterator combinators ->](06_closures.md)
**Next**: [Sec.7 -- Tuples and tuple destructure ->](07_tuples.md)
