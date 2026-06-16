# Intermediate 10 — Error handling: `Result<T, E>` + `try`

> **Learning goal**: model fallible computations with a
> Result-style enum, chain operations that short-circuit on
> error, and understand where the `try` keyword fits today.

> **New to this?** Read [Intermediate 10a — Result and `try` primer](10a_result_try_primer.md)
> and [Intermediate 10b — Runtime errors primer](10b_runtime_errors_primer.md) first.

Imagine every step in a recipe either succeeds ("the dough rose")
or fails ("the oven broke"). A `Result` is a small envelope that
holds either the success value (`Ok(...)`) or a description of
what went wrong (`Err(...)`). Instead of checking for failure
after every step with a chain of `if` statements, you mark a
function with `try` and the compiler automatically short-circuits
to the error branch the moment any step returns `Err`. The
kitchen closes, you report what broke, and callers decide how to
handle it.

## The program

```vani
intent "Intermediate 10 worked example — Result + manual propagation.";

enum Result { Ok(i64), Err(i64) }

fn parse_pos(n: i64) -> Result {
  if n < 0 {
    return Result.Err(0 - 1);
  }
  return Result.Ok(n);
}

fn double(n: i64) -> Result {
  return parse_pos(n * 2);
}

// Manual Result propagation: pattern-match each step,
// short-circuit on Err.
fn pipeline(n: i64) -> Result {
  let step1: Result = parse_pos(n);
  return match step1 {
    Result.Ok(v) then double(v),
    Result.Err(e) then Result.Err(e),
  };
}

fn unwrap_or(r: Result, def: i64) -> i64 {
  return match r {
    Result.Ok(v) then v,
    Result.Err(_) then def,
  };
}

fn main() -> i64 {
  print "pipeline(5)   =", unwrap_or(pipeline(5), 0 - 999);
  print "pipeline(-3)  =", unwrap_or(pipeline(0 - 3), 0 - 999);
  return 0;
}
```

## Compile + run

```bash
vanic run ~/int10.vani
```

Output:

```
pipeline(5)   = 10
pipeline(-3)  = -999
```

## Why it works that way

- **There's no built-in `Result<T, E>`** in v1 — you declare
  your own per-function-family enum (per Intermediate §2).
  The convention `enum Result { Ok(i64), Err(i64) }` mirrors
  Rust's `Result<T, E>` shape.
- **Manual propagation** uses `match` to either extract the
  `Ok` value or rebuild and return `Err` unchanged. The pattern
  is repetitive but predictable.
- **`unwrap_or`** is the idiomatic "extract or default"
  helper — return the inner value on `Ok`, the default on
  `Err`.

## The `try` keyword (queued sugar)

vāṇी reserves the `try` keyword for the standard Rust-style
short-circuit:

```vani
let v: i64 = try parse_pos(n);   // queued sugar
```

…desugars to roughly:

```vani
let __t = parse_pos(n);
if /* __t is Err */ { return __t; }
let v: i64 = /* the Ok payload */;
```

In v1 the desugar is enabled **only inside async fn bodies**
(Arc 8 v3.1 Phase 2.4). For ordinary synchronous code you write
the manual `match` shown above; the `try` sugar for sync
contexts is a future-track item. See
`examples/language/english/option_error_propagation.vani` for
the long-form manual style on `Opt`-shaped enums.

## v1 limitations to keep in mind

- **No generic `Result<T, E>`**. v1's enums don't carry type
  parameters yet, so each fallible API declares its own
  Result-shaped enum with concrete payload types.
- **No enum-destructure in `let`** ([L1 in v1_limitations.md](https://github.com/anthropics/claude-code/blob/main/docs/v1_limitations.md))
  — every extraction goes through `match`.

## Challenge

Add a `safe_sqrt(n: i64) -> Result` helper that returns
`Err` for negative inputs and `Ok(approximate_sqrt)` otherwise.
Chain it into `pipeline` so the full sequence
`parse_pos → double → safe_sqrt` propagates errors at any step.

---

**Next**: [§11 — The 22 GoF design patterns →](11_design_patterns.md)
