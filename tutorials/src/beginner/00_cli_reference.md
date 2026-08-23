# Beginner 0 -- CLI Reference: `vanic` commands and flags

> A one-page reference for every `vanic` subcommand, flag, and
> environment variable. Bookmark this; you'll come back often.

---

## Quick-start

```bash
vanic run hello.vani              # compile + run (LLVM backend, default)
vanic run hello.vani --backend=c  # compile + run via C backend
vanic build hello.vani -o hello   # AOT-compile to a native binary
vanic check hello.vani            # type-check only; no output
```

---

## Subcommands

### `vanic run <file.vani>`

Compile and immediately run a program. The LLVM backend (default) emits
`.ll` IR and runs it through `lli` (the LLVM JIT). The C backend emits
`.c`, compiles with `cc`, and runs the resulting binary.

```bash
vanic run hello.vani
vanic run hello.vani --backend=c
vanic run hello.vani --backend=c --link-with helper.c -lm
vanic run hello.vani --target=aarch64-unknown-linux-gnu   # cross + QEMU
vanic run hello.vani --big-o          # print per-fn Big-O before running
```

| Flag | Description |
|------|-------------|
| `--backend=c\|llvm` | Select backend. Default: `llvm`. |
| `--link-with PATH` | Extra `.c` / `.o` / `.a` file to compile alongside (C backend only). Repeatable. |
| `-l<name>` | System library flag forwarded to cc (e.g. `-lm`). C backend only. |
| `--target=<triple>` | Cross-compile triple. Bare-metal triples print a helpful error; Linux cross-targets build an ELF and run via QEMU. |
| `--big-o[=auto\|force\|off]` | Print per-function Big-O complexity to stderr before running. `auto` skips O(1) fns; `force` includes all; `off` disables. |

**Environment variables**: `CC` (C compiler), `LLI` (LLVM JIT), `VANIC_NO_VERIFY=1` (skip SMT checks), `VANIC_SMT_DEBUG=1` (dump Z3 queries).

---

### `vanic build <file.vani>`

AOT-compile to a native binary. Pipeline: emit `.ll` -> `llc -filetype=obj` -> `.o` -> `cc -o <out>`.

```bash
vanic build hello.vani                       # outputs ./hello
vanic build hello.vani -o /tmp/hello         # custom output path
vanic build fw.vani --target=arm-none-eabi -o fw.elf   # cross-compile
vanic build fw.vani --no-std --target=arm-none-eabi    # explicit no-std
vanic build hello.vani --link-with helper.c -lm
```

| Flag | Description |
|------|-------------|
| `-o PATH` / `--out PATH` | Output binary path. Default: source file stem in cwd. |
| `--link-with PATH` | Extra input for the linker. Repeatable. |
| `-l<name>` | System library (e.g. `-lm`). Forwarded to cc. |
| `--target=<triple>` | Cross-compile. Passes `--mtriple=<triple>` to `llc`; selects `$CROSS_CC` or `<triple>-gcc` as linker. Bare-metal triples suppress libc/OpenMP flags. |
| `--no-std` | Omit libc headers from C prelude. Auto-activated for bare-metal triples. |

**Environment variables**: `CC`, `LLC`, `OPT`, `CROSS_CC` (cross-linker override), `QEMU_<ARCH>` (QEMU binary override).

**Runtime-helper object cache**: `sort_runtime.c` and
`parallel_runtime.c` (the C implementations backing `sort()` and
`parallel for`) are static, unchanging sources embedded in `vanic`
itself -- compiling them from scratch on every single build wastes
most of a typical build's wall time for no reason (measured: ~55% of
it). `vanic build` caches their compiled objects under
`$XDG_CACHE_HOME/vanic/runtime-objs/` (falling back to
`$HOME/.cache/vanic/runtime-objs/`, then `%LOCALAPPDATA%\vanic\
runtime-objs\` on Windows) and reuses one only when the source,
compiler, compiler version, flags, and host machine all match
exactly -- anything else is treated as a cache miss and recompiled
normally, so this is purely a speedup, never a correctness
trade-off. Delete that directory any time to force a clean rebuild of
both helpers; there's no flag needed to disable it since a stale or
missing entry always safely falls back to compiling fresh.

---

### `vanic check <file.vani>`

Type-check + SMT-verify a program without producing any output. Fast -- only runs the checker pipeline.

```bash
vanic check hello.vani
vanic check --no-verify hello.vani   # skip SMT; only type-check
vanic check --smt-debug hello.vani   # dump every SMT query + z3 response to stderr
vanic check --big-o hello.vani       # print per-fn Big-O complexity
vanic check --coverage hello.vani    # score feature-combination coverage (see below)
```

| Flag | Description |
|------|-------------|
| `--json` | One combined `{"diagnostics":[...]}` object across all files instead of human-readable text. |
| `--no-verify` | Skip SMT verification (type-checking still runs). Also: `VANIC_NO_VERIFY=1`. |
| `--smt-debug` | Dump every SMT query and z3 response to stderr. Also: `VANIC_SMT_DEBUG=1`. |
| `--big-o[=auto\|force\|off]` | Print per-function Big-O complexity. `auto` (default) skips O(1); `force` includes every fn; `off` is a no-op. |
| `--dump-fingerprints` | Print this program's feature-combination coverage fingerprints (`{shape}#{operation}`, one per line, sorted) instead of `ok: <file>`. See `src/coverage.rs`'s doc comment for the exact format. |
| `--coverage` | Score this program's feature combinations (0-100) against the compiler's own baked-in "known good" database -- see below. |
| `--emit-coverage-issue` | Implies `--coverage`. If the score is below 100, draft a local `<file>.coverage_issue.md` and print the exact `gh issue create` command to run it. `vanic` never files or sends anything on its own -- you decide. |

Exit 0 on success, 1 on any error.

#### Feature-combination coverage scoring

`--coverage` answers a narrower question than "does this program type-
check": *has the compiler's own regression/library test corpus ever
exercised, and verified leak/bug-free, this exact combination of type
shape and operation?* Every real bug found in the 2026-08-21/22 audit
rounds (BUG-216/217/218) was a gap at exactly this granularity -- one
missing arm in a per-element-type dispatch table (`Vec<Graph>#push`,
`Vec<Box<T>>#index_assign`, `RwLock<bool>#rwlock_write`, ...) while
every other combination for the same shape or the same operation
worked fine.

```bash
vanic check my_program.vani --coverage
# coverage: 62/100 (5/8 known feature combinations, db generated ... from <N> file(s))
#   untested combinations:
#     <Shape>#<operation>
#     ...
```

The database (`coverage_fingerprints.json`, baked into the binary --
`--coverage` works fully offline) is generated by `tools/
gen_coverage_db.py`, which reuses `tools/leak_sweep.py`'s existing
ASan+LeakSanitizer+UBSan sweep to determine which `examples/**/*.vani`
files are genuinely leak/bug-clean, then unions the fingerprints
extracted from just those files. A low score isn't necessarily a bug
in your program -- it means this specific combination isn't locked in
by a permanent regression test yet, which is worth knowing either way.
If you want to report the gap, `--emit-coverage-issue` drafts the
issue locally; nothing is ever sent without you running the printed
command yourself.

**This isn't hypothetical -- it already found real gaps once.** The
day `--coverage` shipped, scoring the exact BUG-216/217/218 repro
shapes against the freshly-generated database found all three still
scored well below 100 (`Vec<Graph>#push` 28/100, `Vec<Box<T>>
#index_assign` 38/100, `RwLock<bool>#rwlock_write` 23/100) --
`examples/` had never actually pinned a permanent regression test for
any of them, even though the underlying compiler bugs were already
fixed. Three new `examples/language/english/bug21{6,7,8}_*.vani`
files closed the gaps; all three now score 100/100. The database gets
regenerated (and the binary rebuilt) each time a new example is
added, so it always reflects the CURRENT corpus, not a stale snapshot.

---

### `vanic emit <file.vani>`

Emit lowered source (LLVM IR or C) to stdout or a file.

```bash
vanic emit hello.vani                       # LLVM IR to stdout
vanic emit hello.vani --backend=c           # C source to stdout
vanic emit hello.vani --backend=c --no-std  # C with bare-metal prelude
vanic emit hello.vani -o hello.ll           # LLVM IR to file
vanic emit hello.vani --backend=c -o hello.c
vanic emit hello.vani --big-o               # prepend Big-O annotations
```

`vanic emit-c` is a legacy alias that forces `--backend=c`.

---

### `vanic fmt <file.vani | directory> [--check]`

Format vāṇī source files. Without `--check`, rewrites the file in place.

```bash
vanic fmt hello.vani            # format in place
vanic fmt src/                  # format all .vani files under src/
vanic fmt --check hello.vani    # exit 1 if file is not already formatted
vanic fmt --check src/          # CI-style check for entire tree
```

---

### `vanic test [<file.vani | directory>...]`

Full tour with worked examples:
[Intermediate 16a -- Testing your vāṇī code](../intermediate/16a_testing_primer.md).
Quick reference:

`vanic test` supports two modes:

**`#[test]` attribute mode (recommended)**

Mark individual functions with `#[test]` and give the file no
top-level `fn main`. `vanic test` collects the `#[test]` fns and
gives each one its own synthesized `fn main() -> i64 { return
<fn>(); }` driver, compiled and run as a separate process -- so
one failing test doesn't take the rest of the suite down with it.
A file with `#[test]` fns AND a real `fn main` still runs the
tests, not `main` (`main` stays what `vanic run`/`build` use):

```vani
#[test]
fn addition_works() -> i64 {
  assert 1 + 1 == 2;
  return 0;
}

#[test]
#[should_panic]
fn div_by_zero_panics() -> i64 {
  let _ = 1 / 0;
  return 0;
}
```

```bash
vanic test math_test.vani
# running 2 tests
# test addition_works ... ok
# test div_by_zero_panics ... ok (panicked as expected, 3 ms)
# test result: ok. 2 passed; 0 failed
```

Each test function must take no parameters and return `i64`
(enforced by the ordinary type checker on the synthesized `main`).
Passing is `return 0;` (or any code path that doesn't abort). A
failing `assert`/`assert_eq_*`/other runtime trap inside the test
body aborts that test's process and counts as a failure; the rest
of the suite still runs -- concurrently, by default (see
`--test-threads` below).

`#[should_panic]` (stackable with `#[test]`) inverts pass/fail:
passes iff the process exits non-zero, fails with "did not panic as
expected" on a clean exit. Checker-rejected if used without
`#[test]`.

`assert_eq_i64(a, b)` / `assert_eq_f64` / `assert_eq_bool` /
`assert_eq_str` (builtins, usable anywhere, not just in tests) print
both sides on a mismatch before the same `exit(3)` every other
runtime trap uses -- `assert_eq_str` compares by content, not
pointer identity.

**Legacy mode**

A file with no `#[test]` fns at all (just a `fn main`) falls back to
legacy mode: `main` is expected to return 0 for pass, non-zero for
fail:

```bash
vanic test tests/
vanic test tests/math_test.vani
vanic test                         # no path: defaults to the enclosing
                                    # vani.toml package root, recursively
                                    # (error if no manifest is found)
```

Prints `ok` / `FAILED` per file/test, exits 1 if any fail.

| Flag | Description |
|------|-------------|
| `--filter=<substring>` | Only run tests/files whose label contains the substring (plain substring match, not a glob -- matches `cargo test <substring>`). |
| `--test-threads=<N>` | Worker count for the concurrent execution every discovered test runs under by default (bounded by available CPU parallelism). `=1` forces fully serial. Printed output stays grouped/ordered by file regardless. |
| `--json` | One machine-readable `{"results":[...],"summary":{...}}` object on stdout instead of human-readable lines. |
| `--smt-debug` | Dump every SMT query/response to stderr (also `VANIC_SMT_DEBUG=1`). |

---

### `vanic tokens <file.vani>`

Dump the lexer token stream. Useful for debugging parser or lexer issues.

---

### `vanic ast <file.vani>`

Dump the parsed AST (skips the type checker). Useful when you want to see what the parser produced even if the type checker would reject it.

---

### `vanic ir <file.vani>`

Dump the typed IR -- the representation the backends see after full type-checking. Useful for checking what the backends will lower.

---

### `vanic stack-depth <file.vani>`

Per-function stack frame estimates and maximum stack depth from each entry point. Required for ASIL-D / DO-178C.

```bash
vanic stack-depth fw.vani
vanic stack-depth fw.vani --max=4096        # exit 1 if any path exceeds 4 KiB
vanic stack-depth fw.vani --entry=main      # only report from main
vanic stack-depth fw.vani --format=json     # machine-readable output
```

---

### `vanic acyclicity <file.vani>`

Prove the call graph has no cycles. Catches mutual recursion. Required for DO-178C / ASIL-D.

```bash
vanic acyclicity fw.vani
vanic acyclicity fw.vani --format=csv
```

---

### `vanic deviations <file.vani>`

Extract every `unsafe(reason = "...")` block as a structured deviation record -- the audit artifact for ASIL-D / DO-178C / MISRA sign-off.

```bash
vanic deviations fw.vani
vanic deviations fw.vani --format=json --out=deviations.json
```

---

### `vanic audit-pack <file.vani>`

Run all six audit reports (deviations, stack-depth, acyclicity, hashmap-usage, complexity, safety-attrs) and bundle them into a single Markdown artifact.

```bash
vanic audit-pack fw.vani --out=audit.md
vanic audit-pack fw.vani --max-stack=8192 --max-complexity=20
```

---

### `vanic complexity <file.vani>`

Report per-function Big-O complexity annotations. Same as `--big-o` on run/emit but standalone.

---

### `vanic safety-attrs <file.vani>`

Report which functions carry which safety attributes (`#[no_heap]`, `#[no_float]`, `#[no_nan]`, `#[no_recursion]`, `#[wcet]`, `#[bounded_stack]`, `#[deterministic_timing]`, `#[interrupt]`, composite standards `#[asil_d]` / `#[do178c_level_a]` / `#[misra_c_2012]`, etc.).

---

### `vanic audit-safety <file.vani>`

Verify `#[bounded_stack]`/`#[wcet]` coverage is complete wherever a function
is actually *eligible* for it -- not blanket 100% attribute presence:
fn-pointer params make `#[bounded_stack]` uncomputable, and unbounded
loops/unannotated recursion make `#[wcet]` uncomputable, so both are
legitimately exempt. Vendored `[deps]` functions are excluded. Exit 1 on
any gap. This is what `vanic publish` runs before building the tarball;
also usable standalone against a package or an ordinary program.

```bash
vanic audit-safety src/lib.vani
vanic audit-safety src/lib.vani --format=json
```

---

### Kosh package manager subcommands

| Command | Description |
|---------|-------------|
| `vanic add <name>[@constraint]` | Fetch package from registry -> `vendor/`, update `vani.toml` + `vani.lock` |
| `vanic remove <name>` | Remove package from `vani.toml` |
| `vanic update` | Re-resolve all deps to latest compatible versions |
| `vanic vendor` | Download all deps into `vendor/` |
| `vanic search <query>` | Search the Kosh registry |
| `vanic publish [--allow-partial-safety-coverage]` | Run `audit-safety` (hard-blocks on any gap unless the flag is passed) + build tarball + create GitHub Release + append registry entry |

---

## Global flags

| Flag | Description |
|------|-------------|
| `--no-verify` | Skip SMT verification (type-checking still runs). Also: `VANIC_NO_VERIFY=1`. |
| `--smt-debug` | Dump every Z3 query and response to stderr. Also: `VANIC_SMT_DEBUG=1`. |
| `--deny-warnings` | Escalate every `Severity::Warning` diagnostic (unused variable/parameter, self-assignment, identical if/else branches, a constant-bounds `to`/`downto` mismatch, ...) into a build failure -- CI-style strictness, like rustc's `-D warnings` or gcc's `-Werror`. Works on `check`, `run`, `build`, and `emit`/`emit-c`. Also: `VANIC_DENY_WARNINGS=1`. |
| `-V` / `--version` | Print version and exit. |
| `-h` / `--help` | Print usage and exit. |

---

## Compiler warnings (as of 2026-08-14)

Most vāṇī diagnostics are hard errors -- this is a "catch it at
compile time" language by design. A small, growing set of
diagnostics are genuine **warnings** instead: printed, but they
don't fail the build (unless you pass `--deny-warnings`, above).
Reach for a warning instead of an error when the pattern is *almost
always* a mistake but has a real, legitimate use the compiler can't
tell apart from the mistake at compile time.

Current warnings:

- **Unused variable** -- a `let` binding never referenced again.
- **Unused parameter** -- a function parameter never referenced in
  the body (exempt: `extern "C" fn` declarations, which have no
  body).
- **Self-assignment** -- `x = x;` (almost always a typo for a
  different variable on one side).
- **Identical `if`/`else` branches** -- both branches execute the
  exact same code, so the condition has no effect on behavior.
- **`to`/`downto` bounds direction** -- a `for` loop whose
  compile-time-constant bounds contradict the keyword's direction
  (see [Beginner 5 -- loops](05_loops.md)).

**Silencing a false positive**: prefix the name with an underscore
(`_name`) for the unused-variable/unused-parameter warnings -- e.g.
a lock guard (`Guard<T>`/`ReadGuard<T>`/`WriteGuard<T>`) kept alive
only for its scope-exit unlock, never read directly, should be named
`let _g = mutex_lock(...);`. The other warnings have no suppression
syntax; if the pattern is genuinely intentional, no action is
needed -- it's a warning, not a requirement.

---

## Environment variables summary

| Variable | Effect |
|----------|--------|
| `CC` | C compiler for the C backend. Default: `cc`. |
| `LLC` | LLVM object-code compiler. Default: `llc`. |
| `LLI` | LLVM JIT for `vanic run`. Default: `lli`. |
| `OPT` | LLVM optimizer. Default: `opt`. Skipped gracefully if absent. |
| `Z3` | Z3 SMT solver. Default: `z3`. |
| `CROSS_CC` | Cross-linker override for `--target` builds. |
| `QEMU_<ARCH>` | QEMU user-mode binary override (e.g. `QEMU_AARCH64`). |
| `VANIC_NO_VERIFY=1` | Skip SMT; equivalent to `--no-verify`. |
| `VANIC_SMT_DEBUG=1` | Dump Z3 queries; equivalent to `--smt-debug`. |
| `VANIC_DENY_WARNINGS=1` | Treat every warning as a build failure; equivalent to `--deny-warnings`. |
| `INTENT_TARGET_EMBEDDED=1` | Enable stack-protector hardening on the link line. |
| `INTENT_LIBGOMP` | Non-standard libgomp path for LLVM JIT. |


---

**Next**: [Sec.1 -- Hello, World ->](01_hello_world.md)

