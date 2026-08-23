# Advanced 4d -- Data movement: scatter/gather and simulated DMA

> **Learning goal**: understand the two hardware-adjacent data-movement
> patterns that show up constantly in systems code -- non-contiguous
> memory access (gather/scatter) and asynchronous bulk copy (DMA) --
> and see that both are expressible in 100% safe vāṇी, with a real
> `unsafe` MMIO sketch alongside for what triggering the *actual*
> hardware looks like. Reading order: [Advanced 4c -- Attributes
> reference](04c_attributes_reference.md) -> here ->
> [Advanced 4e -- Cache coherency primer](04e_cache_coherency_primer.md).

## Why these belong together

DMA (Direct Memory Access) hardware and gather/scatter CPU
instructions solve the same underlying problem from two different
angles: moving data around *without* the CPU doing one
load-then-store per element. A DMA controller copies a contiguous
block from A to B while the CPU does something else entirely. A
gather/scatter instruction loads (or stores) several *non-contiguous*
elements in one instruction, picked out by an index vector, instead
of one scalar load per element.

Both have the same hazard shape too: an out-of-range address (DMA)
or an out-of-range index (gather/scatter) doesn't raise a friendly
exception on most real hardware -- it reads or corrupts whatever
memory happens to be at that address. This chapter shows both
patterns built entirely out of bounds-checked `Vec` operations, so
that hazard becomes a catchable `assert` failure instead.

## Gather and scatter, without unsafe code

A **gather** reads `xs[idx[0]], xs[idx[1]], ...` into a fresh, dense
output `Vec` -- the software shape of a SIMD gather instruction (SoA
re-layout, sparse-vector densify, permutation, table lookup). A
**scatter** is the mirror image: write `vals[0], vals[1], ...` to
`xs[idx[0]], xs[idx[1]], ...`.

```vani
fn gather(xs: ref Vec<i64>, idx: ref Vec<i64>) -> Vec<i64> {
  let out: Vec<i64> = vec();
  let i: u64 = 0;
  let n: u64 = len(idx);
  while i < n {
    let src: u64 = idx[i] as u64;
    out = push(out, xs[src]);
    i = i + 1;
  }
  return out;
}

fn scatter(xs: mut ref Vec<i64>, idx: ref Vec<i64>, vals: ref Vec<i64>) -> i64 {
  let i: u64 = 0;
  let n: u64 = len(idx);
  while i < n {
    let dst: u64 = idx[i] as u64;
    let _ = set(xs, dst, vals[i]);
    i = i + 1;
  }
  return 0;
}
```

Nothing here is `unsafe`. Every `xs[src]` and every `set(xs, dst,
...)` goes through vāṇी's ordinary bounds check -- an `idx` value
that's out of range (a corrupted index table, an off-by-one in
whatever produced `idx`) becomes a caught runtime assertion, not a
wild write into whatever `Vec` happens to sit next in memory. On real
SIMD hardware, a gather/scatter instruction handed a bad index either
faults (if the target address is unmapped) or silently reads/writes
neighboring memory (if it happens to be mapped to something else) --
there's no middle ground the hardware enforces for you.

### Scatter's real hazard: duplicate destinations

If `idx` has a repeated destination, the *last* write to that slot
wins and every earlier one is silently discarded:

```vani
let overwrite_idx: Vec<i64> = vec(0, 0, 1);
let overwrite_vals: Vec<i64> = vec(100, 200, 300);
// after scatter: slot 0 holds 200 (100 was clobbered), slot 1 holds 300
```

That's exactly why `parallel for` refuses to let you scatter into a
plain `Vec` by a data-dependent index at all (see [Advanced 2 --
`parallel for` + reductions](02_parallel.md)) -- across threads,
"last write wins" becomes "whichever thread happens to finish last
wins," a genuine data race the checker won't let you write. Run
sequentially it's still legal, just worth knowing. The safe
alternative when duplicates are possible: accumulate instead of
overwrite, the same idea `reduce ... with +;` uses for the parallel
case --

```vani
fn scatter_accumulate(xs: mut ref Vec<i64>, idx: ref Vec<i64>, vals: ref Vec<i64>) -> i64 {
  let i: u64 = 0;
  let n: u64 = len(idx);
  while i < n {
    let dst: u64 = idx[i] as u64;
    let _ = set(xs, dst, xs[dst] + vals[i]);
    i = i + 1;
  }
  return 0;
}
```

Full runnable file, both directions plus the duplicate-destination
demo:
[`examples/language/english/scatter_gather.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/english/scatter_gather.vani).

## A simulated DMA descriptor ring buffer

Real DMA hardware is programmed with **descriptors** -- small records
naming a source, a destination, and a length -- queued into a
fixed-capacity **ring buffer** the controller drains one at a time.
The CPU submits descriptors at the `tail`; the controller (or, in a
polling design, the CPU itself on a later check) drains from the
`head`; both indices wrap around the ring's fixed capacity.

```vani
struct DmaDescriptor { src: i64, dst: i64, n: i64 }

struct DmaRing {
  descs: Vec<DmaDescriptor>,
  head: i64,   // next slot the controller will drain
  tail: i64,   // next slot a submit() will fill
  count: i64,  // how many descriptors are currently queued
}

fn dma_submit(ring: mut ref DmaRing, src: i64, dst: i64, n: i64) -> bool {
  if ring.count == RING_CAPACITY {
    return false; // ring full -- the CPU must check this, same as
                   // polling a real "ring full" status bit
  }
  let d: DmaDescriptor = DmaDescriptor { src: src, dst: dst, n: n };
  let _ = set(mut ref ring.descs, ring.tail, d);
  ring.tail = (ring.tail + 1) % RING_CAPACITY;
  ring.count = ring.count + 1;
  return true;
}

fn dma_controller_step(ring: mut ref DmaRing, mem: mut ref Vec<i64>) -> bool {
  if ring.count == 0 {
    return false; // nothing queued
  }
  let d: DmaDescriptor = ring.descs[ring.head];
  let i: i64 = 0;
  while i < d.n {
    let v: i64 = mem[d.src + i];
    let _ = set(mem, d.dst + i, v);
    i = i + 1;
  }
  ring.head = (ring.head + 1) % RING_CAPACITY;
  ring.count = ring.count - 1;
  return true;
}
```

The copy loop inside `dma_controller_step` is where a *bad*
descriptor (a `src`/`dst`/`n` combination that overruns `mem`) gets
caught -- exactly like real DMA hardware would fault, or (worse, on a
part with no MPU) silently corrupt whatever memory happens to sit
past the buffer's end.

### Reading `mut ref ring.descs`, not `ring.descs`

Notice the submit function writes `set(mut ref ring.descs, ...)`, not
the bare `set(ring.descs, ...)` you might reach for first. This
matters, and getting it wrong used to be a real compiler bug (BUG-223,
fixed 2026-08-23): `set()` has two overloads --

```
set(xs: Vec<T>, i, v) -> Vec<T>          // consuming: takes ownership, returns a new Vec
set(xs: mut ref Vec<T>, i, v) -> i64     // in-place: mutates through the reference
```

`ring.descs` accessed bare, inside a function that only has `ring:
mut ref DmaRing` (not an owned `DmaRing`), looks to the dispatcher
like a plain owned `Vec<DmaDescriptor>` -- so it used to silently pick
the *consuming* overload, which doesn't own what it was handed
(`ring` is only borrowed) and (before the fix) corrupted memory when
the returned "new" Vec got discarded. The compiler now rejects the
bare form outright with `cannot move 'ring.descs' -- 'ring' is only
borrowed here`, so this specific mistake is a compile error, not a
runtime surprise, on any vāṇी version from that fix onward. Write
`mut ref` on the field access itself whenever you're mutating a field
of something you only hold by reference.

Full runnable file, including submitting past the ring's raw
capacity (draining makes room, `tail` wraps) and verifying every
transfer landed correctly:
[`examples/language/english/dma_ring_buffer.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/english/dma_ring_buffer.vani).

## Streaming DMA: double buffering

A single descriptor transfer is a "fire once" DMA. **Streaming** DMA
-- continuous ADC sampling, audio in/out, a sensor feed -- uses two
buffers instead: while the DMA controller fills buffer A from the
peripheral, the CPU consumes buffer B (already full from the previous
round). When both finish, the roles swap. Neither side ever touches
the buffer the other side currently owns, so there's no torn read of
a half-filled buffer.

```vani
struct StreamState {
  buf_a: Vec<i64>,
  buf_b: Vec<i64>,
  a_is_filling: bool, // true: DMA fills A while CPU drains B
  running_sum: i64,
  next_sample: i64,
}

fn dma_fill_active(s: mut ref StreamState) -> i64 { /* fills whichever buffer is active */ }
fn cpu_drain_inactive(s: mut ref StreamState) -> i64 { /* drains the OTHER buffer */ }
fn stream_swap(s: mut ref StreamState) -> i64 { s.a_is_filling = !s.a_is_filling; return 0; }
```

Every round: fill the active buffer, drain the inactive one, swap.
The two operations touch disjoint memory by construction, so their
order relative to each other within a round never matters -- the
same "no torn read" property real double-buffered DMA gives you in
hardware, reproduced here with two `Vec<i64>` buffers and a role flag
instead of two hardware FIFOs. The full example verifies it end to
end with a checksum: every sample the simulated peripheral produces
gets consumed exactly once, no drops, no double-counts --

```vani
let total_samples: i64 = NUM_CHUNKS * CHUNK_SIZE;
let expected: i64 = total_samples * (total_samples + 1) / 2;
assert s.running_sum == expected;
```

Full runnable file:
[`examples/language/english/dma_streaming.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/english/dma_streaming.vani).
This chapter's simulation runs both "sides" sequentially in one
thread to keep the double-buffering invariant itself front and
center; wiring the same two buffers to a real producer task and a
real consumer task (a `Mutex`-protected handoff, or two `task`s
synchronized with a `Barrier` per round) is a natural extension --
see [Advanced 3 -- `task`/`join`/atomics/mutexes/barriers](03_concurrency.md)
for those primitives.

## The real-hardware version: MMIO register programming

Every example above is a *simulation* -- safe, portable, runs on this
machine, and teaches the pattern without needing real DMA silicon.
Triggering an *actual* DMA controller means writing to its control
registers via MMIO (memory-mapped I/O), the same `mmio_write_u32` /
`mmio_read_u32` builtins [Sec.4 -- Embedded targets](04_embedded.md)
introduced for GPIO/UART:

```vani
const DMA_BASE: i64 = 0x40026010; // example: STM32F4 DMA2 Stream0
const DMA_SRC_OFFSET: i64 = 0x00;
const DMA_DST_OFFSET: i64 = 0x04;
const DMA_LEN_OFFSET: i64 = 0x08;
const DMA_CTRL_OFFSET: i64 = 0x0C;
const DMA_CTRL_ENABLE_BIT: u32 = 0x1;
const DMA_CTRL_DONE_BIT: u32 = 0x2;

fn dma_start(src_addr: i64, dst_addr: i64, len_words: i64) -> i64 {
  let _ = mmio_write_u32(DMA_BASE + DMA_SRC_OFFSET, src_addr as u32);
  let _ = mmio_write_u32(DMA_BASE + DMA_DST_OFFSET, dst_addr as u32);
  let _ = mmio_write_u32(DMA_BASE + DMA_LEN_OFFSET, len_words as u32);
  let _ = mmio_write_u32(DMA_BASE + DMA_CTRL_OFFSET, DMA_CTRL_ENABLE_BIT);
  return 0;
}

fn dma_wait_done() -> i64 {
  let done: bool = false;
  while !done {
    let status: u32 = mmio_read_u32(DMA_BASE + DMA_CTRL_OFFSET);
    done = ((status as i64) & (DMA_CTRL_DONE_BIT as i64)) != 0;
  }
  return 0;
}
```

Configure source, destination, and length; set the enable bit; poll
status until the hardware reports completion (a real interrupt-driven
design would sleep instead of busy-poll, waking on the controller's
completion IRQ -- not modeled here, to keep the register-level shape
visible).

**A sharp edge worth knowing about, not just for this file**:
`mmio_read_u32`/`mmio_write_u32` are *not* gated to embedded build
targets the way `volatile_read`/`volatile_write` and `unsafe(reason =
"...")` blocks are -- `vanic check` on this file succeeds on any host,
with no `INTENT_TARGET_EMBEDDED=1` needed. That's not a safety net;
it's an asymmetry in v1's current gating (both families are one
`unsafe.md` "Layer," implemented at different points in the compiler's
history). `vanic check` succeeding does **not** mean `vanic run` is
safe -- this file's registers don't back real memory on an x86-64/
AArch64 dev machine, so running it hosted segfaults immediately,
exactly like [Sec.4's `bare_metal.vani`](04_embedded.md) does for the
same reason. That's expected, not a bug. The intended flow is a real
target:

```bash
vanic build examples/embedded/dma_mmio_trigger.vani \
  --target=thumbv7em-none-eabihf --no-std -o dma_demo.elf
qemu-system-arm -M netduinoplus2 -kernel dma_demo.elf -nographic
```

Full file (adjust `DMA_BASE` and the register layout for your real
target's datasheet before flashing anywhere):
[`examples/embedded/dma_mmio_trigger.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/embedded/dma_mmio_trigger.vani).

## Try it yourself

```bash
vanic run examples/language/english/scatter_gather.vani
vanic run examples/language/english/dma_ring_buffer.vani
vanic run examples/language/english/dma_streaming.vani --backend=c
vanic check examples/embedded/dma_mmio_trigger.vani   # type-checks hosted
vanic build examples/embedded/dma_mmio_trigger.vani -o /tmp/dma_demo && echo "native build OK"
```

Try widening the ring buffer's `RING_CAPACITY` in
`dma_ring_buffer.vani`, or feeding `scatter()` an index that's
deliberately out of range -- confirm you get a clean assertion
failure, not a crash with no explanation.

## Summary

- **Gather/scatter** are non-contiguous `Vec` reads/writes driven by
  an index array -- fully expressible with bounds-checked indexing,
  no `unsafe` needed. Watch for duplicate scatter destinations
  ("last write wins"); accumulate instead of overwrite when that
  matters, the same idea `parallel for`'s `reduce` uses.
- **DMA** is descriptors queued into a fixed-capacity ring, drained
  by a controller that performs the actual copy -- simulate it with a
  `Vec<Descriptor>` ring and a plain copy loop; every hazard (full
  ring, bad descriptor) becomes a caught assertion.
- **Streaming/double-buffered DMA** swaps which of two buffers is
  "filling" vs. "draining" each round -- the two sides never touch
  the same memory at the same time, by construction.
- Mutating a field of something you only hold by reference (`set(mut
  ref t.field, ...)`) needs the `mut ref` written explicitly on the
  field access -- the compiler now enforces this at compile time
  (BUG-223) rather than letting the mistake corrupt memory at
  runtime.
- **Real hardware DMA** is MMIO register programming --
  `mmio_write_u32`/`mmio_read_u32` against the controller's control
  block, no different in kind from the GPIO/UART examples in
  [Sec.4](04_embedded.md), and subject to the same "don't run this
  hosted" caveat.

---

## Cross-references

- [Advanced 2 -- `parallel for` + reductions + race-freedom](02_parallel.md) -- why scatter-by-data-dependent-index is rejected inside `parallel for`
- [Advanced 3 -- `task`/`join`/atomics/mutexes/barriers](03_concurrency.md) -- real-concurrency version of the double-buffering pattern
- [Advanced 4 -- Embedded targets + `unsafe`](04_embedded.md) -- MMIO builtins, `#[no_heap]`, bare-metal basics
- [Advanced 5 -- SIMD and NEON vectorization](05_simd.md) -- the hardware instructions gather/scatter model in software here

---

**Previous**: [Sec.4c -- Attributes reference ->](04c_attributes_reference.md)
**Next**: [Sec.4e -- Cache coherency primer ->](04e_cache_coherency_primer.md)
