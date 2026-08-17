---
name: vani-safety-standards
description: vāṇī's safety-standard alignment arc — `#[no_heap]` / `#[asil_d]` / etc. attribute family + `intentc deviations` extractor for MISRA C 2012 / ISO 26262 ASIL-D / DO-178C Level A / IEC 62304 Class C feasibility.
metadata:
  type: project
---

**Fact:** Following the 2026-06-02 user review of MISRA / AUTOSAR /
ISO 26262 / DO-178C / IEC 62304 standards against vāṇī's current
surface, a safety-standard alignment arc is scheduled BEFORE the
ARCs.md multi-session work (HashMap monomorph / Trie sparse /
closures / wider K-V).

**Why:** unsafe.md is now fully shipped (Layers 1.1–5, 18 commits
2026-06-02). The remaining gap to credible MISRA / ASIL-D /
DO-178C / IEC 62304 marketing is (a) a deviation extractor for
the IR metadata already on `main` and (b) an attribute family that
lets users opt fns into standard-specific constraint sets. Lining
up these primitives BEFORE ARC work means the ARCs (HashMap /
Trie / closures) can be designed with the standard surface
locked, rather than retrofitting after.

**How to apply:**

- The plan-of-record is `~/vani/TODO.md` § *Safety-standard
  alignment* (committed at `bb5b78f`, 2026-06-02). Refer there
  for the full primitives list, composite definitions, and tier
  ordering.
- **Compose-by-union semantics**: a fn with both `#[asil_d]` and
  `#[no_float]` gets the composite's primitives ∪ `{no_float}`.
  Most restrictive wins. Two composites stack the same way.
- **Compile-with-and-without parity is mandatory.** Without any
  tag set or env var, vāṇī behaves exactly as today — no
  compile-time perf or behavior change. Strictness is purely
  opt-in via per-fn tags or global env vars (`INTENT_NO_HEAP=1`,
  etc.).
- **Composites hardcoded** in the compiler for v1; user-facing
  vani.toml override is a future enhancement.
- **Deviations report includes a `target_standard` column** —
  each row from `intentc deviations` tags which composite (or
  `none`) the enclosing fn was annotated for. Lets reviewers
  filter by standard target.

**Tier-1 (~10h, scheduled before ARCs):**
1. `intentc deviations` extractor (CSV + JSON + human-readable)
2. `#[no_heap]` attribute + `INTENT_NO_HEAP=1` global mode
3. Stack-depth bound checker (`intentc stack-depth`)

**Tier-2 (~25h, after Tier 1):** `Mmio<T, ADDR>`, `#[interrupt]`,
`#[no_float]`, complexity warning, `#[no_recursion]` strict,
pointer-arith diagnostic.

**Tier-3 (each multi-day, after Tier 2):** `#[wcet]`,
`#[bounded_stack]`, call-graph acyclicity, `#[deterministic_timing]`,
`pure fn` → MISRA 13.1/13.2 tightening.

**Standard composites** (hardcoded primitives expansion):
- `#[misra_c_2012]` → `no_heap, no_recursion, no_goto_implicit, no_setjmp`
- `#[asil_d]` (ISO 26262) → `no_heap, no_recursion, bounded_stack, wcet, no_unsafe`
- `#[do178c_level_a]` → `no_heap, no_recursion, bounded_stack, wcet, deterministic_timing`
- `#[iec_62304_class_c]` → `no_heap, no_unsafe, bounded_stack`
- `#[autosar_cpp14]` — not a direct vāṇī map (C++-specific)

**Status (2026-06-02):** plan-only commit landed; implementation
queued. User said "list tasks, ask before coding." No code in
flight.

Related: [[vani-embedded-position]], [[user-embedded-background]],
[[vani-affine-standing]].
