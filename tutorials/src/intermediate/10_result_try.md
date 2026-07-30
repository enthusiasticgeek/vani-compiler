# Intermediate 10 -- Error handling: `Result<T, E>` + `try`

> **Learning goal**: model fallible computations with a
> Result-style enum, chain operations that short-circuit on
> error, and understand where the `try` keyword fits today.

> **New to this?** Read [Intermediate 10a -- Result and `try` primer](10a_result_try_primer.md)
> and [Intermediate 10b -- Runtime errors primer](10b_runtime_errors_primer.md) first.

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
intent "Intermediate 10 worked example -- Result + manual propagation.";

fn parse_pos(n: i64) -> Result<i64, i64> {
  if n < 0 {
    return Result.Err(0 - 1);
  }
  return Result.Ok(n);
}

fn double(n: i64) -> Result<i64, i64> {
  return parse_pos(n * 2);
}

// Manual Result propagation: pattern-match each step,
// short-circuit on Err.
fn pipeline(n: i64) -> Result<i64, i64> {
  let step1: Result<i64, i64> = parse_pos(n);
  return match step1 {
    Result.Ok(v) then double(v),
    Result.Err(e) then Result.Err(e),
  };
}

fn unwrap_or(r: Result<i64, i64>, def: i64) -> i64 {
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

- **`Result<T, E>` IS a built-in generic enum** in v1 --
  `Result.Ok(v)` / `Result.Err(e)` work directly with no
  declaration needed, confirmed directly (this used to be
  documented as "declare your own" in an earlier draft of this
  chapter; that's no longer accurate -- the generic ships
  built-in, same as `Option<T>`). You're still free to declare
  your own differently-shaped enum for a specific error family
  when `Result<T, E>`'s exact shape doesn't fit, using the same
  `enum Name { Variant(Payload), ... }` syntax from
  [Intermediate 2](02_enums_payloads.md).
- **Manual propagation** uses `match` to either extract the
  `Ok` value or rebuild and return `Err` unchanged. The pattern
  is repetitive but predictable.
- **`unwrap_or`** is the idiomatic "extract or default"
  helper -- return the inner value on `Ok`, the default on
  `Err`.

## The `try` keyword and `?` (works for `Option<T>`, not yet for `Result<T, E>`)

vāṇी reserves `try EXPR` and the postfix `?` for the standard
short-circuit propagation. **They already work today -- but only
for enums shaped like `Option<T>`** (exactly one payloaded
variant + one payload-less variant), confirmed directly on both
backends -- see [Intermediate 10a](10a_result_try_primer.md) for
the full story and a working `Option<T>` example. `Result<T, E>`
specifically doesn't qualify: both `Ok(T)` and `Err(E)` are
payloaded, so `try`/`?` reject it with a shape-mismatch
diagnostic. Until `Result<T, E>` support lands, the manual
`match` pattern from the previous section IS the idiom for
`Result`-returning chains. See
`examples/language/english/option_error_propagation.vani` for
the long-form manual style on `Opt`-shaped enums, and
[10c -- multi-error patterns](10c_error_patterns_primer.md) for
composing across multiple error types.

Using `try` on `Result<T, E>`, in a sync function body, is
exactly the rejected case:

<img class="manas" src="../images/mascot/manas_mascot_error.png" title="this code does not compile!"/>

```vani
fn pipeline_try(n: i64) -> Result<i64, i64> {
  let v: i64 = try parse_pos(n);   // rejected -- see below
  return Result.Ok(v * 2);
}
```

```
error: `try` requires the enum 'Result' to have exactly one
       payloaded variant and one payload-less variant; got 2
       payloaded and 0 payload-less.
error: `try EXPR` is reserved as a keyword but the desugar to
       match-with-early-return is still in progress (T2.6 Phase 2).
       Write the pattern manually: `match opt { Opt.Some(v) then v,
       Opt.None then return Opt.None };`
  let v: i64 = try parse_pos(n);
               ^^^^^^^^^^^^^^^^
```

Both diagnostics fire together, confirmed directly: the
shape-mismatch check and the T2.6-phase-2 desugar-gap check both
run and both report.

## v1 limitations to keep in mind

- **`try`/`?` need `Option<T>`'s shape**. `Result<T, E>` (two
  payloaded variants) isn't supported yet -- see above.
- **Enum payload extraction goes through `match` or `if let`**.
  There is no `let Ok(v) = r;` destructuring syntax for enum
  payloads directly in a `let` statement in v1 (confirmed
  directly: "expected '='"); use a `match` arm, or `if let
  Result.Ok(v) = r { ... }`, to bind the inner value.

## Challenge

Add a `safe_sqrt(n: i64) -> Result<i64, i64>` helper that returns
`Err` for negative inputs and `Ok(approximate_sqrt)` otherwise.
Chain it into `pipeline` so the full sequence
`parse_pos -> double -> safe_sqrt` propagates errors at any step.

---

**Previous**: [Sec.10b -- Runtime errors and panic-free design primer ->](10b_runtime_errors_primer.md)
**Next**: [Sec.10c -- Error patterns: nested errors, context, and FFI translation ->](10c_error_patterns_primer.md)
