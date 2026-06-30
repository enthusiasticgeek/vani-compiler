# Advanced 4 -- Embedded targets + `unsafe` + region typing

> **Learning goal**: opt into the `unsafe` operations needed
> for embedded / systems work (raw pointers, region-scoped
> arenas, manual memory) -- and understand how vāṇी keeps the
> safety boundary explicit at the source level.

> **New to this?** Read [Advanced 4a -- Embedded primer](04a_embedded_primer.md) first.

Think of `unsafe` like a "service door" in a shopping mall.
The mall's public corridors are safe, well-lit, and governed
by posted rules. The service door lets authorized staff (the
ones who KNOW the rules) step behind the scenes to do things
the public corridors can't: move large equipment, access
electrical panels, interact with the building's raw
infrastructure. `unsafe(reason = "raw pointer write -- hardware register") {}` in vāṇी works the same way:
the rest of the language's safety rules still apply everywhere
else; the `unsafe` block is a clearly-marked zone where the
compiler relaxes exactly the restrictions that embedded
hardware access requires -- raw pointer reads, memory-mapped
I/O -- while keeping the boundary visible so reviewers know
which code needs extra scrutiny.

## The `unsafe` block

```vani
intent "Advanced 4 -- raw pointer + region-scoped arena.";

fn poke(addr: *mut i32, val: i32) -> i64 {
  unsafe(reason = "raw pointer write -- hardware register") {
    *addr = val;
  }
  return 0;
}
```

- `unsafe(reason = "raw pointer write -- hardware register") { ... }` blocks are the only place where the
  following are allowed:
  - Dereferencing a raw pointer (`*p`).
  - Calling a `pure extern "C" fn` that's marked `unsafe`.
  - `Pool` / `Handle` borrows that escape their region.
- The compiler proves *every other* operation safe. The
  `unsafe` keyword is a request to suspend specific safety
  rules in a small, identifiable scope.

## Raw pointers: `*const T` and `*mut T`

For interop with hardware MMIO, FFI buffers, or hand-written
allocators:

```vani
fn write_register(base: *mut u32, offset: u64, value: u32) -> i64 {
  unsafe(reason = "raw pointer write -- hardware register") {
    *(base + offset) = value;
  }
  return 0;
}
```

The C backend lowers these to plain `*` dereferences; the
LLVM backend uses `load` / `store` instructions.

## Region typing: `Pool<T>` + `Handle<T>`

For embedded targets without a malloc, vāṇी provides an
arena-style allocator scoped to a `region` block:

```vani
intent "Advanced 4 -- region-scoped Pool<i64>.";

fn use_pool() -> i64 {
  region {
    let pool: Pool<i64> = pool_new(64);    // 64 i64 slots
    let h1: Handle<i64> = pool_alloc(mut ref pool, 7);
    let h2: Handle<i64> = pool_alloc(mut ref pool, 42);
    let sum: i64 = handle_read(ref h1) + handle_read(ref h2);
    print "sum =", sum;
    return sum;
  }
  // Handles cannot escape the region.
}
```

- **`region { ... }`** opens a fresh region scope. Allocations
  inside live for the region; the compiler proves no
  `Handle<T>` escapes via the borrow checker.
- **`Pool<T>`** is the arena. `pool_new(capacity)` creates one
  with N slots backed by a stack-allocated array (no malloc).
- **`Handle<T>`** is an affine reference into the pool. Read
  with `handle_read`; write with `handle_write`.
- The C lowering is a static array + an index counter -- zero
  runtime allocator overhead.

## What's safe vs unsafe in this layer

| Layer | Operation | Safety |
|---|---|---|
| 1.1 | `*const T` / `*mut T` deref | unsafe |
| 1.2 | `Pool<T>` / `Handle<T>` | safe (region typed) |
| 2 | C extern fn call | safe (caller's responsibility) |
| 3 | `unsafe(reason = "raw pointer write -- hardware register") { ... }` block | author's responsibility |
| 4.1 | `-fstack-protector-strong` | safe (opt-in build flag) |

See [`unsafe.md`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/unsafe.md)
in the repo for the full layered design.

## Compile-time safety annotations

For embedded and safety-critical targets, vāṇī provides four
attributes that the compiler enforces statically -- no runtime
overhead.

### `#[no_heap]`

Forbids any heap allocation (no `Vec`, `OwnedStr`, `HashMap`, etc.)
inside the annotated function, *transitively* through all callees.

```vani
#[no_heap]
fn isr_handler(counter: i64) -> i64 {
  // Arithmetic and Pool<T> are fine.
  // Vec / OwnedStr / HashMap would be a compile error here.
  return counter + 1;
}
```

The compiler traverses the full call graph; if *any* callee
(directly or indirectly) allocates, you get a diagnostic naming
the chain.

### `#[bounded_stack(bytes=N)]`

Asserts that the function's stack frame -- plus all transitive
callees -- fits within N bytes. Useful when targeting MCUs with
a few kilobytes of stack.

```vani
#[bounded_stack(bytes=512)]
fn sensor_read(pin: i64) -> i64 {
  let raw: i64 = pin * 3;
  return raw;
}
```

Setting `bytes=0` is a compile error (the attribute requires a
positive budget). Setting the budget smaller than the actual frame
produces a diagnostic with the measured size.

### `#[deterministic_timing]`

Rejects `while` loops, unbounded `for`, and `if` arms with
different call costs -- anything that would make execution time
depend on the input. The function must be straight-line or use
only `for` loops with a compile-time-constant upper bound.

```vani
#[deterministic_timing]
fn mix_sample(a: i64, b: i64, t: i64) -> i64 {
  // OK: straight-line arithmetic.
  return a + (b - a) * t / 100;
}
```

This attribute is targeted at audio DSP callbacks, control-loop
ISRs, and cryptographic primitives where constant-time is a
security or real-time requirement.

### `#[recursion_bound(N)]`

Caps the maximum recursion depth at N. Together with
`#[bounded_stack]`, this lets the compiler prove the total stack
usage is finite.

```vani
#[recursion_bound(32)]
#[bounded_stack(bytes=4096)]
fn tree_height(depth: i64) -> i64 {
  if depth <= 0 {
    return 0;
  }
  return 1 + tree_height(depth - 1);
}
```

### Combining annotations

Attributes stack: a function may carry any combination of the four.
The checker runs each enforcement pass independently.

```vani
#[no_heap]
#[bounded_stack(bytes=256)]
#[deterministic_timing]
fn critical_isr(x: i64) -> i64 {
  return x ^ (x >> 1);    // XOR with right-shift: deterministic, no heap, tiny frame
}
```

## When you'll reach for this layer

- **MCU firmware** with no malloc and known memory budgets.
- **FFI** where the C side hands you a pre-allocated buffer.
- **DMA** descriptors and ring buffers.
- **Custom allocators** for game engines or hot loops.

For everything else, stay in the safe subset -- the productivity
gain from the SMT verifier and affine ownership is the whole
point.

## Examples in the repo

- `examples/language/english/region_pool.vani` -- Pool/Handle
  end-to-end.
- `examples/language/english/raw_ptr_*.vani` -- raw-pointer
  variants.

## Challenge

Write a `circular_buffer` module that uses `Pool<i64>` to
back a fixed-size ring with `push` and `pop` operations.
Return a sentinel from `pop` when empty.

---

**Next**: [Sec.5 -- The `dyn` vtable layout + safety boundary ->](05_vtables.md)
