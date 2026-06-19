# vāṇī — Work Queue

> This file has been condensed on 2026-06-19.
> Pre-v0.1.0 planning history is in **[TODO_ARCHIVE.md](TODO_ARCHIVE.md)**.
> The canonical current work queue (actionable, checkbox-ordered) is in
> **[docs/TODO_CURRENT.md](docs/TODO_CURRENT.md)**.

## Current status (as of 2026-06-19)

- **Version**: `0.1.4-dev` (tagged `v0.1.0`, `v0.1.1`, `v0.1.2`, `v0.1.3`; `v0.1.3` shipped
  2026-06-19 with system requirements tables + Debian Buster guide + test-count
  updates; `v0.1.2` shipped same day with Win64/AArch64 ABI, dialect purity,
  SOV fn/struct/enum, translator CLI v2, tutorials, examples reorganisation).
- **Tests**: 2421+ lib + 54 e2e parity-green on Linux and Windows.
- **Dialects**: 62 across 26 scripts.
- **Blocked**: macOS hardware, grammar consultant, proper IOCP, AArch64 CI.

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
