# vāṇī — Work Queue

> This file has been condensed on 2026-06-19.
> Pre-v0.1.0 planning history is in **[TODO_ARCHIVE.md](TODO_ARCHIVE.md)**.
> The canonical current work queue (actionable, checkbox-ordered) is in
> **[docs/TODO_CURRENT.md](docs/TODO_CURRENT.md)**.

## Current status (as of 2026-07-28)

- **Version**: `0.9.4-dev` (tagged v0.1.0 through v0.9.4-dev; see RELEASING.md for full history).
- **Tests**: 2600 lib tests passing, **0 failing** — first fully clean
  `cargo test --lib` run in a while; the 3 Win64 FFI-ABI failures that
  had been showing up (and being written off as "pre-existing,
  unrelated") were finally diagnosed and fixed as BUG-26, see below.
- **Dialects**: 62 across 26 scripts.
- **Blocked**: macOS hardware, grammar consultant, AArch64/RISC-V benchmark hardware, proper IOCP, crates.io API token.
- **No known bugs remain open.** BUG-5 through BUG-26 are all fixed —
  see [docs/TODO_CURRENT.md](docs/TODO_CURRENT.md) for full writeups.
  Highlights from the 2026-07-27/28 sessions: BUG-13 through BUG-23
  covered a range of codegen-correctness fixes (RwLock/Mutex payload
  types, slice-match pattern guards, a C-backend Vec-bounds hint
  false-firing on a fresh-in-loop-body Vec); BUG-21 Path B shipped a
  genuine new language feature (`Task<R>` — `task <fn>(args…)` /
  `join <name>` spawn a real OS thread and carry a typed return value
  back across it, not just the old payload-free block form); auditing
  that feature's own code for the same bug class found and fixed
  BUG-24 (a real heap buffer overflow in the LLVM backend's task-spawn
  context sizing) and BUG-25 (unsound struct-size accounting in the
  `#[bounded_stack]` safety-critical stack-overflow verifier); BUG-26
  diagnosed and fixed the 3 long-standing "pre-existing" test failures
  instead of continuing to write them off.
- **2026-07-25/26 session**: BUG-6 through BUG-12 found and fixed (real
  dangling-reference / codegen-correctness bugs, not just missing
  features). Ref-capturing closures went from "scoped only" to fully
  implemented: v-fix, v1 (real `Closure` values), v2 (non-escape
  enforcement), and v3 (`vani-optimize` v0.1.5 gained `Closure`-accepting
  variants) all shipped — see
  [docs/ref_capturing_closures_design.md](docs/ref_capturing_closures_design.md).

## Open items (summary)

See [docs/TODO_CURRENT.md](docs/TODO_CURRENT.md) for the full checkbox list.
All items within our control are done. What's left is either blocked
on something external, or an open design decision nobody's greenlit:

| # | Item | Effort | Status |
|---|------|--------|--------|
| 1 | Publish to crates.io (`cargo publish`) | < 1 h | Blocked — crates.io API token |
| 14 | Homebrew formula | dedicated session | Blocked — macOS hardware |
| 27 | Inline `print` format specs (Rust `{:03}`/`{:.2}` syntax) | ~unscoped | Open design question, not started — the *capability* already exists via plain function calls; this is purely about whether to add template-string syntax on top |
| ARM-3 | AArch64 benchmark run (real silicon, all benchmarks) | ~2 h + hardware | Blocked — needs Graviton 3 / Pi 4 / Apple Silicon (QEMU already covers correctness, not real-hardware numbers) |
| RVV-bench | RISC-V Vector benchmark run | ~unscoped | Blocked — needs real RISC-V hardware with the V extension |

## Blocked items

| Item | Blocker |
|---|---|
| macOS empirical verification | Darwin hardware needed |
| Grammar consultant pass (non-Devanagari dialect keyword review) | External native-speaker review |
| Windows IOCP proper rewrite (overlapped I/O) | Genuine design problem, not just access — readiness-vs-completion model mismatch (R8 in docs/decisions.md) |
| AArch64 / RISC-V benchmark runs | Real hardware needed (QEMU CI already covers correctness) |
| crates.io publish — ready to go | crates.io API token needed (`cargo login`) |
