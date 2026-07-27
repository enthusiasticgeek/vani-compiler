# Advanced 5 -- SIMD and NEON vectorization

> **Learning goal**: understand vāṇī's three-layer SIMD story —
> auto-vectorization (free), the `#[vectorize]` hint (better
> pipelining), and the `vec128<T>` type + `simd_*` builtins
> (explicit register control). Know when each layer is the right tool
> and how they compose on x86-64 and AArch64/NEON.
>
> Reading order:
> [Advanced 4b -- Cross-compilation](04b_cross_compile_primer.md)
> -> here.

---

## One stamp, four cookies

Picture rolling out cookie dough and cutting shapes one at a time
with a single cutter: press, lift, move, press, lift, move -- one
cookie per press. Now swap in a cutter that has *four* shapes welded
into one frame, spaced to match the dough. One press, one lift, and
you've cut four cookies at once -- the same motion, four times the
output, because the four cuts happen simultaneously rather than one
after another.

That's the whole idea behind **SIMD** (Single Instruction, Multiple
Data): instead of a CPU adding one pair of numbers, then the next
pair, then the next, a SIMD instruction loads several numbers into
one wide register and adds all of them in a single step -- one
"press" that produces several results at once. The three layers
below are three different levels of control over how and when
vāṇी reaches for the four-shape cutter instead of the one-shape
cutter.

## Why three layers?

| Layer | What it does | When to use it |
|-------|-------------|----------------|
| Auto-vectorization | LLVM turns scalar loops into SIMD automatically | Always on; free. Use for every tight loop. |
| `#[vectorize]` attribute | Adds software-pipeline interleaving (×4 interleave count) on top of auto-vectorize | Add when profiling shows a bottleneck loop that auto-vectorize alone doesn't saturate |
| `vec128<T>` + `simd_*` | Explicit 128-bit register operations you control | When you need a specific permutation, reduction, or FFI-bridged intrinsic that LLVM won't produce on its own |

Most programs only need Layer 1. Layer 2 costs nothing to add and
rarely hurts. Layer 3 is a precision tool — reach for it when the
generated assembly shows the loop you need and LLVM still won't
produce it.

---

## Layer 1 — auto-vectorization (always on)

Every `while` loop in vāṇī gets `!llvm.loop.vectorize.enable`
metadata. LLVM's loop vectorizer then decides whether the loop is
safe and profitable to vectorize based on data-dependency analysis.

The width it chooses depends on the **target**:

| Target | `vectorize.width` hint | Natural register |
|--------|----------------------|-----------------|
| x86-64 (default host) | 4 | 128-bit SSE / 256-bit AVX2 |
| `aarch64-*` | 2 | 128-bit NEON (64-bit elements) |

The hint is conservative; LLVM often widens further when the
element type is narrower (e.g., a `Vec<i32>` loop on AArch64 may
use all 4 NEON lanes naturally).

### Example — dot product auto-vectorized

```vani
// vani-lang: english
intent "dot product";

fn dot(a: ref Vec<i64>, b: ref Vec<i64>, n: i64) -> i64 {
    let s: i64 = 0;
    let i: i64 = 0;
    while i < n {
        s = s + a[i] * b[i];
        i = i + 1;
    }
    return s;
}
fn main() -> i64 {
    let a: Vec<i64> = vec_fill(8, 1 as i64);
    let b: Vec<i64> = vec_fill(8, 2 as i64);
    print dot(ref a, ref b, 8);
    return 0;
}
```

Build and inspect:

```bash
vanic build dot.vani -o dot_ll --emit-llvm
# Look for: <2 x i64> or <4 x i64> multiply instructions in the .ll
```

---

## Layer 2 — `#[vectorize]` (software-pipeline hint)

The `#[vectorize]` attribute adds `!llvm.loop.interleave.count = 4`
to every `while` loop in the marked function. This tells LLVM's
backend to **software-pipeline four loop iterations** — filling
pipeline bubbles with work from the next iteration before the
current one completes. It is additive: auto-vectorization still
runs in addition to interleaving.

```vani
// vani-lang: english
intent "vectorized sum";

#[vectorize]
fn sum(data: ref Vec<i64>, n: i64) -> i64 {
    let s: i64 = 0;
    let i: i64 = 0;
    while i < n {
        s = s + data[i];
        i = i + 1;
    }
    return s;
}
fn main() -> i64 {
    let v: Vec<i64> = vec_fill(1000000, 1 as i64);
    print sum(ref v, 1000000);
    return 0;
}
```

**Rules:**
- `#[vectorize]` applies to the function, not individual loops.
  All `while` loops in the function get the interleave hint.
- It has no effect on non-loop code.
- It does not force vectorization of non-vectorizable loops —
  LLVM still has the final say on safety.

**AArch64 / NEON note:** on `--target aarch64-unknown-linux-gnu`
the width hint drops to 2 (NEON 64-bit pairs). `#[vectorize]`
still adds the interleave hint, so you get 2-wide NEON lanes ×4
interleaved — effectively 8 elements in flight per cycle on in-order
cores like Cortex-A53.

---

## Layer 3 — `vec128<T>` and the `simd_*` builtins

`vec128<T>` is a **128-bit SIMD register value** holding `N` lanes
of type `T`. It is a first-class vāṇी type: you can declare variables,
pass them to functions, and return them.

### Lane counts

| Element type | Lanes | LLVM type |
|-------------|-------|-----------|
| `i8` / `u8` | 16 | `<16 x i8>` |
| `i16` / `u16` | 8 | `<8 x i16>` |
| `i32` / `u32` | 4 | `<4 x i32>` |
| `f32` | 4 | `<4 x float>` |
| `i64` / `u64` | 2 | `<2 x i64>` |
| `f64` | 2 | `<2 x double>` |

The element type must be a numeric scalar. `vec128<bool>` and
`vec128<Vec<i64>>` are compile errors.

### The seven builtins

| Builtin | Signature | What it does |
|---------|-----------|-------------|
| `simd_splat` | `(val: T) -> vec128<T>` | Broadcast scalar to all lanes |
| `simd_load` | `(v: Vec<T> \| ref Vec<T>, idx: i64) -> vec128<T>` | Load N lanes from `v[idx..]` |
| `simd_store` | `(v: Vec<T> \| ref Vec<T>, idx: i64, data: vec128<T>) -> Vec<T>` | Store N lanes to `v[idx..]` |
| `simd_add` | `(a: vec128<T>, b: vec128<T>) -> vec128<T>` | Lane-wise add |
| `simd_sub` | `(a: vec128<T>, b: vec128<T>) -> vec128<T>` | Lane-wise subtract |
| `simd_mul` | `(a: vec128<T>, b: vec128<T>) -> vec128<T>` | Lane-wise multiply |
| `simd_reduce_add` | `(v: vec128<T>) -> T` | Horizontal add (all lanes → one scalar) |

`simd_load` and `simd_store` access the **heap buffer** of a
`Vec<T>` directly — no bounds checking, no copy of the fat pointer.
The caller is responsible for ensuring `idx + N ≤ len(v)`.

### Example — explicit SAXPY (single-precision)

```vani
// vani-lang: english
// y[i..i+4] += alpha * x[i..i+4]  -- four f32 lanes at a time
intent "SAXPY f32";

fn saxpy_f32(
    y: ref Vec<f32>,
    x: ref Vec<f32>,
    alpha: f32,
    n: i64
) -> i64 {
    let splat_alpha: vec128<f32> = simd_splat(alpha);
    let i: i64 = 0;
    while i + 4 <= n {
        let xi: vec128<f32> = simd_load(x, i);
        let yi: vec128<f32> = simd_load(y, i);
        let ax: vec128<f32> = simd_mul(splat_alpha, xi);
        let res: vec128<f32> = simd_add(yi, ax);
        let _ = simd_store(y, i, res);
        i = i + 4;
    }
    return 0;
}

fn main() -> i64 {
    let n: i64 = 8;
    let x: Vec<f32> = vec_fill(n, 2.0 as f32);
    let y: Vec<f32> = vec_fill(n, 1.0 as f32);
    let _ = saxpy_f32(ref y, ref x, 3.0 as f32, n);
    // y[i] should now be 1 + 3*2 = 7
    return 0;
}
```

### Example — horizontal sum with `simd_reduce_add`

```vani
// vani-lang: english
intent "horizontal sum";

fn hsum(v: ref Vec<i32>, n: i64) -> i64 {
    let total: i32 = 0 as i32;
    let i: i64 = 0;
    while i + 4 <= n {
        let chunk: vec128<i32> = simd_load(v, i);
        total = total + simd_reduce_add(chunk);
        i = i + 4;
    }
    // scalar tail
    while i < n {
        total = total + v[i] as i32;
        i = i + 1;
    }
    return total as i64;
}

fn main() -> i64 {
    let v: Vec<i32> = vec_fill(16, 1 as i32);
    print hsum(ref v, 16);   // 16
    return 0;
}
```

---

## Layer 4 — `vec256<T>` and the `simd256_*` builtins

`vec256<T>` is a **256-bit SIMD register value** — twice the width of
`vec128<T>`. It holds more lanes per operation, which means fewer loop
iterations on the same data:

| Element type | Lanes in `vec128<T>` | Lanes in `vec256<T>` |
|-------------|---------------------|---------------------|
| `i8` / `u8` | 16 | 32 |
| `i16` / `u16` | 8 | 16 |
| `i32` / `u32` / `f32` | 4 | 8 |
| `i64` / `u64` / `f64` | 2 | 4 |

The `simd256_*` builtins mirror the `simd_*` set exactly — they take
`vec256<T>` arguments instead of `vec128<T>`:

| Builtin | Signature | What it does |
|---------|-----------|-------------|
| `simd256_splat` | `(val: T) -> vec256<T>` | Broadcast scalar to all lanes |
| `simd256_load` | `(v: Vec<T>, idx: i64) -> vec256<T>` | Load N lanes from `v[idx..]` |
| `simd256_store` | `(v: Vec<T>, idx: i64, d: vec256<T>) -> Vec<T>` | Store N lanes |
| `simd256_add` | `(a: vec256<T>, b: vec256<T>) -> vec256<T>` | Lane-wise add |
| `simd256_sub` | `(a: vec256<T>, b: vec256<T>) -> vec256<T>` | Lane-wise subtract |
| `simd256_mul` | `(a: vec256<T>, b: vec256<T>) -> vec256<T>` | Lane-wise multiply |
| `simd256_reduce_add` | `(v: vec256<T>) -> T` | Horizontal sum of all lanes |

### Example — 8-lane f32 dot product

```vani
fn dot256(a: ref Vec<f32>, b: ref Vec<f32>, n: i64) -> f32 {
    let acc: vec256<f32> = simd256_splat(0.0 as f32);
    let i: i64 = 0;
    while i + 8 <= n {
        let ai: vec256<f32> = simd256_load(a, i);
        let bi: vec256<f32> = simd256_load(b, i);
        acc = simd256_add(acc, simd256_mul(ai, bi));
        i = i + 8;
    }
    let s: f32 = simd256_reduce_add(acc);
    // scalar tail
    while i < n {
        s = s + a[i] * b[i];
        i = i + 1;
    }
    return s;
}
```

The step is 8 instead of 4 (compared to `vec128<f32>`). On x86-64 with
AVX2, LLVM lowers the `<8 x float>` IR to `ymm` register operations
(`vfmadd231ps ymm0, ymm1, ymm2`). On AArch64 without SVE, LLVM
legalises the 256-bit type as two 128-bit NEON registers and emits
pairs of `fmla v0.4s` instructions.

### Platform mapping

| Platform | vec256<f32> ISA mapping | Notes |
|----------|------------------------|-------|
| x86-64 + AVX2 | `ymm` registers, 8 f32/op | LLVM selects `vfmadd231ps` etc. |
| x86-64 (no AVX) | 2× SSE registers (legalized by LLVM) | Still correct; no AVX needed |
| AArch64 (no SVE) | 2× NEON `v`-registers | Two 128-bit passes per loop iter |
| AArch64 + SVE | Single scalable vector ≥ 256 bits | Optimal with `--sve` / `--sve2` |
| RISC-V (RVV, VLEN≥256) | Single `vl=8` e32 register | Optimal on SiFive X280 |
| RISC-V (RVV, VLEN=128) | Two `vl=4` passes | Correct; LLVM handles it |

### When to prefer vec256 over vec128

Use `vec256<T>` when:
- The target is x86-64 with AVX2 (LSCPU shows `avx2`) — halves loop iterations
- The target is AArch64 + SVE — fits in one scalable register
- The target is RISC-V with VLEN=256 (SiFive X280, Milk-V Pioneer)

Use `vec128<T>` when:
- The target is AArch64 without SVE — `vec256` adds two-register overhead with no throughput gain
- You are targeting a minimal RVV core with VLEN=128

When in doubt, profile both. On this project's x86-64 benchmark machine
(Intel i5-1035G1, Ice Lake — has AVX2), `vec256<f32>` reduces loop
iterations by half and benchmark 12 (`12_simd256_dot`) measures the
wall-clock benefit.

---

## Layer 5 — `vec512<T>` and the `simd512_*` builtins

`vec512<T>` is a **512-bit SIMD register value** — twice the width of
`vec256<T>`. On x86-64 with AVX-512, this maps to a `zmm` register;
on AArch64 with SVE-512 it is a scalable vector fixed at 512 bits;
on RISC-V with RVV VLEN=512 it is a single vector register group.

| Element type | Lanes `vec128` | Lanes `vec256` | Lanes `vec512` |
|-------------|---------------|---------------|---------------|
| `i8` / `u8` | 16 | 32 | 64 |
| `i16` / `u16` | 8 | 16 | 32 |
| `i32` / `u32` / `f32` | 4 | 8 | 16 |
| `i64` / `u64` / `f64` | 2 | 4 | 8 |

The `simd512_*` builtins mirror the `simd256_*` set exactly:

| Builtin | Signature | What it does |
|---------|-----------|-------------|
| `simd512_splat` | `(val: T) -> vec512<T>` | Broadcast scalar to all lanes |
| `simd512_load` | `(v: ref Vec<T>, idx: i64) -> vec512<T>` | Load N lanes from `v[idx..]` |
| `simd512_store` | `(v: ref Vec<T>, idx: i64, d: vec512<T>) -> i64` | Store N lanes |
| `simd512_add` | `(a: vec512<T>, b: vec512<T>) -> vec512<T>` | Lane-wise add |
| `simd512_sub` | `(a: vec512<T>, b: vec512<T>) -> vec512<T>` | Lane-wise subtract |
| `simd512_mul` | `(a: vec512<T>, b: vec512<T>) -> vec512<T>` | Lane-wise multiply |
| `simd512_reduce_add` | `(v: vec512<T>) -> T` | Horizontal sum of all lanes |

### Example — 16-lane f32 dot product

```vani
fn dot512(a: ref Vec<f32>, b: ref Vec<f32>, n: i64) -> f32 {
    let acc: vec512<f32> = simd512_splat(0.0 as f32);
    let i: i64 = 0;
    while i + 16 <= n {
        let ai: vec512<f32> = simd512_load(a, i);
        let bi: vec512<f32> = simd512_load(b, i);
        acc = simd512_add(acc, simd512_mul(ai, bi));
        i = i + 16;
    }
    let s: f32 = simd512_reduce_add(acc);
    // scalar tail for remaining elements
    while i < n {
        s = s + a[i] * b[i];
        i = i + 1;
    }
    return s;
}

fn main() -> i64 {
    let n: i64 = 32;
    let a: Vec<f32> = vec_fill(n, 1.0 as f32);
    let b: Vec<f32> = vec_fill(n, 2.0 as f32);
    let result: f32 = dot512(ref a, ref b, n);
    print result;   // 64.0  (32 × 1.0 × 2.0)
    return 0;
}
```

The step is 16 (sixteen `f32` lanes per iteration). On x86-64 with
AVX-512, LLVM lowers the `<16 x float>` IR to `zmm` register operations
(`vfmadd231ps zmm0, zmm1, zmm2`). On targets without AVX-512, LLVM
legalises the 512-bit type into multiple narrower registers (two `ymm`
on AVX2, four `xmm` on SSE) — the code remains correct with reduced
throughput.

### Platform mapping

| Platform | `vec512<f32>` mapping | Notes |
|----------|-----------------------|-------|
| x86-64 + AVX-512 | Single `zmm` register, 16 f32/op | `vfmadd231ps zmm0,zmm1,zmm2` |
| x86-64 + AVX2 (no AVX-512) | 2× `ymm` (legalised) | Correct; half the throughput of native 512 |
| x86-64 (SSE only) | 4× `xmm` (legalised) | Correct; quarter throughput |
| AArch64 + SVE-512 | Single scalable `z`-register | `fadd z0.s, z1.s, z2.s` |
| AArch64 (NEON, no SVE) | 4× NEON registers | Legalised; correct |
| RISC-V + RVV VLEN=512 | Single `vl=16` e32 group | Optimal on VisionFive 2 / T-Head C910 |
| RISC-V + RVV VLEN=256 | Two vector-register groups | Legalised |
| RISC-V + RVV VLEN=128 | Four vector-register groups | Legalised |

### When to prefer vec512 over vec256

Use `vec512<T>` when:
- The target CPU has AVX-512 (`lscpu | grep avx512f`) — Intel Ice Lake-SP,
  Sapphire Rapids, AMD Zen 4, AWS Graviton-3 (via SVE-512)
- The target is AArch64 with SVE-512 (Graviton-3, Neoverse V1/V2) — one
  scalable register per operation, no legalisation cost
- The target is RISC-V with VLEN=512 (T-Head C910 in LicheePi 4A,
  some SiFive cores) — maximum RVV throughput with a single vector group

Use `vec256<T>` or `vec128<T>` when:
- The CPU has AVX2 but not AVX-512 — `vec512` legalises to two `ymm`
  registers with an extra merge step; `vec256` is usually faster
- The AArch64 target has NEON only (Cortex-A53/A72/A78 without SVE) —
  `vec512` legalises to four NEON registers; `vec128` processes one
  true register per operation with lower overhead
- Code size matters: AVX-512 encodings are larger than AVX2

When in doubt, benchmark with the specific CPU and data size.
`vec256<f32>` is the safe default for cross-platform code; reach for
`vec512` only after confirming the target has native support.

---

## AArch64 / NEON specifics

On AArch64 targets the `vec128<T>` type maps directly to NEON
128-bit registers (`v0`–`v31`). LLVM picks the right NEON
instruction for each `simd_*` builtin:

| Builtin | AArch64 instruction (i32 example) |
|---------|----------------------------------|
| `simd_splat` | `dup v0.4s, w0` |
| `simd_add` | `add v0.4s, v1.4s, v2.4s` |
| `simd_mul` | `mul v0.4s, v1.4s, v2.4s` |
| `simd_reduce_add` | `addv s0, v1.4s` |
| `simd_load` | `ldr q0, [x0, x1, lsl #2]` |
| `simd_store` | `str q0, [x0, x1, lsl #2]` |

No intrinsics, no `#include <arm_neon.h>`, no C interop needed.

### SVE / SVE2 (scalable vectors)

For Cortex-A510 / A710 / A715 and Neoverse N2 / V1 that have SVE,
you can opt in at the compiler level to widen the register file
beyond 128 bits. `vec128<T>` still emits fixed-width 128-bit
vectors; the SVE flags affect LLVM's auto-vectorizer only:

```bash
# Enable SVE (scalable vector extension, 128–2048 bit)
vanic build saxpy.vani --target=aarch64-unknown-linux-gnu --sve

# Enable SVE2 (AArch64 v9, includes SVE)
vanic build saxpy.vani --target=aarch64-unknown-linux-gnu --sve2
```

Both flags are ignored on non-AArch64 targets (the compiler emits
a clear error). `--sve2` wins if both are given.

---

## FFI shim escape hatch

When you need an intrinsic that `vec128<T>` does not expose —
`vaddq_s64` with a carry, `vmull_high_u32`, AES acceleration — use
an FFI shim: a tiny C file that wraps the intrinsic and is linked
with `--link` at build time.

See [docs/simd_ffi_shims.md](../../../docs/simd_ffi_shims.md) for
full NEON and AVX2 shim examples and the type-mapping table.

---

## Combining all three layers

Layers 1, 2, and 3 compose:

```vani
// vani-lang: english
intent "combined SIMD layers";

#[vectorize]               // Layer 2: interleave scalar loops
fn process(
    out: ref Vec<f32>,
    inp: ref Vec<f32>,
    n: i64
) -> i64 {
    // Layer 3: explicit vec128 for the SIMD hot path
    let i: i64 = 0;
    while i + 4 <= n {
        let v: vec128<f32> = simd_load(inp, i);
        let v2: vec128<f32> = simd_mul(v, v);
        let _ = simd_store(out, i, v2);
        i = i + 4;
    }
    // Layer 1: auto-vectorized scalar tail
    while i < n {
        out[i] = inp[i] * inp[i];
        i = i + 1;
    }
    return 0;
}

fn main() -> i64 {
    let n: i64 = 8;
    let inp: Vec<f32> = vec_fill(n, 3.0 as f32);
    let out: Vec<f32> = vec_fill(n, 0.0 as f32);
    let _ = process(ref out, ref inp, n);
    return 0;
}
```

The `#[vectorize]` interleave hint applies to the scalar tail loop;
the SIMD hot path uses `vec128<f32>` directly. LLVM sees both and
can schedule across them.

---

## Decision guide

```
Is the loop already fast enough?
    Yes → stop. Auto-vectorization is working.
    No  → profile first. Is this loop the bottleneck?
              No  → optimize elsewhere.
              Yes → try #[vectorize] first (one attribute, free).
                        Still slow?
                            → Use vec128<T> for the hot path.
                            Still slow?
                                → Try vec256<T> if target has AVX2 / SVE / VLEN≥256.
                                Still slow?
                                    → Try vec512<T> if target has AVX-512 / SVE-512 / VLEN=512.
                                    → Use FFI shim for exotic intrinsics not exposed by simd512_*.
```

---

## Cross-references

- [04b Cross-compilation primer](04b_cross_compile_primer.md) — `--target`, `--cpu`, `--sve`/`--sve2` flags
- [04_embedded Embedded + unsafe](04_embedded.md) — MMIO, `#[no_heap]`, bare-metal `parallel for` notes
- [04c Attributes reference](04c_attributes_reference.md) — `#[vectorize]` and all other `fn` attributes
- [docs/simd_ffi_shims.md](../../../docs/simd_ffi_shims.md) — NEON / AVX2 FFI shim cookbook
- [Benchmark 11 — SIMD dot product](../../../benchmarks/11_simd_dot/README.md) — vec128 explicit vs. auto-vectorized timings
- [Benchmark 12 — SIMD-256 dot product](../../../benchmarks/12_simd256_dot/README.md) — vec256 vs vec128 vs auto-vectorized on x86-64 AVX2


---

**Previous**: [Sec.4c -- Attributes reference ->](04c_attributes_reference.md)
**Next**: [Sec.5 -- The dyn vtable layout + safety boundary ->](05_vtables.md)

