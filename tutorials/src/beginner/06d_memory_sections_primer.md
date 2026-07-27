# Beginner 6d -- Program memory layout: `.text`, `.rodata`, `.data`, `.bss` (intuition primer)

> **Learning goal**: understand the handful of named regions a
> compiled program is divided into *before it even starts
> running* -- where the instructions live, where string literals
> live, and the difference between a global that costs disk
> space and one that doesn't. This is the "third region" that
> [Beginner 6b](06b_heap_vs_stack_primer.md) waved at and
> deferred.

This chapter has **no compiler code**. Pure intuition.

## The stack and heap aren't the whole story

[Beginner 6b](06b_heap_vs_stack_primer.md) covered two regions
that exist *while your program runs*: the stack (function-call
frames) and the heap (`Vec`, `OwnedStr`, `Box` data). Both are
empty when the program starts -- they fill up as `main` runs.

But something has to be true *before* a single instruction
executes: the instructions themselves have to be somewhere in
memory, and so does `"hello"` in `print "hello";`. That "somewhere"
is baked into the compiled binary itself, in a small set of named
sections. A linker (the tool that stitches compiled `.o` files into
one executable) groups every piece of output into one of these
sections by kind.

## The moving truck

Picture packing a moving truck for a household move. Everything
that goes in the truck falls into one of a few labeled zones, and
the mover decides which zone based on one question: *does this need
to physically ride in the truck, or can we just note it down and
deal with it at the new place?*

- **The furniture that's already assembled** -- a bookshelf, a sofa
  -- goes in as-is, no packing needed, ready to use the moment it's
  unloaded. That's `.text`: your compiled functions, ready to run.
- **The box labeled "fragile -- do not open," full of framed photos
  and heirlooms that never change** -- it rides in the truck exactly
  as packed, and nobody's allowed to write into it at the new house.
  That's `.rodata`: read-only constants like `"hello"`.
- **The box labeled "kitchen -- already has plates in it"** -- it
  also rides in the truck, pre-filled with specific items, but
  unlike the fragile box you're allowed to swap what's in it once
  you unpack. That's `.data`: globals that start with a real,
  non-zero value.
- **The empty dresser you're planning to fill with towels once you
  arrive** -- there's no reason to load an empty dresser full of
  nothing. The mover just writes "1 empty dresser, towel-sized" on
  the inventory sheet and buys an *identical empty dresser* at the
  destination. Nothing rode in the truck; only the size was noted.
  That's `.bss`: globals that start at zero -- why ship a million
  zero bytes when "make this many zero bytes" is one line on the
  inventory?

The first three zones cost truck space (disk space) in proportion to
what's actually in them. The fourth zone costs nothing to ship --
only something to remember.

## The four sections

Think of the compiled binary as a shipping crate with labeled
compartments:

| Section | Holds | Read-only? | Present in the binary FILE? |
|---|---|---|---|
| **`.text`** | Compiled machine instructions (your functions) | Yes (and executable) | Yes -- every byte |
| **`.rodata`** | Read-only constants: string literals, other compile-time constants | Yes | Yes -- every byte |
| **`.data`** | Global/static variables that start with a **non-zero** initial value | No | Yes -- every byte (the initial values are stored) |
| **`.bss`** | Global/static variables that start at **zero** | No | **No** -- only a size is recorded |

`.text`, `.rodata`, and `.data` all cost disk (or Flash) space
proportional to their content, because the file has to store the
actual bytes. `.bss` is the odd one out: a global array of
1,000,000 zeroed `i64`s costs 8MB of *RAM* at runtime but **zero
bytes in the binary file** -- there's no point storing a million
zeroes on disk when "zero this many bytes" is a two-word
instruction. ("bss" is a historical assembler term -- "Block
Started by Symbol" -- that stuck around for seventy years because
nobody found a better name.)

## Who fills `.bss` with zeroes?

Something has to actually zero that RAM before your code can rely
on it reading as zero. Two answers, depending on platform:

- **Hosted programs** (running under an OS -- Linux, macOS,
  Windows): the OS loader zeroes `.bss`'s pages as part of setting
  up the process, before `main` runs. You never think about it.
- **Bare-metal / embedded** (no OS -- see
  [Advanced 4a](../advanced/04a_embedded_primer.md)): there's no
  loader. The very first code that runs after reset -- conventionally
  named `Reset_Handler` -- is responsible for copying `.data`'s
  initial values from Flash into RAM AND zeroing `.bss`, by hand,
  before calling `main`. [Advanced 4b's linker-script
  table](../advanced/04b_cross_compile_primer.md#link_section----place-code--data-at-a-specific-address)
  shows where these sections get mapped on a Cortex-M target.

## Where vāṇी constructs land

| vāṇी construct | Section |
|---|---|
| A string literal `"hello"` (backing bytes for `Str`) | `.rodata` -- see [Beginner 6, strings](06_strings.md) |
| A compiled function body | `.text` |
| A local variable (`let x: i64 = 5;`) | the **stack**, not a static section -- see [Beginner 6b](06b_heap_vs_stack_primer.md) |
| `Vec<T>` / `OwnedStr` backing bytes | the **heap**, not a static section |

Notice what's missing from that table: a vāṇी *global mutable
variable*. vāṇी v1 has no `static` keyword and no mutable
module-level state -- every binding is either a local (stack), a
heap allocation reached through a local handle, or a `.rodata`
constant. That's a deliberate simplification (mutable globals are
a classic source of hidden coupling and data races), and it also
means `.data` and `.bss` are populated almost entirely by the C
runtime your program links against, not by your own vāṇी code --
**except on embedded targets**, where `#[link_section]` (see
[Advanced 4b](../advanced/04b_cross_compile_primer.md)) lets you
place hardware-specific data explicitly, and where the
`Reset_Handler` you write is exactly the code responsible for
the `.data`-copy / `.bss`-zero step described above.

## A summary you can carry

- A compiled binary is divided into named sections *before it
  runs*: `.text` (code), `.rodata` (read-only constants),
  `.data` (initialized globals), `.bss` (zero-initialized
  globals).
- `.text` / `.rodata` / `.data` all store real bytes in the
  binary file. `.bss` stores only a *size* -- the zero bytes are
  synthesized at load time, not read from disk.
- On a hosted OS, the loader zeroes `.bss` for you. On bare
  metal, your own `Reset_Handler` does it by hand, along with
  copying `.data` from Flash to RAM.
- vāṇी string literals live in `.rodata`; compiled functions
  live in `.text`. vāṇी v1 has no mutable globals, so `.data`
  and `.bss` are mostly a "who links your program" concern
  rather than a "what did I write" concern -- until you're
  targeting embedded, where they become directly visible.
- This is a *different* split from the stack/heap split in
  [Beginner 6b](06b_heap_vs_stack_primer.md): stack and heap are
  regions that fill up *while the program runs*; `.text` /
  `.rodata` / `.data` / `.bss` are regions baked into the binary
  *before* it ever starts.

## Cross-reference

- [Beginner 6b -- Heap and stack primer](06b_heap_vs_stack_primer.md)
  -- the two regions that exist only while the program runs
- [Beginner 6 -- Strings (`Str` vs `OwnedStr`)](06_strings.md)
  -- `Str` literals point into `.rodata`
- [Advanced 4a -- Embedded primer](../advanced/04a_embedded_primer.md)
  -- why bare-metal targets have no OS loader to zero `.bss` for you
- [Advanced 4b -- Cross-compilation primer](../advanced/04b_cross_compile_primer.md)
  -- the linker-script section table, `#[link_section]`, and
  `Reset_Handler`


---

**Previous**: [Sec.6 -- Strings (Str vs OwnedStr) ->](06_strings.md)
**Next**: [Sec.7a -- Tuples and destructuring primer ->](07a_tuples_primer.md)
