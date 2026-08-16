# vāṇी v1 — known limitations

> Single canonical catalog of v1 deviations from textbook
> behavior. Each entry: what it is, why it's that way today,
> the workaround currently in use, and a pointer to the place
> the user-visible workaround appears.

> **⚠️ Reader notice — many of these limitations have already been fixed.**
>
> This catalog was started at v0.1.0 and is kept as a permanent reference.
> Entries that have been resolved carry a **✅ SHIPPED (version / date)**
> badge in their heading. Only the items **without** a ✅ badge are still
> open in the current release.
>
> **At v0.8.1 (2026-07-25): 19 of 19 original entries resolved; L20–L22 added
> and fixed; L23 fixed in two phases, 2026-07-22 and 2026-07-24; L25 added
> 2026-07-24 and fixed 2026-07-25. Open items: L5 (by design), L6 (by
> design), L10-macOS (no hardware), L13 (partial — `match` SOV by design),
> L14 (by design for v1), L24 (Windows-only, scoped).**
>
> **Update (2026-08-14): L26–L30 added since the note above, none of
> them fixed this pass** — L26 (loop-body bounds-check elision
> disabled for soundness after BUG-181, no unsound-recovery fix
> planned), L27 (`vanic run`'s JIT path deliberately skips `opt -O3`,
> a latency/pathological-loop trade-off, not applied), L28
> (float-to-int `as i64` casts stay unchecked pending a semantics
> decision — checked vs. saturating), L29 (✅ partially — `downto`
> shipped 2026-08-13; `step`/stride-N still open), L30 (`tcp_recv`'s
> buffer has no byte-accessor builtin yet, found building the
> networked tic-tac-toe capstone, filed not fixed). Still open, same
> as before: L5, L6, L10-macOS, L13 (partial), L14, L24, L25.
>
> **Update (2026-08-15): L31 added, not fixable in vāṇी.** A
> `detach`'d task still running when `main` returns can segfault
> under `vanic run` (LLVM `lli` JIT) -- found via localfuzz,
> root-caused to `lli`'s own JIT-code-teardown-vs-still-running-
> pthread race (confirmed via `vanic build`/AOT running the identical
> program correctly every time), so there's no vāṇी-side codegen fix
> available. Documented as a `vanic run`-only caveat instead.
>
> | # | Summary | Status |
> |---|---|---|
> | L1 | Enum destructure-bindings of affine payloads | ✅ Resolved v0.1.0 (2026-06-07) |
> | L2 | `Box<T>` owning heap pointer | ✅ Resolved v0.1.0 (2026-06-08) |
> | L3 | Pattern-match scrutinee must be by value | ✅ Resolved v0.1.0 (2026-06-07) |
> | L4 | Reference types in let/struct/Vec/return positions | ✅ Resolved v0.1.0 (2026-06-09); path-D deferred by design |
> | L5 | `let mut x` not supported | ⬜ By design — use `mut ref` parameter |
> | L6 | `for VAR in xs` consumes; borrow with `ref` | ⬜ By design — write `for v in ref xs` |
> | L7 | `for VAR in ref obj.field` | ✅ Resolved v0.1.0 (2026-06-07) |
> | L8 | C-codegen: `Vec<dyn Iface>` collision in struct fields | ✅ Resolved v0.1.0 (2026-06-07) |
> | L9 | LLVM backend: non-ASCII identifiers | ✅ Resolved v0.1.0 (2026-06-08) |
> | L10 | macOS runtime unverified; Windows fully verified | ⬜ macOS deferred (no host); Windows ✅ 2026-06-16 |
> | L11 | Runtime PRINT output uses ASCII numerals in Devanagari mode | ✅ Resolved v0.1.0 (2026-06-07) |
> | L12 | SMT can't prove across function-call boundaries | ✅ Resolved v0.1.0 |
> | L13 | SOV reshape: `match`-as-statement stays keyword-first | ⬜ Partially resolved 2026-06-19 — `fn`/`struct`/`enum` SOV ✅; match by design |
> | L14 | Dialect-aware errors translate prefix only | ⬜ By design for v1 |
> | L15 | `Mutex<T>` payload limited to `i64` | ✅ Resolved v0.1.1 (2026-06-18) |
> | L16 | `Barrier` synchronization primitive missing | ✅ Resolved v0.1.1 (2026-06-18) |
> | L17 | `RwLock<T>` / `ReadGuard` / `WriteGuard` missing | ✅ Resolved v0.1.1 (2026-06-18) |
> | L18 | File I/O, stdin, stderr, flush — no native surface | ✅ Resolved v0.1.5 (2026-06-21) |
> | L19 | Bare-metal / custom OS — five gaps block production use | ✅ All 5 gaps resolved v0.1.6 (2026-06-21) |
> | L20 | S-19 lock-order detection is intra-procedural only | ✅ Fixed 2026-07-12 — held-set transitive analysis |
> | L21 | S-20 ISR mutex detection does not follow helper calls | ✅ Fixed 2026-07-12 — collect_locked_mutexes follows calls |
> | L22 | MISRA 13.2 eval-order check: adjacent args only | ✅ Fixed 2026-07-12 — any-distance duplicate detection |
> | L23 | `pub(kosh)`: external Kosh-boundary access enforced; same-project sibling-module access now allowed | ✅ Fixed 2026-07-24 (phase 2) |
> | L24 | `parallel for`'s Windows thread count is fixed at build time, not run time | ⬜ Windows-only; scoped, not started |
> | L25 | Windows: `print`/`f64_to_str` scientific-notation exponent width differs between C and LLVM backends | ⬜ Windows-only; scoped, not started |
> | L26 | Vec/array indexing inside a loop body never elides its bounds check, even when provably safe | ⬜ By design since BUG-181 (2026-08-12) — soundness over performance |
> | L27 | `vanic run`'s LLVM JIT path skips the `opt` optimizer, unlike `vanic build`/`--backend=c` | ⬜ Not fixed — deliberate latency/pathological-loop trade-off |
> | L28 | `as i64` (and other float-to-int casts) is unchecked, real UB when the value doesn't fit | ⬜ Not fixed — needs a checked-vs-saturating semantics decision |
> | L29 | `for i from lo to hi` is ascending-only | ✅ Partially resolved 2026-08-13 — `downto` added; `step`/stride-N still unsupported |
> | L30 | `tcp_recv`'s received bytes are not inspectable from vani code | ⬜ Not fixed — no `tcp_buf_byte_at`-style builtin yet |
> | L31 | `detach`'d task still running when `main` returns can segfault under `vanic run` (LLVM `lli` JIT only) | ⬜ Not fixable in vāṇी — root-caused to upstream `lli`'s own JIT teardown; `vanic build`/AOT confirmed correct |

Cross-referenced from:
- [`examples/language/english/design_patterns/README.md`](../examples/language/english/design_patterns/README.md) — the GoF pattern examples that hit each limitation
- [`STATUS.md`](../STATUS.md) — per-feature status banners
- [`TODO.md`](../TODO.md) — work items that lift each limitation

---

## Type-system limitations

### L1 — Enum destructure-bindings of affine payloads ✅ SHIPPED 2026-06-07

**Status**: Resolved in Phase 11 (second installment) for
by-value scrutinees. The `ref` scrutinee case is still
documented (see "remaining" below).

Pattern-match arms can now bind enum variant payloads of any
type — `Copy` (scalar), `OwnedStr` (heap-string, exposed as a
`Str` view), AND **affine** types like `Vec<T>` or
owning-field structs.

```vani
enum Node {
  Leaf(i64),
  Branch(Vec<i64>),
}

fn first(n: Node) -> i64 {
  return match n {
    Node.Leaf(v) then v,
    Node.Branch(xs) then xs[0],     // ✅ now works
  };
}
```

**Design**: the affine binding is a **non-owning view** over
the scrutinee's heap. The scrutinee's own scope-exit
variant-drop keeps the single ownership of the buffer; the
binding's scope-exit is suppressed (`no_drop: true` flag in
the checker). Arm bodies can **read** the binding freely; they
should **not** consume it (passing by value to a fn that takes
by value would double-free with the scrutinee's drop).

**Fix surface**:
- `src/checker.rs` — match-arm binding check: drop the
  "non-Copy payload type" rejection for by-value scrutinees;
  set `no_drop: true` on the binding's `VarInfo` when the
  binding type is non-Copy.

**Remaining limitation**: through a `ref T` / `mut ref T`
scrutinee (Phase 11 L3), affine-payload bindings are still
rejected — a proper borrow-view binding type is needed there.
The diagnostic now points at the documented workaround
(move-by-value).

**Regression coverage**:
- `lib.rs::affine_enum_payload_destructure_compiles_l1` —
  by-value scrutinee, simple `xs[0]` read.
- `lib.rs::affine_enum_payload_borrow_into_helper_l1` —
  arm body passes the binding by `ref` to a helper.
- `lib.rs::match_through_ref_with_affine_payload_still_rejected_l1`
  — pins the ref-scrutinee-still-rejected boundary.
- `lib.rs::enum_non_copy_payload_binding_now_compiles_via_value_l1`
  — updated from the previous rejection test.

### L2 — `Box<T>` owning heap pointer ✅ SHIPPED (Phases 1 + 2 + 3 + 3b + recursive-drop + dyn-sugar)

**Status**: All four phases plus the recursive-drop follow-up
plus the expected-type-threading sugar shipped 2026-06-07 →
2026-06-08. Box\<T\> for Copy + sized inner types,
`Box<dyn Iface>` (heap-allocated concrete behind an owning fat
pointer), `Box<Vec<T>>`, and `Box<OwnedStr>` all work on both
backends (C + LLVM).

- The recursive-drop wiring chains scope-exit Drop into the
  inner type's destructor before freeing the box's own heap
  slot; the source binding is marked moved by
  `check_box_builtin` so its own Drop is suppressed.
- The expected-type-threading sugar lets `let b: Box<dyn Iface>
  = box(value);` work without the `as dyn Iface` cast (Var-
  shaped source only, same restriction as the explicit-cast
  form).
- `unbox(ref b)` is still gated to Copy / dyn inner — copying
  a non-Copy inner by value would alias the heap slot the box
  still owns. Pass `ref b` to a function or store the box in a
  struct field to use Vec / OwnedStr inner.

```vani
struct Circle { r: i64 }
interface Drawable { fn area(self: Circle) -> i64; }
implement Drawable for Circle {
  fn area(self: Circle) -> i64 { return self.r * self.r; }
}

// Originally documented L2 blocker — works in Phase 3:
struct Drawer { rend: Box<dyn Drawable> }

fn main() -> i64 {
  let c: Circle = Circle { r: 7 };
  let b: Box<dyn Drawable> = box(c as dyn Drawable);  // ✅ heap-
                                                       // alloc +
                                                       // own fat ptr
  return b.area();                                     // ✅ 49
}
```

**Phase 3 surface**:
- `Box<dyn Iface>` parses + type-checks; checker validates the
  source implements the interface.
- `box(value as dyn Iface)` heap-allocates a copy of `value`,
  builds the owning fat pointer { vtable, heap_data_ptr }, stores
  in the local. The local IS the 16-byte fat pointer struct
  (NOT a pointer to one).
- Method dispatch through `Box<dyn Iface>` / `ref Box<dyn Iface>`
  uses the same vtable path as plain `dyn Iface`.
- Drop emits `free(box.data)` — frees the heap concrete; the
  fat-pointer struct in the local alloca is reclaimed with the
  stack frame.
- Struct fields can hold `Box<dyn Iface>`; the outer struct's
  drop walker chains into the field's `.data` free.

**Phase 3b add (LLVM backend codegen)** — 2026-06-08:
- `llvm_type_string(Type::Box(Box::new(Type::Object(name))))`
  → `%intent_dyn_<Iface>` (the fat-pointer struct, 16 bytes),
  NOT a pointer to one.
- `llvm_byte_size(Type::Box(Box::new(Type::Object(_))))` → 16.
- `__box_new` LLVM IR — when inner is `Type::Object(iface)`:
  ```llvm
  %raw = call i8* @malloc(i64 <sizeof concrete>)
  %heap = bitcast i8* %raw to %Struct_Concrete*
  %loaded = load %Struct_Concrete, %Struct_Concrete* %src_addr
  store %Struct_Concrete %loaded, %Struct_Concrete* %heap
  %s0 = insertvalue %intent_dyn_Iface undef, %intent_vtbl_Iface* @intent_vtbl_Iface_Concrete, 0
  %s1 = insertvalue %intent_dyn_Iface %s0, i8* %raw, 1
  ```
- Drop emission: load the fat pointer struct, `extractvalue …, 1`
  to pull out the i8* `.data`, `call void @free(i8* …)`.
- The pre-codegen `program_uses_box_dyn` gate is removed; both
  backends now reach parity for the full v1 surface.

**Remaining follow-up (queued, optional)**:
- **`box(concrete)` without explicit `as dyn Iface`** — needs
  expected-type threading into `check_box_builtin`. Cosmetic
  convenience; the current `box(value as dyn Iface)` surface
  is fully functional.

```vani
fn main() -> i64 {
  let b: Box<i64> = box(42);        // ✅ heap-allocates the slot
  let v: i64 = unbox(ref b);        // ✅ reads back; b still owns
  return v;                          // implicit free() at scope exit
}

struct Point { x: i64, y: i64 }
struct Holder { b: Box<Point> }     // ✅ Box in struct field
                                    // outer struct drop chains
                                    // into the Box's free()
```

**Phase 1 surface**:
- `Box<T>` type where T is Copy + sized (primitives like `i64`,
  `bool`, …; Copy structs).
- `box(expr)` constructor (heap-allocates + copies expr into
  the slot).
- `unbox(ref b)` reader (returns the inner value by Copy).
- Affine: each Box has exactly one owner; transferring requires
  a move; the implicit scope-exit drop frees the heap slot.
- Struct-field storage works — the outer struct's drop walker
  chains into the Box field's `free()`.

**Remaining restrictions, as of Phase 1 (2026-06-07 -- see the
"Superseded" note right below for what shipped since)**:
- **`Box<dyn Iface>`** — the original documented blocker
  (`struct Drawer { r: Box<dyn Renderer> }`) needs vtable
  plumbing through the heap allocation. Phase 3.
- **Box of affine inner type** (`Box<Vec<i64>>`, `Box<OwnedStr>`)
  — requires recursive drop walks. Queued.

**Superseded (this whole "Phase 1 surface" section below is a
frozen 2026-06-07 snapshot; not updated as later phases shipped)**:
`Box<dyn Iface>` (Phase 3, 2026-06-08), `Box<Vec<T>>`/`Box<OwnedStr>`
(L2 follow-up, 2026-06-08), and — as of BUG-97, 2026-08-04 — ANY
struct type (not just Copy ones), including self-referential
("recursive") structs like `struct Node { next: Option<Box<Node>>
}`, all now work. `check_box_builtin`'s own doc comment in
checker.rs and [the Box/RAII tutorial](../tutorials/src/intermediate/03a_box_raii_primer.md)
describe the current supported surface; the still-open restrictions
today are `Box<Box<T>>` and `Box` of a tuple.

**Phase 2 add (LLVM backend codegen)** — 2026-06-07:
- `llvm_type_string(Type::Box(inner))` → `{T}*`.
- `__box_new` LLVM IR: `call i8* @malloc(i64 <sizeof>)` +
  `bitcast i8* to T*` + `store T <value>, T* <ptr>`.
- `__box_get` LLVM IR: `load T*` from the `ref Box<T>` (T**),
  then `load T` from the resulting Box pointer.
- Scope-exit drop in `TypedStmt::Drop` arm: load T*, bitcast to
  i8*, `call void @free(i8* …)`.
- `is_scalar(Type::Box(_))` returns true so the uniform Let
  alloca-and-store path applies.
- Pre-codegen `program_uses_box` gate removed.

**Fix surface** (already in `main` as of 2026-06-07):
- `src/ast.rs::Type::Box(Box<Type>)` — new affine variant.
- `src/parser.rs` — `Box<T>` recognized as a type when followed
  by `<` (lookahead so user `struct Box { … }` still parses).
- `src/checker.rs::check_box_builtin` + `check_unbox_builtin` —
  the constructor + reader, with the v1 Copy-only restriction.
- `src/backend_c.rs` — `__box_new` emission as a GCC compound
  statement `({ T* __b = malloc(sizeof(T)); *__b = (x); __b; })`,
  `__box_get` as `(*(*ref))`, scope-exit drop as `free(b)`,
  struct-field drop chain in `emit_struct_field_drops`, type
  spelling in `c_type_name` / `format_declarator` / `c_element_storage`.
- `src/backend_llvm.rs::program_uses_box` — pre-codegen detector
  that surfaces the actionable `--backend=c` directive.

**Regression coverage**:
- `lib.rs::box_i64_alloc_unbox_compiles` — primitive round-trip.
- `lib.rs::box_bool_alloc_unbox_compiles` — Box<bool>.
- `lib.rs::box_copy_struct_compiles` — Box<MyStruct>.
- `lib.rs::box_in_struct_field_compiles_and_frees` — the
  documented blocker shape (Box as struct field).
- `lib.rs::box_rejects_non_copy_inner_type` — pins the v1 Copy
  restriction with a Box<Vec<i64>> rejection.
- `lib.rs::box_llvm_codegen_emits_malloc_store_free` — pins the
  LLVM IR shape (`@malloc` + `bitcast i8*` + `@free`).
- `lib.rs::box_llvm_struct_field_storage_compiles` — Box as
  struct field reaches LLVM codegen.
- `lib.rs::box_dyn_iface_construction_compiles` — Box<dyn Iface>
  round-trip (Phase 3 + 3b).
- `lib.rs::box_dyn_iface_method_dispatch_through_ref` — method
  call through `ref Box<dyn Iface>` parameter.
- `lib.rs::box_dyn_iface_in_struct_field_unblocks_l2` — the
  originally documented L2 blocker case.
- [`examples/language/english/box_dyn_iface.vani`](../examples/language/english/box_dyn_iface.vani)
  runs on both backends — prints `49 16`.

### L3 — Pattern-match scrutinee must be by value ✅ SHIPPED 2026-06-07

**Status**: Resolved in Phase 11 (first installment).

`match` now accepts a scrutinee of type `ref T` / `mut ref T`
where T is an enum, integer, or bool. The checker unwraps the
reference for the dispatch-shape check; codegen inserts a
`load` (LLVM IR) or `*` deref (C) before reading the tag / payload.

```vani
fn unwrap_or(r: ref Result, def: i64) -> i64 {       // ✅ now works
  return match r {
    Result.Ok(v) then v,
    Result.Err(_) then def,
  };
}
```

**Fix surface**:
- `src/checker.rs` — `match`-expression dispatch: unwrap
  `Type::Ref` / `Type::RefMut` before classifying the
  scrutinee shape.
- `src/backend_llvm.rs` — `TypedExprKind::Match` arm: emit a
  `load` through the pointer when `scrutinee.ty` is a reference,
  then take the existing payloaded-enum dispatch path on the
  loaded value.
- `src/backend_c.rs` — same shape via a `(*expr)` deref before
  the existing `__scr.tag` / `__scr.payload` reads. Also fixed
  `format_declarator` to emit `const Enum_<Name>*` (not
  `const int32_t*`) for `ref T` where T is a payloaded enum.

**Regression coverage**:
- `lib.rs::match_through_ref_scrutinee_compiles_and_dispatches`
  pins the lift + the C-emit param-type shape.
- `lib.rs::match_through_mut_ref_scrutinee_compiles`
  pins the `mut ref` variant.

---

## Reference + binding limitations

### L4 — Reference types in let/struct/Vec/return positions ✅ FULLY SHIPPED Phases 1+3+4 (B) + Path (C) lifetime elision (2026-06-09)

```vani
let r: ref Foo = ref some_foo;       // ❌ — let annotation cannot be a reference type
```

**Why**: vāṇी references are second-class — they live only in
parameter / argument position. Storing a reference in a `let`
binding would require first-class lifetime tracking that v1
doesn't have.

**Workaround**: pass the reference directly through function
parameters; bind the value first and take `ref` at the call
site.

**Partial lift shipped 2026-06-08 (early session)**: synthesized
v3.1 Task structs (`Task__<fn>`) accept `ref Struct` /
`mut ref Struct` parameter types.

**Phase 1 shipped 2026-06-08**: refs in `let` bindings now
accept. `let r: ref Foo = ref foo;` compiles end-to-end.

**Phase 3 + scope-escape analyzer shipped 2026-06-08**:
user-declared struct fields can now hold ref types
(`struct Bag { item: ref Foo }`). The scope-escape analyzer
catches the new escape vectors:
- `return Bag { item: ref local };` — REJECTED (local drops on
  return, leaving the returned Bag with a dangling ref).
- `bag.item = ref inner_scope_local;` where `bag` is in an
  outer scope — REJECTED (inner local drops at scope-exit
  before bag does).

The walker treats Call/MethodCall/Len/Index args as
"consuming" positions (refs there don't propagate to the
enclosing value), so `read_bag(ref b)` and `len(ref xs)` in
return position still pass.

**Two real bugs found and fixed 2026-07-25** (scoping ref-capturing
closures surfaced both — see `docs/ref_capturing_closures_design.md` and
`docs/TODO_CURRENT.md`'s BUG-7/BUG-8 for full writeups; both fixed same
day, not just filed):
- **BUG-7**: the escape check above had a real bypass — routing the exact
  same escaping struct through one extra `let` binding first
  (`let h = Bag { item: ref local }; return h;`) slipped past undetected,
  a confirmed live dangling reference at runtime, not just a
  diagnostic-timing gap. Fixed.
- **BUG-8**: even the *legitimate*, non-escaping case this section
  describes as "shipped" — `struct Bag { item: ref Foo }`, construct and
  read within a safe scope — silently returned garbage under the LLVM
  backend when indexing through a `ref`-typed **Vec** field specifically
  (`struct Holder { v: ref Vec<f64> }`, `h.v[i]`); the C backend was
  unaffected. The regression test cited for this Phase
  (`l4_b_phase3_user_struct_ref_field_now_accepted`, `lib.rs:9709`) only
  ever called `compile()`/`compile_to_llvm()` — never actually executed
  the emitted IR — so a codegen-only value bug could pass it cleanly.
  Fixed; new test actually executes via `lli`.

**Phase 4 shipped 2026-06-09**: `Vec<ref T>` and `Vec<mut ref T>`
are now first-class element types. Both backends emit per-shape
typedef + helpers (`intent_vec_ref_<inner_tag>` / `intent_vec_refmut_<inner_tag>`);
`element_tag` and `vec_struct_tag` route `Type::Ref(_)` /
`Type::RefMut(_)` to identifier-safe suffixes. Since refs are
Copy, `__free` skips per-element drop. The scope-escape analyzer
extends to `push(mut ref vec, ref X)`: when X's binding sits at
a deeper scope than the Vec receiver, the analyzer rejects with
a "ref to '{X}' would dangle when '{X}'s scope ends" diagnostic.
Acceptance: `examples/language/english/vec_of_ref.vani` (ASan + LSan clean on the
C backend; cross-backend stdout parity).

**Path (C) shipped 2026-06-09**: `fn foo(p: ref T) -> ref T`
under the single-ref-parameter elision rule. The returned ref's
lifetime is inferred to equal the single ref parameter's
lifetime; no `'a` syntax. Zero-ref-param or two-or-more-ref-param
returns reject with clear diagnostics suggesting refactoring.
Call sites propagate the source's lifetime through to subsequent
escape checks (push/FieldAssign/Return), so a chain of ref-
returning calls is correctly bound to the original source. See
[`examples/language/english/path_c_ref_returns.vani`](../examples/language/english/path_c_ref_returns.vani)
for the canonical shapes.

**Still rejected** (path-D territory, deferred indefinitely):
- Multi-input distinct lifetimes (`fn pick<'a, 'b>(a: &'a T, b: &'b T) -> &'a T`).
- Lifetime-parameterized struct definitions (`struct View<'a>`).
- Closures capturing refs that outlive the closure declaration.

### L5 — `let mut x` is not a thing

vāṇī's `let` always binds a single owner. There's no
explicitly-mutable `let mut x = ...;` form.

```vani
let mut x: i64 = 0;       // ❌ — expected identifier where `mut` appears
```

**Why**: mutability is governed by how a binding's value is
later used (the method receiver shape `mut ref self` triggers
the borrow, not the let-binding declaration). The Rust-style
`let mut` would be redundant in vāṇी's model.

**Workaround**: declare `let x: T = ...;` and use mutations
through a `mut ref` parameter or a method declared with
`fn ... (self: mut ref T, ...)`. See the
[Proxy pattern example](../examples/language/english/design_patterns/structural/proxy.vani).

---

## Iteration limitations

### L6 — `for VAR in xs` consumes; use `for VAR in ref xs` to borrow

```vani
let xs: Vec<i64> = vec(1, 2, 3);
for v in xs { ... }       // ❌ — moves xs; can't be used after
for v in ref xs { ... }   // ✅ — borrows by reference
```

**Why**: vāṇी's affine ownership means a default value-form `for`
would consume the Vec. The keyword-first `ref` annotation makes
the borrow explicit at the loop head.

**Workaround**: write `for v in ref xs` whenever you want to
keep using `xs` after the loop. The compiler error message
already nudges toward this fix.

### L7 — `for VAR in ref obj.field` ✅ SHIPPED 2026-06-07

**Status**: Resolved in Phase 11 (third installment).

Iteration over a struct field through the for-loop head is now
supported on both backends.

```vani
methods on Subject {
  fn publish(self: ref Subject, value: i64) -> i64 {
    for o in ref self.observers {
      // ...
    }
    return 0;
  }
}
```

Consuming a field (`for v in self.field` without `ref`) is
still rejected because it would move out of a field; the
diagnostic now points at the `ref` workaround.

**Fix surface**:
- `src/parser.rs` — `parse_for_stmt_inner` accepts a dotted
  identifier path (`IDENT . IDENT (. IDENT)*`) as the iterable.
  Stored as a single dot-separated string in the existing
  `Stmt::ForIter::collection` field.
- `src/checker.rs` — main type-check arm: split the collection
  on `.`, resolve the head via `env.lookup`, walk the field
  chain through `env.lookup_struct`, land on the field's
  underlying type. Move-out-of-field rejected with workaround
  pointer.
- `src/backend_c.rs::emit_for_iter` — when the collection name
  contains `.`, build the C accessor as `(*head).field` (so
  `ref Bag` heads get the right deref). Plain non-dotted
  collections take the original `local_name` path unchanged.
- `src/backend_llvm.rs` — `TypedStmt::ForIter` handler: when
  the collection name contains `.`, look up the head's
  address in `ctx.locals`, then walk each field name in the
  path emitting a `getelementptr` through the
  `LLVM_STRUCT_FIELDS_REGISTRY`. The resulting addr is used
  as the effective `coll_addr` for the existing Vec/Array
  iteration emit.

**Regression coverage**:
- `lib.rs::for_iter_field_through_ref_self_compiles_l7` —
  the worked Bag-iteration example.
- `lib.rs::for_iter_field_consume_form_is_rejected_l7` —
  pins the move-out-of-field rejection.

---

## Backend-specific limitations

### L8 — C-codegen: `Vec<dyn Iface>` as a struct field with multiple dyn types ✅ SHIPPED 2026-06-07

**Status**: Resolved in Phase 1.2.

`Vec<dyn Iface>` bundles now include the Iface name in the C
typedef (`intent_vec_dyn_<Iface>`) so two struct fields holding
`Vec<dyn A>` and `Vec<dyn B>` get distinct bundles. Vec bundles
for `dyn Iface` elements are also deferred to the unified topo
loop so they're emitted after `emit_dyn_iface_typedefs` lands
the per-Iface fat-pointer typedefs — no more
"unknown type name `intent_dyn_Drawable`" at cc time.

**Fix surface**:
- `backend_c.rs::element_tag` — added the `Type::Object(iface)`
  arm returning `dyn_<iface>` so `Vec<dyn Drawable>` lands at
  `intent_vec_dyn_Drawable`, distinct from `intent_vec_dyn_Loggable`.
- `backend_c.rs::vec_element_has_user_struct` — extended to
  also return true for `Type::Object`, deferring its Vec bundle
  to the unified topo loop (which runs after the dyn typedef
  pass).

**Regression coverage**: `lib.rs::two_dyn_iface_vecs_in_struct_field_emit_distinct_c_bundles`
pins both bundle names + their relative ordering vs the dyn
typedef.

**Note**: the SSA-C path was already correct (it builds its Vec
bundle around the per-Iface element type at lowering time); the
collision only happened on tree-C. The fix is C-only.

### L9 — LLVM backend: identifiers with non-ASCII chars ✅ SHIPPED 2026-06-08 (gaps found + fixed 2026-08-10, BUG-168)

LLVM IR's bare-identifier grammar restricts characters to
printable ASCII, and C's identifier grammar is similarly
ASCII-only. Devanagari/Bengali/Tamil/etc. function / struct /
local names mangle to a hex-encoded form on emission: LLVM via
`llvm_mangle_ident` (see
[src/backend_llvm.rs:239](../src/backend_llvm.rs)), C via the
analogous `sanitize_ident` in `src/backend_c.rs`. Both encode
each non-ASCII character's codepoint as `_<hex>_` (an injective
mapping — collision-resistant even when the compiler can't just
drop straight to raw UTF-8 bytes, which C's identifier grammar
also forbids).

**Status**: Shipped 2026-06-08, but incompletely -- re-verified
2026-08-10 while translating Sanskrit/Hindi/Marathi example
identifiers and found **BUG-168**: 1 LLVM call site (a
match-arm-binding value-copy path) and 7 C call sites (match-arm
bindings, `let`/reassign desugar inside blocks, mixed-payload
enum union member names, shallow-free cleanup, and a closure
env-capture uid) spliced the raw source identifier directly into
the emitted symbol instead of routing through the sanitizer,
producing invalid LLVM IR / C on a Devanagari-named local
crossing a `?`-operator desugar. All 8 sites fixed; the earlier
"C backend uses UTF-8 directly" framing above was never accurate
-- `sanitize_ident` has existed and hex-encoded non-ASCII since
BUG-105/106 (2026-08-04), it just wasn't called at every site.

---

## Platform / runtime limitations

### L10 — macOS runtime verification deferred; Windows fully verified ✅ 2026-06-16

**Windows status** (verified 2026-06-16 on Windows 11 Home, Rust
1.96 GNU, LLVM 22, z3 4.16, gcc 12 via MSYS2):
- Compiler builds cleanly (`cargo build --release`).
- 2108+ lib tests pass (`cargo test`).
- All 5 async-TCP examples (`async_showcase`, `echo_loop`,
  `echo_loop_break`, `echo_match_stress`, `tcp_echo_epoll`) pass
  on both C and LLVM backends end-to-end.
- Root cause of Windows async hang fixed 2026-06-16 (commit `8193760`):
  `WSAECONNRESET` (10054) / `WSAECONNABORTED` (10053) from an RST-close
  were mapped to `-1` (error), not `0` (EOF). WSAPoll kept returning
  "ready" on the error socket → infinite recv loop. Fix: both C-backend
  and LLVM-backend Windows `recv_nb` helpers now return `0` for both
  reset codes.
- Six Linux-only tests (`epoll_emits_helpers_in_llvm`,
  `host_is_linux_helper_present`, etc.) are correctly guarded
  with `#[cfg(target_os = "linux")]`.
- `volatile_read`/`volatile_write` builtins ship on all 4 Windows
  backends (commit `2cea04a`).
- `echo_loop_windows_byte_count_matches_c` e2e test de-ignored and
  green after the WSAECONNRESET fix.

**macOS status**: still deferred — no Darwin host available.
`#elif defined(__APPLE__)` branches (kqueue, `EVFILT_TIMER`,
`__error()`, `pipe+pthread` timer) are compiled-in but unrun.
Report issues so Phase 5 hot-spots in
[ARC8_V3_PLAN.md](../ARC8_V3_PLAN.md) get tuned.

**Workaround**: Linux and Windows users are fully supported.
macOS users should try the C backend (`--backend=c`) first and
file issues with the kqueue error message.

### L11 — Runtime PRINT output uses ASCII numerals ✅ SHIPPED 2026-06-07

**Status**: Resolved in Phase 1.1.

`print x` where `x: i64` (or i8/i16/i32/u8/u16/u32/u64) now emits
the decimal in Devanagari digits (`०..९`) when the file declares
`// vani-lang: sanskrit | hindi | marathi`. The conversion is a
digit-by-digit UTF-8 codepoint replacement (U+0966..U+096F via
the 3-byte sequence `E0 A5 (A6..AF)`); a leading ASCII `-` for
negative numbers is preserved verbatim.

**Coverage**:
- Tree-C backend (`backend_c.rs`) — emits `intent_print_int_dev`
  helper into the runtime prelude; the printf-fallback arm in
  `emit_print_expr_no_newline` dispatches to it.
- SSA-C backend (`ssa_backend_c.rs`) — same helper (shared via
  `emit_intent_print_int_dev_c`); the `intent_print_item`
  handler routes integer width arms through it.
- Tree-LLVM (`backend_llvm.rs`) — defines
  `@intent_print_int_dev(i64)` in pure LLVM IR (snprintf +
  putchar loop, no platform-dependent stdout globals); the
  signed + unsigned int arms dispatch to it.
- SSA-LLVM (`ssa_backend_llvm.rs`) — same IR helper, dispatched
  from the integer fallback in the `intent_print_item` handler.

**Mechanism**:
1. The lexer's `detect_language_pragma` populates a thread-local
   `PrintLangMode` (`Ascii` | `Devanagari`) when the
   `// vani-lang:` line resolves to a Devanagari dialect.
2. `lib.rs::compile` saves the mode after lexing the user
   source, restores it after `inject_prelude` lexes the pragma-
   free PRELUDE (which would otherwise reset to Ascii).
3. Each backend reads the mode at emit time and gates both the
   helper definition and the print-site dispatch on
   `PrintLangMode::Devanagari`.

**F64 / Str / Bool**: unchanged — they keep the printf path. The
helper is integer-only in v1; floats would need a separate
fraction-digit pass and bool/string don't need numeral
translation.

**Regression coverage**: `lib.rs::devanagari_pragma_emits_devanagari_print_digits_c`
and `lib.rs::ascii_pragma_keeps_printf_for_int_print_c` pin the
emit shape so a future refactor can't silently regress either
side.

---

## Verification (SMT) limitations

### L12 — SMT can't prove across function-call boundaries by default ✅ SHIPPED

**Status**: Resolved. The SMT verifier bridges through calls
whose callee declares an `ensures` clause referencing the
special return-value identifier `_return`.

```vani
fn add(a: i64, b: i64) -> i64
ensures _return == a + b;
{
  return a + b;
}

fn main() -> i64 {
  prove add(3, 4) == 7;          // ✅ proven via ensures clause
  prove add(double(3), 1) == 7;  // ✅ nested calls also work
  return 0;
}
```

When the callee has no `ensures` clause, the verifier surfaces
the actionable hint suggesting one be added:

> cannot prove expression: SMT encoder skipped this query
> (function call `foo` not supported in SMT v1) (add an
> 'ensures' clause to the callee so the verifier can use its
> return value).

**Fix surface** (already in `main` as of 2026-06-07):
- `src/checker.rs::prove_with_calls` — orchestrates the
  rewrite passes.
- `src/checker.rs::rewrite_calls_to_fresh_vars` — replaces
  `f(a, b)` in the proof goal with a fresh symbol
  `__call_N`, registers `__call_N`'s type, and injects the
  callee's `ensures` clauses (with `_return` → `__call_N` and
  formal-params → actual-args substitutions) as facts.
- `src/checker.rs::rewrite_method_calls_to_calls` —
  desugars `p.m(a)` into `Type_m(p, a)` before the call-
  rewriter sees it, so method ensures bridge the same way.

**Regression coverage**:
- `lib.rs::smt_proves_through_ensures_bridge` pins the
  basic add-via-ensures case.
- `lib.rs::smt_proves_through_nested_call_ensures` pins
  the nested-call substitution (`add(double(3), 1) == 7`).
- `lib.rs::smt_hint_when_callee_missing_ensures` pins the
  "add an ensures clause" diagnostic.

**Recursive / mutually-recursive / reentrant calls are not a special
case of this mechanism — they're the same mechanism.** `check_function`
(`checker.rs:11396`) checks each function exactly once, in a flat
top-level loop (`checker.rs:1787-1788`); a `Call` node's fact-generation
(`record_ensures_facts`, `checker.rs:39149`; `verify_call_args_in_expr`,
`checker.rs:39459`) only ever looks up the callee's *signature*
(`requires`/`ensures`) in the `signatures: &HashMap<String, Signature>`
table — never the callee's body, so it can't distinguish "callee is
some other function" from "callee is the function currently being
checked." There is deliberately no "currently verifying" stack
(confirmed absent by grep, and stated at `checker.rs:11704-11707`): none
is needed, because a call site never re-descends into a callee's body
in the first place, recursive or not. The practical upshot: a
recursive call's `ensures` is *assumed* as a fact at the recursive call
site while proving the *current* call's `ensures` — i.e. the `ensures`
clause doubles as an induction hypothesis, so it must be tight enough
for the induction step to hold, not just true for the base case. A
too-loose `ensures` (e.g. only `_return >= 0` on a summing recursion)
lets the solver pick an unconstrained huge value for the recursive
sub-call and find an overflow counterexample; tightening the `ensures`
to bound growth (e.g. `_return <= n * K`) gives the solver what it
needs to discharge the inductive step. See [Sec.12 SMT
deep-dive](https://github.com/enthusiasticgeek/vani-compiler/blob/main/tutorials/src/intermediate/12_smt_deepdive.md#recursive-and-reentrant-calls)
for a worked before/after example of exactly this.

**Complexity**: because callee bodies are never revisited, per-function
fact generation is a single structural walk over that function's own
AST (no blowup with recursion depth), and each proof obligation is one
Z3 query over the accumulated `smt_facts`, capped at a 5-second
wall-clock timeout (`run_z3`, `smt.rs:259-272`) with an exact-text query
cache (`smt.rs:165-197`) to skip repeat solver calls.

---

## Language-surface limitations

### L13 — SOV reshape: `match`-as-statement stays keyword-first ✅ Partially resolved 2026-06-19

vāṇी's SOV (Subject–Object–Verb) parser covers:
- 8 statement verb-at-end shapes: `let` / `return` / `print` / `assert` / `prove` / range-`for` / `if`/`else` / `while`
- **3 top-level declaration shapes (new in item 16, 2026-06-19)**: `fn` (name-first), `struct` (name-first), `enum` (name-first)

The one remaining keyword-first-only construct is **`match`-as-statement**. `match` is expression-only in vāṇी; there is no `;`-terminated match statement in either keyword-first or SOV form.

**Why**: `match x { … }` is already natural in SOV files — the scrutinee `x` comes before the verb-like `match` keyword. Forcing a terminal verb (`x match { … } ;`) would be awkward and is "declined as design" for v1.

**Workaround**: use SOV-let to bind a match result:
```vani
r: i64 = x match { 0 then 0, _ then 1 } माना;
```
or use the keyword-first form: `let r = match x { … };`

**SOV fn/struct/enum surface (item 16, 2026-06-19)**:
```vani
// SOV fn — verb after the signature:
add(a: i64, b: i64) -> i64 fn { return a + b; }

// SOV struct — keyword after the name:
Point struct { x: i64, y: i64, }

// SOV enum — keyword after the name:
Dir enum { North, South, }
```
The parser rewrites the token stream to canonical keyword-first order before dispatching to the existing parse functions, so downstream passes are unaware of the surface form.

### L14 — Dialect-aware errors translate prefix only

When a file declares `// vani-lang: <dialect>`, error messages
render with localized labels (Sanskrit `त्रुटिः`, Hindi `त्रुटि`,
Marathi `चूक`, plus ~58 other dialects shipped through 2026-06-08
— including Russian `ошибка`, German `Fehler`, Hebrew `שגיאה`,
Cherokee `ᎤᎴᏗ`, etc.) and a translated leading prefix for the
most common error families. The body of the error stays English
so search engines + existing docs still match.

**Why**: full body translation requires translating dynamic
content (paths, type names, variable names) — too noisy for
v1. The leading prefix gives the user a dialect-aware entry
point without making search worse.

**Workaround**: native-speaker linguists adding to the prefix
table in
[`src/diagnostic.rs:localize_message`](../src/diagnostic.rs).
Tracked in [`docs/archive/grammar_review_queue.md`](archive/grammar_review_queue.md).

---

## Concurrency limitations

### L15 — `Mutex<T>` payload limited to `i64` ✅ SHIPPED v0.1.1 (2026-06-18)

**Status**: Resolved. `Mutex<T>` is now parametric over any element type.

Previously `mutex_new` / `mutex_lock` / `guard_get` / `guard_set` only
accepted `i64` payloads. As of v0.1.1, `T` can be any type: scalars,
`bool`, structs, enums, `Vec<T>`, `OwnedStr`.

```vani
struct Config { limit: i64, debug: bool }
let cfg: Mutex<Config> = mutex_new(Config { limit: 100, debug: false });
{
  let g: Guard<Config> = mutex_lock(ref cfg);
  let c: Config = guard_get(ref g);
  print "limit =", c.limit;
}
```

`Atomic<T>` payload remains i64-width only (i8..i64, u8..u64, bool).

### L16 — `Barrier` synchronization primitive missing ✅ SHIPPED v0.1.1 (2026-06-18)

**Status**: Resolved. `barrier_new(n)` / `barrier_wait(mut ref b)` shipped
on all backends (Linux, Windows, macOS).

A Barrier makes all N threads wait at a checkpoint until every thread
has arrived. Uses a generation counter to prevent ABA races — safe to
reuse in a loop. `barrier_wait` returns `true` for exactly the last
thread to arrive.

```vani
let b: Barrier = barrier_new(3);
let t1: Task<i64> = task stage_one(1, mut ref b);
let t2: Task<i64> = task stage_one(2, mut ref b);
let _ = stage_one(3, mut ref b);  // main thread is the third
let _ = join t1;
let _ = join t2;
```

### L17 — `RwLock<T>` / `ReadGuard<T>` / `WriteGuard<T>` missing ✅ SHIPPED v0.1.1 (2026-06-18)

**Status**: Resolved. `rwlock_new` / `rwlock_read` / `rwlock_write` /
`read_guard_get` / `write_guard_set` shipped on all backends.
State encoding: `0` = unlocked, `N > 0` = N concurrent readers, `-1` =
write-locked. Both guard types are affine; their Drop triggers the
appropriate unlock automatically (RAII). Parametric over any element
type `T`.

```vani
let rw: RwLock<i64> = rwlock_new(0);

let r: ReadGuard<i64> = rwlock_read(ref rw);
let v: i64 = read_guard_get(ref r);
// ReadGuard drops here — read lock released

let w: WriteGuard<i64> = rwlock_write(mut ref rw);
let _ = write_guard_set(mut ref w, v + 1);
// WriteGuard drops here — write lock released
```

**Channel<T, N>** also became parametric over any element type in v0.1.1
(previously scalar-only). No L-number assigned because it was documented
as a missing feature in STATUS.md rather than a listed limitation here.

---

## I/O limitations

### L18 — File I/O, stdin, stderr, flush ✅ SHIPPED v0.1.5 (2026-06-21)

**Status**: Resolved. Native `FileHandle`, `file_open`, `file_read_line`,
`file_write`, `file_close`, `file_flush`, `stdin_read_line`,
`flush_stdout`, and the `eprint` statement all ship in v0.1.5 on both
C and LLVM backends.

**IO-1 update (2026-07-21, v0.5.4-dev)**: `file_open` grew a required
third `buffered: bool` argument — `true` for normal libc buffering
(the old default and behavior), `false` to call `setvbuf(f, NULL,
_IONBF, 0)` so every `file_write` reaches the OS immediately without
an explicit `file_flush`. This is a **breaking change** to `file_open`'s
arity — the old 2-arg call is now a compile error. C backend: routes
through a new `intent_file_open` runtime helper. LLVM backend: inlined
directly as `@fopen` + a conditional branch to a block calling
`@setvbuf` (no custom `@intent_*` symbol — see the note below about why
that path is a linking trap). `_IONBF`'s value is hardcoded per libc
family (glibc/BSD `_IONBF=2`, MSVC/MinGW `_IONBF=4`) since the LLVM path
calls `setvbuf` directly rather than through C headers; gated by the
same host-only `cfg!(target_os = ...)` limitation as `host_is_windows`
et al. (not yet cross-compilation-target-aware).

```vani
let f: FileHandle = file_open("/tmp/log.txt", "w", true);
if file_is_ok(ref f) {
  let _ = file_write(mut ref f, "hello\n");
  let _ = file_flush(mut ref f);
}
// f auto-closes at scope exit (RAII)

let line: OwnedStr = stdin_read_line();
let _ = flush_stdout();
eprint "fatal:", 42;
```

See [`examples/language/english/file_io.vani`](../examples/language/english/file_io.vani)
for the full worked example.

**Fixed 2026-07-24** (was: newly discovered bug 2026-07-21, NOT part of
IO-1): `file_read_line` and `stdin_read_line` were completely broken on
the LLVM backend, both `vanic run` (JIT) and `vanic build` (AOT) —
`backend_llvm.rs` emitted `call i8* @intent_file_read_line(...)` with no
corresponding `declare` or definition reachable from the LLVM path.
Fixed by defining both directly as ordinary LLVM IR functions in the
preamble, built from already-declared libc externs. See **BUG-1** in
`docs/TODO_CURRENT.md` for the full writeup, including a second,
related SSA-dispatch bug found and fixed in the same pass.

**Remaining scope** (device I/O — UART / I2C / SPI / RS485 / CAN):
these are kernel-ioctl-specific and remain a C-shim + FFI pattern by
design, not a gap being tracked toward native support. The relevant
kernel structs (`termios`, `i2c_msg`, `spi_ioc_transfer`, `can_frame`)
are all aggregate-by-value, which v1 rejects at the FFI boundary —
same reason as `termios` below. Write a thin C shim exposing
scalar-only functions and compile with `--link-with`. On bare-metal
targets (no OS, no ioctl) the equivalent path is `volatile_read`/
`volatile_write` directly against the peripheral's memory-mapped
registers instead of an FFI shim — see the embedded tutorial.
**PCIe / NVMe**: no native or shim pattern beyond what's already
here — PCIe config-space and NVMe are either accessed through an OS
driver (same `extern "C"` FFI pattern as UART below, against
`libpciaccess`/`libnvme` or a vendor SDK) or, for bare-metal driver
code talking to a memory-mapped BAR directly, the same
`volatile_read`/`volatile_write` MMIO primitives — there's no
protocol-specific language surface for either, by the same design
call as UART/I2C/SPI.

---

*Historical note (pre-v0.1.5): vāṇी v1 had no built-in file I/O layer.
The only I/O primitive was `print` / `write` to stdout.*

| Feature | v1 status (pre-v0.1.5) |
|---|---|
| `print` / `write` → stdout | ✅ shipped |
| `eprint` / stderr output | ❌ no language surface |
| stdin / `read_line` | ❌ no language surface |
| Flat file I/O (`open`, `read`, `write`, `close`) | ❌ no language surface |
| Device I/O (RS232 / RS485 / UART / `ioctl`) | ❌ no language surface |
| `flush` / `setbuf` / unbuffered stdout | ❌ no language surface |

For **device I/O** (Linux serial ports / UART), the `struct termios`
ABI is aggregate-by-value, which v1 rejects at the FFI boundary.
Write a thin C shim (`uart_helper.c`) that exposes scalar-only
functions and compile with `--link-with uart_helper.c`:

```c
// uart_helper.c
#include <termios.h>
#include <fcntl.h>
#include <unistd.h>

int uart_open(const char *path, int baud) {
    int fd = open(path, O_RDWR | O_NOCTTY | O_SYNC);
    // ... configure termios, set baud ...
    return fd;
}
int uart_write(int fd, const char *buf, int n) { return write(fd, buf, n); }
int uart_read(int fd, char *buf, int n)        { return read(fd, buf, n); }
int uart_close(int fd)                         { return close(fd); }
```

```vani
extern "C" fn uart_open(path: Str, baud: i32) -> i32;
extern "C" fn uart_write(fd: i32, buf: Str, n: i32) -> i32;
extern "C" fn uart_read(fd: i32, buf: Str, n: i32) -> i32;
extern "C" fn uart_close(fd: i32) -> i32;
```

```bash
vanic build my_uart.vani -o my_uart --link-with uart_helper.c
```

The same shape covers **I2C** and **SPI** on Linux — `i2c_msg` /
`spi_ioc_transfer` are just as aggregate-by-value as `termios`, so
the shim's job is the same: take scalars in, do the ioctl internally,
return scalars out.

```c
// i2c_helper.c
#include <linux/i2c-dev.h>
#include <sys/ioctl.h>
#include <fcntl.h>
#include <unistd.h>

int i2c_open(const char *path, int addr) {
    int fd = open(path, O_RDWR);
    if (fd >= 0) ioctl(fd, I2C_SLAVE, addr);
    return fd;
}
int i2c_write(int fd, const unsigned char *buf, int n) { return write(fd, buf, n); }
int i2c_read(int fd, unsigned char *buf, int n)        { return read(fd, buf, n); }
int i2c_close(int fd)                                  { return close(fd); }
```

```vani
extern "C" fn i2c_open(path: Str, addr: i32) -> i32;
extern "C" fn i2c_write(fd: i32, buf: Str, n: i32) -> i32;
extern "C" fn i2c_read(fd: i32, buf: Str, n: i32) -> i32;
extern "C" fn i2c_close(fd: i32) -> i32;
```

```c
// spi_helper.c
#include <linux/spi/spidev.h>
#include <sys/ioctl.h>
#include <fcntl.h>
#include <unistd.h>

int spi_open(const char *path, int mode, int speed_hz) {
    int fd = open(path, O_RDWR);
    if (fd >= 0) {
        ioctl(fd, SPI_IOC_WR_MODE, &mode);
        ioctl(fd, SPI_IOC_WR_MAX_SPEED_HZ, &speed_hz);
    }
    return fd;
}
int spi_transfer(int fd, unsigned char *tx, unsigned char *rx, int n) {
    struct spi_ioc_transfer t = { .tx_buf = (unsigned long)tx, .rx_buf = (unsigned long)rx, .len = n };
    return ioctl(fd, SPI_IOC_MESSAGE(1), &t);
}
int spi_close(int fd) { return close(fd); }
```

```vani
extern "C" fn spi_open(path: Str, mode: i32, speed_hz: i32) -> i32;
extern "C" fn spi_transfer(fd: i32, tx: Str, rx: Str, n: i32) -> i32;
extern "C" fn spi_close(fd: i32) -> i32;
```

**CAN** (SocketCAN on Linux) follows the same shim pattern against
`can_frame`; omitted here since it's a direct copy of the I2C/SPI
shape with different ioctl calls.

**Multi-line print**: `print` always emits exactly one trailing
newline. To print multiple lines use multiple `print` statements, or
embed `\n` escape sequences inside a string literal:

```vani
print "line one";
print "line two";
// OR:
print "line one\nline two";  // trailing \n still appended — gives 3 lines
```

**Tutorial reference**: [Intermediate 9 — FFI](../tutorials/src/intermediate/09_ffi.md)
shows the `--link-with` pattern for device I/O (UART/serial) that
still requires a C shim.

---

## Bare-metal / OS limitations

### L19 — Five gaps block bare-metal / custom OS production use ✅ All gaps resolved v0.1.6 (2026-06-21)

**Status**: All five gaps resolved. G2–G5 shipped 2026-06-21 (see commits
0f15440, 10118c1, 1bef47b). G1 (`--target <triple>`) shipped 2026-06-21
(cross-compile LLVM backend). Each was tracked in
[`docs/TODO_CURRENT.md`](TODO_CURRENT.md) items 18–22.

What **already ships** and works on bare metal today:
- `#[no_heap]` — compiler rejects any transitive `malloc` call
- `#[bounded_stack(N)]` — enforces stack budget across call graph
- `#[recursion_bound(N)]` — bounded recursion
- `#[interrupt]` — ISR calling convention
- `mmio_read_u32` / `mmio_write_u32` — 32-bit MMIO register access
- `volatile_read` / `volatile_write` — volatile memory operations
- `unsafe_alloc` / `unsafe_free` — raw manual allocation
- `region_new` / `region_alloc_i64` — bump/arena allocator
- `pool_new` / `pool_alloc` / `pool_free` — typed pool allocator
- `bptr_new` / `bptr_get` / `bptr_set` — bounded pointer (safe array-in-a-slab)
- FFI via `extern "C"` — call assembly startup code, C HAL drivers
- SMT `requires`/`ensures`/`prove` — formally-verified critical paths
- Affine types — use-after-free impossible at compile time

The five gaps:

---

#### G1 — No cross-compilation target flag (TODO 18) ✅ SHIPPED v0.1.6 2026-06-21

`vanic build` now accepts `--target=<triple>` (or `--target <triple>`).
It passes `--mtriple=<triple>` to `llc` and selects the cross-linker
via `$CROSS_CC` or `<triple>-gcc` (stripping `unknown-` from the triple).
Bare-metal triples (`*-none-eabi`, `*-elf`) suppress host libc/libm/OpenMP
link flags and auto-activate `--no-std` mode.

`vanic run --target=<triple>` also works:
- Bare-metal triples → helpful error pointing to `vanic build` + QEMU.
- Linux cross-targets → builds an ELF and runs via QEMU user-mode
  (`qemu-<arch>-static` on PATH or `$QEMU_<ARCH>` env var).

```bash
# ARM Cortex-M bare-metal
vanic build firmware.vani --target=arm-none-eabi -o firmware.elf \
  --link-with startup.c --link-with linker.ld

# RISC-V 32-bit bare-metal (CROSS_CC override if toolchain differs)
CROSS_CC=riscv32-elf-gcc vanic build blink.vani \
  --target=riscv32-unknown-none-elf -o blink.elf

# AArch64 Linux cross-compile + QEMU run
vanic run hello.vani --target=aarch64-unknown-linux-gnu
```

---

#### G2 — No `no_std` mode — C prelude always includes libc headers (TODO 19) ✅ SHIPPED v0.1.6 2026-06-21

`vanic emit --backend=c --no-std` (and `vanic emit-c --no-std`) suppress
all `#include <std*.h>` lines and emit a minimal typedef block instead
(`uint8_t`, `int64_t`, `size_t`, `uintptr_t`, `NULL`, plus forward
declarations for `malloc`/`free`/`abort` only). Bare-metal triples via
`--target` auto-activate no-std.

---

#### G3 — No `#[link_section]` attribute (TODO 20) ✅ SHIPPED v0.1.6 2026-06-21

`#[link_section = ".vectors"]` on `fn` declarations now emits
`__attribute__((section(".vectors")))` in C and `section ".vectors"` in
the LLVM IR `define` line.

---

#### G4 — No `#[no_mangle]` — generated symbols are name-mangled (TODO 21) ✅ SHIPPED v0.1.6 2026-06-21

`#[no_mangle]` on `fn` declarations now emits the bare name (no
`intent_` prefix, no Unicode mangling) in both C and LLVM backends.
Linker scripts can reference `Reset_Handler`, `_start`, `HardFault_Handler`
etc. directly.

---

#### G5 — MMIO only 32-bit — no `u8`/`u16` register access (TODO 22) ✅ SHIPPED v0.1.6 2026-06-21

`mmio_read_u8(addr)`, `mmio_read_u16(addr)`, `mmio_write_u8(addr, val)`,
`mmio_write_u16(addr, val)` all ship. They lower to
`*(volatile uint8_t*)` / `*(volatile uint16_t*)` in C and to volatile
`i8`/`i16` load/store with `zext`/`trunc` in LLVM IR.

---

#### Native bare-metal workflow (all gaps closed)

```bash
# ARM Cortex-M4 — single command, LLVM backend
vanic build firmware.vani \
  --target=arm-none-eabi \
  --link-with startup.c \
  -o firmware.elf

# Explicit no-std C emission (for review / hand-edit)
vanic emit firmware.vani --backend=c --no-std -o firmware.c
arm-none-eabi-gcc -mcpu=cortex-m4 -mthumb -nostdlib -ffreestanding \
  -T linker.ld firmware.c startup.c -o firmware.elf

# RISC-V 32-bit
CROSS_CC=riscv32-elf-gcc \
  vanic build blink.vani --target=riscv32-unknown-none-elf -o blink.elf

# 8/16-bit MMIO (previously needed FFI shims)
# mmio_write_u8(0x40020018, 0x05);  -- works natively now
```

#### Legacy workaround (obsolete since v0.1.6)

```bash
# 1. Emit C
vanic emit-c --backend=c firmware.vani > firmware.c

# 2. Strip libc includes (G2 workaround)
sed -i '/#include <std/d; /#include <string/d' firmware.c

# 3. Add your own forward decls and compile
arm-none-eabi-gcc -mcpu=cortex-m4 -mthumb -nostdlib -ffreestanding \
  -T linker.ld firmware.c startup.c hal.c -o firmware.elf
```

Use `#[no_heap]` on every function to get a compile error if any path
accidentally calls `malloc`. Use `mmio_read_u32`/`mmio_write_u32` for
32-bit peripheral registers; use FFI + C shims for 8/16-bit registers
until G5 is fixed.

---

## Adding to this catalog

When you hit a new v1 deviation:
1. Add an entry here with the L<N> label.
2. Cross-reference from any example file that exercises the
   workaround.
3. If the underlying bug has a fix path, link it in `TODO.md`.
4. If it's documented as "by design" (like L13), name the
   design rationale.

---

## Safety-analysis scope limitations

These three entries describe analysis passes that exist and fire
correctly for the common case, but have bounded scope. They are **not**
compiler bugs — the passes produce no false negatives within their
documented scope. They are documentation entries for Tool Qualification
purposes.

### L20 — S-19: lock-order deadlock detection is intra-procedural ✅ Fixed 2026-07-12

**Fix applied.** `enforce_lock_order` now uses a held-set analysis
(`build_lock_edges` / `build_lock_edges_expr`) that follows calls into
user-defined helpers with a clone of the caller's current held-lock set.
Callee-acquired locks are released on return (clone discarded), preventing
false cross-call ordering constraints. The `gap_s19_*` adversarial test
now asserts rejection. The original scope description is preserved below
for historical reference.

**Original scope.** `enforce_lock_order` (`src/safety.rs`) collected
`mutex_lock` call sequences by walking each function's body directly,
then builds a global acquisition-order graph from consecutive pairs
and runs DFS cycle detection. This correctly detects:

- Two functions that each acquire the same two mutexes in opposite
  orders (direct deadlock pattern).
- A single function whose control-flow branches acquire the same
  two mutexes in opposite orders.

It does **not** detect transitive patterns where function A acquires
`m_x` then calls a helper that acquires `m_y`, while function B
acquires `m_y` then calls a helper that acquires `m_x`. The helper's
lock is not visible in A's or B's body sequence.

**Evidence.** `gap_s19_lock_order_via_transitive_call_not_detected` in
`tests/safety_adversarial.rs` demonstrates the undetected case.

**Certification impact.** ISO 26262 Part 6 / DO-178C Table A-7 do not
require a single tool to find all deadlock patterns — they require the
verification method to be documented. This limitation must be disclosed
in the Tool Qualification Document. Supplement with:

- Code review checklist: *"All `mutex_lock` calls reachable from a
  function are listed in the lock-acquisition order table."*
- Integration test: exercise the lock-acquisition paths concurrently
  under a RTOS scheduler (if applicable) or a controlled sequential
  simulator.

**Fix path.** ✅ Implemented: replaced flat-sequence collection with
`build_lock_edges` held-set analysis. `gap_s19_*` now calls
`assert_rejected("S-19")`.

---

### L21 — S-20: ISR priority-inversion check does not follow helper calls ✅ Fixed 2026-07-12

**Fix applied.** `collect_locked_mutexes` / `collect_locked_mutexes_stmts` /
`collect_locked_mutexes_expr` now accept `fn_map` and `visiting` parameters.
When a call to a user-defined (non-extern) function is encountered, the
function's body is recursively walked to collect transitively acquired mutexes.
Cycle guard via `visiting` prevents infinite recursion. The `gap_s20_*`
adversarial test now asserts rejection.

**Original scope.** `enforce_isr_preemption` (`src/safety.rs`) called
`collect_locked_mutexes` on each ISR's body. A `mutex_lock` in the ISR
body was attributed to that ISR's lock set. A `mutex_lock` inside a
helper function called by the ISR was **not** attributed to the ISR.

Two ISRs at different priorities sharing a mutex through a common helper
will not trigger the `[S-20]` warning.

**Evidence.** `gap_s20_isr_mutex_through_helper_not_detected` in
`tests/safety_adversarial.rs`.

**Certification impact.** Same disclosure path as L20. Additionally:

- **Preferred mitigation**: use `Atomic<T>` for any resource shared
  across ISR priority levels. Atomics are immune to priority inversion
  and the compiler enforces the no-mutex requirement.
- **Secondary mitigation**: flatten mutex acquisition into the ISR body
  directly. The existing direct-body check then catches ordering issues.

**Fix path.** ✅ Implemented: `collect_locked_mutexes` now follows calls
transitively. `gap_s20_*` now calls `assert_rejected("S-20")`.

---

### L22 — MISRA 13.2: eval-order check fires for adjacent duplicate args only ✅ Fixed 2026-07-12

**Fix applied.** `check_eval_order_expr` (`src/safety.rs`) now uses
`seen.remove()` instead of `seen.get()`, so any second occurrence of a
variable in the same call's argument list fires the diagnostic regardless
of whether the two occurrences are adjacent. The `gap_misra_13_2_*`
adversarial test now asserts rejection.

**Original scope.** `enforce_misra_eval_order` (`src/safety.rs`) flagged a
variable that appears in two *consecutive* argument positions
(positions `k` and `k+1`) of the same call. A variable in positions
`0` and `2` with an unrelated argument in between was not flagged.

**Severity.** MISRA C 2012 Rule 13.2 is an **Advisory** rule. Per the
MISRA compliance process, advisory violations may be documented as
approved deviations in the project's MISRA Compliance Matrix without
requiring a code change.

**Evidence.** `gap_misra_13_2_non_adjacent_duplicate_not_detected` in
`tests/safety_adversarial.rs`.

**Practical risk.** The undetected pattern (`foo(x, y, x)`) is unusual
in production code and is almost always caught during review. The
checked pattern (adjacent: `foo(x, x)`) covers the most common and
most dangerous real-world occurrence.

**Fix path.** ✅ Implemented: `seen.remove()` replaces `seen.get()`;
adjacency guard removed. `gap_misra_13_2_*` now calls
`assert_rejected("MISRA 13.2")`.

---

## Module system limitations

### L23 -- `pub(kosh)` is enforced for external Kosh-package access; same-project sibling-module access now works ✅ Fixed 2026-07-24 (phase 2)

**Phase 1 (2026-07-22).** `pub(kosh)` restricts access from outside its
declaring module: `flatten_modules_in_program` (`checker.rs`) mangles a
`pub(kosh)` item to a third form (`<mod>__kosh__<name>`) distinct from both
plain `pub` (`<mod>__<name>`) and private (`<mod>__priv__<name>`) -- the
same trick private items already used to become unreachable via any
externally-written qualified path, one tier up. Verified directly: a
`[deps]` package exposing `pub(kosh) fn internal_helper(...)`, called as
`pkgname::internal_helper(...)` from a separate consumer project, is
rejected with `function 'pkgname::internal_helper' is pub(kosh) -- visible
only within its own package, not to external consumers`.

**Phase 2 (2026-07-24).** Phase 1's mangled-name approach couldn't yet
distinguish "a different, unrelated kosh calling in" from "a sibling
module in the *same* project calling across module boundaries" -- both
arrived as an identical, already-parser-mangled `mod__name` string with no
caller-identity information attached. So the tutorial's own worked example
(`tutorials/src/beginner/09a_modules_primer.md`) -- `module report`
reaching into a sibling `module stats`'s `pub(kosh) fn sum_all` via
`stats::sum_all(...)` -- was also rejected, stricter than the intended
design (only a *different kosh* consuming `stats` as a `[deps]` package
should be rejected).

Fixed by threading `kosh_boundary_names` (the set of top-level modules
that are wrapped `[deps]` packages -- see `wrap_deps_into_combined` in
`lib.rs`) into `checker::check`/`check_library` and on into
`flatten_modules_in_program`. A new post-flatten pass walks every
function/impl-method/methods-block body: for an unresolved `mod__item`
reference that matches a registered `mod__kosh__item`, it rewrites the
reference to the real mangled name whenever *neither* the calling item's
nor the target's top-level module is a `[deps]` boundary module (both are
part of "the current project," whatever its internal module layout).
External-boundary access is untouched -- it still falls through to the
existing `lookup_kosh_item` rejection diagnostic. Verified directly: the
`report`/`stats` sibling-module fixture above now compiles and runs; a
`[deps]`-wrapped module's `pub(kosh)` item reached via a qualified path
from outside that package is still rejected (`checker.rs`/`lib.rs` test
suite: `pub_kosh_qualifier_allows_same_project_sibling_module_access`,
`pub_kosh_qualifier_still_rejects_external_kosh_boundary_access`,
`pub_kosh_qualifier_allows_qualified_access_from_same_project`).

No remaining workaround needed -- `pub(kosh)` can now be used for its
intended purpose: package-internal helpers shared across your own
project's modules, hidden from `[deps]` consumers.

### L24 -- `parallel for`'s Windows thread count is fixed at build time, not run time

On Windows, `parallel for`'s worker-thread count is decided by
the *machine that ran `vanic build`* and baked into the generated
binary as a constant -- not decided by the machine that later
runs the binary. Build on an 8-core Windows box, ship the binary
to a 32-core one, and it still only ever spawns 8 worker threads
until you rebuild there (or set an env var at build time -- see
below).

**Where it lives**: `backend_llvm.rs`'s Win32 `parallel for`
dispatch (`host_uses_win32_threading()`, gated purely on
`cfg!(target_os = "windows")` -- i.e. the OS the *compiler itself*
is running on, not a `--target` triple). Thread count `N` is
resolved once, at codegen time, from `OMP_NUM_THREADS` if that
env var is set when `vanic build`/`vanic run` executes, else from
`std::thread::available_parallelism()` -- **read on the build
host**, then written into the LLVM IR as a literal integer (an
`alloca [N x {...}]` sized array of per-thread argument structs).
The comment at the call site is explicit about the tradeoff: this
was a deliberate choice to avoid an `@getenv` call in the
generated IR, at the cost of the count no longer being resolvable
at the eventual run site.

**Why only Windows**: the non-Windows path emits a portable
`call void @GOMP_parallel(...)` (GNU OpenMP), and libgomp
resolves its own thread count **at run time** on whichever
machine executes the binary -- respecting `OMP_NUM_THREADS` at run
time, or `sysconf(_SC_NPROCESSORS_ONLN)`-style queries otherwise.
Linux/macOS binaries built with `parallel for` already scale
correctly to the running machine's core count; only the Win32
CreateThread-based dispatch path has this gap, because it hand-
rolls its own thread pool instead of delegating to an OpenMP
runtime.

**Workaround**: set `OMP_NUM_THREADS` to the *target* machine's
core count before running `vanic build`, or simply rebuild on the
machine (or a same-core-count machine) the binary will actually
run on.

**Fix path**: replace the compile-time-resolved constant with a
runtime call to a Windows API that returns the live processor
count -- `GetActiveProcessorCount(ALL_PROCESSOR_GROUPS)` (or
`GetSystemInfo`/`GetNativeSystemInfo` for the simpler, single-
processor-group case) -- emitted as an LLVM `declare` + `call`,
mirroring how `GOMP_parallel` is already just an external call on
the non-Windows path. Not started; scoped but untouched.

### L25 -- Windows: `print`/`f64_to_str` scientific-notation exponent width differs between the C and LLVM backends ✅ Fixed 2026-07-25

`print`ing (or `f64_to_str`-converting) an `f64` whose magnitude is
large/small enough that the default `%g` formatting switches to
scientific notation produces **different text on the two backends,
on Windows, for the identical program and value**:

```
$ vanic run f.vani --backend=c     # prints: 1e+06
$ vanic run f.vani                 # prints: 1e+006   (LLVM/JIT)
```

Verified directly across several magnitudes (`1000000.0`,
`12345678.9`, `123456789.123456`, `0.0000001234`) -- the C backend
always emits a 2-digit exponent, the LLVM backend always emits a
3-digit exponent, on the same Windows host, same `snprintf`-based
formatting call in both cases.

**Root cause**: MinGW's `<stdio.h>` macro-redirects `printf`/
`vsnprintf` in *C source* to its own ANSI/C99-compliant
`__mingw_printf`/`__mingw_vsnprintf` (statically linked from
`libmingwex.a`) -- a preprocessor-level trick that only applies when
compiling actual C source, which is why the C backend gets the
2-digit convention for free. Hand-emitted LLVM IR has no
preprocessor: `backend_llvm.rs`'s raw `declare i32 @printf(...)` /
`@vsnprintf(...)` linked straight to msvcrt.dll's legacy, non-C99
formatter instead (confirmed via `objdump -p` on the built binary --
it imported `_vsnprintf` from `msvcrt.dll` directly, not the ANSI
version). This reproduced identically under both `vanic run` (JIT)
and `vanic build` (AOT), not just the JIT path.

**Impact**: cosmetic (both strings parse back to the same `f64`
value with `parse_float`), but it breaks any golden-output test
that diffs `print` output for large/small `f64` values across
backends on Windows, and it means a value's printed form isn't
fully portable across `vanic run` vs. `vanic run --backend=c`.
Not observed on Linux/macOS (both backends link the same glibc/libc
`printf` there) -- this is Windows-only, and only for values that
actually hit scientific notation (`%g`'s default precision is 6
significant digits; anything with an exponent magnitude below that
stays in fixed notation on both backends, no divergence).

**Workaround** (no longer needed for this bug specifically, still
true on its own merits): `f64_to_str_fixed(x, decimals)` uses fixed
notation (`%.*f`), which never switches to scientific notation, so it
was never affected by this gap either way. See [Beginner 6 --
Strings](../tutorials/src/beginner/06_strings.md) for the full
`f64_to_str` vs. `f64_to_str_fixed` caveats.

**Fix**: both LLVM backends' Windows-only preamble shims now declare
and route `printf`/`snprintf`/`dprintf` through `__mingw_vprintf`/
`__mingw_vsnprintf` instead of the raw msvcrt-resolving externs. That
alone fixed `vanic build` (AOT linking resolves `__mingw_*` from
`libmingwex.a` normally) but broke `vanic run`: `lli`'s JIT symbol
resolver only sees symbols exported from a *loaded DLL*, and those
two functions live in a static archive, never loaded as one. Fixed
the same way MATH-1 fixed the equivalent JIT/AOT split for
`sort_runtime.c` -- a tiny shim (`mingw_ansi_stdio_shim.c` + a `.def`
file force-linking and re-exporting the two symbols under their real
names) compiled once per process into a real DLL and `-load`ed into
`lli`. Full writeup, including a regression this uncovered in 20
pre-existing `lli_*` tests, in `docs/TODO_CURRENT.md`'s BUG-5 / L25
entry.

### L26 -- Vec/array indexing inside a loop body never elides its bounds check, even when provably safe

Prior to 2026-08-12, the SMT elision pass would sometimes elide the
runtime bounds check on `xs[i]` when `i` was a loop's own induction
variable and the index was provably in range for every iteration
(e.g. `for i from 0 to len(xs) { xs[i] }`). As of the BUG-181 fix,
**no index expression inside any loop body (`while`, `for`,
`for..in`) is ever elided, regardless of whether it's actually
safe** -- the runtime `if (i >= len(xs)) abort();` guard is always
present in the emitted code for these sites now.

**Why**: the elision pass reasons from `smt_facts`, a running list
of "facts assumed true at this point," which is deliberately allowed
to go stale across loop iterations for an unrelated reason (loop-
invariant-preservation checking needs the pre-iteration fact set
intact). BUG-181 found that this staleness let the pass "prove" an
index in-bounds using a fact that was only true on the loop's first
iteration (e.g. `j == 0`), producing an unconditional out-of-bounds
memory access -- a real memory-safety hole (SIGSEGV on the C
backend), not a false-positive rejection. The fix was a blanket
`if inside_loop { return; }` guard on the Index-elision arm,
matching the guard the arithmetic-overflow-elision arm already had
(added earlier for BUG-127). This trades a narrow, previously-real
performance optimization for soundness: the fix cannot distinguish
a `for` loop's compiler-synthesized, always-safe induction variable
from an arbitrary hand-mutated `while`-loop variable, so it
conservatively disables elision for both.

**Impact**: performance only, not correctness -- code that indexes a
`Vec`/array inside a loop keeps its runtime bounds check where it
used to be optimized away. Measured cost: bounds checks alone can be
5-15% overhead on numerical code (see [Intermediate 12b -- Compile
time vs runtime primer](../tutorials/src/intermediate/12b_compile_time_vs_runtime_primer.md)).
Code outside loop bodies (straight-line accesses guarded by a
`requires` clause, as in [Intermediate 10b](../tutorials/src/intermediate/10b_runtime_errors_primer.md)'s
`sum_first_three` example) is unaffected -- this limitation is
specific to accesses lexically inside a loop body.

**Workaround**: none that recovers the optimization soundly today.
A future loop-invariant-aware elision pass -- one that reasons about
per-iteration induction-variable bounds explicitly (the way
`verify_loop_invariants_with_havoc` already does for invariant
preservation) rather than reusing possibly-stale `smt_facts` -- could
recover this class of elision without reintroducing BUG-181's hole.
Filed as part of the round-12 audit focus; see
`docs/BUG_PATTERN_AUDIT_TODO_12.md`.

### L27 -- `vanic run`'s LLVM JIT path skips the `opt` optimizer, unlike `vanic build`/`--backend=c`

Found via `tools/localfuzz` (2026-08-12): a loop counter fuzzer-mutated
to start at `i64::MIN` (needing ~9.2 quintillion increments to reach
the loop's real exit condition) completed **instantly** under `vanic
build` (AOT) and `vanic run --backend=c`, but hung indefinitely under
plain `vanic run` (the default LLVM JIT path) -- confirmed directly,
not just observed:

```
$ timeout 5 vanic run f.vani --backend=c   # exits 0 instantly
$ timeout 5 vanic build f.vani -o f && ./f  # exits 0 instantly
$ timeout 5 vanic run f.vani                # times out (exit 124)
```

**Root cause**: `vanic build`'s LLVM pipeline runs `opt -O3` on the
generated `.ll` before handing it to `llc` (see `src/main.rs`'s AOT
path, and the C backend gets the equivalent via `cc -O2`/`-O3`).
`opt -O3` includes scalar-evolution / induction-variable analysis that
can collapse a simple counting loop like `while (n < LIMIT) { ...
n = n + 1; }` into a near-constant-time computation when the loop body
has no other observable side effect until it exits -- which is exactly
this shape. `vanic run`'s JIT path, by contrast, writes the raw,
**unoptimized** `.ll` straight to `lli` with no `opt` pass at all. For
almost all real programs this difference is invisible (`lli`'s own
MCJIT still runs at native speed per instruction); it only becomes
visible for a loop shape whose *iteration count* an optimizer can
collapse away entirely -- there, the JIT genuinely executes every one
of billions/quintillions of iterations one at a time, with no
shortcut, and can appear to hang forever on a program that finishes
instantly any other way.

**Impact**: performance only, not correctness -- given enough time,
`vanic run`'s JIT would produce the same answer. In practice this
means a loop with an accidentally-enormous range (a real off-by-one
bug in user code, not just a fuzzer artifact) can look like a hang
under plain `vanic run` while the exact same program runs instantly
under `--backend=c` or `vanic build`, which is a confusing signal when
debugging -- "which backend I used" shouldn't determine whether a
program appears to hang.

**Workaround**: use `vanic build` (or `vanic run --backend=c`) instead
of the default JIT path when debugging a loop that seems to hang, or
if you suspect an accidentally-huge range. `VANIC_NO_VERIFY=1` does
not help here (this isn't an SMT-verification cost).

**Fix**: not applied. Adding an `opt -O3` pass to the JIT path is a
real design trade-off, not a clear-cut bug fix -- it would add a
non-trivial startup-latency cost to *every* `vanic run` invocation
(including the common case of quickly re-running a small test/example
script, which is the JIT path's whole reason to exist) to fix a rare
pathological-loop case that `vanic build`/`--backend=c` already handle
correctly. Left as a documented, understood limitation rather than
changed unilaterally.

### L28 -- `as i64` (and other float-to-int casts) is unchecked, real UB when the value doesn't fit ✅ Fixed 2026-08-16

Found via `tools/localfuzz` (2026-08-12): a fuzzer-mutated program
computed an `f32` value far outside `i64`'s representable range, then
cast it with `as i64`. Both backends "succeed" (no panic, no
diagnostic on either side) but disagree on the process exit code --
confirmed with a minimal 4-line repro, independent of the SIMD context
the original finding used:

```vani
fn main() -> i64 {
  let s: f32 = 3.0 + 9223372036854775807;
  return s as i64;
}
```

```
$ vanic run f.vani               # exits 0
$ vanic run f.vani --backend=c   # exits 255
```

**Root cause**: `s` (an `f32`) ends up holding a value around 2^63 --
`3.0 + 9223372036854775807` (`i64::MAX`) gets promoted through
`double` (which can represent that magnitude, if not exactly) and then
narrowed to `f32` for the `let s: f32 = ...` assignment. `s as i64`
then converts a float that's at or beyond `i64::MAX + 1` back to
`int64_t` -- the C standard states this is **undefined behavior** when
the value doesn't fit the target integer type, and the C backend emits
a bare `(int64_t)(...)` cast with no range check, so the compiled
program inherits that UB directly: on this machine, GCC's
`cvttss2si` produces `INT64_MIN` as the "integer indefinite" result,
which further truncates through `main`'s `int64_t -> int` return-value
narrowing to produce exit code 255. LLVM's `fptosi` instruction is
*also* documented as producing a poison value for an out-of-range
input -- not the same guaranteed behavior as C, just a different
flavor of undefined, which is exactly why the two backends disagree
rather than agreeing on some other wrong-but-consistent answer.

**Impact**: correctness, in the narrow sense that a float-to-int `as`
cast is unchecked in both backends -- this cuts against the language's
own stated design contract ("hosted programs are safe by construction
-- no segfault surface"; see the `unsafe(reason = "...")` gating
diagnostic's wording, which makes exactly this promise for the rest of
the language). Integer arithmetic overflow, division/shift, and
bounds accesses are all checked and produce a well-defined panic in
v1; float-to-int narrowing casts are the one numeric-conversion
category that currently isn't. Practically rare -- it requires a
float value that's already outside the target integer range, not
just precision loss from an in-range float -- but "rare" isn't "safe,"
and the divergent exit code means a program relying on this exit code
(e.g. shell scripting around `vanic run` vs `vanic build`) gets a
different answer depending purely on which backend ran it.

**Workaround**: none needed for well-behaved programs -- keep
float-to-int casts within the target type's representable range (the
same discipline `as i32`/`as i8` truncating casts already require of
callers). If a value's range genuinely can't be bounded statically,
clamp it explicitly before casting (e.g. `f64_clamp(x, i64::MIN as
f64, i64::MAX as f64) as i64`) rather than relying on the cast itself
to do anything sensible.

**Fix (2026-08-16)**: chose the runtime-range-check option, not
saturating casts -- consistent with how every other checked operation
in v1 behaves (overflow, division/shift-by-zero, out-of-bounds access:
all trap with a defined message and `exit(3)`, none silently clamp).
Every float-to-int `as` cast (`f32`/`f64` -> any of `i8/i16/i32/i64/
u8/u16/u32/u64`) now emits a range check comparing the source value
against the target type's min/max, expressed as exact power-of-two
double-literal bounds (`-128.0`/`128.0` for `i8`, ...,
`-9223372036854775808.0`/`9223372036854775808.0` for `i64`,
`0.0`/`18446744073709551616.0` for `u64`, etc.) so the bounds
themselves are exactly representable and never round the wrong way.
The upper bound uses a strict `<` (not `<=`), and because any
floating-point comparison against NaN is always false, `NaN as i64`
correctly traps too with no separate `isnan` check needed. On
out-of-range input the program traps with `float-to-int cast out of
range` and exits 3 -- on both backends, closing the exit-code
divergence this entry originally documented (255 vs. 0).

Implemented identically in effect but idiomatically per backend,
since each of the 4 emission paths (tree-C, SSA-C, tree-LLVM,
SSA-LLVM) already had its own established convention for checked
operations and this fix matches each rather than introducing a new,
inconsistent pattern: tree-C emits a named, reusable
`intent_check_float_to_<ty>` helper function (only for the type
combinations actually used in the program, `src/backend_c.rs`); SSA-C
inlines the check directly at each cast site as a C `if` statement,
matching how SSA-C already inlines every other checked op
(`src/ssa_backend_c.rs`); SSA-LLVM pre-scans the whole module for the
`(source, target)` type pairs actually needed and emits one
`alwaysinline` LLVM function per pair (`src/ssa_backend_llvm.rs`);
tree-LLVM inlines the check via fresh basic blocks at each cast site,
since the tree-walking emitter has no whole-module pre-scan pass
(`src/backend_llvm.rs`). `InstrKind::Cast` gained a `checked: bool`
field in `src/ssa.rs` (mirroring the existing `InstrKind::Binary`
precedent), set to true only when the source is `F32`/`F64` and the
target is an integer type -- integer-to-integer and int-to-float
casts are unaffected. Verified across in-range, out-of-range,
negative-to-unsigned, and NaN cases on both backends, plus the full
`cargo test --release --workspace` / `backend_crosscheck.py` /
`leak_sweep.py` battery against the whole example corpus with zero
regressions.

### L29 -- `for i from lo to hi` is ascending-only ✅ Partially resolved 2026-08-13 (`downto` added; `step`/stride-N still unsupported)

`for VAR from START to END { ... }` only ever counted up by 1, over
the half-open range `START, START+1, ..., END-1`
(`src/parser.rs::parse_for_stmt_inner`, ~line 3957). There was no
`downto` keyword, no `step`/`by` clause, and no direction inference
from `START` vs `END`. If `START >= END`, the range was (and still
is) legitimately empty and the loop body runs **zero times**, with
no diagnostic:

```vani
fn main() -> i64 {
  let count: i64 = 0;
  for i from 5 to 0 {
    count = count + 1;
  }
  print "count =", count;   // count = 0
  return 0;
}
```

This is existing, deliberate, regression-tested behavior, not a bug
-- see `for_loop_reverse_range_compiles` and `for_loop_empty_range_compiles`
in `src/lib.rs` (~lines 4797-4823). It's listed here because the
surface reads like it should count down (`for i from 5 to 0` looks
like English for "5 down to 0"), and the compiler gives no error or
warning when that intuition is wrong -- the same class of silent
footgun as the half-open upper bound itself, just easier to trip on.

**Fix (2026-08-13)**: added `downto` as the descending counterpart of
`to`, step 1, same half-open convention (`for i from 5 downto 0`
walks `5, 4, 3, 2, 1`, excluding `0`, mirroring how `to` excludes its
own upper bound). Shipped English-only first, then extended to every
dialect that already has a native `to`/`until` spelling -- all 62,
via the same-day BUG-170-style keyword-parity sweep (each dialect's
`downto` word is a new coinage compounding its existing `to` word
with its word for "down"/"below"; see
`docs/archive/grammar_review_queue.md`'s "downto keyword-parity
sweep" section for the full per-dialect table and confidence ratings
-- several are flagged **Low** confidence pending native-speaker
review, same languages/scripts as the BUG-171 review queue). A
mechanical parity test (`downto_keyword_parity_all_62_dialects` in
`src/lib.rs`) asserts every dialect with a `to` spelling also has a
`downto` spelling; full end-to-end compiles additionally cover
Devanagari (both English and SOV word order), Japanese, and Russian.
`parallel for` doesn't support `downto` yet either (rejected at parse
time with a clear diagnostic) -- only sequential `for`. The fix
threads a new `descending: bool`
field through `Stmt::For`/`TypedStmt::For` (extending the existing
node rather than a full while-loop desugaring, to keep the loop
variable's existing dedicated scoping and avoid re-deriving label
attachment for `break`/`continue`) and updates every site that baked
in "ascending" arithmetic: the SMT bound-fact mirror (`var <= start
&& var > end` instead of `var >= start && var < end`), the implicit
per-iteration reassignment tracking (`var - 1` instead of `var + 1`),
both backends' loop codegen (`src/backend_c.rs`: `>` / `--`;
`src/backend_llvm.rs`: `sgt`/`ugt` / `sub`), WCET trip-count
computation (`src/safety.rs`), and the AST pretty-printer
(`src/format.rs`). Descending loops are routed through the tree
backends rather than the SSA pass (`src/main.rs::stmt_ssa_supported`
rejects `descending` there), matching the existing pattern for other
SSA-unsupported constructs -- ascending loops are unaffected.
Regression tests: `for_loop_downto_parses_and_compiles`,
`for_loop_downto_empty_when_start_le_end_compiles`,
`for_loop_downto_rejected_on_parallel_for` in `src/lib.rs`, plus an
end-to-end stdout check on both backends in `tests/run_end_to_end.rs`
against `examples/language/english/for_loop_downto.vani`.

**Still open**: `step`/`by` (a stride other than 1, ascending or
descending) is not implemented -- use a `while` loop with manual
arithmetic:

```vani
let i: i64 = 10;
while i <= 20 {
  print i;
  i = i + 3;       // step of 3
}
```

See [Beginner 5's "Counting down with `downto`, or stepping by more
than 1"](https://github.com/enthusiasticgeek/vani-compiler/blob/main/tutorials/src/beginner/05_loops.md#counting-down-with-downto-or-stepping-by-more-than-1)
for the tutorial-side coverage.

### L30 -- `tcp_recv`'s received bytes are not inspectable from vani code ✅ Fixed 2026-08-16

`tcp_recv(fd, max) -> i64` fills an opaque, thread-local 4KB scratch
buffer and returns the byte count -- but there is no builtin to read
the buffer's actual *contents* from vani source. The only documented
follow-up is `tcp_send_buf`, which echoes the raw bytes back out over
a socket (`tcp_recv` then `tcp_send_buf`, the pattern every existing
`tcp_*.vani` example uses). There is no `tcp_buf_byte_at` or
equivalent, despite `src/checker.rs`'s own doc comment for `tcp_recv`
mentioning one ("Inspect via `tcp_buf_byte_at` follow-up builtins") --
that builtin was never actually implemented; the comment is aspirational,
not current behavior.

Found while building
[`examples/language/english/tic_tac_toe_networked_timed.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/english/tic_tac_toe_networked_timed.vani)
(see [Advanced 3c](https://github.com/enthusiasticgeek/vani-compiler/blob/main/tutorials/src/advanced/03c_timed_tic_tac_toe_capstone.md)),
which needed to send an actual move number (0-8) over a TCP
connection and read it back on the other side. Worked around by
encoding the value as **message length** instead of content (send a
filler string of length `value + 1`, decode via `tcp_recv`'s own
byte-count return), which stays entirely within what v1 already
supports -- but that's a workaround for a real gap, not a general
solution (it can't carry more than one small integer per message,
and doesn't scale to arbitrary payloads/text).

**Fix (2026-08-16)**: added `tcp_buf_byte_at(i: i64) -> i64`, mirroring
`str_byte_at`'s exact shape and contract -- returns the byte at index
`i` of the shared thread-local `intent_tcp_buf` scratch buffer
(0-255), no bounds check, same "caller's responsibility" convention
`str_byte_at` already uses. Registered in `src/checker.rs`'s builtin
table and routed through the existing `check_tcp_builtin` dispatch
(1 argument, `i64` return, same validation path as `tcp_close`/
`tcp_accept`). Implemented on tree-C (`src/backend_c.rs`, a plain
`intent_tcp_buf[i]` array index) and tree-LLVM (`src/backend_llvm.rs`,
`getelementptr` + `load i8` + `zext` to `i64`) only -- confirmed
empirically (by inspecting generated C's variable-naming convention
across several `tcp_recv`-shaped test programs) that every `tcp_*`
builtin already falls back to the tree backends in practice even
though `tcp_recv` is nominally SSA-eligible, so the SSA-C/SSA-LLVM
paths were correctly left untouched rather than speculatively
implemented. Works only against the blocking v1.6 `tcp_recv` family's
buffer, as originally scoped -- `tcp_recv_nb`/`io_recv_async`'s
Arc 8 non-blocking buffers are a separate, still-open question if a
future need arises. Verified end-to-end (buffer written by
`tcp_recv`, read back via `tcp_buf_byte_at` against known byte
values) on both backends, plus the same zero-regression full
verification battery as L28.

### L31 -- `detach`'d tasks still running when `main` returns can crash under `vanic run` (LLVM `lli` JIT only)

Found via localfuzz (2026-08-15): a mutated
`examples/language/english/detach_heartbeat.vani` (the detached
heartbeat's loop counter changed to start near `i64::MIN`, so it's
still running when `main` returns almost immediately) segfaulted
(`rc=139`) under `vanic run` (LLVM, the default). Reproduced reliably
(3/3) with a minimal, non-mutated repro too: `detach` a task with a
plain, long-running counting loop (no huge/adversarial values needed
-- 2 billion iterations of `i = i + 1` is enough), `detach` it, and
return from `main` immediately. `lli` prints its own "PLEASE submit a
bug report to https://github.com/llvm/llvm-project/issues/" crash
banner -- the same misleading-JIT-crash signature already documented
for BUG-106/108/110/113/115/117/120/162 (a real vāṇी runtime trap
that `lli`'s JIT engine reports as if it were an LLVM internal
crash), except this time there's no controlled trap underneath it at
all -- this is a genuine segfault.

**Root-caused, not vāṇी's own bug**: the identical program, built
with `vanic build` (real AOT native compilation, no `lli` involved)
and then run directly, completes cleanly and correctly every time --
confirmed 3/3 runs, `exit 0`, both the background heartbeat's own
prints (when it finishes fast enough to) and `main`'s own output
behave exactly as documented. `vanic run` (LLVM, no `--backend`)
literally shells out to the external `lli` binary
(`src/main.rs`, `env::var("LLI").unwrap_or_else(|_| "lli".to_string())`)
to JIT-execute the emitted `.ll` -- vāṇी's own codegen is
byte-identical between the JIT and AOT paths, and AOT is proven
correct, so the bug lives entirely inside `lli`'s own JIT engine:
most likely, `lli` tears down (unmaps/frees) its JIT-compiled machine
code when the JIT'd `main` function returns, without any awareness
that a real OS-level pthread spawned via vāṇी's `task`/`detach`
runtime may still be executing machine code the JIT engine owns --
a classic "the CPU is still executing code whose memory was just
freed" segfault, entirely inside upstream LLVM's `lli` tool, outside
what vāṇी's own source can patch.

**Not fixed this pass** -- there's no vāṇी-side code change available
(the bug isn't in vāṇī's emitted IR or runtime C shims, confirmed by
AOT working). The honest mitigation is the documentation update this
pass DID make: `tutorials/src/advanced/03_concurrency.md`'s `detach`
section now calls out that a `detach`'d task still running when
`main` returns is only safe under `vanic build` (AOT) -- use `vanic
run --backend=c` or `vanic build` if a detached task's runtime might
outlive `main`, not the default LLVM `lli`-JIT path. A real fix would
mean either patching upstream `lli` (out of scope) or `vanic run`
itself explicitly draining/joining detached OS threads before
`lli`'s own process teardown runs, which would need to happen from
outside the JIT'd program (`vanic`'s own subprocess wrapper around
`lli`), not from generated IR -- worth a dedicated design pass if this
turns out to bite real (non-fuzzer-manufactured) programs.
