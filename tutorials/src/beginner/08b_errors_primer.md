# Beginner 8b -- Errors as values (intuition primer)

> **Learning goal**: understand why vāṇī represents failure as an
> ordinary value rather than throwing an exception. Build a first
> mental model of `Option` so you recognise the pattern before the
> full intermediate treatment.

---

## The two schools of thought

When a function can't do its job, it has to tell the caller somehow.
Two philosophies dominate:

### Exceptions (Python, Java, C++, JavaScript)

A failing function *throws*. Control flow jumps invisibly to whoever
*catches* it -- possibly several frames up the call stack. Normal-path
code and error-path code live in separate places.

Problems:
- Any call can fail invisibly. The type signature doesn't say so.
- The compiler can't verify you handled every failure mode.
- Stack unwinding has hidden cost and hidden control flow.

### Errors as values (vāṇī, Rust, Go)

A failing function returns a *value* that says either "here's the
result" or "here's what went wrong". The caller receives it like any
other value and must actively handle both cases.

```vani
fn safe_div(a: i64, b: i64) -> Option<i64> {
  if b == 0 { return Option.None; }
  return Option.Some(a / b);
}
```

The return type `Option<i64>` advertises the possibility of failure.
The caller cannot accidentally ignore it -- the type says there might
be no value.

---

## `Option<T>` -- the simplest error carrier

`Option<T>` has exactly two variants:

| Variant | Meaning |
|---------|---------|
| `Option.Some(value)` | Success -- here is the result |
| `Option.None` | Failure or absence -- nothing to give back |

Use `match` to handle both cases:

```vani
fn main() -> i64 {
  let result: Option<i64> = safe_div(10, 2);
  match result {
    Option.Some(v) then print "answer = ", v,
    Option.None    then print "division by zero",
  }
  return 0;
}
```

Output: `answer = 5`

Try it with a zero divisor:

```vani
fn main() -> i64 {
  let result: Option<i64> = safe_div(10, 0);
  match result {
    Option.Some(v) then print "answer = ", v,
    Option.None    then print "division by zero",
  }
  return 0;
}
```

Output: `division by zero`

---

## Why `match` is the right tool

`match` is *exhaustive* -- the compiler rejects code that omits a
variant. You cannot accidentally forget the error case:

<img class="manas" src="../images/mascot/manas_mascot_error.png" title="this code does not compile!"/>

```vani
/* compile error: match is not exhaustive -- Option.None not covered */
match result {
  Option.Some(v) then print v,
}
```

This is the key advantage over exceptions: forgetting to handle an
error is a *compile-time* mistake, not a silent runtime surprise.

---

## `assert` -- when failure is a bug, not a user error

Sometimes a condition must be true and if it isn't the program has a
bug. `assert` is for that:

<img class="manas" src="../images/mascot/manas_mascot_caution.png" title="this code needs extra care"/>

```vani
assert len(xs) > 0, "list must not be empty";
```

`assert` calls `abort()` on failure. There is no recovery. Reserve it
for programmer errors (invariants), not user input or external failures.

---

## The full error-handling story

This primer covers the concept. The complete picture is in the
intermediate track:

- **[Intermediate 10a -- Result, `try`, and `?`](../intermediate/10a_result_try_primer.md)** --
  `Result<T, E>` for failures with diagnostic info, and `try`/`?` for
  short-circuiting chains of fallible operations.
- **[Intermediate 13 -- `Option<T>`](../intermediate/13_option.md)** --
  the full Option API including `option_map`, `option_and_then`,
  `option_unwrap_or`.

---

**Previous**: [Sec.8 -- Pattern match on integers + booleans ->](08_match.md)
**Next**: [Sec.9 -- First contract: assert / prove / requires ->](09_smt_intro.md)
