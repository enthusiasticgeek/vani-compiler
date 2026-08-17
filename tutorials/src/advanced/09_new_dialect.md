# Advanced 9 -- Adding a new dialect

> **Learning goal**: walk through the exact steps to add a new
> dialect to vāṇī, using the per-script abstraction set up in
> Phase 5b / 6.

**Who this chapter is for**: contributors who want to add a
new human-language dialect (keyword table, examples, tests).
Adding a Tier I dialect (a language that shares script and
grammar patterns with an existing one) typically takes 4-8
hours; a brand-new script family takes 15-30 hours. Read this
chapter before opening a PR -- it covers the exact files to
touch, the test checklist, and the native-speaker review gate.

## Two levels of "adding a dialect"

There are two distinct levels of work:

| Level | What it unlocks | Effort |
|-------|----------------|--------|
| **Translator only** | `vani_translate.py` can output the language; you can read source in that language if you already have it | Add an entry to `LANG_TABLES` in `tools/regen_vani_translate_keywords.py` and re-run it — no Rust changes |
| **Compiler-level** | `vanic` can parse, type-check, and compile `.vani` files *written* in that language | Lexer + backend + diagnostic changes in Rust |

The translator already covers all 63 languages the compiler itself
supports (see [Section 8](08_translator.md)) — there's no
translator-only superset anymore, so this row only applies if you're
extending the translator's keyword coverage for a language it's
missing a specific word for, not adding a wholly new one. For
compiler-level support (a genuinely new language), continue with the
rest of this chapter.

## Quick decision tree

Before you start, decide which case you're in:

1. **New script** (own Unicode block) -> full work.
2. **Existing script, new dialect** (e.g. Assamese reusing
   Bengali) -> much lighter.

The 9-language Phase 6 batch is the reference: each new Brahmi
script lit up via ~10 lines of mechanical change per language,
once the per-script abstraction landed.

## Case 1: new script (full work)

Suppose you're adding Burmese (မြန်မာ, U+1000..U+109F).

### 1. Lexer -- keyword table

In `src/lexer.rs`, add a new `*_keyword` function:

```rust
fn burmese_keyword(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "လုပ်ဆောင်ချက်" => TokenKind::Fn,
        "ထား" => TokenKind::Let,
        "ပြန်" => TokenKind::Return,
        // ...~30 more entries
        _ => return None,
    };
    Some(kind)
}
```

Then wire it into the fallback chain in `lex_unicode_ident`:

```rust
let kind = devanagari_keyword(text)
    .or_else(|| bengali_keyword(text))
    .or_else(|| tamil_keyword(text))
    .or_else(|| burmese_keyword(text))   // <-- new
    ...
```

### 2. Dialect enum + pragma

Add the `DialectLang` variant:

```rust
enum DialectLang {
    ...
    Burmese,
}
```

Add the pragma alias in `detect_language_pragma`:

```rust
"burmese" | "myanmar" | "my" => Some(DialectLang::Burmese),
```

Update `DialectLang::name()` and `DialectLang::script()`.

### 3. Script enum

Add a new `Script` variant + Unicode block check in
`Script::classify`:

```rust
if ('\u{1000}'..='\u{109F}').contains(&c) {
    return Script::Burmese;
}
```

Update `script_label`.

### 4. PrintLangMode + numerals

Add a new `PrintLangMode::Burmese` variant. Wire it through:

- The lex-end mapping (DialectLang -> PrintLangMode).
- Backend-c helper: `emit_intent_print_int_bur_c` with the
  correct UTF-8 middle byte for Burmese numerals (U+1040 =
  `0xE1 0x81 0x80`; lead byte = 0x81 -- note this is
  *outside* the U+0xxx range, so the existing helper-emit
  template needs a tweak).
- Same for SSA-C, tree-LLVM, SSA-LLVM.

<img class="manas" src="../images/mascot/manas_mascot_caution.png" title="this code needs extra care"/>

**This step is real -- and shipped Burmese is still missing it** --
confirmed by testing (`examples/language/burmese/basics.vani`, a real
example in the repo, prints its integer result in plain ASCII digits,
not Burmese digits). `PrintLangMode` (`src/lexer.rs`) has no
`Burmese` variant at all; the enum's last entry is `Persian`. Steps
1-3, 5, and 6 below ARE fully done for Burmese (it's a real,
compiler-level dialect you can use today, per Phase 13.30) -- only
this specific numeral-printing step was never finished. If you pick
Burmese as your reference dialect while reading this chapter, don't
expect step 4 to have a finished example to point at; it's exactly
the kind of gap this Challenge section's "sketch the full set of
changes" exercise is good practice for.

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
4. Map it to the existing `PrintLangMode` (Assamese ->
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
- `Script::classify` is a linear scan over Unicode ranges --
  add a new range, you're done.
- `script_label` is a static match -- one new arm.
- `enforce_language_purity` is **completely unchanged** -- it
  tracks "first observed script" generically.

## What's not handled

- **Visual bidi shaping**. The Perso-Arabic Urdu dialect
  shipped in Phase 12 *does* work, because the lexer reads
  UTF-8 in logical (byte) order -- RTL is a rendering concern
  of the editor, not a compiler concern. But cursor
  navigation + selection in RTL source files is the editor's
  job, not vāṇī's.
- **Logographic scripts** (Mandarin, Japanese kanji) historically
  needed a tokenizer that knew about CJK word boundaries.
  Japanese (Phase 9b) and Mandarin (Phase 10.2, 2026-06-08)
  both ship today -- the convention is that users separate
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
- Tests: `src/lib.rs` -- search for
  `_pragma_compiles_and_emits_`.

## Challenge

Tibetan, Lao, and Khmer are already in the *translator* (see
[Section 8](08_translator.md)). For compiler-level support, pick
one of them and sketch the full set of changes described above:
lexer keyword function, `DialectLang` variant, `Script` range,
`PrintLangMode` variant, diagnostic labels. Write it as a draft PR
description — you don't need to compile, but writing out the change
forces you to read the relevant source files and understand the
abstraction boundaries.

---

**Previous**: [Sec.8 -- Writing a cross-language translator extension ->](08_translator.md)
**Next**: [Sec.10 -- Compiler internals tour ->](10_internals.md)
