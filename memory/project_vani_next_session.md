---
name: project-vani-next-session
description: Next-session handoff for vani-compiler — open work, pick-up order, and edge test cases to add (current as of 2026-06-12)
metadata:
  type: project
---

Full handoff lives in STATUS.md "NEXT SESSION HANDOFF — 2026-06-12" block. This file is the quick-load version.

**Why:** Session paused 2026-06-12 after error-message elaboration arc. All Tier 1 + Tier 2 work shipped. Picking up here next time avoids re-deriving what's open.

**How to apply:** Read this at session start, then open STATUS.md for the detailed task descriptions and test names.

## Current state (2026-06-12, end of session 3)

- 2117 lib tests green (Windows + Linux)
- All e2e tests pass; 5 async-TCP tests skipped on Windows (IOCP gap)
- 62 dialects across 26 scripts
- Last commit: `de91e1d` — "feat(diagnostics): add duplicate_declaration family, wire 3 sites + 4 match-arm type_mismatch"
- Elaboration: **40 families**, **63 wired sites**, **16 elaboration tests**

## Work order for next session

### Step 1 — Continue error-message elaboration (IN PROGRESS)

Stats at session end: 40 families / 63 wired sites / 597 total diagnostic sites.

**High-value remaining targets in checker.rs (un-elaborated):**

| Site | Message | Suggested family |
|---|---|---|
| ~1226 | `"function '{}' is a built-in name and cannot be redefined"` | `duplicate_declaration` already exists |
| ~4443 | `"type alias '{}' is already declared"` | `duplicate_declaration` already exists |
| ~7828 | `"parameter '{}' is already defined"` | new `duplicate_parameter` |
| ~10411 | `"'reduce {}': name is not declared in scope"` | `unknown_variable` already exists |
| ~12121, ~12363 | `"match scrutinee is {}, but pattern is not a string/float literal"` | new `match_wrong_pattern_type` |
| Various | Struct literal field type mismatches beyond `struct_literal_missing_field` | `type_mismatch` already exists |

**Already shipped this session:**
- `missing_main_function`, `main_wrong_signature` ✓
- `unknown_function` ✓
- `struct_not_declared`, `enum_not_declared` ✓
- `duplicate_declaration` (struct/enum/function, 3 sites) ✓
- `type_mismatch` wired to all 4 match-arm body mismatch sites ✓
- 4 new elaboration families added (total: 40)

### Step 2 — Windows IOCP (larger arc, D.1 in TODO.md)

Root cause of skipped tests: sockets need `WSA_FLAG_OVERLAPPED` + `WSASend`/`WSARecv` with OVERLAPPED structs.
Entry points:
- `src/backend_llvm.rs` `emit_intent_epoll_helpers_llvm_windows`
- `examples/tcp_echo_epoll.vani`

### Step 3 — Pick one user-queued feature

| Feature | Effort | Notes |
|---|---|---|
| Big-O annotation (--big-o flag) | 12–20h | New src/big_o.rs; hook into vanic check output; v1: loop-nesting + builtin asymptotics |
| Tutorials rewrite for non-CS readers | 20–40h | tutorials/src/beginner/ + intermediate/ — analogy chapters before formal definitions |

### Deferred (do not touch unless asked)

- macOS empirical verification (no Darwin host)
- Arc 9 Kosh package manager (pending registry choice)
- CI / GH-Actions (Tier 4 — last)
- Grammar consultant pass (ongoing/external)

### Pending e2e-only edge tests (low priority)

- `windows_brahmi_numeral_output_no_crt_reorder` — needs binary execution
- `windows_tcp_echo_blocking_three_clients` — needs live TCP server
- `windows_snprintf_dprintf_shim_roundtrip` — needs binary execution

## Commit cadence reminder

Commit after every 2–3 tests pass so the user can push immediately. Don't batch to end of session.
See [[feedback-commit-cadence]].
