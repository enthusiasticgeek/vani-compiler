# Intermediate 6b -- Iterators and combinators (intuition primer)

> **Learning goal**: build a mental model of "iterators" and
> the "combinator" pattern -- the functional-programming style
> of expressing computations over collections via chains of
> small operations. Reading order:
> [06a closures primer](06a_closures_primer.md) -> here ->
> [Intermediate 6 closures + iterator combinators](06_closures.md).

This chapter has **no compiler code**. Pure intuition.

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
let total: i64 = xs.filter(|x| x % 2 == 0)
                   .map(|x| x * 2)
                   .fold(0, |acc, x| acc + x);
```

Each step does ONE thing. The chain READS like the
description: "filter ... then map ... then fold." If you need
to change one step, you change one step. The code is its own
documentation.

That's the iterator-combinator style. vāṇी supports it through
function-style helpers (`vec_filter`, `vec_map`, `vec_fold`)
and the method-call sugar (`xs.filter(...)`, `xs.map(...)`,
`xs.fold(...)`).

## An iterator as a "tap"

Picture each step as a kitchen-sink tap.

- **`xs`** is the water source -- the full list of numbers.
- **`.filter(f)`** is a tap that lets some water through and
  blocks the rest. (`f` is a closure deciding pass/block per
  drop.)
- **`.map(g)`** is a tap that REPLACES each drop with a
  different drop. (`g` is the transformation.)
- **`.fold(init, h)`** is the bucket at the end. It collects
  all the drops into one value using `h`.

Chain the taps; water flows through each in turn. The shape of
the chain matches the shape of the computation.

## The combinator vocabulary

You'll see these names across many languages. vāṇी's spellings:

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
consumer. Adapters don't actually do anything until the
consumer pulls -- that's called *lazy evaluation*.

## Lazy evaluation -- what it means and why it matters

In the imperative code, the `for` loop runs once, top to
bottom. Every operation happens for every element, in order.

In the combinator code, no work happens until the consumer
asks for it. When `.fold(0, ...)` is called, it requests an
element from `.map`. `.map` requests an element from
`.filter`. `.filter` requests an element from `xs`. The
element flows back up: pass-or-block, transform, accumulate.
Then the cycle repeats for the next element.

This matters when chains involve `.take(n)`:

```vani
let first_three_doubled: Vec<i64> = xs.map(|x| x * 2).take(3).collect();
```

If `xs` has a million elements, the lazy chain doesn't double
all million. It doubles ONE, hands it to `.take`, `.take`
forwards it, `.collect` collects it. Then the SECOND. Then
the THIRD. After three, `.take(3)` says "no more"; the chain
stops. The remaining 999,997 elements are never touched.

Lazy evaluation makes chains efficient even when intermediate
operations would be expensive on the full input.

## The closure connection

Every adapter takes a closure (chapter 06a). The closure
captures whatever local context the adapter needs. This is
why iterators + closures live in the same conceptual area --
they compose to express most "operations over a collection"
patterns.

```vani
let threshold: i64 = compute_threshold();
let large_ones: Vec<i64> = xs.filter(|x| x > threshold).collect();
                              // ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
                              //  closure captures threshold
```

`threshold` is computed once outside the chain; the closure
inside `.filter` reaches out and captures it. Without closures,
you'd need a separate top-level function plus a way to pass
threshold to it.

## Fusion -- what the compiler does for performance

A common worry: "doesn't all this chaining make a lot of
intermediate Vecs and Closures and slow things down?"

In a naive implementation, yes. In vāṇी, no -- the compiler
**fuses** adjacent combinators into a single loop.

```vani
let total: i64 = xs.map(|x| x * 2)
                   .filter(|x| x > 10)
                   .sum();
```

A naive compiler would: build a doubled-vec, build a
filtered-vec, sum the filtered-vec. Three passes, two
intermediate allocations.

vāṇี's fuser sees the chain and emits:

```
total = 0
for x in xs:
  let y = x * 2          // map step
  if y > 10:             // filter step
    total = total + y    // sum step
```

ONE pass, ZERO intermediate allocations. The combinator
syntax compiles to the same machine code as the hand-written
loop -- but you wrote it more readably.

This is the "best of both worlds": functional clarity at the
source level, imperative efficiency at the machine level.
Other languages call this "loop fusion" or "deforestation".

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
- **Adapters**: `map`, `filter`, `take`, `drop` -- chain them.
- **Consumers**: `fold`, `sum`, `count`, `collect` -- end the
  chain.
- **Lazy evaluation**: chains only do work when the consumer
  asks. `.take(3)` after a million-element source touches
  only 3 elements.
- **Fusion**: the compiler combines adjacent combinators into
  a single loop. Functional syntax, imperative codegen.

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

