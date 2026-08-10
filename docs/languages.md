# vāṇī — Global Language Coverage

vāṇī supports 62 dialects across 26 scripts via a per-file pragma:

```vani
// vani-lang: hindi

fn मुख्य() -> i64 {
    माना x: i64 = 42;
    लिखो x;
    पुनरागम 0;
}
```

The English-keyword default always works and is unchanged. The pragma opts a
single file into a different surface; mixing is not allowed within one file.

> **⚠️ Verification caveat.** The authors have first-hand fluency in English and
> the Devanagari Indo-Aryan family (Sanskrit / Hindi / Marathi). Every other
> dialect's keyword table was drafted from reference grammars, tatsama/loan-word
> patterns, and CS-vocabulary conventions, **but has not been validated by a
> native speaker**. Keyword choices may sound wrong, overly formal, or archaic.
>
> **If you read any listed language natively, please open an issue or PR.**
> The lexer table is one file; corrections are a mechanical 6-touchpoint change.
> Non-Devanagari-Indo-Aryan dialects should be treated as *technical
> proofs-of-concept* until a grammar-consultant pass lands.

---

## Tier I — Indian subcontinent

| # | Language | Script | Status | Verified by native speaker |
|---|----------|--------|--------|---------------------------|
| 1 | Sanskrit (*saṁskṛta*) | Devanagari | ✅ Shipped — 91 aliases, 8 SOV shapes, 11 examples | Authors (primary) |
| 2 | Hindi (*hindī*) | Devanagari | ✅ Shipped — 9 examples | Authors (primary) |
| 3 | Marathi (*marāṭhī*) | Devanagari | ✅ Shipped — 9 examples | Authors (primary) |
| 4 | Bengali (*baṅlā*) | Bengali (Brahmi) | ✅ Shipped — 46/46 structure keywords, SOV supported | ❌ Needs review |
| 5 | Gujarati (*gujarātī*) | Gujarati (Brahmi) | ✅ Shipped — 46/46 structure keywords, SOV supported | ❌ Needs review |
| 6 | Punjabi — Gurmukhi (*pañjābī*) | Gurmukhi (Brahmi) | ✅ Shipped — 46/46 structure keywords, SOV supported | ❌ Needs review |
| 6b | Punjabi — Shahmukhi | Perso-Arabic (RTL) | ✅ Shipped — inherits Urdu's 46/46, SOV supported | ❌ Needs review |
| 7 | Tamil (*tamiḻ*) | Tamil (Dravidian) | ✅ Shipped — 46/46 structure keywords, SOV supported | ❌ Needs review |
| 8 | Telugu (*telugu*) | Telugu | ✅ Shipped — 46/46 structure keywords, SOV supported | ❌ Needs review |
| 9 | Kannada (*kannaḍa*) | Kannada | ✅ Shipped — 46/46 structure keywords, SOV supported | ❌ Needs review |
| 10 | Malayalam (*malayāḷam*) | Malayalam | ✅ Shipped — 46/46 structure keywords, SOV supported | ❌ Needs review |
| 11 | Urdu (*urdū*) | Perso-Arabic (RTL) | ✅ Shipped — 46/46 structure keywords, SOV supported | ❌ Needs review |
| 12 | Odia (*oṛiā*) | Odia (Brahmi) | ✅ Shipped — 46/46 structure keywords, SOV supported | ❌ Needs review |
| 13 | Assamese (*ɔxɔmia*) | Assamese/Bengali (Brahmi) | ✅ Shipped — inherits Bengali's 46/46, SOV supported | ❌ Needs review |
| 14 | Sindhi (*sindhī*) | Perso-Arabic (RTL) | ✅ Shipped — inherits Urdu's 46/46, SOV supported | ❌ Needs review |
| 15 | Nepali (*nepālī*) | Devanagari | ✅ Shipped — inherits Sanskrit/Hindi/Marathi's 46/46, SOV supported | ❌ Needs review |
| 16 | Konkani (*kõkaṇī*) | Devanagari | ✅ Shipped (Devanagari only) — inherits Sanskrit/Hindi/Marathi's 46/46, SOV supported | ❌ Needs review |
| 17 | Maithili (*maithilī*) | Devanagari | ✅ Shipped (Devanagari only) — inherits Sanskrit/Hindi/Marathi's 46/46, SOV supported | ❌ Needs review |
| 18 | Sinhala (*siṁhala*) | Sinhala (Brahmi) | ✅ Shipped — 46/46 structure keywords, SOV supported | ❌ Needs review |

**BUG-169 (2026-08-10)**: a keyword-parity audit found rows 4-18 missing
5-16 of the 46 required structure keywords each (`Pure`/`Extern`/
`Parallel`/`Reduce`/`With`/`Task`/`Join`/`Interface`/`Implement`/`Where`/
`Is`/`Methods`/`EPrint`/`Try`/`Unsafe`/`RegionKw` in various
combinations across the 10 dialects with their own dedicated keyword
table — Bengali, Tamil, Telugu, Gujarati, Punjabi, Kannada, Malayalam,
Odia, Sinhala, Urdu). All fixed; a permanent regression test
(`bug169_india_dialects_structure_keyword_parity` in `src/lib.rs`)
mechanically re-derives coverage from `src/lexer.rs`'s own source on
every test run, so a future keyword addition that misses a dialect
fails CI immediately. Assamese/Sindhi/Punjabi-Shahmukhi (which have no
dedicated table of their own — they inherit Bengali's / Urdu's / Urdu's
respectively) and Nepali/Maithili/Konkani (inherit the Sanskrit/Hindi/
Marathi union) automatically picked up the same fix; a second test
(`bug169_union_inheritance_dialects_compile`) proves that inheritance
holds end-to-end with a real compile.

**SOV vs. SVO**: every Indian-subcontinent language in this table is
verb-final (SOV) in natural word order — a well-known areal
(Sprachbund) feature shared across the Indo-Aryan and Dravidian
families here, with no SVO exceptions. The parser's SOV verb-at-end
grammar (`x लिख;` = `print x;`, the `IDENT for ...` range-for shape,
etc.) turned out to already be dialect-agnostic despite being
documented and commented as "Devanagari SOV" throughout the codebase —
the detectors key off `TokenKind`s (`Return`/`Print`/`EPrint`/`Assert`/
`Prove`/`Let`/`For`), not the declared dialect, so SOV syntax works for
any dialect whose keyword table maps to those kinds. Verified directly
against Bengali (Brahmi), Tamil (Dravidian), and Urdu (Perso-Arabic,
RTL) as of BUG-169 — representative samples across every script family
in this table. The one piece that remains genuinely Devanagari-only is
*multi-word* alias merging (e.g. Hindi's two-token "के लिए" for `for`)
— a separate lexer feature from SOV grammar itself, since every
dialect here already has its own single-token spelling for every SOV-
eligible keyword.

---

## Tier II — Global

| # | Language | Script | Word order | Status | Verified |
|---|----------|--------|-----------|--------|---------|
| 1 | Spanish (*español*) | Latin | SVO | ✅ 46/46 keywords | ❌ |
| 2 | French (*français*) | Latin | SVO | ✅ 46/46 keywords | ❌ |
| 3 | German (*deutsch*) | Latin | V2/SOV | ✅ 46/46 keywords, SOV supported | ❌ |
| 4 | Russian (*русский*) | Cyrillic | SVO | ✅ 46/46 keywords | ❌ |
| 5 | Italian (*italiano*) | Latin | SVO | ✅ 46/46 keywords | ❌ |
| 6 | Portuguese (*português*) | Latin | SVO | ✅ 46/46 keywords | ❌ |
| 7 | Polish (*polski*) | Latin | SVO | ✅ 46/46 keywords | ❌ |
| 8 | Turkish (*Türkçe*) | Latin | SOV | ✅ 46/46 keywords, SOV supported | ❌ |
| 9 | Vietnamese (*Tiếng Việt*) | Latin | SVO | ✅ 46/46 keywords | ❌ |
| 10 | Romanian (*română*) | Latin | SVO | ✅ 46/46 keywords | ❌ |
| 11 | Dutch (*Nederlands*) | Latin | V2/SOV | ✅ 46/46 keywords, SOV supported | ❌ |
| 12 | Hungarian (*magyar*) | Latin | SOV | ✅ 46/46 keywords, SOV supported | ❌ |
| 13 | Czech (*čeština*) | Latin | free | ✅ 46/46 keywords | ❌ |
| 14 | Slovak (*slovenčina*) | Latin | free | ✅ 46/46 keywords | ❌ |
| 15 | Swedish (*svenska*) | Latin | SVO | ✅ 46/46 keywords | ❌ |
| 16 | Norwegian (*norsk bokmål*) | Latin | SVO | ✅ 46/46 keywords | ❌ |
| 17 | Danish (*dansk*) | Latin | SVO | ✅ 46/46 keywords | ❌ |
| 18 | Finnish (*suomi*) | Latin | SVO | ✅ 46/46 keywords | ❌ |
| 19 | Catalan (*català*) | Latin | SVO | ✅ 46/46 keywords | ❌ |
| 20 | Arabic (*العربية*) | Arabic (RTL) | VSO | ✅ 46/46 keywords | ❌ |
| 21 | Korean (*한국어*) | Hangul | SOV | ✅ 46/46 keywords, SOV supported | ❌ |
| 22 | Japanese (*日本語*) | Kanji+Hiragana+Katakana | SOV | ✅ 46/46 keywords, SOV supported | ❌ |
| 23 | Greek (*Ελληνικά*) | Greek | SVO | ✅ 46/46 keywords | ❌ |
| 24 | Hebrew (*עברית*) | Hebrew (RTL) | SVO | ✅ 46/46 keywords | ❌ |
| 25 | Thai (*ไทย*) | Thai | SVO | ✅ 46/46 keywords | ❌ |
| 26 | Khmer (*ខ្មែរ*) | Khmer | SVO | ✅ 46/46 keywords | ❌ |
| 27 | Burmese (*မြန်မာ*) | Myanmar | SOV | ✅ 46/46 keywords, SOV supported | ❌ |
| 28 | Lao (*ລາວ*) | Lao | SVO | ✅ 46/46 keywords | ❌ |
| 29 | Amharic (*አማርኛ*) | Ethiopic | SOV | ✅ 46/46 keywords, SOV supported | ❌ |
| 30 | Tibetan (*བོད་ཡིག*) | Tibetan | SOV | ✅ 46/46 keywords, SOV supported | ❌ |
| 31 | Cherokee (*ᏣᎳᎩ*) | Cherokee syllabary | SOV | ✅ 46/46 keywords, SOV supported | ❌ |
| 32 | Mongolian (*ᠮᠣᠩᠭᠣᠯ*) | Mongolian traditional | SOV | ✅ 46/46 keywords, SOV supported | ❌ |
| 33 | Armenian (*Հայերեն*) | Armenian | SOV | ✅ 46/46 keywords, SOV supported | ❌ |
| 34 | Georgian (*ქართული*) | Georgian Mkhedruli | SOV | ✅ 46/46 keywords, SOV supported | ❌ |
| 35 | Indonesian (*Bahasa Indonesia*) | Latin | SVO | ✅ 46/46 keywords | ❌ |
| 36 | Malay (*Bahasa Melayu*) | Latin | SVO | ✅ 46/46 keywords | ❌ |
| 37 | Filipino (*Tagalog*) | Latin | VSO | ✅ 46/46 keywords | ❌ |
| 38 | Swahili (*Kiswahili*) | Latin | SVO | ✅ 46/46 keywords | ❌ |
| 39 | Yoruba (*Èdè Yorùbá*) | Latin | SVO | ✅ 46/46 keywords | ❌ |
| 40 | Hausa | Latin | SVO | ✅ 46/46 keywords | ❌ |
| 41 | Persian (*فارسی*) | Perso-Arabic (RTL) | SOV | ✅ 46/46 keywords, SOV supported | ❌ |
| 42 | Pashto (*پښتو*) | Perso-Arabic (RTL) | SOV | ✅ 46/46 keywords, SOV supported | ❌ |
| 62 | Mandarin Chinese (*中文*) | Han logograms | SVO | ✅ 46/46 keywords | ❌ |

**BUG-170 (2026-08-10)**: the "global languages" round of the
keyword-parity audit (following BUG-166's Sanskrit/Hindi/Marathi and
BUG-169's 10-dialect India sweep). Found 49 of these 60 keyword-table
functions each missing 3-44 of the 46 required structure keywords --
`Reduce`/`With`/`EPrint` was the near-universal minimal gap; several
Latin-script dialects' native (accented) tables were only 15-30%
populated, since most of their real coverage lives in a sibling
`_ascii_keyword` table instead (both are consulted for a pragma-
declared file -- see `Lexer::lex_ident`'s dispatch chain -- so the two
tables' union is what actually matters). All 43 affected dialects are
now fully 46/46 -- confirmed both by a mechanical union-aware parity
check and by real `vanic check`/`run` smoke tests. Two dialects
(Persian, Pashto) had a working, complete keyword table in `src/
lexer.rs` this whole time but were never listed in either tier of this
document -- added above. Also found and fixed a latent, unrelated bug
while auditing Pashto: 5 keyword entries (`As`/`Else`/`While`/`For`/
`Mut`) were spelled with an internal space (e.g. `"په توګه"` for `As`)
-- these could never match, since Pashto has no multi-word merge pass
(unlike Devanagari's), so the lexer's whitespace-delimited tokenizer
could never produce the multi-token span the pattern expected. Fixed
by fusing each into a single token.

**SOV vs. SVO**: the doc's own Word-order column already flagged which
Tier II dialects are verb-final -- German (V2/SOV in subordinate
clauses), Turkish, Dutch (V2/SOV), Hungarian, Korean, Japanese,
Burmese, Amharic, Tibetan, Cherokee, Mongolian, Armenian, Georgian,
Persian, and Pashto. As with the Tier I finding, the parser's SOV
verb-at-end grammar turned out to already be dialect-agnostic (keyed
on `TokenKind`, not the declared dialect), so no parser changes were
needed -- verified directly against Japanese (Kanji/Hiragana/
Katakana), Korean (Hangul), and Turkish (Latin) with real compile-and-
run programs on both backends, including the `IDENT for ... { }`
range-for SOV shape. VSO languages (Arabic, Filipino) and free-word-
order ones (Czech, Slovak) correctly do NOT get SOV grammar forced on
them -- their existing keyword-first surface already matches their
natural order.

---

## How to add or correct a dialect

Adding a new dialect or correcting keywords is a 6-touchpoint mechanical change:

1. `src/lexer.rs` — Script block classification + keyword aliases
2. `src/lexer.rs` — `DialectLang` enum entry
3. `src/lexer.rs` — `enforce_language_purity` range
4. `src/parser.rs` — `// vani-lang:` pragma recognition
5. `src/checker.rs` — diagnostic language labels
6. `examples/language/<dialect>/basics.vani` — minimal example

Open a PR with the diff; the maintainers will merge native-speaker corrections
without requiring a full review cycle.

---

## Native numerals

Brahmi-derived scripts print in their own numeral glyphs when `// vani-lang:`
is set. RTL scripts (Arabic, Hebrew, Urdu, Sindhi, Shahmukhi) use the correct
Unicode numeral block.

| Script family | Numeral block |
|---------------|--------------|
| Devanagari | U+0966–096F (०–९) |
| Bengali / Assamese | U+09E6–09EF (০–৯) |
| Gujarati | U+0AE6–0AEF (૦–૯) |
| Tamil | U+0BE6–0BEF (௦–௯) |
| Telugu | U+0C66–0C6F (౦–౯) |
| Kannada | U+0CE6–0CEF (೦–೯) |
| Malayalam | U+0D66–0D6F (൦–൯) |
| Odia | U+0B66–0B6F (୦–୯) |
| Sinhala | U+0DE6–0DEF (෦–෯) |
| Arabic-Indic (Urdu, Sindhi, Shahmukhi) | U+0660–0669 (٠–٩) |
