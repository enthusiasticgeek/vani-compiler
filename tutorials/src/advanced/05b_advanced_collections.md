# Advanced 5b -- Advanced collections: Graph, BST, Trie, SkipList, UnionFind, BloomFilter, Deque, BTreeSet, BTreeMap, BinaryHeap

> **Learning goal**: reach for the right built-in data structure
> for graph problems, prefix matching, ordered sets, disjoint
> sets, probabilistic membership, double-ended queues, sorted
> maps, and priority queues -- all with affine ownership and no
> manual memory management.

> **Prerequisites**: [Intermediate 14 -- HashMap & HashSet](../intermediate/14_collections.md)
> and [Intermediate 3b -- Affine ownership](../intermediate/03b_affine_deeper_primer.md).

---

## Before you dive in: ten everyday shapes

Each structure below solves one recognizable everyday problem. Keep
these pictures in mind while reading the code -- the API calls are
just that picture, spelled out:

- **Graph** -- a subway map. Stations are nodes, the lines
  connecting them are edges, and a trip's travel time is the edge's
  weight. `dijkstra` is "what's the fastest way from this station
  to that one," the same question a transit app answers for you.
- **BST** -- a phone book that reorganizes itself as you add names,
  while always staying alphabetically sorted and evenly balanced
  from left to right, so a lookup never has to search one lopsided
  side of the shelf.
- **Trie** -- the letter-by-letter narrowing of an old rotary
  directory or a game of 20-questions-by-prefix: typing "C", then
  "CA", then "CAT" walks you down a shared branch that every word
  starting with "CAT" hangs off of -- the engine behind autocomplete.
- **SkipList** -- an express train system layered over a local one.
  Most trips start on the express line, which skips dozens of stops
  at once, then drop to the local line only for the last short
  stretch -- much faster on average than a purely local ride, without
  needing a rigid schedule (it's randomized, not perfectly planned).
- **UnionFind** -- tracking friend groups at a party as introductions
  happen. Each "these two just became friends" merges their two
  circles into one; the structure can instantly answer "are these
  two people in the same friend group now?" without re-tracing every
  introduction.
- **BloomFilter** -- a bouncer's rapid-fire memory check using a
  handful of stamps rather than a full guest list: it can confidently
  say "definitely not on the list," or "probably on the list, worth a
  closer look" -- but it will never wrongly turn away someone who
  really is on the list.
- **Deque** -- a buffet line you're allowed to join or leave from
  *either* end, front or back, instead of only the back.
- **BTreeSet** -- like `HashSet<T>`, but the members come out
  sorted whenever you ask for a range, and "give me everything
  between 10 and 20" is a single fast call instead of a full scan.
- **BTreeMap** -- a filing cabinet where the folders are kept in
  key order, so "show me every folder labeled 100 through 200"
  pulls a contiguous slice instead of checking every folder.
- **BinaryHeap** -- the fast-lane line at a hospital triage desk:
  whoever needs attention most (the smallest value) is always at
  the front, no matter what order patients walked in.

## Graph -- weighted directed graph

```vani
intent "Graph example";

fn main() -> i64 {
  // graph_new(num_nodes) -- nodes are i64 indices in [0, n)
  let g: Graph = graph_new(5);

  // add_edge(src, dst, weight) -- directed, weighted
  let _ = g.add_edge(0, 1, 4);
  let _ = g.add_edge(0, 2, 1);
  let _ = g.add_edge(2, 1, 2);
  let _ = g.add_edge(1, 3, 1);
  let _ = g.add_edge(3, 4, 3);

  print "nodes:", g.num_nodes();
  print "edges:", g.num_edges();
  print "bfs reach from 0:", g.bfs_reach(0);    // 5 (all nodes)
  print "dfs reach from 0:", g.dfs_reach(0);    // 5

  // dijkstra returns Option<i64> -- None if unreachable
  let dist: Option<i64> = g.dijkstra(0, 4);
  return match dist {
    Option.Some(d) then { print "shortest 0->4:", d; 0 },
    Option.None     then { print "unreachable"; 1 },
  };
}
```

### Graph API

| Builtin | Signature | Description |
|---------|-----------|-------------|
| `graph_new` | `(n: i64) -> Graph` | `n` nodes, no edges |
| `add_edge` | `(mut ref g, src, dst, w: i64) -> i64` | add directed edge, return new edge count |
| `num_nodes` | `(ref g) -> i64` | node count |
| `num_edges` | `(ref g) -> i64` | edge count |
| `bfs_reach` | `(ref g, start: i64) -> i64` | BFS reachable node count |
| `dfs_reach` | `(ref g, start: i64) -> i64` | DFS reachable node count |
| `dijkstra` | `(ref g, src, dst: i64) -> Option<i64>` | shortest weighted path; `None` if unreachable |

Method-sugar: `g.add_edge(...)`, `g.bfs_reach(...)`, etc.
Scope-exit Drop frees the three edge arrays.

---

## BST -- binary search tree (AVL self-balancing)

```vani
intent "BST example";

fn main() -> i64 {
  let b: Bst<i64> = bst_new();

  let _ = b.insert(5);
  let _ = b.insert(3);
  let _ = b.insert(7);
  let _ = b.insert(1);

  print "contains 3:", b.contains(3);   // true
  print "contains 6:", b.contains(6);   // false
  print "len:", b.len();                // 4
  // `print` can't take an Option<T> (or any enum) directly --
  // confirmed by testing ("cannot print an enum directly") --
  // unwrap it first.
  print "min:", option_unwrap_or(b.min(), -1);   // 1
  print "max:", option_unwrap_or(b.max(), -1);   // 7

  let _ = b.remove(3);
  print "after remove 3, len:", b.len(); // 3
  return 0;
}
```

### BST API

| Builtin | Signature | Description |
|---------|-----------|-------------|
| `bst_new` | `() -> Bst<i64>` | empty tree |
| `insert` | `(mut ref b, x: i64) -> bool` | insert; `true` if new |
| `contains` | `(ref b, x: i64) -> bool` | membership test |
| `remove` | `(mut ref b, x: i64) -> bool` | remove; `true` if found |
| `len` | `(ref b) -> i64` | element count |
| `min` | `(ref b) -> Option<i64>` | smallest element |
| `max` | `(ref b) -> Option<i64>` | largest element |

AVL rotations keep height O(log n) even on sorted insertion. Scope-exit Drop frees the arena arrays.

---

## Trie -- prefix tree

```vani
intent "Trie example";

fn main() -> i64 {
  let t: Trie = trie_new();

  let _ = t.insert("hello");
  let _ = t.insert("help");
  let _ = t.insert("world");

  print "contains 'hello':", t.contains("hello");      // true
  print "contains 'hell':", t.contains("hell");        // false (not inserted)
  print "starts_with 'hel':", t.starts_with("hel");   // true
  print "starts_with 'wor':", t.starts_with("wor");   // true
  print "words:", t.len();                             // 3
  return 0;
}
```

### Trie API

| Builtin | Signature | Description |
|---------|-----------|-------------|
| `trie_new` | `() -> Trie` | empty trie |
| `insert` | `(mut ref t, s: Str) -> bool` | insert word; `true` if new |
| `contains` | `(ref t, s: Str) -> bool` | exact word match |
| `starts_with` | `(ref t, prefix: Str) -> bool` | any word starts with prefix |
| `len` | `(ref t) -> i64` | number of words |
| `node_count` | `(ref t) -> i64` | internal arena node count |

Backing: flat 256 x N child-index arena. Any nonzero byte is a valid input character. Scope-exit Drop frees the two arrays.

---

## SkipList -- probabilistic ordered set

```vani
intent "SkipList example";

fn main() -> i64 {
  let sl: SkipList = skiplist_new();

  let _ = sl.insert(10);
  let _ = sl.insert(5);
  let _ = sl.insert(20);
  let _ = sl.insert(5);   // duplicate -- returns false

  print "len:", sl.len();           // 3
  print "contains 5:", sl.contains(5);    // true
  print "contains 7:", sl.contains(7);    // false
  // Same as Bst above -- print can't take Option<T> directly.
  print "min:", option_unwrap_or(sl.min(), -1);   // 5
  print "max:", option_unwrap_or(sl.max(), -1);   // 20
  return 0;
}
```

### SkipList API

| Builtin | Signature | Description |
|---------|-----------|-------------|
| `skiplist_new` | `() -> SkipList` | empty ordered set |
| `insert` | `(mut ref sl, x: i64) -> bool` | insert; `true` if new |
| `contains` | `(ref sl, x: i64) -> bool` | O(log n) membership |
| `len` | `(ref sl) -> i64` | element count |
| `min` | `(ref sl) -> Option<i64>` | smallest |
| `max` | `(ref sl) -> Option<i64>` | largest |

MAX_LEVEL = 8; LCG-based random level selection. Scope-exit Drop frees the three backing arrays.

---

## UnionFind -- disjoint-set with path compression

```vani
intent "UnionFind example -- connected components";

fn main() -> i64 {
  // 6 nodes; each starts in its own set
  let uf: UnionFind = union_find_new(6);

  // Connect pairs to build components: {0,1,2}, {3,4}, {5}
  let _ = union_find_union(mut ref uf, 0, 1);
  let _ = union_find_union(mut ref uf, 1, 2);
  let _ = union_find_union(mut ref uf, 3, 4);

  print "components:", union_find_count(ref uf);            // 3
  print "0 and 2 connected:", union_find_connected(mut ref uf, 0, 2); // true
  print "0 and 3 connected:", union_find_connected(mut ref uf, 0, 3); // false
  return 0;
}
```

### UnionFind API

| Builtin | Signature | Description |
|---------|-----------|-------------|
| `union_find_new` | `(n: i64) -> UnionFind` | `n` singletons |
| `union_find_union` | `(mut ref uf, a, b: i64) -> bool` | merge sets; `true` if actually merged |
| `union_find_find` | `(mut ref uf, x: i64) -> i64` | root of x's set (with path compression) |
| `union_find_connected` | `(mut ref uf, a, b: i64) -> bool` | same set? |
| `union_find_count` | `(ref uf) -> i64` | number of disjoint sets |

`find` and `union` require `mut ref` because path compression mutates the parent array. `count` is read-only.

---

## BloomFilter -- probabilistic membership

```vani
intent "BloomFilter example";

fn main() -> i64 {
  // 1024 bits, 4 hash positions per element
  let bf: BloomFilter = bloom_filter_new(1024, 4);

  let _ = bf.insert(42);
  let _ = bf.insert(100);
  let _ = bf.insert(7);

  print "contains 42:", bf.contains(42);   // true  (definite)
  print "contains 99:", bf.contains(99);   // false (definite negative)
  // Note: false *positives* are possible; false *negatives* are not.

  print "bit count:", bf.len();             // 1024
  print "inserts:", bf.count();             // 3
  return 0;
}
```

### BloomFilter API

| Builtin | Signature | Description |
|---------|-----------|-------------|
| `bloom_filter_new` | `(bits: i64, hashes: i64) -> BloomFilter` | allocate bit array |
| `insert` | `(mut ref bf, x: i64) -> i64` | set hash positions; return insert count |
| `contains` | `(ref bf, x: i64) -> bool` | probabilistic membership test |
| `len` | `(ref bf) -> i64` | number of bits in the array |
| `count` | `(ref bf) -> i64` | number of insertions |

<img class="manas" src="../images/mascot/manas_mascot_caution.png" title="this code needs extra care"/>

False positives are possible (the filter may say "yes" for something never inserted). False negatives are impossible (a `false` answer is definitive). Choose `bits` ~= 10x your expected element count for a ~1% false-positive rate with 4 hashes.

---

## Deque -- double-ended queue

```vani
intent "Deque example -- sliding window";

fn main() -> i64 {
  let d: Deque<i64> = deque_new();

  let _ = deque_push_back(mut ref d, 1);
  let _ = deque_push_back(mut ref d, 2);
  let _ = deque_push_front(mut ref d, 0);

  print "len:", deque_len(ref d);         // 3   [0, 1, 2]
  // Same as Bst/SkipList above -- print can't take Option<T>
  // directly.
  print "front:", option_unwrap_or(deque_peek_front(ref d), -1); // 0
  print "back:", option_unwrap_or(deque_peek_back(ref d), -1);   // 2

  let front: Option<i64> = deque_pop_front(mut ref d);
  print "popped front:", option_unwrap_or(front, -1); // 0
  print "len after pop:", deque_len(ref d); // 2
  return 0;
}
```

### Deque API

| Builtin | Signature | Description |
|---------|-----------|-------------|
| `deque_new` | `() -> Deque<i64>` | empty ring buffer |
| `deque_push_back` | `(mut ref d, v: i64) -> i64` | append; return new len |
| `deque_push_front` | `(mut ref d, v: i64) -> i64` | prepend; return new len |
| `deque_pop_back` | `(mut ref d) -> Option<i64>` | remove + return last |
| `deque_pop_front` | `(mut ref d) -> Option<i64>` | remove + return first |
| `deque_peek_back` | `(ref d) -> Option<i64>` | peek last (no remove) |
| `deque_peek_front` | `(ref d) -> Option<i64>` | peek first (no remove) |
| `deque_len` | `(ref d) -> i64` | current element count |

O(1) amortized at both ends. Ring buffer grows by doubling. Scope-exit Drop frees the heap data buffer.

---

## BTreeSet -- sorted set with fast range queries

```vani
intent "BTreeSet example -- sorted membership + range";

fn main() -> i64 {
  let s: BTreeSet<i64> = btreeset_new();

  let _ = btreeset_insert(mut ref s, 30);
  let _ = btreeset_insert(mut ref s, 10);
  let _ = btreeset_insert(mut ref s, 20);
  let _ = btreeset_insert(mut ref s, 10);   // duplicate -- returns false

  print "len:", btreeset_len(ref s);              // 3
  print "contains 20:", btreeset_contains(ref s, 20);   // true
  print "contains 25:", btreeset_contains(ref s, 25);   // false
  // Same as Bst/SkipList/Deque above -- print can't take Option<T> directly.
  print "min:", option_unwrap_or(btreeset_min(ref s), -1);   // 10
  print "max:", option_unwrap_or(btreeset_max(ref s), -1);   // 30

  // range(lo, hi) is INCLUSIVE on both ends -- appends matches
  // (sorted ascending) to an existing Vec, doesn't return one.
  let hits: Vec<i64> = vec();
  let n: i64 = btreeset_range(ref s, 10, 20, mut ref hits);
  print "range [10,20] count:", n;    // 2
  print "range [10,20]:", hits[0], hits[1];   // 10 20

  let _ = btreeset_remove(mut ref s, 10);
  print "after remove 10, len:", btreeset_len(ref s);   // 2
  return 0;
}
```

### BTreeSet API

| Builtin | Signature | Description |
|---------|-----------|-------------|
| `btreeset_new` | `() -> BTreeSet<i64>` | empty sorted set |
| `btreeset_insert` | `(mut ref s, x: i64) -> bool` | insert; `true` if new |
| `btreeset_contains` | `(ref s, x: i64) -> bool` | membership test |
| `btreeset_remove` | `(mut ref s, x: i64) -> bool` | remove; `true` if found |
| `btreeset_len` | `(ref s) -> i64` | element count |
| `btreeset_min` | `(ref s) -> Option<i64>` | smallest element |
| `btreeset_max` | `(ref s) -> Option<i64>` | largest element |
| `btreeset_range` | `(ref s, lo, hi: i64, mut ref Vec<i64> out) -> i64` | append every `x` in `[lo, hi]` to `out`; return count appended |
| `btreeset_clear` | `(mut ref s) -> i64` | remove everything; return prior length |

Under the hood it's one sorted `Vec<i64>` -- looking a value up jumps
straight to it instead of scanning (fast), but inserting or removing
has to shuffle the rest of the array over to keep it sorted (slower
than `HashSet<T>`, which doesn't keep order). Pick `BTreeSet<i64>`
when you need sorted order or range queries; pick `HashSet<T>` when
you just need fast membership and don't care about order. v1 is
`i64`-element only. `btreeset_range` **appends** to `out` rather than
clearing it first -- pass a fresh `Vec` if you don't want prior
contents mixed in.

---

## BTreeMap -- sorted key-value map with fast range queries

```vani
intent "BTreeMap example -- sorted key-value store + range";

fn main() -> i64 {
  let m: BTreeMap<i64, i64> = btreemap_new();

  // insert returns Option<i64> -- the PREVIOUS value, or None if new.
  let prev: Option<i64> = btreemap_insert(mut ref m, 100, 1);
  print "first insert, prev:", option_unwrap_or(prev, -1);   // -1 (was None)
  let _ = btreemap_insert(mut ref m, 300, 3);
  let _ = btreemap_insert(mut ref m, 200, 2);
  let overwrite: Option<i64> = btreemap_insert(mut ref m, 100, 99);
  print "overwrite 100, prev:", option_unwrap_or(overwrite, -1);   // 1

  print "len:", btreemap_len(ref m);                             // 3
  print "get 200:", option_unwrap_or(btreemap_get(ref m, 200), -1);   // 2
  print "contains_key 400:", btreemap_contains_key(ref m, 400);       // false
  print "min_key:", option_unwrap_or(btreemap_min_key(ref m), -1);    // 100
  print "max_key:", option_unwrap_or(btreemap_max_key(ref m), -1);    // 300

  // range_keys / range_values both take [lo, hi] inclusive and
  // append to an existing Vec, same convention as btreeset_range.
  let keys: Vec<i64> = vec();
  let _ = btreemap_range_keys(ref m, 100, 200, mut ref keys);
  print "keys in [100,200]:", keys[0], keys[1];     // 100 200

  let vals: Vec<i64> = vec();
  let _ = btreemap_range_values(ref m, 100, 200, mut ref vals);
  // 99 2 -- 100's value was overwritten to 99 by the second insert above.
  print "values in [100,200]:", vals[0], vals[1];

  let removed: Option<i64> = btreemap_remove(mut ref m, 300);
  print "removed 300, was:", option_unwrap_or(removed, -1);   // 3
  return 0;
}
```

### BTreeMap API

| Builtin | Signature | Description |
|---------|-----------|-------------|
| `btreemap_new` | `() -> BTreeMap<i64, i64>` | empty sorted map |
| `btreemap_insert` | `(mut ref m, k, v: i64) -> Option<i64>` | insert/overwrite; returns previous value or `None` |
| `btreemap_get` | `(ref m, k: i64) -> Option<i64>` | look up by key (by-value copy) |
| `btreemap_contains_key` | `(ref m, k: i64) -> bool` | key membership test |
| `btreemap_remove` | `(mut ref m, k: i64) -> Option<i64>` | remove; returns removed value or `None` |
| `btreemap_len` | `(ref m) -> i64` | entry count |
| `btreemap_min_key` | `(ref m) -> Option<i64>` | smallest key |
| `btreemap_max_key` | `(ref m) -> Option<i64>` | largest key |
| `btreemap_range_keys` | `(ref m, lo, hi: i64, mut ref Vec<i64> out) -> i64` | append keys in `[lo, hi]` to `out`; return count |
| `btreemap_range_values` | `(ref m, lo, hi: i64, mut ref Vec<i64> out) -> i64` | append the matching values (same key range, same order) to `out`; return count |
| `btreemap_clear` | `(mut ref m) -> i64` | remove everything; return prior length |

Same underlying idea as `BTreeSet<i64>` above -- a sorted `keys` array
kept in step with a `values` array -- so it has the same trade-off:
fast sorted lookups and range queries, slower insert/remove than
`HashMap<K,V>`. v1 is `i64` key and `i64` value only. Like
`btreeset_range`, the two range functions **append** rather than
overwrite `out`.

---

## BinaryHeap -- priority queue (min-heap)

```vani
intent "BinaryHeap example -- priority queue";

fn main() -> i64 {
  let h: BinaryHeap<i64> = binary_heap_new();

  let _ = binary_heap_push(mut ref h, 30);
  let _ = binary_heap_push(mut ref h, 10);
  let _ = binary_heap_push(mut ref h, 20);
  let _ = binary_heap_push(mut ref h, 5);

  print "len:", binary_heap_len(ref h);   // 4
  // Min-heap: the SMALLEST value is always at the top.
  print "peek:", option_unwrap_or(binary_heap_peek(ref h), -1);   // 5

  // Pop drains in ascending order.
  let a: Option<i64> = binary_heap_pop(mut ref h);
  let b: Option<i64> = binary_heap_pop(mut ref h);
  print "pop order:", option_unwrap_or(a, -1), option_unwrap_or(b, -1);   // 5 10
  print "len after 2 pops:", binary_heap_len(ref h);   // 2
  return 0;
}
```

### BinaryHeap API

| Builtin | Signature | Description |
|---------|-----------|-------------|
| `binary_heap_new` | `() -> BinaryHeap<i64>` | empty heap |
| `binary_heap_push` | `(mut ref h, v: i64) -> i64` | insert; return new length |
| `binary_heap_pop` | `(mut ref h) -> Option<i64>` | remove + return the smallest element |
| `binary_heap_peek` | `(ref h) -> Option<i64>` | look at the smallest without removing |
| `binary_heap_len` | `(ref h) -> i64` | element count |
| `binary_heap_clear` | `(mut ref h) -> i64` | remove everything; return prior length |

`push` and `pop` both stay fast (proportional to the height of the
tree, not the number of elements) because the heap keeps itself
loosely sorted rather than fully sorted -- it only guarantees the
smallest value is at the top, not that everything is in order. v1 is
`i64`-element only, and it's always smallest-first -- for a
largest-first queue, push negated values and negate again on
pop/peek.

<img class="manas" src="../images/mascot/manas_mascot_caution.png" title="two heap APIs -- pick the right one"/>

There's a second, lighter-weight heap API that operates directly on
a plain `Vec<i64>` instead of a dedicated `BinaryHeap<i64>` value --
`heap_push`/`heap_pop`/`heap_peek`/`heapify`. Same smallest-at-the-top
ordering and speed, but no separate type to manage:

```vani
intent "Vec-backed heap -- heapify an existing Vec in place";

fn main() -> i64 {
  let xs: Vec<i64> = vec();
  xs = push(xs, 30);
  xs = push(xs, 10);
  xs = push(xs, 20);
  xs = push(xs, 5);

  // Turn an already-built Vec into heap order in one O(n) pass --
  // BinaryHeap<i64> has no equivalent; you'd have to push one at a time.
  let _ = heapify(mut ref xs);
  print "top after heapify:", option_unwrap_or(heap_peek(ref xs), -1);   // 5

  let _ = heap_push(mut ref xs, 1);
  let top: Option<i64> = heap_pop(mut ref xs);
  print "popped:", option_unwrap_or(top, -1);   // 1
  return 0;
}
```

Reach for `BinaryHeap<i64>` when the heap **is** the data structure
you're carrying around (pass it to functions, store it in a struct
field). Reach for `heap_push`/`heapify` on a raw `Vec<i64>` when you
already have the values in a `Vec` and just want heap ordering --
`heapify` turns an unsorted `Vec` into heap order in a single O(n)
pass, which `BinaryHeap<i64>` can't do (it only supports one-at-a-time
`push`).

---

## Storing these inside a `Vec<T>`

All ten structures on this page are affine, heap-owning handles just
like `Vec<i64>` or `Box<T>` -- and they can themselves be the element
type of a `Vec<T>`: `Vec<Graph>`, `Vec<UnionFind>`, `Vec<Bst<i64>>`,
and so on all work correctly, on both backends, including `push`,
scope-exit drop, and (via `set`) overwrite of an existing slot. Each
element gets its own correctly-sized generated free/clear/set bundle,
the same way a `struct` or `enum` element does.

```vani
let g1: Graph = graph_new(3);
let _ = g1.add_edge(0, 1, 5);
let graphs: Vec<Graph> = vec();
let graphs: Vec<Graph> = push(graphs, g1);
for g in ref graphs {
  print g.num_nodes();
}
// scope exit: each Graph's edge arrays are freed, then the Vec's own
// backing buffer.
```

This is a fairly recent fix (BUG-216, 2026-08-21) -- earlier compiler
versions crashed or leaked on this exact combination, since these ten
types are large, variable-sized handles and the Vec byte-size
estimator + per-element drop dispatch hadn't been extended to cover
them. See `examples/language/english/bug216_vec_of_graph.vani` for a
complete, `assert`-verified regression example, and run `vanic check
<file> --coverage` (see the [CLI reference](../beginner/00_cli_reference.md#vanic-check-filevani))
if you want to check whether a specific type/operation combination in
*your* program is one this compiler's own regression corpus actually
exercises.

## Which collection to use?

| Need | Collection |
|------|-----------|
| Membership with fast lookup -- ordered | `Bst<i64>` / `SkipList` |
| Membership -- unordered, fast | `HashSet<T>` |
| Membership -- ordered, with fast range queries | `BTreeSet<i64>` |
| Membership -- probabilistic, memory-constrained | `BloomFilter` |
| Prefix / autocomplete queries | `Trie` |
| Graph traversal + shortest paths | `Graph` |
| Connected components / cycle detection | `UnionFind` |
| FIFO queue with O(1) front + back | `Deque<i64>` |
| Key-value store -- unordered, fast | `HashMap<K,V>` |
| Key-value store -- sorted, with fast range queries | `BTreeMap<i64,i64>` |
| Priority queue / "smallest first" scheduling | `BinaryHeap<i64>` or `heap_push`/`heapify` on a `Vec<i64>` |

---

**Previous**: [Sec.5 -- The dyn vtable layout + safety boundary ->](05_vtables.md)
**Next**: [Advanced 6 -- SMT trace debugging](06_smt_debug.md)
