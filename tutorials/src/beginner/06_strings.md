# Beginner 6 -- Strings (`Str` vs `OwnedStr`)

> **Learning goal**: use `Str` for compile-time string literals,
> construct and concatenate `OwnedStr` values at runtime, and
> understand the ownership difference between the two types.

> **New to this?** Read [Beginner 6a -- Pointers and references primer](06a_pointers_refs_primer.md)
> for the address/value analogy first, then
> [Beginner 6c -- Ownership and move primer](06c_ownership_primer.md)
> to understand why heap-owning types like `OwnedStr` follow move semantics.

Think of a `Str` like a sticky note with directions to a book
on a library shelf: it POINTS at text that lives in the compiled
binary's read-only section (`.rodata`). It doesn't own the
bytes, can't modify them, and the pointer costs only 8 bytes.
An `OwnedStr` is like buying your own copy of the book: the
bytes live on the heap, you own them (and free them at scope
exit), and you can build them dynamically at runtime.

Most programs start with `Str` literals; they graduate to
`OwnedStr` when they need to **build** strings at runtime by
concatenating, trimming, converting numbers, or composing
parts.

---

## Part 1 — `Str`: borrowed string literals

### The program

Save this in `~/lesson6a.vani`:

```vani
intent "Lesson 6a -- Str borrowed literals.";

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

```bash
vanic run ~/lesson6a.vani
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

### How `Str` works

- **`Str` is a pointer to a NUL-terminated byte buffer** in the
  program's `.rodata` section. Passing `Str` copies the pointer
  (8 bytes), not the underlying bytes.
- **`==` / `!=` use byte equality** via `strcmp`. No surprises.
- **`<`, `<=`, `>`, `>=` also work** via lexicographic strcmp.
  `"apple" < "banana"` is `true`.
- **`len(s)` returns `u64`** (non-negative by definition).

---

## Part 2 — `OwnedStr`: heap-allocated strings you build at runtime

You need `OwnedStr` whenever you construct a string at runtime:
concatenating two pieces, converting a number, trimming
whitespace, or any other operation that returns new bytes.

### Concatenation: `+` returns `OwnedStr`

Adding two strings (or any string-like values) with `+` always
returns an `OwnedStr`:

```vani
intent "Lesson 6b -- OwnedStr concatenation.";

fn full_name(first: Str, last: Str) -> OwnedStr {
  return first + " " + last;
}

fn numbered_item(label: Str, n: i64) -> OwnedStr {
  let n_str: OwnedStr = i64_to_str(n);
  return label + n_str;
}

fn main() -> i64 {
  let name: OwnedStr = full_name("Alice", "Smith");
  print name;

  let item: OwnedStr = numbered_item("item-", 42);
  print item;

  // Concatenating Str + Str -> OwnedStr
  let a: Str = "foo";
  let b: Str = "bar";
  let ab: OwnedStr = a + b;
  print ab;

  return 0;
}
```

```bash
vanic run ~/lesson6b.vani
```

Expected output:

```
Alice Smith
item-42
foobar
```

All combinations work: `Str + Str`, `Str + OwnedStr`,
`OwnedStr + Str`, and `OwnedStr + OwnedStr` all return a new
`OwnedStr`. The left operand's bytes plus the right operand's
bytes are concatenated into a fresh heap allocation.

### Converting numbers to strings

```vani
let n: i64 = 42;
let s: OwnedStr = i64_to_str(n);       // "42"

let pi: f64 = 3.14;
let ps: OwnedStr = f64_to_str(pi);     // "3.14" (or similar)

let b: bool = true;
let bs: OwnedStr = bool_to_str(b);     // "true"
```

### `OwnedStr` auto-coerces to `Str` in read-only positions

Wherever a function expects a `Str` argument, you can pass an
`OwnedStr`. The compiler auto-borrows it (no allocation, no copy):

```vani
fn greet(name: Str) -> i64 {
  print "hello,", name;
  return 0;
}

fn main() -> i64 {
  let owned: OwnedStr = "Alice" + " Smith";
  greet(owned);     // OwnedStr used where Str expected -- fine
  print len(owned); // len() also accepts OwnedStr
  return 0;
}
```

The `owned` binding stays alive; only a read-only pointer is
passed. `owned` is freed at end of `main`.

The same auto-coercion also works when you narrow a **fresh**
`OwnedStr`-producing call straight into a `Str`-typed `let`, with
no intermediate `OwnedStr` binding of your own:

```vani
fn main() -> i64 {
  let label: Str = i64_to_str(42);   // fresh OwnedStr narrowed to Str
  print label;
  return 0;
}
```

The compiler transparently keeps an `OwnedStr` behind the scenes
to own the allocation and free it at scope exit — `label` itself
is just a read-only view into it, same as the function-argument
case above. You don't need to write the two-step form
(`let tmp: OwnedStr = i64_to_str(42); let label: Str = tmp;`)
yourself; either form works identically.

> **Caveat (async fn bodies only)**: inside an `async fn`, this
> exact narrowing pattern (`let label: Str = <fresh OwnedStr
> call>;`) currently leaks the allocation instead of freeing it.
> The async state-machine transform hoists locals it recognizes
> into persistent per-coroutine state before the checker ever sees
> the body, and its handling of an `OwnedStr`-owns/`Str`-views-into
> relationship across a suspend point isn't sound yet — the
> compiler-managed temp described above isn't safe to introduce
> there. Splitting it into two `let`s yourself
> (`let tmp: OwnedStr = i64_to_str(mode); let label: Str = tmp;`)
> is **not** a safe workaround either — it still compiles, but
> trades the leak for a heap-use-after-free instead, which is
> worse. There is currently no source-level workaround that avoids
> both; the fix has to be in the async transform itself. Tracked in
> `docs/BUG_PATTERN_AUDIT_TODO_8.md`.

### Storing an `OwnedStr` into a struct's `Str` field is rejected

The `let`-narrowing case above is safe because the compiler-managed
temp's scope is tied directly to the `let` itself. That safety
doesn't carry over to a struct field — a struct can easily outlive
whatever `OwnedStr` you tried to view into it:

```vani
struct Holder { s: Str, n: i64 }
fn main() -> i64 {
  let h: Holder = Holder { s: "", n: 0 };
  {
    let owned: OwnedStr = i64_to_str(99);
    h.s = owned;   // rejected -- see below
  }
  print h.s;
  return 0;
}
```
```
error: cannot store a freshly-owned `OwnedStr` into `Str`-typed field 's' -- the struct
can outlive the `OwnedStr`'s own scope, which would free the buffer while the field is
still readable (a use-after-free)
    h.s = owned;
          ^^^^^
```

The same rejection applies to initializing the field directly in a
struct literal (`Holder { s: owned, .. }`) and to writing through a
`Vec` element's field (`xs[i].s = owned;`). **Fix**: declare the
field as `OwnedStr` instead of `Str` — then it owns its own copy, and
the struct's own Drop frees it correctly:

```vani
struct Holder { s: OwnedStr, n: i64 }
fn make() -> Holder {
  let owned: OwnedStr = i64_to_str(77);
  return Holder { s: owned, n: 1 };   // fine -- s owns the allocation
}
fn main() -> i64 {
  let h: Holder = make();
  print h.s;
  return 0;
}
```

### Copying a `Str` literal into an `OwnedStr`

To get an owned copy of a literal, concatenate with an empty
string:

```vani
let literal: Str    = "hello";
let owned:   OwnedStr = literal + "";   // copies bytes to heap
```

This is the idiomatic conversion when you need to store a
`Str` literal in a structure that requires `OwnedStr`.

---

## Part 3 — What the compiler catches

### Assigning a `Str` literal to an `OwnedStr` variable

`Str` does NOT automatically become `OwnedStr`. The directions
(pointer) don't become the book (heap copy) without work.

<img class="manas" src="../images/mascot/manas_mascot_error.png" title="this code does not compile!"/>

```vani
// WRONG
let s: OwnedStr = "hello";
```

```
error: let initializer must be assignable to OwnedStr, got Str
  let s: OwnedStr = "hello";
                    ^^^^^^^
  help: use `s + ""` to copy a Str into an OwnedStr
```

**Fix**: concatenate with `""` to heap-copy: `let s: OwnedStr = "hello" + "";`

<img class="manas" src="../images/mascot/manas_mascot_success.png" title="this is the correct, working version"/>

```vani
let s: OwnedStr = "hello" + "";
print s;
```

### Passing `Str` where `OwnedStr` is expected

`OwnedStr` auto-coerces **down** to `Str` (borrowed read). The
reverse — `Str` to `OwnedStr` — requires an explicit copy:

<img class="manas" src="../images/mascot/manas_mascot_error.png" title="this code does not compile!"/>

```vani
fn needs_owned(s: OwnedStr) -> i64 { print s; return 0; }

fn main() -> i64 {
  let s: Str = "hello";
  needs_owned(s);    // error: Str does not auto-coerce to OwnedStr
  return 0;
}
```

```
error: argument 1 to 'needs_owned' must be assignable to OwnedStr, got Str
  needs_owned(s);
              ^
  help: use `s + ""` to copy a Str into an OwnedStr
```

**Fix**: pass `s + ""` to heap-copy the literal first.

<img class="manas" src="../images/mascot/manas_mascot_success.png" title="this is the correct, working version"/>

```vani
fn needs_owned(s: OwnedStr) -> i64 { print s; return 0; }

fn main() -> i64 {
  let s: Str = "hello";
  needs_owned(s + "");
  return 0;
}
```

---

## String builtins reference

vāṇी ships a rich set of string builtins.

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
| `str_join(v, sep)` | `ref Vec<OwnedStr>, Str -> OwnedStr` | join with separator |
| `str_index_of(s, sub)` | `Str, Str -> Option<i64>` | index of first occurrence, `Option.None` if absent |
| `substring(s, start, len)` | `Str, i64, i64 -> OwnedStr` | extract slice |
| `str_repeat(s, n)` | `Str, i64 -> OwnedStr` | repeat N times |
| `str_pad_left(s, n, c)` | `Str, i64, Str -> OwnedStr` | left-pad to width N |
| `str_pad_right(s, n, c)` | `Str, i64, Str -> OwnedStr` | right-pad to width N |
| `str_reverse(s)` | `Str -> OwnedStr` | reverse the characters |
| `str_lines(s)` | `Str -> Vec<OwnedStr>` | split on newlines |
| `parse_int(s)` | `Str -> Option<i64>` | parse decimal integer |
| `i64_to_str(n)` | `i64 -> OwnedStr` | integer to string |
| `f64_to_str(n)` | `f64 -> OwnedStr` | float to string, compact (`%g`) -- trailing zeros stripped |
| `f64_to_str_fixed(n, decimals)` | `f64, i64 -> OwnedStr` | float to string with exactly `decimals` digits after the point, zero-padded -- the Rust `{:.N}` / C `printf("%.*f", ...)` equivalent |
| `bool_to_str(b)` | `bool -> OwnedStr` | `"true"` or `"false"` |

All builtins that return `OwnedStr` allocate a new heap buffer.
All builtins accept either `Str` or `OwnedStr` as input (the
`OwnedStr` auto-coercion handles it).

`f64_to_str` vs. `f64_to_str_fixed`: `f64_to_str(3.1)` gives
`"3.1"` (compact, no trailing zeros); `f64_to_str_fixed(3.1, 2)`
gives `"3.10"` (always exactly 2 digits after the point, even if
that means padding with zeros). Reach for `f64_to_str_fixed`
whenever the output needs a *fixed* number of decimal places --
currency, fixed-width tables, anything a human will compare
column-by-column. Negative `decimals` is clamped to 0.
Combined with `str_pad_left(i64_to_str(n), width, "0")` for
integers, this covers the same ground as Rust's `{:.N}` / `{:0N}`
format specifiers -- as ordinary function calls rather than a
`{}` mini-language, since `print` has no format-string syntax of
its own (a comma-separated list of items, each printed as-is).

<img class="manas" src="../images/mascot/manas_mascot_caution.png" title="this code needs extra care"/>

**Caveats for `print`ing an `f64` directly (and for `f64_to_str`)**:
raw `print x;` on an `f64` goes through the exact same `%g`-based
formatting as `f64_to_str` -- there's no separate code path, so
these caveats apply equally to plain `print` statements, not just
explicit `f64_to_str` calls.

- **6 significant digits by default, with real precision loss.**
  `%g`'s default precision is 6 significant digits, not "however
  many digits round-trip." `print 123456789.123456;` prints
  `1.23457e+08` -- everything past the 6th significant digit is
  gone, not just hidden from display. If you need the value back
  losslessly, keep the `f64` around and only format for display;
  don't parse the printed string back with `parse_float`.
  (Confirmed directly on both backends against a current build --
  a bare float literal passed straight to `print`, with no
  intermediate `let`, used to crash the LLVM backend outright and
  print the wrong value on the C backend, BUG-123, fixed
  2026-08-06.)
- **Verified backend-parity gap in scientific-notation exponent
  width on Windows.** Once a magnitude is large/small enough that
  `%g` switches to scientific notation, the C backend and the LLVM
  backend disagree on Windows: `vanic run f.vani --backend=c`
  prints `1e+06`, while `vanic run f.vani` (LLVM/JIT) prints
  `1e+006` for the identical program and value -- a 2-digit vs.
  3-digit exponent, respectively (legacy MSVCRT vs. UCRT
  `snprintf` conventions; the C backend links the host's `cc`,
  the LLVM backend's `vsnprintf` shim resolves differently). Two
  backends of the same compiler producing different text for the
  same `print` statement is a real gap, not a rounding nuance --
  don't compare `print` output across backends in golden-file
  tests on Windows if the value might hit scientific notation.
  Tracked as L25 in [`docs/v1_limitations.md`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/docs/v1_limitations.md).
- **The portable, deterministic alternative is `f64_to_str_fixed`.**
  It formats with `%.*f` (fixed notation), which never switches to
  scientific notation and so never hits the exponent-width gap
  above. If you need output that's identical across backends and
  platforms -- test assertions, log lines diffed in CI, anything
  compared byte-for-byte -- prefer `print f64_to_str_fixed(x, n);`
  over `print x;` for any `f64` whose magnitude you don't tightly
  control.

**Caveats for `f64_to_str_fixed`**:

- **NaN / Infinity spelling is platform-dependent.** Like
  `f64_to_str`, it just calls the local C library's `snprintf`,
  so it inherits whatever that library prints for non-finite
  values. On a Windows build linked against the legacy MSVCRT
  runtime, `f64_to_str_fixed(f64_nan(), 2)` prints `"1.#R"` and
  `f64_to_str_fixed(f64_inf(), 2)` prints `"1.#J"` -- not the
  C99 `"nan"` / `"inf"` you'd get on Linux/macOS (glibc) or a
  UCRT-linked Windows build. Don't pattern-match on a specific
  spelling; check `f64_is_nan(x)` / `f64_is_finite(x)` first if
  you need to handle these cases.
- **Rounding at exact halfway points follows the local C
  library's `printf("%.*f", ...)`, which is round-to-even, not
  round-away-from-zero.** `f64_to_str_fixed(0.125, 2)` gives
  `"0.12"` (0.125 is exactly representable in `f64`, and 2 --
  the nearer even digit -- is what round-half-to-even picks),
  not `"0.13"`. Not guaranteed bit-identical to Rust's `{:.2}`
  at every halfway case, since Rust's float formatter doesn't
  go through the C library at all -- if a halfway-rounding
  result matters for a test assertion, verify it against the
  real compiler rather than assuming either convention.
- **Decimal separator is always `.`**, regardless of OS locale
  (no locale-aware formatting, matching every other vāṇी
  numeric builtin).

Quick example:

```vani
intent "Lesson 6 -- string builtins sampler.";

fn main() -> i64 {
  let s: Str = "  Hello, World!  ";

  // Builtins that return OwnedStr
  let trimmed: OwnedStr = str_trim(s);
  let upper:   OwnedStr = str_to_upper("hello");
  let lower:   OwnedStr = str_to_lower("WORLD");
  let rpt:     OwnedStr = str_repeat("ab", 3);

  print "trimmed:", trimmed;
  print "upper:", upper;
  print "lower:", lower;
  print "contains 'World':", str_contains(s, "World");
  print "starts with spaces:", str_starts_with(s, "  ");
  print "replace:", str_replace("foo bar foo", "foo", "baz");
  print "repeated:", rpt;

  // OwnedStr auto-coerces where Str expected
  print "len of upper:", len(upper);
  print "trimmed contains 'World':", str_contains(trimmed, "World");

  // Lexicographic ordering: < / <= / > / >= work on both Str and OwnedStr.
  // A `print` item can't be a bare comparison expression -- compute it
  // into a `let` first, then print the bool.
  let apple_lt_banana: bool = "apple" < "banana";
  print "apple < banana:", apple_lt_banana;
  let upper_gt_lower: bool = upper > lower;
  print "upper > lower:", upper_gt_lower;   // false: uppercase < lowercase in ASCII

  let parsed: Option<i64> = parse_int("42");
  print "parsed 42:", option_unwrap_or(parsed, -1);

  // f64_to_str_fixed: always exactly `decimals` digits, zero-padded --
  // portable/deterministic across backends, unlike raw `print` on an f64.
  print "fixed 2dp:", f64_to_str_fixed(3.1, 2);

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
len of upper: 5
trimmed contains 'World': true
apple < banana: true
upper > lower: false
parsed 42: 42
fixed 2dp: 3.10
```

---

## Summary: `Str` vs `OwnedStr`

| | `Str` | `OwnedStr` |
|---|---|---|
| Storage | Pointer into `.rodata` (8 bytes) | Heap-allocated byte buffer |
| Created by | String literals `"..."` | Concatenation `+`, builtins, conversions |
| Ownership | Borrowed (no destructor) | Owned (freed at scope exit) |
| Modify | No | Yes (via builtins) |
| Pass to `Str` param | Directly | Auto-coerces (no copy) |
| Pass to `OwnedStr` param | `s + ""` (explicit heap copy) | Directly |
| `==` / `!=` | ✓ (byte equality via strcmp) | ✓ |
| `<` / `<=` / `>` / `>=` | ✓ (lexicographic via strcmp) | ✓ |

---

## Challenge

Write a function `is_yes(s: Str) -> bool` that returns `true`
for any of `"y"`, `"yes"`, `"Y"`, or `"YES"`, and `false`
otherwise. Test it on a handful of inputs in `main`.

<details>
<summary>Solution</summary>

```vani
fn is_yes(s: Str) -> bool {
  if s == "y" { return true; }
  if s == "yes" { return true; }
  if s == "Y" { return true; }
  if s == "YES" { return true; }
  return false;
}
```

A cleaner version using `match` will appear in Sec.8.

</details>

---

**Previous**: [Sec.6c -- Ownership and move ->](06c_ownership_primer.md)
**Next**: [Sec.6d -- Program memory layout primer ->](06d_memory_sections_primer.md)
