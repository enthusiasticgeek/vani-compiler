# Intermediate 3a — `Box<T>` and RAII (intuition primer)

> **Learning goal**: understand what `Box<T>` is, why you'd
> reach for it, and the "RAII" pattern it embodies. Reading
> order: [Beginner 6a/6b/6c primers](../beginner/06a_pointers_refs_primer.md)
> → [Intermediate 3 affine ownership](03_affine.md) → here.

This chapter has **no compiler code**. Pure intuition.

## The problem: where do I put this?

You know from the heap-vs-stack primer that big or dynamic
values live on the heap. You've used `Vec<T>` — a heap-living
sequence — and the compiler handles its allocation +
cleanup automatically.

But sometimes you want a SINGLE value on the heap, not a
sequence. Why?

### Case 1: the struct field that holds different concrete types

```
struct Drawer {
  shape: dyn Shape,   // ← problem: which size?
}
```

`Circle` is 8 bytes, `Square` is 8, but `Triangle` might be 24
(three coordinates). The compiler can't reserve "the right
amount" — it doesn't know which type will be inside.

Solution: store a `dyn Shape` *handle* in `Drawer`, with the
actual shape value on the heap. The handle is fixed-size (16
bytes — vtable + data pointer), so `Drawer` has a known size.

```
struct Drawer {
  shape: Box<dyn Shape>,   // ← 16 bytes regardless of inner type
}
```

`Box<dyn Shape>` IS that fat-pointer-plus-heap-storage handle.

### Case 2: a recursive data structure

```
struct Node {
  value: i64,
  next: Node,    // ← problem: how big is Node?
}
```

`Node` would have to contain another `Node` of the same size,
which contains another, infinitely. The compiler rejects this.

Solution: the `next` field is a `Box<Node>`. A `Box<Node>` is
8 bytes (just a pointer). The actual next-node lives on the
heap. The recursion ends naturally (or via `Option<Box<Node>>`
for the last element).

### Case 3: large struct in a hot loop

A 4 KB struct on the stack means every recursive call or
returns-by-value copies 4 KB. Putting it in a `Box<T>` means
each call copies 8 bytes (the pointer); the struct stays put
on the heap.

## What `Box<T>` actually is

A `Box<T>` is exactly two things:

1. **A pointer** to a heap allocation that holds a `T`.
2. **Ownership** of that allocation — when the `Box` goes out
   of scope, the heap allocation is freed.

In memory:

```
The binding "b" (on the stack, 8 bytes):
┌─────────────────────────────────┐
│  pointer to heap location       │
└────────────────┬────────────────┘
                 ↓
The heap location (sizeof(T) bytes):
┌─────────────────────────────────┐
│  the actual T value             │
└─────────────────────────────────┘
```

`box(value)` is the operation that:
1. Asks the heap allocator for `sizeof(T)` bytes.
2. Moves `value` into that heap slot.
3. Returns a `Box<T>` (the pointer) wrapping it.

When the `Box<T>` binding goes out of scope, the compiler:
1. Runs the inner T's destructor (if it has one — e.g. if T is
   itself a Vec).
2. Frees the heap allocation.

You don't write any of this. The compiler does it from the
ownership rule.

## What "RAII" means

**RAII** = **Resource Acquisition Is Initialization** — a name
from C++ but the idea generalizes. The rule:

> A value's CLEANUP is tied to its SCOPE.

`Box<T>` is one example: when the binding's scope ends, the
heap is freed. Other examples in vāṇี:

- `Vec<T>` — when the binding's scope ends, the buffer is
  freed.
- `OwnedStr` — when the binding's scope ends, the bytes are
  freed.
- `Guard<T>` from `mutex_lock` — when the guard's scope ends,
  the mutex is unlocked.
- `Task` from `task t { ... }` — joined / consumed at scope
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

— all of those happen via the scope-exit auto-cleanup of the
owning binding.

## The two paths a `Box<T>` value takes

Same as any owning binding (from the ownership primer):

### Move

```rust
let b: Box<Foo> = box(Foo { x: 42 });
let c: Box<Foo> = b;   // moves; b is now invalid
```

`c` now owns the heap allocation. `b` is gone (compile error
to use it).

### Borrow

```rust
let b: Box<Foo> = box(Foo { x: 42 });
let n: i64 = read_foo(ref b);   // borrows; b still owns
```

`read_foo` gets temporary access to the Box. `b` is still
valid afterward.

## `Box<T>` vs `Vec<T>` — what's the difference?

Both put data on the heap. The difference:

- `Vec<T>` holds **many** Ts in a contiguous buffer. You can
  add/remove. The buffer can resize.
- `Box<T>` holds **exactly one** T. Fixed size at creation.

You'd use `Box<T>`:
- For a single struct that needs to be on the heap (recursion,
  size, dyn-coercion).
- When you need a **stable pointer** — the T's address doesn't
  change as the program runs (a Vec's buffer can move when it
  resizes).

You'd use `Vec<T>`:
- For sequences of any kind.

You'd use both:
- `Vec<Box<T>>` — a sequence of heap-allocated Ts. Each
  element is its own heap allocation; the Vec stores pointers
  to them. Useful when individual elements are expensive to
  move OR when you need pointer-stability per element.

## When NOT to use `Box<T>`

If your value is small and Copy (e.g. `Box<i64>`), you almost
never want this — just use `i64`. The Box adds a heap
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
[`examples/language/english/box_dyn_iface.vani`](https://github.com/.../box_dyn_iface.vani)
and [`box_recursive_drop.vani`](https://github.com/.../box_recursive_drop.vani)
are the worked examples.

## Cross-reference

- [Beginner 6a — pointers/references primer](../beginner/06a_pointers_refs_primer.md)
- [Beginner 6b — heap/stack primer](../beginner/06b_heap_vs_stack_primer.md)
- [Beginner 6c — ownership/move primer](../beginner/06c_ownership_primer.md)
- [Intermediate 3 — Affine ownership](03_affine.md)
- [Intermediate 4a — `dyn Iface` primer](04a_dyn_iface_primer.md) —
  `Box<dyn Iface>` combines this chapter's heap allocation
  with that chapter's dynamic dispatch.
- [Intermediate 5 — Dynamic dispatch](05_dyn.md) — the actual
  code.
