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
  requires a >= 0 && b >= 0
  ensures  result == a + b
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

---

## Install

```bash
cargo install vanic          # from crates.io
vanic build hello.vani -o hello
./hello
```

See [INSTALL.md](INSTALL.md) for platform-specific prerequisites (z3, LLVM tools).

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
