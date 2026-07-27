# Beginner 8a -- Pattern matching (intuition primer)

> **Learning goal**: build a mental model of "pattern matching"
> -- a control-flow form more flexible than `if`/`else` chains
> AND more readable than nested switches. Reading order: this
> is foundational for chapter 8 + intermediate's enums-with-
> payloads. Read it before
> [Beginner 8 -- Pattern match on integers + booleans](08_match.md).

This chapter has **no compiler code**. Pure intuition.

## The post-office sorting clerk

Picture a clerk at a post office sorting an incoming pile of mail.
Each item is different: a postcard, a letter, a small parcel, a
tube with a poster rolled up inside. The clerk doesn't ask fifty
yes/no questions per item ("is it a postcard? no. is it a letter?
no. is it a parcel? ..."). They glance at the item's *shape*, and
that single glance tells them which bin it goes in AND what to do
next:

- **Postcard** -> straight into the "local mail" bin, no further
  handling.
- **Letter** -> check the return-address label, then into the
  matching city bin.
- **Small parcel** -> weigh it, then route by weight class.
- **Tube** -> handle with care, separate bin.

Notice two things the clerk is doing at once: (1) figuring out
*which kind* of item this is, by its shape, and (2) *pulling out
the specific detail* they need from it (the address on a letter,
the weight of a parcel) in the same glance. They don't sort first
and then separately go dig for the address -- recognizing the shape
and extracting the useful part happen together.

That's exactly what `match` does with a value in code: it looks at
*what kind of thing* the value is, and in the same step, pulls out
whatever's inside it under a name you can use. The rest of this
chapter is that sorting clerk, spelled out as code.

## What `match` lets you do

You have a value. You want to do different things depending on
what's in it. The naive way is a chain of `if`/`else`:

```vani
if x == 0 {
  print "zero";
} else if x == 1 {
  print "one";
} else if x >= 2 && x < 10 {
  print "small";
} else if x >= 10 {
  print "big";
} else {
  print "negative";
}
```

Works, but:
- The variable `x` is repeated five times.
- It's not visually obvious that these branches are
  "mutually exclusive alternatives on the same value."
- Adding a new case means writing another `else if x == ...`.

`match` lets you write the same thing as:

```vani
match x {
  0 then print "zero",
  1 then print "one",
  2 then print "small",   // (etc., one per value)
  _ then print "other",   // wildcard for everything else
}
```

The variable is mentioned ONCE. Each branch is a *pattern*
followed by `then` followed by an action. The visual layout
reflects what the code is doing: "look at x; pick the
matching branch."

## Why `match` is more powerful than if/else

For simple equality (`x == 0`), `match` is just visual
syntax. The real power kicks in when patterns become
**destructuring**.

### Pattern 1: extract the payload from an enum variant

You have a `Result<i64, ParseError>`. You want to handle both
cases AND pull out the inner value:

```vani
match parse_result {
  Result.Ok(n) then print "parsed:", n,
  Result.Err(e) then print "failed:", e.code,
}
```

The `(n)` in `Ok(n)` introduces a NEW LOCAL BINDING. When
this branch runs, `n` is the i64 inside the Ok. You didn't
have to call `.unwrap()` or do a `if let` dance -- the pattern
extracted the value.

Same for `(e)` in `Err(e)` -- `e` is the ParseError inside the
Err.

### Pattern 2: tuples

```vani
match (x, y) {
  (0, 0) then print "origin",
  (0, _) then print "y-axis",
  (_, 0) then print "x-axis",
  (a, b) then print "point at", a, b,
}
```

The pattern `(a, b)` destructures the tuple and binds the
components. `_` is the don't-care placeholder.

### Pattern 3: literal + binding combined

```vani
match status {
  0 then print "ok",
  n if n < 0 then print "error:", n,    // n is bound;
                                        //  guard adds a condition
  n then print "warning code:", n,
}
```

A pattern can bind a name AND have a guard (`if condition`).
The branch runs only if both the pattern matches AND the
guard is true.

## Exhaustiveness -- the compiler-checked guarantee

A loose `if`/`else` chain might handle 4 cases and forget the
5th -- a runtime "fell through everything" silently-wrong-
behavior. With `match`, the compiler **checks that every
possible value is covered**.

<img class="manas" src="../images/mascot/manas_mascot_error.png" title="this code does not compile!"/>

```vani
enum Color { Red, Green, Blue }

fn name(c: Color) -> Str {
  match c {
    Color.Red then "red",
    Color.Green then "green",
    // forgot Blue!
  }
}
```

Compile error: "match is not exhaustive -- variant Blue not
covered". You can't accidentally drop a case. Either you
handle each variant, or you add `_ then default-value`
explicitly to acknowledge you're catching the rest.

Add the missing arm and the same match compiles:

<img class="manas" src="../images/mascot/manas_mascot_success.png" title="this is the correct, working version"/>

```vani
enum Color { Red, Green, Blue }

fn name(c: Color) -> Str {
  return match c {
    Color.Red then "red",
    Color.Green then "green",
    Color.Blue then "blue",
  };
}
```

This is one of the most useful properties of `match`. As your
enum grows (you add a new variant `Blue` to `Color`), every
`match` in your codebase that doesn't have a wildcard becomes
a compile error -- forcing you to update each match site to
handle the new variant. You CAN'T forget.

## When the value is "or" -- match is the natural fit

Sums (one of these N variants) and products (this AND that
combined) are the two basic ways of building data types.

- **Product**: a struct has X and Y and Z all at once.
- **Sum**: an enum is X or Y or Z (exactly one).

`match` is THE way to destructure a sum. Each arm names one
variant.

Many real-world types are sums:
- `Option<T>` = `Some(T)` or `None`
- `Result<T, E>` = `Ok(T)` or `Err(E)`
- `Shape` = `Circle(...)` or `Square(...)` or `Triangle(...)`
- `Json` = `Number(...)` or `String(...)` or `Array(...)` or
  `Object(...)` or `Null`

Functions that consume sums almost always start with `match
input`.

## `match` as an expression, not a statement

A subtle but important property: in vāṇी, `match` is an
EXPRESSION. It produces a value.

```vani
let name: Str = match c {
  Color.Red then "red",
  Color.Green then "green",
  Color.Blue then "blue",
};
```

Each arm's body produces the same-typed value; the match's
value is whichever arm matched. No `if`/`else` ternary needed;
no separate variable initialized in each branch.

This means you can use match wherever a value is expected --
return position, function arguments, struct fields:

```vani
fn describe(c: Color) -> Str {
  return match c {
    Color.Red then "warm",
    Color.Green then "calm",
    Color.Blue then "cool",
  };
}
```

## Common patterns in practice

### Default with a name

```vani
let response: i64 = match input_kind {
  "ping" then 100,
  "echo" then 200,
  other then handle_unknown(other),
                // ^ `other` is bound to the unmatched value
};
```

Same as `_` (catch-all) but with a name so you can use the
value in the branch body.

### Match on a structured Result

```vani
match parse_command(buf) {
  Result.Ok(Command.Quit) then exit(0),
  Result.Ok(Command.Status) then print_status(),
  Result.Ok(Command.Echo(s)) then print s,
  Result.Err(e) then print "bad command:", e,
}
```

Nested patterns -- `Ok(Command.Echo(s))` peels two layers:
"the Result is Ok, AND the inner Command is Echo, AND extract
the inner Str into `s`."

### Range patterns (where supported)

```vani
match code {
  0 then "ok",
  1..99 then "informational",
  100..199 then "redirect",
  200..299 then "client error",
  _ then "server error",
}
```

(`..` syntax for ranges; check the formal chapter for the
exact spelling vāṇी uses.)

## A summary you can carry

- **`match`** = compact, exhaustiveness-checked alternative to
  `if`/`else` chains.
- **Patterns** destructure values into pieces, binding names
  to inner parts (especially enum variants).
- The compiler **enforces exhaustiveness** -- you can't forget
  to handle a variant. Add a variant later -> every non-
  wildcard match becomes a compile error until updated.
- `match` is an **expression** -- produces a value. Use it
  inline in return positions, struct fields, etc.
- Default to `match` for sum types (enum, Result, Option) and
  for "many-way dispatch on a value" -- even when `if`/`else`
  would work, `match` reads more clearly.

That's pattern matching. The next chapter ([Beginner 8](08_match.md))
shows the basic syntax for integers + booleans; the
intermediate-track chapter ([Intermediate 2](../intermediate/02_enums_payloads.md))
covers enum-with-payload destructuring in depth.

## Cross-reference

- [Beginner 8 -- Pattern match on integers + booleans](08_match.md)
  -- basic syntax
- [Intermediate 2 -- Enums with payloads + match arms](../intermediate/02_enums_payloads.md)
  -- payloaded-enum destructuring; the most common match use case
- [Intermediate 10a -- Result and try primer](../intermediate/10a_result_try_primer.md)
  -- Result/Option destructuring is half "match", half
  `try`/`?` sugar


---

**Previous**: [Sec.7 -- Arrays and Vec<T> basics ->](07_vec_arrays.md)
**Next**: [Sec.8 -- Pattern match on integers + booleans ->](08_match.md)
