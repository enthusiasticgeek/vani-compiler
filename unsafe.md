# Embedded-vāṇī Unsafe + Memory-Safety Plan

Goal: vāṇी usable in embedded systems where:
- `unsafe(reason = "...") { ... }` blocks are necessary (raw pointer arithmetic,
  MMIO, DMA, hand-rolled allocators)
- No garbage collector
- Maximum catch of memory errors at compile time
- Cheap runtime checks where compile-time can't reach

Picked at design time: **hybrid path** — generational handles ship
first (v1) for the mainstream embedded user; region typing lands
later (v2) as a power-user option for safety-critical workloads.

**Why hybrid:** generational handles are syntactically invisible
(low learning curve) but cost ~3–5 cycles per dereference on
Cortex-M and don't give compile-time use-after-free proof. Region
typing gives zero-cost-abstraction + compile-time guarantees but
requires the user to write lifetime annotations. Most embedded
engineers want the first; safety-critical (ASIL-D, DO-178C,
IEC 62304) requires the second. Both compile down to similar code
shapes once the user picks a stance per type.

**Decision sequencing:** Layers 1–4 ship generational handles first
(weeks). Layer 5 (regions) is queued for after — once Handle<T> is
proven in the wild and we have user feedback on what region syntax
should feel like.

Existing baseline (do not redo): affine ownership for Vec / OwnedStr
/ Atomic / Mutex / Guard / Channel / Task. Lightweight borrow checker
(not Rust-style). Both reused unchanged.

---

## Layer 1 — Compile-time, always on (zero runtime cost)

### 1.1 Lexical `unsafe(reason = "...") { ... }` containment

**What:** Raw pointer types (`*T`, `*mut T`) may only appear inside
the body of a function marked `unsafe(reason = "...")`, or inside an
`unsafe(reason = "...") { ... }` block. They cannot appear in a
function's parameter or return type unless that function is itself
unsafe. Same for struct/enum field declarations.

**The `reason = "..."` clause is mandatory at parse time.** Empty
strings are rejected. The reason is part of the syntax — not a
convention, not a comment. It's stored on the AST node, threaded
through the IR, and emitted as machine-readable debug metadata so
certification tooling can extract deviation records from the
compiled artifact (target: ASIL-D / DO-178C / IEC 62304 deviation
audit trails).

**Why mandatory in-syntax reason vs. a `// SAFETY:` comment:**
- Comments are stripped by the lexer and lost downstream — no IR
  metadata, no machine-readable extraction.
- Reasons-in-syntax cannot be silently deleted; they're part of the
  AST. A reviewer who removes the reason gets a parse error.
- The reason can be uniformized: certification toolchains emit
  every deviation as a row in a structured report keyed by
  (file, line, reason). Comments require ad-hoc parsing.
- The user has stated preference for this form (2026-06-02).

**Why:** Forces all raw-pointer manipulation to be syntactically
located in code the reader knows to scrutinize, AND forces a
machine-readable justification per occurrence. Strictly safer than
Rust's `unsafe { ... } // SAFETY: ...` convention because the
justification is parser-enforced.

**Reason-string rules (v1):**
- Non-empty: `unsafe(reason = "")` is a parse error.
- Bounded length: max 256 chars to keep IR metadata compact.
- ASCII-printable; no control chars, no embedded newlines.
- Recommended prefix conventions (not enforced) for certification
  tooling: `"MMIO: ..."`, `"FFI: ..."`, `"DMA: ..."`,
  `"transmute: ..."`, `"vendor-SDK: ..."`. Tooling can group by
  prefix to surface deviation categories.

**Files touched:**
- `src/ast.rs` — add `UnsafeBlock { reason: String, body: Vec<Stmt> }`
  variant to `Stmt`. Add `unsafe_reason: Option<String>` to `Function`
  for `unsafe(reason = "...") fn` declarations.
- `src/lexer.rs` / parser — recognize `unsafe(reason = "...")`
  syntax; reject `unsafe` without the reason clause; reject empty
  reason string.
- `src/checker.rs` — add `is_unsafe_context: bool` to checker state;
  reject raw pointer types when false.
- `src/backend_c.rs` — emit each block's reason as a comment marker
  + optionally a `.unsafe_deviations` section entry.
- `src/backend_llvm.rs` — emit reason as DWARF `DW_AT_description`
  or `!dbg` metadata on the IR for the block's first instruction;
  enables `objdump` / `llvm-dwarfdump` extraction.

**Estimate:** 3–4h (was 2–3h; +1h for reason plumbing through IR
metadata). Single commit.

**Acceptance:**
```vani
fn safe_fn(p: *i64) -> i64 { ... }    // ERROR: raw pointer in safe sig
unsafe(reason = "vendor-SDK: STM32 HAL callback signature")
fn ok_fn(p: *i64) -> i64 { ... }       // OK

fn another() {
  let x: i64 = 0;
  // ERROR — `unsafe` without reason is a parse error:
  unsafe { let p: *const i64 = &x as *const i64; }

  // ERROR — empty reason:
  unsafe(reason = "") { let p: *const i64 = &x as *const i64; }

  // OK:
  unsafe(reason = "MMIO: ad-hoc base address for GPIOA::ODR") {
    let p: *const i64 = &x as *const i64;
  }
}
```

**Deviation-record extraction (target):**
```
$ intentc build --report-deviations target/firmware.elf
file                  line  prefix      reason
src/hal_stm32f4.vani  142   vendor-SDK  STM32 HAL callback signature
src/dma.vani          88    DMA         scatter-gather descriptor walk
src/dma.vani          203   MMIO        DMA controller base register
... 3 deviations total, 2 prefix categories
```
This is the artifact a safety-critical reviewer wants for sign-off.

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
unsafe(reason = "vendor-SDK: stack-pointer-returning prototype")
fn bad() -> *const i64 {
  let x: i64 = 42;
  return &x as *const i64;  // ERROR: pointer to dead stack
}
unsafe(reason = "vendor-SDK: pass-through pointer thread")
fn ok(global: *const i64) -> *const i64 {
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
unsafe(reason = "MMIO: byte read at user-supplied addr")
fn read_byte(p: *const u8) -> Tainted<u8> { return *p; }

fn safe_caller(p: *const u8) {  // ERROR: *const in safe sig
}

unsafe(reason = "MMIO: bounded-region byte fetch")
fn safe_caller2(p: *const u8) -> i64 {
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

- **Full Rust-style borrow checker.** You said you have a simpler
  variant — keep it; don't rebuild.
- **Garbage collector.** Out of scope by user requirement.
- **Capability tokens.** Overlaps too much with affine ownership;
  the marginal user value isn't worth the syntactic overhead.
- **ASAN/MSan runtime.** Too heavy for embedded; the canary +
  generational handle combination covers the common cases at a
  fraction of the cost.

(Region typing is **deferred to Layer 5**, not skipped — see below.)

---

## Layer 5 — Region typing (future, after Layer 1–4 ship)

**Status:** queued for after generational handles ship and stabilize.
Not gated by any Arc work. Adds a **second** pointer-safety mechanism
alongside Handle<T>; the two coexist (Handle for the 90% case,
`&'arena T` for hot loops and safety-critical workloads). The user
picks per-type.

**Goal:** allow zero-runtime-cost pointer derefs with compile-time
use-after-free guarantee. Necessary for safety-critical embedded
(ASIL-D automotive, DO-178C avionics, IEC 62304 medical) where:
- runtime checks can't be relied on to catch all UAF before
  certification, and
- every cycle is budgeted

**Design sketch (v0, will refine before implementation):**

```vani
// Region introduced by a scope block. All allocations inside
// the block carry the region's lifetime.
region {
  let mut arena: Region;
  let p: &'arena Node = arena.alloc(Node { ... });
  // ... use p freely; no runtime check, no taint.
}
// `p` cannot escape the region block; checker enforces.
```

**Key rules:**
- `&'r T` is a pointer with a region tag `'r`.
- A `&'r T` cannot be stored into a slot whose lifetime exceeds `'r`.
- A `region { ... }` block introduces a fresh `'r`; allocations are
  bump-pointer in v1, free at block exit.
- No generation counter, no slot lookup, no taint propagation.
- Affine ownership still applies to the Region itself (one owner per
  arena).

**Files touched (estimate):**
- `src/ast.rs` — add `'lifetime` parameter to `Type::Ref` and
  `Type::Ptr`; add `region { ... }` statement.
- `src/checker.rs` — region inference + lifetime-bound enforcement
  on stores/returns. Largest piece of work.
- `src/backend_c.rs` — emit per-region bump allocator struct + bulk
  free at scope exit.
- `src/backend_llvm.rs` — same.
- Lifetime annotations on existing standard library helpers as
  opt-in (most stay Handle-based; performance-critical ones get
  region variants).

### Sub-steps (rough estimate, ~15–25h across ~8 commits)

1. **5.1 — Parser: `region { ... }` block + `'name` lifetime tokens. (~2h)**
2. **5.2 — AST: `Type::Ref(lifetime, inner)`. (~1h)**
3. **5.3 — Checker: region scope tracking + lifetime inference. (~5–7h)**
4. **5.4 — Checker: lifetime-bound store/return enforcement. (~3–4h)**
5. **5.5 — C backend: bump-allocator emission for regions. (~3h)**
6. **5.6 — LLVM backend: same. (~3h)**
7. **5.7 — Stdlib opt-in lifetime-annotated variants for hot Vec / Trie ops. (~2h)**
8. **5.8 — Safety-critical example: ring buffer using regions, cross-backend parity. (~1h)**

**Acceptance:**
```vani
region {
  let arena: Region;
  let head: &'arena Node = arena.alloc(Node::new());
  // ... build a tree of nodes, all referencing each other via
  //     &'arena. Zero runtime cost per deref. Compile-time use-
  //     after-free check.
}
// head, arena, and all allocations dropped together here.
```

If a user writes a function that returns `&'a Node` for some `'a`
that exceeds the calling region's lifetime, the checker rejects it
at compile time — same class of guarantee Rust provides, scoped to
the regions feature.

**Interop with generational handles:**
- A function can accept `Handle<T>` OR `&'a T` at the API boundary.
- Conversions allowed only one-way: `&'a T → Handle<T>` requires
  the value to be moved into a Pool (consuming the borrow).
- The other direction (`Handle<T> → &'a T`) requires a `pool.get(h)`
  call and the resulting `&'a T` is bounded by the pool's region.

**Why this isn't ready to ship now:**
- Generational handles are simpler and unblock 90% of embedded use
  today. Ship that first.
- Region syntax / semantics deserve user feedback before we lock it
  in. Better to evolve the API with one user community than two.
- Region typing benefits compound with experience writing
  lifetime-annotated code; embedded engineers will need ramp time.

---

## Suggested implementation order

### v1 — generational handles (mainstream embedded)

1. **Layer 1.1** — Lexical `unsafe` containment (2–3h)
2. **Layer 1.2** — No-escape on `&local` (3–4h)
3. **Layer 4.1** — Stack canaries via toolchain flag (30min, free win)
4. **Layer 2.1a–2.1c** — `Handle<T>` type + Pool<T> codegen (5–7h)
5. **Layer 3.1** — Canary words around unsafe alloc (2h)
6. **Layer 1.3** — Taint bit on unsafe-derived values (4–5h)
7. **Layer 2.2** — Handle as the only `unsafe` escape (1h)
8. **Layer 3.2** — `BoundedPtr<T>` fat pointer (3h)
9. **Layer 4.2** — ARM MTE flag (1h)

**v1 estimated effort:** 22–31 hours across ~12 commits.

### v2 — region typing (safety-critical opt-in)

10. **Layer 5.1–5.8** — Region typing (15–25h across ~8 commits)

**Sequencing constraint:** v1 must ship + stabilize first. We want
real user feedback on Handle<T> ergonomics before locking in region
syntax. Estimate v1 → v2 gap: weeks-to-months of usage.

**Combined total:** 37–56 hours across ~20 commits, across two
shipping phases.

---

## Cross-cutting decisions

- **Tainted<T>**, **Handle<T>**, and (later) **`&'arena T`** are the
  blessed types that cross safe/unsafe. Document this explicitly in
  the language guide.
- **`unsafe`** is the only escape hatch for raw pointers. No
  "trusted libs," no privileged crates.
- **Release mode** strips: canaries (Layer 3.1), taint runtime
  checks (Layer 1.3's `assert_safe`), bounds checks on `BoundedPtr`
  (Layer 3.2). Generational handles (Layer 2) stay on — they're the
  load-bearing safety net. Region pointers (Layer 5) are compile-
  time only, nothing to strip.
- **Two-tier safety mental model:** Handle<T> is the default,
  ergonomic, runtime-checked path. `&'arena T` is the opt-in,
  zero-cost, compile-time-proven path for hot loops and safety-
  critical workloads. Users pick per-type, not per-program.
- **Migration story:** code written against Handle<T> stays valid
  forever; users can selectively migrate hot paths to regions when
  the perf budget demands it. No big-bang rewrite.

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
5. Layer 5 (regions) waits for v1 (Layers 1–4) to ship and gather
   real-world feedback. Don't start before that.

Arc 1 (HashMap monomorphization) is the long pole; this safety work
shouldn't gate it.

## Safety-critical readiness checklist

Once Layer 5 ships, vāṇी can credibly claim suitability for:
- **ASIL-D automotive** (ISO 26262) — needs compile-time UAF proof
  (regions) + canaries + taint
- **DO-178C avionics** (Level A) — same plus deterministic timing
  (regions, no runtime gen-check tax)
- **IEC 62304 medical** (Class C) — same plus auditable allocation
  sites (regions make allocator boundaries explicit)

Until Layer 5 ships, vāṇी is positioned for **mainstream embedded /
IoT / consumer firmware / robotics** — substantially safer than
C/C++ but not certifiable for the categories above.
