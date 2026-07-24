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

<img class="manas" src="../images/mascot/manas_mascot_caution.png" title="this code needs extra care"/>

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
- The compiler proves *every other* operation safe. The
  `unsafe` keyword is a request to suspend specific safety
  rules in a small, identifiable scope. `Pool<T>`/`Handle<T>`
  and `Region`/`ArenaRef<T>` (below) are NOT in this list --
  both are safe-by-construction APIs usable without an `unsafe`
  block, just with different safety proofs (runtime vs.
  compile-time).

## Raw pointers: `*const T` and `*mut T`

For interop with hardware MMIO, FFI buffers, or hand-written
allocators:

<img class="manas" src="../images/mascot/manas_mascot_caution.png" title="this code needs extra care"/>

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

## Two arena mechanisms: `Pool<T>` (v1, runtime-checked) vs `Region` + `ArenaRef<T>` (v2, compile-time-checked)

`unsafe.md`'s embedded plan ships arena-style allocation in two
separate layers, with different cost/flexibility tradeoffs. They
are **not** the same feature, and one is *not* built inside the
other -- a common mix-up. Pick based on what you need:

| | `Pool<T>` / `Handle<T>` (Layer 2) | `Region` / `ArenaRef<T>` (Layer 5) |
|---|---|---|
| Safety proof | **Runtime**: each access checks a generation counter | **Compile-time**: escape analysis proves no dangling ref |
| Cost | A branch + counter compare per `pool_get` | Zero -- `aref_load`/`aref_store` compile to a bare deref |
| A `Handle`/`ArenaRef` can... | outlive the `Pool` (stale access returns `Option::None`, not UB) | **not** outlive its `Region` (rejected at compile time) |
| Needs `unsafe(...)` / embedded gate? | No -- works on hosted targets | No for local use; **yes** (`INTENT_TARGET_EMBEDDED=1`) the moment `ArenaRef<T>` appears in a `fn` parameter or return type |
| Use when | You need a reference that can legitimately outlive its allocator (caches, graphs, long-lived tables) | You need zero-cost slots scoped to one function/block (hot loops, ISRs, DSP buffers) |

### `Pool<T>` + `Handle<T>` -- generational handles

```vani
intent "Pool<i64> / Handle<i64> — generational handles.";

fn unwrap_or(o: Option<i64>, def: i64) -> i64 {
  return match o {
    Option.Some(v) then v,
    Option.None then def,
  };
}

fn use_pool() -> i64 {
  let p: Pool<i64> = pool_new();
  let h1: Handle<i64> = pool_alloc(mut ref p, 7);
  let h2: Handle<i64> = pool_alloc(mut ref p, 42);
  let sum: i64 = unwrap_or(pool_get(ref p, h1), 0)
               + unwrap_or(pool_get(ref p, h2), 0);
  print "sum =", sum;
  let _ = pool_free(mut ref p, h1);   // stale h1 now reads back None
  return sum;
}
```

- `pool_new()` takes **no arguments** -- capacity grows as you
  `pool_alloc` (heap-backed, not a fixed-size stack array).
- `pool_alloc(mut ref p, v)` returns a `Handle<i64>` -- a `(slot,
  generation)` pair, `Copy`, not a pointer.
- `pool_get(ref p, h) -> Option<i64>` is the only way to read: a
  handle whose slot was freed and reused returns `Option::None`
  instead of reading garbage -- that's the runtime check paying for
  itself. There is no `handle_read` / `handle_write` builtin.
- `pool_free(mut ref p, h)` frees one slot. `Pool`'s own scope-exit
  drop frees everything still live when `p` goes out of scope.
- No `region` block involved -- `Pool<T>` is its own affine owner,
  usable directly inside any function, on hosted targets, with no
  `unsafe` wrapper.

Full runnable version (double-free + stale-handle behavior
included): [`examples/language/english/pool.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/english/pool.vani).

### `region { ... }` + `Region` / `ArenaRef<T>` -- compile-time-checked bump arena

```vani
fn use_region() -> i64 {
  region arena {
    let a: ArenaRef<i64> = region_borrow_i64(mut ref arena, 10);
    let b: ArenaRef<i64> = region_borrow_i64(mut ref arena, 32);
    let _ = aref_store(a, aref_load(a) + aref_load(b));
    return aref_load(a);   // 42
  }
  // `arena`'s entire backing storage frees in one call here.
}
```

- **`region <name> { ... }`** -- note the name is required
  (`region { ... }` with no name is a parse error). It desugars to
  `let <name>: Region = region_new();` followed by the block body,
  so `<name>`'s scope-exit drop frees the whole arena in one O(1)
  call.
- **`region_borrow_i64(mut ref r, v)`** bump-allocates one `i64`
  slot in `r`, writes `v`, and returns an `ArenaRef<i64>` -- read
  with `aref_load`, write with `aref_store`. Both lower to a plain
  pointer deref on both backends; there's no bounds check, no
  canary, no generation check, because the compiler's escape
  analysis is the entire safety proof.
- **The escape check, verified directly**: returning an `ArenaRef`
  tied to a `Region` declared inside the same function --
  `fn dangler() -> ArenaRef<i64> { region r { return
  region_borrow_i64(mut ref r, 1); } }` -- is rejected at compile
  time (`ArenaRef ... cannot be returned -- the source storage dies
  on function exit`), whether the escape happens directly on
  `return` or via an intermediate `let` binding. A `Region` received
  as a `mut ref Region` **parameter**, by contrast, outlives the
  callee's frame, so an `ArenaRef` derived from it can freely flow
  back out.
- **The embedded gate applies only to signatures, not local use.**
  `ArenaRef<T>` (and `Region`, though that one's unrestricted) can
  be used freely as a local variable's type inside one function
  body on a hosted target -- the example above needs no `unsafe`
  block and no env var. The gate only fires the moment `ArenaRef<T>`
  appears as a `fn` **parameter or return type** (`raw pointer type
  ArenaRef<i64> not permitted in function signature on hosted
  targets`) -- set `INTENT_TARGET_EMBEDDED=1` to write a helper `fn`
  that takes or returns one directly. Until then, keep `ArenaRef`
  values inside a single function (as in the example) or wrap the
  work in a `Region`-parameter function instead.

Full runnable version: [`examples/language/english/region_arena.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/english/region_arena.vani).

## What's safe vs unsafe in this layer

| Layer | Operation | Safety |
|---|---|---|
| 1.1 | `*const T` / `*mut T` deref | unsafe |
| 2 | `Pool<T>` / `Handle<T>` | safe (runtime-checked) |
| 2 | C extern fn call | safe (caller's responsibility) |
| 3 | `unsafe(reason = "raw pointer write -- hardware register") { ... }` block | author's responsibility |
| 4.1 | `-fstack-protector-strong` | safe (opt-in build flag) |
| 5 | `region { ... }` / `Region` / `ArenaRef<T>` (local use) | safe (compile-time-checked, zero cost) |

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

- [`examples/language/english/pool.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/english/pool.vani)
  -- `Pool<T>` / `Handle<T>` end-to-end, including stale-handle and
  double-free behavior.
- [`examples/language/english/region_arena.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/english/region_arena.vani)
  -- `region { ... }` / `Region` / `ArenaRef<T>` end-to-end.

## `parallel for` is not available on bare-metal

`parallel for … reduce` lowers to `pthread_create` / `CreateThread`.
Those symbols do not exist on `arm-none-eabi` or `thumbv7em-none-eabihf` —
the linker will fail with an undefined-reference error.

**Options on bare-metal:**

| Approach | When to use |
|---|---|
| Sequential loop | Single-core MCU (Cortex-M0/M0+), latency is fine |
| FreeRTOS `xTaskCreate` via FFI | Dual-core MCU (RP2040), need true parallelism |
| DMA offload | Peripheral handles the accumulation (ADC, DSP core) |

Example — FreeRTOS task via FFI on an RP2040:

```vani
extern fn xTaskCreate(f: fn() -> i64, name: ref i8,
                      stack: i64, arg: i64, prio: i64, handle: i64) -> i64;
extern fn vTaskStartScheduler() -> i64;

fn core1_task() -> i64 {
    // process second half of buffer here
    return 0;
}

fn main() -> i64 {
    // core 0 processes first half inline; core 1 via FreeRTOS task
    let _ = xTaskCreate(core1_task, ref "c1" as i8, 512, 0, 1, 0);
    let _ = vTaskStartScheduler();
    return 0;
}
```

For single-core targets, just use a plain `while` loop — no workaround
needed, and the verifier + SMT checks still apply.

## Challenge

Write a `circular_buffer` module that uses `Pool<i64>` to
back a fixed-size ring with `push` and `pop` operations.
Return a sentinel from `pop` when empty.

---

**Previous**: [Sec.4b -- Cross-compilation and bare-metal targets primer ->](04b_cross_compile_primer.md)
**Next**: [Sec.4c -- Attributes reference ->](04c_attributes_reference.md)
