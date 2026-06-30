# Beginner 3 -- Functions and the four return aliases

> **Learning goal**: declare functions, pass parameters by value,
> return a value, and learn that `return` has three Devanagari
> aliases for use in Sanskrit / Hindi / Marathi files.

A function is a named recipe. You write it once, give it a name,
and call it whenever you need the same task done. `fn add(a, b)`
is like a recipe card labelled "Add two numbers": the card takes
two ingredients (`a` and `b`) and produces one result (the sum).
Calling `add(3, 4)` is like saying "follow the Add recipe with
ingredients 3 and 4" -- it produces 7 and hands it back to whoever
asked. Functions let you avoid copying the same instructions
everywhere; change the recipe once and every place that uses it
automatically gets the update.

## The program

Save this in `~/lesson3.vani`:

```vani
intent "Lesson 3 worked example -- functions, parameters, return.";

fn add(a: i64, b: i64) -> i64 {
  return a + b;
}

fn double(n: i64) -> i64 {
  return n * 2;
}

fn area_of_circle(r: f64) -> f64 {
  return 3.14159 * r * r;
}

fn shout(msg: Str) -> i64 {
  print msg;
  return 0;
}

fn main() -> i64 {
  let sum: i64 = add(3, 4);
  print "add(3, 4) =", sum;

  let composed: i64 = double(add(2, 3));
  print "double(add(2, 3)) =", composed;

  let circle_area: f64 = area_of_circle(2.5);
  print "area(2.5) =", circle_area;

  shout("hello from a helper");
  return 0;
}
```

## Compile + run

```bash
vanic run ~/lesson3.vani
```

Expected output:

```
add(3, 4) = 7
double(add(2, 3)) = 10
area(2.5) = 19.6349
hello from a helper
```

## Why it works that way

- **Function syntax**: `fn name(p1: T1, p2: T2) -> R { ... }`.
  Every parameter is typed. The return type comes after `->`.
  Functions returning *nothing* return `i64` and use `0` by
  convention (vāṇी v1 has no `()` unit type at the language
  surface; the `shout` helper above uses this convention).
- **Pass by value**. Primitives (`i64`, `f64`, `bool`, ...) and
  `Str` (a borrowed string) copy on call. To pass a `Vec<T>`
  or struct by reference, use `ref` / `mut ref` -- that's a
  later lesson.
- **`return` has dialect aliases**. The same statement is
  spelled four ways depending on the file's `// vani-lang:`
  pragma:

  | Pragma | Spelling |
  |---|---|
  | (none) or `english` | `return <expr>;` |
  | `sanskrit` | `<expr> पुनरागम;` (verb-at-end) |
  | `hindi` | `<expr> लौटाओ;` (verb-at-end) |
  | `marathi` | `<expr> परत;` (verb-at-end) |

  Inside a Devanagari-pragma file the verb-at-end form mirrors
  how those languages naturally place the verb. The lesson uses
  English-keyword form because that's the canonical surface.
- **Recursion just works**. There's no special `fn rec` syntax.
  See `bounded_score` in `examples/language/english/basics.vani`
  for a contract-style example, and Sec.9 for the SMT-friendly form
  with a `requires` clause.

## Challenge

Write a function `triangle_area(base: f64, height: f64) -> f64`
that computes `0.5 * base * height`, then call it from `main`
twice with different arguments and print both results.

<details>
<summary>Solution</summary>

```vani
fn triangle_area(base: f64, height: f64) -> f64 {
  return 0.5 * base * height;
}

fn main() -> i64 {
  print "triangle(3.0, 4.0) =", triangle_area(3.0, 4.0);
  print "triangle(7.0, 2.0) =", triangle_area(7.0, 2.0);
  return 0;
}
```

</details>

---

**Next**: [Sec.4 -- `if` / `else` ->](04_if_else.md)
