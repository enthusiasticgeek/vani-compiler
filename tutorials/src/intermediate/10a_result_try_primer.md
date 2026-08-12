# Intermediate 10a -- Result, `try`, and `?` (intuition primer)

> **Learning goal**: build a mental model of "errors as
> values" -- the alternative to exceptions that vāṇी (and Rust)
> use. Why this design choice, and how the `try` keyword + `?`
> operator make it ergonomic in practice. Reading order:
> [04a dyn primer](04a_dyn_iface_primer.md) and
> [02_enums_payloads](02_enums_payloads.md) help if you haven't
> met payloaded enums yet; then this; then
> [Intermediate 10 Result + try](10_result_try.md).

This chapter is mostly intuition, with real code illustrating each
pattern.

## The relay race

Picture a 4x100 meter relay team. Four runners, one baton. The
rules are simple but strict: each runner sprints their leg, then
hands the baton to the next runner inside a small marked zone. If
the handoff succeeds, the next runner takes off immediately. If the
handoff FAILS -- the baton is dropped, or fumbled outside the zone
-- the race for that team is over, right there, on the spot. Nobody
two legs downstream says "well, I didn't hear about the drop, I'll
just run my leg anyway with an imaginary baton." The whole team's
run stops at the exact leg where it went wrong, and the official
reports precisely which handoff failed.

Notice what does NOT happen. Runner 3 doesn't finish their leg,
arrive at the exchange zone, and stand there for ten minutes
wondering whether runner 2 is coming. Runner 4 doesn't take a guess
and start running with nothing in their hand, hoping there's a
baton. The check happens at every single handoff, automatically, as
a condition of moving forward -- if the baton isn't successfully in
the next runner's hand, the race does not continue to the next leg.

Also notice that a *successful* handoff is completely uneventful.
The baton passes, the next runner is already moving, nobody blows a
whistle, nobody stops to look around -- the race just continues at
full speed as if nothing needed checking at all. It's only on
failure that anything visible happens.

Now map that onto a chain of function calls. Each function in the
chain is a runner. What it returns -- succeed with a result, or
fail with a reason -- is the baton handoff. A chain like "parse the
input, then validate it, then look it up, then compute the answer"
is a relay: parse hands off to validate, validate hands off to
lookup, lookup hands off to compute. If ANY leg fails, you don't
want the rest of the chain to keep running on a value that was
never successfully produced -- you want the whole chain to stop
immediately, at that exact leg, and report which one failed.

That's exactly what `Result<T, E>` plus `try` / `?` give you.
`Result<T, E>` is the handoff report for one leg -- either "here's
the baton, cleanly passed" (`Ok(value)`) or "dropped it, here's why"
(`Err(error)`). The `?` (or `try`) is the automatic official
standing at every exchange zone: if the handoff succeeded, the next
runner goes; if it failed, the whole relay stops right there and
reports the failing leg -- automatically, without you writing "did
this handoff succeed? let me check..." by hand after every single
call.

## The problem: things go wrong

Programs encounter failure constantly:
- A file you tried to open doesn't exist.
- A network request timed out.
- A parser hit malformed input.
- A division by zero would happen.
- A division by an unknown value (the caller's input) MIGHT
  be zero -- you don't know yet.

How does a function signal "I couldn't do my job"? Two
philosophies have dominated:

### Philosophy 1: exceptions (Python, Java, C++, JavaScript)

A failing function "throws". Control flow jumps to whoever
"catches" it, possibly several call frames up. The normal
return path is for success; failure is a different return
path that unwinds the stack until caught.

Pros:
- Calling code looks clean. Errors handled "out of band".
- Stack traces are automatic.

Cons:
- The control flow is invisible at call sites. `f()` MIGHT
  return, or MIGHT jump to a catch block 5 frames up. You
  can't tell from reading.
- Forgotten exceptions become crashes.
- Exception types are usually weakly typed (any class can be
  thrown).
- Hard to optimize -- every call is potentially a non-local
  jump.

### Philosophy 2: errors as values (vāṇी, Rust, Go, Haskell)

A failing function RETURNS a value indicating failure. There's
no special "throw". The return type carries either a success
value or an error value.

Pros:
- All control flow is visible. `f()` returns. Always. The
  caller decides what to do with the result.
- Errors are part of the type system -- the compiler enforces
  you handle them.
- Easy to optimize -- no non-local jumps.

Cons:
- Calling code has to handle the error case at every call. Can
  be noisy without language support.

vāṇी picks Philosophy 2 and adds **language support** to make
it ergonomic.

## The shape: `Result<T, E>`

The standard "did it work?" type:

```vani
enum Result<T, E> {
  Ok(T),    // success -- wraps a T
  Err(E),   // failure -- wraps an E
}
```

A function that might fail returns `Result<the-value-it-would-
have-returned, the-kind-of-error>`:

```vani
fn parse_int(s: Str) -> Result<i64, ParseError> { ... }
```

You can't accidentally use the return value as if it were just
`i64`. The type system forces you to inspect -- was it `Ok(n)`
or `Err(e)`?

## The naive way (without language support)

You inspect every Result with `if let` / `else if let`:

```vani
fn parse_num(s: Str) -> Result<i64, ParseError> { ... }

fn pipeline(s: Str) -> Result<i64, ParseError> {
  let parsed: Result<i64, ParseError> = parse_num(s);
  let n: i64 = 0;
  if let Result.Ok(v) = parsed {
    n = v;
  } else if let Result.Err(e) = parsed {
    return Result.Err(e);   // bail out
  }
  let validated: Result<i64, ParseError> = validate(n);
  let v: i64 = 0;
  if let Result.Ok(vv) = validated {
    v = vv;
  } else if let Result.Err(e) = validated {
    return Result.Err(e);
  }
  return Result.Ok(v * 2);
}
```

(Not `parse_int` -- that's one of vāṇी's own built-in names and
can't be redefined; and not a `match`-expression with a `return`
arm -- verified directly that doesn't parse: a `let x = match {
... }` puts every arm in expression position, and `return` is a
statement, not an expression, so it can't appear there. `if let` /
`else if let` are statements, which is exactly why they're the
right tool for "extract a value, or bail out of the enclosing
function" -- confirmed working on both backends.)

This works but is repetitive. Every fallible call has the same
pattern: "if Ok, continue; if Err, bail out propagating the
error."

## The `try` keyword and `?` operator -- works today for `Option<T>`, not yet for `Result<T, E>`

vāṇी reserves `try EXPR` and the postfix `?` operator for
exactly this pattern. **The real v1 boundary is narrower than
"not implemented" -- it already works, but only for enums
shaped like `Option<T>`** (exactly one payloaded variant plus
one payload-less variant). Confirmed directly, both forms, both
backends:

```vani
fn lookup(x: i64) -> Option<i64> {
  if x > 0 { return Option.Some(x * 2); }
  return Option.None;
}

fn pipeline(x: i64) -> Option<i64> {
  let a: i64 = try lookup(x);      // prefix form -- if None, return it
  let b: i64 = lookup(a)?;         // postfix form -- same meaning
  return Option.Some(b + 1);
}
```

This compiles and runs correctly today, both `try` and `?`,
both backends.

**`Result<T, E>` is where it's still unimplemented**, because
`Result`'s `Err(E)` variant is ALSO payloaded (unlike `Option`'s
payload-less `None`) -- `try`/`?` require exactly one payloaded
+ one payload-less variant, and `Result<T, E>` has two payloaded
variants:

```vani
// Rejected today -- Result has 2 payloaded variants, try/? need 1+1:
fn pipeline(s: Str) -> Result<i64, ParseError> {
  let n: i64 = try parse_num(s);   // if Err, return it
  let v: i64 = validate(n)?;       // postfix form -- same meaning
  return Result.Ok(v * 2);
}
```

<img class="manas" src="../images/mascot/manas_mascot_error.png" title="this code does not compile!"/>

```
`try` requires the enum 'Result' to have exactly one payloaded
variant and one payload-less variant; got 2 payloaded and 0
payload-less.
`try EXPR` is reserved as a keyword but the desugar to
match-with-early-return is still in progress (T2.6 Phase 2).
Write the pattern manually: `match opt { Opt.Some(v) then v,
Opt.None then return Opt.None };`
```

There's a second restriction even for the working `Option<T>`
case, worth knowing before you rely on it: **between the
`try`-let and the function's final `return`, only `let`,
`print`, and simple reassignments are permitted** -- `if`,
`while`, and `for` in that span aren't supported yet (T2.6
phase 2 again, a narrower gap than the whole feature). Keep any
conditional logic before the first `try`/`?` or after the last
one, not straddling it.

**Until `Result<T, E>` support lands, write `Result` propagation
by hand** using the match-then-bail pattern from the previous
section. The manual form is what [Intermediate 10](10_result_try.md)
shows.

## "`Option<T>`" -- for "absent" rather than "failed"

Sometimes a function might not have a value to return -- not
because something WENT WRONG, but because there's just nothing
there. Looking up a key in a map; reading the first element of
a possibly-empty Vec.

```vani
enum Option<T> {
  Some(T),
  None,
}
```

`Option<T>` is the simpler cousin of Result. Both support
`try` and `?` in vāṇी; they propagate "absence" or "failure"
respectively up the call chain.

## When NOT to use Result

Some operations genuinely can't fail (or fail only in ways that
indicate compiler/programmer bugs, not runtime conditions).
For those, return the value directly:

- `len(xs)` returns `u64`, not `Result<u64, _>`. Length is
  always known.
- `string + string` returns `OwnedStr`, not `Result<OwnedStr,
  _>`. Concat can't fail in a normal flow.
- `xs[i]` returns `T`. Bounds-checked at compile-time when
  possible; runtime panic for unprovable cases is a
  programmer bug, not a return-able error.

Result is for *expected* failures -- conditions the caller
should reasonably anticipate and handle: missing files, bad
input, network outages.

## Pairing with SMT

Sometimes you can ELIMINATE a Result by proving the failure
case impossible:

```vani
fn divide(a: i64, b: i64) -> Result<i64, DivisionError>
  // returns Err(...) when b == 0

fn divide_unchecked(a: i64, b: i64) -> i64
  requires b != 0;
  // no Result needed -- compiler proves b != 0
```

The unchecked version takes a `requires` contract. Callers must
prove (to the SMT solver) that `b` is non-zero. In return, they
get a plain `i64` -- no Result wrapping, no `try`, just the
value.

This composes beautifully with the contract system from
chapter [12a SMT primer](12a_smt_primer.md): use contracts for
"can't happen if the caller is well-behaved", use Result for
"might happen at runtime depending on data."

## "Why not just return a sentinel like -1?"

Some C APIs use `-1` to mean "error". Why is Result better?

1. **The type system enforces handling.** You can't
   accidentally use `-1` as a real result; the compiler makes
   you destructure the Result first.
2. **Multiple error types**. `Err(NotFound)` vs `Err(Permission
   Denied)` vs `Err(Timeout)` -- each gets a distinct enum
   variant. Sentinel values run out fast.
3. **Composability**. `try`/`?` work over any Result. Sentinel
   conventions don't compose; every API has its own.

## A summary you can carry

- **Errors as values**: failing functions RETURN a Result.
  Caller inspects the type, handles success vs failure.
- **`Result<T, E>`** -- `Ok(T)` for success, `Err(E)` for
  failure.
- **`Option<T>`** -- `Some(T)` for present, `None` for absent.
- **`try EXPR`** -- if Ok, give me the value; if Err, return
  the Err from the enclosing function. Syntactic sugar for a
  match-then-bail pattern.
- **`EXPR?`** -- postfix form of the same thing. Identical AST.
- Use Result for **expected, runtime-dependent failures**.
- Pair with **SMT contracts** to eliminate Results where the
  failure case can be proven impossible.

This pattern + the SMT pattern (from 12a) are the two halves
of vāṇी's "compile-time-proves-correctness" + "runtime-explicit-
handling" story. Together they cover most error scenarios
without exceptions OR uncaught crashes.

The next chapter ([Intermediate 10](10_result_try.md)) shows
the full syntax + worked examples.

## Cross-reference

- [Intermediate 10 -- Result + try](10_result_try.md) -- actual
  syntax + worked examples
- [Intermediate 12a -- SMT primer](12a_smt_primer.md) -- the
  "compile-time-prove-it-can't-fail" alternative to Result;
  use both together
- [Intermediate 2 -- Enums with payloads + match arms](02_enums_payloads.md)
  -- Result and Option are payloaded enums; the matching
  syntax is shared
- [Intermediate 4c -- Generics primer](04c_generics_primer.md)
  -- `Result<T, E>` is a two-parameter generic enum;
  monomorphization specializes per (T, E) pair used
- [Intermediate 10b -- Runtime errors + panic-free design](10b_runtime_errors_primer.md)
  -- when to reach for Result/? vs. `assert` / contracts;
  the segfault-free guarantee; what hits `abort` and what
  doesn't


---

**Previous**: [Sec.9d -- Build-system integration ->](09d_build_systems.md)
**Next**: [Sec.10b -- Runtime errors and panic-free design primer ->](10b_runtime_errors_primer.md)

