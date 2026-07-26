# Ref-capturing closures — scoping + implementation log (v-fix, v1, v2, v3 done)

**Status:** all four phases (v-fix, v1, v2, v3) implemented and pushed
2026-07-25. Written in response to a real, repeated need (`vani-ml`
v0.1.0's `logreg_fit` couldn't reuse `vani-optimize`'s
`gradient_descent_fixed`/`backtracking` for exactly this reason — see
`kosh-index/ROADMAP.md`'s "ML tier" section).

**v3 used the additive approach, not the "instead of" wording this
document originally used**: `Closure(...)->...` and `fn(...)->...` have
no implicit coercion (confirmed by direct test), so changing
`vani-optimize`'s existing functions' parameter types outright would have
broken every current caller, including its own tests/examples. Added
`gradient_descent_fixed_closure`/`armijo_line_search_closure`/
`gradient_descent_backtracking_closure` instead, leaving the originals
untouched — genuinely a minor version bump (v0.1.5), zero breakage.

**Two more real bugs found and fixed validating v3 (BUG-10, BUG-11)** —
see `docs/TODO_CURRENT.md`. BUG-10 was severe: without it, v3 as written
would have broken *every* existing `vani-optimize` consumer, not just
been inert for them (any program including the new functions — even
unused — failed to compile, since the closure struct typedef they
reference was never emitted unless a matching closure literal happened
to exist in the same compiled program).

**Known remaining gap, found while building v2 (filed as BUG-9, NOT
fixed)**: v2's Return/push checks are sound, but the FieldAssign check
(and by the same logic, potentially the push check too) can be fooled
when the container is reached through a `ref`/`mut ref` **parameter**
rather than an owned local — see "BUG-9" below. This is a pre-existing
gap in the original L4 Phase 3 mechanism (2026-06-08), not something v2
introduced, and it applies equally to plain (non-closure) `ref`-field
structs. Not fixed — flagged as a real, live caveat on v2's "non-escape
enforcement" claim.

**Along the way, found a real soundness bug (BUG-7) plus a second, related
codegen bug (BUG-8) — both fixed same day (2026-07-25), see
`docs/TODO_CURRENT.md`.** BUG-7 was a confirmed dangling-reference bug,
not just a missing feature; BUG-8 was found writing BUG-7's positive
control (the *legitimate* case) and turned out to be broken too, silently,
on the LLVM backend only. See "Prerequisite: BUG-7 (and BUG-8)" below --
kept here for the full context even though both are now fixed, since the
write-up explains exactly what machinery a future closures implementation
would be building on top of.

---

## Correcting the record: ref-capturing closures partially exist already

`docs/missing_features.md` and `docs/v1_limitations.md` (L4) both describe
ref-capturing closures as flatly unsupported, filed under "path-D,
deferred indefinitely." That's not quite right, and the actual boundary
matters a lot for scoping. Confirmed by direct test (2026-07-25, this
`vanic` build):

**Works today** — `[ref name]` explicit capture-list syntax (`ARC 3a`,
shipped alongside L5's affine closures), for a `let`-bound closure called
directly by name within the same scope, including non-Copy captures like
`Vec<f64>`:

```vani
fn main() -> i64 {
    let v: Vec<f64> = vec(1.0, 2.0, 3.0);
    let f: fn(i64) -> f64 = fn(x: i64) -> f64 [ref v] { return v[0] + (x as f64); };
    let r: f64 = f(5);   // runs correctly: 6.0
    return 0;
}
```

**Does NOT work** — using that same closure as a genuine first-class
value: returning it, storing it, or passing it as an argument to another
function (even one whose parameter is correctly typed `Closure(i64) ->
f64`, the real closure-value type, not the raw-fn-pointer `fn(i64) ->
f64`):

```vani
fn apply(f: Closure(i64) -> f64, x: i64) -> f64 { return f(x); }
fn main() -> i64 {
    let v: Vec<f64> = vec(1.0, 2.0, 3.0);
    let g: fn(i64) -> f64 = fn(x: i64) -> f64 [ref v] { return v[0] + (x as f64); };
    let r: f64 = apply(g, 5);   // error: unknown variable 'g'
    return 0;
}
```

Root cause, precisely pinned: `checker.rs`'s `lambda_lift_program` (the
pass that hoists closures and, for by-value/Copy captures, synthesizes an
env-struct + a magic `__intent_make_closure_*` call so the binding becomes
a real `Closure(...)->...` value — Arc 5c, already shipped and working,
confirmed via the by-value positive control above) has this exact comment
at `checker.rs:2016-2019`:

> `// Arc 5c: synthesize env-struct + register closure-make magic-call
> entry so Var(bind_name) in value position ... can resolve to a Closure
> value. Skip ref-captures for v1 (only by-value Copy captures supported
> in the Closure-value path).`

When `ref_captures` is non-empty, that whole branch is skipped. The
closure is instead registered only in a `closure_handles` map used to
textually rewrite *direct* call sites (`f(args)` → a call to the hoisted
function with the ref threaded through) — never a `Stmt::Let` that
actually binds `g` to anything at runtime. That's why `g` is "unknown":
it never existed as a value, only as a compile-time call-site macro.

**Practical implication**: `[ref name]` closures today are call-site
sugar for a same-scope helper function, not real closures. This is
exactly why `vani-optimize`'s higher-order solvers — which need a genuine
`fn`/`Closure` *value* passed in as an argument — can't be fed one that
captures training data by reference. Passing a plain top-level named
function (no capture at all) works fine today; the gap is specifically
**a value with a captured reference inside it**.

---

## Prerequisite: BUG-7 + BUG-8 (both fixed 2026-07-25)

Any design here would extend the *env-struct* mechanism (already used for
by-value closures) to also hold `ref T` fields, and would need the
existing L4 Phase-3/4 "scope-escape analyzer" (`checker.rs`, the
`Stmt::Return` handler around line 10692) to reject a Closure value whose
env-struct holds a ref that would dangle. That analyzer had a confirmed,
live bypass, independent of closures entirely — filed and fixed same day
as BUG-7:

```vani
struct Holder { v: ref Vec<f64> }
fn make() -> Holder {
    let v: Vec<f64> = vec(1.0, 2.0, 3.0);
    return Holder { v: ref v };   // correctly REJECTED (inline shape)
}
```
is caught. But:
```vani
fn make() -> Holder {
    let v: Vec<f64> = vec(1.0, 2.0, 3.0);
    let h: Holder = Holder { v: ref v };
    return h;                     // vanic check: ok  <-- WRONG
}
// caller: print h.v[0];  ->  1.2655e-311 (garbage, not 1.0)
```
was silently accepted and produced a live use-after-free at runtime — not
a diagnostic-quality gap, an actual memory-safety bug. Root cause: the
alias-chasing added for exactly this class of bypass
(`collect_var_ref_aliases`, L4 Phase 4, 2026-06-09) only tracked bindings
whose *own declared type* is `ref T`; it had no notion of "this owned
struct's *field* holds a ref sourced from X." A closure's synthesized
env-struct is structurally identical to `Holder` here — an owned struct
with a `ref T` field — so any ref-capturing-closure design that leans on
this escape analyzer would have inherited this hole for free, silently.
**Fixed 2026-07-25** (see `docs/TODO_CURRENT.md`'s BUG-7 entry for the
exact fix: `compute_ref_aliases_from_let_rhs` gained a `StructLit` arm,
and the `Stmt::Let` handler's type-guard was relaxed to call it for every
binding shape, not just `ref`-typed ones).

**BUG-8, found immediately after** while writing BUG-7's positive control
(the *legitimate*, non-escaping version of the shape above): even that
case silently returned garbage — `h.v[i]` for a `ref`-typed **Vec** field
misread the Vec struct's own `data` pointer as the requested element,
LLVM backend only (C backend was correct). Also fixed same day — see
`docs/TODO_CURRENT.md`'s BUG-8 entry. Relevant here because it confirms
the *existing*, already-"shipped" ref-typed-struct-field machinery this
design would build on had never actually been executed and checked for a
correct *value*, only checked for compiling — the same class of gap BUG-6
exposed for unary minus. Any future closures work building on this
machinery should keep that pattern in mind: compiling without error is not
evidence of a correct runtime value.

---

## Two design paths

### Path A — general lifetime variables (full path-D)

What `docs/decisions.md`'s 2026-06-09 entry already declined: `'a`/`'b`
syntax, lifetime parameters propagating through fn signatures *and*
struct definitions, multi-lifetime borrow-check rules, closure-env
lifetime tracking. Unchanged from that decision's own reasoning — this is
still large, still open-ended, and the team's stated rationale ("the vast
majority of practical cases involve a single source; explicit lifetime
variables for the rare N-ref case add syntax complexity disproportionate
to its use") still holds for the *general* case. Two real hits on the
narrower ref-capture gap (this document, plus the anticipated `vani-ml`
v0.3.0 autodiff core) don't actually argue for general multi-lifetime
variables — both hits are the *same* narrower shape (path B below), not
the N-distinct-lifetimes case path-D was declined for. **Not recommended
to reopen based on evidence gathered so far.**

### Path B — non-escaping ref-capturing closure values (recommended target)

Don't build general lifetime inference. Extend the *existing, working*
Arc-5c machinery (env-struct synthesis + magic-make-closure + `Closure`
type) to also accept ref-captured fields, but make the resulting value
**provably non-escaping by construction** — reusing the same category of
check the scope-escape analyzer already does for `Holder`-style structs
(once BUG-7's gap in it is closed), not a new lifetime-inference system.
Concretely: a `Closure` value with any ref-captured field may be passed
as a function argument or used within the block it was created in, but
may never be returned, stored into a struct/Vec field, or bound to a
name whose scope is checked to outlive any of its captured refs' scopes.
This is a bounded generalization of a check the compiler already makes
(local-escapes-via-return, local-escapes-via-struct-field) applied to one
more value shape (a `Closure`'s synthesized env), not a new subsystem.

**Phased breakdown** (versions are proposed, not committed):

| Phase | Scope | Depends on | Risk / notes |
|---|---|---|---|
| ~~v-fix~~ ✅ done 2026-07-25 | Fix BUG-7 (checker: escape-analyzer bypass) and BUG-8 (LLVM backend: garbage value reading a `ref`-typed Vec field), found along the way. | -- | Both fixed same day. BUG-7: `compute_ref_aliases_from_let_rhs` gained a `StructLit` arm, the `Stmt::Let` guard was relaxed to run it unconditionally. BUG-8: an extra `load` to dereference a field-slot address before treating it as the Vec's own address. 16+32-test regression spot-check clean, all four `vani-ml` tests still pass. See `docs/TODO_CURRENT.md`'s BUG-7/BUG-8 entries for full detail. |
| ~~v1~~ ✅ done 2026-07-25 | Extended `lambda_lift_program`'s Arc-5c path (`checker.rs`, previously gated `if ref_captures_clone.is_empty()`, now runs unconditionally) to also synthesize an env-struct + magic-make-closure call when ref-captures are present, with `Ref<T>` env-struct fields (the typing already used for the inline path). Produces a real `Closure` value for a ref-capturing closure for the first time — verified end-to-end: `apply(g, 5)` where `g` ref-captures a `Vec<f64>` now runs correctly on **both** backends. | v-fix | Mechanical extension, as predicted — but the C backend needed its own fix: the closure-registry codegen used `c_leaf_type` (a `&'static str` lookup for simple types only) on the capture types, which produced the invalid placeholder `/* ref */` for a `Ref<Vec<T>>` capture. Fixed by switching to `format_declarator` (the same function real `ref T` function parameters already render through, so the `const`-qualified spelling matches the hoisted function's own separately-emitted declaration byte-for-byte — an earlier attempt using `c_element_storage` compiled but produced a `const`-mismatch warning-then-hard-conflicting-declaration error). No LLVM backend changes needed at all — its trampoline/constructor codegen was already fully generic over the capture type. New tests: `lli_runs_ref_capturing_closure_passed_as_higher_order_fn_arg` (`backend_llvm.rs`, actually executes), `ref_capturing_closure_as_value_passes_to_higher_order_fn` (`lib.rs`, pins both backends' generated shape). 74-test regression spot-check clean (closures + L4/escape + struct-field/Vec-indexing), all four `vani-ml` tests + the example on both backends still pass. **No non-escape enforcement yet** — a ref-capturing `Closure` value can currently be returned/stored/escaped with no safety check at all; that's v2, not yet started. |
| ~~v2~~ ✅ done 2026-07-25 | Non-escape enforcement: reject any use of a ref-capturing `Closure` value that would let it outlive a captured ref's scope. | v1 | Turned out smaller than the "highest-risk" label implied, because it reduced almost entirely to **alias propagation into machinery that already existed**: `compute_ref_aliases_from_let_rhs` gained a `Call`-to-magic-make-closure arm (mirroring BUG-7's `StructLit` arm) so a `Closure`-typed binding gets correct `ref_aliases`; the *existing* Return and FieldAssign escape checks then reject it with zero further changes (neither has a restrictive type guard). The `push` check *did* need a one-line widening (its guard only fired for `Vec<ref T>` elements; extended to also cover `Vec<Closure(...)->...>`). Verified: `return g;` (direct + two-hop `let`-chain) and `push(mut ref closures, g)` into an outer-scope `Vec<Closure>` are both now rejected with clear diagnostics; passing `g` as a call argument or calling it directly are both unaffected (Call args were already a "consuming position," per L4 Phase 3's original design). Two new negative-case tests (`ref_capturing_closure_returned_directly_is_rejected`, `ref_capturing_closure_pushed_into_outer_scope_vec_is_rejected`, `lib.rs`) plus a 97-test regression spot-check. **Found and filed BUG-9 along the way** (not fixed) — see below; a real, pre-existing gap unrelated to closures specifically, but one that undercuts the FieldAssign half of this phase's guarantee in one specific shape. |
| ~~v3~~ ✅ done 2026-07-25 | Added `_closure`-suffixed variants (`gradient_descent_fixed_closure`, `armijo_line_search_closure`, `gradient_descent_backtracking_closure`) to `vani-optimize` accepting `Closure(...)->...`, additive alongside the untouched originals. | v2 | Turned out **not** low-risk as originally estimated — found BUG-10 (a function merely *taking* a `Closure`-typed parameter, with no matching closure literal anywhere in the program, failed to compile on both backends) and BUG-11 (C-backend-only: a closure shape referencing `Vec<T>` could have its typedef ordered before `Vec<T>`'s own). BUG-10 in particular meant the additive approach would otherwise have broken every existing `vani-optimize` consumer outright, not just been a no-op for them — this was the real risk in v3, not the signature-type change itself. Both fixed same day; see `docs/TODO_CURRENT.md`. `vani-optimize` v0.1.5 committed and pushed to its own repo; **not published** to the Kosh registry (stopped for an explicit go-ahead, per this session's pattern). |

**Recommended order**: ~~v-fix~~ → ~~v1~~ → ~~v2~~ → v3. **Confirm before
starting each phase** — v-fix, v1, and v2 are done (2026-07-25); v3 still
needs an explicit go-ahead before starting.

**Explicitly out of scope**: general multi-parameter lifetime variables
(path A above), lifetime-parameterized struct *definitions* (`struct
View<'a>` — a user-authored struct holding a ref field with a name-level
lifetime parameter, as opposed to the compiler-synthesized closure
env-structs this document is about), closures that need to escape their
creating function's scope while still borrowing (e.g. returning a
ref-capturing closure to a caller who keeps the borrowed data alive
longer — Path A territory, not attempted here).

### BUG-9 (found during v2, ✅ fixed 2026-07-26 — pre-existing, not closure-specific)

While testing v2's FieldAssign coverage, found that the check can be
fooled when the assignment target is reached through a `ref`/`mut ref`
**parameter** rather than an owned local:

```vani
struct Holder { v: ref Vec<f64> }
fn fill(h: mut ref Holder) -> i64 {
    let v: Vec<f64> = vec(1.0, 2.0, 3.0);
    h.v = ref v;   // vanic check: ok -- should be rejected
    return 0;
}
```
`h`'s actual `Holder` lives in the *caller's* frame (potentially far
outliving `fill`), but `fill`'s local `v` is dropped when `fill` returns
— after which `h.v` (visible in the caller) dangles. `vanic check` accepts
this. **Confirmed pre-existing and unrelated to closures**: reproduces
identically with a plain `ref`-field struct, no `Closure` involved, so it
predates this session's work and isn't something v2 introduced — but it
does mean v2's FieldAssign-based closure-escape protection has the same
hole (`b.c = g;` through a `mut ref` parameter `b` is equally unchecked).

**Root cause**: the FieldAssign check (`checker.rs`, L4 Phase 2,
2026-06-08) compares `env.lookup_depth(obj_name)` (the object's *lexical
declaration depth within the current function*) against the ref source's
depth, rejecting when the source is declared deeper. This conflates two
different things: a `ref`/`mut ref` **parameter**'s depth-within-the-
current-function tells you nothing about how long the object it points to
actually lives (it lives in the *caller's* frame, of unknown — but
certainly longer — lifetime), yet the check treats it the same as an
**owned local** binding's depth (which correctly bounds the object's
lifetime to the current function). Parameters and top-level function-body
locals appear to share the same depth number in practice, so a
same-top-level-depth local `v` isn't flagged as "deeper" even though it's
categorically less long-lived than whatever `h` points to.

**Fixed**: took option (a) — when the assignment target is reached through
a `mut ref` (`through_mut_ref`, already computed above this check), skip
the depth comparison and instead require the ref source to be one of the
current function's own parameters, matching `Return`'s existing rule
exactly. Turned out to be a small, surgical fix, as guessed — `function:
&Function` was already in scope in `check_one_stmt` (the same place
`Return`'s check reads `function.params` from), so no plumbing was
needed. Verified: the repro above now rejects; a positive control
(assigning a ref sourced from `fill`'s own parameter through the same
`mut ref` target) still compiles and runs correctly; 112-test regression
spot-check clean; `vani-ml`/`vani-optimize` full suites re-verified on
both backends.

**BUG-12, found immediately after (NOT fixed)**: `push(mut ref xs, ref
X)`'s scope-escape check has the identical `lookup_depth`-through-a-
`mut-ref`-parameter flaw, confirmed via direct test. Not fixed in the
same pass: unlike FieldAssign, the push check (`check_push_builtin`,
called from `check_call`) doesn't have `function: &Function` in scope,
and `check_call` is a much more widely-called part of the checker —
threading a "current function's parameter names" signal through it is a
bigger, higher-blast-radius change than BUG-9's fix was, and needs its
own careful pass rather than being folded into this one. See
`docs/TODO_CURRENT.md`'s BUG-12 entry for the full repro and a sketch of
lower-risk fix shapes (e.g. a thread-local populated per-function-entry,
avoiding the `check_call` plumbing entirely).

---

## Effort estimate

Calibrated the same way `kosh-index/ROADMAP.md`'s package-effort table is
— relative units, not wall-clock, though this table has no prior track
record to calibrate against the way the kosh math packages did (12
shipped packages' actual effort vs. estimate, all correct in retrospect).
Take this one estimate with more uncertainty than that table's:

- **v-fix**: ✅ done — turned out to be a small, surgical fix in both
  cases once the root cause was pinned down precisely (a couple of hours
  each including the direct-test investigation that found them), matching
  the "Medium" `TODO_CURRENT.md` bucket's low end rather than its high end.
  The uncertainty was in *finding* the exact root cause, not in the size
  of the eventual patch.
- **v1**: ✅ done — the checker-side change was indeed a few hours, as
  predicted, comparable to `vani-bignum`'s BUG-4 fix. The one thing this
  estimate missed: it assumed "codegen (both backends)" would need
  checking but not necessarily changing; in fact the C backend needed a
  real fix too (the closure-registry's `capture_types` rendering used a
  simple-leaf-type-only helper that couldn't spell a `Ref<Vec<T>>`
  capture type) — LLVM needed no changes at all. Total time was still
  well within the original estimate, just split differently than
  expected between "check" and "fix."
- **v2**: ✅ done — the "actual unknown" resolved cleanly: the accept/
  reject rule turned out to already exist (Return/FieldAssign's existing
  logic), so v2 was almost entirely alias-propagation plumbing plus one
  guard widening, comparable to the v-fix bugs in size, not to
  `vani-symbolic`'s v0.2.0. The genuine unknown that remained (BUG-9) was
  a pre-existing gap unrelated to the closure-specific work, found by
  testing thoroughly rather than by the closure design itself being hard.
- **v3**: trivial, a signature-type edit in one already-published package.

Overall: **a large single-digit-days-to-low-weeks effort, not the
multi-week-minimum, open-ended estimate path A (full lifetime variables)
would require** — the key finding of this scoping pass is that the real
gap is narrower than `docs/missing_features.md`'s framing suggested, and
most of the machinery it needs already exists and already works for the
by-value case.
