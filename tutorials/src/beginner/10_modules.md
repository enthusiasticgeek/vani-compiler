# Beginner 10 — Modules and `pub`

> **Learning goal**: organize code into `module` blocks, mark
> exposed items with `pub`, reference them with `::`, and bring
> them into scope with `use`.

## The program

Save this in `~/lesson10.vani`:

```rust
intent "Lesson 10 worked example — modules + pub + use.";

module math {
  pub fn square(n: i64) -> i64 {
    return n * n;
  }

  pub fn cube(n: i64) -> i64 {
    return n * n * n;
  }

  fn internal_helper(n: i64) -> i64 {
    return n + 1;
  }

  pub fn next_square(n: i64) -> i64 {
    return square(internal_helper(n));
  }
}

use math::square;

fn main() -> i64 {
  let a: i64 = square(5);
  print "square(5) =", a;

  let b: i64 = math::cube(3);
  print "math::cube(3) =", b;

  let c: i64 = math::next_square(4);
  print "math::next_square(4) =", c;

  return 0;
}
```

## Compile + run

```bash
vanic run ~/lesson10.vani
```

Expected output:

```
square(5) = 25
math::cube(3) = 27
math::next_square(4) = 25
```

## Why it works that way

- **`module name { ... }`** introduces a namespace. Items
  inside are addressed as `name::item`. Modules can be nested
  arbitrarily.
- **Items are private by default**. `internal_helper` above has
  no `pub`, so callers outside `math` can't see it. `square`,
  `cube`, and `next_square` are explicitly `pub`.
- **Intra-module names need no prefix**. `next_square` calls
  bare `square(...)` and `internal_helper(...)`; the compiler's
  name-resolution pass knows it's inside `math` and rewrites
  the calls to `math__square` / `math__internal_helper` at the
  IR layer.
- **`use math::square;` imports a single item** into the
  current scope under its bare name. Multi-item form:
  `use math::{square, cube};`. Glob form: `use math::*;` (every
  direct public child of `math`, non-transitive — sub-modules
  aren't pulled in).
- **`as` renames**. `use math::square as sq;` brings `square`
  into scope as `sq` only. Useful when two modules expose the
  same name.
- **One file per module isn't required in v1**. Modules can
  live in the same file as `main`. Multi-file projects use
  `vani.toml` + `use "path";` declarations to splice files
  together — that's Intermediate §8.

## Challenge

Add a `geom` module to `lesson10.vani` with a `pub fn area(w:
i64, h: i64) -> i64` returning `w * h`, and a private helper
function. Call `area` from `main` two ways: once with the full
path `geom::area(...)` and once after a `use geom::area;`. Verify
both calls produce identical output.

<details>
<summary>Solution</summary>

```rust
module geom {
  fn ensure_positive(n: i64) -> i64 {
    if n < 0 {
      return 0 - n;
    }
    return n;
  }

  pub fn area(w: i64, h: i64) -> i64 {
    return ensure_positive(w) * ensure_positive(h);
  }
}

use geom::area;

fn main() -> i64 {
  print "geom::area(3, 4) =", geom::area(3, 4);
  print "area(3, 4)       =", area(3, 4);
  return 0;
}
```

</details>

---

**Next**: [§11 — Challenges →](11_challenges.md)
