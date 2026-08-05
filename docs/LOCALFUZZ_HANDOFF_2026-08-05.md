# localfuzz handoff — 2026-08-05

**STATUS (2026-08-05, later same day): fully closed out.** The one real bug this file
tracked (section 2, the SSA-LLVM int-literal-to-float `let` bug) is FIXED and shipped
as **BUG-111** — see `docs/TODO_CURRENT.md`'s BUG-111 entry for the full root-cause
writeup and fix. The two items in section 3 remain explicitly NOT new work (unchanged
from when this file was written). Section 2 below is left as-is (historical record of
how the bug was found/scoped, matching the style of the previous handoff once its
issues were closed) rather than rewritten — read it as "what was true when this file
was written," not as still-open work.

Read this whole file before touching anything. Fresh session, zero memory of how this
list was produced. This is a MUCH smaller handoff than
`docs/LOCALFUZZ_HANDOFF_2026-08-04.md` (which is now fully closed out — BUG-107
through BUG-110, all fixed and shipped) — just one real, well-scoped, high-impact bug,
plus two items that are explicitly NOT new work so you don't waste time re-investigating
them.

## 0. Do this first, every time, before trusting anything below

```bash
cd ~/source/vani-compiler && python3 scripts/localfuzz_status.py
```

Refreshes the worktree (merges `main`, rebuilds `vanic`, restarts the harness),
re-clusters current findings, prints the fresh highest `BUG-N`. THEN re-verify the repro
below still reproduces on the freshly-built binary before spending real time on it —
same discipline as the previous handoff, see its section 0 for why this matters (a
separate automated process also lands fixes to `main` concurrently — see
`feedback_vani_concurrent_localfuzz_process` in session memory if you have access).

**State as of writing this file**: worktree at commit
`f3dd02ff17934bf489e39a56fad475c567d2f22a` (2026-08-05T00:00:42-04:00), which includes
BUG-107 through BUG-110. `main` at `bad661728aa095c08891bcd4221274cd7a347f97`.
Everything below was verified against that exact build. Re-verify against whatever
`main` is by the time you read this.

## 1. Before writing any BUG-N entry

```bash
cd ~/source/vani-compiler
git fetch origin main
git show origin/main:docs/TODO_CURRENT.md | grep -oE "BUG-[0-9]+" | sort -t- -k2 -n | uniq | tail -1
```

Get the number FRESH, not from this file. As of writing, highest is `BUG-110`, next
free is `BUG-111` — but check again, don't trust this number by the time you read it.

## 2. The real bug: SSA-LLVM emits invalid IR for `let x: f64 = <integer literal>;`

**This is a genuine, new, unfixed, well-scoped compiler bug** — not a fuzzer-broken
test program (unlike the other two items in section 3 below). Confirmed via 4
independent localfuzz findings (Korean, Tibetan x2, plus hand-written minimal repros)
and isolated to a 4-line reproduction.

- **Minimal repro** (doesn't need any localfuzz finding — just write this file):
  ```vani
  fn main() -> i64 {
    let n: f64 = 0;
    print n;
    return 0;
  }
  ```
- **Reproduce**: `vanic run <repro>` (LLVM, default backend — no `--backend` flag
  needed, this IS the default/SSA fast path). `vanic run <repro> --backend=c` works
  fine (prints `0`), confirming this is LLVM-specific.
- **Symptom**: `lli` rejects the generated `.ll` outright at parse time —
  ```
  lli: lli: /tmp/vanic-....ll:68:27: error: integer constant must have integer type
    %v_0 = fadd double 0.0, 0
                            ^
  ```
  The float-identity-op pattern (`fadd double 0.0, <value>`, used to materialize a
  value of type `f64` as an SSA value with no actual computation) is emitting the
  literal's INTEGER spelling (`0`, or `7`, or whatever) instead of converting it to a
  float-literal spelling (`0.0`, `7.0`). LLVM IR requires float-typed constant operands
  to be spelled as floats (`0.0`, not `0`) — this is invalid syntax, not a semantic
  error, so it's rejected before the program ever runs.
- **Confirmed NOT specific to the literal `0`**: `let n: f64 = 7;` fails identically
  (`fadd double 0.0, 7` — same missing `.0`).
- **Confirmed SSA-LLVM-specific, not tree-LLVM**: adding a struct literal anywhere in
  the program (which forces the whole program off the SSA-LLVM fast path per
  `expr_ssa_supported` in `main.rs` — same mechanism as BUG-108/109/110 from
  yesterday's handoff) makes it work correctly:
  ```vani
  struct Foo { a: i64 }
  fn main() -> i64 {
    let g: Foo = Foo { a: 1 };   // <- forces tree-LLVM
    let n: f64 = 0;
    print n;                     // prints "0" correctly
    return 0;
  }
  ```
  So the bug is specifically in `ssa_backend_llvm.rs`'s handling of an integer-literal
  RHS being coerced to an `f64`-typed `let` binding — most likely wherever it emits the
  `fadd double 0.0, <operand>` identity-op pattern (or equivalent) for materializing a
  float-typed constant, and needs to spell an `Operand::Const(Const::Int(_))` as a
  float literal (`{n}.0`) rather than passing the integer's own literal text straight
  through when the target type is `f64`/`f32`. Cross-check against how
  `backend_llvm.rs` (tree) or `ssa_backend_c.rs` correctly handle the identical
  `int-literal -> f64` coercion for comparison — this is precisely the class of "one
  codegen path has it right, the other doesn't" bug this project has repeatedly hit
  (see BUG-107/108/109/110's writeups in `docs/TODO_CURRENT.md` for the established
  pattern and fix shape). NOT root-caused beyond identifying the exact invalid IR line
  and the SSA-vs-tree split — the actual fix site in `ssa_backend_llvm.rs` hasn't been
  located yet.
- **Blast radius**: `let x: f64 = <int literal>;` is an extremely common pattern
  (e.g. `let total: f64 = 0;` to start an accumulator) — likely to affect a meaningful
  fraction of real f64-using programs on the default LLVM backend, not just fuzzer
  edge cases. High priority despite the small/simple repro.
- **Do NOT confuse with BUG-76.** The localfuzz digest's naive keyword-matching
  auto-flagged a 2-finding cluster (`20260804-192840-backend-divergence-05bd1b1e14`,
  `20260804-213005-backend-divergence-deb22ec24a` — both Tibetan `for_loops.vani`
  fuzzed variants) as a "possible match: BUG-76" purely because the stderr contains the
  substring "integer constant must have integer type" — which ALSO appears in BUG-76's
  writeup in `TODO_CURRENT.md` for a completely unrelated reason. Checked both: they're
  this SAME `f64 = int literal` bug, not BUG-76. Trust the actual repro content over the
  digest's keyword match every time (this is a recurring gotcha — see the previous
  handoff's section 0 for the general "don't trust digest matching" rule).
- **What "done" looks like**: same rigor as every other `BUG-N` — root-cause for real,
  fix `ssa_backend_llvm.rs`, add a `src/lib.rs` unit test (going through
  `lower_program` + `ssa_backend_llvm::emit`, NOT `compile_to_llvm` — that calls the
  tree backend directly and won't exercise this path at all, exactly the coverage gap
  BUG-110's writeup documents in detail) and a `tests/run_end_to_end.rs` subprocess
  test, run `cargo test --release --workspace` clean, write the `BUG-N` entry into
  `docs/TODO_CURRENT.md` using a number checked fresh per section 1, push, poll CI.

## 3. NOT new work — don't re-investigate these

- **`20260804-204024-backend-divergence-ffadfdc1f9`** (self-referential struct holding
  `Vec<Self>`, e.g. `struct Node { children: Vec<Node> }`, on the LLVM backend) — this
  is the **already-known, already-tracked LLVM counterpart of `BUG-31`**. `BUG-31`
  itself is `[x]` fixed on the C backend (see `docs/TODO_CURRENT.md`). The repro file's
  own header comment already documents this exact gap: "the LLVM backend produces a
  native binary that crashes silently... a separate, unfixed bug... tracked as
  follow-up work." If you want to pick this up, it's legitimate real work (not fixed
  yet), just not something the localfuzz harness discovered for the first time
  yesterday — it's been a known gap for longer. Worth a `BUG-N` if someone
  root-causes and fixes it, but don't write "found by localfuzz" in the entry; it's
  more accurate to say "long-standing known gap, LLVM counterpart of BUG-31."
- **`20260804-222218-run-crash-d66ccbb300`** (Hebrew `first_negative`-style example) —
  **not a compiler bug**, a fuzzer-broken test program. The `while` loop's counter
  (`ע`) is only ever incremented... it isn't — there's no increment statement anywhere
  in the loop body outside the early-`break` branch, so if the first list element isn't
  negative, the loop spins on index 0 forever. Confirmed: both backends hang
  identically (`timeout 5 vanic run <repro> --backend=c` → exit 124 on both). Matches
  the "Lower priority, not investigated further" pattern from yesterday's handoff
  (BOTH backends hanging identically = broken test program, not a divergence). Skip
  unless you're specifically doing a sweep of that whole low-priority cluster.

## 4. Tools available to you

Same as yesterday's handoff, section 4 — `python3 scripts/localfuzz_status.py`,
`~/source/vani-compiler-localfuzz/tools/localfuzz/refresh.sh`,
`tools/localfuzz/digest.py`, `tools/localfuzz/DIGEST_LATEST.md`. Harness keeps running
in the background (nightly refresh 03:00, digest 06:00) — check `DIGEST_LATEST.md` for
anything newer than this file too.

**One operational note**: `vani-localfuzz-harness` showed `activating/auto-restart`
(not settled `active/running`) in the last status check before this file was written —
worth a glance at `journalctl --user -u vani-localfuzz-harness -f` if the digest looks
stale or stops producing new findings; may just be a transient restart cycle, wasn't
investigated further.

**Pitfall from yesterday, worth repeating**: if you develop a fix in the
`vani-compiler-localfuzz` worktree and port it to `~/source/vani-compiler` (main) via
patches, do NOT `git checkout -- <file>` the worktree's now-redundant uncommitted
copy as a cleanup step — that restores from the worktree's OWN (possibly stale) branch
HEAD, not from `origin/main`, and can silently revert your fix in the worktree. Run
`refresh.sh` instead (merges `origin/main` properly). See
`feedback_vani_localfuzz_worktree_checkout_pitfall` in session memory if you have
access.
