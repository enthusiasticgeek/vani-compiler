# Beginner 8a -- Pattern matching (intuition primer)

> **Learning goal**: build a mental model of "pattern matching"
> -- a control-flow form more flexible than `if`/`else` chains
> AND more readable than nested switches. Reading order: this
> is foundational for chapter 8 + intermediate's enums-with-
> payloads. Read it before
> [Beginner 8 -- Pattern match on integers + booleans](08_match.md).

This chapter leads with intuition, not syntax drills — but real
`match` code shows up throughout to ground the analogy, including a
brief look at `enum` declarations (full chapter later — see the
callout further down before you get there).

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

### Pattern 2: what vāṇी doesn't destructure

<img class="manas" src="../images/mascot/manas_mascot_caution.png" title="this code needs extra care"/>

Rust-style `match` can destructure a tuple shape directly:
`match (x, y) { (0, 0) => ..., (a, b) => ... }`. **vāṇी's v1
`match` can't** -- patterns are limited to enum variants
(single-level, one bound name), integer/bool/string/float
literals, and Vec/array slice patterns. There's no tuple
pattern and no way to bind a name to "whatever didn't match a
literal" except the scrutinee's own name (next section).

For tuple-shaped dispatch, use plain `if`/`else` on the
components instead -- exactly the "naive way" from the top of
this chapter:

```vani
fn describe_point(x: i64, y: i64) -> Str {
  if x == 0 && y == 0 {
    return "origin";
  } else if x == 0 {
    return "y-axis";
  } else if y == 0 {
    return "x-axis";
  }
  return "point";
}
```

### Pattern 3: a wildcard with a guard

```vani
fn classify(status: i64) -> Str {
  return match status {
    0 then "ok",
    _ if status < 0 then "error",
    _ then "warning",
  };
}
```

A `_` arm can carry a guard (`if condition`) -- the branch
runs only when the guard is true, and the guard (and the
branch body) can freely reference `status`, the scrutinee's
own name, which is still in scope. This is also why you don't
write `n if n < 0` here the way you might in Rust: vāṇी match
patterns don't introduce a fresh binding for "whatever this
arm caught" -- only enum-variant and slice patterns bind new
names. When you need the value inside a catch-all-style arm,
reach for the original scrutinee variable, not a pattern-bound
name.

## Exhaustiveness -- the compiler-checked guarantee

A loose `if`/`else` chain might handle 4 cases and forget the
5th -- a runtime "fell through everything" silently-wrong-
behavior. With `match`, the compiler **checks that every
possible value is covered**.

> **New syntax ahead**: `enum Name { VariantA, VariantB, ... }`
> declares a type that's exactly ONE of a fixed set of named
> variants — `Color` below is `Red` OR `Green` OR `Blue`, never more
> than one at a time. That's also what `Result<i64, ParseError>`
> from Pattern 1 above already was — `Ok(...)` or `Err(...)`, never
> both — just built into the language instead of hand-declared. The
> "why this shape, why call it a sum" explanation is a few
> paragraphs down; `enum` itself gets a full chapter later
> ([Intermediate 2](../intermediate/02_enums_payloads.md)).

<img class="manas" src="../images/mascot/manas_mascot_error.png" title="this code does not compile!"/>

```vani
enum Color { Red, Green, Blue }

fn name(c: Color) -> Str {
  return match c {
    Color.Red then "red",
    Color.Green then "green",
    // forgot Blue!
  };
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

### Default that still uses the value

```vani
let response: i64 = match input_kind {
  "ping" then 100,
  "echo" then 200,
  _ then handle_unknown(input_kind),
                // ^ reads the scrutinee directly, not a pattern binding
};
```

Same as `_` (catch-all), but the branch body reaches past the
`match` to read `input_kind` -- the variable being matched --
directly, since it's still an ordinary in-scope binding. There
is no way to attach a fresh name to "the value this wildcard
arm caught" the way Rust's `other =>` does; `_` never binds
anything, in any position.

### Nested variants -- one level at a time

<img class="manas" src="../images/mascot/manas_mascot_caution.png" title="this code needs extra care"/>

Rust can peel two layers of variant in one pattern:
`Ok(Command::Echo(s)) =>`. **vāṇी can't** -- `EnumName.Variant(binding)`
patterns take exactly one plain binding name, never another
pattern. Write nested dispatch as two flat matches instead --
extract the inner value with a plain binding in the outer
match, then match on it separately (a real, ordinary
function call, so it composes and is independently testable):

```vani
enum Command { Quit, Status, Echo(i64) }
enum ParseResult { Ok(Command), Err(i64) }

fn dispatch_command(cmd: Command) -> i64 {
  return match cmd {
    Command.Quit then 0,
    Command.Status then 1,
    Command.Echo(payload) then payload,
  };
}

fn dispatch(pr: ParseResult) -> i64 {
  return match pr {
    ParseResult.Ok(cmd) then dispatch_command(cmd),
    ParseResult.Err(code) then 0 - code,
  };
}
```

### Range-like dispatch: chained guards

```vani
fn describe(code: i64) -> Str {
  return match code {
    0 then "ok",
    _ if code >= 1 && code <= 99 then "informational",
    _ if code >= 100 && code <= 199 then "redirect",
    _ if code >= 200 && code <= 299 then "client error",
    _ then "server error",
  };
}
```

vāṇी has no `1..99`-style range pattern syntax. The real
idiom is a chain of guarded `_` arms, each testing a range with
a plain boolean condition, ending in an unguarded `_` as the
final fallback -- exactly the "wildcard with a guard" shape
from Pattern 3 above, chained as many times as you need.

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
