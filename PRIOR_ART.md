# Defensive Prior Art Disclosure — vāṇी Compiler

**Project**: vāṇी compiler (`vani-compiler`)
**Maintainer**: Pratik M. Tambe &lt;enthusiasticgeek@gmail.com&gt;
**Purpose**: This document is a **defensive publication**. It publicly
discloses novel techniques implemented in this project so that no person
or entity may subsequently obtain a patent covering them. Under 35 U.S.C.
§102 and equivalent provisions in most jurisdictions, publication of an
invention before a patent application is filed constitutes prior art that
bars the grant of a patent on that invention.

Every entry below was first publicly disclosed via a commit to a public
GitHub repository. The commit hash and date serve as timestamped,
publicly accessible evidence of prior art. This document itself is
additionally published to reinforce and summarize those disclosures.

> **Note**: This is not a patent, trademark, or copyright claim. It is a
> deliberate disclosure to create and preserve prior art in the public domain.

---

## How to use this document

If a patent application is filed on any technique described below:

1. The commit date and hash constitute prior art evidence predating any
   filing made after those dates.
2. The Maintainer or any third party may submit this document and the
   cited commit as prior art evidence to the USPTO (or EPO / JPO / IPO)
   during examination or via inter partes review (IPR).
3. Contact &lt;enthusiasticgeek@gmail.com&gt; immediately if you detect a
   relevant patent filing.

---

## Disclosed Techniques

### PA-1 — vāṇी programming language design and compiler architecture

**First public disclosure**: 2026-05-21
**Git evidence**: Initial commit "vāṇी (VANI) — Verbose Alternative Natural Interface"

The vāṇी programming language is a statically typed, affine-ownership,
Devanagari-keyword-compatible language with the following novel
combination of properties:

- Dual compiler backends (C source emission and LLVM IR emission) from a
  single typed intermediate representation, enabling cross-platform and
  cross-architecture compilation without a separate frontend per backend.
- An affine type system (each value used at most once unless marked Copy)
  integrated with a scope-exit Drop system that emits deterministic
  destructor calls in both the C and LLVM backends.
- A formal-verification layer (SMT-based bounds checking via
  `src/verifier.rs`) embedded in the compilation pipeline that discharges
  array-bounds and integer-range obligations before emitting runtime guards,
  reducing unnecessary runtime overhead while preserving safety.
- Devanagari script keyword aliases for every reserved word, enabling
  source programs written in Sanskrit-derived vocabulary to compile
  identically to ASCII-keyword equivalents.
- A unified `TypedIR` (typed intermediate representation) that is shared
  between the type checker, the SSA lowerer, the verifier, and both code
  generators.

---

### PA-2 — Cross-backend SIMD type system (`vec128<T>`)

**First public disclosure**: 2026-07-10
**Git evidence**: "feat(simd): Option 3 — native vec128<T> type with 7 SIMD builtins"

A language-level 128-bit SIMD register type (`vec128<T>`) with a fixed
set of 7 portable builtins (`simd_splat`, `simd_load`, `simd_store`,
`simd_add`, `simd_sub`, `simd_mul`, `simd_reduce_add`) that:

- Are typed at the language level (the type checker validates element-type
  consistency across all builtin call sites).
- Emit different but semantically equivalent code from each backend:
  the C backend emits GCC/Clang vector extensions
  (`__attribute__((vector_size(16)))`); the LLVM backend emits LLVM
  vector IR (`<N x T>`) that LLVM lowers to the target ISA's SIMD
  registers (SSE/AVX2 on x86-64, NEON on AArch64, RVV on RISC-V).
- Are Copy values in the affine type system (SIMD registers do not own
  heap memory, so they are freely copyable without violating the affine
  invariant).
- Support chained `simd_store` calls by returning `Vec<T>` from store
  operations, enabling pipeline expressions without intermediate bindings.

This combination — language-level SIMD types that (a) carry affine-system
Copy semantics, (b) are backend-polymorphic, and (c) reduce to ISA-native
instructions on each target — is novel as of 2026-07-10.

---

### PA-3 — 256-bit SIMD register type (`vec256<T>`) and `simd256_*` builtins

**First public disclosure**: 2026-07-10
**Git evidence**: "SIMD-9: add vec256<T> + simd256_* builtins (256-bit SIMD)"

An extension of PA-2 to 256-bit width. `vec256<T>` carries:

- Double the lane count of `vec128<T>` for each element type
  (e.g. `vec256<f32>` = 8 lanes vs 4).
- Seven parallel builtins (`simd256_splat`, `simd256_load`,
  `simd256_store`, `simd256_add`, `simd256_sub`, `simd256_mul`,
  `simd256_reduce_add`) that share no name with the 128-bit set,
  enabling both widths to coexist in a single source file.
- LLVM IR type `<N x T>` with 32-byte alignment; on x86-64+AVX2 LLVM
  lowers this to `ymm` registers; on AArch64 without SVE, LLVM legalises
  to two 128-bit NEON registers; on RISC-V V extension (VLEN≥256), to a
  single vector register.
- Stack-depth accounting of 32 bytes (vs 16 for `vec128<T>`).

The specific design of a named 256-bit SIMD type in an affine type system
with portable builtins and ISA-adaptive backend lowering is novel as of
2026-07-10.

---

### PA-4 — `#[vectorize]` software-pipelining attribute

**First public disclosure**: 2026-07-09
**Git evidence**: "feat: SIMD support — #[vectorize] attribute + FFI shim docs"

A function-level attribute (`#[vectorize]`) that injects LLVM loop
vectorization metadata (`!llvm.loop.vectorize.enable`,
`!llvm.loop.interleave.count`) into every `while` loop in the annotated
function body. The interleave count is fixed at 4 by default (software
pipelining). This is distinct from auto-vectorization in that:

- It is explicitly requested by the programmer, not inferred.
- The interleave count is controlled by the attribute (extendable to
  `#[vectorize(interleave=8)]`).
- It composes with the explicit `vec128<T>` / `vec256<T>` SIMD types:
  a function may use both `#[vectorize]` on scalar loops and explicit
  SIMD builtins on the hot path, with the compiler scheduling across both.

---

### PA-5 — `parallel for … reduce` with thread-local accumulation and atomic combine

**First public disclosure**: 2026-07-03
**Git evidence**: "v0.5: thread-local accumulation in parallel-for reduces"

A parallel-for construct that lowers to OpenMP or POSIX threads with the
following specific lowering strategy:

- Each worker thread accumulates into a **non-atomic stack-local
  accumulator** during the parallel body (avoiding per-element atomic
  read-modify-write operations).
- A single `atomicrmw` (or CAS loop for non-commutative operators such
  as `*`) combines each thread's local accumulator into the shared result
  at the **exit of the parallel region**, not at each element.
- The LLVM IR emits `@llvm.lifetime.start` / `@llvm.lifetime.end` markers
  around the per-thread stack accumulators and `alwaysinline` on the
  outlined parallel body function to enable LICM and
  ConstraintElimination across thread-boundary calls.

This specific combination of thread-local accumulation with a single
atomic combine at region exit (rather than per-element atomic or reduction
variable) as the lowering strategy for a language-level `reduce` clause
is novel as of 2026-07-03.

---

### PA-6 — QEMU system-mode bare-metal integration via `--qemu-machine`

**First public disclosure**: 2026-07-10
**Git evidence**: "feat(run): add --qemu-machine=<board> for bare-metal system-mode QEMU"

A compiler CLI flag (`--qemu-machine=<board>`) that:

1. Accepts a bare-metal LLVM target triple (e.g. `arm-none-eabi`,
   `riscv32-unknown-none-elf`) and a board name (e.g. `lm3s6965evb`,
   `sifive_e`).
2. Compiles the source program to an ELF file via the LLVM backend.
3. Discovers the correct `qemu-system-*` binary via an internal
   board-to-QEMU-command map (`board_to_qemu_cmd`) that sets
   `-semihosting` for ARM boards and `-bios none` for RISC-V boards.
4. Invokes the QEMU system-mode emulator with `-machine <board> -kernel
   <elf>` and streams the exit code back to the user.
5. Supports an env-var override (`QEMU_SYSTEM_<ARCH>`) to pin a specific
   QEMU binary version in CI.

The combination of (a) a language-level compiler flag that abstracts over
board-specific QEMU arguments, (b) automatic semihosting/bios flag
selection by architecture, and (c) ELF → QEMU invocation with streaming
exit code — integrated into a compiler `run` subcommand — is novel as of
2026-07-10.

---

### PA-7 — AArch64 QEMU user-mode CI for a pure-Rust compiler with no native LLVM library dependency

**First public disclosure**: 2026-07-09
**Git evidence**: "feat: ARM-6 — AArch64 CI via QEMU user-mode"

The vāṇī compiler emits LLVM IR as text and invokes `lli` / `llc` as
external subprocesses, rather than linking against LLVM's C++ libraries.
This architectural choice means the compiler binary itself is pure Rust
with no native (C++) shared-library dependency, making it cross-compilable
to any Rust target triple without a matching LLVM SDK. The CI configuration
exploits this property to run the full compiler unit test suite on emulated
AArch64 and RISC-V 64-bit hardware via QEMU user-mode, using
`CARGO_TARGET_*_RUNNER` to transparently execute cross-compiled test
binaries. This technique for validating a compiler's own internal logic on
a foreign ISA without cross-building the LLVM toolchain is novel as of
2026-07-09.

---

### PA-8 — Affine ownership with automatic scope-exit Drop in both C and LLVM backends

**First public disclosure**: 2026-05-21
**Git evidence**: "Bounded generics, affine struct fields, auto-Drop at scope exit" (2026-05-21)

An affine type system (each binding used at most once unless the type
implements `is_copy()`) where:

- The type checker inserts `TypedStmt::Drop(binding)` nodes at every
  scope exit for bindings of non-Copy type.
- Both the C backend (`backend_c.rs`) and the LLVM backend
  (`backend_llvm.rs`) consume these Drop nodes and emit type-appropriate
  destructor calls (`free()` for `Box<T>`, recursive field drops for
  structs and enums containing heap-owning fields, etc.).
- SIMD register types (`vec128<T>`, `vec256<T>`) are exempt from Drop via
  `is_copy() → true`, correctly modeling the semantics of CPU registers
  that do not own heap memory.
- The SSA lowerer preserves Drop nodes through its CFG construction so
  that destructor calls remain correctly ordered in the presence of
  branching, early returns, and loop exits.

The dual-backend (C + LLVM) implementation of affine Drop with unified
IR-level Drop nodes is novel as of 2026-05-21.

---

### PA-9 — Vectorize-width target-aware hint emission

**First public disclosure**: 2026-07-09
**Git evidence**: "feat: ARM-1 + ARM-2 + ARM-5 — target-aware vectorize hints, --cpu= flag"

The LLVM backend inspects the active LLVM target triple and emits
different `!llvm.loop.vectorize.width` metadata values based on the
target architecture:
- AArch64 targets: width = 2 (NEON 128-bit, f64 lanes)
- x86-64 targets: width = 4 (SSE 128-bit, f32 lanes)
- Other targets: width = 4 (default)

This per-target width selection, applied automatically without programmer
annotation, is novel as of 2026-07-09.

---

## How to add to this document

When a new novel technique is implemented, add an entry here **before
merging** the feature PR. Include:
- A `PA-N` identifier
- A plain-language description of the technique
- The earliest public disclosure date (the PR merge date if the feature
  was developed in a private branch, or the commit date if developed
  directly on `main`)
- The git commit hash or message

Maintainer: treat a pull request that implements a novel algorithm as
incomplete until a `PRIOR_ART.md` entry is added.

---

*This document is a living record. All techniques listed here are in the
public domain by reason of publication. Last updated: 2026-07-10.*
