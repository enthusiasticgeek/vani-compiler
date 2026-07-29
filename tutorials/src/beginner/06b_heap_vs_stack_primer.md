# Beginner 6b -- Heap and stack (intuition primer)

> **Learning goal**: build a mental model of "stack" and "heap"
> -- the two regions where your program's data lives. If you've
> just read the [pointers and references primer](06a_pointers_refs_primer.md),
> this is the natural follow-up. The two chapters together set
> up everything later chapters say about `Vec`, `OwnedStr`, and
> ownership.

This chapter has **no compiler examples**. Pure intuition. The
goal is to make the words "stack-allocated" and "heap-allocated"
feel concrete enough that you can reason about them as you read
later chapters.

## The two filing systems

Imagine a busy office. People arrive with paperwork, work on it,
finish, and leave. The office has two places to keep papers:

### The stack (a clipboard tower)

There's a tower of clipboards by the door. When a person walks
in, they grab the top clipboard from a fresh stack, write their
name and a couple of notes, and clip their paperwork to it.
When they leave, they hand the clipboard back. The top of the
tower is always the most-recent arrival. People who came in
earlier are *underneath* -- you can't reach them without
first dealing with everyone on top.

Three things matter about the clipboard tower:

1. **Insanely fast.** Grabbing a clipboard takes a moment.
   Returning it takes a moment.
2. **Strict order.** Last in, first out (LIFO). The latest
   person to arrive is the first to leave.
3. **Limited space.** Each clipboard is small. You can clip a
   few small notes -- names, phone numbers, dates. You can't
   clip a giant binder to a clipboard.

### The heap (a warehouse with a librarian)

Behind a counter at the back of the office, there's a vast
warehouse. If someone needs to store something bigger than fits
on a clipboard -- a stack of contracts, a portfolio, anything
big -- they walk to the counter and ask the librarian:

> "Hi, I need shelf space for this 200-page document."

The librarian finds an empty spot, hands the person a small
slip of paper with the shelf location (`Aisle 3, Shelf 7,
Position 12`), and the person tucks that slip into their
clipboard.

Three things matter about the warehouse:

1. **Slower** -- walking to the counter, talking to the
   librarian, finding the shelf takes more time than grabbing
   a clipboard.
2. **No order.** People can store things in any sequence; the
   librarian handles placement.
3. **Big.** The warehouse is enormous. Documents of any size
   go in.

## How a running program uses both

Your program does the same dance constantly:

```
fn greet(name: Str) -> i64 {
  let count: i64 = len(name) as i64;
  print "hello,", name;
  return count;
}
```

When `greet` runs:

1. A clipboard is grabbed (`count` and `name` get clipped onto it).
2. The body runs.
3. When `greet` returns, the clipboard is handed back -- `count`
   and `name` are gone, the next function down the line gets
   the same clipboard space.

That's the **stack** in action: each function call is a
clipboard. The function's local variables live on that
clipboard. When the function returns, the clipboard's
contents are discarded.

But what about a `Vec<i64>` with a million items? A clipboard
is too small. So vāṇी splits it:

```
The clipboard holds a small SLIP:
  - "shelf-location for the data" -> Aisle 3, Shelf 7
  - "how many items are stored"  -> 1,000,000
  - "how much shelf space is reserved" -> 1,200,000

The warehouse holds the actual million numbers.
```

The Vec's *handle* (3 small numbers: pointer + length +
capacity) lives on the stack -- fits on the clipboard. The
*data* (the million numbers) lives in the warehouse -- the
heap.

This is how every "smart pointer" / "owning container" works:
small handle on the stack, real data on the heap.

## What lives where, in vāṇी

Don't memorize this -- it'll be obvious from context. But for
reference:

**Stack (clipboard)**:
- All numeric values (`i64`, `bool`, `f64`, etc.)
- `Str` (the handle -- but the actual character bytes live in
  `.rodata`, the binary's read-only data section, a third kind
  of region covered in [Beginner 6d](06d_memory_sections_primer.md))
- Fixed-size arrays `[T; N]` when `N` is small
- Plain structs (composed of stack-living fields)
- All references / pointers (an address is a small number)

**Heap (warehouse)**:
- `Vec<T>` data (the handle is on the stack, the array is on
  the heap)
- `OwnedStr` text data
- `Box<T>` data (a box's whole job is "put this on the heap")
- Inside collections: `HashMap`, `BTreeMap`, etc.

## Why split the world this way?

You might wonder: "why not put everything on the warehouse?
Then I don't have to think about it." Three reasons:

1. **Speed.** The clipboard is cached in the CPU. The warehouse
   is across the room. Stack reads are 10-100x faster than
   heap reads for the same byte count.
2. **Cleanup.** The clipboard cleans itself -- when the function
   returns, the slot is freed automatically. Heap items need
   to be tracked: someone has to remember to give the space
   back to the librarian when done.
3. **Allocation cost.** Grabbing a clipboard is free (push the
   stack pointer). Asking the librarian for shelf space costs
   real time (the warehouse has to find a free spot, mark it
   used, return the slip).

So vāṇी puts small + bounded things on the stack (free, fast,
auto-cleaned) and only goes to the warehouse for things that
need to be big or whose size isn't known up front.

## "But who tells the librarian to give the space back?"

The answer is what makes vāṇी's design interesting.

In some languages, the *programmer* does -- they explicitly call
`free()` when done with each heap allocation. Forget, and you
leak memory. Free too early, and you crash with a dangling
pointer. C does this.

In other languages, a *garbage collector* does -- a background
process roams the heap, finds anything nobody points to
anymore, and frees it. Reliable but slow and unpredictable.
Python, Java, Go do this.

vāṇी picks a third path: the *compiler* does, automatically,
at compile time. When the variable holding the handle goes out
of scope (its clipboard slot is being returned), the compiler
inserts the `free()` call right before. No background process.
No memory leaks. No forgetting.

The rule it uses to figure out when each thing should be freed
is called **ownership** -- the next intuition primer (which is
[Intermediate 3](../intermediate/03_affine.md) in its formal
form) will explain it.

## A summary you can carry

- **Stack** = clipboard tower. Fast, strict last-in-first-out
  order, small per-item space. Holds local variables.
- **Heap** = warehouse. Slower, any order, large per-item
  space. Holds dynamic data (Vec, OwnedStr, Box).
- Most "smart" containers are a HYBRID: small handle on the
  stack, big data on the heap.
- vāṇी's compiler tracks heap items via **ownership** and
  inserts `free()` calls automatically when the owning binding
  goes out of scope. No GC, no leaks, no manual free.

That's the intuition. When the next chapter introduces `Vec`,
you'll know what "the data lives on the heap, the handle on
the stack" means. When the intermediate track introduces
ownership, you'll know what's being tracked and why.

## Cross-reference

- [Beginner 6a -- pointers and references primer](06a_pointers_refs_primer.md)
  -- the sibling chapter on addresses and references.
- [Beginner 7 -- Arrays and `Vec<T>` basics](07_vec_arrays.md)
  -- first compiler code using the handle/data split.
- [Intermediate 3 -- Affine ownership](../intermediate/03_affine.md)
  -- the formal rule the compiler uses to free heap items.
- [Beginner 6d -- Program memory layout primer](06d_memory_sections_primer.md)
  -- the sections that exist *before* the program runs:
  `.text` / `.rodata` / `.data` / `.bss`.


---

**Previous**: [Sec.6a -- Pointers and references primer ->](06a_pointers_refs_primer.md)
**Next**: [Sec.6c -- Ownership and move primer ->](06c_ownership_primer.md)
