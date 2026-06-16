# Intermediate 6 — Closures and iterator combinators

> **Learning goal**: bind anonymous functions to `fn`-pointer
> values, pass them to higher-order helpers, and build your
> own `fold` over a `Vec<i64>`.

> **New to this?** Read [Intermediate 6a — Closures primer](06a_closures_primer.md)
> and [Intermediate 6b — Iterators primer](06b_iterators_primer.md) first.

Imagine handing a recipe card to a chef: the card IS the
instruction (a function), not a named dish on the menu.
Higher-order functions work the same way — instead of calling
a specific named function, you pass one in as a parameter,
and the caller decides at runtime what "the thing to do" is.
This lets you write `map`, `filter`, and `fold` once and reuse
them for any operation the caller supplies.

## The program

```vani
intent "Intermediate 6 worked example — anonymous fns + higher-order helpers.";

fn apply(f: fn(i64) -> i64, x: i64) -> i64 {
  return f(x);
}

fn fold(xs: ref Vec<i64>, init: i64, op: fn(i64, i64) -> i64) -> i64 {
  let acc: i64 = init;
  let i: u64 = 0;
  while i < len(xs) {
    acc = op(acc, xs[i]);
    i = i + 1;
  }
  return acc;
}

fn main() -> i64 {
  // Lambda literal in value position.
  let double: fn(i64) -> i64 = fn(x: i64) -> i64 { return x + x; };
  print "double(5) =", apply(double, 5);

  let add: fn(i64, i64) -> i64 = fn(a: i64, b: i64) -> i64 { return a + b; };
  let xs: Vec<i64> = vec(1, 2, 3, 4, 5);
  print "sum =", fold(ref xs, 0, add);

  let max: fn(i64, i64) -> i64 =
    fn(a: i64, b: i64) -> i64 { if a > b { return a; } return b; };
  print "max =", fold(ref xs, xs[0], max);

  return 0;
}
```

## Compile + run

```bash
vanic run ~/int6.vani
```

Output:

```
double(5) = 10
sum = 15
max = 5
```

## Why it works that way

- **`fn(p: T, …) -> R { body }`** in *value position* is an
  anonymous function literal. The compiler's lambda-lift pass
  hoists each one into a generated top-level
  `__anon_fn_<N>(p: T, …) -> R` and replaces the expression
  with a function pointer.
- **Function-pointer types**: `fn(i64) -> i64`,
  `fn(i64, i64) -> i64`, etc. These are first-class values you
  can store in `let` bindings, pass as parameters, and store in
  struct fields.
- **No captured environment in v1**. The body sees only its own
  params, top-level functions, and builtins. A reference to a
  surrounding `let` binding produces an *"unknown variable"*
  error. Closures with captures are queued for a later phase;
  build today's combinators with explicit-parameter lambdas.
- **Statement-style body only**: `fn(x) -> i64 { return x + x; }`,
  not `fn(x) => x + x`. The expression-body sugar is deferred.

## Comparison to Rust

| Rust | vāṇी v1 |
|---|---|
| `|x| x + x` | `fn(x: i64) -> i64 { return x + x; }` |
| Closure with capture (`move \|x\|`) | Not yet — workaround: pass captures as explicit params |
| `iter().map(f).collect()` chains | Hand-rolled `fold` / loops (this lesson) |

The combinator-style chain isn't in the v1 stdlib yet — but the
underlying mechanism (anon fns + higher-order helpers) is, and
you can build your own `map_into` / `filter_into` / `fold` on
top of it.

## Challenge

Write a `map_into(src: ref Vec<i64>, dst: mut ref Vec<i64>, f:
fn(i64) -> i64)` helper that pushes `f(src[i])` into `dst` for
each element. Use it to build a `Vec` of squares of `[1..6]`,
then `fold` it to get the sum-of-squares.

---

**Next**: [§7 — Tuples and tuple destructure →](07_tuples.md)
