# Beginner 1b — Block Comments `/* ... */`

> **Learning goal**: use multi-line block comments to annotate any part
> of a vāṇी program, including nested comments and inline annotations.

---

## The two comment styles

vāṇी supports two comment forms:

| Style | Syntax | Use case |
|-------|--------|----------|
| Line comment | `// text to end of line` | Short notes, disabling a line |
| Block comment | `/* text */` | Multi-line notes, inline annotation |

---

## Block comment basics

A block comment opens with `/*` and closes with `*/`. Everything between
is ignored by the compiler — it can span any number of lines:

```vani
/* This function computes the nth Fibonacci number.
   It uses iteration, not recursion, so it runs in
   O(n) time with O(1) space. */
fn fib(n: i64) -> i64 {
  let a: i64 = 0;
  let b: i64 = 1;
  let i: i64 = 0;
  while i < n {
    let tmp: i64 = a + b;
    a = b;
    b = tmp;
    i = i + 1;
  }
  return a;
}
```

---

## Inline block comments

Because `/* */` has a definite end, you can drop one anywhere inside an
expression or statement — even between a type annotation and its value:

```vani
let x: i64 = /* result of 6 * 7 */ 42;
let limit: i64 = /* exclusive upper bound */ 100;
```

The compiler strips the comment and sees `let x: i64 = 42;` as normal.

---

## Nested block comments

vāṇी block comments nest to any depth. The `*/` closes the *innermost*
open `/*`, so you can comment out a block that already contains a comment:

```vani
/* outer comment
   /* inner comment — still inside outer */
   back in outer
*/
```

This is useful when you want to disable a section of code that itself
has block comments in it — something line comments can't do safely.

```vani
/* temporarily disabled
fn old_approach(n: i64) -> i64 {
  /* old algorithm — O(n^2) */
  let result: i64 = 0;
  /* ... */
  return result;
}
*/
fn main() -> i64 {
  return 0;
}
```

Nesting can go as deep as needed: `/* /* /* deep */ */ */` is valid.

---

## Empty block comment

`/**/` (open immediately closed) is a valid, zero-character comment:

```vani
let y: i64 = /**/ 0;   /* same as: let y: i64 = 0; */
```

---

## Unterminated comment → compile error

Forgetting the closing `*/` is caught cleanly at compile time:

```vani
/* this comment is never closed
fn main() -> i64 { return 0; }
```

```
error: unterminated block comment
```

No crash, no silent truncation — the compiler rejects the file with a
precise diagnostic.

---

## Quick reference

```vani
// single-line comment

/* single-line block comment */

/* multi-line
   block comment */

let x: i64 = /* inline */ 42;

/* outer /* nested */ still outer */

/**/  /* empty comment */
```

---

**Next**: [§2 — Variables, types, and operators →](02_variables.md)
