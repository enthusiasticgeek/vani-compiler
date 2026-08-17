# Missing language features and their vāṇी workarounds

> Audience: experienced systems programmers asking "does
> vāṇī have X?" and what to do when the honest answer is no.
> Updated 2026-07-17 — marked 11 features shipped since 2026-06-20.

vāṇī ships with a deliberately small feature surface — the
goal is everything that's there is fully verified and
composable, not everything that exists in Rust or C++.
This doc enumerates the genuinely-useful features that
**aren't** in v1, paired with the idiomatic vāṇī substitute
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

**vāṇī today:** monomorphized generics work (`fn id<T>(x:
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

The vāṇī version pays one indirect call per method; the
Rust version monomorphizes. Trade.

### Higher-rank polymorphism (`for<'a>`, `Box<dyn for<T> Fn(T)>`)

**Not in vāṇī.** No one's reached for it; v1 closures use
fixed argument types in their `Closure<T1, T2> -> R` shape.

### Generic associated types (Rust's GATs)

**Not in vāṇī.** Niche even in Rust. No workaround needed —
the patterns that use GATs (streaming iterators with
lifetime-parameterized items) don't compose with vāṇī's
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

✅ **Shipped (XL4, 2026-07-15)** — see commit history. The monomorphizer is now
worklist-based multi-pass: each new specialization is scanned for further generic
calls until the needed-set is stable. Two-level and three-level chains now compile.

### Type-state via phantom types

**Not in vāṇī.** No `PhantomData<T>`; structs can only carry
fields whose types are part of the runtime representation.

**Workaround:** enum-tag-based state machines. A `Connection`
type with `enum Status { Open, Closed }` carries the same
type-state information at runtime; the compiler's
exhaustiveness check on match arms gives you compile-time
verification at every branch.

---

## Ownership + borrowing

### Reference counting (`Rc<T>` / `Arc<T>` / `Weak<T>`)

**Deliberately not in vāṇī.** See
[Intermediate 3c shared-ownership-without-Rc](../tutorials/src/intermediate/03c_shared_ownership_primer.md)
and
[Intermediate 3d cyclic-references](../tutorials/src/intermediate/03d_cyclic_references_primer.md)
for the full story.

**Workaround (five patterns):** just-borrow, indices-into-Vec,
arena allocation, channels, Mutex<T>. Covers ~95% of Rc use
cases; the remaining 5% (third-party plugins, multi-modal
DOM-like graphs) reach for `unsafe(reason = "...")`.

### Recursive / self-referential types (tree/list nodes via `Box<Self>`)

**Not in vāṇī.** A type can't contain a `Box` of itself, directly or
through a wrapper struct — the classic Rust
`enum Expr { Num(i64), Add(Box<Expr>, Box<Expr>) }` shape doesn't
compile. Confirmed by direct test (2026-07-24, while scoping
`vani-symbolic`'s expression-tree representation): two independent
blockers stack here, either one enough to reject it. (1) Enum
variants only admit a **single** payload field in v1, so
`Add(Box<Expr>, Box<Expr>)` is rejected outright ("only single-field
payloads supported in v1"); wrapping the two children in a carrier
struct doesn't help, because (2) `box()`'s admitted payload types are
Copy scalars/structs, `dyn Iface`, `Vec<T>`, and `OwnedStr` only — a
struct or enum that (transitively) contains the type being defined is
never one of those, so `box(child_of_type_being_defined)` is rejected
too ("got `Expr`... other owning inner types remain a follow-up").
`Box<T>` itself is fully shipped (see L2 in
[`v1_limitations.md`](v1_limitations.md)) — what's missing is
specifically the self-referential case.

**Workaround: arena representation.** Store every node in a flat
`Vec<Node>` and reference children by `i64` index (`-1` for "none")
instead of by pointer — the same "flat `Vec` + explicit index
convention" idiom already used throughout the
[Kosh math ecosystem](https://github.com/enthusiasticgeek/kosh-index)
(`vani-tensor`'s shape encoding, `vani-discrete`'s adjacency matrix,
`vani-sparse`'s COO/CSR). Confirmed working end-to-end (both backends,
`vanic check`) with a `struct Node { kind: i64, value: i64, left: i64,
right: i64 }` arena and a recursive `eval(arena: ref Vec<Node>, idx:
i64) -> i64` walker. This sidesteps the limitation entirely (no
recursive type is ever declared — `Node` is a plain Copy struct) and,
as a side benefit, keeps tree operations expressible as ordinary
`Vec`-indexed recursion or iteration rather than pointer-chasing,
which plays better with the safety-attribute system's existing
"unbounded recursion is `#[wcet]`-exempt but must still terminate"
model.

### Lifetime variables (`'a`, `'b`)

**Not in vāṇī (path-D, deferred indefinitely).** v1 has
single-param lifetime elision for ref returns (path-C); see
[Intermediate 3e lifetimes](../tutorials/src/intermediate/03e_lifetimes_primer.md).
The rejected cases are multi-input distinct lifetimes and
lifetime-parameterized struct definitions.

**Ref-capturing closures — correction (2026-07-25):** this used to be
listed alongside the other path-D-rejected cases, but that's not quite
right. The `[ref name]` explicit capture-list syntax (Arc 5c / ARC 3a)
already works today for a closure called directly by name within the
same scope, including non-Copy captures like `Vec<f64>` — confirmed by
direct test. What doesn't work is using such a closure as a genuine
first-class value (returning it, storing it, passing it to a
higher-order function) — `checker.rs`'s closure-lift pass explicitly
skips the Closure-value path when ref-captures are present (see
`docs/ref_capturing_closures_design.md` for the full scoping writeup,
including a real soundness bug (BUG-7) found in the scope-escape
analyzer along the way). That's a narrower, smaller gap than "no lifetime
variables at all," and is scoped as a separate, not-yet-started proposal.

**Workaround:**
- Multi-input distinct lifetimes → split into two narrower fns.
- Lifetime-parameterized struct → store `Box<T>` or
  indices into the owner.
- Ref-capturing closures used as a value (returned/stored/passed to a
  higher-order fn) → restructure to pass refs as plain fn args instead of
  captures (a non-capturing named function works fine as a `Closure`/`fn`
  value today). Calling a `[ref name]` closure directly by name in the
  same scope already works and needs no workaround.

### Custom `Drop` impl

**Partial.** `iface Drop` works; the compiler honors
`fn drop(self: mut ref Self) -> i64` at scope-exit for the
single-impl case. **Multiple Drop impls on the same type**
aren't supported (the impl collector takes the first).

**Workaround:** if you need cleanup beyond the default
field-by-field drop, write one Drop impl that does
everything.

### Drop ordering control

**Not in vāṇī.** Drop fires in reverse-declaration order
within a block. No `ManuallyDrop<T>` to opt one binding out
of the auto-drop pass.

**Workaround:** explicit consumption order. `let _ = take(a);
let _ = take(b);` ensures `a` is consumed before `b`.

### Custom allocator

**Not in vāṇī.** All allocs go through libc `malloc`/`free`.

**Workaround:** for embedded targets, `region { ... }` blocks
allocate from a stack buffer (Layer 5 of unsafe.md; planned
for v2). For now, custom allocation is `unsafe(reason =
"...")` with manual `*mut T` arithmetic.

---

## Async + concurrency

### `async fn` returning an opaque `impl Future`

**Not in vāṇī.** Async fns lower to a synthesized `Task__X`
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

**Not in vāṇī.** No `Stream` trait; no `for await` loop.

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

✅ **Shipped 2026-07-16 (L3).** `select { await <poll_call> then <binding> { body } … }`
desugars to a `while true` loop with one `if poll_rN != -2` arm per branch; first
ready arm executes and breaks.

### Async with `Pin<&mut Self>` self-references

**Explicitly not in vāṇī** (🛑 NON-COMPLIANT under affine
ownership). The state machine vāṇī synthesizes is plain
struct-with-fields; no self-references in the state struct.

**Workaround:** restructure the async fn so locals don't
need to alias across `await` points.

### Threads with shared mutable state (no Mutex)

**Not in vāṇī.** Cross-thread state goes through `Atomic<T>`
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

**Not in vāṇī.** No source-level code generation.

**Workaround:** code-generate at build time with an external
script. The `tools/llm_context/bundle.py` script is itself
an example — drives `vani_translate.py` ALIASES, emits a
bundle from repo data.

### Declarative macros (`macro_rules!`)

**Not in vāṇī.** No `macro_rules!` equivalent.

**Workaround:** generic functions cover most "abstract over
type" cases; for repeated boilerplate (e.g. one Drop impl
per dozen structs), a build-time codegen script.

### Reflection / runtime type introspection

**Not in vāṇī.** No `TypeId`, no `Any`, no field-by-name
runtime access.

**Workaround:** `dyn Iface` for runtime polymorphism;
explicit enum-tag dispatch when you'd reach for type-based
dispatch in another language.

### Attribute macros (`#[derive(Debug)]`)

**Not in vāṇī.** No `derive(Debug)` / `derive(Clone)` / `derive(Eq)`
etc. `Eq` (like every interface) is always opt-in and hand-written --
`implement Eq for T { fn eq(self: ref T, other: ref T) -> bool { ... } }`
for non-Copy structs, or `self: T, other: T` by value for Copy ones
(see [`struct_eq.vani`](../examples/language/english/struct_eq.vani)
and the [interfaces primer](../tutorials/src/intermediate/04b_interfaces_primer.md#no-derive----eq-and-other-traits-are-always-hand-written)).
Print formatting for structs similarly requires a `print` builtin
extension or hand-written field-by-field code.

**Is this a real gap?** Not a capability one -- everything derive
would generate is still directly expressible by hand, and it matches
vāṇī's explicit-over-implicit posture (interface conformance is
always an opt-in `implement` block, never inferred from shape). The
real cost is boilerplate that scales with struct count × trait count
-- as the [kosh ecosystem](https://github.com/enthusiasticgeek/kosh-index/blob/main/ROADMAP.md)
grows, every struct needing `Eq` gets a few hand-written lines instead
of one attribute. `vani-bignum`'s `BigInt` is the reference example of
the pattern in a real published package. Revisit only if boilerplate
volume becomes a measured pain point across packages, not preemptively.

**Workaround:** write the `implement Eq for T` / print code yourself,
or use `vanic emit --backend=c` to read the C representation when
debugging.

---

## Memory management

### Custom allocator per-type

**Not in vāṇī.** Every allocation goes through global libc
`malloc`/`free`.

**Workaround:** `region { ... }` blocks (v2 / Layer 5) for
embedded; `Vec<T>` re-use (drain + clear + push) within a
single owner.

### `Box::leak` / intentionally leaked allocations

**Not in vāṇī.** Affine ownership requires every
heap-owning value to drop on scope exit.

**Workaround:** if you genuinely need a lifetime-of-the-
program allocation, declare it in `main` and pass refs
everywhere. The "leak" becomes "long-lived owner."

### Memory layout control (`#[repr(C)]`, `#[repr(packed)]`)

✅ **Shipped 2026-07-16 (L2).** `#[repr(C)]` and `#[repr(packed)]` at struct
declaration sites. C backend emits `__attribute__((packed))` for packed; LLVM
backend emits `<{ ... }>` packed-struct syntax.

---

## Pattern matching + control flow

### Or-patterns (`Some(1) | Some(2) then ...`)

✅ **Shipped 2026-07-15 (M2).** `|`-separated patterns in a single arm; expands
to synthetic arms sharing the same body before type-checking.

### Pattern guards (`Some(n) then n > 0 ...`)

✅ **Shipped 2026-07-15 (M3).** Optional `if <expr>` after the pattern in a match
arm; guarded + unguarded arms for the same variant merge into one switch case with
if/else inside.

### Slice patterns (`[first, .., last]`)

✅ **Shipped 2026-07-15 (L1).** `[first, .., last]` destructuring on `Vec<T>` /
`[T; N]` in match position; `..` matches zero or more middle elements (no binding
in v1). Both backends emit index + length checks.

### `if let` / `while let`

✅ **Shipped 2026-07-15 (M1).** Both forms desugar to `match expr { Opt.Some(v)
then { … } _ then {} }`; the checker handles the resulting match arms.

### Labeled break / continue

✅ **Shipped 2026-06-23 (item 25).** `break inner` / `break middle` / `break outer`
exit a specific enclosing loop by position. Parser uses 2-token lookahead; checker
assigns synthetic `__vani_pos_N` labels; LLVM backend searches by label.

### Try-block / `try { ... }`

**Not in vāṇī.** No try-block construct (the `try EXPR`
keyword + postfix `?` cover the propagation cases).

**Workaround:** wrap the fallible operations in a helper fn
that returns `Result<T, E>` and `try` / `?` propagates.

---

## Numerics + types

### Bigint / arbitrary-precision

**Not in vāṇī.** Integers are `i64` / `u64` / smaller.
Runtime overflow guards are now emitted (L4, 2026-07-16) — every signed `+`, `-`,
`*` site gets a guard; the SMT pass elides guards it can prove safe from `requires`
bounds. See the **Integer overflow runtime guards** row in Mixed-feature gaps below
(now marked fixed).

**Workaround:** for cryptographic / arbitrary-precision
work, use C bigint via FFI (`extern "C"` declarations to
GMP / mbed-crypto).

### Float bit-twiddling without union

**Partial.** `f64_to_bits(x: f64) -> u64` + `f64_from_bits`
shipped; works for the common cases.

### `Decimal` / fixed-point

**Not in vāṇī.** Use scaled integers (multiply by 100 for
2-decimal money math).

### SIMD intrinsics

**Partially supported.** vāṇī has three explicit SIMD register types (all shipped):

- `vec128<T>` — 128-bit, 7 builtins (`simd_splat/load/store/add/sub/mul/reduce_add`).
  Maps to NEON on AArch64, SSE on x86-64, RVV on RISC-V.
- `vec256<T>` — 256-bit, 7 builtins (`simd256_*`). ✅ Shipped SIMD-9 (2026-07-10).
  Maps to AVX2 `ymm`; 2×NEON on AArch64 without SVE; RVV with `vsetvli vl=8`.
- `vec512<T>` — 512-bit, 7 builtins (`simd512_*`). ✅ Shipped M4 (2026-07-15).
  Targets AVX-512 zmm / SVE-512 / RVV VLEN=512.

**Still not available:** platform-specific intrinsics (`__m256i`, `vaddq_s64`,
`_mm_aesenc_si128`, etc.). For these, use an `extern "C"` FFI shim compiled
with the target's intrinsic headers. See `docs/simd_ffi_shims.md`.

### GPU / hardware-accelerated math (CUDA, ROCm, TensorRT, DLA)

**Not in vāṇī, and no compiler backend is planned.** There's no PTX/
SPIR-V/HSA lowering path — vāṇī only ever emits LLVM IR or C. Same
answer applies to any dedicated hardware-acceleration API (CUDA
Runtime/cuBLAS/cuDNN, ROCm/HIP, TensorRT, DLA via TensorRT's
execution-provider flag).

**Workaround: `extern "C"` FFI, the same mechanism SIMD intrinsics
above already uses**, not a new compiler feature. This is
lower-risk than it sounds — confirmed directly against the existing
FFI ABI rules (`tutorials/src/intermediate/09a_ffi_primer.md`):
scalars/`Str`/`ref T`/`mut ref T`/`#[repr(C)]` structs already cross
the boundary cleanly, and `ref Vec<T>` passing a contiguous buffer
pointer to a C function is already proven end-to-end by the SIMD
shims above (`docs/simd_ffi_shims.md`'s `neon_matmul(a: ref Vec<i64>,
b: ref Vec<i64>, ...)`). A device pointer can be carried as an opaque
`i64` handle (`mut ref i64` for an out-parameter like
`cudaMalloc(void**, size_t)`'s device-pointer slot) rather than a raw
pointer type, which does NOT cross the FFI boundary in v1. The GPU
kernel/device code itself still has to be written and compiled by the
vendor's own toolchain (`nvcc`, `hipcc`) and linked in via
`--link-with`/`-l<name>` — vāṇī can never emit device code without a
real GPU backend, independent of the FFI question.

Full scoping + effort estimate for building this as Kosh packages
(`vani-cuda`, `vani-rocm`, `vani-tensorrt`): see kosh-index/ROADMAP.md's
"Planned: hardware-acceleration tier".

---

## Modules + packages

### Re-export with rename

✅ **Shipped 2026-07-14 (B2).** `pub use foo::bar as baz;` is supported; the
regression test `top_level_use_of_pub_use_as_rename` locks the interaction.

### Visibility modifiers beyond `pub` / `pub(kosh)`

**Limited; `pub(kosh)` is enforced for external Kosh-package access,
not yet for same-project sibling modules.** ✅ Partially fixed
2026-07-22 (see `docs/v1_limitations.md` L23). `pub(kosh)` now mangles
to a name no externally-written qualified reference can match (same
trick private items use, one tier up) -- verified directly: a `[deps]`
package's `pub(kosh)` function called as `pkgname::item` from a
separate consumer project is correctly rejected. Still open: a
same-project sibling module reaching across module boundaries into
another module's `pub(kosh)` item is *also* currently rejected (no
caller-identity tracking yet to distinguish that from a genuinely
external Kosh consumer) -- stricter than the intended design, though
not a regression (before this fix nothing was enforced at all). Rust's
`pub(crate)`, `pub(super)`, `pub(in path)` aren't there either.

**Workaround for the remaining gap:** don't rely on `pub(kosh)` for
same-project cross-module sharing yet -- use plain `pub`, or move the
sharing functions into the same module so intra-module bare calls
apply instead. Module-private and (as of this release) `pub(kosh)`
against external Kosh consumers are both genuinely enforced.

### Kosh dependency namespacing

✅ **Shipped 2026-07-21** (Kosh namespacing arc, 6 phases — see
`docs/kosh_namespacing_design.md`). Every `[deps]` package is compiled
inside an implicit `module <pkg_name> { ... }`, so its functions (and
any exported struct types) are called as `pkgname::item` — a package
defining `fn abs(...)` no longer collides with the vāṇī builtin `abs`,
or with any other package's `abs`, ever. Dependency resolution is fully
transitive and deduplicated by `(name, version)`: two packages that
share a third ("diamond" dependency) resolve to one compiled copy
regardless of how deeply each vendors it. Circular `[deps]` graphs are
rejected at compile time with a full cycle-chain diagnostic
(`pkg_a -> pkg_b -> pkg_a`), reusing the same Tarjan SCC algorithm that
backs `vanic acyclicity`. `vani.lock` records the full resolved graph,
not just direct deps.

### Workspace / multi-crate package

**Not in vāṇī.** Each `vani.toml` is a single crate; no
workspace concept.

**Workaround:** monorepo with one manifest, or build each
"crate" separately and link via `--link-with`. Note this is
distinct from the Kosh *dependency* namespacing above (which is
shipped) — a workspace would let one `vani.toml` define multiple
crates sharing a lockfile, which doesn't exist yet.

### Built-in test runner (`#[test]`)

✅ **Shipped 2026-07-16 (XL2).** `#[test]` attribute on functions; `vanic test
file.vani` collects test fns, synthesises a harness main (each fn called; pass =
print "ok", fail = assert aborts with message), compiles+runs via CC.

---

## Summary table

| Feature | Status | Workaround |
|---|---|---|
| Generic trait bounds | indirect | iface dispatch as `dyn Iface` param |
| Generic fn calling generic fn (nested mono) | ✅ SHIPPED XL4 2026-07-15 | — |
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
| `select!` | ✅ SHIPPED L3 2026-07-16 | — |
| Pin<&mut Self> | NOT planned | restructure to avoid self-refs |
| Mutex<Vec<T>> | ✅ SHIPPED v0.1.1 | — |
| Proc macros | absent | external build-time codegen script |
| `macro_rules!` | absent | generic fns + per-call codegen |
| Reflection | absent | `dyn Iface` / explicit enum dispatch |
| Custom layout (`#[repr(C)]`, `#[repr(packed)]`) | ✅ SHIPPED L2 2026-07-16 | — |
| Or-patterns | ✅ SHIPPED M2 2026-07-15 | — |
| Pattern guards | ✅ SHIPPED M3 2026-07-15 | — |
| Slice patterns | ✅ SHIPPED L1 2026-07-15 | — |
| `if let` / `while let` | ✅ SHIPPED M1 2026-07-15 | — |
| Labeled break/continue | ✅ SHIPPED item-25 2026-06-23 | — |
| Bigint | absent | C FFI (GMP) |
| Runtime integer overflow guards | ✅ SHIPPED L4 2026-07-16 | — |
| Decimal / fixed-point | absent | scaled integers |
| SIMD intrinsics (vec128/256/512) | partial — explicit types shipped; platform intrinsics absent | `extern "C"` FFI shim |
| `pub use foo::bar as baz` | ✅ SHIPPED B2 2026-07-14 | — |
| Workspaces | absent | monorepo or `--link-with` |
| `#[test]` runner | ✅ SHIPPED XL2 2026-07-16 | — |

The deliberate omissions (Rc, multi-lifetime, Pin) are
philosophical — vāṇী picks structural prevention over
flexibility. Of the original accidental omissions, 11 have
shipped since 2026-06-20. Remaining gaps: Bigint, Decimal,
Workspaces, generic trait-bound syntax, Stream<T>, proc macros.

---

## Mixed-feature gaps

Combinations of v1-supported features that don't compose
today. Each row lists the shape, the reason it rejects, and
the v1 workaround. The adversarial test set under
[`examples/edge_cases/`](../examples/edge_cases/) keeps
checking these as the compiler evolves.

| Combination | Why it rejects | Workaround |
|---|---|---|
| `(Box<T>, U)` tuple element | ✅ **Fixed (v0.1.4, 2026-06-20)** — tuples now allow non-Copy elements; the tuple itself becomes non-Copy; scope-exit Drop walks each element; moving a non-Copy var into a tuple marks it moved; LetTuple destructuring marks the source binding moved. Regression test: `mix_tuple_non_copy.vani`. | No workaround needed. |
| `(OwnedStr, U)` tuple element | ✅ **Fixed (v0.1.4, 2026-06-20)** — same fix as above. | No workaround needed. |
| `Option<Box<T>>` enum payload | ✅ **Fixed (v0.1.4, 2026-06-20)** — checker now admits `Box<T>` in enum variant payloads; C + LLVM backends emit correct scope-exit Drop for Box payloads. Regression test: `mix_box_enum_payload.vani`. | No workaround needed. |
| `Result<Box<T>, E>` | ✅ **Fixed (v0.1.4, 2026-06-20)** — same fix as above; `Box<T>` is now a valid payload for any enum variant, including Result-style enums. | No workaround needed. |
| `Vec<(i64, OwnedStr)>` etc. with non-Copy tuple element | Tuples themselves now allow non-Copy elements (v0.1.4), but `Vec<non-Copy-tuple>` has not been verified end-to-end yet. | If you hit issues, wrap the tuple in a named struct (structs with non-Copy fields in Vecs are well-tested). |
| `HashMap<K, V>` with non-scalar V | ✅ **Fixed (Arc 4)** — `hashmap_insert` now accepts `OwnedStr`, `Vec<i64>`, tuple, `f64`, and `Vec`-typed values. Full K-V matrix shipped. | No workaround needed. |
| `Mutex<Vec<T>>`, `Atomic<Vec<T>>` | `Mutex<T>` is now parametric over any T (v0.1.1) — `Mutex<Vec<i64>>` works. `Atomic<T>` payload is still i64-width–shaped only. | For Atomic, use a `Mutex<Vec<T>>` instead; or channel-transfer ownership. |
| Closure capturing non-Copy binding | ✅ **Fixed 2026-07-15 (L5)** — FnOnce semantics: heap-malloc env, env-nulled after call, scope-exit Drop, double-call guard. Tests: `aff_closure_*` in lib.rs. | No workaround needed. |
| `box(X { ... } as dyn Iface)` inline | **Was a compiler panic; fixed 2026-06-09.** Now works on both backends. | Pinned by a lib regression test; previously the workaround was `let v = X { ... }; box(v as dyn Iface);` — the let-bind form was always safe. |
| `enum Outer { Wrap(Inner) }` where Inner is also an enum | **Was a rejection ("payload must be assignable to Inner, got Inner"); fixed 2026-06-09.** | Pinned by `nested_enum_payload_accepts_enum_construction`; the parser-stamped `Type::Struct(Inner)` is now resolved to `Type::Enum(Inner)` for enum variant payloads (the resolve pass missed them before). |
| Inline closure inside `implement Iface for T { fn m { ... } }` | **Was a compiler panic ("anonymous fn survived lambda-lift"); fixed 2026-06-09.** | The lambda-lift pass now walks impl + methods-block bodies BEFORE they're hoisted. Pinned by two regression tests. |
| Anonymous fn called inline from a Vec slot (`fs[0](10)`) | ✅ **Fixed 2026-07-14 (B3)** — `ExprKind::IndirectCall` AST node added; parser emits it for non-Var callees; checker type-checks callee as `FnPtr`/`Closure`. | No workaround needed. |
| Generic fn `<T>` inferred from complex argument types (e.g. `Vec<T>` with Vec-typed arg) | Rejected — v1 inference supports literal / Var / (v3.1 only) Ref(Var) at the T-position. | Pre-extract the arg into a Var, or restructure the fn to not be generic over composite types. |
| Turbofish syntax (`f::<i64>(arg)`) | Not supported in v1. | Rely on inference (which is limited; see above). |
| OwnedStr payload bound in match arm returned as OwnedStr | ✅ **Fixed 2026-07-15 (M5)** — scrutinee Drop suppressed only on direct move-out (arm body = Var(binding)); view-only / no-binding arms retain the Drop. Fixes double-free exit-116 crash. | No workaround needed. |
| **Integer overflow runtime guards** | ✅ **Fixed 2026-07-16 (L4)** — `llvm.sadd/ssub/smul.with.overflow` in LLVM; `__builtin_add/sub/mul_overflow` in C. SMT elision extended to discharge guards when `requires` bounds prove safety. | No workaround needed. |
| Two refs to the same Vec at once (read + write) | Aliasing rule: many shared XOR one mut. | End one borrow before taking the other; or split the operation into two passes. |
| Generic fn with multiple lifetime-distinct ref params returning a ref | Path-D territory; v1 only does single-ref-param elision. | Split into two narrower fns, each with one ref param. |
| `async fn` containing a `dyn Iface` method call across an `await` | ✅ **Confirmed working (2026-08-03, feature-combination gap audit category 11)** — this entry was stale. A `dyn Iface`-typed local binding held across TWO separate `await` points, with a method call both before and after the second await, produces correct output on both backends (verified with two different concrete types behind the same `dyn` binding). No Pin-like restriction is needed or present. | No workaround needed. |
| Recursive `Drop` impl that calls another `Drop` impl on a borrowed field | Borrow-checker rule: `mut ref Self` during drop can't pass to another `Drop` taking `mut ref Self`. | Implement Drop only at the outermost level; let the compiler chain field-by-field drops automatically. |
| `ref Vec<T>` parameter + loop-bound bounds check (C backend) | ✅ **Fixed 2026-07-14 (B1)** — `while_bounds_hints` now tracks `is_ref` per vec name; emits `xs->len` for `ref Vec<T>` params (C pointer) instead of `xs.len`. | No workaround needed. |

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

The honest list: vāṇī covers the **structural** ergonomics
of Rust (ownership, types, async) and a **subset** of the
syntactic ergonomics. If you want maximally-concise
expression of every CS pattern, Rust is the answer. If you
want a smaller, fully-verified surface that's safe by
construction on the hosted target, that's what vāṇī ships.
