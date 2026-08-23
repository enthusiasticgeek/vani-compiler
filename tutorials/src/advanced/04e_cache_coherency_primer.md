# Advanced 4e -- Cache coherency: the MESI protocol (primer)

> **Learning goal**: understand *why* multi-core CPUs need a
> coherency protocol at all, and walk through MESI (Modified /
> Exclusive / Shared / Invalid) -- the specific state machine most
> real hardware runs -- as a small, deterministic vāṇी simulation you
> can read top to bottom. Reading order:
> [Advanced 4d -- DMA and scatter/gather](04d_dma_scatter_gather.md)
> -> here -> [Advanced 5 -- SIMD](05_simd.md).

## The problem

Every core in a multi-core CPU has its own private L1 cache. That's
what makes multiple cores fast -- each one can read and write its
working set without going out to shared memory on every access. But
it creates an obvious hazard: if core 0 and core 1 both cache the
same memory address, and core 1 writes a new value, core 0's cached
copy is now **stale**. If core 0 keeps reading its cached copy
without knowing about the write, it sees the wrong answer -- exactly
the kind of correctness bug that's invisible in code review and only
shows up as a flaky test on a busy machine.

Real hardware solves this with a **cache coherency protocol**: a set
of rules, enforced by dedicated hardware (a "snoop" bus or a
directory), that every core's cache controller follows so caching
never becomes visible as a correctness problem to software. The most
widely implemented one is **MESI**, named for its four states.

## The four states

Every cache line (in every core, independently) is in exactly one of:

| State | Meaning |
|---|---|
| **M**odified | Valid, dirty. This core has the ONLY correct copy anywhere -- not even memory is up to date. |
| **E**xclusive | Valid, clean, matches memory. No other core has it cached. |
| **S**hared | Valid, clean, matches memory. One or more OTHER cores may also have it cached. |
| **I**nvalid | No valid copy. Any read is a miss. |

The two transitions that matter are a **read miss** (this core wants
the line but its own copy is Invalid) and a **write** (this core
wants to modify the line). Read misses only ever change state
peacefully (Shared/Exclusive); writes are the disruptive one --
before a core can write, every *other* core's copy must be
invalidated, because after the write, only the writer's copy is
correct.

## The simulation

Real MESI runs as hardware watching a shared bus in real time. This
chapter's example models the same rules as a plain sequential trace
over a `Vec<i64>` of per-core state tags -- deterministic, easy to
step through, and the point (the protocol's rules) doesn't need real
concurrency to demonstrate. (Layering the same idea over real
`task`s and a `Mutex`-protected bus is a natural follow-up exercise --
see [Advanced 3](03_concurrency.md) for those primitives.)

```vani
const INVALID: i64 = 0;
const SHARED: i64 = 1;
const EXCLUSIVE: i64 = 2;
const MODIFIED: i64 = 3;

struct Bus {
  cache: Vec<i64>, // cache[core] = that core's MESI state
  mem: i64,        // shared memory's current value
}
```

### Read miss

```vani
fn handle_read(bus: mut ref Bus, core: i64) -> i64 {
  if bus.cache[core] != INVALID {
    return bus.mem; // cache hit, no bus transaction
  }
  if any_other_in_state(bus, core, MODIFIED) {
    // a sibling has the ONLY correct copy -- snoop it, both end up Shared
    let _ = downgrade_others_to_shared(bus, core);
    let _ = set(mut ref bus.cache, core, SHARED);
    return bus.mem;
  }
  if any_other_in_state(bus, core, EXCLUSIVE) || any_other_in_state(bus, core, SHARED) {
    // a sibling already has a clean copy -- both end up Shared
    let _ = downgrade_others_to_shared(bus, core);
    let _ = set(mut ref bus.cache, core, SHARED);
    return bus.mem;
  }
  // nobody else has it -- sole ownership, no need to tell anyone
  let _ = set(mut ref bus.cache, core, EXCLUSIVE);
  return bus.mem;
}
```

Three cases, in priority order: if a sibling holds **Modified**, its
value is the only truth in the system (memory itself is stale) --
snoop it, downgrade both to Shared. If a sibling holds **Exclusive**
or **Shared**, memory is already correct -- read from memory, both end
up Shared. If **nobody** has it cached, this core gets **Exclusive**:
sole ownership means a later write from this same core won't need to
invalidate anyone.

### Write

```vani
fn handle_write(bus: mut ref Bus, core: i64, value: i64) -> i64 {
  let _ = invalidate_others(bus, core);
  bus.mem = value;
  let _ = set(mut ref bus.cache, core, MODIFIED);
  return 0;
}
```

Every write invalidates every other core's copy first, unconditionally
-- there is no "Shared -> Modified" transition that skips telling the
siblings. After a write, this core alone holds Modified; everyone
else is Invalid and will take a read miss (routed through the
Modified-snoop branch above) the next time they touch the line.

## Walking the trace

The full example runs this sequence and asserts the state after
every step:

```
core 0 reads (cold)                 -> core 0: Exclusive
core 1 reads (0 has it Exclusive)   -> core 0, core 1: Shared
core 2 writes 42                    -> core 0, core 1: Invalid; core 2: Modified, mem=42
core 0 reads (must snoop core 2)    -> core 0, core 2: Shared, value read = 42
core 1 writes 99                    -> core 0, core 2: Invalid; core 1: Modified, mem=99
```

The line to read twice is the fourth step: core 0's read, after core
2's write, correctly returns **42** -- the value core 2 wrote, even
though `bus.mem` was only updated by `handle_write` and core 0 never
directly touched core 2's cache. That's the coherency invariant in
one assertion: **every reader sees the latest write**, regardless of
which core wrote it or which cores have stale copies lying around.
Get the snoop-on-Modified case wrong (e.g. read straight from `mem`
without checking for a Modified sibling first) and this exact
assertion is what would catch it -- `mem` alone isn't trustworthy
once any core holds Modified.

```vani
let v0b: i64 = handle_read(mut ref bus, 0);
assert v0b == 42; // the coherency invariant
```

Full runnable file:
[`examples/language/english/cache_coherency_mesi.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/english/cache_coherency_mesi.vani).

## Why this matters even if you never write cache-coherency code

You'll never hand-implement MESI -- it's fixed in silicon. What this
buys you is the *mental model* for a class of bug that's otherwise
mysterious: "it works on my laptop but flakes on the build server,"
where the real difference is core count and cache-line contention.
Understanding that every core's cache is privately consistent but
*globally* synchronized only through this exact protocol -- and that
the protocol is only as fast as its worst case, a cache line bouncing
Modified between cores on every access ("false sharing," when two
unrelated variables happen to share a cache line and two cores hammer
them alternately) -- explains real performance cliffs you can
actually go find and fix, usually by padding a struct so two
hot fields land in different cache lines instead of one.

## Try it yourself

```bash
vanic run examples/language/english/cache_coherency_mesi.vani
vanic run examples/language/english/cache_coherency_mesi.vani --backend=c
```

Add a fourth core and a request sequence where two cores race to
write the same line back to back -- confirm the invalidations still
leave exactly one core Modified and every prior writer's stale copy
Invalid. Try removing the Modified-snoop branch from `handle_read`
(fall through to reading `bus.mem` directly) and watch the `assert
v0b == 42` step fail -- a concrete demonstration of exactly the bug
class real coherency protocols exist to prevent.

## Summary

- Multi-core caching creates a staleness hazard; **MESI** is the
  protocol (in hardware) that makes caching invisible to software
  correctness.
- Four states: **M**odified (dirty, sole truth), **E**xclusive (clean,
  sole copy), **S**hared (clean, possibly multiple copies),
  **I**nvalid (no valid copy).
- A **read miss** peacefully joins Shared (or gets Exclusive if
  nobody else has it); a **write** unconditionally invalidates every
  other copy first.
- The coherency invariant every implementation must preserve: every
  reader sees the most recent write, regardless of which core wrote
  it.
- This simulation is a deterministic sequential trace, not real
  concurrency -- the protocol's *rules* are the point, not thread
  scheduling (which [Advanced 3](03_concurrency.md) already covers).

---

## Cross-references

- [Advanced 4d -- DMA and scatter/gather](04d_dma_scatter_gather.md) -- the other hardware-adjacent simulated-example chapter
- [Advanced 3 -- `task`/`join`/atomics/mutexes/barriers](03_concurrency.md) -- real concurrency primitives, for layering onto this simulation
- [Advanced 5 -- SIMD and NEON vectorization](05_simd.md) -- another place memory-layout decisions (contiguous vs. not) drive performance

---

**Previous**: [Sec.4d -- DMA and scatter/gather ->](04d_dma_scatter_gather.md)
**Next**: [Sec.5 -- SIMD and NEON vectorization ->](05_simd.md)
