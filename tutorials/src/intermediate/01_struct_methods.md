# Intermediate 1 -- Structs and methods

> **Learning goal**: define a `struct`, attach `methods` to it,
> and pass it around by value with field access.

A `struct` is a named collection of fields -- think of a business
card: it groups `name`, `email`, and `phone` together under one
label so you can hand the whole card to someone instead of
passing three separate pieces of paper. `methods` are the
actions that make sense to do WITH that card (`fn print_card`,
`fn update_email`). Grouping data + actions together is the
core idea behind OOP, and vāṇī does it without inheritance:
just structs + methods.

## The program

```vani
intent "Intermediate 1 worked example -- structs and methods.";

struct Point { x: i64, y: i64 }

methods on Point {
  fn manhattan(self: Point) -> i64 {
    let dx: i64 = if self.x < 0 { 0 - self.x } else { self.x };
    let dy: i64 = if self.y < 0 { 0 - self.y } else { self.y };
    return dx + dy;
  }

  fn translated(self: Point, dx: i64, dy: i64) -> Point {
    return Point { x: self.x + dx, y: self.y + dy };
  }
}

fn main() -> i64 {
  let p: Point = Point { x: 3, y: 4 };
  let m: i64 = p.manhattan();
  print "manhattan(3, 4) =", m;

  let q: Point = p.translated(10, 0 - 5);
  print "translated.x =", q.x;
  print "translated.y =", q.y;

  return 0;
}
```

## Compile + run

```bash
vanic run ~/int1.vani
```

Output:

```
manhattan(3, 4) = 7
translated.x = 13
translated.y = -1
```

## Why it works that way

- **`struct Name { f1: T1, f2: T2 }`** declares a named product
  type. Fields are typed; the comma between fields is required;
  no trailing comma in v1.
- **`methods on Name { ... }`** is the method block syntax.
  Inside, each `fn`'s **first parameter must be explicitly typed
  as the struct**: `fn foo(self: Point, ...)`. There's no `&self`
  / `&mut self` shorthand in v1.
- **Field access uses `.`**: `p.x`, `p.y`. Reading a field by
  value copies the primitive; reading a `Vec<T>` field through
  a borrow keeps ownership with the struct.
- **Construction**: `Point { x: 3, y: 4 }`. Every field must be
  set; field name punning (`Point { x, y }`) isn't supported
  in v1.
- **Methods on values**: methods that take `self: Point` get a
  *copy* of the struct. Returning a modified copy (like
  `translated`) is the immutable-update pattern.

## What the compiler catches

### Struct literals must set every field

The prose above says "every field must be set" -- here's what happens
when you leave one out:

<img class="manas" src="../images/mascot/manas_mascot_error.png" title="this code does not compile!"/>

```vani
struct Point { x: i64, y: i64 }

fn main() -> i64 {
  let p: Point = Point { x: 3 };   // missing y
  return p.x;
}
```

```
error: struct 'Point' has 2 fields, literal provides 1
  let p: Point = Point { x: 3 };
                 ^^^^^^^^^^^^^^
  help: 1. The struct declares 2 fields, but the literal provides 1.
  help: 2. Every field must be set in a struct literal — there are no default field values in v1, and field name punning (`Point { x, y }`) isn't supported either.
  help: 3. Add the missing field(s), or remove the extra one(s) if the struct's declared fields legitimately changed.
```

**Fix**: supply every declared field:

<img class="manas" src="../images/mascot/manas_mascot_success.png" title="this is the correct, working version"/>

```vani
struct Point { x: i64, y: i64 }

fn main() -> i64 {
  let p: Point = Point { x: 3, y: 4 };
  return p.x + p.y;
}
```

## Challenge

Add a `distance_squared(other: Point) -> i64` method that
returns `(self.x - other.x)^2 + (self.y - other.y)^2`. Call it
twice from `main` and verify both results print.

<details>
<summary>Solution</summary>

```vani
methods on Point {
  fn distance_squared(self: Point, other: Point) -> i64 {
    let dx: i64 = self.x - other.x;
    let dy: i64 = self.y - other.y;
    return dx * dx + dy * dy;
  }
}

fn main() -> i64 {
  let a: Point = Point { x: 0, y: 0 };
  let b: Point = Point { x: 3, y: 4 };
  print "d^2 =", a.distance_squared(b);
  return 0;
}
```

</details>

---

**Previous**: [Sec.14 -- Capstone: a class grade-report tool ->](../beginner/14_gradebook_capstone.md)
**Next**: [Sec.2 -- Enums with payloads + match arms ->](02_enums_payloads.md)
