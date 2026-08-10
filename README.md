# vāṇī (वाणी) — the vāṇī compiler & programming language

<p align="center">
<img src="vani_logo2.png" alt="v□~A□□~Gī logo" width=420">
</p>

<p align="center">
  <a href="https://enthusiasticgeek.github.io/vani-compiler/"><strong>📖 Tutorial Book</strong></a>
  &nbsp;•&nbsp;
  <a href="docs/language_manual.md"><strong>Language Manual</strong></a>
  &nbsp;•&nbsp;
  <a href="docs/languages.md"><strong>Language Coverage</strong></a>
  &nbsp;•&nbsp;
  <a href="docs/kosh_design.md"><strong>Kosh Package Manager</strong></a>
  &nbsp;•&nbsp;
  <a href="https://github.com/enthusiasticgeek/vani-compiler/releases"><strong>Releases</strong></a>
</p>

<p align="center">
  <a href="https://github.com/enthusiasticgeek/vani-compiler/actions/workflows/deploy-tutorials.yml">
    <img src="https://github.com/enthusiasticgeek/vani-compiler/actions/workflows/deploy-tutorials.yml/badge.svg" alt="Deploy Tutorials">
  </a>
</p>

**Verbose Alternative Natural Interface — code like you speak.**

vāṇī (pronounced *vaa-NEE*; Sanskrit वाणी = *speech*) is a systems language with the semantic model of Rust/C++ — static types, affine ownership, LLVM/C codegen, no GC — but a surface that reads left-to-right at speaking pace. It also natively understands 62 human languages via a `// vani-lang:` pragma.

*Familiar terrain, lighter outerwear.* If you've programmed in **C**, the route here should feel **C-scenic** — the same close-to-the-metal view, the same predictable cost, with the guardrails you used to keep in your head now kept by the compiler. If you're at home in **Rust**, the model is more **Rust-ic** than a re-invention — the same affine ownership, second-class references, monomorphized generics, and deterministic drop, dressed in softer punctuation. (These comparisons are descriptive; see *Trademark* below.)

> **Naming.** The CLI binary is **`vanic`** — a contraction of *vāṇī* + *saṃkalaka* (Sanskrit: "assembler / collector"). Other GitHub projects named "vani" are unrelated to this work.

> **Development note.** vāṇī is an experiment in human-directed AI-assisted compiler construction. The language architecture, design decisions, feature roadmap, and safety philosophy were conceived and directed by the author; the implementation was carried out by [Claude](https://claude.ai) (Anthropic) through iterative prompt engineering. This approach lets a single person build a production-grade compiler at a pace that would otherwise require a team — but it also means the codebase is young and bugs are to be expected. **vāṇī is still in its infancy.** If you hit a rough edge, please open an issue — early adopters shape the language.

---

## In one glance

```vani
fn add(a: i64, b: i64) -> i64
requires a >= 0 && b >= 0;
ensures _return == a + b;
{
  return a + b;
}

fn main() -> i64 {
  let xs: Vec<i64> = vec(3, 1, 4, 1, 5);
  sort(mut ref xs);
  let k: i64 = vec_kth_smallest(ref xs, 2);   // 3
  prove 2 + 2 == 4;
  print k;
  return 0;
}
```

| Rust / C++ | vāṇī | |
|------------|------|-|
| `&xs` | `ref xs` | shared borrow |
| `&mut xs` | `mut ref xs` | mutable borrow |
| `pub(crate) fn` | `pub(kosh) fn` | package-internal |
| `Vec::with_capacity(n)` | `vec_with_capacity(n)` | no path syntax |
| `match Some(x) => …` | `match Opt.Some(x) then …` | `then` not `=>` |
| `xs?` | `try xs` or `xs?` | both spellings compile |

### Same program, three ways

Every keyword above has alternate spellings — plain-English word forms
for `->` / `let` / `return`, and full Devanagari for `// vani-lang:
sanskrit` — so the same program reads however you speak. All three
below type-check, run, and print the same value.

**Verbose English** (`->` → `yields`, `let` → `assign`, `return` → `give_back`):

```vani
fn add(a: i64, b: i64) yields i64
requires a >= 0 && b >= 0;
ensures _return == a + b;
{
  give_back a + b;
}

fn main() yields i64 {
  assign xs: Vec<i64> = vec(3, 1, 4, 1, 5);
  sort(mut ref xs);
  assign k: i64 = vec_kth_smallest(ref xs, 2);   // 3
  prove 2 + 2 == 4;
  print k;
  give_back 0;
}
```

**Sanskrit** (`// vani-lang: sanskrit`; `main` is always literal — the
entry point is never translated):

```vani
// vani-lang: sanskrit

कार्य add(a: i64, b: i64) -> i64
अपेक्षित a >= 0 && b >= 0;
सुनिश्चयित _return == a + b;
{
  पुनरागम a + b;
}

कार्य main() -> i64 {
  माना xs: Vec<i64> = vec(3, 1, 4, 1, 5);
  sort(परिवर्तनीय दृष्ट्या xs);
  माना k: i64 = vec_kth_smallest(दृष्ट्या xs, 2);   // 3
  प्रमाण 2 + 2 == 4;
  लिख k;
  पुनरागम 0;
}
```

The Sanskrit version prints `३` — the same value `3`, rendered in
Devanagari numerals to match the source script. Builtin function names
(`vec`, `sort`, `vec_kth_smallest`) and the reserved `main` entry point
are never translated; only language keywords are.

---

## Install

```bash
cargo install vanic          # from crates.io
vanic build hello.vani -o hello
./hello
```

See [INSTALL.md](INSTALL.md) for platform-specific prerequisites (z3, LLVM tools).

---

## Platform & architecture support

Pre-built `vanic` binaries ship for all five targets on every [GitHub release](https://github.com/enthusiasticgeek/vani-compiler/releases). `cargo install vanic` works on any Rust-supported host.

| | x86-64 | AArch64 / ARM64 | RISC-V 64 |
|---|---|---|---|
| **Linux** | ✅ verified (Ubuntu 20–24, Debian, Arch, Fedora) | ✅ verified (cross-compiled; runs natively) | ✅ cross-compiled; QEMU run tested |
| **macOS** | ✅ binary shipped | ✅ Apple Silicon — binary shipped | — |
| **Windows** | ✅ verified — MSYS2 GNU toolchain + MSVC | — | — |
| **WSL 2** | ✅ identical to Linux | — | — |
| **Bare-metal** | C backend → link with any BSP | C backend → link with any BSP | C backend → link with any BSP |

> macOS binaries are built and shipped by CI; empirical on-device verification is pending (no macOS host). All lib tests and e2e tests pass on Linux x86-64 and Windows x86-64.

### SIMD widths

| Type | Width | x86-64 | AArch64 | RISC-V |
|---|---|---|---|---|
| `vec128<T>` | 128-bit | SSE2 / SSE4.1 `xmm` | NEON `v` regs | RVV VLEN=128 |
| `vec256<T>` | 256-bit | AVX-256 `ymm` | SVE-256 | RVV VLEN=256 |
| `vec512<T>` | 512-bit | AVX-512 `zmm` | SVE-512 | RVV VLEN=512 |

All three widths are ordinary generic types with a consistent builtin shape per width (`simd_add`/`simd256_add`/`simd512_add`, etc. — vec128's prefix has no width number) — no architecture-specific intrinsic headers required.

---

## Why vāṇī?

Systems languages occupy a narrow band: C is powerful but unsafe; Rust is safe but imposes lifetime annotations; neither has formal proofs or safety-certification enforcement in the language itself. vāṇī fills a specific gap.

| | C | Rust | vāṇī |
|---|---|---|---|
| Memory safety | ✗ manual | ✅ lifetime annotations required | ✅ no annotation syntax |
| Formal proofs (`requires` / `ensures`) | ✗ | ✗ | ✅ Z3-backed, compile-time |
| Safety-critical standards (ASIL-D, DO-178C) | external lint | external lint | ✅ compiler-enforced attributes |
| SIMD | intrinsic headers | `std::simd` (nightly) | ✅ stable first-class types |
| Source in any human language | ✗ | ✗ | ✅ 62 dialects, 26 scripts |
| Readable C output for audit / embedded | ✅ | ✗ | ✅ C backend |

Six concrete reasons to choose vāṇī:

**1. Ownership without lifetime annotations.**
Rust's affine ownership model, but you never write `'a`. References are second-class (`ref x`, `mut ref x`) — their scope is enforced at the call site by the type system, not by a separate lifetime parameter. The mental load of borrow-checking is real; the annotation syntax is optional complexity vāṇī removes.

**2. Formal verification as language syntax, not a plugin.**
`requires`, `ensures`, `prove`, and loop `invariant` are keywords, not library macros or separate tooling. Z3 discharges them at compile time via a three-stage pipeline (constant-fold → structural tautology → full SMT solve). Contracts on arithmetic, ordering, and data-structure invariants are verified in the same `vanic check` pass that type-checks your program.

**3. Safety-critical certification in the compiler.**
`#[asil_d]`, `#[do178c_level_a]`, `#[wcet = "120ns"]`, `#[no_nan]`, `#[no_heap]` are enforced by the type and safety checker, not by an external linting overlay. Annotate a function with `#[asil_d]` and the compiler rejects FP without determinism guards, unguarded recursion, missing `wcet` bounds, and non-deterministic timing — at the same pass that checks types. ISO 26262, DO-178C, IEC 62304, and MISRA C 2012 rule coverage is built in.

**4. Write source code in any of 62 human languages.**
Add `// vani-lang: hindi` and every keyword (`let`, `fn`, `return`, `match`) accepts its Devanagari equivalent. 62 dialects across 26 scripts are shipped and enforced: Devanagari, Bengali, Tamil, Arabic, Japanese, Mandarin, Russian, and more. A dialect purity checker rejects out-of-language identifiers, keeping multilingual codebases coherent. No other production systems language offers this.

**5. SIMD at every width with no intrinsic headers.**
`vec128<T>`, `vec256<T>`, and `vec512<T>` are generic types you use like any other. `simd256_add(a, b)` compiles to AVX-256 `ymm` operations on x86-64, NEON on AArch64, and RVV on RISC-V. You write once; the backend selects the right instruction set. No `#include <immintrin.h>`, no `_mm256_add_ps`, no `#ifdef __AVX2__`.

**6. Two backends for two deployment realities.**
The LLVM backend (`--backend=llvm`, default) optimises through `opt -O3` and `llc` for maximum throughput. The C backend (`--backend=c`) emits readable, portable C that can be compiled with any C11 compiler on any target, audited line-by-line, and linked into existing embedded BSPs without an LLVM toolchain on the host. Switch with one flag; both backends pass the same test suite.

---

## What it targets

| Domain | Key feature |
|--------|-------------|
| Embedded / bare-metal | `--no-std`, `#[no_mangle]`, `#[link_section]`, QEMU cross-run |
| Safety-critical | `#[asil_d]` / `#[do178c_level_a]` / `#[no_nan]` / `#[wcet]` + SMT proofs |
| Concurrent systems | Affine tasks, `parallel for`, `Mutex<T>`, `Channel<T>`, `RwLock<T>` |
| Formal verification | `requires` / `ensures` / `prove` / loop `invariant` backed by Z3 |
| Multilingual | 62 dialects across 26 scripts via `// vani-lang: hindi` (see [Language Coverage](docs/languages.md)) |
| Package management | `vanic add / vendor / publish / search`; live [Kosh registry](https://enthusiasticgeek.github.io/kosh-index/) |

### 1. Systems Programming & OS Kernels
Affine ownership eliminates double-free and use-after-free at compile time. Direct LLVM / C codegen produces deterministic, GC-free output suitable for kernel modules, bootloaders, and embedded operating systems. The Arc 8 async state machine compiles to a zero-allocation poll loop compatible with bare-metal schedulers.

### 2. Embedded & Bare-Metal Systems
The four-layer unsafe model (L1–L4) plus `requires` / `ensures` annotations maps directly onto MISRA C 2012, ISO 26262 ASIL-D, DO-178C (DAL A), and IEC 62304 Class C requirements. C backend output is readable, portable, and linkable against an existing embedded BSP without a Rust toolchain on the target.

### 3. Formal Verification & Proof-Assisted Programming
Z3 SMT integration discharges `requires`, `ensures`, `prove`, and loop `invariant` clauses at compile time. The three-stage pipeline (constant-fold → structural tautology → full Z3 solve) makes most arithmetic contracts free; Z3 is only invoked when the simpler passes cannot decide. Suitable for financial arithmetic, cryptographic protocol correctness, and safety-interlock logic.

### 4. Concurrent & Parallel Systems
The effects checker statically verifies race-freedom in `parallel for` reductions and rejects impure closures at the call site. Task handles are affine — forgetting to `join` a task is a compile error, not a runtime thread leak. Mutex / Guard RAII and Channel queues cover the classic producer-consumer patterns.

### 5. Networking & I/O-Bound Services
Arc 8 async/await compiles to cooperative state machines with epoll (Linux), kqueue (macOS), and IOCP (Windows) backends. CancelToken auto-plumbing provides graceful shutdown without manual flag threading. TCP echo, connection-pool, and multi-client examples ship in `examples/language/english/`.

### 6. High-Performance Data Processing
Monomorphized generics, a hand-rolled standard library (Vec, HashMap, BTreeMap, BinaryHeap, Graph, SkipList, Union-Find), and verified parallel reductions (+, *, min, max, &&, ||) enable pipeline-style batch processing with no GC pause jitter.

### 7. Real-Time & Safety-Critical Control Systems
Deterministic drop order, no allocator surprises, and SMT-verified loop bounds make vāṇī suitable for PLC-like control loops, motor controllers, and avionics flight software where timing jitter and memory corruption are unacceptable.

### 8. Multilingual & Localised Software
62 dialects across 26 scripts (Devanagari, Bengali, Tamil, Arabic, Japanese, Mandarin, and more) let teams write source that reads in their native language. Per-file dialect purity rejects out-of-language identifiers, keeping a codebase coherent across multilingual contributors. Particularly suited to educational software and government/public-sector tooling targeting the Indian subcontinent.

### 9. FFI & C Interoperability
Full SysV ABI / Win64 / AArch64 struct-return lowering lets vāṇī modules be called from C or call into libc, OpenSSL, SQLite, and similar libraries. `extern "C"` declarations and `--link-with` handle the link step; the C backend produces `.c` suitable for integration into legacy build systems without LLVM on the host.

### 10. Data Structures & Algorithm Libraries
The standard library ships affine-first containers across four complexity tiers. All containers are composable (`Vec<Box<dyn Iface>>`, `HashMap<OwnedStr, Vec<T>>`, etc.) and drop correctly under the affine ownership model. Suitable for competitive programming scaffolds, reference implementations, and algorithm correctness benchmarks backed by Z3 proofs.

---

## Key docs

| Document | Contents |
|----------|---------|
| [Tutorial Book](https://enthusiasticgeek.github.io/vani-compiler/) | 84 lessons: Beginner (25) → Intermediate (37) → Advanced (22) |
| [Language Manual](docs/language_manual.md) | Types, ownership, control flow, SIMD, FFI, tooling |
| [Language Coverage](docs/languages.md) | All 62 human-language dialects + verification status |
| [Design Philosophy](docs/philosophy.md) | Why the design is the way it is; comparisons with Rust/C++ |
| [Safety Standards](tutorials/src/advanced/12_safety_standards.md) | ASIL-D, DO-178C, IEC 61508, MISRA C 2012 compliance |
| [Benchmarks](benchmarks/README.md) | 12 benchmarks vs C / C++ / Rust — catalogue, methodology, open gaps |
| [Benchmark Results](benchmarks/results/RESULTS.md) | Latest timing results (auto-generated by `run_benchmarks.py`) |
| [Known Limitations](docs/v1_limitations.md) | Every known v1 deviation, workarounds, fix-queue pointers |
| [Kosh Package Manager](docs/kosh_design.md) | `vanic add / vendor / publish / search`; live registry |
| [INSTALL.md](INSTALL.md) | Per-platform setup (Linux / macOS / Windows) |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Pre-PR checklist, code conventions, commit style |
| [CHANGELOG.md](CHANGELOG.md) | Release history |

---

## Contributing

Open an issue or PR. The lexer keyword table is one file — adding or correcting a human-language dialect is a mechanical 6-touchpoint change. Native-speaker corrections for any of the 62 shipped dialects are especially welcome; see [Language Coverage](docs/languages.md) for per-dialect verification status.

[CONTRIBUTING.md](CONTRIBUTING.md) · [ONBOARDING.md](ONBOARDING.md) · [STATUS.md](STATUS.md)

---

## License & Trademark

Released under the [MIT License](LICENSE).

### Trademark

The project name **VANI** (वाणी, *vāṇī*) and the tagline *"code like you speak"* are unregistered common-law marks of The VANI Authors. You may use them to refer to the project ("compatible with VANI", "implementation of VANI") and in good-faith forks. Please don't use them in a way that implies endorsement by the project, or as your own product brand. If in doubt, ask in an issue.

**Third-party marks.** Names such as *Rust*, *C*, *C++*, *LLVM*, *Linux*, *Windows*, *macOS*, *Z3*, *Python*, *Sanskrit*, *Hindi*, *Marathi*, and any others used in comparison or discussion are the marks of their respective owners. References here are descriptive (nominative fair use) and do not imply affiliation, sponsorship, or endorsement. Playful coinages like *"C-scenic"* and *"Rust-ic"* are English wordplay, not adoption of any third-party mark.
