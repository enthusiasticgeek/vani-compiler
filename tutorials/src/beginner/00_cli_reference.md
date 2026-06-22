# CLI Reference — `vanic` command reference

> A one-page reference for every `vanic` subcommand, flag, and
> environment variable. Bookmark this; you'll come back often.

---

## Quick-start

```bash
vanic run hello.vani              # compile + run (LLVM backend, default)
vanic run hello.vani --backend=c  # compile + run via C backend
vanic build hello.vani -o hello   # AOT-compile to a native binary
vanic check hello.vani            # type-check only; no output
```

---

## Subcommands

### `vanic run <file.vani>`

Compile and immediately run a program. The LLVM backend (default) emits
`.ll` IR and runs it through `lli` (the LLVM JIT). The C backend emits
`.c`, compiles with `cc`, and runs the resulting binary.

```bash
vanic run hello.vani
vanic run hello.vani --backend=c
vanic run hello.vani --backend=c --link-with helper.c -lm
vanic run hello.vani --target=aarch64-unknown-linux-gnu   # cross + QEMU
vanic run hello.vani --big-o          # print per-fn Big-O before running
```

| Flag | Description |
|------|-------------|
| `--backend=c\|llvm` | Select backend. Default: `llvm`. |
| `--link-with PATH` | Extra `.c` / `.o` / `.a` file to compile alongside (C backend only). Repeatable. |
| `-l<name>` | System library flag forwarded to cc (e.g. `-lm`). C backend only. |
| `--target=<triple>` | Cross-compile triple. Bare-metal triples print a helpful error; Linux cross-targets build an ELF and run via QEMU. |
| `--big-o[=auto\|force\|off]` | Print per-function Big-O complexity to stderr before running. `auto` skips O(1) fns; `force` includes all; `off` disables. |

**Environment variables**: `CC` (C compiler), `LLI` (LLVM JIT), `VANIC_NO_VERIFY=1` (skip SMT checks), `VANIC_SMT_DEBUG=1` (dump Z3 queries).

---

### `vanic build <file.vani>`

AOT-compile to a native binary. Pipeline: emit `.ll` → `llc -filetype=obj` → `.o` → `cc -o <out>`.

```bash
vanic build hello.vani                       # outputs ./hello
vanic build hello.vani -o /tmp/hello         # custom output path
vanic build fw.vani --target=arm-none-eabi -o fw.elf   # cross-compile
vanic build fw.vani --no-std --target=arm-none-eabi    # explicit no-std
vanic build hello.vani --link-with helper.c -lm
```

| Flag | Description |
|------|-------------|
| `-o PATH` / `--out PATH` | Output binary path. Default: source file stem in cwd. |
| `--link-with PATH` | Extra input for the linker. Repeatable. |
| `-l<name>` | System library (e.g. `-lm`). Forwarded to cc. |
| `--target=<triple>` | Cross-compile. Passes `--mtriple=<triple>` to `llc`; selects `$CROSS_CC` or `<triple>-gcc` as linker. Bare-metal triples suppress libc/OpenMP flags. |
| `--no-std` | Omit libc headers from C prelude. Auto-activated for bare-metal triples. |

**Environment variables**: `CC`, `LLC`, `OPT`, `CROSS_CC` (cross-linker override), `QEMU_<ARCH>` (QEMU binary override).

---

### `vanic check <file.vani>`

Type-check + SMT-verify a program without producing any output. Fast — only runs the checker pipeline.

```bash
vanic check hello.vani
vanic check --no-verify hello.vani   # skip SMT; only type-check
```

Exit 0 on success, 1 on any error.

---

### `vanic emit <file.vani>`

Emit lowered source (LLVM IR or C) to stdout or a file.

```bash
vanic emit hello.vani                       # LLVM IR to stdout
vanic emit hello.vani --backend=c           # C source to stdout
vanic emit hello.vani --backend=c --no-std  # C with bare-metal prelude
vanic emit hello.vani -o hello.ll           # LLVM IR to file
vanic emit hello.vani --backend=c -o hello.c
vanic emit hello.vani --big-o               # prepend Big-O annotations
```

`vanic emit-c` is a legacy alias that forces `--backend=c`.

---

### `vanic fmt <file.vani | directory> [--check]`

Format vāṇी source files. Without `--check`, rewrites the file in place.

```bash
vanic fmt hello.vani            # format in place
vanic fmt src/                  # format all .vani files under src/
vanic fmt --check hello.vani    # exit 1 if file is not already formatted
vanic fmt --check src/          # CI-style check for entire tree
```

---

### `vanic test <file.vani | directory>`

Run `vanic test` files — each file's `main` is expected to return 0 for pass, non-zero for fail.

```bash
vanic test tests/
vanic test tests/math_test.vani
```

Prints `ok` / `FAIL` per file, exits 1 if any fail.

---

### `vanic tokens <file.vani>`

Dump the lexer token stream. Useful for debugging parser or lexer issues.

---

### `vanic ast <file.vani>`

Dump the parsed AST (skips the type checker). Useful when you want to see what the parser produced even if the type checker would reject it.

---

### `vanic ir <file.vani>`

Dump the typed IR — the representation the backends see after full type-checking. Useful for checking what the backends will lower.

---

### `vanic stack-depth <file.vani>`

Per-function stack frame estimates and maximum stack depth from each entry point. Required for ASIL-D / DO-178C.

```bash
vanic stack-depth fw.vani
vanic stack-depth fw.vani --max=4096        # exit 1 if any path exceeds 4 KiB
vanic stack-depth fw.vani --entry=main      # only report from main
vanic stack-depth fw.vani --format=json     # machine-readable output
```

---

### `vanic acyclicity <file.vani>`

Prove the call graph has no cycles. Catches mutual recursion. Required for DO-178C / ASIL-D.

```bash
vanic acyclicity fw.vani
vanic acyclicity fw.vani --format=csv
```

---

### `vanic deviations <file.vani>`

Extract every `unsafe(reason = "…")` block as a structured deviation record — the audit artifact for ASIL-D / DO-178C / MISRA sign-off.

```bash
vanic deviations fw.vani
vanic deviations fw.vani --format=json --out=deviations.json
```

---

### `vanic audit-pack <file.vani>`

Run all six audit reports (deviations, stack-depth, acyclicity, hashmap-usage, complexity, safety-attrs) and bundle them into a single Markdown artifact.

```bash
vanic audit-pack fw.vani --out=audit.md
vanic audit-pack fw.vani --max-stack=8192 --max-complexity=20
```

---

### `vanic complexity <file.vani>`

Report per-function Big-O complexity annotations. Same as `--big-o` on run/emit but standalone.

---

### `vanic safety-attrs <file.vani>`

Report which functions carry which safety attributes (`#[no_heap]`, `#[bounded_stack]`, `#[interrupt]`, `#[no_mangle]`, etc.).

---

### Kosh package manager subcommands

| Command | Description |
|---------|-------------|
| `vanic add <name>[@constraint]` | Fetch package from registry → `vendor/`, update `vani.toml` + `vani.lock` |
| `vanic remove <name>` | Remove package from `vani.toml` |
| `vanic update` | Re-resolve all deps to latest compatible versions |
| `vanic vendor` | Download all deps into `vendor/` |
| `vanic search <query>` | Search the Kosh registry |
| `vanic publish` | Build tarball + create GitHub Release + append registry entry |

---

## Global flags

| Flag | Description |
|------|-------------|
| `--no-verify` | Skip SMT verification (type-checking still runs). Also: `VANIC_NO_VERIFY=1`. |
| `--smt-debug` | Dump every Z3 query and response to stderr. Also: `VANIC_SMT_DEBUG=1`. |
| `-V` / `--version` | Print version and exit. |
| `-h` / `--help` | Print usage and exit. |

---

## Environment variables summary

| Variable | Effect |
|----------|--------|
| `CC` | C compiler for the C backend. Default: `cc`. |
| `LLC` | LLVM object-code compiler. Default: `llc`. |
| `LLI` | LLVM JIT for `vanic run`. Default: `lli`. |
| `OPT` | LLVM optimizer. Default: `opt`. Skipped gracefully if absent. |
| `Z3` | Z3 SMT solver. Default: `z3`. |
| `CROSS_CC` | Cross-linker override for `--target` builds. |
| `QEMU_<ARCH>` | QEMU user-mode binary override (e.g. `QEMU_AARCH64`). |
| `VANIC_NO_VERIFY=1` | Skip SMT; equivalent to `--no-verify`. |
| `VANIC_SMT_DEBUG=1` | Dump Z3 queries; equivalent to `--smt-debug`. |
| `INTENT_TARGET_EMBEDDED=1` | Enable stack-protector hardening on the link line. |
| `INTENT_LIBGOMP` | Non-standard libgomp path for LLVM JIT. |
