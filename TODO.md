# vāṇī — Work Queue

> This file has been condensed on 2026-06-19.
> Pre-v0.1.0 planning history is in **[TODO_ARCHIVE.md](TODO_ARCHIVE.md)**.
> The canonical current work queue (actionable, checkbox-ordered) is in
> **[docs/TODO_CURRENT.md](docs/TODO_CURRENT.md)**.

## Current status (as of 2026-06-19)

- **Version**: `0.1.2-dev` (tagged `v0.1.0` and `v0.1.1`; `v0.1.1` shipped
  2026-06-18 with Barrier, RwLock<T>/ReadGuard/WriteGuard, parametric
  Mutex<T>/Channel<T>, Traits phase 2, kosh config).
- **Tests**: ~2091 lib + 54 e2e parity-green on Linux and Windows.
- **Dialects**: 62 across 26 scripts.
- **Blocked**: macOS hardware, grammar consultant, IOCP, AArch64 CI.

## Open items (summary)

See [docs/TODO_CURRENT.md](docs/TODO_CURRENT.md) for the full checkbox list.
Immediate items:

| # | Item | Effort |
|---|------|--------|
| 1 | Publish to crates.io (`cargo publish`) | < 1 h |
| 3 | Remove `intentc` legacy binary at v0.2 boundary | < 1 h |
| 11 | A.2 Examples reorganization (language subfolders) | 2–4 h |
| 12 | Arc 7 Win64/AArch64 ABI classifier | 6–8 h |
| 13 | Finer Sanskrit/Hindi/Marathi purity gate | 4–8 h |
| 15 | Homebrew formula (gated on macOS verification) | dedicated session |
| 16 | Cross-language `.vani` translator CLI | 4–6 h |
| 17 | SOV completion (mechanical parser side) | 10–15 h |

## Blocked items

| Item | Blocker |
|---|---|
| macOS empirical verification | Darwin hardware needed |
| Grammar consultant pass | External native-speaker review |
| Windows IOCP async-TCP | Readiness-vs-completion model mismatch (R8 in docs/decisions.md) |
| Arc 7 Win64/AArch64 CI wiring | CI runner setup |
