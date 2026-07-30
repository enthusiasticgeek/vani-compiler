# Intermediate 10b -- Runtime errors, panic-free design, and the segfault-free guarantee

> **Learning goal**: build the mental model for what *can*
> fail at runtime in a vāṇी program, what *cannot* by
> construction (segfaults, UAF, double-free, dangling-ref
> deref), and the everyday discipline for writing programs
> that never reach an abort. Reading order: at minimum
> [Intermediate 10a -- Result/try primer](10a_result_try_primer.md)
> + [Beginner 09 -- first contract](../beginner/09_smt_intro.md).
> Optional follow-up: [Intermediate 12a -- SMT primer](12a_smt_primer.md).

This chapter has **no compiler code**. Pure intuition with
side-by-side comparisons.

## The safety interlock

Think about a microwave oven. Physically, mechanically, if the
door is open, the magnetron will not run. Manufacturers didn't put
a sticker on the door reading "please don't run this with the door
open" and hope for the best -- they wired a switch into the door
hinge itself. Open door means power to the emitter is cut, full
stop, by the machine's own construction. You cannot "accidentally"
microwave your hand through the open door, because the state "door
open + emitter on" simply cannot occur. It isn't that the microwave
is careful; it's that the dangerous state has no wire connecting to
it at all.

Now think about a car's transmission. Most modern automatics will
not let you shift into reverse while you're doing 60 miles an hour
on the highway. It's not that the car "notices" you did something
bad and slams the brakes in a panic. The mechanism simply refuses
to engage the reverse gear above some safe threshold. If you yank
the lever, nothing catastrophic happens -- the linkage just doesn't
complete the shift. Compare that to what WOULD happen if the
gearbox complied: gears shredding, the driveshaft locking at
highway speed, a violent, unpredictable, un-recoverable mechanical
failure -- sparks and smoke.

Notice the shape both examples share: the machine doesn't wait for
the dangerous thing to happen and then scramble to react. It makes
the dangerous state physically unreachable in the first place. And
where it CAN'T rule something out mechanically -- say, the oven's
timer dial gets turned to an invalid number -- a well-designed
appliance still fails in a controlled, predictable way: a fault
light, a clear beep code, a documented error state you can look up
-- never silent internal damage that shows up as a mystery later.

This is the split vāṇी programs live by. Some categories of
runtime disaster -- the programming equivalent of "door open +
emitter on," things like writing past the end of an array, using
memory after it's been freed, following a null pointer -- are made
structurally unreachable by the compiler itself, the same way the
door interlock makes the dangerous microwave state unreachable. You
didn't have to remember to check for them; there is no path to them
at all. Other categories -- a value that's legitimately out of
range, a contract the caller failed to uphold -- can't be ruled out
by construction, so instead of corrupting memory unpredictably, the
program stops in a controlled, named, diagnosable way, the way the
transmission just declines the shift instead of shredding gears.
That controlled stop is vāṇी's `abort()`: not a crash in the
"anything could happen" sense, but a deliberate, predictable
refusal to continue in a dangerous state.

## What "runtime error" even means

In most languages, "runtime error" is a grab-bag of very
different failure modes:

- **Bug-class crashes**: null-pointer dereference, use-after-
  free, double-free, buffer overrun, wild stack-smash. The
  program is in *undefined behavior* -- anything could happen
  next. C programs segfault here; C++ too; Rust forbids
  these in safe code; Python's interpreter shields you but
  C extensions can still crash.
- **Logic-class crashes**: division by zero, integer overflow,
  array out-of-bounds, assertion failure. The program could
  have *known* about these statically with enough effort. C
  invokes UB; Java throws; Python raises; Rust panics.
- **Recoverable failures**: file not found, network timeout,
  parse error, user typed nonsense. These are not bugs --
  they're inputs the program needs to handle. C returns -1
  / sets errno; Java throws checked exceptions; Python
  raises; Rust returns `Result<T, E>`.

vāṇी's design splits these cleanly:

| Class | vāṇी's stance |
|---|---|
| Bug-class crashes | **Structurally impossible on hosted targets** -- affine ownership + bounds checks + no raw pointers + scope-escape analysis eliminate the surface. |
| Logic-class crashes | **Compile-time when SMT can prove**; otherwise a clean `abort()` at the operation with a diagnostic. No corrupted state. |
| Recoverable failures | **Always values** -- `Result<T, E>` or `Option<T>` returned, propagated via `try` / `?`, matched by the caller. No exceptions, no unwinding. |

The rest of this chapter unpacks each row.

## Row 1: the "no segfault" guarantee

Hosted-target vāṇी (Linux / macOS / Windows; no embedded
`unsafe`) has **no segfault surface** in source-level code.
The reasons stack:

### 1. No raw pointers, no nullable references

There is no `*const T` / `*mut T` in the safe language. The
pointer-shaped vocabulary is `ref T` / `mut ref T` (scope-
checked second-class references), `Box<T>` (single-owner
heap allocation with auto-drop), `dyn Iface` fat pointers,
`Box<dyn Iface>`, and indices into `Vec<T>`. None of these
can be null. `Option<Box<T>>` makes "maybe-allocated" an
explicit type that the caller MUST match on before
dereferencing -- no silent nullptr crash.

### 2. Affine ownership eliminates UAF and double-free

Every heap-owning value (`Vec<T>`, `OwnedStr`, `Box<T>`,
`Mutex<T>`, `Channel<T, N>`, `Task`, user structs with
heap fields) has exactly one owner. After `let y = x;`,
`x` is a compile-time error to reference. Drop fires
exactly once on the new owner. No double-free path exists
to be exercised. No UAF path either -- after drop, the
binding is unreachable.

### 3. Scope-escape analysis prevents dangling references

A `ref T` / `mut ref T` can appear in parameters, `let`
bindings, and user struct fields. The scope-escape analyzer
walks every shape that COULD let the reference outlive its
source -- returning the ref-holding struct, storing it in a
`Vec`, holding `mut ref T` across a suspend point -- and
rejects them at compile time with the exact source line.

### 4. Bounds-checked indexing by default

`xs[i]` on a `Vec<T>` or array fires the bounds check unless
SMT proved at compile time that `i < len(xs)`. The check is
emitted by both backends. Failure path: `abort()` with a
diagnostic, NOT a buffer overrun into adjacent memory.

### 5. No unsafe on hosted

`unsafe(reason = "...")` is rejected by the hosted-build
path. It only opens on `--target embedded`, and even there,
Layer 1-3 of the safety net ([unsafe.md](https://github.com/enthusiasticgeek/vani-compiler/blob/main/unsafe.md))
catches the majority of pointer-class mistakes.

### What this rules out

The classic segfault sources from C/C++ don't exist:

- `NULL` dereference -> no nullable pointers
- Use-after-free -> affine + scope-escape
- Double-free -> affine drop fires exactly once
- Buffer overrun (read or write) -> bounds checks
- Dangling stack-frame return -> scope-escape rejects at parse
- Wild pointer arithmetic -> no raw pointers in safe code
- Stack smash via unbounded `strcpy` -> no unbounded copy primitives
- Type confusion via `transmute` -> not in safe code

A hosted-target vāṇी program **cannot segfault from source
code alone**. If one segfaults, the cause is in linked C
(FFI), the OS (running out of stack), or a compiler bug.

## Row 2: the abort surface

The structural guarantees above leave a small, well-named
set of *logic-class* checks the compiler emits at runtime
when SMT can't discharge them statically. When one fails,
the program calls `abort()` -- a clean, deterministic, named
termination, NOT undefined behavior.

### What can abort

| Trigger | Source-level operation | What the abort prints |
|---|---|---|
| `assert(p)` fires with `p == false` | Any `assert ...;` | `"assertion failed: <expr> at <file>:<line>"` |
| `prove(p)` fires (strict assert) | Any `prove ...;` | `"prove failed: <expr> at <file>:<line>"` |
| `requires` fails at function entry | Any function call where SMT couldn't prove the pre-condition | `"precondition failed: <expr> on call to <fn>"` |
| `ensures` fails at function return | Any function return where SMT couldn't prove the post-condition | `"postcondition failed: <expr> in <fn>"` |
| `invariant` fails on loop entry / exit | A `for` / `while` loop | `"invariant failed at <kind>: <expr>"` |
| Index out-of-bounds (SMT-unprovable site) | `xs[i]` / `arr[i]` | `"index out of bounds: i=<n>, len=<n>"` |
| Integer overflow (SMT-unprovable site) | `a + b`, `a * b`, etc. on signed integers | `"integer overflow: <op> at <file>:<line>"` |
| Divide / modulo by zero | `a / b` / `a % b` when SMT can't prove `b != 0` | `"divide by zero at <file>:<line>"` |
| Shift past width | `a << k` / `a >> k` when SMT can't prove `k < width(a)` | `"shift overflow at <file>:<line>"` |

That's it. Every other operation either succeeds, returns a
`Result<T, E>` / `Option<T>` for the caller to handle, or is
structurally prevented (Row 1).

### What `abort` does, exactly

When one of the conditions above fires:

1. The compiler-emitted check prints the diagnostic to
   stderr.
2. `abort()` from libc is called (POSIX `SIGABRT`; on
   Windows, `_exit(3)`).
3. The process terminates immediately. No destructors run.
   No `finally` blocks. No cleanup beyond what the OS does
   on process exit.
4. The exit code is platform-conventional (typically
   non-zero; on POSIX, a signal-terminated process exits
   with `128 + SIGABRT = 134`).

This is **graceful in the diagnostic sense** -- a named,
deterministic event with a printable cause -- but **terminal
in the cleanup sense** -- no chance to recover, no chance to
flush buffers manually, no chance to write a crash dump from
inside vāṇी code.

The intentional trade: abort is reserved for "I have
detected a bug; the program's invariants are violated;
continuing would be wrong." Recoverable conditions are
`Result<T, E>`, not abort.

## Row 3: the graceful patterns

For everything that *can* fail and that the program should
handle, use `Result<T, E>` (or `Option<T>` for "value or
absent") and propagate with `try` / `?`.

### When to return `Result<T, E>` vs. `assert`

The deciding question: **is this a contract the caller is
obligated to uphold, or is this input that the caller might
legitimately produce?**

- Contract -> `requires` (compile-time check if possible,
  abort if SMT can't discharge). Example: `fn sqrt_int(n:
  i64) requires n >= 0 -> i64` -- the caller MUST pass
  non-negative. Passing negative is a bug.
- Input -> `Result<T, E>`. Example: `fn parse_int(s: Str)
  -> Result<i64, ParseError>` -- the caller might
  legitimately pass non-numeric text. Returning the
  failure as a value lets the caller decide.

A useful rule of thumb: **if the caller could write tests
that intentionally pass the bad value to verify error
handling, it's a `Result`. If passing the bad value would
be a test of the type system itself, it's a contract.**

### Worked split -- a small parser

```vani
fn parse_int(s: Str) -> Result<i64, OwnedStr> { ... }
// Recoverable: user input might be "abc". Caller decides.

fn lookup_key(map: ref HashMap, key: Str) -> Option<i64> { ... }
// Recoverable: key might be absent. Caller decides.

fn slot_at(xs: ref Vec<i64>, i: u64) -> i64 {
  // CONTRACT: caller promises i < len(xs).
  // If SMT can't prove it, runtime bounds check fires + abort.
  // Used internally where the caller has already validated.
  return xs[i];
}

fn validated_at(xs: ref Vec<i64>, i: u64) -> Option<i64> {
  // Recoverable: any i is acceptable input. Caller decides.
  if i >= len(xs) {
    return None;
  }
  return Some(xs[i]);
}
```

The same underlying op (read index `i` from a Vec) gets two
signatures based on whose responsibility it is to validate.
The split is a design choice, not a compiler decision.

### Propagation with `?` / `try`

When a function's body calls many fallible functions, the
postfix `?` operator (or the `try EXPR` keyword -- same AST
node, two surface spellings) propagates failures up. **As
[Intermediate 10a](10a_result_try_primer.md) covers in detail,
this works today for `Option<T>`-shaped returns, not yet for
`Result<T, E>`** (confirmed directly: a
two-payloaded-variant enum like `Result` is rejected --
`try`/`?` need exactly one payloaded + one payload-less
variant). The built-in `parse_int(s: Str) -> Option<i64>`
already returns the supported shape, so it's a working example
today, no "intended future syntax" caveat needed:

```vani
fn safe_div(num: i64, den: i64) -> Option<i64> {
  if den == 0 { return Option.None; }
  return Option.Some(num / den);
}

fn compute(n: i64) -> Option<i64> {
  let r: i64 = safe_div(100, n)?;
  return Option.Some(r);
}
```

Three things to notice:

1. Each `?` desugars to "if `None`, return `None`
   immediately; if `Some`, unwrap to the value." Propagation
   is automatic.
2. The function's own return type is `Option<...>`, so the
   propagated absence is already the right shape -- no
   re-wrapping needed.
3. **A real, confirmed v1 gap worth knowing before you lean on
   this**: a function with a heap-owning (`OwnedStr`, `Vec<T>`,
   ...) PARAMETER, combined with a `?`/`try` that actually takes
   the early-return path inside that same function, crashes
   (confirmed both backends, unrelated to whether the parameter
   is even used by the `try`/`?` call). Keep functions that use
   `?`/`try` to scalar (`i64`/`f64`/`bool`) parameters only;
   parse/extract any `OwnedStr` input with `if let` *before*
   calling into a `?`-using helper, the way the worked example in
   [Intermediate 10](10_result_try.md) does.

Once `Result<T, E>` support lands, the equivalent
`Result`-returning chain (`parse_int`-style parsing, a lookup,
each propagated with `?`) will read the same way -- for now,
write `Result` propagation by hand with `if let` / `else if let`
per the previous section, or reshape the fallible step to return
`Option<T>` if "why" doesn't matter to the caller.

## Row 4: lifting runtime checks to compile time

The SMT layer ([Intermediate 12a primer](12a_smt_primer.md))
can prove that a runtime check ALWAYS succeeds, in which
case the compiler elides the check entirely. The check
that's never there can never fire.

### Index access -- bounds elision

```vani
fn sum_first_three(xs: ref Vec<i64>) -> i64
  requires len(xs) >= 3
{
  return xs[0] + xs[1] + xs[2];
}
```

The `requires len(xs) >= 3` clause is checked at every call
site. At each `xs[i]` inside the body, SMT discharges
`i < len(xs)` from the pre-condition. The generated code
contains **no bounds check** for these three indices. The
program is faster AND can't possibly fire that abort
because the path doesn't exist in the emitted code.

If you remove the `requires`, the compiler emits the
runtime checks -- they MIGHT fire on a 0-length input.

### Integer overflow -- range elision

```vani
fn safe_add(a: i64, b: i64) -> i64
  requires a >= 0 && a <= 1000000
  requires b >= 0 && b <= 1000000
{
  return a + b;
}
```

SMT proves `a + b` cannot overflow given the pre-conditions
(both inputs <= 1M, sum <= 2M, well within i64). The overflow
guard is elided.

### Divide-by-zero elision

```vani
fn safe_divide(num: i64, den: i64) -> i64
  requires den != 0
{
  return num / den;
}
```

Same shape. The runtime "is `den == 0`?" check is gone
because the contract excludes it.

### The reward

Real programs have ~5-15% runtime check overhead from these
emitted guards. Threading `requires` through hot-path
functions is the discipline for eliminating that overhead
in numerics, parsers, codecs, and tight loops. The check
that never runs is the fastest check.

### When SMT can't discharge

The guard stays. The program is no slower than it would be
in a language that ALWAYS does the check (most languages).
The guard's behavior on failure is `abort` -- not silent UB,
not corrupted state.

You can ALWAYS keep the guards on with `INTENTC_NO_VERIFY=1`
during development; profile-guided you can ALSO ship with
elision aggressive in release builds. The trade is yours
per build.

## Row 5: what graceful abort looks like at runtime

When you DO hit a `requires` / `assert` / unprovable bounds
check in production, what the user sees:

```
$ ./my_program
... normal output ...
precondition failed: len(xs) >= 3 on call to sum_first_three
  at src/processing.vani:42
$ echo $?
134
```

Three properties:

1. **Named.** The specific assertion / pre-condition is
   printed. Not just "segmentation fault" -- the exact
   contract that was violated.
2. **Located.** The source file and line. No address-only
   trace; no need for symbol resolution.
3. **Deterministic.** Same input produces the same abort
   message. Reproducible in a test harness.

### Catching an abort with a signal handler (services)

For long-running services that want to write a crash dump
or notify oncall before exiting, the conventional approach
is an external supervisor (systemd, k8s probe, your service
manager). vāṇी does not currently expose an in-process
SIGABRT hook because the design treats abort as
"contract violated, anything I do next could compound the
bug." The supervisor approach keeps recovery logic *outside*
the program whose invariants just failed.

If you genuinely need in-process crash handling (e.g., to
upload a Sentry-style crash report), use FFI to install a
C-level `signal(SIGABRT, handler)` *before* main returns,
in a tiny FFI shim. Keep the handler async-signal-safe (no
mallocs, no locks).

## Side-by-side -- same input, four languages

A program that reads `argv[1]`, parses it as an integer,
divides 100 by it, and prints the result.

### C (with no validation)

```c
int main(int argc, char** argv) {
    int n = atoi(argv[1]);   // <- argv[1] could be NULL -> segfault
    int r = 100 / n;         // <- n could be 0 -> SIGFPE / abort
    printf("%d\n", r);       // <- n could be negative; atoi could
                             //   silently truncate; UB on overflow
    return 0;
}
```

Failure modes: segfault if argv[1] is missing; SIGFPE if
argument is "0"; silent wrong answer if argument is
"99999999999"; silent wrong answer if argument is "abc"
(atoi returns 0, then divide by zero, then SIGFPE).

### Rust

```vani
fn main() {
    let n: i32 = std::env::args().nth(1).unwrap().parse().unwrap();
    let r = 100 / n;
    println!("{}", r);
}
```

Two `unwrap()` calls panic on missing or non-numeric input.
The divide panics on zero. Integer overflow is checked in
debug, silently wraps in release -- a footgun. The panics
unwind by default (drops run); aborts on `panic=abort`
builds.

### Python

```python
import sys
n = int(sys.argv[1])
print(100 // n)
```

Raises `IndexError` if `argv[1]` missing; `ValueError` if
not numeric; `ZeroDivisionError` if zero. All catchable with
`try / except`. No segfault possible (CPython is memory-safe
for pure-Python code).

### vāṇी

<img class="manas" src="../images/mascot/manas_mascot_caution.png" title="adapted, not a literal port -- see the notes below"/>

vāṇी's `main` can't take `argv` directly -- `fn main() -> i64`
takes no parameters at all (command-line args need an `extern`
FFI declaration for the C runtime's `argc`/`argv`, out of scope
here); and `parse_int` returns `Option<i64>`, not `Result`, so
the comparison reaches for `stdin_read_line()` (the native,
FFI-free way to get user input -- see
[Intermediate 9c](09c_file_io.md)) and `Option` instead. Verified
end-to-end on both backends, all three paths (valid divide,
divide-by-zero, non-numeric input):

```vani
fn safe_div(num: i64, den: i64) -> Option<i64> {
  if den == 0 {
    return Option.None;
  }
  return Option.Some(num / den);
}

fn compute(n: i64) -> Option<i64> {
  let r: i64 = safe_div(100, n)?;
  return Option.Some(r);
}

fn main() -> i64 {
  let _ = flush_stdout();
  print "n:";
  let _ = flush_stdout();
  let line: OwnedStr = stdin_read_line();

  // Extract `n` BEFORE calling into any `?`/`try`-using
  // function -- see the callout in the previous section for
  // why an OwnedStr parameter can't safely sit alongside a
  // `?`/`try` that early-returns in the same function today.
  let parsed: Option<i64> = parse_int(line);
  let n: i64 = 0;
  if let Option.None = parsed {
    print "not a number";
    return 2;
  } else if let Option.Some(v) = parsed {
    n = v;
  }

  let result: Option<i64> = compute(n);
  if let Option.Some(r) = result {
    print r;
    return 0;
  }
  print "divide by zero";
  return 3;
}
```

Each failure mode is a named return code with a printed
message. `parse_int` is `Option` -- bad text goes to the `None`
arm. `safe_div` rejects zero up front, propagated through
`compute` via `?`. The program cannot segfault, cannot panic,
cannot abort.

Trade: more code than the C / Rust / Python versions. In
return: every failure mode is *visible at the type level*
and *handled at the call site*. There is nowhere a future
maintainer could introduce a crash without changing the
type signatures.

## What if I genuinely WANT abort behavior?

Sometimes the right move is "if this is wrong, kill the
program -- I can't recover." Two paths:

### 1. `assert` for invariants you've verified

<img class="manas" src="../images/mascot/manas_mascot_caution.png" title="this code needs extra care"/>

```vani
fn process(xs: ref Vec<i64>) -> i64 {
  // The caller has already validated this; if it's wrong,
  // there's a bug in the validator.
  assert len(xs) > 0;
  return xs[0];
}
```

Same as Rust's `debug_assert!` / Java's `assert`. Use for
"this should be true; if it isn't, abort with a named
message instead of corrupting state."

### 2. `prove` for "I'm asserting this is mathematically
   true and want SMT to verify it now"

<img class="manas" src="../images/mascot/manas_mascot_caution.png" title="this code needs extra care"/>

```vani
fn next_power_of_two(n: i64) -> i64 {
  ...
  prove result >= n && result <= 2 * n;
  return result;
}
```

`prove` is the strict form: SMT MUST discharge it at compile
time, OR the build fails. Use when you have an invariant
that the rest of the program's correctness depends on and
you don't want to ship without proof.

These tools are *for* aborting -- they're not bugs to be
avoided. They mark "if this fails, the bug is here, in this
named contract."

## A summary you can carry

- **Bug-class crashes** (segfault, UAF, double-free, dangling
  ref, buffer overrun) are **structurally impossible** on
  hosted vāṇी. Affine ownership + bounds checks + no raw
  pointers + scope-escape analysis remove the surface.
- **Logic-class crashes** are **compile-time when SMT can
  prove**; otherwise a clean named `abort()` at the
  operation with a diagnostic. The abort surface is small
  and named: assert / prove / requires / ensures /
  invariant / index OOB / overflow / div-by-zero / shift
  past width.
- **Recoverable failures** are always values -- `Result<T, E>`
  / `Option<T>` propagated via `?` / `try`. No exceptions,
  no unwinding, no "uncaught exception" surprise.
- The split between `Result<T, E>` and `assert` is a design
  choice: contracts (caller obligated to uphold) -> `assert`
  / `requires`; inputs (caller might legitimately produce
  bad value) -> `Result<T, E>`.
- **Lift checks to compile time** with `requires` / `ensures`
  -- SMT elides the runtime guards, programs run faster, and
  the abort that's not emitted can't fire.
- Abort is **terminal, diagnostic, deterministic**. No
  destructors run, no cleanup, no recovery from inside
  the program. Crash recovery lives in the supervisor /
  service manager, outside.

The takeaway: **vāṇी replaces "what could go wrong at
runtime?" with "what HAVE I declared is allowed to go
wrong?"** Everything else is either structurally
impossible or named at a contract.

## Cross-reference

- [Intermediate 10a -- Result / try / `?` primer](10a_result_try_primer.md)
  -- graceful error propagation
- [Intermediate 10 -- Result, `try`, and the `?` operator](10_result_try.md)
  -- syntax + worked examples
- [Beginner 09 -- first contract `assert` / `prove` / `requires`](../beginner/09_smt_intro.md)
  -- the assert side
- [Intermediate 12a -- SMT primer](12a_smt_primer.md) -- the
  intuition for compile-time discharge
- [Intermediate 12 -- SMT deep-dive](12_smt_deepdive.md) --
  the full SMT surface
- [Advanced 6 -- SMT trace debugging](../advanced/06_smt_debug.md)
  -- when SMT *can't* discharge: how to read the trace,
  what to strengthen
- [Advanced 4 -- Embedded + `unsafe` + regions](../advanced/04_embedded.md)
  -- the only place the safe-by-construction surface opens,
  and how Layer 1-3 keep it bounded


---

**Previous**: [Sec.10a -- Result / try / ? primer ->](10a_result_try_primer.md)
**Next**: [Sec.10 -- Error handling: Result<T, E> + try ->](10_result_try.md)

