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

`vec_max_by`/`vec_min_by` are the "extremum by a custom key" siblings
of `vec_argmin`/`vec_argmax` -- instead of the index, they hand back
the element itself, chosen by a key function you supply (same shape
as `sort_by`'s comparator, but a single-argument key extractor):

```vani
intent "extremes by custom key";

fn identity(x: i64) -> i64 { return x; }

fn main() -> i64 {
  let xs: Vec<i64> = vec(3, 1, 4, 1, 5, 9, 2, 6);
  print "max_by:", option_unwrap_or(vec_max_by(ref xs, identity), -1);   // 9
  print "min_by:", option_unwrap_or(vec_min_by(ref xs, identity), -1);   // 1
  return 0;
}
```

| Builtin | Description |
|---------|-------------|
| `vec_max_by(ref xs, key: fn(i64)->i64) -> Option<i64>` | element with the largest `key(x)`; `None` if empty |
| `vec_min_by(ref xs, key: fn(i64)->i64) -> Option<i64>` | element with the smallest `key(x)`; `None` if empty |

---

## Slicing: take, drop, first, last

```vani
intent "Vec slicing";

fn is_even(x: i64) -> bool { return x % 2 == 0; }

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

  let tk: Vec<i64> = vec_take(ref xs, 3);
  print "take(3):", vec_to_str(ref tk);         // [1, 2, 3]
  let dr: Vec<i64> = vec_drop(ref xs, 3);
  print "drop(3):", vec_to_str(ref dr);         // [4, 5]

  let tkw: Vec<i64> = vec_take_while(ref xs, is_even);
  print "take_while(is_even):", vec_to_str(ref tkw);   // [] -- xs[0]=1 already fails
  let drw: Vec<i64> = vec_drop_while(ref xs, is_even);
  print "drop_while(is_even):", vec_to_str(ref drw);   // [1, 2, 3, 4, 5] -- nothing dropped

  print "first:", option_unwrap_or(vec_first(ref xs), -1);   // 1
  print "last:", option_unwrap_or(vec_last(ref xs), -1);     // 5
  return 0;
}
```

| Builtin | Description |
|---------|-------------|
| `vec_take(ref xs, n) -> Vec<i64>` | first `n` elements (fewer if `xs` is shorter) |
| `vec_drop(ref xs, n) -> Vec<i64>` | everything after the first `n` elements |
| `vec_take_while(ref xs, pred) -> Vec<i64>` | longest prefix where `pred` holds |
| `vec_drop_while(ref xs, pred) -> Vec<i64>` | the remaining suffix after that prefix |
| `vec_first(ref xs) -> Option<i64>` / `vec_last(ref xs) -> Option<i64>` | first/last element, or `None` if empty |

---

## Search and counting

Beyond `find`/`contains` from the combinators section below, there's
a family of position- and count-oriented searches:

```vani
intent "search and counting";

fn is_even(x: i64) -> bool { return x % 2 == 0; }

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
  let xs: Vec<i64> = vec(1, 2, 3, 4, 3, 5);

  print "position(is_even):", option_unwrap_or(vec_position(ref xs, is_even), -1);  // 1 (index of 2)
  print "count_if(is_even):", vec_count_if(ref xs, is_even);      // 2

  print "count_value(3):", vec_count_value(ref xs, 3);            // 2
  print "index_of_value(3):", vec_index_of_value(ref xs, 3);      // 2 (first occurrence)
  print "last_index_of_value(3):", vec_last_index_of_value(ref xs, 3);  // 4
  let iov: Vec<i64> = vec_indices_of_value(ref xs, 3);
  print "indices_of_value(3):", vec_to_str(ref iov);              // [2, 4]

  print "count_distinct:", vec_count_distinct(ref xs);            // 5
  return 0;
}
```

| Builtin | Returns | Description |
|---------|---------|-------------|
| `vec_position(ref xs, pred: fn(i64)->bool)` | `Option<i64>` | index of the first element where `pred` holds |
| `vec_count_if(ref xs, pred)` | `i64` | count of elements where `pred` holds |
| `vec_count_value(ref xs, v)` | `i64` | count of elements equal to `v` |
| `vec_index_of_value(ref xs, v)` | `i64` (`-1` if absent) | index of the first occurrence of `v` |
| `vec_last_index_of_value(ref xs, v)` | `i64` (`-1` if absent) | index of the last occurrence |
| `vec_indices_of_value(ref xs, v)` | `Vec<i64>` | every index where `v` occurs |
| `vec_count_distinct(ref xs)` | `i64` | number of distinct values |

Note the return-type split: `vec_position` (a predicate search) uses
`Option<i64>` like `find`; the value-searches (`vec_index_of_value`
et al.) use a bare `i64` with a `-1` sentinel instead.

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
| `vec_running_product(ref xs)` | `rp[i] = xs[0] * ... * xs[i]` |
| `vec_running_mean(ref xs)` | `rm[i] = mean(xs[0..=i])` (integer floor division, like `vec_mean`) |
| `vec_running_xor(ref xs)` / `vec_running_and(ref xs)` / `vec_running_or(ref xs)` | same idea with bitwise `^`/`&`/`\|` instead of `+` |

```vani
intent "more running operations";

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
  let rp: Vec<i64> = vec_running_product(ref xs);
  print "running product:", vec_to_str(ref rp);   // [1, 2, 6, 24, 120]
  let rm: Vec<i64> = vec_running_mean(ref xs);
  print "running mean:", vec_to_str(ref rm);       // [1, 1, 2, 2, 3]
  return 0;
}
```

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

The set operations that return a `Vec<i64>` (rather than a `bool`)
have their own builtins:

```vani
intent "Vec set-building ops";

fn main() -> i64 {
  let a: Vec<i64> = vec(1, 2, 3, 4);
  let b: Vec<i64> = vec(3, 4, 5, 6);

  let it: Vec<i64> = vec_intersect(ref a, ref b);
  print "intersect[0]:", it[0];    // 3
  print "intersect[1]:", it[1];    // 4

  let df: Vec<i64> = vec_difference(ref a, ref b);
  print "difference[0]:", df[0];   // 1 (in a, not in b)
  print "difference[1]:", df[1];   // 2

  let un: Vec<i64> = vec_union(ref a, ref b);
  print "union len:", len(ref un) as i64;   // 6 (1,2,3,4,5,6)

  print "equal_seq(a,a):", vec_equal_seq(ref a, ref a);   // true -- same elements, same ORDER
  return 0;
}
```

| Builtin | Description |
|---------|-------------|
| `vec_intersect(ref a, ref b) -> Vec<i64>` | elements present in both |
| `vec_difference(ref a, ref b) -> Vec<i64>` | elements in `a` but not `b` |
| `vec_union(ref a, ref b) -> Vec<i64>` | all distinct elements from both |
| `vec_equal_seq(ref a, ref b) -> bool` | same elements in the same ORDER (unlike `vec_equal_set`) |
| `vec_diff(ref xs) -> Vec<i64>` | **not a set op** -- consecutive differences: `[xs[1]-xs[0], xs[2]-xs[1], ...]` |

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

**Fused variants**, for when map/filter/fold would otherwise build
an intermediate `Vec` you'd immediately consume -- these do it in
one pass:

```vani
intent "fused map/filter/fold";

fn double(x: i64) -> i64 { return x * 2; }
fn is_even(x: i64) -> bool { return x % 2 == 0; }
fn add(a: i64, b: i64) -> i64 { return a + b; }

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

  // map then fold, one pass -- no intermediate Vec<i64>
  print "map_fold(double,add):", vec_map_fold(ref xs, 0, double, add);   // 30

  // filter then fold, one pass
  print "filter_fold(is_even,add):", vec_filter_fold(ref xs, 0, is_even, add);  // 6

  // map then filter (predicate runs on the MAPPED value), one pass
  let mfil: Vec<i64> = vec_map_filter(ref xs, double, is_even);
  print "map_filter(double,is_even):", vec_to_str(ref mfil);   // [2, 4, 6, 8, 10] -- all even after doubling

  // map, filter, then fold -- one pass, no intermediate Vec at all
  print "map_filter_fold:", vec_map_filter_fold(ref xs, 0, double, is_even, add);  // 30
  return 0;
}
```

| Builtin | Description |
|---------|-------------|
| `vec_map_fold(ref xs, init, map_fn, fold_fn) -> i64` | `fold(map(xs))` in one pass |
| `vec_filter_fold(ref xs, init, pred, fold_fn) -> i64` | `fold(filter(xs))` in one pass |
| `vec_map_filter(ref xs, map_fn, pred) -> Vec<i64>` | `filter(map(xs))` in one pass -- `pred` sees the mapped value |
| `vec_map_filter_fold(ref xs, init, map_fn, pred, fold_fn) -> i64` | all three, one pass |

---

## Combining and building vecs

```vani
intent "combining and building vecs";

fn add(a: i64, b: i64) -> i64 { return a + b; }

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
  let xs: Vec<i64> = vec(1, 2, 3);
  let ys: Vec<i64> = vec(10, 20, 30);

  let zw: Vec<i64> = vec_zip_with(ref xs, ref ys, add);
  print "zip_with(add):", vec_to_str(ref zw);   // [11, 22, 33]

  let cc: Vec<i64> = vec_concat(ref xs, ref ys);
  print "concat:", vec_to_str(ref cc);           // [1, 2, 3, 10, 20, 30]

  // extend mutates the first Vec in place and returns the new length
  // (it does NOT return a Vec -- unlike concat, which builds a fresh one).
  let grown: Vec<i64> = vec(1, 2, 3);
  let new_len: i64 = vec_extend(mut ref grown, ref ys);
  print "extend new_len:", new_len;              // 6
  print "extend result:", vec_to_str(ref grown); // [1, 2, 3, 10, 20, 30]

  let rp: Vec<i64> = vec_repeat(7, 3);
  print "repeat(7,3):", vec_to_str(ref rp);       // [7, 7, 7]

  let io: Vec<i64> = vec_iota(5);
  print "iota(5):", vec_to_str(ref io);           // [0, 1, 2, 3, 4]

  let wc: Vec<i64> = vec_with_capacity(100);
  print "with_capacity(100) starts at len:", len(ref wc) as i64;   // 0
  return 0;
}
```

| Builtin | Description |
|---------|-------------|
| `vec_zip_with(ref xs, ref ys, f: fn(i64,i64)->i64) -> Vec<i64>` | combine two vecs element-wise with `f` |
| `vec_chain(ref xs, ref ys) -> Vec<i64>` | same as `vec_concat` -- lazy-iterator naming carried over, behaves identically here |
| `vec_concat(ref xs, ref ys) -> Vec<i64>` | build a fresh `Vec` holding `xs` then `ys` |
| `vec_extend(mut ref xs, ref ys) -> i64` | append `ys` onto `xs` **in place**; returns the new length (not a Vec!) |
| `vec_repeat(v, n) -> Vec<i64>` | `n` copies of `v` |
| `vec_iota(n) -> Vec<i64>` | `[0, 1, ..., n-1]` |
| `vec_with_capacity(n) -> Vec<i64>` | empty `Vec`, pre-reserving room for `n` elements (avoids reallocation on the first `n` pushes) |

---

## Dedup and uniqueness

`dedup` (from the sorting section) only removes *consecutive*
duplicates -- these work regardless of order:

```vani
intent "dedup and uniqueness";

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
  let xs: Vec<i64> = vec(1, 1, 2, 2, 3, 4, 5);

  let uq: Vec<i64> = vec_unique(ref xs);
  print "unique:", vec_to_str(ref uq);              // [1, 2, 3, 4, 5]

  let dc: Vec<i64> = vec_dedup_consecutive(ref xs);
  print "dedup_consecutive:", vec_to_str(ref dc);   // [1, 2, 3, 4, 5]

  print "is_sorted_unique:", vec_is_sorted_unique(ref xs);   // false -- has duplicates
  return 0;
}
```

| Builtin | Description |
|---------|-------------|
| `vec_unique(ref xs) -> Vec<i64>` | every distinct value, first-occurrence order |
| `vec_dedup_consecutive(ref xs) -> Vec<i64>` | same as `dedup`'s effect, but non-mutating (returns a fresh `Vec`) |
| `vec_is_sorted_unique(ref xs) -> bool` | true if sorted ascending AND no duplicates |
| `vec_reverse_copy(ref xs) -> Vec<i64>` | non-mutating version of `reverse` |

---

## Sortedness and pattern checks

```vani
intent "sortedness checks";

fn main() -> i64 {
  let xs: Vec<i64> = vec(1, 2, 3, 4, 5);
  print "all_equal:", vec_all_equal(ref xs);        // false
  print "is_sorted_asc:", vec_is_sorted_asc(ref xs);   // true
  print "is_sorted_desc:", vec_is_sorted_desc(ref xs); // false

  let pal: Vec<i64> = vec(1, 2, 1);
  print "is_palindrome:", vec_is_palindrome(ref pal);  // true
  return 0;
}
```

| Builtin | Description |
|---------|-------------|
| `vec_all_equal(ref xs) -> bool` | every element is the same value |
| `vec_is_sorted_asc(ref xs) -> bool` / `vec_is_sorted_desc(ref xs) -> bool` | already sorted, without sorting a copy to check |
| `vec_is_palindrome(ref xs) -> bool` | reads the same forwards and backwards |

---

## Sliding windows and chunking

`vec_sliding_*` reduce each window to one number; `vec_windows`
keeps every window intact as its own `Vec`:

```vani
intent "sliding windows and chunks";

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
  let xs: Vec<i64> = vec(1, 2, 3, 4, 5, 6, 7);

  let sm: Vec<i64> = vec_sliding_max(ref xs, 2);
  print "sliding_max(2):", vec_to_str(ref sm);   // [2, 3, 4, 5, 6, 7]
  let ss: Vec<i64> = vec_sliding_sum(ref xs, 2);
  print "sliding_sum(2):", vec_to_str(ref ss);   // [3, 5, 7, 9, 11, 13]

  // vec_chunks: non-overlapping groups of size k (last one may be short)
  let ch: Vec<Vec<i64>> = vec_chunks(ref xs, 3);
  print "chunks(3) count:", len(ref ch) as i64;      // 3 -- [1,2,3] [4,5,6] [7]
  let chunk0: Vec<i64> = clone_at(ref ch, 0);
  print "chunk 0 len:", len(ref chunk0) as i64;       // 3

  // vec_windows: overlapping groups of size k, one per starting position
  let wn: Vec<Vec<i64>> = vec_windows(ref xs, 3);
  print "windows(3) count:", len(ref wn) as i64;      // 5 (7 - 3 + 1)

  // vec_flatten: the inverse of chunks/windows -- Vec<Vec<i64>> -> Vec<i64>
  let fl: Vec<i64> = vec_flatten(ref ch);
  print "flatten(chunks) len:", len(ref fl) as i64;   // 7 -- back to the original length

  // vec_group_by_value: chunks of equal consecutive-or-not values
  let grp: Vec<i64> = vec(1, 1, 2, 2, 2, 3);
  let gb: Vec<Vec<i64>> = vec_group_by_value(ref grp);
  print "group_by_value groups:", len(ref gb) as i64;   // 3
  return 0;
}
```

Since `Vec<Vec<i64>>`'s element type (`Vec<i64>`) isn't `Copy`,
indexing it directly (`ch[0]`) is rejected -- use `clone_at(ref ch,
i)` to get an owned copy of one inner `Vec`, same rule as any other
non-Copy element (see the affine ownership chapter).

| Builtin | Description |
|---------|-------------|
| `vec_sliding_max(ref xs, k)` / `vec_sliding_min(ref xs, k)` | max/min of each `k`-wide window |
| `vec_sliding_sum(ref xs, k)` / `vec_sliding_product(ref xs, k)` | sum/product of each window |
| `vec_chunks(ref xs, k) -> Vec<Vec<i64>>` | non-overlapping `k`-sized groups |
| `vec_windows(ref xs, k) -> Vec<Vec<i64>>` | overlapping `k`-sized windows, one per position |
| `vec_flatten(ref xss: Vec<Vec<i64>>) -> Vec<i64>` | concatenate all inner vecs |
| `vec_group_by_value(ref xs) -> Vec<Vec<i64>>` | group consecutive equal values together |

---

## Elementwise scalar arithmetic

Apply the same operation to every element -- the "SIMD-flavored"
builtins, all `(ref xs, scalar) -> Vec<i64>`:

```vani
intent "elementwise scalar arithmetic";

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

  let added: Vec<i64> = vec_add_scalar(ref xs, 10);
  print "add_scalar(10):", vec_to_str(ref added);   // [11, 12, 13, 14, 15]
  let mulled: Vec<i64> = vec_mul_scalar(ref xs, 3);
  print "mul_scalar(3):", vec_to_str(ref mulled);     // [3, 6, 9, 12, 15]
  let clamped: Vec<i64> = vec_clamp_scalar(ref xs, 2, 4);
  print "clamp_scalar(2,4):", vec_to_str(ref clamped);  // [2, 2, 3, 4, 4]

  let neg: Vec<i64> = vec(0 - 3, 1, 0 - 4);
  let ab: Vec<i64> = vec_abs(ref neg);
  print "abs:", vec_to_str(ref ab);         // [3, 1, 4]
  let ng: Vec<i64> = vec_negate(ref neg);
  print "negate:", vec_to_str(ref ng);   // [3, -1, 4]
  let sg: Vec<i64> = vec_signum(ref neg);
  print "signum:", vec_to_str(ref sg);   // [-1, 1, -1]
  let sq: Vec<i64> = vec_square(ref neg);
  print "square:", vec_to_str(ref sq);   // [9, 1, 16]
  return 0;
}
```

| Builtin | Description |
|---------|-------------|
| `vec_add_scalar` / `vec_sub_scalar` / `vec_mul_scalar` / `vec_div_scalar` / `vec_mod_scalar` `(ref xs, k) -> Vec<i64>` | `xs[i] OP k` for every element |
| `vec_pow_scalar(ref xs, k) -> Vec<i64>` | `xs[i]^k` for every element |
| `vec_shl_scalar` / `vec_shr_scalar` `(ref xs, k) -> Vec<i64>` | shift every element left/right by `k` bits |
| `vec_min_with_scalar` / `vec_max_with_scalar` `(ref xs, k) -> Vec<i64>` | `min`/`max` of each element against `k` |
| `vec_clamp_scalar(ref xs, lo, hi) -> Vec<i64>` | clamp every element into `[lo, hi]` |
| `vec_abs` / `vec_negate` / `vec_signum` / `vec_square` `(ref xs) -> Vec<i64>` | per-element `abs`/negate/`signum`/square, no scalar argument |

---

## Comparison masks and pairwise operations

Masks compare every element against a scalar, producing a `Vec<i64>`
of `0`/`1` (not `Vec<bool>`); pairwise operations combine two vecs
element-by-element, index-for-index:

```vani
intent "masks and pairwise ops";

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
  let eqm: Vec<i64> = vec_eq_mask(ref xs, 3);
  print "eq_mask(3):", vec_to_str(ref eqm);   // [0, 0, 1, 0, 0]
  let ltm: Vec<i64> = vec_lt_mask(ref xs, 3);
  print "lt_mask(3):", vec_to_str(ref ltm);   // [1, 1, 0, 0, 0]

  let a: Vec<i64> = vec(1, 2, 3);
  let b: Vec<i64> = vec(10, 20, 30);
  let apw: Vec<i64> = vec_add_pairwise(ref a, ref b);
  print "add_pairwise:", vec_to_str(ref apw);   // [11, 22, 33]
  let mpw: Vec<i64> = vec_mul_pairwise(ref a, ref b);
  print "mul_pairwise:", vec_to_str(ref mpw);   // [10, 40, 90]
  let mnpw: Vec<i64> = vec_min_pairwise(ref a, ref b);
  print "min_pairwise:", vec_to_str(ref mnpw);   // [1, 2, 3]
  return 0;
}
```

| Builtin | Description |
|---------|-------------|
| `vec_eq_mask` / `vec_ne_mask` / `vec_lt_mask` / `vec_le_mask` / `vec_gt_mask` / `vec_ge_mask` `(ref xs, k) -> Vec<i64>` | `1` where the comparison holds, `0` elsewhere |
| `vec_add_pairwise` / `vec_sub_pairwise` / `vec_mul_pairwise` `(ref a, ref b) -> Vec<i64>` | `a[i] OP b[i]` for every index |
| `vec_min_pairwise` / `vec_max_pairwise` `(ref a, ref b) -> Vec<i64>` | element-wise min/max between two vecs |

---

## Rotate, shift, and padding

```vani
intent "rotate, shift, padding";

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

  let rl: Vec<i64> = vec_rotate_left(ref xs, 2);
  print "rotate_left(2):", vec_to_str(ref rl);   // [3, 4, 5, 1, 2]
  let rr: Vec<i64> = vec_rotate_right(ref xs, 2);
  print "rotate_right(2):", vec_to_str(ref rr); // [4, 5, 1, 2, 3]
  let sl: Vec<i64> = vec_shift_left(ref xs, 2);
  print "shift_left(2):", vec_to_str(ref sl);     // [3, 4, 5, 0, 0] -- zero-filled, not wrapped

  let pl: Vec<i64> = vec_pad_left(ref xs, 8, 0);
  print "pad_left(8,0):", vec_to_str(ref pl);    // [0, 0, 0, 1, 2, 3, 4, 5]
  let pr: Vec<i64> = vec_pad_right(ref xs, 8, 0);
  print "pad_right(8,0):", vec_to_str(ref pr);  // [1, 2, 3, 4, 5, 0, 0, 0]
  let rv: Vec<i64> = vec_replace_value(ref xs, 3, 99);
  print "replace_value(3,99):", vec_to_str(ref rv);  // [1, 2, 99, 4, 5]
  return 0;
}
```

| Builtin | Description |
|---------|-------------|
| `vec_rotate_left` / `vec_rotate_right` `(ref xs, n) -> Vec<i64>` | rotate elements by `n` positions -- nothing is lost |
| `vec_shift_left` / `vec_shift_right` `(ref xs, n) -> Vec<i64>` | shift by `n` positions -- vacated slots become `0` |
| `vec_pad_left` / `vec_pad_right` `(ref xs, target_len, fill) -> Vec<i64>` | pad up to `target_len` with `fill` (no-op if already long enough) |
| `vec_replace_value(ref xs, old, new) -> Vec<i64>` | non-mutating version of `vec_replace_all` |

---

## Merging and inserting into sorted vecs

```vani
intent "merge and insert sorted";

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
  let a: Vec<i64> = vec(1, 3, 5);
  let b: Vec<i64> = vec(2, 4, 6);

  let ms: Vec<i64> = vec_merge_sorted(ref a, ref b);
  print "merge_sorted:", vec_to_str(ref ms);         // [1, 2, 3, 4, 5, 6]

  let ins: Vec<i64> = vec_insert_sorted(ref a, 4);
  print "insert_sorted(4):", vec_to_str(ref ins);    // [1, 3, 4, 5]

  print "range_span:", vec_range_span(ref a);        // 4 (max - min)
  return 0;
}
```

| Builtin | Description |
|---------|-------------|
| `vec_merge_sorted(ref a, ref b) -> Vec<i64>` | merge two ALREADY-SORTED vecs into one sorted vec, O(n+m) |
| `vec_insert_sorted(ref xs, v) -> Vec<i64>` | insert `v` into an already-sorted `xs`, keeping it sorted |
| `vec_range_span(ref xs) -> i64` | `max(xs) - min(xs)` in one call |
| `vec_intersperse(ref xs, sep) -> Vec<i64>` | insert `sep` between every pair of elements: `[1,2,3]` -> `[1, sep, 2, sep, 3]` |

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

**Out-of-bounds `i`:** all four index-taking mutators above
(`vec_remove_at`, `insert`, `swap_remove`, and `clone_at` from
elsewhere in this guide) trap cleanly and consistently on both
backends when `i` is out of range -- the same convention plain
indexing (`xs[i]`) uses. There's no silent garbage read and no
undefined behavior to worry about; an out-of-range index is a hard
stop, not a value you have to defensively check for.

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
