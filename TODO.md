# vāṇī — Work Queue

> This file has been condensed on 2026-06-19.
> Pre-v0.1.0 planning history is in **[TODO_ARCHIVE.md](TODO_ARCHIVE.md)**.
> The canonical current work queue (actionable, checkbox-ordered) is in
> **[docs/TODO_CURRENT.md](docs/TODO_CURRENT.md)**.

## Current status (as of 2026-08-12)

- **Version**: `0.9.7-dev` (tagged v0.1.0 through v0.9.7-dev; see RELEASING.md for full history).
  (2026-08-12). See [RELEASING.md](RELEASING.md) for the full version
  history and [CHANGELOG.md](CHANGELOG.md) for release-by-release detail.
- **Tests**: 2906 lib tests passing, **0 failing**, plus 12 other test
  suites (integration tests, cross-target QEMU runs, the ASan/LSan/UBSan
  example-corpus sweep, the `vani_translate.py` regression suite) all
  green in CI.
- **Dialects**: 62 across 26 scripts (63 including English) — see
  [docs/languages.md](docs/languages.md) for per-dialect verification
  status.
- **Blocked**: macOS hardware, grammar consultant, AArch64/RISC-V
  benchmark hardware, proper IOCP, crates.io API token — unchanged since
  the last update (see "Blocked items" below).
- **No known bugs remain open.** BUG-1 through BUG-184 are all fixed —
  see [docs/TODO_CURRENT.md](docs/TODO_CURRENT.md) for the full,
  chronological per-bug writeup (this file used to inline a running
  highlights summary here; that became unmaintainable well before
  BUG-184, so this section now just points at the real log instead of
  drifting further out of sync with it). Recent milestones: three
  patch releases since this section was last updated (v0.9.1, v0.9.2,
  v0.9.3), a 3-bug soundness series in the SMT bounds-elision pass
  shared by every backend (BUG-181/182/183 — a stale-fact class of bug
  that let the compiler prove a provably-unsafe array index "safe"),
  and `tools/vani_translate.py` (translate `.vani` source between all
  63 dialects) going from silently broken for ~24% of its claimed
  language coverage to fully regression-tested in CI.

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
