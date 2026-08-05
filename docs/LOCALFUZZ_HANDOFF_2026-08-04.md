# localfuzz handoff — 2026-08-04

Read this whole file before touching anything. It's a handoff to a fresh
Claude Code session with zero memory of how this list was produced —
everything you need is below, but a couple of steps are load-bearing
(rebuild first, re-check BUG-N numbering fresh) and skipping them will
waste your first hour.

## 0. Do this first, every time, before trusting anything below

The compiler moves fast — a separate automated process also lands fixes
to `main` (see `feedback_vani_concurrent_localfuzz_process` if you have
access to session memory; if not, just know: `main` may have advanced
since this file was written, possibly even fixing some of the bugs
below). Confirmed repeatedly on 2026-08-04: the localfuzz worktree's
compiler binary went stale relative to `main` **twice in one day**, and
some earlier findings turned out to already be fixed by fixes that
landed while nobody was looking.

```bash
cd ~/source/vani-compiler && python3 scripts/localfuzz_status.py
```

This refreshes the worktree (merges `main`, rebuilds `vanic`, restarts
the harness), re-clusters current findings, and prints the current
highest `BUG-N` fresh. Run it, THEN re-verify each item below still
reproduces on the freshly-built binary before spending real time on it:

```bash
cd ~/source/vani-compiler-localfuzz
./target/release/vanic check tools/localfuzz/findings/<dir>/repro.vani
./target/release/vanic run   tools/localfuzz/findings/<dir>/repro.vani --backend=c ; echo "c rc=$?"
./target/release/vanic run   tools/localfuzz/findings/<dir>/repro.vani            ; echo "llvm rc=$?"
```

If a repro's C/LLVM exit codes now MATCH (or check now rejects it
cleanly), it's already fixed — skip it, don't re-fix it. This happened
today: BUG-105's fix (a general `sanitize_ident` collision fix) silently
also fixed an unrelated-looking finding that never keyword-matched
BUG-105's own description. Don't trust "not obviously related to any
BUG-N" as proof something is still broken, and don't trust "no longer
flagged in DIGEST_LATEST.md" either — always re-run the repro yourself.

**State as of this update**: Issues 1 and 2 below (original numbering)
are now FIXED as BUG-107 and BUG-108, landed on `main` at commit
`744dc29f43619142127228b8d867bf63240f36c5` (2026-08-04T17:59:54-04:00),
CI + CodeQL green. **Issue 2 turned out to be misdiagnosed** in the
original version of this handoff — see its section below before doing
anything with it; the real bug was much broader than "a `mut ref
Vec<T>` write-back bug" and the fix is already in. Issues 3 and 4 are
still open; Issue 3 got real bisection work today (one of its two
repros turned out not to be a bug at all — see below) but no fix.
Worktree rebuilt at commit `26f3e25033c2463e6dae6ad7d3f60ab06ebc5477`
(2026-08-04T22:48:59-04:00), which includes BUG-107/108. Re-verify
against whatever `main` is by the time you read this, same as always.

## 1. Before writing any BUG-N entry

```bash
cd ~/source/vani-compiler
git fetch origin main
git show origin/main:docs/TODO_CURRENT.md | grep -oE "BUG-[0-9]+" | sort -t- -k2 -n | uniq | tail -1
```

Get the number FRESH, not from this file (the concurrent process may
have taken a number you'd otherwise collide with). `git fetch` and
diff against `origin/main` before pushing, same reason.

## 2. The open issues, priority order

### Issue 1 — FIXED as BUG-107 (2026-08-04)

C backend couldn't compile a struct field of type `Vec<Box<dyn
Trait>>`. Root cause: `backend_c.rs`'s `vec_element_has_user_struct`
had no `Type::Box` arm, so it didn't recurse into `Box`'s inner type
and missed that `Box<dyn Iface>` (stored BY VALUE, 16-byte fat
pointer) has the same forward-reference dependency on
`emit_dyn_iface_typedefs` that a bare `Vec<dyn Iface>` field already
correctly deferred for. See `docs/TODO_CURRENT.md`'s BUG-107 entry for
the full writeup. Nothing further to do here.

### Issue 2 — FIXED as BUG-108 (2026-08-04), but the ORIGINAL diagnosis below was wrong

**If you're reading this because you're about to work Issue 2: don't
trust the paragraph that used to be here.** The original version of
this handoff characterized this finding as "C's `mut ref Vec<T>`
out-parameter doesn't write back correctly, LLVM handles it fine."
That was backwards and incomplete. Re-verifying on a fresh rebuild
(per section 0's own rule) showed LLVM was ALSO wrong — `astar`
returned `None` for every call and `topo_sort`'s `order[i]` printed
garbage large integers instead of C's clean `index out of bounds`
abort. The real root cause had nothing to do with `Graph`/`astar`/
`topo_sort` at all: `graph_new(-1)` (a fuzzer-mutated invalid negative
node count in this specific repro) was a total red herring. Bisected
down to: **the tree-walking LLVM backend's Vec index read, write, and
mut-ref-element codegen had NO runtime bounds check at all**, and ANY
struct-typed local anywhere in the program (not Graph-specific) forces
the WHOLE program off the SSA-LLVM fast path onto this unchecked tree
path (`expr_ssa_supported` in `main.rs` unconditionally rejects
`StructLit`/`FieldAccess`). Minimal repro that reproduced identically
with zero `Graph` involvement:
```vani
struct Foo { a: i64, b: i64 }
fn main() -> i64 {
  let g: Foo = Foo { a: 1, b: 2 };
  let order: Vec<i64> = vec();
  for i from 0 to 5 { print order[i]; }
  return 0;
}
```
Fixed by adding a `@__intent_bounds_check` helper (mirroring the one
`ssa_backend_llvm.rs` already had) to tree-LLVM's own preamble and
wiring it into the 4 affected sites (`Index` read, `IndexAssign`
write, `RefMutIndex`, and the Vec<bool> packed-bit read/write
variants). See `docs/TODO_CURRENT.md`'s BUG-108 entry for the full
writeup, including an explicit **scope note**: only the 3 codegen
sites directly implicated by this finding were audited/fixed.
`backend_llvm.rs` is large; it's plausible other Vec-touching
tree-LLVM call sites (builtin helpers like `binary_search`,
`swap_remove`, etc. — also tree-LLVM-only per the same denylist) have
the identical missing-check pattern and were NOT swept this pass.
**If you have spare time before picking up Issue 3/4, a dedicated pass
grepping tree-LLVM's Vec-adjacent codegen for a GEP not preceded by a
`@__intent_bounds_check` call is worth doing** — this bug class is real
and this pass only chased the one reproducing instance.

### Issue 3: LLVM backend "hangs" on two task/async examples — re-scoped, only half-investigated

The original framing ("two unrelated-looking repros that are probably
the same task/async root cause") was wrong in an interesting way: they
turned out to be two COMPLETELY unrelated things, and repro A isn't
even a compiler bug.

- **Repro A** — `tools/localfuzz/findings/20260803-003108-run-crash-4804c21458/repro.vani`.
  **Investigated 2026-08-04, conclusion: NOT a task/async bug, likely
  not a fixable bug at all.** The actual repro (despite being
  catalogued as "Swedish task/async") is `for_loops.vani` — no task/
  async anywhere. It's a fuzzer-mutated `for i from
  -9223372036854775808 to 5 { antal = antal + 1; }` (a ~9.223
  quintillion-iteration loop — `5 - i64::MIN` slightly exceeds
  `i64::MAX`) followed by `assert räkna() == 5` (a nonsensical
  assertion given the loop's real iteration count). Modifying the
  function to add an early-return once `antal > 20` makes BOTH
  backends complete instantly with IDENTICAL output — proving there's
  no arithmetic/codegen divergence in the loop itself; both backends
  execute it correctly step-by-step. The divergence ONLY shows up when
  the loop is left to attempt the full ~9.2-quintillion-iteration run:
  C "completes" (with `assertion failed:`, exit 3) inside the 10s
  timeout while LLVM doesn't. Working theory (NOT confirmed further):
  `backend_c.rs` emits a real C `for` loop that `cc` at -O2/-O3 can
  likely strength-reduce / recognize as a closed-form induction
  variable and fold away, while `lli` (the LLVM interpreter/JIT this
  project uses to `run` LLVM output) has no reason to apply the same
  optimization and genuinely walks all ~9.2 quintillion iterations —
  which is not completable in any practical timeout regardless of
  backend correctness. If this theory is right, there's no bug to fix
  here; it's a fundamental AOT-optimized-C vs. JIT-interpreted-LLVM
  artifact on a computationally-infeasible fuzzer mutation, same
  category as the "lower priority, not investigated further" cluster
  at the bottom of this doc. Confirm the strength-reduction theory (or
  find a real bug) before spending more time on this one; don't just
  assume it's closeable, but don't expect a normal BUG-N fix either.

- **Repro B — FIXED as BUG-109 (2026-08-04).** Wasn't a task/async bug
  at all, and wasn't in `Task__handle`/`__poll_handle`/`io_recv_async`/
  epoll — all of that machinery was fully correct. Root cause: tree-
  LLVM's `Vec<bool>` LITERAL construction (`emit_vec_let_from_literal`
  in `backend_llvm.rs`) used a byte-addressed, one-bool-per-slot
  buffer layout, incompatible with the packed (64-bools-per-i64-word)
  layout every other `intent_vec_bool` op (`Index` read, `IndexAssign`
  write, `push`) expects. `let alive: Vec<bool> = vec(true, true,
  true);` in the repro read `alive[1]`/`alive[2]` back as `false` from
  the moment of construction — so the round-robin scheduler's `if
  alive[j] { poll pool[j] }` simply never revisited those two slots
  again, even though their peers' data was sitting ready in the kernel
  socket buffer the whole time. Indistinguishable from a genuine
  infinite scheduling hang without bisecting the async machinery away
  entirely (which is exactly how this got found — see the BUG-109
  entry in `docs/TODO_CURRENT.md` on `main` for the full bisection
  trail and fix). The two dead-end leads noted in the previous version
  of this section (epoll timeout truncation, `task{}` threading model)
  were correctly ruled out and are NOT the bug — don't re-chase them.

### Issue 4 (most severe — double bug): LLVM `lli` crashes outright AND C backend hangs, on the same input

- **Repro**: `tools/localfuzz/findings/20260803-033452-run-crash-99db3e1928/repro.vani` (Odia-language keywords example; also exercises the parallel/sort runtime libs — the `lli` invocation loads `libgomp.so.1`, a `vanic-sortlib-*.so`, and a `vanic-parlib-*.so`)
- **Reproduce**:
  - `vanic run <repro>` (LLVM): crashes immediately with
    `PLEASE submit a bug report to https://github.com/llvm/llvm-project/issues/` and a stack dump inside `libLLVM.so.19.1` — this is `lli` itself crashing, not a vani-level error.
  - `timeout 10 vanic run <repro> --backend=c`: hangs, gets killed by timeout.
- **Lead**: neither backend handles this input correctly, which makes
  it the highest-value target even though it'll probably take the most
  digging. Since `lli` is crashing inside LLVM's own machinery (not a
  clean "invalid IR" parse error like BUG-88's class), the generated
  `.ll` is likely passing IR verification but constructing something
  that trips up the JIT at a lower level (possibly related to the
  parallel/sort library loading shown in the `lli` invocation args —
  worth checking whether this repro is exercising `sort`/`parallel`
  constructs specifically, and whether backend_llvm.rs's codegen for
  those interacts badly with something else in this file). Get the
  full `lli` stack dump (`vanic run <repro> 2>&1 | head -50`) before
  starting — I only captured the first few frames.

### Lower priority, not investigated further

An 11-finding cluster where BOTH backends hang identically (same
exit behavior on both — not a divergence). Likely a fuzzer-mutation
that introduced a genuine infinite loop into the test program itself,
not a compiler bug. Example: `tools/localfuzz/findings/20260802-180614-run-crash-1a1740134e/repro.vani`.
Worth a quick look if the four issues above are cleared, not before.

## 3. What "done" looks like for each

Same rigor as every other `BUG-N` in this project (see any recent entry
in `docs/TODO_CURRENT.md` for the expected shape): root-cause it for
real (don't guess-and-check), fix it, add both a `src/lib.rs` unit test
and a `tests/run_end_to_end.rs` subprocess test that exercises both
backends, run `cargo test --release --workspace` clean, then write the
`BUG-N` entry into `docs/TODO_CURRENT.md` on `main` using a number you
checked fresh per section 1. Full `cargo test` — not just `--lib` — see
[[feedback_vani_ci_driven_workflow]] if you have session memory access
(skip local full runs where reasonable, batch fixes, push+poll CI).

## 4. Tools available to you

- `python3 scripts/localfuzz_status.py` (from `~/source/vani-compiler`) — daily briefing: pipeline health, fresh digest, BUG-N number.
- `~/source/vani-compiler-localfuzz/tools/localfuzz/refresh.sh` — merge main + rebuild + restart, safe to run anytime by hand.
- `~/source/vani-compiler-localfuzz/tools/localfuzz/digest.py` — dedup findings by signature; `--all` for a full re-scan.
- `~/source/vani-compiler-localfuzz/tools/localfuzz/DIGEST_LATEST.md` — current deduped view (but see section 0's caveat about trusting it blindly).
- The harness (`vani-localfuzz-harness`/`vani-localfuzz-ollama` systemd --user services) keeps running and finding MORE stuff in the background — nightly refresh (03:00) and digest (06:00) timers are enabled with linger, so check `DIGEST_LATEST.md` for anything newer than this file too.
