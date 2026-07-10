# ARM / AArch64 / NEON status in vāṇī (v0.2.4+)

> Written 2026-07-06. Updated 2026-07-10.

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

### 1. `vectorize.width` hint is target-aware ✅ fixed v0.2.4+

The compiler now reads the active LLVM target triple and emits
`vectorize.width = 2` for AArch64 targets and `width = 4` for x86-64.
The LLVM target triple is set via `set_target_triple()` before codegen
and consulted by `vectorize_width()` in `backend_llvm.rs`.

### 2. Explicit NEON via `vec128<T>` ✅ shipped v0.2.4+

`vec128<T>` is a 128-bit SIMD register value holding N lanes of type T.
Seven built-in operations lower directly to NEON instructions:

| Builtin | AArch64 (i32 example) |
|---------|----------------------|
| `simd_splat(x)` | `dup v0.4s, w0` |
| `simd_add(a, b)` | `add v0.4s, v1.4s, v2.4s` |
| `simd_mul(a, b)` | `mul v0.4s, v1.4s, v2.4s` |
| `simd_reduce_add(v)` | `addv s0, v1.4s` |
| `simd_load(vec, i)` | `ldr q0, [x0, x1, lsl #2]` |
| `simd_store(vec, i, d)` | `str q0, [x0, x1, lsl #2]` |

For exotic intrinsics not yet in the builtin set, the FFI shim
escape hatch remains available — see `docs/simd_ffi_shims.md`.

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

> **Note on raw NEON / SVE intrinsics:** there is no user-visible
> `@neon_vaddq_s64` or `target_feature(sve)` surface beyond what `vec128<T>` +
> `simd_*` builtins expose. For exotic intrinsics not yet in the builtin set,
> use the FFI shim escape hatch — see `docs/simd_ffi_shims.md`.

---

## Recommended reading

- `tutorials/src/advanced/04b_cross_compile_primer.md` — full `--target=` walkthrough
- `tutorials/src/advanced/04_embedded.md` — bare-metal `--no-std` tutorial
- `tutorials/src/advanced/05_simd.md` — three-layer SIMD guide (auto / `#[vectorize]` / `vec128<T>`)
- `examples/language/english/bare_metal.vani` — minimal bare-metal example
- `docs/simd_ffi_shims.md` — NEON / AVX2 FFI shim cookbook
- `docs/missing_features.md` — broader language gap inventory
