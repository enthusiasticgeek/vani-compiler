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
fn read_config(path: Str) -> Result<i64, IoError>   { ... }
fn parse_value(s: Str)    -> Result<i64, ParseError> { ... }
```

You want one function that calls both. v1 has no generic
`Result<T, E>` or trait-object error type, so the answer is a
**union error enum** that names every error kind the caller
might receive:

```vani
enum ConfigError {
  Io(i64),      // wraps IoError's payload
  Parse(i64),   // wraps ParseError's payload
}

fn load(path: Str) -> Result<i64, ConfigError> {
  // Step 1: call read_config, convert its error to ConfigError::Io
  let raw: i64 = match read_config(path) {
    Result.Ok(v)  then v,
    Result.Err(e) then return Result.Err(ConfigError.Io(e)),
  };

  // Step 2: call parse_value, convert its error to ConfigError::Parse
  let value: i64 = match parse_value(path) {
    Result.Ok(v)  then v,
    Result.Err(e) then return Result.Err(ConfigError.Parse(e)),
  };

  return Result.Ok(value);
}
```

The caller matches on `ConfigError` and handles each case
independently:

```vani
fn main() -> i64 {
  match load("config.toml") {
    Result.Ok(v)                     then { print "loaded:", v; }
    Result.Err(ConfigError.Io(e))    then { print "IO error:", e; }
    Result.Err(ConfigError.Parse(e)) then { print "parse error:", e; }
  }
  return 0;
}
```

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

Define your error enum to carry an `OwnedStr` context slot:

```vani
enum AppError {
  Parse(OwnedStr),   // message + site
  Io(OwnedStr),
}
```

At each propagation site, attach the context string before
returning:

```vani
fn load_user(path: Str) -> Result<i64, AppError> {
  match read_config(path) {
    Result.Ok(v)  then { /* continue */ }
    Result.Err(_) then {
      return Result.Err(AppError.Io(
        "load_user: could not read " + path
      ));
    }
  }
  // ... rest of function
  return Result.Ok(0);
}

fn load_product(path: Str) -> Result<i64, AppError> {
  match read_config(path) {
    Result.Ok(v)  then { /* continue */ }
    Result.Err(_) then {
      return Result.Err(AppError.Io(
        "load_product: could not read " + path
      ));
    }
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

instead of two identical `AppError.Io(13)` values.

**Trade-off**: the context string is heap-allocated (`OwnedStr`).
In hot-path code (tight loops, parsers), prefer a numeric error
code enum and add context only at the outermost boundary where
you format the message for the user.

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
fn lookup_or_error(map: ref HashMap<Str, i64>, key: Str)
    -> Result<i64, AppError> {
  match map_get(ref map, key) {
    Option.Some(v) then return Result.Ok(v),
    Option.None    then return Result.Err(
      AppError.Parse("key not found: " + key)
    ),
  }
}
```

This is the standard "option-to-result" lift: you know the
value should be present and its absence is an error worth
reporting with a message.

---

## Pattern 5: FFI error translation

C functions signal errors through return values (typically -1 or
0) and `errno`. The canonical vāṇी pattern wraps the C call in
a thin function that translates to a `Result`:

```vani
extern "C" fn c_open(path: *const i8, flags: i32) -> i32;
extern "C" fn c_errno() -> i32;

enum SysError { Os(i64) }   // payload is the errno value

fn open_file(path: Str) -> Result<i64, SysError> {
  // Convert Str to a null-terminated C string via FFI shim
  let fd: i32 = c_open(str_to_cstr(path), 0);
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

---

## Pattern 6: the "unwrap or abort" shortcut

Sometimes you know the Result must be Ok (you've already
validated earlier) and writing a full match is noise. The
idiomatic vāṇी approach is an `assert`-style helper:

```vani
fn unwrap_or_abort(r: Result, msg: Str) -> i64 {
  return match r {
    Result.Ok(v)  then v,
    Result.Err(_) then {
      print msg;
      assert false;  // abort with the message above
      0              // unreachable; satisfies type checker
    },
  };
}
```

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

**Previous**: [10b -- Runtime errors and abort](10b_runtime_errors_primer.md)  
**Next**: [`Option<T>` and the option builtins ->](13_option.md)
