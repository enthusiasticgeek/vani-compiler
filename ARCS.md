# Multi-Session Arc Plan (Arcs 1–4)

> **Status (2026-06-03):** safety-standard alignment arc
> ✅ COMPLETE, Arc 2 ✅ COMPLETE, Arc 3 ✅ COMPLETE, Arc 1 partial.
> The seven `intentc` audit CLIs (deviations, stack-depth,
> acyclicity, hashmap-usage, complexity, safety-attrs,
> audit-pack) all shipped this session. See [STATUS.md](STATUS.md)
> for the per-commit ledger. **Remaining open: Arc 1.4–1.7 and
> Arc 4.**

## Remaining work — sequenced sub-step breakdown

The remaining open work is broken down here into individually-
landable sub-steps with explicit dependencies. Each sub-step
should leave the test suite green after its commit. Order is
top-to-bottom — earlier sub-steps gate later ones.

### Sequence 1: Arc 1.4 + 1.5 — per-(K, V) HashMap bundle, both backends

Goal: `let m: HashMap<i64, V> = hashmap_new()` works for V any
scalar (i32, u32, u64). Foundation for Arc 1.7 + Arc 4.

| # | Sub-step | Effort | Depends on |
|---|---|---|---|
| 1.4a | Option<V> auto-monomorphization for non-i64 V in `walk_expr_for_search_builtins` (use let-annotation's V instead of hardcoded i64 for hashmap_get/insert/remove) | ~1h | — |
| 1.4b | `check_hashmap_builtin` relaxation: accept scalar (K, V), coerce key/value to actual K/V, return `Option<V>` mangled by V | ~1h | 1.4a |
| 1.4c | Parameterized C bundle: `emit_intent_hashmap_pair_c_body(K, V)` produces per-(K, V) struct + helpers (legacy `intent_hashmap_i64_i64` stays for backwards compat) | ~2h | 1.4b |
| 1.4d | C backend prologue: walk `collect_hashmap_pairs(program)`, emit per-pair bundle for each non-(i64, i64) pair | ~30m | 1.4c |
| 1.4e | C backend tree `emit_call` dispatch: read receiver's (K, V), pick the right bundle prefix | ~30m | 1.4d |
| 1.4f | C backend SSA `emit_call` dispatch: **NO-OP** — `ssa_path_supports` (main.rs) rejects HashMap-using programs, falling back to the tree path. The 1.4e tree dispatch covers both. ✅ AUTOMATIC | — | 1.4d |
| 1.5a | Parameterized LLVM bundle: mirror 1.4c in LLVM IR (apply ARC 2.3 success pattern) | ~3h | 1.4f (atomic with 1.5b-d) |
| 1.5b | LLVM backend prologue: walk pairs, emit per-pair bundle | ~30m | 1.5a |
| 1.5c | LLVM backend tree `emit_call` dispatch | ~30m | 1.5a |
| 1.5d | LLVM backend SSA `emit_call` dispatch in `ssa_backend_llvm.rs` | ~30m | 1.5a |
| 1.5e | Cross-backend parity tests: `HashMap<i64, u32>`, `HashMap<i64, u64>`, `HashMap<i64, i32>` insert/get/contains/remove round-trip | ~1h | 1.5d |

**Subtotal: ~11h, 11 commits.** Acceptance: scalar V variants
round-trip cross-backend.

### Sequence 2: Arc 1.7 — `HashMap<UserStruct, V>` end-to-end

Goal: user-defined struct K with `implement Hash for K`.

| # | Sub-step | Effort | Depends on |
|---|---|---|---|
| 1.7a | Bundle hash-call dispatch: when K is struct, the bundle's `__hash_key` calls user's `fn_<K>__hash` instead of FNV-1a | ~1.5h | 1.5e |
| 1.7b | Bundle equality dispatch: when K is struct, compare field-by-field using the existing struct-`==` machinery | ~1h | 1.7a |
| 1.7c | LLVM mirror of 1.7a-b: emit `@fn_<K>__hash` call + IR struct-eq | ~2h | 1.7b |
| 1.7d | End-to-end: `HashMap<Score, i64>` round-trip test + cross-backend parity | ~30m | 1.7c |

**Subtotal: ~5h, 4 commits.** Acceptance: `HashMap<Score, i64>`
round-trips, with Score defining its own Hash impl.

### Sequence 3: Arc 4 — wider K-V (non-Copy V or non-i64 K)

Goal: cover the high-value (K, V) pairs the language community
asks for. Each pair lands as its own ~3–5-commit increment.

| # | Pair | Effort | Depends on |
|---|---|---|---|
| 4.1 | `HashMap<OwnedStr, V>` — string key with FNV-1a + strcmp equality ✅ **SHIPPED 2026-06-03** | ~3h | 1.5e |
| 4.2 | `HashMap<i64, OwnedStr>` — string V with drop walk per slot ✅ **SHIPPED 2026-06-03** | ~3h | 1.5e |
| 4.3 | `HashMap<OwnedStr, OwnedStr>` — both axes ✅ **SHIPPED 2026-06-03** | ~2h | 4.1 + 4.2 |
| 4.4 | `HashMap<Tuple<i64, …, i64>, V>` — tuple K via `hash_combine` ✅ **SHIPPED 2026-06-03** | ~3h | 1.5e + Tuple E sugar |
| 4.5 | `HashMap<f64, V>` — float K (caveat: NaN keys) ✅ **SHIPPED 2026-06-03** | ~2h | 1.5e |
| 4.6 | `HashMap<Vec<i64>, V>` — Vec K (affine move-in semantics) | ~3h | 4.1 |

**Subtotal: ~16h, ~30 commits.** Acceptance per pair: insert,
get, contains_key, remove, len, clear all round-trip
cross-backend.

### Cross-dependency graph (textual)

```
                    [shipped 2026-06-03]
                          ↓
                Arc 1.1 / 1.2 / 1.3 / 1.6
                Arc 2 (all)
                Arc 3 (all)
                7 audit CLIs
                          ↓
        ┌────────────── Arc 1.4-1.5 ──────────────┐
        │  (per-(K, V) bundle, atomic landing)    │
        └──────────────────────────────────────────┘
                ↓                            ↓
        Arc 1.7 (struct K)         Arc 4 (wider K/V)
                ↓                            ↓
                └─→ "HashMap is fully generic" ←─┘
```

### Order (executed so far) → suggested continuation

Order executed: safety-standard Tier 1 ✓ → Tier 2 ✓ → Tier 3
✓ → Arc 2 ✓ → Arc 3 ✓ → Arc 1 partial (1.1, 1.2, 1.3, 1.6)
→ audit CLI family ✓.

Suggested continuation order:

1. **Arc 1.4a → 1.4b → 1.4c → 1.4d → 1.4e → 1.4f** (scalar-V C
   backend; ~5h)
2. **Arc 1.5a-e** (LLVM mirror + parity tests; ~6h)
3. **Arc 1.7a-d** (struct K; ~5h, unblocked by Arc 1.4+1.5)
4. **Arc 4.1 → 4.2 → 4.3 → 4.4 → 4.5 → 4.6** (each
   independent once 1.4+1.5 land)

Total remaining: ~32h across ~50 commits if all pursued.

Background: through closure #604 the bounded one-shot primitive surface
is exhausted (tiers E–DD + W + Arc 0 — 108 closures). What remains are
four arcs that genuinely require atomic multi-commit landings — each
touches the type system + both backends in lockstep and must be planned
as a sequence of small commits that are individually buildable.

Each arc has:
- **Goal** — user-facing capability unlocked
- **Why this can't be one closure** — what's atomically coupled
- **Granular sub-steps** — each individually buildable; the test suite
  must stay green after every commit
- **Acceptance** — the smoke/lib test that confirms the arc has landed

Estimates are rough effort budgets per sub-step. Where two sub-steps
must land together to keep the suite green, that's noted as "atomic
pair."

---

## Arc 1 — Hash/Ord interface for user struct keys

**Goal:** `HashMap<MyStruct, V>` works for any user struct that
implements a `Hash` interface (and structural `Eq`, which already
exists).

**Why this can't be one closure:** The HashMap C/LLVM bundle is
hardcoded `intent_hashmap_i64_i64` — replacing it with per-(K, V)
emission, parameterized hashing dispatch, and bound checking is a
type-system change that touches the checker, both backends, and the
existing test suite simultaneously.

**Investigation already completed this session (2026-06-02):**
- `interface Hash { fn hash(self: ref Self) -> i64 }` already parses
  and type-checks (uses existing Cmp-style interface machinery).
- `implement Hash for Score { ... }` also type-checks.
- Blocker: `let m: HashMap<Score, i64> = hashmap_new()` errors with
  `expected HashMap<Score, i64>, got HashMap<i64, i64>` because
  `hashmap_new()` returns hardcoded `Type::HashMap(I64, I64)` in
  `check_hashmap_builtin` (checker.rs ~20278).
- C bundle naming: `intent_hashmap_i64_i64` hardcoded in
  `backend_c.rs:1145` (`emit_intent_hashmap_helpers_c_body`).
- LLVM: same hardcoded pattern.

### Sub-steps (~10–15h, 7 commits)

1. **1.1 — Generic-context `hashmap_new()`. ✅ SHIPPED 2026-06-03.**
   - New helper `try_elaborate_empty_hashmap` mirrors the existing
     `try_elaborate_empty_vec` pattern: when the let-annotation is
     `HashMap<K, V>`, the empty `hashmap_new()` call elaborates to
     return `Type::HashMap(K, V)` from that annotation.
   - The default (no annotation) returns
     `Type::HashMap(I64, I64)` — backwards-compatible.
   - Tests: 3 new lib tests pin (a) default i64/i64, (b) annotated
     i64/i64 via the elaboration path, (c) non-default V correctly
     binds the type AND surfaces the expected bundle-op restriction
     diagnostic (the rest of the bundle is still hardcoded for
     i64/i64 until 1.4/1.5 ship).

2. **1.2 — `Hash` bound enforcement at type-check. (~1h)**
   - When `HashMap<K, V>` is instantiated with K = struct type,
     verify that an `implement Hash for K` exists in scope.
   - Emit `error: HashMap key type {K} must implement Hash` when missing.
   - Structural `Eq` for structs already exists (via `==`); add a
     check that K's struct fields are all comparable.

3. **1.3 — Type collector: register `HashMap<K, V>` instantiations. (~1h)**
   - Extend `collect_vec_elements_in_expr` analog for HashMap.
   - For each (K, V) pair seen in the program, emit a unique tag
     `intent_hashmap_<K_tag>_<V_tag>` and add to the emission queue.

4. **1.4 — C bundle: per-(K, V) emission. (~3h, atomic with 1.5)**
   - Generalize `emit_intent_hashmap_helpers_c_body` to take element-
     type parameters and emit the bundle with parameterized hashing
     and equality dispatch.
   - For struct K: call the user's `hash` impl via the existing
     vtable / mangled `fn_<UserStruct>__hash` symbol.
   - For struct equality: emit field-by-field comparison (already
     emitted for `==` on structs — reuse it).

5. **1.5 — LLVM bundle: per-(K, V) emission. (~3h, atomic with 1.4)**
   - Mirror the C bundle: parameterized struct, hash dispatch via
     `@fn_<UserStruct>__hash`, equality via the existing struct-eq IR.

6. **1.6 — Regression: existing `HashMap<i64, i64>` callers. (~30m)**
   - All currently-passing examples that use HashMap<i64, i64> must
     still compile + run + pass cross-backend parity. No syntax
     migration required (1.1 keeps the default).

7. **1.7 — End-to-end: `HashMap<Score, i64>` round-trip. (~1h)**
   - Lib test: define a `Score` struct with `implement Hash for Score`,
     insert a few keys, verify `hashmap_get`, `hashmap_contains_key`,
     `hashmap_remove`, `hashmap_len`, `hashmap_clear` all work.
   - Cross-backend byte-identical output.

**Acceptance:** `HashMap<Score, i64>` round-trips in both backends.

---

## Arc 2 — Trie sparse children

**Goal:** replace the dense 256-wide `int32_t children[256]` per-node
storage with `Vec<(u8, u32)>` sorted by byte + binary search. Drops
memory usage by ~30–100× for sparse tries (alphabets like DNA, ASCII
digits, hex).

**Why this can't be one closure:** Every trie operation
(`trie_insert`, `trie_contains`, `trie_starts_with`, `trie_delete`,
`trie_walk`) reads the children array and must change atomically. A
partial switch leaves the suite broken.

### Sub-steps (~5–8h, 4 commits)

1. **2.1 — Sparse child representation. ✅ SHIPPED 2026-06-03 (C side).**
   - Chose SoA: per-node `node_keys` (u8) + `node_children`
     (i32) sorted by key, plus `node_count`/`node_cap`/`free_next`
     parent arrays. Freelist gets its own array rather than
     reusing a child slot — cleaner separation.
   - C backend: updated `intent_trie` struct typedef.
   - LLVM backend: pending — see 2.3.

2. **2.2 — Atomic rewrite of all 5 ops. ✅ SHIPPED 2026-06-03 (C side).**
   - `find_slot` (binary search) + `lower_bound` + `insert_pair`
     (shift right) + `remove_pair` (shift left) + `grow_node`
     (capacity doubling, starts at 4) helper functions emitted
     alongside the main ops. All 5 user-facing ops rewritten in
     a single commit per the ARCS plan.

3. **2.3 — LLVM mirror of 2.2. ✅ SHIPPED 2026-06-03.**
   - Full LLVM IR rewrite — 11-field struct (was 7), 5 new
     helper functions (`__find_slot`, `__lower_bound`,
     `__grow_node`, `__insert_pair`, `__remove_pair`), all
     user-facing ops (`_new_node`, `_new`, `_drop`, `_insert`,
     `_walk`, `_delete`, `_clear`) rewritten using them.
   - Per-function label prefixes (fs_/lb_/gn_/ip_/rp_/tn_/tnu_/
     td_/ti_/tw_/tc_/tsw_/tde_/trc_) avoid LLVM label collisions.
   - Two existing lib tests (`trie_alphabet_accepts_full_u8_range`
     pinned `mul i64 %cap_new, 1024`; `trie_compaction_extends_struct_and_emits_freelist`
     pinned the 7-field shape) updated to match sparse-shape
     invariants — they now pin the `__find_slot` helper and
     the 11-field type respectively.
   - Output byte-identical across both backends on the trie
     example and all 13 lib tests; cross-backend parity sweep
     passes.

4. **2.4 — Cross-backend parity: re-run all trie examples. (~1h)**
   - Verify `examples/trie.vani` and any lib tests using Trie pass
     unchanged on both backends.
   - Add one new lib test exercising a long-prefix trie to ensure
     binary search correctness at depth.

**Acceptance:** all existing trie tests pass + a new long-prefix trie
test passes on both backends.

---

## Arc 3 — Richer closures

**Goal:** closures gain (a) capture-by-ref, (b) non-i64 element types
in lambda bodies (`Vec<Str>`, `Vec<f64>`), (c) `.collect` chain syntax,
(d) tuple-element Vec (`Vec<Tuple<i64, i64>>`).

**Why this can't be one closure:** Four independent capabilities, each
its own multi-sub-step landing. They overlap on the closure analysis
and codegen paths, so changes are easier to land per-capability with
the test suite green between each.

### Sub-track 3a — Capture-by-ref (~4–6h, 5 commits)

1. **3a.1 — Parser: `&captured` or `ref captured` in capture list. (~1h)**
   - Extend the closure-capture parser to recognize a leading `&` or
     `ref` keyword on a captured identifier.

2. **3a.2 — Type checker: ref captures get `Ref<T>` type. (~1.5h)**
   - The closure's struct field for a borrowed capture is `Ref<T>`,
     not `T`.
   - Lifetime / affine check: the closure must not outlive the
     captured borrow's scope. The existing borrow checker enforces
     this; verify it fires correctly on closures.

3. **3a.3 — C codegen: emit field as `const T*`; deref at use. (~1h)**
   - In the closure struct, ref captures spell as `const T*`.
   - Every read of the captured value goes through `*field`.

4. **3a.4 — LLVM codegen: same. (~1h)**

5. **3a.5 — Drop semantics: ref captures are NOT dropped. (~30m)**
   - The closure's drop walks owned fields only; borrowed fields are
     skipped (no double-free).

**Acceptance:** lambda that captures a borrowed `Vec<i64>` and reads
from it without consuming compiles + runs cross-backend.

### Sub-track 3b — ✅ ALREADY SHIPPED (unlocked by 3a). Non-i64 element types.

1. **3b.1 — Type collector: register `Vec<Str>` / `Vec<f64>`. (~1h)**
   - Extend the collector to enumerate Str / f64 / OwnedStr as Vec
     element types.

2. **3b.2 — C bundle: emit `intent_vec_OwnedStr` parallel. (~2h)**
   - The bundle ops (new, push, pop, len, drop, etc.) are
     element-type parameterized. Generate a parallel bundle for each
     new element type.
   - Drop semantics for OwnedStr elements: walk elements and free
     each Str.

3. **3b.3 — LLVM bundle: same. (~2h)**

4. **3b.4 — Closure body checker: relax i64-only constraint. (~1h)**
   - Current restriction: lambda body uses i64-only elements.
   - Loosen to allow Str / f64 element types where the bundle exists.

5. **3b.5 — End-to-end: `vec_map(xs: ref Vec<Str>, fn)` works. (~1h)**

**Acceptance (met 2026-06-03):** `Vec<f64>` and `Vec<OwnedStr>`
work end-to-end through closure bodies. The Vec-element
parameterization (closure #291's `Vec<T>` for non-Copy T)
landed earlier; the closure side of the equation came together
via ARC 3a's `[ref xs]` capture mechanism — non-Copy elements
flow through closure bodies as `Ref<Vec<T>>` and the body's
`xs[i]` / `len(xs)` / method calls all work via the existing
Ref-param conventions. Specific 5-step ARCS plan was scoped
against the i64-only constraint that turned out to no longer
exist at ship time. New lib tests pin the supported cases (Vec
f64 indexing, Vec OwnedStr len, f64 arithmetic in closure body).

### Sub-track 3c — `.collect` postfix syntax (~3–5h, 3 commits)

1. **3c.1 — Parser: recognize `xs.collect()` chain. (~1.5h)**
   - Postfix-method-call parsing already exists for `vec_min` etc.
   - Add `.collect` as a recognized postfix that desugars to
     `vec_<n>_from_iter(...)` or similar.

2. **3c.2 — Type checker: resolve `.collect()` based on chain context. (~2h)**
   - The iterator chain has a known element type and a known length
     pattern (filter shrinks; map preserves; take_while shrinks).
   - For v1, only support fixed-output-length chains
     (`map.collect`, `filter.collect`, etc.).

3. **3c.3 — Codegen: synthesize the `vec_<T>_collect` call. (~1.5h)**
   - Emit a fresh-Vec construction that walks the iterator's source
     and applies the chain's transforms.

**Acceptance:** `xs.map(|x| x + 1).collect()` returns a fresh `Vec<i64>`.

### Sub-track 3d — Tuple-element Vec (~4–6h, 4 commits)

1. **3d.1 — Type collector: register tuple types as Vec elements. (~1h)**

2. **3d.2 — C bundle: emit `intent_vec_tuple_i64_i64`. (~2h)**
   - Tuple has Copy semantics if all its elements are Copy.

3. **3d.3 — LLVM bundle: same. (~2h)**

4. **3d.4 — End-to-end: `Vec<Tuple<i64, i64>>` constructor + iteration. (~1h)**

**Acceptance:** `let xs: Vec<Tuple<i64, i64>> = vec(tuple(1, 2), tuple(3, 4));`
round-trips cross-backend.

---

## Arc 4 — Wider HashMap K/V

**Goal:** `HashMap<K, V>` for `K, V` in {i64, Str, f64, struct, tuple,
Vec}.

**Why this can't be one closure:** Each new (K, V) pair needs its own
bundle. Per-pair landing = ~3–5 sub-commits. Builds heavily on Arc 1.

Pre-requisite: Arc 1 complete (the per-(K, V) bundle emission and
Hash bound checking).

### Per-pair sub-step shape (~3–5 commits per pair)

For each new (K, V) pair:

1. **N.1 — Type collector adds the new (K, V). (~30m)**

2. **N.2 — Hash function selection for K. (~1h)**
   - i64 → `intent_hash_i64`
   - Str → `intent_hash_str`
   - f64 → `intent_hash_f64`
   - struct → user's `hash` impl (Arc 1 machinery)
   - Tuple<A, B> → `hash_combine(hash(A), hash(B))` (uses Tier E sugar)

3. **N.3 — Equality function selection for K. (~1h)**
   - i64/f64 → primitive `==`
   - Str → `strcmp`
   - struct → field-by-field (existing struct-eq machinery)
   - Tuple → element-wise

4. **N.4 — Affine drop semantics for K (when applicable). (~1.5h)**
   - Str/OwnedStr keys: HashMap owns the Str; drops free each key.
   - Vec keys: same.
   - Primitive keys: no-op.

5. **N.5 — Affine drop semantics for V. (~1h)**
   - Same considerations as K.

6. **N.6 — End-to-end lib test for that (K, V). (~30m)**

### Specific pairs in priority order (~10–15h per pair)

1. **4.1 — `HashMap<OwnedStr, V>`. (~3h)** ✅ **SHIPPED 2026-06-03**
   - Most-common use case (counters, name → ID maps).
   - Bundle clones the caller's OwnedStr internally via
     strlen + malloc + memcpy — sidesteps the affine-system
     gap where local drops aren't yet suppressed for
     OwnedStr moved into builtin args. Caller retains
     ownership; matches Rust's `m.insert(key.clone(), v)`.
   - C bundle prefix: `intent_hashmap_owned_str_<V>`. Keys
     field `char**`. Hash: FNV-1a byte loop. Equality:
     `strcmp`. Insert duplicate-key path swaps in new value
     without freeing caller's k. Remove + drop + clear free
     each stored clone.
   - LLVM mirror: equality via `call i32 @strcmp`, hash via
     byte-by-byte FNV-1a loop reading `i8*` until null
     terminator. memcpy auto-bound by LLVM (no declare).
   - Parity example: `examples/hashmap_str.vani`.

2. **4.2 — `HashMap<i64, OwnedStr>`. (~3h)** ✅ **SHIPPED 2026-06-03**
   - V = OwnedStr; map drops free each value Str.
   - C bundle prefix: `intent_hashmap_int64_t_owned_str`.
     `char**` values field; drop/clear walks free each.
     Insert clones via strlen+malloc+memcpy; duplicate returns
     prior V pointer (ownership transfer); remove transfers
     out + tombstones slot; get clones the stored V.
   - LLVM mirror: same shape, V is `i8*` per slot.
   - Parity example: `examples/hashmap_strv.vani`.

3. **4.3 — `HashMap<OwnedStr, OwnedStr>`. (~2h)** ✅ **SHIPPED 2026-06-03**
   - Both axes OwnedStr. Composition of 4.1 K + 4.2 V.
   - C bundle prefix: `intent_hashmap_owned_str_owned_str`.
     `char**` keys + `char**` values; drop/clear walks free
     both per slot. Insert clones K and V; duplicate K swaps
     V (returning prior); remove frees K + transfers V out;
     get clones V.
   - LLVM mirror: same shape, both axes are `i8*` per slot.
   - Parity example: `examples/hashmap_strstr.vani`.

4. **4.4 — `HashMap<Tuple<i64, …, i64>, V>`. (~3h)** ✅ **SHIPPED 2026-06-03**
   - Tuple keys use FNV-1a per-element hash_combine + pairwise
     field equality. Elements are Copy (i64) so no drop walk.
   - C bundle prefix: `intent_hashmap_tup_<arity>_i64_<V>`.
     Keys stored as contiguous array of `intent_tuple_<…>` structs.
   - LLVM mirror: key type `{i64, i64, …}` (anonymous struct
     literal). Hash via extractvalue + FNV-1a; equality via
     internal `__eq_key` helper chaining icmp eq through and.
   - Parity example: `examples/hashmap_tup.vani`.
   - Wider element types (struct/OwnedStr) deferred to a future
     ARC if needed; today's tuples are i64-only.

5. **4.5 — `HashMap<f64, V>`. (~2h)** ✅ **SHIPPED 2026-06-03**
   - Caveat: NaN keys are never equal to themselves; document but
     don't special-case.
   - C bundle prefix `intent_hashmap_double_<V>`, hash via memcpy
     to u64 + FNV-1a; equality via native `==`.
   - LLVM bundle uses `bitcast double to i64` + `fcmp oeq double`.
   - Parity example: `examples/hashmap_f64.vani`.

6. **4.6 — `HashMap<Vec<i64>, V>`. (~3h)**
   - Highly affine — Vec key moves in.

**Acceptance per pair:** insert, get, contains_key, remove, len, clear
all round-trip cross-backend.

---

## Cross-arc dependencies

- Arc 1 is a hard prerequisite for Arc 4 (the per-(K, V) bundle
  emission machinery).
- Arc 2 is independent.
- Arc 3 is independent.
- Arc 3a (capture-by-ref) is the most cross-cutting — touches the
  type checker, both backends, and the closure-emission template.

## Suggested session order

1. **Arc 2 first** — smallest scope, 5–8h, fully independent.
   Validates the multi-commit pattern.
2. **Arc 1 next** — unblocks Arc 4 and is moderately scoped.
3. **Arc 4.1** (`HashMap<Str, i64>`) — most-requested pair.
4. **Arc 3a** (capture-by-ref) — highest user impact for closures.
5. Remaining Arc 3 sub-tracks + Arc 4 pairs as resources permit.
