# vāṇī design idioms -- intuition primer

> **Read this before the GoF tutorial.** The Gang of Four
> patterns were named for class-based, inheritance-heavy
> languages. vāṇी has a different set of primitives -- affine
> ownership, interfaces, closures, enums, channels -- and
> several GoF patterns reduce to a one-liner or disappear
> entirely. Knowing the idiomatic vāṇी shape first makes the
> GoF translation easier to read.

## Local cooking customs

Imagine you move to a new city and want to cook a dish from a cuisine
you didn't grow up with -- say, a noodle stir-fry. You find a recipe,
follow the steps, and the result is edible. Technically you
succeeded: you have food, it tastes fine, nobody's going hungry.

But if you watch someone who grew up cooking that dish, you notice
they do things differently, even though the ingredient list is
nearly identical. They cut the vegetables at a particular angle so
they cook evenly and stay crisp. They add the aromatics before the
protein, not after, because the order changes what flavor the oil
picks up. They plate it a certain way -- noodles first, toppings
arranged so the color balance reads as "done right" to anyone else
from that food culture. None of this is enforced by a health
inspector or a law. Your version and theirs are both safe to eat,
both technically "a stir-fry." But theirs is instantly recognizable
to another local cook as the way it's supposed to be done -- and
because it follows the shared conventions, another local cook can
glance at it, understand every choice at once, and immediately riff
on it: swap a vegetable, adjust the heat, extend the recipe --
without first having to puzzle out why it was built that way.

Your version, cut and stacked and seasoned your own improvised way,
might taste just as good -- but a local cook has to stop and study it
before they can extend it, because it doesn't follow the patterns
they already know how to read at a glance.

That gap -- between "technically works" and "recognizable,
comfortable, and easy for other locals to build on" -- is exactly
what a programming language's **idioms** are about. Any code that
compiles is like the edible stir-fry: it runs, it's not wrong. But
vāṇी, like every language, has its own local cooking customs --
conventional shapes for common problems -- that aren't enforced by
the compiler (the compiler doesn't care about your knife technique)
but ARE what makes your code instantly readable to anyone else who's
used to vāṇी's kitchen, and easy for them to modify later without
first reverse-engineering your particular choices. The rest of this
chapter is a tour of those local customs: the "way locals do it" for
problems that, in OOP languages, usually get solved with a class
hierarchy.

## Idiom 1: enum instead of a class hierarchy

In OOP you might write a `Shape` base class with `Circle` and
`Rect` subclasses. In vāṇी the natural form is a single enum:

```
enum Shape {
  Circle { radius: i64 },
  Rect   { w: i64, h: i64 },
}

fn area(s: Shape) -> i64 {
  match s {
    Shape::Circle { radius: r } then { return 3 * r * r; }
    Shape::Rect   { w, h }      then { return w * h; }
  }
}
```

This collapses **Visitor**, **State**, and **Command** from
three-class hierarchies into a match expression. The compiler
enforces exhaustiveness, so you cannot forget a case.

## Idiom 2: closure or function pointer instead of Strategy

The Strategy pattern in OOP wraps an algorithm in an object so
it can be swapped at runtime. In vāṇी a closure or a plain
function pointer does the same job with less ceremony:

```
fn apply(xs: ref Vec<i64>, key: fn(i64) -> i64) -> i64 {
  let best: i64 = vec_get(ref xs, 0);
  let i: i64 = 1;
  while i < length(ref xs) {
    let v: i64 = vec_get(ref xs, i);
    if key(v) > key(best) { best = v; }
    i = i + 1;
  }
  return best;
}
```

Pass `fn(x: i64) -> i64 { return x; }` for max,
`fn(x: i64) -> i64 { return 0 - x; }` for min. No interface,
no struct, no `dyn`. The GoF Strategy tutorial still shows the
`dyn SortStrategy` form because that's useful when the strategy
itself carries state.

## Idiom 3: interface + `<T: Iface>` for the stateful strategy case

When the "strategy" needs its own data, bind it with a generic
bound instead of a base class:

```
interface Sorter {
  fn sort_key(self: Self, v: i64) -> i64;
}

fn best<T: Sorter>(xs: ref Vec<i64>, s: T) -> i64 {
  // ...same loop, calls s.sort_key(v)
}
```

The `<T: Sorter>` inline bound (available since v0.5.3) replaces
the verbose `where T is Sorter` clause. The compiler
monomorphizes at the call site -- zero vtable overhead.

## Idiom 4: ownership transfer instead of RAII destructor classes

In OOP, Decorator and Proxy often wrap an object in a class that
calls cleanup in its destructor. In vāṇी the affine type system
makes ownership transfer explicit:

```
fn with_lock(m: ref Mutex<i64>, f: fn() -> i64) -> i64 {
  let guard = mutex_lock(ref m);   // acquires
  let result = f();
  mutex_unlock(guard);             // releases -- single clear owner
  return result;
}
```

There is no hidden destructor. The lock guard is an affine value
-- it cannot be copied, so it must be released exactly once.

## Idiom 5: Vec of function pointers instead of Observer subscriber list

Observer in OOP maintains a list of subscriber objects. vāṇी's
equivalent is a `Vec<fn(i64) -> i64>` -- no interface wrapping
needed when all subscribers share the same signature:

```
fn notify(listeners: ref Vec<fn(i64) -> i64>, event: i64) -> i64 {
  let i: i64 = 0;
  while i < length(ref listeners) {
    let f: fn(i64) -> i64 = vec_get(ref listeners, i);
    let _ = f(event);
    i = i + 1;
  }
  return 0;
}
```

When subscribers carry different state or need different
signatures, use `Vec<dyn EventHandler>` -- see the full GoF
tutorial.

## Idiom 6: arena Vec instead of recursive object graph

Composite, Decorator, and any tree-shaped structure face a
problem: vāṇī has no `Box<T>` (no recursively-typed fields). The
canonical answer is an **arena**: a flat `Vec<Node>` where each
node holds *indices* into the same Vec, not pointers:

```
struct Node {
  value:    i64,
  children: Vec<i64>,   // indices into the arena
}

fn sum_tree(arena: ref Vec<Node>, idx: i64) -> i64 {
  let node = vec_get(ref arena, idx);
  let total: i64 = node.value;
  let i: i64 = 0;
  while i < length(ref node.children) {
    let child_idx = vec_get(ref node.children, i);
    total = total + sum_tree(ref arena, child_idx);
    i = i + 1;
  }
  return total;
}
```

This pattern appears in Composite and any linked-structure
example in the GoF tutorial.

## Quick reference

| OOP instinct | vāṇी idiom |
|---|---|
| Subclass hierarchy | Enum variants + match |
| Strategy object | `fn` pointer or closure |
| Stateful strategy | Generic `<T: Iface>` bound |
| Destructor / RAII wrapper | Affine ownership transfer |
| Observer subscriber list | `Vec<fn(...)>` or `Vec<dyn Iface>` |
| Recursive pointer graph | Index-based arena `Vec<Node>` |
| Global mutable singleton | `Atomic<T>` in caller scope, passed by `ref` |

---

**Previous**: [Sec.15b -- Vec statistics and combinators ->](15b_vec_stats.md)
**Next**: [SOLID design principles ->](11b_solid_primer.md)
