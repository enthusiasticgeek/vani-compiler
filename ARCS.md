# Multi-Session Arc Plan (Arcs 1–4)

> **Sequenced after the safety-standard alignment arc.** See
> [TODO.md](TODO.md) → *Safety-standard alignment*. Per the 2026-06-02
> direction, MISRA / ASIL-D / DO-178C / IEC 62304 attribute work
> (Tiers 1 → 2 → 3) lands before any ARC work begins. Reason: the
> safety-standard primitives (`#[no_heap]`, stack-depth check, etc.)
> may surface constraint violations in HashMap / Trie / closure
> code paths that need to be addressed as part of the arcs
> themselves — easier to design the arcs against the locked
> standard surface than to retrofit later.
>
> Order: **safety-standard Tier 1** → **safety-standard Tier 2** →
> **ARCs (Arc 2 → Arc 1 → Arc 4.1 → Arc 3a → rest)** →
> **safety-standard Tier 3**.

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

1. **1.1 — Generic-context `hashmap_new()`. (~1.5h)**
   - Change `check_hashmap_builtin` to read the let-binding's type
     annotation (the existing inference for `vec()` is the model)
     and return `Type::HashMap(K, V)` accordingly.
   - Fall back to `Type::HashMap(I64, I64)` when no annotation is
     present (backwards-compatible default).
   - **Test:** existing `HashMap<i64, i64>` examples keep working.

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

1. **2.1 — Sparse child representation. (~1.5h)**
   - Decide: struct-of-arrays (`u8* keys, u32* child_idx` per node)
     vs array-of-structs (`{u8, u32}* pairs`). SoA is more
     cache-friendly; AoS is simpler.
   - Update the `intent_trie` struct typedef in both backends.
   - Initialize new nodes with empty child lists (capacity 0).

2. **2.2 — Atomic rewrite of all 5 ops: insert/contains/delete/
   starts_with/walk. (~3h)** — must land as a single commit.
   - For each operation, replace the `children[byte]` direct lookup
     with a binary search over the sorted child-byte array.
   - For `insert`: find insertion position via lower_bound; shift
     trailing entries; insert (byte, new_child_idx).
   - For `delete`: similar shift-out.
   - Maintain the freelist for reusing freed node slots.

3. **2.3 — LLVM mirror of 2.2. (~2h)**
   - The LLVM trie has parallel code; rewrite the same five ops in IR.

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

### Sub-track 3b — Non-i64 element types (`Vec<Str>`, `Vec<f64>`, etc.) (~4–6h, 5 commits)

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

**Acceptance:** `vec_map(xs: ref Vec<Str>, |s| s)` round-trips
cross-backend.

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

1. **4.1 — `HashMap<Str, i64>`. (~3h)**
   - Most-common use case (counters, name → ID maps).
   - Str key hashing via existing `intent_hash_str`.
   - Borrowed-Str keys vs OwnedStr keys: decide on a convention.
     Probably OwnedStr (HashMap owns the key).

2. **4.2 — `HashMap<i64, Str>`. (~3h)**
   - V = OwnedStr; map drops free each value Str.

3. **4.3 — `HashMap<Str, Str>`. (~2h)**
   - Both axes OwnedStr.

4. **4.4 — `HashMap<Tuple<i64, i64>, V>`. (~3h)**
   - Tuple keys use `hash_pair` / `hash_combine` from Tier E.

5. **4.5 — `HashMap<f64, V>`. (~2h)**
   - Caveat: NaN keys are never equal to themselves; document but
     don't special-case.

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
