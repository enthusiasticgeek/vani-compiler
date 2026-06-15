# Intermediate 2 — Enums with payloads + match arms

> **Learning goal**: declare a tagged-union enum, construct
> variants with payload data, and destructure them with
> `match` arms.

## The program

```vani
intent "Intermediate 2 worked example — enums with payloads.";

enum Result { Ok(i64), Err(i64) }

fn safe_div(a: i64, b: i64) -> Result {
  if b == 0 {
    return Result.Err(0 - 1);
  }
  return Result.Ok(a / b);
}

fn unwrap_or(r: Result, def: i64) -> i64 {
  return match r {
    Result.Ok(v) then v,
    Result.Err(_) then def,
  };
}

fn main() -> i64 {
  let r1: Result = safe_div(20, 4);
  let r2: Result = safe_div(10, 0);
  print "20/4 =", unwrap_or(r1, 0 - 999);
  print "10/0 =", unwrap_or(r2, 0 - 999);
  return 0;
}
```

## Compile + run

```bash
vanic run ~/int2.vani
```

Output:

```
20/4 = 5
10/0 = -999
```

## Why it works that way

- **`enum Name { V1(T1), V2(T2), V3 }`** declares a tagged union.
  Each variant can be either a tag (no payload) or a tag+payload.
  The payload type goes in parentheses after the variant name.
- **v1 restriction**: a single payload type *per variant* only —
  multi-payload tuples (`Ok(i64, Str)`) aren't supported. Wrap
  multi-field variants in a struct instead, and put the struct
  type in the payload.
- **Construction**: `Result.Ok(42)` — note the dot, not the
  double-colon. This is one of the small surface-syntax diffs
  from Rust.
- **Match destructuring**: `Result.Ok(v) then v` extracts the
  payload as a fresh `v` binding scoped to the arm. `Result.Err(_)`
  matches but discards the payload.
- **Match is an expression** (Beginner §8). Return its value
  with `return match ... { ... };`.

## v1 limitations to know about

These are listed in [`docs/v1_limitations.md`](https://github.com/anthropics/claude-code/blob/main/docs/v1_limitations.md) — keep them in mind:

- **No enum-destructure in `let`**: `let Result.Ok(v) = r;`
  doesn't work. You always go through `match`.
- **No nested patterns**: `Some(Some(v))` patterns aren't
  supported; flatten with two `match` levels.
- **`Box<T>` is unsupported**: recursive enums (`Tree(Box<Tree>,
  Box<Tree>)`) need a workaround using arena indices. The
  Composite design pattern example shows the tagged-struct
  workaround.

## Challenge

Define `enum Color { Red, Green, Blue, Custom(i64) }` and a
function `brightness(c: Color) -> i64` that returns 100 for
`Red`, 80 for `Green`, 60 for `Blue`, and the payload itself
for `Custom(n)`. Print results for several inputs.

<details>
<summary>Solution</summary>

```vani
enum Color { Red, Green, Blue, Custom(i64) }

fn brightness(c: Color) -> i64 {
  return match c {
    Color.Red then 100,
    Color.Green then 80,
    Color.Blue then 60,
    Color.Custom(n) then n,
  };
}

fn main() -> i64 {
  print brightness(Color.Red);
  print brightness(Color.Custom(42));
  return 0;
}
```

</details>

---

**Next**: [§3 — Affine ownership →](03_affine.md)
