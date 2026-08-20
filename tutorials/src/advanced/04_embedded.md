# Advanced 4 -- Embedded targets + `unsafe` + region typing

> **Learning goal**: opt into the `unsafe` operations needed
> for embedded / systems work (raw pointers, region-scoped
> arenas, manual memory) -- and understand how vāṇī keeps the
> safety boundary explicit at the source level.

> **New to this?** Read [Advanced 4a -- Embedded primer](04a_embedded_primer.md) first.

Think of `unsafe` like a "service door" in a shopping mall.
The mall's public corridors are safe, well-lit, and governed
by posted rules. The service door lets authorized staff (the
ones who KNOW the rules) step behind the scenes to do things
the public corridors can't: move large equipment, access
electrical panels, interact with the building's raw
infrastructure. `unsafe(reason = "raw pointer write -- hardware register") {}` in vāṇī works the same way:
the rest of the language's safety rules still apply everywhere
else; the `unsafe` block is a clearly-marked zone where the
compiler relaxes exactly the restrictions that embedded
hardware access requires -- raw pointer reads, memory-mapped
I/O -- while keeping the boundary visible so reviewers know
which code needs extra scrutiny.

## The `unsafe` block

**Before running any example on this page**: `unsafe(reason = "...")`
is rejected outright on a normal (hosted) build -- confirmed by
testing, this applies to every example below, not just the ones that
mention it. Set one environment variable first:

```bash
INTENT_TARGET_EMBEDDED=1 vanic run your_file.vani
```

Without it, `vanic run`/`vanic check` reject the block before even
looking inside it: `unsafe(reason = "…") is gated to embedded build
targets`. This is deliberate, not a bug to work around -- vāṇी's
design promise for ordinary (hosted) programs is "no segfault surface,
no use-after-free, no buffer overrun, full stop," and allowing
`unsafe` there would break that promise. Opting into
`INTENT_TARGET_EMBEDDED=1` is you explicitly saying "I know I'm
leaving that guarantee behind for this specific code."

<img class="manas" src="../images/mascot/manas_mascot_caution.png" title="this code needs extra care"/>

```vani
intent "Advanced 4 -- raw pointer + region-scoped arena.";

fn poke(addr: *mut i64, val: i64) -> i64 {
  unsafe(reason = "raw pointer write -- hardware register") {
    let _ = raw_store(addr, val);
  }
  return 0;
}
```

- `unsafe(reason = "raw pointer write -- hardware register") { ... }` blocks are the only place where the
  following are allowed:
  - Reading/writing through a raw pointer, via the builtins
    `raw_load(p) -> Tainted<T>` / `raw_store(p, v) -> i64` --
    **NOT** a bare `*p` / `*p = v` dereference, which isn't valid
    syntax at all (confirmed by testing: "expected expression" /
    "expected statement"). `raw_load`'s result comes back wrapped
    in `Tainted<T>` and must be unwrapped with `assert_safe(t) ->
    T` before use, forcing an explicit "I vouch for this read" at
    the call site.
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
allocators. **Pointer arithmetic (`base + offset`) is rejected
outright** -- confirmed by testing ("pointer arithmetic on raw
pointer ... MISRA C 2012 Rule 18.4 forbids +/-/shift/bitwise on
pointer types"). For offset/indexed access, wrap the pointer in a
`BoundedPtr<T>` and use bounds-checked `bptr_get`/`bptr_set`
instead:

<img class="manas" src="../images/mascot/manas_mascot_caution.png" title="this code needs extra care"/>

```vani
fn write_register(base: *mut i64, count: i64, offset: i64, value: i64) -> i64 {
  unsafe(reason = "raw pointer write -- hardware register") {
    let bp: BoundedPtr<i64> = bptr_new(base, count, count);
    let _ = bptr_set(mut ref bp, offset, value);
  }
  return 0;
}
```

The C backend lowers `raw_load`/`raw_store`/`bptr_get`/`bptr_set`
to plain `*` dereferences; the LLVM backend uses `load` / `store`
instructions.

| Builtin | Signature | Description |
|---------|-----------|-------------|
| `bptr_new` | `(p: *mut i64, len, cap: i64) -> BoundedPtr<i64>` | wrap a raw pointer with a length + capacity |
| `bptr_get` | `(ref bp, i: i64) -> Option<i64>` | bounds-checked read; `None` if `i` is out of range |
| `bptr_set` | `(mut ref bp, i, v: i64) -> bool` | bounds-checked write; `false` if `i` is out of range |
| `bptr_len` | `(ref bp) -> i64` | the length passed to `bptr_new` |

Unlike `raw_load`/`raw_store`, `bptr_get`/`bptr_set` don't need
`Tainted<i64>`/`assert_safe` -- the bounds check itself is the
safety proof, so a bad index just returns `None` / `false` instead
of reading/writing out of bounds.

### Seeing the safety net actually catch something

The whole point of `BoundedPtr` is that a mistake becomes a normal,
recoverable value instead of memory corruption. This is worth seeing
happen rather than taking on faith -- a full, runnable example
(`INTENT_TARGET_EMBEDDED=1 vanic run` required, per the note above):

```vani
intent "BoundedPtr catches an out-of-range access instead of corrupting memory.";

fn main() -> i64 {
  unsafe(reason = "scratch buffer via raw pointer for a bounds-check demo") {
    let p: *mut i64 = unsafe_alloc(3);
    let bp: BoundedPtr<i64> = bptr_new(p, 3, 3);
    let _ = bptr_set(mut ref bp, 0, 10);
    let _ = bptr_set(mut ref bp, 1, 20);
    let _ = bptr_set(mut ref bp, 2, 30);

    // index 5 is past the end of a 3-slot buffer.
    let ok: bool = bptr_set(mut ref bp, 5, 999);
    print "writing index 5 (out of range) succeeded:", ok;         // false

    let v5: i64 = option_unwrap_or(bptr_get(ref bp, 5), 0 - 1);
    print "reading index 5 (out of range, safe fallback):", v5;     // -1

    let _ = unsafe_free(p);
  }
  return 0;
}
```

In a language with bare C-style pointers, index 5 into a 3-slot
buffer just reads or writes whatever memory happens to sit past the
buffer's end -- maybe nothing visible goes wrong today, maybe it
silently corrupts an unrelated variable, maybe it crashes, and which
one happens can depend on the compiler, the optimization level, or
what else is sitting in memory nearby. Here, the same mistake
produces `false` and `-1` -- ordinary values your program can check
and handle, confirmed identical on both backends.

### Manual heap allocation: `unsafe_alloc` / `unsafe_free`

For a heap block whose size isn't known until runtime (unlike
`region`'s arena, which is scoped to one block, or `Pool<T>`, which
is meant for many same-sized slots), there's a direct `malloc`/`free`
pair:

```vani
fn scratch_buffer(n: i64) -> i64 {
  unsafe(reason = "manual heap block for a scratch computation") {
    let p: *mut i64 = unsafe_alloc(n);
    let _ = raw_store(p, 100);
    let t: Tainted<i64> = raw_load(p);
    let v: i64 = assert_safe(t);
    let _ = unsafe_free(p);
    return v;
  }
}
```

| Builtin | Signature | Description |
|---------|-----------|-------------|
| `unsafe_alloc` | `(n: i64) -> *mut i64` | allocate room for `n` `i64`s on the heap |
| `unsafe_free` | `(p: *mut i64) -> i64` | release a block returned by `unsafe_alloc` |

There's no bounds checking and no generation tracking here -- this
is the same trust level as C's `malloc`/`free`, just wrapped in
`unsafe(reason = ...)` so the deviation is labeled at the call site.
Prefer `Pool<T>` (safe, runtime-checked) or `region { ... }` (safe,
compile-time-checked) above whenever either fits; reach for
`unsafe_alloc`/`unsafe_free` only when you genuinely need a
runtime-sized block with neither a pool's generation slots nor a
region's single-scope lifetime.

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

For a real-world (not toy) use of `Pool<T>`/`Handle<T>`, see
[`examples/language/english/handle_job_queue.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/english/handle_job_queue.vani):
a job queue where jobs get cancelled mid-flight while other code
still holds their handles, and re-checking every handle safely
skips the cancelled ones (`pool_get` returning `None`) instead of
crashing or double-freeing.

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
  return 0;   // unreachable in practice, but required: the checker's
              // return-completeness pass doesn't treat a `region`
              // block's own `return` as covering the whole function
              // (confirmed by testing -- omitting this line gets
              // "function 'use_region' must return a i64").
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
- **`region_len(ref r) -> i64`** returns how many `i64` slots have
  been bump-allocated in `r` so far.
- **`region_alloc_i64(mut ref r, v) -> *mut i64`** is the same
  bump-allocation as `region_borrow_i64`, but hands back a raw
  `*mut i64` instead of an `ArenaRef<i64>` -- no compile-time escape
  check, so it's gated behind `unsafe(...)` and reads/writes go
  through `raw_load`/`raw_store` like any other raw pointer. Reach
  for `region_borrow_i64` by default; `region_alloc_i64` exists for
  the rare case where you need to hand the slot to `unsafe`-only code
  (e.g. an FFI call taking a raw pointer) that can't accept an
  `ArenaRef<i64>`.
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

<img class="manas" src="../images/mascot/manas_mascot_error.png" title="this code does not compile!"/>

```vani
fn dangler() -> ArenaRef<i64> {
  region r {
    return region_borrow_i64(mut ref r, 1);
  }
}
```

`r`'s backing storage frees when the `region` block exits --
returning an `ArenaRef` tied to it leaves a dangling reference,
so this is rejected at compile time.

<img class="manas" src="../images/mascot/manas_mascot_success.png" title="this is the correct, working version"/>

```vani
fn make_ref(r: mut ref Region) -> ArenaRef<i64> {
  return region_borrow_i64(r, 1);
}
```

Same idea, but `r` is a `mut ref Region` **parameter** instead of a
`region`-block-local -- the caller's `Region` outlives this
function's frame, so the `ArenaRef` it returns is never dangling.

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

For a real-world (not toy) use of `region`/`ArenaRef<i64>`, see
[`examples/language/english/arena_batch_parse.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/english/arena_batch_parse.vani):
parsing a batch of comma-separated sensor readings, arena-allocating
each parsed value, and freeing the entire batch in one O(1) call
when the `region` ends -- the classic per-batch/per-request/
per-frame arena pattern compilers, parsers, and servers all use.

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

### `#[bounded(N)]`

Caps the maximum recursion depth at N (`#[recursion_bound(N)]` is
NOT a real attribute -- confirmed by testing, rejected as unknown;
the compiler's own diagnostic lists `#[bounded(N)]` as the
recognized name). Together with `#[bounded_stack]`, this lets the
compiler prove the total stack usage is finite.

```vani
#[bounded(32)]
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
extern "C" fn xTaskCreate(f: fn() -> i64, name: Str,
                      stack: i64, arg: i64, prio: i64, handle: i64) -> i64;
extern "C" fn vTaskStartScheduler() -> i64;

fn core1_task() -> i64 {
    // process second half of buffer here
    return 0;
}

fn main() -> i64 {
    // core 0 processes first half inline; core 1 via FreeRTOS task
    let _ = xTaskCreate(core1_task, "c1", 512, 0, 1, 0);
    let _ = vTaskStartScheduler();
    return 0;
}
```

(Two syntax fixes over an earlier version of this page, both
confirmed by testing: `extern` alone doesn't parse -- it's always
`extern "C" fn` with an explicit ABI string, same as every other FFI
declaration; and `name: ref i8` + `ref "c1" as i8` at the call site
doesn't type-check at all (`ref` can only borrow a named variable,
never a literal) -- a C string parameter is just `name: Str`, and a
plain string literal passes directly, no `ref`/cast needed.)

For single-core targets, just use a plain `while` loop — no workaround
needed, and the verifier + SMT checks still apply.

## Challenge

Write a `circular_buffer` module that uses `Pool<i64>` to
back a fixed-size ring with `push` and `pop` operations.
Return a sentinel from `pop` when empty.

---

**Previous**: [Sec.4b -- Cross-compilation and bare-metal targets primer ->](04b_cross_compile_primer.md)
**Next**: [Sec.4c -- Attributes reference ->](04c_attributes_reference.md)
