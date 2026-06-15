# Intermediate 12a — SMT, `requires`, `ensures` (intuition primer)

> **Learning goal**: build a mental model of "the compiler can
> mathematically prove things about your code." This is vāṇी's
> most distinctive feature — and the one most foreign to
> readers coming from any mainstream language. Reading order:
> this is foundational; read it before
> [Beginner 9 SMT intro](../beginner/09_smt_intro.md) for first
> contracts, then [Intermediate 12 SMT deep-dive](12_smt_deepdive.md).

This chapter has **no compiler code**. Pure intuition.

## The pitch in one sentence

> Some of the bugs that other languages catch at runtime — or
> not at all — vāṇी catches at *compile time* by proving them
> impossible.

If that sounds like magic, it isn't. It's a specific
mathematical technique called **SMT solving**. This chapter
explains what that means, and what it lets you do.

## Start with the most common bug: out-of-bounds access

Look at this snippet (any language):

```vani
let xs = [10, 20, 30];   // 3 elements
let i = compute_index();
let value = xs[i];        // ← what if i is 5? Or -1?
```

In C: undefined behavior. Maybe a crash, maybe a corrupted
read.

In Python / Java / Go: a runtime exception. Your program
crashes when it gets there, not when you wrote the code.

In Rust: a runtime panic. Same — checked at access, not
compile.

In vāṇी, *if the compiler can prove `0 ≤ i < 3` at compile
time*, the bounds-check is elided entirely. No runtime cost.
No possibility of an out-of-bounds bug. **The compiler proved
it can't happen.**

How? With a math tool called an **SMT solver**.

## What an SMT solver IS

**SMT** = **Satisfiability Modulo Theories**. The name is
academic; the idea is friendly.

An SMT solver is a program that takes a set of mathematical
statements and answers: "is there any combination of values
that makes all these statements true at once?" If yes, it
hands you the values. If no, it says "unsatisfiable" — the
statements contradict each other; no such values exist.

vāṇी uses **Z3**, a popular open-source SMT solver from
Microsoft Research. You don't write Z3 expressions directly —
the compiler translates your code into a Z3 query, asks the
solver, gets the answer, and uses it to make decisions.

### A toy example

Imagine the solver is given:
```
x > 0
x < 10
x is an integer
x * 2 = 14
```

It thinks for a moment and replies: "`x = 7` satisfies all of
those." (You can verify: 7 > 0 ✓, 7 < 10 ✓, 7×2 = 14 ✓.)

Now ask it:
```
x > 0
x < 10
x is an integer
x * 2 = 7
```

Reply: "unsatisfiable." (No integer x times 2 equals 7.)

This is the core capability the compiler uses, applied to
properties of YOUR code.

## How vāṇी uses it

You add **contracts** to your functions:

```vani
fn divide(a: i64, b: i64) -> i64
  requires b != 0;
{
  return a / b;
}
```

The `requires` clause says: "callers must promise that b is
not zero." When a caller writes `divide(100, x)`, the
compiler asks Z3:

> "Given everything I know about `x` at this point in the
> program, can `x` be 0?"

If Z3 says "no, it can't" → the call is fine.
If Z3 says "yes, it could be 0 here" → compile error: the
caller didn't satisfy the contract.

This is how vāṇी turns "this function requires a positive
number" from a comment-in-prose into a compile-time check.

## Three contract keywords

### `requires` — pre-condition on the caller

"Before you call me, this must be true."

```vani
fn sqrt_int(n: i64) -> i64
  requires n >= 0;
{
  ...
}
```

The caller has to prove `n >= 0` is true at the call site.
Inside `sqrt_int`'s body, the compiler ASSUMES `n >= 0` (it
was guaranteed by the contract).

### `ensures` — post-condition the function promises

"When I return, the result will satisfy this."

```vani
fn abs(n: i64) -> i64
  ensures _return >= 0;
{
  if n < 0 { return 0 - n; }
  return n;
}
```

`_return` is the magic name for the function's return value
in contracts. After the call, the compiler KNOWS the result
is ≥ 0 — that knowledge propagates into the caller's proof
context.

### `assert` — inline runtime check (with SMT-side proof attempt)

"Here, at this point in the body, this should be true."

```vani
fn process(xs: ref Vec<i64>) -> i64
  requires len(xs) > 0;
{
  assert xs[0] >= 0 - 100;
  return xs[0];
}
```

The compiler tries to prove the assertion at compile time. If
it succeeds → assertion is elided (zero runtime cost, like
bounds checks). If it can't prove it → the assertion becomes
a runtime check (program panics if it fails).

This is the bridge between "I'd like a compile-time guarantee"
and "I'll settle for a runtime check if you can't prove it" —
you get the same code shape either way.

## Why this matters in practice

### 1. Bounds checks vanish

```vani
fn first_three_sum(xs: ref Vec<i64>) -> i64
  requires len(xs) >= 3;
{
  return xs[0] + xs[1] + xs[2];
}
```

Three array accesses, ZERO runtime bounds checks. The compiler
proved each one is in-bounds using the `requires` clause. The
generated machine code is as fast as the C version that
*assumes* the precondition — but here the compiler made the
caller prove it.

### 2. Integer-overflow checks vanish

```vani
fn double(n: i64) -> i64
  requires n <= 1000;
{
  return n * 2;   // ← can this overflow i64? Compiler proves not.
}
```

i64 overflows at ~9 quintillion. Doubling 1000 = 2000. No
overflow possible. Compile-time proof; runtime overflow check
elided.

### 3. Divide-by-zero checks vanish

Same shape: prove `b != 0` once via `requires`, and the
runtime divide-by-zero check goes away.

### 4. Domain-specific contracts

```vani
fn pop_unchecked(xs: mut ref Vec<i64>) -> i64
  requires len(xs) > 0;
  ensures len(xs) == old_len(xs) - 1;
{
  ...
}
```

The function promises its callers: "after I return, the
length is exactly one less than it was when you called me."
Callers can use that fact in their OWN proofs further on.

## What the solver can't do

SMT solvers are powerful but not omniscient. They struggle with:

- **Unbounded loops**: proving "this while loop terminates"
  requires a loop *invariant* you supply manually.
- **Heap aliasing across function calls**: vāṇी sidesteps this
  via affine ownership, but the SMT layer doesn't reason
  about all heap shapes.
- **Floating-point edge cases**: NaN handling, denormals — the
  solver has theories for these but they get expensive.
- **Recursion without a recursion bound**: same as loops —
  needs a manual hint.

When the solver fails, you get a clear diagnostic: "couldn't
prove `xs[i] < 100` at this point; consider adding an
invariant or providing more information in the requires
clause." You're never silently left with an unverified
property.

## Doesn't this slow the compiler down?

It does — by some. SMT queries take milliseconds each, and a
modest-sized vāṇी program may issue thousands of them. Two
mitigations:

1. **The compiler is incremental**. SMT queries are cached;
   rebuilding only re-queries what changed.
2. **You can opt out**: set `VANIC_NO_VERIFY=1` for fast
   iteration; SMT is skipped entirely. Re-enable for CI builds.

For real codebases, the slowdown is similar to type-checking
overhead in a typical compiled language. The win — bugs caught
at compile time that *can't reach production* — is usually
worth several seconds of compile time.

## A summary you can carry

- **SMT** = "Satisfiability Modulo Theories." A mathematical
  tool that proves (or disproves) propositions about
  integers, reals, booleans, etc.
- vāṇी uses Z3 (Microsoft Research's SMT solver) for these
  proofs.
- **`requires`** = caller-side pre-condition.
- **`ensures`** = function-side post-condition.
- **`assert`** = mid-body claim; compiler tries to prove,
  falls back to runtime check if it can't.
- When proofs succeed, runtime checks vanish — your code runs
  AS FAST AS the C version that just assumes the precondition,
  but with no UB risk.
- The solver isn't omniscient — loops/recursion may need
  manual invariants.
- `VANIC_NO_VERIFY=1` skips SMT entirely for fast iteration.

This is the feature that most distinguishes vāṇी from Rust /
Swift / Go. It's worth investing time to get comfortable with —
the productivity gains compound.

## Cross-reference

- [Beginner 9 — First contract: assert / prove / requires](../beginner/09_smt_intro.md)
  — first hands-on contract example
- [Intermediate 12 — SMT verification deep-dive](12_smt_deepdive.md)
  — invariants, loop annotations, complex proofs
- [Intermediate 10 — Result + try](10_result_try.md) — how
  contracts compose with explicit error handling
- [Beginner 6c — Ownership primer](../beginner/06c_ownership_primer.md)
  — the OTHER compile-time-bug-prevention story (memory
  safety via ownership; SMT for arithmetic/logic safety).
