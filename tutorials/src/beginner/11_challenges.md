# Beginner 11 — Challenges

> **Learning goal**: stretch yourself with three small projects
> that combine everything from §1–§10. Each has a worked
> solution at the bottom of its section.

These are meant to be done from a blank file. Read the prompt,
implement, then check your answer against the solution. Aim
to do at least one without peeking.

---

## A — FizzBuzz

Write a program that prints the numbers 1 through 15, but:

- For multiples of 3, print `Fizz` next to the number.
- For multiples of 5, print `Buzz`.
- For multiples of both (i.e. 15), print `FizzBuzz`.
- For all others, print `number`.

Hint: use `%` and `if/else if/else`.

<details>
<summary>Solution</summary>

```rust
intent "FizzBuzz from 1 to 15.";

fn classify(n: i64) -> Str {
  if n % 15 == 0 {
    return "FizzBuzz";
  } else if n % 3 == 0 {
    return "Fizz";
  } else if n % 5 == 0 {
    return "Buzz";
  } else {
    return "number";
  }
}

fn main() -> i64 {
  for i from 1 to 16 {
    print i, classify(i);
  }
  return 0;
}
```

Expected output:

```
1 number
2 number
3 Fizz
4 number
5 Buzz
6 Fizz
…
15 FizzBuzz
```

</details>

---

## B — Vector statistics

Given `let xs: Vec<i64> = vec(4, 9, 1, 7, 3, 8, 2);` write three
free-standing helper functions:

- `total_of(xs: ref Vec<i64>) -> i64` — the sum.
- `find_max(xs: ref Vec<i64>) -> i64` — the largest element.
- `find_min(xs: ref Vec<i64>) -> i64` — the smallest element.

…and print each result. Why these names? `vec_sum` is a
built-in name in vāṇी's standard prelude, and `min` / `max`
are stdlib generic free functions — naming yours `vec_min` /
`vec_max` would shadow useful primitives in larger programs.
Picking distinct verb-style names sidesteps the collision and
reads better when you import them with `use`.

<details>
<summary>Solution</summary>

```rust
intent "Vector statistics — sum, max, min.";

fn total_of(xs: ref Vec<i64>) -> i64 {
  let total: i64 = 0;
  let i: u64 = 0;
  while i < len(xs) {
    total = total + xs[i];
    i = i + 1;
  }
  return total;
}

fn find_max(xs: ref Vec<i64>) -> i64 {
  let best: i64 = xs[0];
  let i: u64 = 1;
  while i < len(xs) {
    if xs[i] > best { best = xs[i]; }
    i = i + 1;
  }
  return best;
}

fn find_min(xs: ref Vec<i64>) -> i64 {
  let best: i64 = xs[0];
  let i: u64 = 1;
  while i < len(xs) {
    if xs[i] < best { best = xs[i]; }
    i = i + 1;
  }
  return best;
}

fn main() -> i64 {
  let xs: Vec<i64> = vec(4, 9, 1, 7, 3, 8, 2);
  print "sum =", total_of(ref xs);
  print "max =", find_max(ref xs);
  print "min =", find_min(ref xs);
  return 0;
}
```

</details>

---

## C — Modular grading rubric

Reorganize the `grade(score: i64) -> Str` function from §4
into a `grading` module with two public functions:

- `pass_threshold() -> i64` returning `70`.
- `grade(score: i64) -> Str` using `pass_threshold()` internally
  for the `C / F` cutoff.

Call `grading::grade(...)` from `main` on a handful of scores
and print results.

<details>
<summary>Solution</summary>

```rust
intent "Modular grading rubric.";

module grading {
  pub fn pass_threshold() -> i64 {
    return 70;
  }

  pub fn grade(score: i64) -> Str {
    if score >= 90 { return "A"; }
    if score >= 80 { return "B"; }
    if score >= pass_threshold() { return "C"; }
    return "F";
  }
}

fn main() -> i64 {
  print grading::grade(95);
  print grading::grade(82);
  print grading::grade(74);
  print grading::grade(60);
  return 0;
}
```

</details>

---

**Next**: [§12 — Devanagari surface — optional intro →](12_devanagari.md)
