# Language ergonomics gaps found via DhruvaOS

A running, append-only log of real vani-language limitations found
while writing DhruvaOS (a bare-metal Raspberry Pi kernel,
`~/source/dhruvaos`) — specifically the from-scratch Pi 4/5
(AArch64) port, `kernel/kernel_main_rpi4.vani`. Distinct from
`docs/TODO_CURRENT.md`'s BUG-N entries: those are *bugs* (compiler
does something wrong); everything here is a missing *feature* or
ergonomic gap (compiler does nothing, forcing a workaround) found by
a real, large, external, freestanding consumer of the language.
Kept separate so this specific feedback channel — "what does a real
systems-programming user hit that a language client should have
covered?" — doesn't get lost in the day-to-day bugfix log.

**How to use this list**: each entry has a real file:line reference
into `src/` for where the gap lives, a concrete DhruvaOS call site
that hit it, and the workaround actually shipped (so a future fix
can judge whether the workaround becomes unnecessary or just
redundant). Pick items up in any order — none block each other.

---

## 1. No array-repeat literal syntax (`[expr; N]`)

**Found**: round 86 of the Pi 4/5 port (loopback netif abstraction,
2026-09-06), writing a 512-byte zero-initialized frame buffer.

**Gap**: `src/parser.rs`'s array-literal parser (`parse_primary_expr`,
the `TokenKind::LBracket` arm, ~line 6491-6512) only ever accepts a
comma-separated explicit element list — there is no `[expr; N]`
repeat form at all in expression position. Confirmed empty-handed via
`grep -rn "ArrayRepeat" src/` (no such variant exists anywhere in the
AST) and via a live `vanic check` probe:

```vani
fn zero4() -> [u8; 4] {
  return [0 as u8; 4];   // parse error -- no such syntax
}
```

Every one of Rust's, C99's (with `= {0}` idiom), and Zig's array
languages has some form of this; vani currently forces the user to
hand-enumerate every element, which becomes actively unworkable past
a few dozen elements (DhruvaOS needed a 512-element zero literal for
a netif frame buffer, and Ethernet's real MTU would need 1514).

**Interesting existing internal precedent**: the compiler already
knows how to synthesize exactly this shape internally, just not as
user-facing syntax — see `v31_default_init_expr` in `src/parser.rs`
(~line 7986-7993, "Phase 3f -- `[T; N]`: ArrayLit with N copies of
default(T)"), used only for missing-struct-field default synthesis.
Exposing a `[expr; N]` front-end form that lowers to the same
"N copies" `ArrayLit` this internal helper already builds looks like
a small, well-precedented addition rather than new compiler
machinery — the desugaring target already exists, only the surface
syntax and its parse/typecheck entry point are missing. `expr` would
need to be evaluated once and copied `N` times (for `Copy` element
types only, matching how `[T; N]` locals are already documented as
genuine `Copy` stack values elsewhere in this repo's own docs) or
require `expr` to be a compile-time constant, either is fine —
whichever is the smaller diff.

**Workaround shipped**: a one-off helper function returning the
fully-enumerated literal by value (confirmed via a standalone probe
that a function CAN return a fixed-array type by value — no existing
test in this repo's own suite did this before, but nothing in the
parser/checker rejects it either). See DhruvaOS commit `a9d189d`,
`kernel_main_rpi4.vani`'s `netif_zero_frame_rpi4() -> [u8; 512]`. One
512-element hand-typed literal, called everywhere a zeroed frame
buffer is needed. Chose 512 bytes specifically BECAUSE of this gap
(deliberately smaller than a real 1514-byte Ethernet MTU) — the
missing feature directly shaped a real design decision downstream,
not just a one-time inconvenience.

---

## 2. `let` always requires a full initializer — no uninitialized declaration

**Found**: same round 86 session, immediately adjacent to gap #1
above (the two compound each other).

**Gap**: `src/parser.rs`'s `parse_let_stmt` (~line 4914 onward)
unconditionally calls
`self.expect_keyword("'='", |kind| matches!(kind, TokenKind::Equal))?`
after the optional type annotation — there is no path through this
function that allows `let x: T;` without `= expr`. Confirmed by
reading the function body directly (every branch, including the
destructure-`let` form, requires `=`).

This means a fixed-size local array meant purely as scratch space
(contents to be filled by a loop or a called function immediately
after) has no way to be declared without ALSO providing some initial
value — compounding gap #1 above, since that initial value then also
can't use a repeat-literal shorthand. On a bare-metal target with no
heap allocator (DhruvaOS's own Pi 4/5 port has none, by design), this
pattern — "give me N bytes of scratch, I'll fill it myself" — is
extremely common (every frame buffer, every receive buffer, every
hash-output buffer in `kernel_main_rpi4.vani` needs it).

**Design note for whoever picks this up**: an uninitialized `let`
is memory-safety-relevant (reading before writing would be real
undefined behavior on the C backend, since C locals aren't
zero-initialized by default) — so this likely wants either (a) the
checker enforcing definite-assignment-before-use for any `let`
declared without an initializer (a real, non-trivial dataflow
analysis, but the kind of thing Rust already does for exactly this
reason), or (b) restricting the no-initializer form to only array/
struct types with a well-defined zero value and having the C/LLVM
backends actually emit a zero-fill, sidestepping the analysis
entirely at the cost of a wasted memset the caller didn't need. (b)
composes directly with fixing gap #1 above (implement `[expr; N]`
generally, then `let x: [T; N];` desugars to `let x: [T; N] =
[<T's zero value>; N];` and gap #2 becomes free).

**Workaround shipped**: same as gap #1 — a helper function
supplying the (still fully-enumerated) initial value, so `let`'s
existing "always has an initializer" rule is satisfied trivially.

---

## 3. No reborrow from `mut ref T` to `ref T`

**Found**: round 88 of the Pi 4/5 port (a real shared IPv4 header
module + packet filter, 2026-09-06), writing `ipv4_build_header_rpi4`
and the packet-filter ingress hook in `netif_recv_frame_rpi4`.

**Gap**: a value already held as a `mut ref T` function parameter
cannot be passed to a callee expecting a plain `ref T`, even though a
`mut ref` is a strictly more capable/permissive access than `ref` (in
Rust terms, `&mut T` reborrows as `&T` for free, all the time — this
is one of the most common patterns in Rust code). Confirmed via a
standalone `vanic check` probe:

```vani
fn read_first(buf: ref [u8; 4]) -> u8 { return buf[0]; }
fn write_and_read(buf: mut ref [u8; 4]) -> u8 {
  buf[0] = 9 as u8;
  return read_first(buf);        // error: got mut ref, wanted ref
}
```

with the checker's own error: `argument 1 to 'read_first' must be
assignable to ref [u8; 4], got mut ref [u8; 4]`. Wrapping the
argument in an explicit `ref buf` doesn't help either — it produces a
literal reference-to-a-reference (`ref mut ref [u8; 4]`), not a
reborrow, and fails typecheck just as hard against the callee's plain
`ref [u8; 4]` parameter. There is no syntax in the language today
that gets from "I have a `mut ref T`" to "pass it somewhere that only
needs read access" without changing the callee's own signature.

This compounds badly in exactly the shape a real filtering/parsing
pipeline naturally takes: a function holds a buffer `mut ref` because
it just wrote into it (e.g. finishing a checksum-eligible header) and
then needs to call a handful of small, genuinely read-only helper
functions on that same buffer to finish its work (compute the
checksum it just made room for, in this case). Every one of those
read-only helpers has to be pushed onto `mut ref` too, purely to
satisfy the type checker, not because they need write access — and
that requirement then propagates transitively to every OTHER call
site of those same helpers, even ones that only ever had a `ref` in
hand and now can't call them at all (see gap's own worked case below).

**Note this is a real design tension, not a simple oversight**: one
of vani's own stated design principles (per this repo's docs) is that
`ref`/`mut ref` at a call site make aliasing and mutation cheap to
audit by inspection — a general reborrow rule needs to preserve that
audit property (in particular, the reborrowed `ref` must not remain
usable at the same time as the original `mut ref`, exactly like
Rust's own borrow checker enforces for `&mut` reborrows) rather than
just being a type-level `mut ref T -> ref T` coercion with no
lifetime/exclusivity tracking behind it. Whoever picks this up should
treat it as "does vani want a real (if narrow) borrow-checking pass,"
not "loosen one type rule."

**Workaround shipped, two shapes depending on the situation**:
1. Where an *owned local* already exists (not a parameter) — pass it
   as either kind at each call site instead of trying to convert an
   existing reference value. This works because vani DOES let a plain
   (non-`mut`) `let`-bound local supply `mut ref` (or `ref`) freely at
   its own call site — the restriction is only on re-wrapping an
   *already-reference-typed* value. See DhruvaOS commit `a9d189d`'s
   own SHA-256 code for this pattern already in use before round 88
   ever hit the parameter case.
2. Where the value genuinely only exists behind a `mut ref` PARAMETER
   (no owned local available) — copy through a fresh local first. See
   `netif_recv_frame_rpi4` in `kernel_main_rpi4.vani`: instead of
   filtering `out` (a `mut ref` param) directly, the function now
   reads the frame into a local `[u8; 512]`, passes THAT local as
   `ref` to the (read-only) packet filter, then copies it into `out`
   only if the filter allows it. Costs one extra 512-byte copy per
   receive call — acceptable at this scale, but a real reborrow
   feature would remove both the copy and the extra local entirely.
3. Where a read-only helper is called from both a `ref`-holding site
   and a `mut ref`-holding site (this round's `ipv4_checksum_rpi4`,
   needed from both `ipv4_build_header_rpi4`'s `mut ref frame` and
   `ipv4_verify_checksum_rpi4`'s otherwise-`ref` frame): standardized
   the helper AND every caller in that specific chain on `mut ref`,
   inlining what would otherwise be a delegated call to a `ref`-only
   sub-helper (`arp_read_u16_be_rpi4`) to avoid the mismatch
   recurring one level down. Only done for the specific functions
   that actually needed to interoperate with a `mut ref`-holding
   caller — the rest of the file's read-only helpers (the whole ARP
   module, most of the new IPv4 module) were left on plain `ref`,
   since nothing ever calls them from a `mut ref` context.

---

*(Append new entries below this line as they're found. Keep the
"found in round N" provenance and a real DhruvaOS commit/file
reference on each — that's what makes these actionable instead of
just a wishlist.)*
