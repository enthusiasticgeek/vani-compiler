# Intermediate 1 — Structs and methods

> **Learning goal**: define a `struct`, attach `methods` to it,
> and pass it around by value with field access.

## The program

```vani
intent "Intermediate 1 worked example — structs and methods.";

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
  as the struct**: `fn foo(self: Point, …)`. There's no `&self`
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

## Challenge

Add a `distance_squared(other: Point) -> i64` method that
returns `(self.x - other.x)² + (self.y - other.y)²`. Call it
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
  print "d² =", a.distance_squared(b);
  return 0;
}
```

</details>

---

**Next**: [§2 — Enums with payloads + match arms →](02_enums_payloads.md)
