# vāṇī Language Manual

> **Quick navigation** — use your browser's in-page search (`Ctrl-F` / `Cmd-F`).
> The [Tutorial Book](https://enthusiasticgeek.github.io/vani-compiler/) covers the
> same material with worked examples and exercises.

---

## Contents

1. [Syntax conventions](#syntax-conventions)
2. [Multilingual keywords](#multilingual-keywords)
3. [Types](#types)
4. [Ownership and references](#ownership-and-references)
5. [Control flow](#control-flow)
6. [Functions and closures](#functions-and-closures)
7. [Structs, enums, and generics](#structs-enums-and-generics)
8. [Collections](#collections)
9. [Strings](#strings)
10. [Error handling](#error-handling)
11. [Modules and visibility](#modules-and-visibility)
12. [Concurrency](#concurrency)
13. [Async / await](#async--await)
14. [SMT verification](#smt-verification)
15. [Safety attributes](#safety-attributes)
16. [SIMD and vectorization](#simd-and-vectorization)
17. [File I/O](#file-io)
18. [Bare-metal and cross-compilation](#bare-metal-and-cross-compilation)
19. [Tooling reference](#tooling-reference)
20. [FFI and linking](#ffi-and-linking)
21. [Glossary](#glossary)

---

## Syntax conventions

vāṇī accepts multiple spellings for most constructs. All aliases resolve to the
same AST node — generated code is identical regardless of which form you use.
The compiler ships 62 dialects across 26 scripts (see §[Multilingual keywords](#multilingual-keywords)).

| Canonical | Accepted aliases |
|-----------|-----------------|
| `let`     | `assign` |
| `return`  | `give`, `give_back`, `give back` |
| `pub`     | `public` |
| `module`  | `mod` |
| `fn`      | (no alias; keyword-first) |

Comments: `//` single-line, `/* … */` block (nesting supported).

Human-language aliases are enabled per file with `// vani-lang: hindi` (or any
supported dialect). See [Language Coverage](languages.md).

---

## Multilingual keywords

vāṇī ships 62 dialects across 26 scripts. The tables below show every
structural keyword in **English → Sanskrit → Hindi → Marathi**; the same
pattern extends to all other dialects (see [Language Coverage](languages.md)).

Enable dialect purity per file:

```vani
// vani-lang: sanskrit

कार्य योग(क: i64, ख: i64) -> i64 { पुनरागम क + ख; }
```

### Declarations and visibility

| English | Sanskrit (*saṁskṛta*) | Hindi (*hindī*) | Marathi (*marāṭhī*) |
|---------|----------------------|-----------------|---------------------|
| `fn` | `कार्य` *kārya* | `फलन` *phalan* | `कार्य` *kārya* |
| `let` / `assign` | `माना` *mānā* | `माना` *mānā* | `मान` *māna* |
| `struct` | `संरचना` *saṁracanā* | `संरचना` *saṁracanā* | `संरचना` *saṁracanā* |
| `enum` | `विकल्प` *vikalpa* | `गणन` *gaṇan* | `गणन` *gaṇan* |
| `const` | `स्थिर` *sthira* | `स्थिर` *sthira* | `स्थिर` *sthira* |
| `pub` / `public` | `सार्वजनिक` *sārvajanik* | `सार्वजनिक` *sārvajanik* | `सार्वजनिक` *sārvajanik* |
| `module` / `mod` | `खण्ड` *khaṇḍa* | `मॉड्यूल` *mōḍyūla* | `मॉड्यूल` *mōḍyūla* |
| `use` | `उपयोग` *upayog* | `उपयोग` *upayog* | `उपयोग` *upayog* |
| `extern` | `बाह्य` *bāhya* | `बाह्य` *bāhya* | `बाह्य` *bāhya* |
| `interface` / `trait` | `संकेत` *saṅket* | `संकेत` *saṅket* | `संकेत` *saṅket* |
| `implement` / `impl` | `कार्यान्वित` *kāryānvit* | `कार्यान्वित` *kāryānvit* | `कार्यान्वित` *kāryānvit* |
| `where` | `यत्र` *yatra* | `जहाँ` *jahāṃ* | `जिथे` *jithe* |
| `is` | `अस्ति` *asti* | `है` *hai* | `आहे` *āhe* |

### Control flow

| English | Sanskrit | Hindi | Marathi |
|---------|----------|-------|---------|
| `return` / `give` | `पुनरागम` *punarāgama* | `लौटाओ` *lauṭāo* | `परत` *parat* |
| `if` | `यदि` *yadi* | `अगर` *agar* | `जर` *jar* |
| `else` | `अन्यथा` *anyathā* | `नहीं तो` *nahīṁ to* | `नाहीतर` *nāhītar* |
| `while` | `यावत्` *yāvat* | `जबतक` *jab tak* | `जोपर्यंत` *jopar­yaṃta* |
| `for` | `प्रति` *prati* | `के लिए` *ke liye* | `साठी` *sāṭhī* |
| `from` | `से` *se* | `से` *se* | `से` *se* |
| `to` | `तक` *tak* | `तक` *tak* | `तक` *tak* |
| `break` | `विराम` *virāma* | `रुको` *ruko* | `थांब` *thāmba* |
| `continue` | `अग्रे` *agre* | `आगे` *āge* | `पुढे` *puḍhe* |
| `match` | `मेल` *mela* | `मिलान` *milān* | `जुळवा` *juḷvā* |
| `then` | `तदा` *tadā* | `तो` *to* | `तर` *tar* |

### References and mutation

| English | Sanskrit | Hindi | Marathi |
|---------|----------|-------|---------|
| `ref` | `दृष्ट्या` *dṛṣṭyā* | `देखो` *dekho* | `पहा` *pahā* |
| `mut` | `परिवर्तनीय` *parivartanīya* | `परिवर्तनीय` *parivartanīya* | `बदल` *badla* |

### Verification

| English | Sanskrit | Hindi | Marathi |
|---------|----------|-------|---------|
| `requires` | `अपेक्षित` *apekṣita* | `चाहिए` *cāhiye* | `पाहिजे` *pāhije* |
| `ensures` | `सुनिश्चयित` *sunishcayita* | `निश्चित` *nishcit* | `निश्चित` *nishcit* |
| `prove` | `प्रमाण` *pramāṇa* | `सिद्ध करो` *siddha karo* | `सिद्ध करा` *siddha karā* |
| `assert` | `सिद्धम्` *siddham* | `सुनिश्चित` *sunishcit* | `खात्री` *khātrī* |
| `invariant` | `अपरिवर्तनीय` *aparivartanīya* | `अपरिवर्तनीय` | `अपरिवर्तनीय` |

### I/O

| English | Sanskrit | Hindi | Marathi |
|---------|----------|-------|---------|
| `print` | `लिख` *likha* | `लिखो` *likho* | `लिहा` *lihā* |
| `eprint` | `त्रुटिलिख` *truṭilikha* | `त्रुटिलिखो` *truṭilikho* | `दोषलिहा` *doṣalihā* |

`eprint`'s dialect coverage shipped 2026-08-10 (BUG-166) -- until
then only the bare English spelling existed anywhere in the lexer.

### Concurrency

| English | Sanskrit | Hindi | Marathi |
|---------|----------|-------|---------|
| `parallel` | `समानांतर` *samānāntara* | `समानांतर` *samānāntara* | `समानांतर` *samānāntara* |
| `reduce` | `संक्षेप` *saṁkṣepa* | `संक्षेप` *saṁkṣepa* | `संक्षेप` *saṁkṣepa* |
| `task` | `नियोग` *niyog* | `नियोग` *niyog* | `नियोग` *niyog* |
| `join` | `संयोजन` *saṁyojan* | `संयोजन` *saṁyojan* | `संयोजन` *saṁyojan* |

### Devanagari type names

| English type | Devanagari | Romanization |
|-------------|-----------|--------------|
| `i64` | `पूर्णांक` | *pūrṇāṃka* |
| `f64` | `दशांश` | *daśāṃśa* |
| `bool` | `तर्क` | *tarka* |
| `Vec` | `सूची` | *sūcī* |
| `i8`/`i16`/`i32` | `पूर्णांक८`/`१६`/`३२` | width-explicit |
| `u8`…`u64` | `अहस्ताक्षरित८`…`६४` | unsigned |

Integer and float literals accept Devanagari digits `०–९` (U+0966–U+096F).
`५ * २` parses as `5 * 2`; `३.१४` as the f64 `3.14`.

### SOV (verb-final) statement shapes

For Sanskrit / Hindi / Marathi, the parser accepts **verb-at-end order**
alongside keyword-first:

| Construct | Keyword-first | SOV form |
|-----------|--------------|----------|
| `let` | `माना x: i64 = 5;` | `x: i64 = 5 माना;` |
| `return` | `पुनरागम x;` | `x पुनरागम;` |
| `print` | `लिख x;` | `x लिख;` |
| `prove` | `प्रमाण expr;` | `expr प्रमाण;` |
| range `for` | — | `i प्रति 0 से 3 तक { … }` |

Top-level `fn` / `struct` / `enum` declarations remain keyword-first (SOV
there would feel forced). `match` SOV is available inside SOV-let.

---

## Types

### Scalar types

| Type | Width | Notes |
|------|-------|-------|
| `i8` `i16` `i32` `i64` | 8–64 bit signed | `i64` is the default integer |
| `u8` `u16` `u32` `u64` | 8–64 bit unsigned | |
| `f32` `f64` | 32/64-bit float | |
| `bool` | 1 bit logical | `true` / `false` |

### Compound types

| Type | Syntax | Notes |
|------|--------|-------|
| Fixed array | `[T; N]` | stack-allocated, Copy |
| Vector | `Vec<T>` | heap-allocated, affine |
| SIMD 128-bit | `vec128<T>` | 4×f32, 2×f64, 16×i8, … |
| SIMD 256-bit | `vec256<T>` | 8×f32, 4×f64, … (AVX2/SVE/RVV) |
| Tuple | `(T, U)` | positional fields |
| String (borrowed) | `Str` | `&str` equivalent; `len()` |
| String (owned) | `OwnedStr` | heap, affine |
| Optional | `Opt<T>` | `Opt.Some(x)` / `Opt.None` |
| Result | `Result<T, E>` | `Result.Ok(x)` / `Result.Err(e)` |
| Box | `Box<T>` | heap pointer, affine |
| Interface object | `dyn Iface` | fat pointer, heap |

### Casts

```vani
let x: i64 = 3;
let y: f64 = x as f64;
let z: i32 = y as i32;   // truncates
```

---

## Ownership and references

vāṇī uses **affine ownership** — a value is consumed at most once.

```vani
let a: Vec<i64> = vec(1, 2, 3);
let b: Vec<i64> = a;          // a is moved; a is no longer accessible
```

**References** are second-class (cannot be stored past the call).

```vani
fn sum(xs: ref Vec<i64>) -> i64 { … }   // shared borrow
fn fill(xs: mut ref Vec<i64>) { … }     // mutable borrow

let xs: Vec<i64> = vec(1, 2, 3);
let s: i64 = sum(ref xs);
```

**Smart pointers**

| Type | Semantics |
|------|-----------|
| `Box<T>` | unique heap pointer; drops on scope exit |
| `Arc<T>` | shared heap pointer; ref-counted |
| `Mutex<T>` / `Guard<T>` | mutual exclusion; RAII unlock |
| `RwLock<T>` / `ReadGuard<T>` / `WriteGuard<T>` | reader-writer lock |

---

## Control flow

```vani
// if / else
if x > 0 { … } else { … }

// while
while cond { … }

// for over range -- step 1, half-open. `to` ascends [0, n);
// `downto` descends and excludes its lower bound the same way
// (English dialect only for now). No `step`/stride-N; use `while`.
for i from 0 to n { … }
for i from n downto 0 { … }

// for over collection
for x in ref xs { … }

// named labels
outer: for i from 0 to n {
    inner: while true {
        break outer;
    }
}

// match
match opt {
    Opt.Some(v) then { … }
    Opt.None    then { … }
}
```

---

## Functions and closures

```vani
fn add(a: i64, b: i64) -> i64 { return a + b; }

// function pointer
let f: fn(i64, i64) -> i64 = add;

// closure (captures by ref or move)
let mul: fn(i64) -> i64 = fn(x: i64) -> i64 { return x * 2; };

// higher-order
let doubled: Vec<i64> = vec_map(ref xs, fn(x: i64) -> i64 { return x * 2; });
```

---

## Structs, enums, and generics

```vani
struct Point { x: f64, y: f64 }

enum Shape {
    Circle(f64),
    Rect(f64, f64),
}

struct Pair<T, U> { first: T, second: U }

interface Drawable {
    fn draw(self: ref Self) -> i64;
}

implement Drawable for Point {
    fn draw(self: ref Self) -> i64 { … }
}
```

---

## Collections

| Builtin | Signature | Notes |
|---------|-----------|-------|
| `vec(a, b, …)` | `-> Vec<T>` | literal |
| `vec_fill(n, val)` | `-> Vec<T>` | bulk init |
| `vec_with_capacity(n)` | `-> Vec<T>` | pre-alloc |
| `push(mut ref xs, val)` | `-> ()` | append (bare builtin, not `vec_push`) |
| `len(xs)` | `-> u64` | length; also callable as `xs.len()` method-call sugar, same builtin either way |
| `vec_sum/min/max/mean` | `-> T` | `Vec<i64>` and `Vec<f64>` |
| `vec_fold/map/filter` | HOF | `Vec<i64>` and `Vec<f64>` |
| `vec_dot(ref a, ref b)` | `-> T` | `Vec<i64>→i64`, `Vec<f64>→f64` |
| `sort(mut ref xs)` | in-place | `Vec<i64>` and `Vec<f64>` |
| `vec_kth_smallest(ref xs, k)` | `-> T` | returns -1 / qNaN on OOB |
| `HashMap<K, V>` | | `hashmap_new/insert/get/contains_key/remove` |
| `HashSet<T>` | | `hashset_new/insert/contains/remove` |
| `BTreeMap<K, V>` | | sorted map; `btreemap_range_keys/values` |
| `BTreeSet<T>` | | sorted set; range queries |
| `BinaryHeap<T>` | | `binary_heap_new/push/pop/peek`; max-heap. (`heap_push`/`heap_pop`/`heap_peek` are a separate, older API operating on a raw `Vec<i64>`, not this wrapper type — don't mix the two families.) |
| `Deque<T>` | | ring buffer; `deque_push_front/push_back/pop_front/pop_back` |
| `Graph` | | weighted directed; BFS/DFS/Dijkstra/A*/topo/Kruskal/Prim |
| `Bst<T>` | | AVL self-balancing BST; `bst_insert/remove/contains` |
| `SkipList<T>` | | probabilistic sorted list; `skiplist_insert/remove/max` |
| `UnionFind` | | path-compression + union-by-rank; `union_find_new/union/find` |
| `BloomFilter` | | probabilistic membership; `bloom_filter_insert/contains` |

---

## Strings

```vani
let s: Str = "hello";
let n: i64  = str_len(s);
let t: OwnedStr = str_concat(s, " world");
let i: i64  = str_find(s, "ell");   // -1 if absent
```

String literals support these backslash escapes:

| Escape | Meaning |
|--------|---------|
| `\"` | double quote |
| `\\` | backslash |
| `\n` / `\t` / `\r` | newline / tab / carriage return |
| `\0` | NUL byte |
| `\xHH` | a byte by its 2-digit hex value, `00`-`7f` only (ASCII range -- `Str` is UTF-8 text with no separate byte-string form, so higher values are a compile error). `\x1b` is the ANSI ESC byte, useful for terminal color codes: `"\x1b[31mred\x1b[0m"`. |

A string literal cannot span multiple lines.

---

## Error handling

```vani
fn parse(s: Str) -> Result<i64, Str> { … }

// try propagates Err upward
let n: i64 = try parse("42");

// explicit match
match parse("x") {
    Result.Ok(v)  then { print v; }
    Result.Err(e) then { eprint e; }
}
```

---

## Modules and visibility

```vani
module math {
    pub fn sqrt(x: f64) -> f64 { … }          // public API
    pub(kosh) fn helper() -> f64 { … }        // package-internal only
    fn internal() -> f64 { … }                // private to module
}

use math::sqrt;
let r: f64 = sqrt(2.0);
```

`kosh` (कोश) is Sanskrit for "repository" — `pub(kosh)` is Rust's
`pub(crate)` equivalent. **Enforced for external Kosh-package access**
(as of 2026-07-22): a `pub(kosh)` item called via `pkgname::item` from a
*different* project consuming it as a `[deps]` dependency is rejected —
verified directly. **Not yet enforced for same-project sibling modules**:
one module reaching into a different module's `pub(kosh)` item within the
*same* compile is currently *also* rejected (stricter than the intended
"visible within your whole project" design — real caller-identity
tracking to distinguish that case from a genuinely external consumer is
still open, see `docs/v1_limitations.md` L23). For now, prefer plain
`pub` for cross-module sharing within one project; reach for `pub(kosh)`
specifically to guard a Kosh package's own internal helpers from external
consumers.

### Kosh package dependencies use this same mechanism automatically

Every `[deps]` entry in `vani.toml` is compiled inside an implicit
`module <pkg_name> { ... }` — no `module` keyword needed in the
dependency's own source. Its functions (and any exported struct types)
are called as `pkgname::item`, exactly like an in-file module:

```vani
// vani.toml: [deps] matrix = { path = "./vendor/matrix" }
let y: Vec<f64> = matrix::mat_solve(ref a, ref b, n);
```

This is what makes two unrelated packages — or a package and a vāṇी
builtin — safe to share a function name: they live in different
namespaces and can never collide. See
[Kosh package namespacing design](kosh_namespacing_design.md) and
[the Kosh Packages tutorial](../tutorials/src/intermediate/16_packages.md)
for the full design and worked examples.

---

## Concurrency

```vani
// parallel for with reduction
let sum: i64 = 0;
parallel for i from 0 to n
reduce sum with +;
{
    sum = sum + xs[i];
}

// task (affine handle — forgetting to join is a compile error)
let t: Task<i64> = task expensive_work();
let result: i64 = join t;   // blocks until the task exits

// mutex
let m: Mutex<i64> = mutex_new(0);
{
  let g: Guard<i64> = mutex_lock(ref m);
  guard_set(mut ref g, guard_get(ref g) + 1);
}   // Guard drops here → unlock

// channel — bounded MPMC queue
let ch: Channel<i64, 16> = channel_new();
let _ = channel_send(ref ch, 42);   // blocks if full
let v: i64 = channel_recv(ref ch);  // blocks if empty
```

| Primitive | Use when |
|-----------|----------|
| `Atomic<T>` | Simple counter / flag with no-lock guarantee |
| `Mutex<T>` / `Guard<T>` | Guarded mutation of a value |
| `RwLock<T>` / `ReadGuard<T>` / `WriteGuard<T>` | Read-heavy shared state |
| `Channel<T, N>` | Producer-consumer queue; moves ownership across threads |
| `Condvar` | Wait until a non-trivial predicate is true |
| `Barrier` | All N threads reach a checkpoint before proceeding |

---

## Async / await

```vani
async fn fetch(url: Str) -> Result<OwnedStr, Str> { … }

async fn main_async() -> i64 {
    let body: OwnedStr = try await fetch("http://example.com");
    print body;
    return 0;
}
```

Compiles to a cooperative state machine. Backends: epoll (Linux),
kqueue (macOS), IOCP (Windows).

---

## SMT verification

```vani
fn divide(a: i64, b: i64) -> i64
  requires b != 0;
{
  return a / b;
}

fn count_to(n: i64) -> i64
requires n >= 0;
requires n < 1000;
ensures _return >= 0;
{
  let i: i64 = 0;
  while i < n
  invariant i >= 0;
  invariant i <= n;
  {
    i = i + 1;
  }
  prove i >= 0;
  return i;
}
```

`requires`/`ensures` clauses need a trailing `;` (they're statements,
not a header block); `_return` — not `result` — is the only valid
name for "the value being returned" inside an `ensures`/`prove`
clause; bounded loops use `for i from START to END { … }`, and
unbounded ones use `while COND invariant …; { … }` as shown above.

Backed by Z3. Three-stage pipeline: constant-fold → structural tautology →
full SMT solve. `--no-verify` skips SMT for fast iteration.

**Recursion and reentrancy**: every call — including a function calling
itself, mutual recursion, or a call back into a function currently being
checked — is verified against the callee's `requires`/`ensures`
*signature* only; the checker never re-descends into a callee's body (not
even its own). This means a self-recursive call is proven exactly like any
other call: the caller's `requires` is discharged against the callee's
declared precondition, and the callee's `ensures` is assumed as a fact for
the result. No separate recursion-handling code path or depth tracking
exists, and none is needed — the checker's own control flow never
recurses across the call graph, so mutual/self recursion cannot make it
loop. Practical effect: an `ensures` clause on a recursive function acts
as an induction hypothesis (the recursive call's `ensures` is assumed
while proving the current call's `ensures`), so recursive functions need
a tight enough `ensures` to make that induction step provable — see
[Sec.12 SMT deep-dive](../tutorials/src/intermediate/12_smt_deepdive.md#recursive-and-reentrant-calls)
for a worked example. `#[no_recursion]` (below) rejects recursion
outright instead of verifying it, via an unrelated call-graph cycle
check.

**Complexity**: fact generation is a single linear walk per function
over its own AST; nothing re-walks a callee's body, so cost doesn't
scale with recursion depth. Each proof obligation issues one Z3 query
capped at a 5s timeout, with queries cached by exact text to skip
repeats.

---

## Safety attributes

| Attribute | Effect |
|-----------|--------|
| `#[no_heap]` | Rejects any heap allocation transitively |
| `#[no_float]` | Rejects any floating-point operation |
| `#[no_nan]` | Rejects builtins with NaN-as-error-sentinel (`f64_nan`, `vec_kth_smallest<f64>`) |
| `#[no_recursion]` | Rejects recursive calls |
| `#[wcet(cycles=N)]` | Enforces worst-case execution time bound |
| `#[bounded_stack(bytes=N)]` | Enforces stack frame bound |
| `#[deterministic_timing]` | Rejects timing-variant operations |
| `#[interrupt(priority=N)]` | ISR declaration; priority-inversion checked |
| `#[asil_d]` | ISO 26262 ASIL-D (implies no_heap + no_float + no_nan + no_recursion + wcet + deterministic_timing) |
| `#[do178c_level_a]` | DO-178C DAL A (same implications as asil_d) |
| `#[iec_61508_sil3]` / `#[sil4]` | IEC 61508 SIL-3/4 (implies no_nan) |
| `#[misra_c_2012]` | MISRA C 2012 rule set |

See [tutorials/src/advanced/12_safety_standards.md](../tutorials/src/advanced/12_safety_standards.md)
for the full compliance matrix.

---

## SIMD and vectorization

```vani
fn dot(a: ref Vec<f32>, b: ref Vec<f32>, n: i64) -> f32 {
    let acc: vec256<f32> = simd256_splat(0.0 as f32);
    let i: i64 = 0;
    while i + 8 <= n {
        acc = simd256_add(acc, simd256_mul(simd256_load(a, i),
                                           simd256_load(b, i)));
        i = i + 8;
    }
    return simd256_reduce_add(acc);
}
```

| Type | Lanes (f32) | x86-64 | AArch64 | RISC-V |
|------|------------|--------|---------|--------|
| `vec128<T>` | 4 | `xmm` (SSE) | NEON `v` | RVV VLEN=128 |
| `vec256<T>` | 8 | `ymm` (AVX2) | 2× NEON | RVV VLEN=256 |

`#[vectorize]` hints LLVM to auto-vectorize a loop.

---

## File I/O

```vani
let fh: FileHandle = file_open("data.txt", "r", true);
if file_is_ok(ref fh) {
    let line: OwnedStr = file_read_line(mut ref fh);
    file_write(mut ref fh, "written\n");
    file_close(fh);           // affine — compile error to use after close
}
let input: OwnedStr = stdin_read_line();
```

---

## Bare-metal and cross-compilation

```bash
# Cross-compile for ARM Cortex-M (no libc)
vanic build firmware.vani --target=thumbv7em-none-eabihf --no-std -o firmware.elf

# Cross-compile for AArch64 + QEMU test
vanic build prog.vani --target=aarch64-unknown-linux-gnu -o prog_arm
qemu-aarch64-static prog_arm
```

Attributes: `#[no_mangle]`, `#[link_section = ".vectors"]`.
MMIO: `mmio_read_u32(addr)` / `mmio_write_u32(addr, val)` (also u8/u16).

---

## Tooling reference

```bash
vanic build   prog.vani -o prog          # AOT native binary (LLVM → llc → cc)
vanic run     prog.vani                  # compile + run via lli
vanic check   prog.vani                  # type-check + SMT verify only
vanic check   prog.vani --no-verify      # skip SMT
vanic check   prog.vani --json           # JSON diagnostics
vanic emit    prog.vani                  # LLVM IR (default)
vanic emit    prog.vani --backend=c      # C output
vanic fmt     prog.vani                  # canonical formatting
vanic ast     prog.vani                  # parsed AST dump
vanic ir      prog.vani                  # typed IR dump
vanic tokens  prog.vani                  # token stream
vanic lsp                                # Language Server (stdio)
vanic coverage prog.vani                 # MC/DC coverage map
vanic safety-attrs prog.vani             # list active safety attributes

# Package manager (Kosh)
vanic add foo@^1.0                       # fetch from Kosh registry → vendor/foo/
vanic remove foo                         # remove dep + vendor dir
vanic vendor                             # copy all deps into vendor/
vanic search query                       # search registry by name
vanic update                             # re-resolve deps to latest compatible
vanic audit-safety prog.vani             # #[bounded_stack]/#[wcet] coverage where eligible
vanic publish                            # audit-safety gate + build tarball + create GitHub Release
```

**Editor integration:** Build `intent-lsp` (`cargo build --release --bin intent-lsp`)
and point your editor at the binary. Speaks LSP over stdio; supports hover types,
go-to-definition, find-references, rename, completion, and semantic highlighting.

Full per-editor setup (VS Code, Neovim/nvim-lspconfig, Emacs/eglot):
see [tutorials/src/installation.md — Editor integration (LSP)](../tutorials/src/installation.md#editor-integration-lsp).

---

## FFI and linking

```vani
extern "C" fn printf(fmt: Str, val: i64) -> i32;

fn main() -> i64 {
    printf("value = %ld\n", 42);
    return 0;
}
```

```bash
vanic build prog.vani --link-with libfoo.a -lfoo -o prog
```

The C backend (`--backend=c`) produces a `.c` file suitable for integration
into any existing build system without LLVM on the host.

---

## Glossary

| Term | Meaning |
|------|---------|
| **affine** | Used at most once; compile-time checked |
| **kosh** | Sanskrit for "repository"; used in `pub(kosh)` (≈ Rust `pub(crate)`) |
| **scrutinee** | The expression being matched in a `match` |
| **second-class reference** | A `ref` that cannot escape its scope; no lifetime annotations needed |
| **SMT** | Satisfiability Modulo Theories; Z3 is the solver |
| **vtable** | Fat pointer dispatch table for `dyn Iface` |
| **affine drop** | Deterministic destructor call at scope exit (no GC) |
| **MMIO** | Memory-mapped I/O; `mmio_read/write_u8/u16/u32` |
| **qNaN** | Quiet NaN; returned by `vec_kth_smallest<f64>` on out-of-bounds |
| **kosh** (package) | A single buildable project; defined by `vani.toml` |

Full glossary (60+ terms): see the previous [README.md history](https://github.com/enthusiasticgeek/vani-compiler/blob/v0.4.1/README.md#glossary).
