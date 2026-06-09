# Intermediate 9a — FFI: calling C code from vāṇी (intuition primer)

> **Learning goal**: build a mental model of "FFI" — Foreign
> Function Interface. The way one language calls into another.
> Why this exists, what could go wrong, and what vāṇी does to
> make it tractable. Reading order:
> [Intermediate 9 — FFI: extern "C" + --link-with](09_ffi.md)
> follows this; read this primer first if you've never used
> a foreign-function call before.

This chapter has **no compiler code**. Pure intuition.

## The problem: ecosystem reuse

You're writing a vāṇी program. You need to:
- Compute SHA-256 hashes.
- Query a SQLite database.
- Decode JPEG images.
- Talk to a Bluetooth chip.

Each of these is *already* solved by some C library (openssl,
sqlite3, libjpeg, BlueZ). Writing your own is months of work
PLUS risks bugs the libraries have already squashed.

The vAṇी solution: don't rewrite — call the C library
directly. The mechanism is **FFI** — Foreign Function
Interface. Your vāṇी program calls into pre-compiled C code as
if it were a vāṇी function.

The catch: C and vāṇी disagree about a LOT of things. FFI is
the negotiation that makes them play nice.

## What disagrees

### 1. Memory layout

A vāṇी struct `Point { x: i64, y: i64 }` and a C struct
`struct point { int64_t x; int64_t y; }` might look identical
to you. They probably ARE identical in bytes (16 bytes total).
But:

- vāṇी's `OwnedStr` is a `char*` to heap-allocated NUL-terminated
  bytes. C's `char*` could be that, or could be a pointer into a
  string-literal table, or anywhere else.
- vāṇी's `Vec<i64>` is a 3-word struct (data pointer + length +
  capacity). C has no built-in equivalent — you'd have to pass
  the three pieces separately.
- vāṇी's enums are tagged unions; C enums are just integers.

Some types map cleanly (`i64` ↔ `int64_t`, `Str` ↔ `const
char*`); others need adapters.

### 2. Calling convention

When function `foo` calls `bar`, the arguments and return value
have to pass between them via a specific protocol — which CPU
registers hold which values, who saves what, who cleans up the
stack. This protocol is called a **calling convention**.

The good news: on each platform, there's a *dominant* calling
convention (System V on Linux/macOS, Microsoft x64 on Windows),
and most compilers use it. vāṇी and most C compilers agree on
the platform's dominant convention. FFI between them "just
works" for simple cases.

The bad news: structs-by-value have *intricate* rules — small
structs go in registers; larger ones get spilled to the stack
with hidden alignment padding. Vāṇी handles a subset of these
patterns and rejects the rest with a clear migration hint.

### 3. Ownership

Vāṇी has affine ownership — every value has exactly one
owner; the compiler tracks who owns what. C has no concept of
ownership; pointers are pointers.

When you call into C:
- Passing a Vec → who owns the data? If C frees it, vāṇी's
  affine bookkeeping is now wrong (vāṇी thinks the data still
  exists). If vāṇी frees it AND C also frees it → double-free.
- Receiving a `char*` from C → who frees this? vāṇी doesn't
  know. The user must know whether it's a literal (never free)
  vs a malloc'd string (caller frees) vs a static buffer
  (don't free).

vāṇी sidesteps this for the easy cases (scalars + references)
and uses the `unsafe(reason = "...")` block for the cases
where you have to manually manage ownership across the FFI
boundary.

### 4. Error handling

C signals errors via return values (usually negative for error,
non-negative for success) AND a thread-local `errno` global.
vāṇी uses Result types + the `try`/`?` operators.

The FFI bridge is usually a thin vāṇी wrapper around the C call
that converts (negative return) into a Result variant. You
write the wrapper once; the rest of your vāṇी code uses
idiomatic Result handling.

## How vāṇी spells FFI

The actual chapter ([Intermediate 9](09_ffi.md)) has the
syntax. The shape:

```rust
extern "C" fn sqrt(x: f64) -> f64;

fn main() -> i64 {
  let r: f64 = sqrt(2.0);
  print "sqrt(2) =", r;
  return 0;
}
```

`extern "C"` declares: "there exists a C function with this
name + signature; trust me, link it in." When you run `vanic
build --link-with=m`, vāṇी links against libm and the call
resolves to the actual libm `sqrt`.

For scalars + bool + Str + references, the FFI is **safe by
default** — vāṇी handles all the calling-convention details.

For structs-by-value, the compiler checks if the struct fits
the platform's small-struct convention. Yes → safe. No → the
compiler rejects with a hint: "pass by reference instead."

For raw pointer types (`*const T`, `*mut T`), you'd be working
with C-style memory. These types only appear inside an
`unsafe(reason = "...")` block — vāṇी forces you to mark the
section where the type system isn't tracking ownership.

## Function pointers — calling vāṇी from C

The reverse direction also works. vAṇी functions can be passed
as callbacks to C functions:

```rust
extern "C" fn qsort(
  base: *mut i64, n: u64, size: u64,
  cmp: fn(*const i64, *const i64) -> i64,
);

fn compare(a: *const i64, b: *const i64) -> i64 {
  ...
}

fn main() -> i64 {
  let xs: [i64; 4] = [3, 1, 4, 2];
  qsort(/* ... */, compare);   // vāṇी fn passed to C
  return 0;
}
```

The vāṇी function `compare` becomes a callback C can invoke.
This is how you wire vāṇी logic into existing C frameworks
(libuv, GTK, etc.).

## "ABI" — the deeper word

When you read FFI docs you'll see **ABI** — Application
Binary Interface. It's the broader term covering: calling
convention + type layout + ownership conventions + error
signaling. Two languages have "compatible ABIs" when their
ABIs match enough that they can call each other directly.

vāṇी commits to being **ABI-compatible with C on each
supported platform**. This is what makes FFI work. The two
languages share the platform's calling convention and use
compatible type layouts for the supported primitive types.

vāṇी does NOT commit to a stable ABI across vāṇी VERSIONS.
You can't compile a library with vāṇी 1.0 and call it from
vāṇी 1.1 without recompiling. C++ has the same restriction;
Rust does too.

## When FFI is the right tool

Use FFI when:
- You need a specific existing library (cryptography, codecs,
  databases, OS APIs).
- You're integrating vāṇी into a C codebase incrementally.
- You're targeting a platform where the OS API is C-shaped
  (Linux syscalls, Windows API, POSIX).

DON'T use FFI when:
- A pure-vāṇी alternative exists (use it — better safety
  properties).
- The C library is small/simple (might be easier to port than
  wrap).
- You're tempted to use it "for performance" but haven't
  measured. Vāṇी compiles to native code via LLVM/C
  backend; it's not slower than C for most workloads.

## A summary you can carry

- **FFI** = mechanism for one language to call another. Lets
  vāṇी use the vast C ecosystem.
- **Disagreements**: memory layout, calling convention,
  ownership semantics, error handling. vāṇी handles the easy
  cases automatically; harder cases need explicit `unsafe`
  blocks.
- **`extern "C" fn name(...)`** declares a C function.
  Calling it works like a normal call.
- **`--link-with=libname`** is the linker hint to find the
  actual symbol at build time.
- vāṇी commits to ABI compatibility with C on each platform —
  not across vāṇी versions.

This is enough to read the formal chapter without getting
lost in the syntax. The actual examples will exercise this
shape against libm, libc string functions, and qsort-style
callbacks.

## Cross-reference

- [Intermediate 9 — FFI: `extern "C"` + `--link-with`](09_ffi.md)
  — the actual syntax + worked examples
- [Beginner 6a — Pointers and references primer](../beginner/06a_pointers_refs_primer.md)
  — references map naturally to C pointers
- [Advanced 4 — Embedded targets + `unsafe`](04_embedded.md)
  — the `unsafe(reason = "...")` block is shared between FFI
  and embedded use cases
- [Beginner 6c — Ownership primer](../beginner/06c_ownership_primer.md)
  — why ownership is the gnarliest cross-boundary issue
