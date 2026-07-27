# Advanced 5b -- Advanced collections: Graph, BST, Trie, SkipList, UnionFind, BloomFilter, Deque

> **Learning goal**: reach for the right built-in data structure
> for graph problems, prefix matching, ordered sets, disjoint
> sets, probabilistic membership, and double-ended queues --
> all with affine ownership and no manual memory management.

> **Prerequisites**: [Intermediate 14 -- HashMap & HashSet](../intermediate/14_collections.md)
> and [Intermediate 3b -- Affine ownership](../intermediate/03b_affine_deeper_primer.md).

---

## Before you dive in: seven everyday shapes

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
  print "min:", b.min();                // Option.Some(1)
  print "max:", b.max();                // Option.Some(7)

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
  print "min:", sl.min();          // Option.Some(5)
  print "max:", sl.max();          // Option.Some(20)
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
  print "front:", deque_peek_front(ref d); // Option.Some(0)
  print "back:", deque_peek_back(ref d);   // Option.Some(2)

  let front: Option<i64> = deque_pop_front(mut ref d);
  print "popped front:", front;            // Option.Some(0)
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

## Which collection to use?

| Need | Collection |
|------|-----------|
| Membership with fast lookup -- ordered | `Bst<i64>` / `SkipList` |
| Membership -- unordered, fast | `HashSet<T>` |
| Membership -- probabilistic, memory-constrained | `BloomFilter` |
| Prefix / autocomplete queries | `Trie` |
| Graph traversal + shortest paths | `Graph` |
| Connected components / cycle detection | `UnionFind` |
| FIFO queue with O(1) front + back | `Deque<i64>` |
| Key-value store | `HashMap<K,V>` |

---

**Previous**: [Sec.5 -- The dyn vtable layout + safety boundary ->](05_vtables.md)
**Next**: [Advanced 6 -- SMT trace debugging](06_smt_debug.md)
