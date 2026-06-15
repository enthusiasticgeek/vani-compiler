# Advanced 9 — Adding a new dialect

> **Learning goal**: walk through the exact steps to add a new
> dialect to vāṇी, using the per-script abstraction set up in
> Phase 5b / 6.

## Quick decision tree

Before you start, decide which case you're in:

1. **New script** (own Unicode block) → full work.
2. **Existing script, new dialect** (e.g. Assamese reusing
   Bengali) → much lighter.

The 9-language Phase 6 batch is the reference: each new Brahmi
script lit up via ~10 lines of mechanical change per language,
once the per-script abstraction landed.

## Case 1: new script (full work)

Suppose you're adding Burmese (မြန်မာ, U+1000..U+109F).

### 1. Lexer — keyword table

In `src/lexer.rs`, add a new `*_keyword` function:

```vani
fn burmese_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "လုပ်ဆောင်ချက်" => TokenKind::Fn,
        "ထား" => TokenKind::Let,
        "ပြန်" => TokenKind::Return,
        // …~30 more entries
        _ => return None,
    };
    Some(kind)
}
```

Then wire it into the fallback chain in `lex_unicode_ident`:

```vani
let kind = devanagari_keyword(text)
    .or_else(|| bengali_keyword(text))
    .or_else(|| tamil_keyword(text))
    .or_else(|| burmese_keyword(text))   // <-- new
    ...
```

### 2. Dialect enum + pragma

Add the `DialectLang` variant:

```vani
enum DialectLang {
    ...
    Burmese,
}
```

Add the pragma alias in `detect_language_pragma`:

```vani
"burmese" | "myanmar" | "my" => Some(DialectLang::Burmese),
```

Update `DialectLang::name()` and `DialectLang::script()`.

### 3. Script enum

Add a new `Script` variant + Unicode block check in
`Script::classify`:

```vani
if ('\u{1000}'..='\u{109F}').contains(&c) {
    return Script::Burmese;
}
```

Update `script_label`.

### 4. PrintLangMode + numerals

Add a new `PrintLangMode::Burmese` variant. Wire it through:

- The lex-end mapping (DialectLang → PrintLangMode).
- Backend-c helper: `emit_intent_print_int_bur_c` with the
  correct UTF-8 middle byte for Burmese numerals (U+1040 =
  `0xE1 0x81 0x80`; lead byte = 0x81 — note this is
  *outside* the U+0xxx range, so the existing helper-emit
  template needs a tweak).
- Same for SSA-C, tree-LLVM, SSA-LLVM.

### 5. Diagnostic labels

Add `DiagLang::Burmese` + native error labels + a starter
prefix-translation table.

### 6. Example + tests

Drop `examples/language/burmese/basics.vani` with a small
program. Add to the parity sweep. Add 2-3 lib tests pinning
the helper emit + a cross-script-rejection regression.

### 7. Docs

Update README's Tier-I table, the `tools/llm_context/bundle.py`
output (auto-picks up `ALIASES` changes), and STATUS.md.

## Case 2: existing script, new dialect (Assamese pattern)

If your new dialect shares an existing Brahmi script (like
Assamese sharing Bengali), the work is much lighter:

1. Add a `DialectLang` variant (`DialectLang::Bodo` or
   whatever).
2. Map it to the existing script in `script()`.
3. Add the pragma alias.
4. Map it to the existing `PrintLangMode` (Assamese →
   `PrintLangMode::Bengali`).
5. (Optional) Add a tiny `DiagLang` variant that collapses to
   the existing one in `localize_message`.
6. Example + 1 test.

No new keyword table, no new backend helper, no new numeral
codepoints.

## What gets a free ride from the refactor

Thanks to Phase 6's parameterized print helpers:

- The C backend's `emit_intent_print_int_helper_c(out, suffix,
  lead_byte)` accepts any 3-byte UTF-8 codepoint block. New
  script = one new emit call.
- The LLVM backends' `emit_brahmi_print_helper_ll` + matches!
  arm work the same way.
- `Script::classify` is a linear scan over Unicode ranges —
  add a new range, you're done.
- `script_label` is a static match — one new arm.
- `enforce_language_purity` is **completely unchanged** — it
  tracks "first observed script" generically.

## What's not handled

- **Visual bidi shaping**. The Perso-Arabic Urdu dialect
  shipped in Phase 12 *does* work, because the lexer reads
  UTF-8 in logical (byte) order — RTL is a rendering concern
  of the editor, not a compiler concern. But cursor
  navigation + selection in RTL source files is the editor's
  job, not vāṇी's.
- **Logographic scripts** (Mandarin, Japanese kanji) historically
  needed a tokenizer that knew about CJK word boundaries.
  Japanese (Phase 9b) and Mandarin (Phase 10.2, 2026-06-08)
  both ship today — the convention is that users separate
  identifiers from keywords with whitespace, same as natural
  CJK programming style. No dictionary-driven segmenter
  required.
- **Multi-script bidialects** (Punjabi-Gurmukhi vs Punjabi-
  Shahmukhi) need two parallel pragma tags pointing at two
  different `Script` variants. v1 ships Punjabi-Gurmukhi
  (Phase 6) and queues Punjabi-Shahmukhi (Phase 12.x).

## Source-of-truth pointers

- Lexer changes: search for `Phase 6` in `src/lexer.rs`.
- Backend changes: search for `intent_print_int_helper` in
  `src/backend_c.rs` and `emit_brahmi_print_helper_ll` in
  `src/backend_llvm.rs`.
- Tests: `src/lib.rs` — search for
  `_pragma_compiles_and_emits_`.

## Challenge

Pick a Brahmi-derived script not yet in vāṇी (Tibetan,
Lao, Khmer). Sketch the keyword table for ~10 of the most
important `TokenKind`s. Don't ship — but write the change as a
draft PR to feel the workflow.

---

**Next**: [§10 — Compiler internals tour →](10_internals.md)
