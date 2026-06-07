# Beginner 6 — Strings (`Str` vs `OwnedStr`)

> **Learning goal**: declare and compare `Str` literals, get
> their length, and understand the difference between `Str`
> (borrowed) and `OwnedStr` (heap-allocated).

## The program

Save this in `~/lesson6.vani`:

```rust
intent "Lesson 6 worked example — Str borrowed literals.";

fn role(who: Str) -> Str {
  if who == "admin" {
    return "owner";
  }
  if who == "guest" {
    return "visitor";
  }
  return "member";
}

fn greet(name: Str) -> i64 {
  print "hello,", name;
  return 0;
}

fn main() -> i64 {
  greet("alice");
  greet("bob");

  let r1: Str = role("alice");
  let r2: Str = role("admin");
  let r3: Str = role("guest");
  print "alice =", r1;
  print "admin =", r2;
  print "guest =", r3;

  let n: u64 = len("hello");
  print "len of \"hello\" =", n;

  let same: bool = "abc" == "abc";
  let diff: bool = "abc" != "abd";
  print "same =", same;
  print "diff =", diff;
  return 0;
}
```

## Compile + run

```bash
vanic run ~/lesson6.vani
```

Expected output:

```
hello, alice
hello, bob
alice = member
admin = owner
guest = visitor
len of "hello" = 5
same = true
diff = true
```

## Why it works that way

- **`Str` is borrowed**. It's the type of compile-time string
  literals (`"hello"`, `"alice"`, …). Under the hood it's a
  pointer to a NUL-terminated byte buffer in the program's
  `.rodata` section. You can pass `Str` values around freely;
  they copy a pointer, not the bytes.
- **`OwnedStr` is heap-allocated**. You get it from concatenation
  (`"foo" + bar` returns `OwnedStr`) and from a few stdlib
  helpers. v1 frees it automatically at scope exit (affine
  ownership). For this lesson we stick to `Str` — owned strings
  are a Intermediate-track topic.
- **`==` / `!=` use byte equality**. `"abc" == "abc"` is true.
  vāṇी uses `strcmp` under the hood — no surprises.
- **`len(s)` returns a `u64`**. Note the unsigned width:
  lengths are non-negative by definition, and the SMT verifier
  uses that to prove invariants in §9.
- **No string interpolation in v1**. Compose with `print "x =", x`
  which the runtime spaces out, or build an `OwnedStr` via `+`.
- **`<`, `<=`, `>`, `>=` on strings are NOT supported in v1**.
  Use `==` / `!=` only. Ordering comparisons are tracked as a
  follow-up; see `examples/language/english/string_ops.vani`
  for what's available today.

## Challenge

Write a function `is_yes(s: Str) -> bool` that returns `true`
for any of `"y"`, `"yes"`, `"Y"`, or `"YES"`, and `false`
otherwise. Test it on a handful of inputs in `main`.

<details>
<summary>Solution</summary>

```rust
fn is_yes(s: Str) -> bool {
  if s == "y" {
    return true;
  }
  if s == "yes" {
    return true;
  }
  if s == "Y" {
    return true;
  }
  if s == "YES" {
    return true;
  }
  return false;
}
```

A cleaner version using `match` will appear in §8.

</details>

---

**Next**: [§7 — Arrays and `Vec<T>` basics →](07_vec_arrays.md)
