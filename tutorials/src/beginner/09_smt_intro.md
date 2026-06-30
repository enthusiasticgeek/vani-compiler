# Beginner 9 -- First contract: `assert` / `prove` / `requires`

> **Learning goal**: state preconditions with `requires`, write
> runtime invariants with `assert`, and ask the SMT verifier to
> *prove* arithmetic facts at compile time with `prove`.

Imagine a vending machine with a small sign: "Insert GBP1 or more."
That sign is a *precondition* -- it tells you what's required
BEFORE you press the button. `requires n >= 0` works the same
way: you're documenting "this function only makes sense when
`n` is non-negative," and the compiler enforces it. `assert` is
a self-check mid-function: "at this point in my recipe, the
dough MUST have risen -- crash loudly if it hasn't." `prove`
goes further: instead of checking at runtime, it asks a
mathematical solver to verify the claim is ALWAYS true, before
the code ever runs.

## The program

Save this in `~/lesson9.vani`:

```vani
intent "Lesson 9 worked example -- assert / prove / requires.";

fn double(n: i64) -> i64
requires n >= 0;
requires n <= 1000;
{
  let r: i64 = n * 2;
  assert r >= n;
  return r;
}

fn main() -> i64 {
  let x: i64 = double(7);
  assert x == 14;
  prove 2 + 2 == 4;
  print "double(7) =", x;

  let y: i64 = double(100);
  assert y == 200;
  print "double(100) =", y;
  return 0;
}
```

## Compile + run

```bash
vanic run ~/lesson9.vani
```

Expected output:

```
double(7) = 14
double(100) = 200
```

If everything works there are no SMT errors during compile.
This is the first program where you'll *feel* the verifier
working in the background.

## Why it works that way

vāṇी has **three contract keywords**, each with a different
job:

- **`requires <bool>;`** -- a *precondition* clause that goes
  between the `fn` signature and the body. The caller is
  obligated to ensure the predicate holds before calling.
  Inside the body, the verifier *assumes* the predicate is
  true. `requires n >= 0; requires n <= 1000;` above guarantees
  `n * 2` can't overflow `i64`.
- **`assert <bool>;`** -- a statement inside a function body.
  Asks the verifier to prove the predicate using everything it
  knows so far (parameters, prior `assert`s, prior `let`s). If
  the proof fails at compile time, you get a "proof failed" error
  with an SMT counterexample. If the proof succeeds, no runtime
  code is emitted -- `assert` is *free at runtime* when the SMT
  pass discharges it.
- **`prove <bool>;`** -- same shape as `assert` but for pure
  arithmetic facts you want to express explicitly. `prove 2 + 2
  == 4;` is documentation that compiles. The main practical
  difference: a failing `assert` is a runtime panic when SMT
  can't discharge it; a failing `prove` is always a compile-
  time error.

### What the SMT verifier can and can't do today

In this lesson, all four checks discharge:

| Check | Discharges because |
|---|---|
| `assert r >= n;` inside `double` | knows `n >= 0` from `requires` and `r = n * 2` |
| `assert x == 14;` in `main` | not discharged statically (see below) -- runs as a runtime check |
| `prove 2 + 2 == 4;` | pure integer arithmetic |
| `assert y == 200;` in `main` | runtime check (see L12) |

The two cross-call `assert`s above fall back to **runtime**
checking because v1's SMT encoder can't reason across function
calls without `ensures` clauses on the callee. That's
documented in
[`docs/v1_limitations.md` (L12)](https://github.com/enthusiasticgeek/vani-compiler/blob/main/docs/v1_limitations.md).

The fix is to add an `ensures` clause to `double`:

```vani
fn double(n: i64) -> i64
requires n >= 0;
requires n <= 1000;
ensures result == n * 2;
{ ... }
```

...and now the callers' `assert`s become compile-time `prove`s.
`ensures` is covered in depth in **Intermediate Sec.12 -- SMT
verification deep-dive**.

## Challenge

Add a function `safe_inc(n: i64) -> i64` that requires
`n < 1000` (so adding 1 can't overflow into anything
surprising) and returns `n + 1`. Add an internal `assert n + 1
> n;` to verify it. Call it from `main` and observe that the
verifier discharges the assert at compile time (no runtime
code is emitted for it).

<details>
<summary>Solution</summary>

```vani
fn safe_inc(n: i64) -> i64
requires n < 1000;
{
  assert n + 1 > n;
  return n + 1;
}

fn main() -> i64 {
  print safe_inc(42);
  return 0;
}
```

To prove the `assert n + 1 > n;` is discharged, run
`vanic emit ~/lesson9.vani --backend=c` and grep the C output
for the inner assert -- you'll find no `if (...) abort()` for
the `n + 1 > n` predicate. That's the SMT-elision pass in
action.

</details>

---

**Next**: [Sec.10 -- Modules and `pub` ->](10_modules.md)
