# Intermediate 2b — Match enhancements: `if let`, `while let`, or-patterns, guards, slice patterns

> **Learning goal**: use the four match-extension forms — `if let`, `while let`,
> or-patterns with `|`, and pattern guards with `if` — to write concise
> conditional and loop code without spelling out a full `match` every time.

> **Prerequisite**: [Intermediate 2 — Enums with payloads + match arms](02_enums_payloads.md)

---

## Why these forms exist

A full `match` expression handles every variant. That is the right tool
when you care about every case. But three everyday situations are more
concise with a shorter form:

| Situation | Without | With |
|-----------|---------|------|
| Act only when a variant matches, ignore otherwise | `match x { V(n) then { … }, _ then {} }` | `if let V(n) = x { … }` |
| Loop until a variant no longer matches | `loop { match pop() { V(n) then { … } _ then break } }` | `while let V(n) = pop() { … }` |
| Multiple variants share the same arm body | one arm per variant | `V1 \| V2 then { … }` |
| Refine a variant match by a runtime condition | two arms, same variant | `V(n) if n > 0 then { … }` |

All four forms lower to `match` internally — they are syntax sugar,
not new semantics.

---

## Shared setup

All examples below use this enum and a helper that returns `Option<i64>`:

```vani
intent "Match enhancements worked example.";

enum Status { Active, Pending, Done, Failed }
enum Opt    { None, Some(i64) }

fn try_parse(s: Str) -> Opt {
  // toy parser: returns Some(len) when s is non-empty
  let n: i64 = len(s) as i64;   // len() returns u64; cast to i64
  if n > 0 { return Opt.Some(n); }
  return Opt.None;
}
```

---

## 1. `if let` — act on one variant, skip everything else

```vani
fn show_length(s: Str) -> i64 {
  if let Opt.Some(n) = try_parse(s) {
    print "length:", n;
    return n;
  }
  // execution continues here when the variant does NOT match
  print "empty string";
  return 0;
}
```

`if let Opt.Some(n) = try_parse(s)` captures exactly this shape --
"run this block when the pattern matches, otherwise fall through":

```
match try_parse(s) {
  Opt.Some(n) then { print "length:", n; return n; },
  _           then {},
}
```

(Illustrative, not literal syntax to type yourself -- `match` is
expression-only in v1, so a bare `match` statement like this doesn't
parse on its own. This is the shape `if let` compiles down to
internally, in a position ordinary code can't reach directly.)

The bound variable `n` is in scope only inside the `{ … }` block. The
else path (variant did not match) falls through normally; you can add
an `else` block:

```vani
if let Opt.Some(n) = try_parse(s) {
  print "got", n;
} else {
  print "nothing";
}
```

---

## 2. `while let` — loop as long as a variant keeps matching

```vani
fn drain_all(inputs: ref Vec<Str>) -> i64 {
  let total: i64 = 0;
  let i: u64 = 0;
  while let Opt.Some(n) = try_parse(inputs[i]) {
    total = total + n;
    i = i + 1;
    if i >= len(inputs) { break; }
  }
  return total;
}
```

The loop body runs as long as `try_parse` returns `Opt.Some(n)`. The
first time it returns `Opt.None`, the loop exits. The binding `n` is
re-bound fresh on every iteration — it does not carry over.

**A place `while let` does *not* apply**: draining a `Vec` used as
a stack. It's tempting to reach for `while let Opt.Some(v) = pop(...)`,
but the built-in `pop(mut ref xs)` returns a plain `i64` (it aborts
on an empty Vec, it doesn't hand back an `Option`) -- there's no
variant to match against, so `while let` isn't the right tool here.
Guard the length instead:

```vani
// Drain a Vec<i64> used as a stack (pop from the back)
fn sum_stack(stack: mut ref Vec<i64>) -> i64 {
  let total: i64 = 0;
  while len(stack) > 0 {
    let v: i64 = pop(stack);
    total = total + v;
  }
  return total;
}
```

---

## 3. Or-patterns — multiple variants in one arm

Separate patterns with `|` inside a single match arm. The arm body runs
when ANY of the listed patterns match:

```vani
fn is_running(s: Status) -> bool {
  return match s {
    Status.Active | Status.Pending then true,
    Status.Done   | Status.Failed  then false,
  };
}
```

Rules:
- All patterns in a `|` group must bind the **same set of names** with
  the **same types**. A payload bound in one alternative must be bound
  in all of them (or use `_` to discard it).
- The compiler expands each `|` group into separate arms before
  type-checking, so exhaustiveness is checked correctly.

Or-patterns without payloads (tag-only variants) have no binding
constraint and are always safe to combine:

```vani
enum Dir { North, South, East, West }

fn orientation(direction: Dir) -> Str {
  return match direction {
    Dir.North | Dir.South then "vertical",
    Dir.East  | Dir.West  then "horizontal",
  };
}
```

Or-patterns with payloads — all alternatives must bind the same name:

```vani
enum Msg { Ping(i64), Pong(i64), Other }

fn describe_msg(msg: Msg) -> Str {
  return match msg {
    Msg.Ping(seq) | Msg.Pong(seq) then "seq: " + i64_to_str(seq),
    Msg.Other                     then "other" + "",
  };
}
```

Using a *different* name on each side of `|` breaks the "same names"
rule -- only the last alternative's bindings are actually in scope for
the arm body, so the other name is unresolved:

<img class="manas" src="../images/mascot/manas_mascot_error.png" title="this code does not compile!"/>

```vani
enum Msg { Ping(i64), Other(i64) }

fn show(msg: Msg) -> i64 {
  return match msg {
    Msg.Ping(seq) | Msg.Other(other) then seq,
  };
}
```

```
error: unknown variable 'seq'
    Msg.Ping(seq) | Msg.Other(other) then seq,
                                          ^^^
```

**Fix**: use the same binding name (`seq`) on both sides, or `_` to
discard a payload you don't need on one arm.

---

## 4. Pattern guards — refine a match with a runtime condition

Add `if <condition>` between the pattern and `then` to add an extra
runtime test. The arm only fires when the pattern matches AND the guard
is true:

```vani
fn classify(v: Opt) -> Str {
  return match v {
    Opt.Some(n) if n > 0  then "positive",
    Opt.Some(n) if n < 0  then "negative",
    Opt.Some(_)            then "zero",
    Opt.None               then "absent",
  };
}
```

The compiler merges guarded and unguarded arms for the same variant
into one switch case with nested `if`/`else` — you never get duplicate
case labels. Arm order matters for overlapping guards: the first
matching arm wins.

Guards can use the binding introduced by the pattern:

```vani
match result {
  Result.Ok(v) if v >= threshold then handle_pass(v),
  Result.Ok(v)                   then handle_below(v),
  Result.Err(code)               then handle_error(code),
}
```

Guards can also reference outer variables:

```vani
let limit: i64 = compute_limit();
match entry {
  Entry.Value(n) if n < limit then process_small(n),
  Entry.Value(n)              then process_large(n),
  Entry.Empty                 then {},
}
```

---

## 5. Slice patterns — destructuring sequences (v0.5.3+)

Slice patterns let you match on the **structure** of a `Vec<T>` or
fixed-size array `[T; N]` in a `match` arm, binding elements by
position while `..` absorbs any number of middle elements you don't
need.

Slice-pattern matching takes the `Vec` **by value** -- `match xs`
on a `ref Vec<i64>` parameter is rejected ("match scrutinee must be
an enum, integer, or bool type"); pass the `Vec` itself, not a
reference to it:

```vani
fn describe_vec(xs: Vec<i64>) -> Str {
  return match xs {
    []           then "empty",
    [x]          then "singleton",
    [first, ..]  then "starts with something",
  };
}
```

### Binding first and last

```vani
fn first_and_last(xs: Vec<i64>) -> i64 {
  return match xs {
    []                    then 0,
    [only]                then only,
    [first, .., last]     then first + last,
  };
}
```

- `[first, .., last]` binds the element at index 0 to `first` and the
  final element to `last`. The `..` between them absorbs zero or more
  elements that are not bound to any name.
- `[x]` matches a one-element slice and binds that element to `x`.
- `[]` matches an empty slice.

### Fixed-length patterns

When you know the exact length, you can name every element:

```vani
fn rgb_to_packed(channels: Vec<i64>) -> i64 {
  return match channels {
    [r, g, b] then {
      // exactly three elements -- a block arm's last expression
      // (no `;`, no `return`) is the arm's value; match arms can't
      // contain `return` since match is an expression, not a
      // statement (see Beginner Sec.8)
      r * 65536 + g * 256 + b
    },
    _ then 0 - 1,
  };
}
```

### Slice patterns with guards

<img class="manas" src="../images/mascot/manas_mascot_success.png" title="this is the correct, working version"/>

Pattern guards compose with slice patterns exactly as with enum arms.
A failed guard falls through to the next arm, just like you'd expect:

```vani
fn classify_scores(scores: Vec<i64>) -> Str {
  return match scores {
    []                          then "no data",
    [s] if s >= 90              then "single A",
    [s]                         then "single non-A",
    [first, .., last] if first == last  then "balanced",
    [first, .., last]           then "unbalanced",
  };
}
```

`classify_scores(vec(50))` correctly falls through the failed
`s >= 90` guard to "single non-A"; `classify_scores(vec(1, 2, 3))`
falls through the failed `first == last` guard to "unbalanced". See
[`examples/language/english/slice_pattern_guards.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/english/slice_pattern_guards.vani)
for a runnable version with every case exercised.

### What `..` can and cannot do

- `..` may appear **at most once** per pattern. `[a, .., b, .., c]` is
  rejected — the compiler can't determine which middle runs to the first
  `..` and which to the second.
- `..` absorbs zero or more elements, so `[first, ..]` matches any
  Vec of length ≥ 1 and `[first, .., last]` matches length ≥ 2.
- Slice patterns are exhaustiveness-checked like enum patterns. The
  compiler requires a `_` or `[..]` wildcard arm -- OR complete
  coverage from exact-length arms plus an unconditional (unguarded)
  has_rest arm, the shape `describe_vec` above uses -- to avoid
  "non-exhaustive match" errors.

<img class="manas" src="../images/mascot/manas_mascot_error.png" title="this code does not compile!"/>

```vani
fn classify(xs: Vec<i64>) -> i64 {
  return match xs {
    []  then 0,
    [x] then x,
    // missing: coverage for length >= 2 -- no `_` and no has_rest arm
  };
}
```

```
error: non-exhaustive match: slice/array scrutinees require a
       wildcard `_ then …` arm (or exact-length arms plus a
       `[.., x]`-shaped arm covering every remaining length) to
       cover lengths not explicitly listed
```

---

## Putting it all together

`Task` is a reserved built-in type name in vāṇी (the `task <fn>(...)`
concurrency primitive from Intermediate's later concurrency chapters
uses it) -- name your own enum something else, e.g. `Job`:

```vani
intent "Match enhancements — combined example.";

enum Job { Ready(i64), Blocked, Done }

fn run_queue(tasks: ref Vec<Job>) -> i64 {
  let completed: i64 = 0;
  let i: u64 = 0;

  while i < len(tasks) {
    // Job is non-Copy, so reading a slot by value needs clone_at
    // (indexing directly, or `ref tasks[i]`, isn't allowed here --
    // `ref` only borrows a named variable or a struct field).
    let item: Job = clone_at(tasks, i);
    // if let: skip Blocked and Done silently
    if let Job.Ready(priority) = item {
      // pattern guard: only process high-priority tasks in this pass
      if priority > 5 {
        print "running high-priority task", i;
        completed = completed + 1;
      }
    }
    i = i + 1;
  }
  return completed;
}

fn describe(t: Job) -> Str {
  return match t {
    // or-pattern: Blocked and Done share the same message
    Job.Blocked | Job.Done           then "inactive",
    // guard: distinguish priority tiers
    Job.Ready(p) if p > 5            then "high-priority",
    Job.Ready(p) if p > 0            then "normal",
    Job.Ready(_)                      then "idle",
  };
}

fn main() -> i64 {
  let tasks: Vec<Job> = vec(
    Job.Ready(8),
    Job.Blocked,
    Job.Ready(3),
    Job.Done,
    Job.Ready(6),
  );
  let n: i64 = run_queue(ref tasks);
  print "completed:", n;
  print describe(Job.Ready(8));
  print describe(Job.Blocked);
  print describe(Job.Ready(0 - 1));
  return 0;
}
```

Expected output:

```
running high-priority task 0
running high-priority task 4
completed: 2
high-priority
inactive
idle
```

---

## Summary table

| Form | Syntax | Fires when |
|------|--------|-----------|
| `if let` | `if let Pat = expr { … }` | `expr` matches `Pat` |
| `if let … else` | `if let Pat = expr { … } else { … }` | else when no match |
| `while let` | `while let Pat = expr { … }` | `expr` matches `Pat`; stops on first miss |
| Or-pattern | `Pat1 \| Pat2 then body` | either pattern matches |
| Pattern guard | `Pat if cond then body` | pattern matches AND `cond` is true |
| Slice pattern | `[first, .., last]` | Vec/array with ≥ 2 elements; binds endpoints |
| Empty slice | `[]` | Vec/array with exactly 0 elements |
| Singleton slice | `[x]` | Vec/array with exactly 1 element |

---

## Challenge

Define `enum Shape { Circle(i64), Square(i64), Triangle(TriSides) }`
where `TriSides` is a small struct holding two `i64` sides (an enum
variant can only carry a *single* payload in v1 -- see
[Sec.2](02_enums_payloads.md) -- so a two-field payload like
`Triangle(i64, i64)` needs a wrapper struct, not a tuple). Write:

1. A function that sums the perimeters of every `Circle` in a `Vec`
   (circle = `2 * 3 * r`; skip `Square` and `Triangle`).
2. A `classify(s: Shape) -> Str` function that uses or-patterns to group
   `Circle` and `Square` as `"regular"`, and a guard to label `Triangle`
   as `"scalene"` when the two sides differ or `"isosceles"` when they're equal.

<details>
<summary>Solution</summary>

```vani
struct TriSides { a: i64, b: i64 }
enum Shape { Circle(i64), Square(i64), Triangle(TriSides) }

fn total_circle_perimeter(shapes: ref Vec<Shape>) -> i64 {
  let total: i64 = 0;
  let i: u64 = 0;
  while i < len(shapes) {
    // Shape is non-Copy, so reading a slot by value needs clone_at
    // (same reason as the combined example above).
    let s: Shape = clone_at(shapes, i);
    if let Shape.Circle(r) = s {
      total = total + 2 * 3 * r;
    }
    i = i + 1;
  }
  return total;
}

fn classify(s: Shape) -> Str {
  return match s {
    Shape.Circle(_) | Shape.Square(_)           then "regular",
    Shape.Triangle(sides) if sides.a == sides.b then "isosceles",
    Shape.Triangle(_)                           then "scalene",
  };
}

fn main() -> i64 {
  let shapes: Vec<Shape> = vec(Shape.Circle(5), Shape.Square(4), Shape.Circle(2));
  print total_circle_perimeter(ref shapes);
  print classify(Shape.Circle(5));
  print classify(Shape.Triangle(TriSides { a: 3, b: 3 }));
  print classify(Shape.Triangle(TriSides { a: 3, b: 4 }));
  return 0;
}
```

</details>

---

**Previous**: [Sec.2 -- Enums with payloads ->](02_enums_payloads.md)
**Next**: [Sec.3a -- `Box<T>` and RAII primer ->](03a_box_raii_primer.md)
