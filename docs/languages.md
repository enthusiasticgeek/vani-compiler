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
| 4 | Bengali (*baṅlā*) | Bengali (Brahmi) | ✅ Shipped | ❌ Needs review |
| 5 | Gujarati (*gujarātī*) | Gujarati (Brahmi) | ✅ Shipped | ❌ Needs review |
| 6 | Punjabi — Gurmukhi (*pañjābī*) | Gurmukhi (Brahmi) | ✅ Shipped | ❌ Needs review |
| 6b | Punjabi — Shahmukhi | Perso-Arabic (RTL) | ✅ Shipped | ❌ Needs review |
| 7 | Tamil (*tamiḻ*) | Tamil (Dravidian) | ✅ Shipped | ❌ Needs review |
| 8 | Telugu (*telugu*) | Telugu | ✅ Shipped | ❌ Needs review |
| 9 | Kannada (*kannaḍa*) | Kannada | ✅ Shipped | ❌ Needs review |
| 10 | Malayalam (*malayāḷam*) | Malayalam | ✅ Shipped | ❌ Needs review |
| 11 | Urdu (*urdū*) | Perso-Arabic (RTL) | ✅ Shipped | ❌ Needs review |
| 12 | Odia (*oṛiā*) | Odia (Brahmi) | ✅ Shipped | ❌ Needs review |
| 13 | Assamese (*ɔxɔmia*) | Assamese/Bengali (Brahmi) | ✅ Shipped | ❌ Needs review |
| 14 | Sindhi (*sindhī*) | Perso-Arabic (RTL) | ✅ Shipped | ❌ Needs review |
| 15 | Nepali (*nepālī*) | Devanagari | ✅ Shipped | ❌ Needs review |
| 16 | Konkani (*kõkaṇī*) | Devanagari | ✅ Shipped (Devanagari only) | ❌ Needs review |
| 17 | Maithili (*maithilī*) | Devanagari | ✅ Shipped (Devanagari only) | ❌ Needs review |
| 18 | Sinhala (*siṁhala*) | Sinhala (Brahmi) | ✅ Shipped | ❌ Needs review |

---

## Tier II — Global

| # | Language | Script | Word order | Status | Verified |
|---|----------|--------|-----------|--------|---------|
| 1 | Spanish (*español*) | Latin | SVO | ✅ Shipped | ❌ |
| 2 | French (*français*) | Latin | SVO | ✅ Shipped | ❌ |
| 3 | German (*deutsch*) | Latin | V2/SOV | ✅ Shipped | ❌ |
| 4 | Russian (*русский*) | Cyrillic | SVO | ✅ Shipped | ❌ |
| 5 | Italian (*italiano*) | Latin | SVO | ✅ Shipped | ❌ |
| 6 | Portuguese (*português*) | Latin | SVO | ✅ Shipped | ❌ |
| 7 | Polish (*polski*) | Latin | SVO | ✅ Shipped | ❌ |
| 8 | Turkish (*Türkçe*) | Latin | SOV | ✅ Shipped | ❌ |
| 9 | Vietnamese (*Tiếng Việt*) | Latin | SVO | ✅ Shipped | ❌ |
| 10 | Romanian (*română*) | Latin | SVO | ✅ Shipped | ❌ |
| 11 | Dutch (*Nederlands*) | Latin | V2/SOV | ✅ Shipped | ❌ |
| 12 | Hungarian (*magyar*) | Latin | SOV | ✅ Shipped | ❌ |
| 13 | Czech (*čeština*) | Latin | free | ✅ Shipped | ❌ |
| 14 | Slovak (*slovenčina*) | Latin | free | ✅ Shipped | ❌ |
| 15 | Swedish (*svenska*) | Latin | SVO | ✅ Shipped | ❌ |
| 16 | Norwegian (*norsk bokmål*) | Latin | SVO | ✅ Shipped | ❌ |
| 17 | Danish (*dansk*) | Latin | SVO | ✅ Shipped | ❌ |
| 18 | Finnish (*suomi*) | Latin | SVO | ✅ Shipped | ❌ |
| 19 | Catalan (*català*) | Latin | SVO | ✅ Shipped | ❌ |
| 20 | Arabic (*العربية*) | Arabic (RTL) | VSO | ✅ Shipped | ❌ |
| 21 | Korean (*한국어*) | Hangul | SOV | ✅ Shipped | ❌ |
| 22 | Japanese (*日本語*) | Kanji+Hiragana+Katakana | SOV | ✅ Shipped | ❌ |
| 23 | Greek (*Ελληνικά*) | Greek | SVO | ✅ Shipped | ❌ |
| 24 | Hebrew (*עברית*) | Hebrew (RTL) | SVO | ✅ Shipped | ❌ |
| 25 | Thai (*ไทย*) | Thai | SVO | ✅ Shipped | ❌ |
| 26 | Khmer (*ខ្មែរ*) | Khmer | SVO | ✅ Shipped | ❌ |
| 27 | Burmese (*မြန်မာ*) | Myanmar | SOV | ✅ Shipped | ❌ |
| 28 | Lao (*ລາວ*) | Lao | SVO | ✅ Shipped | ❌ |
| 29 | Amharic (*አማርኛ*) | Ethiopic | SOV | ✅ Shipped | ❌ |
| 30 | Tibetan (*བོད་ཡིག*) | Tibetan | SOV | ✅ Shipped | ❌ |
| 31 | Cherokee (*ᏣᎳᎩ*) | Cherokee syllabary | SOV | ✅ Shipped | ❌ |
| 32 | Mongolian (*ᠮᠣᠩᠭᠣᠯ*) | Mongolian traditional | SOV | ✅ Shipped | ❌ |
| 33 | Armenian (*Հայերեն*) | Armenian | SOV | ✅ Shipped | ❌ |
| 34 | Georgian (*ქართული*) | Georgian Mkhedruli | SOV | ✅ Shipped | ❌ |
| 35 | Indonesian (*Bahasa Indonesia*) | Latin | SVO | ✅ Shipped | ❌ |
| 36 | Malay (*Bahasa Melayu*) | Latin | SVO | ✅ Shipped | ❌ |
| 37 | Filipino (*Tagalog*) | Latin | VSO | ✅ Shipped | ❌ |
| 38 | Swahili (*Kiswahili*) | Latin | SVO | ✅ Shipped | ❌ |
| 39 | Yoruba (*Èdè Yorùbá*) | Latin | SVO | ✅ Shipped | ❌ |
| 40 | Hausa | Latin | SVO | ✅ Shipped | ❌ |
| 62 | Mandarin Chinese (*中文*) | Han logograms | SVO | ✅ Shipped | ❌ |

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
