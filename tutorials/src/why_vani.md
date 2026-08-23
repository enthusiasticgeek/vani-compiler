# Why vāṇी?

Every language asks you to trade something for something else. Here's
what vāṇī trades, and what it doesn't, against the three languages
people usually compare it to.

## The short version

- Against **Python**: you keep the readability, but you stop paying
  for it at runtime -- no interpreter, no GC pause, no GIL, and you
  get compile-time type/ownership/contract checking Python doesn't
  attempt.
- Against **C / C++**: you keep the performance and the direct
  hardware access, but the compiler now enforces memory safety and
  can discharge correctness proofs at compile time instead of hoping
  a code reviewer (or a fuzzer) catches the bug first.
- Against **Rust**: you keep the exact same ownership model and
  performance, but the surface reads at speaking pace instead of
  requiring fluency in `&mut Vec<Box<dyn Trait + 'a>>`-style
  punctuation -- and it's not English-only.

None of this is free. vāṇī is young (see the development note on the
project's [README](https://github.com/enthusiasticgeek/vani-compiler#readme)),
and v1 has real, documented limitations. But the tradeoffs above are
the actual bet the language is making, not marketing.

## vs. Python

Python is many people's first language, and for good reason -- it
already reads close to pseudocode. vāṇī isn't trying to out-read
Python. It's trying to keep that readability while removing the
things you'd normally have to give up Python for once a program needs
to be fast, safe under load, or provably correct.

| Aspect | Python | vāṇī |
|---|---|---|
| Execution | Interpreted (CPython bytecode + reference-counting GC) | Compiles to native machine code (LLVM) or portable C -- no interpreter, no GC |
| Typing | Dynamic; type hints are optional and unchecked at runtime by default | Static, checked at compile time |
| Performance | Slow for CPU-bound work; typically needs C extensions (NumPy, etc.) to go fast | Native-code performance, comparable to C / Rust |
| Memory safety | Safe via GC + refcounting, at a real runtime cost (pauses, refcount churn) | Safe via affine ownership, checked at compile time, zero runtime cost |
| Concurrency | The GIL prevents true parallel threads; `multiprocessing` / `asyncio` work around it | Real OS threads: `task` / `join`, `parallel for`, `Atomic` / `Mutex` / `Channel` / `Barrier` / `RwLock` |
| Formal verification | None built in | Z3 SMT: `requires` / `ensures` / `prove`, discharged at compile time |
| Deployment | Needs a matching Python interpreter + dependencies on the target machine | A single native binary; nothing else has to be installed |
| Readability | Already close to pseudocode -- this is vāṇī's benchmark, not its target to beat | Keyword-first surface, same readability goal, across 62 human languages |

If you've ever prototyped in Python and then had to rewrite the hot
path in C for performance, or reach for `multiprocessing` to get real
parallelism, or add `mypy` to catch what dynamic typing missed --
those are exactly the seams vāṇī is trying to remove by starting from
a compiled, statically-typed, ownership-checked foundation instead of
bolting one on later.

## vs. C / C++

*Familiar terrain, lighter outerwear.* If you've written C, vāṇī's
execution model is not a re-invention -- same close-to-the-metal
view, same predictable cost. What changes is who's responsible for
the guardrails.

| Aspect | C / C++ | vāṇī |
|---|---|---|
| Memory safety | Manual (C) or RAII (C++); use-after-free / double-free are the programmer's job to avoid | Affine ownership, compiler-enforced -- these bug classes are compile errors |
| Undefined behavior | Pervasive (uninitialized reads, signed overflow, out-of-bounds access, ...) | Eliminated at the ownership + bounds level; `unsafe` blocks are opt-in and explicit |
| Formal verification | External tools only (if used at all) | First-class `requires` / `ensures` / `prove`, built into the compiler |
| Safety standards | MISRA C, DO-178C, etc. checked by external linters/auditors | `#[misra_c_2012]`, `#[asil_d]`, `#[wcet]`, and friends checked by the compiler itself |
| Readability | Punctuation-heavy | Keyword-first (`ref` / `mut ref` instead of `&` / `&mut`) |
| Interop | N/A | Compiles to C (`--backend=c`), so output drops into an existing C build system without needing LLVM on the target -- a practical bridge for legacy embedded codebases |

This is the pairing vāṇī leans on hardest for its safety-critical
pitch: the same category of bug that MISRA audits and static analyzers
exist to catch in C is, in vāṇī, a compiler error at `vanic build`
time.

## vs. Rust

vāṇī's semantic model *is* Rust's -- affine ownership, second-class
references, monomorphized generics, deterministic drop order, no
garbage collector. This isn't a "Rust-inspired" language with its own
rules; it's the same rules with a different surface.

| Aspect | Rust | vāṇī |
|---|---|---|
| Syntax | Punctuation-heavy (`&`, `&mut`, `?`, `<T>` bounds) | Keyword-first, with natural-language spelling aliases |
| Ownership | Lifetimes + borrow checker | Affine ownership + second-class refs (no lifetime annotations to learn) |
| Generics | Trait bounds + `where` clauses | `where T is Iface` |
| Error handling | `Result<T,E>` + `?` | `Result<T,E>` + `try` (`?` also compiles) |
| Verification | None built in | Z3 SMT: `requires` / `ensures` / `prove` |
| Safety attributes | None | `#[asil_d]`, `#[no_nan]`, `#[wcet]`, ... |
| Multilingual | No | 62 dialects across 26 scripts, opt-in per file via `// vani-lang:` |
| C backend | No | Yes -- `--backend=c` |

The bet here is narrow and specific: `&mut Vec<Box<dyn Trait + 'a>>`
is precise, but it does not read at speaking pace. If a domain expert
who isn't a full-time programmer -- an avionics engineer, a
pharmacologist -- needs to read a safety-critical function's contract,
vāṇī's surface is trying to make that possible without a tutorial on
lifetime syntax, while keeping every guarantee Rust gives you.

vāṇī is an independent project with no affiliation with the Rust
project or Rust Foundation.

## Is vāṇī object-oriented?

Short answer: no, not in the class-and-inheritance sense -- and this
trips up people coming from Java, C#, C++, or Python, so it's worth
stating plainly before you hit `struct` and `methods on` and start
reaching for instincts that don't quite apply.

There is no `class` keyword, no inheritance, and no constructors that
run implicit setup logic. What vāṇī has instead:

| If you're looking for... | vāṇī has |
|---|---|
| A class | `struct` (plain data) + a separate `methods on TypeName { ... }` block that attaches functions to it |
| A constructor | An ordinary function that returns the struct by value (`fn new() -> Point { return Point { x: 0, y: 0 }; }`) -- no hidden initialization, nothing runs that isn't written on the page |
| Inheritance / subclassing | Nothing -- there is no "extends," no base class, no virtual dispatch table for a struct hierarchy |
| Polymorphism | `interface` + `dyn Iface` (trait-object-style dynamic dispatch, one vtable, no diamond problem) or `where T is Iface` (compile-time generic dispatch, monomorphized -- zero runtime cost) |
| Encapsulation | Module-level visibility (`pub`/private, see [Beginner 9a -- Modules primer](beginner/09a_modules_primer.md)), not per-object access control |
| Data with multiple shapes | `enum` with payloads + `match` -- a sum type, not a class hierarchy |

`methods on Point { fn area(self: ref Point) -> i64 { ... } }` looks
like a class from a distance -- you call it `p.area()` -- but it's
sugar over a free function that happens to take `Point` (or `ref
Point` / `mut ref Point`) as its first argument. There's no dynamic
dispatch involved unless you explicitly reach for `dyn Iface`; a
`struct`'s methods are resolved at compile time, same cost as calling
any other function.

If a label helps: vāṇī is closer to **Rust's paradigm** than to
Java's or Python's -- data and behavior are related but not fused,
polymorphism is opt-in and explicit (interfaces, not implicit
subclassing), and the thing actually enforcing correctness across all
of it is the ownership system, not the object model. [Intermediate 1
-- Structs and methods](intermediate/01_struct_methods.md) is where
this becomes concrete with real code; [Intermediate 11a -- vāṇī
idioms primer](intermediate/11a_vani_idioms_primer.md) and
[Intermediate 11b -- SOLID primer](intermediate/11b_solid_primer.md)
walk through the specific idiom to reach for when an OOP instinct
(a base class, a Strategy object, a Decorator) doesn't have a direct
vāṇī equivalent.

## What this doesn't mean

vāṇī is not claiming to be strictly better than any of these
languages -- each has a domain where it's still the right tool
(Python for quick scripts and data-science glue, C for existing
embedded codebases you're not rewriting, Rust for a mature ecosystem
and crates.io). vāṇī is making a specific bet for a specific gap: code
that needs Rust/C-grade performance and safety guarantees, but where
punctuation-dense syntax and English-only keywords are a real barrier
-- for beginners, for non-native English speakers, or for domain
experts who need to read and sign off on the contracts without
learning the language first.

---

Ready to see it in practice? **[Begin with `Hello, World` ->](beginner/01_hello_world.md)**
