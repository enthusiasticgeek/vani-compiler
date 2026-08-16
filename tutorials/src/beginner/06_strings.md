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

> **Async fn bodies**: this pattern also works correctly inside an
> `async fn`, for the common case — `let label: Str =
> i64_to_str(mode);` (or `f64_to_str` / `f64_to_str_fixed` /
> `bool_to_str`) is recognized directly by the async state-machine
> transform, which hoists the underlying `OwnedStr` into the
> generated coroutine's persistent state as a real owning field, not
> just a view. Less common `OwnedStr` sources the transform doesn't
> specifically recognize (e.g. building the string via `+`
> concatenation, or a user-defined function that returns `OwnedStr`)
> fall back to the same auto-borrow-only behavior as before this
> fix — safe, but the allocation leaks for the lifetime of that
> `Task`. Tracked in `docs/BUG_PATTERN_AUDIT_TODO_8.md`.

### Storing an `OwnedStr` into a struct's `Str` field is rejected

> **New syntax ahead**: this section uses `struct` — a named bundle
> of fields, like `Holder { s: Str, n: i64 }` below, which groups an
> `s` and an `n` value together under one type name so you can pass
> both around as a single unit (`h.s` reads the `s` field, `h.n` the
> `n` field). `struct` gets its own full chapter later
> ([Intermediate 1](../intermediate/01_struct_methods.md)); this is
> just enough to follow the example below, which is really about
> `OwnedStr`'s lifetime, not about `struct` itself.

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

### Passing a fresh `OwnedStr` into a container builtin — or an ordinary function

`hashmap_insert(mut ref m, k, v)`, `hashmap_get`/`hashmap_contains_key`/
`hashmap_remove`'s key argument, `Trie`'s `.insert(...)`/
`.contains(...)`/`.starts_with(...)`/`.delete(...)` methods, and **any
ordinary user-defined function taking a `Str` parameter** all accept a
**freshly-computed** `OwnedStr` argument correctly — passing an
already-owned binding just moves (or borrows) it as usual, and a
freshly-computed value (e.g. `i64_to_str(1)` called directly, not
through a `let`) is freed right after the call once nothing else needs
it:

```vani
fn takes_str(s: Str) -> i64 { return len(s) as i64; }

fn main() -> i64 {
  let m: HashMap<OwnedStr, OwnedStr> = hashmap_new();
  let _ = hashmap_insert(mut ref m, i64_to_str(1), i64_to_str(100));  // fine
  let r: Option<OwnedStr> = hashmap_get(ref m, i64_to_str(1));        // fine

  let t: Trie = trie_new();
  let _ = t.insert(i64_to_str(2));                                   // fine

  let n: i64 = takes_str(i64_to_str(12345));                         // fine
  print n;
  return 0;
}
```

> **Fixed 2026-08-14 (BUG-193)**: this same "fresh value, no owning
> binding" pattern used to leak (never crash — always safely, just an
> unreclaimed allocation) when passed directly as an argument to an
> **ordinary function** taking `Str` (`my_fn(i64_to_str(5))`). Confirmed
> fixed via a direct LeakSanitizer check on the exact repro above — no
> workaround needed anymore, though binding to a `let` first
> (`let k: OwnedStr = i64_to_str(5); my_fn(k);`) is still fine too, if
> you'd rather keep the value around afterward.

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
integers, this covers the same ground as ordinary function calls.
`print` also has direct, literal syntax for the common case --
covered next.

## Inline `print` format specs: `x:03` / `y:.2`

A `print` item can carry an inline width/precision spec, written
right after the value with no space: `print x:03;` pads `x` to width
3, zero-filled; `print y:.2;` prints `y` (an `f32`/`f64`) with
exactly 2 digits after the point; `print z:08.3;` combines both
(zero-padded to total width 8, 3 decimal digits). This is the same
idea as Rust's `{:03}` / `{:.2}` or C's `printf("%03d")` /
`printf("%.2f")`, just written postfix instead of inside a template
string:

```vani
fn main() -> i64 {
  print 5:03;              // "005"
  print 3.14159:.2;        // "3.14"
  print 3.14159:08.3;      // "0003.142"
  return 0;
}
```

Grammar: an optional `0` flag (zero-fill instead of the default
space-fill) immediately followed by optional width digits, then an
optional `.` and precision digits -- `'0'? WIDTH? ('.' PRECISION)?`.
At least one of width or precision must be present (a bare `:` with
nothing after it is just the ordinary colon token used everywhere
else in the language -- struct field types, type annotations,
labels -- there's no ambiguity since none of those put a digit or
`.` directly after `:` with no space).

**Rules**:

- **Width** works on any numeric type (`i8`..`i64`, `u8`..`u64`,
  `f32`, `f64`) -- it pads the printed representation, right-
  aligned, to at least that many characters.
- **Precision** (`.N`) is valid only on `f32`/`f64` -- it forces
  fixed-notation rounding to exactly `N` digits after the point,
  the same rounding `f64_to_str_fixed` uses. Precision on an integer
  is a compile-time error (`` `:.N` precision is only valid on
  f32/f64 print items ``), not a silent no-op.
- **Any spec at all on `bool`/`Str`/`OwnedStr`/a struct/an enum** is
  a compile-time error (`` format specs aren't supported on `<type>`
  print items ``) -- there's no implicit stringification to apply a
  width to.
- **Width/precision are compile-time literal digits only.** There's
  no `x:0{n}`-style runtime-substituted width -- every backend bakes
  a literal `printf`-style format string at the call site, never one
  assembled at runtime.
- **One narrow, documented no-op**: a signed integer's width spec
  has no effect when the file uses a non-ASCII `// vani-lang:`
  dialect that renders digits as localized codepoints (Devanagari,
  Bengali, Tamil, ...) -- those route through a dedicated per-script
  renderer, not `printf`, so there's no format-string slot for width
  to apply to. `eprint` is unaffected (it always renders integers as
  plain ASCII regardless of the file's dialect, so its format specs
  always apply). Reach for `str_pad_left(i64_to_str(n), width, "0")`
  if you need a padded localized-digit integer.

This is purely a print-site convenience over what `f64_to_str_fixed`
+ `str_pad_left` already let you build as an `OwnedStr` and print
normally -- reach for the function-call form instead when you need
the formatted text as a value (to concatenate, store, pass to
another call), not just to print it immediately.

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

### Byte-level access

For when you need a `Str`'s raw bytes rather than substring
matching -- parsing a wire format, hand-rolling a tokenizer, or just
inspecting one character's code point:

```vani
intent "byte-level string access";

fn main() -> i64 {
  let s: Str = "Hello, World! 123";

  print "byte_at(0):", str_byte_at(s, 0);                  // 72 ('H')
  print "len_bytes:", str_len_bytes(s);                    // 17
  print "starts_with_byte('H'=72):", str_starts_with_byte(s, 72);  // true
  print "ends_with_byte('3'=51):", str_ends_with_byte(s, 51);      // true
  print "byte_count('l'=108):", str_byte_count(s, 108);            // 3
  print "index_of_byte('W'=87):", option_unwrap_or(str_index_of_byte(s, 87), -1);        // 7
  print "last_index_of_byte('l'=108):", option_unwrap_or(str_last_index_of_byte(s, 108), -1);  // 10
  print "first_byte:", option_unwrap_or(str_first_byte(s), -1);    // 72
  print "last_byte:", option_unwrap_or(str_last_byte(s), -1);      // 51 ('3')
  return 0;
}
```

| Builtin | Signature | Description |
|---|---|---|
| `str_byte_at(s, i)` | `Str, i64 -> i64` | byte value at index `i` (no bounds check -- caller's responsibility) |
| `str_len_bytes(s)` | `Str -> i64` | byte length (same as `len(s)` for ASCII; differs for multi-byte UTF-8) |
| `str_starts_with_byte(s, b)` / `str_ends_with_byte(s, b)` | `Str, i64 -> bool` | does the first/last byte equal `b`? |
| `str_byte_count(s, b)` | `Str, i64 -> i64` | how many times byte `b` occurs |
| `str_index_of_byte(s, b)` / `str_last_index_of_byte(s, b)` | `Str, i64 -> Option<i64>` | index of the first/last occurrence of byte `b` |
| `str_first_byte(s)` / `str_last_byte(s)` | `Str -> Option<i64>` | first/last byte, or `None` if `s` is empty |

### ASCII-class counting and classification

```vani
intent "ASCII-class counting";

fn main() -> i64 {
  let s: Str = "Hello, World! 123";

  print "digits:", str_count_ascii_digits(s);         // 3
  print "alpha:", str_count_ascii_alpha(s);            // 10
  print "alphanumeric:", str_count_ascii_alphanumeric(s);  // 13
  print "whitespace:", str_count_ascii_whitespace(s);  // 2
  print "upper:", str_count_ascii_upper(s);            // 2
  print "lower:", str_count_ascii_lower(s);            // 8
  print "punct:", str_count_ascii_punct(s);            // 2 (',' and '!')
  print "control:", str_count_ascii_control(s);        // 0

  print "is_ascii:", str_is_ascii(s);                       // true
  print "is_digit_only('123'):", str_is_digit_only("123");  // true
  print "is_alpha_only('abc'):", str_is_alpha_only("abc");  // true
  print "is_empty(''):", str_is_empty("");                  // true
  return 0;
}
```

| Builtin | Signature | Description |
|---|---|---|
| `str_count_ascii_digits/alpha/alphanumeric/whitespace/upper/lower/punct/control(s)` | `Str -> i64` | count of ASCII bytes in that class |
| `str_is_ascii(s)` | `Str -> bool` | every byte is in the 0-127 ASCII range |
| `str_is_digit_only(s)` / `str_is_alpha_only(s)` / `str_is_alphanumeric_only(s)` / `str_is_whitespace_only(s)` | `Str -> bool` | every character belongs to that class (empty string is vacuously true) |
| `str_is_empty(s)` | `Str -> bool` | `len(s) == 0`, spelled out as a builtin |

**Single-byte classifiers** (`is_ascii_digit` etc., no `str_` prefix)
take a raw byte CODE (an `i64`, from e.g. `str_byte_at`), not a
`Str` -- easy to confuse with the `str_is_*_only` family above,
which classify a whole string:

```vani
intent "single-byte classifiers";

fn main() -> i64 {
  print "is_ascii_digit('5'):", is_ascii_digit(53);            // true -- 53 is '5'
  print "is_ascii_alpha('a'):", is_ascii_alpha(97);             // true -- 97 is 'a'
  print "is_ascii_alphanumeric('5'):", is_ascii_alphanumeric(53);  // true
  print "is_ascii_whitespace(' '):", is_ascii_whitespace(32);   // true -- 32 is ' '
  return 0;
}
```

| Builtin | Signature | Description |
|---|---|---|
| `is_ascii_digit(byte)` / `is_ascii_alpha(byte)` / `is_ascii_alphanumeric(byte)` / `is_ascii_whitespace(byte)` | `i64 -> bool` | classify ONE byte code, not a `Str` |

### Characters, stripping, and `parse_bool`

> **New syntax ahead**: `str_chars` below returns a `Vec<i64>` — a
> growable list type with its own full chapter later
> ([Beginner 7](07_vec_arrays.md)). All you need here: `Vec<i64>` is
> a numbered sequence of `i64` values, `ch[0]` reads the first one,
> and `len(ref ch)` counts how many there are. The `match` at the
> bottom is previewed the same way — full treatment in
> [Beginner 8](08_match.md) and its
> [Option primer](08b_errors_primer.md); here it just picks `v` out
> of `Option.Some(v)`, or falls back to `false` for `Option.None`.

```vani
intent "chars, strip, parse_bool";

fn main() -> i64 {
  // str_chars: a Str exploded into one i64 byte code per element.
  let ch: Vec<i64> = str_chars("abc");
  print "chars len:", len(ref ch) as i64;   // 3
  print "chars[0]:", ch[0];                  // 97 ('a')

  print "strip_prefix:", str_strip_prefix("hello world", "hello ");  // "world"
  print "strip_suffix:", str_strip_suffix("hello world", " world");  // "hello"
  print "count_char('l' in 'hello'):", str_count_char("hello", "l"); // 2

  // parse_bool returns Option<bool> -- unwrap with match (option_unwrap_or
  // is i64/f64-specific, so it doesn't apply to Option<bool>).
  let pb: Option<bool> = parse_bool("true");
  let ok: bool = match pb {
    Option.Some(v) then v,
    Option.None then false,
  };
  print "parse_bool('true'):", ok;   // true
  return 0;
}
```

| Builtin | Signature | Description |
|---|---|---|
| `str_chars(s)` | `Str -> Vec<i64>` | one byte code per character (ASCII; not full Unicode codepoint decoding) |
| `str_strip_prefix(s, prefix)` / `str_strip_suffix(s, suffix)` | `Str, Str -> OwnedStr` | remove the prefix/suffix if present; return `s` unchanged if it doesn't match |
| `str_count_char(s, needle)` | `Str, Str -> i64` | count of occurrences of `needle` (itself a `Str`, not a byte code) |
| `parse_bool(s)` | `Str -> Option<bool>` | parse `"true"`/`"false"`; `None` for anything else |

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
