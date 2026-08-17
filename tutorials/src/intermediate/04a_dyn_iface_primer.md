# Intermediate 4a -- What's a `dyn Iface`? (intuition primer)

> **Learning goal**: build a mental model of "dynamic dispatch"
> using everyday analogies, BEFORE chapter 5 shows you the
> code. If you've never heard the term "interface" + "trait
> object" + "virtual table" in your life, start here. If you
> already know what a Rust trait object is, skip to
> [Intermediate 5](05_dyn.md).

This chapter is pure intuition — the couple of code snippets below
are explicitly marked `HYPOTHETICAL`, previewing what the next
chapters let you actually write.

## The problem: a zoo of shapes

Imagine you're writing a drawing program. You want to compute
the area of various shapes -- circles, squares, triangles. Each
shape has a different formula:

- Circle: pi x r^2
- Square: side x side
- Triangle: 1/2 x base x height

Imagine a `Shape` **interface** (vāṇī's word for "the set of
methods any shape supports") with one method, `area` — the next
two chapters ([4b](04b_interfaces_primer.md),
[Intermediate 4](04_generics_iface.md)) show you how to actually
write one. For now, just picture each shape type (`Circle`,
`Square`, `Triangle`) having its own `implement Shape for X` block
that fills in `area` for that type.

Now you want a list of shapes: `[circle1, square1, triangle1,
circle2]`. The catch: each item is a DIFFERENT TYPE. A `Vec<Circle>`
holds only circles. A `Vec<Square>` holds only squares. There's
no `Vec<Anything-That-Implements-Shape>`.

...or is there?

## The detective and the case file

Think about how a detective handles a stack of case files. Each
file describes a different crime: a robbery, a fraud, a
forgery. The detective doesn't need a separate filing cabinet
per crime-type. Instead, every case file follows a **standard
format**:

- A label saying what kind of case it is (`"robbery"`).
- A pointer to the specific report (a paper that knows ONLY
  about robberies).
- Behind the report, a list of *standard actions* the
  detective can take on ANY case: `read_summary()`,
  `mark_solved()`, `archive()`.

When the detective opens a file, they don't need to know in
advance whether it's a robbery or a fraud. They look at the
label, follow the pointer to the specific report, and call
the standard action they need. The pointer leads to a
different report depending on the case type -- but the **set of
actions** is identical, so the detective can do their job
without knowing the specific type.

This is the trick: every case is wrapped in the same
*standard packaging* with the *standard set of actions*, even
though the inside (the specific report) is different.

That's a `dyn Iface` in vāṇī.

## What `dyn Iface` actually is

A `dyn Iface` is a **standard packaging** for any type that
implements `Iface`. The packaging is just **two pointers** in
memory:

```
+--------------------------+
|  pointer to a "vtable"   | <- the standard set of actions
+--------------------------+
|  pointer to the value    | <- the specific report
+--------------------------+
```

(`vtable` is just a fancy name for "the list of standard
actions" -- a table of pointers to the methods for whichever
type is inside.)

When you call `shape.area()` on a `dyn Shape`:

1. The compiler reads the *first pointer* (vtable).
2. It looks up the `area` slot in that table.
3. It calls whatever function is stored there, passing the
   *second pointer* (the actual Circle/Square/Triangle) as
   the `self` arg.

The `dyn Shape` doesn't need to know which type is inside. It
just knows "vtable has area at slot 0, the value-pointer is
the right type for that area function". The vtable pointer is
how the compiler routes the call to the right code.

## Why is this useful?

Three reasons, each something you'll hit immediately:

### 1. Heterogeneous collections

`Vec<dyn Shape>` is one collection holding mixed Circles,
Squares, Triangles. Without `dyn`, you'd need separate vecs per
type -- a pain to manage and inflexible.

```vani
// HYPOTHETICAL -- what dyn lets you write
let shapes: Vec<dyn Shape> = vec(c as dyn Shape, s as dyn Shape);
for shape in ref shapes {
  print "area:", shape.area();
}
```

The `for` loop calls `area` on each. Each call follows the
right vtable; the same source code handles all three shape
types.

### 2. Functions that take "any shape"

```vani
fn print_area(shape: ref dyn Shape) -> i64 {
  print "area:", shape.area();
  return 0;
}
```

This function works on any type that implements `Shape`. The
caller picks the specific type; the function only cares about
the **interface**.

### 3. Plugin-style architectures

A library can ship an interface `Logger` and let users provide
their own implementations (`FileLogger`, `NetworkLogger`,
`StdoutLogger`). The library code holds a `dyn Logger`; the
user picks which implementation to plug in.

## What's the cost?

Three small costs you pay for the flexibility:

1. **Two pointers per value** instead of one. A `dyn Shape` is
   16 bytes (8 bytes vtable + 8 bytes value pointer); a plain
   pointer is 8.
2. **One extra hop per method call**. The compiler can't
   inline through a `dyn` call because it doesn't know the
   concrete type at compile time. (The cost is small: it's
   one indirect call, like calling a function pointer.)
3. **No compile-time inlining**. With a non-`dyn` interface,
   the compiler often generates specialized code per concrete
   type -- fast but bigger binary. With `dyn`, one function
   serves all types -- small but slightly slower per call.

For most code, the costs are invisible. For tight inner loops
that get called millions of times, you'd use non-`dyn`
interfaces (chapter 4) for the inlining win.

## When NOT to use `dyn Iface`

A common mistake is reaching for `dyn` for everything. The rule
of thumb:

- If you KNOW the concrete type at the call site -> use the
  plain type or non-`dyn` interface (chapter 4 generics).
- If you have a HETEROGENEOUS COLLECTION or a generic plugin
  point -> `dyn Iface` is the right choice.
- If your `Vec<dyn Foo>` only ever has one variant inside ->
  refactor to `Vec<TheActualType>` -- you're paying for
  flexibility you're not using.

## `Box<dyn Iface>` -- owning a `dyn`

One more variant you'll see: `Box<dyn Iface>` puts the
detective's case file *on the heap*, with the dyn packaging
owning that heap allocation. Two reasons to want this:

1. **Struct fields**: storing a heterogeneous shape inside a
   struct (`struct Drawer { rend: Box<dyn Renderer> }`). The
   plain `dyn Renderer` couldn't be stored -- the value would
   need an owner. `Box<dyn Renderer>` owns the heap-allocated
   value AND wears the dyn packaging.
2. **Lifetime**: when you want the shape to outlive a
   particular function -- passing it back as a return value,
   storing it for later. `Box` provides the heap-storage
   discipline `dyn` lacks on its own.

## A summary you can carry

- A `dyn Iface` is a **two-pointer package**: vtable + value.
- The vtable is the **standard set of actions** for whichever
  type is inside -- looked up at runtime, called via one
  indirect call.
- Use `dyn` for heterogeneous collections + plugin-style
  architectures.
- Don't use `dyn` when you know the concrete type -- non-`dyn`
  interfaces inline better.
- `Box<dyn Iface>` owns the heap value AND carries the dyn
  packaging. The shape lives wherever the `Box` lives.

That's the intuition. The next chapter ([Intermediate 5](05_dyn.md))
shows you the actual syntax.

## Cross-reference

- [Beginner 6a -- pointers and references primer](../beginner/06a_pointers_refs_primer.md)
  -- the address-of-value mental model `dyn` builds on.
- [Beginner 6b -- heap vs stack primer](../beginner/06b_heap_vs_stack_primer.md)
  -- explains where `Box<dyn Iface>`'s heap data lives.
- [Intermediate 4 -- Generics and interfaces](04_generics_iface.md)
  -- non-`dyn` (static) dispatch for the cases where you know
  the type.
- [Intermediate 5 -- Dynamic dispatch](05_dyn.md) -- the actual
  code with `Vec<dyn Shape>` etc.


---

**Previous**: [Sec.3 -- Affine ownership: ref / mut ref ->](03_affine.md)
**Next**: [Sec.4b -- Interfaces and static dispatch primer ->](04b_interfaces_primer.md)

