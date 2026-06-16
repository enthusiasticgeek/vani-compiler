# Advanced 5 — The `dyn` vtable layout + safety boundary

> **Learning goal**: understand how `dyn Iface` is represented
> at runtime, when the compiler can prove a call is safe, and
> where the C-backend code-gen lives.

You already know that `dyn Iface` lets you call the right method
regardless of which concrete type is inside the envelope. This
chapter shows you the machinery that makes that work: the
**vtable** (virtual dispatch table). Think of a vtable as a
printed directory card stapled inside each envelope. The card
lists the methods the interface promises ("area → page 7,
draw → page 12") with each entry pointing to the concrete
function for the specific type inside. When the runtime calls
`d.area()`, it flips to the right entry in the card and jumps
there — one extra pointer dereference, but no branching or
type-checking. This chapter is for readers who want to know
EXACTLY what memory is laid out and why.

## The fat-pointer layout

A `dyn Iface` value is a 16-byte struct on a 64-bit target:

```c
typedef struct {
  const intent_vtbl_Iface* vtable;  // 8 bytes — table of method pointers
  void* data;                       // 8 bytes — pointer to the concrete value
} intent_dyn_Iface;
```

- `vtable` points at a per-(T, Iface) static vtable struct.
  Each implementation produces one vtable; the implementations
  are picked at coercion time, not at the call site.
- `data` points at the concrete value. For let-bound coercions
  the data lives on the stack; for `Vec<dyn Iface>` elements
  the storage strategy depends on whether each element is
  a copy-by-value primitive shape or a heap-promoted slot.

## The vtable struct

For each `interface Iface { fn m1(...); fn m2(...); }`, the
compiler emits one vtable struct shape:

```c
typedef struct {
  R1 (*m1)(void* data, T1 arg, ...);
  R2 (*m2)(void* data, T2 arg, ...);
} intent_vtbl_Iface;
```

For each `implement Iface for T`:

```c
static const intent_vtbl_Iface intent_vtbl_Iface_T = {
  .m1 = &intent_fn_T_m1_thunk,
  .m2 = &intent_fn_T_m2_thunk,
};
```

Each `_thunk` casts `data` back to `T*` and dispatches to the
user-written method body.

## Dispatch

A call `dyn_iface.m1(arg)` lowers to:

```c
dyn_iface.vtable->m1(dyn_iface.data, arg)
```

One indirection through the vtable pointer. There's no
type-tag check at runtime — the type system has already proved
the vtable is the right one for the data behind it.

## The safety boundary

The static type system ensures:

1. A `T → dyn Iface` coercion only happens when
   `implement Iface for T` is in scope.
2. The data pointer stays alive for at least as long as the
   `dyn Iface` value (let-bound source, or heap-promoted slot
   in a `Vec<dyn Iface>`).
3. No two methods can see different concrete data layouts
   under the same vtable pointer (per-(T, Iface) vtables
   prevent this).

If those invariants hold, dispatch is memory-safe with the
same guarantees as the static-dispatch generic in
[Intermediate §4](../intermediate/04_generics_iface.md). The
runtime cost is one extra load — the vtable indirection.

## What this means for `Vec<dyn Iface>`

The Phase 1.2 fix ([L8 in v1_limitations.md](https://github.com/anthropics/claude-code/blob/main/docs/v1_limitations.md))
solved a real bug: `Vec<dyn Drawable>` and `Vec<dyn Loggable>`
stored as struct fields used to share one bundle typedef.
Per-Iface naming (`intent_vec_dyn_Drawable` /
`intent_vec_dyn_Loggable`) gives each one a distinct typedef
that holds the right fat-pointer type.

## Source-of-truth pointers

- **C codegen of the vtable struct**: see `emit_dyn_iface_typedefs`
  in `src/backend_c.rs`.
- **C codegen of the dispatch site**: search for `DynDispatch`
  in `src/backend_c.rs`.
- **Coercion checker**: `check_dyn_coerce` in `src/checker.rs`.
- **End-to-end examples**: the 22 GoF design patterns at
  `examples/language/english/design_patterns/` — `observer.vani`,
  `strategy.vani`, `factory_method.vani` all use `dyn Iface`
  in different shapes.

## When to peek under the hood

Most user code doesn't need to know the vtable layout. Reach
for this knowledge when:

- You're debugging a "method not found" diagnostic — usually
  the coercion source isn't a let-bound variable.
- You're profiling and the vtable indirection is on a hot
  path — switch to the static-dispatch generic.
- You're writing FFI that exposes a `dyn Iface` value to a C
  caller — you need to know the layout to write the C-side
  struct.

## Challenge

Read `examples/language/english/dyn_dispatch.vani`. Run
`vanic emit … --backend=c` and find the emitted vtable
struct for the `Drawable` interface. Trace one method call
from the `dyn_dispatch` source line through the C output to
the underlying `intent_fn_Circle_area_thunk` body.

---

**Next**: [§6 — SMT trace debugging →](06_smt_debug.md)
