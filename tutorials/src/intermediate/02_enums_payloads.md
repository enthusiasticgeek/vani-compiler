# Intermediate 2 -- Enums with payloads + match arms

> **Learning goal**: declare a tagged-union enum, construct
> variants with payload data, and destructure them with
> `match` arms.

An enum is like a multiple-choice form field: the value is
ALWAYS one of a fixed set of options, and each option can carry
its own data. Think of a delivery status: it's either
`Shipped(tracking_number)`, `Delivered(timestamp)`, or
`Failed(reason)` -- never more than one at a time, and each
variant carries different information. `match` is the natural
companion: it lets you write separate instructions for each
possible status, and the compiler ensures you handle ALL of
them (so no delivery status goes unprocessed).

## The program

```vani
intent "Intermediate 2 worked example -- enums with payloads.";

enum Result { Ok(i64), Err(i64) }

fn safe_div(a: i64, b: i64) -> Result {
  if b == 0 {
    return Result.Err(0 - 1);
  }
  return Result.Ok(a / b);
}

fn unwrap_or(r: Result, def: i64) -> i64 {
  return match r {
    Result.Ok(v) then v,
    Result.Err(_) then def,
  };
}

fn main() -> i64 {
  let r1: Result = safe_div(20, 4);
  let r2: Result = safe_div(10, 0);
  print "20/4 =", unwrap_or(r1, 0 - 999);
  print "10/0 =", unwrap_or(r2, 0 - 999);
  return 0;
}
```

## Compile + run

```bash
vanic run ~/int2.vani
```

Output:

```
20/4 = 5
10/0 = -999
```

## Why it works that way

- **`enum Name { V1(T1), V2(T2), V3 }`** declares a tagged union.
  Each variant can be either a tag (no payload) or a tag+payload.
  The payload type goes in parentheses after the variant name.
- **v1 restriction**: a single payload type *per variant* only --
  multi-payload tuples (`Ok(i64, Str)`) aren't supported. Wrap
  multi-field variants in a struct instead, and put the struct
  type in the payload.
- **Construction**: `Result.Ok(42)` -- note the dot, not the
  double-colon. This is one of the small surface-syntax diffs
  from Rust.
- **Match destructuring**: `Result.Ok(v) then v` extracts the
  payload as a fresh `v` binding scoped to the arm. `Result.Err(_)`
  matches but discards the payload.
- **Match is an expression** (Beginner Sec.8). Return its value
  with `return match ... { ... };`.

## v1 limitations to know about

These are listed in [`docs/v1_limitations.md`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/docs/v1_limitations.md) -- keep them in mind:

- **No enum-destructure in `let`**: `let Result.Ok(v) = r;`
  doesn't work. You always go through `match`.
- **No nested patterns**: `Some(Some(v))` patterns aren't
  supported; flatten with two `match` levels.
- **`Box<T>` works fine in general** -- including a self-
  referential *struct* (`struct Node { next: Option<Box<Node>> }`
  compiles and runs correctly). What's **not** supported is boxing
  an *enum*: `box(some_enum_value)` is rejected ("box() v1
  supports Copy + sized element types... got `Tree`"), and an enum
  variant can't take a non-Copy struct payload either (a struct
  containing `Box<T>` fields isn't an admitted enum payload type).
  So a recursive **enum** specifically (`Tree.Node(Box<Tree>,
  Box<Tree>)`) needs a workaround using arena indices -- recursive
  *structs* don't. The Composite design pattern example shows the
  tagged-struct workaround for the enum case.

### Seeing the `let`-destructure rejection

<img class="manas" src="../images/mascot/manas_mascot_error.png" title="this code does not compile!"/>

```vani
enum Result { Ok(i64), Err(i64) }

fn main() -> i64 {
  let r: Result = Result.Ok(5);
  let Result.Ok(v) = r;   // no enum-destructure in let
  return v;
}
```

```
error: expected '='
  let Result.Ok(v) = r;
            ^
```

**Fix**: go through `match` instead, exactly like `unwrap_or` does
above.

### The `Box<T>` workaround needs care

The arena-index workaround mentioned above compiles cleanly, but it
trades away the compiler's help: nothing stops you from writing an
out-of-range or stale index, so the discipline of keeping indices
valid is now on you instead of on the type system.

<img class="manas" src="../images/mascot/manas_mascot_caution.png" title="this code needs extra care"/>

```vani
struct TreeNode { value: i64, left: i64, right: i64 }   // -1 = no child

fn sum_tree(nodes: ref Vec<TreeNode>, idx: i64) -> i64 {
  if idx < 0 {
    return 0;
  }
  let n: TreeNode = nodes[idx];
  return n.value + sum_tree(nodes, n.left) + sum_tree(nodes, n.right);
}

fn main() -> i64 {
  let nodes: Vec<TreeNode> = vec(
    TreeNode { value: 1, left: 1, right: 2 },
    TreeNode { value: 2, left: 0 - 1, right: 0 - 1 },
    TreeNode { value: 3, left: 0 - 1, right: 0 - 1 },
  );
  print "sum =", sum_tree(ref nodes, 0);
  return 0;
}
```

`left` / `right` are plain `i64` indices into `nodes`, not `Box<Tree>`
pointers -- there's no cycle, no `Rc`, and the whole tree frees at
once when `nodes` drops. The catch: `-1` as a "no child" sentinel is
a convention the compiler doesn't check, so an off-by-one edit here
is a runtime bug, not a compile error.

## Challenge

Define `enum Color { Red, Green, Blue, Custom(i64) }` and a
function `brightness(c: Color) -> i64` that returns 100 for
`Red`, 80 for `Green`, 60 for `Blue`, and the payload itself
for `Custom(n)`. Print results for several inputs.

<details>
<summary>Solution</summary>

```vani
enum Color { Red, Green, Blue, Custom(i64) }

fn brightness(c: Color) -> i64 {
  return match c {
    Color.Red then 100,
    Color.Green then 80,
    Color.Blue then 60,
    Color.Custom(n) then n,
  };
}

fn main() -> i64 {
  print brightness(Color.Red);
  print brightness(Color.Custom(42));
  return 0;
}
```

</details>

---

**Previous**: [Sec.1 -- Structs and methods ->](01_struct_methods.md)
**Next**: [Sec.2b -- Match enhancements ->](02b_match_enhancements.md)
