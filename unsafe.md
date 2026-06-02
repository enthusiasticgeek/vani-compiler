# Embedded-vāṇī Unsafe + Memory-Safety Plan

Goal: vāṇी usable in embedded systems where:
- `unsafe { ... }` blocks are necessary (raw pointer arithmetic,
  MMIO, DMA, hand-rolled allocators)
- No garbage collector
- Maximum catch of memory errors at compile time
- Cheap runtime checks where compile-time can't reach

Pick chosen at design time: **generational handles** over region
typing. Generational handles add a small runtime cost (~1 load + 1
cmp per dereference) but are syntactically invisible; region typing
is purely compile-time but requires lifetime annotations on every
pointer-holding type, which raises the bar for embedded engineers.

Existing baseline (do not redo): affine ownership for Vec / OwnedStr
/ Atomic / Mutex / Guard / Channel / Task. Lightweight borrow checker
(not Rust-style). Both reused unchanged.

---

## Layer 1 — Compile-time, always on (zero runtime cost)

### 1.1 Lexical `unsafe { ... }` containment

**What:** Raw pointer types (`*T`, `*mut T`) may only appear inside
the body of a function marked `unsafe`, or inside an `unsafe { ... }`
block. They cannot appear in a function's parameter or return type
unless that function is itself `unsafe`. Same for struct/enum field
declarations.

**Why:** Forces all raw-pointer manipulation to be syntactically
located in code the reader knows to scrutinize. Mirrors Rust's
unsafe-block discipline but stricter on type signatures.

**Files touched:**
- `src/checker.rs` — add `is_unsafe_context: bool` to the checker
  state; reject raw pointer types when false.
- `src/ast.rs` — add `UnsafeBlock { body: Vec<Stmt> }` variant to
  `Stmt`; parser already may have similar precedent.

**Estimate:** 2–3h. Single commit.

**Acceptance:**
```vani
fn safe_fn(p: *i64) -> i64 { ... }    // ERROR: raw pointer in safe sig
unsafe fn ok_fn(p: *i64) -> i64 { ... } // OK
fn another() {
  let x: i64 = 0;
  unsafe { let p: *const i64 = &x as *const i64; }  // OK
}
```

### 1.2 No-escape analysis on `&local`

**What:** A raw pointer derived from a stack-local variable cannot:
- be returned from the function
- be stored into a heap-allocated location (Vec slot, struct field on
  the heap, etc.)
- be assigned to a global or `static mut`

This is a dataflow analysis over raw-pointer-typed values. Inside an
`unsafe` block: same rules apply (no escape) unless explicitly using
a heap-allocation primitive.

**Why:** Eliminates the classic "returns pointer to dead stack frame"
class of bug entirely at compile time.

**Files touched:**
- `src/checker.rs` — add `escape_analysis` pass post-typecheck for
  any function containing raw-pointer derivations.

**Estimate:** 3–4h. Single commit.

**Acceptance:**
```vani
unsafe fn bad() -> *const i64 {
  let x: i64 = 42;
  return &x as *const i64;  // ERROR: pointer to dead stack
}
unsafe fn ok(global: *const i64) -> *const i64 {
  return global;            // OK: param pointer threaded through
}
```

### 1.3 Taint bit on unsafe-derived values

**What:** Any value loaded through a raw pointer, or any
non-primitive value constructed inside an `unsafe` block, carries a
compile-time "tainted" flag in its type. Storing a tainted value into
a non-tainted location requires an explicit `assert_safe(x)` wrapper
that runtime-checks the invariant the user is promising.

**Why:** Forces users to be explicit about "this came from unsafe and
I claim it's now valid." Without this, unsafe code can silently
poison the rest of the program.

**Files touched:**
- `src/ast.rs` / `src/checker.rs` — extend `Type` with a `Tainted`
  wrapper, propagate through assignments and binops, strip via
  `assert_safe`.
- The wrapper emits a compile error when the user tries to *use*
  the tainted value in a safe-typed slot without going through
  `assert_safe`.

**Estimate:** 4–5h. May need 2 commits (parser + checker, then
propagation pass).

**Acceptance:**
```vani
unsafe fn read_byte(p: *const u8) -> Tainted<u8> { return *p; }
fn safe_caller(p: *const u8) {  // ERROR: *const in safe sig
}
unsafe fn safe_caller2(p: *const u8) -> i64 {
  let b: Tainted<u8> = read_byte(p);
  let v: i64 = assert_safe(b) as i64;  // user vouches; release-mode
                                        // no-op, debug-mode panics if
                                        // tagged-fat-pointer cannot
                                        // verify
  return v;
}
```

---

## Layer 2 — Runtime, generational handles (chosen)

### 2.1 `Handle<T>` type — generational, opaque

**What:** Add a builtin type `Handle<T>` representing a non-Copy,
non-nullable opaque reference to a `T`. Internally:
```
struct Handle<T> {
  slot_idx: u32,
  generation: u32,
}
```
The corresponding storage is a slot pool:
```
struct Pool<T> {
  slots: Vec<T>,
  generations: Vec<u32>,
  free_list: Vec<u32>,
}
```
Operations:
- `pool.alloc(value) -> Handle<T>` — returns a fresh handle
- `pool.get(handle) -> Option<&T>` — returns Some only if the slot's
  generation matches the handle's; None on use-after-free
- `pool.free(handle)` — bumps the slot's generation, pushes to
  free_list

**Why generational:** Free is O(1). Use-after-free is detected at
runtime by the generation mismatch — no UB. The handle is two `u32`s
(8 bytes total, same as a pointer on 64-bit systems for most purposes).

**Files touched:**
- `src/ast.rs` — add `Type::Handle(Box<Type>)`.
- `src/checker.rs` — add `pool_new` / `pool_alloc` / `pool_get` /
  `pool_free` builtins.
- `src/backend_c.rs` — emit `intent_pool_<T>` struct + ops, parameterized
  on element type via the existing collector pattern.
- `src/backend_llvm.rs` — same.

**Estimate:** 5–7h, ~3 commits:
- 2.1a Type + checker (no codegen)
- 2.1b C bundle emission for `Pool<i64>` (and the type collector to
  pick up other element types)
- 2.1c LLVM bundle emission

**Acceptance:** Round-trip test that alloc, get, free, then re-get
returns None (i.e., generation bump caught use-after-free).

### 2.2 Make `Handle<T>` the only blessed escape from `unsafe`

**What:** When user code wants to "smuggle" a long-lived reference
out of an `unsafe` block, they must wrap it in a `Handle<T>` via the
pool API. Raw `*T` cannot cross the unsafe boundary (Layer 1.1
already enforces this).

**Why:** Channels all dynamic-lifetime data through the
generation-checked path. Static-lifetime / stack-bound data is
handled by Layer 1; everything else flows through Handle.

**Files touched:** none new — falls out of 1.1 + 2.1.

**Estimate:** 1h documentation + acceptance test.

---

## Layer 3 — Always on in debug, strippable in release

### 3.1 Canary words around `unsafe`-allocated regions

**What:** Any allocation made inside `unsafe` via the language's
malloc primitive (e.g., `unsafe_alloc<T>(n)`) is bracketed:
```
[CANARY_PRE: 0xDEADBEEFCAFEBABE]
[user data, n × sizeof(T) bytes]
[CANARY_POST: 0xBAADF00DDEADC0DE]
```
`unsafe_free` verifies both canaries before reclaiming the region.
On mismatch, panics with "buffer overrun detected at address X."

**Why:** Cheap to add (~16 bytes per allocation, ~2 cycles at free
time). Catches common buffer overruns at the moment of free.

**Files touched:**
- `src/backend_c.rs` — augment the `unsafe_alloc` / `unsafe_free`
  helper emission to include canaries when `INTENT_DEBUG` is defined;
  no-op canaries (just malloc/free) when `INTENT_RELEASE`.
- `src/backend_llvm.rs` — same.

**Estimate:** 2h. Single commit.

### 3.2 Bounds-checked fat pointers for `Vec` / `Array` indexing (already shipped)

**What:** Reuse the existing bounds-check that the safe `Vec`/`Array`
indexing path emits. When raw pointer arithmetic inside `unsafe`
derives a pointer from a `Vec`'s data, an *optional* fat-pointer
wrapper `BoundedPtr<T>` carries the bounds along.

`BoundedPtr<T>` is:
```
struct BoundedPtr<T> {
  data: *T,
  len: usize,
  capacity: usize,
}
```
Dereferencing via `BoundedPtr.get(i)` is bounds-checked; via the raw
`.data` field is not.

**Why:** Lets users opt into bounds checks inside `unsafe` without
forcing them. Strip-able in release via a `--release` build flag.

**Files touched:**
- `src/checker.rs` — recognize the `BoundedPtr<T>` builtin type.
- `src/backend_c.rs`, `src/backend_llvm.rs` — emit the bundle.

**Estimate:** 3h. Single commit.

---

## Layer 4 — Hardware-assisted (free when available)

### 4.1 Stack canaries via `-fstack-protector` (toolchain flag)

**What:** Pass `-fstack-protector-strong` to the C compiler and the
analogous `--frame-pointer=all` + LLVM stack-protector pass to the
LLVM backend.

**Why:** Catches stack-smashing in any function that has buffers,
allocas, or string operations. ~2 instructions per frame, free
otherwise.

**Files touched:**
- `src/main.rs` — add the flag to the `build` and `run` subcommands'
  invocation of `cc` and `clang`.

**Estimate:** 30min. Single commit.

### 4.2 ARM MTE (Memory Tagging Extension) — optional release-build flag

**What:** When target is ARMv8.5+, emit the `-march=armv8.5-a+memtag`
flag. Hardware then tags every pointer with a 4-bit value and every
memory region with a 4-bit tag; mismatch traps. Free in HW.

**Why:** When deploying to Cortex-A or Apple Silicon, MTE catches
use-after-free and most buffer-overrun bugs at zero runtime cost.

**Files touched:** `src/main.rs` — add `--target-mte` flag.

**Estimate:** 1h (testing on actual hardware is out of scope).

---

## Skipped intentionally

- **Region typing.** Picked generational handles instead. Two
  techniques solving the same problem (use-after-free); doubling up
  doubles the implementation cost without doubling the catch rate.
- **Full Rust-style borrow checker.** You said you have a simpler
  variant — keep it; don't rebuild.
- **Garbage collector.** Out of scope by user requirement.
- **Capability tokens.** Overlaps too much with affine ownership;
  the marginal user value isn't worth the syntactic overhead.
- **ASAN/MSan runtime.** Too heavy for embedded; the canary +
  generational handle combination covers the common cases at a
  fraction of the cost.

---

## Suggested implementation order

1. **Layer 1.1** — Lexical `unsafe` containment (2–3h)
2. **Layer 1.2** — No-escape on `&local` (3–4h)
3. **Layer 4.1** — Stack canaries via toolchain flag (30min, free win)
4. **Layer 2.1a–2.1c** — `Handle<T>` type + Pool<T> codegen (5–7h)
5. **Layer 3.1** — Canary words around unsafe alloc (2h)
6. **Layer 1.3** — Taint bit on unsafe-derived values (4–5h)
7. **Layer 2.2** — Handle as the only `unsafe` escape (1h)
8. **Layer 3.2** — `BoundedPtr<T>` fat pointer (3h)
9. **Layer 4.2** — ARM MTE flag (1h)

**Total estimated effort:** 22–31 hours across ~12 commits.

---

## Cross-cutting decisions

- **Tainted<T>** and **Handle<T>** are the only two new types that
  cross safe/unsafe. Document this explicitly in the language guide.
- **`unsafe`** is the only escape hatch. No "trusted libs," no
  privileged crates.
- **Release mode** strips: canaries (Layer 3.1), taint runtime
  checks (Layer 1.3's `assert_safe`), bounds checks on `BoundedPtr`
  (Layer 3.2). Generational handles (Layer 2) stay on — they're
  the load-bearing safety net.

---

## Cross-arc coordination

This work is **independent of Arcs 1–4** (Hash/Ord, Trie sparse,
richer closures, wider HashMap K/V). Can interleave; suggested
ordering:

1. Land Layer 1 (compile-time) first — small commits, no codegen.
2. Land Layer 2 (generational handles) — establishes the runtime
   safety net before any unsafe-heavy embedded examples ship.
3. Layers 3–4 can land any time after.
4. Arc 2 (trie sparse children) can land in parallel — independent.

Arc 1 (HashMap monomorphization) is the long pole; this safety work
shouldn't gate it.
