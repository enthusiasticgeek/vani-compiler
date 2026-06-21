# Intermediate 9b — Native file I/O (primer)

> **Learning goal**: understand how vāṇī's `FileHandle` type
> gives you safe, RAII-managed file access without FFI — and
> when you still need FFI for device I/O.
> Reading order: [Intermediate 9 — FFI](09_ffi.md) → here →
> [Intermediate 10 — Result/try](10_result_try.md).

This chapter has **no compiler code**. Pure concepts, then the
one-page API reference.

## Why a dedicated FileHandle type?

Before v0.1.5, the only way to open a file in vāṇī was to call
`fopen` via `extern "C"` and treat the `FILE*` as an opaque
`i64`. That worked — but it leaked files on early return, and
there was no type that enforced "close me exactly once."

A `FileHandle` is an **affine type**: the compiler tracks it
like an owned heap pointer. When the binding goes out of scope,
the compiler inserts an automatic `fclose`. You cannot move it
into two places; you cannot forget to close it.

Think of it like a library book that's chipped: the turnstile
won't let you out without returning it, and you can only carry
one copy.

## The RAII pattern: close on scope exit

```
╔══ scope entry ══════════════════════════╗
║  let fw: FileHandle = file_open(…)     ║
║  use fw …                              ║
╚══ scope exit — compiler inserts: ══════╝
          fclose(fw)   ← automatic
```

This is the same pattern `OwnedStr` uses for heap strings and
`Vec<T>` uses for heap arrays.

## The nine builtins

| Builtin | What it does |
|---------|--------------|
| `file_open(path, mode)` | Opens `path` in `mode` (`"r"`, `"w"`, `"a"`, `"r+"`, …). Returns a `FileHandle`. |
| `file_is_ok(ref fh)` | Returns `true` if the handle is valid (fopen succeeded). Always check before reading/writing. |
| `file_read_line(mut ref fh)` | Reads one line (up to `\n`) into a fresh `OwnedStr`. Returns empty string at EOF. |
| `file_write(mut ref fh, s)` | Writes `s` (a `Str`) to the file. Returns bytes written or -1. |
| `file_flush(mut ref fh)` | Flushes the write buffer. Returns 0 on success. |
| `file_close(fh)` | Explicitly closes the file now (consumes the handle). Automatic close also happens at scope exit. |
| `stdin_read_line()` | Reads one line from stdin into an `OwnedStr`. |
| `flush_stdout()` | Flushes stdout (useful before `stdin_read_line` in interactive mode). |
| `eprint ITEMS` | Writes to stderr, same syntax as `print`. Not a function — a statement. |

## Reference vs `mut ref`

The access model follows vāṇī's normal borrowing rules:

- `file_is_ok(ref fh)` — read-only check; borrows the handle.
- `file_read_line(mut ref fh)` — advances the read position; needs a mutable borrow.
- `file_write(mut ref fh, s)` — writes bytes; mutable borrow.
- `file_close(fh)` — **consumes** the handle (no `ref`). After this call, `fh` is gone; the compiler rejects any further use.

## The `eprint` statement

```vani
eprint "error:", msg;
```

Writes to `stderr` — the error output channel separate from `stdout`. Use it for diagnostics that shouldn't mix with program output:

```
vanic run my_prog.vani > output.txt   # stdout → file
                                       # stderr → terminal (visible)
```

## When do you still need FFI?

| Scenario | Use |
|----------|-----|
| Flat files (text logs, config) | `FileHandle` — no FFI needed |
| stdin line-by-line | `stdin_read_line()` — no FFI needed |
| stderr messages | `eprint` — no FFI needed |
| Serial port (RS232 / RS485 / UART) | FFI + C shim — kernel `struct termios` is aggregate-by-value, rejected at the FFI boundary |
| I2C / SPI peripherals | FFI + C shim — same reason |
| Binary seek / random access | FFI — `fseek`/`ftell` not yet native |

## A mental model summary

- **`FileHandle`** is an affine heap resource — it tracks a C `FILE*` safely, with automatic close.
- **Read before write**: always check `file_is_ok` before using a handle.
- **`file_close`** is optional if you're happy with scope-exit close, but explicit close is good practice when you need to know the close succeeded.
- **`eprint`** is to stderr what `print` is to stdout — same syntax, different channel.

## Cross-reference

- [Intermediate 9 — FFI: `extern "C"` + `--link-with`](09_ffi.md) — the FFI patterns needed for device I/O that file I/O builtins don't cover
- [Intermediate 9 FFI § File I/O section](09_ffi.md) — updated to show the boundary between native and FFI I/O
- [Advanced 4 — Embedded targets](../advanced/04_embedded.md) — where `eprint` and `FileHandle` fit in firmware work
- [`examples/language/english/file_io.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/english/file_io.vani) — runnable worked example
