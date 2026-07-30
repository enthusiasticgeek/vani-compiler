# Intermediate 15b -- Vec statistics and combinators

> **Learning goal**: use the built-in vector statistics and
> combinators -- sorting variants, argmin/argmax, quantiles,
> running sums, set operations, and statistical aggregates --
> without writing loops by hand.

> **Prerequisites**: [Beginner 7 -- Arrays and Vec<T>](../beginner/07_vec_arrays.md)
> and [Intermediate 15 -- Math, random numbers, and clone](15_math_rng.md).

`print` only accepts scalar values (i64, f64, bool, Str, OwnedStr) --
it can't print a `Vec<T>` directly. The examples below use a small
`vec_to_str` helper to render a `Vec<i64>` as a bracketed string for
display purposes.

---

## Sorting variants

```vani
intent "Vec sorting";

fn vec_to_str(v: ref Vec<i64>) -> OwnedStr {
  let s: OwnedStr = "[" + "";
  let i: i64 = 0;
  let n: i64 = len(v) as i64;
  while i < n {
    s = s + i64_to_str(v[i]);
    if i < n - 1 { s = s + ", "; }
    i = i + 1;
  }
  s = s + "]";
  return s;
}

fn main() -> i64 {
  let xs: Vec<i64> = vec(3, 1, 4, 1, 5, 9, 2, 6);

  sort(mut ref xs);           // ascending in place
  print "sorted:", vec_to_str(ref xs);

  sort_desc(mut ref xs);      // descending in place
  print "desc:", vec_to_str(ref xs);

  // sort_by: custom comparator (returns negative / 0 / positive)
  let ys: Vec<i64> = vec(10, 3, 7, 1);
  sort_by(mut ref ys, fn(a: i64, b: i64) -> i64 { return a - b; });
  print "custom sort:", vec_to_str(ref ys);

  return 0;
}
```

Expected output:

```
sorted: [1, 1, 2, 3, 4, 5, 6, 9]
desc: [9, 6, 5, 4, 3, 2, 1, 1]
custom sort: [1, 3, 7, 10]
```

| Builtin | What it does |
|---------|-------------|
| `sort(mut ref xs)` | Sort `Vec<i64>` or `Vec<f64>` ascending in place |
| `sort_desc(mut ref xs)` | Sort descending in place |
| `sort_by(mut ref xs, cmp)` | Custom comparator `fn(T,T)->i64`; works on `Vec<T>` for any `Copy` `T`, not just `i64`/`f64` |
| `dedup(mut ref xs)` | Remove consecutive duplicates (sort first) |
| `reverse(mut ref xs)` | Reverse in place |
| `binary_search(ref xs, v)` | Binary search on sorted vec; returns index or -1 |

`sort`/`sort_desc` (no comparator) stay `i64`/`f64`-only -- there's no
derivable ascending order for a struct. `sort_by` has no such limit
since the caller supplies the order, so it works on a `Vec` of any
`Copy` type, including structs:

```vani
struct Point { x: i64, y: i64 }

fn cmp_by_x(a: Point, b: Point) -> i64 { return a.x - b.x; }

fn main() -> i64 {
  let pts: Vec<Point> = vec(Point { x: 3, y: 0 }, Point { x: 1, y: 0 });
  sort_by(mut ref pts, cmp_by_x);
  print "sorted by x[0]:", pts[0].x;  // 1
  print "sorted by x[1]:", pts[1].x;  // 3
  return 0;
}
```

(A `Vec<T>` where `T` is non-`Copy`, e.g. `Vec<OwnedStr>`, is still
correctly rejected with a diagnostic.)

<img class="manas" src="../images/mascot/manas_mascot_error.png" title="this code does not compile!"/>

```vani
let names: Vec<OwnedStr> = vec("banana" + "", "apple" + "");
sort(mut ref names);
```

```
error: sort() only supports `Vec<i64> or Vec<f64>` in v1, got element type OwnedStr
  sort(mut ref names);
       ^^^^^^^^^^^^^
```

---

## Argmin / argmax and k-th smallest

`vec_argmin`/`vec_argmax`/`vec_min`/`vec_max` take a **required
fallback value** as their second argument, used if the `Vec` is
empty (so the return type can stay a plain `i64`/`f64` instead of an
`Option`):

```vani
intent "Vec argmin/argmax";

fn main() -> i64 {
  let xs: Vec<i64> = vec(3, 1, 4, 1, 5, 9);

  let mn: i64 = vec_argmin(ref xs, -1);  // index of minimum (0-indexed); -1 if empty
  let mx: i64 = vec_argmax(ref xs, -1);  // index of maximum; -1 if empty
  print "argmin index:", mn;             // 1 (value 1)
  print "argmax index:", mx;             // 5 (value 9)

  // k-th smallest (0-indexed, not sorted -- uses quickselect)
  let med: i64 = vec_kth_smallest(mut ref xs, 2);
  print "3rd smallest:", med;            // 3

  return 0;
}
```

| Builtin | `Vec<i64>` returns | `Vec<f64>` returns | Description |
|---------|-------------------|-------------------|-------------|
| `vec_argmin(ref xs, fallback)` | `i64` | `i64` | Index of minimum element (always integer); `fallback` used if empty |
| `vec_argmax(ref xs, fallback)` | `i64` | `i64` | Index of maximum element; `fallback` used if empty |
| `vec_kth_smallest(ref xs, k)` | `i64` | `f64` (qNaN if k out of bounds) | k-th smallest; quickselect |
| `vec_median(ref xs)` | `i64` | `f64` | Median value; quickselect |

---

## Statistical aggregates

`vec_min`/`vec_max` also take a required fallback value (same
empty-`Vec` rationale as `vec_argmin`/`vec_argmax` above):

```vani
intent "Vec statistics";

fn main() -> i64 {
  let xs: Vec<i64> = vec(2, 4, 4, 4, 5, 5, 7, 9);

  print "sum:",  vec_sum(ref xs);               // 40
  print "min:",  vec_min(ref xs, 0);            // 2
  print "max:",  vec_max(ref xs, 0);            // 9
  print "mean:", vec_mean(ref xs);              // 5 (integer division)
  print "mode:", vec_mode(ref xs);              // 4 (most frequent)

  // harmonic/geometric mean of TWO scalars (not a Vec reduction)
  print "harmonic mean(2,4):", f64_harmonic_mean(2.0, 4.0);   // 2.667
  print "geometric mean(2,4):", f64_geometric_mean(2.0, 4.0); // 2.828

  return 0;
}
```

| Builtin | `Vec<i64>` returns | `Vec<f64>` returns | Description |
|---------|-------------------|-------------------|-------------|
| `vec_sum(ref xs)` | `i64` | `f64` | Sum of all elements |
| `vec_min(ref xs, fallback)` | `i64` | `f64` | Minimum element; `fallback` used if empty |
| `vec_max(ref xs, fallback)` | `i64` | `f64` | Maximum element; `fallback` used if empty |
| `vec_mean(ref xs)` | `i64` (floor div) | `f64` | Arithmetic mean |
| `vec_mode(ref xs)` | `i64` | — | Most frequent element |

`f64_harmonic_mean(a, b)` and `f64_geometric_mean(a, b)` are
**2-argument scalar math functions** (mean of exactly two numbers),
not `Vec<f64>` reductions -- despite the name, they don't take a
`ref Vec<f64>`. See [Sec.15a](15a_math_deep.md) for the rest of the
scalar math library.

---

## Running / cumulative operations

```vani
intent "Running sums";

fn vec_to_str(v: ref Vec<i64>) -> OwnedStr {
  let s: OwnedStr = "[" + "";
  let i: i64 = 0;
  let n: i64 = len(v) as i64;
  while i < n {
    s = s + i64_to_str(v[i]);
    if i < n - 1 { s = s + ", "; }
    i = i + 1;
  }
  s = s + "]";
  return s;
}

fn main() -> i64 {
  let xs: Vec<i64> = vec(1, 2, 3, 4, 5);

  let rs: Vec<i64> = vec_running_sum(ref xs);
  print "running sum:", vec_to_str(ref rs);       // [1, 3, 6, 10, 15]

  let cm: Vec<i64> = vec_cumulative_max(ref xs);
  print "cumulative max:", vec_to_str(ref cm);    // [1, 2, 3, 4, 5]

  let cn: Vec<i64> = vec_cumulative_min(ref xs);
  print "cumulative min:", vec_to_str(ref cn);    // [1, 1, 1, 1, 1]

  return 0;
}
```

| Builtin | Output |
|---------|--------|
| `vec_running_sum(ref xs)` | Prefix sums: `rs[i] = xs[0] + ... + xs[i]` |
| `vec_cumulative_max(ref xs)` | `cm[i] = max(xs[0..=i])` |
| `vec_cumulative_min(ref xs)` | `cn[i] = min(xs[0..=i])` |

---

## Set operations

```vani
intent "Vec set ops";

fn main() -> i64 {
  let a: Vec<i64> = vec(1, 2, 3, 4);
  let b: Vec<i64> = vec(3, 4, 5, 6);

  print "subset:",    vec_subset_of(ref a, ref b);    // false (1,2 not in b)
  print "disjoint:",  vec_disjoint(ref a, ref b);     // false (3,4 shared)
  print "equal set:", vec_equal_set(ref a, ref b);    // false

  let c: Vec<i64> = vec(1, 2);
  print "c subset a:", vec_subset_of(ref c, ref a);   // true

  return 0;
}
```

| Builtin | Description |
|---------|-------------|
| `vec_subset_of(ref a, ref b)` | Every element of `a` appears in `b` |
| `vec_disjoint(ref a, ref b)` | No element appears in both |
| `vec_equal_set(ref a, ref b)` | Same elements (order-independent) |

These do linear scans -- for large vecs, load into a `HashSet<i64>` first.

---

## Functional combinators

`map`/`filter`/`fold` are called as `vec_map`/`vec_filter`/`vec_fold`
(bare `map`/`filter`/`fold` don't exist as standalone functions).
`find(ref xs, needle)` searches for a **value**, not a predicate --
it returns `Option<i64>` (the index of the first occurrence, or
`None`), not a bare `i64` with a `-1` sentinel:

```vani
intent "Vec combinators";

fn double(x: i64) -> i64 { return x * 2; }
fn is_even(x: i64) -> bool { return x % 2 == 0; }
fn add(acc: i64, x: i64) -> i64 { return acc + x; }

fn vec_to_str(v: ref Vec<i64>) -> OwnedStr {
  let s: OwnedStr = "[" + "";
  let i: i64 = 0;
  let n: i64 = len(v) as i64;
  while i < n {
    s = s + i64_to_str(v[i]);
    if i < n - 1 { s = s + ", "; }
    i = i + 1;
  }
  s = s + "]";
  return s;
}

fn main() -> i64 {
  let xs: Vec<i64> = vec(1, 2, 3, 4, 5);

  let doubled: Vec<i64> = vec_map(ref xs, double);
  print "map:", vec_to_str(ref doubled);           // [2, 4, 6, 8, 10]

  let evens: Vec<i64> = vec_filter(ref xs, is_even);
  print "filter:", vec_to_str(ref evens);          // [2, 4]

  let total: i64 = vec_fold(ref xs, 0, add);
  print "fold:", total;                            // 15

  // find: index of the first element equal to a value, or None
  let idx: Option<i64> = find(ref xs, 2);
  print "find(2) index:", option_unwrap_or(idx, -1);  // 1

  // contains: any element equals v?
  print "contains 3:", contains(ref xs, 3);        // true

  return 0;
}
```

| Builtin | `Vec<i64>` form | `Vec<f64>` form | Description |
|---------|-----------------|-----------------|-------------|
| `vec_map(ref xs, f)` | `fn(i64)->i64 -> Vec<i64>` | `fn(f64)->f64 -> Vec<f64>` | Apply f to each element |
| `vec_filter(ref xs, pred)` | `fn(i64)->bool -> Vec<i64>` | `fn(f64)->bool -> Vec<f64>` | Keep elements where pred is true |
| `vec_fold(ref xs, init, f)` | `i64, fn(i64,i64)->i64 -> i64` | `f64, fn(f64,f64)->f64 -> f64` | Left-fold with initial value |
| `vec_dot(ref xs, ref ys)` | `-> i64` | `-> f64` | Inner / dot product |
| `find(ref xs, needle)` | `-> Option<i64>` | — | Index of first element equal to `needle`, or `None` |
| `contains(ref xs, v)` | `-> bool` | — | Any element equals v |

---

## Mutation combinators

```vani
fn vec_to_str(v: ref Vec<i64>) -> OwnedStr {
  let s: OwnedStr = "[" + "";
  let i: i64 = 0;
  let n: i64 = len(v) as i64;
  while i < n {
    s = s + i64_to_str(v[i]);
    if i < n - 1 { s = s + ", "; }
    i = i + 1;
  }
  s = s + "]";
  return s;
}

fn main() -> i64 {
  let xs: Vec<i64> = vec(3, 1, 4, 1, 5, 2);

  // replace all occurrences of 1 with 99
  vec_replace_all(mut ref xs, 1, 99);
  print "after replace:", vec_to_str(ref xs);   // [3, 99, 4, 99, 5, 2]

  // remove element at index 2
  vec_remove_at(mut ref xs, 2);
  print "after remove_at(2):", vec_to_str(ref xs);  // [3, 99, 99, 5, 2]

  // swap indices 0 and 4
  vec_swap(mut ref xs, 0, 4);
  print "after swap(0,4):", vec_to_str(ref xs); // [2, 99, 99, 5, 3]

  return 0;
}
```

| Builtin | Description |
|---------|-------------|
| `vec_replace_all(mut ref xs, old, new)` | Replace every `old` with `new` |
| `vec_remove_at(mut ref xs, i)` | Remove element at index i; shift tail left |
| `vec_swap(mut ref xs, i, j)` | Swap elements at indices i and j |
| `insert(mut ref xs, i, v)` | Insert v at index i; shift tail right |
| `swap_remove(mut ref xs, i)` | Remove at i by swapping with last; O(1) |
| `clear(mut ref xs)` | Remove all elements (keeps allocation) |

---

## Putting it together: frequency analysis

```vani
intent "letter frequency";

fn vec_to_str(v: ref Vec<i64>) -> OwnedStr {
  let s: OwnedStr = "[" + "";
  let i: i64 = 0;
  let n: i64 = len(v) as i64;
  while i < n {
    s = s + i64_to_str(v[i]);
    if i < n - 1 { s = s + ", "; }
    i = i + 1;
  }
  s = s + "]";
  return s;
}

fn main() -> i64 {
  // Count how many times each distinct value appears
  let data: Vec<i64> = vec(3, 1, 4, 1, 5, 9, 2, 6, 5, 3, 5);

  // sort to group duplicates, then count runs
  sort(mut ref data);
  print "sorted:", vec_to_str(ref data);
  print "mode (most frequent):", vec_mode(ref data);  // 5
  print "median:", vec_median(mut ref data);           // 4

  return 0;
}
```

---

**Previous**: [Sec.15a -- Math library deep-dive ->](15a_math_deep.md)
**Next**: [Sec.11a -- vāṇी design idioms primer ->](11a_vani_idioms_primer.md)
