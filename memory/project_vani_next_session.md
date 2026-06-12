---
name: project-vani-next-session
description: Next-session handoff for vani-compiler — open work, pick-up order, and edge test cases to add (current as of 2026-06-11)
metadata:
  type: project
---

Full handoff lives in STATUS.md "NEXT SESSION HANDOFF — 2026-06-11" block. This file is the quick-load version.

**Why:** Session paused 2026-06-11 after Windows full e2e parity landed (commit 6255af8). All Tier 1 + Tier 2 work shipped. Picking up here next time avoids re-deriving what's open.

**How to apply:** Read this at session start, then open STATUS.md for the detailed task descriptions and test names.

## Current state (2026-06-12)

- 2108 lib tests green (Windows + Linux)
- All e2e tests pass; 5 async-TCP tests skipped on Windows (IOCP gap)
- 62 dialects across 26 scripts
- Last commit: `826cf18` — "test: add integer overflow, ref/lifetime, and Windows regression edge tests"

## Work order for next session

### Step 1 — Add edge test cases (**MOSTLY DONE** — see below for remaining)

All go in `src/lib.rs` as `#[test]` unless noted.

**DONE — all shipped:**
- Integer overflow (×4 wrapping + ×1 const-error + ×1 compile-both) ✓
- Generic monomorphization (3-level chain, nongeneric bridge, two-call-sites) ✓
- OwnedStr match-arm mismatch (all-concat workaround, bare-literal-mismatch) ✓
- Ref / lifetime (3-ref-param reject, vec re-access after block, struct-ref method) ✓
- Windows regression (deep-recursion, llvm-printf-not-putchar) ✓
- echo_loop IOCP mismatch documented in e2e tests with `#[ignore]` ✓

**PENDING — need e2e binary execution (low priority):**
- `windows_brahmi_numeral_output_no_crt_reorder` — needs binary execution
- `windows_tcp_echo_blocking_three_clients` — needs live TCP server
- `windows_snprintf_dprintf_shim_roundtrip` — needs binary execution

### Step 2 — Pick one user-queued feature

| Feature | Effort | Notes |
|---|---|---|
| **volatile_read / volatile_write built-ins** | ~~4–6h~~ | **SHIPPED 2026-06-12** (commit `2cea04a`). 3 lib tests + examples/embedded/mmio_blink.vani. All 4 backends (AST C, AST LLVM, SSA C, SSA LLVM). |
| Error-message elaboration | 8–15h | src/checker.rs + src/diagnostic.rs — add elaboration vec, seed 20–30 families |
| Big-O annotation (--big-o flag) | 12–20h | New src/big_o.rs; hook into vanic check output; v1: loop-nesting + builtin asymptotics |
| Tutorials rewrite for non-CS readers | 20–40h | tutorials/src/beginner/ + intermediate/ — analogy chapters before formal definitions |

### Step 3 — Windows IOCP (larger arc, D.1 in TODO.md)

Root cause of skipped tests: sockets need `WSA_FLAG_OVERLAPPED` + `WSASend`/`WSARecv` with OVERLAPPED structs.
Entry points:
- `src/backend_llvm.rs` `emit_intent_epoll_helpers_llvm_windows`
- `examples/tcp_echo_epoll.vani`

### Deferred (do not touch unless asked)

- macOS empirical verification (no Darwin host)
- Arc 9 Kosh package manager (pending registry choice)
- CI / GH-Actions (Tier 4 — last)
- Grammar consultant pass (ongoing/external)

## Commit cadence reminder

Commit after every 2–3 tests pass so the user can push immediately. Don't batch to end of session.
See [[feedback-commit-cadence]].
