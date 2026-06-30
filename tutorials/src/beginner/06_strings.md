# Beginner 6 -- Strings (`Str` vs `OwnedStr`)

> **Learning goal**: declare and compare `Str` literals, get
> their length, and understand the difference between `Str`
> (borrowed) and `OwnedStr` (heap-allocated).

> **New to this?** Read [Beginner 6a -- Pointers and references primer](06a_pointers_refs_primer.md)
> for the address/value analogy first.

Think of a `Str` like a sticky note with directions to a book
on a library shelf: it POINTS at some text that lives elsewhere
in the program (usually hardcoded in the compiled binary), but
it doesn't OWN that text. An `OwnedStr` is like buying your
own copy of the book -- the heap memory is yours, you can
modify it, and when you're done it gets freed. Most programs
only need to READ string literals (`Str` is enough); you need
`OwnedStr` when you CONSTRUCT strings at runtime by combining
or modifying parts.

## The program

Save this in `~/lesson6.vani`:

```vani
intent "Lesson 6 worked example -- Str borrowed literals.";

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
  literals (`"hello"`, `"alice"`, ...). Under the hood it's a
  pointer to a NUL-terminated byte buffer in the program's
  `.rodata` section. You can pass `Str` values around freely;
  they copy a pointer, not the bytes.
- **`OwnedStr` is heap-allocated**. You get it from concatenation
  (`"foo" + bar` returns `OwnedStr`) and from a few stdlib
  helpers. v1 frees it automatically at scope exit (affine
  ownership). For this lesson we stick to `Str` -- owned strings
  are a Intermediate-track topic.
- **`==` / `!=` use byte equality**. `"abc" == "abc"` is true.
  vāṇी uses `strcmp` under the hood -- no surprises.
- **`len(s)` returns a `u64`**. Note the unsigned width:
  lengths are non-negative by definition, and the SMT verifier
  uses that to prove invariants in Sec.9.
- **No string interpolation in v1**. Compose with `print "x =", x`
  which the runtime spaces out, or build an `OwnedStr` via `+`.
- **`<`, `<=`, `>`, `>=` on strings are NOT supported in v1**.
  Use `==` / `!=` only. Ordering comparisons are tracked as a
  follow-up; see `examples/language/english/string_ops.vani`
  for what's available today.

## String builtins reference

vāṇī ships a rich set of string builtins. These all accept `Str`
arguments and return the type shown.

| Builtin | Signature | Returns |
|---|---|---|
| `str_contains(s, sub)` | `Str, Str -> bool` | substring test |
| `str_starts_with(s, pre)` | `Str, Str -> bool` | prefix test |
| `str_ends_with(s, suf)` | `Str, Str -> bool` | suffix test |
| `str_to_upper(s)` | `Str -> OwnedStr` | uppercase copy |
| `str_to_lower(s)` | `Str -> OwnedStr` | lowercase copy |
| `str_trim(s)` | `Str -> OwnedStr` | strip leading/trailing whitespace |
| `str_replace(s, from, to)` | `Str, Str, Str -> OwnedStr` | replace all occurrences |
| `str_split(s, sep)` | `Str, Str -> Vec<OwnedStr>` | split on separator |
| `str_join(v, sep)` | `Vec<OwnedStr>, Str -> OwnedStr` | join with separator |
| `str_index_of(s, sub)` | `Str, Str -> i64` | index of first occurrence, -1 if absent |
| `substring(s, start, len)` | `Str, i64, i64 -> OwnedStr` | extract slice |
| `str_repeat(s, n)` | `Str, i64 -> OwnedStr` | repeat N times |
| `str_pad_left(s, n, c)` | `Str, i64, Str -> OwnedStr` | left-pad to width N |
| `str_pad_right(s, n, c)` | `Str, i64, Str -> OwnedStr` | right-pad to width N |
| `str_reverse(s)` | `Str -> OwnedStr` | reverse the characters |
| `str_lines(s)` | `Str -> Vec<OwnedStr>` | split on newlines |
| `parse_int(s)` | `Str -> Option<i64>` | parse decimal integer |
| `i64_to_str(n)` | `i64 -> OwnedStr` | integer to string |
| `bool_to_str(b)` | `bool -> OwnedStr` | `"true"` or `"false"` |

Quick example:

```vani
intent "Lesson 6 -- string builtins sampler.";

fn main() -> i64 {
  let s: Str = "  Hello, World!  ";

  print "trimmed:", str_trim(s);
  print "upper:", str_to_upper("hello");
  print "lower:", str_to_lower("WORLD");
  print "contains 'World':", str_contains(s, "World");
  print "starts with spaces:", str_starts_with(s, "  ");
  print "replace:", str_replace("foo bar foo", "foo", "baz");
  print "repeated:", str_repeat("ab", 3);

  let parsed: Option<i64> = parse_int("42");
  print "parsed 42:", option_unwrap_or(parsed, -1);

  return 0;
}
```

Expected output:

```
trimmed: Hello, World!
upper: HELLO
lower: world
contains 'World': true
starts with spaces: true
replace: baz bar baz
repeated: ababab
parsed 42: 42
```

## Challenge

Write a function `is_yes(s: Str) -> bool` that returns `true`
for any of `"y"`, `"yes"`, `"Y"`, or `"YES"`, and `false`
otherwise. Test it on a handful of inputs in `main`.

<details>
<summary>Solution</summary>

```vani
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

A cleaner version using `match` will appear in Sec.8.

</details>

---

**Next**: [Sec.7 -- Arrays and `Vec<T>` basics ->](07_vec_arrays.md)
