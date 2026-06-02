---
name: user-embedded-background
description: User comes from an embedded systems background and wants to use vāṇī on embedded targets — shapes design decisions around no_std, MMIO, and the embedded-only unsafe block.
metadata:
  type: user
---

User has an embedded systems background and is designing vāṇī
(`~/vani`, the Rust-inspired compiler with affine ownership, no
borrow-checker, no GC) with embedded targets as a first-class
planned use case — not an afterthought.

Implications for collaboration on vāṇī:

- When proposing features, weigh the embedded story explicitly
  (`no_std` mode, allocator dependence, stack budget, MMIO,
  interrupt safety, ISR calling conventions) rather than only the
  hosted-target view.
- The user wants `unsafe(reason = "...") { ... }` permitted
  **on embedded build triples only**, gated to embedded by the
  parser. Hosted builds reject the keyword at parse time. The
  `reason = "..."` clause is mandatory at parse time (locked
  2026-06-02); the reason string is emitted as IR/DWARF metadata
  so certification tooling (ASIL-D / DO-178C / IEC 62304) can
  extract deviation records from the compiled artifact. Inside
  `unsafe`, affine / Drop / move tracking still runs — only
  pointer-safety and type-punning invariants are suspended. See
  [[vani-embedded-position]] and `~/vani/unsafe.md` (4-layer v1
  generational-handles plan + v2 region-typing plan).
- Hardware-driver intuition (vendor SDKs, MMIO, ISRs, DMA
  buffers, linker sections, peripheral typestate, register-block
  layouts) is part of the user's mental model — references to
  these don't need ground-up explanation.
- Reason the user gave for permitting `unsafe`: a pure "no unsafe
  ever" policy loses to FFI-into-C in practice (vendor SDKs,
  custom DMA controllers, peripherals the language doesn't model
  all need *some* raw load/store path). FFI escapes affine
  tracking entirely, so a scoped
  `unsafe(reason = "...") { ... }` is the *more* auditable choice
  for irreducibly platform-specific operations — and the
  parser-enforced reason clause makes every escape grep-able and
  machine-extractable for certification review.
