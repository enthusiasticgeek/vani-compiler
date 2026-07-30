# Intermediate 14 -- `HashMap<K,V>` and `HashSet<T>`

> **Learning goal**: store and retrieve key->value pairs with
> `HashMap<K,V>`, use `HashSet<T>` for membership tests, and
> understand the `mut ref` discipline that both collections
> require.

Imagine a locker room where every locker has a number (the key)
and holds a value (your gym bag). `HashMap<K,V>` is that locker
room: `hashmap_get` is "open locker N and hand me what's inside",
`hashmap_insert` is "put this bag in locker N", and
`hashmap_contains_key` is "is locker N occupied?" All of these
return immediately -- no need to walk the whole room.
`HashSet<T>` is the same idea but the locker holds nothing; all
you care about is WHETHER a locker number exists in the set.

## `HashMap<K,V>` -- key/value map

### Create + insert

```vani
let m: HashMap<i64, i64> = hashmap_new();

// Returns Option<i64>: Some(old_value) on overwrite, None on first insert.
let prev: Option<i64> = hashmap_insert(mut ref m, 1, 100);
```

The `mut ref` is required because the map is mutated (possibly
resized). Forgetting `mut` is a compile error:

<img class="manas" src="../images/mascot/manas_mascot_error.png" title="this code does not compile!"/>

```vani
let scores: HashMap<i64, i64> = hashmap_new();
let prev: Option<i64> = hashmap_insert(ref scores, 1, 100);   // missing `mut`
```

```
error: hashmap_insert() requires a `mut ref HashMap<K, V>` argument, got ref HashMap<i64, i64>
  let prev: Option<i64> = hashmap_insert(ref scores, 1, 100);
                                         ^^^^^^^^^^
```

### Full API

| Builtin | Signature | Returns |
|---|---|---|
| `hashmap_new()` | `-> HashMap<K,V>` | empty map |
| `hashmap_insert(mut ref m, k, v)` | `-> Option<V>` | previous value or `None` |
| `hashmap_get(ref m, k)` | `-> Option<V>` | value or `None` |
| `hashmap_contains_key(ref m, k)` | `-> bool` | membership test |
| `hashmap_remove(mut ref m, k)` | `-> Option<V>` | removed value or `None` |
| `hashmap_len(ref m)` | `-> i64` | number of entries |
| `hashmap_clear(mut ref m)` | `-> i64` | removes all entries (returns 0) |

### Worked example

```vani
intent "Intermediate 14 -- HashMap<i64, i64> basics.";

fn main() -> i64 {
  let scores: HashMap<i64, i64> = hashmap_new();

  // Store scores for players 1, 2, 3.
  let _ = hashmap_insert(mut ref scores, 1, 42);
  let _ = hashmap_insert(mut ref scores, 2, 87);
  let _ = hashmap_insert(mut ref scores, 3, 55);

  // Retrieve a score; default to 0 if not found.
  let s1: i64 = option_unwrap_or(hashmap_get(ref scores, 1), 0);
  let s4: i64 = option_unwrap_or(hashmap_get(ref scores, 4), 0);
  print "player 1 score:", s1;     // 42
  print "player 4 score:", s4;     // 0 (not found)

  // Overwrite: insert returns the old value.
  let old: Option<i64> = hashmap_insert(mut ref scores, 1, 99);
  print "player 1 old score:", option_unwrap_or(old, -1);  // 42
  print "player 1 new score:", option_unwrap_or(hashmap_get(ref scores, 1), 0); // 99

  // Existence check.
  print "has player 2:", hashmap_contains_key(ref scores, 2);  // true
  print "has player 9:", hashmap_contains_key(ref scores, 9);  // false

  // Remove.
  let removed: Option<i64> = hashmap_remove(mut ref scores, 2);
  print "removed player 2:", option_unwrap_or(removed, -1);    // 87
  print "map len:", hashmap_len(ref scores);                   // 2

  return 0;
}
```

Expected output:

```
player 1 score: 42
player 4 score: 0
player 1 old score: 42
player 1 new score: 99
has player 2: true
has player 9: false
removed player 2: 87
map len: 2
```

## Key types supported in v1

| Key type | Notes |
|---|---|
| `i64` | default; hashed via FNV-1a |
| `f64` | bit-exact equality (no NaN de-dup) |
| `OwnedStr` | string-keyed maps; use `hashmap_str.vani` pattern |
| `(i64, i64)` | tuple keys; see `hashmap_tup.vani` |
| `Vec<i64>` | vector keys; deep-equal + hash |

## `HashSet<T>` -- membership set

A `HashSet<T>` is a map with no value -- you only care whether an
element is in the set.

### API

| Builtin | Signature | Returns |
|---|---|---|
| `hashset_new()` | `-> HashSet<T>` | empty set |
| `hashset_insert(mut ref s, v)` | `-> bool` | `true` if newly inserted |
| `hashset_contains(ref s, v)` | `-> bool` | membership test |
| `hashset_remove(mut ref s, v)` | `-> bool` | `true` if removed |
| `hashset_len(ref s)` | `-> i64` | number of elements |
| `hashset_clear(mut ref s)` | `-> i64` | removes all elements |

### Worked example

```vani
intent "Intermediate 14 -- HashSet<i64> deduplication.";

fn main() -> i64 {
  let seen: HashSet<i64> = hashset_new();
  let v: Vec<i64> = vec(3, 1, 4, 1, 5, 9, 2, 6, 5, 3, 5);

  // Collect unique values.
  let i: i64 = 0;
  while i < (len(v) as i64) {
    let _ = hashset_insert(mut ref seen, v[i]);
    i = i + 1;
  }
  print "unique count:", hashset_len(ref seen);     // 7

  print "contains 4:", hashset_contains(ref seen, 4);   // true
  print "contains 7:", hashset_contains(ref seen, 7);   // false

  // Remove and re-insert.
  let removed: bool = hashset_remove(mut ref seen, 9);
  print "removed 9:", removed;                          // true
  print "contains 9:", hashset_contains(ref seen, 9);   // false

  let new_insert: bool = hashset_insert(mut ref seen, 9);
  print "re-inserted 9:", new_insert;                   // true

  return 0;
}
```

## Method-call sugar

Both `HashMap` and `HashSet` support the `.method(...)` call syntax
as shorthand:

```vani
// These pairs are equivalent:
hashmap_get(ref m, k)           // builtin form
m.get(k)                        // sugar form

hashset_insert(mut ref s, v)    // builtin form
s.insert(v)                     // sugar form
```

The sugar is documented in
`examples/language/english/container_method_sugar.vani`.

## Common patterns

**Word-frequency counter**:
```vani
// Count how many times each number appears in a list.
let freq: HashMap<i64, i64> = hashmap_new();
let i: i64 = 0;
while i < (len(data) as i64) {
  let cur: i64 = option_unwrap_or(hashmap_get(ref freq, data[i]), 0);
  let _ = hashmap_insert(mut ref freq, data[i], cur + 1);
  i = i + 1;
}
```

**Deduplication pipeline**:
```vani
let seen: HashSet<i64> = hashset_new();
let unique: Vec<i64> = vec();
let i: i64 = 0;
while i < (len(data) as i64) {
  if !hashset_contains(ref seen, data[i]) {
    let _ = hashset_insert(mut ref seen, data[i]);
    push(mut ref unique, data[i]);
  }
  i = i + 1;
}
```

## v1 limitations to keep in mind

- **No generic value types beyond the listed key types.** String
  values and tuple values work today;
  `Vec<T>` values require `hashmap_strv.vani`-style pattern.
- **No iteration over map entries.** v1 has no `for (k, v) in m`
  syntax; collect the keys into a `Vec` first if you need to walk
  all entries.
- **Tombstone-based remove.** Heavily remove-intensive workloads
  trigger a periodic rehash that reclaims tombstones. This is
  transparent; just be aware the map may reallocate.

## Challenge

Build a "two-sum" solver: given a `Vec<i64>` and a target sum,
find two indices `i` and `j` such that `v[i] + v[j] == target`.
Use a `HashMap<i64, i64>` mapping each value to its first index for
an O(n) solution.

---

**Previous**: [Sec.13 -- `Option<T>` and the option builtins ->](13_option.md)
**Next**: [Sec.15 -- Math, random numbers, and clone ->](15_math_rng.md)
