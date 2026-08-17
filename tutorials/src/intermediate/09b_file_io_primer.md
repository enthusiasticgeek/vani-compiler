# Intermediate 9b -- Native file I/O (primer)

> **Learning goal**: understand how vāṇī's `FileHandle` type
> gives you safe, RAII-managed file access without FFI -- and
> when you still need FFI for device I/O.
> Reading order: [Intermediate 9 -- FFI](09_ffi.md) -> here ->
> [Intermediate 10 -- Result/try](10_result_try.md).

This chapter leads with concepts, then a one-page API reference
with real code.

## Checking out a library book

Picture a public library. To read a book, you don't just grab it off
the shelf and walk out -- you take it to the front desk and check it
out. The librarian scans it against your library card, and now
there's a record: this card has this book, checked out right now.
You didn't get a photocopy of every page in advance -- you got
permission to read the book, plus a record of which book you're
holding.

Once you're home, you don't photograph all three hundred pages before
you start reading. You open to chapter one, read it, then turn to
chapter two, then chapter three -- a chunk at a time, in order, only
pulling in the next piece when you're ready for it. You might read
quickly or slowly, skim or take notes, but either way you're pulling
content out of the same checked-out book, a bit at a time, not all at
once in one giant gulp.

When you're done, you return the book to the desk. The librarian
updates the record: this card no longer has this book. Only after
that return is the book free for the next patron to check out. If
you forgot to return it, the record would just sit there forever,
saying you still have a book you're not even reading anymore -- and
eventually the library would have to chase you down about it.

And sometimes a book itself has a problem: a chipped barcode the
scanner can't read, water-damaged pages, or it was pulled for
rebinding and isn't actually on the shelf where the catalog claims.
When that happens, checkout itself fails, or reading partway through
gives you garbage instead of the next chapter. You have to notice the
book is unusable before you trust anything you tried to read from it.

A `FileHandle` in vāṇī is exactly this checkout record. **Opening a
file** (`file_open(...)`) is checking the book out -- you get back a
`FileHandle`, the record of which file you're now holding.
**Reading** a file a line or chunk at a time (`file_read_line`) is
reading a chapter at a time rather than photographing the whole book
up front. **Closing the handle** is returning the book to the desk --
after that, the record is cleared, and unlike a forgetful human, the
compiler makes sure this always happens, even if you leave the
function early. And a **file I/O error** -- a path that doesn't
exist, permissions that block you, a device that vanished -- is the
equivalent of that chipped, unreadable book: something you have to
detect before you trust anything you tried to read from it.

Keep the librarian's desk in mind. Everything below -- `FileHandle`,
opening, reading, closing, checking for errors -- is that same
checkout process, spelled out with vāṇī's exact function names.

## Why a dedicated FileHandle type?

Before v0.1.5, the only way to open a file in vāṇī was to call
`fopen` via `extern "C"` and treat the `FILE*` as an opaque
`i64`. That worked -- but it leaked files on early return, and
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
+== scope entry ==========================+
|  let fw: FileHandle = file_open(...)     |
|  use fw ...                              |
+== scope exit -- compiler inserts: ======+
          fclose(fw)   <- automatic
```

This is the same pattern `OwnedStr` uses for heap strings and
`Vec<T>` uses for heap arrays.

## The nine builtins

| Builtin | What it does |
|---------|--------------|
| `file_open(path, mode, buffered)` | Opens `path` in `mode` (`"r"`, `"w"`, `"a"`, `"r+"`, ...). `buffered: bool` -- `true` for normal libc buffering, `false` for `setvbuf(..., _IONBF, ...)` (every write hits the OS immediately). Returns a `FileHandle`. |
| `file_is_ok(ref fh)` | Returns `true` if the handle is valid (fopen succeeded). Always check before reading/writing. |
| `file_read_line(mut ref fh)` | Reads one line (up to `\n`) into a fresh `OwnedStr`. Returns empty string at EOF. |
| `file_write(mut ref fh, s)` | Writes `s` (a `Str`) to the file. Returns bytes written or -1. |
| `file_flush(mut ref fh)` | Flushes the write buffer. Returns 0 on success. |
| `file_close(fh)` | Explicitly closes the file now (consumes the handle). Automatic close also happens at scope exit. |
| `stdin_read_line()` | Reads one line from stdin into an `OwnedStr`. |
| `flush_stdout()` | Flushes stdout (useful before `stdin_read_line` in interactive mode). |
| `eprint ITEMS` | Writes to stderr, same syntax as `print`. Not a function -- a statement. |

## Reference vs `mut ref`

The access model follows vāṇī's normal borrowing rules:

- `file_is_ok(ref fh)` -- read-only check; borrows the handle.
- `file_read_line(mut ref fh)` -- advances the read position; needs a mutable borrow.
- `file_write(mut ref fh, s)` -- writes bytes; mutable borrow.
- `file_close(fh)` -- **consumes** the handle (no `ref`). After this call, `fh` is gone; the compiler rejects any further use.

## The `eprint` statement

```vani
eprint "error:", msg;
```

Writes to `stderr` -- the error output channel separate from `stdout`. Use it for diagnostics that shouldn't mix with program output:

```
vanic run my_prog.vani > output.txt   # stdout -> file
                                       # stderr -> terminal (visible)
```

## When do you still need FFI?

| Scenario | Use |
|----------|-----|
| Flat files (text logs, config) | `FileHandle` -- no FFI needed |
| stdin line-by-line | `stdin_read_line()` -- no FFI needed |
| stderr messages | `eprint` -- no FFI needed |
| Serial port (RS232 / RS485 / UART) | FFI + C shim -- kernel `struct termios` is aggregate-by-value, rejected at the FFI boundary |
| I2C / SPI peripherals | FFI + C shim -- same reason |
| Binary seek / random access | FFI -- `fseek`/`ftell` not yet native |

## A mental model summary

- **`FileHandle`** is an affine heap resource -- it tracks a C `FILE*` safely, with automatic close.
- **Read before write**: always check `file_is_ok` before using a handle.
- **`file_close`** is optional if you're happy with scope-exit close, but explicit close is good practice when you need to know the close succeeded.
- **`eprint`** is to stderr what `print` is to stdout -- same syntax, different channel.

## Cross-reference

- [Intermediate 9 -- FFI: `extern "C"` + `--link-with`](09_ffi.md) -- the FFI patterns needed for device I/O that file I/O builtins don't cover
- [Intermediate 9 FFI Sec. File I/O section](09_ffi.md) -- updated to show the boundary between native and FFI I/O
- [Advanced 4 -- Embedded targets](../advanced/04_embedded.md) -- where `eprint` and `FileHandle` fit in firmware work
- [`examples/language/english/file_io.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/english/file_io.vani) -- runnable worked example


---

**Previous**: [Sec.9 -- FFI: extern C + --link-with ->](09_ffi.md)
**Next**: [Sec.9c -- Native file I/O: FileHandle + eprint ->](09c_file_io.md)

