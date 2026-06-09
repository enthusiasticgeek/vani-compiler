# Installing vāṇी

vāṇī's compiler is written in Rust. To build it, you need:

| Tool | Used by | Required? |
|---|---|---|
| **Rust toolchain** (1.75+) | building the `vanic` binary | **required** |
| **z3** | SMT verifier (`requires` / `ensures` / `prove`) | required for full check |
| **clang / gcc / msvc** | C backend (`--backend=c`) + final link step | required for `vanic run --backend=c` and `vanic build` |
| **LLVM tools** (`lli`, `llc`, `opt`) | LLVM backend (`--backend=llvm`, default) | required for default `vanic run` and `vanic build` |

> `vanic check` (typecheck + SMT only) needs just Rust + z3. The
> LLVM tools and a C compiler are only used by `run` / `build`
> code-emission paths.

Set `VANIC_NO_VERIFY=1` (or the legacy `INTENTC_NO_VERIFY=1`) to
skip SMT entirely for fast iteration on non-proof code changes.

---

## Linux

### Debian / Ubuntu / WSL

```bash
sudo apt update
sudo apt install -y build-essential z3 llvm clang
```

`build-essential` brings in `gcc` + `make` + libc headers.

### Fedora / RHEL / Rocky / AlmaLinux

```bash
sudo dnf install -y gcc make z3 llvm clang
```

### Arch / Manjaro

```bash
sudo pacman -S --needed base-devel z3 llvm clang
```

### Alpine

```bash
sudo apk add build-base z3 llvm clang
```

### Rust toolchain (any Linux distro)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustup default stable
```

---

## macOS

### With Homebrew (recommended)

```bash
brew install z3 llvm rustup-init
rustup-init -y
```

After `brew install llvm`, you may need to add the keg-only LLVM
to PATH so `lli` / `llc` / `opt` resolve:

```bash
echo 'export PATH="$(brew --prefix llvm)/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

### With MacPorts

```bash
sudo port install z3 llvm-17 clang-17
sudo port select --set llvm mp-llvm-17
sudo port select --set clang mp-clang-17
```

Then install Rust via `rustup` (the curl one-liner under Linux
above works on macOS too).

> **Apple Silicon (M1/M2/M3)**: both Homebrew and MacPorts paths
> work natively; vāṇी's C and LLVM backends emit ARM64 code
> directly. No Rosetta needed.

### Arc 8 I/O note for macOS

The C backend's `epoll` / `timerfd` family branches to
**kqueue** + **EVFILT_READ** + a pipe+pthread userspace timer
on `__APPLE__`. The same surface in LLVM IR is in
`emit_intent_epoll_helpers_llvm_darwin`. **Verification deferred**
at landing time (no macOS host available); the macOS branch
exercises on first build there — file issues if anything trips.
See [docs/v1_limitations.md L10](docs/v1_limitations.md).

---

## Windows

### Option 1: WSL2 (recommended, easiest)

The smoothest path is WSL2 with a Linux distribution — follow the
Debian/Ubuntu instructions above. WSL2 gives you a real Linux
runtime including `epoll`, `nanosleep`, and `__errno_location()`,
which means you stay on the Linux-verified code path.

```powershell
# In an Administrator PowerShell:
wsl --install
# After reboot:
wsl --set-default-version 2
```

Then in your WSL2 shell, follow the Linux install steps.

### Option 2: Native Windows (no WSL)

Native Windows builds use the `_WIN32` branch of vāṇी's runtime
(IOCP for epoll, winsock2 for TCP, `Sleep` for timers).
**Verification of this path is deferred** at landing time — see
[docs/v1_limitations.md L10](docs/v1_limitations.md). The
following install path should work but expect to find rough
edges:

```powershell
# Install Chocolatey first (https://chocolatey.org/install)
# Then:
choco install -y rustup.install
choco install -y llvm
choco install -y visualstudio2022buildtools --package-parameters "--add Microsoft.VisualStudio.Component.VC.Tools.x86.x64"
choco install -y mingw  # optional, if you prefer gcc over MSVC for the C backend
```

For z3 on Windows, the easiest path is to download a prebuilt
release from <https://github.com/Z3Prover/z3/releases> and add
the extracted `bin\` directory to your `PATH`. Verify with
`z3 --version`.

`rustup-init.exe` from <https://rustup.rs> works equivalently if
you prefer not to use Chocolatey.

> **Windows LLVM TCP IR**: the LLVM backend's Windows TCP helpers
> were added late (commit `cedf20d`); they declare i64 SOCKET +
> winsock2 + WSAStartup once-init. See
> [docs/v1_limitations.md L10](docs/v1_limitations.md) for the
> hot-spots to verify on a Windows host.

---

## Verify your install

```bash
cargo --version
rustc --version
z3 --version
lli --version          # LLVM JIT
llc --version          # LLVM static compiler
opt --version          # LLVM IR optimizer
cc --version           # C compiler (gcc or clang)
```

Then build vāṇी itself:

```bash
git clone https://github.com/ptamb3/vani.git
cd vani
cargo build --release   # builds target/release/vanic + target/release/intentc (legacy alias)
cargo test              # 1894 lib + 54 parity tests; ~90s on a modern laptop
```

A successful build leaves `target/release/vanic` ready to run.
Try a Hello World:

```bash
./target/release/vanic run examples/language/english/basics.vani
```

You should see the output `42`.

---

## Optional: MCP server for AI-assisted code generation

vāṇी ships an [MCP](https://modelcontextprotocol.io/) server at
`tools/llm_context/mcp_server.py` so AI agents (Claude Desktop,
Claude Code, Cursor) can pull language context AND call
`vanic check` / `vanic run` / `vanic emit-c` on their own
generated source. The static-bundle paste workflow
(`tools/llm_context/bundle.py`) needs nothing beyond Python 3;
the MCP server needs one extra package and the `VANI_BIN` env
var pointing at the compiler binary.

```bash
pip install mcp                                           # MCP SDK
export VANI_BIN=$(realpath target/release/vanic)          # for the server to find vanic
```

Then wire `tools/llm_context/mcp_server.py` into your client's
MCP config (see [Advanced 11 — LLM workflows](tutorials/src/advanced/11_llm_workflows.md)
in the tutorials for full Claude Desktop / Cursor config blocks).
None of this is required for using `vanic` directly — skip if
you're not running an MCP-speaking host.

---

## Troubleshooting

### `error: linker 'cc' not found`

Install your platform's C compiler (`build-essential` on Debian,
`gcc` on Fedora, Xcode Command Line Tools on macOS via
`xcode-select --install`).

### `lli: command not found`

LLVM tools aren't on `PATH`. On macOS with Homebrew, the
`llvm` formula is keg-only — re-run the `brew --prefix llvm` PATH
export from the macOS section above.

### z3 missing or wrong version

The Z3 versions vāṇī tests against are 4.8+ (anything modern
will work). If `z3 --version` errors with "command not found",
the binary isn't on `PATH`. On Windows, double-check that the
Z3 release `bin\` directory is in `PATH`.

### Tests fail with stack overflow

The `.cargo/config.toml` in the repo sets
`RUST_MIN_STACK=33554432` (32MB) for `cargo test`. If you're
running test binaries directly (not via `cargo test`), set the
env var manually:

```bash
RUST_MIN_STACK=33554432 ./target/debug/deps/vani-*
```

### Linux + LLVM 17 vs LLVM 18 differences

vāṇी has been tested against LLVM 14–18. If you hit a backend
error with a specific LLVM major version, file an issue with
the `lli --version` output.

---

## Verifying the install across all examples

For a deeper smoke test, run the parity sweep (every example
file across both C and LLVM backends):

```bash
cargo test --test run_end_to_end llvm_backend_run_produces_same_output_as_c
```

This takes ~60 seconds and exercises ~150 examples (the 133
English + 22 GoF design patterns + ~25 Devanagari examples) on
both backends. A clean pass means your install is healthy.
