# Intermediate 12 -- SMT verification deep-dive

> **Learning goal**: write `ensures` postconditions, see how the
> verifier composes them across calls, and learn the
> bounds-thinking that keeps the SMT pass tractable.

> **New to this?** Read [Intermediate 12a -- SMT primer](12a_smt_primer.md) and
> [Intermediate 12b -- Compile time vs runtime primer](12b_compile_time_vs_runtime_primer.md) first.

Imagine a legal contract: the buyer promises to pay (precondition,
`requires`) and the seller promises to deliver by Friday
(postcondition, `ensures`). An SMT verifier reads both promises
and asks a solver to check if there's ANY possible input that
could make the seller break their promise. If the solver finds
one, compilation fails with a proof obligation that names the
counterexample. If the solver can't find one, the contract is
mathematically discharged -- not just tested, *proven*.

## The program

```vani
intent "Intermediate 12 worked example -- SMT contracts in depth.";

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

The interesting part isn't the runtime output -- it's the
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
software needs -- just made explicit.

Drop the bounding `requires` clauses down to just `a >= b` and
the solver finds exactly that kind of counterexample -- an `a`
near `i64::MAX` paired with a `b` near `i64::MIN` -- and refuses
to compile the function:

<img class="manas" src="../images/mascot/manas_mascot_error.png" title="this code does not compile!"/>

```vani
fn checked_sub_unbounded(a: i64, b: i64) -> i64
requires a >= b;
ensures _return >= 0;
{
  return a - b;
}

fn main() -> i64 {
  print "checked_sub_unbounded(20, 7) =", checked_sub_unbounded(20, 7);
  return 0;
}
```

```
error: function 'checked_sub_unbounded' ensures clause does not hold at this return
       [counterexample: a = 9223372036854775806, b = -9223372032559808512]
ensures _return >= 0;
        ^^^^^^^^^^^^
```

Without a lower bound on `b` (`b >= 0`), nothing stops `b` from
being deeply negative -- and `a - b` with a very negative `b`
behaves like `a + |b|`, which overflows past `i64::MAX` for the
solver's counterexample assignment. The `ensures _return >= 0;`
can no longer be discharged, so the function is rejected.

## Where SMT can and can't help

| Works today | Doesn't yet |
|---|---|
| Integer + bool arithmetic | Float arithmetic |
| Comparisons (`<`, `<=`, `==`, ...) | String predicates |
| Quantifier-free `requires` / `ensures` | Existential / forall quantifiers |
| `assert` in straight-line code | Recursion across the call boundary without `ensures` |
| Loop body with `invariant` | Loops without an invariant |
| Cross-function reasoning via `ensures` | Polymorphic generic bodies (each instantiation re-verifies) |
| Struct field access (`p.x`) in `requires`/`ensures`/`prove`, for any struct-typed binding (a literal-init local, a `ref` parameter, ...) | A `Vec<Struct>`/`Array<Struct,N>` element's field (`xs[i].x`) -- array theory only models scalar `Vec`/`Array` elements |

The big gap to be aware of: **the v1 encoder can't reason
across a function call unless the callee carries an
`ensures` clause** ([L12 in v1_limitations.md](https://github.com/enthusiasticgeek/vani-compiler/blob/main/docs/v1_limitations.md)).
Beginner Sec.9 covered the case where a `prove y == double(7)`
fails for this reason; here, the *same* call site succeeds
because `double_if_positive` (and `checked_sub`) carry the
needed `ensures`.

## Recursive and reentrant calls

The verifier never inlines or unrolls a callee's body -- not for an
ordinary call, and not for a recursive one either. Every call site is
checked the same way: substitute the actual arguments into the
callee's *declared* `requires`/`ensures` and reason from that
signature alone. A self-call, a mutual-recursion call, or a call back
into a function that's currently being checked all look identical to
the verifier -- it only ever consults the callee's contract, never its
body. There's no "currently verifying" stack and no recursion-depth
limit in the checker, because none is needed: a call site never
re-descends into a callee's body, so the checker's own logic can't
loop just because your program's call graph does.

One consequence: on a recursive call, the callee's `ensures` is
*assumed* as a fact -- which, for the recursive case, means it's
assumed about the very call you're in the middle of proving. That's
an induction hypothesis, and like any induction proof, it only works
if the hypothesis is strong enough to carry the inductive step. Watch
what happens with a postcondition that's true, but too weak:

```vani
fn sum_to(n: i64) -> i64
requires n >= 0;
requires n <= 1000;
ensures _return >= 0;
{
  if n == 0 {
    return 0;
  }
  return n + sum_to(n - 1);
}
```

```
error: function 'sum_to' ensures clause does not hold at this return
       [counterexample: n = 1000, sum_to((n - 1)) = 9223372036854775800]
ensures _return >= 0;
        ^^^^^^^^^^^^
```

`_return >= 0` is true of `sum_to` -- but it's the *only* thing the
solver is allowed to assume about `sum_to(n - 1)` at the recursive
call site, so it's free to pick `9223372036854775800` (any
non-negative value) as that call's result, and `n +
9223372036854775800` overflows `i64`. This is direct evidence the
verifier isn't inlining: if it were substituting the real recursive
computation, no such counterexample would exist -- `sum_to(999)` is
actually `499500`, nowhere near overflow. The counterexample is an
artifact of assume-guarantee reasoning, not a real bug in `sum_to`.

The fix is a tighter `ensures` -- one that bounds growth enough for
the inductive step to go through:

```vani
fn sum_to(n: i64) -> i64
requires n >= 0;
requires n <= 1000;
ensures _return >= 0;
ensures _return <= n * 1000000;
{
  if n == 0 {
    return 0;
  }
  return n + sum_to(n - 1);
}
```

Now the induction step is: assuming `sum_to(n - 1) <= (n - 1) *
1000000` (the hypothesis, for the recursive call), prove `n +
sum_to(n - 1) <= n * 1000000`. That's linear arithmetic Z3 discharges
easily, so this version compiles and runs -- `sum_to(5)` still
correctly returns `15`. The lesson generalizes: a recursive function's
`ensures` isn't just documentation of its result, it's the exact fact
budget every recursive call site has to work with, so it needs to be
tight enough to prove itself from itself.

If you want to forbid recursion outright rather than verify it,
`#[no_recursion]` (see [Advanced 12 -- Safety
standards](../advanced/12_safety_standards.md)) rejects any recursive
call at compile time via a call-graph cycle check -- a completely
separate mechanism from the SMT contract machinery above, with no
attempt to prove anything about what the recursion computes.

### Is this fast? (Big-O of the SMT pass)

Because a call site never re-examines the callee's body, the checker's
cost **does not scale with recursion depth or call-graph shape** --
only with the size of the AST actually being walked. Concretely:

- Each function's fact generation is one structural walk over that
  function's own body -- O(nodes in that function), full stop, whether
  or not it's recursive.
- Across the whole program, the checker's own control flow visits each
  function exactly once (`checker.rs:1787`), so the pass is O(total
  AST size) overall, not exponential in the call graph.
- Each `assert`/`ensures`/`requires` obligation issues one Z3 query
  built from the facts accumulated so far, so per function the total
  query text is roughly quadratic in statement count in the worst
  case -- not driven by recursion at all.
- Each Z3 call is capped at a 5-second wall-clock timeout, and queries
  are cached by exact text so an incremental rebuild skips proofs
  whose facts didn't change.

The same "why is this slow" mitigations from [Sec.12a](12a_smt_primer.md#doesnt-this-slow-the-compiler-down)
apply here unchanged: `VANIC_NO_VERIFY=1` for fast local iteration,
full verification for CI.

## How to debug a failing proof

Run with `VANIC_SMT_DEBUG=1`:

```bash
VANIC_SMT_DEBUG=1 vanic check ~/int12.vani
```

The compiler prints every SMT query and Z3's response to stderr.
A "proof failed" diagnostic includes the **counterexample** --
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

**Congratulations -- you've completed the Intermediate track!**

Next steps:

- **[Advanced track](../advanced/01_async.md)** -- async / await,
  parallel-for + race-freedom, embedded targets, vtable
  internals, dialect contribution.
- Build something. Pick a small CLI or library, ship it, file
  issues for anything that pushed back. The compiler's most
  honest design feedback is from real programs.


---

**Previous**: [Sec.12b -- Compile time vs runtime primer ->](12b_compile_time_vs_runtime_primer.md)
**Next**: [Sec.16 -- Packages with Kosh ->](16_packages.md)

