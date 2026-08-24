# Beginner 5 -- `while` and `for` loops

> **Learning goal**: write a `while` loop with an explicit
> condition, a `for ... from ... to ...` range loop, and use
> `break` / `return` to exit early.

> **New to this?** Read [Beginner 5a -- Recursion primer](05a_recursion_primer.md) first
> for why we repeat things -- then come back here for the loop syntax.

A loop is just an instruction you give a very obedient assistant:
"keep doing X until I tell you to stop." A `while` loop says
"keep going *while* this condition is true." A `for` loop with a
range says "do this for each number from A to B." When you
hit `break` it's like saying "stop now, we're done" and walking
out the door mid-task.

## The program

Save this in `~/lesson5.vani`:

```vani
intent "Lesson 5 worked example -- while and for loops.";

fn sum_to_n(n: i64) -> i64 {
  let total: i64 = 0;
  let i: i64 = 1;
  while i <= n {
    total = total + i;
    i = i + 1;
  }
  return total;
}

fn product_of_range(lo: i64, hi: i64) -> i64 {
  let prod: i64 = 1;
  for k from lo to hi {
    prod = prod * k;
  }
  return prod;
}

fn first_multiple_of_seven(start: i64) -> i64 {
  let n: i64 = start;
  while true {
    if n % 7 == 0 {
      return n;
    }
    n = n + 1;
  }
  return 0 - 1;
}

fn main() -> i64 {
  print "sum_to_n(10) =", sum_to_n(10);
  print "product_of_range(1, 5) =", product_of_range(1, 5);
  print "first_multiple_of_seven(20) =", first_multiple_of_seven(20);
  return 0;
}
```

## Compile + run

```bash
vanic run ~/lesson5.vani
```

Expected output:

```
sum_to_n(10) = 55
product_of_range(1, 5) = 24
first_multiple_of_seven(20) = 21
```

## Why it works that way

- **`while <bool> { ... }`** runs the body until the condition is
  false. Don't forget to advance the loop variable (`i = i + 1`)
  or the loop is infinite. There's no postfix `i++` -- the
  language prefers spelled-out arithmetic.
- **`for k from lo to hi { ... }`** is a half-open range:
  `k` takes values `lo, lo+1, ..., hi-1`. The upper bound is
  *exclusive*, which is why `product_of_range(1, 5)` computes
  `1 * 2 * 3 * 4 = 24`, not `1 * 2 * 3 * 4 * 5 = 120`.
- **`while true { ... }`** loops indefinitely. Combine with
  `return` (or `break`) to exit. The compiler proves
  reachability: the final `return 0 - 1;` after a
  `while true` is reachable for the type-checker but unreachable
  at runtime, and that's fine.
- **`break;`** exits the nearest enclosing loop;
  **`continue;`** skips to the next iteration.
- **There's no `let mut`** -- `let i: i64 = 0;` declares a
  variable that's already mutable via `i = i + 1`. v1 treats
  every `let` binding as mutable for primitives; the `mut`
  keyword is reserved for `mut ref` parameters in Sec.3 of the
  Intermediate track. This is a documented v1 deviation --
  see [`docs/v1_limitations.md`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/docs/v1_limitations.md).

### Counting down with `downto`, or stepping by more than 1

`for k from lo to hi { ... }` only counts **up**, one at a time.
To count down, use its descending counterpart, `downto`, in place
of `to`:

```vani
fn countdown_sum(n: i64) -> i64 {
  let total: i64 = 0;
  for i from n downto 0 {
    total = total + i;
  }
  return total;
}
```

```
countdown_sum(5) = 15
```

`for i from n downto 0` walks `n, n-1, ..., 1` -- **excluding**
`0`, the same half-open convention `to` uses for its own upper
bound (`lo to hi` excludes `hi`; `hi downto lo` excludes `lo`).
That's why `countdown_sum(5)` sums `5, 4, 3, 2, 1 = 15`, not
`5, 4, 3, 2, 1, 0`.

By default `to`/`downto` both step by 1. For a different stride,
add a `step N` clause after the range bounds:

```vani
fn sum_every_third(start: i64, end: i64) -> i64 {
  let total: i64 = 0;
  for i from start to end step 3 {
    total = total + i;
  }
  return total;
}
```

```
sum_every_third(1, 10) = 12
```

`for i from 1 to 10 step 3` walks `1, 4, 7` (sum `12`) -- half-open
still excludes `10`, same as a plain `to` loop excludes its own
upper bound, so `10` itself is never visited even though it lines up
with the stride. Want `1, 4, 7, 10` included instead? Write `for i
from 1 to 11 step 3` -- one past `10` is enough for the half-open
exclusion to land past it, not on it. `step` only ever names the
SIZE of each hop; `to`/`downto` still pick the direction, exactly as
before. `step` works the same way with `downto`:

```vani
fn sum_descending_step(hi: i64, lo: i64, stride: i64) -> i64 {
  let total: i64 = 0;
  for i from hi downto lo step stride {
    total = total + i;
  }
  return total;
}
```

```
sum_descending_step(20, 0, 3) = 77
```

`sum_descending_step(20, 0, 3)` walks `20, 17, 14, 11, 8, 5, 2` (sum
`77`) -- same half-open convention as a plain `downto` (stops once
the next value would reach or pass `lo`), just with a stride of `3`
instead of `1`. Omitting `step` defaults to a stride of 1, identical
to the loops earlier in this chapter.

A `step` of `0` or a negative number is always a mistake -- there's
no direction to infer from it the way an empty `to`/`downto` range
is a legitimate (if unusual) edge case -- so the compiler rejects it
outright at compile time when it's a literal, and traps cleanly at
runtime if it's a variable whose value turns out to be non-positive.
`parallel for` doesn't support `step` (or `downto`) yet either;
both are sequential-only for now.

<img class="manas" src="../images/mascot/manas_mascot_caution.png" title="this code does not do what it looks like it does"/>

**`for ... from ... to` still doesn't count down, even if you
swap the bounds.** `for i from 5 to 0 { ... }` is not a countdown
-- `to` is always ascending, so `5 to 0` is a range with nothing
in it (`5 > 0`, so there's no valid `lo, lo+1, ...` sequence below
the upper bound), and the loop body runs **zero times**, silently
-- no error, and it still *compiles and runs*, but the compiler
now flags it with a warning when both bounds are literal/constant
(added 2026-08-14 -- see below):

```vani
fn main() -> i64 {
  let count: i64 = 0;
  for i from 5 to 0 {
    count = count + 1;
  }
  print "count =", count;   // count = 0, not 5
  return 0;
}
```

This is the same half-open-range rule from the bullet above (`lo
to hi` means `lo, lo+1, ..., hi-1`) -- it just surprises people
more when `lo > hi`, since "count from 5 to 0" reads like English
for "count down." If you meant to count down, write `for i from 5
downto 0` instead.

**A constant-bounds mismatch like this now produces a compiler
warning** (`'for i from 5 to 0' never executes -- 5 > 0, but 'to'
counts UP (needs start <= end)`), printed by `vanic check`/`build`/
`run` but -- deliberately -- **not** a build-blocking error: an
empty range from constant bounds is usually a mistake (swapped
operands, or the wrong keyword), but it's occasionally intentional
(generic code where a length parameter happens to collapse the
range to empty), so the compiler tells you and lets you decide.
Equal bounds (`for i from 3 to 3`) are never flagged -- that's a
completely ordinary empty-range edge case, not a direction mismatch.
A runtime-computed bound (`for i from lo to hi` where `hi` comes
from a function argument) can't be judged at compile time and is
never flagged either, no matter what value it ends up holding.

### A closer look: don't forget to advance

<img class="manas" src="../images/mascot/manas_mascot_caution.png" title="this code needs extra care"/>

```vani
fn count_to(n: i64) -> i64 {
  let i: i64 = 0;
  while i < n {
    print i;
    // forgot: i = i + 1;
  }
  return i;
}
```

This compiles cleanly -- `i < n` is a perfectly valid `bool`
condition -- but nothing inside the loop body changes `i`, so it
never becomes `false`. The loop runs forever. It's the same shape
of bug for `while true { ... }`: the compiler can't tell you "you
forgot to update the loop variable," so it's on you to double-check
that every loop body moves its condition toward `false` (or hits a
`break`/`return`).

## Challenge

Write a `count_digits(n: i64) -> i64` that returns how many
decimal digits `n` has. Use a `while` loop dividing by 10. Test
it on 7, 42, and 12345.

<details>
<summary>Solution</summary>

```vani
fn count_digits(n: i64) -> i64 {
  if n == 0 {
    return 1;
  }
  let count: i64 = 0;
  let m: i64 = n;
  while m > 0 {
    count = count + 1;
    m = m / 10;
  }
  return count;
}
```

For negatives, you'd flip the sign first; the language has no
`abs` keyword but `if n < 0 { m = 0 - n }` is enough.

</details>

---

**Previous**: [Sec.4 -- `if` / `else` ->](04_if_else.md)
**Next**: [Sec.5a -- Recursion intuition primer ->](05a_recursion_primer.md)
