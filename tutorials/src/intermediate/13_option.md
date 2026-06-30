# Intermediate 13 -- `Option<T>` and the option builtins

> **Learning goal**: represent "a value that might not exist" with
> `Option<T>`, use the `option_*` builtin combinators to chain
> optional computations without writing a `match` every time, and
> understand when to prefer the match form vs the combinator form.

Think of `Option<T>` like a gift-wrapped box. When the gift is
inside you have `Option.Some(value)` -- you unwrap the box and use
it. When the box is empty you have `Option.None` -- there is
nothing to unwrap, but the box itself is still valid and you have
to decide what to do (use a default, skip the step, propagate the
emptiness). The `option_*` builtins are just helpers that save you
from hand-writing a `match` envelope every time you transform or
inspect the box.

## Declaring `Option<T>`

`Option<T>` is a built-in generic enum with two variants:

```vani
// The compiler pre-declares this; you don't need to write it.
// enum Option<T> { Some(T), None }
```

For v1, `T` must be `i64` or `f64`. Construct a value directly:

```vani
let present: Option<i64> = Option.Some(42);
let absent:  Option<i64> = Option.None;
```

Extract with `match`:

```vani
let val: i64 = match present {
  Option.Some(v) then v,
  Option.None    then 0,    // default
};
```

## The `option_*` builtins

| Builtin | Signature | Returns |
|---|---|---|
| `option_unwrap_or(o, def)` | `Option<i64>, i64 -> i64` | inner value or `def` |
| `option_is_some(o)` | `Option<i64> -> bool` | `true` if `Some` |
| `option_is_none(o)` | `Option<i64> -> bool` | `true` if `None` |
| `option_map(o, f)` | `Option<i64>, fn(i64)->i64 -> Option<i64>` | `Some(f(v))` or `None` |
| `option_filter(o, pred)` | `Option<i64>, fn(i64)->bool -> Option<i64>` | `Some(v)` if pred passes, else `None` |
| `option_or(o, fallback)` | `Option<i64>, Option<i64> -> Option<i64>` | `o` if `Some`, else `fallback` |
| `option_and_then(o, f)` | `Option<i64>, fn(i64)->Option<i64> -> Option<i64>` | flat-maps; propagates `None` |

For `f64` payloads, use the `_f64` suffixed variants:
`option_unwrap_or_f64`, `option_is_some_f64`, `option_is_none_f64`.

## The program

```vani
intent "Intermediate 13 -- Option<i64> builtins.";

fn safe_div(a: i64, b: i64) -> Option<i64> {
  if b == 0 {
    return Option.None;
  }
  return Option.Some(a / b);
}

fn main() -> i64 {
  let a: Option<i64> = safe_div(10, 2);
  let b: Option<i64> = safe_div(10, 0);

  // option_unwrap_or: extract or default
  print "10/2 =", option_unwrap_or(a, -1);       // 5
  print "10/0 =", option_unwrap_or(b, -1);       // -1

  // option_is_some / option_is_none
  print "a is some:", option_is_some(a);          // true
  print "b is none:", option_is_none(b);          // true

  // option_map: transform the value inside, skip if None
  let doubled: Option<i64> = option_map(a, fn(v: i64) -> i64 { return v * 2; });
  print "doubled 10/2 =", option_unwrap_or(doubled, -1);  // 10

  // option_filter: keep the value only if condition holds
  let big: Option<i64> = option_filter(a, fn(v: i64) -> bool { return v > 3; });
  let small: Option<i64> = option_filter(a, fn(v: i64) -> bool { return v > 99; });
  print "filter >3 (5):", option_unwrap_or(big, -1);      // 5
  print "filter >99 (none):", option_unwrap_or(small, -1);// -1

  // option_or: first Some wins
  let c: Option<i64> = option_or(b, Option.Some(99));
  print "None or Some(99):", option_unwrap_or(c, -1);     // 99

  // option_and_then: chain two optional steps (flatmap)
  let result: Option<i64> = option_and_then(
    safe_div(100, 4),                           // Some(25)
    fn(v: i64) -> Option<i64> { return safe_div(v, 5); }   // Some(5)
  );
  print "100/4 then /5 =", option_unwrap_or(result, -1);  // 5

  let broken: Option<i64> = option_and_then(
    safe_div(100, 0),                           // None -- stops here
    fn(v: i64) -> Option<i64> { return safe_div(v, 5); }
  );
  print "100/0 then /5 =", option_unwrap_or(broken, -1);  // -1

  return 0;
}
```

## Compile + run

```bash
vanic run ~/int13.vani
```

Expected output:

```
10/2 = 5
10/0 = -1
a is some: true
b is none: true
doubled 10/2 = 10
filter >3 (5): 5
filter >99 (none): -1
None or Some(99): 99
100/4 then /5 = 5
100/0 then /5 = -1
```

## match vs combinators -- when to use which

- **`match`**: when you need to branch on both `Some` and `None`
  and the two branches do structurally different things.
- **`option_map` / `option_and_then`**: when you want to transform
  or chain *inside* the option without explicitly writing both
  arms every time. Chains read left-to-right:
  `option_and_then(option_and_then(x, step1), step2)`.
- **`option_unwrap_or`**: the natural end of a chain -- collapse to
  a concrete value.

## How `Option<T>` relates to `hashmap_get`

`hashmap_get` and `hashmap_insert` both return `Option<T>` (see
Sec.14). The `option_*` builtins let you inline those result-checks
without a dedicated `unwrap_or` helper function in every file:

```vani
// Instead of match every time:
let v: i64 = option_unwrap_or(hashmap_get(ref m, k), -1);
```

## Challenge

Write a `lookup_chain` that looks up a key in one map, uses the
result as a key to look up a second map, and returns `Option<i64>`
from the second map -- using only `option_and_then`.

---

**Next**: [Sec.11 -- The 22 GoF design patterns ->](11_design_patterns.md)
