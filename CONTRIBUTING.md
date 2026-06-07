# Contributing

Thanks for poking at this compiler. The fastest way to get oriented
is [ONBOARDING.md](ONBOARDING.md) — it covers the tool prerequisites,
the project layout, the pipeline, and an "end-to-end add a feature"
checklist.

## Before you open a PR

1. **`cargo test`** (full suite — 1916 lib + 54 e2e + 11 vtables-phase3 +
   2 user-drop-by-ref + 1 ssa-examples in ~90s. `.cargo/config.toml`
   sets `RUST_MIN_STACK=32MB` so parallel-for + reduction tests have
   stack headroom.).
2. **`cargo clippy`** (3 known warnings are tolerated;
   anything new should be addressed or justified).
3. **`cargo build`** — clean with no new warnings.
4. **Cross-backend parity:**
   `cargo test llvm_backend_run_produces_same_output_as_c`
   must still pass. New examples are auto-picked-up if you wire
   them through both `check_examples_all_succeed` and a
   `run_<example>_example` test.

## Code conventions

* LLVM is the default backend. New work targets it first.
* **The C backend (`backend_c.rs`) is NOT deprecated anymore** — it
  carries first-class macOS + Windows support via `#ifdef`-branched
  emit ([backend_c.rs:emit_intent_epoll_helpers_c](src/backend_c.rs))
  that the LLVM backend hasn't matched yet (Windows LLVM TCP IR is
  deferred). Treat the two backends as peers; mirror new features
  across both where the platform matters. The module-top deprecation
  comment is stale and queued for removal in [TODO.md](TODO.md).
* The verifier's failure modes are conservative: when SMT
  can't discharge a goal, the elision pass leaves runtime
  guards in place. Sound-then-precise.
* Diagnostics go through `src/diagnostic.rs` so they show up
  in both human and JSON output.
* Tests live next to the feature they cover: unit tests in
  `lib.rs::tests`, integration tests in
  `tests/run_end_to_end.rs`. Examples in `examples/` document
  features by demonstration.

## Commit messages

One-line summary in present tense, then a paragraph if needed.
The body should explain *why*, not duplicate the diff. Examples:

```
Fix Drop-before-Return UAF in checker

The checker emitted Drop statements before Return, but Return's
expression was evaluated lazily by the backend after the drops,
so `return xs[1]` (Vec going out of scope) read freed memory.
Cache the return value into a temp, then drop, then return the
temp. Pre-existed in both backends.
```

## Filing issues

Small repro program > prose. Include the `intentc` command line,
the source file, and the output (stderr inclusive). For verifier
issues, the SMT trace via `INTENTC_SMT_DEBUG=1` is gold.
