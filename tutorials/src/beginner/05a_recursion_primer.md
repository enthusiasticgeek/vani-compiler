# Beginner 5a -- Recursion (intuition primer)

> **Learning goal**: understand what recursion IS, when it's
> the right tool, and how to read recursive functions without
> getting lost in the call stack. Reading order: read any
> time after [Beginner 03 -- Functions and the four return
> aliases](03_functions.md) and [Beginner 05 -- Loops](05_loops.md).

This chapter leads with an analogy, then real (small) vāṇī functions
throughout to ground it -- nothing here uses syntax beyond what
[Beginner 3](03_functions.md) and [Beginner 5](05_loops.md) already
covered. Read the analogy first.

## The nesting dolls

Picture a Russian nesting doll (matryoshka) -- a wooden doll that
opens up to reveal a slightly smaller doll inside, which opens to
reveal a smaller one still, and so on.

To find out how many dolls are in the set, you don't need a special
counting trick. You do the same simple thing over and over:

1. **Open the doll in front of you.**
2. **Is there another doll inside?**
   - If **no** -- you've hit the smallest doll. Stop. That's `1` more
     doll and nothing left to open.
   - If **yes** -- set that inner doll in front of you and go back to
     step 1. Whatever number you get from opening *that* doll, add 1
     for the one you just opened.

Nobody taught you a "nesting-doll counting formula." You just repeated
the same two-step check on a smaller doll each time, and trusted that
the smaller doll would follow the same rule. Eventually a doll doesn't
open anymore -- that's what stops you. Without a smallest solid doll,
you'd be opening dolls forever.

That's **recursion**: a task defined as "do a step, then do the exact
same task again on a smaller piece of the problem," plus one rule for
when the pieces run out.

- The **smallest solid doll** is the *base case* -- the point where
  the task stops calling itself and just answers directly.
- **Opening a doll and handing off to the smaller one** is the
  *recursive case* -- the task calls itself again, but on something
  strictly smaller than before.
- The stack of dolls sitting open on the table while you work your
  way inward is exactly what a computer's *call stack* is doing --
  each doll (function call) waits, half-opened, until the one inside
  it finishes.

Now the code below is just this same nesting-doll process, spelled
out for the computer.

## A function that uses itself

You've seen a function call other functions:

```vani
fn double(n: i64) -> i64 { return n * 2; }

fn quadruple(n: i64) -> i64 {
  return double(double(n));
}
```

`quadruple` calls `double` twice. Normal stuff.

A **recursive** function does something stranger -- it calls
**itself**:

```vani
fn countdown(n: i64) -> i64 {
  if n <= 0 { return 0; }
  print n;
  return countdown(n - 1);
}
```

`countdown(3)` prints `3 2 1` and then returns 0. The first
call's body decides "we're not done yet" and calls
`countdown(2)`. That call prints `2` and calls `countdown(1)`.
That prints `1` and calls `countdown(0)`. That sees `n <= 0`
and returns 0. The whole stack unwinds back to the original
caller.

Two parts every recursive function MUST have:

1. **Base case.** The "stop" condition. `countdown` has
   `if n <= 0 { return 0; }`. Without this, recursion runs
   forever (or until the stack overflows).
2. **Recursive case.** The "and now do the rest of the work
   on a smaller version" call. `countdown` returns
   `countdown(n - 1)`. The argument is *smaller* than `n`
   -- that's what makes the recursion eventually hit the
   base case.

Forget the base case -> stack overflow.
Forget to shrink the input -> stack overflow.
Both -> stack overflow.

## When recursion is the right tool

Three classic shapes where recursion reads cleaner than a loop:

### Shape 1: tree-like data

A tree of nodes (filesystem directory, AST, JSON parse tree)
has the recursive shape "a node is a value plus zero or more
child nodes." Code that walks the tree is naturally recursive:

```vani
struct Node {
  value: i64,
  children: Vec<Node>,
}

fn visit(node: ref Node) -> i64 {
  // Do something with this node's value...
  print node.value;
  // ...then recurse into each child. `ref` can only borrow a
  // named variable or a struct field, not an index expression
  // (`ref node.children[i]` doesn't parse) -- so pull each child
  // out with `clone_at` first, then borrow the local.
  let i: u64 = 0;
  while i < len(node.children) {
    let child: Node = clone_at(ref node.children, i);
    let _ = visit(ref child);
    i = i + 1;
  }
  return 0;
}
```

(Run this shape yourself: `examples/language/english/self_referential_struct_vec.vani`
in the compiler's own repo -- works on both backends.)

The shape of the code mirrors the shape of the data. Trying
to flatten this into a loop usually requires an explicit
stack -- recursion is using the call stack you already have.

### Shape 2: divide-and-conquer

Some algorithms split a problem in half, solve each half,
then combine. Merge sort, binary search, quick sort, FFT --
all natural recursive shapes:

```
sort(xs):
  if len(xs) <= 1: return xs
  mid = len(xs) / 2
  left = sort(xs[..mid])
  right = sort(xs[mid..])
  return merge(left, right)
```

Each recursive call works on half the input. The recursion
depth is `log n` (you can halve a list `log_2 n` times before
hitting 1 element). The total work across all levels is
`O(n log n)` -- that's where merge sort's complexity comes
from.

### Shape 3: mathematical recurrences

Some math is defined recursively. Factorial:
`n! = n x (n-1)!` and `0! = 1`. Fibonacci:
`fib(n) = fib(n-1) + fib(n-2)` and `fib(0) = 0, fib(1) = 1`.
The code that computes these mirrors the math directly:

```vani
fn fact(n: i64) -> i64 {
  if n <= 1 { return 1; }
  return n * fact(n - 1);
}
```

The translation from math definition to code is mechanical
when the math is recursive.

## When recursion is the wrong tool

A loop is usually faster + simpler when the problem isn't
naturally tree-shaped or divide-and-conquer:

- Counting from 1 to 100 -- use a `while` or `for`.
- Summing a Vec -- use a loop with an accumulator.
- Polling for an event -- use `while`.

Why faster? Each function call has overhead -- push a stack
frame, pass arguments, return value, pop the frame. A loop
keeps everything in the same stack frame. For tight numeric
code, the loop wins.

Why simpler? A recursive function with `n` levels uses `n`
stack frames at once. If `n` is 1 million, you've used
1 million stack frames. Modern OSes give you maybe 8 MB of
stack -- that's ~100,000 frames at most. Deep recursion
blows the stack.

## Reading a recursive function

The "trick" to reading recursion is **trusting the recursive
call**. Don't try to expand it mentally.

When you see `return countdown(n - 1)`, don't think:
> "Hmm, that calls countdown which prints, then calls
> countdown again which prints, then calls countdown again..."

Think:
> "By induction, `countdown(n - 1)` does the right thing for
> `n - 1`. So `countdown(n)` is: print `n`, then do the
> right thing for `n - 1`. That covers everything from `n`
> down to 0."

This is the same pattern as math induction:
1. **Base case**: `countdown(0)` works (prints nothing, returns 0).
2. **Inductive step**: assume `countdown(k-1)` works. Show
   `countdown(k)` works (it prints `k`, then calls the
   already-assumed-working `countdown(k-1)`).

The proof IS the program. Read the base case, read the
recursive case, and trust the recursive call.

## What goes wrong

### Stack overflow

The most common bug. Either no base case, or the base case is
unreachable for some inputs. Result: the program crashes when
the stack runs out.

<img class="manas" src="../images/mascot/manas_mascot_caution.png" title="this code needs extra care"/>

```vani
fn bad(n: i64) -> i64 {
  return bad(n - 1);  // No base case -> stack overflow.
}
```

Detection: vāṇī's `--big-o` flag reports `O(recursive)` --
the analyzer flags the call but doesn't prove termination.
Use `requires`/`ensures` SMT clauses to prove the recursion
shrinks (see [Beginner 09 SMT intro](09_smt_intro.md)).

### Exponential blowup

The recursive Fibonacci above is `O(2^n)` -- every call spawns
two more. `fib(40)` makes ~1 billion calls. Compute
`fib(100)` recursively and you'll wait until the heat death
of the universe.

<img class="manas" src="../images/mascot/manas_mascot_caution.png" title="this code needs extra care"/>

```vani
fn fib(n: i64) -> i64 {
  if n <= 1 { return n; }
  return fib(n - 1) + fib(n - 2);   // Two recursive calls!
}
```

Fix: rewrite as a loop with two accumulators, or memoize
intermediate results.

### Mutual recursion

A calls B, B calls A. vāṇī allows this but the call-graph
analyzer treats them like self-recursion. Same rules: base
case, shrinking argument. Same risks: stack overflow.

## Recursion vs loops in vāṇी

vāṇī allows both freely. Pick based on the data shape:

- Linear data, simple accumulator -> loop.
- Tree data, divide-and-conquer, recursive math -> recursion.

The compiler doesn't optimize recursion to loops automatically
(no "tail-call optimization" in v1) -- so a recursive countdown
of 1 million WILL blow the stack. When in doubt for deep
linear cases, prefer the loop.

## A summary you can carry

- **Recursion** = a function that calls itself.
- Every recursive function has a **base case** (stops the
  recursion) and a **recursive case** (calls itself on a
  smaller input).
- Use recursion for tree-shaped data, divide-and-conquer
  algorithms, and recursive math.
- Use loops for linear iteration, accumulation, and tight
  numeric code (loops are faster + don't blow the stack).
- Read recursive code by **trusting the recursive call** --
  it's induction, not iteration.
- Watch out for: no base case (stack overflow), exponential
  blowup (re-computing subproblems), and the per-call
  overhead.

The takeaway: **recursion is induction in code form.** When
the data has a recursive shape, the code that walks it has
a recursive shape too -- and the two shapes match makes the
code unusually clear.

## Cross-reference

- [Beginner 5 -- While and for loops](05_loops.md) -- the
  iterative counterpart
- [Beginner 9 -- First contract (`assert` / `prove` /
  `requires`)](09_smt_intro.md) -- proving termination of a
  recursive function via `requires n >= 0`
- [Intermediate 3a -- Box+RAII primer](../intermediate/03a_box_raii_primer.md)
  -- recursive data structures (`struct Node { next:
  Option<Box<Node>> }`) and how the compiler stores them
- [Intermediate 3d -- Cyclic references primer](../intermediate/03d_cyclic_references_primer.md)
  -- walking a parent<->child tree without recursing into
  cycles (use indices)
- [Beginner 13a -- Big-O primer](13a_big_o_primer.md) -- the
  `O(recursive)` annotation the compiler emits for self-
  recursive functions


---

**Previous**: [Sec.5 -- while and or loops ->](05_loops.md)
**Next**: [Sec.5b -- Print blocks primer ->](05b_print_block_primer.md)
