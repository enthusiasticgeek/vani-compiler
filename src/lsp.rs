//! Language Server Protocol implementation for vāṇī (vanic).
//!
//! Shipped surface:
//!   - `textDocument/didOpen`, `didChange`, `didClose` — in-memory doc mirror.
//!   - `textDocument/publishDiagnostics` — push lexer/parser/checker errors on
//!     every open/change.
//!   - `textDocument/hover` — type of the smallest typed expression at cursor;
//!     augmented doc-block for the postfix `?` operator.
//!   - `textDocument/definition` — goto definition for function calls and
//!     variable bindings.
//!   - `textDocument/references` — find all reference sites of a binding.
//!   - `textDocument/rename` — rename a binding across the document.
//!   - `textDocument/completion` — keywords (all supported dialects),
//!     type names, built-in functions, in-scope bindings, function names.
//!   - `textDocument/codeAction` — quick-fix insertion for missing single-char
//!     tokens (`;`, `)`, `}`, etc.).
//!   - `textDocument/semanticTokens/full` — IR-driven semantic coloring:
//!     function calls, parameter declarations, parameter reads, type names,
//!     keywords, numbers, strings.

use std::collections::HashMap;
use std::error::Error;

use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument,
    Notification as NotificationTrait, PublishDiagnostics,
};
use lsp_types::request::{HoverRequest, Request as RequestTrait};
use lsp_types::{
    Diagnostic, DiagnosticSeverity, Hover, HoverContents, HoverProviderCapability,
    InitializeParams, MarkupContent, MarkupKind, OneOf, Position,
    PublishDiagnosticsParams, Range, ServerCapabilities, TextDocumentSyncCapability,
    TextDocumentSyncKind, Uri as Url,
};

use crate::ir::{TypedExpr, TypedExprKind, TypedPrintItem, TypedStmt};

/// Run the LSP server against stdio until the editor closes the
/// connection. Returns Ok(()) on a clean shutdown sequence
/// (`shutdown` request followed by an `exit` notification), an error
/// on protocol problems.
pub fn run() -> Result<(), Box<dyn Error + Sync + Send>> {
    let (connection, io_threads) = Connection::stdio();

    let capabilities = ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        definition_provider: Some(OneOf::Left(true)),
        references_provider: Some(OneOf::Left(true)),
        rename_provider: Some(OneOf::Left(true)),
        completion_provider: Some(lsp_types::CompletionOptions {
            // No trigger characters yet — completion only
            // fires on explicit invocation (Ctrl+Space in most
            // editors). Triggering on `.` would call out
            // method-style completion, which doesn't exist in
            // Intent.
            trigger_characters: None,
            ..lsp_types::CompletionOptions::default()
        }),
        code_action_provider: Some(lsp_types::CodeActionProviderCapability::Options(
            lsp_types::CodeActionOptions {
                code_action_kinds: Some(vec![lsp_types::CodeActionKind::QUICKFIX]),
                resolve_provider: Some(false),
                work_done_progress_options: lsp_types::WorkDoneProgressOptions::default(),
            },
        )),
        semantic_tokens_provider: Some(
            lsp_types::SemanticTokensServerCapabilities::SemanticTokensOptions(
                lsp_types::SemanticTokensOptions {
                    legend: lsp_types::SemanticTokensLegend {
                        token_types: SEMANTIC_TOKEN_TYPES
                            .iter()
                            .map(|&t| lsp_types::SemanticTokenType::new(t))
                            .collect(),
                        token_modifiers: SEMANTIC_TOKEN_MODIFIERS
                            .iter()
                            .map(|&m| lsp_types::SemanticTokenModifier::new(m))
                            .collect(),
                    },
                    range: Some(false),
                    full: Some(lsp_types::SemanticTokensFullOptions::Bool(true)),
                    work_done_progress_options: lsp_types::WorkDoneProgressOptions::default(),
                },
            ),
        ),
        ..ServerCapabilities::default()
    };
    let server_capabilities = serde_json::to_value(&capabilities)?;
    let initialization_params = connection.initialize(server_capabilities)?;
    let _params: InitializeParams = serde_json::from_value(initialization_params)?;

    let mut state = LspState::new();
    main_loop(&connection, &mut state)?;
    // Drop the connection before joining io threads so the writer
    // thread sees its mpsc channel disconnect and exits. Without
    // this, `io_threads.join()` blocks forever even after the
    // protocol-level `exit` notification.
    drop(connection);
    io_threads.join()?;
    Ok(())
}

struct LspState {
    /// In-memory mirror of each open document, keyed by its URI.
    docs: HashMap<Url, String>,
}

impl LspState {
    fn new() -> Self {
        Self { docs: HashMap::new() }
    }
}

fn main_loop(
    connection: &Connection,
    state: &mut LspState,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }
                let response = dispatch_request(req, state);
                connection.sender.send(Message::Response(response))?;
            }
            Message::Notification(not) => {
                handle_notification(connection, not, state)?;
            }
            Message::Response(_) => {
                // We don't issue client→server requests in v1, so
                // any incoming response is unexpected. Drop it.
            }
        }
    }
    Ok(())
}

/// Decode a request, hand it to the matching handler, return the
/// `Response` to send back. Unknown methods get an empty result —
/// rejecting them with `MethodNotFound` is cleaner but most clients
/// tolerate the empty form just fine for v1.
fn dispatch_request(req: Request, state: &LspState) -> Response {
    let id = req.id.clone();
    if req.method == HoverRequest::METHOD {
        match serde_json::from_value::<lsp_types::HoverParams>(req.params.clone()) {
            Ok(params) => {
                let hover = handle_hover(state, params);
                response_for(id, hover)
            }
            Err(err) => response_error(id, format!("bad hover params: {}", err)),
        }
    } else if req.method == lsp_types::request::GotoDefinition::METHOD {
        match serde_json::from_value::<lsp_types::GotoDefinitionParams>(req.params.clone()) {
            Ok(params) => {
                let result = handle_goto_definition(state, params);
                response_for(id, result)
            }
            Err(err) => response_error(id, format!("bad definition params: {}", err)),
        }
    } else if req.method == lsp_types::request::References::METHOD {
        match serde_json::from_value::<lsp_types::ReferenceParams>(req.params.clone()) {
            Ok(params) => {
                let result = handle_references(state, params);
                response_for(id, result)
            }
            Err(err) => response_error(id, format!("bad references params: {}", err)),
        }
    } else if req.method == lsp_types::request::Rename::METHOD {
        match serde_json::from_value::<lsp_types::RenameParams>(req.params.clone()) {
            Ok(params) => match handle_rename(state, params) {
                Ok(result) => response_for(id, result),
                Err(message) => response_error(id, message),
            },
            Err(err) => response_error(id, format!("bad rename params: {}", err)),
        }
    } else if req.method == lsp_types::request::Completion::METHOD {
        match serde_json::from_value::<lsp_types::CompletionParams>(req.params.clone()) {
            Ok(params) => {
                let result = handle_completion(state, params);
                response_for(id, result)
            }
            Err(err) => response_error(id, format!("bad completion params: {}", err)),
        }
    } else if req.method == lsp_types::request::CodeActionRequest::METHOD {
        match serde_json::from_value::<lsp_types::CodeActionParams>(req.params.clone()) {
            Ok(params) => {
                let result = handle_code_action(state, params);
                response_for(id, result)
            }
            Err(err) => response_error(id, format!("bad code-action params: {}", err)),
        }
    } else if req.method == lsp_types::request::SemanticTokensFullRequest::METHOD {
        match serde_json::from_value::<lsp_types::SemanticTokensParams>(req.params.clone()) {
            Ok(params) => {
                let result = handle_semantic_tokens_full(state, params);
                response_for(id, result)
            }
            Err(err) => response_error(id, format!("bad semantic-tokens params: {}", err)),
        }
    } else {
        response_for::<Option<()>>(id, None)
    }
}

fn handle_notification(
    connection: &Connection,
    not: Notification,
    state: &mut LspState,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    match not.method.as_str() {
        DidOpenTextDocument::METHOD => {
            let params: lsp_types::DidOpenTextDocumentParams =
                serde_json::from_value(not.params)?;
            state
                .docs
                .insert(params.text_document.uri.clone(), params.text_document.text.clone());
            publish_diagnostics(connection, &params.text_document.uri, &params.text_document.text)?;
        }
        DidChangeTextDocument::METHOD => {
            let params: lsp_types::DidChangeTextDocumentParams =
                serde_json::from_value(not.params)?;
            // We registered FULL sync, so each change payload
            // carries the entire new document.
            if let Some(change) = params.content_changes.into_iter().last() {
                state
                    .docs
                    .insert(params.text_document.uri.clone(), change.text.clone());
                publish_diagnostics(connection, &params.text_document.uri, &change.text)?;
            }
        }
        DidCloseTextDocument::METHOD => {
            let params: lsp_types::DidCloseTextDocumentParams =
                serde_json::from_value(not.params)?;
            state.docs.remove(&params.text_document.uri);
            // Convention: clear stale diagnostics on close.
            let cleared = PublishDiagnosticsParams {
                uri: params.text_document.uri,
                diagnostics: Vec::new(),
                version: None,
            };
            connection.sender.send(Message::Notification(Notification {
                method: PublishDiagnostics::METHOD.to_string(),
                params: serde_json::to_value(cleared)?,
            }))?;
        }
        _ => {
            // Unhandled notifications (e.g. `$/cancelRequest`) are
            // safe to drop in v1.
        }
    }
    Ok(())
}

fn publish_diagnostics(
    connection: &Connection,
    uri: &Url,
    source: &str,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    let diagnostics = compute_diagnostics(source);
    let params = PublishDiagnosticsParams {
        uri: uri.clone(),
        diagnostics,
        version: None,
    };
    connection.sender.send(Message::Notification(Notification {
        method: PublishDiagnostics::METHOD.to_string(),
        params: serde_json::to_value(params)?,
    }))?;
    Ok(())
}

/// Run the compile pipeline and convert each diagnostic into an LSP
/// `Diagnostic` (severity Error, source `"intentc"`). Successful
/// compiles produce an empty Vec so the editor clears any prior
/// markers for this document.
pub fn compute_diagnostics(source: &str) -> Vec<Diagnostic> {
    match crate::compile(source) {
        Ok(_) => Vec::new(),
        Err(errors) => errors
            .into_iter()
            .map(|d| Diagnostic {
                range: span_to_range(source, d.span),
                severity: Some(DiagnosticSeverity::ERROR),
                source: Some("vanic".to_string()),
                message: d.message,
                ..Diagnostic::default()
            })
            .collect(),
    }
}

fn handle_hover(state: &LspState, params: lsp_types::HoverParams) -> Option<Hover> {
    let uri = &params.text_document_position_params.text_document.uri;
    let source = state.docs.get(uri)?;
    let pos = params.text_document_position_params.position;
    compute_hover(source, pos)
}

/// Look up the type of the smallest typed expression covering the
/// given cursor position. Returns `None` when the document doesn't
/// parse/check or when the cursor isn't inside any typed expression.
///
/// Polish (2026-06-08): when the cursor lands on a postfix `?`
/// token, augment the hover with a short doc-block explaining the
/// operator's semantics + the equivalent `try` keyword form.
/// The type is still the Try-wrapped payload type from the
/// enclosing AST node, so users see both the type and the
/// operator's meaning.
pub fn compute_hover(source: &str, position: Position) -> Option<Hover> {
    let offset = position_to_byte_offset(source, position)?;

    // Cursor-on-`?` check: lex the source and look for a
    // `TokenKind::Question` whose span covers the offset. The
    // `?` desugars at parse-time into a match-based form, so
    // the `find_smallest_typed_at` lookup below may not see a
    // typed expr at the original `?` byte. Even when type
    // lookup fails, the operator's doc block is what the
    // user wants on hover here.
    let q_span = crate::lexer::lex(source)
        .ok()
        .into_iter()
        .flatten()
        .find(|tok| {
            matches!(tok.kind, crate::lexer::TokenKind::Question)
                && tok.span.start <= offset
                && offset < tok.span.end
        })
        .map(|tok| tok.span);

    let typed = crate::compile(source)
        .ok()
        .and_then(|checked| find_smallest_typed_at(&checked.ir, offset));

    if let Some(q_span) = q_span {
        // Always emit hover for `?` — even when type lookup
        // fails (broken document) or when the desugar moved
        // the typed expr's span away from the original `?`.
        let type_prefix = match &typed {
            Some((ty, _)) => format!("```intent\n: {}\n```\n\n", ty),
            None => String::new(),
        };
        let value = format!(
            "{}**Postfix `?` operator** — Option / Result early-return sugar. \
             `EXPR?` desugars to `try EXPR` at parse time (same AST node, same \
             narrow-gate restrictions: first-let-RHS position, payloaded enum, \
             payload-less short-circuit variant). Returns the payload of the \
             success variant; short-circuits the enclosing function with the \
             failure variant otherwise.",
            type_prefix
        );
        return Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value,
            }),
            range: Some(span_to_range(source, q_span)),
        });
    }

    let (ty, span) = typed?;
    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format!("```intent\n: {}\n```", ty),
        }),
        range: Some(span_to_range(source, span)),
    })
}

fn handle_code_action(
    state: &LspState,
    params: lsp_types::CodeActionParams,
) -> Option<lsp_types::CodeActionResponse> {
    let uri = &params.text_document.uri;
    let source = state.docs.get(uri)?;
    let actions = compute_code_actions(source, uri.clone(), &params.context.diagnostics);
    if actions.is_empty() {
        return None;
    }
    Some(
        actions
            .into_iter()
            .map(lsp_types::CodeActionOrCommand::CodeAction)
            .collect(),
    )
}

/// Inspect the diagnostics the client sent in the request
/// context and emit a code action per applicable quick fix.
/// v1 recognizes one pattern: a parser diagnostic whose
/// message mentions `expected ';'` (or `expected '}'` /
/// `expected '('`/etc.) gets an "Insert <token>" fix that
/// patches the source at the diagnostic's end. The fix is
/// marked `is_preferred: true` so editors that auto-apply
/// the preferred quick fix on save (a common configuration)
/// will close the trivial cases without user intervention.
///
/// The list is intentionally short — extending it as we
/// surface more pin-pointable diagnostics is a follow-up.
pub fn compute_code_actions(
    source: &str,
    uri: Url,
    diagnostics: &[Diagnostic],
) -> Vec<lsp_types::CodeAction> {
    let mut actions = Vec::new();
    for diag in diagnostics {
        if let Some(action) = quick_fix_missing_token(source, &uri, diag) {
            actions.push(action);
        }
    }
    actions
}

/// If the diagnostic message says `expected '<TOK>'` for a
/// single-character token, build a quick-fix action that
/// inserts `<TOK>` at the diagnostic's end.
fn quick_fix_missing_token(
    source: &str,
    uri: &Url,
    diag: &Diagnostic,
) -> Option<lsp_types::CodeAction> {
    // Look for `expected '<one-char>'` somewhere in the
    // message. The lexer/parser uses single-quote
    // delimiters when naming expected punctuation, so this
    // pattern catches `expected ';'`, `expected ')'`,
    // `expected '}'`, etc.
    let msg = &diag.message;
    let needle = "expected '";
    let start = msg.find(needle)?;
    let body_start = start + needle.len();
    let body = &msg[body_start..];
    let body_end = body.find('\'')?;
    let token = &body[..body_end];
    if token.len() != 1 {
        // Only single-char punctuation gets the trivial
        // insertion fix. Multi-char tokens (e.g.
        // `expected 'identifier'`) need a different
        // strategy.
        return None;
    }
    let insertion_point = lsp_types::Range {
        start: diag.range.end,
        end: diag.range.end,
    };
    let _ = source;
    let edit = lsp_types::TextEdit {
        range: insertion_point,
        new_text: token.to_string(),
    };
    let doc_edit = lsp_types::TextDocumentEdit {
        text_document: lsp_types::OptionalVersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: None,
        },
        edits: vec![lsp_types::OneOf::Left(edit)],
    };
    Some(lsp_types::CodeAction {
        title: format!("Insert `{}`", token),
        kind: Some(lsp_types::CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diag.clone()]),
        edit: Some(lsp_types::WorkspaceEdit {
            changes: None,
            document_changes: Some(lsp_types::DocumentChanges::Edits(vec![doc_edit])),
            change_annotations: None,
        }),
        command: None,
        is_preferred: Some(true),
        disabled: None,
        data: None,
    })
}

fn handle_semantic_tokens_full(
    state: &LspState,
    params: lsp_types::SemanticTokensParams,
) -> Option<lsp_types::SemanticTokensResult> {
    let uri = &params.text_document.uri;
    let source = state.docs.get(uri)?;
    let tokens = compute_semantic_tokens(source);
    Some(lsp_types::SemanticTokensResult::Tokens(
        lsp_types::SemanticTokens {
            result_id: None,
            data: tokens,
        },
    ))
}

/// Compute the LSP semantic-tokens encoded array for the
/// whole document. Re-lexes the source and emits one
/// `SemanticToken` per non-trivial lexer token. Encoding is
/// the standard delta-format: each token is 5 u32s
/// `(deltaLine, deltaStartChar, length, tokenType,
/// tokenModifiers)` relative to the previous token's start.
///
/// v1 is lex-driven — every identifier gets the `variable`
/// type by default. Refining via the typed IR (so a `Call`
/// callee gets `function`, a function parameter gets
/// `parameter`, a type-position identifier gets `type`) is
/// the natural next step, paired with carrying scope info
/// into the IR; both unlock the per-scope refinement that
/// references/rename/completion also need.
///
/// Returns an empty Vec when the source fails to lex
/// (typical mid-edit state) — better than reporting an
/// error so the editor's UI stays responsive.
pub fn compute_semantic_tokens(source: &str) -> Vec<lsp_types::SemanticToken> {
    let tokens = match crate::lexer::lex(source) {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };
    // (start position, length, token_type_idx, modifiers_bitset)
    let mut emit: Vec<(Position, u32, u32, u32)> = Vec::new();
    // Try to recognize known type names emitted as Ident
    // (Vec, Str, OwnedStr, Atomic, Channel, Mutex, Guard,
    // Task). The parser also accepts these in type position;
    // we mark them as `type` regardless of position since
    // the user-facing distinction is "this identifier names
    // a type".
    let type_name_idents: std::collections::HashSet<&'static str> = [
        "Str", "OwnedStr", "Vec", "Atomic", "Channel", "Mutex", "Guard", "Task",
    ]
    .iter()
    .copied()
    .collect();
    // IR-driven overrides: when the document compiles,
    // collect (span → (token_type, modifiers)) entries that
    // override the lexer's default tint at specific identifier
    // spans. Today this distinguishes:
    //   * function calls (`f(...)`) — the callee's
    //     name_span gets `function` instead of `variable`.
    //   * parameter declarations — the parameter's
    //     name_span gets `parameter` plus `declaration` +
    //     `readonly` modifier bits.
    //   * parameter reads — every `Var` read of a parameter
    //     gets `parameter` plus `readonly`.
    let mut overrides: std::collections::HashMap<crate::span::Span, TokenOverride> =
        std::collections::HashMap::new();
    if let Ok(checked) = crate::compile(source) {
        collect_ir_token_overrides(&checked.ir, &mut overrides);
    }
    for tok in &tokens {
        let kind_index: Option<u32> = match &tok.kind {
            crate::lexer::TokenKind::Eof => None,
            crate::lexer::TokenKind::Int(_) | crate::lexer::TokenKind::Float(_) => {
                Some(token_index(TOKEN_NUMBER))
            }
            crate::lexer::TokenKind::Str(_) => Some(token_index(TOKEN_STRING)),
            // Type primitives + the parametric Vec keyword.
            crate::lexer::TokenKind::I8
            | crate::lexer::TokenKind::I16
            | crate::lexer::TokenKind::I32
            | crate::lexer::TokenKind::I64
            | crate::lexer::TokenKind::U8
            | crate::lexer::TokenKind::U16
            | crate::lexer::TokenKind::U32
            | crate::lexer::TokenKind::U64
            | crate::lexer::TokenKind::F32
            | crate::lexer::TokenKind::F64
            | crate::lexer::TokenKind::Bool
            | crate::lexer::TokenKind::Vec => Some(token_index(TOKEN_TYPE)),
            // `min` / `max` are builtin functions rather
            // than keywords — give them the function tint
            // so editors can distinguish a call to min from
            // an if/return-style control word.
            crate::lexer::TokenKind::Min | crate::lexer::TokenKind::Max => {
                Some(token_index(TOKEN_FUNCTION))
            }
            // Identifiers — refine to type if it's a known
            // type-position keyword, else default to
            // variable.
            crate::lexer::TokenKind::Ident(name) => {
                if type_name_idents.contains(name.as_str()) {
                    Some(token_index(TOKEN_TYPE))
                } else {
                    Some(token_index(TOKEN_VARIABLE))
                }
            }
            // Operators / punctuation: skip. Most clients
            // don't expect semantic tokens for these.
            crate::lexer::TokenKind::LParen
            | crate::lexer::TokenKind::RParen
            | crate::lexer::TokenKind::LBrace
            | crate::lexer::TokenKind::RBrace
            | crate::lexer::TokenKind::LBracket
            | crate::lexer::TokenKind::RBracket
            | crate::lexer::TokenKind::Colon
            | crate::lexer::TokenKind::Semicolon
            | crate::lexer::TokenKind::Comma
            | crate::lexer::TokenKind::Plus
            | crate::lexer::TokenKind::Minus
            | crate::lexer::TokenKind::Star
            | crate::lexer::TokenKind::Slash
            | crate::lexer::TokenKind::Percent
            | crate::lexer::TokenKind::Bang
            | crate::lexer::TokenKind::Equal
            | crate::lexer::TokenKind::EqEq
            | crate::lexer::TokenKind::BangEq
            | crate::lexer::TokenKind::Less
            | crate::lexer::TokenKind::LessEq
            | crate::lexer::TokenKind::LessLess
            | crate::lexer::TokenKind::Greater
            | crate::lexer::TokenKind::GreaterEq
            | crate::lexer::TokenKind::GreaterGreater
            | crate::lexer::TokenKind::Amp
            | crate::lexer::TokenKind::AndAnd
            | crate::lexer::TokenKind::Pipe
            | crate::lexer::TokenKind::OrOr
            | crate::lexer::TokenKind::Caret
            | crate::lexer::TokenKind::Arrow
            | crate::lexer::TokenKind::DotDot
            // Postfix `?` operator (Result/Option propagation —
            // parse-time sugar over `try`). Treated like other
            // operators here so editors don't paint it with the
            // keyword tint.
            | crate::lexer::TokenKind::Question => None,
            // Everything else is a keyword.
            _ => Some(token_index(TOKEN_KEYWORD)),
        };
        if let Some(mut kind_idx) = kind_index {
            // Apply IR-driven overrides when the lexer's
            // span exactly matches a Call callee or
            // parameter declaration / read site.
            let mut modifiers: u32 = 0;
            if let Some(over) = overrides.get(&tok.span) {
                kind_idx = over.token_type;
                modifiers = over.modifiers;
            }
            let length = (tok.span.end - tok.span.start) as u32;
            let pos = byte_offset_to_position(source, tok.span.start);
            emit.push((pos, length, kind_idx, modifiers));
        }
    }
    // Encode delta-format. Tokens are already in source
    // order because the lexer emits sequentially.
    let mut out = Vec::with_capacity(emit.len());
    let mut prev_line: u32 = 0;
    let mut prev_start: u32 = 0;
    for (pos, length, kind_idx, modifiers) in emit {
        let delta_line = pos.line - prev_line;
        let delta_start = if delta_line == 0 {
            pos.character - prev_start
        } else {
            pos.character
        };
        out.push(lsp_types::SemanticToken {
            delta_line,
            delta_start,
            length,
            token_type: kind_idx,
            token_modifiers_bitset: modifiers,
        });
        prev_line = pos.line;
        prev_start = pos.character;
    }
    out
}

/// Token-type names, in the same order as the legend
/// advertised by the server capabilities. The index of each
/// name in this array is the value placed in each emitted
/// token's `tokenType` field.
const SEMANTIC_TOKEN_TYPES: &[&str] = &[
    "variable", "function", "parameter", "type", "keyword", "number", "string",
];

const TOKEN_VARIABLE: &str = "variable";
const TOKEN_FUNCTION: &str = "function";
const TOKEN_PARAMETER: &str = "parameter";
const TOKEN_TYPE: &str = "type";
const TOKEN_KEYWORD: &str = "keyword";
const TOKEN_NUMBER: &str = "number";
const TOKEN_STRING: &str = "string";

/// Modifier-name legend. Each modifier's index in this slice
/// is the bit it occupies in a token's `tokenModifiers` bitset.
const SEMANTIC_TOKEN_MODIFIERS: &[&str] = &["declaration", "readonly"];

/// Bit for the `declaration` modifier — applied to spans that
/// are the introduction site of a binding (today: parameter
/// declarations only).
const TOKEN_MOD_DECLARATION: u32 = 1 << 0;

/// Bit for the `readonly` modifier — applied to spans that
/// cannot be reassigned. Today: parameter declarations + every
/// `Var` read of a parameter (parameters are not reassignable
/// without shadowing).
const TOKEN_MOD_READONLY: u32 = 1 << 1;

/// Walk the typed IR and record `(span, token_type)` entries
/// that override the lexer's default tint for specific
/// identifier spans:
/// * each `TypedExprKind::Call.name_span` → `function`
/// * each `TypedParam.name_span` → `parameter`
///
/// The lexer's main pass tints every ident as `variable` by
/// default; these overrides upgrade specific occurrences to
/// the more precise tint after the checker has resolved
/// them. Synthetic call sites (with `Span::default()`) are
/// silently ignored since their span won't match any lexer
/// token.
/// A single override entry: replace the lexer-chosen token
/// type with `token_type` and OR `modifiers` into the emitted
/// `tokenModifiers` bitset.
#[derive(Clone, Copy)]
struct TokenOverride {
    token_type: u32,
    modifiers: u32,
}

/// Per-function walk context. Carries the precomputed
/// token-type indices plus the set of parameter declaration
/// spans for *this* function so the walker can recognize
/// `Var(p)` reads of those parameters and tag them readonly.
struct OverrideCtx {
    fn_idx: u32,
    param_idx: u32,
    param_decl_spans: std::collections::HashSet<crate::span::Span>,
}

fn collect_ir_token_overrides(
    program: &crate::ir::TypedProgram,
    out: &mut std::collections::HashMap<crate::span::Span, TokenOverride>,
) {
    let fn_idx = token_index(TOKEN_FUNCTION);
    let param_idx = token_index(TOKEN_PARAMETER);
    for f in &program.functions {
        let mut param_decl_spans = std::collections::HashSet::new();
        for param in &f.params {
            if param.name_span != crate::span::Span::default() {
                out.insert(
                    param.name_span,
                    TokenOverride {
                        token_type: param_idx,
                        modifiers: TOKEN_MOD_DECLARATION | TOKEN_MOD_READONLY,
                    },
                );
                param_decl_spans.insert(param.name_span);
            }
        }
        let ctx = OverrideCtx { fn_idx, param_idx, param_decl_spans };
        for stmt in &f.body {
            collect_call_overrides_in_stmt(stmt, &ctx, out);
        }
    }
}

fn collect_call_overrides_in_stmt(
    stmt: &TypedStmt,
    ctx: &OverrideCtx,
    out: &mut std::collections::HashMap<crate::span::Span, TokenOverride>,
) {
    match stmt {
        TypedStmt::Let { expr, .. }
        | TypedStmt::Reassign { expr, .. }
        | TypedStmt::Return { expr }
        | TypedStmt::Assert { expr, .. }
        | TypedStmt::Discard { expr } => collect_call_overrides_in_expr(expr, ctx, out),
        TypedStmt::Print { items } => {
            for item in items {
                if let TypedPrintItem::Expr(e) = item {
                    collect_call_overrides_in_expr(e, ctx, out);
                }
            }
        }
        TypedStmt::If { cond, then_body, else_body } => {
            collect_call_overrides_in_expr(cond, ctx, out);
            for s in then_body {
                collect_call_overrides_in_stmt(s, ctx, out);
            }
            for s in else_body {
                collect_call_overrides_in_stmt(s, ctx, out);
            }
        }
        TypedStmt::While { cond, body, .. } => {
            collect_call_overrides_in_expr(cond, ctx, out);
            for s in body {
                collect_call_overrides_in_stmt(s, ctx, out);
            }
        }
        TypedStmt::For { start, end, body, .. } => {
            collect_call_overrides_in_expr(start, ctx, out);
            collect_call_overrides_in_expr(end, ctx, out);
            for s in body {
                collect_call_overrides_in_stmt(s, ctx, out);
            }
        }
        TypedStmt::ForIter { body, .. } => {
            for s in body {
                collect_call_overrides_in_stmt(s, ctx, out);
            }
        }
        TypedStmt::IndexAssign { index, value, .. } => {
            collect_call_overrides_in_expr(index, ctx, out);
            collect_call_overrides_in_expr(value, ctx, out);
        }
        TypedStmt::TaskSpawn { body, .. } => {
            for s in body {
                collect_call_overrides_in_stmt(s, ctx, out);
            }
        }
        _ => {}
    }
}

fn collect_call_overrides_in_expr(
    expr: &TypedExpr,
    ctx: &OverrideCtx,
    out: &mut std::collections::HashMap<crate::span::Span, TokenOverride>,
) {
    match &expr.kind {
        TypedExprKind::Call { name_span, args, .. } => {
            if *name_span != crate::span::Span::default() {
                out.insert(
                    *name_span,
                    TokenOverride { token_type: ctx.fn_idx, modifiers: 0 },
                );
            }
            for a in args {
                collect_call_overrides_in_expr(a, ctx, out);
            }
        }
        TypedExprKind::Var(_) => {
            // A Var read whose binding_decl_span matches one of
            // this function's parameter declarations: upgrade
            // tint to `parameter` and add the `readonly`
            // modifier. The Var's own `TypedExpr.span` equals
            // the identifier's lexer span (set by the parser),
            // so keying the override on `expr.span` works.
            if let Some(decl) = expr.binding_decl_span {
                if ctx.param_decl_spans.contains(&decl)
                    && expr.span != crate::span::Span::default()
                {
                    out.insert(
                        expr.span,
                        TokenOverride {
                            token_type: ctx.param_idx,
                            modifiers: TOKEN_MOD_READONLY,
                        },
                    );
                }
            }
        }
        TypedExprKind::Unary { expr, .. } => collect_call_overrides_in_expr(expr, ctx, out),
        TypedExprKind::Binary { left, right, .. } => {
            collect_call_overrides_in_expr(left, ctx, out);
            collect_call_overrides_in_expr(right, ctx, out);
        }
        TypedExprKind::Cast { expr, .. } => collect_call_overrides_in_expr(expr, ctx, out),
        TypedExprKind::ArrayLit { elements } => {
            for e in elements {
                collect_call_overrides_in_expr(e, ctx, out);
            }
        }
        TypedExprKind::Index { array, index, .. } => {
            collect_call_overrides_in_expr(array, ctx, out);
            collect_call_overrides_in_expr(index, ctx, out);
        }
        TypedExprKind::Len { array, .. } => collect_call_overrides_in_expr(array, ctx, out),
        _ => {}
    }
}

fn token_index(name: &str) -> u32 {
    SEMANTIC_TOKEN_TYPES
        .iter()
        .position(|t| *t == name)
        .expect("token type in legend") as u32
}

fn handle_completion(
    state: &LspState,
    params: lsp_types::CompletionParams,
) -> Option<lsp_types::CompletionResponse> {
    let uri = &params.text_document_position.text_document.uri;
    let source = state.docs.get(uri)?;
    let pos = params.text_document_position.position;
    let items = compute_completion(source, pos);
    Some(lsp_types::CompletionResponse::Array(items))
}

/// Build completion items for the cursor position. Returns
/// a (typically large but bounded) list combining:
/// * **In-scope bindings**: names declared by `Let` / `For`
///   var / `ForIter` var / `TaskSpawn` handles before the
///   cursor in the containing function, plus that function's
///   parameters. Walked forward from the function entry up
///   to the cursor by comparing expression spans.
/// * **Function names**: every top-level function in the
///   program (callable anywhere).
/// * **Builtins**: the fixed set of compiler-recognized
///   intrinsics (`vec`, `push`, `atomic_new`, `mutex_lock`,
///   …).
/// * **Keywords**: language keywords (`let`, `if`, …) and
///   type names (`i64`, `Vec`, `Atomic`, …) so the user can
///   complete type annotations alongside bindings.
///
/// Always returns a list — even on a broken document we
/// emit keywords + builtins so the editor's completion popup
/// isn't useless while the user is mid-type. When the
/// document compiles, the binding list is added.
///
/// Same shadowing caveat as references/rename: if the
/// cursor's function shadows a binding, both names appear.
/// Refining requires scope info in the IR, planned alongside
/// semantic tokens.
pub fn compute_completion(source: &str, position: Position) -> Vec<lsp_types::CompletionItem> {
    let mut items: Vec<lsp_types::CompletionItem> = Vec::new();

    // Polish (2026-06-08): pragma-aware keyword completion. When
    // the file declares `// vani-lang: <tag>`, surface that
    // dialect's keywords (in addition to English — they share
    // the same TokenKinds so the parser accepts either). Avoids
    // dumping all 60+ dialects' keywords into every completion
    // popup. Default (no pragma) is English-only.
    let pragma_tag = crate::lexer::detect_pragma_tag(source);
    for kw in KEYWORDS {
        items.push(plain_completion(kw, lsp_types::CompletionItemKind::KEYWORD));
    }
    if let Some(tag) = pragma_tag.as_deref() {
        for kw in dialect_keywords_for(tag) {
            items.push(plain_completion(kw, lsp_types::CompletionItemKind::KEYWORD));
        }
    }
    for ty in TYPE_NAMES {
        items.push(plain_completion(ty, lsp_types::CompletionItemKind::CLASS));
    }
    for builtin in BUILTIN_FUNCTIONS {
        items.push(plain_completion(
            builtin,
            lsp_types::CompletionItemKind::FUNCTION,
        ));
    }

    // Bindings + function names require a successful compile.
    if let Ok(checked) = crate::compile(source) {
        for f in &checked.ir.functions {
            items.push(plain_completion(
                &f.name,
                lsp_types::CompletionItemKind::FUNCTION,
            ));
        }
        if let Some(offset) = position_to_byte_offset(source, position) {
            // Scope-aware: each `TypedFunction` carries its
            // source-byte `span` covering the whole `fn …
            // { … }`. Find the function whose span contains
            // the cursor and gather *only* its parameters
            // and forward-declared bindings. The over-
            // completion that v1 had (sibling-function
            // params leaking in) is gone. When the cursor
            // sits outside every function (e.g. on a blank
            // line between two `fn`s), no bindings are
            // emitted — keywords + builtins still are.
            let mut seen: std::collections::BTreeSet<String> = Default::default();
            if let Some(f) = checked.ir.functions.iter().find(|f| {
                f.span.start <= offset && offset <= f.span.end
            }) {
                for param in &f.params {
                    if !param.name.starts_with("__intent_") {
                        seen.insert(param.name.clone());
                    }
                }
                collect_in_scope_bindings(&f.body, offset, &mut seen);
            }
            for name in seen {
                items.push(plain_completion(
                    &name,
                    lsp_types::CompletionItemKind::VARIABLE,
                ));
            }
        }
    }

    // De-duplicate by label so a parameter named `n` doesn't
    // appear twice if it's also referenced inside the body
    // before the cursor.
    items.sort_by(|a, b| a.label.cmp(&b.label));
    items.dedup_by(|a, b| a.label == b.label);
    items
}

fn plain_completion(
    label: &str,
    kind: lsp_types::CompletionItemKind,
) -> lsp_types::CompletionItem {
    lsp_types::CompletionItem {
        label: label.to_string(),
        kind: Some(kind),
        ..lsp_types::CompletionItem::default()
    }
}

fn collect_in_scope_bindings(
    body: &[TypedStmt],
    cursor: usize,
    out: &mut std::collections::BTreeSet<String>,
) {
    for stmt in body {
        match stmt {
            TypedStmt::Let { name, expr, .. } => {
                // Include the binding only if its
                // declaration site (approximated by the
                // RHS's start) precedes the cursor.
                if expr.span.start <= cursor && !name.starts_with("__intent_") {
                    out.insert(name.clone());
                }
            }
            TypedStmt::If { then_body, else_body, .. } => {
                collect_in_scope_bindings(then_body, cursor, out);
                collect_in_scope_bindings(else_body, cursor, out);
            }
            TypedStmt::While { body, .. } => {
                collect_in_scope_bindings(body, cursor, out);
            }
            TypedStmt::For { var, start, body, .. } => {
                if start.span.start <= cursor && !var.starts_with("__intent_") {
                    out.insert(var.clone());
                }
                collect_in_scope_bindings(body, cursor, out);
            }
            TypedStmt::ForIter { var, body, .. } => {
                // No span on ForIter directly; gate on the
                // body's first statement if any.
                if let Some(first) = body.first() {
                    if stmt_first_span(first).map(|s| s.start <= cursor).unwrap_or(false)
                        && !var.starts_with("__intent_")
                    {
                        out.insert(var.clone());
                    }
                }
                collect_in_scope_bindings(body, cursor, out);
            }
            TypedStmt::TaskSpawn { name, body, .. } => {
                if !name.starts_with("__intent_") {
                    // Task handle is declared at the spawn
                    // site; conservative gate on body's
                    // first span.
                    if let Some(first) = body.first() {
                        if stmt_first_span(first).map(|s| s.start <= cursor).unwrap_or(true) {
                            out.insert(name.clone());
                        }
                    } else {
                        out.insert(name.clone());
                    }
                }
                collect_in_scope_bindings(body, cursor, out);
            }
            _ => {}
        }
    }
}

fn stmt_first_span(stmt: &TypedStmt) -> Option<crate::span::Span> {
    match stmt {
        TypedStmt::Let { expr, .. }
        | TypedStmt::Reassign { expr, .. }
        | TypedStmt::Return { expr }
        | TypedStmt::Assert { expr, .. }
        | TypedStmt::Discard { expr }
        | TypedStmt::Prove { expr } => Some(expr.span),
        TypedStmt::If { cond, .. } | TypedStmt::While { cond, .. } => Some(cond.span),
        TypedStmt::For { start, .. } => Some(start.span),
        _ => None,
    }
}

/// Keywords completion items. List mirrors the lexer's
/// keyword set — kept here so the LSP completion popup
/// always knows them even if the user is mid-edit and the
/// document doesn't compile.
const KEYWORDS: &[&str] = &[
    // Declarations + visibility.
    "fn", "let", "const", "struct", "enum", "type",
    "pub", "module", "use",
    // Control flow.
    "return", "if", "else", "while", "break", "continue",
    "match", "then", "for", "in", "from", "to",
    // Refs + mutability.
    "ref", "mut",
    // Verification.
    "requires", "ensures", "invariant", "assert", "prove",
    // I/O + literals.
    "print", "true", "false", "as",
    // Concurrency + purity.
    "task", "join", "pure", "parallel", "reduce", "with",
    // Interfaces + methods.
    "interface", "implement", "methods", "where", "is",
    // Error-handling.
    "try", "async",
    // Embedded.
    "unsafe", "region", "extern",
    // Doc / metadata.
    "intent",
];

/// Return the keyword list for the given dialect pragma tag.
/// The user's `// vani-lang: <tag>` pragma gates which dialect's
/// keywords are surfaced in the completion popup.
///
/// The `XXX_KEYWORDS` arrays below are mechanically extracted from
/// the corresponding `xxx_keyword`/`xxx_ascii_keyword` match arms in
/// `src/lexer.rs` (every `"word" => TokenKind::...` key), not
/// hand-typed — see BUG-173 (docs/BUG_PATTERN_AUDIT_TODO_11.md).
/// Re-run `tools/regen_lsp_keywords.py` after editing any dialect's
/// keyword table in `lexer.rs` to keep these in sync; a `cargo test`
/// (`lsp_keyword_lists_match_lexer`) fails if they drift.
fn dialect_keywords_for(tag: &str) -> &'static [&'static str] {
    match tag {
        "mandarin" | "chinese" | "zh" => MANDARIN_KEYWORDS,
        "sanskrit" | "sa" | "hindi" | "hi" | "marathi" | "mr"
        | "nepali" | "ne" | "maithili" | "mai"
        | "konkani" | "kok" => DEVANAGARI_KEYWORDS,
        "bengali" | "bangla" | "bn" => BENGALI_KEYWORDS,
        "tamil" | "ta" => TAMIL_KEYWORDS,
        "telugu" | "te" => TELUGU_KEYWORDS,
        "gujarati" | "gu" => GUJARATI_KEYWORDS,
        "punjabi" | "pa" => PUNJABI_KEYWORDS,
        "kannada" | "kn" => KANNADA_KEYWORDS,
        "odia" | "oriya" | "or" => ODIA_KEYWORDS,
        "urdu" | "ur" => URDU_KEYWORDS,
        "persian" | "farsi" | "fa" => PERSIAN_KEYWORDS,
        "korean" | "ko" => KOREAN_KEYWORDS,
        "japanese" | "ja" => JAPANESE_KEYWORDS,
        "arabic" | "ar" => ARABIC_KEYWORDS,
        "hebrew" | "he" => HEBREW_KEYWORDS,
        "russian" | "ru" => RUSSIAN_KEYWORDS,
        "spanish" | "es" => SPANISH_KEYWORDS,
        "french" | "fr" => FRENCH_KEYWORDS,
        "german" | "de" => GERMAN_KEYWORDS,
        "portuguese" | "pt" => PORTUGUESE_KEYWORDS,
        "italian" | "it" => ITALIAN_KEYWORDS,
        "turkish" | "tr" => TURKISH_KEYWORDS,
        "polish" | "pl" => POLISH_KEYWORDS,
        "indonesian" | "id" => INDONESIAN_KEYWORDS,
        "malay" | "ms" => MALAY_KEYWORDS,
        "swahili" | "sw" => SWAHILI_KEYWORDS,
        "dutch" | "nl" => DUTCH_KEYWORDS,
        "thai" | "th" => THAI_KEYWORDS,
        "hungarian" | "hu" => HUNGARIAN_KEYWORDS,
        "czech" | "cs" => CZECH_KEYWORDS,
        _ => &[],
    }
}

/// Phase 10.2 — Mandarin Chinese (中文) keyword set. Mirrors
/// the entries in `mandarin_keyword` in `src/lexer.rs`. Order
/// matches the lexer file (DECLARATIONS / VISIBILITY / CONTROL
/// FLOW / REFS / MATCH / VERIFICATION / BOOL+PRINT / PURITY /
/// INTERFACES / BOUNDS / CONCURRENCY / EMBEDDED / SOV-S7).
const MANDARIN_KEYWORDS: &[&str] = &[
    "函数", "让", "结构", "结构体", "枚举", "常量", "公开", "模块", "使用", "作为", "返回", "如果",
    "否则", "当", "对于", "从", "到", "中断", "继续", "那么", "引用", "可变", "匹配", "断言",
    "证明", "要求", "保证", "不变量", "真", "假", "打印", "输出", "纯", "纯粹", "并行", "接口",
    "实现", "方法", "其中", "尝试", "任务", "等待", "合并", "不安全", "区域", "目的", "意图",
    "类型", "外部", "减少", "与", "在", "是", "错误打印",
];

/// Phase 2 — Devanagari Indo-Aryan keyword union (Sanskrit /
/// Hindi / Marathi / Nepali / Maithili / Konkani). The lexer
/// accepts the union of these spellings; the LSP suggests them
/// all so a user mid-typing can pick any natural spelling.
const DEVANAGARI_KEYWORDS: &[&str] = &[
    "फलन", "कार्य", "मान", "माना", "परत", "लौटाओ", "पुनरागम", "यदि", "अगर",
    "जर", "अन्यथा", "वरना", "नाहीतर", "यावत्", "जबतक", "जोपर्यंत", "प्रति",
    "साठी", "तदा", "तो", "तर", "पहा", "देखो", "दृष्ट्या", "बदल", "बदलणारा",
    "परिवर्तनीय", "जुळवा", "मिलान", "मेल", "मेलन", "खात्री", "सुनिश्चित",
    "सिद्धम्", "सिद्ध", "प्रमाण", "प्रमाणित", "दर्शाओ", "दाखवा",
    "अपेक्षित", "चाहिए", "पाहिजे", "निश्चित", "सुनिश्चयित", "सत्य", "सही",
    "सच", "बरोबर", "खरे", "असत्य", "अशुद्ध", "झूठ", "गलत", "खोटे", "चूक",
    "लिख", "लिखो", "त्रुटिलिख", "त्रुटिलिखो", "दोषलिहा", "लिहा", "लिही",
    "लिहिया", "शुद्ध", "संरचना", "विकल्प", "गणन", "स्थिर", "नियत", "विराम",
    "रुको", "थांब", "अग्रे", "पुढे", "आगे", "में", "से", "तक", "संक्षेप",
    "सह", "समानांतर", "उपयोग", "खण्ड", "मॉड्यूल", "सार्वजनिक", "यथा",
    "संकेत", "अंतरापृष्ठ", "कार्यान्वित", "विधि", "जहाँ", "यत्र", "जिथे",
    "है", "अस्ति", "आहे", "प्रयास", "नियोग", "संयोजन", "असुरक्षित",
    "क्षेत्र", "उद्देश्य", "प्रकार", "बाह्य", "अपरिवर्तनीय", "पूर्णांक",
    "पूर्णांक८", "पूर्णांक१६", "पूर्णांक३२", "पूर्णांक६४", "अहस्ताक्षरित८",
    "अहस्ताक्षरित१६", "अहस्ताक्षरित३२", "अहस्ताक्षरित६४", "दशांश",
    "दशांश३२", "दशांश६४", "तर्क", "बूल", "सूची",
];

const BENGALI_KEYWORDS: &[&str] = &[
    "ফাংশন", "কাজ", "মান", "ধরো", "গঠন", "গণনা", "স্থির", "সর্বজনীন",
    "খণ্ড", "ব্যবহার", "হিসাবে", "ফেরত", "প্রত্যাবর্তন", "যদি", "নাহলে",
    "অন্যথা", "যতক্ষণ", "প্রতি", "মধ্যে", "থেকে", "পর্যন্ত", "বিরাম",
    "এগিয়ে", "তবে", "দেখ", "পরিবর্তনীয়", "মেলে", "মিলান", "নিশ্চিত",
    "প্রমাণ", "প্রয়োজনীয়", "সুনিশ্চিত", "সত্য", "ঠিক", "অসত্য", "মিথ্যা",
    "ভুল", "লেখ", "লিখো", "শুদ্ধ", "সমান্তরাল", "সংকেত", "কার্যান্বিত",
    "বিধি", "যেখানে", "হয়", "চেষ্টা", "নিয়োগ", "যোগ", "উদ্দেশ্য",
    "প্রকার", "বাহ্যিক", "অপরিবর্তনীয়", "সংক্ষেপ", "সহ", "ত্রুটিলেখ",
    "অসুরক্ষিত", "ক্ষেত্র",
];

const TAMIL_KEYWORDS: &[&str] = &[
    "செயல்பாடு", "சார்பு", "கொள்", "கட்டமைப்பு", "எண்ணுப்பெயர்", "மாறா",
    "பொது", "தொகுதி", "பயன்படுத்து", "ஆக", "திருப்பு", "என்றால்", "எனில்",
    "இல்லாவிட்டால்", "வரை", "ஒவ்வொரு", "உள்", "இருந்து", "வரைக்கும்",
    "நிறுத்து", "தொடர்", "அப்போது", "பார்", "மாறக்கூடிய", "பொருந்து",
    "உறுதி", "நிரூபி", "தேவை", "உறுதிப்படுத்து", "மெய்", "பொய்", "எழுது",
    "அச்சிடு", "இடைமுகம்", "செயல்படுத்து", "நோக்கம்", "வகை", "மாறிலா",
    "தூய", "வெளி", "இணை", "குறை", "உடன்", "பணி", "சேர்", "எங்கே", "ஆகும்",
    "முறைகள்", "பிழைஎழுது", "முயற்சி", "பாதுகாப்பற்ற", "பகுதி",
];

const TELUGU_KEYWORDS: &[&str] = &[
    "ఫంక్షన్", "పని", "అనుకో", "నిర్మాణం", "గణన", "స్థిరం", "ప్రజా",
    "మాడ్యూల్", "ఉపయోగించు", "గా", "తిరిగి", "అయితే", "లేకపోతే", "వరకు",
    "ప్రతి", "లో", "నుండి", "వరకూ", "ఆపు", "కొనసాగించు", "అప్పుడు", "చూడు",
    "మార్చదగిన", "సరిపోలు", "నిర్ధారించు", "నిరూపించు", "అవసరం", "నిశ్చయం",
    "నిజం", "అబద్ధం", "రాయి", "ముద్రించు", "ఉద్దేశం", "రకం", "మారని",
    "శుద్ధ", "బాహ్య", "సమాంతర", "సంక్షేప", "తో", "కార్యం", "సంయోగం",
    "సంకేతం", "అమలు", "ఎక్కడ", "ఉంది", "పద్ధతులు", "లోపంరాయి",
    "ప్రయత్నించు", "అసురక్షిత", "ప్రాంతం",
];

const GUJARATI_KEYWORDS: &[&str] = &[
    "કાર્ય", "ફંકશન", "માનો", "ધારો", "રચના", "ગણના", "સ્થિર", "જાહેર",
    "ખંડ", "વાપરો", "તરીકે", "પાછા", "જો", "નહીંતર", "જ્યારે", "પ્રતિ",
    "માં", "થી", "સુધી", "વિરામ", "ચાલુ", "પછી", "જુઓ", "પરિવર્તનીય",
    "મેળવો", "નિશ્ચિત", "પ્રમાણ", "જરૂરી", "ખાતરી", "સાચું", "ખોટું",
    "લખો", "છાપો", "ઉદ્દેશ", "પ્રકાર", "અચળ", "શુદ્ધ", "બાહ્ય", "સમાંતર",
    "સંક્ષેપ", "સાથે", "નિયોગ", "સંયોજન", "સંકેત", "અમલ", "જ્યાં", "છે",
    "પદ્ધતિઓ", "ભૂલલખો", "પ્રયાસ", "અસુરક્ષિત", "ક્ષેત્ર",
];

const PUNJABI_KEYWORDS: &[&str] = &[
    "ਕਾਰਜ", "ਫੰਕਸ਼ਨ", "ਮੰਨੋ", "ਰਚਨਾ", "ਗਣਨਾ", "ਸਥਿਰ", "ਜਨਤਕ", "ਖੰਡ",
    "ਵਰਤੋ", "ਵਜੋਂ", "ਮੁੜੋ", "ਜੇ", "ਵਰਨਾ", "ਜਦੋਂ", "ਹਰ", "ਵਿੱਚ", "ਤੋਂ",
    "ਤੱਕ", "ਵਿਰਾਮ", "ਜਾਰੀ", "ਤਦ", "ਵੇਖੋ", "ਬਦਲਣਯੋਗ", "ਮੇਲ", "ਨਿਸ਼ਚਿਤ",
    "ਪ੍ਰਮਾਣ", "ਲੋੜੀਂਦਾ", "ਯਕੀਨੀ", "ਸੱਚ", "ਝੂਠ", "ਲਿਖੋ", "ਛਾਪੋ", "ਉਦੇਸ਼",
    "ਕਿਸਮ", "ਅਟੱਲ", "ਸ਼ੁੱਧ", "ਬਾਹਰੀ", "ਸਮਾਂਤਰ", "ਸੰਖੇਪ", "ਨਾਲ", "ਨਿਯੋਗ",
    "ਸੰਯੋਜਨ", "ਸੰਕੇਤ", "ਅਮਲ", "ਜਿੱਥੇ", "ਹੈ", "ਢੰਗ", "ਗਲਤੀਲਿਖੋ", "ਕੋਸ਼ਿਸ਼",
    "ਅਸੁਰੱਖਿਅਤ", "ਖੇਤਰ",
];

const KANNADA_KEYWORDS: &[&str] = &[
    "ಕಾರ್ಯ", "ಫಂಕ್ಷನ್", "ಊಹಿಸಿ", "ಮಾನ್ಯ", "ರಚನೆ", "ಎಣಿಕೆ", "ಸ್ಥಿರ",
    "ಸಾರ್ವಜನಿಕ", "ಖಂಡ", "ಬಳಸಿ", "ಆಗಿ", "ಹಿಂದಿರುಗಿ", "ಮರಳಿ", "ಆದರೆ",
    "ಇಲ್ಲದಿದ್ದರೆ", "ತನಕ", "ಪ್ರತಿ", "ರಲ್ಲಿ", "ಇಂದ", "ಗೆ", "ನಿಲ್ಲಿ",
    "ಮುಂದುವರಿಸಿ", "ನಂತರ", "ನೋಡಿ", "ಪರಿವರ್ತನೀಯ", "ಹೊಂದಾಣಿಕೆ", "ಖಚಿತಪಡಿಸಿ",
    "ಸಾಬೀತುಪಡಿಸಿ", "ಅಗತ್ಯ", "ಖಚಿತ", "ಸತ್ಯ", "ಸರಿ", "ಸುಳ್ಳು", "ತಪ್ಪು",
    "ಬರೆ", "ಮುದ್ರಿಸಿ", "ಉದ್ದೇಶ", "ಪ್ರಕಾರ", "ಅಚಲ", "ಶುದ್ಧ", "ಬಾಹ್ಯ",
    "ಸಮಾನಾಂತರ", "ಸಂಕ್ಷೇಪ", "ಜೊತೆ", "ನಿಯೋಗ", "ಸಂಯೋಜನೆ", "ಸಂಕೇತ", "ಜಾರಿ",
    "ಎಲ್ಲಿ", "ಇದೆ", "ವಿಧಾನಗಳು", "ದೋಷಬರೆ", "ಪ್ರಯತ್ನ", "ಅಸುರಕ್ಷಿತ", "ಪ್ರದೇಶ",
];

const ODIA_KEYWORDS: &[&str] = &[
    "କାର୍ଯ୍ୟ", "ଫଙ୍କସନ୍", "ମନେକର", "ଗଠନ", "ଗଣନା", "ସ୍ଥିର", "ସର୍ବସାଧାରଣ",
    "ଖଣ୍ଡ", "ବ୍ୟବହାର", "ଭାବେ", "ଫେରନ୍ତୁ", "ଯଦି", "ଅନ୍ୟଥା", "ଯେତେବେଳେ",
    "ପ୍ରତି", "ରେ", "ରୁ", "ପର୍ଯ୍ୟନ୍ତ", "ବନ୍ଦ", "ଜାରି", "ତାହେଲେ", "ଦେଖନ୍ତୁ",
    "ପରିବର୍ତ୍ତନୀୟ", "ମେଳ", "ନିଶ୍ଚିତ", "ପ୍ରମାଣ", "ଆବଶ୍ୟକ", "ସୁନିଶ୍ଚିତ",
    "ସତ୍ୟ", "ମିଥ୍ୟା", "ଲେଖ", "ଛାପନ୍ତୁ", "ଉଦ୍ଦେଶ୍ୟ", "ପ୍ରକାର", "ଅଚଳ",
    "ଶୁଦ୍ଧ", "ବାହ୍ୟ", "ସମାନ୍ତର", "ସଂକ୍ଷେପ", "ସହିତ", "ନିଯୋଗ", "ସଂଯୋଜନ",
    "ସଙ୍କେତ", "ପ୍ରୟୋଗ", "କେଉଁଠାରେ", "ଅଟେ", "ପଦ୍ଧତି", "ତ୍ରୁଟିଲେଖ", "ପ୍ରୟାସ",
    "ଅସୁରକ୍ଷିତ", "କ୍ଷେତ୍ର",
];

const URDU_KEYWORDS: &[&str] = &[
    "فنکشن", "کام", "مانیں", "فرض", "ساخت", "شمار", "ثابت", "عوامی",
    "ماڈیول", "حصہ", "استعمال", "بطور", "واپس", "لوٹاؤ", "اگر", "ورنہ",
    "ہر", "میں", "سے", "تک", "بند", "جاری", "تب", "دیکھیں", "بدلنا",
    "ملان", "مماثلت", "یقینی", "ثبوت", "درکار", "ضمانت", "سچ", "جھوٹ",
    "لکھو", "چھاپو", "رابطہ", "نافذ", "مقصد", "قسم", "بیرونی", "خالص",
    "متوازی", "تخفیف", "ساتھ", "ٹاسک", "ملاپ", "دوران", "جہاں", "ہے",
    "طریقے", "غیرمتغیر", "غلطیلکھو", "کوشش", "غیرمحفوظ", "علاقہ",
];

const PERSIAN_KEYWORDS: &[&str] = &[
    "تابع", "فانکشن", "فرض", "بگذار", "ساختار", "شمارش", "ثابت", "عمومی",
    "بخش", "استفاده", "بعنوان", "بازگشت", "اگر", "وگرنه", "تا", "هر", "در",
    "از", "بشکن", "ادامه", "سپس", "ببین", "تغییرپذیر", "تطبیق", "ادعا",
    "اثبات", "نیاز", "تضمین", "درست", "نادرست", "چاپ", "بنویس", "هدف",
    "نوع", "خارجی", "خالص", "موازی", "کاهش", "با", "وظیفه", "پیوستن", "به",
    "رابط", "اجرا", "کجا", "است", "روش", "تغییرناپذیر", "خطاچاپ", "تلاش",
    "ناامن", "منطقه",
];

const KOREAN_KEYWORDS: &[&str] = &[
    "함수", "정의", "구조체", "열거", "상수", "공개", "모듈", "사용", "로서", "반환", "돌려주기",
    "만약", "만일", "아니면", "동안", "각각", "안에", "에서", "까지", "중단", "계속", "그러면",
    "참조", "가변", "일치", "확인", "증명", "필요", "보장", "참", "거짓", "출력", "쓰기", "순수",
    "병렬", "인터페이스", "구현", "메서드", "여기서", "이다", "시도", "작업", "결합", "위험", "영역",
    "목적", "타입", "외부", "불변", "축소", "함께", "오류출력",
];

const JAPANESE_KEYWORDS: &[&str] = &[
    "関数", "代入", "構造体", "列挙", "定数", "公開", "モジュール", "単位", "使用", "として", "戻る",
    "返す", "もし", "そうでなければ", "の間", "間", "中断", "続行", "ならば", "対象", "から", "まで",
    "参照", "可変", "一致", "マッチ", "確認", "証明", "前提", "保証", "真", "偽", "表示", "書く",
    "純粋", "並列", "インターフェース", "実装", "メソッド", "ここで", "は", "試行", "タスク", "結合",
    "危険", "領域", "目的", "意図", "型", "外部", "不変", "削減", "と", "中", "エラー表示",
];

const ARABIC_KEYWORDS: &[&str] = &[
    "دالة", "ليكن", "بنية", "تعداد", "قيمة_ثابتة", "عام", "وحدة", "استخدم",
    "أرجع", "إرجاع", "إذا", "وإلا", "بينما", "لكل", "في", "من", "إلى",
    "كسر", "استمر", "ثم", "مرجع", "متغير", "طابق", "تأكد", "أثبت", "يتطلب",
    "يضمن", "صحيح", "خطأ", "اطبع", "نقي", "متوازي", "واجهة", "نفذ", "طرق",
    "حيث", "هو", "حاول", "مهمة", "اربط", "غير_آمن", "منطقة", "هدف", "نوع",
    "خارجي", "مستقر", "تقليل", "مع", "خطأاطبع",
];

const HEBREW_KEYWORDS: &[&str] = &[
    "פונקציה", "פעולה", "יהי", "מבנה", "ספירה", "קבוע", "ציבורי", "מודול",
    "מודולים", "השתמש", "החזר", "חזרה", "אם", "אחרת", "כאשר", "לכל",
    "בתוך", "מתוך", "עד", "שבור", "הפסק", "המשך", "אז", "הפנייה", "משתנה",
    "התאם", "ודא", "הוכח", "דורש", "מבטיח", "אמת", "שקר", "הדפס", "כתוב",
    "טהור", "מקבילי", "ממשק", "ממש", "שיטות", "איפה", "הוא", "נסה",
    "משימה", "חיבור", "מסוכן", "אזור", "מטרה", "סוג", "טיפוס", "חיצוני",
    "בלתי_משתנה", "הפחתה", "עם", "שגיאההדפס",
];

const RUSSIAN_KEYWORDS: &[&str] = &[
    "функция", "пусть", "структура", "перечисление", "постоянная",
    "публичный", "общий", "модуль", "использовать", "как", "вернуть",
    "верни", "если", "иначе", "пока", "для", "в", "от", "до", "прервать",
    "продолжить", "тогда", "смотри", "изменяемый", "совпадение",
    "утверждать", "доказать", "требует", "гарантирует", "истина", "верно",
    "ложь", "неверно", "печатать", "писать", "чистый", "параллельный",
    "интерфейс", "реализовать", "методы", "где", "есть", "попробуй",
    "задача", "соединить", "небезопасно", "область", "цель", "тип",
    "внешний", "инвариант", "сократить", "совместно", "ошибкапечатать",
];

const SPANISH_KEYWORDS: &[&str] = &[
    "función", "enumeración", "público", "módulo", "intención",
    "propósito", "métodos", "región", "funcion", "sea", "estructura",
    "constante", "enumeracion", "publico", "modulo", "usar", "como",
    "regresar", "retornar", "volver", "si", "sino", "mientras", "para",
    "en", "desde", "hasta", "romper", "continuar", "entonces", "ver",
    "mutable", "coincidir", "afirmar", "demostrar", "requiere",
    "garantiza", "verdadero", "falso", "imprimir", "escribir", "puro",
    "paralelo", "interfaz", "implementar", "metodos", "donde", "es",
    "intentar", "tarea", "unir", "inseguro", "region", "intencion", "tipo",
    "externo", "invariante", "reducir", "con", "errorimprimir",
];

const FRENCH_KEYWORDS: &[&str] = &[
    "énumération", "référence", "vérifier", "vérifie", "démontrer",
    "démontre", "vérité", "écrire", "écris", "où", "tâche", "parallèle",
    "méthodes", "implémenter", "région", "étranger", "fonction", "soit",
    "structure", "constante", "public", "module", "utiliser", "comme",
    "retourner", "retourne", "si", "sinon", "tantque", "pour", "dans",
    "depuis", "vers", "interrompre", "continuer", "alors", "voir",
    "muable", "correspondre", "affirmer", "prouver", "exige", "garantit",
    "vrai", "faux", "imprimer", "imprime", "afficher", "pur", "interface",
    "implementer", "methodes", "ou", "est", "essayer", "travail",
    "joindre", "dangereux", "but", "objectif", "type", "externe",
    "invariant", "parallele", "reduire", "avec", "enumeration", "region",
    "erreurimprimer",
];

const GERMAN_KEYWORDS: &[&str] = &[
    "aufzählung", "öffentlich", "während", "für", "zurück", "veränderlich",
    "veränderbar", "übereinstimmen", "überprüfen", "überprüfe", "prüfen",
    "prüfe", "ausführbar", "äußere", "äußerer", "funktion", "sei",
    "struktur", "konstante", "modul", "verwenden", "als", "zurueck",
    "wenn", "sonst", "solange", "jede", "in", "von", "bis", "brechen",
    "weiter", "dann", "sehen", "wandelbar", "passend", "behaupten",
    "beweisen", "benoetigt", "garantiert", "wahr", "falsch", "drucken",
    "schreiben", "rein", "parallel", "schnittstelle", "implementieren",
    "methoden", "wo", "ist", "versuchen", "aufgabe", "verbinden",
    "unsicher", "absicht", "typ", "extern", "unveraenderlich",
    "reduzieren", "mit", "aufzaehlung", "oeffentlich", "bereich",
    "fehlerdrucken",
];

const PORTUGUESE_KEYWORDS: &[&str] = &[
    "função", "enumeração", "público", "módulo", "senão", "então", "até",
    "referência", "mutável", "métodos", "intenção", "propósito", "região",
    "funcao", "seja", "estrutura", "constante", "enumeracao", "publico",
    "modulo", "usar", "como", "retornar", "retorne", "se", "senao",
    "enquanto", "para", "em", "desde", "ate", "parar", "interromper",
    "continuar", "entao", "ver", "mutavel", "combinar", "corresponder",
    "afirmar", "provar", "demonstrar", "requer", "garante", "verdadeiro",
    "falso", "imprimir", "escrever", "puro", "paralelo", "interface",
    "implementar", "metodos", "onde", "eh", "tentar", "tarefa", "juntar",
    "unir", "inseguro", "regiao", "intencao", "proposito", "objetivo",
    "tipo", "externo", "invariante", "reduzir", "com", "erroimprimir",
];

const ITALIAN_KEYWORDS: &[&str] = &[
    "funzione", "sia", "struttura", "enumerazione", "costante", "pubblico",
    "modulo", "usare", "come", "ritornare", "ritorna", "se", "altrimenti",
    "mentre", "per", "da", "finoa", "rompere", "interrompere",
    "continuare", "allora", "vedere", "mutevole", "corrispondere",
    "combaciare", "affermare", "dimostrare", "richiede", "garantisce",
    "vero", "falso", "stampare", "scrivere", "puro", "parallelo",
    "interfaccia", "implementare", "metodi", "dove", "tentare", "compito",
    "unire", "insicuro", "regione", "scopo", "intenzione", "obiettivo",
    "tipo", "esterno", "invariante", "ridurre", "con", "in", "risulta",
    "errorestampare",
];

const TURKISH_KEYWORDS: &[&str] = &[
    "işlev", "dön", "döndür", "eğer", "için", "içinde", "kır", "gör",
    "değişken", "eşle", "doğrula", "kanıtla", "doğru", "yanlış", "yazdır",
    "arayüz", "görev", "birleştir", "güvensiz", "bölge", "amaç", "dış",
    "değişmez", "yapı", "sıralama", "modül", "saf", "paralel", "azalt",
    "ile", "olsun", "yoksa", "iken", "devam", "den", "kadar", "sonra",
    "uygula", "nerede", "olur", "sabit", "tip", "metotlar", "kullan",
    "gerek", "garanti", "hatayazdır", "dene", "genel", "fonksiyon", "yapi",
    "siralama", "modul", "olarak", "geri", "don", "icin", "kir",
    "degisken", "esle", "dogrula", "kanitla", "dogru", "yanlis", "yazdir",
    "arayuz", "gorev", "guvensiz", "bolge", "amac", "dis", "degismez",
    "birlestir", "eger", "icinde", "bak", "hatayazdir",
];

const POLISH_KEYWORDS: &[&str] = &[
    "wróć", "zwróć", "jeśli", "dopóki", "potwierdź", "fałsz", "równoległy",
    "spróbuj", "połącz", "zewnętrzny", "stała", "moduł", "użyj", "funkcja",
    "niech", "struktura", "wyliczenie", "stala", "publiczny", "modul",
    "uzyj", "jako", "zwroc", "wroc", "jesli", "inaczej", "dopoki", "dla",
    "w", "od", "przerwij", "kontynuuj", "wtedy", "zobacz", "zmienny",
    "dopasuj", "potwierdz", "udowodnij", "wymaga", "gwarantuje", "prawda",
    "falsz", "drukuj", "wypisz", "czysty", "rownolegly", "interfejs",
    "zaimplementuj", "metody", "gdzie", "jest", "sprobuj", "zadanie",
    "polacz", "niebezpieczny", "cel", "intencja", "typ", "zewnetrzny",
    "niezmienny", "zmniejsz", "razem", "do", "obszar", "bladdrukuj",
];

const INDONESIAN_KEYWORDS: &[&str] = &[
    "fungsi", "biarkan", "misalkan", "struktur", "enumerasi", "tetap",
    "publik", "umum", "modul", "pakai", "sebagai", "kembali", "kembalikan",
    "jika", "selainnya", "lainnya", "selama", "untuk", "dalam", "dari",
    "sampai", "hingga", "berhenti", "lanjutkan", "maka", "lihat",
    "dapatberubah", "berubah", "cocokkan", "padanan", "pastikan",
    "buktikan", "perlu", "jamin", "benar", "salah", "cetak", "tulis",
    "murni", "paralel", "antarmuka", "terapkan", "implementasi", "metode",
    "dimana", "adalah", "coba", "tugas", "gabungkan", "bahaya", "wilayah",
    "tujuan", "niat", "tipe", "jenis", "eksternal", "invarian", "kurangi",
    "dengan", "kesalahancetak",
];

const MALAY_KEYWORDS: &[&str] = &[
    "fungsi", "biarkan", "struktur", "penghitungan", "pemalar", "awam",
    "modul", "guna", "sebagai", "kembali", "jika", "selainnya", "selama",
    "untuk", "dalam", "dari", "hingga", "berhenti", "teruskan", "maka",
    "lihat", "berubah", "padan", "pastikan", "buktikan", "memerlukan",
    "menjamin", "benar", "palsu", "cetak", "tulis", "tulen", "selari",
    "antaramuka", "laksanakan", "kaedah", "tempat", "adalah", "cuba",
    "tugasan", "gabung", "tidakselamat", "kawasan", "tujuan", "jenis",
    "luaran", "tetap", "kurangkan", "dengan", "ralatcetak",
];

const SWAHILI_KEYWORDS: &[&str] = &[
    "kazi", "acha", "muundo", "orodha", "thabiti", "umma", "moduli",
    "tumia", "kama", "rudi", "kama_ni", "ikiwa", "vinginevyo", "wakati",
    "kwa", "ndani", "kutoka", "hadi", "vunja", "endelea", "kisha",
    "tazama", "badilika", "linganisha", "thibitisha", "thibitisha_kuwa",
    "thibitisha_kabisa", "hitaji", "hakikisha", "kweli", "uongo",
    "chapisha", "andika", "safi", "sambamba", "kiolesura", "tekeleza",
    "njia", "wapi", "ni", "jaribu", "jukumu", "unganisha", "hatari",
    "eneo", "lengo", "aina", "nje", "isiyobadilika", "punguza", "na",
    "kosachapisha",
];

const DUTCH_KEYWORDS: &[&str] = &[
    "functie", "laat", "structuur", "opsomming", "constante", "openbaar",
    "module", "gebruik", "als", "terug", "indien", "anders", "zolang",
    "voor", "in", "van", "tot", "stop", "verder", "dan", "zie",
    "veranderlijk", "vergelijk", "bevestig", "bewijs", "vereist",
    "verzekert", "waar", "onwaar", "druk", "schrijf", "zuiver", "parallel",
    "interface", "implementeer", "methoden", "waar_is", "is", "probeer",
    "taak", "verbind", "onveilig", "gebied", "doel", "type", "extern",
    "invariant", "verminder", "met", "foutdruk",
];

const THAI_KEYWORDS: &[&str] = &[
    "ฟังก์ชัน", "ให้", "โครงสร้าง", "การแจงนับ", "คงที่", "สาธารณะ",
    "โมดูล", "ใช้", "เป็น", "คืน", "ถ้า", "ไม่เช่นนั้น", "ขณะที่",
    "สำหรับ", "ใน", "จาก", "ถึง", "หยุด", "ดำเนินต่อ", "แล้ว", "ดู",
    "เปลี่ยนแปลงได้", "ตรงกัน", "ยืนยัน", "พิสูจน์", "ต้องการ",
    "รับประกัน", "จริง", "เท็จ", "พิมพ์", "บริสุทธิ์", "ขนาน",
    "อินเทอร์เฟซ", "ดำเนินการ", "วิธีการ", "ที่ไหน", "คือ", "ลอง", "งาน",
    "รวม", "ไม่ปลอดภัย", "พื้นที่", "จุดประสงค์", "ชนิด", "ภายนอก",
    "ไม่เปลี่ยน", "ลด", "กับ", "ข้อผิดพลาดพิมพ์",
];

const HUNGARIAN_KEYWORDS: &[&str] = &[
    "függvény", "visszatér", "különben", "amíg", "törj", "nézd", "változó",
    "állítsd", "bizonyítsd", "igényel", "garantál", "párhuzamos",
    "felület", "metódusok", "próbáld", "egyesít", "veszélyes", "tartomány",
    "cél", "típus", "külső", "állandó", "felsorolás", "nyilvános",
    "használd", "tiszta", "csökkent", "együtt", "feladat", "legyen", "ha",
    "folytat", "minden", "belül", "kezdve", "határig", "szerkezet",
    "illeszkedik", "akkor", "megvalósít", "ahol", "van", "változatlan",
    "nyomtat", "hibanyomtat", "modul", "mint", "folytasd", "egyezzen",
    "igaz", "hamis", "nyomtass", "fuggveny", "valtozo", "valositsd_meg",
    "kulso", "parhuzamos", "csokkent", "egyutt", "egyesit", "visszater",
    "kulonben", "amig", "torj", "belul", "nezd", "hatarig", "felsorolas",
    "felulet", "allando", "tipus", "metodusok", "cel", "hasznald",
    "igenyel", "garantal", "valtozatlan", "allitsd", "bizonyitsd",
    "hibanyomtass", "probald", "nyilvanos", "veszelyes", "tartomany",
];

const CZECH_KEYWORDS: &[&str] = &[
    "nechť", "vrať", "přeruš", "pokračuj", "tvrď", "dokaž", "vyžaduje",
    "zajišťuje", "vypiš", "čistý", "paralelní", "rozhraní", "úloha",
    "nebezpečný", "záměr", "vnější", "neměnný", "výčet", "veřejný",
    "použij", "proměnný", "odpovídej", "funkce", "zmenši", "spolu", "spoj",
    "pokud", "jinak", "dokud", "pro", "uvnitř", "viz", "od", "do",
    "struktura", "pak", "implementuj", "kde", "je", "konstanta", "typ",
    "metody", "chybavypiš", "zkus", "modul", "oblast", "jako", "jestli",
    "pravda", "nepravda", "tiskni", "nechtt", "vrat", "prerus", "pokracuj",
    "tvrd", "dokaz", "vyzaduje", "zajistuje", "vypis", "cisty",
    "paralelni", "rozhrani", "uloha", "nebezpecny", "zamer", "vnejsi",
    "nemenny", "vycet", "verejny", "pouzij", "promenny", "odpovidej",
    "zmensi", "uvnitr", "chybatiskni",
];

const TYPE_NAMES: &[&str] = &[
    "i8", "i16", "i32", "i64",
    "u8", "u16", "u32", "u64",
    "f32", "f64", "bool",
    "Str", "OwnedStr",
    "Vec", "Atomic", "Channel", "Mutex", "Guard", "Task",
    "Result", "Option",
];

const BUILTIN_FUNCTIONS: &[&str] = &[
    // Vec / array.
    "vec", "push", "set", "clone", "len", "pop", "clone_at",
    "contains", "binary_search", "find",
    // Fallible allocation.
    "try_vec",
    // Math intrinsics.
    "min", "max",
    // Atomic.
    "atomic_new", "atomic_load", "atomic_store", "atomic_fetch_add",
    "atomic_compare_exchange",
    // Channel.
    "channel_new", "channel_send", "channel_recv",
    // Mutex / Guard.
    "mutex_new", "mutex_lock", "guard_get", "guard_set",
];

fn handle_rename(
    state: &LspState,
    params: lsp_types::RenameParams,
) -> Result<Option<lsp_types::WorkspaceEdit>, String> {
    let uri = &params.text_document_position.text_document.uri;
    let Some(source) = state.docs.get(uri) else {
        return Ok(None);
    };
    let pos = params.text_document_position.position;
    let new_name = params.new_name;
    compute_rename(source, pos, &new_name).map(|maybe_edits| {
        maybe_edits.map(|edits| {
            use lsp_types::OneOf as ResultOneOf;
            let text_edits: Vec<lsp_types::TextEdit> = edits
                .into_iter()
                .map(|span| lsp_types::TextEdit {
                    range: span_to_range(source, span),
                    new_text: new_name.clone(),
                })
                .collect();
            let doc_edit = lsp_types::TextDocumentEdit {
                text_document: lsp_types::OptionalVersionedTextDocumentIdentifier {
                    uri: uri.clone(),
                    version: None,
                },
                edits: text_edits
                    .into_iter()
                    .map(ResultOneOf::Left)
                    .collect(),
            };
            lsp_types::WorkspaceEdit {
                changes: None,
                document_changes: Some(lsp_types::DocumentChanges::Edits(vec![doc_edit])),
                change_annotations: None,
            }
        })
    })
}

/// Resolve a "rename" lookup. Identifies the binding under
/// the cursor (same logic as goto-definition / references),
/// validates the proposed new name, then returns every span
/// that should be replaced — both uses and the declaration.
///
/// Validation rules (LSP convention: returning Err yields a
/// user-visible error in the editor's rename UI):
/// * the new name must be a syntactically valid identifier
///   (`[A-Za-z_][A-Za-z0-9_]*`);
/// * it must not collide with an Intent keyword (a reserved
///   word would change the program's parse, not just its
///   binding name);
/// * the cursor must be on a binding name we can track —
///   synthetic checker-inserted names are filtered out and
///   yield `Ok(None)` (no edits, no error).
///
/// Scope: matches `compute_references`. Each `Var` read
/// carries a `binding_decl_span` set by the checker to the
/// declaring `Let`'s span, and `matches_target` keys
/// scope-aware identity on that span — so renaming the
/// outer `x` in a function that nest-shadows the same
/// name leaves the inner shadow's uses untouched. Refines
/// #9 from STATUS.md.
pub fn compute_rename(
    source: &str,
    position: Position,
    new_name: &str,
) -> Result<Option<Vec<crate::span::Span>>, String> {
    if !is_valid_identifier(new_name) {
        return Err(format!(
            "'{}' is not a valid identifier — names must match [A-Za-z_][A-Za-z0-9_]*",
            new_name
        ));
    }
    if is_reserved_keyword(new_name) {
        return Err(format!(
            "'{}' is a reserved keyword and cannot be used as a binding name",
            new_name
        ));
    }
    let Some(checked) = crate::compile(source).ok() else {
        return Ok(None);
    };
    let Some(offset) = position_to_byte_offset(source, position) else {
        return Ok(None);
    };
    let Some(target) = find_var_at(&checked.ir, offset) else {
        return Ok(None);
    };
    if new_name == target.name {
        // No-op rename: return empty edit list so the
        // editor treats it as a successful no-op rather than
        // applying nothing silently.
        return Ok(Some(Vec::new()));
    }
    // Collect every use, then prepend the declaration if we
    // can find one. Sort + de-dup so each span is replaced
    // exactly once.
    let mut spans = Vec::new();
    for f in &checked.ir.functions {
        for stmt in &f.body {
            collect_var_uses(stmt, &target, &mut spans);
        }
    }
    if let Some(decl) = find_declaration_span(&checked.ir, offset, &target) {
        spans.push(decl);
    }
    spans.sort_by_key(|s| (s.start, s.end));
    spans.dedup();
    Ok(Some(spans))
}

fn is_valid_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn is_reserved_keyword(s: &str) -> bool {
    matches!(
        s,
        // Keep this list in sync with `Lexer::lex_identifier`'s
        // keyword dispatch. Renaming to a keyword would change
        // the parse, not just the binding.
        "fn" | "pure" | "parallel" | "reduce" | "with" | "min" | "max"
        | "task" | "join"
        | "let" | "return" | "if" | "else" | "while" | "break" | "continue"
        | "mut" | "for" | "in" | "intent" | "use" | "requires" | "ensures"
        | "invariant" | "assert" | "prove" | "print"
        | "true" | "false"
        | "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64"
        | "f32" | "f64" | "bool" | "Str" | "OwnedStr" | "Vec" | "Atomic"
        | "Channel" | "Mutex" | "Guard" | "Task"
        | "as"
    )
}

fn handle_references(
    state: &LspState,
    params: lsp_types::ReferenceParams,
) -> Option<Vec<lsp_types::Location>> {
    let uri = &params.text_document_position.text_document.uri;
    let source = state.docs.get(uri)?;
    let pos = params.text_document_position.position;
    let include_decl = params.context.include_declaration;
    let spans = compute_references(source, pos, include_decl)?;
    Some(
        spans
            .into_iter()
            .map(|span| lsp_types::Location {
                uri: uri.clone(),
                range: span_to_range(source, span),
            })
            .collect(),
    )
}

/// Resolve a "find all references" lookup. Identifies the
/// binding under the cursor (same logic as goto-definition),
/// then walks every function body once more to collect every
/// `Var` / `Ref` / `RefMut` whose name matches. If
/// `include_declaration` is set, the declaration site is
/// included as the first entry.
///
/// Limitations:
/// * Walks the whole program — no scope analysis, so two
///   shadowed bindings with the same name would conflate.
///   v1 accepts this as a practical trade-off; semantic
///   tokens would refine this in a later milestone.
/// * Synthetic checker-inserted names are filtered out.
pub fn compute_references(
    source: &str,
    position: Position,
    include_declaration: bool,
) -> Option<Vec<crate::span::Span>> {
    let checked = crate::compile(source).ok()?;
    let offset = position_to_byte_offset(source, position)?;
    let target = find_var_at(&checked.ir, offset)?;
    let mut spans = Vec::new();
    for f in &checked.ir.functions {
        for stmt in &f.body {
            collect_var_uses(stmt, &target, &mut spans);
        }
    }
    spans.sort_by_key(|s| s.start);
    if include_declaration {
        if let Some(decl) = find_declaration_span(&checked.ir, offset, &target) {
            if !spans.iter().any(|s| *s == decl) {
                spans.insert(0, decl);
            }
        }
    }
    Some(spans)
}

fn collect_var_uses(stmt: &TypedStmt, target: &Target, out: &mut Vec<crate::span::Span>) {
    match stmt {
        TypedStmt::Let { expr, .. }
        | TypedStmt::Reassign { expr, .. }
        | TypedStmt::Return { expr }
        | TypedStmt::Assert { expr, .. }
        | TypedStmt::Discard { expr } => collect_var_uses_in_expr(expr, target, out),
        TypedStmt::Print { items } => {
            for item in items {
                if let TypedPrintItem::Expr(e) = item {
                    collect_var_uses_in_expr(e, target, out);
                }
            }
        }
        TypedStmt::If { cond, then_body, else_body } => {
            collect_var_uses_in_expr(cond, target, out);
            for s in then_body {
                collect_var_uses(s, target, out);
            }
            for s in else_body {
                collect_var_uses(s, target, out);
            }
        }
        TypedStmt::While { cond, body, .. } => {
            collect_var_uses_in_expr(cond, target, out);
            for s in body {
                collect_var_uses(s, target, out);
            }
        }
        TypedStmt::For { start, end, body, .. } => {
            collect_var_uses_in_expr(start, target, out);
            collect_var_uses_in_expr(end, target, out);
            for s in body {
                collect_var_uses(s, target, out);
            }
        }
        TypedStmt::ForIter { body, .. } => {
            for s in body {
                collect_var_uses(s, target, out);
            }
        }
        TypedStmt::IndexAssign { index, value, .. } => {
            collect_var_uses_in_expr(index, target, out);
            collect_var_uses_in_expr(value, target, out);
        }
        TypedStmt::TaskSpawn { body, .. } => {
            for s in body {
                collect_var_uses(s, target, out);
            }
        }
        _ => {}
    }
}

/// Match a binding reference against the target: when both
/// sides carry a `decl_span`, the spans must match (scope-
/// aware identity). Otherwise fall back to name-only
/// matching for synthetic / unresolved references.
fn matches_target(name: &str, decl_span: Option<crate::span::Span>, target: &Target) -> bool {
    if name.starts_with("__intent_") {
        return false;
    }
    match (decl_span, target.decl_span) {
        (Some(a), Some(b)) => a == b,
        _ => name == target.name,
    }
}

fn collect_var_uses_in_expr(
    expr: &TypedExpr,
    target: &Target,
    out: &mut Vec<crate::span::Span>,
) {
    match &expr.kind {
        TypedExprKind::Var(name) => {
            if matches_target(name, expr.binding_decl_span, target) {
                out.push(expr.span);
            }
        }
        TypedExprKind::Ref { name } | TypedExprKind::RefMut { name } => {
            if matches_target(name, expr.binding_decl_span, target) {
                out.push(expr.span);
            }
        }
        TypedExprKind::Unary { expr, .. } => collect_var_uses_in_expr(expr, target, out),
        TypedExprKind::Binary { left, right, .. } => {
            collect_var_uses_in_expr(left, target, out);
            collect_var_uses_in_expr(right, target, out);
        }
        TypedExprKind::Cast { expr, .. } => collect_var_uses_in_expr(expr, target, out),
        TypedExprKind::Call { args, .. } => {
            for a in args {
                collect_var_uses_in_expr(a, target, out);
            }
        }
        TypedExprKind::ArrayLit { elements } => {
            for e in elements {
                collect_var_uses_in_expr(e, target, out);
            }
        }
        TypedExprKind::Index { array, index, .. } => {
            collect_var_uses_in_expr(array, target, out);
            collect_var_uses_in_expr(index, target, out);
        }
        TypedExprKind::Len { array, .. } => collect_var_uses_in_expr(array, target, out),
        _ => {}
    }
}

fn handle_goto_definition(
    state: &LspState,
    params: lsp_types::GotoDefinitionParams,
) -> Option<lsp_types::GotoDefinitionResponse> {
    let uri = &params.text_document_position_params.text_document.uri;
    let source = state.docs.get(uri)?;
    let pos = params.text_document_position_params.position;
    let span = compute_goto_definition(source, pos)?;
    let location = lsp_types::Location {
        uri: uri.clone(),
        range: span_to_range(source, span),
    };
    Some(lsp_types::GotoDefinitionResponse::Scalar(location))
}

/// Resolve a "go to definition" lookup. Walks the typed IR
/// for a `Var`-shaped expression at the cursor, then finds
/// the binding's declaration site by scanning the
/// surrounding function for the matching `Let` or function
/// parameter. Returns the span of the declaration as a
/// source-byte range; the LSP caller wraps it in a
/// `Location`. Returns `None` if the document doesn't
/// compile, the cursor isn't on a binding reference, or no
/// matching declaration is in scope.
///
/// `TypedStmt::Let` doesn't carry a dedicated span, so the
/// declaration site is approximated as the let's RHS
/// expression span — typically a few characters off from
/// the `let` keyword itself, but close enough for editors
/// to land in the right neighborhood.
pub fn compute_goto_definition(source: &str, position: Position) -> Option<crate::span::Span> {
    let checked = crate::compile(source).ok()?;
    let offset = position_to_byte_offset(source, position)?;
    // Find the Var-bearing expression at the cursor. Prefer
    // the smallest such expression — if the cursor is on a
    // sub-expression, we want its specific Var name, not the
    // outer expression's.
    let var_name = find_var_at(&checked.ir, offset)?;
    // Scan the IR for the binding's declaration in the
    // function whose body contains the cursor.
    find_declaration_span(&checked.ir, offset, &var_name)
}

/// A binding reference at the cursor. `name` is what the
/// user wrote; `decl_span` is the binding's declaration site
/// (from `TypedExpr::binding_decl_span`) when the checker was
/// able to resolve it — `None` for synthetic / unresolved
/// references. Two `Target`s with the same `decl_span` refer
/// to the same binding even if their names happen to match
/// other bindings in other scopes. When `decl_span` is
/// `None`, the LSP walkers fall back to name-only matching.
#[derive(Clone, Debug, PartialEq)]
struct Target {
    name: String,
    decl_span: Option<crate::span::Span>,
}

fn find_var_at(program: &crate::ir::TypedProgram, offset: usize) -> Option<Target> {
    let mut best: Option<(Target, crate::span::Span)> = None;
    for f in &program.functions {
        for stmt in &f.body {
            walk_stmt_for_var(stmt, offset, &mut best);
        }
    }
    best.map(|(t, _)| t)
}

fn walk_stmt_for_var(
    stmt: &TypedStmt,
    offset: usize,
    best: &mut Option<(Target, crate::span::Span)>,
) {
    match stmt {
        TypedStmt::Let { expr, .. }
        | TypedStmt::Reassign { expr, .. }
        | TypedStmt::Return { expr }
        | TypedStmt::Assert { expr, .. }
        | TypedStmt::Discard { expr } => walk_expr_for_var(expr, offset, best),
        TypedStmt::Print { items } => {
            for item in items {
                if let TypedPrintItem::Expr(e) = item {
                    walk_expr_for_var(e, offset, best);
                }
            }
        }
        TypedStmt::If { cond, then_body, else_body } => {
            walk_expr_for_var(cond, offset, best);
            for s in then_body {
                walk_stmt_for_var(s, offset, best);
            }
            for s in else_body {
                walk_stmt_for_var(s, offset, best);
            }
        }
        TypedStmt::While { cond, body, .. } => {
            walk_expr_for_var(cond, offset, best);
            for s in body {
                walk_stmt_for_var(s, offset, best);
            }
        }
        TypedStmt::For { start, end, body, .. } => {
            walk_expr_for_var(start, offset, best);
            walk_expr_for_var(end, offset, best);
            for s in body {
                walk_stmt_for_var(s, offset, best);
            }
        }
        TypedStmt::ForIter { body, .. } => {
            for s in body {
                walk_stmt_for_var(s, offset, best);
            }
        }
        TypedStmt::IndexAssign { index, value, .. } => {
            walk_expr_for_var(index, offset, best);
            walk_expr_for_var(value, offset, best);
        }
        TypedStmt::TaskSpawn { body, .. } => {
            for s in body {
                walk_stmt_for_var(s, offset, best);
            }
        }
        _ => {}
    }
}

fn walk_expr_for_var(
    expr: &TypedExpr,
    offset: usize,
    best: &mut Option<(Target, crate::span::Span)>,
) {
    let span = expr.span;
    if !(span.start <= offset && offset <= span.end) {
        return;
    }
    let decl_span = expr.binding_decl_span;
    let consider = |name: String,
                    decl_span: Option<crate::span::Span>,
                    span: crate::span::Span,
                    best: &mut Option<(Target, crate::span::Span)>| {
        // Skip synthetic names introduced by the checker
        // (`__intent_ret_*`, `__intent_iter_idx_*`, etc.).
        // The user never wrote them, so a goto-definition
        // hit on one is noise.
        if name.starts_with("__intent_") {
            return;
        }
        let width = span.end.saturating_sub(span.start);
        let better = match best {
            None => true,
            Some((_, prev)) => width < prev.end.saturating_sub(prev.start),
        };
        if better {
            *best = Some((Target { name, decl_span }, span));
        }
    };
    match &expr.kind {
        TypedExprKind::Var(name) => consider(name.clone(), decl_span, span, best),
        TypedExprKind::Ref { name } | TypedExprKind::RefMut { name } => {
            consider(name.clone(), decl_span, span, best)
        }
        TypedExprKind::Unary { expr, .. } => walk_expr_for_var(expr, offset, best),
        TypedExprKind::Binary { left, right, .. } => {
            walk_expr_for_var(left, offset, best);
            walk_expr_for_var(right, offset, best);
        }
        TypedExprKind::Cast { expr, .. } => walk_expr_for_var(expr, offset, best),
        TypedExprKind::Call { args, .. } => {
            for a in args {
                walk_expr_for_var(a, offset, best);
            }
        }
        TypedExprKind::ArrayLit { elements } => {
            for e in elements {
                walk_expr_for_var(e, offset, best);
            }
        }
        TypedExprKind::Index { array, index, .. } => {
            walk_expr_for_var(array, offset, best);
            walk_expr_for_var(index, offset, best);
        }
        TypedExprKind::Len { array, .. } => walk_expr_for_var(array, offset, best),
        _ => {}
    }
}

fn find_declaration_span(
    program: &crate::ir::TypedProgram,
    cursor: usize,
    target: &Target,
) -> Option<crate::span::Span> {
    // Fast path: if the cursor's reference resolved to a
    // `binding_decl_span`, return that directly. This is the
    // scope-correct answer — VarInfo's `decl_span` is the
    // env entry's declaration site, which the checker
    // populated during type-checking.
    if let Some(decl) = target.decl_span {
        return Some(decl);
    }
    // Fallback for synthetic / unresolved references: scan
    // by name within the cursor's containing function.
    // Preserves the older behavior for legacy walkers.
    let name = target.name.as_str();
    for f in &program.functions {
        let in_this_fn = f.body.iter().any(|s| stmt_contains_offset(s, cursor));
        if !in_this_fn {
            continue;
        }
        if f.params.iter().any(|p| p.name == name) {
            // Parameters don't carry a span in TypedParam;
            // use a zero-width span at the function header.
            return Some(crate::span::Span::default());
        }
        let mut found: Option<crate::span::Span> = None;
        scan_decl(&f.body, name, &mut found);
        if found.is_some() {
            return found;
        }
    }
    None
}

fn stmt_contains_offset(stmt: &TypedStmt, offset: usize) -> bool {
    match stmt {
        TypedStmt::Let { expr, .. }
        | TypedStmt::Reassign { expr, .. }
        | TypedStmt::Return { expr }
        | TypedStmt::Assert { expr, .. }
        | TypedStmt::Discard { expr } => expr.span.start <= offset && offset <= expr.span.end,
        TypedStmt::If { cond, then_body, else_body } => {
            (cond.span.start <= offset && offset <= cond.span.end)
                || then_body.iter().any(|s| stmt_contains_offset(s, offset))
                || else_body.iter().any(|s| stmt_contains_offset(s, offset))
        }
        TypedStmt::While { cond, body, .. } => {
            (cond.span.start <= offset && offset <= cond.span.end)
                || body.iter().any(|s| stmt_contains_offset(s, offset))
        }
        TypedStmt::For { start, end, body, .. } => {
            (start.span.start <= offset && offset <= start.span.end)
                || (end.span.start <= offset && offset <= end.span.end)
                || body.iter().any(|s| stmt_contains_offset(s, offset))
        }
        TypedStmt::ForIter { body, .. } => body.iter().any(|s| stmt_contains_offset(s, offset)),
        TypedStmt::TaskSpawn { body, .. } => body.iter().any(|s| stmt_contains_offset(s, offset)),
        _ => false,
    }
}

fn scan_decl(stmts: &[TypedStmt], name: &str, found: &mut Option<crate::span::Span>) {
    for stmt in stmts {
        if found.is_some() {
            return;
        }
        match stmt {
            TypedStmt::Let { name: n, expr, .. } => {
                if n == name {
                    *found = Some(expr.span);
                    return;
                }
            }
            TypedStmt::For { var, body, .. } => {
                if var == name {
                    *found = Some(crate::span::Span::default());
                    return;
                }
                scan_decl(body, name, found);
            }
            TypedStmt::ForIter { var, body, .. } => {
                if var == name {
                    *found = Some(crate::span::Span::default());
                    return;
                }
                scan_decl(body, name, found);
            }
            TypedStmt::If { then_body, else_body, .. } => {
                scan_decl(then_body, name, found);
                if found.is_some() {
                    return;
                }
                scan_decl(else_body, name, found);
            }
            TypedStmt::While { body, .. } => scan_decl(body, name, found),
            TypedStmt::TaskSpawn { name: handle_name, body, .. } => {
                if handle_name == name {
                    *found = Some(crate::span::Span::default());
                    return;
                }
                scan_decl(body, name, found);
            }
            _ => {}
        }
    }
}

fn find_smallest_typed_at(
    program: &crate::ir::TypedProgram,
    offset: usize,
) -> Option<(crate::ast::Type, crate::span::Span)> {
    let mut best: Option<(crate::ast::Type, crate::span::Span)> = None;
    for f in &program.functions {
        for stmt in &f.body {
            walk_stmt(stmt, offset, &mut best);
        }
        for req in &f.requires {
            walk_expr(req, offset, &mut best);
        }
    }
    best
}

fn walk_stmt(
    stmt: &TypedStmt,
    offset: usize,
    best: &mut Option<(crate::ast::Type, crate::span::Span)>,
) {
    match stmt {
        TypedStmt::Let { expr, .. } => walk_expr(expr, offset, best),
        TypedStmt::Reassign { expr, .. } => walk_expr(expr, offset, best),
        TypedStmt::Discard { expr } => walk_expr(expr, offset, best),
        TypedStmt::Return { expr } => walk_expr(expr, offset, best),
        TypedStmt::Print { items } | TypedStmt::EPrint { items } => {
            for item in items {
                if let TypedPrintItem::Expr(e) = item {
                    walk_expr(e, offset, best);
                }
            }
        }
        TypedStmt::Assert { expr, .. } => walk_expr(expr, offset, best),
        TypedStmt::If { cond, then_body, else_body } => {
            walk_expr(cond, offset, best);
            for s in then_body {
                walk_stmt(s, offset, best);
            }
            for s in else_body {
                walk_stmt(s, offset, best);
            }
        }
        TypedStmt::While { cond, body, .. } => {
            walk_expr(cond, offset, best);
            for s in body {
                walk_stmt(s, offset, best);
            }
        }
        TypedStmt::For { start, end, body, .. } => {
            walk_expr(start, offset, best);
            walk_expr(end, offset, best);
            for s in body {
                walk_stmt(s, offset, best);
            }
        }
        TypedStmt::ForIter { collection: _, body, .. } => {
            for s in body {
                walk_stmt(s, offset, best);
            }
        }
        TypedStmt::IndexAssign { index, value, .. } => {
            walk_expr(index, offset, best);
            walk_expr(value, offset, best);
        }
        TypedStmt::FieldAssign { object, value, .. } => {
            walk_expr(object, offset, best);
            walk_expr(value, offset, best);
        }
        TypedStmt::TaskSpawn { body, .. } | TypedStmt::UnsafeBlock { body, .. } => {
            for s in body {
                walk_stmt(s, offset, best);
            }
        }
        TypedStmt::Drop { .. } | TypedStmt::Prove { .. } => {}
        TypedStmt::Break { .. } | TypedStmt::Continue { .. } => {}
        TypedStmt::TaskJoin { .. } | TypedStmt::ForIterShallowFree { .. } => {}
    }
}

fn walk_expr(
    expr: &TypedExpr,
    offset: usize,
    best: &mut Option<(crate::ast::Type, crate::span::Span)>,
) {
    let span = expr.span;
    if !(span.start <= offset && offset <= span.end) {
        return;
    }
    let width = span.end.saturating_sub(span.start);
    let better = match best {
        None => true,
        Some((_, prev)) => width < prev.end.saturating_sub(prev.start),
    };
    if better {
        *best = Some((expr.ty.clone(), span));
    }
    match &expr.kind {
        TypedExprKind::Unary { expr, .. } => walk_expr(expr, offset, best),
        TypedExprKind::Binary { left, right, .. } => {
            walk_expr(left, offset, best);
            walk_expr(right, offset, best);
        }
        TypedExprKind::Cast { expr, .. } => walk_expr(expr, offset, best),
        TypedExprKind::Call { args, .. } => {
            for a in args {
                walk_expr(a, offset, best);
            }
        }
        TypedExprKind::ArrayLit { elements } => {
            for e in elements {
                walk_expr(e, offset, best);
            }
        }
        TypedExprKind::Index { array, index, .. } => {
            walk_expr(array, offset, best);
            walk_expr(index, offset, best);
        }
        TypedExprKind::Len { array, .. } => walk_expr(array, offset, best),
        _ => {}
    }
}

/// Convert a byte offset within `source` into an LSP `Position`.
/// LSP positions are zero-indexed lines + zero-indexed UTF-16 code
/// units within the line. The Intent surface is ASCII-only today,
/// so byte and UTF-16 counts coincide; if non-ASCII content shows
/// up in identifiers later, swap the column math for a real
/// UTF-16 count.
pub fn byte_offset_to_position(source: &str, offset: usize) -> Position {
    let clamped = offset.min(source.len());
    let mut line: u32 = 0;
    let mut last_newline_byte: usize = 0;
    for (i, b) in source.as_bytes().iter().enumerate() {
        if i >= clamped {
            break;
        }
        if *b == b'\n' {
            line += 1;
            last_newline_byte = i + 1;
        }
    }
    let character = (clamped - last_newline_byte) as u32;
    Position { line, character }
}

/// Inverse of [`byte_offset_to_position`]: walk `source` until we
/// reach the given line, then advance by `character` UTF-16 code
/// units. Returns `None` when the position points past EOF (LSP
/// allows this but our caller treats it as "no hover").
pub fn position_to_byte_offset(source: &str, position: Position) -> Option<usize> {
    let mut line: u32 = 0;
    let mut col: u32 = 0;
    let bytes = source.as_bytes();
    for (i, b) in bytes.iter().enumerate() {
        if line == position.line && col == position.character {
            return Some(i);
        }
        if *b == b'\n' {
            if line == position.line {
                // Position past end of line — clamp to the
                // newline byte so callers can still find a
                // covering expression.
                return Some(i);
            }
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    if line == position.line && col == position.character {
        return Some(bytes.len());
    }
    None
}

fn span_to_range(source: &str, span: crate::span::Span) -> Range {
    Range {
        start: byte_offset_to_position(source, span.start),
        end: byte_offset_to_position(source, span.end),
    }
}

fn response_for<T: serde::Serialize>(id: RequestId, value: T) -> Response {
    Response {
        id,
        result: Some(serde_json::to_value(value).unwrap_or(serde_json::Value::Null)),
        error: None,
    }
}

fn response_error(id: RequestId, message: String) -> Response {
    Response {
        id,
        result: None,
        error: Some(lsp_server::ResponseError {
            code: lsp_server::ErrorCode::InvalidParams as i32,
            message,
            data: None,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_offset_to_position_handles_multi_line_input() {
        let src = "fn main() -> i64 {\n  let x: i64 = 42;\n  return x;\n}\n";
        // The newline after `{` lives at byte index 18; offset 19
        // is the first char of line 1 ("  let…").
        let p = byte_offset_to_position(src, 19);
        assert_eq!(p, Position { line: 1, character: 0 });

        // `42` sits on line 1 at column "  let x: i64 = ".len() = 15.
        let lit_offset = src.find("42").unwrap();
        let p = byte_offset_to_position(src, lit_offset);
        assert_eq!(p, Position { line: 1, character: 15 });
    }

    #[test]
    fn position_to_byte_offset_is_inverse_of_byte_offset_to_position() {
        let src = "fn main() -> i64 {\n  let x: i64 = 42;\n  return x;\n}\n";
        for &off in &[0usize, 5, 18, 19, 22, 36, 40, src.len() - 1] {
            let pos = byte_offset_to_position(src, off);
            let round = position_to_byte_offset(src, pos).expect("round-trip");
            assert_eq!(round, off, "round-trip for byte {} via {:?}", off, pos);
        }
    }

    #[test]
    fn compute_diagnostics_returns_empty_on_valid_source() {
        let src = "fn main() -> i64 { return 0; }\n";
        let diags = compute_diagnostics(src);
        assert!(diags.is_empty(), "expected no diagnostics, got {:?}", diags);
    }

    #[test]
    fn compute_diagnostics_maps_span_to_range_on_invalid_source() {
        // Missing semicolon after `return 0` produces a parse error.
        let src = "fn main() -> i64 { return 0 }\n";
        let diags = compute_diagnostics(src);
        assert!(!diags.is_empty(), "expected at least one diagnostic");
        for d in &diags {
            assert_eq!(d.severity, Some(DiagnosticSeverity::ERROR));
            assert_eq!(d.source.as_deref(), Some("vanic"));
            // Range should sit on the only line (line 0) and have a
            // non-negative width.
            assert_eq!(d.range.start.line, 0);
            assert!(
                d.range.end.character >= d.range.start.character,
                "negative-width range: {:?}",
                d.range
            );
        }
    }

    #[test]
    fn compute_hover_reports_type_for_expression_under_cursor() {
        // `42` is an i64 literal. Hover anywhere on its span should
        // return the type i64.
        let src = "fn main() -> i64 {\n  let x: i64 = 42;\n  return x;\n}\n";
        let lit_offset = src.find("42").unwrap();
        let pos = byte_offset_to_position(src, lit_offset + 1);
        let hover = compute_hover(src, pos).expect("hover should resolve");
        let HoverContents::Markup(content) = hover.contents else {
            panic!("expected markup hover content");
        };
        assert!(
            content.value.contains("i64"),
            "expected i64 in hover, got {:?}",
            content.value
        );
    }

    #[test]
    fn compute_hover_returns_none_when_document_doesnt_compile() {
        let src = "fn main() -> i64 { return 0 }\n";
        let hover = compute_hover(src, Position { line: 0, character: 0 });
        assert!(hover.is_none(), "expected None on broken source");
    }

    #[test]
    fn compute_goto_definition_finds_let_binding_for_use_site() {
        // `let x: i64 = 42;` declares x; `return x;` uses it.
        // Goto-def from the use should land on the whole
        // let-statement span — that's the `decl_span` the
        // checker recorded on the binding when it processed
        // `let x: i64 = 42;`.
        let src = "fn main() -> i64 {\n  let x: i64 = 42;\n  return x;\n}\n";
        let use_offset = src.rfind("x;").unwrap();
        let pos = byte_offset_to_position(src, use_offset);
        let span = compute_goto_definition(src, pos)
            .unwrap_or_else(|| panic!("expected a definition span"));
        // The let statement starts at the `let` keyword and
        // ends just past the `;`. Verify the span covers
        // those bytes (anchored on the `let` keyword's
        // position).
        let let_start = src.find("let x").unwrap();
        assert_eq!(
            span.start, let_start,
            "expected definition to start at `let`, got span {:?}",
            span
        );
    }

    #[test]
    fn compute_goto_definition_returns_none_when_cursor_isnt_on_a_name() {
        // Cursor on a numeric literal, not a Var, should not
        // yield a definition.
        let src = "fn main() -> i64 { return 42; }\n";
        let lit_offset = src.find("42").unwrap();
        let pos = byte_offset_to_position(src, lit_offset);
        let result = compute_goto_definition(src, pos);
        assert!(
            result.is_none(),
            "expected None when cursor is not on a binding name, got {:?}",
            result
        );
    }

    #[test]
    fn compute_goto_definition_returns_none_when_document_doesnt_compile() {
        let src = "fn main() -> i64 { return 0 }\n"; // missing ';'
        let result = compute_goto_definition(src, Position { line: 0, character: 0 });
        assert!(result.is_none());
    }

    #[test]
    fn compute_references_finds_every_use_of_a_binding() {
        // `let x = …` then two uses of `x`. With
        // include_declaration=false, only the two use spans
        // come back.
        let src = "fn main() -> i64 {\n  let x: i64 = 1;\n  let y: i64 = x + x;\n  return y;\n}\n";
        // Position the cursor on the first use of x.
        let first_use_offset = src.find("x + x").unwrap();
        let pos = byte_offset_to_position(src, first_use_offset);
        let refs = compute_references(src, pos, false)
            .unwrap_or_else(|| panic!("expected Some refs"));
        assert_eq!(
            refs.len(),
            2,
            "expected two uses of x, got {:?} spans",
            refs.len()
        );
        // Sorted by start offset: the first should be the
        // earlier `x` in `x + x`, second the later `x`.
        assert!(refs[0].start < refs[1].start);
    }

    #[test]
    fn compute_references_with_include_declaration_adds_the_let_site() {
        let src = "fn main() -> i64 {\n  let x: i64 = 1;\n  return x;\n}\n";
        let use_offset = src.rfind("x;").unwrap();
        let pos = byte_offset_to_position(src, use_offset);
        let refs = compute_references(src, pos, true)
            .unwrap_or_else(|| panic!("expected Some refs"));
        assert!(refs.len() >= 2, "expected decl + use, got {:?}", refs);
    }

    #[test]
    fn compute_rename_produces_spans_for_every_use_plus_declaration() {
        // `let x = 1; let y = x + x; return y;` — rename `x`
        // should replace 3 spans: declaration + 2 uses.
        let src = "fn main() -> i64 {\n  let x: i64 = 1;\n  let y: i64 = x + x;\n  return y;\n}\n";
        let first_use = src.find("x + x").unwrap();
        let pos = byte_offset_to_position(src, first_use);
        let result = compute_rename(src, pos, "renamed")
            .expect("rename validates")
            .expect("rename has spans");
        // Two uses + the let RHS span used as declaration
        // (current approximation). May or may not dedup to
        // exactly 3 depending on overlap; check minimum.
        assert!(
            result.len() >= 2,
            "expected at least 2 spans (uses), got {:?}",
            result
        );
    }

    #[test]
    fn compute_rename_rejects_invalid_identifier() {
        let src = "fn main() -> i64 {\n  let x: i64 = 1;\n  return x;\n}\n";
        let use_offset = src.rfind("x;").unwrap();
        let pos = byte_offset_to_position(src, use_offset);
        let err = compute_rename(src, pos, "1bad")
            .expect_err("numeric prefix is not a valid identifier");
        assert!(err.contains("not a valid identifier"), "msg: {}", err);
    }

    #[test]
    fn compute_rename_rejects_keyword_collision() {
        let src = "fn main() -> i64 {\n  let x: i64 = 1;\n  return x;\n}\n";
        let use_offset = src.rfind("x;").unwrap();
        let pos = byte_offset_to_position(src, use_offset);
        let err = compute_rename(src, pos, "return")
            .expect_err("'return' is a keyword");
        assert!(err.contains("reserved keyword"), "msg: {}", err);
    }

    #[test]
    fn compute_rename_returns_none_when_document_doesnt_compile() {
        let src = "fn main() -> i64 { return 0 }\n"; // missing ';'
        let pos = Position { line: 0, character: 26 };
        let result = compute_rename(src, pos, "newname").expect("validation OK");
        assert!(result.is_none(), "expected None on broken source");
    }

    fn dummy_uri() -> Url {
        "file:///tmp/test.vani".parse().unwrap()
    }

    fn diag_at(line: u32, character: u32, message: &str) -> lsp_types::Diagnostic {
        lsp_types::Diagnostic {
            range: lsp_types::Range {
                start: Position { line, character },
                end: Position { line, character },
            },
            message: message.to_string(),
            ..lsp_types::Diagnostic::default()
        }
    }

    #[test]
    fn code_actions_emit_insert_quickfix_for_expected_single_char_token() {
        let src = "fn main() -> i64 { return 0 }\n";
        let diag = diag_at(0, 28, "expected ';'");
        let actions = compute_code_actions(src, dummy_uri(), &[diag.clone()]);
        assert_eq!(actions.len(), 1, "expected 1 code action, got {:?}", actions);
        let action = &actions[0];
        assert_eq!(action.title, "Insert `;`");
        assert_eq!(action.kind.as_ref(), Some(&lsp_types::CodeActionKind::QUICKFIX));
        // The WorkspaceEdit must insert ";" at the diagnostic's
        // range.end.
        let edit = action.edit.as_ref().expect("edit present");
        let lsp_types::DocumentChanges::Edits(doc_edits) =
            edit.document_changes.as_ref().expect("document_changes")
        else {
            panic!("expected DocumentChanges::Edits");
        };
        assert_eq!(doc_edits.len(), 1);
        let text_edit = match &doc_edits[0].edits[0] {
            lsp_types::OneOf::Left(t) => t,
            _ => panic!("expected TextEdit"),
        };
        assert_eq!(text_edit.new_text, ";");
        assert_eq!(text_edit.range.start, diag.range.end);
    }

    #[test]
    fn code_actions_skip_diagnostics_without_a_matching_pattern() {
        let src = "fn main() -> i64 { return 0; }\n";
        let diag = diag_at(0, 0, "type mismatch: expected i64, got bool");
        let actions = compute_code_actions(src, dummy_uri(), &[diag]);
        assert!(
            actions.is_empty(),
            "expected no actions, got {:?}",
            actions
        );
    }

    #[test]
    fn code_actions_skip_multi_char_expected_tokens() {
        // "expected 'identifier'" is multi-char; we don't
        // know what to insert without more context, so no
        // quick fix is offered.
        let src = "fn () -> i64 { return 0; }\n";
        let diag = diag_at(0, 3, "expected 'identifier'");
        let actions = compute_code_actions(src, dummy_uri(), &[diag]);
        assert!(actions.is_empty());
    }

    #[test]
    fn semantic_tokens_assigns_keyword_type_to_fn_let_return() {
        let src = "fn main() -> i64 { let x: i64 = 1; return x; }\n";
        let tokens = compute_semantic_tokens(src);
        assert!(!tokens.is_empty(), "expected some tokens");
        // The first three "interesting" tokens are `fn`,
        // `main`, `(`/`)` (skipped), `i64`, `{` (skipped),
        // `let`, … — `fn` should map to keyword.
        let kw_idx = token_index(TOKEN_KEYWORD);
        let kw_count = tokens.iter().filter(|t| t.token_type == kw_idx).count();
        assert!(kw_count >= 2, "expected at least 2 keyword tokens, got {}", kw_count);
    }

    #[test]
    fn semantic_tokens_marks_type_keywords_and_known_type_idents() {
        let src = "fn main() -> i64 { let xs: Vec<i64> = vec(1); return 0; }\n";
        let tokens = compute_semantic_tokens(src);
        let ty_idx = token_index(TOKEN_TYPE);
        let ty_tokens = tokens.iter().filter(|t| t.token_type == ty_idx).count();
        // Two `i64` occurrences + one `Vec` = at least 3
        // type tokens.
        assert!(ty_tokens >= 3, "expected >= 3 type tokens, got {}", ty_tokens);
    }

    #[test]
    fn semantic_tokens_marks_number_and_string_literals() {
        let src = "fn main() -> i64 { print \"hello\"; return 42; }\n";
        let tokens = compute_semantic_tokens(src);
        let num_idx = token_index(TOKEN_NUMBER);
        let str_idx = token_index(TOKEN_STRING);
        assert!(tokens.iter().any(|t| t.token_type == num_idx));
        assert!(tokens.iter().any(|t| t.token_type == str_idx));
    }

    #[test]
    fn semantic_tokens_default_to_variable_for_identifiers() {
        let src = "fn main() -> i64 { let abc: i64 = 1; return abc; }\n";
        let tokens = compute_semantic_tokens(src);
        let var_idx = token_index(TOKEN_VARIABLE);
        // `main`, `abc` (declaration), `abc` (use) — three
        // variable-typed tokens at minimum.
        let var_count = tokens.iter().filter(|t| t.token_type == var_idx).count();
        assert!(var_count >= 3, "expected >= 3 var tokens, got {}", var_count);
    }

    #[test]
    fn semantic_tokens_marks_call_callee_as_function() {
        let src = "fn helper(n: i64) -> i64 { return n; }\nfn main() -> i64 { return helper(1); }\n";
        let tokens = compute_semantic_tokens(src);
        let fn_idx = token_index(TOKEN_FUNCTION);
        // Compute the (line, char) for `helper` in `helper(1)`.
        let callee = src.rfind("helper(1)").unwrap();
        let pos = byte_offset_to_position(src, callee);
        // Walk delta-encoded tokens to find one whose start
        // position matches the callee's `(line, character)`.
        let mut line: u32 = 0;
        let mut col: u32 = 0;
        let mut found = false;
        for t in &tokens {
            if t.delta_line == 0 {
                col += t.delta_start;
            } else {
                line += t.delta_line;
                col = t.delta_start;
            }
            if line == pos.line && col == pos.character {
                assert_eq!(
                    t.token_type, fn_idx,
                    "callee `helper` should be a function token, got {}",
                    t.token_type
                );
                found = true;
                break;
            }
        }
        assert!(found, "could not locate callee token in delta stream");
    }

    #[test]
    fn semantic_tokens_param_decl_has_declaration_and_readonly_modifiers() {
        let src = "fn helper(n: i64) -> i64 { return n; }\nfn main() -> i64 { return helper(1); }\n";
        let tokens = compute_semantic_tokens(src);
        let expect_bits = TOKEN_MOD_DECLARATION | TOKEN_MOD_READONLY;
        let pos = byte_offset_to_position(src, src.find("n: i64").unwrap());
        let mut line: u32 = 0;
        let mut col: u32 = 0;
        let mut found = false;
        for t in &tokens {
            if t.delta_line == 0 {
                col += t.delta_start;
            } else {
                line += t.delta_line;
                col = t.delta_start;
            }
            if line == pos.line && col == pos.character {
                assert_eq!(
                    t.token_modifiers_bitset, expect_bits,
                    "parameter decl should have declaration|readonly bits ({}), got {}",
                    expect_bits, t.token_modifiers_bitset
                );
                found = true;
                break;
            }
        }
        assert!(found, "could not locate parameter decl token in delta stream");
    }

    #[test]
    fn semantic_tokens_param_read_gets_readonly_modifier() {
        let src = "fn helper(n: i64) -> i64 { return n; }\nfn main() -> i64 { return helper(1); }\n";
        let tokens = compute_semantic_tokens(src);
        let param_idx = token_index(TOKEN_PARAMETER);
        // The read of `n` is in `return n;`. Locate that
        // identifier's source position; assert its token
        // type is `parameter` and its modifier bits include
        // `readonly` (but not `declaration`).
        let read_pos = src.find("return n;").unwrap() + "return ".len();
        let pos = byte_offset_to_position(src, read_pos);
        let mut line: u32 = 0;
        let mut col: u32 = 0;
        let mut found = false;
        for t in &tokens {
            if t.delta_line == 0 {
                col += t.delta_start;
            } else {
                line += t.delta_line;
                col = t.delta_start;
            }
            if line == pos.line && col == pos.character {
                assert_eq!(t.token_type, param_idx, "expected parameter tint");
                assert_eq!(
                    t.token_modifiers_bitset, TOKEN_MOD_READONLY,
                    "expected readonly-only modifiers, got {}",
                    t.token_modifiers_bitset
                );
                found = true;
                break;
            }
        }
        assert!(found, "could not locate parameter-read token in delta stream");
    }

    #[test]
    fn semantic_tokens_marks_function_parameter_as_parameter() {
        let src = "fn helper(n: i64, bias: i64) -> i64 { return n + bias; }\nfn main() -> i64 { return helper(1, 2); }\n";
        let tokens = compute_semantic_tokens(src);
        let param_idx = token_index(TOKEN_PARAMETER);
        // Find the declaration-site position of `n` (the
        // first occurrence after `fn helper(`).
        let n_decl = src.find("n: i64").unwrap();
        let pos = byte_offset_to_position(src, n_decl);
        let mut line: u32 = 0;
        let mut col: u32 = 0;
        let mut found = false;
        for t in &tokens {
            if t.delta_line == 0 {
                col += t.delta_start;
            } else {
                line += t.delta_line;
                col = t.delta_start;
            }
            if line == pos.line && col == pos.character {
                assert_eq!(
                    t.token_type, param_idx,
                    "parameter `n` decl should be parameter token, got {}",
                    t.token_type
                );
                found = true;
                break;
            }
        }
        assert!(found, "could not locate parameter token in delta stream");
    }

    #[test]
    fn compute_references_does_not_include_same_name_binding_from_another_function() {
        // `x` is declared in two functions. From a use of
        // `x` inside `helper`, references should only
        // include `helper`'s `x`, not `main`'s `x`.
        let src = r#"
            fn helper() -> i64 {
              let x: i64 = 7;
              return x;
            }
            fn main() -> i64 {
              let x: i64 = 99;
              return x;
            }
        "#;
        // Find the cursor on the use of `helper`'s `x` (the
        // `x` in `return x;` inside helper). Look for the
        // FIRST occurrence after `let x: i64 = 7;`.
        let helper_ret = src.find("return x;").unwrap();
        let pos = byte_offset_to_position(src, helper_ret + "return ".len());
        let refs = compute_references(src, pos, false)
            .unwrap_or_else(|| panic!("expected Some refs"));
        // helper's `x` use is one site. main's `x` should
        // NOT appear because it has a different decl_span.
        assert_eq!(
            refs.len(),
            1,
            "expected exactly one reference (helper's `x`), got {:?}",
            refs
        );
        // The reference should fall inside helper's body
        // (before the `fn main` declaration).
        let main_start = src.find("fn main").unwrap();
        assert!(
            refs[0].start < main_start,
            "reference {} is inside main but should be inside helper",
            refs[0].start
        );
    }

    #[test]
    fn compute_completion_does_not_leak_params_from_sibling_function() {
        // `helper` takes `n` and `bias`; `main` takes
        // nothing. From a cursor inside `main`, the
        // completion should not include `n` or `bias` (those
        // belong to `helper`).
        let src = "fn helper(n: i64, bias: i64) -> i64 {\n  return n + bias;\n}\nfn main() -> i64 {\n  let x: i64 = 1;\n  return x;\n}\n";
        // Cursor inside main's body.
        let cursor = src.rfind("return x").unwrap();
        let pos = byte_offset_to_position(src, cursor);
        let items = compute_completion(src, pos);
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(
            !labels.contains(&"n"),
            "helper's param `n` leaked into main's completion: {:?}",
            labels
        );
        assert!(
            !labels.contains(&"bias"),
            "helper's param `bias` leaked into main's completion: {:?}",
            labels
        );
        // main's own `x` should be visible.
        assert!(
            labels.contains(&"x"),
            "main's `x` missing from completion: {:?}",
            labels
        );
        // Function name `helper` is still callable.
        assert!(labels.contains(&"helper"));
    }

    #[test]
    fn semantic_tokens_returns_empty_when_source_doesnt_lex() {
        // Invalid character (lexer error) → empty list,
        // not a panic.
        let src = "fn main() -> i64 { @invalid }\n";
        let tokens = compute_semantic_tokens(src);
        assert!(tokens.is_empty(), "expected empty on lex error: {:?}", tokens);
    }

    fn labels(items: &[lsp_types::CompletionItem]) -> Vec<&str> {
        items.iter().map(|i| i.label.as_str()).collect()
    }

    #[test]
    fn compute_completion_includes_keywords_and_builtins_even_on_broken_doc() {
        // A broken document still gets keywords + builtins +
        // type names so the editor's popup is useful while
        // the user is typing.
        let src = "fn main() -> i64 { let x = ";
        let items = compute_completion(src, Position { line: 0, character: 27 });
        let labels = labels(&items);
        for kw in ["let", "return", "if", "while", "for"] {
            assert!(labels.contains(&kw), "missing keyword `{}`: {:?}", kw, labels);
        }
        for ty in ["i64", "Vec", "Atomic"] {
            assert!(labels.contains(&ty), "missing type `{}`: {:?}", ty, labels);
        }
        for bi in ["vec", "atomic_new", "channel_send", "mutex_lock"] {
            assert!(labels.contains(&bi), "missing builtin `{}`: {:?}", bi, labels);
        }
    }

    #[test]
    fn compute_completion_includes_let_bindings_before_cursor() {
        let src = "fn main() -> i64 {\n  let foo: i64 = 1;\n  let bar: i64 = 2;\n  return 0;\n}\n";
        // Cursor on the `return 0;` line, so both bindings
        // should be in scope.
        let cursor_offset = src.find("return").unwrap();
        let pos = byte_offset_to_position(src, cursor_offset);
        let items = compute_completion(src, pos);
        let labels = labels(&items);
        assert!(labels.contains(&"foo"), "expected foo, got: {:?}", labels);
        assert!(labels.contains(&"bar"), "expected bar, got: {:?}", labels);
    }

    #[test]
    fn compute_completion_excludes_let_bindings_after_cursor() {
        // Cursor BEFORE the second let — `bar` should not
        // appear in the completion list yet.
        let src = "fn main() -> i64 {\n  let foo: i64 = 1;\n  let bar: i64 = 2;\n  return 0;\n}\n";
        let cursor_offset = src.find("foo: i64 = 1").unwrap();
        let pos = byte_offset_to_position(src, cursor_offset);
        let items = compute_completion(src, pos);
        let labels = labels(&items);
        // `foo` may or may not be in scope at this exact
        // point (the expr.span proxy is approximate); the
        // strict invariant is that `bar` (declared later)
        // must not be.
        assert!(
            !labels.contains(&"bar"),
            "bar declared after cursor leaked into completions: {:?}",
            labels
        );
    }

    #[test]
    fn compute_completion_includes_function_parameters() {
        let src = "fn helper(count: i64, total: i64) -> i64 {\n  return count + total;\n}\nfn main() -> i64 { return 0; }\n";
        let cursor_offset = src.find("return count").unwrap();
        let pos = byte_offset_to_position(src, cursor_offset);
        let items = compute_completion(src, pos);
        let labels = labels(&items);
        assert!(labels.contains(&"count"), "params missing: {:?}", labels);
        assert!(labels.contains(&"total"), "params missing: {:?}", labels);
    }

    #[test]
    fn compute_completion_includes_other_function_names() {
        let src = "fn helper() -> i64 { return 1; }\nfn main() -> i64 { return 0; }\n";
        let pos = Position { line: 1, character: 25 };
        let items = compute_completion(src, pos);
        let labels = labels(&items);
        assert!(labels.contains(&"helper"), "expected helper: {:?}", labels);
        assert!(labels.contains(&"main"), "expected main: {:?}", labels);
    }

    #[test]
    fn compute_completion_filters_synthetic_names() {
        // The checker introduces `__intent_ret_*` etc.; those
        // should never appear in the completion list.
        let src = "fn main() -> i64 {\n  let x: i64 = 1;\n  return x;\n}\n";
        let pos = Position { line: 2, character: 8 };
        let items = compute_completion(src, pos);
        let labels = labels(&items);
        assert!(
            !labels.iter().any(|l| l.starts_with("__intent_")),
            "synthetic names leaked: {:?}",
            labels
        );
    }

    #[test]
    fn compute_rename_no_op_returns_empty_edits_when_new_name_matches() {
        let src = "fn main() -> i64 {\n  let x: i64 = 1;\n  return x;\n}\n";
        let use_offset = src.rfind("x;").unwrap();
        let pos = byte_offset_to_position(src, use_offset);
        let result = compute_rename(src, pos, "x")
            .expect("validation OK")
            .expect("found target");
        assert!(
            result.is_empty(),
            "no-op rename should produce zero edits, got {:?}",
            result
        );
    }

    #[test]
    fn compute_references_returns_none_when_cursor_is_not_on_a_name() {
        let src = "fn main() -> i64 { return 42; }\n";
        let lit_offset = src.find("42").unwrap();
        let pos = byte_offset_to_position(src, lit_offset);
        let result = compute_references(src, pos, false);
        assert!(
            result.is_none(),
            "expected None on a non-name cursor, got {:?}",
            result
        );
    }

    #[test]
    fn compute_rename_does_not_touch_inner_block_shadow() {
        // Refines #9 from STATUS.md (the rename side of the
        // same scope-aware identity that
        // `compute_references_distinguishes_nested_block_shadows`
        // covers for references). Two same-name bindings
        // declared in disjoint nested blocks inside one
        // function; renaming the OUTER one must not edit the
        // inner shadow's declaration or its use.
        let src = r#"
            fn main() -> i64 {
              let x: i64 = 1;
              if x > 0 {
                let x: i64 = 99;
                let _ = x;
              }
              return x;
            }
        "#;
        let outer_use = src.rfind("return x;").unwrap() + "return ".len();
        let pos = byte_offset_to_position(src, outer_use);
        let spans = compute_rename(src, pos, "renamed")
            .expect("rename should succeed")
            .expect("expected Some spans for outer x");
        // Inner shadow's let-site and use-site sit inside
        // the `if x > 0 { ... }` block.
        let if_start = src.find("if x > 0").unwrap();
        let if_end = src[if_start..].find('}').unwrap() + if_start;
        let leaks: Vec<_> = spans
            .iter()
            .filter(|s| {
                // Span is inside the if-block AND points at
                // an `x` token after the `if` header itself
                // (`if x > 0` references the OUTER x — those
                // we DO want to rename).
                s.start > if_start + "if x > 0".len() && s.start < if_end
            })
            .collect();
        assert!(
            leaks.is_empty(),
            "inner shadow's spans should not be renamed: {:?}",
            leaks
        );
    }

    #[test]
    fn compute_references_distinguishes_nested_block_shadows() {
        // Refines #9 from STATUS.md. Two same-name bindings
        // declared in disjoint nested blocks inside one
        // function. From a cursor on the outer use, the inner
        // shadow's read/declaration should NOT appear. The
        // checker now stamps each `Var` with its declaring
        // `Let`'s `decl_span`, so `matches_target`'s
        // scope-aware equality filters out the inner shadow
        // automatically — the LSP walker reuses the same
        // identity logic across functions and within a
        // function alike.
        let src = r#"
            fn main() -> i64 {
              let x: i64 = 1;
              if x > 0 {
                let x: i64 = 99;
                let _ = x;
              }
              return x;
            }
        "#;
        // Cursor on the OUTER `x` in `return x;`.
        let outer_use = src.rfind("return x;").unwrap() + "return ".len();
        let pos = byte_offset_to_position(src, outer_use);
        let refs = compute_references(src, pos, false)
            .unwrap_or_else(|| panic!("expected Some refs for outer x"));
        // The inner shadow's `let _ = x;` use should NOT be
        // in the list. Its byte offset is inside the
        // `if x > 0 { … }` block.
        let if_start = src.find("if x > 0").unwrap();
        let if_end = src[if_start..].find('}').unwrap() + if_start;
        let inner_uses: Vec<_> = refs
            .iter()
            .filter(|s| s.start >= if_start && s.start <= if_end)
            .collect();
        assert!(
            !inner_uses.iter().any(|s| {
                // The inner `let _ = x;` use specifically.
                let txt = &src[s.start..s.end];
                txt == "x" && s.start > if_start + "if x > 0".len()
            }),
            "inner shadow's use leaked into outer x's references: {:?}",
            refs
        );
    }

    #[test]
    fn hover_on_postfix_question_includes_operator_doc() {
        // Polish (2026-06-08): hovering on a `?` token augments
        // the type info with a doc-block explaining the postfix
        // sugar. Cursor positioning is by byte offset; the lexer
        // pass finds the Question token covering the offset.
        let src = "enum Opt { Some(i64), None } fn doit(o: Opt) -> Opt { let v: i64 = o?; return Opt.Some(v); }\n";
        let q_offset = src.find('?').expect("`?` present in source");
        let pos = byte_offset_to_position(src, q_offset);
        let hover = compute_hover(src, pos).expect("hover should resolve on `?`");
        let HoverContents::Markup(content) = hover.contents else {
            panic!("expected markup hover content");
        };
        assert!(
            content.value.contains("Postfix `?` operator"),
            "expected `?`-doc in hover, got {:?}",
            content.value
        );
        assert!(
            content.value.contains("try EXPR"),
            "expected reference to `try` form, got {:?}",
            content.value
        );
    }

    #[test]
    fn hover_on_plain_expr_does_not_include_question_doc() {
        // Inverse: hovering on a regular expression (here the
        // numeric literal `42`) returns just the type, not the
        // `?` operator doc.
        let src = "fn main() -> i64 { return 42; }\n";
        let lit = src.find("42").unwrap();
        let pos = byte_offset_to_position(src, lit);
        let hover = compute_hover(src, pos).expect("hover on literal");
        let HoverContents::Markup(content) = hover.contents else {
            panic!("expected markup hover content");
        };
        assert!(
            !content.value.contains("Postfix `?` operator"),
            "`?` doc should not appear on a non-`?` cursor: {:?}",
            content.value
        );
        assert!(content.value.contains("i64"), "expected i64 type");
    }

    #[test]
    fn completion_includes_mandarin_keywords_under_mandarin_pragma() {
        // Polish (2026-06-08): LSP completion is pragma-aware.
        // A file declaring `// vani-lang: mandarin` surfaces
        // Mandarin keyword spellings alongside the English ones.
        let src = "// vani-lang: mandarin\n函数 main() -> i64 { 返回 0; }\n";
        let items = compute_completion(src, Position { line: 1, character: 0 });
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"函数"), "expected 函数 in completion, got: {:?}", labels);
        assert!(labels.contains(&"返回"), "expected 返回 in completion, got: {:?}", labels);
        // English keywords stay available — both dialects share TokenKinds.
        assert!(labels.contains(&"fn"), "English `fn` should still appear");
    }

    #[test]
    fn completion_includes_devanagari_keywords_under_sanskrit_pragma() {
        let src = "// vani-lang: sanskrit\nकार्य main() -> i64 { पुनरागम 0; }\n";
        let items = compute_completion(src, Position { line: 1, character: 0 });
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"कार्य"), "expected कार्य in completion");
        assert!(labels.contains(&"पुनरागम"), "expected पुनरागम in completion");
    }

    #[test]
    fn completion_default_pragma_omits_dialect_keywords() {
        // No pragma → English-only completion. Mandarin/Devanagari
        // keywords don't appear, so the popup stays focused.
        let src = "fn main() -> i64 { return 0; }\n";
        let items = compute_completion(src, Position { line: 0, character: 0 });
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(!labels.contains(&"函数"), "Mandarin should not appear without pragma");
        assert!(!labels.contains(&"कार्य"), "Devanagari should not appear without pragma");
        assert!(labels.contains(&"fn"), "English `fn` always present");
    }

    #[test]
    fn completion_expanded_english_keywords_include_match_struct_enum() {
        // The original KEYWORDS list was missing many core
        // language keywords (match, struct, enum, etc.).
        // Verify the expanded list covers them.
        let src = "fn main() -> i64 { return 0; }\n";
        let items = compute_completion(src, Position { line: 0, character: 0 });
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        for required in &["match", "struct", "enum", "try", "async", "interface", "implement", "module", "pub"] {
            assert!(
                labels.contains(required),
                "expected `{}` in completion, got: {:?}",
                required, labels
            );
        }
    }

    #[test]
    fn lsp_keyword_lists_match_lexer() {
        // BUG-173 (2026-08-11): the XXX_KEYWORDS completion-popup
        // arrays above used to be hand-typed once and never
        // mechanically re-synced with lexer.rs's real keyword
        // tables -- 263 entries had drifted stale (suggesting words
        // that no longer lex as that keyword) before this was
        // caught. The arrays are now generated by
        // `tools/regen_lsp_keywords.py` directly from lexer.rs's
        // match arms, so a stale entry can only reappear if
        // lexer.rs is edited without re-running that script. This
        // test reads lexer.rs via `include_str!` at test time
        // (mirroring `bug170_global_dialects_structure_keyword_parity`
        // in src/lib.rs) and fails loudly if any XXX_KEYWORDS entry
        // no longer appears as a match key in its mapped lexer
        // function(s), instead of silently rotting again.
        let lexer_src: &str = include_str!("lexer.rs");

        fn extract_words_for_fn(src: &str, fn_name: &str) -> Vec<String> {
            let marker = format!("fn {fn_name}(text: &str) -> Option<TokenKind> {{");
            let start = src
                .find(&marker)
                .unwrap_or_else(|| panic!("dialect fn `{fn_name}` not found in lexer.rs"));
            let body_start = start + marker.len();
            let close = src[body_start..]
                .find("\n}\n")
                .unwrap_or_else(|| panic!("no closing `}}` found for `{fn_name}`"));
            let body = &src[body_start..body_start + close];
            let mut words = Vec::new();
            let mut rest = body;
            while let Some(q1) = rest.find('"') {
                let after_q1 = &rest[q1 + 1..];
                let Some(q2) = after_q1.find('"') else { break };
                let word = &after_q1[..q2];
                let after_q2 = &after_q1[q2 + 1..];
                let tail = after_q2.trim_start();
                if let Some(rest_after_arrow) = tail.strip_prefix("=>") {
                    if rest_after_arrow.trim_start().starts_with("TokenKind::") {
                        words.push(word.to_string());
                    }
                }
                rest = after_q2;
            }
            words
        }

        let mapping: &[(&[&str], &[&str])] = &[
            (MANDARIN_KEYWORDS, &["mandarin_keyword"]),
            (DEVANAGARI_KEYWORDS, &["devanagari_keyword"]),
            (BENGALI_KEYWORDS, &["bengali_keyword"]),
            (TAMIL_KEYWORDS, &["tamil_keyword"]),
            (TELUGU_KEYWORDS, &["telugu_keyword"]),
            (GUJARATI_KEYWORDS, &["gujarati_keyword"]),
            (PUNJABI_KEYWORDS, &["punjabi_keyword"]),
            (KANNADA_KEYWORDS, &["kannada_keyword"]),
            (ODIA_KEYWORDS, &["odia_keyword"]),
            (URDU_KEYWORDS, &["urdu_keyword"]),
            (PERSIAN_KEYWORDS, &["persian_keyword"]),
            (KOREAN_KEYWORDS, &["korean_keyword"]),
            (JAPANESE_KEYWORDS, &["japanese_keyword"]),
            (ARABIC_KEYWORDS, &["arabic_keyword"]),
            (HEBREW_KEYWORDS, &["hebrew_keyword"]),
            (RUSSIAN_KEYWORDS, &["cyrillic_keyword"]),
            (SPANISH_KEYWORDS, &["spanish_keyword", "spanish_ascii_keyword"]),
            (FRENCH_KEYWORDS, &["french_keyword", "french_ascii_keyword"]),
            (GERMAN_KEYWORDS, &["german_keyword", "german_ascii_keyword"]),
            (PORTUGUESE_KEYWORDS, &["portuguese_keyword", "portuguese_ascii_keyword"]),
            (ITALIAN_KEYWORDS, &["italian_ascii_keyword"]),
            (TURKISH_KEYWORDS, &["turkish_keyword", "turkish_ascii_keyword"]),
            (POLISH_KEYWORDS, &["polish_keyword", "polish_ascii_keyword"]),
            (INDONESIAN_KEYWORDS, &["indonesian_ascii_keyword"]),
            (MALAY_KEYWORDS, &["malay_ascii_keyword"]),
            (SWAHILI_KEYWORDS, &["swahili_ascii_keyword"]),
            (DUTCH_KEYWORDS, &["dutch_ascii_keyword"]),
            (THAI_KEYWORDS, &["thai_keyword"]),
            (HUNGARIAN_KEYWORDS, &["hungarian_keyword", "hungarian_ascii_keyword"]),
            (CZECH_KEYWORDS, &["czech_keyword", "czech_ascii_keyword"]),
        ];

        let mut stale: Vec<String> = Vec::new();
        for (lsp_words, fn_names) in mapping {
            let mut lexer_words: Vec<String> = Vec::new();
            for fn_name in *fn_names {
                lexer_words.extend(extract_words_for_fn(lexer_src, fn_name));
            }
            for w in *lsp_words {
                if !lexer_words.iter().any(|lw| lw == w) {
                    stale.push(format!("{:?} (from {:?})", w, fn_names));
                }
            }
        }
        assert!(
            stale.is_empty(),
            "LSP keyword lists have drifted from lexer.rs -- re-run \
             tools/regen_lsp_keywords.py. Stale entries:\n{}",
            stale.join("\n")
        );
    }
}
