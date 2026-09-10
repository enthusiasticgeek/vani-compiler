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

## 1. No array-repeat literal syntax (`[expr; N]`) — FIXED 2026-09-10

**Fixed**: `src/parser.rs`'s `TokenKind::LBracket` arm in
`parse_primary_expr` now accepts `[expr; N]`, desugaring at parse time
to N clones of the same `Expr` AST node -- exactly the "N copies"
`ArrayLit` shape `v31_default_init_expr` already built internally.
`N` must be a compile-time constant (literal int or a previously-
declared `const NAME: i64 = <int>;`), mirroring `parse_type`'s own
`[T; N]` array-length acceptance rule rather than inventing a new one.
Verified via a standalone probe (int-literal length, const-name
length, alongside the pre-existing comma-list form) on both the LLVM
and C backends, plus the full local test suite (278/279 passing; the
1 failure, `concurrent_pipeline_dashboard_example...`, is pre-existing
flaky concurrency test infrastructure unrelated to this change --
confirmed by rerunning it alone, which passed). Commit `8b2bc9bb`.

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

## 2. `let` always requires a full initializer — no uninitialized declaration — FIXED 2026-09-10

**Fixed**: `parse_let_stmt` now accepts `let x: T;` (type annotation,
no `= expr`) for any `T` the v3.1 async-fn default-init synthesizer's
own `v31_local_type_allowed` already recognizes as having a
well-defined zero value -- desugars eagerly at parse time to `let x: T
= v31_default_init_expr(T);`, exactly option (b) from this gap's own
original design note. By the time the checker/backends see the
statement it's indistinguishable from one the user wrote with an
explicit zero initializer, so no new dataflow analysis was needed.

Also extended `v31_local_type_allowed`/`v31_default_init_expr`
themselves (previously only `i64`/`bool`/`f64`/`str`/`OwnedStr`) to
cover every sized integer width (`u8`/`u16`/`u32`/`u64`/`i8`/`i16`/
`i32`, defaulting to an explicit `0 as <T>` cast expr, matching this
codebase's own established zero-literal idiom) -- without this, gap
#2's own primary motivating case, `let buf: [u8; 512];` (a bare-metal
scratch frame buffer with no heap allocator), would have stayed
rejected even though `let buf: [i64; 512];` was already accepted. This
extension is a strict superset for the pre-existing synthesizer too
(struct-field defaults), not a behavior change for anything it already
handled.

Verified via 3 standalone probes (scalar/array/struct zero values +
the scratch-then-fill pattern this gap exists to unblock; the
`f32`-has-no-default error path; the no-annotation `let v;` case still
producing the original "expected '='" error) on both backends, plus
the full local test suite (0 failures). Commit range starting
`87805ef5` (parser.rs's own combined gap #2 + integer-width diff).

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

## 3. No reborrow from `mut ref T` to `ref T` — PARTIALLY FIXED 2026-09-10 (call-argument case only)

**Fixed (narrow scope)**: a `mut ref T` value already in hand (a bare
variable reference -- `f(buf)`, not `f(ref buf)`/`f(mut ref buf)`) can
now satisfy a `ref T` parameter at a direct function-call argument
position without an explicit cast. `src/checker.rs`'s new
`reborrow_mut_ref_as_ref_arg` fires only when: the argument expression
is a plain `ExprKind::Var`, its checked type is `Type::RefMut(inner)`,
and the parameter wants exactly `Type::Ref(inner)` (identical inner
type) -- it retypes the argument in place (`Ref`/`RefMut` share an
identical runtime representation, a plain pointer, in both backends;
this is a pure relabeling, not a value transformation).

**Deliberately NOT a general reborrow feature**: this gap's own
original writeup flagged the real design tension -- "does vani want a
real (if narrow) borrow-checking pass" with lifetime/exclusivity
tracking, not just a type-coercion rule. This fix takes the narrowest
sound slice of that: reborrowing ONLY at a direct-call argument
position, where the reborrowed `ref` never outlives that one call (not
stored, not returned, not bound to a `let`). `let y: ref T = mut_ref_
var;` (a persistent reborrowed binding) and reborrow through other
expression positions (field access, index, indirect/fn-pointer calls)
remain unimplemented -- picking those up would need the fuller
borrow-checking pass this writeup originally called out, not an
extension of this same narrow mechanism.

**Soundness reasoning**: the existing argument-list aliasing check
(`classify_arg`/`check_arg_aliasing`, `src/checker.rs`) already treats
a bare ref-typed `Var` argument as an untracked Copy value (per its
own pre-existing comment: "Re-borrows of `&T`/`&mut T` could alias the
underlying owner but we don't track that yet") -- this fix's retyping
introduces no NEW aliasing hazard beyond that pre-existing, already-
accepted limitation, since it only changes what TYPE a `mut ref T`
argument presents as at one call site, not whether it's tracked.

Verified via 3 standalone probes on both backends: this gap's own
exact worked example (`read_first`/`write_and_read`), a negative
control confirming `ref T -> mut ref T` is still correctly rejected
(the reborrow is intentionally asymmetric), and workaround case 3 from
this entry's own "workaround shipped" section below (one `ref`-typed
helper now callable from both a `ref`-holding site and a `mut ref`-
holding site, no more standardizing every caller on `mut ref`). Full
local suite: 3030 lib tests + 278/279 e2e tests (the 1 "failure",
`detach_heartbeat_example...`, confirmed flaky/unrelated -- passed 3/3
in isolated reruns, a non-deterministic concurrent-print interleaving
issue with no connection to call-argument type coercion).

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

## 4. Fixed arrays (`[T; N]`) are move-only on plain `let`/`=` — no Copy, no `.clone()` — FIXED 2026-09-10

**Fixed**: `Type::is_copy()` (`src/ast.rs`) now recurses into the
array element type (`Type::Array { element, .. } => element.is_copy()`)
instead of unconditionally returning `false` -- mirroring the
`Type::Tuple` rule immediately below it ("Copy only when ALL elements
are Copy"). Both backends ALREADY emitted a real whole-array value
copy for `let ys: [T; N] = xs;` regardless of this flag (LLVM: load/
store the aggregate; C: memcpy) -- this was purely a checker-level
move-tracking restriction with no codegen dependency, so the fix is a
single match-arm change plus one follow-on codegen fix below.

**A real regression found and fixed along the way**: the C backend's
generic `Vec<T>.sort_by()` helper (`backend_c.rs`, the
`if element.is_copy() { ... }` block emitting a quicksort
implementation) had silently relied on `is_copy() == true` also
implying "plain C `=` assignment and a bare `{ct} key = a[i];`-style
local works for this element type" -- true for scalars/structs/enums,
but never true for a raw C array (C arrays can't be assigned via `=`
or copy-initialized as a plain local, only `memcpy`'d). Before this
fix, `Type::Array` was ALWAYS `is_copy() == false`, so this whole
sort/sort_by codegen path was simply never emitted for any
`Vec<[T; N]>` instantiation -- once arrays could be Copy, the C
backend tried to emit `intent_arr2_Struct_Point key = a[i];`-style
code, which doesn't compile in C. Fixed by excluding `Type::Array`
from that specific gate (`element.is_copy() && !matches!(element,
Type::Array { .. })`), restoring the exact pre-existing behavior for
array elements (sort/sort_by unavailable for `Vec<[T; N]>`, same as
before) without reverting the fix for actual scalar/struct Copy
types. Found via the full local e2e test suite, not manual review --
`vec_of_array_of_struct_from_named_variables_runs_correctly_on_both_
backends` failed with a C compile error inside a never-called-at-
runtime helper function, since many of this codebase's C helper
functions are emitted eagerly per `Vec<T>` instantiation regardless
of whether the program actually calls them.

Also updated 3 pre-existing lib.rs unit tests
(`let_alias_moves_source_array`, `move_into_function_consumes_array`,
`task_rejects_non_copy_capture_by_value`) that had used `[i64; N]`/
`[u32; N]` purely as a stand-in "non-Copy" fixture type -- retargeted
to `[OwnedStr; N]` (genuinely non-Copy) to keep covering real array
move/capture semantics, and added 2 new tests
(`copy_element_array_survives_move_into_function`,
`copy_element_array_let_alias_does_not_move`) covering the new
Copy-array behavior itself, including gap #4's own original worked
example from this file. Both new tests use `assert` rather than
`prove` -- the SMT-based `prove` verifier doesn't yet track array
VALUES symbolically across a copy (arrays were always-moved before
this fix, so it never needed to), a separate, deeper limitation
outside this gap's own scope; real runtime correctness (including
mutation-isolation -- proving the copy is a true bytewise copy, not
an aliased reference) was separately confirmed via standalone probes
run live on both backends.

Full local suite: 3030 lib tests + 279 e2e tests, 0 failures.

**Found**: round 95 of the Pi 4/5 port (field25519 field arithmetic +
X25519 Diffie-Hellman, 2026-09-06), writing the RFC 7748 Montgomery
ladder in `x25519_scalarmult_rpi4`.

**Gap**: `let y: [u32; 8] = x;` (or plain `y = x;` for two existing
locals) MOVES `x` rather than copying it — a later use of `x` in the
same function fails typecheck with `cannot borrow 'x' after it was
moved`, confirmed via a standalone probe (`movetest.vani`):

```vani
fn use_it(v: ref [u32; 4]) -> u32 { return v[0]; }
fn f() -> u32 {
  let x: [u32; 4] = [1 as u32, 2 as u32, 3 as u32, 4 as u32];
  let y: [u32; 4] = x;   // moves x
  let a: u32 = use_it(ref x);   // error: cannot borrow 'x' after it was moved
  return a + y[0];
}
```

There is also no escape hatch: array types have no `.clone()` method
(`clonetest.vani` probe: "methods are attached to struct/enum types
only in v1"), so the only way to duplicate a `[T; N]` value today is a
hand-written element-by-element copy loop.

**This corrects a wrong assumption already written into this repo's
own docs** (gap #1 above, line ~58: "matching how `[T; N]` locals are
already documented as genuine `Copy` stack values elsewhere in this
repo's own docs") and into an earlier DhruvaOS project memory note
that called `[T; N]` arrays "genuine Copy stack values." That claim
is only true for *passing* an array by `ref`/`mut ref` at a call site
(the callee borrows the caller's storage, nothing is copied or
moved) — it does NOT hold for plain value-binding (`let`/`=`), which
is move-only with no opt-in Copy. For a value type with no heap
involvement (a `[u32; 8]` is 32 bytes of plain data, no different
from a struct of 8 `u32` fields), forcing move-only semantics with no
Copy/Clone escape hatch is a real ergonomic gap other Rust-like
languages close via `#[derive(Copy, Clone)]` for arrays of Copy
element types.

**Workaround shipped**: every place a genuine duplicate was needed
(not just a borrow), replaced the natural `let y = x;` with an
explicit zero-then-copy-loop:

```vani
let x3: [u32; 8] = fe_zero_rpi4();
let x3i: i64 = 0;
while x3i < 8 {
  x3[x3i] = x1[x3i];
  x3i = x3i + 1;
}
```

Also drove a broader design choice this round: because this gap
compounds with gap #3 (no reborrow) and a separate confirmed
aliasing-XOR rule (a `mut ref` borrow of a variable cannot coexist
with any other borrow of that same variable in one call — expected,
correct behavior for a memory-safe language, not itself a gap, but
worth recording as context: `f(ref x, ref y, mut ref x)` fails with
"argument list aliases 'x': an '&mut' borrow cannot coexist with
another use of the same variable in the same call"), the whole
field25519/X25519 module was designed around bignum/field functions
**returning their result by value** rather than Pi 1's ARM32
out-parameter convention — sidesteps all three constraints at once,
since an owned local can always supply either `ref` or `mut ref` at
its own call site, and `x = f(ref x)` (read via `ref`, then reassign
from the call's own return after the borrow ends) was confirmed via
probe (`reassigntest.vani`) to work correctly. See DhruvaOS
`kernel_main_rpi4.vani`'s `field25519_add_rpi4`/`field25519_mul_rpi4`/
etc. and `x25519_scalarmult_rpi4`.

---

*(Append new entries below this line as they're found. Keep the
"found in round N" provenance and a real DhruvaOS commit/file
reference on each — that's what makes these actionable instead of
just a wishlist.)*
