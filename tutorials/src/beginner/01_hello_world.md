# Beginner 1 -- Hello, World

> **Learning goal**: write your first vāṇी program, compile it
> with the C and the LLVM backend, and read the output.

## The program

Save this in `~/hello.vani`:

```vani
intent "First vāṇी program -- prints a greeting and returns 0.";

fn main() -> i64 {
  print "Hello, vāṇी!";
  return 0;
}
```

Three things to notice already:

- The first line is `intent "...";`. Every vāṇी file declares its
  *intent* -- a free-text description of what the program does.
  It's not a comment: the compiler accepts it as a statement, so
  you'll see it referenced when we get to SMT contracts.
- The entry point is `fn main() -> i64`. `i64` is a signed
  64-bit integer; `main` must return it (this is your shell exit
  code).
- `print` is a statement, not a function call. You'll see why in
  [Sec.3 Functions](03_functions.md) -- it's part of a small family
  of *verb-at-end* aliases that comes from vāṇी's dialect
  support.

## Compile + run

Two backends ship with the compiler. **LLVM** is the default
because the LLVM IR is portable; **C** is a fallback that's a
good debugging surface (you can read the generated C).

```bash
# LLVM backend (default) -- runs via `lli`
vanic run ~/hello.vani

# C backend -- emits C, invokes your system `cc`, runs the
# resulting binary
vanic run ~/hello.vani --backend=c
```

Both should print:

```
Hello, vāṇी!
```

If you want the artifacts on disk for inspection:

```bash
vanic emit ~/hello.vani                  # LLVM IR to stdout
vanic emit ~/hello.vani --backend=c      # C source to stdout
vanic build ~/hello.vani -o ~/hello      # native binary at ~/hello
```

## Why it works that way

A few one-line answers for the things you'll wonder about:

- **Why `intent "...";`?** It anchors what the file is *for*. The
  beginner-friendly answer is "it's documentation that lives in
  the AST." The fuller answer involves the SMT verifier and
  shows up in [Sec.9](09_smt_intro.md).
- **Why must `main` return `i64`?** Because the shell exit code
  is a byte and a signed 64-bit integer is the smallest type
  that's both wide enough and easy to write into. There's no
  `int` keyword -- vāṇी is strict about width (`i8`, `i16`, `i32`,
  `i64`, plus the unsigned siblings).
- **Why no `;` after the function body's `}`?** Same convention
  as Rust: braces are statements, semicolons aren't required to
  close them.

## Challenge

Modify `hello.vani` to print **two** lines. The compiler accepts
multiple `print` statements in sequence. Try it before peeking
at the solution.

<details>
<summary>Solution</summary>

```vani
intent "Two-line greeting.";

fn main() -> i64 {
  print "Hello, vāṇी!";
  print "Welcome to the tutorial.";
  return 0;
}
```

You can also pass multiple arguments to a single `print` -- the
runtime prints them separated by spaces:

```vani
print "Hello,", "vāṇी!";
```

</details>

---

**Previous**: [Sec.0 -- CLI reference ->](00_cli_reference.md)
**Next**: [Sec.1b -- Block comments primer ->](01b_block_comments_primer.md)
