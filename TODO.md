# vāṇī — Work Queue

> This file has been condensed on 2026-06-19.
> Pre-v0.1.0 planning history is in **[TODO_ARCHIVE.md](TODO_ARCHIVE.md)**.
> The canonical current work queue (actionable, checkbox-ordered) is in
> **[docs/TODO_CURRENT.md](docs/TODO_CURRENT.md)**.

## Current status (as of 2026-07-13)

- **Version**: `0.4.4` (tagged v0.1.0 through v0.4.4; see RELEASING.md for full history).
- **Tests**: 2466+ lib tests passing.
- **Dialects**: 62 across 26 scripts.
- **Blocked**: macOS hardware, grammar consultant, proper IOCP, crates.io API token.

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
