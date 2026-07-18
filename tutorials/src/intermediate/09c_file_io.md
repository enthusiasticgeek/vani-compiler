# Intermediate 9c -- File I/O: `FileHandle` + `eprint`

> **Learning goal**: open, read, write, and close files using
> vāṇī's native `FileHandle` type; write to stderr with
> `eprint`; read stdin line-by-line with `stdin_read_line`.

> **New to this?** Read [Intermediate 9b -- File I/O primer](09b_file_io_primer.md) first.

## The program

```vani
intent "Intermediate 9c -- native file I/O.";

fn main() -> i64 {
  // -- write ------------------------------------------------
  let fw: FileHandle = file_open("/tmp/vani_hello.txt", "w");
  if !file_is_ok(ref fw) {
    eprint "error: could not open file for writing";
    return 1;
  }
  let _ = file_write(mut ref fw, "hello from vani\n");
  let _ = file_write(mut ref fw, "second line\n");
  let _ = file_flush(mut ref fw);
  let _ = file_close(fw);           // explicit close; could omit (scope-exit does it)

  // -- read back --------------------------------------------
  let fr: FileHandle = file_open("/tmp/vani_hello.txt", "r");
  if !file_is_ok(ref fr) {
    eprint "error: could not open file for reading";
    return 1;
  }
  let line1: OwnedStr = file_read_line(mut ref fr);
  let line2: OwnedStr = file_read_line(mut ref fr);
  print "line 1:", line1;
  print "line 2:", line2;
  // fr is closed automatically at scope exit

  return 0;
}
```

## Compile + run

```bash
vanic run file_io.vani
```

Output:

```
line 1: hello from vani
line 2: second line
```

## Why it works that way

- **`file_open(path, mode)`** calls `fopen` internally and
  stores the `FILE*` inside a `FileHandle`. The handle is
  affine -- the compiler tracks it like an `OwnedStr`.
- **`file_is_ok(ref fh)`** checks that `fopen` succeeded
  (returned a non-null pointer). Always test before reading
  or writing; if the check fails, any use of the handle is
  undefined behaviour in C -- vāṇī's builtin makes this
  explicit by requiring you to check before the mutable
  operations are useful.
- **`file_read_line(mut ref fh)`** reads up to and including
  the next `\n` byte (or EOF), returns an `OwnedStr` you own.
  At EOF it returns an empty string. Passing `mut ref` because
  reading advances the file position.
- **`file_write(mut ref fh, s)`** accepts any `Str`-coercible
  value (a string literal, an `OwnedStr` coerced to `Str`, etc.).
  Returns the number of bytes written, or -1 on error.
- **`file_flush(mut ref fh)`** flushes the OS write buffer.
  Important before `file_close` when the OS might batch writes.
- **`file_close(fh)`** consumes the handle -- no `ref`. The
  compiler marks `fw` / `fr` as moved; any use after this
  is a compile error. Scope-exit also closes automatically
  if you don't call `file_close` explicitly.

## Reading stdin

```vani
intent "stdin example";

fn main() -> i64 {
  let _ = flush_stdout();           // flush pending output before prompting
  print "enter your name:";
  let _ = flush_stdout();
  let name: OwnedStr = stdin_read_line();
  print "hello,", name;
  return 0;
}
```

`stdin_read_line()` reads one line (up to `\n`) from standard
input. The returned `OwnedStr` includes the trailing newline if
one was present.

## Writing to stderr

The `eprint` statement sends output to stderr -- the diagnostic
channel that isn't captured when the user redirects stdout:

```vani
eprint "fatal: file not found:", path;
```

`eprint` accepts the same comma-separated list as `print` and
adds a newline at the end.

## Append mode

```vani
let fa: FileHandle = file_open("/var/log/app.log", "a");
if file_is_ok(ref fa) {
  let _ = file_write(mut ref fa, "2026-06-21: started\n");
  let _ = file_close(fa);
}
```

`"a"` (append) positions the write head at the end of the
file on every write. Existing content is preserved.

## Common gotchas

- **Check `file_is_ok` before using the handle.** If `fopen`
  failed (wrong path, no permission), the handle holds a null
  pointer. The builtins guard against it, but checking early
  lets you emit a useful error message.
- **`file_read_line` at EOF returns `""`**, not an error.
  Loop until you get an empty string to read a whole file:

  ```vani
  let fr: FileHandle = file_open("data.txt", "r");
  let line: OwnedStr = file_read_line(mut ref fr);
  while line != "" {
    print line;
    line = file_read_line(mut ref fr);
  }
  ```

- **Binary I/O** is not yet native. Use FFI for `fread`/`fwrite`.
- **Random access** (`fseek`/`ftell`) is not yet native. Use FFI.

## Challenge

Write a program that reads a filename from stdin, opens it in
read mode, and counts + prints the number of lines.

<details>
<summary>Solution</summary>

```vani
intent "line counter";

fn main() -> i64 {
  let _ = flush_stdout();
  print "filename:";
  let _ = flush_stdout();
  let name: OwnedStr = stdin_read_line();
  let fh: FileHandle = file_open(name, "r");
  if !file_is_ok(ref fh) {
    eprint "error: cannot open", name;
    return 1;
  }
  let count: i64 = 0;
  let line: OwnedStr = file_read_line(mut ref fh);
  while line != "" {
    count = count + 1;
    line = file_read_line(mut ref fh);
  }
  print "lines:", count;
  return 0;
}
```

</details>

---

**Previous**: [Sec.9b -- Native file I/O primer ->](09b_file_io_primer.md)
**Next**: [Sec.9d -- Build-system integration ->](09d_build_systems.md)
