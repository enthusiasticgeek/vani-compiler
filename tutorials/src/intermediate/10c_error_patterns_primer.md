# Intermediate 10c -- Error patterns: nested errors, context, and FFI translation

> **Prerequisites**: [10a -- Result and try primer](10a_result_try_primer.md),
> [10b -- Runtime errors and abort](10b_runtime_errors_primer.md),
> [10 -- Result + manual propagation](10_result_try.md).
> This page covers the patterns you reach for once single-error
> propagation feels routine.

---

## Pattern 1: multiple error types in one function (the union enum)

The most common real-world complication: a function calls several
sub-functions that each return a *different* error type.

```vani
fn read_config(path: Str) -> IoResult    { ... }
fn parse_value(s: Str)    -> ParseResult { ... }
```

`Result<T, E>` is a built-in generic (see [Sec.10](10_result_try.md)),
but a single instantiation can only carry one concrete `E`. When two
sub-functions fail with genuinely different error shapes, wrap them
in a **union error enum** that names every error kind the caller
might receive, and have each sub-function return its own small
Ok/Err enum rather than `Result<T, E>` directly -- this also sidesteps
a current compiler limitation (constructing more than one distinct
`Result<T, E>`/`Option<T>` instantiation in the same program can break;
see the note at the end of this pattern):

```vani
enum IoResult    { Ok(i64), Err(i64) }
enum ParseResult { Ok(i64), Err(i64) }

fn read_config(path: Str) -> IoResult    { return IoResult.Ok(1); }
fn parse_value(s: Str)    -> ParseResult { return ParseResult.Ok(2); }

enum ConfigError {
  Io(i64),      // wraps IoResult's payload
  Parse(i64),   // wraps ParseResult's payload
}

fn load(path: Str) -> Result<i64, ConfigError> {
  // Step 1: call read_config, convert its error to ConfigError::Io
  let raw: i64 = 0;
  let step1: IoResult = read_config(path);
  if let IoResult.Ok(v) = step1 {
    raw = v;
  } else if let IoResult.Err(e) = step1 {
    return Result.Err(ConfigError.Io(e));
  }

  // Step 2: call parse_value, convert its error to ConfigError::Parse
  let value: i64 = 0;
  let step2: ParseResult = parse_value(path);
  if let ParseResult.Ok(v) = step2 {
    value = v;
  } else if let ParseResult.Err(e) = step2 {
    return Result.Err(ConfigError.Parse(e));
  }

  return Result.Ok(raw + value);
}
```

The caller unwraps the outer `Result` and then the inner `ConfigError`
with nested `if let` / `else if let` chains -- a single pattern can't
destructure both levels at once (`Result.Err(ConfigError.Io(e))` as
one pattern doesn't parse):

```vani
fn main() -> i64 {
  let outcome: Result<i64, ConfigError> = load("config.toml");
  if let Result.Ok(v) = outcome {
    print "loaded:", v;
  } else if let Result.Err(err) = outcome {
    if let ConfigError.Io(e) = err {
      print "IO error:", e;
    } else if let ConfigError.Parse(e) = err {
      print "parse error:", e;
    }
  }
  return 0;
}
```

> **Compiler limitation**: constructing two *different* instantiations
> of the same built-in generic enum (e.g. both `Result<i64, i64>` and
> `Result<i64, ConfigError>`, or `Option<i64>` alongside `Option<OwnedStr>`)
> anywhere in one program currently breaks every constructor call site
> for that generic. Pattern *matching* on an already-typed value is
> unaffected -- only *construction* breaks. Until this is fixed, keep
> sub-function results in their own small hand-declared enums (as
> above) and reserve `Result<T, E>` for a single outermost boundary.

### Naming the union enum

Name it after the *operation*, not the error types. `ConfigError`
describes what failed (loading config); `IoOrParseError` describes
the mechanism. The caller cares about the operation.

---

## Pattern 2: nested Result -- when to flatten

A nested `Result<Result<T, E1>, E2>` arises when an outer
operation wraps an inner one that can also fail. It is almost
always a sign that the two error layers should be merged into a
union enum (Pattern 1), not stacked.

**Avoid**:

<img class="manas" src="../images/mascot/manas_mascot_caution.png" title="this code needs extra care"/>

```vani
fn outer() -> Result<Result<i64, ParseError>, IoError> {
  // Caller must double-match: awkward and error-prone
}
```

**Prefer** -- flatten with a union enum:

```vani
enum OuterError { Io(i64), Parse(i64) }

fn outer() -> Result<i64, OuterError> { ... }
```

The only time a nested Result is intentionally kept is when the
*two failure modes have different recovery strategies* that the
caller must distinguish at structurally different levels. In
practice this is rare in vāṇी code; reach for the union enum
first.

---

## Pattern 3: adding context to propagated errors

A bare `ParseError(7)` tells the caller what went wrong but not
*where* or *why*. In production programs the same error type
(parse failure, IO error) can arise in dozens of call sites; the
caller needs context to distinguish them.

**Pattern: wrap with a message string**

`Result<T, E>`'s `E` position accepts `OwnedStr` directly -- you
don't need to wrap it in a custom enum (in fact, wrapping `OwnedStr`
inside a non-Copy custom enum is rejected: "supported payloads are
Copy types, OwnedStr, Vec\<T\>, Box\<T\>, ..."). Use
`Result<i64, OwnedStr>` and attach the context string at each
propagation site:

```vani
fn load_user(path: Str) -> Result<i64, OwnedStr> {
  if !read_config_ok(path) {
    return Result.Err("load_user: could not read " + path);
  }
  return Result.Ok(0);
}

fn load_product(path: Str) -> Result<i64, OwnedStr> {
  if !read_config_ok(path) {
    return Result.Err("load_product: could not read " + path);
  }
  return Result.Ok(0);
}
```

Now two IO errors from different call sites produce different
messages. The caller sees:

```
IO error: load_user: could not read /etc/users.toml
IO error: load_product: could not read /etc/products.toml
```

instead of two identical numeric error codes.

**Trade-off**: the context string is heap-allocated (`OwnedStr`).
In hot-path code (tight loops, parsers), prefer a numeric error
code and add context only at the outermost boundary where you
format the message for the user.

---

## Pattern 4: Option<T> -- absence is not failure

`Option<T>` is for "this value might not exist" -- not for "an
error occurred". The distinction matters:

| Use `Result<T, E>` | Use `Option<T>` |
|---|---|
| File not found | Key not in map |
| Parse failure | First element of a Vec |
| Network timeout | Field that might be unset |
| Auth rejected | Search that might return nothing |

Mixing them up produces confusing call sites. `Result.Err`
implies something *went wrong*; `Option.None` implies the value
simply *isn't there*.

**Converting Option to Result when you need a reason**:

```vani
fn lookup_or_error(map: ref HashMap<i64, i64>, key: i64)
    -> Result<i64, OwnedStr> {
  let found: Option<i64> = hashmap_get(map, key);
  if let Option.Some(v) = found {
    return Result.Ok(v);
  }
  return Result.Err("key not found: " + i64_to_str(key));
}
```

This is the standard "option-to-result" lift: you know the
value should be present and its absence is an error worth
reporting with a message.

> **Note**: `HashMap<Str, V>` (a borrowed key) is rejected -- use
> `HashMap<OwnedStr, V>` or a scalar key type like `i64` (as above).
> Separately, passing `ref HashMap<OwnedStr, V>` as a function
> parameter currently miscompiles under `--backend=c` (the LLVM
> backend handles it correctly); if you need an owned-string key
> passed by reference into a function, test on the C backend first
> or keep the map access inside the same function.

---

## Pattern 5: FFI error translation

C functions signal errors through return values (typically -1 or
0) and `errno`. The canonical vāṇी pattern wraps the C call in
a thin function that translates to a `Result`:

```vani
extern "C" fn c_open(path: Str, flags: i32) -> i32;
extern "C" fn c_errno() -> i32;

enum SysError { Os(i64) }   // payload is the errno value

fn open_file(path: Str) -> Result<i64, SysError> {
  let fd: i32 = c_open(path, 0);
  if fd < 0 {
    let code: i64 = c_errno() as i64;
    return Result.Err(SysError.Os(code));
  }
  return Result.Ok(fd as i64);
}
```

**Rules for FFI error wrapping**:
1. **Never let C's -1 escape into vāṇी code as a raw integer.**
   Wrap at the boundary; internal vāṇी code sees only `Result`.
2. **One wrapper per C function family.** Don't scatter raw
   `extern` calls through the codebase; put them in one FFI
   module with `Result`-returning wrappers.
3. **Preserve the errno.** Capture `errno` immediately after
   the failing call -- the next system call will overwrite it.
4. **Use `Str`, not raw pointers, at the FFI boundary.** Raw
   pointer types (`*const T` / `*mut T`) can't cross as
   `extern "C" fn` parameters or returns in v1 -- pass `Str`
   (or `ref T`) instead; there's no `str_to_cstr` builtin needed.

---

## Pattern 6: the "unwrap or abort" shortcut

Sometimes you know the Result must be Ok (you've already
validated earlier) and writing a full match is noise. The
idiomatic vāṇी approach is an `assert`-style helper:

```vani
fn unwrap_or_abort(r: Result<i64, i64>, msg: Str) -> i64 {
  if let Result.Ok(v) = r {
    return v;
  }
  print msg;
  assert false;   // abort with the message above
  return 0;       // unreachable; satisfies type checker
}
```

> **Note**: a `then { ... }` match arm can only hold a single
> expression, not a multi-statement block -- so the abort path
> (print, then assert, then a placeholder return) is written as
> plain fall-through code after an `if let` early return instead
> of as a `match` arm.

Use this only for values you have *already proven cannot fail*
via earlier validation or `requires` contracts. If the assertion
ever fires in production it means the earlier validation was
incomplete -- a programmer bug, not a recoverable condition.

---

## Quick reference

| Situation | Pattern |
|---|---|
| One function, multiple error types | Union error enum (Pattern 1) |
| Nested `Result<Result<...>>` | Flatten to union enum (Pattern 2) |
| Same error from many call sites | Context string in error payload (Pattern 3) |
| Value that might be absent (not failed) | `Option<T>`, not `Result` (Pattern 4) |
| C / FFI error codes | Thin wrapper → `Result<T, SysError>` (Pattern 5) |
| "I know this can't fail" | `unwrap_or_abort` + earlier `assert`/`requires` (Pattern 6) |

---

**Previous**: [Sec.10 -- Error handling: `Result<T, E>` + `try` ->](10_result_try.md)
**Next**: [Sec.13 -- `Option<T>` and the option builtins ->](13_option.md)
