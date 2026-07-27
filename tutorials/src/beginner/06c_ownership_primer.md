# Beginner 6c -- Ownership and move (intuition primer)

> **Learning goal**: build the "one-owner-at-a-time" mental
> model that explains why vāṇी doesn't have a garbage collector
> AND doesn't make you free heap memory yourself. Reading order:
> [06a pointers/refs](06a_pointers_refs_primer.md) ->
> [06b heap/stack](06b_heap_vs_stack_primer.md) -> here.

This chapter has **no compiler code**. Pure intuition. The
formal version is [Intermediate 3 -- Affine ownership](../intermediate/03_affine.md).

## Recap: where we are

You know from the heap/stack chapter that:
- Small / bounded values live on the **stack** -- auto-cleaned
  when their function returns.
- Big / dynamic values (a `Vec<i64>` with a million items, an
  `OwnedStr` from a string concat) have their data on the
  **heap** -- someone must explicitly tell the system to release
  it later.

You also know vāṇी's choice for who that "someone" is: not the
programmer (manual `free`), not a garbage collector -- the
**compiler**, automatically, at compile time.

The rule the compiler uses to figure out when to free things is
**ownership**. This chapter is about that rule.

## The rule, in one sentence

**Every value has exactly one owning binding. When that
binding goes out of scope, the value is freed.**

Read it again. That's the whole thing. Everything else is
applying this rule to specific situations.

## The shared-bicycle analogy

You have a bicycle. Three things are true:

1. **You own it.** It's yours. You're responsible for it.
2. **If you give it to a friend**, they own it now. You don't.
3. **If you only let them ride it for an afternoon and they
   bring it back**, you still own it. They had it temporarily.

These map directly to vāṇी:

1. `let bike: Bike = Bike { ... };` -- `bike` is the owner.
2. `let friend_bike: Bike = bike;` -- ownership **moves** from
   `bike` to `friend_bike`. Now `bike` is invalid; reading it
   is a compile error.
3. `let n: i64 = inspect(ref bike);` -- `inspect` **borrows**
   the bike for the duration of the call. `bike` still owns
   it; using `bike` after the call is fine.

The "move" in (2) and "borrow" in (3) are the two paths a value
can take. The compiler tracks which path happened.

## Why this matters

Without the one-owner rule, you have problems:

### Problem 1: who frees it?

If TWO bindings both own the same value:

```
let a: Vec<i64> = vec(1, 2, 3);
let b: Vec<i64> = a;        // hypothetical: BOTH own?
```

When `a` goes out of scope, it calls `free()` on the data. When
`b` goes out of scope, it ALSO calls `free()` on the data --
double-free. Memory corruption. Crash.

vāṇी's rule prevents this. The `let b = a;` MOVES ownership.
`a` is now invalid; only `b` will free the data when it
exits scope. One free per allocation. Safe.

### Problem 2: the dangling pointer

If a binding's data is freed but someone still has a *pointer*
(reference) to it:

```
let r: ref Vec<i64>;
{
  let xs: Vec<i64> = vec(1, 2, 3);
  r = ref xs;
}  // xs goes out of scope -> its data is freed
print r[0];  // r points at freed memory -> undefined behavior
```

vāṇी's compiler tracks references and rejects this at compile
time -- you don't even get to run the program. The reference
isn't allowed to outlive the value it points to.

### Problem 3: forgetting to free

In languages where YOU write the `free()`, you sometimes forget
-- and your program leaks memory. Worse, you sometimes free the
SAME thing twice (problem 1) or free something that's still
being used (problem 2).

vāṇी moves all three problems from runtime crashes to
compile-time errors. If your program compiles, the cleanup is
correct.

## "Move" feels weird at first

Coming from Python or Java, you might think:

```python
a = [1, 2, 3]
b = a               # <- "they share the same list, right?"
a.append(4)
print(b)            # -> [1, 2, 3, 4]
```

In Python, `a = [...]` creates a list. `b = a` makes `b` point
at the SAME list. Both names refer to one list. Appending via
`a` changes what `b` sees. This is *shared-reference*
semantics.

vāṇी (and Rust) take a different stance: `let b: Vec<i64> = a;`
*moves the list from a to b*. `a` is now an invalid name.
You'd write your program differently if you want both names
to mutate the same data -- either pass a `mut ref` around, or
restructure.

Why this design? Because *shared mutation* is the source of
most multi-threading bugs (race conditions) AND most subtle
program errors ("I changed this list over here, why is THAT
function seeing the old value?"). vāṇी opts out of shared
mutation by default; you opt back in explicitly with
references.

## What "moves" and what doesn't

**Types that move on assignment** (the value transfers; the
source is invalidated):
- `Vec<T>`, `OwnedStr`, `Box<T>`, `HashMap`, `HashSet`, etc. --
  heap-owning types
- User-defined structs containing any of the above
- Affine types: `Task`, `Atomic`, `Mutex`, `Guard`, `Channel`

**Types that COPY on assignment** (the source stays valid;
both bindings independently own a copy):
- All scalars: `i64`, `bool`, `f64`, etc.
- `Str` (the borrowed-pointer kind; a `Str` value is just an
  address -- copying the address is cheap and creates no
  aliasing issue because Str data is read-only)
- Fixed-size arrays `[T; N]` of Copy elements
- Tuples / structs of Copy fields

The compiler knows which is which. You'll get clear errors when
you accidentally move a Copy type (rare) or try to use a moved
non-Copy value.

## When it shows up in practice

The most common shape is **function arguments**:

```vani
fn consume(xs: Vec<i64>) -> i64 {
  return xs[0];
}

fn main() -> i64 {
  let xs: Vec<i64> = vec(1, 2, 3);
  let r: i64 = consume(xs);   // xs MOVES into consume
  // print xs[1];              // <- would fail to compile;
                                //   xs is moved
  return r;
}
```

Uncomment that line and the compiler stops you:

<img class="manas" src="../images/mascot/manas_mascot_error.png" title="this code does not compile!"/>

```vani
fn consume(xs: Vec<i64>) -> i64 {
  return xs[0];
}

fn main() -> i64 {
  let xs: Vec<i64> = vec(1, 2, 3);
  let r: i64 = consume(xs);
  print xs[1];
  return r;
}
```

If you don't want to lose `xs`, take a borrow:

<img class="manas" src="../images/mascot/manas_mascot_success.png" title="this is the correct, working version"/>

```vani
fn peek(xs: ref Vec<i64>) -> i64 {
  return xs[0];
}

fn main() -> i64 {
  let xs: Vec<i64> = vec(1, 2, 3);
  let r: i64 = peek(ref xs);  // borrows; xs still owned by main
  let s: i64 = peek(ref xs);  // borrows again; still fine
  return r + s;
}
```

This pattern -- borrow when you can, move when ownership
genuinely transfers -- is how vāṇी (and Rust) code feels in
practice.

## A summary you can carry

- **Every value has exactly one owning binding.** When that
  binding goes out of scope, the value's cleanup runs.
- **Move** = ownership transfers; the source name becomes
  invalid. Default for heap-owning types.
- **Borrow** (`ref T` / `mut ref T`) = temporary access; the
  owner is unchanged. Use when a function only needs to read
  (or mutate) without taking the value forever.
- **Copy** = some types (numbers, `Str`, etc.) are
  inexpensively cloned on assignment. The source stays valid.
- This rule eliminates double-frees, dangling pointers, and
  memory leaks **at compile time**, with no GC.

That's ownership. The intermediate-track chapter on affine
ownership shows the syntax + compiler errors in detail; the
mental model you've built here is what makes those errors
read like sensible feedback instead of cryptic noise.

## "Affine"? "Linear"? Where do those words come from?

You'll see "affine ownership" in vāṇी's docs. The word "affine"
comes from logic -- *affine logic* treats each resource as
something that can be used at most once (you can drop it
unused, but you can't use it twice). vāṇी's bindings are
"affine" because each value can be moved at most once.

A related word is "linear" -- same idea but stricter: each
resource MUST be used exactly once (no dropping). vāṇी is
affine, not linear -- you can declare a value and never
explicitly consume it (the scope-exit cleanup uses it
implicitly).

You don't need these words to write vāṇी code. They're just
the formal vocabulary the docs use.

## Cross-reference

- [Beginner 6a -- pointers and references primer](06a_pointers_refs_primer.md)
- [Beginner 6b -- heap and stack primer](06b_heap_vs_stack_primer.md)
- [Intermediate 3 -- Affine ownership](../intermediate/03_affine.md)
  -- formal syntax + compiler errors + worked examples
- [Intermediate 4a -- `dyn Iface` primer](../intermediate/04a_dyn_iface_primer.md)


---

**Previous**: [Sec.6b -- Heap and stack primer ->](06b_heap_vs_stack_primer.md)
**Next**: [Sec.6 -- Strings (Str vs OwnedStr) ->](06_strings.md)
