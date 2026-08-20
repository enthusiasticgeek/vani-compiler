# Advanced 4a -- Embedded, `unsafe`, and regions (intuition primer)

> **Learning goal**: build a mental model of "embedded
> programming" -- what's different about running on tiny
> hardware -- and the language features vāṇī provides for it
> (`unsafe(reason = "...")` blocks + region typing). Reading
> order: [Beginner 6b heap/stack primer](../beginner/06b_heap_vs_stack_primer.md)
> -> this -> [Advanced 4 -- Embedded](04_embedded.md).

This chapter is mostly intuition, with real code illustrating
embedded-specific constructs once the analogy lands.

## The microwave and the laptop

Think about your laptop for a second. Dozens of things are running
on it right now -- a browser with fifteen tabs, a music player, a
background virus scanner, this very editor -- and an operating
system is constantly deciding whose turn it is to use the CPU,
handing out slices of memory to whoever asks, and juggling all of it
so smoothly you never notice the plumbing. If you want to install a
brand-new program tomorrow, you just download it and run it. The
laptop has gigabytes of memory to spare and barely notices.

Now think about the microwave sitting on your kitchen counter. It
also has a tiny computer inside it -- a real chip, running real
code -- but the two situations could not be more different. The
microwave's chip runs exactly ONE program, and it has run that same
one program, unchanged, since the day it left the factory. There is
no operating system inside a microwave juggling between programs,
because there's only ever one program to juggle -- there's nothing
to arbitrate. There's no "install a new app" button, because the
code is soldered into the hardware, not loaded from a disk. And
instead of gigabytes of memory, the microwave's chip typically has a
handful of KILOBYTES -- enough to track the clock, the power level,
and whether the door is open, and not really anything more.

If the microwave's program has a bug -- say it forgets to ever
release some memory it borrowed -- there's no operating system
standing by to notice, kill the misbehaving program, and reclaim the
memory for the next one. There IS no "next one." Whatever the
program does, it does directly to the hardware, permanently, for as
long as the microwave sits on your counter. A crashed browser tab
just closes; a crashed microwave chip is a microwave you have to
unplug and plug back in, or worse, one that gets stuck with the
turntable spinning forever.

This is the split embedded programming lives on. Writing software
for your laptop is writing for an environment with an operating
system underneath you, catching your mistakes, cleaning up after
you, and giving you room to be a little careless. Writing software
for the microwave's chip -- or a pacemaker, a thermostat, a
satellite -- means there is no OS underneath you at all. Whatever
safety net you want, you have to build into the program itself,
because nothing else is watching.

That's exactly why the rest of this chapter reads stricter than
ordinary vāṇī code: `unsafe(reason = "...")` blocks, region typing,
and hard limits on heap and stack usage are the tools vāṇī gives you
to be careful BY HAND, on the microwave's chip, in a world where
there's no operating system left to be careful for you.

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

vāṇī has language-level features specifically for this
environment.

## The first feature: explicit `no_heap`

A function (or whole program) can be annotated:

```vani
#[no_heap]
fn process_packet(buf: ref [u8; 256]) -> i64 { ... }
```

This says: "compile-time check that this function does not
trigger any heap allocation." Any code path that would call
`malloc` (directly or via `vec(...)`, `OwnedStr`, `box(...)`,
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

## Before that: what makes a raw pointer different from `ref`?

[Beginner 6a](../beginner/06a_pointers_refs_primer.md) built the
library-card picture: a `ref` is like telling a friend a book's shelf
location instead of handing them the book. That picture already
covers 95% of vāṇी code -- but it left out one detail on purpose,
because it only applies here, in embedded/unsafe territory.

A real library has a librarian. When you say "Aisle 3, Shelf 5,
Position 12," the librarian's system already guarantees that position
exists, that it currently holds a book (not empty space), and that
nobody is going to reshelve it out from under you mid-checkout. That
guarantee is exactly what `ref`/`mut ref` give you -- the compiler
plays librarian, and it refuses to compile code where the guarantee
could be broken. That's WHY chapters before this one never mention
"what if the address is wrong."

A **raw pointer** (`*const T` / `*mut T`) is the same "aisle, shelf,
position" address -- but written on a sticky note with no librarian
attached. Nothing checks that the position you wrote down is real.
Nothing stops you writing down a position for a shelf that was already
cleared out. Nothing stops two different notes claiming the exact same
spot at once, one of them about to overwrite what the other expects to
still be there. The note is just three numbers -- it's on YOU to know
they're still good before you act on them.

This isn't a flaw in raw pointers -- it's the whole reason they exist.
Sometimes the "shelf" isn't managed by any librarian at all: a
hardware register at a fixed memory address that the CPU itself
defines, not vāṇी's allocator; a buffer handed to you by C code that
was never taught about `ref`; a block you're managing yourself because
you know its lifetime better than the compiler's rules can express.
`unsafe(reason = "...")`, `Tainted<T>`, and `BoundedPtr<T>` (all
covered below and in [Advanced 4](04_embedded.md)) are vāṇी's way of
saying: "fine, no librarian here -- but you still have to sign your
name every time you skip the checkout desk, and here are some optional
guard rails (bounds-checked wrappers) you can bolt back on yourself."

| | `ref T` / `mut ref T` | `*const T` / `*mut T` |
|---|---|---|
| Who checks the address is valid? | The compiler, always | Nobody -- you |
| Can two conflicting ones exist at once? | Compiler rejects it | Yes, silently |
| Can it point at freed/reused memory? | Compiler rejects it | Yes, silently (a "dangling pointer" / "use-after-free") |
| Needs an `unsafe` block? | Never | Yes, to read or write through it |
| Where you'll actually meet it | Everywhere in ordinary vāṇी code | Hardware registers, C interop, hand-written allocators |

## The fourth feature: `unsafe(reason = "...")`

Sometimes you genuinely need to:
- Talk directly to a hardware register (a specific memory
  address).
- Do pointer arithmetic that vāṇī's ownership system can't
  track.
- Call into legacy C firmware that doesn't follow vāṇī rules.

For the first case, note that `volatile_read`/`volatile_write`
(guaranteeing the compiler won't coalesce or reorder register
accesses) DON'T actually need an `unsafe` block at all -- they take
a plain `ref`/`mut ref i64`, gated only on the embedded build target
(`INTENT_TARGET_EMBEDDED=1`; see
[`examples/embedded/mmio_blink.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/embedded/mmio_blink.vani)
for the full runnable pattern). v1 has no way to construct a raw
pointer from an arbitrary hardcoded address (`0x40020000 as *mut T`
is rejected outright -- casting TO a raw pointer type only works
FROM an existing vāṇī binding, `&x as *const T` / `&mut x as *mut
T`); real firmware binds a fixed hardware address to a vāṇī variable
via a linker symbol instead (`#[link_section]`, covered in
[Advanced 4b](04b_cross_compile_primer.md)), not pointer casting.

`unsafe` genuinely IS required for the second and third cases --
raw pointer arithmetic and legacy C interop. One prerequisite before
the example below will actually run: `unsafe(reason = "...")` itself
is also gated to `INTENT_TARGET_EMBEDDED=1` (rejected outright without
it, on a normal hosted build) -- see the note at the top of
[Advanced 4's "The unsafe block"](04_embedded.md#the-unsafe-block)
for why.

<img class="manas" src="../images/mascot/manas_mascot_caution.png" title="this code needs extra care"/>

```vani
unsafe(reason = "scratch buffer via raw pointer for pointer-arithmetic demo") {
  let p: *mut i64 = unsafe_alloc(3);
  let bp: BoundedPtr<i64> = bptr_new(p, 3, 3);
  let _ = bptr_set(mut ref bp, 0, 42);
  let v: i64 = option_unwrap_or(bptr_get(ref bp, 0), 0 - 1);
  print "v =", v;
}
```

Two things matter:

1. **`unsafe` is a BLOCK, not a function modifier**. The unsafe
   region is *visible in the source* -- anyone reading the
   code sees exactly where vāṇī's safety guarantees stop. The
   rest of your code is still type-checked.

2. **`reason = "..."`** is MANDATORY. You can't just write
   `unsafe { ... }`. Every unsafe block must explain why it's
   needed. The reason becomes part of your code review
   evidence -- "is this *really* necessary, and is the
   explanation correct?"

These two design choices distinguish vāṇī's `unsafe` from
C-style "everything is unsafe, who cares?". In C, every pointer
dereference could be unsafe but nothing flags it. In vāṇī, the
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
script), you usually don't need any of these. The default vāṇī
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

Together these let vāṇī target embedded shapes without losing
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

