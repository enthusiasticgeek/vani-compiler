# Advanced 4c -- Function attributes reference

> A one-page reference for every `#[attribute]` vāṇी supports
> on `fn` declarations. Attributes are the primary way to
> add safety requirements, linker directives, and verification
> constraints at the function level.

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

### `#[bounded_stack(N)]`

Assert that the function's total stack usage (frame size + all
transitive calls) does not exceed `N` bytes.

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
| `#[bounded_stack(N)]` | Stack budget <= N bytes | None (compile-time estimate) |
| `#[recursion_bound(N)]` | Max recursion depth | 1 counter increment per call |
| `#[deterministic_timing]` | No dynamic-dispatch / unbounded loops | None (compile-time) |
| `#[interrupt]` | ISR calling convention | Register save/restore at entry/exit |
| `#[bounded(N)]` | Allow bounded mutual recursion in acyclicity | None |
| `#[no_mangle]` | Exact symbol name in object file | None |
| `#[link_section = "..."]` | ELF section placement | None |

---

## Cross-reference

- [Advanced 4a -- Embedded primer](04a_embedded_primer.md) -- the big picture of embedded constraints
- [Advanced 4b -- Cross-compilation primer](04b_cross_compile_primer.md) -- `--target`, `--no-std`, bare-metal workflow
- [Advanced 4 -- Embedded targets + `unsafe`](04_embedded.md) -- worked examples with `#[no_heap]` + `#[bounded_stack]`
- [CLI Reference](../beginner/00_cli_reference.md) -- `vanic stack-depth`, `vanic acyclicity`, `vanic safety-attrs`
