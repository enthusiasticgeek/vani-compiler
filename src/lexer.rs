use crate::diagnostic::Diagnostic;
use crate::span::Span;

#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    Ident(String),
    Int(i128),
    Float(f64),
    Str(String),
    Fn,
    /// `pure` function modifier: keyword that precedes `fn`.
    /// Marks the function as side-effect-free.
    Pure,
    /// `extern "C" fn name(params) -> R;` — FFI declaration
    /// (closure #269). The body is supplied by an externally-
    /// linked object file; the checker registers the signature
    /// only and never validates a body. Calls to the fn emit
    /// the bare C-ABI symbol (no `fn_` prefix) so external
    /// linkers find it.
    Extern,
    /// `parallel` loop modifier: keyword that precedes `for`.
    /// Marks the iteration as independently parallelizable
    /// (verified by the effects checker).
    Parallel,
    /// `reduce <var> with <op>;` clause on a `parallel for`. The
    /// body must update `<var>` only via the named op; each thread
    /// accumulates a partial value and the runtime combines them.
    Reduce,
    /// Part of the `reduce <var> with <op>;` clause syntax.
    With,
    /// `min` reduction op + builtin function `min(a, b)`.
    Min,
    /// `max` reduction op + builtin function `max(a, b)`.
    Max,
    /// `task <name> { ... }` — declares an affine handle of type
    /// `Task` and a side-effect-free body that runs once. v1
    /// lowers sequentially; the verifier is the value-add.
    Task,
    /// `join <name>;` — consumes a `Task` handle. v1 lowers to a
    /// no-op once the spawn's body has executed.
    Join,
    Let,
    Return,
    If,
    Else,
    While,
    Break,
    Continue,
    Mut,
    For,
    In,
    /// `ref x` — prefix borrow operator. Replaces the older
    /// `&x` shape; the same keyword is used in type position
    /// (`ref T`) and at call-site / for-iter borrows. Refines
    /// T0.0 of the consolidated TODO.
    Ref,
    /// `struct Name { f1: T1, … }` — top-level record-type
    /// declaration. T1.2.
    Struct,
    /// `enum Name { Variant1, Variant2, … }` — top-level
    /// tagged-union declaration. T1.3.
    Enum,
    /// `match expr { Pat then expr, … }` — pattern-match
    /// expression. T1.3.
    Match,
    /// `Pattern then body` — match-arm separator. T1.3.
    Then,
    /// `interface Name { fn …; }` — abstract behavior
    /// declaration. T1.5.
    Interface,
    /// `implement Iface for Type { … }` — bind interface
    /// methods to a concrete type. T1.5.
    Implement,
    /// `where T is Iface` — generic bound clause. T1.5.
    Where,
    /// `T is Iface` — bound predicate keyword. T1.5.
    Is,
    /// `const NAME: T = expr;` — top-level compile-time
    /// constant. v1 restricts the initializer to a literal
    /// expression and the type to Copy. T4.15.
    Const,
    /// `type Name = Type;` — top-level type alias. v1
    /// rejects recursive aliases. T4.15 (type-alias half).
    Type,
    /// `methods on TypeName { fn foo(self: …) -> … { … } }`
    /// — group of methods attached to a concrete type.
    /// Method bodies lower to free functions with names
    /// mangled as `<TypeName>_<methodName>`, so callers can
    /// write `p.foo(args)` and have the checker rewrite the
    /// MethodCall into the mangled call. T1.2 phase 2a.
    Methods,
    /// `from EXPR` — opening of the range form
    /// `from <start> to <end>` used by `for` / `parallel for`.
    /// Replaces `<start>..<end>`. T0.0.
    From,
    /// `to EXPR` — closing of the range form (and future slice
    /// shape `xs[lo to hi]`). T0.0.
    To,
    DotDot,
    /// `.` — field access (`p.x`) and tuple-index (`t.0`)
    /// postfix operator. Distinct from `DotDot`. T1.1 / T1.2.
    Dot,
    Intent,
    Use,
    Requires,
    Ensures,
    Invariant,
    Assert,
    Prove,
    Print,
    /// `try EXPR` — error-propagation sugar over payloaded
    /// enums. If `EXPR` evaluates to the enum's payload-less
    /// "early-return" variant (e.g. `Opt.None`), the enclosing
    /// function returns that value immediately. Otherwise the
    /// payload is extracted and becomes the value of the `try`
    /// expression. Requires the enclosing function's return
    /// type to match the enum type. T2.6.
    Try,
    Len,
    As,
    True,
    False,
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    Bool,
    Vec,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Colon,
    ColonColon,
    Semicolon,
    Comma,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Bang,
    Equal,
    EqEq,
    BangEq,
    Less,
    LessEq,
    LessLess,
    Greater,
    GreaterEq,
    GreaterGreater,
    Amp,
    AndAnd,
    Pipe,
    OrOr,
    Caret,
    /// `#` — start of an attribute marker (`#[bounded(N)]`,
    /// etc.). Closure #286.
    Hash,
    /// `?` — postfix Result/Option propagation operator. Sugar
    /// over the `try` keyword: `EXPR?` desugars to `try EXPR` at
    /// parse time, so all of `try`'s narrow-gate restrictions
    /// (Option-like enum, first-let-RHS position, payload-less
    /// short-circuit) apply unchanged. Surface affordance for
    /// users who expect Rust's `?`. Arc 8 v3.1 sugar.
    Question,
    Arrow,
    /// `module name { ... }` — namespace declaration (closure
    /// #242). vāṇī uses Rust-style modules: explicit paths
    /// with `::` separator, `pub` for export, private-by-default
    /// inside the module. Top-level items stay globally visible
    /// for back-compat.
    Module,
    /// `pub` modifier: makes an item visible from outside its
    /// module. Default visibility for module-scoped items is
    /// private. Top-level items (not inside any `module`) stay
    /// globally visible.
    Pub,
    /// `region <name> { ... }` — opens a lexical region block.
    /// Sugar for `{ let <name>: Region = region_new(); <body>; }`.
    /// The bump-allocator arena is initialized at block entry
    /// and freed at block exit via the existing scope-exit drop.
    /// Layer 5 of the embedded-vāṇी unsafe plan (`unsafe.md`).
    RegionKw,
    /// `unsafe(reason = "...") { ... }` — opens a lexically
    /// scoped block where raw-pointer / FFI primitives that
    /// the affine + Z3 surface can't verify are permitted. The
    /// `reason = "..."` clause is mandatory at parse time; the
    /// reason string is threaded through the IR and emitted as
    /// machine-readable debug metadata so certification tooling
    /// (ASIL-D / DO-178C / IEC 62304) can extract deviation
    /// records from the compiled artifact. Embedded build
    /// triples only — hosted builds reject this keyword at
    /// parse time. Layer 1.1 of the embedded-vāṇी unsafe plan
    /// (`unsafe.md`).
    Unsafe,
    Eof,
}

/// Resolve a Devanagari keyword alias to its English-equivalent
/// `TokenKind`. Returns `None` for any non-alias string, which the
/// caller treats as a regular Unicode identifier name.
///
/// V1 ships a small first cut covering the most common control-flow
/// and verification keywords across Sanskrit / Hindi / Marathi.
/// Conflicts where the same Devanagari word would map to two
/// different English keywords are resolved in favor of the most
/// idiomatic single-word form; multi-word aliases (e.g. `के लिए`
/// for `for`, `नहीं तो` for `else`) are deferred until the lexer
/// gains lookahead over whitespace.
///
/// The table is intentionally conservative — finalized aliases per
/// language will land with grammar consultant review per Roadmap
/// item #9.
fn devanagari_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        // fn
        "फलन" => TokenKind::Fn,           // phalan (Hindi/Marathi: "function")
        "कार्य" => TokenKind::Fn,         // kārya (Sanskrit/Marathi: "function/work")
        // let
        "मान" => TokenKind::Let,          // māna (Marathi: "assume/let")
        "माना" => TokenKind::Let,         // mānā (Sanskrit/Hindi)
        // return
        "परत" => TokenKind::Return,       // parat (Marathi: "back")
        "लौटाओ" => TokenKind::Return,     // lauṭāo (Hindi: "return!")
        "पुनरागम" => TokenKind::Return,   // punarāgama (Sanskrit)
        // if / else
        "यदि" => TokenKind::If,           // yadi (Sanskrit/Hindi: "if")
        "अगर" => TokenKind::If,           // agar (Hindi: "if")
        "जर" => TokenKind::If,            // jar (Marathi: "if")
        "अन्यथा" => TokenKind::Else,      // anyathā (Sanskrit: "else")
        "वरना" => TokenKind::Else,         // varnā (Hindi: "otherwise") — closure #267
        "नाहीतर" => TokenKind::Else,      // nāhītar (Marathi: "else")
        // while
        "यावत्" => TokenKind::While,      // yāvat (Sanskrit: "while/until")
        "जबतक" => TokenKind::While,       // jab tak (Hindi: "until")
        "जोपर्यंत" => TokenKind::While,   // jopa­ryanta (Marathi: "until")
        // for
        "प्रति" => TokenKind::For,        // prati (Sanskrit: "for each")
        "साठी" => TokenKind::For,         // sāṭhī (Marathi: "for")
        // match arm "then"
        "तदा" => TokenKind::Then,         // tadā (Sanskrit: "then")
        "तो" => TokenKind::Then,          // to (Hindi: "then")
        "तर" => TokenKind::Then,          // tar (Marathi: "then")
        // ref
        "पहा" => TokenKind::Ref,          // pahā (Marathi: "see/look")
        "देखो" => TokenKind::Ref,         // dekho (Hindi: "see!")
        "दृष्ट्या" => TokenKind::Ref,     // dṛṣṭyā (Sanskrit: instrumental "via sight / by reference") — SOV-S10 add 2026-06-06
        // mut — closure #267 fills Sanskrit + Hindi gaps. Marathi
        // गzaps closed 2026-06-07 (linguistic audit): बदल alone is
        // the noun "change"; the Marathi adjective for "mutable" is
        // बदलणारा (changing) or the formal बदलण्यायोग्य.
        "बदल" => TokenKind::Mut,          // badla (Marathi: "change" — informal/noun)
        "बदलणारा" => TokenKind::Mut,      // badalṇārā (Marathi: "changing/mutable")
        "परिवर्तनीय" => TokenKind::Mut,   // parivartanīya (Sanskrit/Hindi/Marathi tatsama: "mutable")
        // match — मेल is the colloquial Sanskrit/Hindi form;
        // मेलन (melana, "joining/matching") is the classical
        // Sanskrit deverbal noun. Added 2026-06-07.
        "जुळवा" => TokenKind::Match,      // juḷvā (Marathi: "match")
        "मिलान" => TokenKind::Match,      // milān (Hindi: "match")
        "मेल" => TokenKind::Match,        // mela (Sanskrit: "join/match")
        "मेलन" => TokenKind::Match,       // melana (Sanskrit classical: "joining/matching")
        // assert
        "खात्री" => TokenKind::Assert,    // khātrī (Marathi: "certainty")
        "सुनिश्चित" => TokenKind::Assert, // sunishchit (Hindi: "ensured")
        "सिद्धम्" => TokenKind::Assert,   // siddham (Sanskrit)
        // prove — closure #267 fills Hindi + Marathi single-word
        "सिद्ध" => TokenKind::Prove,      // siddha (Sanskrit root)
        "प्रमाण" => TokenKind::Prove,     // pramāṇa (Sanskrit: "proof")
        "प्रमाणित" => TokenKind::Prove,   // pramāṇita (Hindi/Marathi: "proven")
        "दर्शाओ" => TokenKind::Prove,     // darśāo (Hindi imperative: "show!")
        "दाखवा" => TokenKind::Prove,      // dākhvā (Marathi imperative: "show!")
        // requires / ensures
        "अपेक्षित" => TokenKind::Requires, // apekṣita (Sanskrit: "required")
        "चाहिए" => TokenKind::Requires,    // cāhiye (Hindi: "needs")
        "पाहिजे" => TokenKind::Requires,   // pāhije (Marathi: "needs")
        // ensures — `निश्चित` shared Hindi/Marathi; add a Sanskrit
        // alternate. Closure #267.
        "निश्चित" => TokenKind::Ensures,   // nishchit (Hindi/Marathi: "definite")
        "सुनिश्चयित" => TokenKind::Ensures, // sunischayita (Sanskrit: "ensured")
        // bool literals — `सत्य/असत्य` are tatsama (Sanskrit
        // loanwords) widely used in all three languages.
        //
        // Linguistic audit 2026-06-07: `सही` means "signature"
        // in Marathi (a noun), not "correct" — Hindi-only now.
        // `अशुद्ध` strictly means "impure" in Marathi; natural
        // Marathi for "false" is खोटे (khote, "false/lie") or
        // चूक (chūk, "mistake"). Natural Hindi for "true"/"false"
        // is सच (sach) / झूठ (jhūṭh).
        "सत्य" => TokenKind::True,         // satya (Sanskrit, shared tatsama)
        "सही" => TokenKind::True,          // sahī (Hindi colloquial: "correct")
        "सच" => TokenKind::True,           // sach (Hindi natural: "truth")
        "बरोबर" => TokenKind::True,        // barobar (Marathi natural: "correct/right")
        "खरे" => TokenKind::True,          // khare (Marathi natural: "true")
        "असत्य" => TokenKind::False,       // asatya (Sanskrit, shared tatsama)
        "अशुद्ध" => TokenKind::False,      // aśuddha (Hindi: "incorrect")
        "झूठ" => TokenKind::False,         // jhūṭh (Hindi natural: "false/lie")
        "गलत" => TokenKind::False,         // galat (Hindi natural: "wrong/incorrect")
        "खोटे" => TokenKind::False,        // khoṭe (Marathi natural: "false/lie")
        "चूक" => TokenKind::False,         // chūk (Marathi natural: "mistake/wrong")
        // print / write — `लिख` (likh, root for "write") +
        // imperative `लिखो` (likho, "write!"). `छाप` (chāp,
        // "imprint/stamp") was the previous spelling but
        // feels off for screen output; removed in favor of
        // the natural "write" verb across all three
        // Devanagari-script languages.
        "लिख" => TokenKind::Print,         // likh (Sanskrit root: "write")
        "लिखो" => TokenKind::Print,        // likho (Hindi imperative: "write!")
        // Marathi conjugates the "write" verb from a different
        // root — लिह्- (lih-), not लिख्- (likh-). The natural
        // imperatives in Marathi are लिहा (lihā) / लिही (lihī)
        // / लिहिया (lihiyā). Native-speaker correction
        // 2026-06-07.
        "लिहा" => TokenKind::Print,        // lihā (Marathi formal imperative)
        "लिही" => TokenKind::Print,        // lihī (Marathi informal singular)
        "लिहिया" => TokenKind::Print,      // lihiyā (Marathi imperative variant)
        // pure — `शुद्ध` is tatsama, shared across all three.
        "शुद्ध" => TokenKind::Pure,        // śuddha (Sanskrit/Hindi/Marathi: "pure")
        // struct / enum — closure #267 fills gaps. `संरचना`
        // is tatsama and works in Marathi too.
        "संरचना" => TokenKind::Struct,     // saṁracanā (Sanskrit/Hindi/Marathi: "structure")
        "विकल्प" => TokenKind::Enum,       // vikalpa (Sanskrit: "option/alternative")
        "गणन" => TokenKind::Enum,          // gaṇan (Hindi/Marathi: "enumeration")
        // const
        "स्थिर" => TokenKind::Const,       // sthira (Sanskrit/Hindi/Marathi: "fixed/constant")
        "नियत" => TokenKind::Const,        // niyat (Hindi/Marathi: "fixed/determined")
        // break / continue
        "विराम" => TokenKind::Break,       // virāma (Sanskrit: "pause/stop")
        "रुको" => TokenKind::Break,        // ruko (Hindi: "stop")
        "थांब" => TokenKind::Break,        // thāmba (Marathi: "stop")
        "अग्रे" => TokenKind::Continue,    // agre (Sanskrit: "forward") — closure #267
        "पुढे" => TokenKind::Continue,     // puḍhe (Marathi: "ahead/onward")
        "आगे" => TokenKind::Continue,      // āge (Hindi: "ahead")
        // for-loop range words
        "में" => TokenKind::In,             // meṁ (Hindi: "in")
        "से" => TokenKind::From,           // se (Hindi: "from")
        "तक" => TokenKind::To,             // tak (Hindi: "to/until")
        // reduce / with for `parallel for X reduce Y with op` —
        // `संक्षेप` / `सह` are tatsama and work in all three.
        "संक्षेप" => TokenKind::Reduce,    // saṁkṣepa (Sanskrit/Hindi/Marathi: "reduction")
        "सह" => TokenKind::With,           // saha (Sanskrit/Hindi/Marathi: "with")
        // parallel — closure #267 adds a single-word alias
        // (the existing multi-word `समान्तर प्रति` stays for
        // back-compat with Sanskrit-style writing).
        "समानांतर" => TokenKind::Parallel, // samānāntara (Sanskrit/Hindi/Marathi: "parallel")
        // Closure #267: namespace + concurrency keywords now
        // have Devanagari aliases. These are technical terms
        // with no single natural translation in any of the
        // three languages; we pick a Sanskrit-root form that
        // works as tatsama (loanword) in Hindi and Marathi too.
        // `kosh` (कोश, "treasure/repository") is already vāṇी's
        // name for the crate concept — aliased at the parser
        // level via `pub(kosh)` syntax, not at the lexer.
        //
        // use / module / pub / as — namespace imports
        "उपयोग" => TokenKind::Use,         // upayog (Sanskrit/Hindi/Marathi: "use")
        "खण्ड" => TokenKind::Module,       // khaṇḍa (Sanskrit/Hindi/Marathi: "section/module")
        "मॉड्यूल" => TokenKind::Module,    // mōḍyūla (Hindi/Marathi loanword: "module")
        "सार्वजनिक" => TokenKind::Pub,     // sārvajanik (Sanskrit/Hindi/Marathi: "public")
        "यथा" => TokenKind::As,            // yathā (Sanskrit/Hindi/Marathi: "as/like")
        // interface / implement / methods
        "संकेत" => TokenKind::Interface,   // saṅket (Sanskrit/Hindi/Marathi: "protocol/sign")
        "अंतरापृष्ठ" => TokenKind::Interface, // antarāpṛṣṭha (Sanskrit literal: "inter-face")
        "कार्यान्वित" => TokenKind::Implement, // kāryānvit (Sanskrit/Hindi/Marathi: "to put into effect")
        "विधि" => TokenKind::Methods,       // vidhi (Sanskrit/Hindi/Marathi: "method/procedure")
        // where / is — for generic bounds (`where T is Trait`)
        "जहाँ" => TokenKind::Where,        // jahām̐ (Hindi: "where")
        "यत्र" => TokenKind::Where,        // yatra (Sanskrit: "where")
        "जिथे" => TokenKind::Where,        // jithe (Marathi: "where")
        "है" => TokenKind::Is,             // hai (Hindi: "is")
        "अस्ति" => TokenKind::Is,          // asti (Sanskrit: "is")
        "आहे" => TokenKind::Is,            // āhe (Marathi: "is")
        // try (Rust `?` operator analog) / task / join
        "प्रयास" => TokenKind::Try,        // prayās (Sanskrit/Hindi/Marathi: "attempt")
        "नियोग" => TokenKind::Task,        // niyog (Sanskrit/Hindi/Marathi: "assignment/task")
        "संयोजन" => TokenKind::Join,       // saṁyojan (Sanskrit/Hindi/Marathi: "joining")
        // unsafe — tatsama Sanskrit-root form, shared across all
        // three languages. Layer 1.1 of the embedded plan.
        "असुरक्षित" => TokenKind::Unsafe,  // asurakṣita (Sanskrit/Hindi/Marathi: "unprotected")
        // region — `kṣetra` is tatsama Sanskrit, works as a
        // loanword in Hindi/Marathi. Layer 5 of the embedded
        // plan.
        "क्षेत्र" => TokenKind::RegionKw,  // kṣetra (Sanskrit/Hindi/Marathi: "region/area")
        // SOV-S7 (2026-06-06): close the four English-only gaps.
        // All four are Sanskrit-root tatsama forms — work as
        // loanwords in Hindi + Marathi too.
        "उद्देश्य" => TokenKind::Intent,    // uddeśya (Sanskrit/Hindi/Marathi: "goal/intent")
        "प्रकार" => TokenKind::Type,        // prakāra (Sanskrit/Hindi/Marathi: "type/kind")
        "बाह्य" => TokenKind::Extern,       // bāhya (Sanskrit/Hindi/Marathi: "external")
        "अपरिवर्तनीय" => TokenKind::Invariant, // aparivartanīya (Sanskrit/Hindi/Marathi: "unchanging")
        // === Devanagari type-name aliases (2026-06-06) ===
        // Type names stay outside the structure-purity gate per
        // src/lexer.rs:is_structure_keyword_kind, so these are
        // freely mixable in any dialect (or with English type
        // names). All are Sanskrit-root tatsama working as
        // loanwords in Hindi + Marathi.
        "पूर्णांक" => TokenKind::I64,       // pūrṇāṅka (Sanskrit/Hindi/Marathi: "integer")
        "पूर्णांक८" => TokenKind::I8,       // pūrṇāṅka-8
        "पूर्णांक१६" => TokenKind::I16,
        "पूर्णांक३२" => TokenKind::I32,
        "पूर्णांक६४" => TokenKind::I64,
        "अहस्ताक्षरित८" => TokenKind::U8,   // ahastākṣarita-8 (unsigned-8)
        "अहस्ताक्षरित१६" => TokenKind::U16,
        "अहस्ताक्षरित३२" => TokenKind::U32,
        "अहस्ताक्षरित६४" => TokenKind::U64,
        "दशांश" => TokenKind::F64,         // daśāṁśa (Sanskrit/Hindi/Marathi: "decimal/fractional")
        "दशांश३२" => TokenKind::F32,
        "दशांश६४" => TokenKind::F64,
        "तर्क" => TokenKind::Bool,         // tarka (Sanskrit/Hindi/Marathi: "logic/reasoning")
        "बूल" => TokenKind::Bool,          // būla (transliterated "bool" — short common form)
        "सूची" => TokenKind::Vec,          // sūcī (Sanskrit/Hindi/Marathi: "list")
        _ => return None,
    };
    Some(kind)
}

/// Phase 5b (2026-06-07): Bengali keyword resolution. Bengali
/// (বাংলা) is an Indo-Aryan language written in the Bengali
/// Brahmi-derived script (U+0980..U+09FF) — distinct from
/// Devanagari. Tatsama (Sanskrit-derived) vocabulary dominates
/// the technical-keyword set so most spellings here are
/// transliterations of the same Sanskrit roots used by
/// Devanagari aliases above. v1 ships a starter set; native
/// alternatives can be layered as user requests come in.
fn bengali_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        // === DECLARATIONS ===
        "ফাংশন" => TokenKind::Fn,             // phangshon (function — loanword)
        "কাজ" => TokenKind::Fn,                // kāj (work; idiomatic alt)
        "মান" => TokenKind::Let,               // mān (Sanskrit root: assume/let)
        "ধরো" => TokenKind::Let,               // dhoro (Bengali colloquial: let)
        "গঠন" => TokenKind::Struct,            // gathan (structure)
        "গণনা" => TokenKind::Enum,             // ganana (enumeration)
        "স্থির" => TokenKind::Const,           // sthir (constant — same Sanskrit root)
        // === VISIBILITY / MODULES ===
        "সর্বজনীন" => TokenKind::Pub,         // sarbajanin (public)
        "খণ্ড" => TokenKind::Module,           // khanda (module — same Sanskrit root)
        "ব্যবহার" => TokenKind::Use,           // byabahar (use)
        "হিসাবে" => TokenKind::As,             // hisabe (as)
        // === CONTROL FLOW ===
        "ফেরত" => TokenKind::Return,           // pherat (back)
        "প্রত্যাবর্তন" => TokenKind::Return,   // pratyabartan (formal return)
        "যদি" => TokenKind::If,                // jadi (if)
        "নাহলে" => TokenKind::Else,            // nahole (otherwise)
        "অন্যথা" => TokenKind::Else,           // anyatha (else; same Sanskrit root)
        "যতক্ষণ" => TokenKind::While,          // jatakshan (as long as)
        "প্রতি" => TokenKind::For,             // prati (for each)
        "মধ্যে" => TokenKind::In,              // madhye (in)
        "থেকে" => TokenKind::From,             // theke (from)
        "পর্যন্ত" => TokenKind::To,            // paryanta (to/until)
        "বিরাম" => TokenKind::Break,           // biram (pause; break)
        "এগিয়ে" => TokenKind::Continue,       // egiye (forward; continue)
        "তবে" => TokenKind::Then,              // tobe (then)
        // === REFERENCES + MUT ===
        "দেখ" => TokenKind::Ref,               // dekh (see; reference)
        "পরিবর্তনীয়" => TokenKind::Mut,       // paribartaniya (mutable)
        // === MATCHING ===
        "মেলে" => TokenKind::Match,            // mele (matches)
        "মিলান" => TokenKind::Match,           // milan (matching)
        // === VERIFICATION ===
        "নিশ্চিত" => TokenKind::Assert,        // nishchit (assured; assert)
        "প্রমাণ" => TokenKind::Prove,          // praman (proof; same Sanskrit root)
        "প্রয়োজনীয়" => TokenKind::Requires,  // proyojaniya (required)
        "সুনিশ্চিত" => TokenKind::Ensures,     // sunishchit (assured outcome)
        // === BOOL / PRINT ===
        // Bengali natural-everyday forms added 2026-06-07 audit:
        // `ঠিক` (thik, "correct/right") + `মিথ্যা` (mithya, "lie")
        // / `ভুল` (bhul, "wrong/mistake") read more naturally than
        // the formal Sanskrit-rooted সত্য/অসত্য in everyday code.
        "সত্য" => TokenKind::True,             // satya (truth — tatsama)
        "ঠিক" => TokenKind::True,              // thik (natural everyday: "correct/right")
        "অসত্য" => TokenKind::False,           // asatya (untruth — tatsama)
        "মিথ্যা" => TokenKind::False,          // mithya (natural: "lie/false")
        "ভুল" => TokenKind::False,             // bhul (natural everyday: "wrong/mistake")
        "লেখ" => TokenKind::Print,             // lekh (write — same Sanskrit root)
        "লিখো" => TokenKind::Print,            // likho (write; alt)
        // === PURITY / PARALLEL ===
        "শুদ্ধ" => TokenKind::Pure,            // shuddha (pure)
        "সমান্তরাল" => TokenKind::Parallel,    // samantaral (parallel)
        // === INTERFACES / METHODS ===
        "সংকেত" => TokenKind::Interface,       // sanket (signal; interface)
        "কার্যান্বিত" => TokenKind::Implement, // karyanvit (implementing)
        "বিধি" => TokenKind::Methods,          // bidhi (method/rule)
        // === BOUNDS ===
        "যেখানে" => TokenKind::Where,          // jekhane (where)
        "হয়" => TokenKind::Is,                 // hay (is)
        // === CONCURRENCY ===
        "চেষ্টা" => TokenKind::Try,            // cheshta (try)
        "নিয়োগ" => TokenKind::Task,            // niyog (task — same Sanskrit root)
        "যোগ" => TokenKind::Join,              // jog (join)
        // === SOV-S7 PARITY ===
        "উদ্দেশ্য" => TokenKind::Intent,       // uddeshya (intent — same Sanskrit root)
        "প্রকার" => TokenKind::Type,           // prakar (type — same Sanskrit root)
        "বাহ্যিক" => TokenKind::Extern,        // bahyik (external)
        "অপরিবর্তনীয়" => TokenKind::Invariant,// aparibartaniya (invariant)
        _ => return None,
    };
    Some(kind)
}

/// Phase 6 (2026-06-07): Tamil keyword resolution. Tamil (தமிழ்)
/// is the largest Dravidian language — distinct linguistic
/// family from Indo-Aryan, so tatsama (Sanskrit-derived)
/// keywords aren't a natural fit. v1 ships a starter set drawn
/// from existing Tamil CS pedagogy and standard transliterations
/// of programming-language concepts. Native-speaker review +
/// alias additions welcome via PR.
fn tamil_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        // === DECLARATIONS ===
        "செயல்பாடு" => TokenKind::Fn,           // seyalpaadu (function)
        "சார்பு" => TokenKind::Fn,                // saarbu (alt function)
        "கொள்" => TokenKind::Let,                  // kol (let/assume)
        "இருக்க" => TokenKind::Let,               // irukka (let it be)
        "கட்டமைப்பு" => TokenKind::Struct,        // kattamaippu (structure)
        "எண்ணுப்பெயர்" => TokenKind::Enum,        // ennuppeyar (enum)
        "மாறா" => TokenKind::Const,                // maaraa (unchanging)
        // === VISIBILITY ===
        "பொது" => TokenKind::Pub,                  // pothu (public)
        "தொகுதி" => TokenKind::Module,            // thoguthi (module)
        "பயன்படுத்து" => TokenKind::Use,          // payanpaduthu (use)
        "ஆக" => TokenKind::As,                     // aaga (as)
        // === CONTROL FLOW ===
        "திருப்பு" => TokenKind::Return,          // thiruppu (return)
        "என்றால்" => TokenKind::If,               // endraal (if)
        "எனில்" => TokenKind::If,                  // enil (if — alt)
        "இல்லாவிட்டால்" => TokenKind::Else,      // illaavittaal (else)
        "வரை" => TokenKind::While,                 // varai (while/until)
        "ஒவ்வொரு" => TokenKind::For,               // ovvoru (each/for)
        "உள்" => TokenKind::In,                    // ul (in)
        "இருந்து" => TokenKind::From,             // irundhu (from)
        "வரைக்கும்" => TokenKind::To,             // varaikkum (to)
        "நிறுத்து" => TokenKind::Break,           // niruthu (stop)
        "தொடர்" => TokenKind::Continue,           // thodar (continue)
        "அப்போது" => TokenKind::Then,              // appothu (then)
        // === REFERENCES + MUT ===
        "பார்" => TokenKind::Ref,                  // paar (see; reference)
        "மாறக்கூடிய" => TokenKind::Mut,           // maarakkooodiya (mutable)
        // === MATCH ===
        "பொருந்து" => TokenKind::Match,           // poruthu (match)
        // === VERIFICATION ===
        "உறுதி" => TokenKind::Assert,              // uruthi (assert)
        "நிரூபி" => TokenKind::Prove,              // niroopi (prove)
        "தேவை" => TokenKind::Requires,             // thaevai (requires)
        "உறுதிப்படுத்து" => TokenKind::Ensures,   // uruthippadutthu (ensures)
        // === BOOL / PRINT ===
        "மெய்" => TokenKind::True,                 // mey (true)
        "பொய்" => TokenKind::False,                // poy (false)
        "எழுது" => TokenKind::Print,               // ezhuthu (write)
        "அச்சிடு" => TokenKind::Print,             // achchidu (print)
        // === INTERFACES ===
        "இடைமுகம்" => TokenKind::Interface,       // idaimukham (interface)
        "செயல்படுத்து" => TokenKind::Implement,   // seyalpaduthuhu (implement)
        // === SOV-S7 parity ===
        "நோக்கம்" => TokenKind::Intent,            // nokkam (intent)
        "வகை" => TokenKind::Type,                  // vagai (type/kind)
        _ => return None,
    };
    Some(kind)
}

/// Phase 6 (2026-06-07): Telugu keyword resolution. Telugu
/// (తెలుగు) is the second-largest Dravidian language. Heavier
/// Sanskrit influence than Tamil so a few keywords keep
/// transliterated tatsama roots; mostly native Telugu vocabulary.
fn telugu_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "ఫంక్షన్" => TokenKind::Fn,                // function (loanword)
        "పని" => TokenKind::Fn,                    // pani (work)
        "అనుకో" => TokenKind::Let,                 // anuko (let/assume)
        "నిర్మాణం" => TokenKind::Struct,           // nirmaanam (structure)
        "గణన" => TokenKind::Enum,                  // ganana (enumeration)
        "స్థిరం" => TokenKind::Const,              // sthiram (constant — tatsama)
        "ప్రజా" => TokenKind::Pub,                 // prajaa (public)
        "మాడ్యూల్" => TokenKind::Module,           // module (loanword)
        "ఉపయోగించు" => TokenKind::Use,             // upayoginchu (use)
        "గా" => TokenKind::As,                     // gaa (as)
        "తిరిగి" => TokenKind::Return,             // thirigi (return)
        "అయితే" => TokenKind::If,                  // ayithe (if)
        "లేకపోతే" => TokenKind::Else,              // lekapote (else)
        "వరకు" => TokenKind::While,                // varaku (while/until)
        "ప్రతి" => TokenKind::For,                 // prathi (for each — tatsama)
        "లో" => TokenKind::In,                     // lo (in)
        "నుండి" => TokenKind::From,                // nundi (from)
        "వరకూ" => TokenKind::To,                   // varakuu (to)
        "ఆపు" => TokenKind::Break,                 // aapu (stop)
        "కొనసాగించు" => TokenKind::Continue,       // konasaaginchu (continue)
        "అప్పుడు" => TokenKind::Then,              // appudu (then)
        "చూడు" => TokenKind::Ref,                  // chudu (see)
        "మార్చదగిన" => TokenKind::Mut,            // maarchadagina (mutable)
        "సరిపోలు" => TokenKind::Match,            // saripolu (match)
        "నిర్ధారించు" => TokenKind::Assert,        // nirdhaarinchu (assert)
        "నిరూపించు" => TokenKind::Prove,           // niroopinchu (prove)
        "అవసరం" => TokenKind::Requires,            // avasaram (requires)
        "నిశ్చయం" => TokenKind::Ensures,           // nishchayam (ensures)
        "నిజం" => TokenKind::True,                 // nijam (true)
        "అబద్ధం" => TokenKind::False,              // abaddham (false)
        "రాయి" => TokenKind::Print,                // raayi (write)
        "ముద్రించు" => TokenKind::Print,           // mudrinchu (print)
        "ఉద్దేశం" => TokenKind::Intent,            // uddesam (intent — tatsama)
        "రకం" => TokenKind::Type,                  // rakam (type/kind)
        _ => return None,
    };
    Some(kind)
}

/// Phase 6 (2026-06-07): Gujarati keyword resolution. Gujarati
/// (ગુજરાતી) is Indo-Aryan, geographically near Marathi, so the
/// tatsama set largely transliterates — but the script is
/// Gujarati's own (U+0A80..U+0AFF), distinct from Devanagari.
fn gujarati_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "કાર્ય" => TokenKind::Fn,                  // kaarya (work/function — tatsama)
        "ફંકશન" => TokenKind::Fn,                  // function (loanword)
        "માનો" => TokenKind::Let,                  // maano (assume)
        "ધારો" => TokenKind::Let,                  // dhaaro (suppose)
        "રચના" => TokenKind::Struct,               // rachana (structure)
        "ગણના" => TokenKind::Enum,                 // ganana (enumeration)
        "સ્થિર" => TokenKind::Const,               // sthir (constant — tatsama)
        "જાહેર" => TokenKind::Pub,                 // jaahaer (public)
        "ખંડ" => TokenKind::Module,                // khand (module — tatsama)
        "વાપરો" => TokenKind::Use,                 // vaapro (use)
        "તરીકે" => TokenKind::As,                  // tareeke (as)
        "પાછા" => TokenKind::Return,               // paachhaa (back)
        "જો" => TokenKind::If,                     // jo (if)
        "નહીંતર" => TokenKind::Else,               // naheentar (else)
        "જ્યારે" => TokenKind::While,             // jyaare (while/when — single-word form)
        "પ્રતિ" => TokenKind::For,                 // prati (for each — tatsama)
        "માં" => TokenKind::In,                    // maan (in)
        "થી" => TokenKind::From,                   // thee (from)
        "સુધી" => TokenKind::To,                   // sudhee (to)
        "વિરામ" => TokenKind::Break,               // viraam (pause/break — tatsama)
        "ચાલુ" => TokenKind::Continue,             // chaaloo (continue)
        "પછી" => TokenKind::Then,                  // pachhee (then)
        "જુઓ" => TokenKind::Ref,                   // juo (see)
        "પરિવર્તનીય" => TokenKind::Mut,            // parivartaneeya (mutable — tatsama)
        "મેળવો" => TokenKind::Match,               // melavo (match)
        "નિશ્ચિત" => TokenKind::Assert,            // nishchit (assert — tatsama)
        "પ્રમાણ" => TokenKind::Prove,              // pramaan (proof — tatsama)
        "જરૂરી" => TokenKind::Requires,            // jaruri (required)
        "ખાતરી" => TokenKind::Ensures,             // khaatari (ensures)
        "સાચું" => TokenKind::True,                // saachun (true)
        "ખોટું" => TokenKind::False,               // khotun (false)
        "લખો" => TokenKind::Print,                 // lakho (write)
        "છાપો" => TokenKind::Print,                // chhaapo (print)
        "ઉદ્દેશ" => TokenKind::Intent,             // uddhesh (intent — tatsama)
        "પ્રકાર" => TokenKind::Type,               // prakaar (type — tatsama)
        "અચળ" => TokenKind::Invariant,             // achal (invariant — "unchanging", tatsama)
        _ => return None,
    };
    Some(kind)
}

/// Phase 6 (2026-06-07): Punjabi-Gurmukhi keyword resolution.
/// Indian Punjabi is written in Gurmukhi (U+0A00..U+0A7F); the
/// language is Indo-Aryan, so tatsama vocabulary transliterates
/// well. Pakistani Punjabi uses Shahmukhi (Perso-Arabic, RTL)
/// which is queued separately for the RTL pass.
fn punjabi_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "ਕਾਰਜ" => TokenKind::Fn,                  // kaaraj (function/task — tatsama)
        "ਫੰਕਸ਼ਨ" => TokenKind::Fn,                  // function (loanword)
        "ਮੰਨੋ" => TokenKind::Let,                  // manno (assume)
        "ਰਚਨਾ" => TokenKind::Struct,               // rachnaa (structure)
        "ਗਣਨਾ" => TokenKind::Enum,                 // gannaa (enumeration)
        "ਸਥਿਰ" => TokenKind::Const,                // sthir (constant — tatsama)
        "ਜਨਤਕ" => TokenKind::Pub,                  // jantak (public)
        "ਖੰਡ" => TokenKind::Module,                // khand (module — tatsama)
        "ਵਰਤੋ" => TokenKind::Use,                  // varto (use)
        "ਵਜੋਂ" => TokenKind::As,                   // vajon (as)
        "ਮੁੜੋ" => TokenKind::Return,               // mudho (return)
        "ਜੇ" => TokenKind::If,                     // je (if)
        "ਨਹੀਂ ਤਾਂ" => TokenKind::Else,             // nahin taan (else — multi-word; TBD)
        "ਜਦੋਂ ਤੱਕ" => TokenKind::While,            // jadon takk (while/until)
        "ਹਰ" => TokenKind::For,                    // har (every/for)
        "ਵਿੱਚ" => TokenKind::In,                   // vich (in)
        "ਤੋਂ" => TokenKind::From,                  // ton (from)
        "ਤੱਕ" => TokenKind::To,                    // takk (to)
        "ਵਿਰਾਮ" => TokenKind::Break,               // viraam (pause — tatsama)
        "ਜਾਰੀ" => TokenKind::Continue,             // jaari (continue)
        "ਤਦ" => TokenKind::Then,                   // tad (then — tatsama)
        "ਵੇਖੋ" => TokenKind::Ref,                  // vekho (see)
        "ਬਦਲਣਯੋਗ" => TokenKind::Mut,              // badlanyogh (mutable)
        "ਮੇਲ" => TokenKind::Match,                 // mel (match)
        "ਨਿਸ਼ਚਿਤ" => TokenKind::Assert,            // nishchit (assert — tatsama)
        "ਪ੍ਰਮਾਣ" => TokenKind::Prove,              // pramaan (proof — tatsama)
        "ਲੋੜੀਂਦਾ" => TokenKind::Requires,         // lorindaa (required)
        "ਯਕੀਨੀ" => TokenKind::Ensures,             // yakeeni (ensures)
        "ਸੱਚ" => TokenKind::True,                  // sach (true)
        "ਝੂਠ" => TokenKind::False,                 // jhuth (false)
        "ਲਿਖੋ" => TokenKind::Print,                // likho (write)
        "ਛਾਪੋ" => TokenKind::Print,                // chhapo (print)
        "ਉਦੇਸ਼" => TokenKind::Intent,              // udesh (intent — tatsama)
        "ਕਿਸਮ" => TokenKind::Type,                 // kism (type)
        _ => return None,
    };
    Some(kind)
}

/// Phase 6 (2026-06-07): Kannada (ಕನ್ನಡ) keyword resolution.
/// Dravidian language with heavy tatsama (Sanskrit-rooted)
/// loanwords in technical vocabulary. v1 starter set.
fn kannada_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "ಕಾರ್ಯ" => TokenKind::Fn,                  // kaarya (work/function — tatsama)
        "ಫಂಕ್ಷನ್" => TokenKind::Fn,                  // function loanword
        "ಊಹಿಸಿ" => TokenKind::Let,                  // oohisi (assume)
        "ಮಾನ್ಯ" => TokenKind::Let,                  // maanya (let it be)
        "ರಚನೆ" => TokenKind::Struct,                // rachane (structure)
        "ಎಣಿಕೆ" => TokenKind::Enum,                  // enike (enumeration)
        "ಸ್ಥಿರ" => TokenKind::Const,                 // sthira (constant — tatsama)
        "ಸಾರ್ವಜನಿಕ" => TokenKind::Pub,             // saarvajanika (public — tatsama)
        "ಖಂಡ" => TokenKind::Module,                  // khanda (module — tatsama)
        "ಬಳಸಿ" => TokenKind::Use,                    // balasi (use)
        "ಆಗಿ" => TokenKind::As,                      // aagi (as)
        "ಹಿಂದಿರುಗಿ" => TokenKind::Return,           // hindirugi (return)
        "ಮರಳಿ" => TokenKind::Return,                  // marali (return back)
        "ಆದರೆ" => TokenKind::If,                     // aadare (if)
        "ಇಲ್ಲದಿದ್ದರೆ" => TokenKind::Else,           // illadiddare (else)
        "ತನಕ" => TokenKind::While,                   // tanaka (until/while)
        "ಪ್ರತಿ" => TokenKind::For,                   // prati (for each — tatsama)
        "ರಲ್ಲಿ" => TokenKind::In,                    // ralli (in)
        "ಇಂದ" => TokenKind::From,                    // inda (from)
        "ಗೆ" => TokenKind::To,                       // ge (to)
        "ನಿಲ್ಲಿ" => TokenKind::Break,                // nilli (stop)
        "ಮುಂದುವರಿಸಿ" => TokenKind::Continue,        // munduvarisi (continue)
        "ನಂತರ" => TokenKind::Then,                   // nantara (then)
        "ನೋಡಿ" => TokenKind::Ref,                    // nodi (see)
        "ಪರಿವರ್ತನೀಯ" => TokenKind::Mut,             // parivartaneeya (mutable — tatsama)
        "ಹೊಂದಾಣಿಕೆ" => TokenKind::Match,            // hondaanike (matching)
        "ಖಚಿತಪಡಿಸಿ" => TokenKind::Assert,           // khachitapadisi (assert)
        "ಸಾಬೀತುಪಡಿಸಿ" => TokenKind::Prove,          // saabeetupadisi (prove)
        "ಅಗತ್ಯ" => TokenKind::Requires,              // agatya (required)
        "ಖಚಿತ" => TokenKind::Ensures,                // khachita (assured)
        "ಸತ್ಯ" => TokenKind::True,                   // satya (true — tatsama)
        "ಸರಿ" => TokenKind::True,                    // sari (natural everyday: "correct/right")
        "ಸುಳ್ಳು" => TokenKind::False,                // sullu (false — natural)
        "ತಪ್ಪು" => TokenKind::False,                 // tappu (natural everyday: "wrong/mistake")
        "ಬರೆ" => TokenKind::Print,                   // bare (write)
        "ಮುದ್ರಿಸಿ" => TokenKind::Print,             // mudrisi (print — tatsama)
        "ಉದ್ದೇಶ" => TokenKind::Intent,               // uddesha (intent — tatsama)
        "ಪ್ರಕಾರ" => TokenKind::Type,                 // prakaara (type — tatsama)
        _ => return None,
    };
    Some(kind)
}

/// Phase 6 (2026-06-07): Malayalam (മലയാളം) keyword resolution.
/// Dravidian language; heavy Sanskrit borrowings in formal
/// register. v1 starter set.
fn malayalam_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "കാര്യം" => TokenKind::Fn,                  // kaaryam (work/function — tatsama)
        "ഫംഗ്ഷൻ" => TokenKind::Fn,                   // function loanword
        "കരുതുക" => TokenKind::Let,                  // karuthuka (assume)
        "ഘടന" => TokenKind::Struct,                  // ghadana (structure)
        "എണ്ണൽ" => TokenKind::Enum,                  // ennal (enumeration)
        "സ്ഥിരം" => TokenKind::Const,                // sthiram (constant — tatsama)
        "പൊതു" => TokenKind::Pub,                    // pothu (public)
        "ഖണ്ഡം" => TokenKind::Module,                // khandam (module — tatsama)
        "ഉപയോഗിക്കുക" => TokenKind::Use,            // upayogikkuka (use)
        "ആയി" => TokenKind::As,                      // aayi (as)
        "തിരികെ" => TokenKind::Return,                // thirike (return)
        "എങ്കിൽ" => TokenKind::If,                   // enkil (if)
        "അല്ലെങ്കിൽ" => TokenKind::Else,            // allenkil (else)
        "വരെ" => TokenKind::While,                   // vare (until/while)
        "ഓരോ" => TokenKind::For,                     // oro (each/for)
        "ഇൽ" => TokenKind::In,                       // il (in)
        "നിന്ന്" => TokenKind::From,                  // ninnu (from)
        "വരെക്കും" => TokenKind::To,                // varekkum (to)
        "നിർത്തുക" => TokenKind::Break,             // nirthuka (stop)
        "തുടരുക" => TokenKind::Continue,            // thudaruka (continue)
        "പിന്നെ" => TokenKind::Then,                  // pinne (then)
        "നോക്കുക" => TokenKind::Ref,                 // nookkuka (see)
        "മാറ്റാവുന്ന" => TokenKind::Mut,            // maattaavunna (mutable)
        "പൊരുത്തപ്പെടുത്തുക" => TokenKind::Match,   // poruthappeduthuka (match)
        "ഉറപ്പിക്കുക" => TokenKind::Assert,         // urappikkuka (assure)
        "തെളിയിക്കുക" => TokenKind::Prove,          // theliyikkuka (prove)
        "ആവശ്യം" => TokenKind::Requires,             // aavasyam (required — tatsama)
        "ഉറപ്പ്" => TokenKind::Ensures,              // urappu (assurance)
        "സത്യം" => TokenKind::True,                  // sathyam (true — tatsama)
        "ശരി" => TokenKind::True,                    // shari (natural everyday: "correct/right")
        "അസത്യം" => TokenKind::False,                // asathyam (false — tatsama)
        "തെറ്റ്" => TokenKind::False,                // thettu (natural everyday: "wrong/mistake")
        "എഴുതുക" => TokenKind::Print,                // ezhuthuka (write)
        "അച്ചടിക്കുക" => TokenKind::Print,          // achchadikuka (print)
        "ഉദ്ദേശ്യം" => TokenKind::Intent,            // uddeshyam (intent — tatsama)
        "തരം" => TokenKind::Type,                    // tharam (type)
        _ => return None,
    };
    Some(kind)
}

/// Phase 6 (2026-06-07): Odia (ଓଡ଼ିଆ) keyword resolution.
/// Indo-Aryan SOV; tatsama-friendly so most technical
/// vocabulary transliterates from Sanskrit roots.
fn odia_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "କାର୍ଯ୍ୟ" => TokenKind::Fn,                 // kaarya (work — tatsama)
        "ଫଙ୍କସନ୍" => TokenKind::Fn,                  // function loanword
        "ମନେକର" => TokenKind::Let,                   // manekara (assume)
        "ଗଠନ" => TokenKind::Struct,                   // gathana (structure)
        "ଗଣନା" => TokenKind::Enum,                    // gananaa (enumeration)
        "ସ୍ଥିର" => TokenKind::Const,                 // sthira (constant — tatsama)
        "ସର୍ବସାଧାରଣ" => TokenKind::Pub,             // sarbasaadhaarana (public)
        "ଖଣ୍ଡ" => TokenKind::Module,                  // khanda (module — tatsama)
        "ବ୍ୟବହାର" => TokenKind::Use,                 // byabahaara (use)
        "ଭାବେ" => TokenKind::As,                      // bhabe (as)
        "ଫେରନ୍ତୁ" => TokenKind::Return,              // pherantu (return)
        "ଯଦି" => TokenKind::If,                      // jadi (if)
        "ନ ହେଲେ" => TokenKind::Else,                 // na hele (else — multi-word; queued)
        "ଯେତେବେଳେ" => TokenKind::While,             // jetebele (while)
        "ପ୍ରତି" => TokenKind::For,                   // prati (for each — tatsama)
        "ରେ" => TokenKind::In,                       // re (in)
        "ରୁ" => TokenKind::From,                     // ru (from)
        "ପର୍ଯ୍ୟନ୍ତ" => TokenKind::To,                // paryanta (until)
        "ବନ୍ଦ" => TokenKind::Break,                  // banda (stop)
        "ଜାରି" => TokenKind::Continue,               // jaari (continue)
        "ତାହେଲେ" => TokenKind::Then,                  // tahele (then)
        "ଦେଖନ୍ତୁ" => TokenKind::Ref,                  // dekhantu (see)
        "ପରିବର୍ତ୍ତନୀୟ" => TokenKind::Mut,           // paribartaniya (mutable — tatsama)
        "ମେଳ" => TokenKind::Match,                    // mela (match)
        "ନିଶ୍ଚିତ" => TokenKind::Assert,              // nishchita (assured — tatsama)
        "ପ୍ରମାଣ" => TokenKind::Prove,                // pramaana (proof — tatsama)
        "ଆବଶ୍ୟକ" => TokenKind::Requires,             // aabashyaka (required)
        "ସୁନିଶ୍ଚିତ" => TokenKind::Ensures,           // sunischita (assured)
        "ସତ୍ୟ" => TokenKind::True,                    // satya (true — tatsama)
        "ମିଥ୍ୟା" => TokenKind::False,                 // mithya (false — tatsama)
        "ଲେଖ" => TokenKind::Print,                    // lekha (write)
        "ଛାପନ୍ତୁ" => TokenKind::Print,                // chhapantu (print)
        "ଉଦ୍ଦେଶ୍ୟ" => TokenKind::Intent,              // uddeshya (intent — tatsama)
        "ପ୍ରକାର" => TokenKind::Type,                  // prakaara (type — tatsama)
        _ => return None,
    };
    Some(kind)
}

/// Phase 6 (2026-06-07): Sinhala (සිංහල) keyword resolution.
/// Indo-Aryan, Sri Lankan. Heavy Sanskrit influence in formal
/// register; native Sinhala for everyday verbs. v1 starter set.
fn sinhala_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "කාර්යය" => TokenKind::Fn,                  // kaaryaya (work — tatsama)
        "ශ්‍රිතය" => TokenKind::Fn,                  // shritaya (function)
        "අනුමානය" => TokenKind::Let,                 // anumaanaya (assume)
        "ව්‍යුහය" => TokenKind::Struct,              // vyuhaya (structure)
        "ගණනය" => TokenKind::Enum,                   // ganannaya (enumeration)
        "ස්ථිර" => TokenKind::Const,                 // sthira (constant — tatsama)
        "පොදු" => TokenKind::Pub,                    // podu (public/common)
        "මොඩියුලය" => TokenKind::Module,             // module loanword
        "භාවිතා" => TokenKind::Use,                  // bhaavithaa (use)
        "ලෙස" => TokenKind::As,                      // lesa (as)
        "ආපසු" => TokenKind::Return,                  // aapasu (return back)
        "නම්" => TokenKind::If,                       // nam (if)
        "නොඑසේ නම්" => TokenKind::Else,              // no-ese nam (else — multi-word; queued)
        "තෙක්" => TokenKind::While,                  // thek (while/until)
        "සෑම" => TokenKind::For,                     // saema (every/for)
        "තුළ" => TokenKind::In,                      // thula (in)
        "සිට" => TokenKind::From,                     // sita (from)
        "දක්වා" => TokenKind::To,                     // dakvaa (to)
        "නවත්වන්න" => TokenKind::Break,              // nawathwanna (stop)
        "ඉදිරියට" => TokenKind::Continue,             // idiriyata (forward)
        "පසු" => TokenKind::Then,                     // pasu (then)
        "බලන්න" => TokenKind::Ref,                   // balanna (see/look)
        "පරිවර්තනීය" => TokenKind::Mut,              // parivarthaneeya (mutable — tatsama)
        "ගැලපීම" => TokenKind::Match,                // gaalapeema (matching)
        "තහවුරු" => TokenKind::Assert,                // thahawuru (assert)
        "ඔප්පු" => TokenKind::Prove,                 // oppu (prove)
        "අවශ්‍ය" => TokenKind::Requires,             // avashya (required — tatsama)
        "සහතික" => TokenKind::Ensures,                // sahathika (assured)
        "සත්‍ය" => TokenKind::True,                   // sathya (true — tatsama)
        "හරි" => TokenKind::True,                    // hari (natural everyday: "correct/right")
        "අසත්‍ය" => TokenKind::False,                 // asathya (false — tatsama)
        "වැරදි" => TokenKind::False,                  // varadi (natural everyday: "wrong/mistake")
        "ලියන්න" => TokenKind::Print,                 // liyanna (write)
        "මුද්‍රණය" => TokenKind::Print,               // mudranaya (print — tatsama)
        "අරමුණ" => TokenKind::Intent,                 // aramuna (intent/purpose)
        "වර්ගය" => TokenKind::Type,                   // vargaya (type)
        _ => return None,
    };
    Some(kind)
}

/// Phase 12 (2026-06-07): Urdu (اردو) keyword resolution.
/// Indo-Aryan with Hindustani vocabulary at the spoken level;
/// the surface forks into Hindi (Devanagari) vs Urdu (Perso-
/// Arabic) at the script + register layer. Many Urdu terms
/// have Persian/Arabic roots in the technical register while
/// the conversational register tracks Hindi closely.
fn urdu_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        // === DECLARATIONS ===
        "فنکشن" => TokenKind::Fn,             // function (loanword)
        "کام" => TokenKind::Fn,                // kaam (work)
        "مانیں" => TokenKind::Let,             // maanen (assume — formal)
        "فرض" => TokenKind::Let,               // farz (suppose)
        "ساخت" => TokenKind::Struct,           // saakht (structure)
        "شمار" => TokenKind::Enum,             // shumaar (enumeration)
        "ثابت" => TokenKind::Const,            // saabit (constant)
        // === VISIBILITY / MODULES ===
        "عوامی" => TokenKind::Pub,             // awaami (public)
        "ماڈیول" => TokenKind::Module,         // module (loanword)
        "حصہ" => TokenKind::Module,            // hissa (part — alt)
        "استعمال" => TokenKind::Use,           // istemaal (use)
        "بطور" => TokenKind::As,               // bataur (as)
        // === CONTROL FLOW ===
        "واپس" => TokenKind::Return,           // vapas (back)
        "لوٹاؤ" => TokenKind::Return,          // lautao (return — Hindi-shared)
        "اگر" => TokenKind::If,                // agar (if)
        "ورنہ" => TokenKind::Else,             // varna (otherwise)
        "ہر" => TokenKind::For,                // har (every/for)
        "میں" => TokenKind::In,                // mein (in)
        "سے" => TokenKind::From,               // se (from)
        "تک" => TokenKind::To,                 // tak (to)
        "بند" => TokenKind::Break,             // band (closed/stop)
        "جاری" => TokenKind::Continue,         // jaari (continue)
        "تب" => TokenKind::Then,               // tab (then)
        // === REFERENCES + MUT ===
        "دیکھیں" => TokenKind::Ref,            // dekhen (see)
        "بدلنا" => TokenKind::Mut,             // badalna (changing/mutable)
        // === MATCHING ===
        "ملان" => TokenKind::Match,            // milaan (match)
        "مماثلت" => TokenKind::Match,          // mumasilat (matching — alt)
        // === VERIFICATION ===
        "یقینی" => TokenKind::Assert,          // yaqeeni (assured)
        "ثبوت" => TokenKind::Prove,            // saboot (proof)
        "درکار" => TokenKind::Requires,        // darkaar (required)
        "ضمانت" => TokenKind::Ensures,         // zamaanat (guarantee)
        // === BOOL / PRINT ===
        "سچ" => TokenKind::True,               // sach (truth)
        "جھوٹ" => TokenKind::False,            // jhoot (lie)
        "لکھو" => TokenKind::Print,            // likho (write)
        "چھاپو" => TokenKind::Print,           // chhaapo (print)
        // === INTERFACES ===
        "رابطہ" => TokenKind::Interface,       // raabta (interface)
        "نافذ" => TokenKind::Implement,        // naafiz (implementing)
        // === SOV-S7 PARITY ===
        "مقصد" => TokenKind::Intent,           // maqsad (intent)
        "قسم" => TokenKind::Type,              // kism (type)
        "بیرونی" => TokenKind::Extern,         // bairooni (external)
        _ => return None,
    };
    Some(kind)
}

/// Phase 12.4 (2026-06-07): Persian / Farsi (فارسی) keyword
/// resolution. Iranian language; shares Perso-Arabic script
/// with Urdu but uses distinct everyday vocabulary. Technical
/// register heavy on classical Persian roots; we ship a v1
/// starter set covering the highest-frequency structure
/// keywords.
fn persian_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        // === DECLARATIONS ===
        "تابع" => TokenKind::Fn,               // tābe' (function)
        "فانکشن" => TokenKind::Fn,             // fankshen (loanword)
        "فرض" => TokenKind::Let,               // farz (suppose)
        "بگذار" => TokenKind::Let,             // begozār (let it be)
        "ساختار" => TokenKind::Struct,         // sākhtār (structure)
        "شمارش" => TokenKind::Enum,            // shomāresh (enumeration)
        "ثابت" => TokenKind::Const,            // sābet (constant)
        // === VISIBILITY ===
        "عمومی" => TokenKind::Pub,             // omumī (public)
        "بخش" => TokenKind::Module,            // bakhsh (section)
        "استفاده" => TokenKind::Use,           // estefādeh (use)
        "بعنوان" => TokenKind::As,             // be-onvān (as)
        // === CONTROL FLOW ===
        "بازگشت" => TokenKind::Return,         // bāzgasht (return)
        "اگر" => TokenKind::If,                // agar (if)
        "وگرنه" => TokenKind::Else,            // vagarna (otherwise)
        "تا" => TokenKind::While,              // tā (until/while)
        "هر" => TokenKind::For,                // har (each/for)
        "در" => TokenKind::In,                 // dar (in)
        "از" => TokenKind::From,               // az (from)
        "بشکن" => TokenKind::Break,            // beshkan (break)
        "ادامه" => TokenKind::Continue,        // edāmeh (continue)
        "سپس" => TokenKind::Then,              // sepas (then)
        // === REFERENCES + MUT ===
        "ببین" => TokenKind::Ref,              // bebin (see/look)
        "تغییرپذیر" => TokenKind::Mut,         // taghyīr-pazīr (mutable)
        // === MATCH ===
        "تطبیق" => TokenKind::Match,           // tatbīq (matching)
        // === VERIFICATION ===
        "ادعا" => TokenKind::Assert,           // ed'ā (claim/assert)
        "اثبات" => TokenKind::Prove,           // esbāt (proof)
        "نیاز" => TokenKind::Requires,         // niāz (need)
        "تضمین" => TokenKind::Ensures,         // tazmīn (guarantee)
        // === BOOL / PRINT ===
        "درست" => TokenKind::True,             // dorost (correct/true)
        "نادرست" => TokenKind::False,          // nādorost (false)
        "چاپ" => TokenKind::Print,             // chāp (print)
        "بنویس" => TokenKind::Print,           // benevis (write)
        // === SOV-S7 PARITY ===
        "هدف" => TokenKind::Intent,            // hadaf (intent)
        "نوع" => TokenKind::Type,              // nō' (type)
        "خارجی" => TokenKind::Extern,          // khārejī (external)
        _ => return None,
    };
    Some(kind)
}

/// Phase 12.5 (2026-06-07): Pashto (پښتو) keyword resolution.
/// Iranian language family with its own vocabulary distinct
/// from Persian and Urdu. Pashto uses extended Arabic letters
/// for unique sounds (ښ ګ ړ ټ ډ ڼ etc).
fn pashto_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        // Function / declarations
        "فنکشن" => TokenKind::Fn,              // fankshan (loanword)
        "کار" => TokenKind::Fn,                // kār (work — shared Persian root)
        "ووایه" => TokenKind::Let,             // wowāya (assume)
        "جوړښت" => TokenKind::Struct,          // jorrxṣ̌t (structure)
        "شمېرل" => TokenKind::Enum,            // shmerel (counting)
        "ثابت" => TokenKind::Const,            // sābet (constant)
        // Visibility
        "عمومي" => TokenKind::Pub,             // omumī (public)
        "برخه" => TokenKind::Module,           // barkha (part)
        "وکاروه" => TokenKind::Use,            // wakaraweh (use)
        "په توګه" => TokenKind::As,            // pe toga (as) — multi-word; not folded yet
        // Control flow
        "بېرته" => TokenKind::Return,          // berta (back)
        "که" => TokenKind::If,                 // ka (if)
        "که نه" => TokenKind::Else,            // ka na (else — multi-word)
        "تر څو" => TokenKind::While,           // tar tso (until)
        "هر یو" => TokenKind::For,             // har yew (each one — multi-word)
        "په" => TokenKind::In,                 // pe (in)
        "له" => TokenKind::From,               // le (from)
        "ودروه" => TokenKind::Break,           // wadrawa (stop)
        "دوام" => TokenKind::Continue,         // dawām (continuation)
        "بیا" => TokenKind::Then,              // bya (then)
        // References / mut
        "وګوره" => TokenKind::Ref,             // waguwra (see)
        "د بدلون وړ" => TokenKind::Mut,        // dalbedlun war (changeable — multi-word)
        // Match
        "سمون" => TokenKind::Match,            // samun (alignment)
        // Verification
        "تایید" => TokenKind::Assert,          // tāyid (confirm)
        "ثبوت" => TokenKind::Prove,            // sboot (proof)
        "اړتیا" => TokenKind::Requires,        // arrtya (need)
        "ډاډ" => TokenKind::Ensures,           // ḍāḍ (assurance)
        // Bool / print
        "سم" => TokenKind::True,               // sam (correct)
        "ناسم" => TokenKind::False,            // nāsam (incorrect)
        "ولیکه" => TokenKind::Print,           // wlika (write)
        // SOV-S7 parity
        "موخه" => TokenKind::Intent,           // mokha (purpose)
        "ډول" => TokenKind::Type,              // ḍol (type)
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.29 (2026-06-08): Khmer (ខ្មែរ). ~16M speakers.
fn khmer_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "មុខងារ" => TokenKind::Fn,             // muk-ngear (function)
        "អោយ" => TokenKind::Let,                // aoy (let)
        "ត្រលប់" => TokenKind::Return,          // trolop (return)
        "បើ" => TokenKind::If,                  // boe (if)
        "ផ្សេង" => TokenKind::Else,             // psen (other / else)
        "ខណៈ" => TokenKind::While,              // khna (while)
        "សម្រាប់" => TokenKind::For,            // samrap (for)
        "ក្នុង" => TokenKind::In,               // knong (inside)
        "ពី" => TokenKind::From,                // pi (from)
        "ដល់" => TokenKind::To,                 // dol (to)
        "បំបាក់" => TokenKind::Break,           // bombak (break)
        "បន្ត" => TokenKind::Continue,          // bont (continue)
        "បន្ទាប់មក" => TokenKind::Then,         // bontoap mok (then)
        "មើល" => TokenKind::Ref,                // mel (look)
        "អាចផ្លាស់ប្តូរ" => TokenKind::Mut,    // achplas pdor (changeable)
        "ផ្គូផ្គង" => TokenKind::Match,         // phkuphkong (match)
        "បញ្ជាក់" => TokenKind::Assert,         // banh-cheak (confirm)
        "បង្ហាញ" => TokenKind::Prove,           // bang-haynh (show)
        "ត្រូវការ" => TokenKind::Requires,      // trov ka (requires)
        "ធានា" => TokenKind::Ensures,           // thnea (guarantee)
        "ពិត" => TokenKind::True,               // pit (true)
        "មិនពិត" => TokenKind::False,           // min pit (not true)
        "បោះពុម្ព" => TokenKind::Print,         // boh-pum (print)
        "បរិសុទ្ធ" => TokenKind::Pure,          // borisuthr (pure)
        "ស្របគ្នា" => TokenKind::Parallel,      // srab knea (parallel)
        "ចំណុចប្រទាក់" => TokenKind::Interface, // chamnuch protaak
        "វិធីសាស្ត្រ" => TokenKind::Methods,    // vithisar (methods)
        "ណា" => TokenKind::Where,               // na (where)
        "គឺ" => TokenKind::Is,                  // kue (is)
        "ព្យាយាម" => TokenKind::Try,            // pyayam (try)
        "ភារកិច្ច" => TokenKind::Task,          // pheakkechh (task)
        "ភ្ជាប់" => TokenKind::Join,            // phjeap (join)
        "មិនមានសុវត្ថិភាព" => TokenKind::Unsafe, // unsafe
        "តំបន់" => TokenKind::RegionKw,         // tombon (region)
        "គោលបំណង" => TokenKind::Intent,         // kolboumnong (goal)
        "ប្រភេទ" => TokenKind::Type,            // probhett (type)
        "ខាងក្រៅ" => TokenKind::Extern,         // khang krov (outside)
        "មិនប្រែប្រួល" => TokenKind::Invariant, // unchanging
        "រចនាសម្ព័ន្ធ" => TokenKind::Struct,    // structure
        "ការរាប់បញ្ចូល" => TokenKind::Enum,     // counting
        "ថេរ" => TokenKind::Const,              // theer (constant)
        "សាធារណៈ" => TokenKind::Pub,            // satheareak (public)
        "ម៉ូឌុល" => TokenKind::Module,          // modul (loanword)
        "ប្រើ" => TokenKind::Use,               // proeu (use)
        "ជា" => TokenKind::As,                  // chea (as)
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.30 (2026-06-08): Burmese (မြန်မာ). ~33M speakers.
fn burmese_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "လုပ်ဆောင်ချက်" => TokenKind::Fn,       // loutsaungchet (fn)
        "ထား" => TokenKind::Let,                // htar (let)
        "ပြန်" => TokenKind::Return,            // pyan (return)
        "ဆိုလျှင်" => TokenKind::If,            // sho lyin (if)
        "မဟုတ်ပါက" => TokenKind::Else,          // ma hote pa (else)
        "နေစဉ်" => TokenKind::While,            // ne sin (while)
        "အတွက်" => TokenKind::For,              // atwet (for)
        "ထဲမှာ" => TokenKind::In,               // htel hma (in)
        "မှ" => TokenKind::From,                // hma (from)
        "သို့" => TokenKind::To,                // sou (to)
        "ရပ်" => TokenKind::Break,              // yat (break)
        "ဆက်လုပ်" => TokenKind::Continue,       // set lout (continue)
        "ထို့နောက်" => TokenKind::Then,         // htou nout (then)
        "ကြည့်" => TokenKind::Ref,              // kyi (look)
        "ပြောင်းလဲနိုင်" => TokenKind::Mut,    // pyaung le nain (mutable)
        "ကိုက်ညီ" => TokenKind::Match,          // kaik nyi (match)
        "သေချာ" => TokenKind::Assert,           // the cha (confirm)
        "သက်သေပြ" => TokenKind::Prove,          // thethe pya (prove)
        "လို" => TokenKind::Requires,           // lou (need)
        "ဆောင်ရွက်" => TokenKind::Ensures,      // saung ywet (ensure)
        "မှန်" => TokenKind::True,              // man (true)
        "မှား" => TokenKind::False,             // m`a (false)
        "ပုံနှိပ်" => TokenKind::Print,         // poun hnip (print)
        "သန့်ရှင်း" => TokenKind::Pure,         // than shin (pure)
        "ပြိုင်တူ" => TokenKind::Parallel,      // pyaung tu (parallel)
        "မျက်နှာပြင်" => TokenKind::Interface,  // myet hna pyin
        "နည်းလမ်း" => TokenKind::Methods,       // ne lam
        "ဘယ်မှာ" => TokenKind::Where,           // be hma
        "ဖြစ်သည်" => TokenKind::Is,             // pyit de
        "ကြိုးစား" => TokenKind::Try,           // kyo sar
        "တာဝန်" => TokenKind::Task,             // ta wun
        "ပူးပေါင်း" => TokenKind::Join,         // poo paung
        "ဘေးကင်းမှု" => TokenKind::Unsafe,      // be kin hmu
        "ဒေသ" => TokenKind::RegionKw,           // de tha
        "ရည်ရွယ်ချက်" => TokenKind::Intent,     // yi ywet chet
        "အမျိုးအစား" => TokenKind::Type,        // amyo asar
        "အပြင်" => TokenKind::Extern,           // a pyin
        "မပြောင်းလဲ" => TokenKind::Invariant,   // ma pyaung le
        "ဖွဲ့စည်းပုံ" => TokenKind::Struct,     // pwe si poun
        "စာရင်း" => TokenKind::Enum,            // sayin
        "ပုံသေ" => TokenKind::Const,            // poun the
        "အများပြည်သူ" => TokenKind::Pub,        // amya pyi thu
        "ယူနစ်" => TokenKind::Module,           // yu nit (loanword)
        "သုံး" => TokenKind::Use,               // thoun
        "အဖြစ်" => TokenKind::As,               // a pyit
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.31 (2026-06-08): Amharic (አማርኛ). ~32M speakers.
fn amharic_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "ተግባር" => TokenKind::Fn,               // tegbar (function/task)
        "ይሁን" => TokenKind::Let,                // yihun (let)
        "መልስ" => TokenKind::Return,             // mels (return)
        "ከ" => TokenKind::If,                   // ke (if)
        "ካልሆነ" => TokenKind::Else,              // kal hone (if not)
        "ሲ" => TokenKind::While,                // si (while)
        "ለ" => TokenKind::For,                  // le (for)
        "ውስጥ" => TokenKind::In,                 // wist (in)
        "ስብር" => TokenKind::Break,              // sber (break)
        "ቀጥል" => TokenKind::Continue,           // qetel (continue)
        "ከዚያ" => TokenKind::Then,               // kezia (then)
        "ይመልከት" => TokenKind::Ref,              // yimelket (look)
        "ሊቀየር" => TokenKind::Mut,               // likeyer (changeable)
        "ተዛመደ" => TokenKind::Match,             // tezamede (match)
        "አረጋግጥ" => TokenKind::Assert,           // aregagit (confirm)
        "አስረዳ" => TokenKind::Prove,             // asreda (prove)
        "ይፈልጋል" => TokenKind::Requires,         // yefelegal (needs)
        "ያረጋግጣል" => TokenKind::Ensures,         // yaregagital (ensures)
        "እውነት" => TokenKind::True,              // ewenet (truth)
        "ሐሰት" => TokenKind::False,              // haset (lie)
        "ህትመት" => TokenKind::Print,             // htmet (print)
        "ንጹህ" => TokenKind::Pure,               // ntsuh (pure)
        "ትይዩ" => TokenKind::Parallel,           // tyiyu (parallel)
        "በይነገጽ" => TokenKind::Interface,        // beyinegets
        "ዘዴዎች" => TokenKind::Methods,           // zedewotch
        "የት" => TokenKind::Where,               // yet (where)
        "ነው" => TokenKind::Is,                  // new (is)
        "ሞክር" => TokenKind::Try,                // mokr (try)
        "ስራ" => TokenKind::Task,                // sra (task)
        "ቀላቀል" => TokenKind::Join,              // kelakel
        "አደገኛ" => TokenKind::Unsafe,            // adegegna (dangerous)
        "ክልል" => TokenKind::RegionKw,           // kll (region)
        "ዓላማ" => TokenKind::Intent,             // alama (goal)
        "አይነት" => TokenKind::Type,              // aynet
        "ውጫዊ" => TokenKind::Extern,             // wchawi (external)
        "የማይለወጥ" => TokenKind::Invariant,       // unchanging
        "መዋቅር" => TokenKind::Struct,            // mewakr
        "ቆጠራ" => TokenKind::Enum,               // qotera
        "ቋሚ" => TokenKind::Const,               // qwami
        "ሕዝባዊ" => TokenKind::Pub,               // hzbawi
        "ሞዱል" => TokenKind::Module,             // modul
        "ተጠቀም" => TokenKind::Use,               // tetekem
        "እንደ" => TokenKind::As,                 // ende
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.32 (2026-06-08): Tibetan (བོད་ཡིག). ~7M speakers.
/// Note: Tibetan uses the tsek (་) as syllable separator within
/// a word; the resulting "word" is one ident.
fn tibetan_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "ལས་ཀ" => TokenKind::Fn,                // lay-ka (work / fn)
        "ཡོད་པར་ཤོག" => TokenKind::Let,         // yod-par-shog (let it be)
        "ལོག" => TokenKind::Return,              // lokh (return)
        "གལ་ཏེ" => TokenKind::If,                // galte (if)
        "གཞན" => TokenKind::Else,                // zhan (else)
        "བར" => TokenKind::While,                // bar (while)
        "ལ" => TokenKind::For,                   // la (for)
        "ནང" => TokenKind::In,                   // nang (in)
        "ནས" => TokenKind::From,                 // nas (from)
        "བར་དུ" => TokenKind::To,                // bar-du (until)
        "འགོག" => TokenKind::Break,              // gog (block)
        "མུ་མཐུད" => TokenKind::Continue,        // mu-thud (continue)
        "དེ་ནས" => TokenKind::Then,              // de-nas (then)
        "ལྟ" => TokenKind::Ref,                  // lta (see)
        "འགྱུར" => TokenKind::Mut,               // gyur (change)
        "མཐུན" => TokenKind::Match,              // thun (agree)
        "ངེས" => TokenKind::Assert,              // nges (certain)
        "བསྒྲུབས" => TokenKind::Prove,           // sgrubs (proven)
        "དགོས" => TokenKind::Requires,           // gos (need)
        "ཁག" => TokenKind::Ensures,              // khag (guarantee)
        "བདེན" => TokenKind::True,               // den (true)
        "རྫུན" => TokenKind::False,              // dzun (false)
        "པར" => TokenKind::Print,                // par (print)
        "གཙང" => TokenKind::Pure,                // tsang (pure)
        "མཉམ" => TokenKind::Parallel,            // nyam (parallel)
        "གང" => TokenKind::Where,                // gang (where)
        "ཡིན" => TokenKind::Is,                  // yin (is)
        "འབད" => TokenKind::Try,                 // ed (try)
        "ལས" => TokenKind::Task,                 // las (work — could conflict; pragma-gated)
        "མཐུན་སྦྱོར" => TokenKind::Join,         // thun-jor
        "ཉེན་ཁ" => TokenKind::Unsafe,            // nyen-kha (danger)
        "ཁུལ" => TokenKind::RegionKw,            // khul (region)
        "དམིགས་ཡུལ" => TokenKind::Intent,        // mig-yul (goal)
        "རིགས" => TokenKind::Type,               // rig (type / kind)
        "ཕྱི" => TokenKind::Extern,              // chi (outer)
        "རྩིས" => TokenKind::Enum,               // tsis (counting)
        "རྟག" => TokenKind::Const,               // tag (constant)
        "སྤྱི" => TokenKind::Pub,                // chi (public)
        "ཚན" => TokenKind::Module,               // tsen (module)
        "བཀོལ" => TokenKind::Use,                // kol (use)
        "དུ" => TokenKind::As,                   // du (as)
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.33 (2026-06-08): Cherokee (ᏣᎳᎩ). Minimal keyword
/// set to give the syllabary a host in vāṇी.
fn cherokee_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "ᏗᎦᏬᏂᎯᏍᏗ" => TokenKind::Fn,             // digawonihisdi (function)
        "ᎠᏁᎳ" => TokenKind::Let,                 // anela (let)
        "ᏗᎬᏎᏗ" => TokenKind::Return,             // digusedi (return)
        "ᎢᏳᏃ" => TokenKind::If,                  // iyuno (if)
        "ᎪᎯ" => TokenKind::Else,                 // gohi (other)
        "ᏰᎵᏊ" => TokenKind::While,               // yelikwu (while)
        "ᏌᏊ" => TokenKind::For,                  // sakwu (each / for)
        "ᎭᏫᎾ" => TokenKind::In,                  // hawina (in)
        "ᎤᎵᏍᎩᏗ" => TokenKind::Break,             // ulisgidi (break)
        "ᏗᎧᎵᏍᏗ" => TokenKind::Continue,          // diqalisdi (continue)
        "ᎯᎪᎲᎢ" => TokenKind::Ref,                // higohvi (see)
        "ᏚᎵᎮᎵᎬᎢ" => TokenKind::Mut,             // dulihelvgvi (changeable)
        "ᎤᏙᎯᏳ" => TokenKind::True,               // udohiyu (true)
        "ᎤᏝ" => TokenKind::False,                // utla (false)
        "ᎠᎴᏂᏍᎬᎢ" => TokenKind::Print,           // alenisgvgi (print)
        "ᎯᏍᏗᏎᏍᏗ" => TokenKind::Assert,          // confirm
        "ᎠᎩᏠᏯᏍᏗ" => TokenKind::Prove,           // demonstrate
        "ᎠᏙᏢᏍᎩ" => TokenKind::Struct,           // structure
        "ᎬᏙᏗ" => TokenKind::Use,                 // gvdodi (use)
        "ᎤᎲᏍᏛ" => TokenKind::Intent,             // uhvsdv (purpose)
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.34 (2026-06-08): Lao (ລາວ). ~30M speakers. Closely
/// related to Thai but distinct script.
fn lao_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "ໜ້າທີ່" => TokenKind::Fn,               // na thi (function)
        "ໃຫ້" => TokenKind::Let,                  // hai (give / let)
        "ກັບຄືນ" => TokenKind::Return,            // kap kuen (return)
        "ຖ້າ" => TokenKind::If,                   // tha (if)
        "ບໍ່ດັ່ງນັ້ນ" => TokenKind::Else,         // bo dang nan
        "ໃນຂະນະທີ່" => TokenKind::While,          // nai khanat thi
        "ສຳລັບ" => TokenKind::For,                // samlap (for)
        "ໃນ" => TokenKind::In,                    // nai (in)
        "ຈາກ" => TokenKind::From,                 // chak (from)
        "ເຖິງ" => TokenKind::To,                  // theung (to)
        "ຢຸດ" => TokenKind::Break,                // yut (stop)
        "ສືບຕໍ່" => TokenKind::Continue,          // sueb to (continue)
        "ແລ້ວ" => TokenKind::Then,                // laeo (then)
        "ເບິ່ງ" => TokenKind::Ref,                // beung (look)
        "ປ່ຽນແປງໄດ້" => TokenKind::Mut,          // pian paeng dai
        "ກົງກັນ" => TokenKind::Match,             // kong kan (match)
        "ຢືນຢັນ" => TokenKind::Assert,            // yuenyan (confirm)
        "ພິສູດ" => TokenKind::Prove,              // phisuat (prove)
        "ຕ້ອງການ" => TokenKind::Requires,         // tongkan (need)
        "ຮັບປະກັນ" => TokenKind::Ensures,         // hap pa kan
        "ຈິງ" => TokenKind::True,                 // jing (true)
        "ບໍ່ຈິງ" => TokenKind::False,             // bo jing (false)
        "ພິມ" => TokenKind::Print,                // phim (print)
        "ບໍລິສຸດ" => TokenKind::Pure,             // bolisuth (pure)
        "ຂະໜານ" => TokenKind::Parallel,           // khanan (parallel)
        "ສ່ວນຕິດຕໍ່" => TokenKind::Interface,     // suan tit to
        "ວິທີການ" => TokenKind::Methods,          // vithikan (methods)
        "ບ່ອນທີ່" => TokenKind::Where,            // bon thi (where)
        "ແມ່ນ" => TokenKind::Is,                  // maen (is)
        "ລອງ" => TokenKind::Try,                  // long (try)
        "ວຽກງານ" => TokenKind::Task,              // viak ngan (task)
        "ເຊື່ອມ" => TokenKind::Join,              // sueam (join)
        "ບໍ່ປອດໄພ" => TokenKind::Unsafe,          // bo pot phai (unsafe)
        "ພູມພາກ" => TokenKind::RegionKw,          // phumphak (region)
        "ຈຸດປະສົງ" => TokenKind::Intent,          // chutpasong (intent)
        "ປະເພດ" => TokenKind::Type,               // paphet (type)
        "ພາຍນອກ" => TokenKind::Extern,            // phay nok (external)
        "ບໍ່ປ່ຽນ" => TokenKind::Invariant,        // bo pian (invariant)
        "ໂຄງສ້າງ" => TokenKind::Struct,           // khong sang (structure)
        "ການນັບ" => TokenKind::Enum,              // kan nap (counting)
        "ຄົງທີ່" => TokenKind::Const,             // khong thi (constant)
        "ສາທາລະນະ" => TokenKind::Pub,             // satharana (public)
        "ໂມດູນ" => TokenKind::Module,             // modun (module)
        "ໃຊ້" => TokenKind::Use,                  // sai (use)
        "ເປັນ" => TokenKind::As,                  // pen (as)
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.35 (2026-06-08): Mongolian traditional (ᠮᠣᠩᠭᠣᠯ).
/// ~6M speakers in Inner Mongolia. Minimal v1 keyword set.
fn mongolian_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "ᠴᠠᠭ" => TokenKind::Fn,                  // tsag (function — using "task")
        "ᠶᠠᠪᠤᠭᠤᠯ" => TokenKind::Let,           // yabuul (send / let)
        "ᠪᠤᠴᠠ" => TokenKind::Return,             // butsa (return!)
        "ᠬᠡᠷᠪᠡ" => TokenKind::If,                // kerbe (if)
        "ᠡᠰᠡᠪᠡᠯ" => TokenKind::Else,             // esebel (or else)
        "ᠶᠠᠭ᠎ᠠ" => TokenKind::While,             // yaga (while)
        "ᠬᠠᠷᠠᠭᠠᠯᠵᠠᠯ" => TokenKind::For,         // for
        "ᠠᠴᠠ" => TokenKind::From,                // atsa (from)
        "ᠬᠦᠷᠲᠡᠯᠡ" => TokenKind::To,              // kürtele (to)
        "ᠵᠣᠭᠰᠣ" => TokenKind::Break,             // zogso (stop)
        "ᠦᠷᠭᠦᠯᠵᠢᠯᠡ" => TokenKind::Continue,     // urgelje (continue)
        "ᠳᠠᠷᠠᠭ᠎ᠠ" => TokenKind::Then,            // daraga (then)
        "ᠦᠵᠡ" => TokenKind::Ref,                 // üze (see)
        "ᠦᠨᠡᠨ" => TokenKind::True,               // ünen (true)
        "ᠬᠤᠳᠠᠯ" => TokenKind::False,             // hudal (false)
        "ᠬᠡᠪᠯᠡ" => TokenKind::Print,             // keble (print)
        "ᠪᠠᠲᠤᠯ" => TokenKind::Assert,            // batul (verify)
        "ᠨᠣᠲᠠᠯᠠ" => TokenKind::Prove,            // notala (prove)
        "ᠴᠡᠪᠡᠷ" => TokenKind::Pure,              // tseber (pure)
        "ᠡᠭᠦᠷᠭᠡ" => TokenKind::Task,             // ügüre (task)
        "ᠬᠡᠷᠡᠭᠯᠡ" => TokenKind::Use,             // keregle (use)
        "ᠬᠡᠯᠪᠡᠷᠢ" => TokenKind::Type,            // kelberi (type / kind)
        "ᠪᠦᠳᠦᠭᠴᠡ" => TokenKind::Struct,          // büdüktse (structure)
        "ᠵᠣᠷᠢᠯᠭ᠎ᠠ" => TokenKind::Intent,         // zorilga (goal / purpose)
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.24 (2026-06-08): Slovak (slovenčina). Third Slavic
/// Latin variant. Shares ASCII fallbacks with Czech but uses
/// distinct keyword choices.
fn slovak_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "vráť" => TokenKind::Return,           // return!
        "preruš" => TokenKind::Break,          // break!
        "pokračuj" => TokenKind::Continue,     // continue!
        "potvrď" => TokenKind::Assert,         // confirm
        "dokáž" => TokenKind::Prove,           // prove!
        "vyžaduje" => TokenKind::Requires,     // requires
        "zaručuje" => TokenKind::Ensures,      // guarantees
        "vytlač" => TokenKind::Print,          // print
        "píš" => TokenKind::Print,             // write (alt)
        "čistý" => TokenKind::Pure,            // pure
        "paralelný" => TokenKind::Parallel,    // parallel
        "rozhranie" => TokenKind::Interface,   // interface
        "úloha" => TokenKind::Task,            // task
        "nebezpečný" => TokenKind::Unsafe,     // unsafe
        "oblasť" => TokenKind::RegionKw,       // region
        "účel" => TokenKind::Intent,           // purpose
        "vonkajší" => TokenKind::Extern,       // external
        "nemenný" => TokenKind::Invariant,     // unchanging
        "štruktúra" => TokenKind::Struct,      // structure
        "verejný" => TokenKind::Pub,           // public
        "použi" => TokenKind::Use,             // use!
        "meniteľný" => TokenKind::Mut,         // changeable
        "pokiaľ" => TokenKind::While,          // as long as
        "kým" => TokenKind::While,             // until (while alt)
        "porovnaj" => TokenKind::Match,        // compare!
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.24 pure-ASCII Slovak. Pragma-gated.
fn slovak_ascii_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "funkcia" => TokenKind::Fn,            // function
        "nech" => TokenKind::Let,              // let
        "konstanta" => TokenKind::Const,       // constant
        "modul" => TokenKind::Module,          // module
        "ako" => TokenKind::As,                // as
        "ak" => TokenKind::If,                 // if
        "kedy" => TokenKind::If,               // when (alt)
        "inak" => TokenKind::Else,             // otherwise
        "pre" => TokenKind::For,               // for
        "od" => TokenKind::From,               // from
        "do" => TokenKind::To,                 // to
        "potom" => TokenKind::Then,            // then
        "pozri" => TokenKind::Ref,             // see
        "pravda" => TokenKind::True,           // true
        "nepravda" => TokenKind::False,        // false
        "metody" => TokenKind::Methods,        // methods (ASCII alt)
        "kde" => TokenKind::Where,             // where
        "je" => TokenKind::Is,                 // is
        "skus" => TokenKind::Try,              // try (no diacritic alt)
        "skús" => TokenKind::Try,              // (with diacritic — but routed via non-ASCII path; harmless duplicate)
        "spoj" => TokenKind::Join,             // join
        "typ" => TokenKind::Type,              // type
        "vypocet" => TokenKind::Enum,          // enum (no diacritic)
        // No-diacritic alts:
        "vrat" => TokenKind::Return,           // return
        "perus" => TokenKind::Break,           // (no ř)
        "pokracuj" => TokenKind::Continue,     // (no č)
        "potvrd" => TokenKind::Assert,         // (no ď)
        "dokaz" => TokenKind::Prove,           // (no ž)
        "vyzaduje" => TokenKind::Requires,     // (no ž)
        "zarucuje" => TokenKind::Ensures,      // (no č)
        "vytlac" => TokenKind::Print,          // (no č)
        "pis" => TokenKind::Print,             // (no š)
        "cisty" => TokenKind::Pure,            // (no č)
        "paralelny" => TokenKind::Parallel,    // (no ý)
        "rozhranie" => TokenKind::Interface,   // (already ASCII)
        "uloha" => TokenKind::Task,            // (no ú)
        "nebezpecny" => TokenKind::Unsafe,     // (no č)
        "oblast" => TokenKind::RegionKw,       // (no ť)
        "ucel" => TokenKind::Intent,           // (no ú)
        "vonkajsi" => TokenKind::Extern,       // (no š)
        "nemenny" => TokenKind::Invariant,     // (no ý)
        "struktura" => TokenKind::Struct,      // (no š)
        "verejny" => TokenKind::Pub,           // (no ý)
        "pouzi" => TokenKind::Use,             // (no ž)
        "menitelny" => TokenKind::Mut,         // (no ľ, ý)
        "pokial" => TokenKind::While,          // (no ľ)
        "porovnaj" => TokenKind::Match,        // (already ASCII)
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.25 (2026-06-08): Finnish (suomi). Second Uralic
/// after Hungarian — distinct keyword set (Finnish and
/// Hungarian split ~9000 years ago).
fn finnish_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "tehtävä" => TokenKind::Task,          // task
        "rajapintä" => TokenKind::Interface,   // interface (variant)
        "vahvistä" => TokenKind::Assert,       // (rare variant)
        "muuttumatön" => TokenKind::Invariant, // unchanging (variant)
        "käytä" => TokenKind::Use,             // use!
        "missä" => TokenKind::Where,           // where
        "lähtien" => TokenKind::From,          // from
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.25 (2026-06-08): pure-ASCII Finnish. Pragma-gated.
/// Most Finnish keywords are pure-ASCII since Finnish uses only
/// ä/ö as native non-ASCII letters (and many keywords avoid
/// them).
fn finnish_ascii_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "funktio" => TokenKind::Fn,            // function
        "olkoon" => TokenKind::Let,            // "let it be"
        "rakenne" => TokenKind::Struct,        // structure
        "luettelointi" => TokenKind::Enum,     // enumeration
        "vakio" => TokenKind::Const,           // constant
        "julkinen" => TokenKind::Pub,          // public
        "moduuli" => TokenKind::Module,        // module
        "kuten" => TokenKind::As,              // as / like
        "palaa" => TokenKind::Return,          // return / come back
        "jos" => TokenKind::If,                // if
        "muuten" => TokenKind::Else,           // otherwise
        "kun" => TokenKind::While,             // when / while
        "jokaiselle" => TokenKind::For,        // for each
        "sisalla" => TokenKind::In,            // inside (no ä alt)
        "asti" => TokenKind::To,               // until
        "katkaise" => TokenKind::Break,        // break!
        "jatka" => TokenKind::Continue,        // continue!
        "sitten" => TokenKind::Then,           // then
        "katso" => TokenKind::Ref,             // see
        "muuttuva" => TokenKind::Mut,          // changing
        "vastaa" => TokenKind::Match,          // correspond
        "vahvista" => TokenKind::Assert,       // confirm
        "todista" => TokenKind::Prove,         // prove
        "vaatii" => TokenKind::Requires,       // requires
        "takaa" => TokenKind::Ensures,         // guarantees
        "tosi" => TokenKind::True,             // true
        "epatosi" => TokenKind::False,         // false (no ä)
        "tulosta" => TokenKind::Print,         // print
        "puhdas" => TokenKind::Pure,           // pure
        "rinnakkainen" => TokenKind::Parallel, // parallel
        "rajapinta" => TokenKind::Interface,   // interface
        "toteuta" => TokenKind::Implement,     // implement
        "menetelmat" => TokenKind::Methods,    // methods (no ä)
        "on" => TokenKind::Is,                 // is
        "kokeile" => TokenKind::Try,           // try
        "tehtava" => TokenKind::Task,          // task (no ä)
        "yhdista" => TokenKind::Join,          // join (no ä)
        "vaarallinen" => TokenKind::Unsafe,    // dangerous
        "alue" => TokenKind::RegionKw,         // area
        "tarkoitus" => TokenKind::Intent,      // purpose
        "tyyppi" => TokenKind::Type,           // type
        "ulkoinen" => TokenKind::Extern,       // external
        "muuttumaton" => TokenKind::Invariant, // unchanging (no ö)
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.26 (2026-06-08): Catalan (català). Sixth Romance
/// Latin variant. Distinctive interpunct (l·l) but most
/// keywords just use à/è/é/í/ï/ò/ó/ú/ü.
fn catalan_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "funció" => TokenKind::Fn,             // function
        "enumeració" => TokenKind::Enum,       // enumeration
        "públic" => TokenKind::Pub,            // public
        "mòdul" => TokenKind::Module,          // module
        "està" => TokenKind::Is,               // is (Catalan "is")
        "és" => TokenKind::Is,                 // is (alt)
        "aleshores" => TokenKind::Then,        // then (ASCII actually)
        "coincideix" => TokenKind::Match,      // matches (ASCII)
        "demostra" => TokenKind::Prove,        // (ASCII)
        "mètodes" => TokenKind::Methods,       // methods
        "regió" => TokenKind::RegionKw,        // region
        "propòsit" => TokenKind::Intent,       // purpose
        "imprimeix" => TokenKind::Print,       // (ASCII)
        "interfície" => TokenKind::Interface,  // interface
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.26 (2026-06-08): pure-ASCII Catalan. Pragma-gated.
fn catalan_ascii_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "funcio" => TokenKind::Fn,             // function (no accent)
        "sigui" => TokenKind::Let,             // let it be
        "estructura" => TokenKind::Struct,     // structure
        "enumeracio" => TokenKind::Enum,       // enumeration (no accent)
        "constant" => TokenKind::Const,        // constant
        "public" => TokenKind::Pub,            // public (no accent)
        "modul" => TokenKind::Module,          // module (no accent)
        "usa" => TokenKind::Use,               // use
        "com" => TokenKind::As,                // as / like
        "retorna" => TokenKind::Return,        // return
        "si" => TokenKind::If,                 // if
        "altrament" => TokenKind::Else,        // otherwise
        "sino" => TokenKind::Else,             // else (alt)
        "mentre" => TokenKind::While,          // while
        "per" => TokenKind::For,               // for
        "en" => TokenKind::In,                 // in
        "des" => TokenKind::From,              // from
        "fins" => TokenKind::To,               // to / until
        "trenca" => TokenKind::Break,          // break
        "continua" => TokenKind::Continue,     // continue
        "aleshores" => TokenKind::Then,        // then
        "veure" => TokenKind::Ref,             // see
        "mutable" => TokenKind::Mut,           // mutable
        "canviable" => TokenKind::Mut,         // changeable (alt)
        "coincideix" => TokenKind::Match,      // matches
        "afirma" => TokenKind::Assert,         // assert
        "demostra" => TokenKind::Prove,        // prove
        "requereix" => TokenKind::Requires,    // requires
        "garanteix" => TokenKind::Ensures,     // guarantees
        "cert" => TokenKind::True,             // true / certain
        "veritable" => TokenKind::True,        // true (alt)
        "fals" => TokenKind::False,            // false
        "imprimeix" => TokenKind::Print,       // print
        "pur" => TokenKind::Pure,              // pure
        "interface" => TokenKind::Interface,   // interface (loanword)
        "implementa" => TokenKind::Implement,  // implement
        "metodes" => TokenKind::Methods,       // methods (no accent)
        "on" => TokenKind::Where,              // where
        "prova" => TokenKind::Try,             // try / test
        "tasca" => TokenKind::Task,            // task
        "uneix" => TokenKind::Join,            // join
        "insegur" => TokenKind::Unsafe,        // unsafe
        "regio" => TokenKind::RegionKw,        // region (no accent)
        "proposit" => TokenKind::Intent,       // purpose (no accent)
        "objectiu" => TokenKind::Intent,       // objective (alt)
        "tipus" => TokenKind::Type,            // type
        "extern" => TokenKind::Extern,         // external (same as English)
        "invariant" => TokenKind::Invariant,   // invariant (same)
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.27 (2026-06-08): Yoruba (Èdè Yorùbá) — Niger-Congo,
/// ~50M speakers. Latin script with sub-dot marks (ẹ/ọ/ṣ) plus
/// extensive tone marks. The keyword table holds the natural
/// diacritic forms; pragma-gated ASCII fallbacks for code
/// editors that struggle with the marks.
fn yoruba_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "iṣẹ́" => TokenKind::Fn,                // work / function
        "jẹ́" => TokenKind::Let,                // let
        "padà" => TokenKind::Return,           // return / come back
        "bí" => TokenKind::If,                 // if
        "nígbà" => TokenKind::While,           // while
        "fún" => TokenKind::For,               // for
        "nínú" => TokenKind::In,               // inside
        "láti" => TokenKind::From,             // from
        "dé" => TokenKind::To,                 // to
        "tẹ̀síwájú" => TokenKind::Continue,    // continue
        "nígbànáà" => TokenKind::Then,         // then
        "wò" => TokenKind::Ref,                // look at
        "àyípadà" => TokenKind::Mut,           // change / mutable
        "bámu" => TokenKind::Match,            // match
        "jẹ́risí" => TokenKind::Assert,        // confirm
        "fihàn" => TokenKind::Prove,           // show
        "nílò" => TokenKind::Requires,         // need
        "òótọ́" => TokenKind::True,            // truth
        "irọ́" => TokenKind::False,            // lie
        "tẹ̀" => TokenKind::Print,             // print / press
        "mímọ́" => TokenKind::Pure,            // pure
        "ìpinnu" => TokenKind::Intent,         // decision / purpose
        "irú" => TokenKind::Type,              // kind / type
        "ìta" => TokenKind::Extern,            // outside
        "ìṣù" => TokenKind::Module,            // module
        "lò" => TokenKind::Use,                // use
        "bí_ti" => TokenKind::As,              // as
        "iṣẹ" => TokenKind::Task,              // task
        "ọ̀nà" => TokenKind::Struct,           // way / structure
        "ibo" => TokenKind::Where,             // where (ASCII actually)
        "gbangba" => TokenKind::Pub,           // public (ASCII)
        "ni" => TokenKind::Is,                 // is (ASCII)
        "gbiyanju" => TokenKind::Try,          // try (ASCII)
        "darapọ" => TokenKind::Join,           // join (has ọ)
        "àìláàbò" => TokenKind::Unsafe,        // unsafe
        "agbègbè" => TokenKind::RegionKw,      // area
        "akáṣe" => TokenKind::Parallel,        // parallel
        "ifaramọ" => TokenKind::Interface,     // commitment / interface
        "ipa" => TokenKind::Methods,           // methods
        "àìyípadà" => TokenKind::Invariant,    // not changing
        "àkọsílẹ̀" => TokenKind::Enum,         // record
        "dáwọ́dúró" => TokenKind::Break,       // stop / break
        "àlàfo" => TokenKind::Const,           // constant
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.28 (2026-06-08): Hausa — Afroasiatic, ~80M
/// speakers. Latin Boko script with implosive consonants
/// ɓ/ɗ/ƙ/ƴ.
fn hausa_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "ɗaya" => TokenKind::Parallel,         // parallel (uses ɗ)
        "ƙarya" => TokenKind::False,           // lie (uses ƙ)
        "nau'i" => TokenKind::Type,            // kind (apostrophe)
        // (Most Hausa keywords are pure ASCII; only a few use
        // the implosive consonants.)
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.28 (2026-06-08): pure-ASCII Hausa. Pragma-gated.
fn hausa_ascii_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "aiki" => TokenKind::Fn,               // work / function
        "bari" => TokenKind::Let,              // let
        "tsari" => TokenKind::Struct,          // structure
        "lissafi" => TokenKind::Enum,          // enumeration
        "tabbas" => TokenKind::Const,          // constant / surely
        "gama_gari" => TokenKind::Pub,         // general / public
        "sashe" => TokenKind::Module,          // section / module
        "amfani" => TokenKind::Use,            // use / benefit
        "kamar" => TokenKind::As,              // like / as
        "koma" => TokenKind::Return,           // go back / return
        "idan" => TokenKind::If,               // if
        "ko_kuwa" => TokenKind::Else,          // or otherwise
        "yayin" => TokenKind::While,           // while / during
        "ga" => TokenKind::For,                // for / to
        "cikin" => TokenKind::In,              // inside
        "daga" => TokenKind::From,             // from
        "zuwa" => TokenKind::To,               // to / toward
        "dakatar" => TokenKind::Break,         // halt
        "ci_gaba" => TokenKind::Continue,      // go on
        "sannan" => TokenKind::Then,           // then / next
        "duba" => TokenKind::Ref,              // look at
        "canzawa" => TokenKind::Mut,           // changing
        "dace" => TokenKind::Match,            // match / fit
        "tabbatar" => TokenKind::Assert,       // confirm
        "nuna" => TokenKind::Prove,            // show
        "bukata" => TokenKind::Requires,       // need
        "tabbace" => TokenKind::Ensures,       // ensured
        "gaskiya" => TokenKind::True,          // truth
        "rubuta" => TokenKind::Print,          // write
        "tsabta" => TokenKind::Pure,           // cleanliness
        "madaidaici" => TokenKind::Parallel,   // parallel (ASCII alt)
        "hannu" => TokenKind::Interface,       // hand / interface
        "aiwatar" => TokenKind::Implement,     // implement
        "hanyoyi" => TokenKind::Methods,       // ways / methods
        "ina" => TokenKind::Where,             // where
        "ne" => TokenKind::Is,                 // is (masc.)
        "gwadawa" => TokenKind::Try,           // try
        "hadawa" => TokenKind::Join,           // join
        "kasada" => TokenKind::Unsafe,         // danger
        "yanki" => TokenKind::RegionKw,        // region
        "nufin" => TokenKind::Intent,          // purpose
        "waje" => TokenKind::Extern,           // outside
        "a_canzawa" => TokenKind::Invariant,   // not changing
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.20 (2026-06-08): Norwegian (norsk bokmål) keyword
/// resolution. Second Nordic dialect after Swedish; uses å/æ/ø.
fn norwegian_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "være" => TokenKind::Let,              // "be"
        "påstå" => TokenKind::Assert,          // claim / assert
        "prøv" => TokenKind::Try,              // try!
        "formål" => TokenKind::Intent,         // purpose
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.20 (2026-06-08): pure-ASCII Norwegian. Pragma-gated.
/// Norwegian is mostly pure-ASCII (few keyword glyphs carry the
/// å/æ/ø marks) so this table holds the bulk of the surface.
fn norwegian_ascii_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "funksjon" => TokenKind::Fn,           // function
        "la" => TokenKind::Let,                // let (short imperative)
        "struktur" => TokenKind::Struct,       // structure
        "oppregning" => TokenKind::Enum,       // enumeration
        "konstant" => TokenKind::Const,        // constant
        "offentlig" => TokenKind::Pub,         // public
        "modul" => TokenKind::Module,          // module
        "bruk" => TokenKind::Use,              // use
        "som" => TokenKind::As,                // as
        "returner" => TokenKind::Return,       // return!
        "tilbake" => TokenKind::Return,        // back (alt)
        "hvis" => TokenKind::If,               // if
        "ellers" => TokenKind::Else,           // else
        "mens" => TokenKind::While,            // while
        "fra" => TokenKind::From,              // from
        "til" => TokenKind::To,                // to
        "bryt" => TokenKind::Break,            // break
        "fortsett" => TokenKind::Continue,     // continue
        "da" => TokenKind::Then,               // then
        "endrelig" => TokenKind::Mut,          // mutable (alt)
        "foranderlig" => TokenKind::Mut,       // changeable
        "sammenlign" => TokenKind::Match,      // compare / match
        "bekreft" => TokenKind::Assert,        // confirm (ASCII alt)
        "bevis" => TokenKind::Prove,           // prove
        "krever" => TokenKind::Requires,       // requires
        "garanterer" => TokenKind::Ensures,    // guarantees
        "sant" => TokenKind::True,             // true
        "usant" => TokenKind::False,           // false
        "skriv" => TokenKind::Print,           // write (shared w/ Swedish ASCII)
        "ren" => TokenKind::Pure,              // pure
        "parallell" => TokenKind::Parallel,    // parallel
        "grensesnitt" => TokenKind::Interface, // interface
        "implementer" => TokenKind::Implement, // implement
        "metoder" => TokenKind::Methods,       // methods
        "hvor" => TokenKind::Where,            // where
        "er" => TokenKind::Is,                 // is
        "oppgave" => TokenKind::Task,          // task
        "forene" => TokenKind::Join,           // join
        "usikker" => TokenKind::Unsafe,        // unsafe
        "ekstern" => TokenKind::Extern,        // external
        "uforanderlig" => TokenKind::Invariant, // invariant
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.21 (2026-06-08): Danish (dansk) keyword resolution.
/// Third Nordic dialect. Shares å/æ/ø with Norwegian; distinct
/// keyword choices.
fn danish_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "fortsæt" => TokenKind::Continue,      // continue
        "så" => TokenKind::Then,               // so / then
        "påstå" => TokenKind::Assert,          // claim / assert
        "kræver" => TokenKind::Requires,       // requires
        "grænseflade" => TokenKind::Interface, // interface
        "implementér" => TokenKind::Implement, // implement!
        "prøv" => TokenKind::Try,              // try!
        "forén" => TokenKind::Join,            // join
        "område" => TokenKind::RegionKw,       // area / region
        "formål" => TokenKind::Intent,         // purpose
        "optælling" => TokenKind::Enum,        // enumeration
        "mutérbar" => TokenKind::Mut,          // mutable (alt)
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.21 (2026-06-08): pure-ASCII Danish. Pragma-gated.
fn danish_ascii_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "funktion" => TokenKind::Fn,           // function
        "lad" => TokenKind::Let,               // let
        "struktur" => TokenKind::Struct,       // structure
        "konstant" => TokenKind::Const,        // constant
        "offentlig" => TokenKind::Pub,         // public
        "modul" => TokenKind::Module,          // module
        "brug" => TokenKind::Use,              // use
        "som" => TokenKind::As,                // as
        "returner" => TokenKind::Return,       // return
        "vend" => TokenKind::Return,           // turn (alt)
        "hvis" => TokenKind::If,               // if
        "ellers" => TokenKind::Else,           // else
        "mens" => TokenKind::While,            // while
        "fra" => TokenKind::From,              // from
        "til" => TokenKind::To,                // to
        "bryd" => TokenKind::Break,            // break
        "fortsaet" => TokenKind::Continue,     // continue (no diacritic)
        "saa" => TokenKind::Then,              // then (no å)
        "se" => TokenKind::Ref,                // see
        "foranderlig" => TokenKind::Mut,       // changeable
        "match" => TokenKind::Match,           // match (loanword)
        "bekraeft" => TokenKind::Assert,       // confirm (no æ)
        "paastaa" => TokenKind::Assert,        // (no å alt)
        "bevis" => TokenKind::Prove,           // prove
        "kraever" => TokenKind::Requires,      // (no æ)
        "garanterer" => TokenKind::Ensures,    // guarantees
        "sandt" => TokenKind::True,            // true
        "falsk" => TokenKind::False,           // false
        "udskriv" => TokenKind::Print,         // print out
        "ren" => TokenKind::Pure,              // pure
        "parallel" => TokenKind::Parallel,     // parallel
        "graenseflade" => TokenKind::Interface, // (no æ)
        "implementer" => TokenKind::Implement, // implement
        "metoder" => TokenKind::Methods,       // methods
        "hvor" => TokenKind::Where,            // where
        "er" => TokenKind::Is,                 // is
        "proev" => TokenKind::Try,             // (no ø)
        "opgave" => TokenKind::Task,           // task
        "foren" => TokenKind::Join,            // join (no é)
        "usikker" => TokenKind::Unsafe,        // unsafe
        "omraade" => TokenKind::RegionKw,      // area (no å)
        "formaal" => TokenKind::Intent,        // purpose (no å)
        "type" => TokenKind::Type,             // type (loanword)
        "extern" => TokenKind::Extern,         // external
        "uforanderlig" => TokenKind::Invariant, // invariant
        "optaelling" => TokenKind::Enum,       // (no æ)
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.22 (2026-06-08): Armenian (Հայերեն) keyword
/// resolution. First Caucasus-region script. Block
/// U+0530..058F. SVO grammar.
fn armenian_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "ֆունկցիա" => TokenKind::Fn,           // funktsia (function)
        "թող" => TokenKind::Let,                // togh (let)
        "կառուցվածք" => TokenKind::Struct,     // karutsvatsk (structure)
        "թվարկում" => TokenKind::Enum,         // tvarkum (enumeration)
        "հաստատուն" => TokenKind::Const,       // hastatun (constant)
        "հանրային" => TokenKind::Pub,          // hanrayin (public)
        "մոդուլ" => TokenKind::Module,         // modul (module)
        "օգտագործել" => TokenKind::Use,        // ogtagortsel (use)
        "որպես" => TokenKind::As,              // vorpes (as)
        "վերադարձ" => TokenKind::Return,       // veradarcz (return)
        "եթե" => TokenKind::If,                // yete (if)
        "այլապես" => TokenKind::Else,          // aylapes (otherwise)
        "քանի" => TokenKind::While,            // qani (while)
        "ամեն" => TokenKind::For,              // amen (each / for)
        "մեջ" => TokenKind::In,                // mech (in)
        "ից" => TokenKind::From,               // its (from suffix)
        "մինչև" => TokenKind::To,              // minchev (until)
        "ընդհատել" => TokenKind::Break,        // yndhatel (interrupt)
        "շարունակել" => TokenKind::Continue,   // sharunakel (continue)
        "ապա" => TokenKind::Then,              // apa (then)
        "տեսնել" => TokenKind::Ref,            // tesnel (see)
        "փոփոխական" => TokenKind::Mut,         // popoxakan (variable / mutable)
        "համապատասխանեցնել" => TokenKind::Match, // hamapatasxanetsnel (match)
        "հաստատել" => TokenKind::Assert,       // hastatel (assert)
        "ապացուցել" => TokenKind::Prove,       // apatsutsel (prove)
        "պահանջում" => TokenKind::Requires,    // pahanchum (requires)
        "երաշխավորում" => TokenKind::Ensures,  // yerashxavorum (guarantees)
        "ճշմարիտ" => TokenKind::True,          // tcshmarit (true)
        "կեղծ" => TokenKind::False,            // keltz (false)
        "տպել" => TokenKind::Print,            // tpel (print)
        "մաքուր" => TokenKind::Pure,           // mak'ur (pure)
        "զուգահեռ" => TokenKind::Parallel,     // zugaherr (parallel)
        "միջերես" => TokenKind::Interface,     // mijeres (interface)
        "իրականացնել" => TokenKind::Implement, // irakanatsnel (realize / implement)
        "մեթոդներ" => TokenKind::Methods,      // metodner (methods)
        "որտեղ" => TokenKind::Where,           // vortegh (where)
        "է" => TokenKind::Is,                  // e (is)
        "փորձել" => TokenKind::Try,            // porcel (try)
        "խնդիր" => TokenKind::Task,            // xndir (task)
        "միանալ" => TokenKind::Join,           // mianal (join)
        "անապահով" => TokenKind::Unsafe,       // anapahov (unsafe)
        "տարածք" => TokenKind::RegionKw,       // taratsk (region)
        "նպատակ" => TokenKind::Intent,         // npatak (purpose)
        "տեսակ" => TokenKind::Type,            // tesak (type / kind)
        "արտաքին" => TokenKind::Extern,        // artak'in (external)
        "անփոփոխ" => TokenKind::Invariant,     // anpopox (unchanging)
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.23 (2026-06-08): Georgian (ქართული) keyword
/// resolution. Mkhedruli (lowercase modern Georgian). SVO
/// grammar.
fn georgian_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "ფუნქცია" => TokenKind::Fn,            // punkcia (function)
        "მიეცი" => TokenKind::Let,             // miexi (let / "be given")
        "სტრუქტურა" => TokenKind::Struct,      // strukturra (structure)
        "ჩამოთვლა" => TokenKind::Enum,         // chamotvla (listing / enum)
        "მუდმივი" => TokenKind::Const,         // mudmivi (constant)
        "საჯარო" => TokenKind::Pub,            // sajaro (public)
        "მოდული" => TokenKind::Module,         // moduli (module)
        "გამოყენება" => TokenKind::Use,        // gamokeneba (use)
        "როგორც" => TokenKind::As,             // rogorts (as)
        "დაბრუნება" => TokenKind::Return,      // dabruneba (return)
        "თუ" => TokenKind::If,                 // tu (if)
        "სხვა" => TokenKind::Else,             // sxva (else / other)
        "სანამ" => TokenKind::While,           // sanam (while)
        "თითოეული" => TokenKind::For,          // titoeuli (each)
        "ში" => TokenKind::In,                 // shi (in)
        "დან" => TokenKind::From,              // dan (from suffix)
        "მდე" => TokenKind::To,                // mde (to suffix)
        "შეჩერება" => TokenKind::Break,        // shechereba (halt)
        "გაგრძელება" => TokenKind::Continue,   // gagrdzeleba (continue)
        "მაშინ" => TokenKind::Then,            // mashin (then)
        "ნახე" => TokenKind::Ref,              // naxe (see!)
        "ცვალებადი" => TokenKind::Mut,         // tsvaledadi (changeable)
        "შესაბამისობა" => TokenKind::Match,    // shesabamis oba (correspondence)
        "დაამოწმე" => TokenKind::Assert,       // daamotsme (verify!)
        "დაამტკიცე" => TokenKind::Prove,       // daamtkitse (prove!)
        "მოითხოვს" => TokenKind::Requires,     // moitxovs (requires)
        "უზრუნველყოფს" => TokenKind::Ensures,  // uzrunvelyofs (ensures)
        "ჭეშმარიტი" => TokenKind::True,        // tches_mariti (true)
        "მცდარი" => TokenKind::False,          // mtsdari (false / mistaken)
        "ბეჭდვა" => TokenKind::Print,          // bechdva (printing)
        "სუფთა" => TokenKind::Pure,            // sup'ta (pure / clean)
        "პარალელური" => TokenKind::Parallel,   // paraleluri (parallel)
        "ინტერფეისი" => TokenKind::Interface,  // interfeisi (interface)
        "განხორციელება" => TokenKind::Implement, // ganxortsieleba (implementation)
        "მეთოდები" => TokenKind::Methods,      // metodebi (methods)
        "სად" => TokenKind::Where,             // sad (where)
        "არის" => TokenKind::Is,               // aris (is)
        "სცადე" => TokenKind::Try,             // stsade (try!)
        "დავალება" => TokenKind::Task,         // davaleba (task / assignment)
        "შეერთება" => TokenKind::Join,         // sheert'eba (joining)
        "სახიფათო" => TokenKind::Unsafe,       // saxipato (dangerous)
        "რეგიონი" => TokenKind::RegionKw,      // regioni (region — loanword)
        "მიზანი" => TokenKind::Intent,         // mizani (goal)
        "ტიპი" => TokenKind::Type,             // tipi (type)
        "გარე" => TokenKind::Extern,           // gare (outer)
        "უცვლელი" => TokenKind::Invariant,     // utsvleli (unchanging)
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.16 (2026-06-08): Hungarian (magyar) keyword
/// resolution. Uralic family with distinctive double-acute
/// ő/ű in addition to standard á/é/í/ó/ö/ú/ü.
fn hungarian_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "függvény" => TokenKind::Fn,           // function
        "visszatér" => TokenKind::Return,      // return
        "különben" => TokenKind::Else,         // otherwise
        "amíg" => TokenKind::While,            // while
        "törj" => TokenKind::Break,            // break!
        "nézd" => TokenKind::Ref,              // look!
        "változó" => TokenKind::Mut,           // variable / mutable
        "állítsd" => TokenKind::Assert,        // assert!
        "bizonyítsd" => TokenKind::Prove,      // prove!
        "igényel" => TokenKind::Requires,      // requires
        "garantál" => TokenKind::Ensures,      // guarantees
        "párhuzamos" => TokenKind::Parallel,   // parallel
        "felület" => TokenKind::Interface,     // interface
        "metódusok" => TokenKind::Methods,     // methods
        "próbáld" => TokenKind::Try,           // try!
        "egyesít" => TokenKind::Join,          // join
        "veszélyes" => TokenKind::Unsafe,      // dangerous
        "tartomány" => TokenKind::RegionKw,    // region
        "cél" => TokenKind::Intent,            // goal
        "típus" => TokenKind::Type,            // type
        "külső" => TokenKind::Extern,          // external
        "állandó" => TokenKind::Const,         // constant
        "felsorolás" => TokenKind::Enum,       // enumeration
        "nyilvános" => TokenKind::Pub,         // public
        "használd" => TokenKind::Use,          // use!
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.16 (2026-06-08): pure-ASCII Hungarian. Pragma-gated.
fn hungarian_ascii_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "legyen" => TokenKind::Let,            // "let it be"
        "szerkezet" => TokenKind::Struct,      // structure
        "modul" => TokenKind::Module,          // module
        "mint" => TokenKind::As,               // as
        "ha" => TokenKind::If,                 // if
        "minden" => TokenKind::For,            // every / for each
        "folytasd" => TokenKind::Continue,     // continue!
        "akkor" => TokenKind::Then,            // then
        "egyezzen" => TokenKind::Match,        // match
        "igaz" => TokenKind::True,             // true
        "hamis" => TokenKind::False,           // false
        "nyomtass" => TokenKind::Print,        // print!
        "tiszta" => TokenKind::Pure,           // pure
        "ahol" => TokenKind::Where,            // where
        "van" => TokenKind::Is,                // is / exists
        "feladat" => TokenKind::Task,          // task
        // ASCII no-diacritic alts where applicable:
        "fuggveny" => TokenKind::Fn,           // function (no diacritic)
        "valtozo" => TokenKind::Mut,           // mutable (no diacritic)
        "valositsd_meg" => TokenKind::Implement, // implement
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.17 (2026-06-08): Czech (čeština) keyword resolution.
/// Slavic Latin with distinctive ř + extensive háček diacritics.
fn czech_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "nechť" => TokenKind::Let,             // (subjunctive "let")
        "vrať" => TokenKind::Return,           // return!
        "přeruš" => TokenKind::Break,          // interrupt!
        "pokračuj" => TokenKind::Continue,     // continue!
        "tvrď" => TokenKind::Assert,           // assert!
        "dokaž" => TokenKind::Prove,           // prove!
        "vyžaduje" => TokenKind::Requires,     // requires
        "zajišťuje" => TokenKind::Ensures,     // ensures
        "vypiš" => TokenKind::Print,           // write out
        "čistý" => TokenKind::Pure,            // pure
        "paralelní" => TokenKind::Parallel,    // parallel
        "rozhraní" => TokenKind::Interface,    // interface
        "úloha" => TokenKind::Task,            // task
        "nebezpečný" => TokenKind::Unsafe,     // unsafe / dangerous
        "záměr" => TokenKind::Intent,          // intent
        "vnější" => TokenKind::Extern,         // external
        "neměnný" => TokenKind::Invariant,     // invariant
        "výčet" => TokenKind::Enum,            // enumeration
        "veřejný" => TokenKind::Pub,           // public
        "použij" => TokenKind::Use,            // use!
        "proměnný" => TokenKind::Mut,          // variable / mutable
        "odpovídej" => TokenKind::Match,       // correspond! / match
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.17 (2026-06-08): pure-ASCII Czech. Pragma-gated.
fn czech_ascii_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "funkce" => TokenKind::Fn,             // function
        "struktura" => TokenKind::Struct,      // structure
        "konstanta" => TokenKind::Const,       // constant
        "modul" => TokenKind::Module,          // module
        "jako" => TokenKind::As,               // as
        "pokud" => TokenKind::If,              // if
        "jestli" => TokenKind::If,             // if (alt)
        "jinak" => TokenKind::Else,            // else / otherwise
        "dokud" => TokenKind::While,           // while
        "pro" => TokenKind::For,               // for
        "od" => TokenKind::From,               // from
        "do" => TokenKind::To,                 // to
        "pak" => TokenKind::Then,              // then
        "viz" => TokenKind::Ref,               // see
        "pravda" => TokenKind::True,           // true
        "nepravda" => TokenKind::False,        // false
        "tiskni" => TokenKind::Print,          // print!
        "metody" => TokenKind::Methods,        // methods
        "implementuj" => TokenKind::Implement, // implement!
        "kde" => TokenKind::Where,             // where
        "je" => TokenKind::Is,                 // is
        "zkus" => TokenKind::Try,              // try!
        "spoj" => TokenKind::Join,             // join
        "oblast" => TokenKind::RegionKw,       // area / region
        "typ" => TokenKind::Type,              // type
        // No-diacritic alts:
        "nechtt" => TokenKind::Let,            // (rare alt)
        "vrat" => TokenKind::Return,           // return (no diacritic)
        "prerus" => TokenKind::Break,          // interrupt (no diacritic)
        "pokracuj" => TokenKind::Continue,     // (no diacritic)
        "tvrd" => TokenKind::Assert,           // (no diacritic)
        "dokaz" => TokenKind::Prove,           // (no diacritic)
        "vyzaduje" => TokenKind::Requires,     // (no diacritic)
        "zajistuje" => TokenKind::Ensures,     // (no diacritic)
        "vypis" => TokenKind::Print,           // (no diacritic alt)
        "cisty" => TokenKind::Pure,            // (no diacritic)
        "paralelni" => TokenKind::Parallel,    // (no diacritic)
        "rozhrani" => TokenKind::Interface,    // (no diacritic)
        "uloha" => TokenKind::Task,            // (no diacritic)
        "nebezpecny" => TokenKind::Unsafe,     // (no diacritic)
        "zamer" => TokenKind::Intent,          // (no diacritic)
        "vnejsi" => TokenKind::Extern,         // (no diacritic)
        "nemenny" => TokenKind::Invariant,     // (no diacritic)
        "vycet" => TokenKind::Enum,            // (no diacritic)
        "verejny" => TokenKind::Pub,           // (no diacritic)
        "pouzij" => TokenKind::Use,            // (no diacritic)
        "promenny" => TokenKind::Mut,          // (no diacritic)
        "odpovidej" => TokenKind::Match,       // (no diacritic)
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.18 (2026-06-08): Swedish (svenska) keyword
/// resolution. First Nordic dialect; uses å/ä/ö.
fn swedish_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "låt" => TokenKind::Let,               // let
        "återvänd" => TokenKind::Return,       // return!
        "för" => TokenKind::For,               // for
        "från" => TokenKind::From,             // from
        "fortsätt" => TokenKind::Continue,     // continue!
        "så" => TokenKind::Then,               // then / so
        "föränderlig" => TokenKind::Mut,       // changeable
        "påstå" => TokenKind::Assert,          // claim / assert
        "kräver" => TokenKind::Requires,       // requires
        "säkerställer" => TokenKind::Ensures,  // ensures
        "gränssnitt" => TokenKind::Interface,  // interface
        "där" => TokenKind::Where,             // where
        "är" => TokenKind::Is,                 // is
        "försök" => TokenKind::Try,            // try!
        "förena" => TokenKind::Join,           // join
        "osäker" => TokenKind::Unsafe,         // unsafe
        "oföränderlig" => TokenKind::Invariant, // unchanging
        "uppräkning" => TokenKind::Enum,       // enumeration
        "använd" => TokenKind::Use,            // use!
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.18 (2026-06-08): pure-ASCII Swedish. Pragma-gated.
fn swedish_ascii_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "funktion" => TokenKind::Fn,           // function
        "vara" => TokenKind::Let,              // be
        "struktur" => TokenKind::Struct,       // structure
        "konstant" => TokenKind::Const,        // constant
        "offentlig" => TokenKind::Pub,         // public
        "modul" => TokenKind::Module,          // module
        "som" => TokenKind::As,                // as
        "om" => TokenKind::If,                 // if
        "annars" => TokenKind::Else,           // otherwise
        "medan" => TokenKind::While,           // while
        "till" => TokenKind::To,               // to
        "bryt" => TokenKind::Break,            // break
        "se" => TokenKind::Ref,                // see
        "matcha" => TokenKind::Match,          // match
        "bevisa" => TokenKind::Prove,          // prove
        "sant" => TokenKind::True,             // true
        "falskt" => TokenKind::False,          // false
        "skriv" => TokenKind::Print,           // write
        "ren" => TokenKind::Pure,              // pure
        "parallell" => TokenKind::Parallel,    // parallel
        "implementera" => TokenKind::Implement, // implement
        "metoder" => TokenKind::Methods,       // methods
        "uppgift" => TokenKind::Task,          // task
        "syfte" => TokenKind::Intent,          // purpose
        "typ" => TokenKind::Type,              // type
        "extern" => TokenKind::Extern,         // external
        // No-diacritic alts:
        "lat" => TokenKind::Let,               // let (no å)
        "atervand" => TokenKind::Return,       // return (no å/ä)
        "fortsatt" => TokenKind::Continue,     // continue (no ä)
        "sa" => TokenKind::Then,               // then (no å)
        "foranderlig" => TokenKind::Mut,       // (no ö/ä)
        "krever" => TokenKind::Requires,       // (no ä) — non-standard but plausible
        "der" => TokenKind::Where,             // where (no ä)
        "ar" => TokenKind::Is,                 // is (no ä) — single-ish
        "forsok" => TokenKind::Try,            // (no ö)
        "forena" => TokenKind::Join,           // (no ö)
        "osaker" => TokenKind::Unsafe,         // (no ä)
        "uppraekning" => TokenKind::Enum,      // (no ä — ae replacement)
        "anvand" => TokenKind::Use,            // (no ä)
        "for" => TokenKind::For,               // (no ö — could conflict; pragma-gated safe)
        "fran" => TokenKind::From,             // (no å)
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.19 (2026-06-08): Filipino / Tagalog keyword
/// resolution. Austronesian basic-Latin dialect. Pure-ASCII so
/// fully pragma-gated.
fn filipino_ascii_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "gawain" => TokenKind::Fn,             // work / function
        "hayaan" => TokenKind::Let,            // let
        "istraktura" => TokenKind::Struct,     // structure
        "pagbilang" => TokenKind::Enum,        // enumeration
        "pirme" => TokenKind::Const,           // constant
        "pampubliko" => TokenKind::Pub,        // public
        "modyul" => TokenKind::Module,         // module
        "gamitin" => TokenKind::Use,           // use!
        "bilang" => TokenKind::As,             // as
        "ibalik" => TokenKind::Return,         // return!
        "kung" => TokenKind::If,               // if
        "kundi" => TokenKind::Else,            // else
        "habang" => TokenKind::While,          // while
        "sa" => TokenKind::In,                 // in
        "mula" => TokenKind::From,             // from
        "hanggang" => TokenKind::To,           // until
        "tumigil" => TokenKind::Break,         // stop
        "magpatuloy" => TokenKind::Continue,   // continue!
        "tingnan" => TokenKind::Ref,           // look at
        "nababago" => TokenKind::Mut,          // changeable
        "tugmain" => TokenKind::Match,         // match!
        "patunayan" => TokenKind::Assert,      // verify!
        "ipakita" => TokenKind::Prove,         // show / prove
        "kailangan" => TokenKind::Requires,    // needs
        "tiyakin" => TokenKind::Ensures,       // ensure
        "totoo" => TokenKind::True,            // true
        "mali" => TokenKind::False,            // wrong / false
        "isulat" => TokenKind::Print,          // write
        "dalisay" => TokenKind::Pure,          // pure
        "magkatulad" => TokenKind::Parallel,   // parallel / similar
        "ipatupad" => TokenKind::Implement,    // implement!
        "pamamaraan" => TokenKind::Methods,    // methods
        "saan" => TokenKind::Where,            // where
        "ay" => TokenKind::Is,                 // is
        "subukan" => TokenKind::Try,           // try!
        "tungkulin" => TokenKind::Task,        // task / duty
        "pagsama" => TokenKind::Join,          // join
        "mapanganib" => TokenKind::Unsafe,     // dangerous
        "rehiyon" => TokenKind::RegionKw,      // region
        "layunin" => TokenKind::Intent,        // purpose
        "uri" => TokenKind::Type,              // type / kind
        "panlabas" => TokenKind::Extern,       // outside
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.12 (2026-06-08): Vietnamese (Tiếng Việt) keyword
/// resolution. First Southeast Asian Latin-script dialect.
/// Distinctive diacritic + tone-mark combinations
/// (ă/â/đ/ê/ô/ơ/ư + 5 tones). This table holds natural
/// accented forms; the pure-ASCII fallbacks live in
/// `vietnamese_ascii_keyword`.
fn vietnamese_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "hàm" => TokenKind::Fn,               // (function)
        "trả_về" => TokenKind::Return,        // (return value)
        "nếu" => TokenKind::If,               // (if)
        "ngược_lại" => TokenKind::Else,       // (otherwise)
        "trong_khi" => TokenKind::While,      // (while)
        "với_mỗi" => TokenKind::For,          // (for each)
        "từ" => TokenKind::From,              // (from)
        "đến" => TokenKind::To,               // (to)
        "ngắt" => TokenKind::Break,           // (break)
        "tiếp_tục" => TokenKind::Continue,    // (continue)
        "thì" => TokenKind::Then,             // (then)
        "tham_chiếu" => TokenKind::Ref,       // (reference)
        "có_thể_thay_đổi" => TokenKind::Mut,  // (changeable)
        "khớp" => TokenKind::Match,           // (match)
        "khẳng_định" => TokenKind::Assert,    // (assert)
        "chứng_minh" => TokenKind::Prove,     // (prove)
        "yêu_cầu" => TokenKind::Requires,     // (requires)
        "đảm_bảo" => TokenKind::Ensures,      // (ensures)
        "đúng" => TokenKind::True,            // (true / correct)
        "giao_diện" => TokenKind::Interface,  // (interface)
        "phương_thức" => TokenKind::Methods,  // (methods)
        "ở_đâu" => TokenKind::Where,          // (where)
        "là" => TokenKind::Is,                // (is)
        "thử" => TokenKind::Try,              // (try)
        "công_việc" => TokenKind::Task,       // (task)
        "kết_hợp" => TokenKind::Join,         // (join)
        "không_an_toàn" => TokenKind::Unsafe, // (unsafe)
        "vùng" => TokenKind::RegionKw,        // (region)
        "mục_đích" => TokenKind::Intent,      // (purpose)
        "kiểu" => TokenKind::Type,            // (type)
        "bất_biến" => TokenKind::Invariant,   // (invariant)
        "công_khai" => TokenKind::Pub,        // (public)
        "mô_đun" => TokenKind::Module,        // (module)
        "cấu_trúc" => TokenKind::Struct,      // (structure)
        "liệt_kê" => TokenKind::Enum,         // (enumeration)
        "hằng" => TokenKind::Const,           // (constant)
        "thuần_túy" => TokenKind::Pure,       // (pure)
        "song_song" => TokenKind::Parallel,   // (parallel)
        "triển_khai" => TokenKind::Implement, // (implement)
        "sử_dụng" => TokenKind::Use,          // (use)
        "như" => TokenKind::As,               // (as)
        "đặt" => TokenKind::Let,              // (set / let)
        "sai" => TokenKind::False,            // (false / wrong)
        "bên_ngoài" => TokenKind::Extern,     // (external)
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.12 (2026-06-08): pure-ASCII Vietnamese keyword
/// table. Vietnamese keywords are mostly non-ASCII (extensive
/// tone+diacritic marks), but `in_ra` (print) is one of the few
/// natural pure-ASCII keywords. Pragma-gated.
fn vietnamese_ascii_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "in_ra" => TokenKind::Print,          // print (pure-ASCII Vietnamese)
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.13 (2026-06-08): Romanian (limba română) keyword
/// resolution. Distinctive ă/â/î/ș/ț diacritics. This table
/// holds non-ASCII forms; pure-ASCII fallbacks are
/// `romanian_ascii_keyword`.
fn romanian_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "funcție" => TokenKind::Fn,           // (function)
        "întoarce" => TokenKind::Return,      // (return)
        "dacă" => TokenKind::If,              // (if)
        "altfel" => TokenKind::Else,          // (else) — also pure-ASCII, registered here for completeness
        "cât_timp" => TokenKind::While,       // (while; multi-word)
        "pentru" => TokenKind::For,           // (for) — ASCII, but listed
        "până" => TokenKind::To,              // (until)
        "rupe" => TokenKind::Break,           // (break) — ASCII
        "continuă" => TokenKind::Continue,    // (continue!)
        "atunci" => TokenKind::Then,          // (then) — ASCII
        "vezi" => TokenKind::Ref,             // (see) — ASCII
        "schimbabil" => TokenKind::Mut,       // (changeable) — ASCII
        "potrivește" => TokenKind::Match,     // (match!)
        "afirmă" => TokenKind::Assert,        // (assert!)
        "dovedește" => TokenKind::Prove,      // (prove!)
        "necesită" => TokenKind::Requires,    // (requires)
        "garantează" => TokenKind::Ensures,   // (ensures)
        "adevărat" => TokenKind::True,        // (true)
        "fals" => TokenKind::False,           // (false) — ASCII
        "tipărește" => TokenKind::Print,      // (print!)
        "scrie" => TokenKind::Print,          // (write — ASCII alt)
        "interfață" => TokenKind::Interface,  // (interface)
        "implementează" => TokenKind::Implement, // (implement!)
        "metode" => TokenKind::Methods,       // (methods) — ASCII
        "unde" => TokenKind::Where,           // (where) — ASCII
        "este" => TokenKind::Is,              // (is) — ASCII
        "încearcă" => TokenKind::Try,         // (try!)
        "sarcină" => TokenKind::Task,         // (task)
        "unește" => TokenKind::Join,          // (join!)
        "nesigur" => TokenKind::Unsafe,       // (unsafe) — ASCII
        "regiune" => TokenKind::RegionKw,     // (region) — ASCII
        "scop" => TokenKind::Intent,          // (goal) — ASCII
        "tip" => TokenKind::Type,             // (type) — ASCII
        "extern" => TokenKind::Extern,        // (external) — ASCII, shared with English alias
        "invariant" => TokenKind::Invariant,  // (invariant) — ASCII
        "structură" => TokenKind::Struct,     // (structure)
        "enumerare" => TokenKind::Enum,       // (enumeration)
        "constantă" => TokenKind::Const,      // (constant)
        "public" => TokenKind::Pub,           // (public) — ASCII
        "modul" => TokenKind::Module,         // (module) — ASCII
        "folosește" => TokenKind::Use,        // (use!)
        "ca" => TokenKind::As,                // (as) — ASCII
        "fie" => TokenKind::Let,              // (let it be) — ASCII
        "pur" => TokenKind::Pure,             // (pure) — ASCII
        "paralel" => TokenKind::Parallel,     // (parallel) — ASCII
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.13 (2026-06-08): pure-ASCII Romanian keyword table.
/// Pragma-gated. Many Romanian keywords are inherently ASCII
/// so this table is intentionally smaller — many entries above
/// in `romanian_keyword` are already ASCII and reachable via
/// either path (they just need pragma gating to fire as
/// keywords).
fn romanian_ascii_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        // ASCII versions and no-diacritic alternates:
        "functie" => TokenKind::Fn,           // function (no diacritic)
        "intoarce" => TokenKind::Return,      // return (no diacritic)
        "daca" => TokenKind::If,              // if (no diacritic)
        "altfel" => TokenKind::Else,
        "cat_timp" => TokenKind::While,       // while (no diacritic)
        "pentru" => TokenKind::For,
        "pana" => TokenKind::To,              // until (no diacritic)
        "rupe" => TokenKind::Break,
        "continua" => TokenKind::Continue,    // (no diacritic alt)
        "atunci" => TokenKind::Then,
        "vezi" => TokenKind::Ref,
        "schimbabil" => TokenKind::Mut,
        "potriveste" => TokenKind::Match,     // (no diacritic)
        "afirma" => TokenKind::Assert,        // (no diacritic)
        "dovedeste" => TokenKind::Prove,      // (no diacritic)
        "necesita" => TokenKind::Requires,    // (no diacritic)
        "garanteaza" => TokenKind::Ensures,   // (no diacritic)
        "adevarat" => TokenKind::True,        // (no diacritic)
        "fals" => TokenKind::False,
        "tipareste" => TokenKind::Print,      // (no diacritic)
        "scrie" => TokenKind::Print,
        "interfata" => TokenKind::Interface,  // (no diacritic)
        "implementeaza" => TokenKind::Implement, // (no diacritic)
        "metode" => TokenKind::Methods,
        "unde" => TokenKind::Where,
        "este" => TokenKind::Is,
        "incearca" => TokenKind::Try,         // (no diacritic)
        "sarcina" => TokenKind::Task,         // (no diacritic)
        "uneste" => TokenKind::Join,          // (no diacritic)
        "nesigur" => TokenKind::Unsafe,
        "regiune" => TokenKind::RegionKw,
        "scop" => TokenKind::Intent,
        "tip" => TokenKind::Type,
        "invariant" => TokenKind::Invariant,
        "structura" => TokenKind::Struct,     // (no diacritic)
        "enumerare" => TokenKind::Enum,
        "constanta" => TokenKind::Const,      // (no diacritic)
        "public" => TokenKind::Pub,
        "modul" => TokenKind::Module,
        "foloseste" => TokenKind::Use,        // (no diacritic)
        "ca" => TokenKind::As,
        "fie" => TokenKind::Let,
        "pur" => TokenKind::Pure,
        "paralel" => TokenKind::Parallel,
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.14 (2026-06-08): Dutch (Nederlands) keyword
/// resolution. Basic-Latin Germanic. Mostly pure-ASCII surface.
fn dutch_ascii_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "functie" => TokenKind::Fn,           // function
        "laat" => TokenKind::Let,             // let
        "structuur" => TokenKind::Struct,     // structure
        "opsomming" => TokenKind::Enum,       // enumeration
        "constante" => TokenKind::Const,      // constant
        "openbaar" => TokenKind::Pub,         // public / open
        "module" => TokenKind::Module,        // module
        "gebruik" => TokenKind::Use,          // use
        "als" => TokenKind::As,               // as
        "terug" => TokenKind::Return,         // back / return
        "indien" => TokenKind::If,            // if
        "anders" => TokenKind::Else,          // otherwise
        "zolang" => TokenKind::While,         // as long as / while
        "voor" => TokenKind::For,             // for
        "in" => TokenKind::In,                // in (same as English)
        "van" => TokenKind::From,             // from
        "tot" => TokenKind::To,               // to / until
        "stop" => TokenKind::Break,           // stop
        "verder" => TokenKind::Continue,      // continue
        "dan" => TokenKind::Then,             // then
        "zie" => TokenKind::Ref,              // see
        "veranderlijk" => TokenKind::Mut,     // changeable
        "vergelijk" => TokenKind::Match,      // compare / match
        "bevestig" => TokenKind::Assert,      // confirm / assert
        "bewijs" => TokenKind::Prove,         // prove
        "vereist" => TokenKind::Requires,     // requires
        "verzekert" => TokenKind::Ensures,    // ensures
        "waar" => TokenKind::True,            // true
        "onwaar" => TokenKind::False,         // false / untrue
        "druk" => TokenKind::Print,           // print
        "schrijf" => TokenKind::Print,        // write (alt)
        "zuiver" => TokenKind::Pure,          // pure
        "parallel" => TokenKind::Parallel,    // parallel
        "interface" => TokenKind::Interface,  // interface
        "implementeer" => TokenKind::Implement, // implement
        "methoden" => TokenKind::Methods,     // methods
        "waar_is" => TokenKind::Where,        // where_is (avoid `waar` collision with True)
        "is" => TokenKind::Is,                // is (same as English)
        "probeer" => TokenKind::Try,          // try
        "taak" => TokenKind::Task,            // task
        "verbind" => TokenKind::Join,         // connect / join
        "onveilig" => TokenKind::Unsafe,      // unsafe
        "gebied" => TokenKind::RegionKw,      // region / area
        "doel" => TokenKind::Intent,          // goal
        "type" => TokenKind::Type,            // type (loanword)
        "extern" => TokenKind::Extern,        // external (same as English alias)
        "invariant" => TokenKind::Invariant,  // invariant (same)
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.15 (2026-06-08): Thai (ไทย) keyword resolution.
/// First Thai-script dialect. SVO grammar; all keywords start
/// with Thai-block codepoints so they route through
/// `lex_unicode_ident`.
fn thai_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "ฟังก์ชัน" => TokenKind::Fn,         // fangkchan (function)
        "ให้" => TokenKind::Let,              // hai (let / give)
        "โครงสร้าง" => TokenKind::Struct,    // khrongsang (structure)
        "การแจงนับ" => TokenKind::Enum,      // kan jaengnap (enumeration)
        "คงที่" => TokenKind::Const,         // khongthi (fixed / constant)
        "สาธารณะ" => TokenKind::Pub,         // satarana (public)
        "โมดูล" => TokenKind::Module,        // modun (module — loanword)
        "ใช้" => TokenKind::Use,             // chai (use)
        "เป็น" => TokenKind::As,             // pen (as / is)
        "คืน" => TokenKind::Return,          // khuen (return!)
        "ถ้า" => TokenKind::If,              // tha (if)
        "ไม่เช่นนั้น" => TokenKind::Else,    // maichennan (otherwise)
        "ขณะที่" => TokenKind::While,        // khanathi (while)
        "สำหรับ" => TokenKind::For,          // samrap (for)
        "ใน" => TokenKind::In,               // nai (in)
        "จาก" => TokenKind::From,            // chak (from)
        "ถึง" => TokenKind::To,              // thueng (to / until)
        "หยุด" => TokenKind::Break,          // yut (stop)
        "ดำเนินต่อ" => TokenKind::Continue,  // damnoen to (continue)
        "แล้ว" => TokenKind::Then,           // laeo (then / already)
        "ดู" => TokenKind::Ref,              // du (see)
        "เปลี่ยนแปลงได้" => TokenKind::Mut,  // plian plaeng dai (changeable)
        "ตรงกัน" => TokenKind::Match,        // trongkan (match)
        "ยืนยัน" => TokenKind::Assert,       // yuenyan (confirm)
        "พิสูจน์" => TokenKind::Prove,       // phisut (prove)
        "ต้องการ" => TokenKind::Requires,    // tongkan (requires)
        "รับประกัน" => TokenKind::Ensures,   // rapprakan (guarantees)
        "จริง" => TokenKind::True,           // jing (true)
        "เท็จ" => TokenKind::False,          // thet (false)
        "พิมพ์" => TokenKind::Print,         // phim (print)
        "บริสุทธิ์" => TokenKind::Pure,      // borisut (pure)
        "ขนาน" => TokenKind::Parallel,       // khanan (parallel)
        "อินเทอร์เฟซ" => TokenKind::Interface, // interface (loanword)
        "ดำเนินการ" => TokenKind::Implement, // damnoen kan (execute / implement)
        "วิธีการ" => TokenKind::Methods,     // withikan (methods)
        "ที่ไหน" => TokenKind::Where,        // thinai (where)
        "คือ" => TokenKind::Is,              // khue (is)
        "ลอง" => TokenKind::Try,             // long (try)
        "งาน" => TokenKind::Task,            // ngan (task / work)
        "รวม" => TokenKind::Join,            // ruam (join / combine)
        "ไม่ปลอดภัย" => TokenKind::Unsafe,   // mai plotphai (unsafe)
        "พื้นที่" => TokenKind::RegionKw,    // phuenthi (area / region)
        "จุดประสงค์" => TokenKind::Intent,   // chut prasong (purpose)
        "ชนิด" => TokenKind::Type,           // chanit (kind / type)
        "ภายนอก" => TokenKind::Extern,       // phainok (external)
        "ไม่เปลี่ยน" => TokenKind::Invariant, // mai plian (invariant)
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.8 (2026-06-08): Polish (polski) keyword resolution.
/// Sixth Latin-with-accents Tier II dialect, first Slavic Latin
/// variant. This table holds the natural non-ASCII keyword
/// forms (uses ą/ć/ę/ł/ń/ó/ś/ź/ż); pure-ASCII forms live in
/// `polish_ascii_keyword` and are pragma-gated.
fn polish_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "wróć" => TokenKind::Return,          // wruć (return)
        "zwróć" => TokenKind::Return,         // zwruć (return)
        "jeśli" => TokenKind::If,             // jeshli (if)
        "dopóki" => TokenKind::While,         // dopuki (while / as long as)
        "potwierdź" => TokenKind::Assert,     // potvyerdż (confirm)
        "fałsz" => TokenKind::False,          // fawsh (false)
        "równoległy" => TokenKind::Parallel,  // ruvnowegwy (parallel)
        "spróbuj" => TokenKind::Try,          // spruboy (try!)
        "połącz" => TokenKind::Join,          // powunch (join!)
        "zewnętrzny" => TokenKind::Extern,    // zevnentshny (external)
        "stała" => TokenKind::Const,          // stawa (constant)
        "moduł" => TokenKind::Module,         // moduw (module)
        "użyj" => TokenKind::Use,             // uzhyy (use!)
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.8 (2026-06-08): pure-ASCII Polish keyword table.
/// Pragma-gated.
fn polish_ascii_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        // === DECLARATIONS ===
        "funkcja" => TokenKind::Fn,           // function
        "niech" => TokenKind::Let,            // let
        "struktura" => TokenKind::Struct,     // structure
        "wyliczenie" => TokenKind::Enum,      // enumeration
        "stala" => TokenKind::Const,          // constant (no diacritic alt)
        // === VISIBILITY / MODULES ===
        "publiczny" => TokenKind::Pub,        // public
        "modul" => TokenKind::Module,         // module (no diacritic alt)
        "uzyj" => TokenKind::Use,             // use (no diacritic alt)
        "jako" => TokenKind::As,              // as
        // === CONTROL FLOW ===
        "zwroc" => TokenKind::Return,         // return (no diacritic alt)
        "wroc" => TokenKind::Return,          // return (no diacritic alt)
        "jesli" => TokenKind::If,             // if (no diacritic alt)
        "inaczej" => TokenKind::Else,         // else / otherwise
        "dopoki" => TokenKind::While,         // while (no diacritic alt)
        "dla" => TokenKind::For,              // for
        "w" => TokenKind::In,                 // in (single char — pragma-gated safe)
        "od" => TokenKind::From,              // from
        "przerwij" => TokenKind::Break,       // break
        "kontynuuj" => TokenKind::Continue,   // continue
        "wtedy" => TokenKind::Then,           // then
        // === REFS / MUT ===
        "zobacz" => TokenKind::Ref,           // see
        "zmienny" => TokenKind::Mut,          // variable / mutable
        // === MATCH ===
        "dopasuj" => TokenKind::Match,        // match!
        // === VERIFICATION ===
        "potwierdz" => TokenKind::Assert,     // confirm (no diacritic alt)
        "udowodnij" => TokenKind::Prove,      // prove!
        "wymaga" => TokenKind::Requires,      // requires
        "gwarantuje" => TokenKind::Ensures,   // guarantees
        // === BOOL / PRINT ===
        "prawda" => TokenKind::True,          // true / truth
        "falsz" => TokenKind::False,          // false (no diacritic alt)
        "drukuj" => TokenKind::Print,         // print!
        "wypisz" => TokenKind::Print,         // write out (alt)
        // === PURITY / PARALLEL ===
        "czysty" => TokenKind::Pure,          // pure
        "rownolegly" => TokenKind::Parallel,  // parallel (no diacritic alt)
        // === INTERFACES / METHODS ===
        "interfejs" => TokenKind::Interface,  // interface
        "zaimplementuj" => TokenKind::Implement, // implement!
        "metody" => TokenKind::Methods,       // methods
        // === BOUNDS ===
        "gdzie" => TokenKind::Where,          // where
        "jest" => TokenKind::Is,              // is
        // === CONCURRENCY ===
        "sprobuj" => TokenKind::Try,          // try (no diacritic alt)
        "zadanie" => TokenKind::Task,         // task
        "polacz" => TokenKind::Join,          // join (no diacritic alt)
        // === EMBEDDED ===
        "niebezpieczny" => TokenKind::Unsafe, // unsafe
        // === SOV-S7 PARITY ===
        "cel" => TokenKind::Intent,           // goal
        "intencja" => TokenKind::Intent,      // intent (alt)
        "typ" => TokenKind::Type,             // type
        "zewnetrzny" => TokenKind::Extern,    // external (no diacritic alt)
        "niezmienny" => TokenKind::Invariant, // invariant
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.9 (2026-06-08): Turkish (Türkçe) keyword resolution.
/// Seventh Latin-with-accents Tier II dialect, Turkic family.
/// Distinctive dotless ı / dotted İ + ç/ğ/ö/ş/ü diacritics.
fn turkish_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "işlev" => TokenKind::Fn,             // ishlev (function)
        "dön" => TokenKind::Return,           // don (return!)
        "döndür" => TokenKind::Return,        // dondur (return - causative)
        "eğer" => TokenKind::If,              // eyer (if)
        "için" => TokenKind::For,             // ichin (for)
        "içinde" => TokenKind::In,            // ichinde (inside)
        "kır" => TokenKind::Break,            // kir (break!)
        "gör" => TokenKind::Ref,              // gor (see)
        "değişken" => TokenKind::Mut,         // deyishken (variable)
        "eşle" => TokenKind::Match,           // eshle (match!)
        "doğrula" => TokenKind::Assert,       // dogrula (verify!)
        "kanıtla" => TokenKind::Prove,        // kanitla (prove!)
        "doğru" => TokenKind::True,           // dogru (true / correct)
        "yanlış" => TokenKind::False,         // yanlish (false / wrong)
        "yazdır" => TokenKind::Print,         // yazdir (write out!)
        "arayüz" => TokenKind::Interface,     // arayuz (interface)
        "görev" => TokenKind::Task,           // gorev (task)
        "birleştir" => TokenKind::Join,       // birleshtir (join!)
        "güvensiz" => TokenKind::Unsafe,      // guvensiz (unsafe)
        "bölge" => TokenKind::RegionKw,       // bolge (region)
        "amaç" => TokenKind::Intent,          // amach (goal)
        "dış" => TokenKind::Extern,           // dish (external)
        "değişmez" => TokenKind::Invariant,   // deyishmez (unchanging)
        "yapı" => TokenKind::Struct,          // yapi (structure)
        "sıralama" => TokenKind::Enum,        // siralama (enumeration)
        "modül" => TokenKind::Module,         // modul (module)
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.9 (2026-06-08): pure-ASCII Turkish keyword table.
/// Pragma-gated.
fn turkish_ascii_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        // === DECLARATIONS ===
        "fonksiyon" => TokenKind::Fn,         // function (loanword alt)
        "olsun" => TokenKind::Let,            // "let it be"
        "yapi" => TokenKind::Struct,          // structure (no diacritic)
        "siralama" => TokenKind::Enum,        // enumeration (no diacritic)
        "sabit" => TokenKind::Const,          // constant / fixed
        // === VISIBILITY / MODULES ===
        "genel" => TokenKind::Pub,            // general / public
        "modul" => TokenKind::Module,         // module (no diacritic)
        "kullan" => TokenKind::Use,           // use
        "olarak" => TokenKind::As,            // as
        // === CONTROL FLOW ===
        "geri" => TokenKind::Return,          // back (alt simplification)
        "don" => TokenKind::Return,           // return (no diacritic)
        "yoksa" => TokenKind::Else,           // else / otherwise
        "iken" => TokenKind::While,           // while
        "icin" => TokenKind::For,             // for (no diacritic)
        "den" => TokenKind::From,             // from
        "kadar" => TokenKind::To,             // until
        "kir" => TokenKind::Break,            // break (no diacritic)
        "devam" => TokenKind::Continue,       // continue
        // === REFS / MUT ===
        "degisken" => TokenKind::Mut,         // mutable (no diacritic)
        // === MATCH ===
        "esle" => TokenKind::Match,           // match (no diacritic)
        // === VERIFICATION ===
        "dogrula" => TokenKind::Assert,       // assert (no diacritic)
        "kanitla" => TokenKind::Prove,        // prove (no diacritic)
        "gerek" => TokenKind::Requires,       // requires
        "garanti" => TokenKind::Ensures,      // guarantee (loanword)
        // === BOOL / PRINT ===
        "dogru" => TokenKind::True,           // true (no diacritic)
        "yanlis" => TokenKind::False,         // false (no diacritic)
        "yazdir" => TokenKind::Print,         // print (no diacritic)
        // === PURITY / PARALLEL ===
        "saf" => TokenKind::Pure,             // pure
        "paralel" => TokenKind::Parallel,     // parallel
        // === INTERFACES / METHODS ===
        "arayuz" => TokenKind::Interface,     // interface (no diacritic)
        "uygula" => TokenKind::Implement,     // implement
        "metotlar" => TokenKind::Methods,     // methods
        // === BOUNDS ===
        "nerede" => TokenKind::Where,         // where
        "olur" => TokenKind::Is,              // is / becomes
        // === CONCURRENCY ===
        "dene" => TokenKind::Try,             // try
        "gorev" => TokenKind::Task,           // task (no diacritic)
        // === EMBEDDED ===
        "guvensiz" => TokenKind::Unsafe,      // unsafe (no diacritic)
        "bolge" => TokenKind::RegionKw,       // region (no diacritic)
        // === SOV-S7 PARITY ===
        "amac" => TokenKind::Intent,          // goal (no diacritic)
        "tip" => TokenKind::Type,             // type
        "dis" => TokenKind::Extern,           // external (no diacritic)
        "degismez" => TokenKind::Invariant,   // invariant (no diacritic)
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.10 (2026-06-08): Malay (Bahasa Melayu) keyword
/// resolution. Second basic-Latin Tier II dialect after
/// Indonesian. Linguistically closely related to Indonesian
/// (mutually intelligible at the spoken level) but has its
/// own keyword preferences and a distinct standardized form.
fn malay_ascii_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        // === DECLARATIONS ===
        "fungsi" => TokenKind::Fn,            // function (shared with Indonesian)
        "biarkan" => TokenKind::Let,          // let it be
        "struktur" => TokenKind::Struct,      // structure
        "penghitungan" => TokenKind::Enum,    // enumeration
        "pemalar" => TokenKind::Const,        // constant (Malay distinct from Indo)
        // === VISIBILITY / MODULES ===
        "awam" => TokenKind::Pub,             // public (Malay)
        "modul" => TokenKind::Module,         // module
        "guna" => TokenKind::Use,             // use
        "sebagai" => TokenKind::As,           // as
        // === CONTROL FLOW ===
        "kembali" => TokenKind::Return,       // return
        "jika" => TokenKind::If,              // if
        "selainnya" => TokenKind::Else,       // else
        "selama" => TokenKind::While,         // while
        "untuk" => TokenKind::For,            // for
        "dalam" => TokenKind::In,             // in
        "dari" => TokenKind::From,            // from
        "hingga" => TokenKind::To,            // until
        "berhenti" => TokenKind::Break,       // stop
        "teruskan" => TokenKind::Continue,    // continue (Malay)
        "maka" => TokenKind::Then,            // then
        // === REFS / MUT ===
        "lihat" => TokenKind::Ref,            // see
        "berubah" => TokenKind::Mut,          // changing
        // === MATCH ===
        "padan" => TokenKind::Match,          // match (Malay)
        // === VERIFICATION ===
        "pastikan" => TokenKind::Assert,      // make sure
        "buktikan" => TokenKind::Prove,       // prove
        "memerlukan" => TokenKind::Requires,  // requires (Malay)
        "menjamin" => TokenKind::Ensures,     // guarantees (Malay)
        // === BOOL / PRINT ===
        "benar" => TokenKind::True,           // true
        "palsu" => TokenKind::False,          // false (Malay; Indo uses `salah`)
        "cetak" => TokenKind::Print,          // print
        "tulis" => TokenKind::Print,          // write
        // === PURITY / PARALLEL ===
        "tulen" => TokenKind::Pure,           // pure (Malay)
        "selari" => TokenKind::Parallel,      // parallel (Malay)
        // === INTERFACES / METHODS ===
        "antaramuka" => TokenKind::Interface, // interface (Malay)
        "laksanakan" => TokenKind::Implement, // implement (Malay)
        "kaedah" => TokenKind::Methods,       // methods (Malay)
        // === BOUNDS ===
        "tempat" => TokenKind::Where,         // place / where (Malay)
        "adalah" => TokenKind::Is,            // is
        // === CONCURRENCY ===
        "cuba" => TokenKind::Try,             // try (Malay)
        "tugasan" => TokenKind::Task,         // task (Malay)
        "gabung" => TokenKind::Join,          // join
        // === EMBEDDED ===
        "tidakselamat" => TokenKind::Unsafe,  // unsafe (compound)
        "kawasan" => TokenKind::RegionKw,     // region (Malay)
        // === SOV-S7 PARITY ===
        "tujuan" => TokenKind::Intent,        // purpose / intent
        "jenis" => TokenKind::Type,           // type / kind
        "luaran" => TokenKind::Extern,        // external (Malay)
        "tetap" => TokenKind::Invariant,      // fixed
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.11 (2026-06-08): Swahili (Kiswahili) keyword
/// resolution. First African Tier II dialect, lingua franca of
/// East Africa. Basic Latin alphabet, SVO grammar.
fn swahili_ascii_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        // === DECLARATIONS ===
        "kazi" => TokenKind::Fn,              // work / function
        "acha" => TokenKind::Let,             // let
        "muundo" => TokenKind::Struct,        // structure
        "orodha" => TokenKind::Enum,          // list / enumeration
        "thabiti" => TokenKind::Const,        // constant / fixed
        // === VISIBILITY / MODULES ===
        "umma" => TokenKind::Pub,             // public / community
        "moduli" => TokenKind::Module,        // module (loanword)
        "tumia" => TokenKind::Use,            // use!
        "kama" => TokenKind::As,              // as / like
        // === CONTROL FLOW ===
        "rudi" => TokenKind::Return,          // return!
        "kama_ni" => TokenKind::If,           // if it is — compound
        "ikiwa" => TokenKind::If,             // if (alt)
        "vinginevyo" => TokenKind::Else,      // otherwise
        "wakati" => TokenKind::While,         // while / during
        "kwa" => TokenKind::For,              // for
        "ndani" => TokenKind::In,             // inside
        "kutoka" => TokenKind::From,          // from
        "hadi" => TokenKind::To,              // to / until
        "vunja" => TokenKind::Break,          // break!
        "endelea" => TokenKind::Continue,     // continue!
        "kisha" => TokenKind::Then,           // then
        // === REFS / MUT ===
        "tazama" => TokenKind::Ref,           // look at!
        "badilika" => TokenKind::Mut,         // changeable
        // === MATCH ===
        "linganisha" => TokenKind::Match,     // match / compare
        // === VERIFICATION ===
        "thibitisha" => TokenKind::Assert,    // verify!
        "thibitisha_kuwa" => TokenKind::Prove, // prove that — compound; use single
        "thibitisha_kabisa" => TokenKind::Prove, // prove completely (alt)
        "hitaji" => TokenKind::Requires,      // need
        "hakikisha" => TokenKind::Ensures,    // ensure
        // === BOOL / PRINT ===
        "kweli" => TokenKind::True,           // true
        "uongo" => TokenKind::False,          // false / lie
        "chapisha" => TokenKind::Print,       // print!
        "andika" => TokenKind::Print,         // write!
        // === PURITY / PARALLEL ===
        "safi" => TokenKind::Pure,            // pure / clean
        "sambamba" => TokenKind::Parallel,    // parallel
        // === INTERFACES / METHODS ===
        "kiolesura" => TokenKind::Interface,  // interface
        "tekeleza" => TokenKind::Implement,   // implement / execute
        "njia" => TokenKind::Methods,         // ways / methods
        // === BOUNDS ===
        "wapi" => TokenKind::Where,           // where
        "ni" => TokenKind::Is,                // is
        // === CONCURRENCY ===
        "jaribu" => TokenKind::Try,           // try!
        "jukumu" => TokenKind::Task,          // task / responsibility
        "unganisha" => TokenKind::Join,       // join / connect
        // === EMBEDDED ===
        "hatari" => TokenKind::Unsafe,        // danger / unsafe
        "eneo" => TokenKind::RegionKw,        // region / area
        // === SOV-S7 PARITY ===
        "lengo" => TokenKind::Intent,         // goal
        "aina" => TokenKind::Type,            // type / kind
        "nje" => TokenKind::Extern,           // outside / external
        "isiyobadilika" => TokenKind::Invariant, // unchanging
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.6 (2026-06-08): Italian (italiano) keyword
/// resolution. Italian keyword surface is mostly pure ASCII —
/// the only natural non-ASCII keyword would be `è` (single-char
/// grave-accented "is"), which is too short / ambiguous to
/// register as a keyword. The full surface is pragma-gated
/// like Indonesian.
fn italian_ascii_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        // === DECLARATIONS ===
        "funzione" => TokenKind::Fn,          // function
        "sia" => TokenKind::Let,              // "let it be" (subjunctive)
        "struttura" => TokenKind::Struct,     // structure
        "enumerazione" => TokenKind::Enum,    // enumeration
        "costante" => TokenKind::Const,       // constant
        // === VISIBILITY / MODULES ===
        "pubblico" => TokenKind::Pub,         // public
        "modulo" => TokenKind::Module,        // module
        "usare" => TokenKind::Use,            // use
        "come" => TokenKind::As,              // as
        // === CONTROL FLOW ===
        "ritornare" => TokenKind::Return,     // return
        "ritorna" => TokenKind::Return,       // return! (imperative)
        "se" => TokenKind::If,                // if
        "altrimenti" => TokenKind::Else,      // else / otherwise
        "mentre" => TokenKind::While,         // while
        "per" => TokenKind::For,              // for / "for each"
        "da" => TokenKind::From,              // from
        "fino" => TokenKind::To,              // until / to
        "rompere" => TokenKind::Break,        // break
        "interrompere" => TokenKind::Break,   // interrupt (alt)
        "continuare" => TokenKind::Continue,  // continue
        "allora" => TokenKind::Then,          // then
        // === REFS / MUT ===
        "vedere" => TokenKind::Ref,           // see / reference
        "mutevole" => TokenKind::Mut,         // changeable / mutable
        // === MATCH ===
        "corrispondere" => TokenKind::Match,  // match / correspond
        "combaciare" => TokenKind::Match,     // match / fit together (alt)
        // === VERIFICATION ===
        "affermare" => TokenKind::Assert,     // assert
        "dimostrare" => TokenKind::Prove,     // prove / demonstrate
        "richiede" => TokenKind::Requires,    // requires
        "garantisce" => TokenKind::Ensures,   // guarantees
        // === BOOL / PRINT ===
        "vero" => TokenKind::True,            // true
        "falso" => TokenKind::False,          // false
        "stampare" => TokenKind::Print,       // print
        "scrivere" => TokenKind::Print,       // write (alt)
        // === PURITY / PARALLEL ===
        "puro" => TokenKind::Pure,            // pure
        "parallelo" => TokenKind::Parallel,   // parallel
        // === INTERFACES / METHODS ===
        "interfaccia" => TokenKind::Interface, // interface
        "implementare" => TokenKind::Implement, // implement
        "metodi" => TokenKind::Methods,       // methods
        // === BOUNDS ===
        "dove" => TokenKind::Where,           // where
        // === CONCURRENCY ===
        "tentare" => TokenKind::Try,          // try / attempt
        "compito" => TokenKind::Task,         // task / assignment
        "unire" => TokenKind::Join,           // join / unite
        // === EMBEDDED ===
        "insicuro" => TokenKind::Unsafe,      // unsafe / insecure
        "regione" => TokenKind::RegionKw,     // region
        // === SOV-S7 PARITY ===
        "scopo" => TokenKind::Intent,         // purpose / intent
        "intenzione" => TokenKind::Intent,    // intent (alt)
        "obiettivo" => TokenKind::Intent,     // objective (alt)
        "tipo" => TokenKind::Type,            // type
        "esterno" => TokenKind::Extern,       // external
        "invariante" => TokenKind::Invariant, // invariant
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.7 (2026-06-08): Modern Standard Arabic (العربية)
/// keyword resolution. Distinct from the shipped Perso-Arabic
/// dialects which use the Arabic SCRIPT for Indo-Iranian /
/// Indo-Aryan languages (Urdu, Sindhi, Shahmukhi, Persian,
/// Pashto). This table holds native Arabic vocabulary on the
/// existing Script::Arabic infrastructure. All keywords start
/// non-ASCII (Arabic block U+0600..06FF) so they route through
/// `lex_unicode_ident`.
fn arabic_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        // === DECLARATIONS ===
        "دالة" => TokenKind::Fn,              // dāla (function)
        "ليكن" => TokenKind::Let,             // li-yakun ("let there be")
        "بنية" => TokenKind::Struct,          // binya (structure)
        "تعداد" => TokenKind::Enum,           // ta'dād (enumeration)
        "قيمة_ثابتة" => TokenKind::Const,     // qima thabita (constant value)
        // === VISIBILITY / MODULES ===
        "عام" => TokenKind::Pub,              // 'āmm (general / public)
        "وحدة" => TokenKind::Module,          // waḥda (unit / module)
        "استخدم" => TokenKind::Use,           // istakhdim (use!)
        // === CONTROL FLOW ===
        "أرجع" => TokenKind::Return,          // arja' (return!)
        "إرجاع" => TokenKind::Return,         // irjā' (return — noun)
        "إذا" => TokenKind::If,               // idhā (if)
        "وإلا" => TokenKind::Else,            // wa-illā ("and otherwise")
        "بينما" => TokenKind::While,          // bayna-mā (while)
        "لكل" => TokenKind::For,              // li-kull (for each)
        "في" => TokenKind::In,                // fī (in)
        "من" => TokenKind::From,              // min (from)
        "إلى" => TokenKind::To,               // ilā (to / until)
        "كسر" => TokenKind::Break,            // kasr (break)
        "استمر" => TokenKind::Continue,       // istamir (continue!)
        "ثم" => TokenKind::Then,              // thumma (then)
        // === REFS / MUT ===
        "مرجع" => TokenKind::Ref,             // marja' (reference)
        "متغير" => TokenKind::Mut,            // mutaghayyir (changing / variable)
        // === MATCH ===
        "طابق" => TokenKind::Match,           // ṭābiq (match!)
        // === VERIFICATION ===
        "تأكد" => TokenKind::Assert,          // ta'akkad (make sure)
        "أثبت" => TokenKind::Prove,           // athbit (prove!)
        "يتطلب" => TokenKind::Requires,       // yatatallab (requires)
        "يضمن" => TokenKind::Ensures,         // yaḍman (guarantees)
        // === BOOL / PRINT ===
        "صحيح" => TokenKind::True,            // ṣaḥīḥ (true / correct)
        "خطأ" => TokenKind::False,            // khaṭa' (wrong / mistake)
        "اطبع" => TokenKind::Print,           // iṭba' (print!)
        // === PURITY / PARALLEL ===
        "نقي" => TokenKind::Pure,             // naqī (pure)
        "متوازي" => TokenKind::Parallel,      // mutawāzī (parallel)
        // === INTERFACES / METHODS ===
        "واجهة" => TokenKind::Interface,      // wājiha (interface)
        "نفذ" => TokenKind::Implement,        // naffidh (execute / implement)
        "طرق" => TokenKind::Methods,          // ṭuruq (methods / ways)
        // === BOUNDS ===
        "حيث" => TokenKind::Where,            // ḥaythu (where)
        "هو" => TokenKind::Is,                // huwa (he / it / is)
        // === CONCURRENCY ===
        "حاول" => TokenKind::Try,             // ḥāwil (try!)
        "مهمة" => TokenKind::Task,            // muhimma (task / mission)
        "اربط" => TokenKind::Join,            // urbiṭ (link!)
        // === EMBEDDED ===
        "غير_آمن" => TokenKind::Unsafe,       // ghayr āmin (unsafe — compound)
        "منطقة" => TokenKind::RegionKw,       // minṭaqa (region / area)
        // === SOV-S7 PARITY ===
        "هدف" => TokenKind::Intent,           // hadaf (goal / intent)
        "نوع" => TokenKind::Type,             // naw' (type / kind)
        "خارجي" => TokenKind::Extern,         // khārijī (external)
        "ثابت" => TokenKind::Invariant,       // thābit (constant / invariant)
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.4 (2026-06-08): Greek (Ελληνικά) keyword resolution.
/// First Greek-script dialect. Uses modern Greek's monotonic
/// accent system (single acute mark + diaeresis). All keywords
/// start with non-ASCII (Greek block starts at U+0370) so they
/// route through `lex_unicode_ident`.
fn greek_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        // === DECLARATIONS ===
        "συνάρτηση" => TokenKind::Fn,         // synártisi (function)
        "έστω" => TokenKind::Let,             // ésto (let / "let it be")
        "δομή" => TokenKind::Struct,          // domí (structure)
        "απαρίθμηση" => TokenKind::Enum,      // aparíthmisi (enumeration)
        "σταθερά" => TokenKind::Const,        // stathérá (constant)
        // === VISIBILITY / MODULES ===
        "δημόσιο" => TokenKind::Pub,          // dimósio (public)
        "ενότητα" => TokenKind::Module,       // enótita (unit / module)
        "άρθρωμα" => TokenKind::Module,       // árthroma (module — alt)
        "χρήση" => TokenKind::Use,            // khrísi (use)
        "ως" => TokenKind::As,                // os (as)
        // === CONTROL FLOW ===
        "επιστροφή" => TokenKind::Return,     // epistrofí (return)
        "αν" => TokenKind::If,                // an (if)
        "αλλιώς" => TokenKind::Else,          // alliós (else)
        "όσο" => TokenKind::While,            // óso (while / as long as)
        "για" => TokenKind::For,              // gia (for)
        "σε" => TokenKind::In,                // se (in)
        "από" => TokenKind::From,             // apó (from)
        "μέχρι" => TokenKind::To,             // méhri (until / to)
        "διακοπή" => TokenKind::Break,        // diakopí (interruption / break)
        "συνέχεια" => TokenKind::Continue,    // synéheia (continuation)
        "τότε" => TokenKind::Then,            // tóte (then)
        // === REFS / MUT ===
        "αναφορά" => TokenKind::Ref,          // anaforá (reference)
        "μεταβλητό" => TokenKind::Mut,        // metavlitó (changeable / mutable)
        // === MATCH ===
        "αντιστοιχία" => TokenKind::Match,    // antistoikhía (correspondence)
        // === VERIFICATION ===
        "επιβεβαίωση" => TokenKind::Assert,   // epivevaíosi (confirmation)
        "απόδειξη" => TokenKind::Prove,       // apódeixi (proof)
        "απαιτεί" => TokenKind::Requires,     // apaiteí (requires)
        "εγγυάται" => TokenKind::Ensures,     // engyátai (guarantees)
        // === BOOL / PRINT ===
        "αληθές" => TokenKind::True,          // alithés (true)
        "ψευδές" => TokenKind::False,         // pseudés (false)
        "εκτύπωση" => TokenKind::Print,       // ektýposi (print)
        "γράψε" => TokenKind::Print,          // grápse (write! — alt)
        // === PURITY / PARALLEL ===
        "καθαρό" => TokenKind::Pure,          // katharó (pure)
        "παράλληλο" => TokenKind::Parallel,   // parállilo (parallel)
        // === INTERFACES / METHODS ===
        "διεπαφή" => TokenKind::Interface,    // diepafí (interface)
        "υλοποίηση" => TokenKind::Implement,  // ylopoíisi (implementation)
        "μέθοδοι" => TokenKind::Methods,      // méthodoi (methods)
        // === BOUNDS ===
        "όπου" => TokenKind::Where,           // ópou (where)
        "είναι" => TokenKind::Is,             // eínai (is)
        // === CONCURRENCY ===
        "δοκιμή" => TokenKind::Try,           // dokimí (try)
        "εργασία" => TokenKind::Task,         // ergasía (work / task)
        "ένωση" => TokenKind::Join,           // énosi (union / join)
        // === EMBEDDED ===
        "επικίνδυνο" => TokenKind::Unsafe,    // epikíndyno (dangerous)
        "περιοχή" => TokenKind::RegionKw,     // perioχí (region)
        // === SOV-S7 PARITY ===
        "σκοπός" => TokenKind::Intent,        // skopós (purpose)
        "τύπος" => TokenKind::Type,           // týpos (type)
        "εξωτερικό" => TokenKind::Extern,     // exoterikó (external)
        "αμετάβλητο" => TokenKind::Invariant, // ametávlito (invariant)
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.5 (2026-06-08): Hebrew (עברית) keyword resolution.
/// First Hebrew-script dialect. RTL writing direction is a
/// rendering concern only — the lexer reads UTF-8 in logical
/// (byte) order, same approach as the shipped Perso-Arabic
/// batch.
fn hebrew_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        // === DECLARATIONS ===
        "פונקציה" => TokenKind::Fn,           // funktsia (function — loanword)
        "פעולה" => TokenKind::Fn,             // peulah (action — alt for fn)
        "יהי" => TokenKind::Let,              // yehi ("let there be")
        "מבנה" => TokenKind::Struct,          // mivneh (structure)
        "ספירה" => TokenKind::Enum,           // sefirah (counting / enumeration)
        "קבוע" => TokenKind::Const,           // kavua (constant / fixed)
        // === VISIBILITY / MODULES ===
        "ציבורי" => TokenKind::Pub,           // tsiburi (public)
        "מודול" => TokenKind::Module,         // modul (module — loanword)
        "מודולים" => TokenKind::Module,       // moduliym (modules — alt)
        "השתמש" => TokenKind::Use,            // hishtamesh (use!)
        // === CONTROL FLOW ===
        "החזר" => TokenKind::Return,          // hakher (return!)
        "חזרה" => TokenKind::Return,          // chazara (return — noun)
        "אם" => TokenKind::If,                // im (if)
        "אחרת" => TokenKind::Else,            // aheret (else / otherwise)
        "כאשר" => TokenKind::While,           // kasher (while / when)
        "עבור" => TokenKind::For,             // avur (for / "on behalf of")
        "בתוך" => TokenKind::In,              // betoch (inside)
        "מתוך" => TokenKind::From,            // mitokh (from)
        "עד" => TokenKind::To,                // ad (until)
        "שבור" => TokenKind::Break,           // shvor (break!)
        "הפסק" => TokenKind::Break,           // hafsek (stop! — alt)
        "המשך" => TokenKind::Continue,        // hemshech (continue!)
        "אז" => TokenKind::Then,              // az (then)
        // === REFS / MUT ===
        "הפנייה" => TokenKind::Ref,           // hapnaya (reference)
        "משתנה" => TokenKind::Mut,            // mishtaneh (changing / variable)
        // === MATCH ===
        "התאם" => TokenKind::Match,           // hat'em (match!)
        // === VERIFICATION ===
        "ודא" => TokenKind::Assert,           // vada (verify!)
        "הוכח" => TokenKind::Prove,           // hokach (prove!)
        "דורש" => TokenKind::Requires,        // doresh (requires)
        "מבטיח" => TokenKind::Ensures,        // mavtiakh (guarantees)
        // === BOOL / PRINT ===
        "אמת" => TokenKind::True,             // emet (truth / true)
        "שקר" => TokenKind::False,            // sheker (lie / false)
        "הדפס" => TokenKind::Print,           // hadpes (print!)
        "כתוב" => TokenKind::Print,           // ktov (write! — alt)
        // === PURITY / PARALLEL ===
        "טהור" => TokenKind::Pure,            // tahor (pure)
        "מקבילי" => TokenKind::Parallel,      // makbili (parallel)
        // === INTERFACES / METHODS ===
        "ממשק" => TokenKind::Interface,       // memshak (interface)
        "ממש" => TokenKind::Implement,        // mamesh (implement / realize)
        "שיטות" => TokenKind::Methods,        // shitot (methods)
        // === BOUNDS ===
        "איפה" => TokenKind::Where,           // eyfo (where)
        "הוא" => TokenKind::Is,               // hu (is — "he/it")
        // === CONCURRENCY ===
        "נסה" => TokenKind::Try,              // naseh (try!)
        "משימה" => TokenKind::Task,           // mesimah (task)
        "חיבור" => TokenKind::Join,           // khibur (connection / join)
        // === EMBEDDED ===
        "מסוכן" => TokenKind::Unsafe,         // mesukan (dangerous / unsafe)
        "אזור" => TokenKind::RegionKw,        // azor (region / area)
        // === SOV-S7 PARITY ===
        "מטרה" => TokenKind::Intent,          // matarah (goal / intent)
        "סוג" => TokenKind::Type,             // sug (kind)
        "טיפוס" => TokenKind::Type,           // tipus (type — alt loanword)
        "חיצוני" => TokenKind::Extern,        // khitsoni (external)
        "בלתי-משתנה" => TokenKind::Invariant, // bilti-mishtaneh (unchanging)
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.3 (2026-06-08): Indonesian (Bahasa Indonesia)
/// keyword resolution. First basic-Latin Tier II dialect — has
/// no diacritics, so the only path is the pragma-gated ASCII
/// keyword table (no companion non-ASCII table). The pragma
/// gate is REQUIRED: words like `untuk`, `jika`, `benar`,
/// `salah`, `cetak` are all natural Indonesian identifiers
/// that English code might legitimately use as variable names.
fn indonesian_ascii_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        // === DECLARATIONS ===
        "fungsi" => TokenKind::Fn,            // function
        "biarkan" => TokenKind::Let,          // "let it be"
        "misalkan" => TokenKind::Let,         // "suppose" (alt)
        "struktur" => TokenKind::Struct,      // structure
        "enumerasi" => TokenKind::Enum,       // enumeration
        "tetap" => TokenKind::Const,          // fixed / constant
        // === VISIBILITY / MODULES ===
        "publik" => TokenKind::Pub,           // public
        "umum" => TokenKind::Pub,             // general / common (alt)
        "modul" => TokenKind::Module,         // module
        "pakai" => TokenKind::Use,            // use
        "sebagai" => TokenKind::As,           // as
        // === CONTROL FLOW ===
        "kembali" => TokenKind::Return,       // return
        "kembalikan" => TokenKind::Return,    // return! (imperative)
        "jika" => TokenKind::If,              // if
        "selainnya" => TokenKind::Else,       // else / "other than"
        "lainnya" => TokenKind::Else,         // other (alt)
        "selama" => TokenKind::While,         // while / during
        "untuk" => TokenKind::For,            // for
        "dalam" => TokenKind::In,             // inside
        "dari" => TokenKind::From,            // from
        "sampai" => TokenKind::To,            // until / to
        "hingga" => TokenKind::To,            // until (alt)
        "berhenti" => TokenKind::Break,       // stop / break
        "lanjutkan" => TokenKind::Continue,   // continue
        "maka" => TokenKind::Then,            // then
        // === REFS / MUT ===
        "lihat" => TokenKind::Ref,            // see / look
        "dapatberubah" => TokenKind::Mut,     // changeable (compound)
        "berubah" => TokenKind::Mut,          // changing
        // === MATCH ===
        "cocokkan" => TokenKind::Match,       // match!
        "padanan" => TokenKind::Match,        // match (noun, alt)
        // === VERIFICATION ===
        "pastikan" => TokenKind::Assert,      // make sure / assert
        "buktikan" => TokenKind::Prove,       // prove
        "perlu" => TokenKind::Requires,       // needs / requires
        "jamin" => TokenKind::Ensures,        // guarantee
        // === BOOL / PRINT ===
        "benar" => TokenKind::True,           // true / correct
        "salah" => TokenKind::False,          // false / wrong
        "cetak" => TokenKind::Print,          // print
        "tulis" => TokenKind::Print,          // write (alt)
        // === PURITY / PARALLEL ===
        "murni" => TokenKind::Pure,           // pure
        "paralel" => TokenKind::Parallel,     // parallel (loanword)
        // === INTERFACES / METHODS ===
        "antarmuka" => TokenKind::Interface,  // interface
        "terapkan" => TokenKind::Implement,   // apply / implement
        "implementasi" => TokenKind::Implement, // implementation (alt)
        "metode" => TokenKind::Methods,       // methods
        // === BOUNDS ===
        "dimana" => TokenKind::Where,         // where
        "adalah" => TokenKind::Is,            // is
        // === CONCURRENCY ===
        "coba" => TokenKind::Try,             // try
        "tugas" => TokenKind::Task,           // task
        "gabungkan" => TokenKind::Join,       // join!
        // === EMBEDDED ===
        "bahaya" => TokenKind::Unsafe,        // danger / unsafe
        "wilayah" => TokenKind::RegionKw,     // region
        // === SOV-S7 PARITY ===
        "tujuan" => TokenKind::Intent,        // purpose / intent
        "niat" => TokenKind::Intent,          // intent (alt)
        "tipe" => TokenKind::Type,            // type
        "jenis" => TokenKind::Type,           // kind (alt)
        "eksternal" => TokenKind::Extern,     // external
        "invarian" => TokenKind::Invariant,   // invariant (loanword)
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.2 (2026-06-08): Portuguese (português) keyword
/// resolution. Fourth Latin-with-accents Tier II dialect. Same
/// v1 split as Spanish/French/German: this table holds the
/// natural non-ASCII forms (`função`, `módulo`, `público`,
/// `região`, `intenção`, `enumeração`, `métodos`, `até`,
/// `senão`, `então`, `mutável`, `referência`, `não`) that can't
/// collide with English identifiers; the pure-ASCII forms live
/// in `portuguese_ascii_keyword` and are pragma-gated.
fn portuguese_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        // === DECLARATIONS ===
        "função" => TokenKind::Fn,            // function
        "enumeração" => TokenKind::Enum,      // enumeration
        // === VISIBILITY / MODULES ===
        "público" => TokenKind::Pub,          // public
        "módulo" => TokenKind::Module,        // module
        // === CONTROL FLOW ===
        "senão" => TokenKind::Else,           // else / otherwise
        "então" => TokenKind::Then,           // then
        "até" => TokenKind::To,               // to / until
        // === REFS / MUT ===
        "referência" => TokenKind::Ref,       // reference
        "mutável" => TokenKind::Mut,          // mutable
        // === INTERFACES / METHODS ===
        "métodos" => TokenKind::Methods,      // methods
        // === SOV-S7 PARITY ===
        "intenção" => TokenKind::Intent,      // intent
        "propósito" => TokenKind::Intent,     // purpose (alt)
        "região" => TokenKind::RegionKw,      // region
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.2 (2026-06-08): pure-ASCII Portuguese keyword table.
/// Pragma-gated like Spanish/French/German — only consulted
/// when the file declares `// vani-lang: portuguese` (or
/// `brasileiro`, `pt`).
fn portuguese_ascii_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        // === DECLARATIONS ===
        "funcao" => TokenKind::Fn,            // function (sem til alt)
        "seja" => TokenKind::Let,             // "let it be"
        "estrutura" => TokenKind::Struct,     // structure
        "constante" => TokenKind::Const,      // constant
        "enumeracao" => TokenKind::Enum,      // enumeration (sem til)
        // === VISIBILITY / MODULES ===
        "publico" => TokenKind::Pub,          // public (sem acento)
        "modulo" => TokenKind::Module,        // module (sem acento)
        "usar" => TokenKind::Use,             // use
        "como" => TokenKind::As,              // as
        // === CONTROL FLOW ===
        "retornar" => TokenKind::Return,      // return
        "retorne" => TokenKind::Return,       // return! (imperative)
        "se" => TokenKind::If,                // if
        "senao" => TokenKind::Else,           // else (sem til)
        "enquanto" => TokenKind::While,       // while
        "para" => TokenKind::For,             // for
        "em" => TokenKind::In,                // in
        "desde" => TokenKind::From,           // from
        "ate" => TokenKind::To,               // until (sem acento)
        "parar" => TokenKind::Break,          // stop / break
        "interromper" => TokenKind::Break,    // interrupt (alt)
        "continuar" => TokenKind::Continue,   // continue
        "entao" => TokenKind::Then,           // then (sem til)
        // === REFS / MUT ===
        "ver" => TokenKind::Ref,              // see
        "mutavel" => TokenKind::Mut,          // mutable (sem acento)
        // === MATCH ===
        "combinar" => TokenKind::Match,       // match / combine
        "corresponder" => TokenKind::Match,   // match / correspond (alt)
        // === VERIFICATION ===
        "afirmar" => TokenKind::Assert,       // assert
        "provar" => TokenKind::Prove,         // prove
        "demonstrar" => TokenKind::Prove,     // demonstrate (alt)
        "requer" => TokenKind::Requires,      // requires
        "garante" => TokenKind::Ensures,      // guarantees
        // === BOOL / PRINT ===
        "verdadeiro" => TokenKind::True,      // true
        "falso" => TokenKind::False,          // false
        "imprimir" => TokenKind::Print,       // print
        "escrever" => TokenKind::Print,       // write (alt)
        // === PURITY / PARALLEL ===
        "puro" => TokenKind::Pure,            // pure
        "paralelo" => TokenKind::Parallel,    // parallel
        // === INTERFACES / METHODS ===
        "interface" => TokenKind::Interface,  // interface
        "implementar" => TokenKind::Implement, // implement
        "metodos" => TokenKind::Methods,      // methods (sem acento)
        // === BOUNDS ===
        "onde" => TokenKind::Where,           // where
        "eh" => TokenKind::Is,                // "is" (ASCII transliteration
                                              // of `é`; sometimes used
                                              // in informal Portuguese)
        // === CONCURRENCY ===
        "tentar" => TokenKind::Try,           // try
        "tarefa" => TokenKind::Task,          // task
        "juntar" => TokenKind::Join,          // join
        "unir" => TokenKind::Join,            // unite (alt)
        // === EMBEDDED ===
        "inseguro" => TokenKind::Unsafe,      // unsafe
        "regiao" => TokenKind::RegionKw,      // region (sem til)
        // === SOV-S7 PARITY ===
        "intencao" => TokenKind::Intent,      // intent (sem til)
        "proposito" => TokenKind::Intent,     // purpose (sem acento, alt)
        "objetivo" => TokenKind::Intent,      // objective (alt)
        "tipo" => TokenKind::Type,            // type
        "externo" => TokenKind::Extern,       // external
        "invariante" => TokenKind::Invariant, // invariant
        _ => return None,
    };
    Some(kind)
}

/// Phase pragma-threading (2026-06-08): pure-ASCII Spanish
/// keyword table. Only consulted when the file declares
/// `// vani-lang: spanish`. The non-ASCII Spanish keywords
/// (función, módulo, público, intención, …) live in
/// `spanish_keyword` and fire regardless of pragma since they
/// can't collide with English identifiers; this table covers
/// the natural pure-ASCII forms (si, para, sea, regresar, etc.)
/// that would collide.
fn spanish_ascii_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        // === DECLARATIONS ===
        "funcion" => TokenKind::Fn,           // function (sin tilde alt for función)
        "sea" => TokenKind::Let,              // "let it be"
        "estructura" => TokenKind::Struct,    // structure
        "constante" => TokenKind::Const,      // constant
        "enumeracion" => TokenKind::Enum,     // enumeration (sin tilde alt)
        // === VISIBILITY / MODULES ===
        "publico" => TokenKind::Pub,          // public (sin tilde alt)
        "modulo" => TokenKind::Module,        // module (sin tilde alt)
        "usar" => TokenKind::Use,             // use
        "como" => TokenKind::As,              // as
        // === CONTROL FLOW ===
        "regresar" => TokenKind::Return,      // return / go back
        "retornar" => TokenKind::Return,      // return (alt)
        "volver" => TokenKind::Return,        // return (alt)
        "si" => TokenKind::If,                // if
        "sino" => TokenKind::Else,            // else / "if not"
        "mientras" => TokenKind::While,       // while
        "para" => TokenKind::For,             // for
        "en" => TokenKind::In,                // in
        "desde" => TokenKind::From,           // from
        "hasta" => TokenKind::To,             // until / to
        "romper" => TokenKind::Break,         // break
        "continuar" => TokenKind::Continue,   // continue
        "entonces" => TokenKind::Then,        // then
        // === REFS / MUT ===
        "ver" => TokenKind::Ref,              // see / reference
        "mutable" => TokenKind::Mut,          // mutable
        // === MATCH ===
        "coincidir" => TokenKind::Match,      // match / coincide
        // === VERIFICATION ===
        "afirmar" => TokenKind::Assert,       // assert / affirm
        "demostrar" => TokenKind::Prove,      // prove / demonstrate
        "requiere" => TokenKind::Requires,    // requires
        "garantiza" => TokenKind::Ensures,    // guarantees
        // === BOOL / PRINT ===
        "verdadero" => TokenKind::True,       // true
        "falso" => TokenKind::False,          // false
        "imprimir" => TokenKind::Print,       // print
        "escribir" => TokenKind::Print,       // write (alt)
        // === PURITY / PARALLEL ===
        "puro" => TokenKind::Pure,            // pure
        "paralelo" => TokenKind::Parallel,    // parallel
        // === INTERFACES / METHODS ===
        "interfaz" => TokenKind::Interface,   // interface
        "implementar" => TokenKind::Implement, // implement
        "metodos" => TokenKind::Methods,      // methods (sin tilde alt)
        // === BOUNDS ===
        "donde" => TokenKind::Where,          // where (relative — no accent)
        "es" => TokenKind::Is,                // is
        // === CONCURRENCY ===
        "intentar" => TokenKind::Try,         // try
        "tarea" => TokenKind::Task,           // task
        "unir" => TokenKind::Join,            // join / unite
        // === EMBEDDED ===
        "inseguro" => TokenKind::Unsafe,      // unsafe
        "region" => TokenKind::RegionKw,      // region (sin tilde alt)
        // === SOV-S7 PARITY ===
        "intencion" => TokenKind::Intent,     // intent (sin tilde alt)
        "tipo" => TokenKind::Type,            // type
        "externo" => TokenKind::Extern,       // external
        "invariante" => TokenKind::Invariant, // invariant
        _ => return None,
    };
    Some(kind)
}

/// Phase pragma-threading (2026-06-08): pure-ASCII French
/// keyword table. Same gating as Spanish — only consulted under
/// `// vani-lang: french`.
fn french_ascii_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        // === DECLARATIONS ===
        "fonction" => TokenKind::Fn,          // function
        "soit" => TokenKind::Let,             // "let"
        "structure" => TokenKind::Struct,     // structure
        "constante" => TokenKind::Const,      // constant
        // === VISIBILITY / MODULES ===
        "public" => TokenKind::Pub,           // public
        "module" => TokenKind::Module,        // module
        "utiliser" => TokenKind::Use,         // use
        "comme" => TokenKind::As,             // as
        // === CONTROL FLOW ===
        "retourner" => TokenKind::Return,     // return
        "retourne" => TokenKind::Return,      // return! (imperative)
        "si" => TokenKind::If,                // if
        "sinon" => TokenKind::Else,           // else
        "tandis" => TokenKind::While,         // while
        "pour" => TokenKind::For,             // for
        "dans" => TokenKind::In,              // in
        "depuis" => TokenKind::From,          // from
        "vers" => TokenKind::To,              // toward / to
        "interrompre" => TokenKind::Break,    // break
        "continuer" => TokenKind::Continue,   // continue
        "alors" => TokenKind::Then,           // then
        // === REFS / MUT ===
        "voir" => TokenKind::Ref,             // see
        "muable" => TokenKind::Mut,           // mutable / changeable
        // === MATCH ===
        "correspondre" => TokenKind::Match,   // match / correspond
        // === VERIFICATION ===
        "affirmer" => TokenKind::Assert,      // assert
        "prouver" => TokenKind::Prove,        // prove
        "exige" => TokenKind::Requires,       // requires
        "garantit" => TokenKind::Ensures,     // guarantees
        // === BOOL / PRINT ===
        "vrai" => TokenKind::True,            // true
        "faux" => TokenKind::False,           // false
        "imprimer" => TokenKind::Print,       // print
        "afficher" => TokenKind::Print,       // display (alt)
        // === PURITY / PARALLEL ===
        "pur" => TokenKind::Pure,             // pure
        // === INTERFACES / METHODS ===
        "interface" => TokenKind::Interface,  // interface
        "implementer" => TokenKind::Implement, // implement (no accent alt)
        "methodes" => TokenKind::Methods,     // methods (no accent alt)
        // === BOUNDS ===
        "ou" => TokenKind::Where,             // where (no accent — distinct from "ou" or)
                                              // careful: `ou` also means "or"
                                              // but vāṇी uses `||` so safe
        "est" => TokenKind::Is,               // is
        // === CONCURRENCY ===
        "essayer" => TokenKind::Try,          // try
        "tache" => TokenKind::Task,           // task (no accent alt)
        "joindre" => TokenKind::Join,         // join
        // === EMBEDDED ===
        "dangereux" => TokenKind::Unsafe,     // dangerous
        // === SOV-S7 PARITY ===
        "but" => TokenKind::Intent,           // goal / intent
        "objectif" => TokenKind::Intent,      // objective (alt)
        "type" => TokenKind::Type,            // type
        "externe" => TokenKind::Extern,       // external
        "invariant" => TokenKind::Invariant,  // invariant
        _ => return None,
    };
    Some(kind)
}

/// Phase pragma-threading (2026-06-08): pure-ASCII German
/// keyword table. Same gating — only under `// vani-lang: german`.
fn german_ascii_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        // === DECLARATIONS ===
        "funktion" => TokenKind::Fn,          // function
        "sei" => TokenKind::Let,              // "let" / "be"
        "struktur" => TokenKind::Struct,      // structure
        "konstante" => TokenKind::Const,      // constant
        // === VISIBILITY / MODULES ===
        "modul" => TokenKind::Module,         // module
        "verwenden" => TokenKind::Use,        // use
        "als" => TokenKind::As,               // as
        // === CONTROL FLOW ===
        "zurueck" => TokenKind::Return,       // return (no umlaut alt for `zurück`)
        "wenn" => TokenKind::If,              // if
        "sonst" => TokenKind::Else,           // else / otherwise
        "solange" => TokenKind::While,        // as long as / while
        "jede" => TokenKind::For,             // every / for
        "in" => TokenKind::In,                // in (same as English; harmless overlap
                                              // since "in" is already an English
                                              // keyword)
        "von" => TokenKind::From,             // from
        "bis" => TokenKind::To,               // to / until
        "brechen" => TokenKind::Break,        // break
        "weiter" => TokenKind::Continue,      // continue
        "dann" => TokenKind::Then,            // then
        // === REFS / MUT ===
        "sehen" => TokenKind::Ref,            // see
        "wandelbar" => TokenKind::Mut,        // changeable / mutable
        // === MATCH ===
        "passend" => TokenKind::Match,        // matching
        // === VERIFICATION ===
        "behaupten" => TokenKind::Assert,     // assert / claim
        "beweisen" => TokenKind::Prove,       // prove
        "benoetigt" => TokenKind::Requires,   // requires (no umlaut alt)
        "garantiert" => TokenKind::Ensures,   // guarantees
        // === BOOL / PRINT ===
        "wahr" => TokenKind::True,            // true
        "falsch" => TokenKind::False,         // false
        "drucken" => TokenKind::Print,        // print
        "schreiben" => TokenKind::Print,      // write (alt)
        // === PURITY / PARALLEL ===
        "rein" => TokenKind::Pure,            // pure
        "parallel" => TokenKind::Parallel,    // parallel (loanword)
        // === INTERFACES / METHODS ===
        "schnittstelle" => TokenKind::Interface, // interface
        "implementieren" => TokenKind::Implement, // implement
        "methoden" => TokenKind::Methods,     // methods
        // === BOUNDS ===
        "wo" => TokenKind::Where,             // where
        "ist" => TokenKind::Is,               // is
        // === CONCURRENCY ===
        "versuchen" => TokenKind::Try,        // try
        "aufgabe" => TokenKind::Task,         // task / assignment
        "verbinden" => TokenKind::Join,       // join / connect
        // === EMBEDDED ===
        "unsicher" => TokenKind::Unsafe,      // unsafe / insecure
        // === SOV-S7 PARITY ===
        "absicht" => TokenKind::Intent,       // intent
        "typ" => TokenKind::Type,             // type
        "extern" => TokenKind::Extern,        // external (same as English alias!)
        "unveraenderlich" => TokenKind::Invariant, // invariant (no umlaut alt)
        _ => return None,
    };
    Some(kind)
}

/// Phase 8b.1 (2026-06-07): Spanish (español) keyword resolution.
/// First Latin-script Tier II dialect. To avoid breaking existing
/// English code, v1 only registers Spanish keywords whose natural
/// spelling contains non-ASCII characters — these can't collide
/// with English identifiers since the lexer wouldn't tokenize an
/// ASCII English keyword the same way. Pure-ASCII Spanish keywords
/// (`si` for if, `para` for for, `verdadero` for true, etc.)
/// require pragma threading into the Lexer struct to enable
/// safely; queued for a v2 follow-up. In v1 a Spanish-pragma file
/// uses these non-ASCII Spanish aliases alongside English ASCII
/// keywords — partial Spanish surface, but every keyword that has
/// a natural non-ASCII Spanish form is available.
fn spanish_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        // === DECLARATIONS ===
        "función" => TokenKind::Fn,           // function
        "enumeración" => TokenKind::Enum,     // enumeration
        // === VISIBILITY / MODULES ===
        "público" => TokenKind::Pub,          // public
        "módulo" => TokenKind::Module,        // module
        // === SOV-S7 PARITY (all natural non-ASCII Spanish) ===
        "intención" => TokenKind::Intent,     // intent
        "propósito" => TokenKind::Intent,     // purpose (alt for intent)
        "métodos" => TokenKind::Methods,      // methods
        "región" => TokenKind::RegionKw,      // region
        // === BOUNDS ===
        // `dónde` (interrogative "where?") is distinct from `donde`
        // (relative "where"); both have a use, but only the accented
        // form is non-ASCII so safe to register without pragma gate.
        "dónde" => TokenKind::Where,          // where (interrogative)
        _ => return None,
    };
    Some(kind)
}

/// Phase 10.1 (2026-06-07): German (Deutsch) keyword resolution.
/// Third Latin-with-accents Tier II dialect. Same v1 design as
/// Spanish and French: only genuinely natural non-ASCII German
/// keywords are registered (umlauts ä/ö/ü and ß), so pure-ASCII
/// German words (`Funktion`, `wenn`, `dann`, `wahr`, `falsch`,
/// etc.) don't accidentally collide with user identifiers in
/// non-pragma files. Pragma threading queued for v2 to unlock
/// the full German keyword set.
fn german_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        // === DECLARATIONS ===
        "aufzählung" => TokenKind::Enum,       // enumeration (has ä)
        // === VISIBILITY ===
        "öffentlich" => TokenKind::Pub,        // public (has ö)
        // === CONTROL FLOW ===
        "während" => TokenKind::While,         // while (has ä)
        "für" => TokenKind::For,               // for (has ü)
        "zurück" => TokenKind::Return,         // return / back (has ü)
        "auflösen" => TokenKind::Break,        // resolve / break (has ö)
        // === REFS / MUT ===
        "veränderlich" => TokenKind::Mut,      // changeable / mutable (has ä)
        "veränderbar" => TokenKind::Mut,       // alt mutable form (has ä)
        // === MATCH / VERIFICATION ===
        "übereinstimmen" => TokenKind::Match,  // match / agree (has ü)
        "überprüfen" => TokenKind::Assert,     // verify (has ü)
        "überprüfe" => TokenKind::Assert,      // verify! imperative (has ü)
        "prüfen" => TokenKind::Assert,         // check (has ü)
        "prüfe" => TokenKind::Assert,          // check! imperative (has ü)
        // === PRINT ===
        "ausführen" => TokenKind::Print,       // execute (has ü)
        "ausführe" => TokenKind::Print,        // execute! imperative (has ü)
        // === CONCURRENCY ===
        "ausführbar" => TokenKind::Task,       // executable / task (has ü)
        // === SOV-S7 PARITY ===
        "möglichkeit" => TokenKind::Intent,    // possibility / intent (has ö)
        "äußere" => TokenKind::Extern,         // external (has ä + ß)
        "äußerer" => TokenKind::Extern,        // external (declined form, has ä + ß)
        _ => return None,
    };
    Some(kind)
}

/// Phase 13.1 (2026-06-07): Korean (한국어) keyword resolution.
/// First Hangul-script dialect. SOV grammar continues the
/// Japanese precedent — keyword-first surface in v1, native SOV
/// statement shapes queued behind the same generalization needed
/// for Japanese SOV. The keyword set uses precomposed Hangul
/// syllables (Unicode block U+AC00..U+D7AF) rather than
/// decomposed jamo, matching how modern Korean is typed and
/// rendered.
fn korean_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        // === DECLARATIONS ===
        "함수" => TokenKind::Fn,             // hamsu (function)
        "정의" => TokenKind::Let,            // jeongui (definition / let)
        "구조체" => TokenKind::Struct,       // gujoche (structure)
        "열거" => TokenKind::Enum,           // yeolgeo (enumeration)
        "상수" => TokenKind::Const,          // sangsu (constant)
        // === VISIBILITY / MODULES ===
        "공개" => TokenKind::Pub,            // gonggae (public / open)
        "모듈" => TokenKind::Module,         // modyul (module — loanword)
        "사용" => TokenKind::Use,            // sayong (use)
        "로서" => TokenKind::As,             // roseo (as)
        // === CONTROL FLOW ===
        "반환" => TokenKind::Return,         // banhwan (return)
        "돌려주기" => TokenKind::Return,     // dollyeojugi (give back)
        "만약" => TokenKind::If,             // manyak (if)
        "만일" => TokenKind::If,             // manil (if — alt)
        "아니면" => TokenKind::Else,         // animyeon (otherwise)
        "동안" => TokenKind::While,          // dongan (while / during)
        "각각" => TokenKind::For,            // gakgak (each / for)
        "안에" => TokenKind::In,             // ane (in / inside)
        "에서" => TokenKind::From,           // eseo (from)
        "까지" => TokenKind::To,             // kkaji (until)
        "중단" => TokenKind::Break,          // jungdan (interruption)
        "계속" => TokenKind::Continue,       // gyesok (continue)
        "그러면" => TokenKind::Then,         // geureomyeon (then)
        // === REFS / MUT ===
        "참조" => TokenKind::Ref,            // chamjo (reference)
        "가변" => TokenKind::Mut,            // gabyeon (changeable / mutable)
        // === MATCH ===
        "일치" => TokenKind::Match,          // ilchi (match / agreement)
        // === VERIFICATION ===
        "확인" => TokenKind::Assert,         // hwagin (verify / confirm)
        "증명" => TokenKind::Prove,          // jeungmyeong (prove)
        "필요" => TokenKind::Requires,       // piryo (need / require)
        "보장" => TokenKind::Ensures,        // bojang (guarantee)
        // === BOOL / PRINT ===
        "참" => TokenKind::True,             // cham (true)
        "거짓" => TokenKind::False,          // geojit (false)
        "출력" => TokenKind::Print,          // chullyeok (output)
        "쓰기" => TokenKind::Print,          // sseugi (write — alt)
        // === PURITY / PARALLEL ===
        "순수" => TokenKind::Pure,           // sunsu (pure)
        "병렬" => TokenKind::Parallel,       // byeongnyeol (parallel)
        // === INTERFACES / METHODS ===
        "인터페이스" => TokenKind::Interface, // inteopeyiseu (interface — loanword)
        "구현" => TokenKind::Implement,      // guhyeon (implementation)
        "메서드" => TokenKind::Methods,      // meseodeu (methods — loanword)
        // === BOUNDS ===
        "여기서" => TokenKind::Where,        // yeogiseo (where / here)
        "이다" => TokenKind::Is,             // ida (to be — Korean copula)
        // === CONCURRENCY ===
        "시도" => TokenKind::Try,            // sido (try / attempt)
        "작업" => TokenKind::Task,           // jageop (task / work)
        "결합" => TokenKind::Join,           // gyeolhap (join / unite)
        // === EMBEDDED ===
        "위험" => TokenKind::Unsafe,         // wiheom (danger / unsafe)
        "영역" => TokenKind::RegionKw,       // yeongyeok (region / area)
        // === SOV-S7 PARITY ===
        "목적" => TokenKind::Intent,         // mokjeok (purpose / intent)
        "타입" => TokenKind::Type,           // taip (type — loanword)
        "외부" => TokenKind::Extern,         // oebu (external)
        "불변" => TokenKind::Invariant,      // bulbyeon (invariant)
        _ => return None,
    };
    Some(kind)
}

/// Phase 9b (2026-06-07): Japanese (日本語) keyword resolution.
/// First three-script dialect and first non-Indic SOV target. The
/// keyword set freely mixes Kanji (function = 関数), Katakana
/// (task = タスク), and Hiragana (verb endings). v1 ships
/// keyword-first surface — Japanese SOV grammar forms (もし x
/// ならば { ... }) queued for v2.
fn japanese_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        // === DECLARATIONS ===
        "関数" => TokenKind::Fn,             // kansuu (function — Kanji)
        "代入" => TokenKind::Let,            // dainyuu (assignment)
        "構造体" => TokenKind::Struct,       // kouzoutai (structure)
        "列挙" => TokenKind::Enum,           // rekkyou (enumeration)
        "定数" => TokenKind::Const,          // teisuu (constant)
        // === VISIBILITY / MODULES ===
        "公開" => TokenKind::Pub,            // koukai (public)
        "モジュール" => TokenKind::Module,    // mojuuru (module — Katakana loanword)
        "単位" => TokenKind::Module,         // tan'i (unit — alt for module)
        "使用" => TokenKind::Use,            // shiyou (use)
        "として" => TokenKind::As,           // toshite (as — Hiragana)
        // === CONTROL FLOW ===
        "戻る" => TokenKind::Return,         // modoru (return / go back)
        "返す" => TokenKind::Return,         // kaesu (return — transitive)
        "もし" => TokenKind::If,             // moshi (if — Hiragana)
        "そうでなければ" => TokenKind::Else, // sou denakereba (otherwise — Hiragana)
        "の間" => TokenKind::While,          // no aida (while — multi-codepoint, single ident)
        "間" => TokenKind::While,            // aida (while — short form)
        "中断" => TokenKind::Break,          // chuudan (interrupt / break)
        "続行" => TokenKind::Continue,       // zokkou (continue / proceed)
        "ならば" => TokenKind::Then,         // naraba (then — Hiragana)
        "対象" => TokenKind::For,            // taishou (for each / target)
        "から" => TokenKind::From,           // kara (from — Hiragana)
        "まで" => TokenKind::To,             // made (to/until — Hiragana)
        // === REFS / MUT ===
        "参照" => TokenKind::Ref,            // sanshou (reference)
        "可変" => TokenKind::Mut,            // kahen (changeable / mutable)
        // === MATCH ===
        "一致" => TokenKind::Match,          // icchi (match / agreement)
        "マッチ" => TokenKind::Match,        // macchi (match — Katakana loanword)
        // === VERIFICATION ===
        "確認" => TokenKind::Assert,         // kakunin (assert / confirm)
        "証明" => TokenKind::Prove,          // shoumei (prove / proof)
        "前提" => TokenKind::Requires,       // zentei (precondition / requires)
        "保証" => TokenKind::Ensures,        // hoshou (guarantee / ensures)
        // === BOOL / PRINT ===
        "真" => TokenKind::True,             // shin (true / truth)
        "偽" => TokenKind::False,            // gi (false / falsity)
        "表示" => TokenKind::Print,          // hyouji (display)
        "書く" => TokenKind::Print,          // kaku (write)
        // === PURITY / PARALLEL ===
        "純粋" => TokenKind::Pure,           // junsui (pure)
        "並列" => TokenKind::Parallel,       // heiretsu (parallel)
        // === INTERFACES / METHODS ===
        "インターフェース" => TokenKind::Interface, // intaafeesu (interface — Katakana)
        "実装" => TokenKind::Implement,      // jissou (implementation)
        "メソッド" => TokenKind::Methods,    // mesoddo (methods — Katakana)
        // === BOUNDS ===
        "ここで" => TokenKind::Where,        // koko de (where — Hiragana)
        "は" => TokenKind::Is,               // wa (topic particle — used as "is")
        // === CONCURRENCY ===
        "試行" => TokenKind::Try,            // shikou (try)
        "タスク" => TokenKind::Task,         // tasuku (task — Katakana)
        "結合" => TokenKind::Join,           // ketsugou (join / union)
        // === EMBEDDED ===
        "危険" => TokenKind::Unsafe,         // kiken (danger / unsafe)
        "領域" => TokenKind::RegionKw,       // ryouiki (region / area)
        // === SOV-S7 PARITY ===
        "目的" => TokenKind::Intent,         // mokuteki (purpose / intent)
        "意図" => TokenKind::Intent,         // ito (intention)
        "型" => TokenKind::Type,             // kata (type)
        "外部" => TokenKind::Extern,         // gaibu (external)
        "不変" => TokenKind::Invariant,      // fuhen (invariant)
        _ => return None,
    };
    Some(kind)
}

/// Phase 10.2 (2026-06-08): Mandarin Chinese (中文) keyword resolution.
/// Shares the CJK Unified Ideographs block (U+4E00..9FFF +
/// Extension A U+3400..4DBF) with Japanese, so this fn lives
/// downstream of `japanese_keyword` in the dispatch chain.
/// Mandarin keyword strings are all pure-Han — they don't
/// collide with the Japanese mixed Kanji+Hiragana / Kanji+
/// Katakana forms used by `japanese_keyword`. SVO grammar so
/// existing keyword-first parser applies.
///
/// Idiomatic separation: users put whitespace between identifiers
/// and keywords (e.g. `函数 add(a: 整数, b: 整数) -> 整数`),
/// same convention as Japanese code. No dictionary-driven
/// segmenter required for v1.
fn mandarin_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        // === DECLARATIONS ===
        "函数" => TokenKind::Fn,             // hánshù (function)
        "函数主要" => TokenKind::Fn,         // alt with main; kept for completeness
        "让" => TokenKind::Let,              // ràng (let / make)
        "结构" => TokenKind::Struct,         // jiégòu (structure)
        "结构体" => TokenKind::Struct,       // jiégòutǐ (struct body)
        "枚举" => TokenKind::Enum,           // méijǔ (enumerate)
        "常量" => TokenKind::Const,          // chángliàng (constant)
        // === VISIBILITY / MODULES ===
        "公开" => TokenKind::Pub,            // gōngkāi (public)
        "模块" => TokenKind::Module,         // mókuài (module)
        "使用" => TokenKind::Use,            // shǐyòng (use)
        "作为" => TokenKind::As,             // zuòwéi (as)
        // === CONTROL FLOW ===
        "返回" => TokenKind::Return,         // fǎnhuí (return)
        "如果" => TokenKind::If,             // rúguǒ (if)
        "否则" => TokenKind::Else,           // fǒuzé (otherwise / else)
        "当" => TokenKind::While,            // dāng (while / when)
        "对于" => TokenKind::For,            // duìyú (for)
        "从" => TokenKind::From,             // cóng (from)
        "到" => TokenKind::To,               // dào (to)
        "中断" => TokenKind::Break,          // zhōngduàn (break / interrupt)
        "继续" => TokenKind::Continue,       // jìxù (continue)
        "那么" => TokenKind::Then,           // nàme (then)
        // === REFS / MUT ===
        "引用" => TokenKind::Ref,            // yǐnyòng (reference)
        "可变" => TokenKind::Mut,            // kěbiàn (mutable / changeable)
        // === MATCH ===
        "匹配" => TokenKind::Match,          // pǐpèi (match)
        // === VERIFICATION ===
        "断言" => TokenKind::Assert,         // duànyán (assert)
        "证明" => TokenKind::Prove,          // zhèngmíng (prove)
        "要求" => TokenKind::Requires,       // yāoqiú (requires / precondition)
        "保证" => TokenKind::Ensures,        // bǎozhèng (ensures / guarantee)
        "不变量" => TokenKind::Invariant,    // bùbiànliàng (invariant)
        // === BOOL / PRINT ===
        "真" => TokenKind::True,             // zhēn (true). NOTE: collides with
                                              // Japanese "shin" (also TokenKind::True),
                                              // which is harmless — both dialects mean True.
        "假" => TokenKind::False,            // jiǎ (false)
        "打印" => TokenKind::Print,          // dǎyìn (print)
        "输出" => TokenKind::Print,          // shūchū (output)
        // === PURITY / PARALLEL ===
        "纯" => TokenKind::Pure,             // chún (pure)
        "纯粹" => TokenKind::Pure,           // chúncuì (pure). NOTE: also Japanese
                                              // "junsui" → TokenKind::Pure; same meaning.
        "并行" => TokenKind::Parallel,       // bìngxíng (parallel)
        // === INTERFACES / METHODS ===
        "接口" => TokenKind::Interface,      // jiēkǒu (interface)
        "实现" => TokenKind::Implement,      // shíxiàn (implement)
        "方法" => TokenKind::Methods,        // fāngfǎ (method)
        // === BOUNDS ===
        "其中" => TokenKind::Where,          // qízhōng (among / where)
        // === CONCURRENCY ===
        "尝试" => TokenKind::Try,            // chángshì (try)
        "任务" => TokenKind::Task,           // rènwù (task)
        "等待" => TokenKind::Join,           // děngdài (wait / join)
        "合并" => TokenKind::Join,           // hébìng (merge / join)
        // === EMBEDDED ===
        "不安全" => TokenKind::Unsafe,       // bù'ānquán (unsafe)
        "区域" => TokenKind::RegionKw,       // qūyù (region / area)
        // === SOV-S7 PARITY ===
        "目的" => TokenKind::Intent,         // mùdì (purpose). NOTE: same string
                                              // as Japanese "mokuteki" → Intent —
                                              // both dialects use 目的 for intent.
        "意图" => TokenKind::Intent,         // yìtú (intention)
        "类型" => TokenKind::Type,           // lèixíng (type)
        "外部" => TokenKind::Extern,         // wàibù (external)
        _ => return None,
    };
    Some(kind)
}

/// Phase 8b.3 (2026-06-07): French (français) keyword resolution.
/// Second Latin-with-accents Tier II dialect. Same v1 design as
/// Spanish: only natural non-ASCII French keywords are registered,
/// so pure-ASCII French keywords (`si`, `pour`, `module`, etc.)
/// don't accidentally collide with user identifiers in non-pragma
/// files. Pragma threading queued for v2.
fn french_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        // === DECLARATIONS ===
        "énumération" => TokenKind::Enum,    // enumeration
        // === REFS / MUT / VIS ===
        "référence" => TokenKind::Ref,       // reference
        // === MATCH / VERIFICATION ===
        "vérifier" => TokenKind::Assert,     // verify (infinitive)
        "vérifie" => TokenKind::Assert,      // verify! (imperative)
        "démontrer" => TokenKind::Prove,     // prove (infinitive)
        "démontre" => TokenKind::Prove,      // prove! (imperative)
        // === BOOL ===
        "vérité" => TokenKind::True,         // truth
        // === PRINT ===
        "écrire" => TokenKind::Print,        // to write
        "écris" => TokenKind::Print,         // write! (imperative)
        "imprimé" => TokenKind::Print,       // printed (past participle alt)
        // === BOUNDS ===
        "où" => TokenKind::Where,            // where (très naturel)
        // === CONCURRENCY ===
        "tâche" => TokenKind::Task,          // task
        "parallèle" => TokenKind::Parallel,  // parallel
        // === INTERFACES / METHODS ===
        "méthodes" => TokenKind::Methods,    // methods
        "implémenter" => TokenKind::Implement, // implement
        // === SOV-S7 PARITY ===
        "région" => TokenKind::RegionKw,     // region
        "étranger" => TokenKind::Extern,     // foreign / external
        _ => return None,
    };
    Some(kind)
}

/// Phase 8b.2 (2026-06-07): Russian (русский) keyword resolution.
/// First Cyrillic-script dialect. SVO grammar so existing keyword-
/// first statement parser applies directly — no SOV plumbing.
/// Uses Arabic 0-9 numerals (Cyrillic numeric letter notation is
/// archaic, not used in modern Russian).
fn cyrillic_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        // === DECLARATIONS ===
        "функция" => TokenKind::Fn,           // funktsiya (function — loanword)
        "дело" => TokenKind::Fn,              // delo (work — alt)
        "пусть" => TokenKind::Let,            // pust' (let — natural Russian)
        "структура" => TokenKind::Struct,     // struktura (structure)
        "перечисление" => TokenKind::Enum,    // perechislenie (enumeration)
        "постоянная" => TokenKind::Const,     // postoyannaya (constant)
        // === VISIBILITY / MODULES ===
        "публичный" => TokenKind::Pub,        // publichnyy (public — formal)
        "общий" => TokenKind::Pub,            // obshchiy (common/public)
        "модуль" => TokenKind::Module,        // modul' (module)
        "использовать" => TokenKind::Use,     // ispol'zovat' (use)
        "как" => TokenKind::As,               // kak (as)
        // === CONTROL FLOW ===
        "вернуть" => TokenKind::Return,       // vernut' (return)
        "верни" => TokenKind::Return,         // verni (return! — imperative)
        "если" => TokenKind::If,              // yesli (if)
        "иначе" => TokenKind::Else,           // inache (else)
        "пока" => TokenKind::While,           // poka (while/as long as)
        "для" => TokenKind::For,              // dlya (for)
        "в" => TokenKind::In,                 // v (in)
        "от" => TokenKind::From,              // ot (from)
        "до" => TokenKind::To,                // do (to/until)
        "прервать" => TokenKind::Break,       // prervat' (interrupt/break)
        "продолжить" => TokenKind::Continue,  // prodolzhit' (continue)
        "тогда" => TokenKind::Then,           // togda (then)
        // === REFS / MUT ===
        "смотри" => TokenKind::Ref,           // smotri (see/look! — imperative)
        "изменяемый" => TokenKind::Mut,       // izmenyayemyy (mutable)
        // === MATCH ===
        "совпадение" => TokenKind::Match,     // sovpadeniye (match/coincidence)
        // === VERIFICATION ===
        "утверждать" => TokenKind::Assert,    // utverzhdat' (assert)
        "доказать" => TokenKind::Prove,       // dokazat' (prove)
        "требует" => TokenKind::Requires,     // trebuet (requires)
        "гарантирует" => TokenKind::Ensures,  // garantiruet (ensures)
        // === BOOL / PRINT ===
        "истина" => TokenKind::True,          // istina (truth — formal)
        "верно" => TokenKind::True,           // verno (correct/right — everyday)
        "ложь" => TokenKind::False,           // lozh' (lie — formal)
        "неверно" => TokenKind::False,        // neverno (incorrect — everyday)
        "печатать" => TokenKind::Print,       // pechatat' (print)
        "писать" => TokenKind::Print,         // pisat' (write — alt)
        // === PURITY / PARALLEL ===
        "чистый" => TokenKind::Pure,          // chistyy (pure)
        "параллельный" => TokenKind::Parallel, // parallel'nyy (parallel)
        // === INTERFACES / METHODS ===
        "интерфейс" => TokenKind::Interface,  // interfeys (interface — loanword)
        "реализовать" => TokenKind::Implement, // realizovat' (implement)
        "методы" => TokenKind::Methods,       // metody (methods)
        // === BOUNDS ===
        "где" => TokenKind::Where,            // gde (where)
        "есть" => TokenKind::Is,              // yest' (is — Russian copula)
        // === CONCURRENCY ===
        "попытка" => TokenKind::Try,          // popytka (attempt/try)
        "задача" => TokenKind::Task,          // zadacha (task)
        "соединить" => TokenKind::Join,       // soyedinit' (join/unite)
        // === EMBEDDED ===
        "небезопасно" => TokenKind::Unsafe,   // nebezopasno (unsafe)
        "область" => TokenKind::RegionKw,     // oblast' (region/area)
        // === SOV-S7 PARITY (not actually SOV here — Russian is
        // SVO — but these are the keyword names) ===
        "цель" => TokenKind::Intent,          // tsel' (goal/intent)
        "тип" => TokenKind::Type,             // tip (type)
        "внешний" => TokenKind::Extern,       // vneshniy (external)
        "инвариант" => TokenKind::Invariant,  // invariant (loanword)
        _ => return None,
    };
    Some(kind)
}

pub fn lex(source: &str) -> Result<Vec<Token>, Diagnostic> {
    let mut tokens = Lexer::new(source).lex()?;
    merge_multi_word_devanagari_aliases(&mut tokens, source);
    merge_give_back_ascii_alias(&mut tokens, source);
    enforce_language_purity(&tokens, source)?;
    // Phase 1.1 (2026-06-07): record the file's pragma so the
    // backends can switch numeric print output to Devanagari
    // digits when the source declared a Devanagari dialect.
    // Reset every lex() so per-test isolation holds.
    let mode = match detect_language_pragma(source) {
        Some(DialectLang::Sanskrit)
        | Some(DialectLang::Hindi)
        | Some(DialectLang::Marathi)
        | Some(DialectLang::Nepali)
        | Some(DialectLang::Maithili)
        | Some(DialectLang::KonkaniDev) => PrintLangMode::Devanagari,
        // Phase 5b (2026-06-07): Bengali numerals (০..৯) live in
        // their own UTF-8 codepoint range. Separate helper in
        // each backend keeps the Devanagari path byte-identical.
        Some(DialectLang::Bengali) => PrintLangMode::Bengali,
        // Phase 6 (2026-06-07): one PrintLangMode per Brahmi
        // script so each backend can dispatch to the right
        // numeral helper without runtime conditionals.
        Some(DialectLang::Tamil) => PrintLangMode::Tamil,
        Some(DialectLang::Telugu) => PrintLangMode::Telugu,
        Some(DialectLang::Gujarati) => PrintLangMode::Gujarati,
        Some(DialectLang::Punjabi) => PrintLangMode::Gurmukhi,
        // Phase 6 second half (2026-06-07).
        Some(DialectLang::Kannada) => PrintLangMode::Kannada,
        Some(DialectLang::Malayalam) => PrintLangMode::Malayalam,
        Some(DialectLang::Odia) => PrintLangMode::Odia,
        // Assamese shares the Bengali Unicode block; numeral
        // codepoints are identical (U+09E6..09EF).
        Some(DialectLang::Assamese) => PrintLangMode::Bengali,
        Some(DialectLang::Sinhala) => PrintLangMode::Sinhala,
        // Phase 12 (2026-06-07). Sindhi + Shahmukhi reuse the
        // Urdu print helper since they all use Eastern Arabic-
        // Indic numerals ٠..٩.
        Some(DialectLang::Urdu)
        | Some(DialectLang::Sindhi)
        | Some(DialectLang::PunjabiShahmukhi)
            => PrintLangMode::Urdu,
        // Phase 12.4/12.5: Persian + Pashto use Persian-Indic
        // numerals ('۰..۹') — distinct UTF-8 byte sequence
        // from Urdu's Eastern Arabic-Indic.
        Some(DialectLang::Persian)
        | Some(DialectLang::Pashto)
            => PrintLangMode::Persian,
        _ => PrintLangMode::Ascii,
    };
    PROGRAM_PRINT_LANG_MODE.with(|c| c.set(mode));
    Ok(tokens)
}

/// Phase 1.1 (2026-06-07): per-program print-output language
/// mode. Set by `lex()` from the `// vani-lang:` pragma; read by
/// the C + LLVM backends at print-emit time to choose between
/// ASCII `%lld` and a Devanagari-digit helper.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum PrintLangMode {
    Ascii,
    Devanagari,
    // Phase 5b (2026-06-07): Bengali numerals — distinct
    // codepoints (U+09E6..U+09EF) from Devanagari (U+0966..U+096F).
    // Each backend dispatches to its own helper based on this.
    Bengali,
    // Phase 6 (2026-06-07): Brahmi-derived language batch.
    // Each script has its own numeral block at U+xxE6..xxEF
    // (or equivalent offset). One PrintLangMode variant per
    // script lets each backend dispatch to the right helper.
    Tamil,      // U+0BE6..0BEF '௦..௯'
    Telugu,     // U+0C66..0C6F '౦..౯'
    Gujarati,   // U+0AE6..0AEF '૦..૯'
    Gurmukhi,   // U+0A66..0A6F '੦..੯' (Punjabi-Gurmukhi)
    // Phase 6 second half (2026-06-07).
    Kannada,    // U+0CE6..0CEF '೦..೯'
    Malayalam,  // U+0D66..0D6F '൦..൯'
    Odia,       // U+0B66..0B6F '୦..୯'
    Sinhala,    // U+0DE6..0DEF '෦..෯'  (Lith Illakkam — modern set)
    // Phase 12 (2026-06-07): first non-Brahmi, Perso-Arabic
    // (RTL). Eastern Arabic-Indic digits '٠..٩' at
    // U+0660..0669 → 2-byte UTF-8 `D9 A0+d`.
    Urdu,
    // Phase 12.4 (2026-06-07): Persian (Extended) Arabic-Indic
    // digits '۰..۹' at U+06F0..06F9 → 2-byte UTF-8 `DB B0+d`.
    // Used by Persian/Farsi and Pashto.
    Persian,
}

thread_local! {
    static PROGRAM_PRINT_LANG_MODE: std::cell::Cell<PrintLangMode> =
        const { std::cell::Cell::new(PrintLangMode::Ascii) };
}

pub fn current_print_lang_mode() -> PrintLangMode {
    PROGRAM_PRINT_LANG_MODE.with(|c| c.get())
}

pub fn set_current_print_lang_mode(mode: PrintLangMode) {
    PROGRAM_PRINT_LANG_MODE.with(|c| c.set(mode));
}

/// Per-file language purity gate (closure #236). vāṇī supports
/// English structure keywords (`fn`, `let`, `return`, …) and a
/// Devanagari alias table covering Sanskrit / Hindi / Marathi.
/// A file should commit to ONE script: mixing the English form
/// with Devanagari forms in the same file surfaces as a clear
/// "language mismatch" diagnostic so the reader doesn't have to
/// mentally parse two structure-keyword systems at once.
///
/// V1 enforces script-level purity (English vs Devanagari).
/// Finer-grained Sanskrit / Hindi / Marathi distinction within
/// Devanagari is deferred — the existing alias table maps some
/// words ambiguously (e.g. `यदि` is both Sanskrit and Hindi).
/// Grammar-consultant review is the gate for that next step.
///
/// Type names (`i64`, `bool`, `Vec`, …) and the boolean literals
/// (`true`/`false`) stay neutral so a Hindi file can still write
/// `फलन add(a: i64, b: i64) -> i64`. The gate looks only at
/// structure keywords.
fn script_label(script: Script) -> &'static str {
    match script {
        Script::Latin => "English",
        Script::Devanagari => "Devanagari",
        Script::Bengali => "Bengali",
        Script::Tamil => "Tamil",
        Script::Telugu => "Telugu",
        Script::Gujarati => "Gujarati",
        Script::Gurmukhi => "Gurmukhi (Punjabi)",
        Script::Kannada => "Kannada",
        Script::Malayalam => "Malayalam",
        Script::Odia => "Odia",
        Script::Sinhala => "Sinhala",
        Script::Arabic => "Perso-Arabic",
        Script::Cyrillic => "Cyrillic",
        Script::Japanese => "CJK (Han / Hiragana / Katakana)",
        Script::Hangul => "Hangul (Korean)",
        Script::Greek => "Greek",
        Script::Hebrew => "Hebrew",
        Script::Thai => "Thai",
        Script::Armenian => "Armenian",
        Script::Georgian => "Georgian",
        Script::Khmer => "Khmer",
        Script::Burmese => "Myanmar (Burmese)",
        Script::Ethiopic => "Ethiopic",
        Script::Tibetan => "Tibetan",
        Script::Cherokee => "Cherokee",
        Script::Lao => "Lao",
        Script::Mongolian => "Mongolian (traditional)",
    }
}

fn enforce_language_purity(tokens: &[Token], source: &str) -> Result<(), Diagnostic> {
    // SOV-S8 (2026-06-06) + Phase 5b/6 (2026-06-07): per-file
    // script purity. Track the first observed script's span;
    // any later keyword from a different script is rejected.
    // The `// vani-lang:` pragma, when present, narrows further
    // by requiring the observed script == the dialect's script.
    //
    // The N² explicit per-pair checks the older code had are
    // collapsed into one comparison against the first script.
    // Adding a new Brahmi-derived script (Phase 6 cont'd: Kannada,
    // Malayalam, Odia, Sinhala, …) is now a no-op here.
    let declared = detect_language_pragma(source);
    let mut first_seen: Option<(Script, Span)> = None;
    for tok in tokens {
        if !is_structure_keyword_kind(&tok.kind) {
            continue;
        }
        let text = &source[tok.span.start..tok.span.end];
        let script = Script::classify(text);
        // Cross-script mixing check.
        if let Some((prior_script, prior_span)) = first_seen {
            if script != prior_script {
                let level = if prior_script == Script::Latin || script == Script::Latin {
                    "language"
                } else {
                    "script"
                };
                return Err(Diagnostic::new(
                    tok.span,
                    format!(
                        "{} mismatch: file already used a {} structure keyword \
                         (see span {}..{}), can't switch to a {} alias mid-file. \
                         Pick one script per file.",
                        level,
                        script_label(prior_script),
                        prior_span.start, prior_span.end,
                        script_label(script),
                    ),
                ));
            }
        } else {
            first_seen = Some((script, tok.span));
        }
        // Pragma narrowing.
        if let Some(lang) = declared {
            let declared_script = lang.script();
            if declared_script != script {
                // English-pragma + non-Latin keyword OR non-English-pragma
                // + Latin keyword are both real bugs. (We do NOT
                // reject Latin keywords in a Devanagari-pragma file
                // with no Devanagari structure keywords — that's the
                // back-compat case where `vanic fmt` canonicalized
                // a Devanagari file to English keywords while
                // keeping the pragma; the cross-script check above
                // already accepts this scenario since first_seen
                // becomes Latin.)
                if lang == DialectLang::English && script != Script::Latin {
                    return Err(Diagnostic::new(
                        tok.span,
                        format!(
                            "vani-lang pragma declared `english` but the file \
                             uses a {} structure keyword `{}` — pick one \
                             dialect per file or drop the pragma to fall back \
                             to script-level purity.",
                            script_label(script), text
                        ),
                    ));
                }
                if script != Script::Latin && declared_script != Script::Latin {
                    return Err(Diagnostic::new(
                        tok.span,
                        format!(
                            "vani-lang pragma declared `{}` ({} script) but the \
                             file uses {} keyword `{}` — pick one script per file.",
                            lang.name(),
                            script_label(declared_script),
                            script_label(script),
                            text,
                        ),
                    ));
                }
            }
            // Devanagari sub-dialect narrowing (Sanskrit vs Hindi vs Marathi).
            // Only applies when both the pragma and the keyword are
            // Devanagari-script.
            if script == Script::Devanagari
                && declared_script == Script::Devanagari
                && !spelling_supports_dialect(text, lang)
            {
                return Err(Diagnostic::new(
                    tok.span,
                    format!(
                        "vani-lang pragma declared `{}` but Devanagari \
                         keyword `{}` is not in that dialect's keyword set. \
                         Use an alias supported by your declared dialect, or \
                         drop the pragma to allow any Devanagari alias.",
                        lang.name(), text
                    ),
                ));
            }
        }
    }
    Ok(())
}

/// SOV-S8 — the four supported language pragmas for finer-than-
/// script-level purity. `English` is included so the user can
/// explicitly opt out of mixed-script files even when the source
/// is purely English-keyword (otherwise the gate is no-op there).
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum DialectLang {
    Sanskrit,
    Hindi,
    Marathi,
    English,
    // Phase 2 (2026-06-07): Tier I Indo-Aryan dialects sharing
    // the Devanagari script + heavy tatsama (Sanskrit-rooted)
    // vocabulary with the original three. v1 accepts the
    // union of the Sanskrit/Hindi/Marathi keyword set for these
    // dialects; native-vernacular spellings can be layered on
    // as user requests come in.
    Nepali,
    Maithili,
    KonkaniDev,
    // Phase 5b (2026-06-07): first non-Devanagari Brahmi-derived
    // script. Indo-Aryan SOV grammar shared with the Devanagari
    // group; the Unicode block (U+0980..U+09FF) is disjoint so
    // the script-purity gate must generalize from "Devanagari vs
    // English" to N-script. Future Tamil / Telugu / Kannada /
    // Malayalam / Odia / Assamese / Sinhala / Gujarati / Punjabi-
    // Gurmukhi reuse this per-script abstraction.
    Bengali,
    // Phase 6 (2026-06-07): Brahmi-derived batch — first four
    // languages riding the per-script abstraction set up in
    // Phase 5b. Tamil + Telugu are Dravidian (non-Indo-Aryan,
    // so the tatsama vocabulary assumption breaks); Gujarati +
    // Punjabi-Gurmukhi are Indo-Aryan with tatsama-friendly
    // technical vocabulary. Each gets its own Unicode block,
    // keyword table, numeral codepoints, and pragma.
    Tamil,
    Telugu,
    Gujarati,
    Punjabi,    // Gurmukhi-script Punjabi (Indian). Shahmukhi (Pakistani,
                // Perso-Arabic, RTL) deferred — different bidirectionality.
    // Phase 6 second half (2026-06-07): remaining Brahmi-derived
    // scripts. Kannada + Malayalam are Dravidian; Odia + Assamese
    // + Sinhala are Indo-Aryan. Assamese is the architectural
    // odd-one-out — it shares the Bengali Unicode block with
    // only a couple of Assamese-specific characters, so the
    // dialect maps to `Script::Bengali` rather than getting its
    // own script enum variant.
    Kannada,
    Malayalam,
    Odia,
    Assamese,   // Bengali-script Indo-Aryan; aliases pulled from the Bengali table.
    Sinhala,
    // Phase 12 (2026-06-07): first Perso-Arabic dialect. Urdu
    // (اردو) is Indo-Aryan with Hindustani vocabulary shared
    // with Hindi; the script (and surface conventions) are
    // Perso-Arabic. RTL text-direction is a rendering concern
    // — the lexer reads UTF-8 in logical order, so no special
    // bidi handling is needed at the parse level.
    Urdu,
    // Phase 12.2 (2026-06-07): Sindhi (سنڌي), Indo-Aryan,
    // Pakistani/Indian. Perso-Arabic script, same Eastern
    // Arabic-Indic numerals as Urdu. v1 accepts the Urdu
    // keyword union; native Sindhi vocabulary layered later.
    Sindhi,
    // Phase 12.3 (2026-06-07): Punjabi-Shahmukhi (Pakistani
    // Punjabi), the Perso-Arabic counterpart to Punjabi-
    // Gurmukhi already shipped in Phase 6.
    PunjabiShahmukhi,
    // Phase 12.4 (2026-06-07): Persian / Farsi (فارسی),
    // Iranian. Same script family but its OWN numeral block:
    // U+06F0..06F9 '۰..۹' encode to UTF-8 `DB B0+d` instead
    // of Urdu's `D9 A0+d`. Validates the variable-prefix
    // helper across a second non-Brahmi numeral block.
    Persian,
    // Phase 12.5 (2026-06-07): Pashto (پښتو), Iranian /
    // Pakistan + Afghanistan. Extended Arabic alphabet with
    // Pashto-specific letters (ښ ګ ړ etc). Uses Persian-Indic
    // numerals in modern publishing.
    Pashto,
    // Phase 8b.2 (2026-06-07): Russian (русский) — first Tier
    // II Cyrillic-script dialect. SVO grammar so no SOV
    // statement-shape plumbing is needed; existing keyword-
    // first statement parser applies. Uses Arabic 0-9 numerals
    // (Cyrillic-numeral letter notation is archaic and not used
    // in modern Russian) — routes through the ASCII print
    // helper, no new numeral codepoint helper required.
    Russian,
    // Phase 8b.1 (2026-06-07): Spanish (español) — first Tier
    // II Latin-script (with accents) dialect. SVO grammar so
    // existing keyword-first statement parser applies. Stays
    // on Script::Latin since Spanish keywords with accents
    // (función, módulo, público) live in Latin-1 Supplement
    // (U+0080..U+00FF), which is not its own script — the
    // accented chars are just continuation bytes within Latin.
    Spanish,
    // Phase 8b.3 (2026-06-07): French (français) — second
    // Latin-with-accents Tier II dialect. Rides the same
    // unified `lex_ident` infrastructure shipped with Spanish.
    // SVO grammar, Latin-1 Supplement accents (é, è, à, ù, ê,
    // â, ô, î, ï, ç).
    French,
    // Phase 9b (2026-06-07): Japanese (日本語) — first three-
    // script dialect (Hiragana + Katakana + Kanji) and first
    // non-Indic SOV target. v1 ships keyword-first surface
    // (関数 main() { 戻る 0; }) — Japanese SOV grammar forms
    // (もし x ならば { ... }) queued for v2 once the SOV
    // statement-shape detector generalizes beyond Devanagari.
    Japanese,
    // Phase 10.1 (2026-06-07): German (Deutsch) — third
    // Latin-with-accents Tier II dialect. Rides the same
    // unified `lex_ident` infrastructure as Spanish + French.
    // German umlauts ä/ö/ü and ß all live in Latin-1
    // Supplement (U+00C4, U+00D6, U+00DC, U+00DF and their
    // lowercase counterparts). German is V2 (verb-second) in
    // main clauses and SOV in subordinate clauses, but v1
    // keyword-first surface applies cleanly — V2 / subordinate-
    // SOV parser hooks queued for v2.
    German,
    // Phase 13.1 (2026-06-07): Korean (한국어) — first Hangul-
    // script dialect. SOV grammar like Japanese; the keyword
    // table uses precomposed Hangul syllables (e.g. 함수 for
    // "function" — ham + su). Korean continues the SOV-for-
    // now-keyword-first design from Japanese.
    Korean,
    // Phase 13.2 (2026-06-08): Portuguese (português) — fourth
    // Latin-with-accents Tier II dialect (after Spanish, French,
    // German). Rides both the unified `lex_ident` non-ASCII
    // continuation path AND the pragma-threading enabler so
    // natural pure-ASCII Portuguese (`funcao`, `seja`, `se`,
    // `enquanto`, `verdadeiro`, ...) works alongside accented
    // forms (`função`, `não`, `até`, `senão`). Brazilian and
    // European Portuguese share the same keyword surface.
    Portuguese,
    // Phase 13.3 (2026-06-08): Indonesian (Bahasa Indonesia) —
    // first BASIC-Latin Tier II dialect, fully enabled by the
    // pragma-threading shipped in this session. Indonesian has
    // no diacritics so it can't use the unified-lex_ident path
    // (no non-ASCII anchors); the pragma-gated ASCII keyword
    // table is the only path. SVO grammar — keyword-first
    // works directly.
    Indonesian,
    // Phase 13.4 (2026-06-08): Greek (Ελληνικά) — first
    // Greek-script dialect. Modern Greek uses the monotonic
    // accent system (single acute + diaeresis) for stress
    // marks; the keyword table follows monotonic conventions.
    // SVO grammar.
    Greek,
    // Phase 13.5 (2026-06-08): Hebrew (עברית) — second RTL
    // script dialect after Perso-Arabic. SVO grammar.
    Hebrew,
    // Phase 13.6 (2026-06-08): Italian (italiano) — fifth
    // Latin-with-accents dialect. Italian keyword surface is
    // mostly pure ASCII (`funzione`, `sia`, `se`, `mentre`,
    // `per`, `vero`, `falso`, ...), so the dialect rides the
    // pragma-threading enabler primarily; a handful of accented
    // forms (`è` etc.) are deliberately omitted in v1 since
    // they're single-char or otherwise ambiguous as keywords.
    Italian,
    // Phase 13.7 (2026-06-08): Modern Standard Arabic (العربية)
    // — distinct from the shipped Perso-Arabic dialects which
    // use the Arabic SCRIPT for Indo-Iranian/Indo-Aryan
    // languages (Urdu, Sindhi, Shahmukhi, Persian, Pashto).
    // Native Arabic vocabulary on the existing Script::Arabic
    // infrastructure. SVO/VSO grammar; v1 ships keyword-first.
    Arabic,
    // Phase 13.8 (2026-06-08): Polish (polski) — sixth
    // Latin-with-accents Tier II dialect. First Slavic Latin
    // variant. Polish uses extensive diacritics: ą/ć/ę/ł/ń/ó/ś/ź/ż
    // — natural non-ASCII keyword forms exist for many words.
    Polish,
    // Phase 13.9 (2026-06-08): Turkish (Türkçe) — seventh
    // Latin-with-accents dialect, Turkic family. Distinctive
    // dotless ı / dotted İ + ç/ğ/ö/ş/ü diacritics. Agglutinative
    // but v1 keyword set is small enough that the SVO-ish
    // surface works keyword-first.
    Turkish,
    // Phase 13.10 (2026-06-08): Malay (Bahasa Melayu) — second
    // basic-Latin Tier II dialect after Indonesian. Closely
    // related to Indonesian linguistically; sibling pragma-
    // gated keyword set.
    Malay,
    // Phase 13.11 (2026-06-08): Swahili (Kiswahili) — first
    // African Tier II dialect. Basic Latin alphabet, SVO
    // grammar, lingua franca of East Africa.
    Swahili,
    // Phase 13.12 (2026-06-08): Vietnamese (Tiếng Việt) — first
    // Southeast Asian Tier II dialect using Latin script.
    // Distinctive extensive diacritic + tone-mark system
    // (ă/â/đ/ê/ô/ơ/ư + 5 tone marks combine to ~100+ unique
    // glyphs). SVO grammar.
    Vietnamese,
    // Phase 13.13 (2026-06-08): Romanian (limba română) —
    // completes the Romance family extension in vāṇी (Spanish +
    // French + Italian + Portuguese + Romanian). Distinctive
    // diacritics: ă/â/î/ș/ț.
    Romanian,
    // Phase 13.14 (2026-06-08): Dutch (Nederlands) — basic-
    // Latin Germanic dialect. Mostly pure-ASCII keyword surface
    // (the occasional diaeresis ë/ï in regular text is rare in
    // technical vocabulary).
    Dutch,
    // Phase 13.15 (2026-06-08): Thai (ไทย) — first Thai-script
    // dialect. Block U+0E00..U+0E7F. SVO grammar; no spaces
    // between words in prose Thai, but vāṇी keywords are
    // individually tokenized with surrounding whitespace as
    // separators (same convention modern Thai code uses).
    Thai,
    // Phase 13.16 (2026-06-08): Hungarian (magyar) — Uralic
    // family, first non-Indo-European Latin-script dialect.
    // Distinctive double-acute ő/ű diacritics in addition to
    // standard á/é/í/ó/ö/ú/ü.
    Hungarian,
    // Phase 13.17 (2026-06-08): Czech (čeština) — second Slavic
    // Latin variant (after Polish). Distinctive ř (the only
    // language to use it) + many háček diacritics (č/ď/ě/ň/š/ť/ž
    // + standard á/é/í/ó/ú/ý) plus ů.
    Czech,
    // Phase 13.18 (2026-06-08): Swedish (svenska) — first
    // Nordic dialect. Uses å/ä/ö (the Nordic core diacritic
    // set; Norwegian/Danish use å/æ/ø; Finnish uses ä/ö only).
    Swedish,
    // Phase 13.19 (2026-06-08): Filipino (Tagalog-based) —
    // first Austronesian basic-Latin dialect (alongside the
    // Indonesian/Malay sibling pair already shipped). ~45M
    // speakers in the Philippines.
    Filipino,
    // Phase 13.20 (2026-06-08): Norwegian (norsk bokmål) —
    // second Nordic dialect after Swedish. Uses å/æ/ø (the
    // Norwegian/Danish Nordic core; Swedish swaps æ/ø for ä/ö).
    Norwegian,
    // Phase 13.21 (2026-06-08): Danish (dansk) — third Nordic
    // dialect. Shares å/æ/ø with Norwegian; closely related but
    // distinct keyword choices.
    Danish,
    // Phase 13.22 (2026-06-08): Armenian (Հայերեն) — first
    // Caucasus-region script. Block U+0530..058F. ~6M speakers.
    // The script is alphabetic with both upper- and lowercase
    // forms; vāṇी keywords stay lowercase.
    Armenian,
    // Phase 13.23 (2026-06-08): Georgian (ქართული) — second
    // Caucasus-region script. Block U+10A0..10FF (Asomtavruli /
    // Mkhedruli main script) + U+2D00..2D2F (Nuskhuri). ~4M
    // speakers. Mkhedruli is the standard modern form.
    Georgian,
    // Phase 13.24 (2026-06-08): Slovak (slovenčina) — third
    // Slavic Latin variant (after Polish, Czech). Similar to
    // Czech with ľ/ŕ/ô plus standard Slavic diacritics.
    Slovak,
    // Phase 13.25 (2026-06-08): Finnish (suomi) — second Uralic
    // (after Hungarian). Distinct from Hungarian — different
    // branch of Uralic. Uses ä/ö only.
    Finnish,
    // Phase 13.26 (2026-06-08): Catalan (català) — sixth
    // Romance Latin variant (after Spanish + French + Italian +
    // Portuguese + Romanian). Distinctive interpunct (l·l).
    Catalan,
    // Phase 13.27 (2026-06-08): Yoruba (Èdè Yorùbá) — Niger-
    // Congo family, ~50M speakers in West Africa. Latin script
    // with extensive sub-dot marks (ẹ/ọ/ṣ) + tone marks.
    Yoruba,
    // Phase 13.28 (2026-06-08): Hausa — Afroasiatic family,
    // ~80M speakers (largely in Nigeria + Niger). Latin Boko
    // script with implosive consonants ɓ/ɗ/ƙ/ƴ.
    Hausa,
    // Phase 13.29 (2026-06-08): Khmer (ខ្មែរ) — block
    // U+1780..17FF. ~16M speakers in Cambodia.
    Khmer,
    // Phase 13.30 (2026-06-08): Burmese (မြန်မာ) — Myanmar
    // block U+1000..109F. ~33M speakers.
    Burmese,
    // Phase 13.31 (2026-06-08): Amharic (አማርኛ) — Ethiopic
    // block U+1200..137F. ~32M speakers in Ethiopia.
    Amharic,
    // Phase 13.32 (2026-06-08): Tibetan (བོད་ཡིག) — block
    // U+0F00..0FFF. ~7M speakers.
    Tibetan,
    // Phase 13.33 (2026-06-08): Cherokee (ᏣᎳᎩ) — Cherokee
    // syllabary U+13A0..13FF. Endangered (~2K speakers); v1
    // ships a minimal keyword set to give the syllabary a
    // host in vāṇी.
    Cherokee,
    // Phase 13.34 (2026-06-08): Lao (ລາວ) — block
    // U+0E80..0EFF. ~30M speakers; closely related to Thai.
    Lao,
    // Phase 13.35 (2026-06-08): Mongolian (ᠮᠣᠩᠭᠣᠯ) — traditional
    // vertical script block U+1800..18AF. ~6M speakers
    // (Inner Mongolia). The lexer reads UTF-8 in logical
    // (byte) order so vertical rendering is purely a display
    // concern.
    Mongolian,
    // Phase 10.2 (2026-06-08): Mandarin Chinese (中文) — the
    // load-bearing CJK target. ~1.1B speakers. Shares the
    // CJK Unified Ideographs block (U+4E00..9FFF + Extension A
    // U+3400..4DBF) with Japanese, so the same `Script::Japanese`
    // slot covers both for the purity gate. Disambiguation
    // between the two relies on the pragma + the keyword
    // table: Japanese keywords use mixed Kanji + Hiragana
    // (関数 / もし); Mandarin keywords are pure-Han (函数 /
    // 如果). Users separate identifiers from keywords with
    // whitespace, same as the Japanese convention — no
    // dictionary-driven segmenter required for v1.
    Mandarin,
}

impl DialectLang {
    fn name(self) -> &'static str {
        match self {
            DialectLang::Sanskrit => "sanskrit",
            DialectLang::Hindi => "hindi",
            DialectLang::Marathi => "marathi",
            DialectLang::English => "english",
            DialectLang::Nepali => "nepali",
            DialectLang::Maithili => "maithili",
            DialectLang::KonkaniDev => "konkani",
            DialectLang::Bengali => "bengali",
            DialectLang::Tamil => "tamil",
            DialectLang::Telugu => "telugu",
            DialectLang::Gujarati => "gujarati",
            DialectLang::Punjabi => "punjabi",
            DialectLang::Kannada => "kannada",
            DialectLang::Malayalam => "malayalam",
            DialectLang::Odia => "odia",
            DialectLang::Assamese => "assamese",
            DialectLang::Sinhala => "sinhala",
            DialectLang::Urdu => "urdu",
            DialectLang::Sindhi => "sindhi",
            DialectLang::PunjabiShahmukhi => "punjabi-shahmukhi",
            DialectLang::Persian => "persian",
            DialectLang::Pashto => "pashto",
            DialectLang::Russian => "russian",
            DialectLang::Spanish => "spanish",
            DialectLang::French => "french",
            DialectLang::Japanese => "japanese",
            DialectLang::German => "german",
            DialectLang::Korean => "korean",
            DialectLang::Portuguese => "portuguese",
            DialectLang::Indonesian => "indonesian",
            DialectLang::Greek => "greek",
            DialectLang::Hebrew => "hebrew",
            DialectLang::Italian => "italian",
            DialectLang::Arabic => "arabic",
            DialectLang::Polish => "polish",
            DialectLang::Turkish => "turkish",
            DialectLang::Malay => "malay",
            DialectLang::Swahili => "swahili",
            DialectLang::Vietnamese => "vietnamese",
            DialectLang::Romanian => "romanian",
            DialectLang::Dutch => "dutch",
            DialectLang::Thai => "thai",
            DialectLang::Hungarian => "hungarian",
            DialectLang::Czech => "czech",
            DialectLang::Swedish => "swedish",
            DialectLang::Filipino => "filipino",
            DialectLang::Norwegian => "norwegian",
            DialectLang::Danish => "danish",
            DialectLang::Armenian => "armenian",
            DialectLang::Georgian => "georgian",
            DialectLang::Slovak => "slovak",
            DialectLang::Finnish => "finnish",
            DialectLang::Catalan => "catalan",
            DialectLang::Yoruba => "yoruba",
            DialectLang::Hausa => "hausa",
            DialectLang::Khmer => "khmer",
            DialectLang::Burmese => "burmese",
            DialectLang::Amharic => "amharic",
            DialectLang::Tibetan => "tibetan",
            DialectLang::Cherokee => "cherokee",
            DialectLang::Lao => "lao",
            DialectLang::Mongolian => "mongolian",
            DialectLang::Mandarin => "mandarin",
        }
    }

    /// Phase 5b (2026-06-07): which Brahmi-derived script does
    /// this dialect use? Drives both the lexer's per-script
    /// purity gate AND the per-script numeral PRINT helper
    /// selection. English maps to Latin; the original three
    /// Devanagari dialects + their Tier-I extensions all map
    /// to Devanagari; Bengali (the first non-Devanagari Brahmi
    /// script) maps to its own script.
    fn script(self) -> Script {
        match self {
            DialectLang::English => Script::Latin,
            DialectLang::Sanskrit
            | DialectLang::Hindi
            | DialectLang::Marathi
            | DialectLang::Nepali
            | DialectLang::Maithili
            | DialectLang::KonkaniDev => Script::Devanagari,
            DialectLang::Bengali => Script::Bengali,
            DialectLang::Tamil => Script::Tamil,
            DialectLang::Telugu => Script::Telugu,
            DialectLang::Gujarati => Script::Gujarati,
            DialectLang::Punjabi => Script::Gurmukhi,
            DialectLang::Kannada => Script::Kannada,
            DialectLang::Malayalam => Script::Malayalam,
            DialectLang::Odia => Script::Odia,
            // Phase 6 (2026-06-07): Assamese is written in
            // the Bengali script with two Assamese-specific
            // characters (`ৰ`, `ৱ`) layered on top. v1 routes
            // it through Script::Bengali — the Bengali keyword
            // table already covers Assamese.
            DialectLang::Assamese => Script::Bengali,
            DialectLang::Sinhala => Script::Sinhala,
            DialectLang::Urdu => Script::Arabic,
            DialectLang::Sindhi => Script::Arabic,
            DialectLang::PunjabiShahmukhi => Script::Arabic,
            DialectLang::Persian => Script::Arabic,
            DialectLang::Pashto => Script::Arabic,
            DialectLang::Russian => Script::Cyrillic,
            DialectLang::Spanish => Script::Latin,
            DialectLang::French => Script::Latin,
            DialectLang::Japanese => Script::Japanese,
            DialectLang::German => Script::Latin,
            DialectLang::Korean => Script::Hangul,
            DialectLang::Portuguese => Script::Latin,
            DialectLang::Indonesian => Script::Latin,
            DialectLang::Greek => Script::Greek,
            DialectLang::Hebrew => Script::Hebrew,
            DialectLang::Italian => Script::Latin,
            DialectLang::Arabic => Script::Arabic,
            DialectLang::Polish => Script::Latin,
            DialectLang::Turkish => Script::Latin,
            DialectLang::Malay => Script::Latin,
            DialectLang::Swahili => Script::Latin,
            DialectLang::Vietnamese => Script::Latin,
            DialectLang::Romanian => Script::Latin,
            DialectLang::Dutch => Script::Latin,
            DialectLang::Thai => Script::Thai,
            DialectLang::Hungarian => Script::Latin,
            DialectLang::Czech => Script::Latin,
            DialectLang::Swedish => Script::Latin,
            DialectLang::Filipino => Script::Latin,
            DialectLang::Norwegian => Script::Latin,
            DialectLang::Danish => Script::Latin,
            DialectLang::Armenian => Script::Armenian,
            DialectLang::Georgian => Script::Georgian,
            DialectLang::Slovak => Script::Latin,
            DialectLang::Finnish => Script::Latin,
            DialectLang::Catalan => Script::Latin,
            DialectLang::Yoruba => Script::Latin,
            DialectLang::Hausa => Script::Latin,
            DialectLang::Khmer => Script::Khmer,
            DialectLang::Burmese => Script::Burmese,
            DialectLang::Amharic => Script::Ethiopic,
            DialectLang::Tibetan => Script::Tibetan,
            DialectLang::Cherokee => Script::Cherokee,
            DialectLang::Lao => Script::Lao,
            DialectLang::Mongolian => Script::Mongolian,
            DialectLang::Mandarin => Script::Japanese,
        }
    }
}

/// Phase 5b/6 (2026-06-07): script abstraction underlying the
/// language-purity gate. `Latin` covers ASCII keywords (English);
/// `Devanagari` covers Sanskrit / Hindi / Marathi and the Tier-I
/// extensions Nepali / Maithili / Konkani. Each Brahmi-derived
/// script ships as its own variant — disjoint Unicode blocks
/// mean a single keyword can only belong to one script.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Script {
    Latin,
    Devanagari,
    Bengali,
    Tamil,
    Telugu,
    Gujarati,
    Gurmukhi,   // Punjabi-Gurmukhi
    // Phase 6 second half (2026-06-07).
    Kannada,
    Malayalam,
    Odia,
    Sinhala,
    // Phase 12 (2026-06-07): Perso-Arabic — first non-Brahmi
    // script. U+0600..U+06FF + Arabic Supplement + Arabic
    // Extended ranges. RTL text-direction is a rendering
    // detail; the lexer reads UTF-8 in logical (byte) order.
    Arabic,
    // Phase 8b.2 (2026-06-07): Cyrillic — first non-Brahmi,
    // non-Arabic script. Russian + later Ukrainian / Belarusian
    // / Bulgarian / Serbian Cyrillic. Block U+0400..U+04FF +
    // Cyrillic Supplement U+0500..U+052F + Cyrillic Extended-A
    // U+2DE0..U+2DFF and Extended-B U+A640..U+A69F. LTR like
    // Latin so no bidi handling needed.
    Cyrillic,
    // Phase 9b (2026-06-07): Japanese — first script that
    // legitimately mixes three Unicode blocks within a single
    // keyword. Hiragana (U+3040..U+309F, phonetic native),
    // Katakana (U+30A0..U+30FF, phonetic foreign), CJK Unified
    // Ideographs (U+4E00..U+9FFF, Kanji), and CJK Extension A
    // (U+3400..U+4DBF). All three blocks collapse to this one
    // Script variant so a Japanese-pragma file is free to mix
    // 関数 (Kanji) + タスク (Katakana) + ならば (Hiragana) the
    // way native Japanese code does. LTR.
    Japanese,
    // Phase 13.1 (2026-06-07): Hangul — Korean's featural
    // alphabet packaged as precomposed syllables. Block
    // U+AC00..U+D7AF (Hangul Syllables) is the main code-point
    // range used by modern Korean; U+1100..U+11FF (Hangul Jamo)
    // gives the underlying jamo components and U+A960..U+A97F
    // / U+D7B0..U+D7FF carry the Extended-A/B supplements.
    // Korean is SOV; keyword-first surface in v1 (SOV grammar
    // queued behind the same generalization needed for
    // Japanese).
    Hangul,
    // Phase 13.4 (2026-06-08): Greek — first Greek-script
    // dialect. Block U+0370..U+03FF (Greek and Coptic) covers
    // both modern and polytonic Greek letters; U+1F00..U+1FFF
    // (Greek Extended) carries the polytonic-accented forms
    // used in classical / ecclesiastical texts. Modern Greek
    // uses the monotonic system (single acute accent + diaeresis)
    // and is SVO.
    Greek,
    // Phase 13.5 (2026-06-08): Hebrew — second RTL script
    // (after Perso-Arabic). Block U+0590..U+05FF covers Hebrew
    // letters + vowel points (niqqud) + cantillation marks +
    // punctuation. Like the shipped Perso-Arabic dialects, the
    // lexer reads UTF-8 in logical (byte) order so no special
    // bidi handling is needed at the parse level — the RTL
    // direction is a rendering concern.
    Hebrew,
    // Phase 13.15 (2026-06-08): Thai — block U+0E00..U+0E7F.
    // LTR. Modern Thai prose lacks word-internal spaces but
    // vāṇी keywords rely on whitespace as separators, which
    // matches how Thai programmers write source files.
    Thai,
    // Phase 13.22 (2026-06-08): Armenian — block U+0530..058F.
    // Alphabetic with upper/lowercase forms; the keyword
    // table uses lowercase Mesropian letters.
    Armenian,
    // Phase 13.23 (2026-06-08): Georgian — main block
    // U+10A0..10FF (Asomtavruli + Mkhedruli) plus U+2D00..2D2F
    // (Nuskhuri). Modern Georgian uses Mkhedruli (lowercase-
    // only — no letter case in modern Georgian).
    Georgian,
    // Phase 13.29 (2026-06-08): Khmer — block U+1780..17FF.
    Khmer,
    // Phase 13.30 (2026-06-08): Myanmar (Burmese) — block
    // U+1000..109F.
    Burmese,
    // Phase 13.31 (2026-06-08): Ethiopic syllabary — block
    // U+1200..137F (main) + U+1380..139F (supplement) +
    // U+2D80..2DDF (extended) + U+AB00..AB2F (Ethiopic
    // Extended-A).
    Ethiopic,
    // Phase 13.32 (2026-06-08): Tibetan — block U+0F00..0FFF.
    Tibetan,
    // Phase 13.33 (2026-06-08): Cherokee syllabary — block
    // U+13A0..13FF (main) + U+AB70..ABBF (supplement).
    Cherokee,
    // Phase 13.34 (2026-06-08): Lao — block U+0E80..0EFF.
    Lao,
    // Phase 13.35 (2026-06-08): Mongolian — block U+1800..18AF.
    // Traditional vertical script; the lexer reads UTF-8 in
    // logical (byte) order so vertical rendering is a display
    // concern only.
    Mongolian,
}

impl Script {
    /// Classify a keyword token's first non-ASCII character.
    /// Returns `Latin` for pure-ASCII text and for text whose
    /// first non-ASCII codepoint sits in neither known Brahmi
    /// block (the future-script-forward-compat fallback).
    fn classify(text: &str) -> Script {
        for c in text.chars() {
            if ('\u{0900}'..='\u{097F}').contains(&c)
                || ('\u{A8E0}'..='\u{A8FF}').contains(&c)
            {
                return Script::Devanagari;
            }
            if ('\u{0980}'..='\u{09FF}').contains(&c) {
                return Script::Bengali;
            }
            // Phase 6 (2026-06-07): Brahmi-derived batch.
            if ('\u{0A00}'..='\u{0A7F}').contains(&c) {
                return Script::Gurmukhi;
            }
            if ('\u{0A80}'..='\u{0AFF}').contains(&c) {
                return Script::Gujarati;
            }
            if ('\u{0B00}'..='\u{0B7F}').contains(&c) {
                return Script::Odia;
            }
            if ('\u{0B80}'..='\u{0BFF}').contains(&c) {
                return Script::Tamil;
            }
            if ('\u{0C00}'..='\u{0C7F}').contains(&c) {
                return Script::Telugu;
            }
            if ('\u{0C80}'..='\u{0CFF}').contains(&c) {
                return Script::Kannada;
            }
            if ('\u{0D00}'..='\u{0D7F}').contains(&c) {
                return Script::Malayalam;
            }
            if ('\u{0D80}'..='\u{0DFF}').contains(&c) {
                return Script::Sinhala;
            }
            // Phase 12 (2026-06-07): Arabic block + supplements.
            if ('\u{0600}'..='\u{06FF}').contains(&c)
                || ('\u{0750}'..='\u{077F}').contains(&c)
                || ('\u{08A0}'..='\u{08FF}').contains(&c)
                || ('\u{FB50}'..='\u{FDFF}').contains(&c)
                || ('\u{FE70}'..='\u{FEFF}').contains(&c)
            {
                return Script::Arabic;
            }
            // Phase 8b.2 (2026-06-07): Cyrillic block + supplements.
            if ('\u{0400}'..='\u{04FF}').contains(&c)
                || ('\u{0500}'..='\u{052F}').contains(&c)
                || ('\u{2DE0}'..='\u{2DFF}').contains(&c)
                || ('\u{A640}'..='\u{A69F}').contains(&c)
            {
                return Script::Cyrillic;
            }
            // Phase 9b (2026-06-07): Japanese — three blocks
            // collapse to a single Script variant since native
            // Japanese code mixes Hiragana + Katakana + Kanji
            // within a single keyword (関数 / タスク / もし etc).
            if ('\u{3040}'..='\u{309F}').contains(&c)        // Hiragana
                || ('\u{30A0}'..='\u{30FF}').contains(&c)    // Katakana
                || ('\u{4E00}'..='\u{9FFF}').contains(&c)    // CJK Unified Ideographs
                || ('\u{3400}'..='\u{4DBF}').contains(&c)    // CJK Extension A
            {
                return Script::Japanese;
            }
            // Phase 13.1 (2026-06-07): Hangul — Korean syllables
            // (precomposed) + Jamo (decomposed) + Extended-A/B.
            if ('\u{AC00}'..='\u{D7AF}').contains(&c)        // Hangul Syllables
                || ('\u{1100}'..='\u{11FF}').contains(&c)    // Hangul Jamo
                || ('\u{A960}'..='\u{A97F}').contains(&c)    // Hangul Jamo Extended-A
                || ('\u{D7B0}'..='\u{D7FF}').contains(&c)    // Hangul Jamo Extended-B
            {
                return Script::Hangul;
            }
            // Phase 13.4 (2026-06-08): Greek and Coptic + Greek
            // Extended for polytonic-accented forms.
            if ('\u{0370}'..='\u{03FF}').contains(&c)        // Greek and Coptic
                || ('\u{1F00}'..='\u{1FFF}').contains(&c)    // Greek Extended
            {
                return Script::Greek;
            }
            // Phase 13.5 (2026-06-08): Hebrew (RTL like Arabic
            // but distinct Unicode block).
            if ('\u{0590}'..='\u{05FF}').contains(&c) {
                return Script::Hebrew;
            }
            // Phase 13.15 (2026-06-08): Thai script.
            if ('\u{0E00}'..='\u{0E7F}').contains(&c) {
                return Script::Thai;
            }
            // Phase 13.22 (2026-06-08): Armenian.
            if ('\u{0530}'..='\u{058F}').contains(&c) {
                return Script::Armenian;
            }
            // Phase 13.23 (2026-06-08): Georgian (Mkhedruli +
            // Asomtavruli main block + Nuskhuri supplement).
            if ('\u{10A0}'..='\u{10FF}').contains(&c)
                || ('\u{2D00}'..='\u{2D2F}').contains(&c)
            {
                return Script::Georgian;
            }
            // Phase 13.29: Khmer.
            if ('\u{1780}'..='\u{17FF}').contains(&c) {
                return Script::Khmer;
            }
            // Phase 13.30: Myanmar (Burmese).
            if ('\u{1000}'..='\u{109F}').contains(&c) {
                return Script::Burmese;
            }
            // Phase 13.31: Ethiopic (Amharic).
            if ('\u{1200}'..='\u{137F}').contains(&c)
                || ('\u{1380}'..='\u{139F}').contains(&c)
                || ('\u{2D80}'..='\u{2DDF}').contains(&c)
                || ('\u{AB00}'..='\u{AB2F}').contains(&c)
            {
                return Script::Ethiopic;
            }
            // Phase 13.32: Tibetan.
            if ('\u{0F00}'..='\u{0FFF}').contains(&c) {
                return Script::Tibetan;
            }
            // Phase 13.33: Cherokee syllabary.
            if ('\u{13A0}'..='\u{13FF}').contains(&c)
                || ('\u{AB70}'..='\u{ABBF}').contains(&c)
            {
                return Script::Cherokee;
            }
            // Phase 13.34: Lao.
            if ('\u{0E80}'..='\u{0EFF}').contains(&c) {
                return Script::Lao;
            }
            // Phase 13.35: Mongolian traditional.
            if ('\u{1800}'..='\u{18AF}').contains(&c) {
                return Script::Mongolian;
            }
        }
        Script::Latin
    }
}

/// Scan the first ~10 lines of source for a `// vani-lang: <name>`
/// pragma comment. Returns the declared dialect when found, or
/// None for back-compat (no pragma → script-level purity only).
/// Public helper used by the LSP (and any downstream tool) to
/// recover the raw `// vani-lang: <tag>` value without needing
/// access to the `DialectLang` enum. Returns the lowercase tag
/// (`"mandarin"`, `"sanskrit"`, etc.) or `None` when no pragma
/// is present in the first 10 comment lines.
pub fn detect_pragma_tag(source: &str) -> Option<String> {
    for (i, line) in source.lines().enumerate() {
        if i > 10 {
            break;
        }
        let trimmed = line.trim_start();
        if !trimmed.starts_with("//") {
            continue;
        }
        let body = trimmed.trim_start_matches("//").trim();
        let Some(rest) = body.strip_prefix("vani-lang:").or_else(|| {
            body.strip_prefix("vani-lang :")
        }) else {
            continue;
        };
        return Some(rest.trim().to_ascii_lowercase());
    }
    None
}

fn detect_language_pragma(source: &str) -> Option<DialectLang> {
    for (i, line) in source.lines().enumerate() {
        if i > 10 {
            break;
        }
        let trimmed = line.trim_start();
        if !trimmed.starts_with("//") {
            continue;
        }
        let body = trimmed.trim_start_matches("//").trim();
        // Accept `vani-lang: <lang>` (lowercase tag, optional
        // spaces). Reject malformed forms silently — the pragma
        // is opt-in and shouldn't break files that happen to
        // contain `vani-lang` in unrelated comments.
        let Some(rest) = body.strip_prefix("vani-lang:").or_else(|| {
            body.strip_prefix("vani-lang :")
        }) else {
            continue;
        };
        let name = rest.trim().to_ascii_lowercase();
        return match name.as_str() {
            "sanskrit" | "saṁskṛta" | "sa" => Some(DialectLang::Sanskrit),
            "hindi" | "hindī" | "hi" => Some(DialectLang::Hindi),
            "marathi" | "marāṭhī" | "mr" => Some(DialectLang::Marathi),
            "english" | "en" => Some(DialectLang::English),
            // Phase 2 (2026-06-07): Tier I dialect extensions.
            // Pragma tags + ISO 639 codes per dialect.
            "nepali" | "nepālī" | "ne" => Some(DialectLang::Nepali),
            "maithili" | "maithilī" | "mai" => Some(DialectLang::Maithili),
            "konkani" | "koṅkaṇī" | "kok" => Some(DialectLang::KonkaniDev),
            // Phase 5b (2026-06-07): Bengali (বাংলা).
            "bengali" | "bangla" | "bāṅlā" | "bn" => Some(DialectLang::Bengali),
            // Phase 6 (2026-06-07): Brahmi-derived batch.
            "tamil" | "tamiḻ" | "ta" => Some(DialectLang::Tamil),
            "telugu" | "telugū" | "te" => Some(DialectLang::Telugu),
            "gujarati" | "gujarātī" | "gu" => Some(DialectLang::Gujarati),
            "punjabi" | "pañjābī" | "pa" => Some(DialectLang::Punjabi),
            // Phase 6 second half (2026-06-07).
            "kannada" | "kannaḍa" | "kn" => Some(DialectLang::Kannada),
            "malayalam" | "malayāḷam" | "ml" => Some(DialectLang::Malayalam),
            "odia" | "oṛiā" | "oriya" | "or" => Some(DialectLang::Odia),
            "assamese" | "ɔxɔmia" | "as" => Some(DialectLang::Assamese),
            "sinhala" | "siṁhala" | "si" => Some(DialectLang::Sinhala),
            // Phase 12 (2026-06-07): Perso-Arabic dialect.
            "urdu" | "urdū" | "ur" => Some(DialectLang::Urdu),
            // Phase 12.2/12.3 (2026-06-07): more Perso-Arabic.
            "sindhi" | "sindhī" | "sd" => Some(DialectLang::Sindhi),
            "punjabi-shahmukhi" | "shahmukhi" | "pnb"
                => Some(DialectLang::PunjabiShahmukhi),
            // Phase 12.4 / 12.5 (2026-06-07).
            "persian" | "farsi" | "fārsī" | "fa"
                => Some(DialectLang::Persian),
            "pashto" | "paṣ́tō" | "ps"
                => Some(DialectLang::Pashto),
            // Phase 8b.2 (2026-06-07): first Cyrillic dialect.
            "russian" | "русский" | "ru" => Some(DialectLang::Russian),
            // Phase 8b.1 (2026-06-07): first Latin-script Tier II
            // dialect (with non-ASCII accent chars).
            "spanish" | "español" | "castellano" | "es"
                => Some(DialectLang::Spanish),
            // Phase 8b.3 (2026-06-07): second Latin-with-accents.
            "french" | "français" | "francais" | "fr"
                => Some(DialectLang::French),
            // Phase 9b (2026-06-07): first non-Indic SOV target.
            "japanese" | "日本語" | "nihongo" | "ja"
                => Some(DialectLang::Japanese),
            // Phase 10.1 (2026-06-07): third Latin-with-accents.
            "german" | "deutsch" | "de" => Some(DialectLang::German),
            // Phase 13.1 (2026-06-07): first Hangul-script dialect.
            "korean" | "한국어" | "hangugeo" | "ko"
                => Some(DialectLang::Korean),
            // Phase 13.2 (2026-06-08): fourth Latin-with-accents.
            // Accepts both European (português) and Brazilian
            // (brasileiro) spellings; same surface either way.
            "portuguese" | "português" | "portugues" | "pt"
                | "brasileiro" | "brasil"
                => Some(DialectLang::Portuguese),
            // Phase 13.3 (2026-06-08): first basic-Latin dialect
            // riding pure pragma-threading (no diacritics).
            "indonesian" | "indonesia" | "bahasa" | "id"
                => Some(DialectLang::Indonesian),
            // Phase 13.4 (2026-06-08): first Greek-script dialect.
            "greek" | "ελληνικά" | "ellinika" | "el"
                => Some(DialectLang::Greek),
            // Phase 13.5 (2026-06-08): second RTL dialect (after
            // Perso-Arabic batch). Hebrew uses its own Unicode
            // block U+0590..U+05FF.
            "hebrew" | "עברית" | "ivrit" | "he" | "iw"
                => Some(DialectLang::Hebrew),
            // Phase 13.6 (2026-06-08): fifth Latin-with-accents
            // (Romance family completes — Spanish + French +
            // Portuguese + Italian + German already shipped).
            "italian" | "italiano" | "it" => Some(DialectLang::Italian),
            // Phase 13.7 (2026-06-08): Modern Standard Arabic
            // distinct from the shipped Perso-Arabic dialects.
            "arabic" | "العربية" | "arabi" | "ar"
                => Some(DialectLang::Arabic),
            // Phase 13.8 (2026-06-08): sixth Latin-with-accents
            // — first Slavic Latin dialect.
            "polish" | "polski" | "pl" => Some(DialectLang::Polish),
            // Phase 13.9 (2026-06-08): seventh Latin-with-accents
            // — Turkic family.
            "turkish" | "türkçe" | "turkce" | "tr"
                => Some(DialectLang::Turkish),
            // Phase 13.10 (2026-06-08): second basic-Latin
            // (after Indonesian).
            "malay" | "melayu" | "bahasa-melayu" | "ms"
                => Some(DialectLang::Malay),
            // Phase 13.11 (2026-06-08): first African dialect.
            "swahili" | "kiswahili" | "sw"
                => Some(DialectLang::Swahili),
            // Phase 13.12 (2026-06-08): first Southeast Asian
            // Latin-script dialect.
            "vietnamese" | "tiếng-việt" | "tiengviet" | "vi"
                => Some(DialectLang::Vietnamese),
            // Phase 13.13 (2026-06-08): completes Romance family.
            "romanian" | "română" | "romana" | "ro"
                => Some(DialectLang::Romanian),
            // Phase 13.14 (2026-06-08): basic-Latin Germanic.
            "dutch" | "nederlands" | "nl"
                => Some(DialectLang::Dutch),
            // Phase 13.15 (2026-06-08): first Thai-script dialect.
            "thai" | "ไทย" | "th"
                => Some(DialectLang::Thai),
            // Phase 13.16 (2026-06-08): Uralic.
            "hungarian" | "magyar" | "hu"
                => Some(DialectLang::Hungarian),
            // Phase 13.17 (2026-06-08): Slavic Latin.
            "czech" | "čeština" | "cestina" | "cs"
                => Some(DialectLang::Czech),
            // Phase 13.18 (2026-06-08): Nordic.
            "swedish" | "svenska" | "sv"
                => Some(DialectLang::Swedish),
            // Phase 13.19 (2026-06-08): Austronesian basic Latin.
            "filipino" | "tagalog" | "fil" | "tl"
                => Some(DialectLang::Filipino),
            // Phase 13.20 (2026-06-08): second Nordic.
            "norwegian" | "norsk" | "bokmål" | "bokmal" | "no" | "nb"
                => Some(DialectLang::Norwegian),
            // Phase 13.21 (2026-06-08): third Nordic.
            "danish" | "dansk" | "da"
                => Some(DialectLang::Danish),
            // Phase 13.22 (2026-06-08): first Caucasus-region
            // script.
            "armenian" | "հայերեն" | "hayeren" | "hy"
                => Some(DialectLang::Armenian),
            // Phase 13.23 (2026-06-08): second Caucasus script.
            "georgian" | "ქართული" | "kartuli" | "ka"
                => Some(DialectLang::Georgian),
            // Phase 13.24 (2026-06-08): third Slavic.
            "slovak" | "slovenčina" | "slovencina" | "sk"
                => Some(DialectLang::Slovak),
            // Phase 13.25 (2026-06-08): second Uralic.
            "finnish" | "suomi" | "fi"
                => Some(DialectLang::Finnish),
            // Phase 13.26 (2026-06-08): sixth Romance.
            "catalan" | "català" | "catala" | "ca"
                => Some(DialectLang::Catalan),
            // Phase 13.27 (2026-06-08): Niger-Congo W. Africa.
            "yoruba" | "yorùbá" | "yo"
                => Some(DialectLang::Yoruba),
            // Phase 13.28 (2026-06-08): Afroasiatic W. Africa.
            "hausa" | "hawsa" | "ha"
                => Some(DialectLang::Hausa),
            // Phase 13.29..13.35 — seven new-Script dialects.
            "khmer" | "ខ្មែរ" | "km"
                => Some(DialectLang::Khmer),
            "burmese" | "myanmar" | "မြန်မာ" | "my"
                => Some(DialectLang::Burmese),
            "amharic" | "አማርኛ" | "am"
                => Some(DialectLang::Amharic),
            "tibetan" | "བོད་ཡིག" | "bo"
                => Some(DialectLang::Tibetan),
            "cherokee" | "ᏣᎳᎩ" | "chr"
                => Some(DialectLang::Cherokee),
            "lao" | "ລາວ" | "lo"
                => Some(DialectLang::Lao),
            "mongolian" | "ᠮᠣᠩᠭᠣᠯ" | "mn"
                => Some(DialectLang::Mongolian),
            // Phase 10.2 (2026-06-08): Mandarin Chinese — pragma
            // disambiguates between Japanese and Mandarin since
            // they share the CJK Unified Ideographs block.
            "mandarin" | "chinese" | "中文" | "汉语" | "漢語" | "zh"
                => Some(DialectLang::Mandarin),
            _ => None,
        };
    }
    None
}

/// SOV-S8 — language-tag table for Devanagari spellings. Each
/// spelling maps to the set of Indo-Aryan dialects that natively
/// support it. Source: per-line comments in `devanagari_keyword`
/// + `multi_word_devanagari_keyword` above. Tatsama (Sanskrit-
/// root loanwords used in Hindi/Marathi) tag all three dialects.
fn spelling_supports_dialect(spelling: &str, lang: DialectLang) -> bool {
    // Phase 2 (2026-06-07): Tier I dialect extensions. Nepali /
    // Maithili / Konkani-Devanagari are Indo-Aryan languages
    // that share heavy tatsama (Sanskrit-rooted) vocabulary
    // with the original three. v1 admits any spelling already
    // supported by Sanskrit, Hindi, or Marathi — this is the
    // permissive starting point. Native-vernacular spellings
    // (e.g. Nepali-specific verb forms) can be layered on as
    // user requests come in; tighten the gate then. Closure
    // path mirrors what Sanskrit already does: list spellings
    // explicitly only when the dialect set is a strict subset.
    if matches!(lang, DialectLang::Nepali | DialectLang::Maithili | DialectLang::KonkaniDev) {
        return spelling_supports_dialect(spelling, DialectLang::Sanskrit)
            || spelling_supports_dialect(spelling, DialectLang::Hindi)
            || spelling_supports_dialect(spelling, DialectLang::Marathi);
    }
    // Order matters: list each spelling once with all dialects
    // that support it. Fallthrough returns true for unknown
    // spellings (forward-compat: a future alias addition shouldn't
    // need to update this table simultaneously to avoid
    // false rejections; the script-level gate above catches the
    // real cross-language errors).
    use DialectLang::*;
    let langs: &[DialectLang] = match spelling {
        // === DECLARATIONS ===
        "फलन" => &[Hindi, Marathi],
        "कार्य" => &[Sanskrit, Hindi, Marathi],  // tatsama
        "मान" => &[Marathi],
        "माना" => &[Sanskrit, Hindi],
        "मानो" => &[Hindi],
        "संरचना" => &[Sanskrit, Hindi, Marathi],
        "विकल्प" => &[Sanskrit],
        "गणन" => &[Hindi, Marathi],
        "स्थिर" => &[Sanskrit, Hindi, Marathi],  // tatsama
        "नियत" => &[Hindi, Marathi],
        // === VISIBILITY / MODULES ===
        "सार्वजनिक" => &[Sanskrit, Hindi, Marathi],  // tatsama
        "खण्ड" => &[Sanskrit, Hindi, Marathi],  // tatsama
        "मॉड्यूल" => &[Hindi, Marathi],
        "उपयोग" => &[Sanskrit, Hindi, Marathi],  // tatsama
        "यथा" => &[Sanskrit, Hindi, Marathi],  // tatsama
        // === CONTROL FLOW ===
        "परत" => &[Marathi],
        "लौटाओ" => &[Hindi],
        "पुनरागम" => &[Sanskrit],
        "यदि" => &[Sanskrit, Hindi],
        "अगर" => &[Hindi],
        "जर" => &[Marathi],
        "अन्यथा" => &[Sanskrit],
        "वरना" => &[Hindi],
        "नाहीतर" => &[Marathi],
        "यावत्" => &[Sanskrit],
        "जबतक" => &[Hindi],
        "जोपर्यंत" => &[Marathi],
        "प्रति" => &[Sanskrit],
        "साठी" => &[Marathi],
        // Indo-Aryan postpositions widely understood across all
        // three dialects in modern usage (including modern
        // Sanskrit pedagogy). Tagged as shared so a Sanskrit-
        // pragma file can still use the natural `0 से 3 तक`
        // range syntax without dropping into the pragma-free
        // back-compat mode.
        "में" => &[Sanskrit, Hindi, Marathi],
        "से" => &[Sanskrit, Hindi, Marathi],
        "तक" => &[Sanskrit, Hindi, Marathi],
        "तदा" => &[Sanskrit],
        "तो" => &[Hindi],
        "तर" => &[Marathi],
        "विराम" => &[Sanskrit, Hindi, Marathi],  // tatsama
        "रुको" => &[Hindi],
        "थांब" => &[Marathi],
        "अग्रे" => &[Sanskrit],
        "पुढे" => &[Marathi],
        "आगे" => &[Hindi],
        // === REFS / MUT ===
        "पहा" => &[Marathi],
        "देखो" => &[Hindi],
        "दृष्ट्या" => &[Sanskrit],
        "बदल" => &[Marathi],
        "बदलणारा" => &[Marathi],  // Marathi natural mutable adjective
        "परिवर्तनीय" => &[Sanskrit, Hindi, Marathi],  // tatsama, formal in all three
        // === MATCH ===
        "जुळवा" => &[Marathi],
        "मिलान" => &[Hindi],
        "मेल" => &[Sanskrit],
        "मेलन" => &[Sanskrit],  // classical Sanskrit deverbal
        // === VERIFICATION ===
        "खात्री" => &[Marathi],
        "सुनिश्चित" => &[Hindi],
        "सिद्धम्" => &[Sanskrit],
        "सिद्ध" => &[Sanskrit, Hindi, Marathi],  // tatsama root
        "प्रमाण" => &[Sanskrit, Hindi, Marathi],  // tatsama
        "प्रमाणित" => &[Hindi, Marathi],
        "दर्शाओ" => &[Hindi],
        "दाखवा" => &[Marathi],
        "अपेक्षित" => &[Sanskrit],
        "चाहिए" => &[Hindi],
        "पाहिजे" => &[Marathi],
        "निश्चित" => &[Hindi, Marathi],
        "सुनिश्चयित" => &[Sanskrit],
        // === BOOL / PRINT ===
        "सत्य" => &[Sanskrit, Hindi, Marathi],  // tatsama
        // सही means "signature" in Marathi (noun) — Hindi-only.
        // Native-speaker audit 2026-06-07.
        "सही" => &[Hindi],
        "सच" => &[Hindi],          // Hindi natural truth
        "बरोबर" => &[Marathi],    // Marathi natural true
        "खरे" => &[Marathi],      // Marathi natural true
        "असत्य" => &[Sanskrit, Hindi, Marathi],  // tatsama
        // अशुद्ध strictly means "impure" in Marathi — Hindi-only
        // for "false". Native-speaker audit 2026-06-07.
        "अशुद्ध" => &[Hindi],
        "झूठ" => &[Hindi],         // Hindi natural false
        "गलत" => &[Hindi],         // Hindi natural wrong/false
        "खोटे" => &[Marathi],     // Marathi natural false
        "चूक" => &[Marathi],      // Marathi natural false/mistake
        "लिख" => &[Sanskrit],
        "लिखो" => &[Hindi],  // Marathi uses लिह- root forms below
        "लिहा" => &[Marathi],
        "लिही" => &[Marathi],
        "लिहिया" => &[Marathi],
        // === PURITY / PARALLELISM ===
        "शुद्ध" => &[Sanskrit, Hindi, Marathi],  // tatsama
        "समानांतर" => &[Sanskrit, Hindi, Marathi],  // tatsama
        "संक्षेप" => &[Sanskrit, Hindi, Marathi],  // tatsama
        "सह" => &[Sanskrit, Hindi, Marathi],  // tatsama
        // === INTERFACES / METHODS ===
        "संकेत" => &[Sanskrit, Hindi, Marathi],  // tatsama
        "अंतरापृष्ठ" => &[Sanskrit],
        "कार्यान्वित" => &[Sanskrit, Hindi, Marathi],  // tatsama
        "विधि" => &[Sanskrit, Hindi, Marathi],  // tatsama
        // === BOUNDS ===
        "जहाँ" => &[Hindi],
        "यत्र" => &[Sanskrit],
        "जिथे" => &[Marathi],
        "है" => &[Hindi],
        "अस्ति" => &[Sanskrit],
        "आहे" => &[Marathi],
        // === CONCURRENCY ===
        "प्रयास" => &[Sanskrit, Hindi, Marathi],  // tatsama
        "नियोग" => &[Sanskrit, Hindi, Marathi],  // tatsama
        "संयोजन" => &[Sanskrit, Hindi, Marathi],  // tatsama
        // === EMBEDDED ===
        "असुरक्षित" => &[Sanskrit, Hindi, Marathi],  // tatsama
        "क्षेत्र" => &[Sanskrit, Hindi, Marathi],  // tatsama
        // === SOV-S7 ADDS (all Sanskrit-root tatsama) ===
        "उद्देश्य" => &[Sanskrit, Hindi, Marathi],
        "प्रकार" => &[Sanskrit, Hindi, Marathi],
        "बाह्य" => &[Sanskrit, Hindi, Marathi],
        "अपरिवर्तनीय" => &[Sanskrit, Hindi, Marathi],
        // === MULTI-WORD ALIASES ===
        // (the lexer fuses these post-tokenization; the span text
        // is the full multi-word phrase)
        "नहीं तो" => &[Hindi],
        "के लिए" => &[Hindi],
        "सिद्ध करो" => &[Hindi],
        "सिद्ध करा" => &[Marathi],
        "समान्तर प्रति" => &[Sanskrit],
        // Unknown spelling — be permissive. The script-level
        // gate above already catches structural mistakes.
        _ => return true,
    };
    langs.contains(&lang)
}

/// Returns true when the token is a *structure* keyword — the
/// kind that should be subject to the language-purity gate.
/// Type names, literals, identifiers, operators, and the
/// boolean literals stay neutral so they can appear in any
/// language file. Add new structure keywords here when extending
/// the lexer.
fn is_structure_keyword_kind(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Fn
            | TokenKind::Pure
            | TokenKind::Extern
            | TokenKind::Parallel
            | TokenKind::Reduce
            | TokenKind::With
            | TokenKind::Task
            | TokenKind::Join
            | TokenKind::Let
            | TokenKind::Return
            | TokenKind::If
            | TokenKind::Else
            | TokenKind::While
            | TokenKind::Break
            | TokenKind::Continue
            | TokenKind::Mut
            | TokenKind::For
            | TokenKind::In
            | TokenKind::Ref
            | TokenKind::From
            | TokenKind::To
            | TokenKind::Struct
            | TokenKind::Enum
            | TokenKind::Match
            | TokenKind::Then
            | TokenKind::Interface
            | TokenKind::Implement
            | TokenKind::Where
            | TokenKind::Is
            | TokenKind::Const
            | TokenKind::Type
            | TokenKind::Methods
            | TokenKind::Intent
            | TokenKind::Use
            | TokenKind::Requires
            | TokenKind::Ensures
            | TokenKind::Invariant
            | TokenKind::Assert
            | TokenKind::Prove
            | TokenKind::Print
            | TokenKind::Try
            | TokenKind::Module
            | TokenKind::Pub
            | TokenKind::Unsafe
            | TokenKind::RegionKw
    )
}

/// Post-lex pass that merges adjacent token pairs whose combined
/// text matches a multi-word Devanagari keyword alias. Examples:
/// Hindi `नहीं तो` (`nahīṁ to`, "else"), `के लिए` (`ke liye`,
/// "for"), `सिद्ध करो` (`siddha karo`, "prove"). The lexer's main
/// pass only sees whitespace-separated words, so multi-word
/// aliases need this stitching after the fact.
///
/// Reads the original source text via each token's span so it can
/// inspect words that were already resolved to single-word aliases
/// (e.g. `तो` lexed as `Then`). The multi-word form takes
/// precedence when both words are present and the combined string
/// matches a multi-word alias.
fn merge_multi_word_devanagari_aliases(tokens: &mut Vec<Token>, source: &str) {
    let mut i = 0;
    while i + 1 < tokens.len() {
        let a_span = tokens[i].span;
        let b_span = tokens[i + 1].span;
        // Skip merging across token gaps that contain more than
        // whitespace (the merger pattern is `WORD WORD` with only
        // ASCII spaces / tabs in between).
        if !whitespace_only(source, a_span.end, b_span.start) {
            i += 1;
            continue;
        }
        let a_text = source.get(a_span.start..a_span.end);
        let b_text = source.get(b_span.start..b_span.end);
        if let (Some(a), Some(b)) = (a_text, b_text) {
            // Both word slices must contain non-ASCII bytes (i.e.
            // they're Devanagari, not English keywords). Avoids
            // accidentally merging `let x` or similar.
            if a.bytes().any(|byte| byte >= 0x80)
                && b.bytes().any(|byte| byte >= 0x80)
            {
                let combined = format!("{} {}", a, b);
                if let Some(kind) = multi_word_devanagari_keyword(&combined) {
                    let merged_span = a_span.merge(b_span);
                    tokens[i] = Token { kind, span: merged_span };
                    tokens.remove(i + 1);
                    continue;
                }
            }
        }
        i += 1;
    }
}

/// Closure #255: fold the two-word ASCII phrase `give back`
/// into a single `Return` token. The lexer's main pass already
/// maps the standalone words `give` and `give_back` to
/// `TokenKind::Return`; this pass picks up the writer who
/// preferred two whitespace-separated words. We don't reuse
/// the Devanagari merger because it intentionally rejects
/// ASCII pairs to avoid accidentally merging unrelated
/// identifiers (e.g. `let x` would have collided with a
/// hypothetical `let x` alias). The pattern here is
/// specific: a `Return` token whose source text is exactly
/// `give`, followed by an `Ident` whose source text is
/// exactly `back`, with only whitespace between them. Real
/// `return back;` style code is unaffected because `return`
/// (the canonical form) doesn't trigger.
fn merge_give_back_ascii_alias(tokens: &mut Vec<Token>, source: &str) {
    let mut i = 0;
    while i + 1 < tokens.len() {
        if !matches!(tokens[i].kind, TokenKind::Return) {
            i += 1;
            continue;
        }
        if !matches!(tokens[i + 1].kind, TokenKind::Ident(_)) {
            i += 1;
            continue;
        }
        let a_span = tokens[i].span;
        let b_span = tokens[i + 1].span;
        if !whitespace_only(source, a_span.end, b_span.start) {
            i += 1;
            continue;
        }
        let a_text = source.get(a_span.start..a_span.end);
        let b_text = source.get(b_span.start..b_span.end);
        if matches!(a_text, Some("give")) && matches!(b_text, Some("back")) {
            // Extend the Return token's span to cover both
            // words so diagnostics underline the full phrase,
            // then drop the trailing `back`.
            let merged_span = a_span.merge(b_span);
            tokens[i] = Token {
                kind: TokenKind::Return,
                span: merged_span,
            };
            tokens.remove(i + 1);
            // Don't advance: the new token at `i` might be
            // followed by another mergeable pair (unlikely
            // but cheap to allow).
            continue;
        }
        i += 1;
    }
}

/// True iff `source[start..end]` contains only ASCII whitespace.
fn whitespace_only(source: &str, start: usize, end: usize) -> bool {
    source.get(start..end)
        .map(|s| s.bytes().all(|b| b == b' ' || b == b'\t'))
        .unwrap_or(false)
}

/// Resolve a multi-word Devanagari phrase to its English-equivalent
/// `TokenKind`. The merger only consults this when both words were
/// lexed as Devanagari Idents (i.e., neither was a single-word
/// alias on its own). For v1, this is the safe overlap because
/// none of these phrases share their first word with a single-word
/// alias.
fn multi_word_devanagari_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "नहीं तो" => TokenKind::Else,       // nahīṁ to (Hindi: "if not / else")
        "के लिए" => TokenKind::For,         // ke liye (Hindi: "for the sake of")
        "सिद्ध करो" => TokenKind::Prove,    // siddha karo (Hindi: "prove!")
        "सिद्ध करा" => TokenKind::Prove,    // siddha karā (Marathi: "prove!")
        "समान्तर प्रति" => TokenKind::Parallel, // samāntara prati (Sanskrit)
        _ => return None,
    };
    Some(kind)
}

/// A `// …` comment recovered from source for later use by tools
/// (currently the formatter). The lexer's main pass drops comments
/// to keep the token stream lean for parsing; this side-channel scan
/// recovers them with their byte spans so a downstream formatter can
/// re-interleave them at the right indent.
#[derive(Clone, Debug, PartialEq)]
pub struct Comment {
    /// The full text of the line including the leading `//`. Trailing
    /// whitespace before the newline is preserved verbatim so that a
    /// careful tool could reproduce the original exactly; the
    /// formatter trims it.
    pub text: String,
    pub span: Span,
}

/// Scan `source` for `//` line comments, returning them in document
/// order. String literals are skipped correctly so `"//"` inside a
/// string is not mistaken for a comment. This is a deliberately
/// separate pass from `lex`: keeping comments off the main token
/// stream avoids polluting every parser site with comment-skipping
/// logic.
pub fn extract_comments(source: &str) -> Vec<Comment> {
    let bytes = source.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => {
                // Skip a string literal. Honors `\X` two-byte escapes
                // so that `"\""` isn't terminated by the inner quote.
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == b'\\' && i + 1 < bytes.len() {
                        i += 2;
                        continue;
                    }
                    if bytes[i] == b'"' {
                        i += 1;
                        break;
                    }
                    if bytes[i] == b'\n' {
                        // The real lexer will surface this. Bail out
                        // so we don't claim everything after as
                        // string content.
                        break;
                    }
                    i += 1;
                }
            }
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'/' => {
                let start = i;
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                let text = std::str::from_utf8(&bytes[start..i])
                    .unwrap_or("")
                    .to_string();
                out.push(Comment {
                    text,
                    span: Span::new(start, i),
                });
            }
            _ => i += 1,
        }
    }
    out
}

struct Lexer<'a> {
    source: &'a str,
    bytes: &'a [u8],
    pos: usize,
    tokens: Vec<Token>,
    /// Phase: pragma threading (2026-06-08). The pragma is
    /// scanned once up-front by `detect_language_pragma` and
    /// cached here so the per-dialect ASCII-keyword lookups
    /// (Spanish `si`/`para`/`verdadero`, French `fonction`/`pour`,
    /// German `wenn`/`wahr`) can fire only inside a file that
    /// declares the matching dialect. Without this gate, those
    /// words would collide with potential user identifiers in
    /// English files. None means no pragma (or an unrecognized
    /// one) — the lexer falls back to the English-only ASCII
    /// keyword table just like before.
    pragma: Option<DialectLang>,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            bytes: source.as_bytes(),
            pos: 0,
            tokens: Vec::new(),
            pragma: detect_language_pragma(source),
        }
    }

    fn lex(mut self) -> Result<Vec<Token>, Diagnostic> {
        while !self.is_at_end() {
            let start = self.pos;
            let byte = self.advance();

            match byte {
                b' ' | b'\r' | b'\t' | b'\n' => {}
                b'/' if self.match_byte(b'/') => self.skip_line_comment(),
                b'0'..=b'9' => self.lex_number(start)?,
                b'a'..=b'z' | b'A'..=b'Z' | b'_' => self.lex_ident(start),
                // Non-ASCII byte: start of a UTF-8 multi-byte
                // codepoint sequence. Devanagari letters (U+0900
                // – U+097F) and numerals (U+0966 – U+096F) live
                // here. Numerals start `E0 A5 A6..A5 AF` in UTF-8;
                // dispatch them to `lex_devanagari_number`, others
                // to `lex_unicode_ident`. Item #9 — Sanskrit /
                // Hindi / Marathi.
                b if b >= 0x80 => {
                    if b == 0xE0
                        && self.peek() == Some(0xA5)
                        && matches!(self.peek_next(), Some(0xA6..=0xAF))
                    {
                        self.lex_devanagari_number(start)?;
                    } else {
                        self.lex_unicode_ident(start);
                    }
                }
                b'"' => self.lex_string(start)?,
                b'(' => self.push(TokenKind::LParen, start),
                b')' => self.push(TokenKind::RParen, start),
                b'{' => self.push(TokenKind::LBrace, start),
                b'}' => self.push(TokenKind::RBrace, start),
                b'[' => self.push(TokenKind::LBracket, start),
                b']' => self.push(TokenKind::RBracket, start),
                b':' if self.match_byte(b':') => self.push(TokenKind::ColonColon, start),
                b':' => self.push(TokenKind::Colon, start),
                b';' => self.push(TokenKind::Semicolon, start),
                b',' => self.push(TokenKind::Comma, start),
                b'+' => self.push(TokenKind::Plus, start),
                b'-' if self.match_byte(b'>') => self.push(TokenKind::Arrow, start),
                b'-' => self.push(TokenKind::Minus, start),
                b'*' => self.push(TokenKind::Star, start),
                b'/' => self.push(TokenKind::Slash, start),
                b'%' => self.push(TokenKind::Percent, start),
                b'!' if self.match_byte(b'=') => self.push(TokenKind::BangEq, start),
                b'!' => self.push(TokenKind::Bang, start),
                b'=' if self.match_byte(b'=') => self.push(TokenKind::EqEq, start),
                b'=' => self.push(TokenKind::Equal, start),
                b'<' if self.match_byte(b'<') => self.push(TokenKind::LessLess, start),
                b'<' if self.match_byte(b'=') => self.push(TokenKind::LessEq, start),
                b'<' => self.push(TokenKind::Less, start),
                b'>' if self.match_byte(b'>') => self.push(TokenKind::GreaterGreater, start),
                b'>' if self.match_byte(b'=') => self.push(TokenKind::GreaterEq, start),
                b'>' => self.push(TokenKind::Greater, start),
                b'&' if self.match_byte(b'&') => self.push(TokenKind::AndAnd, start),
                b'&' => self.push(TokenKind::Amp, start),
                b'|' if self.match_byte(b'|') => self.push(TokenKind::OrOr, start),
                b'|' => self.push(TokenKind::Pipe, start),
                b'^' => self.push(TokenKind::Caret, start),
                b'.' if self.match_byte(b'.') => self.push(TokenKind::DotDot, start),
                b'.' => self.push(TokenKind::Dot, start),
                // Closure #286: `#` for attribute syntax,
                // e.g. `#[bounded(N)]`. v1 only recognizes
                // the literal `bounded` attribute; future
                // attributes (`inline`, `deprecated`, etc.)
                // ride the same token.
                b'#' => self.push(TokenKind::Hash, start),
                b'?' => self.push(TokenKind::Question, start),
                other => {
                    return Err(Diagnostic::new(
                        Span::new(start, start + 1),
                        format!("unexpected character '{}'", other as char),
                    ));
                }
            }
        }

        self.tokens.push(Token {
            kind: TokenKind::Eof,
            span: Span::new(self.source.len(), self.source.len()),
        });
        Ok(self.tokens)
    }

    /// Lex a Devanagari integer literal — sequence of digits from
    /// `०१२३४५६७८९` (U+0966 – U+096F). The first lead byte `0xE0`
    /// has already been consumed; the next two bytes are read as
    /// part of the first digit, then any subsequent Devanagari
    /// digits are consumed too. The resulting digit string is
    /// translated to ASCII and parsed via `i128::from_str_radix`.
    /// No suffix / float / radix / underscore support in this
    /// first cut — Devanagari literals are integers only, for
    /// readability of small numbers in source. Item #9 follow-up.
    fn lex_devanagari_number(&mut self, start: usize) -> Result<(), Diagnostic> {
        // Consume the remaining two bytes of the first codepoint
        // (`0xA5` then `0xA6..=0xAF` — already pre-checked at the
        // dispatch site).
        self.advance(); // 0xA5
        self.advance(); // digit byte 0xA6..AF
        // Consume any further Devanagari digits.
        while self.peek() == Some(0xE0)
            && self.peek_next() == Some(0xA5)
            && matches!(
                self.bytes.get(self.pos + 2).copied(),
                Some(0xA6..=0xAF)
            )
        {
            self.advance(); // 0xE0
            self.advance(); // 0xA5
            self.advance(); // digit
        }
        // Devanagari float support (2026-06-06): a `.` immediately
        // after the integer part followed by another Devanagari
        // digit makes this a float literal. Matches the ASCII path
        // shape (`peek == '.' && peek_next ∈ digit`).
        let mut is_float = false;
        let dev_digit_starts = |b0: Option<u8>, b1: Option<u8>, b2: Option<u8>| -> bool {
            b0 == Some(0xE0) && b1 == Some(0xA5) && matches!(b2, Some(0xA6..=0xAF))
        };
        if self.peek() == Some(b'.')
            && dev_digit_starts(
                self.bytes.get(self.pos + 1).copied(),
                self.bytes.get(self.pos + 2).copied(),
                self.bytes.get(self.pos + 3).copied(),
            )
        {
            is_float = true;
            self.advance(); // '.'
            while self.peek() == Some(0xE0)
                && self.peek_next() == Some(0xA5)
                && matches!(
                    self.bytes.get(self.pos + 2).copied(),
                    Some(0xA6..=0xAF)
                )
            {
                self.advance();
                self.advance();
                self.advance();
            }
        }
        let span = Span::new(start, self.pos);
        let raw = &self.source[start..self.pos];
        let mut ascii_digits = String::with_capacity(raw.chars().count());
        for ch in raw.chars() {
            if ch == '.' {
                ascii_digits.push('.');
            } else {
                // Devanagari digit codepoints U+0966..U+096F map
                // to ASCII '0'..'9' by subtracting 0x0966.
                let code = ch as u32;
                ascii_digits.push((b'0' + (code - 0x0966) as u8) as char);
            }
        }
        if is_float {
            let value = ascii_digits.parse::<f64>().map_err(|_| {
                Diagnostic::new(span, format!("invalid Devanagari float '{}'", raw))
            })?;
            self.tokens.push(Token {
                kind: TokenKind::Float(value),
                span,
            });
        } else {
            let value: i128 = ascii_digits.parse().map_err(|_| {
                Diagnostic::new(span, format!("invalid Devanagari integer '{}'", raw))
            })?;
            self.tokens.push(Token {
                kind: TokenKind::Int(value),
                span,
            });
        }
        Ok(())
    }

    fn lex_number(&mut self, start: usize) -> Result<(), Diagnostic> {
        let first = self.bytes[start];
        if first == b'0' && matches!(self.peek(), Some(b'x' | b'X' | b'b' | b'B' | b'o' | b'O')) {
            return self.lex_radix_int(start);
        }

        while matches!(self.peek(), Some(b'0'..=b'9' | b'_')) {
            self.advance();
        }

        let mut is_float = false;

        if self.peek() == Some(b'.') && matches!(self.peek_next(), Some(b'0'..=b'9')) {
            is_float = true;
            self.advance();
            while matches!(self.peek(), Some(b'0'..=b'9' | b'_')) {
                self.advance();
            }
        }

        if matches!(self.peek(), Some(b'e' | b'E')) {
            let exponent_start = self.pos;
            self.advance();
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.advance();
            }
            if matches!(self.peek(), Some(b'0'..=b'9')) {
                is_float = true;
                while matches!(self.peek(), Some(b'0'..=b'9' | b'_')) {
                    self.advance();
                }
            } else {
                return Err(Diagnostic::new(
                    Span::new(exponent_start, self.pos),
                    "expected digits after float exponent",
                ));
            }
        }

        let span = Span::new(start, self.pos);
        let raw = &self.source[start..self.pos];
        let cleaned = strip_underscores(raw);

        if is_float {
            let value = cleaned.parse::<f64>().map_err(|_| {
                Diagnostic::new(span, format!("float literal '{}' cannot be parsed", raw))
            })?;
            if !value.is_finite() {
                return Err(Diagnostic::new(
                    span,
                    format!("float literal '{}' is not finite", raw),
                ));
            }
            self.tokens.push(Token {
                kind: TokenKind::Float(value),
                span,
            });
            return Ok(());
        }

        let value = cleaned.parse::<i128>().map_err(|_| {
            Diagnostic::new(
                span,
                format!("integer literal '{}' does not fit in i128", raw),
            )
        })?;

        self.tokens.push(Token {
            kind: TokenKind::Int(value),
            span,
        });
        Ok(())
    }

    fn lex_radix_int(&mut self, start: usize) -> Result<(), Diagnostic> {
        let prefix = self.advance();
        let (radix, name): (u32, &str) = match prefix {
            b'x' | b'X' => (16, "hex"),
            b'b' | b'B' => (2, "binary"),
            b'o' | b'O' => (8, "octal"),
            _ => unreachable!("called only on valid radix prefixes"),
        };

        let digits_start = self.pos;
        while let Some(byte) = self.peek() {
            if byte == b'_' || is_digit_for_radix(byte, radix) {
                self.advance();
            } else {
                break;
            }
        }

        if self.pos == digits_start {
            return Err(Diagnostic::new(
                Span::new(start, self.pos),
                format!("expected {} digits after '0{}' prefix", name, prefix as char),
            ));
        }

        let span = Span::new(start, self.pos);
        let cleaned = strip_underscores(&self.source[digits_start..self.pos]);
        let value = i128::from_str_radix(&cleaned, radix).map_err(|_| {
            Diagnostic::new(
                span,
                format!(
                    "{} integer literal '{}' does not fit in i128",
                    name,
                    &self.source[start..self.pos]
                ),
            )
        })?;

        self.tokens.push(Token {
            kind: TokenKind::Int(value),
            span,
        });
        Ok(())
    }

    /// Lex an identifier that begins with a non-ASCII codepoint
    /// (e.g. Devanagari letters). Consumes every following byte
    /// that's either an identifier-continuation ASCII character
    /// or any non-ASCII byte (which by validated-UTF-8 source
    /// invariant means it's part of another codepoint). Then
    /// matches the resulting string against the Devanagari
    /// keyword-alias table — if a hit, route to the corresponding
    /// English TokenKind. Otherwise treat as a Unicode identifier
    /// name (`Ident`).
    fn lex_unicode_ident(&mut self, start: usize) {
        while let Some(b) = self.peek() {
            if matches!(b, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_')
                || b >= 0x80
            {
                self.advance();
            } else {
                break;
            }
        }
        let text = &self.source[start..self.pos];
        // Phase 5b + 6 (2026-06-07): every Brahmi script lives
        // in its own disjoint Unicode block so a given spelling
        // can match at most one table — order is purely a hot-
        // path heuristic (Devanagari shipped earliest, has the
        // most aliases). Each `.or_else` adds <50ns when the
        // chosen language is later in the chain.
        let kind = devanagari_keyword(text)
            .or_else(|| bengali_keyword(text))
            .or_else(|| tamil_keyword(text))
            .or_else(|| telugu_keyword(text))
            .or_else(|| gujarati_keyword(text))
            .or_else(|| punjabi_keyword(text))
            .or_else(|| kannada_keyword(text))
            .or_else(|| malayalam_keyword(text))
            .or_else(|| odia_keyword(text))
            .or_else(|| sinhala_keyword(text))
            .or_else(|| urdu_keyword(text))
            .or_else(|| persian_keyword(text))
            .or_else(|| pashto_keyword(text))
            .or_else(|| cyrillic_keyword(text))
            // Phase 8b.1/8b.3 (2026-06-07): Latin-with-accents
            // dialects (Spanish, French) — their non-ASCII
            // keywords starting with an accented letter (e.g.
            // French `écris`, `étranger`, `énumération`) route
            // through this entry point instead of `lex_ident`,
            // so the lookup chain has to live in both places.
            .or_else(|| spanish_keyword(text))
            .or_else(|| french_keyword(text))
            // Phase 9b (2026-06-07): Japanese — all Japanese
            // keywords start non-ASCII (Hiragana / Katakana /
            // Kanji code points all sit above U+0080).
            .or_else(|| japanese_keyword(text))
            // Phase 10.2 (2026-06-08): Mandarin — pure-Han keywords
            // on the shared CJK code-point block. Routes AFTER
            // Japanese so Japanese's mixed-Kanji+Hiragana forms
            // (関数 / もし) match their TokenKind first; pure-Han
            // Mandarin forms (函数 / 如果) fall through to here.
            .or_else(|| mandarin_keyword(text))
            // Phase 13.1 (2026-06-07): Korean — Hangul syllables
            // all sit at U+AC00+ (above U+0080).
            .or_else(|| korean_keyword(text))
            // Phase 13.2 (2026-06-08): Portuguese — non-ASCII
            // keywords (até, senão, então, mutável, função, etc.)
            // route through this entry point when they start
            // with a non-ASCII byte (não, …).
            .or_else(|| portuguese_keyword(text))
            // Phase 13.4 (2026-06-08): Greek — every keyword
            // starts with a Greek-block codepoint (U+0370+).
            .or_else(|| greek_keyword(text))
            // Phase 13.5 (2026-06-08): Hebrew — every keyword
            // starts with a Hebrew-block codepoint (U+0590+).
            .or_else(|| hebrew_keyword(text))
            // Phase 13.7 (2026-06-08): Modern Standard Arabic
            // — distinct from the Indo-Iranian Perso-Arabic
            // dialects; native Arabic vocabulary on the same
            // Script::Arabic infrastructure.
            .or_else(|| arabic_keyword(text))
            // Phase 13.8/13.9 (2026-06-08): Polish + Turkish
            // non-ASCII forms via the lex_unicode_ident entry
            // when keywords start with a diacritic letter.
            .or_else(|| polish_keyword(text))
            .or_else(|| turkish_keyword(text))
            // Phase 13.12 (2026-06-08): Vietnamese — extensive
            // diacritic + tone-mark combinations.
            .or_else(|| vietnamese_keyword(text))
            // Phase 13.13 (2026-06-08): Romanian — ă/â/î/ș/ț.
            .or_else(|| romanian_keyword(text))
            // Phase 13.15 (2026-06-08): Thai script keywords.
            .or_else(|| thai_keyword(text))
            // Phase 13.16/13.17/13.18: Hungarian + Czech + Swedish
            // non-ASCII forms (when starting with a diacritic).
            .or_else(|| hungarian_keyword(text))
            .or_else(|| czech_keyword(text))
            .or_else(|| swedish_keyword(text))
            // Phase 13.20/13.21: Norwegian + Danish non-ASCII.
            .or_else(|| norwegian_keyword(text))
            .or_else(|| danish_keyword(text))
            // Phase 13.22/13.23: Armenian + Georgian scripts.
            .or_else(|| armenian_keyword(text))
            .or_else(|| georgian_keyword(text))
            // Phase 13.24/25/26/27/28: more Latin variants.
            .or_else(|| slovak_keyword(text))
            .or_else(|| finnish_keyword(text))
            .or_else(|| catalan_keyword(text))
            .or_else(|| yoruba_keyword(text))
            .or_else(|| hausa_keyword(text))
            // Phase 13.29..13.35: seven new-Script dialects.
            .or_else(|| khmer_keyword(text))
            .or_else(|| burmese_keyword(text))
            .or_else(|| amharic_keyword(text))
            .or_else(|| tibetan_keyword(text))
            .or_else(|| cherokee_keyword(text))
            .or_else(|| lao_keyword(text))
            .or_else(|| mongolian_keyword(text))
            // Phase 10.1 (2026-06-07): German Latin-with-accents
            // — keywords starting with non-ASCII (`äußere`,
            // `öffentlich`, `überprüfen`) route through this
            // unicode entry point.
            .or_else(|| german_keyword(text))
            .unwrap_or_else(|| TokenKind::Ident(text.to_owned()));
        self.tokens.push(Token {
            kind,
            span: Span::new(start, self.pos),
        });
    }

    fn lex_ident(&mut self, start: usize) {
        // Phase 8b.1 (2026-06-07): also consume non-ASCII bytes
        // as identifier continuation so Latin-script-with-accent
        // keywords (Spanish `función`, `módulo`, `público`;
        // French `très`; German `ä/ö/ü`) tokenize as one word.
        // The byte-loop is the same as `lex_unicode_ident`; the
        // entry point differs only in which dispatch arm the
        // main lex loop chose.
        while let Some(b) = self.peek() {
            if matches!(b, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_')
                || b >= 0x80
            {
                self.advance();
            } else {
                break;
            }
        }

        let text = &self.source[start..self.pos];
        // English keyword table — primary spelling on the
        // left, alias rows below it. Each alias maps to the
        // same TokenKind so the parser doesn't need to know
        // about the alternate spelling. Alias selection is
        // conservative: only word forms that are very
        // unlikely to collide with user-chosen identifiers
        // (variable / param / field names). Common
        // identifier-shaped words like `def`, `function`,
        // `bind`, `mutable`, `constant`, `otherwise` are
        // deliberately NOT added — they'd silently break
        // existing user code that uses them as names. Once
        // per-file language purity (TODO item) ships, that
        // gate can declare safe-vs-collision contexts and
        // unlock the broader set.
        let kind = match text {
            "fn" => TokenKind::Fn,
            "pure" => TokenKind::Pure,
            "extern" => TokenKind::Extern,
            "unsafe" => TokenKind::Unsafe,
            "region" => TokenKind::RegionKw,
            "parallel" => TokenKind::Parallel,
            "reduce" => TokenKind::Reduce,
            "with" => TokenKind::With,
            "task" => TokenKind::Task,
            "join" => TokenKind::Join,
            // Note: `min` / `max` are NOT global reserved
            // keywords — they're context-sensitive
            // identifiers used by `reduce X with min;`
            // and the `min(a,b)` / `max(a,b)` intrinsics.
            // Users can declare struct fields, locals,
            // and other names called `min`/`max` without
            // collision.
            // Local binding: `let` is the idiomatic form;
            // `assign` reads naturally for newcomers approaching
            // from a Python / pseudo-code background. Closure
            // #255 — pure surface alias, identical AST.
            "let" | "assign" => TokenKind::Let,
            // Function exit: `return` and three English-natural
            // aliases. `give` is the verb form ("give the
            // value"); `give_back` is the snake-case multi-word
            // form; the two-word `give back` is folded later in
            // a post-lex pass (`merge_multi_word_give_back`) so
            // the surface accepts whichever spelling the writer
            // prefers. Closure #255.
            "return" | "give" | "give_back" => TokenKind::Return,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "mut" => TokenKind::Mut,
            "for" => TokenKind::For,
            "in" => TokenKind::In,
            "ref" => TokenKind::Ref,
            "from" => TokenKind::From,
            "to" => TokenKind::To,
            // Data shape: `struct` / `record`.
            "struct" | "record" => TokenKind::Struct,
            "enum" => TokenKind::Enum,
            "match" => TokenKind::Match,
            "then" => TokenKind::Then,
            // Interface: `interface` / `trait` (Rust-style).
            "interface" | "trait" => TokenKind::Interface,
            // Implementation: `implement` / `impl` (Rust-style).
            "implement" | "impl" => TokenKind::Implement,
            // Module declaration: `module` (canonical) / `mod`
            // (Rust-shorthand alias). Closure #242.
            "module" | "mod" => TokenKind::Module,
            // Visibility modifier: `pub` (canonical, Rust-style)
            // / `public` (alias for newcomers). Makes a
            // module-scoped item visible from outside the
            // module. Closure #242.
            "pub" | "public" => TokenKind::Pub,
            "where" => TokenKind::Where,
            "is" => TokenKind::Is,
            "const" => TokenKind::Const,
            "type" => TokenKind::Type,
            "methods" => TokenKind::Methods,
            "intent" => TokenKind::Intent,
            "use" => TokenKind::Use,
            "requires" => TokenKind::Requires,
            "ensures" => TokenKind::Ensures,
            "invariant" => TokenKind::Invariant,
            "assert" => TokenKind::Assert,
            "prove" => TokenKind::Prove,
            // Output: `print` (legacy / C-Python heritage) /
            // `write` (matches `write(stdout, ...)` style).
            // `write` is preferred in new code; both currently
            // accepted.
            "print" | "write" => TokenKind::Print,
            "try" => TokenKind::Try,
            "len" => TokenKind::Len,
            "as" => TokenKind::As,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            // Return-type arrow word forms: `returns` /
            // `yields` mean the same as `->`. Reads
            // naturally: `fn f(x: i64) yields i64 { ... }`.
            // Both words are uncommon as identifiers.
            "returns" | "yields" => TokenKind::Arrow,
            "i8" => TokenKind::I8,
            "i16" => TokenKind::I16,
            "i32" => TokenKind::I32,
            "i64" => TokenKind::I64,
            "u8" => TokenKind::U8,
            "u16" => TokenKind::U16,
            "u32" => TokenKind::U32,
            "u64" => TokenKind::U64,
            "f32" => TokenKind::F32,
            "f64" => TokenKind::F64,
            "bool" => TokenKind::Bool,
            "Vec" => TokenKind::Vec,
            // Phase 8b.1 (2026-06-07): when the word contains
            // non-ASCII characters (e.g. Spanish `función`,
            // French `très`), fall through to the script-keyword
            // chain instead of defaulting to `Ident`. Pure-ASCII
            // unknown words still become `Ident` so user
            // identifiers like `foo` don't accidentally match a
            // dialect keyword. Latin-with-accent dialects
            // (Spanish, future French/German) live in their own
            // tables called from this chain.
            _ if text.bytes().any(|b| b >= 0x80) => spanish_keyword(text)
                .or_else(|| french_keyword(text))
                .or_else(|| german_keyword(text))
                .or_else(|| portuguese_keyword(text))
                .or_else(|| polish_keyword(text))
                .or_else(|| turkish_keyword(text))
                .or_else(|| vietnamese_keyword(text))
                .or_else(|| romanian_keyword(text))
                .or_else(|| hungarian_keyword(text))
                .or_else(|| czech_keyword(text))
                .or_else(|| swedish_keyword(text))
                .or_else(|| norwegian_keyword(text))
                .or_else(|| danish_keyword(text))
                .or_else(|| slovak_keyword(text))
                .or_else(|| finnish_keyword(text))
                .or_else(|| catalan_keyword(text))
                .or_else(|| yoruba_keyword(text))
                .or_else(|| hausa_keyword(text))
                .unwrap_or_else(|| TokenKind::Ident(text.to_owned())),
            // Phase pragma threading (2026-06-08): pure-ASCII text
            // that doesn't match an English keyword routes through
            // the per-dialect ASCII tables ONLY when the file
            // declares that dialect's pragma. Otherwise the text
            // becomes a plain Ident — protects English code from
            // accidentally matching `si` (Spanish if) / `para`
            // (Spanish for) / `wahr` (German true) etc. as
            // keywords.
            _ => {
                let pragma_match = match self.pragma {
                    Some(DialectLang::Spanish) => spanish_ascii_keyword(text),
                    Some(DialectLang::French) => french_ascii_keyword(text),
                    Some(DialectLang::German) => german_ascii_keyword(text),
                    Some(DialectLang::Portuguese) => portuguese_ascii_keyword(text),
                    Some(DialectLang::Indonesian) => indonesian_ascii_keyword(text),
                    Some(DialectLang::Italian) => italian_ascii_keyword(text),
                    Some(DialectLang::Polish) => polish_ascii_keyword(text),
                    Some(DialectLang::Turkish) => turkish_ascii_keyword(text),
                    Some(DialectLang::Malay) => malay_ascii_keyword(text),
                    Some(DialectLang::Swahili) => swahili_ascii_keyword(text),
                    Some(DialectLang::Romanian) => romanian_ascii_keyword(text),
                    Some(DialectLang::Dutch) => dutch_ascii_keyword(text),
                    Some(DialectLang::Vietnamese) => vietnamese_ascii_keyword(text),
                    Some(DialectLang::Hungarian) => hungarian_ascii_keyword(text),
                    Some(DialectLang::Czech) => czech_ascii_keyword(text),
                    Some(DialectLang::Swedish) => swedish_ascii_keyword(text),
                    Some(DialectLang::Filipino) => filipino_ascii_keyword(text),
                    Some(DialectLang::Norwegian) => norwegian_ascii_keyword(text),
                    Some(DialectLang::Danish) => danish_ascii_keyword(text),
                    Some(DialectLang::Slovak) => slovak_ascii_keyword(text),
                    Some(DialectLang::Finnish) => finnish_ascii_keyword(text),
                    Some(DialectLang::Catalan) => catalan_ascii_keyword(text),
                    Some(DialectLang::Hausa) => hausa_ascii_keyword(text),
                    _ => None,
                };
                pragma_match.unwrap_or_else(|| TokenKind::Ident(text.to_owned()))
            }
        };

        self.tokens.push(Token {
            kind,
            span: Span::new(start, self.pos),
        });
    }

    fn lex_string(&mut self, start: usize) -> Result<(), Diagnostic> {
        let mut value = String::new();

        while let Some(byte) = self.peek() {
            match byte {
                b'"' => {
                    self.advance();
                    self.tokens.push(Token {
                        kind: TokenKind::Str(value),
                        span: Span::new(start, self.pos),
                    });
                    return Ok(());
                }
                b'\\' => {
                    self.advance();
                    let Some(escaped) = self.peek() else {
                        break;
                    };
                    self.advance();
                    match escaped {
                        b'"' => value.push('"'),
                        b'\\' => value.push('\\'),
                        b'n' => value.push('\n'),
                        b't' => value.push('\t'),
                        b'r' => value.push('\r'),
                        b'0' => value.push('\0'),
                        other => {
                            return Err(Diagnostic::new(
                                Span::new(self.pos.saturating_sub(2), self.pos),
                                format!("unknown escape sequence '\\{}'", other as char),
                            ));
                        }
                    }
                }
                b'\n' => {
                    return Err(Diagnostic::new(
                        Span::new(start, self.pos),
                        "string literal cannot span lines",
                    ));
                }
                _ => {
                    let char_start = self.pos;
                    let ch = self
                        .next_char()
                        .ok_or_else(|| Diagnostic::new(
                            Span::new(char_start, self.pos),
                            "invalid character in string literal",
                        ))?;
                    value.push(ch);
                }
            }
        }

        Err(Diagnostic::new(
            Span::new(start, self.pos),
            "unterminated string literal",
        ))
    }

    fn skip_line_comment(&mut self) {
        while !matches!(self.peek(), None | Some(b'\n')) {
            self.advance();
        }
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.bytes.len()
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn peek_next(&self) -> Option<u8> {
        self.bytes.get(self.pos + 1).copied()
    }

    fn advance(&mut self) -> u8 {
        let byte = self.bytes[self.pos];
        self.pos += 1;
        byte
    }

    fn next_char(&mut self) -> Option<char> {
        let ch = self.source[self.pos..].chars().next()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }

    fn match_byte(&mut self, expected: u8) -> bool {
        if self.peek() == Some(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn push(&mut self, kind: TokenKind, start: usize) {
        self.tokens.push(Token {
            kind,
            span: Span::new(start, self.pos),
        });
    }
}

fn strip_underscores(text: &str) -> String {
    text.chars().filter(|ch| *ch != '_').collect()
}

fn is_digit_for_radix(byte: u8, radix: u32) -> bool {
    match radix {
        2 => matches!(byte, b'0' | b'1'),
        8 => matches!(byte, b'0'..=b'7'),
        16 => byte.is_ascii_hexdigit(),
        10 => byte.is_ascii_digit(),
        _ => false,
    }
}
