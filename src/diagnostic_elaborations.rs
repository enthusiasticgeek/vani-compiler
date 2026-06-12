//! Step-by-step elaboration strings for the 25 most-common
//! diagnostic families.
//!
//! User-direction item (added 2026-06-08): when an error is
//! non-obvious, the diagnostic gains a numbered "help:"
//! breakdown after the standard message. Each elaboration
//! follows the WHAT / WHY / HOW shape:
//!
//!   1. WHAT went wrong (in user-friendly terms, restated).
//!   2. WHY the compiler enforces this (the rule or invariant).
//!   3. HOW to fix it (concrete refactor or alternative).
//!
//! Call sites attach the elaboration via
//! `Diagnostic::new(span, msg).with_elaboration(<family>())`.
//!
//! The vast majority of diagnostic sites in the compiler do
//! NOT use elaboration — the message itself is enough. This
//! module seeds the 25 sites that experience has shown are
//! the most confusing for newcomers.

/// Move-after-use (affine ownership). One of the highest-volume
/// diagnostic families in user code; the elaboration explains
/// affine ownership in concrete terms.
pub fn move_after_use(name: &str) -> Vec<String> {
    vec![
        format!(
            "After `let other = {}`, `{}` is no longer usable — \
             ownership has transferred to `other`.",
            name, name,
        ),
        "vāṇी uses affine ownership: each heap-owning value has \
         exactly one owner at a time. Reading after move would \
         alias the owner, breaking the no-implicit-clone story \
         AND opening a double-free door."
            .to_string(),
        format!(
            "Either (a) borrow instead of moving (`ref {}` at the \
             use site if a function takes `ref T`), (b) call \
             `.clone()` explicitly if you want two copies (and \
             see the cost at the call site), or (c) restructure \
             so the second use happens before the move.",
            name,
        ),
    ]
}

/// Aliasing mut + shared (or two mut borrows). The XOR rule.
pub fn alias_mut_with_shared(name: &str) -> Vec<String> {
    vec![
        format!(
            "While `{}` is borrowed mutably (`mut ref`), no other \
             borrow (shared or mut) on `{}` may exist.",
            name, name,
        ),
        "The XOR rule: many shared borrows OR one mut borrow, \
         never both at once. This is what makes vāṇी's compile-\
         time race-freedom work — the same rule prevents data \
         races at runtime."
            .to_string(),
        "End the mut borrow first (return from the call, exit \
         the block) before taking the shared borrow, or use \
         shared borrows only if the operation doesn't mutate."
            .to_string(),
    ]
}

/// Type-mismatch at let-initializer / parameter / argument.
pub fn type_mismatch(expected: &str, got: &str) -> Vec<String> {
    vec![
        format!(
            "The value has type `{}`, but the slot expects `{}`.",
            got, expected,
        ),
        "vāṇी doesn't do implicit conversions between sized \
         integer types or between Str / OwnedStr — every \
         conversion is explicit so the cost / loss is visible \
         at the call site."
            .to_string(),
        format!(
            "If `{}` and `{}` are integer widths, add an explicit \
             `as {}` cast. If they're Str / OwnedStr, use the \
             builtins (`s + \"\"` to copy a Str into an OwnedStr, \
             or pass `ref str_value` for read-only). If they're \
             distinct user types, the conversion needs an \
             explicit constructor.",
            got, expected, expected,
        ),
    ]
}

/// Return-position scope-escape: ref to a local in returned value.
pub fn return_escape_local(name: &str) -> Vec<String> {
    vec![
        format!(
            "The returned value carries a `ref` (direct or inside \
             a struct field / Vec slot) to local binding `{}`. \
             That local drops when the function returns.",
            name,
        ),
        "References are second-class in vāṇी — they cannot \
         outlive the binding they borrow from. Returning a ref \
         to a local would dangle the moment the caller reads \
         through it."
            .to_string(),
        format!(
            "Either (a) return the value by `move` instead of by \
             ref (drop the `ref` keyword; clone if needed), \
             (b) take `{}` as a `ref` parameter so the caller \
             owns it, or (c) wait for path-C (lifetime elision) \
             which lifts the single-ref-param case.",
            name,
        ),
    ]
}

/// Scope-escape on struct field / Vec push of a deeper-scope ref.
pub fn scope_escape_deeper(name: &str) -> Vec<String> {
    vec![
        format!(
            "The ref to `{}` is being stored in a place whose \
             scope outlives `{}` — `{}` drops first, leaving the \
             stored pointer dangling.",
            name, name, name,
        ),
        "References must always be borrowed from a same-or-outer \
         scope. The scope-escape analyzer rejects every shape \
         that would leak a pointer past its source's scope-exit."
            .to_string(),
        format!(
            "Move `{}`'s declaration up to the same scope as the \
             container (or earlier), so it outlives every borrow \
             of it. Alternatively, store an owned copy of `{}` \
             instead of a ref.",
            name, name,
        ),
    ]
}

/// Function return-type is `ref T` (not yet supported).
pub fn ret_type_is_ref() -> Vec<String> {
    vec![
        "A function return type cannot be a `ref T` in v1."
            .to_string(),
        "Returning refs requires lifetime tracking (path-C \
         in the design): the compiler has to infer which \
         input ref the output borrows from. v1 ships without \
         this; the rejection is the safety guard."
            .to_string(),
        "Either return the value by `move` (no `ref` in the \
         return type), or return an `i64` / index into a Vec \
         that the caller already owns. The lifetime-elision \
         arc (path-C) will lift this rejection in a future \
         release."
            .to_string(),
    ]
}

/// Implicit integer promotion (u64 ↔ i64 etc.).
pub fn no_implicit_int_promotion(a: &str, b: &str) -> Vec<String> {
    vec![
        format!(
            "The two operands have types `{}` and `{}`. vāṇी \
             requires an explicit cast — no silent conversion.",
            a, b,
        ),
        "Integer promotion in C / C++ silently changes signedness \
         and width, hiding precision loss + overflow bugs. vāṇी \
         makes every conversion visible so the cost is at the \
         call site."
            .to_string(),
        format!(
            "Pick the target width explicitly: `(x as {})` or \
             `(y as {})` depending on which operand should be \
             converted. If both should be the same type, change \
             the declaration of one to match the other.",
            b, a,
        ),
    ]
}

/// Match exhaustiveness failure.
pub fn match_not_exhaustive(missing: &str) -> Vec<String> {
    vec![
        format!(
            "The match doesn't cover every possible value — \
             specifically, `{}` is unreachable from the listed \
             arms.",
            missing,
        ),
        "vāṇी requires exhaustive match so a future enum-variant \
         addition forces every match site to be updated. Silent \
         fall-through is a class of bug the type system can \
         prevent."
            .to_string(),
        format!(
            "Add an arm for `{}`, or add a wildcard `_ then ...` \
             arm if you want to catch every other case with one \
             branch. The wildcard explicitly says \"I considered \
             this; it's intentional\".",
            missing,
        ),
    ]
}

/// SMT precondition failed at call site.
pub fn smt_requires_failed(fn_name: &str) -> Vec<String> {
    vec![
        format!(
            "The pre-condition (`requires` clause) of `{}` could \
             not be discharged at this call site.",
            fn_name,
        ),
        "vāṇी uses an SMT solver (Z3) to prove `requires` / \
         `ensures` clauses at compile time. The solver couldn't \
         derive enough information from the call site's context \
         to prove the pre-condition holds."
            .to_string(),
        format!(
            "Either (a) add an explicit `assert <precondition>;` \
             at the call site so the solver sees the fact \
             locally, (b) strengthen the surrounding `requires` \
             clauses to carry the fact down, or (c) verify with \
             `INTENT_TRACE_SMT=1` to see exactly which sub-goal \
             the solver couldn't discharge.",
        ),
    ]
}

/// SMT post-condition violation at return.
pub fn smt_ensures_failed(fn_name: &str) -> Vec<String> {
    vec![
        format!(
            "The post-condition (`ensures` clause) of `{}` could \
             not be discharged for this return path.",
            fn_name,
        ),
        "vāṇी's SMT layer proves that every `return EXPR` \
         satisfies every `ensures` clause. If the solver can't \
         build the proof, the function is rejected — preventing \
         programs that quietly violate their own contracts."
            .to_string(),
        "Either (a) the ensures clause is wrong — relax it; \
         (b) the return expression is wrong — fix it; or (c) \
         the SMT solver needs an extra fact: add an `assert` in \
         the body that captures the invariant the solver \
         couldn't infer."
            .to_string(),
    ]
}

/// Wrong arity at a function call.
pub fn wrong_arity(expected: usize, got: usize) -> Vec<String> {
    vec![
        format!(
            "The function takes {} argument{}, but {} {} provided.",
            expected,
            if expected == 1 { "" } else { "s" },
            got,
            if got == 1 { "was" } else { "were" },
        ),
        "Every fn call in vāṇी is statically resolved — no \
         varargs in v1, no default arguments. The arity is part \
         of the type signature."
            .to_string(),
        "Either pass the missing arguments, or update the \
         function's signature if its arity legitimately \
         changed. If you want to pass extra context, add a \
         struct parameter that bundles them."
            .to_string(),
    ]
}

/// Captured-variable mutation in parallel for body.
pub fn parallel_for_mutates_capture(name: &str) -> Vec<String> {
    vec![
        format!(
            "The `parallel for` body writes to `{}`, a binding \
             declared OUTSIDE the loop body. Concurrent writes \
             to one location race.",
            name,
        ),
        "vāṇी proves race-freedom at compile time by rejecting \
         every observable side effect in a parallel-for body \
         that ISN'T explicitly declared as a reduction. The \
         check fires on captured Copy mutations same as on \
         non-Copy ones — there's no \"safe\" race."
            .to_string(),
        format!(
            "Either (a) declare `{}` as a reduction \
             (`reduce {} as ...`) so the loop combines per-\
             iteration partial values, (b) use an `Atomic<i64>` \
             cell and `atomic_add(ref {})` (sequencer-checked \
             cross-iter merge), or (c) accept that the work \
             can't parallelize and switch to a regular `for`.",
            name, name, name,
        ),
    ]
}

/// Field access on a non-struct / unknown field.
pub fn field_not_found(field: &str, ty: &str) -> Vec<String> {
    vec![
        format!(
            "Type `{}` doesn't have a field named `{}`.",
            ty, field,
        ),
        "vāṇी resolves field access at type-check time — every \
         field name must be declared in the type's struct \
         definition (or in an `iface` for method calls)."
            .to_string(),
        format!(
            "Check the `{}` declaration for the actual field \
             names; common typos are `_` vs camelCase. If you \
             meant a method, ensure it's declared in a \
             `methods on {}` block or via an iface impl.",
            ty, ty,
        ),
    ]
}

/// Use of an undeclared variable.
pub fn unknown_variable(name: &str) -> Vec<String> {
    vec![
        format!(
            "There is no binding named `{}` in scope at this \
             point.",
            name,
        ),
        "vāṇी uses lexical scope: a `let` binding is visible \
         from its declaration to the end of the enclosing \
         block. Inner-block bindings don't leak out; \
         already-moved bindings stop being readable."
            .to_string(),
        format!(
            "Either (a) typo — check the spelling against the \
             closest declaration, (b) the binding is in a \
             different scope — move the declaration up, or \
             (c) you moved it earlier — see the move-recovery \
             hint (\"borrow instead of moving\" / \"clone if you \
             want two copies\")."
        ),
    ]
}

/// Cannot move out of a borrowed binding.
pub fn move_out_of_borrowed(name: &str) -> Vec<String> {
    vec![
        format!(
            "`{}` is currently borrowed (a `ref` / `mut ref` is \
             live), so moving its value would invalidate the \
             outstanding borrow.",
            name,
        ),
        "Affine ownership + second-class refs: if any borrow is \
         active on a binding, the binding itself can't be \
         moved — that would leave the borrow pointing into \
         freed memory."
            .to_string(),
        format!(
            "Wait for the borrow to end (the function returning \
             / the block ending) before moving, or restructure \
             so the move happens before the borrow is taken. \
             For values you frequently want to both move and \
             share, consider switching to `Box<T>` and passing \
             `ref Box<{}>` borrows.",
            name,
        ),
    ]
}

/// Missing return statement in a non-i64 fn.
pub fn missing_return(fn_name: &str, ret_ty: &str) -> Vec<String> {
    vec![
        format!(
            "Function `{}` declares return type `{}`, but at \
             least one path through the body reaches the end \
             without a `return`.",
            fn_name, ret_ty,
        ),
        "Every non-`-> i64` function must end on every path \
         with an explicit `return EXPR`. v1 doesn't have \
         expression-bodied fns or block-as-expression returns; \
         every exit is an explicit statement."
            .to_string(),
        "Add a `return EXPR;` at the end of the missing path. \
         For match expressions, ensure every arm contains a \
         `return`. For if/else, ensure both branches return \
         (or merge to a single post-if return)."
            .to_string(),
    ]
}

/// Vec push() requires a Vec or mut-ref Vec receiver.
pub fn push_wrong_receiver(got: &str) -> Vec<String> {
    vec![
        format!(
            "`push` expects the first argument to be a `Vec<T>` \
             (by value, consuming form) or a `mut ref Vec<T>` \
             (in-place form). Got `{}`.",
            got,
        ),
        "Vec mutators have two surface forms — consuming \
         (`xs = push(xs, v)`) and in-place (`push(mut ref xs, \
         v)`). The in-place form is the idiomatic choice when \
         `xs` is owned by another binding (struct field, etc.)."
            .to_string(),
        "Either pass `xs` directly by value (and rebind the \
         result), or pass `mut ref xs` to mutate in place. \
         The mut-ref form returns the new `len` as i64, NOT \
         the Vec."
            .to_string(),
    ]
}

/// Interface not implemented for receiver.
pub fn iface_not_impl(iface: &str, ty: &str) -> Vec<String> {
    vec![
        format!(
            "Type `{}` does not implement interface `{}`.",
            ty, iface,
        ),
        "vāṇी resolves interface dispatch (both static and \
         `dyn Iface`) at type-check time. Calling an iface \
         method requires an `implement <Iface> for <T> { ... }` \
         block to be in scope."
            .to_string(),
        format!(
            "Add `implement {} for {} {{ ... }}` with the \
             required method bodies. The compiler will report \
             which methods are still missing.",
            iface, ty,
        ),
    ]
}

/// Enum variant arity mismatch.
pub fn enum_variant_arity(
    variant: &str,
    expected: usize,
    got: usize,
) -> Vec<String> {
    vec![
        format!(
            "Variant `{}` takes {} payload field{}, but {} \
             {} provided.",
            variant,
            expected,
            if expected == 1 { "" } else { "s" },
            got,
            if got == 1 { "was" } else { "were" },
        ),
        "Enum variants in vāṇी have a fixed shape: zero or \
         more positional payload fields. Constructor and match \
         arm both have to match the shape exactly."
            .to_string(),
        format!(
            "Check the `enum` declaration: every `{}(...)` \
             constructor + every `{}(...)` match arm must use \
             the same number of fields as declared.",
            variant, variant,
        ),
    ]
}

/// Cannot assign through immutable ref / non-mut binding.
pub fn assign_to_immutable(name: &str) -> Vec<String> {
    vec![
        format!(
            "`{}` is a `ref T` (shared, immutable) — assigning \
             through it requires `mut ref T` instead.",
            name,
        ),
        "Shared refs are read-only by design. The XOR rule \
         (many shared XOR one mut) means a shared ref cannot \
         be promoted to a mut ref without ending every other \
         outstanding borrow first."
            .to_string(),
        format!(
            "Change the parameter / let to `mut ref {}` if you \
             genuinely need to mutate. If the mutation is \
             optional, return a new value by `move` instead.",
            name,
        ),
    ]
}

/// `assert <p>;` — SMT couldn't discharge.
pub fn assert_not_proven(predicate: &str) -> Vec<String> {
    vec![
        format!(
            "The SMT solver couldn't prove `{}` at this point — \
             so the compiler emits a runtime check that fires \
             abort() if the assert ever fails at runtime.",
            predicate,
        ),
        "Note: this is NOT a build failure — the program still \
         compiles. It's a hint that you could prove this \
         statically by strengthening the surrounding `requires` \
         / `ensures` / `invariant` clauses, eliminating the \
         runtime check entirely."
            .to_string(),
        "Run with `INTENT_TRACE_SMT=1` to see exactly which \
         sub-goal the solver couldn't discharge. Common fix: \
         add a `requires` clause on the enclosing function \
         that captures the missing fact."
            .to_string(),
    ]
}

/// Index out of bounds — SMT can't prove.
pub fn index_oob_not_proven(name: &str) -> Vec<String> {
    vec![
        format!(
            "The index expression on `{}` couldn't be proven \
             in-bounds at compile time — a runtime bounds \
             check stays.",
            name,
        ),
        "vāṇी bounds-checks every Vec / array index by default. \
         When SMT can prove `i < len(xs)` from the surrounding \
         context, the check is elided; otherwise it stays in \
         the emitted code and fires abort() on out-of-bounds."
            .to_string(),
        format!(
            "Either (a) add an explicit `if i >= len({}) {{ \
             return ...; }}` guard so the check becomes \
             unreachable for the indexed path, or (b) add a \
             `requires i < len({})` clause on the enclosing \
             function so SMT sees it.",
            name, name,
        ),
    ]
}

/// `unsafe { ... }` block on a hosted target.
pub fn unsafe_on_hosted() -> Vec<String> {
    vec![
        "`unsafe(reason = \"...\")` is rejected on hosted \
         targets (Linux / macOS / Windows). It only opens on \
         `--target embedded`."
            .to_string(),
        "vāṇी's design contract: hosted programs are safe by \
         construction — no segfault surface, no UAF, no buffer \
         overrun. Allowing `unsafe` on hosted would compromise \
         that story."
            .to_string(),
        "If you genuinely need the operation: pick an embedded \
         target with `--target embedded`. If you're trying to \
         do something the safe path doesn't cover, file an \
         issue — most operations have a safe equivalent (FFI, \
         Box, Pool, etc.)."
            .to_string(),
    ]
}

/// Drop-trait double-impl.
pub fn duplicate_drop_impl(ty: &str) -> Vec<String> {
    vec![
        format!(
            "Type `{}` already has a `Drop` impl in scope. \
             vāṇी allows exactly one Drop per type.",
            ty,
        ),
        "Drop is special: the compiler emits a call to it on \
         every scope-exit path. Multiple Drop impls would race \
         to register; the language picks the single-Drop \
         constraint instead so the cleanup is deterministic."
            .to_string(),
        format!(
            "Merge the two impls into one `implement Drop for \
             {} {{ fn drop(self: mut ref Self) -> i64 {{ ... }} \
             }}` block, or delete the duplicate.",
            ty,
        ),
    ]
}

/// Vec index returns a non-Copy element by value (would alias).
pub fn vec_index_non_copy_aliases(name: &str) -> Vec<String> {
    vec![
        format!(
            "Indexing `{}[i]` would copy a non-Copy element by \
             value, aliasing the slot inside the Vec — \
             double-free on scope-exit drop.",
            name,
        ),
        "Affine ownership + Vec storage: a non-Copy element \
         (Vec, OwnedStr, struct with heap fields) lives in \
         exactly one place. Bare `xs[i]` would clone the \
         pointer without cloning the heap."
            .to_string(),
        format!(
            "Use `clone_at(ref {}, i)` to get an owned deep \
             copy (explicit, visible cost), or `ref {}[i]` to \
             get a `ref T` borrow into the slot (no copy, \
             same XOR rules as any ref).",
            name, name,
        ),
    ]
}

/// Pure function with observable side effect.
pub fn pure_fn_has_effect(fn_name: &str) -> Vec<String> {
    vec![
        format!(
            "Function `{}` is declared as a `pure` function, \
             but its body has an observable side effect.",
            fn_name,
        ),
        "Pure functions in vāṇी are honored by both SMT \
         (allowed inside `requires` / `ensures`) AND the \
         parallel-for safety pass. They must be safe to call \
         multiple times with the same arguments and produce \
         the same result, no externally visible mutation."
            .to_string(),
        "Remove the `pure` qualifier if the function genuinely \
         needs the side effect (then it can't be used in SMT \
         clauses or pure-call positions); or refactor the body \
         to return the would-be side effect as a value the \
         caller threads through explicitly."
            .to_string(),
    ]
}

/// Closure captures an affine binding by value.
pub fn closure_captures_affine(name: &str) -> Vec<String> {
    vec![
        format!(
            "Closure body uses `{}` from the enclosing scope, \
             but `{}` is non-Copy (heap-owning). Closures \
             cannot move-capture in v1.",
            name, name,
        ),
        "vāṇी closures are zero-allocation by default: the \
         captured environment is materialized as a small \
         struct, and the closure value is a fat pointer (16 \
         bytes). Move-capture would require runtime allocation \
         for the captured value's heap; v1 keeps captures \
         Copy-only to preserve the no-implicit-clone story."
            .to_string(),
        format!(
            "Either (a) borrow `{}` into the closure via `ref \
             {}` capture (handled automatically by the \
             compiler when `{}` is non-Copy in the captured \
             expression), or (b) clone the value first and \
             move the clone in, or (c) restructure to thread \
             the value through the closure's parameter list.",
            name, name, name,
        ),
    ]
}

/// Struct literal missing a required field.
pub fn struct_literal_missing_field(ty: &str, field: &str) -> Vec<String> {
    vec![
        format!(
            "The `{}` literal does not initialize field `{}`, \
             which is required.",
            ty, field,
        ),
        "vāṇी has no default field values and no partial \
         construction — every field must be set at the literal \
         site so the compiler can guarantee the struct is fully \
         initialized on every path."
            .to_string(),
        format!(
            "Add `{}: <value>` to the literal. To see all \
             required fields, find the `struct {}` declaration \
             in the source (or use `vanic check` — it lists \
             every missing field in one pass).",
            field, ty,
        ),
    ]
}

/// Method call on a type that has no such method.
pub fn method_not_found(method: &str, ty: &str) -> Vec<String> {
    vec![
        format!(
            "Type `{}` does not have a method named `{}`.",
            ty, method,
        ),
        "Methods in vāṇी must be declared explicitly — either \
         in a `methods on T { ... }` block or via \
         `implement <Iface> for T { ... }`. There is no \
         implicit method inheritance or duck-typed dispatch."
            .to_string(),
        format!(
            "Either (a) add `methods on {} {{ fn {}(self: ref \
             {}, …) -> … {{ … }} }}` to declare it, (b) check \
             for a typo against the declared method names, or \
             (c) check that the correct `implement` block is in \
             scope for the type.",
            ty, method, ty,
        ),
    ]
}

/// Reference to an undeclared struct type.
pub fn unknown_struct_type(name: &str) -> Vec<String> {
    vec![
        format!(
            "No struct named `{}` is in scope at this point.",
            name,
        ),
        "vāṇी resolves every type name at compile time. The \
         type must be declared (with `struct {}`) before any \
         use — forward references aren't supported in v1, and \
         there's no implicit import."
            .to_string(),
        format!(
            "Either (a) typo — compare against the `struct` \
             declaration name, (b) the struct is in another \
             file — add `include \"path.vani\";` at the top, \
             or (c) the struct hasn't been declared yet — add \
             `struct {} {{ … }}` above this use.",
            name,
        ),
    ]
}

/// Assignment / reassignment to a variable that was never declared.
pub fn assign_to_unknown_variable(name: &str) -> Vec<String> {
    vec![
        format!(
            "`{}` has not been declared with `let` in this \
             scope, so assigning to it is rejected.",
            name,
        ),
        "vāṇी requires an explicit `let name: Type = value;` \
         declaration before a binding can be used or mutated. \
         There is no implicit variable creation on assignment \
         (unlike Python / JavaScript)."
            .to_string(),
        format!(
            "Add `let {}: <Type> = <initial_value>;` before \
             this assignment. If `{}` IS declared but in an \
             outer or sibling block, move the `let` up so it's \
             in scope here.",
            name, name,
        ),
    ]
}

/// Iterate over a non-Vec / non-range type with `for`.
pub fn for_over_non_iterable(ty: &str) -> Vec<String> {
    vec![
        format!(
            "`for` can only iterate over `Vec<T>` or an integer \
             range (`a .. b`). The expression here has type `{}`.",
            ty,
        ),
        "vāṇी's `for` statement is not polymorphic — there is \
         no `Iterator` trait in v1. Only the two built-in \
         iterable shapes are supported."
            .to_string(),
        "Either (a) collect results into a `Vec<T>` and iterate \
         over that, (b) convert the expression to a range \
         `0 .. n`, or (c) use a `while` loop with an explicit \
         index for more complex iteration patterns."
            .to_string(),
    ]
}
