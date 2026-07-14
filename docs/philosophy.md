# vāṇī — Design Philosophy

---

## The core bet

Most programming languages treat natural-language readability as cosmetic —
something you add after the semantics are locked. vāṇī inverts this: the
natural-language surface is the primary design constraint, and the semantic
model (borrowed from Rust) is chosen to be *expressible* in that surface without
losing any safety guarantees.

The three commitments that make this work:

1. **Same execution model as Rust / C / C++.** No interpreter, no GC, no hidden
   allocator. The output is deterministic LLVM IR or C. `prove` / `ensures` /
   `requires` are discharged at compile time by Z3; runtime cost matches
   idiomatic Rust.

2. **Multiple spellings, one AST.** `return`, `give`, `give_back`, `give back`
   all produce the same `TokenKind`. The checker, SMT layer, and backends see
   identical IR regardless of which spelling the writer chose. Two files that
   differ only in alias choice produce byte-identical LLVM IR after `vanic fmt`.

3. **Keywords replace punctuation where it reads hardest.** `ref` / `mut ref`
   instead of `&` / `&mut`. `try` instead of `?` (though `?` also works).
   `module foo::bar` instead of `mod foo { mod bar { … } }`. The model is
   strictly Rust-compatible; only the surface changes.

---

## Why not Rust syntax?

Rust's punctuation is optimised for terseness, not readability-at-a-glance.
`&mut Vec<Box<dyn Trait + 'a>>` is precise but does not read at speaking pace.
vāṇī's goal is that a non-programmer domain expert — an avionics engineer,
a pharmacologist, a mathematician — can read a safety-critical function's
contracts without a tutorial on lifetime syntax. The semantic model stays the
same; the surface becomes the interface.

---

## Composition over inheritance

vāṇī has no class hierarchy. Polymorphism is achieved through **interfaces**
(`interface Drawable { fn draw(self: ref Self) -> i64; }`) implemented by any
type. Dynamic dispatch uses `dyn Iface` fat pointers when needed; static
dispatch (monomorphized generics `where T is Iface`) is the default.

This matches Rust's trait model. The reason it works: composition produces
smaller, testable units with explicit dependency surfaces. Inheritance creates
hidden coupling across a class tree that is hard to verify with SMT.

---

## `try` as value-flow, not control-flow

`try expr` propagates `Result.Err` upward from the current function — it is not
an exception. The error path is visible in the type signature (`-> Result<T, E>`),
trackable by the SMT layer, and has no hidden stack unwinding. This is identical
to Rust's `?` operator; `?` is accepted as an alias.

---

## Affine ownership vs garbage collection

GC-pause jitter is unacceptable in real-time and safety-critical systems. Affine
ownership gives deterministic drop order at zero runtime cost: when a binding
goes out of scope, its destructor runs immediately, in source order. No GC
thread, no pause, no unpredictable latency. The tradeoff is that the programmer
must reason about ownership — but the compiler enforces it, so the reasoning is
checked, not assumed.

---

## Why Rust for the compiler core?

The compiler is implemented in Rust. The reasons:

- **Memory safety without a GC.** A compiler that manages complex AST / IR
  graphs must not leak or corrupt its own data structures. Rust gives this
  at zero runtime cost.
- **Algebraic data types + pattern matching.** The IR / AST transformations
  are expressed as `match` over typed enums — exhaustiveness checking catches
  missed cases at compile time.
- **Deterministic performance.** Compilation latency is part of the developer
  experience. Rust's allocator and scheduler behavior is predictable.
- **No hidden dependencies.** The compiler is a single binary with no runtime
  interpreter or managed heap. This matches vāṇī's own design philosophy.

Python is fine for experiments, AI orchestration, and test scaffolding (the
`tools/` directory uses it extensively). But the core compiler is Rust — not
because Rust is fashionable, but because it is the right tool for a
safety-critical, performance-sensitive, long-lived codebase.

---

## Comparison with Rust

| Aspect | Rust | vāṇī |
|--------|------|------|
| Syntax | Punctuation-heavy | Keyword-first, natural-language aliases |
| Ownership | Lifetimes + borrow checker | Affine + second-class refs (no lifetime annotations) |
| Generics | Trait bounds + where clauses | `where T is Iface` |
| Error handling | `Result<T,E>` + `?` | `Result<T,E>` + `try` (or `?`) |
| Verification | None built-in | Z3 SMT: `requires` / `ensures` / `prove` |
| Safety attrs | None | `#[asil_d]`, `#[no_nan]`, `#[wcet]`, … |
| Multilingual | No | 62 dialects across 26 scripts via `// vani-lang:` |
| C backend | No | Yes (via `--backend=c` or `emit-c`) |

vāṇī is an independent project with no affiliation with the Rust project or
Rust Foundation.

---

## Comparison with C / C++

| Aspect | C / C++ | vāṇī |
|--------|---------|------|
| Memory safety | Manual / RAII (C++) | Affine ownership, compiler-enforced |
| Undefined behavior | Pervasive | Eliminated at the ownership + bounds level |
| Formal verification | External tools only | First-class `requires` / `ensures` / `prove` |
| Safety standards | MISRA C (external checker) | `#[misra_c_2012]` built into compiler |
| Readability | Punctuation-heavy | Keyword-first |

The C backend means vāṇī output can be integrated into any existing C build
system without LLVM on the target — a practical bridge for legacy embedded
codebases.

---

## Known design limitations

See [docs/v1_limitations.md](v1_limitations.md) for the full catalogue of known
v1 deviations with workarounds and fix-queue pointers.

Notably:
- SOV (verb-final) word order is partially wired: range `for` + 4 verb-at-end
  statement shapes. Full verb-final for `fn`, `struct`, `enum`, `match`
  declarations is a future arc.
- The non-Devanagari-Indo-Aryan dialect keyword tables have not been validated
  by native speakers. See [docs/languages.md](languages.md).
