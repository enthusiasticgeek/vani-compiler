# Installation

## Quick install (pre-built binary)

The fastest path — no Rust toolchain needed.

**Linux / macOS:**

```bash
curl -fsSL https://raw.githubusercontent.com/enthusiasticgeek/vani-compiler/main/install.sh | sh
```

This downloads the correct binary for your platform and installs `vanic` to `/usr/local/bin`.  Pass `--prefix $HOME/.local` if you prefer a non-root location.

**Windows (PowerShell):**

```powershell
irm https://raw.githubusercontent.com/enthusiasticgeek/vani-compiler/main/install.ps1 | iex
```

This downloads the Windows binary and installs it to `%LOCALAPPDATA%\vanic\bin`, adding that directory to your user PATH automatically.

> **After quick-install** you still need z3, gcc/clang, and lli for the full feature set.
> `vanic check` works with just the binary; `vanic run` needs the tools below.

---

## Build from source

Before you can follow any lesson you need three things on your machine:

1. **Rust toolchain** — to build the `vanic` compiler from source
2. **z3** — the SMT solver (`requires` / `ensures` / `prove` contracts use it)
3. **A C compiler + LLVM tools** — for the two code-emission backends

> `vanic check` (type-check + SMT only) needs just Rust + z3.
> `vanic run` (the command every lesson uses) additionally needs either
> `lli` (LLVM backend, the default) or `gcc`/`clang` (C backend, `--backend=c`).

---

## Linux

### Debian / Ubuntu / WSL2

```bash
sudo apt update
sudo apt install -y build-essential z3 llvm clang git
```

`build-essential` brings `gcc`, `make`, and libc headers.

### Fedora / RHEL / Rocky / AlmaLinux

```bash
sudo dnf install -y gcc make z3 llvm clang git
```

### Arch / Manjaro

```bash
sudo pacman -S --needed base-devel z3 llvm clang git
```

### Rust toolchain (any Linux distro)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustup default stable
```

### Build vanic

```bash
git clone https://github.com/enthusiasticgeek/vani-compiler.git
cd vani-compiler
cargo build --release
```

### Add vanic to PATH

```bash
# Option A — symlink into an already-on-PATH directory:
sudo ln -sf "$(pwd)/target/release/vanic" /usr/local/bin/vanic

# Option B — add the release dir to PATH permanently (in ~/.bashrc or ~/.zshrc):
echo 'export PATH="$HOME/vani-compiler/target/release:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

Verify:

```bash
vanic --version
```

---

## macOS

### Homebrew

```bash
brew install z3 llvm rustup-init git
rustup-init -y
```

After `brew install llvm`, add the LLVM tools to PATH (the formula is keg-only):

```bash
echo 'export PATH="$(brew --prefix llvm)/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

### Build vanic

```bash
git clone https://github.com/enthusiasticgeek/vani-compiler.git
cd vani-compiler
cargo build --release
sudo ln -sf "$(pwd)/target/release/vanic" /usr/local/bin/vanic
```

> **Apple Silicon (M1/M2/M3)**: the Homebrew path works natively;
> no Rosetta needed.

---

## Windows

Two options. **WSL2 is recommended** because it gives you a full Linux
runtime and all async I/O features work without the IOCP limitation.

### Option 1 — WSL2 (recommended)

Open **Administrator PowerShell**:

```powershell
wsl --install
# Reboot, then:
wsl --set-default-version 2
```

Inside your WSL2 shell, follow the **Linux (Debian/Ubuntu)** steps above.

### Option 2 — Native Windows 11

Verified: all **2089 compiler tests** pass on Windows 11 with this setup.

#### Step 1 — Rust (GNU toolchain)

```powershell
winget install Rustlang.Rustup --accept-package-agreements --accept-source-agreements
```

Open a **new** PowerShell window, then:

```powershell
rustup toolchain install stable-x86_64-pc-windows-gnu
rustup default stable-x86_64-pc-windows-gnu
```

> The GNU target uses `gcc` from MSYS2 (Step 2) and has no extra
> dependency. If you already have Visual Studio 2017+, the default
> MSVC toolchain also works — skip the `rustup default` switch.

#### Step 2 — GCC via MSYS2

Download and install MSYS2 from <https://www.msys2.org>.
The installer adds `C:\msys64\mingw64\bin` to your **system** PATH.

Verify in a **new** PowerShell window:

```powershell
gcc --version
```

#### Step 3 — LLVM tools (`lli`, `llc`, `opt`)

`winget install LLVM.LLVM` ships Clang but **not** `lli`/`llc`/`opt`.
Install the full LLVM set via MSYS2:

```powershell
C:\msys64\usr\bin\pacman.exe -Sy mingw-w64-x86_64-llvm --noconfirm
```

This places `lli.exe`, `llc.exe`, and `opt.exe` under `C:\msys64\mingw64\bin`.

> `lli` is only needed for `vanic run --backend=llvm`.
> The C backend (`--backend=c`) needs only `gcc` and works without `lli`.

#### Step 4 — z3 SMT solver

```powershell
C:\msys64\usr\bin\pacman.exe -Sy mingw-w64-x86_64-z3 --noconfirm
```

This places `z3.exe` in `C:\msys64\mingw64\bin` alongside gcc and lli.

Verify:

```powershell
z3 --version   # Z3 version 4.x
```

#### Step 5 — Build vanic

```powershell
git clone https://github.com/enthusiasticgeek/vani-compiler.git
cd vani-compiler
cargo build --release
```

#### Step 6 — Add vanic to PATH

Open **System Properties → Environment Variables → User variables → Path → New**
and add:

```
C:\path\to\vani-compiler\target\release
```

(replace `C:\path\to\` with wherever you cloned the repo)

Open a **new** PowerShell window and verify:

```powershell
vanic --version
```

#### Final PATH reference

After completing all steps:

| Directory | Contains |
|---|---|
| `C:\msys64\mingw64\bin` | `gcc`, `lli`, `llc`, `opt`, `z3` |
| `C:\Users\<you>\.cargo\bin` | `cargo`, `rustc`, `rustup` |
| `C:\path\to\vani-compiler\target\release` | `vanic.exe` |

---

## Verify your install

**Linux / macOS / WSL2:**

```bash
vanic --version
rustc --version
z3 --version
lli --version
```

**Native Windows:**

```powershell
vanic --version
rustc --version
z3 --version
lli --version
gcc --version
```

---

## Quick smoke test — Hello, World

**Linux / macOS / WSL2:**

```bash
vanic run examples/language/english/basics.vani
```

**Windows:**

```powershell
vanic run examples\language\english\basics.vani
```

Expected output: `42`

If you see `42`, your install is healthy and you're ready for [Lesson 1 →](beginner/01_hello_world.md).

---

## Troubleshooting

### `error: linker 'cc' not found`

- **Linux**: `sudo apt install build-essential` (or `gcc` on Fedora)
- **macOS**: `xcode-select --install`
- **Windows**: switch to the GNU toolchain (`rustup default stable-x86_64-pc-windows-gnu`) and ensure `C:\msys64\mingw64\bin` is on PATH

### `lli: command not found`

- **Linux/macOS**: LLVM not on PATH — check `llvm` package installation
- **macOS Homebrew**: re-run the `brew --prefix llvm` PATH export
- **Windows**: install via MSYS2 (`pacman -Sy mingw-w64-x86_64-llvm`)
- Note: `lli` is only needed for `--backend=llvm`; `--backend=c` works without it

### `z3: command not found`

- **Linux**: `sudo apt install z3`
- **Windows**: `C:\msys64\usr\bin\pacman.exe -Sy mingw-w64-x86_64-z3 --noconfirm`
- Set `VANIC_NO_VERIFY=1` to skip SMT entirely for fast iteration

### `cargo: command not found` (Windows)

Add `C:\Users\<you>\.cargo\bin` to your **system** PATH (not just user PATH):
System Properties → Environment Variables → System variables → Path → New.
Then restart the terminal.

### `pacman` mirrors unreachable (Windows)

```powershell
C:\msys64\usr\bin\pacman.exe -Sy msys2-keyring --noconfirm
C:\msys64\usr\bin\pacman.exe -Sy mingw-w64-x86_64-llvm --noconfirm
```

### Stack overflow in tests

The repo's `.cargo/config.toml` sets `RUST_MIN_STACK=33554432` automatically
for `cargo test`. If running test binaries directly, set it first:

```bash
# Linux/macOS
RUST_MIN_STACK=33554432 ./target/debug/deps/vani-*
```

```powershell
# Windows
$env:RUST_MIN_STACK = "33554432"
.\target\debug\deps\vani-*.exe
```

---

**→ Ready? [Begin with Hello, World](beginner/01_hello_world.md)**
