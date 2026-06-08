//! C backend.
//!
//! # TODO(deprecate): C backend slated for removal.
//!
//! LLVM is the project's default and primary backend
//! ([src/backend_llvm.rs](backend_llvm.rs)). The C backend is kept
//! for back-compat — `intentc emit-c` and `intentc emit --backend=c`
//! still work, and the `llvm_backend_run_produces_same_output_as_c`
//! integration test diffs the two on every example to guard against
//! divergence.
//!
//! When the LLVM backend has had enough run time in production:
//! - Remove `CBackend` and this module.
//! - Drop the `emit-c` subcommand alias from `src/main.rs`.
//! - Drop the `--backend=c` path from `parse_emit_args`.
//! - Retire the cross-backend equivalence test (it'll have no C path
//!   to compare against).
//! - Audit and remove the C-pinned tests in `lib.rs` that assert on
//!   `intent_check_*` / `v_*` C-specific identifiers.

use crate::ast::{BinaryOp, Type, UnaryOp};
use crate::backend::Backend;
use crate::ir::{TypedExpr, TypedExprKind, TypedFunction, TypedProgram, TypedStmt};
use std::collections::BTreeSet;

pub struct CBackend;

impl Backend for CBackend {
    fn name(&self) -> &'static str {
        "c"
    }

    fn emit(&self, program: &TypedProgram) -> String {
        emit_c(program)
    }
}

thread_local! {
    /// Per-program buffer for outlined task bodies. emit_stmt
    /// for `TypedStmt::TaskSpawn` appends one `static void*
    /// intent_task_<n>(void* ctx_raw) { … }` per spawn site
    /// here; emit_c prepends the buffer between the runtime
    /// preamble and the user functions so the outline name
    /// is visible at the spawn-site call.
    static TASK_OUTLINES: std::cell::RefCell<String> = std::cell::RefCell::new(String::new());
    /// Monotonic counter assigning outline IDs. Reset at the
    /// start of every `emit_c` call.
    static TASK_OUTLINE_COUNTER: std::cell::Cell<u32> = std::cell::Cell::new(0);
    /// Closure #269: set of `extern "C"` fn names. Populated at
    /// the start of `emit_c` from any `is_extern` function in
    /// the program. Consulted by the Call emitter to choose the
    /// bare C-ABI name (no `fn_` prefix).
    pub(crate) static C_EXTERN_FN_REGISTRY:
        std::cell::RefCell<std::collections::HashSet<String>> =
        std::cell::RefCell::new(std::collections::HashSet::new());
    /// Per-program registry of enum payload types. Populated
    /// at the start of `emit_c` from `program.enums`. Maps
    /// each enum name → `Some(payload_ty)` if any variant has
    /// a payload (v1 requires all payloaded variants to share
    /// the same payload type), or `None` for plain enums.
    /// Consulted by `c_type_name(Type::Enum)` so payloaded
    /// enums route to the tagged-union struct typedef instead
    /// of the bare `int32_t` tag. T1.3 phase 2b.
    static ENUM_PAYLOAD_REGISTRY: std::cell::RefCell<std::collections::HashMap<String, Type>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
    /// Per-program registry of struct field lists. Populated at
    /// the start of `emit_c` from `program.structs` and consulted
    /// by the `TypedStmt::Drop` handler to free each owning
    /// (`OwnedStr`) field when the struct binding goes out of
    /// scope. T1.2 phase 2b.
    static STRUCT_FIELDS_REGISTRY: std::cell::RefCell<std::collections::HashMap<String, Vec<(String, Type)>>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
    /// Names of structs / enums that have an `implement Drop
    /// for T` impl in the program (hoisted to `T_drop`).
    /// Populated at the start of `emit_c` from the function
    /// table. Consulted by the `TypedStmt::Drop` handler to
    /// auto-call the user's `drop(self)` method at scope exit
    /// when the type has no owning fields. T2.7 phase 2.
    static USER_DROP_REGISTRY: std::cell::RefCell<std::collections::HashSet<String>> =
        std::cell::RefCell::new(std::collections::HashSet::new());
    /// Per-enum list of variant tags that carry a payload.
    /// Populated alongside `ENUM_PAYLOAD_REGISTRY` at the start
    /// of `emit_c`. The Drop handler reads this to switch on
    /// the active tag and free the heap payload only when one
    /// of the listed variants is in scope. T1.3 + T1.2 phase 2b.
    static ENUM_PAYLOAD_TAGS_REGISTRY:
        std::cell::RefCell<std::collections::HashMap<String, Vec<u32>>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
    /// Closure #283: per-variant payload-type registry,
    /// keyed by enum name. Stores `Vec<(variant_name,
    /// Option<Type>)>` in declaration order. Populated for
    /// every enum that has at least one payloaded variant.
    /// Enables mixed-payload-type enums (e.g. `Result<T,
    /// E> { Ok(T), Err(E) }`) by routing each variant's
    /// payload through its own union member, rather than
    /// forcing all variants to share a single payload type.
    pub(crate) static ENUM_VARIANT_PAYLOADS_REGISTRY:
        std::cell::RefCell<
            std::collections::HashMap<String, Vec<(String, Option<Type>)>>
        > = std::cell::RefCell::new(std::collections::HashMap::new());
}

/// Per-name C struct typedef for a payloaded enum. Prefixed
/// with `Enum_` so the emitted C identifier is distinct from
/// any builtin. T1.3 phase 2b.
pub(crate) fn enum_c_name(name: &str) -> String {
    format!("Enum_{}", name)
}

/// Return true if any variant of this enum carries a payload.
/// T1.3 phase 2b.
fn enum_has_payload(decl: &crate::ir::TypedEnumDecl) -> bool {
    decl.payload_types.iter().any(|p| p.is_some())
}

/// Closure #283: true if this enum carries variants with
/// payloads of differing types (e.g. `Result<i64, OwnedStr>`
/// where Ok carries i64 and Err carries OwnedStr). Triggers
/// the new union-layout codegen path. Single-payload-type
/// enums stay on the legacy `{ tag; T payload; }` layout for
/// back-compat (no test breakage).
fn enum_has_mixed_payloads(decl: &crate::ir::TypedEnumDecl) -> bool {
    let payloaded: Vec<&Type> = decl
        .payload_types
        .iter()
        .filter_map(|p| p.as_ref())
        .collect();
    if payloaded.len() < 2 {
        return false;
    }
    let first = payloaded[0];
    payloaded[1..].iter().any(|t| *t != first)
}

/// Closure #283: C identifier for the per-variant union
/// member name. `v_Ok`, `v_Err`, etc. Mirrored on the LLVM
/// side via `intent_enum_v_<variant>`-style globals.
pub(crate) fn enum_variant_member(variant: &str) -> String {
    format!("v_{}", variant)
}

/// Common payload type across all payloaded variants of the
/// enum. Returns None for payload-less enums. Assumes the
/// checker has already validated uniformity. T1.3 phase 2b.
fn enum_common_payload_ty(decl: &crate::ir::TypedEnumDecl) -> Option<Type> {
    decl.payload_types.iter().find_map(|p| p.clone())
}

pub fn emit_c(program: &TypedProgram) -> String {
    TASK_OUTLINES.with(|b| b.borrow_mut().clear());
    TASK_OUTLINE_COUNTER.with(|c| c.set(0));
    // Closure #269: populate the extern-fn registry from the
    // program's extern declarations. The Call emitter consults
    // this to skip the `fn_` prefix on calls to FFI symbols.
    C_EXTERN_FN_REGISTRY.with(|r| {
        let mut reg = r.borrow_mut();
        reg.clear();
        for f in &program.functions {
            if f.is_extern {
                reg.insert(f.name.clone());
            }
        }
    });
    // Populate the enum payload registry from the program's
    // enum decls so `c_type_name(Type::Enum)` routes
    // payloaded enums to their tagged-union struct typedef.
    // T1.3 phase 2b.
    ENUM_PAYLOAD_REGISTRY.with(|r| {
        let mut reg = r.borrow_mut();
        reg.clear();
        for decl in &program.enums {
            if let Some(payload_ty) = enum_common_payload_ty(decl) {
                reg.insert(decl.name.clone(), payload_ty);
            }
        }
    });
    ENUM_PAYLOAD_TAGS_REGISTRY.with(|r| {
        let mut reg = r.borrow_mut();
        reg.clear();
        for decl in &program.enums {
            let tags: Vec<u32> = decl
                .payload_types
                .iter()
                .enumerate()
                .filter_map(|(i, p)| p.as_ref().map(|_| i as u32))
                .collect();
            if !tags.is_empty() {
                reg.insert(decl.name.clone(), tags);
            }
        }
    });
    // Closure #283: per-variant payload registry. Populated
    // for every payloaded enum (uniform OR mixed) so the
    // codegen sites can choose the right access pattern.
    ENUM_VARIANT_PAYLOADS_REGISTRY.with(|r| {
        let mut reg = r.borrow_mut();
        reg.clear();
        for decl in &program.enums {
            if !enum_has_payload(decl) {
                continue;
            }
            let pairs: Vec<(String, Option<Type>)> = decl
                .variants
                .iter()
                .zip(decl.payload_types.iter())
                .map(|(name, pty)| (name.clone(), pty.clone()))
                .collect();
            reg.insert(decl.name.clone(), pairs);
        }
    });
    STRUCT_FIELDS_REGISTRY.with(|r| {
        let mut reg = r.borrow_mut();
        reg.clear();
        for decl in &program.structs {
            reg.insert(decl.name.clone(), decl.fields.clone());
        }
    });
    USER_DROP_REGISTRY.with(|r| {
        let mut reg = r.borrow_mut();
        reg.clear();
        for f in &program.functions {
            if let Some(type_name) = f.name.strip_suffix("_drop") {
                reg.insert(type_name.to_string());
            }
        }
    });
    // Emit the body first (Vec bundles + intents + functions + main),
    // then prepend includes + only the runtime helpers it actually
    // references. Keeps the generated C tidy when SMT elision discharges
    // all the runtime guards.
    let mut body = String::new();

    let mut vec_elements = BTreeSet::<String>::new();
    let mut element_types: Vec<Type> = Vec::new();
    let mut channel_seen = BTreeSet::<String>::new();
    let mut channel_specs: Vec<(Type, u64)> = Vec::new();
    let mut tuple_seen = BTreeSet::<String>::new();
    let mut tuple_shapes: Vec<Vec<Type>> = Vec::new();
    for function in &program.functions {
        collect_vec_elements(&function.return_type, &mut vec_elements, &mut element_types);
        collect_channel_specs(
            &function.return_type,
            &mut channel_seen,
            &mut channel_specs,
        );
        collect_tuple_shapes(
            &function.return_type,
            &mut tuple_seen,
            &mut tuple_shapes,
        );
        for param in &function.params {
            collect_vec_elements(&param.ty, &mut vec_elements, &mut element_types);
            collect_channel_specs(&param.ty, &mut channel_seen, &mut channel_specs);
            collect_tuple_shapes(&param.ty, &mut tuple_seen, &mut tuple_shapes);
        }
        for stmt in &function.body {
            collect_vec_elements_in_stmt(stmt, &mut vec_elements, &mut element_types);
            collect_channel_specs_in_stmt(stmt, &mut channel_seen, &mut channel_specs);
            collect_tuple_shapes_in_stmt(stmt, &mut tuple_seen, &mut tuple_shapes);
        }
    }
    // Collect any Vec element types referenced from struct
    // fields and emit those Vec bundles BEFORE the struct
    // typedefs, so a `struct Bag { contents: Vec<i64> }`
    // resolves `intent_vec_int64_t` at its own declaration.
    // Track the early-emitted set so the post-struct pass
    // doesn't re-emit the same bundle. T1.2 phase 2b.
    let mut struct_field_vec_seen = BTreeSet::<String>::new();
    let mut struct_field_vec_elements: Vec<Type> = Vec::new();
    for decl in &program.structs {
        for (_, fty) in &decl.fields {
            collect_vec_elements(fty, &mut struct_field_vec_seen, &mut struct_field_vec_elements);
        }
    }
    // Enum payload types may also be Vec<T>. Walk
    // `program.enums` for each payloaded variant and queue
    // any Vec element types so the bundle is in scope when
    // the `typedef struct { int32_t tag; intent_vec_<T>
    // payload; } Enum_<Name>;` line lands further below.
    // Closure #118.
    for decl in &program.enums {
        for payload in &decl.payload_types {
            if let Some(ty) = payload {
                collect_vec_elements(ty, &mut struct_field_vec_seen, &mut struct_field_vec_elements);
            }
        }
    }
    // Vec-bundle emit is now SPLIT into two phases (2026-06-06):
    //   1. Vec<primitive> bundles (no user-struct deps) emit HERE,
    //      same position as before. Enums + structs further below
    //      may reference them via their payload / field types.
    //   2. Vec<UserStruct> bundles are deferred into the UNIFIED
    //      topological walk further down, alongside struct typedefs.
    //      Pre-existing bug fix: `struct Holder { items: Vec<Point> }`
    //      used to fail on C because intent_vec_Struct_Point's
    //      typedef (referencing Struct_Point* + sizeof(Struct_Point))
    //      emitted BEFORE Struct_Point's typedef. The unified
    //      interleaving fixes this without breaking enums-with-
    //      Vec<primitive>-payload (closure #118).
    fn vec_element_has_user_struct(ty: &Type) -> bool {
        match ty {
            Type::Struct(_) => true,
            // Phase 1.2 (2026-06-07): `dyn Iface` fat-pointer
            // typedef is emitted by `emit_dyn_iface_typedefs`
            // BELOW the early Vec-bundle pass, so any
            // Vec<dyn Iface> bundle must also be deferred to
            // the unified topo loop (which runs after the dyn
            // typedefs land). Closes L8.
            Type::Object(_) => true,
            Type::Vec(inner)
            | Type::Array { element: inner, .. } => vec_element_has_user_struct(inner),
            _ => false,
        }
    }
    let mut emitted_vec_bundles: BTreeSet<String> = BTreeSet::new();
    for element in &struct_field_vec_elements {
        if vec_element_has_user_struct(element) {
            continue; // Deferred to the unified topo loop below.
        }
        emit_vec_bundle(element, &mut body);
        emitted_vec_bundles.insert(element_tag(element));
    }
    if !struct_field_vec_elements.is_empty() {
        // Newline trigger preserved for the case where ALL Vec
        // bundles are primitive (everything emitted in this pass).
        body.push('\n');
    }
    // Vtables Phase 4: forward-declare the per-Iface vtable
    // tag + `intent_dyn_<Iface>` fat-pointer typedef BEFORE
    // struct typedefs so structs can carry `dyn Iface`
    // fields. The full vtable struct body is emitted AFTER
    // struct typedefs by `emit_dyn_iface_vtable_bodies`
    // (its fn-ptr slots may reference `Struct_<T>` types).
    let used_dyn_ifaces_forward = collect_used_dyn_ifaces(program);
    emit_dyn_iface_typedefs(&mut body, &used_dyn_ifaces_forward);
    if !used_dyn_ifaces_forward.is_empty() {
        body.push('\n');
    }
    // Phase 3e fix (and pre-existing: structs containing
    // payloaded-enum fields): emit payloaded enum typedefs
    // BEFORE struct typedefs so a struct field of type
    // `Enum_<Name>` resolves at declaration time. The same
    // typedef bodies are emitted later (see the duplicated
    // block below) — tracked in `enum_typedefs_pre_emitted`
    // so the second pass skips them. Assumes no enum payload
    // depends on a struct (a true Struct-in-Enum case would
    // need topological sort across both kinds; track via
    // ARC8_V3_PLAN.md if hit).
    let mut enum_typedefs_pre_emitted: std::collections::HashSet<String> =
        std::collections::HashSet::new();
    {
        let mut any_pre = false;
        for decl in &program.enums {
            if !enum_has_payload(decl) {
                continue;
            }
            if enum_has_mixed_payloads(decl) {
                body.push_str("typedef struct { int32_t tag; union {\n");
                for (variant, pty) in decl.variants.iter().zip(decl.payload_types.iter()) {
                    let Some(payload_ty) = pty.as_ref() else { continue; };
                    let member = enum_variant_member(variant);
                    let payload_decl = match payload_ty {
                        Type::Array { .. } => format_declarator(payload_ty, &member),
                        _ => format!("{} {}", c_element_storage(payload_ty), member),
                    };
                    body.push_str(&format!("    {};\n", payload_decl));
                }
                body.push_str(&format!("}} u; }} {};\n", enum_c_name(&decl.name)));
                enum_typedefs_pre_emitted.insert(decl.name.clone());
                any_pre = true;
                continue;
            }
            let Some(payload_ty) = enum_common_payload_ty(decl) else { continue; };
            let payload_decl = match &payload_ty {
                Type::Array { .. } => format_declarator(&payload_ty, "payload"),
                _ => format!("{} payload", c_type_name(&payload_ty)),
            };
            body.push_str(&format!(
                "typedef struct {{ int32_t tag; {}; }} {};\n",
                payload_decl,
                enum_c_name(&decl.name)
            ));
            enum_typedefs_pre_emitted.insert(decl.name.clone());
            any_pre = true;
        }
        if any_pre {
            body.push('\n');
        }
    }
    // UNIFIED TOPOLOGICAL EMIT for user structs + Vec bundles.
    // Pre-existing Vec<Struct>-in-struct-field bug fix
    // (2026-06-06): the previous emit order (Vec bundles first,
    // then struct typedefs) failed for `struct Holder { items:
    // Vec<Point> }` because intent_vec_Struct_Point references
    // Struct_Point + sizeof(Struct_Point) before Struct_Point's
    // typedef lands. Iterate-to-fixpoint emits each node only
    // when all its dependencies are already emitted:
    //   - Struct S depends on: each Struct in its field types
    //     (direct or via [T; N] / Vec<T>), AND intent_vec_T for
    //     each Vec<T> field (the field's spelling references
    //     the bundle typedef).
    //   - intent_vec_T bundle depends on: any user struct
    //     referenced by T (recursively through Vec<Vec<X>> /
    //     [Vec<X>; N] / etc.).
    // Closure #164.
    fn struct_deps_in_ty(ty: &Type, out: &mut Vec<String>) {
        match ty {
            Type::Struct(name) => out.push(name.clone()),
            Type::Array { element, .. } => struct_deps_in_ty(element, out),
            Type::Vec(element) => struct_deps_in_ty(element, out),
            _ => {}
        }
    }
    // Vec bundle tags that this struct's fields reference. Each
    // such Vec<T> bundle's typedef must be emitted before the
    // struct can spell `intent_vec_<T> fieldname;`.
    fn vec_bundle_deps_in_ty(ty: &Type, out: &mut Vec<String>) {
        match ty {
            Type::Vec(element) => {
                out.push(element_tag(element));
                vec_bundle_deps_in_ty(element, out);
            }
            Type::Array { element, .. } => vec_bundle_deps_in_ty(element, out),
            _ => {}
        }
    }
    let by_name: std::collections::HashMap<&str, &crate::ir::TypedStructDecl> = program
        .structs
        .iter()
        .map(|d| (d.name.as_str(), d))
        .collect();
    let mut emitted_structs: std::collections::HashSet<String> =
        std::collections::HashSet::new();
    // Map from element_tag → element Type so the unified loop
    // can re-emit. Stable iteration order for deterministic output.
    let mut vec_elements_by_tag: std::collections::BTreeMap<String, Type> =
        std::collections::BTreeMap::new();
    for element in &struct_field_vec_elements {
        vec_elements_by_tag
            .entry(element_tag(element))
            .or_insert_with(|| element.clone());
    }
    let struct_decls: Vec<&crate::ir::TypedStructDecl> = program.structs.iter().collect();
    loop {
        let mut progress = false;
        // Try emit any pending Vec bundle whose struct deps are all
        // satisfied. (Primitive Vec bundles have no struct deps.)
        let pending_tags: Vec<String> = vec_elements_by_tag
            .keys()
            .filter(|t| !emitted_vec_bundles.contains(*t))
            .cloned()
            .collect();
        for tag in pending_tags {
            let element = vec_elements_by_tag.get(&tag).unwrap().clone();
            let mut deps: Vec<String> = Vec::new();
            struct_deps_in_ty(&element, &mut deps);
            if deps.iter().all(|d| emitted_structs.contains(d) || !by_name.contains_key(d.as_str())) {
                emit_vec_bundle(&element, &mut body);
                emitted_vec_bundles.insert(tag);
                progress = true;
            }
        }
        // Try emit any pending struct whose struct + vec-bundle deps
        // are all satisfied.
        for decl in &struct_decls {
            if emitted_structs.contains(&decl.name) {
                continue;
            }
            let mut sdeps: Vec<String> = Vec::new();
            let mut vdeps: Vec<String> = Vec::new();
            for (_, fty) in &decl.fields {
                struct_deps_in_ty(fty, &mut sdeps);
                vec_bundle_deps_in_ty(fty, &mut vdeps);
            }
            let sok = sdeps
                .iter()
                .all(|d| emitted_structs.contains(d) || !by_name.contains_key(d.as_str()));
            let vok = vdeps
                .iter()
                .all(|t| emitted_vec_bundles.contains(t) || !vec_elements_by_tag.contains_key(t));
            if sok && vok {
                emit_struct_bundle(decl, &mut body);
                emitted_structs.insert(decl.name.clone());
                progress = true;
            }
        }
        if !progress {
            break;
        }
    }
    if !program.structs.is_empty() || !struct_field_vec_elements.is_empty() {
        body.push('\n');
    }
    // Arc 5c: emit per-(args, ret) Closure fat-pointer struct
    // typedefs + per-closure trampolines. Scans
    // `CLOSURE_MAKE_REGISTRY` populated by the lift pass. Each
    // unique (args, ret) signature gets one typedef
    // `intent_closure_<args>_<ret>` shaped as
    // `{ uint64_t env; R (*call)(uint64_t env, args); }`.
    // Each registered closure also gets a trampoline that
    // casts the env-uint64 back to the env-struct, reads the
    // captures, and calls `__anon_fn_<N>(captures..., args)`.
    {
        use std::collections::HashMap as HM;
        let entries: Vec<(String, (String, String, Vec<Type>, Vec<Type>, Type))> =
            crate::ast::CLOSURE_MAKE_REGISTRY.with(|r| {
                r.borrow().iter().map(|(k, v)| (k.clone(), v.clone())).collect()
            });
        // Emit closure-struct typedef per unique (args, ret) shape.
        let mut emitted_closure_structs: HM<String, ()> = HM::new();
        for (_, (_, _, _, args, ret)) in &entries {
            let sname = closure_c_struct_name(args, ret);
            if emitted_closure_structs.insert(sname.clone(), ()).is_some() {
                continue;
            }
            // Build call-field type: R (*call)(uint64_t env, T1, T2, ...)
            let ret_c = c_leaf_type(ret);
            let mut arg_decls: Vec<String> = vec!["uint64_t".to_string()];
            for a in args {
                arg_decls.push(c_leaf_type(a).to_string());
            }
            body.push_str(&format!(
                "typedef struct {{ uint64_t env; {ret_c} (*call)({args}); }} {sname};\n",
                ret_c = ret_c,
                args = arg_decls.join(", "),
                sname = sname,
            ));
        }
        if !emitted_closure_structs.is_empty() {
            body.push('\n');
        }
        // Forward-declare each hoisted closure fn so the
        // trampolines below can call them without implicit
        // declaration warnings.
        for (_, (hoist_name, _, capture_types, args, ret)) in &entries {
            let ret_c = c_leaf_type(ret);
            let mut decl_params: Vec<String> = Vec::new();
            for cty in capture_types {
                decl_params.push(c_leaf_type(cty).to_string());
            }
            for cty in args {
                decl_params.push(c_leaf_type(cty).to_string());
            }
            body.push_str(&format!(
                "static {ret_c} fn_{hn}({params});\n",
                ret_c = ret_c,
                hn = hoist_name,
                params = decl_params.join(", "),
            ));
        }
        if !entries.is_empty() {
            body.push('\n');
        }
        // Emit trampoline + magic-call constructor per registered closure.
        // The constructor `fn___intent_make_closure_N(captures...)`
        // stack-allocates the env-struct and returns the Closure
        // by value. Naturally, returning a struct that contains
        // a pointer into a local stack-frame would dangle — so
        // we mark these as `static inline` and require the C
        // compiler to inline them at the call site. v1
        // restriction: the closure-binding's lifetime is the
        // enclosing fn's stack frame; passing it OUT of the fn
        // (return value / global / heap struct field) is undefined.
        for (magic_name, (hoist_name, env_struct_name, capture_types, args, ret)) in &entries {
            let sname = closure_c_struct_name(args, ret);
            let trampoline_name = format!("{}__trampoline", hoist_name);
            // Capture-arg-list for the trampoline body call:
            // env->cap_N for each capture, then the trampoline's
            // own args.
            let mut call_args: Vec<String> = Vec::new();
            // We don't have the capture *names* in scope here
            // (only types); but the env-struct's fields use the
            // ORIGINAL capture names. We can pull them from
            // program.structs which contains the env struct.
            let env_struct_decl = program.structs.iter()
                .find(|d| d.name == *env_struct_name)
                .cloned();
            let capture_names: Vec<String> = if let Some(d) = &env_struct_decl {
                d.fields.iter().map(|(n, _)| n.clone()).collect()
            } else {
                (0..capture_types.len()).map(|i| format!("c{}", i)).collect()
            };
            for cname in &capture_names {
                call_args.push(format!("env->{}", cname));
            }
            // Trampoline param names: y0, y1, ...
            let mut tramp_params: Vec<String> = vec!["uint64_t env_addr".to_string()];
            let mut tramp_call_extra: Vec<String> = Vec::new();
            for (i, a) in args.iter().enumerate() {
                tramp_params.push(format!("{} y{}", c_leaf_type(a), i));
                tramp_call_extra.push(format!("y{}", i));
            }
            call_args.extend(tramp_call_extra);
            let env_c = struct_c_name(env_struct_name);
            let ret_c = c_leaf_type(ret);
            body.push_str(&format!(
                "static {ret_c} {tn}({tp}) {{\n\
                 \x20 {env_c}* env = ({env_c}*)(uintptr_t)env_addr;\n\
                 \x20 return fn_{hn}({args});\n\
                 }}\n",
                ret_c = ret_c,
                tn = trampoline_name,
                tp = tramp_params.join(", "),
                env_c = env_c,
                hn = hoist_name,
                args = call_args.join(", "),
            ));
            // Magic-call constructor (used for the Let RHS).
            // Stack-allocates the env-struct, returns the closure
            // pair by value. Marked `static inline` so the C
            // compiler can fold it into the caller's frame
            // (closure env stays alive for the enclosing scope).
            let ctor_name = format!("fn_{}", magic_name);
            let mut ctor_params: Vec<String> = Vec::new();
            let mut ctor_field_inits: Vec<String> = Vec::new();
            for (i, (cname, cty)) in capture_names.iter().zip(capture_types.iter()).enumerate() {
                let _ = i;
                ctor_params.push(format!("{} p_{}", c_leaf_type(cty), cname));
                ctor_field_inits.push(format!(".{} = p_{}", cname, cname));
            }
            let params_str = if ctor_params.is_empty() {
                "void".to_string()
            } else {
                ctor_params.join(", ")
            };
            body.push_str(&format!(
                "static inline {sname} {ctor}({params}) {{\n\
                 \x20 static __thread {env_c} __env_slot;\n\
                 \x20 __env_slot = ({env_c}){{ {inits} }};\n\
                 \x20 {sname} __c;\n\
                 \x20 __c.env = (uint64_t)(uintptr_t)&__env_slot;\n\
                 \x20 __c.call = &{tn};\n\
                 \x20 return __c;\n\
                 }}\n\n",
                sname = sname,
                ctor = ctor_name,
                params = params_str,
                env_c = env_c,
                inits = ctor_field_inits.join(", "),
                tn = trampoline_name,
            ));
        }
    }
    // Emit a per-name C struct typedef for each payloaded
    // enum. Layout: `typedef struct { int32_t tag; T payload;
    // } Enum_<Name>;` where T is the shared payload type for
    // all payload-bearing variants. Plain enums stay as
    // bare `int32_t` tags (no typedef needed). T1.3 phase 2b.
    let mut any_enum_emitted = false;
    for decl in &program.enums {
        if !enum_has_payload(decl) {
            continue;
        }
        // Phase 3e fix: payloaded enum typedefs that were
        // pre-emitted (before struct typedefs) so struct fields
        // can reference them must be skipped here to avoid a
        // duplicate-typedef compile error.
        if enum_typedefs_pre_emitted.contains(&decl.name) {
            continue;
        }
        // Closure #283: mixed-payload-type enums lay out
        // each variant's payload through its own union
        // member, keyed by variant name (`u.v_<variant>`).
        // Single-payload-type enums keep the legacy
        // `{ tag; T payload; }` layout for back-compat.
        if enum_has_mixed_payloads(decl) {
            body.push_str(&format!(
                "typedef struct {{ int32_t tag; union {{\n",
            ));
            for (variant, pty) in decl.variants.iter().zip(decl.payload_types.iter()) {
                let Some(payload_ty) = pty.as_ref() else {
                    continue;
                };
                let member = enum_variant_member(variant);
                // Closure #283: `c_type_name` resolves
                // payload-less enums (Type::Enum without
                // `Enum_` typedef) to their `int32_t` tag
                // form, avoiding "Enum_AllocError
                // undeclared" errors when a mixed-payload
                // variant's payload is itself a payload-less
                // enum. `c_element_storage` is even safer for
                // struct payloads since it routes nested
                // Vec / nested-struct types correctly.
                let payload_decl = match payload_ty {
                    Type::Array { .. } => format_declarator(payload_ty, &member),
                    _ => format!("{} {}", c_element_storage(payload_ty), member),
                };
                body.push_str(&format!("    {};\n", payload_decl));
            }
            body.push_str(&format!("}} u; }} {};\n", enum_c_name(&decl.name)));
            any_enum_emitted = true;
            continue;
        }
        let payload_ty = match enum_common_payload_ty(decl) {
            Some(ty) => ty,
            None => continue,
        };
        // Array payloads need the `T name[N]` declarator
        // form rather than `intent_arr<N>_<T> name` (which
        // would require the typedef and complicate the
        // initializer story). Mirrors the struct-field array
        // handling from closure #100. Closure #119.
        let payload_decl = match &payload_ty {
            Type::Array { .. } => format_declarator(&payload_ty, "payload"),
            _ => format!("{} payload", c_type_name(&payload_ty)),
        };
        body.push_str(&format!(
            "typedef struct {{ int32_t tag; {}; }} {};\n",
            payload_decl,
            enum_c_name(&decl.name)
        ));
        any_enum_emitted = true;
    }
    if any_enum_emitted {
        body.push('\n');
    }
    // Emit tuple typedefs BEFORE vec / array typedefs so a
    // `Vec<(i64, i64)>` element can reference the tuple
    // struct. Inner-first dedup keeps nested tuples (when
    // we lift the Copy-only restriction later) ordered
    // correctly. T1.1.
    for shape in &tuple_shapes {
        emit_tuple_bundle(shape, &mut body);
    }
    if !tuple_shapes.is_empty() {
        body.push('\n');
    }
    // Per-shape array typedefs for any `Array<T, N>` that
    // appears as a Vec element (a `Vec<[i64; 4]>` needs
    // `typedef int64_t intent_arr4_int64_t[4];` in scope
    // before its helper bundle). Refines #7 phase 2c. Walks
    // only the Vec-element axis since arrays NOT inside Vecs
    // stay inlined in their declarators.
    let mut array_typedefs_seen = BTreeSet::<String>::new();
    for element in &element_types {
        emit_array_typedefs_for(element, &mut array_typedefs_seen, &mut body);
    }
    if !array_typedefs_seen.is_empty() {
        body.push('\n');
    }

    // Closure #239: array-return struct wrappers. C can't
    // return values of bare array type (arrays decay to
    // pointers in return position), so we wrap each array
    // return type in a per-shape `typedef struct { T data[N]; }
    // intent_arr_ret_<N>_<T>;` and emit at fn boundaries:
    // - Prototype/definition use the struct as the return
    //   type spelling (instead of the placeholder
    //   `/* array */`).
    // - Return statement wraps the array value in the struct
    //   compound literal.
    // - Let from an array-returning call unwraps via
    //   `__tmp.data` + memcpy to populate the local array.
    let mut array_return_seen = BTreeSet::<String>::new();
    for function in &program.functions {
        if let Type::Array { element, length } = &function.return_type {
            let name = array_return_struct_name(element, *length);
            if array_return_seen.insert(name.clone()) {
                body.push_str(&format!(
                    "typedef struct {{ {} data[{}]; }} {};\n",
                    c_element_storage(element),
                    length,
                    name,
                ));
            }
        }
    }
    if !array_return_seen.is_empty() {
        body.push('\n');
    }
    for element in &element_types {
        // Skip Vec bundles already emitted in the pre-struct
        // pass for fields like `struct Bag { contents: Vec<i64> }`.
        // T1.2 phase 2b.
        if emitted_vec_bundles.contains(&element_tag(element)) {
            continue;
        }
        emit_vec_bundle(element, &mut body);
    }

    // Closure #350: emit `intent_str_split` after the Vec
    // bundles so its `intent_vec_owned_str` return type is in
    // scope. Gated on actual program usage to avoid an unused
    // helper when no caller exists.
    if program_uses_str_split(program) {
        emit_intent_str_split_c(&mut body);
        emit_intent_str_join_c(&mut body);
        emit_intent_str_lines_c(&mut body);
    }
    // Closure #381: str_pad_left / str_pad_right helpers.
    // Always-on (small inline-able functions; tagged INTENT_UNUSED).
    emit_intent_str_pad_c(&mut body);
    // Closure #390: str_reverse helper. Always-on (no Vec
    // dependency).
    emit_intent_str_reverse_c(&mut body);
    // Closure #394 + #395: str_strip_prefix / str_strip_suffix /
    // str_count_char. All always-on (no Vec / Option deps).
    emit_intent_str_strip_c(&mut body);
    emit_intent_str_count_char_c(&mut body);
    // Closure #390: str_chars helper returns Vec<i64>, so gate
    // on the graph_vec_builtin walker that's already tracking
    // Vec<i64> demand — that walker fires emission of
    // `intent_vec_int64_t` typedef + helpers, so str_chars's
    // body can reference them safely.
    if program_uses_graph_vec_builtin(program) {
        emit_intent_str_chars_c(&mut body);
    }

    // Closure #356: Vec<i64> utility helpers (vec_range /
    // vec_repeat / vec_extend / vec_concat). All reference
    // `intent_vec_int64_t`, so emit after the Vec bundle pass.
    // Same body-substring gate as other Vec<i64> deps.
    if program_uses_graph_vec_builtin(program) {
        emit_intent_vec_int64_utility_helpers_c(&mut body);
    }
    // Closure #593: vec_chunks builds Vec<Vec<i64>>. Gated separately
    // because the helper references intent_vec_vec_int64_t which is
    // only emitted when the program uses Vec<Vec<i64>>.
    if program_uses_vec_chunks(program) {
        emit_intent_vec_chunks_helper_c(&mut body);
    }

    // Closure #357: Option<i64> ergonomics — unwrap_or /
    // is_some / is_none. Emit when Option__i64 is in the
    // payload registry (i.e. when any other Option<i64>-
    // returning builtin or user code has put it there).
    {
        let has_option_i64 = ENUM_PAYLOAD_REGISTRY.with(|r| {
            r.borrow().contains_key("Option__i64")
        });
        if has_option_i64 {
            emit_intent_option_i64_helpers_c(&mut body);
        }
    }
    // Closure #360: Option<f64> ergonomics — same triad as #357
    // but on the Enum_Option__f64 struct (already plumbed for
    // parse_float). Gated on Option__f64 being in the payload
    // registry.
    {
        let has_option_f64 = ENUM_PAYLOAD_REGISTRY.with(|r| {
            r.borrow().contains_key("Option__f64")
        });
        if has_option_f64 {
            emit_intent_option_f64_helpers_c(&mut body);
        }
    }

    for intent in &program.intents {
        body.push_str("/* intent: ");
        body.push_str(&escape_comment(intent));
        body.push_str(" */\n");
    }
    if !program.intents.is_empty() {
        body.push('\n');
    }

    let used_dyn_ifaces = collect_used_dyn_ifaces(program);
    // Phase 4: the full vtable struct body needs `Struct_<T>`
    // visible, so emit it AFTER struct typedefs (line below).
    // The body is appended to a separate buffer that gets
    // spliced in after structs are declared.
    emit_dyn_iface_vtable_bodies(&mut body, &used_dyn_ifaces);

    // Data-structures roadmap: emit array sort/find/etc
    // helpers AFTER all type declarations (enums, structs,
    // vec bundles) but BEFORE function prototypes, so the
    // helpers' uses of `Enum_Option__i64` resolve. Gated on
    // whether the program actually uses `[i64; N]` anywhere
    // — checked via a quick walk over fn signatures / bodies.
    if program_uses_i64_array(program) {
        let has_option_i64 = ENUM_PAYLOAD_REGISTRY.with(|r| {
            r.borrow().contains_key("Option__i64")
        });
        emit_intent_array_helpers_i64_unconditional(&mut body, has_option_i64);
    }
    // Deque<i64> helpers: emitted in body so `Enum_Option__i64`
    // typedef (added by the enum decl pass above) is visible
    // when the pop / peek helpers are defined.
    if program_uses_i64_deque(program) {
        let has_option_i64 = ENUM_PAYLOAD_REGISTRY.with(|r| {
            r.borrow().contains_key("Option__i64")
        });
        emit_intent_deque_helpers_c_body(&mut body, has_option_i64);
    }
    // HashSet<i64> helpers: same body-position rationale as
    // deque (no Option in v1, so no enum-typedef dependency,
    // but keep alongside deque for consistency).
    if program_uses_i64_hashset(program) {
        emit_intent_hashset_helpers_c_body(&mut body);
    }
    // Layer 2 of `unsafe.md` — Pool<i64> / Handle<i64> helpers.
    // The bundle's `pool_get` returns `Enum_Option__i64`; the
    // `Option__i64` monomorph is auto-registered by the
    // pool_get-uses-Option pre-pass in checker.rs, so the
    // typedef is in scope by the time this bundle is emitted.
    if program_uses_i64_pool(program) {
        emit_intent_pool_helpers_c_body(&mut body);
    }
    // Layer 3.1 of `unsafe.md` — canary-protected heap
    // allocator. Gated on program usage. Always-on canaries
    // (no debug/release distinction yet); cost is ~24 bytes
    // per allocation + ~4 cycles at free time, well within
    // embedded budgets.
    if program_uses_unsafe_alloc(program) {
        emit_intent_unsafe_alloc_helpers_c_body(&mut body);
    }
    // Layer 3.2 of `unsafe.md` — BoundedPtr<i64> fat pointer.
    // The `bptr_get` builtin returns `Enum_Option__i64`; the
    // auto-register pre-pass in checker.rs ensures the
    // Option__i64 typedef is materialized before this bundle.
    if program_uses_bptr(program) {
        emit_intent_bptr_helpers_c_body(&mut body);
    }
    // Layer 5 v2 foundation of `unsafe.md` — Region bump-
    // allocator arena.
    if program_uses_region(program) {
        emit_intent_region_helpers_c_body(&mut body);
    }
    if program_uses_i64_i64_hashmap(program) {
        let has_option_i64 = ENUM_PAYLOAD_REGISTRY.with(|r| {
            r.borrow().contains_key("Option__i64")
        });
        emit_intent_hashmap_helpers_c_body(&mut body, has_option_i64);
    }
    // ARC 1.4d: walk every HashMap<K, V> pair the collector
    // found and emit a per-pair bundle for each non-(i64, i64)
    // shape. The legacy (i64, i64) pair above stays — both
    // emitters produce equivalent code for that case; the
    // collector-driven path skips it to avoid duplicate
    // definitions in the same translation unit.
    {
        let pairs = crate::hashmap_bundle::collect_hashmap_pairs(program);
        for p in &pairs {
            // Skip the legacy pair — already emitted above.
            if matches!(p.key, Type::I64) && matches!(p.value, Type::I64) {
                continue;
            }
            let v_tag = match &p.value {
                Type::I8 => "int8_t",
                Type::I16 => "int16_t",
                Type::I32 => "int32_t",
                Type::I64 => "int64_t",
                Type::U8 => "uint8_t",
                Type::U16 => "uint16_t",
                Type::U32 => "uint32_t",
                Type::U64 => "uint64_t",
                Type::Bool => "bool",
                // ARC 4.2: OwnedStr V — bundle stores `char*` per
                // slot, drop walks free each value.
                Type::OwnedStr => "owned_str",
                _ => continue,
            };
            let v_mangle = match &p.value {
                Type::I8 => "i8",
                Type::I16 => "i16",
                Type::I32 => "i32",
                Type::I64 => "i64",
                Type::U8 => "u8",
                Type::U16 => "u16",
                Type::U32 => "u32",
                Type::U64 => "u64",
                Type::Bool => "bool",
                // ARC 4.2: Option<OwnedStr> uses `OwnedStr` mangle
                // (matches the existing enum-mono naming).
                Type::OwnedStr => "OwnedStr",
                _ => continue,
            };
            let opt_name = format!("Option__{}", v_mangle);
            let has_option_v = ENUM_PAYLOAD_REGISTRY.with(|r| {
                r.borrow().contains_key(&opt_name)
            });
            // ARC 1.7: dispatch on K — scalar (i64) uses the
            // scalar-K bundle; struct K uses the struct-K
            // bundle (delegates hash + eq to user fns).
            match (&p.key, &p.value) {
                // ARC 4.2: K=i64, V=OwnedStr — V drop walks on
                // drop/clear/insert; _insert clones V internally;
                // _insert/_remove transfer prior V ownership back
                // to the caller via Option<OwnedStr>; _get clones.
                (Type::I64, Type::OwnedStr) => {
                    emit_intent_hashmap_pair_c_body_i64k_strv(
                        &mut body, v_mangle, has_option_v,
                    );
                }
                (Type::I64, _) => {
                    emit_intent_hashmap_pair_c_body(
                        &mut body, v_tag, v_tag, v_mangle, has_option_v,
                    );
                }
                // ARC 4.5: f64 K — built-in `==` equality (NaN
                // caveat documented in unsafe.md) + reinterpret-
                // bits FNV-1a hashing.
                (Type::F64, _) => {
                    emit_intent_hashmap_pair_c_body_f64k(
                        &mut body, v_tag, v_tag, v_mangle, has_option_v,
                    );
                }
                // ARC 4.3: OwnedStr K + OwnedStr V — both axes
                // heap-owned by the map; drop walks free both
                // K and V per slot.
                (Type::OwnedStr, Type::OwnedStr) => {
                    emit_intent_hashmap_pair_c_body_strk_strv(
                        &mut body, v_mangle, has_option_v,
                    );
                }
                // ARC 4.1: OwnedStr K — strcmp equality, FNV-1a
                // byte hash. Map owns each key pointer; drop /
                // clear walk all occupied slots and free them.
                (Type::OwnedStr, _) => {
                    emit_intent_hashmap_pair_c_body_strk(
                        &mut body, v_tag, v_tag, v_mangle, has_option_v,
                    );
                }
                // ARC 4.4: Tuple<i64, …, i64> K — hash_combine of
                // per-element FNV-1a hashes; pairwise field eq.
                // Elements are Copy so no drop walk.
                (Type::Tuple(els), _) if els.iter().all(|t| matches!(t, Type::I64)) => {
                    emit_intent_hashmap_pair_c_body_tuple_i64k(
                        &mut body, els.len(), v_tag, v_tag, v_mangle, has_option_v,
                    );
                }
                // ARC 4.6: Vec<i64> K — length-prefixed FNV-1a
                // hash + len-then-memcmp equality. Map deep-
                // clones each Vec's data array on insert; drop
                // walks free each stored data buffer.
                (Type::Vec(inner), _) if matches!(inner.as_ref(), Type::I64) => {
                    emit_intent_hashmap_pair_c_body_vec_i64k(
                        &mut body, v_tag, v_tag, v_mangle, has_option_v,
                    );
                }
                (Type::Struct(k_name), _) => {
                    let k_ctype = format!("Struct_{}", k_name);
                    emit_intent_hashmap_struct_pair_c_body(
                        &mut body, k_name, &k_ctype, v_tag, v_tag,
                        v_mangle, has_option_v,
                    );
                }
                _ => continue,
            };
        }
    }
    if program_uses_i64_btreeset(program) {
        let has_option_i64 = ENUM_PAYLOAD_REGISTRY.with(|r| {
            r.borrow().contains_key("Option__i64")
        });
        let emit_vec_dep = program_uses_graph_vec_builtin(program);
        emit_intent_btreeset_helpers_c_body(&mut body, has_option_i64, emit_vec_dep);
    }
    if program_uses_i64_i64_btreemap(program) {
        let has_option_i64 = ENUM_PAYLOAD_REGISTRY.with(|r| {
            r.borrow().contains_key("Option__i64")
        });
        let emit_vec_dep = program_uses_graph_vec_builtin(program);
        emit_intent_btreemap_helpers_c_body(&mut body, has_option_i64, emit_vec_dep);
    }
    if program_uses_union_find(program) {
        emit_intent_union_find_helpers_c_body(&mut body);
    }
    if program_uses_i64_binary_heap(program) {
        let has_option_i64 = ENUM_PAYLOAD_REGISTRY.with(|r| {
            r.borrow().contains_key("Option__i64")
        });
        emit_intent_binary_heap_helpers_c_body(&mut body, has_option_i64);
    }
    if program_uses_bloom_filter(program) {
        emit_intent_bloom_filter_helpers_c_body(&mut body);
    }
    if program_uses_i64_bst(program) {
        let has_option_i64 = ENUM_PAYLOAD_REGISTRY.with(|r| {
            r.borrow().contains_key("Option__i64")
        });
        emit_intent_bst_i64_helpers_c_body(&mut body, has_option_i64);
    }
    if program_uses_graph(program) {
        let has_option_i64 = ENUM_PAYLOAD_REGISTRY.with(|r| {
            r.borrow().contains_key("Option__i64")
        });
        let emit_vec_dep = program_uses_graph_vec_builtin(program);
        emit_intent_graph_helpers_c_body(&mut body, has_option_i64, emit_vec_dep);
    }
    if program_uses_trie(program) {
        emit_intent_trie_helpers_c_body(&mut body);
    }
    if program_uses_skiplist(program) {
        let has_option_i64 = ENUM_PAYLOAD_REGISTRY.with(|r| {
            r.borrow().contains_key("Option__i64")
        });
        emit_intent_skiplist_helpers_c_body(&mut body, has_option_i64);
    }

    for function in &program.functions {
        emit_prototype(function, &mut body);
    }
    body.push('\n');

    emit_dyn_iface_vtables(&mut body, &used_dyn_ifaces);

    // Emit function bodies into a separate buffer so the
    // task-outlining side-effect (TASK_OUTLINES) can be
    // spliced between the prototypes and the bodies. Task
    // outlines call user functions, so they need to see the
    // prototypes but be defined before the function bodies
    // that reference the outline names.
    let mut function_bodies = String::new();
    for function in &program.functions {
        emit_function(function, &mut function_bodies);
        function_bodies.push('\n');
    }
    // Splice outlines between prototypes and function bodies.
    TASK_OUTLINES.with(|b| {
        let outlines = std::mem::take(&mut *b.borrow_mut());
        body.push_str(&outlines);
    });
    body.push_str(&function_bodies);

    body.push_str("int main(void) {\n");
    body.push_str("  return (int)fn_main();\n");
    body.push_str("}\n");

    let mut out = String::new();
    out.push_str("#include <assert.h>\n");
    out.push_str("#include <stdatomic.h>\n");
    out.push_str("#include <stdbool.h>\n");
    out.push_str("#include <stdint.h>\n");
    out.push_str("#include <stdio.h>\n");
    out.push_str("#include <stdlib.h>\n");
    out.push_str("#include <string.h>\n");
    out.push_str("#include <math.h>\n");
    // INTENT_UNUSED is referenced by every Vec helper and
    // by the threading wrappers below, so define it
    // unconditionally even if no runtime guard helpers
    // survived the SMT-elision pass.
    out.push_str("#if defined(__GNUC__) || defined(__clang__)\n");
    out.push_str("#define INTENT_UNUSED __attribute__((unused))\n");
    out.push_str("#else\n");
    out.push_str("#define INTENT_UNUSED\n");
    out.push_str("#endif\n\n");
    emit_intent_print_int_dev_c(&mut out);
    emit_intent_print_int_ben_c(&mut out);
    emit_intent_print_int_tam_c(&mut out);
    emit_intent_print_int_tel_c(&mut out);
    emit_intent_print_int_guj_c(&mut out);
    emit_intent_print_int_pan_c(&mut out);
    emit_intent_print_int_kan_c(&mut out);
    emit_intent_print_int_mal_c(&mut out);
    emit_intent_print_int_odi_c(&mut out);
    emit_intent_print_int_sin_c(&mut out);
    emit_intent_print_int_urd_c(&mut out);
    emit_intent_print_int_per_c(&mut out);
    emit_intent_thread_wrappers_c(&mut out);
    emit_runtime_helpers(&mut out, &body);
    emit_intent_str_concat_c(&mut out);
    emit_intent_str_trim_c(&mut out);
    emit_intent_str_replace_c(&mut out);
    emit_intent_substring_c(&mut out);
    emit_intent_str_repeat_c(&mut out);
    emit_intent_str_case_c(&mut out);
    emit_intent_i64_to_str_c(&mut out);
    emit_concurrency_runtime_helpers(&mut out, &body, &channel_specs);
    emit_intent_rng_helpers_c(&mut out, &body);
    emit_intent_hash_helpers_c(&mut out, &body);
    emit_intent_sleep_ms_helper_c(&mut out, &body);
    // Force TCP helpers to emit when epoll helpers do so the
    // `accept()` / `recv()` declares + the thread-local buffer
    // are available to the nb variants AND `read()` lands for
    // sleep_ms_finish. Same fall-through as the LLVM side.
    let need_epoll_helpers = body.contains("intent_epoll_")
        || body.contains("intent_tcp_set_nonblocking")
        || body.contains("intent_tcp_accept_nb")
        || body.contains("intent_tcp_recv_nb")
        || body.contains("intent_sleep_ms_async")
        || body.contains("intent_sleep_ms_finish");
    let uses_arc8_io = need_epoll_helpers
        || body.contains("intent_tcp_")
        || body.contains("intent_sleep_ms(");
    // Arc 8 v3.1 Phase 5 — compile-time gate. The Arc 8 I/O
    // runtime (sleep_ms / TCP / epoll / nb variants / timer)
    // is now supported on Linux + macOS via dual-target C
    // (the emitted helpers branch at C-compile time via
    // `#ifdef __APPLE__` / `#elif defined(__linux__)`). Windows
    // (Phase 6) routes through a winsock2 / IOCP `#ifdef _WIN32`
    // branch. Hosts outside those three platforms fail loud
    // during codegen instead of breaking silently at C-compile
    // time with `<sys/socket.h>: No such file or directory`.
    if uses_arc8_io && !crate::backend_llvm::host_supports_arc8_io() {
        panic!(
            "Arc 8 I/O runtime (sleep_ms, TCP family, epoll + nb \
             variants, sleep_ms_async) is supported on Linux, \
             macOS, and Windows. The current host target is not \
             one of those three; see ARC8_V3_PLAN.md Phase 5 \
             (macOS kqueue) and Phase 6 (Windows IOCP) for the \
             porting model. Add a `host_is_*` helper in \
             backend_llvm.rs if you're bringing up a new platform."
        );
    }
    if need_epoll_helpers && !body.contains("intent_tcp_") {
        // Synthesize a `intent_tcp_` reference to flip the
        // gate in emit_intent_tcp_helpers_c. Cheap hack vs
        // restructuring the gate predicates.
        let body_with_force = format!("/*intent_tcp_force*/\n{}", body);
        emit_intent_tcp_helpers_c(&mut out, &body_with_force);
    } else {
        emit_intent_tcp_helpers_c(&mut out, &body);
    }
    emit_intent_epoll_helpers_c(&mut out, &body);
    out.push_str(&body);
    out
}

/// Arc 8 v2 + Phase 5 — epoll/kqueue + non-blocking I/O runtime
/// helpers. Linux uses `<sys/epoll.h>` + `<sys/timerfd.h>`.
/// macOS uses `<sys/event.h>` (kqueue + EVFILT_READ +
/// EVFILT_TIMER) with a userspace pipe2+pthread timer shim
/// because kqueue's EVFILT_TIMER isn't itself an fd that can be
/// polled via the epoll-shaped API. The C compiler picks the
/// right branch at C-compile time via `__APPLE__` /
/// `__linux__` / `_WIN32` macros, so a single emit handles all
/// three platforms.
///
/// VERIFICATION DEFERRED for macOS and Windows branches — no
/// host access in the current dev environment. The Linux branch
/// is byte-identical to the pre-Phase-5 emitter so existing
/// Linux verification stays green.
fn emit_intent_epoll_helpers_c(out: &mut String, body: &str) {
    if !body.contains("intent_epoll_") && !body.contains("intent_tcp_set_nonblocking")
        && !body.contains("intent_tcp_accept_nb") && !body.contains("intent_tcp_recv_nb")
        && !body.contains("intent_sleep_ms_async") && !body.contains("intent_sleep_ms_finish")
    {
        return;
    }
    out.push_str(
        "#if defined(_WIN32)\n\
         #include <winsock2.h>\n\
         #include <ws2tcpip.h>\n\
         #include <windows.h>\n\
         /* winsock2.h defines SOCKET as an unsigned integer\n\
          * handle; cast through SOCKET when bridging int64_t. */\n\
         #elif defined(__APPLE__)\n\
         #include <sys/event.h>\n\
         #include <sys/time.h>\n\
         #include <fcntl.h>\n\
         #include <unistd.h>\n\
         #include <pthread.h>\n\
         #include <stdlib.h>\n\
         #else\n\
         #include <sys/epoll.h>\n\
         #include <sys/timerfd.h>\n\
         #include <fcntl.h>\n\
         #include <unistd.h>\n\
         #endif\n\
         #if defined(_WIN32)\n\
         /* Phase 6 (Windows IOCP) — see ARC8_V3_PLAN.md.\n\
          * VERIFICATION DEFERRED: no Windows host access. */\n\
         static INTENT_UNUSED int64_t intent_epoll_new(void) {\n\
         \x20 HANDLE h = CreateIoCompletionPort(INVALID_HANDLE_VALUE, NULL, 0, 0);\n\
         \x20 return (h == NULL) ? -1 : (int64_t)(intptr_t)h;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_epoll_add_read(int64_t epfd, int64_t fd) {\n\
         \x20 /* IOCP associates the SOCKET with the completion port;\n\
          \x20\x20\x20\x20 read readiness comes via posted overlapped recv.\n\
          \x20\x20\x20\x20 The vāṇी epoll API is event-driven rather than\n\
          \x20\x20\x20\x20 readiness-driven on Windows — Phase 6c work. */\n\
         \x20 HANDLE h = (HANDLE)(intptr_t)epfd;\n\
         \x20 HANDLE r = CreateIoCompletionPort((HANDLE)(SOCKET)fd, h, (ULONG_PTR)fd, 0);\n\
         \x20 return (r == h) ? 0 : -1;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_epoll_wait_one(int64_t epfd, int64_t timeout_ms) {\n\
         \x20 HANDLE h = (HANDLE)(intptr_t)epfd;\n\
         \x20 DWORD bytes = 0; ULONG_PTR key = 0; LPOVERLAPPED ov = NULL;\n\
         \x20 DWORD tmo = (timeout_ms < 0) ? INFINITE : (DWORD)timeout_ms;\n\
         \x20 BOOL ok = GetQueuedCompletionStatus(h, &bytes, &key, &ov, tmo);\n\
         \x20 if (!ok && ov == NULL) {\n\
         \x20   /* WAIT_TIMEOUT vs error; use GetLastError. */\n\
         \x20   return (GetLastError() == WAIT_TIMEOUT) ? -2 : -1;\n\
         \x20 }\n\
         \x20 return (int64_t)key;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_epoll_close(int64_t epfd) {\n\
         \x20 HANDLE h = (HANDLE)(intptr_t)epfd;\n\
         \x20 return CloseHandle(h) ? 0 : -1;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_tcp_set_nonblocking(int64_t fd) {\n\
         \x20 u_long nb = 1;\n\
         \x20 return (ioctlsocket((SOCKET)fd, FIONBIO, &nb) == 0) ? 0 : -1;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_tcp_accept_nb(int64_t server_fd) {\n\
         \x20 SOCKET cfd = accept((SOCKET)server_fd, NULL, NULL);\n\
         \x20 if (cfd != INVALID_SOCKET) return (int64_t)cfd;\n\
         \x20 return (WSAGetLastError() == WSAEWOULDBLOCK) ? -2 : -1;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_tcp_recv_nb(int64_t fd, int64_t max) {\n\
         \x20 if (max < 0) return -1;\n\
         \x20 size_t want = (size_t)max;\n\
         \x20 if (want > sizeof(intent_tcp_buf)) want = sizeof(intent_tcp_buf);\n\
         \x20 int n = recv((SOCKET)fd, (char*)intent_tcp_buf, (int)want, 0);\n\
         \x20 if (n >= 0) return (int64_t)n;\n\
         \x20 return (WSAGetLastError() == WSAEWOULDBLOCK) ? -2 : -1;\n\
         }\n\
         /* Windows timer: CreateWaitableTimer + thread that posts a\n\
          * completion packet on the user's IOCP when the timer fires.\n\
          * The returned \"fd\" is a unique sentinel key the user passes\n\
          * to intent_sleep_ms_finish (which is a no-op cleanup since\n\
          * the timer thread frees its own handle). */\n\
         struct __intent_timer_win { HANDLE timer; HANDLE iocp; ULONG_PTR key; int64_t ms; };\n\
         static DWORD WINAPI __intent_timer_win_thread(LPVOID arg) {\n\
         \x20 struct __intent_timer_win* a = (struct __intent_timer_win*)arg;\n\
         \x20 Sleep((DWORD)(a->ms < 0 ? 0 : a->ms));\n\
         \x20 if (a->iocp) PostQueuedCompletionStatus(a->iocp, 0, a->key, NULL);\n\
         \x20 free(a);\n\
         \x20 return 0;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_sleep_ms_async(int64_t ms) {\n\
         \x20 /* Without knowing which IOCP the caller will use, fire\n\
          \x20\x20\x20\x20 a self-contained Sleep thread that returns a key\n\
          \x20\x20\x20\x20 the user's epoll_add_read step will associate.\n\
          \x20\x20\x20\x20 Phase 6c follow-up: thread the IOCP handle in. */\n\
         \x20 struct __intent_timer_win* a = (struct __intent_timer_win*)malloc(sizeof(*a));\n\
         \x20 if (!a) return -1;\n\
         \x20 a->timer = NULL; a->iocp = NULL; a->key = (ULONG_PTR)(uintptr_t)a; a->ms = ms;\n\
         \x20 HANDLE th = CreateThread(NULL, 0, __intent_timer_win_thread, a, 0, NULL);\n\
         \x20 if (!th) { free(a); return -1; }\n\
         \x20 CloseHandle(th);\n\
         \x20 return (int64_t)a->key;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_sleep_ms_finish(int64_t fd) {\n\
         \x20 (void)fd;\n\
         \x20 /* Thread freed its own state; nothing to clean up. */\n\
         \x20 return 1;\n\
         }\n\
         #elif defined(__APPLE__)\n\
         /* Phase 5 (macOS kqueue) — see ARC8_V3_PLAN.md.\n\
          * VERIFICATION DEFERRED: no macOS host access. */\n\
         static INTENT_UNUSED int64_t intent_epoll_new(void) {\n\
         \x20 int fd = kqueue();\n\
         \x20 return (fd < 0) ? -1 : (int64_t)fd;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_epoll_add_read(int64_t epfd, int64_t fd) {\n\
         \x20 struct kevent kev;\n\
         \x20 EV_SET(&kev, (int)fd, EVFILT_READ, EV_ADD | EV_CLEAR, 0, 0, NULL);\n\
         \x20 int rc;\n\
         \x20 do { rc = kevent((int)epfd, &kev, 1, NULL, 0, NULL); }\n\
         \x20 while (rc < 0 && errno == EINTR);\n\
         \x20 return (rc == 0) ? 0 : -1;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_epoll_wait_one(int64_t epfd, int64_t timeout_ms) {\n\
         \x20 struct kevent kev;\n\
         \x20 struct timespec ts;\n\
         \x20 struct timespec* tsp;\n\
         \x20 int rc;\n\
         \x20 if (timeout_ms < 0) {\n\
         \x20   tsp = NULL;\n\
         \x20 } else {\n\
         \x20   ts.tv_sec = (time_t)(timeout_ms / 1000);\n\
         \x20   ts.tv_nsec = (long)((timeout_ms % 1000) * 1000000L);\n\
         \x20   tsp = &ts;\n\
         \x20 }\n\
         \x20 do { rc = kevent((int)epfd, NULL, 0, &kev, 1, tsp); }\n\
         \x20 while (rc < 0 && errno == EINTR);\n\
         \x20 if (rc < 0) return -1;\n\
         \x20 if (rc == 0) return -2;\n\
         \x20 return (int64_t)(int)kev.ident;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_epoll_close(int64_t epfd) {\n\
         \x20 return (close((int)epfd) == 0) ? 0 : -1;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_tcp_set_nonblocking(int64_t fd) {\n\
         \x20 int flags = fcntl((int)fd, F_GETFL, 0);\n\
         \x20 if (flags < 0) return -1;\n\
         \x20 return (fcntl((int)fd, F_SETFL, flags | O_NONBLOCK) == 0) ? 0 : -1;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_tcp_accept_nb(int64_t server_fd) {\n\
         \x20 int cfd = accept((int)server_fd, NULL, NULL);\n\
         \x20 if (cfd >= 0) return (int64_t)cfd;\n\
         \x20 if (errno == EAGAIN || errno == EWOULDBLOCK) return -2;\n\
         \x20 return -1;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_tcp_recv_nb(int64_t fd, int64_t max) {\n\
         \x20 if (max < 0) return -1;\n\
         \x20 size_t want = (size_t)max;\n\
         \x20 if (want > sizeof(intent_tcp_buf)) want = sizeof(intent_tcp_buf);\n\
         \x20 ssize_t n = recv((int)fd, intent_tcp_buf, want, 0);\n\
         \x20 if (n >= 0) return (int64_t)n;\n\
         \x20 if (errno == EAGAIN || errno == EWOULDBLOCK) return -2;\n\
         \x20 return -1;\n\
         }\n\
         /* macOS userspace timer-fd shim — pipe2 isn't on macOS so\n\
          * we use pipe() + fcntl(O_NONBLOCK), and a detached pthread\n\
          * that sleeps then writes one byte to wake epoll_wait_one. */\n\
         struct __intent_timer_args { int wfd; int64_t ms; };\n\
         static void* __intent_timer_thread(void* arg) {\n\
         \x20 struct __intent_timer_args* a = (struct __intent_timer_args*)arg;\n\
         \x20 struct timespec ts;\n\
         \x20 ts.tv_sec = (time_t)(a->ms / 1000);\n\
         \x20 ts.tv_nsec = (long)((a->ms % 1000) * 1000000L);\n\
         \x20 (void)nanosleep(&ts, NULL);\n\
         \x20 uint64_t one = 1;\n\
         \x20 (void)write(a->wfd, &one, sizeof(one));\n\
         \x20 close(a->wfd);\n\
         \x20 free(a);\n\
         \x20 return NULL;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_sleep_ms_async(int64_t ms) {\n\
         \x20 if (ms < 0) return -1;\n\
         \x20 int p[2]; if (pipe(p) < 0) return -1;\n\
         \x20 int flags = fcntl(p[0], F_GETFL, 0);\n\
         \x20 if (flags < 0) { close(p[0]); close(p[1]); return -1; }\n\
         \x20 if (fcntl(p[0], F_SETFL, flags | O_NONBLOCK) < 0) { close(p[0]); close(p[1]); return -1; }\n\
         \x20 struct __intent_timer_args* a = (struct __intent_timer_args*)malloc(sizeof(*a));\n\
         \x20 if (!a) { close(p[0]); close(p[1]); return -1; }\n\
         \x20 a->wfd = p[1]; a->ms = ms;\n\
         \x20 pthread_t th;\n\
         \x20 if (pthread_create(&th, NULL, __intent_timer_thread, a) != 0) {\n\
         \x20   free(a); close(p[0]); close(p[1]); return -1;\n\
         \x20 }\n\
         \x20 pthread_detach(th);\n\
         \x20 return (int64_t)p[0];\n\
         }\n\
         static INTENT_UNUSED int64_t intent_sleep_ms_finish(int64_t fd) {\n\
         \x20 uint64_t exp = 0;\n\
         \x20 ssize_t n = read((int)fd, &exp, sizeof(exp));\n\
         \x20 close((int)fd);\n\
         \x20 if (n < 0) return -1;\n\
         \x20 return (int64_t)(n > 0 ? 1 : 0);\n\
         }\n\
         #else\n\
         /* Linux epoll + timerfd — original v2 path. */\n\
         static INTENT_UNUSED int64_t intent_epoll_new(void) {\n\
         \x20 int fd = epoll_create1(0);\n\
         \x20 return (fd < 0) ? -1 : (int64_t)fd;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_epoll_add_read(int64_t epfd, int64_t fd) {\n\
         \x20 struct epoll_event ev;\n\
         \x20 ev.events = EPOLLIN;\n\
         \x20 ev.data.fd = (int)fd;\n\
         \x20 return (epoll_ctl((int)epfd, EPOLL_CTL_ADD, (int)fd, &ev) == 0) ? 0 : -1;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_epoll_wait_one(int64_t epfd, int64_t timeout_ms) {\n\
         \x20 struct epoll_event ev;\n\
         \x20 int rc;\n\
         \x20 do { rc = epoll_wait((int)epfd, &ev, 1, (int)timeout_ms); }\n\
         \x20 while (rc < 0 && errno == EINTR);\n\
         \x20 if (rc < 0) return -1;\n\
         \x20 if (rc == 0) return -2;\n\
         \x20 return (int64_t)ev.data.fd;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_epoll_close(int64_t epfd) {\n\
         \x20 return (close((int)epfd) == 0) ? 0 : -1;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_tcp_set_nonblocking(int64_t fd) {\n\
         \x20 int flags = fcntl((int)fd, F_GETFL, 0);\n\
         \x20 if (flags < 0) return -1;\n\
         \x20 return (fcntl((int)fd, F_SETFL, flags | O_NONBLOCK) == 0) ? 0 : -1;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_tcp_accept_nb(int64_t server_fd) {\n\
         \x20 int cfd = accept((int)server_fd, NULL, NULL);\n\
         \x20 if (cfd >= 0) return (int64_t)cfd;\n\
         \x20 if (errno == EAGAIN || errno == EWOULDBLOCK) return -2;\n\
         \x20 return -1;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_tcp_recv_nb(int64_t fd, int64_t max) {\n\
         \x20 if (max < 0) return -1;\n\
         \x20 size_t want = (size_t)max;\n\
         \x20 if (want > sizeof(intent_tcp_buf)) want = sizeof(intent_tcp_buf);\n\
         \x20 ssize_t n = recv((int)fd, intent_tcp_buf, want, 0);\n\
         \x20 if (n >= 0) return (int64_t)n;\n\
         \x20 if (errno == EAGAIN || errno == EWOULDBLOCK) return -2;\n\
         \x20 return -1;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_sleep_ms_async(int64_t ms) {\n\
         \x20 if (ms < 0) return -1;\n\
         \x20 int fd = timerfd_create(CLOCK_MONOTONIC, TFD_NONBLOCK);\n\
         \x20 if (fd < 0) return -1;\n\
         \x20 struct itimerspec val;\n\
         \x20 val.it_interval.tv_sec = 0;\n\
         \x20 val.it_interval.tv_nsec = 0;\n\
         \x20 val.it_value.tv_sec = (time_t)(ms / 1000);\n\
         \x20 val.it_value.tv_nsec = (long)((ms % 1000) * 1000000L);\n\
         \x20 if (timerfd_settime(fd, 0, &val, NULL) < 0) { close(fd); return -1; }\n\
         \x20 return (int64_t)fd;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_sleep_ms_finish(int64_t fd) {\n\
         \x20 uint64_t exp = 0;\n\
         \x20 ssize_t n = read((int)fd, &exp, sizeof(exp));\n\
         \x20 close((int)fd);\n\
         \x20 if (n < 0) return -1;\n\
         \x20 return (int64_t)exp;\n\
         }\n\
         #endif\n\n",
    );
}

/// Arc 8 step 8e proper — TCP runtime helpers. All eight
/// builtins (tcp_listen / tcp_socket_port / tcp_accept /
/// tcp_connect_local / tcp_send_str / tcp_recv /
/// tcp_send_buf / tcp_close) emit only when the program
/// references any tcp_* helper. Thread-local 4KB recv
/// buffer means concurrent `task` bodies have independent
/// scratch space.
fn emit_intent_tcp_helpers_c(out: &mut String, body: &str) {
    if !body.contains("intent_tcp_") {
        return;
    }
    out.push_str(
        "#if defined(_WIN32)\n\
         /* Phase 6 (Windows IOCP) — winsock2 brought in alongside\n\
          * the epoll shim above. VERIFICATION DEFERRED — no\n\
          * Windows host access. */\n\
         #include <string.h>\n\
         #pragma comment(lib, \"ws2_32.lib\")\n\
         static int __intent_winsock_inited = 0;\n\
         static void __intent_winsock_startup(void) {\n\
         \x20 if (__intent_winsock_inited) return;\n\
         \x20 WSADATA wsa; (void)WSAStartup(MAKEWORD(2, 2), &wsa);\n\
         \x20 __intent_winsock_inited = 1;\n\
         }\n\
         #else\n\
         #include <sys/socket.h>\n\
         #include <netinet/in.h>\n\
         #include <arpa/inet.h>\n\
         #include <unistd.h>\n\
         #include <string.h>\n\
         #include <errno.h>\n\
         #endif\n\
         #if defined(_MSC_VER)\n\
         /* MSVC: no _Thread_local — use the equivalent\n\
          * __declspec(thread) keyword. */\n\
         static __declspec(thread) unsigned char intent_tcp_buf[4096];\n\
         #else\n\
         static _Thread_local unsigned char intent_tcp_buf[4096];\n\
         #endif\n\
         #if defined(_WIN32)\n\
         /* Windows TCP — SOCKET is an unsigned handle, WSAGetLastError\n\
          * replaces errno for socket ops, and recv/send return int\n\
          * rather than ssize_t. */\n\
         static INTENT_UNUSED int64_t intent_tcp_listen(int64_t port) {\n\
         \x20 __intent_winsock_startup();\n\
         \x20 SOCKET s = socket(AF_INET, SOCK_STREAM, IPPROTO_TCP);\n\
         \x20 if (s == INVALID_SOCKET) return -1;\n\
         \x20 BOOL opt = 1;\n\
         \x20 (void)setsockopt(s, SOL_SOCKET, SO_REUSEADDR, (const char*)&opt, sizeof(opt));\n\
         \x20 struct sockaddr_in sa;\n\
         \x20 memset(&sa, 0, sizeof(sa));\n\
         \x20 sa.sin_family = AF_INET;\n\
         \x20 sa.sin_addr.s_addr = htonl(INADDR_LOOPBACK);\n\
         \x20 sa.sin_port = htons((uint16_t)port);\n\
         \x20 if (bind(s, (struct sockaddr*)&sa, sizeof(sa)) == SOCKET_ERROR) { closesocket(s); return -1; }\n\
         \x20 if (listen(s, 16) == SOCKET_ERROR) { closesocket(s); return -1; }\n\
         \x20 return (int64_t)s;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_tcp_socket_port(int64_t fd) {\n\
         \x20 struct sockaddr_in sa;\n\
         \x20 int slen = (int)sizeof(sa);\n\
         \x20 if (getsockname((SOCKET)fd, (struct sockaddr*)&sa, &slen) == SOCKET_ERROR) return -1;\n\
         \x20 return (int64_t)ntohs(sa.sin_port);\n\
         }\n\
         static INTENT_UNUSED int64_t intent_tcp_accept(int64_t server_fd) {\n\
         \x20 SOCKET cfd = accept((SOCKET)server_fd, NULL, NULL);\n\
         \x20 return (cfd == INVALID_SOCKET) ? -1 : (int64_t)cfd;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_tcp_connect_local(int64_t port) {\n\
         \x20 __intent_winsock_startup();\n\
         \x20 SOCKET s = socket(AF_INET, SOCK_STREAM, IPPROTO_TCP);\n\
         \x20 if (s == INVALID_SOCKET) return -1;\n\
         \x20 struct sockaddr_in sa;\n\
         \x20 memset(&sa, 0, sizeof(sa));\n\
         \x20 sa.sin_family = AF_INET;\n\
         \x20 sa.sin_addr.s_addr = htonl(INADDR_LOOPBACK);\n\
         \x20 sa.sin_port = htons((uint16_t)port);\n\
         \x20 if (connect(s, (struct sockaddr*)&sa, sizeof(sa)) == SOCKET_ERROR) { closesocket(s); return -1; }\n\
         \x20 return (int64_t)s;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_tcp_send_str(int64_t fd, const char* s) {\n\
         \x20 if (!s) return -1;\n\
         \x20 size_t len = strlen(s);\n\
         \x20 size_t off = 0;\n\
         \x20 while (off < len) {\n\
         \x20   int n = send((SOCKET)fd, s + off, (int)(len - off), 0);\n\
         \x20   if (n == SOCKET_ERROR) return -1;\n\
         \x20   if (n == 0) return -1;\n\
         \x20   off += (size_t)n;\n\
         \x20 }\n\
         \x20 return (int64_t)len;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_tcp_recv(int64_t fd, int64_t max) {\n\
         \x20 if (max < 0) return -1;\n\
         \x20 size_t want = (size_t)max;\n\
         \x20 if (want > sizeof(intent_tcp_buf)) want = sizeof(intent_tcp_buf);\n\
         \x20 int n = recv((SOCKET)fd, (char*)intent_tcp_buf, (int)want, 0);\n\
         \x20 return (n == SOCKET_ERROR) ? -1 : (int64_t)n;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_tcp_send_buf(int64_t fd, int64_t n) {\n\
         \x20 if (n < 0 || (size_t)n > sizeof(intent_tcp_buf)) return -1;\n\
         \x20 size_t off = 0;\n\
         \x20 while (off < (size_t)n) {\n\
         \x20   int m = send((SOCKET)fd, (const char*)intent_tcp_buf + off, (int)((size_t)n - off), 0);\n\
         \x20   if (m == SOCKET_ERROR) return -1;\n\
         \x20   if (m == 0) return -1;\n\
         \x20   off += (size_t)m;\n\
         \x20 }\n\
         \x20 return (int64_t)n;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_tcp_close(int64_t fd) {\n\
         \x20 return (closesocket((SOCKET)fd) == 0) ? 0 : -1;\n\
         }\n\
         #else\n\
         /* Linux + macOS: shared POSIX socket implementation. The\n\
          * call surface is identical on both — macOS gets the same\n\
          * code as Linux. */\n\
         static INTENT_UNUSED int64_t intent_tcp_listen(int64_t port) {\n\
         \x20 int fd = socket(AF_INET, SOCK_STREAM, 0);\n\
         \x20 if (fd < 0) return -1;\n\
         \x20 int opt = 1;\n\
         \x20 (void)setsockopt(fd, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt));\n\
         \x20 struct sockaddr_in sa;\n\
         \x20 memset(&sa, 0, sizeof(sa));\n\
         \x20 sa.sin_family = AF_INET;\n\
         \x20 sa.sin_addr.s_addr = htonl(INADDR_LOOPBACK);\n\
         \x20 sa.sin_port = htons((uint16_t)port);\n\
         \x20 if (bind(fd, (struct sockaddr*)&sa, sizeof(sa)) < 0) { close(fd); return -1; }\n\
         \x20 if (listen(fd, 16) < 0) { close(fd); return -1; }\n\
         \x20 return (int64_t)fd;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_tcp_socket_port(int64_t fd) {\n\
         \x20 struct sockaddr_in sa;\n\
         \x20 socklen_t slen = sizeof(sa);\n\
         \x20 if (getsockname((int)fd, (struct sockaddr*)&sa, &slen) < 0) return -1;\n\
         \x20 return (int64_t)ntohs(sa.sin_port);\n\
         }\n\
         static INTENT_UNUSED int64_t intent_tcp_accept(int64_t server_fd) {\n\
         \x20 int cfd;\n\
         \x20 do { cfd = accept((int)server_fd, NULL, NULL); }\n\
         \x20 while (cfd < 0 && errno == EINTR);\n\
         \x20 return (int64_t)cfd;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_tcp_connect_local(int64_t port) {\n\
         \x20 int fd = socket(AF_INET, SOCK_STREAM, 0);\n\
         \x20 if (fd < 0) return -1;\n\
         \x20 struct sockaddr_in sa;\n\
         \x20 memset(&sa, 0, sizeof(sa));\n\
         \x20 sa.sin_family = AF_INET;\n\
         \x20 sa.sin_addr.s_addr = htonl(INADDR_LOOPBACK);\n\
         \x20 sa.sin_port = htons((uint16_t)port);\n\
         \x20 int rc;\n\
         \x20 do { rc = connect(fd, (struct sockaddr*)&sa, sizeof(sa)); }\n\
         \x20 while (rc < 0 && errno == EINTR);\n\
         \x20 if (rc < 0) { close(fd); return -1; }\n\
         \x20 return (int64_t)fd;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_tcp_send_str(int64_t fd, const char* s) {\n\
         \x20 if (!s) return -1;\n\
         \x20 size_t len = strlen(s);\n\
         \x20 size_t off = 0;\n\
         \x20 while (off < len) {\n\
         \x20   ssize_t n = send((int)fd, s + off, len - off, 0);\n\
         \x20   if (n < 0) { if (errno == EINTR) continue; return -1; }\n\
         \x20   if (n == 0) return -1;\n\
         \x20   off += (size_t)n;\n\
         \x20 }\n\
         \x20 return (int64_t)len;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_tcp_recv(int64_t fd, int64_t max) {\n\
         \x20 if (max < 0) return -1;\n\
         \x20 size_t want = (size_t)max;\n\
         \x20 if (want > sizeof(intent_tcp_buf)) want = sizeof(intent_tcp_buf);\n\
         \x20 ssize_t n;\n\
         \x20 do { n = recv((int)fd, intent_tcp_buf, want, 0); }\n\
         \x20 while (n < 0 && errno == EINTR);\n\
         \x20 return (int64_t)n;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_tcp_send_buf(int64_t fd, int64_t n) {\n\
         \x20 if (n < 0 || (size_t)n > sizeof(intent_tcp_buf)) return -1;\n\
         \x20 size_t off = 0;\n\
         \x20 while (off < (size_t)n) {\n\
         \x20   ssize_t m = send((int)fd, intent_tcp_buf + off, (size_t)n - off, 0);\n\
         \x20   if (m < 0) { if (errno == EINTR) continue; return -1; }\n\
         \x20   if (m == 0) return -1;\n\
         \x20   off += (size_t)m;\n\
         \x20 }\n\
         \x20 return (int64_t)n;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_tcp_close(int64_t fd) {\n\
         \x20 return (close((int)fd) == 0) ? 0 : -1;\n\
         }\n\
         #endif\n\n",
    );
}

/// Arc 8 step 8e + Phase 5/6 — runtime helper for
/// `sleep_ms(ms)`. POSIX path (Linux + macOS) uses `nanosleep`.
/// Windows path uses `Sleep(ms)` from `<windows.h>` — already
/// included via the epoll helpers when those emit, but we also
/// re-include here for the case where only `sleep_ms` is used
/// without any epoll/TCP helpers. Returns 0 on success; -1 on
/// EINTR (caller can retry on POSIX). Negative `ms` is a no-op.
fn emit_intent_sleep_ms_helper_c(out: &mut String, body: &str) {
    if !body.contains("intent_sleep_ms") {
        return;
    }
    out.push_str(
        "#if defined(_WIN32)\n\
         #include <windows.h>\n\
         static INTENT_UNUSED int64_t intent_sleep_ms(int64_t ms) {\n\
         \x20 if (ms <= 0) return 0;\n\
         \x20 Sleep((DWORD)ms);\n\
         \x20 return 0;\n\
         }\n\
         #else\n\
         #include <time.h>\n\
         #include <errno.h>\n\
         static INTENT_UNUSED int64_t intent_sleep_ms(int64_t ms) {\n\
         \x20 if (ms <= 0) return 0;\n\
         \x20 struct timespec req; struct timespec rem;\n\
         \x20 req.tv_sec = (time_t)(ms / 1000);\n\
         \x20 req.tv_nsec = (long)((ms % 1000) * 1000000L);\n\
         \x20 while (nanosleep(&req, &rem) == -1) {\n\
         \x20   if (errno != EINTR) return -1;\n\
         \x20   req = rem;\n\
         \x20 }\n\
         \x20 return 0;\n\
         }\n\
         #endif\n\n",
    );
}

/// Data-structures roadmap Level 1 — runtime helpers for the
/// array variants of `sort` / `sort_by` / `reverse` / `find` /
/// `contains` / `binary_search`. v1: i64 element only. Arrays
/// are pointer + length so a single set of helpers covers
/// every `[i64; N]` shape. The unconditional variant always
/// emits the helpers — the call site is gated by
/// `program_uses_i64_array`.
/// Data-structures roadmap Level 1 — RNG runtime helpers
/// (xorshift64). Thread-local state means each `task` has an
/// independent stream. seed_rng(0) resets to a fixed nonzero
/// default to avoid the xorshift trap of getting stuck at 0.
fn emit_intent_rng_helpers_c(out: &mut String, body: &str) {
    if !body.contains("intent_rng_") {
        return;
    }
    out.push_str(
        "static _Thread_local uint64_t intent_rng_state = 0x123456789abcdef0ULL;\n\
         static INTENT_UNUSED int64_t intent_rng_seed(uint64_t s) {\n\
         \x20 intent_rng_state = s == 0 ? 0x123456789abcdef0ULL : s;\n\
         \x20 return 0;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_rng_next(void) {\n\
         \x20 uint64_t x = intent_rng_state;\n\
         \x20 x ^= x << 13;\n\
         \x20 x ^= x >> 7;\n\
         \x20 x ^= x << 17;\n\
         \x20 intent_rng_state = x;\n\
         \x20 return (int64_t)x;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_rng_in_range(int64_t lo, int64_t hi) {\n\
         \x20 if (lo >= hi) return lo;\n\
         \x20 uint64_t span = (uint64_t)(hi - lo);\n\
         \x20 uint64_t r = (uint64_t)intent_rng_next();\n\
         \x20 return lo + (int64_t)(r % span);\n\
         }\n\n",
    );
}

/// Walk the program for any `Deque<i64>` type usage. Triggers
/// emission of the deque runtime helpers in body (so the
/// Enum_Option__i64 typedef is visible when the pop/peek
/// helpers are defined).
pub(crate) fn program_uses_i64_deque(program: &TypedProgram) -> bool {
    fn ty_uses(ty: &Type) -> bool {
        match ty {
            Type::Deque(element) if matches!(**element, Type::I64) => true,
            Type::Vec(inner) | Type::Ref(inner) | Type::RefMut(inner) => ty_uses(inner),
            Type::Array { element, .. } => ty_uses(element),
            _ => false,
        }
    }
    for f in &program.functions {
        if ty_uses(&f.return_type) {
            return true;
        }
        for p in &f.params {
            if ty_uses(&p.ty) {
                return true;
            }
        }
        for s in &f.body {
            if stmt_uses_i64_deque(s) {
                return true;
            }
        }
    }
    false
}

fn stmt_uses_i64_deque(stmt: &crate::ir::TypedStmt) -> bool {
    use crate::ir::TypedStmt as S;
    fn ty_uses(ty: &Type) -> bool {
        matches!(ty, Type::Deque(element) if matches!(**element, Type::I64))
            || matches!(ty,
                Type::Vec(i) | Type::Ref(i) | Type::RefMut(i) if ty_uses(i))
    }
    match stmt {
        S::Let { ty, .. } | S::Reassign { ty, .. } | S::Drop { ty, .. } => ty_uses(ty),
        S::If { then_body, else_body, .. } => {
            then_body.iter().any(stmt_uses_i64_deque)
                || else_body.iter().any(stmt_uses_i64_deque)
        }
        S::While { body, .. } | S::For { body, .. } | S::ForIter { body, .. } => {
            body.iter().any(stmt_uses_i64_deque)
        }
        _ => false,
    }
}

/// Data-structures roadmap Level 2 — Deque<i64> ring buffer
/// runtime helpers. `intent_deque_i64` is a 4-field struct:
/// data pointer, front index, len, capacity. Mod-capacity
/// arithmetic implements the wrap-around. Grow doubles
/// capacity and unwraps the ring so future ops see a
/// contiguous prefix. v1 i64 only; Option<i64> return for
/// pop/peek gated on `has_option_i64` flag from the caller.
fn emit_intent_deque_helpers_c_body(out: &mut String, has_option_i64: bool) {
    out.push_str(
        "typedef struct { int64_t* data; uint64_t front; uint64_t len; uint64_t capacity; } intent_deque_i64;\n\
         static INTENT_UNUSED intent_deque_i64 intent_deque_i64_new(void) {\n\
         \x20 intent_deque_i64 d; d.data = (int64_t*)0; d.front = 0; d.len = 0; d.capacity = 0; return d;\n\
         }\n\
         static INTENT_UNUSED void intent_deque_i64_drop(intent_deque_i64* d) {\n\
         \x20 if (d->data) free(d->data);\n\
         \x20 d->data = (int64_t*)0; d->front = 0; d->len = 0; d->capacity = 0;\n\
         }\n\
         static INTENT_UNUSED void intent_deque_i64_grow(intent_deque_i64* d) {\n\
         \x20 uint64_t new_cap = d->capacity == 0 ? 4 : d->capacity * 2;\n\
         \x20 int64_t* new_data = (int64_t*)malloc(new_cap * sizeof(int64_t));\n\
         \x20 if (!new_data) abort();\n\
         \x20 /* Unwrap the ring into the new buffer. */\n\
         \x20 for (uint64_t i = 0; i < d->len; i++) {\n\
         \x20   new_data[i] = d->data[(d->front + i) % d->capacity];\n\
         \x20 }\n\
         \x20 if (d->data) free(d->data);\n\
         \x20 d->data = new_data;\n\
         \x20 d->front = 0;\n\
         \x20 d->capacity = new_cap;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_deque_i64_push_back(intent_deque_i64* d, int64_t v) {\n\
         \x20 if (d->len >= d->capacity) intent_deque_i64_grow(d);\n\
         \x20 uint64_t back = (d->front + d->len) % d->capacity;\n\
         \x20 d->data[back] = v;\n\
         \x20 d->len++;\n\
         \x20 return (int64_t)d->len;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_deque_i64_push_front(intent_deque_i64* d, int64_t v) {\n\
         \x20 if (d->len >= d->capacity) intent_deque_i64_grow(d);\n\
         \x20 d->front = (d->front + d->capacity - 1) % d->capacity;\n\
         \x20 d->data[d->front] = v;\n\
         \x20 d->len++;\n\
         \x20 return (int64_t)d->len;\n\
         }\n\
         /* Closure #354: clear() — free the ring buffer, reset to\n\
          * empty. Returns prior len. */\n\
         static INTENT_UNUSED int64_t intent_deque_i64_clear(intent_deque_i64* d) {\n\
         \x20 int64_t prior = (int64_t)d->len;\n\
         \x20 if (d->data) free(d->data);\n\
         \x20 d->data = (int64_t*)0;\n\
         \x20 d->front = 0;\n\
         \x20 d->len = 0;\n\
         \x20 d->capacity = 0;\n\
         \x20 return prior;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_deque_i64_len(const intent_deque_i64* d) {\n\
         \x20 return (int64_t)d->len;\n\
         }\n",
    );
    if has_option_i64 {
        out.push_str(
            "static INTENT_UNUSED Enum_Option__i64 intent_deque_i64_pop_back(intent_deque_i64* d) {\n\
             \x20 Enum_Option__i64 r;\n\
             \x20 if (d->len == 0) { r.tag = 1; r.payload = 0; return r; }\n\
             \x20 d->len--;\n\
             \x20 uint64_t back = (d->front + d->len) % d->capacity;\n\
             \x20 r.tag = 0; r.payload = d->data[back];\n\
             \x20 return r;\n\
             }\n\
             static INTENT_UNUSED Enum_Option__i64 intent_deque_i64_pop_front(intent_deque_i64* d) {\n\
             \x20 Enum_Option__i64 r;\n\
             \x20 if (d->len == 0) { r.tag = 1; r.payload = 0; return r; }\n\
             \x20 r.tag = 0; r.payload = d->data[d->front];\n\
             \x20 d->front = (d->front + 1) % d->capacity;\n\
             \x20 d->len--;\n\
             \x20 return r;\n\
             }\n\
             static INTENT_UNUSED Enum_Option__i64 intent_deque_i64_peek_back(const intent_deque_i64* d) {\n\
             \x20 Enum_Option__i64 r;\n\
             \x20 if (d->len == 0) { r.tag = 1; r.payload = 0; return r; }\n\
             \x20 uint64_t back = (d->front + d->len - 1) % d->capacity;\n\
             \x20 r.tag = 0; r.payload = d->data[back];\n\
             \x20 return r;\n\
             }\n\
             static INTENT_UNUSED Enum_Option__i64 intent_deque_i64_peek_front(const intent_deque_i64* d) {\n\
             \x20 Enum_Option__i64 r;\n\
             \x20 if (d->len == 0) { r.tag = 1; r.payload = 0; return r; }\n\
             \x20 r.tag = 0; r.payload = d->data[d->front];\n\
             \x20 return r;\n\
             }\n\n",
        );
    }
}

/// Walk the program for any `HashSet<i64>` type usage.
pub(crate) fn program_uses_region(program: &TypedProgram) -> bool {
    fn ty_uses(ty: &Type) -> bool {
        match ty {
            Type::Region => true,
            Type::Vec(inner) | Type::Ref(inner) | Type::RefMut(inner) => ty_uses(inner),
            _ => false,
        }
    }
    fn expr_uses(e: &crate::ir::TypedExpr) -> bool {
        use crate::ir::TypedExprKind as K;
        if ty_uses(&e.ty) { return true; }
        match &e.kind {
            K::Call { name, args, .. } => {
                matches!(name.as_str(), "region_new" | "region_alloc_i64" | "region_len")
                    || args.iter().any(expr_uses)
            }
            K::Binary { left, right, .. } => expr_uses(left) || expr_uses(right),
            K::Unary { expr, .. } | K::Cast { expr, .. } => expr_uses(expr),
            _ => false,
        }
    }
    fn stmt_uses(s: &crate::ir::TypedStmt) -> bool {
        use crate::ir::TypedStmt as S;
        match s {
            S::Let { ty, expr, .. } | S::Reassign { ty, expr, .. } => {
                ty_uses(ty) || expr_uses(expr)
            }
            S::Drop { ty, .. } => ty_uses(ty),
            S::Return { expr } | S::Assert { expr, .. } | S::Prove { expr } => expr_uses(expr),
            S::Discard { expr } => expr_uses(expr),
            S::If { cond, then_body, else_body } => {
                expr_uses(cond) || then_body.iter().any(stmt_uses) || else_body.iter().any(stmt_uses)
            }
            S::While { cond, body } => expr_uses(cond) || body.iter().any(stmt_uses),
            S::For { start, end, body, .. } => {
                expr_uses(start) || expr_uses(end) || body.iter().any(stmt_uses)
            }
            S::ForIter { body, .. }
            | S::TaskSpawn { body, .. }
            | S::UnsafeBlock { body, .. } => body.iter().any(stmt_uses),
            S::IndexAssign { index, value, .. } => expr_uses(index) || expr_uses(value),
            S::FieldAssign { object, value, .. } => expr_uses(object) || expr_uses(value),
            _ => false,
        }
    }
    for f in &program.functions {
        if ty_uses(&f.return_type) { return true; }
        for p in &f.params { if ty_uses(&p.ty) { return true; } }
        if f.body.iter().any(stmt_uses) { return true; }
    }
    false
}

pub(crate) fn program_uses_bptr(program: &TypedProgram) -> bool {
    fn ty_uses(ty: &Type) -> bool {
        match ty {
            Type::BoundedPtr(_) => true,
            Type::Vec(inner) | Type::Ref(inner) | Type::RefMut(inner) => ty_uses(inner),
            _ => false,
        }
    }
    fn expr_uses(e: &crate::ir::TypedExpr) -> bool {
        use crate::ir::TypedExprKind as K;
        if ty_uses(&e.ty) { return true; }
        match &e.kind {
            K::Call { name, args, .. } => {
                matches!(name.as_str(), "bptr_new" | "bptr_get" | "bptr_set" | "bptr_len")
                    || args.iter().any(expr_uses)
            }
            K::Binary { left, right, .. } => expr_uses(left) || expr_uses(right),
            K::Unary { expr, .. } | K::Cast { expr, .. } => expr_uses(expr),
            _ => false,
        }
    }
    fn stmt_uses(s: &crate::ir::TypedStmt) -> bool {
        use crate::ir::TypedStmt as S;
        match s {
            S::Let { ty, expr, .. } | S::Reassign { ty, expr, .. } => {
                ty_uses(ty) || expr_uses(expr)
            }
            S::Drop { ty, .. } => ty_uses(ty),
            S::Return { expr } | S::Assert { expr, .. } | S::Prove { expr } => expr_uses(expr),
            S::Discard { expr } => expr_uses(expr),
            S::If { cond, then_body, else_body } => {
                expr_uses(cond) || then_body.iter().any(stmt_uses) || else_body.iter().any(stmt_uses)
            }
            S::While { cond, body } => expr_uses(cond) || body.iter().any(stmt_uses),
            S::For { start, end, body, .. } => {
                expr_uses(start) || expr_uses(end) || body.iter().any(stmt_uses)
            }
            S::ForIter { body, .. }
            | S::TaskSpawn { body, .. }
            | S::UnsafeBlock { body, .. } => body.iter().any(stmt_uses),
            S::IndexAssign { index, value, .. } => expr_uses(index) || expr_uses(value),
            S::FieldAssign { object, value, .. } => expr_uses(object) || expr_uses(value),
            _ => false,
        }
    }
    for f in &program.functions {
        if ty_uses(&f.return_type) { return true; }
        for p in &f.params { if ty_uses(&p.ty) { return true; } }
        if f.body.iter().any(stmt_uses) { return true; }
    }
    false
}

pub(crate) fn program_uses_unsafe_alloc(program: &TypedProgram) -> bool {
    fn expr_uses(e: &crate::ir::TypedExpr) -> bool {
        use crate::ir::TypedExprKind as K;
        match &e.kind {
            K::Call { name, args, .. } => {
                name == "unsafe_alloc" || name == "unsafe_free"
                    || args.iter().any(expr_uses)
            }
            K::Binary { left, right, .. } => expr_uses(left) || expr_uses(right),
            K::Unary { expr, .. } | K::Cast { expr, .. } => expr_uses(expr),
            _ => false,
        }
    }
    fn stmt_uses(s: &crate::ir::TypedStmt) -> bool {
        use crate::ir::TypedStmt as S;
        match s {
            S::Let { expr, .. } | S::Reassign { expr, .. } => expr_uses(expr),
            S::Return { expr } | S::Assert { expr, .. } | S::Prove { expr } => expr_uses(expr),
            S::Discard { expr } => expr_uses(expr),
            S::If { cond, then_body, else_body } => {
                expr_uses(cond)
                    || then_body.iter().any(stmt_uses)
                    || else_body.iter().any(stmt_uses)
            }
            S::While { cond, body } => expr_uses(cond) || body.iter().any(stmt_uses),
            S::For { start, end, body, .. } => {
                expr_uses(start) || expr_uses(end) || body.iter().any(stmt_uses)
            }
            S::ForIter { body, .. }
            | S::TaskSpawn { body, .. }
            | S::UnsafeBlock { body, .. } => body.iter().any(stmt_uses),
            S::IndexAssign { index, value, .. } => expr_uses(index) || expr_uses(value),
            S::FieldAssign { object, value, .. } => expr_uses(object) || expr_uses(value),
            _ => false,
        }
    }
    program.functions.iter().any(|f| f.body.iter().any(stmt_uses))
}

pub(crate) fn program_uses_i64_pool(program: &TypedProgram) -> bool {
    fn ty_uses(ty: &Type) -> bool {
        match ty {
            Type::Pool(element) if matches!(**element, Type::I64) => true,
            Type::Handle(element) if matches!(**element, Type::I64) => true,
            Type::Vec(inner) | Type::Ref(inner) | Type::RefMut(inner) => ty_uses(inner),
            _ => false,
        }
    }
    fn stmt_uses(s: &crate::ir::TypedStmt) -> bool {
        use crate::ir::TypedStmt as S;
        match s {
            S::Let { ty, .. } | S::Reassign { ty, .. } | S::Drop { ty, .. } => ty_uses(ty),
            S::If { then_body, else_body, .. } => {
                then_body.iter().any(stmt_uses) || else_body.iter().any(stmt_uses)
            }
            S::While { body, .. } | S::For { body, .. } | S::ForIter { body, .. } => {
                body.iter().any(stmt_uses)
            }
            S::TaskSpawn { body, .. } | S::UnsafeBlock { body, .. } => {
                body.iter().any(stmt_uses)
            }
            _ => false,
        }
    }
    for f in &program.functions {
        if ty_uses(&f.return_type) {
            return true;
        }
        for p in &f.params {
            if ty_uses(&p.ty) {
                return true;
            }
        }
        for s in &f.body {
            if stmt_uses(s) {
                return true;
            }
        }
    }
    false
}

pub(crate) fn program_uses_i64_hashset(program: &TypedProgram) -> bool {
    fn ty_uses(ty: &Type) -> bool {
        match ty {
            Type::HashSet(element) if matches!(**element, Type::I64) => true,
            Type::Vec(inner) | Type::Ref(inner) | Type::RefMut(inner) => ty_uses(inner),
            _ => false,
        }
    }
    for f in &program.functions {
        if ty_uses(&f.return_type) {
            return true;
        }
        for p in &f.params {
            if ty_uses(&p.ty) {
                return true;
            }
        }
        for s in &f.body {
            if stmt_uses_i64_hashset(s) {
                return true;
            }
        }
    }
    false
}

fn stmt_uses_i64_hashset(stmt: &crate::ir::TypedStmt) -> bool {
    use crate::ir::TypedStmt as S;
    fn ty_uses(ty: &Type) -> bool {
        matches!(ty, Type::HashSet(element) if matches!(**element, Type::I64))
            || matches!(ty,
                Type::Vec(i) | Type::Ref(i) | Type::RefMut(i) if ty_uses(i))
    }
    match stmt {
        S::Let { ty, .. } | S::Reassign { ty, .. } | S::Drop { ty, .. } => ty_uses(ty),
        S::If { then_body, else_body, .. } => {
            then_body.iter().any(stmt_uses_i64_hashset)
                || else_body.iter().any(stmt_uses_i64_hashset)
        }
        S::While { body, .. } | S::For { body, .. } | S::ForIter { body, .. } => {
            body.iter().any(stmt_uses_i64_hashset)
        }
        _ => false,
    }
}

/// Layer 2 of `unsafe.md` — `Pool<i64>` / `Handle<i64>`
/// runtime helpers. Generational slot pool.
///
/// `intent_pool_i64`: `{ int64_t* slots; uint32_t* generations;
///                       uint32_t* free_list; size_t len;
///                       size_t capacity; size_t free_count; }`.
/// `intent_handle_i64`: `{ uint32_t slot_idx; uint32_t generation; }`
/// (8 bytes total — same machine width as a pointer on 64-bit).
///
/// Operations:
/// - `pool_new` zero-inits the struct (all pointers NULL).
/// - `pool_alloc` reuses a free-list slot when available; else
///   appends a new slot (doubling capacity on demand). Every
///   newly-allocated-or-reused slot has its generation set to
///   the next nonzero value (starting at 1; zero is reserved
///   for "this slot has never been allocated"). Returns
///   `{slot_idx, generation}` — the handle.
/// - `pool_get` checks `slot_idx < len && generations[slot_idx]
///   == handle.generation`. On match → `Option::Some(value)`. On
///   mismatch → `Option::None` (the load-bearing UAF /
///   double-free signal).
/// - `pool_free` checks the same generation predicate. On match,
///   bumps the slot's generation (so any surviving handle to
///   the slot will see a mismatch next time) and pushes the
///   `slot_idx` onto `free_list`. On mismatch → silent no-op
///   (double-free is harmless).
/// - `intent_pool_i64_drop` frees the three heap arrays at
///   scope exit (affine).
///
/// The Option<i64> return depends on the `Enum_Option__i64`
/// typedef having been emitted by the enum-bundle pass. Same
/// pattern as hashmap_get / btreeset_min / etc. The
/// `has_option_i64` flag at the call site gates the bundle
/// (and inhibits emission if Option<i64> isn't in the
/// program — the bundle's `pool_get` returns Option<i64>, so
/// without that decl in the registry, the program never
/// actually exercises `pool_get` and we can omit it
/// safely... but for simplicity we always emit the bundle
/// when Pool<i64>/Handle<i64> appears, and rely on the
/// Option__i64 monomorph being auto-registered by the
/// pool_get-uses-Option pre-pass).
fn emit_intent_pool_helpers_c_body(out: &mut String) {
    out.push_str(
        "typedef struct { uint32_t slot_idx; uint32_t generation; } intent_handle_i64;\n\
         typedef struct { int64_t* slots; uint32_t* generations; uint32_t* free_list; size_t len; size_t capacity; size_t free_count; } intent_pool_i64;\n\
         static INTENT_UNUSED intent_pool_i64 intent_pool_i64_new(void) {\n\
         \x20 intent_pool_i64 p;\n\
         \x20 p.slots = (int64_t*)0;\n\
         \x20 p.generations = (uint32_t*)0;\n\
         \x20 p.free_list = (uint32_t*)0;\n\
         \x20 p.len = 0; p.capacity = 0; p.free_count = 0;\n\
         \x20 return p;\n\
         }\n\
         static INTENT_UNUSED void intent_pool_i64_drop(intent_pool_i64* p) {\n\
         \x20 if (p->slots) free(p->slots);\n\
         \x20 if (p->generations) free(p->generations);\n\
         \x20 if (p->free_list) free(p->free_list);\n\
         \x20 p->slots = (int64_t*)0; p->generations = (uint32_t*)0; p->free_list = (uint32_t*)0;\n\
         \x20 p->len = 0; p->capacity = 0; p->free_count = 0;\n\
         }\n\
         static INTENT_UNUSED void intent_pool_i64__grow(intent_pool_i64* p) {\n\
         \x20 size_t new_cap = p->capacity == 0 ? 4 : p->capacity * 2;\n\
         \x20 int64_t* slots = (int64_t*)realloc(p->slots, new_cap * sizeof(int64_t));\n\
         \x20 uint32_t* gens = (uint32_t*)realloc(p->generations, new_cap * sizeof(uint32_t));\n\
         \x20 uint32_t* freelist = (uint32_t*)realloc(p->free_list, new_cap * sizeof(uint32_t));\n\
         \x20 if (!slots || !gens || !freelist) abort();\n\
         \x20 /* Zero the newly-grown generation slots so the\n\
         \x20  * first allocation into them starts at gen=1. */\n\
         \x20 for (size_t i = p->capacity; i < new_cap; i++) gens[i] = 0;\n\
         \x20 p->slots = slots; p->generations = gens; p->free_list = freelist;\n\
         \x20 p->capacity = new_cap;\n\
         }\n\
         static INTENT_UNUSED intent_handle_i64 intent_pool_i64_alloc(intent_pool_i64* p, int64_t v) {\n\
         \x20 intent_handle_i64 h;\n\
         \x20 if (p->free_count > 0) {\n\
         \x20   /* Reuse a free slot. Generation was bumped at\n\
         \x20    * free time; just write the value back. */\n\
         \x20   p->free_count--;\n\
         \x20   uint32_t idx = p->free_list[p->free_count];\n\
         \x20   p->slots[idx] = v;\n\
         \x20   h.slot_idx = idx;\n\
         \x20   h.generation = p->generations[idx];\n\
         \x20   return h;\n\
         \x20 }\n\
         \x20 if (p->len == p->capacity) intent_pool_i64__grow(p);\n\
         \x20 uint32_t idx = (uint32_t)p->len;\n\
         \x20 p->slots[idx] = v;\n\
         \x20 /* Fresh slot: bump generation from 0 to 1 so the\n\
         \x20  * handle distinguishes from the never-allocated\n\
         \x20  * sentinel. */\n\
         \x20 p->generations[idx] = 1;\n\
         \x20 p->len++;\n\
         \x20 h.slot_idx = idx;\n\
         \x20 h.generation = 1;\n\
         \x20 return h;\n\
         }\n\
         static INTENT_UNUSED Enum_Option__i64 intent_pool_i64_get(const intent_pool_i64* p, intent_handle_i64 h) {\n\
         \x20 Enum_Option__i64 r;\n\
         \x20 if (h.slot_idx >= p->len || p->generations[h.slot_idx] != h.generation) {\n\
         \x20   r.tag = 1; r.payload = 0; /* Option::None */\n\
         \x20   return r;\n\
         \x20 }\n\
         \x20 r.tag = 0; /* Option::Some */\n\
         \x20 r.payload = p->slots[h.slot_idx];\n\
         \x20 return r;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_pool_i64_free(intent_pool_i64* p, intent_handle_i64 h) {\n\
         \x20 /* Generation check turns double-free into a\n\
         \x20  * silent no-op (and use-after-free in pool_get\n\
         \x20  * into a None). */\n\
         \x20 if (h.slot_idx >= p->len || p->generations[h.slot_idx] != h.generation) return 0;\n\
         \x20 p->generations[h.slot_idx]++;\n\
         \x20 p->free_list[p->free_count] = h.slot_idx;\n\
         \x20 p->free_count++;\n\
         \x20 return 0;\n\
         }\n\n",
    );
}

/// Layer 3.1 of `unsafe.md` — canary-protected heap
/// allocator. Replaces the prior inline-calloc lowering of
/// `unsafe_alloc` / `unsafe_free` with helper calls so the
/// canary words land at consistent offsets.
///
/// Memory layout per allocation:
///   [ size: i64 ][ MAGIC_PREFIX: i64 ][ user N×i64 ][ MAGIC_SUFFIX: i64 ]
///    ^base       ^base+8              ^base+16      ^base+16+N*8
///
/// The returned user pointer is `base + 16`. `unsafe_free`
/// walks back to `base`, verifies both canaries, then frees.
/// A mismatch on either canary aborts the program with a clear
/// diagnostic — the call site is the most recent thing in the
/// stack trace, which is exactly what an incident reviewer
/// wants.
///
/// Cost analysis (Cortex-M / x86-64 64-bit):
/// - +24 bytes per allocation (8 size + 8 prefix-canary +
///   8 suffix-canary).
/// - ~4 cycles per `unsafe_free` (two i64 loads + two
///   comparisons + the existing `free` call).
/// - Zero cost on the read/write path — `raw_load` /
///   `raw_store` go straight through the user pointer; the
///   canary words live outside that addressable range.
fn emit_intent_unsafe_alloc_helpers_c_body(out: &mut String) {
    out.push_str(
        "#define INTENT_UNSAFE_ALLOC_PREFIX_CANARY ((int64_t)0xDEADBEEFCAFEBABEULL)\n\
         #define INTENT_UNSAFE_ALLOC_SUFFIX_CANARY ((int64_t)0xBAADF00DDEADC0DEULL)\n\
         static INTENT_UNUSED int64_t* intent_unsafe_alloc(int64_t n) {\n\
         \x20 if (n < 0) abort();\n\
         \x20 size_t total = ((size_t)n + 3) * sizeof(int64_t);\n\
         \x20 int64_t* base = (int64_t*)calloc(total, 1);\n\
         \x20 if (!base) abort();\n\
         \x20 base[0] = n;\n\
         \x20 base[1] = INTENT_UNSAFE_ALLOC_PREFIX_CANARY;\n\
         \x20 base[n + 2] = INTENT_UNSAFE_ALLOC_SUFFIX_CANARY;\n\
         \x20 return base + 2;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_unsafe_free(int64_t* p) {\n\
         \x20 if (!p) return 0;\n\
         \x20 int64_t* base = p - 2;\n\
         \x20 int64_t n = base[0];\n\
         \x20 if (base[1] != INTENT_UNSAFE_ALLOC_PREFIX_CANARY) {\n\
         \x20   fprintf(stderr, \"intent: unsafe_alloc prefix canary corrupted (buffer underrun?)\\n\");\n\
         \x20   abort();\n\
         \x20 }\n\
         \x20 if (base[n + 2] != INTENT_UNSAFE_ALLOC_SUFFIX_CANARY) {\n\
         \x20   fprintf(stderr, \"intent: unsafe_alloc suffix canary corrupted (buffer overrun?)\\n\");\n\
         \x20   abort();\n\
         \x20 }\n\
         \x20 free(base);\n\
         \x20 return 0;\n\
         }\n\n",
    );
}

/// Layer 3.2 of `unsafe.md` — `BoundedPtr<i64>` fat pointer
/// helpers. The struct wraps a raw pointer with its bounds:
///   { int64_t* data; uint64_t len; uint64_t capacity; }
///
/// `bptr_get` / `bptr_set` are bounds-checked at runtime
/// against `len`. Out-of-bounds reads return `Option::None`;
/// out-of-bounds writes return `false` with no store performed.
/// `capacity` is recorded for future-resize APIs (Layer 3.2+
/// could add `bptr_with_capacity` / `bptr_push` patterns) but
/// not yet exercised — `len` is the active bound for v1.
fn emit_intent_bptr_helpers_c_body(out: &mut String) {
    out.push_str(
        "typedef struct { int64_t* data; uint64_t len; uint64_t capacity; } intent_bptr_i64;\n\
         static INTENT_UNUSED intent_bptr_i64 intent_bptr_i64_new(int64_t* p, int64_t len, int64_t capacity) {\n\
         \x20 intent_bptr_i64 bp;\n\
         \x20 bp.data = p;\n\
         \x20 bp.len = (uint64_t)(len < 0 ? 0 : len);\n\
         \x20 bp.capacity = (uint64_t)(capacity < 0 ? 0 : capacity);\n\
         \x20 return bp;\n\
         }\n\
         static INTENT_UNUSED Enum_Option__i64 intent_bptr_i64_get(const intent_bptr_i64* bp, int64_t i) {\n\
         \x20 Enum_Option__i64 r;\n\
         \x20 if (i < 0 || (uint64_t)i >= bp->len) { r.tag = 1; r.payload = 0; return r; }\n\
         \x20 r.tag = 0; r.payload = bp->data[i]; return r;\n\
         }\n\
         static INTENT_UNUSED bool intent_bptr_i64_set(intent_bptr_i64* bp, int64_t i, int64_t v) {\n\
         \x20 if (i < 0 || (uint64_t)i >= bp->len) return false;\n\
         \x20 bp->data[i] = v; return true;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_bptr_i64_len(const intent_bptr_i64* bp) {\n\
         \x20 return (int64_t)bp->len;\n\
         }\n\n",
    );
}

/// Layer 5 v2 foundation of `unsafe.md` — `Region` bump-
/// allocator arena helpers.
///
/// Struct layout: `{ int64_t* data; size_t len; size_t capacity; }`
///
/// `region_new`: zero-initializes the struct (lazy allocation —
/// the first `region_alloc_i64` triggers the initial malloc).
///
/// `region_alloc_i64(r, v)`: if `len == capacity`, grow the
/// buffer (double, starting from 8 slots). Write `v` into
/// `data[len]`, increment `len`, return `data + len - 1`.
///
/// `region_drop`: a single `free` on the data buffer. Every
/// allocation in the arena gets freed together; no per-slot
/// bookkeeping. Deterministic O(1) regardless of allocation
/// count.
///
/// Note that `data` may be reallocated as the arena grows.
/// Pointers handed out by `region_alloc_i64` before a grow
/// become stale after the grow — a hazard the full Layer 5
/// design avoids via `&'arena T` lifetime tracking. This
/// v1 scaffolding documents the hazard but doesn't yet
/// prevent it; users must complete all allocations before
/// using any returned pointers, OR set capacity up front via
/// a future `region_with_capacity` builtin.
fn emit_intent_region_helpers_c_body(out: &mut String) {
    out.push_str(
        "typedef struct { int64_t* data; size_t len; size_t capacity; } intent_region;\n\
         static INTENT_UNUSED intent_region intent_region_new(void) {\n\
         \x20 intent_region r;\n\
         \x20 r.data = (int64_t*)0;\n\
         \x20 r.len = 0; r.capacity = 0;\n\
         \x20 return r;\n\
         }\n\
         static INTENT_UNUSED void intent_region_drop(intent_region* r) {\n\
         \x20 if (r->data) free(r->data);\n\
         \x20 r->data = (int64_t*)0;\n\
         \x20 r->len = 0; r->capacity = 0;\n\
         }\n\
         static INTENT_UNUSED int64_t* intent_region_alloc_i64(intent_region* r, int64_t v) {\n\
         \x20 if (r->len == r->capacity) {\n\
         \x20   size_t new_cap = r->capacity == 0 ? 8 : r->capacity * 2;\n\
         \x20   int64_t* new_data = (int64_t*)realloc(r->data, new_cap * sizeof(int64_t));\n\
         \x20   if (!new_data) abort();\n\
         \x20   r->data = new_data;\n\
         \x20   r->capacity = new_cap;\n\
         \x20 }\n\
         \x20 r->data[r->len] = v;\n\
         \x20 int64_t* slot = r->data + r->len;\n\
         \x20 r->len++;\n\
         \x20 return slot;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_region_len(const intent_region* r) {\n\
         \x20 return (int64_t)r->len;\n\
         }\n\n",
    );
}

/// Data-structures roadmap Level 2 — HashSet<i64> runtime
/// helpers. Open-addressing linear probing with empty(0) /
/// occupied(1) slot tags. Grow doubles capacity when load
/// >= 50%. Hash via the existing intent_hash_i64. v1 i64
/// only; hashset_remove deferred.
fn emit_intent_hashset_helpers_c_body(out: &mut String) {
    out.push_str(
        "typedef struct { int64_t* keys; uint8_t* occ; uint64_t len; uint64_t capacity; uint64_t tombstones; } intent_hashset_i64;\n\
         /* occ byte states (closure #342):\n\
          *   0 = empty       — terminates probe chains\n\
          *   1 = occupied    — slot in use\n\
          *   2 = tombstone   — slot removed; probe must continue past it\n\
          * `tombstones` counts state-2 slots. Insert / grow treat\n\
          * `(len + tombstones) * 2 >= capacity` as the grow threshold\n\
          * so a remove-heavy workload eventually triggers a rehash\n\
          * that clears the tombstones. */\n\
         static INTENT_UNUSED uint64_t intent_hashset_i64__hash_key(int64_t k);\n\
         static INTENT_UNUSED intent_hashset_i64 intent_hashset_i64_new(void) {\n\
         \x20 intent_hashset_i64 s;\n\
         \x20 s.keys = (int64_t*)0; s.occ = (uint8_t*)0;\n\
         \x20 s.len = 0; s.capacity = 0; s.tombstones = 0;\n\
         \x20 return s;\n\
         }\n\
         static INTENT_UNUSED void intent_hashset_i64_drop(intent_hashset_i64* s) {\n\
         \x20 if (s->keys) free(s->keys);\n\
         \x20 if (s->occ) free(s->occ);\n\
         \x20 s->keys = (int64_t*)0; s->occ = (uint8_t*)0;\n\
         \x20 s->len = 0; s->capacity = 0; s->tombstones = 0;\n\
         }\n\
         /* FNV-1a over the 8 bytes of an i64 — inline so we\n\
         \x20  don't require intent_hash_i64 to be emitted. */\n\
         static INTENT_UNUSED uint64_t intent_hashset_i64__hash_key(int64_t k) {\n\
         \x20 uint64_t h = 0xcbf29ce484222325ULL;\n\
         \x20 uint64_t u = (uint64_t)k;\n\
         \x20 for (int i = 0; i < 8; i++) {\n\
         \x20   h ^= (u >> (i * 8)) & 0xffULL;\n\
         \x20   h *= 0x100000001b3ULL;\n\
         \x20 }\n\
         \x20 return h;\n\
         }\n\
         /* Rehash-only insert (used during grow): assumes the key\n\
          * isn't already in the table and skips the dup check. */\n\
         static INTENT_UNUSED void intent_hashset_i64__insert_into(intent_hashset_i64* s, int64_t k) {\n\
         \x20 uint64_t mask = s->capacity - 1;\n\
         \x20 uint64_t i = intent_hashset_i64__hash_key(k) & mask;\n\
         \x20 while (s->occ[i] == 1) {\n\
         \x20   i = (i + 1) & mask;\n\
         \x20 }\n\
         \x20 s->keys[i] = k; s->occ[i] = 1; s->len++;\n\
         }\n\
         static INTENT_UNUSED void intent_hashset_i64__grow(intent_hashset_i64* s) {\n\
         \x20 uint64_t old_cap = s->capacity;\n\
         \x20 int64_t* old_keys = s->keys;\n\
         \x20 uint8_t* old_occ = s->occ;\n\
         \x20 uint64_t new_cap = old_cap == 0 ? 8 : old_cap * 2;\n\
         \x20 s->keys = (int64_t*)malloc(new_cap * sizeof(int64_t));\n\
         \x20 s->occ = (uint8_t*)calloc(new_cap, 1);\n\
         \x20 if (!s->keys || !s->occ) abort();\n\
         \x20 s->len = 0;\n\
         \x20 s->capacity = new_cap;\n\
         \x20 s->tombstones = 0;\n\
         \x20 for (uint64_t i = 0; i < old_cap; i++) {\n\
         \x20   if (old_occ[i] == 1) intent_hashset_i64__insert_into(s, old_keys[i]);\n\
         \x20 }\n\
         \x20 if (old_keys) free(old_keys);\n\
         \x20 if (old_occ) free(old_occ);\n\
         }\n\
         static INTENT_UNUSED bool intent_hashset_i64_insert(intent_hashset_i64* s, int64_t k) {\n\
         \x20 if (s->capacity == 0 || ((s->len + s->tombstones) * 2) >= s->capacity) intent_hashset_i64__grow(s);\n\
         \x20 uint64_t mask = s->capacity - 1;\n\
         \x20 uint64_t i = intent_hashset_i64__hash_key(k) & mask;\n\
         \x20 /* First-tombstone-or-empty placement strategy: walk past\n\
          * tombstones in case the key already lives further down the\n\
          * probe chain; remember the first tombstone position so we\n\
          * can reuse it if we hit an empty slot without finding the\n\
          * key. */\n\
         \x20 int64_t first_tomb = -1;\n\
         \x20 while (s->occ[i] != 0) {\n\
         \x20   if (s->occ[i] == 1 && s->keys[i] == k) return false; /* already present */\n\
         \x20   if (s->occ[i] == 2 && first_tomb == -1) first_tomb = (int64_t)i;\n\
         \x20   i = (i + 1) & mask;\n\
         \x20 }\n\
         \x20 if (first_tomb != -1) {\n\
         \x20   uint64_t slot = (uint64_t)first_tomb;\n\
         \x20   s->keys[slot] = k; s->occ[slot] = 1;\n\
         \x20   s->len++; s->tombstones--;\n\
         \x20 } else {\n\
         \x20   s->keys[i] = k; s->occ[i] = 1; s->len++;\n\
         \x20 }\n\
         \x20 return true;\n\
         }\n\
         static INTENT_UNUSED bool intent_hashset_i64_contains(const intent_hashset_i64* s, int64_t k) {\n\
         \x20 if (s->capacity == 0) return false;\n\
         \x20 uint64_t mask = s->capacity - 1;\n\
         \x20 uint64_t i = intent_hashset_i64__hash_key(k) & mask;\n\
         \x20 while (s->occ[i] != 0) {\n\
         \x20   if (s->occ[i] == 1 && s->keys[i] == k) return true;\n\
         \x20   i = (i + 1) & mask;\n\
         \x20 }\n\
         \x20 return false;\n\
         }\n\
         /* Closure #342: remove a key. Probe past tombstones until we\n\
          * hit a matching occupied slot (mark as tombstone, increment\n\
          * tombstones, decrement len) or an empty slot (key absent —\n\
          * return false). */\n\
         static INTENT_UNUSED bool intent_hashset_i64_remove(intent_hashset_i64* s, int64_t k) {\n\
         \x20 if (s->capacity == 0) return false;\n\
         \x20 uint64_t mask = s->capacity - 1;\n\
         \x20 uint64_t i = intent_hashset_i64__hash_key(k) & mask;\n\
         \x20 while (s->occ[i] != 0) {\n\
         \x20   if (s->occ[i] == 1 && s->keys[i] == k) {\n\
         \x20     s->occ[i] = 2;\n\
         \x20     s->len--;\n\
         \x20     s->tombstones++;\n\
         \x20     return true;\n\
         \x20   }\n\
         \x20   i = (i + 1) & mask;\n\
         \x20 }\n\
         \x20 return false;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_hashset_i64_len(const intent_hashset_i64* s) {\n\
         \x20 return (int64_t)s->len;\n\
         }\n\
         /* Closure #353: clear() — drop the backing buffers and\n\
          * reset to empty state (capacity=0 too, so the next\n\
          * insert reallocates fresh). Returns prior len. */\n\
         static INTENT_UNUSED int64_t intent_hashset_i64_clear(intent_hashset_i64* s) {\n\
         \x20 int64_t prior = (int64_t)s->len;\n\
         \x20 if (s->keys) free(s->keys);\n\
         \x20 if (s->occ) free(s->occ);\n\
         \x20 s->keys = (int64_t*)0;\n\
         \x20 s->occ = (uint8_t*)0;\n\
         \x20 s->len = 0;\n\
         \x20 s->capacity = 0;\n\
         \x20 s->tombstones = 0;\n\
         \x20 return prior;\n\
         }\n\n",
    );
}

/// Walk the program for any `HashMap<i64, i64>` type usage.
pub(crate) fn program_uses_i64_i64_hashmap(program: &TypedProgram) -> bool {
    fn ty_uses(ty: &Type) -> bool {
        match ty {
            Type::HashMap(k, v)
                if matches!(**k, Type::I64) && matches!(**v, Type::I64) =>
            {
                true
            }
            Type::Vec(inner) | Type::Ref(inner) | Type::RefMut(inner) => ty_uses(inner),
            _ => false,
        }
    }
    for f in &program.functions {
        if ty_uses(&f.return_type) {
            return true;
        }
        for p in &f.params {
            if ty_uses(&p.ty) {
                return true;
            }
        }
        for s in &f.body {
            if stmt_uses_i64_i64_hashmap(s) {
                return true;
            }
        }
    }
    false
}

fn stmt_uses_i64_i64_hashmap(stmt: &crate::ir::TypedStmt) -> bool {
    use crate::ir::TypedStmt as S;
    fn ty_uses(ty: &Type) -> bool {
        matches!(ty, Type::HashMap(k, v)
            if matches!(**k, Type::I64) && matches!(**v, Type::I64))
            || matches!(ty,
                Type::Vec(i) | Type::Ref(i) | Type::RefMut(i) if ty_uses(i))
    }
    match stmt {
        S::Let { ty, .. } | S::Reassign { ty, .. } | S::Drop { ty, .. } => ty_uses(ty),
        S::If { then_body, else_body, .. } => {
            then_body.iter().any(stmt_uses_i64_i64_hashmap)
                || else_body.iter().any(stmt_uses_i64_i64_hashmap)
        }
        S::While { body, .. } | S::For { body, .. } | S::ForIter { body, .. } => {
            body.iter().any(stmt_uses_i64_i64_hashmap)
        }
        _ => false,
    }
}

/// Data-structures roadmap Level 2 — HashMap<i64, i64> runtime
/// helpers. Open-addressing with parallel keys/values/occ
/// arrays. Hash via inlined FNV-1a (matches hash_i64). v1
/// (i64, i64) only. `hashmap_get` and `hashmap_insert` return
/// `Option<i64>` and so are gated on the Option__i64 enum
/// being registered. `hashmap_contains_key` / `_len` are
/// always emitted.
/// ARC 1.4c — parameterized HashMap bundle for K = i64, V = any
/// scalar (i8/i16/i32/i64/u8/u16/u32/u64/bool). The legacy
/// `intent_hashmap_i64_i64_*` bundle stays untouched for
/// backwards-compat; this emitter produces additional per-(K, V)
/// bundles named `intent_hashmap_int64_t_<V_tag>_*` (matching
/// the collector's tag scheme from src/hashmap_bundle.rs).
///
/// Arguments:
///   `v_tag`         — V's C-leaf identifier, e.g. "uint32_t"
///   `v_ctype`       — same as v_tag (the C names happen to match)
///   `option_v_mangle` — V's type_mangle suffix, e.g. "u32",
///                      used in the `Enum_Option__<x>` name
///   `has_option_v`  — whether the program registers Option<V>;
///                     gates emission of get / insert / remove
///                     (which return Option<V>)
fn emit_intent_hashmap_pair_c_body(
    out: &mut String,
    v_tag: &str,
    v_ctype: &str,
    option_v_mangle: &str,
    has_option_v: bool,
) {
    let prefix = format!("intent_hashmap_int64_t_{}", v_tag);
    let opt_v = format!("Enum_Option__{}", option_v_mangle);
    out.push_str(&format!(
        "typedef struct {{ int64_t* keys; {v_ctype}* values; uint8_t* occ; uint64_t len; uint64_t capacity; uint64_t tombstones; }} {prefix};\n\
         static INTENT_UNUSED {prefix} {prefix}_new(void) {{\n\
         \x20 {prefix} m;\n\
         \x20 m.keys = (int64_t*)0; m.values = ({v_ctype}*)0; m.occ = (uint8_t*)0;\n\
         \x20 m.len = 0; m.capacity = 0; m.tombstones = 0;\n\
         \x20 return m;\n\
         }}\n\
         static INTENT_UNUSED void {prefix}_drop({prefix}* m) {{\n\
         \x20 if (m->keys) free(m->keys);\n\
         \x20 if (m->values) free(m->values);\n\
         \x20 if (m->occ) free(m->occ);\n\
         \x20 m->keys = 0; m->values = 0; m->occ = 0;\n\
         \x20 m->len = 0; m->capacity = 0; m->tombstones = 0;\n\
         }}\n\
         static INTENT_UNUSED uint64_t {prefix}__hash_key(int64_t k) {{\n\
         \x20 uint64_t h = 0xcbf29ce484222325ULL;\n\
         \x20 uint64_t u = (uint64_t)k;\n\
         \x20 for (int i = 0; i < 8; i++) {{\n\
         \x20   h ^= (u >> (i * 8)) & 0xffULL;\n\
         \x20   h *= 0x100000001b3ULL;\n\
         \x20 }}\n\
         \x20 return h;\n\
         }}\n\
         static INTENT_UNUSED void {prefix}__insert_raw({prefix}* m, int64_t k, {v_ctype} v) {{\n\
         \x20 uint64_t mask = m->capacity - 1;\n\
         \x20 uint64_t i = {prefix}__hash_key(k) & mask;\n\
         \x20 while (m->occ[i] == 1) {{\n\
         \x20   i = (i + 1) & mask;\n\
         \x20 }}\n\
         \x20 m->keys[i] = k; m->values[i] = v; m->occ[i] = 1; m->len++;\n\
         }}\n\
         static INTENT_UNUSED void {prefix}__grow({prefix}* m) {{\n\
         \x20 uint64_t old_cap = m->capacity;\n\
         \x20 int64_t* old_keys = m->keys;\n\
         \x20 {v_ctype}* old_values = m->values;\n\
         \x20 uint8_t* old_occ = m->occ;\n\
         \x20 uint64_t new_cap = old_cap == 0 ? 8 : old_cap * 2;\n\
         \x20 m->keys = (int64_t*)malloc(new_cap * sizeof(int64_t));\n\
         \x20 m->values = ({v_ctype}*)malloc(new_cap * sizeof({v_ctype}));\n\
         \x20 m->occ = (uint8_t*)calloc(new_cap, 1);\n\
         \x20 if (!m->keys || !m->values || !m->occ) abort();\n\
         \x20 m->len = 0;\n\
         \x20 m->capacity = new_cap;\n\
         \x20 m->tombstones = 0;\n\
         \x20 for (uint64_t i = 0; i < old_cap; i++) {{\n\
         \x20   if (old_occ[i] == 1) {prefix}__insert_raw(m, old_keys[i], old_values[i]);\n\
         \x20 }}\n\
         \x20 if (old_keys) free(old_keys);\n\
         \x20 if (old_values) free(old_values);\n\
         \x20 if (old_occ) free(old_occ);\n\
         }}\n\
         static INTENT_UNUSED bool {prefix}_contains_key(const {prefix}* m, int64_t k) {{\n\
         \x20 if (m->capacity == 0) return false;\n\
         \x20 uint64_t mask = m->capacity - 1;\n\
         \x20 uint64_t i = {prefix}__hash_key(k) & mask;\n\
         \x20 while (m->occ[i] != 0) {{\n\
         \x20   if (m->occ[i] == 1 && m->keys[i] == k) return true;\n\
         \x20   i = (i + 1) & mask;\n\
         \x20 }}\n\
         \x20 return false;\n\
         }}\n\
         static INTENT_UNUSED int64_t {prefix}_len(const {prefix}* m) {{\n\
         \x20 return (int64_t)m->len;\n\
         }}\n\
         static INTENT_UNUSED int64_t {prefix}_clear({prefix}* m) {{\n\
         \x20 int64_t prior = (int64_t)m->len;\n\
         \x20 if (m->keys) free(m->keys);\n\
         \x20 if (m->values) free(m->values);\n\
         \x20 if (m->occ) free(m->occ);\n\
         \x20 m->keys = (int64_t*)0;\n\
         \x20 m->values = ({v_ctype}*)0;\n\
         \x20 m->occ = (uint8_t*)0;\n\
         \x20 m->len = 0;\n\
         \x20 m->capacity = 0;\n\
         \x20 m->tombstones = 0;\n\
         \x20 return prior;\n\
         }}\n",
        v_ctype = v_ctype,
        prefix = prefix,
    ));
    if has_option_v {
        out.push_str(&format!(
            "static INTENT_UNUSED {opt_v} {prefix}_get(const {prefix}* m, int64_t k) {{\n\
             \x20 {opt_v} r;\n\
             \x20 if (m->capacity == 0) {{ r.tag = 1; r.payload = 0; return r; }}\n\
             \x20 uint64_t mask = m->capacity - 1;\n\
             \x20 uint64_t i = {prefix}__hash_key(k) & mask;\n\
             \x20 while (m->occ[i] != 0) {{\n\
             \x20   if (m->occ[i] == 1 && m->keys[i] == k) {{ r.tag = 0; r.payload = m->values[i]; return r; }}\n\
             \x20   i = (i + 1) & mask;\n\
             \x20 }}\n\
             \x20 r.tag = 1; r.payload = 0; return r;\n\
             }}\n\
             static INTENT_UNUSED {opt_v} {prefix}_insert({prefix}* m, int64_t k, {v_ctype} v) {{\n\
             \x20 if (m->capacity == 0 || ((m->len + m->tombstones) * 2) >= m->capacity) {prefix}__grow(m);\n\
             \x20 {opt_v} r;\n\
             \x20 uint64_t mask = m->capacity - 1;\n\
             \x20 uint64_t i = {prefix}__hash_key(k) & mask;\n\
             \x20 int64_t first_tomb = -1;\n\
             \x20 while (m->occ[i] != 0) {{\n\
             \x20   if (m->occ[i] == 1 && m->keys[i] == k) {{\n\
             \x20     r.tag = 0; r.payload = m->values[i];\n\
             \x20     m->values[i] = v;\n\
             \x20     return r;\n\
             \x20   }}\n\
             \x20   if (m->occ[i] == 2 && first_tomb == -1) first_tomb = (int64_t)i;\n\
             \x20   i = (i + 1) & mask;\n\
             \x20 }}\n\
             \x20 if (first_tomb != -1) {{\n\
             \x20   uint64_t slot = (uint64_t)first_tomb;\n\
             \x20   m->keys[slot] = k; m->values[slot] = v; m->occ[slot] = 1;\n\
             \x20   m->len++; m->tombstones--;\n\
             \x20 }} else {{\n\
             \x20   m->keys[i] = k; m->values[i] = v; m->occ[i] = 1; m->len++;\n\
             \x20 }}\n\
             \x20 r.tag = 1; r.payload = 0; return r;\n\
             }}\n\
             static INTENT_UNUSED {opt_v} {prefix}_remove({prefix}* m, int64_t k) {{\n\
             \x20 {opt_v} r;\n\
             \x20 if (m->capacity == 0) {{ r.tag = 1; r.payload = 0; return r; }}\n\
             \x20 uint64_t mask = m->capacity - 1;\n\
             \x20 uint64_t i = {prefix}__hash_key(k) & mask;\n\
             \x20 while (m->occ[i] != 0) {{\n\
             \x20   if (m->occ[i] == 1 && m->keys[i] == k) {{\n\
             \x20     r.tag = 0; r.payload = m->values[i];\n\
             \x20     m->occ[i] = 2;\n\
             \x20     m->len--;\n\
             \x20     m->tombstones++;\n\
             \x20     return r;\n\
             \x20   }}\n\
             \x20   i = (i + 1) & mask;\n\
             \x20 }}\n\
             \x20 r.tag = 1; r.payload = 0; return r;\n\
             }}\n\n",
            v_ctype = v_ctype,
            prefix = prefix,
            opt_v = opt_v,
        ));
    }
}

/// ARC 1.7 — parameterized HashMap bundle for K = user struct,
/// V = any scalar. The struct K must implement both `Hash` and
/// `Eq` (checker enforces this). The bundle delegates:
///   - hash function → user's `fn_<K>__hash(K)` returning i64
///   - key equality → user's `fn_<K>__eq(K, K)` returning bool
///
/// `k_name`     — struct name, e.g. "Score"
/// `k_ctype`    — full C-leaf type, e.g. "Struct_Score"
/// `v_tag`      — V's C-leaf tag, e.g. "int64_t"
/// `v_ctype`    — V's full C type spelling (same as v_tag for
///                scalars)
/// `option_v_mangle` — Option<V> enum suffix, e.g. "i64"
/// `has_option_v` — gates get/insert/remove
fn emit_intent_hashmap_struct_pair_c_body(
    out: &mut String,
    k_name: &str,
    k_ctype: &str,
    v_tag: &str,
    v_ctype: &str,
    option_v_mangle: &str,
    has_option_v: bool,
) {
    let prefix = format!("intent_hashmap_Struct_{}_{}", k_name, v_tag);
    let opt_v = format!("Enum_Option__{}", option_v_mangle);
    // User-defined interface methods are emitted as
    // `fn_<TypeName>_<method>` by the existing interface
    // codegen (single underscore between type + method).
    let hash_fn = format!("fn_{}_hash", k_name);
    let eq_fn = format!("fn_{}_eq", k_name);
    out.push_str(&format!(
        "typedef struct {{ {k_ctype}* keys; {v_ctype}* values; uint8_t* occ; uint64_t len; uint64_t capacity; uint64_t tombstones; }} {prefix};\n\
         /* Declare the user-defined hash + eq fns; they may be\n\
          * defined later in the same translation unit. */\n\
         static int64_t {hash_fn}({k_ctype} self);\n\
         static bool {eq_fn}({k_ctype} self, {k_ctype} other);\n\
         static INTENT_UNUSED {prefix} {prefix}_new(void) {{\n\
         \x20 {prefix} m;\n\
         \x20 m.keys = ({k_ctype}*)0; m.values = ({v_ctype}*)0; m.occ = (uint8_t*)0;\n\
         \x20 m.len = 0; m.capacity = 0; m.tombstones = 0;\n\
         \x20 return m;\n\
         }}\n\
         static INTENT_UNUSED void {prefix}_drop({prefix}* m) {{\n\
         \x20 if (m->keys) free(m->keys);\n\
         \x20 if (m->values) free(m->values);\n\
         \x20 if (m->occ) free(m->occ);\n\
         \x20 m->keys = 0; m->values = 0; m->occ = 0;\n\
         \x20 m->len = 0; m->capacity = 0; m->tombstones = 0;\n\
         }}\n\
         static INTENT_UNUSED uint64_t {prefix}__hash_key({k_ctype} k) {{\n\
         \x20 int64_t raw = {hash_fn}(k);\n\
         \x20 /* FNV-1a over the raw i64 hash so struct hash\n\
          *    impls that return e.g. a single field's value\n\
          *    still distribute across the table. */\n\
         \x20 uint64_t h = 0xcbf29ce484222325ULL;\n\
         \x20 uint64_t u = (uint64_t)raw;\n\
         \x20 for (int i = 0; i < 8; i++) {{\n\
         \x20   h ^= (u >> (i * 8)) & 0xffULL;\n\
         \x20   h *= 0x100000001b3ULL;\n\
         \x20 }}\n\
         \x20 return h;\n\
         }}\n\
         static INTENT_UNUSED void {prefix}__insert_raw({prefix}* m, {k_ctype} k, {v_ctype} v) {{\n\
         \x20 uint64_t mask = m->capacity - 1;\n\
         \x20 uint64_t i = {prefix}__hash_key(k) & mask;\n\
         \x20 while (m->occ[i] == 1) {{\n\
         \x20   i = (i + 1) & mask;\n\
         \x20 }}\n\
         \x20 m->keys[i] = k; m->values[i] = v; m->occ[i] = 1; m->len++;\n\
         }}\n\
         static INTENT_UNUSED void {prefix}__grow({prefix}* m) {{\n\
         \x20 uint64_t old_cap = m->capacity;\n\
         \x20 {k_ctype}* old_keys = m->keys;\n\
         \x20 {v_ctype}* old_values = m->values;\n\
         \x20 uint8_t* old_occ = m->occ;\n\
         \x20 uint64_t new_cap = old_cap == 0 ? 8 : old_cap * 2;\n\
         \x20 m->keys = ({k_ctype}*)malloc(new_cap * sizeof({k_ctype}));\n\
         \x20 m->values = ({v_ctype}*)malloc(new_cap * sizeof({v_ctype}));\n\
         \x20 m->occ = (uint8_t*)calloc(new_cap, 1);\n\
         \x20 if (!m->keys || !m->values || !m->occ) abort();\n\
         \x20 m->len = 0;\n\
         \x20 m->capacity = new_cap;\n\
         \x20 m->tombstones = 0;\n\
         \x20 for (uint64_t i = 0; i < old_cap; i++) {{\n\
         \x20   if (old_occ[i] == 1) {prefix}__insert_raw(m, old_keys[i], old_values[i]);\n\
         \x20 }}\n\
         \x20 if (old_keys) free(old_keys);\n\
         \x20 if (old_values) free(old_values);\n\
         \x20 if (old_occ) free(old_occ);\n\
         }}\n\
         static INTENT_UNUSED bool {prefix}_contains_key(const {prefix}* m, {k_ctype} k) {{\n\
         \x20 if (m->capacity == 0) return false;\n\
         \x20 uint64_t mask = m->capacity - 1;\n\
         \x20 uint64_t i = {prefix}__hash_key(k) & mask;\n\
         \x20 while (m->occ[i] != 0) {{\n\
         \x20   if (m->occ[i] == 1 && {eq_fn}(m->keys[i], k)) return true;\n\
         \x20   i = (i + 1) & mask;\n\
         \x20 }}\n\
         \x20 return false;\n\
         }}\n\
         static INTENT_UNUSED int64_t {prefix}_len(const {prefix}* m) {{\n\
         \x20 return (int64_t)m->len;\n\
         }}\n\
         static INTENT_UNUSED int64_t {prefix}_clear({prefix}* m) {{\n\
         \x20 int64_t prior = (int64_t)m->len;\n\
         \x20 if (m->keys) free(m->keys);\n\
         \x20 if (m->values) free(m->values);\n\
         \x20 if (m->occ) free(m->occ);\n\
         \x20 m->keys = ({k_ctype}*)0;\n\
         \x20 m->values = ({v_ctype}*)0;\n\
         \x20 m->occ = (uint8_t*)0;\n\
         \x20 m->len = 0;\n\
         \x20 m->capacity = 0;\n\
         \x20 m->tombstones = 0;\n\
         \x20 return prior;\n\
         }}\n",
        k_ctype = k_ctype, v_ctype = v_ctype,
        prefix = prefix, hash_fn = hash_fn, eq_fn = eq_fn,
    ));
    if has_option_v {
        out.push_str(&format!(
            "static INTENT_UNUSED {opt_v} {prefix}_get(const {prefix}* m, {k_ctype} k) {{\n\
             \x20 {opt_v} r;\n\
             \x20 if (m->capacity == 0) {{ r.tag = 1; r.payload = 0; return r; }}\n\
             \x20 uint64_t mask = m->capacity - 1;\n\
             \x20 uint64_t i = {prefix}__hash_key(k) & mask;\n\
             \x20 while (m->occ[i] != 0) {{\n\
             \x20   if (m->occ[i] == 1 && {eq_fn}(m->keys[i], k)) {{ r.tag = 0; r.payload = m->values[i]; return r; }}\n\
             \x20   i = (i + 1) & mask;\n\
             \x20 }}\n\
             \x20 r.tag = 1; r.payload = 0; return r;\n\
             }}\n\
             static INTENT_UNUSED {opt_v} {prefix}_insert({prefix}* m, {k_ctype} k, {v_ctype} v) {{\n\
             \x20 if (m->capacity == 0 || ((m->len + m->tombstones) * 2) >= m->capacity) {prefix}__grow(m);\n\
             \x20 {opt_v} r;\n\
             \x20 uint64_t mask = m->capacity - 1;\n\
             \x20 uint64_t i = {prefix}__hash_key(k) & mask;\n\
             \x20 int64_t first_tomb = -1;\n\
             \x20 while (m->occ[i] != 0) {{\n\
             \x20   if (m->occ[i] == 1 && {eq_fn}(m->keys[i], k)) {{\n\
             \x20     r.tag = 0; r.payload = m->values[i];\n\
             \x20     m->values[i] = v;\n\
             \x20     return r;\n\
             \x20   }}\n\
             \x20   if (m->occ[i] == 2 && first_tomb == -1) first_tomb = (int64_t)i;\n\
             \x20   i = (i + 1) & mask;\n\
             \x20 }}\n\
             \x20 if (first_tomb != -1) {{\n\
             \x20   uint64_t slot = (uint64_t)first_tomb;\n\
             \x20   m->keys[slot] = k; m->values[slot] = v; m->occ[slot] = 1;\n\
             \x20   m->len++; m->tombstones--;\n\
             \x20 }} else {{\n\
             \x20   m->keys[i] = k; m->values[i] = v; m->occ[i] = 1; m->len++;\n\
             \x20 }}\n\
             \x20 r.tag = 1; r.payload = 0; return r;\n\
             }}\n\
             static INTENT_UNUSED {opt_v} {prefix}_remove({prefix}* m, {k_ctype} k) {{\n\
             \x20 {opt_v} r;\n\
             \x20 if (m->capacity == 0) {{ r.tag = 1; r.payload = 0; return r; }}\n\
             \x20 uint64_t mask = m->capacity - 1;\n\
             \x20 uint64_t i = {prefix}__hash_key(k) & mask;\n\
             \x20 while (m->occ[i] != 0) {{\n\
             \x20   if (m->occ[i] == 1 && {eq_fn}(m->keys[i], k)) {{\n\
             \x20     r.tag = 0; r.payload = m->values[i];\n\
             \x20     m->occ[i] = 2;\n\
             \x20     m->len--;\n\
             \x20     m->tombstones++;\n\
             \x20     return r;\n\
             \x20   }}\n\
             \x20   i = (i + 1) & mask;\n\
             \x20 }}\n\
             \x20 r.tag = 1; r.payload = 0; return r;\n\
             }}\n\n",
            k_ctype = k_ctype, v_ctype = v_ctype,
            prefix = prefix, eq_fn = eq_fn, opt_v = opt_v,
        ));
    }
}

/// ARC 4.5 — `HashMap<f64, V>` for V scalar. Built-in `==`
/// equality (with NaN caveat: NaN != NaN, so NaN keys are
/// effectively unrecoverable). Hash function reinterprets the
/// f64 bits as a uint64 then FNV-1a's them — same byte
/// distribution as the i64 path.
fn emit_intent_hashmap_pair_c_body_f64k(
    out: &mut String,
    v_tag: &str,
    v_ctype: &str,
    option_v_mangle: &str,
    has_option_v: bool,
) {
    let prefix = format!("intent_hashmap_double_{}", v_tag);
    let opt_v = format!("Enum_Option__{}", option_v_mangle);
    out.push_str(&format!(
        "typedef struct {{ double* keys; {v_ctype}* values; uint8_t* occ; uint64_t len; uint64_t capacity; uint64_t tombstones; }} {prefix};\n\
         static INTENT_UNUSED {prefix} {prefix}_new(void) {{\n\
         \x20 {prefix} m;\n\
         \x20 m.keys = (double*)0; m.values = ({v_ctype}*)0; m.occ = (uint8_t*)0;\n\
         \x20 m.len = 0; m.capacity = 0; m.tombstones = 0;\n\
         \x20 return m;\n\
         }}\n\
         static INTENT_UNUSED void {prefix}_drop({prefix}* m) {{\n\
         \x20 if (m->keys) free(m->keys);\n\
         \x20 if (m->values) free(m->values);\n\
         \x20 if (m->occ) free(m->occ);\n\
         \x20 m->keys = 0; m->values = 0; m->occ = 0;\n\
         \x20 m->len = 0; m->capacity = 0; m->tombstones = 0;\n\
         }}\n\
         static INTENT_UNUSED uint64_t {prefix}__hash_key(double k) {{\n\
         \x20 uint64_t bits = 0;\n\
         \x20 memcpy(&bits, &k, 8);\n\
         \x20 uint64_t h = 0xcbf29ce484222325ULL;\n\
         \x20 for (int i = 0; i < 8; i++) {{\n\
         \x20   h ^= (bits >> (i * 8)) & 0xffULL;\n\
         \x20   h *= 0x100000001b3ULL;\n\
         \x20 }}\n\
         \x20 return h;\n\
         }}\n\
         static INTENT_UNUSED void {prefix}__insert_raw({prefix}* m, double k, {v_ctype} v) {{\n\
         \x20 uint64_t mask = m->capacity - 1;\n\
         \x20 uint64_t i = {prefix}__hash_key(k) & mask;\n\
         \x20 while (m->occ[i] == 1) {{\n\
         \x20   i = (i + 1) & mask;\n\
         \x20 }}\n\
         \x20 m->keys[i] = k; m->values[i] = v; m->occ[i] = 1; m->len++;\n\
         }}\n\
         static INTENT_UNUSED void {prefix}__grow({prefix}* m) {{\n\
         \x20 uint64_t old_cap = m->capacity;\n\
         \x20 double* old_keys = m->keys;\n\
         \x20 {v_ctype}* old_values = m->values;\n\
         \x20 uint8_t* old_occ = m->occ;\n\
         \x20 uint64_t new_cap = old_cap == 0 ? 8 : old_cap * 2;\n\
         \x20 m->keys = (double*)malloc(new_cap * sizeof(double));\n\
         \x20 m->values = ({v_ctype}*)malloc(new_cap * sizeof({v_ctype}));\n\
         \x20 m->occ = (uint8_t*)calloc(new_cap, 1);\n\
         \x20 if (!m->keys || !m->values || !m->occ) abort();\n\
         \x20 m->len = 0;\n\
         \x20 m->capacity = new_cap;\n\
         \x20 m->tombstones = 0;\n\
         \x20 for (uint64_t i = 0; i < old_cap; i++) {{\n\
         \x20   if (old_occ[i] == 1) {prefix}__insert_raw(m, old_keys[i], old_values[i]);\n\
         \x20 }}\n\
         \x20 if (old_keys) free(old_keys);\n\
         \x20 if (old_values) free(old_values);\n\
         \x20 if (old_occ) free(old_occ);\n\
         }}\n\
         static INTENT_UNUSED bool {prefix}_contains_key(const {prefix}* m, double k) {{\n\
         \x20 if (m->capacity == 0) return false;\n\
         \x20 uint64_t mask = m->capacity - 1;\n\
         \x20 uint64_t i = {prefix}__hash_key(k) & mask;\n\
         \x20 while (m->occ[i] != 0) {{\n\
         \x20   if (m->occ[i] == 1 && m->keys[i] == k) return true;\n\
         \x20   i = (i + 1) & mask;\n\
         \x20 }}\n\
         \x20 return false;\n\
         }}\n\
         static INTENT_UNUSED int64_t {prefix}_len(const {prefix}* m) {{\n\
         \x20 return (int64_t)m->len;\n\
         }}\n\
         static INTENT_UNUSED int64_t {prefix}_clear({prefix}* m) {{\n\
         \x20 int64_t prior = (int64_t)m->len;\n\
         \x20 if (m->keys) free(m->keys);\n\
         \x20 if (m->values) free(m->values);\n\
         \x20 if (m->occ) free(m->occ);\n\
         \x20 m->keys = (double*)0;\n\
         \x20 m->values = ({v_ctype}*)0;\n\
         \x20 m->occ = (uint8_t*)0;\n\
         \x20 m->len = 0;\n\
         \x20 m->capacity = 0;\n\
         \x20 m->tombstones = 0;\n\
         \x20 return prior;\n\
         }}\n",
        v_ctype = v_ctype, prefix = prefix,
    ));
    if has_option_v {
        out.push_str(&format!(
            "static INTENT_UNUSED {opt_v} {prefix}_get(const {prefix}* m, double k) {{\n\
             \x20 {opt_v} r;\n\
             \x20 if (m->capacity == 0) {{ r.tag = 1; r.payload = 0; return r; }}\n\
             \x20 uint64_t mask = m->capacity - 1;\n\
             \x20 uint64_t i = {prefix}__hash_key(k) & mask;\n\
             \x20 while (m->occ[i] != 0) {{\n\
             \x20   if (m->occ[i] == 1 && m->keys[i] == k) {{ r.tag = 0; r.payload = m->values[i]; return r; }}\n\
             \x20   i = (i + 1) & mask;\n\
             \x20 }}\n\
             \x20 r.tag = 1; r.payload = 0; return r;\n\
             }}\n\
             static INTENT_UNUSED {opt_v} {prefix}_insert({prefix}* m, double k, {v_ctype} v) {{\n\
             \x20 if (m->capacity == 0 || ((m->len + m->tombstones) * 2) >= m->capacity) {prefix}__grow(m);\n\
             \x20 {opt_v} r;\n\
             \x20 uint64_t mask = m->capacity - 1;\n\
             \x20 uint64_t i = {prefix}__hash_key(k) & mask;\n\
             \x20 int64_t first_tomb = -1;\n\
             \x20 while (m->occ[i] != 0) {{\n\
             \x20   if (m->occ[i] == 1 && m->keys[i] == k) {{\n\
             \x20     r.tag = 0; r.payload = m->values[i];\n\
             \x20     m->values[i] = v;\n\
             \x20     return r;\n\
             \x20   }}\n\
             \x20   if (m->occ[i] == 2 && first_tomb == -1) first_tomb = (int64_t)i;\n\
             \x20   i = (i + 1) & mask;\n\
             \x20 }}\n\
             \x20 if (first_tomb != -1) {{\n\
             \x20   uint64_t slot = (uint64_t)first_tomb;\n\
             \x20   m->keys[slot] = k; m->values[slot] = v; m->occ[slot] = 1;\n\
             \x20   m->len++; m->tombstones--;\n\
             \x20 }} else {{\n\
             \x20   m->keys[i] = k; m->values[i] = v; m->occ[i] = 1; m->len++;\n\
             \x20 }}\n\
             \x20 r.tag = 1; r.payload = 0; return r;\n\
             }}\n\
             static INTENT_UNUSED {opt_v} {prefix}_remove({prefix}* m, double k) {{\n\
             \x20 {opt_v} r;\n\
             \x20 if (m->capacity == 0) {{ r.tag = 1; r.payload = 0; return r; }}\n\
             \x20 uint64_t mask = m->capacity - 1;\n\
             \x20 uint64_t i = {prefix}__hash_key(k) & mask;\n\
             \x20 while (m->occ[i] != 0) {{\n\
             \x20   if (m->occ[i] == 1 && m->keys[i] == k) {{\n\
             \x20     r.tag = 0; r.payload = m->values[i];\n\
             \x20     m->occ[i] = 2;\n\
             \x20     m->len--;\n\
             \x20     m->tombstones++;\n\
             \x20     return r;\n\
             \x20   }}\n\
             \x20   i = (i + 1) & mask;\n\
             \x20 }}\n\
             \x20 r.tag = 1; r.payload = 0; return r;\n\
             }}\n\n",
            v_ctype = v_ctype, prefix = prefix, opt_v = opt_v,
        ));
    }
}

/// ARC 4.1 — `HashMap<OwnedStr, V>` for V scalar. The map
/// owns each key pointer (via internal `strdup`) — `_drop` /
/// `_clear` walk all occupied slots and `free()` the stored
/// copies. The bundle clones internally because the language
/// affine system doesn't yet suppress local drops for OwnedStr
/// args moved into builtins; cloning makes the user-visible
/// "drop the local after the call" semantics safe (no
/// double-free), and matches Rust's `m.insert(key.clone(), v)`
/// ergonomics. FNV-1a hash over the bytes; equality via
/// `strcmp`.
fn emit_intent_hashmap_pair_c_body_strk(
    out: &mut String,
    v_tag: &str,
    v_ctype: &str,
    option_v_mangle: &str,
    has_option_v: bool,
) {
    let prefix = format!("intent_hashmap_owned_str_{}", v_tag);
    let opt_v = format!("Enum_Option__{}", option_v_mangle);
    out.push_str(&format!(
        "typedef struct {{ char** keys; {v_ctype}* values; uint8_t* occ; uint64_t len; uint64_t capacity; uint64_t tombstones; }} {prefix};\n\
         static INTENT_UNUSED {prefix} {prefix}_new(void) {{\n\
         \x20 {prefix} m;\n\
         \x20 m.keys = (char**)0; m.values = ({v_ctype}*)0; m.occ = (uint8_t*)0;\n\
         \x20 m.len = 0; m.capacity = 0; m.tombstones = 0;\n\
         \x20 return m;\n\
         }}\n\
         static INTENT_UNUSED void {prefix}_drop({prefix}* m) {{\n\
         \x20 if (m->keys) {{\n\
         \x20   for (uint64_t i = 0; i < m->capacity; i++) {{\n\
         \x20     if (m->occ[i] == 1 && m->keys[i]) free(m->keys[i]);\n\
         \x20   }}\n\
         \x20   free(m->keys);\n\
         \x20 }}\n\
         \x20 if (m->values) free(m->values);\n\
         \x20 if (m->occ) free(m->occ);\n\
         \x20 m->keys = 0; m->values = 0; m->occ = 0;\n\
         \x20 m->len = 0; m->capacity = 0; m->tombstones = 0;\n\
         }}\n\
         static INTENT_UNUSED uint64_t {prefix}__hash_key(const char* k) {{\n\
         \x20 uint64_t h = 0xcbf29ce484222325ULL;\n\
         \x20 for (const char* p = k; *p; p++) {{\n\
         \x20   h ^= (uint64_t)(unsigned char)(*p);\n\
         \x20   h *= 0x100000001b3ULL;\n\
         \x20 }}\n\
         \x20 return h;\n\
         }}\n\
         static INTENT_UNUSED void {prefix}__insert_owned_raw({prefix}* m, char* k_owned, {v_ctype} v) {{\n\
         \x20 uint64_t mask = m->capacity - 1;\n\
         \x20 uint64_t i = {prefix}__hash_key(k_owned) & mask;\n\
         \x20 while (m->occ[i] == 1) {{\n\
         \x20   i = (i + 1) & mask;\n\
         \x20 }}\n\
         \x20 m->keys[i] = k_owned; m->values[i] = v; m->occ[i] = 1; m->len++;\n\
         }}\n\
         static INTENT_UNUSED void {prefix}__grow({prefix}* m) {{\n\
         \x20 uint64_t old_cap = m->capacity;\n\
         \x20 char** old_keys = m->keys;\n\
         \x20 {v_ctype}* old_values = m->values;\n\
         \x20 uint8_t* old_occ = m->occ;\n\
         \x20 uint64_t new_cap = old_cap == 0 ? 8 : old_cap * 2;\n\
         \x20 m->keys = (char**)malloc(new_cap * sizeof(char*));\n\
         \x20 m->values = ({v_ctype}*)malloc(new_cap * sizeof({v_ctype}));\n\
         \x20 m->occ = (uint8_t*)calloc(new_cap, 1);\n\
         \x20 if (!m->keys || !m->values || !m->occ) abort();\n\
         \x20 m->len = 0;\n\
         \x20 m->capacity = new_cap;\n\
         \x20 m->tombstones = 0;\n\
         \x20 for (uint64_t i = 0; i < old_cap; i++) {{\n\
         \x20   if (old_occ[i] == 1) {prefix}__insert_owned_raw(m, old_keys[i], old_values[i]);\n\
         \x20 }}\n\
         \x20 if (old_keys) free(old_keys);\n\
         \x20 if (old_values) free(old_values);\n\
         \x20 if (old_occ) free(old_occ);\n\
         }}\n\
         static INTENT_UNUSED bool {prefix}_contains_key(const {prefix}* m, const char* k) {{\n\
         \x20 if (m->capacity == 0) return false;\n\
         \x20 uint64_t mask = m->capacity - 1;\n\
         \x20 uint64_t i = {prefix}__hash_key(k) & mask;\n\
         \x20 while (m->occ[i] != 0) {{\n\
         \x20   if (m->occ[i] == 1 && strcmp(m->keys[i], k) == 0) return true;\n\
         \x20   i = (i + 1) & mask;\n\
         \x20 }}\n\
         \x20 return false;\n\
         }}\n\
         static INTENT_UNUSED int64_t {prefix}_len(const {prefix}* m) {{\n\
         \x20 return (int64_t)m->len;\n\
         }}\n\
         static INTENT_UNUSED int64_t {prefix}_clear({prefix}* m) {{\n\
         \x20 int64_t prior = (int64_t)m->len;\n\
         \x20 if (m->keys) {{\n\
         \x20   for (uint64_t i = 0; i < m->capacity; i++) {{\n\
         \x20     if (m->occ[i] == 1 && m->keys[i]) free(m->keys[i]);\n\
         \x20   }}\n\
         \x20   free(m->keys);\n\
         \x20 }}\n\
         \x20 if (m->values) free(m->values);\n\
         \x20 if (m->occ) free(m->occ);\n\
         \x20 m->keys = (char**)0;\n\
         \x20 m->values = ({v_ctype}*)0;\n\
         \x20 m->occ = (uint8_t*)0;\n\
         \x20 m->len = 0;\n\
         \x20 m->capacity = 0;\n\
         \x20 m->tombstones = 0;\n\
         \x20 return prior;\n\
         }}\n",
        v_ctype = v_ctype, prefix = prefix,
    ));
    if has_option_v {
        // contains_key style probe for get; insert/remove free
        // their respective key strings appropriately.
        out.push_str(&format!(
            "static INTENT_UNUSED {opt_v} {prefix}_get(const {prefix}* m, const char* k) {{\n\
             \x20 {opt_v} r;\n\
             \x20 if (m->capacity == 0) {{ r.tag = 1; r.payload = 0; return r; }}\n\
             \x20 uint64_t mask = m->capacity - 1;\n\
             \x20 uint64_t i = {prefix}__hash_key(k) & mask;\n\
             \x20 while (m->occ[i] != 0) {{\n\
             \x20   if (m->occ[i] == 1 && strcmp(m->keys[i], k) == 0) {{ r.tag = 0; r.payload = m->values[i]; return r; }}\n\
             \x20   i = (i + 1) & mask;\n\
             \x20 }}\n\
             \x20 r.tag = 1; r.payload = 0; return r;\n\
             }}\n\
             static INTENT_UNUSED {opt_v} {prefix}_insert({prefix}* m, const char* k, {v_ctype} v) {{\n\
             \x20 if (m->capacity == 0 || ((m->len + m->tombstones) * 2) >= m->capacity) {prefix}__grow(m);\n\
             \x20 {opt_v} r;\n\
             \x20 uint64_t mask = m->capacity - 1;\n\
             \x20 uint64_t i = {prefix}__hash_key(k) & mask;\n\
             \x20 int64_t first_tomb = -1;\n\
             \x20 while (m->occ[i] != 0) {{\n\
             \x20   if (m->occ[i] == 1 && strcmp(m->keys[i], k) == 0) {{\n\
             \x20     /* Duplicate key — keep the existing key copy,\n\
             \x20      * swap in the new value. Caller still owns k. */\n\
             \x20     r.tag = 0; r.payload = m->values[i];\n\
             \x20     m->values[i] = v;\n\
             \x20     return r;\n\
             \x20   }}\n\
             \x20   if (m->occ[i] == 2 && first_tomb == -1) first_tomb = (int64_t)i;\n\
             \x20   i = (i + 1) & mask;\n\
             \x20 }}\n\
             \x20 /* Clone the caller's key — affine system doesn't\n\
             \x20  * yet suppress the local drop for OwnedStr moved\n\
             \x20  * into builtin args, so cloning makes the user's\n\
             \x20  * scope-exit free safe (no double-free). Inline\n\
             \x20  * the strdup (POSIX-only otherwise) via malloc +\n\
             \x20  * memcpy so the bundle is C11-pure. */\n\
             \x20 size_t k_len = strlen(k);\n\
             \x20 char* k_owned = (char*)malloc(k_len + 1);\n\
             \x20 if (!k_owned) abort();\n\
             \x20 memcpy(k_owned, k, k_len + 1);\n\
             \x20 if (first_tomb != -1) {{\n\
             \x20   uint64_t slot = (uint64_t)first_tomb;\n\
             \x20   m->keys[slot] = k_owned; m->values[slot] = v; m->occ[slot] = 1;\n\
             \x20   m->len++; m->tombstones--;\n\
             \x20 }} else {{\n\
             \x20   m->keys[i] = k_owned; m->values[i] = v; m->occ[i] = 1; m->len++;\n\
             \x20 }}\n\
             \x20 r.tag = 1; r.payload = 0; return r;\n\
             }}\n\
             static INTENT_UNUSED {opt_v} {prefix}_remove({prefix}* m, const char* k) {{\n\
             \x20 {opt_v} r;\n\
             \x20 if (m->capacity == 0) {{ r.tag = 1; r.payload = 0; return r; }}\n\
             \x20 uint64_t mask = m->capacity - 1;\n\
             \x20 uint64_t i = {prefix}__hash_key(k) & mask;\n\
             \x20 while (m->occ[i] != 0) {{\n\
             \x20   if (m->occ[i] == 1 && strcmp(m->keys[i], k) == 0) {{\n\
             \x20     r.tag = 0; r.payload = m->values[i];\n\
             \x20     free(m->keys[i]);\n\
             \x20     m->keys[i] = 0;\n\
             \x20     m->occ[i] = 2;\n\
             \x20     m->len--;\n\
             \x20     m->tombstones++;\n\
             \x20     return r;\n\
             \x20   }}\n\
             \x20   i = (i + 1) & mask;\n\
             \x20 }}\n\
             \x20 r.tag = 1; r.payload = 0; return r;\n\
             }}\n\n",
            v_ctype = v_ctype, prefix = prefix, opt_v = opt_v,
        ));
    }
}

/// ARC 4.4 — `HashMap<(i64, i64, …, i64), V>` for V scalar.
/// K elements are i64 (Copy) so no drop walk. Hash via
/// hash_combine of element FNV-1a hashes; equality is pairwise
/// field compare. Uses the existing `intent_tuple_<…>` struct
/// typedef as the key cell type (already emitted by the tuple
/// preamble whenever the program uses a tuple of that shape).
fn emit_intent_hashmap_pair_c_body_tuple_i64k(
    out: &mut String,
    arity: usize,
    v_tag: &str,
    v_ctype: &str,
    option_v_mangle: &str,
    has_option_v: bool,
) {
    let prefix = format!("intent_hashmap_tup_{}_i64_{}", arity, v_tag);
    // Element tags repeated: `intent_tuple_int64_t_int64_t…`.
    let tuple_struct = {
        let parts: Vec<&str> = (0..arity).map(|_| "int64_t").collect();
        format!("intent_tuple_{}", parts.join("_"))
    };
    let opt_v = format!("Enum_Option__{}", option_v_mangle);
    // Build the element-wise hash_combine: start with FNV
    // offset basis; for each field, FNV-1a 8 bytes of that
    // field into the running hash. Same byte distribution as
    // the i64-K bundle, extended per-element.
    let mut hash_body = String::new();
    for i in 0..arity {
        hash_body.push_str(&format!(
            "\x20 {{\n\
             \x20   uint64_t u{i} = (uint64_t)k._{i};\n\
             \x20   for (int b = 0; b < 8; b++) {{\n\
             \x20     h ^= (u{i} >> (b * 8)) & 0xffULL;\n\
             \x20     h *= 0x100000001b3ULL;\n\
             \x20   }}\n\
             \x20 }}\n",
            i = i
        ));
    }
    // Build the element-wise equality: `k1._0 == k2._0 && k1._1 == k2._1`.
    let eq_parts: Vec<String> = (0..arity)
        .map(|i| format!("a._{i} == b._{i}", i = i))
        .collect();
    let eq_expr = if eq_parts.is_empty() { "1".to_string() } else { eq_parts.join(" && ") };
    out.push_str(&format!(
        "typedef struct {{ {tuple_struct}* keys; {v_ctype}* values; uint8_t* occ; uint64_t len; uint64_t capacity; uint64_t tombstones; }} {prefix};\n\
         static INTENT_UNUSED bool {prefix}__eq_key({tuple_struct} a, {tuple_struct} b) {{\n\
         \x20 return {eq_expr};\n\
         }}\n\
         static INTENT_UNUSED {prefix} {prefix}_new(void) {{\n\
         \x20 {prefix} m;\n\
         \x20 m.keys = ({tuple_struct}*)0; m.values = ({v_ctype}*)0; m.occ = (uint8_t*)0;\n\
         \x20 m.len = 0; m.capacity = 0; m.tombstones = 0;\n\
         \x20 return m;\n\
         }}\n\
         static INTENT_UNUSED void {prefix}_drop({prefix}* m) {{\n\
         \x20 if (m->keys) free(m->keys);\n\
         \x20 if (m->values) free(m->values);\n\
         \x20 if (m->occ) free(m->occ);\n\
         \x20 m->keys = 0; m->values = 0; m->occ = 0;\n\
         \x20 m->len = 0; m->capacity = 0; m->tombstones = 0;\n\
         }}\n\
         static INTENT_UNUSED uint64_t {prefix}__hash_key({tuple_struct} k) {{\n\
         \x20 uint64_t h = 0xcbf29ce484222325ULL;\n\
         {hash_body}\
         \x20 return h;\n\
         }}\n\
         static INTENT_UNUSED void {prefix}__insert_raw({prefix}* m, {tuple_struct} k, {v_ctype} v) {{\n\
         \x20 uint64_t mask = m->capacity - 1;\n\
         \x20 uint64_t i = {prefix}__hash_key(k) & mask;\n\
         \x20 while (m->occ[i] == 1) {{\n\
         \x20   i = (i + 1) & mask;\n\
         \x20 }}\n\
         \x20 m->keys[i] = k; m->values[i] = v; m->occ[i] = 1; m->len++;\n\
         }}\n\
         static INTENT_UNUSED void {prefix}__grow({prefix}* m) {{\n\
         \x20 uint64_t old_cap = m->capacity;\n\
         \x20 {tuple_struct}* old_keys = m->keys;\n\
         \x20 {v_ctype}* old_values = m->values;\n\
         \x20 uint8_t* old_occ = m->occ;\n\
         \x20 uint64_t new_cap = old_cap == 0 ? 8 : old_cap * 2;\n\
         \x20 m->keys = ({tuple_struct}*)malloc(new_cap * sizeof({tuple_struct}));\n\
         \x20 m->values = ({v_ctype}*)malloc(new_cap * sizeof({v_ctype}));\n\
         \x20 m->occ = (uint8_t*)calloc(new_cap, 1);\n\
         \x20 if (!m->keys || !m->values || !m->occ) abort();\n\
         \x20 m->len = 0;\n\
         \x20 m->capacity = new_cap;\n\
         \x20 m->tombstones = 0;\n\
         \x20 for (uint64_t i = 0; i < old_cap; i++) {{\n\
         \x20   if (old_occ[i] == 1) {prefix}__insert_raw(m, old_keys[i], old_values[i]);\n\
         \x20 }}\n\
         \x20 if (old_keys) free(old_keys);\n\
         \x20 if (old_values) free(old_values);\n\
         \x20 if (old_occ) free(old_occ);\n\
         }}\n\
         static INTENT_UNUSED bool {prefix}_contains_key(const {prefix}* m, {tuple_struct} k) {{\n\
         \x20 if (m->capacity == 0) return false;\n\
         \x20 uint64_t mask = m->capacity - 1;\n\
         \x20 uint64_t i = {prefix}__hash_key(k) & mask;\n\
         \x20 while (m->occ[i] != 0) {{\n\
         \x20   if (m->occ[i] == 1 && {prefix}__eq_key(m->keys[i], k)) return true;\n\
         \x20   i = (i + 1) & mask;\n\
         \x20 }}\n\
         \x20 return false;\n\
         }}\n\
         static INTENT_UNUSED int64_t {prefix}_len(const {prefix}* m) {{\n\
         \x20 return (int64_t)m->len;\n\
         }}\n\
         static INTENT_UNUSED int64_t {prefix}_clear({prefix}* m) {{\n\
         \x20 int64_t prior = (int64_t)m->len;\n\
         \x20 if (m->keys) free(m->keys);\n\
         \x20 if (m->values) free(m->values);\n\
         \x20 if (m->occ) free(m->occ);\n\
         \x20 m->keys = ({tuple_struct}*)0;\n\
         \x20 m->values = ({v_ctype}*)0;\n\
         \x20 m->occ = (uint8_t*)0;\n\
         \x20 m->len = 0;\n\
         \x20 m->capacity = 0;\n\
         \x20 m->tombstones = 0;\n\
         \x20 return prior;\n\
         }}\n",
        tuple_struct = tuple_struct, v_ctype = v_ctype, prefix = prefix,
        hash_body = hash_body, eq_expr = eq_expr,
    ));
    if has_option_v {
        out.push_str(&format!(
            "static INTENT_UNUSED {opt_v} {prefix}_get(const {prefix}* m, {tuple_struct} k) {{\n\
             \x20 {opt_v} r;\n\
             \x20 if (m->capacity == 0) {{ r.tag = 1; r.payload = 0; return r; }}\n\
             \x20 uint64_t mask = m->capacity - 1;\n\
             \x20 uint64_t i = {prefix}__hash_key(k) & mask;\n\
             \x20 while (m->occ[i] != 0) {{\n\
             \x20   if (m->occ[i] == 1 && {prefix}__eq_key(m->keys[i], k)) {{ r.tag = 0; r.payload = m->values[i]; return r; }}\n\
             \x20   i = (i + 1) & mask;\n\
             \x20 }}\n\
             \x20 r.tag = 1; r.payload = 0; return r;\n\
             }}\n\
             static INTENT_UNUSED {opt_v} {prefix}_insert({prefix}* m, {tuple_struct} k, {v_ctype} v) {{\n\
             \x20 if (m->capacity == 0 || ((m->len + m->tombstones) * 2) >= m->capacity) {prefix}__grow(m);\n\
             \x20 {opt_v} r;\n\
             \x20 uint64_t mask = m->capacity - 1;\n\
             \x20 uint64_t i = {prefix}__hash_key(k) & mask;\n\
             \x20 int64_t first_tomb = -1;\n\
             \x20 while (m->occ[i] != 0) {{\n\
             \x20   if (m->occ[i] == 1 && {prefix}__eq_key(m->keys[i], k)) {{\n\
             \x20     r.tag = 0; r.payload = m->values[i];\n\
             \x20     m->values[i] = v;\n\
             \x20     return r;\n\
             \x20   }}\n\
             \x20   if (m->occ[i] == 2 && first_tomb == -1) first_tomb = (int64_t)i;\n\
             \x20   i = (i + 1) & mask;\n\
             \x20 }}\n\
             \x20 if (first_tomb != -1) {{\n\
             \x20   uint64_t slot = (uint64_t)first_tomb;\n\
             \x20   m->keys[slot] = k; m->values[slot] = v; m->occ[slot] = 1;\n\
             \x20   m->len++; m->tombstones--;\n\
             \x20 }} else {{\n\
             \x20   m->keys[i] = k; m->values[i] = v; m->occ[i] = 1; m->len++;\n\
             \x20 }}\n\
             \x20 r.tag = 1; r.payload = 0; return r;\n\
             }}\n\
             static INTENT_UNUSED {opt_v} {prefix}_remove({prefix}* m, {tuple_struct} k) {{\n\
             \x20 {opt_v} r;\n\
             \x20 if (m->capacity == 0) {{ r.tag = 1; r.payload = 0; return r; }}\n\
             \x20 uint64_t mask = m->capacity - 1;\n\
             \x20 uint64_t i = {prefix}__hash_key(k) & mask;\n\
             \x20 while (m->occ[i] != 0) {{\n\
             \x20   if (m->occ[i] == 1 && {prefix}__eq_key(m->keys[i], k)) {{\n\
             \x20     r.tag = 0; r.payload = m->values[i];\n\
             \x20     m->occ[i] = 2;\n\
             \x20     m->len--;\n\
             \x20     m->tombstones++;\n\
             \x20     return r;\n\
             \x20   }}\n\
             \x20   i = (i + 1) & mask;\n\
             \x20 }}\n\
             \x20 r.tag = 1; r.payload = 0; return r;\n\
             }}\n\n",
            tuple_struct = tuple_struct, v_ctype = v_ctype, prefix = prefix, opt_v = opt_v,
        ));
    }
}

/// ARC 4.2 — `HashMap<i64, OwnedStr>`. Map owns each V `char*`
/// pointer via internal clone-on-insert (strlen + malloc + memcpy
/// — same affine-system workaround as ARC 4.1 for K=OwnedStr).
/// Drop/clear walk all occupied slots and `free` each stored V.
/// `_insert` returns the prior V pointer ownership to the caller
/// on duplicate; `_remove` transfers the stored V pointer to the
/// caller; `_get` returns a fresh clone (so map and caller each
/// own one).
fn emit_intent_hashmap_pair_c_body_i64k_strv(
    out: &mut String,
    option_v_mangle: &str,
    has_option_v: bool,
) {
    let prefix = "intent_hashmap_int64_t_owned_str";
    let opt_v = format!("Enum_Option__{}", option_v_mangle);
    out.push_str(&format!(
        "typedef struct {{ int64_t* keys; char** values; uint8_t* occ; uint64_t len; uint64_t capacity; uint64_t tombstones; }} {prefix};\n\
         static INTENT_UNUSED {prefix} {prefix}_new(void) {{\n\
         \x20 {prefix} m;\n\
         \x20 m.keys = (int64_t*)0; m.values = (char**)0; m.occ = (uint8_t*)0;\n\
         \x20 m.len = 0; m.capacity = 0; m.tombstones = 0;\n\
         \x20 return m;\n\
         }}\n\
         static INTENT_UNUSED void {prefix}_drop({prefix}* m) {{\n\
         \x20 if (m->values) {{\n\
         \x20   for (uint64_t i = 0; i < m->capacity; i++) {{\n\
         \x20     if (m->occ[i] == 1 && m->values[i]) free(m->values[i]);\n\
         \x20   }}\n\
         \x20   free(m->values);\n\
         \x20 }}\n\
         \x20 if (m->keys) free(m->keys);\n\
         \x20 if (m->occ) free(m->occ);\n\
         \x20 m->keys = 0; m->values = 0; m->occ = 0;\n\
         \x20 m->len = 0; m->capacity = 0; m->tombstones = 0;\n\
         }}\n\
         static INTENT_UNUSED uint64_t {prefix}__hash_key(int64_t k) {{\n\
         \x20 uint64_t h = 0xcbf29ce484222325ULL;\n\
         \x20 uint64_t u = (uint64_t)k;\n\
         \x20 for (int i = 0; i < 8; i++) {{\n\
         \x20   h ^= (u >> (i * 8)) & 0xffULL;\n\
         \x20   h *= 0x100000001b3ULL;\n\
         \x20 }}\n\
         \x20 return h;\n\
         }}\n\
         /* _insert_owned_raw takes an already-owned V pointer\n\
          * (used by __grow, which moves V's across the rehash\n\
          * without re-cloning). */\n\
         static INTENT_UNUSED void {prefix}__insert_owned_raw({prefix}* m, int64_t k, char* v_owned) {{\n\
         \x20 uint64_t mask = m->capacity - 1;\n\
         \x20 uint64_t i = {prefix}__hash_key(k) & mask;\n\
         \x20 while (m->occ[i] == 1) {{\n\
         \x20   i = (i + 1) & mask;\n\
         \x20 }}\n\
         \x20 m->keys[i] = k; m->values[i] = v_owned; m->occ[i] = 1; m->len++;\n\
         }}\n\
         static INTENT_UNUSED void {prefix}__grow({prefix}* m) {{\n\
         \x20 uint64_t old_cap = m->capacity;\n\
         \x20 int64_t* old_keys = m->keys;\n\
         \x20 char** old_values = m->values;\n\
         \x20 uint8_t* old_occ = m->occ;\n\
         \x20 uint64_t new_cap = old_cap == 0 ? 8 : old_cap * 2;\n\
         \x20 m->keys = (int64_t*)malloc(new_cap * sizeof(int64_t));\n\
         \x20 m->values = (char**)malloc(new_cap * sizeof(char*));\n\
         \x20 m->occ = (uint8_t*)calloc(new_cap, 1);\n\
         \x20 if (!m->keys || !m->values || !m->occ) abort();\n\
         \x20 m->len = 0;\n\
         \x20 m->capacity = new_cap;\n\
         \x20 m->tombstones = 0;\n\
         \x20 for (uint64_t i = 0; i < old_cap; i++) {{\n\
         \x20   if (old_occ[i] == 1) {prefix}__insert_owned_raw(m, old_keys[i], old_values[i]);\n\
         \x20 }}\n\
         \x20 if (old_keys) free(old_keys);\n\
         \x20 if (old_values) free(old_values);\n\
         \x20 if (old_occ) free(old_occ);\n\
         }}\n\
         static INTENT_UNUSED bool {prefix}_contains_key(const {prefix}* m, int64_t k) {{\n\
         \x20 if (m->capacity == 0) return false;\n\
         \x20 uint64_t mask = m->capacity - 1;\n\
         \x20 uint64_t i = {prefix}__hash_key(k) & mask;\n\
         \x20 while (m->occ[i] != 0) {{\n\
         \x20   if (m->occ[i] == 1 && m->keys[i] == k) return true;\n\
         \x20   i = (i + 1) & mask;\n\
         \x20 }}\n\
         \x20 return false;\n\
         }}\n\
         static INTENT_UNUSED int64_t {prefix}_len(const {prefix}* m) {{\n\
         \x20 return (int64_t)m->len;\n\
         }}\n\
         static INTENT_UNUSED int64_t {prefix}_clear({prefix}* m) {{\n\
         \x20 int64_t prior = (int64_t)m->len;\n\
         \x20 if (m->values) {{\n\
         \x20   for (uint64_t i = 0; i < m->capacity; i++) {{\n\
         \x20     if (m->occ[i] == 1 && m->values[i]) free(m->values[i]);\n\
         \x20   }}\n\
         \x20   free(m->values);\n\
         \x20 }}\n\
         \x20 if (m->keys) free(m->keys);\n\
         \x20 if (m->occ) free(m->occ);\n\
         \x20 m->keys = (int64_t*)0;\n\
         \x20 m->values = (char**)0;\n\
         \x20 m->occ = (uint8_t*)0;\n\
         \x20 m->len = 0;\n\
         \x20 m->capacity = 0;\n\
         \x20 m->tombstones = 0;\n\
         \x20 return prior;\n\
         }}\n",
        prefix = prefix,
    ));
    if has_option_v {
        out.push_str(&format!(
            "static INTENT_UNUSED {opt_v} {prefix}_get(const {prefix}* m, int64_t k) {{\n\
             \x20 {opt_v} r;\n\
             \x20 if (m->capacity == 0) {{ r.tag = 1; r.payload = (char*)0; return r; }}\n\
             \x20 uint64_t mask = m->capacity - 1;\n\
             \x20 uint64_t i = {prefix}__hash_key(k) & mask;\n\
             \x20 while (m->occ[i] != 0) {{\n\
             \x20   if (m->occ[i] == 1 && m->keys[i] == k) {{\n\
             \x20     /* Clone the stored V so caller's auto-drop\n\
             \x20      * doesn't disturb the map's copy. */\n\
             \x20     const char* src = m->values[i];\n\
             \x20     size_t n = strlen(src);\n\
             \x20     char* copy = (char*)malloc(n + 1);\n\
             \x20     if (!copy) abort();\n\
             \x20     memcpy(copy, src, n + 1);\n\
             \x20     r.tag = 0; r.payload = copy; return r;\n\
             \x20   }}\n\
             \x20   i = (i + 1) & mask;\n\
             \x20 }}\n\
             \x20 r.tag = 1; r.payload = (char*)0; return r;\n\
             }}\n\
             static INTENT_UNUSED {opt_v} {prefix}_insert({prefix}* m, int64_t k, const char* v) {{\n\
             \x20 if (m->capacity == 0 || ((m->len + m->tombstones) * 2) >= m->capacity) {prefix}__grow(m);\n\
             \x20 {opt_v} r;\n\
             \x20 uint64_t mask = m->capacity - 1;\n\
             \x20 uint64_t i = {prefix}__hash_key(k) & mask;\n\
             \x20 int64_t first_tomb = -1;\n\
             \x20 while (m->occ[i] != 0) {{\n\
             \x20   if (m->occ[i] == 1 && m->keys[i] == k) {{\n\
             \x20     /* Duplicate K: transfer the stored V pointer\n\
             \x20      * ownership to the caller (return it), then\n\
             \x20      * clone the new V and store the clone. */\n\
             \x20     r.tag = 0; r.payload = m->values[i];\n\
             \x20     size_t n_new = strlen(v);\n\
             \x20     char* v_owned = (char*)malloc(n_new + 1);\n\
             \x20     if (!v_owned) abort();\n\
             \x20     memcpy(v_owned, v, n_new + 1);\n\
             \x20     m->values[i] = v_owned;\n\
             \x20     return r;\n\
             \x20   }}\n\
             \x20   if (m->occ[i] == 2 && first_tomb == -1) first_tomb = (int64_t)i;\n\
             \x20   i = (i + 1) & mask;\n\
             \x20 }}\n\
             \x20 /* Clone the caller's V — affine system doesn't\n\
             \x20  * yet suppress local drops for OwnedStr moved into\n\
             \x20  * builtin args; cloning sidesteps the double-free. */\n\
             \x20 size_t n = strlen(v);\n\
             \x20 char* v_owned = (char*)malloc(n + 1);\n\
             \x20 if (!v_owned) abort();\n\
             \x20 memcpy(v_owned, v, n + 1);\n\
             \x20 if (first_tomb != -1) {{\n\
             \x20   uint64_t slot = (uint64_t)first_tomb;\n\
             \x20   m->keys[slot] = k; m->values[slot] = v_owned; m->occ[slot] = 1;\n\
             \x20   m->len++; m->tombstones--;\n\
             \x20 }} else {{\n\
             \x20   m->keys[i] = k; m->values[i] = v_owned; m->occ[i] = 1; m->len++;\n\
             \x20 }}\n\
             \x20 r.tag = 1; r.payload = (char*)0; return r;\n\
             }}\n\
             static INTENT_UNUSED {opt_v} {prefix}_remove({prefix}* m, int64_t k) {{\n\
             \x20 {opt_v} r;\n\
             \x20 if (m->capacity == 0) {{ r.tag = 1; r.payload = (char*)0; return r; }}\n\
             \x20 uint64_t mask = m->capacity - 1;\n\
             \x20 uint64_t i = {prefix}__hash_key(k) & mask;\n\
             \x20 while (m->occ[i] != 0) {{\n\
             \x20   if (m->occ[i] == 1 && m->keys[i] == k) {{\n\
             \x20     /* Transfer V pointer ownership to caller. */\n\
             \x20     r.tag = 0; r.payload = m->values[i];\n\
             \x20     m->values[i] = (char*)0;\n\
             \x20     m->occ[i] = 2;\n\
             \x20     m->len--;\n\
             \x20     m->tombstones++;\n\
             \x20     return r;\n\
             \x20   }}\n\
             \x20   i = (i + 1) & mask;\n\
             \x20 }}\n\
             \x20 r.tag = 1; r.payload = (char*)0; return r;\n\
             }}\n\n",
            prefix = prefix, opt_v = opt_v,
        ));
    }
}

/// ARC 4.3 — `HashMap<OwnedStr, OwnedStr>` — both K and V are
/// heap-owned by the map. Insert clones both K and V (strlen+
/// malloc+memcpy); drop/clear walk frees both. Duplicate K
/// keeps the existing K, swaps V (returns prior V to caller).
/// Remove frees K, transfers V out. Get clones the stored V.
fn emit_intent_hashmap_pair_c_body_strk_strv(
    out: &mut String,
    option_v_mangle: &str,
    has_option_v: bool,
) {
    let prefix = "intent_hashmap_owned_str_owned_str";
    let opt_v = format!("Enum_Option__{}", option_v_mangle);
    out.push_str(&format!(
        "typedef struct {{ char** keys; char** values; uint8_t* occ; uint64_t len; uint64_t capacity; uint64_t tombstones; }} {prefix};\n\
         static INTENT_UNUSED {prefix} {prefix}_new(void) {{\n\
         \x20 {prefix} m;\n\
         \x20 m.keys = (char**)0; m.values = (char**)0; m.occ = (uint8_t*)0;\n\
         \x20 m.len = 0; m.capacity = 0; m.tombstones = 0;\n\
         \x20 return m;\n\
         }}\n\
         static INTENT_UNUSED void {prefix}_drop({prefix}* m) {{\n\
         \x20 if (m->keys) {{\n\
         \x20   for (uint64_t i = 0; i < m->capacity; i++) {{\n\
         \x20     if (m->occ[i] == 1) {{\n\
         \x20       if (m->keys[i]) free(m->keys[i]);\n\
         \x20       if (m->values[i]) free(m->values[i]);\n\
         \x20     }}\n\
         \x20   }}\n\
         \x20   free(m->keys);\n\
         \x20 }}\n\
         \x20 if (m->values) free(m->values);\n\
         \x20 if (m->occ) free(m->occ);\n\
         \x20 m->keys = 0; m->values = 0; m->occ = 0;\n\
         \x20 m->len = 0; m->capacity = 0; m->tombstones = 0;\n\
         }}\n\
         static INTENT_UNUSED uint64_t {prefix}__hash_key(const char* k) {{\n\
         \x20 uint64_t h = 0xcbf29ce484222325ULL;\n\
         \x20 for (const char* p = k; *p; p++) {{\n\
         \x20   h ^= (uint64_t)(unsigned char)(*p);\n\
         \x20   h *= 0x100000001b3ULL;\n\
         \x20 }}\n\
         \x20 return h;\n\
         }}\n\
         static INTENT_UNUSED void {prefix}__insert_owned_raw({prefix}* m, char* k_owned, char* v_owned) {{\n\
         \x20 uint64_t mask = m->capacity - 1;\n\
         \x20 uint64_t i = {prefix}__hash_key(k_owned) & mask;\n\
         \x20 while (m->occ[i] == 1) {{\n\
         \x20   i = (i + 1) & mask;\n\
         \x20 }}\n\
         \x20 m->keys[i] = k_owned; m->values[i] = v_owned; m->occ[i] = 1; m->len++;\n\
         }}\n\
         static INTENT_UNUSED void {prefix}__grow({prefix}* m) {{\n\
         \x20 uint64_t old_cap = m->capacity;\n\
         \x20 char** old_keys = m->keys;\n\
         \x20 char** old_values = m->values;\n\
         \x20 uint8_t* old_occ = m->occ;\n\
         \x20 uint64_t new_cap = old_cap == 0 ? 8 : old_cap * 2;\n\
         \x20 m->keys = (char**)malloc(new_cap * sizeof(char*));\n\
         \x20 m->values = (char**)malloc(new_cap * sizeof(char*));\n\
         \x20 m->occ = (uint8_t*)calloc(new_cap, 1);\n\
         \x20 if (!m->keys || !m->values || !m->occ) abort();\n\
         \x20 m->len = 0;\n\
         \x20 m->capacity = new_cap;\n\
         \x20 m->tombstones = 0;\n\
         \x20 for (uint64_t i = 0; i < old_cap; i++) {{\n\
         \x20   if (old_occ[i] == 1) {prefix}__insert_owned_raw(m, old_keys[i], old_values[i]);\n\
         \x20 }}\n\
         \x20 if (old_keys) free(old_keys);\n\
         \x20 if (old_values) free(old_values);\n\
         \x20 if (old_occ) free(old_occ);\n\
         }}\n\
         static INTENT_UNUSED bool {prefix}_contains_key(const {prefix}* m, const char* k) {{\n\
         \x20 if (m->capacity == 0) return false;\n\
         \x20 uint64_t mask = m->capacity - 1;\n\
         \x20 uint64_t i = {prefix}__hash_key(k) & mask;\n\
         \x20 while (m->occ[i] != 0) {{\n\
         \x20   if (m->occ[i] == 1 && strcmp(m->keys[i], k) == 0) return true;\n\
         \x20   i = (i + 1) & mask;\n\
         \x20 }}\n\
         \x20 return false;\n\
         }}\n\
         static INTENT_UNUSED int64_t {prefix}_len(const {prefix}* m) {{\n\
         \x20 return (int64_t)m->len;\n\
         }}\n\
         static INTENT_UNUSED int64_t {prefix}_clear({prefix}* m) {{\n\
         \x20 int64_t prior = (int64_t)m->len;\n\
         \x20 if (m->keys) {{\n\
         \x20   for (uint64_t i = 0; i < m->capacity; i++) {{\n\
         \x20     if (m->occ[i] == 1) {{\n\
         \x20       if (m->keys[i]) free(m->keys[i]);\n\
         \x20       if (m->values[i]) free(m->values[i]);\n\
         \x20     }}\n\
         \x20   }}\n\
         \x20   free(m->keys);\n\
         \x20 }}\n\
         \x20 if (m->values) free(m->values);\n\
         \x20 if (m->occ) free(m->occ);\n\
         \x20 m->keys = (char**)0;\n\
         \x20 m->values = (char**)0;\n\
         \x20 m->occ = (uint8_t*)0;\n\
         \x20 m->len = 0;\n\
         \x20 m->capacity = 0;\n\
         \x20 m->tombstones = 0;\n\
         \x20 return prior;\n\
         }}\n",
        prefix = prefix,
    ));
    if has_option_v {
        out.push_str(&format!(
            "static INTENT_UNUSED {opt_v} {prefix}_get(const {prefix}* m, const char* k) {{\n\
             \x20 {opt_v} r;\n\
             \x20 if (m->capacity == 0) {{ r.tag = 1; r.payload = (char*)0; return r; }}\n\
             \x20 uint64_t mask = m->capacity - 1;\n\
             \x20 uint64_t i = {prefix}__hash_key(k) & mask;\n\
             \x20 while (m->occ[i] != 0) {{\n\
             \x20   if (m->occ[i] == 1 && strcmp(m->keys[i], k) == 0) {{\n\
             \x20     const char* src = m->values[i];\n\
             \x20     size_t n = strlen(src);\n\
             \x20     char* copy = (char*)malloc(n + 1);\n\
             \x20     if (!copy) abort();\n\
             \x20     memcpy(copy, src, n + 1);\n\
             \x20     r.tag = 0; r.payload = copy; return r;\n\
             \x20   }}\n\
             \x20   i = (i + 1) & mask;\n\
             \x20 }}\n\
             \x20 r.tag = 1; r.payload = (char*)0; return r;\n\
             }}\n\
             static INTENT_UNUSED {opt_v} {prefix}_insert({prefix}* m, const char* k, const char* v) {{\n\
             \x20 if (m->capacity == 0 || ((m->len + m->tombstones) * 2) >= m->capacity) {prefix}__grow(m);\n\
             \x20 {opt_v} r;\n\
             \x20 uint64_t mask = m->capacity - 1;\n\
             \x20 uint64_t i = {prefix}__hash_key(k) & mask;\n\
             \x20 int64_t first_tomb = -1;\n\
             \x20 while (m->occ[i] != 0) {{\n\
             \x20   if (m->occ[i] == 1 && strcmp(m->keys[i], k) == 0) {{\n\
             \x20     /* Duplicate K: keep existing K, swap V. */\n\
             \x20     r.tag = 0; r.payload = m->values[i];\n\
             \x20     size_t nv2 = strlen(v);\n\
             \x20     char* v_owned2 = (char*)malloc(nv2 + 1);\n\
             \x20     if (!v_owned2) abort();\n\
             \x20     memcpy(v_owned2, v, nv2 + 1);\n\
             \x20     m->values[i] = v_owned2;\n\
             \x20     return r;\n\
             \x20   }}\n\
             \x20   if (m->occ[i] == 2 && first_tomb == -1) first_tomb = (int64_t)i;\n\
             \x20   i = (i + 1) & mask;\n\
             \x20 }}\n\
             \x20 /* Clone both K and V. */\n\
             \x20 size_t nk = strlen(k);\n\
             \x20 char* k_owned = (char*)malloc(nk + 1);\n\
             \x20 if (!k_owned) abort();\n\
             \x20 memcpy(k_owned, k, nk + 1);\n\
             \x20 size_t nv = strlen(v);\n\
             \x20 char* v_owned = (char*)malloc(nv + 1);\n\
             \x20 if (!v_owned) abort();\n\
             \x20 memcpy(v_owned, v, nv + 1);\n\
             \x20 if (first_tomb != -1) {{\n\
             \x20   uint64_t slot = (uint64_t)first_tomb;\n\
             \x20   m->keys[slot] = k_owned; m->values[slot] = v_owned; m->occ[slot] = 1;\n\
             \x20   m->len++; m->tombstones--;\n\
             \x20 }} else {{\n\
             \x20   m->keys[i] = k_owned; m->values[i] = v_owned; m->occ[i] = 1; m->len++;\n\
             \x20 }}\n\
             \x20 r.tag = 1; r.payload = (char*)0; return r;\n\
             }}\n\
             static INTENT_UNUSED {opt_v} {prefix}_remove({prefix}* m, const char* k) {{\n\
             \x20 {opt_v} r;\n\
             \x20 if (m->capacity == 0) {{ r.tag = 1; r.payload = (char*)0; return r; }}\n\
             \x20 uint64_t mask = m->capacity - 1;\n\
             \x20 uint64_t i = {prefix}__hash_key(k) & mask;\n\
             \x20 while (m->occ[i] != 0) {{\n\
             \x20   if (m->occ[i] == 1 && strcmp(m->keys[i], k) == 0) {{\n\
             \x20     /* Free K; transfer V ownership to caller. */\n\
             \x20     r.tag = 0; r.payload = m->values[i];\n\
             \x20     free(m->keys[i]);\n\
             \x20     m->keys[i] = (char*)0;\n\
             \x20     m->values[i] = (char*)0;\n\
             \x20     m->occ[i] = 2;\n\
             \x20     m->len--;\n\
             \x20     m->tombstones++;\n\
             \x20     return r;\n\
             \x20   }}\n\
             \x20   i = (i + 1) & mask;\n\
             \x20 }}\n\
             \x20 r.tag = 1; r.payload = (char*)0; return r;\n\
             }}\n\n",
            prefix = prefix, opt_v = opt_v,
        ));
    }
}

/// ARC 4.6 — `HashMap<Vec<i64>, V>` for V scalar. K is the
/// existing `intent_vec_int64_t` struct (data + len + cap);
/// map deep-clones the data array on insert (same affine
/// workaround as ARC 4.1/4.2 — sidesteps local-drop double-
/// free). Drop/clear walk frees each stored Vec's data
/// buffer. Hash: FNV-1a over each i64's 8 bytes, prefixed by
/// the length so empty/short Vec values disambiguate; same
/// byte distribution as the i64-K bundle generalized.
/// Equality: lengths-equal then byte memcmp of data.
fn emit_intent_hashmap_pair_c_body_vec_i64k(
    out: &mut String,
    v_tag: &str,
    v_ctype: &str,
    option_v_mangle: &str,
    has_option_v: bool,
) {
    let prefix = format!("intent_hashmap_vec_int64_t_{}", v_tag);
    let opt_v = format!("Enum_Option__{}", option_v_mangle);
    out.push_str(&format!(
        "typedef struct {{ intent_vec_int64_t* keys; {v_ctype}* values; uint8_t* occ; uint64_t len; uint64_t capacity; uint64_t tombstones; }} {prefix};\n\
         static INTENT_UNUSED bool {prefix}__eq_key(intent_vec_int64_t a, intent_vec_int64_t b) {{\n\
         \x20 if (a.len != b.len) return false;\n\
         \x20 if (a.len == 0) return true;\n\
         \x20 return memcmp(a.data, b.data, (size_t)a.len * sizeof(int64_t)) == 0;\n\
         }}\n\
         static INTENT_UNUSED {prefix} {prefix}_new(void) {{\n\
         \x20 {prefix} m;\n\
         \x20 m.keys = (intent_vec_int64_t*)0; m.values = ({v_ctype}*)0; m.occ = (uint8_t*)0;\n\
         \x20 m.len = 0; m.capacity = 0; m.tombstones = 0;\n\
         \x20 return m;\n\
         }}\n\
         static INTENT_UNUSED void {prefix}_drop({prefix}* m) {{\n\
         \x20 if (m->keys) {{\n\
         \x20   for (uint64_t i = 0; i < m->capacity; i++) {{\n\
         \x20     if (m->occ[i] == 1 && m->keys[i].data) free(m->keys[i].data);\n\
         \x20   }}\n\
         \x20   free(m->keys);\n\
         \x20 }}\n\
         \x20 if (m->values) free(m->values);\n\
         \x20 if (m->occ) free(m->occ);\n\
         \x20 m->keys = 0; m->values = 0; m->occ = 0;\n\
         \x20 m->len = 0; m->capacity = 0; m->tombstones = 0;\n\
         }}\n\
         static INTENT_UNUSED uint64_t {prefix}__hash_key(intent_vec_int64_t k) {{\n\
         \x20 uint64_t h = 0xcbf29ce484222325ULL;\n\
         \x20 /* Length-prefix so empty / shorter Vecs distribute. */\n\
         \x20 {{\n\
         \x20   uint64_t u_len = (uint64_t)k.len;\n\
         \x20   for (int b = 0; b < 8; b++) {{\n\
         \x20     h ^= (u_len >> (b * 8)) & 0xffULL;\n\
         \x20     h *= 0x100000001b3ULL;\n\
         \x20   }}\n\
         \x20 }}\n\
         \x20 for (int64_t e = 0; e < k.len; e++) {{\n\
         \x20   uint64_t ue = (uint64_t)k.data[e];\n\
         \x20   for (int b = 0; b < 8; b++) {{\n\
         \x20     h ^= (ue >> (b * 8)) & 0xffULL;\n\
         \x20     h *= 0x100000001b3ULL;\n\
         \x20   }}\n\
         \x20 }}\n\
         \x20 return h;\n\
         }}\n\
         static INTENT_UNUSED void {prefix}__insert_owned_raw({prefix}* m, intent_vec_int64_t k_owned, {v_ctype} v) {{\n\
         \x20 uint64_t mask = m->capacity - 1;\n\
         \x20 uint64_t i = {prefix}__hash_key(k_owned) & mask;\n\
         \x20 while (m->occ[i] == 1) {{\n\
         \x20   i = (i + 1) & mask;\n\
         \x20 }}\n\
         \x20 m->keys[i] = k_owned; m->values[i] = v; m->occ[i] = 1; m->len++;\n\
         }}\n\
         static INTENT_UNUSED void {prefix}__grow({prefix}* m) {{\n\
         \x20 uint64_t old_cap = m->capacity;\n\
         \x20 intent_vec_int64_t* old_keys = m->keys;\n\
         \x20 {v_ctype}* old_values = m->values;\n\
         \x20 uint8_t* old_occ = m->occ;\n\
         \x20 uint64_t new_cap = old_cap == 0 ? 8 : old_cap * 2;\n\
         \x20 m->keys = (intent_vec_int64_t*)malloc(new_cap * sizeof(intent_vec_int64_t));\n\
         \x20 m->values = ({v_ctype}*)malloc(new_cap * sizeof({v_ctype}));\n\
         \x20 m->occ = (uint8_t*)calloc(new_cap, 1);\n\
         \x20 if (!m->keys || !m->values || !m->occ) abort();\n\
         \x20 m->len = 0;\n\
         \x20 m->capacity = new_cap;\n\
         \x20 m->tombstones = 0;\n\
         \x20 for (uint64_t i = 0; i < old_cap; i++) {{\n\
         \x20   if (old_occ[i] == 1) {prefix}__insert_owned_raw(m, old_keys[i], old_values[i]);\n\
         \x20 }}\n\
         \x20 if (old_keys) free(old_keys);\n\
         \x20 if (old_values) free(old_values);\n\
         \x20 if (old_occ) free(old_occ);\n\
         }}\n\
         static INTENT_UNUSED bool {prefix}_contains_key(const {prefix}* m, intent_vec_int64_t k) {{\n\
         \x20 if (m->capacity == 0) return false;\n\
         \x20 uint64_t mask = m->capacity - 1;\n\
         \x20 uint64_t i = {prefix}__hash_key(k) & mask;\n\
         \x20 while (m->occ[i] != 0) {{\n\
         \x20   if (m->occ[i] == 1 && {prefix}__eq_key(m->keys[i], k)) return true;\n\
         \x20   i = (i + 1) & mask;\n\
         \x20 }}\n\
         \x20 return false;\n\
         }}\n\
         static INTENT_UNUSED int64_t {prefix}_len(const {prefix}* m) {{\n\
         \x20 return (int64_t)m->len;\n\
         }}\n\
         static INTENT_UNUSED int64_t {prefix}_clear({prefix}* m) {{\n\
         \x20 int64_t prior = (int64_t)m->len;\n\
         \x20 if (m->keys) {{\n\
         \x20   for (uint64_t i = 0; i < m->capacity; i++) {{\n\
         \x20     if (m->occ[i] == 1 && m->keys[i].data) free(m->keys[i].data);\n\
         \x20   }}\n\
         \x20   free(m->keys);\n\
         \x20 }}\n\
         \x20 if (m->values) free(m->values);\n\
         \x20 if (m->occ) free(m->occ);\n\
         \x20 m->keys = (intent_vec_int64_t*)0;\n\
         \x20 m->values = ({v_ctype}*)0;\n\
         \x20 m->occ = (uint8_t*)0;\n\
         \x20 m->len = 0;\n\
         \x20 m->capacity = 0;\n\
         \x20 m->tombstones = 0;\n\
         \x20 return prior;\n\
         }}\n",
        v_ctype = v_ctype, prefix = prefix,
    ));
    if has_option_v {
        out.push_str(&format!(
            "static INTENT_UNUSED {opt_v} {prefix}_get(const {prefix}* m, intent_vec_int64_t k) {{\n\
             \x20 {opt_v} r;\n\
             \x20 if (m->capacity == 0) {{ r.tag = 1; r.payload = 0; return r; }}\n\
             \x20 uint64_t mask = m->capacity - 1;\n\
             \x20 uint64_t i = {prefix}__hash_key(k) & mask;\n\
             \x20 while (m->occ[i] != 0) {{\n\
             \x20   if (m->occ[i] == 1 && {prefix}__eq_key(m->keys[i], k)) {{ r.tag = 0; r.payload = m->values[i]; return r; }}\n\
             \x20   i = (i + 1) & mask;\n\
             \x20 }}\n\
             \x20 r.tag = 1; r.payload = 0; return r;\n\
             }}\n\
             static INTENT_UNUSED {opt_v} {prefix}_insert({prefix}* m, intent_vec_int64_t k, {v_ctype} v) {{\n\
             \x20 if (m->capacity == 0 || ((m->len + m->tombstones) * 2) >= m->capacity) {prefix}__grow(m);\n\
             \x20 {opt_v} r;\n\
             \x20 uint64_t mask = m->capacity - 1;\n\
             \x20 uint64_t i = {prefix}__hash_key(k) & mask;\n\
             \x20 int64_t first_tomb = -1;\n\
             \x20 while (m->occ[i] != 0) {{\n\
             \x20   if (m->occ[i] == 1 && {prefix}__eq_key(m->keys[i], k)) {{\n\
             \x20     /* Duplicate K — keep existing K clone, swap V. */\n\
             \x20     r.tag = 0; r.payload = m->values[i];\n\
             \x20     m->values[i] = v;\n\
             \x20     return r;\n\
             \x20   }}\n\
             \x20   if (m->occ[i] == 2 && first_tomb == -1) first_tomb = (int64_t)i;\n\
             \x20   i = (i + 1) & mask;\n\
             \x20 }}\n\
             \x20 /* Deep-clone the caller's Vec data buffer. */\n\
             \x20 intent_vec_int64_t k_owned;\n\
             \x20 k_owned.len = k.len;\n\
             \x20 k_owned.capacity = k.len;\n\
             \x20 if (k.len > 0) {{\n\
             \x20   k_owned.data = (int64_t*)malloc((size_t)k.len * sizeof(int64_t));\n\
             \x20   if (!k_owned.data) abort();\n\
             \x20   memcpy(k_owned.data, k.data, (size_t)k.len * sizeof(int64_t));\n\
             \x20 }} else {{\n\
             \x20   k_owned.data = (int64_t*)0;\n\
             \x20 }}\n\
             \x20 if (first_tomb != -1) {{\n\
             \x20   uint64_t slot = (uint64_t)first_tomb;\n\
             \x20   m->keys[slot] = k_owned; m->values[slot] = v; m->occ[slot] = 1;\n\
             \x20   m->len++; m->tombstones--;\n\
             \x20 }} else {{\n\
             \x20   m->keys[i] = k_owned; m->values[i] = v; m->occ[i] = 1; m->len++;\n\
             \x20 }}\n\
             \x20 r.tag = 1; r.payload = 0; return r;\n\
             }}\n\
             static INTENT_UNUSED {opt_v} {prefix}_remove({prefix}* m, intent_vec_int64_t k) {{\n\
             \x20 {opt_v} r;\n\
             \x20 if (m->capacity == 0) {{ r.tag = 1; r.payload = 0; return r; }}\n\
             \x20 uint64_t mask = m->capacity - 1;\n\
             \x20 uint64_t i = {prefix}__hash_key(k) & mask;\n\
             \x20 while (m->occ[i] != 0) {{\n\
             \x20   if (m->occ[i] == 1 && {prefix}__eq_key(m->keys[i], k)) {{\n\
             \x20     r.tag = 0; r.payload = m->values[i];\n\
             \x20     if (m->keys[i].data) free(m->keys[i].data);\n\
             \x20     m->keys[i].data = (int64_t*)0;\n\
             \x20     m->keys[i].len = 0;\n\
             \x20     m->keys[i].capacity = 0;\n\
             \x20     m->occ[i] = 2;\n\
             \x20     m->len--;\n\
             \x20     m->tombstones++;\n\
             \x20     return r;\n\
             \x20   }}\n\
             \x20   i = (i + 1) & mask;\n\
             \x20 }}\n\
             \x20 r.tag = 1; r.payload = 0; return r;\n\
             }}\n\n",
            v_ctype = v_ctype, prefix = prefix, opt_v = opt_v,
        ));
    }
}

fn emit_intent_hashmap_helpers_c_body(out: &mut String, has_option_i64: bool) {
    out.push_str(
        "typedef struct { int64_t* keys; int64_t* values; uint8_t* occ; uint64_t len; uint64_t capacity; uint64_t tombstones; } intent_hashmap_i64_i64;\n\
         /* occ byte states (closure #343):\n\
          *   0 = empty       — terminates probe chains\n\
          *   1 = occupied    — slot in use\n\
          *   2 = tombstone   — slot removed; probe must continue past it\n\
          * Grow triggers on (len + tombstones) * 2 >= capacity so a\n\
          * remove-heavy workload eventually rehashes and clears\n\
          * tombstones. */\n\
         static INTENT_UNUSED intent_hashmap_i64_i64 intent_hashmap_i64_i64_new(void) {\n\
         \x20 intent_hashmap_i64_i64 m;\n\
         \x20 m.keys = (int64_t*)0; m.values = (int64_t*)0; m.occ = (uint8_t*)0;\n\
         \x20 m.len = 0; m.capacity = 0; m.tombstones = 0;\n\
         \x20 return m;\n\
         }\n\
         static INTENT_UNUSED void intent_hashmap_i64_i64_drop(intent_hashmap_i64_i64* m) {\n\
         \x20 if (m->keys) free(m->keys);\n\
         \x20 if (m->values) free(m->values);\n\
         \x20 if (m->occ) free(m->occ);\n\
         \x20 m->keys = 0; m->values = 0; m->occ = 0;\n\
         \x20 m->len = 0; m->capacity = 0; m->tombstones = 0;\n\
         }\n\
         static INTENT_UNUSED uint64_t intent_hashmap_i64_i64__hash_key(int64_t k) {\n\
         \x20 uint64_t h = 0xcbf29ce484222325ULL;\n\
         \x20 uint64_t u = (uint64_t)k;\n\
         \x20 for (int i = 0; i < 8; i++) {\n\
         \x20   h ^= (u >> (i * 8)) & 0xffULL;\n\
         \x20   h *= 0x100000001b3ULL;\n\
         \x20 }\n\
         \x20 return h;\n\
         }\n\
         /* Rehash-only insert (used during grow): assumes no\n\
          * tombstones exist (just-allocated occ array). */\n\
         static INTENT_UNUSED void intent_hashmap_i64_i64__insert_raw(intent_hashmap_i64_i64* m, int64_t k, int64_t v) {\n\
         \x20 uint64_t mask = m->capacity - 1;\n\
         \x20 uint64_t i = intent_hashmap_i64_i64__hash_key(k) & mask;\n\
         \x20 while (m->occ[i] == 1) {\n\
         \x20   i = (i + 1) & mask;\n\
         \x20 }\n\
         \x20 m->keys[i] = k; m->values[i] = v; m->occ[i] = 1; m->len++;\n\
         }\n\
         static INTENT_UNUSED void intent_hashmap_i64_i64__grow(intent_hashmap_i64_i64* m) {\n\
         \x20 uint64_t old_cap = m->capacity;\n\
         \x20 int64_t* old_keys = m->keys;\n\
         \x20 int64_t* old_values = m->values;\n\
         \x20 uint8_t* old_occ = m->occ;\n\
         \x20 uint64_t new_cap = old_cap == 0 ? 8 : old_cap * 2;\n\
         \x20 m->keys = (int64_t*)malloc(new_cap * sizeof(int64_t));\n\
         \x20 m->values = (int64_t*)malloc(new_cap * sizeof(int64_t));\n\
         \x20 m->occ = (uint8_t*)calloc(new_cap, 1);\n\
         \x20 if (!m->keys || !m->values || !m->occ) abort();\n\
         \x20 m->len = 0;\n\
         \x20 m->capacity = new_cap;\n\
         \x20 m->tombstones = 0;\n\
         \x20 for (uint64_t i = 0; i < old_cap; i++) {\n\
         \x20   if (old_occ[i] == 1) intent_hashmap_i64_i64__insert_raw(m, old_keys[i], old_values[i]);\n\
         \x20 }\n\
         \x20 if (old_keys) free(old_keys);\n\
         \x20 if (old_values) free(old_values);\n\
         \x20 if (old_occ) free(old_occ);\n\
         }\n\
         static INTENT_UNUSED bool intent_hashmap_i64_i64_contains_key(const intent_hashmap_i64_i64* m, int64_t k) {\n\
         \x20 if (m->capacity == 0) return false;\n\
         \x20 uint64_t mask = m->capacity - 1;\n\
         \x20 uint64_t i = intent_hashmap_i64_i64__hash_key(k) & mask;\n\
         \x20 while (m->occ[i] != 0) {\n\
         \x20   if (m->occ[i] == 1 && m->keys[i] == k) return true;\n\
         \x20   i = (i + 1) & mask;\n\
         \x20 }\n\
         \x20 return false;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_hashmap_i64_i64_len(const intent_hashmap_i64_i64* m) {\n\
         \x20 return (int64_t)m->len;\n\
         }\n\
         /* Closure #353: clear() — drop the three parallel buffers\n\
          * and reset to empty state. Returns prior len. */\n\
         static INTENT_UNUSED int64_t intent_hashmap_i64_i64_clear(intent_hashmap_i64_i64* m) {\n\
         \x20 int64_t prior = (int64_t)m->len;\n\
         \x20 if (m->keys) free(m->keys);\n\
         \x20 if (m->values) free(m->values);\n\
         \x20 if (m->occ) free(m->occ);\n\
         \x20 m->keys = (int64_t*)0;\n\
         \x20 m->values = (int64_t*)0;\n\
         \x20 m->occ = (uint8_t*)0;\n\
         \x20 m->len = 0;\n\
         \x20 m->capacity = 0;\n\
         \x20 m->tombstones = 0;\n\
         \x20 return prior;\n\
         }\n",
    );
    if has_option_i64 {
        out.push_str(
            "static INTENT_UNUSED Enum_Option__i64 intent_hashmap_i64_i64_get(const intent_hashmap_i64_i64* m, int64_t k) {\n\
             \x20 Enum_Option__i64 r;\n\
             \x20 if (m->capacity == 0) { r.tag = 1; r.payload = 0; return r; }\n\
             \x20 uint64_t mask = m->capacity - 1;\n\
             \x20 uint64_t i = intent_hashmap_i64_i64__hash_key(k) & mask;\n\
             \x20 while (m->occ[i] != 0) {\n\
             \x20   if (m->occ[i] == 1 && m->keys[i] == k) { r.tag = 0; r.payload = m->values[i]; return r; }\n\
             \x20   i = (i + 1) & mask;\n\
             \x20 }\n\
             \x20 r.tag = 1; r.payload = 0; return r;\n\
             }\n\
             static INTENT_UNUSED Enum_Option__i64 intent_hashmap_i64_i64_insert(intent_hashmap_i64_i64* m, int64_t k, int64_t v) {\n\
             \x20 if (m->capacity == 0 || ((m->len + m->tombstones) * 2) >= m->capacity) intent_hashmap_i64_i64__grow(m);\n\
             \x20 Enum_Option__i64 r;\n\
             \x20 uint64_t mask = m->capacity - 1;\n\
             \x20 uint64_t i = intent_hashmap_i64_i64__hash_key(k) & mask;\n\
             \x20 /* First-tombstone-or-empty placement: walk past tombstones in\n\
              * case the key already lives further down the probe chain. */\n\
             \x20 int64_t first_tomb = -1;\n\
             \x20 while (m->occ[i] != 0) {\n\
             \x20   if (m->occ[i] == 1 && m->keys[i] == k) {\n\
             \x20     r.tag = 0; r.payload = m->values[i];\n\
             \x20     m->values[i] = v;\n\
             \x20     return r;\n\
             \x20   }\n\
             \x20   if (m->occ[i] == 2 && first_tomb == -1) first_tomb = (int64_t)i;\n\
             \x20   i = (i + 1) & mask;\n\
             \x20 }\n\
             \x20 if (first_tomb != -1) {\n\
             \x20   uint64_t slot = (uint64_t)first_tomb;\n\
             \x20   m->keys[slot] = k; m->values[slot] = v; m->occ[slot] = 1;\n\
             \x20   m->len++; m->tombstones--;\n\
             \x20 } else {\n\
             \x20   m->keys[i] = k; m->values[i] = v; m->occ[i] = 1; m->len++;\n\
             \x20 }\n\
             \x20 r.tag = 1; r.payload = 0; return r;\n\
             }\n\
             /* Closure #343: remove. Returns Some(prev_value) if the key\n\
              * was present (marks slot as tombstone), None otherwise. */\n\
             static INTENT_UNUSED Enum_Option__i64 intent_hashmap_i64_i64_remove(intent_hashmap_i64_i64* m, int64_t k) {\n\
             \x20 Enum_Option__i64 r;\n\
             \x20 if (m->capacity == 0) { r.tag = 1; r.payload = 0; return r; }\n\
             \x20 uint64_t mask = m->capacity - 1;\n\
             \x20 uint64_t i = intent_hashmap_i64_i64__hash_key(k) & mask;\n\
             \x20 while (m->occ[i] != 0) {\n\
             \x20   if (m->occ[i] == 1 && m->keys[i] == k) {\n\
             \x20     r.tag = 0; r.payload = m->values[i];\n\
             \x20     m->occ[i] = 2;\n\
             \x20     m->len--;\n\
             \x20     m->tombstones++;\n\
             \x20     return r;\n\
             \x20   }\n\
             \x20   i = (i + 1) & mask;\n\
             \x20 }\n\
             \x20 r.tag = 1; r.payload = 0; return r;\n\
             }\n\n",
        );
    }
}

/// Walk the program for any `BTreeSet<i64>` type usage.
pub(crate) fn program_uses_i64_btreeset(program: &TypedProgram) -> bool {
    fn ty_uses(ty: &Type) -> bool {
        match ty {
            Type::BTreeSet(element) if matches!(**element, Type::I64) => true,
            Type::Vec(inner) | Type::Ref(inner) | Type::RefMut(inner) => ty_uses(inner),
            _ => false,
        }
    }
    for f in &program.functions {
        if ty_uses(&f.return_type) {
            return true;
        }
        for p in &f.params {
            if ty_uses(&p.ty) {
                return true;
            }
        }
        for s in &f.body {
            if stmt_uses_i64_btreeset(s) {
                return true;
            }
        }
    }
    false
}

fn stmt_uses_i64_btreeset(stmt: &crate::ir::TypedStmt) -> bool {
    use crate::ir::TypedStmt as S;
    fn ty_uses(ty: &Type) -> bool {
        matches!(ty, Type::BTreeSet(element) if matches!(**element, Type::I64))
            || matches!(ty,
                Type::Vec(i) | Type::Ref(i) | Type::RefMut(i) if ty_uses(i))
    }
    match stmt {
        S::Let { ty, .. } | S::Reassign { ty, .. } | S::Drop { ty, .. } => ty_uses(ty),
        S::If { then_body, else_body, .. } => {
            then_body.iter().any(stmt_uses_i64_btreeset)
                || else_body.iter().any(stmt_uses_i64_btreeset)
        }
        S::While { body, .. } | S::For { body, .. } | S::ForIter { body, .. } => {
            body.iter().any(stmt_uses_i64_btreeset)
        }
        _ => false,
    }
}

/// Data-structures roadmap Level 2 — BTreeSet<i64> runtime
/// helpers. v1 backed by sorted Vec<i64>: binary_search for
/// lookup (O(log n)), memmove shift for insert / remove
/// (O(n)). Naturally sorted iteration order. Real B-tree
/// arena variant queued for Level 4.
fn emit_intent_btreeset_helpers_c_body(out: &mut String, has_option_i64: bool, emit_vec_dep: bool) {
    out.push_str(
        "typedef struct { int64_t* keys; uint64_t len; uint64_t capacity; } intent_btreeset_i64;\n\
         static INTENT_UNUSED intent_btreeset_i64 intent_btreeset_i64_new(void) {\n\
         \x20 intent_btreeset_i64 s; s.keys = (int64_t*)0; s.len = 0; s.capacity = 0; return s;\n\
         }\n\
         static INTENT_UNUSED void intent_btreeset_i64_drop(intent_btreeset_i64* s) {\n\
         \x20 if (s->keys) free(s->keys);\n\
         \x20 s->keys = (int64_t*)0; s->len = 0; s->capacity = 0;\n\
         }\n\
         /* Returns the slot index where `k` lives or would be\n\
         \x20  inserted to keep the array sorted ascending. */\n\
         static INTENT_UNUSED uint64_t intent_btreeset_i64__lower_bound(const intent_btreeset_i64* s, int64_t k) {\n\
         \x20 uint64_t lo = 0; uint64_t hi = s->len;\n\
         \x20 while (lo < hi) {\n\
         \x20   uint64_t mid = lo + (hi - lo) / 2;\n\
         \x20   if (s->keys[mid] < k) lo = mid + 1; else hi = mid;\n\
         \x20 }\n\
         \x20 return lo;\n\
         }\n\
         static INTENT_UNUSED bool intent_btreeset_i64_contains(const intent_btreeset_i64* s, int64_t k) {\n\
         \x20 uint64_t i = intent_btreeset_i64__lower_bound(s, k);\n\
         \x20 return i < s->len && s->keys[i] == k;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_btreeset_i64_len(const intent_btreeset_i64* s) {\n\
         \x20 return (int64_t)s->len;\n\
         }\n\
         /* Closure #353: clear() — free the sorted key buffer\n\
          * and reset to empty. Returns prior len. */\n\
         static INTENT_UNUSED int64_t intent_btreeset_i64_clear(intent_btreeset_i64* s) {\n\
         \x20 int64_t prior = (int64_t)s->len;\n\
         \x20 if (s->keys) free(s->keys);\n\
         \x20 s->keys = (int64_t*)0;\n\
         \x20 s->len = 0;\n\
         \x20 s->capacity = 0;\n\
         \x20 return prior;\n\
         }\n\
         static INTENT_UNUSED bool intent_btreeset_i64_insert(intent_btreeset_i64* s, int64_t k) {\n\
         \x20 uint64_t i = intent_btreeset_i64__lower_bound(s, k);\n\
         \x20 if (i < s->len && s->keys[i] == k) return false;\n\
         \x20 if (s->len >= s->capacity) {\n\
         \x20   s->capacity = s->capacity ? s->capacity * 2 : 4;\n\
         \x20   s->keys = (int64_t*)realloc(s->keys, s->capacity * sizeof(int64_t));\n\
         \x20   if (!s->keys) abort();\n\
         \x20 }\n\
         \x20 if (i < s->len) memmove(s->keys + i + 1, s->keys + i, (s->len - i) * sizeof(int64_t));\n\
         \x20 s->keys[i] = k;\n\
         \x20 s->len++;\n\
         \x20 return true;\n\
         }\n\
         static INTENT_UNUSED bool intent_btreeset_i64_remove(intent_btreeset_i64* s, int64_t k) {\n\
         \x20 uint64_t i = intent_btreeset_i64__lower_bound(s, k);\n\
         \x20 if (i >= s->len || s->keys[i] != k) return false;\n\
         \x20 if (i + 1 < s->len) memmove(s->keys + i, s->keys + i + 1, (s->len - i - 1) * sizeof(int64_t));\n\
         \x20 s->len--;\n\
         \x20 return true;\n\
         }\n\n",
    );
    if emit_vec_dep {
        out.push_str(
            "/* Closure #346: range query. Appends every key k in\n\
             \x20 * [lo, hi] (inclusive) to `out` in sorted ascending\n\
             \x20 * order. Returns the number of keys appended.\n\
             \x20 * O(log n + matches). */\n\
             static INTENT_UNUSED int64_t intent_btreeset_i64_range(const intent_btreeset_i64* s, int64_t lo, int64_t hi, intent_vec_int64_t* out) {\n\
             \x20 if (lo > hi) return 0;\n\
             \x20 uint64_t i = intent_btreeset_i64__lower_bound(s, lo);\n\
             \x20 int64_t added = 0;\n\
             \x20 while (i < s->len && s->keys[i] <= hi) {\n\
             \x20   if (out->len >= out->capacity) {\n\
             \x20     uint64_t new_cap = out->capacity == 0 ? 8 : out->capacity * 2;\n\
             \x20     out->data = (int64_t*)realloc(out->data, (size_t)new_cap * sizeof(int64_t));\n\
             \x20     if (!out->data) abort();\n\
             \x20     out->capacity = new_cap;\n\
             \x20   }\n\
             \x20   out->data[out->len++] = s->keys[i];\n\
             \x20   added++; i++;\n\
             \x20 }\n\
             \x20 return added;\n\
             }\n\n",
        );
    }
    if has_option_i64 {
        out.push_str(
            "/* Closure #352: O(1) min / max on the sorted-Vec backing.\n\
             \x20 * Keys are stored ascending, so keys[0] = min and\n\
             \x20 * keys[len-1] = max. Returns Option<i64> (None on empty). */\n\
             static INTENT_UNUSED Enum_Option__i64 intent_btreeset_i64_min(const intent_btreeset_i64* s) {\n\
             \x20 Enum_Option__i64 r;\n\
             \x20 if (s->len == 0) { r.tag = 1; r.payload = 0; return r; }\n\
             \x20 r.tag = 0; r.payload = s->keys[0]; return r;\n\
             }\n\
             static INTENT_UNUSED Enum_Option__i64 intent_btreeset_i64_max(const intent_btreeset_i64* s) {\n\
             \x20 Enum_Option__i64 r;\n\
             \x20 if (s->len == 0) { r.tag = 1; r.payload = 0; return r; }\n\
             \x20 r.tag = 0; r.payload = s->keys[s->len - 1]; return r;\n\
             }\n\n",
        );
    }
}

/// Walk the program for any `BTreeMap<i64, i64>` type usage.
pub(crate) fn program_uses_i64_i64_btreemap(program: &TypedProgram) -> bool {
    fn ty_uses(ty: &Type) -> bool {
        match ty {
            Type::BTreeMap(k, v)
                if matches!(**k, Type::I64) && matches!(**v, Type::I64) =>
            {
                true
            }
            Type::Vec(inner) | Type::Ref(inner) | Type::RefMut(inner) => ty_uses(inner),
            _ => false,
        }
    }
    for f in &program.functions {
        if ty_uses(&f.return_type) {
            return true;
        }
        for p in &f.params {
            if ty_uses(&p.ty) {
                return true;
            }
        }
        for s in &f.body {
            if stmt_uses_i64_i64_btreemap(s) {
                return true;
            }
        }
    }
    false
}

fn stmt_uses_i64_i64_btreemap(stmt: &crate::ir::TypedStmt) -> bool {
    use crate::ir::TypedStmt as S;
    fn ty_uses(ty: &Type) -> bool {
        matches!(ty, Type::BTreeMap(k, v)
            if matches!(**k, Type::I64) && matches!(**v, Type::I64))
            || matches!(ty,
                Type::Vec(i) | Type::Ref(i) | Type::RefMut(i) if ty_uses(i))
    }
    match stmt {
        S::Let { ty, .. } | S::Reassign { ty, .. } | S::Drop { ty, .. } => ty_uses(ty),
        S::If { then_body, else_body, .. } => {
            then_body.iter().any(stmt_uses_i64_i64_btreemap)
                || else_body.iter().any(stmt_uses_i64_i64_btreemap)
        }
        S::While { body, .. } | S::For { body, .. } | S::ForIter { body, .. } => {
            body.iter().any(stmt_uses_i64_i64_btreemap)
        }
        _ => false,
    }
}

/// Data-structures roadmap Level 2 — BTreeMap<i64, i64> runtime
/// helpers (closure #307). v1 backed by parallel sorted `keys`
/// + `values` Vecs. Binary-search lower_bound for lookup (O(log
/// n)), memmove shift for insert / remove (O(n)). Naturally
/// sorted iteration order. `btreemap_get` / `btreemap_insert`
/// / `btreemap_remove` return `Option<i64>` and so are gated on
/// the Option__i64 enum being registered. `_contains_key` /
/// `_len` are always emitted.
fn emit_intent_btreemap_helpers_c_body(out: &mut String, has_option_i64: bool, emit_vec_dep: bool) {
    out.push_str(
        "typedef struct { int64_t* keys; int64_t* values; uint64_t len; uint64_t capacity; } intent_btreemap_i64_i64;\n\
         static INTENT_UNUSED intent_btreemap_i64_i64 intent_btreemap_i64_i64_new(void) {\n\
         \x20 intent_btreemap_i64_i64 m;\n\
         \x20 m.keys = (int64_t*)0; m.values = (int64_t*)0; m.len = 0; m.capacity = 0;\n\
         \x20 return m;\n\
         }\n\
         static INTENT_UNUSED void intent_btreemap_i64_i64_drop(intent_btreemap_i64_i64* m) {\n\
         \x20 if (m->keys) free(m->keys);\n\
         \x20 if (m->values) free(m->values);\n\
         \x20 m->keys = (int64_t*)0; m->values = (int64_t*)0; m->len = 0; m->capacity = 0;\n\
         }\n\
         static INTENT_UNUSED uint64_t intent_btreemap_i64_i64__lower_bound(const intent_btreemap_i64_i64* m, int64_t k) {\n\
         \x20 uint64_t lo = 0; uint64_t hi = m->len;\n\
         \x20 while (lo < hi) {\n\
         \x20   uint64_t mid = lo + (hi - lo) / 2;\n\
         \x20   if (m->keys[mid] < k) lo = mid + 1; else hi = mid;\n\
         \x20 }\n\
         \x20 return lo;\n\
         }\n\
         static INTENT_UNUSED bool intent_btreemap_i64_i64_contains_key(const intent_btreemap_i64_i64* m, int64_t k) {\n\
         \x20 uint64_t i = intent_btreemap_i64_i64__lower_bound(m, k);\n\
         \x20 return i < m->len && m->keys[i] == k;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_btreemap_i64_i64_len(const intent_btreemap_i64_i64* m) {\n\
         \x20 return (int64_t)m->len;\n\
         }\n\
         /* Closure #353: clear() — free both parallel key/value\n\
          * buffers and reset to empty. Returns prior len. */\n\
         static INTENT_UNUSED int64_t intent_btreemap_i64_i64_clear(intent_btreemap_i64_i64* m) {\n\
         \x20 int64_t prior = (int64_t)m->len;\n\
         \x20 if (m->keys) free(m->keys);\n\
         \x20 if (m->values) free(m->values);\n\
         \x20 m->keys = (int64_t*)0;\n\
         \x20 m->values = (int64_t*)0;\n\
         \x20 m->len = 0;\n\
         \x20 m->capacity = 0;\n\
         \x20 return prior;\n\
         }\n",
    );
    if has_option_i64 {
        out.push_str(
            "static INTENT_UNUSED Enum_Option__i64 intent_btreemap_i64_i64_get(const intent_btreemap_i64_i64* m, int64_t k) {\n\
             \x20 Enum_Option__i64 r;\n\
             \x20 uint64_t i = intent_btreemap_i64_i64__lower_bound(m, k);\n\
             \x20 if (i < m->len && m->keys[i] == k) { r.tag = 0; r.payload = m->values[i]; return r; }\n\
             \x20 r.tag = 1; r.payload = 0; return r;\n\
             }\n\
             static INTENT_UNUSED Enum_Option__i64 intent_btreemap_i64_i64_insert(intent_btreemap_i64_i64* m, int64_t k, int64_t v) {\n\
             \x20 Enum_Option__i64 r;\n\
             \x20 uint64_t i = intent_btreemap_i64_i64__lower_bound(m, k);\n\
             \x20 if (i < m->len && m->keys[i] == k) {\n\
             \x20   r.tag = 0; r.payload = m->values[i];\n\
             \x20   m->values[i] = v;\n\
             \x20   return r;\n\
             \x20 }\n\
             \x20 if (m->len >= m->capacity) {\n\
             \x20   m->capacity = m->capacity ? m->capacity * 2 : 4;\n\
             \x20   m->keys = (int64_t*)realloc(m->keys, m->capacity * sizeof(int64_t));\n\
             \x20   m->values = (int64_t*)realloc(m->values, m->capacity * sizeof(int64_t));\n\
             \x20   if (!m->keys || !m->values) abort();\n\
             \x20 }\n\
             \x20 if (i < m->len) {\n\
             \x20   memmove(m->keys + i + 1, m->keys + i, (m->len - i) * sizeof(int64_t));\n\
             \x20   memmove(m->values + i + 1, m->values + i, (m->len - i) * sizeof(int64_t));\n\
             \x20 }\n\
             \x20 m->keys[i] = k; m->values[i] = v;\n\
             \x20 m->len++;\n\
             \x20 r.tag = 1; r.payload = 0; return r;\n\
             }\n\
             static INTENT_UNUSED Enum_Option__i64 intent_btreemap_i64_i64_remove(intent_btreemap_i64_i64* m, int64_t k) {\n\
             \x20 Enum_Option__i64 r;\n\
             \x20 uint64_t i = intent_btreemap_i64_i64__lower_bound(m, k);\n\
             \x20 if (i >= m->len || m->keys[i] != k) { r.tag = 1; r.payload = 0; return r; }\n\
             \x20 r.tag = 0; r.payload = m->values[i];\n\
             \x20 if (i + 1 < m->len) {\n\
             \x20   memmove(m->keys + i, m->keys + i + 1, (m->len - i - 1) * sizeof(int64_t));\n\
             \x20   memmove(m->values + i, m->values + i + 1, (m->len - i - 1) * sizeof(int64_t));\n\
             \x20 }\n\
             \x20 m->len--;\n\
             \x20 return r;\n\
             }\n\
             /* Closure #352: O(1) min / max key on the sorted-Vec\n\
             \x20 * backing. keys[0] = smallest key, keys[len-1] = largest.\n\
             \x20 * Returns Option<i64> (None on empty). */\n\
             static INTENT_UNUSED Enum_Option__i64 intent_btreemap_i64_i64_min_key(const intent_btreemap_i64_i64* m) {\n\
             \x20 Enum_Option__i64 r;\n\
             \x20 if (m->len == 0) { r.tag = 1; r.payload = 0; return r; }\n\
             \x20 r.tag = 0; r.payload = m->keys[0]; return r;\n\
             }\n\
             static INTENT_UNUSED Enum_Option__i64 intent_btreemap_i64_i64_max_key(const intent_btreemap_i64_i64* m) {\n\
             \x20 Enum_Option__i64 r;\n\
             \x20 if (m->len == 0) { r.tag = 1; r.payload = 0; return r; }\n\
             \x20 r.tag = 0; r.payload = m->keys[m->len - 1]; return r;\n\
             }\n\n",
        );
    }
    if emit_vec_dep {
        out.push_str(
            "/* Closure #346: range queries on a BTreeMap. Two\n\
             \x20 * parallel helpers: range_keys appends every key k in\n\
             \x20 * [lo, hi] to `out`; range_values appends each\n\
             \x20 * corresponding value (parallel order). Returns the\n\
             \x20 * number of entries appended. O(log n + matches). */\n\
             static INTENT_UNUSED int64_t intent_btreemap_i64_i64_range_keys(const intent_btreemap_i64_i64* m, int64_t lo, int64_t hi, intent_vec_int64_t* out) {\n\
             \x20 if (lo > hi) return 0;\n\
             \x20 uint64_t i = intent_btreemap_i64_i64__lower_bound(m, lo);\n\
             \x20 int64_t added = 0;\n\
             \x20 while (i < m->len && m->keys[i] <= hi) {\n\
             \x20   if (out->len >= out->capacity) {\n\
             \x20     uint64_t new_cap = out->capacity == 0 ? 8 : out->capacity * 2;\n\
             \x20     out->data = (int64_t*)realloc(out->data, (size_t)new_cap * sizeof(int64_t));\n\
             \x20     if (!out->data) abort();\n\
             \x20     out->capacity = new_cap;\n\
             \x20   }\n\
             \x20   out->data[out->len++] = m->keys[i];\n\
             \x20   added++; i++;\n\
             \x20 }\n\
             \x20 return added;\n\
             }\n\
             static INTENT_UNUSED int64_t intent_btreemap_i64_i64_range_values(const intent_btreemap_i64_i64* m, int64_t lo, int64_t hi, intent_vec_int64_t* out) {\n\
             \x20 if (lo > hi) return 0;\n\
             \x20 uint64_t i = intent_btreemap_i64_i64__lower_bound(m, lo);\n\
             \x20 int64_t added = 0;\n\
             \x20 while (i < m->len && m->keys[i] <= hi) {\n\
             \x20   if (out->len >= out->capacity) {\n\
             \x20     uint64_t new_cap = out->capacity == 0 ? 8 : out->capacity * 2;\n\
             \x20     out->data = (int64_t*)realloc(out->data, (size_t)new_cap * sizeof(int64_t));\n\
             \x20     if (!out->data) abort();\n\
             \x20     out->capacity = new_cap;\n\
             \x20   }\n\
             \x20   out->data[out->len++] = m->values[i];\n\
             \x20   added++; i++;\n\
             \x20 }\n\
             \x20 return added;\n\
             }\n\n",
        );
    }
}

/// Walk the program for any `UnionFind` type usage. Closure #325.
pub(crate) fn program_uses_union_find(program: &TypedProgram) -> bool {
    fn ty_uses(ty: &Type) -> bool {
        match ty {
            Type::UnionFind => true,
            Type::Vec(inner) | Type::Ref(inner) | Type::RefMut(inner) => ty_uses(inner),
            _ => false,
        }
    }
    for f in &program.functions {
        if ty_uses(&f.return_type) {
            return true;
        }
        for p in &f.params {
            if ty_uses(&p.ty) {
                return true;
            }
        }
        for s in &f.body {
            if stmt_uses_union_find(s) {
                return true;
            }
        }
    }
    false
}

fn stmt_uses_union_find(stmt: &crate::ir::TypedStmt) -> bool {
    use crate::ir::TypedStmt as S;
    fn ty_uses(ty: &Type) -> bool {
        matches!(ty, Type::UnionFind)
            || matches!(ty,
                Type::Vec(i) | Type::Ref(i) | Type::RefMut(i) if ty_uses(i))
    }
    match stmt {
        S::Let { ty, .. } | S::Reassign { ty, .. } | S::Drop { ty, .. } => ty_uses(ty),
        S::If { then_body, else_body, .. } => {
            then_body.iter().any(stmt_uses_union_find)
                || else_body.iter().any(stmt_uses_union_find)
        }
        S::While { body, .. } | S::For { body, .. } | S::ForIter { body, .. } => {
            body.iter().any(stmt_uses_union_find)
        }
        _ => false,
    }
}

/// Data-structures roadmap Level 4 #1 — Union-Find runtime
/// helpers (closure #325). v1 fixed i64 element type. Backing:
/// parallel `parent` + `rank` i64 arrays. Find uses iterative
/// path compression; union uses union-by-rank. `count` tracks
/// the number of distinct sets — decremented on each
/// successful merge.
fn emit_intent_union_find_helpers_c_body(out: &mut String) {
    out.push_str(
        "typedef struct { int64_t* parent; int64_t* rank; uint64_t n; uint64_t sets; } intent_union_find;\n\
         static INTENT_UNUSED intent_union_find intent_union_find_new(int64_t n) {\n\
         \x20 intent_union_find uf;\n\
         \x20 if (n < 0) n = 0;\n\
         \x20 uf.n = (uint64_t)n;\n\
         \x20 uf.sets = (uint64_t)n;\n\
         \x20 if (n == 0) {\n\
         \x20   uf.parent = (int64_t*)0; uf.rank = (int64_t*)0; return uf;\n\
         \x20 }\n\
         \x20 uf.parent = (int64_t*)malloc((uint64_t)n * sizeof(int64_t));\n\
         \x20 uf.rank = (int64_t*)calloc((uint64_t)n, sizeof(int64_t));\n\
         \x20 if (!uf.parent || !uf.rank) abort();\n\
         \x20 for (int64_t i = 0; i < n; i++) uf.parent[i] = i;\n\
         \x20 return uf;\n\
         }\n\
         static INTENT_UNUSED void intent_union_find_drop(intent_union_find* uf) {\n\
         \x20 if (uf->parent) free(uf->parent);\n\
         \x20 if (uf->rank) free(uf->rank);\n\
         \x20 uf->parent = (int64_t*)0; uf->rank = (int64_t*)0;\n\
         \x20 uf->n = 0; uf->sets = 0;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_union_find_find(intent_union_find* uf, int64_t x) {\n\
         \x20 if (x < 0 || (uint64_t)x >= uf->n) return x;\n\
         \x20 /* Walk to the root. */\n\
         \x20 int64_t r = x;\n\
         \x20 while (uf->parent[r] != r) r = uf->parent[r];\n\
         \x20 /* Path-compress: point every node on the walk\n\
         \x20  * straight at the root. */\n\
         \x20 int64_t cur = x;\n\
         \x20 while (uf->parent[cur] != r) {\n\
         \x20   int64_t next = uf->parent[cur];\n\
         \x20   uf->parent[cur] = r;\n\
         \x20   cur = next;\n\
         \x20 }\n\
         \x20 return r;\n\
         }\n\
         static INTENT_UNUSED bool intent_union_find_union(intent_union_find* uf, int64_t a, int64_t b) {\n\
         \x20 int64_t ra = intent_union_find_find(uf, a);\n\
         \x20 int64_t rb = intent_union_find_find(uf, b);\n\
         \x20 if (ra == rb) return false;\n\
         \x20 /* Union-by-rank: shorter tree becomes child. */\n\
         \x20 if (uf->rank[ra] < uf->rank[rb]) {\n\
         \x20   uf->parent[ra] = rb;\n\
         \x20 } else if (uf->rank[ra] > uf->rank[rb]) {\n\
         \x20   uf->parent[rb] = ra;\n\
         \x20 } else {\n\
         \x20   uf->parent[rb] = ra;\n\
         \x20   uf->rank[ra] += 1;\n\
         \x20 }\n\
         \x20 if (uf->sets > 0) uf->sets -= 1;\n\
         \x20 return true;\n\
         }\n\
         static INTENT_UNUSED bool intent_union_find_connected(intent_union_find* uf, int64_t a, int64_t b) {\n\
         \x20 return intent_union_find_find(uf, a) == intent_union_find_find(uf, b);\n\
         }\n\
         static INTENT_UNUSED int64_t intent_union_find_count(const intent_union_find* uf) {\n\
         \x20 return (int64_t)uf->sets;\n\
         }\n\
         /* Closure #355: clear() — reset to all-singletons state.\n\
          * Each parent[i] = i, rank[i] = 0, sets = n. Keeps n\n\
          * (construction-time size) so the structure stays usable.\n\
          * Returns prior set count (uf.sets before reset). */\n\
         static INTENT_UNUSED int64_t intent_union_find_clear(intent_union_find* uf) {\n\
         \x20 int64_t prior = (int64_t)uf->sets;\n\
         \x20 if (uf->parent && uf->n > 0) {\n\
         \x20   for (uint64_t i = 0; i < uf->n; i++) uf->parent[i] = (int64_t)i;\n\
         \x20 }\n\
         \x20 if (uf->rank && uf->n > 0) {\n\
         \x20   memset(uf->rank, 0, (size_t)(uf->n * sizeof(int64_t)));\n\
         \x20 }\n\
         \x20 uf->sets = uf->n;\n\
         \x20 return prior;\n\
         }\n\n",
    );
}

/// Walk the program for any `BinaryHeap<i64>` type usage. Closure #326.
pub(crate) fn program_uses_i64_binary_heap(program: &TypedProgram) -> bool {
    fn ty_uses(ty: &Type) -> bool {
        match ty {
            Type::BinaryHeap(element) if matches!(**element, Type::I64) => true,
            Type::Vec(inner) | Type::Ref(inner) | Type::RefMut(inner) => ty_uses(inner),
            _ => false,
        }
    }
    for f in &program.functions {
        if ty_uses(&f.return_type) {
            return true;
        }
        for p in &f.params {
            if ty_uses(&p.ty) {
                return true;
            }
        }
        for s in &f.body {
            if stmt_uses_i64_binary_heap(s) {
                return true;
            }
        }
    }
    false
}

fn stmt_uses_i64_binary_heap(stmt: &crate::ir::TypedStmt) -> bool {
    use crate::ir::TypedStmt as S;
    fn ty_uses(ty: &Type) -> bool {
        matches!(ty, Type::BinaryHeap(element) if matches!(**element, Type::I64))
            || matches!(ty,
                Type::Vec(i) | Type::Ref(i) | Type::RefMut(i) if ty_uses(i))
    }
    match stmt {
        S::Let { ty, .. } | S::Reassign { ty, .. } | S::Drop { ty, .. } => ty_uses(ty),
        S::If { then_body, else_body, .. } => {
            then_body.iter().any(stmt_uses_i64_binary_heap)
                || else_body.iter().any(stmt_uses_i64_binary_heap)
        }
        S::While { body, .. } | S::For { body, .. } | S::ForIter { body, .. } => {
            body.iter().any(stmt_uses_i64_binary_heap)
        }
        _ => false,
    }
}

/// Data-structures roadmap Level 4 #2 — BinaryHeap<i64> runtime
/// helpers (closure #326). Heap-ordered single buffer + len +
/// capacity. push sift-ups; pop sift-downs. Min-heap (root is
/// smallest). pop / peek return `Option<i64>` — gated on the
/// Option__i64 enum being registered. v1 i64 element only.
fn emit_intent_binary_heap_helpers_c_body(out: &mut String, has_option_i64: bool) {
    out.push_str(
        "typedef struct { int64_t* data; uint64_t len; uint64_t capacity; } intent_binary_heap_i64;\n\
         static INTENT_UNUSED intent_binary_heap_i64 intent_binary_heap_i64_new(void) {\n\
         \x20 intent_binary_heap_i64 h; h.data = (int64_t*)0; h.len = 0; h.capacity = 0; return h;\n\
         }\n\
         static INTENT_UNUSED void intent_binary_heap_i64_drop(intent_binary_heap_i64* h) {\n\
         \x20 if (h->data) free(h->data);\n\
         \x20 h->data = (int64_t*)0; h->len = 0; h->capacity = 0;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_binary_heap_i64_push(intent_binary_heap_i64* h, int64_t v) {\n\
         \x20 if (h->len >= h->capacity) {\n\
         \x20   h->capacity = h->capacity ? h->capacity * 2 : 4;\n\
         \x20   h->data = (int64_t*)realloc(h->data, h->capacity * sizeof(int64_t));\n\
         \x20   if (!h->data) abort();\n\
         \x20 }\n\
         \x20 uint64_t i = h->len;\n\
         \x20 h->data[i] = v;\n\
         \x20 h->len++;\n\
         \x20 /* Sift-up. */\n\
         \x20 while (i > 0) {\n\
         \x20   uint64_t p = (i - 1) / 2;\n\
         \x20   if (h->data[i] >= h->data[p]) break;\n\
         \x20   int64_t t = h->data[i]; h->data[i] = h->data[p]; h->data[p] = t;\n\
         \x20   i = p;\n\
         \x20 }\n\
         \x20 return (int64_t)h->len;\n\
         }\n\
         /* Closure #354: clear() — free the heap buffer, reset to\n\
          * empty. Returns prior len. */\n\
         static INTENT_UNUSED int64_t intent_binary_heap_i64_clear(intent_binary_heap_i64* h) {\n\
         \x20 int64_t prior = (int64_t)h->len;\n\
         \x20 if (h->data) free(h->data);\n\
         \x20 h->data = (int64_t*)0;\n\
         \x20 h->len = 0;\n\
         \x20 h->capacity = 0;\n\
         \x20 return prior;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_binary_heap_i64_len(const intent_binary_heap_i64* h) {\n\
         \x20 return (int64_t)h->len;\n\
         }\n",
    );
    if has_option_i64 {
        out.push_str(
            "static INTENT_UNUSED Enum_Option__i64 intent_binary_heap_i64_peek(const intent_binary_heap_i64* h) {\n\
             \x20 Enum_Option__i64 r;\n\
             \x20 if (h->len == 0) { r.tag = 1; r.payload = 0; return r; }\n\
             \x20 r.tag = 0; r.payload = h->data[0]; return r;\n\
             }\n\
             static INTENT_UNUSED Enum_Option__i64 intent_binary_heap_i64_pop(intent_binary_heap_i64* h) {\n\
             \x20 Enum_Option__i64 r;\n\
             \x20 if (h->len == 0) { r.tag = 1; r.payload = 0; return r; }\n\
             \x20 r.tag = 0; r.payload = h->data[0];\n\
             \x20 h->len--;\n\
             \x20 if (h->len > 0) {\n\
             \x20   h->data[0] = h->data[h->len];\n\
             \x20   /* Sift-down. */\n\
             \x20   uint64_t i = 0;\n\
             \x20   while (1) {\n\
             \x20     uint64_t l = 2 * i + 1;\n\
             \x20     uint64_t r2 = 2 * i + 2;\n\
             \x20     uint64_t smallest = i;\n\
             \x20     if (l < h->len && h->data[l] < h->data[smallest]) smallest = l;\n\
             \x20     if (r2 < h->len && h->data[r2] < h->data[smallest]) smallest = r2;\n\
             \x20     if (smallest == i) break;\n\
             \x20     int64_t t = h->data[i]; h->data[i] = h->data[smallest]; h->data[smallest] = t;\n\
             \x20     i = smallest;\n\
             \x20   }\n\
             \x20 }\n\
             \x20 return r;\n\
             }\n\n",
        );
    }
}

/// Walk the program for any `BloomFilter` type usage. Closure #327.
pub(crate) fn program_uses_bloom_filter(program: &TypedProgram) -> bool {
    fn ty_uses(ty: &Type) -> bool {
        match ty {
            Type::BloomFilter => true,
            Type::Vec(inner) | Type::Ref(inner) | Type::RefMut(inner) => ty_uses(inner),
            _ => false,
        }
    }
    for f in &program.functions {
        if ty_uses(&f.return_type) {
            return true;
        }
        for p in &f.params {
            if ty_uses(&p.ty) {
                return true;
            }
        }
        for s in &f.body {
            if stmt_uses_bloom_filter(s) {
                return true;
            }
        }
    }
    false
}

fn stmt_uses_bloom_filter(stmt: &crate::ir::TypedStmt) -> bool {
    use crate::ir::TypedStmt as S;
    fn ty_uses(ty: &Type) -> bool {
        matches!(ty, Type::BloomFilter)
            || matches!(ty,
                Type::Vec(i) | Type::Ref(i) | Type::RefMut(i) if ty_uses(i))
    }
    match stmt {
        S::Let { ty, .. } | S::Reassign { ty, .. } | S::Drop { ty, .. } => ty_uses(ty),
        S::If { then_body, else_body, .. } => {
            then_body.iter().any(stmt_uses_bloom_filter)
                || else_body.iter().any(stmt_uses_bloom_filter)
        }
        S::While { body, .. } | S::For { body, .. } | S::ForIter { body, .. } => {
            body.iter().any(stmt_uses_bloom_filter)
        }
        _ => false,
    }
}

/// Data-structures roadmap Level 4 #6 — BloomFilter runtime
/// helpers (closure #327). Fixed-size bit-array probed at
/// `num_hashes` positions per insert; multi-hash derived from
/// the FNV-1a `intent_hash_i64` builtin via two-hash double
/// hashing. False positives possible, false negatives
/// impossible. v1 keys are i64.
fn emit_intent_bloom_filter_helpers_c_body(out: &mut String) {
    out.push_str(
        "typedef struct { uint8_t* bits; int64_t num_bits; int64_t num_hashes; int64_t insert_count; } intent_bloom_filter;\n\
         static INTENT_UNUSED uint64_t intent_bloom_filter_hash2(int64_t x) {\n\
         \x20 uint64_t h = 0x84222325cbf29ce4ULL;\n\
         \x20 uint64_t u = (uint64_t)x;\n\
         \x20 for (int i = 0; i < 8; i++) {\n\
         \x20   h ^= (u >> (i * 8)) & 0xffULL;\n\
         \x20   h *= 0xc6a4a7935bd1e995ULL;\n\
         \x20 }\n\
         \x20 return h;\n\
         }\n\
         static INTENT_UNUSED intent_bloom_filter intent_bloom_filter_new(int64_t num_bits, int64_t num_hashes) {\n\
         \x20 intent_bloom_filter bf;\n\
         \x20 if (num_bits <= 0) num_bits = 64;\n\
         \x20 if (num_hashes <= 0) num_hashes = 1;\n\
         \x20 int64_t bytes = (num_bits + 7) / 8;\n\
         \x20 bf.bits = (uint8_t*)calloc((size_t)bytes, 1);\n\
         \x20 if (!bf.bits) abort();\n\
         \x20 bf.num_bits = bytes * 8;\n\
         \x20 bf.num_hashes = num_hashes;\n\
         \x20 bf.insert_count = 0;\n\
         \x20 return bf;\n\
         }\n\
         static INTENT_UNUSED void intent_bloom_filter_drop(intent_bloom_filter* bf) {\n\
         \x20 if (bf->bits) free(bf->bits);\n\
         \x20 bf->bits = (uint8_t*)0; bf->num_bits = 0; bf->num_hashes = 0; bf->insert_count = 0;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_bloom_filter_insert(intent_bloom_filter* bf, int64_t x) {\n\
         \x20 uint64_t h1 = intent_hash_i64(x);\n\
         \x20 uint64_t h2 = intent_bloom_filter_hash2(x);\n\
         \x20 if (h2 == 0) h2 = 1;\n\
         \x20 uint64_t nb = (uint64_t)bf->num_bits;\n\
         \x20 for (int64_t k = 0; k < bf->num_hashes; k++) {\n\
         \x20   uint64_t pos = (h1 + (uint64_t)k * h2) % nb;\n\
         \x20   bf->bits[pos >> 3] |= (uint8_t)(1u << (pos & 7));\n\
         \x20 }\n\
         \x20 bf->insert_count++;\n\
         \x20 return bf->insert_count;\n\
         }\n\
         static INTENT_UNUSED bool intent_bloom_filter_contains(const intent_bloom_filter* bf, int64_t x) {\n\
         \x20 uint64_t h1 = intent_hash_i64(x);\n\
         \x20 uint64_t h2 = intent_bloom_filter_hash2(x);\n\
         \x20 if (h2 == 0) h2 = 1;\n\
         \x20 uint64_t nb = (uint64_t)bf->num_bits;\n\
         \x20 for (int64_t k = 0; k < bf->num_hashes; k++) {\n\
         \x20   uint64_t pos = (h1 + (uint64_t)k * h2) % nb;\n\
         \x20   if (!(bf->bits[pos >> 3] & (uint8_t)(1u << (pos & 7)))) return false;\n\
         \x20 }\n\
         \x20 return true;\n\
         }\n\
         /* Closure #354: clear() — zero the bit array via memset,\n\
          * reset insert_count. Keeps num_bits/num_hashes config\n\
          * (set at construction time) so the filter stays usable.\n\
          * Returns prior insert_count. */\n\
         static INTENT_UNUSED int64_t intent_bloom_filter_clear(intent_bloom_filter* bf) {\n\
         \x20 int64_t prior = bf->insert_count;\n\
         \x20 if (bf->bits && bf->num_bits > 0) {\n\
         \x20   size_t bytes = (size_t)((bf->num_bits + 7) / 8);\n\
         \x20   memset(bf->bits, 0, bytes);\n\
         \x20 }\n\
         \x20 bf->insert_count = 0;\n\
         \x20 return prior;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_bloom_filter_len(const intent_bloom_filter* bf) {\n\
         \x20 return bf->num_bits;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_bloom_filter_count(const intent_bloom_filter* bf) {\n\
         \x20 return bf->insert_count;\n\
         }\n\n",
    );
}

/// Walk the program for any `Bst<i64>` type usage. Closure #328.
pub(crate) fn program_uses_i64_bst(program: &TypedProgram) -> bool {
    fn ty_uses(ty: &Type) -> bool {
        match ty {
            Type::Bst(element) if matches!(**element, Type::I64) => true,
            Type::Vec(inner) | Type::Ref(inner) | Type::RefMut(inner) => ty_uses(inner),
            _ => false,
        }
    }
    for f in &program.functions {
        if ty_uses(&f.return_type) {
            return true;
        }
        for p in &f.params {
            if ty_uses(&p.ty) {
                return true;
            }
        }
        for s in &f.body {
            if stmt_uses_i64_bst(s) {
                return true;
            }
        }
    }
    false
}

fn stmt_uses_i64_bst(stmt: &crate::ir::TypedStmt) -> bool {
    use crate::ir::TypedStmt as S;
    fn ty_uses(ty: &Type) -> bool {
        matches!(ty, Type::Bst(element) if matches!(**element, Type::I64))
            || matches!(ty,
                Type::Vec(i) | Type::Ref(i) | Type::RefMut(i) if ty_uses(i))
    }
    match stmt {
        S::Let { ty, .. } | S::Reassign { ty, .. } | S::Drop { ty, .. } => ty_uses(ty),
        S::If { then_body, else_body, .. } => {
            then_body.iter().any(stmt_uses_i64_bst)
                || else_body.iter().any(stmt_uses_i64_bst)
        }
        S::While { body, .. } | S::For { body, .. } | S::ForIter { body, .. } => {
            body.iter().any(stmt_uses_i64_bst)
        }
        _ => false,
    }
}

/// Data-structures roadmap Level 4 #3 — Bst<i64> runtime helpers
/// (closure #328, AVL balancing added in closure #332). Node
/// arena: parallel `keys` (i64) + `left`/`right` (i32 child
/// indices, -1 = no child) + `heights` (u8 per-node) arrays.
/// Insert and remove keep the tree AVL-balanced via iterative
/// path tracking + the four-case rebalance (LL/LR/RR/RL).
/// min/max return `Option<i64>` and gate on Option__i64.
fn emit_intent_bst_i64_helpers_c_body(out: &mut String, has_option_i64: bool) {
    out.push_str(
        "typedef struct { int64_t* keys; int32_t* left; int32_t* right; int64_t root; int64_t len; int64_t capacity; uint8_t* heights; } intent_bst_i64;\n\
         static INTENT_UNUSED intent_bst_i64 intent_bst_i64_new(void) {\n\
         \x20 intent_bst_i64 b;\n\
         \x20 b.keys = (int64_t*)0; b.left = (int32_t*)0; b.right = (int32_t*)0;\n\
         \x20 b.heights = (uint8_t*)0;\n\
         \x20 b.root = -1; b.len = 0; b.capacity = 0;\n\
         \x20 return b;\n\
         }\n\
         static INTENT_UNUSED void intent_bst_i64_drop(intent_bst_i64* b) {\n\
         \x20 if (b->keys) free(b->keys);\n\
         \x20 if (b->left) free(b->left);\n\
         \x20 if (b->right) free(b->right);\n\
         \x20 if (b->heights) free(b->heights);\n\
         \x20 b->keys = (int64_t*)0; b->left = (int32_t*)0; b->right = (int32_t*)0;\n\
         \x20 b->heights = (uint8_t*)0;\n\
         \x20 b->root = -1; b->len = 0; b->capacity = 0;\n\
         }\n\
         static INTENT_UNUSED void intent_bst_i64_grow(intent_bst_i64* b) {\n\
         \x20 int64_t new_cap = b->capacity ? b->capacity * 2 : 8;\n\
         \x20 b->keys    = (int64_t*)realloc(b->keys,    (size_t)new_cap * sizeof(int64_t));\n\
         \x20 b->left    = (int32_t*)realloc(b->left,    (size_t)new_cap * sizeof(int32_t));\n\
         \x20 b->right   = (int32_t*)realloc(b->right,   (size_t)new_cap * sizeof(int32_t));\n\
         \x20 b->heights = (uint8_t*)realloc(b->heights, (size_t)new_cap * sizeof(uint8_t));\n\
         \x20 if (!b->keys || !b->left || !b->right || !b->heights) abort();\n\
         \x20 b->capacity = new_cap;\n\
         }\n\
         static INTENT_UNUSED uint8_t intent_bst_i64_h(const intent_bst_i64* b, int32_t i) {\n\
         \x20 return (i == -1) ? 0 : b->heights[i];\n\
         }\n\
         static INTENT_UNUSED void intent_bst_i64_update_height(intent_bst_i64* b, int64_t node) {\n\
         \x20 uint8_t lh = intent_bst_i64_h(b, b->left[node]);\n\
         \x20 uint8_t rh = intent_bst_i64_h(b, b->right[node]);\n\
         \x20 b->heights[node] = (uint8_t)(1 + ((lh > rh) ? lh : rh));\n\
         }\n\
         static INTENT_UNUSED int64_t intent_bst_i64_rotate_right(intent_bst_i64* b, int64_t x) {\n\
         \x20 int64_t y = (int64_t)b->left[x];\n\
         \x20 int32_t y_right = b->right[y];\n\
         \x20 b->left[x] = y_right;\n\
         \x20 b->right[y] = (int32_t)x;\n\
         \x20 intent_bst_i64_update_height(b, x);\n\
         \x20 intent_bst_i64_update_height(b, y);\n\
         \x20 return y;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_bst_i64_rotate_left(intent_bst_i64* b, int64_t x) {\n\
         \x20 int64_t y = (int64_t)b->right[x];\n\
         \x20 int32_t y_left = b->left[y];\n\
         \x20 b->right[x] = y_left;\n\
         \x20 b->left[y] = (int32_t)x;\n\
         \x20 intent_bst_i64_update_height(b, x);\n\
         \x20 intent_bst_i64_update_height(b, y);\n\
         \x20 return y;\n\
         }\n\
         /* Rebalance the subtree rooted at `node` after a height\n\
          * change to one of its children, returning the new root\n\
          * of that subtree (may be unchanged). Caller is\n\
          * responsible for relinking it into the parent. */\n\
         static INTENT_UNUSED int64_t intent_bst_i64_rebalance(intent_bst_i64* b, int64_t node) {\n\
         \x20 intent_bst_i64_update_height(b, node);\n\
         \x20 int lh = (int)intent_bst_i64_h(b, b->left[node]);\n\
         \x20 int rh = (int)intent_bst_i64_h(b, b->right[node]);\n\
         \x20 int balance = lh - rh;\n\
         \x20 if (balance > 1) {\n\
         \x20   int64_t l = (int64_t)b->left[node];\n\
         \x20   int llh = (int)intent_bst_i64_h(b, b->left[l]);\n\
         \x20   int lrh = (int)intent_bst_i64_h(b, b->right[l]);\n\
         \x20   if (lrh > llh) {\n\
         \x20     b->left[node] = (int32_t)intent_bst_i64_rotate_left(b, l);\n\
         \x20   }\n\
         \x20   return intent_bst_i64_rotate_right(b, node);\n\
         \x20 }\n\
         \x20 if (balance < -1) {\n\
         \x20   int64_t r = (int64_t)b->right[node];\n\
         \x20   int rlh = (int)intent_bst_i64_h(b, b->left[r]);\n\
         \x20   int rrh = (int)intent_bst_i64_h(b, b->right[r]);\n\
         \x20   if (rlh > rrh) {\n\
         \x20     b->right[node] = (int32_t)intent_bst_i64_rotate_right(b, r);\n\
         \x20   }\n\
         \x20   return intent_bst_i64_rotate_left(b, node);\n\
         \x20 }\n\
         \x20 return node;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_bst_i64_emplace(intent_bst_i64* b, int64_t x) {\n\
         \x20 if (b->len >= b->capacity) intent_bst_i64_grow(b);\n\
         \x20 int64_t idx = b->len;\n\
         \x20 b->keys[idx] = x; b->left[idx] = -1; b->right[idx] = -1;\n\
         \x20 b->heights[idx] = 1;\n\
         \x20 b->len++;\n\
         \x20 return idx;\n\
         }\n\
         static INTENT_UNUSED bool intent_bst_i64_insert(intent_bst_i64* b, int64_t x) {\n\
         \x20 if (b->root == -1) {\n\
         \x20   b->root = intent_bst_i64_emplace(b, x);\n\
         \x20   return true;\n\
         \x20 }\n\
         \x20 /* Walk down, recording the search path. Depth bound is\n\
          * the AVL height of an n-node tree, well below 64 for any\n\
          * tree that fits in i32 child indices. */\n\
         \x20 int64_t path[64];\n\
         \x20 int8_t  path_dir[64];  /* 0 = went left, 1 = went right */\n\
         \x20 int     plen = 0;\n\
         \x20 int64_t cur = b->root;\n\
         \x20 while (1) {\n\
         \x20   int64_t k = b->keys[cur];\n\
         \x20   if (x == k) return false;\n\
         \x20   path[plen] = cur;\n\
         \x20   if (x < k) {\n\
         \x20     path_dir[plen++] = 0;\n\
         \x20     if (b->left[cur] == -1) {\n\
         \x20       int64_t new_idx = intent_bst_i64_emplace(b, x);\n\
         \x20       b->left[cur] = (int32_t)new_idx;\n\
         \x20       break;\n\
         \x20     }\n\
         \x20     cur = (int64_t)b->left[cur];\n\
         \x20   } else {\n\
         \x20     path_dir[plen++] = 1;\n\
         \x20     if (b->right[cur] == -1) {\n\
         \x20       int64_t new_idx = intent_bst_i64_emplace(b, x);\n\
         \x20       b->right[cur] = (int32_t)new_idx;\n\
         \x20       break;\n\
         \x20     }\n\
         \x20     cur = (int64_t)b->right[cur];\n\
         \x20   }\n\
         \x20 }\n\
         \x20 /* Walk back up: recompute heights, rotate where needed, \n\
          * and relink the rotated subtree root into the parent. */\n\
         \x20 for (int i = plen - 1; i >= 0; i--) {\n\
         \x20   int64_t node = path[i];\n\
         \x20   int64_t new_root = intent_bst_i64_rebalance(b, node);\n\
         \x20   if (i == 0) {\n\
         \x20     b->root = new_root;\n\
         \x20   } else {\n\
         \x20     int64_t parent = path[i - 1];\n\
         \x20     if (path_dir[i - 1] == 0) b->left[parent] = (int32_t)new_root;\n\
         \x20     else                       b->right[parent] = (int32_t)new_root;\n\
         \x20   }\n\
         \x20 }\n\
         \x20 return true;\n\
         }\n\
         static INTENT_UNUSED bool intent_bst_i64_contains(const intent_bst_i64* b, int64_t x) {\n\
         \x20 int64_t cur = b->root;\n\
         \x20 while (cur != -1) {\n\
         \x20   int64_t k = b->keys[cur];\n\
         \x20   if (x == k) return true;\n\
         \x20   cur = (x < k) ? (int64_t)b->left[cur] : (int64_t)b->right[cur];\n\
         \x20 }\n\
         \x20 return false;\n\
         }\n\
         /* Remove by key with AVL rebalance. Standard iterative\n\
          * algorithm: walk down recording the path, unlink the\n\
          * found node (in-order successor for two-children case),\n\
          * then walk back up the path rebalancing. Deleted arena\n\
          * slots stay tombstoned (no compaction). */\n\
         static INTENT_UNUSED bool intent_bst_i64_remove(intent_bst_i64* b, int64_t x) {\n\
         \x20 if (b->root == -1) return false;\n\
         \x20 int64_t path[64];\n\
         \x20 int8_t  path_dir[64];\n\
         \x20 int     plen = 0;\n\
         \x20 int64_t cur = b->root;\n\
         \x20 int     found_at = -1;\n\
         \x20 while (cur != -1) {\n\
         \x20   int64_t k = b->keys[cur];\n\
         \x20   if (x == k) { found_at = plen; break; }\n\
         \x20   path[plen] = cur;\n\
         \x20   path_dir[plen] = (x < k) ? 0 : 1;\n\
         \x20   plen++;\n\
         \x20   cur = (x < k) ? (int64_t)b->left[cur] : (int64_t)b->right[cur];\n\
         \x20 }\n\
         \x20 if (found_at < 0) return false;\n\
         \x20 /* `cur` is the found node; `plen` is its parent's index in path */\n\
         \x20 int32_t found_l = b->left[cur];\n\
         \x20 int32_t found_r = b->right[cur];\n\
         \x20 int64_t replacement;\n\
         \x20 if (found_l != -1 && found_r != -1) {\n\
         \x20   /* Two children: copy in-order successor's key up,\n\
          * then unlink the successor. The successor's path\n\
          * starts with the found node, then walks into the\n\
          * right subtree and as far left as possible. */\n\
         \x20   path[plen] = cur;\n\
         \x20   path_dir[plen] = 1;\n\
         \x20   plen++;\n\
         \x20   int64_t s = (int64_t)found_r;\n\
         \x20   while (b->left[s] != -1) {\n\
         \x20     path[plen] = s;\n\
         \x20     path_dir[plen] = 0;\n\
         \x20     plen++;\n\
         \x20     s = (int64_t)b->left[s];\n\
         \x20   }\n\
         \x20   b->keys[cur] = b->keys[s];\n\
         \x20   replacement = (int64_t)b->right[s];\n\
         \x20 } else {\n\
         \x20   replacement = (found_l != -1) ? (int64_t)found_l : (int64_t)found_r;\n\
         \x20 }\n\
         \x20 /* Relink the replacement into the parent (or the root). */\n\
         \x20 if (plen == 0) {\n\
         \x20   b->root = replacement;\n\
         \x20 } else {\n\
         \x20   int64_t parent = path[plen - 1];\n\
         \x20   if (path_dir[plen - 1] == 0) b->left[parent] = (int32_t)replacement;\n\
         \x20   else                          b->right[parent] = (int32_t)replacement;\n\
         \x20 }\n\
         \x20 b->len--;\n\
         \x20 /* Rebalance up the path. */\n\
         \x20 for (int i = plen - 1; i >= 0; i--) {\n\
         \x20   int64_t node = path[i];\n\
         \x20   int64_t new_root = intent_bst_i64_rebalance(b, node);\n\
         \x20   if (i == 0) {\n\
         \x20     b->root = new_root;\n\
         \x20   } else {\n\
         \x20     int64_t parent = path[i - 1];\n\
         \x20     if (path_dir[i - 1] == 0) b->left[parent] = (int32_t)new_root;\n\
         \x20     else                       b->right[parent] = (int32_t)new_root;\n\
         \x20   }\n\
         \x20 }\n\
         \x20 return true;\n\
         }\n\
         /* Closure #354: clear() — free all four parallel arrays\n\
          * (keys / left / right / heights), reset root=-1 and\n\
          * len=0. Returns prior len. */\n\
         static INTENT_UNUSED int64_t intent_bst_i64_clear(intent_bst_i64* b) {\n\
         \x20 int64_t prior = b->len;\n\
         \x20 if (b->keys) free(b->keys);\n\
         \x20 if (b->left) free(b->left);\n\
         \x20 if (b->right) free(b->right);\n\
         \x20 if (b->heights) free(b->heights);\n\
         \x20 b->keys = (int64_t*)0;\n\
         \x20 b->left = (int32_t*)0;\n\
         \x20 b->right = (int32_t*)0;\n\
         \x20 b->heights = (uint8_t*)0;\n\
         \x20 b->root = -1;\n\
         \x20 b->len = 0;\n\
         \x20 b->capacity = 0;\n\
         \x20 return prior;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_bst_i64_len(const intent_bst_i64* b) {\n\
         \x20 return b->len;\n\
         }\n",
    );
    if has_option_i64 {
        out.push_str(
            "static INTENT_UNUSED Enum_Option__i64 intent_bst_i64_min(const intent_bst_i64* b) {\n\
             \x20 Enum_Option__i64 r;\n\
             \x20 if (b->root == -1) { r.tag = 1; r.payload = 0; return r; }\n\
             \x20 int64_t cur = b->root;\n\
             \x20 while (b->left[cur] != -1) cur = (int64_t)b->left[cur];\n\
             \x20 r.tag = 0; r.payload = b->keys[cur]; return r;\n\
             }\n\
             static INTENT_UNUSED Enum_Option__i64 intent_bst_i64_max(const intent_bst_i64* b) {\n\
             \x20 Enum_Option__i64 r;\n\
             \x20 if (b->root == -1) { r.tag = 1; r.payload = 0; return r; }\n\
             \x20 int64_t cur = b->root;\n\
             \x20 while (b->right[cur] != -1) cur = (int64_t)b->right[cur];\n\
             \x20 r.tag = 0; r.payload = b->keys[cur]; return r;\n\
             }\n\n",
        );
    }
}

/// Walk the program for any `Graph` type usage. Closure #329.
pub(crate) fn program_uses_graph(program: &TypedProgram) -> bool {
    fn ty_uses(ty: &Type) -> bool {
        match ty {
            Type::Graph => true,
            Type::Vec(inner) | Type::Ref(inner) | Type::RefMut(inner) => ty_uses(inner),
            _ => false,
        }
    }
    for f in &program.functions {
        if ty_uses(&f.return_type) {
            return true;
        }
        for p in &f.params {
            if ty_uses(&p.ty) {
                return true;
            }
        }
        for s in &f.body {
            if stmt_uses_graph(s) {
                return true;
            }
        }
    }
    false
}

/// Closure #350: walk the program for any call to `str_split`.
/// Used to gate `intent_str_split` emission — the helper
/// references `intent_vec_owned_str` which is itself
/// element-type-gated. Both helpers must be in scope before
/// any caller, so emission is deferred to the body's Vec-
/// bundle pass.
pub(crate) fn program_uses_str_split(program: &TypedProgram) -> bool {
    use crate::ir::TypedExprKind as E;
    use crate::ir::TypedStmt as S;
    fn expr_uses(expr: &crate::ir::TypedExpr) -> bool {
        match &expr.kind {
            E::Call { name, args, .. } => {
                // Closure #379: str_join shares the
                // `intent_vec_owned_str` dependency with str_split.
                // Closure #381: str_lines also returns a
                // Vec<OwnedStr>, same dependency.
                if name == "str_split" || name == "str_join" || name == "str_lines" {
                    return true;
                }
                args.iter().any(expr_uses)
            }
            E::Unary { expr, .. } | E::Cast { expr, .. } => expr_uses(expr),
            E::Len { array, .. } => expr_uses(array),
            E::Binary { left, right, .. } => expr_uses(left) || expr_uses(right),
            E::CallIndirect { callee, args } => {
                expr_uses(callee) || args.iter().any(expr_uses)
            }
            E::ArrayLit { elements } => elements.iter().any(expr_uses),
            E::Index { array, index, .. } => expr_uses(array) || expr_uses(index),
            E::Tuple { elements } => elements.iter().any(expr_uses),
            E::TupleAccess { tuple, .. } => expr_uses(tuple),
            E::StructLit { fields, .. } => fields.iter().any(|(_, v)| expr_uses(v)),
            E::FieldAccess { object, .. } => expr_uses(object),
            E::EnumVariantWithPayload { payload, .. } => expr_uses(payload),
            E::IfExpr { cond, then_value, else_value } => {
                expr_uses(cond) || expr_uses(then_value) || expr_uses(else_value)
            }
            E::Match { scrutinee, arms } => {
                expr_uses(scrutinee) || arms.iter().any(|a| expr_uses(&a.body))
            }
            E::Block { stmts, tail } => {
                stmts.iter().any(stmt_walk) || expr_uses(tail)
            }
            _ => false,
        }
    }
    fn stmt_walk(s: &S) -> bool {
        match s {
            S::Let { expr, .. }
            | S::Reassign { expr, .. }
            | S::Return { expr }
            | S::Assert { expr, .. }
            | S::Prove { expr } => expr_uses(expr),
            S::Discard { expr } => expr_uses(expr),
            S::Print { items } => items.iter().any(|it| match it {
                crate::ir::TypedPrintItem::Expr(e) => expr_uses(e),
                _ => false,
            }),
            S::If { cond, then_body, else_body, .. } => {
                expr_uses(cond)
                    || then_body.iter().any(stmt_walk)
                    || else_body.iter().any(stmt_walk)
            }
            S::While { cond, body, .. } => {
                expr_uses(cond) || body.iter().any(stmt_walk)
            }
            S::For { start, end, body, .. } => {
                expr_uses(start) || expr_uses(end) || body.iter().any(stmt_walk)
            }
            S::ForIter { body, .. } => body.iter().any(stmt_walk),
            _ => false,
        }
    }
    for f in &program.functions {
        if f.body.iter().any(stmt_walk) {
            return true;
        }
    }
    false
}

/// Walk the program for any builtin that emits code referencing
/// the `intent_vec_int64_t` runtime struct: `graph_astar` /
/// `graph_topo_sort` (closures #334 / #335) and the BTreeSet /
/// BTreeMap range queries (closure #346). We only emit those
/// helpers when actually used — otherwise programs that use
/// Graph or BTree* without `Vec<i64>` would fail to compile.
pub(crate) fn program_uses_graph_vec_builtin(program: &TypedProgram) -> bool {
    use crate::ir::TypedExprKind as E;
    use crate::ir::TypedStmt as S;
    fn expr_uses(expr: &crate::ir::TypedExpr) -> bool {
        match &expr.kind {
            E::Call { name, args, .. } => {
                if name == "graph_astar"
                    || name == "graph_topo_sort"
                    || name == "btreeset_range"
                    || name == "btreemap_range_keys"
                    || name == "btreemap_range_values"
                    || name == "vec_range"
                    || name == "vec_repeat"
                    || name == "vec_extend"
                    || name == "vec_concat"
                    || name == "vec_reverse_copy"
                    || name == "vec_unique"
                    || name == "vec_iota"
                    || name == "vec_first"
                    || name == "vec_last"
                    || name == "vec_running_sum"
                    || name == "vec_cumulative_max"
                    || name == "vec_cumulative_min"
                    || name == "vec_running_product"
                    || name == "vec_running_xor"
                    || name == "vec_running_and"
                    || name == "vec_running_or"
                    || name == "vec_all_equal"
                    || name == "vec_is_sorted_asc"
                    || name == "vec_is_sorted_desc"
                    || name == "vec_is_palindrome"
                    || name == "vec_sliding_max"
                    || name == "vec_sliding_min"
                    || name == "vec_sliding_sum"
                    || name == "vec_sliding_product"
                    || name == "vec_abs"
                    || name == "vec_negate"
                    || name == "vec_signum"
                    || name == "vec_square"
                    || name == "vec_add_scalar"
                    || name == "vec_sub_scalar"
                    || name == "vec_mul_scalar"
                    || name == "vec_div_scalar"
                    || name == "vec_eq_mask"
                    || name == "vec_ne_mask"
                    || name == "vec_lt_mask"
                    || name == "vec_le_mask"
                    || name == "vec_gt_mask"
                    || name == "vec_ge_mask"
                    || name == "vec_min_with_scalar"
                    || name == "vec_max_with_scalar"
                    || name == "vec_clamp_scalar"
                    || name == "vec_add_pairwise"
                    || name == "vec_sub_pairwise"
                    || name == "vec_mul_pairwise"
                    || name == "vec_min_pairwise"
                    || name == "vec_max_pairwise"
                    || name == "vec_mod_scalar"
                    || name == "vec_pow_scalar"
                    || name == "vec_shl_scalar"
                    || name == "vec_shr_scalar"
                    || name == "vec_rotate_left"
                    || name == "vec_rotate_right"
                    || name == "vec_shift_left"
                    || name == "vec_shift_right"
                    || name == "vec_subset_of"
                    || name == "vec_disjoint"
                    || name == "vec_equal_set"
                    || name == "vec_equal_seq"
                    || name == "vec_diff"
                    || name == "vec_pad_left"
                    || name == "vec_pad_right"
                    || name == "vec_replace_value"
                    || name == "vec_count_distinct"
                    || name == "vec_indices_of_value"
                    || name == "vec_dedup_consecutive"
                    || name == "vec_mean"
                    || name == "vec_merge_sorted"
                    || name == "vec_insert_sorted"
                    || name == "vec_is_sorted_unique"
                    || name == "vec_range_span"
                    || name == "vec_mode"
                    || name == "vec_kth_smallest"
                    || name == "vec_median"
                    || name == "vec_running_mean"
                    || name == "vec_intersperse"
                    || name == "vec_dot"
                    || name == "vec_intersect"
                    || name == "vec_difference"
                    || name == "vec_union"
                    || name == "str_chars"
                {
                    return true;
                }
                args.iter().any(expr_uses)
            }
            E::Unary { expr, .. } | E::Cast { expr, .. } => expr_uses(expr),
            E::Len { array, .. } => expr_uses(array),
            E::Binary { left, right, .. } => expr_uses(left) || expr_uses(right),
            E::CallIndirect { callee, args } => {
                expr_uses(callee) || args.iter().any(expr_uses)
            }
            E::ArrayLit { elements } => elements.iter().any(expr_uses),
            E::Index { array, index, .. } => expr_uses(array) || expr_uses(index),
            E::Tuple { elements } => elements.iter().any(expr_uses),
            E::TupleAccess { tuple, .. } => expr_uses(tuple),
            E::StructLit { fields, .. } => fields.iter().any(|(_, v)| expr_uses(v)),
            E::FieldAccess { object, .. } => expr_uses(object),
            E::EnumVariantWithPayload { payload, .. } => expr_uses(payload),
            E::IfExpr { cond, then_value, else_value } => {
                expr_uses(cond) || expr_uses(then_value) || expr_uses(else_value)
            }
            E::Match { scrutinee, arms } => {
                expr_uses(scrutinee) || arms.iter().any(|a| expr_uses(&a.body))
            }
            E::Block { stmts, tail } => {
                stmts.iter().any(stmt_walk) || expr_uses(tail)
            }
            _ => false,
        }
    }
    fn stmt_walk(s: &S) -> bool {
        match s {
            S::Let { expr, .. }
            | S::Reassign { expr, .. }
            | S::Return { expr }
            | S::Assert { expr, .. }
            | S::Prove { expr } => expr_uses(expr),
            S::Discard { expr } => expr_uses(expr),
            S::Print { items } => items.iter().any(|it| match it {
                crate::ir::TypedPrintItem::Expr(e) => expr_uses(e),
                _ => false,
            }),
            S::If { cond, then_body, else_body, .. } => {
                expr_uses(cond)
                    || then_body.iter().any(stmt_walk)
                    || else_body.iter().any(stmt_walk)
            }
            S::While { cond, body, .. } => {
                expr_uses(cond) || body.iter().any(stmt_walk)
            }
            S::For { start, end, body, .. } => {
                expr_uses(start) || expr_uses(end) || body.iter().any(stmt_walk)
            }
            S::ForIter { body, .. } => {
                body.iter().any(stmt_walk)
            }
            _ => false,
        }
    }
    for f in &program.functions {
        if f.body.iter().any(stmt_walk) {
            return true;
        }
    }
    false
}

fn stmt_uses_graph(stmt: &crate::ir::TypedStmt) -> bool {
    use crate::ir::TypedStmt as S;
    fn ty_uses(ty: &Type) -> bool {
        matches!(ty, Type::Graph)
            || matches!(ty,
                Type::Vec(i) | Type::Ref(i) | Type::RefMut(i) if ty_uses(i))
    }
    match stmt {
        S::Let { ty, .. } | S::Reassign { ty, .. } | S::Drop { ty, .. } => ty_uses(ty),
        S::If { then_body, else_body, .. } => {
            then_body.iter().any(stmt_uses_graph)
                || else_body.iter().any(stmt_uses_graph)
        }
        S::While { body, .. } | S::For { body, .. } | S::ForIter { body, .. } => {
            body.iter().any(stmt_uses_graph)
        }
        _ => false,
    }
}

/// Data-structures roadmap Level 4 #5 — Graph runtime helpers
/// (closure #329). Weighted directed graph storing per-edge
/// parallel arrays (edge_src, edge_dst : i32; edge_weight : i64).
/// BFS / DFS reachability use heap-allocated visited + queue/stack
/// arrays. Dijkstra uses an O(V^2) inner loop (linear scan for
/// next-min) — no dependency on BinaryHeap. graph_dijkstra
/// returns Option<i64>, gated on Option__i64 being registered.
fn emit_intent_graph_helpers_c_body(out: &mut String, has_option_i64: bool, emit_vec_dep: bool) {
    out.push_str(
        "typedef struct { int64_t num_nodes; int32_t* edge_src; int32_t* edge_dst; int64_t* edge_weight; int64_t num_edges; int64_t edge_capacity; int32_t* adj_start; int32_t* adj_csr_dst; int64_t* adj_csr_weight; int32_t* rev_adj_start; int32_t* rev_adj_csr_src; int64_t* rev_adj_csr_weight; } intent_graph;\n\
         static INTENT_UNUSED intent_graph intent_graph_new(int64_t n) {\n\
         \x20 intent_graph g;\n\
         \x20 g.num_nodes = (n < 0) ? 0 : n;\n\
         \x20 g.edge_src = (int32_t*)0; g.edge_dst = (int32_t*)0; g.edge_weight = (int64_t*)0;\n\
         \x20 g.num_edges = 0; g.edge_capacity = 0;\n\
         \x20 g.adj_start = (int32_t*)0; g.adj_csr_dst = (int32_t*)0; g.adj_csr_weight = (int64_t*)0;\n\
         \x20 g.rev_adj_start = (int32_t*)0; g.rev_adj_csr_src = (int32_t*)0; g.rev_adj_csr_weight = (int64_t*)0;\n\
         \x20 return g;\n\
         }\n\
         /* Closure #336 + #338: invalidate both CSR caches (forward and\n\
          * reverse). Called by add_edge and at the start of drop. NULL\n\
          * adj_start / rev_adj_start = corresponding cache invalid. */\n\
         static INTENT_UNUSED void intent_graph_invalidate_csr(intent_graph* g) {\n\
         \x20 if (g->adj_start) free(g->adj_start);\n\
         \x20 if (g->adj_csr_dst) free(g->adj_csr_dst);\n\
         \x20 if (g->adj_csr_weight) free(g->adj_csr_weight);\n\
         \x20 g->adj_start = (int32_t*)0; g->adj_csr_dst = (int32_t*)0; g->adj_csr_weight = (int64_t*)0;\n\
         \x20 if (g->rev_adj_start) free(g->rev_adj_start);\n\
         \x20 if (g->rev_adj_csr_src) free(g->rev_adj_csr_src);\n\
         \x20 if (g->rev_adj_csr_weight) free(g->rev_adj_csr_weight);\n\
         \x20 g->rev_adj_start = (int32_t*)0; g->rev_adj_csr_src = (int32_t*)0; g->rev_adj_csr_weight = (int64_t*)0;\n\
         }\n\
         /* Closure #336: build the CSR adjacency cache on first use.\n\
          * Allocates adj_start[num_nodes+1] + adj_csr_dst[num_edges]\n\
          * + adj_csr_weight[num_edges]. Two-pass: count out-degrees,\n\
          * compute prefix sums, then bucket-scatter edges. The graph\n\
          * is logically const here — we cast away const because the\n\
          * CSR cache is mutable cache state. */\n\
         static INTENT_UNUSED void intent_graph_build_csr_if_needed(const intent_graph* g_ro) {\n\
         \x20 intent_graph* g = (intent_graph*)g_ro;\n\
         \x20 if (g->adj_start) return;\n\
         \x20 if (g->num_nodes <= 0) return;\n\
         \x20 int64_t nn = g->num_nodes;\n\
         \x20 int64_t ne = g->num_edges;\n\
         \x20 g->adj_start = (int32_t*)malloc((size_t)(nn + 1) * sizeof(int32_t));\n\
         \x20 if (!g->adj_start) abort();\n\
         \x20 for (int64_t i = 0; i <= nn; i++) g->adj_start[i] = 0;\n\
         \x20 /* Count out-degrees in adj_start[s+1]. */\n\
         \x20 for (int64_t e = 0; e < ne; e++) {\n\
         \x20   int32_t s = g->edge_src[e];\n\
         \x20   if (s >= 0 && (int64_t)s < nn) g->adj_start[s + 1]++;\n\
         \x20 }\n\
         \x20 /* Prefix sum to convert counts to start offsets. */\n\
         \x20 for (int64_t i = 1; i <= nn; i++) g->adj_start[i] += g->adj_start[i - 1];\n\
         \x20 int64_t total = (int64_t)g->adj_start[nn];\n\
         \x20 if (total > 0) {\n\
         \x20   g->adj_csr_dst = (int32_t*)malloc((size_t)total * sizeof(int32_t));\n\
         \x20   g->adj_csr_weight = (int64_t*)malloc((size_t)total * sizeof(int64_t));\n\
         \x20   if (!g->adj_csr_dst || !g->adj_csr_weight) abort();\n\
         \x20 }\n\
         \x20 /* Per-source bucket cursor: starts at adj_start[s]. */\n\
         \x20 int32_t* cur = (int32_t*)malloc((size_t)nn * sizeof(int32_t));\n\
         \x20 if (!cur) abort();\n\
         \x20 for (int64_t i = 0; i < nn; i++) cur[i] = g->adj_start[i];\n\
         \x20 for (int64_t e = 0; e < ne; e++) {\n\
         \x20   int32_t s = g->edge_src[e];\n\
         \x20   if (s < 0 || (int64_t)s >= nn) continue;\n\
         \x20   int32_t pos = cur[s]++;\n\
         \x20   g->adj_csr_dst[pos] = g->edge_dst[e];\n\
         \x20   g->adj_csr_weight[pos] = g->edge_weight[e];\n\
         \x20 }\n\
         \x20 free(cur);\n\
         }\n\
         /* Closure #338: build the REVERSE CSR adjacency cache.\n\
          * Mirrors build_csr_if_needed but keyed on destination —\n\
          * rev_adj_start[v] is the offset into rev_adj_csr_src where\n\
          * node v's incoming edges begin; the entries record the\n\
          * source of each incoming edge plus its weight. Used by\n\
          * Prim's undirected interpretation to walk \"the other end\"\n\
          * of every edge incident to a node. */\n\
         static INTENT_UNUSED void intent_graph_build_rev_csr_if_needed(const intent_graph* g_ro) {\n\
         \x20 intent_graph* g = (intent_graph*)g_ro;\n\
         \x20 if (g->rev_adj_start) return;\n\
         \x20 if (g->num_nodes <= 0) return;\n\
         \x20 int64_t nn = g->num_nodes;\n\
         \x20 int64_t ne = g->num_edges;\n\
         \x20 g->rev_adj_start = (int32_t*)malloc((size_t)(nn + 1) * sizeof(int32_t));\n\
         \x20 if (!g->rev_adj_start) abort();\n\
         \x20 for (int64_t i = 0; i <= nn; i++) g->rev_adj_start[i] = 0;\n\
         \x20 /* Count in-degrees in rev_adj_start[d+1]. */\n\
         \x20 for (int64_t e = 0; e < ne; e++) {\n\
         \x20   int32_t d = g->edge_dst[e];\n\
         \x20   if (d >= 0 && (int64_t)d < nn) g->rev_adj_start[d + 1]++;\n\
         \x20 }\n\
         \x20 for (int64_t i = 1; i <= nn; i++) g->rev_adj_start[i] += g->rev_adj_start[i - 1];\n\
         \x20 int64_t total = (int64_t)g->rev_adj_start[nn];\n\
         \x20 if (total > 0) {\n\
         \x20   g->rev_adj_csr_src = (int32_t*)malloc((size_t)total * sizeof(int32_t));\n\
         \x20   g->rev_adj_csr_weight = (int64_t*)malloc((size_t)total * sizeof(int64_t));\n\
         \x20   if (!g->rev_adj_csr_src || !g->rev_adj_csr_weight) abort();\n\
         \x20 }\n\
         \x20 int32_t* cur = (int32_t*)malloc((size_t)nn * sizeof(int32_t));\n\
         \x20 if (!cur) abort();\n\
         \x20 for (int64_t i = 0; i < nn; i++) cur[i] = g->rev_adj_start[i];\n\
         \x20 for (int64_t e = 0; e < ne; e++) {\n\
         \x20   int32_t d = g->edge_dst[e];\n\
         \x20   if (d < 0 || (int64_t)d >= nn) continue;\n\
         \x20   int32_t pos = cur[d]++;\n\
         \x20   g->rev_adj_csr_src[pos] = g->edge_src[e];\n\
         \x20   g->rev_adj_csr_weight[pos] = g->edge_weight[e];\n\
         \x20 }\n\
         \x20 free(cur);\n\
         }\n\
         static INTENT_UNUSED void intent_graph_drop(intent_graph* g) {\n\
         \x20 intent_graph_invalidate_csr(g);\n\
         \x20 if (g->edge_src) free(g->edge_src);\n\
         \x20 if (g->edge_dst) free(g->edge_dst);\n\
         \x20 if (g->edge_weight) free(g->edge_weight);\n\
         \x20 g->edge_src = (int32_t*)0; g->edge_dst = (int32_t*)0; g->edge_weight = (int64_t*)0;\n\
         \x20 g->num_nodes = 0; g->num_edges = 0; g->edge_capacity = 0;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_graph_add_edge(intent_graph* g, int64_t src, int64_t dst, int64_t w) {\n\
         \x20 intent_graph_invalidate_csr(g);\n\
         \x20 if (g->num_edges >= g->edge_capacity) {\n\
         \x20   g->edge_capacity = g->edge_capacity ? g->edge_capacity * 2 : 8;\n\
         \x20   g->edge_src = (int32_t*)realloc(g->edge_src, (size_t)g->edge_capacity * sizeof(int32_t));\n\
         \x20   g->edge_dst = (int32_t*)realloc(g->edge_dst, (size_t)g->edge_capacity * sizeof(int32_t));\n\
         \x20   g->edge_weight = (int64_t*)realloc(g->edge_weight, (size_t)g->edge_capacity * sizeof(int64_t));\n\
         \x20   if (!g->edge_src || !g->edge_dst || !g->edge_weight) abort();\n\
         \x20 }\n\
         \x20 g->edge_src[g->num_edges] = (int32_t)src;\n\
         \x20 g->edge_dst[g->num_edges] = (int32_t)dst;\n\
         \x20 g->edge_weight[g->num_edges] = w;\n\
         \x20 g->num_edges++;\n\
         \x20 return g->num_edges;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_graph_num_nodes(const intent_graph* g) {\n\
         \x20 return g->num_nodes;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_graph_num_edges(const intent_graph* g) {\n\
         \x20 return g->num_edges;\n\
         }\n\
         /* Closure #355: clear() — free all edge storage + CSR\n\
          * caches, reset edge count to 0. Keeps num_nodes\n\
          * (construction-time identity) so the graph stays\n\
          * a valid empty-edge graph on the same node set.\n\
          * Returns prior num_edges. */\n\
         static INTENT_UNUSED int64_t intent_graph_clear(intent_graph* g) {\n\
         \x20 int64_t prior = g->num_edges;\n\
         \x20 if (g->edge_src) free(g->edge_src);\n\
         \x20 if (g->edge_dst) free(g->edge_dst);\n\
         \x20 if (g->edge_weight) free(g->edge_weight);\n\
         \x20 g->edge_src = (int32_t*)0;\n\
         \x20 g->edge_dst = (int32_t*)0;\n\
         \x20 g->edge_weight = (int64_t*)0;\n\
         \x20 g->num_edges = 0;\n\
         \x20 g->edge_capacity = 0;\n\
         \x20 intent_graph_invalidate_csr(g);\n\
         \x20 return prior;\n\
         }\n\
         /* Closure #336: BFS now iterates neighbors via the CSR\n\
          * adjacency cache, dropping per-pop edge iteration from\n\
          * O(num_edges) to O(degree). Overall: O(V+E) instead of\n\
          * O(V*E). */\n\
         static INTENT_UNUSED int64_t intent_graph_bfs_reach(const intent_graph* g, int64_t start) {\n\
         \x20 if (g->num_nodes <= 0 || start < 0 || start >= g->num_nodes) return 0;\n\
         \x20 intent_graph_build_csr_if_needed(g);\n\
         \x20 uint8_t* visited = (uint8_t*)calloc((size_t)g->num_nodes, 1);\n\
         \x20 int64_t* queue = (int64_t*)malloc((size_t)g->num_nodes * sizeof(int64_t));\n\
         \x20 if (!visited || !queue) abort();\n\
         \x20 int64_t qh = 0, qt = 0, count = 0;\n\
         \x20 queue[qt++] = start; visited[start] = 1; count = 1;\n\
         \x20 while (qh < qt) {\n\
         \x20   int64_t u = queue[qh++];\n\
         \x20   int32_t k0 = g->adj_start[u];\n\
         \x20   int32_t k1 = g->adj_start[u + 1];\n\
         \x20   for (int32_t k = k0; k < k1; k++) {\n\
         \x20     int64_t v = (int64_t)g->adj_csr_dst[k];\n\
         \x20     if (v < 0 || v >= g->num_nodes) continue;\n\
         \x20     if (!visited[v]) { visited[v] = 1; queue[qt++] = v; count++; }\n\
         \x20   }\n\
         \x20 }\n\
         \x20 free(visited); free(queue);\n\
         \x20 return count;\n\
         }\n\
         /* Closure #336: DFS uses the same CSR cache. */\n\
         static INTENT_UNUSED int64_t intent_graph_dfs_reach(const intent_graph* g, int64_t start) {\n\
         \x20 if (g->num_nodes <= 0 || start < 0 || start >= g->num_nodes) return 0;\n\
         \x20 intent_graph_build_csr_if_needed(g);\n\
         \x20 uint8_t* visited = (uint8_t*)calloc((size_t)g->num_nodes, 1);\n\
         \x20 int64_t* stack = (int64_t*)malloc((size_t)g->num_nodes * sizeof(int64_t));\n\
         \x20 if (!visited || !stack) abort();\n\
         \x20 int64_t sp = 0, count = 0;\n\
         \x20 stack[sp++] = start; visited[start] = 1; count = 1;\n\
         \x20 while (sp > 0) {\n\
         \x20   int64_t u = stack[--sp];\n\
         \x20   int32_t k0 = g->adj_start[u];\n\
         \x20   int32_t k1 = g->adj_start[u + 1];\n\
         \x20   for (int32_t k = k0; k < k1; k++) {\n\
         \x20     int64_t v = (int64_t)g->adj_csr_dst[k];\n\
         \x20     if (v < 0 || v >= g->num_nodes) continue;\n\
         \x20     if (!visited[v]) { visited[v] = 1; stack[sp++] = v; count++; }\n\
         \x20   }\n\
         \x20 }\n\
         \x20 free(visited); free(stack);\n\
         \x20 return count;\n\
         }\n\
         /* Closure #333 + #337: Kahn's topological sort via the CSR\n\
          * adjacency cache. Returns true iff not all nodes can be\n\
          * peeled off (i.e., a directed cycle exists). */\n\
         static INTENT_UNUSED bool intent_graph_has_cycle(const intent_graph* g) {\n\
         \x20 if (g->num_nodes <= 0) return false;\n\
         \x20 intent_graph_build_csr_if_needed(g);\n\
         \x20 int64_t* in_deg = (int64_t*)calloc((size_t)g->num_nodes, sizeof(int64_t));\n\
         \x20 int64_t* queue  = (int64_t*)malloc((size_t)g->num_nodes * sizeof(int64_t));\n\
         \x20 if (!in_deg || !queue) abort();\n\
         \x20 /* Walk CSR neighbor list of every source to count in-degrees. */\n\
         \x20 for (int64_t s = 0; s < g->num_nodes; s++) {\n\
         \x20   int32_t k0 = g->adj_start[s];\n\
         \x20   int32_t k1 = g->adj_start[s + 1];\n\
         \x20   for (int32_t k = k0; k < k1; k++) {\n\
         \x20     int32_t d = g->adj_csr_dst[k];\n\
         \x20     if (d >= 0 && (int64_t)d < g->num_nodes) in_deg[d]++;\n\
         \x20   }\n\
         \x20 }\n\
         \x20 int64_t qh = 0, qt = 0;\n\
         \x20 for (int64_t i = 0; i < g->num_nodes; i++) {\n\
         \x20   if (in_deg[i] == 0) queue[qt++] = i;\n\
         \x20 }\n\
         \x20 int64_t processed = 0;\n\
         \x20 while (qh < qt) {\n\
         \x20   int64_t u = queue[qh++];\n\
         \x20   processed++;\n\
         \x20   int32_t k0 = g->adj_start[u];\n\
         \x20   int32_t k1 = g->adj_start[u + 1];\n\
         \x20   for (int32_t k = k0; k < k1; k++) {\n\
         \x20     int64_t v = (int64_t)g->adj_csr_dst[k];\n\
         \x20     if (v < 0 || v >= g->num_nodes) continue;\n\
         \x20     in_deg[v]--;\n\
         \x20     if (in_deg[v] == 0) queue[qt++] = v;\n\
         \x20   }\n\
         \x20 }\n\
         \x20 free(in_deg); free(queue);\n\
         \x20 return processed < g->num_nodes;\n\
         }\n",
    );
    if emit_vec_dep {
        out.push_str(
            "/* Closure #335: topological sort. Pushes node indices\n\
          * into `out` in Kahn-order; returns the count of nodes\n\
          * actually appended (== num_nodes for a DAG, less if\n\
          * the graph has a cycle). The caller usually checks\n\
          * `graph_has_cycle` separately before relying on the\n\
          * order. We grow `out->data` via realloc rather than\n\
          * depending on the per-element `__push` helper so the\n\
          * gate logic stays simple. */\n\
         static INTENT_UNUSED int64_t intent_graph_topo_sort(const intent_graph* g, intent_vec_int64_t* out) {\n\
         \x20 if (g->num_nodes <= 0) return 0;\n\
         \x20 intent_graph_build_csr_if_needed(g);\n\
         \x20 int64_t* in_deg = (int64_t*)calloc((size_t)g->num_nodes, sizeof(int64_t));\n\
         \x20 int64_t* queue  = (int64_t*)malloc((size_t)g->num_nodes * sizeof(int64_t));\n\
         \x20 if (!in_deg || !queue) abort();\n\
         \x20 for (int64_t s = 0; s < g->num_nodes; s++) {\n\
         \x20   int32_t k0 = g->adj_start[s];\n\
         \x20   int32_t k1 = g->adj_start[s + 1];\n\
         \x20   for (int32_t k = k0; k < k1; k++) {\n\
         \x20     int32_t d = g->adj_csr_dst[k];\n\
         \x20     if (d >= 0 && (int64_t)d < g->num_nodes) in_deg[d]++;\n\
         \x20   }\n\
         \x20 }\n\
         \x20 int64_t qh = 0, qt = 0;\n\
         \x20 for (int64_t i = 0; i < g->num_nodes; i++) {\n\
         \x20   if (in_deg[i] == 0) queue[qt++] = i;\n\
         \x20 }\n\
         \x20 /* Reserve space in `out` for up to num_nodes new entries. */\n\
         \x20 uint64_t needed = out->len + (uint64_t)g->num_nodes;\n\
         \x20 if (out->capacity < needed) {\n\
         \x20   uint64_t new_cap = out->capacity == 0 ? 8 : out->capacity;\n\
         \x20   while (new_cap < needed) new_cap *= 2;\n\
         \x20   out->data = (int64_t*)realloc(out->data, (size_t)new_cap * sizeof(int64_t));\n\
         \x20   if (!out->data) abort();\n\
         \x20   out->capacity = new_cap;\n\
         \x20 }\n\
         \x20 int64_t processed = 0;\n\
         \x20 while (qh < qt) {\n\
         \x20   int64_t u = queue[qh++];\n\
         \x20   out->data[out->len++] = u;\n\
         \x20   processed++;\n\
         \x20   int32_t k0 = g->adj_start[u];\n\
         \x20   int32_t k1 = g->adj_start[u + 1];\n\
         \x20   for (int32_t k = k0; k < k1; k++) {\n\
         \x20     int64_t v = (int64_t)g->adj_csr_dst[k];\n\
         \x20     if (v < 0 || v >= g->num_nodes) continue;\n\
         \x20     in_deg[v]--;\n\
         \x20     if (in_deg[v] == 0) queue[qt++] = v;\n\
         \x20   }\n\
         \x20 }\n\
         \x20 free(in_deg); free(queue);\n\
         \x20 return processed;\n\
         }\n",
        );
    }
    if has_option_i64 {
        out.push_str(
            "static INTENT_UNUSED Enum_Option__i64 intent_graph_dijkstra(const intent_graph* g, int64_t src, int64_t dst) {\n\
             \x20 Enum_Option__i64 r;\n\
             \x20 if (g->num_nodes <= 0 || src < 0 || src >= g->num_nodes || dst < 0 || dst >= g->num_nodes) {\n\
             \x20   r.tag = 1; r.payload = 0; return r;\n\
             \x20 }\n\
             \x20 if (src == dst) { r.tag = 0; r.payload = 0; return r; }\n\
             \x20 intent_graph_build_csr_if_needed(g);\n\
             \x20 int64_t INF = 0x7fffffffffffffffLL;\n\
             \x20 int64_t* dist = (int64_t*)malloc((size_t)g->num_nodes * sizeof(int64_t));\n\
             \x20 uint8_t* done = (uint8_t*)calloc((size_t)g->num_nodes, 1);\n\
             \x20 if (!dist || !done) abort();\n\
             \x20 for (int64_t i = 0; i < g->num_nodes; i++) dist[i] = INF;\n\
             \x20 dist[src] = 0;\n\
             \x20 for (int64_t iter = 0; iter < g->num_nodes; iter++) {\n\
             \x20   int64_t u = -1; int64_t best = INF;\n\
             \x20   for (int64_t i = 0; i < g->num_nodes; i++) {\n\
             \x20     if (!done[i] && dist[i] < best) { best = dist[i]; u = i; }\n\
             \x20   }\n\
             \x20   if (u == -1 || best == INF) break;\n\
             \x20   done[u] = 1;\n\
             \x20   if (u == dst) break;\n\
             \x20   int32_t k0 = g->adj_start[u];\n\
             \x20   int32_t k1 = g->adj_start[u + 1];\n\
             \x20   for (int32_t k = k0; k < k1; k++) {\n\
             \x20     int64_t v = (int64_t)g->adj_csr_dst[k];\n\
             \x20     if (v < 0 || v >= g->num_nodes) continue;\n\
             \x20     int64_t nd = best + g->adj_csr_weight[k];\n\
             \x20     if (nd < dist[v]) dist[v] = nd;\n\
             \x20   }\n\
             \x20 }\n\
             \x20 int64_t d = dist[dst];\n\
             \x20 free(dist); free(done);\n\
             \x20 if (d == INF) { r.tag = 1; r.payload = 0; }\n\
             \x20 else { r.tag = 0; r.payload = d; }\n\
             \x20 return r;\n\
             }\n\
             /* Closure #333: Kruskal's MST with an insertion-sorted\n\
              * edge index array + path-compressed Union-Find.\n\
              * Treats edges as undirected (the original directed\n\
              * (src,dst) pair contributes one undirected u-v edge\n\
              * with the recorded weight). Returns None when the\n\
              * graph is disconnected or has 0 nodes. */\n\
             static INTENT_UNUSED Enum_Option__i64 intent_graph_mst_kruskal(const intent_graph* g) {\n\
             \x20 Enum_Option__i64 r;\n\
             \x20 if (g->num_nodes <= 0) { r.tag = 1; r.payload = 0; return r; }\n\
             \x20 if (g->num_nodes == 1) { r.tag = 0; r.payload = 0; return r; }\n\
             \x20 int64_t ne = g->num_edges;\n\
             \x20 int64_t* idx = (int64_t*)malloc((size_t)((ne == 0 ? 1 : ne)) * sizeof(int64_t));\n\
             \x20 int64_t* parent = (int64_t*)malloc((size_t)g->num_nodes * sizeof(int64_t));\n\
             \x20 if (!idx || !parent) abort();\n\
             \x20 for (int64_t i = 0; i < ne; i++) idx[i] = i;\n\
             \x20 /* Insertion sort by edge_weight ascending. */\n\
             \x20 for (int64_t i = 1; i < ne; i++) {\n\
             \x20   int64_t cur = idx[i];\n\
             \x20   int64_t cw = g->edge_weight[cur];\n\
             \x20   int64_t j = i - 1;\n\
             \x20   while (j >= 0 && g->edge_weight[idx[j]] > cw) {\n\
             \x20     idx[j + 1] = idx[j];\n\
             \x20     j--;\n\
             \x20   }\n\
             \x20   idx[j + 1] = cur;\n\
             \x20 }\n\
             \x20 for (int64_t i = 0; i < g->num_nodes; i++) parent[i] = i;\n\
             \x20 int64_t total = 0;\n\
             \x20 int64_t in_mst = 0;\n\
             \x20 int64_t need = g->num_nodes - 1;\n\
             \x20 for (int64_t k = 0; k < ne; k++) {\n\
             \x20   int64_t e = idx[k];\n\
             \x20   int64_t s = (int64_t)g->edge_src[e];\n\
             \x20   int64_t d = (int64_t)g->edge_dst[e];\n\
             \x20   if (s < 0 || s >= g->num_nodes || d < 0 || d >= g->num_nodes) continue;\n\
             \x20   /* find(s) with iterative path compression */\n\
             \x20   int64_t rs = s; while (parent[rs] != rs) rs = parent[rs];\n\
             \x20   int64_t p = s; while (parent[p] != rs) { int64_t n = parent[p]; parent[p] = rs; p = n; }\n\
             \x20   int64_t rd = d; while (parent[rd] != rd) rd = parent[rd];\n\
             \x20   p = d; while (parent[p] != rd) { int64_t n = parent[p]; parent[p] = rd; p = n; }\n\
             \x20   if (rs == rd) continue;\n\
             \x20   parent[rs] = rd;\n\
             \x20   total += g->edge_weight[e];\n\
             \x20   in_mst++;\n\
             \x20   if (in_mst >= need) break;\n\
             \x20 }\n\
             \x20 free(idx); free(parent);\n\
             \x20 if (in_mst >= need) { r.tag = 0; r.payload = total; }\n\
             \x20 else { r.tag = 1; r.payload = 0; }\n\
             \x20 return r;\n\
             }\n\
             /* Closure #333: Prim's MST with an O(V^2) linear scan\n\
              * for next-min (no BinaryHeap dependency). Treats\n\
              * edges as undirected. */\n\
             static INTENT_UNUSED Enum_Option__i64 intent_graph_mst_prim(const intent_graph* g) {\n\
             \x20 Enum_Option__i64 r;\n\
             \x20 if (g->num_nodes <= 0) { r.tag = 1; r.payload = 0; return r; }\n\
             \x20 /* Closure #338: walk u's neighbors via both forward and\n\
              * reverse CSRs, dropping the inner loop from O(num_edges)\n\
              * to O(degree). */\n\
             \x20 intent_graph_build_csr_if_needed(g);\n\
             \x20 intent_graph_build_rev_csr_if_needed(g);\n\
             \x20 int64_t INF = 0x7fffffffffffffffLL;\n\
             \x20 uint8_t* in_tree = (uint8_t*)calloc((size_t)g->num_nodes, 1);\n\
             \x20 int64_t* best = (int64_t*)malloc((size_t)g->num_nodes * sizeof(int64_t));\n\
             \x20 if (!in_tree || !best) abort();\n\
             \x20 for (int64_t i = 0; i < g->num_nodes; i++) best[i] = INF;\n\
             \x20 best[0] = 0;\n\
             \x20 int64_t total = 0; int64_t added = 0;\n\
             \x20 for (int64_t iter = 0; iter < g->num_nodes; iter++) {\n\
             \x20   int64_t u = -1; int64_t u_w = INF;\n\
             \x20   for (int64_t i = 0; i < g->num_nodes; i++) {\n\
             \x20     if (!in_tree[i] && best[i] < u_w) { u_w = best[i]; u = i; }\n\
             \x20   }\n\
             \x20   if (u == -1) break;\n\
             \x20   in_tree[u] = 1;\n\
             \x20   total += u_w;\n\
             \x20   added++;\n\
             \x20   /* Outgoing edges u→v via forward CSR. */\n\
             \x20   int32_t f_k0 = g->adj_start[u];\n\
             \x20   int32_t f_k1 = g->adj_start[u + 1];\n\
             \x20   for (int32_t k = f_k0; k < f_k1; k++) {\n\
             \x20     int64_t v = (int64_t)g->adj_csr_dst[k];\n\
             \x20     if (v < 0 || v >= g->num_nodes) continue;\n\
             \x20     if (in_tree[v]) continue;\n\
             \x20     int64_t w = g->adj_csr_weight[k];\n\
             \x20     if (w < best[v]) best[v] = w;\n\
             \x20   }\n\
             \x20   /* Incoming edges v→u via reverse CSR (undirected interp). */\n\
             \x20   int32_t r_k0 = g->rev_adj_start[u];\n\
             \x20   int32_t r_k1 = g->rev_adj_start[u + 1];\n\
             \x20   for (int32_t k = r_k0; k < r_k1; k++) {\n\
             \x20     int64_t v = (int64_t)g->rev_adj_csr_src[k];\n\
             \x20     if (v < 0 || v >= g->num_nodes) continue;\n\
             \x20     if (in_tree[v]) continue;\n\
             \x20     int64_t w = g->rev_adj_csr_weight[k];\n\
             \x20     if (w < best[v]) best[v] = w;\n\
             \x20   }\n\
             \x20 }\n\
             \x20 free(in_tree); free(best);\n\
             \x20 if (added == g->num_nodes) { r.tag = 0; r.payload = total; }\n\
             \x20 else { r.tag = 1; r.payload = 0; }\n\
             \x20 return r;\n\
             }\n",
        );
        if emit_vec_dep {
            out.push_str(
                "/* Closure #334 + #337: A* shortest path with user-provided\n\
              * heuristic vector, iterating neighbors via the CSR\n\
              * cache from closure #336. `h->data[i]` is the heuristic\n\
              * estimate of the remaining cost from node i to dst.\n\
              * Admissibility is the caller's responsibility — a zero\n\
              * heuristic reduces A* to Dijkstra. Returns None on size\n\
              * mismatch or unreachable. */\n\
             static INTENT_UNUSED Enum_Option__i64 intent_graph_astar(const intent_graph* g, int64_t src, int64_t dst, const intent_vec_int64_t* h) {\n\
             \x20 Enum_Option__i64 r;\n\
             \x20 if (g->num_nodes <= 0 || src < 0 || src >= g->num_nodes || dst < 0 || dst >= g->num_nodes) {\n\
             \x20   r.tag = 1; r.payload = 0; return r;\n\
             \x20 }\n\
             \x20 if (h->len != (uint64_t)g->num_nodes) {\n\
             \x20   r.tag = 1; r.payload = 0; return r;\n\
             \x20 }\n\
             \x20 if (src == dst) { r.tag = 0; r.payload = 0; return r; }\n\
             \x20 intent_graph_build_csr_if_needed(g);\n\
             \x20 int64_t INF = 0x7fffffffffffffffLL;\n\
             \x20 int64_t* gs = (int64_t*)malloc((size_t)g->num_nodes * sizeof(int64_t));\n\
             \x20 uint8_t* done = (uint8_t*)calloc((size_t)g->num_nodes, 1);\n\
             \x20 if (!gs || !done) abort();\n\
             \x20 for (int64_t i = 0; i < g->num_nodes; i++) gs[i] = INF;\n\
             \x20 gs[src] = 0;\n\
             \x20 for (int64_t iter = 0; iter < g->num_nodes; iter++) {\n\
             \x20   int64_t u = -1; int64_t best = INF;\n\
             \x20   for (int64_t i = 0; i < g->num_nodes; i++) {\n\
             \x20     if (done[i] || gs[i] == INF) continue;\n\
             \x20     int64_t hi = h->data[i];\n\
             \x20     int64_t f;\n\
             \x20     if (gs[i] > INF - hi) f = INF;\n\
             \x20     else f = gs[i] + hi;\n\
             \x20     if (f < best) { best = f; u = i; }\n\
             \x20   }\n\
             \x20   if (u == -1) break;\n\
             \x20   done[u] = 1;\n\
             \x20   if (u == dst) break;\n\
             \x20   int64_t gu = gs[u];\n\
             \x20   int32_t k0 = g->adj_start[u];\n\
             \x20   int32_t k1 = g->adj_start[u + 1];\n\
             \x20   for (int32_t k = k0; k < k1; k++) {\n\
             \x20     int64_t v = (int64_t)g->adj_csr_dst[k];\n\
             \x20     if (v < 0 || v >= g->num_nodes) continue;\n\
             \x20     int64_t nd = gu + g->adj_csr_weight[k];\n\
             \x20     if (nd < gs[v]) gs[v] = nd;\n\
             \x20   }\n\
             \x20 }\n\
             \x20 int64_t d = gs[dst];\n\
             \x20 free(gs); free(done);\n\
             \x20 if (d == INF) { r.tag = 1; r.payload = 0; }\n\
             \x20 else { r.tag = 0; r.payload = d; }\n\
             \x20 return r;\n\
             }\n\n",
            );
        }
    }
}

/// Walk the program for any `Trie` type usage. Closure #330.
pub(crate) fn program_uses_trie(program: &TypedProgram) -> bool {
    fn ty_uses(ty: &Type) -> bool {
        match ty {
            Type::Trie => true,
            Type::Vec(inner) | Type::Ref(inner) | Type::RefMut(inner) => ty_uses(inner),
            _ => false,
        }
    }
    for f in &program.functions {
        if ty_uses(&f.return_type) {
            return true;
        }
        for p in &f.params {
            if ty_uses(&p.ty) {
                return true;
            }
        }
        for s in &f.body {
            if stmt_uses_trie(s) {
                return true;
            }
        }
    }
    false
}

fn stmt_uses_trie(stmt: &crate::ir::TypedStmt) -> bool {
    use crate::ir::TypedStmt as S;
    fn ty_uses(ty: &Type) -> bool {
        matches!(ty, Type::Trie)
            || matches!(ty,
                Type::Vec(i) | Type::Ref(i) | Type::RefMut(i) if ty_uses(i))
    }
    match stmt {
        S::Let { ty, .. } | S::Reassign { ty, .. } | S::Drop { ty, .. } => ty_uses(ty),
        S::If { then_body, else_body, .. } => {
            then_body.iter().any(stmt_uses_trie)
                || else_body.iter().any(stmt_uses_trie)
        }
        S::While { body, .. } | S::For { body, .. } | S::ForIter { body, .. } => {
            body.iter().any(stmt_uses_trie)
        }
        _ => false,
    }
}

/// Data-structures roadmap Level 4 #4 — Trie runtime helpers
/// (closure #330). Prefix tree on a node arena restricted to
/// lowercase a-z keys. `children` is a flat array of 26 ×
/// num_nodes i32 child indices (-1 = no child); `is_end`
/// is a per-node bool. Insertion / lookup short-circuit to
/// false on any non-a-z input character.
fn emit_intent_trie_helpers_c_body(out: &mut String) {
    // ARC 2.1-2.3 — sparse-children rewrite. The per-node storage
    // is now struct-of-arrays: `node_keys[idx]` is a sorted u8 array
    // and `node_children[idx]` is the parallel i32 child-index array,
    // both length `node_count[idx]` and allocated capacity
    // `node_cap[idx]`. Lookup is binary search; insert + delete shift
    // entries to maintain sort order. Freelist is a separate per-node
    // `free_next` array (no longer reusing a child slot). Memory
    // usage: O(actual_children) per node instead of fixed 256
    // entries — ~30–100× savings for sparse alphabets (DNA, ASCII
    // digits, hex). Live node count is still num_nodes - free_count.
    out.push_str(
        "typedef struct { uint8_t** node_keys; int32_t** node_children; uint16_t* node_count; uint16_t* node_cap; int64_t* free_next; uint8_t* is_end; int64_t num_nodes; int64_t capacity; int64_t num_words; int64_t free_head; int64_t free_count; } intent_trie;\n\
         /* Closure #345: alphabet generalized to the full u8 range.\n\
          * Every nonzero byte is a valid character. C strings are\n\
          * nul-terminated, so byte 0 still terminates a word; the\n\
          * empty string targets the root node. */\n\
         static INTENT_UNUSED bool intent_trie_valid_str(const char* s) {\n\
         \x20 return s != (const char*)0;\n\
         }\n\
         /* Binary search the sorted keys array of `node`; return the\n\
          * slot index if the key is present, -1 otherwise. */\n\
         static INTENT_UNUSED int64_t intent_trie__find_slot(const intent_trie* t, int64_t node, uint8_t key) {\n\
         \x20 uint16_t lo = 0, hi = t->node_count[node];\n\
         \x20 while (lo < hi) {\n\
         \x20   uint16_t mid = (lo + hi) / 2;\n\
         \x20   uint8_t k = t->node_keys[node][mid];\n\
         \x20   if (k == key) return (int64_t)mid;\n\
         \x20   if (k < key) lo = (uint16_t)(mid + 1); else hi = mid;\n\
         \x20 }\n\
         \x20 return -1;\n\
         }\n\
         /* Binary search for the first slot whose key is >= `key`;\n\
          * the insertion point that preserves sorted order. */\n\
         static INTENT_UNUSED uint16_t intent_trie__lower_bound(const intent_trie* t, int64_t node, uint8_t key) {\n\
         \x20 uint16_t lo = 0, hi = t->node_count[node];\n\
         \x20 while (lo < hi) {\n\
         \x20   uint16_t mid = (lo + hi) / 2;\n\
         \x20   if (t->node_keys[node][mid] < key) lo = (uint16_t)(mid + 1); else hi = mid;\n\
         \x20 }\n\
         \x20 return lo;\n\
         }\n\
         /* Ensure node has room for at least `min_cap` children.\n\
          * Doubles capacity on grow; starts at 4 for the first child. */\n\
         static INTENT_UNUSED void intent_trie__grow_node(intent_trie* t, int64_t node, uint16_t min_cap) {\n\
         \x20 if (t->node_cap[node] >= min_cap) return;\n\
         \x20 uint16_t new_cap = t->node_cap[node] ? t->node_cap[node] : 4;\n\
         \x20 while (new_cap < min_cap) new_cap = (uint16_t)(new_cap * 2);\n\
         \x20 t->node_keys[node] = (uint8_t*)realloc(t->node_keys[node], (size_t)new_cap * sizeof(uint8_t));\n\
         \x20 t->node_children[node] = (int32_t*)realloc(t->node_children[node], (size_t)new_cap * sizeof(int32_t));\n\
         \x20 if (!t->node_keys[node] || !t->node_children[node]) abort();\n\
         \x20 t->node_cap[node] = new_cap;\n\
         }\n\
         /* Insert (key, child) at slot `pos` of node, shifting existing\n\
          * entries right by one. Caller has already grown capacity. */\n\
         static INTENT_UNUSED void intent_trie__insert_pair(intent_trie* t, int64_t node, uint16_t pos, uint8_t key, int32_t child) {\n\
         \x20 uint16_t cnt = t->node_count[node];\n\
         \x20 for (uint16_t i = cnt; i > pos; i--) {\n\
         \x20   t->node_keys[node][i] = t->node_keys[node][i - 1];\n\
         \x20   t->node_children[node][i] = t->node_children[node][i - 1];\n\
         \x20 }\n\
         \x20 t->node_keys[node][pos] = key;\n\
         \x20 t->node_children[node][pos] = child;\n\
         \x20 t->node_count[node] = (uint16_t)(cnt + 1);\n\
         }\n\
         /* Remove the entry at slot `pos` of node, shifting trailing\n\
          * entries left by one. */\n\
         static INTENT_UNUSED void intent_trie__remove_pair(intent_trie* t, int64_t node, uint16_t pos) {\n\
         \x20 uint16_t cnt = t->node_count[node];\n\
         \x20 for (uint16_t i = pos; i + 1 < cnt; i++) {\n\
         \x20   t->node_keys[node][i] = t->node_keys[node][i + 1];\n\
         \x20   t->node_children[node][i] = t->node_children[node][i + 1];\n\
         \x20 }\n\
         \x20 t->node_count[node] = (uint16_t)(cnt - 1);\n\
         }\n\
         /* Allocate or recycle a node. Freelist (LIFO) reuses slots\n\
          * before extending the arena. */\n\
         static INTENT_UNUSED int64_t intent_trie_new_node(intent_trie* t) {\n\
         \x20 if (t->free_head != -1) {\n\
         \x20   int64_t idx = t->free_head;\n\
         \x20   t->free_head = t->free_next[idx];\n\
         \x20   t->free_count--;\n\
         \x20   /* recycled slot already has node_count=0; clear is_end\n\
          *    just in case. node_keys/children buffers may still be\n\
          *    allocated from prior use — that's fine, we just reuse. */\n\
         \x20   t->node_count[idx] = 0;\n\
         \x20   t->is_end[idx] = 0;\n\
         \x20   t->free_next[idx] = -1;\n\
         \x20   return idx;\n\
         \x20 }\n\
         \x20 if (t->num_nodes >= t->capacity) {\n\
         \x20   int64_t old_cap = t->capacity;\n\
         \x20   t->capacity = t->capacity ? t->capacity * 2 : 8;\n\
         \x20   t->node_keys = (uint8_t**)realloc(t->node_keys, (size_t)t->capacity * sizeof(uint8_t*));\n\
         \x20   t->node_children = (int32_t**)realloc(t->node_children, (size_t)t->capacity * sizeof(int32_t*));\n\
         \x20   t->node_count = (uint16_t*)realloc(t->node_count, (size_t)t->capacity * sizeof(uint16_t));\n\
         \x20   t->node_cap = (uint16_t*)realloc(t->node_cap, (size_t)t->capacity * sizeof(uint16_t));\n\
         \x20   t->free_next = (int64_t*)realloc(t->free_next, (size_t)t->capacity * sizeof(int64_t));\n\
         \x20   t->is_end = (uint8_t*)realloc(t->is_end, (size_t)t->capacity * sizeof(uint8_t));\n\
         \x20   if (!t->node_keys || !t->node_children || !t->node_count || !t->node_cap || !t->free_next || !t->is_end) abort();\n\
         \x20   /* Zero-init the newly-allocated slots so they read as\n\
          *    empty/uninitialized rather than holding stale realloc'd bytes. */\n\
         \x20   for (int64_t i = old_cap; i < t->capacity; i++) {\n\
         \x20     t->node_keys[i] = (uint8_t*)0;\n\
         \x20     t->node_children[i] = (int32_t*)0;\n\
         \x20     t->node_count[i] = 0;\n\
         \x20     t->node_cap[i] = 0;\n\
         \x20     t->free_next[i] = -1;\n\
         \x20     t->is_end[i] = 0;\n\
         \x20   }\n\
         \x20 }\n\
         \x20 int64_t idx = t->num_nodes;\n\
         \x20 t->node_keys[idx] = (uint8_t*)0;\n\
         \x20 t->node_children[idx] = (int32_t*)0;\n\
         \x20 t->node_count[idx] = 0;\n\
         \x20 t->node_cap[idx] = 0;\n\
         \x20 t->free_next[idx] = -1;\n\
         \x20 t->is_end[idx] = 0;\n\
         \x20 t->num_nodes++;\n\
         \x20 return idx;\n\
         }\n\
         static INTENT_UNUSED intent_trie intent_trie_new(void) {\n\
         \x20 intent_trie t;\n\
         \x20 t.node_keys = (uint8_t**)0; t.node_children = (int32_t**)0;\n\
         \x20 t.node_count = (uint16_t*)0; t.node_cap = (uint16_t*)0;\n\
         \x20 t.free_next = (int64_t*)0; t.is_end = (uint8_t*)0;\n\
         \x20 t.num_nodes = 0; t.capacity = 0; t.num_words = 0;\n\
         \x20 t.free_head = -1; t.free_count = 0;\n\
         \x20 (void)intent_trie_new_node(&t);  /* root = 0 */\n\
         \x20 return t;\n\
         }\n\
         static INTENT_UNUSED void intent_trie_drop(intent_trie* t) {\n\
         \x20 if (t->node_keys && t->node_children) {\n\
         \x20   for (int64_t i = 0; i < t->num_nodes; i++) {\n\
         \x20     if (t->node_keys[i]) free(t->node_keys[i]);\n\
         \x20     if (t->node_children[i]) free(t->node_children[i]);\n\
         \x20   }\n\
         \x20 }\n\
         \x20 if (t->node_keys) free(t->node_keys);\n\
         \x20 if (t->node_children) free(t->node_children);\n\
         \x20 if (t->node_count) free(t->node_count);\n\
         \x20 if (t->node_cap) free(t->node_cap);\n\
         \x20 if (t->free_next) free(t->free_next);\n\
         \x20 if (t->is_end) free(t->is_end);\n\
         \x20 t->node_keys = (uint8_t**)0; t->node_children = (int32_t**)0;\n\
         \x20 t->node_count = (uint16_t*)0; t->node_cap = (uint16_t*)0;\n\
         \x20 t->free_next = (int64_t*)0; t->is_end = (uint8_t*)0;\n\
         \x20 t->num_nodes = 0; t->capacity = 0; t->num_words = 0;\n\
         \x20 t->free_head = -1; t->free_count = 0;\n\
         }\n\
         static INTENT_UNUSED bool intent_trie_insert(intent_trie* t, const char* s) {\n\
         \x20 if (!intent_trie_valid_str(s)) return false;\n\
         \x20 if (*s == 0) {\n\
         \x20   if (t->is_end[0]) return false;\n\
         \x20   t->is_end[0] = 1; t->num_words++; return true;\n\
         \x20 }\n\
         \x20 int64_t cur = 0;\n\
         \x20 for (const char* p = s; *p; p++) {\n\
         \x20   uint8_t c = (uint8_t)*p;\n\
         \x20   int64_t slot = intent_trie__find_slot(t, cur, c);\n\
         \x20   if (slot == -1) {\n\
         \x20     int64_t nx = intent_trie_new_node(t);\n\
         \x20     uint16_t pos = intent_trie__lower_bound(t, cur, c);\n\
         \x20     intent_trie__grow_node(t, cur, (uint16_t)(t->node_count[cur] + 1));\n\
         \x20     intent_trie__insert_pair(t, cur, pos, c, (int32_t)nx);\n\
         \x20     cur = nx;\n\
         \x20   } else {\n\
         \x20     cur = (int64_t)t->node_children[cur][slot];\n\
         \x20   }\n\
         \x20 }\n\
         \x20 if (t->is_end[cur]) return false;\n\
         \x20 t->is_end[cur] = 1; t->num_words++; return true;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_trie_walk(const intent_trie* t, const char* s) {\n\
         \x20 if (!intent_trie_valid_str(s)) return -1;\n\
         \x20 int64_t cur = 0;\n\
         \x20 for (const char* p = s; *p; p++) {\n\
         \x20   uint8_t c = (uint8_t)*p;\n\
         \x20   int64_t slot = intent_trie__find_slot(t, cur, c);\n\
         \x20   if (slot == -1) return -1;\n\
         \x20   cur = (int64_t)t->node_children[cur][slot];\n\
         \x20 }\n\
         \x20 return cur;\n\
         }\n\
         static INTENT_UNUSED bool intent_trie_contains(const intent_trie* t, const char* s) {\n\
         \x20 int64_t cur = intent_trie_walk(t, s);\n\
         \x20 if (cur == -1) return false;\n\
         \x20 return t->is_end[cur] != 0;\n\
         }\n\
         static INTENT_UNUSED bool intent_trie_starts_with(const intent_trie* t, const char* s) {\n\
         \x20 int64_t cur = intent_trie_walk(t, s);\n\
         \x20 return cur != -1;\n\
         }\n\
         static INTENT_UNUSED bool intent_trie_delete(intent_trie* t, const char* s) {\n\
         \x20 if (!intent_trie_valid_str(s)) return false;\n\
         \x20 if (*s == 0) {\n\
         \x20   if (!t->is_end[0]) return false;\n\
         \x20   t->is_end[0] = 0; t->num_words--; return true;\n\
         \x20 }\n\
         \x20 size_t n = 0;\n\
         \x20 for (const char* p = s; *p; p++) n++;\n\
         \x20 int64_t* path_node = (int64_t*)malloc((n + 1) * sizeof(int64_t));\n\
         \x20 uint8_t* path_byte = (uint8_t*)malloc(n * sizeof(uint8_t));\n\
         \x20 if (!path_node || !path_byte) abort();\n\
         \x20 path_node[0] = 0;\n\
         \x20 int64_t cur = 0;\n\
         \x20 for (size_t i = 0; i < n; i++) {\n\
         \x20   uint8_t c = (uint8_t)s[i];\n\
         \x20   int64_t slot = intent_trie__find_slot(t, cur, c);\n\
         \x20   if (slot == -1) { free(path_node); free(path_byte); return false; }\n\
         \x20   path_byte[i] = c;\n\
         \x20   cur = (int64_t)t->node_children[cur][slot];\n\
         \x20   path_node[i + 1] = cur;\n\
         \x20 }\n\
         \x20 if (!t->is_end[cur]) { free(path_node); free(path_byte); return false; }\n\
         \x20 t->is_end[cur] = 0;\n\
         \x20 t->num_words--;\n\
         \x20 /* Walk back up; free dead nodes one at a time. */\n\
         \x20 for (size_t step = n; step > 0; step--) {\n\
         \x20   int64_t node = path_node[step];\n\
         \x20   if (node == 0) break;\n\
         \x20   if (t->is_end[node]) break;\n\
         \x20   if (t->node_count[node] != 0) break;\n\
         \x20   int64_t parent = path_node[step - 1];\n\
         \x20   uint16_t pos = (uint16_t)intent_trie__find_slot(t, parent, path_byte[step - 1]);\n\
         \x20   intent_trie__remove_pair(t, parent, pos);\n\
         \x20   t->free_next[node] = t->free_head;\n\
         \x20   t->free_head = node;\n\
         \x20   t->free_count++;\n\
         \x20 }\n\
         \x20 free(path_node); free(path_byte);\n\
         \x20 return true;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_trie_clear(intent_trie* t) {\n\
         \x20 int64_t prior = t->num_words;\n\
         \x20 /* Free every per-node keys/children buffer, but keep\n\
          *    the parent arrays so future inserts can reuse the\n\
          *    capacity. Reset every node to the empty state, then\n\
          *    re-establish the root. */\n\
         \x20 if (t->capacity > 0) {\n\
         \x20   for (int64_t i = 0; i < t->num_nodes; i++) {\n\
         \x20     if (t->node_keys[i]) { free(t->node_keys[i]); t->node_keys[i] = (uint8_t*)0; }\n\
         \x20     if (t->node_children[i]) { free(t->node_children[i]); t->node_children[i] = (int32_t*)0; }\n\
         \x20     t->node_count[i] = 0;\n\
         \x20     t->node_cap[i] = 0;\n\
         \x20     t->free_next[i] = -1;\n\
         \x20     t->is_end[i] = 0;\n\
         \x20   }\n\
         \x20   t->num_nodes = 1;  /* root */\n\
         \x20 } else {\n\
         \x20   t->num_nodes = 0;\n\
         \x20 }\n\
         \x20 t->num_words = 0;\n\
         \x20 t->free_head = -1;\n\
         \x20 t->free_count = 0;\n\
         \x20 return prior;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_trie_len(const intent_trie* t) {\n\
         \x20 return t->num_words;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_trie_node_count(const intent_trie* t) {\n\
         \x20 return t->num_nodes - t->free_count;\n\
         }\n\n",
    );
}

/// Walk the program for any `SkipList` type usage. Closure #331.
pub(crate) fn program_uses_skiplist(program: &TypedProgram) -> bool {
    fn ty_uses(ty: &Type) -> bool {
        match ty {
            Type::SkipList => true,
            Type::Vec(inner) | Type::Ref(inner) | Type::RefMut(inner) => ty_uses(inner),
            _ => false,
        }
    }
    for f in &program.functions {
        if ty_uses(&f.return_type) {
            return true;
        }
        for p in &f.params {
            if ty_uses(&p.ty) {
                return true;
            }
        }
        for s in &f.body {
            if stmt_uses_skiplist(s) {
                return true;
            }
        }
    }
    false
}

fn stmt_uses_skiplist(stmt: &crate::ir::TypedStmt) -> bool {
    use crate::ir::TypedStmt as S;
    fn ty_uses(ty: &Type) -> bool {
        matches!(ty, Type::SkipList)
            || matches!(ty,
                Type::Vec(i) | Type::Ref(i) | Type::RefMut(i) if ty_uses(i))
    }
    match stmt {
        S::Let { ty, .. } | S::Reassign { ty, .. } | S::Drop { ty, .. } => ty_uses(ty),
        S::If { then_body, else_body, .. } => {
            then_body.iter().any(stmt_uses_skiplist)
                || else_body.iter().any(stmt_uses_skiplist)
        }
        S::While { body, .. } | S::For { body, .. } | S::ForIter { body, .. } => {
            body.iter().any(stmt_uses_skiplist)
        }
        _ => false,
    }
}

/// Data-structures roadmap Level 4 #7 — SkipList<i64> runtime
/// helpers (closure #331). Probabilistic ordered set on a node
/// arena. MAX_LEVEL fixed at 8; per-node forward[] indices
/// stored in a flat `i32*` array of capacity × 8 entries. Node
/// 0 is the head sentinel (its key is unused). Geometric level
/// distribution driven by an LCG seed stored in the struct.
/// min/max return Option<i64> and gate on Option__i64.
fn emit_intent_skiplist_helpers_c_body(out: &mut String, has_option_i64: bool) {
    out.push_str(
        "#define INTENT_SKIPLIST_MAX_LEVEL 8\n\
         typedef struct { int64_t* keys; int32_t* forward; int32_t* node_levels; uint64_t rng_state; int64_t num_nodes; int64_t capacity; int64_t num_keys; int64_t tail_node; } intent_skiplist_i64;\n\
         static INTENT_UNUSED uint64_t intent_skiplist_i64_rand(intent_skiplist_i64* sl) {\n\
         \x20 sl->rng_state = sl->rng_state * 6364136223846793005ULL + 1442695040888963407ULL;\n\
         \x20 return sl->rng_state;\n\
         }\n\
         static INTENT_UNUSED int32_t intent_skiplist_i64_random_level(intent_skiplist_i64* sl) {\n\
         \x20 int32_t lvl = 1;\n\
         \x20 while (lvl < INTENT_SKIPLIST_MAX_LEVEL) {\n\
         \x20   if ((intent_skiplist_i64_rand(sl) & 1) == 0) break;\n\
         \x20   lvl++;\n\
         \x20 }\n\
         \x20 return lvl;\n\
         }\n\
         static INTENT_UNUSED void intent_skiplist_i64_ensure_cap(intent_skiplist_i64* sl, int64_t needed) {\n\
         \x20 if (needed <= sl->capacity) return;\n\
         \x20 int64_t new_cap = sl->capacity ? sl->capacity * 2 : 8;\n\
         \x20 while (new_cap < needed) new_cap *= 2;\n\
         \x20 sl->keys = (int64_t*)realloc(sl->keys, (size_t)new_cap * sizeof(int64_t));\n\
         \x20 sl->forward = (int32_t*)realloc(sl->forward, (size_t)new_cap * INTENT_SKIPLIST_MAX_LEVEL * sizeof(int32_t));\n\
         \x20 sl->node_levels = (int32_t*)realloc(sl->node_levels, (size_t)new_cap * sizeof(int32_t));\n\
         \x20 if (!sl->keys || !sl->forward || !sl->node_levels) abort();\n\
         \x20 sl->capacity = new_cap;\n\
         }\n\
         static INTENT_UNUSED intent_skiplist_i64 intent_skiplist_i64_new(void) {\n\
         \x20 intent_skiplist_i64 sl;\n\
         \x20 sl.keys = (int64_t*)0; sl.forward = (int32_t*)0; sl.node_levels = (int32_t*)0;\n\
         \x20 sl.rng_state = 0x9E3779B97F4A7C15ULL;\n\
         \x20 sl.num_nodes = 0; sl.capacity = 0; sl.num_keys = 0;\n\
         \x20 sl.tail_node = -1;\n\
         \x20 intent_skiplist_i64_ensure_cap(&sl, 1);\n\
         \x20 /* Head sentinel at index 0: key unused, all forward = -1, level = MAX_LEVEL. */\n\
         \x20 sl.keys[0] = 0;\n\
         \x20 for (int k = 0; k < INTENT_SKIPLIST_MAX_LEVEL; k++) sl.forward[k] = -1;\n\
         \x20 sl.node_levels[0] = INTENT_SKIPLIST_MAX_LEVEL;\n\
         \x20 sl.num_nodes = 1;\n\
         \x20 return sl;\n\
         }\n\
         static INTENT_UNUSED void intent_skiplist_i64_drop(intent_skiplist_i64* sl) {\n\
         \x20 if (sl->keys) free(sl->keys);\n\
         \x20 if (sl->forward) free(sl->forward);\n\
         \x20 if (sl->node_levels) free(sl->node_levels);\n\
         \x20 sl->keys = (int64_t*)0; sl->forward = (int32_t*)0; sl->node_levels = (int32_t*)0;\n\
         \x20 sl->num_nodes = 0; sl->capacity = 0; sl->num_keys = 0;\n\
         \x20 sl->tail_node = -1;\n\
         }\n\
         static INTENT_UNUSED bool intent_skiplist_i64_insert(intent_skiplist_i64* sl, int64_t x) {\n\
         \x20 int32_t update[INTENT_SKIPLIST_MAX_LEVEL];\n\
         \x20 int64_t cur = 0;\n\
         \x20 for (int lvl = INTENT_SKIPLIST_MAX_LEVEL - 1; lvl >= 0; lvl--) {\n\
         \x20   for (;;) {\n\
         \x20     int32_t next = sl->forward[cur * INTENT_SKIPLIST_MAX_LEVEL + lvl];\n\
         \x20     if (next == -1) break;\n\
         \x20     if (sl->keys[next] >= x) break;\n\
         \x20     cur = (int64_t)next;\n\
         \x20   }\n\
         \x20   update[lvl] = (int32_t)cur;\n\
         \x20 }\n\
         \x20 int32_t cand = sl->forward[(int64_t)update[0] * INTENT_SKIPLIST_MAX_LEVEL + 0];\n\
         \x20 if (cand != -1 && sl->keys[cand] == x) return false;\n\
         \x20 int32_t new_lvl = intent_skiplist_i64_random_level(sl);\n\
         \x20 intent_skiplist_i64_ensure_cap(sl, sl->num_nodes + 1);\n\
         \x20 int64_t new_idx = sl->num_nodes;\n\
         \x20 sl->keys[new_idx] = x;\n\
         \x20 sl->node_levels[new_idx] = new_lvl;\n\
         \x20 for (int lvl = 0; lvl < new_lvl; lvl++) {\n\
         \x20   sl->forward[new_idx * INTENT_SKIPLIST_MAX_LEVEL + lvl] = sl->forward[(int64_t)update[lvl] * INTENT_SKIPLIST_MAX_LEVEL + lvl];\n\
         \x20   sl->forward[(int64_t)update[lvl] * INTENT_SKIPLIST_MAX_LEVEL + lvl] = (int32_t)new_idx;\n\
         \x20 }\n\
         \x20 for (int lvl = new_lvl; lvl < INTENT_SKIPLIST_MAX_LEVEL; lvl++) {\n\
         \x20   sl->forward[new_idx * INTENT_SKIPLIST_MAX_LEVEL + lvl] = -1;\n\
         \x20 }\n\
         \x20 /* Closure #341 tail tracker: if the new node's level-0\n\
          * forward is -1, it's now the rightmost node — update tail. */\n\
         \x20 if (sl->forward[new_idx * INTENT_SKIPLIST_MAX_LEVEL + 0] == -1) {\n\
         \x20   sl->tail_node = new_idx;\n\
         \x20 }\n\
         \x20 sl->num_nodes++; sl->num_keys++;\n\
         \x20 return true;\n\
         }\n\
         static INTENT_UNUSED bool intent_skiplist_i64_contains(const intent_skiplist_i64* sl, int64_t x) {\n\
         \x20 int64_t cur = 0;\n\
         \x20 for (int lvl = INTENT_SKIPLIST_MAX_LEVEL - 1; lvl >= 0; lvl--) {\n\
         \x20   for (;;) {\n\
         \x20     int32_t next = sl->forward[cur * INTENT_SKIPLIST_MAX_LEVEL + lvl];\n\
         \x20     if (next == -1) break;\n\
         \x20     if (sl->keys[next] >= x) break;\n\
         \x20     cur = (int64_t)next;\n\
         \x20   }\n\
         \x20 }\n\
         \x20 int32_t cand = sl->forward[cur * INTENT_SKIPLIST_MAX_LEVEL + 0];\n\
         \x20 return (cand != -1 && sl->keys[cand] == x);\n\
         }\n\
         /* Closure #339: remove a key. Standard skip-list removal —\n\
          * walk down the levels recording the update[] array (nodes\n\
          * whose level-l forward pointer might need to skip past the\n\
          * removed node), then for each level where update[lvl]'s\n\
          * forward equals the candidate, redirect it to the\n\
          * candidate's own forward. Returns true iff a node was\n\
          * removed. Arena slots stay tombstoned (no compaction). */\n\
         static INTENT_UNUSED bool intent_skiplist_i64_remove(intent_skiplist_i64* sl, int64_t x) {\n\
         \x20 int32_t update[INTENT_SKIPLIST_MAX_LEVEL];\n\
         \x20 int64_t cur = 0;\n\
         \x20 for (int lvl = INTENT_SKIPLIST_MAX_LEVEL - 1; lvl >= 0; lvl--) {\n\
         \x20   for (;;) {\n\
         \x20     int32_t next = sl->forward[cur * INTENT_SKIPLIST_MAX_LEVEL + lvl];\n\
         \x20     if (next == -1) break;\n\
         \x20     if (sl->keys[next] >= x) break;\n\
         \x20     cur = (int64_t)next;\n\
         \x20   }\n\
         \x20   update[lvl] = (int32_t)cur;\n\
         \x20 }\n\
         \x20 int32_t cand = sl->forward[(int64_t)update[0] * INTENT_SKIPLIST_MAX_LEVEL + 0];\n\
         \x20 if (cand == -1 || sl->keys[cand] != x) return false;\n\
         \x20 int32_t cand_lvl = sl->node_levels[cand];\n\
         \x20 for (int lvl = 0; lvl < cand_lvl; lvl++) {\n\
         \x20   if (sl->forward[(int64_t)update[lvl] * INTENT_SKIPLIST_MAX_LEVEL + lvl] == cand) {\n\
         \x20     sl->forward[(int64_t)update[lvl] * INTENT_SKIPLIST_MAX_LEVEL + lvl] =\n\
         \x20       sl->forward[(int64_t)cand * INTENT_SKIPLIST_MAX_LEVEL + lvl];\n\
         \x20   }\n\
         \x20 }\n\
         \x20 /* Closure #341 tail tracker: if we removed the tail, the\n\
          * new tail is update[0]. If update[0] is the head sentinel\n\
          * (index 0) the list is now empty (tail = -1). */\n\
         \x20 if ((int64_t)cand == sl->tail_node) {\n\
         \x20   sl->tail_node = (update[0] == 0) ? -1 : (int64_t)update[0];\n\
         \x20 }\n\
         \x20 sl->num_keys--;\n\
         \x20 return true;\n\
         }\n\
         /* Closure #354: clear() — reset to the single-head-sentinel\n\
          * state without freeing the backing buffers. Zeros the\n\
          * head's forward pointers and tail_node, resets num_keys\n\
          * to 0 and num_nodes to 1 (just the head). Returns prior\n\
          * num_keys. */\n\
         static INTENT_UNUSED int64_t intent_skiplist_i64_clear(intent_skiplist_i64* sl) {\n\
         \x20 int64_t prior = sl->num_keys;\n\
         \x20 if (sl->capacity > 0 && sl->forward && sl->node_levels) {\n\
         \x20   for (int k = 0; k < INTENT_SKIPLIST_MAX_LEVEL; k++) {\n\
         \x20     sl->forward[k] = -1;\n\
         \x20   }\n\
         \x20   sl->node_levels[0] = INTENT_SKIPLIST_MAX_LEVEL;\n\
         \x20   sl->num_nodes = 1;\n\
         \x20 } else {\n\
         \x20   sl->num_nodes = 0;\n\
         \x20 }\n\
         \x20 sl->num_keys = 0;\n\
         \x20 sl->tail_node = -1;\n\
         \x20 return prior;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_skiplist_i64_len(const intent_skiplist_i64* sl) {\n\
         \x20 return sl->num_keys;\n\
         }\n",
    );
    if has_option_i64 {
        out.push_str(
            "static INTENT_UNUSED Enum_Option__i64 intent_skiplist_i64_min(const intent_skiplist_i64* sl) {\n\
             \x20 Enum_Option__i64 r;\n\
             \x20 int32_t first = sl->forward[0];\n\
             \x20 if (first == -1) { r.tag = 1; r.payload = 0; return r; }\n\
             \x20 r.tag = 0; r.payload = sl->keys[first]; return r;\n\
             }\n\
             /* Closure #341: O(1) max via the maintained tail_node. */\n\
             static INTENT_UNUSED Enum_Option__i64 intent_skiplist_i64_max(const intent_skiplist_i64* sl) {\n\
             \x20 Enum_Option__i64 r;\n\
             \x20 if (sl->tail_node == -1) { r.tag = 1; r.payload = 0; return r; }\n\
             \x20 r.tag = 0; r.payload = sl->keys[sl->tail_node]; return r;\n\
             }\n\n",
        );
    }
}

/// Data-structures roadmap Level 1 — FNV-1a hash helpers.
/// Offset basis 0xcbf29ce484222325, prime 0x100000001b3.
fn emit_intent_hash_helpers_c(out: &mut String, body: &str) {
    if !body.contains("intent_hash_") && !body.contains("intent_siphash") {
        return;
    }
    out.push_str(
        "static INTENT_UNUSED uint64_t intent_hash_i64(int64_t x) {\n\
         \x20 uint64_t h = 0xcbf29ce484222325ULL;\n\
         \x20 uint64_t u = (uint64_t)x;\n\
         \x20 for (int i = 0; i < 8; i++) {\n\
         \x20   h ^= (u >> (i * 8)) & 0xffULL;\n\
         \x20   h *= 0x100000001b3ULL;\n\
         \x20 }\n\
         \x20 return h;\n\
         }\n\
         static INTENT_UNUSED uint64_t intent_hash_str(const char* s) {\n\
         \x20 uint64_t h = 0xcbf29ce484222325ULL;\n\
         \x20 if (s == 0) return h;\n\
         \x20 for (; *s; s++) {\n\
         \x20   h ^= (uint64_t)(unsigned char)(*s);\n\
         \x20   h *= 0x100000001b3ULL;\n\
         \x20 }\n\
         \x20 return h;\n\
         }\n\
         static INTENT_UNUSED uint64_t intent_hash_combine(uint64_t a, uint64_t b) {\n\
         \x20 /* boost::hash_combine-style mixer, FNV-tuned. */\n\
         \x20 a ^= b + 0x9e3779b97f4a7c15ULL + (a << 6) + (a >> 2);\n\
         \x20 return a;\n\
         }\n\
         /* Closure #347: hash_f64 — FNV-1a on the raw IEEE-754 bit\n\
         \x20  pattern. Normalizes -0.0 to +0.0 (same hash) since the\n\
         \x20  two values compare equal in == . NaNs are NOT normalized:\n\
         \x20  per IEEE-754, NaN != NaN, so users should compare via\n\
         \x20  isnan() rather than rely on hash collisions for them. */\n\
         static INTENT_UNUSED uint64_t intent_hash_f64(double x) {\n\
         \x20 if (x == 0.0) x = 0.0;  /* fold -0.0 → +0.0 */\n\
         \x20 uint64_t u;\n\
         \x20 memcpy(&u, &x, sizeof(u));\n\
         \x20 uint64_t h = 0xcbf29ce484222325ULL;\n\
         \x20 for (int i = 0; i < 8; i++) {\n\
         \x20   h ^= (u >> (i * 8)) & 0xffULL;\n\
         \x20   h *= 0x100000001b3ULL;\n\
         \x20 }\n\
         \x20 return h;\n\
         }\n\
         /* Closure #351: SipHash-2-4 — keyed adversarial-resistant\n\
         \x20  hash. Same shape as the FNV-1a family but takes an\n\
         \x20  explicit 128-bit key (two u64s) so callers in HashSet/\n\
         \x20  HashMap settings can resist hash-flooding attacks by\n\
         \x20  randomizing per process. The core helper hashes a byte\n\
         \x20  span; thin _i64 / _str wrappers pack a scalar / borrow\n\
         \x20  the C-string bytes. Matches the canonical SipHash-2-4\n\
         \x20  spec (2 compression rounds, 4 finalization rounds).\n\
         \x20  Validated against published test vectors. */\n\
         #define INTENT_SIP_ROTL(x, b) (((x) << (b)) | ((x) >> (64 - (b))))\n\
         #define INTENT_SIP_ROUND \\\n\
         \x20 v0 += v1; v1 = INTENT_SIP_ROTL(v1, 13); v1 ^= v0; v0 = INTENT_SIP_ROTL(v0, 32); \\\n\
         \x20 v2 += v3; v3 = INTENT_SIP_ROTL(v3, 16); v3 ^= v2; \\\n\
         \x20 v0 += v3; v3 = INTENT_SIP_ROTL(v3, 21); v3 ^= v0; \\\n\
         \x20 v2 += v1; v1 = INTENT_SIP_ROTL(v1, 17); v1 ^= v2; v2 = INTENT_SIP_ROTL(v2, 32)\n\
         static INTENT_UNUSED uint64_t intent_siphash24_bytes(uint64_t k0, uint64_t k1, const uint8_t* m, size_t n) {\n\
         \x20 uint64_t v0 = k0 ^ 0x736f6d6570736575ULL;\n\
         \x20 uint64_t v1 = k1 ^ 0x646f72616e646f6dULL;\n\
         \x20 uint64_t v2 = k0 ^ 0x6c7967656e657261ULL;\n\
         \x20 uint64_t v3 = k1 ^ 0x7465646279746573ULL;\n\
         \x20 size_t blocks = n / 8;\n\
         \x20 for (size_t i = 0; i < blocks; i++) {\n\
         \x20   uint64_t mw;\n\
         \x20   memcpy(&mw, m + i * 8, 8);\n\
         \x20   v3 ^= mw;\n\
         \x20   INTENT_SIP_ROUND; INTENT_SIP_ROUND;\n\
         \x20   v0 ^= mw;\n\
         \x20 }\n\
         \x20 size_t tail_off = blocks * 8;\n\
         \x20 size_t tail_n = n - tail_off;\n\
         \x20 uint64_t b = ((uint64_t)n) << 56;\n\
         \x20 for (size_t i = 0; i < tail_n; i++) {\n\
         \x20   b |= ((uint64_t)(m[tail_off + i])) << (i * 8);\n\
         \x20 }\n\
         \x20 v3 ^= b;\n\
         \x20 INTENT_SIP_ROUND; INTENT_SIP_ROUND;\n\
         \x20 v0 ^= b;\n\
         \x20 v2 ^= 0xff;\n\
         \x20 INTENT_SIP_ROUND; INTENT_SIP_ROUND; INTENT_SIP_ROUND; INTENT_SIP_ROUND;\n\
         \x20 return v0 ^ v1 ^ v2 ^ v3;\n\
         }\n\
         static INTENT_UNUSED uint64_t intent_siphash_i64(uint64_t k0, uint64_t k1, int64_t x) {\n\
         \x20 uint8_t buf[8];\n\
         \x20 memcpy(buf, &x, 8);\n\
         \x20 return intent_siphash24_bytes(k0, k1, buf, 8);\n\
         }\n\
         static INTENT_UNUSED uint64_t intent_siphash_str(uint64_t k0, uint64_t k1, const char* s) {\n\
         \x20 if (!s) return intent_siphash24_bytes(k0, k1, (const uint8_t*)\"\", 0);\n\
         \x20 return intent_siphash24_bytes(k0, k1, (const uint8_t*)s, strlen(s));\n\
         }\n\n",
    );
}

/// Walk the program for any `[i64; N]` type usage. The check
/// triggers emission of the array-i64 runtime helpers.
pub(crate) fn program_uses_i64_array(program: &TypedProgram) -> bool {
    fn ty_uses(ty: &Type) -> bool {
        match ty {
            Type::Array { element, .. } if matches!(**element, Type::I64) => true,
            Type::Array { element, .. } => ty_uses(element),
            Type::Vec(inner)
            | Type::Ref(inner)
            | Type::RefMut(inner)
            | Type::Atomic(inner)
            | Type::Mutex(inner)
            | Type::Guard(inner) => ty_uses(inner),
            Type::Channel(inner, _) => ty_uses(inner),
            Type::Tuple(es) => es.iter().any(ty_uses),
            Type::FnPtr(ps, r) => ps.iter().any(ty_uses) || ty_uses(r),
            _ => false,
        }
    }
    for f in &program.functions {
        if ty_uses(&f.return_type) {
            return true;
        }
        for p in &f.params {
            if ty_uses(&p.ty) {
                return true;
            }
        }
        if function_body_uses_i64_array(&f.body) {
            return true;
        }
    }
    for s in &program.structs {
        for (_, fty) in &s.fields {
            if ty_uses(fty) {
                return true;
            }
        }
    }
    false
}

fn function_body_uses_i64_array(stmts: &[crate::ir::TypedStmt]) -> bool {
    use crate::ir::TypedStmt as S;
    fn ty_uses(ty: &Type) -> bool {
        matches!(ty, Type::Array { element, .. } if matches!(**element, Type::I64))
            || match ty {
                Type::Array { element, .. } => ty_uses(element),
                Type::Vec(i) | Type::Ref(i) | Type::RefMut(i) => ty_uses(i),
                _ => false,
            }
    }
    for s in stmts {
        match s {
            S::Let { ty, .. } | S::Reassign { ty, .. } | S::Drop { ty, .. } => {
                if ty_uses(ty) {
                    return true;
                }
            }
            S::If {
                then_body,
                else_body,
                ..
            } => {
                if function_body_uses_i64_array(then_body)
                    || function_body_uses_i64_array(else_body)
                {
                    return true;
                }
            }
            S::While { body, .. } | S::For { body, .. } | S::ForIter { body, .. } => {
                if function_body_uses_i64_array(body) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

fn emit_intent_array_helpers_i64_unconditional(out: &mut String, has_option_i64: bool) {
    out.push_str(
        "typedef int64_t (*intent_array_int64_t__cmp_fn)(int64_t, int64_t);\n\
         static INTENT_UNUSED int64_t intent_array_int64_t__cmp_ascending(int64_t a, int64_t b) {\n\
         \x20 return (a > b) - (a < b);\n\
         }\n\
         static INTENT_UNUSED void intent_array_int64_t__qsort_impl(int64_t* a, int64_t lo, int64_t hi, intent_array_int64_t__cmp_fn cmp) {\n\
         \x20 while (lo < hi) {\n\
         \x20   if (hi - lo < 16) {\n\
         \x20     for (int64_t i = lo + 1; i <= hi; i++) {\n\
         \x20       int64_t key = a[i];\n\
         \x20       int64_t j = i - 1;\n\
         \x20       while (j >= lo && cmp(a[j], key) > 0) {\n\
         \x20         a[j + 1] = a[j];\n\
         \x20         j--;\n\
         \x20       }\n\
         \x20       a[j + 1] = key;\n\
         \x20     }\n\
         \x20     return;\n\
         \x20   }\n\
         \x20   int64_t mid = lo + (hi - lo) / 2;\n\
         \x20   int64_t pivot = a[mid];\n\
         \x20   int64_t i = lo - 1;\n\
         \x20   int64_t j = hi + 1;\n\
         \x20   for (;;) {\n\
         \x20     do { i++; } while (cmp(a[i], pivot) < 0);\n\
         \x20     do { j--; } while (cmp(a[j], pivot) > 0);\n\
         \x20     if (i >= j) break;\n\
         \x20     int64_t tmp = a[i]; a[i] = a[j]; a[j] = tmp;\n\
         \x20   }\n\
         \x20   if (j - lo < hi - (j + 1)) {\n\
         \x20     intent_array_int64_t__qsort_impl(a, lo, j, cmp);\n\
         \x20     lo = j + 1;\n\
         \x20   } else {\n\
         \x20     intent_array_int64_t__qsort_impl(a, j + 1, hi, cmp);\n\
         \x20     hi = j;\n\
         \x20   }\n\
         \x20 }\n\
         }\n\
         static INTENT_UNUSED int64_t intent_array_int64_t__sort(int64_t* a, uint64_t n) {\n\
         \x20 if (n > 1) intent_array_int64_t__qsort_impl(a, 0, (int64_t)n - 1, intent_array_int64_t__cmp_ascending);\n\
         \x20 return 0;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_array_int64_t__sort_by(int64_t* a, uint64_t n, intent_array_int64_t__cmp_fn cmp) {\n\
         \x20 if (n > 1) intent_array_int64_t__qsort_impl(a, 0, (int64_t)n - 1, cmp);\n\
         \x20 return 0;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_array_int64_t__reverse(int64_t* a, uint64_t n) {\n\
         \x20 if (n < 2) return 0;\n\
         \x20 uint64_t i = 0; uint64_t j = n - 1;\n\
         \x20 while (i < j) {\n\
         \x20   int64_t tmp = a[i]; a[i] = a[j]; a[j] = tmp;\n\
         \x20   i++; j--;\n\
         \x20 }\n\
         \x20 return 0;\n\
         }\n\
         static INTENT_UNUSED bool intent_array_int64_t__contains(const int64_t* a, uint64_t n, int64_t needle) {\n\
         \x20 for (uint64_t i = 0; i < n; i++) { if (a[i] == needle) return true; }\n\
         \x20 return false;\n\
         }\n",
    );
    if has_option_i64 {
        out.push_str(
            "static INTENT_UNUSED Enum_Option__i64 intent_array_int64_t__find(const int64_t* a, uint64_t n, int64_t needle) {\n\
             \x20 Enum_Option__i64 r; bool found = false; uint64_t idx = 0;\n\
             \x20 for (idx = 0; idx < n; idx++) { if (a[idx] == needle) { found = true; break; } }\n\
             \x20 if (found) { r.tag = 0; r.payload = (int64_t)idx; } else { r.tag = 1; r.payload = 0; }\n\
             \x20 return r;\n\
             }\n\
             static INTENT_UNUSED Enum_Option__i64 intent_array_int64_t__binary_search(const int64_t* a, uint64_t n, int64_t needle) {\n\
             \x20 Enum_Option__i64 r; int64_t lo = 0; int64_t hi = (int64_t)n - 1; bool found = false; int64_t mid = 0;\n\
             \x20 while (lo <= hi) {\n\
             \x20   mid = lo + (hi - lo) / 2;\n\
             \x20   int64_t v = a[mid];\n\
             \x20   if (v == needle) { found = true; break; }\n\
             \x20   else if (v < needle) lo = mid + 1;\n\
             \x20   else hi = mid - 1;\n\
             \x20 }\n\
             \x20 if (found) { r.tag = 0; r.payload = mid; } else { r.tag = 1; r.payload = 0; }\n\
             \x20 return r;\n\
             }\n",
        );
    }
    out.push('\n');
}

/// Emit the runtime helpers for `Channel<i64>` and `Mutex<i64>`
/// when the generated body references them. Both helpers are
/// header-only (static inline) so they participate in linkage
/// without an out-of-tree runtime library; the C11 atomics
/// `<stdatomic.h>` include is already in the preamble. The
/// substring check on `body` keeps the helpers from showing up
/// in programs that don't use them.
/// Emit a per-(T, N) Vyukov MPSC ring buffer struct + the
/// three operation helpers (new / send / recv). The struct
/// layout mirrors what the previous i64/16-only emit produced;
/// the element width and capacity are now substituted in.
fn emit_channel_bundle(element: &Type, capacity: u64, out: &mut String) {
    let struct_name = c_channel_storage(element, capacity);
    let struct_name_upper = struct_name.to_uppercase();
    let cap_macro = format!("{}_CAP", struct_name_upper);
    let mask_expr = format!("({} - 1)", cap_macro);
    let elem_c = c_leaf_type(element);
    let new_fn = c_channel_helper(element, capacity, "new");
    let send_fn = c_channel_helper(element, capacity, "send");
    let recv_fn = c_channel_helper(element, capacity, "recv");
    out.push_str(&format!(
        "#define {cap} {capacity}\n\
         typedef struct {{\n\
         \x20 {elem} buf[{cap}];\n\
         \x20 /* Per-slot publication sequence number — Vyukov MPSC.\n\
         \x20    seq[i]==n means slot i is in round n. Producer enters round\n\
         \x20    t when seq[t & MASK]==t and publishes via seq=t+1; consumer\n\
         \x20    enters round h when seq==h+1 and releases via seq=h+CAP. */\n\
         \x20 _Atomic int64_t seq[{cap}];\n\
         \x20 _Atomic int64_t head;\n\
         \x20 _Atomic int64_t tail;\n\
         }} {struct_name};\n\
         static {elem} {send}({struct_name}* c, {elem} v) INTENT_UNUSED;\n\
         static {elem} {send}({struct_name}* c, {elem} v) {{\n\
         \x20 int64_t t;\n\
         \x20 while (1) {{\n\
         \x20   t = atomic_load_explicit(&c->tail, memory_order_seq_cst);\n\
         \x20   int64_t s = atomic_load_explicit(&c->seq[t & {mask}], memory_order_seq_cst);\n\
         \x20   int64_t diff = s - t;\n\
         \x20   if (diff == 0) {{\n\
         \x20     int64_t expected = t;\n\
         \x20     if (atomic_compare_exchange_strong_explicit(&c->tail, &expected, t + 1, memory_order_seq_cst, memory_order_seq_cst)) {{\n\
         \x20       break;\n\
         \x20     }}\n\
         \x20   }} else if (diff < 0) {{\n\
         \x20     /* channel full — slot t still holds round t-CAP data */\n\
         \x20   }}\n\
         \x20   /* else: another producer raced ahead; reload tail */\n\
         \x20 }}\n\
         \x20 c->buf[t & {mask}] = v;\n\
         \x20 atomic_store_explicit(&c->seq[t & {mask}], t + 1, memory_order_seq_cst);\n\
         \x20 return v;\n\
         }}\n\
         static {elem} {recv}({struct_name}* c) INTENT_UNUSED;\n\
         static {elem} {recv}({struct_name}* c) {{\n\
         \x20 int64_t h = atomic_load_explicit(&c->head, memory_order_seq_cst);\n\
         \x20 while (1) {{\n\
         \x20   int64_t s = atomic_load_explicit(&c->seq[h & {mask}], memory_order_seq_cst);\n\
         \x20   if (s == h + 1) break;\n\
         \x20 }}\n\
         \x20 {elem} v = c->buf[h & {mask}];\n\
         \x20 atomic_store_explicit(&c->seq[h & {mask}], h + {cap}, memory_order_seq_cst);\n\
         \x20 atomic_store_explicit(&c->head, h + 1, memory_order_seq_cst);\n\
         \x20 return v;\n\
         }}\n\
         static {struct_name} {new}(void) INTENT_UNUSED;\n\
         static {struct_name} {new}(void) {{\n\
         \x20 {struct_name} c;\n\
         \x20 for (int i = 0; i < {cap}; i++) c.buf[i] = ({elem})0;\n\
         \x20 for (int i = 0; i < {cap}; i++) atomic_store_explicit(&c.seq[i], (int64_t)i, memory_order_seq_cst);\n\
         \x20 atomic_store_explicit(&c.head, 0, memory_order_seq_cst);\n\
         \x20 atomic_store_explicit(&c.tail, 0, memory_order_seq_cst);\n\
         \x20 return c;\n\
         }}\n\n",
        cap = cap_macro,
        capacity = capacity,
        elem = elem_c,
        mask = mask_expr,
        struct_name = struct_name,
        new = new_fn,
        send = send_fn,
        recv = recv_fn,
    ));
}

fn emit_concurrency_runtime_helpers(
    out: &mut String,
    body: &str,
    channel_specs: &[(Type, u64)],
) {
    let needs_mutex = body.contains("intent_mutex_i64") || body.contains("intent_guard_i64");
    let needs_condvar = body.contains("intent_condvar");
    let needs_tasks = body.contains("intent_task_handle");
    if needs_tasks {
        // Handle: pthread thread id + ctx pointer (for free
        // at join time). Task body lowering emits an outline
        // function per spawn site whose signature is
        // `void* fn(void* ctx)`.
        out.push_str(
            "typedef struct { intent_thread_t thread; void* ctx; } intent_task_handle;\n\n",
        );
    }
    for (element, capacity) in channel_specs {
        emit_channel_bundle(element, *capacity, out);
    }
    if needs_mutex {
        emit_intent_mutex_helpers_c(out);
    }
    if needs_condvar {
        emit_intent_condvar_helpers_c(out);
    }
}

/// Closure: condition-variable runtime helpers. The cv state
/// (a `_Atomic int seq`) lives by-value on the stack — like
/// `Mutex<i64>` and `Guard<i64>` — so the Drop is a no-op (no
/// heap to free). All entrypoints take `intent_condvar*` so the
/// affine handle's `ref cv` parameter shape works uniformly.
/// `wait` snapshots the seq under the mutex, releases the
/// mutex, kernel-waits (futex/WaitOnAddress), then re-acquires
/// the mutex on wake. The seq trick prevents lost notifies: a
/// notify between release and park advances seq, so the kernel
/// wait returns immediately.
pub(crate) fn emit_intent_condvar_helpers_c(out: &mut String) {
    out.push_str(
        "/* Condition-variable runtime (stack-by-value, like Mutex). */\n\
             typedef struct { _Atomic int seq; } intent_condvar;\n\
             static intent_condvar intent_condvar_new(void) INTENT_UNUSED;\n\
             static intent_condvar intent_condvar_new(void) {\n\
             \x20 intent_condvar cv;\n\
             \x20 atomic_store_explicit(&cv.seq, 0, memory_order_seq_cst);\n\
             \x20 return cv;\n\
             }\n\
             static int64_t intent_condvar_wait(intent_condvar* cv, intent_guard_i64* g) INTENT_UNUSED;\n\
             static int64_t intent_condvar_wait(intent_condvar* cv, intent_guard_i64* g) {\n\
             \x20 int snapshot = atomic_load_explicit(&cv->seq, memory_order_seq_cst);\n\
             \x20 /* Release the mutex while we wait. */\n\
             \x20 intent_guard_i64_unlock(g);\n\
             #if defined(__linux__) || defined(_WIN32)\n\
             \x20 intent_mutex_futex_wait(&cv->seq, snapshot);\n\
             #else\n\
             \x20 /* Other platforms: brief spin + yield until the seq\n\
             \x20    counter advances. Less efficient but correct. */\n\
             \x20 while (atomic_load_explicit(&cv->seq, memory_order_seq_cst) == snapshot) {\n\
             \x20   intent_thread_yield();\n\
             \x20 }\n\
             #endif\n\
             \x20 /* Re-acquire the mutex so the caller's guard is valid on\n\
             \x20    return. We re-lock the SAME underlying mutex. */\n\
             \x20 intent_guard_i64 reacquired = intent_mutex_i64_lock(g->m);\n\
             \x20 (void)reacquired;\n\
             \x20 return 0;\n\
             }\n\
             static bool intent_condvar_wait_timeout(intent_condvar* cv, intent_guard_i64* g, int64_t timeout_ms) INTENT_UNUSED;\n\
             static bool intent_condvar_wait_timeout(intent_condvar* cv, intent_guard_i64* g, int64_t timeout_ms) {\n\
             \x20 int snapshot = atomic_load_explicit(&cv->seq, memory_order_seq_cst);\n\
             \x20 intent_guard_i64_unlock(g);\n\
             \x20 bool notified = false;\n\
             #if defined(__linux__)\n\
             \x20 struct timespec ts;\n\
             \x20 ts.tv_sec = (time_t)(timeout_ms / 1000);\n\
             \x20 ts.tv_nsec = (long)((timeout_ms % 1000) * 1000000L);\n\
             \x20 long rc = syscall(SYS_futex, (int*)&cv->seq, FUTEX_WAIT_PRIVATE, snapshot, &ts, (void*)0, 0);\n\
             \x20 notified = (rc == 0) ||\n\
             \x20            (atomic_load_explicit(&cv->seq, memory_order_seq_cst) != snapshot);\n\
             #elif defined(_WIN32)\n\
             \x20 int compare = snapshot;\n\
             \x20 BOOL ok = WaitOnAddress((volatile VOID*)&cv->seq, &compare, sizeof(int), (DWORD)timeout_ms);\n\
             \x20 notified = ok &&\n\
             \x20            (atomic_load_explicit(&cv->seq, memory_order_seq_cst) != snapshot);\n\
             #else\n\
             \x20 /* Other platforms: spin with yield, abort at deadline. */\n\
             \x20 int64_t spent = 0;\n\
             \x20 while (atomic_load_explicit(&cv->seq, memory_order_seq_cst) == snapshot && spent < timeout_ms) {\n\
             \x20   intent_thread_yield();\n\
             \x20   spent += 1;\n\
             \x20 }\n\
             \x20 notified = (atomic_load_explicit(&cv->seq, memory_order_seq_cst) != snapshot);\n\
             #endif\n\
             \x20 intent_guard_i64 reacquired = intent_mutex_i64_lock(g->m);\n\
             \x20 (void)reacquired;\n\
             \x20 return notified;\n\
             }\n\
             static int64_t intent_condvar_notify_one(intent_condvar* cv) INTENT_UNUSED;\n\
             static int64_t intent_condvar_notify_one(intent_condvar* cv) {\n\
             \x20 atomic_fetch_add_explicit(&cv->seq, 1, memory_order_seq_cst);\n\
             #if defined(__linux__) || defined(_WIN32)\n\
             \x20 intent_mutex_futex_wake(&cv->seq, 1);\n\
             #endif\n\
             \x20 return 0;\n\
             }\n\
             static int64_t intent_condvar_notify_all(intent_condvar* cv) INTENT_UNUSED;\n\
             static int64_t intent_condvar_notify_all(intent_condvar* cv) {\n\
             \x20 atomic_fetch_add_explicit(&cv->seq, 1, memory_order_seq_cst);\n\
             #if defined(__linux__) || defined(_WIN32)\n\
             \x20 intent_mutex_futex_wake(&cv->seq, 0x7fffffff);\n\
             #endif\n\
             \x20 return 0;\n\
             }\n\n",
    );
}

/// Emit the i64-only `Mutex` / `Guard` runtime helpers
/// (Drepper three-state futex lock on Linux,
/// WaitOnAddress/WakeByAddress on Windows, sched_yield
/// fallback elsewhere). Shared between tree-C and SSA-C —
/// always-safe to emit, but typically only fires when the
/// program actually uses `Mutex<i64>` / `Guard<i64>` (the
/// caller does the substring check).
pub(crate) fn emit_intent_mutex_helpers_c(out: &mut String) {
    out.push_str(
        "/* Drepper-style three-state futex lock. State 0 = unlocked, 1 =\n\
             \x20  locked-no-waiters, 2 = locked-waiters-present. Lock attempts\n\
             \x20  CAS 0->1 for the uncontended fast path; on contention it\n\
             \x20  marks state=2 (atomic_exchange) then parks in the kernel via\n\
             \x20  the host's kernel-wait primitive (futex on Linux,\n\
             \x20  WaitOnAddress on Windows) until the unlocker stores 0 and\n\
             \x20  wakes it. Unlock optimizes for the no-waiters case: an\n\
             \x20  `atomic_fetch_sub` of 1 against state returns 1 on the\n\
             \x20  fast path (was-1, now-0; nothing to wake); on the slow\n\
             \x20  path it returns 2, the unlocker resets state to 0 and\n\
             \x20  wakes one waiter. Other platforms fall back to the\n\
             \x20  intent_thread_yield backoff. */\n\
             #if defined(__linux__)\n\
             # include <linux/futex.h>\n\
             # include <sys/syscall.h>\n\
             # include <unistd.h>\n\
             static long intent_mutex_futex_wait(_Atomic int* p, int v) INTENT_UNUSED;\n\
             static long intent_mutex_futex_wait(_Atomic int* p, int v) {\n\
             \x20 return syscall(SYS_futex, (int*)p, FUTEX_WAIT_PRIVATE, v, (void*)0, (void*)0, 0);\n\
             }\n\
             static long intent_mutex_futex_wake(_Atomic int* p, int n) INTENT_UNUSED;\n\
             static long intent_mutex_futex_wake(_Atomic int* p, int n) {\n\
             \x20 return syscall(SYS_futex, (int*)p, FUTEX_WAKE_PRIVATE, n, (void*)0, (void*)0, 0);\n\
             }\n\
             #elif defined(_WIN32)\n\
             static long intent_mutex_futex_wait(_Atomic int* p, int v) INTENT_UNUSED;\n\
             static long intent_mutex_futex_wait(_Atomic int* p, int v) {\n\
             \x20 int compare = v;\n\
             \x20 WaitOnAddress((volatile VOID*)p, &compare, sizeof(int), INFINITE);\n\
             \x20 return 0;\n\
             }\n\
             static long intent_mutex_futex_wake(_Atomic int* p, int n) INTENT_UNUSED;\n\
             static long intent_mutex_futex_wake(_Atomic int* p, int n) {\n\
             \x20 if (n == 1) WakeByAddressSingle((PVOID)p);\n\
             \x20 else WakeByAddressAll((PVOID)p);\n\
             \x20 return 0;\n\
             }\n\
             #endif\n\
             typedef struct { int64_t value; _Atomic int locked; } intent_mutex_i64;\n\
             typedef struct { intent_mutex_i64* m; } intent_guard_i64;\n\
             static intent_mutex_i64 intent_mutex_i64_new(int64_t initial) INTENT_UNUSED;\n\
             static intent_mutex_i64 intent_mutex_i64_new(int64_t initial) {\n\
             \x20 intent_mutex_i64 m;\n\
             \x20 m.value = initial;\n\
             \x20 atomic_store_explicit(&m.locked, 0, memory_order_seq_cst);\n\
             \x20 return m;\n\
             }\n\
             static intent_guard_i64 intent_mutex_i64_lock(intent_mutex_i64* m) INTENT_UNUSED;\n\
             static intent_guard_i64 intent_mutex_i64_lock(intent_mutex_i64* m) {\n\
             #if defined(__linux__) || defined(_WIN32)\n\
             \x20 int c = 0;\n\
             \x20 if (!atomic_compare_exchange_strong_explicit(&m->locked, &c, 1, memory_order_seq_cst, memory_order_seq_cst)) {\n\
             \x20   /* Slow path: mark state=2 (waiter present) then park. */\n\
             \x20   if (c != 2) c = atomic_exchange_explicit(&m->locked, 2, memory_order_seq_cst);\n\
             \x20   while (c != 0) {\n\
             \x20     intent_mutex_futex_wait(&m->locked, 2);\n\
             \x20     c = atomic_exchange_explicit(&m->locked, 2, memory_order_seq_cst);\n\
             \x20   }\n\
             \x20 }\n\
             #else\n\
             \x20 /* Other platforms: intent_thread_yield backoff (less efficient\n\
             \x20    but correct). See the futex/WaitOnAddress branch above. */\n\
             \x20 int expected = 0;\n\
             \x20 int spins = 0;\n\
             \x20 while (!atomic_compare_exchange_weak_explicit(&m->locked, &expected, 1, memory_order_seq_cst, memory_order_seq_cst)) {\n\
             \x20   expected = 0;\n\
             \x20   spins++;\n\
             \x20   if (spins >= 32) { intent_thread_yield(); spins = 0; }\n\
             \x20 }\n\
             #endif\n\
             \x20 intent_guard_i64 g;\n\
             \x20 g.m = m;\n\
             \x20 return g;\n\
             }\n\
             static int64_t intent_guard_i64_get(const intent_guard_i64* g) INTENT_UNUSED;\n\
             static int64_t intent_guard_i64_get(const intent_guard_i64* g) {\n\
             \x20 return g->m->value;\n\
             }\n\
             static int64_t intent_guard_i64_set(const intent_guard_i64* g, int64_t v) INTENT_UNUSED;\n\
             static int64_t intent_guard_i64_set(const intent_guard_i64* g, int64_t v) {\n\
             \x20 g->m->value = v;\n\
             \x20 return v;\n\
             }\n\
             static void intent_guard_i64_unlock(intent_guard_i64* g) INTENT_UNUSED;\n\
             static void intent_guard_i64_unlock(intent_guard_i64* g) {\n\
             #if defined(__linux__) || defined(_WIN32)\n\
             \x20 /* If the previous state was 1 (no waiters), the fetch_sub\n\
             \x20    leaves state at 0 and there's nothing to wake.  If it was\n\
             \x20    2 (waiters), reset state to 0 and wake one. */\n\
             \x20 if (atomic_fetch_sub_explicit(&g->m->locked, 1, memory_order_seq_cst) != 1) {\n\
             \x20   atomic_store_explicit(&g->m->locked, 0, memory_order_seq_cst);\n\
             \x20   intent_mutex_futex_wake(&g->m->locked, 1);\n\
             \x20 }\n\
             #else\n\
             \x20 atomic_store_explicit(&g->m->locked, 0, memory_order_seq_cst);\n\
             #endif\n\
             }\n\n",
    );
}

fn collect_vec_elements(
    ty: &Type,
    seen: &mut BTreeSet<String>,
    out: &mut Vec<Type>,
) {
    match ty {
        Type::Vec(element) => {
            // Recurse FIRST so inner element types are
            // pushed before the outer. The emit loop relies
            // on this order: emitting `intent_vec_vec_int64_t`
            // needs `intent_vec_int64_t`'s typedef already in
            // scope. Refines #7's #7c.
            collect_vec_elements(element, seen, out);
            // Dedup key must distinguish nested element types
            // (was: `c_leaf_type` which collapses every
            // Vec-of-X to `"/* vec */"`).
            let key = element_tag(element);
            if seen.insert(key) {
                out.push((**element).clone());
            }
        }
        Type::Array { element, .. } => collect_vec_elements(element, seen, out),
        Type::Ref(inner) | Type::RefMut(inner) => collect_vec_elements(inner, seen, out),
        // L2 follow-up (2026-06-08): `Box<Vec<T>>` as a struct
        // field type needs the inner Vec bundle pre-emitted
        // (the typedef references `intent_vec_<T>` in the
        // field's pointer-to spelling). Without this, structs
        // carrying Box<Vec<T>> emit before their dependency.
        Type::Box(inner) => collect_vec_elements(inner, seen, out),
        _ => {}
    }
}

fn collect_vec_elements_in_stmt(
    stmt: &TypedStmt,
    seen: &mut BTreeSet<String>,
    out: &mut Vec<Type>,
) {
    match stmt {
        TypedStmt::Let { ty, expr, .. } | TypedStmt::Reassign { ty, expr, .. } => {
            collect_vec_elements(ty, seen, out);
            collect_vec_elements_in_expr(expr, seen, out);
        }
        TypedStmt::Drop { ty, .. } => collect_vec_elements(ty, seen, out),
        TypedStmt::Discard { expr } => collect_vec_elements_in_expr(expr, seen, out),
        TypedStmt::Return { expr }
        | TypedStmt::Assert { expr, .. }
        | TypedStmt::Prove { expr } => collect_vec_elements_in_expr(expr, seen, out),
        TypedStmt::Print { items } => {
            for it in items {
                if let crate::ir::TypedPrintItem::Expr(e) = it {
                    collect_vec_elements_in_expr(e, seen, out);
                }
            }
        }
        TypedStmt::If {
            cond,
            then_body,
            else_body,
        } => {
            collect_vec_elements_in_expr(cond, seen, out);
            for s in then_body {
                collect_vec_elements_in_stmt(s, seen, out);
            }
            for s in else_body {
                collect_vec_elements_in_stmt(s, seen, out);
            }
        }
        TypedStmt::While { cond, body } => {
            collect_vec_elements_in_expr(cond, seen, out);
            for s in body {
                collect_vec_elements_in_stmt(s, seen, out);
            }
        }
        TypedStmt::Break | TypedStmt::Continue => {}
        TypedStmt::IndexAssign { index, value, .. } => {
            collect_vec_elements_in_expr(index, seen, out);
            collect_vec_elements_in_expr(value, seen, out);
        }
        TypedStmt::FieldAssign { object, value, .. } => {
            collect_vec_elements_in_expr(object, seen, out);
            collect_vec_elements_in_expr(value, seen, out);
        }
        TypedStmt::For {
            start, end, body, ..
        } => {
            collect_vec_elements_in_expr(start, seen, out);
            collect_vec_elements_in_expr(end, seen, out);
            for s in body {
                collect_vec_elements_in_stmt(s, seen, out);
            }
        }
        TypedStmt::ForIter {
            element_ty,
            collection_ty,
            body,
            ..
        } => {
            collect_vec_elements(element_ty, seen, out);
            collect_vec_elements(collection_ty, seen, out);
            for s in body {
                collect_vec_elements_in_stmt(s, seen, out);
            }
        }
        TypedStmt::TaskSpawn { body, .. } | TypedStmt::UnsafeBlock { body, .. } => {
            for s in body {
                collect_vec_elements_in_stmt(s, seen, out);
            }
        }
        TypedStmt::TaskJoin { .. } => {}
    }
}

fn collect_vec_elements_in_expr(
    expr: &TypedExpr,
    seen: &mut BTreeSet<String>,
    out: &mut Vec<Type>,
) {
    collect_vec_elements(&expr.ty, seen, out);
    match &expr.kind {
        TypedExprKind::Unary { expr, .. } => collect_vec_elements_in_expr(expr, seen, out),
        TypedExprKind::Binary { left, right, .. } => {
            collect_vec_elements_in_expr(left, seen, out);
            collect_vec_elements_in_expr(right, seen, out);
        }
        TypedExprKind::Call { args, .. } | TypedExprKind::ArrayLit { elements: args } => {
            for arg in args {
                collect_vec_elements_in_expr(arg, seen, out);
            }
        }
        TypedExprKind::Cast { expr, .. } => collect_vec_elements_in_expr(expr, seen, out),
        TypedExprKind::Index { array, index, .. } => {
            collect_vec_elements_in_expr(array, seen, out);
            collect_vec_elements_in_expr(index, seen, out);
        }
        TypedExprKind::Len { array, .. } => collect_vec_elements_in_expr(array, seen, out),
        TypedExprKind::Tuple { elements } => {
            for e in elements {
                collect_vec_elements_in_expr(e, seen, out);
            }
        }
        TypedExprKind::StructLit { fields, .. } => {
            for (_, e) in fields {
                collect_vec_elements_in_expr(e, seen, out);
            }
        }
        TypedExprKind::FieldAccess { object, .. } => {
            collect_vec_elements_in_expr(object, seen, out);
        }
        TypedExprKind::TupleAccess { tuple, .. } => {
            collect_vec_elements_in_expr(tuple, seen, out);
        }
        TypedExprKind::EnumVariantWithPayload { payload, .. } => {
            collect_vec_elements_in_expr(payload, seen, out);
        }
        TypedExprKind::Match { scrutinee, arms } => {
            collect_vec_elements_in_expr(scrutinee, seen, out);
            for arm in arms {
                collect_vec_elements_in_expr(&arm.body, seen, out);
            }
        }
        TypedExprKind::IfExpr { cond, then_value, else_value } => {
            collect_vec_elements_in_expr(cond, seen, out);
            collect_vec_elements_in_expr(then_value, seen, out);
            collect_vec_elements_in_expr(else_value, seen, out);
        }
        TypedExprKind::Block { stmts, tail } => {
            for s in stmts {
                collect_vec_elements_in_stmt(s, seen, out);
            }
            collect_vec_elements_in_expr(tail, seen, out);
        }
        _ => {}
    }
}

/// Walk every type position reachable from `ty` and record
/// distinct tuple shapes (keyed on `tuple_c_struct` name) into
/// `out`. Inner-first so a future nested-tuple shape appears
/// before any outer that references it. T1.1.
fn collect_tuple_shapes(
    ty: &Type,
    seen: &mut BTreeSet<String>,
    out: &mut Vec<Vec<Type>>,
) {
    match ty {
        Type::Tuple(elements) => {
            for e in elements {
                collect_tuple_shapes(e, seen, out);
            }
            let key = tuple_c_struct(elements);
            if seen.insert(key) {
                out.push(elements.clone());
            }
        }
        Type::Vec(inner)
        | Type::Ref(inner)
        | Type::RefMut(inner)
        | Type::Atomic(inner)
        | Type::Mutex(inner)
        | Type::Guard(inner) => collect_tuple_shapes(inner, seen, out),
        Type::Array { element, .. } => collect_tuple_shapes(element, seen, out),
        Type::Channel(element, _) => collect_tuple_shapes(element, seen, out),
        Type::FnPtr(params, ret) => {
            for p in params {
                collect_tuple_shapes(p, seen, out);
            }
            collect_tuple_shapes(ret, seen, out);
        }
        _ => {}
    }
}

fn collect_tuple_shapes_in_stmt(
    stmt: &TypedStmt,
    seen: &mut BTreeSet<String>,
    out: &mut Vec<Vec<Type>>,
) {
    match stmt {
        TypedStmt::Let { ty, expr, .. } | TypedStmt::Reassign { ty, expr, .. } => {
            collect_tuple_shapes(ty, seen, out);
            collect_tuple_shapes_in_expr(expr, seen, out);
        }
        TypedStmt::Drop { ty, .. } => collect_tuple_shapes(ty, seen, out),
        TypedStmt::Discard { expr } => collect_tuple_shapes_in_expr(expr, seen, out),
        TypedStmt::Return { expr }
        | TypedStmt::Assert { expr, .. }
        | TypedStmt::Prove { expr } => collect_tuple_shapes_in_expr(expr, seen, out),
        TypedStmt::Print { items } => {
            for it in items {
                if let crate::ir::TypedPrintItem::Expr(e) = it {
                    collect_tuple_shapes_in_expr(e, seen, out);
                }
            }
        }
        TypedStmt::If { cond, then_body, else_body } => {
            collect_tuple_shapes_in_expr(cond, seen, out);
            for s in then_body {
                collect_tuple_shapes_in_stmt(s, seen, out);
            }
            for s in else_body {
                collect_tuple_shapes_in_stmt(s, seen, out);
            }
        }
        TypedStmt::While { cond, body } => {
            collect_tuple_shapes_in_expr(cond, seen, out);
            for s in body {
                collect_tuple_shapes_in_stmt(s, seen, out);
            }
        }
        TypedStmt::For { start, end, body, .. } => {
            collect_tuple_shapes_in_expr(start, seen, out);
            collect_tuple_shapes_in_expr(end, seen, out);
            for s in body {
                collect_tuple_shapes_in_stmt(s, seen, out);
            }
        }
        TypedStmt::ForIter { body, .. } => {
            for s in body {
                collect_tuple_shapes_in_stmt(s, seen, out);
            }
        }
        TypedStmt::IndexAssign { index, value, .. } => {
            collect_tuple_shapes_in_expr(index, seen, out);
            collect_tuple_shapes_in_expr(value, seen, out);
        }
        TypedStmt::FieldAssign { object, value, .. } => {
            collect_tuple_shapes_in_expr(object, seen, out);
            collect_tuple_shapes_in_expr(value, seen, out);
        }
        TypedStmt::TaskSpawn { body, .. } => {
            for s in body {
                collect_tuple_shapes_in_stmt(s, seen, out);
            }
        }
        _ => {}
    }
}

fn collect_tuple_shapes_in_expr(
    expr: &TypedExpr,
    seen: &mut BTreeSet<String>,
    out: &mut Vec<Vec<Type>>,
) {
    collect_tuple_shapes(&expr.ty, seen, out);
    match &expr.kind {
        TypedExprKind::Tuple { elements } => {
            for e in elements {
                collect_tuple_shapes_in_expr(e, seen, out);
            }
        }
        TypedExprKind::TupleAccess { tuple, .. } => {
            collect_tuple_shapes_in_expr(tuple, seen, out);
        }
        TypedExprKind::Unary { expr, .. } => {
            collect_tuple_shapes_in_expr(expr, seen, out)
        }
        TypedExprKind::Binary { left, right, .. } => {
            collect_tuple_shapes_in_expr(left, seen, out);
            collect_tuple_shapes_in_expr(right, seen, out);
        }
        TypedExprKind::Call { args, .. } | TypedExprKind::ArrayLit { elements: args } => {
            for a in args {
                collect_tuple_shapes_in_expr(a, seen, out);
            }
        }
        TypedExprKind::Cast { expr, .. } => collect_tuple_shapes_in_expr(expr, seen, out),
        TypedExprKind::Index { array, index, .. } => {
            collect_tuple_shapes_in_expr(array, seen, out);
            collect_tuple_shapes_in_expr(index, seen, out);
        }
        TypedExprKind::Len { array, .. } => collect_tuple_shapes_in_expr(array, seen, out),
        TypedExprKind::CallIndirect { callee, args } => {
            collect_tuple_shapes_in_expr(callee, seen, out);
            for a in args {
                collect_tuple_shapes_in_expr(a, seen, out);
            }
        }
        // Closure #198: control-flow expressions can carry
        // tuple-typed sub-expressions (e.g. inner Lets of a
        // Block-expr, branch values of an IfExpr/Match). The
        // collector previously fell through `_ => {}` for
        // these shapes, so a tuple type that only appeared
        // inside a Block-expr inner Let never got its
        // `intent_tuple_<…>` typedef emitted and cc rejected
        // with `unknown type name intent_tuple_<…>`.
        TypedExprKind::Block { stmts, tail } => {
            for s in stmts {
                collect_tuple_shapes_in_stmt(s, seen, out);
            }
            collect_tuple_shapes_in_expr(tail, seen, out);
        }
        TypedExprKind::IfExpr { cond, then_value, else_value } => {
            collect_tuple_shapes_in_expr(cond, seen, out);
            collect_tuple_shapes_in_expr(then_value, seen, out);
            collect_tuple_shapes_in_expr(else_value, seen, out);
        }
        TypedExprKind::Match { scrutinee, arms } => {
            collect_tuple_shapes_in_expr(scrutinee, seen, out);
            for arm in arms {
                collect_tuple_shapes_in_expr(&arm.body, seen, out);
            }
        }
        _ => {}
    }
}

/// Emit the C runtime helper `intent_str_concat` used by both
/// the tree-C backend and the SSA-C backend for Str/OwnedStr
/// `+` lowering. Allocates a fresh buffer, copies both
/// operands, NUL-terminates, and frees each operand whose
/// `_owned` flag is non-zero.
/// Emit the cross-platform `intent_thread_t` typedef plus
/// `intent_thread_create` / `intent_thread_join` /
/// `intent_thread_yield` wrappers. Dispatches on
/// `#if defined(_WIN32)` so the same C source links on
/// Linux/macOS (pthread) and Windows (CreateThread/
/// WaitForSingleObject/SwitchToThread). Shared between the
/// tree-C backend and the SSA-C backend (the SSA-C task
/// outlining references `intent_thread_create`/
/// `intent_thread_join`). Always emitted; small footprint.
pub(crate) fn emit_intent_thread_wrappers_c(out: &mut String) {
    out.push_str("#if defined(_WIN32)\n");
    out.push_str("# include <windows.h>\n");
    out.push_str("# include <synchapi.h>\n");
    out.push_str("typedef HANDLE intent_thread_t;\n");
    out.push_str("static int intent_thread_create(intent_thread_t* th, void* (*fn)(void*), void* arg) INTENT_UNUSED;\n");
    out.push_str("static int intent_thread_create(intent_thread_t* th, void* (*fn)(void*), void* arg) {\n");
    out.push_str("  *th = CreateThread(NULL, 0, (LPTHREAD_START_ROUTINE)fn, arg, 0, NULL);\n");
    out.push_str("  return *th == NULL ? -1 : 0;\n");
    out.push_str("}\n");
    out.push_str("static int intent_thread_join(intent_thread_t th) INTENT_UNUSED;\n");
    out.push_str("static int intent_thread_join(intent_thread_t th) {\n");
    out.push_str("  WaitForSingleObject(th, INFINITE);\n");
    out.push_str("  CloseHandle(th);\n");
    out.push_str("  return 0;\n");
    out.push_str("}\n");
    out.push_str("static void intent_thread_yield(void) INTENT_UNUSED;\n");
    out.push_str("static void intent_thread_yield(void) { SwitchToThread(); }\n");
    out.push_str("#else\n");
    out.push_str("# include <pthread.h>\n");
    out.push_str("# include <sched.h>\n");
    out.push_str("typedef pthread_t intent_thread_t;\n");
    out.push_str("static int intent_thread_create(intent_thread_t* th, void* (*fn)(void*), void* arg) INTENT_UNUSED;\n");
    out.push_str("static int intent_thread_create(intent_thread_t* th, void* (*fn)(void*), void* arg) {\n");
    out.push_str("  return pthread_create(th, NULL, fn, arg);\n");
    out.push_str("}\n");
    out.push_str("static int intent_thread_join(intent_thread_t th) INTENT_UNUSED;\n");
    out.push_str("static int intent_thread_join(intent_thread_t th) {\n");
    out.push_str("  return pthread_join(th, NULL);\n");
    out.push_str("}\n");
    out.push_str("static void intent_thread_yield(void) INTENT_UNUSED;\n");
    out.push_str("static void intent_thread_yield(void) { sched_yield(); }\n");
    out.push_str("#endif\n\n");
}

/// Phase 6 + 12 (2026-06-07): parameterized numeral-print
/// helper. Each script's digit codepoint encodes to a sequence
/// of UTF-8 bytes — Brahmi-derived scripts use a 3-byte form
/// (`E0 <lead> A6+d`), Arabic-Indic uses a 2-byte form
/// (`D9 A0+d`). The helper template emits `prefix_bytes`
/// literally then `base_byte + d` for the digit's last byte.
/// `suffix` is the function-name component (`dev` / `ben` /
/// `urd` / ...).
fn emit_intent_print_int_helper_c(
    out: &mut String,
    suffix: &str,
    prefix_bytes: &[u8],
    base_byte: u8,
) {
    let mut prefix_emit = String::new();
    for &b in prefix_bytes {
        prefix_emit.push_str(&format!("buf[o++] = (char){:#04x}; ", b));
    }
    out.push_str(&format!(
        "static INTENT_UNUSED void intent_print_int_{suffix}(long long n) {{\n\
         \x20 char ascii[24];\n\
         \x20 int len = snprintf(ascii, sizeof(ascii), \"%lld\", n);\n\
         \x20 if (len <= 0) return;\n\
         \x20 char buf[80];\n\
         \x20 int o = 0;\n\
         \x20 for (int i = 0; i < len; i++) {{\n\
         \x20   char c = ascii[i];\n\
         \x20   if (c == '-') {{ buf[o++] = '-'; continue; }}\n\
         \x20   int d = c - '0';\n\
         \x20   {prefix_emit}buf[o++] = (char)({base_byte:#04x} + d);\n\
         \x20 }}\n\
         \x20 buf[o] = '\\0';\n\
         \x20 fputs(buf, stdout);\n\
         }}\n\n",
        suffix = suffix,
        prefix_emit = prefix_emit,
        base_byte = base_byte,
    ));
}

pub(crate) fn emit_intent_print_int_dev_c(out: &mut String) {
    if matches!(crate::lexer::current_print_lang_mode(),
                crate::lexer::PrintLangMode::Devanagari) {
        // Devanagari '०..९' at U+0966..96F → UTF-8 `E0 A5 A6+d`.
        emit_intent_print_int_helper_c(out, "dev", &[0xE0, 0xA5], 0xA6);
    }
}

pub(crate) fn emit_intent_print_int_ben_c(out: &mut String) {
    if matches!(crate::lexer::current_print_lang_mode(),
                crate::lexer::PrintLangMode::Bengali) {
        // Bengali '০..৯' at U+09E6..9EF → UTF-8 `E0 A7 A6+d`.
        emit_intent_print_int_helper_c(out, "ben", &[0xE0, 0xA7], 0xA6);
    }
}

/// Phase 6 (2026-06-07): Tamil '௦..௯' at U+0BE6..0BEF.
pub(crate) fn emit_intent_print_int_tam_c(out: &mut String) {
    if matches!(crate::lexer::current_print_lang_mode(),
                crate::lexer::PrintLangMode::Tamil) {
        emit_intent_print_int_helper_c(out, "tam", &[0xE0, 0xAF], 0xA6);
    }
}

/// Phase 6 (2026-06-07): Telugu '౦..౯' at U+0C66..0C6F.
pub(crate) fn emit_intent_print_int_tel_c(out: &mut String) {
    if matches!(crate::lexer::current_print_lang_mode(),
                crate::lexer::PrintLangMode::Telugu) {
        emit_intent_print_int_helper_c(out, "tel", &[0xE0, 0xB1], 0xA6);
    }
}

/// Phase 6 (2026-06-07): Gujarati '૦..૯' at U+0AE6..0AEF.
pub(crate) fn emit_intent_print_int_guj_c(out: &mut String) {
    if matches!(crate::lexer::current_print_lang_mode(),
                crate::lexer::PrintLangMode::Gujarati) {
        emit_intent_print_int_helper_c(out, "guj", &[0xE0, 0xAB], 0xA6);
    }
}

/// Phase 6 (2026-06-07): Gurmukhi (Punjabi) '੦..੯' at U+0A66..0A6F.
pub(crate) fn emit_intent_print_int_pan_c(out: &mut String) {
    if matches!(crate::lexer::current_print_lang_mode(),
                crate::lexer::PrintLangMode::Gurmukhi) {
        emit_intent_print_int_helper_c(out, "pan", &[0xE0, 0xA9], 0xA6);
    }
}

/// Phase 6 second half (2026-06-07): remaining Brahmi-derived
/// scripts. Each follows the identical pattern — gate on
/// PrintLangMode, emit the helper with the matching middle
/// UTF-8 byte for that script's numeral block.
pub(crate) fn emit_intent_print_int_kan_c(out: &mut String) {
    if matches!(crate::lexer::current_print_lang_mode(),
                crate::lexer::PrintLangMode::Kannada) {
        // Kannada '೦..೯' at U+0CE6..0CEF → UTF-8 `E0 B3 A6+d`.
        emit_intent_print_int_helper_c(out, "kan", &[0xE0, 0xB3], 0xA6);
    }
}

pub(crate) fn emit_intent_print_int_mal_c(out: &mut String) {
    if matches!(crate::lexer::current_print_lang_mode(),
                crate::lexer::PrintLangMode::Malayalam) {
        // Malayalam '൦..൯' at U+0D66..0D6F → UTF-8 `E0 B5 A6+d`.
        emit_intent_print_int_helper_c(out, "mal", &[0xE0, 0xB5], 0xA6);
    }
}

pub(crate) fn emit_intent_print_int_odi_c(out: &mut String) {
    if matches!(crate::lexer::current_print_lang_mode(),
                crate::lexer::PrintLangMode::Odia) {
        // Odia '୦..୯' at U+0B66..0B6F → UTF-8 `E0 AD A6+d`.
        emit_intent_print_int_helper_c(out, "odi", &[0xE0, 0xAD], 0xA6);
    }
}

pub(crate) fn emit_intent_print_int_sin_c(out: &mut String) {
    if matches!(crate::lexer::current_print_lang_mode(),
                crate::lexer::PrintLangMode::Sinhala) {
        // Sinhala Lith Illakkam '෦..෯' at U+0DE6..0DEF → UTF-8 `E0 B7 A6+d`.
        emit_intent_print_int_helper_c(out, "sin", &[0xE0, 0xB7], 0xA6);
    }
}

/// Phase 12 (2026-06-07): Urdu — first Perso-Arabic script.
/// Eastern Arabic-Indic digits '٠..٩' at U+0660..0669 use a
/// 2-byte UTF-8 sequence `D9 A0+d` — the helper template
/// parameterizes on prefix-byte length to cover both 2-byte
/// Arabic-Indic and 3-byte Brahmi.
pub(crate) fn emit_intent_print_int_urd_c(out: &mut String) {
    if matches!(crate::lexer::current_print_lang_mode(),
                crate::lexer::PrintLangMode::Urdu) {
        emit_intent_print_int_helper_c(out, "urd", &[0xD9], 0xA0);
    }
}

/// Phase 12.4 (2026-06-07): Persian (Extended) Arabic-Indic
/// digits '۰..۹' at U+06F0..06F9 → UTF-8 `DB B0+d`. A second
/// non-Brahmi numeral block, distinct from Urdu's.
pub(crate) fn emit_intent_print_int_per_c(out: &mut String) {
    if matches!(crate::lexer::current_print_lang_mode(),
                crate::lexer::PrintLangMode::Persian) {
        emit_intent_print_int_helper_c(out, "per", &[0xDB], 0xB0);
    }
}

pub(crate) fn emit_intent_str_concat_c(out: &mut String) {
    out.push_str(
        "static char* intent_str_concat(const char* l, int l_owned, const char* r, int r_owned) INTENT_UNUSED;\n\
         static char* intent_str_concat(const char* l, int l_owned, const char* r, int r_owned) {\n\
         \x20 size_t ln = strlen(l), rn = strlen(r);\n\
         \x20 char* out = (char*)malloc(ln + rn + 1);\n\
         \x20 memcpy(out, l, ln);\n\
         \x20 memcpy(out + ln, r, rn);\n\
         \x20 out[ln + rn] = 0;\n\
         \x20 if (l_owned) free((void*)l);\n\
         \x20 if (r_owned) free((void*)r);\n\
         \x20 return out;\n\
         }\n\n",
    );
}

/// Closure #350: heap-allocating `str_split`. Returns a fresh
/// `Vec<OwnedStr>` containing the substrings of `s` between
/// non-overlapping occurrences of `delim`. Each element is a
/// freshly-malloc'd char* so the Vec's per-element Drop is
/// well-defined.
///
/// Edge cases:
///   - `delim` empty: returns a single-element vec containing
///     dup(s). (Splitting on "" otherwise diverges / has no
///     sensible default.)
///   - `s` empty: returns a vec with a single empty OwnedStr.
///   - No match: returns a vec with a single dup(s) element.
///   - Consecutive delims: empty OwnedStrs in between (matches
///     the canonical split semantics).
///
/// Gated by the `intent_vec_owned_str` typedef + helpers being
/// present, which the existing Vec-element walker auto-emits
/// when the program uses `Vec<OwnedStr>` anywhere — including
/// at this function's return type.
/// Closure #358: numeric-to-string conversion.
/// `i64_to_str(x: i64) -> OwnedStr` produces the decimal
/// representation of x as a freshly-malloc'd char*. Uses
/// `snprintf` for the format work; max representable length
/// (incl sign + NUL) is 21 bytes.
///
/// Closure #359: `f64_to_str(x: f64) -> OwnedStr` mirrors
/// the i64 form but with `%g` (compact representation that
/// avoids trailing zeros / picks between fixed and scientific
/// notation). 32-byte scratch buffer is enough for all
/// double-precision outputs in practice.
///
/// Closure #361: `bool_to_str(b: bool) -> OwnedStr` returns
/// a malloc'd copy of "true" / "false". Heap-allocated for
/// consistency with the other to_str helpers — the OwnedStr
/// return path lets the result be `+`-concatenated with other
/// OwnedStr / Str.
pub(crate) fn emit_intent_i64_to_str_c(out: &mut String) {
    out.push_str(
        "static char* intent_i64_to_str(int64_t x) INTENT_UNUSED;\n\
         static char* intent_i64_to_str(int64_t x) {\n\
         \x20 char buf[21];\n\
         \x20 int n = snprintf(buf, sizeof(buf), \"%lld\", (long long)x);\n\
         \x20 if (n < 0) abort();\n\
         \x20 char* out = (char*)malloc((size_t)n + 1);\n\
         \x20 if (!out) abort();\n\
         \x20 memcpy(out, buf, (size_t)n + 1);\n\
         \x20 return out;\n\
         }\n\
         static char* intent_f64_to_str(double x) INTENT_UNUSED;\n\
         static char* intent_f64_to_str(double x) {\n\
         \x20 char buf[32];\n\
         \x20 int n = snprintf(buf, sizeof(buf), \"%g\", x);\n\
         \x20 if (n < 0) abort();\n\
         \x20 if ((size_t)n >= sizeof(buf)) n = (int)(sizeof(buf) - 1);\n\
         \x20 char* out = (char*)malloc((size_t)n + 1);\n\
         \x20 if (!out) abort();\n\
         \x20 memcpy(out, buf, (size_t)n);\n\
         \x20 out[n] = 0;\n\
         \x20 return out;\n\
         }\n\
         static char* intent_bool_to_str(bool b) INTENT_UNUSED;\n\
         static char* intent_bool_to_str(bool b) {\n\
         \x20 const char* src = b ? \"true\" : \"false\";\n\
         \x20 size_t n = b ? 4 : 5;\n\
         \x20 char* out = (char*)malloc(n + 1);\n\
         \x20 if (!out) abort();\n\
         \x20 memcpy(out, src, n + 1);\n\
         \x20 return out;\n\
         }\n\n",
    );
}

/// Closure #357: Option<i64> ergonomic helpers — eliminate
/// the per-example `unwrap_or` boilerplate users were
/// hand-writing.
///
///   option_unwrap_or(o, def) -> i64    payload if Some(_), else def
///   option_is_some(o)        -> bool   tag == 0
///   option_is_none(o)        -> bool   tag != 0
///
/// All operate on the `Enum_Option__i64` struct (tag: i32,
/// payload: i64). Caller passes by-value; the tag and payload
/// are read directly from the struct fields.
pub(crate) fn emit_intent_option_i64_helpers_c(out: &mut String) {
    out.push_str(
        "static INTENT_UNUSED int64_t intent_option_i64_unwrap_or(Enum_Option__i64 o, int64_t def) INTENT_UNUSED;\n\
         static INTENT_UNUSED int64_t intent_option_i64_unwrap_or(Enum_Option__i64 o, int64_t def) {\n\
         \x20 return (o.tag == 0) ? (int64_t)o.payload : def;\n\
         }\n\
         static INTENT_UNUSED bool intent_option_i64_is_some(Enum_Option__i64 o) INTENT_UNUSED;\n\
         static INTENT_UNUSED bool intent_option_i64_is_some(Enum_Option__i64 o) {\n\
         \x20 return o.tag == 0;\n\
         }\n\
         static INTENT_UNUSED bool intent_option_i64_is_none(Enum_Option__i64 o) INTENT_UNUSED;\n\
         static INTENT_UNUSED bool intent_option_i64_is_none(Enum_Option__i64 o) {\n\
         \x20 return o.tag != 0;\n\
         }\n\n",
    );
}

/// Closure #360: Option<f64> ergonomic helpers (parallels #357
/// but on `Enum_Option__f64`, which is already plumbed for
/// parse_float's return type).
pub(crate) fn emit_intent_option_f64_helpers_c(out: &mut String) {
    out.push_str(
        "static INTENT_UNUSED double intent_option_f64_unwrap_or(Enum_Option__f64 o, double def) INTENT_UNUSED;\n\
         static INTENT_UNUSED double intent_option_f64_unwrap_or(Enum_Option__f64 o, double def) {\n\
         \x20 return (o.tag == 0) ? o.payload : def;\n\
         }\n\
         static INTENT_UNUSED bool intent_option_f64_is_some(Enum_Option__f64 o) INTENT_UNUSED;\n\
         static INTENT_UNUSED bool intent_option_f64_is_some(Enum_Option__f64 o) {\n\
         \x20 return o.tag == 0;\n\
         }\n\
         static INTENT_UNUSED bool intent_option_f64_is_none(Enum_Option__f64 o) INTENT_UNUSED;\n\
         static INTENT_UNUSED bool intent_option_f64_is_none(Enum_Option__f64 o) {\n\
         \x20 return o.tag != 0;\n\
         }\n\n",
    );
}

/// Closure #356: Vec<i64> utility helpers — small constructors
/// and combinators that operate on the existing
/// `intent_vec_int64_t` struct.
///
///   vec_range(lo, hi) -> Vec<i64>     [lo, lo+1, ..., hi-1]
///   vec_repeat(v, n)  -> Vec<i64>     n copies of v
///   vec_extend(mut ref xs, ref ys) -> i64    appends ys to xs;
///                                            returns new len
///   vec_concat(ref xs, ref ys)     -> Vec<i64>   fresh xs ++ ys
/// Closures #593-#596: detect any vec_chunks / vec_windows /
/// vec_flatten / vec_group_by_value call. All four helpers
/// reference `intent_vec_vec_int64_t` (the nested-Vec struct)
/// which is only declared when the program uses Vec<Vec<i64>>,
/// so the helper emission must be gated.
pub(crate) fn program_uses_vec_chunks(program: &TypedProgram) -> bool {
    // Substring-on-emitted-IR gate, like other on-demand helpers.
    // We can't directly inspect the typed program here without
    // re-traversing — but since the helper is small and only
    // matters when the symbol literally appears, we just walk
    // function bodies looking for any "vec_chunks" name.
    use crate::ir::TypedExprKind as E;
    use crate::ir::TypedStmt as S;
    fn expr_uses(expr: &crate::ir::TypedExpr) -> bool {
        match &expr.kind {
            E::Call { name, args, .. } => {
                if matches!(name.as_str(),
                    "vec_chunks" | "vec_windows" | "vec_flatten"
                    | "vec_group_by_value"
                ) { return true; }
                args.iter().any(expr_uses)
            }
            E::Unary { expr, .. } | E::Cast { expr, .. } => expr_uses(expr),
            E::Len { array, .. } => expr_uses(array),
            E::Binary { left, right, .. } => expr_uses(left) || expr_uses(right),
            E::CallIndirect { callee, args } => {
                expr_uses(callee) || args.iter().any(expr_uses)
            }
            E::ArrayLit { elements } => elements.iter().any(expr_uses),
            E::Index { array, index, .. } => expr_uses(array) || expr_uses(index),
            E::Tuple { elements } => elements.iter().any(expr_uses),
            E::TupleAccess { tuple, .. } => expr_uses(tuple),
            E::StructLit { fields, .. } => fields.iter().any(|(_, v)| expr_uses(v)),
            E::FieldAccess { object, .. } => expr_uses(object),
            E::EnumVariantWithPayload { payload, .. } => expr_uses(payload),
            E::IfExpr { cond, then_value, else_value } => {
                expr_uses(cond) || expr_uses(then_value) || expr_uses(else_value)
            }
            E::Match { scrutinee, arms } => {
                expr_uses(scrutinee) || arms.iter().any(|a| expr_uses(&a.body))
            }
            E::Block { stmts, tail } => {
                stmts.iter().any(stmt_walk) || expr_uses(tail)
            }
            _ => false,
        }
    }
    fn stmt_walk(s: &S) -> bool {
        match s {
            S::Let { expr, .. }
            | S::Reassign { expr, .. }
            | S::Return { expr }
            | S::Assert { expr, .. }
            | S::Prove { expr } => expr_uses(expr),
            S::Discard { expr } => expr_uses(expr),
            S::Print { items } => items.iter().any(|it| match it {
                crate::ir::TypedPrintItem::Expr(e) => expr_uses(e),
                _ => false,
            }),
            S::If { cond, then_body, else_body, .. } => {
                expr_uses(cond)
                    || then_body.iter().any(stmt_walk)
                    || else_body.iter().any(stmt_walk)
            }
            S::While { cond, body, .. } => {
                expr_uses(cond) || body.iter().any(stmt_walk)
            }
            S::For { start, end, body, .. } => {
                expr_uses(start) || expr_uses(end) || body.iter().any(stmt_walk)
            }
            S::ForIter { body, .. } => body.iter().any(stmt_walk),
            _ => false,
        }
    }
    for f in &program.functions {
        if f.body.iter().any(stmt_walk) {
            return true;
        }
    }
    false
}

/// Emit the vec_chunks helper. References intent_vec_vec_int64_t,
/// which is only in scope when the program uses Vec<Vec<i64>>.
pub(crate) fn emit_intent_vec_chunks_helper_c(out: &mut String) {
    out.push_str(
        "static INTENT_UNUSED intent_vec_vec_int64_t intent_vec_vec_int64_t_chunks(const intent_vec_int64_t* xs, int64_t k) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_vec_int64_t intent_vec_vec_int64_t_chunks(const intent_vec_int64_t* xs, int64_t k) {\n\
         \x20 intent_vec_vec_int64_t r; r.data = (intent_vec_int64_t*)0; r.len = 0; r.capacity = 0;\n\
         \x20 if (!xs || xs->len == 0 || k <= 0) return r;\n\
         \x20 uint64_t uk = (uint64_t)k;\n\
         \x20 uint64_t num_chunks = (xs->len + uk - 1) / uk;\n\
         \x20 r.capacity = num_chunks;\n\
         \x20 r.data = (intent_vec_int64_t*)malloc(num_chunks * sizeof(intent_vec_int64_t));\n\
         \x20 if (!r.data) abort();\n\
         \x20 for (uint64_t i = 0; i < num_chunks; i++) {\n\
         \x20   uint64_t start = i * uk;\n\
         \x20   uint64_t end = start + uk;\n\
         \x20   if (end > xs->len) end = xs->len;\n\
         \x20   uint64_t clen = end - start;\n\
         \x20   intent_vec_int64_t inner;\n\
         \x20   inner.data = (int64_t*)malloc(clen * sizeof(int64_t));\n\
         \x20   if (!inner.data) abort();\n\
         \x20   for (uint64_t j = 0; j < clen; j++) inner.data[j] = xs->data[start + j];\n\
         \x20   inner.len = clen;\n\
         \x20   inner.capacity = clen;\n\
         \x20   r.data[i] = inner;\n\
         \x20 }\n\
         \x20 r.len = num_chunks;\n\
         \x20 return r;\n\
         }\n\
         /* Closure #594: vec_windows(xs, k) — overlapping length-k windows. Empty if n<k or k<=0. */\n\
         static INTENT_UNUSED intent_vec_vec_int64_t intent_vec_vec_int64_t_windows(const intent_vec_int64_t* xs, int64_t k) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_vec_int64_t intent_vec_vec_int64_t_windows(const intent_vec_int64_t* xs, int64_t k) {\n\
         \x20 intent_vec_vec_int64_t r; r.data = (intent_vec_int64_t*)0; r.len = 0; r.capacity = 0;\n\
         \x20 if (!xs || k <= 0 || (uint64_t)k > xs->len) return r;\n\
         \x20 uint64_t uk = (uint64_t)k;\n\
         \x20 uint64_t num = xs->len - uk + 1;\n\
         \x20 r.capacity = num;\n\
         \x20 r.data = (intent_vec_int64_t*)malloc(num * sizeof(intent_vec_int64_t));\n\
         \x20 if (!r.data) abort();\n\
         \x20 for (uint64_t i = 0; i < num; i++) {\n\
         \x20   intent_vec_int64_t inner;\n\
         \x20   inner.data = (int64_t*)malloc(uk * sizeof(int64_t));\n\
         \x20   if (!inner.data) abort();\n\
         \x20   for (uint64_t j = 0; j < uk; j++) inner.data[j] = xs->data[i + j];\n\
         \x20   inner.len = uk;\n\
         \x20   inner.capacity = uk;\n\
         \x20   r.data[i] = inner;\n\
         \x20 }\n\
         \x20 r.len = num;\n\
         \x20 return r;\n\
         }\n\
         /* Closure #596: vec_group_by_value(xs) — group consecutive equal values. */\n\
         static INTENT_UNUSED intent_vec_vec_int64_t intent_vec_vec_int64_t_group_by_value(const intent_vec_int64_t* xs) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_vec_int64_t intent_vec_vec_int64_t_group_by_value(const intent_vec_int64_t* xs) {\n\
         \x20 intent_vec_vec_int64_t r; r.data = (intent_vec_int64_t*)0; r.len = 0; r.capacity = 0;\n\
         \x20 if (!xs || xs->len == 0) return r;\n\
         \x20 /* Two-pass: count groups, allocate, then fill. */\n\
         \x20 uint64_t ng = 1;\n\
         \x20 for (uint64_t i = 1; i < xs->len; i++) if (xs->data[i] != xs->data[i-1]) ng++;\n\
         \x20 r.capacity = ng;\n\
         \x20 r.data = (intent_vec_int64_t*)malloc(ng * sizeof(intent_vec_int64_t));\n\
         \x20 if (!r.data) abort();\n\
         \x20 uint64_t gi = 0;\n\
         \x20 uint64_t run_start = 0;\n\
         \x20 for (uint64_t i = 1; i <= xs->len; i++) {\n\
         \x20   if (i == xs->len || xs->data[i] != xs->data[i-1]) {\n\
         \x20     uint64_t rlen = i - run_start;\n\
         \x20     intent_vec_int64_t inner;\n\
         \x20     inner.data = (int64_t*)malloc(rlen * sizeof(int64_t));\n\
         \x20     if (!inner.data) abort();\n\
         \x20     for (uint64_t j = 0; j < rlen; j++) inner.data[j] = xs->data[run_start + j];\n\
         \x20     inner.len = rlen;\n\
         \x20     inner.capacity = rlen;\n\
         \x20     r.data[gi++] = inner;\n\
         \x20     run_start = i;\n\
         \x20   }\n\
         \x20 }\n\
         \x20 r.len = ng;\n\
         \x20 return r;\n\
         }\n\
         /* Closure #595: vec_flatten(xss) — concatenate inner Vecs. */\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_flatten(const intent_vec_vec_int64_t* xss) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_flatten(const intent_vec_vec_int64_t* xss) {\n\
         \x20 intent_vec_int64_t r; r.data = (int64_t*)0; r.len = 0; r.capacity = 0;\n\
         \x20 if (!xss || xss->len == 0) return r;\n\
         \x20 uint64_t total = 0;\n\
         \x20 for (uint64_t i = 0; i < xss->len; i++) total += xss->data[i].len;\n\
         \x20 if (total == 0) return r;\n\
         \x20 r.capacity = total;\n\
         \x20 r.data = (int64_t*)malloc(total * sizeof(int64_t));\n\
         \x20 if (!r.data) abort();\n\
         \x20 uint64_t k = 0;\n\
         \x20 for (uint64_t i = 0; i < xss->len; i++) {\n\
         \x20   const intent_vec_int64_t* inner = &xss->data[i];\n\
         \x20   for (uint64_t j = 0; j < inner->len; j++) r.data[k++] = inner->data[j];\n\
         \x20 }\n\
         \x20 r.len = total;\n\
         \x20 return r;\n\
         }\n\n"
    );
}

pub(crate) fn emit_intent_vec_int64_utility_helpers_c(out: &mut String) {
    out.push_str(
        "static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_range(int64_t lo, int64_t hi) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_range(int64_t lo, int64_t hi) {\n\
         \x20 intent_vec_int64_t v; v.data = (int64_t*)0; v.len = 0; v.capacity = 0;\n\
         \x20 if (hi <= lo) return v;\n\
         \x20 uint64_t n = (uint64_t)(hi - lo);\n\
         \x20 v.data = (int64_t*)malloc((size_t)n * sizeof(int64_t));\n\
         \x20 if (!v.data) abort();\n\
         \x20 for (uint64_t i = 0; i < n; i++) v.data[i] = lo + (int64_t)i;\n\
         \x20 v.len = n;\n\
         \x20 v.capacity = n;\n\
         \x20 return v;\n\
         }\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_repeat(int64_t val, int64_t n) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_repeat(int64_t val, int64_t n) {\n\
         \x20 intent_vec_int64_t v; v.data = (int64_t*)0; v.len = 0; v.capacity = 0;\n\
         \x20 if (n <= 0) return v;\n\
         \x20 uint64_t un = (uint64_t)n;\n\
         \x20 v.data = (int64_t*)malloc((size_t)un * sizeof(int64_t));\n\
         \x20 if (!v.data) abort();\n\
         \x20 for (uint64_t i = 0; i < un; i++) v.data[i] = val;\n\
         \x20 v.len = un;\n\
         \x20 v.capacity = un;\n\
         \x20 return v;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_vec_int64_t_extend(intent_vec_int64_t* xs, const intent_vec_int64_t* ys) INTENT_UNUSED;\n\
         static INTENT_UNUSED int64_t intent_vec_int64_t_extend(intent_vec_int64_t* xs, const intent_vec_int64_t* ys) {\n\
         \x20 if (ys->len == 0) return (int64_t)xs->len;\n\
         \x20 uint64_t need = xs->len + ys->len;\n\
         \x20 if (need > xs->capacity) {\n\
         \x20   uint64_t new_cap = xs->capacity ? xs->capacity : 4;\n\
         \x20   while (new_cap < need) new_cap *= 2;\n\
         \x20   xs->data = (int64_t*)realloc(xs->data, (size_t)new_cap * sizeof(int64_t));\n\
         \x20   if (!xs->data) abort();\n\
         \x20   xs->capacity = new_cap;\n\
         \x20 }\n\
         \x20 memcpy(xs->data + xs->len, ys->data, (size_t)ys->len * sizeof(int64_t));\n\
         \x20 xs->len = need;\n\
         \x20 return (int64_t)xs->len;\n\
         }\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_concat(const intent_vec_int64_t* xs, const intent_vec_int64_t* ys) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_concat(const intent_vec_int64_t* xs, const intent_vec_int64_t* ys) {\n\
         \x20 intent_vec_int64_t v; v.data = (int64_t*)0; v.len = 0; v.capacity = 0;\n\
         \x20 uint64_t total = xs->len + ys->len;\n\
         \x20 if (total == 0) return v;\n\
         \x20 v.data = (int64_t*)malloc((size_t)total * sizeof(int64_t));\n\
         \x20 if (!v.data) abort();\n\
         \x20 if (xs->len > 0) memcpy(v.data, xs->data, (size_t)xs->len * sizeof(int64_t));\n\
         \x20 if (ys->len > 0) memcpy(v.data + xs->len, ys->data, (size_t)ys->len * sizeof(int64_t));\n\
         \x20 v.len = total;\n\
         \x20 v.capacity = total;\n\
         \x20 return v;\n\
         }\n\
         /* Closure #407: set ops on Vec<i64>. O(n*m) — fine for\n\
          * the v1 audience. Each returns a fresh deduplicated\n\
          * Vec<i64>. */\n\
         static INTENT_UNUSED int intent_vec_int64_t_contains_value(const intent_vec_int64_t* xs, int64_t v) INTENT_UNUSED;\n\
         static INTENT_UNUSED int intent_vec_int64_t_contains_value(const intent_vec_int64_t* xs, int64_t v) {\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) if (xs->data[i] == v) return 1;\n\
         \x20 return 0;\n\
         }\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_intersect(const intent_vec_int64_t* xs, const intent_vec_int64_t* ys) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_intersect(const intent_vec_int64_t* xs, const intent_vec_int64_t* ys) {\n\
         \x20 intent_vec_int64_t out;\n\
         \x20 out.data = (int64_t*)0; out.len = 0; out.capacity = 0;\n\
         \x20 if (xs->len == 0 || ys->len == 0) return out;\n\
         \x20 out.capacity = xs->len < ys->len ? xs->len : ys->len;\n\
         \x20 out.data = (int64_t*)malloc(out.capacity * sizeof(int64_t));\n\
         \x20 if (!out.data) abort();\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) {\n\
         \x20   int64_t v = xs->data[i];\n\
         \x20   if (intent_vec_int64_t_contains_value(ys, v) && !intent_vec_int64_t_contains_value(&out, v)) out.data[out.len++] = v;\n\
         \x20 }\n\
         \x20 return out;\n\
         }\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_difference(const intent_vec_int64_t* xs, const intent_vec_int64_t* ys) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_difference(const intent_vec_int64_t* xs, const intent_vec_int64_t* ys) {\n\
         \x20 intent_vec_int64_t out;\n\
         \x20 out.data = (int64_t*)0; out.len = 0; out.capacity = 0;\n\
         \x20 if (xs->len == 0) return out;\n\
         \x20 out.capacity = xs->len;\n\
         \x20 out.data = (int64_t*)malloc(out.capacity * sizeof(int64_t));\n\
         \x20 if (!out.data) abort();\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) {\n\
         \x20   int64_t v = xs->data[i];\n\
         \x20   if (!intent_vec_int64_t_contains_value(ys, v) && !intent_vec_int64_t_contains_value(&out, v)) out.data[out.len++] = v;\n\
         \x20 }\n\
         \x20 return out;\n\
         }\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_union(const intent_vec_int64_t* xs, const intent_vec_int64_t* ys) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_union(const intent_vec_int64_t* xs, const intent_vec_int64_t* ys) {\n\
         \x20 intent_vec_int64_t out;\n\
         \x20 out.data = (int64_t*)0; out.len = 0; out.capacity = 0;\n\
         \x20 uint64_t total = xs->len + ys->len;\n\
         \x20 if (total == 0) return out;\n\
         \x20 out.capacity = total;\n\
         \x20 out.data = (int64_t*)malloc(out.capacity * sizeof(int64_t));\n\
         \x20 if (!out.data) abort();\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) {\n\
         \x20   int64_t v = xs->data[i];\n\
         \x20   if (!intent_vec_int64_t_contains_value(&out, v)) out.data[out.len++] = v;\n\
         \x20 }\n\
         \x20 for (uint64_t i = 0; i < ys->len; i++) {\n\
         \x20   int64_t v = ys->data[i];\n\
         \x20   if (!intent_vec_int64_t_contains_value(&out, v)) out.data[out.len++] = v;\n\
         \x20 }\n\
         \x20 return out;\n\
         }\n\
         /* Closure #399: vec_dot(ref xs, ref ys) -> i64.\n\
          * Dot product (sum of xs[i] * ys[i]); truncates to the\n\
          * shorter Vec. */\n\
         static INTENT_UNUSED int64_t intent_vec_int64_t_dot(const intent_vec_int64_t* xs, const intent_vec_int64_t* ys) INTENT_UNUSED;\n\
         static INTENT_UNUSED int64_t intent_vec_int64_t_dot(const intent_vec_int64_t* xs, const intent_vec_int64_t* ys) {\n\
         \x20 uint64_t n = xs->len < ys->len ? xs->len : ys->len;\n\
         \x20 int64_t acc = 0;\n\
         \x20 for (uint64_t i = 0; i < n; i++) acc = acc + xs->data[i] * ys->data[i];\n\
         \x20 return acc;\n\
         }\n\
         /* Closure #398: vec_running_sum(ref xs) -> Vec<i64>.\n\
          * Cumulative sum: result[i] = sum(xs[0..=i]). */\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_running_sum(const intent_vec_int64_t* xs) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_running_sum(const intent_vec_int64_t* xs) {\n\
         \x20 intent_vec_int64_t v; v.data = (int64_t*)0; v.len = 0; v.capacity = 0;\n\
         \x20 if (!xs || xs->len == 0) return v;\n\
         \x20 v.capacity = xs->len;\n\
         \x20 v.data = (int64_t*)malloc(v.capacity * sizeof(int64_t));\n\
         \x20 if (!v.data) abort();\n\
         \x20 int64_t acc = 0;\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) {\n\
         \x20   acc = acc + xs->data[i];\n\
         \x20   v.data[i] = acc;\n\
         \x20 }\n\
         \x20 v.len = xs->len;\n\
         \x20 return v;\n\
         }\n\
         /* Closures #512-#515: vec_running_product / vec_running_xor / vec_running_and / vec_running_or.\n\
          * Same shape as vec_running_sum (#398) with different monoid:\n\
          *   product (identity 1, op mul)\n\
          *   xor     (identity 0, op xor)\n\
          *   and     (identity -1 / all-ones, op bitand)\n\
          *   or      (identity 0, op bitor) */\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_running_product(const intent_vec_int64_t* xs) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_running_product(const intent_vec_int64_t* xs) {\n\
         \x20 intent_vec_int64_t v; v.data = (int64_t*)0; v.len = 0; v.capacity = 0;\n\
         \x20 if (!xs || xs->len == 0) return v;\n\
         \x20 v.capacity = xs->len;\n\
         \x20 v.data = (int64_t*)malloc(v.capacity * sizeof(int64_t));\n\
         \x20 if (!v.data) abort();\n\
         \x20 int64_t acc = 1;\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) {\n\
         \x20   acc = acc * xs->data[i];\n\
         \x20   v.data[i] = acc;\n\
         \x20 }\n\
         \x20 v.len = xs->len;\n\
         \x20 return v;\n\
         }\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_running_xor(const intent_vec_int64_t* xs) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_running_xor(const intent_vec_int64_t* xs) {\n\
         \x20 intent_vec_int64_t v; v.data = (int64_t*)0; v.len = 0; v.capacity = 0;\n\
         \x20 if (!xs || xs->len == 0) return v;\n\
         \x20 v.capacity = xs->len;\n\
         \x20 v.data = (int64_t*)malloc(v.capacity * sizeof(int64_t));\n\
         \x20 if (!v.data) abort();\n\
         \x20 int64_t acc = 0;\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) {\n\
         \x20   acc = acc ^ xs->data[i];\n\
         \x20   v.data[i] = acc;\n\
         \x20 }\n\
         \x20 v.len = xs->len;\n\
         \x20 return v;\n\
         }\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_running_and(const intent_vec_int64_t* xs) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_running_and(const intent_vec_int64_t* xs) {\n\
         \x20 intent_vec_int64_t v; v.data = (int64_t*)0; v.len = 0; v.capacity = 0;\n\
         \x20 if (!xs || xs->len == 0) return v;\n\
         \x20 v.capacity = xs->len;\n\
         \x20 v.data = (int64_t*)malloc(v.capacity * sizeof(int64_t));\n\
         \x20 if (!v.data) abort();\n\
         \x20 int64_t acc = (int64_t)-1;\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) {\n\
         \x20   acc = acc & xs->data[i];\n\
         \x20   v.data[i] = acc;\n\
         \x20 }\n\
         \x20 v.len = xs->len;\n\
         \x20 return v;\n\
         }\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_running_or(const intent_vec_int64_t* xs) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_running_or(const intent_vec_int64_t* xs) {\n\
         \x20 intent_vec_int64_t v; v.data = (int64_t*)0; v.len = 0; v.capacity = 0;\n\
         \x20 if (!xs || xs->len == 0) return v;\n\
         \x20 v.capacity = xs->len;\n\
         \x20 v.data = (int64_t*)malloc(v.capacity * sizeof(int64_t));\n\
         \x20 if (!v.data) abort();\n\
         \x20 int64_t acc = 0;\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) {\n\
         \x20   acc = acc | xs->data[i];\n\
         \x20   v.data[i] = acc;\n\
         \x20 }\n\
         \x20 v.len = xs->len;\n\
         \x20 return v;\n\
         }\n\
         /* Closures #569-#572: sort-free analytics on Vec<i64>.\n\
          *   range_span(xs): max - min, single pass. Empty → 0.\n\
          *   mode(xs): most-common value; ties → smallest. O(n²). Empty → 0.\n\
          *   kth_smallest(xs, k): the k-th smallest (0-indexed) via\n\
          *     count-based O(n²). Returns -1 if k out of bounds.\n\
          *   median(xs): lower median via kth_smallest((n-1)/2). Empty → 0. */\n\
         static INTENT_UNUSED int64_t intent_vec_int64_t_range_span(const intent_vec_int64_t* xs) INTENT_UNUSED;\n\
         static INTENT_UNUSED int64_t intent_vec_int64_t_range_span(const intent_vec_int64_t* xs) {\n\
         \x20 if (!xs || xs->len == 0) return 0;\n\
         \x20 int64_t mn = xs->data[0], mx = xs->data[0];\n\
         \x20 for (uint64_t i = 1; i < xs->len; i++) {\n\
         \x20   int64_t v = xs->data[i];\n\
         \x20   if (v < mn) mn = v;\n\
         \x20   if (v > mx) mx = v;\n\
         \x20 }\n\
         \x20 return mx - mn;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_vec_int64_t_mode(const intent_vec_int64_t* xs) INTENT_UNUSED;\n\
         static INTENT_UNUSED int64_t intent_vec_int64_t_mode(const intent_vec_int64_t* xs) {\n\
         \x20 if (!xs || xs->len == 0) return 0;\n\
         \x20 int64_t best_v = xs->data[0];\n\
         \x20 int64_t best_c = 1;\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) {\n\
         \x20   int64_t v = xs->data[i];\n\
         \x20   int64_t c = 0;\n\
         \x20   for (uint64_t j = 0; j < xs->len; j++) if (xs->data[j] == v) c++;\n\
         \x20   if (c > best_c || (c == best_c && v < best_v)) {\n\
         \x20     best_v = v;\n\
         \x20     best_c = c;\n\
         \x20   }\n\
         \x20 }\n\
         \x20 return best_v;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_vec_int64_t_kth_smallest(const intent_vec_int64_t* xs, int64_t k) INTENT_UNUSED;\n\
         static INTENT_UNUSED int64_t intent_vec_int64_t_kth_smallest(const intent_vec_int64_t* xs, int64_t k) {\n\
         \x20 if (!xs || xs->len == 0) return -1;\n\
         \x20 if (k < 0 || k >= (int64_t)xs->len) return -1;\n\
         \x20 /* For each candidate x in xs, fewer = #{xs[j] < x}, nm = #{xs[j] <= x}.\n\
         \x20  * The k-th smallest is the value where fewer <= k < nm. */\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) {\n\
         \x20   int64_t x = xs->data[i];\n\
         \x20   int64_t fewer = 0, nm = 0;\n\
         \x20   for (uint64_t j = 0; j < xs->len; j++) {\n\
         \x20     int64_t y = xs->data[j];\n\
         \x20     if (y < x) fewer++;\n\
         \x20     if (y <= x) nm++;\n\
         \x20   }\n\
         \x20   if (fewer <= k && k < nm) return x;\n\
         \x20 }\n\
         \x20 return -1; /* unreachable for non-empty xs */\n\
         }\n\
         static INTENT_UNUSED int64_t intent_vec_int64_t_median(const intent_vec_int64_t* xs) INTENT_UNUSED;\n\
         static INTENT_UNUSED int64_t intent_vec_int64_t_median(const intent_vec_int64_t* xs) {\n\
         \x20 if (!xs || xs->len == 0) return 0;\n\
         \x20 int64_t k = (int64_t)((xs->len - 1) / 2);\n\
         \x20 return intent_vec_int64_t_kth_smallest(xs, k);\n\
         }\n\
         /* Closure #603: vec_running_mean — running integer average per index. */\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_running_mean(const intent_vec_int64_t* xs) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_running_mean(const intent_vec_int64_t* xs) {\n\
         \x20 intent_vec_int64_t r; r.data = (int64_t*)0; r.len = 0; r.capacity = 0;\n\
         \x20 if (!xs || xs->len == 0) return r;\n\
         \x20 r.capacity = xs->len;\n\
         \x20 r.data = (int64_t*)malloc(xs->len * sizeof(int64_t));\n\
         \x20 if (!r.data) abort();\n\
         \x20 int64_t acc = 0;\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) {\n\
         \x20   acc += xs->data[i];\n\
         \x20   r.data[i] = acc / (int64_t)(i + 1);\n\
         \x20 }\n\
         \x20 r.len = xs->len;\n\
         \x20 return r;\n\
         }\n\
         /* Closure #604: vec_intersperse — insert sep between elements. */\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_intersperse(const intent_vec_int64_t* xs, int64_t sep) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_intersperse(const intent_vec_int64_t* xs, int64_t sep) {\n\
         \x20 intent_vec_int64_t r; r.data = (int64_t*)0; r.len = 0; r.capacity = 0;\n\
         \x20 if (!xs || xs->len == 0) return r;\n\
         \x20 uint64_t out_len = xs->len * 2 - 1;\n\
         \x20 r.capacity = out_len;\n\
         \x20 r.data = (int64_t*)malloc(out_len * sizeof(int64_t));\n\
         \x20 if (!r.data) abort();\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) {\n\
         \x20   r.data[i * 2] = xs->data[i];\n\
         \x20   if (i + 1 < xs->len) r.data[i * 2 + 1] = sep;\n\
         \x20 }\n\
         \x20 r.len = out_len;\n\
         \x20 return r;\n\
         }\n\
         /* Closures #566-#568: sort/merge helpers on Vec<i64>.\n\
          *   merge_sorted(xs, ys): O(n+m) two-pointer merge of two pre-sorted Vecs.\n\
          *     If inputs aren't sorted ascending, the output won't be either —\n\
          *     caller's responsibility.\n\
          *   insert_sorted(xs, v): O(n) — find first index i where xs[i] >= v,\n\
          *     return fresh Vec with v inserted at that index. Length n+1.\n\
          *   is_sorted_unique(xs): true iff strictly ascending (no equal adjacent\n\
          *     values). Empty/1-elt → true (vacuous). */\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_merge_sorted(const intent_vec_int64_t* xs, const intent_vec_int64_t* ys) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_merge_sorted(const intent_vec_int64_t* xs, const intent_vec_int64_t* ys) {\n\
         \x20 intent_vec_int64_t r; r.data = (int64_t*)0; r.len = 0; r.capacity = 0;\n\
         \x20 uint64_t xn = xs ? xs->len : 0;\n\
         \x20 uint64_t yn = ys ? ys->len : 0;\n\
         \x20 uint64_t total = xn + yn;\n\
         \x20 if (total == 0) return r;\n\
         \x20 r.capacity = total;\n\
         \x20 r.data = (int64_t*)malloc(total * sizeof(int64_t));\n\
         \x20 if (!r.data) abort();\n\
         \x20 uint64_t i = 0, j = 0, k = 0;\n\
         \x20 while (i < xn && j < yn) {\n\
         \x20   if (xs->data[i] <= ys->data[j]) r.data[k++] = xs->data[i++];\n\
         \x20   else r.data[k++] = ys->data[j++];\n\
         \x20 }\n\
         \x20 while (i < xn) r.data[k++] = xs->data[i++];\n\
         \x20 while (j < yn) r.data[k++] = ys->data[j++];\n\
         \x20 r.len = total;\n\
         \x20 return r;\n\
         }\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_insert_sorted(const intent_vec_int64_t* xs, int64_t v) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_insert_sorted(const intent_vec_int64_t* xs, int64_t v) {\n\
         \x20 intent_vec_int64_t r; r.data = (int64_t*)0; r.len = 0; r.capacity = 0;\n\
         \x20 uint64_t n = xs ? xs->len : 0;\n\
         \x20 uint64_t out_len = n + 1;\n\
         \x20 r.capacity = out_len;\n\
         \x20 r.data = (int64_t*)malloc(out_len * sizeof(int64_t));\n\
         \x20 if (!r.data) abort();\n\
         \x20 uint64_t insert_at = n;\n\
         \x20 for (uint64_t i = 0; i < n; i++) if (xs->data[i] >= v) { insert_at = i; break; }\n\
         \x20 for (uint64_t i = 0; i < insert_at; i++) r.data[i] = xs->data[i];\n\
         \x20 r.data[insert_at] = v;\n\
         \x20 for (uint64_t i = insert_at; i < n; i++) r.data[i + 1] = xs->data[i];\n\
         \x20 r.len = out_len;\n\
         \x20 return r;\n\
         }\n\
         static INTENT_UNUSED bool intent_vec_int64_t_is_sorted_unique(const intent_vec_int64_t* xs) INTENT_UNUSED;\n\
         static INTENT_UNUSED bool intent_vec_int64_t_is_sorted_unique(const intent_vec_int64_t* xs) {\n\
         \x20 if (!xs || xs->len < 2) return true;\n\
         \x20 for (uint64_t i = 1; i < xs->len; i++) if (xs->data[i] <= xs->data[i - 1]) return false;\n\
         \x20 return true;\n\
         }\n\
         /* Closures #562-#565: analytics + dedup_consecutive.\n\
          *   count_distinct(xs) -> i64: O(n^2) count of unique values\n\
          *   indices_of_value(xs, v) -> Vec<i64>: all indices where xs[i] == v\n\
          *   dedup_consecutive(xs) -> Vec<i64>: remove only adjacent duplicates\n\
          *   mean(xs) -> i64: integer mean (sum/len); empty Vec → 0 */\n\
         static INTENT_UNUSED int64_t intent_vec_int64_t_count_distinct(const intent_vec_int64_t* xs) INTENT_UNUSED;\n\
         static INTENT_UNUSED int64_t intent_vec_int64_t_count_distinct(const intent_vec_int64_t* xs) {\n\
         \x20 if (!xs || xs->len == 0) return 0;\n\
         \x20 int64_t c = 0;\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) {\n\
         \x20   bool first = true;\n\
         \x20   for (uint64_t j = 0; j < i; j++) if (xs->data[j] == xs->data[i]) { first = false; break; }\n\
         \x20   if (first) c++;\n\
         \x20 }\n\
         \x20 return c;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_vec_int64_t_mean(const intent_vec_int64_t* xs) INTENT_UNUSED;\n\
         static INTENT_UNUSED int64_t intent_vec_int64_t_mean(const intent_vec_int64_t* xs) {\n\
         \x20 if (!xs || xs->len == 0) return 0;\n\
         \x20 int64_t acc = 0;\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) acc += xs->data[i];\n\
         \x20 return acc / (int64_t)xs->len;\n\
         }\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_indices_of_value(const intent_vec_int64_t* xs, int64_t v) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_indices_of_value(const intent_vec_int64_t* xs, int64_t v) {\n\
         \x20 intent_vec_int64_t r; r.data = (int64_t*)0; r.len = 0; r.capacity = 0;\n\
         \x20 if (!xs || xs->len == 0) return r;\n\
         \x20 /* Count first, then allocate exactly. */\n\
         \x20 uint64_t c = 0;\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) if (xs->data[i] == v) c++;\n\
         \x20 if (c == 0) return r;\n\
         \x20 r.capacity = c;\n\
         \x20 r.data = (int64_t*)malloc(c * sizeof(int64_t));\n\
         \x20 if (!r.data) abort();\n\
         \x20 uint64_t k = 0;\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) if (xs->data[i] == v) r.data[k++] = (int64_t)i;\n\
         \x20 r.len = c;\n\
         \x20 return r;\n\
         }\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_dedup_consecutive(const intent_vec_int64_t* xs) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_dedup_consecutive(const intent_vec_int64_t* xs) {\n\
         \x20 intent_vec_int64_t r; r.data = (int64_t*)0; r.len = 0; r.capacity = 0;\n\
         \x20 if (!xs || xs->len == 0) return r;\n\
         \x20 r.capacity = xs->len;\n\
         \x20 r.data = (int64_t*)malloc(xs->len * sizeof(int64_t));\n\
         \x20 if (!r.data) abort();\n\
         \x20 r.data[0] = xs->data[0];\n\
         \x20 uint64_t out = 1;\n\
         \x20 for (uint64_t i = 1; i < xs->len; i++) if (xs->data[i] != xs->data[i - 1]) r.data[out++] = xs->data[i];\n\
         \x20 r.len = out;\n\
         \x20 return r;\n\
         }\n\
         /* Closure #558: vec_diff(xs) -> Vec<i64> of first differences.\n\
          *   result[i] = xs[i+1] - xs[i], length = max(0, n-1). */\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_diff(const intent_vec_int64_t* xs) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_diff(const intent_vec_int64_t* xs) {\n\
         \x20 intent_vec_int64_t r; r.data = (int64_t*)0; r.len = 0; r.capacity = 0;\n\
         \x20 if (!xs || xs->len < 2) return r;\n\
         \x20 uint64_t out_len = xs->len - 1;\n\
         \x20 r.capacity = out_len;\n\
         \x20 r.data = (int64_t*)malloc(out_len * sizeof(int64_t));\n\
         \x20 if (!r.data) abort();\n\
         \x20 for (uint64_t i = 0; i < out_len; i++) r.data[i] = xs->data[i + 1] - xs->data[i];\n\
         \x20 r.len = out_len;\n\
         \x20 return r;\n\
         }\n\
         /* Closures #559/#560: vec_pad_left / vec_pad_right(xs, target_len, fill).\n\
          *   pad_left: if target_len > n, prepend (target_len - n) fill values;\n\
          *             otherwise return a copy of xs (no truncation).\n\
          *   pad_right: if target_len > n, append (target_len - n) fill values;\n\
          *              otherwise return a copy. target_len < 0 → empty Vec. */\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_pad_left(const intent_vec_int64_t* xs, int64_t target_len, int64_t fill) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_pad_left(const intent_vec_int64_t* xs, int64_t target_len, int64_t fill) {\n\
         \x20 intent_vec_int64_t r; r.data = (int64_t*)0; r.len = 0; r.capacity = 0;\n\
         \x20 if (target_len < 0) return r;\n\
         \x20 if (!xs) {\n\
         \x20   if (target_len == 0) return r;\n\
         \x20   r.capacity = (uint64_t)target_len;\n\
         \x20   r.data = (int64_t*)malloc(r.capacity * sizeof(int64_t));\n\
         \x20   if (!r.data) abort();\n\
         \x20   for (uint64_t i = 0; i < r.capacity; i++) r.data[i] = fill;\n\
         \x20   r.len = r.capacity;\n\
         \x20   return r;\n\
         \x20 }\n\
         \x20 uint64_t want = (uint64_t)target_len;\n\
         \x20 uint64_t out_len = want > xs->len ? want : xs->len;\n\
         \x20 if (out_len == 0) return r;\n\
         \x20 r.capacity = out_len;\n\
         \x20 r.data = (int64_t*)malloc(out_len * sizeof(int64_t));\n\
         \x20 if (!r.data) abort();\n\
         \x20 uint64_t pad = out_len - xs->len;\n\
         \x20 for (uint64_t i = 0; i < pad; i++) r.data[i] = fill;\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) r.data[pad + i] = xs->data[i];\n\
         \x20 r.len = out_len;\n\
         \x20 return r;\n\
         }\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_pad_right(const intent_vec_int64_t* xs, int64_t target_len, int64_t fill) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_pad_right(const intent_vec_int64_t* xs, int64_t target_len, int64_t fill) {\n\
         \x20 intent_vec_int64_t r; r.data = (int64_t*)0; r.len = 0; r.capacity = 0;\n\
         \x20 if (target_len < 0) return r;\n\
         \x20 if (!xs) {\n\
         \x20   if (target_len == 0) return r;\n\
         \x20   r.capacity = (uint64_t)target_len;\n\
         \x20   r.data = (int64_t*)malloc(r.capacity * sizeof(int64_t));\n\
         \x20   if (!r.data) abort();\n\
         \x20   for (uint64_t i = 0; i < r.capacity; i++) r.data[i] = fill;\n\
         \x20   r.len = r.capacity;\n\
         \x20   return r;\n\
         \x20 }\n\
         \x20 uint64_t want = (uint64_t)target_len;\n\
         \x20 uint64_t out_len = want > xs->len ? want : xs->len;\n\
         \x20 if (out_len == 0) return r;\n\
         \x20 r.capacity = out_len;\n\
         \x20 r.data = (int64_t*)malloc(out_len * sizeof(int64_t));\n\
         \x20 if (!r.data) abort();\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) r.data[i] = xs->data[i];\n\
         \x20 for (uint64_t i = xs->len; i < out_len; i++) r.data[i] = fill;\n\
         \x20 r.len = out_len;\n\
         \x20 return r;\n\
         }\n\
         /* Closure #561: vec_replace_value(xs, old, new) — return fresh Vec\n\
          * with every occurrence of `old` replaced by `new`. Length unchanged. */\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_replace_value(const intent_vec_int64_t* xs, int64_t old_v, int64_t new_v) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_replace_value(const intent_vec_int64_t* xs, int64_t old_v, int64_t new_v) {\n\
         \x20 intent_vec_int64_t r; r.data = (int64_t*)0; r.len = 0; r.capacity = 0;\n\
         \x20 if (!xs || xs->len == 0) return r;\n\
         \x20 r.capacity = xs->len;\n\
         \x20 r.data = (int64_t*)malloc(r.capacity * sizeof(int64_t));\n\
         \x20 if (!r.data) abort();\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) r.data[i] = (xs->data[i] == old_v) ? new_v : xs->data[i];\n\
         \x20 r.len = xs->len;\n\
         \x20 return r;\n\
         }\n\
         /* Closures #554-#557: dual-Vec<i64> bool predicates.\n\
          *   subset_of(xs, ys): every elt of xs appears somewhere in ys\n\
          *   disjoint(xs, ys): no elt appears in both\n\
          *   equal_set(xs, ys): xs and ys have the same set of elts (mult ignored)\n\
          *     i.e. subset(xs, ys) and subset(ys, xs)\n\
          *   equal_seq(xs, ys): same length, xs[i] == ys[i] for all i */\n\
         static INTENT_UNUSED bool intent_vec_int64_t_subset_of(const intent_vec_int64_t* xs, const intent_vec_int64_t* ys) INTENT_UNUSED;\n\
         static INTENT_UNUSED bool intent_vec_int64_t_subset_of(const intent_vec_int64_t* xs, const intent_vec_int64_t* ys) {\n\
         \x20 if (!xs || xs->len == 0) return true;\n\
         \x20 if (!ys) return false;\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) {\n\
         \x20   bool found = false;\n\
         \x20   for (uint64_t j = 0; j < ys->len; j++) if (ys->data[j] == xs->data[i]) { found = true; break; }\n\
         \x20   if (!found) return false;\n\
         \x20 }\n\
         \x20 return true;\n\
         }\n\
         static INTENT_UNUSED bool intent_vec_int64_t_disjoint(const intent_vec_int64_t* xs, const intent_vec_int64_t* ys) INTENT_UNUSED;\n\
         static INTENT_UNUSED bool intent_vec_int64_t_disjoint(const intent_vec_int64_t* xs, const intent_vec_int64_t* ys) {\n\
         \x20 if (!xs || !ys || xs->len == 0 || ys->len == 0) return true;\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) {\n\
         \x20   for (uint64_t j = 0; j < ys->len; j++) if (ys->data[j] == xs->data[i]) return false;\n\
         \x20 }\n\
         \x20 return true;\n\
         }\n\
         static INTENT_UNUSED bool intent_vec_int64_t_equal_set(const intent_vec_int64_t* xs, const intent_vec_int64_t* ys) INTENT_UNUSED;\n\
         static INTENT_UNUSED bool intent_vec_int64_t_equal_set(const intent_vec_int64_t* xs, const intent_vec_int64_t* ys) {\n\
         \x20 return intent_vec_int64_t_subset_of(xs, ys) && intent_vec_int64_t_subset_of(ys, xs);\n\
         }\n\
         static INTENT_UNUSED bool intent_vec_int64_t_equal_seq(const intent_vec_int64_t* xs, const intent_vec_int64_t* ys) INTENT_UNUSED;\n\
         static INTENT_UNUSED bool intent_vec_int64_t_equal_seq(const intent_vec_int64_t* xs, const intent_vec_int64_t* ys) {\n\
         \x20 if (!xs || !ys) return false;\n\
         \x20 if (xs->len != ys->len) return false;\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) if (xs->data[i] != ys->data[i]) return false;\n\
         \x20 return true;\n\
         }\n\
         /* Closures #550-#553: positional rotations and shifts on Vec<i64>.\n\
          *   rotate_left(xs, k): cyclic shift left by k; result[i] = xs[(i+k) mod n]\n\
          *   rotate_right(xs, k): cyclic shift right by k; result[i] = xs[(i-k) mod n]\n\
          *   shift_left(xs, k): non-cyclic; first n-k elements then k zeros; k >= n → all zeros\n\
          *   shift_right(xs, k): non-cyclic; k zeros then first n-k elements; k >= n → all zeros\n\
          *   k < 0 → empty Vec for all four. */\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_rotate_left(const intent_vec_int64_t* xs, int64_t k) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_rotate_left(const intent_vec_int64_t* xs, int64_t k) {\n\
         \x20 intent_vec_int64_t r; r.data = (int64_t*)0; r.len = 0; r.capacity = 0;\n\
         \x20 if (!xs || xs->len == 0 || k < 0) return r;\n\
         \x20 r.capacity = xs->len;\n\
         \x20 r.data = (int64_t*)malloc(r.capacity * sizeof(int64_t));\n\
         \x20 if (!r.data) abort();\n\
         \x20 uint64_t shift = (uint64_t)k % xs->len;\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) r.data[i] = xs->data[(i + shift) % xs->len];\n\
         \x20 r.len = xs->len;\n\
         \x20 return r;\n\
         }\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_rotate_right(const intent_vec_int64_t* xs, int64_t k) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_rotate_right(const intent_vec_int64_t* xs, int64_t k) {\n\
         \x20 intent_vec_int64_t r; r.data = (int64_t*)0; r.len = 0; r.capacity = 0;\n\
         \x20 if (!xs || xs->len == 0 || k < 0) return r;\n\
         \x20 r.capacity = xs->len;\n\
         \x20 r.data = (int64_t*)malloc(r.capacity * sizeof(int64_t));\n\
         \x20 if (!r.data) abort();\n\
         \x20 uint64_t shift = (uint64_t)k % xs->len;\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) {\n\
         \x20   uint64_t src = (i + xs->len - shift) % xs->len;\n\
         \x20   r.data[i] = xs->data[src];\n\
         \x20 }\n\
         \x20 r.len = xs->len;\n\
         \x20 return r;\n\
         }\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_shift_left(const intent_vec_int64_t* xs, int64_t k) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_shift_left(const intent_vec_int64_t* xs, int64_t k) {\n\
         \x20 intent_vec_int64_t r; r.data = (int64_t*)0; r.len = 0; r.capacity = 0;\n\
         \x20 if (!xs || xs->len == 0 || k < 0) return r;\n\
         \x20 r.capacity = xs->len;\n\
         \x20 r.data = (int64_t*)malloc(r.capacity * sizeof(int64_t));\n\
         \x20 if (!r.data) abort();\n\
         \x20 uint64_t shift = (uint64_t)k > xs->len ? xs->len : (uint64_t)k;\n\
         \x20 uint64_t kept = xs->len - shift;\n\
         \x20 for (uint64_t i = 0; i < kept; i++) r.data[i] = xs->data[i + shift];\n\
         \x20 for (uint64_t i = kept; i < xs->len; i++) r.data[i] = 0;\n\
         \x20 r.len = xs->len;\n\
         \x20 return r;\n\
         }\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_shift_right(const intent_vec_int64_t* xs, int64_t k) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_shift_right(const intent_vec_int64_t* xs, int64_t k) {\n\
         \x20 intent_vec_int64_t r; r.data = (int64_t*)0; r.len = 0; r.capacity = 0;\n\
         \x20 if (!xs || xs->len == 0 || k < 0) return r;\n\
         \x20 r.capacity = xs->len;\n\
         \x20 r.data = (int64_t*)malloc(r.capacity * sizeof(int64_t));\n\
         \x20 if (!r.data) abort();\n\
         \x20 uint64_t shift = (uint64_t)k > xs->len ? xs->len : (uint64_t)k;\n\
         \x20 for (uint64_t i = 0; i < shift; i++) r.data[i] = 0;\n\
         \x20 for (uint64_t i = shift; i < xs->len; i++) r.data[i] = xs->data[i - shift];\n\
         \x20 r.len = xs->len;\n\
         \x20 return r;\n\
         }\n\
         /* Closures #546-#549: modular/bit-shift scalar broadcast on Vec<i64>.\n\
          *   mod_scalar(xs, v): xs[i] mod v  (empty Vec if v == 0)\n\
          *   pow_scalar(xs, k): xs[i] ^^ k via repeated multiply (empty if k < 0)\n\
          *   shl_scalar(xs, k): xs[i] << k  (0..=63 valid; out-of-range → empty)\n\
          *   shr_scalar(xs, k): xs[i] >> k  arithmetic right-shift (0..=63 valid) */\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_mod_scalar(const intent_vec_int64_t* xs, int64_t v) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_mod_scalar(const intent_vec_int64_t* xs, int64_t v) {\n\
         \x20 intent_vec_int64_t r; r.data = (int64_t*)0; r.len = 0; r.capacity = 0;\n\
         \x20 if (!xs || xs->len == 0 || v == 0) return r;\n\
         \x20 r.capacity = xs->len;\n\
         \x20 r.data = (int64_t*)malloc(r.capacity * sizeof(int64_t));\n\
         \x20 if (!r.data) abort();\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) r.data[i] = xs->data[i] % v;\n\
         \x20 r.len = xs->len;\n\
         \x20 return r;\n\
         }\n\
         static INTENT_UNUSED int64_t intent_vec_i64_pow_one(int64_t b, int64_t k) INTENT_UNUSED;\n\
         static INTENT_UNUSED int64_t intent_vec_i64_pow_one(int64_t b, int64_t k) {\n\
         \x20 int64_t r = 1;\n\
         \x20 for (int64_t i = 0; i < k; i++) r = r * b;\n\
         \x20 return r;\n\
         }\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_pow_scalar(const intent_vec_int64_t* xs, int64_t k) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_pow_scalar(const intent_vec_int64_t* xs, int64_t k) {\n\
         \x20 intent_vec_int64_t r; r.data = (int64_t*)0; r.len = 0; r.capacity = 0;\n\
         \x20 if (!xs || xs->len == 0 || k < 0) return r;\n\
         \x20 r.capacity = xs->len;\n\
         \x20 r.data = (int64_t*)malloc(r.capacity * sizeof(int64_t));\n\
         \x20 if (!r.data) abort();\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) r.data[i] = intent_vec_i64_pow_one(xs->data[i], k);\n\
         \x20 r.len = xs->len;\n\
         \x20 return r;\n\
         }\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_shl_scalar(const intent_vec_int64_t* xs, int64_t k) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_shl_scalar(const intent_vec_int64_t* xs, int64_t k) {\n\
         \x20 intent_vec_int64_t r; r.data = (int64_t*)0; r.len = 0; r.capacity = 0;\n\
         \x20 if (!xs || xs->len == 0 || k < 0 || k > 63) return r;\n\
         \x20 r.capacity = xs->len;\n\
         \x20 r.data = (int64_t*)malloc(r.capacity * sizeof(int64_t));\n\
         \x20 if (!r.data) abort();\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) {\n\
         \x20   /* cast to unsigned to make the shift well-defined on negative inputs */\n\
         \x20   uint64_t ux = (uint64_t)xs->data[i];\n\
         \x20   r.data[i] = (int64_t)(ux << k);\n\
         \x20 }\n\
         \x20 r.len = xs->len;\n\
         \x20 return r;\n\
         }\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_shr_scalar(const intent_vec_int64_t* xs, int64_t k) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_shr_scalar(const intent_vec_int64_t* xs, int64_t k) {\n\
         \x20 intent_vec_int64_t r; r.data = (int64_t*)0; r.len = 0; r.capacity = 0;\n\
         \x20 if (!xs || xs->len == 0 || k < 0 || k > 63) return r;\n\
         \x20 r.capacity = xs->len;\n\
         \x20 r.data = (int64_t*)malloc(r.capacity * sizeof(int64_t));\n\
         \x20 if (!r.data) abort();\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) r.data[i] = xs->data[i] >> k;\n\
         \x20 r.len = xs->len;\n\
         \x20 return r;\n\
         }\n\
         /* Closures #541-#545: element-wise binary ops between two Vec<i64>.\n\
          *   add / sub / mul / min / max — result[i] = xs[i] op ys[i],\n\
          *   length = min(len(xs), len(ys)). */\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_add_pairwise(const intent_vec_int64_t* xs, const intent_vec_int64_t* ys) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_add_pairwise(const intent_vec_int64_t* xs, const intent_vec_int64_t* ys) {\n\
         \x20 intent_vec_int64_t r; r.data = (int64_t*)0; r.len = 0; r.capacity = 0;\n\
         \x20 if (!xs || !ys) return r;\n\
         \x20 uint64_t n = xs->len < ys->len ? xs->len : ys->len;\n\
         \x20 if (n == 0) return r;\n\
         \x20 r.capacity = n;\n\
         \x20 r.data = (int64_t*)malloc(n * sizeof(int64_t));\n\
         \x20 if (!r.data) abort();\n\
         \x20 for (uint64_t i = 0; i < n; i++) r.data[i] = xs->data[i] + ys->data[i];\n\
         \x20 r.len = n;\n\
         \x20 return r;\n\
         }\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_sub_pairwise(const intent_vec_int64_t* xs, const intent_vec_int64_t* ys) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_sub_pairwise(const intent_vec_int64_t* xs, const intent_vec_int64_t* ys) {\n\
         \x20 intent_vec_int64_t r; r.data = (int64_t*)0; r.len = 0; r.capacity = 0;\n\
         \x20 if (!xs || !ys) return r;\n\
         \x20 uint64_t n = xs->len < ys->len ? xs->len : ys->len;\n\
         \x20 if (n == 0) return r;\n\
         \x20 r.capacity = n;\n\
         \x20 r.data = (int64_t*)malloc(n * sizeof(int64_t));\n\
         \x20 if (!r.data) abort();\n\
         \x20 for (uint64_t i = 0; i < n; i++) r.data[i] = xs->data[i] - ys->data[i];\n\
         \x20 r.len = n;\n\
         \x20 return r;\n\
         }\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_mul_pairwise(const intent_vec_int64_t* xs, const intent_vec_int64_t* ys) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_mul_pairwise(const intent_vec_int64_t* xs, const intent_vec_int64_t* ys) {\n\
         \x20 intent_vec_int64_t r; r.data = (int64_t*)0; r.len = 0; r.capacity = 0;\n\
         \x20 if (!xs || !ys) return r;\n\
         \x20 uint64_t n = xs->len < ys->len ? xs->len : ys->len;\n\
         \x20 if (n == 0) return r;\n\
         \x20 r.capacity = n;\n\
         \x20 r.data = (int64_t*)malloc(n * sizeof(int64_t));\n\
         \x20 if (!r.data) abort();\n\
         \x20 for (uint64_t i = 0; i < n; i++) r.data[i] = xs->data[i] * ys->data[i];\n\
         \x20 r.len = n;\n\
         \x20 return r;\n\
         }\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_min_pairwise(const intent_vec_int64_t* xs, const intent_vec_int64_t* ys) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_min_pairwise(const intent_vec_int64_t* xs, const intent_vec_int64_t* ys) {\n\
         \x20 intent_vec_int64_t r; r.data = (int64_t*)0; r.len = 0; r.capacity = 0;\n\
         \x20 if (!xs || !ys) return r;\n\
         \x20 uint64_t n = xs->len < ys->len ? xs->len : ys->len;\n\
         \x20 if (n == 0) return r;\n\
         \x20 r.capacity = n;\n\
         \x20 r.data = (int64_t*)malloc(n * sizeof(int64_t));\n\
         \x20 if (!r.data) abort();\n\
         \x20 for (uint64_t i = 0; i < n; i++) r.data[i] = xs->data[i] < ys->data[i] ? xs->data[i] : ys->data[i];\n\
         \x20 r.len = n;\n\
         \x20 return r;\n\
         }\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_max_pairwise(const intent_vec_int64_t* xs, const intent_vec_int64_t* ys) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_max_pairwise(const intent_vec_int64_t* xs, const intent_vec_int64_t* ys) {\n\
         \x20 intent_vec_int64_t r; r.data = (int64_t*)0; r.len = 0; r.capacity = 0;\n\
         \x20 if (!xs || !ys) return r;\n\
         \x20 uint64_t n = xs->len < ys->len ? xs->len : ys->len;\n\
         \x20 if (n == 0) return r;\n\
         \x20 r.capacity = n;\n\
         \x20 r.data = (int64_t*)malloc(n * sizeof(int64_t));\n\
         \x20 if (!r.data) abort();\n\
         \x20 for (uint64_t i = 0; i < n; i++) r.data[i] = xs->data[i] > ys->data[i] ? xs->data[i] : ys->data[i];\n\
         \x20 r.len = n;\n\
         \x20 return r;\n\
         }\n\
         /* Closure #540: vec_clamp_scalar(ref xs, lo, hi).\n\
          * result[i] = (xs[i] < lo) ? lo : (xs[i] > hi) ? hi : xs[i]. */\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_clamp_scalar(const intent_vec_int64_t* xs, int64_t lo, int64_t hi) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_clamp_scalar(const intent_vec_int64_t* xs, int64_t lo, int64_t hi) {\n\
         \x20 intent_vec_int64_t r; r.data = (int64_t*)0; r.len = 0; r.capacity = 0;\n\
         \x20 if (!xs || xs->len == 0) return r;\n\
         \x20 r.capacity = xs->len;\n\
         \x20 r.data = (int64_t*)malloc(r.capacity * sizeof(int64_t));\n\
         \x20 if (!r.data) abort();\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) {\n\
         \x20   int64_t x = xs->data[i];\n\
         \x20   r.data[i] = x < lo ? lo : (x > hi ? hi : x);\n\
         \x20 }\n\
         \x20 r.len = xs->len;\n\
         \x20 return r;\n\
         }\n\
         /* Closures #538/#539: scalar min/max — element-wise floor/ceil.\n\
          *   min_with_scalar(xs, v): elt-wise min(xs[i], v)\n\
          *   max_with_scalar(xs, v): elt-wise max(xs[i], v) */\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_min_with_scalar(const intent_vec_int64_t* xs, int64_t v) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_min_with_scalar(const intent_vec_int64_t* xs, int64_t v) {\n\
         \x20 intent_vec_int64_t r; r.data = (int64_t*)0; r.len = 0; r.capacity = 0;\n\
         \x20 if (!xs || xs->len == 0) return r;\n\
         \x20 r.capacity = xs->len;\n\
         \x20 r.data = (int64_t*)malloc(r.capacity * sizeof(int64_t));\n\
         \x20 if (!r.data) abort();\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) r.data[i] = xs->data[i] < v ? xs->data[i] : v;\n\
         \x20 r.len = xs->len;\n\
         \x20 return r;\n\
         }\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_max_with_scalar(const intent_vec_int64_t* xs, int64_t v) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_max_with_scalar(const intent_vec_int64_t* xs, int64_t v) {\n\
         \x20 intent_vec_int64_t r; r.data = (int64_t*)0; r.len = 0; r.capacity = 0;\n\
         \x20 if (!xs || xs->len == 0) return r;\n\
         \x20 r.capacity = xs->len;\n\
         \x20 r.data = (int64_t*)malloc(r.capacity * sizeof(int64_t));\n\
         \x20 if (!r.data) abort();\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) r.data[i] = xs->data[i] > v ? xs->data[i] : v;\n\
         \x20 r.len = xs->len;\n\
         \x20 return r;\n\
         }\n\
         /* Closures #532-#537: scalar comparison masks on Vec<i64>.\n\
          * Each returns a fresh Vec<i64> of 0/1 elements where 1\n\
          * indicates the comparison holds at that index. Useful for\n\
          * branchless SIMD-style programming. */\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_eq_mask(const intent_vec_int64_t* xs, int64_t v) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_eq_mask(const intent_vec_int64_t* xs, int64_t v) {\n\
         \x20 intent_vec_int64_t r; r.data = (int64_t*)0; r.len = 0; r.capacity = 0;\n\
         \x20 if (!xs || xs->len == 0) return r;\n\
         \x20 r.capacity = xs->len;\n\
         \x20 r.data = (int64_t*)malloc(r.capacity * sizeof(int64_t));\n\
         \x20 if (!r.data) abort();\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) r.data[i] = (xs->data[i] == v) ? 1 : 0;\n\
         \x20 r.len = xs->len;\n\
         \x20 return r;\n\
         }\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_ne_mask(const intent_vec_int64_t* xs, int64_t v) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_ne_mask(const intent_vec_int64_t* xs, int64_t v) {\n\
         \x20 intent_vec_int64_t r; r.data = (int64_t*)0; r.len = 0; r.capacity = 0;\n\
         \x20 if (!xs || xs->len == 0) return r;\n\
         \x20 r.capacity = xs->len;\n\
         \x20 r.data = (int64_t*)malloc(r.capacity * sizeof(int64_t));\n\
         \x20 if (!r.data) abort();\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) r.data[i] = (xs->data[i] != v) ? 1 : 0;\n\
         \x20 r.len = xs->len;\n\
         \x20 return r;\n\
         }\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_lt_mask(const intent_vec_int64_t* xs, int64_t v) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_lt_mask(const intent_vec_int64_t* xs, int64_t v) {\n\
         \x20 intent_vec_int64_t r; r.data = (int64_t*)0; r.len = 0; r.capacity = 0;\n\
         \x20 if (!xs || xs->len == 0) return r;\n\
         \x20 r.capacity = xs->len;\n\
         \x20 r.data = (int64_t*)malloc(r.capacity * sizeof(int64_t));\n\
         \x20 if (!r.data) abort();\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) r.data[i] = (xs->data[i] < v) ? 1 : 0;\n\
         \x20 r.len = xs->len;\n\
         \x20 return r;\n\
         }\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_le_mask(const intent_vec_int64_t* xs, int64_t v) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_le_mask(const intent_vec_int64_t* xs, int64_t v) {\n\
         \x20 intent_vec_int64_t r; r.data = (int64_t*)0; r.len = 0; r.capacity = 0;\n\
         \x20 if (!xs || xs->len == 0) return r;\n\
         \x20 r.capacity = xs->len;\n\
         \x20 r.data = (int64_t*)malloc(r.capacity * sizeof(int64_t));\n\
         \x20 if (!r.data) abort();\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) r.data[i] = (xs->data[i] <= v) ? 1 : 0;\n\
         \x20 r.len = xs->len;\n\
         \x20 return r;\n\
         }\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_gt_mask(const intent_vec_int64_t* xs, int64_t v) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_gt_mask(const intent_vec_int64_t* xs, int64_t v) {\n\
         \x20 intent_vec_int64_t r; r.data = (int64_t*)0; r.len = 0; r.capacity = 0;\n\
         \x20 if (!xs || xs->len == 0) return r;\n\
         \x20 r.capacity = xs->len;\n\
         \x20 r.data = (int64_t*)malloc(r.capacity * sizeof(int64_t));\n\
         \x20 if (!r.data) abort();\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) r.data[i] = (xs->data[i] > v) ? 1 : 0;\n\
         \x20 r.len = xs->len;\n\
         \x20 return r;\n\
         }\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_ge_mask(const intent_vec_int64_t* xs, int64_t v) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_ge_mask(const intent_vec_int64_t* xs, int64_t v) {\n\
         \x20 intent_vec_int64_t r; r.data = (int64_t*)0; r.len = 0; r.capacity = 0;\n\
         \x20 if (!xs || xs->len == 0) return r;\n\
         \x20 r.capacity = xs->len;\n\
         \x20 r.data = (int64_t*)malloc(r.capacity * sizeof(int64_t));\n\
         \x20 if (!r.data) abort();\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) r.data[i] = (xs->data[i] >= v) ? 1 : 0;\n\
         \x20 r.len = xs->len;\n\
         \x20 return r;\n\
         }\n\
         /* Closures #528-#531: Vec<i64> scalar-broadcast arithmetic.\n\
          *   add_scalar(xs, v): xs[i] + v\n\
          *   sub_scalar(xs, v): xs[i] - v\n\
          *   mul_scalar(xs, v): xs[i] * v\n\
          *   div_scalar(xs, v): xs[i] / v  (returns empty Vec if v == 0) */\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_add_scalar(const intent_vec_int64_t* xs, int64_t v) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_add_scalar(const intent_vec_int64_t* xs, int64_t v) {\n\
         \x20 intent_vec_int64_t r; r.data = (int64_t*)0; r.len = 0; r.capacity = 0;\n\
         \x20 if (!xs || xs->len == 0) return r;\n\
         \x20 r.capacity = xs->len;\n\
         \x20 r.data = (int64_t*)malloc(r.capacity * sizeof(int64_t));\n\
         \x20 if (!r.data) abort();\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) r.data[i] = xs->data[i] + v;\n\
         \x20 r.len = xs->len;\n\
         \x20 return r;\n\
         }\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_sub_scalar(const intent_vec_int64_t* xs, int64_t v) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_sub_scalar(const intent_vec_int64_t* xs, int64_t v) {\n\
         \x20 intent_vec_int64_t r; r.data = (int64_t*)0; r.len = 0; r.capacity = 0;\n\
         \x20 if (!xs || xs->len == 0) return r;\n\
         \x20 r.capacity = xs->len;\n\
         \x20 r.data = (int64_t*)malloc(r.capacity * sizeof(int64_t));\n\
         \x20 if (!r.data) abort();\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) r.data[i] = xs->data[i] - v;\n\
         \x20 r.len = xs->len;\n\
         \x20 return r;\n\
         }\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_mul_scalar(const intent_vec_int64_t* xs, int64_t v) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_mul_scalar(const intent_vec_int64_t* xs, int64_t v) {\n\
         \x20 intent_vec_int64_t r; r.data = (int64_t*)0; r.len = 0; r.capacity = 0;\n\
         \x20 if (!xs || xs->len == 0) return r;\n\
         \x20 r.capacity = xs->len;\n\
         \x20 r.data = (int64_t*)malloc(r.capacity * sizeof(int64_t));\n\
         \x20 if (!r.data) abort();\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) r.data[i] = xs->data[i] * v;\n\
         \x20 r.len = xs->len;\n\
         \x20 return r;\n\
         }\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_div_scalar(const intent_vec_int64_t* xs, int64_t v) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_div_scalar(const intent_vec_int64_t* xs, int64_t v) {\n\
         \x20 intent_vec_int64_t r; r.data = (int64_t*)0; r.len = 0; r.capacity = 0;\n\
         \x20 if (!xs || xs->len == 0 || v == 0) return r;\n\
         \x20 r.capacity = xs->len;\n\
         \x20 r.data = (int64_t*)malloc(r.capacity * sizeof(int64_t));\n\
         \x20 if (!r.data) abort();\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) r.data[i] = xs->data[i] / v;\n\
         \x20 r.len = xs->len;\n\
         \x20 return r;\n\
         }\n\
         /* Closures #524-#527: element-wise unary Vec<i64> transforms.\n\
          *   abs     — |xs[i]|  (LLONG_MIN stays LLONG_MIN, i.e. wraps as per llabs spec)\n\
          *   negate  — -xs[i]\n\
          *   signum  — -1/0/+1\n\
          *   square  — xs[i] * xs[i] (may overflow; wrap is well-defined on signed i64 here) */\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_abs(const intent_vec_int64_t* xs) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_abs(const intent_vec_int64_t* xs) {\n\
         \x20 intent_vec_int64_t v; v.data = (int64_t*)0; v.len = 0; v.capacity = 0;\n\
         \x20 if (!xs || xs->len == 0) return v;\n\
         \x20 v.capacity = xs->len;\n\
         \x20 v.data = (int64_t*)malloc(v.capacity * sizeof(int64_t));\n\
         \x20 if (!v.data) abort();\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) {\n\
         \x20   int64_t x = xs->data[i];\n\
         \x20   v.data[i] = x < 0 ? -x : x;\n\
         \x20 }\n\
         \x20 v.len = xs->len;\n\
         \x20 return v;\n\
         }\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_negate(const intent_vec_int64_t* xs) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_negate(const intent_vec_int64_t* xs) {\n\
         \x20 intent_vec_int64_t v; v.data = (int64_t*)0; v.len = 0; v.capacity = 0;\n\
         \x20 if (!xs || xs->len == 0) return v;\n\
         \x20 v.capacity = xs->len;\n\
         \x20 v.data = (int64_t*)malloc(v.capacity * sizeof(int64_t));\n\
         \x20 if (!v.data) abort();\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) v.data[i] = -xs->data[i];\n\
         \x20 v.len = xs->len;\n\
         \x20 return v;\n\
         }\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_signum(const intent_vec_int64_t* xs) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_signum(const intent_vec_int64_t* xs) {\n\
         \x20 intent_vec_int64_t v; v.data = (int64_t*)0; v.len = 0; v.capacity = 0;\n\
         \x20 if (!xs || xs->len == 0) return v;\n\
         \x20 v.capacity = xs->len;\n\
         \x20 v.data = (int64_t*)malloc(v.capacity * sizeof(int64_t));\n\
         \x20 if (!v.data) abort();\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) {\n\
         \x20   int64_t x = xs->data[i];\n\
         \x20   v.data[i] = (x > 0) - (x < 0);\n\
         \x20 }\n\
         \x20 v.len = xs->len;\n\
         \x20 return v;\n\
         }\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_square(const intent_vec_int64_t* xs) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_square(const intent_vec_int64_t* xs) {\n\
         \x20 intent_vec_int64_t v; v.data = (int64_t*)0; v.len = 0; v.capacity = 0;\n\
         \x20 if (!xs || xs->len == 0) return v;\n\
         \x20 v.capacity = xs->len;\n\
         \x20 v.data = (int64_t*)malloc(v.capacity * sizeof(int64_t));\n\
         \x20 if (!v.data) abort();\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) v.data[i] = xs->data[i] * xs->data[i];\n\
         \x20 v.len = xs->len;\n\
         \x20 return v;\n\
         }\n\
         /* Closures #520-#523: vec_sliding_max/min/sum/product.\n\
          * Rolling window of size k over Vec<i64>; returns a fresh\n\
          * Vec<i64> of length n-k+1. Empty result when n < k or k <= 0.\n\
          * v1 implementation: O(n*k) per window, simple loop.\n\
          * O(n) deque/segment-tree algorithms are a future optimization. */\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_sliding_max(const intent_vec_int64_t* xs, int64_t k) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_sliding_max(const intent_vec_int64_t* xs, int64_t k) {\n\
         \x20 intent_vec_int64_t v; v.data = (int64_t*)0; v.len = 0; v.capacity = 0;\n\
         \x20 if (!xs || k <= 0 || (uint64_t)k > xs->len) return v;\n\
         \x20 uint64_t out_len = xs->len - (uint64_t)k + 1;\n\
         \x20 v.capacity = out_len;\n\
         \x20 v.data = (int64_t*)malloc(out_len * sizeof(int64_t));\n\
         \x20 if (!v.data) abort();\n\
         \x20 for (uint64_t i = 0; i < out_len; i++) {\n\
         \x20   int64_t m = xs->data[i];\n\
         \x20   for (int64_t j = 1; j < k; j++) if (xs->data[i + (uint64_t)j] > m) m = xs->data[i + (uint64_t)j];\n\
         \x20   v.data[i] = m;\n\
         \x20 }\n\
         \x20 v.len = out_len;\n\
         \x20 return v;\n\
         }\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_sliding_min(const intent_vec_int64_t* xs, int64_t k) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_sliding_min(const intent_vec_int64_t* xs, int64_t k) {\n\
         \x20 intent_vec_int64_t v; v.data = (int64_t*)0; v.len = 0; v.capacity = 0;\n\
         \x20 if (!xs || k <= 0 || (uint64_t)k > xs->len) return v;\n\
         \x20 uint64_t out_len = xs->len - (uint64_t)k + 1;\n\
         \x20 v.capacity = out_len;\n\
         \x20 v.data = (int64_t*)malloc(out_len * sizeof(int64_t));\n\
         \x20 if (!v.data) abort();\n\
         \x20 for (uint64_t i = 0; i < out_len; i++) {\n\
         \x20   int64_t m = xs->data[i];\n\
         \x20   for (int64_t j = 1; j < k; j++) if (xs->data[i + (uint64_t)j] < m) m = xs->data[i + (uint64_t)j];\n\
         \x20   v.data[i] = m;\n\
         \x20 }\n\
         \x20 v.len = out_len;\n\
         \x20 return v;\n\
         }\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_sliding_sum(const intent_vec_int64_t* xs, int64_t k) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_sliding_sum(const intent_vec_int64_t* xs, int64_t k) {\n\
         \x20 intent_vec_int64_t v; v.data = (int64_t*)0; v.len = 0; v.capacity = 0;\n\
         \x20 if (!xs || k <= 0 || (uint64_t)k > xs->len) return v;\n\
         \x20 uint64_t out_len = xs->len - (uint64_t)k + 1;\n\
         \x20 v.capacity = out_len;\n\
         \x20 v.data = (int64_t*)malloc(out_len * sizeof(int64_t));\n\
         \x20 if (!v.data) abort();\n\
         \x20 int64_t acc = 0;\n\
         \x20 for (int64_t j = 0; j < k; j++) acc += xs->data[(uint64_t)j];\n\
         \x20 v.data[0] = acc;\n\
         \x20 for (uint64_t i = 1; i < out_len; i++) {\n\
         \x20   acc = acc - xs->data[i - 1] + xs->data[i + (uint64_t)k - 1];\n\
         \x20   v.data[i] = acc;\n\
         \x20 }\n\
         \x20 v.len = out_len;\n\
         \x20 return v;\n\
         }\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_sliding_product(const intent_vec_int64_t* xs, int64_t k) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_sliding_product(const intent_vec_int64_t* xs, int64_t k) {\n\
         \x20 intent_vec_int64_t v; v.data = (int64_t*)0; v.len = 0; v.capacity = 0;\n\
         \x20 if (!xs || k <= 0 || (uint64_t)k > xs->len) return v;\n\
         \x20 uint64_t out_len = xs->len - (uint64_t)k + 1;\n\
         \x20 v.capacity = out_len;\n\
         \x20 v.data = (int64_t*)malloc(out_len * sizeof(int64_t));\n\
         \x20 if (!v.data) abort();\n\
         \x20 for (uint64_t i = 0; i < out_len; i++) {\n\
         \x20   int64_t p = 1;\n\
         \x20   for (int64_t j = 0; j < k; j++) p = p * xs->data[i + (uint64_t)j];\n\
         \x20   v.data[i] = p;\n\
         \x20 }\n\
         \x20 v.len = out_len;\n\
         \x20 return v;\n\
         }\n\
         /* Closures #516-#519: Vec<i64> predicates.\n\
          *   all_equal       — true iff all elts equal (vacuous: empty/single → true)\n\
          *   is_sorted_asc   — true iff non-decreasing (vacuous: empty/single → true)\n\
          *   is_sorted_desc  — true iff non-increasing (vacuous: empty/single → true)\n\
          *   is_palindrome   — true iff xs == reverse(xs) (vacuous: empty/single → true) */\n\
         static INTENT_UNUSED bool intent_vec_int64_t_all_equal(const intent_vec_int64_t* xs) INTENT_UNUSED;\n\
         static INTENT_UNUSED bool intent_vec_int64_t_all_equal(const intent_vec_int64_t* xs) {\n\
         \x20 if (!xs || xs->len < 2) return true;\n\
         \x20 int64_t v = xs->data[0];\n\
         \x20 for (uint64_t i = 1; i < xs->len; i++) if (xs->data[i] != v) return false;\n\
         \x20 return true;\n\
         }\n\
         static INTENT_UNUSED bool intent_vec_int64_t_is_sorted_asc(const intent_vec_int64_t* xs) INTENT_UNUSED;\n\
         static INTENT_UNUSED bool intent_vec_int64_t_is_sorted_asc(const intent_vec_int64_t* xs) {\n\
         \x20 if (!xs || xs->len < 2) return true;\n\
         \x20 for (uint64_t i = 1; i < xs->len; i++) if (xs->data[i] < xs->data[i-1]) return false;\n\
         \x20 return true;\n\
         }\n\
         static INTENT_UNUSED bool intent_vec_int64_t_is_sorted_desc(const intent_vec_int64_t* xs) INTENT_UNUSED;\n\
         static INTENT_UNUSED bool intent_vec_int64_t_is_sorted_desc(const intent_vec_int64_t* xs) {\n\
         \x20 if (!xs || xs->len < 2) return true;\n\
         \x20 for (uint64_t i = 1; i < xs->len; i++) if (xs->data[i] > xs->data[i-1]) return false;\n\
         \x20 return true;\n\
         }\n\
         static INTENT_UNUSED bool intent_vec_int64_t_is_palindrome(const intent_vec_int64_t* xs) INTENT_UNUSED;\n\
         static INTENT_UNUSED bool intent_vec_int64_t_is_palindrome(const intent_vec_int64_t* xs) {\n\
         \x20 if (!xs || xs->len < 2) return true;\n\
         \x20 uint64_t i = 0; uint64_t j = xs->len - 1;\n\
         \x20 while (i < j) {\n\
         \x20   if (xs->data[i] != xs->data[j]) return false;\n\
         \x20   i++; j--;\n\
         \x20 }\n\
         \x20 return true;\n\
         }\n\
         /* Closures #510/#511: vec_cumulative_max / vec_cumulative_min.\n\
          * Running max / min: result[i] = extremum(xs[0..=i]). */\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_cumulative_max(const intent_vec_int64_t* xs) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_cumulative_max(const intent_vec_int64_t* xs) {\n\
         \x20 intent_vec_int64_t v; v.data = (int64_t*)0; v.len = 0; v.capacity = 0;\n\
         \x20 if (!xs || xs->len == 0) return v;\n\
         \x20 v.capacity = xs->len;\n\
         \x20 v.data = (int64_t*)malloc(v.capacity * sizeof(int64_t));\n\
         \x20 if (!v.data) abort();\n\
         \x20 int64_t m = xs->data[0]; v.data[0] = m;\n\
         \x20 for (uint64_t i = 1; i < xs->len; i++) {\n\
         \x20   if (xs->data[i] > m) m = xs->data[i];\n\
         \x20   v.data[i] = m;\n\
         \x20 }\n\
         \x20 v.len = xs->len;\n\
         \x20 return v;\n\
         }\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_cumulative_min(const intent_vec_int64_t* xs) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_cumulative_min(const intent_vec_int64_t* xs) {\n\
         \x20 intent_vec_int64_t v; v.data = (int64_t*)0; v.len = 0; v.capacity = 0;\n\
         \x20 if (!xs || xs->len == 0) return v;\n\
         \x20 v.capacity = xs->len;\n\
         \x20 v.data = (int64_t*)malloc(v.capacity * sizeof(int64_t));\n\
         \x20 if (!v.data) abort();\n\
         \x20 int64_t m = xs->data[0]; v.data[0] = m;\n\
         \x20 for (uint64_t i = 1; i < xs->len; i++) {\n\
         \x20   if (xs->data[i] < m) m = xs->data[i];\n\
         \x20   v.data[i] = m;\n\
         \x20 }\n\
         \x20 v.len = xs->len;\n\
         \x20 return v;\n\
         }\n\
         /* Closure #382: vec_iota(n) -> Vec<i64>. Fills [0, n).\n\
          * Specialization of vec_range(0, n) — slightly tighter than\n\
          * the general range form since we always start at 0. */\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_iota(int64_t n) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_iota(int64_t n) {\n\
         \x20 intent_vec_int64_t v; v.data = (int64_t*)0; v.len = 0; v.capacity = 0;\n\
         \x20 if (n <= 0) return v;\n\
         \x20 v.capacity = (uint64_t)n;\n\
         \x20 v.data = (int64_t*)malloc(v.capacity * sizeof(int64_t));\n\
         \x20 if (!v.data) abort();\n\
         \x20 for (int64_t i = 0; i < n; i++) v.data[i] = i;\n\
         \x20 v.len = (uint64_t)n;\n\
         \x20 return v;\n\
         }\n\
         /* Closure #371: vec_reverse_copy / vec_unique. Both take a\n\
          * const-pointer to the source Vec<i64> and produce a fresh\n\
          * heap-allocated Vec<i64>. vec_reverse_copy is a straight\n\
          * memcpy in reverse order; vec_unique walks once tracking\n\
          * whether each element appeared earlier (O(n^2) — fine for\n\
          * the v1 i64-only audience). */\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_reverse_copy(const intent_vec_int64_t* xs) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_reverse_copy(const intent_vec_int64_t* xs) {\n\
         \x20 intent_vec_int64_t v; v.data = (int64_t*)0; v.len = 0; v.capacity = 0;\n\
         \x20 if (!xs || xs->len == 0) return v;\n\
         \x20 v.capacity = xs->len;\n\
         \x20 v.data = (int64_t*)malloc(v.capacity * sizeof(int64_t));\n\
         \x20 if (!v.data) abort();\n\
         \x20 for (size_t i = 0; i < xs->len; ++i) {\n\
         \x20   v.data[xs->len - 1 - i] = xs->data[i];\n\
         \x20 }\n\
         \x20 v.len = xs->len;\n\
         \x20 return v;\n\
         }\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_unique(const intent_vec_int64_t* xs) INTENT_UNUSED;\n\
         static INTENT_UNUSED intent_vec_int64_t intent_vec_int64_t_unique(const intent_vec_int64_t* xs) {\n\
         \x20 intent_vec_int64_t v; v.data = (int64_t*)0; v.len = 0; v.capacity = 0;\n\
         \x20 if (!xs || xs->len == 0) return v;\n\
         \x20 v.capacity = xs->len;\n\
         \x20 v.data = (int64_t*)malloc(v.capacity * sizeof(int64_t));\n\
         \x20 if (!v.data) abort();\n\
         \x20 for (size_t i = 0; i < xs->len; ++i) {\n\
         \x20   int seen = 0;\n\
         \x20   for (size_t j = 0; j < v.len; ++j) {\n\
         \x20     if (v.data[j] == xs->data[i]) { seen = 1; break; }\n\
         \x20   }\n\
         \x20   if (!seen) { v.data[v.len++] = xs->data[i]; }\n\
         \x20 }\n\
         \x20 return v;\n\
         }\n\n",
    );
    // Closure #385: vec_first / vec_last(ref xs) -> Option<i64>.
    // Gated on Option<i64> being in the payload registry — the
    // helper bodies reference `Enum_Option__i64`. If callers
    // already use anything that materialized the Option<i64>
    // monomorph (the auto-mono walker registers it for these),
    // the struct is in scope.
    let has_option_i64 = ENUM_PAYLOAD_REGISTRY.with(|r| {
        r.borrow().contains_key("Option__i64")
    });
    if has_option_i64 {
        out.push_str(
            "static INTENT_UNUSED Enum_Option__i64 intent_vec_int64_t_first(const intent_vec_int64_t* xs) INTENT_UNUSED;\n\
             static INTENT_UNUSED Enum_Option__i64 intent_vec_int64_t_first(const intent_vec_int64_t* xs) {\n\
             \x20 Enum_Option__i64 r;\n\
             \x20 if (!xs || xs->len == 0) { r.tag = 1; return r; }\n\
             \x20 r.tag = 0; r.payload = xs->data[0];\n\
             \x20 return r;\n\
             }\n\
             static INTENT_UNUSED Enum_Option__i64 intent_vec_int64_t_last(const intent_vec_int64_t* xs) INTENT_UNUSED;\n\
             static INTENT_UNUSED Enum_Option__i64 intent_vec_int64_t_last(const intent_vec_int64_t* xs) {\n\
             \x20 Enum_Option__i64 r;\n\
             \x20 if (!xs || xs->len == 0) { r.tag = 1; return r; }\n\
             \x20 r.tag = 0; r.payload = xs->data[xs->len - 1];\n\
             \x20 return r;\n\
             }\n\n",
        );
    }
}

/// Closure #379: `str_join(ref strs, sep) -> OwnedStr`. Two-pass
/// concat: compute the total byte length, allocate once, then
/// memcpy each segment with sep between them.
pub(crate) fn emit_intent_str_join_c(out: &mut String) {
    out.push_str(
        "static char* intent_str_join(const intent_vec_owned_str* xs, const char* sep) INTENT_UNUSED;\n\
         static char* intent_str_join(const intent_vec_owned_str* xs, const char* sep) {\n\
         \x20 if (!xs || xs->len == 0) {\n\
         \x20   char* e = (char*)malloc(1);\n\
         \x20   if (!e) abort();\n\
         \x20   e[0] = 0;\n\
         \x20   return e;\n\
         \x20 }\n\
         \x20 size_t sep_l = sep ? strlen(sep) : 0;\n\
         \x20 size_t total = 0;\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) {\n\
         \x20   const char* s = xs->data[i];\n\
         \x20   if (s) total += strlen(s);\n\
         \x20 }\n\
         \x20 if (xs->len > 1) total += sep_l * (xs->len - 1);\n\
         \x20 char* out = (char*)malloc(total + 1);\n\
         \x20 if (!out) abort();\n\
         \x20 char* p = out;\n\
         \x20 for (uint64_t i = 0; i < xs->len; i++) {\n\
         \x20   if (i > 0 && sep_l > 0) {\n\
         \x20     memcpy(p, sep, sep_l);\n\
         \x20     p += sep_l;\n\
         \x20   }\n\
         \x20   const char* s = xs->data[i];\n\
         \x20   if (s) {\n\
         \x20     size_t sl = strlen(s);\n\
         \x20     memcpy(p, s, sl);\n\
         \x20     p += sl;\n\
         \x20   }\n\
         \x20 }\n\
         \x20 out[total] = 0;\n\
         \x20 return out;\n\
         }\n\n",
    );
}

/// Closure #381: `str_lines(s) -> Vec<OwnedStr>`. Splits `s`
/// on '\n', stripping any trailing '\r' for CRLF compatibility.
/// Implemented as a wrapper around `intent_str_split(s, "\n")`
/// that walks the resulting Vec<OwnedStr> and drops trailing
/// CR bytes in place. Pure copy semantics — caller drops the
/// returned Vec.
pub(crate) fn emit_intent_str_lines_c(out: &mut String) {
    out.push_str(
        "static intent_vec_owned_str intent_str_lines(const char* s) INTENT_UNUSED;\n\
         static intent_vec_owned_str intent_str_lines(const char* s) {\n\
         \x20 intent_vec_owned_str v = intent_str_split(s, \"\\n\");\n\
         \x20 for (uint64_t i = 0; i < v.len; i++) {\n\
         \x20   char* line = v.data[i];\n\
         \x20   if (!line) continue;\n\
         \x20   size_t ll = strlen(line);\n\
         \x20   if (ll > 0 && line[ll - 1] == '\\r') line[ll - 1] = 0;\n\
         \x20 }\n\
         \x20 return v;\n\
         }\n\n",
    );
}

/// Closure #394: `str_strip_prefix(s, p)` / `str_strip_suffix(s,
/// sfx)` -> OwnedStr. Returns a fresh copy with the prefix /
/// suffix removed if it matches; otherwise returns a fresh copy
/// of `s` unchanged. Empty prefix / suffix never matches in the
/// "strip" sense, so returns `s` unchanged.
pub(crate) fn emit_intent_str_strip_c(out: &mut String) {
    out.push_str(
        "static char* intent_str_strip_prefix(const char* s, const char* p) INTENT_UNUSED;\n\
         static char* intent_str_strip_prefix(const char* s, const char* p) {\n\
         \x20 size_t sl = s ? strlen(s) : 0;\n\
         \x20 size_t pl = p ? strlen(p) : 0;\n\
         \x20 size_t off = 0;\n\
         \x20 if (pl > 0 && pl <= sl && strncmp(s, p, pl) == 0) off = pl;\n\
         \x20 size_t out_len = sl - off;\n\
         \x20 char* out = (char*)malloc(out_len + 1);\n\
         \x20 if (!out) abort();\n\
         \x20 if (out_len > 0) memcpy(out, s + off, out_len);\n\
         \x20 out[out_len] = 0;\n\
         \x20 return out;\n\
         }\n\
         static char* intent_str_strip_suffix(const char* s, const char* sfx) INTENT_UNUSED;\n\
         static char* intent_str_strip_suffix(const char* s, const char* sfx) {\n\
         \x20 size_t sl = s ? strlen(s) : 0;\n\
         \x20 size_t fl = sfx ? strlen(sfx) : 0;\n\
         \x20 size_t out_len = sl;\n\
         \x20 if (fl > 0 && fl <= sl && strncmp(s + sl - fl, sfx, fl) == 0) out_len = sl - fl;\n\
         \x20 char* out = (char*)malloc(out_len + 1);\n\
         \x20 if (!out) abort();\n\
         \x20 if (out_len > 0) memcpy(out, s, out_len);\n\
         \x20 out[out_len] = 0;\n\
         \x20 return out;\n\
         }\n\n",
    );
}

/// Closure #395: `str_count_char(s, ch) -> i64`. Counts the
/// occurrences of the first byte of `ch` in `s`. Empty ch
/// returns 0 (no characters to count).
pub(crate) fn emit_intent_str_count_char_c(out: &mut String) {
    out.push_str(
        "static int64_t intent_str_count_char(const char* s, const char* ch) INTENT_UNUSED;\n\
         static int64_t intent_str_count_char(const char* s, const char* ch) {\n\
         \x20 if (!s || !ch || ch[0] == 0) return 0;\n\
         \x20 char target = ch[0];\n\
         \x20 int64_t n = 0;\n\
         \x20 for (const char* p = s; *p; p++) if (*p == target) n++;\n\
         \x20 return n;\n\
         }\n\n",
    );
}

/// Closure #390: `str_reverse(s) -> OwnedStr`. Byte-reverse
/// a Str (not codepoint-reverse — UTF-8 sequences are
/// byte-wise reversed).
pub(crate) fn emit_intent_str_reverse_c(out: &mut String) {
    out.push_str(
        "static char* intent_str_reverse(const char* s) INTENT_UNUSED;\n\
         static char* intent_str_reverse(const char* s) {\n\
         \x20 size_t sl = s ? strlen(s) : 0;\n\
         \x20 char* out = (char*)malloc(sl + 1);\n\
         \x20 if (!out) abort();\n\
         \x20 for (size_t i = 0; i < sl; i++) out[i] = s[sl - 1 - i];\n\
         \x20 out[sl] = 0;\n\
         \x20 return out;\n\
         }\n\n",
    );
}

/// Closure #390: `str_chars(s) -> Vec<i64>`. Returns a fresh
/// Vec<i64> where each element is the byte value [0..255] of
/// `s`. Note: byte-level, not codepoint-level.
pub(crate) fn emit_intent_str_chars_c(out: &mut String) {
    out.push_str(
        "static intent_vec_int64_t intent_str_chars(const char* s) INTENT_UNUSED;\n\
         static intent_vec_int64_t intent_str_chars(const char* s) {\n\
         \x20 intent_vec_int64_t out;\n\
         \x20 size_t sl = s ? strlen(s) : 0;\n\
         \x20 out.len = sl; out.capacity = sl;\n\
         \x20 if (sl == 0) { out.data = (int64_t*)0; return out; }\n\
         \x20 out.data = (int64_t*)malloc(sl * sizeof(int64_t));\n\
         \x20 if (!out.data) abort();\n\
         \x20 for (size_t i = 0; i < sl; i++) out.data[i] = (int64_t)(uint8_t)s[i];\n\
         \x20 return out;\n\
         }\n\n",
    );
}

/// Closure #381: str_pad_left / str_pad_right(s, n, ch). Pad
/// `s` with the first byte of `ch` until total length is at
/// least `n`. If `s` is already at least `n` bytes, returns a
/// fresh malloc'd copy unchanged. `ch` is a Str — only its
/// first byte is used (empty `ch` defaults to ' ').
pub(crate) fn emit_intent_str_pad_c(out: &mut String) {
    out.push_str(
        "static char* intent_str_pad_left(const char* s, int64_t n, const char* ch) INTENT_UNUSED;\n\
         static char* intent_str_pad_left(const char* s, int64_t n, const char* ch) {\n\
         \x20 size_t sl = s ? strlen(s) : 0;\n\
         \x20 char fill = (ch && ch[0]) ? ch[0] : ' ';\n\
         \x20 size_t target = (n < 0 || (uint64_t)n < sl) ? sl : (size_t)n;\n\
         \x20 char* out = (char*)malloc(target + 1);\n\
         \x20 if (!out) abort();\n\
         \x20 size_t pad_count = target - sl;\n\
         \x20 for (size_t i = 0; i < pad_count; i++) out[i] = fill;\n\
         \x20 if (sl > 0 && s) memcpy(out + pad_count, s, sl);\n\
         \x20 out[target] = 0;\n\
         \x20 return out;\n\
         }\n\
         static char* intent_str_pad_right(const char* s, int64_t n, const char* ch) INTENT_UNUSED;\n\
         static char* intent_str_pad_right(const char* s, int64_t n, const char* ch) {\n\
         \x20 size_t sl = s ? strlen(s) : 0;\n\
         \x20 char fill = (ch && ch[0]) ? ch[0] : ' ';\n\
         \x20 size_t target = (n < 0 || (uint64_t)n < sl) ? sl : (size_t)n;\n\
         \x20 char* out = (char*)malloc(target + 1);\n\
         \x20 if (!out) abort();\n\
         \x20 if (sl > 0 && s) memcpy(out, s, sl);\n\
         \x20 for (size_t i = sl; i < target; i++) out[i] = fill;\n\
         \x20 out[target] = 0;\n\
         \x20 return out;\n\
         }\n\n",
    );
}

pub(crate) fn emit_intent_str_split_c(out: &mut String) {
    out.push_str(
        "static intent_vec_owned_str intent_str_split(const char* s, const char* delim) INTENT_UNUSED;\n\
         static intent_vec_owned_str intent_str_split(const char* s, const char* delim) {\n\
         \x20 intent_vec_owned_str out;\n\
         \x20 out.data = (char**)0; out.len = 0; out.capacity = 0;\n\
         \x20 if (!s) s = \"\";\n\
         \x20 size_t dl = (delim == 0) ? 0 : strlen(delim);\n\
         \x20 size_t sl = strlen(s);\n\
         \x20 /* Empty delim: one element = dup(s). */\n\
         \x20 if (dl == 0) {\n\
         \x20   char* dup = (char*)malloc(sl + 1);\n\
         \x20   if (!dup) abort();\n\
         \x20   memcpy(dup, s, sl + 1);\n\
         \x20   out.capacity = 4;\n\
         \x20   out.data = (char**)malloc(out.capacity * sizeof(char*));\n\
         \x20   if (!out.data) abort();\n\
         \x20   out.data[0] = dup;\n\
         \x20   out.len = 1;\n\
         \x20   return out;\n\
         \x20 }\n\
         \x20 const char* p = s;\n\
         \x20 while (1) {\n\
         \x20   const char* m = strstr(p, delim);\n\
         \x20   size_t span = m ? (size_t)(m - p) : strlen(p);\n\
         \x20   char* dup = (char*)malloc(span + 1);\n\
         \x20   if (!dup) abort();\n\
         \x20   if (span > 0) memcpy(dup, p, span);\n\
         \x20   dup[span] = 0;\n\
         \x20   /* inline push (no per-type helper dependency). */\n\
         \x20   if (out.len >= out.capacity) {\n\
         \x20     out.capacity = out.capacity ? out.capacity * 2 : 4;\n\
         \x20     out.data = (char**)realloc(out.data, out.capacity * sizeof(char*));\n\
         \x20     if (!out.data) abort();\n\
         \x20   }\n\
         \x20   out.data[out.len++] = dup;\n\
         \x20   if (!m) break;\n\
         \x20   p = m + dl;\n\
         \x20 }\n\
         \x20 return out;\n\
         }\n\n",
    );
}

/// Closure #349: heap-allocating `str_replace`. Returns a
/// fresh OwnedStr where every occurrence of `from` in `s` has
/// been replaced by `to`. Empty `from` is treated as no-op (a
/// matching loop would otherwise diverge). NULL `s` returns a
/// fresh empty string; NULL `from`/`to` are treated as empty.
/// Two-pass: first count occurrences to size the buffer, then
/// walk + copy. Reuses no input bytes — caller can drop the
/// inputs independently.
pub(crate) fn emit_intent_str_replace_c(out: &mut String) {
    out.push_str(
        "static char* intent_str_replace(const char* s, const char* from, const char* to) INTENT_UNUSED;\n\
         static char* intent_str_replace(const char* s, const char* from, const char* to) {\n\
         \x20 if (!s) s = \"\";\n\
         \x20 if (!from) from = \"\";\n\
         \x20 if (!to) to = \"\";\n\
         \x20 size_t fn_len = strlen(from);\n\
         \x20 size_t s_len = strlen(s);\n\
         \x20 if (fn_len == 0) {\n\
         \x20   char* dup = (char*)malloc(s_len + 1);\n\
         \x20   if (!dup) abort();\n\
         \x20   memcpy(dup, s, s_len + 1);\n\
         \x20   return dup;\n\
         \x20 }\n\
         \x20 size_t to_len = strlen(to);\n\
         \x20 /* Pass 1: count non-overlapping matches. */\n\
         \x20 size_t hits = 0;\n\
         \x20 {\n\
         \x20   const char* p = s;\n\
         \x20   while (1) {\n\
         \x20     const char* m = strstr(p, from);\n\
         \x20     if (!m) break;\n\
         \x20     hits++;\n\
         \x20     p = m + fn_len;\n\
         \x20   }\n\
         \x20 }\n\
         \x20 /* New length: original - hits*from + hits*to. */\n\
         \x20 size_t new_len = s_len + hits * to_len - hits * fn_len;\n\
         \x20 char* out_buf = (char*)malloc(new_len + 1);\n\
         \x20 if (!out_buf) abort();\n\
         \x20 /* Pass 2: walk + copy spans + replacements. */\n\
         \x20 const char* src = s;\n\
         \x20 char* dst = out_buf;\n\
         \x20 while (1) {\n\
         \x20   const char* m = strstr(src, from);\n\
         \x20   if (!m) break;\n\
         \x20   size_t span = (size_t)(m - src);\n\
         \x20   if (span > 0) memcpy(dst, src, span);\n\
         \x20   dst += span;\n\
         \x20   if (to_len > 0) memcpy(dst, to, to_len);\n\
         \x20   dst += to_len;\n\
         \x20   src = m + fn_len;\n\
         \x20 }\n\
         \x20 /* Copy the tail after the last match (or all of s). */\n\
         \x20 size_t tail = strlen(src);\n\
         \x20 if (tail > 0) memcpy(dst, src, tail);\n\
         \x20 dst[tail] = 0;\n\
         \x20 return out_buf;\n\
         }\n\n",
    );
}

/// Closure #366: `substring(s: Str, start: i64, len: i64) ->
/// OwnedStr`. Returns a freshly-malloc'd copy of the bytes at
/// `[start, start+len)` in `s`. Negative `start` / `len` are
/// treated as zero; the window is clamped against `strlen(s)`
/// so out-of-bounds reads can't escape the input buffer. NULL
/// `s` is treated as the empty string.
pub(crate) fn emit_intent_substring_c(out: &mut String) {
    out.push_str(
        "static char* intent_substring(const char* s, int64_t start, int64_t len) INTENT_UNUSED;\n\
         static char* intent_substring(const char* s, int64_t start, int64_t len) {\n\
         \x20 size_t sl = s ? strlen(s) : 0;\n\
         \x20 int64_t lo = start < 0 ? 0 : start;\n\
         \x20 int64_t want = len < 0 ? 0 : len;\n\
         \x20 if ((uint64_t)lo > (uint64_t)sl) lo = (int64_t)sl;\n\
         \x20 int64_t remaining = (int64_t)sl - lo;\n\
         \x20 int64_t take = want < remaining ? want : remaining;\n\
         \x20 char* out = (char*)malloc((size_t)take + 1);\n\
         \x20 if (!out) abort();\n\
         \x20 if (take > 0 && s) memcpy(out, s + lo, (size_t)take);\n\
         \x20 out[take] = 0;\n\
         \x20 return out;\n\
         }\n\n",
    );
}

/// Closure #369: `str_to_upper(s) / str_to_lower(s) -> OwnedStr`.
/// ASCII case conversion (non-ASCII bytes passed through
/// unchanged). NULL input produces a fresh empty string. Both
/// helpers share `intent_str_case_c` for code-reuse — the only
/// difference is the per-byte delta sign.
pub(crate) fn emit_intent_str_case_c(out: &mut String) {
    out.push_str(
        "static char* intent_str_to_upper(const char* s) INTENT_UNUSED;\n\
         static char* intent_str_to_upper(const char* s) {\n\
         \x20 size_t sl = s ? strlen(s) : 0;\n\
         \x20 char* out = (char*)malloc(sl + 1);\n\
         \x20 if (!out) abort();\n\
         \x20 for (size_t i = 0; i < sl; ++i) {\n\
         \x20   char c = s[i];\n\
         \x20   out[i] = (c >= 'a' && c <= 'z') ? (char)(c - 32) : c;\n\
         \x20 }\n\
         \x20 out[sl] = 0;\n\
         \x20 return out;\n\
         }\n\
         static char* intent_str_to_lower(const char* s) INTENT_UNUSED;\n\
         static char* intent_str_to_lower(const char* s) {\n\
         \x20 size_t sl = s ? strlen(s) : 0;\n\
         \x20 char* out = (char*)malloc(sl + 1);\n\
         \x20 if (!out) abort();\n\
         \x20 for (size_t i = 0; i < sl; ++i) {\n\
         \x20   char c = s[i];\n\
         \x20   out[i] = (c >= 'A' && c <= 'Z') ? (char)(c + 32) : c;\n\
         \x20 }\n\
         \x20 out[sl] = 0;\n\
         \x20 return out;\n\
         }\n\n",
    );
}

/// Closure #368: `str_repeat(s, n) -> OwnedStr`. Concatenates
/// `s` with itself `n` times. Negative `n` produces empty;
/// NULL `s` treated as empty. Length check guards against
/// overflow (`n * strlen(s)` must fit in size_t).
pub(crate) fn emit_intent_str_repeat_c(out: &mut String) {
    out.push_str(
        "static char* intent_str_repeat(const char* s, int64_t n) INTENT_UNUSED;\n\
         static char* intent_str_repeat(const char* s, int64_t n) {\n\
         \x20 size_t sl = s ? strlen(s) : 0;\n\
         \x20 if (n <= 0 || sl == 0) {\n\
         \x20   char* e = (char*)malloc(1);\n\
         \x20   if (!e) abort();\n\
         \x20   e[0] = 0;\n\
         \x20   return e;\n\
         \x20 }\n\
         \x20 if ((uint64_t)n > (SIZE_MAX - 1) / sl) abort();\n\
         \x20 size_t total = sl * (size_t)n;\n\
         \x20 char* out = (char*)malloc(total + 1);\n\
         \x20 if (!out) abort();\n\
         \x20 char* p = out;\n\
         \x20 for (int64_t i = 0; i < n; ++i) {\n\
         \x20   memcpy(p, s, sl);\n\
         \x20   p += sl;\n\
         \x20 }\n\
         \x20 out[total] = 0;\n\
         \x20 return out;\n\
         }\n\n",
    );
}

/// Closure #348: heap-allocating `str_trim`. Returns a fresh
/// OwnedStr with leading and trailing ASCII whitespace (the
/// standard `isspace()` set: space, \\t, \\n, \\v, \\f, \\r)
/// stripped. The output buffer is always freshly malloc'd so
/// the OwnedStr scope-exit Drop is well-defined; even when no
/// trimming is needed and even on an empty input, we still
/// hand back a heap-owned copy.
pub(crate) fn emit_intent_str_trim_c(out: &mut String) {
    out.push_str(
        "static char* intent_str_trim(const char* s) INTENT_UNUSED;\n\
         static char* intent_str_trim(const char* s) {\n\
         \x20 if (!s) {\n\
         \x20   char* e = (char*)malloc(1);\n\
         \x20   if (!e) abort();\n\
         \x20   e[0] = 0;\n\
         \x20   return e;\n\
         \x20 }\n\
         \x20 const char* lo = s;\n\
         \x20 while (*lo == ' ' || *lo == '\\t' || *lo == '\\n' || *lo == '\\v' || *lo == '\\f' || *lo == '\\r') lo++;\n\
         \x20 size_t n = strlen(lo);\n\
         \x20 while (n > 0) {\n\
         \x20   char c = lo[n - 1];\n\
         \x20   if (c == ' ' || c == '\\t' || c == '\\n' || c == '\\v' || c == '\\f' || c == '\\r') n--;\n\
         \x20   else break;\n\
         \x20 }\n\
         \x20 char* out = (char*)malloc(n + 1);\n\
         \x20 if (!out) abort();\n\
         \x20 if (n > 0) memcpy(out, lo, n);\n\
         \x20 out[n] = 0;\n\
         \x20 return out;\n\
         }\n\n",
    );
}

pub(crate) fn vec_c_struct(element: &Type) -> String {
    format!("intent_vec_{}", element_tag(element))
}

/// Build a C-identifier-safe tag for an element type. The tag
/// is used as the suffix on per-type helper names (e.g. `vec_int64_t`,
/// `vec_vec_int64_t`, `vec_arr4_int64_t`). Composable so that
/// nested aggregates (`Vec<Vec<i64>>`, `Vec<[i64; 4]>`) get
/// distinct, deterministic identifiers — refines #7 from
/// STATUS.md (was: returned `"/*_vec_*/"` for any `Vec<_>`
/// element, collapsing every nested Vec type to the same tag).
pub(crate) fn element_tag(element: &Type) -> String {
    match element {
        Type::Vec(inner) => format!("vec_{}", element_tag(inner)),
        Type::Array { element: inner, length } => {
            format!("arr{}_{}", length, element_tag(inner))
        }
        // Nominal types route through their per-name C
        // struct spelling so `Vec<Point>` becomes
        // `intent_vec_Struct_Point` rather than the
        // opaque `/*_struct_*/` placeholder. T1.2 +
        // Vec<Struct> support.
        Type::Struct(name) => struct_c_name(name),
        // Payloaded enums lower to the `Enum_<Name>`
        // tagged-union struct (see ENUM_PAYLOAD_REGISTRY).
        // For `Vec<Msg>` the per-shape Vec typedef must
        // reference that struct, not the int32_t tag
        // — closure #151 (was emitting `intent_vec_int32_t`
        // for any enum element and then trying to store
        // `Enum_<Name>` struct literals into int32_t
        // slots, failing at cc). Tag-only enums keep the
        // int32_t spelling via the fallback below since
        // they don't appear in ENUM_PAYLOAD_REGISTRY.
        Type::Enum(name)
            if ENUM_PAYLOAD_REGISTRY.with(|r| r.borrow().contains_key(name)) =>
        {
            enum_c_name(name)
        }
        Type::Tuple(elements) => tuple_c_struct(elements),
        // `Str` / `OwnedStr` both spell as a `char*` /
        // `const char*` in C — neither is a valid C
        // identifier suffix. Spell them as `str` /
        // `owned_str` so `Vec<OwnedStr>` becomes
        // `intent_vec_owned_str` and friends. Closure
        // #136 (was emitting `intent_vec_char*` and
        // failing to compile).
        Type::Str => "str".to_string(),
        Type::OwnedStr => "owned_str".to_string(),
        // Closure #211: `Atomic<T>` / `Channel<T, N>` must
        // include the element type (and capacity) in the
        // element-tag so per-shape Vec typedefs stay
        // distinct. The c_leaf_type fallback returned the
        // hardcoded `_Atomic int64_t` / `intent_channel_int64_t_16`
        // strings for any Atomic/Channel, collapsing every
        // `Vec<Atomic<T>>` (or `Vec<Channel<T,N>>`) to the
        // same typedef name. With two distinct (T, …) shapes
        // in the same program, the second `vec()` call used
        // the first's typedef which had the wrong element
        // type — ASan-detected stack-buffer-overflow on
        // memcpy when widths differed (u32 vs u8).
        Type::Atomic(element) => format!("atomic_{}", element_tag(element)),
        Type::Channel(element, capacity) => {
            format!("channel_{}_{}", element_tag(element), capacity)
        }
        // Closure #214: `fn(T1, T2) -> R` falls through to
        // `c_leaf_type(FnPtr) = "void*"`, and the `*` in the
        // typedef name (`intent_vec_void*`) breaks C parsing.
        // Spell it as `fnptr` — distinct from any scalar
        // type, identifier-safe. All fn-ptrs share the same
        // C representation (`void*` cast in/out for indirect
        // calls), so a single per-element-tag typedef is
        // correct regardless of parameter/return types.
        Type::FnPtr(_, _) => "fnptr".to_string(),
        // Phase 1.2 (2026-06-07): `dyn Iface` must include the
        // Iface name so two struct fields holding `Vec<dyn A>`
        // and `Vec<dyn B>` get DISTINCT bundle typedefs.
        // c_leaf_type can only return &'static str so it
        // returned the generic "intent_dyn" for every Iface;
        // every Vec<dyn …> then collapsed to one bundle name
        // `intent_vec_intent_dyn` that referenced an undefined
        // type. Per-Iface naming aligns Vec<dyn …> with the
        // per-Iface fat-pointer typedef `intent_dyn_<Iface>`
        // emitted by `emit_dyn_iface_typedefs`. Closes L8.
        Type::Object(iface_name) => format!("dyn_{}", iface_name),
        _ => c_leaf_type(element).replace(' ', "_"),
    }
}

pub(crate) fn vec_helper(element: &Type, op: &str) -> String {
    format!("{}__{}", vec_c_struct(element), op)
}

/// Storage struct name for `Channel<T, N>` in the C backend.
/// Combines the element's C spelling (sanitized) with the
/// capacity so each (T, N) used in the program gets its own
/// struct + runtime helpers. e.g. `Channel<i32, 32>` →
/// `intent_channel_int32_t_32`.
pub(crate) fn c_channel_storage(element: &Type, capacity: u64) -> String {
    format!("intent_channel_{}_{}", element_tag(element), capacity)
}

/// Per-(T, N) channel helper name: e.g. `_send` / `_recv` /
/// `_new`.
pub(crate) fn c_channel_helper(element: &Type, capacity: u64, op: &str) -> String {
    format!("{}_{}", c_channel_storage(element, capacity), op)
}

/// Recover the `(T, N)` shape from a `&Channel<T, N>` /
/// `&mut Channel<T, N>` operand type. Shared with SSA-C.
pub(crate) fn channel_inner_from_ref_pub(ty: &Type) -> (Type, u64) {
    channel_inner_from_ref(ty)
}

/// Emit one per-(T, N) channel bundle (struct + helpers).
/// Shared with SSA-C.
pub(crate) fn emit_channel_bundle_pub(
    element: &Type,
    capacity: u64,
    out: &mut String,
) {
    emit_channel_bundle(element, capacity, out)
}

/// Collect every unique `(T, N)` `Channel` spec reachable
/// from `ty`. `seen` dedups by the channel's struct name so
/// nested types (`Vec<Channel<i64, 8>>`, `Ref<Channel<…>>`)
/// don't emit the same bundle twice. Used during preamble
/// emission to generate exactly the per-(T, N) runtime
/// helpers the program references.
pub(crate) fn collect_channel_specs(
    ty: &Type,
    seen: &mut BTreeSet<String>,
    out: &mut Vec<(Type, u64)>,
) {
    match ty {
        Type::Channel(element, capacity) => {
            let key = c_channel_storage(element, *capacity);
            if seen.insert(key) {
                out.push(((**element).clone(), *capacity));
            }
            collect_channel_specs(element, seen, out);
        }
        Type::Vec(element) | Type::Atomic(element) | Type::Mutex(element) | Type::Guard(element) => {
            collect_channel_specs(element, seen, out);
        }
        Type::Array { element, .. } => collect_channel_specs(element, seen, out),
        Type::Ref(inner) | Type::RefMut(inner) => collect_channel_specs(inner, seen, out),
        _ => {}
    }
}

pub(crate) fn collect_channel_specs_in_stmt(
    stmt: &TypedStmt,
    seen: &mut BTreeSet<String>,
    out: &mut Vec<(Type, u64)>,
) {
    match stmt {
        TypedStmt::Let { ty, expr, .. } | TypedStmt::Reassign { ty, expr, .. } => {
            collect_channel_specs(ty, seen, out);
            collect_channel_specs_in_expr(expr, seen, out);
        }
        TypedStmt::Drop { ty, .. } => collect_channel_specs(ty, seen, out),
        TypedStmt::Discard { expr } => collect_channel_specs_in_expr(expr, seen, out),
        TypedStmt::Return { expr }
        | TypedStmt::Assert { expr, .. }
        | TypedStmt::Prove { expr } => collect_channel_specs_in_expr(expr, seen, out),
        TypedStmt::Print { items } => {
            for it in items {
                if let crate::ir::TypedPrintItem::Expr(e) = it {
                    collect_channel_specs_in_expr(e, seen, out);
                }
            }
        }
        TypedStmt::If { cond, then_body, else_body } => {
            collect_channel_specs_in_expr(cond, seen, out);
            for s in then_body {
                collect_channel_specs_in_stmt(s, seen, out);
            }
            for s in else_body {
                collect_channel_specs_in_stmt(s, seen, out);
            }
        }
        TypedStmt::While { cond, body } => {
            collect_channel_specs_in_expr(cond, seen, out);
            for s in body {
                collect_channel_specs_in_stmt(s, seen, out);
            }
        }
        TypedStmt::Break | TypedStmt::Continue => {}
        TypedStmt::IndexAssign { index, value, base_ty, .. } => {
            collect_channel_specs(base_ty, seen, out);
            collect_channel_specs_in_expr(index, seen, out);
            collect_channel_specs_in_expr(value, seen, out);
        }
        TypedStmt::FieldAssign { object, value, .. } => {
            collect_channel_specs_in_expr(object, seen, out);
            collect_channel_specs_in_expr(value, seen, out);
        }
        TypedStmt::For { start, end, body, .. } => {
            collect_channel_specs_in_expr(start, seen, out);
            collect_channel_specs_in_expr(end, seen, out);
            for s in body {
                collect_channel_specs_in_stmt(s, seen, out);
            }
        }
        TypedStmt::ForIter { element_ty, collection_ty, body, .. } => {
            collect_channel_specs(element_ty, seen, out);
            collect_channel_specs(collection_ty, seen, out);
            for s in body {
                collect_channel_specs_in_stmt(s, seen, out);
            }
        }
        TypedStmt::TaskSpawn { body, .. } | TypedStmt::UnsafeBlock { body, .. } => {
            for s in body {
                collect_channel_specs_in_stmt(s, seen, out);
            }
        }
        TypedStmt::TaskJoin { .. } => {}
    }
}

pub(crate) fn collect_channel_specs_in_expr(
    expr: &TypedExpr,
    seen: &mut BTreeSet<String>,
    out: &mut Vec<(Type, u64)>,
) {
    collect_channel_specs(&expr.ty, seen, out);
    match &expr.kind {
        TypedExprKind::Unary { expr, .. } => collect_channel_specs_in_expr(expr, seen, out),
        TypedExprKind::Binary { left, right, .. } => {
            collect_channel_specs_in_expr(left, seen, out);
            collect_channel_specs_in_expr(right, seen, out);
        }
        TypedExprKind::Call { args, .. } | TypedExprKind::ArrayLit { elements: args } => {
            for arg in args {
                collect_channel_specs_in_expr(arg, seen, out);
            }
        }
        TypedExprKind::Cast { expr, .. } => collect_channel_specs_in_expr(expr, seen, out),
        TypedExprKind::Index { array, index, .. } => {
            collect_channel_specs_in_expr(array, seen, out);
            collect_channel_specs_in_expr(index, seen, out);
        }
        TypedExprKind::Len { array, .. } => collect_channel_specs_in_expr(array, seen, out),
        _ => {}
    }
}

pub(crate) fn emit_vec_bundle(element: &Type, out: &mut String) {
    let struct_name = vec_c_struct(element);
    // Element's full C type spelling. For primitive scalars
    // this is `c_leaf_type` (e.g. `int64_t`). For aggregates
    // (`Vec<T>`, `Array<T, N>`) we route through `c_type_name`
    // / `c_array_type_name` so a `Vec<Vec<i64>>` element spells
    // as `intent_vec_int64_t` (the inner struct typedef
    // emitted earlier in the bundle list). Refines #7 — was
    // emitting `"/* vec */"` for any Vec-element, which the C
    // compiler then choked on.
    let c_element = c_element_storage(element);
    let element_is_copy = element.is_copy();
    // Fixed-size array elements need memcpy-based slot
    // writes (C forbids `arr1 = arr2` via `=`). Phase 2c.
    let element_is_array = matches!(element, Type::Array { .. });

    out.push_str(&format!(
        "typedef struct {{ {ct}* data; uint64_t len; uint64_t capacity; }} {sn};\n",
        ct = c_element,
        sn = struct_name
    ));

    out.push_str(&format!(
        "static INTENT_UNUSED {sn} {sn}__from(uint64_t n, const {ct}* init) {{\
\n    {sn} v;\
\n    v.data = ({ct}*)malloc((n == 0 ? 1 : n) * sizeof({ct}));\
\n    if (!v.data) abort();\
\n    if (n > 0) memcpy(v.data, init, n * sizeof({ct}));\
\n    v.len = n;\
\n    v.capacity = n == 0 ? 1 : n;\
\n    return v;\
\n}}\n",
        sn = struct_name,
        ct = c_element
    ));

    // Array elements need memcpy; struct/scalar elements
    // assign directly. Phase 2c (#7).
    let push_store = if element_is_array {
        format!(
            "    memcpy(xs.data[xs.len], v, sizeof({}));\
\n    xs.len++;",
            c_element,
        )
    } else {
        "    xs.data[xs.len++] = v;".to_string()
    };
    out.push_str(&format!(
        "static INTENT_UNUSED {sn} {sn}__push({sn} xs, {ct} v) {{\
\n    if (xs.len >= xs.capacity) {{\
\n        xs.capacity = xs.capacity ? xs.capacity * 2 : 1;\
\n        xs.data = ({ct}*)realloc(xs.data, xs.capacity * sizeof({ct}));\
\n        if (!xs.data) abort();\
\n    }}\
\n{store}\
\n    return xs;\
\n}}\n",
        sn = struct_name,
        ct = c_element,
        store = push_store,
    ));

    // In-place push for `push(mut ref xs, v)` — operates on a
    // pointer to the Vec struct. Used when the Vec is owned by
    // another binding (e.g. a struct field) and the caller
    // doesn't want to consume + reassign. T1.2 phase 2b
    // follow-up.
    let push_mut_store = if element_is_array {
        format!(
            "    memcpy(xs->data[xs->len], v, sizeof({}));\n    xs->len++;",
            c_element,
        )
    } else {
        "    xs->data[xs->len++] = v;".to_string()
    };
    out.push_str(&format!(
        "static INTENT_UNUSED int64_t {sn}__push_mut({sn}* xs, {ct} v) {{\
\n    if (xs->len >= xs->capacity) {{\
\n        xs->capacity = xs->capacity ? xs->capacity * 2 : 1;\
\n        xs->data = ({ct}*)realloc(xs->data, xs->capacity * sizeof({ct}));\
\n        if (!xs->data) abort();\
\n    }}\
\n{store}\
\n    return (int64_t)xs->len;\
\n}}\n",
        sn = struct_name,
        ct = c_element,
        store = push_mut_store,
    ));

    // Closure #219: in-place `pop(mut ref xs) -> T` — abort on
    // empty, otherwise decrement len and return the last
    // element by-move. For non-Copy element types (OwnedStr,
    // Vec<U>, Struct with owning fields), the returned value
    // carries ownership of the slot's heap; the Vec's
    // scope-exit `__free` walks elements based on the post-
    // pop len so the moved-out slot won't be re-freed.
    // For fixed-size array element types (`[T; N]`), C
    // forbids returning a bare array by value — the helper
    // returns a struct wrapping the array would complicate
    // codegen significantly. Defer Vec<[T;N]> pop to a
    // follow-up; for now reject via the checker if it ever
    // surfaces. Most callers don't need that shape.
    if !element_is_array {
        out.push_str(&format!(
            "static INTENT_UNUSED {ct} {sn}__pop_mut({sn}* xs) {{\
\n    if (xs->len == 0) {{\
\n        fprintf(stderr, \"pop on empty Vec\\n\");\
\n        abort();\
\n    }}\
\n    xs->len--;\
\n    return xs->data[xs->len];\
\n}}\n",
            sn = struct_name,
            ct = c_element,
        ));
    }

    // Data-structures roadmap Level 1 mutators:
    // swap_remove / insert / clear. Skipped for array elements
    // (matches pop's gate; the checker also rejects them).
    if !element_is_array {
        // swap_remove(i): tmp = xs->data[i]; xs->data[i] =
        // xs->data[len-1]; len--; return tmp. O(1) — order
        // NOT preserved.
        out.push_str(&format!(
            "static INTENT_UNUSED {ct} {sn}__swap_remove({sn}* xs, uint64_t i) {{\
\n    if (i >= xs->len) {{ fprintf(stderr, \"swap_remove: index out of bounds\\n\"); abort(); }}\
\n    {ct} tmp = xs->data[i];\
\n    xs->len--;\
\n    if (i < xs->len) {{ xs->data[i] = xs->data[xs->len]; }}\
\n    return tmp;\
\n}}\n",
            sn = struct_name,
            ct = c_element,
        ));
        // insert(i, v): grow if needed; memmove slots i.. right
        // by one; place v at slot i. v is consumed (single-
        // owner transfer into the slot).
        out.push_str(&format!(
            "static INTENT_UNUSED int64_t {sn}__insert({sn}* xs, uint64_t i, {ct} v) {{\
\n    if (i > xs->len) {{ fprintf(stderr, \"insert: index out of bounds\\n\"); abort(); }}\
\n    if (xs->len >= xs->capacity) {{\
\n        xs->capacity = xs->capacity ? xs->capacity * 2 : 1;\
\n        xs->data = ({ct}*)realloc(xs->data, xs->capacity * sizeof({ct}));\
\n        if (!xs->data) abort();\
\n    }}\
\n    if (i < xs->len) {{ memmove(xs->data + i + 1, xs->data + i, (xs->len - i) * sizeof({ct})); }}\
\n    xs->data[i] = v;\
\n    xs->len++;\
\n    return (int64_t)xs->len;\
\n}}\n",
            sn = struct_name,
            ct = c_element,
        ));
        // clear: walk each live slot, drop its owning content
        // (when non-Copy), set len=0. Buffer + capacity stay.
        let elem_drop_walk = if element_is_copy {
            String::new()
        } else {
            // Reuse the same per-element drop spelling the
            // __free helper uses, but loop only over live
            // slots and skip the final free(xs->data).
            let one = c_element_drop_old("xs->data[__ci]", element);
            format!(
                "    for (uint64_t __ci = 0; __ci < xs->len; __ci++) {{\
\n{}\n    }}\n",
                one,
            )
        };
        out.push_str(&format!(
            "static INTENT_UNUSED int64_t {sn}__clear({sn}* xs) {{\
\n{drop}    xs->len = 0;\
\n    return 0;\
\n}}\n",
            sn = struct_name,
            drop = elem_drop_walk,
        ));
    }

    // Data-structures roadmap Level 1: `reverse(mut ref xs)`.
    // Two-pointer in-place swap; works for any Copy element
    // type. Array-element slots use memcpy through a scratch
    // buffer; scalar slots use the natural three-temp swap.
    if element.is_copy() {
        let swap_body = if element_is_array {
            format!(
                "        {ct} tmp;\n        memcpy(tmp, xs->data[i], sizeof({ct}));\n        memcpy(xs->data[i], xs->data[j], sizeof({ct}));\n        memcpy(xs->data[j], tmp, sizeof({ct}));",
                ct = c_element,
            )
        } else {
            format!(
                "        {ct} tmp = xs->data[i];\n        xs->data[i] = xs->data[j];\n        xs->data[j] = tmp;",
                ct = c_element,
            )
        };
        out.push_str(&format!(
            "static INTENT_UNUSED int64_t {sn}__reverse({sn}* xs) {{\
\n    if (xs->len < 2) {{ return 0; }}\
\n    uint64_t i = 0;\
\n    uint64_t j = xs->len - 1;\
\n    while (i < j) {{\
\n{body}\
\n        i++;\
\n        j--;\
\n    }}\
\n    return 0;\
\n}}\n",
            sn = struct_name,
            body = swap_body,
        ));
    }

    // Data-structures roadmap Level 1: in-place `sort` /
    // `sort_by` on `Vec<i64>`. v1 restricts to i64 — the
    // runtime helper is monomorphized over that width. The
    // checker rejects non-i64 element types at the call site
    // so this emit gate matches the surface. The comparator
    // takes i64 values directly (i64 is Copy); strcmp
    // convention: negative / zero / positive.
    if matches!(element, Type::I64) {
        out.push_str(&format!(
            "typedef int64_t (*{sn}__cmp_fn)(int64_t, int64_t);\n",
            sn = struct_name,
        ));
        out.push_str(&format!(
            "static INTENT_UNUSED int64_t {sn}__cmp_ascending(int64_t a, int64_t b) {{\
\n    return (a > b) - (a < b);\
\n}}\n",
            sn = struct_name,
        ));
        // Hoare-partition quicksort with insertion-sort cutoff
        // (N < 16).
        out.push_str(&format!(
            "static INTENT_UNUSED void {sn}__qsort_impl(int64_t* a, int64_t lo, int64_t hi, {sn}__cmp_fn cmp) {{\
\n    while (lo < hi) {{\
\n        if (hi - lo < 16) {{\
\n            for (int64_t i = lo + 1; i <= hi; i++) {{\
\n                int64_t key = a[i];\
\n                int64_t j = i - 1;\
\n                while (j >= lo && cmp(a[j], key) > 0) {{\
\n                    a[j + 1] = a[j];\
\n                    j--;\
\n                }}\
\n                a[j + 1] = key;\
\n            }}\
\n            return;\
\n        }}\
\n        int64_t mid = lo + (hi - lo) / 2;\
\n        int64_t pivot = a[mid];\
\n        int64_t i = lo - 1;\
\n        int64_t j = hi + 1;\
\n        for (;;) {{\
\n            do {{ i++; }} while (cmp(a[i], pivot) < 0);\
\n            do {{ j--; }} while (cmp(a[j], pivot) > 0);\
\n            if (i >= j) break;\
\n            int64_t tmp = a[i]; a[i] = a[j]; a[j] = tmp;\
\n        }}\
\n        /* Tail-recurse on the larger side to bound stack depth. */\
\n        if (j - lo < hi - (j + 1)) {{\
\n            {sn}__qsort_impl(a, lo, j, cmp);\
\n            lo = j + 1;\
\n        }} else {{\
\n            {sn}__qsort_impl(a, j + 1, hi, cmp);\
\n            hi = j;\
\n        }}\
\n    }}\
\n}}\n",
            sn = struct_name,
        ));
        out.push_str(&format!(
            "static INTENT_UNUSED int64_t {sn}__sort({sn}* xs) {{\
\n    if (xs->len > 1) {{\
\n        {sn}__qsort_impl(xs->data, 0, (int64_t)xs->len - 1, {sn}__cmp_ascending);\
\n    }}\
\n    return 0;\
\n}}\n",
            sn = struct_name,
        ));
        out.push_str(&format!(
            "static INTENT_UNUSED int64_t {sn}__sort_by({sn}* xs, {sn}__cmp_fn cmp) {{\
\n    if (xs->len > 1) {{\
\n        {sn}__qsort_impl(xs->data, 0, (int64_t)xs->len - 1, cmp);\
\n    }}\
\n    return 0;\
\n}}\n",
            sn = struct_name,
        ));
        // Data-structures roadmap Level 3 — eager iterator
        // combinators (closure #309). v1 Vec<i64> only; both
        // helpers borrow xs and take an explicit fn-ptr.
        // map allocates a fresh result Vec (caller owns + drops);
        // fold returns a scalar. The cmp_fn typedef above has
        // the same signature as fold's combiner so we reuse it.
        out.push_str(&format!(
            "typedef int64_t (*{sn}__map_fn)(int64_t);\n",
            sn = struct_name,
        ));
        out.push_str(&format!(
            "static INTENT_UNUSED {sn} {sn}__map(const {sn}* xs, {sn}__map_fn f) {{\
\n    {sn} out;\
\n    out.len = xs->len;\
\n    out.capacity = xs->len;\
\n    if (xs->len == 0) {{ out.data = (int64_t*)0; return out; }}\
\n    out.data = (int64_t*)malloc(xs->len * sizeof(int64_t));\
\n    if (!out.data) abort();\
\n    for (uint64_t i = 0; i < xs->len; i++) {{\
\n        out.data[i] = f(xs->data[i]);\
\n    }}\
\n    return out;\
\n}}\n",
            sn = struct_name,
        ));
        out.push_str(&format!(
            "static INTENT_UNUSED int64_t {sn}__fold(const {sn}* xs, int64_t init, {sn}__cmp_fn g) {{\
\n    int64_t acc = init;\
\n    for (uint64_t i = 0; i < xs->len; i++) {{\
\n        acc = g(acc, xs->data[i]);\
\n    }}\
\n    return acc;\
\n}}\n",
            sn = struct_name,
        ));
        // Forward `__pred_fn` typedef so the fused combinators
        // below can refer to it before `__filter`'s emission
        // (which historically declared it). Closure #317.
        out.push_str(&format!(
            "typedef bool (*{sn}__pred_fn)(int64_t);\n",
            sn = struct_name,
        ));
        // vec_map_fold (closure #316): fused map-then-fold,
        // single pass, no intermediate Vec allocation.
        // Signature `int64_t (*)(int64_t)` for the mapper +
        // existing `__cmp_fn` for the combiner.
        out.push_str(&format!(
            "static INTENT_UNUSED int64_t {sn}__map_fold(const {sn}* xs, int64_t init, {sn}__map_fn f, {sn}__cmp_fn g) {{\
\n    int64_t acc = init;\
\n    for (uint64_t i = 0; i < xs->len; i++) {{\
\n        acc = g(acc, f(xs->data[i]));\
\n    }}\
\n    return acc;\
\n}}\n",
            sn = struct_name,
        ));
        // Rest of the fused combinator family (closure #317).
        // All single-pass except __map_filter which is two-pass
        // (count, allocate, fill — mirrors __filter's shape so
        // the output Vec has zero wasted capacity).
        out.push_str(&format!(
            "static INTENT_UNUSED int64_t {sn}__filter_fold(const {sn}* xs, int64_t init, {sn}__pred_fn p, {sn}__cmp_fn g) {{\
\n    int64_t acc = init;\
\n    for (uint64_t i = 0; i < xs->len; i++) {{\
\n        if (p(xs->data[i])) acc = g(acc, xs->data[i]);\
\n    }}\
\n    return acc;\
\n}}\n",
            sn = struct_name,
        ));
        out.push_str(&format!(
            "static INTENT_UNUSED {sn} {sn}__map_filter(const {sn}* xs, {sn}__map_fn f, {sn}__pred_fn p) {{\
\n    {sn} out;\
\n    uint64_t hits = 0;\
\n    for (uint64_t i = 0; i < xs->len; i++) {{\
\n        if (p(f(xs->data[i]))) hits++;\
\n    }}\
\n    out.len = hits;\
\n    out.capacity = hits;\
\n    if (hits == 0) {{ out.data = (int64_t*)0; return out; }}\
\n    out.data = (int64_t*)malloc(hits * sizeof(int64_t));\
\n    if (!out.data) abort();\
\n    uint64_t w = 0;\
\n    for (uint64_t i = 0; i < xs->len; i++) {{\
\n        int64_t mapped = f(xs->data[i]);\
\n        if (p(mapped)) out.data[w++] = mapped;\
\n    }}\
\n    return out;\
\n}}\n",
            sn = struct_name,
        ));
        out.push_str(&format!(
            "static INTENT_UNUSED int64_t {sn}__map_filter_fold(const {sn}* xs, int64_t init, {sn}__map_fn f, {sn}__pred_fn p, {sn}__cmp_fn g) {{\
\n    int64_t acc = init;\
\n    for (uint64_t i = 0; i < xs->len; i++) {{\
\n        int64_t mapped = f(xs->data[i]);\
\n        if (p(mapped)) acc = g(acc, mapped);\
\n    }}\
\n    return acc;\
\n}}\n",
            sn = struct_name,
        ));
        // vec_chain (closure #324): concatenate two Vec<i64>s
        // into a fresh result. Output capacity = sum of input
        // lengths, exact allocation.
        out.push_str(&format!(
            "static INTENT_UNUSED {sn} {sn}__chain(const {sn}* xs, const {sn}* ys) {{\
\n    {sn} out;\
\n    uint64_t total = xs->len + ys->len;\
\n    out.len = total;\
\n    out.capacity = total;\
\n    if (total == 0) {{ out.data = (int64_t*)0; return out; }}\
\n    out.data = (int64_t*)malloc(total * sizeof(int64_t));\
\n    if (!out.data) abort();\
\n    if (xs->len) memcpy(out.data, xs->data, xs->len * sizeof(int64_t));\
\n    if (ys->len) memcpy(out.data + xs->len, ys->data, ys->len * sizeof(int64_t));\
\n    return out;\
\n}}\n",
            sn = struct_name,
        ));
        // vec_sum / vec_product / vec_min / vec_max /
        // vec_count / vec_any / vec_all (closure #322): single-
        // pass reductions with fixed kernels.
        out.push_str(&format!(
            "static INTENT_UNUSED int64_t {sn}__sum(const {sn}* xs) {{\
\n    int64_t acc = 0;\
\n    for (uint64_t i = 0; i < xs->len; i++) acc += xs->data[i];\
\n    return acc;\
\n}}\n",
            sn = struct_name,
        ));
        out.push_str(&format!(
            "static INTENT_UNUSED int64_t {sn}__product(const {sn}* xs) {{\
\n    int64_t acc = 1;\
\n    for (uint64_t i = 0; i < xs->len; i++) acc *= xs->data[i];\
\n    return acc;\
\n}}\n",
            sn = struct_name,
        ));
        out.push_str(&format!(
            "static INTENT_UNUSED int64_t {sn}__min(const {sn}* xs, int64_t def) {{\
\n    if (xs->len == 0) return def;\
\n    int64_t m = xs->data[0];\
\n    for (uint64_t i = 1; i < xs->len; i++) if (xs->data[i] < m) m = xs->data[i];\
\n    return m;\
\n}}\n",
            sn = struct_name,
        ));
        out.push_str(&format!(
            "static INTENT_UNUSED int64_t {sn}__max(const {sn}* xs, int64_t def) {{\
\n    if (xs->len == 0) return def;\
\n    int64_t m = xs->data[0];\
\n    for (uint64_t i = 1; i < xs->len; i++) if (xs->data[i] > m) m = xs->data[i];\
\n    return m;\
\n}}\n",
            sn = struct_name,
        ));
        out.push_str(&format!(
            "static INTENT_UNUSED int64_t {sn}__argmin(const {sn}* xs, int64_t def) {{\
\n    if (xs->len == 0) return def;\
\n    int64_t mv = xs->data[0]; int64_t mi = 0;\
\n    for (uint64_t i = 1; i < xs->len; i++) if (xs->data[i] < mv) {{ mv = xs->data[i]; mi = (int64_t)i; }}\
\n    return mi;\
\n}}\n",
            sn = struct_name,
        ));
        out.push_str(&format!(
            "static INTENT_UNUSED int64_t {sn}__argmax(const {sn}* xs, int64_t def) {{\
\n    if (xs->len == 0) return def;\
\n    int64_t mv = xs->data[0]; int64_t mi = 0;\
\n    for (uint64_t i = 1; i < xs->len; i++) if (xs->data[i] > mv) {{ mv = xs->data[i]; mi = (int64_t)i; }}\
\n    return mi;\
\n}}\n",
            sn = struct_name,
        ));
        out.push_str(&format!(
            "static INTENT_UNUSED int64_t {sn}__count_value(const {sn}* xs, int64_t v) {{\
\n    int64_t c = 0;\
\n    for (uint64_t i = 0; i < xs->len; i++) if (xs->data[i] == v) c++;\
\n    return c;\
\n}}\n",
            sn = struct_name,
        ));
        out.push_str(&format!(
            "static INTENT_UNUSED int64_t {sn}__index_of_value(const {sn}* xs, int64_t v) {{\
\n    for (uint64_t i = 0; i < xs->len; i++) if (xs->data[i] == v) return (int64_t)i;\
\n    return -1;\
\n}}\n",
            sn = struct_name,
        ));
        out.push_str(&format!(
            "static INTENT_UNUSED int64_t {sn}__last_index_of_value(const {sn}* xs, int64_t v) {{\
\n    for (int64_t i = (int64_t)xs->len - 1; i >= 0; i--) if (xs->data[i] == v) return i;\
\n    return -1;\
\n}}\n",
            sn = struct_name,
        ));
        out.push_str(&format!(
            "static INTENT_UNUSED int64_t {sn}__count(const {sn}* xs, {sn}__pred_fn p) {{\
\n    int64_t c = 0;\
\n    for (uint64_t i = 0; i < xs->len; i++) if (p(xs->data[i])) c++;\
\n    return c;\
\n}}\n",
            sn = struct_name,
        ));
        out.push_str(&format!(
            "static INTENT_UNUSED bool {sn}__any(const {sn}* xs, {sn}__pred_fn p) {{\
\n    for (uint64_t i = 0; i < xs->len; i++) if (p(xs->data[i])) return true;\
\n    return false;\
\n}}\n",
            sn = struct_name,
        ));
        out.push_str(&format!(
            "static INTENT_UNUSED bool {sn}__all(const {sn}* xs, {sn}__pred_fn p) {{\
\n    for (uint64_t i = 0; i < xs->len; i++) if (!p(xs->data[i])) return false;\
\n    return true;\
\n}}\n",
            sn = struct_name,
        ));
        // vec_take / vec_drop (closure #313): eager slicing.
        // take returns the first min(n, len) elements; drop
        // returns the rest. Negative n clamps to 0. The result
        // Vec is freshly allocated and the caller owns it.
        out.push_str(&format!(
            "static INTENT_UNUSED {sn} {sn}__take(const {sn}* xs, int64_t n) {{\
\n    {sn} out;\
\n    int64_t take = n < 0 ? 0 : n;\
\n    if ((uint64_t)take > xs->len) take = (int64_t)xs->len;\
\n    out.len = (uint64_t)take;\
\n    out.capacity = (uint64_t)take;\
\n    if (take == 0) {{ out.data = (int64_t*)0; return out; }}\
\n    out.data = (int64_t*)malloc((uint64_t)take * sizeof(int64_t));\
\n    if (!out.data) abort();\
\n    memcpy(out.data, xs->data, (uint64_t)take * sizeof(int64_t));\
\n    return out;\
\n}}\n",
            sn = struct_name,
        ));
        out.push_str(&format!(
            "static INTENT_UNUSED {sn} {sn}__drop(const {sn}* xs, int64_t n) {{\
\n    {sn} out;\
\n    int64_t drop = n < 0 ? 0 : n;\
\n    if ((uint64_t)drop > xs->len) drop = (int64_t)xs->len;\
\n    uint64_t kept = xs->len - (uint64_t)drop;\
\n    out.len = kept;\
\n    out.capacity = kept;\
\n    if (kept == 0) {{ out.data = (int64_t*)0; return out; }}\
\n    out.data = (int64_t*)malloc(kept * sizeof(int64_t));\
\n    if (!out.data) abort();\
\n    memcpy(out.data, xs->data + drop, kept * sizeof(int64_t));\
\n    return out;\
\n}}\n",
            sn = struct_name,
        ));
        // vec_filter (closure #310): two-pass — count matches
        // first, allocate exactly that many slots, then fill.
        // Predicate signature is `bool (*)(int64_t)`. The
        // `__pred_fn` typedef is forwarded earlier in the bundle
        // (closure #317) so the fused combinator family above
        // can refer to it.
        out.push_str(&format!(
            "static INTENT_UNUSED {sn} {sn}__filter(const {sn}* xs, {sn}__pred_fn p) {{\
\n    {sn} out;\
\n    uint64_t hits = 0;\
\n    for (uint64_t i = 0; i < xs->len; i++) {{\
\n        if (p(xs->data[i])) {{ hits++; }}\
\n    }}\
\n    out.len = hits;\
\n    out.capacity = hits;\
\n    if (hits == 0) {{ out.data = (int64_t*)0; return out; }}\
\n    out.data = (int64_t*)malloc(hits * sizeof(int64_t));\
\n    if (!out.data) abort();\
\n    uint64_t w = 0;\
\n    for (uint64_t i = 0; i < xs->len; i++) {{\
\n        if (p(xs->data[i])) {{ out.data[w++] = xs->data[i]; }}\
\n    }}\
\n    return out;\
\n}}\n",
            sn = struct_name,
        ));
        // Closure #378: vec_position(ref xs, pred) -> Option<i64>.
        // First-index helper; mirrors filter's scan loop but
        // short-circuits on the first hit and returns the index.
        // Gated on Option__i64 being in the payload registry —
        // otherwise the `Enum_Option__i64` typedef wouldn't be
        // declared and the helper would fail to compile.
        let has_option_i64 = ENUM_PAYLOAD_REGISTRY
            .with(|r| r.borrow().contains_key("Option__i64"));
        if has_option_i64 {
            out.push_str(&format!(
                "static INTENT_UNUSED Enum_Option__i64 {sn}__position(const {sn}* xs, {sn}__pred_fn p) {{\
\n    Enum_Option__i64 r;\
\n    for (uint64_t i = 0; i < xs->len; i++) {{\
\n        if (p(xs->data[i])) {{ r.tag = 0; r.payload = (int64_t)i; return r; }}\
\n    }}\
\n    r.tag = 1;\
\n    return r;\
\n}}\n",
                sn = struct_name,
            ));
        }
        // Closure #386: vec_count_if(ref xs, pred) -> i64.
        // Plain i64 return — no Option<i64> dependency, always
        // emitted.
        out.push_str(&format!(
            "static INTENT_UNUSED int64_t {sn}__count_if(const {sn}* xs, {sn}__pred_fn p) {{\
\n    int64_t hits = 0;\
\n    for (uint64_t i = 0; i < xs->len; i++) {{\
\n        if (p(xs->data[i])) {{ hits++; }}\
\n    }}\
\n    return hits;\
\n}}\n",
            sn = struct_name,
        ));
        // Closure #392: vec_max_by / vec_min_by. Key fn has
        // shape `i64 (*)(i64)`. Walk xs tracking the best
        // element + its key score; return Option.None for an
        // empty Vec. Both helpers share `intent_key_fn` typedef
        // (declared once at file scope under the per-Vec helper
        // block — kept inline here for self-contained emit).
        if has_option_i64 {
            out.push_str(&format!(
                "typedef int64_t (*{sn}__key_fn)(int64_t);\n",
                sn = struct_name,
            ));
            out.push_str(&format!(
                "static INTENT_UNUSED Enum_Option__i64 {sn}__max_by(const {sn}* xs, {sn}__key_fn k) {{\
\n    Enum_Option__i64 r;\
\n    if (xs->len == 0) {{ r.tag = 1; return r; }}\
\n    int64_t best = xs->data[0]; int64_t best_k = k(best);\
\n    for (uint64_t i = 1; i < xs->len; i++) {{\
\n        int64_t cur = xs->data[i]; int64_t cur_k = k(cur);\
\n        if (cur_k > best_k) {{ best = cur; best_k = cur_k; }}\
\n    }}\
\n    r.tag = 0; r.payload = best; return r;\
\n}}\n",
                sn = struct_name,
            ));
            out.push_str(&format!(
                "static INTENT_UNUSED Enum_Option__i64 {sn}__min_by(const {sn}* xs, {sn}__key_fn k) {{\
\n    Enum_Option__i64 r;\
\n    if (xs->len == 0) {{ r.tag = 1; return r; }}\
\n    int64_t best = xs->data[0]; int64_t best_k = k(best);\
\n    for (uint64_t i = 1; i < xs->len; i++) {{\
\n        int64_t cur = xs->data[i]; int64_t cur_k = k(cur);\
\n        if (cur_k < best_k) {{ best = cur; best_k = cur_k; }}\
\n    }}\
\n    r.tag = 0; r.payload = best; return r;\
\n}}\n",
                sn = struct_name,
            ));
        }
        // Closure #397: vec_zip_with(ref xs, ref ys, f) ->
        // Vec<i64>. Truncates to the shorter Vec.
        out.push_str(&format!(
            "typedef int64_t (*{sn}__zip_fn)(int64_t, int64_t);\n",
            sn = struct_name,
        ));
        out.push_str(&format!(
            "static INTENT_UNUSED {sn} {sn}__zip_with(const {sn}* xs, const {sn}* ys, {sn}__zip_fn f) {{\
\n    {sn} out;\
\n    uint64_t n = xs->len < ys->len ? xs->len : ys->len;\
\n    out.len = n; out.capacity = n;\
\n    if (n == 0) {{ out.data = (int64_t*)0; return out; }}\
\n    out.data = (int64_t*)malloc(n * sizeof(int64_t));\
\n    if (!out.data) abort();\
\n    for (uint64_t i = 0; i < n; i++) out.data[i] = f(xs->data[i], ys->data[i]);\
\n    return out;\
\n}}\n",
            sn = struct_name,
        ));
        // Closure #389: vec_take_while / vec_drop_while.
        out.push_str(&format!(
            "static INTENT_UNUSED {sn} {sn}__take_while(const {sn}* xs, {sn}__pred_fn p) {{\
\n    {sn} out;\
\n    uint64_t n = 0;\
\n    while (n < xs->len && p(xs->data[n])) {{ n++; }}\
\n    out.len = n; out.capacity = n;\
\n    if (n == 0) {{ out.data = (int64_t*)0; return out; }}\
\n    out.data = (int64_t*)malloc(n * sizeof(int64_t));\
\n    if (!out.data) abort();\
\n    memcpy(out.data, xs->data, n * sizeof(int64_t));\
\n    return out;\
\n}}\n",
            sn = struct_name,
        ));
        out.push_str(&format!(
            "static INTENT_UNUSED {sn} {sn}__drop_while(const {sn}* xs, {sn}__pred_fn p) {{\
\n    {sn} out;\
\n    uint64_t skip = 0;\
\n    while (skip < xs->len && p(xs->data[skip])) {{ skip++; }}\
\n    uint64_t n = xs->len - skip;\
\n    out.len = n; out.capacity = n;\
\n    if (n == 0) {{ out.data = (int64_t*)0; return out; }}\
\n    out.data = (int64_t*)malloc(n * sizeof(int64_t));\
\n    if (!out.data) abort();\
\n    memcpy(out.data, xs->data + skip, n * sizeof(int64_t));\
\n    return out;\
\n}}\n",
            sn = struct_name,
        ));
        // dedup: remove consecutive duplicates. Returns the
        // post-dedup length so the caller can verify the work
        // was done. Sort first if you want unique-set behavior.
        out.push_str(&format!(
            "static INTENT_UNUSED int64_t {sn}__dedup({sn}* xs) {{\
\n    if (xs->len < 2) {{ return (int64_t)xs->len; }}\
\n    uint64_t w = 1;\
\n    for (uint64_t r = 1; r < xs->len; r++) {{\
\n        if (xs->data[r] != xs->data[w - 1]) {{\
\n            xs->data[w] = xs->data[r];\
\n            w++;\
\n        }}\
\n    }}\
\n    xs->len = w;\
\n    return (int64_t)w;\
\n}}\n",
            sn = struct_name,
        ));
        // Data-structures roadmap Level 2 — BinaryHeap-on-Vec.
        // Min-heap with sift-up (push) / sift-down (pop) /
        // Floyd O(n) heapify. v1 i64 element only.
        // heap_push / sift_up / sift_down / heapify always
        // emit; heap_pop / heap_peek gated on `Option__i64`
        // being in the enum registry (forward-references
        // `Enum_Option__i64` otherwise).
        out.push_str(&format!(
            "static INTENT_UNUSED void {sn}__heap_sift_up({sn}* xs, uint64_t i) {{\
\n    while (i > 0) {{\
\n        uint64_t p = (i - 1) / 2;\
\n        if (xs->data[i] >= xs->data[p]) break;\
\n        int64_t t = xs->data[i]; xs->data[i] = xs->data[p]; xs->data[p] = t;\
\n        i = p;\
\n    }}\
\n}}\n\
static INTENT_UNUSED void {sn}__heap_sift_down({sn}* xs, uint64_t i) {{\
\n    uint64_t n = xs->len;\
\n    while (1) {{\
\n        uint64_t l = 2 * i + 1;\
\n        uint64_t r = 2 * i + 2;\
\n        uint64_t s = i;\
\n        if (l < n && xs->data[l] < xs->data[s]) s = l;\
\n        if (r < n && xs->data[r] < xs->data[s]) s = r;\
\n        if (s == i) break;\
\n        int64_t t = xs->data[i]; xs->data[i] = xs->data[s]; xs->data[s] = t;\
\n        i = s;\
\n    }}\
\n}}\n\
static INTENT_UNUSED int64_t {sn}__heap_push({sn}* xs, int64_t v) {{\
\n    if (xs->len >= xs->capacity) {{\
\n        xs->capacity = xs->capacity ? xs->capacity * 2 : 1;\
\n        xs->data = (int64_t*)realloc(xs->data, xs->capacity * sizeof(int64_t));\
\n        if (!xs->data) abort();\
\n    }}\
\n    xs->data[xs->len] = v;\
\n    xs->len++;\
\n    {sn}__heap_sift_up(xs, xs->len - 1);\
\n    return (int64_t)xs->len;\
\n}}\n\
static INTENT_UNUSED int64_t {sn}__heapify({sn}* xs) {{\
\n    if (xs->len < 2) return 0;\
\n    for (int64_t i = (int64_t)(xs->len / 2) - 1; i >= 0; i--) {{\
\n        {sn}__heap_sift_down(xs, (uint64_t)i);\
\n    }}\
\n    return 0;\
\n}}\n",
            sn = struct_name,
        ));
        let has_option_i64_heap = ENUM_PAYLOAD_REGISTRY.with(|r| {
            r.borrow().contains_key("Option__i64")
        });
        if has_option_i64_heap {
            out.push_str(&format!(
                "static INTENT_UNUSED {opt_name} {sn}__heap_pop({sn}* xs) {{\
\n    {opt_name} r;\
\n    if (xs->len == 0) {{ r.tag = 1; r.payload = 0; return r; }}\
\n    int64_t top = xs->data[0];\
\n    xs->len--;\
\n    if (xs->len > 0) {{\
\n        xs->data[0] = xs->data[xs->len];\
\n        {sn}__heap_sift_down(xs, 0);\
\n    }}\
\n    r.tag = 0; r.payload = top;\
\n    return r;\
\n}}\n\
static INTENT_UNUSED {opt_name} {sn}__heap_peek(const {sn}* xs) {{\
\n    {opt_name} r;\
\n    if (xs->len == 0) {{ r.tag = 1; r.payload = 0; return r; }}\
\n    r.tag = 0; r.payload = xs->data[0];\
\n    return r;\
\n}}\n",
                sn = struct_name,
                opt_name = "Enum_Option__i64",
            ));
        }
    }

    // `__set(xs, i, v)`: store the new value at xs.data[i].
    // For non-Copy elements (Vec<T>, Array<T, N>) the old slot
    // value's resources are released first via the element-
    // specific cleanup (recursive free for `Vec<T>`, no-op for
    // arrays since their backing storage is inline in the
    // outer buffer). Without the cleanup an overwrite would
    // leak the prior inner-Vec's heap buffer.
    let set_cleanup = if element_is_copy {
        String::new()
    } else {
        c_element_drop_old("xs.data[i]", element)
    };
    let set_store = if element_is_array {
        format!(
            "    memcpy(xs.data[i], v, sizeof({}));",
            c_element,
        )
    } else {
        "    xs.data[i] = v;".to_string()
    };
    out.push_str(&format!(
        "static INTENT_UNUSED {sn} {sn}__set({sn} xs, uint64_t i, {ct} v) {{\
\n    assert(i < xs.len);\
{cleanup}\
\n{store}\
\n    return xs;\
\n}}\n",
        sn = struct_name,
        ct = c_element,
        cleanup = set_cleanup,
        store = set_store,
    ));

    // `__clone(xs)`: malloc a new buffer + copy each element.
    // For Copy elements a single memcpy suffices. For non-Copy
    // elements (`Vec<T>`) each slot needs the element's own
    // deep-clone helper so the duplicated buffer doesn't alias
    // the source's inner storage (which would cause double-
    // frees when both Vecs are dropped). Arrays-of-Copy slots
    // are themselves Copy (memcpy is fine).
    let clone_body = if element_is_copy {
        format!(
            "    if (xs.len > 0) memcpy(c.data, xs.data, xs.len * sizeof({ct}));",
            ct = c_element,
        )
    } else if element_is_array {
        // Arrays-of-Copy slots are themselves Copy bytes —
        // memcpy the whole buffer (matches Copy element
        // path). Phase 2c.
        format!(
            "    if (xs.len > 0) memcpy(c.data, xs.data, xs.len * sizeof({ct}));",
            ct = c_element,
        )
    } else {
        format!(
            "    for (uint64_t k = 0; k < xs.len; ++k) {{\
\n        c.data[k] = {dup};\
\n    }}",
            dup = c_element_deep_clone("xs.data[k]", element),
        )
    };
    out.push_str(&format!(
        "static INTENT_UNUSED {sn} {sn}__clone({sn} xs) {{\
\n    {sn} c;\
\n    c.data = ({ct}*)malloc((xs.len == 0 ? 1 : xs.len) * sizeof({ct}));\
\n    if (!c.data) abort();\
\n{body}\
\n    c.len = xs.len;\
\n    c.capacity = xs.len == 0 ? 1 : xs.len;\
\n    return c;\
\n}}\n",
        sn = struct_name,
        ct = c_element,
        body = clone_body,
    ));

    // `__free(xs)`: for Copy elements just free the heap
    // buffer. For non-Copy element types we first walk every
    // live slot and free each element's inner resources (the
    // element's own drop), then free the outer buffer.
    if element_is_copy {
        out.push_str(&format!(
            "static INTENT_UNUSED void {sn}__free({sn} xs) {{ free(xs.data); }}\n\n",
            sn = struct_name
        ));
    } else {
        let inner_drop = c_element_drop_old("xs.data[k]", element);
        out.push_str(&format!(
            "static INTENT_UNUSED void {sn}__free({sn} xs) {{\
\n    for (uint64_t k = 0; k < xs.len; ++k) {{\
{inner}\
\n    }}\
\n    free(xs.data);\
\n}}\n\n",
            sn = struct_name,
            inner = inner_drop,
        ));
    }
}

/// Storage-type C spelling for a value of type `ty`. The
/// difference between this and `c_leaf_type` is aggregate
/// handling: for `Vec<U>` we want the struct typedef
/// (`intent_vec_<U>`), not the placeholder `"/* vec */"`; for
/// `[T; N]` we want the per-shape typedef alias. New for #7;
/// used inside vec bundle bodies where the element type may
/// itself be a Vec (so we'd otherwise emit invalid C).
pub(crate) fn c_element_storage(ty: &Type) -> String {
    match ty {
        Type::Vec(inner) => vec_c_struct(inner),
        Type::Array { .. } => array_c_typedef(ty),
        Type::Tuple(elements) => tuple_c_struct(elements),
        Type::Struct(name) => struct_c_name(name),
        // Payloaded enums spell as `Enum_<Name>`; tag-only
        // enums keep `int32_t` via the c_leaf_type fallback.
        // Closure #151 (parallel to the element_tag fix).
        Type::Enum(name)
            if ENUM_PAYLOAD_REGISTRY.with(|r| r.borrow().contains_key(name)) =>
        {
            enum_c_name(name)
        }
        // Closure #208: `Channel<T, N>` is parametric over
        // both element width and capacity. The c_leaf_type
        // fallback returns the hardcoded
        // `intent_channel_int64_t_16` (the comment there
        // explicitly notes callers must special-case
        // Channel). Without this arm, a `Channel<i64, 4>`
        // struct field declared as `intent_channel_int64_t_16`
        // doesn't match the constructor's
        // `intent_channel_int64_t_4_new()` return type and cc
        // rejects with "incompatible types".
        Type::Channel(element, capacity) => c_channel_storage(element, *capacity),
        // Closure #209: same shape for `Atomic<T>`. The
        // c_leaf_type fallback returns `_Atomic int64_t`
        // for any Atomic; an `Atomic<u32>` struct field
        // declared at i64 width would silently use the wrong
        // memory size on platforms where i64 and u32 atomics
        // have different alignment / lock-free behavior.
        // `c_atomic_storage(element)` returns
        // `_Atomic <c_leaf_type(element)>` — the right
        // per-element width.
        Type::Atomic(element) => c_atomic_storage(element),
        // Vtables Phase 4: `dyn Iface` storage spells as
        // `intent_dyn_<Iface>` (the per-Iface fat-pointer
        // typedef emitted in the preamble). Without this arm
        // a struct field / Vec element of `dyn Iface` falls
        // through to the placeholder `intent_dyn` from
        // `c_leaf_type` and cc rejects with "unknown type".
        Type::Object(iface) => format!("intent_dyn_{}", iface),
        // L2 Phase 1 (2026-06-07): Box<T> storage spelling is
        // `T*` — a single owning pointer. The c_leaf_type
        // fallback returns the `/* Box<T> */` placeholder; this
        // arm produces the real type so struct fields and Vec
        // elements compile cleanly.
        // L2 Phase 3 (2026-06-08): Box<dyn Iface> spelling is
        // the fat-pointer struct itself (16 bytes), NOT a
        // pointer to it. The struct's `.data` field owns the
        // heap allocation; the local IS the fat pointer.
        Type::Box(inner) => match &**inner {
            Type::Object(iface) => format!("intent_dyn_{}", iface),
            _ => format!("{}*", format_declarator(inner, "").trim()),
        },
        _ => c_leaf_type(ty).to_string(),
    }
}

/// C-side typedef name for `[T; N]` used inside helper
/// signatures. Built per-shape so a `Vec<[i64; 4]>` element
/// spells as `intent_arr4_int64_t` — distinct from any
/// scalar/vec spelling. The typedef itself is emitted upstream
/// in `emit_array_typedefs_for`.
pub(crate) fn array_c_typedef(ty: &Type) -> String {
    let Type::Array { element, length } = ty else {
        unreachable!("array_c_typedef called on non-array");
    };
    format!("intent_arr{}_{}", length, element_tag(element))
}

/// Closure #239: per-shape struct wrapping `[T; N]` for use
/// in return position. C arrays can't be values in return
/// position; the struct gets passed by value and the caller
/// memcpys `.data` into a local array.
pub(crate) fn array_return_struct_name(element: &Type, length: u64) -> String {
    format!("intent_arr_ret_{}_{}", length, element_tag(element))
}

/// Walk a Vec-element type and emit a `typedef` for every
/// `Array<T, N>` shape that appears, deduplicated against
/// `seen` (keyed on the typedef name). Recurses through
/// nested aggregates so a `Vec<[[i64; 2]; 3]>` would emit
/// both the inner and outer array typedefs. New for #7 phase
/// 2c.
pub(crate) fn emit_array_typedefs_for(
    ty: &Type,
    seen: &mut BTreeSet<String>,
    out: &mut String,
) {
    match ty {
        Type::Array { element, length } => {
            // Recurse first so nested array shapes are
            // declared before the outer typedef references
            // them (mirrors the inner-first Vec bundle
            // order).
            emit_array_typedefs_for(element, seen, out);
            let name = array_c_typedef(ty);
            if seen.insert(name.clone()) {
                let inner_spelling = match element.as_ref() {
                    Type::Array { .. } => array_c_typedef(element),
                    Type::Vec(_) => vec_c_struct(element),
                    _ => c_leaf_type(element).to_string(),
                };
                out.push_str(&format!(
                    "typedef {} {}[{}];\n",
                    inner_spelling, name, length,
                ));
            }
        }
        Type::Vec(inner) | Type::Ref(inner) | Type::RefMut(inner) => {
            emit_array_typedefs_for(inner, seen, out);
        }
        _ => {}
    }
}

/// Drop-old-slot expression: a C statement (or empty) that
/// releases the resources owned by `slot`, whose value-type
/// is `ty`. For `Vec<U>` we recurse through the inner Vec's
/// `__free` helper. Arrays of Copy contain no heap so they
/// need nothing. Used by `__set` and `__free` to keep the
/// cleanup shape in one place.
pub(crate) fn c_element_drop_old(slot: &str, ty: &Type) -> String {
    match ty {
        Type::Vec(inner) => format!(
            "\n        {helper}({slot});",
            helper = vec_helper(inner, "free"),
            slot = slot,
        ),
        Type::OwnedStr => format!("\n        free((void*){slot});", slot = slot),
        Type::Struct(name) => {
            // Drop each owning field of the struct slot via the
            // shared `emit_struct_field_drops` helper. If the
            // struct has no owning fields (or isn't registered),
            // emit nothing — matches the previous behavior.
            // Closure #127.
            let fields = STRUCT_FIELDS_REGISTRY
                .with(|r| r.borrow().get(name).cloned())
                .unwrap_or_default();
            if fields.is_empty() {
                return String::new();
            }
            let mut body = String::new();
            let empty: std::collections::HashSet<&String> =
                std::collections::HashSet::new();
            emit_struct_field_drops(slot, name, &fields, &empty, &mut body);
            if body.is_empty() {
                return String::new();
            }
            // `emit_struct_field_drops` emits each line with a
            // leading two-space indent. The Vec __free body
            // expects each statement to be indented by 8 spaces
            // (inside a 4-space-indented `for` block in a 4-space
            // indented helper). Re-indent and prepend a leading
            // newline so we slot cleanly in.
            let mut reindented = String::new();
            for line in body.lines() {
                let trimmed = line.trim_start();
                if trimmed.is_empty() {
                    continue;
                }
                reindented.push_str("\n        ");
                reindented.push_str(trimmed);
            }
            reindented
        }
        Type::Enum(name) => {
            // Drop the enum's payload when the active tag is
            // one of the payloaded variants. Mirrors the
            // scope-exit Drop logic for enums. Closure #151
            // (`Vec<PayloadedEnum>` was leaking each
            // element's payload at outer __free time).
            let payload_ty = ENUM_PAYLOAD_REGISTRY
                .with(|r| r.borrow().get(name).cloned());
            let free_expr: Option<String> = match &payload_ty {
                Some(Type::OwnedStr) => {
                    Some(format!("free((void*){slot}.payload);", slot = slot))
                }
                Some(Type::Vec(element)) => Some(format!(
                    "{helper}({slot}.payload);",
                    helper = vec_helper(element, "free"),
                    slot = slot
                )),
                _ => None,
            };
            let Some(free_call) = free_expr else {
                return String::new();
            };
            let payload_tags: Vec<u32> = ENUM_PAYLOAD_TAGS_REGISTRY
                .with(|r| r.borrow().get(name).cloned().unwrap_or_default());
            if payload_tags.is_empty() {
                return String::new();
            }
            let cases: Vec<String> = payload_tags
                .iter()
                .map(|t| format!("case {}", t))
                .collect();
            format!(
                "\n        switch ({slot}.tag) {{ {}: {} break; default: break; }}",
                cases.join(": "),
                free_call,
                slot = slot
            )
        }
        _ => String::new(),
    }
}

/// Deep-clone expression for a value of type `ty`. For Copy
/// values the original is returned (memcpy semantics are
/// correct). For `Vec<U>` we route through the inner Vec's
/// `__clone`. New for #7.
pub(crate) fn c_element_deep_clone(slot: &str, ty: &Type) -> String {
    match ty {
        Type::Vec(inner) => format!(
            "{helper}({slot})",
            helper = vec_helper(inner, "clone"),
            slot = slot,
        ),
        // Closure #152: `clone(Vec<OwnedStr>)` /
        // `clone(Vec<Enum_with_OwnedStr>)` was shallow-
        // copying the heap pointer, then both source and
        // clone double-freed at scope exit.
        //
        // OwnedStr: round-trip through `intent_str_concat`
        // with an empty literal — the helper mallocs a
        // fresh buffer of the source's length and memcpy's
        // the bytes, giving us a strdup-like deep copy.
        Type::OwnedStr => format!(
            "intent_str_concat({slot}, 0, \"\", 0)",
            slot = slot
        ),
        // Closure #153: `Vec<Struct{heap-field}>` clone was
        // shallow-copying the struct, so every heap-shaped
        // field pointer was shared between source and clone
        // and double-freed at scope exit. Reconstruct the
        // struct with each owning field deep-cloned
        // (recursive call) and Copy fields copied as-is.
        Type::Struct(name) => {
            let fields = STRUCT_FIELDS_REGISTRY
                .with(|r| r.borrow().get(name).cloned())
                .unwrap_or_default();
            let has_owning = fields.iter().any(|(_, ty)| !ty.is_copy());
            if !has_owning {
                return slot.to_string();
            }
            let mut parts: Vec<String> = Vec::with_capacity(fields.len());
            for (fname, fty) in &fields {
                let field_slot = format!("({}).{}", slot, fname);
                let field_clone = c_element_deep_clone(&field_slot, fty);
                parts.push(format!(".{} = {}", fname, field_clone));
            }
            return format!(
                "(({}){{ {} }})",
                struct_c_name(name),
                parts.join(", ")
            );
        }
        // Enum with OwnedStr payload: tag-switched ternary
        // — for payloaded tags, reconstruct the enum
        // struct with a deep-cloned payload; otherwise
        // keep the struct as-is.
        Type::Enum(name) => {
            let payload_ty = ENUM_PAYLOAD_REGISTRY
                .with(|r| r.borrow().get(name).cloned());
            let payload_tags: Vec<u32> = ENUM_PAYLOAD_TAGS_REGISTRY
                .with(|r| r.borrow().get(name).cloned().unwrap_or_default());
            match (&payload_ty, payload_tags.is_empty()) {
                (Some(Type::OwnedStr), false) => {
                    let mut cond = String::from("0");
                    for t in &payload_tags {
                        cond = format!("({} || {}.tag == {})", cond, slot, t);
                    }
                    format!(
                        "(({}) ? (({}){{ .tag = ({}).tag, .payload = intent_str_concat(({}).payload, 0, \"\", 0) }}) : ({}))",
                        cond,
                        enum_c_name(name),
                        slot,
                        slot,
                        slot
                    )
                }
                _ => slot.to_string(),
            }
        }
        _ => slot.to_string(),
    }
}

fn emit_prototype(function: &TypedFunction, out: &mut String) {
    if function.is_extern {
        out.push_str("extern ");
        out.push_str(&c_type_name(&function.return_type));
        out.push(' ');
        out.push_str(&function.name);
        out.push('(');
        emit_params(function, out);
        out.push_str(");\n");
        return;
    }
    out.push_str("static ");
    out.push_str(&c_type_name(&function.return_type));
    out.push(' ');
    out.push_str(&function_name(&function.name));
    out.push('(');
    emit_params(function, out);
    out.push_str(");\n");
}

fn emit_function(function: &TypedFunction, out: &mut String) {
    // Closure #269: `extern "C" fn name(...) -> R;` emits a
    // forward declaration of the bare C symbol (no `fn_`
    // prefix, no `static` storage class) and returns. The
    // linker provides the body.
    if function.is_extern {
        out.push_str("extern ");
        out.push_str(&c_type_name(&function.return_type));
        out.push(' ');
        out.push_str(&function.name);
        out.push('(');
        emit_params(function, out);
        out.push_str(");\n");
        return;
    }
    // Closure #286: `#[bounded(N)]` attribute emits a
    // thread-local depth counter + bound check at fn entry.
    // GCC's __attribute__((cleanup)) ensures the decrement
    // runs on every exit path (including early returns).
    // Same shape works on clang.
    if let Some(bound) = function.recursion_bound {
        let counter_name = format!("__intent_depth_{}", function.name);
        let dec_helper = format!("__intent_dec_depth_{}", function.name);
        out.push_str(&format!(
            "static __thread int {} = 0;\n", counter_name
        ));
        out.push_str(&format!(
            "static void {}(int* __u) {{ (void)__u; --{}; }}\n",
            dec_helper, counter_name
        ));
        out.push_str("static ");
        out.push_str(&c_type_name(&function.return_type));
        out.push(' ');
        out.push_str(&function_name(&function.name));
        out.push('(');
        emit_params(function, out);
        out.push_str(") {\n");
        out.push_str(&format!(
            "  int __depth_guard __attribute__((cleanup({}))) = 0;\n  (void)__depth_guard;\n",
            dec_helper
        ));
        out.push_str(&format!(
            "  if (++{} > {}) {{ \
              fprintf(stderr, \"recursion bound exceeded in '{}' (#[bounded({})]); aborting\\n\"); \
              abort(); \
            }}\n",
            counter_name, bound, function.name, bound
        ));
        for requirement in &function.requires {
            out.push_str("  assert(");
            out.push_str(&emit_expr(requirement));
            out.push_str(");\n");
        }
        for stmt in &function.body {
            emit_stmt(stmt, out);
        }
        out.push_str("}\n");
        return;
    }
    out.push_str("static ");
    out.push_str(&c_type_name(&function.return_type));
    out.push(' ');
    out.push_str(&function_name(&function.name));
    out.push('(');
    emit_params(function, out);
    out.push_str(") {\n");

    for requirement in &function.requires {
        out.push_str("  assert(");
        out.push_str(&emit_expr(requirement));
        out.push_str(");\n");
    }

    for stmt in &function.body {
        emit_stmt(stmt, out);
    }

    out.push_str("}\n");
}

fn emit_params(function: &TypedFunction, out: &mut String) {
    if function.params.is_empty() {
        out.push_str("void");
        return;
    }

    for (index, param) in function.params.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        out.push_str(&format_declarator(&param.ty, &local_name(&param.name)));
    }
}

fn emit_stmt(stmt: &TypedStmt, out: &mut String) {
    match stmt {
        TypedStmt::Let { name, ty, expr } => {
            // Closure #276 + Arc 5 polish: when the Let RHS is
            // the synthetic-prelude Block emitted by the dyn-
            // coerce hoist (one or more `__dyn_src_*` lets
            // followed by a DynCoerce tail OR a Vec(...) call
            // whose args are DynCoerce wrappers), emit the
            // prelude stmts at the OUTER level so the temps'
            // storage lives for the enclosing block — not just
            // the GCC stmt-expr. Without this, `&__dyn_src`
            // would dangle by the time the fat pointer's data
            // slot is read. Recognized by name prefix so other
            // Block-RHS shapes (closure #200, etc.) still go
            // through the regular stmt-expr path.
            if let TypedExprKind::Block { stmts, tail } = &expr.kind {
                let all_dyn_src_prelude = !stmts.is_empty()
                    && stmts.iter().all(|s| matches!(
                        s,
                        TypedStmt::Let { name: n, .. } if n.starts_with("__dyn_src_")
                    ));
                if all_dyn_src_prelude {
                    for s in stmts {
                        emit_stmt(s, out);
                    }
                    emit_stmt(
                        &TypedStmt::Let {
                            name: name.clone(),
                            ty: ty.clone(),
                            expr: (**tail).clone(),
                        },
                        out,
                    );
                    return;
                }
            }
            out.push_str("  ");
            if let Type::Array { element, length } = ty {
                if let TypedExprKind::ArrayLit { elements } = &expr.kind {
                    let element_strs: Vec<String> = elements.iter().map(emit_expr).collect();
                    // Use the per-shape storage spelling for
                    // aggregate elements (`Struct_Point`,
                    // `intent_tuple_…`) so `[Point; 3]` arrays
                    // emit valid C declarations rather than
                    // the `/* struct */` placeholder.
                    out.push_str(&c_element_storage(element));
                    out.push(' ');
                    out.push_str(&local_name(name));
                    out.push('[');
                    out.push_str(&length.to_string());
                    out.push_str("] = { ");
                    out.push_str(&element_strs.join(", "));
                    out.push_str(" };\n");
                } else {
                    // Closure #239: if the RHS is a Call /
                    // Block / other shape whose value-type is
                    // Array, it returns the struct wrapper
                    // (`intent_arr_ret_<N>_<T>`). Spill into a
                    // struct temp first, then memcpy `.data`
                    // into the local array. Plain Var / FieldAccess
                    // / Index sources that emit as an lvalue
                    // (decaying naturally to the element-type
                    // pointer) still work with the original
                    // memcpy-from-array form.
                    let needs_struct_unwrap = matches!(
                        &expr.kind,
                        TypedExprKind::Call { .. }
                            | TypedExprKind::Block { .. }
                            | TypedExprKind::IfExpr { .. }
                            | TypedExprKind::Match { .. }
                    );
                    out.push_str(&c_element_storage(element));
                    out.push(' ');
                    out.push_str(&local_name(name));
                    out.push('[');
                    out.push_str(&length.to_string());
                    out.push_str("];\n");
                    if needs_struct_unwrap {
                        let wrapper = array_return_struct_name(element, *length);
                        out.push_str(&format!(
                            "  {} _intent_ret_{} = {};\n  memcpy({}, _intent_ret_{}.data, sizeof({}));\n",
                            wrapper,
                            name,
                            emit_expr(expr),
                            local_name(name),
                            name,
                            local_name(name),
                        ));
                    } else {
                        out.push_str("  memcpy(");
                        out.push_str(&local_name(name));
                        out.push_str(", ");
                        out.push_str(&emit_expr(expr));
                        out.push_str(", sizeof(");
                        out.push_str(&local_name(name));
                        out.push_str("));\n");
                    }
                }
            } else if matches!(ty, Type::FnPtr(_, _)) {
                // C function-pointer declarators have to wrap
                // the binding name inside `(*name)` so the
                // tokens parse — `int64_t (*v)(int64_t) =
                // expr;`. Reuse format_declarator which knows
                // the syntax.
                out.push_str(&format_declarator(ty, &local_name(name)));
                out.push_str(" = ");
                out.push_str(&emit_expr(expr));
                out.push_str(";\n");
            } else {
                out.push_str(&c_type_name(ty));
                out.push(' ');
                out.push_str(&local_name(name));
                out.push_str(" = ");
                out.push_str(&emit_expr(expr));
                out.push_str(";\n");
            }
        }
        TypedStmt::Reassign {
            name,
            ty,
            expr,
            drop_old,
        } => {
            if *drop_old {
                // Heap-shaped reassign: evaluate the RHS into
                // a temp first, free the OLD value, then move
                // the temp into the binding. The order matters
                // — the RHS may consume the binding itself
                // (e.g. `xs = push(xs, k)` returns a fresh vec
                // that takes ownership of the old buffer), and
                // freeing-before-evaluating would crash. Vec
                // was wired in closure #8 (`drop_old`
                // self-consuming reassign). Closure #133
                // extends the same pattern to OwnedStr — the
                // bare-`x = "b" + ""` case was silently
                // leaking the previous heap string.
                match ty {
                    Type::Vec(element) => {
                        let struct_name = vec_c_struct(element);
                        let tmp = format!("_intent_tmp_{}", name);
                        out.push_str("  {\n");
                        out.push_str("    ");
                        out.push_str(&struct_name);
                        out.push(' ');
                        out.push_str(&tmp);
                        out.push_str(" = ");
                        out.push_str(&emit_expr(expr));
                        out.push_str(";\n    ");
                        out.push_str(&vec_helper(element, "free"));
                        out.push('(');
                        out.push_str(&local_name(name));
                        out.push_str(");\n    ");
                        out.push_str(&local_name(name));
                        out.push_str(" = ");
                        out.push_str(&tmp);
                        out.push_str(";\n  }\n");
                    }
                    Type::OwnedStr => {
                        let tmp = format!("_intent_tmp_{}", name);
                        out.push_str("  {\n");
                        out.push_str("    char* ");
                        out.push_str(&tmp);
                        out.push_str(" = ");
                        out.push_str(&emit_expr(expr));
                        out.push_str(";\n");
                        out.push_str("    free((void*)");
                        out.push_str(&local_name(name));
                        out.push_str(");\n    ");
                        out.push_str(&local_name(name));
                        out.push_str(" = ");
                        out.push_str(&tmp);
                        out.push_str(";\n  }\n");
                    }
                    Type::Struct(struct_name) => {
                        // Closure #147: reassigning a struct
                        // binding that owns heap fields was
                        // leaking the previous fields' heap.
                        // Evaluate RHS into a tmp, walk the
                        // OLD binding's per-field drops, then
                        // move the tmp in.
                        let fields = STRUCT_FIELDS_REGISTRY
                            .with(|r| r.borrow().get(struct_name).cloned())
                            .unwrap_or_default();
                        let tmp = format!("_intent_tmp_{}", name);
                        out.push_str("  {\n");
                        out.push_str("    ");
                        out.push_str(&struct_c_name(struct_name));
                        out.push(' ');
                        out.push_str(&tmp);
                        out.push_str(" = ");
                        out.push_str(&emit_expr(expr));
                        out.push_str(";\n");
                        let empty: std::collections::HashSet<&String> =
                            std::collections::HashSet::new();
                        emit_struct_field_drops(
                            &local_name(name),
                            struct_name,
                            &fields,
                            &empty,
                            out,
                        );
                        out.push_str("    ");
                        out.push_str(&local_name(name));
                        out.push_str(" = ");
                        out.push_str(&tmp);
                        out.push_str(";\n  }\n");
                    }
                    Type::Enum(enum_name) => {
                        // Closure #147: reassigning a
                        // payloaded enum binding was leaking
                        // the previous payload heap. Eval
                        // RHS into a tmp, switch on the OLD
                        // tag to free the payload, then move
                        // the tmp in.
                        let payload_ty = ENUM_PAYLOAD_REGISTRY
                            .with(|r| r.borrow().get(enum_name).cloned());
                        let free_expr: Option<String> = match &payload_ty {
                            Some(Type::OwnedStr) => Some(format!(
                                "free((void*){}.payload)",
                                local_name(name)
                            )),
                            Some(Type::Vec(element)) => Some(format!(
                                "{}({}.payload)",
                                vec_helper(element, "free"),
                                local_name(name)
                            )),
                            _ => None,
                        };
                        let tmp = format!("_intent_tmp_{}", name);
                        out.push_str("  {\n");
                        out.push_str("    ");
                        out.push_str(&enum_c_name(enum_name));
                        out.push(' ');
                        out.push_str(&tmp);
                        out.push_str(" = ");
                        out.push_str(&emit_expr(expr));
                        out.push_str(";\n");
                        if let Some(free_call) = free_expr {
                            let payload_tags: Vec<u32> =
                                ENUM_PAYLOAD_TAGS_REGISTRY.with(|r| {
                                    r.borrow()
                                        .get(enum_name)
                                        .cloned()
                                        .unwrap_or_default()
                                });
                            if !payload_tags.is_empty() {
                                let cases: Vec<String> = payload_tags
                                    .iter()
                                    .map(|t| format!("case {}", t))
                                    .collect();
                                out.push_str(&format!(
                                    "    switch ({}.tag) {{ {}: {}; break; default: break; }}\n",
                                    local_name(name),
                                    cases.join(": "),
                                    free_call
                                ));
                            }
                        }
                        out.push_str("    ");
                        out.push_str(&local_name(name));
                        out.push_str(" = ");
                        out.push_str(&tmp);
                        out.push_str(";\n  }\n");
                    }
                    _ => {
                        out.push_str("  ");
                        out.push_str(&local_name(name));
                        out.push_str(" = ");
                        out.push_str(&emit_expr(expr));
                        out.push_str(";\n");
                    }
                }
            } else {
                out.push_str("  ");
                out.push_str(&local_name(name));
                out.push_str(" = ");
                out.push_str(&emit_expr(expr));
                out.push_str(";\n");
            }
        }
        TypedStmt::Drop { name, ty, moved_fields } => match ty {
            Type::Vec(element) => {
                out.push_str("  ");
                out.push_str(&vec_helper(element, "free"));
                out.push('(');
                out.push_str(&local_name(name));
                out.push_str(");\n");
            }
            Type::OwnedStr => {
                // Owned strings are heap-allocated by the concat
                // path (malloc); free the buffer here.
                out.push_str("  free((void*)");
                out.push_str(&local_name(name));
                out.push_str(");\n");
            }
            Type::Guard(_) => {
                // RAII: dropping a guard releases the lock.
                // The guard's `m` field still points at the
                // mutex storage; the unlock helper resets the
                // `locked` flag.
                out.push_str("  intent_guard_i64_unlock(&");
                out.push_str(&local_name(name));
                out.push_str(");\n");
            }
            Type::Condvar => {
                // Stack-by-value (mirrors Mutex): no heap to
                // free at scope exit. The binding's bits are
                // reclaimed with the stack frame; pending
                // waiters (if any) are the user's responsibility
                // to drain via notify_all before the condvar
                // goes out of scope.
            }
            Type::Deque(_) => {
                // Affine handle: free the ring buffer's heap
                // data at scope exit. The struct itself is
                // stack-allocated; only `data` lives on the
                // heap.
                out.push_str("  intent_deque_i64_drop(&");
                out.push_str(&local_name(name));
                out.push_str(");\n");
            }
            Type::Box(inner) => {
                // L2 Phase 1 (2026-06-07): Box<T> owns a single
                // heap slot allocated by box(x). Scope-exit drop
                // simply `free`s the slot. The binding's machine
                // word (the pointer) is reclaimed with the stack
                // frame.
                // L2 Phase 3 (2026-06-08): Box<dyn Iface> is a
                // fat pointer struct with owning `.data`. Free
                // `.data` (the heap concrete); the struct itself
                // is in the local alloca and reclaimed with the
                // stack frame.
                // L2 follow-up (2026-06-08): Box<Vec<T>> and
                // Box<OwnedStr> own a heap slot whose contained
                // value ALSO owns heap memory. Chain the inner
                // type's Drop before freeing the box's slot:
                //   Box<Vec<T>>:    intent_vec_T__free(*box);
                //                   free(box);
                //   Box<OwnedStr>:  free(*box);  free(box);
                match &**inner {
                    Type::Object(_) => {
                        out.push_str("  free(");
                        out.push_str(&local_name(name));
                        out.push_str(".data);\n");
                    }
                    Type::Vec(element) => {
                        out.push_str("  ");
                        out.push_str(&vec_helper(element, "free"));
                        out.push_str("(*");
                        out.push_str(&local_name(name));
                        out.push_str(");\n");
                        out.push_str("  free(");
                        out.push_str(&local_name(name));
                        out.push_str(");\n");
                    }
                    Type::OwnedStr => {
                        out.push_str("  free((void*)*");
                        out.push_str(&local_name(name));
                        out.push_str(");\n");
                        out.push_str("  free(");
                        out.push_str(&local_name(name));
                        out.push_str(");\n");
                    }
                    _ => {
                        out.push_str("  free(");
                        out.push_str(&local_name(name));
                        out.push_str(");\n");
                    }
                }
            }
            Type::HashSet(_) => {
                out.push_str("  intent_hashset_i64_drop(&");
                out.push_str(&local_name(name));
                out.push_str(");\n");
            }
            Type::HashMap(k, v) => {
                // ARC 1.4e: dispatch the drop call onto the
                // right per-(K, V) bundle. Legacy (i64, i64)
                // keeps the legacy prefix.
                out.push_str(&format!(
                    "  {}_drop(&",
                    hashmap_prefix_from_kv(k, v),
                ));
                out.push_str(&local_name(name));
                out.push_str(");\n");
            }
            Type::BTreeSet(_) => {
                out.push_str("  intent_btreeset_i64_drop(&");
                out.push_str(&local_name(name));
                out.push_str(");\n");
            }
            Type::BTreeMap(_, _) => {
                out.push_str("  intent_btreemap_i64_i64_drop(&");
                out.push_str(&local_name(name));
                out.push_str(");\n");
            }
            Type::UnionFind => {
                out.push_str("  intent_union_find_drop(&");
                out.push_str(&local_name(name));
                out.push_str(");\n");
            }
            Type::BinaryHeap(_) => {
                out.push_str("  intent_binary_heap_i64_drop(&");
                out.push_str(&local_name(name));
                out.push_str(");\n");
            }
            Type::BloomFilter => {
                out.push_str("  intent_bloom_filter_drop(&");
                out.push_str(&local_name(name));
                out.push_str(");\n");
            }
            Type::Bst(_) => {
                out.push_str("  intent_bst_i64_drop(&");
                out.push_str(&local_name(name));
                out.push_str(");\n");
            }
            Type::Graph => {
                out.push_str("  intent_graph_drop(&");
                out.push_str(&local_name(name));
                out.push_str(");\n");
            }
            Type::Trie => {
                out.push_str("  intent_trie_drop(&");
                out.push_str(&local_name(name));
                out.push_str(");\n");
            }
            Type::SkipList => {
                out.push_str("  intent_skiplist_i64_drop(&");
                out.push_str(&local_name(name));
                out.push_str(");\n");
            }
            Type::Pool(_) => {
                // Layer 2 of `unsafe.md` — affine handle: free
                // the three heap arrays (slots / generations /
                // free_list) at scope exit. The struct itself
                // is stack-allocated; only the arrays live on
                // the heap.
                out.push_str("  intent_pool_i64_drop(&");
                out.push_str(&local_name(name));
                out.push_str(");\n");
            }
            Type::Region => {
                // Layer 5 v2 foundation of `unsafe.md` — single
                // `free` on the bump buffer at scope exit.
                // Every allocation handed out by this region
                // becomes invalid simultaneously, by construction.
                out.push_str("  intent_region_drop(&");
                out.push_str(&local_name(name));
                out.push_str(");\n");
            }
            Type::Struct(struct_name) => {
                // Auto-call the user's `Drop` impl when one
                // exists. Two flavors:
                // * `fn drop(self: T)` (by-value) consumes the
                //   binding — valid only when the struct has
                //   no owning fields (otherwise the per-field
                //   pass would double-free what user-drop
                //   already consumed). This is the original
                //   T2.7 phase 2 shape.
                // * `fn drop(self: mut ref T)` (by-ref) runs
                //   user cleanup THEN the per-field pass.
                //   Epic C — unblocks user-Drop for structs
                //   that own OwnedStr / Vec / nested-struct
                //   fields. The user code can read/write
                //   field values; the per-field pass still
                //   reclaims the heap afterwards.
                let fields = STRUCT_FIELDS_REGISTRY
                    .with(|r| r.borrow().get(struct_name).cloned())
                    .unwrap_or_default();
                let has_user_drop = USER_DROP_REGISTRY
                    .with(|r| r.borrow().contains(struct_name));
                let user_drop_by_ref = crate::ast::user_drop_is_by_ref(struct_name);
                let has_owning_field = fields.iter().any(|(_, ty)| {
                    matches!(ty, Type::OwnedStr | Type::Vec(_) | Type::Struct(_))
                });
                if has_user_drop && user_drop_by_ref {
                    out.push_str("  (void)");
                    out.push_str(&function_name(&format!("{}_drop", struct_name)));
                    out.push_str("(&");
                    out.push_str(&local_name(name));
                    out.push_str(");\n");
                    // Fall through to the per-field drop pass.
                } else if has_user_drop && !has_owning_field {
                    out.push_str("  (void)");
                    out.push_str(&function_name(&format!("{}_drop", struct_name)));
                    out.push_str("(");
                    out.push_str(&local_name(name));
                    out.push_str(");\n");
                    // User drop consumed the value; skip the
                    // per-field free pass below.
                    return;
                }
                // Free every owning field of the struct.
                // OwnedStr fields free their heap buffer; Vec
                // fields go through the per-element-type
                // `intent_vec_<T>__free` helper. Stack-shaped
                // affine fields ([T;N], Task, Atomic) need no
                // runtime drop. Fields are freed in reverse
                // declaration order so destruction mirrors the
                // construction order (Rust's RAII convention).
                // Partial-moved fields are skipped — their
                // value is owned by another binding now.
                // T1.2 phase 2b.
                let moved: std::collections::HashSet<&String> = moved_fields.iter().collect();
                emit_struct_field_drops(
                    &local_name(name),
                    struct_name,
                    &fields,
                    &moved,
                    out,
                );
            }
            Type::Enum(enum_name) => {
                // Payloaded enums with a heap-shaped payload
                // free the payload when the active variant
                // matches. Closure #283: mixed-payload enums
                // route through per-variant `.u.v_<variant>`
                // access (one switch case per variant with
                // owning payload); single-payload enums keep
                // the legacy `.payload` path.
                let variant_payloads = ENUM_VARIANT_PAYLOADS_REGISTRY
                    .with(|r| r.borrow().get(enum_name).cloned());
                let is_mixed_local = variant_payloads.as_ref().map(|v| {
                    let payloads: Vec<&Type> =
                        v.iter().filter_map(|(_, p)| p.as_ref()).collect();
                    payloads.len() >= 2
                        && payloads[1..].iter().any(|t| *t != payloads[0])
                }).unwrap_or(false);
                let local = local_name(name);
                if is_mixed_local {
                    let variants = variant_payloads.unwrap();
                    let mut cases: Vec<String> = Vec::new();
                    for (tag, (vname, pty)) in variants.iter().enumerate() {
                        let Some(pty) = pty.as_ref() else { continue; };
                        let free_for_variant: Option<String> = match pty {
                            Type::OwnedStr => Some(format!(
                                "free((void*){}.u.{})",
                                local, enum_variant_member(vname)
                            )),
                            Type::Vec(element) => Some(format!(
                                "{}({}.u.{})",
                                vec_helper(element, "free"),
                                local, enum_variant_member(vname)
                            )),
                            _ => None,
                        };
                        if let Some(call) = free_for_variant {
                            cases.push(format!(
                                "case {}: {}; break;",
                                tag, call
                            ));
                        }
                    }
                    if !cases.is_empty() {
                        out.push_str(&format!(
                            "  switch ({}.tag) {{ {} default: break; }}\n",
                            local,
                            cases.join(" ")
                        ));
                    }
                    return;
                }
                let payload_ty = ENUM_PAYLOAD_REGISTRY
                    .with(|r| r.borrow().get(enum_name).cloned());
                let free_expr: Option<String> = match &payload_ty {
                    Some(Type::OwnedStr) => Some(format!(
                        "free((void*){}.payload)",
                        local
                    )),
                    Some(Type::Vec(element)) => Some(format!(
                        "{}({}.payload)",
                        vec_helper(element, "free"),
                        local
                    )),
                    _ => None,
                };
                if let Some(free_call) = free_expr {
                    let payload_tags: Vec<u32> =
                        ENUM_PAYLOAD_TAGS_REGISTRY.with(|r| {
                            r.borrow()
                                .get(enum_name)
                                .cloned()
                                .unwrap_or_default()
                        });
                    if !payload_tags.is_empty() {
                        let cases: Vec<String> = payload_tags
                            .iter()
                            .map(|t| format!("case {}", t))
                            .collect();
                        out.push_str(&format!(
                            "  switch ({}.tag) {{ {}: {}; break; default: break; }}\n",
                            local,
                            cases.join(": "),
                            free_call
                        ));
                    }
                }
            }
            Type::Array { element, length } => {
                // Closure #291 Phase 3 + 4: arrays of
                // non-Copy elements need per-slot drop at
                // scope exit. For Copy element types nothing
                // to do (no heap behind any slot).
                if element.is_copy() {
                    return;
                }
                let local = local_name(name);
                for i in 0..*length {
                    match element.as_ref() {
                        Type::Vec(inner) => {
                            out.push_str(&format!(
                                "  {}({}[{}]);\n",
                                vec_helper(inner, "free"),
                                local, i
                            ));
                        }
                        Type::OwnedStr => {
                            out.push_str(&format!(
                                "  free((void*){}[{}]);\n",
                                local, i
                            ));
                        }
                        Type::Struct(struct_name) => {
                            // Phase 4 (closure #291): walk
                            // each slot's owning fields. The
                            // slot expression is `{local}[i]`;
                            // reuse `emit_struct_field_drops`
                            // which understands the field
                            // registry + per-field free shapes.
                            let fields = STRUCT_FIELDS_REGISTRY
                                .with(|r| r.borrow().get(struct_name).cloned())
                                .unwrap_or_default();
                            let empty: std::collections::HashSet<&String> =
                                std::collections::HashSet::new();
                            let slot_expr = format!("{}[{}]", local, i);
                            emit_struct_field_drops(
                                &slot_expr,
                                struct_name,
                                &fields,
                                &empty,
                                out,
                            );
                        }
                        _ => {
                            // Nested-array / tuple / enum
                            // element types are rare in
                            // practice; if a real test
                            // surfaces, mirror the Struct
                            // arm with the appropriate
                            // per-slot drop sequence.
                        }
                    }
                }
            }
            _ => {
                // Other affine types (Task, Atomic,
                // Channel, Mutex — all stack-allocated structs
                // without heap-owned buffers) emit no runtime
                // drop.
            }
        },
        TypedStmt::Discard { expr } => match &expr.ty {
            Type::Vec(element) => {
                // Bind to a brace-scoped tmp so we can free the buffer. The
                // brace-scope means consecutive `let _ = ...` don't collide.
                let struct_name = vec_c_struct(element);
                out.push_str("  {\n    ");
                out.push_str(&struct_name);
                out.push_str(" _intent_discard = ");
                out.push_str(&emit_expr(expr));
                out.push_str(";\n    ");
                out.push_str(&vec_helper(element, "free"));
                out.push_str("(_intent_discard);\n  }\n");
            }
            Type::OwnedStr => {
                // Closure #134: `let _ = make_owned_str();` must
                // free the returned heap string, otherwise the
                // allocation leaks. Bind to a brace-scoped tmp
                // so consecutive discards don't collide.
                out.push_str("  {\n    char* _intent_discard = ");
                out.push_str(&emit_expr(expr));
                out.push_str(";\n    free((void*)_intent_discard);\n  }\n");
            }
            Type::Array { element, length } => {
                // Arrays have stack lifetime. Still materialize the RHS into
                // a brace-scoped tmp so its side-effecting subexpressions
                // run; C disallows casting an array directly to void.
                out.push_str("  {\n    ");
                if let TypedExprKind::ArrayLit { elements } = &expr.kind {
                    let element_strs: Vec<String> = elements.iter().map(emit_expr).collect();
                    out.push_str(c_leaf_type(element));
                    out.push(' ');
                    out.push_str("_intent_discard[");
                    out.push_str(&length.to_string());
                    out.push_str("] = { ");
                    out.push_str(&element_strs.join(", "));
                    out.push_str(" };\n    (void)_intent_discard;\n  }\n");
                } else {
                    out.push_str(c_leaf_type(element));
                    out.push_str(" _intent_discard[");
                    out.push_str(&length.to_string());
                    out.push_str("];\n    memcpy(_intent_discard, ");
                    out.push_str(&emit_expr(expr));
                    out.push_str(", sizeof(_intent_discard));\n    (void)_intent_discard;\n  }\n");
                }
            }
            Type::Struct(struct_name) => {
                // Closure #145: `let _ = make_struct();` for a
                // struct with heap-shaped fields (OwnedStr,
                // Vec<T>, nested Struct with owning fields)
                // was leaking the per-field heap. Bind to a
                // brace-scoped tmp, walk the struct's fields,
                // and emit the same per-field free chain the
                // scope-exit Drop pass uses. Struct without
                // owning fields → just `(void)(...)`.
                //
                // Closure #277: also fire the user's `Drop`
                // impl when present. Mirrors the `TypedStmt::
                // Drop` arm for `Type::Struct`. Without this,
                // `let _ = make();` silently skipped user-
                // declared cleanup even though end-of-scope
                // drop ran it correctly.
                let fields = STRUCT_FIELDS_REGISTRY
                    .with(|r| r.borrow().get(struct_name).cloned())
                    .unwrap_or_default();
                let has_owning = fields.iter().any(|(_, ty)| {
                    !ty.is_copy()
                });
                let has_user_drop = USER_DROP_REGISTRY
                    .with(|r| r.borrow().contains(struct_name));
                let user_drop_by_ref = crate::ast::user_drop_is_by_ref(struct_name);
                // By-value user-Drop with no owning fields:
                // user-drop consumes the binding; no per-field
                // pass needed.
                if has_user_drop && !user_drop_by_ref && !has_owning {
                    out.push_str("  (void)");
                    out.push_str(&function_name(&format!("{}_drop", struct_name)));
                    out.push_str("(");
                    out.push_str(&emit_expr(expr));
                    out.push_str(");\n");
                    return;
                }
                if has_owning || has_user_drop {
                    out.push_str("  {\n    ");
                    out.push_str(&struct_c_name(struct_name));
                    out.push_str(" _intent_discard = ");
                    out.push_str(&emit_expr(expr));
                    out.push_str(";\n");
                    if has_user_drop && user_drop_by_ref {
                        out.push_str("    (void)");
                        out.push_str(&function_name(&format!(
                            "{}_drop",
                            struct_name
                        )));
                        out.push_str("(&_intent_discard);\n");
                    }
                    let empty: std::collections::HashSet<&String> =
                        std::collections::HashSet::new();
                    emit_struct_field_drops(
                        "_intent_discard",
                        struct_name,
                        &fields,
                        &empty,
                        out,
                    );
                    out.push_str("  }\n");
                } else {
                    out.push_str("  (void)(");
                    out.push_str(&emit_expr(expr));
                    out.push_str(");\n");
                }
            }
            Type::Enum(enum_name) => {
                // Closure #146: `let _ = make_enum();` for an
                // enum with a heap-shaped payload (OwnedStr,
                // Vec<T>) was leaking. Mirror the scope-exit
                // Drop logic from `TypedStmt::Drop`'s
                // `Type::Enum` arm: bind to a brace-scoped
                // tmp, switch on the tag, and free the
                // payload for variants that carry one.
                let payload_ty = ENUM_PAYLOAD_REGISTRY
                    .with(|r| r.borrow().get(enum_name).cloned());
                let free_expr: Option<String> = match &payload_ty {
                    Some(Type::OwnedStr) => Some(
                        "free((void*)_intent_discard.payload)".to_string(),
                    ),
                    Some(Type::Vec(element)) => Some(format!(
                        "{}(_intent_discard.payload)",
                        vec_helper(element, "free")
                    )),
                    _ => None,
                };
                if let Some(free_call) = free_expr {
                    let payload_tags: Vec<u32> =
                        ENUM_PAYLOAD_TAGS_REGISTRY.with(|r| {
                            r.borrow()
                                .get(enum_name)
                                .cloned()
                                .unwrap_or_default()
                        });
                    if !payload_tags.is_empty() {
                        let cases: Vec<String> = payload_tags
                            .iter()
                            .map(|t| format!("case {}", t))
                            .collect();
                        out.push_str("  {\n    ");
                        out.push_str(&enum_c_name(enum_name));
                        out.push_str(" _intent_discard = ");
                        out.push_str(&emit_expr(expr));
                        out.push_str(";\n");
                        out.push_str(&format!(
                            "    switch (_intent_discard.tag) {{ {}: {}; break; default: break; }}\n  }}\n",
                            cases.join(": "),
                            free_call
                        ));
                    } else {
                        out.push_str("  (void)(");
                        out.push_str(&emit_expr(expr));
                        out.push_str(");\n");
                    }
                } else {
                    out.push_str("  (void)(");
                    out.push_str(&emit_expr(expr));
                    out.push_str(");\n");
                }
            }
            _ => {
                out.push_str("  (void)(");
                out.push_str(&emit_expr(expr));
                out.push_str(");\n");
            }
        },
        TypedStmt::Return { expr } => {
            // Closure #239: when returning an array-typed
            // value, wrap it in the per-shape struct wrapper.
            // `return [1, 2, 3];` for an `[i64; 3]` return
            // type becomes `return (intent_arr_ret_3_int64_t){
            //   .data = {1, 2, 3}};` so C can pass it by value.
            if let Type::Array { element, length } = &expr.ty {
                let wrapper = array_return_struct_name(element, *length);
                // Inline-array-literal path emits the elements
                // directly into the .data initializer. For
                // any other shape (e.g. a Var referencing a
                // local array), use a memcpy through a stack
                // temp to materialize the struct value.
                if let TypedExprKind::ArrayLit { elements } = &expr.kind {
                    let parts: Vec<String> = elements.iter().map(emit_expr).collect();
                    out.push_str(&format!(
                        "  return ({}){{ .data = {{ {} }} }};\n",
                        wrapper,
                        parts.join(", ")
                    ));
                } else {
                    let elem_storage = c_element_storage(element);
                    out.push_str(&format!(
                        "  {{ {} __intent_ret_data[{}]; \
memcpy(__intent_ret_data, ({}), sizeof(__intent_ret_data)); \
{} __intent_ret = {{0}}; \
memcpy(__intent_ret.data, __intent_ret_data, sizeof(__intent_ret_data)); \
return __intent_ret; }}\n",
                        elem_storage,
                        length,
                        emit_expr(expr),
                        wrapper,
                    ));
                }
                return;
            }
            out.push_str("  return ");
            out.push_str(&emit_expr(expr));
            out.push_str(";\n");
        }
        TypedStmt::Assert { expr, message } => {
            // C `assert` macro stringifies its sole argument. To emit a
            // custom message, fall back to `if (!cond) { fprintf(stderr,...);
            // abort(); }` which keeps the same abort-on-failure shape.
            if let Some(msg) = message {
                out.push_str("  if (!(");
                out.push_str(&emit_expr(expr));
                out.push_str(")) { fprintf(stderr, \"assertion failed: ");
                out.push_str(&escape_c_string(msg));
                out.push_str("\\n\"); abort(); }\n");
            } else {
                out.push_str("  assert(");
                out.push_str(&emit_expr(expr));
                out.push_str(");\n");
            }
        }
        TypedStmt::Prove { expr } => {
            out.push_str("  /* proven by compiler: ");
            out.push_str(&escape_comment(&emit_expr(expr)));
            out.push_str(" */\n");
        }
        TypedStmt::Print { items } => emit_print_items(items, out),
        TypedStmt::If {
            cond,
            then_body,
            else_body,
        } => {
            out.push_str("  if (");
            out.push_str(&emit_expr(cond));
            out.push_str(") {\n");
            for s in then_body {
                emit_stmt(s, out);
            }
            out.push_str("  }");
            if !else_body.is_empty() {
                out.push_str(" else {\n");
                for s in else_body {
                    emit_stmt(s, out);
                }
                out.push_str("  }");
            }
            out.push('\n');
        }
        TypedStmt::While { cond, body } => {
            out.push_str("  while (");
            out.push_str(&emit_expr(cond));
            out.push_str(") {\n");
            for s in body {
                emit_stmt(s, out);
            }
            out.push_str("  }\n");
        }
        TypedStmt::Break => {
            out.push_str("  break;\n");
        }
        TypedStmt::Continue => {
            out.push_str("  continue;\n");
        }
        TypedStmt::IndexAssign {
            name,
            base_ty,
            index,
            field_path,
            value,
            checked,
        } => emit_index_assign(name, base_ty, index, field_path, value, *checked, out),
        TypedStmt::FieldAssign {
            object,
            field,
            through_mut_ref,
            value,
            ..
        } => {
            // C emit: `obj.field = value;` for owned struct
            // values, `obj->field = value;` for `mut ref`
            // borrows (typed-AST `RefMut` collapses to a
            // pointer in C codegen — see field-access
            // emission). T1.2 phase 2a follow-up.
            //
            // Heap-shaped field overwrite: when the field
            // type is OwnedStr or Vec<T>, the previous slot's
            // resources must be freed before the new value
            // is stored, otherwise the old allocation leaks.
            // Mirrors the leaf-Drop logic in `emit_index_assign`
            // (closure #126 / F2). Closure #132.
            let obj = emit_expr(object);
            let v = emit_expr(value);
            let op = if *through_mut_ref { "->" } else { "." };
            let lvalue = format!("{}{}{}", obj, op, field);
            match &value.ty {
                Type::OwnedStr => {
                    out.push_str(&format!("  free((void*){});\n", lvalue));
                }
                Type::Vec(element) => {
                    out.push_str(&format!(
                        "  {}({});\n",
                        vec_helper(element, "free"),
                        lvalue
                    ));
                }
                Type::Struct(struct_name) => {
                    // Closure #148: assigning a struct-typed
                    // field (`t.inner = newInner`) must free
                    // the previous struct's owning fields,
                    // otherwise the nested heap leaks.
                    let fields = STRUCT_FIELDS_REGISTRY
                        .with(|r| r.borrow().get(struct_name).cloned())
                        .unwrap_or_default();
                    let has_owning = fields.iter().any(|(_, ty)| !ty.is_copy());
                    if has_owning {
                        let empty: std::collections::HashSet<&String> =
                            std::collections::HashSet::new();
                        emit_struct_field_drops(
                            &lvalue,
                            struct_name,
                            &fields,
                            &empty,
                            out,
                        );
                    }
                }
                Type::Enum(enum_name) => {
                    // Closure #148: assigning a payloaded-enum-
                    // typed field must free the previous
                    // payload heap. Same shape as the Reassign
                    // enum case.
                    let payload_ty = ENUM_PAYLOAD_REGISTRY
                        .with(|r| r.borrow().get(enum_name).cloned());
                    let free_expr: Option<String> = match &payload_ty {
                        Some(Type::OwnedStr) => Some(format!(
                            "free((void*){}.payload)",
                            lvalue
                        )),
                        Some(Type::Vec(element)) => Some(format!(
                            "{}({}.payload)",
                            vec_helper(element, "free"),
                            lvalue
                        )),
                        _ => None,
                    };
                    if let Some(free_call) = free_expr {
                        let payload_tags: Vec<u32> =
                            ENUM_PAYLOAD_TAGS_REGISTRY.with(|r| {
                                r.borrow()
                                    .get(enum_name)
                                    .cloned()
                                    .unwrap_or_default()
                            });
                        if !payload_tags.is_empty() {
                            let cases: Vec<String> = payload_tags
                                .iter()
                                .map(|t| format!("case {}", t))
                                .collect();
                            out.push_str(&format!(
                                "  switch ({}.tag) {{ {}: {}; break; default: break; }}\n",
                                lvalue,
                                cases.join(": "),
                                free_call
                            ));
                        }
                    }
                }
                _ => {}
            }
            // Phase 3f — C array-to-array assignment is invalid
            // (arrays decay to pointers in `=` position). For
            // array-typed fields, route through memcpy instead.
            // Required by the v3.1 transform's NonSuspendLet
            // pattern when an `[T; N]` local is hoisted into the
            // task struct (`__t->arr = __v3_tmp_arr;` would
            // otherwise fail to compile).
            if matches!(value.ty, Type::Array { .. }) {
                out.push_str(&format!(
                    "  memcpy(&{}, &{}, sizeof({}));\n",
                    lvalue, v, lvalue
                ));
            } else {
                out.push_str(&format!("  {} = {};\n", lvalue, v));
            }
        }
        TypedStmt::For {
            var,
            ty,
            start,
            end,
            body,
            parallel,
            reductions,
        } => {
            let local = local_name(var);
            let c_ty = c_leaf_type(ty);
            if *parallel {
                // Effects verifier has proven the body is pure
                // (no shared mutable state, no I/O, no consuming
                // mutator calls); reductions are carved out via
                // the `reduction(op:var)` clause so OpenMP gives
                // each thread a private partial and combines.
                // Compilers without `-fopenmp` issue an "unknown
                // pragma" warning and fall back to sequential —
                // also correct.
                let mut pragma = String::from("omp parallel for");
                for r in reductions {
                    pragma.push_str(&format!(
                        " reduction({}:{})",
                        r.op.display_symbol(),
                        local_name(&r.var)
                    ));
                }
                out.push_str(&format!("  _Pragma(\"{}\")\n", pragma));
            }
            out.push_str(&format!(
                "  for ({0} {1} = {2}; {1} < {3}; {1}++) {{\n",
                c_ty,
                local,
                emit_expr(start),
                emit_expr(end)
            ));
            for s in body {
                emit_stmt(s, out);
            }
            out.push_str("  }\n");
        }
        TypedStmt::ForIter {
            var,
            element_ty,
            collection,
            collection_ty,
            consumes,
            body,
        } => emit_for_iter(
            var,
            element_ty,
            collection,
            collection_ty,
            *consumes,
            body,
            out,
        ),
        TypedStmt::TaskSpawn { name, body, captures } => {
            // Spawn the task on a real pthread. Allocate a
            // per-spawn outline ID, emit the outline + ctx
            // struct into the module-scope TASK_OUTLINES
            // buffer, and at the spawn site malloc +
            // populate the ctx, then call pthread_create.
            let id = TASK_OUTLINE_COUNTER.with(|c| {
                let n = c.get();
                c.set(n + 1);
                n
            });
            let struct_name = format!("intent_task_{}_ctx", id);
            let outline_fn = format!("intent_task_{}", id);
            // Build the outline + struct typedef in a side
            // buffer.
            let mut outline = String::new();
            outline.push_str(&format!("typedef struct {} {{\n", struct_name));
            for (cap_name, cap_ty) in captures {
                outline.push_str(&format!(
                    "  {};\n",
                    format_declarator(cap_ty, &format!("cap_{}", cap_name))
                ));
            }
            outline.push_str(&format!("}} {};\n\n", struct_name));
            outline.push_str(&format!(
                "static void* {}(void* _ctx_raw) {{\n",
                outline_fn
            ));
            outline.push_str(&format!(
                "  {}* ctx = ({}*)_ctx_raw;\n",
                struct_name, struct_name
            ));
            // Locals re-aliasing the ctx fields so the body's
            // emit (which uses local_name(...) for variables)
            // sees the captures as ordinary locals.
            for (cap_name, cap_ty) in captures {
                outline.push_str(&format!(
                    "  {} = ctx->cap_{};\n",
                    format_declarator(cap_ty, &local_name(cap_name)),
                    cap_name
                ));
            }
            for s in body {
                emit_stmt(s, &mut outline);
            }
            outline.push_str("  return (void*)0;\n");
            outline.push_str("}\n\n");
            TASK_OUTLINES.with(|b| b.borrow_mut().push_str(&outline));

            // Spawn-site code: allocate the ctx, populate
            // each capture, build the handle, fire
            // pthread_create.
            out.push_str(&format!(
                "  intent_task_handle {};\n",
                local_name(name)
            ));
            out.push_str(&format!(
                "  {}* _intent_ctx_{} = ({}*)malloc(sizeof({}));\n",
                struct_name, id, struct_name, struct_name
            ));
            for (cap_name, _) in captures {
                out.push_str(&format!(
                    "  _intent_ctx_{}->cap_{} = {};\n",
                    id,
                    cap_name,
                    local_name(cap_name)
                ));
            }
            out.push_str(&format!(
                "  intent_thread_create(&{}.thread, {}, _intent_ctx_{});\n",
                local_name(name),
                outline_fn,
                id
            ));
            out.push_str(&format!(
                "  {}.ctx = _intent_ctx_{};\n",
                local_name(name),
                id
            ));
        }
        TypedStmt::TaskJoin { name } => {
            // Real-thread join: block until the worker
            // exits and free the heap-allocated ctx struct.
            out.push_str(&format!(
                "  intent_thread_join({}.thread);\n",
                local_name(name)
            ));
            out.push_str(&format!("  free({}.ctx);\n", local_name(name)));
        }
        TypedStmt::UnsafeBlock { reason, body } => {
            // Layer 1.1 of unsafe.md. The reason string is the
            // user-facing justification, escaped here for C
            // block-comment safety (close-comment sequence and
            // newlines stripped at parse time). Emitted as both
            // an inline block-comment marker AND accumulated
            // into the per-translation-unit deviation table
            // (TASK_OUTLINES-style side buffer) so a
            // certification-tooling pass can dump every
            // deviation site with its reason. The body is
            // emitted unchanged — at Layer 1.1 the unsafe
            // boundary is purely metadata; raw pointer types
            // and Tainted<T> propagation land in Layers 1.2 and
            // 1.3 and will be guarded by the existing parse-
            // time enforcement that raw types appear only
            // inside this scope.
            out.push_str("  /* UNSAFE-DEVIATION: ");
            out.push_str(&escape_comment(reason));
            out.push_str(" */\n");
            out.push_str("  {\n");
            for s in body {
                emit_stmt(s, out);
            }
            out.push_str("  }\n");
        }
    }
}

fn emit_for_iter(
    var: &str,
    element_ty: &Type,
    collection: &str,
    collection_ty: &Type,
    consumes: bool,
    body: &[TypedStmt],
    out: &mut String,
) {
    let idx = format!("_intent_idx_{}", var);
    let elem_local = local_name(var);
    // Phase 11 (2026-06-07): the collection name may be a
    // dotted path (`obj.field`, `obj.f1.f2`) when the for-loop
    // iterated a struct field. Build the C accessor by
    // local_name-ing the head and chaining `.field` literals
    // (always with `.`, since we deref pointer-typed heads via
    // an explicit `(*head)` prefix). Lifts L7 from
    // docs/v1_limitations.md.
    //
    // Because we don't have the head's type info at this site
    // (we only have collection_ty which is the FIELD's type),
    // we conservatively emit `(*head).field` if the head's
    // C-mangled name starts with `v_` AND the dotted form is
    // present. This works uniformly: in v1 the only way to get
    // a dotted path through `for-iter` is from a method's
    // `self: ref T` borrowing the field, so the head is a
    // pointer in C.
    let coll_local: String = if collection.contains('.') {
        let mut parts = collection.split('.');
        let head = parts.next().unwrap();
        let head_local = local_name(head);
        let rest: Vec<&str> = parts.collect();
        format!("(*{}).{}", head_local, rest.join("."))
    } else {
        local_name(collection)
    };
    let underlying = collection_ty.deref();
    let is_ref = collection_ty.is_any_ref();

    // (length_expr, element_access)
    let (length_expr, elem_access) = match underlying {
        Type::Array { length, .. } => {
            (format!("{}", length), format!("{}[{}]", coll_local, idx))
        }
        Type::Vec(_) => {
            let prefix = if is_ref {
                format!("(*{})", coll_local)
            } else {
                coll_local.clone()
            };
            (
                format!("{}.len", prefix),
                format!("{}.data[{}]", prefix, idx),
            )
        }
        _ => return, // checker rejects other cases
    };

    out.push_str(&format!(
        "  for (uint64_t {0} = 0; {0} < {1}; {0}++) {{\n",
        idx, length_expr
    ));
    // Use the element's full storage spelling (handles
    // `Vec<U>` aggregates via the per-type typedef alias).
    // Was emitting `"/* vec */"` for nested Vec elements.
    // Refines #7 phase 2.
    out.push_str(&format!(
        "    {} {} = {};\n",
        c_element_storage(element_ty),
        elem_local,
        elem_access
    ));
    for s in body {
        emit_stmt(s, out);
    }
    out.push_str("  }\n");

    // Consuming iteration owns the source for the duration of the loop.
    // For owned `Vec<T>`, the buffer must be freed when the loop exits.
    // Arrays have stack lifetime so no free is needed.
    //
    // For non-Copy elements, each slot was loaded into `x` and freed
    // by x's scope-exit drop in the body. Routing through
    // `intent_vec_<T>__free` here would re-walk every slot
    // (closure #127's per-element drop) and double-free. Skip the
    // helper and emit only the outer buffer free.
    if consumes && !is_ref {
        if let Type::Vec(element) = underlying {
            if element.is_copy() {
                out.push_str(&format!(
                    "  {}({});\n",
                    vec_helper(element, "free"),
                    coll_local
                ));
            } else {
                out.push_str(&format!(
                    "  free({}.data);\n",
                    coll_local
                ));
            }
        }
    }
}

/// Emit per-field free calls for a struct binding at the
/// given C path (e.g. `v_o` or `v_o.inner`). Recursively
/// descends into nested struct fields. Heap fields
/// (OwnedStr, Vec) emit a free; nested struct fields recurse;
/// other field types are no-ops. Fields are walked in
/// reverse declaration order (Rust RAII convention).
/// T1.2 phase 2b + D2.
fn emit_struct_field_drops(
    path: &str,
    struct_name: &str,
    fields: &[(String, Type)],
    moved: &std::collections::HashSet<&String>,
    out: &mut String,
) {
    for (field_name, field_ty) in fields.iter().rev() {
        if moved.contains(field_name) {
            continue;
        }
        match field_ty {
            Type::OwnedStr => {
                out.push_str("  free((void*)");
                out.push_str(path);
                out.push('.');
                out.push_str(field_name);
                out.push_str(");\n");
            }
            Type::Vec(element) => {
                out.push_str("  ");
                out.push_str(&vec_helper(element, "free"));
                out.push('(');
                out.push_str(path);
                out.push('.');
                out.push_str(field_name);
                out.push_str(");\n");
            }
            // L2 Phase 1 (2026-06-07): Box<T> field drop. The
            // struct's scope-exit drop chains into the field's
            // free() so heap slots owned by Box-typed fields are
            // released along with the outer struct.
            // L2 Phase 3 (2026-06-08): Box<dyn Iface> field
            // owns its `.data` heap slot; free that rather than
            // the field address.
            // L2 follow-up (2026-06-08): Box<Vec<T>> / Box<OwnedStr>
            // field — chain into the inner type's Drop before
            // freeing the box slot itself.
            Type::Box(box_inner) => match &**box_inner {
                Type::Object(_) => {
                    out.push_str("  free(");
                    out.push_str(path);
                    out.push('.');
                    out.push_str(field_name);
                    out.push_str(".data);\n");
                }
                Type::Vec(element) => {
                    out.push_str("  ");
                    out.push_str(&vec_helper(element, "free"));
                    out.push_str("(*");
                    out.push_str(path);
                    out.push('.');
                    out.push_str(field_name);
                    out.push_str(");\n");
                    out.push_str("  free(");
                    out.push_str(path);
                    out.push('.');
                    out.push_str(field_name);
                    out.push_str(");\n");
                }
                Type::OwnedStr => {
                    out.push_str("  free((void*)*");
                    out.push_str(path);
                    out.push('.');
                    out.push_str(field_name);
                    out.push_str(");\n");
                    out.push_str("  free(");
                    out.push_str(path);
                    out.push('.');
                    out.push_str(field_name);
                    out.push_str(");\n");
                }
                _ => {
                    out.push_str("  free(");
                    out.push_str(path);
                    out.push('.');
                    out.push_str(field_name);
                    out.push_str(");\n");
                }
            },
            Type::Struct(inner_name) => {
                // Recurse into the nested struct's fields.
                let inner_fields = STRUCT_FIELDS_REGISTRY
                    .with(|r| r.borrow().get(inner_name).cloned())
                    .unwrap_or_default();
                if !inner_fields.is_empty() {
                    let inner_path = format!("{}.{}", path, field_name);
                    let empty: std::collections::HashSet<&String> =
                        std::collections::HashSet::new();
                    emit_struct_field_drops(
                        &inner_path,
                        inner_name,
                        &inner_fields,
                        &empty,
                        out,
                    );
                }
            }
            _ => {}
        }
    }
    let _ = struct_name; // reserved for future per-struct diagnostics
}

fn emit_index_assign(
    name: &str,
    base_ty: &Type,
    index: &TypedExpr,
    field_path: &[(String, u32)],
    value: &TypedExpr,
    checked: bool,
    out: &mut String,
) {
    let local = local_name(name);
    let index_str = emit_expr(index);
    let value_str = emit_expr(value);

    // Build the per-field suffix once: `.field1.field2…`.
    // Empty for plain `xs[i] = v;`. T1.2 phase 2b follow-up.
    let field_suffix: String = field_path
        .iter()
        .map(|(name, _)| format!(".{}", name))
        .collect();

    let underlying = base_ty.deref();
    let through_ref = base_ty.is_ref_mut();

    let element_ty: Option<Type> = match underlying {
        Type::Array { element, .. } => Some((**element).clone()),
        Type::Vec(element) => Some((**element).clone()),
        _ => None,
    };

    // Resolve the leaf field type for the field_path (if any).
    // If the leaf is a heap-shaped field (OwnedStr / Vec<T>),
    // we must Drop the old slot value before overwriting it,
    // otherwise the previous heap allocation leaks. The Copy
    // gate in the checker permits this only at the leaf
    // position; intermediate segments stay Copy. F2 / #126.
    let leaf_ty: Option<Type> = element_ty.as_ref().and_then(|el| {
        let mut cur = el.clone();
        for (seg, _) in field_path {
            let Type::Struct(struct_name) = &cur else {
                return None;
            };
            let fields = STRUCT_FIELDS_REGISTRY
                .with(|r| r.borrow().get(struct_name).cloned())
                .unwrap_or_default();
            let next = fields.iter().find(|(n, _)| n == seg).map(|(_, t)| t.clone());
            cur = next?;
        }
        Some(cur)
    });

    // Build the lvalue prefix and slot index expression for
    // the chosen container shape. The lvalue used for the
    // pre-Drop free MUST match the lvalue used for the store,
    // so we compute it once.
    let (slot_lvalue, store_line): (Option<String>, String) = match underlying {
        Type::Array { length, .. } => {
            let idx_expr = if checked {
                format!("intent_check_bounds((uint64_t)({}), {})", index_str, length)
            } else {
                index_str.clone()
            };
            let lv = format!("{}[{}]{}", local, idx_expr, field_suffix);
            let store = format!("  {} = {};\n", lv, value_str);
            (Some(lv), store)
        }
        Type::Vec(_) => {
            let prefix = if through_ref {
                format!("(*{})", local)
            } else {
                local.clone()
            };
            let idx_expr = if checked {
                format!(
                    "intent_check_bounds((uint64_t)({}), {}.len)",
                    index_str, prefix
                )
            } else {
                format!("(uint64_t)({})", index_str)
            };
            let lv = format!("{}.data[{}]{}", prefix, idx_expr, field_suffix);
            let store = format!("  {} = {};\n", lv, value_str);
            (Some(lv), store)
        }
        _ => (
            None,
            format!("  /* unsupported index-assign target for {} */\n", base_ty),
        ),
    };

    if let (Some(lv), Some(lty)) = (slot_lvalue.as_ref(), leaf_ty.as_ref()) {
        // Mixed-place leaf drop (closure #126 / F2): when the
        // assignment writes through a field path, free the OLD
        // leaf field's heap before storing the new value.
        if !field_path.is_empty() {
            match lty {
                Type::OwnedStr => {
                    out.push_str(&format!("  free((void*){});\n", lv));
                }
                Type::Vec(elem) => {
                    out.push_str(&format!("  {}({});\n", vec_helper(elem, "free"), lv));
                }
                _ => {}
            }
        } else {
            // Whole-element overwrite (closure #149 / #150):
            // `xs[i] = newval` for ANY heap-shaped element
            // must free the OLD slot's heap before the store.
            // Previously only the field_path != [] case was
            // handled at the leaf level, so several
            // whole-element shapes leaked.
            match lty {
                Type::OwnedStr => {
                    // Closure #150: `Vec<OwnedStr>[i] = "x" + "y"`
                    // — free the old i8* before storing the new.
                    out.push_str(&format!("  free((void*){});\n", lv));
                }
                Type::Vec(elem) => {
                    // Closure #150: `Vec<Vec<i64>>[i] = vec(…)`
                    // — call the inner __free over the old
                    // slot before storing the new struct.
                    out.push_str(&format!(
                        "  {}({});\n",
                        vec_helper(elem, "free"),
                        lv
                    ));
                }
                Type::Struct(struct_name) => {
                    let fields = STRUCT_FIELDS_REGISTRY
                        .with(|r| r.borrow().get(struct_name).cloned())
                        .unwrap_or_default();
                    let has_owning = fields.iter().any(|(_, ty)| !ty.is_copy());
                    if has_owning {
                        let empty: std::collections::HashSet<&String> =
                            std::collections::HashSet::new();
                        emit_struct_field_drops(
                            lv,
                            struct_name,
                            &fields,
                            &empty,
                            out,
                        );
                    }
                }
                Type::Enum(enum_name) => {
                    let payload_ty = ENUM_PAYLOAD_REGISTRY
                        .with(|r| r.borrow().get(enum_name).cloned());
                    let free_expr: Option<String> = match &payload_ty {
                        Some(Type::OwnedStr) => Some(format!(
                            "free((void*){}.payload)",
                            lv
                        )),
                        Some(Type::Vec(element)) => Some(format!(
                            "{}({}.payload)",
                            vec_helper(element, "free"),
                            lv
                        )),
                        _ => None,
                    };
                    if let Some(free_call) = free_expr {
                        let payload_tags: Vec<u32> =
                            ENUM_PAYLOAD_TAGS_REGISTRY.with(|r| {
                                r.borrow()
                                    .get(enum_name)
                                    .cloned()
                                    .unwrap_or_default()
                            });
                        if !payload_tags.is_empty() {
                            let cases: Vec<String> = payload_tags
                                .iter()
                                .map(|t| format!("case {}", t))
                                .collect();
                            out.push_str(&format!(
                                "  switch ({}.tag) {{ {}: {}; break; default: break; }}\n",
                                lv,
                                cases.join(": "),
                                free_call
                            ));
                        }
                    }
                }
                _ => {}
            }
        }
    }

    out.push_str(&store_line);
}

/// Emit a `print item1, item2, …;` statement. Each item is printed
/// without a newline; a single space separates adjacent items; a
/// final newline terminates the line.
fn emit_print_items(items: &[crate::ir::TypedPrintItem], out: &mut String) {
    use crate::ir::TypedPrintItem;
    for (i, item) in items.iter().enumerate() {
        match item {
            TypedPrintItem::Str(s) => {
                // fputs doesn't append a newline; perfect for the
                // mid-line case.
                out.push_str("  fputs(\"");
                out.push_str(&escape_c_string(s));
                out.push_str("\", stdout);\n");
            }
            TypedPrintItem::Expr(expr) => emit_print_expr_no_newline(expr, out),
        }
        if i + 1 < items.len() {
            out.push_str("  fputs(\" \", stdout);\n");
        }
    }
    out.push_str("  putchar('\\n');\n");
}

fn emit_print_expr_no_newline(expr: &TypedExpr, out: &mut String) {
    match &expr.ty {
        Type::Bool => {
            out.push_str("  fputs(");
            out.push_str(&emit_expr(expr));
            out.push_str(" ? \"true\" : \"false\", stdout);\n");
        }
        Type::U8 | Type::U16 | Type::U32 | Type::U64 => {
            out.push_str("  printf(\"%llu\", (unsigned long long)(");
            out.push_str(&emit_expr(expr));
            out.push_str("));\n");
        }
        Type::F32 | Type::F64 => {
            out.push_str("  printf(\"%g\", (double)(");
            out.push_str(&emit_expr(expr));
            out.push_str("));\n");
        }
        Type::Str => {
            out.push_str("  fputs(");
            out.push_str(&emit_expr(expr));
            out.push_str(", stdout);\n");
        }
        Type::OwnedStr => {
            // Conservative whitelist: only Call returning
            // OwnedStr (intent_str_concat / user fn) and
            // Binary `+` (string concat) are guaranteed-fresh
            // heap-producers in v1. Var / FieldAccess /
            // TupleAccess reference a value owned by some
            // binding (whose scope-exit Drop frees the heap)
            // — freeing after print would double-free. Bind
            // to a brace-scoped tmp so the free has a stable
            // handle and consecutive prints don't collide.
            // Closure #135.
            let is_fresh = crate::ir::is_fresh_owned_str(expr);
            if is_fresh {
                out.push_str("  {\n    char* _intent_print_tmp = ");
                out.push_str(&emit_expr(expr));
                out.push_str(";\n");
                out.push_str("    fputs(_intent_print_tmp, stdout);\n");
                out.push_str("    free((void*)_intent_print_tmp);\n");
                out.push_str("  }\n");
            } else {
                out.push_str("  fputs(");
                out.push_str(&emit_expr(expr));
                out.push_str(", stdout);\n");
            }
        }
        Type::Array { .. } | Type::Vec(_) => {
            out.push_str("  /* aggregate print not supported */\n");
        }
        _ => {
            // Phase 1.1 (2026-06-07): when the source file
            // declared a Devanagari dialect via `// vani-lang:`,
            // route integer prints through the
            // `intent_print_int_dev` helper which emits
            // Devanagari digit codepoints (०..९ at U+0966..96F).
            // Phase 5b (2026-06-07): Bengali dialect routes
            // through `intent_print_int_ben` (০..৯ at
            // U+09E6..9EF) — same shape, different codepoints.
            // Phase 6 (2026-06-07): one suffix per Brahmi script.
            // The helper-emit gate above ensures only the selected
            // script's helper is present in the module.
            let suffix = match crate::lexer::current_print_lang_mode() {
                crate::lexer::PrintLangMode::Devanagari => Some("dev"),
                crate::lexer::PrintLangMode::Bengali => Some("ben"),
                crate::lexer::PrintLangMode::Tamil => Some("tam"),
                crate::lexer::PrintLangMode::Telugu => Some("tel"),
                crate::lexer::PrintLangMode::Gujarati => Some("guj"),
                crate::lexer::PrintLangMode::Gurmukhi => Some("pan"),
                crate::lexer::PrintLangMode::Kannada => Some("kan"),
                crate::lexer::PrintLangMode::Malayalam => Some("mal"),
                crate::lexer::PrintLangMode::Odia => Some("odi"),
                crate::lexer::PrintLangMode::Sinhala => Some("sin"),
                crate::lexer::PrintLangMode::Urdu => Some("urd"),
                crate::lexer::PrintLangMode::Persian => Some("per"),
                crate::lexer::PrintLangMode::Ascii => None,
            };
            if let Some(s) = suffix {
                out.push_str(&format!("  intent_print_int_{}((long long)(", s));
                out.push_str(&emit_expr(expr));
                out.push_str("));\n");
            } else {
                out.push_str("  printf(\"%lld\", (long long)(");
                out.push_str(&emit_expr(expr));
                out.push_str("));\n");
            }
        }
    }
}

fn emit_expr(expr: &TypedExpr) -> String {
    match &expr.kind {
        TypedExprKind::Int(value) => value.to_string(),
        TypedExprKind::Float(value) => emit_float_literal(*value, &expr.ty),
        TypedExprKind::Bool(value) => {
            if *value {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        TypedExprKind::Str(text) => format!("\"{}\"", escape_c_string(text)),
        TypedExprKind::Var(name) => local_name(name),
        TypedExprKind::Unary { op, expr } => {
            let symbol = match op {
                UnaryOp::Neg => "-",
                UnaryOp::Not => "!",
            };
            format!("({}{})", symbol, emit_expr(expr))
        }
        TypedExprKind::Binary { op, left, right, checked } => {
            emit_binary(*op, left, right, *checked, &expr.ty)
        }
        TypedExprKind::Call { name, args, .. } => emit_call(name, args, &expr.ty),
        TypedExprKind::Cast { expr, ty } => {
            // Raw-pointer-target casts (`&x as *const T`,
            // `&mut x as *mut T`) — Layer 1.1+ of `unsafe.md`.
            // The target type's C spelling has a `*` after the
            // pointee, so the cast operator wraps `(const T*)`
            // or `(T*)` around the operand. The standard
            // `c_leaf_type` path returns the
            // `/* *const T */` placeholder for these — handle
            // them here directly.
            let cast_spell = match ty {
                Type::Ptr(inner) => {
                    let inner_decl = format_declarator(inner, "").trim_end().to_string();
                    format!("const {}*", inner_decl)
                }
                Type::PtrMut(inner) => {
                    let inner_decl = format_declarator(inner, "").trim_end().to_string();
                    format!("{}*", inner_decl)
                }
                _ => c_leaf_type(ty).to_string(),
            };
            format!("(({})({}))", cast_spell, emit_expr(expr))
        }
        TypedExprKind::ArrayLit { elements } => {
            let array_ty = match &expr.ty {
                Type::Array { element, length } => format!("{}[{}]", c_leaf_type(element), length),
                _ => "/* array */".to_string(),
            };
            let parts: Vec<String> = elements.iter().map(emit_expr).collect();
            format!("(({}){{ {} }})", array_ty, parts.join(", "))
        }
        TypedExprKind::Index {
            array,
            index,
            checked,
        } => emit_index(array, index, *checked),
        TypedExprKind::Len { array, length } => emit_len(array, *length),
        TypedExprKind::Ref { name } | TypedExprKind::RefMut { name } => {
            // For arrays, C array-decay means just passing the name works.
            // For Vecs and primitives, take the address.
            let inner_ty = match &expr.ty {
                Type::Ref(inner) | Type::RefMut(inner) => inner,
                _ => unreachable!("Ref/RefMut TypedExpr must have ref type"),
            };
            match &**inner_ty {
                Type::Array { .. } => local_name(name),
                _ => format!("&{}", local_name(name)),
            }
        }
        TypedExprKind::RefField { object, field, object_ty, .. }
        | TypedExprKind::RefMutField { object, field, object_ty, .. } => {
            // `ref t.x` / `mut ref t.x` — take the address of
            // the struct field. C array-decay applies the same
            // way as for plain `Ref { name }`: passing
            // `v_t.field` works without `&` for array fields.
            // When `object` is bound to a ref-typed value
            // (e.g. `self: ref T` in a method body) we use
            // `v_t->field` since v_t is a pointer. Closure
            // #165.
            let inner_ty = match &expr.ty {
                Type::Ref(inner) | Type::RefMut(inner) => inner,
                _ => unreachable!("RefField/RefMutField must have ref type"),
            };
            let sep = if object_ty.is_any_ref() { "->" } else { "." };
            // Closure #210: when the object is borrowed via
            // `ref T` (read-only borrow), the C parameter is
            // `const T*`, so `&v_t->field` would be
            // `const FieldType*`. For Mutex/Atomic/Channel
            // fields, the helper functions
            // (`intent_mutex_i64_lock` etc.) take non-const
            // pointers — atomic-style ops conceptually
            // mutate even via a read-only borrow. Without a
            // const-strip cast, cc warns
            // `-Wdiscarded-qualifiers`. Closure #176
            // already handled the analogous shape for direct
            // `ref Mutex<T>` / `ref Channel<T,N>` / `ref
            // Atomic<T>` params; #210 covers field-borrow
            // through `ref Struct`.
            let needs_const_strip = object_ty.is_ref()
                && matches!(
                    &**inner_ty,
                    Type::Mutex(_) | Type::Atomic(_) | Type::Channel(_, _)
                );
            match &**inner_ty {
                Type::Array { .. } => format!("{}{}{}", local_name(object), sep, field),
                _ if needs_const_strip => {
                    let storage = match &**inner_ty {
                        Type::Mutex(_) => "intent_mutex_i64".to_string(),
                        Type::Atomic(element) => c_atomic_storage(element),
                        Type::Channel(element, capacity) => {
                            c_channel_storage(element, *capacity)
                        }
                        _ => unreachable!(),
                    };
                    format!(
                        "({}*)&{}{}{}",
                        storage,
                        local_name(object),
                        sep,
                        field
                    )
                }
                _ => format!("&{}{}{}", local_name(object), sep, field),
            }
        }
        TypedExprKind::FnRef { name, .. } => {
            // C function names decay to function pointers
            // when used in non-call positions, so emitting the
            // bare prefixed identifier just works.
            function_name(name)
        }
        TypedExprKind::CallIndirect { callee, args } => {
            // Arc 5c: when the callee is Closure-typed, lower
            // as `c.call(c.env, args...)` — dispatch through
            // the embedded fn-pointer with the env prepended.
            // FnPtr-typed callees use the simple indirect form
            // (function pointers auto-deref at call).
            let callee_c = emit_expr(callee);
            let arg_parts: Vec<String> = args.iter().map(emit_expr).collect();
            if matches!(callee.ty, Type::Closure(_, _)) {
                let mut all_args: Vec<String> = vec![format!("{}.env", callee_c)];
                all_args.extend(arg_parts);
                format!("{}.call({})", callee_c, all_args.join(", "))
            } else {
                format!("{}({})", callee_c, arg_parts.join(", "))
            }
        }
        TypedExprKind::Tuple { elements } => {
            // `(intent_tuple_<shape>){ ._0 = …, ._1 = …, … }`
            // designated-initializer form. The struct typedef is
            // emitted in the preamble's `emit_tuple_bundle` pass.
            // Refines T1.1 phase 2.
            let elem_tys: Vec<Type> = elements.iter().map(|e| e.ty.clone()).collect();
            let struct_name = tuple_c_struct(&elem_tys);
            let parts: Vec<String> = elements
                .iter()
                .enumerate()
                .map(|(i, e)| format!("._{} = {}", i, emit_expr(e)))
                .collect();
            format!("({}){{ {} }}", struct_name, parts.join(", "))
        }
        TypedExprKind::TupleAccess { tuple, index } => {
            let inner = emit_expr(tuple);
            format!("({})._{}", inner, index)
        }
        TypedExprKind::StructLit { type_name, fields } => {
            // `(Struct_<Name>){ .field1 = …, .field2 = … }`
            // designated-initializer compound literal. T1.2.
            // Array-typed fields with an inline `[…]` ArrayLit
            // initializer use a bare-brace `{e1, e2, …}` form
            // since C forbids assigning a compound-literal-array
            // to a struct member of array type. T1.2 phase 2b.
            let parts: Vec<String> = fields
                .iter()
                .map(|(n, e)| {
                    let rhs = match (&e.ty, &e.kind) {
                        (Type::Array { .. }, TypedExprKind::ArrayLit { elements }) => {
                            let parts: Vec<String> = elements.iter().map(emit_expr).collect();
                            format!("{{ {} }}", parts.join(", "))
                        }
                        _ => emit_expr(e),
                    };
                    format!(".{} = {}", n, rhs)
                })
                .collect();
            format!("({}){{ {} }}", struct_c_name(type_name), parts.join(", "))
        }
        TypedExprKind::FieldAccess { object, field, .. } => {
            // Through-a-borrow access uses `->`; by-value
            // uses `.`. Distinguish via the operand's type.
            let inner = emit_expr(object);
            if object.ty.is_any_ref() {
                format!("({})->{}", inner, field)
            } else {
                format!("({}).{}", inner, field)
            }
        }
        TypedExprKind::EnumVariant { enum_name, tag, .. } => {
            // Plain (payload-less) variant: just the tag.
            // Payloaded enum's payload-less variant: build a
            // tagged-union struct with `.tag` set and the
            // payload field zero-initialized. For mixed-payload
            // enums (closure #283) the payload sits inside a
            // `.u` union; the payload-less variant just sets
            // the tag and leaves the union zeroed. Aggregate
            // payload types need brace-init.
            let payloaded = ENUM_PAYLOAD_REGISTRY.with(|r| r.borrow().contains_key(enum_name));
            if payloaded {
                let is_mixed = ENUM_VARIANT_PAYLOADS_REGISTRY.with(|r| {
                    r.borrow().get(enum_name).map(|v| {
                        let payloads: Vec<&Type> =
                            v.iter().filter_map(|(_, p)| p.as_ref()).collect();
                        payloads.len() >= 2
                            && payloads[1..].iter().any(|t| *t != payloads[0])
                    }).unwrap_or(false)
                });
                if is_mixed {
                    // Mixed-payload: just set the tag; the
                    // union's storage is zero-initialized
                    // implicitly by the absent designator.
                    return format!(
                        "(({}){{ .tag = (int32_t){} }})",
                        enum_c_name(enum_name),
                        tag,
                    );
                }
                let payload_ty = ENUM_PAYLOAD_REGISTRY
                    .with(|r| r.borrow().get(enum_name).cloned())
                    .expect("just checked payloaded");
                let payload_zero = match &payload_ty {
                    Type::Vec(_) | Type::Tuple(_) | Type::Struct(_) | Type::Array { .. } => "{0}",
                    _ => "0",
                };
                format!(
                    "(({}){{ .tag = (int32_t){}, .payload = {} }})",
                    enum_c_name(enum_name),
                    tag,
                    payload_zero
                )
            } else {
                format!("((int32_t){})", tag)
            }
        }
        TypedExprKind::EnumVariantWithPayload { enum_name, tag, payload, .. } => {
            // T1.3 phase 2b: build the tagged-union struct
            // literal with both `.tag` and `.payload` set.
            // Array payloads need a bare-brace `{e1, e2, …}`
            // initializer since C forbids assigning a
            // compound-literal array into a struct field of
            // array type. Same fix as struct fields in
            // closure #100. Closure #119.
            let payload_str = match (&payload.ty, &payload.kind) {
                (Type::Array { .. }, TypedExprKind::ArrayLit { elements }) => {
                    let parts: Vec<String> = elements.iter().map(emit_expr).collect();
                    format!("{{ {} }}", parts.join(", "))
                }
                _ => emit_expr(payload),
            };
            // Closure #283: mixed-payload-type enums store
            // the payload through a per-variant union member
            // (`.u.v_<variant>`). Single-payload-type enums
            // keep the legacy `.payload` field for back-
            // compat.
            let is_mixed = ENUM_VARIANT_PAYLOADS_REGISTRY.with(|r| {
                r.borrow().get(enum_name).map(|v| {
                    let payloads: Vec<&Type> =
                        v.iter().filter_map(|(_, p)| p.as_ref()).collect();
                    payloads.len() >= 2
                        && payloads[1..].iter().any(|t| *t != payloads[0])
                }).unwrap_or(false)
            });
            if is_mixed {
                // Look up the variant name from the
                // per-variant registry by tag.
                let variant_name = ENUM_VARIANT_PAYLOADS_REGISTRY.with(|r| {
                    r.borrow().get(enum_name).and_then(|v| {
                        v.get(*tag as usize).map(|(n, _)| n.clone())
                    })
                }).unwrap_or_else(|| format!("tag{}", tag));
                let member = enum_variant_member(&variant_name);
                return format!(
                    "(({}){{ .tag = (int32_t){}, .u = {{ .{} = {} }} }})",
                    enum_c_name(enum_name),
                    tag,
                    member,
                    payload_str
                );
            }
            format!(
                "(({}){{ .tag = (int32_t){}, .payload = {} }})",
                enum_c_name(enum_name),
                tag,
                payload_str
            )
        }
        TypedExprKind::Match { scrutinee, arms } => {
            // GCC statement-expression: switch on the tag,
            // materialize each arm's value into a fresh
            // temp, yield the temp. Exhaustiveness is
            // checker-enforced; if there's no wildcard arm
            // the default aborts so out-of-spec values trip
            // loudly. With a wildcard arm, the default
            // branch *is* its body. T1.3 (wildcard).
            // Use `c_type_name` so payloaded-enum result
            // types render as `Enum_<Name>` rather than the
            // bare `int32_t` tag (the latter would mismatch
            // the arm bodies' struct literals when the match
            // returns a payloaded enum). Closure #130
            // (`try` follow-up + Match-on-Enum-result C
            // codegen fix).
            let result_ty = c_type_name(&expr.ty);
            // T1.3 phase 2b: detect whether scrutinee is a
            // payloaded enum so dispatch can use `.tag` and
            // payload bindings can be extracted via `.payload`.
            //
            // Phase 11 (2026-06-07): if the scrutinee is a
            // `ref T` / `mut ref T`, the C value is a pointer.
            // Dereference once before reading .tag / .payload.
            // Lifts L3 from docs/v1_limitations.md.
            let effective_scrut_ty = match &scrutinee.ty {
                Type::Ref(inner) | Type::RefMut(inner) => (**inner).clone(),
                _ => scrutinee.ty.clone(),
            };
            let scrut_is_ref = matches!(
                &scrutinee.ty,
                Type::Ref(_) | Type::RefMut(_),
            );
            let scrutinee_payloaded = match &effective_scrut_ty {
                Type::Enum(name) => {
                    ENUM_PAYLOAD_REGISTRY.with(|r| r.borrow().contains_key(name))
                }
                _ => false,
            };
            let scr_full_raw = emit_expr(scrutinee);
            let scr_full = if scrut_is_ref {
                format!("(*({}))", scr_full_raw)
            } else {
                scr_full_raw
            };
            let mut body = String::new();
            // For payloaded enums, materialize the scrutinee
            // into a fresh local so we can read both .tag (for
            // dispatch) and .payload (for binding) without
            // re-evaluating the source expression.
            let dispatch = if scrutinee_payloaded {
                let enum_name = match &effective_scrut_ty {
                    Type::Enum(n) => n,
                    _ => unreachable!(),
                };
                body.push_str(&format!(
                    "{} __scr = ({}); ",
                    enum_c_name(enum_name),
                    scr_full
                ));
                "__scr.tag".to_string()
            } else if matches!(&effective_scrut_ty, Type::Bool) {
                // Closure #205: gcc warns
                // `switch condition has boolean value` (-Wswitch-bool)
                // when the dispatch expression is bool-typed. Cast
                // to int so the switch's `case 0` / `case 1` arms
                // dispatch on the canonical 0/1 representation
                // without the warning.
                format!("((int){})", scr_full)
            } else {
                scr_full
            };
            body.push_str(&format!("{} __r; ", result_ty));
            body.push_str(&format!("switch ({}) {{ ", dispatch));
            let mut wildcard_body: Option<String> = None;
            for arm in arms {
                if arm.is_wildcard {
                    let arm_v = emit_expr(&arm.body);
                    wildcard_body = Some(arm_v);
                    continue;
                }
                // For VariantWithBinding patterns, emit a fresh
                // scoped block that declares the local binding
                // initialized from the payload (legacy
                // `.payload` for single-type, or `.u.v_<variant>`
                // for mixed-payload — closure #283).
                let arm_block = if let Some((bname, bty)) = &arm.binding {
                    let body_v = emit_expr(&arm.body);
                    // Phase 11 (2026-06-07): use the dereffed
                    // scrutinee type if the original was a ref.
                    let scrutinee_enum_name = match &effective_scrut_ty {
                        Type::Enum(n) => Some(n.clone()),
                        _ => None,
                    };
                    let payload_access = if let Some(enum_n) = &scrutinee_enum_name {
                        let is_mixed = ENUM_VARIANT_PAYLOADS_REGISTRY.with(|r| {
                            r.borrow().get(enum_n).map(|v| {
                                let payloads: Vec<&Type> =
                                    v.iter().filter_map(|(_, p)| p.as_ref()).collect();
                                payloads.len() >= 2
                                    && payloads[1..].iter().any(|t| *t != payloads[0])
                            }).unwrap_or(false)
                        });
                        if is_mixed {
                            let variant_name = ENUM_VARIANT_PAYLOADS_REGISTRY.with(|r| {
                                r.borrow().get(enum_n).and_then(|v| {
                                    v.get(arm.tag as usize).map(|(n, _)| n.clone())
                                })
                            }).unwrap_or_else(|| format!("tag{}", arm.tag));
                            format!("__scr.u.{}", enum_variant_member(&variant_name))
                        } else {
                            "__scr.payload".to_string()
                        }
                    } else {
                        "__scr.payload".to_string()
                    };
                    format!(
                        "{{ {} v_{} = {}; __r = ({}); }}",
                        c_type_name(bty),
                        bname,
                        payload_access,
                        body_v
                    )
                } else {
                    let body_v = emit_expr(&arm.body);
                    format!("__r = ({});", body_v)
                };
                if let Some(int_v) = arm.int_value {
                    body.push_str(&format!(
                        "case {}: {} break; ",
                        int_v, arm_block
                    ));
                } else {
                    body.push_str(&format!(
                        "case {}: {} break; ",
                        arm.tag, arm_block
                    ));
                }
            }
            match wildcard_body {
                Some(w) => body.push_str(&format!("default: __r = ({}); break; ", w)),
                None => body.push_str("default: abort(); "),
            }
            body.push_str("} __r; ");
            format!("({{ {}}})", body)
        }
        TypedExprKind::IfExpr { cond, then_value, else_value } => {
            // Plain C ternary — branches are always single
            // expressions so this is unambiguous. T4
            // (if-as-expression).
            let c = emit_expr(cond);
            let t = emit_expr(then_value);
            let e = emit_expr(else_value);
            format!("(({}) ? ({}) : ({}))", c, t, e)
        }
        TypedExprKind::Block { stmts, tail } => {
            // GCC statement-expression form: `({ T a = e1;
            // T b = e2; print …; tail; })`. The tail's value
            // is the last evaluated sub-expression. T-block.
            // V1 admits Let + Print stmts; the checker rejects
            // anything else from user-written blocks. Synthetic
            // blocks (e.g. the `match-str` desugar) can also
            // include `Drop` stmts so the temp scrutinee gets
            // released after the if-chain evaluates. Closure
            // #137.
            let mut body = String::from("({ ");
            for s in stmts {
                match s {
                    TypedStmt::Let { name, ty, expr: rhs } => {
                        body.push_str(&format!(
                            "{} v_{} = ({}); ",
                            c_type_name(ty),
                            name,
                            emit_expr(rhs)
                        ));
                    }
                    TypedStmt::Print { items } => {
                        emit_print_items(items, &mut body);
                    }
                    TypedStmt::Reassign { name, expr: rhs, .. } => {
                        // Block-expr Reassign: simple stored-
                        // value update. Mirrors the stmt-level
                        // Reassign emit for the trivial case.
                        body.push_str(&format!(
                            "v_{} = ({}); ",
                            name,
                            emit_expr(rhs),
                        ));
                    }
                    TypedStmt::While { cond, body: while_body } => {
                        // Block-expr While loop (closure #238).
                        // Emit `while (cond) { body }` inside the
                        // GCC statement-expression. Inner body
                        // currently restricted to Assign / Print
                        // by the Block-expr checker, both of
                        // which round-trip through the top-level
                        // stmt emitter cleanly.
                        body.push_str(&format!("while (({})) {{ ", emit_expr(cond)));
                        for inner in while_body {
                            match inner {
                                TypedStmt::Reassign { name, expr: rhs, .. } => {
                                    body.push_str(&format!(
                                        "v_{} = ({}); ",
                                        name,
                                        emit_expr(rhs),
                                    ));
                                }
                                TypedStmt::Print { items } => {
                                    emit_print_items(items, &mut body);
                                }
                                _ => {
                                    body.push_str("/* unsupported while-body stmt */ ");
                                }
                            }
                        }
                        body.push_str("} ");
                    }
                    TypedStmt::Discard { expr: discard_expr } => {
                        // Closure #200: `let _ = expr;` inside
                        // a Block-expr. Evaluate the RHS for
                        // side effects and free any heap result.
                        // Brace-scope each so consecutive
                        // discards don't collide on the tmp
                        // name. Mirrors the regular stmt-level
                        // discard handling (closures #134, #145,
                        // #146).
                        let rhs_c = emit_expr(discard_expr);
                        match &discard_expr.ty {
                            Type::OwnedStr => {
                                body.push_str(&format!(
                                    "{{ char* _intent_discard = ({}); free((void*)_intent_discard); }} ",
                                    rhs_c
                                ));
                            }
                            Type::Vec(element) => {
                                let s_name = vec_c_struct(element);
                                body.push_str(&format!(
                                    "{{ {} _intent_discard = ({}); {}(_intent_discard); }} ",
                                    s_name,
                                    rhs_c,
                                    vec_helper(element, "free"),
                                ));
                            }
                            Type::Struct(struct_name) => {
                                let fields = STRUCT_FIELDS_REGISTRY
                                    .with(|r| r.borrow().get(struct_name).cloned())
                                    .unwrap_or_default();
                                let has_owning =
                                    fields.iter().any(|(_, ty)| !ty.is_copy());
                                if has_owning {
                                    let mut field_drops = String::new();
                                    let empty: std::collections::HashSet<&String> =
                                        std::collections::HashSet::new();
                                    emit_struct_field_drops(
                                        "_intent_discard",
                                        struct_name,
                                        &fields,
                                        &empty,
                                        &mut field_drops,
                                    );
                                    body.push_str(&format!(
                                        "{{ {} _intent_discard = ({}); {}}} ",
                                        struct_c_name(struct_name),
                                        rhs_c,
                                        field_drops,
                                    ));
                                } else {
                                    body.push_str(&format!("(void)({}); ", rhs_c));
                                }
                            }
                            Type::Enum(enum_name) => {
                                let payload_ty = ENUM_PAYLOAD_REGISTRY
                                    .with(|r| r.borrow().get(enum_name).cloned());
                                let free_expr: Option<String> = match &payload_ty {
                                    Some(Type::OwnedStr) => Some(
                                        "free((void*)_intent_discard.payload)".to_string(),
                                    ),
                                    Some(Type::Vec(element)) => Some(format!(
                                        "{}(_intent_discard.payload)",
                                        vec_helper(element, "free")
                                    )),
                                    _ => None,
                                };
                                if let Some(free_call) = free_expr {
                                    let payload_tags: Vec<u32> = ENUM_PAYLOAD_TAGS_REGISTRY
                                        .with(|r| {
                                            r.borrow()
                                                .get(enum_name)
                                                .cloned()
                                                .unwrap_or_default()
                                        });
                                    if !payload_tags.is_empty() {
                                        let cases: Vec<String> = payload_tags
                                            .iter()
                                            .map(|t| format!("case {}", t))
                                            .collect();
                                        body.push_str(&format!(
                                            "{{ {} _intent_discard = ({}); switch (_intent_discard.tag) {{ {}: {}; break; default: break; }} }} ",
                                            format!("Enum_{}", enum_name),
                                            rhs_c,
                                            cases.join(": "),
                                            free_call,
                                        ));
                                    } else {
                                        body.push_str(&format!("(void)({}); ", rhs_c));
                                    }
                                } else {
                                    body.push_str(&format!("(void)({}); ", rhs_c));
                                }
                            }
                            _ => {
                                // Copy / scalar / non-heap discards:
                                // just evaluate for side effects.
                                body.push_str(&format!("(void)({}); ", rhs_c));
                            }
                        }
                    }
                    TypedStmt::Drop { name, ty, .. } => match ty {
                        Type::OwnedStr => {
                            body.push_str(&format!(
                                "free((void*){}); ",
                                local_name(name)
                            ));
                        }
                        Type::Vec(element) => {
                            body.push_str(&format!(
                                "{}({}); ",
                                vec_helper(element, "free"),
                                local_name(name)
                            ));
                        }
                        Type::Struct(struct_name) => {
                            // Closure #192: Block-expr Drop
                            // for a struct binding with
                            // heap-owning fields. The
                            // inject_branch_drops machinery
                            // (closure #179) wraps if-expr /
                            // match arms with Drops for the
                            // OTHER branches' Vars. For
                            // Struct-typed Vars the per-field
                            // free chain has to run; previously
                            // this arm fell through to `_ =>
                            // {}` and the unchosen branch's
                            // heap leaked.
                            let fields = STRUCT_FIELDS_REGISTRY
                                .with(|r| r.borrow().get(struct_name).cloned())
                                .unwrap_or_default();
                            // Closure #207: if the struct has a
                            // user-declared `implement Drop for T`
                            // AND no owning fields, the auto-
                            // call invokes the user's drop method.
                            // Mirrors the regular stmt-level
                            // Struct Drop arm (lines 1965-1987).
                            // Without this, a Block-expr inner
                            // Let of a Copy-but-user-Drop struct
                            // (e.g. `Resource` with only an
                            // i64 field plus `implement Drop`)
                            // silently skipped the user drop at
                            // scope exit.
                            let has_user_drop = USER_DROP_REGISTRY
                                .with(|r| r.borrow().contains(struct_name));
                            let has_owning_field = fields.iter().any(|(_, ty)| {
                                matches!(ty, Type::OwnedStr | Type::Vec(_))
                            });
                            if has_user_drop && !has_owning_field {
                                body.push_str(&format!(
                                    "(void){}({}); ",
                                    function_name(&format!("{}_drop", struct_name)),
                                    local_name(name),
                                ));
                                continue;
                            }
                            let empty: std::collections::HashSet<&String> =
                                std::collections::HashSet::new();
                            // emit_struct_field_drops appends
                            // to `out` with `  ` indent + `\n`
                            // suffix per line. Inside the
                            // statement-expression we strip
                            // newlines so it stays one inline
                            // sequence.
                            let mut tmp = String::new();
                            emit_struct_field_drops(
                                &local_name(name),
                                struct_name,
                                &fields,
                                &empty,
                                &mut tmp,
                            );
                            for line in tmp.lines() {
                                let trimmed = line.trim_start();
                                if !trimmed.is_empty() {
                                    body.push_str(trimmed);
                                    body.push(' ');
                                }
                            }
                        }
                        Type::Enum(enum_name) => {
                            // Closure #193: parallel to the
                            // Struct arm. Block-expr Drop for
                            // a payloaded enum needs to switch
                            // on the active tag and free the
                            // heap payload — otherwise the
                            // unchosen branch's payload leaks.
                            let payload_ty = ENUM_PAYLOAD_REGISTRY
                                .with(|r| r.borrow().get(enum_name).cloned());
                            let free_expr: Option<String> = match &payload_ty {
                                Some(Type::OwnedStr) => Some(format!(
                                    "free((void*){}.payload)",
                                    local_name(name)
                                )),
                                Some(Type::Vec(element)) => Some(format!(
                                    "{}({}.payload)",
                                    vec_helper(element, "free"),
                                    local_name(name)
                                )),
                                _ => None,
                            };
                            if let Some(free_call) = free_expr {
                                let payload_tags: Vec<u32> =
                                    ENUM_PAYLOAD_TAGS_REGISTRY.with(|r| {
                                        r.borrow()
                                            .get(enum_name)
                                            .cloned()
                                            .unwrap_or_default()
                                    });
                                if !payload_tags.is_empty() {
                                    let cases: Vec<String> = payload_tags
                                        .iter()
                                        .map(|t| format!("case {}", t))
                                        .collect();
                                    body.push_str(&format!(
                                        "switch ({}.tag) {{ {}: {}; break; default: break; }} ",
                                        local_name(name),
                                        cases.join(": "),
                                        free_call,
                                    ));
                                }
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
            body.push_str(&format!("({}); }})", emit_expr(tail)));
            body
        }
        TypedExprKind::DynDispatch {
            receiver, iface_name: _, method: _, slot_index, args, ..
        } => {
            // Vtables Phase 3 (tree-C) + 4c: dispatch through
            // the fat pointer's vtable slot. For an owned
            // receiver `(recv).vtable->m<slot>((recv).data)`.
            // For a borrowed receiver (`ref dyn Iface` /
            // `mut ref dyn Iface`) the C value is a pointer
            // to the fat pointer, so deref via `(*recv)` first.
            let recv_c = emit_expr(receiver);
            let recv_lval = match receiver.ty {
                Type::Ref(_) | Type::RefMut(_) => format!("(*({}))", recv_c),
                _ => format!("({})", recv_c),
            };
            let mut arg_parts: Vec<String> = Vec::with_capacity(args.len() + 1);
            arg_parts.push(format!("{}.data", recv_lval));
            for a in args {
                arg_parts.push(emit_expr(a));
            }
            format!(
                "({}.vtable->m{}({}))",
                recv_lval, slot_index, arg_parts.join(", ")
            )
        }
        TypedExprKind::DynCoerce { value, iface_name, from_type_name, from_ty: _ } => {
            // Vtables Phase 3 (tree-C): materialize the fat
            // pointer literal. The data slot must hold the
            // address of a stable lvalue. Var sources point at
            // the binding's stack address (lives for the
            // enclosing block). Non-Var sources are pre-hoisted
            // into a synthetic let by the checker pass — by
            // the time codegen runs, every DynCoerce should
            // have a Var source. Closure #276.
            match &value.kind {
                TypedExprKind::Var(name) => {
                    format!(
                        "((intent_dyn_{iface}){{ .vtable = &intent_vtbl_{iface}_{ty}, .data = (void*)&{lvalue} }})",
                        iface = iface_name,
                        ty = from_type_name,
                        lvalue = local_name(name),
                    )
                }
                _ => unreachable!(
                    "DynCoerce non-Var source reached codegen; the checker's \
                     synthetic-let hoist should have rewritten it. iface={}",
                    iface_name
                ),
            }
        }
    }
}

/// Per-shape C struct name for a tuple type. Mirrors
/// `vec_c_struct` — the elements' tags get concatenated
/// with `_` so distinct shapes never collide. T1.1.
pub(crate) fn tuple_c_struct(elements: &[Type]) -> String {
    let tags: Vec<String> = elements.iter().map(element_tag).collect();
    format!("intent_tuple_{}", tags.join("_"))
}

/// Emit the typedef for a tuple shape (`typedef struct { … }
/// intent_tuple_<shape>;`). Each element becomes a numbered
/// field `_0`, `_1`, … so `.0` / `.1` access in source
/// lowers to `._0` / `._1`. Called from the preamble after
/// `emit_array_typedefs_for` so any nested array element
/// typedefs are already in scope.
pub(crate) fn emit_tuple_bundle(elements: &[Type], out: &mut String) {
    let struct_name = tuple_c_struct(elements);
    out.push_str(&format!("typedef struct {{\n"));
    for (i, ty) in elements.iter().enumerate() {
        let storage = c_element_storage(ty);
        out.push_str(&format!("    {} _{};\n", storage, i));
    }
    out.push_str(&format!("}} {};\n", struct_name));
}

/// ARC 1.4e helper: derive the per-(K, V) HashMap bundle prefix
/// from the HashMap type itself. The (i64, i64) case maps to the
/// legacy bundle name so existing callers don't need migration.
fn hashmap_prefix_from_kv(k: &Type, v: &Type) -> String {
    let k_tag = hashmap_type_tag_c_owned(k);
    let v_tag = hashmap_type_tag_c_owned(v);
    // Legacy (i64, i64) bundle keeps its original name.
    if matches!(k, Type::I64) && matches!(v, Type::I64) {
        return "intent_hashmap_i64_i64".to_string();
    }
    format!("intent_hashmap_{}_{}", k_tag, v_tag)
}

/// Resolve the bundle prefix from a result type (used for
/// `hashmap_new`, which has no receiver arg). Falls back to the
/// legacy prefix if the type isn't HashMap (shouldn't happen in
/// well-typed programs).
fn hashmap_prefix_from_ty(ty: &Type) -> String {
    match ty {
        Type::HashMap(k, v) => hashmap_prefix_from_kv(k, v),
        _ => "intent_hashmap_i64_i64".to_string(),
    }
}

/// Resolve the bundle prefix from a receiver arg type (after
/// stripping the Ref/RefMut wrapper). Used for the other
/// hashmap_* builtins whose first arg is the map.
fn hashmap_prefix_from_recv(ty: &Type) -> String {
    let inner = match ty {
        Type::Ref(inner) | Type::RefMut(inner) => inner.as_ref(),
        other => other,
    };
    hashmap_prefix_from_ty(inner)
}

/// Per-type C-leaf tag for use in HashMap bundle names. Mirrors
/// the C-leaf type spelling so the emitter produces matching
/// identifiers. Returns owned String because struct K paths
/// synthesize names dynamically.
fn hashmap_type_tag_c_owned(ty: &Type) -> String {
    match ty {
        Type::I8 => "int8_t".to_string(),
        Type::I16 => "int16_t".to_string(),
        Type::I32 => "int32_t".to_string(),
        Type::I64 => "int64_t".to_string(),
        Type::U8 => "uint8_t".to_string(),
        Type::U16 => "uint16_t".to_string(),
        Type::U32 => "uint32_t".to_string(),
        Type::U64 => "uint64_t".to_string(),
        Type::Bool => "bool".to_string(),
        // ARC 4.5: f64 K — bundle prefix `intent_hashmap_double_<V>`.
        Type::F64 => "double".to_string(),
        // ARC 4.1: OwnedStr K — bundle prefix
        // `intent_hashmap_owned_str_<V>`. Map owns each key
        // pointer; FNV-1a byte hash + strcmp equality.
        Type::OwnedStr => "owned_str".to_string(),
        // ARC 1.7: struct K — tag matches the C-emitted struct
        // typedef name (`Struct_<name>`).
        Type::Struct(name) => format!("Struct_{}", name),
        // ARC 4.4: tuple K — tag flattens to `tup_<n>_i64` for an
        // n-element i64 tuple, so the bundle prefix stays
        // identifier-safe. Keeps shape distinct from
        // `intent_tuple_<…>` (the standard tuple typedef name)
        // so the two strings can coexist in the same translation
        // unit without collision.
        Type::Tuple(els) if els.iter().all(|t| matches!(t, Type::I64)) => {
            format!("tup_{}_i64", els.len())
        }
        // ARC 4.6: Vec<i64> K — bundle prefix
        // `intent_hashmap_vec_int64_t_<V>`. Keys field stores
        // the existing `intent_vec_int64_t` typedef by value.
        Type::Vec(inner) if matches!(inner.as_ref(), Type::I64) => {
            "vec_int64_t".to_string()
        }
        _ => "i64".to_string(),
    }
}

#[allow(dead_code)]
fn hashmap_type_tag_c(ty: &Type) -> &'static str {
    match ty {
        Type::I8 => "int8_t",
        Type::I16 => "int16_t",
        Type::I32 => "int32_t",
        Type::I64 => "int64_t",
        Type::U8 => "uint8_t",
        Type::U16 => "uint16_t",
        Type::U32 => "uint32_t",
        Type::U64 => "uint64_t",
        Type::Bool => "bool",
        _ => "i64",
    }
}

fn emit_call(name: &str, args: &[TypedExpr], result_ty: &Type) -> String {
    match name {
        // L2 Phase 1 (2026-06-07): box(x) heap-allocates a slot
        // for x and returns a Box<T>. Lowered to a GCC compound
        // statement expression: `({ T* __b = malloc(sizeof(T));
        // *__b = (x); __b; })`. The result type is Box<T>; we
        // extract T to size the allocation and cast the pointer.
        // Use `c_type_name` for the spelling (knows about
        // `Struct_<Name>`, etc.); `c_leaf_type` returns a
        // `/* struct */` placeholder for nominal types.
        "__box_new" if args.len() == 1 => {
            let inner_ty = match result_ty {
                Type::Box(inner) => &**inner,
                _ => unreachable!("__box_new's result type must be Box<T>"),
            };
            // L2 Phase 3 (2026-06-08): Box<dyn Iface>. The
            // argument is a DynCoerce node — extract the
            // concrete type + iface name, heap-allocate the
            // CONCRETE (not the fat pointer), and emit the
            // owning fat pointer literal. The Box<dyn Iface>
            // local is itself the 16-byte fat pointer struct
            // (NOT a pointer to one) with `.data` pointing
            // into the heap. Drop frees `.data`.
            if let Type::Object(iface) = inner_ty {
                if let TypedExprKind::DynCoerce { value, from_type_name, .. } = &args[0].kind {
                    // The DynCoerce source must be a Var by
                    // the checker's synthetic-let hoist
                    // invariant (closure #276); the concrete
                    // value lives in a stack alloca that we
                    // need to copy into the heap slot.
                    let src_name = match &value.kind {
                        TypedExprKind::Var(n) => n.clone(),
                        _ => unreachable!(
                            "Box<dyn Iface> DynCoerce source must be a Var; got {:?}",
                            value.kind
                        ),
                    };
                    let concrete_ty = format_declarator(&value.ty, "").trim().to_string();
                    return format!(
                        "({{ {ct}* __heap = ({ct}*)malloc(sizeof({ct})); *__heap = ({src}); \
                          (intent_dyn_{iface}){{ .vtable = &intent_vtbl_{iface}_{conc}, .data = (void*)__heap }}; }})",
                        ct = concrete_ty,
                        src = local_name(&src_name),
                        iface = iface,
                        conc = from_type_name,
                    );
                }
                unreachable!(
                    "Box<dyn Iface> __box_new expected a DynCoerce arg; got {:?}",
                    args[0].kind
                );
            }
            // c_type_name returns "Struct_Point" / "int64_t" etc.
            // Wrap in a recoverable spelling via format_declarator
            // — without the inner-decl trim it's the right storage
            // spelling for malloc/sizeof.
            let c_ty = format_declarator(inner_ty, "").trim().to_string();
            let val = emit_expr(&args[0]);
            return format!(
                "({{ {0}* __b = ({0}*)malloc(sizeof({0})); *__b = ({1}); __b; }})",
                c_ty, val
            );
        }
        // L2 Phase 1: unbox(ref b) reads the inner value of a
        // Box<T>. Argument is a `ref Box<T>` which lowers to
        // `T**` in C; the read is `(*(*ref))` — first deref the
        // ref, then deref the Box pointer.
        "__box_get" if args.len() == 1 => {
            let arg = emit_expr(&args[0]);
            return format!("(*(*({})))", arg);
        }
        "min" => {
            // Inline ternary. Operands are evaluated once each
            // (no fresh stmt-emit machinery available here), so a
            // side-effecting subexpression would run twice. The
            // effects checker rejects impure operands in pure-fn
            // / parallel-for bodies, which is where reduction
            // bodies live — so this restriction is invisible to
            // users today.
            let a = emit_expr(&args[0]);
            let b = emit_expr(&args[1]);
            return format!("(({}) < ({}) ? ({}) : ({}))", a, b, a, b);
        }
        "max" => {
            let a = emit_expr(&args[0]);
            let b = emit_expr(&args[1]);
            return format!("(({}) > ({}) ? ({}) : ({}))", a, b, a, b);
        }
        "clamp" if args.len() == 3 => {
            // Inline `(x < lo ? lo : (x > hi ? hi : x))`. Same
            // multi-evaluation caveat as min/max — operands may
            // be evaluated up to three times, but the effects
            // checker rejects impure operands in places where
            // this matters. The 3-arg gate is the C-backend
            // half of the user-shadowing escape hatch: a user-
            // defined `fn clamp(x: i64) -> i64` (a non-3-arg
            // homonym) falls through to the `fn_clamp` user-fn
            // path below.
            let x = emit_expr(&args[0]);
            let lo = emit_expr(&args[1]);
            let hi = emit_expr(&args[2]);
            return format!(
                "(({x}) < ({lo}) ? ({lo}) : (({x}) > ({hi}) ? ({hi}) : ({x})))",
                x = x, lo = lo, hi = hi
            );
        }
        // Atomic builtins. Each call lowers to a single
        // C11 `<stdatomic.h>` operation with seq_cst memory
        // order. Element type T is recovered from the call's
        // typed arguments: `atomic_new` uses the result_ty
        // (`Atomic<T>`); the others read T off the value
        // argument's type (args[1]) since the checker has
        // already coerced it to T. The cell argument lowers
        // to `_Atomic <c_ty>*` per `format_declarator`.
        "atomic_new" => {
            return format!("({})", emit_expr(&args[0]));
        }
        "atomic_load" => {
            return format!(
                "atomic_load_explicit({}, memory_order_seq_cst)",
                emit_expr(&args[0])
            );
        }
        "atomic_store" => {
            let cell = emit_expr(&args[0]);
            let v = emit_expr(&args[1]);
            let elt_c = c_leaf_type(&args[1].ty);
            // C11 atomic_store_explicit returns void. Wrap in
            // a GNU/C statement-expression so the call site can
            // still consume a value of element type T (we
            // return the value that was stored).
            return format!(
                "({{ {elt} __v = ({v}); atomic_store_explicit({cell}, __v, memory_order_seq_cst); __v; }})",
                elt = elt_c,
                v = v,
                cell = cell
            );
        }
        "atomic_fetch_add" => {
            return format!(
                "atomic_fetch_add_explicit({}, {}, memory_order_seq_cst)",
                emit_expr(&args[0]),
                emit_expr(&args[1])
            );
        }
        "atomic_compare_exchange" => {
            // C11's `atomic_compare_exchange_*_explicit` takes a
            // pointer to the expected value (which it writes the
            // observed value into on failure). Wrap in a GNU
            // statement-expression so the call site sees a
            // single bool result without exposing the
            // intermediate.
            let cell = emit_expr(&args[0]);
            let exp = emit_expr(&args[1]);
            let new = emit_expr(&args[2]);
            let elt_c = c_leaf_type(&args[1].ty);
            return format!(
                "({{ {elt} __cas_exp = ({exp}); atomic_compare_exchange_strong_explicit({cell}, &__cas_exp, ({new}), memory_order_seq_cst, memory_order_seq_cst); }})",
                elt = elt_c,
                exp = exp,
                cell = cell,
                new = new
            );
        }
        "channel_new" => {
            // The result type carries (T, N); dispatch to the
            // matching per-(T, N) helper.
            let (element, capacity) = match result_ty {
                Type::Channel(elt, cap) => (elt.as_ref().clone(), *cap),
                _ => unreachable!("channel_new must return Channel<T, N>"),
            };
            return format!("{}()", c_channel_helper(&element, capacity, "new"));
        }
        "channel_send" => {
            // args[0] is `&Channel<T, N>` / `&mut Channel<T, N>`.
            // Recover (T, N) from its type, dispatch.
            let (element, capacity) = channel_inner_from_ref(&args[0].ty);
            return format!(
                "{}({}, {})",
                c_channel_helper(&element, capacity, "send"),
                emit_expr(&args[0]),
                emit_expr(&args[1])
            );
        }
        "channel_recv" => {
            let (element, capacity) = channel_inner_from_ref(&args[0].ty);
            return format!(
                "{}({})",
                c_channel_helper(&element, capacity, "recv"),
                emit_expr(&args[0])
            );
        }
        "mutex_new" => {
            return format!("intent_mutex_i64_new({})", emit_expr(&args[0]));
        }
        "mutex_lock" => {
            return format!("intent_mutex_i64_lock({})", emit_expr(&args[0]));
        }
        "guard_get" => {
            return format!("intent_guard_i64_get({})", emit_expr(&args[0]));
        }
        "guard_set" => {
            return format!(
                "intent_guard_i64_set({}, {})",
                emit_expr(&args[0]),
                emit_expr(&args[1])
            );
        }
        "condvar_new" => {
            return "intent_condvar_new()".to_string();
        }
        "condvar_wait" => {
            return format!(
                "intent_condvar_wait({}, {})",
                emit_expr(&args[0]),
                emit_expr(&args[1])
            );
        }
        "condvar_wait_timeout" => {
            return format!(
                "intent_condvar_wait_timeout({}, {}, {})",
                emit_expr(&args[0]),
                emit_expr(&args[1]),
                emit_expr(&args[2])
            );
        }
        "condvar_notify_one" => {
            return format!("intent_condvar_notify_one({})", emit_expr(&args[0]));
        }
        "condvar_notify_all" => {
            return format!("intent_condvar_notify_all({})", emit_expr(&args[0]));
        }
        "try_vec" => {
            // Closure #284: try_vec(n) -> Result<Vec<i64>,
            // AllocError>. Emits a GCC statement-expression
            // that mallocs an i64-buffer with null-check and
            // builds the Result tagged union. result_ty is
            // the mangled `Type::Enum("Result__<Vec...>__AllocError")`.
            let result_enum = match result_ty {
                Type::Enum(n) => n.clone(),
                _ => unreachable!("try_vec must return Type::Enum(Result__...)"),
            };
            let n_expr = emit_expr(&args[0]);
            let vec_struct = vec_c_struct(&Type::I64);
            let result_c = enum_c_name(&result_enum);
            // Variant tags: Ok=0, Err=1 (declaration order in
            // `enum Result<T, E> { Ok(T), Err(E) }`).
            // Note: AllocError is a payload-less enum →
            // lowers to `int32_t` (no Enum_AllocError
            // typedef). The Err variant just gets the tag
            // (0 = OutOfMemory in declaration order).
            return format!(
                "({{ \
                  uint64_t __try_vec_n = ({n}); \
                  int64_t* __try_vec_data = (int64_t*)malloc((__try_vec_n == 0 ? 1 : __try_vec_n) * sizeof(int64_t)); \
                  {result} __try_vec_r; \
                  if (__try_vec_data == NULL) {{ \
                    __try_vec_r.tag = 1; \
                    __try_vec_r.u.v_Err = (int32_t)0; \
                  }} else {{ \
                    {vs} __try_vec_v; \
                    __try_vec_v.data = __try_vec_data; \
                    __try_vec_v.len = 0; \
                    __try_vec_v.capacity = __try_vec_n == 0 ? 1 : __try_vec_n; \
                    __try_vec_r.tag = 0; \
                    __try_vec_r.u.v_Ok = __try_vec_v; \
                  }} \
                  __try_vec_r; \
                }})",
                n = n_expr,
                result = result_c,
                vs = vec_struct,
            );
        }
        "vec" => {
            let element = match result_ty {
                Type::Vec(element) => element,
                _ => unreachable!("vec() must return Vec<_>"),
            };
            // Use the element's storage spelling (handles
            // `Vec<U>` aggregates as `intent_vec_<U>`).
            // `c_leaf_type` was right for scalars but emits
            // `"/* vec */"` placeholders for nested Vecs.
            let c_element = c_element_storage(element);
            // For Array elements: C forbids initializing one
            // array from a compound-literal-as-rvalue (gcc:
            // "array initialized from non-constant array
            // expression"). The vec-emit normally turns
            // ArrayLit args into `((int64_t[4]){...})`
            // compound literals via `emit_expr`; for the
            // outer brace-list of a `(intent_arr4_int64_t[N]){...}`
            // initializer we need plain `{...}` so the outer
            // array directly initializes from braced
            // element-lists. Strip the cast for ArrayLit
            // args when this is the case. Refines #7 phase 2c.
            let element_is_array = matches!(element.as_ref(), Type::Array { .. });
            let parts: Vec<String> = args
                .iter()
                .map(|a| {
                    if element_is_array {
                        if let TypedExprKind::ArrayLit { elements } = &a.kind {
                            let inner: Vec<String> =
                                elements.iter().map(emit_expr).collect();
                            return format!("{{ {} }}", inner.join(", "));
                        }
                    }
                    emit_expr(a)
                })
                .collect();
            // C99 forbids zero-length array literals, so the
            // empty-vec case (e.g. `let xs: Vec<i64> = vec();`
            // — #8 from STATUS.md) passes NULL through the
            // `__from(0, NULL)` shape. The runtime helper
            // already special-cases `n == 0` and skips the
            // memcpy.
            if parts.is_empty() {
                format!(
                    "{}(0, (const {}*)0)",
                    vec_helper(element, "from"),
                    c_element
                )
            } else {
                let array_literal = format!(
                    "({}[{}]){{ {} }}",
                    c_element,
                    parts.len(),
                    parts.join(", ")
                );
                format!(
                    "{}({}, (const {}*){})",
                    vec_helper(element, "from"),
                    parts.len(),
                    c_element,
                    array_literal
                )
            }
        }
        "push" => {
            let element = match result_ty {
                Type::Vec(element) => element,
                _ => unreachable!("push() must return Vec<_>"),
            };
            format!(
                "{}({}, {})",
                vec_helper(element, "push"),
                emit_expr(&args[0]),
                emit_expr(&args[1])
            )
        }
        "push_mut" => {
            // In-place push: first arg is `mut ref Vec<T>`,
            // which lowers to a pointer to the Vec struct.
            // Element type comes from peeking through the ref.
            let element = match args[0].ty.deref() {
                Type::Vec(element) => element.clone(),
                _ => unreachable!("push_mut() arg 0 must be (mut ref) Vec<_>"),
            };
            format!(
                "{}({}, {})",
                vec_helper(&element, "push_mut"),
                emit_expr(&args[0]),
                emit_expr(&args[1])
            )
        }
        "pop" => {
            // In-place pop: first arg is `mut ref Vec<T>`.
            // The helper aborts on empty, otherwise decrements
            // `len` and returns the element (by-move for
            // non-Copy elements). Closure #219.
            let element = match args[0].ty.deref() {
                Type::Vec(element) => element.clone(),
                _ => unreachable!("pop() arg 0 must be (mut ref) Vec<_>"),
            };
            format!(
                "{}({})",
                vec_helper(&element, "pop_mut"),
                emit_expr(&args[0]),
            )
        }
        "sort" => {
            // In-place ascending sort. v1: i64 element.
            // Dispatches on Vec vs Array.
            match args[0].ty.deref() {
                Type::Vec(element) => format!(
                    "{}({})",
                    vec_helper(element, "sort"),
                    emit_expr(&args[0])
                ),
                Type::Array { length, .. } => format!(
                    "intent_array_int64_t__sort((int64_t*)({xs}), (uint64_t){len}LL)",
                    xs = emit_expr(&args[0]),
                    len = length,
                ),
                _ => unreachable!("sort() arg 0 must be (mut ref) Vec<_> or [T; N]"),
            }
        }
        // Closure #370: in-place descending sort. Composes
        // sort() + reverse() at the call site to avoid a new
        // runtime helper (and an extra `vec_helper` entry).
        // Statement-expression returns 0 so the call type-checks
        // as i64 like the other in-place mutators.
        "sort_desc" => {
            match args[0].ty.deref() {
                Type::Vec(element) => format!(
                    "({{ {sort}({xs}); {rev}({xs}); 0; }})",
                    sort = vec_helper(element, "sort"),
                    rev = vec_helper(element, "reverse"),
                    xs = emit_expr(&args[0]),
                ),
                Type::Array { length, .. } => format!(
                    "({{ intent_array_int64_t__sort((int64_t*)({xs}), (uint64_t){len}LL); intent_array_int64_t__reverse((int64_t*)({xs}), (uint64_t){len}LL); 0; }})",
                    xs = emit_expr(&args[0]),
                    len = length,
                ),
                _ => unreachable!("sort_desc() arg 0 must be (mut ref) Vec<_> or [T; N]"),
            }
        }
        "sort_by" => {
            // In-place sort with user comparator
            // `fn(i64, i64) -> i64`. v1: i64 element.
            match args[0].ty.deref() {
                Type::Vec(element) => format!(
                    "{}({}, {})",
                    vec_helper(element, "sort_by"),
                    emit_expr(&args[0]),
                    emit_expr(&args[1])
                ),
                Type::Array { length, .. } => format!(
                    "intent_array_int64_t__sort_by((int64_t*)({xs}), (uint64_t){len}LL, {cmp})",
                    xs = emit_expr(&args[0]),
                    len = length,
                    cmp = emit_expr(&args[1]),
                ),
                _ => unreachable!("sort_by() arg 0 must be (mut ref) Vec<_> or [T; N]"),
            }
        }
        "i64_to_str" => format!(
            "intent_i64_to_str(({}))",
            emit_expr(&args[0])
        ),
        "f64_to_str" => format!(
            "intent_f64_to_str(({}))",
            emit_expr(&args[0])
        ),
        "bool_to_str" => format!(
            "intent_bool_to_str(({}))",
            emit_expr(&args[0])
        ),
        "option_unwrap_or" => format!(
            "intent_option_i64_unwrap_or(({}), ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        // Closure #377: option_map(o, f) — inline statement-
        // expression that branches on the tag, calls f on the
        // payload, and re-packs Option::Some. None passes
        // through unchanged.
        "option_map" => format!(
            "({{ Enum_Option__i64 __om_o = ({o}); Enum_Option__i64 __om_r; if (__om_o.tag == 0) {{ __om_r.tag = 0; __om_r.payload = ({f})(__om_o.payload); }} else {{ __om_r.tag = 1; }} __om_r; }})",
            o = emit_expr(&args[0]),
            f = emit_expr(&args[1]),
        ),
        // Closure #384: option_filter(o, pred) — keep Some(v) if
        // pred(v) is true; None and Some(v)-where-!pred(v) both
        // become None.
        "option_filter" => format!(
            "({{ Enum_Option__i64 __of_o = ({o}); Enum_Option__i64 __of_r; if (__of_o.tag == 0 && ({p})(__of_o.payload)) {{ __of_r = __of_o; }} else {{ __of_r.tag = 1; }} __of_r; }})",
            o = emit_expr(&args[0]),
            p = emit_expr(&args[1]),
        ),
        // Closure #384: option_or(a, b) — first Some wins; if a
        // is None, fall back to b. Pure value combinator (b is
        // always evaluated; no short-circuit).
        "option_or" => format!(
            "({{ Enum_Option__i64 __oo_a = ({a}); Enum_Option__i64 __oo_b = ({b}); (__oo_a.tag == 0) ? __oo_a : __oo_b; }})",
            a = emit_expr(&args[0]),
            b = emit_expr(&args[1]),
        ),
        // Closure #391: option_and_then(o, f) — flatmap. Branch
        // on the tag, call f on the payload (which itself
        // returns Option<i64>), and pass that through. None
        // passes through unchanged.
        "option_and_then" => format!(
            "({{ Enum_Option__i64 __oat_o = ({o}); Enum_Option__i64 __oat_r; if (__oat_o.tag == 0) {{ __oat_r = ({f})(__oat_o.payload); }} else {{ __oat_r.tag = 1; }} __oat_r; }})",
            o = emit_expr(&args[0]),
            f = emit_expr(&args[1]),
        ),
        "option_is_some" => format!(
            "intent_option_i64_is_some(({}))",
            emit_expr(&args[0])
        ),
        "option_is_none" => format!(
            "intent_option_i64_is_none(({}))",
            emit_expr(&args[0])
        ),
        "option_unwrap_or_f64" => format!(
            "intent_option_f64_unwrap_or(({}), ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "option_is_some_f64" => format!(
            "intent_option_f64_is_some(({}))",
            emit_expr(&args[0])
        ),
        "option_is_none_f64" => format!(
            "intent_option_f64_is_none(({}))",
            emit_expr(&args[0])
        ),
        "vec_range" => format!(
            "intent_vec_int64_t_range(({}), ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "vec_repeat" => format!(
            "intent_vec_int64_t_repeat(({}), ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "vec_extend" => format!(
            "intent_vec_int64_t_extend({}, {})",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "vec_concat" => format!(
            "intent_vec_int64_t_concat({}, {})",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        // Closure #382: vec_iota(n) -> Vec<i64>.
        "vec_iota" => format!(
            "intent_vec_int64_t_iota(({}))",
            emit_expr(&args[0])
        ),
        // Closure #398: vec_running_sum(ref xs) -> Vec<i64>.
        "vec_running_sum" => format!(
            "intent_vec_int64_t_running_sum({})",
            emit_expr(&args[0])
        ),
        // Closures #510/#511: vec_cumulative_max / vec_cumulative_min.
        "vec_cumulative_max" => format!(
            "intent_vec_int64_t_cumulative_max({})",
            emit_expr(&args[0])
        ),
        "vec_cumulative_min" => format!(
            "intent_vec_int64_t_cumulative_min({})",
            emit_expr(&args[0])
        ),
        // Closures #512-#515: monoidal running reductions.
        "vec_running_product" => format!(
            "intent_vec_int64_t_running_product({})",
            emit_expr(&args[0])
        ),
        "vec_running_xor" => format!(
            "intent_vec_int64_t_running_xor({})",
            emit_expr(&args[0])
        ),
        "vec_running_and" => format!(
            "intent_vec_int64_t_running_and({})",
            emit_expr(&args[0])
        ),
        "vec_running_or" => format!(
            "intent_vec_int64_t_running_or({})",
            emit_expr(&args[0])
        ),
        // Closures #516-#519: Vec<i64> predicates.
        "vec_all_equal" => format!(
            "intent_vec_int64_t_all_equal({})",
            emit_expr(&args[0])
        ),
        "vec_is_sorted_asc" => format!(
            "intent_vec_int64_t_is_sorted_asc({})",
            emit_expr(&args[0])
        ),
        "vec_is_sorted_desc" => format!(
            "intent_vec_int64_t_is_sorted_desc({})",
            emit_expr(&args[0])
        ),
        "vec_is_palindrome" => format!(
            "intent_vec_int64_t_is_palindrome({})",
            emit_expr(&args[0])
        ),
        // Closures #520-#523: sliding window reductions.
        "vec_sliding_max" => format!(
            "intent_vec_int64_t_sliding_max({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "vec_sliding_min" => format!(
            "intent_vec_int64_t_sliding_min({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "vec_sliding_sum" => format!(
            "intent_vec_int64_t_sliding_sum({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "vec_sliding_product" => format!(
            "intent_vec_int64_t_sliding_product({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        // Closures #524-#527: element-wise unary Vec<i64> transforms.
        "vec_abs" => format!(
            "intent_vec_int64_t_abs({})",
            emit_expr(&args[0])
        ),
        "vec_negate" => format!(
            "intent_vec_int64_t_negate({})",
            emit_expr(&args[0])
        ),
        "vec_signum" => format!(
            "intent_vec_int64_t_signum({})",
            emit_expr(&args[0])
        ),
        "vec_square" => format!(
            "intent_vec_int64_t_square({})",
            emit_expr(&args[0])
        ),
        // Closures #528-#531: scalar-broadcast arithmetic.
        "vec_add_scalar" => format!(
            "intent_vec_int64_t_add_scalar({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "vec_sub_scalar" => format!(
            "intent_vec_int64_t_sub_scalar({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "vec_mul_scalar" => format!(
            "intent_vec_int64_t_mul_scalar({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "vec_div_scalar" => format!(
            "intent_vec_int64_t_div_scalar({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        // Closures #532-#537: scalar comparison masks.
        "vec_eq_mask" | "vec_ne_mask"
        | "vec_lt_mask" | "vec_le_mask"
        | "vec_gt_mask" | "vec_ge_mask" => {
            let op = name.strip_prefix("vec_").unwrap();
            format!(
                "intent_vec_int64_t_{}({}, ({}))",
                op,
                emit_expr(&args[0]),
                emit_expr(&args[1])
            )
        }
        // Closures #538/#539: scalar min/max.
        "vec_min_with_scalar" | "vec_max_with_scalar" => {
            let op = name.strip_prefix("vec_").unwrap();
            format!(
                "intent_vec_int64_t_{}({}, ({}))",
                op,
                emit_expr(&args[0]),
                emit_expr(&args[1])
            )
        }
        // Closure #540: vec_clamp_scalar.
        "vec_clamp_scalar" => format!(
            "intent_vec_int64_t_clamp_scalar({}, ({}), ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1]),
            emit_expr(&args[2])
        ),
        // Closures #541-#545: element-wise binary between two Vec<i64>.
        "vec_add_pairwise" | "vec_sub_pairwise"
        | "vec_mul_pairwise" | "vec_min_pairwise"
        | "vec_max_pairwise" | "vec_merge_sorted" => {
            let op = name.strip_prefix("vec_").unwrap();
            format!(
                "intent_vec_int64_t_{}({}, {})",
                op,
                emit_expr(&args[0]),
                emit_expr(&args[1])
            )
        }
        "vec_insert_sorted" => format!(
            "intent_vec_int64_t_insert_sorted({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "vec_is_sorted_unique" => format!(
            "intent_vec_int64_t_is_sorted_unique({})",
            emit_expr(&args[0])
        ),
        // Closures #569-#572: sort-free analytics.
        "vec_range_span" => format!(
            "intent_vec_int64_t_range_span({})",
            emit_expr(&args[0])
        ),
        "vec_mode" => format!(
            "intent_vec_int64_t_mode({})",
            emit_expr(&args[0])
        ),
        "vec_median" => format!(
            "intent_vec_int64_t_median({})",
            emit_expr(&args[0])
        ),
        "vec_running_mean" => format!(
            "intent_vec_int64_t_running_mean({})",
            emit_expr(&args[0])
        ),
        "vec_intersperse" => format!(
            "intent_vec_int64_t_intersperse({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "vec_kth_smallest" => format!(
            "intent_vec_int64_t_kth_smallest({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        // Closures #546-#549: modular/bit-shift scalar broadcast.
        "vec_mod_scalar" | "vec_pow_scalar"
        | "vec_shl_scalar" | "vec_shr_scalar" => {
            let op = name.strip_prefix("vec_").unwrap();
            format!(
                "intent_vec_int64_t_{}({}, ({}))",
                op,
                emit_expr(&args[0]),
                emit_expr(&args[1])
            )
        }
        // Closures #550-#553: positional rotations and shifts.
        "vec_rotate_left" | "vec_rotate_right"
        | "vec_shift_left" | "vec_shift_right" => {
            let op = name.strip_prefix("vec_").unwrap();
            format!(
                "intent_vec_int64_t_{}({}, ({}))",
                op,
                emit_expr(&args[0]),
                emit_expr(&args[1])
            )
        }
        // Closures #554-#557: dual-vec bool predicates.
        "vec_subset_of" | "vec_disjoint"
        | "vec_equal_set" | "vec_equal_seq" => {
            let op = name.strip_prefix("vec_").unwrap();
            format!(
                "intent_vec_int64_t_{}({}, {})",
                op,
                emit_expr(&args[0]),
                emit_expr(&args[1])
            )
        }
        // Closure #558: vec_diff(ref xs) -> Vec<i64>.
        "vec_diff" => format!(
            "intent_vec_int64_t_diff({})",
            emit_expr(&args[0])
        ),
        // Closures #562 / #565: scalar reductions.
        "vec_count_distinct" => format!(
            "intent_vec_int64_t_count_distinct({})",
            emit_expr(&args[0])
        ),
        "vec_mean" => format!(
            "intent_vec_int64_t_mean({})",
            emit_expr(&args[0])
        ),
        // Closure #563: vec_indices_of_value(ref xs, v) -> Vec<i64>.
        "vec_indices_of_value" => format!(
            "intent_vec_int64_t_indices_of_value({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        // Closure #564: vec_dedup_consecutive(ref xs) -> Vec<i64>.
        "vec_dedup_consecutive" => format!(
            "intent_vec_int64_t_dedup_consecutive({})",
            emit_expr(&args[0])
        ),
        // Closures #593-#596: nested-Vec constructors.
        "vec_chunks" => format!(
            "intent_vec_vec_int64_t_chunks({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "vec_windows" => format!(
            "intent_vec_vec_int64_t_windows({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "vec_group_by_value" => format!(
            "intent_vec_vec_int64_t_group_by_value({})",
            emit_expr(&args[0])
        ),
        "vec_flatten" => format!(
            "intent_vec_int64_t_flatten({})",
            emit_expr(&args[0])
        ),
        // Closures #559-#561: 3-arg fresh-Vec builders.
        "vec_pad_left" | "vec_pad_right" | "vec_replace_value" => {
            let op = name.strip_prefix("vec_").unwrap();
            format!(
                "intent_vec_int64_t_{}({}, ({}), ({}))",
                op,
                emit_expr(&args[0]),
                emit_expr(&args[1]),
                emit_expr(&args[2])
            )
        }
        // Closure #399: vec_dot(ref xs, ref ys) -> i64.
        "vec_dot" => format!(
            "intent_vec_int64_t_dot({}, {})",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        // Closure #407: set ops on Vec<i64>.
        "vec_intersect" => format!(
            "intent_vec_int64_t_intersect({}, {})",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "vec_difference" => format!(
            "intent_vec_int64_t_difference({}, {})",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "vec_union" => format!(
            "intent_vec_int64_t_union({}, {})",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        // Closure #385: vec_first / vec_last(ref xs) -> Option<i64>.
        "vec_first" => format!(
            "intent_vec_int64_t_first({})",
            emit_expr(&args[0])
        ),
        "vec_last" => format!(
            "intent_vec_int64_t_last({})",
            emit_expr(&args[0])
        ),
        // Closure #371: fresh-allocating reverse / dedup.
        "vec_reverse_copy" => format!(
            "intent_vec_int64_t_reverse_copy({})",
            emit_expr(&args[0])
        ),
        "vec_unique" => format!(
            "intent_vec_int64_t_unique({})",
            emit_expr(&args[0])
        ),
        "vec_map" => {
            // vec_map(ref xs: Vec<i64>, f) -> Vec<i64>. Eager;
            // helper materializes a new Vec. Closure #309.
            match args[0].ty.deref() {
                Type::Vec(element) => format!(
                    "{}({}, {})",
                    vec_helper(element, "map"),
                    emit_expr(&args[0]),
                    emit_expr(&args[1])
                ),
                _ => unreachable!("vec_map() arg 0 must be ref Vec<i64>"),
            }
        }
        "vec_filter" => {
            // vec_filter(ref xs: Vec<i64>, p) -> Vec<i64>.
            // Eager; helper materializes a new Vec. Closure #310.
            match args[0].ty.deref() {
                Type::Vec(element) => format!(
                    "{}({}, {})",
                    vec_helper(element, "filter"),
                    emit_expr(&args[0]),
                    emit_expr(&args[1])
                ),
                _ => unreachable!("vec_filter() arg 0 must be ref Vec<i64>"),
            }
        }
        // Closure #378: vec_position(ref xs, pred) -> Option<i64>.
        "vec_position" => {
            match args[0].ty.deref() {
                Type::Vec(element) => format!(
                    "{}({}, {})",
                    vec_helper(element, "position"),
                    emit_expr(&args[0]),
                    emit_expr(&args[1])
                ),
                _ => unreachable!("vec_position() arg 0 must be ref Vec<i64>"),
            }
        }
        // Closure #387: vec_swap(mut ref xs, i, j) -> i64.
        "vec_swap" => {
            format!(
                "({{ intent_vec_int64_t* __vs_xs = ({xs}); int64_t __vs_i = ({i}); int64_t __vs_j = ({j}); int64_t __vs_t = __vs_xs->data[__vs_i]; __vs_xs->data[__vs_i] = __vs_xs->data[__vs_j]; __vs_xs->data[__vs_j] = __vs_t; (int64_t)0; }})",
                xs = emit_expr(&args[0]),
                i = emit_expr(&args[1]),
                j = emit_expr(&args[2]),
            )
        }
        // Closure #388: vec_remove_at(mut ref xs, i) -> i64.
        "vec_remove_at" => {
            format!(
                "({{ intent_vec_int64_t* __vra_xs = ({xs}); int64_t __vra_i = ({i}); int64_t __vra_r = __vra_xs->data[__vra_i]; for (uint64_t __k = (uint64_t)__vra_i; __k + 1 < __vra_xs->len; __k++) {{ __vra_xs->data[__k] = __vra_xs->data[__k + 1]; }} __vra_xs->len--; __vra_r; }})",
                xs = emit_expr(&args[0]),
                i = emit_expr(&args[1]),
            )
        }
        // Closure #396: vec_replace_all(mut ref xs, old, new) -> i64.
        // Walks xs once, swapping every occurrence of old with
        // new. Returns the count of replacements.
        "vec_replace_all" => {
            format!(
                "({{ intent_vec_int64_t* __vrp_xs = ({xs}); int64_t __vrp_old = ({old}); int64_t __vrp_new = ({new}); int64_t __vrp_n = 0; for (uint64_t __vrp_i = 0; __vrp_i < __vrp_xs->len; __vrp_i++) {{ if (__vrp_xs->data[__vrp_i] == __vrp_old) {{ __vrp_xs->data[__vrp_i] = __vrp_new; __vrp_n++; }} }} __vrp_n; }})",
                xs = emit_expr(&args[0]),
                old = emit_expr(&args[1]),
                new = emit_expr(&args[2]),
            )
        }
        // Closure #397: vec_zip_with(ref xs, ref ys, f) -> Vec<i64>.
        "vec_zip_with" => {
            match args[0].ty.deref() {
                Type::Vec(element) => format!(
                    "{}({}, {}, {})",
                    vec_helper(element, "zip_with"),
                    emit_expr(&args[0]),
                    emit_expr(&args[1]),
                    emit_expr(&args[2])
                ),
                _ => unreachable!("vec_zip_with() arg 0 must be ref Vec<i64>"),
            }
        }
        // Closure #386: vec_count_if(ref xs, pred) -> i64.
        "vec_count_if" => {
            match args[0].ty.deref() {
                Type::Vec(element) => format!(
                    "{}({}, {})",
                    vec_helper(element, "count_if"),
                    emit_expr(&args[0]),
                    emit_expr(&args[1])
                ),
                _ => unreachable!("vec_count_if() arg 0 must be ref Vec<i64>"),
            }
        }
        // Closure #392: vec_max_by / vec_min_by.
        "vec_max_by" | "vec_min_by" => {
            let op = if *&name == "vec_max_by" { "max_by" } else { "min_by" };
            match args[0].ty.deref() {
                Type::Vec(element) => format!(
                    "{}({}, {})",
                    vec_helper(element, op),
                    emit_expr(&args[0]),
                    emit_expr(&args[1])
                ),
                _ => unreachable!("{}() arg 0 must be ref Vec<i64>", name),
            }
        }
        // Closure #389: vec_take_while / vec_drop_while.
        "vec_take_while" | "vec_drop_while" => {
            let op = if *&name == "vec_take_while" { "take_while" } else { "drop_while" };
            match args[0].ty.deref() {
                Type::Vec(element) => format!(
                    "{}({}, {})",
                    vec_helper(element, op),
                    emit_expr(&args[0]),
                    emit_expr(&args[1])
                ),
                _ => unreachable!("{}() arg 0 must be ref Vec<i64>", name),
            }
        }
        "vec_take" | "vec_drop" => {
            // vec_take / vec_drop (closure #313): eager slicing.
            let op = name.strip_prefix("vec_").unwrap();
            match args[0].ty.deref() {
                Type::Vec(element) => format!(
                    "{}({}, ({}))",
                    vec_helper(element, op),
                    emit_expr(&args[0]),
                    emit_expr(&args[1])
                ),
                _ => unreachable!("{}() arg 0 must be ref Vec<i64>", name),
            }
        }
        "vec_map_fold" => {
            // vec_map_fold (closure #316): fused map+fold,
            // no intermediate Vec. args = ref xs, init, f, g.
            match args[0].ty.deref() {
                Type::Vec(element) => format!(
                    "{}({}, ({}), {}, {})",
                    vec_helper(element, "map_fold"),
                    emit_expr(&args[0]),
                    emit_expr(&args[1]),
                    emit_expr(&args[2]),
                    emit_expr(&args[3])
                ),
                _ => unreachable!("vec_map_fold() arg 0 must be ref Vec<i64>"),
            }
        }
        "vec_filter_fold" => {
            // Closure #317. args = ref xs, init, p, g.
            match args[0].ty.deref() {
                Type::Vec(element) => format!(
                    "{}({}, ({}), {}, {})",
                    vec_helper(element, "filter_fold"),
                    emit_expr(&args[0]),
                    emit_expr(&args[1]),
                    emit_expr(&args[2]),
                    emit_expr(&args[3])
                ),
                _ => unreachable!("vec_filter_fold() arg 0 must be ref Vec<i64>"),
            }
        }
        "vec_map_filter" => {
            // Closure #317. args = ref xs, f, p.
            match args[0].ty.deref() {
                Type::Vec(element) => format!(
                    "{}({}, {}, {})",
                    vec_helper(element, "map_filter"),
                    emit_expr(&args[0]),
                    emit_expr(&args[1]),
                    emit_expr(&args[2])
                ),
                _ => unreachable!("vec_map_filter() arg 0 must be ref Vec<i64>"),
            }
        }
        "vec_chain" => {
            // Closure #324. args = ref xs, ref ys.
            match args[0].ty.deref() {
                Type::Vec(element) => format!(
                    "{}({}, {})",
                    vec_helper(element, "chain"),
                    emit_expr(&args[0]),
                    emit_expr(&args[1])
                ),
                _ => unreachable!("vec_chain() arg 0 must be ref Vec<i64>"),
            }
        }
        "vec_sum" | "vec_product" => {
            // Closure #322. args = ref xs.
            match args[0].ty.deref() {
                Type::Vec(element) => format!(
                    "{}({})",
                    vec_helper(element, name.strip_prefix("vec_").unwrap()),
                    emit_expr(&args[0])
                ),
                _ => unreachable!("{}() arg 0 must be ref Vec<i64>", name),
            }
        }
        "vec_min" | "vec_max" => {
            // Closure #322. args = ref xs, default.
            match args[0].ty.deref() {
                Type::Vec(element) => format!(
                    "{}({}, ({}))",
                    vec_helper(element, name.strip_prefix("vec_").unwrap()),
                    emit_expr(&args[0]),
                    emit_expr(&args[1])
                ),
                _ => unreachable!("{}() arg 0 must be ref Vec<i64>", name),
            }
        }
        "vec_argmin" | "vec_argmax" => {
            // Closures #505/#506. args = ref xs, default.
            match args[0].ty.deref() {
                Type::Vec(element) => format!(
                    "{}({}, ({}))",
                    vec_helper(element, name.strip_prefix("vec_").unwrap()),
                    emit_expr(&args[0]),
                    emit_expr(&args[1])
                ),
                _ => unreachable!("{}() arg 0 must be ref Vec<i64>", name),
            }
        }
        "vec_count_value" | "vec_index_of_value" | "vec_last_index_of_value" => {
            // Closures #507/#508/#509. args = ref xs, search-value.
            match args[0].ty.deref() {
                Type::Vec(element) => format!(
                    "{}({}, ({}))",
                    vec_helper(element, name.strip_prefix("vec_").unwrap()),
                    emit_expr(&args[0]),
                    emit_expr(&args[1])
                ),
                _ => unreachable!("{}() arg 0 must be ref Vec<i64>", name),
            }
        }
        "vec_count" | "vec_any" | "vec_all" => {
            // Closure #322. args = ref xs, predicate.
            match args[0].ty.deref() {
                Type::Vec(element) => format!(
                    "{}({}, {})",
                    vec_helper(element, name.strip_prefix("vec_").unwrap()),
                    emit_expr(&args[0]),
                    emit_expr(&args[1])
                ),
                _ => unreachable!("{}() arg 0 must be ref Vec<i64>", name),
            }
        }
        "vec_map_filter_fold" => {
            // Closure #317. args = ref xs, init, f, p, g.
            match args[0].ty.deref() {
                Type::Vec(element) => format!(
                    "{}({}, ({}), {}, {}, {})",
                    vec_helper(element, "map_filter_fold"),
                    emit_expr(&args[0]),
                    emit_expr(&args[1]),
                    emit_expr(&args[2]),
                    emit_expr(&args[3]),
                    emit_expr(&args[4])
                ),
                _ => unreachable!("vec_map_filter_fold() arg 0 must be ref Vec<i64>"),
            }
        }
        "vec_fold" => {
            // vec_fold(ref xs: Vec<i64>, init, g) -> i64. Closure #309.
            match args[0].ty.deref() {
                Type::Vec(element) => format!(
                    "{}({}, {}, {})",
                    vec_helper(element, "fold"),
                    emit_expr(&args[0]),
                    emit_expr(&args[1]),
                    emit_expr(&args[2])
                ),
                _ => unreachable!("vec_fold() arg 0 must be ref Vec<i64>"),
            }
        }
        "reverse" | "dedup" => {
            // reverse: Vec OR Array. dedup: Vec only.
            match args[0].ty.deref() {
                Type::Vec(element) => format!(
                    "{}({})",
                    vec_helper(element, name),
                    emit_expr(&args[0])
                ),
                Type::Array { length, .. } if name == "reverse" => format!(
                    "intent_array_int64_t__reverse((int64_t*)({xs}), (uint64_t){len}LL)",
                    xs = emit_expr(&args[0]),
                    len = length,
                ),
                _ => unreachable!("{name}() arg 0 must be (mut ref) Vec<_>"),
            }
        }
        "contains" => {
            // Linear scan; returns bool. xs is `ref Vec<i64>`
            // or `ref [i64; N]`.
            match args[0].ty.deref() {
                Type::Array { length, .. } => format!(
                    "intent_array_int64_t__contains((const int64_t*)({xs}), (uint64_t){len}LL, ({n}))",
                    xs = emit_expr(&args[0]),
                    len = length,
                    n = emit_expr(&args[1]),
                ),
                _ => format!(
                    "({{ const intent_vec_int64_t* __cv = ({xs}); int64_t __cn = ({n}); bool __cr = false; for (uint64_t __ci = 0; __ci < __cv->len; __ci++) {{ if (__cv->data[__ci] == __cn) {{ __cr = true; break; }} }} __cr; }})",
                    xs = emit_expr(&args[0]),
                    n = emit_expr(&args[1]),
                ),
            }
        }
        "find" => {
            // Linear scan; returns Option<i64>. v1: i64
            // element. Option<T>'s C layout is
            // `{ int32_t tag; T payload; }`.
            let opt_name = match result_ty {
                Type::Enum(name) => name.clone(),
                _ => unreachable!("find() must return Type::Enum(Option__i64)"),
            };
            let opt_c = enum_c_name(&opt_name);
            match args[0].ty.deref() {
                Type::Array { length, .. } => format!(
                    "intent_array_int64_t__find((const int64_t*)({xs}), (uint64_t){len}LL, ({n}))",
                    xs = emit_expr(&args[0]),
                    len = length,
                    n = emit_expr(&args[1]),
                ),
                _ => format!(
                    "({{ const intent_vec_int64_t* __fv = ({xs}); int64_t __fn = ({n}); {opt} __fr; bool __ff = false; uint64_t __fi = 0; for (__fi = 0; __fi < __fv->len; __fi++) {{ if (__fv->data[__fi] == __fn) {{ __ff = true; break; }} }} if (__ff) {{ __fr.tag = 0; __fr.payload = (int64_t)__fi; }} else {{ __fr.tag = 1; __fr.payload = 0; }} __fr; }})",
                    xs = emit_expr(&args[0]),
                    n = emit_expr(&args[1]),
                    opt = opt_c,
                ),
            }
        }
        "swap_remove" => {
            // mut ref Vec<T>, i -> T (moves slot out, swaps
            // with last). v1 rejects array element types in
            // the checker.
            let element = match args[0].ty.deref() {
                Type::Vec(element) => element.clone(),
                _ => unreachable!("swap_remove() arg 0 must be (mut ref) Vec<_>"),
            };
            format!(
                "{}({}, (uint64_t)({}))",
                vec_helper(&element, "swap_remove"),
                emit_expr(&args[0]),
                emit_expr(&args[1])
            )
        }
        "insert" => {
            let element = match args[0].ty.deref() {
                Type::Vec(element) => element.clone(),
                _ => unreachable!("insert() arg 0 must be (mut ref) Vec<_>"),
            };
            format!(
                "{}({}, (uint64_t)({}), {})",
                vec_helper(&element, "insert"),
                emit_expr(&args[0]),
                emit_expr(&args[1]),
                emit_expr(&args[2])
            )
        }
        "clear" => {
            let element = match args[0].ty.deref() {
                Type::Vec(element) => element.clone(),
                _ => unreachable!("clear() arg 0 must be (mut ref) Vec<_>"),
            };
            format!(
                "{}({})",
                vec_helper(&element, "clear"),
                emit_expr(&args[0])
            )
        }
        "str_contains" => {
            format!(
                "(strstr(({s}), ({n})) != NULL)",
                s = emit_expr(&args[0]),
                n = emit_expr(&args[1]),
            )
        }
        // Closure #365: str_index_of(haystack, needle) -> Option<i64>.
        // Inline statement-expression: strstr the needle, then
        // pack the byte-offset (or None) into an Enum_Option__i64.
        "str_index_of" => {
            format!(
                "({{ const char* __sio_s = ({s}); const char* __sio_m = strstr(__sio_s, ({n})); Enum_Option__i64 __sio_r; if (__sio_m == NULL) {{ __sio_r.tag = 1; }} else {{ __sio_r.tag = 0; __sio_r.payload = (int64_t)(__sio_m - __sio_s); }} __sio_r; }})",
                s = emit_expr(&args[0]),
                n = emit_expr(&args[1]),
            )
        }
        // Closure #366: substring(s, start, len) -> OwnedStr.
        "substring" => {
            format!(
                "intent_substring(({}), ({}), ({}))",
                emit_expr(&args[0]),
                emit_expr(&args[1]),
                emit_expr(&args[2]),
            )
        }
        // Closure #368: str_repeat(s, n) -> OwnedStr.
        "str_repeat" => {
            format!(
                "intent_str_repeat(({}), ({}))",
                emit_expr(&args[0]),
                emit_expr(&args[1]),
            )
        }
        // Closure #379: str_join(ref strs, sep) -> OwnedStr.
        "str_join" => format!(
            "intent_str_join(({}), ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1]),
        ),
        // Closure #381: str padding / line splitting.
        "str_pad_left" => format!(
            "intent_str_pad_left(({}), ({}), ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1]),
            emit_expr(&args[2]),
        ),
        "str_pad_right" => format!(
            "intent_str_pad_right(({}), ({}), ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1]),
            emit_expr(&args[2]),
        ),
        "str_lines" => format!(
            "intent_str_lines(({}))",
            emit_expr(&args[0]),
        ),
        // Closure #390: str_chars / str_reverse.
        "str_chars" => format!(
            "intent_str_chars(({}))",
            emit_expr(&args[0]),
        ),
        "str_reverse" => format!(
            "intent_str_reverse(({}))",
            emit_expr(&args[0]),
        ),
        // Closures #394, #395.
        "str_strip_prefix" => format!(
            "intent_str_strip_prefix(({}), ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1]),
        ),
        "str_strip_suffix" => format!(
            "intent_str_strip_suffix(({}), ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1]),
        ),
        "str_count_char" => format!(
            "intent_str_count_char(({}), ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1]),
        ),
        // Closure #369: ASCII case conversion -> OwnedStr.
        "str_to_upper" => {
            format!("intent_str_to_upper(({}))", emit_expr(&args[0]))
        }
        "str_to_lower" => {
            format!("intent_str_to_lower(({}))", emit_expr(&args[0]))
        }
        // Closure #373: parse_bool(s) -> Option<bool>. Inline
        // statement-expression — recognizes "true" / "false"
        // exactly (case-sensitive); anything else is None.
        "parse_bool" => {
            format!(
                "({{ const char* __pb_s = ({s}); Enum_Option__bool __pb_r; if (strcmp(__pb_s, \"true\") == 0) {{ __pb_r.tag = 0; __pb_r.payload = true; }} else if (strcmp(__pb_s, \"false\") == 0) {{ __pb_r.tag = 0; __pb_r.payload = false; }} else {{ __pb_r.tag = 1; }} __pb_r; }})",
                s = emit_expr(&args[0]),
            )
        }
        "str_starts_with" => {
            // strncmp(s, p, strlen(p)) == 0. Cache the prefix
            // length so it isn't computed twice.
            format!(
                "({{ const char* __sw_s = ({s}); const char* __sw_p = ({p}); size_t __sw_pl = strlen(__sw_p); (strncmp(__sw_s, __sw_p, __sw_pl) == 0); }})",
                s = emit_expr(&args[0]),
                p = emit_expr(&args[1]),
            )
        }
        "str_ends_with" => {
            format!(
                "({{ const char* __ew_s = ({s}); const char* __ew_u = ({u}); size_t __ew_sl = strlen(__ew_s); size_t __ew_ul = strlen(__ew_u); (__ew_ul <= __ew_sl && strcmp(__ew_s + __ew_sl - __ew_ul, __ew_u) == 0); }})",
                s = emit_expr(&args[0]),
                u = emit_expr(&args[1]),
            )
        }
        "str_trim" => {
            format!("intent_str_trim(({}))", emit_expr(&args[0]))
        }
        "str_replace" => {
            format!(
                "intent_str_replace(({}), ({}), ({}))",
                emit_expr(&args[0]),
                emit_expr(&args[1]),
                emit_expr(&args[2])
            )
        }
        "str_split" => {
            format!(
                "intent_str_split(({}), ({}))",
                emit_expr(&args[0]),
                emit_expr(&args[1])
            )
        }
        "parse_int" => {
            // strtoll converts the prefix; we require the
            // ENTIRE string to be consumed for a successful
            // parse. Empty string → None.
            let opt_name = match result_ty {
                Type::Enum(name) => name.clone(),
                _ => unreachable!("parse_int must return Type::Enum(Option__i64)"),
            };
            let opt_c = enum_c_name(&opt_name);
            format!(
                "({{ const char* __pi_s = ({s}); char* __pi_end = (char*)0; long long __pi_v = strtoll(__pi_s, &__pi_end, 10); {opt} __pi_r; if (__pi_end != __pi_s && *__pi_end == 0 && *__pi_s != 0) {{ __pi_r.tag = 0; __pi_r.payload = (int64_t)__pi_v; }} else {{ __pi_r.tag = 1; __pi_r.payload = 0; }} __pi_r; }})",
                s = emit_expr(&args[0]),
                opt = opt_c,
            )
        }
        "btreeset_new" => "intent_btreeset_i64_new()".to_string(),
        "btreeset_insert" => format!(
            "intent_btreeset_i64_insert({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "btreeset_contains" => format!(
            "intent_btreeset_i64_contains({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "btreeset_remove" => format!(
            "intent_btreeset_i64_remove({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "btreeset_len" => format!(
            "intent_btreeset_i64_len({})",
            emit_expr(&args[0])
        ),
        "btreeset_range" => format!(
            "intent_btreeset_i64_range({}, ({}), ({}), {})",
            emit_expr(&args[0]),
            emit_expr(&args[1]),
            emit_expr(&args[2]),
            emit_expr(&args[3])
        ),
        "btreeset_min" => format!(
            "intent_btreeset_i64_min({})",
            emit_expr(&args[0])
        ),
        "btreeset_max" => format!(
            "intent_btreeset_i64_max({})",
            emit_expr(&args[0])
        ),
        "btreeset_clear" => format!(
            "intent_btreeset_i64_clear({})",
            emit_expr(&args[0])
        ),
        "btreemap_new" => "intent_btreemap_i64_i64_new()".to_string(),
        "btreemap_insert" => format!(
            "intent_btreemap_i64_i64_insert({}, ({}), ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1]),
            emit_expr(&args[2])
        ),
        "btreemap_get" => format!(
            "intent_btreemap_i64_i64_get({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "btreemap_contains_key" => format!(
            "intent_btreemap_i64_i64_contains_key({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "btreemap_remove" => format!(
            "intent_btreemap_i64_i64_remove({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "btreemap_len" => format!(
            "intent_btreemap_i64_i64_len({})",
            emit_expr(&args[0])
        ),
        "btreemap_range_keys" => format!(
            "intent_btreemap_i64_i64_range_keys({}, ({}), ({}), {})",
            emit_expr(&args[0]),
            emit_expr(&args[1]),
            emit_expr(&args[2]),
            emit_expr(&args[3])
        ),
        "btreemap_range_values" => format!(
            "intent_btreemap_i64_i64_range_values({}, ({}), ({}), {})",
            emit_expr(&args[0]),
            emit_expr(&args[1]),
            emit_expr(&args[2]),
            emit_expr(&args[3])
        ),
        "btreemap_min_key" => format!(
            "intent_btreemap_i64_i64_min_key({})",
            emit_expr(&args[0])
        ),
        "btreemap_max_key" => format!(
            "intent_btreemap_i64_i64_max_key({})",
            emit_expr(&args[0])
        ),
        "btreemap_clear" => format!(
            "intent_btreemap_i64_i64_clear({})",
            emit_expr(&args[0])
        ),
        // Closure #325: Union-Find dispatch.
        "union_find_new" => format!(
            "intent_union_find_new(({}))",
            emit_expr(&args[0])
        ),
        "union_find_union" => format!(
            "intent_union_find_union({}, ({}), ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1]),
            emit_expr(&args[2])
        ),
        "union_find_find" => format!(
            "intent_union_find_find({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "union_find_connected" => format!(
            "intent_union_find_connected({}, ({}), ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1]),
            emit_expr(&args[2])
        ),
        "union_find_count" => format!(
            "intent_union_find_count({})",
            emit_expr(&args[0])
        ),
        "union_find_clear" => format!(
            "intent_union_find_clear({})",
            emit_expr(&args[0])
        ),
        // Closure #326: BinaryHeap dispatch.
        "binary_heap_new" => "intent_binary_heap_i64_new()".to_string(),
        "binary_heap_push" => format!(
            "intent_binary_heap_i64_push({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "binary_heap_pop" => format!(
            "intent_binary_heap_i64_pop({})",
            emit_expr(&args[0])
        ),
        "binary_heap_peek" => format!(
            "intent_binary_heap_i64_peek({})",
            emit_expr(&args[0])
        ),
        "binary_heap_len" => format!(
            "intent_binary_heap_i64_len({})",
            emit_expr(&args[0])
        ),
        "binary_heap_clear" => format!(
            "intent_binary_heap_i64_clear({})",
            emit_expr(&args[0])
        ),
        // Closure #327: BloomFilter dispatch.
        "bloom_filter_new" => format!(
            "intent_bloom_filter_new(({}), ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "bloom_filter_insert" => format!(
            "intent_bloom_filter_insert({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "bloom_filter_contains" => format!(
            "intent_bloom_filter_contains({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "bloom_filter_len" => format!(
            "intent_bloom_filter_len({})",
            emit_expr(&args[0])
        ),
        "bloom_filter_count" => format!(
            "intent_bloom_filter_count({})",
            emit_expr(&args[0])
        ),
        "bloom_filter_clear" => format!(
            "intent_bloom_filter_clear({})",
            emit_expr(&args[0])
        ),
        // Closure #328: Bst dispatch.
        "bst_new" => "intent_bst_i64_new()".to_string(),
        "bst_insert" => format!(
            "intent_bst_i64_insert({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "bst_contains" => format!(
            "intent_bst_i64_contains({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "bst_remove" => format!(
            "intent_bst_i64_remove({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "bst_len" => format!(
            "intent_bst_i64_len({})",
            emit_expr(&args[0])
        ),
        "bst_clear" => format!(
            "intent_bst_i64_clear({})",
            emit_expr(&args[0])
        ),
        "bst_min" => format!(
            "intent_bst_i64_min({})",
            emit_expr(&args[0])
        ),
        "bst_max" => format!(
            "intent_bst_i64_max({})",
            emit_expr(&args[0])
        ),
        // Closure #329: Graph dispatch.
        "graph_new" => format!(
            "intent_graph_new(({}))",
            emit_expr(&args[0])
        ),
        "graph_add_edge" => format!(
            "intent_graph_add_edge({}, ({}), ({}), ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1]),
            emit_expr(&args[2]),
            emit_expr(&args[3])
        ),
        "graph_num_nodes" => format!(
            "intent_graph_num_nodes({})",
            emit_expr(&args[0])
        ),
        "graph_num_edges" => format!(
            "intent_graph_num_edges({})",
            emit_expr(&args[0])
        ),
        "graph_clear" => format!(
            "intent_graph_clear({})",
            emit_expr(&args[0])
        ),
        "graph_bfs_reach" => format!(
            "intent_graph_bfs_reach({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "graph_dfs_reach" => format!(
            "intent_graph_dfs_reach({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "graph_dijkstra" => format!(
            "intent_graph_dijkstra({}, ({}), ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1]),
            emit_expr(&args[2])
        ),
        "graph_has_cycle" => format!(
            "intent_graph_has_cycle({})",
            emit_expr(&args[0])
        ),
        "graph_mst_kruskal" => format!(
            "intent_graph_mst_kruskal({})",
            emit_expr(&args[0])
        ),
        "graph_mst_prim" => format!(
            "intent_graph_mst_prim({})",
            emit_expr(&args[0])
        ),
        "graph_astar" => format!(
            "intent_graph_astar({}, ({}), ({}), {})",
            emit_expr(&args[0]),
            emit_expr(&args[1]),
            emit_expr(&args[2]),
            emit_expr(&args[3])
        ),
        "graph_topo_sort" => format!(
            "intent_graph_topo_sort({}, {})",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        // Closure #330: Trie dispatch.
        "trie_new" => "intent_trie_new()".to_string(),
        "trie_insert" => format!(
            "intent_trie_insert({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "trie_contains" => format!(
            "intent_trie_contains({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "trie_starts_with" => format!(
            "intent_trie_starts_with({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "trie_delete" => format!(
            "intent_trie_delete({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "trie_clear" => format!(
            "intent_trie_clear({})",
            emit_expr(&args[0])
        ),
        "trie_len" => format!(
            "intent_trie_len({})",
            emit_expr(&args[0])
        ),
        "trie_node_count" => format!(
            "intent_trie_node_count({})",
            emit_expr(&args[0])
        ),
        // Closure #331: SkipList dispatch.
        "skiplist_new" => "intent_skiplist_i64_new()".to_string(),
        "skiplist_insert" => format!(
            "intent_skiplist_i64_insert({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "skiplist_contains" => format!(
            "intent_skiplist_i64_contains({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "skiplist_remove" => format!(
            "intent_skiplist_i64_remove({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "skiplist_len" => format!(
            "intent_skiplist_i64_len({})",
            emit_expr(&args[0])
        ),
        "skiplist_clear" => format!(
            "intent_skiplist_i64_clear({})",
            emit_expr(&args[0])
        ),
        "skiplist_min" => format!(
            "intent_skiplist_i64_min({})",
            emit_expr(&args[0])
        ),
        "skiplist_max" => format!(
            "intent_skiplist_i64_max({})",
            emit_expr(&args[0])
        ),
        // ARC 1.4e: dispatch on the (K, V) types at the call
        // site to pick the right bundle prefix. `hashmap_new`
        // reads (K, V) from result_ty; other ops read from
        // args[0].ty after stripping the Ref/RefMut. The
        // legacy (i64, i64) shape always uses the legacy
        // `intent_hashmap_i64_i64` prefix; non-(i64, i64) uses
        // the per-pair `intent_hashmap_<K_tag>_<V_tag>` form.
        "hashmap_new" => format!("{}_new()", hashmap_prefix_from_ty(result_ty)),
        "hashmap_insert" => format!(
            "{}_insert({}, ({}), ({}))",
            hashmap_prefix_from_recv(&args[0].ty),
            emit_expr(&args[0]),
            emit_expr(&args[1]),
            emit_expr(&args[2])
        ),
        "hashmap_get" => format!(
            "{}_get({}, ({}))",
            hashmap_prefix_from_recv(&args[0].ty),
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "hashmap_contains_key" => format!(
            "{}_contains_key({}, ({}))",
            hashmap_prefix_from_recv(&args[0].ty),
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "hashmap_remove" => format!(
            "{}_remove({}, ({}))",
            hashmap_prefix_from_recv(&args[0].ty),
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "hashmap_len" => format!(
            "{}_len({})",
            hashmap_prefix_from_recv(&args[0].ty),
            emit_expr(&args[0])
        ),
        "hashmap_clear" => format!(
            "{}_clear({})",
            hashmap_prefix_from_recv(&args[0].ty),
            emit_expr(&args[0])
        ),
        "hashset_remove" => format!(
            "intent_hashset_i64_remove({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        // Layer 1.3 of `unsafe.md` — `Tainted<T>` is purely a
        // type-level discipline; both `taint` and `assert_safe`
        // are identity at runtime. Wrap the operand in parens
        // so any compound expression renders cleanly.
        "taint" | "assert_safe" => format!("({})", emit_expr(&args[0])),
        // Layer 1.3 of `unsafe.md` — raw load/store. `raw_load`
        // dereferences a `*const T` / `*mut T` to read; the
        // value flows back wrapped in `Tainted<T>` at the type
        // level. `raw_store` writes through a `*mut T`. The C
        // emission is the straightforward `*p` / `*p = v`
        // expression — the unsafe surface IS the C surface here.
        "raw_load" => format!("(*({}))", emit_expr(&args[0])),
        "raw_store" => format!(
            "((*({})) = ({}), (int64_t)0)",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        // T2.1 of safety-standard arc — MMIO volatile load/store.
        // `volatile` qualifier prevents the compiler from
        // coalescing, reordering, or eliding accesses. The i64
        // address is cast through `uintptr_t` to satisfy ISO C
        // strict-aliasing on integer-to-pointer conversion.
        "mmio_read_u32" => format!(
            "(*((const volatile uint32_t*)((uintptr_t)({}))))",
            emit_expr(&args[0])
        ),
        "mmio_write_u32" => format!(
            "((*((volatile uint32_t*)((uintptr_t)({})))) = ({}), (int64_t)0)",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        // Layer 3.1 of `unsafe.md` — canary-protected heap
        // allocation. Routes through `intent_unsafe_alloc` /
        // `intent_unsafe_free` helpers emitted by
        // `emit_intent_unsafe_alloc_helpers_c_body`. The helpers
        // bracket each allocation with two i64 magic words and
        // verify both at free time.
        "unsafe_alloc" => format!("intent_unsafe_alloc({})", emit_expr(&args[0])),
        "unsafe_free" => format!("intent_unsafe_free({})", emit_expr(&args[0])),
        // Layer 3.2 of `unsafe.md` — BoundedPtr<i64> ops.
        "bptr_new" => format!(
            "intent_bptr_i64_new({}, ({}), ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1]),
            emit_expr(&args[2])
        ),
        "bptr_get" => format!(
            "intent_bptr_i64_get({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "bptr_set" => format!(
            "intent_bptr_i64_set({}, ({}), ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1]),
            emit_expr(&args[2])
        ),
        "bptr_len" => format!("intent_bptr_i64_len({})", emit_expr(&args[0])),
        // Layer 5 v2 foundation of `unsafe.md` — Region ops.
        "region_new" => "intent_region_new()".to_string(),
        "region_alloc_i64" | "region_borrow_i64" => format!(
            "intent_region_alloc_i64({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "region_len" => format!("intent_region_len({})", emit_expr(&args[0])),
        // Layer 5 lifetime-tagged ops — same machine semantics
        // as raw load/store but no Tainted wrapping (the
        // compile-time scope binding is the safety proof).
        "aref_load" => format!("(*({}))", emit_expr(&args[0])),
        "aref_store" => format!(
            "((*({})) = ({}), (int64_t)0)",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "pool_new" => "intent_pool_i64_new()".to_string(),
        "pool_alloc" => format!(
            "intent_pool_i64_alloc({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "pool_get" => format!(
            "intent_pool_i64_get({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "pool_free" => format!(
            "intent_pool_i64_free({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "hashset_new" => "intent_hashset_i64_new()".to_string(),
        "hashset_insert" => format!(
            "intent_hashset_i64_insert({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "hashset_contains" => format!(
            "intent_hashset_i64_contains({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "hashset_len" => format!(
            "intent_hashset_i64_len({})",
            emit_expr(&args[0])
        ),
        "hashset_clear" => format!(
            "intent_hashset_i64_clear({})",
            emit_expr(&args[0])
        ),
        "deque_new" => "intent_deque_i64_new()".to_string(),
        "deque_push_back" => format!(
            "intent_deque_i64_push_back({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "deque_push_front" => format!(
            "intent_deque_i64_push_front({}, ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "deque_pop_back" => format!(
            "intent_deque_i64_pop_back({})",
            emit_expr(&args[0])
        ),
        "deque_pop_front" => format!(
            "intent_deque_i64_pop_front({})",
            emit_expr(&args[0])
        ),
        "deque_peek_back" => format!(
            "intent_deque_i64_peek_back({})",
            emit_expr(&args[0])
        ),
        "deque_peek_front" => format!(
            "intent_deque_i64_peek_front({})",
            emit_expr(&args[0])
        ),
        "deque_len" => format!(
            "intent_deque_i64_len({})",
            emit_expr(&args[0])
        ),
        "deque_clear" => format!(
            "intent_deque_i64_clear({})",
            emit_expr(&args[0])
        ),
        "heap_push" | "heap_pop" | "heap_peek" | "heapify" => {
            let element = match args[0].ty.deref() {
                Type::Vec(element) => element.clone(),
                _ => unreachable!("heap_* requires Vec argument"),
            };
            if name == "heap_push" {
                format!(
                    "{}({}, ({}))",
                    vec_helper(&element, "heap_push"),
                    emit_expr(&args[0]),
                    emit_expr(&args[1])
                )
            } else {
                format!(
                    "{}({})",
                    vec_helper(&element, name),
                    emit_expr(&args[0])
                )
            }
        }
        "hash_i64" => {
            format!("intent_hash_i64(({}))", emit_expr(&args[0]))
        }
        "hash_f64" => {
            format!("intent_hash_f64(({}))", emit_expr(&args[0]))
        }
        "hash_str" => {
            format!("intent_hash_str(({}))", emit_expr(&args[0]))
        }
        "siphash_i64" => {
            format!(
                "intent_siphash_i64(({}), ({}), ({}))",
                emit_expr(&args[0]),
                emit_expr(&args[1]),
                emit_expr(&args[2])
            )
        }
        "siphash_str" => {
            format!(
                "intent_siphash_str(({}), ({}), ({}))",
                emit_expr(&args[0]),
                emit_expr(&args[1]),
                emit_expr(&args[2])
            )
        }
        "hash_combine" => {
            format!(
                "intent_hash_combine(({}), ({}))",
                emit_expr(&args[0]),
                emit_expr(&args[1])
            )
        }
        "hash_combine_3" => {
            format!(
                "intent_hash_combine(intent_hash_combine(({}), ({})), ({}))",
                emit_expr(&args[0]),
                emit_expr(&args[1]),
                emit_expr(&args[2])
            )
        }
        "hash_combine_4" => {
            format!(
                "intent_hash_combine(intent_hash_combine(intent_hash_combine(({}), ({})), ({})), ({}))",
                emit_expr(&args[0]),
                emit_expr(&args[1]),
                emit_expr(&args[2]),
                emit_expr(&args[3])
            )
        }
        "hash_pair" => {
            format!(
                "intent_hash_combine(intent_hash_i64(({})), intent_hash_i64(({})))",
                emit_expr(&args[0]),
                emit_expr(&args[1])
            )
        }
        "hash_triple" => {
            format!(
                "intent_hash_combine(intent_hash_combine(intent_hash_i64(({})), intent_hash_i64(({}))), intent_hash_i64(({})))",
                emit_expr(&args[0]),
                emit_expr(&args[1]),
                emit_expr(&args[2])
            )
        }
        "f64_hash_pair" => {
            format!(
                "intent_hash_combine(intent_hash_f64(({})), intent_hash_f64(({})))",
                emit_expr(&args[0]),
                emit_expr(&args[1])
            )
        }
        "f64_hash_triple" => {
            format!(
                "intent_hash_combine(intent_hash_combine(intent_hash_f64(({})), intent_hash_f64(({}))), intent_hash_f64(({})))",
                emit_expr(&args[0]),
                emit_expr(&args[1]),
                emit_expr(&args[2])
            )
        }
        "str_hash_pair" => {
            format!(
                "intent_hash_combine(intent_hash_str(({})), intent_hash_str(({})))",
                emit_expr(&args[0]),
                emit_expr(&args[1])
            )
        }
        "str_hash_triple" => {
            format!(
                "intent_hash_combine(intent_hash_combine(intent_hash_str(({})), intent_hash_str(({}))), intent_hash_str(({})))",
                emit_expr(&args[0]),
                emit_expr(&args[1]),
                emit_expr(&args[2])
            )
        }
        "seed_rng" => {
            format!("intent_rng_seed(({}))", emit_expr(&args[0]))
        }
        // Arc 8 step 8e — `sleep_ms(ms) -> i64`. Wraps the
        // emitted runtime helper `intent_sleep_ms` (defined in
        // the C prologue when the program references it).
        // Always returns 0; passes ms straight through.
        "sleep_ms" => {
            format!("intent_sleep_ms(({}))", emit_expr(&args[0]))
        }
        // Arc 8 step 8e proper — TCP networking primitives.
        // All resolve to runtime helpers emitted by
        // emit_intent_tcp_helpers_c when any tcp_* builtin
        // is referenced.
        "tcp_listen" => {
            format!("intent_tcp_listen(({}))", emit_expr(&args[0]))
        }
        "tcp_socket_port" => {
            format!("intent_tcp_socket_port(({}))", emit_expr(&args[0]))
        }
        "tcp_accept" => {
            format!("intent_tcp_accept(({}))", emit_expr(&args[0]))
        }
        "tcp_connect_local" => {
            format!("intent_tcp_connect_local(({}))", emit_expr(&args[0]))
        }
        "tcp_send_str" => {
            format!(
                "intent_tcp_send_str(({}), ({}))",
                emit_expr(&args[0]),
                emit_expr(&args[1])
            )
        }
        "tcp_recv" => {
            format!(
                "intent_tcp_recv(({}), ({}))",
                emit_expr(&args[0]),
                emit_expr(&args[1])
            )
        }
        "tcp_send_buf" => {
            format!(
                "intent_tcp_send_buf(({}), ({}))",
                emit_expr(&args[0]),
                emit_expr(&args[1])
            )
        }
        "tcp_close" => {
            format!("intent_tcp_close(({}))", emit_expr(&args[0]))
        }
        // Arc 8 v2 — epoll + non-blocking I/O. Each resolves to
        // a runtime helper emitted by emit_intent_epoll_helpers_c.
        "epoll_new" => "intent_epoll_new()".to_string(),
        "epoll_add_read" => format!(
            "intent_epoll_add_read(({}), ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "epoll_wait_one" => format!(
            "intent_epoll_wait_one(({}), ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "epoll_close" => format!(
            "intent_epoll_close(({}))",
            emit_expr(&args[0])
        ),
        "tcp_set_nonblocking" => format!(
            "intent_tcp_set_nonblocking(({}))",
            emit_expr(&args[0])
        ),
        "tcp_accept_nb" => format!(
            "intent_tcp_accept_nb(({}))",
            emit_expr(&args[0])
        ),
        "tcp_recv_nb" => format!(
            "intent_tcp_recv_nb(({}), ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        // Arc 8 v3.1 Phase 0 — timerfd-based non-blocking sleep.
        "sleep_ms_async" => format!(
            "intent_sleep_ms_async(({}))",
            emit_expr(&args[0])
        ),
        "sleep_ms_finish" => format!(
            "intent_sleep_ms_finish(({}))",
            emit_expr(&args[0])
        ),
        "rand_i64" => "intent_rng_next()".to_string(),
        "rand_in_range" => {
            format!(
                "intent_rng_in_range(({}), ({}))",
                emit_expr(&args[0]),
                emit_expr(&args[1])
            )
        }
        // Closure #588: rand_f64() — uniform [0,1).
        // Use 53 bits of the rng output and divide by 2^53 for exact [0,1).
        "rand_f64" => "(((double)((uint64_t)intent_rng_next() >> 11)) / 9007199254740992.0)".to_string(),
        // Closure #589: rand_in_range_f64(lo, hi) — uniform [lo, hi).
        "rand_in_range_f64" => format!(
            "({{ double __rfr_lo = ({}); double __rfr_hi = ({}); double __rfr_u = ((double)((uint64_t)intent_rng_next() >> 11)) / 9007199254740992.0; __rfr_lo + (__rfr_hi - __rfr_lo) * __rfr_u; }})",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        // Closure #590: rand_bool() — uniform coin flip.
        "rand_bool" => "((((uint64_t)intent_rng_next()) & 1ULL) != 0)".to_string(),
        // Closure #591: rand_choice(ref xs) — pick uniformly; empty → -1.
        "rand_choice" => format!(
            "({{ const intent_vec_int64_t* __rc_xs = ({}); (__rc_xs->len == 0) ? (int64_t)-1 : __rc_xs->data[(uint64_t)intent_rng_next() % __rc_xs->len]; }})",
            emit_expr(&args[0])
        ),
        // Closure #592: rand_normal() — standard normal N(0,1) via Box-Muller.
        // u1 guarded > 0 by re-sampling if it lands at exactly 0 (vanishingly rare).
        "rand_normal" => "({ double __rn_u1; do { __rn_u1 = ((double)((uint64_t)intent_rng_next() >> 11)) / 9007199254740992.0; } while (__rn_u1 == 0.0); double __rn_u2 = ((double)((uint64_t)intent_rng_next() >> 11)) / 9007199254740992.0; sqrt(-2.0 * log(__rn_u1)) * cos(6.283185307179586 * __rn_u2); })".to_string(),
        "pow" => {
            format!(
                "pow(({}), ({}))",
                emit_expr(&args[0]),
                emit_expr(&args[1])
            )
        }
        "sqrt" | "sin" | "cos" | "tan" | "floor" | "ceil"
        | "log" | "log2" | "log10" | "exp"
        // Closure #414: inverse + hyperbolic trig (libm).
        | "asin" | "acos" | "atan" | "sinh" | "cosh" | "tanh" => {
            format!("{}(({}))", name, emit_expr(&args[0]))
        }
        "atan2" => {
            format!(
                "atan2(({}), ({}))",
                emit_expr(&args[0]),
                emit_expr(&args[1])
            )
        }
        // Closure #364: float classification via <math.h>
        // macros. Cast bool-ish int result through `!= 0` so the
        // expression has explicit bool type (isfinite returns
        // non-zero on finite, zero otherwise — C semantics).
        "f64_is_nan" => format!("(isnan(({})) != 0)", emit_expr(&args[0])),
        "f64_is_inf" => format!("(isinf(({})) != 0)", emit_expr(&args[0])),
        "f64_is_finite" => format!("(isfinite(({})) != 0)", emit_expr(&args[0])),
        // Closure #367: f64 math constants (zero-arg, return f64).
        // Use math.h's named constants when available; fall back
        // to hex-float literals for portability. NaN / INFINITY
        // are C99 macros (always available with <math.h>).
        "f64_pi" => "(3.14159265358979323846)".to_string(),
        "f64_e" => "(2.71828182845904523536)".to_string(),
        "f64_inf" => "((double)INFINITY)".to_string(),
        "f64_nan" => "((double)NAN)".to_string(),
        // Closure #404: i64 / f64 boundary constants.
        // INT64_MIN's literal form needs care: -9223372036854775808
        // would parse as `-(9223372036854775808)` where the
        // positive operand overflows i64. Use the (INT64_MAX) - 1
        // form or the hex literal — we go with the explicit
        // unary form `(-INT64_MAX - 1)` for maximum portability.
        "i64_min_value" => "((int64_t)(-9223372036854775807LL - 1LL))".to_string(),
        "i64_max_value" => "((int64_t)9223372036854775807LL)".to_string(),
        // DBL_MAX is in <float.h> but math.h pulls it in
        // transitively on glibc. Be explicit by using the
        // hex-float literal that matches IEEE-754 DBL_MAX.
        "f64_max_finite" => "(1.7976931348623157e308)".to_string(),
        // Closure #415: IEEE-754 small-magnitude constants.
        // DBL_EPSILON, DBL_MIN, smallest subnormal.
        "f64_epsilon" => "(2.220446049250313e-16)".to_string(),
        "f64_min_positive" => "(2.2250738585072014e-308)".to_string(),
        "f64_min_subnormal" => "(5e-324)".to_string(),
        // Closure #405: Python-style floor division. C's `/`
        // truncates toward zero — for negative dividends with
        // positive divisors (and vice versa), `floor(a/b)` is
        // one less than the truncated quotient when there's a
        // non-zero remainder. The `(a ^ b) < 0` test detects
        // "different sign" via the high bit XOR.
        "i64_div_floor" => {
            format!(
                "({{ int64_t __da = ({}); int64_t __db = ({}); int64_t __dq = __da / __db; int64_t __dr = __da % __db; if (__dr != 0 && ((__da ^ __db) < 0)) __dq--; __dq; }})",
                emit_expr(&args[0]),
                emit_expr(&args[1])
            )
        }
        // i64_mod_floor(a, b) — always non-negative remainder
        // when b > 0; sign matches b for negative b. Defined as
        // `a - i64_div_floor(a, b) * b`.
        "i64_mod_floor" => {
            format!(
                "({{ int64_t __ma = ({}); int64_t __mb = ({}); int64_t __mr = __ma % __mb; if (__mr != 0 && ((__ma ^ __mb) < 0)) __mr += __mb; __mr; }})",
                emit_expr(&args[0]),
                emit_expr(&args[1])
            )
        }
        // Closure #408: integer log2.
        // log2_floor(n) = 63 - clz(n) for n > 0; -1 for n <= 0
        //   (we return -1 to signal "undefined" rather than abort —
        //   callers can check the sentinel).
        // log2_ceil(n) = log2_floor(n - 1) + 1 for n >= 2;
        //   0 for n == 1; -1 for n <= 0.
        "i64_log2_floor" => {
            format!(
                "({{ int64_t __lf_n = ({}); __lf_n <= 0 ? (int64_t)-1 : (int64_t)(63 - __builtin_clzll((unsigned long long)__lf_n)); }})",
                emit_expr(&args[0])
            )
        }
        "i64_log2_ceil" => {
            format!(
                "({{ int64_t __lc_n = ({}); __lc_n <= 0 ? (int64_t)-1 : (__lc_n == 1 ? (int64_t)0 : (int64_t)(64 - __builtin_clzll((unsigned long long)(__lc_n - 1)))); }})",
                emit_expr(&args[0])
            )
        }
        // Closure #409: power-of-2 helpers.
        // is_power_of_2(n): n > 0 && (n & (n-1)) == 0.
        // next_power_of_2(n): n <= 1 ? 1 : 2 ^ log2_ceil(n) =
        //   1 << (64 - clz(n - 1)).
        "i64_is_power_of_2" => {
            format!(
                "({{ int64_t __pn = ({}); (bool)(__pn > 0 && (__pn & (__pn - 1)) == 0); }})",
                emit_expr(&args[0])
            )
        }
        "i64_next_power_of_2" => {
            // Returns 1 for n <= 1; otherwise the smallest power
            // of 2 >= n. For n = 2^k exactly, returns n (since
            // log2_ceil(2^k) = k → 1 << k = 2^k).
            format!(
                "({{ int64_t __np = ({}); __np <= 1 ? (int64_t)1 : (int64_t)((int64_t)1 << (64 - __builtin_clzll((unsigned long long)(__np - 1)))); }})",
                emit_expr(&args[0])
            )
        }
        // Closure #410: saturating arithmetic — clamps to
        // INT64_MIN / INT64_MAX on overflow. Routes through
        // GCC / Clang `__builtin_*_overflow`. If overflow, the
        // saturation direction depends on the sign-prediction:
        //   add: a + b overflows positive iff both >= 0;
        //   sub: a - b overflows positive iff a >= 0 && b < 0;
        //   mul: signed-multiply overflow direction is the sign
        //        of the (mathematical) product, which equals
        //        sign(a) XOR sign(b) (≥ 0 for same-sign, < 0 for
        //        different-sign), with the special case where
        //        either operand is 0 (no overflow possible).
        "i64_saturating_add" => {
            format!(
                "({{ int64_t __sa_a = ({}); int64_t __sa_b = ({}); int64_t __sa_r; if (__builtin_add_overflow(__sa_a, __sa_b, &__sa_r)) __sa_r = (__sa_a >= 0) ? (int64_t)9223372036854775807LL : (int64_t)(-9223372036854775807LL - 1LL); __sa_r; }})",
                emit_expr(&args[0]),
                emit_expr(&args[1])
            )
        }
        "i64_saturating_sub" => {
            format!(
                "({{ int64_t __ss_a = ({}); int64_t __ss_b = ({}); int64_t __ss_r; if (__builtin_sub_overflow(__ss_a, __ss_b, &__ss_r)) __ss_r = (__ss_a >= 0) ? (int64_t)9223372036854775807LL : (int64_t)(-9223372036854775807LL - 1LL); __ss_r; }})",
                emit_expr(&args[0]),
                emit_expr(&args[1])
            )
        }
        "i64_saturating_mul" => {
            // For mul, sign of overflow is sign(a) XOR sign(b).
            // If either operand is 0, no overflow possible.
            format!(
                "({{ int64_t __sm_a = ({}); int64_t __sm_b = ({}); int64_t __sm_r; if (__builtin_mul_overflow(__sm_a, __sm_b, &__sm_r)) __sm_r = ((__sm_a ^ __sm_b) < 0) ? (int64_t)(-9223372036854775807LL - 1LL) : (int64_t)9223372036854775807LL; __sm_r; }})",
                emit_expr(&args[0]),
                emit_expr(&args[1])
            )
        }
        // Closure #411: scalar binary min / max / clamp.
        // i64 versions use plain C ternary on signed compare.
        // f64 versions emit `fmin` / `fmax` from <math.h> so
        // NaN-handling matches the IEEE-754 semantics LLVM's
        // minnum / maxnum intrinsics also use.
        "i64_min" => format!(
            "({{ int64_t __ma = ({}); int64_t __mb = ({}); __ma < __mb ? __ma : __mb; }})",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "i64_max" => format!(
            "({{ int64_t __ma = ({}); int64_t __mb = ({}); __ma > __mb ? __ma : __mb; }})",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "i64_clamp" => format!(
            "({{ int64_t __cx = ({}); int64_t __clo = ({}); int64_t __chi = ({}); __cx < __clo ? __clo : (__cx > __chi ? __chi : __cx); }})",
            emit_expr(&args[0]),
            emit_expr(&args[1]),
            emit_expr(&args[2])
        ),
        // Closure #458: i64_min_3(a, b, c) — three-arg min via
        // chained ternary.
        "i64_min_3" => format!(
            "({{ int64_t __m3a = ({}); int64_t __m3b = ({}); int64_t __m3c = ({}); int64_t __m3ab = __m3a < __m3b ? __m3a : __m3b; __m3ab < __m3c ? __m3ab : __m3c; }})",
            emit_expr(&args[0]),
            emit_expr(&args[1]),
            emit_expr(&args[2])
        ),
        // Closure #459: i64_max_3(a, b, c) — three-arg max.
        "i64_max_3" => format!(
            "({{ int64_t __M3a = ({}); int64_t __M3b = ({}); int64_t __M3c = ({}); int64_t __M3ab = __M3a > __M3b ? __M3a : __M3b; __M3ab > __M3c ? __M3ab : __M3c; }})",
            emit_expr(&args[0]),
            emit_expr(&args[1]),
            emit_expr(&args[2])
        ),
        // Closure #460: f64_min_3 — three-arg float min via
        // chained fmin (IEEE-754 NaN-aware via libm).
        "f64_min_3" => format!(
            "fmin(fmin(({}), ({})), ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1]),
            emit_expr(&args[2])
        ),
        // Closure #461: f64_max_3 — three-arg float max via fmax.
        "f64_max_3" => format!(
            "fmax(fmax(({}), ({})), ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1]),
            emit_expr(&args[2])
        ),
        "f64_min" => format!("fmin(({}), ({}))", emit_expr(&args[0]), emit_expr(&args[1])),
        "f64_max" => format!("fmax(({}), ({}))", emit_expr(&args[0]), emit_expr(&args[1])),
        "f64_clamp" => format!(
            "({{ double __cx = ({}); double __clo = ({}); double __chi = ({}); __cx < __clo ? __clo : (__cx > __chi ? __chi : __cx); }})",
            emit_expr(&args[0]),
            emit_expr(&args[1]),
            emit_expr(&args[2])
        ),
        // Closure #412: integer square root via Heron / Newton's
        // method on integers. Returns 0 for negative input;
        // returns n for n in {0, 1}; otherwise iterates
        //   x = n; y = (x + n/x) / 2
        //   while (y < x): x = y; y = (x + n/x) / 2
        // and returns x. Converges in O(log log n) iterations.
        "i64_isqrt" => format!(
            "({{ int64_t __n = ({}); int64_t __ir; if (__n < 0) __ir = 0; else if (__n < 2) __ir = __n; else {{ int64_t __ix = __n; int64_t __iy = (__ix + __n / __ix) / 2; while (__iy < __ix) {{ __ix = __iy; __iy = (__ix + __n / __ix) / 2; }} __ir = __ix; }} __ir; }})",
            emit_expr(&args[0])
        ),
        // Closure #413: trig / geometry helpers.
        // f64_hypot uses libm `hypot()` — the overflow-safe form
        // of sqrt(a*a + b*b). Angle-conversion ops use plain
        // multiply with the inverse constant. π/180 and 180/π
        // are written as full-precision doubles.
        "f64_hypot" => format!(
            "hypot(({}), ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "f64_to_radians" => format!(
            "(({}) * 0.017453292519943295)",
            emit_expr(&args[0])
        ),
        "f64_to_degrees" => format!(
            "(({}) * 57.29577951308232)",
            emit_expr(&args[0])
        ),
        // Closure #416: IEEE-754 math primitives from libm.
        // copysign(magnitude, sign): |magnitude| with sign(sign).
        // fma(a, b, c): fused a*b+c, single rounding.
        // remainder(x, y): IEEE 754 remainder (round-to-even
        // quotient).
        "f64_copysign" => format!(
            "copysign(({}), ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "f64_fma" => format!(
            "fma(({}), ({}), ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1]),
            emit_expr(&args[2])
        ),
        "f64_remainder" => format!(
            "remainder(({}), ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        // Closure #417: IEEE-754 classification predicates.
        // C's <math.h> macros handle these — wrap in `!= 0` so
        // the result has explicit bool type.
        "f64_is_normal" => format!("(isnormal(({})) != 0)", emit_expr(&args[0])),
        "f64_is_subnormal" => format!(
            "(fpclassify(({})) == FP_SUBNORMAL)",
            emit_expr(&args[0])
        ),
        "f64_sign_bit" => format!("(signbit(({})) != 0)", emit_expr(&args[0])),
        // Closure #418: next representable double via libm's
        // nextafter(). next_up walks toward +inf; next_down
        // walks toward -inf.
        "f64_next_up" => format!(
            "nextafter(({}), (double)INFINITY)",
            emit_expr(&args[0])
        ),
        "f64_next_down" => format!(
            "nextafter(({}), -(double)INFINITY)",
            emit_expr(&args[0])
        ),
        // Closure #419: integer division companions to div_floor.
        // div_ceil(a, b) rounds quotient toward +infinity:
        //   q = a/b (truncation), r = a%b; if r != 0 and signs
        //   match, bump q by 1.
        // div_round(a, b) rounds half away from zero. Computed
        // via absolute values: q = (|a| + |b|/2) / |b|; sign is
        // negative iff signs differ.
        "i64_div_ceil" => format!(
            "({{ int64_t __a = ({}); int64_t __b = ({}); int64_t __q = __a / __b; int64_t __r = __a % __b; if (__r != 0 && ((__a < 0) == (__b < 0))) __q += 1; __q; }})",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "i64_div_round" => format!(
            "({{ int64_t __a = ({}); int64_t __b = ({}); int64_t __aa = __a < 0 ? -__a : __a; int64_t __ab = __b < 0 ? -__b : __b; int64_t __q = (__aa + __ab / 2) / __ab; ((__a < 0) != (__b < 0)) ? -__q : __q; }})",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        // Closure #420: float truncation + fractional part.
        // trunc(x) truncates toward zero (libm). frac(x) returns
        // the signed fractional part such that x = trunc(x) + frac(x).
        "f64_trunc" => format!("trunc(({}))", emit_expr(&args[0])),
        "f64_frac" => format!(
            "({{ double __fx = ({}); __fx - trunc(__fx); }})",
            emit_expr(&args[0])
        ),
        // Closure #421: decimal digit count. n = 0 → 1 (the
        // digit '0'). For negative n, count the digits of |n|;
        // the sign is not a digit. INT64_MIN edge: cast through
        // uint64_t so -INT64_MIN doesn't overflow signed.
        "i64_count_digits" => format!(
            "({{ int64_t __n = ({}); int64_t __c; if (__n == 0) {{ __c = 1; }} else {{ uint64_t __un = (__n < 0) ? ((uint64_t)(-(__n + 1)) + 1) : (uint64_t)__n; __c = 0; while (__un > 0) {{ __c += 1; __un /= 10; }} }} __c; }})",
            emit_expr(&args[0])
        ),
        // Closure #422: floor(log10(n)). Returns 0 for n <= 0
        // (defensive — log10 of non-positive is undefined).
        "i64_log10_floor" => format!(
            "({{ int64_t __n = ({}); int64_t __c; if (__n <= 0) {{ __c = 0; }} else {{ __c = 0; while (__n >= 10) {{ __n /= 10; __c += 1; }} }} __c; }})",
            emit_expr(&args[0])
        ),
        // Closure #423: ceil(log10(n)). Smallest k such that
        // 10^k >= n. Returns 0 for n <= 1.
        "i64_log10_ceil" => format!(
            "({{ int64_t __n = ({}); int64_t __c; if (__n <= 1) {{ __c = 0; }} else {{ __c = 0; int64_t __m = 1; while (__m < __n) {{ __m *= 10; __c += 1; }} }} __c; }})",
            emit_expr(&args[0])
        ),
        // Closure #424: modular exponentiation (a^b mod m) via
        // square-and-multiply. Returns 0 for m <= 0, b < 0, or
        // m == 1 (defensive defaults). Assumes m * m fits in
        // i64 (i.e., m < 2^31.5 ≈ 3.04e9) — for larger m the
        // r*a or a*a multiplications can overflow; caller must
        // use a wider type / external lib in that case.
        "i64_pow_mod" => format!(
            "({{ int64_t __pma = ({}); int64_t __pmb = ({}); int64_t __pmm = ({}); int64_t __pmr; if (__pmm <= 1 || __pmb < 0) {{ __pmr = 0; }} else {{ __pmr = 1; __pma = ((__pma % __pmm) + __pmm) % __pmm; while (__pmb > 0) {{ if (__pmb & 1) __pmr = (__pmr * __pma) % __pmm; __pma = (__pma * __pma) % __pmm; __pmb = __pmb >> 1; }} }} __pmr; }})",
            emit_expr(&args[0]),
            emit_expr(&args[1]),
            emit_expr(&args[2])
        ),
        // Closure #425: trial-division primality with 6k+/-1
        // optimization. Returns false for n < 2; true for n in
        // {2, 3}; rejects even / divisible-by-3; iterates only
        // candidates of the form 6k+/-1 up to sqrt(n).
        "i64_is_prime" => format!(
            "({{ int64_t __ipn = ({}); bool __ipr; if (__ipn < 2) {{ __ipr = false; }} else if (__ipn < 4) {{ __ipr = true; }} else if (__ipn % 2 == 0 || __ipn % 3 == 0) {{ __ipr = false; }} else {{ __ipr = true; int64_t __ipi = 5; while (__ipi * __ipi <= __ipn) {{ if (__ipn % __ipi == 0 || __ipn % (__ipi + 2) == 0) {{ __ipr = false; break; }} __ipi += 6; }} }} __ipr; }})",
            emit_expr(&args[0])
        ),
        // Closure #427: saturating factorial. Returns 0 for
        // negative n (defensive — n! undefined for negative).
        // For n > 20, returns INT64_MAX (21! > 2^63 - 1). For
        // n in [0, 20], computes by direct multiplication.
        "i64_factorial" => format!(
            "({{ int64_t __fn = ({}); int64_t __fr; if (__fn < 0) {{ __fr = 0; }} else if (__fn > 20) {{ __fr = (int64_t)9223372036854775807LL; }} else {{ __fr = 1; for (int64_t __fi = 2; __fi <= __fn; __fi += 1) {{ __fr *= __fi; }} }} __fr; }})",
            emit_expr(&args[0])
        ),
        // Closure #428: saturating Fibonacci. F(0) = 0, F(1) = 1,
        // F(n) = F(n-1) + F(n-2). F(92) = 7540113804746346429 is
        // the largest Fibonacci that fits in i64; F(93) overflows.
        // Returns 0 for negative n; saturates to INT64_MAX for n > 92.
        "i64_fibonacci" => format!(
            "({{ int64_t __fbn = ({}); int64_t __fbr; if (__fbn < 0) {{ __fbr = 0; }} else if (__fbn > 92) {{ __fbr = (int64_t)9223372036854775807LL; }} else if (__fbn < 2) {{ __fbr = __fbn; }} else {{ int64_t __fba = 0; int64_t __fbb = 1; for (int64_t __fbi = 2; __fbi <= __fbn; __fbi += 1) {{ int64_t __fbt = __fba + __fbb; __fba = __fbb; __fbb = __fbt; }} __fbr = __fbb; }} __fbr; }})",
            emit_expr(&args[0])
        ),
        // Closure #431: binomial coefficient C(n, k).
        //   returns 0 for k < 0, k > n, n < 0 (defensive)
        //   returns 1 for k == 0 or k == n
        //   uses symmetry C(n, k) = C(n, n-k) to minimize iterations
        //   iterative formula: r = r * (n - i + 1) / i
        // Saturates to INT64_MAX on intermediate overflow
        // (detected via __builtin_mul_overflow). Even when the
        // final C(n, k) would fit in i64, the running product
        // r * (n - i + 1) can overflow before the divide;
        // saturating makes overflow visible to the caller.
        "i64_binomial" => format!(
            "({{ int64_t __bcn = ({}); int64_t __bck = ({}); int64_t __bcr; if (__bck < 0 || __bcn < 0 || __bck > __bcn) {{ __bcr = 0; }} else if (__bck == 0 || __bck == __bcn) {{ __bcr = 1; }} else {{ if (__bck > __bcn - __bck) __bck = __bcn - __bck; __bcr = 1; bool __bc_ov = false; for (int64_t __bci = 1; __bci <= __bck && !__bc_ov; __bci += 1) {{ int64_t __bc_prod; if (__builtin_mul_overflow(__bcr, __bcn - __bci + 1, &__bc_prod)) {{ __bcr = (int64_t)9223372036854775807LL; __bc_ov = true; }} else {{ __bcr = __bc_prod / __bci; }} }} }} __bcr; }})",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        // Closure #447: permutation count P(n, k) = n! / (n-k)!.
        // Iterative product (n) * (n-1) * ... * (n-k+1) — exact
        // i64 path with overflow detection. Saturates to
        // INT64_MAX on overflow; returns 0 for k < 0, n < 0, or
        // k > n (no such permutation).
        "i64_perm" => format!(
            "({{ int64_t __pmn = ({}); int64_t __pmk = ({}); int64_t __pmr; if (__pmk < 0 || __pmn < 0 || __pmk > __pmn) {{ __pmr = 0; }} else if (__pmk == 0) {{ __pmr = 1; }} else {{ __pmr = 1; bool __pm_ov = false; for (int64_t __pmi = 0; __pmi < __pmk && !__pm_ov; __pmi += 1) {{ int64_t __pm_prod; if (__builtin_mul_overflow(__pmr, __pmn - __pmi, &__pm_prod)) {{ __pmr = (int64_t)9223372036854775807LL; __pm_ov = true; }} else {{ __pmr = __pm_prod; }} }} }} __pmr; }})",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        // Closure #446: wrap value into [lo, hi) using floor-mod.
        // For empty range (hi <= lo), returns x unchanged.
        // Useful for cyclic indexing (toroidal grids) and angle
        // normalization (e.g., wrap to [0, 360) or [-π, π)).
        "i64_wrap" => format!(
            "({{ int64_t __iwx = ({}); int64_t __iwlo = ({}); int64_t __iwhi = ({}); int64_t __iwr = __iwhi - __iwlo; int64_t __iwout; if (__iwr <= 0) {{ __iwout = __iwx; }} else {{ int64_t __iwrel = __iwx - __iwlo; int64_t __iwm = __iwrel % __iwr; if (__iwm < 0) __iwm += __iwr; __iwout = __iwlo + __iwm; }} __iwout; }})",
            emit_expr(&args[0]),
            emit_expr(&args[1]),
            emit_expr(&args[2])
        ),
        "f64_wrap" => format!(
            "({{ double __fwx = ({}); double __fwlo = ({}); double __fwhi = ({}); double __fwr = __fwhi - __fwlo; double __fwout; if (__fwr <= 0.0) {{ __fwout = __fwx; }} else {{ double __fwrel = __fwx - __fwlo; double __fwm = fmod(__fwrel, __fwr); if (__fwm < 0.0) __fwm += __fwr; __fwout = __fwlo + __fwm; }} __fwout; }})",
            emit_expr(&args[0]),
            emit_expr(&args[1]),
            emit_expr(&args[2])
        ),
        // Closure #448: f64_mod_floor(x, y) — Python-style float
        // modulo. result = x - y * floor(x / y). Sign of result
        // matches sign of y. For y == 0, returns 0 (defensive).
        "f64_mod_floor" => format!(
            "({{ double __mfx = ({}); double __mfy = ({}); __mfy == 0.0 ? 0.0 : (__mfx - __mfy * floor(__mfx / __mfy)); }})",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        // Closure #444: overflow-safe average using bit-twiddling.
        // (a & b) + ((a ^ b) >> 1) never overflows even when
        // a + b would (e.g., avg(INT64_MAX, INT64_MAX) = INT64_MAX).
        // Uses arithmetic shift right so the result floors toward
        // -infinity for mixed-sign inputs.
        "i64_avg" => format!(
            "({{ int64_t __ava = ({}); int64_t __avb = ({}); (__ava & __avb) + ((__ava ^ __avb) >> 1); }})",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        // Closure #429: neural net activations.
        // sigmoid(x) = 1 / (1 + exp(-x))
        // softsign(x) = x / (1 + |x|)
        "f64_sigmoid" => format!(
            "({{ double __sx = ({}); 1.0 / (1.0 + exp(-__sx)); }})",
            emit_expr(&args[0])
        ),
        "f64_softsign" => format!(
            "({{ double __ssx = ({}); __ssx / (1.0 + fabs(__ssx)); }})",
            emit_expr(&args[0])
        ),
        // Closure #430: GLSL-style step functions.
        // step(edge, x) = x < edge ? 0 : 1
        // smoothstep(edge0, edge1, x) = t*t*(3 - 2*t) where
        //   t = clamp((x - edge0) / (edge1 - edge0), 0, 1)
        "f64_step" => format!(
            "((({}) < ({})) ? 0.0 : 1.0)",
            emit_expr(&args[1]),
            emit_expr(&args[0])
        ),
        "f64_smoothstep" => format!(
            "({{ double __sse0 = ({}); double __sse1 = ({}); double __ssx2 = ({}); double __sst = (__ssx2 - __sse0) / (__sse1 - __sse0); if (__sst < 0.0) __sst = 0.0; else if (__sst > 1.0) __sst = 1.0; __sst * __sst * (3.0 - 2.0 * __sst); }})",
            emit_expr(&args[0]),
            emit_expr(&args[1]),
            emit_expr(&args[2])
        ),
        // Closure #445: inverse linear interpolation.
        // inv_lerp(a, b, x) = (x - a) / (b - a). Returns
        // t such that lerp(a, b, t) ≈ x.
        "f64_inv_lerp" => format!(
            "({{ double __ila = ({}); double __ilb = ({}); double __ilx = ({}); (__ilx - __ila) / (__ilb - __ila); }})",
            emit_expr(&args[0]),
            emit_expr(&args[1]),
            emit_expr(&args[2])
        ),
        // Chebyshev (L^∞) distance: max(|x|, |y|).
        "f64_chebyshev" => format!(
            "({{ double __cba = fabs(({})); double __cbb = fabs(({})); __cba > __cbb ? __cba : __cbb; }})",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        // Closure #449: L^1 (Manhattan) norm: |a| + |b|.
        "f64_l1_norm" => format!(
            "(fabs(({})) + fabs(({})))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        // Closure #449: ceil(sqrt(n)). Returns 0 for n <= 0.
        // For positive n, computes floor(sqrt(n)) via Newton's
        // method then bumps by 1 unless n is a perfect square.
        "i64_isqrt_ceil" => format!(
            "({{ int64_t __isn = ({}); int64_t __isr; if (__isn <= 0) {{ __isr = 0; }} else if (__isn < 2) {{ __isr = __isn; }} else {{ int64_t __ix = __isn; int64_t __iy = (__ix + __isn / __ix) / 2; while (__iy < __ix) {{ __ix = __iy; __iy = (__ix + __isn / __ix) / 2; }} __isr = (__ix * __ix == __isn) ? __ix : (__ix + 1); }} __isr; }})",
            emit_expr(&args[0])
        ),
        // Closure #468: is_perfect_square(n) — n is a non-negative
        // perfect square iff floor(sqrt(n))^2 == n.
        "i64_is_perfect_square" => format!(
            "({{ int64_t __ipsn = ({}); bool __ipsr; if (__ipsn < 0) {{ __ipsr = false; }} else if (__ipsn < 2) {{ __ipsr = true; }} else {{ int64_t __ix = __ipsn; int64_t __iy = (__ix + __ipsn / __ix) / 2; while (__iy < __ix) {{ __ix = __iy; __iy = (__ix + __ipsn / __ix) / 2; }} __ipsr = (__ix * __ix == __ipsn); }} __ipsr; }})",
            emit_expr(&args[0])
        ),
        // Closure #469: divisor count τ(n). Walk i from 1 to
        // sqrt(n); for each divisor, count its pair. Subtract 1
        // if n is a perfect square (sqrt counted twice).
        // Returns 0 for n <= 0 (defensive).
        "i64_divisor_count" => format!(
            "({{ int64_t __dcn = ({}); int64_t __dcr; if (__dcn <= 0) {{ __dcr = 0; }} else {{ __dcr = 0; int64_t __dci = 1; while (__dci * __dci <= __dcn) {{ if (__dcn % __dci == 0) {{ if (__dci * __dci == __dcn) __dcr += 1; else __dcr += 2; }} __dci += 1; }} }} __dcr; }})",
            emit_expr(&args[0])
        ),
        // Closure #470: divisor sum σ(n) — sum of positive
        // divisors of n. Same structure as #469 but adds
        // (i + n/i) per divisor pair instead of counting.
        "i64_divisor_sum" => format!(
            "({{ int64_t __dsn = ({}); int64_t __dsr; if (__dsn <= 0) {{ __dsr = 0; }} else {{ __dsr = 0; int64_t __dsi = 1; while (__dsi * __dsi <= __dsn) {{ if (__dsn % __dsi == 0) {{ if (__dsi * __dsi == __dsn) __dsr += __dsi; else __dsr += __dsi + __dsn / __dsi; }} __dsi += 1; }} }} __dsr; }})",
            emit_expr(&args[0])
        ),
        // Closure #471: Euler's totient φ(n) — count of k in
        // [1, n] coprime to n. Uses Euler product:
        //   φ(n) = n · prod_{p|n} (1 - 1/p)
        // implemented iteratively by `result -= result/p` for
        // each prime p dividing n (found by trial division).
        "i64_totient" => format!(
            "({{ int64_t __ttn = ({}); int64_t __ttr; if (__ttn <= 0) {{ __ttr = 0; }} else {{ __ttr = __ttn; int64_t __ttm = __ttn; int64_t __tti = 2; while (__tti * __tti <= __ttm) {{ if (__ttm % __tti == 0) {{ __ttr -= __ttr / __tti; while (__ttm % __tti == 0) __ttm /= __tti; }} __tti += 1; }} if (__ttm > 1) __ttr -= __ttr / __ttm; }} __ttr; }})",
            emit_expr(&args[0])
        ),
        // Closure #472: radical rad(n) — product of distinct
        // prime factors of n. rad(12) = 2·3 = 6. rad(prime) = prime.
        // Returns 0 for n <= 0 (defensive).
        "i64_radical" => format!(
            "({{ int64_t __rdn = ({}); int64_t __rdr; if (__rdn <= 0) {{ __rdr = 0; }} else {{ __rdr = 1; int64_t __rdm = __rdn; int64_t __rdi = 2; while (__rdi * __rdi <= __rdm) {{ if (__rdm % __rdi == 0) {{ __rdr *= __rdi; while (__rdm % __rdi == 0) __rdm /= __rdi; }} __rdi += 1; }} if (__rdm > 1) __rdr *= __rdm; }} __rdr; }})",
            emit_expr(&args[0])
        ),
        // Closure #473: next prime ≥ n. Returns 2 for n ≤ 2.
        // Bumps odd candidates by 2 and uses 6k±1 trial division
        // inline for primality testing.
        "i64_next_prime" => format!(
            "({{ int64_t __npn = ({}); int64_t __npr; if (__npn <= 2) {{ __npr = 2; }} else if (__npn == 3) {{ __npr = 3; }} else {{ int64_t __npc = (__npn % 2 == 0) ? __npn + 1 : __npn; while (1) {{ bool __np_isp; if (__npc % 3 == 0) {{ __np_isp = (__npc == 3); }} else {{ __np_isp = true; int64_t __npi = 5; while (__npi * __npi <= __npc) {{ if (__npc % __npi == 0 || __npc % (__npi + 2) == 0) {{ __np_isp = false; break; }} __npi += 6; }} }} if (__np_isp) {{ __npr = __npc; break; }} __npc += 2; }} }} __npr; }})",
            emit_expr(&args[0])
        ),
        // Closures #476-#479: single-bit ops.
        // set_bit(n, i) = n | (1 << i)
        // clear_bit(n, i) = n & ~(1 << i)
        // toggle_bit(n, i) = n ^ (1 << i)
        // test_bit(n, i) = (n >> i) & 1 != 0
        "i64_set_bit" => format!(
            "(({}) | ((int64_t)1 << ({})))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "i64_clear_bit" => format!(
            "(({}) & ~((int64_t)1 << ({})))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "i64_toggle_bit" => format!(
            "(({}) ^ ((int64_t)1 << ({})))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "i64_test_bit" => format!(
            "((((uint64_t)({}) >> ({})) & 1) != 0)",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        // Closure #573: i64_byte_at(x, i) — byte i (0..=7) of x as 0-255.
        // Out-of-range i → 0 (defensive).
        "i64_byte_at" => format!(
            "({{ int64_t __bax = ({}); int64_t __bai = ({}); (int64_t)((__bai < 0 || __bai > 7) ? 0 : (((uint64_t)__bax >> (__bai * 8)) & 0xFFULL)); }})",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        // Closure #574: i64_set_byte(x, i, b) — set byte i of x to (b & 0xFF).
        // Out-of-range i → x unchanged (defensive).
        "i64_set_byte" => format!(
            "({{ int64_t __sbx = ({}); int64_t __sbi = ({}); int64_t __sbb = ({}); (__sbi < 0 || __sbi > 7) ? __sbx : (int64_t)(((uint64_t)__sbx & ~(0xFFULL << (__sbi * 8))) | (((uint64_t)__sbb & 0xFFULL) << (__sbi * 8))); }})",
            emit_expr(&args[0]),
            emit_expr(&args[1]),
            emit_expr(&args[2])
        ),
        // Closures #575/#576: count leading/trailing ones — invert + count zeros.
        // clz/ctz of 0 is UB; guard with the all-ones special case (~x == 0).
        "i64_count_leading_ones" => format!(
            "({{ uint64_t __cl = ~((uint64_t)({})); (int64_t)(__cl == 0 ? 64 : __builtin_clzll(__cl)); }})",
            emit_expr(&args[0])
        ),
        "i64_count_trailing_ones" => format!(
            "({{ uint64_t __ct = ~((uint64_t)({})); (int64_t)(__ct == 0 ? 64 : __builtin_ctzll(__ct)); }})",
            emit_expr(&args[0])
        ),
        // Closure #597: i64_parity — odd popcount; xor of all bits.
        "i64_parity" => format!(
            "((int64_t)(__builtin_popcountll((uint64_t)({})) & 1))",
            emit_expr(&args[0])
        ),
        // Closure #598: i64_mod_pos(x, m) — always-positive mod for m > 0.
        // If m == 0 → returns 0 (defensive). If m < 0, undefined-but-defined: uses |m|.
        "i64_mod_pos" => format!(
            "({{ int64_t __mpx = ({}); int64_t __mpm = ({}); if (__mpm < 0) __mpm = -__mpm; (__mpm == 0) ? (int64_t)0 : (int64_t)(((__mpx % __mpm) + __mpm) % __mpm); }})",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        // Closure #599: i64_cube_root — integer cube root.
        // libm cbrt seeds, then fix-up loop nails the floor in case of f64 rounding.
        "i64_cube_root" => format!(
            "({{ int64_t __crx = ({}); int64_t __crn = __crx < 0 ? -__crx : __crx; int64_t __crr = (int64_t)cbrt((double)__crn); if (__crr < 0) __crr = 0; while ((__crr + 1) * (__crr + 1) * (__crr + 1) <= __crn) __crr++; while (__crr > 0 && __crr * __crr * __crr > __crn) __crr--; __crx < 0 ? -__crr : __crr; }})",
            emit_expr(&args[0])
        ),
        // Closure #600: f64_pow_int(base, k) — integer-exponent power.
        // k < 0 → 1 / pow(base, -k). k == 0 → 1.0.
        "f64_pow_int" => format!(
            "({{ double __fpb = ({}); int64_t __fpk = ({}); double __fpr = 1.0; int64_t __fpn = __fpk < 0 ? -__fpk : __fpk; for (int64_t __i = 0; __i < __fpn; __i++) __fpr *= __fpb; (__fpk < 0) ? (1.0 / __fpr) : __fpr; }})",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        // Closure #601: f64_round_to_multiple(x, m) — round x to nearest multiple of m.
        // m <= 0 → x unchanged.
        "f64_round_to_multiple" => format!(
            "({{ double __frx = ({}); double __frm = ({}); (__frm <= 0.0) ? __frx : (round(__frx / __frm) * __frm); }})",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        // Closure #602: f64_quadratic_root(a, b, c) — positive-discriminant root of ax^2 + bx + c.
        // Returns (-b + sqrt(b^2 - 4ac)) / (2a). NaN on negative discriminant or a==0.
        "f64_quadratic_root" => format!(
            "({{ double __qra = ({}); double __qrb = ({}); double __qrc = ({}); double __disc = __qrb * __qrb - 4.0 * __qra * __qrc; (__qra == 0.0 || __disc < 0.0) ? (0.0 / 0.0) : ((-__qrb + sqrt(__disc)) / (2.0 * __qra)); }})",
            emit_expr(&args[0]),
            emit_expr(&args[1]),
            emit_expr(&args[2])
        ),
        // Closure #480: reverse_bits — bit-level reversal via
        // standard parallel-swap sequence.
        "i64_reverse_bits" => format!(
            "({{ uint64_t __rbx = (uint64_t)({}); __rbx = ((__rbx >> 1) & 0x5555555555555555ULL) | ((__rbx & 0x5555555555555555ULL) << 1); __rbx = ((__rbx >> 2) & 0x3333333333333333ULL) | ((__rbx & 0x3333333333333333ULL) << 2); __rbx = ((__rbx >> 4) & 0x0F0F0F0F0F0F0F0FULL) | ((__rbx & 0x0F0F0F0F0F0F0F0FULL) << 4); __rbx = ((__rbx >> 8) & 0x00FF00FF00FF00FFULL) | ((__rbx & 0x00FF00FF00FF00FFULL) << 8); __rbx = ((__rbx >> 16) & 0x0000FFFF0000FFFFULL) | ((__rbx & 0x0000FFFF0000FFFFULL) << 16); __rbx = (__rbx >> 32) | (__rbx << 32); (int64_t)__rbx; }})",
            emit_expr(&args[0])
        ),
        // Closure #475: modular multiplicative inverse via the
        // extended Euclidean algorithm. Returns x s.t.
        // (a · x) mod m == 1; returns 0 if no inverse exists
        // (gcd(a, m) != 1) or m ≤ 1 (defensive).
        "i64_mod_inverse" => format!(
            "({{ int64_t __mia = ({}); int64_t __mim = ({}); int64_t __mir; if (__mim <= 1) {{ __mir = 0; }} else {{ int64_t __mi_g = __mim; int64_t __mi_x = 0; int64_t __mi_og = ((__mia % __mim) + __mim) % __mim; int64_t __mi_ox = 1; while (__mi_og != 0) {{ int64_t __mi_q = __mi_g / __mi_og; int64_t __mi_tmp = __mi_g - __mi_q * __mi_og; __mi_g = __mi_og; __mi_og = __mi_tmp; __mi_tmp = __mi_x - __mi_q * __mi_ox; __mi_x = __mi_ox; __mi_ox = __mi_tmp; }} if (__mi_g != 1) {{ __mir = 0; }} else {{ __mir = ((__mi_x % __mim) + __mim) % __mim; }} }} __mir; }})",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        // Closure #474: largest prime ≤ n. Returns 0 for n < 2.
        // n=2 → 2; n=3 → 3; else step odd candidates downward.
        "i64_prev_prime" => format!(
            "({{ int64_t __ppn = ({}); int64_t __ppr; if (__ppn < 2) {{ __ppr = 0; }} else if (__ppn == 2) {{ __ppr = 2; }} else if (__ppn < 5) {{ __ppr = 3; }} else {{ int64_t __ppc = (__ppn % 2 == 0) ? __ppn - 1 : __ppn; while (1) {{ bool __pp_isp; if (__ppc < 5) {{ __pp_isp = (__ppc == 2 || __ppc == 3); }} else if (__ppc % 3 == 0) {{ __pp_isp = false; }} else {{ __pp_isp = true; int64_t __ppi = 5; while (__ppi * __ppi <= __ppc) {{ if (__ppc % __ppi == 0 || __ppc % (__ppi + 2) == 0) {{ __pp_isp = false; break; }} __ppi += 6; }} }} if (__pp_isp) {{ __ppr = __ppc; break; }} __ppc -= 2; if (__ppc < 2) {{ __ppr = 2; break; }} }} }} __ppr; }})",
            emit_expr(&args[0])
        ),
        // Closure #438: quintic smoothstep. Polynomial
        // 6t^5 - 15t^4 + 10t^3 has zero first AND second
        // derivatives at t=0 and t=1, giving smoother
        // (C^2-continuous) transitions than the cubic form.
        "f64_smoothstep5" => format!(
            "({{ double __s5e0 = ({}); double __s5e1 = ({}); double __s5x = ({}); double __s5t = (__s5x - __s5e0) / (__s5e1 - __s5e0); if (__s5t < 0.0) __s5t = 0.0; else if (__s5t > 1.0) __s5t = 1.0; __s5t * __s5t * __s5t * (__s5t * (__s5t * 6.0 - 15.0) + 10.0); }})",
            emit_expr(&args[0]),
            emit_expr(&args[1]),
            emit_expr(&args[2])
        ),
        // Closure #432: ML activations.
        // relu(x) = max(0, x)
        // leaky_relu(x, alpha) = x if x >= 0 else alpha * x
        // softplus(x) = log(1 + exp(x)) — smooth relu
        "f64_relu" => format!(
            "({{ double __rx = ({}); __rx > 0.0 ? __rx : 0.0; }})",
            emit_expr(&args[0])
        ),
        "f64_leaky_relu" => format!(
            "({{ double __lrx = ({}); double __lra = ({}); __lrx >= 0.0 ? __lrx : __lra * __lrx; }})",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "f64_softplus" => format!(
            "log(1.0 + exp(({})))",
            emit_expr(&args[0])
        ),
        // Closure #439: additional ML primitives.
        // swish(x) = x * sigmoid(x) = x / (1 + exp(-x))
        // logit(x) = log(x / (1 - x))  — inverse of sigmoid
        "f64_swish" => format!(
            "({{ double __wx = ({}); __wx / (1.0 + exp(-__wx)); }})",
            emit_expr(&args[0])
        ),
        "f64_logit" => format!(
            "({{ double __lx = ({}); log(__lx / (1.0 - __lx)); }})",
            emit_expr(&args[0])
        ),
        // Closure #440: signal-processing + safe division.
        // sinc(x) = sin(x)/x  with sinc(0) = 1 (removable
        // singularity). Caller responsible for catastrophic
        // cancellation near 0; for high-precision near-zero
        // work, use Taylor approx 1 - x²/6 + x⁴/120 - ...
        // safe_div(a, b, default) = a/b if b != 0, else default.
        "f64_sinc" => format!(
            "({{ double __sicx = ({}); __sicx == 0.0 ? 1.0 : sin(__sicx) / __sicx; }})",
            emit_expr(&args[0])
        ),
        "f64_safe_div" => format!(
            "({{ double __sda = ({}); double __sdb = ({}); double __sdd = ({}); __sdb == 0.0 ? __sdd : __sda / __sdb; }})",
            emit_expr(&args[0]),
            emit_expr(&args[1]),
            emit_expr(&args[2])
        ),
        // Closure #450: defensive math.
        // safe_sqrt(x): sqrt(x) for x >= 0, else 0.
        // i64_safe_div(a, b, default): a/b for b != 0, else default.
        "f64_safe_sqrt" => format!(
            "({{ double __ssx = ({}); __ssx < 0.0 ? 0.0 : sqrt(__ssx); }})",
            emit_expr(&args[0])
        ),
        "i64_safe_div" => format!(
            "({{ int64_t __isda = ({}); int64_t __isdb = ({}); int64_t __isdd = ({}); __isdb == 0 ? __isdd : __isda / __isdb; }})",
            emit_expr(&args[0]),
            emit_expr(&args[1]),
            emit_expr(&args[2])
        ),
        // Closure #451: safe_log(x, default) and geometric_mean.
        // safe_log: log(x) for x > 0, else default.
        // geometric_mean: sqrt(a*b) for a*b >= 0, else 0.
        "f64_safe_log" => format!(
            "({{ double __slx = ({}); double __sld = ({}); __slx > 0.0 ? log(__slx) : __sld; }})",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "f64_geometric_mean" => format!(
            "({{ double __gma = ({}); double __gmb = ({}); double __gmp = __gma * __gmb; __gmp < 0.0 ? 0.0 : sqrt(__gmp); }})",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        // Closure #452: harmonic + quadratic means.
        // harmonic(a, b) = 2ab / (a + b), or 0 if a + b == 0.
        // quadratic(a, b) = sqrt((a² + b²) / 2).  (= RMS for 2 vals)
        "f64_harmonic_mean" => format!(
            "({{ double __hma = ({}); double __hmb = ({}); double __hms = __hma + __hmb; __hms == 0.0 ? 0.0 : (2.0 * __hma * __hmb) / __hms; }})",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "f64_quadratic_mean" => format!(
            "({{ double __qma = ({}); double __qmb = ({}); sqrt((__qma * __qma + __qmb * __qmb) / 2.0); }})",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        // Closure #453: log_b(x, base) = log(x) / log(base).
        // Algebraic identity covers any positive base except 1.
        // For base in (0, 1): mathematically valid, gives
        // negative values for x > 1. For invalid inputs the
        // libm log() returns NaN/Inf — propagated rather than
        // masked.
        "f64_log_b" => format!(
            "(log(({})) / log(({})))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        // Closure #433: libm special functions.
        //   erf(x)    — Gauss error function
        //   erfc(x)   — complementary error function (= 1 - erf(x))
        //   tgamma(x) — true gamma function: gamma(n+1) = n!
        //   lgamma(x) — log(|gamma(x)|)
        "f64_erf" => format!("erf(({}))", emit_expr(&args[0])),
        "f64_erfc" => format!("erfc(({}))", emit_expr(&args[0])),
        "f64_tgamma" => format!("tgamma(({}))", emit_expr(&args[0])),
        "f64_lgamma" => format!("lgamma(({}))", emit_expr(&args[0])),
        // Closure #434: numerical-stability helpers from libm.
        //   cbrt(x)  — exact cube root, handles negative x
        //   expm1(x) — exp(x) - 1, accurate near 0
        //   log1p(x) — log(1 + x), accurate near 0
        "f64_cbrt" => format!("cbrt(({}))", emit_expr(&args[0])),
        "f64_expm1" => format!("expm1(({}))", emit_expr(&args[0])),
        "f64_log1p" => format!("log1p(({}))", emit_expr(&args[0])),
        // Closure #435: base-2 and base-10 exp. exp2 is C99
        // standard libm. exp10 is a GNU extension on some
        // platforms; implement portably as pow(10, x).
        "f64_exp2" => format!("exp2(({}))", emit_expr(&args[0])),
        "f64_exp10" => format!("pow(10.0, ({}))", emit_expr(&args[0])),
        // Closure #437: reciprocal sqrt + decimal-place rounding.
        // inv_sqrt(x) = 1 / sqrt(x)
        // round_to(x, d) = round(x * 10^d) / 10^d
        "f64_inv_sqrt" => format!("(1.0 / sqrt(({})))", emit_expr(&args[0])),
        "f64_round_to" => format!(
            "({{ double __rtx = ({}); int64_t __rtd = ({}); double __rtm = pow(10.0, (double)__rtd); round(__rtx * __rtm) / __rtm; }})",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        // Closures #481-#483: reciprocal trig (sec/csc/cot).
        "f64_sec" => format!("(1.0 / cos(({})))", emit_expr(&args[0])),
        "f64_csc" => format!("(1.0 / sin(({})))", emit_expr(&args[0])),
        "f64_cot" => format!("(cos(({0})) / sin(({0})))", emit_expr(&args[0])),
        // Closure #484: normal_pdf(x, mean, sd) — Gaussian PDF.
        //   f(x) = 1/(sd · √(2π)) · exp(-½ · ((x-mean)/sd)²)
        // For sd <= 0 returns 0 (defensive — would otherwise NaN).
        "f64_normal_pdf" => format!(
            "({{ double __npdx = ({}); double __npdm = ({}); double __npds = ({}); double __npdr; if (__npds <= 0.0) {{ __npdr = 0.0; }} else {{ double __npdz = (__npdx - __npdm) / __npds; __npdr = exp(-0.5 * __npdz * __npdz) / (__npds * 2.5066282746310002); }} __npdr; }})",
            emit_expr(&args[0]),
            emit_expr(&args[1]),
            emit_expr(&args[2])
        ),
        // Closure #485: normal_cdf(x, m, s) = ½(1 + erf((x-m)/(s·√2))).
        // sd ≤ 0 returns 0 (defensive). √2 ≈ 1.4142135623730951.
        "f64_normal_cdf" => format!(
            "({{ double __ncdx = ({}); double __ncdm = ({}); double __ncds = ({}); double __ncdr; if (__ncds <= 0.0) {{ __ncdr = 0.0; }} else {{ __ncdr = 0.5 * (1.0 + erf((__ncdx - __ncdm) / (__ncds * 1.4142135623730951))); }} __ncdr; }})",
            emit_expr(&args[0]),
            emit_expr(&args[1]),
            emit_expr(&args[2])
        ),
        // Closure #486: lerp_clamp(a, b, t) = lerp(a, b, clamp01(t)).
        "f64_lerp_clamp" => format!(
            "({{ double __lca = ({}); double __lcb = ({}); double __lct = ({}); double __lctc = __lct < 0.0 ? 0.0 : (__lct > 1.0 ? 1.0 : __lct); __lca + (__lcb - __lca) * __lctc; }})",
            emit_expr(&args[0]),
            emit_expr(&args[1]),
            emit_expr(&args[2])
        ),
        // Closure #487: atan2 returning degrees.
        "f64_atan2_deg" => format!(
            "(atan2(({}), ({})) * 57.29577951308232)",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        // Closure #490: atan returning degrees.
        "f64_atan_deg" => format!(
            "(atan(({})) * 57.29577951308232)",
            emit_expr(&args[0])
        ),
        // Closures #577/#578: asin/acos returning degrees.
        "f64_asin_deg" => format!(
            "(asin(({})) * 57.29577951308232)",
            emit_expr(&args[0])
        ),
        "f64_acos_deg" => format!(
            "(acos(({})) * 57.29577951308232)",
            emit_expr(&args[0])
        ),
        // Closures #579/#580/#581: sec/csc/cot taking degrees.
        "f64_sec_deg" => format!(
            "(1.0 / cos(({}) * 0.017453292519943295))",
            emit_expr(&args[0])
        ),
        "f64_csc_deg" => format!(
            "(1.0 / sin(({}) * 0.017453292519943295))",
            emit_expr(&args[0])
        ),
        "f64_cot_deg" => format!(
            "(cos(({}) * 0.017453292519943295) / sin(({}) * 0.017453292519943295))",
            emit_expr(&args[0]),
            emit_expr(&args[0])
        ),
        // Closure #491: RGB → grayscale via ITU-R BT.601 weights:
        //   Y = 0.299·R + 0.587·G + 0.114·B
        "f64_rgb_to_grayscale" => format!(
            "(0.299 * ({}) + 0.587 * ({}) + 0.114 * ({}))",
            emit_expr(&args[0]),
            emit_expr(&args[1]),
            emit_expr(&args[2])
        ),
        // Closures #492-#495: RGB pack / unpack as 24-bit value.
        // pack:    (r & 0xFF) << 16 | (g & 0xFF) << 8 | (b & 0xFF)
        // unpack:  byte extraction at each shift position.
        "i64_pack_rgb" => format!(
            "((({}) & 0xFF) << 16 | (({}) & 0xFF) << 8 | (({}) & 0xFF))",
            emit_expr(&args[0]),
            emit_expr(&args[1]),
            emit_expr(&args[2])
        ),
        "i64_unpack_rgb_r" => format!("((({}) >> 16) & 0xFF)", emit_expr(&args[0])),
        "i64_unpack_rgb_g" => format!("((({}) >> 8) & 0xFF)", emit_expr(&args[0])),
        "i64_unpack_rgb_b" => format!("(({}) & 0xFF)", emit_expr(&args[0])),
        // Closure #496: f64_remap(x, from_lo, from_hi, to_lo, to_hi)
        // = to_lo + (x - from_lo) * (to_hi - to_lo) / (from_hi - from_lo).
        // No clamping — extrapolation works outside [from_lo, from_hi].
        "f64_remap" => format!(
            "({{ double __rmx = ({}); double __rmfl = ({}); double __rmfh = ({}); double __rmtl = ({}); double __rmth = ({}); __rmtl + (__rmx - __rmfl) * (__rmth - __rmtl) / (__rmfh - __rmfl); }})",
            emit_expr(&args[0]),
            emit_expr(&args[1]),
            emit_expr(&args[2]),
            emit_expr(&args[3]),
            emit_expr(&args[4])
        ),
        // Closure #488: uniform random in [0, 1). Uses
        // intent_rng_next() (the same source as rand_i64).
        // Divides by 2^63 to map u63 magnitude to [0, 1).
        "f64_uniform_random" => {
            "((double)(((uint64_t)intent_rng_next()) >> 1) / 9223372036854775808.0)".to_string()
        }
        // Closure #489: inverse smoothstep — solves smoothstep(0,1,t) = y
        // analytically: t = 0.5 - sin(asin(1 - 2y) / 3).
        // y outside [0, 1] clamps to 0 or 1.
        "f64_inv_smoothstep" => format!(
            "({{ double __isy = ({}); double __isr; if (__isy <= 0.0) {{ __isr = 0.0; }} else if (__isy >= 1.0) {{ __isr = 1.0; }} else {{ __isr = 0.5 - sin(asin(1.0 - 2.0 * __isy) / 3.0); }} __isr; }})",
            emit_expr(&args[0])
        ),
        // Closure #426: byte access. Caller is responsible for
        // bounds — out-of-range reads are undefined behavior
        // (matches the safety contract of pointer arithmetic).
        "str_byte_at" => format!(
            "((int64_t)(unsigned char)(({})[({})]))",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "str_len_bytes" => format!(
            "((int64_t)strlen(({})))",
            emit_expr(&args[0])
        ),
        // Closure #436: byte-level prefix/suffix checks.
        // starts_with_byte: s[0] != 0 and s[0] == b
        // ends_with_byte: strlen(s) > 0 and s[len-1] == b
        "str_starts_with_byte" => format!(
            "({{ const char* __sb_s = ({}); int64_t __sb_b = ({}); (__sb_s[0] != 0 && (int64_t)(unsigned char)__sb_s[0] == __sb_b); }})",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        "str_ends_with_byte" => format!(
            "({{ const char* __se_s = ({}); int64_t __se_b = ({}); int64_t __se_n = (int64_t)strlen(__se_s); (__se_n > 0 && (int64_t)(unsigned char)__se_s[__se_n - 1] == __se_b); }})",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        // Closures #582-#587: str classifier predicates.
        // is_empty: s[0] == 0.
        "str_is_empty" => format!(
            "(({})[0] == 0)",
            emit_expr(&args[0])
        ),
        // is_ascii: every byte has high bit clear.
        "str_is_ascii" => format!(
            "({{ const char* __sa_s = ({}); bool __sa_r = true; for (const unsigned char* __sa_p = (const unsigned char*)__sa_s; *__sa_p; __sa_p++) if (*__sa_p > 127) {{ __sa_r = false; break; }} __sa_r; }})",
            emit_expr(&args[0])
        ),
        // is_digit_only: non-empty and every byte in '0'..='9'.
        "str_is_digit_only" => format!(
            "({{ const char* __sd_s = ({}); bool __sd_r; if (!__sd_s[0]) __sd_r = false; else {{ __sd_r = true; for (const unsigned char* __sd_p = (const unsigned char*)__sd_s; *__sd_p; __sd_p++) if (*__sd_p < '0' || *__sd_p > '9') {{ __sd_r = false; break; }} }} __sd_r; }})",
            emit_expr(&args[0])
        ),
        // is_alpha_only: non-empty and every byte in 'A'..='Z' or 'a'..='z'.
        "str_is_alpha_only" => format!(
            "({{ const char* __sap_s = ({}); bool __sap_r; if (!__sap_s[0]) __sap_r = false; else {{ __sap_r = true; for (const unsigned char* __sap_p = (const unsigned char*)__sap_s; *__sap_p; __sap_p++) {{ unsigned char __sap_c = *__sap_p; bool __sap_ok = (__sap_c >= 'A' && __sap_c <= 'Z') || (__sap_c >= 'a' && __sap_c <= 'z'); if (!__sap_ok) {{ __sap_r = false; break; }} }} }} __sap_r; }})",
            emit_expr(&args[0])
        ),
        // is_alphanumeric_only: non-empty and every byte is ascii alphanumeric.
        "str_is_alphanumeric_only" => format!(
            "({{ const char* __san_s = ({}); bool __san_r; if (!__san_s[0]) __san_r = false; else {{ __san_r = true; for (const unsigned char* __san_p = (const unsigned char*)__san_s; *__san_p; __san_p++) {{ unsigned char __san_c = *__san_p; bool __san_ok = (__san_c >= '0' && __san_c <= '9') || (__san_c >= 'A' && __san_c <= 'Z') || (__san_c >= 'a' && __san_c <= 'z'); if (!__san_ok) {{ __san_r = false; break; }} }} }} __san_r; }})",
            emit_expr(&args[0])
        ),
        // is_whitespace_only: non-empty and every byte is in " \t\n\r\v\f".
        "str_is_whitespace_only" => format!(
            "({{ const char* __sw_s = ({}); bool __sw_r; if (!__sw_s[0]) __sw_r = false; else {{ __sw_r = true; for (const unsigned char* __sw_p = (const unsigned char*)__sw_s; *__sw_p; __sw_p++) {{ unsigned char __sw_c = *__sw_p; bool __sw_ok = (__sw_c == ' ' || __sw_c == '\\t' || __sw_c == '\\n' || __sw_c == '\\r' || __sw_c == 11 || __sw_c == 12); if (!__sw_ok) {{ __sw_r = false; break; }} }} }} __sw_r; }})",
            emit_expr(&args[0])
        ),
        // Closure #454: count ASCII decimal-digit bytes in s.
        "str_count_ascii_digits" => format!(
            "({{ const char* __cdg_s = ({}); int64_t __cdg_n = 0; for (const char* __cdg_p = __cdg_s; *__cdg_p != 0; __cdg_p++) {{ unsigned char __cdg_c = (unsigned char)*__cdg_p; if (__cdg_c >= 48 && __cdg_c <= 57) __cdg_n += 1; }} __cdg_n; }})",
            emit_expr(&args[0])
        ),
        // Closure #455: count ASCII alphabetic bytes in s
        // (A-Z = 65..90, a-z = 97..122).
        "str_count_ascii_alpha" => format!(
            "({{ const char* __cal_s = ({}); int64_t __cal_n = 0; for (const char* __cal_p = __cal_s; *__cal_p != 0; __cal_p++) {{ unsigned char __cal_c = (unsigned char)*__cal_p; if ((__cal_c >= 65 && __cal_c <= 90) || (__cal_c >= 97 && __cal_c <= 122)) __cal_n += 1; }} __cal_n; }})",
            emit_expr(&args[0])
        ),
        // Closure #456: count alphanumeric bytes
        // (digits + A-Z + a-z).
        "str_count_ascii_alphanumeric" => format!(
            "({{ const char* __can_s = ({}); int64_t __can_n = 0; for (const char* __can_p = __can_s; *__can_p != 0; __can_p++) {{ unsigned char __can_c = (unsigned char)*__can_p; if ((__can_c >= 48 && __can_c <= 57) || (__can_c >= 65 && __can_c <= 90) || (__can_c >= 97 && __can_c <= 122)) __can_n += 1; }} __can_n; }})",
            emit_expr(&args[0])
        ),
        // Closure #457: count ASCII whitespace bytes
        // (space=32, tab=9, LF=10, VT=11, FF=12, CR=13).
        // The 9..13 range covers tab/LF/VT/FF/CR contiguously.
        "str_count_ascii_whitespace" => format!(
            "({{ const char* __cws_s = ({}); int64_t __cws_n = 0; for (const char* __cws_p = __cws_s; *__cws_p != 0; __cws_p++) {{ unsigned char __cws_c = (unsigned char)*__cws_p; if (__cws_c == 32 || (__cws_c >= 9 && __cws_c <= 13)) __cws_n += 1; }} __cws_n; }})",
            emit_expr(&args[0])
        ),
        // Closure #462: count uppercase A-Z bytes (65..90).
        "str_count_ascii_upper" => format!(
            "({{ const char* __cup_s = ({}); int64_t __cup_n = 0; for (const char* __cup_p = __cup_s; *__cup_p != 0; __cup_p++) {{ unsigned char __cup_c = (unsigned char)*__cup_p; if (__cup_c >= 65 && __cup_c <= 90) __cup_n += 1; }} __cup_n; }})",
            emit_expr(&args[0])
        ),
        // Closure #463: count lowercase a-z bytes (97..122).
        "str_count_ascii_lower" => format!(
            "({{ const char* __clo_s = ({}); int64_t __clo_n = 0; for (const char* __clo_p = __clo_s; *__clo_p != 0; __clo_p++) {{ unsigned char __clo_c = (unsigned char)*__clo_p; if (__clo_c >= 97 && __clo_c <= 122) __clo_n += 1; }} __clo_n; }})",
            emit_expr(&args[0])
        ),
        // Closure #464: count ASCII punctuation bytes.
        // Punctuation = printable (33..126) AND NOT alphanumeric.
        // Equivalent four-range form: 33..47, 58..64, 91..96, 123..126.
        "str_count_ascii_punct" => format!(
            "({{ const char* __cpu_s = ({}); int64_t __cpu_n = 0; for (const char* __cpu_p = __cpu_s; *__cpu_p != 0; __cpu_p++) {{ unsigned char __cpu_c = (unsigned char)*__cpu_p; if ((__cpu_c >= 33 && __cpu_c <= 47) || (__cpu_c >= 58 && __cpu_c <= 64) || (__cpu_c >= 91 && __cpu_c <= 96) || (__cpu_c >= 123 && __cpu_c <= 126)) __cpu_n += 1; }} __cpu_n; }})",
            emit_expr(&args[0])
        ),
        // Closure #465: count ASCII control bytes (1..31 OR 127).
        // NUL (0) can't appear in C strings, so it's omitted from
        // the predicate.
        "str_count_ascii_control" => format!(
            "({{ const char* __cct_s = ({}); int64_t __cct_n = 0; for (const char* __cct_p = __cct_s; *__cct_p != 0; __cct_p++) {{ unsigned char __cct_c = (unsigned char)*__cct_p; if ((__cct_c >= 1 && __cct_c <= 31) || __cct_c == 127) __cct_n += 1; }} __cct_n; }})",
            emit_expr(&args[0])
        ),
        // Closure #466: first byte of s as Option<i64>.
        "str_first_byte" => format!(
            "({{ const char* __fb_s = ({}); Enum_Option__i64 __fb_r; if (__fb_s[0] == 0) {{ __fb_r.tag = 1; __fb_r.payload = 0; }} else {{ __fb_r.tag = 0; __fb_r.payload = (int64_t)(unsigned char)__fb_s[0]; }} __fb_r; }})",
            emit_expr(&args[0])
        ),
        // Closure #467: last byte of s as Option<i64>.
        "str_last_byte" => format!(
            "({{ const char* __lb_s = ({}); size_t __lb_n = strlen(__lb_s); Enum_Option__i64 __lb_r; if (__lb_n == 0) {{ __lb_r.tag = 1; __lb_r.payload = 0; }} else {{ __lb_r.tag = 0; __lb_r.payload = (int64_t)(unsigned char)__lb_s[__lb_n - 1]; }} __lb_r; }})",
            emit_expr(&args[0])
        ),
        // Closure #441: count occurrences of byte b in s.
        // Walks the string until the null terminator.
        "str_byte_count" => format!(
            "({{ const char* __bc_s = ({}); int64_t __bc_b = ({}); int64_t __bc_n = 0; for (const char* __bc_p = __bc_s; *__bc_p != 0; __bc_p++) {{ if ((int64_t)(unsigned char)*__bc_p == __bc_b) __bc_n += 1; }} __bc_n; }})",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        // Closure #442: first index of byte b in s, as Option<i64>.
        // strchr returns NULL if not found; otherwise pointer to
        // the byte. Note: strchr(s, 0) returns pointer to the
        // null terminator (i.e., index strlen(s)) — users
        // searching for the terminator should handle this.
        "str_index_of_byte" => format!(
            "({{ const char* __sib_s = ({}); int64_t __sib_b = ({}); const char* __sib_m = strchr(__sib_s, (int)__sib_b); Enum_Option__i64 __sib_r; if (__sib_m == NULL) {{ __sib_r.tag = 1; __sib_r.payload = 0; }} else {{ __sib_r.tag = 0; __sib_r.payload = (int64_t)(__sib_m - __sib_s); }} __sib_r; }})",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        // Closure #443: last occurrence of byte b in s, as
        // Option<i64>. strrchr returns NULL or pointer to last
        // matching byte.
        "str_last_index_of_byte" => format!(
            "({{ const char* __slib_s = ({}); int64_t __slib_b = ({}); const char* __slib_m = strrchr(__slib_s, (int)__slib_b); Enum_Option__i64 __slib_r; if (__slib_m == NULL) {{ __slib_r.tag = 1; __slib_r.payload = 0; }} else {{ __slib_r.tag = 0; __slib_r.payload = (int64_t)(__slib_m - __slib_s); }} __slib_r; }})",
            emit_expr(&args[0]),
            emit_expr(&args[1])
        ),
        // Closure #406: linear interpolation + clamp to [0, 1].
        // lerp(a, b, t) = a + (b - a) * t. Standard form;
        // overflow-safe within representable range.
        "f64_lerp" => {
            format!(
                "({{ double __la = ({}); double __lb = ({}); double __lt = ({}); __la + (__lb - __la) * __lt; }})",
                emit_expr(&args[0]),
                emit_expr(&args[1]),
                emit_expr(&args[2])
            )
        }
        // clamp01(x) = max(0, min(1, x))
        "f64_clamp01" => {
            format!(
                "({{ double __cx = ({}); __cx < 0.0 ? 0.0 : (__cx > 1.0 ? 1.0 : __cx); }})",
                emit_expr(&args[0])
            )
        }
        // Closure #372: float-to-int rounding.
        // f64_round: round half away from zero (libc `llround`).
        // f64_trunc_to_i64: C truncating cast — chops the fractional
        // part toward zero.
        "f64_round" => format!("((int64_t)llround(({})))", emit_expr(&args[0])),
        "f64_trunc_to_i64" => format!("((int64_t)({}))", emit_expr(&args[0])),
        // Closure #380: integer math. Inline implementations to
        // avoid linking against libm (which only provides
        // double-typed helpers). GCD uses Euclidean iteration;
        // LCM = abs(a*b)/gcd handles overflow loosely (we trust
        // i64 to hold the result for any reasonable input). POW
        // uses fast-exponentiation by squaring; negative exp
        // returns 0 by convention.
        "i64_gcd" => {
            format!(
                "({{ int64_t __ga = ({}); int64_t __gb = ({}); if (__ga < 0) __ga = -__ga; if (__gb < 0) __gb = -__gb; while (__gb != 0) {{ int64_t __t = __ga % __gb; __ga = __gb; __gb = __t; }} __ga; }})",
                emit_expr(&args[0]),
                emit_expr(&args[1])
            )
        }
        "i64_lcm" => {
            // Use a result var to keep the GNU statement-expression's
            // tail an expression. The if/else above writes __lr;
            // the trailing `__lr;` is the statement-expr's value.
            format!(
                "({{ int64_t __la = ({}); int64_t __lb = ({}); int64_t __lr; if (__la == 0 || __lb == 0) {{ __lr = 0; }} else {{ int64_t __aa = __la < 0 ? -__la : __la; int64_t __bb = __lb < 0 ? -__lb : __lb; int64_t __g = __aa; int64_t __h = __bb; while (__h != 0) {{ int64_t __t2 = __g % __h; __g = __h; __h = __t2; }} __lr = (__aa / __g) * __bb; }} __lr; }})",
                emit_expr(&args[0]),
                emit_expr(&args[1])
            )
        }
        "i64_pow" => {
            format!(
                "({{ int64_t __pb = ({}); int64_t __pe = ({}); int64_t __pr = 1; if (__pe < 0) {{ __pr = 0; }} else {{ while (__pe > 0) {{ if (__pe & 1) __pr = __pr * __pb; __pb = __pb * __pb; __pe = __pe >> 1; }} }} __pr; }})",
                emit_expr(&args[0]),
                emit_expr(&args[1])
            )
        }
        // Closure #401: bit-manipulation primitives.
        // GCC / Clang __builtin_popcountll / clzll / ctzll all
        // take `unsigned long long`. Returns are wrapped as
        // (int64_t)... For zero input, clzll/ctzll are
        // undefined per the ABI, so we special-case 0 → 64.
        "i64_count_set_bits" => {
            format!(
                "((int64_t)__builtin_popcountll((unsigned long long)({})))",
                emit_expr(&args[0])
            )
        }
        "i64_leading_zeros" => {
            format!(
                "({{ unsigned long long __lzv = (unsigned long long)({}); __lzv == 0 ? (int64_t)64 : (int64_t)__builtin_clzll(__lzv); }})",
                emit_expr(&args[0])
            )
        }
        "i64_trailing_zeros" => {
            format!(
                "({{ unsigned long long __tzv = (unsigned long long)({}); __tzv == 0 ? (int64_t)64 : (int64_t)__builtin_ctzll(__tzv); }})",
                emit_expr(&args[0])
            )
        }
        // Closure #402: byte-swap + rotate.
        "i64_bswap" => {
            format!(
                "((int64_t)__builtin_bswap64((unsigned long long)({})))",
                emit_expr(&args[0])
            )
        }
        "i64_rotate_left" => {
            // Mask shift count to [0..63] to avoid UB when n is
            // out of range. ((x << n) | (x >> (64 - n))) is the
            // canonical rotate idiom; the masking + a conditional
            // avoids the (64 - 0) shift-by-64 UB on n = 0.
            format!(
                "({{ unsigned long long __rlv = (unsigned long long)({}); int64_t __rln = ({}) & 63; __rln == 0 ? (int64_t)__rlv : (int64_t)((__rlv << __rln) | (__rlv >> (64 - __rln))); }})",
                emit_expr(&args[0]),
                emit_expr(&args[1])
            )
        }
        "i64_rotate_right" => {
            format!(
                "({{ unsigned long long __rrv = (unsigned long long)({}); int64_t __rrn = ({}) & 63; __rrn == 0 ? (int64_t)__rrv : (int64_t)((__rrv >> __rrn) | (__rrv << (64 - __rrn))); }})",
                emit_expr(&args[0]),
                emit_expr(&args[1])
            )
        }
        // Closure #403: IEEE-754 bit-level reinterpretation.
        // memcpy is the strict-aliasing-safe spelling — gcc/clang
        // both optimize it to a no-op move at -O1+. Avoids the
        // union-pun and the *(uint64_t*)&x form.
        "f64_to_bits" => {
            format!(
                "({{ double __fb_v = ({}); int64_t __fb_r; memcpy(&__fb_r, &__fb_v, sizeof(__fb_r)); __fb_r; }})",
                emit_expr(&args[0])
            )
        }
        "f64_from_bits" => {
            format!(
                "({{ int64_t __ff_b = ({}); double __ff_r; memcpy(&__ff_r, &__ff_b, sizeof(__ff_r)); __ff_r; }})",
                emit_expr(&args[0])
            )
        }
        // Closure #393: i64_abs_diff(a, b) — |a - b| in signed
        // arithmetic. Done via select on a < b to avoid overflow
        // on borderline values like INT64_MIN / INT64_MAX.
        "i64_abs_diff" => {
            format!(
                "({{ int64_t __ad_a = ({}); int64_t __ad_b = ({}); (__ad_a < __ad_b) ? (__ad_b - __ad_a) : (__ad_a - __ad_b); }})",
                emit_expr(&args[0]),
                emit_expr(&args[1])
            )
        }
        // i64_signum(x) — returns -1 / 0 / +1.
        "i64_signum" => {
            format!(
                "({{ int64_t __sn = ({}); (int64_t)((__sn > 0) - (__sn < 0)); }})",
                emit_expr(&args[0])
            )
        }
        // f64_signum(x) — returns -1.0 / 0.0 / +1.0. NaN stays NaN.
        "f64_signum" => {
            format!(
                "({{ double __fs = ({}); isnan(__fs) ? __fs : ((__fs > 0.0) - (__fs < 0.0)); }})",
                emit_expr(&args[0])
            )
        }
        // Closure #400: ASCII byte-class predicates. Inline byte
        // range checks — independent of locale (libc's ctype is
        // locale-sensitive).
        "is_ascii_digit" => {
            format!(
                "({{ int64_t __ad = ({}); (__ad >= 48 && __ad <= 57); }})",
                emit_expr(&args[0])
            )
        }
        "is_ascii_alpha" => {
            format!(
                "({{ int64_t __aa = ({}); ((__aa >= 65 && __aa <= 90) || (__aa >= 97 && __aa <= 122)); }})",
                emit_expr(&args[0])
            )
        }
        "is_ascii_alphanumeric" => {
            format!(
                "({{ int64_t __an = ({}); ((__an >= 48 && __an <= 57) || (__an >= 65 && __an <= 90) || (__an >= 97 && __an <= 122)); }})",
                emit_expr(&args[0])
            )
        }
        "is_ascii_whitespace" => {
            // Matches ' ', '\t' (9), '\n' (10), '\v' (11), '\f' (12),
            // '\r' (13). The contiguous 9..=13 range + ' ' (32) match
            // the same set as C's isspace() under the C locale.
            format!(
                "({{ int64_t __aw = ({}); ((__aw >= 9 && __aw <= 13) || __aw == 32); }})",
                emit_expr(&args[0])
            )
        }
        "abs" => {
            // Overload: i64 → llabs / (x<0?-x:x); f64 → fabs.
            // Other signed ints get cast to i64.
            match &args[0].ty {
                Type::F64 | Type::F32 => format!("fabs(({}))", emit_expr(&args[0])),
                _ => format!("llabs(({}))", emit_expr(&args[0])),
            }
        }
        "parse_float" => {
            let opt_name = match result_ty {
                Type::Enum(name) => name.clone(),
                _ => unreachable!("parse_float must return Type::Enum(Option__f64)"),
            };
            let opt_c = enum_c_name(&opt_name);
            format!(
                "({{ const char* __pf_s = ({s}); char* __pf_end = (char*)0; double __pf_v = strtod(__pf_s, &__pf_end); {opt} __pf_r; if (__pf_end != __pf_s && *__pf_end == 0 && *__pf_s != 0) {{ __pf_r.tag = 0; __pf_r.payload = __pf_v; }} else {{ __pf_r.tag = 1; __pf_r.payload = 0; }} __pf_r; }})",
                s = emit_expr(&args[0]),
                opt = opt_c,
            )
        }
        "binary_search" => {
            // Standard binary search; assumes xs is sorted
            // ascending. Returns Option<i64>(index) on match.
            let opt_name = match result_ty {
                Type::Enum(name) => name.clone(),
                _ => unreachable!("binary_search() must return Type::Enum(Option__i64)"),
            };
            let opt_c = enum_c_name(&opt_name);
            match args[0].ty.deref() {
                Type::Array { length, .. } => format!(
                    "intent_array_int64_t__binary_search((const int64_t*)({xs}), (uint64_t){len}LL, ({n}))",
                    xs = emit_expr(&args[0]),
                    len = length,
                    n = emit_expr(&args[1]),
                ),
                _ => format!(
                    "({{ const intent_vec_int64_t* __bv = ({xs}); int64_t __bn = ({n}); {opt} __br; int64_t __blo = 0; int64_t __bhi = (int64_t)__bv->len - 1; bool __bf = false; int64_t __bm = 0; while (__blo <= __bhi) {{ __bm = __blo + (__bhi - __blo) / 2; int64_t __bv0 = __bv->data[__bm]; if (__bv0 == __bn) {{ __bf = true; break; }} else if (__bv0 < __bn) {{ __blo = __bm + 1; }} else {{ __bhi = __bm - 1; }} }} if (__bf) {{ __br.tag = 0; __br.payload = __bm; }} else {{ __br.tag = 1; __br.payload = 0; }} __br; }})",
                    xs = emit_expr(&args[0]),
                    n = emit_expr(&args[1]),
                    opt = opt_c,
                ),
            }
        }
        "set" => {
            let element = match result_ty {
                Type::Vec(element) => element,
                _ => unreachable!("set() must return Vec<_>"),
            };
            format!(
                "{}({}, (uint64_t)({}), {})",
                vec_helper(element, "set"),
                emit_expr(&args[0]),
                emit_expr(&args[1]),
                emit_expr(&args[2])
            )
        }
        "clone" => {
            let element = match result_ty {
                Type::Vec(element) => element,
                _ => unreachable!("clone() must return Vec<_>"),
            };
            format!(
                "{}({})",
                vec_helper(element, "clone"),
                emit_expr(&args[0])
            )
        }
        "clone_at" => {
            // `clone_at(xs, i)`: return a deep copy of slot i.
            // For Copy elements the raw slot value is itself
            // a fresh independent copy (memcpy semantics).
            // For Vec<U> elements we call the inner's __clone
            // so the result owns its own buffer — refines #7
            // phase 2d. Source operand may be `Vec<T>` or
            // `&Vec<T>` / `&mut Vec<T>`; collection_expr
            // figures out the actual storage spelling so the
            // emitted access (`v.data[i]` vs `v->data[i]`)
            // is well-formed.
            let xs_arg = &args[0];
            let underlying = xs_arg.ty.deref();
            // Closure #291: `clone_at(ref [T; N], i)` accepts
            // arrays alongside Vec. Arrays index directly as
            // `xs[i]` (C array decay); Vec uses `.data[i]`.
            let element_ty = match underlying {
                Type::Vec(element) => &**element,
                Type::Array { element, .. } => &**element,
                other => {
                    unreachable!("clone_at requires Vec or Array, got {:?}", other)
                }
            };
            let is_array = matches!(underlying, Type::Array { .. });
            let xs_str = emit_expr(xs_arg);
            let access_via_ref = matches!(
                &xs_arg.ty,
                Type::Ref(_) | Type::RefMut(_)
            );
            // Wrap xs_str in parens so `&xs->data[i]`
            // parses as `(&xs)->data[i]` — `->` binds
            // tighter than unary `&` so naked
            // concatenation breaks.
            let slot = if is_array {
                // C array indexing: arrays decay to T* so
                // both `xs[i]` (value) and `xs[i]` (ref —
                // ref-of-array passes the decayed pointer)
                // index the same way. No `*` indirection
                // needed.
                format!("({})[{}]", xs_str, emit_expr(&args[1]))
            } else if access_via_ref {
                format!("({})->data[{}]", xs_str, emit_expr(&args[1]))
            } else {
                format!("({}).data[{}]", xs_str, emit_expr(&args[1]))
            };
            // Element-aware deep-clone: recurse through
            // `c_element_deep_clone` so a `Vec<Vec<U>>` slot
            // routes through the inner Vec's __clone helper.
            // For Copy elements the helper returns the slot
            // unchanged (memcpy semantics).
            c_element_deep_clone(&slot, element_ty)
        }
        _ => {
            let rendered_args = args.iter().map(emit_expr).collect::<Vec<_>>().join(", ");
            // Closure #269: extern "C" fns emit a bare C-ABI
            // call (no `fn_` prefix). The C_EXTERN_FN_REGISTRY
            // gets populated at backend entry from the
            // program's extern fn list.
            let is_extern = C_EXTERN_FN_REGISTRY
                .with(|r| r.borrow().contains(name));
            let symbol = if is_extern {
                name.to_string()
            } else {
                function_name(name)
            };
            format!("{}({})", symbol, rendered_args)
        }
    }
}

fn emit_index(array: &TypedExpr, index: &TypedExpr, checked: bool) -> String {
    let index_str = emit_expr(index);
    let array_str = emit_expr(array);
    // For Ref/RefMut types, C array decay handles arrays automatically; Vec needs explicit (*ptr).
    let (underlying, is_ref) = match &array.ty {
        Type::Ref(inner) | Type::RefMut(inner) => (&**inner, true),
        other => (other, false),
    };
    match underlying {
        Type::Array { length, .. } => {
            if checked {
                format!(
                    "({}[intent_check_bounds((uint64_t)({}), {})])",
                    array_str, index_str, length
                )
            } else {
                format!("({}[{}])", array_str, index_str)
            }
        }
        Type::Vec(element) => {
            // Fresh-Vec operand: bind to a brace-scoped tmp,
            // read .data[i], then free the buffer via
            // `intent_vec_<T>__free`. Without this the heap
            // leaks. Var / FieldAccess Vec operands keep the
            // simple form — binding owns the buffer. Closure
            // #142.
            if !is_ref && crate::ir::is_fresh_non_copy(array) {
                let struct_name = vec_c_struct(element);
                let free_helper = vec_helper(element, "free");
                let elem_storage = c_element_storage(element);
                if checked {
                    return format!(
                        "(({{ {sn} _intent_idx_tmp = ({arr}); {es} _intent_idx_r = _intent_idx_tmp.data[intent_check_bounds((uint64_t)({idx}), _intent_idx_tmp.len)]; {fh}(_intent_idx_tmp); _intent_idx_r; }}))",
                        sn = struct_name,
                        arr = array_str,
                        es = elem_storage,
                        idx = index_str,
                        fh = free_helper
                    );
                } else {
                    return format!(
                        "(({{ {sn} _intent_idx_tmp = ({arr}); {es} _intent_idx_r = _intent_idx_tmp.data[(uint64_t)({idx})]; {fh}(_intent_idx_tmp); _intent_idx_r; }}))",
                        sn = struct_name,
                        arr = array_str,
                        es = elem_storage,
                        idx = index_str,
                        fh = free_helper
                    );
                }
            }
            let prefix = if is_ref {
                format!("(*{})", array_str)
            } else {
                array_str.clone()
            };
            if checked {
                format!(
                    "({}.data[intent_check_bounds((uint64_t)({}), {}.len)])",
                    prefix, index_str, prefix
                )
            } else {
                format!("({}.data[(uint64_t)({})])", prefix, index_str)
            }
        }
        _ => format!("({}[{}])", array_str, index_str),
    }
}

fn emit_len(array: &TypedExpr, static_length: u64) -> String {
    let (underlying, is_ref) = match &array.ty {
        Type::Ref(inner) | Type::RefMut(inner) => (&**inner, true),
        other => (other, false),
    };
    match underlying {
        Type::Array { .. } => format!("((uint64_t){})", static_length),
        Type::Vec(element) => {
            // Fresh-Vec operand: bind to a brace-scoped tmp,
            // read .len, then free the buffer via the
            // matching `intent_vec_<T>__free` helper. Var /
            // FieldAccess Vec operands keep the simple
            // `(v.len)` form — binding owns the buffer.
            // Closure #141.
            let array_str = emit_expr(array);
            if !is_ref && crate::ir::is_fresh_non_copy(array) {
                let struct_name = vec_c_struct(element);
                let free_helper = vec_helper(element, "free");
                return format!(
                    "((uint64_t)({{ {sn} _intent_len_tmp = ({arr}); uint64_t _intent_len_r = _intent_len_tmp.len; {fh}(_intent_len_tmp); _intent_len_r; }}))",
                    sn = struct_name,
                    arr = array_str,
                    fh = free_helper
                );
            }
            if is_ref {
                format!("((*{}).len)", array_str)
            } else {
                format!("({}.len)", array_str)
            }
        }
        Type::Str | Type::OwnedStr => {
            // Fresh OwnedStr operand: free the heap after
            // strlen via a GCC statement-expression. Var /
            // FieldAccess / Str operands stay non-consuming.
            // Closure #140.
            //
            // Closure #262: when the operand is a borrow (Ref
            // / RefMut), the C expression has type `char**` /
            // `const char**` — strlen wants the inner pointer.
            // Dereference once with `(*expr)`. Without this,
            // `len(ref s)` for `s: OwnedStr` returned
            // `strlen(&s)` which read junk from the pointer's
            // own bytes (≈ 6 on x86-64 little-endian).
            let arg_str = emit_expr(array);
            let arg_expr = if is_ref {
                format!("(*{})", arg_str)
            } else {
                arg_str
            };
            if crate::ir::is_fresh_owned_str(array) {
                format!(
                    "((uint64_t)({{ char* _intent_len_tmp = ({}); uint64_t _intent_len_r = (uint64_t)strlen(_intent_len_tmp); free((void*)_intent_len_tmp); _intent_len_r; }}))",
                    arg_expr
                )
            } else {
                format!("((uint64_t)strlen({}))", arg_expr)
            }
        }
        _ => format!("((uint64_t){})", static_length),
    }
}

fn emit_binary(
    op: BinaryOp,
    left: &TypedExpr,
    right: &TypedExpr,
    checked: bool,
    _result_type: &Type,
) -> String {
    // Str/OwnedStr concat: `a + b` → an inline call to a runtime
    // helper that mallocs a fresh buffer of size strlen(a) +
    // strlen(b) + 1, copies both, and returns the new pointer.
    // OwnedStr operands are consumed (their backing buffer is
    // freed by the helper before it returns the new buffer);
    // the checker has already marked the underlying bindings
    // as moved so they can't be used afterward. The owned
    // flag uses the same conservative whitelist as the rest
    // of the fresh-OwnedStr handlers (Call / Binary / Block
    // / IfExpr / Match) — Var / FieldAccess operands share
    // their buffer with a live binding and would double-free
    // at the binding's scope-exit Drop. Closure #144 widened
    // the previous `matches!(ty, OwnedStr)` check that
    // double-freed `t.name + "x"` (FieldAccess + Str).
    if matches!(op, BinaryOp::Add)
        && matches!(left.ty, Type::Str | Type::OwnedStr)
        && matches!(right.ty, Type::Str | Type::OwnedStr)
    {
        let lhs_owned = crate::ir::owned_str_consumed_at_concat(left);
        let rhs_owned = crate::ir::owned_str_consumed_at_concat(right);
        return format!(
            "intent_str_concat({}, {}, {}, {})",
            emit_expr(left),
            if lhs_owned { 1 } else { 0 },
            emit_expr(right),
            if rhs_owned { 1 } else { 0 },
        );
    }
    // Str/OwnedStr comparisons lower to strcmp(a, b) <op> 0 instead
    // of pointer comparison. Either type is accepted in either
    // position — strcmp only reads, so OwnedStr is auto-borrowed.
    if matches!(
        op,
        BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge
    ) && matches!(left.ty, Type::Str | Type::OwnedStr)
      && matches!(right.ty, Type::Str | Type::OwnedStr)
    {
        let cmp = match op {
            BinaryOp::Eq => "==",
            BinaryOp::Ne => "!=",
            BinaryOp::Lt => "<",
            BinaryOp::Le => "<=",
            BinaryOp::Gt => ">",
            BinaryOp::Ge => ">=",
            _ => unreachable!(),
        };
        // Fresh-OwnedStr operands need a free after strcmp;
        // bind to brace-scoped tmps so the values stay alive
        // for the compare and get released after. Closure #140.
        let l_fresh = crate::ir::is_fresh_owned_str(left);
        let r_fresh = crate::ir::is_fresh_owned_str(right);
        if l_fresh || r_fresh {
            let mut body = String::from("({ ");
            body.push_str(&format!("const char* _intent_cmp_l = ({}); ", emit_expr(left)));
            body.push_str(&format!("const char* _intent_cmp_r = ({}); ", emit_expr(right)));
            body.push_str(&format!(
                "bool _intent_cmp_r_b = (strcmp(_intent_cmp_l, _intent_cmp_r) {} 0); ",
                cmp
            ));
            if l_fresh {
                body.push_str("free((void*)_intent_cmp_l); ");
            }
            if r_fresh {
                body.push_str("free((void*)_intent_cmp_r); ");
            }
            body.push_str("_intent_cmp_r_b; })");
            return body;
        }
        return format!("(strcmp({}, {}) {} 0)", emit_expr(left), emit_expr(right), cmp);
    }

    let right_expr = match op {
        BinaryOp::Div | BinaryOp::Rem if checked => {
            format!("{}({})", divisor_helper(&right.ty), emit_expr(right))
        }
        BinaryOp::Shl | BinaryOp::Shr if checked => {
            let bits = left.ty.bits().unwrap_or(64);
            format!("{}({}, {})", shift_helper(&right.ty), emit_expr(right), bits)
        }
        _ => emit_expr(right),
    };

    format!("({} {} {})", emit_expr(left), op.display_symbol(), right_expr)
}

fn emit_float_literal(value: f64, ty: &Type) -> String {
    if *ty == Type::F32 {
        format!("{:?}f", value as f32)
    } else {
        format!("{:?}", value)
    }
}

/// C-specific spelling for a leaf type. Used wherever the backend
/// emits a type name into the generated C source. Lives in this
/// module (not in `ast::Type`) so the AST stays backend-agnostic
/// for the upcoming LLVM backend migration.
pub(crate) fn c_leaf_type(ty: &Type) -> &'static str {
    match ty {
        Type::I8 => "int8_t",
        Type::I16 => "int16_t",
        Type::I32 => "int32_t",
        Type::I64 => "int64_t",
        Type::U8 => "uint8_t",
        Type::U16 => "uint16_t",
        Type::U32 => "uint32_t",
        Type::U64 => "uint64_t",
        Type::F32 => "float",
        Type::F64 => "double",
        Type::Bool => "bool",
        Type::Str => "const char*",
        Type::OwnedStr => "char*",
        Type::Array { .. } => "/* array */",
        Type::Vec(_) => "/* vec */",
        Type::Ref(_) => "/* ref */",
        Type::RefMut(_) => "/* ref mut */",
        // `Task` lowers to a small handle struct: the
        // pthread_t plus the heap-allocated ctx pointer so
        // join can free the ctx after pthread_join returns.
        // The typedef sits in the runtime preamble alongside
        // the channel / mutex helpers.
        Type::Task => "intent_task_handle",
        // `Atomic<T>` is parametric over T (integer widths +
        // bool). c_leaf_type cannot synthesize a `String`, so
        // callers that need the storage spelling for a specific
        // atomic call into `c_atomic_storage` instead. The
        // arm below is reachable only from places that look at
        // `Type::Atomic` generically without spelling it
        // (e.g. divisor-helper / shift-helper unreachable
        // arms); returning the i64 form keeps any escapee
        // valid C while a stricter audit would replace it
        // with `unreachable!`.
        Type::Atomic(_) => "_Atomic int64_t",
        // `Channel<T, N>` is parametric over both element
        // width and capacity. c_leaf_type can't synthesize a
        // String for each (T, N) pair; callers that need the
        // storage spelling use `c_channel_storage(element, N)`
        // directly. Hitting this arm means a caller forgot to
        // special-case Channel — fall back to the i64/16 form
        // so output stays valid C rather than panicking, but a
        // stricter audit would `unreachable!`.
        Type::Channel(_, _) => "intent_channel_int64_t_16",
        // `Mutex<T>` storage is a 2-field struct: payload + a
        // CAS-based spin lock. v1: i64 payload only.
        Type::Mutex(_) => "intent_mutex_i64",
        // `Guard<T>` is a thin handle holding a pointer back to
        // its mutex. The scope-exit drop unlocks. v1: i64
        // payload.
        Type::Guard(_) => "intent_guard_i64",
        // `Condvar` is a signaling primitive — a heap-allocated
        // futex / WaitOnAddress seq counter under the hood. The
        // affine handle is just a pointer to that storage.
        Type::Condvar => "intent_condvar",
        // `Deque<T>` is a ring buffer with heap data. v1 i64
        // only; the type spelling is fixed at the i64 form
        // since c_leaf_type can't synthesize per-T strings.
        Type::Deque(_) => "intent_deque_i64",
        // `HashSet<T>` — open-addressing hash set. v1 i64 only.
        Type::HashSet(_) => "intent_hashset_i64",
        // `HashMap<K, V>` — open-addressing. v1 (i64, i64) only.
        Type::HashMap(_, _) => "intent_hashmap_i64_i64",
        // `BTreeSet<T>` — sorted-Vec backed. v1 i64 only.
        Type::BTreeSet(_) => "intent_btreeset_i64",
        // `BTreeMap<K, V>` — sorted-Vec backed parallel arrays. v1 (i64, i64) only.
        Type::BTreeMap(_, _) => "intent_btreemap_i64_i64",
        // `UnionFind` — Level 4 #1 arena-based disjoint-set.
        Type::UnionFind => "intent_union_find",
        // `BinaryHeap<T>` — Level 4 #2 dedicated min-heap. v1 i64.
        Type::BinaryHeap(_) => "intent_binary_heap_i64",
        // `BloomFilter` — Level 4 #6 probabilistic membership tester.
        Type::BloomFilter => "intent_bloom_filter",
        // `Bst<T>` — Level 4 #3 binary search tree on node arena. v1 i64.
        Type::Bst(_) => "intent_bst_i64",
        // `Graph` — Level 4 #5 weighted directed graph. v1 i64 weights.
        Type::Graph => "intent_graph",
        // `Trie` — Level 4 #4 prefix tree on node arena. v1 a-z alphabet.
        Type::Trie => "intent_trie",
        // `SkipList` — Level 4 #7 probabilistic ordered set. v1 i64.
        Type::SkipList => "intent_skiplist_i64",
        // `fn(T1, T2) -> R` has no fixed leaf spelling in C —
        // function-pointer types are declarator-shaped
        // (`R (*name)(T1, T2)`). Callers that need to emit a
        // declaration use `format_declarator` which special-
        // cases FnPtr. Hitting this arm means a caller asked
        // for a bare type name where only a declarator would
        // be syntactically valid; return an opaque pointer
        // typedef so the build doesn't break, but a stricter
        // audit would `unreachable!`.
        Type::FnPtr(_, _) => "void*",
        // Arc 5c: `Closure(args) -> ret` is a fat-pointer struct.
        // The leaf-name fallback is opaque; callers that need
        // the real typedef spelling go through `closure_struct_name`.
        Type::Closure(_, _) => "/* closure */",
        // Tuple `(T1, T2, …)` lowers to a per-shape C struct
        // (`intent_tuple_<tags>`) emitted in the preamble.
        // `c_leaf_type` can't synthesize a `String` so it
        // returns an opaque placeholder; callers that need
        // the storage spelling go through `c_type_name` or
        // `c_element_storage`, both of which know to emit
        // `tuple_c_struct(elements)`. Hitting this arm means
        // a caller treated a Tuple as a leaf — fall back to
        // `void*` so output stays valid C. Refines T1.1.
        Type::Tuple(_) => "/* tuple */",
        // `Struct(name)` lowers to a per-name C struct
        // (`Struct_<name>`) emitted in the preamble. Same
        // routing principle as Tuple: leaf callers get an
        // opaque placeholder; the call sites that need the
        // real spelling go through `c_type_name` /
        // `c_element_storage`. T1.2.
        Type::Struct(_) => "/* struct */",
        Type::Enum(_) => "int32_t",
        // Type params should be substituted before reaching
        // codegen — hitting this arm means a generic call
        // wasn't monomorphized. Fall back to opaque pointer
        // so the build doesn't die; phase 2 will remove. T1.4.
        Type::Param(_) => "void*",
        // Same story for Type::Apply — the monomorphization
        // pass should have replaced every generic
        // instantiation with a concrete mangled
        // Struct/Enum. Closure #281.
        Type::Apply { .. } => "void*",
        // `dyn Iface` is a fat pointer `{ &vtable, &data }`.
        // c_leaf_type can't synthesize the per-Iface typedef
        // name (returns &'static str); callers that need
        // the storage spelling will use a c_object_storage
        // helper added in Phase 3. Hitting this arm means
        // a caller treated `dyn Iface` as a leaf — fall
        // back to a generic two-pointer struct typedef so
        // the build doesn't break. Phase 1.
        Type::Object(_) => "intent_dyn",
        // Raw pointers — `*const T` / `*mut T`. c_leaf_type
        // returns &'static str so it can't synthesize the
        // per-T spelling; callers that need the full
        // declarator form (e.g. `int64_t const*`) route
        // through `c_type_name` which knows to recurse into
        // the pointee. The fallback placeholder keeps any
        // leaf-only path emitting valid C. Layer 1.1+ of
        // the unsafe plan.
        Type::Ptr(_) => "/* *const T */",
        Type::PtrMut(_) => "/* *mut T */",
        // `Pool<T>` / `Handle<T>` — Layer 2 of `unsafe.md`.
        // V1: T = i64 only, so the leaf spelling is fixed at
        // the i64 form. The bundle (`emit_intent_pool_helpers_c_body`)
        // emits the typedef + helpers when the program uses
        // Pool / Handle, gated by `program_uses_i64_pool`.
        Type::Pool(_) => "intent_pool_i64",
        Type::Handle(_) => "intent_handle_i64",
        // `Tainted<T>` — Layer 1.3 of `unsafe.md`. At codegen
        // time the taint is purely a type-level discipline;
        // the underlying value has the same machine
        // representation as the wrapped T. V1: only
        // `Tainted<i64>` is producible, so the leaf spelling
        // is fixed at `int64_t`.
        Type::Tainted(_) => "int64_t",
        // `BoundedPtr<T>` — Layer 3.2 of `unsafe.md`. Fat
        // pointer struct emitted by the bundle when the
        // program actually uses it. V1: T = i64.
        Type::BoundedPtr(_) => "intent_bptr_i64",
        // `Region` — Layer 5 v2 foundation of `unsafe.md`.
        // Bump-allocator arena struct emitted by the bundle.
        Type::Region => "intent_region",
        // `ArenaRef<T>` — Layer 5 lifetime-tagged pointer.
        // Lowers to `int64_t*` (same as raw pointer); the
        // safety is compile-time only.
        Type::ArenaRef(_) => "int64_t*",
        // `Box<T>` — owning heap pointer. L2 Phase 1. c_leaf_type
        // returns &'static str so it can't synthesize per-T
        // spellings; callers needing the full declarator (e.g.
        // `int64_t*` for `Box<i64>`) route through `c_type_name`
        // which recurses into the inner type. The fallback here
        // keeps any leaf-only path emitting valid C.
        Type::Box(_) => "/* Box<T> */",
    }
}

fn c_type_name(ty: &Type) -> String {
    match ty {
        // Layer 1.1+ of `unsafe.md` — raw pointer storage uses
        // the full declarator form (`const T*` / `T*`), not the
        // leaf-comment placeholder. Used by the let-binding
        // path for `let p: *const i64 = ...;`.
        Type::Ptr(inner) => {
            let inner_decl = format_declarator(inner, "").trim_end().to_string();
            format!("const {}*", inner_decl)
        }
        Type::PtrMut(inner) => {
            let inner_decl = format_declarator(inner, "").trim_end().to_string();
            format!("{}*", inner_decl)
        }
        // L2 Phase 1 (2026-06-07): Box<T> lowers to T* — a single
        // 64-bit owning pointer to the heap slot. malloc'd by
        // box(x), freed by the scope-exit drop emission.
        // L2 Phase 3 (2026-06-08): Box<dyn Iface> is the 16-byte
        // fat pointer struct itself (with owning .data), NOT a
        // pointer to one.
        Type::Box(inner) => match &**inner {
            Type::Object(iface) => format!("intent_dyn_{}", iface),
            _ => {
                let inner_decl = format_declarator(inner, "").trim_end().to_string();
                format!("{}*", inner_decl)
            }
        },
        Type::Vec(element) => vec_c_struct(element),
        // Closure #239: arrays in return-type position are
        // spelled as the struct wrapper `intent_arr_ret_<N>_<T>`.
        // c_type_name is called by emit_prototype +
        // emit_function for the return type and (mostly) by
        // Let stmts for binding storage. The Let path passes
        // through `format_declarator` instead so the array
        // declarator form keeps working for locals.
        Type::Array { element, length } => array_return_struct_name(element, *length),
        Type::Ref(_) | Type::RefMut(_) => {
            unreachable!("reference types do not appear in return positions")
        }
        Type::Atomic(element) => c_atomic_storage(element),
        Type::Channel(element, capacity) => c_channel_storage(element, *capacity),
        // ARC 1.4e: per-(K, V) HashMap struct name.
        Type::HashMap(k, v) => hashmap_prefix_from_kv(k, v),
        Type::Tuple(elements) => tuple_c_struct(elements),
        Type::Object(iface) => format!("intent_dyn_{}", iface),
        Type::Struct(name) => struct_c_name(name),
        // Arc 5c: `Closure(args) -> ret` lowers to a per-shape
        // fat-pointer struct typedef `intent_closure_<args>_<ret>`.
        // Emitted in the preamble once per unique signature.
        Type::Closure(args, ret) => closure_c_struct_name(args, ret),
        // T1.3 phase 2b: payloaded enums lower to the
        // tagged-union struct (`Enum_<Name>`); plain enums
        // stay as bare `int32_t` tags (via the c_leaf_type
        // fallthrough below). The registry is populated at
        // the start of `emit_c`.
        Type::Enum(name) => {
            let payloaded = ENUM_PAYLOAD_REGISTRY.with(|r| r.borrow().contains_key(name));
            if payloaded {
                enum_c_name(name)
            } else {
                "int32_t".to_string()
            }
        }
        other => c_leaf_type(other).to_string(),
    }
}

/// Per-name C struct typedef for a user-declared struct.
/// Prefixes with `Struct_` so the emitted C identifier is
/// distinct from any builtin. T1.2.
pub(crate) fn struct_c_name(name: &str) -> String {
    format!("Struct_{}", name)
}

/// Arc 5c: per-(args, ret) C struct typedef for a Closure
/// fat-pointer. The type erases the env's concrete shape;
/// runtime carries `(uint64_t env_addr, R (*call)(uint64_t, args))`.
pub(crate) fn closure_c_struct_name(args: &[Type], ret: &Type) -> String {
    let arg_tags: Vec<String> = args.iter().map(c_leaf_simple_tag).collect();
    let ret_tag = c_leaf_simple_tag(ret);
    format!("intent_closure_{}_{}", arg_tags.join("_"), ret_tag)
}

fn c_leaf_simple_tag(ty: &Type) -> String {
    match ty {
        Type::I8 => "i8".to_string(),
        Type::I16 => "i16".to_string(),
        Type::I32 => "i32".to_string(),
        Type::I64 => "i64".to_string(),
        Type::U8 => "u8".to_string(),
        Type::U16 => "u16".to_string(),
        Type::U32 => "u32".to_string(),
        Type::U64 => "u64".to_string(),
        Type::F32 => "f32".to_string(),
        Type::F64 => "f64".to_string(),
        Type::Bool => "bool".to_string(),
        _ => "i64".to_string(),
    }
}

/// Walk a Vec-element type and emit a `typedef` for every
/// struct shape that appears. Per-name emit. T1.2.
pub(crate) fn emit_struct_bundle(
    decl: &crate::ir::TypedStructDecl,
    out: &mut String,
) {
    let cname = struct_c_name(&decl.name);
    out.push_str("typedef struct {\n");
    for (fname, fty) in &decl.fields {
        // `format_declarator` handles arrays natively — `[T;N]`
        // becomes `T fname[N]` so the field is a real C array,
        // not a missing typedef ref. Other field types fall
        // through to their normal storage spelling.
        match fty {
            Type::Array { .. } => {
                out.push_str("    ");
                out.push_str(&format_declarator(fty, fname));
                out.push_str(";\n");
            }
            _ => {
                let storage = c_element_storage(fty);
                out.push_str(&format!("    {} {};\n", storage, fname));
            }
        }
    }
    out.push_str(&format!("}} {};\n", cname));
}

/// Storage type spelling for `Atomic<T>` in declarations:
/// `_Atomic <c_leaf_type(T)>`. The `_Atomic` qualifier guides
/// the C11 atomic ops to use the natural width of T. The
/// element T is restricted by the checker
/// (`is_supported_atomic_element`) to the integer widths plus
/// bool, so `c_leaf_type(element)` always returns a primitive
/// spelling.
fn c_atomic_storage(element: &Type) -> String {
    format!("_Atomic {}", c_leaf_type(element))
}

/// Helper: given `&Channel<T, N>` or `&mut Channel<T, N>`,
/// return `(T, N)`. Panics on shapes the type-checker
/// shouldn't ever produce.
fn channel_inner_from_ref(ty: &Type) -> (Type, u64) {
    match ty {
        Type::Ref(inner) | Type::RefMut(inner) => match inner.as_ref() {
            Type::Channel(elt, cap) => ((**elt).clone(), *cap),
            other => unreachable!("channel ref inner must be Channel<T, N>, got {:?}", other),
        },
        other => unreachable!("channel arg must be &Channel<T, N>, got {:?}", other),
    }
}

fn format_declarator(ty: &Type, name: &str) -> String {
    match ty {
        Type::Array { element, length } => {
            format!("{} {}[{}]", c_leaf_type(element), name, length)
        }
        Type::Vec(element) => format!("{} {}", vec_c_struct(element), name),
        Type::Tuple(elements) => format!("{} {}", tuple_c_struct(elements), name),
        Type::Object(iface) => format!("intent_dyn_{} {}", iface, name),
        Type::Struct(sname) => format!("{} {}", struct_c_name(sname), name),
        // Arc 5c: Closure params/locals declared as the
        // per-(args, ret) fat-pointer struct typedef.
        Type::Closure(args, ret) => format!("{} {}", closure_c_struct_name(args, ret), name),
        // T1.3 phase 2b: payloaded enums lower to the
        // tagged-union struct (Enum_<Name>); plain enums
        // stay as bare int32_t tags (falls through to
        // `c_leaf_type` via `other`).
        Type::Enum(ename) if ENUM_PAYLOAD_REGISTRY.with(|r| r.borrow().contains_key(ename)) => {
            format!("{} {}", enum_c_name(ename), name)
        }
        Type::Ptr(inner) => {
            // `*const T` lowers to a `const T*` (or `const
            // intent_X*` for aggregate inners). For pointer-to-
            // void style usage (struct etc.), recursion through
            // `format_declarator` on the inner type with an
            // empty name gives us a leaf storage spelling that
            // we then prefix with `const ` and suffix with `*`.
            // Layer 1.1+ of `unsafe.md`.
            let inner_decl = format_declarator(inner, "").trim_end().to_string();
            format!("const {}* {}", inner_decl, name)
        }
        Type::PtrMut(inner) => {
            // `*mut T` — same shape minus the `const`. Same
            // recursion pattern. Layer 1.1+ of `unsafe.md`.
            let inner_decl = format_declarator(inner, "").trim_end().to_string();
            format!("{}* {}", inner_decl, name)
        }
        // L2 Phase 1 (2026-06-07): `Box<T>` storage spelling is
        // `T* name` — same shape as `*mut T` but the affine
        // ownership is enforced at the checker level. malloc'd
        // by `box(x)`, freed by the scope-exit drop emission.
        // L2 Phase 3 (2026-06-08): `Box<dyn Iface>` is the
        // 16-byte fat pointer struct itself (NOT a pointer);
        // the struct's `.data` field owns the heap concrete.
        Type::Box(inner) => match &**inner {
            Type::Object(iface) => format!("intent_dyn_{} {}", iface, name),
            _ => {
                let inner_decl = format_declarator(inner, "").trim_end().to_string();
                format!("{}* {}", inner_decl, name)
            }
        },
        Type::Ref(inner) => match &**inner {
            Type::Array { element, .. } => format!("const {}* {}", c_leaf_type(element), name),
            Type::Vec(element) => format!("const {}* {}", vec_c_struct(element), name),
            // `&Atomic<T>` drops the `const` qualifier: atomic
            // operations always conceptually mutate the cell;
            // C11 atomics don't model a "read-only borrow" any
            // differently, and a `const _Atomic *` would
            // reject `atomic_store_explicit`.
            Type::Atomic(element) => format!("{}* {}", c_atomic_storage(element), name),
            Type::Channel(element, capacity) => {
                format!("{}* {}", c_channel_storage(element, *capacity), name)
            }
            Type::Tuple(elements) => format!("const {}* {}", tuple_c_struct(elements), name),
            Type::Struct(sname) => format!("const {}* {}", struct_c_name(sname), name),
            // Vtables Phase 4c: `ref dyn Iface` lowers to a
            // pointer to the per-Iface fat pointer typedef.
            Type::Object(iface) => format!("const intent_dyn_{}* {}", iface, name),
            // L2 Phase 3 (2026-06-08): `ref Box<dyn Iface>` —
            // same shape as `ref dyn Iface` since Box<dyn Iface>
            // is itself the fat pointer struct.
            Type::Box(box_inner) => match &**box_inner {
                Type::Object(iface) => format!("const intent_dyn_{}* {}", iface, name),
                _ => format!("const {}* {}",
                    format_declarator(box_inner, "").trim_end(), name),
            },
            // Phase 11 (2026-06-07): `ref T` where T is a
            // payloaded enum lowers to a pointer to the
            // `Enum_<Name>` tagged-union struct, NOT to the
            // `int32_t` tag-only form. The match-on-ref path
            // (lifting L3) reads .tag / .payload through this
            // pointer.
            Type::Enum(ename)
                if ENUM_PAYLOAD_REGISTRY.with(|r| r.borrow().contains_key(ename)) =>
            {
                format!("const {}* {}", enum_c_name(ename), name)
            }
            other => format!("const {}* {}", c_leaf_type(other), name),
        },
        Type::RefMut(inner) => match &**inner {
            Type::Array { element, .. } => format!("{}* {}", c_leaf_type(element), name),
            Type::Vec(element) => format!("{}* {}", vec_c_struct(element), name),
            Type::Atomic(element) => format!("{}* {}", c_atomic_storage(element), name),
            Type::Channel(element, capacity) => {
                format!("{}* {}", c_channel_storage(element, *capacity), name)
            }
            Type::Tuple(elements) => format!("{}* {}", tuple_c_struct(elements), name),
            Type::Struct(sname) => format!("{}* {}", struct_c_name(sname), name),
            // Vtables Phase 4c: `mut ref dyn Iface` lowers
            // to a (mutable) pointer to the fat pointer.
            Type::Object(iface) => format!("intent_dyn_{}* {}", iface, name),
            // L2 Phase 3 (2026-06-08): `mut ref Box<dyn Iface>`.
            Type::Box(box_inner) => match &**box_inner {
                Type::Object(iface) => format!("intent_dyn_{}* {}", iface, name),
                _ => format!("{}* {}",
                    format_declarator(box_inner, "").trim_end(), name),
            },
            // Phase 11 (2026-06-07): `mut ref T` where T is a
            // payloaded enum lowers to a pointer to the
            // `Enum_<Name>` tagged-union struct.
            Type::Enum(ename)
                if ENUM_PAYLOAD_REGISTRY.with(|r| r.borrow().contains_key(ename)) =>
            {
                format!("{}* {}", enum_c_name(ename), name)
            }
            other => format!("{}* {}", c_leaf_type(other), name),
        },
        Type::Atomic(element) => format!("{} {}", c_atomic_storage(element), name),
        Type::Channel(element, capacity) => {
            format!("{} {}", c_channel_storage(element, *capacity), name)
        }
        Type::FnPtr(params, ret) => {
            // C function pointer declarator:
            //   R (*name)(T1, T2, ...)
            // We format each parameter via format_declarator
            // with a synthetic empty name, then collapse the
            // trailing space — keeps array/vec/ref decay
            // happening through the existing machinery.
            let params_c: Vec<String> = params
                .iter()
                .map(|t| {
                    // No parameter name in fn-pointer
                    // declarators; format_declarator expects
                    // one so pass "" and trim. For pure-scalar
                    // params the result is "<ty> " which
                    // trims clean.
                    let s = format_declarator(t, "");
                    s.trim_end().to_string()
                })
                .collect();
            // Closure #216: when the return type is itself a
            // FnPtr, naive declarator nesting (`R (*)(...) (*name)()`)
            // is ill-formed C — the inner fn-ptr declarator
            // can't appear as a prefix-only type. Proper C
            // declarator nesting (`R (*(*name)())(...)`) is
            // syntactically complex to synthesize correctly.
            // Since all fn-ptrs are interchangeable at the C
            // storage level (`void*` in struct fields / Vec
            // slots — closures #214/#215), drop the inner
            // signature and emit `void*` for the return when
            // it's a FnPtr. Call sites handle the implicit
            // conversion (caller's `let f: fn(...) -> R = p();`
            // assigns a `void*` to a fn-pointer-typed local
            // which gcc accepts with an implicit conversion).
            let ret_decl = if matches!(ret.as_ref(), Type::FnPtr(_, _)) {
                "void*".to_string()
            } else {
                let r = format_declarator(ret, "");
                r.trim_end().to_string()
            };
            format!("{} (*{})({})", ret_decl, name, params_c.join(", "))
        }
        other => format!("{} {}", c_leaf_type(other), name),
    }
}

fn emit_runtime_helpers(out: &mut String, body: &str) {
    // Only emit helpers actually called from the body. We previously
    // emitted all of them with INTENT_UNUSED to suppress warnings,
    // but the dead helpers cluttered the generated C. Filtering by a
    // simple substring check on the rendered body keeps the output
    // proportional to what the program actually uses.
    let needs_bounds = body.contains("intent_check_bounds(");
    let divisor_kinds: &[(&str, &str, &str)] = &[
        ("i8", "int8_t", "0"),
        ("i16", "int16_t", "0"),
        ("i32", "int32_t", "0"),
        ("i64", "int64_t", "0"),
        ("u8", "uint8_t", "0"),
        ("u16", "uint16_t", "0"),
        ("u32", "uint32_t", "0"),
        ("u64", "uint64_t", "0"),
        ("f32", "float", "0.0f"),
        ("f64", "double", "0.0"),
    ];
    let shift_kinds: &[(&str, &str, bool)] = &[
        ("i8", "int8_t", true),
        ("i16", "int16_t", true),
        ("i32", "int32_t", true),
        ("i64", "int64_t", true),
        ("u8", "uint8_t", false),
        ("u16", "uint16_t", false),
        ("u32", "uint32_t", false),
        ("u64", "uint64_t", false),
    ];
    let used_divisors: Vec<&(&str, &str, &str)> = divisor_kinds
        .iter()
        .filter(|(ty, _, _)| body.contains(&format!("intent_check_{}_divisor(", ty)))
        .collect();
    let used_shifts: Vec<&(&str, &str, bool)> = shift_kinds
        .iter()
        .filter(|(ty, _, _)| body.contains(&format!("intent_check_{}_shift(", ty)))
        .collect();

    if !needs_bounds && used_divisors.is_empty() && used_shifts.is_empty() {
        return;
    }

    if needs_bounds {
        out.push_str("static INTENT_UNUSED inline uint64_t intent_check_bounds(uint64_t index, uint64_t length) { assert(index < length); return index; }\n");
    }

    for (ty, c_ty, zero) in &used_divisors {
        out.push_str("static INTENT_UNUSED inline ");
        out.push_str(c_ty);
        out.push_str(" intent_check_");
        out.push_str(ty);
        out.push_str("_divisor(");
        out.push_str(c_ty);
        out.push_str(" x) { assert(x != ");
        out.push_str(zero);
        out.push_str("); return x; }\n");
    }

    for (ty, c_ty, signed) in &used_shifts {
        out.push_str("static INTENT_UNUSED inline ");
        out.push_str(c_ty);
        out.push_str(" intent_check_");
        out.push_str(ty);
        out.push_str("_shift(");
        out.push_str(c_ty);
        out.push_str(" x, unsigned bits) { ");
        if *signed {
            out.push_str("assert(x >= 0 && ");
        } else {
            out.push_str("assert(");
        }
        out.push_str("(uint64_t)x < bits); return x; }\n");
    }
    out.push('\n');
}

fn divisor_helper(ty: &Type) -> &'static str {
    match ty {
        Type::I8 => "intent_check_i8_divisor",
        Type::I16 => "intent_check_i16_divisor",
        Type::I32 => "intent_check_i32_divisor",
        Type::I64 => "intent_check_i64_divisor",
        Type::U8 => "intent_check_u8_divisor",
        Type::U16 => "intent_check_u16_divisor",
        Type::U32 => "intent_check_u32_divisor",
        Type::U64 => "intent_check_u64_divisor",
        Type::F32 => "intent_check_f32_divisor",
        Type::F64 => "intent_check_f64_divisor",
        Type::Bool | Type::Str | Type::OwnedStr | Type::Array { .. } | Type::Vec(_) | Type::Ref(_) | Type::RefMut(_) | Type::Task | Type::Atomic(_) | Type::Channel(_, _) | Type::Mutex(_) | Type::Guard(_) | Type::Condvar | Type::Deque(_) | Type::HashSet(_) | Type::HashMap(_, _) | Type::BTreeSet(_) | Type::BTreeMap(_, _) | Type::UnionFind | Type::BinaryHeap(_) | Type::BloomFilter | Type::Bst(_) | Type::Graph | Type::Trie | Type::SkipList | Type::FnPtr(_, _) | Type::Closure(_, _) | Type::Tuple(_) | Type::Struct(_) | Type::Enum(_) | Type::Apply { .. } | Type::Param(_) | Type::Object(_) | Type::Ptr(_) | Type::PtrMut(_) | Type::Pool(_) | Type::Handle(_) | Type::Tainted(_) | Type::BoundedPtr(_) | Type::Region | Type::ArenaRef(_) | Type::Box(_) => {
            unreachable!("non-numeric type cannot be a divisor")
        }
    }
}

fn shift_helper(ty: &Type) -> &'static str {
    match ty {
        Type::I8 => "intent_check_i8_shift",
        Type::I16 => "intent_check_i16_shift",
        Type::I32 => "intent_check_i32_shift",
        Type::I64 => "intent_check_i64_shift",
        Type::U8 => "intent_check_u8_shift",
        Type::U16 => "intent_check_u16_shift",
        Type::U32 => "intent_check_u32_shift",
        Type::U64 => "intent_check_u64_shift",
        Type::F32
        | Type::F64
        | Type::Bool
        | Type::Str
        | Type::OwnedStr
        | Type::Array { .. }
        | Type::Vec(_)
        | Type::Ref(_)
        | Type::RefMut(_)
        | Type::Task
        | Type::Atomic(_)
        | Type::Channel(_, _)
        | Type::Mutex(_)
        | Type::Guard(_)
        | Type::Condvar
        | Type::Deque(_)
        | Type::HashSet(_)
        | Type::HashMap(_, _)
        | Type::BTreeSet(_)
        | Type::BTreeMap(_, _)
        | Type::UnionFind
        | Type::BinaryHeap(_)
        | Type::BloomFilter
        | Type::Bst(_)
        | Type::Graph
        | Type::Trie
        | Type::SkipList
        | Type::FnPtr(_, _) | Type::Closure(_, _) | Type::Tuple(_) | Type::Struct(_) | Type::Enum(_) | Type::Apply { .. } | Type::Param(_) | Type::Object(_) | Type::Ptr(_) | Type::PtrMut(_) | Type::Pool(_) | Type::Handle(_) | Type::Tainted(_) | Type::BoundedPtr(_) | Type::Region | Type::ArenaRef(_) | Type::Box(_) => unreachable!("shift count must be an integer"),
    }
}

pub(crate) fn function_name(name: &str) -> String {
    format!("fn_{}", sanitize_ident(name))
}

fn local_name(name: &str) -> String {
    format!("v_{}", sanitize_ident(name))
}

fn sanitize_ident(name: &str) -> String {
    name.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

/// Escape a string for safe inclusion as a C string literal (already
/// surrounded by `"`s in the emitted code).
fn escape_c_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\x{:02x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn escape_comment(text: &str) -> String {
    text.replace("*/", "* /")
}

/// Walk a TypedProgram and collect the set of interface names
/// that are actually used as `dyn Iface` somewhere — either
/// in a function param/return, struct/enum field, local
/// binding, or expression position. Interfaces declared with
/// `interface` + `implement` but never coerced to dyn don't
/// need vtable scaffolding (and would surface trampoline
/// compile errors against unused signatures).
pub(crate) fn collect_used_dyn_ifaces(program: &TypedProgram) -> std::collections::HashSet<String> {
    fn walk_type(ty: &Type, set: &mut std::collections::HashSet<String>) {
        match ty {
            Type::Object(name) => {
                set.insert(name.clone());
            }
            Type::Vec(inner) | Type::Atomic(inner) | Type::Mutex(inner)
            | Type::Guard(inner) | Type::Ref(inner) | Type::RefMut(inner) => walk_type(inner, set),
            Type::Channel(inner, _) => walk_type(inner, set),
            Type::Tuple(elements) => elements.iter().for_each(|t| walk_type(t, set)),
            Type::FnPtr(params, ret) => {
                params.iter().for_each(|t| walk_type(t, set));
                walk_type(ret, set);
            }
            Type::Array { element, .. } => walk_type(element, set),
            _ => {}
        }
    }
    fn walk_expr(expr: &TypedExpr, set: &mut std::collections::HashSet<String>) {
        walk_type(&expr.ty, set);
        match &expr.kind {
            TypedExprKind::Unary { expr, .. } => walk_expr(expr, set),
            TypedExprKind::Binary { left, right, .. } => {
                walk_expr(left, set);
                walk_expr(right, set);
            }
            TypedExprKind::Call { args, .. } | TypedExprKind::ArrayLit { elements: args } => {
                args.iter().for_each(|a| walk_expr(a, set));
            }
            TypedExprKind::Cast { expr, ty } => {
                walk_expr(expr, set);
                walk_type(ty, set);
            }
            TypedExprKind::Index { array, index, .. } => {
                walk_expr(array, set);
                walk_expr(index, set);
            }
            TypedExprKind::Len { array, .. } => walk_expr(array, set),
            TypedExprKind::CallIndirect { callee, args } => {
                walk_expr(callee, set);
                args.iter().for_each(|a| walk_expr(a, set));
            }
            TypedExprKind::Tuple { elements } => elements.iter().for_each(|e| walk_expr(e, set)),
            TypedExprKind::TupleAccess { tuple, .. } => walk_expr(tuple, set),
            TypedExprKind::StructLit { fields, .. } => {
                fields.iter().for_each(|(_, e)| walk_expr(e, set));
            }
            TypedExprKind::FieldAccess { object, .. } => walk_expr(object, set),
            TypedExprKind::EnumVariantWithPayload { payload, payload_ty, .. } => {
                walk_expr(payload, set);
                walk_type(payload_ty, set);
            }
            TypedExprKind::Match { scrutinee, arms } => {
                walk_expr(scrutinee, set);
                arms.iter().for_each(|a| walk_expr(&a.body, set));
            }
            TypedExprKind::IfExpr { cond, then_value, else_value } => {
                walk_expr(cond, set);
                walk_expr(then_value, set);
                walk_expr(else_value, set);
            }
            TypedExprKind::Block { stmts, tail } => {
                stmts.iter().for_each(|s| walk_stmt(s, set));
                walk_expr(tail, set);
            }
            TypedExprKind::DynDispatch { receiver, args, iface_name, .. } => {
                set.insert(iface_name.clone());
                walk_expr(receiver, set);
                args.iter().for_each(|a| walk_expr(a, set));
            }
            TypedExprKind::DynCoerce { value, iface_name, from_ty, .. } => {
                set.insert(iface_name.clone());
                walk_expr(value, set);
                walk_type(from_ty, set);
            }
            _ => {}
        }
    }
    fn walk_stmt(stmt: &TypedStmt, set: &mut std::collections::HashSet<String>) {
        match stmt {
            TypedStmt::Let { ty, expr, .. } => {
                walk_type(ty, set);
                walk_expr(expr, set);
            }
            TypedStmt::Reassign { ty, expr, .. } => {
                walk_type(ty, set);
                walk_expr(expr, set);
            }
            TypedStmt::Drop { ty, .. } => walk_type(ty, set),
            TypedStmt::Discard { expr } => walk_expr(expr, set),
            TypedStmt::Return { expr } => walk_expr(expr, set),
            TypedStmt::Assert { expr, .. } | TypedStmt::Prove { expr } => walk_expr(expr, set),
            TypedStmt::Print { items } => {
                for item in items {
                    if let crate::ir::TypedPrintItem::Expr(e) = item {
                        walk_expr(e, set);
                    }
                }
            }
            TypedStmt::If { cond, then_body, else_body } => {
                walk_expr(cond, set);
                then_body.iter().for_each(|s| walk_stmt(s, set));
                else_body.iter().for_each(|s| walk_stmt(s, set));
            }
            TypedStmt::While { cond, body } => {
                walk_expr(cond, set);
                body.iter().for_each(|s| walk_stmt(s, set));
            }
            TypedStmt::For { ty, start, end, body, .. } => {
                walk_type(ty, set);
                walk_expr(start, set);
                walk_expr(end, set);
                body.iter().for_each(|s| walk_stmt(s, set));
            }
            TypedStmt::ForIter { element_ty, collection_ty, body, .. } => {
                walk_type(element_ty, set);
                walk_type(collection_ty, set);
                body.iter().for_each(|s| walk_stmt(s, set));
            }
            TypedStmt::IndexAssign { index, value, base_ty, .. } => {
                walk_type(base_ty, set);
                walk_expr(index, set);
                walk_expr(value, set);
            }
            TypedStmt::FieldAssign { object, value, .. } => {
                walk_expr(object, set);
                walk_expr(value, set);
            }
            TypedStmt::TaskSpawn { body, captures, .. } => {
                captures.iter().for_each(|(_, t)| walk_type(t, set));
                body.iter().for_each(|s| walk_stmt(s, set));
            }
            TypedStmt::UnsafeBlock { body, .. } => {
                body.iter().for_each(|s| walk_stmt(s, set));
            }
            TypedStmt::TaskJoin { .. } | TypedStmt::Break | TypedStmt::Continue => {}
        }
    }
    let mut set = std::collections::HashSet::new();
    for f in &program.functions {
        walk_type(&f.return_type, &mut set);
        for p in &f.params { walk_type(&p.ty, &mut set); }
        for s in &f.body { walk_stmt(s, &mut set); }
    }
    for sd in &program.structs {
        for (_, fty) in &sd.fields { walk_type(fty, &mut set); }
    }
    for ed in &program.enums {
        for pt in &ed.payload_types {
            if let Some(t) = pt { walk_type(t, &mut set); }
        }
    }
    set
}

/// Vtables Phase 3 + Phase 4: emit per-Iface vtable forward
/// decl + `intent_dyn_<Iface>` fat-pointer typedef so structs
/// declared LATER can carry `dyn Iface` fields. The full
/// `struct intent_vtbl_<Iface>` body is emitted by
/// `emit_dyn_iface_vtable_bodies` AFTER struct typedefs so
/// it can reference `Struct_<T>` arg types if any iface
/// method takes a struct by value. Only emits for ifaces
/// actually used as `dyn Iface` somewhere.
fn emit_dyn_iface_typedefs(out: &mut String, used: &std::collections::HashSet<String>) {
    for iface in crate::ast::all_iface_names() {
        if !used.contains(&iface) { continue; }
        out.push_str(&format!(
            "typedef struct intent_vtbl_{iface} intent_vtbl_{iface};\n",
            iface = iface,
        ));
        out.push_str(&format!(
            "typedef struct intent_dyn_{iface} {{ \
const intent_vtbl_{iface}* vtable; void* data; \
}} intent_dyn_{iface};\n",
            iface = iface
        ));
    }
}

/// Vtables Phase 4: emit the full body of each
/// `struct intent_vtbl_<Iface>` after struct typedefs are
/// declared, so the fn-ptr slots can reference `Struct_<T>`
/// for methods that take structs by value.
fn emit_dyn_iface_vtable_bodies(
    out: &mut String,
    used: &std::collections::HashSet<String>,
) {
    for iface in crate::ast::all_iface_names() {
        if !used.contains(&iface) { continue; }
        let Some(methods) = crate::ast::iface_methods_for(&iface) else {
            continue;
        };
        out.push_str(&format!("struct intent_vtbl_{} {{\n", iface));
        for (idx, (_name, params, ret)) in methods.iter().enumerate() {
            let ret_ty = c_type_name(ret);
            let arg_decls: Vec<String> = std::iter::once("void*".to_string())
                .chain(params.iter().skip(1).map(|t| format_declarator(t, "").trim().to_string()))
                .collect();
            out.push_str(&format!(
                "    {} (*m{})({});\n",
                ret_ty, idx, arg_decls.join(", ")
            ));
        }
        out.push_str("};\n");
    }
}

/// Vtables Phase 3: emit per-(T, Iface) trampolines and the
/// static vtable instances they populate. A trampoline
/// converts `void* self` to the concrete self shape declared
/// by the impl method (by-value, ref, or mut-ref) and forwards
/// to the hoisted `<Type>_<method>` function.
fn emit_dyn_iface_vtables(out: &mut String, used: &std::collections::HashSet<String>) {
    for iface in crate::ast::all_iface_names() {
        if !used.contains(&iface) { continue; }
        let Some(methods) = crate::ast::iface_methods_for(&iface) else {
            continue;
        };
        for type_name in crate::ast::impls_for_iface(&iface) {
            for (idx, (method_name, params, ret)) in methods.iter().enumerate() {
                let ret_ty = c_type_name(ret);
                let self_ty = &params[0];
                let mut sig_args: Vec<String> = vec!["void* __intent_self".to_string()];
                let mut forwarded: Vec<String> = Vec::new();
                // The trampoline's self-shape follows the
                // iface declaration (value, ref, or mut-ref)
                // but the underlying nominal type comes from
                // THIS impl's concrete `for_type` — not the
                // example self the iface declaration spelled.
                // Otherwise heterogeneous impls (Circle.area,
                // Square.area) would all cast to the iface's
                // first declared self, which is wrong for any
                // non-first impl.
                let impl_struct_name = format!("Struct_{}", type_name);
                let self_forward = match self_ty {
                    Type::Struct(_) | Type::Enum(_) => {
                        format!("*(({}*)__intent_self)", impl_struct_name)
                    }
                    Type::Ref(_) => {
                        format!("(const {}*)__intent_self", impl_struct_name)
                    }
                    Type::RefMut(_) => {
                        format!("({}*)__intent_self", impl_struct_name)
                    }
                    other => {
                        panic!(
                            "vtables Phase 3: unsupported self shape `{}` for \
                             interface '{}' method '{}' — v1 supports value, \
                             ref, and mut-ref receivers only",
                            other, iface, method_name
                        );
                    }
                };
                forwarded.push(self_forward);
                for (i, pt) in params.iter().enumerate().skip(1) {
                    let pname = format!("__intent_arg{}", i);
                    sig_args.push(format_declarator(pt, &pname));
                    forwarded.push(pname);
                }
                out.push_str(&format!(
                    "static {ret} intent_trampoline_{type_name}_{iface}_{slot}_{method}({sig}) {{\n",
                    ret = ret_ty,
                    type_name = type_name,
                    iface = iface,
                    slot = idx,
                    method = method_name,
                    sig = sig_args.join(", "),
                ));
                out.push_str(&format!(
                    "    return fn_{type_name}_{method}({fwd});\n",
                    type_name = type_name,
                    method = method_name,
                    fwd = forwarded.join(", "),
                ));
                out.push_str("}\n");
            }
            out.push_str(&format!(
                "static const intent_vtbl_{iface} intent_vtbl_{iface}_{type_name} = {{\n",
                iface = iface,
                type_name = type_name,
            ));
            for (idx, (method_name, _, _)) in methods.iter().enumerate() {
                out.push_str(&format!(
                    "    .m{slot} = intent_trampoline_{type_name}_{iface}_{slot}_{method},\n",
                    slot = idx,
                    type_name = type_name,
                    iface = iface,
                    method = method_name,
                ));
            }
            out.push_str("};\n");
        }
    }
}
