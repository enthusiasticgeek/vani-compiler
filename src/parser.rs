use crate::ast::{
    BinaryOp, ConstDecl, EnumDecl, EnumVariant, Expr, ExprKind, Function, ImplDecl, Intent,
    InterfaceDecl, InterfaceMethod, MatchArm, MethodsBlock, Param, Pattern, Program, Reduction,
    ReductionOp, Stmt, StructDecl, StructField, Type, TypeAlias, UnaryOp, Use, WhereClause,
};
use crate::span::Span;

/// Parser-internal sum of the three `use`-statement shapes
/// (closures #245, #247). The top-level parse loop dispatches
/// each variant to the matching list on `Program`.
enum UseDecl {
    File(Use),
    Path(crate::ast::UsePath),
    PathMulti(Vec<crate::ast::UsePath>),
}
use crate::diagnostic::Diagnostic;
use crate::lexer::{Token, TokenKind};

pub fn parse(tokens: Vec<Token>) -> (Program, Vec<Diagnostic>) {
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program();
    (program, parser.errors)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    errors: Vec<Diagnostic>,
    /// Names of type parameters declared on the function
    /// currently being parsed (e.g. `["T", "U"]` for
    /// `fn pair<T, U>(...)`). `parse_type` consults this so a
    /// bare uppercase identifier resolves to `Type::Param`
    /// instead of `Type::Struct`. Refines T1.4.
    current_type_params: std::collections::HashSet<String>,
    /// Const declarations seen so far in the source, mapped
    /// to their literal i128 value. Populated by
    /// `parse_const_decl` when the initializer is an integer
    /// literal (including negative literals via the `Minus`
    /// prefix). Consulted by `parse_type` when an identifier
    /// appears in an array-length slot (`[T; SIZE]`). Forward
    /// references and non-literal const initializers aren't
    /// supported here. T0.0 follow-up (closure #120).
    const_int_values: std::collections::HashMap<String, i128>,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            pos: 0,
            errors: Vec::new(),
            current_type_params: std::collections::HashSet::new(),
            const_int_values: std::collections::HashMap::new(),
        }
    }

    fn parse_program(&mut self) -> Program {
        // Arc 8 v3.1 Phase 1 — clear the v3.1 task registry so
        // multi-program test runs (lib tests, parity sweeps)
        // don't accumulate stale synthesized struct/poll-fn
        // pairs from earlier compiles in the same process.
        crate::ast::V31_TASK_REGISTRY.with(|r| r.borrow_mut().clear());
        // Phase 3c — clear the enum-name → variants registry for
        // the same reason. Populated as enum decls are parsed
        // (see the EnumDecl path inside the parse loop).
        crate::ast::V31_ENUM_REGISTRY.with(|r| r.borrow_mut().clear());
        // Phase 3d — clear the struct-name → fields registry.
        // Populated as struct decls are parsed (see the StructDecl
        // paths inside the parse loop).
        crate::ast::V31_STRUCT_REGISTRY.with(|r| r.borrow_mut().clear());

        let mut intents = Vec::new();
        let mut functions = Vec::new();
        let mut uses = Vec::new();
        let mut use_paths: Vec<crate::ast::UsePath> = Vec::new();
        let mut structs = Vec::new();
        let mut enums = Vec::new();
        let mut interfaces = Vec::new();
        let mut impls = Vec::new();
        let mut consts = Vec::new();
        let mut type_aliases = Vec::new();
        let mut methods_blocks = Vec::new();
        let mut modules: Vec<crate::ast::ModuleDecl> = Vec::new();

        while !self.check(|kind| matches!(kind, TokenKind::Eof)) {
            // Closure #242: module declarations. v1 supports
            // only top-level modules (no nesting). Items inside
            // are stored in the ModuleDecl; the checker walks
            // them later and mangles names + enforces
            // visibility.
            if self.check(|k| matches!(k, TokenKind::Module)) {
                match self.parse_module_decl() {
                    Ok(m) => modules.push(m),
                    Err(e) => {
                        self.errors.push(e);
                        self.sync_to_top_level();
                    }
                }
                continue;
            }
            if self.check(|kind| matches!(kind, TokenKind::Intent)) {
                match self.parse_intent() {
                    Ok(i) => intents.push(i),
                    Err(e) => {
                        self.errors.push(e);
                        self.sync_to_top_level();
                    }
                }
            } else if self.check(|kind| matches!(kind, TokenKind::Use)) {
                // Top-level `use` — `is_pub` is meaningless
                // here (top-level items are globally visible
                // already); pass false.
                match self.parse_use(false) {
                    Ok(UseDecl::File(u)) => uses.push(u),
                    Ok(UseDecl::Path(p)) => use_paths.push(p),
                    Ok(UseDecl::PathMulti(ps)) => use_paths.extend(ps),
                    Err(e) => {
                        self.errors.push(e);
                        self.sync_to_top_level();
                    }
                }
            } else if self.check(|k| matches!(k, TokenKind::Struct)) {
                match self.parse_struct_decl() {
                    Ok(s) => {
                        // Phase 3d — stash struct_name → fields so
                        // the v3.1 desugar can synthesize per-field
                        // default-init for struct locals.
                        crate::ast::V31_STRUCT_REGISTRY.with(|reg| {
                            let fields: Vec<(String, Type)> = s.fields.iter()
                                .map(|f| (f.name.clone(), f.ty.clone()))
                                .collect();
                            reg.borrow_mut().insert(s.name.clone(), fields);
                        });
                        structs.push(s);
                    }
                    Err(e) => {
                        self.errors.push(e);
                        self.sync_to_top_level();
                    }
                }
            } else if self.check(|k| matches!(k, TokenKind::Enum)) {
                match self.parse_enum_decl() {
                    Ok(e) => {
                        // Phase 3c — stash enum_name → variants
                        // into V31_ENUM_REGISTRY so the v3.1
                        // default-init synthesizer can find the
                        // first unit variant for enum locals.
                        crate::ast::V31_ENUM_REGISTRY.with(|reg| {
                            let variants: Vec<(String, Vec<Type>)> = e.variants.iter()
                                .map(|v| (v.name.clone(), v.payload.clone()))
                                .collect();
                            reg.borrow_mut().insert(e.name.clone(), variants);
                        });
                        enums.push(e);
                    }
                    Err(err) => {
                        self.errors.push(err);
                        self.sync_to_top_level();
                    }
                }
            } else if self.check(|k| matches!(k, TokenKind::Interface)) {
                match self.parse_interface_decl() {
                    Ok(d) => interfaces.push(d),
                    Err(err) => {
                        self.errors.push(err);
                        self.sync_to_top_level();
                    }
                }
            } else if self.check(|k| matches!(k, TokenKind::Implement)) {
                match self.parse_impl_decl() {
                    Ok(d) => impls.push(d),
                    Err(err) => {
                        self.errors.push(err);
                        self.sync_to_top_level();
                    }
                }
            } else if self.check(|k| matches!(k, TokenKind::Const)) {
                match self.parse_const_decl() {
                    Ok(c) => consts.push(c),
                    Err(err) => {
                        self.errors.push(err);
                        self.sync_to_top_level();
                    }
                }
            } else if self.check(|k| matches!(k, TokenKind::Type)) {
                match self.parse_type_alias() {
                    Ok(a) => type_aliases.push(a),
                    Err(err) => {
                        self.errors.push(err);
                        self.sync_to_top_level();
                    }
                }
            } else if self.check(|k| matches!(k, TokenKind::Methods)) {
                match self.parse_methods_block() {
                    Ok(m) => methods_blocks.push(m),
                    Err(err) => {
                        self.errors.push(err);
                        self.sync_to_top_level();
                    }
                }
            } else if self.check(|k| matches!(k, TokenKind::Hash)) {
                // Closure #286: `#[bounded(N)]` attribute
                // before a function declaration. v1 only
                // recognizes the literal `bounded` attribute;
                // future attributes (`inline`, `deprecated`,
                // etc.) ride the same parser.
                match self.parse_attributed_fn() {
                    Ok(f) => functions.push(f),
                    Err(e) => {
                        self.errors.push(e);
                        self.sync_to_top_level();
                    }
                }
            } else if self.check(|kind| matches!(kind, TokenKind::Pure))
                && matches!(
                    self.tokens.get(self.pos + 1).map(|t| &t.kind),
                    Some(TokenKind::Extern)
                )
            {
                // Closure #271: `pure extern "C" fn name(...) -> R;`
                // The `pure` opt-in lets parallel-for / pure-fn
                // bodies call this extern. Caller's responsibility
                // to ensure the foreign symbol is actually pure
                // (no side effects, no shared state, deterministic
                // output) — vāṇी can't verify across the FFI
                // boundary.
                self.bump();
                match self.parse_extern_fn() {
                    Ok(mut f) => {
                        f.is_pure = true;
                        functions.push(f);
                    }
                    Err(e) => {
                        self.errors.push(e);
                        self.sync_to_top_level();
                    }
                }
            } else if self.check(|kind| matches!(kind, TokenKind::Fn | TokenKind::Pure))
                || self.check_async_prefix()
            {
                match self.parse_function() {
                    Ok(f) => functions.push(f),
                    Err(e) => {
                        self.errors.push(e);
                        self.sync_to_top_level();
                    }
                }
            } else if self.check(|kind| matches!(kind, TokenKind::Extern)) {
                // Closure #269: top-level `extern "C" fn name(...) -> R;`
                // FFI declaration. Parser handles it; the resulting
                // Function has an empty body and `is_extern = true`
                // for downstream effect / codegen routing.
                match self.parse_extern_fn() {
                    Ok(f) => functions.push(f),
                    Err(e) => {
                        self.errors.push(e);
                        self.sync_to_top_level();
                    }
                }
            } else {
                let err = self.error_here("expected 'use', 'intent', 'struct', or 'fn'");
                self.errors.push(err);
                if !self.check(|kind| matches!(kind, TokenKind::Eof)) {
                    self.bump();
                }
                self.sync_to_top_level();
            }
        }

        // Arc 8 v3.1 Phase 1 — flush V31_TASK_REGISTRY entries
        // synthesized by try_v31_transform during parse_function.
        // Each entry contributes one struct + one poll fn into
        // the program-level decls.
        //
        // Span rewrite: each synthesized item gets a span AFTER
        // all user-level decls (max_user_pos + N). This matters
        // for the `intentc fmt` pass which sorts top-items by
        // span.start — without the rewrite, the poll fn (which
        // inherits its source async fn's span) would interleave
        // back-to-back with the original fn instead of landing
        // at end-of-program where `parse_program` puts it.
        // Pre-Phase-4c-fix the round-trip diverged for async_*
        // examples.
        let max_user_pos: usize = functions
            .iter()
            .map(|f| f.span.end)
            .chain(structs.iter().map(|s| s.span.end))
            .max()
            .unwrap_or(0);
        let mut functions = functions;
        let mut structs = structs;
        crate::ast::V31_TASK_REGISTRY.with(|reg| {
            let mut bump: usize = 1;
            for (mut s, mut f) in reg.borrow_mut().drain(..) {
                let s_start = max_user_pos + bump;
                bump += 1;
                let s_end = max_user_pos + bump;
                bump += 1;
                s.span = crate::span::Span::new(s_start, s_end);
                let f_start = max_user_pos + bump;
                bump += 1;
                let f_end = max_user_pos + bump;
                bump += 1;
                f.span = crate::span::Span::new(f_start, f_end);
                structs.push(s);
                functions.push(f);
            }
        });

        Program {
            intents,
            functions,
            uses,
            structs,
            enums,
            interfaces,
            impls,
            consts,
            type_aliases,
            methods_blocks,
            modules,
            use_paths,
        }
    }

    /// Closure #242: parse a `module name { items… }` block.
    /// Items inside follow the same grammar as top-level items;
    /// each can be prefixed with `pub` to export. v1 forbids
    /// nested `module` declarations.
    fn parse_module_decl(&mut self) -> Result<crate::ast::ModuleDecl, Diagnostic> {
        let start = self.expect_keyword("'module'", |k| matches!(k, TokenKind::Module))?;
        let name_tok = self.expect_ident()?;
        let name_span = name_tok.span;
        let name = ident_text(name_tok);
        self.expect_keyword("'{' after module name", |k| matches!(k, TokenKind::LBrace))?;

        let mut functions = Vec::new();
        let mut structs = Vec::new();
        let mut enums = Vec::new();
        let mut interfaces = Vec::new();
        let mut impls = Vec::new();
        let mut consts = Vec::new();
        let mut type_aliases = Vec::new();
        let mut methods_blocks = Vec::new();
        let mut nested_modules: Vec<crate::ast::ModuleDecl> = Vec::new();
        let mut local_use_paths: Vec<crate::ast::UsePath> = Vec::new();
        let mut vis = crate::ast::ModuleVisibility::default();

        while !self.check(|k| matches!(k, TokenKind::RBrace | TokenKind::Eof)) {
            // Closure #256: `use foo::bar;` inside a module body
            // is admitted alongside item declarations. The use
            // path is scoped to this module — the checker's
            // per-module `qualify` adds it to the local alias
            // map so bare references inside the body resolve
            // through it.
            //
            // Closure #257: `pub use foo::bar;` is the re-export
            // form. The `pub` is parsed here so it precedes the
            // `use`; UsePath's `is_pub` flag picks it up. After
            // flattening, the checker builds a global re-export
            // map (`<this_mod>__<local> → <imported_mangled>`)
            // and rewrites external references to the renamed
            // form.
            //
            // Peek for `pub use` so we don't accidentally grab a
            // `pub` that belongs to an upcoming item declaration.
            let pub_use_form = self.check(|k| matches!(k, TokenKind::Pub))
                && matches!(
                    self.tokens.get(self.pos + 1).map(|t| &t.kind),
                    Some(TokenKind::Use)
                );
            if pub_use_form || self.check(|k| matches!(k, TokenKind::Use)) {
                let is_use_pub = pub_use_form;
                if pub_use_form {
                    self.bump(); // consume `pub`
                }
                match self.parse_use(is_use_pub) {
                    Ok(UseDecl::Path(p)) => local_use_paths.push(p),
                    Ok(UseDecl::PathMulti(ps)) => local_use_paths.extend(ps),
                    Ok(UseDecl::File(u)) => {
                        let span = u.span;
                        self.errors.push(Diagnostic::new(
                            span,
                            "`use \"path\";` (file imports) are only \
                             valid at the top level, not inside `module`",
                        ));
                    }
                    Err(e) => {
                        self.errors.push(e);
                        self.sync_past_brace();
                    }
                }
                continue;
            }
            // Optional `pub` modifier with an optional `(kosh)`
            // qualifier — closure #258. `pub(kosh)` records the
            // intent that an item is exported within the kosh
            // but NOT through the kosh boundary into external
            // dependents. Today vāṇī compiles a single kosh,
            // so the bit is preserved without enforcement;
            // when the future kosh boundary lands existing
            // `pub(kosh)` annotations start being enforced
            // without source rewrites.
            let is_pub = self
                .match_token(|k| matches!(k, TokenKind::Pub))
                .is_some();
            let is_kosh_only = if is_pub
                && self.check(|k| matches!(k, TokenKind::LParen))
            {
                // Peek for `(kosh)` — the only qualifier
                // we accept in v1.
                let kosh_ident_next = matches!(
                    self.tokens.get(self.pos + 1).map(|t| &t.kind),
                    Some(TokenKind::Ident(n)) if n == "kosh"
                );
                let close_paren_next = matches!(
                    self.tokens.get(self.pos + 2).map(|t| &t.kind),
                    Some(TokenKind::RParen)
                );
                if kosh_ident_next && close_paren_next {
                    self.bump(); // (
                    self.bump(); // kosh
                    self.bump(); // )
                    true
                } else {
                    let span = self.current().span;
                    self.errors.push(Diagnostic::new(
                        span,
                        "only `pub(kosh)` is supported as a `pub(…)` \
                         qualifier in v1 — write `pub` for kosh-wide \
                         visibility or `pub(kosh)` to mark an item as \
                         internal to this kosh",
                    ));
                    // Skip past the bad qualifier so the rest of
                    // the line parses cleanly instead of
                    // cascading. Consume `( ... )` greedily.
                    self.bump(); // (
                    while !self.check(|k| matches!(
                        k,
                        TokenKind::RParen | TokenKind::RBrace | TokenKind::Eof
                    )) {
                        self.bump();
                    }
                    if self.check(|k| matches!(k, TokenKind::RParen)) {
                        self.bump();
                    }
                    false
                }
            } else {
                false
            };
            // Closure #248: nested `module` blocks are now
            // supported. Recurse into the same parser.
            if self.check(|k| matches!(k, TokenKind::Module)) {
                match self.parse_module_decl() {
                    Ok(m) => {
                        nested_modules.push(m);
                        vis.modules_pub.push(is_pub);
                        vis.modules_kosh_only.push(is_kosh_only);
                    }
                    Err(e) => {
                        self.errors.push(e);
                        self.sync_past_brace();
                    }
                }
                continue;
            }

            if self.check(|k| matches!(k, TokenKind::Struct)) {
                match self.parse_struct_decl() {
                    Ok(s) => {
                        // Phase 3d — same registry population as
                        // the bare-struct branch above; covers
                        // pub/kosh-decorated struct decl sites.
                        crate::ast::V31_STRUCT_REGISTRY.with(|reg| {
                            let fields: Vec<(String, Type)> = s.fields.iter()
                                .map(|f| (f.name.clone(), f.ty.clone()))
                                .collect();
                            reg.borrow_mut().insert(s.name.clone(), fields);
                        });
                        structs.push(s);
                        vis.structs_pub.push(is_pub);
                        vis.structs_kosh_only.push(is_kosh_only);
                    }
                    Err(e) => { self.errors.push(e); self.sync_past_brace(); }
                }
            } else if self.check(|k| matches!(k, TokenKind::Enum)) {
                match self.parse_enum_decl() {
                    Ok(e) => {
                        // Phase 3c — same registry population as
                        // the bare-enum branch above; covers the
                        // pub/kosh-decorated enum site.
                        crate::ast::V31_ENUM_REGISTRY.with(|reg| {
                            let variants: Vec<(String, Vec<Type>)> = e.variants.iter()
                                .map(|v| (v.name.clone(), v.payload.clone()))
                                .collect();
                            reg.borrow_mut().insert(e.name.clone(), variants);
                        });
                        enums.push(e);
                        vis.enums_pub.push(is_pub);
                        vis.enums_kosh_only.push(is_kosh_only);
                    }
                    Err(e) => { self.errors.push(e); self.sync_past_brace(); }
                }
            } else if self.check(|k| matches!(k, TokenKind::Interface)) {
                match self.parse_interface_decl() {
                    Ok(d) => {
                        interfaces.push(d);
                        vis.interfaces_pub.push(is_pub);
                        vis.interfaces_kosh_only.push(is_kosh_only);
                    }
                    Err(e) => { self.errors.push(e); self.sync_past_brace(); }
                }
            } else if self.check(|k| matches!(k, TokenKind::Implement)) {
                match self.parse_impl_decl() {
                    Ok(d) => {
                        impls.push(d);
                        vis.impls_pub.push(is_pub);
                        vis.impls_kosh_only.push(is_kosh_only);
                    }
                    Err(e) => { self.errors.push(e); self.sync_past_brace(); }
                }
            } else if self.check(|k| matches!(k, TokenKind::Const)) {
                match self.parse_const_decl() {
                    Ok(c) => {
                        consts.push(c);
                        vis.consts_pub.push(is_pub);
                        vis.consts_kosh_only.push(is_kosh_only);
                    }
                    Err(e) => { self.errors.push(e); self.sync_past_brace(); }
                }
            } else if self.check(|k| matches!(k, TokenKind::Type)) {
                match self.parse_type_alias() {
                    Ok(a) => {
                        type_aliases.push(a);
                        vis.type_aliases_pub.push(is_pub);
                        vis.type_aliases_kosh_only.push(is_kosh_only);
                    }
                    Err(e) => { self.errors.push(e); self.sync_past_brace(); }
                }
            } else if self.check(|k| matches!(k, TokenKind::Methods)) {
                match self.parse_methods_block() {
                    Ok(m) => {
                        methods_blocks.push(m);
                        vis.methods_blocks_pub.push(is_pub);
                        vis.methods_blocks_kosh_only.push(is_kosh_only);
                    }
                    Err(e) => { self.errors.push(e); self.sync_past_brace(); }
                }
            } else if self.check(|k| matches!(k, TokenKind::Fn | TokenKind::Pure))
                || self.check_async_prefix()
            {
                match self.parse_function() {
                    Ok(f) => {
                        functions.push(f);
                        vis.functions_pub.push(is_pub);
                        vis.functions_kosh_only.push(is_kosh_only);
                    }
                    Err(e) => { self.errors.push(e); self.sync_past_brace(); }
                }
            } else {
                let err = self.error_here(
                    "expected an item declaration (fn / struct / enum / interface / implement / methods / const / type) inside `module`"
                );
                self.errors.push(err);
                if !self.check(|kind| matches!(kind, TokenKind::Eof | TokenKind::RBrace)) {
                    self.bump();
                }
            }
        }

        let close_tok = self.expect_keyword(
            "'}' to close module",
            |k| matches!(k, TokenKind::RBrace),
        )?;
        // Reject glob `use foo::*;` inside modules — v1 doesn't
        // resolve nested-module glob expansion until ALL flatten
        // passes finish, which the per-module qualify map can't
        // see. Surface a clear diagnostic at parse time so the
        // user gets the message at the conflict site.
        for up in &local_use_paths {
            if up.item == "*" {
                self.errors.push(Diagnostic::new(
                    up.span,
                    "glob `use foo::*;` inside a module is not yet \
                     supported — list the items explicitly or hoist \
                     the import to the top level",
                ));
            }
        }
        Ok(crate::ast::ModuleDecl {
            name,
            name_span,
            functions,
            structs,
            enums,
            interfaces,
            impls,
            consts,
            type_aliases,
            methods_blocks,
            modules: nested_modules,
            use_paths: local_use_paths,
            visibility: vis,
            span: start.span.merge(close_tok.span),
        })
    }

    /// Helper: skip tokens until past the next `}` (for error
    /// recovery inside a module body when a single item fails).
    fn sync_past_brace(&mut self) {
        let mut depth = 0i32;
        while !self.check(|k| matches!(k, TokenKind::Eof)) {
            if self.check(|k| matches!(k, TokenKind::LBrace)) {
                depth += 1;
            } else if self.check(|k| matches!(k, TokenKind::RBrace)) {
                if depth == 0 {
                    break;
                }
                depth -= 1;
            }
            self.bump();
        }
    }

    fn parse_methods_block(&mut self) -> Result<MethodsBlock, Diagnostic> {
        let start = self.expect_keyword("'methods'", |k| matches!(k, TokenKind::Methods))?;
        // `on` follows. It's not a reserved keyword (used
        // only in this syntax position), so accept it as an
        // identifier with the literal text "on".
        let on_tok = self.expect_ident()?;
        if ident_text(on_tok.clone()) != "on" {
            return Err(Diagnostic::new(
                on_tok.span,
                "expected 'on' in `methods on <Type> { … }`",
            ));
        }
        let ty_start_span = self.current().span;
        let for_type = self.parse_type()?;
        self.expect_keyword("'{'", |k| matches!(k, TokenKind::LBrace))?;
        let mut methods = Vec::new();
        while !self.check(|k| matches!(k, TokenKind::RBrace | TokenKind::Eof)) {
            methods.push(self.parse_function()?);
        }
        let close = self.expect_keyword("'}'", |k| matches!(k, TokenKind::RBrace))?;
        Ok(MethodsBlock {
            for_type,
            for_type_span: ty_start_span,
            methods,
            span: start.span.merge(close.span),
        })
    }

    fn parse_type_alias(&mut self) -> Result<TypeAlias, Diagnostic> {
        let start = self.expect_keyword("'type'", |k| matches!(k, TokenKind::Type))?;
        let name_tok = self.expect_ident()?;
        let name_span = name_tok.span;
        let name = ident_text(name_tok);
        self.expect_keyword("'='", |k| matches!(k, TokenKind::Equal))?;
        let target = self.parse_type()?;
        let semi = self.expect_keyword("';'", |k| matches!(k, TokenKind::Semicolon))?;
        Ok(TypeAlias {
            name,
            name_span,
            target,
            span: start.span.merge(semi.span),
        })
    }

    fn parse_const_decl(&mut self) -> Result<ConstDecl, Diagnostic> {
        let start = self.expect_keyword("'const'", |k| matches!(k, TokenKind::Const))?;
        let name_tok = self.expect_ident()?;
        let name_span = name_tok.span;
        let name = ident_text(name_tok);
        self.expect_keyword("':'", |k| matches!(k, TokenKind::Colon))?;
        let ty = self.parse_type()?;
        self.expect_keyword("'='", |k| matches!(k, TokenKind::Equal))?;
        let value = self.parse_expr()?;
        let semi = self.expect_keyword("';'", |k| matches!(k, TokenKind::Semicolon))?;
        // Stash the integer-valued initializer so `parse_type`
        // can resolve a later `[T; NAME]` length reference.
        // Only literal forms (`42`, `-1`) qualify. T0.0
        // follow-up (closure #120).
        if let Some(v) = expr_as_int_literal(&value, &self.const_int_values) {
            self.const_int_values.insert(name.clone(), v);
        }
        Ok(ConstDecl {
            name,
            name_span,
            ty,
            value,
            span: start.span.merge(semi.span),
        })
    }

    fn parse_interface_decl(&mut self) -> Result<InterfaceDecl, Diagnostic> {
        let start = self.expect_keyword("'interface'", |k| matches!(k, TokenKind::Interface))?;
        let name_tok = self.expect_ident()?;
        let name_span = name_tok.span;
        let name = ident_text(name_tok);
        self.expect_keyword("'{'", |k| matches!(k, TokenKind::LBrace))?;
        let mut methods = Vec::new();
        while !self.check(|k| matches!(k, TokenKind::RBrace | TokenKind::Eof)) {
            let fn_tok = self.expect_keyword("'fn'", |k| matches!(k, TokenKind::Fn))?;
            let m_name_tok = self.expect_ident()?;
            let m_name_span = m_name_tok.span;
            let m_name = ident_text(m_name_tok);
            self.expect_keyword("'('", |k| matches!(k, TokenKind::LParen))?;
            let mut params = Vec::new();
            if !self.check(|k| matches!(k, TokenKind::RParen)) {
                loop {
                    let p_tok = self.expect_ident()?;
                    let p_name_span = p_tok.span;
                    let p_name = ident_text(p_tok);
                    self.expect_keyword("':'", |k| matches!(k, TokenKind::Colon))?;
                    let ty = self.parse_type()?;
                    params.push(Param {
                        name: p_name,
                        ty,
                        name_span: p_name_span,
                        span: p_name_span,
                    });
                    if self.match_token(|k| matches!(k, TokenKind::Comma)).is_none() {
                        break;
                    }
                    if self.check(|k| matches!(k, TokenKind::RParen)) {
                        break;
                    }
                }
            }
            self.expect_keyword("')'", |k| matches!(k, TokenKind::RParen))?;
            self.expect_keyword(
                "'returns'", // checker uses display-only `returns`; parser still accepts `->`
                |k| matches!(k, TokenKind::Arrow),
            )?;
            let return_type = self.parse_type()?;
            // Phase 2: default body — `{ ... }` instead of `;`
            let (default_body, end_span) =
                if self.check(|k| matches!(k, TokenKind::LBrace)) {
                    let body = self.parse_block()?;
                    let end = if let Some(last) = body.last() {
                        last.span()
                    } else {
                        fn_tok.span
                    };
                    (Some(body), end)
                } else {
                    let semi = self.expect_keyword("';'", |k| matches!(k, TokenKind::Semicolon))?;
                    (None, semi.span)
                };
            methods.push(InterfaceMethod {
                name: m_name,
                name_span: m_name_span,
                params,
                return_type,
                span: fn_tok.span.merge(end_span),
                default_body,
            });
        }
        let close = self.expect_keyword("'}'", |k| matches!(k, TokenKind::RBrace))?;
        Ok(InterfaceDecl {
            name,
            name_span,
            methods,
            span: start.span.merge(close.span),
        })
    }

    fn parse_impl_decl(&mut self) -> Result<ImplDecl, Diagnostic> {
        let start = self.expect_keyword("'implement'", |k| matches!(k, TokenKind::Implement))?;
        // Phase 2: blanket impl type params — `implement<T> Iface for Type<T>`
        let mut impl_type_params: Vec<String> = Vec::new();
        if self.match_token(|k| matches!(k, TokenKind::Less)).is_some() {
            loop {
                let tp_tok = self.expect_ident()?;
                impl_type_params.push(ident_text(tp_tok));
                if self.match_token(|k| matches!(k, TokenKind::Comma)).is_none() {
                    break;
                }
                if self.check(|k| matches!(k, TokenKind::Greater | TokenKind::GreaterGreater)) {
                    break;
                }
            }
            self.expect_close_angle()?;
            // Register as type params so parse_type resolves them as Type::Param
            for tp in &impl_type_params {
                self.current_type_params.insert(tp.clone());
            }
        }
        let iface_tok = self.expect_ident()?;
        let interface_name = ident_text(iface_tok);
        // `for` is a reserved keyword (used in `for i from … to
        // …`), so dispatch on the token kind rather than trying
        // to grab it as an identifier.
        self.expect_keyword("'for' in `implement <Iface> for <Type>`", |k| {
            matches!(k, TokenKind::For)
        })?;
        let for_type = self.parse_type()?;
        // Phase 2: optional `where T is Iface` bounds on blanket impls.
        let mut impl_where_clauses: Vec<WhereClause> = Vec::new();
        if self.match_token(|k| matches!(k, TokenKind::Where)).is_some() {
            loop {
                let tp_tok = self.expect_ident()?;
                let tp_span = tp_tok.span;
                let tp_name = ident_text(tp_tok);
                self.expect_keyword("'is'", |k| matches!(k, TokenKind::Is))?;
                let iface_tok = self.expect_ident()?;
                let iface_span = iface_tok.span;
                let iface_name = ident_text(iface_tok);
                impl_where_clauses.push(WhereClause {
                    type_param: tp_name,
                    interface_name: iface_name,
                    span: tp_span.merge(iface_span),
                });
                if self.match_token(|k| matches!(k, TokenKind::Comma)).is_none() {
                    break;
                }
                if self.check(|k| matches!(k, TokenKind::LBrace)) {
                    break;
                }
            }
        }
        // Clean up blanket type params from current_type_params after parsing
        // for_type and where clauses (before parsing method bodies).
        for tp in &impl_type_params {
            self.current_type_params.remove(tp);
        }
        // Re-register for method body parsing.
        for tp in &impl_type_params {
            self.current_type_params.insert(tp.clone());
        }
        self.expect_keyword("'{'", |k| matches!(k, TokenKind::LBrace))?;
        let mut methods = Vec::new();
        while !self.check(|k| matches!(k, TokenKind::RBrace | TokenKind::Eof)) {
            methods.push(self.parse_function()?);
        }
        let close = self.expect_keyword("'}'", |k| matches!(k, TokenKind::RBrace))?;
        // Remove blanket type params from scope after impl block.
        for tp in &impl_type_params {
            self.current_type_params.remove(tp);
        }
        Ok(ImplDecl {
            interface_name,
            type_params: impl_type_params,
            where_clauses: impl_where_clauses,
            for_type,
            methods,
            span: start.span.merge(close.span),
            // Default: top-level. The flattening pass sets
            // this to Some(module_name) when the impl was
            // declared inside a `module { ... }` block.
            home_module: None,
        })
    }

    fn parse_enum_decl(&mut self) -> Result<EnumDecl, Diagnostic> {
        let start = self.expect_keyword("'enum'", |k| matches!(k, TokenKind::Enum))?;
        let name_tok = self.expect_ident()?;
        let name_span = name_tok.span;
        let name = ident_text(name_tok);
        // Closure #281: optional generic type parameters
        // `enum Option<T> { … }` / `enum Result<T, E> { … }`.
        // Mirrors the fn-generic parser at parse_function.
        let mut type_params: Vec<String> = Vec::new();
        if self.match_token(|k| matches!(k, TokenKind::Less)).is_some() {
            loop {
                let tp_tok = self.expect_ident()?;
                let tp_name = ident_text(tp_tok);
                type_params.push(tp_name);
                if self.match_token(|k| matches!(k, TokenKind::Comma)).is_none() {
                    break;
                }
                if self.check(|k| matches!(k, TokenKind::Greater | TokenKind::GreaterGreater)) {
                    break;
                }
            }
            self.expect_close_angle()?;
        }
        // Register type params so `parse_type` resolves them
        // as `Type::Param` inside variant payloads. Restored
        // at end.
        let saved_tp = self.current_type_params.clone();
        for tp in &type_params {
            self.current_type_params.insert(tp.clone());
        }
        self.expect_keyword("'{'", |k| matches!(k, TokenKind::LBrace))?;
        let mut variants = Vec::new();
        while !self.check(|k| matches!(k, TokenKind::RBrace | TokenKind::Eof)) {
            let v_tok = self.expect_ident()?;
            let v_span = v_tok.span;
            let v_name = ident_text(v_tok);
            // Optional payload: `Name(T1, T2, …)` — types only,
            // positional. T1.3 phase 2a. Named fields (`Err {
            // code: i64, msg: String }`) land in phase 2b.
            let mut payload: Vec<Type> = Vec::new();
            if self
                .match_token(|k| matches!(k, TokenKind::LParen))
                .is_some()
            {
                if !self.check(|k| matches!(k, TokenKind::RParen)) {
                    loop {
                        let ty = self.parse_type()?;
                        payload.push(ty);
                        if self
                            .match_token(|k| matches!(k, TokenKind::Comma))
                            .is_none()
                        {
                            break;
                        }
                    }
                }
                self.expect_keyword("')'", |k| matches!(k, TokenKind::RParen))?;
            }
            let comma_seen = self
                .match_token(|k| matches!(k, TokenKind::Comma))
                .is_some();
            variants.push(EnumVariant {
                name: v_name,
                name_span: v_span,
                payload,
            });
            if !comma_seen && !self.check(|k| matches!(k, TokenKind::RBrace)) {
                return Err(Diagnostic::new(
                    self.current().span,
                    "expected ',' between enum variants or '}' to close",
                ));
            }
        }
        let close = self.expect_keyword("'}'", |k| matches!(k, TokenKind::RBrace))?;
        self.current_type_params = saved_tp;
        Ok(EnumDecl {
            name,
            name_span,
            type_params,
            variants,
            span: start.span.merge(close.span),
        })
    }

    fn parse_struct_decl(&mut self) -> Result<StructDecl, Diagnostic> {
        let start = self.expect_keyword("'struct'", |k| matches!(k, TokenKind::Struct))?;
        let name_tok = self.expect_ident()?;
        let name_span = name_tok.span;
        let name = ident_text(name_tok);
        // Closure #281: optional generic type parameters
        // `struct Pair<A, B> { first: A, second: B }`.
        let mut type_params: Vec<String> = Vec::new();
        if self.match_token(|k| matches!(k, TokenKind::Less)).is_some() {
            loop {
                let tp_tok = self.expect_ident()?;
                let tp_name = ident_text(tp_tok);
                type_params.push(tp_name);
                if self.match_token(|k| matches!(k, TokenKind::Comma)).is_none() {
                    break;
                }
                if self.check(|k| matches!(k, TokenKind::Greater | TokenKind::GreaterGreater)) {
                    break;
                }
            }
            self.expect_close_angle()?;
        }
        let saved_tp = self.current_type_params.clone();
        for tp in &type_params {
            self.current_type_params.insert(tp.clone());
        }
        self.expect_keyword("'{'", |k| matches!(k, TokenKind::LBrace))?;
        let mut fields = Vec::new();
        while !self.check(|k| matches!(k, TokenKind::RBrace | TokenKind::Eof)) {
            let field_name_tok = self.expect_ident()?;
            let field_name_span = field_name_tok.span;
            let field_name = ident_text(field_name_tok);
            self.expect_keyword("':'", |k| matches!(k, TokenKind::Colon))?;
            let ty = self.parse_type()?;
            // Optional trailing comma. Required between
            // fields; allowed before `}`.
            let comma_seen = self
                .match_token(|k| matches!(k, TokenKind::Comma))
                .is_some();
            fields.push(StructField {
                name: field_name,
                ty,
                span: field_name_span,
            });
            if !comma_seen && !self.check(|k| matches!(k, TokenKind::RBrace)) {
                return Err(Diagnostic::new(
                    self.current().span,
                    "expected ',' between struct fields or '}' to close",
                ));
            }
        }
        let close = self.expect_keyword("'}'", |k| matches!(k, TokenKind::RBrace))?;
        self.current_type_params = saved_tp;
        Ok(StructDecl {
            name,
            name_span,
            type_params,
            fields,
            span: start.span.merge(close.span),
        })
    }

    /// Parse a `use` declaration. The `is_pub` flag is set
    /// by the caller when a `pub` keyword precedes the
    /// `use` (closure #257) — only meaningful inside
    /// `module { }` bodies, where it marks the import as
    /// a re-export. Five forms (closures #245, #247, #248,
    /// #253, #254):
    /// - File import: `use "path/to/file.vani";` (quoted
    ///   string, used by the multi-file pipeline).
    /// - Module-path single import: `use foo::bar;` (deep
    ///   paths `a::b::c::Item` also supported — closure #248).
    /// - Brace-list import: `use foo::{a, b};` — expands to
    ///   one `UsePath` per item.
    /// - Glob import: `use foo::*;` — closure #253. Brings
    ///   every direct public child of `foo` into scope. The
    ///   checker expands at flatten time.
    /// - Optional `as <local>` rename: `use foo::bar as baz;`
    ///   and per-entry inside brace lists. Closure #254 —
    ///   resolves collisions between same-leaf imports.
    /// - Optional `pub` prefix (`pub use foo::bar;`) marks
    ///   the import as a re-export when inside a module
    ///   body — closure #257. The caller passes `is_pub`;
    ///   `parse_use` itself doesn't consume the `pub` token.
    fn parse_use(&mut self, is_pub: bool) -> Result<UseDecl, Diagnostic> {
        let start = self.expect_keyword("'use'", |kind| matches!(kind, TokenKind::Use))?;
        // Peek: string token means file import; identifier
        // means module-path import.
        if matches!(self.current().kind, TokenKind::Str(_)) {
            let path_token = self.expect_string()?;
            let semi = self.expect_keyword("';'", |kind| matches!(kind, TokenKind::Semicolon))?;
            let TokenKind::Str(path) = path_token.kind else {
                unreachable!("expect_string only returns string tokens")
            };
            return Ok(UseDecl::File(Use {
                path,
                span: start.span.merge(semi.span),
            }));
        }
        // Module-path import. Reads `a::b::…::final_segment`
        // — closure #248 added support for deep paths
        // (nested modules). Everything before the final
        // segment is the module prefix; the final segment is
        // the item (single-item form), the `{…}` brace list
        // (multi-item form), or `*` (glob form — closure #253
        // brings every direct public child of the module into
        // scope).
        let mod_tok = self.expect_ident()?;
        let mut module = ident_text(mod_tok);
        self.expect_keyword("'::' in `use` path", |k| matches!(k, TokenKind::ColonColon))?;
        // Greedily consume `IDENT ::` segments until we see
        // either an ident-not-followed-by-`::` (single-item),
        // a `{` (multi-item), or a `*` (glob). The final ident
        // before `;` or `{` is the item; earlier idents are
        // module segments joined with `__`.
        loop {
            if self.check(|k| matches!(k, TokenKind::LBrace | TokenKind::Star)) {
                break;
            }
            // Peek one ahead to decide: if `IDENT ::` we
            // consume the IDENT as a deeper module segment.
            // Otherwise the next ident is the leaf.
            let next_is_pathsep = self
                .tokens
                .get(self.pos + 1)
                .map(|t| matches!(t.kind, TokenKind::ColonColon))
                .unwrap_or(false);
            if !next_is_pathsep {
                break;
            }
            let segment_tok = self.expect_ident()?;
            let segment = ident_text(segment_tok);
            module = format!("{}__{}", module, segment);
            self.expect_keyword(
                "'::' in `use` path",
                |k| matches!(k, TokenKind::ColonColon),
            )?;
        }
        // Glob form: `use foo::*;` — closure #253. Capture as a
        // UsePath with the sentinel item `*`; the checker
        // expands it after module flattening (so it can see
        // which items are public). v1 imports only DIRECT
        // children of the named module — `use foo::*;` does
        // NOT pull in `foo::bar::baz` (the nested-module
        // bar's items). Users who want deeper imports write
        // them explicitly (`use foo::bar::baz;` or
        // `use foo::bar::*;`).
        if self.check(|k| matches!(k, TokenKind::Star)) {
            let star = self.bump();
            let semi = self.expect_keyword("';'", |kind| matches!(kind, TokenKind::Semicolon))?;
            return Ok(UseDecl::Path(crate::ast::UsePath {
                module,
                item: "*".to_string(),
                alias: None,
                is_pub,
                span: start.span.merge(semi.span.merge(star.span)),
            }));
        }
        // Multi-item form: `use foo::{a, b, c as cc};`. Each
        // entry independently accepts an optional `as rename`.
        if self.check(|k| matches!(k, TokenKind::LBrace)) {
            self.bump(); // {
            let mut items: Vec<(String, Option<String>, crate::span::Span)> = Vec::new();
            while !self.check(|k| matches!(k, TokenKind::RBrace | TokenKind::Eof)) {
                let item_tok = self.expect_ident()?;
                let item_span = item_tok.span;
                let item_name = ident_text(item_tok);
                // Closure #254: optional `as <alias>` after each
                // brace-list entry.
                let (alias, end_span) = if self
                    .match_token(|k| matches!(k, TokenKind::As))
                    .is_some()
                {
                    let alias_tok = self.expect_ident()?;
                    let span = alias_tok.span;
                    (Some(ident_text(alias_tok)), span)
                } else {
                    (None, item_span)
                };
                items.push((item_name, alias, item_span.merge(end_span)));
                if self
                    .match_token(|k| matches!(k, TokenKind::Comma))
                    .is_none()
                {
                    break;
                }
                // Allow trailing comma.
                if self.check(|k| matches!(k, TokenKind::RBrace)) {
                    break;
                }
            }
            self.expect_keyword("'}' in `use { … }` list", |k| matches!(k, TokenKind::RBrace))?;
            let semi = self.expect_keyword("';'", |kind| matches!(kind, TokenKind::Semicolon))?;
            if items.is_empty() {
                return Err(Diagnostic::new(
                    start.span.merge(semi.span),
                    "`use foo::{}` must list at least one item",
                ));
            }
            let paths: Vec<crate::ast::UsePath> = items
                .into_iter()
                .map(|(item, alias, item_span)| crate::ast::UsePath {
                    module: module.clone(),
                    item,
                    alias,
                    is_pub,
                    span: start.span.merge(item_span),
                })
                .collect();
            return Ok(UseDecl::PathMulti(paths));
        }
        // Single-item form: `use foo::bar;` or
        // `use foo::bar as baz;` (closure #254 — local rename
        // resolves collisions between same-leaf imports).
        let item_tok = self.expect_ident()?;
        let item = ident_text(item_tok);
        let alias = if self
            .match_token(|k| matches!(k, TokenKind::As))
            .is_some()
        {
            let alias_tok = self.expect_ident()?;
            Some(ident_text(alias_tok))
        } else {
            None
        };
        let semi = self.expect_keyword("';'", |kind| matches!(kind, TokenKind::Semicolon))?;
        Ok(UseDecl::Path(crate::ast::UsePath {
            module,
            item,
            alias,
            is_pub,
            span: start.span.merge(semi.span),
        }))
    }

    /// Skip tokens until we reach a known top-level start (`fn` / `intent`)
    /// or EOF, so the outer loop can resume parsing the next definition.
    fn sync_to_top_level(&mut self) {
        while !self.check(|kind| {
            matches!(
                kind,
                TokenKind::Fn | TokenKind::Pure | TokenKind::Intent | TokenKind::Use | TokenKind::Eof
            )
        }) {
            self.bump();
        }
    }

    /// Skip tokens until we reach a known statement boundary inside a
    /// function body. Consumes a trailing `;` so the outer loop can resume
    /// cleanly on the next statement.
    fn sync_to_stmt(&mut self) {
        while !self.check(|kind| {
            matches!(
                kind,
                TokenKind::Semicolon
                    | TokenKind::RBrace
                    | TokenKind::Eof
                    | TokenKind::Let
                    | TokenKind::Return
                    | TokenKind::Assert
                    | TokenKind::Prove
                    | TokenKind::Print
                    | TokenKind::If
                    | TokenKind::While
                    | TokenKind::For
                    | TokenKind::Break
                    | TokenKind::Continue
            )
        }) {
            self.bump();
        }
        if self.check(|kind| matches!(kind, TokenKind::Semicolon)) {
            self.bump();
        }
    }

    fn parse_intent(&mut self) -> Result<Intent, Diagnostic> {
        let start = self.expect_keyword("intent", |kind| matches!(kind, TokenKind::Intent))?;
        let text_token = self.expect_string()?;
        let semi = self.expect_keyword("';'", |kind| matches!(kind, TokenKind::Semicolon))?;

        let TokenKind::Str(text) = text_token.kind else {
            unreachable!("expect_string only returns string tokens")
        };

        Ok(Intent {
            text,
            span: start.span.merge(semi.span),
        })
    }

    fn parse_function(&mut self) -> Result<Function, Diagnostic> {
        // Arc 8 step 8b — optional `async` modifier before `fn`.
        // `async` is a contextual keyword (still a regular ident
        // in expression position); recognized here only when
        // followed by `fn` or `pure fn`. Sets `is_async` so the
        // body / return-type rewrite below wraps the result in
        // `Future.Ready(...)` and reshapes `-> R` to
        // `-> Future<R>`.
        let is_async = self.check_async_prefix();
        if is_async {
            self.bump(); // consume `async`
        }
        // Optional `pure` modifier before `fn`.
        let is_pure = self
            .match_token(|kind| matches!(kind, TokenKind::Pure))
            .is_some();
        let fn_token = self.expect_keyword("'fn'", |kind| matches!(kind, TokenKind::Fn))?;
        let name_token = self.expect_ident()?;
        let name_span = name_token.span;
        // Devanagari surface (Phase 1) extension: the entry-point
        // function can be spelled `main`, `मुख्य` (mukhya),
        // `प्रमुख` (pramukh), or `प्रधान` (pradhan) — all common
        // Sanskrit/Hindi/Marathi words for "main / primary /
        // principal". The parser canonicalizes any of the
        // Devanagari forms to `main` so the checker's entry-point
        // lookup, the backends' symbol emission, and the runtime
        // all keep treating `main` as the unique entry. A program
        // that declares two of these forms (e.g. both `main` and
        // `मुख्य`) errors at the existing duplicate-fn check.
        let name = canonicalize_entry_point_name(ident_text(name_token));

        // Optional generic parameter list: `<T1, T2, …>` after
        // the fn name. Names recorded into `type_params`; the
        // checker uses them to recognize `Type::Param(name)`
        // inside the signature / body. T1.4 phase 1: syntax
        // accepted; full monomorphization lands in phase 2.
        let mut type_params: Vec<String> = Vec::new();
        if self.match_token(|k| matches!(k, TokenKind::Less)).is_some() {
            loop {
                let tp_tok = self.expect_ident()?;
                let tp_name = ident_text(tp_tok);
                type_params.push(tp_name);
                if self.match_token(|k| matches!(k, TokenKind::Comma)).is_none() {
                    break;
                }
                // Allow trailing comma before `>` so
                // multi-line generic param lists match
                // the style accepted everywhere else.
                if self.check(|k| matches!(k, TokenKind::Greater | TokenKind::GreaterGreater))
                {
                    break;
                }
            }
            self.expect_close_angle()?;
        }
        // Register the type params so `parse_type` resolves
        // them as `Type::Param` everywhere they appear in
        // this function's signature + body. Cleared at end.
        let saved_tp = self.current_type_params.clone();
        for tp in &type_params {
            self.current_type_params.insert(tp.clone());
        }
        self.expect_keyword("'('", |kind| matches!(kind, TokenKind::LParen))?;
        let mut params = Vec::new();
        if !self.check(|kind| matches!(kind, TokenKind::RParen)) {
            loop {
                let param_name_token = self.expect_ident()?;
                let param_name_span = param_name_token.span;
                let param_name = ident_text(param_name_token);
                self.expect_keyword("':'", |kind| matches!(kind, TokenKind::Colon))?;
                let ty = self.parse_type()?;
                // Until the type-annotation grammar exposes
                // a "type span", the parameter's full span
                // matches its name span. Either is fine for
                // LSP semantic tokens; goto-def will pick
                // the smaller name span when the cursor
                // lands directly on the identifier.
                params.push(Param {
                    name: param_name,
                    ty,
                    name_span: param_name_span,
                    span: param_name_span,
                });

                if self
                    .match_token(|kind| matches!(kind, TokenKind::Comma))
                    .is_none()
                {
                    break;
                }
                // Allow trailing comma in param def so
                // multi-line signatures match the style
                // accepted by struct fields, enum variants,
                // and call-site arg lists.
                if self.check(|k| matches!(k, TokenKind::RParen)) {
                    break;
                }
            }
        }
        self.expect_keyword("')'", |kind| matches!(kind, TokenKind::RParen))?;
        // Unit-return shorthand: `fn name() { body }` (no
        // `->` arrow) is sugar for `fn name() -> i64 { body
        // return 0; }`. The parser auto-fills the i64 return
        // type and the body-rewrite pass appends a synthetic
        // `return 0;` if no explicit return is present. The
        // caller can ignore the i64 (use bare `f();` or
        // `let _ = f();`). T1.0 follow-up (closure #115).
        let unit_return = !self.check(|k| matches!(k, TokenKind::Arrow));
        let return_type = if unit_return {
            Type::I64
        } else {
            self.expect_keyword("'->'", |kind| matches!(kind, TokenKind::Arrow))?;
            self.parse_type()?
        };

        // Optional `where T is Iface, U is Hash, …` bounds.
        // T1.5 phase 1: syntax accepted; checker emits a WIP
        // gate if any program declares interfaces or impls,
        // since dispatch + bounded generics land in phase 2.
        let mut where_clauses: Vec<WhereClause> = Vec::new();
        if self
            .match_token(|k| matches!(k, TokenKind::Where))
            .is_some()
        {
            loop {
                let tp_tok = self.expect_ident()?;
                let tp_span = tp_tok.span;
                let tp_name = ident_text(tp_tok);
                self.expect_keyword("'is'", |k| matches!(k, TokenKind::Is))?;
                let iface_tok = self.expect_ident()?;
                let iface_span = iface_tok.span;
                let iface_name = ident_text(iface_tok);
                where_clauses.push(WhereClause {
                    type_param: tp_name,
                    interface_name: iface_name,
                    span: tp_span.merge(iface_span),
                });
                if self
                    .match_token(|k| matches!(k, TokenKind::Comma))
                    .is_none()
                {
                    break;
                }
                // Allow trailing comma in where-clause
                // bounds list — after the final comma
                // the next token is `{` (body start) or
                // a contract keyword.
                if self.check(|k| {
                    matches!(
                        k,
                        TokenKind::LBrace | TokenKind::Requires | TokenKind::Ensures
                    )
                }) {
                    break;
                }
            }
        }

        let mut requires = Vec::new();
        let mut ensures = Vec::new();
        loop {
            if self
                .match_token(|kind| matches!(kind, TokenKind::Requires))
                .is_some()
            {
                let condition = self.parse_expr()?;
                self.expect_keyword("';'", |kind| matches!(kind, TokenKind::Semicolon))?;
                requires.push(condition);
            } else if self
                .match_token(|kind| matches!(kind, TokenKind::Ensures))
                .is_some()
            {
                let condition = self.parse_expr()?;
                self.expect_keyword("';'", |kind| matches!(kind, TokenKind::Semicolon))?;
                ensures.push(condition);
            } else {
                break;
            }
        }

        self.expect_keyword("'{'", |kind| matches!(kind, TokenKind::LBrace))?;
        let mut body = Vec::new();
        while !self.check(|kind| matches!(kind, TokenKind::RBrace | TokenKind::Eof)) {
            match self.parse_stmt() {
                Ok(s) => body.push(s),
                Err(e) => {
                    self.errors.push(e);
                    self.sync_to_stmt();
                }
            }
        }
        let close = self.expect_keyword("'}'", |kind| matches!(kind, TokenKind::RBrace))?;

        // Unit-return shorthand: append a synthetic `return 0;`
        // if the body didn't end with one. Idempotent — if
        // the user wrote `return 0;` themselves it would
        // already be there.
        if unit_return {
            let last_is_return = matches!(body.last(), Some(Stmt::Return { .. }));
            if !last_is_return {
                body.push(Stmt::Return {
                    expr: Expr {
                        kind: ExprKind::Int(0),
                        span: close.span,
                    },
                    span: close.span,
                });
            }
        }

        self.current_type_params = saved_tp;
        // Arc 8 step 8b: async-fn body / return-type desugar.
        // - Wrap return type `R` as `Future<R>` via `Type::Apply`.
        // - Walk the body, replace each `Return { expr }` with
        //   `Return { expr: Future.Ready(expr) }` so the
        //   fn-body's value-flow types as `Future<R>`.
        // v1 ships synchronous semantics: an `async fn` runs to
        // completion immediately on call, returning
        // `Future.Ready(value)`. The user-facing TYPE signature
        // matches Rust's; the actual suspend/resume runtime
        // (state-machine transform + event loop) is queued as
        // Arc 8 steps 8c–8h.
        let (final_return_type, final_body) = if is_async {
            // Arc 8 v3.1 Phase 1 — try the state-machine
            // transform first. If body has io_*_async calls
            // AND satisfies linear-core shape, the transform
            // produces a constructor body + Task struct return
            // type and queues the synthesized struct/poll fn
            // in V31_TASK_REGISTRY for parse_program to flush.
            // Otherwise fall through to v1 sync desugar below.
            match try_v31_transform(
                &name,
                fn_token.span.merge(name_span),
                &type_params,
                &params,
                &body,
                &return_type,
            ) {
                Some(Ok((task_ret, ctor_body))) => (task_ret, ctor_body),
                Some(Err(diag)) => {
                    self.errors.push(diag);
                    let wrapped_ret = Type::Apply {
                        name: "Future".to_string(),
                        args: vec![return_type],
                    };
                    let mut new_body = body;
                    wrap_returns_in_future_ready(&mut new_body);
                    (wrapped_ret, new_body)
                }
                None => {
                    let wrapped_ret = Type::Apply {
                        name: "Future".to_string(),
                        args: vec![return_type],
                    };
                    let mut new_body = body;
                    wrap_returns_in_future_ready(&mut new_body);
                    (wrapped_ret, new_body)
                }
            }
        } else {
            (return_type, body)
        };
        Ok(Function {
            name,
            type_params,
            where_clauses,
            params,
            return_type: final_return_type,
            requires,
            ensures,
            body: final_body,
            span: fn_token.span.merge(close.span).merge(name_span),
            is_pure,
            is_extern: false,
            no_heap: false,
            no_float: false,
            no_recursion: false,
            interrupt: false,
            safety_standard: None,
            bounded_stack: None,
            wcet_cycles: None,
            deterministic_timing: false,
            recursion_bound: None,
        })
    }

    /// Closure #269: parse a body-less `extern "C" fn` declaration.
    /// Surface form:
    ///   `extern "C" fn name(p1: T1, p2: T2, …) -> R;`
    /// The body is supplied by an externally-linked object file
    /// (`.o` / `.a`); the checker registers only the signature.
    /// v1 limits the ABI to `"C"`; other strings reject. Param
    /// + return types are restricted to scalars / `Str` / `ref T`
    /// — affine types (Vec, OwnedStr, etc.) crossing the FFI
    /// boundary need explicit conversion helpers, not implicit.
    /// Parse one-or-more `#[…]` attributes preceding a `fn`
    /// declaration, then the function itself. Recognized
    /// attributes:
    /// - `#[bounded(N)]` — recursion-depth bound (closure
    ///   #286). Sets `recursion_bound = Some(N)`.
    /// - `#[no_heap]` — function (and transitive callees)
    ///   must not allocate. T1.2 of the safety-standard
    ///   alignment arc. Sets `no_heap = true`.
    /// Multiple attributes stack — `#[bounded(10)] #[no_heap]
    /// fn foo() { … }` is valid; constraints union.
    fn parse_attributed_fn(&mut self) -> Result<Function, Diagnostic> {
        let mut bound_value: Option<u64> = None;
        let mut no_heap = false;
        let mut no_float = false;
        let mut no_recursion = false;
        let mut interrupt = false;
        let mut bounded_stack: Option<u64> = None;
        let mut wcet_cycles: Option<u64> = None;
        let mut deterministic_timing = false;
        let mut safety_standard: Option<String> = None;
        while self.check(|k| matches!(k, TokenKind::Hash)) {
            self.bump(); // consume `#`
            self.expect_keyword("'['", |k| matches!(k, TokenKind::LBracket))?;
            let attr_name_tok = self.expect_ident()?;
            let attr_name = ident_text(attr_name_tok);
            match attr_name.as_str() {
                "bounded" => {
                    self.expect_keyword(
                        "'(' after `bounded`",
                        |k| matches!(k, TokenKind::LParen),
                    )?;
                    let n_tok = self.bump();
                    let n = match n_tok.kind {
                        TokenKind::Int(v) if v >= 0 => v as u64,
                        _ => {
                            return Err(Diagnostic::new(
                                n_tok.span,
                                "expected a non-negative integer literal as the bound",
                            ));
                        }
                    };
                    self.expect_keyword("')'", |k| matches!(k, TokenKind::RParen))?;
                    bound_value = Some(n);
                }
                "no_heap" => no_heap = true,
                "no_float" => no_float = true,
                "no_recursion" => no_recursion = true,
                "interrupt" => interrupt = true,
                "deterministic_timing" => deterministic_timing = true,
                // T3.1: `#[bounded_stack(bytes=N)]` — declare a
                // per-fn stack budget. The post-check pass runs
                // the call-graph stack-depth estimator from this
                // fn as entry and verifies the worst-case bound.
                "bounded_stack" => {
                    self.expect_keyword(
                        "'(' after `bounded_stack`",
                        |k| matches!(k, TokenKind::LParen),
                    )?;
                    let key_tok = self.expect_ident()?;
                    let key = ident_text(key_tok);
                    if key != "bytes" {
                        return Err(Diagnostic::new(
                            self.current().span,
                            format!(
                                "expected `bytes` key in `#[bounded_stack(bytes=N)]`, got `{}`",
                                key
                            ),
                        ));
                    }
                    self.expect_keyword(
                        "'=' after `bytes`",
                        |k| matches!(k, TokenKind::Equal),
                    )?;
                    let n_tok = self.bump();
                    let n = match n_tok.kind {
                        TokenKind::Int(v) if v > 0 => v as u64,
                        _ => {
                            return Err(Diagnostic::new(
                                n_tok.span,
                                "expected a positive integer literal as the byte bound",
                            ));
                        }
                    };
                    self.expect_keyword("')'", |k| matches!(k, TokenKind::RParen))?;
                    bounded_stack = Some(n);
                }
                // T3.2: `#[wcet(cycles=N)]` — per-fn worst-case
                // execution time budget. Post-check pass runs a
                // coarse cycle estimator and rejects if the
                // estimate exceeds N or returns UNBOUNDED.
                "wcet" => {
                    self.expect_keyword(
                        "'(' after `wcet`",
                        |k| matches!(k, TokenKind::LParen),
                    )?;
                    let key_tok = self.expect_ident()?;
                    let key = ident_text(key_tok);
                    if key != "cycles" {
                        return Err(Diagnostic::new(
                            self.current().span,
                            format!(
                                "expected `cycles` key in `#[wcet(cycles=N)]`, got `{}`",
                                key
                            ),
                        ));
                    }
                    self.expect_keyword(
                        "'=' after `cycles`",
                        |k| matches!(k, TokenKind::Equal),
                    )?;
                    let n_tok = self.bump();
                    let n = match n_tok.kind {
                        TokenKind::Int(v) if v > 0 => v as u64,
                        _ => {
                            return Err(Diagnostic::new(
                                n_tok.span,
                                "expected a positive integer literal as the cycle bound",
                            ));
                        }
                    };
                    self.expect_keyword("')'", |k| matches!(k, TokenKind::RParen))?;
                    wcet_cycles = Some(n);
                }
                // Standard composite tags. Each names the
                // target safety standard; the parser expands
                // to the constraint set the standard requires
                // (set the underlying primitive bools below).
                // The composite tag string is preserved so the
                // deviations extractor can populate the
                // `target_standard` column.
                "misra_c_2012" | "asil_d" | "do178c_level_a"
                | "iec_62304_class_c" => {
                    if let Some(existing) = &safety_standard {
                        return Err(Diagnostic::new(
                            self.current().span,
                            format!(
                                "function already tagged for `{}` standard — \
                                 stack additional primitives like `#[no_float]` \
                                 to tighten constraints, but only one composite \
                                 standard tag per function",
                                existing
                            ),
                        ));
                    }
                    safety_standard = Some(attr_name.clone());
                }
                other => {
                    return Err(Diagnostic::new(
                        self.current().span,
                        format!(
                            "unknown attribute '#[{}]' — recognized in v1: \
                             primitives `#[bounded(N)]`, `#[no_heap]`, \
                             `#[no_float]`, `#[no_recursion]`, `#[interrupt]`, \
                             `#[bounded_stack(bytes=N)]`, `#[wcet(cycles=N)]`, \
                             `#[deterministic_timing]`; \
                             standard composites `#[misra_c_2012]`, `#[asil_d]`, \
                             `#[do178c_level_a]`, `#[iec_62304_class_c]`",
                            other
                        ),
                    ));
                }
            }
            self.expect_keyword("']'", |k| matches!(k, TokenKind::RBracket))?;
        }
        // Continue with the fn declaration. Supports both
        // plain `fn` and `pure fn`.
        let mut f = self.parse_function()?;
        f.recursion_bound = bound_value;
        f.no_float = no_float;
        // Standard-composite expansion. Each named composite
        // expands to a set of primitive constraints. V1 maps
        // all four composites to the union of currently-
        // shippable primitives (`no_heap` + `no_recursion`);
        // the sets diverge as Tier 3 primitives land
        // (`bounded_stack`, `wcet`, `deterministic_timing`).
        // The composite NAME is preserved on `f.safety_standard`
        // so the deviations extractor can label each record
        // with its target standard.
        let composite_no_heap = safety_standard.is_some();
        let composite_no_recursion = safety_standard.is_some();
        // `#[interrupt]` composite: no_heap + no_recursion +
        // no_lock + no_spawn (last two enforced in
        // `enforce_interrupt`).
        f.no_heap = no_heap || interrupt || composite_no_heap;
        f.no_recursion = no_recursion || interrupt || composite_no_recursion;
        f.interrupt = interrupt;
        f.safety_standard = safety_standard;
        f.bounded_stack = bounded_stack;
        f.wcet_cycles = wcet_cycles;
        f.deterministic_timing = deterministic_timing;
        Ok(f)
    }

    fn parse_extern_fn(&mut self) -> Result<Function, Diagnostic> {
        let start = self
            .expect_keyword("'extern'", |k| matches!(k, TokenKind::Extern))?;
        // Require an explicit ABI string — only "C" in v1.
        let abi_tok = self.expect_string()?;
        let TokenKind::Str(abi) = abi_tok.kind else {
            unreachable!("expect_string only returns Str tokens")
        };
        if abi != "C" {
            return Err(Diagnostic::new(
                abi_tok.span,
                format!(
                    "only `extern \"C\"` is supported in v1; got `extern \"{}\"`",
                    abi
                ),
            ));
        }
        self.expect_keyword("'fn' after `extern \"C\"`", |k| matches!(k, TokenKind::Fn))?;
        let name_tok = self.expect_ident()?;
        let name = ident_text(name_tok);
        self.expect_keyword("'(' after extern fn name", |k| matches!(k, TokenKind::LParen))?;
        let mut params: Vec<Param> = Vec::new();
        if !self.check(|k| matches!(k, TokenKind::RParen)) {
            loop {
                let param_name_tok = self.expect_ident()?;
                let param_name_span = param_name_tok.span;
                let param_name = ident_text(param_name_tok);
                self.expect_keyword("':' after extern param name", |k| matches!(k, TokenKind::Colon))?;
                let ty = self.parse_type()?;
                let param_span = param_name_span.merge(self.current().span);
                params.push(Param {
                    name: param_name,
                    ty,
                    name_span: param_name_span,
                    span: param_span,
                });
                if self.match_token(|k| matches!(k, TokenKind::Comma)).is_none() {
                    break;
                }
            }
        }
        self.expect_keyword("')' to close extern fn params", |k| matches!(k, TokenKind::RParen))?;
        self.expect_keyword("'->' before extern fn return type", |k| matches!(k, TokenKind::Arrow))?;
        let return_type = self.parse_type()?;
        let semi = self
            .expect_keyword("';' after extern fn signature", |k| matches!(k, TokenKind::Semicolon))?;
        Ok(Function {
            name,
            type_params: Vec::new(),
            where_clauses: Vec::new(),
            params,
            return_type,
            requires: Vec::new(),
            ensures: Vec::new(),
            body: Vec::new(),
            span: start.span.merge(semi.span),
            is_pure: false,
            no_heap: false,
            no_float: false,
            no_recursion: false,
            interrupt: false,
            safety_standard: None,
            bounded_stack: None,
            wcet_cycles: None,
            deterministic_timing: false,
            is_extern: true,
            recursion_bound: None,
        })
    }

    fn parse_type(&mut self) -> Result<Type, Diagnostic> {
        // Tuple type `(T1, T2, …, Tn)` — fixed-size product.
        // Must come before any other `(` consumer in this
        // function. v1 caps at 4 elements; the checker
        // enforces the cap so the parser stays simple.
        // Refines T1.1.
        if matches!(self.current().kind, TokenKind::LParen) {
            let start_span = self.current().span;
            self.bump();
            let mut elements = Vec::new();
            elements.push(self.parse_type()?);
            // Must see at least one comma to qualify as a
            // tuple — a single parenthesized type
            // `(T)` is just grouping.
            self.expect_keyword(
                "',' (tuple type needs at least two elements)",
                |k| matches!(k, TokenKind::Comma),
            )?;
            loop {
                elements.push(self.parse_type()?);
                if self
                    .match_token(|k| matches!(k, TokenKind::Comma))
                    .is_none()
                {
                    break;
                }
                // Trailing comma after last element is allowed.
                if self.check(|k| matches!(k, TokenKind::RParen)) {
                    break;
                }
            }
            self.expect_keyword("')'", |k| matches!(k, TokenKind::RParen))?;
            let _ = start_span;
            return Ok(Type::Tuple(elements));
        }
        // `fn(T1, T2, ...) -> R` — first-class function pointer
        // type. Must come BEFORE the `fn` keyword's primary
        // role as a declaration starter (`fn name() -> R { … }`)
        // would steal the lookahead. Here we're already in a
        // type position, so `fn` unambiguously names the
        // function-pointer type constructor.
        if matches!(self.current().kind, TokenKind::Fn) {
            self.bump();
            self.expect_keyword("'('", |kind| matches!(kind, TokenKind::LParen))?;
            let mut params = Vec::new();
            if !self.check(|kind| matches!(kind, TokenKind::RParen)) {
                loop {
                    params.push(self.parse_type()?);
                    if self
                        .match_token(|kind| matches!(kind, TokenKind::Comma))
                        .is_none()
                    {
                        break;
                    }
                }
            }
            self.expect_keyword("')'", |kind| matches!(kind, TokenKind::RParen))?;
            self.expect_keyword("'->'", |kind| matches!(kind, TokenKind::Arrow))?;
            let ret = self.parse_type()?;
            return Ok(Type::FnPtr(params, Box::new(ret)));
        }
        // Type position borrows: `ref T` / `mut ref T`. Refines
        // T0.0 — replaces the prior `&T` / `&mut T` shape with a
        // keyword form. `mut ref T` is the only valid composition;
        // `ref mut T` is intentionally rejected so the modifier
        // order matches the call-site form (`mut ref x`).
        if self.check(|kind| matches!(kind, TokenKind::Mut))
            && matches!(
                self.tokens.get(self.pos + 1).map(|t| &t.kind),
                Some(TokenKind::Ref)
            )
        {
            self.bump(); // mut
            self.bump(); // ref
            let inner = self.parse_type()?;
            return Ok(Type::RefMut(Box::new(inner)));
        }
        if self
            .match_token(|kind| matches!(kind, TokenKind::Ref))
            .is_some()
        {
            let inner = self.parse_type()?;
            return Ok(Type::Ref(Box::new(inner)));
        }
        // Raw pointer types — `*const T` and `*mut T`. Permitted
        // syntactically anywhere a type appears; the checker
        // enforces that they only live inside an
        // `unsafe(reason = "...")` context (block or function).
        // Layer 1.1+ of `unsafe.md`. The `*` token is `Star` —
        // the same token used for binary multiplication; the
        // parser disambiguates by position (type vs expr).
        if matches!(self.current().kind, TokenKind::Star) {
            self.bump(); // *
            if self
                .match_token(|k| matches!(k, TokenKind::Const))
                .is_some()
            {
                let inner = self.parse_type()?;
                return Ok(Type::Ptr(Box::new(inner)));
            }
            if self
                .match_token(|k| matches!(k, TokenKind::Mut))
                .is_some()
            {
                let inner = self.parse_type()?;
                return Ok(Type::PtrMut(Box::new(inner)));
            }
            let span = self.current().span;
            return Err(Diagnostic::new(
                span,
                "raw pointer type needs `*const T` or `*mut T` — \
                 the mutability marker is mandatory so reviewers \
                 can tell at a glance whether the pointer can be \
                 written through (Layer 1.1+ of unsafe.md)",
            ));
        }
        // Friendly diagnostic if the source still uses the old
        // `&T` / `&mut T` shape.
        if matches!(self.current().kind, TokenKind::Amp | TokenKind::AndAnd) {
            let span = self.current().span;
            return Err(Diagnostic::new(
                span,
                "use `ref T` / `mut ref T` for reference types (T0.0 syntax sweep)",
            ));
        }

        if self
            .match_token(|kind| matches!(kind, TokenKind::Vec))
            .is_some()
        {
            self.expect_keyword("'<'", |kind| matches!(kind, TokenKind::Less))?;
            let element = self.parse_type()?;
            self.expect_close_angle()?;
            return Ok(Type::Vec(Box::new(element)));
        }

        if self
            .match_token(|kind| matches!(kind, TokenKind::LBracket))
            .is_some()
        {
            let element = self.parse_type()?;
            self.expect_keyword("';'", |kind| matches!(kind, TokenKind::Semicolon))?;
            let length_token = self.bump();
            // Accept either an integer literal or an
            // identifier naming a previously-declared
            // integer-literal const. T0.0 follow-up (closure
            // #120).
            let raw_length = match &length_token.kind {
                TokenKind::Int(v) => *v,
                TokenKind::Ident(name) => match self.const_int_values.get(name) {
                    Some(v) => *v,
                    None => {
                        return Err(Diagnostic::new(
                            length_token.span,
                            format!(
                                "array length '{}' must be a literal integer or a \
                                 previously-declared `const NAME: i64 = <int>;`",
                                name
                            ),
                        ));
                    }
                },
                _ => {
                    return Err(Diagnostic::new(
                        length_token.span,
                        "expected integer literal or const identifier for array length",
                    ));
                }
            };
            if raw_length < 0 {
                return Err(Diagnostic::new(
                    length_token.span,
                    "array length must be non-negative",
                ));
            }
            if raw_length > u64::MAX as i128 {
                return Err(Diagnostic::new(
                    length_token.span,
                    "array length does not fit in u64",
                ));
            }
            self.expect_keyword("']'", |kind| matches!(kind, TokenKind::RBracket))?;
            return Ok(Type::Array {
                element: Box::new(element),
                length: raw_length as u64,
            });
        }

        // `Str` is recognized as a type via the ident token. It's
        // not a lexer keyword because the identifier `Str` may also
        // come up elsewhere; the type position is the only place we
        // accept it for now. `Task` is recognized the same way.
        if let TokenKind::Ident(name) = &self.current().kind {
            // `dyn IfaceName` — fat-pointer interface object.
            // Epic A Phase 1 (closure #220). Contextual keyword
            // recognition keeps the lexer simple; only the type
            // position interprets `dyn` specially.
            if name == "dyn" {
                self.bump();
                let iface_token = self.bump();
                let iface_name = match &iface_token.kind {
                    TokenKind::Ident(n) => n.clone(),
                    _ => {
                        return Err(Diagnostic::new(
                            iface_token.span,
                            "expected an interface name after `dyn`",
                        ));
                    }
                };
                return Ok(Type::Object(iface_name));
            }
            // Arc 5c: `Closure(T1, T2, …) -> R` — fat-pointer
            // callable. Mirrors `fn(...) -> R` parsing but
            // produces `Type::Closure` so the checker can
            // distinguish closures from plain fn-pointers (the
            // former carries an env, the latter doesn't).
            if name == "Closure" {
                self.bump(); // Closure
                self.expect_keyword("'('", |kind| matches!(kind, TokenKind::LParen))?;
                let mut params = Vec::new();
                if !self.check(|kind| matches!(kind, TokenKind::RParen)) {
                    loop {
                        params.push(self.parse_type()?);
                        if self
                            .match_token(|kind| matches!(kind, TokenKind::Comma))
                            .is_none()
                        {
                            break;
                        }
                    }
                }
                self.expect_keyword("')'", |kind| matches!(kind, TokenKind::RParen))?;
                self.expect_keyword("'->'", |kind| matches!(kind, TokenKind::Arrow))?;
                let ret = self.parse_type()?;
                return Ok(Type::Closure(params, Box::new(ret)));
            }
            if name == "Str" {
                self.bump();
                return Ok(Type::Str);
            }
            if name == "OwnedStr" {
                self.bump();
                return Ok(Type::OwnedStr);
            }
            if name == "Task" {
                self.bump();
                return Ok(Type::Task);
            }
            if name == "Condvar" {
                self.bump();
                return Ok(Type::Condvar);
            }
            if name == "Barrier" {
                self.bump();
                return Ok(Type::Barrier);
            }
            if name == "Deque" {
                self.bump();
                self.expect_keyword("'<'", |kind| matches!(kind, TokenKind::Less))?;
                let element = self.parse_type()?;
                self.expect_close_angle()?;
                return Ok(Type::Deque(Box::new(element)));
            }
            // L2 Phase 1 (2026-06-07): Box<T> — owning heap pointer.
            // Affine; freed at scope exit. v1 restricts T to Copy
            // primitives + Copy structs; the checker enforces.
            // Lookahead: only treat `Box` as the builtin type when
            // immediately followed by `<` (the generic open).
            // Otherwise fall through to the regular type-name path
            // so user code that declares `struct Box { … }` still
            // works.
            if name == "Box"
                && matches!(
                    self.tokens.get(self.pos + 1).map(|t| &t.kind),
                    Some(TokenKind::Less)
                )
            {
                self.bump();
                self.expect_keyword("'<'", |kind| matches!(kind, TokenKind::Less))?;
                let element = self.parse_type()?;
                self.expect_close_angle()?;
                return Ok(Type::Box(Box::new(element)));
            }
            if name == "HashSet" {
                self.bump();
                self.expect_keyword("'<'", |kind| matches!(kind, TokenKind::Less))?;
                let element = self.parse_type()?;
                self.expect_close_angle()?;
                return Ok(Type::HashSet(Box::new(element)));
            }
            if name == "HashMap" {
                self.bump();
                self.expect_keyword("'<'", |kind| matches!(kind, TokenKind::Less))?;
                let k = self.parse_type()?;
                self.expect_keyword("','", |kind| matches!(kind, TokenKind::Comma))?;
                let v = self.parse_type()?;
                self.expect_close_angle()?;
                return Ok(Type::HashMap(Box::new(k), Box::new(v)));
            }
            if name == "BTreeSet" {
                self.bump();
                self.expect_keyword("'<'", |kind| matches!(kind, TokenKind::Less))?;
                let element = self.parse_type()?;
                self.expect_close_angle()?;
                return Ok(Type::BTreeSet(Box::new(element)));
            }
            if name == "BTreeMap" {
                self.bump();
                self.expect_keyword("'<'", |kind| matches!(kind, TokenKind::Less))?;
                let k = self.parse_type()?;
                self.expect_keyword("','", |kind| matches!(kind, TokenKind::Comma))?;
                let v = self.parse_type()?;
                self.expect_close_angle()?;
                return Ok(Type::BTreeMap(Box::new(k), Box::new(v)));
            }
            if name == "UnionFind" {
                // No type params — bare name like `Condvar`.
                self.bump();
                return Ok(Type::UnionFind);
            }
            if name == "BinaryHeap" {
                self.bump();
                self.expect_keyword("'<'", |kind| matches!(kind, TokenKind::Less))?;
                let element = self.parse_type()?;
                self.expect_close_angle()?;
                return Ok(Type::BinaryHeap(Box::new(element)));
            }
            if name == "BloomFilter" {
                self.bump();
                return Ok(Type::BloomFilter);
            }
            if name == "Bst" {
                self.bump();
                self.expect_keyword("'<'", |kind| matches!(kind, TokenKind::Less))?;
                let element = self.parse_type()?;
                self.expect_close_angle()?;
                return Ok(Type::Bst(Box::new(element)));
            }
            if name == "Graph" {
                self.bump();
                return Ok(Type::Graph);
            }
            if name == "Trie" {
                self.bump();
                return Ok(Type::Trie);
            }
            if name == "SkipList" {
                self.bump();
                return Ok(Type::SkipList);
            }
            // `Pool<T>` and `Handle<T>` — Layer 2 of `unsafe.md`.
            // The Pool is affine; the Handle is Copy. Builtins
            // and codegen land in Layers 2.1b / 2.1c.
            if name == "Pool" {
                self.bump();
                self.expect_keyword("'<'", |kind| matches!(kind, TokenKind::Less))?;
                let element = self.parse_type()?;
                self.expect_close_angle()?;
                return Ok(Type::Pool(Box::new(element)));
            }
            if name == "Handle" {
                self.bump();
                self.expect_keyword("'<'", |kind| matches!(kind, TokenKind::Less))?;
                let element = self.parse_type()?;
                self.expect_close_angle()?;
                return Ok(Type::Handle(Box::new(element)));
            }
            // `Tainted<T>` — Layer 1.3 of `unsafe.md`. Wrapper
            // produced by raw-pointer deref (`*p`) once that
            // operator lands; for now the only producer is
            // the explicit `taint(v)` builtin (intended as a
            // testing/bootstrapping hook for the wrapper).
            if name == "Tainted" {
                self.bump();
                self.expect_keyword("'<'", |kind| matches!(kind, TokenKind::Less))?;
                let element = self.parse_type()?;
                self.expect_close_angle()?;
                return Ok(Type::Tainted(Box::new(element)));
            }
            // `BoundedPtr<T>` — Layer 3.2 of `unsafe.md`. Fat
            // pointer with runtime bounds checks on the
            // indexed-access path.
            if name == "BoundedPtr" {
                self.bump();
                self.expect_keyword("'<'", |kind| matches!(kind, TokenKind::Less))?;
                let element = self.parse_type()?;
                self.expect_close_angle()?;
                return Ok(Type::BoundedPtr(Box::new(element)));
            }
            // `Region` — Layer 5 v2 foundation of `unsafe.md`.
            // Bare name (no type params); the v1 scaffolding is
            // bytes-only (`region_alloc_i64` is the only
            // allocator; future commits can add per-T variants).
            if name == "Region" {
                self.bump();
                return Ok(Type::Region);
            }
            // `ArenaRef<T>` — Layer 5 lifetime-tagged pointer.
            // Bound to a Region's scope by the no-escape pass.
            if name == "ArenaRef" {
                self.bump();
                self.expect_keyword("'<'", |kind| matches!(kind, TokenKind::Less))?;
                let element = self.parse_type()?;
                self.expect_close_angle()?;
                return Ok(Type::ArenaRef(Box::new(element)));
            }
            if name == "Atomic" {
                self.bump();
                self.expect_keyword("'<'", |kind| matches!(kind, TokenKind::Less))?;
                let element = self.parse_type()?;
                self.expect_close_angle()?;
                return Ok(Type::Atomic(Box::new(element)));
            }
            if name == "Channel" {
                self.bump();
                self.expect_keyword("'<'", |kind| matches!(kind, TokenKind::Less))?;
                let element = self.parse_type()?;
                // Optional `, N` capacity. The checker
                // validates N is a power of two ≥ 1; we just
                // parse the integer literal here.
                let capacity = if self
                    .match_token(|kind| matches!(kind, TokenKind::Comma))
                    .is_some()
                {
                    let tok = self.current().clone();
                    match tok.kind {
                        TokenKind::Int(n) if n > 0 => {
                            self.bump();
                            n as u64
                        }
                        _ => {
                            return Err(Diagnostic::new(
                                tok.span,
                                "expected positive integer capacity after ',' in Channel<T, N>",
                            ));
                        }
                    }
                } else {
                    16
                };
                self.expect_close_angle()?;
                return Ok(Type::Channel(Box::new(element), capacity));
            }
            if name == "Mutex" {
                self.bump();
                self.expect_keyword("'<'", |kind| matches!(kind, TokenKind::Less))?;
                let element = self.parse_type()?;
                self.expect_close_angle()?;
                return Ok(Type::Mutex(Box::new(element)));
            }
            if name == "Guard" {
                self.bump();
                self.expect_keyword("'<'", |kind| matches!(kind, TokenKind::Less))?;
                let element = self.parse_type()?;
                self.expect_close_angle()?;
                return Ok(Type::Guard(Box::new(element)));
            }
            if name == "RwLock" {
                self.bump();
                self.expect_keyword("'<'", |kind| matches!(kind, TokenKind::Less))?;
                let element = self.parse_type()?;
                self.expect_close_angle()?;
                return Ok(Type::RwLock(Box::new(element)));
            }
            if name == "ReadGuard" {
                self.bump();
                self.expect_keyword("'<'", |kind| matches!(kind, TokenKind::Less))?;
                let element = self.parse_type()?;
                self.expect_close_angle()?;
                return Ok(Type::ReadGuard(Box::new(element)));
            }
            if name == "WriteGuard" {
                self.bump();
                self.expect_keyword("'<'", |kind| matches!(kind, TokenKind::Less))?;
                let element = self.parse_type()?;
                self.expect_close_angle()?;
                return Ok(Type::WriteGuard(Box::new(element)));
            }
            // Single-letter or "T"-prefixed names that match
            // an in-scope type parameter resolve to
            // `Type::Param` so the checker's substitution
            // pass can target them. Anything else (uppercase
            // ident not in `current_type_params`) is a
            // user-declared nominal type — `Type::Struct`
            // is the placeholder until a checker pass
            // determines whether it's actually struct or
            // enum. T1.4.
            if self.current_type_params.contains(name) {
                let n = name.clone();
                self.bump();
                return Ok(Type::Param(n));
            }
            // Closure #248: module-qualified type names with
            // arbitrarily-deep paths `a::b::c::Type`. The
            // last segment must start uppercase (type
            // convention); inner segments are module names
            // joined with `__`.
            if self
                .tokens
                .get(self.pos + 1)
                .map(|t| matches!(t.kind, TokenKind::ColonColon))
                .unwrap_or(false)
            {
                let mut path = name.clone();
                self.bump(); // module name
                loop {
                    self.bump(); // ::
                    let next_tok = self.expect_ident()?;
                    let next = ident_text(next_tok);
                    path = format!("{}__{}", path, next);
                    if !self.check(|k| matches!(k, TokenKind::ColonColon)) {
                        break;
                    }
                }
                let last_segment = path.rsplit("__").next().unwrap_or(&path);
                let starts_uppercase = last_segment
                    .chars()
                    .next()
                    .map(|c| c.is_ascii_uppercase())
                    .unwrap_or(false);
                if !starts_uppercase {
                    return Err(Diagnostic::new(
                        self.current().span,
                        "expected an uppercase type name as the last segment \
                         of the path (only types can appear in type position)",
                    ));
                }
                return Ok(Type::Struct(path));
            }
            if name
                .chars()
                .next()
                .map(|c| c.is_ascii_uppercase())
                .unwrap_or(false)
            {
                let n = name.clone();
                self.bump();
                // Closure #281: optional generic args
                // `Name<T1, T2>`. If present, build a
                // `Type::Apply` for the monomorphization
                // pass to resolve. Otherwise return the bare
                // `Type::Struct(n)` (or `Type::Enum(n)` —
                // disambiguated in the checker).
                if self.match_token(|k| matches!(k, TokenKind::Less)).is_some() {
                    let mut args: Vec<Type> = Vec::new();
                    loop {
                        let arg_ty = self.parse_type()?;
                        args.push(arg_ty);
                        if self
                            .match_token(|k| matches!(k, TokenKind::Comma))
                            .is_none()
                        {
                            break;
                        }
                        if self.check(|k| matches!(k, TokenKind::Greater | TokenKind::GreaterGreater)) {
                            break;
                        }
                    }
                    self.expect_close_angle()?;
                    return Ok(Type::Apply { name: n, args });
                }
                return Ok(Type::Struct(n));
            }
        }

        let ty = match self.current().kind {
            TokenKind::I8 => Type::I8,
            TokenKind::I16 => Type::I16,
            TokenKind::I32 => Type::I32,
            TokenKind::I64 => Type::I64,
            TokenKind::U8 => Type::U8,
            TokenKind::U16 => Type::U16,
            TokenKind::U32 => Type::U32,
            TokenKind::U64 => Type::U64,
            TokenKind::F32 => Type::F32,
            TokenKind::F64 => Type::F64,
            TokenKind::Bool => Type::Bool,
            _ => {
                return Err(self.error_here(
                    "expected type like 'i32', 'u64', 'f64', 'bool', 'Str', or '[T; N]'",
                ))
            }
        };
        self.bump();
        Ok(ty)
    }

    fn parse_stmt(&mut self) -> Result<Stmt, Diagnostic> {
        if self.check(|kind| matches!(kind, TokenKind::Let)) {
            self.parse_let_stmt()
        } else if self.check(|kind| matches!(kind, TokenKind::Return)) {
            self.parse_return_stmt()
        } else if self.check(|kind| matches!(kind, TokenKind::Assert)) {
            self.parse_assert_stmt()
        } else if self.check(|kind| matches!(kind, TokenKind::Prove)) {
            self.parse_prove_stmt()
        } else if self.check(|kind| matches!(kind, TokenKind::Print)) {
            self.parse_print_stmt()
        } else if self.check(|kind| matches!(kind, TokenKind::If)) {
            self.parse_if_stmt()
        } else if self.check(|kind| matches!(kind, TokenKind::Label(_))) {
            self.parse_labeled_loop_stmt()
        } else if self.check(|kind| matches!(kind, TokenKind::While)) {
            self.parse_while_stmt()
        } else if self.check(|kind| matches!(kind, TokenKind::For)) {
            self.parse_for_stmt()
        } else if self.looks_like_sov_for() {
            // Closure #265: Devanagari SOV (subject-object-verb)
            // word order for the range `for`. The English form
            // `for i from 0 to 5` reads as `के लिए i से 0 तक 5`
            // with Devanagari keywords — but that puts `से`
            // (from) BEFORE its operand, which is grammatically
            // wrong in Hindi/Sanskrit/Marathi (postpositions
            // follow nouns). The natural shape is
            // `i के लिए 0 से 5 तक { … }` — variable, then "for"
            // postposition; operand, then `से`; operand, then
            // `तक`. We detect `IDENT For …` and route to the
            // SOV parser. AST shape is identical to the
            // English form.
            self.parse_sov_for_stmt(false)
        } else if self.looks_like_sov_parallel_for() {
            // Same SOV detection for `parallel for`. Hindi:
            // `समान्तर i के लिए 0 से 5 तक reduce total with +; { … }`.
            self.bump(); // consume Parallel keyword
            self.parse_sov_for_stmt(true)
        } else if self.check(|kind| matches!(kind, TokenKind::Parallel)) {
            // `parallel for i in start..end { … }` — the modifier
            // precedes `for`. Consume it then dispatch to the
            // for-stmt parser with the parallel flag.
            self.bump();
            self.parse_parallel_for_stmt()
        } else if self.check(|kind| matches!(kind, TokenKind::Task)) {
            self.parse_task_spawn_stmt()
        } else if self.check(|kind| matches!(kind, TokenKind::Join)) {
            self.parse_task_join_stmt()
        } else if self.check(|kind| matches!(kind, TokenKind::Unsafe)) {
            self.parse_unsafe_block_stmt()
        } else if self.check(|kind| matches!(kind, TokenKind::RegionKw)) {
            self.parse_region_block_stmt()
        } else if self.check(|kind| matches!(kind, TokenKind::Break)) {
            let token = self.bump();
            let label = if self.check(|kind| matches!(kind, TokenKind::Label(_))) {
                let t = self.bump();
                if let TokenKind::Label(name) = t.kind { Some(name) } else { unreachable!() }
            } else {
                None
            };
            let value = if self.check(|kind| matches!(kind, TokenKind::Semicolon)) {
                None
            } else {
                Some(Box::new(self.parse_expr()?))
            };
            let semi = self.expect_keyword("';'", |kind| matches!(kind, TokenKind::Semicolon))?;
            Ok(Stmt::Break {
                label,
                value,
                span: token.span.merge(semi.span),
            })
        } else if self.check(|kind| matches!(kind, TokenKind::Continue)) {
            let token = self.bump();
            let label = if self.check(|kind| matches!(kind, TokenKind::Label(_))) {
                let t = self.bump();
                if let TokenKind::Label(name) = t.kind { Some(name) } else { unreachable!() }
            } else {
                None
            };
            let semi = self.expect_keyword("';'", |kind| matches!(kind, TokenKind::Semicolon))?;
            Ok(Stmt::Continue {
                label,
                span: token.span.merge(semi.span),
            })
        } else if let Some(verb) = self.looks_like_sov_block_verb() {
            // Tier C SOV-S3/S4 (2026-06-06): SOV block-form
            // verb-at-end for `if` / `while`. Indo-Aryan natural
            // shape puts the verb after the condition before the
            // body block: `cond यदि { ... }` / `cond यावत् { ... }`.
            // The classical keyword-first shape (`यदि cond { ... }`)
            // also still works; the detector only fires when the
            // verb is immediately followed by `{` at depth 0.
            //
            // Must run BEFORE looks_like_sov_verb_at_end because
            // the latter's `;`-anchored scan would over-consume
            // into subsequent statements (the SOV-block form has
            // no terminal `;` of its own, so verb-at-end would
            // find a later stmt's `;` and return that stmt's
            // verb).
            self.parse_sov_block_stmt(verb)
        } else if let Some(verb) = self.looks_like_sov_verb_at_end() {
            // Closure #266: Devanagari SOV verb-at-end. Statements
            // that read naturally with the verb at the end
            // (`X पुनरागम;` = "X return;") route through the
            // matching SOV parser. AST shape is identical to
            // the English form.
            self.parse_sov_verb_stmt(verb)
        } else if self.looks_like_assignment() {
            self.parse_assign_stmt()
        } else if self.looks_like_index_assign() {
            self.parse_index_assign_stmt()
        } else if self.looks_like_field_assign() {
            self.parse_field_assign_stmt()
        } else if self.looks_like_index_then_field_assign() {
            // Parse `<ident>[…].field = …;` directly into
            // `Stmt::IndexAssign` with a non-empty
            // `field_path`. T1.2 phase 2b follow-up.
            self.parse_index_then_field_assign_stmt()
        } else if self.check(|kind| matches!(kind, TokenKind::LBrace)) {
            // Bare block `{ … }` as a statement — provides an
            // explicit nested scope. Desugars to
            // `if true { … }` at parse time so the existing
            // If-scope machinery handles binding visibility,
            // affine moves, and codegen. The constant-fold
            // path collapses the `if true` away in both
            // backends. T1.0 follow-up (closure #116).
            let start = self.current().span;
            let stmts = self.parse_block()?;
            let end_span = stmts.last().map(|s| s.span()).unwrap_or(start);
            Ok(Stmt::If {
                cond: Expr {
                    kind: ExprKind::Bool(true),
                    span: start,
                },
                then_body: stmts,
                else_body: Vec::new(),
                span: start.merge(end_span),
            })
        } else {
            // Last-chance fallback: try to parse an expression
            // followed by `;`. This enables side-effect-bearing
            // call / method-call statements (`x.bump();`, `foo();`)
            // without forcing users to write `let _ = …;`. The
            // expression's value is discarded; the checker enforces
            // that the result isn't an affine type that would silently
            // leak (and the existing `let _ = …` desugaring covers
            // the drop chain for Copy results).
            let saved_pos = self.pos;
            let start_span = self.current().span;
            match self.parse_expr() {
                Ok(expr) => {
                    if let Some(semi) = self.match_token(|k| matches!(k, TokenKind::Semicolon)) {
                        // Restrict to call-shaped expressions so we
                        // don't accidentally absorb things that look
                        // like statements gone wrong (e.g. `x;`).
                        if !matches!(expr.kind, ExprKind::Call { .. } | ExprKind::MethodCall { .. }) {
                            self.pos = saved_pos;
                            return Err(self.error_here("expected statement"));
                        }
                        return Ok(Stmt::Let {
                            name: "_".to_string(),
                            annotation: None,
                            expr,
                            span: start_span.merge(semi.span),
                        });
                    }
                    self.pos = saved_pos;
                    Err(self.error_here("expected statement"))
                }
                Err(_) => {
                    self.pos = saved_pos;
                    Err(self.error_here("expected statement"))
                }
            }
        }
    }

    /// `<ident> [ … ] . <ident> =` (or longer chain) —
    /// the not-yet-supported mixed-place-assign shape.
    /// Used to give users a clean diagnostic + workaround
    /// instead of the opaque "expected statement". v1
    /// limitation; lifting it would require place-tracker
    /// codegen for chained-index-and-field lvalues.
    fn looks_like_index_then_field_assign(&self) -> bool {
        if !matches!(self.current().kind, TokenKind::Ident(_)) {
            return false;
        }
        let mut i = self.pos + 1;
        if !matches!(self.tokens.get(i).map(|t| &t.kind), Some(TokenKind::LBracket)) {
            return false;
        }
        let mut depth: i32 = 1;
        i += 1;
        while let Some(tok) = self.tokens.get(i) {
            match tok.kind {
                TokenKind::LBracket => depth += 1,
                TokenKind::RBracket => {
                    depth -= 1;
                    if depth == 0 {
                        i += 1;
                        break;
                    }
                }
                TokenKind::Eof => return false,
                _ => {}
            }
            i += 1;
        }
        // After `]`, look for `.<ident>` followed eventually
        // by `=`.
        if !matches!(self.tokens.get(i).map(|t| &t.kind), Some(TokenKind::Dot)) {
            return false;
        }
        i += 1;
        // Scan past `.<ident>(.<ident>)*` to find the `=`.
        loop {
            if !matches!(
                self.tokens.get(i).map(|t| &t.kind),
                Some(TokenKind::Ident(_))
            ) {
                return false;
            }
            i += 1;
            match self.tokens.get(i).map(|t| &t.kind) {
                Some(TokenKind::Equal) => return true,
                Some(TokenKind::Dot) => {
                    i += 1;
                }
                _ => return false,
            }
        }
    }

    fn looks_like_assignment(&self) -> bool {
        if !matches!(self.current().kind, TokenKind::Ident(_)) {
            return false;
        }
        matches!(
            self.tokens.get(self.pos + 1).map(|t| &t.kind),
            Some(TokenKind::Equal)
        )
    }

    /// `IDENT For …` — Devanagari SOV-style range-for header
    /// (closure #265). Natural Hindi / Sanskrit / Marathi
    /// puts the loop variable BEFORE the `for` postposition
    /// (`के लिए`) and the operands BEFORE `से` (from) / `तक`
    /// (to). The detection key is current==Ident AND next==For.
    fn looks_like_sov_for(&self) -> bool {
        if !matches!(self.current().kind, TokenKind::Ident(_)) {
            return false;
        }
        matches!(
            self.tokens.get(self.pos + 1).map(|t| &t.kind),
            Some(TokenKind::For)
        )
    }

    /// Tier C SOV-S3/S4 (2026-06-06): SOV block-form verb-at-end
    /// detector. Scans forward from `self.pos` looking for an
    /// `If` / `While` / `Match` keyword that's:
    ///   (a) at brace/paren depth 0 (i.e. not inside a sub-
    ///       expression's parentheses or block), and
    ///   (b) immediately followed by `{`
    /// — that combination is the SOV signature
    /// (`<cond> यदि { body }`). Returns the verb's TokenKind so
    /// the dispatcher can route to the matching SOV block parser.
    ///
    /// Returns None on:
    ///   - `;` at depth 0 (statement terminator before any block
    ///     verb — not SOV form)
    ///   - EOF reached without finding a verb+brace at depth 0
    ///   - verb found at the very start (`self.pos`) — that means
    ///     the keyword-first form is being used; let the existing
    ///     dispatchers handle it
    fn looks_like_sov_block_verb(&self) -> Option<TokenKind> {
        let mut depth: i32 = 0;
        let mut i = self.pos;
        loop {
            let Some(tok) = self.tokens.get(i) else {
                return None;
            };
            match &tok.kind {
                TokenKind::If | TokenKind::While | TokenKind::Match if depth == 0 => {
                    if i <= self.pos {
                        return None; // keyword-first form
                    }
                    if let Some(next) = self.tokens.get(i + 1) {
                        if matches!(next.kind, TokenKind::LBrace) {
                            return Some(tok.kind.clone());
                        }
                    }
                    // Otherwise this If/While/Match is inside an
                    // expression (e.g. if-as-expression); skip.
                }
                TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => {
                    depth += 1;
                }
                TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                    depth -= 1;
                    if depth < 0 {
                        return None;
                    }
                }
                TokenKind::Semicolon if depth == 0 => return None,
                TokenKind::Eof => return None,
                _ => {}
            }
            i += 1;
        }
    }

    /// `… VERB ;` — Devanagari SOV statement detector
    /// (closure #266). Hindi / Sanskrit / Marathi grammar is
    /// verb-final, so `मेरा नाम Ryan है` ("my name is Ryan")
    /// reads as "my name Ryan is" with the verb at the end.
    /// The same pattern applies to vāṇी's verb-like
    /// statements: `पुनरागम X;` (return X) reads more
    /// naturally as `X पुनरागम;`. Similarly for `print` →
    /// `लिखो` (Hindi) / `लिहा` (Marathi), `assert` →
    /// `सुनिश्चित` / `खात्री`, `prove` → `सिद्ध` / `प्रमाण`.
    ///
    /// Scans from `self.pos` to the next `;` at depth 0
    /// (tracking parens / brackets / braces). If the token
    /// IMMEDIATELY before that `;` is one of the four verbs,
    /// returns its kind so the dispatcher can route to the
    /// matching SOV parser. Returns None on unbalanced
    /// nesting or no semicolon found.
    fn looks_like_sov_verb_at_end(&self) -> Option<TokenKind> {
        let mut depth: i32 = 0;
        let mut i = self.pos;
        loop {
            let Some(tok) = self.tokens.get(i) else {
                return None;
            };
            match &tok.kind {
                TokenKind::LParen | TokenKind::LBrace | TokenKind::LBracket => {
                    depth += 1;
                }
                TokenKind::RParen | TokenKind::RBrace | TokenKind::RBracket => {
                    depth -= 1;
                    if depth < 0 {
                        return None;
                    }
                }
                TokenKind::Semicolon if depth == 0 => {
                    // Need at least one token before this `;`
                    // AND that token must be a verb-keyword.
                    if i <= self.pos {
                        return None;
                    }
                    let prev = &self.tokens[i - 1];
                    // Tier C SOV-S1 (2026-06-06): added `Let` to
                    // the SOV verb-at-end set. Natural Indo-
                    // Aryan grammar puts `माना` (mānā = "let /
                    // assume") at the end of a binding clause:
                    //   `doubled: i64 = base * 2 माना;`
                    // reads as "doubled, of type i64, equals
                    // base times two, [is] let" with the verb
                    // anchoring the sentence at the end. Hindi
                    // and Marathi use the same shape with their
                    // own forms (`मानो` / `मान`).
                    if matches!(
                        prev.kind,
                        TokenKind::Return
                            | TokenKind::Print
                            | TokenKind::Assert
                            | TokenKind::Prove
                            | TokenKind::Let
                    ) {
                        return Some(prev.kind.clone());
                    }
                    return None;
                }
                TokenKind::Eof => return None,
                _ => {}
            }
            i += 1;
        }
    }

    /// `Parallel IDENT For …` — Devanagari SOV parallel-for
    /// header. Same shape as `looks_like_sov_for` with an
    /// initial `Parallel` keyword.
    fn looks_like_sov_parallel_for(&self) -> bool {
        if !matches!(self.current().kind, TokenKind::Parallel) {
            return false;
        }
        matches!(
            self.tokens.get(self.pos + 1).map(|t| &t.kind),
            Some(TokenKind::Ident(_))
        ) && matches!(
            self.tokens.get(self.pos + 2).map(|t| &t.kind),
            Some(TokenKind::For)
        )
    }

    /// `<ident> (. <ident>)+ =` — a chain of field accesses
    /// followed by an `=`. Used to disambiguate
    /// `p.x = expr;` (field assignment) from a method call.
    /// The chain must end with an ident (not an integer
    /// tuple-index — tuple slots aren't reassignable in v1).
    /// T1.2 phase 2a follow-up.
    fn looks_like_field_assign(&self) -> bool {
        if !matches!(self.current().kind, TokenKind::Ident(_)) {
            return false;
        }
        let mut i = self.pos + 1;
        let mut saw_dot = false;
        loop {
            match self.tokens.get(i).map(|t| &t.kind) {
                Some(TokenKind::Dot) => {
                    saw_dot = true;
                    i += 1;
                    // Next must be an ident (field name)
                    // and not be followed by `(` (that would
                    // be a method call, not a place).
                    if !matches!(
                        self.tokens.get(i).map(|t| &t.kind),
                        Some(TokenKind::Ident(_))
                    ) {
                        return false;
                    }
                    if matches!(
                        self.tokens.get(i + 1).map(|t| &t.kind),
                        Some(TokenKind::LParen)
                    ) {
                        return false;
                    }
                    i += 1;
                }
                Some(TokenKind::Equal) if saw_dot => return true,
                _ => return false,
            }
        }
    }

    fn parse_field_assign_stmt(&mut self) -> Result<Stmt, Diagnostic> {
        // Parse the LHS place expression as a chain of
        // `<ident>(.<ident>)+`. The last `.<ident>` becomes
        // the FieldAssign's field; everything before is the
        // object expression.
        let head_tok = self.expect_ident()?;
        let head_span = head_tok.span;
        let head_name = ident_text(head_tok);
        let mut object = Expr {
            kind: ExprKind::Var(head_name),
            span: head_span,
        };
        // Collect all but the last `.ident` into nested
        // FieldAccess.
        loop {
            // The lookahead above guaranteed at least one
            // `. ident` here.
            self.expect_keyword("'.'", |k| matches!(k, TokenKind::Dot))?;
            let field_tok = self.expect_ident()?;
            let field_span = field_tok.span;
            let field_name = ident_text(field_tok);
            // Is this the final `.field` before `=`? If
            // yes, stop here and emit FieldAssign. Else
            // keep wrapping FieldAccess.
            if matches!(self.current().kind, TokenKind::Equal) {
                self.expect_keyword("'='", |k| matches!(k, TokenKind::Equal))?;
                let value = self.parse_expr()?;
                let semi = self
                    .expect_keyword("';'", |k| matches!(k, TokenKind::Semicolon))?;
                return Ok(Stmt::FieldAssign {
                    object,
                    field: field_name,
                    field_span,
                    value,
                    span: head_span.merge(semi.span),
                });
            }
            object = Expr {
                kind: ExprKind::FieldAccess {
                    object: Box::new(object),
                    field: field_name,
                },
                span: head_span.merge(field_span),
            };
        }
    }

    fn looks_like_index_assign(&self) -> bool {
        if !matches!(self.current().kind, TokenKind::Ident(_)) {
            return false;
        }
        // Scan past a single `[ ... ]` (matching brackets) and check for `=`.
        let mut i = self.pos + 1;
        if !matches!(self.tokens.get(i).map(|t| &t.kind), Some(TokenKind::LBracket)) {
            return false;
        }
        let mut depth: i32 = 1;
        i += 1;
        while let Some(tok) = self.tokens.get(i) {
            match tok.kind {
                TokenKind::LBracket => depth += 1,
                TokenKind::RBracket => {
                    depth -= 1;
                    if depth == 0 {
                        i += 1;
                        break;
                    }
                }
                TokenKind::Eof => return false,
                _ => {}
            }
            i += 1;
        }
        matches!(self.tokens.get(i).map(|t| &t.kind), Some(TokenKind::Equal))
    }

    fn parse_index_assign_stmt(&mut self) -> Result<Stmt, Diagnostic> {
        let name_token = self.expect_ident()?;
        let name_span = name_token.span;
        let name = ident_text(name_token);
        self.expect_keyword("'['", |kind| matches!(kind, TokenKind::LBracket))?;
        let index = self.parse_expr()?;
        self.expect_keyword("']'", |kind| matches!(kind, TokenKind::RBracket))?;
        self.expect_keyword("'='", |kind| matches!(kind, TokenKind::Equal))?;
        let value = self.parse_expr()?;
        let semi = self.expect_keyword("';'", |kind| matches!(kind, TokenKind::Semicolon))?;
        Ok(Stmt::IndexAssign {
            name,
            index,
            field_path: Vec::new(),
            value,
            span: name_span.merge(semi.span),
        })
    }

    /// Parse `<ident>[<index>].<field>(.<field>)* = <expr>;`
    /// into `Stmt::IndexAssign` with a non-empty `field_path`.
    /// The lookahead in `looks_like_index_then_field_assign`
    /// has already validated the surface shape; this just
    /// rebuilds the AST nodes. T1.2 phase 2b follow-up.
    fn parse_index_then_field_assign_stmt(&mut self) -> Result<Stmt, Diagnostic> {
        let name_token = self.expect_ident()?;
        let name_span = name_token.span;
        let name = ident_text(name_token);
        self.expect_keyword("'['", |kind| matches!(kind, TokenKind::LBracket))?;
        let index = self.parse_expr()?;
        self.expect_keyword("']'", |kind| matches!(kind, TokenKind::RBracket))?;
        // Parse one-or-more `.<field>` segments.
        let mut field_path: Vec<String> = Vec::new();
        while self
            .match_token(|k| matches!(k, TokenKind::Dot))
            .is_some()
        {
            let field_tok = self.expect_ident()?;
            field_path.push(ident_text(field_tok));
            if !matches!(
                self.current().kind,
                TokenKind::Dot | TokenKind::Equal
            ) {
                return Err(self.error_here(
                    "expected '.<field>' or '=' after indexed field-access",
                ));
            }
        }
        self.expect_keyword("'='", |kind| matches!(kind, TokenKind::Equal))?;
        let value = self.parse_expr()?;
        let semi = self.expect_keyword("';'", |kind| matches!(kind, TokenKind::Semicolon))?;
        Ok(Stmt::IndexAssign {
            name,
            index,
            field_path,
            value,
            span: name_span.merge(semi.span),
        })
    }

    fn parse_assign_stmt(&mut self) -> Result<Stmt, Diagnostic> {
        let name_token = self.expect_ident()?;
        let name_span = name_token.span;
        let name = ident_text(name_token);
        self.expect_keyword("'='", |kind| matches!(kind, TokenKind::Equal))?;
        let expr = self.parse_expr()?;
        let semi = self.expect_keyword("';'", |kind| matches!(kind, TokenKind::Semicolon))?;
        Ok(Stmt::Assign {
            name,
            expr,
            span: name_span.merge(semi.span),
        })
    }

    fn parse_if_stmt(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect_keyword("'if'", |kind| matches!(kind, TokenKind::If))?;
        let cond = self.parse_expr()?;
        let then_body = self.parse_block()?;
        let (else_body, end_span) = if self
            .match_token(|kind| matches!(kind, TokenKind::Else))
            .is_some()
        {
            if self.check(|kind| matches!(kind, TokenKind::If)) {
                // else-if: re-parse as a nested if statement inside a one-statement else block.
                let inner = self.parse_if_stmt()?;
                let span = inner.span();
                (vec![inner], span)
            } else {
                let stmts = self.parse_block()?;
                let span = stmts
                    .last()
                    .map(|s| s.span())
                    .unwrap_or(start.span);
                (stmts, span)
            }
        } else {
            (Vec::new(), then_body.last().map(|s| s.span()).unwrap_or(start.span))
        };
        Ok(Stmt::If {
            cond,
            then_body,
            else_body,
            span: start.span.merge(end_span),
        })
    }

    fn parse_for_stmt(&mut self) -> Result<Stmt, Diagnostic> {
        self.parse_for_stmt_inner(false)
    }

    fn parse_task_spawn_stmt(&mut self) -> Result<Stmt, Diagnostic> {
        let task_tok = self.expect_keyword("'task'", |kind| matches!(kind, TokenKind::Task))?;
        let name_tok = self.expect_ident()?;
        let name = ident_text(name_tok);
        self.expect_keyword("'{'", |kind| matches!(kind, TokenKind::LBrace))?;
        let mut body = Vec::new();
        while !self.check(|kind| matches!(kind, TokenKind::RBrace | TokenKind::Eof)) {
            body.push(self.parse_stmt()?);
        }
        let close = self.expect_keyword("'}'", |kind| matches!(kind, TokenKind::RBrace))?;
        Ok(Stmt::TaskSpawn {
            name,
            body,
            span: task_tok.span.merge(close.span),
        })
    }

    /// `region <name> { <body> }` — Layer 5 of `unsafe.md`.
    /// Sugar that desugars at parse time to:
    /// ```ignore
    /// {                                 // bare block (fresh scope)
    ///   let <name>: Region = region_new();
    ///   <body>
    ///   // Region's scope-exit drop frees the arena here.
    /// }
    /// ```
    /// The bare-block scoping (already supported by the parser
    /// via `{ ... }`) handles binding visibility and the affine
    /// drop emission for Region. No new AST node is needed; the
    /// existing scope-exit drop machinery does the work.
    ///
    /// ArenaRefs derived from `<name>` via `region_borrow_i64`
    /// are local-origin and cannot escape the block — enforced
    /// by Layer 1.2's no-escape dataflow, already extended to
    /// cover `Type::ArenaRef` in the previous commit.
    fn parse_region_block_stmt(&mut self) -> Result<Stmt, Diagnostic> {
        let region_tok =
            self.expect_keyword("'region'", |k| matches!(k, TokenKind::RegionKw))?;
        let name_tok = self.expect_ident()?;
        let name = ident_text(name_tok);
        self.expect_keyword("'{'", |k| matches!(k, TokenKind::LBrace))?;
        let mut body_stmts: Vec<Stmt> = Vec::new();
        // First statement: `let <name>: Region = region_new();`
        // Built by hand so it shares the region_tok span (so
        // diagnostics point at the `region` keyword for any
        // issue with the synthetic Let).
        body_stmts.push(Stmt::Let {
            name: name.clone(),
            annotation: Some(Type::Region),
            expr: Expr {
                kind: ExprKind::Call {
                    name: "region_new".to_string(),
                    name_span: region_tok.span,
                    args: Vec::new(),
                },
                span: region_tok.span,
            },
            span: region_tok.span,
        });
        // User-written body statements.
        while !self.check(|k| matches!(k, TokenKind::RBrace | TokenKind::Eof)) {
            body_stmts.push(self.parse_stmt()?);
        }
        let close = self.expect_keyword("'}'", |k| matches!(k, TokenKind::RBrace))?;
        let full_span = region_tok.span.merge(close.span);
        // Wrap in `if true { ... }` — same desugaring the
        // parser already uses for bare `{ ... }` statement
        // blocks at `parse_stmt` line ~1894. This gives us
        // a fresh scope so the synthetic Let's binding goes
        // out of scope at the block's `}`, firing the Region
        // drop.
        Ok(Stmt::If {
            cond: Expr {
                kind: ExprKind::Bool(true),
                span: region_tok.span,
            },
            then_body: body_stmts,
            else_body: Vec::new(),
            span: full_span,
        })
    }

    fn parse_task_join_stmt(&mut self) -> Result<Stmt, Diagnostic> {
        let join_tok = self.expect_keyword("'join'", |kind| matches!(kind, TokenKind::Join))?;
        let name_tok = self.expect_ident()?;
        let name = ident_text(name_tok);
        let semi = self.expect_keyword("';'", |kind| matches!(kind, TokenKind::Semicolon))?;
        Ok(Stmt::TaskJoin {
            name,
            span: join_tok.span.merge(semi.span),
        })
    }

    /// `unsafe(reason = "...") { <body> }` — Layer 1.1 of the
    /// embedded-vāṇी unsafe plan (`unsafe.md`). The `reason`
    /// clause is mandatory at parse time. Empty / >256-char /
    /// non-ASCII-printable / newline-containing reason strings
    /// are all parse errors.
    ///
    /// Why these rules (kept terse here; full rationale lives in
    /// `unsafe.md` § "Reason-string rules (v1)"):
    /// - Non-empty: a missing justification defeats the whole
    ///   point of the in-syntax form.
    /// - ≤256 chars: keeps the deviation-record artifact compact
    ///   and discoverable; certification reviewers cluster by
    ///   prefix, not by paragraph.
    /// - ASCII-printable + no newlines: the reason flows through
    ///   IR / DWARF metadata where multi-line / non-printable
    ///   payloads bloat the artifact and break greppability.
    fn parse_unsafe_block_stmt(&mut self) -> Result<Stmt, Diagnostic> {
        let unsafe_tok =
            self.expect_keyword("'unsafe'", |kind| matches!(kind, TokenKind::Unsafe))?;
        // Require the `(reason = "...")` clause. The mandatory-
        // clause form is what makes the deviation-record extraction
        // possible — a bare `unsafe { … }` would silently slip past
        // the audit trail.
        if !self.check(|k| matches!(k, TokenKind::LParen)) {
            return Err(Diagnostic::new(
                self.current().span,
                "`unsafe` requires a `(reason = \"…\")` clause — the \
                 justification is part of the syntax and is emitted as \
                 machine-readable deviation metadata. Example: \
                 `unsafe(reason = \"MMIO: GPIOA::ODR write\") { … }`",
            ));
        }
        self.expect_keyword("'('", |k| matches!(k, TokenKind::LParen))?;
        // Single-keyword `reason` identifier inside the clause.
        // No other keys accepted in v1; future revisions can add
        // structured keys (e.g. `audit_id = "…"`) without breaking
        // existing call sites.
        let reason_ident = self.expect_ident()?;
        let reason_kw = ident_text(reason_ident.clone());
        if reason_kw != "reason" {
            return Err(Diagnostic::new(
                reason_ident.span,
                format!(
                    "expected `reason` inside `unsafe(...)`, got `{}`",
                    reason_kw
                ),
            ));
        }
        self.expect_keyword("'='", |k| matches!(k, TokenKind::Equal))?;
        let reason_tok = self.expect_string()?;
        let reason_span = reason_tok.span;
        let TokenKind::Str(reason) = reason_tok.kind else {
            unreachable!("expect_string only returns string tokens")
        };
        // Reason-string validation. Each rule emits a precise
        // diagnostic so the user sees exactly which constraint
        // failed — important when the source string came from a
        // template or a copy-paste from another file.
        if reason.is_empty() {
            return Err(Diagnostic::new(
                reason_span,
                "`reason` cannot be empty — the deviation-record artifact \
                 needs a non-trivial justification per `unsafe` block. \
                 Recommended prefixes: \"MMIO: …\", \"FFI: …\", \"DMA: …\", \
                 \"transmute: …\", \"vendor-SDK: …\"",
            ));
        }
        if reason.len() > 256 {
            return Err(Diagnostic::new(
                reason_span,
                format!(
                    "`reason` is {} chars (max 256). Keep the in-syntax \
                     reason short; expand it in a nearby `//` comment if \
                     more context is needed.",
                    reason.len()
                ),
            ));
        }
        if reason.contains('\n') || reason.contains('\r') {
            return Err(Diagnostic::new(
                reason_span,
                "`reason` cannot contain newlines — multi-line reasons \
                 don't survive IR / DWARF metadata round-trip and break \
                 deviation-record extraction tooling",
            ));
        }
        if let Some(bad) = reason.chars().find(|c| {
            // Reject control chars (incl. tab) and any non-ASCII.
            // Printable ASCII range: 0x20..=0x7E.
            (*c as u32) < 0x20 || (*c as u32) > 0x7E
        }) {
            return Err(Diagnostic::new(
                reason_span,
                format!(
                    "`reason` contains a non-ASCII-printable character \
                     (U+{:04X}). Deviation-record artifacts require \
                     printable ASCII so they grep cleanly across \
                     toolchains.",
                    bad as u32
                ),
            ));
        }
        self.expect_keyword("')'", |k| matches!(k, TokenKind::RParen))?;
        // Body — same shape as `task <name> { … }` and `if true
        // { … }`. The block's stmts run in a fresh inner scope.
        self.expect_keyword("'{'", |k| matches!(k, TokenKind::LBrace))?;
        let mut body = Vec::new();
        while !self.check(|k| matches!(k, TokenKind::RBrace | TokenKind::Eof)) {
            body.push(self.parse_stmt()?);
        }
        let close = self.expect_keyword("'}'", |k| matches!(k, TokenKind::RBrace))?;
        Ok(Stmt::UnsafeBlock {
            reason,
            reason_span,
            body,
            span: unsafe_tok.span.merge(close.span),
        })
    }

    fn parse_parallel_for_stmt(&mut self) -> Result<Stmt, Diagnostic> {
        // `parallel` was just bumped by the caller. Only the range
        // form supports the parallel marker — iter-style `for x in
        // xs` consumes the collection (which can't be raced over).
        self.parse_for_stmt_inner(true)
    }

    fn parse_for_stmt_inner(&mut self, parallel: bool) -> Result<Stmt, Diagnostic> {
        let start_tok =
            self.expect_keyword("'for'", |kind| matches!(kind, TokenKind::For))?;
        let var_tok = self.expect_ident()?;
        let var = ident_text(var_tok);
        // The two `for` shapes are now disambiguated by the
        // post-counter keyword:
        //   `for VAR in EXPR { ... }`           → collection-iter
        //                                          (consuming or
        //                                           borrowing — `ref EXPR`)
        //   `for VAR from LO to HI { ... }`     → range form
        // Refines T0.0. The prior `0..n` range shape is gone.
        if self.match_token(|k| matches!(k, TokenKind::In)).is_some() {
            // Borrowing form: `for x in ref xs { ... }`. The old
            // `for x in &xs { ... }` shape is gone — surface a
            // friendly hint if encountered.
            if matches!(self.current().kind, TokenKind::Amp) {
                let span = self.current().span;
                return Err(Diagnostic::new(
                    span,
                    "use `for VAR in ref XS { … }` to iterate by borrow (T0.0)",
                ));
            }
            let consumes = !self
                .match_token(|k| matches!(k, TokenKind::Ref))
                .is_some();
            if parallel {
                return Err(Diagnostic::new(
                    start_tok.span,
                    "'parallel' is only valid on a range-form for loop",
                ));
            }
            // Phase 11 (2026-06-07): accept a dotted path
            // (`obj.field`, `obj.f1.f2`) as the collection, not
            // just a bare identifier. The path is stored as a
            // single dot-separated string; the checker resolves
            // the head + walks the field chain to land on the
            // iterable type, and `emit_for_iter` recognizes the
            // dot to emit struct-field access. Lifts L7 from
            // docs/v1_limitations.md.
            let collection_tok = self.expect_ident()?;
            let mut collection = ident_text(collection_tok);
            while self.match_token(|k| matches!(k, TokenKind::Dot)).is_some() {
                let field_tok = self.expect_ident()?;
                collection.push('.');
                collection.push_str(&ident_text(field_tok));
            }
            let body = self.parse_block()?;
            let end_span = body
                .last()
                .map(|s| s.span())
                .unwrap_or(start_tok.span);
            return Ok(Stmt::ForIter {
                label: None,
                var,
                collection,
                consumes,
                body,
                span: start_tok.span.merge(end_span),
            });
        }
        // Range form: `for i from LO to HI invariant ...; { body }`.
        // The lower bound expression follows `from`, the upper
        // follows `to`. Refines T0.0; was `for i in LO..HI`.
        self.expect_keyword(
            "'from' (range form) or 'in' (collection-iter)",
            |k| matches!(k, TokenKind::From),
        )?;
        let start = self.parse_expr()?;
        self.expect_keyword("'to'", |k| matches!(k, TokenKind::To))?;
        let end = self.parse_expr()?;
        let invariants = self.parse_invariants()?;
        let reductions = self.parse_reductions()?;
        if !reductions.is_empty() && !parallel {
            return Err(Diagnostic::new(
                reductions[0].span,
                "'reduce' clauses are only valid on a `parallel for` loop",
            ));
        }
        let body = self.parse_block()?;
        let end_span = body.last().map(|s| s.span()).unwrap_or(start_tok.span);
        Ok(Stmt::For {
            label: None,
            var,
            start,
            end,
            invariants,
            body,
            span: start_tok.span.merge(end_span),
            parallel,
            reductions,
        })
    }

    /// Parse a Devanagari SOV-style range `for` header (closure
    /// #265):
    ///
    ///     IDENT 'के लिए' START 'से' END 'तक' [invariants]
    ///     [reductions] { body }
    ///
    /// Both `के लिए` and `से` / `तक` are already lexed as
    /// `TokenKind::For` / `From` / `To` via the existing
    /// Devanagari alias tables. This parser only swaps the
    /// POSITIONS: variable first, then `for`-postposition,
    /// then operands followed by their postpositions. AST shape
    /// produced is identical to the English form, so downstream
    /// passes (checker, SSA, backends) see no difference.
    ///
    /// The `parallel` flag is set by the caller after consuming
    /// a leading `Parallel` keyword (Hindi: `समान्तर`).
    fn parse_sov_for_stmt(&mut self, parallel: bool) -> Result<Stmt, Diagnostic> {
        let var_tok = self.expect_ident()?;
        let var_span = var_tok.span;
        let var = ident_text(var_tok);
        let for_tok = self.expect_keyword(
            "'for' / 'के लिए' postposition after the loop variable",
            |k| matches!(k, TokenKind::For),
        )?;
        let start_expr = self.parse_expr()?;
        self.expect_keyword(
            "'from' / 'से' postposition after the start value",
            |k| matches!(k, TokenKind::From),
        )?;
        let end_expr = self.parse_expr()?;
        self.expect_keyword(
            "'to' / 'तक' postposition after the end value",
            |k| matches!(k, TokenKind::To),
        )?;
        let invariants = self.parse_invariants()?;
        let reductions = self.parse_reductions()?;
        if !reductions.is_empty() && !parallel {
            return Err(Diagnostic::new(
                reductions[0].span,
                "'reduce' clauses are only valid on a `parallel for` loop",
            ));
        }
        let body = self.parse_block()?;
        let end_span = body
            .last()
            .map(|s| s.span())
            .unwrap_or(for_tok.span);
        Ok(Stmt::For {
            label: None,
            var,
            start: start_expr,
            end: end_expr,
            invariants,
            body,
            span: var_span.merge(end_span),
            parallel,
            reductions,
        })
    }

    fn parse_reductions(&mut self) -> Result<Vec<Reduction>, Diagnostic> {
        let mut out = Vec::new();
        while let Some(start) = self.match_token(|kind| matches!(kind, TokenKind::Reduce)) {
            let var_tok = self.expect_ident()?;
            let var = ident_text(var_tok);
            self.expect_keyword("'with'", |kind| matches!(kind, TokenKind::With))?;
            // Reduction operator — currently `+` only. Other
            // associative ops (`*`, min, max) are an easy follow-on
            // once we have richer operator-symbol parsing for
            // non-Binary positions.
            let op_tok = self.current().clone();
            let op = match op_tok.kind {
                TokenKind::Plus => {
                    self.bump();
                    ReductionOp::Add
                }
                TokenKind::Star => {
                    self.bump();
                    ReductionOp::Mul
                }
                TokenKind::AndAnd => {
                    self.bump();
                    ReductionOp::And
                }
                TokenKind::OrOr => {
                    self.bump();
                    ReductionOp::Or
                }
                TokenKind::Amp => {
                    self.bump();
                    ReductionOp::BitAnd
                }
                TokenKind::Pipe => {
                    self.bump();
                    ReductionOp::BitOr
                }
                TokenKind::Caret => {
                    self.bump();
                    ReductionOp::BitXor
                }
                // `min` and `max` are context-sensitive
                // identifiers (not reserved keywords) — match
                // them by literal text so users can declare
                // struct fields / locals with those names
                // outside this clause.
                TokenKind::Ident(ref n) if n == "min" => {
                    self.bump();
                    ReductionOp::Min
                }
                TokenKind::Ident(ref n) if n == "max" => {
                    self.bump();
                    ReductionOp::Max
                }
                _ => {
                    return Err(Diagnostic::new(
                        op_tok.span,
                        "expected reduction operator (one of `+`, `*`, `&&`, `||`, `&`, `|`, `^`, `min`, `max`)",
                    ));
                }
            };
            let semi = self.expect_keyword("';'", |kind| matches!(kind, TokenKind::Semicolon))?;
            out.push(Reduction {
                var,
                op,
                span: start.span.merge(semi.span),
            });
        }
        Ok(out)
    }

    fn parse_invariants(&mut self) -> Result<Vec<Expr>, Diagnostic> {
        let mut invariants = Vec::new();
        while self
            .match_token(|kind| matches!(kind, TokenKind::Invariant))
            .is_some()
        {
            let expr = self.parse_expr()?;
            self.expect_keyword("';'", |kind| matches!(kind, TokenKind::Semicolon))?;
            invariants.push(expr);
        }
        Ok(invariants)
    }

    fn parse_labeled_loop_stmt(&mut self) -> Result<Stmt, Diagnostic> {
        let label_tok = self.bump(); // consume the Label token
        let label_name = if let TokenKind::Label(name) = label_tok.kind {
            name
        } else {
            unreachable!()
        };
        self.expect_keyword("':' after loop label", |kind| matches!(kind, TokenKind::Colon))?;
        if self.check(|kind| matches!(kind, TokenKind::While)) {
            let mut stmt = self.parse_while_stmt()?;
            if let Stmt::While { ref mut label, .. } = stmt {
                *label = Some(label_name);
            }
            Ok(stmt)
        } else if self.check(|kind| matches!(kind, TokenKind::For)) {
            let mut stmt = self.parse_for_stmt()?;
            match stmt {
                Stmt::For { ref mut label, .. } | Stmt::ForIter { ref mut label, .. } => {
                    *label = Some(label_name);
                }
                _ => {}
            }
            Ok(stmt)
        } else {
            Err(Diagnostic::new(
                label_tok.span,
                "loop label must be followed by 'while' or 'for'",
            ))
        }
    }

    fn parse_while_stmt(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect_keyword("'while'", |kind| matches!(kind, TokenKind::While))?;
        let cond = self.parse_expr()?;
        let invariants = self.parse_invariants()?;
        let body = self.parse_block()?;
        let end_span = body.last().map(|s| s.span()).unwrap_or(start.span);
        Ok(Stmt::While {
            label: None,
            cond,
            invariants,
            body,
            span: start.span.merge(end_span),
        })
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, Diagnostic> {
        self.expect_keyword("'{'", |kind| matches!(kind, TokenKind::LBrace))?;
        let mut body = Vec::new();
        while !self.check(|kind| matches!(kind, TokenKind::RBrace | TokenKind::Eof)) {
            match self.parse_stmt() {
                Ok(s) => body.push(s),
                Err(e) => {
                    self.errors.push(e);
                    self.sync_to_stmt();
                }
            }
        }
        self.expect_keyword("'}'", |kind| matches!(kind, TokenKind::RBrace))?;
        Ok(body)
    }

    fn parse_let_stmt(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect_keyword("'let'", |kind| matches!(kind, TokenKind::Let))?;
        // Destructure form: `let (a, b, …) = expr;` —
        // produces `Stmt::LetTuple`. The checker desugars to
        // a sequence of `Let`s under the hood. T1.1.
        if self.check(|k| matches!(k, TokenKind::LParen)) {
            self.bump();
            let mut names = Vec::new();
            loop {
                let tok = self.expect_ident()?;
                names.push(ident_text(tok));
                if self
                    .match_token(|k| matches!(k, TokenKind::Comma))
                    .is_none()
                {
                    break;
                }
                if self.check(|k| matches!(k, TokenKind::RParen)) {
                    break;
                }
            }
            self.expect_keyword("')'", |k| matches!(k, TokenKind::RParen))?;
            let annotation = if self
                .match_token(|k| matches!(k, TokenKind::Colon))
                .is_some()
            {
                Some(self.parse_type()?)
            } else {
                None
            };
            self.expect_keyword("'='", |k| matches!(k, TokenKind::Equal))?;
            let expr = self.parse_expr()?;
            let semi = self.expect_keyword("';'", |k| matches!(k, TokenKind::Semicolon))?;
            if names.len() < 2 {
                return Err(Diagnostic::new(
                    start.span.merge(semi.span),
                    "destructure-let needs at least two names; use plain `let` for single bindings",
                ));
            }
            return Ok(Stmt::LetTuple {
                names,
                annotation,
                expr,
                span: start.span.merge(semi.span),
            });
        }
        let name_token = self.expect_ident()?;
        let name = ident_text(name_token);
        let annotation = if self
            .match_token(|kind| matches!(kind, TokenKind::Colon))
            .is_some()
        {
            Some(self.parse_type()?)
        } else {
            None
        };
        self.expect_keyword("'='", |kind| matches!(kind, TokenKind::Equal))?;
        let expr = self.parse_expr()?;
        let semi = self.expect_keyword("';'", |kind| matches!(kind, TokenKind::Semicolon))?;

        Ok(Stmt::Let {
            name,
            annotation,
            expr,
            span: start.span.merge(semi.span),
        })
    }

    fn parse_return_stmt(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect_keyword("'return'", |kind| matches!(kind, TokenKind::Return))?;
        let expr = self.parse_expr()?;
        let semi = self.expect_keyword("';'", |kind| matches!(kind, TokenKind::Semicolon))?;
        Ok(Stmt::Return {
            expr,
            span: start.span.merge(semi.span),
        })
    }

    fn parse_assert_stmt(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect_keyword("'assert'", |kind| matches!(kind, TokenKind::Assert))?;
        let expr = self.parse_expr()?;
        // Optional `, "message"` between the condition and the semicolon.
        let message = if self
            .match_token(|kind| matches!(kind, TokenKind::Comma))
            .is_some()
        {
            let msg_token = self.expect_string()?;
            let TokenKind::Str(s) = msg_token.kind else {
                unreachable!("expect_string only returns Str tokens")
            };
            Some(s)
        } else {
            None
        };
        let semi = self.expect_keyword("';'", |kind| matches!(kind, TokenKind::Semicolon))?;
        Ok(Stmt::Assert {
            expr,
            message,
            span: start.span.merge(semi.span),
        })
    }

    fn parse_prove_stmt(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect_keyword("'prove'", |kind| matches!(kind, TokenKind::Prove))?;
        let expr = self.parse_expr()?;
        let semi = self.expect_keyword("';'", |kind| matches!(kind, TokenKind::Semicolon))?;
        Ok(Stmt::Prove {
            expr,
            span: start.span.merge(semi.span),
        })
    }

    fn parse_print_stmt(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect_keyword("'print'", |kind| matches!(kind, TokenKind::Print))?;
        // Comma-separated items: each is a string literal or an
        // expression. `print "x =", x, "(done)";` is legal.
        let mut items = Vec::new();
        loop {
            items.push(self.parse_print_item()?);
            if self
                .match_token(|kind| matches!(kind, TokenKind::Comma))
                .is_some()
            {
                continue;
            }
            break;
        }
        let semi = self.expect_keyword("';'", |kind| matches!(kind, TokenKind::Semicolon))?;
        Ok(Stmt::Print {
            items,
            span: start.span.merge(semi.span),
        })
    }

    /// SOV-form verb-at-end statement dispatch (closure #266).
    /// `looks_like_sov_verb_at_end` has already scanned ahead
    /// and identified which verb closes this statement; we
    /// route to the matching parser. All four AST shapes are
    /// identical to their English counterparts — the only
    /// thing that changed is the surface order.
    /// Tier C SOV-S3/S4 (2026-06-06): parse a SOV block-form
    /// statement (`if`/`while` with the verb between the cond
    /// and the body). The AST shape produced is identical to
    /// the keyword-first form's, so downstream passes don't need
    /// to know which surface was used.
    fn parse_sov_block_stmt(&mut self, verb: TokenKind) -> Result<Stmt, Diagnostic> {
        let start_span = self.current().span;
        // Parse the cond expression (parse_expr stops at the
        // verb since If/While/Match aren't binary operators).
        let cond = self.parse_expr()?;
        // Consume the verb keyword.
        let verb_tok = self.bump();
        match verb {
            TokenKind::If => {
                self.expect_keyword(
                    "'{' (SOV-if then-block)",
                    |k| matches!(k, TokenKind::LBrace),
                )?;
                let mut then_body = Vec::new();
                while !self.check(|k| matches!(k, TokenKind::RBrace | TokenKind::Eof)) {
                    then_body.push(self.parse_stmt()?);
                }
                let then_close = self.expect_keyword(
                    "'}' (SOV-if then-block)",
                    |k| matches!(k, TokenKind::RBrace),
                )?;
                let (else_body, end_span) = if self
                    .match_token(|k| matches!(k, TokenKind::Else))
                    .is_some()
                {
                    // `else { … }` block. We don't support
                    // `else if` chaining in SOV form (the
                    // keyword-first form should be used for
                    // chains; the SOV detector only fires on
                    // the outer-most cond + verb pair).
                    self.expect_keyword(
                        "'{' (SOV-if else-block)",
                        |k| matches!(k, TokenKind::LBrace),
                    )?;
                    let mut else_body = Vec::new();
                    while !self.check(|k| matches!(k, TokenKind::RBrace | TokenKind::Eof)) {
                        else_body.push(self.parse_stmt()?);
                    }
                    let else_close = self.expect_keyword(
                        "'}' (SOV-if else-block)",
                        |k| matches!(k, TokenKind::RBrace),
                    )?;
                    (else_body, else_close.span)
                } else {
                    (vec![], then_close.span)
                };
                Ok(Stmt::If {
                    cond,
                    then_body,
                    else_body,
                    span: start_span.merge(end_span),
                })
            }
            TokenKind::While => {
                self.expect_keyword(
                    "'{' (SOV-while body)",
                    |k| matches!(k, TokenKind::LBrace),
                )?;
                let mut body = Vec::new();
                while !self.check(|k| matches!(k, TokenKind::RBrace | TokenKind::Eof)) {
                    body.push(self.parse_stmt()?);
                }
                let close = self.expect_keyword(
                    "'}' (SOV-while body)",
                    |k| matches!(k, TokenKind::RBrace),
                )?;
                Ok(Stmt::While {
                    label: None,
                    cond,
                    invariants: vec![],
                    body,
                    span: start_span.merge(close.span),
                })
            }
            // `match` at statement position is uncommon (vāṇी
            // match is an expression). The SOV-match shape is
            // implicitly available through SOV-let:
            //   `let r: i64 = scrutinee match { ... } माना;`
            // wraps a match-expression in a SOV-let. The
            // detector reaches here only when match is at
            // statement position WITHOUT an enclosing let, which
            // is unusual; report a clear diagnostic.
            TokenKind::Match => Err(Diagnostic::new(
                verb_tok.span,
                "SOV-match at statement position is unsupported. Match is an expression in vāṇी; wrap it in a SOV-let (`let r: T = scrutinee match { ... } माना;`) or use the keyword-first form (`match scrutinee { ... }`).".to_string(),
            )),
            other => Err(Diagnostic::new(
                verb_tok.span,
                format!(
                    "internal: unexpected SOV block verb {:?} — \
                     expected If / While / Match",
                    other
                ),
            )),
        }
    }

    fn parse_sov_verb_stmt(&mut self, verb: TokenKind) -> Result<Stmt, Diagnostic> {
        let start_span = self.current().span;
        match verb {
            TokenKind::Return => {
                let expr = self.parse_expr()?;
                self.bump(); // consume Return
                let semi = self.expect_keyword("';'", |k| matches!(k, TokenKind::Semicolon))?;
                Ok(Stmt::Return {
                    expr,
                    span: start_span.merge(semi.span),
                })
            }
            // Tier C SOV-S1 (2026-06-06): `<name>[: <type>] = <expr> माना;`
            // — Sanskrit / Hindi / Marathi natural shape for a
            // let-binding. The variable name + optional type
            // annotation + initializer expression all appear
            // before the verb `माना` (mānā) which closes the
            // sentence. Identifier-of-name is the first token;
            // the existing parse_let_binding handles
            // <type> + = + <expr>, so reuse it after bumping
            // past the leading IDENT and faking a keyword-first
            // shape, then consume the trailing Let verb.
            TokenKind::Let => {
                let name_tok = self.expect_ident()?;
                let name = ident_text(name_tok);
                let annotation = if self.match_token(|k| matches!(k, TokenKind::Colon)).is_some() {
                    Some(self.parse_type()?)
                } else {
                    None
                };
                self.expect_keyword("'=' (SOV let needs an initializer)",
                    |k| matches!(k, TokenKind::Equal))?;
                let expr = self.parse_expr()?;
                self.bump(); // consume Let verb
                let semi = self.expect_keyword("';'", |k| matches!(k, TokenKind::Semicolon))?;
                Ok(Stmt::Let {
                    name,
                    annotation,
                    expr,
                    span: start_span.merge(semi.span),
                })
            }
            TokenKind::Prove => {
                let expr = self.parse_expr()?;
                self.bump(); // consume Prove
                let semi = self.expect_keyword("';'", |k| matches!(k, TokenKind::Semicolon))?;
                Ok(Stmt::Prove {
                    expr,
                    span: start_span.merge(semi.span),
                })
            }
            TokenKind::Assert => {
                let expr = self.parse_expr()?;
                let message = if self
                    .match_token(|k| matches!(k, TokenKind::Comma))
                    .is_some()
                {
                    let msg_tok = self.expect_string()?;
                    let TokenKind::Str(s) = msg_tok.kind else {
                        unreachable!("expect_string only returns Str tokens")
                    };
                    Some(s)
                } else {
                    None
                };
                self.bump(); // consume Assert
                let semi = self.expect_keyword("';'", |k| matches!(k, TokenKind::Semicolon))?;
                Ok(Stmt::Assert {
                    expr,
                    message,
                    span: start_span.merge(semi.span),
                })
            }
            TokenKind::Print => {
                let mut items = Vec::new();
                loop {
                    items.push(self.parse_print_item()?);
                    // Stop on Print keyword (the verb-at-end) so
                    // the surrounding loop doesn't consume it.
                    if matches!(self.current().kind, TokenKind::Print) {
                        break;
                    }
                    if self
                        .match_token(|k| matches!(k, TokenKind::Comma))
                        .is_some()
                    {
                        continue;
                    }
                    break;
                }
                self.bump(); // consume Print
                let semi = self.expect_keyword("';'", |k| matches!(k, TokenKind::Semicolon))?;
                Ok(Stmt::Print {
                    items,
                    span: start_span.merge(semi.span),
                })
            }
            other => {
                // Shouldn't reach — looks_like_sov_verb_at_end
                // only returns the four supported verbs.
                Err(Diagnostic::new(
                    start_span,
                    format!(
                        "internal: unexpected SOV verb {:?} — expected \
                         Return / Print / Assert / Prove",
                        other
                    ),
                ))
            }
        }
    }

    fn parse_print_item(&mut self) -> Result<crate::ast::PrintItem, Diagnostic> {
        use crate::ast::PrintItem;
        if let TokenKind::Str(_) = &self.current().kind {
            let tok = self.bump();
            match tok.kind {
                TokenKind::Str(s) => Ok(PrintItem::Str(s)),
                _ => unreachable!(),
            }
        } else {
            let expr = self.parse_expr()?;
            Ok(PrintItem::Expr(expr))
        }
    }

    fn parse_expr(&mut self) -> Result<Expr, Diagnostic> {
        self.parse_binary_expr(1)
    }

    fn parse_binary_expr(&mut self, min_precedence: u8) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_unary_expr()?;

        while let Some((op, precedence)) = self.current_binary_op() {
            if precedence < min_precedence {
                break;
            }

            self.bump();
            let right = self.parse_binary_expr(precedence + 1)?;
            let span = left.span.merge(right.span);
            left = Expr {
                kind: ExprKind::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            };
        }

        Ok(left)
    }

    fn parse_unary_expr(&mut self) -> Result<Expr, Diagnostic> {
        // Borrow expressions: `ref x` (immutable) and
        // `mut ref x` (mutable). The old `&x` / `&mut x`
        // prefix is gone — a friendly diagnostic points at
        // the new shape. Refines T0.0.
        let mut_then_ref = self.check(|k| matches!(k, TokenKind::Mut))
            && matches!(
                self.tokens.get(self.pos + 1).map(|t| &t.kind),
                Some(TokenKind::Ref)
            );
        if mut_then_ref {
            let mut_tok = self.bump();
            self.bump(); // ref
            let inner = self.parse_unary_expr()?;
            let span = mut_tok.span.merge(inner.span);
            return Ok(Expr {
                kind: ExprKind::RefMut {
                    inner: Box::new(inner),
                },
                span,
            });
        }
        if let Some(token) = self.match_token(|kind| matches!(kind, TokenKind::Ref)) {
            let inner = self.parse_unary_expr()?;
            let span = token.span.merge(inner.span);
            return Ok(Expr {
                kind: ExprKind::Ref {
                    inner: Box::new(inner),
                },
                span,
            });
        }
        // Old `&` prefix borrow — surface a guidance error.
        // We can't accept it here because the parser still
        // needs `&` available as the bitwise-AND binary op.
        if let Some(token) = self.match_token(|kind| matches!(kind, TokenKind::Amp)) {
            return Err(Diagnostic::new(
                token.span,
                "use `ref x` (or `mut ref x`) instead of `&x` for borrows (T0.0)",
            ));
        }
        if let Some(token) = self.match_token(|kind| matches!(kind, TokenKind::Minus)) {
            let expr = self.parse_unary_expr()?;
            let span = token.span.merge(expr.span);
            Ok(Expr {
                kind: ExprKind::Unary {
                    op: UnaryOp::Neg,
                    expr: Box::new(expr),
                },
                span,
            })
        } else if let Some(token) = self.match_token(|kind| matches!(kind, TokenKind::Bang)) {
            let expr = self.parse_unary_expr()?;
            let span = token.span.merge(expr.span);
            Ok(Expr {
                kind: ExprKind::Unary {
                    op: UnaryOp::Not,
                    expr: Box::new(expr),
                },
                span,
            })
        } else {
            self.parse_call_expr()
        }
    }

    fn parse_call_expr(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_primary_expr()?;

        loop {
            if self
                .match_token(|kind| matches!(kind, TokenKind::LParen))
                .is_some()
            {
                // Preserve the callee identifier's span
                // before we move `expr.kind` into the Var
                // destructure below — `expr.span` is the
                // Var span (just the identifier) because
                // the primary parser wraps Var in an Expr
                // with its span set to the Ident's span.
                let name_span = expr.span;
                let ExprKind::Var(name) = expr.kind else {
                    return Err(Diagnostic::new(
                        expr.span,
                        "only named functions can be called",
                    ));
                };

                let mut args = Vec::new();
                if !self.check(|kind| matches!(kind, TokenKind::RParen)) {
                    loop {
                        args.push(self.parse_expr()?);
                        if self
                            .match_token(|kind| matches!(kind, TokenKind::Comma))
                            .is_none()
                        {
                            break;
                        }
                        // Allow trailing comma before `)` so
                        // multi-line call sites can use the
                        // same style as struct/enum/array
                        // literals.
                        if self.check(|k| matches!(k, TokenKind::RParen)) {
                            break;
                        }
                    }
                }
                let close = self.expect_keyword("')'", |kind| matches!(kind, TokenKind::RParen))?;
                let span = name_span.merge(close.span);
                // Arc 8 step 8f — `await(expr)` parser-level
                // desugar. Rewrites to a match that extracts
                // the Ready payload; the Pending arm panics
                // (via assert false) and falls through to a
                // literal 0 of the inferred T. v1: works for
                // Future<i64> directly. For other scalar T, the
                // user writes `await(expr) as T`. For non-scalar
                // T, manually destructure with match.
                if is_await_ident(&name) && args.len() == 1 {
                    let inner = args.into_iter().next().unwrap();
                    expr = synthesize_await_desugar(inner, span);
                } else {
                    expr = Expr {
                        kind: ExprKind::Call { name, name_span, args },
                        span,
                    };
                }
            } else if self
                .match_token(|kind| matches!(kind, TokenKind::As))
                .is_some()
            {
                let ty = self.parse_type()?;
                expr = Expr {
                    span: expr.span,
                    kind: ExprKind::Cast {
                        expr: Box::new(expr),
                        ty,
                    },
                };
            } else if self
                .match_token(|kind| matches!(kind, TokenKind::LBracket))
                .is_some()
            {
                let index = self.parse_expr()?;
                let close =
                    self.expect_keyword("']'", |kind| matches!(kind, TokenKind::RBracket))?;
                let span = expr.span.merge(close.span);
                expr = Expr {
                    kind: ExprKind::Index {
                        array: Box::new(expr),
                        index: Box::new(index),
                    },
                    span,
                };
            } else if self
                .match_token(|k| matches!(k, TokenKind::Dot))
                .is_some()
            {
                // `expr.<index>` (tuple access) or
                // `expr.<ident>` (struct field). Disambiguate
                // on the next token. T1.1 / T1.2.
                let next = self.bump();
                let span = expr.span.merge(next.span);
                match next.kind {
                    TokenKind::Int(value) => {
                        if value < 0 || value > u32::MAX as i128 {
                            return Err(Diagnostic::new(
                                next.span,
                                "tuple index must fit in u32",
                            ));
                        }
                        expr = Expr {
                            kind: ExprKind::TupleAccess {
                                tuple: Box::new(expr),
                                index: value as u32,
                            },
                            span,
                        };
                    }
                    // `t.0.0` lexes as `t`, `.`, `Float(0.0)`
                    // because the lexer greedily consumes
                    // `0.0` as a numeric literal. When both
                    // halves are non-negative integers and
                    // the string form is a single dot
                    // separator, treat it as nested tuple
                    // access. T1.1 + nested-tuple support.
                    TokenKind::Float(value) => {
                        // `{:?}` gives round-trippable form
                        // like `0.0`, while `{}` strips
                        // trailing-zero fractions to `0`.
                        let s = format!("{:?}", value);
                        let mut parts = s.split('.');
                        let n_str = parts.next();
                        let m_str = parts.next();
                        let extra = parts.next();
                        match (n_str, m_str, extra) {
                            (Some(n_str), Some(m_str), None) => {
                                if let (Ok(n), Ok(m)) = (
                                    n_str.parse::<u32>(),
                                    m_str.parse::<u32>(),
                                ) {
                                    let inner_span = next.span;
                                    expr = Expr {
                                        kind: ExprKind::TupleAccess {
                                            tuple: Box::new(expr),
                                            index: n,
                                        },
                                        span,
                                    };
                                    expr = Expr {
                                        kind: ExprKind::TupleAccess {
                                            tuple: Box::new(expr),
                                            index: m,
                                        },
                                        span: span.merge(inner_span),
                                    };
                                } else {
                                    return Err(Diagnostic::new(
                                        next.span,
                                        "expected integer (tuple index) or \
                                         identifier (field name) after '.'",
                                    ));
                                }
                            }
                            _ => {
                                return Err(Diagnostic::new(
                                    next.span,
                                    "expected integer (tuple index) or \
                                     identifier (field name) after '.'",
                                ));
                            }
                        }
                    }
                    // `TokenKind::Len` lexes the keyword `len`, used
                    // as the unary builtin `len(xs)`. In method-call
                    // position (`obj.len()`), `len` is unambiguously
                    // an identifier — there's no `len(...)` builtin
                    // syntax after `.`. Recover by mapping the
                    // keyword to a synthetic ident "len" so method-
                    // call sugar works (`m.len()` → MethodCall).
                    // Closure #312.
                    TokenKind::Len => {
                        let name_text = "len".to_string();
                        if self.check(|k| matches!(k, TokenKind::LParen)) {
                            let method_span = next.span;
                            self.expect_keyword("'('", |k| matches!(k, TokenKind::LParen))?;
                            let mut args = Vec::new();
                            if !self.check(|k| matches!(k, TokenKind::RParen)) {
                                loop {
                                    args.push(self.parse_expr()?);
                                    if self
                                        .match_token(|k| matches!(k, TokenKind::Comma))
                                        .is_none()
                                    {
                                        break;
                                    }
                                    if self.check(|k| matches!(k, TokenKind::RParen)) {
                                        break;
                                    }
                                }
                            }
                            let close = self
                                .expect_keyword("')'", |k| matches!(k, TokenKind::RParen))?;
                            expr = Expr {
                                kind: ExprKind::MethodCall {
                                    receiver: Box::new(expr),
                                    method: name_text,
                                    method_span,
                                    args,
                                },
                                span: span.merge(close.span),
                            };
                        } else {
                            expr = Expr {
                                kind: ExprKind::FieldAccess {
                                    object: Box::new(expr),
                                    field: name_text,
                                },
                                span,
                            };
                        }
                    }
                    TokenKind::Ident(name_text) => {
                        // Disambiguate: `expr.foo(args)` is a
                        // MethodCall; `expr.foo` is a
                        // FieldAccess. T1.2 phase 2a.
                        if self.check(|k| matches!(k, TokenKind::LParen)) {
                            let method_span = next.span;
                            self.expect_keyword("'('", |k| matches!(k, TokenKind::LParen))?;
                            let mut args = Vec::new();
                            if !self.check(|k| matches!(k, TokenKind::RParen)) {
                                loop {
                                    args.push(self.parse_expr()?);
                                    if self
                                        .match_token(|k| matches!(k, TokenKind::Comma))
                                        .is_none()
                                    {
                                        break;
                                    }
                                    if self.check(|k| matches!(k, TokenKind::RParen)) {
                                        break;
                                    }
                                }
                            }
                            let close = self
                                .expect_keyword("')'", |k| matches!(k, TokenKind::RParen))?;
                            expr = Expr {
                                kind: ExprKind::MethodCall {
                                    receiver: Box::new(expr),
                                    method: name_text,
                                    method_span,
                                    args,
                                },
                                span: span.merge(close.span),
                            };
                        } else {
                            expr = Expr {
                                kind: ExprKind::FieldAccess {
                                    object: Box::new(expr),
                                    field: name_text,
                                },
                                span,
                            };
                        }
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            next.span,
                            "expected integer (tuple index) or identifier (field name) after '.'",
                        ));
                    }
                }
            } else if let Some(q) =
                self.match_token(|k| matches!(k, TokenKind::Question))
            {
                // Postfix `?` — sugar for `try EXPR`. Builds the
                // same `ExprKind::Try` node the keyword form
                // produces, so the existing `desugar_try_in_v31_body`
                // (Arc 8 Phase 2.4) and the synchronous-fn `try`
                // desugar handle it identically. Narrow-gate
                // restrictions (Option-like enum, first-let-RHS
                // position) apply unchanged — diagnostics surface
                // through the same checker path.
                let span = expr.span.merge(q.span);
                expr = Expr {
                    kind: ExprKind::Try { inner: Box::new(expr) },
                    span,
                };
            } else {
                return Ok(expr);
            }
        }
    }

    fn parse_primary_expr(&mut self) -> Result<Expr, Diagnostic> {
        let token = self.bump();
        match token.kind {
            TokenKind::Int(value) => Ok(Expr {
                kind: ExprKind::Int(value),
                span: token.span,
            }),
            TokenKind::Float(value) => Ok(Expr {
                kind: ExprKind::Float(value),
                span: token.span,
            }),
            TokenKind::True => Ok(Expr {
                kind: ExprKind::Bool(true),
                span: token.span,
            }),
            TokenKind::False => Ok(Expr {
                kind: ExprKind::Bool(false),
                span: token.span,
            }),
            TokenKind::Str(text) => Ok(Expr {
                kind: ExprKind::Str(text),
                span: token.span,
            }),
            TokenKind::LBrace => {
                // Block expression: `{ stmt; stmt; tail-expr }`.
                // V1 admits `let` bindings and `print` stmts
                // in the prefix; the checker enforces the
                // surface restriction (other stmts surface a
                // clean diagnostic with the workaround). The
                // tail expression's value is the block's value.
                // Closure #129 extends the v1 Block MVP.
                let open_span = token.span;
                let mut stmts: Vec<Stmt> = Vec::new();
                loop {
                    if self.check(|k| matches!(k, TokenKind::Let)) {
                        stmts.push(self.parse_let_stmt()?);
                    } else if self.check(|k| matches!(k, TokenKind::Print)) {
                        stmts.push(self.parse_print_stmt()?);
                    } else {
                        break;
                    }
                }
                let tail = self.parse_expr()?;
                let close = self.expect_keyword(
                    "'}' (block-expression close)",
                    |k| matches!(k, TokenKind::RBrace),
                )?;
                Ok(Expr {
                    kind: ExprKind::Block {
                        stmts,
                        tail: Box::new(tail),
                    },
                    span: open_span.merge(close.span),
                })
            }
            TokenKind::Forall => {
                // `forall var: type, body`
                // Parses a universally quantified proof expression.
                // Only meaningful in proof positions (invariant/
                // requires/ensures/prove); checker enforces this.
                let open_span = token.span;
                let var_tok = self.bump();
                let var = match var_tok.kind {
                    TokenKind::Ident(name) => name,
                    _ => return Err(Diagnostic::new(
                        var_tok.span,
                        format!("expected variable name after 'forall', got {:?}", var_tok.kind),
                    )),
                };
                self.expect_keyword("':' after forall variable", |k| matches!(k, TokenKind::Colon))?;
                let ty = self.parse_type()?;
                self.expect_keyword("',' after forall type", |k| matches!(k, TokenKind::Comma))?;
                let body = self.parse_expr()?;
                let end_span = body.span;
                Ok(Expr {
                    kind: ExprKind::Forall { var, ty, body: Box::new(body) },
                    span: open_span.merge(end_span),
                })
            }
            TokenKind::Try => {
                // `try EXPR` — parse inner at call-expr
                // precedence so common forms like
                // `try maybe(5)` or `try Type.helper(args)`
                // bind correctly (without this, `try EXPR`
                // stopped at primary level and the outer
                // postfix `(...)` parser saw the try as the
                // callee, surfacing "only named functions
                // can be called"). Binary `+`/`*` etc. stay
                // outside the try by binding above
                // parse_call_expr's precedence.
                let inner = self.parse_call_expr()?;
                let inner_span = inner.span;
                Ok(Expr {
                    kind: ExprKind::Try { inner: Box::new(inner) },
                    span: token.span.merge(inner_span),
                })
            }
            TokenKind::If => {
                // If-expression: `if cond { expr } else { expr }`.
                // Both branches must be a single expression in
                // braces. Statement-bearing if blocks stay in
                // parse_stmt (which sees `if` at statement
                // position before parse_expr is invoked).
                let cond = self.parse_expr()?;
                self.expect_keyword("'{' (if-expression then-branch)", |k| {
                    matches!(k, TokenKind::LBrace)
                })?;
                let then_value = self.parse_expr()?;
                self.expect_keyword("'}' (if-expression then-branch)", |k| {
                    matches!(k, TokenKind::RBrace)
                })?;
                self.expect_keyword("'else' (if-expression)", |k| {
                    matches!(k, TokenKind::Else)
                })?;
                // `else if cond { … }` chains — the `else`
                // branch is itself an if-expression. Allow
                // `if cond { e1 } else if cond2 { e2 } else
                // { e3 }` as a single nested if-expression
                // tree.
                let (else_value, close_span) =
                    if self.check(|k| matches!(k, TokenKind::If)) {
                        let nested = self.parse_primary_expr()?;
                        let nested_span = nested.span;
                        (nested, nested_span)
                    } else {
                        self.expect_keyword(
                            "'{' (if-expression else-branch)",
                            |k| matches!(k, TokenKind::LBrace),
                        )?;
                        let v = self.parse_expr()?;
                        let close = self.expect_keyword(
                            "'}' (if-expression else-branch)",
                            |k| matches!(k, TokenKind::RBrace),
                        )?;
                        (v, close.span)
                    };
                Ok(Expr {
                    kind: ExprKind::IfExpr {
                        cond: Box::new(cond),
                        then_value: Box::new(then_value),
                        else_value: Box::new(else_value),
                    },
                    span: token.span.merge(close_span),
                })
            }
            TokenKind::Pipe => {
                // Closure #374: anonymous-fn shorthand `|x| x + 1`
                // and `|x, y| x + y`. Desugars to an AnonFn AST
                // node with all parameters typed `i64` and return
                // type `i64` (matches the existing v1 closures
                // surface — all closures take + return i64). Body
                // is `return <expr>;`. Disambiguation: at primary
                // position `|` always starts a closure shorthand;
                // bitwise-or `a | b` is a binary infix operator,
                // never a primary leader. Empty param list
                // requires `||` which lexes as the OrOr token,
                // so `|| expr` is naturally unreachable through
                // this path — keep the shorthand requiring at
                // least one parameter.
                let pipe_span = token.span;
                let mut params: Vec<Param> = Vec::new();
                loop {
                    let pname_tok = self.expect_ident()?;
                    let pname_span = pname_tok.span;
                    let pname = ident_text(pname_tok);
                    params.push(Param {
                        name: pname,
                        ty: Type::I64,
                        name_span: pname_span,
                        span: pname_span,
                    });
                    if self.match_token(|k| matches!(k, TokenKind::Comma)).is_none() {
                        break;
                    }
                }
                self.expect_keyword("'|'", |k| matches!(k, TokenKind::Pipe))?;
                let body_expr = self.parse_expr()?;
                let body_span = body_expr.span;
                // Closure #374 follow-up: peek at the body's
                // top-level operator to infer the return type.
                // Comparison + logical ops produce bool; otherwise
                // default to i64. Lets `|x| x > 5` and
                // `|x| x % 2 == 0` work as predicates for
                // vec_filter / vec_position. Composing closures
                // with binary OR / AND / boolean Not still
                // returns bool. Bool-literal body also returns
                // bool. Anything else (arithmetic, index, etc.)
                // defaults to i64 — matches the dominant
                // vec_map / sort_by / vec_fold use cases.
                let return_type = match &body_expr.kind {
                    ExprKind::Binary { op, .. } => match op {
                        BinaryOp::Eq | BinaryOp::Ne
                        | BinaryOp::Lt | BinaryOp::Le
                        | BinaryOp::Gt | BinaryOp::Ge
                        | BinaryOp::And | BinaryOp::Or => Type::Bool,
                        _ => Type::I64,
                    },
                    ExprKind::Unary { op, .. } => match op {
                        UnaryOp::Not => Type::Bool,
                        _ => Type::I64,
                    },
                    ExprKind::Bool(_) => Type::Bool,
                    _ => Type::I64,
                };
                let body = vec![Stmt::Return {
                    expr: body_expr,
                    span: body_span,
                }];
                Ok(Expr {
                    kind: ExprKind::AnonFn {
                        params,
                        return_type,
                        body,
                        fn_span: pipe_span,
                        // |x| shorthand doesn't support explicit
                        // capture lists — only the `fn(...)` long
                        // form does. Implicit captures stay
                        // by-value.
                        ref_captures: Vec::new(),
                    },
                    span: pipe_span.merge(body_span),
                })
            }
            TokenKind::Fn => {
                // Anonymous fn expression — `fn(p: T) -> R { body }`.
                // Body is parsed as a regular fn body (Vec<Stmt> with
                // auto-`return 0` unit-return shorthand). v1 has no
                // captures — the checker's lambda-lift pass hoists
                // each AnonFn into a generated `__anon_fn_<N>` top-
                // level fn; outer-variable references then surface
                // the usual "unknown variable" diagnostic. Closure #308.
                let fn_span = token.span;
                self.expect_keyword("'('", |k| matches!(k, TokenKind::LParen))?;
                let mut params: Vec<Param> = Vec::new();
                if !self.check(|k| matches!(k, TokenKind::RParen)) {
                    loop {
                        let pname_tok = self.expect_ident()?;
                        let pname_span = pname_tok.span;
                        let pname = ident_text(pname_tok);
                        self.expect_keyword("':'", |k| matches!(k, TokenKind::Colon))?;
                        let pty = self.parse_type()?;
                        params.push(Param {
                            name: pname,
                            ty: pty,
                            name_span: pname_span,
                            span: pname_span,
                        });
                        if self.match_token(|k| matches!(k, TokenKind::Comma)).is_none() {
                            break;
                        }
                        if self.check(|k| matches!(k, TokenKind::RParen)) {
                            break;
                        }
                    }
                }
                self.expect_keyword("')'", |k| matches!(k, TokenKind::RParen))?;
                let unit_return = !self.check(|k| matches!(k, TokenKind::Arrow));
                let return_type = if unit_return {
                    Type::I64
                } else {
                    self.expect_keyword("'->'", |k| matches!(k, TokenKind::Arrow))?;
                    self.parse_type()?
                };
                // ARC 3a: optional explicit capture list
                // `[ref name1, ref name2, ...]` between the
                // return type and the body. Each entry names a
                // free variable in the body that should be
                // captured BY REFERENCE rather than by value.
                // Backwards compatible: omitting the list keeps
                // captures implicit + by-value (today's default).
                let mut ref_captures: Vec<String> = Vec::new();
                if self.check(|k| matches!(k, TokenKind::LBracket)) {
                    self.bump(); // consume `[`
                    if !self.check(|k| matches!(k, TokenKind::RBracket)) {
                        loop {
                            self.expect_keyword(
                                "'ref' before captured identifier",
                                |k| matches!(k, TokenKind::Ref),
                            )?;
                            let cap_tok = self.expect_ident()?;
                            ref_captures.push(ident_text(cap_tok));
                            if self.match_token(|k| matches!(k, TokenKind::Comma)).is_none() {
                                break;
                            }
                            if self.check(|k| matches!(k, TokenKind::RBracket)) {
                                break;
                            }
                        }
                    }
                    self.expect_keyword(
                        "']' to close capture list",
                        |k| matches!(k, TokenKind::RBracket),
                    )?;
                }
                self.expect_keyword("'{'", |k| matches!(k, TokenKind::LBrace))?;
                let mut body: Vec<Stmt> = Vec::new();
                while !self.check(|k| matches!(k, TokenKind::RBrace | TokenKind::Eof)) {
                    match self.parse_stmt() {
                        Ok(s) => body.push(s),
                        Err(e) => {
                            self.errors.push(e);
                            self.sync_to_stmt();
                        }
                    }
                }
                let close = self.expect_keyword("'}'", |k| matches!(k, TokenKind::RBrace))?;
                if unit_return {
                    let last_is_return = matches!(body.last(), Some(Stmt::Return { .. }));
                    if !last_is_return {
                        body.push(Stmt::Return {
                            expr: Expr {
                                kind: ExprKind::Int(0),
                                span: close.span,
                            },
                            span: close.span,
                        });
                    }
                }
                Ok(Expr {
                    kind: ExprKind::AnonFn {
                        params,
                        return_type,
                        body,
                        fn_span,
                        ref_captures,
                    },
                    span: fn_span.merge(close.span),
                })
            }
            TokenKind::Match => {
                let scrutinee = self.parse_expr()?;
                self.expect_keyword("'{'", |k| matches!(k, TokenKind::LBrace))?;
                let mut arms = Vec::new();
                while !self.check(|k| matches!(k, TokenKind::RBrace | TokenKind::Eof)) {
                    // Five pattern shapes in v1:
                    //   - `_` wildcard
                    //   - `42` / `-1` integer literal
                    //   - `true` / `false` bool literal
                    //   - `"foo"` string literal
                    //   - `EnumName.VariantName` variant
                    // Dispatch on the first token: Minus or
                    // Int → integer; True/False → bool; Str
                    // → string; identifier `_` → wildcard;
                    // any other identifier → variant.
                    // T1.3 wildcard + integer-literal pattern.
                    let (pattern, pat_span) = if self
                        .check(|k| matches!(k, TokenKind::True))
                    {
                        let tok = self.bump();
                        (Pattern::Bool(true), tok.span)
                    } else if self.check(|k| matches!(k, TokenKind::False)) {
                        let tok = self.bump();
                        (Pattern::Bool(false), tok.span)
                    } else if self.check(|k| matches!(k, TokenKind::Str(_))) {
                        let tok = self.bump();
                        let span = tok.span;
                        let text = match tok.kind {
                            TokenKind::Str(s) => s,
                            _ => unreachable!(),
                        };
                        (Pattern::Str(text), span)
                    } else if self
                        .check(|k| matches!(k, TokenKind::Minus | TokenKind::Int(_) | TokenKind::Float(_)))
                    {
                        let pat_start = self.current().span;
                        let mut negative = false;
                        if self
                            .match_token(|k| matches!(k, TokenKind::Minus))
                            .is_some()
                        {
                            negative = true;
                        }
                        let lit_tok = self.bump();
                        let lit_span = lit_tok.span;
                        match lit_tok.kind {
                            TokenKind::Int(v) => {
                                let value = if negative {
                                    match v.checked_neg() {
                                        Some(neg) => neg,
                                        None => {
                                            return Err(Diagnostic::new(
                                                pat_start.merge(lit_span),
                                                "integer pattern overflow when negating",
                                            ));
                                        }
                                    }
                                } else {
                                    v
                                };
                                (Pattern::Int(value), pat_start.merge(lit_span))
                            }
                            // Closure #278: float literal pattern.
                            // Scrutinee must be `f32` / `f64`;
                            // dispatch is via `==`. A wildcard arm
                            // is required since the float space is
                            // open. NaN scrutinees never match any
                            // literal arm (IEEE 754).
                            TokenKind::Float(v) => {
                                let value = if negative { -v } else { v };
                                (Pattern::Float(value), pat_start.merge(lit_span))
                            }
                            _ => {
                                return Err(Diagnostic::new(
                                    lit_span,
                                    "expected integer or float literal in match pattern",
                                ));
                            }
                        }
                    } else {
                        let first_tok = self.expect_ident()?;
                        let pat_start = first_tok.span;
                        let first_text = ident_text(first_tok);
                        if first_text == "_" {
                            (Pattern::Wildcard, pat_start)
                        } else {
                            self.expect_keyword(
                                "'.' (variant access in match pattern)",
                                |k| matches!(k, TokenKind::Dot),
                            )?;
                            let variant_tok = self.expect_ident()?;
                            let mut pat_span = pat_start.merge(variant_tok.span);
                            let variant = ident_text(variant_tok);
                            // Optional `(binding)` after the variant
                            // name — payloaded destructure. T1.3
                            // phase 2b. v1 accepts the single-binding
                            // form (`Some(x)`) only; multi-binding
                            // tuple-style destructure is deferred.
                            if self.check(|k| matches!(k, TokenKind::LParen)) {
                                self.bump();
                                let binding_tok = self.expect_ident()?;
                                let binding = ident_text(binding_tok);
                                let close = self.expect_keyword(
                                    "')' (variant payload binding close)",
                                    |k| matches!(k, TokenKind::RParen),
                                )?;
                                pat_span = pat_start.merge(close.span);
                                (
                                    Pattern::VariantWithBinding {
                                        enum_name: first_text,
                                        variant,
                                        binding,
                                    },
                                    pat_span,
                                )
                            } else {
                                (
                                    Pattern::Variant {
                                        enum_name: first_text,
                                        variant,
                                    },
                                    pat_span,
                                )
                            }
                        }
                    };
                    self.expect_keyword("'then'", |k| matches!(k, TokenKind::Then))?;
                    let body = self.parse_expr()?;
                    arms.push(MatchArm {
                        pattern,
                        pattern_span: pat_span,
                        body,
                    });
                    // Comma between arms required; trailing
                    // comma before `}` allowed.
                    if self.match_token(|k| matches!(k, TokenKind::Comma)).is_none() {
                        break;
                    }
                }
                let close = self.expect_keyword(
                    "'}' (match expression)",
                    |k| matches!(k, TokenKind::RBrace),
                )?;
                Ok(Expr {
                    kind: ExprKind::Match {
                        scrutinee: Box::new(scrutinee),
                        arms,
                    },
                    span: token.span.merge(close.span),
                })
            }
            TokenKind::Ident(first_name) => {
                // Closure #242: path expression
                // `module::item`. v1 supports a single `::`
                // (no nested modules). The resulting `name`
                // is the joined path string; later parser
                // logic (struct literal / call / var) uses
                // it unchanged. The checker recognizes
                // `::` in identifier names and routes
                // through module resolution.
                let mut name = first_name.clone();
                let mut name_span = token.span;
                // Closure #248: support deep paths
                // `a::b::c::…` by looping the `::` consumption.
                // Each segment after the first is concatenated
                // with `__` to produce the backend-safe
                // identifier. Nested modules use this to
                // address deeply-nested items.
                while self.check(|k| matches!(k, TokenKind::ColonColon)) {
                    self.bump(); // consume ::
                    let next_tok = self.expect_ident()?;
                    let next_span = next_tok.span;
                    let next_name = ident_text(next_tok);
                    name = format!("{}__{}", name, next_name);
                    name_span = name_span.merge(next_span);
                }
                // Struct literal `Name { field: val, … }` —
                // we recognize the shape by looking past
                // `{` for `ident :`. Anything else means
                // we leave the identifier alone (block,
                // var). The capitalization convention
                // (struct names start uppercase) gates the
                // attempt so plain variables never trip the
                // lookahead. T1.2.
                // For module-qualified names like `foo__Point`
                // the LAST segment's capitalization is what
                // counts.
                // Heuristic: a name is "struct-shaped" when either
                // its FIRST segment OR its LAST segment starts
                // uppercase. This catches both module-qualified
                // user structs (`foo::Point` → `foo__Point`,
                // last segment upper) AND v3.1-synthesized
                // names (`Task__showcase`, first segment upper)
                // whose suffix after `__` is the user's
                // lowercase async-fn name.
                let last_segment: &str = name.rsplit("__").next().unwrap_or(&name);
                let last_upper = last_segment
                    .chars()
                    .next()
                    .map(|c| c.is_ascii_uppercase())
                    .unwrap_or(false);
                let first_upper = name
                    .chars()
                    .next()
                    .map(|c| c.is_ascii_uppercase())
                    .unwrap_or(false);
                let starts_uppercase = last_upper || first_upper;
                let starts_with_lbrace = matches!(self.current().kind, TokenKind::LBrace);
                let inner_is_field = matches!(
                    self.tokens.get(self.pos + 1).map(|t| &t.kind),
                    Some(TokenKind::Ident(_))
                ) && matches!(
                    self.tokens.get(self.pos + 2).map(|t| &t.kind),
                    Some(TokenKind::Colon)
                );
                let inner_is_empty = matches!(
                    self.tokens.get(self.pos + 1).map(|t| &t.kind),
                    Some(TokenKind::RBrace)
                );
                let looks_like_struct = starts_uppercase
                    && starts_with_lbrace
                    && (inner_is_field || inner_is_empty);
                if looks_like_struct {
                    self.bump(); // {
                    let mut fields = Vec::new();
                    while !self.check(|k| matches!(k, TokenKind::RBrace | TokenKind::Eof)) {
                        let fname_tok = self.expect_ident()?;
                        let fname = ident_text(fname_tok);
                        self.expect_keyword("':'", |k| matches!(k, TokenKind::Colon))?;
                        let value = self.parse_expr()?;
                        fields.push((fname, value));
                        if self.match_token(|k| matches!(k, TokenKind::Comma)).is_none() {
                            break;
                        }
                    }
                    let close = self.expect_keyword(
                        "'}' (struct literal)",
                        |k| matches!(k, TokenKind::RBrace),
                    )?;
                    return Ok(Expr {
                        kind: ExprKind::StructLit {
                            type_name: name,
                            type_name_span: name_span,
                            fields,
                        },
                        span: name_span.merge(close.span),
                    });
                }
                Ok(Expr {
                    kind: ExprKind::Var(name),
                    span: name_span,
                })
            }
            TokenKind::LParen => {
                // Parenthesized form: either grouped expression
                // `(e)` or tuple `(e1, e2, …)`. Disambiguate
                // on the comma after the first sub-expression.
                let first = self.parse_expr()?;
                if self
                    .match_token(|k| matches!(k, TokenKind::Comma))
                    .is_some()
                {
                    let mut elements = vec![first];
                    loop {
                        // Trailing comma allowed: stop if we
                        // see `)` right after a comma.
                        if self.check(|k| matches!(k, TokenKind::RParen)) {
                            break;
                        }
                        elements.push(self.parse_expr()?);
                        if self
                            .match_token(|k| matches!(k, TokenKind::Comma))
                            .is_none()
                        {
                            break;
                        }
                    }
                    let close = self.expect_keyword(
                        "')'",
                        |k| matches!(k, TokenKind::RParen),
                    )?;
                    return Ok(Expr {
                        kind: ExprKind::Tuple(elements),
                        span: token.span.merge(close.span),
                    });
                }
                self.expect_keyword("')'", |kind| matches!(kind, TokenKind::RParen))?;
                Ok(first)
            }
            TokenKind::LBracket => {
                let mut elements = Vec::new();
                if !self.check(|kind| matches!(kind, TokenKind::RBracket)) {
                    loop {
                        elements.push(self.parse_expr()?);
                        if self
                            .match_token(|kind| matches!(kind, TokenKind::Comma))
                            .is_none()
                        {
                            break;
                        }
                        // Allow a trailing comma before `]`
                        // so multi-line array literals can use
                        // the same comma-on-every-line style
                        // as struct/enum/methods blocks.
                        if self.check(|k| matches!(k, TokenKind::RBracket)) {
                            break;
                        }
                    }
                }
                let close =
                    self.expect_keyword("']'", |kind| matches!(kind, TokenKind::RBracket))?;
                Ok(Expr {
                    kind: ExprKind::ArrayLit { elements },
                    span: token.span.merge(close.span),
                })
            }
            TokenKind::Len => {
                self.expect_keyword("'('", |kind| matches!(kind, TokenKind::LParen))?;
                let array = self.parse_expr()?;
                let close =
                    self.expect_keyword("')'", |kind| matches!(kind, TokenKind::RParen))?;
                Ok(Expr {
                    kind: ExprKind::Len {
                        array: Box::new(array),
                    },
                    span: token.span.merge(close.span),
                })
            }
            TokenKind::While => {
                // `while cond { ... }` as an expression — yields the
                // value passed to `break val;`. The checker lowers
                // `let x: T = while ... { break val; }` to a temp-var
                // sequence so backends never see WhileLoop directly.
                let open_span = token.span;
                let cond = self.parse_expr()?;
                let body = self.parse_block()?;
                let end_span = body.last().map(|s| s.span()).unwrap_or(open_span);
                Ok(Expr {
                    kind: ExprKind::WhileLoop {
                        label: None,
                        cond: Box::new(cond),
                        body,
                    },
                    span: open_span.merge(end_span),
                })
            }
            TokenKind::Label(_) => {
                // `'name: while cond { ... }` in expression position.
                let label_name = if let TokenKind::Label(n) = token.kind { n } else { unreachable!() };
                self.expect_keyword("':' after loop label", |kind| matches!(kind, TokenKind::Colon))?;
                let while_tok = self.expect_keyword("'while' after labeled expression", |kind| matches!(kind, TokenKind::While))?;
                let cond = self.parse_expr()?;
                let body = self.parse_block()?;
                let end_span = body.last().map(|s| s.span()).unwrap_or(while_tok.span);
                Ok(Expr {
                    kind: ExprKind::WhileLoop {
                        label: Some(label_name),
                        cond: Box::new(cond),
                        body,
                    },
                    span: token.span.merge(end_span),
                })
            }
            // `min(a, b)` / `max(a, b)` no longer get a
            // dedicated parse arm — they're regular
            // identifier calls that the checker dispatches
            // to the intrinsic helper based on the name.
            // This frees `min` / `max` as legal field /
            // local names outside the reduction-op context.
            _ => Err(Diagnostic::new(token.span, "expected expression")),
        }
    }

    fn current_binary_op(&self) -> Option<(BinaryOp, u8)> {
        // Precedence follows Rust: `|` < `^` < `&` < shifts < `+/-`
        // < `*//%`. Comparisons sit above `&&`/`||` and below the
        // bitwise ops, so `a == b | c` parses as `a == (b | c)`.
        // `&` doubles as the prefix reference operator; the unary
        // path is handled separately in `parse_unary_expr`, so
        // listing `Amp` here only affects infix positions.
        match self.current().kind {
            TokenKind::OrOr => Some((BinaryOp::Or, 1)),
            TokenKind::AndAnd => Some((BinaryOp::And, 2)),
            TokenKind::EqEq => Some((BinaryOp::Eq, 3)),
            TokenKind::BangEq => Some((BinaryOp::Ne, 3)),
            TokenKind::Less => Some((BinaryOp::Lt, 4)),
            TokenKind::LessEq => Some((BinaryOp::Le, 4)),
            TokenKind::Greater => Some((BinaryOp::Gt, 4)),
            TokenKind::GreaterEq => Some((BinaryOp::Ge, 4)),
            TokenKind::Pipe => Some((BinaryOp::BitOr, 5)),
            TokenKind::Caret => Some((BinaryOp::BitXor, 6)),
            TokenKind::Amp => Some((BinaryOp::BitAnd, 7)),
            TokenKind::LessLess => Some((BinaryOp::Shl, 8)),
            TokenKind::GreaterGreater => Some((BinaryOp::Shr, 8)),
            TokenKind::Plus => Some((BinaryOp::Add, 9)),
            TokenKind::Minus => Some((BinaryOp::Sub, 9)),
            TokenKind::Star => Some((BinaryOp::Mul, 10)),
            TokenKind::Slash => Some((BinaryOp::Div, 10)),
            TokenKind::Percent => Some((BinaryOp::Rem, 10)),
            _ => None,
        }
    }

    fn expect_ident(&mut self) -> Result<Token, Diagnostic> {
        if self.check(|kind| matches!(kind, TokenKind::Ident(_))) {
            Ok(self.bump())
        } else {
            Err(self.error_here("expected identifier"))
        }
    }

    fn expect_string(&mut self) -> Result<Token, Diagnostic> {
        if self.check(|kind| matches!(kind, TokenKind::Str(_))) {
            Ok(self.bump())
        } else {
            Err(self.error_here("expected string literal"))
        }
    }

    fn expect_close_angle(&mut self) -> Result<(), Diagnostic> {
        let current_kind = self.current().kind.clone();
        match current_kind {
            TokenKind::Greater => {
                self.bump();
                Ok(())
            }
            TokenKind::GreaterGreater => {
                // Split `>>` into `>` + `>` so nested `Vec<Vec<T>>` parses.
                let span = self.current().span;
                let split_start = span.start + 1;
                self.tokens[self.pos] = Token {
                    kind: TokenKind::Greater,
                    span: crate::span::Span::new(split_start, span.end),
                };
                Ok(())
            }
            _ => Err(self.error_here("expected '>'")),
        }
    }

    fn expect_keyword(
        &mut self,
        expected: &'static str,
        predicate: impl FnOnce(&TokenKind) -> bool,
    ) -> Result<Token, Diagnostic> {
        if predicate(&self.current().kind) {
            Ok(self.bump())
        } else {
            Err(self.error_here(format!("expected {}", expected)))
        }
    }

    fn match_token(&mut self, predicate: impl FnOnce(&TokenKind) -> bool) -> Option<Token> {
        if predicate(&self.current().kind) {
            Some(self.bump())
        } else {
            None
        }
    }

    /// Arc 8 step 8b — peek for the `async` contextual keyword.
    /// True iff current token is `Ident("<async-spelling>")` AND
    /// the next is `fn` (with optional `pure` between). The
    /// identifier shape (not a real `TokenKind::Async`) preserves
    /// the existing carve-out: bare identifiers named `async`
    /// in expression position keep working.
    ///
    /// Per-dialect spellings shipped 2026-06-08 (polish queue):
    ///   English   — `async`
    ///   Sanskrit / Hindi / Marathi — `अतुल्यकालिक` (atulyakālika,
    ///                                "non-synchronous")
    ///   Mandarin  — `异步` (yìbù)
    ///   Japanese  — `非同期` (hidouki)
    ///
    /// Caveat: the non-English spellings are tatsama / technical-
    /// coinage attestations, not native-speaker validated.
    /// Grammar-consultant review (Tier 3) will revise these if
    /// the canonical form differs in active CS usage.
    fn check_async_prefix(&self) -> bool {
        let TokenKind::Ident(name) = &self.current().kind else {
            return false;
        };
        if !is_async_ident(name) {
            return false;
        }
        // Peek past the `async` token. Allow `async fn` and
        // `async pure fn`.
        let next = self.tokens.get(self.pos + 1).map(|t| &t.kind);
        match next {
            Some(TokenKind::Fn) => true,
            Some(TokenKind::Pure) => matches!(
                self.tokens.get(self.pos + 2).map(|t| &t.kind),
                Some(TokenKind::Fn)
            ),
            _ => false,
        }
    }

    fn check(&self, predicate: impl FnOnce(&TokenKind) -> bool) -> bool {
        predicate(&self.current().kind)
    }

    fn current(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn bump(&mut self) -> Token {
        let token = self.current().clone();
        if !matches!(token.kind, TokenKind::Eof) {
            self.pos += 1;
        }
        token
    }

    fn error_here(&self, message: impl Into<String>) -> Diagnostic {
        Diagnostic::new(self.current().span, message)
    }
}

fn ident_text(token: Token) -> String {
    match token.kind {
        TokenKind::Ident(name) => name,
        _ => unreachable!("expected identifier"),
    }
}

/// Arc 8 step 8f — synthesize the AST for `await(expr)`:
///   `match expr { Future.Ready(__v) then __v, Future.Pending then 0 }`
/// The Pending arm body is a literal `0` (i64). For v1 this
/// works directly for `Future<i64>`; the user explicitly casts
/// (`await(future_f64) as f64` would type-check via the
/// surrounding context's coercion). Non-scalar T should match
/// Polish (2026-06-08) — async/await dialect lift. Recognize
/// the per-dialect spellings of `async` (as a contextual
/// fn-prefix) and `await` (as a function-call-style operator).
/// Each predicate returns true when the identifier text matches
/// the English form OR one of the registered dialect aliases.
///
/// Spellings: English `async` + Sanskrit-rooted Indo-Aryan
/// `अतुल्यकालिक` (atulyakālika) + Mandarin `异步` (yìbù) +
/// Japanese `非同期` (hidouki). For `await`: English + Indo-
/// Aryan `प्रतीक्षा` (pratīkṣā) + Mandarin `等候` (děnghòu —
/// distinct from `等待` which is Mandarin's `join`) + Japanese
/// `待機` (taiki). Grammar consultant pass (Tier 3) will
/// revise these against native-speaker active-use spellings.
fn is_async_ident(name: &str) -> bool {
    matches!(
        name,
        "async" | "अतुल्यकालिक" | "异步" | "非同期"
    )
}

fn is_await_ident(name: &str) -> bool {
    matches!(
        name,
        "await" | "प्रतीक्षा" | "等候" | "待機"
    )
}

/// manually until the state-machine codegen (Arc 8 step 8c)
/// lands.
fn synthesize_await_desugar(inner: Expr, span: Span) -> Expr {
    let v_var = Expr {
        kind: ExprKind::Var("__await_v".to_string()),
        span,
    };
    let arms = vec![
        MatchArm {
            pattern: Pattern::VariantWithBinding {
                enum_name: "Future".to_string(),
                variant: "Ready".to_string(),
                binding: "__await_v".to_string(),
            },
            pattern_span: span,
            body: v_var,
        },
        MatchArm {
            pattern: Pattern::Variant {
                enum_name: "Future".to_string(),
                variant: "Pending".to_string(),
            },
            pattern_span: span,
            body: Expr {
                kind: ExprKind::Int(0),
                span,
            },
        },
    ];
    Expr {
        kind: ExprKind::Match {
            scrutinee: Box::new(inner),
            arms,
        },
        span,
    }
}

/// Arc 8 step 8b — recursively rewrite every `Return { expr }`
/// statement inside an async fn body so `expr` is wrapped in
/// `Future.Ready(expr)`. Recurses into nested blocks (if /
/// while / for / ForIter / TaskSpawn) so deep returns lift
/// correctly. The body's final implicit `return 0` (added by
/// `parse_function` when the user wrote no explicit return)
/// gets the same treatment — its synthesized expr becomes
/// `Future.Ready(0)`.
fn wrap_returns_in_future_ready(body: &mut Vec<Stmt>) {
    for stmt in body.iter_mut() {
        wrap_returns_in_future_ready_stmt(stmt);
    }
}

fn wrap_returns_in_future_ready_stmt(stmt: &mut Stmt) {
    match stmt {
        Stmt::Return { expr, span } => {
            let inner = std::mem::replace(
                expr,
                Expr {
                    kind: ExprKind::Int(0),
                    span: *span,
                },
            );
            let span = inner.span;
            *expr = Expr {
                kind: ExprKind::MethodCall {
                    receiver: Box::new(Expr {
                        kind: ExprKind::Var("Future".to_string()),
                        span,
                    }),
                    method: "Ready".to_string(),
                    method_span: span,
                    args: vec![inner],
                },
                span,
            };
        }
        Stmt::If { then_body, else_body, .. } => {
            wrap_returns_in_future_ready(then_body);
            wrap_returns_in_future_ready(else_body);
        }
        Stmt::While { body, .. } => wrap_returns_in_future_ready(body),
        Stmt::For { body, .. } => wrap_returns_in_future_ready(body),
        Stmt::ForIter { body, .. } => wrap_returns_in_future_ready(body),
        Stmt::TaskSpawn { body, .. } => wrap_returns_in_future_ready(body),
        _ => {}
    }
}

/// Devanagari surface (Phase 1) entry-point aliases. The
/// Sanskrit/Hindi/Marathi words for "main / primary / principal"
/// canonicalize to the English `main` symbol so the rest of the
/// compiler doesn't need to know that any of the alternative
/// spellings exist.
///
/// Accepted forms:
/// - `main` (English, unchanged)
/// - `मुख्य` (mukhya — common across Sanskrit, Hindi, Marathi;
///   the most natural rendering of "main / chief")
/// - `प्रमुख` (pramukh — Sanskrit/Hindi/Marathi "primary,
///   foremost"; also a common noun form for a head/leader)
/// - `प्रधान` (pradhan — Sanskrit/Hindi/Marathi "principal /
///   chief"; the same word used in "Prime Minister",
///   प्रधानमंत्री)
///
/// A program that declares two of these forms (e.g. both `main`
/// and `मुख्य`) errors at the existing duplicate-fn check —
/// they're aliases for the SAME symbol, not parallel functions.
fn canonicalize_entry_point_name(name: String) -> String {
    match name.as_str() {
        "main" | "मुख्य" | "प्रमुख" | "प्रधान" => "main".to_string(),
        _ => name,
    }
}

/// Recognize integer-literal initializers, including
/// arithmetic over previously-declared consts. Used by
/// `parse_const_decl` to stash literal int values for the
/// `[T; SIZE]` array-length resolver. Mirrors the checker's
/// `literal_const_value` const-fold. T0.0 follow-up.
fn expr_as_int_literal(
    expr: &Expr,
    prior_consts: &std::collections::HashMap<String, i128>,
) -> Option<i128> {
    match &expr.kind {
        ExprKind::Int(v) => Some(*v),
        ExprKind::Var(name) => prior_consts.get(name).copied(),
        ExprKind::Unary { op: UnaryOp::Neg, expr: inner } => {
            expr_as_int_literal(inner, prior_consts)?.checked_neg()
        }
        ExprKind::Binary { op, left, right } => {
            let l = expr_as_int_literal(left, prior_consts)?;
            let r = expr_as_int_literal(right, prior_consts)?;
            match op {
                BinaryOp::Add => l.checked_add(r),
                BinaryOp::Sub => l.checked_sub(r),
                BinaryOp::Mul => l.checked_mul(r),
                BinaryOp::Div if r != 0 => l.checked_div(r),
                BinaryOp::Rem if r != 0 => l.checked_rem(r),
                _ => None,
            }
        }
        _ => None,
    }
}

// =========================================================
// Arc 8 v3.1 Phase 1 — compiler-driven state-machine codegen
// =========================================================
//
// For an `async fn` whose body contains calls to the
// `io_*_async` builtin family, this transform replaces the
// body with a constructor that returns a per-fn task struct,
// and synthesizes both the struct + a `__poll_<name>` function
// that drives the state machine.
//
// Scope (linear-core, Phase 1):
// - Body is LINEAR: only Let / Return / Discard / Print at the
//   top level. No if / while / for / match / try / break /
//   continue. Reject with a clear diagnostic if found.
// - All params + locals are i64. Reject non-i64 with a clear
//   diagnostic.
// - Return type is i64. Each Let RHS is either:
//   * a Call to `io_recv_async(fd, max)`,
//     `io_send_async(fd, n)`, or `io_accept_async(fd)` —
//     these are SUSPEND POINTS that bump the state tag and
//     check for Pending (-2) / Error (-1)
//   * any other i64-typed expression — emitted verbatim in
//     the current state's prologue
// - Return EXPR — rewritten so locals/params reference
//   `t.field`; emitted in the final state.
//
// Out of scope (deferred to Phases 2-4):
// - Control flow inside async body (Phase 2)
// - Non-i64 locals / affine types across await (Phase 3)
// - Multi-await in one expression / ref params / generics /
//   nested async calls / CancelToken auto-plumbing (Phase 4)
//
// The synthesized struct + poll fn get queued in
// `V31_TASK_REGISTRY`; `parse_program` flushes the queue into
// `Program.structs` + `Program.functions` at end-of-parse.

/// Detect whether an `async fn` body contains any
/// `io_*_async` call. Walks Let / Return / Print / Discard /
/// FieldAssign / Assign / IndexAssign statements and the
/// embedded Expr trees. Returns true on the first hit.
pub(crate) fn body_uses_io_async(body: &[Stmt]) -> bool {
    fn expr_uses(e: &Expr) -> bool {
        match &e.kind {
            ExprKind::Call { name, args, .. } => {
                // Phase 4a-broad — `__poll_<inner>` calls trigger
                // the v3.1 transform on their callers too, so an
                // async fn that only awaits sub-tasks (no
                // io_*_async itself) still gets state-machined.
                if name.starts_with("__poll_") || matches!(
                    name.as_str(),
                    "io_recv_async" | "io_send_async" | "io_accept_async"
                ) {
                    return true;
                }
                args.iter().any(expr_uses)
            }
            ExprKind::Binary { left, right, .. } => expr_uses(left) || expr_uses(right),
            ExprKind::Unary { expr, .. }
            | ExprKind::Cast { expr, .. } => expr_uses(expr),
            ExprKind::Index { array, index } => expr_uses(array) || expr_uses(index),
            ExprKind::ArrayLit { elements } | ExprKind::Tuple(elements) => {
                elements.iter().any(expr_uses)
            }
            ExprKind::TupleAccess { tuple, .. } => expr_uses(tuple),
            ExprKind::Len { array } => expr_uses(array),
            ExprKind::Ref { inner } | ExprKind::RefMut { inner } => expr_uses(inner),
            ExprKind::StructLit { fields, .. } => fields.iter().any(|(_, v)| expr_uses(v)),
            ExprKind::FieldAccess { object, .. } => expr_uses(object),
            ExprKind::MethodCall { receiver, args, .. } => {
                expr_uses(receiver) || args.iter().any(expr_uses)
            }
            // Phase 2.3-narrow: walk Match scrutinee + arm
            // bodies so the outer transform-trigger detects
            // suspends nested in match (per-arm state graphs
            // arrive in Phase 2.3a).
            ExprKind::Match { scrutinee, arms } => {
                expr_uses(scrutinee) || arms.iter().any(|a| expr_uses(&a.body))
            }
            _ => false,
        }
    }
    fn stmt_uses(s: &Stmt) -> bool {
        match s {
            Stmt::Let { expr, .. }
            | Stmt::LetTuple { expr, .. }
            | Stmt::Return { expr, .. }
            | Stmt::Assert { expr, .. }
            | Stmt::Prove { expr, .. }
            | Stmt::Assign { expr, .. } => expr_uses(expr),
            Stmt::Print { items, .. } => items.iter().any(|it| match it {
                crate::ast::PrintItem::Expr(e) => expr_uses(e),
                crate::ast::PrintItem::Str(_) => false,
            }),
            Stmt::If { cond, then_body, else_body, .. } => {
                expr_uses(cond)
                    || then_body.iter().any(stmt_uses)
                    || else_body.iter().any(stmt_uses)
            }
            Stmt::While { cond, body, .. } => {
                expr_uses(cond) || body.iter().any(stmt_uses)
            }
            Stmt::IndexAssign { index, value, .. } => expr_uses(index) || expr_uses(value),
            Stmt::FieldAssign { object, value, .. } => expr_uses(object) || expr_uses(value),
            _ => false,
        }
    }
    body.iter().any(stmt_uses)
}

/// Recursively check whether an expression contains any
/// `io_*_async` call. Mirror of `body_uses_io_async` but
/// scoped to a single Expr — used by Phase 2's validator to
/// decide if a control-flow branch needs the full state-
/// splitting transform (deferred to Phase 2.1) or can be
/// emitted verbatim in the current state arm.
fn expr_contains_io_async(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Call { name, args, .. } => {
            // Phase 4a-broad — recognize `__poll_<inner>` calls
            // alongside the io_*_async family so the v3.1
            // transform fires when an async fn body only contains
            // sub-task polls (no other suspends).
            if name.starts_with("__poll_") || matches!(
                name.as_str(),
                "io_recv_async" | "io_send_async" | "io_accept_async"
            ) {
                return true;
            }
            args.iter().any(expr_contains_io_async)
        }
        ExprKind::Binary { left, right, .. } => {
            expr_contains_io_async(left) || expr_contains_io_async(right)
        }
        ExprKind::Unary { expr, .. } | ExprKind::Cast { expr, .. } => {
            expr_contains_io_async(expr)
        }
        ExprKind::Index { array, index } => {
            expr_contains_io_async(array) || expr_contains_io_async(index)
        }
        ExprKind::Len { array } => expr_contains_io_async(array),
        ExprKind::Ref { inner } | ExprKind::RefMut { inner } => {
            expr_contains_io_async(inner)
        }
        ExprKind::ArrayLit { elements } | ExprKind::Tuple(elements) => {
            elements.iter().any(expr_contains_io_async)
        }
        ExprKind::TupleAccess { tuple, .. } => expr_contains_io_async(tuple),
        ExprKind::StructLit { fields, .. } => {
            fields.iter().any(|(_, v)| expr_contains_io_async(v))
        }
        ExprKind::FieldAccess { object, .. } => expr_contains_io_async(object),
        ExprKind::MethodCall { receiver, args, .. } => {
            expr_contains_io_async(receiver) || args.iter().any(expr_contains_io_async)
        }
        // Phase 2.3-narrow: walk Match scrutinee + arm bodies
        // so the v3.1 transform sees suspends nested inside
        // match arms (currently rejected with a Phase 2.3a
        // pointer; future phase adds per-arm state graphs).
        ExprKind::Match { scrutinee, arms } => {
            expr_contains_io_async(scrutinee)
                || arms.iter().any(|a| expr_contains_io_async(&a.body))
        }
        _ => false,
    }
}

/// Recursively check whether a Stmt (or any nested Stmt /
/// Expr) contains an `io_*_async` call. Used by Phase 2's
/// validator to gate `if` / `while` constructs.
fn stmt_contains_io_async(s: &Stmt) -> bool {
    match s {
        Stmt::Let { expr, .. }
        | Stmt::LetTuple { expr, .. }
        | Stmt::Return { expr, .. }
        | Stmt::Assert { expr, .. }
        | Stmt::Prove { expr, .. }
        | Stmt::Assign { expr, .. } => expr_contains_io_async(expr),
        Stmt::Print { items, .. } => items.iter().any(|it| match it {
            crate::ast::PrintItem::Expr(e) => expr_contains_io_async(e),
            crate::ast::PrintItem::Str(_) => false,
        }),
        Stmt::If { cond, then_body, else_body, .. } => {
            expr_contains_io_async(cond)
                || then_body.iter().any(stmt_contains_io_async)
                || else_body.iter().any(stmt_contains_io_async)
        }
        Stmt::While { cond, body, .. } => {
            expr_contains_io_async(cond) || body.iter().any(stmt_contains_io_async)
        }
        Stmt::IndexAssign { index, value, .. } => {
            expr_contains_io_async(index) || expr_contains_io_async(value)
        }
        Stmt::FieldAssign { object, value, .. } => {
            expr_contains_io_async(object) || expr_contains_io_async(value)
        }
        _ => false,
    }
}

/// Phase 2.2 — ANF lift: walk an expression, lifting EVERY
/// `io_*_async(args)` call (even at the top level) into a
/// fresh `__anf_N` Let. Returns (lifted_lets, rewritten_expr).
/// The lifted_lets must be emitted BEFORE the containing
/// statement.
///
/// **Callers MUST decide whether the top-level expr is
/// already at a valid suspend-point position** (direct RHS
/// of a `Let`) and skip calling this fn in that case.
/// `anf_lift_body` handles that distinction.
fn anf_lift_expr(expr: &Expr, counter: &mut usize) -> (Vec<Stmt>, Expr) {
    // Walk subexpressions, lifting nested io_*_async calls.
    match &expr.kind {
        ExprKind::Call { name, name_span, args } => {
            let mut all_lifts = Vec::new();
            let mut new_args = Vec::new();
            for arg in args {
                let (lifts, new_arg) = anf_lift_expr(arg, counter);
                all_lifts.extend(lifts);
                new_args.push(new_arg);
            }
            let new_call = Expr {
                kind: ExprKind::Call {
                    name: name.clone(),
                    name_span: *name_span,
                    args: new_args,
                },
                span: expr.span,
            };
            // If THIS call is io_*_async, lift it now (nested
            // case — caller's expr isn't a direct io_*_async
            // call but this sub-expr is).
            if matches!(
                name.as_str(),
                "io_recv_async" | "io_send_async" | "io_accept_async"
            ) {
                let synth_name = format!("__anf_{}", *counter);
                *counter += 1;
                let lifted_let = Stmt::Let {
                    name: synth_name.clone(),
                    annotation: Some(Type::I64),
                    expr: new_call,
                    span: expr.span,
                };
                all_lifts.push(lifted_let);
                return (
                    all_lifts,
                    Expr {
                        kind: ExprKind::Var(synth_name),
                        span: expr.span,
                    },
                );
            }
            (all_lifts, new_call)
        }
        ExprKind::Binary { op, left, right } => {
            let (mut left_lifts, left_new) = anf_lift_expr(left, counter);
            let (right_lifts, right_new) = anf_lift_expr(right, counter);
            left_lifts.extend(right_lifts);
            (
                left_lifts,
                Expr {
                    kind: ExprKind::Binary {
                        op: *op,
                        left: Box::new(left_new),
                        right: Box::new(right_new),
                    },
                    span: expr.span,
                },
            )
        }
        ExprKind::Unary { op, expr: e } => {
            let (lifts, new_e) = anf_lift_expr(e, counter);
            (
                lifts,
                Expr {
                    kind: ExprKind::Unary {
                        op: *op,
                        expr: Box::new(new_e),
                    },
                    span: expr.span,
                },
            )
        }
        ExprKind::Cast { expr: e, ty } => {
            let (lifts, new_e) = anf_lift_expr(e, counter);
            (
                lifts,
                Expr {
                    kind: ExprKind::Cast {
                        expr: Box::new(new_e),
                        ty: ty.clone(),
                    },
                    span: expr.span,
                },
            )
        }
        // Other expr shapes don't typically contain io_*_async
        // in v3.1 narrow scope; return unchanged.
        _ => (vec![], expr.clone()),
    }
}

/// Phase 2.3a — desugar a `let X = match SCRUT { p0 -> e0,
/// ..., _ -> ed }` where SOME arm contains an io_*_async
/// suspend into a sequence of:
///   let X: i64 = 0;          // placeholder
///   if SCRUT == p0 { X = e0; }
///   else if SCRUT == p1 { X = e1; }
///   ...
///   else { X = ed; }
///
/// Then ANF + Phase 2.1a/b/c machinery handles the rest.
/// Returns None if the match isn't desugarable (non-i64
/// pattern, missing wildcard, etc.).
///
/// Supported in Phase 2.3a:
/// - Pattern::Int(literal) arms
/// - Pattern::Wildcard catch-all (required as last arm)
/// - Arm bodies that produce i64 values
///
/// Phase 2.3b extends literal-pattern support to:
/// - Pattern::Bool — `match b { true then ..., false then ... }`
/// - Pattern::Str  — `match s { "foo" then ..., _ then ... }`
/// - Pattern::Float — `match f { 3.14 then ..., _ then ... }`
///
/// Phase 2.3c extends to:
/// - Pattern::Variant — `match c { Color.Red then ..., _ then ... }`
///   Lowered via a tag-extraction sub-match: the original variant
///   arms become an i64-returning match (`Color.Red then 0,
///   Color.Green then 1, ...`) that runs via the existing
///   variant-tag dispatch; then the if-chain dispatches on the
///   synthesized i64 tag.
///
/// Rejected (deferred to Phase 2.3d):
/// - Pattern::VariantWithBinding (binding scope across the
///   tag-extraction → if-chain split needs explicit relifting)
/// - Mixed Variant + non-Variant patterns in same match
/// - Wildcard not at the end
fn try_desugar_let_match_with_suspends(
    name: &str,
    annotation: &Option<Type>,
    expr: &Expr,
    span: crate::span::Span,
) -> Option<Vec<Stmt>> {
    // Only desugar if expr is a Match AND has suspends.
    let (scrutinee, arms) = match &expr.kind {
        ExprKind::Match { scrutinee, arms } => (scrutinee, arms),
        _ => return None,
    };
    if !expr_contains_io_async(expr) {
        return None; // No suspends — Phase 2.3-narrow handles it.
    }
    // Detect Variant / VariantWithBinding arms. If ANY arm is
    // Variant-shaped, route through the tag-extraction sub-match
    // path. Mixed Variant + non-Variant patterns bail (would
    // mismatch the sub-match's scrutinee type).
    let has_variant = arms.iter().any(|a| {
        matches!(
            &a.pattern,
            crate::ast::Pattern::Variant { .. }
                | crate::ast::Pattern::VariantWithBinding { .. }
        )
    });
    if has_variant {
        return try_desugar_match_via_tag_extraction(
            name, annotation, scrutinee, arms, span,
        );
    }
    // Validate arms: Pattern::Int | Bool | Str | Float OR
    // trailing Pattern::Wildcard.
    //
    // Each lit_arm stores the pre-built comparison literal Expr
    // alongside the arm body, so per-pattern shape is handled
    // once at validation and the nesting loop stays uniform.
    let mut lit_arms: Vec<(Expr, Expr, crate::span::Span)> = Vec::new();
    let mut default_arm: Option<(Expr, crate::span::Span)> = None;
    for (i, arm) in arms.iter().enumerate() {
        let is_last = i == arms.len() - 1;
        let lit_expr_opt: Option<ExprKind> = match &arm.pattern {
            crate::ast::Pattern::Int(lit) => Some(ExprKind::Int(*lit)),
            crate::ast::Pattern::Bool(b) => Some(ExprKind::Bool(*b)),
            crate::ast::Pattern::Str(s) => Some(ExprKind::Str(s.clone())),
            crate::ast::Pattern::Float(f) => Some(ExprKind::Float(*f)),
            crate::ast::Pattern::Wildcard => {
                if !is_last {
                    return None; // Wildcard must be last.
                }
                default_arm = Some((arm.body.clone(), arm.pattern_span));
                None
            }
            // Variant handled above; VariantWithBinding bailed above.
            _ => return None,
        };
        if let Some(lit_kind) = lit_expr_opt {
            let lit_expr = Expr {
                kind: lit_kind,
                span: arm.pattern_span,
            };
            lit_arms.push((lit_expr, arm.body.clone(), arm.pattern_span));
        }
    }
    // Build the placeholder Let + if-else-assign chain.
    let mut out = Vec::new();
    out.push(Stmt::Let {
        name: name.to_string(),
        annotation: annotation.clone(),
        expr: Expr { kind: ExprKind::Int(0), span },
        span,
    });

    // Build innermost else first (default arm or no-op).
    let mut current_else: Vec<Stmt> = if let Some((dexpr, dspan)) = default_arm {
        vec![Stmt::Assign {
            name: name.to_string(),
            expr: dexpr,
            span: dspan,
        }]
    } else {
        Vec::new()
    };

    // Walk lit_arms in REVERSE to nest if-else properly.
    for (lit_expr, arm_expr, arm_span) in lit_arms.into_iter().rev() {
        let cond = Expr {
            kind: ExprKind::Binary {
                op: BinaryOp::Eq,
                left: Box::new((**scrutinee).clone()),
                right: Box::new(lit_expr),
            },
            span: arm_span,
        };
        let then_body = vec![Stmt::Assign {
            name: name.to_string(),
            expr: arm_expr,
            span: arm_span,
        }];
        current_else = vec![Stmt::If {
            cond,
            then_body,
            else_body: std::mem::take(&mut current_else),
            span: arm_span,
        }];
    }
    out.extend(current_else);
    Some(out)
}

/// Phase 2.3c — desugar `let X = match SCRUT { Color.Red then EXPR0,
/// Color.Green then EXPR1, ..., _ then EXPRD };` (with suspends in
/// any arm body) into:
///
/// ```vani
/// let X: i64 = 0;
/// let __match_tag_X: i64 = match SCRUT {
///   Color.Red then 0,
///   Color.Green then 1,
///   ...
///   _ then K
/// };
/// if __match_tag_X == 0 { X = EXPR0; }
/// else if __match_tag_X == 1 { X = EXPR1; }
/// ...
/// else { X = EXPRD; }
/// ```
///
/// The tag-extraction sub-match runs through the existing
/// variant-dispatch path (Phase 2.3-narrow handles match in async
/// fn when no arm suspends — and the tag arms here produce i64
/// literals, so none suspend). The if-chain then handles the
/// per-arm suspending bodies via Phase 2.1a/b's state-splitting.
///
/// Supports `Pattern::Variant` (unit variants, no payload) and
/// `Pattern::VariantWithBinding` (single-binding payload). For
/// VariantWithBinding arms, additionally prepends a re-extraction
/// `let <binding>: i64 = match SCRUT { Pat.Variant(<binding>) then
/// <binding>, _ then 0 };` at the start of the corresponding if-
/// branch so the arm body's references to the binding resolve.
///
/// Caveat: Variant scrutinees must be inline (v3.1 i64-only locals
/// rule). The SCRUT is re-evaluated once per VariantWithBinding
/// arm — fine for pure helper-fn scrutinees, surprising for
/// impure ones. The user can hoist impure scrutinees into a helper
/// fn that returns the variant.
///
/// Caveat: bindings must be i64 typed (v3.1's i64-only locals
/// rule). Non-i64 payloads (e.g. `Result.Err(Str)`) reach the
/// checker with the user's intended payload type but the
/// synthesized `let <binding>: i64` annotation causes a checker
/// type-mismatch — Phase 3 will lift this once non-i64 locals
/// across await are supported.
fn try_desugar_match_via_tag_extraction(
    name: &str,
    annotation: &Option<Type>,
    scrutinee: &Box<Expr>,
    arms: &[crate::ast::MatchArm],
    span: crate::span::Span,
) -> Option<Vec<Stmt>> {
    let mut variant_arms: Vec<(crate::ast::Pattern, Expr, crate::span::Span)> = Vec::new();
    let mut default_arm: Option<(Expr, crate::span::Span)> = None;
    for (i, arm) in arms.iter().enumerate() {
        let is_last = i == arms.len() - 1;
        match &arm.pattern {
            crate::ast::Pattern::Variant { .. }
            | crate::ast::Pattern::VariantWithBinding { .. } => {
                variant_arms.push((
                    arm.pattern.clone(),
                    arm.body.clone(),
                    arm.pattern_span,
                ));
            }
            crate::ast::Pattern::Wildcard => {
                if !is_last {
                    return None;
                }
                default_arm = Some((arm.body.clone(), arm.pattern_span));
            }
            // Mixed shapes bail.
            _ => return None,
        }
    }

    // Build the tag-extraction match arms: each variant arm
    // maps to its index (0..N-1); wildcard maps to N. For
    // VariantWithBinding arms, the binding is preserved in the
    // tag-extraction pattern but the arm body just returns the
    // tag literal — the binding is unused at this step.
    let tag_local = format!("__match_tag_{}", name);
    let mut tag_match_arms: Vec<crate::ast::MatchArm> = Vec::new();
    for (idx, (pat, _body, pat_span)) in variant_arms.iter().enumerate() {
        tag_match_arms.push(crate::ast::MatchArm {
            pattern: pat.clone(),
            pattern_span: *pat_span,
            body: Expr {
                kind: ExprKind::Int(idx as i128),
                span: *pat_span,
            },
        });
    }
    let wildcard_tag = variant_arms.len() as i128;
    if default_arm.is_some() {
        tag_match_arms.push(crate::ast::MatchArm {
            pattern: crate::ast::Pattern::Wildcard,
            pattern_span: span,
            body: Expr {
                kind: ExprKind::Int(wildcard_tag),
                span,
            },
        });
    }
    let tag_match_expr = Expr {
        kind: ExprKind::Match {
            scrutinee: scrutinee.clone(),
            arms: tag_match_arms,
        },
        span,
    };

    // Emit: let X: i64 = 0; let __match_tag_X: i64 = <tag match>;
    let mut out = Vec::new();
    out.push(Stmt::Let {
        name: name.to_string(),
        annotation: annotation.clone(),
        expr: Expr { kind: ExprKind::Int(0), span },
        span,
    });
    out.push(Stmt::Let {
        name: tag_local.clone(),
        annotation: Some(Type::I64),
        expr: tag_match_expr,
        span,
    });

    // Build innermost else first.
    let mut current_else: Vec<Stmt> = if let Some((dexpr, dspan)) = default_arm {
        vec![Stmt::Assign {
            name: name.to_string(),
            expr: dexpr,
            span: dspan,
        }]
    } else {
        Vec::new()
    };

    // Walk variant_arms in REVERSE to nest if-else properly.
    for (idx, (pat, arm_expr, arm_span)) in variant_arms.into_iter().enumerate().rev() {
        let tag_var = Expr {
            kind: ExprKind::Var(tag_local.clone()),
            span: arm_span,
        };
        let lit = Expr {
            kind: ExprKind::Int(idx as i128),
            span: arm_span,
        };
        let cond = Expr {
            kind: ExprKind::Binary {
                op: BinaryOp::Eq,
                left: Box::new(tag_var),
                right: Box::new(lit),
            },
            span: arm_span,
        };

        // For VariantWithBinding, SUBSTITUTE `Var(binding)` in the
        // arm body with an inline re-extraction match expression.
        // We can't use an inner LET because branch-local LETs that
        // collide with task struct field names create a disconnect:
        // the local LET writes to a local, but subsequent reads
        // get rewritten to `__t.<name>` (uninitialized field).
        // Substituting inline sidesteps the LET/field collision.
        //
        // The inner pattern binding uses a fresh `__pat_<binding>`
        // name to keep it out of the task struct's rename set (so
        // the inner match's arm body's read of the payload isn't
        // rewritten).
        //
        // Tradeoff: a binding used N times in the arm body causes
        // N re-evaluations of the scrutinee. Fine for pure helper
        // fn scrutinees, which is the typical pattern.
        let effective_arm_expr = if let crate::ast::Pattern::VariantWithBinding {
            enum_name,
            variant,
            binding,
        } = &pat
        {
            let pat_binding = format!("__pat_{}", binding);
            let extract_arms = vec![
                crate::ast::MatchArm {
                    pattern: crate::ast::Pattern::VariantWithBinding {
                        enum_name: enum_name.clone(),
                        variant: variant.clone(),
                        binding: pat_binding.clone(),
                    },
                    pattern_span: arm_span,
                    body: Expr {
                        kind: ExprKind::Var(pat_binding.clone()),
                        span: arm_span,
                    },
                },
                crate::ast::MatchArm {
                    pattern: crate::ast::Pattern::Wildcard,
                    pattern_span: arm_span,
                    body: Expr {
                        kind: ExprKind::Int(0),
                        span: arm_span,
                    },
                },
            ];
            let extract_match = Expr {
                kind: ExprKind::Match {
                    scrutinee: scrutinee.clone(),
                    arms: extract_arms,
                },
                span: arm_span,
            };
            substitute_var_in_expr(&arm_expr, binding, &extract_match)
        } else {
            arm_expr
        };
        let then_body = vec![Stmt::Assign {
            name: name.to_string(),
            expr: effective_arm_expr,
            span: arm_span,
        }];
        current_else = vec![Stmt::If {
            cond,
            then_body,
            else_body: std::mem::take(&mut current_else),
            span: arm_span,
        }];
    }
    out.extend(current_else);
    Some(out)
}

/// Phase 2.2 — ANF lift a body of statements. Walks each
/// stmt, lifting nested io_*_async calls out of expressions
/// into fresh Let bindings emitted BEFORE the original stmt.
/// Recurses into nested if/while branches. Phase 2.3a
/// desugars `let X = match {...}` with suspending arms into
/// Whether `ty` is permitted as a v3.1 async fn local type. The
/// hard requirement is that the constructor can emit a sensible
/// default-init Expr (since locals start at 0 / false / "" / first-
/// variant before being assigned by the state machine).
///
/// Currently allows:
///   - I64        — default-inits to `0`              (baseline)
///   - Bool       — default-inits to `false`          (Phase 3a)
///   - F64        — default-inits to `0.0`            (Phase 3a)
///   - Str        — default-inits to `""`             (Phase 3b)
///   - OwnedStr   — default-inits to `""`             (Phase 3b)
///   - Enum(name) — default-inits to first UNIT       (Phase 3c)
///                  variant of the enum. Requires the enum decl
///                  to be visible to the parser (lexically before
///                  the async fn) via V31_ENUM_REGISTRY AND to
///                  have at least one unit variant.
///
/// Types deferred to later Phase 3 sub-phases:
///   - Struct(name) / Vec / Array (Phase 3d)
fn v31_local_type_allowed(ty: &Type) -> bool {
    v31_local_type_allowed_with_params(ty, &[])
}

/// Phase 4c-broad — generic-aware variant. When `type_params` is
/// non-empty, accepts `Type::Param(name)` whose `name` matches
/// one of the enclosing fn's declared type params. This lifts
/// the narrow-gate rejection so generic async fns reach the
/// v3.1 synthesizer. Recurses with the same `type_params` for
/// container types (Vec<T>, Array<T,N>) so a Vec<T> param
/// type allows when T is in type_params.
fn v31_local_type_allowed_with_params(ty: &Type, type_params: &[String]) -> bool {
    match ty {
        // Phase 4c-broad: accept generic type params.
        Type::Param(name) => type_params.iter().any(|t| t == name),
        Type::I64 | Type::Bool | Type::F64 | Type::Str | Type::OwnedStr => true,
        // Parser stamps user-typed identifiers as Type::Struct(name)
        // before the checker's pre-pass resolves them to Type::Enum.
        // Try enum registry first; fall back to struct registry
        // (Phase 3d). If neither has the name, reject.
        Type::Enum(name) | Type::Struct(name) => {
            if v31_enum_default_variant(name).is_some() {
                return true;
            }
            // Phase 3d — struct local: allowed only if every field
            // type is itself v31-allowed (recursive). Recursion
            // tracking via STRUCT_VISITING_DEFAULT prevents infinite
            // loops on self-referential structs.
            v31_struct_default_allowed(name)
        }
        // Phase 3d — Vec<T>: element type must also be v31-
        // allowed since the default-init synthesizes a 1-element
        // vec containing the element's default (the empty `vec()`
        // can't infer its element type in StructLit field
        // position).
        //
        // C backend Vec<Struct(name)> case fixed 2026-06-06
        // (commit 4feb5fc) via unified topological emit across
        // struct typedefs + Vec bundles. No backend-side
        // workaround needed.
        Type::Vec(inner) => v31_local_type_allowed_with_params(inner, type_params),
        // Phase 3f — [T; N]: element type must be v31-allowed
        // (default-init emits N copies of default(T) as an
        // ArrayLit). The C backend's FieldAssign routes Array
        // values through memcpy instead of `=` so the synthesizer's
        // NonSuspendLet temp-and-FieldAssign pattern works
        // uniformly. Forbid zero-length arrays as a defensive
        // guard (empty ArrayLit can't infer in StructLit field
        // position).
        Type::Array { element, length } => {
            *length > 0 && v31_local_type_allowed_with_params(element, type_params)
        }
        _ => false,
    }
}

/// Phase 3d — Whether a struct type can have its default-init
/// synthesized. Requires every field type to be v31-allowed
/// (recursive). Uses a thread-local visited set to break cycles
/// on self-referential structs.
fn v31_struct_default_allowed(struct_name: &str) -> bool {
    thread_local! {
        static VISITING: std::cell::RefCell<std::collections::HashSet<String>> =
            std::cell::RefCell::new(std::collections::HashSet::new());
    }
    VISITING.with(|v| {
        if v.borrow().contains(struct_name) {
            // Cycle — assume disallowed (self-referential structs
            // can't have a finite default).
            return false;
        }
        let fields_opt = crate::ast::V31_STRUCT_REGISTRY.with(|reg| {
            reg.borrow().get(struct_name).cloned()
        });
        let Some(fields) = fields_opt else {
            return false;
        };
        v.borrow_mut().insert(struct_name.to_string());
        let ok = fields.iter().all(|(_, ty)| v31_local_type_allowed(ty));
        v.borrow_mut().remove(struct_name);
        ok
    })
}

/// Phase 3c/3e — Look up the first variant of `enum_name` whose
/// payload types are ALL v3.1-allowed (i.e. each payload type
/// `v31_local_type_allowed`). Returns the variant name and its
/// payload types. Unit variants (empty payload) are preferred
/// since they emit a simple FieldAccess; payloaded variants
/// (Phase 3e) emit a MethodCall constructor with each payload
/// recursively default-inited.
///
/// Returns `None` if the enum isn't registered (not lexically
/// visible before the async fn) OR no variant has fully-allowed
/// payloads.
fn v31_enum_default_variant(enum_name: &str) -> Option<(String, Vec<Type>)> {
    crate::ast::V31_ENUM_REGISTRY.with(|reg| {
        let reg_ref = reg.borrow();
        let variants = reg_ref.get(enum_name)?;
        // Prefer unit variants first.
        if let Some((name, _)) = variants.iter().find(|(_, p)| p.is_empty()) {
            return Some((name.clone(), Vec::new()));
        }
        // Phase 3e — fall back to a payloaded variant whose
        // payload types are all v31-allowed.
        variants
            .iter()
            .find(|(_, p)| p.iter().all(v31_local_type_allowed))
            .map(|(name, payload)| (name.clone(), payload.clone()))
    })
}

/// Build a default-init Expr for a v3.1 local field type. Caller
/// has already validated via `v31_local_type_allowed`.
fn v31_default_init_expr(ty: &Type, span: crate::span::Span) -> Expr {
    let kind = match ty {
        Type::I64 => ExprKind::Int(0),
        Type::Bool => ExprKind::Bool(false),
        Type::F64 => ExprKind::Float(0.0),
        Type::Str => ExprKind::Str(String::new()),
        // v3.1 caveat #2 polish (2026-06-08): OwnedStr can't be
        // default-init'd as `ExprKind::Str("")` because that
        // types as Str (not OwnedStr), and `can_assign` doesn't
        // accept Str → OwnedStr in struct-lit field position.
        // Synthesize `"" + ""` instead — the Binary-Add over two
        // Str literals dispatches through `check_str_concat`
        // which returns Type::OwnedStr (heap-allocating
        // `intent_str_concat` of two empty literals; the cost
        // is a single malloc(1) at task construction). Task
        // bodies overwrite this default before any read so the
        // throwaway allocation is harmless under the existing
        // affine-Drop discipline.
        Type::OwnedStr => ExprKind::Binary {
            op: crate::ast::BinaryOp::Add,
            left: Box::new(Expr {
                kind: ExprKind::Str(String::new()),
                span,
            }),
            right: Box::new(Expr {
                kind: ExprKind::Str(String::new()),
                span,
            }),
        },
        // Phase 3d — Vec<T>: 1-element vec containing the
        // element's recursive default. An empty `vec()` would
        // typecheck-fail in StructLit field position (the checker
        // can't infer the element type without a context-aware
        // annotation hook). One default element supplies the
        // inference; the state machine overwrites the field with
        // the user's actual vec() before any read, so the
        // throwaway default is harmless.
        Type::Vec(inner) => ExprKind::Call {
            name: "vec".to_string(),
            name_span: span,
            args: vec![v31_default_init_expr(inner, span)],
        },
        // Phase 3f — [T; N]: ArrayLit with N copies of default(T).
        // The literal `[d0, d1, ..., dN-1]` typechecks as `[T; N]`
        // when the field annotation matches. Validator already
        // ensured length > 0 so the literal isn't empty.
        Type::Array { element, length } => {
            let default = v31_default_init_expr(element, span);
            let elements: Vec<Expr> = (0..*length).map(|_| default.clone()).collect();
            ExprKind::ArrayLit { elements }
        }
        Type::Enum(name) | Type::Struct(name) => {
            // Phase 3d — if `name` is a registered struct (not an
            // enum), synthesize `StructName { f0: d0, f1: d1, ...
            // }` with recursive per-field defaults.
            let struct_fields = crate::ast::V31_STRUCT_REGISTRY.with(|reg| {
                reg.borrow().get(name).cloned()
            });
            if let Some(fields) = struct_fields {
                let init_fields: Vec<(String, Expr)> = fields
                    .iter()
                    .map(|(fname, fty)| {
                        (fname.clone(), v31_default_init_expr(fty, span))
                    })
                    .collect();
                return Expr {
                    kind: ExprKind::StructLit {
                        type_name: name.clone(),
                        type_name_span: span,
                        fields: init_fields,
                    },
                    span,
                };
            }
            // Phase 3c/3e — synthesize the default variant of the
            // registered enum. Unit variants emit `FieldAccess`
            // (`EnumName.Variant`); payloaded variants emit a
            // `MethodCall` constructor (`EnumName.Variant(d0, d1,
            // ...)`) where each `di` is the recursive default-init
            // for the payload type. The validator already ensured
            // a suitable variant exists; if not, we fall through
            // to `Int(0)` (unreachable in correct flow).
            if let Some((variant, payload_types)) = v31_enum_default_variant(name) {
                let enum_var = Box::new(Expr {
                    kind: ExprKind::Var(name.clone()),
                    span,
                });
                if payload_types.is_empty() {
                    // Unit variant — `EnumName.Variant`.
                    ExprKind::FieldAccess {
                        object: enum_var,
                        field: variant,
                    }
                } else {
                    // Phase 3e — payloaded variant. Synthesize
                    // `EnumName.Variant(d0, d1, ...)` as a
                    // MethodCall. Each payload arg recursively
                    // calls `v31_default_init_expr` for its type.
                    let args: Vec<Expr> = payload_types
                        .iter()
                        .map(|t| v31_default_init_expr(t, span))
                        .collect();
                    ExprKind::MethodCall {
                        receiver: enum_var,
                        method: variant,
                        method_span: span,
                        args,
                    }
                }
            } else {
                ExprKind::Int(0)
            }
        }
        _ => ExprKind::Int(0), // unreachable: validator gates the type
    };
    Expr { kind, span }
}

/// Phase 2.4 — desugar `let X: T = try EXPR;` in v3.1 async fn
/// bodies into a multi-stmt sequence that propagates Err early
/// and extracts the Ok payload. See `try_v31_transform` for the
/// full lowering shape.
///
/// The desugar requires:
///   - `return_type` is a registered enum (in V31_ENUM_REGISTRY)
///     OR `Type::Struct(name)` whose name is in the registry.
///   - The enum has at least one variant with a payload (the
///     "Ok" arm); we use its first payload type as T (so the
///     LET's annotation must agree).
///
/// Bails with a clean diagnostic if the return type isn't an
/// enum or the enum shape doesn't match.
fn desugar_try_in_v31_body(
    body: &[Stmt],
    return_type: &Type,
) -> Result<Vec<Stmt>, Diagnostic> {
    // Fast path: no `try` anywhere → return body unchanged.
    let has_try = body.iter().any(|s| match s {
        Stmt::Let { expr, .. } => matches!(expr.kind, ExprKind::Try { .. }),
        _ => false,
    });
    if !has_try {
        return Ok(body.to_vec());
    }
    // Resolve return type to an enum registered in V31_ENUM_REGISTRY.
    let enum_name = match return_type {
        Type::Enum(n) | Type::Struct(n) => n.clone(),
        _ => {
            let span = body.first().map(|s| match s {
                Stmt::Let { span, .. } => *span,
                _ => crate::span::Span::new(0, 0),
            }).unwrap_or(crate::span::Span::new(0, 0));
            return Err(Diagnostic::new(
                span,
                format!(
                    "v3.1 async fn uses `try` but return type {:?} is not an enum — \
                     `try` requires the async fn to return a Result-like enum",
                    return_type
                ),
            ));
        }
    };
    let variants = crate::ast::V31_ENUM_REGISTRY.with(|reg| reg.borrow().get(&enum_name).cloned());
    let Some(variants) = variants else {
        let span = body.first().map(|s| match s {
            Stmt::Let { span, .. } => *span,
            _ => crate::span::Span::new(0, 0),
        }).unwrap_or(crate::span::Span::new(0, 0));
        return Err(Diagnostic::new(
            span,
            format!(
                "v3.1 async fn uses `try` but return type `{}` is not a registered \
                 enum (must be lexically declared before the async fn)",
                enum_name
            ),
        ));
    };
    // Find the first payloaded variant — that's the "Ok" arm
    // (extract its payload as T).
    let ok_variant = variants.iter().find(|(_, p)| !p.is_empty());
    let Some((ok_name, _ok_payload)) = ok_variant else {
        let span = body.first().map(|s| match s {
            Stmt::Let { span, .. } => *span,
            _ => crate::span::Span::new(0, 0),
        }).unwrap_or(crate::span::Span::new(0, 0));
        return Err(Diagnostic::new(
            span,
            format!(
                "v3.1 async fn uses `try` but return type enum `{}` has no \
                 payloaded variant — `try` needs an Ok-like variant to extract",
                enum_name
            ),
        ));
    };
    let ok_name = ok_name.clone();

    let mut out: Vec<Stmt> = Vec::new();
    let mut counter: usize = 0;
    for s in body {
        match s {
            Stmt::Let {
                name: let_name,
                annotation,
                expr,
                span,
            } if matches!(expr.kind, ExprKind::Try { .. }) => {
                let ExprKind::Try { inner } = &expr.kind else { unreachable!() };
                let n = counter;
                counter += 1;
                let try_local = format!("__try_{}_{}", let_name, n);
                let try_local_span = *span;
                let pat_binding = format!("__pat_{}_{}", let_name, n);

                // 1) let __try_X_<n>: ResultType = EXPR;
                out.push(Stmt::Let {
                    name: try_local.clone(),
                    annotation: Some(return_type.clone()),
                    expr: (**inner).clone(),
                    span: try_local_span,
                });

                // 2) if (match __try_X_<n> { Enum.Ok(_) then 0, _ then 1 }) == 1 {
                //      return __try_X_<n>;
                //    }
                let is_err_match = Expr {
                    kind: ExprKind::Match {
                        scrutinee: Box::new(Expr {
                            kind: ExprKind::Var(try_local.clone()),
                            span: try_local_span,
                        }),
                        arms: vec![
                            crate::ast::MatchArm {
                                pattern: crate::ast::Pattern::VariantWithBinding {
                                    enum_name: enum_name.clone(),
                                    variant: ok_name.clone(),
                                    binding: format!("__discard_ok_{}", n),
                                },
                                pattern_span: try_local_span,
                                body: Expr { kind: ExprKind::Int(0), span: try_local_span },
                            },
                            crate::ast::MatchArm {
                                pattern: crate::ast::Pattern::Wildcard,
                                pattern_span: try_local_span,
                                body: Expr { kind: ExprKind::Int(1), span: try_local_span },
                            },
                        ],
                    },
                    span: try_local_span,
                };
                let cond = Expr {
                    kind: ExprKind::Binary {
                        op: BinaryOp::Eq,
                        left: Box::new(is_err_match),
                        right: Box::new(Expr { kind: ExprKind::Int(1), span: try_local_span }),
                    },
                    span: try_local_span,
                };
                // The early-return propagates the Err. For non-
                // i64 async fn return types, route through
                // `__t.__result` + `return 0` (mirrors what the
                // synthesizer's Seg::Return emits) so the
                // Verbatim-block processing doesn't hit the
                // i64-vs-T type mismatch. For i64 returns, plain
                // `return __try_X` is correct since the legacy
                // ABI returns the value directly.
                let early_return = if matches!(return_type, Type::I64) {
                    vec![Stmt::Return {
                        expr: Expr {
                            kind: ExprKind::Var(try_local.clone()),
                            span: try_local_span,
                        },
                        span: try_local_span,
                    }]
                } else {
                    vec![
                        Stmt::FieldAssign {
                            object: Expr {
                                kind: ExprKind::Var("__t".to_string()),
                                span: try_local_span,
                            },
                            field: "__result".to_string(),
                            field_span: try_local_span,
                            value: Expr {
                                kind: ExprKind::Var(try_local.clone()),
                                span: try_local_span,
                            },
                            span: try_local_span,
                        },
                        Stmt::Return {
                            expr: Expr { kind: ExprKind::Int(0), span: try_local_span },
                            span: try_local_span,
                        },
                    ]
                };
                out.push(Stmt::If {
                    cond,
                    then_body: early_return,
                    else_body: Vec::new(),
                    span: try_local_span,
                });

                // 3) let X: T = match __try_X_<n> { Enum.Ok(<pat>) then <pat>, _ then 0 };
                let extract_match = Expr {
                    kind: ExprKind::Match {
                        scrutinee: Box::new(Expr {
                            kind: ExprKind::Var(try_local.clone()),
                            span: try_local_span,
                        }),
                        arms: vec![
                            crate::ast::MatchArm {
                                pattern: crate::ast::Pattern::VariantWithBinding {
                                    enum_name: enum_name.clone(),
                                    variant: ok_name.clone(),
                                    binding: pat_binding.clone(),
                                },
                                pattern_span: try_local_span,
                                body: Expr {
                                    kind: ExprKind::Var(pat_binding.clone()),
                                    span: try_local_span,
                                },
                            },
                            crate::ast::MatchArm {
                                pattern: crate::ast::Pattern::Wildcard,
                                pattern_span: try_local_span,
                                body: Expr { kind: ExprKind::Int(0), span: try_local_span },
                            },
                        ],
                    },
                    span: try_local_span,
                };
                out.push(Stmt::Let {
                    name: let_name.clone(),
                    annotation: annotation.clone(),
                    expr: extract_match,
                    span: *span,
                });
            }
            other => out.push(other.clone()),
        }
    }
    Ok(out)
}

/// Substitute every `Var(target)` in `expr` with `replacement`.
/// Walks all common expression shapes (binary, unary, call,
/// match, method-call, tuple, struct lit, field access, etc).
/// Used by Phase 2.3d's VariantWithBinding desugar to inline a
/// re-extraction match in place of the user's binding.
fn substitute_var_in_expr(expr: &Expr, target: &str, replacement: &Expr) -> Expr {
    let kind = match &expr.kind {
        ExprKind::Var(name) if name == target => return replacement.clone(),
        ExprKind::Binary { op, left, right } => ExprKind::Binary {
            op: *op,
            left: Box::new(substitute_var_in_expr(left, target, replacement)),
            right: Box::new(substitute_var_in_expr(right, target, replacement)),
        },
        ExprKind::Unary { op, expr: e } => ExprKind::Unary {
            op: *op,
            expr: Box::new(substitute_var_in_expr(e, target, replacement)),
        },
        ExprKind::Cast { expr: e, ty } => ExprKind::Cast {
            expr: Box::new(substitute_var_in_expr(e, target, replacement)),
            ty: ty.clone(),
        },
        ExprKind::Call { name, name_span, args } => ExprKind::Call {
            name: name.clone(),
            name_span: *name_span,
            args: args.iter().map(|a| substitute_var_in_expr(a, target, replacement)).collect(),
        },
        ExprKind::Index { array, index } => ExprKind::Index {
            array: Box::new(substitute_var_in_expr(array, target, replacement)),
            index: Box::new(substitute_var_in_expr(index, target, replacement)),
        },
        ExprKind::Len { array } => ExprKind::Len {
            array: Box::new(substitute_var_in_expr(array, target, replacement)),
        },
        ExprKind::Ref { inner } => ExprKind::Ref {
            inner: Box::new(substitute_var_in_expr(inner, target, replacement)),
        },
        ExprKind::RefMut { inner } => ExprKind::RefMut {
            inner: Box::new(substitute_var_in_expr(inner, target, replacement)),
        },
        ExprKind::Match { scrutinee, arms } => ExprKind::Match {
            scrutinee: Box::new(substitute_var_in_expr(scrutinee, target, replacement)),
            arms: arms.iter().map(|a| crate::ast::MatchArm {
                pattern: a.pattern.clone(),
                pattern_span: a.pattern_span,
                body: substitute_var_in_expr(&a.body, target, replacement),
            }).collect(),
        },
        ExprKind::MethodCall { receiver, method, method_span, args } => ExprKind::MethodCall {
            receiver: Box::new(substitute_var_in_expr(receiver, target, replacement)),
            method: method.clone(),
            method_span: *method_span,
            args: args.iter().map(|a| substitute_var_in_expr(a, target, replacement)).collect(),
        },
        ExprKind::Tuple(elements) => ExprKind::Tuple(
            elements.iter().map(|e| substitute_var_in_expr(e, target, replacement)).collect(),
        ),
        ExprKind::TupleAccess { tuple, index } => ExprKind::TupleAccess {
            tuple: Box::new(substitute_var_in_expr(tuple, target, replacement)),
            index: *index,
        },
        ExprKind::FieldAccess { object, field } => ExprKind::FieldAccess {
            object: Box::new(substitute_var_in_expr(object, target, replacement)),
            field: field.clone(),
        },
        ExprKind::StructLit { type_name, type_name_span, fields } => ExprKind::StructLit {
            type_name: type_name.clone(),
            type_name_span: *type_name_span,
            fields: fields.iter().map(|(n, e)| {
                (n.clone(), substitute_var_in_expr(e, target, replacement))
            }).collect(),
        },
        ExprKind::ArrayLit { elements } => ExprKind::ArrayLit {
            elements: elements.iter().map(|e| substitute_var_in_expr(e, target, replacement)).collect(),
        },
        other => other.clone(),
    };
    Expr { kind, span: expr.span }
}

/// `let X = 0; if SCRUT == p0 { X = e0; } else if ...` BEFORE
/// running ANF on each piece.
fn anf_lift_body(body: &[Stmt], counter: &mut usize) -> Vec<Stmt> {
    let mut out = Vec::new();
    for s in body {
        match s {
            Stmt::Let { name, annotation, expr, span } => {
                // Phase 2.3a: if this Let's RHS is a Match
                // expression with suspends in arms, desugar
                // FIRST (then ANF the desugared stmts).
                if let Some(desugared) = try_desugar_let_match_with_suspends(
                    name, annotation, expr, *span,
                ) {
                    // Recursively ANF the desugared stmts.
                    let lifted = anf_lift_body(&desugared, counter);
                    out.extend(lifted);
                    continue;
                }
                // If the Let's RHS is directly an io_*_async
                // call, it's already at a valid suspend-point
                // position — pass through unchanged (but still
                // ANF-lift its ARGS in case they contain
                // nested io_*_async calls).
                let is_direct_suspend = match &expr.kind {
                    ExprKind::Call { name: cname, .. } => matches!(
                        cname.as_str(),
                        "io_recv_async" | "io_send_async" | "io_accept_async"
                    ),
                    _ => false,
                };
                if is_direct_suspend {
                    if let ExprKind::Call { name: cname, name_span, args } = &expr.kind {
                        let mut all_lifts = Vec::new();
                        let mut new_args = Vec::new();
                        for arg in args {
                            let (lifts, new_arg) = anf_lift_expr(arg, counter);
                            all_lifts.extend(lifts);
                            new_args.push(new_arg);
                        }
                        out.extend(all_lifts);
                        out.push(Stmt::Let {
                            name: name.clone(),
                            annotation: annotation.clone(),
                            expr: Expr {
                                kind: ExprKind::Call {
                                    name: cname.clone(),
                                    name_span: *name_span,
                                    args: new_args,
                                },
                                span: expr.span,
                            },
                            span: *span,
                        });
                        continue;
                    }
                }
                let (lifts, new_expr) = anf_lift_expr(expr, counter);
                out.extend(lifts);
                out.push(Stmt::Let {
                    name: name.clone(),
                    annotation: annotation.clone(),
                    expr: new_expr,
                    span: *span,
                });
            }
            Stmt::Return { expr, span } => {
                let (lifts, new_expr) = anf_lift_expr(expr, counter);
                out.extend(lifts);
                out.push(Stmt::Return {
                    expr: new_expr,
                    span: *span,
                });
            }
            Stmt::Assign { name, expr, span } => {
                let (lifts, new_expr) = anf_lift_expr(expr, counter);
                out.extend(lifts);
                out.push(Stmt::Assign {
                    name: name.clone(),
                    expr: new_expr,
                    span: *span,
                });
            }
            Stmt::If { cond, then_body, else_body, span } => {
                let (cond_lifts, new_cond) = anf_lift_expr(cond, counter);
                out.extend(cond_lifts);
                let new_then = anf_lift_body(then_body, counter);
                let new_else = anf_lift_body(else_body, counter);
                out.push(Stmt::If {
                    cond: new_cond,
                    then_body: new_then,
                    else_body: new_else,
                    span: *span,
                });
            }
            // Print / While / other forms — pass through; for
            // Phase 2.2 narrow we assume io_*_async only
            // appears in Let RHS / Return / Assign / if-cond
            // contexts. Nested forms in Print items / While
            // conds would need further casework.
            other => out.push(other.clone()),
        }
    }
    out
}

/// Phase 2.5b — rewrite `break` / `continue` inside a
/// Verbatim Stmt::If's branches to state_tag jumps so they
/// target the ENCLOSING suspending loop (not the synthesized
/// `while true` wrapping the poll fn body). Recurses into
/// nested if branches but NOT into nested user `while` bodies
/// (those have their own loop semantics and break/continue
/// inside should target THAT while).
fn rewrite_break_continue_in_stmts(
    stmts: &[Stmt],
    loop_header: usize,
    post_loop: usize,
    t_param: &str,
) -> Vec<Stmt> {
    let mut out = Vec::new();
    for s in stmts {
        match s {
            Stmt::Break { span, .. } => {
                out.push(Stmt::FieldAssign {
                    object: Expr {
                        kind: ExprKind::Var(t_param.to_string()),
                        span: *span,
                    },
                    field: "state_tag".to_string(),
                    field_span: *span,
                    value: Expr {
                        kind: ExprKind::Int(post_loop as i128),
                        span: *span,
                    },
                    span: *span,
                });
                out.push(Stmt::Continue { label: None, span: *span });
            }
            Stmt::Continue { span, .. } => {
                out.push(Stmt::FieldAssign {
                    object: Expr {
                        kind: ExprKind::Var(t_param.to_string()),
                        span: *span,
                    },
                    field: "state_tag".to_string(),
                    field_span: *span,
                    value: Expr {
                        kind: ExprKind::Int(loop_header as i128),
                        span: *span,
                    },
                    span: *span,
                });
                out.push(Stmt::Continue { label: None, span: *span });
            }
            Stmt::If { cond, then_body, else_body, span } => {
                out.push(Stmt::If {
                    cond: cond.clone(),
                    then_body: rewrite_break_continue_in_stmts(
                        then_body, loop_header, post_loop, t_param,
                    ),
                    else_body: rewrite_break_continue_in_stmts(
                        else_body, loop_header, post_loop, t_param,
                    ),
                    span: *span,
                });
            }
            // Don't recurse into Stmt::While — break/continue
            // inside a USER's nested non-suspending while
            // target THAT while, not the enclosing suspending
            // one. Pass through verbatim.
            other => out.push(other.clone()),
        }
    }
    out
}

/// Phase 2.1c helper — does this body unconditionally exit
/// via Return on every control-flow path? Used to suppress
/// the defensive Jump emitted after a Verbatim stmt that
/// itself always returns (e.g., a non-suspending `if` whose
/// both branches return). Otherwise the Jump becomes dead
/// code and the language's reachability checker rejects it.
fn body_all_paths_return(body: &[Stmt]) -> bool {
    match body.last() {
        Some(Stmt::Return { .. }) => true,
        Some(Stmt::If { then_body, else_body, .. }) => {
            !else_body.is_empty()
                && body_all_paths_return(then_body)
                && body_all_paths_return(else_body)
        }
        _ => false,
    }
}

/// Phase 2.1c — recursively collect i64 Let bindings from a
/// branch body INCLUDING nested if branches. Each binding
/// found at any depth needs a field in the task struct so
/// the state machine can persist it across suspends.
fn collect_branch_locals_recursive(
    body: &[Stmt],
    out: &mut Vec<(String, Type, crate::span::Span)>,
) {
    for s in body {
        match s {
            Stmt::Let { name, annotation, span, .. } => {
                let ty = annotation.clone().unwrap_or(Type::I64);
                // Phase 3a — accept i64 / bool / f64; other types
                // would have been rejected by the branch validator
                // already, so reaching here with a non-allowed type
                // is a no-op.
                if name != "_" && v31_local_type_allowed(&ty) {
                    out.push((name.clone(), ty, *span));
                }
            }
            Stmt::If { then_body, else_body, .. } => {
                collect_branch_locals_recursive(then_body, out);
                collect_branch_locals_recursive(else_body, out);
            }
            _ => {}
        }
    }
}

/// Phase 2.1a + 2.1b branch validator. Each branch of an
/// if-with-suspend must:
/// - Be linear (Let / Discard / Return only — no nested if /
///   while / match / break / continue)
/// - Each Let must be i64
/// - Phase 2.1a required Return at end; **Phase 2.1b lifts
///   this** — branches can fall through to a merge state
///   that the segment collector allocates after both branch
///   recursions.
///
/// On failure, returns a diagnostic pointing at the offending
/// stmt with a clear phase pointer.
fn validate_v31_phase_21a_branch(
    body: &[Stmt],
    branch_label: &str,
    _deferred_phase: &str,
) -> Result<(), Diagnostic> {
    // Phase 2.1b: empty branches are allowed (treated as
    // straight fall-through to the merge state).
    for s in body.iter() {
        match s {
            Stmt::Let { name, annotation, span, .. } => {
                let ty = annotation.clone().unwrap_or(Type::I64);
                if !v31_local_type_allowed(&ty) {
                    return Err(Diagnostic::new(
                        *span,
                        format!(
                            "v3.1 async fn: {} local '{}' has type {:?} — accepted types: i64 / bool / f64 / Str / OwnedStr / registered Enum / registered Struct / Vec<T> / Array<T,N> (Phase 4c-broad generic locals not yet supported inside branches)",
                            branch_label, name, ty
                        ),
                    ));
                }
            }
            Stmt::Return { .. } => {}
            Stmt::Print { items, .. } => {
                // Phase 2.1c+: Print is allowed inside
                // suspending branches if no print item
                // contains a suspend (ANF doesn't yet lift
                // inside print items).
                for it in items {
                    if let crate::ast::PrintItem::Expr(e) = it {
                        if expr_contains_io_async(e) {
                            return Err(Diagnostic::new(
                                crate::span::Span::new(0, 0),
                                format!(
                                    "v3.1 async fn: {} has a `print` item with a suspend — Phase 2.2 ANF doesn't lift into print items yet",
                                    branch_label
                                ),
                            ));
                        }
                    }
                }
            }
            Stmt::Assign { expr, span, .. } => {
                // Phase 2.1c+: Assign is allowed inside
                // suspending branches when RHS doesn't
                // suspend (Phase 2.2 ANF lifts such cases).
                if expr_contains_io_async(expr) {
                    return Err(Diagnostic::new(
                        *span,
                        format!(
                            "v3.1 async fn: {} has an assign whose RHS contains a suspend (Phase 2.2 ANF should have lifted it)",
                            branch_label
                        ),
                    ));
                }
            }
            Stmt::If { cond, then_body, else_body, span } => {
                // Phase 2.1c: nested ifs inside a suspending
                // branch. The recursive `collect_into` in the
                // segment collector already handles state-
                // splitting at arbitrary depth — we just need
                // to recursively validate the nested branches
                // here so non-i64 / break-continue / unsupported
                // shapes inside them still get diagnosed.
                if expr_contains_io_async(cond) {
                    return Err(Diagnostic::new(
                        *span,
                        format!(
                            "v3.1 async fn: nested `if` in {} has a suspend in its condition — needs ANF lifting (Phase 2.2)",
                            branch_label
                        ),
                    ));
                }
                validate_v31_phase_21a_branch(
                    then_body, "nested if then-branch", "Phase 2.1c (deeper nesting)",
                )?;
                validate_v31_phase_21a_branch(
                    else_body, "nested if else-branch", "Phase 2.1c (deeper nesting)",
                )?;
            }
            Stmt::While { cond, body, span, .. } => {
                // Phase 2.5: allow nested while-with-suspend.
                // ANF lifting + collector's state allocation
                // handle it (back-edge Jump emitted at body
                // end; back-Jump triggers `continue` in the
                // while-true wrapping the poll fn body).
                if expr_contains_io_async(cond) {
                    return Err(Diagnostic::new(
                        *span,
                        format!(
                            "v3.1 async fn: nested `while` in {} has a suspend in its condition (Phase 2.2 ANF should have lifted it; report as a v3.1 bug)",
                            branch_label
                        ),
                    ));
                }
                validate_v31_phase_21a_branch(
                    body, "nested while body", "Phase 2.5b (break/continue inside suspending loops)",
                )?;
            }
            Stmt::Break { .. } | Stmt::Continue { .. } => {
                // Phase 2.5b: break/continue allowed inside
                // suspending branches/loops. Base language
                // validates loop context.
                let _ = branch_label;
            }
            _ => {
                return Err(Diagnostic::new(
                    crate::span::Span::new(0, 0),
                    format!(
                        "v3.1 async fn: {} contains an unsupported statement form for Phase 2.1a/b — see ARC8_V3_PLAN.md",
                        branch_label
                    ),
                ));
            }
        }
    }
    Ok(())
}

/// Phase 1+2 narrow-case eligibility check. Returns the
/// locals (in declaration order) on success; an error
/// diagnostic on rejection.
///
/// **Phase 1 (linear core):** Let / Return at top level
/// only, all-i64 params/locals, return i64.
/// **Phase 2 narrow (added 2026-06-04):** also accepts
/// Stmt::If / Stmt::While / Stmt::Assign / Stmt::Print /
/// mid-body Return at the top level — **provided** the
/// embedded if/while branches contain no `io_*_async` call.
/// Bodies that do contain suspends inside control flow are
/// rejected with a "suspend in branch — Phase 2.1" pointer.
fn validate_v31_linear_body(
    type_params: &[String],
    params: &[Param],
    body: &[Stmt],
    return_type: &Type,
) -> Result<Vec<(String, Type, crate::span::Span)>, Diagnostic> {
    // Phase 4c-broad: when the async fn declares type params,
    // type-allow checks accept Type::Param(name) for any
    // declared param. For non-generic async fns, type_params
    // is empty so behavior matches Phase 3.
    //
    // Phase 4c-broad — when return_type is a bare Type::Param,
    // we require at least one fn param to share the same
    // Type::Param so the synthesizer can seed the ctor's
    // `__result` default to that param's runtime value. Without
    // a matching seed, default-init for a generic T has no
    // well-typed value at template time. The smoke pattern
    // `fn identity<T>(fd: i64, x: T) -> T` satisfies this
    // (x: T seeds __result).
    if let Type::Param(ret_name) = return_type {
        let any_param_matches = params.iter().any(|p| {
            matches!(&p.ty, Type::Param(n) if n == ret_name)
        });
        if !any_param_matches {
            return Err(Diagnostic::new(
                body.first().map(|s| match s {
                    Stmt::Let { span, .. }
                    | Stmt::Return { span, .. } => *span,
                    _ => crate::span::Span::new(0, 0),
                }).unwrap_or(crate::span::Span::new(0, 0)),
                format!(
                    "v3.1 async fn with generic return type `{}` must \
                     have at least one parameter of the same generic \
                     type to seed the `__result` default at template \
                     time. The smoke pattern is `fn name<{}>(... x: {} \
                     ...) -> {}`.",
                    ret_name, ret_name, ret_name, ret_name
                ),
            ));
        }
    }
    // Phase 3-returns: return type must be v31-allowed. Non-i64
    // returns route through a synthesized `__result: T` field;
    // poll fn returns status (0 = Ready) instead of the value.
    if !v31_local_type_allowed_with_params(return_type, type_params) {
        return Err(Diagnostic::new(
            body.first().map(|s| match s {
                Stmt::Let { span, .. }
                | Stmt::Return { span, .. } => *span,
                _ => crate::span::Span::new(0, 0),
            }).unwrap_or(crate::span::Span::new(0, 0)),
            format!(
                "v3.1 async fn return type {:?} not yet supported \
                — Phase 3 accepts i64 / bool / f64 / Str / OwnedStr / \
                Enum (registered) / Vec<T> / Struct (registered) / \
                Array<T,N>. Generic returns (Type::Param) deferred \
                to Phase 4c-broad (see ARC8_V3_PLAN.md blocker notes).",
                return_type
            ),
        ));
    }
    // Phase 3-params (Phase 3p): params route through v31_local_
    // type_allowed.
    //
    // A4.4 (2026-06-08): also accept `ref Struct` / `mut ref Struct`
    // as a param type — the v3.1 Task struct field stores a raw
    // pointer to the borrowed struct (no lifetime tracking; user
    // is responsible for keeping the borrow alive across awaits,
    // same as v1 ref discipline elsewhere). This unblocks
    // `async fn foo(token: ref CancelToken) -> i64` which
    // composes with the CancelToken auto-plumbing pass to add
    // cancellation guards before every suspend point.
    //
    // Restrictions kept:
    //   - Ref/RefMut only for params (NOT locals — L4 stays for
    //     locals).
    //   - Inner type must be Struct(name) — no `ref i64`, no
    //     `ref Vec<T>`, no `ref Box<T>`. The narrow case
    //     CancelToken-shaped (a struct with bool / numeric
    //     fields) is the load-bearing one.
    fn param_ty_allowed(ty: &Type, type_params: &[String]) -> bool {
        if v31_local_type_allowed_with_params(ty, type_params) {
            return true;
        }
        match ty {
            Type::Ref(inner) | Type::RefMut(inner) => {
                matches!(&**inner, Type::Struct(_) | Type::Enum(_))
            }
            _ => false,
        }
    }
    for p in params {
        if !param_ty_allowed(&p.ty, type_params) {
            return Err(Diagnostic::new(
                p.span,
                format!(
                    "v3.1 async fn parameter '{}' has type {:?} — Phase 3 accepts i64 / bool / f64 / Str / OwnedStr / Enum (registered) / Vec<T> / Struct (registered) / Array<T,N>",
                    p.name, p.ty
                ),
            ));
        }
    }
    // Walk body. Collect Let locals. Phase 1 accepts Let +
    // Return at top level; Phase 2 narrow ALSO accepts if /
    // while / Assign / Print + mid-body Return — but only
    // when the embedded constructs don't contain io_*_async
    // (suspend-in-branch needs full state-splitting,
    // deferred to Phase 2.1).
    let mut locals: Vec<(String, Type, crate::span::Span)> = Vec::new();
    for s in body.iter() {
        match s {
            Stmt::Let { name, annotation, expr, span, .. } => {
                let ty = annotation.clone().unwrap_or(Type::I64);
                // Phase 4c-broad: reject locals whose type
                // contains a Type::Param. Unlike params (which
                // receive a runtime value from the caller) and
                // the return-via-`__result` slot (which the
                // validator routes through a same-typed param
                // for the initial seed), a fresh local has no
                // well-typed default for generic T at template
                // time. The synthesizer's `v31_default_init_expr`
                // can't produce one either.
                fn ty_has_param(ty: &Type) -> bool {
                    match ty {
                        Type::Param(_) => true,
                        Type::Vec(t) | Type::Ref(t) | Type::RefMut(t) => ty_has_param(t),
                        Type::Array { element, .. } => ty_has_param(element),
                        Type::Apply { args, .. } => args.iter().any(ty_has_param),
                        Type::Tuple(es) => es.iter().any(ty_has_param),
                        _ => false,
                    }
                }
                if ty_has_param(&ty) {
                    return Err(Diagnostic::new(
                        *span,
                        format!(
                            "v3.1 async fn local '{}' has type {:?} containing a generic Type::Param. \
                             Phase 4c-broad accepts Type::Param ONLY in params and return type — \
                             generic locals have no well-typed default value at template time.",
                            name, ty
                        ),
                    ));
                }
                if !v31_local_type_allowed_with_params(&ty, type_params) {
                    return Err(Diagnostic::new(
                        *span,
                        format!(
                            "v3.1 async fn local '{}' has type {:?} — accepted types: i64 / bool / f64 / Str / OwnedStr / registered Enum / registered Struct / Vec<T> / Array<T,N>. For generic async fns, locals stay concrete (Type::Param is allowed only in params and return type).",
                            name, ty
                        ),
                    ));
                }
                // Phase 2.3a: most match-with-suspending-arms
                // shapes are desugared by `anf_lift_body` BEFORE
                // this validator runs (Int / Wildcard patterns).
                // If we still see a `Match` here AND it contains
                // suspends, the desugar bailed because of an
                // unsupported pattern shape (Variant /
                // VariantWithBinding / Bool / Str). Catch with a
                // clear diagnostic pointing at the future
                // sub-phase.
                if let ExprKind::Match { .. } = &expr.kind {
                    if expr_contains_io_async(expr) {
                        return Err(Diagnostic::new(
                            *span,
                            "v3.1 async fn: `match` arm contains an `io_*_async` suspend with an unsupported pattern shape — Phase 2.3a desugars Int + Wildcard patterns; Variant/Bool/Str patterns arrive in a future sub-phase",
                        ));
                    }
                }
                // Skip `let _ = ...` (Discard) from locals — but
                // still process its RHS later in the state machine.
                if name != "_" {
                    locals.push((name.clone(), ty, *span));
                }
            }
            Stmt::Return { .. } => {
                // Phase 2 narrow: Return anywhere in the body
                // is fine. The synthesizer emits it in the
                // current state's arm and the driver loop sees
                // a non-Pending value, exiting the poll loop.
            }
            Stmt::Print { .. } => {
                // Phase 2 narrow: Print is allowed at top level
                // — emits in the current state's arm as a normal
                // side effect. (Print inside branches is also
                // allowed if the branch has no suspends.)
            }
            Stmt::Assign { name: assign_name, expr, span, .. } => {
                if expr_contains_io_async(expr) {
                    return Err(Diagnostic::new(
                        *span,
                        format!(
                            "v3.1 async fn: `{} = ...` RHS contains an `io_*_async` call (suspend point inside an expression — needs ANF lifting, arrives in Phase 2.2)",
                            assign_name
                        ),
                    ));
                }
                // Allowed: rewriter handles outer-scope name
                // assigns by emitting FieldAssign on t.<name>.
            }
            Stmt::If { cond, then_body, else_body, span } => {
                if expr_contains_io_async(cond) {
                    return Err(Diagnostic::new(
                        *span,
                        "v3.1 async fn: `if` condition contains an `io_*_async` call (suspend point inside an expression — needs ANF lifting, arrives in Phase 2.2)",
                    ));
                }
                let then_has_suspend = then_body.iter().any(stmt_contains_io_async);
                let else_has_suspend = else_body.iter().any(stmt_contains_io_async);
                if then_has_suspend || else_has_suspend {
                    // Phase 2.1a: both branches must be linear
                    // (no nested control flow) AND end with
                    // Return. Otherwise reject with phase-pointer.
                    validate_v31_phase_21a_branch(
                        then_body, "if then-branch", "Phase 2.1b (fall-through merge state)",
                    )?;
                    validate_v31_phase_21a_branch(
                        else_body, "if else-branch", "Phase 2.1b (fall-through merge state)",
                    )?;
                    // Collect branch Lets into the outer locals
                    // list so the task struct has fields for
                    // them. Phase 2.1c uses a recursive helper
                    // to also pick up Lets inside nested if
                    // branches — they need persistence too
                    // since the state machine spans nested
                    // suspends.
                    collect_branch_locals_recursive(then_body, &mut locals);
                    collect_branch_locals_recursive(else_body, &mut locals);
                }
            }
            Stmt::While { cond, body: while_body, span, .. } => {
                if expr_contains_io_async(cond) {
                    // Phase 2.2 ANF already handles this before
                    // reaching here, so this path is unlikely.
                    return Err(Diagnostic::new(
                        *span,
                        "v3.1 async fn: `while` condition contains an `io_*_async` call (Phase 2.2 ANF should have lifted it; report as a v3.1 bug)",
                    ));
                }
                if while_body.iter().any(stmt_contains_io_async) {
                    // Phase 2.5: validate body recursively
                    // (allows linear stmts + Lets + Return +
                    // nested ifs per Phase 2.1c rules). The
                    // collector handles state-splitting with
                    // a back-Jump for the loop edge.
                    validate_v31_phase_21a_branch(
                        while_body, "while body", "Phase 2.5b (break/continue inside suspending loops)",
                    )?;
                    // Also collect any Lets inside the loop
                    // body into the outer locals so the task
                    // struct has fields for them.
                    collect_branch_locals_recursive(while_body, &mut locals);
                }
            }
            Stmt::Break { .. } | Stmt::Continue { .. } => {
                // Phase 2.5b: break/continue allowed at top
                // level of async fn body. The base language
                // already validates that they appear only
                // inside loops; if we see one here it's
                // inside a while body or nested if-inside-while
                // (validator descends into nested constructs
                // recursively via validate_v31_phase_21a_branch).
            }
            // Reject other forms with a generic message.
            _ => {
                return Err(Diagnostic::new(
                    crate::span::Span::new(0, 0),
                    "v3.1 async fn body contains an unsupported statement form for Phase 2 (control flow); see ARC8_V3_PLAN.md Phase 2.1+ for deferred shapes",
                ));
            }
        }
    }
    Ok(locals)
}

/// Build a simple i64 binary subtract expression `a - b` —
/// vāṇī source convention for negative literals (no unary
/// minus literal). Used by the synthesized poll fn to compare
/// against the -2 (Pending) and -1 (Error) sentinels.
fn synth_i64_sub(a: i128, b: i128, span: crate::span::Span) -> Expr {
    Expr {
        kind: ExprKind::Binary {
            op: BinaryOp::Sub,
            left: Box::new(Expr { kind: ExprKind::Int(a), span }),
            right: Box::new(Expr { kind: ExprKind::Int(b), span }),
        },
        span,
    }
}

/// Build a `t.field` field-access expression.
fn synth_field_access(obj_name: &str, field: &str, span: crate::span::Span) -> Expr {
    Expr {
        kind: ExprKind::FieldAccess {
            object: Box::new(Expr {
                kind: ExprKind::Var(obj_name.to_string()),
                span,
            }),
            field: field.to_string(),
        },
        span,
    }
}

/// Phase 2 — rewrite every Var inside a Stmt that matches
/// the rename set to a FieldAccess on the task struct. For
/// Stmt::Assign whose LHS name is in the rename set, emit
/// a FieldAssign on `__t.<name>` instead. Recursively
/// handles nested if/while bodies.
fn rewrite_vars_in_stmt(
    stmt: &Stmt,
    rename_set: &std::collections::HashSet<String>,
    obj_name: &str,
) -> Stmt {
    match stmt {
        Stmt::Let { name, annotation, expr, span } => Stmt::Let {
            name: name.clone(),
            annotation: annotation.clone(),
            expr: rewrite_vars_to_fields(expr, rename_set, obj_name),
            span: *span,
        },
        Stmt::Return { expr, span } => Stmt::Return {
            expr: rewrite_vars_to_fields(expr, rename_set, obj_name),
            span: *span,
        },
        // Phase 2.4 — pre-existing FieldAssign (e.g. emitted by
        // the try-desugar's early-return path: `__t.__result =
        // __try_X;`) needs its `value` rewritten so any local-
        // var refs inside it route to `__t.<name>`. Without this,
        // the FieldAssign falls through the default `other =>
        // other.clone()` arm and `Var("__try_X")` stays as a
        // bare local reference that doesn't exist in scope.
        Stmt::FieldAssign { object, field, field_span, value, span } => {
            Stmt::FieldAssign {
                object: rewrite_vars_to_fields(object, rename_set, obj_name),
                field: field.clone(),
                field_span: *field_span,
                value: rewrite_vars_to_fields(value, rename_set, obj_name),
                span: *span,
            }
        }
        Stmt::Assign { name, expr, span } => {
            // If the LHS is an outer-scope name, this becomes
            // a FieldAssign on __t.<name>. Otherwise (inner
            // local scope) keep as Assign on the local Var.
            let new_expr = rewrite_vars_to_fields(expr, rename_set, obj_name);
            if rename_set.contains(name) {
                Stmt::FieldAssign {
                    object: Expr {
                        kind: ExprKind::Var(obj_name.to_string()),
                        span: *span,
                    },
                    field: name.clone(),
                    field_span: *span,
                    value: new_expr,
                    span: *span,
                }
            } else {
                Stmt::Assign {
                    name: name.clone(),
                    expr: new_expr,
                    span: *span,
                }
            }
        }
        Stmt::Print { items, span } => Stmt::Print {
            items: items.iter().map(|it| match it {
                crate::ast::PrintItem::Expr(e) => crate::ast::PrintItem::Expr(
                    rewrite_vars_to_fields(e, rename_set, obj_name),
                ),
                crate::ast::PrintItem::Str(s) => crate::ast::PrintItem::Str(s.clone()),
            }).collect(),
            span: *span,
        },
        Stmt::If { cond, then_body, else_body, span } => Stmt::If {
            cond: rewrite_vars_to_fields(cond, rename_set, obj_name),
            then_body: then_body.iter()
                .map(|s| rewrite_vars_in_stmt(s, rename_set, obj_name))
                .collect(),
            else_body: else_body.iter()
                .map(|s| rewrite_vars_in_stmt(s, rename_set, obj_name))
                .collect(),
            span: *span,
        },
        Stmt::While { label, cond, invariants, body, span } => Stmt::While {
            label: label.clone(),
            cond: rewrite_vars_to_fields(cond, rename_set, obj_name),
            invariants: invariants.iter()
                .map(|e| rewrite_vars_to_fields(e, rename_set, obj_name))
                .collect(),
            body: body.iter()
                .map(|s| rewrite_vars_in_stmt(s, rename_set, obj_name))
                .collect(),
            span: *span,
        },
        // Other statement forms — Phase 2 narrow rejects these
        // in the validator, so this branch is unreachable in
        // well-formed input. Pass through verbatim as a
        // defensive default.
        other => other.clone(),
    }
}

/// Rewrite every `Var(name)` in `expr` to `t.<name>` if `name`
/// is in the rename set. Used inside the poll fn's state arms
/// so that user-written locals + params resolve through the
/// task struct.
fn rewrite_vars_to_fields(
    expr: &Expr,
    rename_set: &std::collections::HashSet<String>,
    obj_name: &str,
) -> Expr {
    let kind = match &expr.kind {
        ExprKind::Var(name) if rename_set.contains(name) => {
            return synth_field_access(obj_name, name, expr.span);
        }
        ExprKind::Binary { op, left, right } => ExprKind::Binary {
            op: *op,
            left: Box::new(rewrite_vars_to_fields(left, rename_set, obj_name)),
            right: Box::new(rewrite_vars_to_fields(right, rename_set, obj_name)),
        },
        ExprKind::Unary { op, expr: e } => ExprKind::Unary {
            op: *op,
            expr: Box::new(rewrite_vars_to_fields(e, rename_set, obj_name)),
        },
        ExprKind::Cast { expr: e, ty } => ExprKind::Cast {
            expr: Box::new(rewrite_vars_to_fields(e, rename_set, obj_name)),
            ty: ty.clone(),
        },
        ExprKind::Call { name, name_span, args } => ExprKind::Call {
            name: name.clone(),
            name_span: *name_span,
            args: args.iter().map(|a| rewrite_vars_to_fields(a, rename_set, obj_name)).collect(),
        },
        ExprKind::Index { array, index } => ExprKind::Index {
            array: Box::new(rewrite_vars_to_fields(array, rename_set, obj_name)),
            index: Box::new(rewrite_vars_to_fields(index, rename_set, obj_name)),
        },
        ExprKind::Len { array } => ExprKind::Len {
            array: Box::new(rewrite_vars_to_fields(array, rename_set, obj_name)),
        },
        ExprKind::Ref { inner } => ExprKind::Ref {
            inner: Box::new(rewrite_vars_to_fields(inner, rename_set, obj_name)),
        },
        ExprKind::RefMut { inner } => ExprKind::RefMut {
            inner: Box::new(rewrite_vars_to_fields(inner, rename_set, obj_name)),
        },
        // Phase 2.3 — Match expression: rewrite scrutinee +
        // each arm's body. Patterns themselves don't contain
        // exprs (just literals / variant names / bindings).
        ExprKind::Match { scrutinee, arms } => ExprKind::Match {
            scrutinee: Box::new(rewrite_vars_to_fields(scrutinee, rename_set, obj_name)),
            arms: arms.iter().map(|a| crate::ast::MatchArm {
                pattern: a.pattern.clone(),
                pattern_span: a.pattern_span,
                body: rewrite_vars_to_fields(&a.body, rename_set, obj_name),
            }).collect(),
        },
        ExprKind::MethodCall { receiver, method, method_span, args } => ExprKind::MethodCall {
            receiver: Box::new(rewrite_vars_to_fields(receiver, rename_set, obj_name)),
            method: method.clone(),
            method_span: *method_span,
            args: args.iter().map(|a| rewrite_vars_to_fields(a, rename_set, obj_name)).collect(),
        },
        ExprKind::Tuple(elements) => ExprKind::Tuple(
            elements.iter().map(|e| rewrite_vars_to_fields(e, rename_set, obj_name)).collect(),
        ),
        ExprKind::TupleAccess { tuple, index } => ExprKind::TupleAccess {
            tuple: Box::new(rewrite_vars_to_fields(tuple, rename_set, obj_name)),
            index: *index,
        },
        ExprKind::FieldAccess { object, field } => ExprKind::FieldAccess {
            object: Box::new(rewrite_vars_to_fields(object, rename_set, obj_name)),
            field: field.clone(),
        },
        ExprKind::StructLit { type_name, type_name_span, fields } => ExprKind::StructLit {
            type_name: type_name.clone(),
            type_name_span: *type_name_span,
            fields: fields.iter().map(|(n, e)| {
                (n.clone(), rewrite_vars_to_fields(e, rename_set, obj_name))
            }).collect(),
        },
        ExprKind::ArrayLit { elements } => ExprKind::ArrayLit {
            elements: elements.iter().map(|e| rewrite_vars_to_fields(e, rename_set, obj_name)).collect(),
        },
        other => other.clone(),
    };
    Expr { kind, span: expr.span }
}

/// Phase 1 transform entry point. If the async fn body is
/// v3.1-eligible (contains io_*_async + linear shape + i64
/// only), synthesize:
/// - A `__TaskFor_<name>` struct with state_tag + params + locals
/// - A `__poll_<name>` fn implementing the state machine
/// - A constructor body that returns the struct
///
/// Pushes the new struct + poll fn into V31_TASK_REGISTRY for
/// parse_program to flush. Returns the new (return_type,
/// constructor_body) for the original async fn to adopt.
///
/// Returns None when the body has no io_*_async calls (caller
/// falls through to v1 sync desugar).
///
/// Returns Some(Err(diag)) when the body has io_*_async calls
/// but doesn't satisfy Phase 1's narrow shape — the diagnostic
/// is bubbled up so the user sees a clear pointer to which
/// later phase handles their case.
/// A4.4 (2026-06-08) — auto-inject cancel-guards before each
/// suspend point in a v3.1 async fn body. Called by
/// `try_v31_transform` only when the async fn has a
/// `ref CancelToken` / `mut ref CancelToken` parameter and the
/// return type is i64.
///
/// For each top-level suspend point (any `Stmt::Let` whose RHS
/// is a Call to `io_*_async` or `__poll_*`), prepends:
///   `if <token>.cancelled { return 0 - 1; }`
/// Suspends nested inside if/while bodies get the same
/// treatment via recursion. The guard uses `0 - 1` rather than
/// a bare `-1` literal because vāṇी doesn't have unary minus
/// (negative literals are lexed as binary subtraction by
/// existing examples; mirror that convention here).
fn inject_cancel_guards(body: &[Stmt], token_name: &str, span: Span) -> Vec<Stmt> {
    fn is_suspend_let(s: &Stmt) -> bool {
        if let Stmt::Let { expr, .. } = s {
            if let ExprKind::Call { name, .. } = &expr.kind {
                return name.starts_with("__poll_")
                    || matches!(
                        name.as_str(),
                        "io_recv_async" | "io_send_async" | "io_accept_async"
                    );
            }
        }
        false
    }
    fn make_guard(token_name: &str, span: Span) -> Stmt {
        // Sentinel = `0 - 1` (i64 = -1). The C / LLVM constant
        // folders collapse it.
        let neg_one = Expr {
            kind: ExprKind::Binary {
                op: crate::ast::BinaryOp::Sub,
                left: Box::new(Expr {
                    kind: ExprKind::Int(0),
                    span,
                }),
                right: Box::new(Expr {
                    kind: ExprKind::Int(1),
                    span,
                }),
            },
            span,
        };
        Stmt::If {
            cond: Expr {
                kind: ExprKind::FieldAccess {
                    object: Box::new(Expr {
                        kind: ExprKind::Var(token_name.to_string()),
                        span,
                    }),
                    field: "cancelled".to_string(),
                },
                span,
            },
            then_body: vec![Stmt::Return {
                expr: neg_one,
                span,
            }],
            else_body: vec![],
            span,
        }
    }
    let mut out: Vec<Stmt> = Vec::new();
    for s in body {
        if is_suspend_let(s) {
            out.push(make_guard(token_name, span));
        }
        match s {
            Stmt::If {
                cond,
                then_body,
                else_body,
                span: s_span,
            } => {
                out.push(Stmt::If {
                    cond: cond.clone(),
                    then_body: inject_cancel_guards(then_body, token_name, span),
                    else_body: inject_cancel_guards(else_body, token_name, span),
                    span: *s_span,
                });
            }
            Stmt::While {
                label,
                cond,
                invariants,
                body: w_body,
                span: s_span,
            } => {
                out.push(Stmt::While {
                    label: label.clone(),
                    cond: cond.clone(),
                    invariants: invariants.clone(),
                    body: inject_cancel_guards(w_body, token_name, span),
                    span: *s_span,
                });
            }
            other => out.push(other.clone()),
        }
    }
    out
}

pub(crate) fn try_v31_transform(
    fn_name: &str,
    fn_name_span: crate::span::Span,
    // Phase 4c-broad: threaded through to the validator so
    // Type::Param(name) is accepted in params + return types
    // when `name` is one of the declared type params. Three
    // pre-existing mono blockers all shipped in this arc:
    //   1. substitute_type_param_in_stmt recurses into Expr
    //      and rewrites Task__/`__poll_` names (commit 4620e98).
    //   2. rewrite_apply_in_ty + add_if_needed skip Param-
    //      bearing Type::Apply use-sites (commit 4620e98).
    //   3. infer_concrete_type_for_call walks the called fn's
    //      declared params to find the T-bearing slot, with
    //      structural unwrap for v3.1 `__poll_*` calls (this
    //      session — checker.rs).
    fn_type_params: &[String],
    params: &[Param],
    body: &[Stmt],
    return_type: &Type,
) -> Option<Result<(Type, Vec<Stmt>), Diagnostic>> {
    if !body_uses_io_async(body) {
        return None; // Caller falls through to v1 desugar.
    }
    // Phase 2.4 — `try EXPR` desugar for v3.1 async fn bodies.
    // Runs FIRST so subsequent passes (ANF, validator,
    // state-machine collector) see only Match expressions —
    // never the Try keyword. Lowers
    //   let X: T = try EXPR;
    // into
    //   let __try_X_<n>: ResultType = EXPR;
    //   if match __try_X_<n> { ResultType.Ok(_) then 0, _ then 1 } == 1 {
    //     return __try_X_<n>;          // propagate Err
    //   }
    //   let X: T = match __try_X_<n> { ResultType.Ok(v) then v, _ then 0 };
    // where ResultType is the async fn's return type (must be a
    // registered enum with a payloaded first variant — the "Ok"
    // arm — and any second variant as the "Err" propagation case).
    let try_desugared_body: Vec<Stmt>;
    let body = match desugar_try_in_v31_body(body, return_type) {
        Ok(v) => {
            try_desugared_body = v;
            try_desugared_body.as_slice()
        }
        Err(d) => return Some(Err(d)),
    };
    // Phase 2.2 — ANF lift nested io_*_async calls out of
    // compound expressions into fresh Let bindings BEFORE
    // running the validator + collector. The lifted body has
    // io_*_async calls only at top-level Let RHS positions
    // (where the segment collector recognizes them as
    // suspend points). The validator + downstream synthesis
    // see the lifted body — they don't need to know ANF
    // happened.
    let mut anf_counter: usize = 0;
    let lifted_body = anf_lift_body(body, &mut anf_counter);
    let body = lifted_body.as_slice();

    // A4.4 (2026-06-08) — CancelToken auto-plumbing. When the
    // async fn has a `ref CancelToken` / `mut ref CancelToken`
    // parameter AND the return type is i64 (sentinel-friendly),
    // walk the lifted body and inject
    //   if <token>.cancelled { return 0 - 1; }
    // immediately before each top-level suspend point (any
    // `Stmt::Let` whose RHS is a Call to `io_*_async` or
    // `__poll_*`). Suspends nested inside if/while bodies get the
    // same treatment via recursion.
    //
    // Non-i64 returns (Phase 3-returns ABI: `__result` field +
    // status code) keep the user's manual cancellation pattern
    // for v1 — auto-injection there would require synthesizing
    // a default `__result` value, which is shape-dependent.
    let cancel_token_param: Option<String> = params.iter().find_map(|p| {
        let inner = match &p.ty {
            Type::Ref(inner) | Type::RefMut(inner) => &**inner,
            _ => return None,
        };
        match inner {
            Type::Struct(name) if name == "CancelToken" => Some(p.name.clone()),
            _ => None,
        }
    });
    let auto_guarded_body: Vec<Stmt>;
    let body = if matches!(return_type, Type::I64) {
        if let Some(token_name) = cancel_token_param.as_ref() {
            auto_guarded_body = inject_cancel_guards(body, token_name, fn_name_span);
            auto_guarded_body.as_slice()
        } else {
            body
        }
    } else {
        body
    };

    let locals = match validate_v31_linear_body(
        fn_type_params, params, body, return_type
    ) {
        Ok(ls) => ls,
        Err(d) => return Some(Err(d)),
    };

    // Synthesized type name MUST start uppercase per the
    // parser's `parse_type` discipline ("only types can
    // appear in type position; identifier must be
    // PascalCase-leading"). Use the existing module-mangle
    // convention with double-underscore separators.
    let task_struct_name = format!("Task__{}", fn_name);
    let poll_fn_name = format!("__poll_{}", fn_name);

    // Build the rename set: all params + all (non-_) locals.
    let mut rename: std::collections::HashSet<String> = std::collections::HashSet::new();
    for p in params {
        rename.insert(p.name.clone());
    }
    for (name, _, _) in &locals {
        rename.insert(name.clone());
    }

    // Task struct fields: state_tag (i64), then one i64 per
    // param (in declaration order), then one i64 per local
    // (in declaration order).
    let mut struct_fields: Vec<StructField> = Vec::new();
    struct_fields.push(StructField {
        name: "state_tag".to_string(),
        ty: Type::I64,
        span: fn_name_span,
    });
    for p in params {
        // Phase 3-params — keep the user's annotation so non-i64
        // params (bool / Str / Enum / etc.) become task struct
        // fields of the correct type, not forced to i64.
        struct_fields.push(StructField {
            name: p.name.clone(),
            ty: p.ty.clone(),
            span: p.span,
        });
    }
    for (name, ty, span) in &locals {
        struct_fields.push(StructField {
            name: name.clone(),
            ty: ty.clone(),
            span: *span,
        });
    }
    // Phase 3-returns — non-i64 async fn return types route the
    // result through a synthesized `__result: T` field. Poll fn
    // returns status (0 = Ready, -2 = Pending, -1 = Error)
    // instead of the value; driver reads `t.__result` on status
    // == 0. For T == i64 the old ABI (poll returns the value)
    // stays in effect so existing examples keep working.
    let return_via_field = !matches!(return_type, Type::I64);
    if return_via_field {
        struct_fields.push(StructField {
            name: "__result".to_string(),
            ty: return_type.clone(),
            span: fn_name_span,
        });
    }

    // Phase 4c-broad: forward the enclosing fn's type params to
    // the synthesized Task__X struct so its field types (which
    // may reference Type::Param(T)) typecheck at template time.
    // Mono later specializes the struct alongside the fn —
    // Task__identity<T> → Task__identity__i64 etc.
    let mut task_struct = StructDecl {
        name: task_struct_name.clone(),
        name_span: fn_name_span,
        type_params: fn_type_params.to_vec(),
        fields: struct_fields,
        span: fn_name_span,
    };

    // Walk the body again, splitting into states.
    // State N spans:
    //   - All non-suspend Let stmts + non-Return Print/Assign
    //     stmts since the previous state
    //   - Plus the suspend-point at the END of state N (if any)
    // After each suspend point, bump state_tag and process the
    // suspend's result.
    //
    // For Phase 1's narrow shape:
    //   body = [ Let(L1, EXPR1), Let(L2, EXPR2), ..., Return(EXPR_R) ]
    // where each Lt either contains a suspend (io_*_async) or not.
    //
    // We emit one state per Let-with-suspend + a final state for
    // the Return.
    //
    // States layout:
    //   state 0: any preceding non-suspend Lets + first suspend (or Return)
    //   state N: result-handling for suspend N-1 + any non-suspend Lets +
    //            next suspend (or Return)
    //
    // For simplicity, this Phase 1 implementation treats EACH
    // Let as its own state segment. Non-suspend Lets are
    // emitted directly inside the next state's prologue.

    // Collect a flat list of (state_index, segment_kind):
    //   - NonSuspendLet(name, expr): runs in the same state
    //     where the previous suspend completed; the local is
    //     saved into the task struct field.
    //   - Suspend(name, builtin, args): runs as a suspend point
    //     (STATE TERMINATOR — bumps state_tag).
    //   - Discard(expr): non-suspend `let _ = ...;` — emitted
    //     verbatim in current state.
    //   - Return(expr): top-level return (STATE TERMINATOR —
    //     exits the poll fn entirely).
    //   - Verbatim(Stmt): Phase 2 narrow — an if / while /
    //     Assign / Print at top level. The whole statement is
    //     emitted verbatim in the current state's arm with
    //     var-rewriting; does NOT terminate the state (a
    //     nested return inside the if's then_body exits the
    //     poll fn naturally without changing the outer state
    //     transitions).
    enum Seg {
        /// Phase 3a — carries the local's annotation so the
        /// synthesizer's temp local + task struct field codegen
        /// uses the correct type (Bool / f64 / Str / Enum / Struct).
        /// Defaults to I64 when the user wrote `let x = ...;`
        /// without an explicit annotation.
        NonSuspendLet { name: String, ty: Type, expr: Expr, span: crate::span::Span },
        /// Phase 2.1 — Suspend carries explicit `bump_to`
        /// because with branching the next state isn't always
        /// `current_index + 1`. Phase 1 sequential bodies still
        /// see `bump_to == current_index + 1` per the collector.
        Suspend {
            local_name: String,
            builtin: String,
            args: Vec<Expr>,
            bump_to: usize,
            span: crate::span::Span,
        },
        Discard { expr: Expr, span: crate::span::Span },
        Return { expr: Expr, span: crate::span::Span },
        Verbatim(Stmt),
        /// Phase 2.1a — `if cond { ... } else { ... }` where at
        /// least one branch contains a suspend. The current
        /// state ends with a conditional jump to either
        /// then_state or else_state; the cascade enters the
        /// matching branch and ignores the unreachable one
        /// (each branch must end with Return per Phase 2.1a's
        /// narrow scope).
        Decision { cond: Expr, then_state: usize, else_state: usize, span: crate::span::Span },
        /// Phase 2.1b — unconditional state_tag bump. Emitted
        /// at the END of a non-return-terminated branch to
        /// transfer control to the merge state. Cascade then
        /// enters the merge state on a subsequent poll() call.
        Jump { target_state: usize, span: crate::span::Span },
    }

    // Build state_bodies directly via a recursive collector.
    // state_bodies[K] = segs that run when state_tag == K.
    // Phase 2.1a allows Stmt::If with suspends in branches —
    // each branch becomes its own state chain rooted at an
    // explicit state index. The cascade pattern relies on
    // monotonic state advances, which holds because branch
    // states are always allocated AFTER the decision state.
    //
    // Defensive `__v3_discard_NNNN` counter: synthetic names
    // for `let _ = io_*_async(...)` so the validator + poll
    // codegen treat them as suspends-that-throw-away-the-result.
    // Counter is incremented across recursive calls so each
    // branch's discards get unique names.
    let mut state_bodies: Vec<Vec<Seg>> = vec![Vec::new()];
    let mut discard_counter: usize = 0;

    // Recursively walk a list of stmts, appending segs to the
    // state pointed at by current_state. May allocate new
    // states. Returns the index of the state pointed at on
    // exit (useful when callers need to know where execution
    // continues — for Phase 2.1a both branches must end with
    // Return so the exit-state isn't consumed).
    /// Returns `true` if the body terminated with a top-level
    /// `Return` (no fall-through). Phase 2.1b uses this to
    /// decide whether to emit a `Jump` to the merge state at
    /// a branch's tail.
    ///
    /// Phase 2.5b: `loop_stack` carries `(loop_header,
    /// post_loop)` pairs for each enclosing suspending while
    /// loop. `break` jumps to the innermost `post_loop`;
    /// `continue` jumps to the innermost `loop_header`. Both
    /// terminate the current state's segment chain.
    fn collect_into(
        stmts: &[Stmt],
        state_bodies: &mut Vec<Vec<Seg>>,
        current_state: &mut usize,
        discard_counter: &mut usize,
        loop_stack: &mut Vec<(usize, usize)>,
    ) -> bool {
        let mut terminated = false;
        for s in stmts {
            if terminated {
                // Unreachable code after a Return — validator
                // should reject this, but bail defensively.
                break;
            }
            match s {
                Stmt::Let { name, annotation, expr, span, .. } => {
                    let is_discard = name == "_";
                    if let ExprKind::Call { name: cname, args, .. } = &expr.kind {
                        // Phase 4a-broad: any `__poll_<inner>` call
                        // is treated as a suspend point alongside
                        // the io_*_async family. The user's
                        // pattern
                        //   let n: i64 = __poll_inner(mut ref sub);
                        // becomes a true suspend in the outer's
                        // state machine: -2 yields outer, < 0
                        // propagates error, otherwise stores the
                        // value + bumps state. Matches the user-
                        // facing `await sub` semantics.
                        let is_poll_call = cname.starts_with("__poll_");
                        if is_poll_call || matches!(
                            cname.as_str(),
                            "io_recv_async" | "io_send_async" | "io_accept_async"
                        ) {
                            let local_name = if is_discard {
                                let n = format!("__v3_discard_{}", *discard_counter);
                                *discard_counter += 1;
                                n
                            } else {
                                name.clone()
                            };
                            // Allocate next state for resume; pass
                            // its index to Suspend so synthesis can
                            // emit the explicit state_tag bump
                            // (Phase 2.1 fix: branches make the
                            // next state's index non-sequential).
                            let next_state = state_bodies.len();
                            state_bodies[*current_state].push(Seg::Suspend {
                                local_name,
                                builtin: cname.clone(),
                                args: args.clone(),
                                bump_to: next_state,
                                span: *span,
                            });
                            state_bodies.push(Vec::new());
                            *current_state = next_state;
                            continue;
                        }
                    }
                    if is_discard {
                        state_bodies[*current_state].push(Seg::Discard {
                            expr: expr.clone(),
                            span: *span,
                        });
                    } else {
                        state_bodies[*current_state].push(Seg::NonSuspendLet {
                            name: name.clone(),
                            ty: annotation.clone().unwrap_or(Type::I64),
                            expr: expr.clone(),
                            span: *span,
                        });
                    }
                }
                Stmt::Return { expr, span } => {
                    state_bodies[*current_state].push(Seg::Return {
                        expr: expr.clone(),
                        span: *span,
                    });
                    terminated = true;
                }
                Stmt::If { cond, then_body, else_body, span } => {
                    let has_suspend = then_body.iter().any(stmt_contains_io_async)
                        || else_body.iter().any(stmt_contains_io_async);
                    if !has_suspend {
                        // Phase 2 narrow path: emit verbatim.
                        // Phase 2.5b: if we're inside a
                        // suspending loop, rewrite any nested
                        // break/continue inside this if's
                        // branches to state_tag jumps targeting
                        // the innermost suspending loop.
                        let verbatim_stmt = if let Some(&(lh, pl)) = loop_stack.last() {
                            let rewritten = rewrite_break_continue_in_stmts(
                                std::slice::from_ref(s), lh, pl, "__t",
                            );
                            // rewrite_break_continue_in_stmts on a
                            // single-element slice returns a Vec
                            // with one element (the rewritten if).
                            rewritten.into_iter().next().unwrap_or_else(|| s.clone())
                        } else {
                            s.clone()
                        };
                        state_bodies[*current_state].push(Seg::Verbatim(verbatim_stmt));
                        // Phase 2.1c: if both branches return,
                        // mark terminated so the caller doesn't
                        // emit a dead Jump after the verbatim.
                        if body_all_paths_return(std::slice::from_ref(s)) {
                            terminated = true;
                        }
                    } else {
                        // Phase 2.1a/b: state-splitting. Allocate
                        // then_state + else_state up front (they
                        // need stable indices for the Decision
                        // seg), push Decision into current state,
                        // recurse into each branch, then allocate
                        // the merge state AFTER both recursions so
                        // its index is highest (Phase 2.1b cascade
                        // invariant). For each branch that fell
                        // through (didn't terminate with Return),
                        // emit a Jump at its tail to the merge.
                        let then_state = state_bodies.len();
                        state_bodies.push(Vec::new());
                        let else_state = state_bodies.len();
                        state_bodies.push(Vec::new());
                        state_bodies[*current_state].push(Seg::Decision {
                            cond: cond.clone(),
                            then_state,
                            else_state,
                            span: *span,
                        });
                        let mut then_current = then_state;
                        let then_terminated = collect_into(
                            then_body, state_bodies, &mut then_current, discard_counter, loop_stack,
                        );
                        let mut else_current = else_state;
                        let else_terminated = collect_into(
                            else_body, state_bodies, &mut else_current, discard_counter, loop_stack,
                        );
                        // Phase 2.1b: if either branch fell
                        // through, allocate a merge state and
                        // emit Jumps. If BOTH branches terminated
                        // with Return (the Phase 2.1a case),
                        // skip the merge — the outer flow can
                        // never reach it. Otherwise the outer
                        // continues at the merge state.
                        if !then_terminated || !else_terminated {
                            let merge_state = state_bodies.len();
                            state_bodies.push(Vec::new());
                            if !then_terminated {
                                state_bodies[then_current].push(Seg::Jump {
                                    target_state: merge_state,
                                    span: *span,
                                });
                            }
                            if !else_terminated {
                                state_bodies[else_current].push(Seg::Jump {
                                    target_state: merge_state,
                                    span: *span,
                                });
                            }
                            *current_state = merge_state;
                        } else {
                            // Both branches Return — outer flow
                            // is dead. Allocate a defensive empty
                            // state. Validator should reject any
                            // stmts after this if.
                            state_bodies.push(Vec::new());
                            *current_state = state_bodies.len() - 1;
                            terminated = true;
                        }
                    }
                }
                Stmt::While { cond, body: while_body, span, .. } => {
                    let has_suspend = while_body.iter().any(stmt_contains_io_async);
                    if !has_suspend {
                        // Phase 2 narrow path: emit verbatim.
                        state_bodies[*current_state].push(Seg::Verbatim(s.clone()));
                    } else {
                        // Phase 2.5: state-splitting for
                        // while-with-suspend.
                        //   K       : current state — push Jump(loop_header)
                        //   K+1     : loop_header — push Decision(cond, body_start, post_loop)
                        //   K+2     : post_loop (allocated up front so Decision can target it)
                        //   K+3..M  : body states (allocated during recursion)
                        //   K+M's tail: backward Jump(loop_header)
                        let loop_header = state_bodies.len();
                        state_bodies.push(Vec::new());
                        let post_loop = state_bodies.len();
                        state_bodies.push(Vec::new());
                        let body_start = state_bodies.len();
                        state_bodies.push(Vec::new());
                        // Jump from current state to loop_header.
                        state_bodies[*current_state].push(Seg::Jump {
                            target_state: loop_header,
                            span: *span,
                        });
                        // At loop_header: Decision over cond.
                        state_bodies[loop_header].push(Seg::Decision {
                            cond: cond.clone(),
                            then_state: body_start,
                            else_state: post_loop,
                            span: *span,
                        });
                        // Recurse into body starting at body_start.
                        // Phase 2.5b: push (loop_header, post_loop)
                        // onto loop_stack so break/continue inside
                        // this loop's body know where to jump.
                        loop_stack.push((loop_header, post_loop));
                        let mut body_current = body_start;
                        let body_terminated = collect_into(
                            while_body, state_bodies, &mut body_current, discard_counter, loop_stack,
                        );
                        loop_stack.pop();
                        // At body's tail (if not return-
                        // terminated), emit BACKWARD Jump to
                        // loop_header. Synthesis recognizes
                        // this and emits `continue;`.
                        if !body_terminated {
                            state_bodies[body_current].push(Seg::Jump {
                                target_state: loop_header,
                                span: *span,
                            });
                        }
                        // Outer continues at post_loop.
                        *current_state = post_loop;
                    }
                }
                Stmt::Assign { .. } | Stmt::Print { .. } => {
                    // Phase 2 narrow: emit verbatim
                    // (validator guarantees no suspend inside).
                    state_bodies[*current_state].push(Seg::Verbatim(s.clone()));
                    if body_all_paths_return(std::slice::from_ref(s)) {
                        terminated = true;
                    }
                }
                Stmt::Break { span, .. } => {
                    // Phase 2.5b: break jumps to the innermost
                    // loop's post_loop state.
                    if let Some(&(_, post_loop)) = loop_stack.last() {
                        state_bodies[*current_state].push(Seg::Jump {
                            target_state: post_loop,
                            span: *span,
                        });
                        // After break, the rest of the current
                        // branch's stmts are unreachable.
                        terminated = true;
                    }
                    // If no enclosing loop, the base language
                    // rejects already; defensively skip.
                }
                Stmt::Continue { span, .. } => {
                    // Phase 2.5b: continue jumps to the innermost
                    // loop's loop_header state.
                    if let Some(&(loop_header, _)) = loop_stack.last() {
                        state_bodies[*current_state].push(Seg::Jump {
                            target_state: loop_header,
                            span: *span,
                        });
                        terminated = true;
                    }
                }
                _ => {}
            }
        }
        terminated
    }

    let mut start_state: usize = 0;
    let mut loop_stack: Vec<(usize, usize)> = Vec::new();
    let _ = collect_into(body, &mut state_bodies, &mut start_state, &mut discard_counter, &mut loop_stack);

    // The `states` shape stays the same as before — outer code
    // just iterates state_bodies. Drop any trailing empty
    // states allocated defensively after Returns/branches.
    let states: Vec<Vec<Seg>> = state_bodies;

    // v3.1 caveat #2 polish, part 2 (2026-06-08): liveness
    // optimization. A local is "state-local" iff every read of
    // it stays inside its declaring state. State-local locals
    // are emitted as poll-fn stack lets (not Task struct
    // fields), reducing per-Task heap footprint.
    //
    // Conservative rule: if a local is ever read in a different
    // state than the one declaring it, treat as cross-state.
    // Conservative is safe — emitting more fields than strictly
    // needed never breaks correctness, only wastes ~8 bytes
    // per stale field.
    //
    // Reads inside an Expr are found by walking the Expr tree
    // for `ExprKind::Var(name)` matches. Reads inside Seg
    // sub-structures (Decision branches, Suspend args, etc.)
    // are folded into the same state's read-set since the
    // segment collector already placed them in the right state
    // group.
    fn expr_reads_into(expr: &Expr, out: &mut std::collections::HashSet<String>) {
        match &expr.kind {
            ExprKind::Var(n) => { out.insert(n.clone()); }
            ExprKind::Binary { left, right, .. } => {
                expr_reads_into(left, out);
                expr_reads_into(right, out);
            }
            ExprKind::Unary { expr, .. } | ExprKind::Cast { expr, .. } => {
                expr_reads_into(expr, out);
            }
            ExprKind::Index { array, index } => {
                expr_reads_into(array, out);
                expr_reads_into(index, out);
            }
            ExprKind::ArrayLit { elements } | ExprKind::Tuple(elements) => {
                for e in elements { expr_reads_into(e, out); }
            }
            ExprKind::TupleAccess { tuple, .. } => expr_reads_into(tuple, out),
            ExprKind::Len { array } => expr_reads_into(array, out),
            ExprKind::Ref { inner } | ExprKind::RefMut { inner } => {
                expr_reads_into(inner, out);
            }
            ExprKind::FieldAccess { object, .. } => expr_reads_into(object, out),
            ExprKind::StructLit { fields, .. } => {
                for (_, v) in fields { expr_reads_into(v, out); }
            }
            ExprKind::Call { args, .. } => {
                for a in args { expr_reads_into(a, out); }
            }
            ExprKind::MethodCall { receiver, args, .. } => {
                expr_reads_into(receiver, out);
                for a in args { expr_reads_into(a, out); }
            }
            ExprKind::Match { scrutinee, arms } => {
                expr_reads_into(scrutinee, out);
                for a in arms { expr_reads_into(&a.body, out); }
            }
            ExprKind::IfExpr { cond, then_value, else_value, .. } => {
                expr_reads_into(cond, out);
                expr_reads_into(then_value, out);
                expr_reads_into(else_value, out);
            }
            ExprKind::Block { stmts, tail } => {
                for s in stmts { stmt_reads_into(s, out); }
                expr_reads_into(tail, out);
            }
            ExprKind::Try { inner } => expr_reads_into(inner, out),
            _ => {}
        }
    }
    fn stmt_reads_into(stmt: &Stmt, out: &mut std::collections::HashSet<String>) {
        match stmt {
            Stmt::Let { expr, .. }
            | Stmt::LetTuple { expr, .. }
            | Stmt::Return { expr, .. }
            | Stmt::Assert { expr, .. }
            | Stmt::Prove { expr, .. }
            | Stmt::Assign { expr, .. } => expr_reads_into(expr, out),
            Stmt::Print { items, .. } => {
                for it in items {
                    if let crate::ast::PrintItem::Expr(e) = it {
                        expr_reads_into(e, out);
                    }
                }
            }
            Stmt::If { cond, then_body, else_body, .. } => {
                expr_reads_into(cond, out);
                for s in then_body { stmt_reads_into(s, out); }
                for s in else_body { stmt_reads_into(s, out); }
            }
            Stmt::While { cond, body: w, .. } => {
                expr_reads_into(cond, out);
                for s in w { stmt_reads_into(s, out); }
            }
            Stmt::IndexAssign { index, value, .. } => {
                expr_reads_into(index, out);
                expr_reads_into(value, out);
            }
            Stmt::FieldAssign { object, value, .. } => {
                expr_reads_into(object, out);
                expr_reads_into(value, out);
            }
            _ => {}
        }
    }
    fn seg_reads_into(seg: &Seg, out: &mut std::collections::HashSet<String>) {
        match seg {
            Seg::NonSuspendLet { expr, .. } => expr_reads_into(expr, out),
            Seg::Suspend { args, .. } => {
                for a in args { expr_reads_into(a, out); }
            }
            Seg::Discard { expr, .. } => expr_reads_into(expr, out),
            Seg::Return { expr, .. } => expr_reads_into(expr, out),
            Seg::Verbatim(s) => stmt_reads_into(s, out),
            Seg::Decision { cond, .. } => expr_reads_into(cond, out),
            Seg::Jump { .. } => {}
        }
    }
    // Per-state declarations + reads.
    let n_states = states.len();
    let mut decl_state: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let mut reads_per_state: Vec<std::collections::HashSet<String>> =
        (0..n_states).map(|_| std::collections::HashSet::new()).collect();
    for (k, segs) in states.iter().enumerate() {
        for seg in segs {
            if let Seg::NonSuspendLet { name, .. } = seg {
                // First declaration wins (locals are unique in
                // v3.1; the validator already guarded shadowing).
                decl_state.entry(name.clone()).or_insert(k);
            }
            seg_reads_into(seg, &mut reads_per_state[k]);
        }
    }
    let mut state_locals: std::collections::HashSet<String> =
        std::collections::HashSet::new();
    for (name, k) in &decl_state {
        let crossed = (0..n_states).any(|j| j != *k && reads_per_state[j].contains(name));
        if !crossed {
            state_locals.insert(name.clone());
        }
    }
    // Filter rename: state-local names refer to poll-fn locals,
    // not Task struct fields.
    for n in &state_locals {
        rename.remove(n);
    }
    // Filter the Task struct's fields. `task_struct` was built
    // earlier (it needs to exist before V31_STRUCT_REGISTRY is
    // consulted by downstream defaults); patch its `fields`
    // in place to drop state-local entries.
    task_struct.fields.retain(|f| !state_locals.contains(&f.name));
    // Note: the `locals` Vec at the parent scope is still used
    // for the validator's defaults + the Task struct's initial
    // ctor body further below. State-local locals stay in
    // `locals` so the validator (which already returned its
    // result earlier) is unaffected. The downstream synthesizer
    // (this block) is the only consumer we need to update.

    // Canonicalize alias names at the typed-IR boundary. v3.1
    // poll fn calls the canonical nb variants directly.
    let canonical = |n: &str| -> String {
        match n {
            "io_recv_async" => "tcp_recv_nb".to_string(),
            "io_send_async" => "tcp_send_buf".to_string(),
            "io_accept_async" => "tcp_accept_nb".to_string(),
            other => other.to_string(),
        }
    };

    // Build the poll fn's body.
    let t_param_name = "__t".to_string();
    let mut poll_body: Vec<Stmt> = Vec::new();
    for (state_idx, state_segs) in states.iter().enumerate() {
        let span = state_segs.first().map(|s| match s {
            Seg::NonSuspendLet { span, .. }
            | Seg::Suspend { span, .. }
            | Seg::Discard { span, .. }
            | Seg::Return { span, .. }
            | Seg::Decision { span, .. }
            | Seg::Jump { span, .. } => *span,
            Seg::Verbatim(s) => match s {
                Stmt::If { span, .. }
                | Stmt::While { span, .. }
                | Stmt::Assign { span, .. }
                | Stmt::Print { span, .. } => *span,
                _ => fn_name_span,
            },
        }).unwrap_or(fn_name_span);

        // `if __t.state_tag == K { ... }`
        let cond = Expr {
            kind: ExprKind::Binary {
                op: BinaryOp::Eq,
                left: Box::new(synth_field_access(&t_param_name, "state_tag", span)),
                right: Box::new(Expr { kind: ExprKind::Int(state_idx as i128), span }),
            },
            span,
        };
        let mut then_body: Vec<Stmt> = Vec::new();

        for seg in state_segs {
            match seg {
                Seg::NonSuspendLet { name, ty, expr, span } => {
                    // v3.1 caveat #2 polish, part 2 (2026-06-08):
                    // state-local locals (declared + read entirely
                    // within ONE state-machine arm) elide the
                    // FieldAssign and the Task struct field —
                    // they live as poll-fn stack locals. Reads
                    // of the name in the same state see the local
                    // directly because the analysis pass above
                    // removed the name from `rename`, so the
                    // rewrite_vars_to_fields call below doesn't
                    // touch them.
                    let rewritten_expr = rewrite_vars_to_fields(expr, &rename, &t_param_name);
                    if state_locals.contains(name) {
                        // State-local: single Let in the state arm,
                        // no field assignment.
                        then_body.push(Stmt::Let {
                            name: name.clone(),
                            annotation: Some(ty.clone()),
                            expr: rewritten_expr,
                            span: *span,
                        });
                    } else {
                        // Cross-state: existing two-step emission.
                        // Phase 3a — `ty` carries the user-written
                        // annotation (Type::I64 by default).
                        let synth_local = format!("__v3_tmp_{}", name);
                        then_body.push(Stmt::Let {
                            name: synth_local.clone(),
                            annotation: Some(ty.clone()),
                            expr: rewritten_expr,
                            span: *span,
                        });
                        then_body.push(Stmt::FieldAssign {
                            object: Expr {
                                kind: ExprKind::Var(t_param_name.clone()),
                                span: *span,
                            },
                            field: name.clone(),
                            field_span: *span,
                            value: Expr {
                                kind: ExprKind::Var(synth_local),
                                span: *span,
                            },
                            span: *span,
                        });
                    }
                }
                Seg::Discard { expr, span } => {
                    // `let _ = EXPR;` — emit as a normal discard with the
                    // expr rewritten so any Var refs hit the task struct.
                    let rewritten_expr = rewrite_vars_to_fields(expr, &rename, &t_param_name);
                    then_body.push(Stmt::Let {
                        name: "_".to_string(),
                        annotation: None,
                        expr: rewritten_expr,
                        span: *span,
                    });
                }
                Seg::Suspend { local_name, builtin, args, bump_to, span } => {
                    // Suspend point. Emit:
                    //   let r: i64 = canonical(arg0, arg1);
                    //   if r == 0 - 2 { return 0 - 2; }
                    //   if r < 0 { return 0 - 1; }
                    //   t.<local_name> = r;
                    //   t.state_tag = K+1;
                    let canonical_name = canonical(builtin);
                    let rewritten_args: Vec<Expr> = args
                        .iter()
                        .map(|a| rewrite_vars_to_fields(a, &rename, &t_param_name))
                        .collect();
                    let r_local = format!("__v3_r{}", state_idx);
                    then_body.push(Stmt::Let {
                        name: r_local.clone(),
                        annotation: Some(Type::I64),
                        expr: Expr {
                            kind: ExprKind::Call {
                                name: canonical_name,
                                name_span: *span,
                                args: rewritten_args,
                            },
                            span: *span,
                        },
                        span: *span,
                    });
                    // if r == 0 - 2 { return 0 - 2; }
                    then_body.push(Stmt::If {
                        cond: Expr {
                            kind: ExprKind::Binary {
                                op: BinaryOp::Eq,
                                left: Box::new(Expr {
                                    kind: ExprKind::Var(r_local.clone()),
                                    span: *span,
                                }),
                                right: Box::new(synth_i64_sub(0, 2, *span)),
                            },
                            span: *span,
                        },
                        then_body: vec![Stmt::Return {
                            expr: synth_i64_sub(0, 2, *span),
                            span: *span,
                        }],
                        else_body: vec![],
                        span: *span,
                    });
                    // if r < 0 { return 0 - 1; }
                    then_body.push(Stmt::If {
                        cond: Expr {
                            kind: ExprKind::Binary {
                                op: BinaryOp::Lt,
                                left: Box::new(Expr {
                                    kind: ExprKind::Var(r_local.clone()),
                                    span: *span,
                                }),
                                right: Box::new(Expr { kind: ExprKind::Int(0), span: *span }),
                            },
                            span: *span,
                        },
                        then_body: vec![Stmt::Return {
                            expr: synth_i64_sub(0, 1, *span),
                            span: *span,
                        }],
                        else_body: vec![],
                        span: *span,
                    });
                    // Save: t.<local_name> = r
                    if !local_name.starts_with("__v3_discard_") {
                        // Ensure the task struct knows about this field.
                        // Already added during locals collection if the
                        // user declared it; synthetic discard fields would
                        // need adding too if we cared (we don't — discards
                        // skip the save).
                        then_body.push(Stmt::FieldAssign {
                            object: Expr {
                                kind: ExprKind::Var(t_param_name.clone()),
                                span: *span,
                            },
                            field: local_name.clone(),
                            field_span: *span,
                            value: Expr {
                                kind: ExprKind::Var(r_local),
                                span: *span,
                            },
                            span: *span,
                        });
                    }
                    // Bump: t.state_tag = bump_to. Phase 2.1
                    // uses explicit per-Suspend bump targets;
                    // sequential Phase 1 cases get bump_to ==
                    // state_idx + 1 from the collector.
                    then_body.push(Stmt::FieldAssign {
                        object: Expr {
                            kind: ExprKind::Var(t_param_name.clone()),
                            span: *span,
                        },
                        field: "state_tag".to_string(),
                        field_span: *span,
                        value: Expr {
                            kind: ExprKind::Int(*bump_to as i128),
                            span: *span,
                        },
                        span: *span,
                    });
                }
                Seg::Return { expr, span } => {
                    let rewritten_expr = rewrite_vars_to_fields(expr, &rename, &t_param_name);
                    // Phase 3-returns — for non-i64 returns route
                    // the value through `__t.__result`, then
                    // return 0 (Ready status). i64 returns keep
                    // the historical ABI (poll returns the value).
                    if return_via_field {
                        then_body.push(Stmt::FieldAssign {
                            object: Expr {
                                kind: ExprKind::Var(t_param_name.clone()),
                                span: *span,
                            },
                            field: "__result".to_string(),
                            field_span: *span,
                            value: rewritten_expr,
                            span: *span,
                        });
                        then_body.push(Stmt::Return {
                            expr: Expr {
                                kind: ExprKind::Int(0),
                                span: *span,
                            },
                            span: *span,
                        });
                    } else {
                        then_body.push(Stmt::Return {
                            expr: rewritten_expr,
                            span: *span,
                        });
                    }
                }
                Seg::Verbatim(stmt) => {
                    // Phase 2 narrow: if/while/Assign/Print at
                    // top level. Recursively rewrite Vars
                    // inside the stmt so any references to
                    // outer-scope locals/params hit the task
                    // struct fields. Then emit verbatim.
                    let rewritten = rewrite_vars_in_stmt(stmt, &rename, &t_param_name);
                    then_body.push(rewritten);
                }
                Seg::Jump { target_state, span } => {
                    // Phase 2.1b: unconditional state_tag bump.
                    // Phase 2.5: if the Jump is backward
                    // (target_state < current state index),
                    // emit `continue;` to re-enter the
                    // while-true wrapping the poll-fn body so
                    // the cascade re-checks states from the
                    // top. Forward jumps fall through normally.
                    then_body.push(Stmt::FieldAssign {
                        object: Expr {
                            kind: ExprKind::Var(t_param_name.clone()),
                            span: *span,
                        },
                        field: "state_tag".to_string(),
                        field_span: *span,
                        value: Expr {
                            kind: ExprKind::Int(*target_state as i128),
                            span: *span,
                        },
                        span: *span,
                    });
                    if *target_state <= state_idx {
                        then_body.push(Stmt::Continue { label: None, span: *span });
                    }
                }
                Seg::Decision { cond, then_state, else_state, span } => {
                    // Phase 2.1a: if-with-suspend. Emit
                    //   if cond_rewritten {
                    //     __t.state_tag = then_state;
                    //   } else {
                    //     __t.state_tag = else_state;
                    //   }
                    // The cascade then enters either then_state
                    // or else_state on the same poll() call —
                    // no return -2 here because the decision
                    // itself isn't a suspend point.
                    let rewritten_cond = rewrite_vars_to_fields(cond, &rename, &t_param_name);
                    let bump_then = Stmt::FieldAssign {
                        object: Expr {
                            kind: ExprKind::Var(t_param_name.clone()),
                            span: *span,
                        },
                        field: "state_tag".to_string(),
                        field_span: *span,
                        value: Expr {
                            kind: ExprKind::Int(*then_state as i128),
                            span: *span,
                        },
                        span: *span,
                    };
                    let bump_else = Stmt::FieldAssign {
                        object: Expr {
                            kind: ExprKind::Var(t_param_name.clone()),
                            span: *span,
                        },
                        field: "state_tag".to_string(),
                        field_span: *span,
                        value: Expr {
                            kind: ExprKind::Int(*else_state as i128),
                            span: *span,
                        },
                        span: *span,
                    };
                    then_body.push(Stmt::If {
                        cond: rewritten_cond,
                        then_body: vec![bump_then],
                        else_body: vec![bump_else],
                        span: *span,
                    });
                }
            }
        }

        poll_body.push(Stmt::If {
            cond,
            then_body,
            else_body: vec![],
            span,
        });
    }
    // Defensive trailing `return 0 - 1;` inside the cascade
    // (reached only when no state-tag matches — i.e., the
    // state machine fell through somehow).
    poll_body.push(Stmt::Return {
        expr: synth_i64_sub(0, 1, fn_name_span),
        span: fn_name_span,
    });

    // Phase 2.5 — wrap the cascade in `while true { ... }`
    // so backward Jumps (loop back-edges) can use `continue`
    // to re-enter the cascade. Non-loop fns hit the trailing
    // return -1 on the first iteration and exit normally.
    let wrapped_poll_body = vec![
        Stmt::While {
            label: None,
            cond: Expr {
                kind: ExprKind::Bool(true),
                span: fn_name_span,
            },
            invariants: vec![],
            body: poll_body,
            span: fn_name_span,
        },
        // Defensive trailing return after the while (unreachable
        // — while-true exits only via `return` inside).
        Stmt::Return {
            expr: synth_i64_sub(0, 1, fn_name_span),
            span: fn_name_span,
        },
    ];
    let poll_body = wrapped_poll_body;

    // Build the poll Function. Phase 4c-broad: when the
    // enclosing async fn is generic, the poll fn is too. Its
    // single param's type is `mut ref Task__X<T1, T2, ...>`
    // (Type::Apply) so mono can substitute T per call site
    // and re-collapse Apply<concrete> → Struct(mangled) via
    // substitute_type_param's Type::Apply branch. For non-
    // generic templates, Type::Apply with empty args degrades
    // gracefully to Type::Struct(name) under existing
    // rewrite_apply_in_ty paths.
    let task_ref_ty = if fn_type_params.is_empty() {
        Type::RefMut(Box::new(Type::Struct(task_struct_name.clone())))
    } else {
        let apply_args: Vec<Type> = fn_type_params
            .iter()
            .map(|n| Type::Param(n.clone()))
            .collect();
        Type::RefMut(Box::new(Type::Apply {
            name: task_struct_name.clone(),
            args: apply_args,
        }))
    };
    let poll_fn = Function {
        name: poll_fn_name,
        type_params: fn_type_params.to_vec(),
        where_clauses: vec![],
        params: vec![Param {
            name: t_param_name.clone(),
            ty: task_ref_ty,
            name_span: fn_name_span,
            span: fn_name_span,
        }],
        return_type: Type::I64,
        requires: vec![],
        ensures: vec![],
        body: poll_body,
        span: fn_name_span,
        is_pure: false,
        is_extern: false,
        no_heap: false,
        no_float: false,
        no_recursion: false,
        interrupt: false,
        safety_standard: None,
        bounded_stack: None,
        wcet_cycles: None,
        deterministic_timing: false,
        recursion_bound: None,
    };

    // Build the constructor body: `return __TaskFor_<name> { state_tag: 0, <params>, <locals>: 0 };`
    let mut ctor_fields: Vec<(String, Expr)> = Vec::new();
    ctor_fields.push((
        "state_tag".to_string(),
        Expr { kind: ExprKind::Int(0), span: fn_name_span },
    ));
    for p in params {
        ctor_fields.push((
            p.name.clone(),
            Expr {
                kind: ExprKind::Var(p.name.clone()),
                span: p.span,
            },
        ));
    }
    for (name, ty, span) in &locals {
        // v3.1 caveat #2 polish, part 2 (2026-06-08): skip
        // state-local locals — they live as poll-fn stack
        // locals, not Task struct fields, so they don't need
        // ctor-side default-init either.
        if state_locals.contains(name) {
            continue;
        }
        // Phase 3a — emit type-appropriate default-init so the
        // constructor's StructLit typechecks for non-i64 fields.
        ctor_fields.push((
            name.clone(),
            v31_default_init_expr(ty, *span),
        ));
    }
    // Phase 3-returns — default-init the synthesized `__result`
    // field if the async fn has a non-i64 return type.
    //
    // Phase 4c-broad: when return_type is Type::Param(name),
    // seed __result from a same-typed param (the validator
    // already enforced that one exists). At template time we
    // can't conjure a default value of generic T, but we CAN
    // reuse a parameter's runtime value as the initial slot
    // contents — the poll fn overwrites `__result` before the
    // first read anyway, so the seed is just a typecheck-
    // satisfying placeholder.
    if return_via_field {
        let init_expr = if let Type::Param(ret_name) = return_type {
            let seed_param = params
                .iter()
                .find(|p| matches!(&p.ty, Type::Param(n) if n == ret_name))
                .expect(
                    "validator guarantees a matching-T param for generic \
                     return type (Phase 4c-broad invariant)",
                );
            Expr {
                kind: ExprKind::Var(seed_param.name.clone()),
                span: seed_param.span,
            }
        } else {
            v31_default_init_expr(return_type, fn_name_span)
        };
        ctor_fields.push(("__result".to_string(), init_expr));
    }
    let ctor_body = vec![Stmt::Return {
        expr: Expr {
            kind: ExprKind::StructLit {
                type_name: task_struct_name.clone(),
                type_name_span: fn_name_span,
                fields: ctor_fields,
            },
            span: fn_name_span,
        },
        span: fn_name_span,
    }];

    // Phase 4 — also register the synthesized Task__X struct in
    // V31_STRUCT_REGISTRY so OTHER v3.1 async fns can declare a
    // local of this type (`let sub: Task__inner = inner(args);`)
    // and the type-allowed gate accepts it. The struct's fields
    // are already typed (we built them above), so the recursive
    // field-default check works for sub-task locals just like
    // any other user struct.
    crate::ast::V31_STRUCT_REGISTRY.with(|reg| {
        let fields: Vec<(String, Type)> = task_struct
            .fields
            .iter()
            .map(|f| (f.name.clone(), f.ty.clone()))
            .collect();
        reg.borrow_mut().insert(task_struct_name.clone(), fields);
    });
    // Push the synthesized struct + poll fn into the registry.
    crate::ast::V31_TASK_REGISTRY.with(|reg| {
        reg.borrow_mut().push((task_struct, poll_fn));
    });

    // Phase 4c-broad: when the async fn is generic, the
    // original fn's new return type is `Task__X<T1, T2, ...>`
    // (Type::Apply) so mono recognizes it as a generic use
    // site and specializes per call. Non-generic templates
    // keep the bare Type::Struct return as before.
    let new_return_ty = if fn_type_params.is_empty() {
        Type::Struct(task_struct_name)
    } else {
        let apply_args: Vec<Type> = fn_type_params
            .iter()
            .map(|n| Type::Param(n.clone()))
            .collect();
        Type::Apply {
            name: task_struct_name,
            args: apply_args,
        }
    };
    Some(Ok((new_return_ty, ctor_body)))
}
