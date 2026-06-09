# Intermediate 12b — Compile time vs runtime (intuition primer)

> **Learning goal**: build a clear mental model of WHICH
> checks happen WHEN. vāṇी moves a lot of work to compile
> time that other languages defer to runtime; understanding
> the split is half the story of writing fast, correct
> programs. Reading order: read after [Intermediate 12a — SMT
> primer](12a_smt_primer.md) and [Intermediate 10b — Runtime
> errors](10b_runtime_errors_primer.md).

This chapter has **no compiler code**. Pure intuition.

## Two questions

Every check the compiler enforces answers one of two
questions:

1. **"Is this even allowed?"** Asked at compile time. The
   compiler reads the source and rejects if the answer is
   no — there's no compiled artifact at all.
2. **"Is this true right now, on this input?"** Asked at
   runtime. The check is emitted into the artifact and
   runs every time the program executes that line.

The slogan: **compile-time checks fail your build; runtime
checks fail your execution.**

## The pyramid

vāṇी layers its checks. Bottom of the pyramid: pervasive,
applies to every line. Top of the pyramid: explicit + opt-in.

```
              ┌──────────────────────────┐
              │  Runtime: assert /       │  ← fires abort() on bad input
              │  prove / requires not    │     at the failing line
              │  discharged by SMT       │
              └──────────────────────────┘
            ┌──────────────────────────────┐
            │  Runtime: bounds check /     │  ← fires abort() on first
            │  overflow guard /            │     out-of-bounds access
            │  div-by-zero guard           │
            │  (only when SMT can't prove) │
            └──────────────────────────────┘
        ┌──────────────────────────────────────┐
        │  Compile-time: SMT solver discharges │  ← proves
        │  requires/ensures/invariant clauses  │     contracts statically
        │  via Z3                              │
        └──────────────────────────────────────┘
    ┌──────────────────────────────────────────────┐
    │  Compile-time: scope-escape analyzer rejects │  ← rejects
    │  refs that would outlive their source        │     dangle shapes
    └──────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────┐
│  Compile-time: type checker + affine ownership      │  ← rejects
│  + race-free `parallel for` body verifier           │     mistypes,
│                                                     │     moves, races
└─────────────────────────────────────────────────────┘
```

Each layer rejects a class of mistakes that the layer above
can't see. Bottom layers fire on every program; top layers
fire only when something would otherwise crash at runtime.

## What's checked at compile time (always)

### Type checking

Every `let x: i64 = "hello";` rejects. Every `print 5 + true;`
rejects. The compiler reads each expression's types and
verifies they line up. Zero runtime cost — the check
disappears after the build.

### Affine ownership

`let y = x; x.foo();` rejects: `x` was moved into `y`.
The compiler tracks each binding's move state through the
function body. No runtime cost — the rejected program never
compiles, so the rejection never runs.

### Scope-escape analysis

`let xs: Vec<ref Foo> = vec(); { let f: Foo = ...; push(mut
ref xs, ref f); }` rejects: the pushed ref points at `f`,
which drops before `xs` does. The analyzer compares
declaration scopes at each ref-source site.

### Race-free `parallel for`

`parallel for i in 0..n { print "hi"; }` rejects: `print`
is an observable side effect that would race. The effects
checker walks every parallel-for body.

### Type-validator rejections

`let xs: Vec<&T>` where `&T` doesn't satisfy the element
type constraint, struct fields with disallowed types,
function signatures with banned shapes — all caught at
parse + check time.

## What's checked at runtime (only when needed)

### Bounds checks

`xs[i]` may be in-bounds or out-of-bounds depending on
runtime values of `i` and `len(xs)`. The compiler emits an
`if (i >= len(xs)) abort();` guard at every index.

**Exception**: when SMT can prove `i < len(xs)` from the
surrounding context (an enclosing `requires` clause, an
`if i < len(xs)` guard already in source, a constant
index on a known-length array), the guard is **elided**.
The runtime check disappears from the artifact.

### Integer overflow

`a + b` may overflow if `a` and `b` are close to `i64::MAX`.
The compiler emits a check. SMT-discharge elides it when
the surrounding `requires` clause proves the operands are
bounded.

### Divide-by-zero

`a / b` may divide by zero. Same shape: runtime guard
unless SMT discharges from `requires b != 0`.

### `assert` / `prove`

`assert n > 0;` always emits a runtime check that fires
`abort()` if false. `prove n > 0;` is the strict form:
SMT MUST discharge it at build time, OR the build fails.
The two are different escape hatches at different ends of
the discharge-confidence spectrum.

### `requires` / `ensures` / `invariant` (at the failing site)

If SMT can't discharge a `requires` at a call site, the
compiler emits a runtime check at the call site. If SMT
can't discharge an `ensures` at a return, the compiler
emits a runtime check at the return. Same for `invariant`
at loop entry / body / exit. The unproven case stays in
the artifact as a guard; the proven case is elided.

## The trade

**Compile-time wins**:
- Zero runtime cost (the check isn't there).
- Failures are caught BEFORE shipping — every user gets
  the same correct binary.
- The compiler proves correctness once; you never have to
  test that particular failure mode.

**Compile-time costs**:
- Slower builds (the compiler does more work).
- Some programs that would "work at runtime" are rejected
  (the analyzer is conservative).
- Strengthening contracts to discharge checks adds source-
  level noise.

**Runtime wins**:
- Programs compile faster (less analysis).
- More flexibility (data that fits a contract at runtime
  doesn't have to fit one at compile time).

**Runtime costs**:
- Every check fires per-execution — bounds checks alone
  can be 5-15% overhead on numerical code.
- Failures crash users, not developers.
- Hard to enumerate every code path that could fire.

vāṇी's design choice: **prefer compile time, fall back to
runtime when SMT can't prove**. The check is always there;
the question is whether it ran at build or at execution.

## When to push a check from runtime to compile time

Look at the emitted artifact (with `vanic emit foo.vani
--backend=c`). If you see a runtime check for an operation
you KNOW is safe, the compiler doesn't know what you know.
The fix: tell it via `requires` or `ensures`.

Example:

```rust
fn sum_first_three(xs: ref Vec<i64>) -> i64 {
  return xs[0] + xs[1] + xs[2];   // 3 bounds checks emitted
}
```

The compiler can't prove `len(xs) >= 3` from this code alone.
Add a contract:

```rust
fn sum_first_three(xs: ref Vec<i64>) -> i64
  requires len(xs) >= 3
{
  return xs[0] + xs[1] + xs[2];   // 0 bounds checks emitted
}
```

Now the contract IS the proof. The runtime checks are gone.
The contract is checked at every CALLER's call site instead
(once per site, not once per index).

## When to leave a check at runtime

The opposite move: when the value genuinely depends on input
that can be anything, keep the runtime check. Trying to
strengthen `requires` past the point of being honest is a
worse trade than the 5% overhead.

Example: a function reading bytes from a network packet has
no static knowledge of what the packet contains. Bounds
checks on the parse stay; their cost is rounding error next
to the syscall.

## The vāṇी payoff

The cumulative effect of layering: hosted vāṇी programs
**cannot segfault from source**. Every memory-corruption
class is either rejected at compile time (affine, scope-
escape, no raw pointers) OR caught at runtime with a clean
abort + diagnostic (bounds check, overflow guard, assert
fire). There is no path from valid source to undefined
behavior.

That's the trade for the longer compile time + the
sometimes-strict contracts: a category of bug that doesn't
happen.

## A summary you can carry

- **Compile time**: type check, affine ownership, scope-
  escape, parallel-for race-freedom, SMT discharge of
  contracts.
- **Runtime**: bounds checks, overflow / div-by-zero
  guards, undischarged assert / prove / requires /
  ensures / invariant.
- The compiler **prefers compile-time** and **falls back
  to runtime** when SMT can't prove the operation safe.
- Strengthen `requires` / `ensures` to lift checks to
  compile time. Read the emitted artifact to see which
  checks survived.
- The cumulative guarantee: hosted vāṇी programs cannot
  segfault from source. Every memory-corruption class is
  either prevented (compile time) or detected (runtime
  abort).

The takeaway: **the same operation may be checked at compile
time OR runtime, depending on how much the compiler can
prove.** Writing better contracts moves checks earlier and
makes the artifact faster — that's the optimization story
in vāṇी.

## Cross-reference

- [Intermediate 12a — SMT primer](12a_smt_primer.md) — the
  proof machinery that lifts runtime checks to compile time
- [Intermediate 10b — Runtime errors primer](10b_runtime_errors_primer.md)
  — what happens when a runtime check DOES fire
- [Beginner 9 — First contract](../beginner/09_smt_intro.md)
  — your first `requires` / `prove` / `assert`
- [Beginner 13a — Big-O primer](../beginner/13a_big_o_primer.md)
  — the `--big-o` flag is also a compile-time analysis;
  the annotation is part of the same "prove it statically"
  story
