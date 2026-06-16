# Intermediate 12 — SMT verification deep-dive

> **Learning goal**: write `ensures` postconditions, see how the
> verifier composes them across calls, and learn the
> bounds-thinking that keeps the SMT pass tractable.

> **New to this?** Read [Intermediate 12a — SMT primer](12a_smt_primer.md) and
> [Intermediate 12b — Compile time vs runtime primer](12b_compile_time_vs_runtime_primer.md) first.

Imagine a legal contract: the buyer promises to pay (precondition,
`requires`) and the seller promises to deliver by Friday
(postcondition, `ensures`). An SMT verifier reads both promises
and asks a solver to check if there's ANY possible input that
could make the seller break their promise. If the solver finds
one, compilation fails with a proof obligation that names the
counterexample. If the solver can't find one, the contract is
mathematically discharged — not just tested, *proven*.

## The program

```vani
intent "Intermediate 12 worked example — SMT contracts in depth.";

// `ensures _return >= 0;` is a postcondition the SMT verifier
// proves at every return. The special `_return` name refers to
// the value being returned.
fn checked_sub(a: i64, b: i64) -> i64
requires a >= b;
requires b >= 0;
requires a <= 1000000000;
ensures _return >= 0;
{
  return a - b;
}

// Callers reuse the callee's ensures: the verifier knows
// `_return >= 0`, so the assert below discharges at compile
// time without runtime code.
fn use_it(x: i64) -> i64
requires x >= 5;
requires x <= 100;
{
  let y: i64 = checked_sub(x, 3);
  assert y >= 0;
  return y;
}

// Stronger postcondition: result >= input for non-negative
// inputs through the if arm; equal to input through the else.
fn double_if_positive(x: i64) -> i64
requires x < 1000000000;
requires x > 0 - 1000000000;
ensures _return >= x;
{
  if x > 0 {
    return x + x;
  }
  return x;
}

fn main() -> i64 {
  print "checked_sub(20, 7)    =", checked_sub(20, 7);
  print "use_it(5)             =", use_it(5);
  print "double_if_positive(7) =", double_if_positive(7);
  return 0;
}
```

## Compile + run

```bash
vanic run ~/int12.vani
```

Output:

```
checked_sub(20, 7)    = 13
use_it(5)             = 2
double_if_positive(7) = 14
```

The interesting part isn't the runtime output — it's the
*compile-time proofs*. The `assert y >= 0;` inside `use_it`
discharges statically: the verifier inlines `checked_sub`'s
`ensures _return >= 0;` at the call site and sees that `y >=
0` follows directly.

## What `_return` is

The special name `_return` refers to the value being returned
by the function. It's the only valid reference to "the result"
in an `ensures` clause; you don't write `result`, you don't
write the literal `return` keyword in a predicate.

```vani
ensures _return >= 0;
ensures _return == n * 2;
ensures _return > x;
```

## Bounds-thinking: why the requires soup

Notice every `requires` in `checked_sub`:

- `a >= b` so `a - b` is non-negative.
- `b >= 0` so the underflow check makes sense.
- `a <= 1000000000` so `a - b` can't overflow even if `b` is
  `i64::MIN`.

That last one is the one that catches everyone first. Without
`a <= 1000000000`, the SMT solver finds a counterexample:
`a = i64::MAX`, `b = i64::MIN`, and `a - b` wraps. v1's
encoder treats `i64` as **two's-complement bounded
arithmetic**; you have to constrain inputs into a window where
the operation can't overflow.

The good news: this is the *same constraint* that production
software needs — just made explicit.

## Where SMT can and can't help

| Works today | Doesn't yet |
|---|---|
| Integer + bool arithmetic | Float arithmetic |
| Comparisons (`<`, `<=`, `==`, …) | String predicates |
| Quantifier-free `requires` / `ensures` | Existential / forall quantifiers |
| `assert` in straight-line code | Recursion across the call boundary without `ensures` |
| Loop body with `invariant` | Loops without an invariant |
| Cross-function reasoning via `ensures` | Polymorphic generic bodies (each instantiation re-verifies) |

The big gap to be aware of: **the v1 encoder can't reason
across a function call unless the callee carries an
`ensures` clause** ([L12 in v1_limitations.md](https://github.com/anthropics/claude-code/blob/main/docs/v1_limitations.md)).
Beginner §9 covered the case where a `prove y == double(7)`
fails for this reason; here, the *same* call site succeeds
because `double_if_positive` (and `checked_sub`) carry the
needed `ensures`.

## How to debug a failing proof

Run with `VANIC_SMT_DEBUG=1`:

```bash
VANIC_SMT_DEBUG=1 vanic check ~/int12.vani
```

The compiler prints every SMT query and Z3's response to stderr.
A "proof failed" diagnostic includes the **counterexample** —
the assignment to free variables under which the predicate is
false. Use it to refine your `requires` clause.

## Challenge

Add `ensures _return == x` for the `x <= 0` branch in
`double_if_positive` (you'll need an *additional* `ensures`
arm, or rewrite the body to make the postcondition follow
syntactically). Verify the new postcondition is provable.

Then try the opposite: relax `requires x < 1000000000` to
`requires x < i64::MAX`. Observe the SMT counterexample. Note
which bound it picks.

---

**Congratulations — you've completed the Intermediate track!**

Next steps:

- **[Advanced track](../advanced/01_async.md)** — async / await,
  parallel-for + race-freedom, embedded targets, vtable
  internals, dialect contribution.
- Build something. Pick a small CLI or library, ship it, file
  issues for anything that pushed back. The compiler's most
  honest design feedback is from real programs.
