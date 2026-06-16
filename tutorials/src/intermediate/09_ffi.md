# Intermediate 9 — FFI: `extern "C"` + `--link-with`

> **Learning goal**: call a C-ABI function from vāṇी, declare
> a foreign function as `pure` so pure callers can use it, and
> link external code at build time via `--link-with` / `-l<name>`.

> **New to this?** Read [Intermediate 9a — FFI primer](09a_ffi_primer.md) first.

Think of a hardware store that sells pre-made door hinges. Your
house is written in vāṇी; the hinges were made in a C factory.
You don't need to understand how the factory works — you just
need to know the hinge's interface (its type signature) and where
to get it (the library). `extern "C"` is the vāṇी way of
describing that interface; `--link-with` tells the compiler where
the factory's finished goods are stored.

## The program

```vani
intent "Intermediate 9 worked example — FFI to libc.";

// `extern "C" fn` declares a foreign function. The linker
// resolves it at build time; the compiler treats it as
// conservatively impure.
extern "C" fn atoi(x: Str) -> i32;

// `pure extern "C" fn` opts the foreign symbol in as side-
// effect-free + deterministic, so `pure fn` bodies can call
// it. The compiler can't verify purity across the FFI
// boundary — you're asserting it.
pure extern "C" fn atoll(x: Str) -> i64;

// A pure fn can compose pure externs.
pure fn parse_sum(a: Str, b: Str) -> i64 {
  return atoll(a) + atoll(b);
}

fn main() -> i64 {
  let n: i32 = atoi("42");
  let s: i64 = parse_sum("3", "4");
  print "atoi(\"42\") =", n;
  print "parse_sum(\"3\", \"4\") =", s;
  return 0;
}
```

## Compile + run

```bash
vanic run ~/int9.vani               # libc is linked by default
```

Output:

```
atoi("42") = 42
parse_sum("3", "4") = 7
```

## Why it works that way

- **`extern "C" fn name(params) -> R;`** is a body-less
  declaration. The compiler emits a forward declaration
  (`extern` in C, `declare` in LLVM IR) against the bare C-ABI
  symbol and lets the linker bind it.
- **Purity is opt-in** at the FFI boundary. By default the
  compiler treats every `extern "C" fn` as impure (the SMT
  engine can't reason across the FFI boundary). Adding `pure`
  in front asserts that the foreign function is side-effect-
  free and deterministic — your responsibility to verify.
- **ABI scope in v1**: scalars (`i8..i64`, `u8..u64`, `f32` /
  `f64`, `bool`), `Str` (NUL-terminated `i8*`), and any
  `ref T` / `mut ref T`. **Aggregate-by-value** (struct /
  tuple / array / enum passed by value) is rejected with a
  `ref T` migration hint to prevent silent ABI corruption.

## Linking external code

vāṇी shells out to your system `cc` for the C backend and
`lli` for the LLVM JIT. Two flags forward to the linker:

```bash
# Compile a C helper alongside the vāṇी source
vanic build foo.vani -o foo --link-with helper.c

# Link a library with -l<name>
vanic build foo.vani -o foo -lm -lcurl
```

`--link-with` accepts `.c`, `.o`, or `.a` paths.

## Common gotchas

- **No null pointer in `Str`**. `Str` in vāṇी always points to
  a valid NUL-terminated buffer. A C function that may return
  NULL needs a wrapper that converts NULL into a sentinel
  string or an `Option`.
- **Errno isn't first-class**. If your C function sets
  `errno`, write a wrapper that captures it and returns a
  `Result<T, i32>`.
- **`pure` is a load-bearing assertion**. Lying to the compiler
  (declaring an impure function as `pure`) can cause the SMT
  pass to elide calls or invariants in surprising ways.
  Verify your declarations against the man page.

## Challenge

Declare an `extern "C" fn strlen(s: Str) -> u64;` and write a
`fn longest_of(a: Str, b: Str) -> Str` that returns whichever
input is longer.

<details>
<summary>Solution</summary>

```vani
extern "C" fn strlen(s: Str) -> u64;

fn longest_of(a: Str, b: Str) -> Str {
  if strlen(a) >= strlen(b) {
    return a;
  }
  return b;
}

fn main() -> i64 {
  print longest_of("hi", "hello");
  return 0;
}
```

</details>

---

**Next**: [§10 — Error handling: `Result<T, E>` + `try` →](10_result_try.md)
