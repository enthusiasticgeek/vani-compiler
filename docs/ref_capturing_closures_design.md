# Ref-capturing closures — scoping document (not started)

**Status:** SCOPING ONLY. No implementation. Written 2026-07-25 in response
to a real, repeated need (`vani-ml` v0.1.0's `logreg_fit` couldn't reuse
`vani-optimize`'s `gradient_descent_fixed`/`backtracking` for exactly this
reason — see `kosh-index/ROADMAP.md`'s "ML tier" section). This is a
decision document, not a start signal.

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
| v1 | Extend `lambda_lift_program`'s Arc-5c path (the `if ref_captures_clone.is_empty()` branch, `checker.rs:2019`) to also synthesize an env-struct + magic-make-closure call when ref-captures are present, with `Ref<T>`/`RefMut<T>` env-struct fields (the typing already used for the inline path, `checker.rs:1954`). Produces a real `Closure` value for the first time. | v-fix | Medium. Mechanical extension of code that already works for the by-value case; main new work is deciding the env-struct's field type for a ref capture and making sure codegen (both backends) handles a `Ref<T>` struct field inside an env-struct the same way it already handles other closure envs. |
| v2 | Non-escape enforcement: reject (using the fixed BUG-7 machinery) any use of a ref-capturing `Closure` value that would let it outlive a captured ref's scope — return, struct/Vec storage, or a `let` binding provably outliving the capture. Accept the "pass directly as a call argument" and "call directly in the same or a nested block" shapes. | v1 | **Highest-risk phase.** This is the actual novel safety argument — needs a clear, written-down rule for exactly which shapes are accepted vs. rejected before writing code, the same way `vani-symbolic`'s v0.2.0 needed a documented policy on which simplification rules fire. Validate against both a positive suite (the `vani-ml`/`vani-optimize` motivating shape: pass a ref-capturing closure as an argument to a higher-order fn, use it, return before the caller's frame does) and a negative suite (every BUG-7-shaped escape attempt, to confirm v-fix + v2 together actually close the hole, not just the two repros already found). |
| v3 | Update `vani-optimize` (and any other package with a fixed `fn(ref Vec<f64>, i64) -> f64`-style objective parameter) to accept `Closure(...)->...` instead of `fn(...)->...`, so `vani-ml`'s `logreg_fit` (and the future autodiff core) can actually pass a ref-capturing closure through. | v2 | Low — signature-type change in an already-published package, republish as a minor version bump. This is the actual payoff step; nothing upstream of it changes `vani-ml`'s behavior. |

**Recommended order**: v-fix → v1 → v2 (budget the most review time here) →
v3. **Confirm before starting each phase.** v-fix should probably happen
regardless of whether the rest of this proceeds, given it's a live
soundness bug with no relationship to closures at all once you strip the
motivating context away.

**Explicitly out of scope**: general multi-parameter lifetime variables
(path A above), lifetime-parameterized struct *definitions* (`struct
View<'a>` — a user-authored struct holding a ref field with a name-level
lifetime parameter, as opposed to the compiler-synthesized closure
env-structs this document is about), closures that need to escape their
creating function's scope while still borrowing (e.g. returning a
ref-capturing closure to a caller who keeps the borrowed data alive
longer — Path A territory, not attempted here).

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
- **v1**: comparable to `vani-bignum`'s BUG-4 fix (a real but mechanical
  parser-dispatch extension) — a few hours once v-fix is settled.
- **v2**: the actual unknown. Writing the accept/reject rule down clearly
  is most of the work; once written, enforcement is "another case in an
  analyzer that already exists," not a new pass. Budget this like
  `vani-symbolic`'s v0.2.0 (highest-risk phase, most review time) rather
  than like a typical `TODO_CURRENT.md` item.
- **v3**: trivial, a signature-type edit in one already-published package.

Overall: **a large single-digit-days-to-low-weeks effort, not the
multi-week-minimum, open-ended estimate path A (full lifetime variables)
would require** — the key finding of this scoping pass is that the real
gap is narrower than `docs/missing_features.md`'s framing suggested, and
most of the machinery it needs already exists and already works for the
by-value case.
