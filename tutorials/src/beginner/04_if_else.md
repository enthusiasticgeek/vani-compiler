# Beginner 4 — `if` / `else`

> **Learning goal**: branch on a `bool` condition, chain
> `else if` arms, combine conditions with `&&` and `||`.

## The program

Save this in `~/lesson4.vani`:

```rust
intent "Lesson 4 worked example — if / else / nested branches.";

fn sign(n: i64) -> i64 {
  if n > 0 {
    return 1;
  } else if n < 0 {
    return 0 - 1;
  } else {
    return 0;
  }
}

fn grade(score: i64) -> Str {
  if score >= 90 {
    return "A";
  } else if score >= 80 {
    return "B";
  } else if score >= 70 {
    return "C";
  } else {
    return "F";
  }
}

fn main() -> i64 {
  print "sign(7) =", sign(7);
  print "sign(0) =", sign(0);
  print "sign(-3) =", sign(0 - 3);

  print "grade(95) =", grade(95);
  print "grade(82) =", grade(82);
  print "grade(60) =", grade(60);

  let x: i64 = 5;
  if x > 0 && x < 10 {
    print "x is a single-digit positive";
  } else {
    print "x is out of single-digit range";
  }
  return 0;
}
```

## Compile + run

```bash
vanic run ~/lesson4.vani
```

Expected output:

```
sign(7) = 1
sign(0) = 0
sign(-3) = -1
grade(95) = A
grade(82) = B
grade(60) = F
x is a single-digit positive
```

## Why it works that way

- **The condition is a `bool`**, not "anything truthy". `if 5 {…}`
  is a type error. vāṇी has no "0 is false, anything else is
  true" coercion — write the comparison you mean (`if x != 0`).
- **`else if` chains as deeply as you need**. Each arm must
  produce the same type if the `if` is used as an expression
  (you'll see expression-form `if` in Intermediate §1); as
  statements they're independent.
- **No standalone unary minus on integer literals**. `0 - 1`
  works; `-1` directly doesn't parse as a literal in v1. You
  write the subtraction explicitly. (For float literals, `-1.0`
  is fine.)
- **`&&` short-circuits**, so does `||`. If the left side of `&&`
  is `false`, the right side never executes — useful when the
  right side would otherwise divide by zero or call into a
  function that depends on the left being true.

## A v1 caveat

The single-arm form is two-arm by default:

```rust
if x > 0 {
  print "positive";
}
// no else — this works, but…
```

When you use `if` as an *expression* (binding its result to a
`let`), you **must** have an `else` arm — otherwise the expression
has no value when the condition is false. The statement form
above is allowed without `else`; the expression form below is
not:

```rust
// statement form — no else required:
if x > 0 { print "positive"; }

// expression form — else required:
let label: Str = if x > 0 { "positive" } else { "non-positive" };
```

## Challenge

Write a `min3(a: i64, b: i64, c: i64) -> i64` function using only
`if` / `else` (no `match` yet) that returns the smallest of the
three arguments. Print `min3(7, 3, 5)` from `main`.

<details>
<summary>Solution</summary>

```rust
fn min3(a: i64, b: i64, c: i64) -> i64 {
  if a <= b && a <= c {
    return a;
  } else if b <= c {
    return b;
  } else {
    return c;
  }
}
```

</details>

---

**Next**: [§5 — `while` and `for` loops →](05_loops.md)
