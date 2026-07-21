# ARM / AArch64 / NEON status in vāṇī (v0.2.4+)

> Written 2026-07-06. Updated 2026-07-21 (vec512 / SVE-512 section added).

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

### 3. `vec256<T>` + `simd256_*` on AArch64 ✅ shipped (SIMD-9, 2026-07-10)

`vec256<T>` is a 256-bit SIMD type with 7 builtins (`simd256_splat`,
`simd256_load`, `simd256_store`, `simd256_add`, `simd256_sub`,
`simd256_mul`, `simd256_reduce_add`). The LLVM IR type is `<N x T>` where N
is twice the vec128 lane count.

**AArch64 without SVE:** LLVM legalises `<8 x float>` as two 128-bit NEON
registers. Each `simd256_*` call lowers to two NEON instructions. Functional,
but no throughput gain vs two `vec128` calls unless the OOO scheduler can
overlap them.

**AArch64 + SVE (`--sve` / `--sve2`):** With a scalable-vector CPU
(`--cpu=neoverse-n2`, `--cpu=a64fx`), LLVM can use a single SVE register of
the appropriate width. A Neoverse N2 has 256-bit SVE; `vec256<f32>` fits in
one SVE `z`-register and the loop body becomes a single `fmla z0.s, z1.s, z2.s`.

| Target | vec256<f32> LLVM output |
|--------|------------------------|
| AArch64, no SVE | 2× `fmla v0.4s, v1.4s, v2.4s` |
| AArch64 + SVE, 256-bit | 1× `fmla z0.s, z1.s, z2.s` |
| AArch64 + SVE, 512-bit | 1× `fmla z0.s, ...` (first half only — VLEN matters) |

To check correctness under QEMU with SVE:

```bash
QEMU_CPU=max qemu-aarch64-static ./program
```

`-cpu max` exposes the widest possible SVE configuration that QEMU emulates.

---

### 4. `vec512<T>` on AArch64 ✅ shipped (M4, v0.5.0, 2026-07-15)

`vec512<T>` is a 512-bit SIMD type with the same 7 builtins as `vec256<T>`
(`simd512_splat`, `simd512_load`, `simd512_store`, `simd512_add`,
`simd512_sub`, `simd512_mul`, `simd512_reduce_add`). The LLVM lowering is
architecture-generic (`<N x T>` where N = 512/bits(T), align 64) — the same
`vec128`/`vec256` pattern extended one step further, so no AArch64-specific
codegen was needed.

**AArch64 without SVE:** LLVM legalises `<16 x float>` as four 128-bit NEON
registers. Functional, no throughput gain vs four `vec128` calls.

**AArch64 + SVE-512 (`--sve` / `--sve2`, VLEN=512 hardware or `-cpu max`
under QEMU):** a single scalable `z`-register holds the full 512 bits —
one `fadd z0.s, z1.s, z2.s` per op, same story as `vec256` + SVE-256 above.

| Target | vec512<f32> LLVM output |
|--------|------------------------|
| AArch64, no SVE | 4× `fadd v0.4s, v1.4s, v2.4s` |
| AArch64 + SVE, 512-bit | 1× `fadd z0.s, z1.s, z2.s` |

See `tutorials/src/advanced/05_simd.md` (Layer 5) for the full builtin
reference and worked dot-product example, and `docs/simd_ffi_shims.md` for
the exotic-intrinsics escape hatch (AVX-512 masking, SVE gather-scatter,
etc. still require an FFI shim -- `vec512<T>` only covers
splat/load/store/add/sub/mul/reduce_add).

---

### 5. `parallel for` unavailable on bare-metal ARM

`parallel for … reduce` emits `CreateThread` (Windows) or `pthread_create`
(POSIX). Neither exists on bare-metal (`--no-std` + `arm-none-eabi`).
Workaround: manual work-splitting via interrupt-driven tasks or an RTOS
(FreeRTOS task API via FFI).

### 6. Thumb-16 / Cortex-M0 not supported

Only Thumb-2 (ARMv7-M and later) is exercised. Cortex-M0/M0+ are Thumb-16
only; the `arm-none-eabi` triple will technically target them but the
generated code may include Thumb-2 instructions unsupported on M0.

### 7. No ARM benchmark results

All numbers in `benchmarks/results/RESULTS.md` were collected on x86-64
(Windows 11 AMD64). There are no AArch64 or Graviton reference runs yet.

### 8. AArch64 CI via QEMU ✅ shipped (SIMD-7, 2026-07-10)

`.github/workflows/ci.yml` includes `test-aarch64-qemu`, which runs
`cargo test --lib --target aarch64-unknown-linux-gnu` under
`qemu-aarch64-static` on every push to `main`. vanic has no native LLVM
library dependency (it shells out to `lli`/`llc`), so the binary is pure
Rust and cross-compiles with no extra steps.

The `tests/edge_cases.rs` integration tests are excluded from the AArch64
run — those spawn the vanic binary which forks `cc`/`lli` (x86-64 host
binaries). Full integration tests on real AArch64 hardware are tracked as
ARM-3.

### 9. SVE / SVE2 — opt-in via `--sve` / `--sve2` ✅ shipped v0.2.4+

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

## QEMU testing for AArch64

`vanic run hello.vani --target=aarch64-unknown-linux-gnu` invokes
`qemu-aarch64-static` automatically if it is on `$PATH` (or `$QEMU_AARCH64`).

This lets an x86-64 development machine run the full compiler + program
pipeline for AArch64 **functionally** — exit codes, stdout, memory safety,
ICE/panic detection. It does **not** give meaningful performance numbers.

The CI job **ARM-6** (shipped v0.2.4) runs `cargo test --lib` under
`qemu-aarch64-static` via `CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUNNER`.

To enable SVE in QEMU:

```bash
QEMU_AARCH64="qemu-aarch64-static -cpu max" \
  vanic run server.vani --target=aarch64-unknown-linux-gnu --sve2
```

Full QEMU setup, CPU flags, and limitations are in **`docs/qemu_testing.md`**.

---

## RISC-V status

### Cross-compilation triples

| Triple | ISA | Use case |
|--------|-----|----------|
| `riscv32-unknown-none-elf` | RV32IMAC | bare-metal ELF |
| `riscv64-unknown-linux-gnu` | RV64GC | Linux userspace |

```bash
# Bare-metal (needs riscv32-elf-gcc or riscv32-unknown-elf-gcc on PATH)
CROSS_CC=riscv32-elf-gcc vanic build blink.vani \
  --target=riscv32-unknown-none-elf -o blink.elf

# Linux 64-bit (cross-linker: riscv64-linux-gnu-gcc)
vanic run hello.vani --target=riscv64-unknown-linux-gnu
# → automatically invokes qemu-riscv64-static if on PATH
```

### RISC-V Vector extension (RVV)

| Feature | Status |
|---------|--------|
| LLVM auto-vectorization → RVV | ✓ works when `--cpu=<v-capable>` e.g. `sifive-x280` |
| `vec128<T>` builtins → RVV | ✓ LLVM lowers 128-bit vector IR to `vsetvli` + `vadd.vv` etc. |
| `vec256<T>` / `vec512<T>` builtins → RVV | ✓ same lowering, legalised into 1-4 vector-register groups depending on hardware VLEN (see table below) |
| Explicit RVV intrinsics | Via FFI shim with `<riscv_vector.h>` (see `docs/simd_ffi_shims.md`) |
| RVV CI | ✗ not yet (RISC-V QEMU CI is a documented gap in ARM-6 follow-up) |
| RVV benchmarks | ✗ real hardware needed |

To use auto-vectorization with RVV, pass a CPU that includes the V extension:

```bash
vanic build dot.vani \
    --target=riscv64-unknown-linux-gnu \
    --cpu=sifive-x280 \
    -o dot

# Run under QEMU with V extension enabled
QEMU_RISCV64="qemu-riscv64-static -cpu rv64,v=true,vlen=256" \
  qemu-riscv64-static -cpu rv64,v=true,vlen=256 ./dot
```

For a complete RVV FFI shim example (explicit `vsetvli`/`vadd.vv` via
`<riscv_vector.h>`) and detailed QEMU CPU flags, see **`docs/qemu_testing.md`**.

`vec256<T>`/`vec512<T>` legalise the same way `vec128<T>` does, just across
more vector-register groups on narrower hardware:

| Hardware VLEN | `vec512<f32>` lowering |
|---|---|
| 512 | 1 group (`vl=16` e32) — optimal |
| 256 | 2 groups |
| 128 | 4 groups — correct, no throughput gain over four `vec128` calls |

### Known gaps

- No RISC-V QEMU CI job (equivalent of ARM-6).
- No RVV-specific lib tests (AArch64 NEON path tested via `cargo test --lib`
  under `qemu-aarch64-static`; RISC-V lacks the equivalent).
- No RISC-V benchmark numbers in `benchmarks/results/RESULTS.md`.

---

## Recommended reading

- `docs/qemu_testing.md` — QEMU setup, CPU flags, what QEMU can/cannot test
- `tutorials/src/advanced/04b_cross_compile_primer.md` — full `--target=` walkthrough
- `tutorials/src/advanced/04_embedded.md` — bare-metal `--no-std` tutorial
- `tutorials/src/advanced/05_simd.md` — three-layer SIMD guide (auto / `#[vectorize]` / `vec128<T>`)
- `examples/language/english/bare_metal.vani` — minimal bare-metal example
- `docs/simd_ffi_shims.md` — NEON / RVV / AVX2 FFI shim cookbook
- `docs/missing_features.md` — broader language gap inventory
