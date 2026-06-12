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

- 2100 lib tests green (Windows + Linux)
- All e2e tests pass; 5 async-TCP tests skipped on Windows (IOCP gap)
- 62 dialects across 26 scripts
- Last commit: `2cea04a` — "feat(embedded): add volatile_read / volatile_write ref-based builtins"

## Work order for next session

### Step 1 — Add edge test cases (low effort, do first, commit per 2–3 tests)

All go in `src/lib.rs` as `#[test]` unless noted.

**Windows regression (prevent future breakage):**
- `windows_deep_recursion_no_stack_overflow` — ≥800 recursive calls, no overflow
- `windows_snprintf_dprintf_shim_roundtrip` — FFI to snprintf/dprintf, check output
- `windows_tcp_echo_blocking_three_clients` — explicit Windows tcp_echo.vani assertion

**Integer overflow (documents wrapping, prevents accidental "fix"):**
- `i64_max_plus_one_wraps_to_min` — 9223372036854775807 + 1 == -9223372036854775808, both backends agree
- `i64_min_minus_one_wraps_to_max`
- `i64_min_times_neg_one_wraps_to_min`
- `u64_max_plus_one_wraps_to_zero`
- `const_overflow_is_a_type_error` — const N: i64 = MAX + 1 must be rejected

**Generic monomorphization:**
- `nested_generic_three_level_chain_fails` — h<T>→g<T>→f<T>, only h<i64> from non-generic → compile error
- `nested_generic_nongeneric_bridge_works` — h<T> calls non-generic bridge() which calls f<i64> → should compile
- `nested_generic_same_type_two_call_sites` — f<i64> has both a non-generic and generic call site → should compile

**OwnedStr / match arms:**
- `ownedstr_all_arms_must_produce_same_type` — mixing Str literal arm with OwnedStr arm is a type error
- `ownedstr_nested_match_concat_workaround` — nested match produces OwnedStr on all paths

**Ref / lifetime:**
- `ref_return_three_ref_params_rejects` — extend existing 2-param test to 3 params
- `vec_ref_push_after_source_borrow_ends` — re-borrow after first borrow ends is allowed
- `struct_field_ref_lifetime_survives_method_call` — struct holding ref T field, method reads it

**Async state machine (Windows mismatch documentation):**
- `echo_loop_windows_byte_count_matches_c` — add a Windows `#[cfg]` + `#[ignore]` variant that documents the mismatch rather than silently skipping it forever

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
