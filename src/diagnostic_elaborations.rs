//! Step-by-step elaboration strings for the 52 most-common
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

/// Function return-type is `ref T`, rejected by the single-ref-param
/// lifetime-elision rule (zero ref params, 2+ ref params, or a ref
/// nested inside a tuple/Vec/array/generic return type).
pub fn ret_type_is_ref() -> Vec<String> {
    vec![
        "A `ref T` / `mut ref T` return type is allowed, but only \
         under vāṇी's single-ref-parameter elision rule: the \
         function must have exactly one `ref`/`mut ref` parameter, \
         and the returned ref borrows from that parameter's source."
            .to_string(),
        "vāṇी doesn't expose Rust-style explicit lifetime variables \
         (`'a`, `'b`) -- the compiler infers the borrow relationship \
         purely from the signature shape. That's why zero ref params, \
         two-or-more ref params, and a ref nested inside another type \
         (a tuple, `Vec`, or `Option<ref T>`) are all rejected: none \
         of those shapes has one unambiguous source to elide from."
            .to_string(),
        "Either return the value by `move` (no `ref` in the return \
         type), restructure to exactly one `ref` parameter (pass the \
         others by value, or split into narrower functions -- one \
         bare `ref` return each), or return an `i64` / index into a \
         Vec that the caller already owns."
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
             `VANIC_SMT_DEBUG=1` to see exactly which sub-goal \
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

/// `implement Iface for T` is missing a required interface method.
pub fn iface_impl_missing_method(method: &str, iface: &str, ty: &str) -> Vec<String> {
    vec![
        format!(
            "The `implement {} for {}` block does not provide \
             a body for method `{}`.",
            iface, ty, method,
        ),
        "This particular method has no default body in the \
         interface declaration, so every `implement` block must \
         supply its own. (Interface methods CAN have a default \
         body that implementors inherit without writing anything — \
         see default methods — but this one doesn't.)"
            .to_string(),
        format!(
            "Add `fn {}(self: ref {}, …) -> … {{ … }}` inside \
             the `implement {} for {}` block. The signature must \
             match the interface declaration exactly (parameter \
             types, return type).",
            method, ty, iface, ty,
        ),
    ]
}

/// `pure fn` or `pure extern` calling a non-pure function.
pub fn pure_fn_calls_non_pure(callee: &str, context_kind: &str) -> Vec<String> {
    vec![
        format!(
            "`{}` calls `{}`, which is not declared `pure`.",
            context_kind, callee,
        ),
        "A `pure fn` may only call other `pure fn`s. Calling a \
         non-pure function would introduce hidden side effects, \
         breaking the guarantees that let vāṇī use `pure fn` \
         inside `requires` / `ensures` clauses and parallel-for \
         bodies."
            .to_string(),
        format!(
            "Either (a) mark `{}` as `pure fn` if it genuinely \
             has no side effects, (b) remove the call and \
             compute the value another way, or (c) remove the \
             `pure` qualifier from the calling function if it \
             legitimately needs the side effect.",
            callee,
        ),
    ]
}

/// Two parameters in the same function share a name.
pub fn duplicate_parameter(name: &str) -> Vec<String> {
    vec![
        format!(
            "Parameter `{}` appears more than once in this function's \
             parameter list.",
            name,
        ),
        "Each parameter name must be unique within a function \
         signature — the compiler uses the name to bind the \
         argument at the call site, so duplicates are ambiguous."
            .to_string(),
        format!(
            "Rename one of the `{}` parameters to a distinct \
             name, e.g. `{}_2` or a more descriptive identifier \
             that reflects its different role.",
            name, name,
        ),
    ]
}

/// Wrong-kind pattern for a match scrutinee (e.g. integer literal
/// used in a string-scrutinee match).
pub fn match_wrong_pattern_type(scrut_ty: &str, expected_form: &str) -> Vec<String> {
    vec![
        format!(
            "The match scrutinee has type `{}`, but this arm's \
             pattern is not a {} literal.",
            scrut_ty, expected_form,
        ),
        "Every arm in a match block must use a pattern literal \
         that is the same kind as the scrutinee. vāṇी does not \
         coerce pattern literals — an integer pattern cannot match \
         a string scrutinee, and vice versa."
            .to_string(),
        format!(
            "Change the pattern to a {} literal (e.g. {} literal \
             form) or use a wildcard `_ then …` arm to catch any \
             value the other arms don't cover.",
            expected_form,
            if expected_form == "string" { "\"text\"" }
            else if expected_form == "float" { "3.14" }
            else { "matching" },
        ),
    ]
}

/// Duplicate top-level declaration (struct, enum, function, const).
pub fn duplicate_declaration(kind: &str, name: &str) -> Vec<String> {
    vec![
        format!(
            "A {} named `{}` is already declared in this program.",
            kind, name,
        ),
        "vāṇी resolves all names globally at compile time. Two \
         top-level declarations with the same name are ambiguous — \
         the compiler cannot know which one a call site refers to."
            .to_string(),
        format!(
            "Either rename one of the `{}` declarations, or \
             remove the duplicate. If this is intentional \
             (e.g. two files define the same helper), move the \
             shared declaration to a single `include`d file.",
            name,
        ),
    ]
}

/// No `fn main()` entry point found in the program.
pub fn missing_main_function() -> Vec<String> {
    vec![
        "Every vāṇī program needs exactly one `fn main() -> i64` \
         to be the entry point."
            .to_string(),
        "The compiler looks for a top-level function named `main` \
         with no parameters and an `i64` return type. Without it, \
         the linker cannot find the entry point and the binary \
         cannot be produced."
            .to_string(),
        "Add a top-level entry point:\n\
         \n\
         fn main() -> i64 {\n\
         \treturn 0;\n\
         }"
            .to_string(),
    ]
}

/// `main` is present but has the wrong signature.
pub fn main_wrong_signature() -> Vec<String> {
    vec![
        "`main` must have exactly the signature `fn main() -> i64` \
         — no parameters, return type `i64`."
            .to_string(),
        "The OS entry-point convention passes command-line \
         arguments via the C runtime (accessible via `argc` / \
         `argv` FFI if needed), so vāṇī's `main` takes no \
         parameters. The `i64` return becomes the process exit \
         code (zero = success)."
            .to_string(),
        "Fix the signature to `fn main() -> i64 { ... }`. If you \
         need command-line arguments, call the C `argc`/`argv` \
         API via `extern` declarations."
            .to_string(),
    ]
}

/// Call to a function that has not been declared.
/// Kosh namespacing arc, Phase 5 (2026-07-21, see
/// `docs/kosh_namespacing_design.md`): migration-UX variant of
/// `unknown_function`, used when the checker finds a module-qualified
/// signature matching the bare name the caller wrote (e.g. `mat_mul`
/// unqualified, but `matrix__mat_mul` exists in the signature table --
/// meaning `matrix::mat_mul` would resolve). Most common case in
/// practice: a package's own internal code, or a consumer's code,
/// still calling a Kosh dependency's function unqualified after
/// namespacing (Phase 3) started requiring the `pkgname::` prefix.
pub fn unknown_function_with_module_suggestion(name: &str, suggestion: &str) -> Vec<String> {
    vec![
        format!(
            "No function named `{}` is visible at this call site -- but `{}` \
             exists.",
            name, suggestion,
        ),
        format!(
            "Did you mean `{}`? vāṇी resolves all function names at compile \
             time from the top-level declarations in the current file; a \
             function that lives inside `module {{ ... }}` (including a Kosh \
             package pulled in via `[deps]`) must be called with its module \
             qualifier -- there is no unqualified fallback lookup.",
            suggestion,
        ),
        format!(
            "If `{}` is a Kosh package dependency, every call site that used \
             to call `{}(...)` unqualified needs updating to `{}` -- this is \
             expected after a package adopts per-package namespacing, not a \
             sign anything is broken on your end.",
            suggestion.split("::").next().unwrap_or(suggestion),
            name,
            suggestion,
        ),
    ]
}

pub fn unknown_function(name: &str) -> Vec<String> {
    vec![
        format!(
            "No function named `{}` is visible at this call site.",
            name,
        ),
        "vāṇī resolves all function names at compile time from \
         the top-level declarations in the current file (and any \
         files pulled in via `include`). There is no dynamic \
         dispatch at the call site — the name must resolve \
         statically."
            .to_string(),
        format!(
            "Either (a) check for a typo in `{}`, (b) make sure \
             the file that declares `{}` is included with \
             `include \"path.vani\";`, or (c) add the function \
             declaration above this call:\n\
             \n\
             fn {}(…) -> i64 {{ … }}",
            name, name, name,
        ),
    ]
}

/// Reference to a struct type that has never been declared.
pub fn struct_not_declared(name: &str) -> Vec<String> {
    vec![
        format!(
            "No `struct {}` declaration is in scope here.",
            name,
        ),
        "vāṇी resolves every type name at compile time. The \
         struct must be declared with a `struct` block before it \
         can be used as a type, in a pattern, or in a literal — \
         forward references and implicit imports are not supported \
         in v1."
            .to_string(),
        format!(
            "Either (a) check for a typo (compare against the \
             `struct` declaration), (b) add `include \"path.vani\";` \
             for the file that defines `{}`, or (c) declare the \
             struct above this use:\n\
             \n\
             struct {} {{ field: i64 }}",
            name, name,
        ),
    ]
}

/// `break` or `continue` used outside a loop body.
pub fn loop_control_outside_loop(keyword: &str) -> Vec<String> {
    vec![
        format!(
            "`{}` is only meaningful inside a loop body — there \
             is no loop to {} here.",
            keyword, keyword,
        ),
        "vāṇी's `break` exits the nearest enclosing `while` loop; \
         `continue` skips to its next iteration. Both are \
         rejected at the top level or inside an `if` / `match` \
         arm that isn't nested inside a loop."
            .to_string(),
        format!(
            "Move the `{}` inside a `while` loop body, or \
             restructure with a boolean flag if you need to stop \
             iteration from outside the loop.",
            keyword,
        ),
    ]
}

/// `print` on a composite type that has no text representation.
pub fn cannot_print_type(ty: &str) -> Vec<String> {
    vec![
        format!(
            "Type `{}` cannot be printed directly with `print`.",
            ty,
        ),
        "vāṇी's `print` statement accepts scalar values (i64, \
         f64, bool, Str, OwnedStr) — composite types (arrays, \
         Vec, structs, tuples, enums) don't have a canonical \
         text representation, so the compiler rejects them to \
         avoid silent truncation."
            .to_string(),
        "Extract the scalar you want to display: index an array \
         or Vec (`xs[i]`), access a struct field (`p.x`), access \
         a tuple element (`t.0`), or use `match` to turn an enum \
         variant into a string or integer before printing."
            .to_string(),
    ]
}

/// Statement or condition that can never execute (dead code).
pub fn unreachable_code(reason: &str) -> Vec<String> {
    vec![
        format!("This code is unreachable: {}.", reason),
        "vāṇी's control-flow analysis tracks every exit point \
         (return, break, continue) and constant-folds boolean \
         conditions. Code that follows an unconditional exit, or \
         sits inside a branch whose condition is always false, \
         can never run — the compiler rejects it rather than \
         silently compiling dead code."
            .to_string(),
        "Either remove the dead statement/branch, or fix the \
         preceding control-flow (e.g. add a `return` only on \
         specific paths, or correct the constant condition)."
            .to_string(),
    ]
}

/// `const` name is already used by another const in scope.
pub fn const_already_declared(name: &str) -> Vec<String> {
    vec![
        format!(
            "A constant named `{}` is already declared in this \
             program.",
            name,
        ),
        "vāṇी resolves all constant names globally at compile time. \
         Two `const` declarations with the same name are ambiguous."
            .to_string(),
        format!(
            "Rename one of the `const {}` declarations to a \
             distinct identifier.",
            name,
        ),
    ]
}

/// `const` type is not a Copy scalar.
pub fn const_type_not_scalar(name: &str, ty: &str) -> Vec<String> {
    vec![
        format!(
            "Constant `{}` has type `{}`, which is not a Copy \
             scalar — only `i64`, `i32`, `i16`, `i8`, `u64`, \
             `u32`, `u16`, `u8`, `f64`, `f32`, and `bool` are \
             supported as const types in v1.",
            name, ty,
        ),
        "Constants in v1 are folded into every use site at \
         compile time. Heap-owning types (Vec, OwnedStr, struct) \
         can't be duplicated this way — they'd need per-use-site \
         allocation. Copy scalars have no heap and can be freely \
         substituted."
            .to_string(),
        format!(
            "Either change `{}` to a Copy scalar type, or store \
             the value in a `let` binding inside `main()` where \
             heap allocation is allowed.",
            name,
        ),
    ]
}

/// `const` initializer is not a literal value.
pub fn const_literal_required(name: &str) -> Vec<String> {
    vec![
        format!(
            "Constant `{}` must be initialized with a literal \
             value (e.g. `42`, `-1`, `3.14`, `true`). Arithmetic \
             expressions and function calls are not supported in \
             const initializers in v1.",
            name,
        ),
        "v1 const-folding uses a simple literal evaluator — it \
         handles negated integer/float literals but not general \
         expressions. Full const-expression evaluation (like \
         Rust's `const fn`) is planned for a future release."
            .to_string(),
        format!(
            "Replace the expression with a pre-computed literal. \
             If the value is derived from arithmetic, compute it \
             by hand and write the result directly as `const {}: \
             <T> = <literal>;`.",
            name,
        ),
    ]
}

/// `const` integer literal doesn't fit in the declared type.
pub fn const_value_out_of_range(name: &str, ty: &str) -> Vec<String> {
    vec![
        format!(
            "The literal value for constant `{}` doesn't fit in \
             type `{}`.",
            name, ty,
        ),
        "vāṇी range-checks integer constants at compile time so \
         the truncation that would happen in C (e.g. `const int8 \
         X = 200;` silently wrapping to -56) is caught before \
         codegen."
            .to_string(),
        format!(
            "Either choose a smaller literal that fits in `{}`, \
             or widen the type (e.g. change `{}` to `i64`).",
            ty, ty,
        ),
    ]
}

/// Type cast that the compiler cannot lower.
pub fn cast_unsupported(from_ty: &str, to_ty: &str) -> Vec<String> {
    vec![
        format!(
            "Cannot cast from `{}` to `{}`.", from_ty, to_ty,
        ),
        "vāṇी's `as` cast supports numeric conversions between \
         integer widths and between integer and float types. \
         Casts between unrelated types (string ↔ integer, struct \
         ↔ integer, bool ↔ float, etc.) are rejected because they \
         have no well-defined portable meaning."
            .to_string(),
        "Use an explicit conversion function instead: \
         `i64_to_str(n)` to format a number as a string, \
         `if b { 1 } else { 0 }` to turn a bool into an integer, \
         or a field accessor to extract a numeric value from a \
         struct."
            .to_string(),
    ]
}

/// Auto-borrowing a freshly-owned `OwnedStr` into a `Str`-typed
/// struct field (via `h.f = owned_expr;` or `Struct { f: owned_expr,
/// .. }`) -- rejected because the struct can outlive the `OwnedStr`
/// source's own scope. BUG-158 (2026-08-10): found via a corpus-wide
/// ASan sweep as a real, general (non-async) heap-use-after-free --
/// `{ let owned: OwnedStr = f(); h.f = owned; }` frees `owned` at the
/// block's end while `h.f` (an alias into the same buffer) is still
/// readable afterward.
pub fn owned_str_escape_into_field(field: &str) -> Vec<String> {
    vec![
        format!(
            "Field `{}` is declared `Str` (a non-owning view), but the value \
             being stored is a freshly-computed `OwnedStr` (a heap-owning \
             value) -- copying its pointer into the field doesn't transfer \
             ownership, and the struct holding the field can easily outlive \
             the `OwnedStr` source's own scope.",
            field
        ),
        "When that happens, the `OwnedStr` source is freed at the end of \
         its own scope while the struct's field still points at the same \
         (now-freed) buffer -- a genuine use-after-free the first time the \
         field is read afterward, not just a leak."
            .to_string(),
        "Either declare the field as `OwnedStr` instead of `Str` so it \
         owns its own copy (the struct's Drop then frees it correctly), or \
         store a `Str` value that's actually safe to view long-term (a \
         string literal, or a binding you know outlives the struct)."
            .to_string(),
    ]
}

/// A built-in function received an argument of the wrong type.
pub fn builtin_wrong_arg_type() -> Vec<String> {
    vec![
        "The argument type does not match what this built-in \
         function expects."
            .to_string(),
        "vāṇी's built-in collection and utility functions are \
         monomorphized for specific concrete types in v1 (e.g. \
         `Vec<i64>`, `ref Vec<i64>`, `mut ref Vec<i64>`). There \
         is no implicit coercion or type widening."
            .to_string(),
        "Check the expected argument type in the error message \
         and adjust the call. Common fixes:\n\
         \n\
         • If the function needs a reference, pass `ref xs` \
           instead of `xs`.\n\
         • If it needs a mutable reference, pass `mut ref xs`.\n\
         • If the element type is wrong, make sure the Vec or \
           collection was declared with `i64` elements."
            .to_string(),
    ]
}

/// A match arm is unreachable because a wildcard `_` arm already
/// appeared above it and covers every remaining case.
pub fn match_arm_after_wildcard() -> Vec<String> {
    vec![
        "This arm can never be reached: an earlier `_ then …` \
         wildcard arm already matches every value not handled by \
         the preceding arms."
            .to_string(),
        "In vāṇी a `match` expression evaluates arms top-to-bottom \
         and stops at the first match. Once a wildcard is placed, \
         no arm below it can ever fire — the compiler rejects this \
         to prevent confusion."
            .to_string(),
        "Either (a) remove this arm if it is truly redundant, \
         (b) move it above the wildcard if it should take priority, \
         or (c) replace the wildcard with explicit patterns that do \
         not cover this arm's value."
            .to_string(),
    ]
}

/// Reference to an enum type that has never been declared.
pub fn enum_not_declared(name: &str) -> Vec<String> {
    vec![
        format!(
            "No `enum {}` declaration is in scope here.",
            name,
        ),
        "vāṇी resolves every type name at compile time. The \
         enum must be declared with an `enum` block before it \
         can appear in a pattern or as a match scrutinee — \
         forward references are not supported in v1."
            .to_string(),
        format!(
            "Either (a) check for a typo (compare against the \
             `enum` declaration name), (b) add `include \"path.vani\";` \
             for the file that defines `{}`, or (c) declare the \
             enum above this use:\n\
             \n\
             enum {} {{ Variant1, Variant2 }}",
            name, name,
        ),
    ]
}

pub fn struct_field_error(struct_name: &str, field_name: &str) -> Vec<String> {
    vec![
        format!(
            "A field in struct '{}' violates a v1 layout restriction.",
            struct_name,
        ),
        "vāṇी v1 enforces field-type constraints on struct declarations to \
         ensure deterministic drop semantics. Non-Copy types that lack a \
         generated drop path, duplicate field names, or too many fields all \
         trigger this diagnostic."
            .to_string(),
        format!(
            "Check the field '{}' and the struct layout rules in the docs. \
             Use only supported field types (Copy scalars, OwnedStr, \
             Vec<T>, [T; N] of Copy, Task, Atomic<T>, Mutex<T>, \
             Channel<T, N>, Box<T>). Rename or remove duplicates.",
            field_name,
        ),
    ]
}

pub fn reserved_type_name(name: &str) -> Vec<String> {
    vec![
        format!(
            "'{}' is a built-in type name reserved by the vāṇी runtime.",
            name,
        ),
        "The compiler hard-codes this identifier as a built-in type; \
         any struct, enum, or alias with the same name would shadow the \
         built-in in confusing ways. The type checker rejects the \
         conflict early to prevent hard-to-diagnose downstream errors."
            .to_string(),
        format!(
            "Rename the declaration to something that doesn't clash with \
             the built-in name '{}'. For example, append a domain qualifier: \
             `My{}`, `App{}`, or a more specific name.",
            name, name, name,
        ),
    ]
}

pub fn non_generic_where_clause(fn_name: &str) -> Vec<String> {
    vec![
        format!(
            "Function '{}' has a `where` clause but no type parameters.",
            fn_name,
        ),
        "`where T is Iface` bounds constrain generic type parameters — \
         they have no meaning when the function is not generic. \
         vāṇी rejects spurious where-clauses to prevent silent \
         over-specification."
            .to_string(),
        format!(
            "Either add type parameters to '{}' (e.g. `fn {}[T](...)`) \
             and keep the where-clause, or remove the where-clause entirely \
             if the function is meant to be concrete.",
            fn_name, fn_name,
        ),
    ]
}

pub fn proof_failed() -> Vec<String> {
    vec![
        "A `prove` statement could not be verified — the expression is always false or the SMT solver found a counterexample.".to_string(),
        "vāṇī's `prove` statement is a compile-time assertion that an expression holds for \
         all inputs satisfying the current `requires` preconditions. The compiler uses an \
         SMT solver to verify this; if the expression evaluates to a constant `false` or \
         the solver returns UNSAT, the proof fails."
            .to_string(),
        "Check the boolean expression in the `prove` statement and your `requires` \
         preconditions. Ensure the expression can be true under the preconditions. \
         If you need to prove something conditional, add the missing precondition to \
         `requires`, or break the proof into smaller steps with intermediate `assert` \
         statements. Run with `VANIC_SMT_DEBUG=1` to see exactly which sub-goal the \
         solver couldn't discharge."
            .to_string(),
    ]
}

pub fn move_nested_field() -> Vec<String> {
    vec![
        "Moving a non-Copy value out of a nested field path (`obj.inner.field`) is not yet supported.".to_string(),
        "vāṇī v1 tracks moved fields one level deep. A nested move like `obj.inner.field` \
         would require tracking that `obj.inner` is partially moved, which is not \
         implemented yet. Allowing it would risk a double-free at the outer struct's drop."
            .to_string(),
        "Move the intermediate struct out first: `let inner = obj.inner;` then \
         `let field = inner.field;`. This two-step sequence is fully supported and \
         keeps the ownership chain explicit."
            .to_string(),
    ]
}

pub fn raw_ptr_escape() -> Vec<String> {
    vec![
        "A raw pointer or ArenaRef derived from a stack-local cannot escape the current function frame.".to_string(),
        "vāṇī's Layer 1 safety analysis tracks raw pointers and ArenaRefs that originate \
         from stack-local storage. When such a pointer is returned, stored into a heap \
         container (Vec slot, struct field), or passed in a way that outlives the frame, \
         the pointed-to storage is already freed — creating a dangling pointer. The \
         compiler rejects these patterns at compile time."
            .to_string(),
        "Pass the value downward to a function that only borrows it and doesn't store it \
         beyond the call. For long-lived references, use `Handle<T>` obtained from a \
         `Pool<T>` allocation (Layer 2 of `unsafe.md`). Alternatively, heap-allocate the \
         value with `Box<T>` so its lifetime is not tied to the frame."
            .to_string(),
    ]
}

pub fn task_affine(task_name: &str) -> Vec<String> {
    vec![
        format!(
            "Task handle '{}' violates the affine (use-exactly-once) ownership rule.",
            task_name,
        ),
        "Every `task` handle spawned in a block must be consumed by exactly one \
         `join` in the same block, after the spawn. Spawning twice, joining without \
         spawning, joining twice, or leaving a handle unjoined all break the affine \
         discipline that ensures every task completes before the enclosing scope exits."
            .to_string(),
        format!(
            "Ensure '{}' is spawned exactly once and joined exactly once, in block order. \
             Cross-block joins are not supported in v1 — the spawn and join must appear \
             in the same statement list. Remove duplicate spawns or joins as appropriate.",
            task_name,
        ),
    ]
}

pub fn move_in_loop(var: &str) -> Vec<String> {
    vec![
        format!(
            "Variable '{}' has a different move/ownership state at loop entry versus loop exit.",
            var,
        ),
        "vāṇī's ownership checker requires that non-Copy bindings have the same move state on \
         every path through a loop body. If the variable is moved on one iteration, it would be \
         unavailable on the next, making the loop unsound."
            .to_string(),
        format!(
            "Either consume '{}' consistently on every path through the loop (rebind or move \
             it the same way on both the taken and not-taken branches), or move it out of the \
             loop entirely. If you need a fresh value each iteration, rebind it with `let` \
             inside the loop body.",
            var,
        ),
    ]
}

pub fn contract_unsatisfiable(fn_name: &str) -> Vec<String> {
    vec![
        format!(
            "Function '{}' has contradictory `requires` clauses — the preconditions can never all be true simultaneously.",
            fn_name,
        ),
        "vāṇī's SMT verifier checks that the conjunction of all `requires` conditions is \
         satisfiable. If the solver returns UNSAT, no valid input can reach the function \
         body — every proof inside it is vacuously true and the call site is unreachable. \
         This is almost always a logic error in the contracts."
            .to_string(),
        format!(
            "Review the `requires` clauses on '{}' and remove contradictory conditions. \
             For example, `requires x > 0` and `requires x < 0` are mutually exclusive. \
             Use a single clause that expresses the true precondition.",
            fn_name,
        ),
    ]
}

pub fn extern_ffi_type() -> Vec<String> {
    vec![
        "An `extern` function parameter or return type is not supported by the FFI ABI."
            .to_string(),
        "vāṇī's v1 FFI bridge supports only scalar types (i64, f64, bool), `Str`, and \
         reference/pointer types as `extern fn` parameters and return values. Passing \
         structs, tuples, `Vec<T>`, or `OwnedStr` by value does not match the \
         System V x86-64 calling convention and would silently corrupt register state."
            .to_string(),
        "Use a reference (`ref T` or `mut ref T`) for struct/aggregate parameters, or \
         restructure the FFI boundary so only scalar and pointer types cross. Refer to \
         the vāṇī FFI guide for the full list of supported ABI types."
            .to_string(),
    ]
}

pub fn generic_infer_failure() -> Vec<String> {
    vec![
        "The monomorphizer could not infer the concrete type for a generic type parameter."
            .to_string(),
        "vāṇī v1 monomorphizes generic functions by inspecting call-site argument types at \
         a pre-type-check pass. It can only resolve type parameters from literal values \
         (integer, float, bool), directly-named variables whose types are declared by a \
         `let`-annotation or function parameter, or (v3.1 only) `ref`/`mut ref` of such \
         a variable. Arbitrary sub-expressions, function-call results, and struct-field \
         accesses are not yet supported."
            .to_string(),
        "Ensure the first generic-typed argument at the call site is either a literal or \
         a named variable with a type annotation (e.g. `let x: i64 = ...;`). If the value \
         comes from a complex expression, bind it to an annotated variable first, then \
         pass that variable to the generic function."
            .to_string(),
    ]
}

pub fn try_desugar_restricted() -> Vec<String> {
    vec![
        "The `try` expression cannot be used in this position or with this return type."
            .to_string(),
        "`try` in vāṇī v1 is a limited desugar: the enclosing function must return an \
         enum with exactly one payloaded variant (the Ok/Some arm) and one payload-less \
         variant (the Err/None arm). Between the `try`-let and the final `return`, only \
         `let`, `print`, and simple reassignments are permitted — control-flow statements \
         like `if`, `while`, and `for` are not yet supported (T2.6 phase 2)."
            .to_string(),
        "Ensure the function returns `Option<T>` or a two-variant enum that matches the \
         required shape. Move any conditional logic around the `try` expression outside \
         the restricted zone, or refactor to use explicit match arms instead of `try`."
            .to_string(),
    ]
}

pub fn const_expr_overflow() -> Vec<String> {
    vec![
        "A constant sub-expression overflowed its result type at compile time.".to_string(),
        "vāṇी evaluates constant expressions at compile time to fold literals and \
         detect definite errors early. Overflow, division by zero, or a value \
         that does not fit in the declared type all make the expression \
         non-representable."
            .to_string(),
        "Check the operands and make sure they stay within the range of the result \
         type. Use an explicit cast (e.g. `as i64`) if you need a wider type, \
         or guard division/remainder with a non-zero denominator check."
            .to_string(),
    ]
}

/// BUG-139 (2026-08-07): a struct field, enum variant payload, or
/// function parameter/return type that names a struct/enum which was
/// never declared. Distinct from `unknown_struct_type` (a runtime
/// construction/field-access site, always a struct specifically) --
/// this fires at DECLARATION time, before the name is known to be
/// meant as a struct vs. an enum, and before it's known to be a typo
/// vs. a genuinely missing declaration vs. a Rust-ism the user
/// expected to exist (`String` instead of `Str`/`OwnedStr` is the
/// single most common case found via localfuzz).
pub fn unknown_type_in_declaration(name: &str) -> Vec<String> {
    vec![
        format!(
            "No struct, enum, or interface named `{}` is declared anywhere in this program.",
            name,
        ),
        "vāṇी resolves every named type at compile time against the program's own \
         struct/enum/interface declarations plus its fixed set of built-in types \
         (i8..i64, u8..u64, f32, f64, bool, Str, OwnedStr, Vec<T>, and friends) -- \
         there is no implicit import and no type inherited from another language."
            .to_string(),
        format!(
            "Either (a) typo -- compare `{}` against the closest real declaration \
             or built-in type name (a common one: vāṇी's owned string type is \
             `OwnedStr`, not `String`), (b) the type is declared in another file \
             -- add `include \"path.vani\";` at the top, or (c) the type hasn't \
             been declared yet -- add `struct {} {{ … }}` or `enum {} {{ … }}` \
             above this use.",
            name, name, name,
        ),
    ]
}
