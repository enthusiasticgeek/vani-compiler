# Intermediate 6b -- Iterators and combinators (intuition primer)

> **Learning goal**: build a mental model of "iterators" and
> the "combinator" pattern -- the functional-programming style
> of expressing computations over collections via chains of
> small operations. Reading order:
> [06a closures primer](06a_closures_primer.md) -> here ->
> [Intermediate 6 closures + iterator combinators](06_closures.md).

This chapter is mostly intuition, with real iterator-combinator
code once the analogy lands.

## The kitchen tap

Picture your kitchen sink. You turn the tap handle and water starts
flowing -- not because a hidden tank behind the wall filled up in
advance and is waiting for you, but because the instant you open the
valve, water begins moving through the pipe, one unit at a time, for
as long as you keep the handle open. Close the handle and the flow
simply stops. Nothing was pre-made, nothing is left over sitting in
a bucket somewhere.

Now imagine you clip a couple of attachments onto the end of the tap,
one after another: first a mesh screen that only lets clear water
through and blocks anything gritty, then a little dye cartridge that
tints whatever passes through it, then finally a cup held under the
spout to catch what comes out. None of these attachments keeps a
private stash of water. The screen doesn't pre-filter a bucketful and
wait around -- it only touches a drop the instant that drop reaches
it. The dye cartridge only tints a drop when a drop is actually
passing through it right then. And critically: **nothing happens at
all until you turn the handle and the cup starts wanting water.** The
whole line -- tap, screen, dye, cup -- sits idle and inert until the
demand at the very end (the cup filling up) pulls a drop through the
whole chain of attachments.

If you only wanted three cupfuls, you'd turn the handle, let exactly
three drops travel through the whole chain, and shut the tap back
off. The screen and the dye cartridge never processed a backlog of
water "just in case" -- they touched only those three drops, one at a
time, only because the cup at the end asked for exactly that many.

That's the whole idea behind an **iterator chain**. `xs.filter(...)
.map(...).fold(...)` is exactly this tap-plus-attachments setup:
`xs` is the water source, `.filter(...)` is the screen, `.map(...)`
is the dye cartridge, and `.fold(...)` (or `.collect()`, or whatever
sits at the very end) is the cup -- the thing that actually asks for
values and makes anything happen at all. Nothing gets computed or
stored in the gaps between attachments; values flow through the
whole chain one at a time, only when the far end asks for the next
one. Programmers call this behavior **lazy evaluation**, and each
attachment clipped onto the chain (the screen, the dye cartridge) is
what the rest of this chapter calls a **combinator** or an
**adapter**. Keep the tap picture in mind -- the rest of this chapter
is just giving these parts their formal names and vāṇī's exact
spellings for them.

## The problem: collections of operations

You have a list of numbers. You want to:
1. Keep only the even ones.
2. Double each.
3. Sum them up.

The "imperative" way -- write a loop:

```vani
let total: i64 = 0;
for x in ref xs {
  if x % 2 == 0 {
    total = total + x * 2;
  }
}
```

This works. But notice: the loop body MIXES three concerns
(filter, transform, accumulate). When you come back to the
code in 6 months, you have to mentally untangle them.

The "iterator combinator" way:

```vani
let evens: Vec<i64> = xs.filter(|x| x % 2 == 0);
let doubled: Vec<i64> = evens.map(|x| x * 2);
let total: i64 = doubled.fold(0, |acc, x| acc + x);
```

Each step does ONE thing. Read top to bottom: "filter ... then
map ... then fold." If you need to change one step, you change
one step. The code is its own documentation.

**A style note, not just a preference**: unlike Rust, v1's
method-call sugar only rewrites a receiver that's a plain,
named variable (`xs.filter(...)`) -- chaining directly onto the
result of another call (`xs.filter(...).map(...)`, all on one
expression) is rejected: "cannot call method 'map' on `Vec<i64>`
-- methods are attached to struct/enum types only in v1" (the
method-call sugar never got a chance to rewrite it, because the
receiver there is a call expression, not a `Var`). Give each
step its own `let` the way the three lines above do -- same
rule that governs `ref`/`mut ref`, which also only bind to a
named place, never an arbitrary expression.

That's the iterator-combinator style. vāṇī supports it through
function-style helpers (`vec_filter`, `vec_map`, `vec_fold`)
and the method-call sugar (`xs.filter(...)`, `xs.map(...)`,
`xs.fold(...)`).

## The combinator vocabulary

You'll see these names across many languages. vāṇī's spellings:

### Producers -- start a chain

- **The Vec itself** -- `xs` produces all its elements in order.
- **`vec_range(0, 10)`** -- produces `0, 1, 2, ..., 9`.

### Adapters -- middle of a chain

Each takes a closure and returns a new iterator with the
closure applied per element.

- **`.map(g)`** -- replace each element with `g(element)`.
- **`.filter(f)`** -- keep only elements where `f(element)` is
  true.
- **`.take(n)`** -- yield only the first n.
- **`.drop(n)`** -- skip the first n, yield the rest.

### Consumers -- end a chain

- **`.fold(init, h)`** -- combine all elements using `h`.
  Returns ONE value.
- **`.sum()`** -- shorthand for `.fold(0, |a, x| a + x)`.
- **`.count()`** -- how many elements made it through?
- **`.collect()`** -- gather the elements into a fresh Vec.

A chain has zero or more adapters between a producer + a
consumer.

## v1 is eager, not lazy -- a real difference from Rust/Python

<img class="manas" src="../images/mascot/manas_mascot_caution.png" title="this is a real v1 boundary, not a style choice"/>

The kitchen-tap picture at the top of this chapter -- water
flowing on demand, nothing pre-made -- is how Rust's iterators
and Python's generators actually work: *lazy evaluation*, where
an adapter only touches an element the instant the consumer
asks for the next one. **vāṇī v1 does NOT work this way.** Every
adapter is **eager**: `.filter(...)` walks the whole input right
then and there and materializes a brand-new `Vec` holding the
results, before the next step ever runs. `.map(...)` does the
same. There is no on-demand pulling in the Rust/Python sense --
but see the "Combined builtins" section below: a `let`-by-`let`
chain in this exact shape is one case where the compiler *does*
quietly collapse the passes for you, without you reaching for
anything by name.

Concretely:

```vani
let evens: Vec<i64> = xs.filter(|x| x % 2 == 0);   // walks ALL of xs, builds a new Vec
let doubled: Vec<i64> = evens.map(|x| x * 2);       // walks ALL of evens, builds another new Vec
```

If `xs` has a million elements, `.filter` walks all million and
allocates a fresh Vec for whatever passed; `.map` then walks
*that* Vec's elements (however many passed the filter) and
allocates a second fresh Vec. `.take(n)` is no exception -- it
still needs its input Vec already fully built; it slices the
first `n` elements off an already-complete result, it does not
short-circuit the step that produced that Vec:

```vani
let all_doubled: Vec<i64> = xs.map(|x| x * 2);      // doubles EVERY element of xs, million included
let first_three: Vec<i64> = all_doubled.take(3);    // then keeps just the first 3
```

`.collect()` fits this model exactly, and explains its own
implementation: since every adapter already returns a real,
finished `Vec`, `.collect()` on a Vec-typed value is just
identity -- it exists so code translated from a lazy language
still reads naturally, not because there's a lazy stream that
needs materializing.

None of this makes combinator chains *wrong* to reach for --
they're still clearer than a hand-rolled loop for most cases --
but budget for the real cost: each step in a chain is a full
pass + a full allocation, same as if you'd written the
intermediate Vecs out by hand. For a chain where that overhead
actually matters, see the combined builtins below.

## The closure connection -- and a real limit worth knowing up front

<img class="manas" src="../images/mascot/manas_mascot_caution.png" title="this is a real v1 boundary, not a style choice"/>

Every adapter takes an anonymous function (chapter 06a covers
"closure" generally). It's tempting to assume that means you
can reach out and capture local context the way a real closure
does -- but `vec_map`/`vec_filter`/`vec_fold` (and everything
this chapter's method-call sugar desugars to) are typed to take
a **plain, non-capturing function pointer**: `fn(i64) -> i64`,
`fn(i64) -> bool`, and so on ([Intermediate 6](06_closures.md)
spells out the exact table). A function pointer is just a code
address -- there's no environment slot for a captured value to
live in, unlike a real `Closure` (chapter 06a's two-pointer
bundle). Confirmed directly: even a `Copy` capture fails --

```vani
let threshold: i64 = 3;
let filtered: Vec<i64> = xs.filter(|x| x > threshold);
// error: unknown variable 'threshold' -- the closure literal
// here compiles straight to a bare fn pointer with no capture
// slot, so `threshold` is simply out of scope inside it.
```

This isn't a version of the "chain directly" restriction from
earlier in this chapter -- it fails exactly the same way even
with `threshold` passed to the equivalent `vec_filter(ref xs,
|x| x > threshold)` free-function form. **Every closure literal
you pass to a `vec_*` combinator must be self-contained** --
reference only its own parameters and top-level items, nothing
from the enclosing scope. If you need outer context, the honest
v1 answer is: write the loop explicitly instead --

```vani
let threshold: i64 = 3;
let large_ones: Vec<i64> = vec();
for x in ref xs {
  if x > threshold {
    let _ = push(mut ref large_ones, x);
  }
}
```

-- which is exactly the "imperative way" this chapter opened by
contrasting with combinators. For this specific "filter by a
captured value" shape, the loop isn't a fallback for people who
don't like combinators; it is, today, the only way that works.

## Combined builtins -- and the compiler's own quiet fusion

A common worry: "doesn't all this chaining make a lot of
intermediate Vecs and slow things down?" Given the previous
section, the naive answer would be: yes, exactly that much --
each step in a `let`-by-`let` chain is its own full pass and its
own fresh allocation. **But v1's compiler actually auto-fuses
exactly this shape for you**, transparently, with no action
required: write `let m = xs.map(f); let t = m.fold(init, g);`
(with `m` used nowhere else), and the compiler rewrites it into a
single fused `map_fold` call before codegen -- confirmed directly
by inspecting the generated C, which contains one call to the
fused helper and no intermediate `m` Vec anywhere. The same pass
recognizes `filter` then `fold`, `map` then `filter`, and (by
fusing twice) the full 3-stage `map` then `filter` then `fold`
chain written as three separate `let`s -- also confirmed directly,
collapsing all the way down to a single `map_filter_fold` call.
It isn't limited to strictly back-to-back `let`s either -- it
still fires across intervening statements as long as none of them
reference the intermediate binding or the original source Vec.

This is **not** a general-purpose fuser, though, and it's worth
being precise about the boundary: it only recognizes this specific
"one `let` produces, a later `let`'s RHS is the sole consumer"
shape -- not the chained single-expression form
`xs.map(...).filter(...).fold(...)` (which, as covered above,
doesn't even parse without intermediate `let`s), and not
combinator shapes outside the four recognized producer/consumer
pairs. If your chain doesn't fit, it costs one pass + one
allocation per step, same as any other eager language without a
fuser for that shape.

For the cases the auto-fuser doesn't reach -- or when you'd
rather be explicit about the fusion instead of relying on the
compiler noticing it -- v1 also ships a small, fixed set of
**hand-written, pre-fused combined builtins** you can call by
name, covering the most common 2- and 3-step shapes, each doing
everything in ONE pass with ZERO intermediate Vecs:

- **`.map_fold(init, mapper, folder)`** -- map then fold.
- **`.filter_fold(init, predicate, folder)`** -- filter then fold.
- **`.map_filter(mapper, predicate)`** -- map then filter, still
  returns a `Vec` (no fold at the end).
- **`.map_filter_fold(init, mapper, predicate, folder)`** -- all
  three in one pass.

```vani
let total: i64 =
  xs.map_filter_fold(0, |x| x * 2, |y| y > 10, |acc, y| acc + y);
```

This one call walks `xs` exactly once, applying the map, the
filter, and the fold inline per element -- equivalent to the
hand-written loop:

```
total = 0
for x in xs:
  let y = x * 2          // map step
  if y > 10:             // filter step
    total = total + y    // fold step
```

ONE pass, ZERO intermediate allocations -- and the same result the
auto-fuser above would already give you for free if you'd instead
written it as three separate `let`s (`let m = xs.map(...); let g
= m.filter(...); let t = g.fold(...);`). Reach for the named
builtin when you want the fusion to be visible in the source
itself, or when your chain doesn't fit the `let`-sequence shape
the auto-fuser looks for. Either way, the chained single-expression
form `.map(...).filter(...).fold(...)` still doesn't even parse --
that restriction is unrelated to fusion and covered above.

## When NOT to use combinators

Some loops don't fit the chain shape cleanly:

- **Multi-output**: a loop that builds two different Vecs from
  one input. Combinator chains naturally produce ONE output;
  forcing two outputs into chains makes the code worse than
  just writing the loop.
- **Inter-element dependencies**: each iteration depends on the
  PREVIOUS iteration's result. (Running sums via `.fold` work;
  more complex dependencies don't.)
- **Side effects per element**: combinators encourage thinking
  of operations as pure transforms. If your "loop body" is
  primarily about side effects (printing, mutation), an
  explicit loop reads cleaner.

Rule of thumb: if you can describe the computation as a
sequence of "for each element, do X" steps, combinators are
the right tool. If the steps involve subtle inter-element
state, write the loop.

## A summary you can carry

- **Iterator** = a sequence of values producible one at a
  time. Vec is the canonical iterator source.
- **Combinator** = a transformation that consumes an iterator
  and produces a new one (adapter) or a single value
  (consumer).
- **Adapters**: `map`, `filter`, `take`, `drop` -- compose them
  via a separate `let` per step (v1's method-call sugar only
  rewrites a plain-variable receiver; `xs.filter(...).map(...)`
  chained directly on one expression doesn't parse).
- **Consumers**: `fold`, `sum`, `count`, `collect` -- end the
  chain.
- **v1 is eager, not lazy, at the source-code level**: each
  `.map`/`.filter`/`.take`/`.drop` call you write is semantically
  a full pass that materializes a fresh `Vec` before the next
  step runs -- unlike Rust/Python, nothing is pulled on-demand,
  and `.take(3)` still requires its input already fully computed.
- **The compiler auto-fuses the common two-`let` shape for you**:
  write `let m = xs.map(f); let t = m.fold(init, g);` (with `m`
  used only by that one `fold`) and the compiler transparently
  rewrites it into a single fused `map_fold` call under the hood
  -- confirmed directly by inspecting the generated C, which
  contains one `..._map_fold(...)` call and no intermediate `m`
  Vec at all. The same fusion recognizes `filter`+`fold`,
  `map`+`filter`, and (by fusing twice) the 3-stage
  `map`+`filter`+`fold` chain, and isn't limited to strictly
  adjacent `let`s -- it still fires across intervening statements
  as long as none of them touch `m` or the original source Vec.
  This is a real, currently-shipped optimization, not "future
  work" -- but it's also not *general* fusion: it only recognizes
  these specific producer-then-consumer `let` shapes, not
  arbitrary combinator combinations or chains that don't fit this
  pattern. `.map_fold`, `.filter_fold`, `.map_filter`, and
  `.map_filter_fold` also exist as hand-written builtins you can
  call directly -- useful when you'd rather be explicit about the
  fusion than rely on the compiler noticing it.

That's iterators. The next chapter ([Intermediate 6](06_closures.md))
shows the actual `.map` / `.filter` / `.fold` syntax + worked
examples.

## Cross-reference

- [Intermediate 6a -- Closures primer](06a_closures_primer.md)
  -- combinators take closures as arguments; the pair is the
  full functional vocabulary
- [Intermediate 6 -- Closures + iterator combinators](06_closures.md)
  -- the formal chapter with all syntax
- [Intermediate 4c -- Generics primer](04c_generics_primer.md)
  -- `vec_map<T, R>` is generic over the input and output
  element types; monomorphization specializes per pair
- [Advanced 2a -- Parallelism primer](../advanced/02a_parallelism_primer.md)
  -- `parallel for` shares the per-element-independent-iteration
  insight that makes both iterators and parallelism work


---

**Previous**: [Sec.6a -- Closures and lambda lifting primer ->](06a_closures_primer.md)
**Next**: [Sec.6 -- Closures and iterator combinators ->](06_closures.md)

