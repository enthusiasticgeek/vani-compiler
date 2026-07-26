# vāṇī — Work Queue

> This file has been condensed on 2026-06-19.
> Pre-v0.1.0 planning history is in **[TODO_ARCHIVE.md](TODO_ARCHIVE.md)**.
> The canonical current work queue (actionable, checkbox-ordered) is in
> **[docs/TODO_CURRENT.md](docs/TODO_CURRENT.md)**.

## Current status (as of 2026-07-26)

- **Version**: `0.8.2-dev` (tagged v0.1.0 through v0.8.2-dev; see RELEASING.md for full history).
- **Tests**: 2579+ lib tests passing (new since 2026-07-13: BUG-6 through BUG-11
  regression tests, see below).
- **Dialects**: 62 across 26 scripts.
- **Blocked**: macOS hardware, grammar consultant, proper IOCP, crates.io API token.
- **2026-07-25/26 session**: BUG-6 through BUG-11 found and fixed (real
  dangling-reference / codegen-correctness bugs, not just missing
  features — see [docs/TODO_CURRENT.md](docs/TODO_CURRENT.md) for full
  writeups). Ref-capturing closures went from "scoped only" to fully
  implemented: v-fix, v1 (real `Closure` values), v2 (non-escape
  enforcement), and v3 (`vani-optimize` v0.1.5 gained `Closure`-accepting
  variants) all shipped — see
  [docs/ref_capturing_closures_design.md](docs/ref_capturing_closures_design.md).
  One item found but deliberately not fixed this session: **BUG-12**
  (`push`'s scope-escape check has the same `mut-ref`-parameter flaw
  BUG-9 fixed for `FieldAssign`) — needs threading parameter-name
  context through the much more widely-called `check_call`, a bigger
  change than fit in this pass.

## Open items (summary)

See [docs/TODO_CURRENT.md](docs/TODO_CURRENT.md) for the full checkbox list.
All items within our control are done. Only blocked items remain:

| # | Item | Effort | Blocker |
|---|------|--------|---------|
| 1 | Publish to crates.io (`cargo publish`) | < 1 h | crates.io API token |
| 14 | Homebrew formula | dedicated session | macOS hardware |

## Blocked items

| Item | Blocker |
|---|---|
| macOS empirical verification | Darwin hardware needed |
| Grammar consultant pass | External native-speaker review |
| Windows IOCP proper rewrite (overlapped I/O) | Readiness-vs-completion model mismatch (R8 in docs/decisions.md) |
| Arc 7 Win64/AArch64 CI wiring | CI runner setup |
| crates.io publish — v0.1.2 tagged and ready | crates.io API token needed (`cargo login`) |
