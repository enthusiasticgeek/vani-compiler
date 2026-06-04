- [vāṇī backend state](project_vani_backend.md) — pipeline, backends, verifier, language surface, current closures landed (refreshed #1-#291 + Arcs 1-9 thru 2026-06-04: closure-as-value, async fn / await / Future / Poll / CancelToken at parser+prelude layer, full SysV float-class FFI)
- [vāṇī STATUS.md update protocol](project_vani_status_file.md) — single-page feature set + TODOs + known issues file; update on every commit that changes any of those three
- [vāṇī design philosophy](feedback_vani_design_philosophy.md) — composition > inheritance, vtables = original intent only, build data structures from Vec, keep language minimal
- [vāṇī file access standing approval](feedback_vani_file_access.md) — read/write under /tmp and ~/vani without re-asking, plus cargo / intentc / git commit autonomy
- [vāṇī language design directions](feedback_vani_language_design.md) — file extension `.vani`, per-file language purity, within-language aliases expected, Cranelift/x86_64-asm deprioritized
- [vāṇī affine ownership — standing v1 decision](project_vani_affine_standing.md) — every container / algorithm / API must carry ✅ AFFINE / ⚠️ AFFINE-TENSION / 🛑 NON-COMPLIANT flag with reasoning
- [vāṇī data structures + algorithms roadmap](project_vani_data_structures_roadmap.md) — Levels 1-4 sequenced (sort / find / HashMap / BTree / Deque / BinaryHeap / closures / iterators / arena-based trees + graphs); all flagged
- [vāṇī container API affine contract](project_vani_container_affine_contract.md) — get / insert / remove / iter shapes for Map / Set / Deque / Heap under single-owner
- [vāṇī condition variables (Condvar) design](project_vani_condvar_design.md) — pairs with Mutex<T> + Guard<T>; futex / WaitOnAddress / pthread-cond codegen; ✅ AFFINE; single-session M effort
- [vāṇī async / asyncio design](project_vani_async_design.md) — compiler-lowered state machines on arena; explicitly NOT Pin / self-references. **Arc 8 FULLY COMPLETE 2026-06-04** on Linux — v1 source surface + v1.5 timers + v1.6 blocking TCP + v2 epoll + non-blocking I/O + v3 async-flavored aliases + hand-rolled state-machine pattern. Five parity-green examples. **v3.1 compiler sugar OPTIONAL** with 15 documented design caveats (linear-body-only initially, local liveness, affine-types-across-await, ANF lifting, etc). **Platform support: Linux only today** — macOS port needs kqueue shim (~8-12h), Windows needs full IOCP redesign (~25-35h)
- [vāṇī embedded position](project_vani_embedded_position.md) — explicit `unsafe { ... }` permitted on embedded build triples only; hosted rejects keyword at parse time; affine still active inside `unsafe`. Implementation plan now lives in `~/vani/unsafe.md`.
- [User embedded background](user_embedded_background.md) — user comes from embedded systems; embedded is first-class planned target for vāṇī, not an afterthought
- [vāṇī safety-standard alignment](project_vani_safety_standards.md) — two-tier attribute family (`#[no_heap]` / `#[asil_d]` / etc.) bringing MISRA C 2012 / ISO 26262 ASIL-D / DO-178C Level A / IEC 62304 Class C feasibility. Compose by union; opt-in plus global env-var modes; compile-with-and-without parity. Scheduled before ARCs. Full plan in `~/vani/TODO.md` § *Safety-standard alignment*.
- **External plan-of-record docs at `~/vani/`** (refreshed 2026-06-04):
  - `~/vani/ARC8_V3_PLAN.md` — **phased execution plan** for Arc 8 v3.1 compiler-driven sugar (5 phases, ~78-98h) + Arc 8 platform port (2 phases, ~35-50h). **Phase 0 + 1 + 2 narrow + 2.1a ✅ COMPLETE 2026-06-04** — compiler-driven `async fn → Task` transform handles linear bodies + non-suspending control flow + suspend-in-branch state-splitting (both branches return-terminated). 5 v3.1 acceptance examples parity-green. Next session: Phase 5 (macOS port ~10-15h) or Phase 2.1b (fall-through merge state ~3-5h).
  - `~/vani/unsafe.md` — 5-layer embedded-safety plan. **✅ FULLY SHIPPED 2026-06-02**.
  - `~/vani/TODO.md` § *Safety-standard alignment* — **✅ FULLY SHIPPED 2026-06-03** (all three tiers + four standard composites on `main`).
  - `~/vani/ARCS.md` — granular sub-step plan for Arcs 1–10. **Arcs 1–6 + 7 SysV + 8 v1 + 9 c/d ✅ COMPLETE thru 2026-06-04.** Open queue: **Arc 8 runtime (8c+8d+8e+8h)** — focused next-session arc; STATUS.md "📋 NEXT SESSION" block carries the verbatim handoff prompt. Arc 9 a/b/e/f deferred pending registry choice; Arc 10 blocked on grammar consultant.

<!--
Consolidation note (2026-05-25):
- vāṇī work formerly ran from ~/shortcut-mcp-server cwd; canonical home is now ~/vani/.
- Memory moved here from ~/.claude/projects/-home-ptambe-shortcut-mcp-server/memory/.
- ~/shortcut-mcp-server/future-compiler/ empty stub removed.
- ~/shortcut-mcp-server/.claude/settings.json stripped of 61 vāṇī-related Bash allowlist entries and the /home/ptambe/future-compiler additionalDirectories entry.
- Historical session JSONL logs under ~/.claude/projects/-home-ptambe-shortcut-mcp-server/ deleted (active session log retained while runtime writes to it).
- Future vāṇī sessions should launch with cwd = ~/vani/.
-->

