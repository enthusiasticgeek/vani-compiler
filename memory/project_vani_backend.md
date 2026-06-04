---
name: project-vani-backend
description: "vāṇī compiler state — pipeline, backends, verifier; refreshed 2026-06-04 to reflect Arcs 1-9 + closure-as-value + async/await v1 + full SysV FFI"
metadata:
  node_type: memory
  type: project
---

`~/vani/` (renamed from `~/future-compiler/` on 2026-05-21) is
a Rust-based verified-affine compiler. **VANI** = *Verbose
Alternative Natural Interface*; *वाणी* = Sanskrit for "speech".

## Pipeline

Lexer → Parser → Type-checker / SMT verifier (z3) → Typed IR
(tree-shaped, [`src/ir.rs`](src/ir.rs)) → SSA IR
([`src/ssa.rs`](src/ssa.rs), [`src/ssa_pass.rs`](src/ssa_pass.rs))
→ one of four backends:

| Backend | Module | Role |
|---|---|---|
| `LlvmBackend` | [`src/backend_llvm.rs`](src/backend_llvm.rs) | Tree-LLVM — fallback for shapes SSA-LLVM doesn't cover |
| `CBackend` | [`src/backend_c.rs`](src/backend_c.rs) | Tree-C — same role for C output |
| `ssa_backend_llvm` | [`src/ssa_backend_llvm.rs`](src/ssa_backend_llvm.rs) | Default LLVM path |
| `ssa_backend_c` | [`src/ssa_backend_c.rs`](src/ssa_backend_c.rs) | Default C path |

The `emit_c_via_ssa` / `emit_llvm_via_ssa` helpers in
[`src/main.rs`](src/main.rs) run a per-program
`ssa_path_supports` gate and fall through to the tree backend
when SSA can't represent a shape (Vec<Atomic|Channel>,
parallel-for-with-payloaded-enums, etc.).

## Language surface (as of #291)

- **Scalars:** i8/i16/i32/i64, u8/u16/u32/u64, f32/f64, bool
- **Aggregates:** `Vec<T>` (heap, affine, monomorphized), `[T;N]` (stack), `Tuple` (Copy-only in v1). **Nested arrays** `[[T;N]; M]` / `[Vec<T>; N]` work end-to-end (closure #291 Phases 1–4 — Copy-element restriction lifted; per-slot per-field drops; `clone_at(ref arr, i)` extended; tree-LLVM Vec rvalue `len` spills to alloca + GEP `.len` + load).
- **Strings:** `Str` (borrowed `const char*`), `OwnedStr` (heap, affine)
- **User types:** `struct {...}`, `enum {Variant, Variant(payload)}`. **Generic struct + enum decls** — `struct Pair<A, B> { … }`, `enum Result<T, E> { Ok(T), Err(E) }` via `Type::Apply { name, args }` + mangled names like `Result__Vec_I64___AllocError` (closure #281).
- **Mixed-payload enums** — variants with different payload types share one enum (closure #283). C uses tagged union (`union { Type0 v_Ok; Type1 v_Err; }`); LLVM uses `[N x i8]` byte buffer + per-variant bitcast.
- **Prelude** — `Option<T>`, `Result<T, E>`, `AllocError` injected at AST level after parsing (NOT source-prepend — would shift diagnostic spans). Closure #282.
- **References:** second-class `ref T` / `mut ref T` (params only)
- **Function-pointers:** `fn(T) -> R` — first-class, Copy. FFI-safe in param + return position (closure #279).
- **`dyn IfaceName`:** Phases 1–5 shipped (closures #220–#228). Owned + `ref dyn` + `Vec<dyn>` + struct fields of dyn; auto-`==` polish; fat-pointer dispatch.
- **FFI v1–v8** — `extern "C" fn` declarations (#269), `--link-with` / `-l<name>` linker flags (#270), call-site checker (#271), codegen with mangled symbols (#272), struct-by-value rejection w/ `ref T` hint (#273), linker-discovery polish (#274), callbacks via `Type::FnPtr` (#279), System V x86-64 small-struct return lowering (#288). Net: `qsort`-style callbacks + libc string / math interop.
- **vani.toml manifest** — `[package].entry` auto-discovery + `find_manifest` parent-walk (#280); `[deps]` inline-table for multi-file dependency wiring (#287). Hand-rolled minimal-TOML parser at `src/manifest.rs`.
- **Attributes — `#[bounded(N)]`** — first attribute in the language (#286). New `#` token. Tree-LLVM uses thread-local globals + per-Return decrement (#289); SSA-LLVM mirrors (#290); C emits a thread-local counter with GCC `__attribute__((cleanup))`.
- **Concurrency:** `parallel for` with reductions (`+`, `*`, `&&`, `||`, `&` / `|` / `^`, `min`, `max`), `task` + `join`, `Atomic<T>`, `Channel<T,N>`, `Mutex<T>` + `Guard<T>` (i64 only). **Queued:** `Condvar` — pairs with `Mutex` / `Guard` for "wait until predicate" patterns; codegen reuses Linux futex / Win32 WaitOnAddress / pthread-cond runtime already in tree.
- **Control flow:** if/else, while, for-range, for-iter, break, continue, match (int / bool / enum / Str / **f64** patterns — closure #278 added `Pattern::Float(f64)` + `check_match_float` desugar), `try` keyword (Rust `?` sugar for early-return on None/Err).
- **Block expressions:** `let r = { stmts; tail };` — Let + Print stmts before tail (closures #129, #194-#201 hardened the scope drops). DynCoerce non-Var hoisted via synthetic Block expr (#276); `let _ = make_struct()` discard frees heap fields (#277).
- **Builtins:** `vec(...)`, `push`, `push_mut`, `pop`, `set`, `clone`, `clone_at` (Vec + arrays), `len`, `atomic_*`, `channel_send/recv`, `mutex_lock`, `guard_get/set`, `try_vec(n) -> Result<Vec<i64>, AllocError>` (#284 — fallible alloc with malloc + null-check).
- **Namespaces / modules:** shipped (closures #242–#258) — `module foo { … }`, `pub` / `pub(kosh)`, `use` import forms (top-level + inside module bodies), `pub use` re-exports, orphan rules.
- **Devanagari Sanskrit / Hindi / Marathi keyword aliases** — Phase 1 (single-word + multi-word post-lex merger, #235–#237) + Phase 2 (SOV word order for range `for`, verb-at-end statements, 3-way alias parity for previously English-only keywords, #265–#267).

## Verifier (z3)

`requires` / `ensures` / `invariant` / `prove` clauses discharge
via z3. BitVec overflow-aware arithmetic, IEEE-754 floats with
casts, shifts, inline-call discharge of callee ensures, loop
invariants with post-loop narrowing, contradictory-requires
detection, Vec-builtin length facts, SMT array theory for
indexing, bounds elision, effects/ownership analysis
(`pure fn`, affine `Vec<T>` / `OwnedStr`), race-freedom proofs
for `parallel for`.

## Affine ownership

Auto-Drop at scope exit for every non-Copy non-moved binding.
Partial-move tracking is one-level deep (`let xs = t.x` works;
nested `let y = t.x.inner` rejected). Multi-field drop order
is declaration-order today (Rust convention = reverse-decl;
deferred). `Drop for T` is suppressed when T has heap fields
(per-field free runs instead).

## Big items closed across sessions (#193-#291)

- Block-expression family (#129, #194-#201, #207): scope-exit
  drops, sibling-let drops, shadow-name handling, `let _`
  discard, RHS move tracking, walker recursion for enum / alias
  / tuple resolution, user-Drop for Copy structs.
- Parametric type spelling (#208-#215): eight bugs across
  `c_element_storage`, `element_tag`, `vec_struct_tag`,
  `format_declarator` for Atomic / Channel / FnPtr shapes.
- `try` keyword polish (#217-#218): nested blocks, multiple
  `try`s in one body.
- `pop(mut ref xs) -> T` builtin (#219) — Vec-as-stack complete.
- **Vtables epic A — all 5 phases (#220-#228)**: `Type::Object`
  + `dyn Iface` parsing, coercion + method dispatch, codegen
  (tree-C + tree-LLVM), collections + borrows (`Vec<dyn>`,
  `ref dyn`, struct fields of dyn), auto-`==` polish. 11
  dedicated phase-3 lib tests.
- User-Drop two-signature shipped (#229): `fn drop(self: T)`
  (by-value — only for T without heap fields) AND
  `fn drop(self: mut ref T)` (runs first, then per-field free —
  works for any T including OwnedStr / Vec / nested-struct
  fields).
- Move-rejection diagnostic carries type-aware fix hint —
  `ref x` for borrowing, `clone(x)` for deep copy; exclusive
  handles say "cannot be cloned" (#260).
- Parallel-for implicit-reduction race check — captured Copy
  mutation without `reduce` clause errors at compile time (#259).
- `examples/memory_safety.vani` — 7 canonical safety patterns
  end-to-end (#261).
- Codegen fixes: SSA-LLVM identity-cast `bitcast` for pointer
  types (#263); `len(ref OwnedStr)` 4-layer dereference fix
  (#262).
- **Namespaces / modules (#242-#258)**: `module foo { … }`,
  `pub` / `pub(kosh)`, `use` import forms (top-level + inside
  module bodies), `pub use` re-exports, orphan rules,
  collision diagnostics, formatter round-trip.
- **Devanagari Phase 2 (#265-#267)**: SOV word-order parsing
  for range `for` (`i के लिए 0 से 5 तक`), verb-at-end statements
  (`X पुनरागम;` / `… लिखो;` / `cond सुनिश्चित;` / `expr प्रमाण;`),
  3-way alias parity for previously English-only keywords.
- **FFI epic — v1-v8 (#269-#274, #279, #285, #288)**:
  `extern "C" fn` declarations, `--link-with` / `-l<name>`
  flags, call-site checker, codegen w/ mangled symbols,
  struct-by-value rejection with `ref T` hint, linker-discovery
  polish, callbacks via `Type::FnPtr`, System V x86-64
  small-struct return lowering.
- Parallel-for purity hole closed in reduction RHS (#275).
- DynCoerce non-Var hoist via synthetic Block expr (#276).
- `let _ = make_struct()` discard of fresh struct value frees
  heap fields (#277).
- **Match on f64 (#278)**: `Pattern::Float(f64)` AST variant +
  `check_match_float` desugar to nested IfExpr; diagnostics
  for missing wildcard, duplicate literals, NaN-in-pattern.
- **vani.toml manifest (#280 + #287)**: v1 `[package].entry`
  auto-discovery; v2 `[deps]` inline-table for multi-file
  dependency wiring. Hand-rolled minimal-TOML parser.
- **Generic struct + enum declarations (#281)**:
  `Type::Apply { name, args }`, monomorphization with mangled
  names. Prelude injected at AST level — `Option<T>`,
  `Result<T, E>`, `AllocError` (#282).
- **Mixed-payload enum lift (#283)**: C tagged union
  + LLVM byte-buffer + bitcast.
- **`try_vec(n) -> Result<Vec<i64>, AllocError>` (#284)** —
  fallible allocation builtin.
- **Attribute syntax + `#[bounded(N)]` (#286, #289, #290)**:
  first attribute in the language. Thread-local counter +
  GCC cleanup attribute (C); thread-local global + per-Return
  decrement (LLVM).
- **Nested arrays — Phases 1-4 (#291)**: array-element Copy
  restriction lifted; `clone_at(ref arr, i)` extended to
  arrays; per-slot per-field drops including struct-slot
  field walks; tree-LLVM Vec rvalue `len` spills to alloca
  + GEP + load.
- Tree-C / SSA-C codegen quality: `-Wstrict-prototypes`,
  `-Wmissing-braces`, `-Wunused-label`, `-Wswitch-bool`,
  `-Wuninitialized`, `-Wdiscarded-qualifiers` clean across all
  63 examples.

## Default-behavior reminders

- **LLVM is the default backend.** `intentc emit` /
  `intentc run` / `intentc build` route through
  `emit_llvm_via_ssa`. C backend stays available via
  `--backend=c`.
- **Use `llvm_type_string` for aggregates and references** —
  `llvm_type` intentionally panics on those so callers don't
  silently pick i64 as a fallback.
- **OMP_NUM_THREADS=1 under lli** — MCJIT isn't thread-safe
  for concurrent function resolution.
- **libgomp `-load=` probing** lives in
  `add_libgomp_load_flags` (in main.rs) and a test-local
  mirror in backend_llvm.rs. Honor `INTENT_LIBGOMP`.

## Test totals (2026-06-04)

**1800 lib + 56 parity tests green.** Cross-backend parity
runner covers all examples (now including closure-as-value,
async fn, await, CancelToken — `examples/closure_as_value.vani`,
`examples/async_fn.vani`, `examples/async_await.vani`). ASan /
UBSan clean across all examples. LLVM `opt -verify` /
`opt -O3` clean. (Win32 LLVM dispatch adds 4 host-gated tests
that fire on Windows hosts only.)

## Arcs landed since #291 (2026-06-03 → 2026-06-04)

- **Arc 4 — full HashMap K-V matrix:** OwnedStr K, OwnedStr V,
  (OwnedStr, OwnedStr), Tuple K, f64 K, Vec<i64> K all
  cross-backend. Examples: `hashmap_{str,strv,strstr,tup,f64,veck}.vani`.
- **Arc 5c — closure-as-value across fn boundaries** (commit
  `7cccc1b`). New `Type::Closure(Args, Ret)`. Lift-pass
  synthesizes env-struct + trampoline + magic call
  `__intent_make_closure_<N>`. C emits per-(args, ret) closure
  struct typedef `{uint64_t env; R (*call)(uint64_t, args);}`;
  LLVM emits `%intent_closure_<sig>` named struct + trampoline
  using `inttoptr`/getelementptr for env access. Call on
  Closure-typed value dispatches via `c.call(c.env, args)`.
  v1 restrictions: per-closure env in static __thread / module-
  global slot (no heap); Closure cannot outlive lift site; Copy
  captures only; no FnPtr→Closure auto-coerce. Example:
  `examples/closure_as_value.vani`.
- **Arc 7 SysV — float-class + mixed int/float ≤ 16 bytes FFI**
  (commit `69b5ec0`). Extended `is_ffi_safe_struct` via
  `is_ffi_safe_scalar` (integer + SSE eightbyte classes).
  Removed over-restrictive LLVM panic; cc-calling-convention
  lowering trusted. Closure #285 covered the integer-only
  ≤ 16-byte subset; Win64 + AArch64 gated on CI.
- **Arc 8 v1 — async fn / await / Future / Poll / CancelToken
  at parser+prelude layer** (commits `2e649ff`, `e50dc20`,
  `25b5a84`). `async fn foo() -> R { … return v; }` desugars to
  `fn foo() -> Future<R> { … return Future.Ready(v); }`;
  `await(expr)` desugars to a `match` that extracts
  `Future.Ready`'s payload. Prelude grew `enum Poll<T>`,
  `enum Future<T>`, `struct CancelToken { cancelled: bool }`.
  `inject_prelude` now forwards prelude structs to
  `program.structs` (had only forwarded enums before). v1
  semantics: async fn runs synchronously, always emits
  `Future.Ready`; users get the right types + syntax; runtime
  steps (state-machine codegen + event loop + non-blocking I/O)
  upgrade later without breaking source.
- **Arc 9 c+d — `pub(kosh)` visibility tier + `pub use` chained
  re-exports** already on `main` via closures #257 + #258.

## Pending epics (post-Arc-9 — 2026-06-04)

| # | Item | Effort | Status |
|---|---|---|---|
| Arc 8 runtime | State-machine codegen (8c) + event-loop runtime (8d) + non-blocking I/O (8e) + `examples/async_io.vani` (8h) | ~25-30h focused / 6-8 commits | **OPEN — next session.** See STATUS.md "📋 NEXT SESSION" block for the verbatim handoff prompt. v1 source surface already shipped (8a/8b/8f/8g). |
| Arc 7 Win64/AArch64 | Cross-platform FFI classifier | ~6-8h | OPEN — gated on cross-platform CI availability |
| Arc 9 a/b/e/f | `kosh.toml` manifest + resolver + registry + stdlib-as-kosh | ~25-30h | **DEFERRED** — registry hosting choice still pending per user |
| Arc 10 | Devanagari SOV grammar (per-language parser mode + SOV constructs + alias-table completion) | ~15-20h | **BLOCKED** — needs grammar consultant per design notes |
| B | Partial-move expansion (multi-level `moved`, multi-field reverse-decl drop, Mutex<non-i64>) | medium per item | Queued |
| C | Mixed-payload-enum drop-dispatch follow-up (`switch (tag)` Drop arms) | small | Queued; construction + match-extract done via #283 |
| D | Block-expr stmt vocabulary extension (admit Reassign / control flow) | medium | Queued |
| E | Condition variables (`Condvar`) — pairs with `Mutex<T>` + `Guard<T>` | medium | ✅ AFFINE; codegen reuses Linux futex / Win32 WaitOnAddress / pthread-cond runtime; queued 2026-05-27 |
| F | Data structures + algorithms roadmap — Levels 1-4 follow-ons (BTree / Deque / BinaryHeap / iterators / arena-based trees + graphs) | high (multi-session) | See [[project-vani-data-structures-roadmap]] |

## Related memory

- [[project-vani-status-file]] — STATUS.md update protocol
- [[feedback-vani-file-access]] — standing approval for /tmp + ~/vani access
