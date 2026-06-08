# vāṇी v1 — known limitations

> Single canonical catalog of v1 deviations from textbook
> behavior. Each entry: what it is, why it's that way today,
> the workaround currently in use, and a pointer to the place
> the user-visible workaround appears.

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

### L2 — No `Box<T>` / owning-interface-object pointer

vāṇी doesn't have a `Box<T>` (owning heap pointer to a sized
type). Owning a `dyn Iface` value inside a struct field is
therefore not supported.

```vani
struct Drawer { r: dyn Renderer }   // ❌ — needs Box<dyn Renderer>
```

**Why**: affine-ownership rules combined with the lack of an
explicit owning-pointer type would force lifetime-erased
storage. v1 deliberately omits it.

**Workaround**: use an integer discriminator + parallel fields,
or pass the dyn value through a function parameter instead of a
struct field. See the
[Bridge pattern example](../examples/language/english/design_patterns/structural/bridge.vani).

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

### L4 — `let` annotation cannot be a reference type

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

### L9 — LLVM backend: identifiers with non-ASCII chars require mangling

LLVM IR's bare-identifier grammar restricts characters to
printable ASCII. Devanagari function / struct names mangle to
`_uHHHH` (uppercase hex per codepoint) on emission.

**Why**: LLVM IR design choice, not a vāṇी limitation per se.

**Workaround**: shipped — `llvm_mangle_ident` handles this
transparently. The C backend uses UTF-8 directly. No user-
visible change.

---

## Platform / runtime limitations

### L10 — macOS + Windows runtime verification deferred

C backend ships with `#ifdef _WIN32` / `#elif defined(__APPLE__)`
branches for the Arc 8 I/O runtime helpers (epoll → kqueue / IOCP,
timerfd → pipe+pthread / `Sleep`, etc.). LLVM IR ships matching
emit paths.

**Why**: no Darwin or Windows host access at landing time. Linux
verification stays green; macOS + Windows branches exercise on
first build there.

**Workaround**: none needed for Linux users. macOS + Windows
users get full Arc 8 I/O via the C backend on first try; report
any kqueue / IOCP / winsock issues so the hot-spots in
[ARC8_V3_PLAN.md](../ARC8_V3_PLAN.md) Phase 5/6 get tuned.

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

### L13 — SOV reshape only for some constructs

vāṇी's SOV (Subject–Object–Verb) parser hooks cover 8 statement
shapes (`let` / `return` / `print` / `assert` / `prove` /
range-`for` / `if`/`else` / `while`). The remaining 4 — `fn`
declarations, `struct` / `enum` declarations, `match`-as-stmt —
stay keyword-first.

**Why**: Indo-Aryan grammar reads those constructs naturally
keyword-first (`यदि...तर्हि`, `मेल x { ... }`); forcing verb-
at-end would feel forced rather than natural.

**Workaround**: use the keyword-first form for those four
constructs. SOV-S2/S4/S5/S6 are documented as "declined as
design" in
[TODO.md §*Why some constructs stay keyword-first*](../TODO.md).

### L14 — Dialect-aware errors translate prefix only

When a file declares `// vani-lang: <dialect>`, error messages
render with localized labels (Sanskrit `त्रुटिः`, Hindi `त्रुटि`,
Marathi `चूक`) and a translated leading prefix for the most
common error families. The body of the error stays English so
search engines + existing docs still match.

**Why**: full body translation requires translating dynamic
content (paths, type names, variable names) — too noisy for
v1. The leading prefix gives the user a dialect-aware entry
point without making search worse.

**Workaround**: native-speaker linguists adding to the prefix
table in
[`src/diagnostic.rs:localize_message`](../src/diagnostic.rs).
Tracked in [`docs/grammar_review_queue.md`](grammar_review_queue.md).

---

## Adding to this catalog

When you hit a new v1 deviation:
1. Add an entry here with the L<N> label.
2. Cross-reference from any example file that exercises the
   workaround.
3. If the underlying bug has a fix path, link it in `TODO.md`.
4. If it's documented as "by design" (like L13), name the
   design rationale.
