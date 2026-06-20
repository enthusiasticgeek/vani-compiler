# Missing language features and their vāṇी workarounds

> Audience: experienced systems programmers asking "does
> vāṇी have X?" and what to do when the honest answer is no.
> Updated 2026-06-20 (HashMap + Mutex entries corrected for v0.1.1/Arc-4 fixes).

vāṇी ships with a deliberately small feature surface — the
goal is everything that's there is fully verified and
composable, not everything that exists in Rust or C++.
This doc enumerates the genuinely-useful features that
**aren't** in v1, paired with the idiomatic vāṇी substitute
when one exists.

> **⚠️ Feature combinations matter as much as features.** A
> feature that works in isolation may still hit a gap when
> composed with another feature. The adversarial test suite
> ([`examples/edge_cases/`](../examples/edge_cases/)) caught
> one real compiler panic — `box(X as dyn Iface)` with an
> inline struct literal — and confirmed several deliberate
> combination restrictions. The **Mixed-feature gaps** section
> at the end of this doc enumerates the combination shapes
> that don't compose in v1 plus the workaround for each.

Sections:

1. [Generics + type system](#generics--type-system)
2. [Ownership + borrowing](#ownership--borrowing)
3. [Async + concurrency](#async--concurrency)
4. [Metaprogramming + reflection](#metaprogramming--reflection)
5. [Memory management](#memory-management)
6. [Pattern matching + control flow](#pattern-matching--control-flow)
7. [Numerics + types](#numerics--types)
8. [Modules + packages](#modules--packages)

---

## Generics + type system

### Generic trait bounds (`T: Hash + Eq`)

**What Rust has:** `fn lookup<T: Hash + Eq>(...)` — the
compiler enforces that `T` implements the listed traits at
each instantiation.

**vāṇी today:** monomorphized generics work (`fn id<T>(x:
T) -> T`), but there's **no syntactic bound expression**.
The compiler can require an iface impl indirectly: HashMap
keys require `Hash for K` via `iface_impl_exists` at use
sites; other generic call sites trust the user.

**Workaround:** for the common case (HashMap-key-style
constraints) the compiler does enforce — just call the iface
method in the body and the implicit constraint surfaces at
instantiation. For more complex bounds, use **`dyn Iface`
parameter types** instead of generic bounds:

```rust
// Rust:  fn process<T: Drawable>(t: T) -> i64
// vāṇी:  fn process(t: dyn Drawable) -> i64
```

The vāṇी version pays one indirect call per method; the
Rust version monomorphizes. Trade.

### Higher-rank polymorphism (`for<'a>`, `Box<dyn for<T> Fn(T)>`)

**Not in vāṇी.** No one's reached for it; v1 closures use
fixed argument types in their `Closure<T1, T2> -> R` shape.

### Generic associated types (Rust's GATs)

**Not in vāṇी.** Niche even in Rust. No workaround needed —
the patterns that use GATs (streaming iterators with
lifetime-parameterized items) don't compose with vāṇी's
single-param lifetime elision anyway.

### Const generics

**Partial.** `[T; N]` works with a literal `N`. You can't
write `fn pad<const N: usize>(xs: [i64; N]) -> [i64; N+1]` —
the `N` in a signature must be a literal at the call site.

**Workaround:** use `Vec<T>` when the size is dynamic.
For genuinely-static N, write per-shape fns and call the
right one.

### Generic functions calling other generic functions (nested monomorphization)

**vāṇī today:** the monomorphizer is **single-pass** — it walks only the
non-generic call sites in the original source, collects the
`(fn, concrete-type)` pairs, and generates specializations.  If a
*specialized* body itself calls another generic function, that inner
generic call is **not collected**: the inner function is never
specialized, its generic template is then removed from the program,
and the call becomes a dangling reference that produces a compile error.

```rust
// Works:
fn wrap<T>(x: T) -> T { return x; }
fn main() -> i64 { return wrap(42); }  // direct generic call → OK

// Fails (single-pass gap):
fn wrap<T>(x: T) -> T { return x; }
fn double_wrap<T>(x: T) -> T { return wrap(x); }  // generic calling generic
fn main() -> i64 { return double_wrap(42); }
// Error: wrap is never called from a non-generic site, so wrap__i64
// is never generated.  double_wrap__i64's body calls the removed template.
```

**Workaround:** flatten the call chain so every generic function is
called **directly from a non-generic function**:

```rust
fn wrap<T>(x: T) -> T { return x; }
fn double_wrap(x: i64) -> i64 { return wrap(x); }  // non-generic wrapper
fn main() -> i64 { return double_wrap(42); }
```

Or inline the inner generic call at each specialized call site.

**When it will be fixed:** when the monomorphizer becomes multi-pass
(iterates over newly-generated specializations until the `needed` set
is stable).  Tracked via the `nested_generic_call_pins_current_behavior`
regression test in `src/lib.rs`.

### Type-state via phantom types

**Not in vāṇी.** No `PhantomData<T>`; structs can only carry
fields whose types are part of the runtime representation.

**Workaround:** enum-tag-based state machines. A `Connection`
type with `enum Status { Open, Closed }` carries the same
type-state information at runtime; the compiler's
exhaustiveness check on match arms gives you compile-time
verification at every branch.

---

## Ownership + borrowing

### Reference counting (`Rc<T>` / `Arc<T>` / `Weak<T>`)

**Deliberately not in vāṇी.** See
[Intermediate 3c shared-ownership-without-Rc](../tutorials/src/intermediate/03c_shared_ownership_primer.md)
and
[Intermediate 3d cyclic-references](../tutorials/src/intermediate/03d_cyclic_references_primer.md)
for the full story.

**Workaround (five patterns):** just-borrow, indices-into-Vec,
arena allocation, channels, Mutex<T>. Covers ~95% of Rc use
cases; the remaining 5% (third-party plugins, multi-modal
DOM-like graphs) reach for `unsafe(reason = "...")`.

### Lifetime variables (`'a`, `'b`)

**Not in vāṇी (path-D, deferred indefinitely).** v1 has
single-param lifetime elision for ref returns (path-C); see
[Intermediate 3e lifetimes](../tutorials/src/intermediate/03e_lifetimes_primer.md).
The rejected cases are multi-input distinct lifetimes,
lifetime-parameterized struct definitions, and ref-capturing
closures.

**Workaround:**
- Multi-input distinct lifetimes → split into two narrower fns.
- Lifetime-parameterized struct → store `Box<T>` or
  indices into the owner.
- Ref-capturing closures → restructure to pass refs as
  closure-fn args, not captures.

### Custom `Drop` impl

**Partial.** `iface Drop` works; the compiler honors
`fn drop(self: mut ref Self) -> i64` at scope-exit for the
single-impl case. **Multiple Drop impls on the same type**
aren't supported (the impl collector takes the first).

**Workaround:** if you need cleanup beyond the default
field-by-field drop, write one Drop impl that does
everything.

### Drop ordering control

**Not in vāṇी.** Drop fires in reverse-declaration order
within a block. No `ManuallyDrop<T>` to opt one binding out
of the auto-drop pass.

**Workaround:** explicit consumption order. `let _ = take(a);
let _ = take(b);` ensures `a` is consumed before `b`.

### Custom allocator

**Not in vāṇी.** All allocs go through libc `malloc`/`free`.

**Workaround:** for embedded targets, `region { ... }` blocks
allocate from a stack buffer (Layer 5 of unsafe.md; planned
for v2). For now, custom allocation is `unsafe(reason =
"...")` with manual `*mut T` arithmetic.

---

## Async + concurrency

### `async fn` returning an opaque `impl Future`

**Not in vāṇी.** Async fns lower to a synthesized `Task__X`
struct + `__poll_X` function (Arc 8 v3.1); the type is
explicitly named, not opaque. Callers see `Task__X` and
must call `__poll_X` directly (or use `await` which desugars
to it).

**Workaround:** the named `Task__X` IS the future — just
treat the name as the return type.

### `async fn` with generic return types

**Fully shipped** (Arc 8 v3.1 Phase 4c-broad — `async fn
identity<T>(...) -> T` compiles and runs end-to-end on both
backends). See [ARC8_V3_PLAN.md](../ARC8_V3_PLAN.md).

### `Stream<T>` (async iterator)

**Not in vāṇी.** No `Stream` trait; no `for await` loop.

**Workaround:** hand-roll a poll loop:

```rust
async fn next_event() -> Option<Event> { ... }

async fn process_stream() -> i64 {
  loop {
    match await next_event() {
      Some(e) then { process(e); },
      None then { return 0; },
    }
  }
}
```

### `select!` over multiple futures

**Not in vāṇी.** Cannot wait on "whichever future completes
first."

**Workaround:** explicit polling. Call each future's poll fn
in a round-robin loop and check for `Ready`:

```rust
let task_a: Task__A = a();
let task_b: Task__B = b();
loop {
  if let Ready(v) = __poll_a(mut ref task_a) { return v; }
  if let Ready(v) = __poll_b(mut ref task_b) { return v; }
}
```

Less ergonomic than `select!` but explicit.

### Async with `Pin<&mut Self>` self-references

**Explicitly not in vāṇी** (🛑 NON-COMPLIANT under affine
ownership). The state machine vāṇी synthesizes is plain
struct-with-fields; no self-references in the state struct.

**Workaround:** restructure the async fn so locals don't
need to alias across `await` points.

### Threads with shared mutable state (no Mutex)

**Not in vāṇी.** Cross-thread state goes through `Atomic<T>`
(lock-free seq-cst) or `Mutex<T>` + `Guard<T>` (RAII unlock).
Sharing a raw `Vec<i64>` between threads is rejected.

**Workaround:** `Atomic<T>` for single-value counters /
flags. `Mutex<Vec<i64>>` **now works** (v0.1.1 made `Mutex<T>`
parametric over any element type). For `Atomic<T>`, the
payload is still i64-width–shaped only; use `Mutex<T>` for
collection payloads.

---

## Metaprogramming + reflection

### Procedural macros

**Not in vāṇी.** No source-level code generation.

**Workaround:** code-generate at build time with an external
script. The `tools/llm_context/bundle.py` script is itself
an example — drives `vani_translate.py` ALIASES, emits a
bundle from repo data.

### Declarative macros (`macro_rules!`)

**Not in vāṇी.** No `macro_rules!` equivalent.

**Workaround:** generic functions cover most "abstract over
type" cases; for repeated boilerplate (e.g. one Drop impl
per dozen structs), a build-time codegen script.

### Reflection / runtime type introspection

**Not in vāṇी.** No `TypeId`, no `Any`, no field-by-name
runtime access.

**Workaround:** `dyn Iface` for runtime polymorphism;
explicit enum-tag dispatch when you'd reach for type-based
dispatch in another language.

### Attribute macros (`#[derive(Debug)]`)

**Not in vāṇी.** No `derive(Debug)` / `derive(Clone)` etc.
Print formatting for structs requires a `print` builtin
extension or hand-written field-by-field code.

**Workaround:** write the print code yourself, or use
`vanic emit --backend=c` to read the C representation when
debugging.

---

## Memory management

### Custom allocator per-type

**Not in vāṇी.** Every allocation goes through global libc
`malloc`/`free`.

**Workaround:** `region { ... }` blocks (v2 / Layer 5) for
embedded; `Vec<T>` re-use (drain + clear + push) within a
single owner.

### `Box::leak` / intentionally leaked allocations

**Not in vāṇी.** Affine ownership requires every
heap-owning value to drop on scope exit.

**Workaround:** if you genuinely need a lifetime-of-the-
program allocation, declare it in `main` and pass refs
everywhere. The "leak" becomes "long-lived owner."

### Memory layout control (`#[repr(C)]`, `#[repr(packed)]`)

**Not in vāṇी.** Struct layout is compiler-chosen (field
order in source = field order in memory; padding follows the
backend's ABI). No layout pragmas.

**Workaround:** for FFI with C, define the struct in vāṇी
in the same field order as the C declaration. For bit-packed
representations (network protocols, hardware registers), use
explicit `u32` / `u64` bit-manipulation helpers
(`i64_set_bit`, `i64_test_bit`, etc.) rather than struct
fields.

---

## Pattern matching + control flow

### Or-patterns (`Some(1) | Some(2) then ...`)

**Not in vāṇी.** Each variant gets its own arm.

**Workaround:** duplicate the body, or extract into a helper
fn.

### Pattern guards (`Some(n) then n > 0 ...`)

**Not in vāṇी.** Match arms are pattern-only; conditions go
inside the arm body.

**Workaround:** match on the variant, then if/else in the
arm body.

### Slice patterns (`[first, .., last]`)

**Not in vāṇी.** Vec/array indexing is positional.

**Workaround:** explicit indexing — `xs[0]` and `xs[len-1]`
with the appropriate `requires len(xs) >= 2`.

### `if let` / `while let`

**Not in vāṇी.** Pattern binding is `match`-only.

**Workaround:** use `match` with one named arm + a wildcard:

```rust
match opt {
  Some(v) then { use(v); }
  None then {},
}
```

### Labeled break / continue

**Not in vāṇी.** `break` / `continue` only target the
nearest enclosing loop.

**Workaround:** extract the inner loop into a helper fn that
returns early.

### Try-block / `try { ... }`

**Not in vāṇी.** No try-block construct (the `try EXPR`
keyword + postfix `?` cover the propagation cases).

**Workaround:** wrap the fallible operations in a helper fn
that returns `Result<T, E>` and `try` / `?` propagates.

---

## Numerics + types

### Bigint / arbitrary-precision

**Not in vāṇी.** Integers are `i64` / `u64` / smaller.
Constant-time overflow is caught at compile time; runtime
arithmetic silently wraps (two's complement) — no guards
are emitted at arithmetic op sites. See the **Integer
overflow runtime guards** row in Mixed-feature gaps below.

**Workaround:** for cryptographic / arbitrary-precision
work, use C bigint via FFI (`extern "C"` declarations to
GMP / mbed-crypto).

### Float bit-twiddling without union

**Partial.** `f64_to_bits(x: f64) -> u64` + `f64_from_bits`
shipped; works for the common cases.

### `Decimal` / fixed-point

**Not in vāṇी.** Use scaled integers (multiply by 100 for
2-decimal money math).

### SIMD intrinsics

**Not in vāṇी.** No `__m128i` / `__m256i` equivalent.

**Workaround:** `parallel for` lowers to OpenMP (C backend)
which the C compiler may auto-vectorize. For explicit SIMD,
`extern "C"` to a C helper compiled with intrinsics.

---

## Modules + packages

### Re-export with rename

**Partial.** `pub use foo::bar;` works; `pub use foo::bar
as baz;` is not yet supported.

**Workaround:** define a wrapper fn `pub fn baz(...) -> ...
{ return bar(...); }`.

### Visibility modifiers beyond `pub` / `pub(kosh)`

**Limited.** v1 has `pub`, `pub(kosh)`, and module-private
(default). Rust's `pub(crate)`, `pub(super)`, `pub(in path)`
aren't there.

**Workaround:** the existing `pub(kosh)` covers
"package-visible"; module-private covers everything else.

### Workspace / multi-crate package

**Not in vāṇी.** Each `vani.toml` is a single crate; no
workspace concept.

**Workaround:** monorepo with one manifest, or build each
"crate" separately and link via `--link-with`. The Kosh
package manager (queued, pending registry-hosting
decision) will lift this.

### Built-in test runner (`#[test]`)

**Not in vāṇी.** Tests are written in the host language
(Rust, for the compiler itself); vāṇी programs invoke
external test runners or assert-based smoke tests.

**Workaround:** assert-based smoke tests in `examples/`
serve as the de-facto test runner. The cross-backend parity
test (`run_end_to_end.rs`) runs every example through both
C and LLVM.

---

## Summary table

| Feature | Status | Workaround |
|---|---|---|
| Generic trait bounds | indirect | iface dispatch as `dyn Iface` param |
| Generic fn calling generic fn (nested mono) | single-pass gap | flatten: make the inner generic call from a non-generic wrapper |
| Higher-rank polymorphism | absent | not needed in practice |
| GATs | absent | not needed in practice |
| Const generics | partial | `Vec<T>` for dynamic; per-shape fns for static |
| Phantom types | absent | enum-tag state machines |
| `Rc<T>` / `Arc<T>` / `Weak<T>` | by design | 5-pattern alternatives (see 03c primer) |
| Multi-input distinct lifetimes | path-D | split into narrower fns |
| Lifetime-parameterized structs | path-D | Box / indices |
| Custom allocator | embedded only | global malloc/free; regions in v2 |
| Drop ordering | reverse-decl only | explicit consumption order |
| `async` with `impl Future` | named Task__X | use the synthesized name |
| `Stream<T>` async iterator | absent | hand-rolled poll loop |
| `select!` | absent | explicit poll round-robin |
| Pin<&mut Self> | NOT planned | restructure to avoid self-refs |
| Mutex<Vec<T>> | i64 only in v1 | channel transfer / task ownership |
| Proc macros | absent | external build-time codegen script |
| `macro_rules!` | absent | generic fns + per-call codegen |
| Reflection | absent | `dyn Iface` / explicit enum dispatch |
| Custom layout (`#[repr(C)]`) | absent | match field order to C declaration |
| Or-patterns | absent | duplicate arms or extract helper fn |
| Pattern guards | absent | match → if/else in body |
| Slice patterns | absent | positional indexing + `requires` |
| `if let` / `while let` | absent | `match` with named + wildcard arm |
| Labeled break/continue | absent | extract inner loop into helper |
| Bigint | absent | C FFI (GMP) |
| Runtime integer overflow guards | absent (wraps) | `requires` on operand bounds; const-time overflow IS caught |
| Decimal / fixed-point | absent | scaled integers |
| SIMD intrinsics | absent | parallel for + auto-vectorize / FFI |
| `pub use foo::bar as baz` | partial | wrapper fn |
| Workspaces | absent | monorepo or `--link-with` |
| `#[test]` runner | absent | `examples/` + cross-backend parity test |

The deliberate omissions (Rc, multi-lifetime, Pin) are
philosophical — vāṇी picks structural prevention over
flexibility. The accidental omissions (or-patterns, slice
patterns, `if let`, `pub use ... as ...`, workspaces) are
v1 gaps; some will land in follow-up releases once a real
use case surfaces.

---

## Mixed-feature gaps

Combinations of v1-supported features that don't compose
today. Each row lists the shape, the reason it rejects, and
the v1 workaround. The adversarial test set under
[`examples/edge_cases/`](../examples/edge_cases/) keeps
checking these as the compiler evolves.

| Combination | Why it rejects | Workaround |
|---|---|---|
| `(Box<T>, U)` tuple element | v1 tuples are Copy-only; Box<T> is affine. The checker rejects with "tuple element N has non-Copy type X — v1 tuples are Copy-only". | Wrap the pair in a named struct: `struct Pair { box: Box<T>, other: U }`. Structs accept non-Copy fields. |
| `(OwnedStr, U)` tuple element | Same reason — OwnedStr is non-Copy. | Same — named struct. |
| `Option<Box<T>>` enum payload | v1 enum variants admit Copy / OwnedStr / Vec / array / Task / Atomic / Mutex / Channel payloads; Box<T> isn't in that list. | (a) Wrap Box<T> in a struct that has Vec<Box<T>> of length 0 or 1; (b) use a tag + separate Box<T> field with a default-init convention. |
| `Result<Box<T>, E>` | Same — enum-payload restriction. | Same workarounds. |
| `Vec<(i64, OwnedStr)>` etc. with non-Copy tuple element | The tuple's non-Copy restriction propagates through Vec. | Wrap the tuple in a named struct (the struct can be a Vec element). |
| `HashMap<K, V>` with non-scalar V | ✅ **Fixed (Arc 4)** — `hashmap_insert` now accepts `OwnedStr`, `Vec<i64>`, tuple, `f64`, and `Vec`-typed values. Full K-V matrix shipped. | No workaround needed. |
| `Mutex<Vec<T>>`, `Atomic<Vec<T>>` | `Mutex<T>` is now parametric over any T (v0.1.1) — `Mutex<Vec<i64>>` works. `Atomic<T>` payload is still i64-width–shaped only. | For Atomic, use a `Mutex<Vec<T>>` instead; or channel-transfer ownership. |
| Closure capturing non-Copy binding | Closures + tasks reject affine captures (rejected with `closure_captures_affine` elaboration). | Pre-extract scalar fields from the affine value, or pass it as a closure-fn argument rather than capturing. |
| `box(X { ... } as dyn Iface)` inline | **Was a compiler panic; fixed 2026-06-09.** Now works on both backends. | Pinned by a lib regression test; previously the workaround was `let v = X { ... }; box(v as dyn Iface);` — the let-bind form was always safe. |
| `enum Outer { Wrap(Inner) }` where Inner is also an enum | **Was a rejection ("payload must be assignable to Inner, got Inner"); fixed 2026-06-09.** | Pinned by `nested_enum_payload_accepts_enum_construction`; the parser-stamped `Type::Struct(Inner)` is now resolved to `Type::Enum(Inner)` for enum variant payloads (the resolve pass missed them before). |
| Inline closure inside `implement Iface for T { fn m { ... } }` | **Was a compiler panic ("anonymous fn survived lambda-lift"); fixed 2026-06-09.** | The lambda-lift pass now walks impl + methods-block bodies BEFORE they're hoisted. Pinned by two regression tests. |
| Anonymous fn called inline from a Vec slot (`fs[0](10)`) | Rejected — "only named functions can be called". | Assign to a `let` first, then call. |
| Generic fn `<T>` inferred from complex argument types (e.g. `Vec<T>` with Vec-typed arg) | Rejected — v1 inference supports literal / Var / (v3.1 only) Ref(Var) at the T-position. | Pre-extract the arg into a Var, or restructure the fn to not be generic over composite types. |
| Turbofish syntax (`f::<i64>(arg)`) | Not supported in v1. | Rely on inference (which is limited; see above). |
| OwnedStr payload bound in match arm returned as OwnedStr | Match-arm binding exposes the payload as `Str` (read-only view), not the owned form. Returning the binding produces a Str-typed arm body. | Wrap the bound arm as `s + ""` to clone into OwnedStr, AND use `"" + ""` for all other literal-string arms so every arm produces OwnedStr (arm types must agree). |
| **Integer overflow runtime guards** | **NOT emitted in v1** despite the README's aspirational claim. `i64::MAX + 1` silently wraps to `i64::MIN` on both backends. This is a real safety gap — the elision pass aspires to keep guards by default and elide via SMT discharge, but the guards aren't actually generated at the arithmetic op sites today. | Reach for `requires` clauses to constrain operand ranges; the SMT pass at least flags overflows when both bounds are known. Treat unbounded i64 arithmetic as wrapping for now. |
| Two refs to the same Vec at once (read + write) | Aliasing rule: many shared XOR one mut. | End one borrow before taking the other; or split the operation into two passes. |
| Generic fn with multiple lifetime-distinct ref params returning a ref | Path-D territory; v1 only does single-ref-param elision. | Split into two narrower fns, each with one ref param. |
| `async fn` containing a `dyn Iface` method call across an `await` | `dyn`-method receivers can't be held across suspend points (Pin-like restriction would be needed). | Resolve the dyn before the await; pre-compute, then await, then use the result. |
| Recursive `Drop` impl that calls another `Drop` impl on a borrowed field | Borrow-checker rule: `mut ref Self` during drop can't pass to another `Drop` taking `mut ref Self`. | Implement Drop only at the outermost level; let the compiler chain field-by-field drops automatically. |

The pattern: when two features both reach for "non-Copy
data", the combination often hits a v1 gap because the
checker hasn't been lifted to handle that interaction yet.
Single-feature use is well-tested; combination depth is
where the bugs live.

### How to find more

The adversarial test set in
[`examples/edge_cases/`](../examples/edge_cases/) is the
canonical place to add new combination tests. The pattern:

1. Write a `.vani` file that combines 2-3 features in a
   way you'd plausibly want.
2. Run `vanic check <file>` and `vanic run <file>
   --backend=c` + `--backend=llvm`.
3. Three outcomes:
   - **Clean run** → add the file as a "should-pass" row
     in the README.
   - **Clear rejection diagnostic** → add a "should-reject"
     row + the workaround for documentation.
   - **Compiler panic / wrong output / silent
     miscompile** → file a bug, write a regression test,
     fix.

Mixed-feature shapes worth probing (none observed broken
in the current set, but watch for):
- Box<T> through generics (`fn foo<T>(b: Box<T>) -> Box<T>`)
- async fn returning a Vec<Box<dyn Iface>>
- `parallel for` over a Vec of structs with OwnedStr fields
- HashMap key = struct with Hash impl
- Match-with-bindings on a deeply nested enum payload
- Custom Drop impl with `mut ref` to a Vec field

The honest list: vāṇी covers the **structural** ergonomics
of Rust (ownership, types, async) and a **subset** of the
syntactic ergonomics. If you want maximally-concise
expression of every CS pattern, Rust is the answer. If you
want a smaller, fully-verified surface that's safe by
construction on the hosted target, that's what vāṇी ships.
