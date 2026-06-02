---
name: vani-embedded-position
description: vāṇī's standing position on embedded targets — explicit `unsafe { ... }` opt-in for embedded build triples only, hosted stays fully checked. Recorded 2026-06-01; expanded 2026-06-02 with the unsafe.md memory-safety plan (Handle<T> v1, regions v2).
metadata:
  type: project
---

**2026-06-02 update — plan-of-record for `unsafe`-block memory
safety:** [~/vani/unsafe.md](../unsafe.md). Hybrid path chosen:

- **v1 (ships first, ~22–31h, ~12 commits):** generational handles.
  `Handle<T>` is a `(slot_idx, generation)` opaque ref; `Pool<T>` is
  the backing slot pool. Use-after-free caught at runtime by
  generation mismatch (~3–5 cycles per deref on Cortex-M).
  Syntactically invisible to mainstream embedded users; low learning
  curve. Plus compile-time layer (lexical unsafe containment,
  no-escape on &local, Tainted<T> wrapper) + canaries + toolchain
  flags.
- **v2 (queued after v1 stabilizes, ~15–25h, ~8 commits):** region
  typing. `region { ... }` blocks introduce arenas; `&'arena T`
  pointers carry compile-time use-after-free proof at zero runtime
  cost. Targeted at safety-critical certification (ASIL-D, DO-178C,
  IEC 62304) where runtime checks aren't acceptable.
- **Coexistence:** users pick per-type. Handle<T> is the
  ergonomic default; `&'arena T` is the opt-in zero-cost path. Code
  written against Handle<T> stays valid forever — no big-bang
  migration. One-way Handle ↔ &'arena conversions allowed under
  documented rules.
- **Skipped intentionally:** Rust-style borrow checker (user has a
  simpler one), garbage collector (user requirement), capability
  tokens (overlaps with affine), ASAN/MSan (too heavy for embedded).
- **Keyword form (decided 2026-06-02):**
  `unsafe(reason = "...") { ... }` — mandatory non-empty reason
  clause, parser-enforced. Reason stored on the AST and threaded
  through IR/DWARF so certification tooling (ASIL-D, DO-178C,
  IEC 62304) can extract deviation records from the compiled
  artifact. NOT a `// SAFETY:` comment convention — the
  justification is part of the syntax, can't be silently deleted.
  Recommended reason-prefix conventions for tooling: `"MMIO: ..."`,
  `"FFI: ..."`, `"DMA: ..."`, `"transmute: ..."`,
  `"vendor-SDK: ..."`.

The original *position* below stands unchanged — `unsafe` is
embedded-only, hosted rejects at parse time, affine still runs
inside `unsafe`. unsafe.md just adds the **how** for the
pointer-safety story.

---

**Fact:** vāṇī (`~/vani`) treats embedded (`no_std`, bare-metal,
MCU) as a first-class planned target. v1 ships hosted only;
embedded is on the queue. The standing policy on the
unsafety question:

- `unsafe { ... }` is permitted **only on embedded build
  triples**. Hosted builds (Linux / Windows / macOS) reject
  the keyword at parse time.
- Inside `unsafe`, the affine checker **still runs**: moves,
  drops, locks, ISR-body restrictions, effect typing, and
  Drop emission all stay active. `unsafe` only suspends:
  - Pointer-safety (raw `*const T` / `*mut T`, pointer
    arithmetic).
  - Type punning (`transmute`-like reinterpretation).
- What `unsafe` is reserved for (the residual the typed
  primitives can't express):
  - Raw MMIO outside `Register<T, ADDR>` / `Mmio<T>`.
  - Inline assembly and platform intrinsics.
  - Vendor SDK FFI the checker can't verify.
  - Custom linker-placed memory ranges and fixed-address
    peripherals the build target doesn't model.
- Goal is **rare** `unsafe` even on embedded — well-written
  vāṇी firmware should have zero or near-zero `unsafe`
  blocks in application code, with the few that exist
  confined to vendor-HAL crates.

Position recorded in [README.md](~/vani/README.md) →
*Embedded targets — current position* and [TODO.md](~/vani/TODO.md)
→ *Embedded targets — design considerations*.

**Why:** User has embedded background (see
[[user-embedded-background]]) and wants the language usable for
real driver / firmware code without forcing a fallback to C+FFI.
Pure "no unsafe ever" loses to FFI-into-C in practice — FFI
escapes affine tracking entirely, so a scoped `unsafe { ... }`
block is the *more* auditable choice for irreducibly
platform-specific operations.

**How to apply:**

- Do not propose hosted `unsafe`. The hosted invariant —
  "every operation checked" — is non-negotiable.
- Do not propose features that weaken affine guarantees inside
  `unsafe`. The block suspends a *narrow* set of invariants by
  design.
- When a new embedded need surfaces, first ask "can a typed
  primitive cover this?" — `unsafe` is the residual, not the
  default. Candidate typed primitives queued for design:
  `Register<T, ADDR>`, `Mmio<T>`, `interrupt fn` calling
  convention, `place_at "section"` attribute, typestate pins,
  bit-precise types.
- Borrow-checking is explicitly **not** the embedded gap —
  affine + Z3 covers memory safety. Real gaps are platform
  capabilities (MMIO, interrupts, linker sections, stack
  budgets) plus the narrow `unsafe` residual.
- High-payoff compile-time mechanisms recorded in TODO.md
  for the embedded design (reuse machinery the language
  already has):
  1. Effect / capability typing (generalize `pure fn`).
  2. Stack-bound proofs (reuse Z3).
  3. Typestate via phantom generics (reuse monomorphization).
