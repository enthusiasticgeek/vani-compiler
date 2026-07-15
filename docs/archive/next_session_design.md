# Next-session design notes: liveness optimization + L4 (B)

> **Archive (2026-06-18):** All items in this document are fully
> shipped. Retained as a reference for the implementation
> decisions made. See [STATUS.md](../STATUS.md) for the full
> commit ledger and [TODO.md](../TODO.md) for the current open
> queue.

**Status (updated 2026-06-09 — ALL DONE):**
  - Item 1 — liveness optimization → ✅ SHIPPED (commit `2a693fb`)
  - Item 2 — L4 (B) Phase 1 → ✅ SHIPPED (commit `229852b`).
  - Item 2 — L4 (B) Phase 3 + scope-escape analyzer → ✅ SHIPPED
    (commit `3c14dce`).
  - Item 2 — L4 (B) Phase 4 (`Vec<ref T>`) → ✅ SHIPPED 2026-06-09.
    Per-shape Vec typedef + helpers on both backends; element_tag /
    vec_struct_tag handle Ref/RefMut; push-site scope-escape
    analyzer. Acceptance: `examples/vec_of_ref.vani` cross-backend
    parity-green and ASan-clean. **L4 (B) is now fully closed.**
    Only path-C (returning refs directly via lifetime elision)
    remains queued — separate research-scale arc.

Investigation 2026-06-08 (late session) showed:
- Phase 1 (let-binding refs) was safer than the design anticipated:
  every escape vector still had a type-level reject, so Phase 1
  shipped WITHOUT the analyzer.
- Phases 3+4 LIFT two of those receiver rejections (struct field +
  Vec element), so they DO need the analyzer the design originally
  anticipated for Phase 2.

The remainder of this doc retains the original investigation
notes for reference. The Item-1 + Item-2-Phase-1 sections are
"done" markers; the Phase 3+4 + analyzer sketch is still
load-bearing for the next session.

---

## Item 1 — v3.1 liveness optimization

**TLDR**: each `let` inside a v3.1 async fn body currently lands
in the synthesized Task struct as a field, regardless of whether
the local's lifetime crosses a suspend point. The optimization
detects "state-local" locals (declared + used entirely within
one state-machine arm) and emits them as poll-fn stack locals
instead of Task fields.

### Where to look

- **Validator** at [`src/parser.rs:6792`](../src/parser.rs#L6792)
  collects every top-level `Stmt::Let` into `locals: Vec<(String,
  Type, Span)>`. Every entry becomes a Task struct field.
- **Segment collector** at
  [`src/parser.rs:7636`](../src/parser.rs#L7636) emits each Let
  as a `Seg::NonSuspendLet { name, ty, expr, span }` into the
  per-state `state_bodies[K]`.
- **Codegen** at
  [`src/parser.rs:7890`](../src/parser.rs#L7890) per
  `Seg::NonSuspendLet` emits BOTH:
  1. A synth local: `let __v3_tmp_<name>: <ty> = <expr>;`
  2. A field assignment: `__t.<name> = __v3_tmp_<name>;`
- **Rename rewrite** at
  [`src/parser.rs:7401`](../src/parser.rs#L7401) puts every local
  + param name into a `HashSet` consulted by
  `rewrite_vars_to_fields` — every body-level Var read becomes
  `__t.<name>`.

### The analysis

After `state_bodies` is collected, walk it twice:

1. **Collect declarations**: for each `Seg::NonSuspendLet { name }`
   in state K, record `decl[name] = K`.
2. **Collect reads**: walk every Seg's contained Expr trees,
   collect `reads[name] = set<states K' that read name>`.

Then `name` is **state-local** iff `reads[name] ⊆ {decl[name]}`
(all reads are in the declaring state).

Subtleties:
- Reads in `Seg::Decision` branches stay in the same state IFF
  the branch body doesn't contain a `Seg::Jump` to a different
  state. Need to walk the decision's nested Seg lists.
- Reads in `Seg::Suspend` args ARE in the declaring state
  (since the suspend's emission happens at the END of the state
  arm). Treat as same-state.
- Reads in `Seg::Return` are in the declaring state's arm.

### Codegen changes

Three sites change:

1. **`rename` set** (line 7401): exclude state-local names so
   `rewrite_vars_to_fields` doesn't replace them with `__t.<name>`.

2. **Task struct fields** (line 7428): filter `locals` to drop
   state-local entries.

3. **`Seg::NonSuspendLet` codegen** (line 7890): when the local
   is state-local, emit only the Let (with `name` instead of the
   `__v3_tmp_<name>` synth name) and SKIP the FieldAssign.

   ```rust
   match seg {
       Seg::NonSuspendLet { name, ty, expr, span } => {
           let rewritten_expr = rewrite_vars_to_fields(expr, &rename, &t_param_name);
           if state_locals.contains(name) {
               // STATE-LOCAL: emit as a poll-fn stack let.
               then_body.push(Stmt::Let {
                   name: name.clone(),
                   annotation: Some(ty.clone()),
                   expr: rewritten_expr,
                   span: *span,
               });
           } else {
               // CROSS-STATE: existing two-step emission.
               let synth_local = format!("__v3_tmp_{}", name);
               then_body.push(Stmt::Let { ... });
               then_body.push(Stmt::FieldAssign { ... });
           }
       }
       // ...
   }
   ```

### Tests to ship

Pin these shapes in `src/lib.rs`:

1. **Pure state-local**: a local declared + read only inside one
   state, never crossing a suspend. Assert the Task struct
   doesn't carry the field.
2. **Cross-state**: a local read after a suspend. Assert the
   Task struct DOES carry the field.
3. **Mixed**: an async fn with both kinds, verify partitioning.
4. **Decision-with-suspend**: a local read inside an if-branch
   that contains its own suspend (cross-state via the decision).
5. **Loop-local**: a local declared inside a `while` body whose
   loop body has a suspend.

Snapshot-style: walk `checked.ir.structs` and assert the Task
struct's field count matches the cross-state subset.

### Acceptance criteria

- All 2030 existing lib tests pass.
- New tests pin the 5 shapes above.
- e2e parity stays green (codegen behavior unchanged for
  cross-state locals; state-locals just emit differently with
  identical semantics).
- ASan-clean on `echo_pool.vani` (the most likely place to
  surface ownership bugs).

### Estimated effort

6-10h focused. Bug risk is real (liveness corner cases) but
bounded.

---

## Item 2 — L4 (B) lexical-scope-only refs

**TLDR**: lift the second-class-refs restriction so refs can live
in `let` bindings, user struct fields, and return types — with the
checker enforcing they don't escape their declaration scope. No
explicit lifetime variables (like Rust's `'a`); the rule is
purely lexical.

### Where to look

- **Let-annotation reject** at
  [`src/checker.rs:8399`](../src/checker.rs#L8399):
  `validate_no_ref(annotation, *span, "let annotation", diagnostics)`
  → diagnostic "references cannot be stored in 'let' bindings".
- **Struct-field reject** at
  [`src/checker.rs:622`](../src/checker.rs#L622):
  `if !is_v31_synth && (field.ty.is_ref() || field.ty.is_ref_mut())`
  → "struct field 'X::Y' cannot be a reference".
  (The `is_v31_synth` carve-out shipped 2026-06-08 for Task__X
  structs; user structs still reject.)
- **Return-type reject**: check
  [`src/checker.rs:1135`](../src/checker.rs#L1135) +
  `Stmt::Return` handler. May or may not have a ref-rejection
  today.
- **Vec element reject** at
  [`src/checker.rs:10699`](../src/checker.rs#L10699):
  "Vec element type cannot be a reference, got `{}`".

### The lift

Four phases, each shippable as a separate commit:

#### Phase 1 — Allow ref types in let annotations

Lift `validate_no_ref` for `let` annotations. The annotation
already type-checks (parser produces `Type::Ref(Box::new(_))`
from `ref T`). Backend lowering: refs are already pointers at
both backends, so a `let r: ref Foo = ref some_foo;` just stores
the pointer.

The validator needs to ensure the RHS is a `ref EXPR` (or another
ref-typed value). The let-binding holds a borrow of `some_foo`;
the binding's lifetime must not exceed `some_foo`'s.

#### Phase 2 — Scope-escape analyzer

A new pass over function bodies that, for each ref-binding,
tracks:
- The set of bindings the ref's source could refer to (alias set).
- Whether the alias set's underlying values are all live at the
  ref-binding's scope-exit.

Scope-escape error fires when:
1. A ref binding is RETURNED from the function (its source goes
   out of scope on return).
2. A ref binding is stored in a struct field whose lifetime
   exceeds the source.
3. A ref binding is captured by a closure that outlives the
   source.
4. A ref binding is stored in a Vec/HashMap/etc. (the container
   may outlive the source).

The analyzer can be conservative: when in doubt, reject. The
simplest version: a let-bound ref can only be used in argument
position to function calls. Returning it, storing it, capturing
it → all errors.

This is essentially Rust's "no NLL, lexical lifetimes only"
borrow checker — much simpler than real lifetime variables.

#### Phase 3 — Allow ref types in user struct fields

Lift the rejection at checker.rs:622 for user structs (already
allowed for synthesized Task__X). The same scope-escape rules
apply: a struct with a ref field can only be constructed with
ref-args whose source-bindings outlive the struct's binding.

Per-language complication: backends already handle ref struct
fields (see the L4 partial lift work — `format_declarator` and
`emit_struct_bundle` both handle `Type::Ref(_)` field types).
This phase mostly removes the gate.

#### Phase 4 — Allow ref types in Vec elements

Lift the rejection at checker.rs:10699. Same scope-escape
discipline; Vec<ref T> means every ref in the Vec borrows from
a same-or-outer-scope source.

### Phases NOT in (B)

- Returning refs from functions ("ref T" return types). That's
  (C) territory — requires Rust-style explicit or inferred
  lifetimes ('a). Reject return-type refs with a clear diagnostic
  pointing at (C) as future work.
- Closures capturing refs. Also (C).
- Multi-level borrow chains (`&&T`). v1 probably doesn't need
  this; defer.

### Tests to ship

For each phase, pin both the accept-case AND the
reject-with-diagnostic case:

- `let r: ref Foo = ref some_foo;` accepts.
- `let r: ref Foo = ref some_foo; return r;` rejects (return-
  escape).
- `let r: ref Foo = ref some_foo; let bag = Bag { item: r };`
  accepts only if Bag's lifetime ≤ some_foo's (the easiest v1
  rule: Bag must be declared INSIDE the scope where some_foo
  is live — both bindings in same fn body block).
- `let r: ref Foo = ref some_foo; let xs: Vec<ref Foo> = vec(r);`
  same rule.
- `fn make_ref() -> ref Foo { let f = Foo {...}; return ref f; }`
  rejects (the natural use-case for (C), explicitly out of (B)
  scope).

### Acceptance criteria

- All existing tests pass.
- New tests pin accept + reject shapes per phase.
- Examples that exercise ref-in-let work end-to-end on both
  backends.
- The scope-escape analyzer's diagnostic clearly explains why a
  rejected case fails AND when (C) would lift the restriction.

### Estimated effort

15-25h across 3-4 sessions for a robust ship. Phase 1+2 are
the load-bearing pieces (~8-12h); Phases 3 and 4 are smaller
gates+tests.

---

## Suggested session ordering

Either piece is shippable independently. Recommended:

1. **Fresh session: Liveness optimization first** (6-10h, one
   session). It's contained, doesn't touch user-facing syntax,
   and shippability is high. Use it as a warmup for the larger
   L4 (B) arc.

2. **Next fresh session: L4 (B) Phase 1 + 2** (8-12h, one
   session). The scope-escape analyzer is the load-bearing
   piece.

3. **Next session after: L4 (B) Phase 3 + 4** (5-10h, one
   session). Smaller gates + tests once the analyzer is in
   place.

Total: ~3-4 fresh sessions to land both.
