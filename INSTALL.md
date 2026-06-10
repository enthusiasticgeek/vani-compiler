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

> **Status (2026-06-10)**: Native Windows builds are verified. The
> compiler builds and all 2073 lib tests pass on Windows 11 with the
> GNU toolchain. The `_WIN32` I/O runtime (IOCP, winsock2, `Sleep`)
> compiles cleanly; runtime end-to-end tests for async TCP/epoll are
> the next verification milestone.

### Option 1: WSL2 (recommended for Linux parity)

WSL2 gives you a real Linux runtime including `epoll`, `nanosleep`,
and `__errno_location()`, keeping you on the fully-verified Linux
code path.

```powershell
# In an Administrator PowerShell:
wsl --install
# After reboot:
wsl --set-default-version 2
```

Then in your WSL2 shell follow the Linux install steps above.

### Option 2: Native Windows (verified on Windows 11)

Native builds use the `_WIN32` runtime (IOCP, winsock2, `Sleep`).
All compiler tests pass. Follow these steps exactly — order matters.

#### Step 1 — Rust toolchain (winget)

```powershell
winget install Rustlang.Rustup --accept-package-agreements --accept-source-agreements
```

After installation **open a new PowerShell window** so `rustup` and
`cargo` are on `PATH`, then add the GNU target:

```powershell
rustup toolchain install stable-x86_64-pc-windows-gnu
rustup default stable-x86_64-pc-windows-gnu
```

> **Why GNU, not MSVC?** The MSVC target requires `link.exe` from
> Visual Studio Build Tools. The GNU target uses `gcc` from MSYS2
> (installed below) and has no extra dependency. If you already have
> VS 2017 or later installed, the MSVC default (`stable-x86_64-pc-windows-msvc`)
> works and you can skip this switch.

#### Step 2 — GCC via MSYS2

If MSYS2 is not already installed, download the installer from
<https://www.msys2.org> and run it. Then add the mingw64 `bin`
directory to your system `PATH`:

```
C:\msys64\mingw64\bin
```

Verify: `gcc --version` in a new PowerShell window.

#### Step 3 — LLVM tools (`lli`, `llc`, `opt`)

The `winget install LLVM.LLVM` package ships Clang but **not**
`lli`, `llc`, or `opt`. Install the full LLVM toolset via MSYS2:

```powershell
C:\msys64\usr\bin\pacman.exe -Sy mingw-w64-x86_64-llvm --noconfirm
```

This places `lli.exe`, `llc.exe`, and `opt.exe` under
`C:\msys64\mingw64\bin` which is already on `PATH` from Step 2.

> If pacman mirrors are unreachable, the Clang front-end alone is
> sufficient to build the compiler and run `cargo test`. The LLVM
> backend emission tests (`vanic run --backend=llvm`) additionally
> need `lli`; the C backend (`--backend=c`, the default) needs only
> `gcc` or `clang`.

#### Step 4 — z3 SMT solver

Download the latest Windows 64-bit release from
<https://github.com/Z3Prover/z3/releases>, extract it anywhere
(e.g. `C:\z3`), and add the `bin\` subdirectory to your system `PATH`:

```powershell
# One-liner that downloads z3 4.16.0 and extracts to C:\z3:
Invoke-WebRequest -Uri "https://github.com/Z3Prover/z3/releases/download/z3-4.16.0/z3-4.16.0-x64-win.zip" `
    -OutFile "$env:TEMP\z3.zip"
Expand-Archive -Path "$env:TEMP\z3.zip" -DestinationPath "C:\z3" -Force
```

Then add `C:\z3\z3-4.16.0-x64-win\bin` to your system `PATH`
(System Properties → Environment Variables → Path → New).

Verify: `z3 --version` should print `Z3 version 4.16.0`.

#### Step 5 — Verify all tools

Open a **new** PowerShell window (so updated `PATH` takes effect):

```powershell
rustc --version      # rustc 1.96.0 or later
cargo --version
gcc --version        # from C:\msys64\mingw64\bin
z3 --version         # Z3 version 4.x
lli --version        # LLVM JIT (if MSYS2 LLVM installed)
llc --version        # LLVM static compiler
```

#### Step 6 — Build and test

```powershell
git clone https://github.com/enthusiasticgeek/vani-compiler.git
cd vani-compiler
cargo build --release
cargo test
```

Expected output: `test result: ok. 2073 passed; 0 failed`

> **Note on `cargo test` stack size**: the test suite sets
> `RUST_MIN_STACK=33554432` via `.cargo/config.toml`. If you run
> test binaries directly (not via `cargo test`), set this manually:
> `$env:RUST_MIN_STACK = "33554432"` before running.

---

## Verify your install

**Linux / macOS / WSL2:**
```bash
cargo --version
rustc --version
z3 --version
lli --version          # LLVM JIT
llc --version          # LLVM static compiler
opt --version          # LLVM IR optimizer
cc --version           # C compiler (gcc or clang)
```

**Native Windows (PowerShell):**
```powershell
cargo --version
rustc --version
z3 --version
gcc --version          # from MSYS2 mingw64
lli --version          # from MSYS2 mingw64-llvm (if installed)
llc --version
```

Then build vāṇी itself:

**Linux / macOS / WSL2:**
```bash
git clone https://github.com/enthusiasticgeek/vani-compiler.git
cd vani-compiler
cargo build --release   # builds target/release/vanic + target/release/intentc (legacy alias)
cargo test              # 2073 lib tests; ~90s on a modern laptop
```

**Native Windows (PowerShell):**
```powershell
git clone https://github.com/enthusiasticgeek/vani-compiler.git
cd vani-compiler
cargo build --release
cargo test              # 2073 passed; 0 failed
```

A successful build leaves `target/release/vanic` (Linux/macOS) or
`target\release\vanic.exe` (Windows) ready to run. Try a Hello World:

**Linux / macOS / WSL2:**
```bash
./target/release/vanic run examples/language/english/basics.vani
```

**Windows:**
```powershell
.\target\release\vanic.exe run examples\language\english\basics.vani
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

**Linux**: install `build-essential` (Debian) or `gcc` (Fedora).
**macOS**: run `xcode-select --install`.
**Windows**: you are likely on the MSVC target without VS Build
Tools. Switch to the GNU target:
```powershell
rustup toolchain install stable-x86_64-pc-windows-gnu
rustup default stable-x86_64-pc-windows-gnu
```
Then ensure `C:\msys64\mingw64\bin` is on `PATH`.

### `error: linker 'link.exe' not found` (Windows)

Same as above — the default `stable-x86_64-pc-windows-msvc`
toolchain needs `link.exe` from VS Build Tools. Switch to the
GNU target (see above) or install Visual Studio 2017+ with the
"C++ build tools" workload from <https://visualstudio.microsoft.com/downloads/>.

### `lli: command not found`

**Linux/macOS**: LLVM tools aren't on `PATH`. On macOS with
Homebrew the `llvm` formula is keg-only — re-run the
`brew --prefix llvm` PATH export from the macOS section above.

**Windows**: `winget install LLVM.LLVM` installs Clang but **not**
`lli`/`llc`/`opt`. Install the full LLVM via MSYS2:
```powershell
C:\msys64\usr\bin\pacman.exe -Sy mingw-w64-x86_64-llvm --noconfirm
```
If MSYS2 mirrors are slow, note that `lli`/`llc`/`opt` are only
needed for `vanic run --backend=llvm`. The C backend
(`--backend=c`, the default) works without them.

### z3 missing or wrong version

Z3 4.8 or later works. If `z3 --version` gives "command not found",
the binary isn't on `PATH`.

**Windows**: confirm the extracted z3 `bin\` directory (e.g.
`C:\z3\z3-4.16.0-x64-win\bin`) is in your system `PATH`, not just
the session `PATH`. Open System Properties → Environment Variables
→ Path → New, then restart your terminal.

### Tests fail with stack overflow

The `.cargo/config.toml` sets `RUST_MIN_STACK=33554432` (32 MB)
for `cargo test`. If running test binaries directly, set it first:

```bash
# Linux / macOS / WSL2
RUST_MIN_STACK=33554432 ./target/debug/deps/vani-*
```

```powershell
# Windows PowerShell
$env:RUST_MIN_STACK = "33554432"
.\target\debug\deps\vani-*.exe
```

### LLVM 17 vs 18 vs 22 differences

vāṇī has been tested against LLVM 14–18 on Linux. LLVM 22 (the
current `winget` release) is supported on Windows for C-backend
builds and `cargo test`. If you hit an IR-emit error with a
specific LLVM version, file an issue including the `lli --version`
output.

### Windows: `pacman` mirrors unreachable or signature errors

MSYS2's package database can be stale on older installations.
Update the keyring and retry:
```powershell
C:\msys64\usr\bin\pacman.exe -Sy msys2-keyring --noconfirm
C:\msys64\usr\bin\pacman.exe -Sy mingw-w64-x86_64-llvm --noconfirm
```
If mirrors remain unreachable, use a VPN or try again later —
the mirrors are community-hosted and occasionally go offline.

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
