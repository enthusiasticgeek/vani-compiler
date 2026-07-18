# Advanced 4c -- Attributes reference

> A one-page reference for every `#[attribute]` vāṇी supports
> on `fn` and `struct` declarations. Attributes add safety
> requirements, linker directives, FFI layout control, and
> verification constraints.

## Struct attributes

### `#[repr(C)]`

Force C-compatible field layout and padding on a struct so it
can be passed to and from `extern "C"` functions by value:

```vani
#[repr(C)]
struct Point { x: i64, y: i64 }

extern "C" fn c_translate(p: Point, dx: i64, dy: i64) -> Point;
```

Without `#[repr(C)]`, vāṇी is free to reorder or pad fields for
its own purposes; with it, the in-memory layout is identical to
what a C compiler would produce for the equivalent `struct`.

**Required when**: passing structs by value across an FFI
boundary, or when the struct must match a hardware register map.

---

### `#[repr(packed)]`

Like `#[repr(C)]` but also removes all padding between fields.
Fields may be unaligned; access can be slower on platforms that
trap on unaligned loads.

```vani
#[repr(packed)]
struct UartPacket { header: i64, payload: i64, crc: i64 }
```

**Use for**: wire protocols, file formats, and register maps
where byte layout is specified by an external standard and
padding must be absent.

---

## Function attributes

---

## Safety and resource attributes

### `#[no_heap]`

Compile-time guarantee that this function (and everything it
transitively calls) never allocates from the heap.

```vani
#[no_heap]
fn process(buf: ref [u8; 256]) -> i64 {
  // Any call to a function that uses Vec, OwnedStr, Box, or
  // any other heap-backed type is a compile error here.
  return buf[0] as i64;
}
```

The compiler verifies the entire transitive call graph. If any
reachable function calls `malloc` (directly or via a built-in),
you get a clear error pointing at the offending call.

**Use for**: interrupt service routines, real-time control loops,
safety-critical paths where allocation must be absent.

---

### `#[no_float]`

Reject any use of floating-point types (`f32`, `f64`) in this
function's signature, local bindings, or transitive calls.

<img class="manas" src="../images/mascot/manas_mascot_caution.png" title="this code needs extra care"/>

```vani
#[no_float]
fn pid_integer(err: i64, integral: i64) -> i64 {
  // Compile error if any f64 type appears anywhere in this fn
  // or any fn it calls.
  return err * 5 + integral / 10;
}
```

Required by ASIL-D and DO-178C Level A when the floating-point
unit is unavailable or when IEEE-754 non-determinism is unacceptable.

---

### `#[no_nan]`

Reject calls to builtins that are **defined** to produce IEEE-754
quiet NaN as part of their error contract:

- `f64_nan()` — the explicit NaN constructor.
- `vec_kth_smallest` on `Vec<f64>` — returns `qNaN` (`0x7FF8000000000000`)
  when `k` is out of bounds.

```vani
#[no_nan]
fn safe_ratio(a: f64, b: f64) -> f64 {
  // Regular arithmetic is fine; f64_nan() would be a compile error.
  if b == 0.0 { return 0.0; }
  return a / b;
}
```

Does **not** statically block arithmetic that *can* produce NaN on
bad inputs (e.g. `sqrt(-1.0)`) — that requires value-range SMT proofs.
Implied by `#[asil_d]` and `#[do178c_level_a]`.

---

### `#[no_recursion]`

Forbid direct or transitive recursion. Stricter than `#[bounded(N)]`:
the diagnostic explicitly names recursion as the cause and detects
mutual recursion via call-graph cycle detection.

```vani
#[no_recursion]
fn safe_sum(xs: ref Vec<i64>) -> i64 {
  let acc: i64 = 0;
  for i in 0..len(xs) { acc = acc + xs[i]; }
  return acc;
}
```

Implied by `#[asil_d]`, `#[do178c_level_a]`, `#[iec_62304_class_c]`,
and `#[misra_c_2012]`.

---

### `#[wcet(cycles = N)]`

Assert that the function's worst-case execution time does not exceed
`N` CPU cycles. The compiler's static cycle estimator checks the
claim; `vanic stack-depth` reports the estimate.

```vani
#[asil_d]
#[bounded_stack(bytes = 512)]
#[wcet(cycles = 2000)]
fn brake_controller(speed: i64, pedal: i64) -> i64 {
  return speed * pedal / 100;
}
```

Required alongside `#[bounded_stack]` when using `#[asil_d]` or
`#[do178c_level_a]`.

---

### `#[bounded_stack(N)]`

Assert that the function's total stack usage (frame size + all
transitive calls) does not exceed `N` bytes.

<img class="manas" src="../images/mascot/manas_mascot_caution.png" title="this code needs extra care"/>

```vani
#[bounded_stack(512)]
fn handle_packet(buf: ref [u8; 64]) -> i64 {
  // Compile error if any path through this fn uses > 512 bytes of stack.
  return 0;
}
```

`vanic stack-depth` reports the estimate; `--max=N` on that
command makes the budget a CI gate.

---

### `#[recursion_bound(N)]`

Limit the maximum recursion depth to `N`. The compiler generates
a runtime depth counter and panics if exceeded.

```vani
#[recursion_bound(100)]
fn fibonacci(n: i64) -> i64 {
  if n <= 1 { return n; }
  return fibonacci(n - 1) + fibonacci(n - 2);
}
```

Required for DO-178C / ASIL-D on functions that must be
"dynamically bounded." Combine with `#[bounded_stack]` for
full stack-safety certification.

---

### `#[deterministic_timing]`

Reject any construct whose execution time is not statically
predictable: dynamic dispatch (`dyn Iface`), unbounded loops
without a known trip count, heap allocation, etc.

```vani
#[deterministic_timing]
fn pid_controller(err: f64, integral: f64) -> f64 {
  return 0.5 * err + 0.1 * integral;
}
```

**Use for**: hard real-time tasks where worst-case execution time
(WCET) analysis is required.

---

### `#[interrupt]`

Mark a function as an interrupt service routine (ISR). The
compiler emits the interrupt-specific calling convention
(callee-saves all registers; no return value coercion on
ARM/RISC-V). Implies `#[no_heap]` -- ISRs must not allocate.

```vani
#[interrupt]
#[no_mangle]
fn SysTick_Handler() -> i64 {
  // Called by hardware on every SysTick tick.
  return 0;
}
```

Combine with `#[no_mangle]` so the linker vector table can
reference the exact symbol name.

---

### `#[bounded(N)]`

For mutually recursive functions: declare that the function is
part of a mutually-recursive group bounded by `N` total steps.
Used by `vanic acyclicity` to allow bounded cycles.

```vani
#[bounded(10)]
fn even(n: i64) -> bool {
  if n == 0 { return true; }
  return odd(n - 1);
}

#[bounded(10)]
fn odd(n: i64) -> bool {
  if n == 0 { return false; }
  return even(n - 1);
}
```

Without `#[bounded]`, `vanic acyclicity` would reject the mutual
recursion between `even` and `odd`.

---

### Composite safety standards

One composite tag expands to a union of primitive constraints.
Two composite tags on the same function are rejected — stack
primitives instead.

| Composite | Implies |
|-----------|--------|
| `#[asil_d]` | `no_heap` + `no_recursion` + `no_float` + `no_nan` + `deterministic_timing`; requires `bounded_stack` + `wcet` |
| `#[do178c_level_a]` | same as `asil_d` |
| `#[iec_62304_class_c]` | `no_heap` + `no_recursion` |
| `#[misra_c_2012]` | `no_heap` + `no_recursion` + MISRA rules 13.2, 13.5, 14.1, 15.5 |

```vani
// ISO 26262 ASIL-D -- most stringent automotive level.
#[asil_d]
#[bounded_stack(bytes = 1024)]
#[wcet(cycles = 3000)]
fn compute_torque(speed: i64, pedal: i64) -> i64 {
  return speed * pedal / 100;
}
```

See [Advanced 12 -- Safety-critical standards](12_safety_standards.md)
for the full expansion matrix and certification workflow.

---

## Linker attributes

### `#[no_mangle]`

Suppress symbol name mangling. The function is emitted with its
exact vāṇी name (no `intent_` prefix, no Unicode encoding).

```vani
#[no_mangle]
fn Reset_Handler() -> i64 {
  // Emitted as:  Reset_Handler  (not intent_Reset_Handler)
  return 0;
}
```

**Required for**: bare-metal reset vectors, OS-ABI entry points
(`_start`, `main` for C interop), interrupt handlers referenced
by name in linker scripts.

Both C and LLVM backends honor this attribute.

---

### `#[link_section = "..."]`

Place the function in a named ELF/PE section instead of the
default `.text`.

```vani
#[no_mangle]
#[link_section = ".text.Reset_Handler"]
fn Reset_Handler() -> i64 { return 0; }

#[link_section = ".isr_vector"]
fn vector_table() -> i64 { return 0; }
```

C backend emits: `__attribute__((section(".text.Reset_Handler")))`.
LLVM backend adds `section ".text.Reset_Handler"` to the `define` line.

**Required for**: bare-metal linker scripts that map specific
sections to physical addresses (Flash base, RAM region, etc.).

---

### `#[inline]`

Hint to the compiler to inline this function's body into every call
site rather than generating a separate stack frame. The
`vanic stack-depth` analyser merges inlined locals into the caller's
frame, so `#[bounded_stack]` budgets account for it correctly.

```vani
#[inline]
fn clamp(v: i64, lo: i64, hi: i64) -> i64 {
  if v < lo { return lo; }
  if v > hi { return hi; }
  return v;
}
```

---

### `#[vectorize]`

Hint to the LLVM backend to auto-vectorize loops in this function
using SIMD instructions (SSE4 / AVX2 on x86-64; NEON on AArch64).
Has no effect in the C backend.

```vani
#[vectorize]
fn dot_product(a: ref Vec<f64>, b: ref Vec<f64>) -> f64 {
  let acc: f64 = 0.0;
  for i in 0..len(a) { acc = acc + a[i] * b[i]; }
  return acc;
}
```

---

## Combining attributes

Attributes can be stacked. A typical bare-metal ISR:

```vani
#[no_mangle]
#[link_section = ".text.isr"]
#[interrupt]
#[no_heap]
#[bounded_stack(256)]
fn HardFault_Handler() -> i64 {
  // Hardware fault handler -- no heap, bounded stack, no mangling,
  // placed in the .text.isr section.
  return 0;
}
```

The compiler verifies all constraints independently. If any is
violated you get a compile-time error with the violated attribute
named in the diagnostic.

---

## Quick-reference table

| Attribute | What it enforces | Runtime cost |
|-----------|-----------------|--------------|
| `#[no_heap]` | Transitive malloc-free | None (compile-time) |
| `#[no_float]` | No f32/f64 types anywhere in fn or callees | None (compile-time) |
| `#[no_nan]` | Reject NaN-contract builtins (`f64_nan`, `vec_kth_smallest<f64>`) | None (compile-time) |
| `#[no_recursion]` | Reject direct/mutual recursion via call-graph cycle | None (compile-time) |
| `#[wcet(cycles=N)]` | Worst-case cycle budget | None (compile-time estimate) |
| `#[bounded_stack(N)]` | Stack budget <= N bytes | None (compile-time estimate) |
| `#[deterministic_timing]` | No dynamic-dispatch / unbounded loops | None (compile-time) |
| `#[interrupt]` | ISR calling convention | Register save/restore at entry/exit |
| `#[recursion_bound(N)]` | Max recursion depth | 1 counter increment per call |
| `#[bounded(N)]` | Allow bounded mutual recursion in acyclicity | None |
| `#[inline]` | Inline body at call sites; stack merged by analyser | None |
| `#[vectorize]` | SIMD auto-vectorization hint (LLVM backend only) | None |
| `#[no_mangle]` | Exact symbol name in object file | None |
| `#[link_section = "..."]` | ELF section placement | None |
| `#[asil_d]` / `#[do178c_level_a]` | Composite: no_heap+no_recursion+no_float+no_nan+det_timing | None |
| `#[iec_62304_class_c]` | Composite: no_heap+no_recursion | None |
| `#[misra_c_2012]` | Composite: no_heap+no_recursion+MISRA rules | None |

---

## Cross-reference

- [Advanced 4a -- Embedded primer](04a_embedded_primer.md) -- the big picture of embedded constraints
- [Advanced 4b -- Cross-compilation primer](04b_cross_compile_primer.md) -- `--target`, `--no-std`, bare-metal workflow
- [Advanced 4 -- Embedded targets + `unsafe`](04_embedded.md) -- worked examples with `#[no_heap]` + `#[bounded_stack]`
- [CLI Reference](../beginner/00_cli_reference.md) -- `vanic stack-depth`, `vanic acyclicity`, `vanic safety-attrs`


---

**Previous**: [Sec.4 -- Embedded targets + unsafe + region typing ->](04_embedded.md)
**Next**: [Sec.5 -- SIMD and NEON vectorization ->](05_simd.md)
