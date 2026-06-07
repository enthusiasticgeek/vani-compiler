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

### L1 — Enum destructure-bindings limited to Copy + OwnedStr payloads

Pattern-match arms can only bind enum variant payloads when the
payload is `Copy` (scalar) or `OwnedStr` (heap-string, exposed as
a `Str` view).

```vani
enum Node {
  Leaf(i64),         // ✅ binding works:  Node.Leaf(v) then ...
  Branch(Vec<i64>),  // ❌ Vec is affine; binding rejected
}
```

**Why**: lifetime tracking through enum-variant destructure
needs more work in the checker; v1 takes the conservative path.

**Workaround**: use a tagged struct (`kind: i64`) with parallel
fields, accessed via field-not-binding. See the
[Composite pattern example](../examples/language/english/design_patterns/structural/composite.vani).

**Fix queued in**: `TODO.md` (deferred — needs lifetime work
in `desugar_enum_match`).

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

### L3 — Pattern-match scrutinee must be by value

`match` expressions require the scrutinee to be an enum / integer
/ bool value, not a reference.

```vani
fn sum_node(n: Node) -> i64 {       // ✅
  return match n { ... };
}

fn sum_node(n: ref Node) -> i64 {   // ❌ — error: match scrutinee
  return match n { ... };           //    must be an enum/int/bool
}
```

**Why**: v1 lowering emits a direct tag-load on the scrutinee
operand; the through-reference path is a separate codegen lane
that hasn't been built.

**Workaround**: take the value by value when the type is `Copy`,
or pass the payload fields explicitly when the enum has affine
variants.

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

### L7 — `for VAR in ref self.field` parses as `ref self` then chokes

Inside a method body, you can't iterate over `self.field`
directly with a `ref` borrow at the for-loop head.

```vani
methods on Subject {
  fn publish(self: ref Subject, value: i64) -> i64 {
    for o in ref self.observers { ... }   // ❌ — parse error
  }
}
```

**Why**: the `for VAR in EXPR` parser only accepts simple var-
or `ref var` expressions as the iterable. A field access through
`self` isn't a recognized iterable expression at the parser
level.

**Workaround**: extract the iteration into a free function that
takes the field as a `ref Vec<T>` parameter. See the
[Observer](../examples/language/english/design_patterns/behavioral/observer.vani)
and [Mediator](../examples/language/english/design_patterns/behavioral/mediator.vani)
pattern examples.

---

## Backend-specific limitations

### L8 — C-codegen: `Vec<dyn Iface>` as a struct field with multiple dyn types

A program that has TWO or more distinct `dyn Iface` types AND
stores a `Vec<dyn IfaceA>` as a struct field hits a C-codegen
type-collision: the emitted `intent_vec_intent_dyn` typedef gets
re-used across both Iface types.

**Why**: the C backend's `Vec<dyn Iface>` bundle naming doesn't
include the Iface name in its struct typedef.

**Workaround**: pass `Vec<dyn Iface>` through a function
parameter instead of storing it as a struct field, OR limit the
program to one dyn-Iface type. See the
[Observer pattern example](../examples/language/english/design_patterns/behavioral/observer.vani).

**Fix queued in**: C-backend bundle-naming pass — include the
Iface name in the typedef. Tracked in `TODO.md` as
*"C codegen: Vec<dyn Iface> struct-field collision"*.

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

### L12 — SMT can't prove across function-call boundaries by default

```vani
prove some_fn(0) == 1;   // ❌ — SMT skipped: function call not supported
```

**Why**: the v1 SMT encoder only handles integer + bool
arithmetic + comparison. Function calls in a `prove` goal need
the callee's `ensures` clause to bridge.

**Workaround**: add `ensures` clauses to the callee, OR convert
the `prove` to a runtime `assert`. See
[`examples/language/sanskrit/pure_devanagari.vani`](../examples/language/sanskrit/pure_devanagari.vani)
for an example that uses `सिद्धम्` (assert) instead of `प्रमाण`
(prove) for results that depend on function calls.

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
