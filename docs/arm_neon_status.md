# ARM / AArch64 / NEON status in vāṇī (v0.2.4)

> Written 2026-07-06. Current as of v0.2.4.

---

## What works today

### Cross-compilation targets (`--target=`)

Pass an LLVM target triple to `vanic build` and it forwards it to `llc`:

```
vanic build firmware.vani --target=arm-none-eabi         -o firmware.elf
vanic build server.vani   --target=aarch64-unknown-linux-gnu -o server
```

Explicitly tested triples:

| Triple | ISA | Use case |
|--------|-----|----------|
| `arm-none-eabi` | Thumb-2 (ARMv7-M+) | bare-metal; Cortex-M3/M4/M7 |
| `thumbv7em-none-eabihf` | ARMv7E-M + HW FPU | bare-metal with FPU |
| `aarch64-unknown-linux-gnu` | AArch64 | Linux userspace (Graviton, Pi 4, M-series) |

### Running natively on an AArch64 host

`cargo build --release` on an ARM64 machine produces a native `vanic` binary.
`backend_llvm.rs` detects `cfg!(target_arch = "aarch64")` and adjusts the
default stack-depth limit accordingly (202 on x86-64, 98 on AArch64).

### NEON auto-vectorization (implicit)

vāṇī emits standard LLVM IR. When `llc` targets AArch64, LLVM's auto-vectorizer
uses NEON (128-bit) registers for loops that carry `!llvm.loop.vectorize.enable`
metadata — including all `parallel for … reduce` bodies and the `vec_fill` fill
loop. No explicit NEON intrinsics needed for basic numeric reductions.

### ARM Memory Tagging Extension (MTE)

A dedicated code path in `main.rs` emits `-march=armv8.5-a+memtag` when
building for ARMv8.5+ targets with memory tagging enabled. This lets the
hardware tag every pointer and catch use-after-free at runtime.

### Bare-metal (`--no-std`)

`--no-std` suppresses libc imports and the `malloc`/`free` declarations.
Linking uses the cross-linker (`arm-none-eabi-gcc`, `aarch64-linux-gnu-gcc`, etc.)
derived from the target triple.  See `tutorials/src/advanced/04b_cross_compile_primer.md`
for the full flow.

---

## Known gaps and limitations

### 1. `vectorize.width` hint is x86-biased

Every reduction loop emits `!llvm.loop.vectorize.width = 4`. On x86-64 AVX2
this maps to 4×64-bit = 256-bit (one YMM register). On AArch64 NEON, a
128-bit register holds only **2×i64**, so width 4 forces two registers and
may confuse the vectorizer. The correct hint for i64 on AArch64 is `width = 2`;
for i32 it is `width = 4`.

### 2. No explicit NEON / SVE intrinsics

There is no user-visible `@neon_vaddq_s64(…)` or `#[target_feature(neon)]`
surface. All SIMD comes from LLVM auto-vectorization. Programs that need hand-
tuned NEON (e.g., cryptography, image processing) must drop to `unsafe { … }`
FFI calling a hand-written C/assembly function.

### 3. `parallel for` unavailable on bare-metal ARM

`parallel for … reduce` emits `CreateThread` (Windows) or `pthread_create`
(POSIX). Neither exists on bare-metal (`--no-std` + `arm-none-eabi`).
Workaround: manual work-splitting via interrupt-driven tasks or an RTOS
(FreeRTOS task API via FFI).

### 4. Thumb-16 / Cortex-M0 not supported

Only Thumb-2 (ARMv7-M and later) is exercised. Cortex-M0/M0+ are Thumb-16
only; the `arm-none-eabi` triple will technically target them but the
generated code may include Thumb-2 instructions unsupported on M0.

### 5. No ARM benchmark results

All numbers in `benchmarks/results/RESULTS.md` were collected on x86-64
(Windows 11 AMD64). There are no AArch64 or Graviton reference runs yet.

### 6. No AArch64 CI runner

The GitHub Actions release workflow runs on `ubuntu-latest` (x86-64) only.
AArch64 cross-test is blocked on adding an `ubuntu-latest` arm64 runner or
self-hosted Graviton/Pi instance.

### 7. SVE / SVE2 — opt-in via `--sve` / `--sve2` ✅ shipped v0.2.4+

```
vanic build server.vani --target=aarch64-unknown-linux-gnu --cpu=neoverse-n2 --sve2
```

Passes `-mattr=+sve` / `-mattr=+sve2` to `llc`. Requires an AArch64 target
(errors with a clear message on any other triple). Pairs with `--cpu=` for
full tuning — e.g. `--cpu=neoverse-n2 --sve2` enables all Graviton 3 /
Neoverse N2 features. LLVM then uses SVE's scalable vector register width
for auto-vectorized loops instead of the fixed 128-bit NEON lanes.

> **Note on explicit NEON / SVE intrinsics:** there is still no user-visible
> `@neon_vaddq_s64` or `target_feature(sve)` surface (gap #2 above).
> All SIMD comes from LLVM auto-vectorization.

---

## Recommended reading

- `tutorials/src/advanced/04b_cross_compile_primer.md` — full `--target=` walkthrough
- `tutorials/src/advanced/04_embedded.md` — bare-metal `--no-std` tutorial
- `examples/language/english/bare_metal.vani` — minimal bare-metal example
- `docs/missing_features.md` — broader language gap inventory
