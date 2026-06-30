# Intermediate 5 -- Dynamic dispatch: `dyn Iface` + `Vec<dyn Iface>`

> **Learning goal**: use `dyn Iface` to hold a value of any
> type that implements the interface, build a heterogeneous
> `Vec<dyn Iface>`, and call methods through it.

> **New to this?** Read [Intermediate 4a -- What's a `dyn Iface`? primer](04a_dyn_iface_primer.md)
> for the analogy first. This chapter is the code surface.

A `dyn Shape` is a sealed envelope: the outside always shows the
same label ("this is a Shape") regardless of what's inside
(Circle, Square, Triangle). Every envelope comes with a tiny
directory (the *vtable*) telling the code exactly which method
to call for whichever concrete type is inside. You can put mixed
envelopes in a single `Vec<dyn Shape>` and call `area()` on each
one -- the right formula is picked at runtime by following the
directory. This is *dynamic dispatch*.

## The program

```vani
intent "Intermediate 5 worked example -- dyn Iface + heterogeneous Vec.";

struct Circle { r: i64 }
struct Square { side: i64 }

interface Shape {
  fn area(self: Circle) -> i64;
}

implement Shape for Circle {
  fn area(self: Circle) -> i64 { return self.r * self.r; }
}

implement Shape for Square {
  fn area(self: Square) -> i64 { return self.side * self.side; }
}

// `dyn Shape` is a fat pointer { vtable, data }. Same call
// site, different concrete impl picked at runtime.
fn area_of(d: dyn Shape) -> i64 {
  return d.area();
}

fn main() -> i64 {
  let c: Circle = Circle { r: 3 };
  let s: Square = Square { side: 5 };

  // Implicit T -> dyn Iface coercion at the call site.
  print "circle =", area_of(c);
  print "square =", area_of(s);

  // Heterogeneous Vec -- the canonical dyn dispatch use case.
  let mixed: Vec<dyn Shape> = vec(Circle { r: 2 }, Square { side: 4 });
  let total: i64 = 0;
  for sh in mixed {
    total = total + sh.area();
  }
  print "total =", total;
  return 0;
}
```

## Compile + run

```bash
vanic run ~/int5.vani
```

Output:

```
circle = 9
square = 25
total = 20
```

## Why it works that way

- **`dyn Iface` is a fat pointer**: a 16-byte struct
  `{ *const VTable, *const Data }`. The vtable is statically
  generated per `(T, Iface)` pair; the data pointer points at
  the heap or stack slot holding the concrete value.
- **Implicit `T -> dyn Iface` coercion** kicks in at let-bindings,
  function arguments, struct fields, and `Vec<dyn Iface>`
  elements whenever the compiler can see an `implement Iface
  for T` in scope. The coercion source must be a `let`-bound
  variable so the data pointer has a stable address.
- **Method dispatch** through `dyn` reads the vtable's slot for
  the method's declaration-order index, then calls the function
  pointer there. No virtual-table lookup overhead beyond the
  single indirection.
- **Heterogeneous `Vec<dyn Shape>`** is the killer feature.
  Every element can be a different concrete type as long as it
  implements `Shape`. The `for sh in mixed` loop iterates
  through the fat pointers; `sh.area()` picks the right impl
  for each element's vtable.

## When to reach for `dyn`

- You're collecting different concrete types into one container.
- The interface has many implementers and code-size matters
  more than per-call performance.
- You want a stable ABI boundary (one fn body, no monomorphized
  copies).

If none of those apply, the static-dispatch generic in [Sec.4](04_generics_iface.md)
is faster and simpler -- no vtable indirection.

## v1 limitations to keep in mind

- **`Vec<dyn Iface>` as a struct field across multiple Iface
  types** was a bug ([L8 in v1_limitations.md](https://github.com/enthusiasticgeek/vani-compiler/blob/main/docs/v1_limitations.md))
  that shipped in Phase 1.2 -- make sure your compiler is up to
  date if you hit confusing C-codegen errors when storing two
  different `Vec<dyn ...>` fields.
- **Methods in the interface declare a concrete receiver
  type** (`self: Circle`) -- that's the v1 static-dispatch
  convention. Look at the design-pattern examples (e.g.
  `examples/language/english/design_patterns/behavioral/observer.vani`)
  for production-shape dyn-dispatch code.

## Challenge

Add a third struct `Triangle { base: i64, height: i64 }` with
its own `Shape` impl returning `base * height / 2`. Put a
`Circle`, `Square`, and `Triangle` into one `Vec<dyn Shape>`
and print each one's area in a loop.

---

**Next**: [Sec.6 -- Closures and iterator combinators ->](06_closures.md)
