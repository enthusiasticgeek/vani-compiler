# Intermediate 9a -- FFI: calling C code from vāṇी (intuition primer)

> **Learning goal**: build a mental model of "FFI" -- Foreign
> Function Interface. The way one language calls into another.
> Why this exists, what could go wrong, and what vāṇī does to
> make it tractable. Reading order:
> [Intermediate 9 -- FFI: extern "C" + --link-with](09_ffi.md)
> follows this; read this primer first if you've never used
> a foreign-function call before.

This chapter is mostly intuition, with real `extern "C"` code once
the analogy lands.

## Hiring a translator

Imagine you run a small business and you've landed a meeting with an
important partner from another country. They speak a language you
don't -- and you don't have time to become fluent in it before
Monday's meeting. So you don't try. Instead, you hire a professional
interpreter, and crucially, you sit down with them BEFORE the meeting
and agree on exactly how the conversation is going to work: what
order topics come up in, which technical terms need an exact
agreed-on translation (so "deposit" means the same thing to both
sides), and what a signed-off "yes" is supposed to look like from
each party. Once that protocol is settled, you walk into the meeting
and talk business. You never learn the partner's language. You don't
need to know how their legal system works internally, or how their
accounting software is structured. You only need to trust that the
interpreter -- following the protocol you both agreed on in advance
-- will faithfully carry your words across the language boundary and
bring their reply back in a shape you understand.

A fixed-format bilingual contract template does the same job in
writing: both sides sign off in advance on which blank means what, in
what order the clauses appear, and what units the numbers are in
(dollars, not some other currency). Neither party has to read the
other's native paperwork conventions to know what's being agreed to.
The template itself IS the agreement about how the two sides will
talk.

That pre-agreed protocol -- not fluency, not shared internals, just
an exact, mutually-understood contract for the conversation -- is
what **FFI** is between two programming languages. vāṇī and C are, in
effect, two parties who don't speak each other's internal language:
they lay data out in memory differently, they disagree about who's
responsible for cleaning up after a value, and their error-signaling
conventions differ. FFI doesn't make vāṇī fluent in C, or C fluent in
vāṇī. Instead, `extern "C" fn ...` is the contract template: it
declares, in a fixed, exact shape, "here's precisely what I'm about
to hand you, and here's precisely what I expect back" -- so the two
sides can cooperate on a single function call without either one
needing to understand how the other works on the inside.

Keep that picture -- interpreter, pre-agreed protocol, neither side
needing to learn the other's internals -- in mind as you read the
rest of this chapter. Every technical wrinkle below (memory layout,
calling conventions, ownership, error handling) is really just one
more thing the protocol has to pin down before the conversation can
happen safely.

## The problem: ecosystem reuse

You're writing a vāṇī program. You need to:
- Compute SHA-256 hashes.
- Query a SQLite database.
- Decode JPEG images.
- Talk to a Bluetooth chip.

Each of these is *already* solved by some C library (openssl,
sqlite3, libjpeg, BlueZ). Writing your own is months of work
PLUS risks bugs the libraries have already squashed.

The vāṇī solution: don't rewrite -- call the C library
directly. The mechanism is **FFI** -- Foreign Function
Interface. Your vāṇī program calls into pre-compiled C code as
if it were a vāṇī function.

The catch: C and vāṇī disagree about a LOT of things. FFI is
the negotiation that makes them play nice.

## What disagrees

### 1. Memory layout

A vāṇī struct `Point { x: i64, y: i64 }` and a C struct
`struct point { int64_t x; int64_t y; }` might look identical
to you. They probably ARE identical in bytes (16 bytes total).
But:

- vāṇī's `OwnedStr` is a `char*` to heap-allocated NUL-terminated
  bytes. C's `char*` could be that, or could be a pointer into a
  string-literal table, or anywhere else.
- vāṇī's `Vec<i64>` is a 3-word struct (data pointer + length +
  capacity). C has no built-in equivalent -- you'd have to pass
  the three pieces separately.
- vāṇī's enums are tagged unions; C enums are just integers.

Some types map cleanly (`i64` <-> `int64_t`, `Str` <-> `const
char*`); others need adapters.

### 2. Calling convention

When function `foo` calls `bar`, the arguments and return value
have to pass between them via a specific protocol -- which CPU
registers hold which values, who saves what, who cleans up the
stack. This protocol is called a **calling convention**.

The good news: on each platform, there's a *dominant* calling
convention (System V on Linux/macOS, Microsoft x64 on Windows),
and most compilers use it. vāṇī and most C compilers agree on
the platform's dominant convention. FFI between them "just
works" for simple cases.

The bad news: structs-by-value have *intricate* rules -- small
structs go in registers; larger ones get spilled to the stack
with hidden alignment padding. Vāṇī handles a subset of these
patterns and rejects the rest with a clear migration hint.

### 3. Ownership

Vāṇī has affine ownership -- every value has exactly one
owner; the compiler tracks who owns what. C has no concept of
ownership; pointers are pointers.

When you call into C:
- Passing a Vec -> who owns the data? If C frees it, vāṇī's
  affine bookkeeping is now wrong (vāṇī thinks the data still
  exists). If vāṇī frees it AND C also frees it -> double-free.
- Receiving a `char*` from C -> who frees this? vāṇī doesn't
  know. The user must know whether it's a literal (never free)
  vs a malloc'd string (caller frees) vs a static buffer
  (don't free).

vāṇī sidesteps this for the easy cases (scalars + references)
and uses the `unsafe(reason = "...")` block for the cases
where you have to manually manage ownership across the FFI
boundary.

### 4. Error handling

C signals errors via return values (usually negative for error,
non-negative for success) AND a thread-local `errno` global.
vāṇī uses Result types + the `try`/`?` operators.

The FFI bridge is usually a thin vāṇī wrapper around the C call
that converts (negative return) into a Result variant. You
write the wrapper once; the rest of your vāṇī code uses
idiomatic Result handling.

## How vāṇी spells FFI

The actual chapter ([Intermediate 9](09_ffi.md)) has the
syntax. The shape:

```vani
extern "C" fn hypot(x: f64, y: f64) -> f64;

fn main() -> i64 {
  let r: f64 = hypot(3.0, 4.0);
  print "hypot(3,4) =", r;
  return 0;
}
```

(`hypot`, not `sqrt` -- vāṇī already ships a builtin named `sqrt`,
and `extern "C" fn sqrt(...)` is rejected as a name collision:
"function 'sqrt' is a built-in name and cannot be redefined."
Pick an FFI symbol name that isn't already one of vāṇī's many
built-in math helpers.)

`extern "C"` declares: "there exists a C function with this
name + signature; trust me, link it in." `vanic build
--link-with=m` is the general way to make sure a libm symbol
resolves at link time; in practice `libm` is folded into `libc`
on most modern toolchains (confirmed: the `hypot` example above
runs correctly via both `vanic run` and a plain `vanic build`
with no `--link-with` flag at all) -- reach for `--link-with`
when a symbol genuinely doesn't resolve without it.

For scalars + bool + Str + references, the FFI is **safe by
default** -- vāṇī handles all the calling-convention details.

For structs-by-value, the compiler checks if the struct fits
the platform's small-struct convention. Yes -> safe. No -> the
compiler rejects with a hint: "pass by reference instead."

**Raw pointer types (`*const T`, `*mut T`) do NOT cross the FFI
boundary at all in v1** -- `extern fn` parameters/returns are
scalars, `Str`, or `ref T` / `mut ref T` only; the checker
rejects a raw-pointer-typed `extern fn` parameter outright
("this type is not yet wired through the v1 FFI ABI; use
scalars, `Str`, or `ref T` instead"). For C functions that take
a pointer, use `ref T` / `mut ref T` on the vāṇī side --
references already compile down to plain pointers at the ABI
level, so they line up with C's `const T*` / `T*` without
needing `unsafe` at all. (Raw `*const T` / `*mut T` still exist
as a type -- see [Advanced 4 -- Embedded](../advanced/04_embedded.md)
-- but they're for embedded/MMIO-style code behind `unsafe`,
gated to `--target embedded`, not for FFI parameter types.)

## Function pointers -- calling vāṇी from C

The reverse direction also works. vāṇī functions can be passed
as callbacks to C functions:

<img class="manas" src="../images/mascot/manas_mascot_caution.png" title="this code needs extra care"/>

```vani
// A hypothetical vendor C library's callback signature
// (illustrative -- there's no real "vendor_sort" linked here;
// the point is the shape of the declaration).
extern "C" fn vendor_sort(
  base: mut ref i64, n: i64,
  cmp: fn(ref i64, ref i64) -> i64
) -> i64;

fn compare(a: ref i64, b: ref i64) -> i64 {
  return 0;
}

fn main() -> i64 {
  return 0;
}
```

The vāṇī function `compare` becomes a callback C can invoke --
note it takes `ref i64` for the elements being compared, not a
raw pointer, matching the FFI ABI rule above. This is how you
wire vāṇī logic into existing C frameworks (libuv, GTK, etc.).

## "ABI" -- the deeper word

When you read FFI docs you'll see **ABI** -- Application
Binary Interface. It's the broader term covering: calling
convention + type layout + ownership conventions + error
signaling. Two languages have "compatible ABIs" when their
ABIs match enough that they can call each other directly.

vāṇī commits to being **ABI-compatible with C on each
supported platform**. This is what makes FFI work. The two
languages share the platform's calling convention and use
compatible type layouts for the supported primitive types.

vāṇī does NOT commit to a stable ABI across vāṇī VERSIONS.
You can't compile a library with vāṇī 1.0 and call it from
vāṇī 1.1 without recompiling. C++ has the same restriction;
Rust does too.

## When FFI is the right tool

Use FFI when:
- You need a specific existing library (cryptography, codecs,
  databases, OS APIs).
- You're integrating vāṇī into a C codebase incrementally.
- You're targeting a platform where the OS API is C-shaped
  (Linux syscalls, Windows API, POSIX).

DON'T use FFI when:
- A pure-vāṇī alternative exists (use it -- better safety
  properties).
- The C library is small/simple (might be easier to port than
  wrap).
- You're tempted to use it "for performance" but haven't
  measured. Vāṇī compiles to native code via LLVM/C
  backend; it's not slower than C for most workloads.

## A summary you can carry

- **FFI** = mechanism for one language to call another. Lets
  vāṇī use the vast C ecosystem.
- **Disagreements**: memory layout, calling convention,
  ownership semantics, error handling. vāṇī handles the easy
  cases automatically; harder cases need explicit `unsafe`
  blocks.
- **`extern "C" fn name(...)`** declares a C function.
  Calling it works like a normal call.
- **`--link-with=libname`** is the linker hint to find the
  actual symbol at build time.
- vāṇī commits to ABI compatibility with C on each platform --
  not across vāṇī versions.

This is enough to read the formal chapter without getting
lost in the syntax. The actual examples will exercise this
shape against libm, libc string functions, and qsort-style
callbacks.

## Cross-reference

- [Intermediate 9 -- FFI: `extern "C"` + `--link-with`](09_ffi.md)
  -- the actual syntax + worked examples
- [Beginner 6a -- Pointers and references primer](../beginner/06a_pointers_refs_primer.md)
  -- references map naturally to C pointers
- [Advanced 4 -- Embedded targets + `unsafe`](../advanced/04_embedded.md)
  -- the `unsafe(reason = "...")` block is shared between FFI
  and embedded use cases
- [Beginner 6c -- Ownership primer](../beginner/06c_ownership_primer.md)
  -- why ownership is the gnarliest cross-boundary issue


---

**Previous**: [Sec.8 -- Multi-file projects + vani.toml ->](08_manifest.md)
**Next**: [Sec.9 -- FFI: extern C + --link-with ->](09_ffi.md)

