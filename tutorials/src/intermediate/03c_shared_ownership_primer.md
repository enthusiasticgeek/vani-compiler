# Intermediate 3c -- Shared ownership without `Rc`/`Arc`

> **Learning goal**: understand vāṇी's intentional omission of
> reference-counted pointers (`Rc<T>` / `Arc<T>` / `Weak<T>`)
> AND the five idiomatic alternatives. Reading order: at
> minimum [Beginner 6c ownership](../beginner/06c_ownership_primer.md)
> + [Intermediate 3a Box+RAII](03a_box_raii_primer.md) +
> [Intermediate 3b affine deeper](03b_affine_deeper_primer.md).

This chapter has **no compiler code**. Pure intuition.

## What vāṇी does NOT have

Rust has `Rc<T>` (single-threaded reference-counted pointer)
and `Arc<T>` (atomic, thread-safe reference-counted pointer).
Both let MULTIPLE owners share one heap allocation; the value
is freed when the LAST owner drops.

vāṇी has **neither**.

It also doesn't have `Weak<T>` (a non-owning pointer that lets
you peek at an Rc-managed value without keeping it alive),
because that's only meaningful in the presence of Rc.

This is a deliberate design choice, not an oversight.

## Why omit them?

Three reasons, in increasing order of importance:

### 1. Cycles leak

`Rc<T>` doesn't free a cycle. If two Rc-managed nodes point
at each other, neither's count ever reaches zero; both leak.
The Rust workaround is to use `Weak<T>` for the back-edges of
graphs -- a chore the user has to remember to do correctly.

A language without Rc avoids this entire class of bugs.

### 2. The cost is real

Every clone of an `Rc<T>` bumps a refcount; every drop
decrements. `Arc<T>` does it with atomic operations
(memory-bus-locking instructions) -- much slower than non-atomic.

For a language whose pitch is "as fast as C, safer than C",
adding silent reference-counting overhead on values that LOOK
like normal pointers undercuts the story.

### 3. It hides the question of "who's responsible?"

Rc/Arc say "many owners; whichever drops last cleans up."
That's flexible, but it also means the cleanup time is
*emergent* -- you can't tell from the source when something
will run. Debugging a "this resource is freed too early" or
"this resource is held too long" bug under Rc is harder than
under single-owner because the *ownership graph* is dynamic.

vāṇी's single-owner-plus-borrows model makes cleanup time
predictable: when the owning binding's scope ends. The trade
is more thinking up front about who owns what -- but it's the
SAME thinking the Rc programmer should be doing anyway, just
forced to be explicit.

## How to share without Rc -- five patterns

vāṇी's borrow system (`ref T` / `mut ref T`) handles MOST
cases that would use Rc in Rust. When that's not enough, five
specific patterns cover the rest.

### Pattern 1: just borrow

The most common pattern: when multiple parts of your code need
to *read* the same data, give each a `ref T`. Many shared
borrows can coexist freely.

```vani
fn use_data(...) -> i64 {
  let xs: Vec<i64> = vec(1, 2, 3, 4);
  let a: i64 = sum(ref xs);
  let b: i64 = max(ref xs);
  let c: i64 = first(ref xs);
  return a + b + c;
}
```

Three "shared owners" in spirit -- all reading `xs`. The
compiler accepts unlimited shared borrows of the same value
because they can't conflict (read-only). No refcount needed:
the borrows end naturally when each call returns, and `xs`
remains the single owner.

This pattern handles ~80% of cases that would reach for Rc.

### Pattern 2: handles (indices), not pointers

The "Entity-Component System" (ECS) pattern from game
development. Instead of pointers to objects, you store objects
in a Vec/Pool and use *indices* as handles.

```vani
struct Entity { id: u32, generation: u32 }
struct World {
  entities: Pool<Entity>,
  positions: Vec<Vector3>,
  velocities: Vec<Vector3>,
}

fn move_entity(world: mut ref World, e: Entity, dt: f64) -> i64 {
  // Look up by index, not pointer
  let pos: Vector3 = world.positions[e.id as u64];
  let vel: Vector3 = world.velocities[e.id as u64];
  world.positions[e.id as u64] = pos + vel * dt;
  return 0;
}
```

The "shared ownership" semantics are simulated: many parts of
the program hold `Entity` handles; the `World` owns the actual
data. Lookups are O(1) array indexing -- faster than chasing
pointers AND cache-friendly because adjacent entities' data
is contiguous.

For graphs, this manifests as `Vec<Node>` with each Node
holding `Vec<u32>` of neighbor indices -- no `Box`, no `Rc`, no
cycles-as-pointers (cycles become valid index-graphs, freed
together when the World drops).

### Pattern 3: arena / region allocation

Allocate many values from a shared arena; free the whole arena
at once. The values "share ownership" of the arena's lifetime
-- they all live as long as the arena does, then die together.

```vani
region scratch {
  let a: ArenaRef<i64> = region_borrow_i64(mut ref scratch, 10);
  let b: ArenaRef<i64> = region_borrow_i64(mut ref scratch, 32);
  // a and b live as long as `scratch` does
  print "sum:", aref_load(a) + aref_load(b);
}  // scratch drops; a + b's storage is freed together
```

Inside the region, you can pass `ArenaRef<i64>` handles freely --
they're all valid until the region ends. No per-value
refcounting; one wholesale cleanup at scope exit. This is
how compilers, parsers, and game-frame allocators often
manage many temporary objects.

vāṇी's region typing ([Advanced 4 -- Embedded](../advanced/04_embedded.md))
enforces at compile time that a region-allocated `ArenaRef`
doesn't escape the region's scope. The shipped slot type is
`i64` -- the `Foo { ... }` struct case shown as the pattern's
*idea* above isn't allocable in a `region` yet; for that, reach
for [Pattern 2](#pattern-2-handles-indices-not-pointers)
(`World` + `Vec<Node>` indices) instead until a generic
`ArenaRef<T>` ships.

### Pattern 4: channels for moving ownership between threads

If you'd reach for `Arc<T>` to share data across threads,
ask first: do you need shared READ access (chapter 5 mutex
pattern) or do you actually need to MOVE the value?

For moves, channels are cleaner:

```vani
let ch: Channel<Vec<i64>, 8> = channel_new();

task producer {
  let data: Vec<i64> = expensive_compute();
  channel_send(mut ref ch, data);    // data moves OUT of producer
}

task consumer {
  let data: Vec<i64> = channel_recv(mut ref ch);
                                      // data moves INTO consumer
  process(data);
}
```

At any moment, exactly one task owns the Vec. There's no
shared mutable state -- no race possible. The channel is the
synchronization primitive AND the ownership-transfer
primitive.

### Pattern 5: `Mutex<T>` for actual shared mutable state

When you genuinely need multiple threads to MUTATE the same
data, use `Mutex<T>`:

```vani
struct SharedCounter { value: i64 }

let counter: Mutex<SharedCounter> = mutex_new(SharedCounter { value: 0 });

task incrementer {
  let g: Guard<SharedCounter> = mutex_lock(ref counter);
  g.value = g.value + 1;
  // g drops at end of scope -> mutex unlocks
}
```

The Mutex is the "shared ownership" mechanism. The `Guard<T>`
RAII handle ensures the lock releases when the scope ends --
no manual unlock. Mutex<T> takes care of cross-thread
synchronization without per-value reference counting.

If you need to hold the mutex across an `await` (chapter
[01a async](../advanced/01a_async_primer.md)), a different
shape applies -- the mutex is part of the Task state.

## What if I really, REALLY need Rc semantics?

A small set of patterns are genuinely awkward without Rc:

- **A graph with cycles where edges aren't easily encoded as
  indices** (e.g., a DOM tree with parent pointers).
- **Cache-like data where many independent consumers can hold
  the value indefinitely** (some objects live forever from
  startup; some are released when the last user finishes).
- **Plugin systems where you don't control the dropping
  order** (a plugin registers a callback; the callback holds
  a value; the plugin unregisters; the callback might still
  be in flight).

For these, vāṇी's current answer is:

1. **Refactor toward handles + Vec** wherever possible. Most
   "graphs with cycles" become cleaner as `Vec<Node>` with
   index-based edges.
2. **Use `unsafe(reason = "...")`** for the unavoidable cases
   -- explicitly mark the section where the type system isn't
   tracking ownership, and explain in the reason string what
   discipline you're maintaining manually.
3. **A v2 `Rc<T>` is conceivable** -- but the design choice
   today is to not have it. If real-world experience shows
   it's needed for a class of programs that no other pattern
   addresses cleanly, the language could add it. So far,
   nothing has surfaced that genuinely needs it.

## Weak pointers -- gone with Rc

`Weak<T>` exists in Rust ONLY because `Rc<T>` exists -- it's
the way to break Rc cycles. With no Rc, there's no need for
Weak.

If you need "look at this value without keeping it alive,"
that's just `ref T` -- a shared borrow. The compiler tracks
borrow scope to ensure the source outlives the borrower.

## A summary you can carry

- vāṇी has **no `Rc`, `Arc`, or `Weak`**. By design.
- Reasons: cycle leaks under Rc, real overhead per
  clone/drop, hidden ownership graph dynamics.
- Five idiomatic alternatives:
  1. **Just borrow** -- many `ref T` co-exist; handles 80%
     of would-be Rc cases.
  2. **Handles, not pointers** -- store data in a Vec/Pool,
     pass indices (ECS / graph-as-index pattern).
  3. **Arena / region** -- shared lifetime by construction,
     wholesale cleanup.
  4. **Channels** -- move ownership between threads
     explicitly.
  5. **`Mutex<T>`** -- synchronized shared mutable state for
     cases that actually need it.
- For the rare genuine Rc-required cases: refactor toward
  handles, or use `unsafe` with a documented discipline.

vāṇी optimizes for *predictable* memory behavior -- you can
always tell when something will be freed. Rc trades that
predictability for flexibility; vāṇी declines that trade as a
default. The patterns above let you achieve almost all of
Rc's expressiveness without the cost.

## Cross-reference

- [Beginner 6c -- Ownership primer](../beginner/06c_ownership_primer.md)
  -- the foundation
- [Intermediate 3a -- Box and RAII primer](03a_box_raii_primer.md)
  -- single-owner heap allocation
- [Intermediate 3b -- Affine deeper primer](03b_affine_deeper_primer.md)
  -- borrow scopes; many-shared-XOR-one-mutable rule
- [Intermediate 3d -- Cyclic references primer](03d_cyclic_references_primer.md)
  -- worked side-by-side examples (parent<->child tree,
  doubly-linked list, observer pattern) of the three
  canonical Rc+Weak shapes translated to vāṇी's
  index-into-Vec pattern
- [Advanced 2a -- Parallelism + race-freedom primer](../advanced/02a_parallelism_primer.md)
  -- channels + mutex as the cross-thread sharing primitives
- [Advanced 4a -- Embedded primer](../advanced/04a_embedded_primer.md)
  -- region typing details


---

**Previous**: [Sec.3b -- Affine ownership deeper pass primer ->](03b_affine_deeper_primer.md)
**Next**: [Sec.3d -- Cyclic references primer ->](03d_cyclic_references_primer.md)

