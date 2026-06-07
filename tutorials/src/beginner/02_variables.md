# Beginner 2 — Variables, types, operators

> **Learning goal**: declare typed variables, do arithmetic with
> integers and floats, and combine booleans.

## The program

Save this in `~/lesson2.vani`:

```rust
intent "Lesson 2 worked example — variables + types + operators.";

fn main() -> i64 {
  let a: i64 = 7;
  let b: i64 = 3;

  let sum: i64 = a + b;
  let diff: i64 = a - b;
  let prod: i64 = a * b;
  let quot: i64 = a / b;
  let rem: i64 = a % b;

  print "sum =", sum;
  print "diff =", diff;
  print "prod =", prod;
  print "quot =", quot;
  print "rem =", rem;

  let pi_approx: f64 = 3.14;
  let area: f64 = pi_approx * 4.0 * 4.0;
  print "area =", area;

  let is_positive: bool = a > 0;
  let either: bool = is_positive || (b < 0);
  print "is_positive =", is_positive;
  print "either =", either;

  return 0;
}
```

## Compile + run

```bash
vanic run ~/lesson2.vani
```

Expected output:

```
sum = 10
diff = 4
prod = 21
quot = 2
rem = 1
area = 50.24
is_positive = true
either = true
```

## Why it works that way

- **Types are explicit by default**. `let a: i64 = 7` names the
  type. You'll see inferred `let answer = …` later, but until you
  trust your guess about what the inferred type is, spelling it
  out is the safe choice.
- **Integer widths matter**. vāṇी has `i8`, `i16`, `i32`, `i64`
  (signed) and `u8`, `u16`, `u32`, `u64` (unsigned). Mixing widths
  without an explicit cast is a type error. Pick `i64` for
  general arithmetic; pick narrower widths when memory layout or
  embedded targets demand it.
- **Floats are `f32` or `f64`**. Same width-strictness rule
  applies: don't multiply an `f64` by an `i64` without casting.
- **`bool` is its own type**, not a 0-or-1 integer. `&&` / `||` /
  `!` work as you'd expect.
- **`/` on integers truncates toward zero**; `%` is the
  matching remainder. `7 / 3 == 2` and `7 % 3 == 1`. For float
  division, both operands must be floats: `7.0 / 3.0 == 2.333…`.
- **`print` accepts multiple comma-separated arguments**. The
  runtime emits them with single spaces in between. There's no
  format-string syntax — just compose with `,`.

## Challenge

Add an `i32` variable to the program, multiply it by `2`, and
print it. Note the **type error** when you mix it with `a` (an
`i64`) without a cast. Then fix the error using a cast.

<details>
<summary>Solution</summary>

```rust
let narrow: i32 = 5;
// print narrow * a;            // error: type mismatch (i32 vs i64)
let widened: i64 = narrow as i64;
print "widened * a =", widened * a;
```

vāṇी uses `as` for explicit numeric casts. There's no implicit
widening — the compiler tells you exactly where the conversion
must happen, which is harder to write but easier to read.

</details>

---

**Next**: [§3 — Functions and the four return aliases →](03_functions.md)
