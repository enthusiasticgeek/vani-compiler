# Beginner 2 -- Variables, types, operators

> **Learning goal**: declare typed variables, do arithmetic with
> integers and floats, and combine booleans.

A variable is a named box. You give the box a label (`a`), say
what kind of thing it holds (`i64` -- a whole number), and put
something in it (`7`). Later, whenever you write `a`, the
program looks up that box and uses what's inside. The *type*
(`i64`, `f64`, `bool`) is the shape of the box -- it determines
what values fit and what you're allowed to do with them (you
can't add a whole number to a `true`/`false`, just like you
can't fit a litre of milk in a coin slot).

## The program

Save this in `~/lesson2.vani`:

```vani
intent "Lesson 2 worked example -- variables + types + operators.";

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
  type. You'll see inferred `let answer = ...` later, but until you
  trust your guess about what the inferred type is, spelling it
  out is the safe choice.
- **Integer widths matter, but *safe* mixing doesn't need a cast**.
  vāṇी has `i8`, `i16`, `i32`, `i64` (signed) and `u8`, `u16`,
  `u32`, `u64` (unsigned). Two operands of the *same signedness*
  (`i32 * i64`, `u8 + u32`, ...) silently widen to the bigger of
  the two -- that direction can't lose information, so there's
  nothing to make explicit. What's rejected without an explicit
  `as` cast is anything that *could* lose information: mixed
  signedness at the same width (`i32 * u32`), or a signed type
  no wider than the unsigned one it's paired with (`i8 + u32`).
  Pick `i64` for general arithmetic; pick narrower widths when
  memory layout or embedded targets demand it.
- **Floats mix with integers the same permissive way**: `f64 * i64`
  silently converts the integer operand to float and evaluates as
  `f64` -- no cast required. This differs from many languages'
  "explicit everywhere" stance; vāṇी's explicitness requirement
  is really about *narrowing or sign-changing* conversions, not
  numeric mixing in general.
- **`bool` is its own type**, not a 0-or-1 integer. `&&` / `||` /
  `!` work as you'd expect.
- **`/` on integers truncates toward zero**; `%` is the
  matching remainder. `7 / 3 == 2` and `7 % 3 == 1`. For float
  division, both operands must be floats: `7.0 / 3.0 == 2.333...`.
- **`print` accepts multiple comma-separated arguments**. The
  runtime emits them with single spaces in between. There's no
  format-string syntax -- just compose with `,`.
- **`print` only accepts scalars** -- `i64`/`f64`/`bool`/`Str`/
  `OwnedStr`. Composite values (arrays, `Vec`, structs, tuples,
  enums) are rejected at compile time with a diagnostic telling
  you which field/element/match-arm to extract instead -- there's
  no auto-`Debug`-style dump of a whole struct or `Vec`.
- **`f64` formatting is `%g`-style by default** (compact, 6
  significant digits, switches to scientific notation for very
  large/small magnitudes) -- it's not lossless and, on Windows,
  it isn't even identical between the two backends for scientific
  notation. See [Beginner 6 -- Strings](06_strings.md#string-builtins-reference)
  for the full caveats and the `f64_to_str_fixed` workaround.

### A closer look: integer division truncates

<img class="manas" src="../images/mascot/manas_mascot_caution.png" title="this code needs extra care"/>

```vani
let q: i64 = 7 / 3;
print "7 / 3 =", q;
```

This compiles cleanly and runs without error -- but the output is
`7 / 3 = 2`, not `2.333...`. Both operands are `i64`, so `/` performs
integer division and truncates toward zero. It's an easy thing to
overlook when you're expecting fractional-looking results; if you want
`2.333...`, both operands need to be `f64` (`7.0 / 3.0`).

## Challenge

Add a `u32` variable to the program and multiply it by `a` (an
`i64`). Note this one *does* type-check and run -- `u32` is
narrower than `i64`, so it's a safe implicit widen. Now add an
`i32` variable at the *same width* as a `u32` and multiply the
two together. This time note the **type error**: same-width
mixed signedness is exactly the case vāṇी won't silently resolve,
because there's no widening direction that's obviously safe.
Then fix the error using a cast.

<details>
<summary>Solution</summary>

```vani
let small_u: u32 = 5;
print "small_u * a =", small_u * a;   // ok: u32 safely widens to i64

let small_i: i32 = 5;
// print small_u * small_i;      // error: no safe implicit integer
//                                // promotion for u32 and i32 (same
//                                // width, different signedness)
let widened: i64 = small_i as i64;
print "widened * a =", widened * a;
```

vāṇी uses `as` for the conversions it won't do implicitly: any
cast that could change a value's meaning (sign-changing, or
narrowing to a type that might not hold the value) has to be
spelled out at the call site.

</details>

## Bitwise operators

Integer types support these bitwise operators. There's no unary
`~` (bitwise-NOT) operator -- get the same result with `n ^ -1`
(XOR against all-ones flips every bit, which *is* what a
two's-complement bitwise-NOT does):

| Operator | Meaning | Example |
|---|---|---|
| `&` | bitwise AND | `12 & 10` -> `8` |
| `\|` | bitwise OR | `12 \| 10` -> `14` |
| `^` | bitwise XOR | `12 ^ 10` -> `6` |
| `^ -1` | bitwise NOT (complement) | `0 ^ -1` -> `-1` |
| `<<` | left shift | `1 << 3` -> `8` |
| `>>` | right shift | `16 >> 2` -> `4` |

```vani
intent "Lesson 2 -- bitwise ops.";

fn main() -> i64 {
  let a: i64 = 0b1100;    // 12
  let b: i64 = 0b1010;    // 10
  print "AND:", a & b;    // 8  (0b1000)
  print "OR:", a | b;     // 14 (0b1110)
  print "XOR:", a ^ b;    // 6  (0b0110)
  print "NOT 0:", 0 ^ -1; // -1 (all bits set in two's-complement)
  print "left shift:", 1 << 4;   // 16
  print "right shift:", 64 >> 3; // 8
  return 0;
}
```

Bitwise operations are also available as named builtins for
advanced cases: `i64_set_bit(n, pos)`, `i64_clear_bit(n, pos)`,
`i64_toggle_bit(n, pos)`, `i64_test_bit(n, pos)`,
`i64_count_set_bits(n)`, `i64_leading_zeros(n)`,
`i64_trailing_zeros(n)`, `i64_rotate_left(n, by)`.

---

**Previous**: [Sec.1b -- Block comments primer ->](01b_block_comments_primer.md)
**Next**: [Sec.3 -- Functions and the four return aliases ->](03_functions.md)
