# Beginner 9a -- Modules and namespaces (intuition primer)

> **Learning goal**: build a mental model of "module" (vāṇी's
> word for "namespace") -- why you'd partition code into named
> regions, how visibility works, and why `pub` exists. Reading
> order: this is foundational; read it before
> [Beginner 10 -- Modules and `pub`](10_modules.md) for the
> actual syntax + worked example.

This chapter has **no compiler code**. Pure intuition.

## The problem: name collisions

You're writing a small program. You have a function `process`.
Done. Easy.

Now your program grows. You add a network layer that needs its
own `process`. And a parser. And a logger. Suddenly:

- `process` (the one you started with -- does what again?)
- `process` (network -- handles packets)
- `process` (parser -- handles tokens)
- `process` (logger -- handles log lines)

Four `process` functions can't live in the same namespace --
the compiler doesn't know which one you mean when you write
`process(x)`. And even if it could, YOU wouldn't be able to
tell them apart at a glance.

**Modules** solve this. A module is a named region -- like a
folder -- that contains its own functions, types, and other
modules. Names inside the module are scoped to it. The
qualified name `net::process` is distinct from
`parser::process` is distinct from `logger::process`.

## The folder analogy

Think of your computer's file system:

```
~/work/
+-- recipes/
|   +-- chocolate-cake.md
+-- code/
|   +-- chocolate-cake.md     <- someone else's, different file
+-- notes/
    +-- chocolate-cake.md     <- yet another, different again
```

Three files named `chocolate-cake.md`, all coexisting because
they're in different folders. To reach one specifically, you
write `~/work/recipes/chocolate-cake.md`.

Modules are folders for code. To reach `process` inside the
`net` module specifically, you write `net::process`.

## Why partition at all?

Three reasons. Two are obvious; the third is the load-bearing
one.

### 1. Naming room

As above -- multiple things can share a short name across
modules without collision.

### 2. Mental organization

`fn parse(s: Str) -> Tokens` belongs near other parser logic.
Grouping related code into a module gives the reader a clue
about what's logically connected.

### 3. Visibility (the load-bearing one)

This is the real reason. Modules let you mark some items as
"public" (the module's exported API) and others as "private"
(internal helpers nobody outside should call).

```
module parser {
  pub fn parse(s: Str) -> Tokens { ... }   // <- exported

  fn tokenize_internal(s: Str) -> Vec<Token> { ... }
  //  ^-- no pub; only parse can call this
}
```

Outside the `parser` module, you write `parser::parse(...)` --
that's allowed because `parse` has `pub`. You CANNOT write
`parser::tokenize_internal(...)` from outside -- the compiler
rejects it.

This is **encapsulation**. The module's author publishes a
small, deliberate API. The internals can change freely without
breaking callers -- only the `pub`-marked items are the
contract.

## Why is this important?

Without visibility controls, every helper function is part of
your public API. Any user can call it. Any change to its
signature breaks their code. You either:

- Never change anything -> stagnation.
- Change things constantly -> users hate you.

`pub` lets you draw a tight contract: "here's what I promise
to keep stable. Everything else is internal and may change
without notice."

The opposite mistake is also avoided: a user can't accidentally
depend on a helper that wasn't intended as API. They'd hit a
"private item" error and have to switch to a `pub` alternative
-- a chance for both sides to do the right thing.

## Three visibility tiers

The default is **private** -- visible only inside the declaring
module.

- `pub fn foo(...)` -> visible **outside the module**, callable
  from anywhere via the qualified path (`mod::foo`), including from
  a different Kosh package that depends on this one.
- (no `pub`) -> visible **only inside this module**. Default.
- `pub(kosh) fn bar(...)` -> visible **within your own project**:
  bare, unqualified calls from inside the same module work (like
  `pub`/private always have), and so does a *qualified* `mod::bar`
  reference from a sibling module elsewhere in your project. What's
  rejected is a *different* Kosh package (a `[deps]` consumer)
  reaching for it via `mod::bar` -- that's the boundary `pub(kosh)`
  exists to enforce.

`kosh` (कोश, "treasure / repository") is vāṇी's word for what
Rust calls a "crate" -- a single buildable package. `pub(kosh)`
protects a package's internal helper from external consumers (the
case that matters most -- see [Sec.16 -- Kosh Packages](../intermediate/16_packages.md))
while still letting every module inside your own project share it.

Here are all three tiers side by side:

```
module stats {
    // Tier 1 — public API. Callers from anywhere can use this,
    // including a different Kosh package consuming this one.
    pub fn mean(xs: ref Vec<i64>, n: i64) -> i64 {
        return sum_all(xs) / n;
    }

    // Tier 2 — package-internal: other modules in YOUR OWN project
    // can call this (e.g. a sibling `module report`), while an
    // external Kosh consumer cannot.
    pub(kosh) fn sum_all(xs: ref Vec<i64>) -> i64 {
        let s: i64 = 0;
        let i: i64 = 0;
        while i < (xs.len() as i64) {
            s = s + xs[i];
            i = i + 1;
        }
        return s;
    }

    // Tier 3 — private. Only callable from inside this module.
    fn assert_nonempty(n: i64) -> i64 {
        assert n > 0, "stats require at least one element";
        return n;
    }
}

// From inside the SAME module (e.g. mean calling sum_all bare, above):
//   sum_all(xs)                -- OK, bare intra-module call always works
//   assert_nonempty(n)         -- OK, bare intra-module call always works

// From a sibling module in the SAME project (e.g. module report):
//   stats::mean(ref xs, n)     -- OK, pub
//   stats::sum_all(ref xs)     -- OK, pub(kosh) allows same-project access
//   stats::assert_nonempty(n)  -- REJECTED: private to stats

// From a DIFFERENT Kosh package consuming `stats` via [deps]:
//   stats::mean(ref xs, n)     -- OK, pub
//   stats::sum_all(ref xs)     -- REJECTED: pub(kosh) stops external
//                                  Kosh-boundary access
//   stats::assert_nonempty(n)  -- REJECTED: private to stats
```

The rule of thumb: use `pub` for the API you're willing to support
forever, including external Kosh consumers; use `pub(kosh)` for a
package-internal helper you want to share freely across your own
project's modules but hide from anyone depending on your package.

## Nested modules

Modules can contain modules:

```
module net {
  pub module tcp {
    pub fn listen(port: i64) -> i64 { ... }
  }
  pub module udp {
    pub fn bind(port: i64) -> i64 { ... }
  }
}
```

The path is read left-to-right with `::`:
- `net::tcp::listen(80)`
- `net::udp::bind(53)`

This nests as deep as you like. Filesystem analogy: nested
folders. Convention: keep nesting shallow (3 levels max is a
good guideline; more starts feeling labyrinthine).

## `use` -- shortcuts

Writing `net::tcp::listen(80)` everywhere gets tedious. The
`use` statement creates a local alias:

```
use net::tcp::listen;
// now `listen(80)` works locally without the full path
```

Or shorten just the module:

```
use net::tcp;
// now `tcp::listen(80)` works -- saves the `net::` prefix
```

Or pull several things at once:

```
use net::tcp::{listen, accept, connect};
```

The `use` declarations sit at the top of a file or module body.
They affect lookups in the surrounding scope.

## A summary you can carry

- A **module** is a named region containing functions, types,
  and other modules. Like a folder in a filesystem.
- Modules let you have multiple things with the same short
  name without collisions (`net::process` != `parser::process`).
- The real reason for modules is **visibility**: `pub` marks
  items as part of the module's exported API; non-`pub` items
  are private to the module.
- `pub(kosh)` is a middle tier: it correctly blocks a *different*
  Kosh package from reaching in via `pkgname::item`, but doesn't
  yet support the intended same-project sibling-module sharing
  (that part still needs `pub`, or moving the sharing into one
  module).
- `use mod::name;` imports a name locally so you don't have
  to write the full path every time.

That's modules. The next chapter ([Beginner 10](10_modules.md))
shows the actual syntax + a worked example.

## Why programmers care about visibility (one more reason)

In a large codebase, two developers might both want to write
helper functions named `validate`. Without modules, they
collide. With modules, each can have their own private
`validate` invisible to the other.

The same goes for FUTURE you, three months later, refactoring
half the code. If your helpers are private, you can rename or
delete them with confidence -- nobody outside the module is
relying on them. If they leaked into the public API, you can't
touch them without coordination.

The discipline of "smallest possible `pub`" is what makes
larger projects refactor-able without breaking the world.

## Cross-reference

- [Beginner 10 -- Modules and `pub`](10_modules.md) -- actual
  syntax + worked example
- [Intermediate 8 -- Multi-file projects + `vani.toml`](../intermediate/08_manifest.md)
  -- how modules grow across multiple files
- [Intermediate 4b -- Interfaces primer](../intermediate/04b_interfaces_primer.md)
  -- modules + interfaces compose: a module can export an
  interface that several types in different modules implement
- [Intermediate 16 -- Kosh Packages](../intermediate/16_packages.md)
  -- every `[deps]` dependency is compiled inside an implicit
  `module <pkg_name> { ... }` automatically, using this exact
  mechanism -- that's why calling a dependency's function looks
  like `matrix::mat_solve(...)` instead of a bare call


---

**Previous**: [Sec.9 -- First contract: assert / prove / requires ->](09_smt_intro.md)
**Next**: [Sec.10 -- Modules and pub ->](10_modules.md)

