# Intermediate 3b -- Affine ownership (deeper pass)

> **Learning goal**: deepen the ownership mental model from
> [Beginner 6c](../beginner/06c_ownership_primer.md) -- what
> "affine" actually means, what the compiler is tracking
> beyond the simple "one owner at a time" rule, and which
> error messages mean what. Reading order: 6c first; then
> here; then [Intermediate 3 -- Affine ownership](03_affine.md)
> for syntax.

This chapter has **no compiler code**. Pure intuition.

## Recap

From 6c: every value has exactly one owning binding. When that
binding goes out of scope, the value is freed. You can MOVE
ownership (`let b = a;` for non-Copy types) or BORROW
temporarily (`ref a` / `mut ref a`).

This chapter goes further. Three concepts the compiler tracks
beyond simple move/borrow:

1. **Partial moves**: moving ONE FIELD out of a struct.
2. **Conditional moves**: moving in some branches but not
   others.
3. **Borrow scopes**: when do borrows END?

## "Affine" vs "linear" -- precise meanings

You met these words in chapter 6c. Here's the precise
mathematical setup, because the words come up in error
messages.

A **linear** resource MUST be used exactly once. You cannot
drop it on the floor; you cannot use it twice.

An **affine** resource can be used AT MOST once. You can
either use it (move/consume it) OR let it scope-exit naturally
(the compiler inserts the cleanup). You cannot use it twice.

vāṇी is affine. You don't have to manually consume every
non-Copy value -- the scope-exit drop handles unused ones. But
you can never use a value AFTER it's been moved.

The formal name in academic literature is *affine type
system*. vāṇी's particular variant follows Rust's design
choices in most respects.

## Partial moves

A struct can have its fields moved INDEPENDENTLY:

```vani
struct Bag { name: OwnedStr, count: i64 }

fn split(b: Bag) -> i64 {
  let owned_name: OwnedStr = b.name;   // moves b.name
  return b.count;                       // b.count still valid
}
```

After `let owned_name = b.name;`, the `name` field of `b` is
moved -- `b` is in a "partially moved" state. Reading `b.name`
again is a compile error; reading `b.count` is fine (Copy
type, never moves).

The compiler tracks which fields have been moved. The
scope-exit drop knows to skip the moved fields when running
destructors.

### What the error looks like

<img class="manas" src="../images/mascot/manas_mascot_error.png" title="this code does not compile!"/>

```
error: field 'name' of 'b' was moved at byte 47,
       cannot read after move
```

A *very* clear error from a fairly subtle situation.

## Conditional moves -- the join-point problem

What if you move a value in ONE branch of an `if`, but not
the other?

```vani
fn maybe_consume(xs: Vec<i64>, do_it: bool) -> i64 {
  let extra: i64 = 0;
  if do_it {
    let other: Vec<i64> = xs;   // moves xs
    print other[0];
  }
  // both branches fall through to here (no early return in
  // the `if` arm this time -- that distinction matters, see below)
  return xs[0] + extra;          // is xs valid here?
}
```

If `do_it == true`: xs was moved inside the `if`, then control
falls through to the shared `return` -- reading `xs[0]` there
would be a use-after-move.

If `do_it == false`: xs is fine.

The compiler can't predict at compile time which branch
runs -- but it can know that AT LEAST ONE PATH moves xs and then
*falls through* to the shared code after the `if`. So it
conservatively treats xs as "possibly moved" from that point on
-- reading xs[0] is rejected.

The fix: either move in BOTH branches, or move in NEITHER, or
(as below) make the moving branch `return` before it would ever
fall through to the shared code -- so there's no post-`if` point
where xs could be read after being moved.

<img class="manas" src="../images/mascot/manas_mascot_success.png" title="this is the correct, working version"/>

```vani
fn maybe_consume_fixed(xs: Vec<i64>, do_it: bool) -> i64 {
  if do_it {
    let other: Vec<i64> = xs;
    return other[0];
  }
  return xs[0];                  // only reaches here if !do_it
}
```

After the rearrangement, every code path either moves xs OR
returns before reading it. The compiler approves.

## Borrow scopes -- when does `ref x` end?

A borrow `ref x` doesn't last forever. It's valid for a
specific scope. The simplest case:

```vani
fn main() -> i64 {
  let xs: Vec<i64> = vec(1, 2, 3);
  let n: i64 = sum(ref xs);   // borrow lives for this call only
  // borrow over here -- xs fully owned again
  let m: i64 = sum(ref xs);   // can borrow again, no conflict
  return n + m;
}
```

Each `ref xs` is its own short-lived borrow. They don't
overlap; the compiler doesn't track them across the call --
once the call returns, the borrow ends.

The tricky case: STORING a borrow.

```vani
let xs: Vec<i64> = vec(1, 2, 3);
let r: ref Vec<i64> = ref xs;
print r[0];
print xs[0];          // <- can we still read xs while r is alive?
```

Answer: yes for **shared (read-only) borrows** like `ref`. You
can have many of them at once. They're all read-only.

The rule -- the same one Rust enforces -- is that a **mutable**
borrow must be exclusive: no reading (or writing) the original
while a `mut ref` alias to it is outstanding, and no taking a
second `ref`/`mut ref` of the same binding either.

<img class="manas" src="../images/mascot/manas_mascot_error.png" title="this code does not compile!"/>

The checker enforces this for **named `mut ref` bindings** --
a `let`-bound alias whose lifetime is its own enclosing scope,
not a one-off `foo(mut ref xs)` call argument (see below for that
distinction):

```vani
let xs: Vec<i64> = vec(1, 2, 3);
let r: mut ref Vec<i64> = mut ref xs;
push(r, 4);
print xs[0];
```

```
error: cannot use 'xs' while it is mutably borrowed by 'r'
```

The same rejection fires for a write through the other alias
(`set(mut ref xs, 0, 99);` while `r` is still live), and for
taking a second borrow -- shared or mutable -- of `xs` while `r`
holds it. The check is scoped to `r`'s own enclosing block: once
`r`'s block exits, `xs` is freely accessible again.

**What this check does NOT (yet) track**: it's a lexical
approximation, not full non-lexical-lifetime analysis --
`r`'s borrow is considered live for the rest of its *declaring
scope*, whether or not `r` is actually used again after some
earlier point. It also only sees NAMED `mut ref`/`ref` bindings;
an inline borrow passed directly as a call argument
(`push(mut ref xs, 4); print xs[0];`) is never stored anywhere,
so per the borrow-scopes rule from the top of this section its
lifetime ends the moment the call returns -- that shape compiles
cleanly, correctly, both before and after this fix.

## What "ends" a borrow?

For a let-bound borrow like `let r = ref xs;`, the borrow
ends when `r` goes out of scope. So if `r` is declared early
in main and never used again, technically it "lives" until
main's end -- making xs inaccessible for that whole stretch.

In practice the compiler is smart enough to see that `r` is
no longer used after some point and ends the borrow there
(non-lexical lifetimes, NLL). But the conservative mental
model -- "the borrow lives until the let-binding's scope ends"
-- is correct for safety reasoning.

For call-arg borrows (`sum(ref xs)`), the borrow ends as soon
as the call returns.

For struct-field borrows (`bag.contents = ref xs;` --
post-Phase 3 of L4), the borrow lives as long as the holder
struct does, and the compiler enforces that the source
outlives the holder.

## The two-way trade

Affine ownership (the move rule) eliminates a class of bugs
(use-after-free, double-free, dangling pointer) at compile time.
Data races specifically depend on the mutable-borrow-exclusivity
rule from the section above, which -- as covered there -- is
enforced for the common named-`mut ref`-binding shape, but is a
lexical approximation rather than full non-lexical-lifetime
analysis (see the section above for exactly what it does and
doesn't catch). Cross-THREAD data races are a separate, already-
independently-enforced mechanism (Copy-only task captures, no
implicit sharing across `task`/`parallel-for` boundaries without
an explicit `Mutex`/`Channel`/`Atomic`) -- see
[Advanced 2a](../advanced/02a_parallelism_primer.md) for how
vāṇी's concurrency primitives handle shared mutable state in
practice.

The cost: you have to think about WHO OWNS WHAT and pass
values around explicitly. Code that "looks fine" in
Python/JS/Java sometimes hits compile errors in vāṇી because
it would have been silently sharing state -- which would
manifest later as bugs.

In practice, after a few days of writing vāṇी, the
constraints feel natural. The compile errors are concrete and
the fixes are mechanical (clone, borrow, restructure). The
"thinking about ownership" cost is front-loaded; once your
mental model matches the compiler's, you stop hitting
ownership errors.

## A summary you can carry

- **Affine** = each value can be used AT MOST once. (Linear
  would be EXACTLY once -- vāṇी is affine, not linear, so
  unused values are auto-dropped.)
- **Partial moves**: a struct's fields can be moved
  individually. The compiler tracks which fields are moved.
- **Conditional moves**: if any path through a control-flow
  construct moves a value, the value is "possibly moved"
  after -- reads are rejected. Fix by making all paths
  consistent.
- **Borrow scopes**: shared borrows (`ref`) can multiply freely --
  confirmed enforced. The "many shared XOR one mutable"
  exclusivity rule for `mut ref` **is enforced for named
  `let`-bound borrows** (see the section above for the exact
  shape and its lexical-scope, not full-NLL, precision) -- an
  inline `foo(mut ref xs)` call argument's borrow still ends,
  untracked, the moment the call returns, matching "Borrow ends"
  below.
- **Borrow ends** when the borrow's binding goes out of
  scope (or earlier via NLL).

That's the deeper affine model. The intermediate-track
[chapter 3](03_affine.md) shows the syntax + actual compiler
diagnostics; this primer makes the compiler errors read
sensibly.

## Cross-reference

- [Beginner 6c -- Ownership primer](../beginner/06c_ownership_primer.md)
  -- the foundation; read this first
- [Intermediate 3 -- Affine ownership](03_affine.md) --
  syntax + compiler errors
- [Beginner 6a -- Pointers/references primer](../beginner/06a_pointers_refs_primer.md)
  -- refs are HOW borrows are expressed
- [Advanced 2a -- Parallelism + race-freedom primer](../advanced/02a_parallelism_primer.md)
  -- affine + the shared-XOR-mutable rule is what eliminates
  data races; this chapter explains the rule, that one applies
  it to concurrency


---

**Previous**: [Sec.3a -- `Box<T>` and RAII primer ->](03a_box_raii_primer.md)
**Next**: [Sec.3c -- Shared ownership primer ->](03c_shared_ownership_primer.md)

