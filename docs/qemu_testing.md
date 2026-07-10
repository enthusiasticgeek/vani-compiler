# QEMU-based testing for cross-architecture targets

> vāṇी v0.2.4+. Updated 2026-07-10.

Running cross-compiled vāṇी programs on hardware you don't have is the job of
**QEMU user-mode emulation**. This document explains what QEMU can and cannot
validate, which architectures are supported, and how to set it up for ARM NEON
and RISC-V Vector (RVV) testing.

---

## What QEMU user-mode does

QEMU user-mode (`qemu-aarch64-static`, `qemu-riscv64-static`, etc.) runs a
single Linux ELF binary compiled for a foreign ISA. It traps every syscall,
translates it to the host OS, and interprets or JIT-translates the foreign
instructions. The host kernel handles memory; QEMU handles the CPU.

This means:

- You can run the **full** `vanic run` pipeline on any Linux cross-target from
  an x86-64 development machine.
- Functional correctness — exit codes, stdout, memory safety — is fully testable.
- Performance numbers are **meaningless** (QEMU speed is a function of host
  clock and JIT quality, not target microarchitecture). Do not benchmark on QEMU.

---

## Setup

### Debian / Ubuntu

```bash
sudo apt-get install qemu-user-static
# installs qemu-aarch64-static, qemu-riscv64-static, etc.
```

### Fedora / RHEL

```bash
sudo dnf install qemu-user-static
```

### macOS (cross-testing only, QEMU static builds)

```bash
brew install qemu
# provides qemu-aarch64 (dynamic); set QEMU_AARCH64=qemu-aarch64
```

### Verify

```bash
qemu-aarch64-static --version    # should print QEMU emulator version ...
qemu-riscv64-static --version
```

---

## How `vanic run` uses QEMU

For a **Linux cross-target** (`aarch64-unknown-linux-gnu`, `riscv64-unknown-linux-gnu`, etc.),
`vanic run` compiles the program to an ELF, then transparently invokes the
appropriate QEMU static binary:

```
vanic run hello.vani --target=aarch64-unknown-linux-gnu
# → compiles hello.vani to hello.elf (AArch64)
# → runs: qemu-aarch64-static hello.elf
```

QEMU binary discovery order (first found wins):

| Environment variable | Example | Fallback |
|---------------------|---------|---------|
| `$QEMU_AARCH64` | `/usr/bin/qemu-aarch64` | `qemu-aarch64-static` on `$PATH` |
| `$QEMU_RISCV64` | `/usr/bin/qemu-riscv64` | `qemu-riscv64-static` on `$PATH` |

For **bare-metal triples** (`arm-none-eabi`, `riscv32-unknown-none-elf`, etc.),
`vanic run` prints a diagnostic and exits — bare-metal ELFs need either real
hardware or `qemu-system-*` (which is a separate, not-yet-integrated path;
see [§ Bare-metal / system-mode](#bare-metal--system-mode-not-integrated) below).

---

## Supported QEMU targets and their SIMD surface

### AArch64 — ARM NEON / SVE

| Feature | How it reaches the target | QEMU testable? |
|---------|--------------------------|---------------|
| LLVM auto-vectorization → NEON | `parallel for` bodies, `vec_fill` | ✓ functional |
| `vec128<T>` explicit SIMD | 7 builtins lower to `dup`, `add.4s`, `addv`, `ldr q0`, etc. | ✓ functional |
| SVE / SVE2 scalable vectors | `--sve` / `--sve2` pass `-mattr=+sve2` to `llc` | ✓ QEMU supports SVE via `-cpu max` |
| FFI NEON shim | `extern "C"` + `--link-with=neon_shim.o` | ✓ if shim compiled for AArch64 |
| Performance / timing | wall-clock benchmarks | ✗ QEMU not representative |

QEMU CPU flag to enable SVE in user-mode:

```bash
QEMU_CPU=max qemu-aarch64-static ./program
# "max" enables all optional AArch64 extensions including SVE2
```

Or set via environment before `vanic run`:

```bash
QEMU_AARCH64="qemu-aarch64-static -cpu max" \
  vanic run server.vani --target=aarch64-unknown-linux-gnu --sve2
```

### RISC-V 64-bit — RVV (RISC-V Vector extension)

| Feature | How it reaches the target | QEMU testable? |
|---------|--------------------------|---------------|
| LLVM auto-vectorization → RVV | `parallel for` bodies when `--cpu=` has V extension | ✓ functional (QEMU ≥ 7.2) |
| `vec128<T>` lowering → LLVM vector IR | LLVM maps 128-bit vector ops to RVV `vsetvli`+`vadd.vv` etc. when `--cpu` includes V | ✓ functional |
| Explicit RVV intrinsics | No native builtins — use FFI shim with `<riscv_vector.h>` | ✓ functional (shim must target rv64gcv) |
| Performance / timing | wall-clock benchmarks | ✗ QEMU not representative |

Enable the RISC-V Vector extension in QEMU:

```bash
# User-mode: -cpu rv64,v=true,vlen=256
QEMU_RISCV64="qemu-riscv64-static -cpu rv64,v=true,vlen=256" \
  vanic run loop.vani --target=riscv64-unknown-linux-gnu --cpu=sifive-x280
```

`vlen=256` tells QEMU to expose 256-bit vector registers (VLEN). Common
values: `128` (minimal RVV), `256` (SiFive X280), `512` (future server cores).

RVV FFI shim example:

```c
// rvv_sum.c — compiled with: riscv64-linux-gnu-gcc -O2 -march=rv64gcv -c rvv_sum.c
#include <riscv_vector.h>
#include <stdint.h>

int64_t rvv_sum(const int64_t *data, int64_t n) {
    vint64m4_t acc = vmv_v_x_i64m4(0, vsetvlmax_e64m4());
    int64_t i = 0;
    while (i < n) {
        size_t vl = vsetvl_e64m4(n - i);
        vint64m4_t v = vle64_v_i64m4(data + i, vl);
        acc = vadd_vv_i64m4(acc, v, vl);
        i += vl;
    }
    return vmv_x_s_i64m4_i64(vredsum_vs_i64m4_i64m4(acc, acc,
                              vmv_v_x_i64m4(0, 1), vsetvlmax_e64m4()));
}
```

```vani
extern "C" fn rvv_sum(data: ref Vec<i64>, n: i64) -> i64;

fn main() -> i64 {
    let xs: Vec<i64> = vec_fill(1024, 1 as i64);
    return rvv_sum(ref xs, len(xs) as i64);
}
```

```bash
riscv64-linux-gnu-gcc -O2 -march=rv64gcv -c rvv_sum.c -o rvv_sum.o

vanic build sum.vani \
    --target=riscv64-unknown-linux-gnu \
    --cpu=sifive-x280 \
    --link-with=rvv_sum.o \
    -o sum

QEMU_RISCV64="qemu-riscv64-static -cpu rv64,v=true,vlen=256" \
  qemu-riscv64-static -cpu rv64,v=true,vlen=256 ./sum
```

---

## What is testable via QEMU

| Test type | C backend | LLVM backend |
|-----------|-----------|-------------|
| Compiler unit tests (`cargo test --lib`) | host only | ✓ runs on QEMU |
| `examples/edge_cases/` integration (`cargo test --test edge_cases`) | host only | not yet cross-run |
| Exit code / stdout correctness | ✓ via `vanic run --target=` | ✓ |
| Compiler ICE / panic detection | ✓ | ✓ |
| Memory safety (no out-of-bounds, no UAF) | ✓ | ✓ |
| NEON / RVV instruction selection | ✓ (binary contains them) | ✓ |
| Actual NEON / RVV performance | ✗ | ✗ |

### Specifically NOT testable via QEMU

- **Benchmark numbers** — all timings in `benchmarks/results/RESULTS.md` were
  collected on real x86-64 hardware. QEMU AArch64 or RISC-V timing is a
  function of the host CPU's single-core speed and QEMU's JIT cache, not the
  target ISA. Never publish QEMU benchmark numbers as target hardware numbers.
- **MMIO / peripheral behavior** — memory-mapped I/O for bare-metal targets
  requires either real hardware or a device-aware `qemu-system-*` board model.
- **Interrupt latency** — IRQ timing on QEMU is not cycle-accurate.
- **Cache behavior** — QEMU does not model cache hierarchy; cache-sensitive
  algorithms (prefetch, non-temporal stores) run the same fast or slow
  regardless of the host or target cache topology.
- **SVE register width on production silicon** — QEMU `-cpu max` exposes the
  maximum possible SVE width; production Graviton 3 uses 256-bit SVE. Test
  with the specific width (`-cpu neoverse-n2,sve256=on`) if precision matters
  for functional tests.

---

## CI integration

### ARM-6: AArch64 QEMU CI (shipped v0.2.4)

`.github/workflows/ci.yml` includes a job that runs the full lib unit test
suite under `qemu-aarch64-static`:

```yaml
- name: Run lib tests under QEMU (AArch64)
  run: |
    sudo apt-get install -y qemu-user-static gcc-aarch64-linux-gnu
    cargo test --lib --target aarch64-unknown-linux-gnu
  env:
    CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUNNER: qemu-aarch64-static
```

This catches:
- AArch64-specific compiler panics (e.g., stack depth limit differences)
- SIMD IR that is valid x86-64 LLVM but rejected by the AArch64 backend
- Endianness bugs (AArch64 is LE like x86-64, so this is low-risk here)

### RISC-V QEMU CI: not yet added

RISC-V CI (equivalent of ARM-6) is a documented gap. Adding it requires:

```yaml
- name: Run lib tests under QEMU (RISC-V 64)
  run: |
    sudo apt-get install -y qemu-user-static gcc-riscv64-linux-gnu
    cargo test --lib --target riscv64gc-unknown-linux-gnu
  env:
    CARGO_TARGET_RISCV64GC_UNKNOWN_LINUX_GNU_RUNNER: qemu-riscv64-static
```

Until then, RISC-V correctness is inferred from the LLVM backend's target
independence — no RISC-V specific bugs are known, but no systematic test run
has been done either.

---

## Bare-metal / system-mode (not integrated)

`qemu-system-arm` and `qemu-system-riscv32` can boot bare-metal ELFs on a
simulated board (e.g., `-machine lm3s6965evb` for Cortex-M3, `-machine
sifive_u` for SiFive U54). This is **not** wired into `vanic run` — bare-metal
triples produce only the ELF; you must invoke `qemu-system-*` manually.

Minimal Cortex-M QEMU run:

```bash
vanic build firmware.vani --target=arm-none-eabi -o firmware.elf

qemu-system-arm \
  -machine lm3s6965evb \
  -nographic \
  -semihosting \
  -kernel firmware.elf
```

Minimal RISC-V 32-bit QEMU run:

```bash
CROSS_CC=riscv32-elf-gcc vanic build blink.vani \
  --target=riscv32-unknown-none-elf -o blink.elf

qemu-system-riscv32 \
  -machine sifive_e \
  -nographic \
  -bios none \
  -kernel blink.elf
```

Integrating `qemu-system-*` into `vanic run` is a future work item (no ticket
yet). The gap is that different boards need different `-machine` flags; there
is no universal "run any ELF" mode for system QEMU the way user-mode has.

---

## Recommended reading

- `docs/arm_neon_status.md` — ARM/AArch64/NEON feature status
- `docs/simd_ffi_shims.md` — NEON / RVV / AVX2 FFI shim cookbook
- `tutorials/src/advanced/04b_cross_compile_primer.md` — `--target=` walkthrough
- `tutorials/src/advanced/05_simd.md` — three-layer SIMD guide
- `benchmarks/results/RESULTS.md` — benchmark numbers (x86-64 only; see above re: QEMU)
