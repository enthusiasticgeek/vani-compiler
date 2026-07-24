# Intermediate 3d -- Cyclic references: Rust's `Weak<T>` vs vāṇी's index handles

> **Learning goal**: see the three canonical shapes that push
> Rust users into `Rc<RefCell<T>> + Weak<T>` -- parent<->child
> trees, doubly-linked lists, observer pattern -- and the
> idiomatic vāṇी translation for each. Reading order:
> [Intermediate 3c -- shared ownership without Rc/Arc](03c_shared_ownership_primer.md)
> + [Intermediate 3a -- Box+RAII primer](03a_box_raii_primer.md).

This chapter has **no compiler code**. Pure intuition with
side-by-side worked examples.

## Why cycles are hard

A *cycle* in a data structure means two values point at each
other, either directly (`A -> B` and `B -> A`) or through a
longer loop (`A -> B -> C -> A`). Cycles are useful: a tree
node wants a `parent` pointer for upward traversal; an
observer wants to call back into the subject that registered
it; a doubly-linked list needs `prev` AND `next`.

Under **single-owner affine** ownership (vāṇी, Rust without
Rc), cycles are *impossible* to express directly with owning
pointers -- every value has exactly one owner, so the graph
has no loops by construction. That's a feature when you
don't need cycles. It's a friction when you do.

Under **reference-counted** ownership (Rust with `Rc<T>`,
C++ with `shared_ptr<T>`), cycles ARE expressible -- but they
LEAK. Every clone bumps a refcount; every drop decrements; a
cycle's refcount never reaches zero; the cleanup never fires.
Rust's `Weak<T>` (or C++'s `weak_ptr<T>`) is the workaround:
a non-owning pointer whose existence doesn't keep the target
alive. The author marks back-edges as `Weak<T>` so the
forward edges still count and the cycle eventually drops.

vāṇी's answer is different: **don't use pointers at all for
cyclic shapes.** Use indices into a `Vec<Node>` (or `Pool<T>`
when generation-checked handles are needed). The cycle exists
in the index graph; the storage is a flat array. Cleanup is
the Vec's drop -- one wholesale free, no per-node count.

This chapter walks three concrete shapes.

## Shape 1 -- Parent <-> child tree

Picture a UI tree (or a DOM, or an AST): each node has zero
or more children AND a back-pointer to its parent for
traversal up the hierarchy. Children own each other along
the forward edge; parents are referenced but not owned.

### The Rust shape

```vani
use std::rc::{Rc, Weak};
use std::cell::RefCell;

struct Node {
    value: i64,
    parent: RefCell<Weak<Node>>,         // <- Weak: back-edge
    children: RefCell<Vec<Rc<Node>>>,    // <- Rc: forward-edge
}

let root = Rc::new(Node {
    value: 0,
    parent: RefCell::new(Weak::new()),
    children: RefCell::new(vec![]),
});

let child = Rc::new(Node {
    value: 1,
    parent: RefCell::new(Rc::downgrade(&root)),    // <- Rc -> Weak
    children: RefCell::new(vec![]),
});

root.children.borrow_mut().push(Rc::clone(&child));
```

Three things to notice:

1. **`Rc<Node>` for children, `Weak<Node>` for parent.** If
   parent were `Rc<Node>`, the cycle wouldn't drop --
   parent's refcount would include the child's pointer
   AND the child's refcount would include the parent's
   pointer. Both stay non-zero forever; nothing frees.
2. **`RefCell<...>` everywhere mutable.** The fields need
   interior mutability so you can update parent / children
   through a shared `Rc<Node>` reference. RefCell pushes
   borrow checking from compile time to runtime; mis-using
   it panics.
3. **`borrow_mut()` chains** at every access. The ergonomic
   cost is real -- `root.children.borrow_mut().push(...)` is
   three layers of indirection: dereference Rc, take mut
   borrow of RefCell, mutate the Vec inside.

### The vāṇी shape

```vani
struct Node {
  value: i64,
  parent: i64,           // -1 for root; otherwise index into nodes[]
  children: Vec<i64>,    // indices into nodes[]
}

struct Tree { nodes: Vec<Node> }

fn add_child(t: mut ref Tree, parent_idx: i64, value: i64) -> i64 {
  let new_idx: i64 = len(t.nodes) as i64;
  let _ = push(mut ref t.nodes, Node {
    value: value,
    parent: parent_idx,
    children: vec(),
  });
  // Wire the parent's children list:
  let _ = push(mut ref t.nodes[parent_idx as u64].children, new_idx);
  return new_idx;
}

fn root(t: mut ref Tree, value: i64) -> i64 {
  let new_idx: i64 = len(t.nodes) as i64;
  let _ = push(mut ref t.nodes, Node {
    value: value,
    parent: 0 - 1,
    children: vec(),
  });
  return new_idx;
}
```

Three things to notice:

1. **One `Tree` owns everything.** The `Tree` struct holds
   the `Vec<Node>`. Nodes don't own each other -- they're
   peers in a flat array. Drop is the Vec's drop, one call.
2. **`parent: i64`, `children: Vec<i64>`.** Both edges are
   indices. The "cycle" exists at the index level --
   `nodes[child].parent == parent_idx` and
   `nodes[parent_idx].children` contains `child` -- but
   neither end OWNS the other. No Rc, no Weak, no RefCell.
3. **Mutation is direct.** `push(mut ref t.nodes[i].children,
   new_idx)` is one borrow chain. Affine ownership of the
   `Tree` lets vāṇी's checker prove this safe at compile
   time -- no runtime borrow checks, no panics.

The cost: indices need a "world" parameter (`t: mut ref
Tree`) threaded through anything that touches the tree. The
Rust version doesn't need this -- each Rc carries its own
context. In exchange, vāṇी has no refcount overhead, no
per-access panic risk, and cache-friendly layout (all nodes
contiguous).

### When does cleanup happen?

- **Rust**: when the last `Rc<Node>` to the root drops, the
  root's strong count hits 0, the root drops, which drops
  its Vec<Rc<Node>> children, which decrements each child's
  count to 0, which drops each child. The cascade can be
  deep. `Weak<Node>` back-pointers are inert during this --
  their non-zero weak count keeps the *control block*
  allocated until the last Weak drops, but the Node's data
  is already gone.
- **vāṇी**: when the `Tree` binding's scope ends, the
  `nodes: Vec<Node>` drops -- one `free` of the contiguous
  buffer (after running each Node's own field destructors).
  No cascade. No per-node bookkeeping.

## Shape 2 -- Doubly-linked list

Each node has `next` and `prev`. Every interior node is
pointed at by two others. Pure-affine ownership can't
express this directly; one of the two pointers has to be
non-owning.

### The Rust shape

```vani
use std::rc::{Rc, Weak};
use std::cell::RefCell;

struct Node {
    value: i64,
    next: RefCell<Option<Rc<Node>>>,      // <- Rc: forward
    prev: RefCell<Option<Weak<Node>>>,    // <- Weak: back
}
```

Same pattern as the tree: pick a direction for ownership
(forward), make the other direction non-owning. The list's
"head" holds an `Rc<Node>` to the first element; each
subsequent forward edge is `Rc<Node>`; every backward edge
is `Weak<Node>`. Drop the head and the whole list cascades.

The walking-backwards code looks like:

```vani
fn walk_back(end: Rc<Node>) {
    let mut cur = Some(end);
    while let Some(n) = cur {
        println!("{}", n.value);
        cur = n.prev.borrow().as_ref().and_then(|w| w.upgrade());
    }
}
```

`w.upgrade()` is the key operation: a `Weak<Node>` can be
asked "is the target still alive?" -- it returns
`Option<Rc<Node>>`, `Some` if the strong count is still
positive, `None` if it dropped. This is what makes Weak safe:
you can't accidentally dereference a freed target.

### The vāṇी shape

```vani
struct Node {
  value: i64,
  next: i64,    // index or -1
  prev: i64,    // index or -1
}

struct List {
  nodes: Vec<Node>,
  head: i64,    // index or -1 if empty
  tail: i64,    // index or -1 if empty
}

fn push_back(l: mut ref List, value: i64) -> i64 {
  let new_idx: i64 = len(l.nodes) as i64;
  let prev_tail: i64 = l.tail;
  let _ = push(mut ref l.nodes, Node {
    value: value,
    next: 0 - 1,
    prev: prev_tail,
  });
  if prev_tail >= 0 {
    l.nodes[prev_tail as u64].next = new_idx;
  } else {
    l.head = new_idx;
  }
  l.tail = new_idx;
  return new_idx;
}

fn walk_back(l: ref List) -> i64 {
  let mut cur: i64 = l.tail;
  while cur >= 0 {
    print l.nodes[cur as u64].value;
    cur = l.nodes[cur as u64].prev;
  }
  return 0;
}
```

Symmetric. Both directions are indices; both are equally
"cheap" to follow. No Rc, no Weak, no upgrade. The List
owns the storage; head/tail are sentinel indices.

### The deletion subtlety

In a Rust+Rc doubly-linked list, deleting a middle node
means dropping its incoming Rc; the Weak prev pointers on
adjacent nodes silently become "dead" and `upgrade()` will
return None for the deleted slot. The forward chain repairs
itself when adjacent nodes' `next` is rewritten.

In a Vec-of-Node list, deleting a middle node has a choice:

1. **Swap-remove** the node from the Vec (replace it with
   the last element, decrement len). Cheap O(1), but every
   index that pointed at the removed-OR-last node is now
   wrong. Either update those indices or use a `Pool<T>`
   with generation tracking.
2. **Mark deleted** without removing -- add a `dead: bool`
   field, skip over dead nodes when traversing. Indices
   stay valid forever, but the Vec grows monotonically.
3. **Use a `Pool<T>`** (vāṇी [unsafe.md](https://github.com/enthusiasticgeek/vani-compiler/blob/main/unsafe.md)
   Layer 2). Generation-tagged handles: deleting a slot
   bumps the generation; `pool.get(h)` returns `None` for
   a stale handle. Same memory layout, type-safe slot
   opacity, O(1) safe delete.

The first option is fine for "build the list, walk it,
drop it." The third is what serious code reaches for. The
second is a debug-mode shortcut.

## Shape 3 -- Observer pattern

A `Subject` notifies many `Observer`s when state changes. The
classical OOP shape has each Observer hold a back-pointer to
the Subject (for unregistering) AND the Subject holds a list
of Observers. Cycle.

### The Rust shape

```vani
use std::rc::{Rc, Weak};
use std::cell::RefCell;

struct Subject {
    observers: RefCell<Vec<Weak<dyn Observer>>>,    // <- Weak so unregister doesn't fight us
    state: RefCell<i64>,
}

trait Observer {
    fn on_change(&self, new_state: i64);
}

struct PrintObserver { id: i64, subject: Weak<Subject> }
impl Observer for PrintObserver {
    fn on_change(&self, new_state: i64) {
        println!("observer {} sees {}", self.id, new_state);
    }
}
```

Both directions are `Weak`. Neither owns the other; an
external `Vec<Rc<PrintObserver>>` (the "registry") owns the
observers; an external `Rc<Subject>` owns the subject. The
Weaks ensure that dropping the registry doesn't keep the
subject alive, and vice versa.

This is fiddly. You have to remember which way ownership
flows (typically: the registry -> observers; the world ->
subject) AND set up the Weak edges correctly AND check
upgrades at every traversal.

### The vāṇी shape

```vani
struct Subject {
  state: i64,
  observers: Vec<i64>,    // indices into world.observers[]
}

struct World {
  subject: Subject,
  observers: Vec<Box<dyn Observer>>,
}

iface Observer {
  fn on_change(self: ref Self, new_state: i64) -> i64;
}

fn notify(w: mut ref World, new_state: i64) -> i64 {
  w.subject.state = new_state;
  let n: i64 = len(w.subject.observers) as i64;
  let mut i: i64 = 0;
  while i < n {
    let idx: i64 = w.subject.observers[i as u64];
    w.observers[idx as u64].on_change(new_state);
    i = i + 1;
  }
  return 0;
}

fn register(w: mut ref World, obs: Box<dyn Observer>) -> i64 {
  let new_idx: i64 = len(w.observers) as i64;
  let _ = push(mut ref w.observers, obs);
  let _ = push(mut ref w.subject.observers, new_idx);
  return new_idx;
}
```

Notice what's NOT there: no observer holds a back-pointer.
The `World` itself is the "shared context" -- both subject
and observers exist within it. Unregister is "remove this
index from subject.observers"; the observer's `Box<dyn Observer>`
slot remains until the World drops, OR you swap-remove from
`w.observers` and update indices, OR (as before) use a Pool
for stable handles.

The back-pointer was load-bearing in Rust ONLY because
nothing else gave the observer access to its subject. In
vāṇी, every callback that needs to mutate the subject takes
`world: mut ref World` as a parameter -- the access is
explicit and tracked.

### What you give up

In Rust, an Observer can hold its subject ref permanently
and notify itself "I want to unregister at end-of-life" via
its Drop impl. In vāṇी, the same operation is "the World
removes me; my Drop runs when my Box is freed." No callback
into a back-pointer needed.

If your design demands "the observer's Drop unregisters from
the subject without the World holding it," that's a genuine
Rc-required pattern. Use `unsafe(reason = "self-deregistering
observer needs raw subject pointer")` or refactor to
World-mediated lifecycle.

## When Rc+Weak is genuinely required

A small set of patterns are genuinely awkward without Rc:

- **Plugin systems** where a third-party plugin registers a
  callback that captures the subject; you don't control the
  callback's lifetime; unregistration may be racy.
- **Long-lived cached objects** with many independent
  consumers and no central registry to track them.
- **A graph with cycles where edges are NOT indexable** --
  e.g., a DOM node referenced by JS code, by layout, by
  rendering, by event handlers, all simultaneously and at
  independent lifetimes.

For these, vāṇी's answer is:

1. **Refactor toward indices + World.** Most "many
   independent consumers" become cleaner as "indices into
   a shared owner." It's more bookkeeping up front, less
   ambiguity at runtime.
2. **`unsafe(reason = "...")` for the unavoidable cases.**
   The reason string documents the discipline. The compiler
   doesn't trade safety silently -- you're explicitly opting
   out for one named reason.
3. **`region { ... }` blocks, for `i64` payloads today.** v2
   regions (Layer 5 in [unsafe.md](https://github.com/enthusiasticgeek/vani-compiler/blob/main/unsafe.md))
   have shipped -- `region name { ... }` + `region_borrow_i64` +
   `ArenaRef<i64>` (see [Advanced 4 -- Embedded](../advanced/04_embedded.md))
   permit cycles between same-region `i64` slots, freed together at
   region exit, zero runtime cost, no Rc semantics required.
   **Not yet generic**: `region_borrow_i64`/`ArenaRef<i64>` only
   cover `i64` -- a `region`-backed cyclic graph of arbitrary
   `Node` structs (the motivating case for this section) still
   needs one of the two options above until a generic
   `ArenaRef<T>` ships.

## Side-by-side cheat sheet

| Need | Rust shape | vāṇी shape |
|---|---|---|
| Parent <-> child tree | `Rc<Node> + Weak<Node>` + RefCell | `Vec<Node>` with `parent: i64` + `children: Vec<i64>` |
| Doubly-linked list | `Rc<Node>` next + `Weak<Node>` prev | `Vec<Node>` with `next: i64` + `prev: i64` |
| Observer pattern | `Weak<dyn Observer>` + `Weak<Subject>` | `World { subject, observers: Vec<Box<dyn Observer>> }` with indices |
| Shared cache | `Rc<CacheEntry>` per consumer | `Pool<CacheEntry>` + `Handle<CacheEntry>` per consumer |
| Self-deregistering | Observer's `Drop` upgrades a Weak<Subject> | refactor to World-mediated; or unsafe |
| Long-lived plugins | `Rc<Callback>` shared | `Pool<Callback>` + generation handles |

The general substitution: **`Rc<T>` becomes `Vec<T>` or
`Pool<T>` shared ownership; `Weak<T>` becomes an `i64` index
or a `Handle<T>`; `RefCell<T>` becomes `mut ref T` on the
owning Vec/Pool.**

## Drop-time comparison

Walking the same 1000-node tree drop, end-to-end:

| Operation | Rust `Rc<Node>` tree | vāṇी `Vec<Node>` tree |
|---|---|---|
| Allocations | 1000 (one per node, plus Rc control block) | 1 (Vec buffer; nodes inline) |
| Drop cascade | 1000 refcount decrements (atomic if Arc), then 1000 frees | 1 free (after per-node field destructors run inline) |
| Worst-case cost | Two refcount instructions per Rc clone everywhere; cascade depth = tree height | Linear scan of Vec at drop |
| Failure mode | Forgetting Weak on a back-edge -> cycle leak | None -- single-owner Vec can't cycle |

For tree-heavy programs (parsers, scene graphs, AST passes),
this difference compounds.

## A summary you can carry

- Three canonical cyclic shapes -- parent<->child tree,
  doubly-linked list, observer pattern -- all use
  `Rc<RefCell<T>> + Weak<T>` in Rust because affine
  ownership otherwise rejects them.
- vāṇी's substitution is **the same shape rewritten as
  indices into a `Vec<T>` (or `Pool<T>` for stable handles)
  owned by a single "World" struct.** The cycle exists in
  the index graph; ownership stays single.
- The cost trade: vāṇी needs a `world: mut ref World`
  parameter threaded through anything touching the
  structure (vs. Rust's per-Rc context). Pay-off: no
  refcount overhead, no `borrow_mut()` panic risk, no
  cycle-leak failure mode, cache-friendly layout, one
  wholesale drop.
- For genuinely unavoidable Rc patterns (third-party
  plugins, multi-modal DOM-like long-lived shared graphs),
  use `unsafe(reason = "...")` with explicit discipline, or
  `i64`-payload `region` blocks today, or await a generic
  `ArenaRef<T>` for the arbitrary-struct case.

The takeaway: **cycles in the data don't require cycles in
ownership.** Decouple them -- flat storage, index edges --
and the language's affine guarantees cover the rest.

## Cross-reference

- [Intermediate 3a -- Box+RAII primer](03a_box_raii_primer.md)
  -- single-owner heap allocation, the recursion shape
  `Option<Box<Node>>` for the simple (non-cyclic) linked list
- [Intermediate 3b -- Affine deeper primer](03b_affine_deeper_primer.md)
  -- many-shared-XOR-one-mutable rule that makes vāṇी's
  reasoning sound
- [Intermediate 3c -- Shared ownership without Rc/Arc](03c_shared_ownership_primer.md)
  -- the broader "no Rc by design" story; this chapter
  zooms in on the cyclic-data subset
- [Intermediate 5 -- Dynamic dispatch](05_dyn.md) --
  `Vec<Box<dyn Iface>>` for the observer pattern's
  heterogeneous observer list
- [Advanced 4 -- Embedded](../advanced/04_embedded.md) -- `region`
  typing (`Region` / `ArenaRef<i64>`, shipped for `i64` payloads)
  for cycles with compile-time lifetimes


---

**Previous**: [Sec.3c -- Shared ownership primer ->](03c_shared_ownership_primer.md)
**Next**: [Sec.3e -- Lifetimes and reference returns primer ->](03e_lifetimes_primer.md)

