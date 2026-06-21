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
> **At v0.1.5 (2026-06-21): 13 of 18 entries fully resolved; 1 partially
> resolved (L13); 4 remain open (L5, L6, L10-macOS, L14).**
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

**Remaining restrictions (queued follow-ups)**:
- **`Box<dyn Iface>`** — the original documented blocker
  (`struct Drawer { r: Box<dyn Renderer> }`) needs vtable
  plumbing through the heap allocation. Phase 3.
- **Box of affine inner type** (`Box<Vec<i64>>`, `Box<OwnedStr>`)
  — requires recursive drop walks. Queued.

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

### L9 — LLVM backend: identifiers with non-ASCII chars ✅ SHIPPED 2026-06-08

LLVM IR's bare-identifier grammar restricts characters to
printable ASCII. Devanagari/Bengali/Tamil/etc. function /
struct / local names mangle to `_uHHHH` (uppercase hex per
codepoint) on emission via `llvm_mangle_ident` (see
[src/backend_llvm.rs:239](../src/backend_llvm.rs)).

**Status**: Fully shipped. Verified 2026-06-08 against
[examples/language/sanskrit/pure_devanagari.vani](../examples/language/sanskrit/pure_devanagari.vani)
(function name `द्विपदगुणक`, locals `वामभाग`/`दक्षिणभाग`/`मूल्य`,
type name `पूर्णांक`) plus Marathi/Bengali examples — every
backend path that emits an LLVM identifier routes through
`llvm_mangle_ident`. No user-visible change; C backend uses
UTF-8 directly.

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
Tracked in [`docs/grammar_review_queue.md`](grammar_review_queue.md).

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

```vani
let f: FileHandle = file_open("/tmp/log.txt", "w");
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

**Remaining scope** (device I/O — UART / I2C / SPI / RS485): these
are kernel-ioctl-specific and remain a C-shim + FFI pattern by
design. The `struct termios` ABI is aggregate-by-value (rejected at
the v1 FFI boundary), so write a thin C shim and use `--link-with`.

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

| Feature | v1 status |
|---|---|
| `print` / `write` → stdout | ✅ Ships (always newline-terminated) |
| `eprint` / stderr output | ❌ No language surface |
| stdin / `read_line` | ❌ No language surface |
| Flat file I/O (`open`, `read`, `write`, `close`) | ❌ No language surface |
| Device I/O (RS232 / RS485 / UART / `ioctl`) | ❌ No language surface |
| `flush` / `setbuf` / unbuffered stdout | ❌ No language surface |

**Why**: stdout-only `print` covers the primary use case (programs that
compute and report). Full file I/O requires affine `FileHandle` types,
a `Result<T, E>` error surface, and per-platform device abstractions —
a dedicated Arc.

**Workaround — FFI**: declare `extern "C"` bindings to libc or POSIX
and use `--link-with` for any C shim layer.

```vani
// ── flat file (libc) ─────────────────────────────────────────────
extern "C" fn fopen(path: Str, mode: Str) -> i64;   // FILE* as i64
extern "C" fn fclose(fp: i64) -> i32;
extern "C" fn fputs_file(s: Str, fp: i64) -> i32;   // C: fputs
extern "C" fn fgets(buf: Str, n: i32, fp: i64) -> i64;
extern "C" fn fflush(fp: i64) -> i32;

fn write_log(path: Str, msg: Str) -> i64 {
  let fp: i64 = fopen(path, "a");
  if fp == 0 { return 0 - 1; }
  let _ = fputs_file(msg, fp);
  let _ = fflush(fp);
  let _ = fclose(fp);
  return 0;
}

// ── stdin ─────────────────────────────────────────────────────────
extern "C" fn getchar() -> i32;   // read one byte from stdin

// ── stderr ────────────────────────────────────────────────────────
extern "C" fn fputs_stderr(s: Str) -> i32;   // thin C wrapper: fputs(s, stderr)
```

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

## Adding to this catalog

When you hit a new v1 deviation:
1. Add an entry here with the L<N> label.
2. Cross-reference from any example file that exercises the
   workaround.
3. If the underlying bug has a fix path, link it in `TODO.md`.
4. If it's documented as "by design" (like L13), name the
   design rationale.
