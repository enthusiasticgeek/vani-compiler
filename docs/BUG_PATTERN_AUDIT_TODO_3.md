# vāṇी — Bug-pattern audit, round 3

**STATUS (2026-08-08): CLOSED.** Both categories fixed same day as
creation — category A (`Box<dyn Iface>` forward-declaration gap) and
category B (shift-amount width mismatch) both landed as BUG-146. See
`docs/TODO_CURRENT.md`'s BUG-146 section for the full fix + verification
writeup. Sequel to `docs/BUG_PATTERN_AUDIT_TODO_2.md` (round 2, fully
closed the same day this file was created — categories A through F,
BUG-141 through BUG-145, plus a localfuzz harness hardening). Round 2's
own closing note said the next hunt needs a genuinely new theme, not a
category G — this file **is** that new theme, but built differently
than round 2 was: instead of deriving categories from round 2's own
fixed bugs' shapes, this round comes from a full triage of the
continuously-running localfuzz harness's backlog (26 candidates
accumulated 2026-08-03 through 2026-08-08, re-verified against a main
build that already includes all of round 2's fixes). Two of those
candidates are genuine, unfixed compiler bugs — both investigated well
past "here's a repro" into an actual root-cause hypothesis, so this file
doubles as a head start on the fix itself, not just a test-case list.

## How this file was built

Ran every one of the 26 `docs/TODO_LOCAL_STAGING.md` candidates still
marked `STATUS: needs human/frontier root-cause review.` (in the
`vani-compiler-localfuzz` worktree, `local-fuzz-findings` branch)
through a standalone triage script pointed at the current `main` build
(commit `a20345e`, includes BUG-141 through BUG-145 and the harness
hardening) — the same "don't trust a possibly-stale worktree binary"
technique the 2026-08-07 backlog triage used. Breakdown:

- **5 stale** (dated 2026-08-03/04): already fixed by later work,
  re-verified clean.
- **14 benign**: the documented, permanent C-exits-134-vs-LLVM-exits-3
  trap-code convention difference (see `10b_runtime_errors_primer.md`),
  which the harness's crude `rc` comparison flags as a "divergence"
  every time even though it's expected, not a bug.
- **7 timeouts, none compiler bugs**: qwen's mutation/generation
  introduced a genuine infinite loop or an absurd `sleep_ms(i64::MAX,
  ...)` duration in the FUZZED SOURCE ITSELF — 4 shared the exact same
  `sleep_ms(9223372036854775807, ...)` async-smoke-test template
  (≈292 million years), 1 was a `while i < 5 { i = i + 1; }` starting
  at `i64::MIN` (a real ~9.2-quintillion-iteration loop, just too slow
  for a 20s harness timeout, not incorrect), 1 had a mutated `i = i +
  -1;` decrement that makes `while i < 50` diverge away from its bound
  forever, and 1 had an extra/unmatched `channel_recv` call spin-
  waiting on a message that was never sent. All closed in the staging
  doc with the specific mechanism identified per-candidate.
- **2 REAL findings** — categories A and B below.

All 26 are closed out in `docs/TODO_LOCAL_STAGING.md` with specific
notes (commit `de4fc66` in the worktree, local-only, matching this
project's established "this branch isn't pushed" convention).

**Before starting**: re-verify freshness the same way — `git fetch
origin && git log origin/main --oneline -3`, and re-check
`docs/TODO_CURRENT.md`'s tail for the actual next-free `BUG-N` (a
concurrent localfuzz process also lands fixes to this repo; don't trust
a number cached from this file). Next free bug number was **BUG-146**
as of this writing (2026-08-08).

Priority key: 🔴 high (root cause confirmed, close to a direct fix) ·
🟡 medium (root cause narrowed to a specific code region, but the exact
mechanism still needs runtime investigation).

---

## A. `Box<dyn Iface>` payload/field never registers its interface for C/LLVM forward-declaration (🔴 high) -- CLOSED 2026-08-08, BUG-146

**Update (2026-08-08)**: fixed. Widened `walk_type`'s match arms in
`collect_used_dyn_ifaces` to cover every "wraps exactly one inner type"
`Type` variant, not just `Box` alone (`Vec128/256/512`, `TaskR`,
`RwLock`, `ReadGuard`, `WriteGuard`, `Deque`, `HashSet`, `BTreeSet`,
`BinaryHeap`, `Bst`, `Ptr`, `PtrMut`, `Pool`, `Handle`, `Tainted`,
`BoundedPtr`, `ArenaRef`, `HashMap`/`BTreeMap`, `Closure`). Verified both
the enum-payload and struct-field repros on both backends, and that the
already-fixed `Vec<Box<dyn Iface>>` case still works. Full writeup in
`docs/TODO_CURRENT.md`'s BUG-146 section.

<details>
<summary>Original writeup, kept for the reasoning trail</summary>

**Where this comes from**: localfuzz candidate
`20260808-150021-backend-divergence-65faa42884` — an `enum` with a
`Box<dyn Iface>`-payloaded variant that's declared but never
constructed anywhere in the program failed to compile on the C backend:

```vani
intent "Box<T> as enum payload — checker + C/LLVM Drop";

interface Drawable {
  fn area(self: ref Self) -> i64;
}

struct Circle { r: u32 }
implement Drawable for Circle {
  fn area(self: ref Circle) -> i64 { return self.r * self.r; }
}

enum Val {
  Int(Box<i64>),
  Shape(Box<dyn Drawable>),
  Empty,
}

fn main() -> i64 {
  let a: Val = Val.Int(box(42));   // Shape variant is NEVER constructed
  let e: Val = Val.Empty;
  print 42;
  return 42;
}
```

C backend: `error: unknown type name 'intent_dyn_Drawable'` at
`intent_dyn_Drawable v_Shape;` (the enum's own tagged-union member for
the unconstructed `Shape` variant). LLVM backend: compiles and runs
clean — its on-demand lowering never touches the unconstructed
variant's storage the same eager way tree-C's whole-module union
emission does. Same "eager C emission references a type that was never
forward-declared because nothing tracked it as needed" shape BUG-139
found for plain name-existence — but this is a REAL, declared
interface (`Drawable` exists), so BUG-139's validator doesn't (and
shouldn't) catch it; this is an emission-ORDERING bug, not a name-
existence one.

**Root cause, found**: `collect_used_dyn_ifaces` (`backend_c.rs`,
~line 22610) walks the whole program collecting every `Type::Object`
(the `dyn Iface` marker type) it finds, so `emit_dyn_iface_typedefs`
knows which `intent_dyn_<Iface>` fat-pointer typedefs to forward-
declare before anything else can reference them. Its `walk_type`
helper has match arms for `Vec`, `Atomic`, `Mutex`, `Guard`, `Ref`,
`RefMut`, `Channel`, `Tuple`, `FnPtr`, `Array` — recursing into each to
find a nested `Type::Object` — but **no arm for `Type::Box`**. A
`Box<dyn Iface>` type anywhere in the program (an enum variant payload,
a struct field, ...) silently falls through to the `_ => {}` catch-all
and never registers its interface, UNLESS the value is actually
constructed somewhere (in which case a separate expression-walking path,
`TypedExprKind::EnumVariantWithPayload` / `DynDispatch`, picks it up
independently — which is exactly why "never constructed" is the
trigger condition). The fix is almost certainly a one-line addition:
```rust
Type::Box(inner) => walk_type(inner, set),
```
right alongside the existing `Vec`/`Atomic`/`Mutex`/etc. arms.

Confirming detail: the comment immediately above `collect_used_dyn_
ifaces`'s call site (~line 510-519) describes fixing the *exact*
analogous gap for `Vec<Box<dyn Iface>>`, but in a **different, sibling
function** (`vec_element_has_user_struct`, which DOES have a
`Type::Box(inner) => vec_element_has_user_struct(inner)` arm) —
`collect_used_dyn_ifaces` just never got the same fix applied to it.

**Also confirmed to affect LLVM, not just C** — broader than the
localfuzz candidate's own C-only symptom suggested. A **struct field**
of type `Box<dyn Iface>` (as opposed to an enum variant payload), where
the struct itself is never constructed, breaks BOTH backends:
```vani
interface Drawable { fn area(self: ref Self) -> i64; }
struct Circle { r: u32 }
implement Drawable for Circle {
  fn area(self: ref Circle) -> i64 { return self.r * self.r; }
}
struct Holder { shape: Box<dyn Drawable>, tag: i64 }
fn main() -> i64 { return 42; }  // Holder is never constructed
```
C: same "unknown type name 'intent_dyn_Drawable'". LLVM: `error: use of
undefined type named 'intent_dyn_Drawable'` in `%Struct_Holder = type {
%intent_dyn_Drawable, i64 }`. This confirms it's the SAME root cause on
both backends, not two independent bugs — `backend_llvm.rs`'s own
`collect_used_dyn_ifaces_llvm` (~line 47564) is a literal one-line
passthrough: `crate::backend_c::collect_used_dyn_ifaces(program)`. Fix
the one function, both backends benefit; no separate LLVM-side fix
needed.

**Candidates to check once the fix lands**:
- Re-verify `Vec<Box<dyn Iface>>` (the already-fixed case per the
  comment above `collect_used_dyn_ifaces`'s call site) still works —
  confirm the new `Box` arm in `collect_used_dyn_ifaces` doesn't somehow
  interact badly with `vec_element_has_user_struct`'s existing, separate
  fix for the same shape.
- A doubly-nested shape: `Vec<Box<dyn Iface>>` as an enum payload or
  struct field (two container layers deep) — does `walk_type`'s
  recursion correctly reach the innermost `Type::Object` once `Box` is
  added, or does `Vec`'s own recursion need to compose correctly with
  the new `Box` arm (it should, structurally, but confirm empirically
  rather than assuming).
- Whether `walk_type`'s match arm list is missing OTHER inner-type
  wrappers besides `Box` — cross-check the full `Type` enum's set of
  "wraps exactly one inner type" variants (the same list
  `type_references_unknown_name` in `checker.rs` enumerates, from
  BUG-139/BUG-144's fixes) against `walk_type`'s arms; candidates worth
  checking specifically: `Pool<T>`, `Handle<T>`, `Tainted<T>`,
  `BoundedPtr<T>`, `ArenaRef<T>`, `Ptr<T>`/`PtrMut<T>` — any of these
  wrapping a `dyn Iface` and never constructed could hit the identical
  gap.

</details>

## B. Shift-amount width mismatch for compound RHS expressions (🟡 medium) -- CLOSED 2026-08-08, BUG-146

**Update (2026-08-08)**: fixed. The checker was correct to leave the
shift-count's width unconstrained (a shift amount is deliberately
allowed a different width than the shifted value) -- the actual gap was
codegen-only, and only on the SSA-LLVM path specifically (tree-LLVM
already had a correct fix for this class; a simple, SSA-eligible program
takes the SSA path by default via the CLI, which is why this went
undetected). Fixed by looking up the RHS operand's own type separately
in `ssa_backend_llvm.rs` and trunc/sext/zext-ing it to match before use.
Verified across `u8`/`Shl`, `u8`/`Shr`, `u32`/`Shl`. Full writeup in
`docs/TODO_CURRENT.md`'s BUG-146 section.

<details>
<summary>Original writeup, kept for the reasoning trail</summary>

**Where this comes from**: localfuzz candidate
`20260808-141920-backend-divergence-fc33c7c09f`, minimized:
```vani
fn main() -> i64 {
  let bits: u8 = 1 as u8;
  let shifted: u8 = bits << 3 + 0;
  print shifted;
  return 0;
}
```
LLVM backend: `lli: ...: error: '%v_1' defined with type 'i64' but
expected 'i8'` in `%v_2 = shl i8 %v_0, %v_1`. Confirmed the trigger is
specifically a **compound** shift-amount expression — `bits << 3`
(a bare literal, no `+ 0`) compiles and runs correctly; `bits << 3 + 0`
does not. Same general "checker accepts a mismatched-width operand,
codegen assumes it matches" shape category A audited in round 2 (fixed
as BUG-138/BUG-141) — but for `Shl`/`Shr`'s right-hand operand, a site
category A's own sweep never touched.

**Root cause, narrowed but not fully nailed down**: `check_shift`
(`checker.rs`, ~line 22701) validates the shift-count operand with
`rhs.ty().is_integer()` only — it never coerces `rhs`'s width to match
`lhs`'s (the same permissive pattern `check_set_builtin`/`clone_at` had
before BUG-138/BUG-141). That part is a clean, confirmed gap. What's
NOT yet clear: `backend_llvm.rs`'s shift emission (~line 5628-5664) DOES
have width-adjustment logic that looks structurally correct on
inspection —
```rust
let r_shift = if matches!(op, BinaryOp::Shl | BinaryOp::Shr)
    && !matches!(right.ty, _ if left.ty.bits() == right.ty.bits())
{
    // ... sext/zext/trunc based on width comparison ...
} else {
    r.clone()
};
```
— and empirically this WOULD correctly truncate an i64-typed shift
count down to `lhs`'s width if `right.ty` genuinely reports `i64` at
this point. Since the observed failure is a raw, untruncated i64 value
reaching the `shl i8` instruction, either `right.ty` isn't actually
`i64` when this code runs (despite `check_shift` never coercing it —
maybe something ELSE narrows the type between checking and codegen),
or this width-adjustment code isn't being reached at all for this
specific shape (maybe constant-folding routes `3 + 0` through a
different `emit_expr` path entirely, e.g. as a pre-folded
`TypedConst::Int` that bypasses the normal `Binary` emission arm this
guard lives in).

**First step for next session**: dump `vanic ir` (the typed-IR debug
dump) for both the working (`bits << 3`) and failing (`bits << 3 + 0`)
cases side by side, and separately dump the emitted LLVM IR for both,
to see exactly where the two diverge — specifically, what `right.ty`
actually is by the time the width-comparison code runs, and whether
`check_shift`'s own constant-folding path (`eval_shift`, invoked when
both operands are compile-time constants) produces a differently-typed
or differently-shaped `TypedExpr` than the non-folded case. Also worth
checking directly: does the identical repro shape reproduce on the C
backend (C's implicit integer promotion may make this a non-issue there,
matching BUG-141's finding that the C backend was structurally immune
to its analogous call-signature-width class), and does `Shr` share the
exact same gap as `Shl`, or diverge from it?

</details>

---

## Process (mirrors rounds 1 and 2's own process sections)

- Category A is the clear first pick — root cause is a one-line fix
  (`Type::Box(inner) => walk_type(inner, set),` in `collect_used_dyn_
  ifaces`, `backend_c.rs` ~line 22610), already confirmed to affect both
  backends via the same shared function. Verify, write regression tests
  (both the original enum-payload shape and the struct-field shape,
  both backends, plus the "already-fixed `Vec<Box<dyn Iface>>` still
  works" regression check), then move to the broader inner-type-wrapper
  sweep listed under category A's own candidates.
- Category B needs actual debugging (typed-IR + LLVM-IR diffing) before
  a fix is safe to attempt — don't guess at a fix without first
  understanding why the width-adjustment code that LOOKS correct isn't
  firing; a wrong guess here risks silently breaking the (correctly-
  passing) bare-literal shift case.
- Batch fixes before a full local `cargo test --release` run +
  commit/push, per this project's established CI-driven workflow — with
  only 2 categories this round, that likely means one commit each
  rather than batching them together, given category B may need
  significantly more investigation time than category A.
- A "clean pass" (repro compiles and runs correctly once fixed, matches
  a hand-computed expected value) still gets a permanent regression
  test, per this project's own convention.
- Check `docs/TODO_LOCAL_STAGING.md` again before starting, even though
  this file's own triage just closed it out — the harness runs
  continuously and may have found more since this file was written.
- Once both categories here are closed, this file's scope is exhausted
  the same way rounds 1 and 2's were. The next hunt should probably
  default to another localfuzz backlog triage (the harness never stops
  generating candidates) rather than re-deriving categories from the
  bug corpus a third time — but check `docs/TODO_LOCAL_STAGING.md`'s
  actual state first; if it's thin, a corpus-wide re-derivation is the
  fallback, same as round 2's own opening reasoning.
