# Intermediate 8 -- Multi-file projects + `vani.toml`

> **Learning goal**: split a program across multiple `.vani`
> files, wire them together with a `vani.toml` manifest, and
> import items between files with `use "path";` declarations.

Think of a multi-file project as a building with separate
rooms. Each `.vani` file is a room; the `vani.toml` manifest
is the building's floor plan that lists all the rooms and
tells the compiler how they connect. `use "path/to/room.vani";`
is how a room says "I need to borrow some furniture from
room B." The manifest exists so you can type `vanic build .`
in the project root and have the compiler find all the rooms
automatically without you listing every file by hand.

## The project layout

```
int8_proj/
+-- vani.toml
+-- src/
    +-- main.vani
    +-- math.vani
```

`vani.toml`:

```toml
[package]
name = "int8_demo"
entry = "src/main.vani"
```

`src/math.vani`:

```vani
intent "Math helpers module.";

module math {
  pub fn square(n: i64) -> i64 {
    return n * n;
  }

  pub fn cube(n: i64) -> i64 {
    return n * n * n;
  }
}
```

`src/main.vani`:

```vani
use "math.vani";
use math::{square, cube};

intent "Multi-file demo -- uses the math helpers next door.";

fn main() -> i64 {
  print "square(5) =", square(5);
  print "cube(3)   =", cube(3);
  return 0;
}
```

## Compile + run

From the project root:

```bash
cd int8_proj
vanic run
```

(No file argument -- the driver walks up looking for `vani.toml`
and uses its `[package].entry` as the source file.)

Output:

```
square(5) = 25
cube(3)   = 27
```

## Why it works that way

- **`vani.toml`** is the project manifest. Minimal v1 shape:
  one `[package]` table with a `name` and an `entry`. The
  `entry` path is relative to the manifest's directory.
- **`use "path";`** at the top of a `.vani` file is the
  file-include directive. The path is relative to the current
  file's directory. The included file's contents are spliced
  into the same translation unit before compilation.
- **`use module::item;`** (no quotes) is the namespace import.
  After splicing, both files share a global module namespace;
  this brings the specific item into bare-name scope. Multi-
  item form: `use module::{a, b, c};`.
- **`pub` requires a module**. Top-level `pub fn` outside a
  `module { ... }` block doesn't parse -- wrap your helpers
  in a `module name { ... }` (per Beginner Sec.10) before
  marking them `pub`.
- **No transitive includes**: a `use "a.vani";` in
  `b.vani` doesn't propagate `a`'s items to whichever file
  uses `b`. Re-export them with `pub use foo::bar;` if you
  need that.

## Caveats in v1

- **One file per `use`** in v1 -- globbing (`use "*.vani";`) is
  deferred.
- **Cyclic includes** are silently dropped: each file is
  included at most once across the dependency tree. Don't
  rely on order-of-inclusion semantics; design with one-way
  dependencies.
- **Diagnostic line numbers** refer to the concatenated buffer,
  not the per-file source. Real per-file mapping is a follow-up.
  Until then, when reading errors, grep for the column of the
  surrounding `fn` to find your file.

## Challenge

Add a third file `src/util.vani` defining a `module util` with
a `pub fn clamp(x: i64, lo: i64, hi: i64) -> i64` that returns
`x` clamped between `lo` and `hi`. Wire it into `main.vani` with
another `use` and call it.

---

**Next**: [Sec.9 -- FFI: `extern "C"` + `--link-with` ->](09_ffi.md)
