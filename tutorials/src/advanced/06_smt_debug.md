# Advanced 6 -- SMT trace debugging

> **Learning goal**: when a `prove` / `assert` / loop
> `invariant` fails, get the SMT solver to show its work and
> turn the counterexample into a fix.

When an SMT proof fails, the solver found a specific input that
BREAKS your contract. Think of it like a QA tester who hands
you a bug report: "I gave your add function `a = -1, b = 2`
and `ensures _return >= 0` was false." The solver's
counterexample IS that bug report. This chapter shows you how
to ask the solver to print the counterexample in human-readable
form, and how to use it to either tighten the contract
(`requires a >= 0`) or fix the code (handle the negative case
explicitly).

## Turning on the trace

Set the env var, then re-run any compile command:

```bash
VANIC_SMT_DEBUG=1 vanic check ~/myprog.vani 2> smt.log
```

Every SMT query the verifier emits -- for `prove`, `requires` at
call sites, `ensures` at returns, and loop invariants -- is
written to stderr in two parts. Note: plain `assert` is NOT in
this list -- confirmed by testing, and by reading
`checker.rs`'s `Stmt::Assert` handling, which never calls into
the SMT layer at all. An `assert` always compiles down to a
runtime check, unconditionally, regardless of whether the
condition happens to be statically provable; only `prove` gets a
compile-time SMT attempt. (This is unrelated to `assert`'s
*optional* `, "message"` form -- that's just a custom panic
string, still runtime-only.)

1. **The query** in SMT-LIB v2 format, ready to paste into a
   standalone `z3` invocation.
2. **The response** -- either `unsat` (proof discharged) or a
   `sat` model showing the counterexample assignment.

The compiler buffers nothing; you can `tail -f smt.log` while
compiling for long files.

## Reading a counterexample

A typical failing `prove` looks like (confirmed by testing; an
earlier version of this page showed `assert x - b >= 0;` here,
but plain `assert` never produces this diagnostic shape -- see
the note above -- only `prove` does):

<img class="manas" src="../images/mascot/manas_mascot_error.png" title="this code does not compile!"/>

```
src/foo.vani:42:5: error: proof failed: SMT counterexample [x = 9223372036854775807, b = -9223372036854775808]
  prove x - b >= 0;
        ^^^^^^^^^^
```

The bracketed `[x = ..., b = ...]` is the **smallest set of
free variables** the solver could pick to make the predicate
false. Read it as "if `x = i64::MAX` and `b = i64::MIN`, then
`x - b` overflows and the inequality doesn't hold."

The fix is usually a tightened `requires` clause. From
Intermediate Sec.12:

<img class="manas" src="../images/mascot/manas_mascot_success.png" title="this is the correct, working version"/>

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
callee's `requires` -- failure mode is a "callee precondition
not met" error.

**Loop invariant too weak.** The verifier needs the invariant
to be:
- True at loop entry.
- Preserved by the body (assume invariant + body produces
  invariant again at the back edge).
- Strong enough at exit to prove the post-loop goal.

A counterexample at "loop invariant is not preserved by the
loop body" tells you the inductive step fails -- usually you
need an *additional* invariant that constrains some other
variable's relationship.

**Function call in `prove` without `ensures` on the callee.**
v1's L12: the encoder can't reason across opaque calls.
Either add an `ensures` to the callee, or convert the `prove`
to a runtime `assert`.

## Tactics for hard proofs

<img class="manas" src="../images/mascot/manas_mascot_awesome.png" title="a good habit worth adopting"/>

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

The real query block for `checked_sub`'s `ensures _return >= 0;`
check looks like this (confirmed by testing with
`VANIC_SMT_DEBUG=1`; an earlier version of this page showed
unbounded `Int` declarations and plain `>=`/`-` operators, which
is NOT how this encoder actually works):

```
(set-logic ALL)
(declare-const v_a (_ BitVec 64))
(declare-const v_b (_ BitVec 64))
(assert (bvsge v_a v_b))
(assert (bvsge v_b (_ bv0 64)))
(assert (bvsle v_a (_ bv1000000000 64)))
(assert (not (bvsge (bvsub v_a v_b) (_ bv0 64))))
(check-sat)
(get-model)
```

Every integer type is encoded as a fixed-width `BitVec`, and
arithmetic uses the `bv`-prefixed bitvector operators (`bvsge`,
`bvsub`, ...) -- not mathematical `Int`. This matters: it means
the solver faithfully models wraparound overflow the same way the
generated C/LLVM code does at runtime, rather than treating `-`
as unbounded-precision subtraction. That's *why* the three
`requires` clauses in `checked_sub` are load-bearing -- without
`a <= 1000000000`, the solver can pick `a` near `i64::MAX` and
`b` near `i64::MIN`, and `a - b` genuinely wraps below zero in
64-bit two's-complement arithmetic, which is exactly the
`[x = 9223372036854775807, b = -9223372036854775808]`
counterexample shown earlier.

`unsat` means the goal is provable -- there's no model that
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

(Recursion *with* an `ensures` works, but the `ensures` doubles as an
induction hypothesis for the recursive call -- see [Intermediate 12's
"Recursive and reentrant calls"](../intermediate/12_smt_deepdive.md#recursive-and-reentrant-calls)
for the mechanics and a worked example.)

When you hit a boundary, the answer is usually "rewrite the
predicate in the supported subset" -- e.g. replace a string
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

**Previous**: [Sec.5b -- Advanced collections ->](05b_advanced_collections.md)
**Next**: [Sec.7 -- Devanagari purity arc ->](07_devanagari_purity.md)
