# Advanced 6 — SMT trace debugging

> **Learning goal**: when a `prove` / `assert` / loop
> `invariant` fails, get the SMT solver to show its work and
> turn the counterexample into a fix.

## Turning on the trace

Set the env var, then re-run any compile command:

```bash
VANIC_SMT_DEBUG=1 vanic check ~/myprog.vani 2> smt.log
```

Every SMT query the verifier emits — for every `prove`,
`assert`, `requires` at call sites, `ensures` at returns, and
loop invariants — is written to stderr in two parts:

1. **The query** in SMT-LIB v2 format, ready to paste into a
   standalone `z3` invocation.
2. **The response** — either `unsat` (proof discharged) or a
   `sat` model showing the counterexample assignment.

The compiler buffers nothing; you can `tail -f smt.log` while
compiling for long files.

## Reading a counterexample

A typical failing assert looks like:

```
src/foo.vani:42:5: error: proof failed: SMT counterexample [x = 9223372036854775807, b = -9223372036854775808]
  assert x - b >= 0;
         ^^^^^^^^^^^
```

The bracketed `[x = ..., b = ...]` is the **smallest set of
free variables** the solver could pick to make the predicate
false. Read it as "if `x = i64::MAX` and `b = i64::MIN`, then
`x - b` overflows and the inequality doesn't hold."

The fix is usually a tightened `requires` clause. From
Intermediate §12:

```vani
fn checked_sub(a: i64, b: i64) -> i64
requires a >= b;
requires b >= 0;           // <- adds upper bound on the subtraction
requires a <= 1000000000;  // <- prevents the i64::MAX edge case
ensures _return >= 0;
{ ... }
```

## Common failure shapes

**Overflow in arithmetic.** Add `requires` bounds that keep
operands inside the safe window.

**Forgot to assume `requires` of a callee.** v1's encoder
inlines `ensures` clauses across calls, but **not**
`requires`. The caller must pass values that satisfy the
callee's `requires` — failure mode is a "callee precondition
not met" error.

**Loop invariant too weak.** The verifier needs the invariant
to be:
- True at loop entry.
- Preserved by the body (assume invariant + body produces
  invariant again at the back edge).
- Strong enough at exit to prove the post-loop goal.

A counterexample at "loop invariant is not preserved by the
loop body" tells you the inductive step fails — usually you
need an *additional* invariant that constrains some other
variable's relationship.

**Function call in `prove` without `ensures` on the callee.**
v1's L12: the encoder can't reason across opaque calls.
Either add an `ensures` to the callee, or convert the `prove`
to a runtime `assert`.

## Tactics for hard proofs

- **Add an intermediate `assert`**. If the failing predicate
  is complex, split it: `assert P;` then `assert P && Q;`.
  The first one discharges; the second pinpoints which
  conjunct fails.
- **Bound your inputs aggressively at first**. A `requires x
  <= 1000` may not be realistic, but it tells you whether the
  proof would *ever* succeed. If yes, relax the bound.
- **Convert to `prove`**. `prove EXPR;` is `assert EXPR;` but
  always compile-time. Sometimes the verifier needs the
  guarantee of *no fallback to runtime* to commit to the
  proof.

## Reading the SMT-LIB output

The query block looks like:

```
(declare-const x Int)
(declare-const b Int)
(assert (>= x b))
(assert (>= b 0))
(assert (<= x 1000000000))
(assert (not (>= (- x b) 0)))    ; the negation of the goal
(check-sat)
```

`unsat` means the goal is provable — there's no model that
satisfies the negated goal under the assumptions. `sat` plus a
`(get-model)` block is the counterexample.

Paste the block into a standalone `z3` invocation to iterate
faster than running the whole compiler:

```bash
z3 smt.log    # straight feed
echo "(declare-const ...) ..." | z3 -in    # one-off
```

## The verifier's encoding boundary

The v1 encoder handles:

- Integer + bool arithmetic with overflow modeling.
- Quantifier-free predicates over let-bindings.
- `if`/`else` paths and `match` arms.
- Function-call `ensures` inlining.
- Loop invariants (entry / preservation / exit).

It doesn't handle yet:

- Float arithmetic.
- Strings (`Str` / `OwnedStr` predicates).
- Quantifiers (`forall i. xs[i] > 0`).
- Recursion across calls without `ensures`.

When you hit a boundary, the answer is usually "rewrite the
predicate in the supported subset" — e.g. replace a string
length check with the `u64` return of `len(s)`.

## Worked debugging session

See `examples/language/english/contracts.vani` and
`examples/language/english/inline_call_proofs.vani` for end-
to-end programs that demonstrate the SMT pass discharging
non-trivial proofs.

## Challenge

Write a function that the verifier can't prove correct, then
incrementally add `requires` clauses until it does. Run with
`VANIC_SMT_DEBUG=1` each iteration and observe how the
counterexample narrows.

---

**Next**: [§7 — Devanagari purity arc →](07_devanagari_purity.md)
