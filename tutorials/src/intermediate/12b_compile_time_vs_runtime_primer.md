# Intermediate 12b -- Compile time vs runtime (intuition primer)

> **Learning goal**: build a clear mental model of WHICH
> checks happen WHEN. vāṇī moves a lot of work to compile
> time that other languages defer to runtime; understanding
> the split is half the story of writing fast, correct
> programs. Reading order: read after [Intermediate 12a -- SMT
> primer](12a_smt_primer.md) and [Intermediate 10b -- Runtime
> errors](10b_runtime_errors_primer.md).

This chapter is mostly intuition, with real code illustrating
several compile-time-vs-runtime tradeoffs.

## The recipe and the cooking

Picture a recipe card sitting on the counter, and picture actually
standing at the stove cooking from it. These are two very different
activities, even though the second one depends on the first.

Reading the recipe card is something you can do entirely on paper,
without touching a single ingredient or turning on a single burner.
You can check: does this recipe list an oven temperature? Yes, 375
degrees, it's right there. Does it list a cook time? Yes, 25
minutes. Is the ingredient list complete, or does step 4 call for
"the sauce" when no sauce was ever listed in the ingredients? You
can catch that mistake sitting at the kitchen table with a pen,
before you've bought a single item, let alone preheated anything.
Every one of those checks is about whether the PLAN makes sense --
and you get the answer without any heat, any raw ingredient, any
risk of burning something.

Actually cooking is a completely different kind of activity. Now
you preheat the oven for real, and real ovens drift a little -- the
dial says 375 but the actual chamber might run a bit hot or a bit
cold depending on the day. Now the dish is really in there, and
whether it comes out perfectly done, underdone, or burned depends
on things the recipe card alone couldn't tell you: your particular
oven, the particular size of the potato you used, whether you got
distracted for five extra minutes. The recipe told you the PLAN was
sound; only the actual cooking tells you how it played out against
the real world, with its real variation, this one time.

Notice the recipe review can catch some mistakes completely (a
missing ingredient, a missing temperature) but it can NEVER catch
others (exactly how done the potato will be at minute 25 in YOUR
oven) -- that only shows up during the actual cooking. And notice
which kind of mistake is cheaper to catch: a typo on a recipe card
costs you nothing to fix with a pen; a dish burning in a real oven
costs you the ingredients and the time.

That's the split this chapter is about. Checking whether your
program's plan is internally consistent -- do the types match,
does every variable exist before it's used, does this function
call have the right number of arguments -- is reading-the-recipe
work: it happens once, on the source code itself, before the
program ever runs, and it's called **compile time**. Checking
something that depends on the real values flowing through the
program while it's actually executing -- is this particular array
index in bounds THIS time, did THIS division actually get a zero --
is cooking-for-real work: it happens while the program runs,
against real data, and it's called **runtime**. The rest of this
chapter is about which of vāṇī's checks happen at which stage, and
why moving a check from the stove to the recipe card, whenever you
honestly can, is almost always a win.

## Two questions

Every check the compiler enforces answers one of two
questions:

1. **"Is this even allowed?"** Asked at compile time. The
   compiler reads the source and rejects if the answer is
   no -- there's no compiled artifact at all.
2. **"Is this true right now, on this input?"** Asked at
   runtime. The check is emitted into the artifact and
   runs every time the program executes that line.

The slogan: **compile-time checks fail your build; runtime
checks fail your execution.**

## The pyramid

vāṇī layers its checks. Bottom of the pyramid: pervasive,
applies to every line. Top of the pyramid: explicit + opt-in.

```
              +--------------------------+
              |  Runtime: assert /       |  <- fires on bad input at
              |  requires not            |     the line (exit code +
              |  discharged by SMT       |     message: see Sec.10b)
              +--------------------------+
            +------------------------------+
            |  Runtime: bounds check /     |  <- fires on first bad
            |  overflow guard /            |     access (exit code +
            |  div-by-zero guard           |     message differ by
            |  (only when SMT can't prove) |     backend: see Sec.10b)
            +------------------------------+
        +--------------------------------------+
        |  Compile-time: SMT solver discharges |  <- proves contracts
        |  requires/ensures/invariant/prove    |     statically; `prove`
        |  clauses via Z3                      |     specifically has NO
        |                                      |     runtime fallback at
        |                                      |     all -- undischarged
        |                                      |     `prove` fails the
        |                                      |     BUILD, not the run
        +--------------------------------------+
    +----------------------------------------------+
    |  Compile-time: scope-escape analyzer rejects |  <- rejects
    |  refs that would outlive their source        |     dangle shapes
    +----------------------------------------------+
+-----------------------------------------------------+
|  Compile-time: type checker + affine ownership      |  <- rejects
|  + race-free `parallel for` body verifier           |     mistypes,
|                                                     |     moves, races
+-----------------------------------------------------+
```

Each layer rejects a class of mistakes that the layer above
can't see. Bottom layers fire on every program; top layers
fire only when something would otherwise crash at runtime.

## What's checked at compile time (always)

### Type checking

Every `let x: i64 = "hello";` rejects. Every `print 5 + true;`
rejects. The compiler reads each expression's types and
verifies they line up. Zero runtime cost -- the check
disappears after the build.

### Affine ownership

`let y = x; x.foo();` rejects: `x` was moved into `y`.
The compiler tracks each binding's move state through the
function body. No runtime cost -- the rejected program never
compiles, so the rejection never runs.

### Scope-escape analysis

`let xs: Vec<ref Foo> = vec(); { let f: Foo = ...; push(mut
ref xs, ref f); }` rejects: the pushed ref points at `f`,
which drops before `xs` does. The analyzer compares
declaration scopes at each ref-source site.

### Race-free `parallel for`

`parallel for i from 0 to n { print "hi"; }` rejects: `print`
is an observable side effect that would race. The effects
checker walks every parallel-for body.

### Type-validator rejections

`let xs: Vec<&T>` where `&T` doesn't satisfy the element
type constraint, struct fields with disallowed types,
function signatures with banned shapes -- all caught at
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

This elision never applies to an index expression lexically
inside a loop body (`while`, `for`, `for..in`), even when the
index is provably safe -- e.g. a `for` loop's own induction
variable. That case keeps its runtime guard unconditionally
as of 2026-08-12; see [`docs/v1_limitations.md`'s L26](https://github.com/enthusiasticgeek/vani-compiler/blob/main/docs/v1_limitations.md)
for why (a real memory-safety bug, not a missed optimization
opportunity, forced this trade-off).

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

If SMT can't discharge a `requires`, the compiler emits a
runtime check at the callee's function entry (every call
into the function is guarded, not just the specific call
site the compiler couldn't verify). As of 2026-08-07,
`ensures` works the same way: if SMT can't discharge it at
a `return`, the compiler emits a runtime check right there
instead of failing the build. `invariant` was converted the
same day: an undischarged loop invariant now gets TWO guard
sites instead of `ensures`'s one -- once at loop entry
(checked on the body's first pass only) and once for
"preservation" (checked at the natural end of every
iteration, and before any `continue` that would otherwise
jump past that check unnoticed). For all three, a genuinely
DISPROVEN clause (SMT finds a real counterexample) still
fails the build outright -- only the "SMT couldn't decide
either way" case falls back to a runtime guard. The unproven
case stays in the artifact as a guard; the proven case is
elided either way.

## The trade

**Compile-time wins**:
- Zero runtime cost (the check isn't there).
- Failures are caught BEFORE shipping -- every user gets
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
- Every check fires per-execution -- bounds checks alone
  can be 5-15% overhead on numerical code.
- Failures crash users, not developers.
- Hard to enumerate every code path that could fire.

vāṇī's design choice: **prefer compile time, fall back to
runtime when SMT can't prove**. The check is always there;
the question is whether it ran at build or at execution.

## When to push a check from runtime to compile time

Look at the emitted artifact (with `vanic emit foo.vani
--backend=c`). If you see a runtime check for an operation
you KNOW is safe, the compiler doesn't know what you know.
The fix: tell it via `requires` or `ensures`.

Example:

```vani
fn sum_first_three(xs: ref Vec<i64>) -> i64 {
  return xs[0] + xs[1] + xs[2];   // 3 bounds checks emitted
}
```

The compiler can't prove `len(xs) >= 3` from this code alone.
Add a contract:

<img class="manas" src="../images/mascot/manas_mascot_success.png" title="this is the correct, working version"/>

```vani
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

The cumulative effect of layering: hosted vāṇī programs
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
  guards, undischarged assert / requires / ensures /
  invariant. A genuinely DISPROVEN contract (a real SMT
  counterexample) still fails the build for all of these --
  only "SMT couldn't decide" falls back to a runtime guard.
- The compiler **prefers compile-time** and **falls back
  to runtime** when SMT can't prove the operation safe.
- Strengthen `requires` / `ensures` to lift checks to
  compile time. Read the emitted artifact to see which
  checks survived.
- The cumulative guarantee: hosted vāṇī programs cannot
  segfault from source. Every memory-corruption class is
  either prevented (compile time) or detected (runtime
  abort).

The takeaway: **the same operation may be checked at compile
time OR runtime, depending on how much the compiler can
prove.** Writing better contracts moves checks earlier and
makes the artifact faster -- that's the optimization story
in vāṇī.

## Cross-reference

- [Intermediate 12a -- SMT primer](12a_smt_primer.md) -- the
  proof machinery that lifts runtime checks to compile time
- [Intermediate 10b -- Runtime errors primer](10b_runtime_errors_primer.md)
  -- what happens when a runtime check DOES fire
- [Beginner 9 -- First contract](../beginner/09_smt_intro.md)
  -- your first `requires` / `prove` / `assert`
- [Beginner 13a -- Big-O primer](../beginner/13a_big_o_primer.md)
  -- the `--big-o` flag is also a compile-time analysis;
  the annotation is part of the same "prove it statically"
  story


---

**Previous**: [Sec.12a -- SMT requires / ensures primer ->](12a_smt_primer.md)
**Next**: [Sec.12 -- SMT verification deep-dive ->](12_smt_deepdive.md)

