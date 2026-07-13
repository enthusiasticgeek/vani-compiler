# vāṇī (वाणी) — the vāṇी compiler & programming language

<p align="center">
  <img src="vani_logo1.png" alt="vāṇī logo1" width="480">
</p>

<p align="center">
  <a href="https://enthusiasticgeek.github.io/vani-compiler/"><strong>📖 Online Tutorial Book</strong></a>
  &nbsp;•&nbsp;
  <a href="https://github.com/enthusiasticgeek/vani-compiler/releases"><strong>Releases</strong></a>
  &nbsp;•&nbsp;
  <a href="docs/v1_limitations.md"><strong>v1 Limitations</strong></a>
</p>

<p align="center">
  <a href="https://github.com/enthusiasticgeek/vani-compiler/actions/workflows/deploy-tutorials.yml">
    <img src="https://github.com/enthusiasticgeek/vani-compiler/actions/workflows/deploy-tutorials.yml/badge.svg" alt="Deploy Tutorials">
  </a>
</p>

**Verbose Alternative Natural Interface (VANI) — code like you speak.**

Pronounced **vaa-NEE** (Sanskrit *vāṇī* — long-a, retroflex-n, long-i;
stress on the second syllable). वाणी is the Sanskrit word for *speech*,
*voice*, or *language itself*.

> **Naming & disambiguation.** This repository is **the vāṇी compiler**
> (वाणी संकलक, *vāṇī saṃkalaka*) and the implementation of
> **the vāṇी programming language** (वाणी कार्यक्रमण भाषा,
> *vāṇī kāryakramaṇa bhāṣā*). Each component carries a Sanskrit
> qualifier to keep the two distinguishable in writing and search:
> *saṃkalaka* (compiler — literally "assembler / collector") and
> *kāryakramaṇa bhāṣā* (programming language — literally
> "operation-procedure language"). The CLI binary is named
> **`vanic`** — a contraction of *vāṇी* + *saṃkalaka*. Other
> GitHub projects that happen to be called "vani" are unrelated to
> this work.

vāṇī is a working systems language. The default surface is
English-keyword + ASCII identifiers and reads like a tightened
Rust / C++. The compiler also natively understands Devanagari
keywords + identifiers + numerals, with optional Sanskrit / Hindi
/ Marathi SOV (verb-final) statement shapes — that's covered at
the bottom of this file in *Language targeting*. The two surfaces
share an identical AST and runtime model; nothing about the
Devanagari support changes how the English-default experience
works.

---

## Table of Contents

This README is organized in eight parts. Read top to bottom for
a full picture, or jump straight to the part you need.

**Part I — Orientation.** What vāṇी is and why it exists.
  - Philosophy
  - Feature set (closure-by-closure ledger, 2026)

**Part II — Quick Language Tour.** Terse reference for everything
the language can do, before the chapters go deep.
  - Language Snapshot — translation tables vs Rust/C++,
    Devanagari notation, mini-tour by feature

**Part III — Memory Safety, Ownership & Concurrency.** What the
compiler guarantees and what falls inside `unsafe`.
  - Memory safety & concurrency model — compile-time
    guarantees, safety net, smart pointers, cyclic data
    structures, embedded position, examples-the-compiler-
    rejects, known gaps

**Part IV -- Language Reference.** Depth for every primitive and
construct. Use as a lookup, in order.
  - Comments
  - Integer rules
  - Float rules
  - Casts
  - Shift and bitwise rules
  - Numeric literals
  - Arrays and ownership
  - Vectors
  - Strings
  - References
  - Control flow + scoping (includes named loop labels)
  - Print and output (`print`, `print { }`, `eprint`)
  - Mutable references and indexed writes
  - Heap allocation (`Box<T>`)
  - Error handling -- `Option<T>`, `Result<T, E>`, `try`/`?`, `match`, `assert`
  - SMT verification -- `requires` / `ensures` / `assert` /
    `prove` / loop invariants / overflow reasoning / assert
    messages / discard pattern / multi-file projects
  - Modules and namespaces
  - Effects, ownership, and parallelism (includes `Atomic<T>`,
    `Mutex<T>`, `RwLock<T>`, `Barrier`, `Channel<T>`,
    `parallel for`, `task`, function pointers)
  - Async concurrency (`async fn`, `await`, `Future<T>`,
    `Poll<T>`, `CancelToken`, TCP/epoll networking)
  - File I/O (`FileHandle`, `eprint`, `stdin_read_line`)

**Part V — Tooling.** Commands + editor integration + build
pipeline.
  - Commands — build / run / check / emit / fmt / ast / LSP
  - Build pipeline + linking
  - Debug subcommands
  - LSP integration
  - JSON diagnostics

**Part VI — Design Philosophy & Comparisons.** Why the design is
the way it is.
  - Composition over inheritance — `dyn Iface` as escape hatch
  - `try` as a value-flow shortcut, not an exception system
  - Data structures + algorithms — affine-first roadmap
  - Current limitations (cross-link to `docs/v1_limitations.md`)
  - Why Rust (for the compiler core)

**Part VII — Roadmap & Status.** Where the project is and where
it's going.
  - Roadmap — small items, multi-session items

**Part VIII — Community & Reference.** How to participate +
the terminology cheat-sheet.
  - Contributing
  - Language targeting (Indian subcontinent → global)
  - **Glossary** — affine, scrutinee, vtable, BitVec, MMIO,
    fat pointer, scope-escape, mangling, lambda lift, … (~60
    terms across ownership, types, pattern matching, compiler
    pipeline, SMT, async, memory).
  - License + trademark

> **Tutorial book** — rendered and searchable at
> **<https://enthusiasticgeek.github.io/vani-compiler/>**
> (auto-deployed via GitHub Actions on every push to `main` that
> touches `tutorials/`). Source lives under [`tutorials/`](tutorials/):
> 20+ intuition primers + the formal Beginner / Intermediate / Advanced
> tracks (15 Beginner + 15 Intermediate + 10 Advanced lessons).
> Limitations are catalogued at [`docs/v1_limitations.md`](docs/v1_limitations.md).
> The closure-by-closure ledger lives at [`STATUS.md`](STATUS.md).
> The active task queue is [`TODO.md`](TODO.md).

---

# Part I — Orientation

## Philosophy

vāṇī is a small systems language **inspired by Rust and C/C++** in
its semantic model — static types, affine ownership, references with
explicit `mut` / `ref` discipline, compile-time monomorphization,
direct LLVM / C code generation, and predictable cost — but with a
surface that **reads as close to natural language as a strict compiler
will let it**.

*Familiar terrain, lighter outerwear.* If you've programmed in **C**,
the route through these primitives should feel **C-scenic** — the same
close-to-the-metal view, the same predictable cost, with the guardrails
that you used to keep in your head now kept by the compiler on your
behalf. If you're at home in **Rust**, the model here is more **Rust-ic**
than a re-invention — the same affine ownership, second-class
references, monomorphized generics, and deterministic Drop, dressed in
softer punctuation. (These comparisons are descriptive — vāṇī is an
independent project with no affiliation; see *Trademark* at the bottom
of this file.)

The goal is to let users *express the same program* in whichever
spelling reads most naturally to them, without weakening the
language's correctness or performance guarantees. Three concrete
commitments make that work:

1. **Same execution model as Rust / C / C++.** The output is
   fully deterministic. The same source compiles to the same LLVM IR
   / C, with the same runtime behavior on a given target, every time.
   No interpreter, no garbage collector, no surprise allocator, no
   hidden control flow. `prove` / `ensures` / `requires` constraints
   are discharged at compile time by Z3-backed SMT; runtime cost is
   what you'd get in idiomatic Rust.
2. **Multiple keywords + aliases let the writer choose tone.** Most
   constructs accept more than one spelling. `let` and `assign` both
   declare a binding. `return`, `give`, `give_back`, and the two-word
   `give back` are all the same. `pub` and `public` are interchangeable.
   `module` accepts `mod`. **Devanagari surface (Phase 1)** further
   aliases the same tokens to Sanskrit (`कार्य`, `पुनरागम`, `माना`, …),
   Hindi (`फ़ंक्शन`, `लौटाओ`, …), and Marathi (`परत`, …) so a program
   can read like Indo-Aryan prose without the lexer caring which
   form was used. Per-file language purity (closure #237) lets a
   project opt into a single language and have the checker reject
   out-of-language identifiers.
3. **Keywords replace punctuation where it matters most.** Where Rust
   reaches for `&`, `&mut`, `::`, `?`, `<T>`, `'a`, vāṇī uses the words
   you'd say out loud — `ref`, `mut ref`, `module foo::bar`, `try`,
   `<T>` (kept for generics), and `where T is Trait`. The result reads
   left-to-right at speaking pace without losing the strictness of the
   Rust semantic model.

The compiler core is in Rust. Python is fine for experiments, AI
orchestration, and testing, but Rust is the better default for a
compiler that must be fast, memory-safe, deterministic, and close to
ABI / native code generation.

## Use Cases

vāṇī targets any domain where **safety, verifiability, and predictable
cost** matter more than runtime dynamism. The ten primary application
areas below are ordered from most to least proven at this compiler
revision.

### 1. Systems Programming & OS Kernels
Affine ownership eliminates double-free and use-after-free at compile
time. Direct LLVM / C codegen produces deterministic, GC-free output
suitable for kernel modules, bootloaders, and embedded operating
systems. The Arc 8 async state machine compiles to a zero-allocation
poll loop compatible with bare-metal schedulers.

### 2. Embedded & Bare-Metal Systems
The four-layer unsafe model (L1–L4) plus `requires` / `ensures`
annotations maps directly onto MISRA C 2012, ISO 26262 ASIL-D,
DO-178C (DAL A), and IEC 62304 Class C requirements. C backend output
is readable, portable, and linkable against an existing embedded BSP
without a Rust toolchain on the target.

### 3. Formal Verification & Proof-Assisted Programming
Z3 SMT integration discharges `requires`, `ensures`, `prove`, and
loop `invariant` clauses at compile time. The three-stage pipeline
(constant-fold → structural tautology → full Z3 solve) makes most
arithmetic contracts free; Z3 is only invoked when the simpler passes
cannot decide. Suitable for financial arithmetic, cryptographic
protocol correctness, and safety-interlock logic.

### 4. Concurrent & Parallel Systems
The effects checker statically verifies race-freedom in `parallel for`
reductions and rejects impure closures at the call site. Task
handles are affine — forgetting to `join` a task is a compile error,
not a runtime thread leak. Mutex / Guard RAII and Channel queues cover
the classic producer-consumer patterns.

### 5. Networking & I/O-Bound Services
Arc 8 async/await compiles to cooperative state machines with
epoll (Linux), kqueue (macOS), and IOCP (Windows) backends. CancelToken
auto-plumbing provides graceful shutdown without manual flag threading.
TCP echo, connection-pool, and multi-client examples ship in
`examples/language/english/`.

### 6. High-Performance Data Processing
Monomorphized generics, a hand-rolled standard library (Vec,
HashMap, BTreeMap, BinaryHeap, Graph, SkipList, Union-Find), and
verified parallel reductions (+, *, min, max, &&, ||) enable
pipeline-style batch processing with no GC pause jitter.

### 7. Real-Time & Safety-Critical Control Systems
Deterministic drop order, no allocator surprises, and SMT-verified
loop bounds make vāṇī suitable for PLC-like control loops, motor
controllers, and avionics flight software where timing jitter and
memory corruption are unacceptable.

### 8. Multilingual & Localised Software
62 dialects across 26 scripts (Devanagari, Bengali, Tamil, Arabic,
Japanese, Mandarin, and more) let teams write source that reads in
their native language. Per-file dialect purity rejects out-of-language
identifiers, keeping a codebase coherent across multilingual
contributors. Particularly suited to educational software and
government/public-sector tooling targeting the Indian subcontinent.

### 9. FFI & C Interoperability
Full SysV ABI / Win64 / AArch64 struct-return lowering (Arc 7) lets
vāṇī modules be called from C or call into libc, OpenSSL, SQLite, and
similar libraries. `extern "C"` declarations and `--link-with` handle
the link step; the C backend produces `.c` suitable for integration
into legacy build systems without LLVM on the host.

### 10. Data Structures & Algorithm Libraries
The standard library ships affine-first containers across four
complexity tiers. All containers are composable (`Vec<Box<dyn Iface>>`,
`HashMap<OwnedStr, Vec<T>>`, etc.) and drop correctly under the affine
ownership model. Suitable for competitive programming scaffolds,
reference implementations, and algorithm correctness benchmarks backed
by Z3 proofs.

---

## Feature set (closures #1–#604)

vāṇī today is a working systems language with the following shipped
features. Surface that **reads natural-language** sits on top of a
semantic model **borrowed from Rust** and a code-generator that
**emits LLVM IR or C** with no runtime layer in between.

**Type system + memory:**
- Scalars (`i8`–`i64`, `u8`–`u64`, `f32`/`f64`, `bool`); fixed-size
  arrays `[T; N]`; heap `Vec<T>`; tuples (2–4 elements); structs (up
  to 64 fields); enums (with payloaded variants); type aliases;
  `const` bindings with literal initializers.
- `Str` (borrowed string) and `OwnedStr` (heap, affine, produced by
  `+` concat).
- **Affine ownership.** `Vec`, `OwnedStr`, `Atomic`, `Mutex`, `Guard`,
  `Channel`, `Task` are all single-owner; the checker tracks moves +
  partial moves and the backends emit deterministic destructors at
  scope exit. User-defined `Drop` interface lets a struct hook into
  the scope-exit flow.
- **References** are second-class keyword-first: `ref T` / `mut ref T`
  in parameter position, `let` bindings, and user struct fields, with
  `ref x` / `mut ref x` at call sites, aliasing rejected at compile
  time, and a scope-escape analyzer that rejects every shape that
  would let the borrow outlive its source.

**Generics + dispatch:**
- **Monomorphized generics** (`fn id<T>(x: T) -> T`) — specialized per
  call-site concrete type.
- **Interfaces** (`interface Show { … }` + `implement Show for T { … }`)
  with **static dispatch** by default and **dynamic dispatch via
  `dyn Iface`** (16-byte fat pointer) for heterogeneous collections.
  Bounded generics: `fn min<T>(a: T, b: T) -> T where T is Cmp`.

**Control flow + verification:**
- `if`/`else`/`else if`, `while`, `for i from lo to hi`, `for x in xs`,
  `break` / `continue`, `match` with payloaded-variant destructure,
  `try EXPR` keyword AND postfix `EXPR?` operator (parse-time
  sugar over the same AST node) for early-return on
  Option/Result-like enums.
- **SMT-discharged** `requires` / `ensures` / `assert` / `prove` /
  `invariant` via Z3. Bounds / divisor / shift / overflow checks
  elided when proven safe.

**Parallelism + concurrency:**
- `parallel for` with verified race-freedom + reductions
  (`reduce x with +`, `*`, `&&`, `||`, `&` / `|` / `^`, `min`, `max`).
- `task <name> { … }` / `join <name>;` with real pthread (Linux)
  and CreateThread (Windows) backing. `Atomic<T>` for shared
  counters, `Mutex<T>` + `Guard<T>` for critical sections (parametric over any element type T since v0.1.1),
  `Channel<T, N>` for queues (struct/enum element types since v0.1.1).
- `Condvar` — `condvar_new` / `condvar_wait(ref cv, mut ref g) /
  condvar_wait_timeout / notify_one / notify_all`. Pairs with
  `Mutex` + `Guard` for "wait until predicate" patterns. ✅
  AFFINE (closure #292). Tree-C + SSA-C use shared runtime
  helpers (futex/WaitOnAddress/spin-yield); tree-LLVM uses
  inline IR; SSA-LLVM falls back to tree-LLVM. See
  [examples/condvar.vani](examples/condvar.vani).
- `Barrier` — N-thread rendezvous (`barrier_new(n)` /
  `barrier_wait(mut ref b) -> bool`). Stack-by-value, affine.
  Generation counter prevents ABA races. Last thread to arrive
  returns `true`. Both C and LLVM backends.
- `RwLock<T>` / `ReadGuard<T>` / `WriteGuard<T>` — readers-writer
  lock parametric over any element type T. `rwlock_read` acquires
  a shared guard; `rwlock_write` acquires an exclusive guard. RAII
  Drop releases the lock. State: 0=unlocked, N>0=N readers, -1=write-locked.
  Both C and LLVM backends. Blanket impl + default methods in
  interfaces also ship in v0.1.1 (Traits phase 2).

**Namespaces + modules:**
- `module foo { … }` (inline + nested + deep `a::b::c::Item` paths).
- Per-item `pub` and `pub(kosh)` visibility.
- `use foo::bar [as baz];`, `use foo::{a, b};`, `use foo::*;` (direct
  children only), `pub use foo::bar;` re-exports (transitively
  resolved), module-local `use` inside `module { }` bodies, orphan
  rules on `implement Iface for T`, collision diagnostics with
  precise `use … as …;` hints.

**Kosh package manager** (shipped 2026-06-17):
- `vani.toml` manifest — `[package]` (name, version, entry) + `[deps]`
  with path deps and semver constraints (`^1.0`, `~1.2`, `>=1.0`).
- `vani.lock` — Cargo.lock-style lockfile, auto-written when manifest
  is newer than lock.
- `vanic vendor` — copies dep source trees into `vendor/<name>/`.
- `vanic add <name>[@constraint]` — fetches from the Kosh registry,
  verifies SHA-256 checksum, extracts to `vendor/<name>/`, updates
  `vani.toml` and `vani.lock`.
- `vanic remove <name>` — removes dep from manifest, deletes `vendor/<name>/`,
  rewrites `vani.lock`.
- `vanic search [<query>]` — list registry packages (optionally by name
  substring).
- `vanic update` — re-resolve all registry deps to latest compatible version
  with checksum verification.
- `vanic apply-publisher` — fetches and displays the Publisher Agreement.
  `vanic apply-publisher --accept-agreement` submits a publisher application
  as a GitHub issue in the registry repo.
- `vanic publish` — builds tarball, auth-gates against `governance.json`
  (`allowed_publishers` / `pending_publishers` / `blacklisted`), creates a
  GitHub Release, and appends an NDJSON line to the sparse index.
- `vanic registry-approve <user>` / `vanic registry-blacklist <user> --reason=…`
  — operator-only commands to approve or blacklist publishers.
- Live registry: **[enthusiasticgeek.github.io/kosh-index](https://enthusiasticgeek.github.io/kosh-index/)**
- Governance model is registry-side: `governance.json` holds allowlist,
  pending list, blacklist, and agreement version — transfers to a committee
  without any compiler change.

**Backends + tooling:**
- LLVM IR (default) — tree path + SSA path with automatic fallback.
- C — same dual-path arrangement.
- `vanic check` (typecheck + SMT), `emit` (lowered source), `run`
  (compile + execute), `build` (AOT to native binary), `fmt`
  (formatter with round-trip + comment preservation), `ast` (AST
  dump), `vendor` / `add` / `publish` (package manager), LSP
  integration. *(Legacy alias `intentc` kept for one release cycle.)*
- Cross-backend parity test pins identical stdout + exit code on
  every example under both backends.

**Multi-file projects:**
- `use "path.vani";` for file-level inclusion; cycle detection;
  diagnostics resolve to the original file:line via a `FileMap`.

**Since closure #258 (2026-05-26 → 2026-05-27):**
- **FFI v1–v8** — `extern "C" fn` declarations (#269), `--link-with`
  linker flag (#270), extern call-site checker (#271), extern codegen
  with mangled symbols (#272), struct-by-value rejection with `ref T`
  hint (#273), linker-discovery polish (#274), FFI callbacks via
  `Type::FnPtr` (#279), System V x86-64 small-struct return lowering
  for FFI (#288). Net: `qsort`-style callbacks and libc string /
  math interop work end-to-end without a runtime shim.
- **vani.toml manifest** — hand-rolled minimal-TOML parser
  (`src/manifest.rs`), `find_manifest` parent-walk, `[package].entry`
  auto-discovery in `intentc build|run|check` (#280); v2 added
  `[deps]` inline-table for multi-file dependency wiring (#287).
- **Generic struct + enum declarations** — `enum Result<T, E> { Ok(T),
  Err(E) }` / `struct Pair<A, B> { … }` (#281). `Type::Apply { name,
  args }` for parse-time generic instantiations; mangled names like
  `Result__Vec_I64___AllocError` flow through the monomorphizer.
- **Mixed-payload enums** — variants with different payload types
  share one enum on both backends (#283). C uses a tagged union
  (`union { Type0 v_Ok; Type1 v_Err; }`); LLVM uses `[N x i8]` byte
  buffer + per-variant bitcast.
- **`try_vec(n) -> Result<Vec<i64>, AllocError>`** — fallible
  allocation builtin emitting malloc + null-check + Result
  construction (#284). Programs handle OOM gracefully.
- **Attribute syntax + `#[bounded(N)]`** — first attribute in the
  language (#286). New `#` token + parser. Tree-LLVM uses
  thread-local globals + per-Return decrement (#289); SSA-LLVM
  mirrors the pattern (#290). C emits a thread-local counter with
  GCC `__attribute__((cleanup))` for the decrement.
- **Nested arrays `[[T; N]; M]` / `[Vec<T>; N]`** — array-element
  Copy restriction lifted, `clone_at(ref arr, i)` extended to arrays,
  per-slot per-field drops including struct fields (#291 Phases 1–4).
  Tree-LLVM `len` of a Vec rvalue (e.g. `len(clone_at(ref xs, i))`)
  now spills to alloca, GEPs `.len`, loads.
- **Prelude injection** — `Option<T>`, `Result<T, E>`, `AllocError`
  injected at AST level (NOT source-prepend, which would shift
  diagnostic line numbers) so user programs use them without `use`
  (#282).
- **Match on `f64` / `f32` scrutinees** — `Pattern::Float(f64)` AST
  variant + `check_match_float` desugar to nested IfExpr; clear
  diagnostics for missing wildcard, duplicate literals, NaN-in-pattern
  (#278).
- **Other closures #275–#277** — parallel-for purity hole closed in
  reduction RHS; DynCoerce non-Var hoist via synthetic Block expr;
  `let _ = make()` discard of fresh struct value frees heap fields.

**Since closure #292 — Data-structures + algorithms roadmap (Levels 1–4):**
Sustained run that takes the language from "scalars + Vec + Channel"
to "carry-your-own data-structures library, all affine, all sharing the
same Drop and codegen disciplines." Each Level's full detail lives in
the *Data structures + algorithms* table further down; the headline
deltas are:
- **Level 1 — operations on existing primitives:**
  `Vec.sort` / `sort_by(cmp_fn)` (#293) · `Vec.reverse` / `dedup` (#294)
  · `Vec.find` / `contains` / `binary_search` (#295) · `Vec.swap_remove`
  / `insert` / `clear` (#296) · `[i64; N]` Array sort / find / contains
  / binary_search (#297) · `str_contains` / `str_starts_with` /
  `str_ends_with` / `parse_int` / `parse_float` (#298) + heap-
  allocating `str_trim` (#348) / `str_replace` (#349) ->
  `OwnedStr`, `str_split` (#350) -> `Vec<OwnedStr>` · math
  (`pow` / `sqrt` / `sin` / `cos` / `tan` / `floor` / `ceil` / overloaded
  `abs`) (#299) · RNG (`seed_rng` / `rand_i64` / `rand_in_range`,
  thread-local xorshift64) (#300) · FNV-1a hash (`hash_i64` / `hash_f64`
  / `hash_str` / `hash_combine`) (#301 + `hash_f64` in #347) +
  adversarial-resistant SipHash-2-4 (`siphash_i64` / `siphash_str`,
  keyed with `(k0, k1)`, spec-vector parity verified) (#351).
- **Level 2 — generic containers** (all i64 in v1): BinaryHeap on
  `Vec<i64>` (#302) · `Deque<i64>` ring buffer w/ 8 builtins (#303) ·
  `HashSet<i64>` open-addressing w/ tombstone-aware remove (#304 +
  #342) · `HashMap<i64, i64>` open-addressing w/ tombstone-aware
  remove (#305 + #343) · `BTreeSet<i64>` on sorted Vec w/ range
  query (#306 + range in #346) · `BTreeMap<i64, i64>` on parallel
  sorted Vecs w/ parallel range_keys / range_values (#307 + range
  queries in #346).
- **Level 3 — closures + iterators:** anonymous fn expressions
  (`fn(x: T) -> R { … }`) lambda-lifted to top-level `__anon_fn_<N>`
  (#308) · eager `vec_map` / `vec_fold` / `vec_filter` on `Vec<i64>`
  (#309 + #310; extended to `Vec<f64>` in F64-3) · method-call sugar across `Vec`, `HashMap`,
  `HashSet`, `BTreeMap`, `BTreeSet`, `Deque` (#311 + #312) ·
  `vec_take` / `vec_drop` + uniform `xs.len()` (#313) · closures
  with captured Copy state (#314), declarable inside `if`/`while`/
  `for` bodies (#315) · fused single-pass family `vec_map_fold` /
  `vec_filter_fold` / `vec_map_filter` / `vec_map_filter_fold`
  (#316 + #317) · auto-fusion of `vec_map + vec_fold` chains (#318) ·
  Vec mutator + search method sugar (`xs.push` / `xs.pop` /
  `xs.reverse` / `xs.dedup` / `xs.find` / `xs.contains` /
  `xs.binary_search` / `xs.swap_remove` / `xs.insert` / `xs.clear`)
  (#320) · `[T; N]` Array sugar (`arr.sort` / `sort_by` / `reverse`
  / `find` / `contains` / `binary_search`) (#321).
- **Level 4 — advanced / domain-specific:** `UnionFind` w/ path
  compression + union-by-rank (#325) · dedicated `BinaryHeap<T>`
  affine handle (#326) · `BloomFilter` (#327) · `Bst<T>` on a node
  arena, upgraded to **AVL self-balancing** (#328 + #332) · weighted
  directed `Graph` w/ lazy CSR adjacency cache (#329 + #336),
  algorithms BFS / DFS / Dijkstra / A* / topo-sort / Kruskal / Prim
  via reverse-CSR cache (#333 → #338) · `Trie` prefix tree with
  exact-word `delete` (#340), **arena compaction via freelist** so
  remove-heavy workloads reclaim slots (#344), and **full u8
  alphabet** — any nonzero byte is a valid character (#345) ·
  `SkipList<T>` MAX_LEVEL=8 w/ `remove` + O(1) `max` via maintained
  `tail_node` (#331 + #339 + #341).

Each shipped closure exits with both backends byte-identical, the
cross-backend parity runner green across every example in
`examples/`, and at least one lib test pinning the new helper name
or struct shape. See [STATUS.md](STATUS.md) for the closure-by-closure
history and [TODO.md](TODO.md) for what's queued. The full Roadmap
(small + multi-session items) is in the README's *Roadmap* section
below.

**Major feature arcs (shipped):**

- **Arc 4 -- full HashMap K-V matrix** -- all six K/V combinations (`OwnedStr`, `i64`, `f64`, `Tuple`, `Vec<i64>`) cross-backend. FNV-1a hashing, per-slot drop, clone-on-insert.
- **Arc 5c -- closure-as-value across fn boundaries** -- `Type::Closure(Args, Ret)`; closures pass as fn args / return values / struct fields. See [examples/closure_as_value.vani](examples/closure_as_value.vani).
- **Arc 7 SysV -- full float-class + mixed int/float <= 16-byte struct FFI** -- completes the SysV ABI classifier; Win64 / AArch64 gated on cross-platform CI.
- **Arc 8 -- async + networking + concurrency + state machines end-to-end** -- `async fn` / `await` / `Future<T>` / `Poll<T>` / `CancelToken`; blocking TCP, epoll non-blocking I/O, compiler-driven state-machine transform (postfix `?`, multi-task scheduling, generic async fns); Linux / macOS / Windows. See [ARC8_V3_PLAN.md](ARC8_V3_PLAN.md) and the examples under `examples/`.
- **Arc 9 -- Kosh package manager** -- `vani.toml` manifest + `vani.lock`; `vanic add / vendor / publish`; live sparse registry at [enthusiasticgeek.github.io/kosh-index](https://enthusiasticgeek.github.io/kosh-index/). See [docs/kosh_design.md](docs/kosh_design.md).

> Per-release changelogs live in [`RELEASE_NOTES/`](RELEASE_NOTES/).
> The closure-by-closure history and current version are in [`STATUS.md`](STATUS.md).



---

# Part II — Quick Language Tour

## Language Snapshot

```intent
intent "Compute a value with checked constraints";

fn add(a: i64, b: i64) -> i64 {
  return a + b;
}

fn main() -> i64 {
  let answer = add(40, 2);
  prove 2 + 2 == 4;
  assert answer >= 0;
  print answer;
  return 0;
}
```

Read it aloud: *"function add takes a and b of type int-64, returns int-64;
return a + b."* The source reads left-to-right at speaking pace.

### Translation from Rust / C++ punctuation

vāṇī keeps the **semantic model** of Rust / C++ (static types, affine
ownership, monomorphized generics, references with explicit `mut`)
but replaces the punctuation soup with keywords. Most of the column
on the left will compile on the right with identical generated code:

| Rust / C++ | vāṇī | Notes |
|---|---|---|
| `&xs` (shared borrow) | `ref xs` | second-class; param + `let` + struct-field + `Vec<ref T>` element positions, scope-escape checked |
| `&mut xs` (mut borrow) | `mut ref xs` | same semantics |
| `fn(&self)` | `fn name(self: ref Type)` | receiver is explicit |
| `Vec::with_capacity(n)` | `vec_with_capacity(n)` | free function — no path |
| `impl Drop for T` | `implement Drop for T` | auto-called at scope exit |
| `match Some(x) => …` | `match Opt.Some(x) then …` | `then` instead of `=>` |
| `xs?` (try operator) | `try expr` *or* `expr?` | both spellings share one AST node |
| `loop { … }` | `while true { … }` | one looping construct |
| `for x in &xs` | `for x in ref xs` | borrow at the loop header |
| `mod foo { … }` | `module foo { … }` | `mod` accepted as alias |
| `pub(crate) fn …` | `pub(kosh) fn …` | कोश = "treasure / repository" |
| `pub use foo::bar;` | `pub use foo::bar;` | re-exports through current module |
| `use foo::*;` (glob) | `use foo::*;` | direct children only, non-transitive |
| `let x = …` | `let x = …` *or* `assign x = …` | aliases pick tone |
| `return x` | `return x` / `give x` / `give_back x` / `give back x` | all canonical |

The compiler never silently changes the meaning of source. Aliasing,
ownership transfer, and pure-vs-effectful boundaries are all visible
in the words on screen — surface aliases never relax a check.

### Deterministic output, multiple ways to spell it

Every alias resolves to the same `TokenKind` at the lexer boundary,
so the AST is identical regardless of which spelling the user picked.
The checker, SMT layer, SSA pass, and backends all see the same IR.
Two source files that differ only in alias choice produce
**byte-identical LLVM IR / C** (after `intentc fmt` re-emits to a
canonical form). The same program in English vs Hindi vs Sanskrit
runs the same instructions on the same target.

### वाणी (*vāṇī*) — Devanagari + the 62-dialect family

> **⚠️ Caveat — natural language support is provisional.**
> The vāṇी authors read and write **English** fluently and have
> **first-hand familiarity with the Devanagari Indo-Aryan family**
> (Sanskrit / Hindi / Marathi as primary, Nepali / Maithili /
> Konkani as close relatives). Every other dialect's keyword
> table — Bengali, Tamil, Telugu, Gujarati, Punjabi, Kannada,
> Malayalam, Odia, Assamese, Sinhala, Urdu, Sindhi, Persian,
> Pashto, Mandarin Chinese, Japanese, Korean, Arabic, Hebrew,
> Greek, Russian, Thai, Khmer, Burmese, Amharic, Tibetan,
> Mongolian, Armenian, Georgian, Cherokee, Lao, Spanish, French,
> German, Italian, Portuguese, Polish, Turkish, Vietnamese,
> Romanian, Dutch, Hungarian, Czech, Slovak, Swedish, Norwegian,
> Danish, Finnish, Catalan, Yoruba, Hausa, Swahili, Indonesian,
> Malay, Filipino, Khmer (and more) — was drafted from
> reference grammars, tatsama / loan-word patterns, and CS-
> vocabulary conventions, **but has NOT been validated by a
> native speaker**. The chosen verbs and idioms may sound
> wrong, formal, or archaic to fluent users. The lexer +
> parser pipeline is correct; the *vocabulary curation* is
> what needs review.
>
> **A grammar-consultant pass — native-speaker review across
> the shipped dialects — is queued as an ongoing external item
> in [TODO.md](TODO.md)**. If you read any of these languages
> natively and find a keyword that's wrong, please open an
> issue or PR; the lexer table is one file and the change is
> mechanical. Treat the non-Devanagari-Indo-Aryan dialects as
> *technical proofs-of-concept* until that pass lands.

Devanagari notation lets the source read in the writer's mother tongue.
The first three languages are **Sanskrit** (*saṁskṛta* — the canonical
Devanagari language and grammar root), **Hindi** (*hindī*), and **Marathi**
(*marāṭhī*). They share the script but use slightly different verbs for
the common keywords. The idea is **alias-based**: every English keyword
gets one or more Devanagari aliases, and the lexer accepts whichever form
the source file uses. A single program may mix forms freely; the compiler
treats them as the same token.

**Phase 1** (closures #235–#237) shipped single-word Devanagari
aliases for the core control / declaration keywords plus multi-word
phrases like `नहीं तो` (else), `के लिए` (for), `सिद्ध करो` (prove) —
fused by a post-lex merger. Per-file script purity (English vs
Devanagari) is enforced automatically: the first structure keyword
sets the script, and the lexer rejects mixing thereafter
([lexer.rs:393–441](src/lexer.rs#L393-L441)). No header opt-in
required.

**Phase 2** (closures #265–#267) added two ergonomic features:

1. **SOV word order (partial).** Indo-Aryan grammar is verb-final
   (postpositions follow the noun). The parser accepts the
   natural shape `i के लिए 0 से 5 तक { … }` (range for) and
   `X पुनरागम;` / `"x =", x लिखो;` / `cond सुनिश्चित;` /
   `expr प्रमाण;` (return / print / assert / prove with the
   verb at the end). The English keyword-first order still works
   — the SOV detector only fires when the leading token isn't
   a verb-keyword.
2. **3-way alias parity.** Sanskrit / Hindi / Marathi each have a
   viable form for ~41 of 45 structure keywords (else `वरना`,
   mut `परिवर्तनीय`, continue `अग्रे`, pub `सार्वजनिक`, module
   `खण्ड` / `मॉड्यूल`, use `उपयोग`, as `यथा`, where `यत्र` /
   `जहाँ` / `जिथे`, is `अस्ति` / `है` / `आहे`, plus interface
   / implement / methods / try / task / join / parallel single-
   word). Sanskrit-root words that work as tatsama (loanwords)
   in Hindi + Marathi are documented as shared across the three.

> **What "partial" actually means (honest status 2026-06-06)**:
> SOV is wired for **range `for` + four verb-at-end statements**
> (`return` / `print` / `assert` / `prove`). Most constructs —
> `let`, `fn`, `struct`, `enum`, `if`, `while`, `match`, top-level
> declarations — **still require keyword-first syntax even in
> Devanagari mode**. A "code as you speak" experience for full
> programs (verb-final everywhere, postpositions everywhere) is the
> next milestone, not a shipped one. See [TODO.md](TODO.md)
> §*Sanskrit-derived SOV completion*.

**Still queued**:
- Full SOV coverage across the remaining ~10 statement categories
  (`let`-binding verb-at-end, `if`/`while` cond-at-end with
  postposition, `fn` declaration with verb-at-end signature, struct
  / enum decl SOV-shape, match-arm SOV shape).
- Four English-only keywords gain Devanagari aliases: `extern`,
  `type`, `intent`, `invariant`.
- Finer-grained Sanskrit-vs-Hindi-vs-Marathi purity gate (today
  it's only English-vs-Devanagari at the script level).
- Grammar-consultant refinement pass — Phase-2 picks are best-
  effort and welcome dialect-specific revision.
- **Cross-language `.vani` source translator** (planned tool):
  one-shot rewrite of a program's keywords between English /
  Sanskrit / Hindi / Marathi so a user can read someone else's
  source in their preferred dialect without losing semantics.
- **Examples reorganization**: each script gets its own subfolder
  under `examples/language/` (`english/`, `sanskrit/`, `hindi/`,
  `marathi/`) and every Devanagari example begins with a
  `श्री।` invocation comment (pūrṇa daṇḍa terminator) per the
  classical convention.

Romanizations follow **IAST** (International Alphabet of Sanskrit
Transliteration) for Sanskrit and a Hunterian-style transliteration for
Hindi / Marathi where IAST conventions diverge from spoken pronunciation
(e.g. word-final `अ` is dropped in Hindi/Marathi but retained in
Sanskrit). Where a vowel has both forms, the spoken form is shown.

Conceptual sketch of what the same program might look like in each:

```intent
// English
fn add(a: i64, b: i64) -> i64 { return a + b; }

// संस्कृत (saṁskṛta — Sanskrit): verbs from classical Sanskrit grammar
कार्य add(a: i64, b: i64) -> i64 { पुनरागम a + b; }
// kārya add(a: i64, b: i64) -> i64 { punarāgama a + b; }

// हिन्दी (hindī — Hindi): common spoken Hindi verbs
फलन add(a: i64, b: i64) -> i64 { लौटाओ a + b; }
// phalan add(a: i64, b: i64) -> i64 { lauṭāo a + b; }

// मराठी (marāṭhī — Marathi): Marathi verbs
कार्य add(a: i64, b: i64) -> i64 { परत a + b; }
// kārya add(a: i64, b: i64) -> i64 { parat a + b; }
```

The complete alias table below gives **every English keyword** in
its Devanagari spelling + romanization for each of the three
shipped Indo-Aryan dialects. **100% coverage**: 46 of 46 structure
keywords have at least one Devanagari alias.

> **Reading order**: Romanizations follow IAST (International
> Alphabet of Sanskrit Transliteration). Read each cell aloud —
> that's the pronunciation contract.

### Declarations + visibility

| English | संस्कृत (Sanskrit) | हिन्दी (Hindi) | मराठी (Marathi) |
|---|---|---|---|
| `fn` | `कार्य` *kārya* | `फलन` *phalan* | `कार्य` *kārya* |
| `let` / `assign` | `माना` *mānā* | `माना` *mānā* | `मान` *māna* |
| `struct` | `संरचना` *saṁracanā* | `संरचना` *saṁracanā* | `संरचना` *saṁracanā* |
| `enum` | `विकल्प` *vikalpa* | `गणन` *gaṇan* | `गणन` *gaṇan* |
| `const` | `स्थिर` *sthira* | `स्थिर` *sthira* | `स्थिर` *sthira* |
| `type` | `प्रकार` *prakāra* | `प्रकार` *prakāra* | `प्रकार` *prakāra* |
| `intent` | `उद्देश्य` *uddeśya* | `उद्देश्य` *uddeśya* | `उद्देश्य` *uddeśya* |
| `extern` | `बाह्य` *bāhya* | `बाह्य` *bāhya* | `बाह्य` *bāhya* |
| `pub` / `public` | `सार्वजनिक` *sārvajanik* | `सार्वजनिक` *sārvajanik* | `सार्वजनिक` *sārvajanik* |
| `module` / `mod` | `खण्ड` *khaṇḍa* | `मॉड्यूल` *mōḍyūla* | `मॉड्यूल` *mōḍyūla* |
| `use` | `उपयोग` *upayog* | `उपयोग` *upayog* | `उपयोग` *upayog* |
| `as` | `यथा` *yathā* | `यथा` *yathā* | `यथा` *yathā* |

### Control flow

| English | संस्कृत (Sanskrit) | हिन्दी (Hindi) | मराठी (Marathi) |
|---|---|---|---|
| `return` / `give` / `give_back` | `पुनरागम` *punarāgama* | `लौटाओ` *lauṭāo* | `परत` *parat* |
| `if` | `यदि` *yadi* | `अगर` *agar* | `जर` *jar* |
| `else` | `अन्यथा` *anyathā* | `वरना` *varnā* / `नहीं तो` *nahīṁ to* | `नाहीतर` *nāhītar* |
| `while` | `यावत्` *yāvat* | `जबतक` *jab tak* | `जोपर्यंत` *jopa­ryanta* |
| `for` | `प्रति` *prati* | `के लिए` *ke liye* | `साठी` *sāṭhī* |
| `in` | `में` *meṁ* | `में` *meṁ* | `में` *meṁ* |
| `from` | `से` *se* | `से` *se* | `से` *se* |
| `to` | `तक` *tak* | `तक` *tak* | `तक` *tak* |
| `break` | `विराम` *virāma* | `रुको` *ruko* | `थांब` *thāmba* |
| `continue` | `अग्रे` *agre* | `आगे` *āge* | `पुढे` *puḍhe* |
| `then` | `तदा` *tadā* | `तो` *to* | `तर` *tar* |
| `match` | `मेल` *mela* | `मिलान` *milān* | `जुळवा` *juḷvā* |

### References + mutation

| English | संस्कृत (Sanskrit) | हिन्दी (Hindi) | मराठी (Marathi) |
|---|---|---|---|
| `ref` | `दृष्ट्या` *dṛṣṭyā* | `देखो` *dekho* | `पहा` *pahā* |
| `mut` | `परिवर्तनीय` *parivartanīya* | `परिवर्तनीय` *parivartanīya* | `बदल` *badla* |

### Verification (SMT-discharged)

| English | संस्कृत (Sanskrit) | हिन्दी (Hindi) | मराठी (Marathi) |
|---|---|---|---|
| `assert` | `सिद्धम्` *siddham* | `सुनिश्चित` *sunishchit* | `खात्री` *khātrī* |
| `prove` | `प्रमाण` *pramāṇa* / `सिद्ध` *siddha* | `सिद्ध करो` *siddha karo* / `प्रमाणित` *pramāṇita* | `सिद्ध करा` *siddha karā* / `दाखवा` *dākhvā* |
| `requires` | `अपेक्षित` *apekṣita* | `चाहिए` *cāhiye* | `पाहिजे` *pāhije* |
| `ensures` | `सुनिश्चयित` *sunishchayita* | `निश्चित` *nishchit* | `निश्चित` *nishchit* |
| `invariant` | `अपरिवर्तनीय` *aparivartanīya* | `अपरिवर्तनीय` *aparivartanīya* | `अपरिवर्तनीय` *aparivartanīya* |

### Booleans + I/O

| English | संस्कृत (Sanskrit) | हिन्दी (Hindi) | मराठी (Marathi) |
|---|---|---|---|
| `true` | `सत्य` *satya* | `सत्य` *satya* / `सही` *sahī* | `सत्य` *satya* / `सही` *sahī* |
| `false` | `असत्य` *asatya* | `असत्य` *asatya* / `अशुद्ध` *aśuddha* | `असत्य` *asatya* / `अशुद्ध` *aśuddha* |
| `print` / `write` | `लिख` *likh* | `लिखो` *likho* | `लिखो` *likho* |

### Concurrency + parallelism

| English | संस्कृत (Sanskrit) | हिन्दी (Hindi) | मराठी (Marathi) |
|---|---|---|---|
| `pure` | `शुद्ध` *śuddha* | `शुद्ध` *śuddha* | `शुद्ध` *śuddha* |
| `parallel` | `समानांतर` *samānāntara* / `समान्तर प्रति` *samāntara prati* | `समानांतर` *samānāntara* | `समानांतर` *samānāntara* |
| `reduce` | `संक्षेप` *saṁkṣepa* | `संक्षेप` *saṁkṣepa* | `संक्षेप` *saṁkṣepa* |
| `with` | `सह` *saha* | `सह` *saha* | `सह` *saha* |
| `task` | `नियोग` *niyog* | `नियोग` *niyog* | `नियोग` *niyog* |
| `join` | `संयोजन` *saṁyojan* | `संयोजन` *saṁyojan* | `संयोजन` *saṁyojan* |
| `try` | `प्रयास` *prayās* | `प्रयास` *prayās* | `प्रयास` *prayās* |

### Interfaces, generics, embedded

| English | संस्कृत (Sanskrit) | हिन्दी (Hindi) | मराठी (Marathi) |
|---|---|---|---|
| `interface` / `trait` | `संकेत` *saṅket* / `अंतरापृष्ठ` *antarāpṛṣṭha* | `संकेत` *saṅket* | `संकेत` *saṅket* |
| `implement` / `impl` | `कार्यान्वित` *kāryānvit* | `कार्यान्वित` *kāryānvit* | `कार्यान्वित` *kāryānvit* |
| `methods` | `विधि` *vidhi* | `विधि` *vidhi* | `विधि` *vidhi* |
| `where` | `यत्र` *yatra* | `जहाँ` *jahām̐* | `जिथे` *jithe* |
| `is` | `अस्ति` *asti* | `है` *hai* | `आहे` *āhe* |
| `unsafe` | `असुरक्षित` *asurakṣita* | `असुरक्षित` *asurakṣita* | `असुरक्षित` *asurakṣita* |
| `region` | `क्षेत्र` *kṣetra* | `क्षेत्र` *kṣetra* | `क्षेत्र` *kṣetra* |

### SOV (Subject-Object-Verb) statement shapes

When the user opts into Sanskrit / Hindi / Marathi grammar
naturally, vāṇी accepts **verb-at-end SOV order** for these
common shapes alongside the keyword-first English order:

| Construct | Keyword-first | SOV verb-at-end |
|---|---|---|
| `let` | `माना x: i64 = 5;` | `x: i64 = 5 माना;` |
| `return` | `पुनरागम x;` | `x पुनरागम;` |
| `print` | `लिख x;` | `x लिख;` |
| `assert` | `सिद्धम् cond;` | `cond सिद्धम्;` |
| `prove` | `प्रमाण expr;` | `expr प्रमाण;` |
| range `for` | (no English shape) | `i प्रति 0 से 3 तक { ... }` |
| `if` / `else` | `यदि cond { ... }` | `cond यदि { ... } अन्यथा { ... }` |
| `while` | `यावत् cond { ... }` | `cond यावत् { ... }` |

Top-level declarations (`fn` / `struct` / `enum`) read naturally
keyword-first in Indo-Aryan grammar; SOV reshape there would feel
forced and is intentionally not supported. `match` SOV is available
inside SOV-let (`let r: T = scrutinee match { ... } माना;`).

### Per-file dialect purity (opt-in)

Drop a `// vani-lang: <sanskrit|hindi|marathi|english>` comment in
the first 10 lines of any `.vani` file to enforce single-dialect
purity. The lexer then rejects any keyword spelling not native to
the declared dialect. Without the pragma, the existing script-
level English-vs-Devanagari gate still applies (back-compat).

The pragma also turns on **dialect-aware error rendering**:
diagnostics emit their `error:` / `note:` labels in the declared
dialect (Sanskrit `त्रुटिः`, Hindi `त्रुटि`, Marathi `चूक`), and
the leading prefix of the most common error families translates
to the dialect (e.g. `unknown variable` → `अज्ञातं चरम्` in
Sanskrit). The English wording is retained after the dialect
phrase so search-engine queries and existing documentation still
match.

### Devanagari numerals, type names, identifiers

Inside a `.vani` source file, every category of token can be
Devanagari:

- **Numerals**: integer + float literals accept Devanagari digits
  `०१२३४५६७८९` (U+0966–U+096F). `५ * २` parses as `5 * 2`; `३.१४`
  parses as the f64 `3.14`. Mixed ASCII / Devanagari digits in
  the same literal are NOT supported (pick one per number).
- **Type names**: `पूर्णांक` (i64), `दशांश` (f64), `तर्क` (bool),
  `सूची` (Vec), `पूर्णांक८/१६/३२/६४` (i8/i16/i32/i64 width-
  explicit), `अहस्ताक्षरित८/.../६४` (u8..u64 unsigned). All are
  Sanskrit-root tatsama forms working as loanwords in Hindi +
  Marathi, so a single Devanagari spelling works across all
  three dialects.
- **Identifiers**: user-defined function / variable / struct /
  field names accept any Devanagari letter. The LLVM backend
  mangles non-ASCII names to `_uHHHH` per codepoint behind the
  scenes (e.g. `द्विपदगुणक` → `_u0926...`); the C backend uses
  the UTF-8 bytes directly (gcc / clang accept UTF-8 in
  identifiers natively).

A fully-Devanagari program is now possible — see
[`examples/language/sanskrit/pure_devanagari.vani`](examples/language/sanskrit/pure_devanagari.vani)
for a complete Pascal's-triangle-row example using Sanskrit
keywords + Sanskrit identifiers + Sanskrit comments + Devanagari
numerals + SOV verb-at-end statements + function-level
`अपेक्षित` (requires) contracts. The reading experience is meant
to feel like Sanskrit prose with the verb closing each clause —
not a transliterated English program.

Pronunciation guide for the diacritics used in the romanizations:

| Mark | Roman | Sound | Example |
|---|---|---|---|
| ā | long-a | as in *father* | *kārya* = "kaar-yuh" |
| ī | long-i | as in *machine* | *vāṇī* = "vaa-nee" |
| ū | long-u | as in *rule* | *mūla* = "moo-luh" |
| ṛ | retroflex r | rolled tongue tip | *kṛṣṇa* = "krish-nuh" |
| ṇ | retroflex n | tongue against palate | *vāṇī* = "vaa-NEE" |
| ṭ / ḍ | retroflex t / d | tongue curled back | *paṭha* = "pa-tha" |
| ś / ṣ | sh-sounds | as in *shoe* / *bush* | *kṛṣṇa* = "krish-nuh" |
| ñ | palatal n | as in *canyon* (ny) | *jña* = "gya" |
| ṁ / ṃ | anusvāra | nasalizes preceding vowel | *saṁskṛta* = "sun-skrit" |
| ḥ | visarga | soft h-release | *namaḥ* = "nam-ah" |

A short worked example: the project name **वाणी** romanizes to **vāṇī**,
read as "vaa-NEE" — long-a, retroflex-n, long-i. The acronym **VANI**
keeps the same three syllables but drops the diacritics for ASCII use.

The actual keyword mapping will be finalized with grammar consultants for
each language so the verbs feel idiomatic and unambiguous in context.
Mixing scripts in the same file is supported by design — a student can
write the keywords in Devanagari and the identifiers in English, or vice
versa.

Supported today (800 lib + 47 e2e tests passing):

### Types
- Scalars: `i8`/`i16`/`i32`/`i64`, `u8`/`u16`/`u32`/`u64`, `f32`/`f64`, `bool`
  (all `Copy`).
- Strings: `Str` (borrowed C-string, `Copy`, `==`/`!=`/`<`/`<=`/`>`/`>=` via
  strcmp), `OwnedStr` (heap, affine, produced by `+` concat).
- Fixed-size stack arrays `[T; N]` (affine) with `xs[i]` and `len(xs)`.
- Heap-allocated `Vec<T>` (affine) with `vec(...)`, `push` / `set` / `clone`,
  `len`, indexing, `clone_at(ref xs, i)` for non-Copy slot reads. Empty
  `vec()` is supported. `Vec<Vec<T>>` and `Vec<Struct>` work. `push` has
  two forms: `push(xs: Vec<T>, v) -> Vec<T>` (consuming) and
  `push(xs: mut ref Vec<T>, v) -> i64` (in-place, returns the new length —
  useful through a struct field). See
  [examples/push_mut.vani](examples/push_mut.vani).
- Tuples `(T1, T2, ...)` (n in 2..=4) with `.0` / `.1` access; destructure
  `let (a, b) = expr;`.
- Structs `struct Point { x: i64, y: i64 }` with up to 64 fields; field access
  `p.x` and field assign `p.x = v;`.
- Enums: `enum Color { Red, Green, Blue }`. Payloaded variants `enum Opt
  { Some(i64), None }` work in both backends — tagged-union codegen lays
  them out as `{ i32 tag, T payload }`. Match destructure
  `Opt.Some(v) then …` binds the payload into the arm scope. V1 limits
  payloads to single Copy fields per variant + uniform payload type
  across variants. See [examples/option_types.vani](examples/option_types.vani).
- Type aliases: `type Coord = (i64, i64);`, `type X = i64;`.
- Constants: `const ANSWER: i64 = 42;` — literal initializers only in v1.

### References (second-class, keyword-first)
- `ref T` (shared) and `mut ref T` (mutable) — parameter types only;
  borrow at call sites with `ref xs` / `mut ref xs`. No reference returns,
  let-bindings, or aggregate elements. Aliasing rejected.
- Indexed write `xs[i] = v;` works on owned `[T;N]` / `Vec<T>` and through
  `mut ref` parameters.
- Auto-deref for indexing and method dispatch.

### Functions, methods, and dispatch
- Functions `fn add(a: i64, b: i64) -> i64 { … }`; pure-fn marker
  `pure fn …` for SMT-callable helpers.
- `methods on T { fn m(self: T) -> R { … } }` blocks. Receivers must be
  `self: T` / `self: ref T` / `self: mut ref T` (keyword-first; `&self`
  rejected). Method dispatch via `recv.method(args)` with auto-ref.
- First-class fn-pointers `fn(T1, ...) -> R` with `FnRef` + indirect call.
- Discarded call statements: `x.bump();` / `foo();` are sugar for
  `let _ = …;` (must be a `Call`/`MethodCall`).

### Control flow + expressions
- `if`/`else`/`else if` chains as statements OR single-expression form
  `if cond { e1 } else { e2 }` (both branches must unify).
- `while cond invariant inv1; invariant inv2; { … }`.
- `for i from lo to hi invariant inv; { … }`, `for x in ref xs { … }`,
  `for x in xs { … }` (consuming).
- `break;` / `continue;`, `assert cond[, "msg"]`, `prove`, `print` (multi-item).
- `match scrutinee { Color.Red then expr, … }` — exhaustive over enum
  variants; integer-literal patterns, `_` wildcard, and **payloaded variant
  destructure** `Opt.Some(v) then …` all supported. Bool / Str / float
  scrutinee patterns are gated.
- **Block expressions** `let r = { let a = …; let b = …; a + b };` — Let
  stmts followed by a tail expression. Inner shadows don't leak.
- **`try EXPR` / `EXPR?`** — Option/Result-like error-propagation sugar.
  In a function whose return type is a payloaded enum, both
  `let v: T = try opt;` and `let v: T = opt?;` extract the payload or
  short-circuit the function with the payload-less variant. The postfix
  `?` form is pure parse-time sugar — it builds the same `ExprKind::Try`
  AST node, so narrow-gate restrictions (let-try as first stmt,
  intermediate lets, return) apply identically. See
  [examples/language/english/try_keyword.vani](examples/language/english/try_keyword.vani) (keyword form)
  and [examples/language/english/try_question_op.vani](examples/language/english/try_question_op.vani) (`?` form).
- Short-circuit `&&` and `||` honor compile-time const folding —
  `false && (provably-bad)` and `true || (provably-bad)` compile cleanly.
- Lexical scoping: inner `let x` shadowing of an outer same-name binding
  is contained to the inner scope (cross-type shadow allowed).

### Generics & interfaces
- **Generic functions** `fn id<T>(x: T) -> T { return x; }` —
  monomorphized at compile time. The pre-pass walks call sites, infers
  T from the first literal argument (v1 restriction), and generates a
  specialized copy per concrete type (`id__i64`, `id__bool`, …). The
  original generic template is dropped before codegen sees it. See
  [examples/generic_functions.vani](examples/generic_functions.vani).
  V1 limits: single type parameter, body must be type-correct without
  knowing T (pass-through patterns).
- **Interfaces** `interface Show { fn show(self: T) -> R; }` + `implement
  Show for Point { fn show(self: Point) -> R { … } }` — static dispatch
  via `recv.show()`. The impl hoists to `T_show`; the existing method-
  dispatch path resolves the call at compile-time based on the receiver's
  type. V1 limits: static dispatch only (no vtables); each impl must cover
  every interface method; signatures must match exactly. See
  [examples/interfaces.vani](examples/interfaces.vani).
- **Drop interface** `implement Drop for T { fn drop(self: T) -> i64 { … } }`
  — auto-called at every scope exit where a non-moved binding of T goes
  out of scope. Users can also call `t.drop()` manually; affine tracking
  marks the binding as moved so the auto-call won't double-fire. When T
  has heap-shaped fields (OwnedStr / Vec), the per-field free pass runs
  instead (the user's drop is then invoked explicitly when richer
  behavior is needed). See
  [examples/drop_interface.vani](examples/drop_interface.vani).
- **Mixed-place assignment** — `xs[i].field = v;` and the deeper
  `xs[i].a.b = v;` write through an index plus a struct field path in
  one statement. Works on owned `Vec<T>` and `[T; N]`. Intermediate
  segments must be Copy structs. The leaf field may be Copy OR a
  heap-shaped type (`OwnedStr` / `Vec<T>`) — when the leaf is heap-
  shaped, both backends free the previous slot value before storing
  the new one, so the old allocation does not leak. See
  [examples/mixed_place_assign.vani](examples/mixed_place_assign.vani).
- **Partial-move tracking** — `let taken = bag.contents;` moves a single
  field out of a struct. The aggregate is still readable for its other
  fields; scope-exit Drop skips the moved field (no double-free); a
  second read of the moved field surfaces a use-after-move diagnostic.
  See [examples/partial_move.vani](examples/partial_move.vani).
- **User-defined `==` via `implement Eq for T`** — `a == b` and `a != b`
  on struct or enum bindings desugar to the hoisted `<T>_eq(a, b)` /
  `!<T>_eq(a, b)` whenever both sides are the same nominal type.
  Convention is `fn eq(self: T, other: T) -> bool`. See
  [examples/struct_eq.vani](examples/struct_eq.vani) and
  [examples/enum_eq.vani](examples/enum_eq.vani).
- **Tuple auto-equality** — tuples are anonymous, so `==` is
  compiler-derived: `(a, b) == (c, d)` rewrites to `a == c && b == d`.
  Each per-element comparison uses the element type's `==` rule
  (built-in for primitives, `<T>_eq` for nominal element types). See
  [examples/tuple_eq.vani](examples/tuple_eq.vani).
- **Field-borrow expressions** — `ref t.f` and `mut ref t.f` take a borrow
  of a struct field. The result type is `&<field_ty>` / `&mut <field_ty>`;
  backends GEP into the struct's storage. Unlocks atomic operations
  through a struct that owns the cell (`atomic_*(ref c.hits)` /
  `atomic_*(mut ref c.hits)`). Single-level only in v1
  (no `ref t.a.b`). See
  [examples/struct_atomic_field.vani](examples/struct_atomic_field.vani).
- **Enums with affine payloads** — Copy types, `OwnedStr`, `Vec<T>`,
  `[T; N]` of Copy elements, `Task`, `Atomic<T>`, `Mutex<T>`, and
  `Channel<T, N>` are all valid as enum payload types in v1; only
  `Guard<T>` still needs codegen work. Heap payloads (OwnedStr, Vec) get a tag-conditional
  free at scope exit; stack-shaped payloads (array, Task, Atomic) need
  no Drop. v1 restriction: destructure-binding patterns (`Some(s)`)
  require Copy payloads. See
  [examples/enum_owned_payload.vani](examples/enum_owned_payload.vani),
  [examples/enum_vec_payload.vani](examples/enum_vec_payload.vani),
  [examples/enum_arr_payload.vani](examples/enum_arr_payload.vani).
- **Structs with affine fields** — `OwnedStr`, `Vec<T>`, `[T; N]` of Copy
  elements, `Task`, `Atomic<T>`, `Mutex<T>`, `Channel<T, N>`, and **nested
  affine structs** are valid struct field types in v1. Both backends
  recursively walk struct types at scope-exit Drop time so a `struct
  Outer { inner: Inner, id: i64 }` where `Inner` has `OwnedStr` /
  `Vec<T>` fields gets full RAII chains. Only `Guard<T>` is still
  rejected. See
  [examples/nested_struct_drop.vani](examples/nested_struct_drop.vani).
  Heap-shaped fields (OwnedStr, Vec) are freed at scope exit; stack-shaped
  fields (arrays, Task, Atomic) need no runtime drop. Struct-literal init
  from a `Var` moves the source binding so a heap value flows `caller →
  struct field → drop` without a double-free. Field-path indexing
  (`t.data[i]`) works through both backends. Mutex / Guard / Channel still
  need explicit wiring. See
  [examples/struct_owned_field.vani](examples/struct_owned_field.vani),
  [examples/struct_mixed_fields.vani](examples/struct_mixed_fields.vani).

### Verification & contracts
- `requires` / `ensures` clauses (terminated with `;`, before the body).
  `_return` references the return value; inline calls discharged via callee
  `ensures`.
- Loop invariants with substitution-based preservation and post-loop facts.
- Three-layer `prove`: constant fold → structural tautology → SMT (Z3).
- BitVec overflow-aware integer arithmetic; IEEE-754 floats (NaN/±inf
  modeled); signed/unsigned compare split; cast-via-extend.
- Symbolic SMT arrays per Vec/array binding with versioned store axioms.
- SMT-driven runtime-guard elision (bounds, divisor, shift checks).
- Compile-time const overflow and divide-by-zero detection.
- `INTENTC_NO_VERIFY=1` opt-out for fast dev iteration.

### File I/O (v0.1.5+)
- **`FileHandle`** — affine RAII handle; auto-`fclose`d at scope exit. Both C and LLVM backends.
- `file_open(path: Str, mode: Str) -> FileHandle`, `file_is_ok(ref fh) -> bool`,
  `file_read_line(mut ref fh) -> OwnedStr`, `file_write(mut ref fh, s) -> i64`,
  `file_close(fh) -> i64`, `file_flush(mut ref fh) -> i64`.
- `stdin_read_line() -> OwnedStr`, `flush_stdout() -> i64`.
- `eprint` statement — writes to stderr (same multi-item syntax as `print`).

### Bare-metal / embedded (v0.1.6+)
- **`#[no_mangle]`** attribute on `fn` — suppresses `intent_` prefix and Unicode mangling in both backends.
- **`#[link_section = "..."]`** attribute on `fn` — `__attribute__((section(...)))` in C; `section "..."` on LLVM IR `define` line.
- **`mmio_read_u8(addr) -> u8`** / **`mmio_write_u8(addr, val) -> i64`** — 8-bit volatile MMIO.
- **`mmio_read_u16(addr) -> u16`** / **`mmio_write_u16(addr, val) -> i64`** — 16-bit volatile MMIO.
- **`--target=<triple>`** on `vanic build` / `vanic run` — cross-compilation; bare-metal triples suppress libc/OpenMP/pthread.
- **`--no-std`** on `vanic emit --backend=c` — suppresses all `#include <std*.h>`; auto-activates for bare-metal triples.

### Affine ownership
- Arrays, `Vec`, `OwnedStr`, `Task`, `Atomic`, `Mutex`, `Guard`, `Channel`,
  `Barrier`, `RwLock`, `ReadGuard`, `WriteGuard`, `FileHandle`
  are affine — moved on use, dropped at end of scope.
- Use-after-move is a compile error with related-span notes pointing at the
  prior move site.
- `let` shadowing drops or consumes the previous binding.
- `_` discard binding (`let _ = expr;`) covers drop for Copy results and
  triggers the affine drop chain for owned ones.

### Concurrency
- `parallel for` with reductions (`+`, `*`, `&&`, `||`, `&`, `|`, `^`,
  `min`, `max`). Verifier proves race-freedom; backends emit real threads
  (libgomp on Linux, CreateThread on Windows).
- `task <name> { … } / join <name>;` — affine handles, Copy-only captures,
  real pthread / CreateThread spawn.
- `Atomic<T>` (i8..i64, u8..u64, bool) — `atomic_new`/`atomic_load`/
  `atomic_store`/`atomic_fetch_add`/`atomic_compare_exchange`.
- `Channel<T, N>` — Vyukov MPSC ring buffer (power-of-2 N).
- `Mutex<T>` + RAII `Guard<T>` — Drepper futex (Linux), WaitOnAddress
  (Windows), sched_yield/SwitchToThread fallback.

### Tooling
- `intentc check / emit / emit-c / run / build / test` with `--json`
  machine-readable diagnostics.
- `intent-lsp` binary with hover, definition, references, rename,
  completion, code actions, semantic tokens (7 token types, 2 modifiers).
- Parser error recovery — multiple errors per compile, not just the first.
- Diagnostics with related-span notes.
- Multi-file projects via `use "path.vani";` (transitive, cycle-detected).

### Backends
- **LLVM** is the default for `emit`/`run`/`build` (AOT via `llc + cc`).
- **C** (`--backend=c`, legacy/deprecation path).
- Both have tree-shaped and SSA pipelines; `intentc` tries SSA first and
  falls back to tree backends on `EmitError`.
---

# Part III — Memory Safety, Ownership & Concurrency

## Memory safety & concurrency model

vāṇī treats **memory and concurrency bugs as compile-time errors
on the safe path** (hosted targets, and embedded code outside an
explicit `unsafe(reason = "...") { ... }` block — see *Embedded
targets — current position* below). The runtime is meant to be
boring: no garbage collector, no event loop today (async / event
loop is queued — compiler-lowered state machines on an arena, not
Rust-style `Pin`; see [TODO.md](TODO.md) *Async / asyncio*), no
allocator-dependent fault injection, no reference counting, no
surprise rescheduling.
Everything that would be a "this might crash at 3 AM in production"
bug in a less strict language fails the type checker on the
developer's laptop — with the embedded `unsafe(reason = "...")`
block as the single, opt-in, lexically scoped exception for
operations the compiler genuinely cannot prove (raw MMIO outside
typed primitives, inline asm, vendor SDK FFI). Affine ownership,
move tracking, ISR / `parallel for` / `task` restrictions, and Drop
emission stay active *inside* `unsafe` — the block only suspends
pointer-safety and type-punning invariants.

### What's caught at compile time

| Bug class | Caught at compile time | How |
|---|---|---|
| **Heap leak** ("forgot to free") | ✅ | Affine ownership. Every heap-owning binding (`Vec`, `OwnedStr`, `Atomic`, `Mutex`, `Guard`, `Channel`, `Task`, struct with heap fields) has exactly one owner. The codegen emits `free` / per-field drop at scope exit deterministically. There is no `forget()` equivalent. |
| **Double-free** | ✅ | Move tracking. After `let y = x;` (where `x: Vec<i64>`), `x` becomes unreadable; the compiler emits a "value 'x' was moved; cannot use after move" diagnostic at the next reference. Drop fires exactly once on the new owner. User-defined `Drop` is also single-fire. |
| **Use-after-free** | ✅ | Same affine machinery + scope-escape analyzer for `ref T` / `mut ref T`. References can now appear in `let` bindings (L4 (B) Phase 1) and in user struct fields (L4 (B) Phase 3); the analyzer in `collect_ref_sources_in_expr` rejects every shape that would let the reference outlive its source — returning the ref-holding struct, storing it in a `Vec`, taking `mut ref T` across a suspend point, etc. The borrow can never outlive the owner. |
| **Dangling reference** | ✅ | Scope-escape analyzer + structural rejects + lifetime elision. Functions **can return references** under the single-ref-parameter elision rule (L4 (C), 2026-06-09) — the return ref's lifetime is inferred from the single ref param; zero/multi-ref-param returns reject with a clear diagnostic. References can be stored in `let` bindings, struct fields, and `Vec<T>` as of L4 (B) (Phases 1+3+4, 2026-06-09); the analyzer at every escape site (push, FieldAssign, return) chases ref aliases through `let r = foo(ref X)` chains so the original source's scope bounds every transitive use. Compiler emits "ref to 'x' would dangle when 'x's scope ends" with the exact source location. Only multi-input distinct-lifetime patterns (Rust-style `'a` / `'b`) remain deferred (path-D, indefinite). |
| **Aliasing mutable + immutable** | ✅ | A `mut ref T` borrow rejects every subsequent shared `ref T` on the same value (and vice-versa) for the scope of the borrow. Diagnostic: "value 'v' is borrowed mutably; cannot also share-borrow". |
| **Data race in `parallel for`** | ✅ | The effects checker walks the loop body and rejects observable side effects: `print`, calls to impure functions, non-Copy moves into the body, indexed writes on captured arrays / `Vec`s. **As of closure #259**, captured Copy-typed mutations are also caught — `total = total + i;` on a binding declared OUTSIDE the body errors with "mutates captured variable 'total' without declaring it as a reduction" and points the user at `reduce` or `Atomic<T>`. Body-local lets remain free to mutate (per-iteration, not shared). Atomic / Mutex captures must be via `ref`. |
| **Data race in `task`** | ✅ | `task` captures are Copy-only by default — affine handles (Vec, Atomic, Mutex, Guard, Channel) can't ride into the thread by value. Shared state goes through `Atomic<T>` (lock-free, seq-cst) or `Mutex<T>` + `Guard<T>` (RAII unlock at scope exit). |
| **Unjoined task** ("thread leak") | ✅ | `Task` is affine. The compiler tracks each handle and requires a matching `join name;` before the handle's scope ends, even on early-return paths. Double-`join` is also rejected. |
| **Forgotten mutex release** | ✅ | `Guard<T>` is affine — taking the lock returns a `Guard` that **must** drop, and Drop emits the unlock. The borrowed inner `T` lives only as long as the Guard; the compiler rejects keeping the inner reference after the Guard drops. |
| **Integer overflow / underflow** | ✅ (where SMT proves) | The bounds-elision pass keeps `if (UB-check)` guards by default and elides only when Z3 proves the operation is in-range. `INTENTC_NO_VERIFY=1` keeps the guards in place. |
| **Array / Vec out-of-bounds** | ✅ (where SMT proves) | Same elision pass on `Index` / `IndexAssign`. Guards stay in place when SMT can't discharge the obligation. |
| **Divide / shift / mod by zero** | ✅ (where SMT proves) | Same. |
| **`assert` / `prove` / `requires` / `ensures` / `invariant`** | ✅ | Discharged by Z3 at check time. `prove` is the strict form (must hold); `ensures` is verified at every return path; `invariant` at loop entry, body, and exit. |

**The table above describes the safe path only.** Inside an
`unsafe(reason = "...") { ... }` block on an embedded target, the
user takes responsibility for the listed invariants — raw pointer
arithmetic, `transmute`-style reinterpretation, MMIO at ad-hoc
addresses, and FFI into untyped vendor SDKs can all bypass these
checks. The mitigations are documented in [unsafe.md](unsafe.md)
and summarized below in *What runs inside `unsafe(reason = "...")`*.

The affine layer (move tracking, Drop emission, ISR / parallel /
task body restrictions) **stays active inside `unsafe`** — the
block only suspends pointer-safety and type-punning invariants, not
ownership. Everything else in the table that doesn't depend on
those two stays enforced.

### What runs inside `unsafe(reason = "...")` (the safety net)

When you do reach for `unsafe(reason = "...")`, the language doesn't
go quiet. The plan-of-record in [unsafe.md](unsafe.md) layers four
mechanisms; v1 (Layers 1–4) ships first:

| Layer | What it catches inside `unsafe` | Cost |
|---|---|---|
| **Lexical containment** (Layer 1.1, ✅ **shipped 2026-06-02**) | The `unsafe(reason = "...") { ... }` block parses, type-checks, and the reason flows through both backends as machine-readable deviation metadata (`/* UNSAFE-DEVIATION: ... */` in tree-C; `; UNSAFE-DEVIATION: ...` in tree-LLVM IR). Reviewers grep these markers to find every escape. Raw `*T` types land in Layer 1.2+ and plug into the boundary already in place. | 0 (parse-time) |
| **Mandatory reason clause** (Layer 1.1, ✅ **shipped 2026-06-02**) | Parser enforces: non-empty, ≤256 chars, ASCII-printable, no embedded newlines. Empty / missing / oversized / non-ASCII / multi-line reasons are parse errors. Hosted builds reject the construct by default; `INTENT_TARGET_EMBEDDED=1` opens it until the `--target embedded` flag lands. | 0 (parse-time) |
| **No-escape on `&local`** (Layer 1.2) | A raw pointer derived from a stack variable cannot return, store into heap, or escape via global. Catches "returns pointer to dead stack frame." | 0 (compile-time dataflow) |
| **`Tainted<T>`** (Layer 1.3) | Values loaded through raw pointers are wrapped in `Tainted<T>`; storing tainted values into safe-typed slots requires explicit `assert_safe(x)`. Catches "unsafe data silently poisons safe code." | 0 (compile-time) |
| **`Handle<T>` + `Pool<T>`** (Layer 2, v1 default) | Generational handles. Use-after-free and double-free are caught at runtime by generation mismatch on `pool.get(h)` → returns `None`. The blessed long-lived "pointer-like" type crossing safe/unsafe. | ~3–5 cycles per deref on Cortex-M |
| **Canary words** (Layer 3.1) | `unsafe_alloc` brackets allocations with magic words; `unsafe_free` verifies. Catches buffer overruns and some double-frees at the moment of free. Debug-only; strippable. | ~16 bytes per alloc, free-time check |
| **`BoundedPtr<T>`** (Layer 3.2) | Fat pointer carrying data + len + capacity. `BoundedPtr.get(i)` is bounds-checked; raw `.data` field is not. Opt-in inside unsafe. | 2× pointer width, +1 cmp per checked access |
| **Stack canaries / ARM MTE** (Layer 4) | Stack smashing (`-fstack-protector-strong`) and HW-tagged use-after-free / overruns (ARMv8.5+ MTE). | 0–2 cycles per stack frame; free in HW |
| **Region typing** (Layer 5, v2 future) | `region { ... }` blocks; `&'arena T` pointers carry compile-time use-after-free proof. For safety-critical certification (ASIL-D, DO-178C, IEC 62304). | 0 runtime cost (compile-time) |

What `unsafe(reason = "...")` does **not** automatically catch:

- **Heap leak.** Inside the block, `unsafe_alloc` without a matching
  `unsafe_free` is a leak. The canary in Layer 3.1 verifies the
  free is well-formed but doesn't tell you a free is missing. Use
  `Handle<T>` / `Pool<T>` (Layer 2) when you can — pool drop
  reclaims everything.
- **Data races on raw aliased writes.** If you derive two `*mut T`
  pointers to the same location and write through both from
  different threads, neither the affine checker nor the v1 layers
  catch this. The `unsafe(reason = "...")` keyword obligates you to
  justify why this won't happen. v2 regions don't help here either.
- **Type punning that the C/LLVM toolchain miscompiles.**
  `transmute`-style casts inside `unsafe` are at the mercy of
  strict-aliasing rules in the underlying C/LLVM compile. The reason
  prefix `"transmute: ..."` flags these for special review.

### What runs (without you reaching for `unsafe`)

vāṇी has **no `unsafe` block on hosted targets** (Linux / Windows /
macOS). Every operation in source is type-checked + affine-tracked.
The compiler doesn't trade safety for ergonomics anywhere — including
for raw pointer arithmetic, mmap, syscalls, or FFI.

The single exception is **embedded / bare-metal targets**, where an
explicit `unsafe(reason = "...") { ... }` block is the opt-in escape
hatch for the narrow set of operations the compiler cannot prove
safe (raw MMIO outside the `mmio_read_u32` / `mmio_write_u32` builtins,
inline assembly, platform intrinsics, custom linker-placed memory).
The keyword has to be typed, the reason has to be filled in, and
the default is checked. Hosted builds reject `unsafe` entirely. See
*Embedded targets — current position* below.

### vāṇī vs Rust — ownership at a glance

vāṇī uses **the same move-by-default model as Rust**. The goal is
that users live with `move` semantics by construction; explicit
`clone()` only happens when the user types it (and the compiler
makes it clear when that's needed). There is **no implicit clone
anywhere** in the language.

| Property | Rust | vāṇī |
|---|---|---|
| Primitive scalars (`i64`, `bool`, …) | Copy | Copy |
| Borrowed string view (`&str` / `Str`) | Copy (pointer) | Copy (pointer) |
| References (`&T` / `&mut T` vs `ref T` / `mut ref T`) | Copy | Copy (second-class; scope-bound via escape analysis — fine in params, `let` bindings, user struct fields, **and `Vec<ref T>` / `Vec<mut ref T>` since 2026-06-09**; cannot be returned from a function) |
| Heap string (`String` / `OwnedStr`) | Move (affine) | Move (affine) |
| Heap vector (`Vec<T>` / `Vec<T>`) | Move (affine) | Move (affine) |
| Fixed array (`[T; N]`) | Copy if `T: Copy`, else Move | Affine (Move) always — explicit |
| Struct (every field Copy) | Copy if `#[derive(Copy)]` | Copy automatically (no derive needed) |
| Struct (any affine field) | Move | Move |
| Enum (every payload Copy) | Copy if derived | Copy automatically |
| Enum (any affine payload) | Move | Move |
| `Atomic<T>` / `Mutex<T>` / `Channel<T, N>` | Affine via lifetime / `Arc` | Affine, single-owner — no `Arc` equivalent |
| Thread handle (`JoinHandle` / `Task`) | Affine; must `join` or detach | Affine; **must `join`** (no detach in v1) |
| Implicit clone anywhere? | Never | Never |
| Explicit `.clone()` cost | Visible at the call site | Visible at the call site |

Two practical takeaways:

- **Reach for `ref` first, `clone()` last.** If a function only needs
  to *read* a `Vec<T>`, declare the parameter as `xs: ref Vec<T>` and
  call it as `f(ref xs)`. The borrow is cheap (pointer-sized) and the
  caller keeps ownership — no copy, no clone, no diagnostic. If the
  callee needs to mutate, use `mut ref Vec<T>` + `f(mut ref xs)`. The
  same convention works through struct fields with `ref t.field`.
- **Auto-borrow does the obvious thing.** Comparing two `OwnedStr` /
  `Vec<T>` operands via `==` or feeding an `OwnedStr` to a function
  that wants `Str` auto-borrows the operand — the binding stays
  usable on the next line. No silent clone.

For deep-copying a single `Vec<T>` slot whose element type is
non-Copy (e.g. `Vec<OwnedStr>`), use the explicit builtin
`clone_at(ref xs, i)`. There is no implicit pathway.

If you write code that *requires* a clone to compile — say,
two threads both need their own copy of a `Vec` — the diagnostic
will point at the consume site with the binding's earlier move
location and the suggestion to either restructure or call
`clone()` explicitly. The compiler never picks the clone for you.

### Smart-pointer primitives — Rust / C++ comparison

Rust ships `Box<T>`, `Rc<T>` / `Arc<T>`, `RefCell<T>`, and `Weak<T>` as
distinct types for different memory-management patterns. C++ has
`unique_ptr`, `shared_ptr`, and `weak_ptr` for the same patterns.
**vāṇी ships none of these.** Each of the use cases is either covered
by an existing primitive or **structurally avoided by the type system**:

| Rust / C++ tool | What it solves | vāṇी's approach |
|---|---|---|
| `Box<T>` / `unique_ptr<T>` | Single-owner heap allocation | **Ships natively as `Box<T>`** (L2 lift Phases 1+2+3+3b, 2026-06-08). `box(value)` heap-allocates a single T and returns an affine handle; the compiler emits recursive-drop on scope exit. Supported inner types: every Copy-sized `T`, `Box<dyn Iface>` (16-byte fat-pointer owning its concrete on the heap), `Box<Vec<T>>` and `Box<OwnedStr>` (both chain drop into the inner buffer), `Box<Box<T>>`, `Box<(...)>`, `Option<Box<T>>` for recursive-data shapes like `struct Node { next: Option<Box<Node>> }`. For sequences, `Vec<T>` and `OwnedStr` remain the dedicated heap-owning primitives. The Tutorial chapter on [Box and RAII](tutorials/src/intermediate/03a_box_raii_primer.md) walks the variations. |
| `Rc<T>` / `Arc<T>` / `shared_ptr<T>` | Reference-counted shared ownership | **Not available by design.** Shared *ownership* is unrepresentable. Producer / consumer parallelism uses `Channel<T, N>`. Shared mutable state across threads uses `Atomic<T>` references or `Mutex<T>` + `Guard<T>` — borrowed (not cloned) into each thread. Non-owning shared *references* are expressible via `Handle<T>` (Layer 2 in [unsafe.md](unsafe.md)) — the Pool owns; multiple Handles can name the same slot. |
| `RefCell<T>` (interior mutability) | Mutate through a shared reference at runtime | **Not available by design.** vāṇी has no runtime borrow-checker — every aliasing rule fires at compile time. The need is mitigated by `mut ref T` parameters + mixed-place assignment (`xs[i].field = v;` writes through an index into a struct field in one statement). |
| `Weak<T>` (cycle breaker) | Non-owning back-reference to break `Rc`/`Arc` cycles | **Not needed for ownership cycles** — single-owner affine types make a cyclic *ownership* graph unrepresentable in the type system. **For data-structure cycles** (parent ↔ child, observer pattern), the supported idioms are: (1) indices into a `Vec<T>` (always available); (2) `Handle<T>` into a `Pool<T>` (Layer 2 in [unsafe.md](unsafe.md), generation-checked so stale handles return `None` instead of dangling); (3) `&'arena T` inside a region block (Layer 5, v2, zero runtime cost for safety-critical workloads). None of these introduce reference-counting overhead. |

### What about cyclic data structures?

Graph-like data (parent ↔ child, observer pattern, doubly-linked list)
typically needs cycles in languages that have shared ownership. In
vāṇी the idiom is **indices into a `Vec`**:

```vani
struct Node {
  value: i64,
  parent: i64,    // index into nodes[]; -1 for root
  children: Vec<i64>,  // indices into nodes[]
}

fn add_child(nodes: mut ref Vec<Node>, parent_idx: i64, value: i64) -> i64 {
  let new_idx: i64 = len(nodes) as i64;
  let _ = push(mut ref nodes, Node {
    value: value,
    parent: parent_idx,
    children: vec(),
  });
  // Update parent's children list — borrow + mixed-place assign.
  // (Sketch — actual API needs a helper since you can't take
  // two mut borrows of the same Vec simultaneously.)
  return new_idx;
}
```

This trades the "ergonomic graph node" for:

- **No cycles by construction** — a Node holds indices, not pointers; nothing the verifier needs to prove about lifetimes.
- **Cache-friendliness** — all Nodes live in one contiguous Vec.
- **Cheap clone / serialize** — Vec<Node> is a flat buffer with no internal heap pointers (when fields are Copy).
- **Compile-time bounds checks** — the SMT layer can prove `idx < len(nodes)` for many patterns and elide the runtime guard.

The trade-off is **less ergonomic for tree-traversal-heavy code** —
parent pointer chases become index lookups. For graph algorithms
that fit naturally on a Vec (BFS, DFS, dependency graphs, ECS-style
arrangements) the index pattern is often *more* idiomatic than the
`Rc<RefCell<Node>>` shape Rust would use.

**Two upcoming alternatives** (see [unsafe.md](unsafe.md)) when raw
`i64` indices feel under-specified:

1. **`Handle<T>` into a `Pool<T>`** (v1, Layer 2). Each handle is a
   `(slot_idx, generation)` pair; deleting and recreating a slot
   bumps the generation, so a stale handle's `pool.get(h)` returns
   `None` instead of dangling. Same cache layout as the indexed-Vec
   pattern but with type-safe slot opacity and runtime
   use-after-free detection.

2. **`region { ... }` blocks with `&'arena T` pointers** (v2,
   Layer 5). Cycles are allowed between same-region allocations;
   all references are tagged with the region's lifetime; the
   compiler statically rejects any attempt to keep a reference
   past the region's end. Zero runtime cost; for safety-critical
   workloads (ASIL-D, DO-178C, IEC 62304).

The `i64`-index idiom stays the v1 baseline. Handle<T> and regions
are opt-in evolutions, not replacements.

`dyn Iface` (closure #220–#228) covers the "heterogeneous collection
without enumerating variants" use case that often pushes Rust users
toward `Box<dyn Trait>`. vāṇी offers both shapes: `Vec<dyn Iface>` is
a vector of fat pointers (16 bytes each: vtable + data pointer) when
you want inline storage; `Box<dyn Iface>` (L2 lift Phase 3, shipped)
is a single-owner heap-allocated dyn handle when a struct field needs
a fixed-size slot for one heterogeneous value with its own heap
lifetime.

### What's NOT in the language (deliberate)

- **No garbage collector.** Affine ownership + deterministic Drop
  cover what GC would cover, without the unpredictable pause.
- **`async` / `await` / networking — ✅ FULLY COMPLETE
  2026-06-08 (Arc 8 v1+v1.5+v1.6+v2+v3.1).** Full user-facing
  async + networking + concurrency surface ships:
  - `async fn` / `await(expr)` / `Future<T>` / `Poll<T>` /
    `CancelToken` parse and run with synchronous semantics.
  - `sleep_ms(ms: i64) -> i64` blocking timer (POSIX
    `nanosleep` with EINTR retry).
  - Full blocking TCP family (8 builtins): `tcp_listen` /
    `tcp_socket_port` / `tcp_accept` / `tcp_connect_local` /
    `tcp_send_str` / `tcp_recv` / `tcp_send_buf` / `tcp_close`.
  - Full epoll + non-blocking I/O family (7 builtins):
    `epoll_new` / `epoll_add_read` / `epoll_wait_one` /
    `epoll_close` / `tcp_set_nonblocking` / `tcp_accept_nb`
    / `tcp_recv_nb`. Composes into single-threaded
    cooperative scheduling — one OS thread, N concurrent
    connections, kernel multiplexed.
  - **Two concurrency models, user's choice:**
    1. Thread-per-task via existing `task` + `join` (real
       OS threads, race-free per the affine checker)
    2. Single-thread cooperative via epoll + nb variants
  - Four acceptance examples cross-backend parity-green:
    [examples/async_io.vani](examples/async_io.vani) (timer
    + task fan-out),
    [examples/tcp_echo.vani](examples/tcp_echo.vani) (1
    client),
    [examples/tcp_multi_echo.vani](examples/tcp_multi_echo.vani)
    (3 task clients),
    [examples/tcp_echo_epoll.vani](examples/tcp_echo_epoll.vani)
    (3 clients on ONE thread via epoll reactor).

  **Arc 8 v3.1 sugar — FEATURE-COMPLETE 2026-06-08.**
  The compiler-driven state-machine transform shipped:
  the parser auto-rewrites `async fn` bodies (including
  `await(expr)`, `try EXPR` / postfix `?`, suspend-in-branch
  state-splitting, nested ifs, loops + `break` / `continue`,
  match-with-suspends across every pattern shape, ANF
  lifting, nested async fns, multi-task scheduling, generic
  `async fn`, and non-i64 types across the entire async-fn
  boundary) into state-machine struct/poll/constructor
  triples over the existing epoll reactor. The hand-rolled
  pattern in `tcp_echo_epoll.vani` and `tcp_echo_state_machine.vani`
  still works; users no longer need to write it.
  28 acceptance examples + generic-async smoke are
  cross-backend parity-green. The v3.1 liveness optimization
  also shipped (state-local locals → poll-fn stack lets,
  cutting state-struct width). Explicitly NOT shipping
  Rust-style `Pin<&mut Self>` self-references (those stay
  🛑 NON-COMPLIANT under affine). See [ARC8_V3_PLAN.md](ARC8_V3_PLAN.md)
  for the phased plan-of-record.
- **No reference counting** (no `Rc` / `Arc` equivalent). Single-owner
  affine ownership means cycles can't form; there's nothing for an
  Rc to count.
- **No `unsafe` escape hatch on hosted targets.** Every operation
  on Linux / Windows / macOS goes through the checked surface.
  Embedded / bare-metal targets are the only place
  `unsafe(reason = "...") { ... }` is permitted — explicitly typed,
  reason-string mandatory at parse time, narrowly scoped, and
  rejected by the hosted-build path. See *Embedded targets —
  current position* below and [unsafe.md](unsafe.md) for the
  layered safety net.
- **No exceptions / no stack unwinding.** Errors are values via
  payloaded enums (`Option`-like / `Result`-like) and propagated
  with either the `try EXPR` keyword or the postfix `EXPR?`
  operator (same AST node, two surface spellings — pick whichever
  reads better; the postfix form chains naturally as
  `foo()?.bar()?`). `assert` triggers a deterministic `abort()`.

### Embedded targets — current position (updated 2026-06-21)

vāṇी's v1 target is hosted (Linux / Windows / macOS). Embedded
(`no_std`, bare-metal, MCU) is a **first-class supported target**
(fully supported as of v0.1.6) — shaped by the same affine +
checked-by-default commitments, with a **narrowly scoped
`unsafe { ... }` escape hatch reserved for embedded builds** to
cover the operations the compiler genuinely cannot prove safe.

**Bare-metal native workflow (v0.1.6+):**

```sh
# Cross-compile for ARM Cortex-M (bare-metal, no libc)
vanic build --target=thumbv7m-none-eabi src/main.vani

# Emit no-std C (suppresses all #include <std*.h>)
vanic emit --backend=c --no-std src/main.vani

# Run a Linux cross-binary via QEMU user-mode
vanic run --target=aarch64-linux-gnu src/main.vani
```

Attributes for bare-metal linking:
```vani
#[no_mangle]
#[link_section = ".text.reset_handler"]
fn reset_handler() -> i64 { return 0; }
```

Volatile MMIO (8, 16, and 32-bit):
```vani
let v8:  u8  = mmio_read_u8(0x4000_0000);
let v16: u16 = mmio_read_u16(0x4000_0002);
let v32: u32 = mmio_read_u32(0x4000_0004);
let _ = mmio_write_u8(0x4000_0001, 0xFF);
let _ = mmio_write_u16(0x4000_0003, 0x1234);
let _ = mmio_write_u32(0x4000_0005, 0xDEAD_BEEF);
```

See [`examples/language/english/bare_metal.vani`](examples/language/english/bare_metal.vani)
and `docs/v1_limitations.md` (L19 fully resolved).

- **Pointers in the source language.** None of the raw kind on
  the safe path. The full safe pointer-shaped vocabulary is
  `ref T` / `mut ref T` (second-class; param + `let` + struct-
  field positions, scope-escape checked), `Box<T>` (single-
  owner heap allocation with auto-recursive-drop), `fn(...) -> R`
  function pointers, `dyn Iface` fat pointers, `Box<dyn Iface>`
  (heap-owning single-dyn-value handle), and indices into
  `Vec<T>` for cyclic / graph shapes. There
  is no `*const T` / `*mut T` outside an `unsafe` block. Most
  embedded code is still expected to be written *without* any
  `unsafe` — through the typed embedded primitives below.
- **Explicit `unsafe` block — embedded-only escape hatch.**
  On embedded builds, `unsafe { ... }` is the **opt-in** path
  for the narrow set of operations the compiler cannot
  prove safe:
  - Raw MMIO outside the typed builtins (`mmio_read_u8/u16/u32` /
    `mmio_write_u8/u16/u32`) — e.g. 64-bit wide registers or
    dynamically computed register addresses.
  - Inline assembly and platform intrinsics.
  - `transmute`-style reinterpretation between layout-equivalent
    types (DMA buffer ↔ packed-struct view).
  - Custom linker-placed memory ranges and fixed-address
    peripherals the build target doesn't model.
  - FFI into vendor SDK functions whose signature the checker
    can't verify.

  The default is still "no `unsafe`" — you have to type the
  keyword, and the block is its own lexical scope so the diff /
  audit / grep story stays simple. Reviewers can find every
  unproven operation with `grep -n unsafe`. Hosted-target
  builds reject `unsafe` blocks at parse time — the keyword
  only compiles when `--target` names an embedded triple (or
  `[target] = "embedded"` is set in the manifest).

  *Why permit it at all when hosted forbids it?* Because for
  the user with an embedded background, the question isn't
  "should I write `unsafe`" — it's "can I write this driver
  at all in this language." A vendor SDK callback, a custom
  DMA controller, a peripheral the language doesn't model:
  these need *some* path to a raw load/store, and pretending
  otherwise just means the driver gets written in C and FFI'd
  in (which is strictly more unsafe than `unsafe { ... }` in
  vāṇी, since FFI escapes affine tracking entirely). Better
  to give the user a typed, scoped, audit-friendly hatch.
- **Allocator dependence.** Today `Vec<T>` / `OwnedStr` /
  `+`-concat / heap-allocating str ops require `malloc` and
  abort on OOM (see *Allocator failure* note above). On MCU
  targets `malloc` may not exist. A `no_std` mode gates those
  primitives off and leans on fixed-size arrays, stack-allocated
  strings, and a fallible-allocation API (`try_vec`).
- **What ships for bare-metal (as of v0.1.6):**
  - `--target=<triple>` on `vanic build` / `vanic run` — selects the
    cross-linker, passes `--mtriple` to `llc`, suppresses libc/OpenMP/
    pthread for bare-metal triples automatically.
  - `--no-std` on `vanic emit --backend=c` — strips all `#include <std*.h>`,
    emits a freestanding typedef block; auto-activates for bare-metal triples.
  - `#[no_mangle]` — suppresses the `intent_` prefix and Unicode mangling so
    linker scripts can reference the bare symbol name.
  - `#[link_section = "..."]` — places the function in a named ELF/COFF
    section (vector tables, `.text.isr`, DMA descriptor regions).
  - `mmio_read_u8` / `mmio_write_u8` / `mmio_read_u16` / `mmio_write_u16` —
    volatile 8/16-bit MMIO builtins (joins the existing 32-bit pair).
  - QEMU user-mode transparent run for Linux cross-targets.
  - L19 in `docs/v1_limitations.md` is **fully resolved** (all 5 gaps).
  What remains deferred: interrupt-service-routine calling conventions,
  bit-precise / packed register layouts, worst-case stack-usage bounds,
  inline assembly, `transmute`-style reinterpretation. Design notes live in
  [TODO.md](TODO.md) → *Embedded targets — design considerations*.
- **The goal is to make `unsafe` rare on embedded too.** Three
  already-feasible compile-time extensions cover most of what
  `unsafe` would otherwise be reached for — leaving `unsafe`
  to the genuinely-unprovable residual:
  - **Effect / capability typing** — generalize `pure fn` to a
    set: `allocates`, `blocks`, `may_panic`, `mmio(<region>)`,
    `interrupt_safe`. An `interrupt fn` body can then be
    *statically* forbidden from calling anything `allocates`
    or `blocks`.
  - **Stack-bound proofs** — Z3 is already on hand for bounds
    elision; reuse it to compute worst-case per-function stack
    and reject programs whose call graph exceeds a target's
    stack budget. Requires the no-recursion + no-alloca
    constraints v1 already enforces.
  - **Typestate via phantom generics** — e.g. `Pin<GpioA, Out>`
    vs `Pin<GpioA, In>` as distinct monomorphizations.
    Compile-error if you call `set_high()` on an input pin.
    Zero runtime cost; falls out of the generics machinery
    that's already shipped.

Not a v1 commitment — recorded so the question doesn't get
re-asked from scratch each session.

**Memory-safety story inside `unsafe` (2026-06-02 hybrid plan).** A
two-phase plan lives in [unsafe.md](unsafe.md). v1 (~22–31h, ~12
commits) ships **generational handles** as the default safety net —
`Handle<T>` is a `(slot_idx, generation)` pair; use-after-free is
caught at runtime by the generation mismatch (~3–5 cycles per
dereference on Cortex-M). v2 (~15–25h, ~8 commits, queued after v1
stabilizes) adds **region typing** as a power-user opt-in for
safety-critical certification (ASIL-D, DO-178C, IEC 62304) — zero
runtime cost, compile-time use-after-free proof via `&'arena T`.
The two coexist; users pick per-type. Code written against
`Handle<T>` stays valid forever — no big-bang migration.

**Keyword form: `unsafe(reason = "...") { ... }`** (decided
2026-06-02). The reason clause is mandatory at parse time —
empty strings rejected. The reason is stored on the AST, threaded
through the IR, and emitted as DWARF / object-section metadata so
certification tooling can extract a structured deviation-record
report straight from the compiled artifact. Reviewers don't have
to grep for `// SAFETY:` comments; the compiler enforces the
justification per occurrence. Recommended prefix conventions for
tooling: `"MMIO: ..."`, `"FFI: ..."`, `"DMA: ..."`,
`"transmute: ..."`, `"vendor-SDK: ..."`.

Plan-of-record: [unsafe.md](unsafe.md). Status (2026-06-02):
**fully shipped** — Layers 1.1 / 1.2 / 1.3 / 2.1 / 2.2 / 3.1 /
3.2 / 4.1 / 4.2 / 5 (foundation + lifetime-tagged `ArenaRef<T>` +
`region <name> { ... }` block sugar) all on `main`.

**Safety-standard alignment — SHIPPED (2026-06-03).** Two-tier
attribute system bringing MISRA C 2012, ISO 26262 ASIL-D,
DO-178C Level A, and IEC 62304 Class C feasibility. All three
tiers + four standard composites are on `main`:

- **Feature primitives** (orthogonal, composable, all shipped):
  `#[bounded(N)]`, `#[no_heap]`, `#[no_float]`, `#[no_nan]`, `#[no_recursion]`,
  `#[interrupt]`, `#[bounded_stack(bytes = N)]`,
  `#[wcet(cycles = N)]`, `#[deterministic_timing]`. Each enforces
  one constraint compiler-side via a dedicated `safety::enforce_*`
  pass (call-graph fixpoint for `no_heap`, BFS cycle detection
  for `no_recursion`, static NaN-contract scan for `no_nan`,
  coarse cycle estimator for `wcet`, etc.).
- **Standard composites** (hardcoded aliases that expand to
  primitive sets, all shipped): `#[misra_c_2012]`, `#[asil_d]`,
  `#[do178c_level_a]`, `#[iec_62304_class_c]`. Two composite
  tags on the same fn rejected (stack primitives instead).
- **MISRA 13.5 tightening (T3.5):** `&&` / `||` whose RHS contains
  a non-pure function call is rejected for `pure fn` and any
  function with a standard-composite tag (short-circuit
  evaluation would make the side effect order-dependent).
- **MMIO volatile load/store (T2.1):** `mmio_read_u8` / `mmio_write_u8` /
  `mmio_read_u16` / `mmio_write_u16` / `mmio_read_u32(addr)` /
  `mmio_write_u32(addr, v)` builtins emit a `volatile` qualifier
  in both backends — required for peripheral registers where
  reads clear IRQ flags and writes latch state. The 8 and 16-bit
  widths ship as of v0.1.6.
- **Compose by union — most restrictive wins.** Composites set
  a baseline; primitives tighten further.
- **Opt-in by design.** Without any tag, vāṇี behaves exactly
  as today (no compile-time perf or behavior change). With a
  tag, the marked function is held to that standard's
  constraints; with the matching global env var
  (`INTENT_NO_HEAP=1`, …), the entire program is held.
- **Audit-artifact CLIs:**
  - `intentc deviations <file>` walks every
    `unsafe(reason = "…")` block and emits a structured record
    (CSV / JSON / human-readable text) with each row tagged by
    the enclosing fn's `target_standard`. The deviation-record
    format ASIL-D / DO-178C reviewers need for sign-off.
  - `intentc stack-depth <file> [--max=N] [--entry=fn]` — per-fn
    frame-size estimates + call-chain max stack depth.
    `#[inline]`-annotated callees have their locals folded into
    the caller's frame rather than a separate push. `--max` is a
    CI-friendly hard gate.
  - `intentc acyclicity <file>` (T3.3) — Tarjan SCC over the call
    graph; reports every cycle. `#[bounded(N)]`-tagged self-loops
    are exempt; everything else exits 1.
  - `intentc coverage <file> [--format=text|json|csv]` — MC/DC
    coverage point map: every `if`/`while`/`assert` decision
    decomposed into atomic sub-conditions. Feed to external test
    harnesses for DO-178C MC/DC evidence.
  - `intentc complexity <file> [--max=N]` — cyclomatic complexity
    per function; exits 1 if any function exceeds `--max`.
  - `intentc safety-attrs <file>` — per-function safety annotation
    table (CSV / JSON / text) for audit dashboards.
- **Concurrency safety passes:**
  - **S-19 — lock-order deadlock detection:** held-set transitive analysis
    (`build_lock_edges`) walks every function and its callees; records
    ordering edges from every currently held lock to each newly acquired
    lock; DFS cycle detection fires `[S-19] potential deadlock` for every
    ordering cycle. Detection is **fully transitive** — callee lock sequences
    are inlined with correct RAII release semantics (L20, fixed 2026-07-12).
  - **S-20 — ISR priority inversion:** `#[interrupt(priority=N)]`
    tracks ISR urgency levels; fires `[S-20] potential priority
    inversion` when two ISRs at different priorities transitively acquire
    a mutex with the same variable name — including mutexes locked inside
    helpers called from the ISR (L21, fixed 2026-07-12).
- **Adversarial test suite:** `tests/safety_adversarial.rs` — 31
  integration tests that probe each safety pass with non-obvious
  inputs: transitive float, indirect recursion, returns buried in
  nested branches, both lock orderings in a single function's if/else
  branches, transitive lock cycles through helpers, ISR mutexes via
  helpers, non-adjacent duplicate call args. All 31 pass.

Plan-of-record + sub-step ledger: [docs/TODO_SAFETY.md](docs/TODO_SAFETY.md)
(all 27 items complete; L20–L22 scope gaps closed 2026-07-12). See
[docs/v1_limitations.md](docs/v1_limitations.md) for historical gap
records. ARC work is active; see [ARCS.md](ARCS.md).

### Examples — what the compiler rejects

These programs all **fail to compile on the safe path** — i.e.
outside any `unsafe(reason = "...") { ... }` block. The diagnostic
text below each is what the user actually sees today (test-pinned
in `src/lib.rs`).

> **Caveat for embedded targets:** inside `unsafe(reason = "...")`,
> raw `*T` operations can violate any of these invariants — the
> user takes responsibility per the documented reason string.
> Runtime safety nets in v1 (canaries, generational handles,
> bounds checks) catch many but not all such bugs; see
> *What runs inside `unsafe(reason = "...")`* above and
> [unsafe.md](unsafe.md) for the full layered plan.

#### Heap leak — impossible by construction

```vani
fn main() -> i64 {
  let v: Vec<i64> = vec(1, 2, 3);
  return 0;
  // v's heap buffer freed automatically at scope exit.
  // No `forget(v)` exists. No way to leak it.
}
```

#### Double-free — rejected via move tracking

```vani
fn main() -> i64 {
  let v: Vec<i64> = vec(1, 2, 3);
  let w: Vec<i64> = v;   // move: w now owns the buffer
  let z: Vec<i64> = v;   // ERROR: value 'v' was moved
  return 0;
}
```

#### Use-after-free — same machinery

```vani
fn consume(xs: Vec<i64>) -> u64 { return len(xs); }

fn main() -> i64 {
  let v: Vec<i64> = vec(1, 2, 3);
  let n: u64 = consume(v);   // move into consume()
  return v[0];               // ERROR: value 'v' was moved
}
```

#### Aliasing — mutable + shared borrow rejected

```vani
fn read(xs: ref Vec<i64>) -> i64 { return xs[0]; }
fn write(xs: mut ref Vec<i64>) -> i64 { xs[0] = 99; return 0; }

fn main() -> i64 {
  let v: Vec<i64> = vec(1, 2, 3);
  let _ = write(mut ref v);
  let _ = read(ref v);   // (OK — sequenced, not aliased)
  // The compiler rejects holding both borrows simultaneously.
  return 0;
}
```

#### Unjoined task — thread leak rejected

```vani
fn main() -> i64 {
  task worker {
    let _ = 42;
  }
  return 0;
  // ERROR: task handle 'worker' was never consumed by `join`
}
```

#### Forgotten mutex unlock — impossible by construction

```vani
fn main() -> i64 {
  let m: Mutex<i64> = mutex_new(0);
  {
    let g: Guard<i64> = mutex_lock(ref m);
    // ... critical section ...
  }   // Guard 'g' drops here; mutex unlocked automatically.
  return 0;
}
```

#### Impure operations inside `parallel for` — rejected

```vani
fn main() -> i64 {
  parallel for i from 0 to 3 {
    print i;   // ERROR: 'parallel for' body cannot contain `print`
               //        (observable I/O is a side effect)
  }
  return 0;
}
```

The same diagnostic fires for calls to impure functions, non-Copy
moves into the body, and indexed writes on captured arrays / `Vec`s.

#### Implicit reduction race — rejected (closure #259)

```vani
fn main() -> i64 {
  let total: i64 = 0;
  parallel for i from 0 to 100 {
    total = total + i;
    // ERROR: 'parallel for' body mutates captured variable 'total'
    //        without declaring it as a reduction; this races at
    //        runtime. Add `reduce total with <op>;` before the body,
    //        or use `Atomic<T>` for a concurrent counter.
  }
  return total;
}
```

The fix is to declare the reduction explicitly:

```vani
fn main() -> i64 {
  let total: i64 = 0;
  parallel for i from 0 to 100
  reduce total with +;
  {
    total = total + i;   // OK: declared reduction. SSA LLVM (default):
                         //     per-thread local accumulator + one
                         //     atomicrmw per thread at loop exit.
                         //     C backend: OpenMP `reduction(+:total)`.
  }
  return total;
}
```

Body-local mutations are still per-iteration and free:

```vani
fn main() -> i64 {
  parallel for i from 0 to 5 {
    let tmp: i64 = i;
    let next: i64 = tmp + 1;   // per-iteration, body-local — fine.
    let _ = next;
  }
  return 0;
}
```

See [examples/memory_safety.vani](examples/memory_safety.vani) for
the seven canonical patterns exercised end-to-end (affine Vec
ownership, explicit clone, push/pop stack, OwnedStr drop, user
Drop, parallel-for reduction, task + join), and
`src/lib.rs::tests` for the negative-test coverage (search for
`expect_err`, `use_after_move`, `double_free`, `unjoined_task`, etc).

### Known gaps (will become checks later)

These are real-world concerns that are **not yet caught at compile
time** — listed honestly so users can plan around them:

- **Recursion stack overflow.** vāṇī doesn't bound recursion depth.
  Deep call chains can blow the OS stack at runtime. Future work
  could add a recursion-depth analysis or a `#[bounded(N)]`
  annotation.
- **Mutex deadlock.** Lock-acquisition-order analysis is not yet
  implemented. Two threads taking the same two mutexes in opposite
  order can deadlock at runtime. Future work: a deadlock-free lock
  ordering pass (Rust doesn't catch this either).
- **Allocator failure (OOM).** vāṇī uses the standard allocator
  (`malloc` / LLVM's allocator); on OOM the program aborts. No
  fallible-allocation API yet.
- **Channel deadlock.** Bounded MPSC channels can wedge if every
  sender + every receiver is blocked. Today this manifests as a
  runtime hang rather than a static error.
- **Integer division by run-time-zero divisor.** When SMT can't
  prove the divisor non-zero, the elision pass leaves the runtime
  guard in (abort on zero). Compile time catches the *provable*
  cases; runtime catches the rest. Same for shift amount validity.
- **Integer arithmetic wrapping (runtime).** `i64::MAX + 1`,
  `i64::MIN - 1`, and `i64::MIN * -1` silently wrap (two's
  complement) on both backends — no overflow guard is emitted at
  arithmetic op sites. *Compile-time* constant overflow IS caught
  (`const N: i64 = 9223372036854775807 + 1` is a type error).
  For runtime safety, constrain operand ranges with `requires`
  clauses so the SMT pass can statically prove they can't wrap.
  Treat all unbounded runtime `i64`/`u64` arithmetic as wrapping
  until runtime guards land in a future pass.

- **Generic function calling another generic function.** The
  monomorphizer is single-pass: it only collects specialization
  requests from non-generic call sites. When a specialized
  `g__i64` body calls another generic `f<T>`, `f__i64` is never
  generated and the build fails with a diagnostic. Workaround:
  ensure every generic function is called directly from a
  non-generic function (flatten the generic call chain). See
  `docs/missing_features.md` for the detailed pattern.

The first two items are the most interesting research directions
for the next year. The rest are likely runtime-aborts-with-clean-
diagnostic forever (which is the same boat as Rust).


---

# Part IV — Language Reference

The chapters in this part are organized as a top-to-bottom
reference: numeric rules first (foundational), then composite
types (arrays, vectors, strings, references), then control
flow + scoping, then verification (SMT), then the higher-level
constructs (modules, effects + concurrency).

Each `##` heading is its own chapter; jump to the one you
need.

## Comments

vani supports two comment forms:

- **Line comments** -- `// text to end of line`. Most common for short annotations.
- **Block comments** -- `/* text */`. Span any number of lines; nest to any depth.
  The closing `*/` pairs with the *innermost* open `/*`, so you can comment out
  a block that already contains block comments.

```vani
// single-line comment

/* single-line block comment */

/* outer
   /* inner -- still inside outer */
   back in outer
*/

let x: i64 = /* inline annotation */ 42;

/**/  /* empty block comment -- valid */
```

Forgetting the closing `*/` is a compile-time error (`unterminated block comment`).

## Integer Rules

Arithmetic operators `+`, `-`, `*`, `/`, and `%` work on integer operands. The
compiler chooses a common result type before checking the expression:

- `i32 + i64` becomes `i64`
- `u32 + u64` becomes `u64`
- `i64 + u32` becomes `i64`, because `i64` can represent every `u32` value
- `i32 + u64` is rejected for now, because neither side can safely represent
  all values from the other side

This is intentionally more conservative than C. A verification-oriented
language should not silently convert `-1` into a huge unsigned value.

Integer constants are flexible until they are assigned or combined with a typed
operand, so these are valid:

```intent
let tiny: u8 = 42;
let wider: i64 = tiny + 1000;
```

But these are rejected at compile time:

```intent
let bad_div = 10 / 0;
let too_large: u8 = 250 + 10;
```

`%` is integer-only. A zero divisor is rejected at compile time when known, and
the C backend emits a runtime assertion around non-constant divisors.

## Float Rules

`f32` is single precision and `f64` is double precision. Float arithmetic works
with signed and unsigned integers:

- `f32 + u32` becomes `f32`
- `f64 + i64` becomes `f64`
- `f32 + f64` becomes `f64`
- a flexible literal such as `3.0` can adapt to a surrounding `f32`

Float constants must stay finite. The compiler rejects constant division by
zero and constant results that become `NaN` or infinity in the target type.
Non-constant float divisors are protected by emitted runtime assertions.

## Casts

Use `as` for explicit numeric casts:

```intent
let wide: u64 = (count as u64) + total;
let precise: f64 = (single as f64) + 2.25;
```

Implicit casts are inserted only when the checker considers them safe for this
prototype. Explicit casts are represented in the typed IR and emitted as C casts,
so generated code makes conversions visible instead of relying on C defaults.

`as` between integer widths is **two-complement truncation** — it wraps,
not saturates, and never errors at compile time on overflow. `200 as i8`
folds to `-56` (the low 8 bits sign-extended), `-1 as u64` folds to
`u64::MAX`, etc. The user opted in to the cast by writing `as`, so the
compile-time fold matches what the emitted code does at runtime. Implicit
coercion (e.g. `let y: i8 = 200;` without `as`) still range-checks and
rejects — only explicit `as` wraps.

## Shift and bitwise rules

`<<` and `>>` work on integers. The left operand determines the result type:

```intent
let bits: u8 = 1 as u8;
let shifted: u8 = bits << 3;
```

The shift count must be non-negative and smaller than the bit width of the left
operand. Known-bad counts such as `(1 as u8) << 8` are compile-time errors, and
the C backend emits runtime assertions for non-constant counts. `>>` is
arithmetic for signed integers and logical for unsigned integers.

Bitwise `&`, `|`, and `^` are integer-only (floats and bools are rejected;
bools have their own logical `&&` and `||`). Precedence follows Rust:
shifts bind tighter than `&`, which binds tighter than `^`, which binds
tighter than `|`, which sits above comparisons. `a == b | c` therefore
parses as `a == (b | c)`. The unary prefix `&` (taking a reference) is
disambiguated by position: only the infix context picks up the new
bitwise binding.

Runtime overflow checks and non-constant proof obligations belong in the next
verification pass. Today, constant mistakes are prevented by the compiler,
risky runtime divisors/counts are asserted in generated C, and richer safety can
be expressed with `requires`, `assert`, and later SMT proofs.

`requires` clauses are currently lowered to runtime `assert` calls in the
emitted C; they will become verification obligations once the SMT pipeline
lands.

`prove` is discharged in three layers, tried in order:

1. **Constant folding** — compile-time-known boolean true.
2. **Structural tautologies** — `x == x`, `!(x != x)`, `x <= x`, etc.
3. **SMT verifier** — encodes the claim plus all in-scope `requires` clauses
   as an SMT-LIB query and asks an external solver (z3) whether the negation
   is unsatisfiable. **Integer types are encoded as fixed-width
   `(_ BitVec N)`**, so overflow is faithfully modeled — `prove x + 1 > x;`
   for `x: i64` is correctly rejected with the counterexample
   `x = 9223372036854775807` (INT64_MAX, where the sum wraps). Comparisons
   pick the signed (`bvslt`/`bvsge`) or unsigned (`bvult`/`bvuge`) form
   from each variable's type. Floats use `(_ FloatingPoint 8 24)` /
   `(_ FloatingPoint 11 53)` with `fp.add`/`fp.lt`/`fp.eq` and `RNE`
   rounding. Integer casts use `sign_extend`/`zero_extend`/`extract`;
   int→float and float→float use `to_fp`. Shifts, array/Vec/reference
   operations and function-call results fall outside the v1 encoder and
   produce a "skipped" diagnostic.

For step 3 to work, install z3 and ensure it's on `$PATH` (or point `$Z3`
at the binary). Without z3, the verifier falls back to layers 1–2 and
reports "no SMT solver available" when those don't suffice.

When z3 returns `sat`, the diagnostic includes a **counterexample**
extracted from z3's model — e.g.
`proof failed: SMT counterexample [x = 0, y = 0]` for `prove x + y > x`.
The model parser handles z3's typical output forms (negative integers via
`(- N)` flatten to `-N`); Vec-length witnesses appear as `len(xs) = …`.

## Numeric Literals

Integer literals may use `_` as a digit separator, and the prefixes `0x`/`0X`,
`0b`/`0B`, and `0o`/`0O` for hex, binary, and octal. Examples:

```intent
let big: i64 = 1_000_000;
let mask: u16 = 0xFF_FF;
let bits: u8  = 0b1010_1010;
```

## Arrays and Ownership

Fixed-size arrays live on the stack and carry their length in the type:

```intent
let xs: [i64; 4] = [10, 20, 30, 40];
let n: u64       = len(xs);   // n == 4
let first: i64   = xs[0];
```

Arrays are **affine** — they are owned by a single binding at a time. Passing
an array to a function or assigning it to another `let` moves it; the source is
unusable after. Numeric primitives stay `Copy` and behave as before:

```intent
fn sum_four(xs: [i64; 4]) -> i64 {
  return xs[0] + xs[1] + xs[2] + xs[3];
}

fn main() -> i64 {
  let xs: [i64; 4] = [1, 2, 3, 4];
  let total = sum_four(xs);    // xs is moved here
  // let bad = xs[0];           // error: 'xs' was moved on the line above
  print total;
  return 0;
}
```

Array element types accept Copy primitives, structs, and tuples. Nested
arrays (`[[i64; 4]; 3]`) and `[Vec<_>; N]` are still gated — the SSA layer's
by-value-element-load path doesn't handle them yet. Array return types are
also rejected (clean diagnostic).

Bounds checks at `xs[i]` are runtime by default. When the index is a
compile-time integer constant in range, the check is elided and the C backend
emits a direct index. Out-of-range constant indices are compile errors.

## Vectors

`Vec<T>` is a heap-allocated, dynamically-sized owned collection. Like arrays,
it is **affine** (moved on use, dropped at end of scope). Element types must be
`Copy`. The four built-in operations are:

```intent
let xs: Vec<i64> = vec(10, 20, 30);
let xs           = push(xs, 40);     // consumes old xs, returns new Vec
let xs           = set(xs, 0, 99);   // functional update; returns new Vec
let ys           = clone(xs);        // independent copy; xs stays usable
let n: u64       = len(xs);          // runtime length
let first        = xs[0];            // always runtime bounds-checked
```

Notes:

- `push` and `set` consume their first argument; `clone` deliberately does not.
- `let` shadowing is the natural way to express functional update — the new
  binding must have the same type as the old.
- Buffers are freed automatically: when a `Vec` binding is shadowed without
  being consumed, or when it falls out of scope at function return without
  being returned.
- Returning a `Vec` from a function transfers ownership to the caller; no
  destructor runs at the callee site.
- The built-in names `vec`, `push`, `set`, and `clone` cannot be redefined as
  user functions.
- `vec()` with zero arguments is supported (empty Vec).
- `Vec<T>` accepts non-`Copy` elements: `Vec<Vec<T>>`, `Vec<[T; N]>`,
  `Vec<OwnedStr>`, and `Vec<Struct>` all work. Reading a non-Copy slot into a
  binding requires `clone_at(ref xs, i)` — bare `let inner = xs[i]` would alias
  the owner's slot and double-free, so the checker rejects it with a hint
  pointing at `clone_at`. At scope exit the Vec's `__free` helper walks every
  live element and drops its owning resources before releasing the buffer, so
  `Vec<OwnedStr>` and `Vec<Struct{…OwnedStr / Vec…}>` don't leak their
  per-element heaps.

Under the hood, the backend monomorphizes one C struct + helper bundle per
distinct element type used:

```c
typedef struct { int64_t* data; uint64_t len; uint64_t capacity; } intent_vec_int64_t;
static intent_vec_int64_t intent_vec_int64_t__push(intent_vec_int64_t xs, int64_t v);
static void intent_vec_int64_t__free(intent_vec_int64_t xs);
// ... etc
```

In-place reuse for `push`/`set` falls out for free: affine ownership
guarantees that `xs` is unique at the call site, so the helpers can mutate the
underlying buffer (and `realloc` it) without violating any aliasing
invariants.

## Strings

Two distinct types share the language's string surface:

- **`Str`** — borrowed, `Copy`, NUL-terminated. Models a pointer to
  either a static string literal or someone else's buffer. Supports
  `==`/`<`/etc. (via `strcmp`), `len(s)` (via `strlen`), passing to
  parameters, comparisons, etc. Always safe to re-use.
- **`OwnedStr`** — heap-allocated, NUL-terminated, **affine**.
  Produced by the `+` concat operator. The compiler tracks
  ownership through moves and inserts a runtime `free` at the end
  of every scope where an `OwnedStr` binding is still live, or
  whenever the value is moved into another concat / a return /
  another scope.

```intent
fn greet(name: Str) -> OwnedStr {
  return "Hello, " + name;   // fresh heap buffer
}

fn main() -> i64 {
  let g: OwnedStr = greet("alice");
  let banged: OwnedStr = g + "!";   // consumes `g`; `g` is now moved
  print banged;                     // freed at end of scope
  return 0;
}
```

The runtime helper `intent_str_concat(l, l_owned, r, r_owned)`
mallocs `strlen(l) + strlen(r) + 1` bytes, memcpys both operands,
NUL-terminates, and frees whichever operand had `*_owned == 1`
before returning the joined buffer. Mixing `Str` and `OwnedStr`
operands in either position works — the `_owned` flag is `0` for
`Str` (borrowed) and `1` for `OwnedStr`.

`len(s)` works for both types and dispatches to `strlen`. The
ordering / equality comparison operators (`==`, `!=`, `<`, `<=`,
`>`, `>=`) accept any combination of `Str` and `OwnedStr` operands
— the `OwnedStr` side is auto-borrowed (the comparison only reads,
so the binding stays live for its scope-end drop). Function
arguments do the same: passing an `OwnedStr` where a `Str`
parameter is expected works and leaves the caller's binding
untouched.

## References

When a function only needs to *read* a `Vec` or array, take a shared reference
instead of consuming the value:

```intent
fn sum(xs: ref Vec<i64>) -> i64 {
  return xs[0] + xs[1] + xs[2];
}

fn main() -> i64 {
  let xs: Vec<i64> = vec(1, 2, 3);
  let total: i64 = sum(ref xs);    // borrow; xs is not consumed
  let first: i64 = xs[0];          // still usable
  return 0;
}
```

Mutable references (`mut ref T`) allow in-place updates through the borrow:

```intent
fn bump(p: mut ref Point) -> i64 {
  p.x = p.x + 1;
  return p.x;
}

fn main() -> i64 {
  let p: Point = Point { x: 0, y: 0 };
  return bump(mut ref p);
}
```

References are **second-class** by design — keyword-first syntax (no `&`,
no `&mut`):

- Type spelling: `ref T` (shared), `mut ref T` (mutable). Rust-style
  `&T` / `&mut T` is rejected.
- Borrow expression: `ref x` / `mut ref x` at call sites. The inner
  expression must be a variable; function-call results and temporaries can't
  be borrowed.
- Allowed *only* as function parameter types (and method `self:` receivers).
  Forbidden as return types, `let` annotations, aggregate elements, and
  nested inside another reference.
- Auto-deref inside the callee: `xs[i]`, `len(xs)`, `p.field`,
  `recv.method()` all work without explicit dereferencing.
- Re-borrow is transparent — passing a `ref T` parameter directly to
  another function expecting `ref T` works.
- Aliasing rejected at call sites: a call cannot pass `mut ref x` alongside
  any other reference to `x`, and cannot pass a moved `x` alongside any
  borrow of `x`.

C lowering: `ref Vec<T>` becomes `const intent_vec_T*`; `mut ref Vec<T>`
becomes `intent_vec_T*`; `ref [T; N]` and `ref i64` become `const T*`.
Auto-deref expands to `(*xs).field` on the Vec case; array-by-pointer uses C
array decay so `xs[i]` continues to work syntactically.

## Control Flow

`if` / `else` / `while` are statements, and a plain `name = expr;` reassigns
an existing binding without redeclaring it:

```intent
fn sum(xs: &Vec<i64>) -> i64 {
  let total: i64 = 0;
  let i: u64 = 0;
  let n: u64 = len(xs);
  while i < n {
    total = total + xs[i];
    i = i + 1;
  }
  return total;
}

fn abs(x: i64) -> i64 {
  if x < 0 {
    return 0 - x;
  } else {
    return x;
  }
}
```

Rules:

- The condition of `if` and `while` must be `bool`.
- Branches share the parent's scope (no nested lexical scope yet). Bindings
  *declared* inside a branch persist after; for affine types, they must be
  consumed or visible in the post-merge state.
- Affine **move-state must reconcile at merges.** If `xs: Vec<T>` is moved in
  one branch of an `if` but not the other, the checker errors and asks you to
  consume or rebind it in both branches.
- For `while`, the body must leave every outer affine binding in the same
  move-state it started in. The natural pattern is to consume-then-rebind:
  `let xs = push(xs, i);` consumes the old `xs` and immediately reassigns it,
  so the body is balanced.
- `return` inside a branch terminates that path; an `if`/`if-else` where every
  path returns is itself terminating, and counts toward the function's
  "must return" obligation.
- Code after a guaranteed `return` (or after an `if-else` where both branches
  return) is rejected as unreachable.
- `name = expr;` requires `name` to be an existing binding; the RHS is coerced
  to its declared type. For affine bindings the old buffer is freed before
  the new value is installed (just like `let`-shadowing).

### Loop control: `break` / `continue`

```intent
fn find_first_negative(xs: &Vec<i64>) -> i64 {
  let i: u64 = 0;
  let result: i64 = 0 - 1;
  while i < len(xs) {
    if xs[i] < 0 {
      result = xs[i];
      break;
    }
    i = i + 1;
  }
  return result;
}
```

- `break;` exits the innermost `while`. `continue;` jumps to the next
  iteration. Both are rejected outside a loop.
- The move-state-balance rule extends to jump points: at any `break`,
  `continue`, or natural fall-through, every outer non-`Copy` binding must be
  in the same move state it had at loop start. So if you `take(xs)` inside
  the body, you must `let xs = ...;` (or `xs = ...;`) before any reachable jump
  out of the loop.
- After an `if`/`while`, the checker conservatively clears compile-time
  constant tracking for all bindings in scope. This avoids unsound `prove`
  discharge when branches mutate values; it's slightly over-conservative
  (constants that survived unchanged are also cleared), and is a known
  follow-up.

### Named loop labels

Put a plain identifier followed by `:` directly before a `for` or `while`
keyword to name that loop. `break name;` exits the named loop and everything
nested inside it. `continue name;` skips to the named loop's next iteration,
bypassing all remaining code in that loop's body (including inner loops).

```vani
outer: for i from 0 to 5 {
  middle: for j from 0 to 5 {
    inner: for k from 0 to 10 {
      if k == 3 { break inner; }     /* exits k-loop only */
      if j == 2 { continue outer; }  /* skips to next i; middle + inner exit */
      if i == 4 { break middle; }    /* exits middle + inner; i continues */
    }
  }
}
```

| Statement | Effect |
|-----------|--------|
| `break inner` | exits k-loop only |
| `break middle` | exits middle-loop + k-loop |
| `break outer` | exits all three loops |
| `continue inner` | next k iteration |
| `continue middle` | next j iteration (skips remaining k iterations) |
| `continue outer` | next i iteration (skips remaining j and k iterations) |

Rules:
- Labels are plain identifiers -- any valid identifier works (`outer`, `search`, `retry`, ...).
- Using a label that does not name any enclosing loop is a **compile-time error**.
- Plain `break;` / `continue;` still target the innermost loop.
- Works on `while` loops identically to `for` loops.

### Lexical scoping

Every `if`/`else`/`while` body opens a new scope:

```intent
fn main() -> i64 {
  let counter: i64 = 0;
  let i: i64 = 0;
  while i < 4 {
    let local: Vec<i64> = vec(i, i + 1, i + 2);   // declared in loop body scope
    if local[0] >= 1 {
      counter = counter + 1;                       // mutates outer counter
    }
    i = i + 1;
  }
  // `local` is not visible here; its buffer was freed each iteration.
  assert counter == 3;
  return 0;
}
```

Rules:

- `let x = …` inside an inner scope introduces a **new** binding for the
  duration of that scope. If the outer scope already has a binding called `x`,
  the inner one shadows it (possibly with a different type) and the outer
  binding is restored when the inner scope ends.
- To **mutate** an outer binding from inside an inner scope, use plain
  assignment `x = …;`. Plain assignment finds `x` via lookup that walks the
  scope stack and updates the binding wherever it lives.
- Bindings declared inside `if`/`while` bodies are dropped automatically at
  the end of their scope. For `Vec<T>` (heap-owned), this emits an
  `intent_vec_T__free` call before the C `}` closes.
- `break` and `continue` insert drop calls for every non-`Copy` live binding
  in scopes opened inside the loop body, in deepest-first order, before the
  C `break;`/`continue;`.

If you used to write `let xs = push(xs, i);` inside a loop body to mutate an
outer `xs`, you must now write `xs = push(xs, i);` — the `let` form
introduces a new inner `xs` that goes away at iteration end, which is almost
never what you wanted.

## Mutable references and indexed writes

When a function needs to *modify* a `Vec` or array element in place, take a
mutable reference and use indexed assignment:

```intent
fn double_each(xs: &mut Vec<i64>) -> u64 {
  let i: u64 = 0;
  while i < len(xs) {
    xs[i] = xs[i] * 2;
    i = i + 1;
  }
  return len(xs);
}

fn fill(xs: &mut [i64; 4], v: i64) -> i64 {
  let i: u64 = 0;
  while i < 4 {
    xs[i] = v;
    i = i + 1;
  }
  return v;
}

fn main() -> i64 {
  let xs: Vec<i64> = vec(1, 2, 3, 4);
  let n: u64 = double_each(&mut xs);
  assert n == 4;
  assert xs[3] == 8;

  let ys: [i64; 4] = [0, 0, 0, 0];
  let _ = fill(&mut ys, 9);
  assert ys[0] == 9;
  return 0;
}
```

Rules:

- `&mut T` is a parameter-only type (same second-class constraint as `&T`).
  No `&mut` returns, no `&mut` let-bindings.
- `&mut x` borrows `x` mutably for the duration of the call. The source must
  be a variable — and **not** itself a shared `&T` (you cannot upgrade an
  immutable borrow to a mutable one). Owned bindings and `&mut T` parameters
  are mutably-borrowable.
- `xs[i] = v;` writes through the subscript. Allowed when `xs` is owned
  (`[T;N]` or `Vec<T>`) or when `xs` is `&mut [T;N]` / `&mut Vec<T>`.
  Writing through `&T` is rejected.
- Bounds are checked at runtime, with the same compile-time elision for
  constant-in-range indices on owned arrays. Constant out-of-range writes
  are compile errors.
- **Aliasing rule (call-site):** within a single function call, the argument
  list cannot pass `&mut x` together with any other reference to `x`, and
  cannot pass a moved `x` together with any borrow of `x`. Multiple `&x`
  borrows of the same variable in one call are fine. Detection is purely
  syntactic at call sites (sound for second-class refs, since they can't
  escape the call).

C lowering: `&mut Vec<T>` becomes `intent_vec_T*` (no `const`); `&mut [T;N]`
becomes `T*`. The reading auto-deref for `xs[i]` / `len(xs)` works through
`&mut` exactly as it does through `&`.

### `for` loops over integer ranges

```intent
fn sum_squares(n: i64) -> i64 {
  let total: i64 = 0;
  for i in 1..n {
    total = total + i * i;
  }
  return total;
}
```

- Syntax: `for var in start..end { body }`. Both bounds must be integers;
  flexible-literal bounds adapt to the typed bound (`for i in 0..len(xs)`
  with `len(xs): u64` types `i` as `u64`).
- The loop variable is scoped to the body. Each iteration runs with the
  current value; the post-step increments by 1 before the next condition
  check, so `continue` correctly advances the counter (unlike a hand-rolled
  `while`).
- Move-balance rules and scope rules for nested let/break/continue work
  identically to `while`.

### Iterating arrays and Vecs

Use `for x in &xs { body }` to walk an array or Vec without consuming it:

```intent
fn sum(xs: &Vec<i64>) -> i64 {
  let total: i64 = 0;
  for x in &xs {
    total = total + x;
  }
  return total;
}

fn max5(xs: &[i64; 5]) -> i64 {
  let best: i64 = xs[0];
  for x in &xs {
    if x > best {
      best = x;
    }
  }
  return best;
}
```

Rules:

- The source `xs` after `&` must be a variable bound to an owned `[T; N]` /
  `Vec<T>` or a borrow `&[T; N]` / `&mut [T; N]` / `&Vec<T>` / `&mut Vec<T>`.
- Element type `T` must be `Copy` (current Vec/array constraint).
- The element variable `x` is bound only inside the loop body; the
  collection is borrowed for the loop and is not moved.
- `break` and `continue` work as in any other loop.
- Lowering: each iteration C-binds `x = xs[i]` (arrays) or `x = (*xs).data[i]`
  (Vec / &Vec) for a synthesized index variable.

**Consuming form**: `for x in xs { body }` (no `&`) moves `xs` into the
loop. For `Vec<T>` the backend frees the buffer immediately after the
loop body, so it's the natural pattern for "process every element then
discard the collection". The source must be an owned `Vec<T>` or `[T; N]`
binding — consuming a `&T` or `&mut T` parameter is rejected (use the
borrow form). After the loop, `xs` is moved; any subsequent use is a
compile error with a related note pointing at the `for` line.

## Error handling

vani has **no exceptions, no `catch`, and no stack unwinding**. All errors
are ordinary values carried in payloaded enums. Every failure path is
statically visible -- no call can silently unwind past your frame.

### `Option<T>` and `Result<T, E>`

The compiler injects two generic error-carrier types into every program via
the prelude:

```vani
enum Option<T> { Some(T), None }
enum Result<T, E> { Ok(T), Err(E) }
```

Use `Option<T>` when a value may simply be absent; `Result<T, E>` when the
error carries diagnostic information.

```vani
fn safe_div(a: i64, b: i64) -> Option<i64> {
  if b == 0 { return Option.None; }
  return Option.Some(a / b);
}

fn parse_port(s: Str) -> Result<i64, Str> {
  let n: Option<i64> = parse_int(s);
  match n {
    Option.Some(v) then {
      if v < 1 || v > 65535 { return Result.Err("out of range"); }
      return Result.Ok(v);
    },
    Option.None then return Result.Err("not a number"),
  }
}
```

### `match` -- recovering from errors

`match` is the primary way to handle (i.e. "catch") a failure arm:

```vani
match safe_div(10, x) {
  Option.Some(v) then print "quotient = ", v,
  Option.None    then print "division by zero",
}

match parse_port("8080") {
  Result.Ok(port)  then start_server(port),
  Result.Err(msg)  then { eprint "bad port: ", msg; return 1; },
}
```

### `try` / `?` -- propagating errors upward

`try EXPR` (equivalently, postfix `EXPR?`) propagates the failure arm by
returning early from the current function. The function's own return type
must be Option- or Result-compatible.

```vani
fn run() -> Option<i64> {
  let a: i64 = try safe_div(100, x);  /* None? return Option.None immediately */
  let b: i64 = try safe_div(a, y);
  return Option.Some(a + b);
}
```

`try` and `?` are the same AST node -- identical semantics, two spellings.
Works for both `Option` (propagates `None`) and `Result` (propagates `Err(e)`
as `Result.Err(e)`).

### `assert` -- non-recoverable invariant violations

`assert cond` / `assert cond, "msg"` calls `abort()` when `cond` is false.
There is no recovery -- the process terminates. Use `assert` for invariants
that must never be violated; use `Result`/`Option` for expected failure modes.

```vani
assert n >= 0, "n must be non-negative";
assert len(xs) > 0;
```

SMT-verified assertions (`prove`) are discharged at compile time and emit
no runtime code when verified. See *SMT verification* below.

### Summary

| Need | Tool |
|------|------|
| Value may be absent | `Option<T>` |
| Operation can fail with diagnostic | `Result<T, E>` |
| Recover from absence or error | `match` on the enum |
| Propagate failure upward | `try EXPR` or `EXPR?` |
| Abort on invariant violation | `assert cond[, "msg"]` |
| Compile-time proof of correctness | `prove` (see *SMT verification*) |

> For the design rationale -- why values over exceptions -- see Part VI,
> *`try` is a value-flow shortcut, not an exception system*.

## SMT verification

`prove` will reach the SMT layer when constant folding and structural
recognition both fail. Example:

```intent
fn safe_subtract(a: i64, b: i64) -> i64
requires a >= b;
{
  prove a - b >= 0;
  return a - b;
}
```

The checker encodes the function's `requires` plus the negation of the prove
expression and asks z3. If z3 returns `unsat`, the proof holds; `sat` means
z3 produced a counterexample and the prove is rejected; `unknown` or "skipped"
(unsupported features) produces a diagnostic suggesting how to simplify the
claim.

**Call sites verify callee `requires`.** When a function with `requires`
clauses is called, the checker substitutes the argument expressions for
the parameter names in each precondition and asks z3 whether the
substituted preconditions hold under the *caller's* current facts. A
counterexample produces a diagnostic such as

```
argument to 'safe_sub' violates its 'requires' clause
  [counterexample: a = 3, b = 7]
note: callee precondition
requires a >= b;
```

before the runtime check would ever fire. Preconditions outside the SMT
v1 fragment fall back silently — the runtime `assert` still guards the
call. Calls inside any statement-level expression (`let`, `=`, `return`,
`assert`, `prove`, `print`, `if`/`while` conditions) are covered.

**Contradictory `requires` are flagged.** Before checking a function's body,
the verifier asks z3 whether the requires clauses are jointly satisfiable.
If they are not, every `prove` in the body would be vacuously true and the
function is unreachable at runtime — both bad signals. A diagnostic such as

```
function 'dead' has contradictory 'requires' clauses; every proof in
its body is vacuously true and the function is unreachable
```

surfaces. Encodings that exceed the SMT v1 fragment fall back to "not
contradictory" (conservative), so the check never produces a false alarm.

Integer semantics in the SMT model are infinite-precision (SMT-LIB `Int`)
plus a range constraint per variable's type. This is sound when arithmetic
stays within the declared range — the same condition the C backend's runtime
already requires for correct execution. Wrap-around / overflow modeling is a
follow-up.

**`len(xs)` over fixed-size arrays** is substituted with the compile-time
length during SMT encoding — even when `xs` was passed by `&` or `&mut`
reference. So `requires i < len(xs)` is dischargeable for `xs: &[T; N]`
arguments.

**`len(xs)` over `Vec<T>`** is encoded as a per-binding opaque SMT integer
`<name>_len` with `>= 0`. The length is treated as an unknown but consistent
value across a single proof — so `requires i < len(xs); prove i < len(xs);`
works (both sides reference the same SMT variable), as does propagating
`ensures _return < len(xs);` from a callee that promises a safe index.

**Vec-builtin length facts.** `let r = <builtin>;` automatically records
the resulting length so subsequent proofs see the relationship between
old and new bindings:

| Builtin                | Recorded fact                  |
|------------------------|--------------------------------|
| `vec(a, b, c)`         | `len(r) == 3`                  |
| `push(xs, v)`          | `len(r) == len(xs) + 1`        |
| `set(xs, i, v)`        | `len(r) == len(xs)`            |
| `clone(xs)`            | `len(r) == len(xs)`            |

So `prove len(push(xs, v)) == len(xs) + 1` discharges (when phrased as
`let ys = push(xs, v); prove len(ys) == len(xs) + 1;` — push consumes
its argument, so the relationship must be captured before the move).
The inline form `prove len(push(xs, v)) == len(xs) + 1` also works:
the verifier rewrites the call to a fresh symbolic Vec constrained by
the same length relationship.

**Stale facts are invalidated on reassignment.** Recording length facts
about a binding raises a question: what happens when that binding is
later reassigned? The verifier drops every fact mentioning the name
(both builtin length facts and ensures-derived facts from `let r =
foo();`) at any same-scope `let` shadow or any `name = expr;`
assignment — *outside a loop body*. Inside a loop body the drop is
suppressed so the substitution-based preservation check at body-end
still sees the entry invariants; preservation then re-establishes the
invariant for the new value via the last-reassignment rewrite.

One incompleteness gained for soundness: `let xs = push(xs, v);`
(same-name shadow with a self-referencing call) records no new fact,
since the natural relationship `len(xs) == len(xs) + 1` would be a
contradiction. Rename to `let ys = push(xs, v);` to recover the
length relationship in proofs.

**Array element reasoning.** Beyond length, the verifier models each
Vec/Array binding with an integer, bool, or float element type as a
symbolic SMT array `arr_<name>: (Array (BV64) Element)`, and reads
encode as `(select arr_<name> idx)`. This lets `prove xs[k] == V`
discharge in several composable shapes:

| Construct                | Fact emitted                                      |
|--------------------------|---------------------------------------------------|
| `let xs = vec(a, b, c)`  | `xs[0] == a`, `xs[1] == b`, `xs[2] == c`          |
| `let xs: [T;N] = [..]`   | per-slot `xs[k] == elements[k]`                   |
| `let ys = set(xs, k, v)` | `ys[k] == v` plus `arr_ys = (store arr_xs k v)`   |
| `let ys = push(xs, v)`   | `ys[len(xs)] == v` plus `arr_ys = (store arr_xs len(xs) v)` |
| `let ys = clone(xs)`     | `arr_ys = arr_xs` (every slot preserved)          |
| `let ys = xs;` (rebind)  | `arr_ys = arr_xs`                                 |
| `xs[k] = v;` (const k)   | bumps xs's SMT-array version; emits `arr_xs_v{N+1} = (store arr_xs_vN k v)`; existing facts get pinned to xs#N so they continue to describe the pre-assign state, while bare `xs` references resolve to the new version |
| `xs[i] = v;` (symbolic i)| same versioning path; the SMT solver can derive `xs[j] == old_value_j` for `j != i` through the store axiom even when `i` is opaque |

The store-axiom facts let the SMT solver derive `ys[j] == xs[j]` for
slots the call didn't touch, and the `Index` encoder is element-
type-aware (BV widths, `Bool`, `(_ FloatingPoint 11 53)` for f64,
`(_ FloatingPoint 8 24)` for f32 with operand-precision threading).

**SMT-array versioning.** Each `xs[i] = v` IndexAssign bumps a
per-binding version counter (tracked in the checker's `VarInfo`)
and emits a synthetic `arr_xs_v{N+1} = (store arr_xs_vN i v)`
axiom. Existing facts are pinned to `xs#N` before the bump so they
continue describing the pre-assign array, while subsequent
references to bare `xs` resolve to the new version at SMT query
time. Cross-binding relations like `arr_ys = arr_xs` (from a
`clone`) survive an IndexAssign on `xs`: `arr_ys` stays equal to
the old `arr_xs_vN`, the store axiom links `arr_xs_vN` to
`arr_xs_v{N+1}`, and the solver can reason about both old and new
states together.

`ensures _return[k] == V` is a first-class shape and propagates to
callers: the existing `record_ensures_facts` substitution rewrites
`_return` to the let-bound result and emits the slot fact. Multiple
per-slot ensures compose into full post-call array identity. See
`examples/array_proofs.vani` for the end-to-end pattern.

**Dev opt-out: `INTENTC_NO_VERIFY=1`.** Setting this env var skips
every SMT round-trip — `prove`, `ensures`, `invariant`, contradictory-
`requires`, call-site `requires`, and bounds-elision all silently
return without contacting z3. Useful for fast iteration when you're
focused on a non-proof code change. Runtime safety guards
(`intent_check_bounds`, divisor, shift, `assert` lowering of
`requires`) are kept in place — the program still runs safely. Do
not set this in CI; verifier-only bugs (a wrong invariant, a
violated ensures) won't surface at compile time.

**SMT-discharged runtime-guard elision.** When the verifier can prove
that an `Index`, `Div`/`Rem` divisor, or `Shl`/`Shr` count is safe
from the in-scope facts, the C backend skips the matching runtime
helper (`intent_check_bounds`, `intent_check_<ty>_divisor`,
`intent_check_<ty>_shift`). Example: in

```intent
fn first(xs: &Vec<i64>) -> i64
requires len(xs) > 0;
{
  return xs[0];
}
```

the `requires len(xs) > 0` is the only fact needed to discharge
`0 < len(xs)`, so the emitted C is the raw `(*xs).data[0]` — no
runtime comparison at the access site. The same elision applies to
`xs[i]` reads inside `for i in 0..len(xs) { … }` and any other
context where the index's bounds are derivable from preconditions
and loop facts. Elision fails closed: when the SMT layer can't
discharge (Unknown / unsupported / no z3), the runtime check stays.

**`ensures` clauses** become contracts. They are verified at every `return`
site (the SMT layer substitutes `_return` with the actual return expression
and checks that requires + branch conditions imply the ensures), and at call
sites they become facts the caller can rely on:

```intent
fn safe_sub(a: i64, b: i64) -> i64
requires a >= b;
ensures _return >= 0;
{
  return a - b;
}

fn caller(a: i64, b: i64) -> i64
requires a >= b;
{
  let r: i64 = safe_sub(a, b);
  prove r >= 0;   // discharged from safe_sub's ensures
  return r;
}
```

When a `let r = foo(args);` appears in a function whose callee has ensures,
the checker substitutes parameter names with the argument expressions and
`_return` with `r`, then appends those facts to the per-scope fact list.
Subsequent `prove` queries in the same scope see them.

Inline calls in proofs work too: `prove foo(args) > 0;` is rewritten so
that the call becomes a fresh symbolic variable, the callee's `ensures`
clauses are substituted onto that variable (and the supplied args), and
the SMT solver discharges the query against those facts. Calls to
functions without `ensures` still surface as unsupported, since there is
nothing for the solver to assume about their return value.

```intent
fn inc(x: i64) -> i64
requires x < 1000;
ensures _return > x;
{
  return x + 1;
}

fn check(x: i64) -> i64
requires x > 0;
requires x < 100;
{
  prove inc(x) > x;  // discharged via inc's ensures, no let-binding needed
  return inc(x);
}
```

Branch conditions are also added to the fact list inside `if`/`else` bodies
(so `if x > 0 { prove x >= 1; }` is dischargeable). Branch-acquired facts
revert at the merge point — with one exception: when exactly one branch
terminates (return/break/continue), execution past the merge must have
taken the *other* branch, so the verifier keeps its guard as a fact.
This makes the early-return idiom

```intent
fn clamp(x: i64) -> i64
ensures _return >= 0;
{
  if x < 0 {
    return 0;
  }
  return x;     // `x >= 0` is in scope on this line.
}
```

verify without an explicit `else`.

The same narrowing applies after a natural loop exit. After
`while cond { … }` (with no `break` in the body), the post-loop facts
include `!cond` plus the invariants — so

```intent
let i: i64 = 0;
while i < 5
invariant i >= 0;
invariant i <= 5;
{
  i = i + 1;
}
prove i == 5;            // discharged: invariants + !cond ⇒ i == 5
```

is provable. The for-loop variant adds `i >= end` rather than `!cond`.
If the body can `break`, both checks are dropped (the loop may exit
with the condition still true).

### Loop invariants

```intent
fn sum_to(n: i64) -> i64
requires n >= 0;
ensures _return >= 0;
{
  let total: i64 = 0;
  let i: i64 = 0;
  while i < n
  invariant i >= 0;
  invariant total >= 0;
  {
    total = total + i;
    i = i + 1;
  }
  prove total >= 0;   // discharged from the invariant
  return total;
}
```

What the verifier does at each `while`/`for` loop with `invariant`s:

1. **Entry**: each invariant must be provable from the current SMT facts
   (function `requires`, branch conditions, prior ensures, and let-known
   constants).
2. **Body visibility**: inside the loop body, both the invariants and the
   loop condition are added as SMT facts so the body's own proves can use
   them. (And for `for i in start..end`, the bound `i < end` is also a body
   fact.)
3. **Preservation** (at body fall-through): each invariant is re-verified
   with a **last-reassignment substitution** applied — if the body
   contains `i = i + 1`, the invariant is checked as if `i` were `i + 1`
   for the purpose of the goal. For-loop bodies also implicitly substitute
   `i` with `i + 1` for the auto-increment. This catches buggy invariants
   like `invariant i < 3;` over `i = i + 1;` while admitting the typical
   linear-counter pattern.
4. **Post-loop**: invariants become SMT facts after the loop, available to
   subsequent `prove`s and to discharge the function's `ensures` clause.

Limitations (honest v1 caveats):

- The substitution captures the *last* reassignment per variable in the
  body — multiple distinct reassignments per iteration aren't tracked
  symbolically. Use a single update per variable per iteration for sound
  preservation checks.
- Reassignments inside nested `if`/`else` branches are merged via the
  union of last-reassigns; reassignments inside nested `while`/`for`
  loops are not propagated outward.
- The natural-exit `!cond` post-loop fact is not added (it would be unsound
  in the presence of `break`).

**Float reasoning** uses SMT-LIB's `FloatingPoint` theory, so IEEE-754
edge cases surface as counterexamples. For example, `prove x + 0.0 == x;`
on `x: f64` is *not* universally true — z3 reports `x = NaN`, since
`NaN + 0.0 = NaN` and `NaN == NaN` is false. Conversely, `prove !(x < x);`
discharges (all FP comparisons with NaN return false). Counterexamples
involving NaN, ±infinity, and signed zeros are rendered as `NaN`,
`+inf`/`-inf`, `0.0`/`-0.0` instead of their raw SMT-LIB s-expressions.

### Overflow-aware integer reasoning

Integer arithmetic is encoded as fixed-width `BitVec`, not infinite-precision
`Int`. This means:

- Wrap-around is faithfully modeled. `x + 1 > x` is **not** universally
  true for `x: i64` — z3 returns the counterexample at `INT64_MAX`. To
  prove arithmetic properties about `+`/`-`/`*`, add a `requires` clause
  bounding the inputs away from overflow (e.g., `requires a >= b;
  requires b >= 0;` for `prove a - b >= 0`).
- Counterexamples render as readable decimals — `x = 9223372036854775807`,
  `y = 0`, `len(xs) = 18446744073709551615` — by parsing z3's hex output
  (`#xffffffffffffffff`) against each variable's type and applying
  signed/unsigned interpretation.
- Comparisons split signed (`bvslt`/`bvsge`/...) vs unsigned
  (`bvult`/`bvuge`/...) based on the operand types.
- Integer casts use `sign_extend` (signed widening), `zero_extend`
  (unsigned widening), and `extract` (narrowing).
- Shifts (`<<`, `>>`) encode to `bvshl` / `bvlshr` / `bvashr`. Signed
  right-shifts use the arithmetic form so the sign bit is replicated.
  The shift count is automatically padded or truncated to match the
  left operand's width, so `x: u64 >> n: u32` proves cleanly.

Still planned: full SSA encoding for stronger preservation reasoning under
multi-reassignment loop bodies.

### Assert messages

`assert cond;` lowers to the C standard `assert(...)` macro. For more
informative runtime failures, pass an optional string after a comma:

```intent
fn lookup(xs: &Vec<i64>, i: u64) -> i64
requires i < len(xs);
{
  assert i < len(xs), "lookup: index out of range";
  return xs[i];
}
```

The custom-message form lowers to an `if (!cond) { fprintf(stderr, ...);
abort(); }` sequence so the printed message reaches stderr before the
process exits. Backslash, quote, newline, and other control characters in
the message are escaped into a valid C string literal.

### Discard pattern: `let _ = ...`

`_` is a write-only discard binding. It evaluates its right-hand side
for side effects (and to consume any affine values it captures) but
never introduces a name you can read back. Repeated discards in the
same scope do not collide because nothing is inserted into the
environment.

```intent
fn pure(x: i64) -> i64 { return x + 1; }

fn main() -> i64 {
  let _ = pure(7);              // Copy result → `(void)(fn_pure(7));`
  let _ = pure(8);              // Independent discard, no name clash.

  let owned: Vec<i64> = vec(1, 2, 3);
  let _ = owned;                // Consumes `owned` and frees its buffer.
  // `owned` is no longer usable here — the checker will reject it.

  return 0;
}
```

Lowering follows the value's category:

- **Copy** types (integers, floats, bool, refs) → `(void)(<expr>);`.
- **`Vec<T>`** → brace-scoped temporary plus a `..._free(...)` call so
  the heap buffer is released exactly once.
- **`[T; N]`** → brace-scoped temporary; the array drops on scope
  exit. The `(void)_intent_discard;` keeps the compiler quiet.

Reference values are rejected outright (`references cannot appear in a
'let _' discard`) because they would dangle the moment the discard
ends.

### Multi-file projects

A file can pull in others with `use "path.vani";`:

```intent
// math.vani
fn double(x: i64) -> i64 { return x * 2; }
```

```intent
// main.vani
use "math.vani";

fn main() -> i64 {
  let v: i64 = double(21);
  assert v == 42;
  return 0;
}
```

`intentc check`/`emit-c`/`run` accept the entry file and recursively resolve
`use` declarations relative to each file's directory. By default,
names from imported files share a flat namespace — but you can carve
out scoped sub-namespaces with **inline `module` blocks** at any level
(see the *Modules and namespaces* section below).

Cycles are detected by canonicalized path: each file is included at most
once across the dependency tree, so `a.vani` `use`-ing `b.vani` and
vice versa works fine.

Diagnostics in multi-file builds now point at the **original** file and
line, not the position in the concatenated buffer. A `FileMap`
(`diagnostic::FileMap`) tracks where each file's content lives in the
combined source, and `format_diagnostics_with_files` /
`format_diagnostics_json_with_files` resolve span offsets back to the
real `path:line:col` for each diagnostic — primary span and every
related note.

Caveats (v1):
- Name collisions across files surface as the normal "function 'X' is
  already defined" diagnostic.

### How linking works (build pipeline)

`intentc build file.vani -o out` lowers the entire program through
the LLVM pipeline:

```
file.vani  →  intentc check       (typecheck + SMT)
            →  emit LLVM IR (.ll)  (SSA path or tree fallback)
            →  opt -O2 (optional)
            →  llc -filetype=obj   (-O2, PIC)  →  .o
            →  cc -o out           (links libc, -pthread)
```

There is **no separate compile-then-link step** today — the whole
program goes through one driver invocation. Multi-file inputs are
**concatenated at the source level** through `use "path.vani";`
before the LLVM backend ever sees them, so all functions land in
one `.o` and `cc` produces the final binary in a single link.

#### Generating `.o` files for external linking

Two ways to produce an object file you can hand to another linker
(GCC / Clang / Rust's linker driver):

```bash
# Step 1: emit LLVM IR
intentc emit my_lib.vani --backend=llvm -o my_lib.ll
# Step 2: assemble to .o
llc -filetype=obj -relocation-model=pic -O=2 my_lib.ll -o my_lib.o
# Step 3: link with anything else
cc -o app my_lib.o c_main.c                    # link with C
clang++ -o app my_lib.o cpp_main.cpp           # link with C++
rustc cargo_main.rs --extern my_lib=my_lib.o  # link with Rust
```

Function symbols in the produced `.o` are named `fn_<vani_name>`
(e.g. `fn add` in vāṇी lowers to `fn_add` in the object). Their
ABI matches the C ABI for the target platform (System V on Linux /
macOS, MSVC on Windows). Declare them on the C / C++ side as:

```c
extern int64_t fn_add(int64_t a, int64_t b);
```

And on the Rust side as:

```rust
extern "C" {
    fn fn_add(a: i64, b: i64) -> i64;
}
```

#### Calling INTO vāṇी from external code

Works today via the `.o` route above. The vāṇी function's signature
must use Copy / pointer-compatible types (scalars, `ref T` borrows,
`Str` borrowed pointer). Affine handles (`Vec<T>`, `OwnedStr`,
`Atomic`, `Mutex`, `Guard`, `Channel`, `Task`) at the ABI boundary
need conversion — currently no FFI helper exists, so the
recommended pattern is to expose scalar / pointer entry points and
let vāṇी own the allocations internally.

#### Calling FROM vāṇी into external code

vāṇी declares foreign functions with the `extern "C" fn` form:

```vani
extern "C" fn abs(x: i32) -> i32;
extern "C" fn sqrt(x: f64) -> f64;
extern "C" fn triple(x: i32) -> i32;   // from your own helper.c

fn main() -> i64 {
  let a: i32 = abs(-7 as i32);    // libc — links by default
  let r: f64 = sqrt(81.0 as f64); // libm — needs -lm
  let t: i32 = triple(7 as i32);  // your code — needs --link-with
  write "abs(-7) =", a;
  write "sqrt(81) =", r;
  write "triple(7) =", t;
  return 0;
}
```

The body is empty; the linker provides the symbol. Codegen emits a
prototype against the bare C-ABI name (LLVM `declare`, C `extern`),
not a `fn_<vani_name>` definition.

`intentc build` accepts two flag groups for the link step:

```bash
intentc build prog.vani --link-with helper.c -o prog   # your .c / .o
intentc build prog.vani -lm -o prog                    # system library
intentc build prog.vani --link-with helper.o -lcurl -o prog   # both
```

`--link-with PATH` (repeatable) hands an extra object or source file
to `cc`. `-l<name>` (repeatable) forwards a library-link flag
verbatim. Both flag groups appear after the vāṇี object so symbol
resolution follows usual link order.

**Effects**: extern fns are conservatively treated as impure. The
SMT engine can't reason across the FFI boundary, so any
`prove`/`assume` involving an extern call must rest on caller-side
invariants. `pure fn` bodies reject impure extern calls.

For foreign functions that are genuinely pure (`abs`, `sqrt`, the
trig functions, `strlen`, etc.), mark the declaration `pure
extern "C" fn name(...) -> R;` to opt into purity. The caller is
asserting the symbol has no side effects, no shared state, and
deterministic output — vāṇी can't verify across the FFI boundary,
so misuse falls back to runtime behavior.

```vani
pure extern "C" fn sqrt(x: f64) -> f64;   // libm — known pure
extern "C" fn rand() -> i32;              // impure — no annotation
```

**ABI scope (v1)**: scalars (`i8..i64`, `u8..u64`, `f32/f64`,
`bool`), `Str` (NUL-terminated `i8*`), and any reference
(`ref T` / `mut ref T`) — pointers cross the FFI boundary
cleanly. The checker rejects unsupported shapes at the extern
declaration site with a `ref T` migration hint:

```vani
// rejected — silent ABI corruption (packed-register passing
// in System V x86-64 wouldn't match vāṇī's emit)
extern "C" fn point_sum(p: Point) -> i32;

// accepted — pass by reference instead
extern "C" fn point_sum(p: ref Point) -> i32;
```

Owned heap handles (`Vec<T>`, `OwnedStr`) are rejected
unconditionally: their drop semantics don't survive crossing the
foreign-code boundary. Exclusive handles (`Atomic<T>`, `Mutex<T>`,
`Channel<T, N>`, `Task`, `Guard<T>`) likewise. Pass scalars / `Str`
/ `ref T` instead and let vāṇी own the allocations.

Still queued: correct ABI lowering for small aggregates by value
(packed-register passing), varargs, function-pointer callbacks,
and packed/repr(C) layout attributes.

See `examples/ffi.vani` for the canonical demo.

### JSON diagnostics

`intentc check file.vani --json` produces a JSON object on stdout
suitable for editor integrations and CI:

```json
{
  "diagnostics": [
    {
      "level": "error",
      "message": "value 'xs' was moved; cannot use after move",
      "primary": { "file": "f.vani", "line": 5, "col": 18, "end_line": 5, "end_col": 20 },
      "related": [
        { "message": "'xs' was moved here",
          "span": { "file": "f.vani", "line": 4, "col": 21, "end_line": 4, "end_col": 23 } }
      ]
    }
  ]
}
```

The output ends with a single newline. On success, the body is
`{"diagnostics":[]}`. Without `--json`, the human-readable form goes to
stderr as before.

## Modules and namespaces

vāṇī has Rust-style inline modules with explicit paths (`::`),
compile-time visibility checks, and a `use`-declaration form for
local aliases. Everything happens at parse/check time — the
backends never see the `module` keyword. Detailed design
rationale lives in [`docs/namespaces_design.md`](docs/namespaces_design.md).

```vani
module geo {
  pub struct Point { x: i64, y: i64 }

  // Private — accessible only inside `geo`.
  fn shift(p: Point, dx: i64) -> Point {
    return Point { x: p.x + dx, y: p.y };
  }

  pub fn origin() -> Point { return Point { x: 0, y: 0 }; }
  pub fn step_right(p: Point) -> Point { return shift(p, 1); }

  // Nested modules work — bare `Point` inside `bounds` would
  // need a path (`geo::Point`) or its own `use`.
  module bounds {
    pub fn area(p: geo::Point) -> i64 { return p.x * p.y; }
  }
}

// Bring items into scope. Five forms:
use geo::Point;                          // single-item
use geo::{origin, step_right};           // multi-item brace list
use geo::*;                              // glob (direct children only)
use geo::bounds::{area as bounds_area};  // per-entry `as` rename
// use geo::*;  // would collide with the lines above — caught at compile time

fn main() -> i64 {
  let p: Point = origin();
  let r: Point = step_right(p);
  let z: i64 = bounds_area(r);
  write "step_right + bounds_area =", z;
  return 0;
}
```

### Key rules

- **Private by default.** Items inside a `module` body need `pub`
  to be reachable from outside the module.
- **`pub(kosh)`** is a finer-grained tier — exported within the
  current kosh but not through the (future) kosh boundary. Today
  it behaves identically to `pub`; the bit is preserved so
  enforcement activates once kosh boundaries ship.
- **Module-local `use`** inside `module body { … }` is scoped to
  that body. It does not leak outside or into nested submodules.
- **`pub use foo::bar;`** inside a module body re-exports the item
  under the current module's namespace (`facade::bar`).
  Re-exports are resolved transitively, so chained `pub use`
  collapses to a single hop.
- **Orphan rule.** `implement Iface for T` must live in the module
  of either `Iface` or `T`, or at the top level. Out-of-place
  impls surface a precise error.
- **Collision diagnostics.** Two `use` paths that bring the same
  local name into scope produce a precise error with a
  `use … as …;` hint. Same goes for the brace-list form.

### What's "kosh"?

**Kosh** (कोश, "treasure / repository") is vāṇī's word for what
Rust calls a *crate* — one compilation unit shipping a public
API surface. The future package registry is **Vāṇī-Kosh**. The
syntax `pub(kosh) fn …` records the intent that an item is
internal to the kosh; today vāṇī compiles a single kosh at a
time so the bit is preparatory. The full package-manager arc
(manifest → resolver → registry CLI → stdlib-as-kosh) is on the
roadmap.

## Effects, ownership, and parallelism

The language has a `pure fn` modifier and a `parallel for` loop
construct. Both are verified by a single **effects checker** that
walks the typed IR and rejects observable side effects:

  - `print` (observable I/O).
  - `assert ..., "msg"` (a runtime abort with a user-facing message).
  - `xs[i] = v` (IndexAssign — mutates a mutable buffer).
  - Reassignment over a non-`Copy` value (`Vec<T>` / `OwnedStr` drop).
  - Consuming a Vec via `for x in xs` (move-and-drop).
  - Calling a non-`pure` function. Heap-allocating builtins are
    also rejected — they're observable through the allocator:
    Vec mutators (`vec`, `push`, `set`, `clone`), `box(...)` (and
    its lowered form `__box_new`), and `+` on strings.
  - RNG family (`rand_i64`, `rand_in_range`, `rand_in_range_f64`,
    `rand_f64`, `rand_bool`, `rand_choice`, `rand_normal`,
    `seed_rng`) — non-deterministic by design; a `pure fn`'s
    output must be a function of its inputs, and IEC 62304
    Class C / DO-178C Level A / ISO 26262 ASIL D all forbid
    non-deterministic calls on safety-critical paths.

A `pure fn` body must satisfy every rule above. A `parallel for`
body is held to exactly the same rules — that's how the verifier
proves each iteration is independent and therefore data-race-free:

```intent
pure fn square(x: i64) -> i64 {
  return x * x;
}

fn main() -> i64 {
  parallel for i in 0..5 {
    let r: i64 = square(i);
    let _ = r;
  }
  return 0;
}
```

**OpenMP parallelism — both backends.**

*C backend.* Each `parallel for` is emitted as a regular C `for`
loop preceded by `_Pragma("omp parallel for")`. The
`run --backend=c` path probes the C compiler for `-fopenmp` and
adds the flag when supported; with it, iterations run on a thread
pool sized by `OMP_NUM_THREADS` (default = CPU count). Compilers
without OpenMP issue an "unknown pragma" warning and fall back to
sequential — also correct, because the verifier already proved
iteration-independent semantics.

*LLVM backend.* Each `parallel for` is lifted into an internal
`@__intent_par_<N>(i8* data)` function. The parent calls
`@GOMP_parallel(body_fn, ctx, 0, 0)` with `ctx = { i64 start,
i64 end, <capture_ptrs>... }`. The capture-pointer suffix carries
one pointer field per outer binding the body reads — the
verifier already proved every such reference is read-only, so
concurrent reads through the same pointer are race-free.

At the call site the parent stores `start`, `end`, and each
capture's parent address into the ctx struct, then bitcasts to
`i8*` and calls `@GOMP_parallel`. Inside the outlined function
each thread unpacks the captures via `getelementptr` + `load`,
registers them in its own local map, then computes its iteration
slice via `omp_get_thread_num()` / `omp_get_num_threads()` and
runs the body for that slice. Non-ref captures (scalars, arrays,
`Vec<T>`) pass the alloca pointer; ref captures (`&T`, `&mut T`)
pass the ref value itself (already a pointer). The body's
existing emit code handles either form transparently through the
normal `Var` lookup.

The `run --backend=llvm` path probes the well-known
`libgomp.so.1` location and adds `-load=<path>` to lli; the
`build` path passes `-fopenmp` to the linker so the emitted
binary is fully parallel.

**Windows hosts.** libgomp isn't available on native Windows
toolchains. When `intentc` is built on Windows the LLVM backend
omits the `@GOMP_parallel` / `omp_get_*` declarations and the
call site open-codes a hardcoded N=4 `@CreateThread` fan-out
instead: tid 0 runs synchronously on the calling thread; tids
1..3 are spawned via `@CreateThread(null, 0, fn, &warg, 0,
null)`, joined with `@WaitForSingleObject(h, -1)`, and released
with `@CloseHandle(h)`. The outlined function's signature
switches to `i8* @__intent_par_<N>(i8* %_arg)` to match the
CreateThread start-routine ABI, and reads its `tid`/`nt` from a
per-thread `WinParArg { i8* ctx, i64 tid, i64 nt }` struct
(filled at the call site) instead of calling
`omp_get_thread_num` / `omp_get_num_threads`. The captured ctx
shape is the same as on POSIX. Thread count is fixed at 4 in
v1; a future revision can plumb a runtime lookup through the
existing WinParArg without changing the outlined-fn shape.

**Note on `lli` + threading.** lli's MCJIT isn't safe for
concurrent function resolution. `intentc run --backend=llvm` sets
`OMP_NUM_THREADS=1` (unless the user overrides) so JIT'd parallel-
for runs sequentially. AOT-built binaries (`intentc build`) get
real parallelism with `OMP_NUM_THREADS` defaulting to the CPU
count.

**Reduction patterns.** A `parallel for` may carry one or more
`reduce <var> with <op>;` clauses. Supported ops:

| Op   | Variable type | C lowering              | SSA LLVM lowering (default)    |
|------|---------------|-------------------------|--------------------------------|
| `+`  | integer       | `reduction(+:var)`      | per-thread `alloca` init'd 0; body accumulates non-atomically; one `atomicrmw add` per thread at exit |
| `*`  | integer       | `reduction(*:var)`      | per-thread `alloca` init'd 1; body accumulates non-atomically; one `cmpxchg`-retry loop per thread at exit |
| `&&` | bool          | `reduction(&&:var)`     | per-thread `alloca` init'd 1 (all-true); body accumulates non-atomically; `atomicrmw and i8*` against an i8 shadow per thread at exit |
| `\|\|` | bool        | `reduction(\|\|:var)`   | per-thread `alloca` init'd 0 (all-false); body accumulates non-atomically; `atomicrmw or i8*` against an i8 shadow per thread at exit |
| `&`  | integer       | `reduction(&:var)`      | per-thread `alloca` init'd -1; one `atomicrmw and` per thread at exit |
| `\|` | integer       | `reduction(\|:var)`     | per-thread `alloca` init'd 0; one `atomicrmw or` per thread at exit |
| `^`  | integer       | `reduction(^:var)`      | per-thread `alloca` init'd 0; one `atomicrmw xor` per thread at exit |
| `min` | integer      | `reduction(min:var)`    | per-thread `alloca` init'd INT64_MAX; one `atomicrmw min` per thread at exit |
| `max` | integer      | `reduction(max:var)`    | per-thread `alloca` init'd INT64_MIN; one `atomicrmw max` per thread at exit |

For `+`, `*`, `&&`, and `||` the checker requires the body to
update `<var>` only as `<var> <op> <expr>` (or `<expr> <op>
<var>`). `min` and `max` are built-in pure intrinsics, so the
body must instead read `<var> = min(<var>, <expr>)` (or
`min(<expr>, <var>)`); same for `max`. In every case the checker
also forbids reads of `<var>` anywhere else in the body —
partial-value visibility would leak otherwise.

The bool-reduction shadow works as follows: at the parallel-for
entry the parent zext-stores the current bool value into a
freshly-allocated `i8` cell, captures the shadow's address into
the outlined fn's ctx struct, and the outlined fn runs
`atomicrmw and/or i8*` against it. On return the parent reads
the shadow, computes `icmp ne i8 …, 0`, and stores the i1 back
into the original alloca.

```intent
let total: i64 = 0;
parallel for i in 0..len(xs)
reduce total with +;
{
  total = total + xs[i];
}
print total;  // sum of xs[0..len(xs)]
```

See `examples/parallel.vani` for a runnable end-to-end
demonstration on both backends.

**Task handles.** `task <name> { … }` declares an affine
`Task` handle and a side-effect-free body. The same purity
rules as a `parallel for` body apply (no `print`, no
`IndexAssign` on captured bindings, no impure calls), and each
handle must be consumed by exactly one `join <name>;` in the
same block — a forgotten join or a double join is a checker
error.

```intent
fn main() -> i64 {
  let xs: [i64; 4] = [2, 3, 4, 5];
  task ta {
    let a: i64 = xs[0] * xs[0];
    let _ = a;
  }
  task tb {
    let b: i64 = xs[3] * xs[3];
    let _ = b;
  }
  join ta;
  join tb;
  return 0;
}
```

Both backends now lower `task` to a real pthread spawn: the
body is outlined into a per-spawn function that receives a
heap-allocated ctx struct holding the captures, the spawn
site calls `pthread_create`, and `join` calls
`pthread_join` and frees the ctx. Captures are restricted
to Copy types — affine handles (Vec/Atomic/Mutex/Guard/
Channel/arrays/OwnedStr) can't ride the ctx by value, so
the supported pattern is to pre-extract scalar values from
them before the spawn site. See `examples/tasks.vani` for
the canonical shape.

**Atomic cells.** The affine model rejects shared mutable
state by default — that's why `parallel for` bodies can't
`IndexAssign` on captured arrays, and why two tasks can't
both own the same `Vec<T>`. For the patterns the affine model
can't express (counters, lock-free queues, lazy caches),
`Atomic<T>` is the opt-in escape hatch. T ranges over the
integer widths `i8`..`i64`, `u8`..`u64`, and `bool`; the five
sequentially-consistent builtins below dispatch on element
width and emit width-appropriate atomic ops on both backends.
`Atomic<bool>` uses an i8 shadow in LLVM (zext/trunc at every
operand boundary because `i1` atomics aren't byte-addressable);
`atomic_fetch_add` is rejected on bool by the checker.

| Builtin                                        | Returns |
|------------------------------------------------|---------|
| `atomic_new(initial: T) -> Atomic<T>`          | affine handle (owned) |
| `atomic_load(a: &Atomic<T>) -> T`              | current value |
| `atomic_store(a: &Atomic<T>, v: T) -> T`       | the stored value (echo) |
| `atomic_fetch_add(a: &Atomic<T>, v: T) -> T`   | the OLD value (pre-add) |
| `atomic_compare_exchange(a: &Atomic<T>, expected: T, new: T) -> bool` | true on success (cell was `expected`, now `new`); false on failure |

All five are unconditionally safe across threads — there's
no need to wrap them in `Mutex` or `Arc`. The C backend lowers
storage as `_Atomic <T>` and uses the C11 `<stdatomic.h>` ops
(`atomic_load_explicit`, `atomic_store_explicit`,
`atomic_fetch_add_explicit`, `atomic_compare_exchange_strong_explicit`,
all with `memory_order_seq_cst`); the LLVM backend emits
width-matched `load atomic iN … seq_cst, align M`, the
matching `store atomic`, `atomicrmw add iN* …`, and
`cmpxchg iN* …` (`atomic_storage_llvm` + `atomic_align` map
each supported element to its IR type and natural alignment).
The handle itself is affine: `Atomic<T>` is not Copy, so each
cell has a unique identity that two threads can share only
via references.

```intent
fn main() -> i64 {
  let counter: Atomic<i64> = atomic_new(0);
  let _o1: i64 = atomic_fetch_add(&counter, 5);
  let _o2: i64 = atomic_fetch_add(&counter, 7);
  return atomic_load(&counter);  // 12
}
```

See `examples/atomics.vani` for a runnable demonstration.

**Channels.** `Channel<T>` is an affine handle to a 16-slot
bounded ring buffer with monotonic `head` / `tail` atomic
counters. `channel_send` blocks (spin) when the buffer is
full; `channel_recv` blocks when it's empty. The buffer
preserves FIFO order — send-send-send-recv-recv-recv returns
the values in the original order. Suitable for hand-off
pipelines where one side produces a small batch before
another consumes. `Channel<T>` defaults to capacity 16; `Channel<T, N>` lets
the user pick the ring size (any power of two ≥ 1). T
ranges over the integer widths `i8`..`i64` / `u8`..`u64`
plus `bool` (the LLVM backend stores bool slots as `[N x
i8]` and zext/trunc's the source-level i1 at each slot
boundary; C uses native `bool buf[N]`). Both backends
generate one per-`(T, N)` struct + runtime helpers, so a
program using `Channel<i64, 16>` and `Channel<i32, 8>`
emits both bundles side by side. The ring uses Vyukov-style
per-slot sequence numbers (`seq[i & (N-1)]`): a producer
enters round `t` only when `seq[t & MASK] == t`, then
publishes via `store atomic seq = t+1`; the consumer waits
for `seq == h + 1` before reading and releases the slot via
`store atomic seq = h + CAP`. This makes the channel MPSC-
safe — producers don't collide on slot claim and consumers
never see unpublished data. (Real-thread parallelism still
waits on the task lowering — see TODO #5.)

| Builtin                                          | Returns |
|--------------------------------------------------|---------|
| `channel_new() -> Channel<T>`                    | affine handle (owned) |
| `channel_send(ch: &Channel<T>, v: T) -> T`       | the sent value (echo) |
| `channel_recv(ch: &Channel<T>) -> T`             | the received value |

```intent
fn main() -> i64 {
  let ch: Channel<i64> = channel_new();
  let _ = channel_send(&ch, 42);
  return channel_recv(&ch);  // 42
}
```

**Mutexes with RAII guards.** `Mutex<T>` is an affine handle to
a value protected by Drepper's three-state futex lock on
Linux. Fast path: a single seq_cst compare-exchange from
unlocked (state=0) to locked-no-waiters (state=1). Under
contention the waiter atomically marks state=2 (waiters
present) and parks in `syscall(SYS_futex, FUTEX_WAIT_PRIVATE)`;
the unlocker `atomic_fetch_sub`s the state and on the
waiters-present path calls `FUTEX_WAKE_PRIVATE` to release one
parked thread. Non-Linux builds fall back to a portable
`sched_yield()` backoff. `mutex_lock(&m)` returns an affine
`Guard<T>` whose scope-exit drop releases the lock — the
RAII pattern. Multiple operations on the value can run under
the same lock acquisition (unlike `Atomic<T>`, where each
call is a single atomic op).

| Builtin                                            | Returns |
|----------------------------------------------------|---------|
| `mutex_new(initial: T) -> Mutex<T>`                | affine mutex (owned) |
| `mutex_lock(m: &Mutex<T>) -> Guard<T>`             | affine guard (owned) |
| `guard_get(g: &Guard<T>) -> T`                     | the protected value |
| `guard_set(g: &Guard<T>, v: T) -> T`               | the stored value (echo) |

```intent
fn double_in_place(m: &Mutex<i64>) -> i64 {
  let g: Guard<i64> = mutex_lock(m);
  let cur: i64 = guard_get(&g);
  let next: i64 = cur + cur;
  let _ = guard_set(&g, next);
  return next;
  // `g` drops here — backend emits the unlock atomic store.
}
```

The C backend declares static-inline runtime helpers for both
(`<stdatomic.h>` ops with `seq_cst` ordering); the LLVM backend
emits inline atomic ops + `cmpxchg`-retry spin loops. Both v1
lowerings are sequential — there's no real threading yet — but
the runtime atomicity is correct so a future threading backend
inherits race-freedom for free.

The checker statically rejects **double acquisition** of the
same mutex while a guard is still alive. The lock is
non-reentrant, so the deadlock that would otherwise occur at
runtime turns into a compile-time error:

```intent
let m: Mutex<i64> = mutex_new(0);
let g1: Guard<i64> = mutex_lock(&m);
let g2: Guard<i64> = mutex_lock(&m);  // error: mutex 'm' is already locked
```

Sequential locks (where the first guard drops before the second
lock) and simultaneous locks on different mutexes are both
accepted. The check is syntactic — it fires when the
`mutex_lock` argument is a direct `&Var` reference or a
bare reference-typed binding; indirect arguments
(`mutex_lock(get_mutex_ref())`) skip the check rather than
overreport.

The same check extends **across function boundaries**. Each
function's signature carries a per-parameter flag for "this
parameter gets locked somewhere in my body". At a call site,
if the caller holds a live guard on a mutex AND the callee is
known to lock the corresponding parameter, the call would
deadlock on entry — flagged at compile time:

```intent
fn lock_it(m: &Mutex<i64>) -> i64 {
  let g: Guard<i64> = mutex_lock(m);
  return guard_get(&g);
}
fn main() -> i64 {
  let m: Mutex<i64> = mutex_new(0);
  let g: Guard<i64> = mutex_lock(&m);
  let _ = lock_it(&m);   // error: cross-function double acquisition
  return 0;
}
```

The cross-function analysis is **transitive**: a
fixpoint pass over the call graph propagates `locks_params`
across calls. So if `helper(m)` returns `lock_it(m)` and
`lock_it` locks its parameter, then `helper` also locks its
parameter, and the call site `helper(&m)` is flagged when
the caller holds a guard on `m`. Calls are inspected by name
in v1 — a function-pointer-style indirect dispatch would
require dataflow on the SSA layer.

See `examples/concurrency.vani` for a runnable demonstration.

**RwLock and ReadGuard / WriteGuard.** `RwLock<T>` is an affine readers-writer
lock. Multiple concurrent readers are allowed; a writer gets exclusive access.
`rwlock_read` returns an affine `ReadGuard<T>`; `rwlock_write` returns an
affine `WriteGuard<T>`. Both guards release the lock at scope exit.

| Builtin | Returns |
|---------|---------|
| `rwlock_new(initial: T) -> RwLock<T>` | affine handle |
| `rwlock_read(rw: ref RwLock<T>) -> ReadGuard<T>` | shared (read) guard |
| `rwlock_write(rw: ref RwLock<T>) -> WriteGuard<T>` | exclusive (write) guard |
| `rguard_get(g: ref ReadGuard<T>) -> T` | read the protected value |
| `wguard_get(g: ref WriteGuard<T>) -> T` | read under write lock |
| `wguard_set(g: ref WriteGuard<T>, v: T) -> T` | update under write lock |

State encoding: 0 = unlocked, N > 0 = N concurrent readers, -1 = write-locked.
T ranges over all Copy element types.

**Barrier.** `Barrier` is an N-thread rendezvous primitive. All participants
call `barrier_wait`; each blocks until all N threads have arrived, then all
are released simultaneously. A generation counter prevents ABA races.

| Builtin | Returns |
|---------|---------|
| `barrier_new(n: i64) -> Barrier` | affine barrier for N threads |
| `barrier_wait(b: mut ref Barrier) -> bool` | blocks until all N arrive; last-to-arrive returns `true` |

`Barrier` is stack-by-value and affine. See `examples/concurrency.vani` for
a demonstration with real threads.

**Function pointers.** `fn(T1, T2, ...) -> R` is a first-class
type. A top-level function name in expression position yields
its function-pointer value, so functions can be passed as
arguments or stored in let bindings of fn-ptr type. Calls
through a fn-ptr binding lower to native function-pointer
invocation (C function pointer / LLVM
`call <ret> (<params>) %ptr(args)`).

```intent
pure fn double(x: i64) -> i64 { return x + x; }
fn apply(f: fn(i64) -> i64, x: i64) -> i64 { return f(x); }
fn main() -> i64 { return apply(double, 7); }   // 14
```

Indirect calls bypass the name-based purity / lock-graph
passes by construction (no signature to consult). The
checker accordingly rejects `CallIndirect` inside
`parallel for` bodies, task bodies, and `pure fn` contexts;
the cross-function deadlock detector reports nothing about
indirect callees rather than making false claims. The SSA
pipeline does not yet lower fn-ptr shapes — the tree-based
backends handle them directly.

See `examples/fn_pointers.vani` for a runnable demonstration.

## Print and output

**`print`** writes a newline-terminated line to stdout. It accepts a
comma-separated list of values:

```vani
print "hello, world";
print x;
print "value = ", x, " and y = ", y;
```

**Print block** -- group multiple output lines under one `print` keyword.
Each `;`-terminated group inside the braces becomes one output line:

```vani
print {
  "name:  ", name;
  "score: ", score;
  "rank:  ", rank;
}
```

This is exactly equivalent to three separate `print` statements.

**`eprint`** -- same syntax as `print` / `print { }`, but writes to **stderr**:

```vani
eprint "error: file not found";
eprint { "path = ", path; "code = ", code; }
```

**`flush_stdout() -> i64`** -- flushes the stdout buffer. Returns 0.

**`stdin_read_line() -> OwnedStr`** -- reads one line from stdin (including
the trailing `
`). The returned `OwnedStr` is affine and freed at scope exit.

## Heap allocation (`Box<T>`)

`Box<T>` is a single-owner heap pointer. `box(value)` allocates `value` on the
heap and returns an affine `Box<T>`. The heap memory is freed automatically
when the `Box<T>` goes out of scope (or when `implement Drop for T` runs
first if T has a custom destructor).

```vani
let b: Box<i64> = box(42);
```

`Box<dyn Iface>` stores a fat pointer (data + vtable) and is the standard
way to hold a heap-allocated trait object:

```vani
let b: Box<dyn Drawable> = box(Circle { r: 5 }) as dyn Drawable;
```

`Box<Vec<T>>` / `Box<OwnedStr>` chain-drop: when the outer Box drops, it
calls the inner type's destructor before freeing the Box allocation itself.

`Box<T>` is affine -- each value has exactly one owner; move semantics apply.

## Async concurrency

vani's async system is based on **compiler-rewritten state machines**.
The `async fn` keyword marks a function whose body the compiler transforms
into a `struct + poll fn + constructor` triple at parse time. No heap
allocation or runtime scheduler is required.

### `async fn` and `await`

```vani
async fn fetch(url: Str) -> OwnedStr {
  let conn: i64 = await(io_connect(addr, port));
  let data: OwnedStr = await(io_recv_async(conn, buf_size));
  return data;
}
```

The compiler rewrites `fetch` into:
- A state-machine struct `Task__fetch` that holds all locals live across
  `await` points.
- A `poll` function `__poll_fetch(mut ref state: Task__fetch) -> Poll<OwnedStr>`
  that advances the machine one step and returns `Poll::Pending` or
  `Poll::Ready(value)`.
- A constructor `__make_fetch(url: Str) -> Task__fetch`.

Callers drive the task with a `while` loop:

```vani
let task: Task__fetch = __make_fetch("example.com");
let result: Poll<OwnedStr> = Poll::Pending;
while true {
  result = __poll_fetch(mut ref task);
  if result is Poll::Ready { break; }
}
```

### Key async types (injected by the prelude)

| Type | Description |
|------|-------------|
| `Future<T>` | A computation that will eventually produce `T` |
| `Poll<T>` | `Poll::Ready(T)` -- done; `Poll::Pending` -- not yet |
| `CancelToken` | Checked before each suspend point; `token.cancelled` signals abort |
| `Task__<fn>` | The synthesized state-machine struct for `async fn <fn>` |

### `await(expr)`

`await(expr)` is a suspend point. The compiler splits the function body at
each `await` call and preserves all live locals in the state struct.

### Postfix `?` and `try`

`try EXPR` (or postfix `EXPR?`) propagates `Err` / `None` -- same as Rust's
`?` operator. Works inside both sync and async functions:

```vani
async fn read_line(fd: i64) -> OwnedStr {
  let n: i64 = io_recv_async(fd, buf)?;
  ...
}
```

### Networking builtins

**Blocking TCP** (all return `i64` -- the fd, or -1 on error):

| Builtin | Description |
|---------|-------------|
| `tcp_connect(host: Str, port: i64) -> i64` | blocking connect |
| `tcp_listen(port: i64) -> i64` | bind + listen |
| `tcp_accept(fd: i64) -> i64` | blocking accept |
| `tcp_read(fd: i64, buf: mut ref [u8; N]) -> i64` | blocking read |
| `tcp_write(fd: i64, data: Str) -> i64` | blocking write |
| `tcp_close(fd: i64) -> i64` | close |

**Epoll / non-blocking I/O** (Linux; macOS kqueue / Windows IOCP via `#ifdef`):

| Builtin | Description |
|---------|-------------|
| `io_epoll_create() -> i64` | create epoll fd |
| `io_epoll_add(epfd: i64, fd: i64) -> i64` | register fd |
| `io_epoll_wait(epfd: i64, timeout: i64) -> i64` | wait; returns ready fd |
| `io_accept_async(fd: i64) -> i64` | non-blocking accept |
| `io_recv_async(fd: i64, n: i64) -> OwnedStr` | non-blocking recv |
| `io_send_async(fd: i64, data: Str) -> i64` | non-blocking send |
| `io_set_nonblocking(fd: i64) -> i64` | set O_NONBLOCK |

**Timers:**

| Builtin | Description |
|---------|-------------|
| `sleep_ms(ms: i64) -> i64` | blocking sleep (POSIX `nanosleep` / Windows `Sleep`) |

See `examples/async_fn.vani`, `examples/tcp_echo_epoll.vani`, and
[ARC8_V3_PLAN.md](ARC8_V3_PLAN.md) for end-to-end examples.

## SIMD and vectorization

vāṇī has three layers of SIMD support, from automatic to explicit:

**Layer 1 — auto-vectorization (always on).** Every `while` loop
gets `!llvm.loop.vectorize.enable` metadata. LLVM produces SSE /
AVX2 (x86-64) or NEON (AArch64) instructions automatically when
the loop is safe to vectorize.

**Layer 2 — `#[vectorize]` attribute.** Adds software-pipeline
interleaving (×4 interleave count) on top of auto-vectorize. One attribute,
zero code change:

```vani
#[vectorize]
fn dot(a: ref Vec<i64>, b: ref Vec<i64>, n: i64) -> i64 {
    let s: i64 = 0;
    let i: i64 = 0;
    while i < n { s = s + a[i] * b[i]; i = i + 1; }
    return s;
}
```

**Layer 3 — `vec128<T>` and `simd_*` builtins.** A 128-bit SIMD
register type with seven built-in operations:

```vani
// Explicit four-lane f32 SAXPY
fn saxpy(y: ref Vec<f32>, x: ref Vec<f32>, alpha: f32, n: i64) -> i64 {
    let a: vec128<f32> = simd_splat(alpha);
    let i: i64 = 0;
    while i + 4 <= n {
        let xi: vec128<f32> = simd_load(x, i);
        let yi: vec128<f32> = simd_load(y, i);
        let _ = simd_store(y, i, simd_add(yi, simd_mul(a, xi)));
        i = i + 4;
    }
    return 0;
}
```

| Builtin | What it does |
|---------|-------------|
| `simd_splat(val: T) -> vec128<T>` | Broadcast scalar to all lanes |
| `simd_load(v: Vec<T>, idx: i64) -> vec128<T>` | Load N lanes from `v[idx..]` |
| `simd_store(v: Vec<T>, idx: i64, d: vec128<T>) -> Vec<T>` | Store N lanes |
| `simd_add` / `simd_sub` / `simd_mul` | Lane-wise arithmetic |
| `simd_reduce_add(v: vec128<T>) -> T` | Horizontal sum |

Lane counts: `i8`/`u8` → 16, `i16`/`u16` → 8, `i32`/`u32`/`f32` → 4,
`i64`/`u64`/`f64` → 2.

On AArch64 targets, `vec128<T>` maps directly to NEON 128-bit
`v`-registers. `--cpu=cortex-a72 --target=aarch64-…` tunes the
instruction scheduler accordingly. Add `--sve` / `--sve2` for
AArch64 v8.2+/v9 SVE auto-vectorization.

**Layer 4 — `vec256<T>` and `simd256_*` builtins.** A 256-bit SIMD
register type — the same seven operations, twice the lanes:

```vani
// Eight-lane f32 dot product
fn dot256(a: ref Vec<f32>, b: ref Vec<f32>, n: i64) -> f32 {
    let acc: vec256<f32> = simd256_splat(0.0 as f32);
    let i: i64 = 0;
    while i + 8 <= n {
        acc = simd256_add(acc, simd256_mul(
            simd256_load(a, i), simd256_load(b, i)));
        i = i + 8;
    }
    return simd256_reduce_add(acc);
}
```

| Builtin | What it does |
|---------|-------------|
| `simd256_splat(val: T) -> vec256<T>` | Broadcast scalar to all lanes |
| `simd256_load(v: Vec<T>, idx: i64) -> vec256<T>` | Load N lanes |
| `simd256_store(v: Vec<T>, idx: i64, d: vec256<T>) -> Vec<T>` | Store N lanes |
| `simd256_add` / `simd256_sub` / `simd256_mul` | Lane-wise arithmetic |
| `simd256_reduce_add(v: vec256<T>) -> T` | Horizontal sum |

`vec256<f32>` has 8 lanes (vs 4 in `vec128<f32>`). On x86-64 with AVX2,
LLVM lowers `<8 x float>` to `ymm` registers. On AArch64 without SVE,
LLVM legalises the type as two 128-bit NEON registers. With `--sve` / `--sve2`,
a single SVE scalable register holds the full 256-bit width.

See [tutorials/advanced/05_simd.md](tutorials/src/advanced/05_simd.md)
for the full guide, decision flowchart, and platform-mapping table.

---

## File I/O

`FileHandle` is an affine RAII handle. The file is closed automatically
when the handle goes out of scope (scope-exit calls `fclose`).

```vani
let fh: FileHandle = file_open("data.txt", "w");
if file_is_ok(ref fh) {
  let _ = file_write(ref fh, "hello
");
}
/* fh closes here */
```

### File API

| Builtin | Description |
|---------|-------------|
| `file_open(path: Str, mode: Str) -> FileHandle` | open file (`"r"`, `"w"`, `"a"`, `"rb"`, ...) |
| `file_is_ok(ref fh: FileHandle) -> bool` | false if open failed |
| `file_read_line(ref fh: FileHandle) -> OwnedStr` | read one line including `
`; empty string at EOF |
| `file_write(ref fh: FileHandle, data: Str) -> i64` | write string; returns bytes written |
| `file_flush(ref fh: FileHandle) -> i64` | flush write buffer |
| `file_close(ref fh: FileHandle) -> i64` | explicit close (optional -- scope-exit also closes) |

`stdin_read_line() -> OwnedStr` and `flush_stdout() -> i64` are covered in the
*Print and output* section above.

See `examples/language/english/file_io.vani` for a runnable demonstration.

---

# Part V — Tooling

## Commands

The compiler has two backends: **LLVM IR (default)** and C (legacy,
on the deprecation path). `--backend=c` opts back into the C output
for `emit` / `run`; the `emit-c` subcommand is a stable alias for
C-only emission. `run` invokes `lli` for LLVM IR and `cc` for C
output. `build` produces a native binary via `llc` + `cc` (linker
only — no C source is compiled).

### Build & run pipeline

```bash
cargo run -- check examples/basics.vani                 # Type-check + verify
cargo run -- check examples/basics.vani --json          # JSON diagnostics
cargo run -- check examples/basics.vani --no-verify     # Skip SMT (dev opt-out)

cargo run -- emit examples/basics.vani                  # LLVM IR (default)
cargo run -- emit examples/basics.vani --backend=c      # C output
cargo run -- emit examples/basics.vani -o /tmp/basics.ll
cargo run -- emit-c examples/basics.vani                # Legacy alias for --backend=c

cargo run -- run examples/basics.vani                   # LLVM via lli (default)
cargo run -- run examples/basics.vani --backend=c       # C via cc

cargo run -- build examples/basics.vani -o /tmp/basics  # AOT native binary
                                                          # (LLVM → llc → cc linker)
```

### Debug subcommands

Useful for hacking on the lexer / parser / checker. Each runs the
pipeline up to a stage and dumps a debug-format representation.

```bash
cargo run -- tokens examples/basics.vani   # Token stream from the lexer
cargo run -- ast    examples/basics.vani   # Parsed AST (skips type checker)
cargo run -- ir     examples/basics.vani   # Typed IR (what the backends see)
```

### Running every example

```bash
cargo test                                                # Full suite + examples
cargo test llvm_backend_run_produces_same_output_as_c     # Cross-backend parity
```

### Editor integration via LSP

The Language Server ships as both `vanic lsp` (invoked via the main binary) and
as a standalone `intent-lsp` binary built from `src/bin/lsp.rs`:

```bash
cargo build --bin intent-lsp
./target/debug/intent-lsp        # speaks LSP over stdio
```

**Full LSP feature set (as of 2026-06-17):**

| Capability | What it does |
|---|---|
| `publishDiagnostics` | Lexer / parser / checker errors on every `didOpen` + `didChange`; source tag `vanic` |
| `hover` | Type of the smallest typed expression at cursor; augmented doc-block for the postfix `?` operator |
| `definition` | Jump to declaration site of any binding (`Var`, `Ref`, `RefMut`) |
| `references` | All reference sites for a binding; scope-separated via `binding_decl_span` |
| `rename` | Rename a binding across the file; validates new name, rejects keyword collisions |
| `completion` | Keywords (English + 30 supported dialects), type names, builtins (`vec`, `push`, `len`, `try_vec`, …), in-scope bindings, function names — scope-aware per function span |
| `codeAction` | Quick-fix insertion for missing single-char tokens (`; ) }` …); marked `is_preferred` for auto-apply-on-save |
| `semanticTokens/full` | Lex-driven coloring with IR-driven overrides: call callees → `function`, parameter declarations → `parameter + declaration + readonly`, parameter reads → `parameter + readonly`, type-position identifiers → `type`, keywords, numbers, strings |

**Dialect-aware completion** — when the file declares `// vani-lang: <tag>`,
the completion popup surfaces that dialect's native keywords alongside English.
All 30 supported dialects have keyword tables wired:
Hindi/Sanskrit/Marathi/Nepali/Maithili/Konkani (Devanagari), Mandarin, Bengali,
Tamil, Telugu, Gujarati, Punjabi, Kannada, Odia, Urdu, Persian, Korean,
Japanese, Arabic, Hebrew, Russian, Spanish, French, German, Portuguese, Italian,
Turkish, Polish, Indonesian, Malay, Swahili, Dutch, Thai, Hungarian, Czech.

Point your editor at `intent-lsp` (or `vanic lsp`) for `*.vani` files.
For Neovim with `nvim-lspconfig`:

```lua
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')
if not configs.vani then
  configs.vani = {
    default_config = {
      cmd = { 'intent-lsp' },
      filetypes = { 'vani' },
      root_dir = lspconfig.util.find_git_ancestor,
      settings = {},
    },
  }
end
lspconfig.vani.setup({})
```

For VS Code, point the `vani` extension's `server.path` setting at the
`intent-lsp` binary. Source tag in all diagnostics is `vanic`.

The cross-backend parity test runs every file under `examples/`
through both `--backend=c` and `--backend=llvm` and diffs stdout
+ exit codes. New examples are picked up automatically when wired
into `check_examples_all_succeed` and a `run_<example>_example`
test (see `tests/run_end_to_end.rs`).

### AI-assisted code generation (LLM bundle + MCP server)

vāṇी ships two scripts under [`tools/llm_context/`](tools/llm_context/)
that wire the compiler into off-the-shelf LLM workflows:

- **[`bundle.py`](tools/llm_context/bundle.py)** — a Markdown
  context bundle (~13K tokens full, trim flags down to ~4K). Pipe
  to clipboard, paste into Claude / ChatGPT / a local Llama, and
  the model generates passable vāṇी with zero training.
  ```bash
  python3 tools/llm_context/bundle.py | pbcopy           # macOS
  python3 tools/llm_context/bundle.py | xclip -sel clip  # X11
  python3 tools/llm_context/bundle.py --no-examples      # ~7K tokens
  ```
- **[`mcp_server.py`](tools/llm_context/mcp_server.py)** — exposes
  the same content as an [MCP](https://modelcontextprotocol.io/)
  server. 8 addressable resources (`vani://aliases`, `vani://patterns`,
  …) + 5 callable tools (`vani_check`, `vani_run`, `vani_emit_c`,
  `list_patterns`, `get_pattern`). Works with Claude Desktop /
  Claude Code / Cursor and any MCP-speaking host. The agent pulls
  just the section it needs AND can verify its own output before
  showing it to you — closing the write-verify-iterate loop.

The compiler's SMT-discharged diagnostics + step-by-step `help:`
elaborations were designed to be readable by both humans AND
LLMs. The same shape of feedback that helps a newcomer fix a
move-after-use also helps an LLM iterate toward a verified
solution. See the dedicated tutorial chapter
[Advanced 11 — Using vāṇी with an LLM](tutorials/src/advanced/11_llm_workflows.md)
for the full write-verify loop walkthrough + Claude Desktop
config.

A LoRA fine-tune of a small open-weights model (Phase ML-3) and
hosted inference (Phase ML-4) are queued but **not shipped** —
both gate on user demand. The bundle + MCP cover the 80% case
today.

---

# Part VI — Design Philosophy & Comparisons

## Design Philosophy & Limitations

vāṇī aims for a **small, fully-verifiable surface** — the core
primitives compose into richer patterns rather than the language
growing a new built-in for every shape. Three design decisions
that come up frequently:

### Composition over inheritance — `dyn Iface` is the escape hatch

Interfaces dispatch **statically** by default: `fn min<T>(a: T, b: T)
-> T where T is Cmp` monomorphizes at each call site to the concrete
T's `cmp` impl. Composition (struct fields + interface bounds) covers
most patterns; static dispatch is faster and lets the verifier see
through every call site.

For **heterogeneous collections** (the workflow where static dispatch
falls short), use `dyn Iface` — a fat pointer carrying `{ &vtable,
&data }` (16 bytes) that holds any T implementing the interface:

```intent
struct Circle { r: i64 }
struct Square { side: i64 }

interface Drawable {
  fn area(self: Circle) -> i64;
}

implement Drawable for Circle { fn area(self: Circle) -> i64 { … } }
implement Drawable for Square { fn area(self: Square) -> i64 { … } }

fn total_area(shapes: Vec<dyn Drawable>) -> i64 {
  let total: i64 = 0;
  for s in shapes { total = total + s.area(); }
  return total;
}
```

`dyn` works at let bindings, fn params (owned + `ref dyn` borrow),
struct fields, and `Vec<dyn Iface>`. No inheritance, no abstract
base classes — just a per-interface vtable with one fn-ptr per
method in declaration order. See
[examples/dyn_dispatch.vani](examples/dyn_dispatch.vani) for the
end-to-end shape. Tagged enums (`enum Shape { Circle(...), … }`) are
still a fine alternative when the variant set is closed and known
at the call site.

### `try` is a value-flow shortcut, not an exception system

vāṇī has no exceptions, no `catch`, no stack unwinding. Errors are
**values** carried in payloaded enums (Option-like, Result-like).
The `try` keyword is the Rust `?` operator — sugar for early-return
on the None / Err arm:

```intent
fn run(opt: Opt) -> Opt {
  let v: i64 = try opt;          // None? return Opt.None now.
  return Opt.Some(v + 1);        // happy path.
}
```

To "catch" a possible-None value, use `match`:

```intent
match maybe_value {
  Opt.Some(v) then use(v),       // happy path
  Opt.None then handle_missing(), // "catch" the None
}
```

Every control-flow path is statically visible — no hidden unwind
from any call site. `assert` triggers `abort()`; there's no
mechanism to recover from a failed assertion.

### Data structures + algorithms — affine-first roadmap

vāṇी keeps **affine ownership** as the standing language decision.
Every container, algorithm, or API on the roadmap is flagged for
compliance — items that fight single-owner semantics are explicitly
marked with the affine-friendly substitute named in the same row.

**Flag legend.** ✅ AFFINE — single-owner holds end-to-end.
⚠️ AFFINE-TENSION — compiles, but the API needs a careful contract
(e.g. `get` returns `Option<ref V>` not `V`; `remove() -> Option<V>`
is the move-out path; `insert(k, v)` consumes both).
🛑 NON-COMPLIANT — cannot ship as designed; substitute named.

**Shipped today.** Backbone primitives that already cover ~70% of
real use cases:

| Structure | Shipped form | Flag |
|-----------|--------------|------|
| Stack | `Vec<T>` + `push(mut ref xs, v)` + `pop(mut ref xs)` (closure #219) | ✅ AFFINE |
| Sort (in-place) | `sort(mut ref xs)` / `sort_by(mut ref xs, cmp: fn(i64, i64) -> i64)` on `Vec<i64>` (closure #293); also `Vec<f64>` — f64 `sort_by` comparator is `fn(f64, f64) -> i64` (F64-1) | ✅ AFFINE |
| Reverse / dedup | `reverse(mut ref xs)` (any Copy element) / `dedup(mut ref xs)` (Vec<i64>) (closure #294) | ✅ AFFINE |
| Search | `find(ref xs, needle) -> Option<i64>` / `contains(ref xs, needle) -> bool` / `binary_search(ref xs, needle) -> Option<i64>` on `Vec<i64>` (closure #295) | ✅ AFFINE |
| Mutators | `swap_remove(mut ref xs, i) -> T` / `insert(mut ref xs, i, v) -> i64` / `clear(mut ref xs) -> i64` (any non-array element) (closure #296) | ✅ AFFINE |
| Array ops | `sort` / `sort_by` / `reverse` / `find` / `contains` / `binary_search` extended to `[i64; N]` (closure #297) | ✅ AFFINE |
| String ops | `str_contains` / `str_starts_with` / `str_ends_with` -> bool, `parse_int` / `parse_float` -> `Option<i64>` / `Option<f64>`, `str_trim` -> `OwnedStr` (heap-allocating, strips ASCII whitespace), `str_replace(s, from, to)` -> `OwnedStr` (two-pass non-overlapping substring replace), `str_split(s, delim)` -> `Vec<OwnedStr>` (heap-allocating per-element span dup) (closure #298 + `str_trim` in **#348** + `str_replace` in **#349** + `str_split` in **#350**) | ✅ AFFINE |
| Math | `pow` / `sqrt` / `sin` / `cos` / `tan` / `floor` / `ceil` (f64 -> f64), `abs` overloaded (i64 -> i64, f64 -> f64) (closure #299) | ✅ AFFINE |
| RNG | `seed_rng(u64)` / `rand_i64()` / `rand_in_range(lo, hi)` — thread-local xorshift64 (closure #300) | ✅ AFFINE |
| Hash | `hash_i64(i64)` / `hash_f64(f64)` / `hash_str(Str)` / `hash_combine(u64, u64)` -> `u64` — FNV-1a; `hash_f64` bitcasts IEEE-754 bits and folds `-0.0`→`+0.0`. Adversarial-resistant SipHash-2-4: `siphash_i64(k0, k1, i64)` / `siphash_str(k0, k1, Str)` -> `u64`, keyed with a 128-bit (k0, k1) pair, spec-vector parity verified (closure #301 + `hash_f64` in **#347** + SipHash-2-4 in **#351**) | ✅ AFFINE |
| BinaryHeap | `heap_push` / `heap_pop` / `heap_peek` / `heapify` on `Vec<i64>` — min-heap (closure #302) | ✅ AFFINE |
| Deque | `Deque<i64>` ring buffer w/ 8 builtins (new / push_back / push_front / pop_back / pop_front / peek_back / peek_front / len) (closure #303) | ✅ AFFINE |
| HashSet | `HashSet<i64>` open-addressing hash set w/ 3-state slot tag (empty / occupied / tombstone); 5 builtins (new / insert / contains / remove / len) + method sugar; `(len + tombstones) ≥ capacity / 2` triggers a rehash that clears tombstones (closure #304 + remove in **#342**) | ✅ AFFINE |
| HashMap | `HashMap<i64, i64>` open-addressing key/value map w/ 3-state slot tag (empty / occupied / tombstone); 6 builtins (new / insert / get / contains_key / remove / len) + method sugar; `(len + tombstones) ≥ capacity / 2` triggers a rehash that clears tombstones (closure #305 + remove in **#343**) | ✅ AFFINE under v1 Copy-V; ⚠️ AFFINE-TENSION when V goes non-Copy |
| BTreeSet | `BTreeSet<i64>` ordered set on sorted-Vec backing w/ 8 builtins (new / insert / contains / remove / len / range / min / max) + method sugar; `range(lo, hi, mut ref out)` appends every key in `[lo, hi]` to `out` in sorted ascending order; `min`/`max` are O(1) and return `Option<i64>` (closure #306 + range in **#346** + min/max in **#352**) | ✅ AFFINE |
| BTreeMap | `BTreeMap<i64, i64>` ordered key/value map on parallel-sorted-Vec backing w/ 10 builtins (new / insert / get / contains_key / remove / len / range_keys / range_values / min_key / max_key) + method sugar; `range_keys` and `range_values` append every entry in `[lo, hi]` to parallel `out` Vecs; `min_key`/`max_key` are O(1) and return `Option<i64>` (closure #307 + range queries in **#346** + min_key/max_key in **#352**) | ✅ AFFINE under v1 Copy-V; ⚠️ AFFINE-TENSION when V goes non-Copy |
| UnionFind | Disjoint-set with path-compressed find + union-by-rank; parallel `parent`/`rank` i64 arrays; 5 builtins + method sugar (closure #325, **first Level 4 arena container**) | ✅ AFFINE |
| BinaryHeap (dedicated) | `BinaryHeap<T>` first-class affine handle (i64 v1) backed by `i64*` + `len` + `cap`; min-heap; 5 builtins (`new` / `push` / `pop` / `peek` / `len`) + method sugar; `pop`/`peek` return `Option<i64>` (closure #326, **Level 4 #2**) | ✅ AFFINE |
| BloomFilter | `BloomFilter` probabilistic membership tester; bit array + `num_bits` + `num_hashes` + `insert_count`; double-hashing on FNV-1a; 5 builtins (`new` / `insert` / `contains` / `len` / `count`) + method sugar; false positives possible, false negatives impossible; v1 keys are i64 (closure #327, **Level 4 #6**) | ✅ AFFINE |
| Bst | `Bst<T>` AVL-balanced binary search tree on a node arena; parallel `keys` (i64) + `left` / `right` (i32 child indices) + `heights` (u8) arrays + root index; 7 builtins (`new` / `insert` / `contains` / `remove` / `len` / `min` / `max`) + method sugar; rotations swap i32 indices, no node moves; `min` / `max` return `Option<i64>` (closure #328 + AVL added in **#332**, **Level 4 #3**) | ✅ AFFINE |
| Graph | `Graph` weighted directed graph; per-edge parallel `edge_src` / `edge_dst` (i32) + `edge_weight` (i64) arrays + a lazy CSR adjacency cache (closure **#336**) that gives BFS/DFS O(V+E) traversal; 12 builtins: `new` / `add_edge` / `num_nodes` / `num_edges` / `bfs_reach` / `dfs_reach` / `dijkstra` / `has_cycle` / `mst_kruskal` / `mst_prim` / `astar(src, dst, ref h: Vec<i64>)` / `topo_sort(mut ref out: Vec<i64>)` + method sugar; `dijkstra` / `mst_*` / `astar` return `Option<i64>`; v1 i64 weights, i32 nodes (closure #329 + algorithm extensions in **#333** / **#334** / **#335** / **#336**, **Level 4 #5**) | ✅ AFFINE |
| Trie | `Trie` prefix tree on a node arena; flat 256 × num_nodes i32 `children` (full u8 alphabet after closure #345) + per-node `is_end` byte + `free_head`/`free_count` freelist; 7 builtins (`new` / `insert` / `contains` / `starts_with` / `delete` / `len` / `node_count`) + method sugar; any nonzero byte is a valid character (mixed-case, punctuation, digits); delete + arena compaction walks back up the path freeing dead-end nodes; recycled slots reused by future `insert`s; `node_count` reflects live nodes only (closure #330 + delete in **#340** + arena compaction in **#344** + u8 alphabet in **#345**, **Level 4 #4**) | ✅ AFFINE |
| SkipList | `SkipList<T>` probabilistic ordered set on a node arena; MAX_LEVEL=8 (v1); flat capacity × 8 i32 `forward` index array + per-node `node_levels` + `tail_node` index for O(1) max; geometric level distribution from a stored LCG seed; 7 builtins (`new` / `insert` / `contains` / `remove` / `len` / `min` / `max`) + method sugar; `min` / `max` are both O(1) and return `Option<i64>`; remove tombstones the slot (no compaction) (closure #331 + remove in **#339** + tail tracker in **#341**, **Level 4 #7**) | ✅ AFFINE |
| Anon fn | `fn(p: T) -> R { body }` in value position; lambda-lifted to `__anon_fn_<N>` (closure #308). v1: no captured environment | ✅ AFFINE |
| Closures w/ captures | `let f = fn(x) -> R { ...captured_n... }; f(...)` — capture-by-value of Copy outer bindings; callable in same fn only (closure #314). Closures may be declared at top level or inside `if`/`while`/`for` bodies (closure #315) | ✅ AFFINE under v1 Copy contract |
| Iter combinators | Eager `vec_map` / `vec_filter` / `vec_fold` on `Vec<i64>` and `Vec<f64>` (F64-3 extended to f64; original Vec<i64>: closures #309 + #310); slicing `vec_take(ref xs, n)` / `vec_drop(ref xs, n)` (closure #313); fused single-pass family `vec_map_fold` / `vec_filter_fold` / `vec_map_filter` / `vec_map_filter_fold` (closures #316 + #317). Pair with anon fns or top-level fn-refs | ✅ AFFINE |
| Method-call sugar | `xs.map(f)` / `xs.filter(p)` / `xs.fold(init, g)` / `xs.sort_by(cmp)` / `xs.sort()` on `Vec<T>` receivers desugar to the builtins (closure #311); `m.get(k)` / `m.insert(k, v)` / `s.contains(v)` / `d.push_back(v)` / `.len()` etc. on HashMap / HashSet / BTreeMap / BTreeSet / Deque (closure #312); `xs.take(n)` / `xs.drop(n)` / uniform `xs.len()` on Vec (closure #313); Vec mutators + search (`xs.push(v)` / `xs.pop()` / `xs.reverse()` / `xs.dedup()` / `xs.swap_remove(i)` / `xs.insert(i, v)` / `xs.clear()` / `xs.find(v)` / `xs.contains(v)` / `xs.binary_search(v)`) (closure #320); `[T; N]` Array sugar (`arr.sort()` / `arr.sort_by(cmp)` / `arr.reverse()` / `arr.find(v)` / `arr.contains(v)` / `arr.binary_search(v)`) (closure #321) | ✅ AFFINE |
| Queue (concurrent) | `Channel<T, N>` MPSC ring buffer w/ futex blocking | ✅ AFFINE |
| Wait / signal | `Condvar` w/ `wait` / `wait_timeout` / `notify_one` / `notify_all` (closure #292) | ✅ AFFINE |
| Array (fixed) | `[T; N]` w/ nested-array support (closure #291) | ✅ AFFINE |
| Heap-vec | `Vec<T>` incl. `Vec<Vec<T>>`, `Vec<Struct{OwnedStr…}>` | ✅ AFFINE |
| Owned string | `OwnedStr` from `"a" + "b"`; `Str` for borrowed | ✅ AFFINE |
| Result / Option | Prelude-injected generic enums (#282 + #281) | ✅ AFFINE |
| Shared atomic | `Atomic<T>` for shared counters | ✅ AFFINE |
| Shared mutable | `Mutex<T>` + `Guard<T>` | ✅ AFFINE |
| Fallible alloc | `try_vec(n) -> Result<Vec<i64>, AllocError>` (#284) | ✅ AFFINE |

**Sequenced queue.** Full per-item detail (with implementation
plan and affine contract) lives in [TODO.md](TODO.md) under the
*Data structures + algorithms roadmap* section.

| Level | Items | Affine flag |
|-------|-------|-------------|
| **1 — Operations on existing primitives** ✅ **COMPLETE** | `Vec.sort` / `sort_by(fn)` (#293) · `Vec.reverse` / `Vec.dedup` (#294) · `Vec.find` / `contains` / `binary_search` (#295) · `Vec.swap_remove` / `insert` / `clear` (#296) · Array ops on `[i64; N]` (#297) · `str_contains` / `str_starts_with` / `str_ends_with` / `parse_int` / `parse_float` (#298) + heap-allocating `str_trim` (#348) / `str_replace` (#349) -> `OwnedStr`, `str_split` (#350) -> `Vec<OwnedStr>` · Math: `pow` / `sqrt` / `sin` / `cos` / `tan` / `floor` / `ceil` + overloaded `abs` (#299) · RNG: `seed_rng` / `rand_i64` / `rand_in_range` (#300) · Hash: `hash_i64` / `hash_f64` / `hash_str` / `hash_combine` (FNV-1a) (#301 + `hash_f64` in #347) + `siphash_i64` / `siphash_str` (SipHash-2-4 keyed, adversarial-resistant) (#351) | ✅ AFFINE |
| **2 — Generic containers** (deps: Level 1, generic decls #281) ✅ **COMPLETE** | ✅ BinaryHeap-on-Vec (#302) · ✅ `Deque<i64>` (#303) · ✅ `HashSet<i64>` (#304) · ✅ `HashMap<i64, i64>` (#305, AFFINE under Copy-V; AFFINE-TENSION queued for non-Copy V) · ✅ `BTreeSet<i64>` (#306, sorted-Vec backing) · ✅ `BTreeMap<i64, i64>` (#307, parallel sorted-Vec backing). Dedicated `BinaryHeap<T>` wrapper landed at Level 4 (#326); node-arena B-tree variants → Level 4 | ✅ / ⚠️ AFFINE-TENSION |
| **3 — Closures + iterators** | ✅ Anonymous fn expressions w/o captures (#308) · ✅ Eager `vec_map` / `vec_fold` / `vec_filter` on Vec<i64> and Vec<f64> via fn-ptr args (#309 + #310; f64 in F64-3) · ✅ Method-call sugar across Vec + affine containers (#311 + #312) · ✅ `vec_take` / `vec_drop` + uniform `xs.len()` (#313) · ✅ Closures w/ captured state (#314 + nested scopes #315) · ✅ Fused single-pass combinators `vec_map_fold` / `vec_filter_fold` / `vec_map_filter` / `vec_map_filter_fold` (#316 + #317) · ✅ Auto-fusion of `vec_map + vec_fold` chains (#318). ⏳ Auto-fusion of more chain shapes; non-Copy captures; capture-by-ref; passing closures as fn-ptr args; `.collect()` / `vec_zip` | ✅ / ⚠️ AFFINE-TENSION |
| **4 — Advanced / domain-specific** | BST / AVL / red-black via node arena + `i32` child indices (✅), B-tree arena (✅), Trie arena (✅), graphs as `Vec<Node>` + `Vec<Vec<u32>>` adjacency (✅), graph algorithms BFS / DFS / Dijkstra / A* / topo / Kruskal / Prim (✅), Union-Find (✅), skip list (✅), Bloom filter (✅) | ✅ AFFINE |

**Deferred / non-compliant** (flagged with reasoning + substitute):

| Item | Why non-compliant | Substitute |
|------|-------------------|------------|
| 🛑 Doubly-linked list w/ raw `prev` / `next` pointers | Two pointers into one node violate single-owner | Index-based Deque (Level 2 #15); index-based BST (Level 4 #20) |
| 🛑 Rc / Arc reference-counted shared ownership | Cycles defeat cycle-free Drop; deliberate v1 trade-off | Index-based graphs (Level 4 #23) for shared refs; `Channel<T, N>` for cross-task ownership; `Mutex<T>` for shared mutable |
| 🛑 Iterators yielding owned `T` | Would move every element out; tail Drop then double-frees | `for x in xs` already iterates by Copy-value (Copy T) or by-ref (non-Copy T); combinator chain (Level 3 #18) is by-ref or consume-whole-Vec via `.fold` / `.collect` |
| 🛑 Self-referential structs (Pin / pinning) | Affine moves invalidate self-pointers | Index-based arena pattern (Level 4 #20–#23) |
| 🛑 Garbage collector (any flavor) | Duplicates affine's deterministic Drop; defeats no-runtime promise | Affine + scope-exit Drop already covers it |

**The principle remains: add a new built-in only when no composition
of existing primitives gets within an order of magnitude of optimal.**
The roadmap above is what to ship — and *how to ship it under affine*
— not a wishlist of every container ever designed.

### Current limitations

The honest list, grouped by which work item closes them:

**Type system**

- Tuples are Copy-only — no `OwnedStr` / `Vec<T>` in a tuple element.
- Generic monomorphization supports one type parameter per fn (`<T>`,
  not `<T, U>`) and infers T from the first T-bearing argument's type.
  The inference walks the param-type pattern and the arg-type in
  lockstep — peeling matching `Box / Vec / Ref / RefMut / Atomic /
  Mutex / Guard / Array` wrappers — so `keep<T>(b: Box<T>)` called
  with `Box<i64>` binds T to `i64`, not to `Box<i64>`. Falls back to
  the legacy whole-arg binding when the shapes don't match.
- No closures with **lexical capture**. Inline anonymous `fn`
  literals work as higher-order arguments (e.g.
  `vec_map(ref xs, fn(x: i64) -> i64 { return x * 2; })`) and
  named-fn pointers travel by value, but neither form captures
  bindings from the enclosing scope — every value the body
  references must be a parameter or a top-level item.
- No `bool ↔ int` cast (deliberate — forces explicit branching).
- `Mutex<T>` restricted to `Mutex<i64>` (other widths waiting on a
  parametric runtime helper).
- Type aliases can't be recursive.

**Affine ownership**

- Partial-move tracking is one level deep — `let xs = t.x` works,
  but `let y = t.x.inner` (nested field move) is rejected (epic B).
- `Drop for T` accepts both `fn drop(self: T)` (by-value — only
  valid when T has no heap-owning fields; consumes self) and
  `fn drop(self: mut ref T)` (runs first, then the auto-per-field
  free runs — works for any T including heap-owning fields).

**`try` desugar**

- `while` loops between `try` and `return` aren't supported —
  while doesn't have a single tail-expression for the Some-arm
  to absorb. `if cond { return X; }` guards work via the
  AST-level guard-if rewriter (closure #232).

**Block expressions**

- `let r = { stmts; tail-expr };` admits `let`, `print`, and
  assignment statements. No inner control flow (`if`/`while`/`for`).
  Hoist control flow outside the block.

**Memory & runtime model**

- No GC, no Rc / Arc — affine + scope-exit Drop only.
- Async / await / coroutines: **✅ FULLY COMPLETE
  2026-06-08 (Arc 8 v1+v1.5+v1.6+v2+v3.1)** — every user-
  visible async + networking + concurrency feature plus the
  hand-rolled state-machine pattern ships: `async fn`,
  `await(expr)`, `Future<T>`, `Poll<T>`, `CancelToken`,
  `sleep_ms`, 8-builtin blocking TCP family, 7-builtin epoll
  + non-blocking I/O for single-thread cooperative
  scheduling, 3-builtin `io_*_async` aliases for the state-
  machine pattern. 28 acceptance examples + generic-async smoke,
  all byte-identical cross-backend. Three concurrency models
  supported: thread-per-task via `task` + `join`, single-thread
  cooperative via epoll + nb variants, OR hand-rolled state
  machines using `io_*_async`. **Arc 8 v3.1 compiler-driven
  sugar — FEATURE-COMPLETE 2026-06-08** — parser-level transform
  auto-generates the struct/poll/constructor triples from an
  `async fn` body across the full surface (suspend-in-branch
  state-splitting, nested ifs, loops + break/continue, match-
  with-suspends, `try EXPR` / postfix `?`, non-i64 boundary
  types, nested async, multi-task scheduling, generic
  `async fn`). Explicitly NOT Rust-style `Pin<&mut Self>`
  self-references (those stay 🛑 NON-COMPLIANT under affine).
  See [ARC8_V3_PLAN.md](ARC8_V3_PLAN.md).
- No exceptions (covered above).

**Tooling**

- Devanagari script support parked at user request — keyword
  aliases land, multi-word aliases + script-aware diagnostics
  deferred.

### Working around the limitations

Every documented limit has a checked-in workaround that
type-checks today. The patterns below are short and copy-
pasteable; pick the one that fits your call site.

**Tuples Copy-only → use a struct.** A struct with named fields
is the affine equivalent of a tuple of mixed-ownership
elements, with the bonus of named accessors.

```intent
// Want:  let t: (i64, OwnedStr) = (1, "hi" + "");   // rejected
struct Pair { id: i64, name: OwnedStr }              // works
let p: Pair = Pair { id: 1, name: "hi" + "" };
print p.name;
```

**Two type params → specialize at the concrete types.** When
you genuinely need two unrelated types in one signature, write
a concrete monomorphic helper per pair you need — `fn add_i64(
a: i64, b: i64) -> i64`, `fn label_str(s: OwnedStr) -> i64` —
and call the specific one. The single-type-param `<T>` form
covers the common identity-style cases; for everything else
specialize by hand.

```intent
// Want:  fn pair<A, B>(a: A, b: B) -> A   // rejected (2 type params)
// Specialize:
fn first_int(a: i64, _b: i64) -> i64 { return a; }
fn first_bool(a: bool, _b: bool) -> bool { return a; }
```

**No lexical capture → pass the value as an argument.** Inline
anonymous fns are the higher-order form vāṇी actually supports;
turn captures into explicit parameters.

```intent
// Want:  let base = 100; let f = |x| x + base;   // rejected
fn add_base(x: i64, base: i64) -> i64 {           // works
  return x + base;
}
// call sites just pass `base` themselves:
let r: i64 = add_base(5, 100);
```

**No bool ↔ int cast → use if-as-expression.** vāṇी's `if`
returns a value, so the explicit branch is one short line.

```intent
// Want:  let n: i64 = b as i64;          // rejected
let n: i64 = if b { 1 } else { 0 };       // works
```

**`Mutex<T>` is i64-only → pack the value as i64.** Booleans
and any integer ≤ 64 bits fit through this; for richer types
keep the data outside the Mutex and gate just the index /
generation counter.

```intent
let m: Mutex<i64> = mutex_new(42 as i64);   // value-by-value
let g: Guard<i64> = mutex_lock(ref m);
let v: i64 = guard_get(ref g);
```

**No recursive type aliases → write the struct directly.** The
restriction is on `type` aliases; nominal structs can recurse
through `Vec<Self>` (or `Box<Self>` when that lands) naturally.

```intent
// Want:  type Tree = Vec<Tree>;                  // rejected
struct TreeNode {                                 // works
  value: i64,
  children: Vec<TreeNode>,
}
```

**Nested partial moves → hoist the intermediate binding.** The
compiler tracks moves one field deep; do the inner level in two
steps so each step is a one-field move.

```intent
// Want:  let s: OwnedStr = o.inner.s;            // rejected
let inner: Inner = o.inner;                       // works
let s: OwnedStr = inner.s;
```

**`while` between `try` and `return` → hoist the loop into a
helper fn.** A separate function gives the `while` its own
return path, freeing the caller's `try`-rewriter to operate on
a flat statement list.

```intent
fn warm_cache() -> i64 {        // hoisted out of the try body
  let i: i64 = 0;
  while i < 5 { let _: i64 = i; }
  return 0;
}
fn run(o: Option<i64>) -> Option<i64> {
  let v: i64 = try o;
  let _: i64 = warm_cache();    // ordinary call site
  return Option.Some(v);
}
```

**Block-expression inner control flow → use if-as-expression.**
The block expression admits `let`/`print`/assignments only, but
`if … else …` is itself an expression and slots in directly.

```intent
// Want:  let r = { if cond { a } else { b } };   // rejected
let r: i64 = if cond { a } else { b };            // works
```

**No Rc / Arc for shared state → `Atomic<T>` or `Mutex<T>`.**
For lock-free counters / flags use `Atomic`; for compound
state use `Mutex<i64>` with the packing recipe above.

```intent
let shared: Atomic<i64> = atomic_new(0);
parallel for i from 0 to 10 {
  let _: i64 = atomic_fetch_add(ref shared, 1);
}
let total: i64 = atomic_load(ref shared);   // 10
```

**No exceptions → `Option<T>` + `try` / `?`.** The desugar
gives the same one-line happy-path as Rust's `?`, without an
unwinder or `try`/`catch` machinery. v1's `try` requires the
enclosing enum to have exactly **one payloaded** + **one
payload-less** variant — `Option<T>` (Some/None) fits, but
`Result<T, E>` (Ok/Err, both payloaded) does NOT, and you'll
hit "requires the enum to have exactly one payloaded variant
and one payload-less variant; got 2 payloaded and 0
payload-less". For Result-shaped flows, write the match by
hand, or define your own enum that matches the 1+1 shape.

```intent
fn divide(a: i64, b: i64) -> Option<i64> {
  if b == 0 { return Option.None; }
  return Option.Some(a / b);
}
fn run() -> Option<i64> {
  let r: i64 = divide(10, 2)?;   // propagates None
  return Option.Some(r);
}
```

What *does* work well (so the limitation list reads in context):
all 58 examples are leak-clean under `gcc -fsanitize=address,leak`,
UBSan-clean, LLVM `opt -verify` clean, cross-backend stdout-parity
tested, and SMT-verified (z3 discharge of `requires` / `ensures` /
`prove` / `invariant`). The `for` body of a `parallel for` is
proved race-free before lowering to OpenMP.

## Why Rust

Rust fits the compiler core because it gives:

- fast lexing, parsing, type checking, and lowering
- strong ownership and enum modeling for AST/IR invariants
- deterministic builds and single-binary distribution
- good FFI and ABI integration
- easy migration to Cranelift, LLVM, or direct assembly backends
- safe concurrency for future parallel compilation and optimization passes

Python still belongs in the system as:

- a research harness
- benchmark runner
- AI planning/orchestration layer
- fuzzing and corpus tooling
- notebook-style design exploration

---

# Part VII — Roadmap & Status

## Roadmap

The work splits into two queues: **small items** (each independently
landable, < 1 session) and **multi-session items** (each touches
checker + IR + multiple backends + tests, ordered by dependency then
effort). See [STATUS.md](STATUS.md) for the live "Known Issues" list and
[TODO.md](TODO.md) for the full closure history.

### Small items

These are contained surface gaps and diagnostic polishes. Most of the
"todo" side will land naturally when the corresponding multi-session item
lands.

**Done (most recent first):**

- ✅ Devanagari Sanskrit / Hindi / Marathi 3-way alias parity —
  `वरना` (else, Hindi), `परिवर्तनीय` (mut), `अग्रे` (continue,
  Sanskrit), `सार्वजनिक` (pub), `खण्ड` (module), `उपयोग` (use),
  `यथा` (as), `यत्र`/`जहाँ`/`जिथे` (where), `अस्ति`/`है`/`आहे`
  (is), `संकेत` (interface), `कार्यान्वित` (implement), `विधि`
  (methods), `प्रयास` (try), `नियोग` (task), `संयोजन` (join),
  `समानांतर` (parallel single-word) — closure #267
- ✅ Devanagari SOV verb-at-end statements — `X पुनरागम;` /
  `… लिखो;` / `cond सुनिश्चित;` / `expr प्रमाण;` — closure #266
- ✅ Devanagari SOV word order for range `for` — `i के लिए 0 से
  5 तक { … }` — closure #265
- ✅ SSA-LLVM multi-block parallel-for atomicrmw emit via
  Phi-traceback (closure #264) — multi-block bodies no longer
  fall back to tree-LLVM
- ✅ Codegen fixes: SSA-LLVM identity-cast `bitcast` for pointer
  types (#263); `len(ref OwnedStr)` 4-layer dereference fix (#262)
- ✅ `examples/memory_safety.vani` — 7 canonical safety patterns
  end-to-end (#261)
- ✅ Move-rejection diagnostic carries a type-aware fix hint —
  `ref x` for borrowing, `clone(x)` for deep copy, exclusive
  handles say "cannot be cloned" (#260)
- ✅ Parallel-for implicit-reduction race check — captured Copy
  mutation without `reduce` clause errors at compile time (#259)
- ✅ Namespaces / modules — `module foo { … }`, `pub` / `pub(kosh)`,
  `use foo::bar [as baz];` / `use foo::*;` / `use foo::{a, b};`,
  module-local `use`, `pub use` re-exports, nested modules with deep
  paths, orphan rules, collision diagnostics — closures #242–#258
- ✅ "Kosh" (कोश) adopted as vāṇī's word for the future crate concept;
  `pub(kosh)` accepted as preparatory syntax
- ✅ Keyword aliases: `assign` (let), `give` / `give_back` /
  `give back` (return)
- ✅ SSA Step 3b — multi-block parallel-for body in SSA-C (closure #251)
- ✅ Array types in fn return position (#239)
- ✅ Formatter support for module blocks + `use_paths` round-trip (#250)
- ✅ `clone_at` on `Vec<Struct>` tree-LLVM lowering
- ✅ Methods without `self` rejected with clean diagnostic
- ✅ Bare-block `{ … }` as statement — helpful diagnostic with workaround
- ✅ Compile-time short-circuit `&&` / `||` honors dead-code RHS
- ✅ Discarded `call();` / `receiver.method();` as a statement
- ✅ Sharper diagnostics for struct / tuple / enum `==` / `!=`
- ✅ `print` of struct / tuple / enum → targeted diagnostics (was: backend panic)
- ✅ Inner-`let` shadow leak in SSA `lower_if` fixed
- ✅ `ArrayLit` as direct fn argument (was: backend panic)
- ✅ Float negation in SSA-LLVM (was: invalid `sub double` IR)
- ✅ Empty `vec()` supported
- ✅ Vec-of-Vec / Vec-of-Struct end-to-end via `clone_at(ref xs, i)`
- ✅ `methods on T { fn m(self: T) … }` with field assignment + auto-ref
- ✅ Match: wildcards + integer + bool + string patterns + enum-to-int cast
- ✅ `if`-as-expression + `else if` chaining + Match phi fix
- ✅ Format polish: trailing commas everywhere, struct/methods round-trips
- ✅ Const decls + type aliases + const overflow check
- ✅ Discarded call-stmt sugar — `let _ = …` desugared at parse
- ✅ Composition coverage — 80+ probe + regression tests across the
  struct / enum / Vec / method / if-expr / match / const / type-alias /
  affine-shadow surfaces

**Todo (small):**

These either land naturally with a queued multi-session item, or are
deliberately deferred as v1 trade-offs.

- ✅ `const N` as array length `[T; N]` — works for previously-declared
  consts with an integer-literal initializer.
- ✅ Const initializer arithmetic — `const B: i64 = A + 1;` (and `* / - %`)
  folds at parse time across previously-declared integer consts.
- ✅ Array types in fn return position — `fn make() -> [i64; 3]`
  compiles + runs on both backends. Tree-C wraps via a per-shape
  struct (`intent_arr_ret_N_T`); tree-LLVM returns `[N x T]` by
  value natively. SSA-LLVM falls back to tree-LLVM for the
  stack-aliasing case. See [examples/array_return.vani](examples/array_return.vani).
- ✅ Nested arrays `[[T; N]; M]` and `[Vec<T>; N]` — closure #291
  Phases 1–4 (2026-05-27). Array-element-must-be-Copy restriction
  lifted; `clone_at(ref arr, i)` extended to arrays; per-slot
  per-field drops including struct-slot field walks; tree-LLVM
  `len` of a Vec rvalue (`len(clone_at(ref xs, i))`) spills to
  alloca, GEPs `.len`, loads.
- ✅ Empty struct `struct E {}` — useful for marker / zero-sized types.
- ✅ Unit-return functions — `fn f() { … }` without `-> Type` is sugar
  for `-> i64` with an implicit `return 0;` appended. Callers invoke as
  a bare statement (`f();`) or via `let _ = f();`. See
  [examples/unit_return.vani](examples/unit_return.vani).
- ✅ Type-associated functions `Type.helper(args)` — declare with
  `methods on T { fn helper(args) -> R { … } }` (no `self`); call as
  `T.helper(args)`. Constructors and other type-namespaced helpers.
  See [examples/type_associated_fn.vani](examples/type_associated_fn.vani).
- ⏳ `bool ↔ int` cast — different semantic domains, forces explicit
  `if cond { 1 } else { 0 }` and vice versa. Trade-off, may stay deferred.
- ✅ SSA bool-print parity — bool prints render as `true`/`false`
  through both SSA backends (closure #117 fixed the `1`/`0` gap).
- ✅ Bare `{ … }` as scope-stmt — provides an explicit nested scope
  for binding visibility. Desugars to `if true { … }` at parse time.
- ✅ `xs[i].field = v` mixed-place assign — including deep paths
  (`xs[i].a.b = v`); each intermediate segment must be a Copy struct
  and the leaf field must be Copy.
- ⏳ Generic function call sites — parses, gated diagnostic, lands with T1.4.
- ⏳ Enum payload variants — parses, gated diagnostic, lands with T1.3 phase 2b.
- ✅ Match on float scrutinee — closure #278 (2026-05-27).
  `Pattern::Float(f64)` AST variant + `check_match_float` desugars
  to a nested IfExpr chain; diagnostics for missing wildcard,
  duplicate literals, NaN-in-pattern, wrong scrutinee type.
(Tuple / struct / enum `==` all ship today — see the
"Generics & interfaces" section above.)

**Trade-offs (working as intended, not on the queue):**

No cross-compilation; Windows parallel-for thread count hardcoded N=4;
references second-class (param-only); natural-exit `!cond` post-loop fact
dropped when body can `break`; `prove foo(args)` requires `foo` to have
`ensures`; `INTENTC_NO_VERIFY=1` skips SMT (dev opt-out, never in CI).

### Multi-session items

Ordered by **dependency first, then effort** (lowest effort wins among
items with the same dependency level). Each fully closes a queued
roadmap surface and unblocks the items below it.

| # | Item | Depends on | Est. effort | Unlocks |
|---|---|---|---|---|
| 1 | ✅ **Block expressions** `let r = { stmts; tail-expr };` | — | low/medium | done 2026-05-21; see [examples/block_expressions.vani](examples/block_expressions.vani) |
| 2 | ✅ **SMT modeling — if-expr, match, struct field access, method calls** | — | medium | done 2026-05-21 (#82 + #84 — full coverage) |
| 3 | ✅ **T1.2 phase 2b: affine struct fields** | — | medium/high | done 2026-05-21 — `struct { … }` admits `OwnedStr`, `Vec<T>`, `[T;N]` of Copy elements, `Task`, `Atomic<T>` as fields; both backends free heap fields (OwnedStr, Vec) at scope exit; struct-literal init moves the source binding; `t.data[i]` indexing works. See [examples/struct_owned_field.vani](examples/struct_owned_field.vani), [examples/struct_mixed_fields.vani](examples/struct_mixed_fields.vani). Mutex/Guard/Channel still need explicit wiring. |
| 4 | ✅ **T1.3 phase 2b: tagged-union codegen + pattern bindings** | — | high | done 2026-05-21 — see [examples/option_types.vani](examples/option_types.vani); both backends |
| 5 | ✅ **T2.6: `try` keyword sugar for Option-like enums** | T1.3 phase 2b | low/medium | done 2026-05-21 — see [examples/try_keyword.vani](examples/try_keyword.vani). Generic Option<T> / Result<T, E> wait on #6 monomorphization. |
| 6 | ✅ **T1.4 phase 2: generic call-site monomorphization** | — | high | done 2026-05-21 — pass-through generics specialize per call-site literal type; see [examples/generic_functions.vani](examples/generic_functions.vani). Var-arg inference + interface bounds pending. |
| 7 | ✅ **T1.5 phase 2 + 3: interface dispatch (static + dynamic) + bounded generics** | T1.4 phase 2 | medium/high | done 2026-05-25 — static `recv.method()` dispatch + bounded generics done 2026-05-21; `dyn Iface` fat-pointer dispatch (owned, `ref dyn`, `Vec<dyn>`, struct fields of dyn) shipped via closures #220-#228, see [examples/dyn_dispatch.vani](examples/dyn_dispatch.vani). |
| 8 | ✅ **T2.7: user-defined Drop interface (auto-call at scope exit)** | T1.5 phase 2, #3 | low/medium | done 2026-05-25 — `implement Drop for T` runs automatically at scope exit. Two signatures supported: `fn drop(self: T)` (by-value, consumes self — only valid when T has no heap-owning fields) and `fn drop(self: mut ref T)` (runs first then per-field free — works for any T including OwnedStr / Vec / nested-struct fields, closure #229). See [examples/drop_interface.vani](examples/drop_interface.vani). |
| 9 | ✅ **Devanagari keyword aliases — Sanskrit / Hindi / Marathi (Phase 1 + 2)** | — | medium/high | Phase 1 done 2026-05-21 (single-word aliases + multi-word fusion `नहीं तो` / `के लिए` / `सिद्ध करो`). Phase 2 done 2026-05-26/27 (closures #265–#267): SOV word order for range `for` (`i के लिए 0 से 5 तक { … }`) and verb-at-end statements (`X पुनरागम;` / `… लिखो;` / `cond सुनिश्चित;` / `expr प्रमाण;`), plus 3-way alias parity for the previously English-only keywords. Grammar-consultant refinement pass still welcome. See [examples/hindi_keywords.vani](examples/hindi_keywords.vani), [examples/sanskrit_keywords.vani](examples/sanskrit_keywords.vani), [examples/marathi_keywords.vani](examples/marathi_keywords.vani). |
| 10 | ✅ **Namespaces — modules, visibility, use, kosh** | — | high | done 2026-05-26 across closures #242–#258. `module foo { … }` blocks (inline + nested + deep `a::b::c::Item` paths), per-item `pub` / `pub(kosh)` visibility, `use foo::bar [as baz];` / `use foo::{a, b};` / `use foo::*;` import forms (top-level AND inside module bodies), `pub use foo::bar;` re-exports (transitively resolved), orphan rules for `implement Iface for T`, collision diagnostics, formatter round-trip. See [examples/modules.vani](examples/modules.vani) and the *Modules and namespaces* section above. The full kosh package-manager arc (manifest, resolver, registry, stdlib-as-kosh) is still on the deferred queue — see [TODO.md](TODO.md) item #10. |
| 11 | ✅ **SSA-LLVM multi-block parallel-for body — atomicrmw emit** | #10 (SSA Step 3b recognizer) | medium/high | done 2026-05-26 (closure #264). The recognizer (#241) accepts multi-block bodies; SSA-C emit landed (#251); SSA-LLVM Phi-traceback now locates the actual reduction-update across conditional branches and replaces it with atomicrmw at its production site. Multi-block bodies (e.g. `parallel for { if cond { acc = acc + i; } }`) no longer fall back to tree-LLVM — they lower directly to atomicrmw in the outlined fn. |
| 12 | ✅ **FFI v1–v8 (`extern "C" fn` end-to-end)** | — | high | done 2026-05-27 across closures #269–#274, #279, #285, #288. `extern "C" fn` declarations, `--link-with PATH` / `-l<name>` flags, extern call-site checker, mangled-symbol codegen, struct-by-value rejection with `ref T` hint, callbacks via `Type::FnPtr`, System V x86-64 small-struct return lowering. Net: `qsort`-style callbacks and libc string / math interop work end-to-end without a runtime shim. |
| 13 | ✅ **vani.toml manifest (v1 + v2 [deps])** | — | medium | done 2026-05-27 (#280 + #287). Hand-rolled minimal-TOML parser, `find_manifest` parent-walk, `[package].entry` auto-discovery, `[deps]` inline-table for multi-file dependency wiring. |
| 14 | ✅ **Generic struct + enum declarations** | #6 | high | done 2026-05-27 (#281 + #282). `Type::Apply { name, args }` for parse-time generic instantiations; mangled names like `Result__Vec_I64___AllocError`; `Option<T>` / `Result<T, E>` / `AllocError` injected at AST level as prelude. |
| 15 | ✅ **Mixed-payload enums + `try_vec(n) -> Result<Vec<i64>, AllocError>`** | #14 | medium/high | done 2026-05-27 (#283 + #284). C uses tagged union `union { Type0 v_Ok; Type1 v_Err; }`; LLVM uses `[N x i8]` byte buffer + per-variant bitcast. `try_vec` builtin emits malloc + null-check + Result construction. |
| 16 | ✅ **Attribute syntax + `#[bounded(N)]`** | — | medium | done 2026-05-27 (#286, #289, #290). First attribute in the language. New `#` token + parser; tree-LLVM uses thread-local globals + per-Return decrement; SSA-LLVM mirrors the pattern; C emits a thread-local counter with GCC `__attribute__((cleanup))` for the decrement. |
| 17 | ✅ **Nested arrays `[[T; N]; M]` / `[Vec<T>; N]`** | — | medium | done 2026-05-27 (#291 Phases 1–4). Array-element Copy restriction lifted; `clone_at(ref arr, i)` extended to arrays; per-slot per-field drops including struct-slot field walks; tree-LLVM `len` of a Vec rvalue spills to alloca, GEPs `.len`, loads. |
| 18 | ⏳ **Data structures + algorithms roadmap (Levels 1–4)** | #14 (for Level 2+) | high (multi-session) | Levels 1–4 sequenced under affine ownership. Level 1: `sort` / `sort_by` / `find` / `binary_search` / `pop` / RNG / Hash interface. Level 2: `HashSet` / `HashMap` (⚠️ AFFINE-TENSION — `get -> Option<ref V>`) / `BTreeSet` / `BTreeMap` / `Deque` / `BinaryHeap`. Level 3: closures + iterator combinators. Level 4: arena-based BST / B-tree / Trie / graphs + algorithms. Full per-item plan in [TODO.md](TODO.md). |
| 19 | ✅ **Condition variables (`Condvar`)** | — | medium (single session) | done 2026-05-28 (closure #292). ✅ AFFINE — new builtin type, stack-by-value. 5 builtins (`condvar_new / wait(ref cv, mut ref g: Guard<i64>) / wait_timeout / notify_one / notify_all`). Tree-C + SSA-C: shared runtime helpers (futex/WaitOnAddress/spin-yield). Tree-LLVM: inline IR per call site (`%intent_condvar = type { i32 }`, atomicrmw + syscall/WakeByAddress). SSA-LLVM: falls back to tree-LLVM. 5 lib tests + `examples/condvar.vani` cross-backend parity. Pending follow-ups: cross-task wait/notify (needs task-capture rule expansion), direct SSA-LLVM support, wider Mutex widths. |
| 20 | ✅ **Async / asyncio** — SHIPPED 2026-06-08 (Arc 8 v1+v1.5+v1.6+v2+v3.1). ⚠️ AFFINE-TENSION via compiler-lowered state machines; 🛑 NOT Pin / self-references | Level 3 closures (#18) | high (multi-session) | Each `async fn` lowers (via Arc 8 v3.1 parser-level transform) to a struct/poll/constructor triple. Builtin TCP + epoll + non-blocking I/O families for single-thread cooperative scheduling; `Channel<T, N>` coordination primitive; `Future<T>` / `Poll<T>` / `CancelToken`. Linux verified; macOS kqueue + Windows IOCP branches ship with deferred host verification. 28 acceptance examples + generic-async smoke cross-backend parity-green. Not shipping: Rust-style `Pin<&mut Self>`, panic-based cancellation, stackful coroutines, async inside `parallel for`. See [ARC8_V3_PLAN.md](ARC8_V3_PLAN.md). |
| 21 | ✅ **Kosh package manager + Vāṇī-Kosh registry** — SHIPPED 2026-06-17 | #10, #13 | high (multi-session) | `vani.toml` with `[package].version` + `[deps]` version constraints; `vani.lock` writer; `vanic vendor` / `vanic add` / `vanic remove` / `vanic search` / `vanic update` / `vanic publish`. SHA-256 checksum verification on download. Live sparse registry at [enthusiasticgeek.github.io/kosh-index](https://enthusiasticgeek.github.io/kosh-index/). Gated publish: Publisher Agreement v1.0 + operator approval + blacklist via `governance.json`. `vanic apply-publisher` / `registry-approve` / `registry-blacklist` commands. See [docs/kosh_design.md](docs/kosh_design.md). |

**Devanagari aliases (#9) — current state + remaining work:**

**Phase 1** (closures #235–#237; 2026-05-21). The lexer recognizes
single-word Devanagari aliases (Sanskrit / Hindi / Marathi) for
`fn` / `let` / `return` / `if` / `else` / `while` / `for` / `prove`
and friends, plus multi-word phrases via a post-lex merger
(`नहीं तो` → else, `के लिए` → for, `सिद्ध करो` → prove). Per-language
**purity v1** lets users opt a file into a single language (Hindi /
Sanskrit / Marathi / English) via a header marker; the checker then
rejects out-of-language identifiers.

**Phase 2** (closures #265–#267; 2026-05-26/27) — closes the two
biggest ergonomic gaps:
- **SOV word-order parsing** (#265 + #266). Range `for` now
  accepts the natural Indo-Aryan shape `i के लिए 0 से 5 तक { … }`
  (variable + `के लिए`; operands + `से` / `तक` postpositions),
  and the four verb-like statements accept the verb-at-end form
  (`X पुनरागम;` = return; `… लिखो;` = print; `cond सुनिश्चित;`
  = assert; `expr प्रमाण;` = prove). The detector keys off
  Ident-followed-by-verb or scan-to-`;`-ending-in-verb so the
  English keyword-first forms still parse.
- **3-way alias parity** (#267). Every previously English-only
  keyword now has a Sanskrit / Hindi / Marathi alias —
  `वरना` (else, Hindi), `परिवर्तनीय` (mut, Sanskrit/Hindi),
  `अग्रे` (continue, Sanskrit), `सार्वजनिक` (pub, all three),
  `खण्ड` (module, all three), `उपयोग` (use, all three),
  `यथा` (as, all three), `यत्र`/`जहाँ`/`जिथे` (where, per
  language), `अस्ति`/`है`/`आहे` (is, per language), plus
  interface / implement / methods / try / task / join /
  parallel single-word. Sanskrit-root tatsama forms (e.g.
  `संरचना` = struct) are documented as shared rather than
  duplicated. A pure-Hindi or pure-Sanskrit or pure-Marathi
  program now reads top-to-bottom with no English fall-back.

**Still queued:**
- **Grammar-consultant refinement.** Phase-2 verb picks are
  best-effort; idiomatic dialect-specific revision is welcome.
- **Script-aware diagnostics (9d).** Errors today emit in
  English; a per-source-script diagnostic mode is queued.

**Long-term beyond v1**

- Cranelift backend (fast native JIT, no LLVM dependency).
- Direct-asm targets (x86_64-linux first, then small-targets).
- Work-stealing scheduler for `task` fan-out.
- SVE/SVE2 `vec256<T>` / `vec512<T>` scalable-vector types.
- GPU / accelerator backends.
- Richer aliasing rules — region / lifetime inference beyond the
  second-class `ref` / `mut ref` discipline.
- AI collaboration: keep human-readable source as the authority, let AI
  produce candidate algorithms, constraints, proofs, tests, and
  target-specific optimizations. The compiler verifies the candidates
  before accepting them.

---

# Part VIII — Community

## Contributing

VANI is an open-source research compiler. Patches, bug reports, and
example programs are all welcome.

- [INSTALL.md](INSTALL.md) — per-platform install instructions
  (Linux / macOS / Windows) with the package commands for
  `z3`, LLVM tools, and the Rust toolchain plus a verify-
  your-install checklist.
- [CONTRIBUTING.md](CONTRIBUTING.md) — pre-PR checklist, code
  conventions, commit-message style, and how to file issues.
- [ONBOARDING.md](ONBOARDING.md) — toolchain prerequisites, project
  layout, and an end-to-end "add a feature" walkthrough.
- [STATUS.md](STATUS.md) — single-page snapshot of the current feature
  set, the priority-ordered TODO queue, and known issues.
- [docs/v1_limitations.md](docs/v1_limitations.md) — single
  catalog of every known v1 deviation from textbook behavior
  (codegen quirks, parser shortcuts, by-design choices) with
  per-entry workarounds + fix-queue pointers.
- [examples/language/english/design_patterns/](examples/language/english/design_patterns/) —
  all 22 Gang-of-Four design patterns implemented in vāṇी
  ([readme](examples/language/english/design_patterns/README.md)
  lists the per-pattern vāṇी deviations from textbook GoF).

## Language targeting (Indian-subcontinent-first, then global)

> The English-keyword default is the day-one path for most users
> and won't change. This section covers the natural-language
> rollout queued on top of it. Skip if you only care about the
> English surface.

> **⚠️ Caveat — every dialect's keyword table needs native-
> speaker review.** The vāṇī authors are fluent in English and
> have first-hand familiarity with the Devanagari Indo-Aryan
> family (Sanskrit / Hindi / Marathi as primary; Nepali /
> Maithili / Konkani-Devanagari as close relatives). Every
> other dialect listed below (Bengali, Tamil, Telugu, Gujarati,
> Punjabi, Kannada, Malayalam, Odia, Assamese, Sinhala, Urdu,
> Sindhi, Persian, Pashto, Mandarin, Japanese, Korean, Arabic,
> Hebrew, Greek, Russian, Thai, Khmer, Burmese, Amharic,
> Tibetan, Mongolian, Armenian, Georgian, Cherokee, Lao,
> Spanish, French, German, Italian, Portuguese, Polish,
> Turkish, Vietnamese, Romanian, Dutch, Hungarian, Czech,
> Slovak, Swedish, Norwegian, Danish, Finnish, Catalan,
> Yoruba, Hausa, Swahili, Indonesian, Malay, Filipino) was
> drafted from reference grammars + tatsama/loan-word patterns
> + CS-vocabulary conventions, but has **NOT been validated
> by a native speaker**. The keyword choices may sound wrong,
> overly formal, or archaic to fluent users. **If you read
> any of these languages natively, please open an issue or
> PR — the lexer table is one file and corrections are
> mechanical to merge.** Treat the non-Devanagari-Indo-Aryan
> dialects as *technical proofs-of-concept* until the grammar-
> consultant pass (queued in [TODO.md](TODO.md)) lands.

vāṇी treats human-spoken languages as a first-class concern in
addition to its English default. The adoption order is **Indian
subcontinent languages first**, then global. The reasoning: SOV
(Subject–Object–Verb) word order + Devanagari-script-or-relative
writing systems are widely shared across the Indian subcontinent,
so a single parser + lexer abstraction extends naturally across
all of them. After that, the global rollout adds languages with
significantly different grammar (SVO, head-final, RTL,
logographic, etc.) one at a time.

### Tier I — Indian subcontinent (priority)

> Major languages of the Indian subcontinent, ordered by speaker
> count + typological proximity to the already-shipped Sanskrit-
> derived three. Devanagari-script languages are easiest to wire
> up (they share the existing lexer pipeline); Brahmi-derived
> non-Devanagari scripts (Tamil / Telugu / Kannada / Malayalam /
> Gujarati / Punjabi-Gurmukhi / Bengali / Odia / Assamese / Sinhala)
> require a per-script Unicode-block extension in
> `enforce_language_purity` but share the SOV grammar pattern.

| # | Language | Script | Status |
|---|---|---|---|
| 1 | Sanskrit (*saṁskṛta*) | Devanagari | ✅ **SHIPPED** — 91 keyword aliases, 8 SOV statement shapes, per-dialect purity pragma, 11 example programs (including a pure-Devanagari Pascal's-triangle showcase) |
| 2 | Hindi (*hindī*) | Devanagari | ✅ **SHIPPED** — same surface as Sanskrit; 9 example programs |
| 3 | Marathi (*marāṭhī*) | Devanagari | ✅ **SHIPPED** — same surface; 9 example programs |
| 4 | Bengali (*baṅlā*) | Bengali (Brahmi-derived) | ✅ **SHIPPED** (Phase 5b, 2026-06-07) — first non-Devanagari Brahmi script; `// vani-lang: bengali`; ~50 keyword aliases; per-script purity gate generalized; Bengali-numeral PRINT helper (`০..৯` at U+09E6..09EF) on all four backends; [`examples/language/bengali/basics.vani`](examples/language/bengali/basics.vani) |
| 5 | Gujarati (*gujarātī*) | Gujarati (Brahmi-derived) | ✅ **SHIPPED** (Phase 6, 2026-06-07) — `// vani-lang: gujarati`; ~35 starter aliases (tatsama-friendly); Gujarati numerals ૦..૯ at U+0AE6..0AEF; [`examples/language/gujarati/basics.vani`](examples/language/gujarati/basics.vani) |
| 6 | Punjabi (*pañjābī*) | Gurmukhi (Brahmi-derived) + Shahmukhi (Perso-Arabic, RTL) | ✅ **SHIPPED — both scripts** — Gurmukhi (Phase 6, `// vani-lang: punjabi`) at U+0A00..0A7F; Shahmukhi (Phase 12.3, `// vani-lang: punjabi-shahmukhi`) shares Urdu's Perso-Arabic helper + dialect tag; [Gurmukhi example](examples/language/punjabi/basics.vani), [Shahmukhi example](examples/language/punjabi_shahmukhi/basics.vani) |
| 7 | Tamil (*tamiḻ*) | Tamil (Brahmi-derived, distinct family — Dravidian, not Indo-Aryan) | ✅ **SHIPPED** (Phase 6, 2026-06-07) — `// vani-lang: tamil`; ~35 starter aliases drawn from native Tamil verbs/nouns (not tatsama); Tamil numerals ௦..௯ at U+0BE6..0BEF; [`examples/language/tamil/basics.vani`](examples/language/tamil/basics.vani) |
| 8 | Telugu (*telugu*) | Telugu | ✅ **SHIPPED** (Phase 6, 2026-06-07) — `// vani-lang: telugu`; ~32 starter aliases (mix of native Telugu + tatsama); Telugu numerals ౦..౯ at U+0C66..0C6F; [`examples/language/telugu/basics.vani`](examples/language/telugu/basics.vani) |
| 9 | Kannada (*kannaḍa*) | Kannada | ✅ **SHIPPED** (Phase 6, 2026-06-07) — `// vani-lang: kannada`; ~35 starter aliases (tatsama + native); Kannada numerals ೦..೯ at U+0CE6..0CEF; [`examples/language/kannada/basics.vani`](examples/language/kannada/basics.vani) |
| 10 | Malayalam (*malayāḷam*) | Malayalam | ✅ **SHIPPED** (Phase 6, 2026-06-07) — `// vani-lang: malayalam`; ~33 starter aliases; Malayalam numerals ൦..൯ at U+0D66..0D6F; [`examples/language/malayalam/basics.vani`](examples/language/malayalam/basics.vani) |
| 11 | Urdu (*urdū*) | Perso-Arabic (RTL) | ✅ **SHIPPED** (Phase 12, 2026-06-07) — first non-Brahmi / first RTL script; `// vani-lang: urdu`; ~35 starter aliases mixing Persian/Arabic technical vocab with Hindustani conversational forms; Eastern Arabic-Indic numerals ٠..٩ at U+0660..0669 (2-byte UTF-8); the print-helper template parameterizes on prefix-byte length to cover both 2-byte Arabic-Indic and 3-byte Brahmi forms; [`examples/language/urdu/basics.vani`](examples/language/urdu/basics.vani) |
| 12 | Odia (*oṛiā*) | Odia (Brahmi-derived) | ✅ **SHIPPED** (Phase 6, 2026-06-07) — `// vani-lang: odia` (alias `oriya`); ~33 starter aliases (tatsama-friendly); Odia numerals ୦..୯ at U+0B66..0B6F; [`examples/language/odia/basics.vani`](examples/language/odia/basics.vani) |
| 13 | Assamese (*ɔxɔmia*) | Assamese (Brahmi-derived; close to Bengali script) | ✅ **SHIPPED** (Phase 6, 2026-06-07) — `// vani-lang: assamese`; reuses the Bengali keyword table + numerals (the two scripts differ by only `ৰ` / `ৱ`); [`examples/language/assamese/basics.vani`](examples/language/assamese/basics.vani) |
| 14 | Sindhi (*sindhī*) | Perso-Arabic (RTL) + Devanagari (rare) | ✅ **SHIPPED — Perso-Arabic** (Phase 12.2, 2026-06-07) — `// vani-lang: sindhi`; reuses Urdu's keyword union + Eastern Arabic-Indic numeral helper; Devanagari Sindhi variant (rare) deferred; [`examples/language/sindhi/basics.vani`](examples/language/sindhi/basics.vani) |
| 15 | Nepali (*nepālī*) | Devanagari | ✅ **SHIPPED** (Phase 2.1, 2026-06-07) — dialect tag `// vani-lang: nepali`; accepts the Sanskrit/Hindi/Marathi keyword union; [`examples/language/nepali/basics.vani`](examples/language/nepali/basics.vani) |
| 16 | Konkani (*kõkaṇī*) | Devanagari + Kannada + Roman + Malayalam (multi-script) | ✅ **SHIPPED — Devanagari only** (Phase 2.3, 2026-06-07) — dialect tag `// vani-lang: konkani`; non-Devanagari scripts (Kannada / Roman) deferred; [`examples/language/konkani/basics.vani`](examples/language/konkani/basics.vani) |
| 17 | Maithili (*maithilī*) | Devanagari + Tirhuta (historic) | ✅ **SHIPPED — Devanagari only** (Phase 2.2, 2026-06-07) — dialect tag `// vani-lang: maithili`; Mithilakshar/Tirhuta script deferred; [`examples/language/maithili/basics.vani`](examples/language/maithili/basics.vani) |
| 18 | Sinhala (*siṁhala*) | Sinhala (Brahmi-derived) | ✅ **SHIPPED** (Phase 6, 2026-06-07) — `// vani-lang: sinhala`; ~33 starter aliases (tatsama + native); Sinhala Lith Illakkam numerals ෦..෯ at U+0DE6..0DEF; [`examples/language/sinhala/basics.vani`](examples/language/sinhala/basics.vani) |
| ... | (smaller subcontinent languages) | various | Queued |

### Tier II — Global (after Tier I)

> Major world languages with non-Indic scripts and grammars. The
> per-script abstraction from Phase 5b (Bengali) plus the
> pragma-threading enabler (commit `c3a2bb6`) made the global
> rollout possible — adding a new dialect is now a 6-touchpoint
> mechanical change (Script + classify range + DialectLang +
> pragma + keyword table + DiagLang) regardless of script family.

| # | Language | Script | Word order | Status |
|---|---|---|---|---|
| 1 | Spanish (*español*) | Latin | SVO | ✅ **SHIPPED** (Phase 8b.1) — natural ASCII + accented surface (`función`, `si`, `para`, `verdadero`, `imprimir`) |
| 2 | French (*français*) | Latin | SVO | ✅ **SHIPPED** (Phase 8b.3) — `fonction`, `si`, `pour`, `vrai`, `écrire` |
| 3 | German (*deutsch*) | Latin | V2 + subordinate SOV | ✅ **SHIPPED** (Phase 10.1) — `funktion`, `wenn`, `solange`, `wahr`, `drucken` |
| 4 | Russian (*русский*) | Cyrillic | SVO + free order | ✅ **SHIPPED** (Phase 8b.2) — first Cyrillic dialect (`функция`, `если`, `пока`, `печатать`) |
| 5 | Italian (*italiano*) | Latin | SVO | ✅ **SHIPPED** (Phase 13.6) — Romance family completes |
| 6 | Portuguese (*português*) | Latin | SVO | ✅ **SHIPPED** (Phase 13.2) — `função`, `seja`, `enquanto`, `imprimir` |
| 7 | Polish (*polski*) | Latin (ą/ć/ę/ł/ń/ó/ś/ź/ż) | SVO | ✅ **SHIPPED** (Phase 13.8) — first Slavic Latin |
| 8 | Turkish (*Türkçe*) | Latin (ç/ğ/ı/İ/ö/ş/ü) | SOV (agglutinative) | ✅ **SHIPPED** (Phase 13.9) — Turkic family |
| 9 | Vietnamese (*Tiếng Việt*) | Latin (extensive tone marks) | SVO | ✅ **SHIPPED** (Phase 13.12) — first Southeast Asian Latin |
| 10 | Romanian (*română*) | Latin (ă/â/î/ș/ț) | SVO | ✅ **SHIPPED** (Phase 13.13) — Romance family |
| 11 | Dutch (*Nederlands*) | Latin | V2 + SOV | ✅ **SHIPPED** (Phase 13.14) — Germanic |
| 12 | Hungarian (*magyar*) | Latin (ő/ű + standard) | SOV (agglutinative) | ✅ **SHIPPED** (Phase 13.16) — Uralic family |
| 13 | Czech (*čeština*) | Latin (ř + háček marks) | free order | ✅ **SHIPPED** (Phase 13.17) — second Slavic |
| 14 | Slovak (*slovenčina*) | Latin | free order | ✅ **SHIPPED** (Phase 13.24) — third Slavic |
| 15 | Swedish (*svenska*) | Latin (å/ä/ö) | SVO | ✅ **SHIPPED** (Phase 13.18) — first Nordic |
| 16 | Norwegian (*norsk bokmål*) | Latin (å/æ/ø) | SVO | ✅ **SHIPPED** (Phase 13.20) — second Nordic |
| 17 | Danish (*dansk*) | Latin (å/æ/ø) | SVO | ✅ **SHIPPED** (Phase 13.21) — third Nordic |
| 18 | Finnish (*suomi*) | Latin (ä/ö) | SVO (agglutinative) | ✅ **SHIPPED** (Phase 13.25) — second Uralic |
| 19 | Catalan (*català*) | Latin | SVO | ✅ **SHIPPED** (Phase 13.26) — sixth Romance |
| 20 | Modern Standard Arabic (*العربية*) | Arabic (RTL) | VSO/SVO | ✅ **SHIPPED** (Phase 13.7) — distinct from shipped Perso-Arabic dialects |
| 21 | Korean (*한국어*) | Hangul (new) | SOV | ✅ **SHIPPED** (Phase 13.1) — first Hangul-script dialect |
| 22 | Japanese (*日本語*) | Kanji + Hiragana + Katakana | SOV | ✅ **SHIPPED** (Phase 9b) — first three-script collapsed Script variant |
| 23 | Greek (*Ελληνικά*) | Greek (new) | SVO | ✅ **SHIPPED** (Phase 13.4) |
| 24 | Hebrew (*עברית*) | Hebrew (new, RTL) | SVO | ✅ **SHIPPED** (Phase 13.5) — second RTL after Perso-Arabic |
| 25 | Thai (*ไทย*) | Thai (new) | SVO | ✅ **SHIPPED** (Phase 13.15) |
| 26 | Khmer (*ខ្មែរ*) | Khmer (new) | SVO | ✅ **SHIPPED** (Phase 13.29) |
| 27 | Burmese (*မြန်မာ*) | Myanmar (new) | SOV | ✅ **SHIPPED** (Phase 13.30) |
| 28 | Lao (*ລາວ*) | Lao (new) | SVO | ✅ **SHIPPED** (Phase 13.34) — Thai sibling |
| 29 | Amharic (*አማርኛ*) | Ethiopic (new) | SOV | ✅ **SHIPPED** (Phase 13.31) — first Ethiopian dialect |
| 30 | Tibetan (*བོད་ཡིག*) | Tibetan (new) | SOV | ✅ **SHIPPED** (Phase 13.32) |
| 31 | Cherokee (*ᏣᎳᎩ*) | Cherokee syllabary (new) | SOV | ✅ **SHIPPED** (Phase 13.33) — endangered, preservation step |
| 32 | Mongolian (*ᠮᠣᠩᠭᠣᠯ*) | Mongolian traditional (new) | SOV | ✅ **SHIPPED** (Phase 13.35) |
| 33 | Armenian (*Հայերեն*) | Armenian (new) | SOV | ✅ **SHIPPED** (Phase 13.22) — first Caucasus script |
| 34 | Georgian (*ქართული*) | Georgian Mkhedruli (new) | SOV | ✅ **SHIPPED** (Phase 13.23) — second Caucasus script |
| 35 | Indonesian (*Bahasa Indonesia*) | Latin (no diacritics) | SVO | ✅ **SHIPPED** (Phase 13.3) — first basic-Latin pragma-threaded |
| 36 | Malay (*Bahasa Melayu*) | Latin | SVO | ✅ **SHIPPED** (Phase 13.10) — Indonesian sibling |
| 37 | Filipino (*Tagalog*) | Latin | VSO | ✅ **SHIPPED** (Phase 13.19) — Austronesian |
| 38 | Swahili (*Kiswahili*) | Latin | SVO | ✅ **SHIPPED** (Phase 13.11) — first East African (Bantu) |
| 39 | Yoruba (*Èdè Yorùbá*) | Latin (ẹ/ọ/ṣ + tone marks) | SVO | ✅ **SHIPPED** (Phase 13.27) — Niger-Congo |
| 40 | Hausa | Latin (ɓ/ɗ/ƙ/ƴ) | SVO | ✅ **SHIPPED** (Phase 13.28) — Afroasiatic |
| 62 | Mandarin Chinese (*中文*) | Han logograms (no whitespace tokenizer) | SVO | ✅ **SHIPPED** (2026-06-08) — 62nd dialect; CJK word-segmentation arc completed |

### Why Indian-subcontinent-first

1. **Underserved by mainstream programming languages.** The
   subcontinent's 1.4B speakers + ~600M secondary speakers have
   essentially zero programming-language support in their mother
   tongues. Every mainstream language assumes English keywords.
2. **Typological cohesion**. SOV + postpositions + Brahmi-or-
   relative scripts mean the parser/lexer abstraction generalizes
   cleanly across the family. Tier I rolls out fast once the first
   three languages ship (which they have).
3. **Cultural alignment.** vāṇी's name (वाणी = "speech"), Sanskrit
   provenance, and the अ → ज्ञ pronunciation conventions all root
   the project in the subcontinent. Honoring that with
   first-class support is the design promise.

After the subcontinent family is comprehensive (~18 languages in
Tier I), Tier II opens with Spanish — the simplest non-SOV
addition — and works through the global priority list.

> **Status (2026-06-06 evening)**: SOV + natural-speech coding
> for Sanskrit / Hindi / Marathi is **Devanagari-purity-arc
> complete**:
>
> - **91 Devanagari aliases** cover **46 of 46** structure keywords
>   ([lexer.rs:222–372](src/lexer.rs#L222-L372)) — full coverage.
> - **SOV word order** wired for **8 statement shapes**: range `for`
>   loops + **`let` binding verb-at-end** (`x: i64 = 5 माना;`) +
>   `if`/`else` block-form + `while` block-form + four verb-at-end
>   stmts (`return` / `print` / `assert` / `prove`)
>   ([parser.rs:2277–2328](src/parser.rs#L2277-L2328) +
>   [parser.rs:3404–3434](src/parser.rs#L3404-L3434)).
> - Other constructs (`fn`, `struct`, `enum` declarations,
>   `match`-as-stmt) **stay keyword-first by design** — Sanskrit
>   `यदि...तर्हि` / Hindi `अगर...तो` are naturally keyword-first
>   in Indo-Aryan grammar; forcing verb-at-end here would feel
>   unnatural. See [TODO.md](TODO.md) §*Why some constructs stay
>   keyword-first* for the per-construct rationale.
> - **Per-file purity** is at the script level (English vs
>   Devanagari, [lexer.rs:393–441](src/lexer.rs#L393-L441)). Finer-
>   grained Sanskrit-vs-Hindi-vs-Marathi enforcement opt-in via
>   `// vani-lang: <dialect>` pragma (SOV-S8).
> - **Dialect-aware error rendering**: pragma-tagged files render
>   diagnostics with Sanskrit / Hindi / Marathi labels +
>   message-prefix translations.
> - **Devanagari numerals + type names + identifiers** all
>   first-class. Pure-Devanagari programs run on both backends —
>   see [`examples/language/sanskrit/pure_devanagari.vani`](examples/language/sanskrit/pure_devanagari.vani).
> - **Cross-language translator** ships at
>   [`tools/vani_translate.py`](tools/vani_translate.py) — rewrite
>   any `.vani` source between English / Sanskrit / Hindi /
>   Marathi, round-trip parity verified on 8 representative
>   examples.
> - **Global languages**: Mandarin Chinese ✅ shipped 2026-06-08
>   (62nd dialect); Spanish + other Romance/Germanic/Slavic families
>   queued. The lexer table receives them readily; remaining work is
>   curating the keyword sets per family.
>
> See [TODO.md](TODO.md) §*Open work — DEPENDENCY-ORDERED* for the
> dependency-ordered remaining queue.

## Glossary

A compact reference for terms used throughout this README. Most
are standard PL / compiler / verification vocabulary; the
**Note** column flags places where vāṇी uses a term in a
specific way (or with a stronger guarantee than the default).

### Ownership, references, and lifetimes

| Term | Meaning |
|---|---|
| **affine** | A value that can be used **at most once** (consume or drop, never both). vāṇी's affine ownership is what makes use-after-move, double-free, and double-close detectable at compile time. Stricter than C++ "move-only"; weaker than fully **linear** (which requires *exactly* once). |
| **linear** | Used *exactly* once — never dropped silently. vāṇी is affine, not linear: an unused value drops at scope exit. The contrast is mainly relevant in academic comparisons. |
| **ownership** | Which name in the program is responsible for releasing a value's resources. vāṇी has at most one owner per value at any moment; the owner's scope-exit triggers Drop. |
| **move** | Transferring ownership from one binding to another. Equivalent to "consume": the source binding can no longer be used. |
| **borrow** | Temporary read-only access via a `ref T` reference. The borrowed value's owner stays unchanged; the borrow may not outlive the owner. |
| **mut ref / mutable borrow** | Temporary read/write access via `mut ref T`. Exclusive while held — no other reference (shared or mut) can coexist. |
| **reborrow** | Constructing a fresh reference from an existing one (e.g. passing `ref x` further inwards). Inherits the parent's lifetime. |
| **alias / aliasing** | Two names that refer to the same underlying value. vāṇी forbids mutable aliasing — one `mut ref` rules out every other reference for its scope. |
| **escape (a reference escapes a scope)** | A reference outliving the local it borrows. The scope-escape analyzer rejects programs where a `ref` is returned, stored in a heap location, or assigned to a global. |
| **elision** | Compiler-inferred ("elided") lifetime — when the user doesn't write `<'a>`, the rules pick a sensible default. vāṇी uses Rust-style elision so most code is annotation-free. |
| **RAII** | "Resource Acquisition Is Initialization" — the resource's lifetime is tied to the scope of its owning binding. vāṇी's Drop runs at scope exit; no need for `defer` or finalizers. |

### Type system

| Term | Meaning |
|---|---|
| **generic / parametric** | A definition that takes one or more type parameters (`fn id<T>(x: T) -> T`). vāṇी supports one type parameter per fn. |
| **monomorphization** | The pass that takes a generic definition (`fn id<T>`) and emits a specialized concrete copy per (template, type-args) tuple it sees called (`fn id__i64`, `fn id__bool`). The resulting program contains no generics at the IR level. |
| **monomorphic** | Already fully-specialized at concrete types — no remaining type parameters. |
| **mangling** | Rewriting a name to encode type-args / module path so the linker can keep distinct instantiations apart (`id__Box_I64_`). |
| **arity** | Number of arguments a function or variant takes. `fn add(a, b)` has arity 2. |
| **coercion** | Implicit type conversion the compiler inserts (e.g. `OwnedStr` → `Str` in read-only positions). vāṇी keeps these conservative — every cross-width / cross-sign integer conversion requires an explicit `as`. |
| **nominal type** | Two types with the same shape are *not* the same type unless they share a name. vāṇी's structs are nominal — `struct A { x: i64 }` and `struct B { x: i64 }` aren't interchangeable. |
| **opaque type** | A name whose definition is hidden from callers — they can hold or pass the value but not read its fields. `Handle<T>` is opaque. |
| **trait / interface** | A named collection of methods that types can implement. vāṇी uses `interface`; Rust users will recognize it as the equivalent of `trait`. |
| **dyn iface** | A trait object — a value whose concrete type is erased and dispatched dynamically via a **vtable**. Written `dyn Iface` in vāṇी. |
| **vtable** | A small table of function pointers (one per interface method) that powers dynamic dispatch on `dyn Iface`. |
| **fat pointer** | A pointer carrying its companion data inline — `Box<dyn Iface>` is `{ data_ptr, vtable_ptr }`; `BoundedPtr<T>` is `{ data, len, capacity }`. Contrast with a **thin pointer** (one word). |

### Pattern matching & enums

| Term | Meaning |
|---|---|
| **scrutinee** | The expression *being matched* — the value on the right of `match … { … }`. In `match k { K.A then 1, K.B then 2 }`, `k` is the scrutinee. |
| **variant** | One alternative of an enum. `enum Opt { Some(i64), None }` has two variants. |
| **payload** | The data a variant carries. `Some(i64)` has an i64 payload; `None` has no payload. |
| **discriminant / tag** | The runtime integer that distinguishes which variant a value holds. |
| **destructure** | Pulling fields / payloads out of a compound value into named bindings — `let (a, b) = pair;` or `K.Some(v) then …` in a match arm. |
| **exhaustive** | Every possible value of the scrutinee type matches at least one arm. vāṇी requires exhaustive matches and rejects gaps. |

### Compiler pipeline

| Term | Meaning |
|---|---|
| **AST** | Abstract Syntax Tree — the parser's structured representation of the source code. |
| **IR** | Intermediate Representation — a typed, post-checker form of the program. vāṇी emits a "tree IR" (close to the AST) and an "SSA IR" used by the optimizer / parallel-for emit. |
| **SSA** | Static Single Assignment — each variable is assigned exactly once; control flow merges become Phi nodes. Makes data-flow analyses straightforward. |
| **lowering** | Translating from a higher-level IR to a lower-level one (typed-IR → SSA → backend-specific code). |
| **emit / emission** | The final code-generation step that produces C or LLVM IR text. |
| **backend** | The half of the compiler that consumes the IR and produces output for a target. vāṇी has C and LLVM backends. |
| **lambda lift** | Hoisting an inline anonymous fn (`fn(x) -> y { … }`) to a top-level fn with a synthesized name so the backend can emit it as a regular symbol. |
| **(de)sugar** | "Sugar" is a convenient syntactic form; "desugar" is the parser- or pre-checker pass that rewrites it into the more verbose, semantically-equivalent core form (`?` becomes `try`, `+= 1` becomes `= … + 1`, etc.). |
| **hoist** | To move a sub-expression or statement *up* to an enclosing scope (e.g. binding a temporary `let` before using it inside a more restricted context). |
| **elaboration** | Adding hint / fix-it text to a diagnostic to help the user recover. |

### SMT verification

| Term | Meaning |
|---|---|
| **SMT** | Satisfiability Modulo Theories — a class of solvers (vāṇī uses Z3) that decide first-order formulas modulo integer/bitvector/float/etc. theories. Underpins `requires` / `ensures` / `prove` / `invariant`. |
| **BitVec** | A fixed-width bit-vector — how SMT models integer types. vāṇी encodes `i64` as `(_ BitVec 64)` so overflow is faithfully modeled (signed and unsigned semantics chosen per variable). |
| **discharge** | "Solve" a proof obligation — show the negation is unsatisfiable, i.e. the claim always holds under the in-scope assumptions. |
| **counterexample** | A concrete assignment of values to free variables under which a `prove` / `ensures` / `invariant` fails. Z3 emits one when the claim can't be discharged. |
| **invariant** (loop) | A claim that holds at every iteration: before entry, after every body, and at exit. The SMT pipeline checks each of those points. |
| **requires** / **ensures** | A function's pre-condition / post-condition. Callers must satisfy `requires` at every call site; the body must establish `ensures` on every return path. |
| **elide** | "Skip emitting" — the bounds-elision / overflow-elision passes turn off the runtime guard for an Index / arithmetic op once SMT discharges the safety obligation. |

### Async, concurrency, and effects

| Term | Meaning |
|---|---|
| **caller / callee** | The "caller" is the fn making the call; the "callee" is the fn being called. Used a lot in lifetime / ownership discussions. |
| **suspend point** | An `await(expr)` or `io_*_async(...)` call where the async runtime can pause the task and return control to the scheduler. State machines are split around these. |
| **poll** | The runtime entry point on an async task — called repeatedly until it returns `Ready(v)` or `Pending`. |
| **yield** | An async task voluntarily relinquishing control (`sleep_ms(0)` is the conventional shape). |
| **future** | An in-progress async computation. vāṇी's `Future<T>` is a state-machine enum with `Ready(T)` / `Pending` variants. |
| **pure fn** | A function with no observable side effects — no I/O, no heap allocation, no non-determinism, no impure callees. Verified by the effects checker. |
| **effects checker** | The pass that decides whether a function body is pure, by walking calls and rejecting any impure builtin or non-pure callee. Same logic gates `parallel for` bodies. |

### Memory & runtime primitives

| Term | Meaning |
|---|---|
| **arena** | A region allocator whose contents are all freed at once when the arena drops. vāṇी's `Region` is a bump-allocator arena available inside `unsafe(reason = "...")` on embedded targets. |
| **bump allocator** | Allocator that increments a single pointer and never reclaims individual cells — the arena reclaims everything at once. O(1) allocation. |
| **BoundedPtr<T>** | A fat pointer carrying `data + len + capacity` so `bptr_get(i)` can return `None` instead of UB on out-of-bounds. Available inside `unsafe(reason = "...")` for low-level interop. |
| **MMIO** | Memory-mapped I/O — reads and writes at hardware-defined addresses that map to device registers. `mmio_read_u32` / `mmio_write_u32` are the vāṇी builtins. |
| **canary** | A sentinel value placed adjacent to a buffer or stack frame so the runtime can detect overflow. Used in C as a stack-smashing defense; vāṇी doesn't need user-level canaries — bounds checks fire before overflow can occur. |
| **prelude** | A small set of declarations the compiler injects before every program: `Option<T>`, `Result<T,E>`, `Future<T>`, `Poll<T>`, `CancelToken`, `AllocError`. |
| **panic / abort** | A non-resumable termination. `assert` failures call `abort()`, which the OS reports as exit-on-signal SIGABRT. |
| **stack / heap** | The stack holds activation records (function-call frames); the heap holds long-lived allocations (`Vec<T>`, `OwnedStr`, `Box<T>`). vāṇी puts the choice in the type (Copy = stack-ish, owning = heap). |
| **deferred** | A diagnostic or work-item delayed until later in the pipeline. |
| **runtime** | Two senses: (a) the bundled C / LLVM helpers the compiler emits alongside user code (string concat, Vec helpers, futex-backed Mutex, etc.); (b) "at runtime" — when the compiled program executes. |
| **transitive** | Following a chain. "Transitive borrow" = a borrow of a borrow. "Transitive impurity" = if `f` calls `g` and `g` is impure, then `f` is impure. |

## License

Released under the [MIT License](LICENSE). VANI / वाणी is a free
non-commercial project; common phrases and the project name carry no
registered trademark — see *Trademark* below.

### Trademark

The project name **VANI** (वाणी, *vāṇī*) and the tagline *"code like you
speak"* are unregistered common-law marks of The VANI Authors. You may
use them to refer to the project ("compatible with VANI", "implementation
of VANI") and in good-faith forks. Please don't use them in a way that
implies endorsement by the project, or as your own product brand. If in
doubt, ask in an issue.

**Third-party marks referenced in this README.** Names such as *Rust*,
*C*, *C++*, *LLVM*, *Linux*, *Windows*, *macOS*, *Z3*, *Python*,
*Sanskrit*, *Hindi*, *Marathi*, and any others used in comparison or
discussion are the marks of their respective owners. References here
are descriptive (nominative fair use) — e.g., to describe inspiration,
compatibility, or platform support — and do not imply affiliation,
sponsorship, or endorsement by those owners. Playful coinages like
*"C-scenic"* and *"Rust-ic"* are English wordplay, not adoption of any
third-party mark.
