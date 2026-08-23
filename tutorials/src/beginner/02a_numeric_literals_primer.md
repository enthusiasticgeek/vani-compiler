# Beginner 2a -- Writing numbers: bases and separators (primer)

> **Learning goal**: know every way vāṇी lets you *spell* a number in
> source code -- decimal, hex, octal, binary, underscore separators,
> and scientific notation -- and when to reach for each. Reading
> order: [Beginner 2 -- Variables, types, operators](02_variables.md)
> -> here -> [Beginner 3 -- Functions](03_functions.md).

This chapter has no new *runtime* behavior. `255`, `0xFF`, and
`0b11111111` all produce the exact same `i64` value -- this is purely
about which spelling is easiest for a human to read in context.

## Same number, different scripts

Think of it like writing the number "one hundred" in different
scripts: `100` (Arabic numerals), `C` (Roman numerals), `१००`
(Devanagari) -- different symbols, identical quantity. A computer's
memory is fundamentally binary, so programmers who work close to the
hardware often want to *see* a value's bit pattern directly instead
of doing decimal-to-binary conversion in their head. vāṇी gives you
four scripts for the same value:

```vani
fn main() -> i64 {
  let decimal: i64 = 255;
  let hex: i64     = 0xFF;      // base 16 -- 2 hex digits per byte
  let octal: i64   = 0o377;     // base 8  -- rare today, historically used for Unix file permissions
  let binary: i64  = 0b11111111; // base 2 -- every digit is one bit

  print decimal;
  print hex;
  print octal;
  print binary;
  // all four print 255 -- same value, four spellings
  return 0;
}
```

| Prefix | Base | When you'd reach for it |
|---|---|---|
| (none) | 10 (decimal) | Everyday counting -- ages, prices, scores, loop bounds |
| `0x` | 16 (hex) | Memory addresses, color codes (`0xFF0000` = red), bitmasks, byte values -- because each hex digit is exactly 4 bits, so a byte is always exactly 2 digits |
| `0o` | 8 (octal) | Rare in new code; mostly seen reading legacy Unix permission bits (`0o755`) |
| `0b` | 2 (binary) | When you want to *see* individual bits -- flags, bitwise-AND/OR/XOR examples like [Beginner 2](02_variables.md)'s |

At least one digit valid for the chosen base must follow the prefix
-- `0o8` (`8` isn't an octal digit) is a compile-time lexer error
("expected octal digits after '0o' prefix"), not a silently-wrong
value.

## Underscore separators

Big decimal numbers are hard to read at a glance -- is `1000000` a
million, or did you miscount a zero? vāṇी lets you drop `_` anywhere
between digits, purely for human eyes; the compiler strips them
before parsing the number:

```vani
fn main() -> i64 {
  let population: i64 = 1_000_000;      // one million, obviously
  let one_million_again: i64 = 1000000; // exactly the same value -- harder to eyeball
  let big_hex: i64 = 0xFFFF_FFFF;       // separators work after a radix prefix too
  print population;
  print one_million_again;
  print big_hex;
  return 0;
}
```

The convention (not enforced by the compiler) is groups of three for
decimal -- `1_000_000`, not `10_00_000` or `1_0_0_0_0_0_0` -- and
whatever grouping makes the bit pattern legible for hex/binary (often
groups of 4 or 8, matching a byte or word boundary).

## Scientific notation (floats only)

`f64`/`f32` literals accept an `e`/`E` exponent suffix -- the same
notation a calculator or a physics textbook uses for very large or
very small numbers:

```vani
fn main() -> i64 {
  let speed_of_light: f64 = 3.0e8;   // 3.0 x 10^8 = 300,000,000
  let tiny: f64 = 2.5e-2;            // 2.5 x 10^-2 = 0.025
  print speed_of_light;
  print tiny;
  return 0;
}
```

This is float-only -- there's no integer equivalent (`1e3` as an
`i64` is a type error, not `1000`). Reach for it when a value's
*order of magnitude* matters more than its exact digits, or when
writing the decimal form out longhand would be error-prone (counting
zeros in `0.000000025` versus reading `2.5e-8`).

## Try it yourself

Write a small program that declares the same value four ways (decimal,
hex, octal, binary) and asserts they're all equal:

```vani
fn main() -> i64 {
  assert 42 == 0x2A;
  assert 42 == 0o52;
  assert 42 == 0b101010;
  print "all four spellings agree";
  return 0;
}
```

Then try `let x: i64 = 0o8;` and read the compiler's error -- confirm
it's caught before the program ever runs, not a silently-wrong value
at runtime.

## Summary

- Four ways to spell an integer: decimal (default), `0x` (hex),
  `0o` (octal), `0b` (binary) -- same value, different human-readable
  script.
- `_` anywhere between digits is a pure readability separator,
  stripped before parsing -- `1_000_000` and `1000000` are identical.
- `e`/`E` scientific notation is float-only (`3.0e8`, `2.5e-2`).
- An invalid digit for the chosen base is a compile-time lexer error,
  never a silently wrong value.

---

**Previous**: [Sec.2 -- Variables, types, operators ->](02_variables.md)
**Next**: [Sec.3 -- Functions and the four return aliases ->](03_functions.md)
