# Glossary

A reference for terms used throughout these tutorials. Most are
standard PL / compiler / verification vocabulary; where vÄá¹‡à¥€
uses a term in a non-default way (or with a stronger guarantee
than usual), the entry calls that out.

If a tutorial chapter introduces a term you don't remember,
this page is the fastest cross-reference. Terms are grouped by
topic â€” same as the [README's glossary][readme-glossary], so
you can move between the two without re-orienting.

[readme-glossary]: https://github.com/enthusiasticgeek/vani-compiler/blob/main/README.md#glossary

## Ownership, references, and lifetimes

| Term | Meaning |
|---|---|
| **affine** | A value that can be used **at most once** (consume or drop, never both). vÄá¹‡à¥€'s affine ownership is what makes use-after-move, double-free, and double-close detectable at compile time. Stricter than C++ "move-only"; weaker than fully **linear** (which requires *exactly* once). |
| **linear** | Used *exactly* once â€” never dropped silently. vÄá¹‡à¥€ is affine, not linear: an unused value drops at scope exit. The contrast is mainly relevant in academic comparisons. |
| **ownership** | Which name in the program is responsible for releasing a value's resources. vÄá¹‡à¥€ has at most one owner per value at any moment; the owner's scope-exit triggers Drop. |
| **move** | Transferring ownership from one binding to another. Equivalent to "consume": the source binding can no longer be used. |
| **borrow** | Temporary read-only access via a `ref T` reference. The borrowed value's owner stays unchanged; the borrow may not outlive the owner. |
| **mut ref / mutable borrow** | Temporary read/write access via `mut ref T`. Exclusive while held â€” no other reference (shared or mut) can coexist. |
| **reborrow** | Constructing a fresh reference from an existing one (e.g. passing `ref x` further inwards). Inherits the parent's lifetime. |
| **alias / aliasing** | Two names that refer to the same underlying value. vÄá¹‡à¥€ forbids mutable aliasing â€” one `mut ref` rules out every other reference for its scope. |
| **escape (a reference escapes a scope)** | A reference outliving the local it borrows. The scope-escape analyzer rejects programs where a `ref` is returned, stored in a heap location, or assigned to a global. |
| **elision** | Compiler-inferred ("elided") lifetime â€” when the user doesn't write `<'a>`, the rules pick a sensible default. vÄá¹‡à¥€ uses Rust-style elision so most code is annotation-free. |
| **RAII** | "Resource Acquisition Is Initialization" â€” the resource's lifetime is tied to the scope of its owning binding. vÄá¹‡à¥€'s Drop runs at scope exit; no need for `defer` or finalizers. |

> Want a worked example? See *Affine ownership: `ref` / `mut ref`*
> ([`intermediate/03_affine.md`](intermediate/03_affine.md)) and
> *Ownership and move â€” intuition primer*
> ([`beginner/06c_ownership_primer.md`](beginner/06c_ownership_primer.md)).

## Type system

| Term | Meaning |
|---|---|
| **generic / parametric** | A definition that takes one or more type parameters (`fn id<T>(x: T) -> T`). vÄá¹‡à¥€ supports one type parameter per fn. |
| **monomorphization** | The pass that takes a generic definition (`fn id<T>`) and emits a specialized concrete copy per (template, type-args) tuple it sees called (`fn id__i64`, `fn id__bool`). The resulting program contains no generics at the IR level. |
| **monomorphic** | Already fully-specialized at concrete types â€” no remaining type parameters. |
| **mangling** | Rewriting a name to encode type-args / module path so the linker can keep distinct instantiations apart (`id__Box_I64_`). |
| **arity** | Number of arguments a function or variant takes. `fn add(a, b)` has arity 2. |
| **coercion** | Implicit type conversion the compiler inserts (e.g. `OwnedStr` â†’ `Str` in read-only positions). vÄá¹‡à¥€ keeps these conservative â€” every cross-width / cross-sign integer conversion requires an explicit `as`. |
| **nominal type** | Two types with the same shape are *not* the same type unless they share a name. vÄá¹‡à¥€'s structs are nominal â€” `struct A { x: i64 }` and `struct B { x: i64 }` aren't interchangeable. |
| **opaque type** | A name whose definition is hidden from callers â€” they can hold or pass the value but not read its fields. `Handle<T>` is opaque. |
| **trait / interface** | A named collection of methods that types can implement. vÄá¹‡à¥€ uses `interface`; Rust users will recognize it as the equivalent of `trait`. |
| **dyn iface** | A trait object â€” a value whose concrete type is erased and dispatched dynamically via a **vtable**. Written `dyn Iface` in vÄá¹‡à¥€. |
| **vtable** | A small table of function pointers (one per interface method) that powers dynamic dispatch on `dyn Iface`. |
| **fat pointer** | A pointer carrying its companion data inline â€” `Box<dyn Iface>` is `{ data_ptr, vtable_ptr }`; `BoundedPtr<T>` is `{ data, len, capacity }`. Contrast with a **thin pointer** (one word). |

> Worked examples: *Generics and monomorphization â€” intuition primer*
> ([`intermediate/04c_generics_primer.md`](intermediate/04c_generics_primer.md));
> *What's a `dyn Iface`?*
> ([`intermediate/04a_dyn_iface_primer.md`](intermediate/04a_dyn_iface_primer.md));
> *The `dyn` vtable layout + safety boundary*
> ([`advanced/05_vtables.md`](advanced/05_vtables.md)).

## Pattern matching & enums

| Term | Meaning |
|---|---|
| **scrutinee** | The expression *being matched* â€” the value on the right of `match â€¦ { â€¦ }`. In `match k { K.A then 1, K.B then 2 }`, `k` is the scrutinee. |
| **variant** | One alternative of an enum. `enum Opt { Some(i64), None }` has two variants. |
| **payload** | The data a variant carries. `Some(i64)` has an i64 payload; `None` has no payload. |
| **discriminant / tag** | The runtime integer that distinguishes which variant a value holds. |
| **destructure** | Pulling fields / payloads out of a compound value into named bindings â€” `let (a, b) = pair;` or `K.Some(v) then â€¦` in a match arm. |
| **exhaustive** | Every possible value of the scrutinee type matches at least one arm. vÄá¹‡à¥€ requires exhaustive matches and rejects gaps. |

> Worked examples: *Pattern matching â€” intuition primer*
> ([`beginner/08a_pattern_match_primer.md`](beginner/08a_pattern_match_primer.md));
> *Enums with payloads + match arms*
> ([`intermediate/02_enums_payloads.md`](intermediate/02_enums_payloads.md)).

## Compiler pipeline

| Term | Meaning |
|---|---|
| **AST** | Abstract Syntax Tree â€” the parser's structured representation of the source code. |
| **IR** | Intermediate Representation â€” a typed, post-checker form of the program. vÄá¹‡à¥€ emits a "tree IR" (close to the AST) and an "SSA IR" used by the optimizer / parallel-for emit. |
| **SSA** | Static Single Assignment â€” each variable is assigned exactly once; control flow merges become Phi nodes. Makes data-flow analyses straightforward. |
| **lowering** | Translating from a higher-level IR to a lower-level one (typed-IR â†’ SSA â†’ backend-specific code). |
| **emit / emission** | The final code-generation step that produces C or LLVM IR text. |
| **backend** | The half of the compiler that consumes the IR and produces output for a target. vÄá¹‡à¥€ has C and LLVM backends. |
| **lambda lift** | Hoisting an inline anonymous fn (`fn(x) -> y { â€¦ }`) to a top-level fn with a synthesized name so the backend can emit it as a regular symbol. |
| **(de)sugar** | "Sugar" is a convenient syntactic form; "desugar" is the parser- or pre-checker pass that rewrites it into the more verbose, semantically-equivalent core form (`?` becomes `try`, `+= 1` becomes `= â€¦ + 1`, etc.). |
| **hoist** | To move a sub-expression or statement *up* to an enclosing scope (e.g. binding a temporary `let` before using it inside a more restricted context). |
| **elaboration** | Adding hint / fix-it text to a diagnostic to help the user recover. |

> Worked example: *Compiler internals tour*
> ([`advanced/10_internals.md`](advanced/10_internals.md)).

## SMT verification

| Term | Meaning |
|---|---|
| **SMT** | Satisfiability Modulo Theories â€” a class of solvers (vÄá¹‡à¥€ uses Z3) that decide first-order formulas modulo integer/bitvector/float/etc. theories. Underpins `requires` / `ensures` / `prove` / `invariant`. |
| **BitVec** | A fixed-width bit-vector â€” how SMT models integer types. vÄá¹‡à¥€ encodes `i64` as `(_ BitVec 64)` so overflow is faithfully modeled (signed and unsigned semantics chosen per variable). |
| **discharge** | "Solve" a proof obligation â€” show the negation is unsatisfiable, i.e. the claim always holds under the in-scope assumptions. |
| **counterexample** | A concrete assignment of values to free variables under which a `prove` / `ensures` / `invariant` fails. Z3 emits one when the claim can't be discharged. |
| **invariant** (loop) | A claim that holds at every iteration: before entry, after every body, and at exit. The SMT pipeline checks each of those points. |
| **requires** / **ensures** | A function's pre-condition / post-condition. Callers must satisfy `requires` at every call site; the body must establish `ensures` on every return path. |
| **elide** | "Skip emitting" â€” the bounds-elision / overflow-elision passes turn off the runtime guard for an Index / arithmetic op once SMT discharges the safety obligation. |

> Worked examples: *First contract: `assert` / `prove` / `requires`*
> ([`beginner/09_smt_intro.md`](beginner/09_smt_intro.md));
> *SMT â€” `requires` / `ensures` intuition primer*
> ([`intermediate/12a_smt_primer.md`](intermediate/12a_smt_primer.md));
> *SMT verification deep-dive*
> ([`intermediate/12_smt_deepdive.md`](intermediate/12_smt_deepdive.md)).

## Async, concurrency, and effects

| Term | Meaning |
|---|---|
| **caller / callee** | The "caller" is the fn making the call; the "callee" is the fn being called. Used a lot in lifetime / ownership discussions. |
| **suspend point** | An `await(expr)` or `io_*_async(...)` call where the async runtime can pause the task and return control to the scheduler. State machines are split around these. |
| **poll** | The runtime entry point on an async task â€” called repeatedly until it returns `Ready(v)` or `Pending`. |
| **yield** | An async task voluntarily relinquishing control (`sleep_ms(0)` is the conventional shape). |
| **future** | An in-progress async computation. vÄá¹‡à¥€'s `Future<T>` is a state-machine enum with `Ready(T)` / `Pending` variants. |
| **pure fn** | A function with no observable side effects â€” no I/O, no heap allocation, no non-determinism, no impure callees. Verified by the effects checker. |
| **effects checker** | The pass that decides whether a function body is pure, by walking calls and rejecting any impure builtin or non-pure callee. Same logic gates `parallel for` bodies. |

> Worked examples: *Async, await, and Task â€” intuition primer*
> ([`advanced/01a_async_primer.md`](advanced/01a_async_primer.md));
> *`parallel for` + reductions + race-freedom*
> ([`advanced/02_parallel.md`](advanced/02_parallel.md));
> *`task` / `join` + atomics / mutexes / channels*
> ([`advanced/03_concurrency.md`](advanced/03_concurrency.md)).

## Memory & runtime primitives

| Term | Meaning |
|---|---|
| **arena** | A region allocator whose contents are all freed at once when the arena drops. vÄá¹‡à¥€'s `Region` is a bump-allocator arena available inside `unsafe(reason = "...")` on embedded targets. |
| **bump allocator** | Allocator that increments a single pointer and never reclaims individual cells â€” the arena reclaims everything at once. O(1) allocation. |
| **BoundedPtr<T>** | A fat pointer carrying `data + len + capacity` so `bptr_get(i)` can return `None` instead of UB on out-of-bounds. Available inside `unsafe(reason = "...")` for low-level interop. |
| **MMIO** | Memory-mapped I/O â€” reads and writes at hardware-defined addresses that map to device registers. `mmio_read_u32` / `mmio_write_u32` are the vÄá¹‡à¥€ builtins. |
| **canary** | A sentinel value placed adjacent to a buffer or stack frame so the runtime can detect overflow. Used in C as a stack-smashing defense; vÄá¹‡à¥€ doesn't need user-level canaries â€” bounds checks fire before overflow can occur. |
| **prelude** | A small set of declarations the compiler injects before every program: `Option<T>`, `Result<T,E>`, `Future<T>`, `Poll<T>`, `CancelToken`, `AllocError`. |
| **panic / abort** | A non-resumable termination. `assert` failures call `abort()`, which the OS reports as exit-on-signal SIGABRT. |
| **stack / heap** | The stack holds activation records (function-call frames); the heap holds long-lived allocations (`Vec<T>`, `OwnedStr`, `Box<T>`). vÄá¹‡à¥€ puts the choice in the type (Copy = stack-ish, owning = heap). |
| **deferred** | A diagnostic or work-item delayed until later in the pipeline. |
| **runtime** | Two senses: (a) the bundled C / LLVM helpers the compiler emits alongside user code (string concat, Vec helpers, futex-backed Mutex, etc.); (b) "at runtime" â€” when the compiled program executes. |
| **transitive** | Following a chain. "Transitive borrow" = a borrow of a borrow. "Transitive impurity" = if `f` calls `g` and `g` is impure, then `f` is impure. |

> Worked examples: *Heap and stack â€” intuition primer*
> ([`beginner/06b_heap_vs_stack_primer.md`](beginner/06b_heap_vs_stack_primer.md));
> *Embedded targets + `unsafe` + region typing*
> ([`advanced/04_embedded.md`](advanced/04_embedded.md));
> *Runtime errors, panic-free design, the segfault-free guarantee â€” intuition primer*
> ([`intermediate/10b_runtime_errors_primer.md`](intermediate/10b_runtime_errors_primer.md)).
