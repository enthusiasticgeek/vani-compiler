# Advanced 4a -- Embedded, `unsafe`, and regions (intuition primer)

> **Learning goal**: build a mental model of "embedded
> programming" -- what's different about running on tiny
> hardware -- and the language features vāṇी provides for it
> (`unsafe(reason = "...")` blocks + region typing). Reading
> order: [Beginner 6b heap/stack primer](../beginner/06b_heap_vs_stack_primer.md)
> -> this -> [Advanced 4 -- Embedded](04_embedded.md).

This chapter has **no compiler code**. Pure intuition.

## What is "embedded"?

Most of programming today targets ROUGHLY one shape of
machine: a laptop, phone, or server with gigabytes of RAM,
billions of CPU cycles per second, an OS that manages memory
+ threads + filesystems, and a runtime library that provides
heap allocation + I/O.

**Embedded** programming targets the OPPOSITE: tiny chips
running INSIDE physical objects. A thermostat. A pacemaker. A
satellite. A keyboard's firmware. The constraints:

- **Memory**: 8KB to 256KB of RAM. Not gigabytes -- kilobytes.
- **CPU**: maybe 16 MHz to 200 MHz. A laptop is 3000+ MHz.
- **No OS** (often): your code runs directly on the metal.
  No filesystem, no threads, no `print`. Just the CPU + the
  pins + the registers.
- **No heap** (often): no `malloc` is available. Every allocation
  is decided at compile time + fixed in size.
- **Real-time deadlines**: the brake-controller chip MUST
  respond within X microseconds. A garbage collector pause
  is catastrophic.
- **Battery**: every CPU cycle drains battery. Idle is
  precious.

Code that works fine on a server explodes here. A `Vec<i64>`
that auto-grows? Forbidden -- there's no heap. A recursive
function with no bound? Forbidden -- stack overflow. An
unbounded loop? Maybe forbidden -- could miss a real-time
deadline.

vāṇी has language-level features specifically for this
environment.

## The first feature: explicit `no_heap`

A function (or whole program) can be annotated:

```vani
#[no_heap]
fn process_packet(buf: ref [u8; 256]) -> i64 { ... }
```

This says: "compile-time check that this function does not
trigger any heap allocation." Any code path that would call
`malloc` (directly or via `Vec::new()`, `OwnedStr`, `Box::new`,
etc.) is rejected at compile time.

The compiler verifies the entire transitive call graph.
You get a guarantee, not a hope.

## The second feature: bounded recursion + stack budgets

A function can declare its maximum stack usage:

```vani
#[bounded_stack(64)]
fn parse_packet(input: ref [u8; 1024]) -> Packet { ... }
```

The compiler ensures the function (and everything it transitively
calls) never uses more than 64 bytes of stack. Recursive
functions need an explicit `#[recursion_bound(N)]` so the
total bound is computable.

On a chip with 8KB of RAM total, you might budget 2KB for the
stack. Each top-level handler can declare its share. The
compiler catches budget violations BEFORE you deploy to
hardware where debugging is excruciating.

## The third feature: deterministic timing

For real-time code:

```vani
#[deterministic_timing]
fn step_motor() -> i64 { ... }
```

The compiler rejects constructs whose execution time isn't
statically predictable:
- Heap allocations (variable time).
- Unbounded loops without invariants.
- Recursive calls without a bound.
- Calls to non-`#[deterministic_timing]` functions.

What remains is straight-line code + bounded loops + bounded
recursion -- execution time you can analyze with
worst-case-execution-time (WCET) tools.

## The fourth feature: `unsafe(reason = "...")`

Sometimes you genuinely need to:
- Talk directly to a hardware register (a specific memory
  address).
- Do pointer arithmetic that vāṇी's ownership system can't
  track.
- Call into legacy C firmware that doesn't follow vāṇी rules.

For these, you write:

<img class="manas" src="../images/mascot/manas_mascot_caution.png" title="this code needs extra care"/>

```vani
unsafe(reason = "GPIO pin 13 toggle -- hardware-defined memory address") {
  let gpio_pin_13: *mut u32 = 0x40020000 as *mut u32;
  write_volatile(gpio_pin_13, 1);
}
```

Two things matter:

1. **`unsafe` is a BLOCK, not a function modifier**. The unsafe
   region is *visible in the source* -- anyone reading the
   code sees exactly where vāṇी's safety guarantees stop. The
   rest of your code is still type-checked.

2. **`reason = "..."`** is MANDATORY. You can't just write
   `unsafe { ... }`. Every unsafe block must explain why it's
   needed. The reason becomes part of your code review
   evidence -- "is this *really* necessary, and is the
   explanation correct?"

These two design choices distinguish vāṇी's `unsafe` from
C-style "everything is unsafe, who cares?". In C, every pointer
dereference could be unsafe but nothing flags it. In vāṇी, the
unsafe blocks are tiny, visible, and required to justify
themselves.

## The fifth feature: region typing

Allocator-free programs often use a pattern called **arena**
or **region** allocation. Instead of a global heap, you have
a fixed-size buffer; you allocate from it bumping a pointer
up; you free the WHOLE buffer at once when you're done.

```vani
region scratch {
  let temp_a: ArenaRef<i64> = region_borrow_i64(mut ref scratch, 100);
  let temp_b: ArenaRef<i64> = region_borrow_i64(mut ref scratch, 200);
  // ... use aref_load(temp_a), aref_load(temp_b) ...
}  // scratch's whole backing storage is freed; temp_a and temp_b are gone
```

Inside the region, allocations are essentially free (just a
pointer bump). The compiler tracks that `temp_a` and `temp_b`
are tied to `scratch`'s lifetime -- they can't escape the
region's scope (trying to return one from the enclosing
function is a compile error, not a runtime one). When the
region ends, all allocations within it are released together.
The shipped v1 slot type is `i64` (see
[Advanced 4 -- Embedded](04_embedded.md) for the full,
runnable `region` / `ArenaRef<i64>` example); a generic
`ArenaRef<T>` for arbitrary structs is future work.

Region typing is the compile-time mechanism that prevents you
from accidentally storing a region-allocated pointer in a
binding that outlives the region. It composes with `no_heap`:
heap-free code can still use regions for scratch space.

## Why "embedded" features matter even on big machines

A surprising number of "embedded" disciplines pay off in
server / desktop code too:

- **`no_heap` for a hot loop** -- guarantee the loop doesn't
  trigger GC-style pauses, even if your overall program does
  use the heap.
- **`bounded_stack` for recursive parsers** -- catch the bug
  where a malicious input triggers stack overflow.
- **Regions for batch processing** -- process N items, all
  allocations live in one region, drop the whole region
  between batches. No fragmentation, no leak risk.

The features are *easier to apply* in embedded because the
constraints force you to think about them. But they're useful
discipline anywhere.

## When NOT to reach for these

For ordinary application code (a web server, a desktop tool, a
script), you usually don't need any of these. The default vāṇी
shape -- ownership-tracked heap allocation, automatic Drop,
unbounded recursion within reason -- is fine. The embedded
features are *opt-in*; you reach for them when the constraints
match.

## A summary you can carry

- **Embedded** = code running on tiny chips: kilobytes of RAM,
  slow CPU, no OS, no heap, real-time deadlines.
- **`#[no_heap]`** = "this function (transitively) never
  allocates from the heap." Compile-time verified.
- **`#[bounded_stack(N)]`** + **`#[recursion_bound(N)]`** =
  static guarantees on stack usage.
- **`#[deterministic_timing]`** = rejects constructs whose
  execution time isn't statically predictable.
- **`unsafe(reason = "...")`** = small visible escape hatch
  for hardware addresses, pointer arithmetic, or legacy C
  interop. The mandatory reason string forces explicit
  justification.
- **Region typing** = arena allocation with compile-time
  scope tracking; allocations can't escape the region.

Together these let vāṇी target embedded shapes without losing
the safety story. The next two chapters cover the rest:
- [Advanced 4b -- Cross-compilation primer](04b_cross_compile_primer.md)
  -- `--target <triple>`, `--no-std`, `#[no_mangle]`, `#[link_section]`,
  MMIO u8/u16, QEMU user-mode run
- [Advanced 4 -- Embedded targets + `unsafe`](04_embedded.md)
  -- worked examples for LED-blink firmware + packet-parsing handler

## Cross-reference

- [Beginner 6b -- Heap and stack primer](../beginner/06b_heap_vs_stack_primer.md)
  -- foundation: why heap is heavy, why embedded code avoids it
- [Beginner 6c -- Ownership primer](../beginner/06c_ownership_primer.md)
  -- region typing is ownership tracking with explicit scope
  boundaries
- [Intermediate 9a -- FFI primer](../intermediate/09a_ffi_primer.md)
  -- embedded code often interops with legacy C firmware via
  `extern "C"` + `unsafe`
- [Advanced 4b -- Cross-compilation primer](04b_cross_compile_primer.md)
  -- `--target`, `--no-std`, `#[no_mangle]`, `#[link_section]`, MMIO u8/u16
- [Advanced 4 -- Embedded targets + `unsafe`](04_embedded.md)
  -- the actual syntax + worked examples


---

**Previous**: [Sec.3b -- Condition variables primer ->](03b_condvar_primer.md)
**Next**: [Sec.4b -- Cross-compilation and bare-metal targets primer ->](04b_cross_compile_primer.md)

