# Intermediate 6 -- Closures and iterator combinators

> **Learning goal**: bind anonymous functions to `fn`-pointer
> values, pass them to higher-order helpers, and build your
> own `fold` over a `Vec<i64>`.

> **New to this?** Read [Intermediate 6a -- Closures primer](06a_closures_primer.md)
> and [Intermediate 6b -- Iterators primer](06b_iterators_primer.md) first.

Imagine handing a recipe card to a chef: the card IS the
instruction (a function), not a named dish on the menu.
Higher-order functions work the same way -- instead of calling
a specific named function, you pass one in as a parameter,
and the caller decides at runtime what "the thing to do" is.
This lets you write `map`, `filter`, and `fold` once and reuse
them for any operation the caller supplies.

## The program

```vani
intent "Intermediate 6 worked example -- anonymous fns + higher-order helpers.";

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

- **`fn(p: T, ...) -> R { body }`** in *value position* is an
  anonymous function literal. The compiler's lambda-lift pass
  hoists each one into a generated top-level
  `__anon_fn_<N>(p: T, ...) -> R` and replaces the expression
  with a function pointer.
- **Function-pointer types**: `fn(i64) -> i64`,
  `fn(i64, i64) -> i64`, etc. These are first-class values you
  can store in `let` bindings, pass as parameters, and store in
  struct fields.
- **Captured environment**: closures can capture surrounding
  `let` bindings by value (move, for owning types such as
  `Vec`/`OwnedStr`) or by copy (for `Copy` types such as
  `i64`/`bool`). A closure that captures an owning value can
  be called at most once — calling it a second time is a
  compile error (affine / FnOnce semantics). See
  [06a -- closures primer](06a_closures_primer.md) for the
  full capture rules.
- **Capturing by reference**: write an explicit `[ref name]`
  list between the return type and the body to borrow a
  variable instead of moving/copying it:

  ```vani
  fn apply(f: Closure(i64) -> f64, x: i64) -> f64 {
    return f(x);
  }

  fn main() -> i64 {
    let data: Vec<f64> = vec(1.0, 2.0, 3.0);
    let lookup: fn(i64) -> f64 = fn(x: i64) -> f64 [ref data] {
      return data[x];
    };
    print "lookup(2) =", apply(lookup, 2);   // 3 -- data borrowed, not moved (whole-number f64 prints without a trailing .0)
    print "still usable:", data[0];          // 1 -- `data` wasn't consumed
    return 0;
  }
  ```

  Note the parameter type on the receiving side: a function that
  accepts a closure *value* (to store it, pass it along, or call
  it indirectly through a variable) must declare that parameter
  as `Closure(...) -> R`, not `fn(...) -> R` — the two don't
  implicitly convert into each other. A closure with no captures,
  or one you only ever call directly by its own `let` name in the
  same scope, can still use the plain `fn(...) -> R` type. See
  [06a's capture rules](06a_closures_primer.md#the-capture-rules)
  for what a `[ref name]`-capturing closure can and can't do yet
  as a value (short version: pass it as an argument or call it
  directly — yes; return it or store it past its capture's scope
  — not yet).
- **Statement-style body only**: `fn(x) -> i64 { return x + x; }`,
  not `fn(x) => x + x`. The expression-body sugar is deferred.

### Calling an owning-capture closure twice

<img class="manas" src="../images/mascot/manas_mascot_error.png" title="this code does not compile!"/>

```vani
// This is a compile error -- `consume` captures `data` (a Vec,
// an owning type) by value, so calling it consumes that capture.
// The second call is a use-after-move.
fn main() -> i64 {
  let data: Vec<i64> = vec(1, 2, 3);
  let consume = fn() -> i64 { return len(data) as i64; };
  let a: i64 = consume();
  let b: i64 = consume();   // ERROR: 'consume' was moved; cannot use after move
  print a, b;
  return 0;
}
```

## Comparison to Rust

| Rust | vāṇी v1 |
|---|---|
| `|x| x + x` | `fn(x: i64) -> i64 { return x + x; }` |
| Closure with capture (`move \|x\|`) | Supported -- by-value for owning types (FnOnce), by-copy for `Copy` types |
| `iter().map(f).collect()` chains | Hand-rolled `fold` / loops (this lesson) |

The combinator-style chain isn't in the v1 stdlib yet -- but the
underlying mechanism (anon fns + higher-order helpers) is, and
you can build your own `map_into` / `filter_into` / `fold` on
top of it.

## Built-in vec combinators

vāṇī ships the common functional combinators as builtins so you
don't have to hand-roll them. `vec_map`, `vec_filter`, `vec_fold`,
`vec_sum`, `vec_min`, and `vec_max` work on both `Vec<i64>` and `Vec<f64>`.

| Builtin | `Vec<i64>` signature | `Vec<f64>` signature | Returns |
|---|---|---|---|
| `vec_map(v, f)` | `fn(i64)->i64 -> Vec<i64>` | `fn(f64)->f64 -> Vec<f64>` | transformed copy |
| `vec_filter(v, pred)` | `fn(i64)->bool -> Vec<i64>` | `fn(f64)->bool -> Vec<f64>` | elements where pred is true |
| `vec_fold(v, init, f)` | `i64, fn(i64,i64)->i64 -> i64` | `f64, fn(f64,f64)->f64 -> f64` | reduce to single value |
| `vec_sum(v)` | `-> i64` | `-> f64` | sum of all elements |
| `vec_min(v)` | `-> i64` | `-> f64` | minimum value |
| `vec_max(v)` | `-> i64` | `-> f64` | maximum value |
| `vec_any(v, pred)` | `fn(i64)->bool -> bool` | — | true if any element matches |
| `vec_all(v, pred)` | `fn(i64)->bool -> bool` | — | true if all elements match |
| `vec_count(v, pred)` | `fn(i64)->bool -> i64` | — | count of matching elements |
| `vec_product(v)` | `-> i64` | — | product of all elements |

```vani
intent "Intermediate 6 -- built-in vec combinators.";

fn main() -> i64 {
  let nums: Vec<i64> = vec(1, 2, 3, 4, 5, 6, 7, 8, 9, 10);

  // vec_map: square every element
  let squares: Vec<i64> = vec_map(ref nums, fn(x: i64) -> i64 { return x * x; });
  print "sum of squares:", vec_sum(ref squares);           // 385

  // vec_filter: keep only even numbers
  let evens: Vec<i64> = vec_filter(ref nums, fn(x: i64) -> bool { return x % 2 == 0; });
  print "even count:", len(evens);                        // 5

  // vec_fold: custom accumulator
  let product: i64 = vec_fold(ref nums, 1, fn(acc: i64, x: i64) -> i64 { return acc * x; });
  print "product:", product;                              // 3628800

  // vec_any / vec_all
  print "any >9:", vec_any(ref nums, fn(x: i64) -> bool { return x > 9; });   // true
  print "all >0:", vec_all(ref nums, fn(x: i64) -> bool { return x > 0; });   // true

  return 0;
}
```

These builtins are the preferred form for common operations; the
hand-rolled `fold` above is the fallback when you need a custom
accumulator shape not covered by the table.

## Challenge

Write a `map_into(src: ref Vec<i64>, dst: mut ref Vec<i64>, f:
fn(i64) -> i64)` helper that pushes `f(src[i])` into `dst` for
each element. Use it to build a `Vec` of squares of `[1..6]`,
then `fold` it to get the sum-of-squares.

---

**Previous**: [Sec.6b -- Iterators and combinators primer ->](06b_iterators_primer.md)
**Next**: [Sec.6c -- Function pointers primer ->](06c_fnptr_primer.md)
