# Beginner 5 â€” `while` and `for` loops

> **Learning goal**: write a `while` loop with an explicit
> condition, a `for ... from ... to ...` range loop, and use
> `break` / `return` to exit early.

> **New to this?** Read [Beginner 5a â€” Recursion primer](05a_recursion_primer.md) first
> for why we repeat things â€” then come back here for the loop syntax.

A loop is just an instruction you give a very obedient assistant:
"keep doing X until I tell you to stop." A `while` loop says
"keep going *while* this condition is true." A `for` loop with a
range says "do this for each number from A to B." When you
hit `break` it's like saying "stop now, we're done" and walking
out the door mid-task.

## The program

Save this in `~/lesson5.vani`:

```vani
intent "Lesson 5 worked example â€” while and for loops.";

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

- **`while <bool> { â€¦ }`** runs the body until the condition is
  false. Don't forget to advance the loop variable (`i = i + 1`)
  or the loop is infinite. There's no postfix `i++` â€” the
  language prefers spelled-out arithmetic.
- **`for k from lo to hi { â€¦ }`** is a half-open range:
  `k` takes values `lo, lo+1, â€¦, hi-1`. The upper bound is
  *exclusive*, which is why `product_of_range(1, 5)` computes
  `1 * 2 * 3 * 4 = 24`, not `1 * 2 * 3 * 4 * 5 = 120`.
- **`while true { â€¦ }`** loops indefinitely. Combine with
  `return` (or `break`) to exit. The compiler proves
  reachability: the final `return 0 - 1;` after a
  `while true` is reachable for the type-checker but unreachable
  at runtime, and that's fine.
- **`break;`** exits the nearest enclosing loop;
  **`continue;`** skips to the next iteration.
- **There's no `let mut`** â€” `let i: i64 = 0;` declares a
  variable that's already mutable via `i = i + 1`. v1 treats
  every `let` binding as mutable for primitives; the `mut`
  keyword is reserved for `mut ref` parameters in Â§3 of the
  Intermediate track. This is a documented v1 deviation â€”
  see [`docs/v1_limitations.md`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/docs/v1_limitations.md).

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

**Next**: [Â§6 â€” Strings (`Str` vs `OwnedStr`) â†’](06_strings.md)
