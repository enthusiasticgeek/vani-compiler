# Unresolved gaps TODO

Created 2026-08-07. A holding list for confirmed-real gaps found
during the BUG-118..125 bug-pattern-audit session and its immediate
follow-up (tutorial-accuracy sweep + a pass over `tools/localfuzz`'s
then-current unmatched findings) that were **found but deliberately
not fixed**, so the next session (or a human) can pick them up
without re-discovering them from scratch.

Next free `BUG-N`: **BUG-134** (re-check before committing -- see
`feedback_vani_concurrent_localfuzz_process` memory, another
automated process lands fixes to this repo's `main` too). BUG-126
(item A1), BUG-127 (item A2), BUG-128 (item A3), BUG-129 (item C1),
BUG-130 (item C2), BUG-131 (item C5), BUG-132 (item C4), and BUG-133
(item C3's `ensures` half) were all fixed 2026-08-07 -- section A is
fully closed out; only C3's `invariant` half remains open --
`ensures`'s own runtime-enforcement half is done, `invariant` needs
its own follow-up pass (two guard sites per loop instead of one,
break/continue handling, per-iteration performance).

Method for picking one up: reproduce the minimal repro given (or the
raw localfuzz finding for the ones that don't have one yet), root-
cause against a fresh `main` checkout, fix, add a regression test,
run the full `cargo test --release` suite, then log it here as
`## BUG-N FIXED` following this repo's established writeup style
(see any entry in `docs/TODO_CURRENT.md` for the template).

---

## A. Real, confirmed bugs with clean minimal repros (highest priority)

These three were found chasing `tools/localfuzz`'s unmatched-cluster
digest from 2026-08-07 down to root cause, then reduced to minimal,
mutation-free repros to confirm they're real (not fuzzer artifacts).
A1 is fixed as BUG-126, A2 is fixed as BUG-127, A3 is fixed as BUG-128 --
section A is fully closed out.

### A1. `let`-shadowing corrupts codegen -- different failure per type, per backend -- **FIXED as BUG-126 (2026-08-07)**

Root cause was NOT shadowing-specific, and NOT a storage-reuse/alloca
mixup as originally hypothesized below (kept verbatim for the
record) -- it was `TypedStmt::Reassign` codegen missing an
Array arm (C backend) and wrongly assuming every reassignable
binding has a real alloca address (LLVM backend, broke specifically
for `ref T`). Same-scope `let`-shadowing merely happens to desugar
to a `Reassign` node (see `checker.rs`'s "Same-scope let ->
Reassign" comment) -- a PLAIN `xs = [...]` / `r = ...;` reassignment
(no shadowing at all) reproduces both bugs identically. Full writeup,
fix, and regression tests: see `## BUG-126` in `docs/TODO_CURRENT.md`.
Original hypothesis and repros kept below for traceability.

Redeclaring the same variable name via a second `let` in the same
scope (legal in this language -- shadowing, not an error) breaks
codegen. Confirmed with two independent minimal repros:

**Array-typed: C backend fails to compile.**

```vani
fn main() -> i64 {
  let xs: [i64; 5] = [1, 2, 3, 4, 5];
  let xs: [i64; 5] = [10, 20, 30, 40, 50];
  print xs[0];
  return 0;
}
```

`vanic run --backend=c` (every time, not a fuzzer-only artifact):

```
cc failed while compiling ...:
.../vanic-....c: In function 'fn_main':
.../vanic-....c:421:8: error: assignment to expression with array type
  421 |   v_xs = ((int64_t[5]){ 10, 20, 30, 40, 50 });
      |        ^
```

Root-cause hypothesis (UNCONFIRMED -- see actual root cause above):
the second `let xs = ...`'s codegen appears to treat the
redeclaration as an ASSIGNMENT into the FIRST `xs`'s existing storage
(`v_xs = (compound literal)`) rather than allocating fresh,
independent storage for the new binding -- which is invalid C for an
array type (C arrays aren't assignable via `=`; needs `memcpy` or a
per-element loop, the same shape the non-shadowed `let`-array-literal
path presumably already uses correctly).

**`ref T`-typed: LLVM backend silently returns a wrong (garbage-
looking) value; C backend is correct.**

```vani
struct Point { x: i64, y: i64 }
fn shared(p: ref Point) -> ref Point { return p; }
fn area(p: ref Point) -> i64 { return p.x * p.y; }
fn main() -> i64 {
  let pt: Point = Point { x: 7, y: 9 };
  let r: ref Point = shared(ref pt);
  let r: ref Point = shared(ref pt);
  print area(r);
  return 0;
}
```

`vanic run` (LLVM, default): prints a huge garbage-looking number
(observed `1266547254094656`, varies by run -- looks like an
uninitialized-memory or aliased-pointer read), expected `63`.
`vanic run --backend=c`: correctly prints `63`.

Root-cause hypothesis (UNCONFIRMED -- see actual root cause above):
likely the SAME underlying "second `let` with the same name reuses
the first's storage/alloca incorrectly" mechanism as A1's array case,
just manifesting as a silently-wrong pointer read on LLVM instead of
a hard C compile error (since `ref T` is pointer-sized, a
storage-reuse bug wouldn't necessarily trip any C-level type check
the way an array literal's compound-literal assignment does).

**Suggested investigation starting point** (SUPERSEDED -- actual fix
was in `Reassign`, not `Let`): find wherever `TypedStmt::Let` codegen
decides the target `alloca`/storage slot for a NEW `let` binding, in
both `backend_llvm.rs` and `backend_c.rs` (and their SSA
counterparts) -- check whether it's keyed purely by NAME (which
would incorrectly reuse a shadowed name's old slot) vs. by a unique
per-declaration ID the checker/IR should already be assigning. This
is exactly the kind of "one construction path uses a different
assumption than the others" root-cause shape this project has hit
repeatedly (BUG-107, BUG-109, BUG-121, BUG-122) -- likely worth
checking EVERY type category (struct, `Vec<T>`, `OwnedStr`, tuples,
enums), not just array and `ref T`, once the real mechanism is found.
(BUG-126's actual fix DID check all these types via direct repros --
struct, `Vec<i64>`, `Str`, tuple, enum, and `ref Vec<i64>` were all
confirmed unaffected; only Array-on-C and ref-on-LLVM had the bug.)

**Original localfuzz findings** (both in `~/source/vani-compiler-localfuzz`,
both fuzzer-mutated by literally duplicating one `let` line -- the
minimal repros above strip the mutation noise):
- `tools/localfuzz/findings/20260806-213628-backend-divergence-4b9c65a80d/`
  (`ref` case, from `examples/language/english/path_c_ref_returns.vani`)
- `tools/localfuzz/findings/20260807-040921-backend-divergence-08a08388e1/`
  (array case, from `examples/language/malayalam/for_loops.vani`)

### A2. Checked-arithmetic elision may be unsound inside a loop (LLVM hangs, doesn't trap) -- **FIXED as BUG-127 (2026-08-07)**

The hypothesis below was directionally right (an elision-soundness bug) but named the
wrong file -- it's `checker.rs`, not `ssa_pass.rs`, and the mechanism is specific: two
existing call sites deliberately keep facts about a reassigned variable alive across a
loop body (for a separate loop-invariant-preservation check), and the overflow-elision
pass used that same not-yet-invalidated fact set, "proving" the guard safe using a fact
(`n == 0`) that was only true on the FIRST iteration. Full writeup, fix, and regression
tests: see `## BUG-127` in `docs/TODO_CURRENT.md`. Original hypothesis and repro kept
below for traceability.

```vani
fn main() -> i64 {
  let n: i64 = 0;
  while n < 100 {
    if n == 5 {
      break;
    }
    n = n + -9223372036854775808;
  }
  assert n == 5;
  print "OK", n;
  return 0;
}
```

`vanic run` (LLVM, default): **hangs forever** (confirmed with a
5-second `timeout`, killed at the deadline every time -- not a slow
computation, a genuine infinite loop). `vanic run --backend=c`:
correctly traps with an overflow diagnostic and exit code 1.

Hypothesis (UNCONFIRMED -- see actual root cause above): `n + i64::MIN` inside the loop should hit the SAME
checked-add overflow guard BUG-119/120 already verified works
correctly for a non-looping repro -- so something about being
inside a LOOP specifically causes the SMT/elision pass
(`ssa_pass.rs`, or the checker's own elision reasoning) to
INCORRECTLY mark this `Binary { checked: true }` as `checked: false`
(provably safe), when it demonstrably is not. If the check is
elided, the raw wrapping `add` silently computes `i64::MIN` then, on
the NEXT iteration, `i64::MIN + i64::MIN` wraps to exactly `0` (since
`-2^63 + -2^63 ≡ 0 (mod 2^64)`) -- producing a 2-cycle oscillation
between `0` and `i64::MIN` that never reaches `5` and never exceeds
`100`, hence the infinite loop. This EXACTLY matches a risk
`docs/BUG_PATTERN_AUDIT_TODO.md` category B flagged by name and
never actually tested: *"could `ssa_pass.rs` ever flip `checked`
from `true` to `false` on a Binary instruction incorrectly? ...
elision reasoning across a loop with a non-monotonic induction
variable."* This repro IS exactly that shape (non-monotonic:
`n` goes `0 -> i64::MIN -> 0 -> i64::MIN -> ...`, never
monotonically increasing despite the `n < 100` loop guard looking
like it should terminate).

**Suggested investigation starting point** (this DID confirm the elision-soundness
diagnosis): dump the LLVM IR for this repro (`vanic emit`) and check whether the `add`
inside the loop is the raw `add` opcode or the checked `@llvm.sadd.with.overflow`
intrinsic call -- that tells you immediately whether this is an elision-soundness bug
(checked flag wrongly false) or something else entirely (e.g. the guard fires but its
own `exit`/`abort` is unreachable due to a different loop-structuring bug).

**Original localfuzz finding**: `tools/localfuzz/findings/
20260806-231108-run-crash-7564fbed6b/` (from
`examples/language/khmer/early_exit.vani`, fuzzer-mutated: the
loop's `n = n + 1;` increment became `n = n + -9223372036854775808;`).

### A3. Checker accepts use-before-declaration inside an `async fn`'s `try`-desugared early-return body -- **FIXED as BUG-128 (2026-08-07)**

Root cause: the v3.1 state-machine transform (`try_v31_transform` in `parser.rs`)
splits the body into per-suspend-point `if state_tag==N` segments and unconditionally
promotes crossing locals to Task struct fields, BEFORE `checker.rs` ever sees the
function -- both the reachability check (dead code now lands in its own reachable
branch) and the use-before-declare check (the name becomes an always-valid `t.n`
field access) are defeated by the transform itself, not by any single missing check.
Full writeup, fix, and regression test: see `## BUG-128` in `docs/TODO_CURRENT.md`.
Original repro and investigation notes kept below for traceability.

```vani
enum FetchResult { Ok(i64), Err(i64) }

fn maybe_size(mode: i64) -> FetchResult {
  return match mode {
    0 then FetchResult.Ok(64),
    _ then FetchResult.Err(0 - 99)
  };
}

async fn fetch(fd: i64, mode: i64) -> FetchResult {
  let size: i64 = try maybe_size(mode);
  return FetchResult.Ok(n);              // `n` used here...
  let n: i64 = io_recv_async(fd, size);  // ...but only declared HERE
}
```

`vanic check` on this (or the full original repro, see below)
returns `ok` -- the checker ACCEPTS it. In a PLAIN (non-async, no
`try`) function, the exact same use-before-declare shape is
correctly rejected with `error: unknown variable 'n'` PLUS
`error: unreachable statement after a control-flow exit` (confirmed
directly, trivial repro: `fn main() -> i64 { return n; let n: i64 = 5; }`).
So this is specifically an async-fn-plus-`try`-desugaring gap, not a
general checker hole.

Consequence: the full original program (see the localfuzz finding
below) HANGS on both backends (`timed_out: true`, empty
stdout/stderr) when actually run -- consistent with codegen
producing something that references uninitialized/garbage storage
for `n`, or the `try`-desugaring producing a malformed control-flow
shape that loops.

**Suggested investigation starting point**: `desugar_try_in_v31_body`
(mentioned in the repro's own header comment) or wherever `async fn`
bodies get their own separate reachability/scope-checking pass --
check whether it runs BEFORE or uses a different traversal than the
normal use-before-declare + unreachable-code checks that correctly
catch this in a plain function.

**Original localfuzz finding**: `tools/localfuzz/findings/
20260806-193817-run-crash-2767ef4c1c/` (from `examples/language/
english/echo_p24_try_keyword.vani` -- NOT obviously fuzzer-mutated;
the repro looks close to the real example file, worth diffing
against `examples/language/english/echo_p24_try_keyword.vani`
directly to confirm whether the SHIPPED example itself has this bug
or the fuzzer introduced the exact reordering).

---

## B. Localfuzz findings audited and explained -- NOT a bug, no action needed

Recorded so a future pass doesn't re-investigate these from scratch.

- `tools/localfuzz/findings/20260807-032718-backend-divergence-5e53ed35f9/`
  (from `examples/language/romanian/basics.vani`): C backend exits 1
  with `"integer overflow in int64_t mul"`, LLVM backend exits 3
  silently. This is the ALREADY-DOCUMENTED, ALREADY-ACCEPTED C-vs-LLVM
  asymmetry `docs/TODO_CURRENT.md`'s BUG-119/120 entries and
  `tutorials/src/intermediate/10b_runtime_errors_primer.md` (updated
  2026-08-07) describe: the C backend still raises a raw `abort()`
  with a message for overflow/bounds/div-by-zero/shift (never
  converted to `exit(3)`, since the misleading-`lli`-crash-banner
  problem those fixes solved never applied to the C backend), while
  the LLVM backend traps silently with `exit(3)`. Not a regression,
  not new -- just localfuzz's mechanical clusterer correctly noticing
  a real difference that's already understood and intentionally
  left as-is.

---

## C. Previously-known gaps, found this session

All five of these are already recorded in `~/.claude/projects/-home-virgo/memory/`
(the auto-memory system) with fuller context; summarized here so
they're not scattered across two places when picking work. C1, C2,
C4, and C5 are fully fixed (2026-08-07); C3 is half-fixed (`ensures`
done as BUG-133, `invariant` still open).

### C1. `requires` on a `ref Vec<T>` parameter hits an older C-backend code path -- **FIXED as BUG-129 (2026-08-07)**

Confirmed root cause was exactly the hypothesized tree-C-vs-SSA-C parity gap, with one
correction: it's not `ref Vec<T>` specifically -- ANY SSA-unsupported feature ANYWHERE
in the same module forces the WHOLE program onto tree-C (`ssa_path_supports` in
main.rs is module-wide, not per-function), so a plain scalar `requires` clause hit the
identical raw-`assert()`/SIGABRT bug whenever the file also happened to contain
something else SSA-C doesn't support. Full writeup, fix, and regression tests: see
`## BUG-129` in `docs/TODO_CURRENT.md`. Original repro kept below for traceability.

Found verifying tutorial accuracy (2026-08-07). A `requires` clause
on a function taking a SCALAR parameter (e.g. `requires n >= 0;`)
gives the expected, BUG-116-fixed behavior on the C backend:
`"assertion failed: precondition violated in '<fn>'"` + clean
`exit(3)`. The SAME shape on a function taking a `ref Vec<T>`
parameter instead hits a raw glibc `assert()` macro and a real
`SIGABRT` -- e.g. for `fn sum_first_three(xs: ref Vec<i64>) -> i64
requires len(xs) >= 3; { ... }` called with a too-short vec:

```
<binary>: /tmp/....c:747: fn_sum_first_three: Assertion
'(((*v_xs).len) >= ((uint64_t)(3)))' failed.
```

Almost certainly a tree-C-vs-SSA-C parity gap (the `ref Vec<T>`
parameter likely forces the function off the SSA-C fast path onto
tree-C, whose OWN `requires`-clause codegen may never have been
converted to the fprintf+exit(3) shape BUG-116/120 added to SSA-C).
Matches category B's own "SSA-vs-tree parity" theme from the closed
bug-pattern audit -- a natural next candidate if that theme gets
revisited.

Memory: `project_vani_requires_ref_vec_param_c_backend_gap_2026_08_07.md`

### C2. `vanic run`'s exit-code reporting masks a signal-killed child as exit 1 -- **FIXED as BUG-130 (2026-08-07)**

Fixed exactly as suggested below: added a `child_exit_code` helper in `main.rs`
reporting `128 + signal` (via `ExitStatusExt::signal()` on Unix) instead of a bare `1`
whenever `status.code()` is `None`, applied at all 5 call sites. A `#[bounded(N)]`
violation on the C backend (which still raises a raw `abort()`) now correctly reports
`134` instead of `1`. Full writeup: see `## BUG-130` in `docs/TODO_CURRENT.md`.
Original notes kept below for traceability.

`src/main.rs` uses `status.code().unwrap_or(1)` (multiple call
sites) to convert a child process's exit status into `vanic`'s own
process exit code. `Option::code()` returns `None` specifically when
the child was killed BY A SIGNAL (not a normal `exit()` call) -- so
any C-backend program that hits a raw `abort()` (see C1, and every
bounds/overflow/div-by-zero/shift check per the now-accurate Sec.10b
tutorial) gets reported by `vanic run` as exit code `1`, NOT the
shell-familiar `134` (128 + `SIGABRT`'s signal 6) a directly-executed
binary shows, and with no indication a signal was involved at all.
Confirmed directly: `vanic emit --backend=c` + manual `cc` + direct
execution shows `134`/`Aborted`; the identical program via
`vanic run --backend=c` shows `1`.

Minor usability gap, not a correctness bug -- a legitimate small fix
would report `128 + signal` (the shell convention) instead of a bare
`1` when `status.code()` is `None`.

Memory: `reference_vani_vanic_run_exit_code_masking_2026_08_07.md`

### C3. `ensures`/`invariant` have zero runtime enforcement on any backend -- **`ensures` half FIXED as BUG-133 (2026-08-07); `invariant` half still open**

Confirmed with the user first (design options sketched, one picked -- see
`## BUG-133` in `docs/TODO_CURRENT.md` for the full writeup) rather than guessing at
the right model. `ensures` now mirrors `requires` exactly: a confirmed counterexample
(`Disproven`) still hard-fails the build, but an undecidable clause (`Unknown`/
`SkippedUnsupported`/`Unavailable`) now compiles clean and gets a real runtime guard
at the `return` site instead of blocking the build -- reusing the existing
`TypedStmt::Assert`/`intent_assert_fail`/`exit(3)` mechanism, so no new backend
codegen was needed on any of the 4 codegen paths.

`invariant` has NOT been converted yet -- it still has the original compile-time-only
behavior described below. Next candidate for a `BUG-134`-style follow-up if picked up
again: two guard sites per loop (entry + end-of-body preservation, mirroring the two
`failure_phrase` call sites `verify_loop_invariants`/`verify_loop_invariants_with_
havoc` already have for the compile-time version), correct `break`/`continue`
handling (preservation only needs checking at the normal loop-back point, not on
`break`), and -- unlike `requires`/`ensures`, which are call-count-scaled -- real
attention to per-ITERATION overhead for a hot loop with an undecidable invariant,
since that cost is scaled by iteration count instead.

Original notes (still accurate for `invariant`, now describing only half the
original claim for `ensures`): known since the 2026-08-05 bug-pattern-audit session
(category A). Unlike `requires` (which falls back to a real runtime guard when SMT
can't discharge it), `ensures` and `invariant` clauses were purely compile-time -- if
SMT can prove the clause, silent success; if SMT returns anything short of a full
proof, the BUILD fails outright; there was no third "runtime guard" path for either.
`tutorials/src/intermediate/10b_runtime_errors_primer.md` and `12b_compile_time_vs_
runtime_primer.md` were corrected (2026-08-07, both passes) to state this precisely
for each clause instead of treating `ensures`/`invariant` as one undifferentiated
gap.

### C4. Non-ASCII struct/enum type names can be declared but never referenced as a type -- **FIXED as BUG-132 (2026-08-07)**

Confirmed with the user this was a real gap worth fixing, not a deliberate v1
restriction, and confirmed the fix direction (Unicode-aware case check: preserve the
PascalCase convention for cased scripts, accept any letter from scripts with no case
distinction) before implementing -- three separate design options were on the table
(see the fix's own writeup for the other two, rejected). Root cause was broader than
just `parse_type`: two OTHER "does this look like a type name" call sites in
`parser.rs` (module-qualified paths, and the `Name { field: val }` struct-literal
lookahead) had the identical `is_ascii_uppercase()` gate, so even fixing `parse_type`
alone wouldn't have let a Myanmar-named struct actually be CONSTRUCTED, only type-
annotated. Full writeup, fix, and regression tests: see `## BUG-132` in
`docs/TODO_CURRENT.md`. Original notes kept below for traceability.

Found auditing category F (non-ASCII identifier collisions,
2026-08-06). `struct ကက { x: i64 }` parses and type-checks fine as a
DECLARATION, but `let a: ကက = ...;` fails to parse at all
(`error: expected type like 'i32', 'u64', 'f64', 'bool', 'Str', or
'[T; N]'`) -- `parse_type` doesn't accept a non-ASCII identifier
token in type-annotation position. Backend-independent (a pure
parser gap, not a codegen collision). No example anywhere in
`examples/language/*/` uses a non-ASCII TYPE name (only non-ASCII
function/variable/field names), so it's genuinely unclear whether
this is a deliberate v1 restriction or an oversight -- worth a
product decision before "fixing" it either way.

Regression test (UPDATED post-fix, previously pinned the rejecting
behavior under this same name): `non_ascii_struct_name_can_be_
declared_and_used_as_a_type_annotation` in `src/lib.rs`.

### C5. `sort_runtime.c`'s AVX-512 codegen ignores actual host CPU capability -- **FIXED as BUG-131 (2026-08-07)**

The earlier "not observed to crash" note below was simply because the investigation
never sorted an array large enough (>= 128 elements) to actually enter `_block_part`
-- fixing this confirmed the crash directly: a 200-element shuffle hit `SIGILL`, and
not even in the block-partition code (the file-wide `#pragma GCC target` let GCC
auto-vectorize `si_recurse` itself with AVX-512 too). Fixing the crash surfaced a
SECOND, more severe pre-existing bug: `double`'s mask compare reused `int64_t`'s raw
bit-pattern comparison, which doesn't preserve true ordering for negative doubles --
also invisible until the crash was fixed, since the crash always fired first on a
non-AVX-512 host. Both fixed via real runtime CPUID dispatch (`__builtin_cpu_supports(
"avx512f")`) plus a genuinely-floating-point compare path for `double`. Full writeup:
see `## BUG-131` in `docs/TODO_CURRENT.md`. Original notes kept below for
traceability.

Found as an aside while fixing BUG-125 (non-x86 cross-compilation).
`#pragma GCC target("avx512f,avx512bw,avx512dq,avx512vl,avx2,bmi2,popcnt")`
forces AVX-512 instruction selection for ANY x86 build target,
regardless of whether the ACTUAL host CPU running the resulting
binary supports AVX-512 -- there's no runtime CPUID dispatch. This
dev machine's own CPU (Haswell, confirmed via `/proc/cpuinfo`
lacking any `avx512*` flag) doesn't support AVX-512; a standalone
test harness compiled with an explicit `-march=native` crashed with
`Illegal instruction`. The REAL `vanic build`/`vanic run` pipeline
(which doesn't pass `-march=native`, letting the `#pragma` alone
drive codegen) was NOT observed to crash on this same machine for a
native (non-cross) build -- so either the specific flags `vanic`
passes somehow avoid the issue, or there's a narrower trigger
condition not yet identified. Not chased further; flagged here as a
real, plausible portability gap (anyone running a `vanic`-built
binary using `sort`/`sort_by` on a genuinely pre-AVX-512 x86_64 CPU
should verify directly before relying on it) rather than confirmed
end-to-end.

**Suggested investigation starting point**: proper fix is runtime
CPUID dispatch (compile 2+ variants of `_block_part`, e.g. an
AVX-512 one and an SSE2/scalar one, and select via `__builtin_cpu_
supports("avx512f")` or an IFUNC resolver) -- a bigger undertaking
than this doc's other items, hence lowest priority here.
