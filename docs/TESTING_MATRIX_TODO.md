# Feature x backend testing matrix (2026-08-01)

Working list for a systematic sweep across tutorial feature areas x
backend x codegen path, done to hunt for bugs the existing test
suite doesn't already cover. Companion to `TODO_CURRENT.md` (which
tracks specific diagnosed bugs) -- this file tracks *coverage gaps*,
i.e. combinations nobody has actually exercised end-to-end yet.

## How backend/path selection actually works

There is no user-facing "ssa" flag. `vanic run/build/emit` only
takes `--backend=<c|llvm>`. Within each backend, `main.rs` tries the
SSA emitter first (`emit_c_via_ssa` / `emit_llvm_via_ssa`) and
silently falls back to the tree emitter (`backend_c.rs` /
`backend_llvm.rs`) whenever the program uses a feature the SSA path
doesn't cover (`ssa_path_supports` in `src/main.rs`). So "SSA vs
tree" is not independently selectable -- it's a deterministic
function of *which language features a program uses*, per backend.

Forced to the **tree** backend on **both** backends when a function
uses any of: struct literals/field access, enum variants (payload
or not), tuples, `dyn` dispatch/coercion, `match` or `if` as an
*expression*, block expressions, `[T; N]` in return position,
`#[no_mangle]`, `eprint`, or most of the "container library"
builtins (`vec_map`/`vec_fold`/... , `hashmap_*`/`hashset_*`/
`btreemap_*`/`btreeset_*`, `union_find_*`, `binary_heap_*`,
`bloom_filter_*`, `bst_*`, `graph_*`, `trie_*`, `skiplist_*` -- see
the full list in `src/main.rs`'s `expr_ssa_supported`).

Attempted via **SSA** first on **both** backends: scalars, `Var`/
`Ref`/`RefMut`/`FnRef`, arithmetic, `if`/`while`/`for` as
*statements*, `Vec<T>` of scalars with basic ops, and (as of the
concurrency work) `Atomic`/`Mutex`/`Guard`/`RwLock`/`Channel`/
`parallel for`/single-block `task`+`join`.

Per-backend extra exceptions (forces tree on one backend only):

| Feature | LLVM | C |
|---|---|---|
| `f64_to_str_fixed`, `file_read_line`/`stdin_read_line` | tree | tree |
| `vec_with_capacity` | SSA (works) | tree (not ported) |
| `Vec<Atomic<T>>` / `Vec<Channel<T,N>>` (any nesting) | tree | SSA (works) |

Net effect: most of **intermediate** (structs, enums, dyn, generics
monomorphized into struct/enum shapes, closures with non-trivial
captures) and a good chunk of **advanced** (vtables, most container
builtins) always exercises the **tree** backends on both `--backend`
values -- the SSA emitters mostly only ever run for **beginner**-
level scalar/Vec-arithmetic code and the concurrency primitives.
That means "test both `--backend` values" already covers the
tree-vs-SSA axis for most subsystems; the exceptions above are the
only cells where backend x path actually gives 3+ distinct
codepaths worth testing separately (`vec_with_capacity`,
`Vec<Atomic|Channel>`).

## Existing coverage snapshot (grep heuristic, 2026-08-01)

Rough keyword-hit counts in `src/lib.rs` (in-process compile-only
tests) vs `tests/run_end_to_end.rs` (real `vanic` binary, both
backends, actual stdout/exit-code assertions -- the only tests that
can catch a backend-specific miscompile or runtime crash). Low e2e
counts are the actual gap; lib.rs count alone is not a substitute.

| Area | lib.rs hits | e2e hits | Read as |
|---|---:|---:|---|
| `dyn` dispatch | 74 | **0** | compile-checked only, never run |
| `vtable` (advanced/05_vtables.md) | 4 | **0** | compile-checked only, never run |
| `Channel<T,N>` | 127 | **0** | compile-checked only, never run |
| `fnptr` | 23 | **0** | compile-checked only, never run |
| `vec_map`/`vec_fold`/iterator-style builtins | 73 | **1** | mostly compile-checked only |
| `task_spawn`/`task_join` | 3 | 1 | thin |
| cross-compile / `--target=` | 4 | 1 | thin |
| `extern "C"` / FFI | 110 | 5 | thin relative to lib.rs weight |
| `union_find`/`binary_heap`/`bloom_filter`/`bst_`/`graph_`/`trie_`/`skiplist` | 401 | 8 | thin relative to weight |
| `big_o`/complexity annotations | 69 | 2 | thin |
| `hashmap`/`hashset`/`btree` | 644 | 26 | moderate |
| SIMD / `vec_with_capacity`/`vec_dot` | 217 | 22 | moderate |
| `no_mangle`/`no_std`/embedded | 142 | 16 | moderate |
| affine/`clone_at`/drop | 498 | 86 | good |
| SMT (`ensures`/`requires`/`invariant`/`prove`) | 1125 | 122 | good |
| `try`/propagation | 72 | 72 | good |

## Priority sweep list (this pass)

Ordered by (real-execution gap) x (blast radius if broken). Each
row gets a real `.vani` snippet run through `vanic run <f>` and
`vanic run <f> --backend=c`, output/exit-code compared against hand-
computed expected values, not just "does it compile."

1. `dyn Iface` dispatch, `Vec<dyn Iface>` -- **0 e2e coverage**,
   despite being a documented v1-limitations-prone area
   (intermediate/05_dyn.md, advanced/05_vtables.md).
2. `Channel<T,N>` send/recv, including the documented
   `Vec<Channel<T,N>>` LLVM-forces-tree / C-uses-SSA split above --
   **0 e2e coverage** and the two backends are known to take
   genuinely different codegen paths for it.
3. Function pointers (`fnptr`, `06c_fnptr_primer.md`) as values,
   params, and struct fields -- **0 e2e coverage**.
4. Iterator-style Vec builtins (`vec_map`/`vec_fold`/`vec_filter`/
   `vec_zip_with`/...) chained together -- **1 e2e hit** despite
   heavy tutorial use (06b_iterators_primer.md, 15_math_rng.md).
5. `task`/`join` (concurrency task primitive, not `parallel for`) --
   thin coverage, and it's explicitly called out in `main.rs`
   comments as having SSA-vs-tree edge cases (multi-block bodies).
6. Cross-compile (`--target=`) combined with embedded/`no_std` +
   `#[no_mangle]` FFI export together (04b/04a/04c primers) -- thin,
   and BUG-44 was found exactly in this intersection.
7. Graph/trie/skiplist/union-find/bloom-filter/BST algorithmic
   builtins actually *run* end-to-end (not just compiled) -- thin
   relative to how much tutorial content leans on them
   (advanced/05b_advanced_collections.md).
8. `vec_with_capacity` under `--backend=c` specifically (the one
   documented per-backend SSA gap in the table above) -- confirm
   the tree-C fallback produces correct values, not just that it
   compiles.

## Non-goals for this pass

- Re-testing SMT/`ensures`/`try`/affine -- already well covered
  (see table); revisit only if a sweep item above turns up a
  related regression.
- Async/await -- has a documented, already-known LLVM/Windows gap
  (advanced/01_async.md); not re-litigating that here.
- Exhaustively enumerating every tutorial code sample individually.
  This file tracks *subsystems*, not a line-by-line tutorial replay.

## Process

For each item found broken: diagnose root cause in the compiler (not
the test), fix it, add both a `src/lib.rs` compile-time test and a
`tests/run_end_to_end.rs` real-binary test (both backends), run the
full `cargo test --release --workspace` suite, record the bug in
`docs/TODO_CURRENT.md` with the fix commit, and update any tutorial
page whose text turns out to assume the broken behavior.
