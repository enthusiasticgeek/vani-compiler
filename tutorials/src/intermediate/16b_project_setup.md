# Intermediate 16b -- Setting up a real project, step by step

> **Learning goal**: go from an empty directory to a multi-file,
> tested, CI-checked vāṇी project -- tying together the manifest,
> modules, Kosh, testing, and build-system chapters you've already
> read into one continuous workflow, plus the parts none of them
> cover on their own: when to split code into a new file vs. a new
> module, what to commit to version control, and a minimal CI
> workflow. No new language features here -- this is process, not
> syntax.

This chapter assumes you've read [Sec.8 -- Multi-file projects +
`vani.toml`](08_manifest.md), [Sec.16 -- Packages with
Kosh](16_packages.md), [Sec.16a -- Testing your vāṇी
code](16a_testing_primer.md), and skimmed [Sec.9d -- Build-system
integration](09d_build_systems.md) at least once. Each of those is a
deep, standalone reference for its one piece. This chapter is the
narrative that connects them -- the sequence of decisions you'd
actually make starting a real project on day one, plus the judgment
calls (split now or not yet, commit `vendor/` or not) the reference
chapters correctly leave to you.

## Step 1: the manifest and the first file

[Sec.8](08_manifest.md) covers the mechanics in full: a `vani.toml`
at the project root with one `[package]` table (`name` + `entry`),
`use "path.vani";` to splice another file in, `use module::item;` to
bring a namespaced item into bare-name scope, and the two sharp edges
worth re-reading before you rely on either -- includes are **not
transitive** (`use "a.vani";` in `b.vani` doesn't propagate to
whoever `use`s `b.vani`) and **cyclic includes silently no-op**
instead of erroring. There's no `vanic new` scaffolding command in
v1 -- you write `vani.toml` by hand, then create the file `entry`
points at:

```
myproject/
├── vani.toml
└── src/
    └── main.vani
```

`vanic run` (no file argument) from anywhere inside `myproject/`
walks up to find `vani.toml` and uses `entry` -- the same manifest
`vanic add`/`vanic publish`/`vanic test` all key off. This is the
one piece of infrastructure everything else in this chapter builds on
top of.

## Step 2: when to split, not just how

Sec.8 shows *how* `use "path.vani";` and `module { ... }` work; the
judgment call of *when* to reach for each is the part that's easy to
get wrong in practice, because both are always available and neither
forces your hand:

- **A new file** is a *physical* split -- reach for it when a piece
  of the program is big enough, or change-independent enough, that it
  deserves its own file on disk (a whole subsystem: `db.vani`,
  `http.vani`, `math.vani`).
- **A `module { ... }` block** (see [Beginner 9a -- Modules
  primer](../beginner/09a_modules_primer.md)) is a *namespace* split
  -- reach for it to avoid name collisions (`net::process` vs.
  `parser::process`) and to control visibility (`pub fn` vs. private),
  independent of which file anything lives in. Note `pub` only parses
  inside a `module { ... }` block -- a bare top-level `pub fn` is a
  syntax error, so this isn't purely optional structure once another
  file needs to import something.

The two compose -- a file can contain a module, and a module's
contents can be spread across files a project grows into.

**A rule of thumb for when to split at all**: don't, until a file
is genuinely hard to scroll through, or two people keep touching the
same file for unrelated reasons (a real merge-conflict signal). A
100-line `main.vani` doesn't need `db.vani` carved out of it yet.

## Step 3: adding a dependency

Once the project needs something it shouldn't reinvent (say, a math
routine from the `kosh-index` registry), add it the same way
[Sec.16](16_packages.md) walks through in full:

```bash
vanic add hello-kosh
```

This updates `vani.toml`'s `[deps]`, vendors the source into
`vendor/hello-kosh/`, and writes/updates `vani.lock`. Nothing about
this step changes because the project now has multiple files -- Kosh
operates on the manifest at the project root regardless of how many
`.vani` files hang off `entry`.

## Step 4: tests

Add `#[test]` functions next to the code they cover (same file or a
dedicated `tests.vani` pulled in with `use`), and run them with:

```bash
vanic test
```

[Sec.16a](16a_testing_primer.md) covers the full picture --
`--filter`, `#[should_panic]`, `assert_eq_*`, `--json` for machine
consumption. The one habit worth forming here: write the test in the
same commit as the code it covers, not as a follow-up. A project with
tests scattered across files but none next to the function that most
needs one is a project where `vanic test` gives false confidence.

## Step 5: wire up an external build (if you need one)

If `myproject` is a standalone vāṇी program, you don't need this step
-- `vanic run` / `vanic build` is the whole build system. Reach for
[Sec.9d](09d_build_systems.md) when vāṇी code needs to link into an
*existing* C/C++ project (a shared library, an app that embeds vāṇी
as one component): `vanic emit --backend=c` (or `--backend=llvm` +
`llc`) produces output any Makefile / CMake / Meson / Ninja project
can compile like any other translation unit.
[`examples/build_systems/myproject/`](https://github.com/enthusiasticgeek/vani-compiler/tree/main/examples/build_systems/myproject)
is a real project shaped exactly like Step 1-2's -- a `main.vani`
that `use`s a `math.vani` module -- extended with FFI (`#[no_mangle]
fn vani_square`) and all four build files, wired up and CI-checked --
worth cloning and building locally before you adapt any of it to your
own project.

## Step 6: version control

What to commit, and why:

| Path | Commit? |
|---|---|
| `vani.toml` | Yes -- the manifest is the project |
| `src/**/*.vani` | Yes -- your actual code |
| `vani.lock` | **Yes.** It records exact resolved dependency versions; without it, two clones of the same repo can silently resolve different versions of a `^1.0`-style constraint. Same reasoning as `Cargo.lock` for a binary/application (vs. a library, where the convention differs) -- a vāṇी project is almost always the "commit the lock" case. |
| `vendor/` | Your call. **Commit it** for fully reproducible builds with zero network access at clone time (the safer default for a small team or a safety-critical project). **`.gitignore` it** for a smaller repo, with teammates running `vanic vendor` once after cloning. |
| `build/`, `build-*/`, other generated output | No -- `.gitignore` it. Regenerable from source every time. |

A minimal `.gitignore` for a project that chooses not to commit
`vendor/`:

```gitignore
vendor/
build/
*.o
*.ll
```

## Step 7: a minimal CI workflow

The goal of CI here is narrow: catch a broken `vanic check`/`vanic
test` before it lands on `main`, the same job your local pre-push
habit already does, just enforced. A GitHub Actions example using the
pre-built-binary install path from [Installation](../installation.md)
(no Rust toolchain needed just to *use* vāṇी, only to build the
compiler itself, which this workflow doesn't do):

```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  check-and-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install vanic
        run: curl -fsSL https://raw.githubusercontent.com/enthusiasticgeek/vani-compiler/main/install.sh | sh

      - name: Install z3, llvm, clang
        run: sudo apt-get update -q && sudo apt-get install -q -y z3 llvm clang

      - name: Type-check
        run: vanic check src/

      - name: Run tests
        run: vanic test
```

If the project vendors dependencies and chose to `.gitignore`
`vendor/` in Step 6, add a `vanic vendor` step before `check`/`test`
so CI resolves the same dependency tree `vani.lock` records.

## Putting it together: the full layout

```
myproject/
├── .github/workflows/ci.yml   <- Step 7
├── .gitignore                 <- Step 6
├── vani.toml                  <- Step 1
├── vani.lock                  <- Step 3 (committed)
├── vendor/
│   └── hello-kosh/             <- Step 3 (committed or not, your call)
├── src/
│   ├── main.vani               <- Step 1
│   ├── math.vani                <- Step 2
│   └── tests.vani                <- Step 4
├── Makefile / CMakeLists.txt   <- Step 5 (only if embedding in a C/C++ build)
└── README.md
```

Every piece of this is something you already know how to build --
this chapter's only new contribution is the order to build it in, and
the judgment calls (split now or not yet; commit `vendor/` or not)
that the reference chapters correctly leave to you.

## Try it yourself

Starting from an empty directory, reproduce Steps 1-4 for a tiny
project of your own (anything -- a unit converter, a word-counter).
Then clone
[`examples/build_systems/myproject/`](https://github.com/enthusiasticgeek/vani-compiler/tree/main/examples/build_systems/myproject)
and build it all four ways (`make`, `cmake --build`, `meson compile`,
`ninja`) to see Step 5 as a real, working project instead of a
snippet.

## Summary

- No `vanic new` in v1 -- write `vani.toml` + `src/main.vani` by
  hand; every other command (`run`, `add`, `test`, `publish`) keys
  off that one manifest.
- Split into a **new file** for a physical/size reason; split into a
  **module** for a namespace/visibility reason. They compose.
- `vanic add` / `vanic test` / `vanic emit` all work unchanged as the
  project grows -- multi-file is not a special mode.
- Commit `vani.toml` and `vani.lock` always; `vendor/` is a judgment
  call; never commit generated build output.
- A minimal CI job is just `vanic check` + `vanic test` behind the
  pre-built-binary installer -- no Rust toolchain required for a
  project that only *uses* vāṇी.

---

## Cross-references

- [Sec.8 -- Multi-file projects + `vani.toml`](08_manifest.md) -- `use "path.vani";`, `use module::item;`, cyclic-include/transitivity caveats
- [Sec.16 -- Packages with Kosh](16_packages.md) -- the full manifest/dependency/publishing reference
- [Sec.16a -- Testing your vāṇी code](16a_testing_primer.md) -- `#[test]`, `vanic test`, `assert_eq_*`
- [Sec.9d -- Build-system integration](09d_build_systems.md) -- Makefile/CMake/Meson/Ninja, embedding vāṇी in a C/C++ build
- [Beginner 9a -- Modules and namespaces primer](../beginner/09a_modules_primer.md) -- the intuition behind `module`/`pub`
- [`examples/build_systems/myproject/`](https://github.com/enthusiasticgeek/vani-compiler/tree/main/examples/build_systems/myproject) -- the real, CI-checked project this chapter walks through building

---

**Previous**: [Sec.16a -- Testing your vāṇी code ->](16a_testing_primer.md)
**Next**: [Sec.17 -- Capstone: a terminal tic-tac-toe game ->](17_tic_tac_toe_capstone.md)
