#!/usr/bin/env bash
# install-cross-qemu.sh -- set up AArch64 and RISC-V 64-bit QEMU
# user-mode emulation + cross-compilers on Debian/Ubuntu, matching
# exactly what vani-compiler's own CI (.github/workflows/ci.yml)
# installs for its test-aarch64-qemu / test-riscv64-qemu jobs.
#
# Installs:
#   qemu-user-static        -- qemu-aarch64-static, qemu-riscv64-static,
#                               and binfmt_misc registration (lets you
#                               run foreign-arch ELF binaries directly,
#                               transparently, no explicit qemu-* prefix
#                               needed for plain `./some-aarch64-binary`)
#   gcc-aarch64-linux-gnu    -- aarch64-linux-gnu-gcc cross-linker
#   gcc-riscv64-linux-gnu    -- riscv64-linux-gnu-gcc cross-linker
#
# Usage:
#   ./install-cross-qemu.sh          # install + verify
#   ./install-cross-qemu.sh --check  # verify only, no install/sudo

set -euo pipefail

CHECK_ONLY=false
[ "${1:-}" = "--check" ] && CHECK_ONLY=true

if ! command -v apt-get >/dev/null 2>&1; then
  echo "error: this script targets Debian/Ubuntu (apt-get not found)." >&2
  exit 1
fi

PACKAGES=(qemu-user-static gcc-aarch64-linux-gnu gcc-riscv64-linux-gnu)

if ! $CHECK_ONLY; then
  echo "==> apt-get update"
  sudo apt-get update -q
  echo "==> installing: ${PACKAGES[*]}"
  sudo apt-get install -q -y "${PACKAGES[@]}"
fi

echo
echo "==> verifying"
fail=0

check_bin() {
  local name="$1"
  if command -v "$name" >/dev/null 2>&1; then
    echo "  ok    $name -> $(command -v "$name")"
  else
    echo "  MISSING $name"
    fail=1
  fi
}

check_bin qemu-aarch64-static
check_bin qemu-riscv64-static
check_bin aarch64-linux-gnu-gcc
check_bin riscv64-linux-gnu-gcc

echo
echo "==> binfmt_misc registration (lets you run foreign-arch ELF"
echo "    binaries directly, e.g. ./some-aarch64-binary, no qemu-*"
echo "    prefix needed)"
# Check the kernel's actual binfmt_misc state directly rather than the
# legacy `update-binfmts` tool -- modern Debian/Ubuntu register these
# via systemd-binfmt + /usr/lib/binfmt.d/*.conf, which update-binfmts
# doesn't know about and will misreport as "not registered".
for arch in qemu-aarch64 qemu-riscv64; do
  f="/proc/sys/fs/binfmt_misc/$arch"
  if [ -r "$f" ] && grep -q "^enabled" "$f"; then
    echo "  ok    $arch: enabled ($(grep '^interpreter' "$f" | awk '{print $2}'))"
  else
    echo "  MISSING $arch: not registered in the kernel"
    fail=1
  fi
done

if [ "$fail" -ne 0 ]; then
  echo
  echo "One or more binaries are missing. Re-run without --check to install." >&2
  exit 1
fi

cat <<'EOF'

==> All set. Reference for using these with vani-compiler:

  # Run a lib test suite cross-target (matches CI's test-aarch64-qemu /
  # test-riscv64-qemu jobs):
  CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
  CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUNNER="qemu-aarch64-static -L /usr/aarch64-linux-gnu" \
    cargo test --lib --target aarch64-unknown-linux-gnu

  CARGO_TARGET_RISCV64GC_UNKNOWN_LINUX_GNU_LINKER=riscv64-linux-gnu-gcc \
  CARGO_TARGET_RISCV64GC_UNKNOWN_LINUX_GNU_RUNNER="qemu-riscv64-static -L /usr/riscv64-linux-gnu" \
    cargo test --lib --target riscv64gc-unknown-linux-gnu

  # Cross-compile + run a .vani program under QEMU directly via vanic:
  vanic run prog.vani --target=aarch64-unknown-linux-gnu
  vanic run prog.vani --target=riscv64-unknown-linux-gnu

  # Same, with RVV (Vector extension) turned on -- both sides must agree:
  QEMU_RISCV64="qemu-riscv64-static -cpu rv64,v=true,vlen=256" \
    vanic run simd.vani --target=riscv64-unknown-linux-gnu --cpu=sifive-x280

  # Same, with AArch64 SVE/SVE2 turned on:
  QEMU_AARCH64="qemu-aarch64-static -cpu max" \
    vanic run simd.vani --target=aarch64-unknown-linux-gnu --sve2

See tutorials/src/advanced/04b_cross_compile_primer.md and
docs/qemu_testing.md in the vani-compiler repo for the full reference.
EOF
