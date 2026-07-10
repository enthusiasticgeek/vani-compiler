# Advanced 4b -- Cross-compilation and bare-metal targets (primer)

> **Learning goal**: understand what cross-compilation means,
> why bare-metal needs special linker attributes and no-stdlib,
> and how vāṇī's `--target`, `--no-std`, `#[no_mangle]`,
> `#[link_section]`, and narrow MMIO builtins fit together.
> Reading order: [Advanced 4a -- Embedded primer](04a_embedded_primer.md)
> -> here -> [Advanced 4 -- Embedded](04_embedded.md).

This chapter has **no compiler code**. Pure concepts, then the
full attribute / flag reference.

---

## What is cross-compilation?

On a typical development laptop (x86-64 Linux or Windows), you
run `vanic build` and get a binary that runs on **the same
machine** -- the host. The compiler, the object code, and the
runtime all share the same CPU instruction set and OS ABI.

**Cross-compilation** produces a binary for a **different**
target -- an ARM Cortex-M microcontroller, a RISC-V board, an
AArch64 server -- that the build machine cannot run directly.

```
Build machine: x86-64 Linux
  -> run vanic build --target=arm-none-eabi
  -> produces:  firmware.elf  (ARM Thumb-2 instructions)
  -> flash to:  STM32 Nucleo board  (ARM Cortex-M4)
```

The LLVM backend already knows how to emit ARM / RISC-V /
AArch64 instructions -- it was designed to be target-independent.
`--target=<triple>` just tells `llc` which instruction set to
use instead of the host default.

---

## The LLVM target triple

A **target triple** is a string in the form:

```
<arch>-<vendor>-<os>[-<abi>]
```

| Triple | Architecture | OS / system |
|--------|-------------|-------------|
| `arm-none-eabi` | ARM Thumb-2 | bare metal, EABI calling convention |
| `thumbv7em-none-eabihf` | ARMv7E-M + hardware FPU | bare metal |
| `riscv32-unknown-none-elf` | RISC-V 32-bit | bare metal ELF |
| `riscv64-unknown-linux-gnu` | RISC-V 64-bit | Linux userspace |
| `aarch64-unknown-linux-gnu` | AArch64 | Linux userspace |
| `x86_64-unknown-linux-musl` | x86-64 | Linux with musl libc |

Triples containing `none`, `eabi`, or ending in `-elf` are
**bare-metal** -- vāṇी automatically treats them differently from
Linux cross-targets.

---

## What changes for bare-metal?

A bare-metal target has:

1. **No operating system** -- no syscalls, no `printf`, no file
   descriptors, no dynamic linker.
2. **No C standard library** -- `malloc`, `free`, `fopen` don't
   exist unless you provide them yourself.
3. **A linker script** -- instead of the OS loader, a
   hand-written `.ld` file maps code and data to physical
   addresses on the chip.
4. **An explicit entry point** -- instead of the OS finding
   `main`, the linker script sets the reset vector to a function
   you name (`Reset_Handler`, `_start`, etc.).

vāṇī has three features that address these directly:

---

## `--no-std` -- strip the C prelude

The C backend normally starts every generated file with:

```c
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
...
```

A bare-metal cross-compiler (`arm-none-eabi-gcc`) has no libc.
Those includes fail.

`--no-std` replaces the include block with a **minimal typedef
block** that defines only the primitive types (`uint8_t`,
`int64_t`, `size_t`, `uintptr_t`) plus forward declarations for
`malloc` / `free` / `abort`. The chip's linker script or startup
code provides those symbols if you use them.

For `--target` triples that are bare-metal, `--no-std` is
**activated automatically**.

```bash
# Explicit
vanic emit firmware.vani --backend=c --no-std -o firmware.c

# Automatic (bare-metal triple implies no-std)
vanic build firmware.vani --target=arm-none-eabi -o firmware.elf
```

---

## `#[no_mangle]` -- use the exact function name

vāṇī mangles every generated function name to avoid collisions:

```
fn main()          -> intent_main
fn Reset_Handler() -> intent_Reset_Handler
```

A bare-metal linker script expects the **exact** name at the
reset vector -- `Reset_Handler`, `_start`, `HardFault_Handler`.
If the symbol is renamed, the linker can't find it.

```vani
#[no_mangle]
fn Reset_Handler() -> i64 {
  // ...
}
```

`#[no_mangle]` suppresses the prefix and any Unicode encoding,
emitting the bare `Reset_Handler` symbol in both the C output
and the LLVM IR. The linker script can now reference it directly.

---

## `#[link_section = "..."]` -- place code / data at a specific address

A typical Cortex-M linker script maps:

| Section | Address | Contents |
|---------|---------|---------|
| `.text` | 0x08000000 (Flash) | Code |
| `.rodata` | 0x08020000 (Flash) | Read-only data |
| `.data` | 0x20000000 (RAM) | Initialized globals |
| `.vectors` | 0x08000000 (Flash start) | Interrupt vector table |

By default all vāṇी functions land in `.text`. To place a
function in a specific section:

```vani
#[no_mangle]
#[link_section = ".text.Reset_Handler"]
fn Reset_Handler() -> i64 { ... }

#[no_mangle]
#[link_section = ".text.isr"]
fn SysTick_Handler() -> i64 { ... }
```

In C output this emits:

```c
__attribute__((section(".text.Reset_Handler")))
int64_t Reset_Handler(void) { ... }
```

In LLVM IR the `define` line gets `section ".text.Reset_Handler"`.

The linker script then controls placement via section names:

```ld
SECTIONS {
  .text.Reset_Handler 0x08000000 : { *(.text.Reset_Handler) }
  .text               0x08000004 : { *(.text*) }
}
```

---

## Narrow MMIO builtins (u8 / u16)

Peripheral registers on a microcontroller are often 8-bit or
16-bit -- GPIO status bytes, UART data registers, ADC result
halves. Since v0.1.6 all four width variants ship:

| Builtin | Width | Direction |
|---------|-------|-----------|
| `mmio_read_u8(addr: i64) -> u8` | 8-bit | read |
| `mmio_read_u16(addr: i64) -> u16` | 16-bit | read |
| `mmio_read_u32(addr: i64) -> u32` | 32-bit | read |
| `mmio_write_u8(addr: i64, val: u8) -> i64` | 8-bit | write |
| `mmio_write_u16(addr: i64, val: u16) -> i64` | 16-bit | write |
| `mmio_write_u32(addr: i64, val: u32) -> i64` | 32-bit | write |

All six lower to `*(volatile uint_N_t*)` casts in C and to a
volatile `i_N` load/store with zero-extension or truncation in
LLVM IR. The `volatile` keyword tells both C and LLVM compilers
not to optimize away the access or reorder it.

---

## The cross-linker selection

`vanic build --target=<triple>` needs a cross-linker that
understands the target ABI. Selection priority:

1. `$CROSS_CC` environment variable (always wins).
2. `<triple>-gcc` with `unknown-` stripped -- e.g.
   `arm-none-eabi-gcc`, `riscv32-none-elf-gcc`,
   `aarch64-linux-gnu-gcc`.
3. If neither is found, the build fails with a clear error.

For most toolchains installed from a package manager (`apt`,
`brew`, `winget`), the derived name is correct automatically.

---

## QEMU user-mode run for Linux cross-targets

For Linux cross-targets (not bare-metal), `vanic run --target`
can run the cross-compiled ELF via QEMU user-mode emulation:

```bash
# Install qemu-user-static (Debian/Ubuntu)
sudo apt install qemu-user-static

# Build + run in one command
vanic run hello.vani --target=aarch64-unknown-linux-gnu
```

QEMU binary lookup priority:
1. `$QEMU_<ARCH>` env var (e.g. `$QEMU_AARCH64`)
2. `qemu-aarch64-static` on PATH
3. `qemu-aarch64` on PATH
4. If none found: ELF is written to a temp file, a hint is
   printed, exit 1.

Bare-metal triples cannot run this way -- `vanic run` rejects
them with a clear error and suggests `vanic build` + physical
flashing.

### Enabling SIMD extensions in QEMU

By default, QEMU emulates the baseline ISA. Explicit SIMD
features -- SVE on AArch64, the Vector extension (RVV) on
RISC-V -- must be enabled via CPU flags.

**AArch64: NEON (always-on) and SVE:**

```bash
# NEON is always present on any AArch64 QEMU target -- no extra flag needed.
vanic run simd.vani --target=aarch64-unknown-linux-gnu

# SVE / SVE2: use -cpu max to enable all optional AArch64 extensions.
QEMU_AARCH64="qemu-aarch64-static -cpu max" \
  vanic run server.vani --target=aarch64-unknown-linux-gnu --sve2
```

The `vec128<T>` builtins (`simd_splat`, `simd_add`, `simd_mul`,
`simd_reduce_add`, `simd_load`, `simd_store`) always lower to NEON
instructions on AArch64. No extra CPU flag is needed to test them
under QEMU.

**RISC-V 64-bit: RVV (Vector extension):**

```bash
# Pass -cpu rv64,v=true,vlen=256 to expose the V extension.
# vlen is the physical vector register width in bits (128, 256, 512 …).
QEMU_RISCV64="qemu-riscv64-static -cpu rv64,v=true,vlen=256" \
  vanic run loop.vani --target=riscv64-unknown-linux-gnu --cpu=sifive-x280
```

`--cpu=sifive-x280` instructs LLVM to emit RVV instructions;
the QEMU `-cpu rv64,v=true` flag tells QEMU to execute them.
Both sides must agree -- omitting either causes illegal-instruction
faults at runtime.

### What QEMU validates vs. what it cannot

| Scenario | QEMU status |
|----------|-------------|
| Exit code / stdout correctness | ✓ fully testable |
| `vec128<T>` NEON / RVV instruction selection | ✓ functional |
| SVE register width variants (`-cpu max` vs `neoverse-n2,sve256=on`) | ✓ functional |
| Compiler ICE / panic detection | ✓ |
| Benchmark / timing numbers | ✗ meaningless -- QEMU speed reflects host JIT, not target ISA |
| MMIO peripheral behavior | ✗ needs `qemu-system-*` + board model |
| Interrupt latency | ✗ not cycle-accurate |

> For the full QEMU reference -- CPU flags, CI setup, RVV FFI shim example,
> and the `vanic run` discovery algorithm -- see **`docs/qemu_testing.md`**.

### Running the edge-case suite under QEMU (AArch64)

The CI job **ARM-6** runs `cargo test --lib` under QEMU to catch
AArch64-specific bugs:

```bash
sudo apt install qemu-user-static gcc-aarch64-linux-gnu
CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUNNER=qemu-aarch64-static \
  cargo test --lib --target aarch64-unknown-linux-gnu
```

Running the full integration tests (`cargo test --test edge_cases`)
under QEMU is not yet automated (tracked as TODO SIMD-7 in
`docs/TODO_CURRENT.md`).

---

## Summary you can carry

| Need | Feature |
|------|---------|
| Produce ARM / RISC-V object code | `vanic build --target=arm-none-eabi` |
| Remove libc headers from C output | `--no-std` (auto for bare-metal `--target`) |
| Exact symbol name for linker script | `#[no_mangle]` |
| Place function in specific ELF section | `#[link_section = ".text.foo"]` |
| 8-bit MMIO register | `mmio_read_u8(addr)` / `mmio_write_u8(addr, val)` |
| 16-bit MMIO register | `mmio_read_u16(addr)` / `mmio_write_u16(addr, val)` |
| Override cross-linker | `CROSS_CC=arm-none-eabi-gcc vanic build ...` |
| Run cross-Linux ELF on host | `vanic run --target=aarch64-unknown-linux-gnu` (needs QEMU) |
| Tune for a specific CPU | `--cpu=cortex-a72` (Pi 4) / `--cpu=neoverse-n2` (Graviton 3) |
| Enable SVE on Neoverse N2 / Graviton 3 | `--sve` (SVE) or `--sve2` (SVE2); AArch64 only |
| Enable SVE2 on Apple M-series / Graviton 4 | `--target=aarch64-… --cpu=apple-m4 --sve2` |

> **NEON auto-vectorization and `vectorize.width`:** when building for AArch64 the
> compiler automatically emits `!llvm.loop.vectorize.width = 2` (instead of the
> x86-biased `4`) so LLVM's vectorizer picks the natural NEON lane count for i64
> loops. With `--sve` / `--sve2` the scalable-vector lowering in `llc` overrides
> this hint and chooses the hardware's native SVE register width.

## Cross-reference

- [Advanced 4a -- Embedded primer](04a_embedded_primer.md) -- the broader embedded picture (`no_heap`, `bounded_stack`, regions)
- [Advanced 4 -- Embedded targets + `unsafe`](04_embedded.md) -- worked examples using these features together
- [Intermediate 9 -- FFI](../intermediate/09_ffi.md) -- device I/O (UART/SPI/I2C) still needs FFI + C shims
- [`examples/language/english/bare_metal.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/english/bare_metal.vani) -- runnable bare-metal example
