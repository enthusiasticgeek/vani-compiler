# Intermediate 16a -- Testing your vāṇी code: `#[test]`, `vanic test`, and `assert_eq_*`

> **Learning goal**: write real, TDD-style tests *in* vāṇī itself --
> discovery, isolation, filtering, expected-failure tests, and
> value-showing assertions -- and know when `vanic test` reaches for
> your Kosh package automatically.

This is a walking tour of
[`examples/language/english/testing_primer.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/english/testing_primer.vani).
If you've used `cargo test`, most of this will feel immediately
familiar -- that's deliberate; `vanic test` borrows its vocabulary and
output shape on purpose.

The core mechanism (`#[test]` + `vanic test`) has existed since
2026-07-16/28; the additions on this page (`--filter`, `#[should_panic]`,
parallel execution, no-args package discovery, `assert_eq_*`) shipped
2026-08-14 in one pass, after an audit found the original feature
solid but under-polished -- worth knowing since a few of the details
below (like `#[should_panic]`) are newer than the core harness itself.

---

## The basics: `#[test]` + `vanic test`

Mark a function `#[test]`, give it no parameters, return `i64`
(`0` for pass), and leave the file with **no top-level `fn main`**:

```vani
#[test]
fn addition_works() -> i64 {
  assert 2 + 2 == 4;
  return 0;
}
```

```bash
vanic test math_test.vani
```

```
running 1 test (math_test.vani)
test addition_works ... ok

test result: ok. 1 passed; 0 failed
```

Each `#[test]` fn runs in its **own process** -- one failing `assert`
never takes the rest of the suite down with it, the same isolation
guarantee `cargo test` gives you. A file can hold as many `#[test]`
fns as you like; `vanic test` discovers and runs all of them.

**A file with `#[test]` fns AND a real `fn main` still runs the
tests**, not `main` -- `fn main` stays exactly what `vanic run`/
`vanic build` use. This mirrors Rust's own `#[cfg(test)] mod tests`
living alongside a binary crate's `fn main`:

```vani
fn double(x: i64) -> i64 { return x * 2; }

#[test]
fn double_works() -> i64 {
  assert double(21) == 42;
  return 0;
}

fn main() -> i64 {
  print "real program output:", double(10);
  return 0;
}
```

`vanic test this_file.vani` runs `double_works`; `vanic run
this_file.vani` still runs `main` and prints `real program output:
20`. Two independent consumers of the same file, no conflict.

**Directories work too**, recursively:

```bash
vanic test tests/
vanic test .                    # every *.vani under the cwd
```

A file with **no** `#[test]` fns (just a `fn main`) falls back to
"legacy" whole-file mode: pass iff the program exits `0`. This is how
`vanic test` behaved for years before `#[test]` existed, and it still
works unchanged for ordinary example/demo files.

---

## Only running some tests: `--filter`

```bash
vanic test math_test.vani --filter=addition
```

A plain substring match against each test's `path::name` label (or
the bare file path in legacy mode) -- matches `cargo test
<substring>`'s own default filter behavior, not a glob or regex. A
file whose `#[test]` fns all get filtered out is skipped silently
(no "0 tests" noise); a legacy file whose path doesn't match the
filter is skipped the same way.

---

## Proving a function correctly rejects bad input: `#[should_panic]`

Testing that something *should* fail is awkward to express with plain
`assert` -- you'd have to invert your own test's pass/fail logic by
hand. `#[should_panic]` does it for you: stack it with `#[test]`, and
the test **passes iff the process panics** (any non-zero exit --
mirrors Rust's own "any panic counts" `#[should_panic]` semantics,
not a specific internal exit code you'd have to know), and **fails
with `did not panic as expected`** if it exits cleanly instead.

```vani
fn divide(a: i64, b: i64) -> i64 {
  assert b != 0, "division by zero";
  return a / b;
}

#[test]
#[should_panic]
fn divide_by_zero_correctly_panics() -> i64 {
  let _ = divide(10, 0);
  return 0;
}
```

```
test divide_by_zero_correctly_panics ... ok (panicked as expected, 4 ms)
```

`#[should_panic]` without `#[test]` is a checker error -- the
attribute is meaningless on an ordinary function, so the compiler
tells you at check time rather than letting it silently do nothing.

---

## Showing *what* went wrong: `assert_eq_i64` / `_f64` / `_bool` / `_str`

Plain `assert a == b;` tells you a test failed -- not what `a` or `b`
actually *were*. You can write `assert a == b, "custom message";` by
hand, but you end up hand-formatting the values into the string every
time. `assert_eq_*` does that for you, the single most-reached-for
convenience in Rust's `assert_eq!`, Python's `assertEqual`, and every
other mainstream test framework:

```vani
fn is_even(n: i64) -> bool { return n % 2 == 0; }

#[test]
fn is_even_classifies_correctly() -> i64 {
  let _ = assert_eq_bool(is_even(4), true);
  let _ = assert_eq_bool(is_even(7), false);
  return 0;
}
```

Both calls above pass -- `is_even(4)` is `true` and `is_even(7)` is
`false`, matching what each call expects. Flip the second expectation
to `true` (deliberately wrong, to see a failure) and `assert_eq_bool`
prints both sides before exiting (same `exit(3)` convention every
other runtime trap already uses):

```vani
let _ = assert_eq_bool(is_even(7), true);   // wrong: 7 is odd
```

```
assertion failed: left != right
  left: false
 right: true
```

Four variants, one per type -- **`assert_eq_str` compares by
content**, not pointer identity: a string literal and a freshly
built `OwnedStr` with the same text still compare equal, the same
way `==` on strings already works elsewhere in the language:

```vani
#[test]
fn assert_eq_str_compares_by_content() -> i64 {
  let built: OwnedStr = "vā" + "ṇी";
  let _ = assert_eq_str(built, "vāṇी");   // passes -- same content, different allocation
  return 0;
}
```

Each call returns `i64` (always `0` when it doesn't trap) -- called
for the side effect, so `let _ = assert_eq_i64(a, b);` is the normal
shape, matching how most side-effecting builtins are used in this
language.

**These four builtins are tree-backend only** (no SSA-backend
lowering, same category as `stdin_ready_within_ms` and a handful of
others) -- transparent to you as a test author, just worth knowing if
you ever inspect emitted IR and wonder why a trivial test function
didn't take the usual fast SSA path.

---

## Running your whole package: no path argument

Inside a directory with a `vani.toml`, `vanic test` with **no path at
all** defaults to that package's root, recursively -- the same
`vani.toml`/`[package]` your `vanic publish`/`vanic add` workflow
already uses (see [Packages with Kosh](16_packages.md)), not a
separate concept:

```bash
cd my_package/
vanic test                 # scans the whole package, cargo-test style
```

Outside any package (no `vani.toml` in the cwd or an ancestor), you
still need to pass a path explicitly -- the error message says so.

---

## Faster suites: parallel execution by default

Every discovered test across every file/path now runs **concurrently**
by default, bounded by available CPU parallelism -- each test was
already isolated in its own process (see above), so there was nothing
stopping them from running at the same time except the old sequential
loop. Printed output is still grouped and ordered by file exactly as
if everything ran one at a time; only the actual compiling+running
happens in parallel, not the order you read the results in.

```bash
vanic test tests/                        # parallel by default
vanic test tests/ --test-threads=1       # fully serial (debugging, or a flaky-looking failure)
vanic test tests/ --test-threads=4       # force a specific worker count
```

---

## CI integration: `--json`

```bash
vanic test tests/ --json
```

```json
{"results":[{"path":"tests/math_test.vani::addition_works","ok":true,"ms":3}],"summary":{"passed":1,"failed":0}}
```

One object on stdout: `results` (per-test `path`/`ok`/`ms`, plus
`exit`/`reason` on failure) and a `summary`. `vanic test` exits `1`
if anything failed, `0` otherwise -- wire straight into a CI step's
own exit-code check without parsing human-readable lines at all.

---

## Try it yourself

1. Run `vanic test examples/language/english/testing_primer.vani`
   and read the output top to bottom -- five tests, one of them
   `#[should_panic]`, one exercising `assert_eq_str`'s content-not-
   pointer comparison.
2. Break one of `testing_primer.vani`'s assertions on purpose (flip
   an expected value) and re-run with `--filter=` narrowed to just
   that test -- confirm the rest of the suite doesn't even attempt to
   run.
3. Time a larger directory of tests with `--test-threads=1` vs. the
   default, using `time vanic test ...` -- the gap widens with more
   files, since each file's compile+run is real, independent work.
4. *(Bigger)* Add a small `tests/` directory to a scratch package
   (`vani.toml` + `entry = "main.vani"`) and run bare `vanic test`
   from inside it -- confirm it finds your tests with zero path
   arguments.

For a larger, realistic use of `#[test]` -- asserting concurrent
scheduler behavior across multiple threads, not just a single
function's return value -- see [Advanced 3e's job scheduler
capstone](../advanced/03e_job_scheduler_capstone.md).

---

## Summary

- `#[test]` on a zero-param, `i64`-returning function + no top-level
  `fn main` in the file puts `vanic test` into harness mode: each
  test its own process, one failure doesn't kill the suite. A file
  with `#[test]` fns AND a real `fn main` still runs the tests, not
  `main` -- `main` stays what `vanic run`/`build` use.
- `--filter=<substring>` narrows to matching tests/files (plain
  substring, `cargo test`-style).
- `#[should_panic]` (stacked with `#[test]`) inverts pass/fail: passes
  iff the process panics, fails with a clear message if it doesn't.
  Checker-rejected without `#[test]`.
- `assert_eq_i64`/`_f64`/`_bool`/`_str` print both sides on mismatch;
  `assert_eq_str` compares by content, not pointer identity.
- No path argument, run from inside a `vani.toml` package, defaults
  to that package's root -- the same manifest `vanic publish`/`add`
  already use, not a new concept.
- Every discovered test runs concurrently by default (`--test-threads`
  to override); printed output stays grouped/ordered by file exactly
  as sequential execution would have produced.
- `--json` for CI: one machine-readable object, `results` + `summary`,
  exit code still `1`/`0`.

---

**Previous**: [Sec.16 -- Packages with Kosh](16_packages.md)
**Next**: [Sec.17 -- Capstone: a terminal tic-tac-toe game ->](17_tic_tac_toe_capstone.md)
