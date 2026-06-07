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
        // mut — closure #267 fills Sanskrit + Hindi gaps
        "बदल" => TokenKind::Mut,          // badla (Marathi root: "change")
        "परिवर्तनीय" => TokenKind::Mut,   // parivartanīya (Sanskrit/Hindi: "mutable")
        // match
        "जुळवा" => TokenKind::Match,      // juḷvā (Marathi: "match")
        "मिलान" => TokenKind::Match,      // milān (Hindi: "match")
        "मेल" => TokenKind::Match,        // mela (Sanskrit: "join/match")
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
        // loanwords) widely used in all three languages. Add
        // colloquial Hindi/Marathi alternates. Closure #267.
        "सत्य" => TokenKind::True,         // satya (Sanskrit, shared)
        "सही" => TokenKind::True,          // sahī (Hindi/Marathi colloquial: "correct")
        "असत्य" => TokenKind::False,       // asatya (Sanskrit, shared)
        "अशुद्ध" => TokenKind::False,      // aśuddha (Hindi/Marathi: "incorrect")
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
        "সত্য" => TokenKind::True,             // satya (truth)
        "অসত্য" => TokenKind::False,           // asatya (untruth)
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
        "જ્યાં સુધી" => TokenKind::While,         // jyaan sudhi (while/until — TBD multi-word)
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
        "ಸುಳ್ಳು" => TokenKind::False,                // sullu (false)
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
        "അസത്യം" => TokenKind::False,                // asathyam (false — tatsama)
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
        "අසත්‍ය" => TokenKind::False,                 // asathya (false — tatsama)
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
        }
        Script::Latin
    }
}

/// Scan the first ~10 lines of source for a `// vani-lang: <name>`
/// pragma comment. Returns the declared dialect when found, or
/// None for back-compat (no pragma → script-level purity only).
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
        "परिवर्तनीय" => &[Sanskrit, Hindi],
        // === MATCH ===
        "जुळवा" => &[Marathi],
        "मिलान" => &[Hindi],
        "मेल" => &[Sanskrit],
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
        "सही" => &[Hindi, Marathi],
        "असत्य" => &[Sanskrit, Hindi, Marathi],  // tatsama
        "अशुद्ध" => &[Hindi, Marathi],
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
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            bytes: source.as_bytes(),
            pos: 0,
            tokens: Vec::new(),
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
            .unwrap_or_else(|| TokenKind::Ident(text.to_owned()));
        self.tokens.push(Token {
            kind,
            span: Span::new(start, self.pos),
        });
    }

    fn lex_ident(&mut self, start: usize) {
        while matches!(
            self.peek(),
            Some(b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_')
        ) {
            self.advance();
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
            _ => TokenKind::Ident(text.to_owned()),
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
