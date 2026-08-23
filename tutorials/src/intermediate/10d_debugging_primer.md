# Intermediate 10d -- Debugging with gdb/lldb (primer)

> **Learning goal**: know what to reach for when `print` statements
> aren't enough -- attaching a real debugger to a compiled vāṇी
> program, setting breakpoints, inspecting variables, and reading a
> backtrace. Reading order: [Sec.10c -- Error patterns
> primer](10c_error_patterns_primer.md) -> here -> [Sec.13 --
> `Option<T>`](13_option.md).

## Why this needs its own chapter

A `vanic build`/`vanic run` binary is not a script running inside an
interpreter -- it's an ordinary native executable, the same kind
`gdb` and `lldb` already know how to debug, because vāṇी compiles
through the same two pipelines (LLVM, or plain C11 via `cc`) every
other systems language uses. Nothing about vāṇī itself changes how a
debugger works. What this chapter covers is specific to vāṇी: which
build command gets you a binary a debugger can actually make sense
of, and what your original variable names look like once they've
gone through the compiler.

## The workflow that gives you full source-level debugging

`vanic build` (the LLVM path) produces a working, unstripped binary,
but without the extra line-number/variable debug info (`-g`-style
DWARF data) a debugger needs for source stepping. For that, emit C
and compile it yourself with `-g`:

```bash
vanic emit myprogram.vani --backend=c -o myprogram.c
cc -g -O0 myprogram.c -o myprogram_debug -pthread -fopenmp -lm
gdb ./myprogram_debug          # or: lldb ./myprogram_debug
```

`-O0` matters as much as `-g` -- an optimized build can reorder or
eliminate the exact statements you're trying to step through, making
the debugger's view of "where you are" misleading. Debug builds and
release builds are different builds; don't ship the `-g -O0` one.

## What your variable names look like

The compiler doesn't preserve your source-level names in the emitted
C -- parameters and locals become `v_0`, `v_1`, `v_2`, ... in
declaration order, and a `fn` you wrote becomes `fn_<name>` in the
symbol table (so `fn add(...)` is `fn_add` to gdb). Nothing is lost
functionally -- `print v_0` shows the real value -- but you have to
map the compiler's names back to yours by reading the surrounding
source line gdb shows you, not by name alone.

```vani
fn add(a: i64, b: i64) -> i64 {
  let sum: i64 = a + b;
  return sum;
}

fn main() -> i64 {
  let x: i64 = add(3, 4);
  print x;
  return 0;
}
```

```
$ gdb -batch -ex "break fn_add" -ex "run" -ex "print v_0" -ex "print v_1" \
      -ex "next" -ex "next" -ex "print v_2" ./myprogram_debug

Breakpoint 1, fn_add (v_0=3, v_1=4) at myprogram.c:2
2  if (__builtin_expect(__builtin_add_overflow(...), 0)) { ... }
$1 = 3
$2 = 4
3  return v_2;
4  }
$3 = 7
```

`a` -> `v_0`, `b` -> `v_1`, `sum` -> `v_2`, in the order they were
declared. A `bt` (backtrace) at that breakpoint shows the real call
chain -- `fn_add` called from `fn_main`, called from C's own `main`
(the tiny `int main(void) { return (int)fn_main(); }` wrapper every
vāṇी program gets):

```
#0  fn_add (v_0=3, v_1=4) at myprogram.c:2
#1  0x... in fn_main () at myprogram.c:8
#2  0x... in main () at myprogram.c:14
```

## Common breakpoint targets

| To break on... | Set a breakpoint on |
|---|---|
| A specific function | `fn_<name>` (e.g. `break fn_add`) |
| The very start of the program | `fn_main` (not C's `main` -- that's just the one-line wrapper) |
| A crash / trap (bounds check, overflow, `assert` failure) | `exit` -- every checked-arithmetic trap, bounds check, and failed `assert` prints its message, then calls libc's `exit(3)` directly at the spot it happened (no shared trap function to name on the C backend), so breaking on `exit` itself catches all of them |

```bash
gdb -batch -ex "break exit" -ex "run" -ex "bt" ./myprogram_debug
```

is the fastest way to turn "my program printed an assertion message
and exited 3, where did that come from?" into an exact source line
and call stack, without adding a single `print` statement. Confirmed
against an out-of-bounds `xs[i]`:

```
Breakpoint 1, __GI_exit (status=3) at ./stdlib/exit.c:148
#0  __GI_exit (status=3) at ./stdlib/exit.c:148
#1  0x... in fn_risky (v_0=..., v_1=10) at myprogram.c:590
#2  0x... in fn_main () at myprogram.c:604
#3  0x... in main () at myprogram.c:613
```

Frame `#1` is the exact line the bounds check fired on, and `v_1=10`
is the out-of-range index that triggered it -- read straight off the
backtrace, no `print` statements added to the program at all. `status
= 3` on the `exit` frame confirms it's a genuine checked trap (versus
a real segfault, which gdb reports as a signal instead of a clean
`exit` breakpoint hit).

## Why not just `gdb` the `vanic build` binary directly?

`vanic run` (no `--backend=c`) JITs through `lli`; there's no
standalone binary to attach a debugger to mid-run at all. `vanic
build` does produce a real, unstripped native binary -- but it always
lowers through `llc -O=3` with no way to opt out in v1, so a small
function like the `add` example above gets inlined away entirely.
Confirmed directly: `break fn_add` on a `vanic build` binary sets the
breakpoint without error, but it never fires -- the function simply
isn't a distinct call anymore by the time the optimizer is done with
it. The symbol not being *stripped* isn't the same as the code being
debuggable. The C-backend `-g -O0` workflow above is the one
reliable path to real step-through debugging in v1 -- it's not just
the more convenient option, it's currently the only one that works.

## Reading a crash without a debugger at all

Most of the time you won't need gdb -- vāṇी's traps are designed to
be self-explanatory. A bounds-check failure, an overflow, or a failed
`assert` prints exactly what went wrong (`integer overflow in int64_t
add`, `assertion failed: ...`) before exiting with a small, meaningful
code (`3` for a checked-arithmetic trap; see [Sec.10b -- Runtime
errors primer](10b_runtime_errors_primer.md) for the exit-code
table). Reach for a debugger specifically when the message alone
doesn't tell you *which call path* got there, or when you need to
inspect a value's exact state right before things went wrong --
`__intent_trap` breakpoints and `bt` are the fast path to both.

## Try it yourself

Take any program you've already written in this tutorial series,
build it with the `-g -O0` recipe above, and set a breakpoint on one
of its functions. Step through with `next`, print a couple of
locals by their `v_N` names, and confirm they match what you'd
expect from reading the source. Then deliberately introduce a bug
that trips an assertion or bounds check, and use `break
__intent_trap` + `bt` to find it without adding a single `print`
statement to your own code.

## Summary

- A compiled vāṇी program is an ordinary native executable -- gdb/lldb
  already know how to debug it, nothing vāṇी-specific about the
  debugger itself.
- The one reliable path to real step-through debugging in v1 is
  `vanic emit --backend=c` + `cc -g -O0` -- `vanic build`'s LLVM path
  always optimizes at `-O3` with no opt-out, which inlines small
  functions away entirely (confirmed: a breakpoint on an inlined
  function sets without error but never fires).
- Your variable names become `v_0`, `v_1`, ... in declaration order;
  your functions become `fn_<name>`. Read the source line gdb shows
  you to map back.
- `break exit` catches every checked-arithmetic trap, bounds check,
  and `assert` failure in one place on the C backend (they all call
  `exit()` directly, no shared trap function to name) -- the fastest
  way to turn a crash message into an exact call stack.

---

## Cross-references

- [Sec.10b -- Runtime errors, panic-free design primer](10b_runtime_errors_primer.md) -- the exit-code table for every trap kind
- [Sec.9d -- Build-system integration](09d_build_systems.md) -- more on `vanic emit --backend=c` + `cc` flags
- [Advanced 6 -- SMT trace debugging](../advanced/06_smt_debug.md) -- debugging a *compile-time proof failure*, a different (and earlier) kind of debugging than this chapter's runtime focus

---

**Previous**: [Sec.10c -- Error patterns primer ->](10c_error_patterns_primer.md)
**Next**: [Sec.13 -- `Option<T>` and the option builtins ->](13_option.md)
