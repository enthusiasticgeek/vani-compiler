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

**State as of writing this file**: worktree rebuilt at commit
`b21be4daf97fe9734a2f3af573da39c148486bda` (2026-08-04T14:48:10-04:00),
which includes BUG-99 through BUG-106. Everything below was verified
against THAT exact build. Re-verify against whatever `main` is by the
time you read this.

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

All four below are **confirmed still open** as of the build/commit
noted in section 0 — live-tested, not just digest-matched. Each repro
lives at `tools/localfuzz/findings/<dir>/repro.vani` in the
`vani-compiler-localfuzz` worktree (branch `local-fuzz-findings`,
already pushed to `origin/local-fuzz-findings` if you want it from a
clean clone instead of this specific worktree).

### Issue 1 (recommend starting here — most contained): C backend can't compile `Vec<Box<dyn Trait>>`

- **Repro**: `tools/localfuzz/findings/20260803-130927-backend-divergence-dc30074c7a/repro.vani`
- **Reproduce**: `vanic run <repro> --backend=c` (LLVM backend runs it fine — `vanic run <repro>` with no flag)
- **Symptom**: `cc` fails to compile the generated C:
  ```
  error: unknown type name 'intent_dyn_Drawable'
    248 | typedef struct { intent_dyn_Drawable* __restrict__ data; ... } intent_vec_box_dyn_Drawable;
  error: 'intent_dyn_Drawable' undeclared (first use in this function)
  ```
- **Lead**: the C backend emits a `Vec<Box<dyn Drawable>>` helper struct
  (`intent_vec_box_dyn_Drawable`) that references a type
  (`intent_dyn_Drawable`) it never actually typedefs/emits anywhere.
  Look in `src/backend_c.rs` for wherever `Vec<Box<dyn T>>` element
  storage typedefs get generated (search for `intent_vec_box_dyn` /
  the vtable-struct emission for `Box<dyn T>` — likely a case where the
  `dyn Trait` element type's typedef is supposed to be emitted before
  its `Vec<...>` wrapper typedef but isn't, or is gated on a condition
  that misses this shape). Compare against how `Vec<Box<T>>` (non-dyn)
  or `Box<dyn T>` on its own (not inside a `Vec`) are handled — one of
  those paths likely DOES emit the missing typedef and this one just
  doesn't reach it.

### Issue 2: C backend's `mut ref Vec<T>` out-parameter doesn't write back correctly (LLVM does)

- **Repro**: `tools/localfuzz/findings/20260803-144958-backend-divergence-2125e1a114/repro.vani` (this is `examples/graph_algo2.vani` — a real shipped example, not a fuzzer mutation, so this may already be reproducible today by just running the shipped example directly with `--backend=c`)
- **Reproduce**: `vanic run <repro> --backend=c`
- **Symptom**: runtime output `index out of bounds: 0, len 0` then a non-zero exit. Originally characterized (wrongly, in an earlier pass) as "a Rust panic inside the compiler" — it is NOT that. It's the COMPILED PROGRAM's own bounds-check runtime helper firing, from this line in the repro:
  ```vani
  let order: Vec<i64> = vec();
  let n_appended: i64 = g.topo_sort(mut ref order);
  ...
  for i from 0 to 5 { print "topo[", i, "] =", order[i]; }
  ```
  `topo_sort(mut ref out: Vec<i64>)` is supposed to push nodes into
  `order` and the caller should see those pushes. `order[i]` failing
  with `len 0` means the C backend's `order` still looks EMPTY to the
  caller after the call returns, even though `topo_sort` (per the LLVM
  backend, which handles this fine) did populate it.
- **Lead**: this smells like a `mut ref Vec<T>` parameter aliasing/
  write-back bug specific to the C backend — the callee's pushes may be
  happening on a local copy of the Vec's header (ptr/len/cap struct)
  instead of through the pointer the caller passed. Look at how
  `backend_c.rs` lowers a `mut ref Vec<T>` parameter — specifically
  whether `push`/`len`-mutating operations inside the callee correctly
  write through to the caller's own `intent_vec_*` struct, or whether
  they mutate a stack-local copy that gets discarded on return. I did
  NOT get further than this — no confirmed root cause, just the
  observed symptom and this hypothesis. Worth first trying a smaller
  repro than the full graph-algorithm file: a two-line function that
  takes `mut ref out: Vec<i64>`, pushes into it, and checks
  `order.len()` in the caller after the call (I attempted this and hit
  a parser error on my own syntax guess for the parameter — check
  `examples/language/english/` or similar for the correct `mut ref
  Vec<T>` parameter syntax before trying to hand-write a minimal repro).

### Issue 3: LLVM backend hangs (infinite loop) on two task/async examples where C completes fine

- **Repro A**: `tools/localfuzz/findings/20260803-003108-run-crash-4804c21458/repro.vani` (Swedish-language task/async example)
- **Repro B**: `tools/localfuzz/findings/20260803-050543-run-crash-6bd324cd8f/repro.vani` (an Arc-concurrency `echo_pool` server example — single-threaded server handling N concurrent client tasks)
- **Reproduce**: `timeout 10 vanic run <repro>` (no `--backend=c`) will hang and get killed; `vanic run <repro> --backend=c` completes normally.
- **Symptom**: no crash, no error — just never terminates under the LLVM backend.
- **Lead**: not root-caused at all yet. Both involve `task`/async
  scheduling constructs. Could be the same root cause (something in
  the LLVM task-scheduling runtime/lowering that under some condition
  spins instead of completing) or two unrelated bugs that happen to
  both be task-related. Worth checking whether they share a specific
  construct (e.g. both use channels, or both use a specific
  cancellation/join pattern) — I didn't diff them closely. Look at
  `ssa_backend_llvm.rs` / whatever handles `task`/`async`/channel
  lowering for the LLVM path, and compare against the equivalent C
  runtime (`backend_c.rs` / `ssa_backend_c.rs`) which handles both
  fine.

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
