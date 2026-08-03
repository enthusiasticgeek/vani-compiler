# Intermediate 3a -- `Box<T>` and RAII (intuition primer)

> **Learning goal**: understand what `Box<T>` is, why you'd
> reach for it, and the "RAII" pattern it embodies. Reading
> order: [Beginner 6a/6b/6c primers](../beginner/06a_pointers_refs_primer.md)
> -> [Intermediate 3 affine ownership](03_affine.md) -> here.

This chapter has **no compiler code**. Pure intuition.

## The storage-unit key

You're renting an apartment (your function's stack frame) that's
too small for a couch. A self-storage facility down the street
solves this: they hold the couch in a unit sized for it, and hand
you a single small **key**. You carry the key in your pocket --
tiny, fits anywhere -- while the actual couch sits somewhere else
entirely.

- **A struct field that needs an unknown-size guest.** Imagine a
  form that has a blank labeled "attach one photo of your item" --
  but items come in wildly different sizes (a stamp vs. a couch).
  The form can't reserve "enough space" for either, so instead the
  blank holds a storage-unit key. Whatever's actually stored can be
  any size; the key itself is always the same small size. That's
  `Box<dyn Shape>` in a struct field.
- **A recursive shape.** A storage unit that, itself, contains a
  full-size second storage unit, which contains a third, forever,
  is impossible to build. But a storage unit that contains a *key*
  to the next unit is completely fine -- keys don't nest, they just
  point onward. That's how a linked chain of `Node`s works: each
  node holds a key to the next node's storage unit, not the next
  node itself.
- **A big item you don't want to lug around.** If your couch has to
  travel with you everywhere (passed into a function, returned back
  out), hauling the whole couch each time is expensive. Hauling just
  the key is cheap -- one small object, same size no matter how big
  the couch is.

A `Box<T>` **is** that key: a small, fixed-size handle on your
stack that points at a `T`-sized unit on the heap.

Now here's the important part of the facility's policy: **you never
have to remember to go empty out your unit.** Your rental is tied to
your membership card. The instant your membership lapses (the key's
scope ends), the facility automatically clears the unit and reissues
the space. You didn't call anyone, you didn't fill out a
cancellation form -- it just happens because the rental was *tied to*
the membership's lifetime, not tracked separately.

That automatic "cleanup happens exactly when the owning thing's
lifetime ends" policy has a name in programming: **RAII**. `Box<T>`
is one example of it in vāṇी; you'll meet several more below.

## The problem: where do I put this?

You know from the heap-vs-stack primer that big or dynamic
values live on the heap. You've used `Vec<T>` -- a heap-living
sequence -- and the compiler handles its allocation +
cleanup automatically.

But sometimes you want a SINGLE value on the heap, not a
sequence. Why?

### Case 1: the struct field that holds different concrete types

```
struct Drawer {
  shape: dyn Shape,   // <- problem: which size?
}
```

`Circle` is 8 bytes, `Square` is 8, but `Triangle` might be 24
(three coordinates). The compiler can't reserve "the right
amount" -- it doesn't know which type will be inside.

Solution: store a `dyn Shape` *handle* in `Drawer`, with the
actual shape value on the heap. The handle is fixed-size (16
bytes -- vtable + data pointer), so `Drawer` has a known size.

```
struct Drawer {
  shape: Box<dyn Shape>,   // <- 16 bytes regardless of inner type
}
```

`Box<dyn Shape>` IS that fat-pointer-plus-heap-storage handle.

### Case 2: a recursive data structure

```
struct Node {
  value: i64,
  next: Node,    // <- problem: how big is Node?
}
```

`Node` would have to contain another `Node` of the same size,
which contains another, infinitely. The compiler rejects this.

Solution: the `next` field is a `Box<Node>`. A `Box<Node>` is
8 bytes (just a pointer). The actual next-node lives on the
heap. The recursion ends naturally (or via `Option<Box<Node>>`
for the last element).

<img class="manas" src="../images/mascot/manas_mascot_caution.png" title="this is a real v1 boundary, not a style choice"/>

**A real v1 boundary, confirmed by testing (2026-08-03)**: this
tutorial only ever BUILDS a `Box<Node>` chain -- it never reads a
field back through one. That's not a stylistic choice: field access
does NOT go through a `Box<T>` in v1. Given `n: Box<Node>`, writing
`n.value` is rejected outright ("field access on non-struct type
Box<Node>"). This is unrelated to ownership or ergonomics -- the
checker's field-access resolution only unwraps `Ref`/`RefMut`
(`ref_to_point.x` reads `(*ref_to_point).x`), not `Box`. There is
currently no supported way to read a field through a bare `Box<T>`
binding. If you need to walk a `Box<Node>` chain, plan around this
until it's fixed.

### Case 3: large struct in a hot loop

A 4 KB struct on the stack means every recursive call or
returns-by-value copies 4 KB. Putting it in a `Box<T>` means
each call copies 8 bytes (the pointer); the struct stays put
on the heap.

## What `Box<T>` actually is

A `Box<T>` is exactly two things:

1. **A pointer** to a heap allocation that holds a `T`.
2. **Ownership** of that allocation -- when the `Box` goes out
   of scope, the heap allocation is freed.

In memory:

```
The binding "b" (on the stack, 8 bytes):
+---------------------------------+
|  pointer to heap location       |
+----------------+----------------+
                 v
The heap location (sizeof(T) bytes):
+---------------------------------+
|  the actual T value             |
+---------------------------------+
```

`box(value)` is the operation that:
1. Asks the heap allocator for `sizeof(T)` bytes.
2. Moves `value` into that heap slot.
3. Returns a `Box<T>` (the pointer) wrapping it.

When the `Box<T>` binding goes out of scope, the compiler:
1. Runs the inner T's destructor (if it has one -- e.g. if T is
   itself a Vec).
2. Frees the heap allocation.

You don't write any of this. The compiler does it from the
ownership rule.

## What "RAII" means

**RAII** = **Resource Acquisition Is Initialization** -- a name
from C++ but the idea generalizes. The rule:

> A value's CLEANUP is tied to its SCOPE.

`Box<T>` is one example: when the binding's scope ends, the
heap is freed. Other examples in vāṇี:

- `Vec<T>` -- when the binding's scope ends, the buffer is
  freed.
- `OwnedStr` -- when the binding's scope ends, the bytes are
  freed.
- `Guard<T>` from `mutex_lock` -- when the guard's scope ends,
  the mutex is unlocked.
- `Task` from `task t { ... }` -- joined / consumed at scope
  end.

The reason RAII matters: you NEVER have to remember to
"release" a resource manually. The compiler emits the
release operation at the right point automatically, based on
the scope structure of your source code.

This is why vāṇी programs don't have:
- `free()` calls
- `unlock()` calls
- `close()` calls (in v1; file handles are queued)
- `dispose()` calls

-- all of those happen via the scope-exit auto-cleanup of the
owning binding.

## The two paths a `Box<T>` value takes

Same as any owning binding (from the ownership primer):

### Move

```vani
let b: Box<Foo> = box(Foo { x: 42 });
let c: Box<Foo> = b;   // moves; b is now invalid
```

`c` now owns the heap allocation. `b` is gone (compile error
to use it).

### Borrow

```vani
let b: Box<Foo> = box(Foo { x: 42 });
let n: i64 = read_foo(ref b);   // borrows; b still owns
```

`read_foo` gets temporary access to the Box. `b` is still
valid afterward.

## `Box<T>` vs `Vec<T>` -- what's the difference?

Both put data on the heap. The difference:

- `Vec<T>` holds **many** Ts in a contiguous buffer. You can
  add/remove. The buffer can resize.
- `Box<T>` holds **exactly one** T. Fixed size at creation.

You'd use `Box<T>`:
- For a single struct that needs to be on the heap (recursion,
  size, dyn-coercion).
- When you need a **stable pointer** -- the T's address doesn't
  change as the program runs (a Vec's buffer can move when it
  resizes).

You'd use `Vec<T>`:
- For sequences of any kind.

You'd use both:
- `Vec<Box<T>>` -- a sequence of heap-allocated Ts. Each
  element is its own heap allocation; the Vec stores pointers
  to them. Useful when individual elements are expensive to
  move OR when you need pointer-stability per element.

## Variations -- what Box wraps in real programs

You've seen `Box<Foo>` for a single user-struct. The shape
works for far more -- and the recursive-drop wiring in vāṇी
makes the affine combinations work too.

### `Box<Vec<T>>` -- heap-pointer to a heap-allocated Vec

```vani
struct Bag { contents: Box<Vec<i64>> }

let v: Vec<i64> = vec(10, 20, 30);
let b: Box<Vec<i64>> = box(v);
let bag: Bag = Bag { contents: box(vec(7, 8, 9)) };
```

The Box holds a *pointer to the Vec struct*. The Vec struct
(3-word handle) lives on the heap; the Vec's *data array* is
on another heap allocation. Two-level heap structure:

```
Box's location -> Vec struct -> data buffer (the actual elements)
```

When `b` goes out of scope, the compiler drops in order:
1. `intent_vec_int64_t__free(*b)` -- free the data buffer.
2. `free(b)` -- free the Vec struct allocation.

This is why vāṇी's recursive-drop wiring matters: BOTH layers
need cleanup. The compiler emits both calls automatically.

### `Box<OwnedStr>` -- heap-pointer to a heap char buffer

```vani
let s: OwnedStr = "hello" + "!";
let b: Box<OwnedStr> = box(s);
```

`OwnedStr` is itself a heap pointer (to char bytes). Wrapping
it in `Box` adds another level. The drop chain:
1. `free(*b)` -- free the char buffer.
2. `free(b)` -- free the slot holding the char pointer.

Less common in practice than `Box<Vec<T>>`. Useful when you
need a stable heap address for a single string.

### `Box<dyn Iface>` -- heap-allocated value behind an interface

```vani
struct Drawer { rend: Box<dyn Renderer> }

let circle: Circle = Circle { r: 7 };
let d: Drawer = Drawer { rend: box(circle) };
```

This was the load-bearing case. `Box<dyn Renderer>` is the
16-byte fat pointer struct that owns its heap concrete. The
heap holds the actual Circle; the fat pointer's `.data` slot
points at it; `.vtable` slot picks the right `render` method.

When `d` drops:
1. The dyn fat pointer's vtable picks the concrete's
   destructor (if any).
2. `free(.data)` -- the heap-allocated Circle.

Phase 1 + 3 + 3b of the L2 lift (described earlier in the
session ledger) wired this end-to-end on both backends.

### Two things `Box<T>` does NOT support yet

Two shapes that sound plausible given everything above -- but
both are explicitly rejected in v1, so it's worth knowing the
boundary rather than guessing:

- **`Box` of a tuple.** `box((42, "answer" + ""))` for a
  `Box<(i64, OwnedStr)>` fails: `box() v1 supports Copy + sized
  element types (primitives, Copy structs), dyn Iface, Vec<T>,
  and OwnedStr; got (i64, OwnedStr)`. Reach for a small named
  struct instead of a tuple if you need to box a multi-component
  value.
- **`Box<Box<T>>` -- pointer to a pointer.** `box(inner)` where
  `inner: Box<i64>` fails with the same diagnostic, which
  explicitly calls this out: "Other owning inner types
  (`Box<Box<T>>`, `Box<HashMap<…>>`, etc.) remain a follow-up."
  So a second Box layer isn't available yet, even though it would
  be a natural way to add indirection in some recursive type
  definitions.

### `Vec<Box<T>>` -- sequence of heap-allocated Ts

```vani
let drawers: Vec<Box<dyn Renderer>> = vec(
  box(circle as dyn Renderer),
  box(square as dyn Renderer),
);
```

The Vec stores N Box-fat-pointers (16 bytes each for dyn boxes).
Each element's actual data is its own heap allocation. The
Vec's buffer is contiguous; the element data points elsewhere.

This is the canonical "heterogeneous collection of trait
objects" pattern -- when you don't know how many shapes you'll
add or which types, store `Box<dyn Iface>` in a Vec.

### `Option<Box<T>>` -- explicit nullable pointer

```vani
struct Node {
  value: i64,
  next: Option<Box<Node>>,
}
```

A linked-list node: either points at the next node (`Some(box(...))`)
or terminates the list (`None`). The `Option` makes the "null
pointer" case explicit at the type level -- you have to match
on it before dereferencing. No silent nullptr crashes.

This is THE canonical recursive data-structure shape in vāṇी /
Rust.

## When NOT to use `Box<T>`

If your value is small and Copy (e.g. `Box<i64>`), you almost
never want this -- just use `i64`. The Box adds a heap
allocation per value with no benefit.

If your struct is small (< 64 bytes) and the recursion / size
isn't an issue, leave it on the stack. The heap allocation +
indirection costs are real.

`Box<T>` is for: I need this thing on the heap, and I need
ownership of that heap allocation tied to my scope.

## A summary you can carry

- A `Box<T>` is a **single-T heap allocation with ownership**.
  Eight bytes on the stack (pointer); `sizeof(T)` on the heap.
- Reach for it when: recursive struct shape, holding a dyn
  type in a struct field, or a single large value you want on
  the heap.
- It follows the same ownership rules as any other binding:
  move on assignment, borrow via `ref`, auto-free at scope
  exit.
- **RAII** is the broader pattern: cleanup tied to scope. Box
  is one example; Vec, OwnedStr, Guard, Task all use the same
  shape.

That's `Box<T>`. The intermediate-track chapter on dyn
dispatch ([5](05_dyn.md)) uses `Box<dyn Iface>` in real code;
[`examples/language/english/box_dyn_iface.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/english/box_dyn_iface.vani)
and [`box_recursive_drop.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/english/box_recursive_drop.vani)
are the worked examples.

## Cross-reference

- [Beginner 6a -- pointers/references primer](../beginner/06a_pointers_refs_primer.md)
- [Beginner 6b -- heap/stack primer](../beginner/06b_heap_vs_stack_primer.md)
- [Beginner 6c -- ownership/move primer](../beginner/06c_ownership_primer.md)
- [Intermediate 3 -- Affine ownership](03_affine.md)
- [Intermediate 4a -- `dyn Iface` primer](04a_dyn_iface_primer.md) --
  `Box<dyn Iface>` combines this chapter's heap allocation
  with that chapter's dynamic dispatch.
- [Intermediate 5 -- Dynamic dispatch](05_dyn.md) -- the actual
  code.


---

**Previous**: [Sec.2b -- Match enhancements ->](02b_match_enhancements.md)
**Next**: [Sec.3b -- Affine ownership deeper pass primer ->](03b_affine_deeper_primer.md)

