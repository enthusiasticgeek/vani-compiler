# Beginner 8 -- Pattern match on integers + booleans

> **Learning goal**: replace long `if`/`else if` chains with
> `match`, and learn that `match` in vāṇī is an **expression**
> using `then` (not `=>`) between the pattern and the arm body.

> **New to this?** Read [Beginner 8a -- Pattern matching primer](08a_pattern_match_primer.md) first.

Pattern matching is like a post-sorting machine at an airport:
each parcel arrives, gets inspected against a set of routing
rules in order ("is it fragile? -> gate 1", "is it oversized?
-> gate 2", "otherwise -> gate 3"), and the FIRST matching rule
wins. `match` works the same way: the compiler checks each arm
in order and runs the body of the first arm whose pattern fits
the value. No fall-through, no hidden priority -- it's exhaustive
(every possible value must be handled) and unambiguous.

## The program

Before the full worked example, notice what happens if you
drop the wildcard arm from `weekday_name`:

<img class="manas" src="../images/mascot/manas_mascot_error.png" title="this code does not compile!"/>

```vani
fn weekday_name(n: i64) -> Str {
  let name: Str = match n {
    1 then "Monday",
    2 then "Tuesday",
    3 then "Wednesday",
    4 then "Thursday",
    5 then "Friday",
    6 then "Saturday",
    7 then "Sunday",
  };
  return name;
}
```

Without a `_ then ...` arm, the match on `n: i64` isn't
exhaustive -- the compiler can't enumerate every possible
`i64` value, so it rejects the match at compile time. The
worked example below keeps the wildcard arm and compiles
cleanly.

Save this in `~/lesson8.vani`:

<img class="manas" src="../images/mascot/manas_mascot_success.png" title="this is the correct, working version"/>

```vani
intent "Lesson 8 worked example -- match on integers and booleans.";

fn weekday_name(n: i64) -> Str {
  let name: Str = match n {
    1 then "Monday",
    2 then "Tuesday",
    3 then "Wednesday",
    4 then "Thursday",
    5 then "Friday",
    6 then "Saturday",
    7 then "Sunday",
    _ then "unknown",
  };
  return name;
}

fn yes_or_no(b: bool) -> Str {
  let s: Str = match b {
    true then "yes",
    false then "no",
  };
  return s;
}

fn classify(n: i64) -> Str {
  let s: Str = match n {
    0 then "zero",
    1 then "one",
    2 then "two",
    _ then "many",
  };
  return s;
}

fn main() -> i64 {
  print weekday_name(1);
  print weekday_name(5);
  print weekday_name(8);

  print yes_or_no(true);
  print yes_or_no(false);

  print classify(0);
  print classify(1);
  print classify(7);
  return 0;
}
```

## Compile + run

```bash
vanic run ~/lesson8.vani
```

Expected output:

```
Monday
Friday
unknown
yes
no
zero
one
many
```

## Why it works that way

- **`match` is an expression**, not a statement. You bind its
  result with `let`: `let s = match x { ... };`. There is **no
  statement-form `match`** in v1 -- the arms can't contain
  `return` directly.
- **The arm separator is `then`**, not `=>` or `->`. The full
  syntax is `<pattern> then <expr>,`. Trailing comma on the last
  arm is allowed.
- **`_` is the wildcard pattern**. It matches anything not
  matched by an earlier arm. Without it, the compiler rejects a
  non-exhaustive match on `i64` since you can't enumerate every
  integer.
- **All arm bodies must have the same type**. `match` is an
  expression whose type is the common type of every arm; mixing
  `Str` and `i64` arms is a type error.
- **`true` and `false` patterns** make `match` on a `bool`
  exhaustive without needing `_`.
- **Why `then` instead of `=>`?** It reads aloud naturally --
  *"match `n`: case 1 then `Monday`, case 2 then `Tuesday`, ..."* --
  which fits vāṇī's "code as speech" philosophy. Sanskrit /
  Hindi / Marathi files use the same `तदा` / `तो` / `तर` keyword
  in the same slot.

## Challenge

Rewrite the `is_yes(s: Str) -> bool` function from Sec.6 using a
single `match` on the string argument. Note that `match` on a
`Str` works the same way as `match` on an integer -- you just
match against string literals.

<details>
<summary>Solution</summary>

```vani
fn is_yes(s: Str) -> bool {
  let ans: bool = match s {
    "y" then true,
    "yes" then true,
    "Y" then true,
    "YES" then true,
    _ then false,
  };
  return ans;
}
```

</details>

---

**Previous**: [Sec.8a -- Pattern matching primer ->](08a_pattern_match_primer.md)
**Next**: [Sec.8b -- Errors as values ->](08b_errors_primer.md)
**Or skip to**: [Sec.9 -- First contract: `assert` / `prove` ->](09_smt_intro.md)
