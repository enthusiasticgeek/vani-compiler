# vāṇī — Work Queue

> This file has been condensed on 2026-06-19.
> Pre-v0.1.0 planning history is in **[TODO_ARCHIVE.md](TODO_ARCHIVE.md)**.
> The canonical current work queue (actionable, checkbox-ordered) is in
> **[docs/TODO_CURRENT.md](docs/TODO_CURRENT.md)**.

## Current status (as of 2026-07-26)

- **Version**: `0.9.1-dev` (tagged v0.1.0 through v0.9.1-dev; see RELEASING.md for full history).
- **Tests**: 2582+ lib tests passing (new since 2026-07-13: BUG-6 through BUG-12
  regression tests, see below).
- **Dialects**: 62 across 26 scripts.
- **Blocked**: macOS hardware, grammar consultant, proper IOCP, crates.io API token.
- **2026-07-25/26 session**: BUG-6 through BUG-12 found and fixed (real
  dangling-reference / codegen-correctness bugs, not just missing
  features — see [docs/TODO_CURRENT.md](docs/TODO_CURRENT.md) for full
  writeups). Ref-capturing closures went from "scoped only" to fully
  implemented: v-fix, v1 (real `Closure` values), v2 (non-escape
  enforcement), and v3 (`vani-optimize` v0.1.5 gained `Closure`-accepting
  variants) all shipped — see
  [docs/ref_capturing_closures_design.md](docs/ref_capturing_closures_design.md).
  **No known bugs remain open** as of this session's end.
- **2026-07-27 update**: this summary is stale relative to
  [docs/TODO_CURRENT.md](docs/TODO_CURRENT.md), which is the
  authoritative up-to-date bug log — BUG-13 through BUG-23 were found
  and (mostly) fixed since the note above was written; BUG-22's
  struct/enum `RwLock`/`Mutex` payload case on `--backend=c` is the one
  still open. BUG-23 (C backend's `while_bounds_hints` referencing a
  Vec declared fresh inside its own loop body — found while fixing
  vani-algebra's `algebra_newton_system_fd`, which couldn't compile on
  `--backend=c` at all) is fixed.

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
