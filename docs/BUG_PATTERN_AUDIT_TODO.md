# Bug-pattern audit TODO

Proposed next-generation bug hunt, created 2026-08-05. The two prior
sweeps (`TESTING_MATRIX_TODO.md`'s container/nesting matrix and
`FEATURE_COMBINATION_GAPS_TODO.md`'s 49-row combination list) are both
fully closed (BUG-68 through BUG-104). Since then, `tools/localfuzz`
found BUG-105 through BUG-112 by *random mutation*, not systematic
feature enumeration -- and every one of those bugs is an instance of a
recurring **root-cause pattern**, not a one-off. This file proposes
tests targeted at those patterns directly, on the theory that hunting
by pattern will find more bugs per hour than hunting by feature-pair
enumeration at this point (the cheap feature-pair bugs are gone).

Method: same as the two closed sweeps. Write the `.vani` snippet,
`vanic check` + `vanic run` on both `--backend` values, compare
against a hand-computed expected value. Bug found -> root-cause, fix,
regression test pair, log in `docs/TODO_CURRENT.md` as `BUG-NNN`. Clean
pass or clean rejection -> still add the permanent test, it closes a
real coverage gap. Batch ~3 fixes before a full `cargo test --release
--workspace` run + commit/push + CI poll, per
`feedback_vani_ci_driven_workflow`.

**Note**: `tools/localfuzz` runs as a separate, roughly-concurrent
automated process against this same repo -- check `BUG-NNN` numbers
are still fresh and `git fetch`/rebase before committing, per
`feedback_vani_concurrent_localfuzz_process`.

---

## A. `requires`/`ensures` runtime-check consistency (🔴 highest priority)

**STATUS (2026-08-05): `requires` fully fixed as BUG-113 + BUG-116. `ensures`
still open (see bottom of this section) -- no confirmed repro yet, no
`TypedFunction` field for it to hook at all.** Full writeup in
`docs/TODO_CURRENT.md`'s "BUG-113/114/115/116" entry. Summary: BUG-113
fixed the tree-LLVM `abort()`-vs-`exit(3)` cosmetic issue for the
sqrt-forces-tree-fallback repro below; BUG-116 (found immediately
after, via the exact "needs a repro that stays on the SSA path"
question this section originally posed) confirmed and fixed the much
more severe deeper hypothesis -- `ssa.rs` genuinely never lowered
`f.requires` at all, and since the checker uses an unprovable
`requires` clause as a license to elide internal `checked` guards
(e.g. `requires b > 0` lets `a / b` skip its divide-by-zero guard),
the combination meant a violated precondition on the SSA path hit a
completely unguarded raw operation (confirmed via direct IR
inspection: a bare `sdiv`, zero guards anywhere). The repro that
finally reached the SSA path wasn't any of the "avoid sqrt" ideas
below -- it was simpler: `requires b > 0` itself (no builtin call at
all) plus an `id()` indirection to hide the violating literal from
the SMT verifier.

**Why this was the top pick**: `checker.rs`'s own
`verify_call_args_in_expr` has a comment admitting the design: an SMT
`Unknown`/`SkippedUnsupported` verdict on a `requires` clause (e.g. a
transcendental float call, or anything the solver can't decide) is
**silently accepted at compile time**, on the explicit assumption that
"the runtime `requires` check still fires." That runtime check is an
unconditional `assert(...)` emitted once per `requires` clause in
`backend_c.rs` (tree) -- but `ssa.rs` has **zero** references to
`function.requires`/`function.ensures` anywhere in its lowering, and
`ssa_path_supports` (`main.rs`) never gates a function out of the SSA
fast path just because it has a `requires`/`ensures` clause. This is
the exact same shape as BUG-110 (a safety guarantee silently dropped
on the default SSA path) and BUG-68 (an SMT-unencodable clause treated
as proven) combined.

**Already confirmed live, not yet fixed** (found while drafting this
list, hand-verified against the current `release` build):
```vani
fn safe_sqrt(x: f64) -> f64 requires sqrt(x) < 1000000.0; {
    return sqrt(x);
}
fn main() -> i64 {
    let y: f64 = safe_sqrt(1.0e30);
    print y;
    return 0;
}
```
`vanic check` accepts this silently (`sqrt` is opaque to Z3, verdict
`Unknown`, no diagnostic). `vanic run --backend=c` correctly aborts
with a clean assertion message. **`vanic run` (default, LLVM)
crashes with `lli`'s "PLEASE submit a bug report" stack-dump** --
raw `abort()` uncaught by any `exit(3)` wrapper, the exact ugly
crash-report symptom BUG-106 fixed for plain `assert` statements.
BUG-106's own writeup explicitly says the two `requires`-clause
`assert()`/`abort()` call sites (`backend_c.rs`, `backend_llvm.rs`)
were "deliberately left untouched" as "not a divergence" -- that
scoping call needs revisiting; this is the same misleading-crash
class BUG-106 exists to prevent.
Note: this specific repro forces tree-LLVM (both `sqrt` and every
other float transcendental are on `expr_ssa_supported`'s denylist in
`main.rs`), so it does NOT yet prove the deeper SSA-path hypothesis
below -- only the "requires assert still uses raw abort on LLVM"
regression.

**RESOLVED as BUG-116** (2026-08-05): the SSA-path repro didn't need
any of the avoid-the-denylist cleverness above -- `requires b > 0`
plus an `id()` indirection to hide the violating `0` from the SMT
verifier was enough, since it's the *elision*, not the clause
evaluation itself, that needed a builtin-free path. `ssa.rs`'s
`lower_function` now lowers every `f.requires` clause into a real
runtime guard at function entry (same shape/helper as
`TypedStmt::Assert`), and `main.rs`'s `ssa_path_supports` gates a
`requires`-bearing function off the SSA path if any clause itself
uses an SSA-unsupported construct (so the abandoned `sqrt`-in-
`requires` case now correctly falls back to tree instead of emitting
invalid IR, which is what actually happened when this was tried
without the gate). See `docs/TODO_CURRENT.md`'s BUG-116 entry for the
full writeup.

**Still open: `ensures`.** `TypedFunction` has NO `ensures` field at
all (confirmed via grep -- it's purely a `checker.rs`/SMT-time
concept per BUG-68's fix, never carried into the IR any backend
sees). So there is categorically zero runtime backstop for an
unprovable `ensures` clause on ANY backend, not just SSA. Unlike
`requires` (a single clean injection point at function entry), a
runtime `ensures` check would need to intercept every `return` site
and substitute the return value into the clause -- a bigger, separate
feature. No confirmed repro yet showing an actual wrong-answer
consequence (only the theoretical gap, by the same reasoning BUG-116
used for `requires`) -- worth constructing one the same way: find an
`ensures` clause the checker can't prove (Unknown verdict) but that
the function's OWN callers rely on as a fact (e.g. via a second
function whose own `requires` clause the first function's `ensures`
is meant to discharge), and see whether wrong output silently
propagates.

**Related doc-accuracy gap found 2026-08-05** (while sweeping
tutorials for staleness after BUG-113/115/116/117 -- pre-existing,
not caused by today's fixes): `tutorials/src/intermediate/
10b_runtime_errors_primer.md`'s Row 2 message-text table claims
specific diagnostic strings for `requires`/`ensures`/`invariant`/
bounds failures that don't match what actually ships:
- `requires`'s documented message
  (`"precondition failed: <expr> on call to <fn>"`, embedding the
  actual clause text) doesn't match BUG-116's real message
  (`"precondition violated in '<fn>'"` -- function name only, no
  expr text, no runtime expr-to-string pretty-printer was built for
  this).
- `ensures` and `invariant` rows document runtime failure messages
  for constructs that have **zero runtime enforcement on any
  backend** (confirmed via grep -- see the `ensures` paragraph
  above; same is true for `invariant`, no `"invariant failed"`/
  `invariant_fail` hit anywhere in any backend either).
- The bounds-check row's documented message
  (`"index out of bounds: i=<n>, len=<n>"`) doesn't match the actual
  emitted string (`"index out of bounds"`, no operands).
Not fixed -- would mean either implementing real `ensures`/
`invariant` runtime checks (ties into the still-open item above) or
walking back the tutorial's promised message formats, both bigger
edits than a token-budget-conscious pass warranted. Worth a dedicated
look alongside whoever picks up the `ensures` item.

## B. SSA-vs-tree safety-check parity audit (🔴)

BUG-108 (Vec bounds) and BUG-110 (checked arithmetic) were both "the
SSA fast path silently drops a runtime guard the tree path has."
`InstrKind::Index`/`IndexAssign` already carry a `checked: bool` flag
(good -- already covers Vec/Array bounds symmetrically with
`Binary.checked` post-BUG-110). Two follow-ups this pattern predicts:

- `ssa_pass.rs`'s `elide_bounds_in_function` and any SMT-elision pass
  flip `checked` from `true` to `false` when it can *statically*
  prove safety. BUG-110's own writeup flagged this as unaudited:
  "could `ssa_pass.rs` ever flip `checked` from `true` to `false` on
  a Binary instruction incorrectly?" -- write a targeted test: a
  provably-safe-looking case that's actually only safe under an
  assumption the elision pass doesn't fully verify (e.g. elision
  reasoning across a loop with a non-monotonic induction variable,
  or across a function-call boundary that mutates state the elision
  pass doesn't track). Also check the mirror case for
  `IndexAssign`'s `index_is_in_bounds_for_base`.
- Every OTHER runtime trap in the language -- not just
  arithmetic/bounds -- audited for the same "does the SSA path even
  implement this trap at all" question: enum-tag validity on `match`
  (exhaustiveness is compile-time, but is there a runtime tag-sanity
  check anywhere, and does it exist on SSA?), `HashMap`/`HashSet` key
  lookup failure modes, `Option`/`Result` unwrap-without-check
  builtins if any exist, `Channel` send-on-closed, `Mutex`
  double-lock/poisoning. **AUDITED, no gap found** (2026-08-06):
  enum-tag validity can't be violated -- there is no `i64 as Enum`
  cast anywhere in `checker.rs` (only the reverse, `Enum as i64`),
  and `raw_load`'s pointee is restricted to `i64` only in v1, so
  there's no way to construct an invalid tag bit-pattern and feed it
  to `match` in the first place; exhaustiveness is genuinely sound.
  `hashmap_get`/`btreemap_get` already return `Option<T>` (no
  unwrap-without-check builtin exists at all -- grepped, none
  found), so key lookup has no panic path to guard. `Channel` has no
  `channel_close` builtin (grepped, none found) and is a blocking
  single-slot rendezvous (send sets value+ready flag, recv
  spin-waits), so "send-on-closed" isn't a reachable state. Mutex
  double-lock detection (`compute_locks_params`,
  `extract_locked_mutex_name`) is a checker-level static analysis
  shared identically by both backends (not an SSA-vs-tree split) and
  is already explicitly documented as intentionally non-transitive
  ("today's check catches the direct case cleanly... future
  enhancement") -- a known, acknowledged limitation, not a silent
  gap in the BUG-108/110/119 sense.
- `#[bounded(N)]` recursion-depth guard (`backend_c.rs` line ~13317,
  seen while reading around the `requires` codegen above) -- is this
  emitted on the SSA path at all, or only tree? Same audit as A/above,
  same file region, cheap to check while already there. **RESOLVED
  as BUG-117** (2026-08-05, picked up separately as a low-risk
  follow-up after BUG-115 landed): both `backend_llvm.rs` (tree) and
  `ssa_backend_llvm.rs` (SSA) had the same raw `call void @abort()`
  -- both switched to `exit(3)`, regression tests added. See
  `docs/TODO_CURRENT.md`'s BUG-117 entry.
- SSA-vs-tree parity for `checked` **specifically inside a function
  with a `requires` clause** -- BUG-116 fixed the case where the
  precondition itself is never enforced, but didn't audit whether
  the checker's elision reasoning (marking an op `checked: false`
  because a `requires` clause "proves" it safe) is ITSELF always
  sound now that the precondition IS enforced. E.g. does the checker
  correctly re-derive elision facts across an `&&`/`||`-combined
  multi-clause `requires`, or a `requires` clause that only
  partially covers an op's safety (e.g. `requires b != 0` but the
  op is `a % b` where `a` could still be `i64::MIN` and `b` could
  still be `-1`, the other overflow case for signed division/rem)?
  Worth one repro targeting exactly that gap. **RESOLVED as BUG-119**
  (2026-08-06) -- turned out bigger than the elision-soundness
  question posed: the repro crashed identically with the `requires`
  clause removed entirely, proving no backend had ANY runtime guard
  for `MIN / -1`/`MIN % -1` at all (not an elision gap -- the
  divisor-only check literally couldn't see the numerator). All four
  backends fixed to validate both operands for signed Div/Rem. See
  `docs/TODO_CURRENT.md`'s BUG-119 entry.

## C. Recursive type-walk helper exhaustiveness audit (🟡)

BUG-107's root cause (`vec_element_has_user_struct` missing a
`Type::Box` arm) is the *sixth* documented instance of "a helper that
recursively walks `Type::` variants for C forward-declaration/typedef
ordering purposes has an incomplete match" (prior five: Closure,
Channel, Mutex/Guard/RwLock, HashMap, Vec128/256/512 -- see
`project_vani_testing_matrix_sweep_2026_08_02` memory's own
follow-up note). Grep `backend_c.rs` for every function matching on
`Type::` for this class of purpose (`c_leaf_type`,
`c_element_storage`, `format_declarator`, `vec_element_has_user_struct`
and siblings) and cross-check each one's arms against the full
`Type` enum, specifically for `Type::Box`, `Type::Tuple`, and
`Type::Object` wrapper cases (the ones most likely to be "forgotten"
since they're structurally different from the common Vec/Array/Struct
cases). Concrete test cases to try once candidates are found:
- `Vec<Box<Struct>>` (not `dyn Iface`) as a struct field -- does
  BUG-107's exact ordering bug recur for a concrete boxed struct?
  **AUDITED, no bug** (2026-08-06): compiles and runs correctly on
  both backends.
- `Tuple<Box<dyn Iface>>`, `HashMap<K, Box<dyn Iface>>`,
  `Option<Box<dyn Iface>>`, `Deque<Box<dyn Iface>>` as struct fields.
  **AUDITED, not reachable** (2026-08-06): all four are rejected
  before codegen by `checker.rs`'s struct-field-type whitelist
  (~line 1128 -- "v1 supports Copy types, OwnedStr, Vec<T>, [T; N]
  of Copy elements, Task, Atomic<T>, Mutex<T>, Channel<T, N>,
  Box<T>, and enum types as struct fields"), which deliberately does
  NOT include bare Tuple/HashMap/Option/Deque as field types at all
  when they carry non-Copy content -- so BUG-107's recursive-walk
  ordering bug class can't recur here; these never reach the
  C-forward-declaration codegen the bug lives in. A real gap, if
  any, would be "should the whitelist accept these" (a feature
  question), not a codegen-ordering bug.
- `Vec<Tuple<vec128<f64>, i64>>` as a struct field (Tuple-wrapping a
  SIMD type -- same "wrapper type forwards the check but the outer
  container's own helper forgets to recurse" shape). **AUDITED, no
  bug** (2026-08-06): compiles and runs correctly on both backends.

## D. Trap exit-code / message consistency matrix (🟡)

BUG-106 fixed `assert` exit-code/message parity across
{C-tree, C-SSA, LLVM-tree, LLVM-SSA}. BUG-108/110 added missing
guards but didn't necessarily verify the *message text*, only guard
*existence*, is consistent across all 4 cells. Build the full matrix
and check both dimensions (does it trap at all, and does the trap
look the same) for:
- Vec/Array bounds violation
- Integer overflow (Add/Sub/Mul), div-by-zero, Rem-by-zero
- Shl/Shr out-of-range shift count
- `requires`/`ensures` violation (see category A -- likely to fail)
- `#[bounded(N)]` recursion limit exceeded
- explicit `assert` with and without a message (already covered by
  BUG-106's regression tests -- use as the template for the others)

For each cell that's a genuine `--backend=c` vs LLVM divergence in
message text only (not existence), it's lower severity than A/B/C but
still worth a `docs/v1_limitations.md` note if intentionally
unfixable (e.g. `lli`'s own crash-report noise is accepted/
documented per BUG-108's own writeup -- don't re-litigate that one,
just confirm no NEW divergences exist elsewhere).

**RESOLVED as BUG-120** (2026-08-06) -- building this exact matrix (forcing the
tree-LLVM path via a `sqrt()` dummy to test each cell) immediately found a real,
NEW crash-report divergence: signed add overflow on tree-LLVM crashed `lli` with
the "PLEASE submit a bug report" banner while the identical program on SSA-LLVM
exited cleanly with `exit(3)` -- BUG-115's sweep fixed SSA-LLVM's checked-
arithmetic guards and the tree-LLVM bounds-check helper, but missed tree-LLVM's
OWN checked-arithmetic guards (Add/Sub/Mul/Div/Rem/Shl/Shr) entirely, plus four
Vec-mutator-builtin guards (`pop`, `swap_remove`, `insert`) that are tree-only
(SSA-denylisted, so had no SSA-LLVM sibling to catch the gap by comparison). All
9 sites fixed to `exit(3)`. Confirmed no further NEW divergences beyond this in
the matrix: `requires`/`#[bounded(N)]`/bounds/`assert` were already covered by
BUG-106/113/115/116/117; the tree-C-vs-SSA-C message-text wording difference
(`i64` vs `int64_t`) noted in BUG-119 remains, but is message-text-only (lower
severity, matches this section's own stated bar for "intentionally unfixable,
just document"). See `docs/TODO_CURRENT.md`'s BUG-120 entry for the full
writeup.

## E. Packed/special-layout element type audit (🟡)

BUG-109 (`Vec<bool>` literals: packed-bit storage vs a byte-addressed
fallback used by exactly one lowering path) is a "one element type
gets special storage, and one of the several codegen paths that
construct/read/write it forgot the special-casing" bug. `bool` is the
only element type this project special-cases for bit-packing today,
but the SAME bug shape (one construction path uses a different layout
assumption than the read/write paths) is worth checking for every
OTHER path that constructs a `Vec<bool>`, not just `let`-literal:
- `Array<bool, N>` literal (fixed-size, not `Vec`) -- same packed-vs-
  unpacked risk? **AUDITED, no bug** (2026-08-06) -- clean on tree-LLVM.
- A struct field of type `Vec<bool>`, literal-initialized inside the
  `StructLit` (different lowering path than a bare `let`).
  **RESOLVED as BUG-122** (2026-08-06) -- construction was actually
  fine; the READ side (`h.flags[i]`, a `FieldAccess`-based index) was
  the real gap, since the packed-bit `Index` special case only fired
  for a bare `Var(name)` base. Fixed; write side (`set(mut ref h.
  flags, i, v)`) confirmed already correct.
- `Vec<Vec<bool>>` (nested) -- does the outer Vec's per-element
  construction correctly delegate to the packed inner constructor?
  **RESOLVED as BUG-122** (2026-08-06) -- no, the nested-`vec(...)`-
  sub-expression codegen had no bool special case at all. Fixed with
  a new value-returning `emit_vec_bool_literal_value` helper.
- A function that directly `return`s a `vec(true, false, ...)`
  literal (no intermediate `let` at all) -- does that skip
  `emit_vec_bool_let_from_literal` entirely (its name suggests it's
  keyed off `let` specifically)? **AUDITED, no bug** (2026-08-06) --
  clean on tree-LLVM (routes through the same fixed sub-expression
  path as the `Vec<Vec<bool>>` case above).
- `HashMap<K, bool>` / `HashSet<bool>`-adjacent value storage, if any
  path constructs those from a literal collection of bools.
  **RESOLVED as BUG-121** (2026-08-06) -- a different bug shape (hard
  compile-time invalid-IR crash, not silent corruption): HashMap
  values store `bool` as `i8` (no bit-packing there at all, unrelated
  to Vec<bool>'s packing), but `Option<bool>`'s enum payload field is
  `i1` -- `insertvalue`ing a loaded `i8` register into it is invalid
  IR. Fixed across all 6 key-type-generic HashMap codegen functions.
  See `docs/TODO_CURRENT.md`'s BUG-121 and BUG-122 entries for full
  writeups. Category E is now fully audited.

## F. Non-ASCII identifier collision, extended scope (🟢)

BUG-105 fixed `sanitize_ident` collisions for function PARAMETER
names specifically (the repro that found it). The same sanitizer
(`function_name`, `local_name`) is used more broadly -- check for the
identical collision class in:
- Two struct fields with different non-ASCII names of equal
  sanitized-length inside the same struct (C struct member names,
  not `v_`-prefixed locals -- likely a DIFFERENT code path than the
  one BUG-105 fixed, worth checking it uses the same fixed sanitizer).
  **AUDITED, no bug** (2026-08-06): struct field names never route
  through `sanitize_ident` at all -- `emit_struct_bundle` emits the
  raw source identifier directly as the C member name, no lossy
  collapsing step, so there's no collision surface by construction.
  Confirmed with a regression test (two equal-length Burmese field
  names, both backends).
- Two enum variant names, same shape. **AUDITED, no bug** (2026-08-06):
  enum variants carry a pre-resolved integer `tag`; the C backend
  never emits a variant-name-derived C identifier at all. No
  sanitizer, no collision surface. Confirmed with a regression test.
- Two local variables (not params) with different non-ASCII names in
  the same function scope -- `local_name` is shared with
  `function_name` per BUG-105's writeup, so this is likely already
  fixed, but wasn't the ORIGINAL repro -- worth a dedicated
  regression test since it's a different call site. **AUDITED,
  already fixed** (2026-08-06) -- confirmed clean with a dedicated
  regression test at this call site.
- A non-ASCII TYPE name (struct/enum name itself, not a variable)
  colliding with another type name -- typedefs, not variables ,so a
  different sanitizer consumer again. **AUDITED, different finding**
  (2026-08-06): not a collision bug -- `struct_c_name`/`enum_c_name`
  also use the raw name (no sanitizer, no collision risk), but a
  non-ASCII struct/enum name can be DECLARED and never actually
  REFERENCED: `parse_type` doesn't accept a non-ASCII identifier
  token in type-annotation position at all (a parser-level gap,
  backend-independent -- both backends fail identically at parse
  time, not a C-vs-LLVM divergence). No example in `examples/
  language/*/` uses a non-ASCII type name, so unclear whether this
  is a deliberate v1 restriction or an oversight -- documented with a
  regression test (`non_ascii_struct_name_declares_but_cannot_be_
  used_as_a_type_annotation`, `src/lib.rs`) rather than fixed, since
  it's out of scope for a mangling-collision audit and the intended
  behavior isn't established either way.

Category F is now fully audited -- no new collision bugs found (BUG-105's fix
already covers every case where the C backend derives an identifier from a
non-ASCII source name via `sanitize_ident`; every OTHER identifier-emission
site turned out to use the raw name directly, with no collision surface at
all). One adjacent parser-level finding (non-ASCII type names) documented but
not fixed, pending a decision on intended behavior.

## G. Const-operand type-tracking gaps in SSA lowering (🟢)

BUG-111's root cause: `Operand::Const` has no `ValueId`, so any SSA
backend helper that infers a value's type via a `value_types` lookup
(keyed by `ValueId`) silently fails for a bare constant and falls
back to a wrong default. `const_operand_natural_type` fixed the ONE
call site found (`InstrKind::Cast`'s LLVM emission). Grep
`ssa_backend_llvm.rs`/`ssa_backend_c.rs` for other `operand_type(`
call sites with an `.unwrap_or_else`/`.unwrap_or` fallback and check
each one for the same "silently wrong default when given a bare
constant" risk. Concrete repro shapes to try, beyond `let x: f64 =
<int literal>;` (already fixed):
- An int literal passed directly as an `f64`/`f32` function
  ARGUMENT (not through a `let`). **AUDITED, no bug** (2026-08-06).
- An int literal as one element of a `Vec<f64>`/`Array<f64,N>`
  literal (`vec(1.0, 2, 3.0)` -- mixed literal forms in one
  container). **AUDITED, no bug** (2026-08-06).
- An int literal on one side of a float comparison inside an `if`
  (`if x > 0 { ... }` where `x: f64` and `0` is the bare int
  literal) -- does comparison codegen hit the same `operand_type`
  path as Cast? **AUDITED, no bug** (2026-08-06).
- An int literal as a struct field initializer for an `f64` field.
  **AUDITED, no bug** (2026-08-06).
- An int literal as the RHS of a float `+=`/`-=` compound-assignment.
  **N/A** -- this language has no compound-assignment operators at
  all (`x += 5;` is a parse error), so this repro shape doesn't apply.

**RESOLVED as BUG-123** (2026-08-06): none of the 5 shapes above found a bug,
but the same `operand_type(...).unwrap_or(...)` anti-pattern turned up at a
call site the list above didn't mention -- `intent_print_item`'s argument-
type dispatch, independently in BOTH `ssa_backend_c.rs` and
`ssa_backend_llvm.rs`. `print 5.5;` (bare float literal, no `let`) crashed
`lli` outright on SSA-LLVM (invalid IR: a float constant embedded on the
integer print branch) and silently printed `5` instead of `5.5` on SSA-C (the
wrong format specifier + truncating cast). Fixed both by deriving the
constant's own type from its `Const` variant instead of defaulting to `I64`.
Swept both files for every other `Operand::Const(_) => None` site -- 3 more
found, all already using the safe `.ok_or_else(...)` pattern (a real
`EmitError`, not a silent wrong default), left unchanged. See
`docs/TODO_CURRENT.md`'s BUG-123 entry for the full writeup. Category G is
now fully audited.

## H. Link-flag parity across build-target x backend matrix (🟢)

BUG-112: `vanic build`'s LLVM host-POSIX branch was missing `-lm`
that the near-identical C-backend link command and the LLVM
`is_cross` branch both already had. This is a 3 (bare_metal / cross /
host-POSIX) x 2 (C backend / LLVM backend) = 6-cell matrix that's
never been exhaustively diffed for flag parity. Concrete checks:
- For each of the 6 cells, list every linker flag the OTHER 5 cells
  use that this cell doesn't, and confirm each omission is
  intentional (e.g. `bare_metal` correctly omitting libc/libm) rather
  than a copy-paste gap.
- Specifically check `-lpthread`/threading-runtime flags across all
  6 cells for a program that uses `task`/`Mutex`/`Channel` --
  BUG-112's own writeup only checked `-lm`; the exact same
  three-branch code structure could have the identical gap for
  `-lpthread` in a branch nobody's tested a threading program against
  yet.
- Run an actual `vanic build` (not just `vanic run`) for a
  math-using AND a threading-using program on every reachable
  cell (host-POSIX x both backends is easy; cross/bare_metal need
  the appropriate `--target=`), not just JIT/interpret it -- BUG-112
  was invisible to `vanic run` entirely (`lli` auto-resolves libm),
  so `vanic build`-specific coverage is the only way to catch this
  class.

---

## Process notes

- Categories A and B are the highest-value: they're about whole
  classes of safety guarantees potentially silently absent on the
  default (SSA) execution path, which is what BUG-108/110 already
  proved happens and is the most severe bug class this project has
  found to date (silent wrong/unsafe behavior on the path most users
  actually hit, not a compile error or a crash).
- Unlike the two closed sweeps (feature x feature enumeration), this
  list is organized by *root-cause pattern*, found by reading the
  bug corpus (`BUG-68` through `BUG-112` in `docs/TODO_CURRENT.md`)
  for recurring shapes rather than by walking the feature matrix.
  Expect a higher bug-per-repro hit rate than the exhausted
  combination-matrix approach, at the cost of needing more
  code-reading (not just spec-reading) to construct each repro.
- Category A's `safe_sqrt` repro above is ready to become BUG-113
  (or whatever the next free number is once localfuzz's concurrent
  numbering is checked) -- root-cause and fix it first before
  spending time on the still-open SSA-path variant, per this
  project's usual "fix what's confirmed, then keep digging" cadence.
