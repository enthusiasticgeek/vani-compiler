# Beginner 6a — Pointers and references (intuition primer)

> **Learning goal**: build a mental model of "pointers" and
> "references" using everyday analogies, BEFORE the
> intermediate-track chapter formalizes them. If you've never
> heard either term, start here. If you already know what a
> Rust borrow is, skip to *Intermediate 3 — Affine ownership*.

This chapter has **no compiler examples**. It's pure intuition.
The next chapter (Vec and arrays) uses values; the intermediate
chapter `03_affine.md` uses references. Read this one first to
have the right pictures in your head when you get there.

## A house and its address

Imagine you live in a house. The house has an **address** —
`42 Oak Street`. The house is a real, physical thing: it has
walls, a kitchen, your stuff inside. The address is just a
piece of writing on a piece of paper. It tells someone WHERE to
find the house, but it isn't the house itself.

If you want a delivery to arrive at your house, you can do one
of two things:

1. **Bring the house to the delivery service**. (Obviously
   impossible — the house is enormous, heavy, fixed in place.)
2. **Send them your address**. They use the address to find the
   house and bring the delivery to it.

When programs work with data, they face the same choice. Data
can be small (a single number — easy to copy around) or large
(a list of a million items, a 4 MB image, a whole struct with
many fields — copying it everywhere would be wasteful and
slow). For large data, programs do what you'd do with a house:
they pass around **addresses**, not the data itself.

> **The vocabulary**:
> - The **value** is the actual thing in memory (the house).
> - A **pointer** or **reference** is just an address — a small
>   number that tells the computer where to find the value.

## Why does this matter?

Two reasons, and you'll hit both in the next few chapters:

### 1. Speed

Copying a 4 MB image takes time. Copying its address takes a
single CPU cycle (an address is typically 8 bytes — same size
as `i64`). Functions that take large arguments take a reference
to them so the call doesn't need to copy the whole thing.

```vani
// SLOW (hypothetical — would copy a million items per call)
fn count_admins(users: Vec<User>) -> i64 { ... }

// FAST (passes the address; the function reads from the
// original storage)
fn count_admins(users: ref Vec<User>) -> i64 { ... }
```

### 2. Letting a function MODIFY the caller's data

If a function gets a *copy*, it can change the copy all it
wants but the caller's original is untouched. If you want a
function to change the caller's actual data, you must give it
the **address** of the caller's data, so it can reach back and
modify it.

```vani
// `who` is a copy. The function changing it doesn't change
// the caller's variable.
fn rename(who: Str) -> i64 { ... }

// `who` is a mutable reference — an address the function can
// reach through to modify the caller's original data.
fn rename(who: mut ref OwnedStr) -> i64 { ... }
```

This is the difference between **read-only access** (`ref T`)
and **read-write access** (`mut ref T`). vāṇी uses two
keywords; many other languages use punctuation (`&T` vs `&mut T`
in Rust; `const T*` vs `T*` in C).

## A simpler analogy: a library card

If `house ↔ address` feels abstract, try this one.

You walk into a library. There's a book you want, but you also
want your friend to be able to read it later. Two options:

1. **Give your friend the book**. Now your friend has it; you
   don't. If you want to read it again, you need it back.
2. **Tell your friend the shelf location** (Aisle 3, Shelf 5,
   Position 12). Both of you can find the book whenever you
   want. Nobody has to physically hand it over.

In option 1, the book is **moved** from you to your friend. In
option 2, both of you have **references** to the book; the book
itself stays on the shelf.

Programming languages with **affine ownership** (vāṇी, Rust)
take both options seriously. You can MOVE a value (option 1)
or BORROW it via a reference (option 2). The compiler enforces
that you never accidentally do both at once in conflicting
ways.

## Stack vs heap (one more piece of vocabulary)

You'll see two memory regions in compiler-talk:

- **Stack**: like a stack of plates. Each function call pushes
  a plate (its local variables) on top; when the function
  returns, the plate is popped off. Fast, but plates have to
  be small and have fixed sizes.
- **Heap**: like a warehouse. Things go in at no particular
  order, can be any size, and stay until someone explicitly
  removes them. Slower to access, but flexible.

A `Vec<i64>` lives partly on the stack (a small handle: pointer
+ length + capacity) and partly on the heap (the actual array of
numbers). When you take a `ref Vec<i64>`, you're getting a
pointer to that small handle on the stack — which itself points
to the heap data.

You don't need to know which is which to USE vāṇी. The compiler
handles all of it. But when you read words like "heap-allocated"
in the next chapter, this is what they mean.

## A summary you can carry

- A **value** is the data itself.
- A **pointer** / **reference** is just an address pointing AT
  the data. Small, cheap to copy.
- Use references when (a) the data is large and copying would
  be wasteful, or (b) you want a function to *modify* the
  caller's data.
- vāṇी distinguishes **read-only** (`ref T`) from
  **read-write** (`mut ref T`) references — both are addresses,
  but the compiler enforces what you're allowed to do through
  each.
- Values mostly live on the **stack** (fast, bounded);
  large/dynamic things live on the **heap** (flexible).

That's it. You haven't typed any code yet — that's intentional.
The next chapter uses values; *Intermediate 3* introduces the
actual syntax. With the mental model from this chapter, both
will feel a lot less mysterious.

## What about ownership? Affinity? "Drop"?

You'll meet those names in the intermediate track. For now,
just remember:

- **Ownership** is the language's rule that says "this value
  has exactly one place responsible for cleaning it up". When
  the owning binding goes out of scope, the value is freed.
- **Affinity** is the formal name for "every value has at most
  one owner at a time" (it's affine — used at most once). It's
  the property that lets the compiler track moves cleanly.
- **Drop** is the act of cleanup — running the value's
  destructor + freeing its heap memory. It happens automatically
  at scope-exit; you don't write it yourself.

These three together explain why vāṇी doesn't have a garbage
collector AND doesn't make you remember to `free()` anything.
The compiler tracks ownership at compile time and inserts the
right cleanup automatically.

You'll see this work in practice once you start writing code
with `Vec`, `OwnedStr`, and structs.
