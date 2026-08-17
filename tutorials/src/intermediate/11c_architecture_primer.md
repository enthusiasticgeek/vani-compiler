# Architectural patterns in vāṇी -- primer

> **Read this after [SOLID](11b_solid_primer.md) and before the
> [GoF tutorial](11_design_patterns.md).** SOLID covers individual
> functions and interfaces; this page scales those ideas up to
> whole-program structure. GoF then fills in named solutions for
> the recurring sub-problems you'll meet inside each layer.

Design patterns (GoF) operate at the class/function level. Architectural
patterns operate at the module/program level -- they answer "how do the
major parts of the program connect?" Two patterns are especially natural
in vāṇī: **Hexagonal** and **Pipeline**.

---

## Pattern 1: Hexagonal (Ports and Adapters)

> *Keep core logic free of I/O. Let adapters plug in at the boundary.*

Also called "Clean Architecture" or "Onion Architecture" in other
communities. The idea is simple:

```
          ┌─────────────────────────┐
          │       Core logic        │  pure functions, no I/O
          │  (structs + interfaces) │
          └────────┬────────────────┘
                   │  depends on abstractions (interfaces)
          ┌────────▼────────────────┐
          │       Adapters          │  implement those interfaces
          │  file I/O, FFI, network │  for each concrete I/O source
          └─────────────────────────┘
```

The core knows only its own interfaces (ports). Adapters implement
those interfaces for specific I/O sources. Swapping an adapter --
say, from a real database to an in-memory stub for testing -- requires
zero changes to the core.

### vāṇी example: a log-analysis tool

**Step 1 -- define the ports (interfaces)**

```
// Port: something that supplies log lines
interface LineSource {
  fn next_line(self: Self) -> Str;
  fn has_more(self: Self)  -> i64;
}

// Port: something that accepts results
interface ResultSink {
  fn emit(self: Self, line: Str) -> i64;
}
```

**Step 2 -- write the core in terms of ports only**

```
fn find_errors<S: LineSource, R: ResultSink>(
  src: S, sink: R, keyword: Str
) -> i64 {
  let count: i64 = 0;
  while src.has_more() == 1 {
    let line: Str = src.next_line();
    if str_contains(line, keyword) == 1 {
      let _ = sink.emit(line);
      count = count + 1;
    }
  }
  return count;
}
```

The core function has no `extern`, no `FileHandle`, no `print` block.
It is a pure transformation of `LineSource → ResultSink`.

**Step 3 -- write adapters for each concrete I/O source**

```
// Adapter A: reads from a Vec<Str> (useful for tests)
struct VecSource { lines: Vec<Str>, pos: i64 }
implement LineSource for VecSource {
  fn next_line(self: VecSource) -> Str {
    let line: Str = vec_get(ref self.lines, self.pos);
    self.pos = self.pos + 1;
    return line;
  }
  fn has_more(self: VecSource) -> i64 {
    return if self.pos < length(ref self.lines) { 1 } else { 0 };
  }
}

// Adapter B: prints to stdout
struct StdoutSink {}
implement ResultSink for StdoutSink {
  fn emit(self: StdoutSink, line: Str) -> i64 {
    print { line }
    return 0;
  }
}

// Adapter C: collects into a Vec (useful for tests)
struct VecSink { results: Vec<OwnedStr> }
implement ResultSink for VecSink {
  fn emit(self: VecSink, line: Str) -> i64 {
    vec_push(ref self.results, line + "");
    return 0;
  }
}
```

**Step 4 -- wire it up in `main`**

```
fn main() -> i64 {
  let logs: Vec<Str> = vec("INFO start", "ERROR disk full", "INFO stop");
  let src: VecSource  = VecSource  { lines: logs, pos: 0 };
  let sink: StdoutSink = StdoutSink {};
  let n: i64 = find_errors(src, sink, "ERROR");
  return n;   // 1
}
```

To test with an in-memory sink: swap `StdoutSink` for `VecSink` --
no other change needed. The core `find_errors` function is untouched
and can be proved correct with `requires` / `ensures` independently
of any I/O adapter.

### When to reach for Hexagonal

- The program has multiple I/O backends (file, network, in-memory test)
- You want to SMT-verify the core logic independently of I/O
- The program will outlive its first I/O technology (e.g. swap SQLite → Postgres)

---

## Pattern 2: Pipeline

> *Transform data through a sequence of independent stages.*

A pipeline is a chain of functions where the output of one stage is
the input of the next. Each stage does one job (SRP), knows nothing
about the stages around it (DIP), and can be tested in isolation.

```
  raw input
      │
      ▼
  ┌───────┐     ┌───────┐     ┌───────┐     ┌───────┐
  │ Stage │────▶│ Stage │────▶│ Stage │────▶│ Stage │──▶ output
  │   1   │     │   2   │     │   3   │     │   4   │
  └───────┘     └───────┘     └───────┘     └───────┘
  tokenise      parse         analyse       emit
```

vāṇī itself is structured this way: source text → tokens → AST →
typed IR → LLVM IR / C.

### vāṇी example: a CSV processing pipeline

```
struct RawRow   { text: Str }
struct ParsedRow { cols: Vec<OwnedStr> }
struct ScoredRow { cols: Vec<OwnedStr>, score: i64 }

// Stage 1: split on comma
fn parse_row(raw: RawRow) -> ParsedRow {
  // simplified: split on first comma only
  let pos: i64 = 0;
  let s: Str = raw.text;
  // ... real split logic omitted for brevity ...
  let cols: Vec<OwnedStr> = vec("col1" + "", "col2" + "");
  return ParsedRow { cols };
}

// Stage 2: score rows by length of first column
fn score_row(parsed: ParsedRow) -> ScoredRow {
  let first: Str = vec_get(ref parsed.cols, 0);
  let score: i64 = length_str(first);
  return ScoredRow { cols: parsed.cols, score };
}

// Stage 3: filter rows below threshold
fn filter_row(scored: ScoredRow, min_score: i64) -> i64 {
  return if scored.score >= min_score { 1 } else { 0 };
}

fn main() -> i64 {
  let raw: Vec<RawRow> = vec(
    RawRow { text: "alice,30" },
    RawRow { text: "bo,25" },
  );
  let kept: i64 = 0;
  let i: i64 = 0;
  while i < length(ref raw) {
    let r = vec_get(ref raw, i);
    let parsed  = parse_row(r);
    let scored  = score_row(parsed);
    if filter_row(scored, 4) == 1 { kept = kept + 1; }
    i = i + 1;
  }
  return kept;   // 1 (only "alice" passes min_score=4)
}
```

Each stage is a plain function with a concrete input and output type.
You can test `parse_row` without `score_row` or `filter_row` in scope.

### Parallel pipeline

When stages are independent and the data set is large, fan out with
`parallel for`:

```
fn run_parallel(rows: ref Vec<RawRow>) -> i64 {
  let scores: Vec<i64> = vec_zeros(length(ref rows));
  parallel for i in 0..length(ref rows) {
    let r      = vec_get(ref rows, i);
    let parsed = parse_row(r);
    let scored = score_row(parsed);
    vec_set(ref scores, i, scored.score);
  }
  return 0;
}
```

The stages are pure functions -- no shared mutable state -- so the
race-freedom checker accepts this without any explicit locks.

### When to reach for Pipeline

- Data flows in one direction with no backtracking
- Each stage is independently testable
- The data set is large enough to benefit from `parallel for`
- You need to insert, remove, or reorder stages without touching others

---

## Combining the two

Hexagonal and Pipeline compose naturally: the core of a Hexagonal
architecture is often itself a Pipeline.

```
  LineSource (port)
       │
       ▼
  ┌────────────┐
  │  tokenise  │  stage 1
  └─────┬──────┘
        │
  ┌─────▼──────┐
  │   filter   │  stage 2
  └─────┬──────┘
        │
  ┌─────▼──────┐
  │   format   │  stage 3
  └─────┬──────┘
        │
  ResultSink (port)
```

The pipeline stages are pure functions (testable in isolation). The
ports are interfaces (swappable adapters). The combination gives you
a program that is correct by construction at the core, flexible at
the boundaries, and testable at every level.

---

## Quick reference

| Pattern | Core idea | vāṇī mechanism | Best for |
|---|---|---|---|
| **Hexagonal** | Core logic depends only on interface ports; adapters plug in at the boundary | `interface` + `implement` + `<T: Port>` generics | Programs with multiple I/O backends, SMT-verifiable cores |
| **Pipeline** | Data flows through a chain of single-purpose stage functions | Plain functions + `parallel for` for fan-out | Compilers, ETL, signal processing, data transformation |

---

**Previous**: [SOLID design principles](11b_solid_primer.md)  
**Next**: [The 22 GoF design patterns ->](11_design_patterns.md)
