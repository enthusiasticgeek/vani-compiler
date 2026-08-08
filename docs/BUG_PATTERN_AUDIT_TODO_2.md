# vāṇी — Bug-pattern audit, round 2 (post-BUG-140)

Created 2026-08-07 for a fresh session to pick up. Mirrors
`docs/BUG_PATTERN_AUDIT_TODO.md`'s own methodology exactly (that document's
8 categories, A through H, are now fully closed — BUG-113 through BUG-125 —
and its own closing note says the next hunt needs a genuinely new theme, not
a category I). This file **is** that new theme: instead of re-deriving
categories from the whole bug corpus again, it's built specifically from the
five root-cause *shapes* that showed up in BUG-126 through BUG-140 (2026-08-07's
localfuzz-driven session) — each of those bugs was found and fixed as an
isolated instance, but every one of them strongly suggests siblings that were
never checked. That's the working hypothesis this file exists to test.

**Why this approach still works**: the two feature-combination sweeps
(BUG-68–104) and the pattern audit (BUG-113–125) both found that this
codebase's bugs cluster around a small number of recurring *shapes* —
duplicate "find every use of X" walkers that don't actually cover every use,
raw `abort()` where `exit(3)` was intended, a fix applied at one call site
but not its siblings. BUG-126–140 add zero new *kinds* of root cause to that
list — they're the same shapes recurring at call sites nobody had checked
yet. That's the signal this file acts on.

**How to pick this up**: this file is self-contained — method, priority key,
and process section all included below, matching the two closed documents'
own structure. For each category: construct a real `.vani` repro (or grep
for existing candidate call sites), run `vanic check` + `vanic run` on both
backends, compare against a hand-computed expected value. A bug found gets
root-caused, fixed, regression-tested (both a fast `src/lib.rs` compile-only
test and a real subprocess `tests/run_end_to_end.rs` test, matching this
project's own established pattern), and logged in `docs/TODO_CURRENT.md` as
the next free `BUG-N` (**check `docs/TODO_CURRENT.md`'s tail for the actual
next-free number before starting** — another automated process
(`tools/localfuzz`) also lands fixes to this repo; re-verify freshness, don't
trust a number cached from an earlier session). A clean pass (no bug found)
still gets a permanent regression test, per this project's own "clean pass
still adds a test" convention — the audit's value is in ruling categories
out as much as finding new bugs.

Priority key: 🔴 high (found a confirmed sibling already / systemic-safety
class) · 🟡 medium (plausible, unconfirmed) · 🟢 low (narrower blast radius
or lower confidence a bug actually exists here).

---

## A. Index-parameter type-width audit across every builtin (🔴 highest priority) -- CLOSED 2026-08-08, BUG-141

**Update (2026-08-08)**: this category is now closed. The originally-planned
approach (triage all ~180 `grep -n "emit_expr(&args\[1\]" src/backend_llvm.rs`
hits) turned out to be the wrong search surface -- corrected methodology and
result below; the original writeup is kept underneath for the reasoning trail.

**Corrected method**: the real question isn't "which codegen sites call
`emit_expr(&args[1], ...)`" but "which checker functions accept an index/count
argument via a bare `index.ty().is_integer()` check and pass its *raw* checked
type through, vs. which ones call `coerce_checked(..., &Type::I64` or `&Type::
U64, ...)` (inserting an actual widening cast into the typed IR before codegen
ever runs)." The latter is safe by construction regardless of what codegen
does with it. `grep -n "is_integer()" src/checker.rs` returns exactly 11 hits
total (not ~180) -- small enough to triage exhaustively in one pass:
- `clone_at` -- already fixed, BUG-138.
- `set` / `set_mut` (`check_set_builtin`) -- **found vulnerable, fixed as
  BUG-141**: the index arg reached `backend_llvm.rs`'s generic vec-builtin
  call site with its raw width, producing a `call` argument that didn't match
  the callee's hard-coded `i64 %i` parameter. Confirmed via `opt -passes=
  verify` that LLVM's verifier actually accepts this (implicit same-symbol
  call-type bitcast is legal IR) and that it happened to still compute correct
  results on this host/LLVM version (x86-64 register zero-extension + LLVM's
  own per-operand CC lowering rescued it) -- not a proven wrong-answer bug in
  practice, but a genuinely invalid declared-vs-actual IR shape worth closing
  on the same footing as BUG-138 rather than leaving it fragile against a
  different target/LLVM version. Fix: route the index arg through the same
  `widen_index_to_i64` helper BUG-138 introduced, special-cased for `set`/
  `set_mut` at that call site.
- `Index` / `IndexAssign` (the `xs[i]` read/write path, checker.rs lines
  21952 and 14007) -- confirmed SAFE independently: both already widen at the
  *backend* level via a pre-existing `widen_index_to_64` helper (predates
  BUG-138; `clone_at`/`set_mut` simply never called it). No bug here.
- `for`-loop range bounds (checker.rs line 14509) -- both bounds get
  `promoted_integer_type` + `coerce_numeric_operand`'d to a shared type; the
  extreme-value gotcha there is BUG-140's already-fixed literal-typing issue,
  not a width-mismatch bug.
- The remaining ~6 hits are generic binary-op / cast / struct-field integer
  checks, unrelated to indexing.

**Why the C backend needed no equivalent fix**: checked `set_mut`'s C
signature (`backend_c.rs`) -- a plain `int64_t i` parameter. C's implicit
integer promotion at the call site correctly widens any narrower argument
type; there's no C-level equivalent of LLVM IR's strict textual-type-matching
requirement that BUG-138/BUG-141 both hit. The C backend is structurally
immune to this whole bug class, not just untested for it.

**Net result**: only 2 of the 11 real candidates were vulnerable (`clone_at`,
`set`/`set_mut`), both now fixed (BUG-138, BUG-141). The generic 8-named-
builtin list this category originally proposed checking (`push`, `pop`,
`insert`, `swap_remove`, `set`, `set_mut`, `get`, `get_mut`) turned out to be
mostly moot: `push`/`pop` take no index at all, `insert`/`swap_remove` were
already safe via `coerce_checked`, and there's no standalone `get`/`get_mut`
builtin in this language (indexing is the `xs[i]` operator, covered above).
Regression tests: `set_mut_with_u32_index_widens_to_i64_in_llvm_ir` and
`set_consuming_with_u16_index_widens_to_i64_in_llvm_ir` (src/lib.rs);
`set_mut_with_narrow_index_types_writes_the_correct_slot_on_both_backends`
(tests/run_end_to_end.rs). Full writeup in `docs/TODO_CURRENT.md`'s BUG-141
section.

<details>
<summary>Original (2026-08-07) category A writeup, kept for the reasoning trail</summary>

**Where this comes from**: BUG-138 — a `u32`-typed index reaching `clone_at`
produced a GEP whose declared index type (hard-coded `i64` in the format
string) didn't match the actual `i32` operand; `lli` rejected the malformed
IR outright. The fix found and patched 9 call sites sharing the exact same
unguarded `let idx = emit_expr(&args[1], ...)` pattern in `backend_llvm.rs`
(`clone_at`'s Array and Vec branches, `vec_remove_at`, and the
`simd_load`/`simd_store`/`simd256_load`/`simd256_store`/`simd512_load`/
`simd512_store` family) — but those 9 were found by grepping for one
*exact* textual pattern, not by systematically checking every builtin that
takes an index/count/length argument. A `grep -n "emit_expr(&args\[1\]"
src/backend_llvm.rs` today returns ~180 hits — the overwhelming majority are
NOT index parameters (most of `args[1]` across ~150 different builtins is an
arbitrary second argument: a divisor, a comparator, a format flag, a second
operand). **The real first step here is triage, not blind fixing**: for each
hit, check (a) does this builtin's signature document/accept a non-`i64`
integer type for this parameter (the checker is permissive — it accepts any
integer type for most index-shaped builtin parameters, confirmed during
BUG-138's investigation), and (b) does the emitted C or LLVM IR use this
value in a context where its exact bit-width matters (a GEP index, a shift
amount, a bounds comparison against a hard-coded-width variable)?

Concrete candidates worth checking first (Vec/array mutators most likely to
share BUG-138's exact shape, since they're semantically closest to
`clone_at`/`vec_remove_at`): `push`, `pop`, `insert`, `swap_remove`, `set`,
`set_mut`, `get`, `get_mut`, and the C-backend equivalents of all of the
above (the LLVM backend was BUG-138's confirmed victim; the C backend uses a
completely different codegen path — `backend_c.rs` — and hasn't been
checked at all for an analogous issue, e.g. a `u8`/`u16` index truncating or
sign-extending incorrectly in raw C pointer arithmetic).

Repro shape (adapt per builtin):
```vani
fn main() -> i64 {
  let xs: Vec<i64> = vec(10, 20, 30);
  let i: u8 = 1;             // try u8, u16, u32 — each is a DIFFERENT LLVM
                              // storage width (i8/i16/i32), each is a
                              // separate potential mismatch against a
                              // hard-coded i64 GEP slot
  return get(ref xs, i);     // swap in each candidate builtin
}
```
Run `vanic emit --backend=<c|llvm>` and inspect the raw output directly for
each — don't just run it and check the exit code, since (per BUG-138) the
failure mode is `lli`/`llc`/`cc` REJECTING malformed output outright, which
`vanic run`/`vanic check` alone won't always surface clearly as "here's the
exact type mismatch."

</details>

## B. Same-scope shadowing beyond plain scalar `let` (🔴 high) -- CLOSED 2026-08-08, BUG-142

**Update (2026-08-08)**: this category is now closed. All 6 originally-
listed candidates were triaged by reading each construct's own
scope-handling code in checker.rs (not just testing blind repros):

- `if let` / `while let` payload bindings, `match` arm bindings, for-loop
  induction variables -- all confirmed SAFE: each calls `env.push_scope()`
  immediately before inserting its binding via `insert_current`, so the
  binding lives in a fresh child scope that's never the SAME scope object
  as anything outside the construct. BUG-137/BUG-141's whole bug class
  requires a literal same-scope collision; a nested scope's shadow of an
  outer name is normal, already-correct lexical scoping, unrelated to it.
- Closure parameters -- confirmed SAFE: closures are lambda-lifted to
  top-level `__anon_fn_<N>` functions (`lambda_lift_program`) before the
  main checker ever runs; each gets checked with a brand-new `Env` from
  scratch, with no shared scope stack with the enclosing function at all.
- **Function parameters vs. module-level `const`s -- found genuinely
  vulnerable, fixed as BUG-142.** `check_function` seeds every top-level
  `const` into the function's root scope (T4.15) and then, with no
  `push_scope()` in between, inserts each parameter AND checks the
  function body's own top-level statements at that *same* scope depth.
  This produced two symptoms sharing one root cause: (a) a parameter
  sharing a const's name got a spurious "parameter already defined"
  diagnostic, and (b) worse, a top-level `let`/`let (a, b)` shadowing a
  same-named const hit the exact same-scope `Reassign` mechanism BUG-137
  extended to tuple-destructure -- but a const has no backend-emitted
  declaration to reassign into. Confirmed via a direct repro (forced onto
  the tree backends, same "SSA's fresh numeric naming sidesteps it"
  situation as BUG-137) to **panic tree-LLVM outright**
  (`unreachable!(): checker: reassign to undeclared binding`) and fail to
  compile on tree-C (`'v_N' undeclared`) -- a more severe failure mode
  than BUG-137's own "clean cc redefinition error." Fixed by gating all
  three sites (the duplicate-param check, `Stmt::Let`'s shadow decision,
  `Stmt::LetTuple`'s shadow decision) on the existing binding's `is_const`
  flag -- a const-shadow now always produces a fresh declaration, matching
  `check_function`'s own doc comment's original intent ("function-scoped
  `let` bindings shadow [consts] naturally").

Regression tests: `let_shadowing_a_top_level_const_declares_fresh_not_
reassign`, `let_tuple_shadowing_a_top_level_const_declares_fresh_not_
reassign`, `function_param_sharing_a_top_level_const_name_is_accepted`,
`real_duplicate_parameter_is_still_rejected` (src/lib.rs);
`let_and_param_shadowing_a_top_level_const_run_correctly_on_tree_backends`
(tests/run_end_to_end.rs). Full writeup in `docs/TODO_CURRENT.md`'s
BUG-142 section.

<details>
<summary>Original (2026-08-07) category B writeup, kept for the reasoning trail</summary>

**Where this comes from**: BUG-137 — `let (q, r) = f(...); let (q, r) =
f(...);` (tuple-destructure, same names, same scope) compiled fine on
LLVM/SSA-C but failed to build on tree-C, because the tuple-destructure
lowering never got the same-scope-shadow → `Reassign` treatment the plain
`Stmt::Let` handler already has (checker.rs, `check_one_stmt`'s `Stmt::Let`
arm: `env.current_get(name)` detects a same-scope prior binding and emits
`TypedStmt::Reassign` instead of a second `TypedStmt::Let`). That specific
fix only touched `Stmt::LetTuple`. Every OTHER binding-introducing statement
shape needs the same check — has anyone confirmed each of these actually
goes through the same safe path, or just assumed it does because tuple-`let`
was the only one that broke?

Candidates, each worth its own repro on tree-C specifically (force it via an
unrelated `#[no_mangle] fn`, since `parallel: bool`/tuple-destructure/etc.
being SSA-unsupported was what routed BUG-137's repro there in the first
place — don't assume a candidate naturally reaches tree-C without forcing
it):
- `if let K.Some(x) = opt { ... }` followed later in the SAME enclosing
  scope by a second, unrelated `if let K.Some(x) = opt2 { ... }` — does `x`
  get the same Reassign treatment, or does each `if let` open its own
  scope that makes this moot (worth confirming either way, not assuming)?
- `while let` pattern bindings, same question.
- `match` arm bindings shadowing an outer name (`let x: i64 = 1; match y {
  K.Some(x) then { ... } }` — is the arm's `x` a fresh nested-scope binding
  already, sidestepping the issue, or does it interact with the outer `x`?).
- A closure parameter shadowing an outer local (`let x: i64 = 1; let f = fn(x:
  i64) -> i64 { return x; };`).
- A `for i in 0..3 { ... }` loop's induction variable shadowing an outer
  `let i: i64 = 5;` declared just before it.
- A function parameter shadowing a module-level `const`.

</details>

## C. Compiler-emitted-pragma + boundary-value UB audit (🔴 high)

**Where this comes from**: BUG-140 — `parallel for i from i64::MIN to 4 {
... }` traps correctly on LLVM but the C backend's `#pragma omp parallel
for` silently executed ZERO iterations, because GCC's OpenMP canonical-loop
trip-count computation (`end - start`, internally signed arithmetic) is
undefined behavior when the true iteration count overflows the loop
variable's type. Fixed with a checked-subtraction guard specific to that one
`_Pragma("omp parallel for ...")` emission site. `backend_c.rs` emits at
least one OTHER `_Pragma`: `_Pragma("GCC ivdep")` for non-parallel `for`
loops (the `else` branch right next to the fix). Has anyone confirmed
`ivdep` doesn't have an analogous GCC-internal UB for the same boundary
values? (Initial informal check during BUG-140 suggested a PLAIN sequential
loop — no pragma at all — runs the extreme-bound case correctly; `ivdep` is
a *vectorization* hint, a different GCC subsystem than OpenMP's trip-count
precomputation, so it may or may not share the bug — this needs its own
standalone C probe, the same technique that confirmed BUG-140's root cause,
not an assumption either way.)

More broadly: the underlying gotcha here is a specific C literal-typing
footgun — `-9223372036854775808` (i64::MIN spelled as a bare literal) is
lexically a positive constant that doesn't fit `long long`, so C types it
`unsigned long long` before negating, producing a value that happens to
still be BIT-CORRECT once stored into an `int64_t` but has triggered at
least one real, confirmed miscompilation (BUG-140) when GCC's own internal
lowering does further signed arithmetic on it before that storage happens.
Grep `backend_c.rs` for every OTHER site that could plausibly emit this
exact literal shape from a user-supplied `i64::MIN`-valued expression (not
just `parallel for`'s start bound) — array bounds, `#[bounded(N)]`'s depth
counter, default/sentinel values, anywhere a signed extreme value could
reach emitted C source as a bare literal rather than going through a
runtime-computed variable.

## D. Type-existence validation: gaps beyond BUG-139's three sites (🟡 medium)

**Where this comes from**: BUG-139 — the checker never validated that a
struct field / enum variant payload / function parameter-or-return type
actually names a real declared type, as long as the bogus type was never
constructed. Fixed with a recursive validator (`type_references_unknown_name`
in `checker.rs`) wired into exactly those three declaration sites. The
validator itself is general (handles all ~61 `Type` variants correctly,
verified against the full example corpus) — the question is whether it's
wired into EVERY site that should call it, or just the three the original
localfuzz finding happened to touch.

Candidates:
- A generic type instantiation's own name (`Vec<BogusGeneric<i64>>` —
  `Type::Apply { name, args }`; the validator's `Apply` arm checks `name`
  against `struct_names`/`enum_names`, but confirm this actually fires for
  a generic struct/enum name that was NEVER declared at all, as opposed to
  one that exists but fails monomorphization for an unrelated reason).
- `interface Foo { fn bar(self: BogusType) -> BogusType; }` — are interface
  method signatures validated the same way regular function signatures are,
  or does `InterfaceDecl`/`InterfaceMethod` skip the check entirely (BUG-139's
  fix touched `program.functions` and `program.structs`/`program.enums`
  directly — `program.interfaces` was never in scope for the fix itself,
  only as a lookup TARGET for `Type::Object` validation).
- A `Closure(Vec<BogusType>) -> i64`-typed LOCAL variable annotation
  (`let f: Closure(BogusType) -> i64 = ...;`) — BUG-139 wired the validator
  into top-level struct fields/enum variants/fn signatures; local `let`
  annotations were never touched.
- A `methods on BogusType { ... }` block or `implement BogusIface for
  BogusType { ... }` — do these get the same validation, given they're
  processed through a different hoisting pass (`hoist_methods_into_functions`)
  than plain function declarations?

## E. `parallel for` + `reduce` correctness audit beyond the one fixed shape (🟡 medium)

**Where this comes from**: BUG-140 found ONE silent-wrong-answer shape in
`parallel for` (an extreme start bound). The feature has several other
edge cases nobody has specifically confirmed correct on both backends with
real hand-computed expected values:
- An empty range (`parallel for i from 5 to 5 { ... }` or `from 5 to 3`) —
  does each reduction operator correctly leave the accumulator at its
  initial value (not, say, some garbage or a spurious trap), on both
  backends?
- A single-iteration range — does the reduction correctly skip any
  combine-step machinery and just use that one iteration's value?
- `reduce` with `min`/`max`/bitwise operators specifically (not just the
  already-tested `*` from BUG-140's repro) under the SAME extreme-bound
  shape BUG-140 fixed for `*` — was the fix (the checked-subtraction guard)
  applied unconditionally before the loop regardless of which reduction
  operator is in use, or could a different operator's code path bypass it?
  (Should be unconditional per the fix's own placement — worth a direct
  confirmation test per operator rather than assuming from reading the
  diff.)
- Nested `parallel for` (one inside another, or inside a plain `for`) —
  given neither backend does real threading yet (`ir.rs`'s own comment:
  "Backends today still lower this as a sequential for loop"), does nesting
  still compute the mathematically correct answer, or does something about
  the reduction-variable/pragma machinery assume non-nesting?

## F. Localfuzz harness's own false-positive surface (🟢 low priority — process improvement, not a compiler bug)

**Where this comes from**: found during the 2026-08-07 77-item backlog
triage — one candidate's "crash" was the harness's own `CRASH_MARKERS`
substring-match heuristic (`tools/localfuzz/harness.py`) tripping on the
literal text `RUST_BACKTRACE` appearing in the FUZZED SOURCE ITSELF (quoted
verbatim in a normal parse-error diagnostic), not an actual Rust panic. The
harness's `is_crash()` does a dumb `any(m in result["stderr"] for m in
CRASH_MARKERS)` check — any qwen-hallucinated garbage source text containing
one of `CRASH_MARKERS`' literal strings ("panicked at", "SIGSEGV", "internal
compiler error", etc.) as DATA rather than an actual signal could produce
the same false positive again. Not a vani-compiler bug and not blocking —
worth a small harness hardening (check the child's actual OS-level
termination signal via `proc.returncode < 0` on POSIX, rather than /
in addition to text-matching) the next time someone is touching
`harness.py` for an unrelated reason. Low priority; listed here so it
isn't silently forgotten, not because it needs its own session.

---

## Process (mirrors both closed audit docs' own process sections)

- Work top-down by priority; category A is the highest-value pick (a
  confirmed sibling bug class with a known, narrow fix pattern already
  proven to work — `widen_index_to_i64` — ready to reapply).
- Batch ~3 confirmed fixes before a full local `cargo test --release` run +
  commit/push, per this project's established CI-driven workflow.
- A "clean pass" (repro compiles and runs correctly, no bug found) is not
  wasted effort — add the regression test anyway, per this project's own
  convention that a negative result still needs to be locked in so it can't
  silently regress later.
- If a category turns out to need more than the estimated single-session
  effort (category A's triage step in particular could balloon if many of
  the ~180 `args[1]` sites turn out to be genuine index parameters), check in
  with the user before expanding scope rather than silently ballooning the
  pass — this project's own history (BUG-116, BUG-121/122, BUG-139) has
  repeated examples of "the real fix is bigger than it looks once you
  start," and the established pattern is to flag that explicitly rather
  than decide alone.
- Once all categories here are closed (or explicitly ruled out with a
  documented negative result), this file's scope is exhausted the same way
  round 1's was — the next hunt after this one needs a genuinely new theme,
  not a category G.
