# Advanced 1 — Async / await and the `Task` transform

> **Learning goal**: declare `async fn`, call it with `await`,
> thread a `CancelToken` through long operations, and
> understand the v1 desugar from `async fn` to `Future<R>`.

## The program

```vani
intent "Advanced 1 — async fn + await + CancelToken.";

async fn fetch(n: i64) -> i64 {
  return n * 7;
}

async fn cancellable(n: i64, token: ref CancelToken) -> i64 {
  if token.cancelled {
    return 0 - 1;
  }
  return n + 100;
}

fn main() -> i64 {
  // Plain async fn + await round-trip.
  let r1: i64 = await(fetch(6));
  print "await fetch(6) =", r1;

  // CancelToken not cancelled — produces value.
  let tok: CancelToken = CancelToken { cancelled: false };
  let r2: i64 = await(cancellable(7, ref tok));
  print "await cancellable(7, !tok) =", r2;

  // CancelToken cancelled — produces sentinel.
  let tok2: CancelToken = CancelToken { cancelled: true };
  let r3: i64 = await(cancellable(7, ref tok2));
  print "await cancellable(7, tok)  =", r3;

  return 0;
}
```

## Compile + run

```bash
vanic run ~/adv1.vani
```

Output:

```
await fetch(6) = 42
await cancellable(7, !tok) = 107
await cancellable(7, tok)  = -1
```

## Why it works that way

- **`async fn foo() -> R { ... }`** desugars to
  `fn foo() -> Future<R> { ...; return Future.Ready(v); ... }`.
  In v1 the body runs to completion synchronously on call;
  the suspend-point state machine ships under Arc 8 step 8c.
- **`await(expr)`** desugars to a `match` that extracts
  `Future.Ready`'s payload. The `Pending` arm body is the
  literal `0` because v1 async fns never produce `Pending`.
  This shape lets you write code that *looks* asynchronous
  while the compiler treats it as straight-line.
- **`CancelToken`** is a prelude-defined struct:
  ```rust
  struct CancelToken { cancelled: bool }
  ```
  Thread it through async functions and check `.cancelled` at
  natural breakpoints. Real suspend-point cancellation lands
  when the state-machine codegen ships.
- **The `try` keyword sugar** ([Intermediate §10](../intermediate/10_result_try.md))
  is enabled *inside async fn bodies* in v1 (Arc 8 v3.1 Phase
  2.4). For Result-returning async fns, you can write `let v:
  i64 = try maybe_fetch();` and the compiler inserts the
  short-circuit on `Err`.

## What's coming and what's queued

| Today | Queued |
|---|---|
| `async fn` + `await` synchronous desugar (v1) AND real suspend-point state machine (v3.1, FEATURE-COMPLETE 2026-06-08) | — |
| `CancelToken` cooperative cancellation AND **A4.4** auto-injected cancel guards at every suspend point | — |
| `try EXPR` keyword AND postfix `EXPR?` operator in both sync + async bodies | — |
| `Future<R>` for scalar R AND v3.1 Task<T> for all v3.1-allowed T | — |
| **A4.3** dynamic-N multi-task scheduling via `mut ref pool[i]` over `Vec<Task__<fn>>` | — |
| Per-dialect spellings: `अतुल्यकालिक` / `异步` / `非同期` for `async`; `प्रतीक्षा` / `等候` / `待機` for `await` | — |

## Common patterns

**Sequential awaits**: write them as you would Rust. The
desugar lowers each to a `match`.

```vani
async fn pipeline(n: i64) -> i64 {
  let a: i64 = await(fetch(n));
  let b: i64 = await(fetch(a));
  return b;
}
```

**Conditional await**: standard `if` works inside async
bodies.

```vani
async fn maybe_fetch(use_cache: bool, key: i64) -> i64 {
  if use_cache {
    return key;
  }
  return await(fetch(key));
}
```

## Challenge

Write an `async fn batch(xs: ref Vec<i64>) -> i64` that calls
`fetch` on each element, sums the results, and returns the
total. Add a `CancelToken` parameter; return `0 - 1` immediately
if `cancelled`.

---

**Next**: [§2 — `parallel for` + race-freedom →](02_parallel.md)
