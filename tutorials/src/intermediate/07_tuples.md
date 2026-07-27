# Intermediate 7 -- Tuples and tuple destructure

> **Learning goal**: return multiple values from a function via
> a tuple, read fields with `.0` / `.1` / `.N`, and unpack with
> `let (a, b, c) = expr;`.

> **New to this?** Read [Beginner 7a -- Tuples and destructuring primer](../beginner/07a_tuples_primer.md) first.

A tuple is an anonymous bundle -- like a shoebox where you throw
in a name, an age, and a score without bothering to name the
box itself. When a function needs to hand back two or three
pieces of information at once, a tuple is the quick alternative
to defining a whole struct just for that one return. You access
slots by position (`.0` is the first item, `.1` the second) and
you can unpack the whole bundle in one line:
`let (name, age) = get_user();`.

## The program

```vani
intent "Intermediate 7 worked example -- tuples and tuple destructure.";

fn divmod(a: i64, b: i64) -> (i64, i64) {
  return (a / b, a % b);
}

fn make_pair() -> (i64, i64, i64) {
  return (10, 20, 30);
}

fn main() -> i64 {
  let pair: (i64, i64) = divmod(17, 5);
  // Field-style access via .0 / .1
  print "quot =", pair.0;
  print "rem =", pair.1;

  // Destructure with `let (a, b, c) = ...;`
  let (a, b, c) = make_pair();
  print "a =", a, "b =", b, "c =", c;

  return 0;
}
```

## Compile + run

```bash
vanic run ~/int7.vani
```

Output:

```
quot = 3
rem = 2
a = 10 b = 20 c = 30
```

## Why it works that way

- **Tuple type**: `(T1, T2, T3, ...)`. Heterogeneous elements;
  fixed arity is part of the type. There's no `Tuple<...>`
  generic wrapper -- the parens-list is the type.
- **Tuple value**: `(e1, e2, e3, ...)`. The element types are
  the types of each expression.
- **Field access**: `.0`, `.1`, `.2`, ... -- zero-indexed,
  positional. Out-of-range access (`.3` on a 2-tuple) is a
  type error.
- **Destructure** with `let (a, b, c) = expr;` binds each
  positional field to a fresh name. The element count must
  match the tuple's arity.
- **Tuples are values**, not references. Returning a tuple
  copies the elements into the caller's stack slot.

### Out-of-range field access

<img class="manas" src="../images/mascot/manas_mascot_error.png" title="this code does not compile!"/>

```vani
// This is a compile error -- `pair` only has fields .0 and .1,
// so `.3` is out of range for its arity:
fn divmod(a: i64, b: i64) -> (i64, i64) {
  return (a / b, a % b);
}

fn main() -> i64 {
  let pair: (i64, i64) = divmod(17, 5);
  print pair.3;   // ERROR: tuple index 3 out of bounds for tuple of arity 2
  return 0;
}
```

## When to reach for a tuple vs a struct

| Use case | Pick |
|---|---|
| Two related values returned from one fn (e.g. divmod) | **Tuple** |
| 3+ named fields, used across many functions | **Struct** |
| Field names matter for readability | **Struct** |
| Anonymous record at one call site | **Tuple** |

If you find yourself documenting `.0` and `.1` in comments,
that's the signal to switch to a struct.

## Challenge

Write `fn stats(xs: ref Vec<i64>) -> (i64, i64, i64)` returning
`(min, max, sum)` of the input. Use destructure at the call site
to bind all three values in one line and print them.

<details>
<summary>Solution</summary>

```vani
fn stats(xs: ref Vec<i64>) -> (i64, i64, i64) {
  let lo: i64 = xs[0];
  let hi: i64 = xs[0];
  let total: i64 = 0;
  let i: u64 = 0;
  while i < len(xs) {
    if xs[i] < lo { lo = xs[i]; }
    if xs[i] > hi { hi = xs[i]; }
    total = total + xs[i];
    i = i + 1;
  }
  return (lo, hi, total);
}

fn main() -> i64 {
  let xs: Vec<i64> = vec(4, 1, 7, 3, 9, 2);
  let (lo, hi, sum) = stats(ref xs);
  print "min =", lo, "max =", hi, "sum =", sum;
  return 0;
}
```

</details>

---

**Previous**: [Sec.6c -- Function pointers primer ->](06c_fnptr_primer.md)
**Next**: [Sec.8 -- Multi-file projects + `vani.toml` ->](08_manifest.md)
