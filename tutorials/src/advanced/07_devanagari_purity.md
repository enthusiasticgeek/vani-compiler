# Advanced 7 -- Devanagari purity arc

> **Learning goal**: understand the `// vani-lang:` pragma's
> purity gate, how the lexer enforces per-file dialect choice,
> and how the SOV verb-at-end statement shapes desugar.

**Who this chapter is for**: developers who want to write or
maintain vāṇी programs in a Devanagari-script dialect (Sanskrit
/ Hindi / Marathi and their close relatives). You'll learn what
the pragma does, what the "purity gate" enforces (no mixing of
dialects within one file), and how Subject-Object-Verb word
order affects statement shape. Skip this chapter if you're only
writing English-keyword vāṇी -- nothing in the main tracks
depends on it.

## The pragma + the gate

Every `.vani` file optionally declares its dialect on the
first ~10 lines:

```vani
// श्री।                    (optional auspicious header)
// vani-lang: sanskrit
उद्देश्य "purity demo";
...
```

The lexer's `enforce_language_purity` pass (in
`src/lexer.rs`) walks the token stream and:

1. Classifies each structure-keyword token into a `Script`
   variant (`Latin`, `Devanagari`, `Bengali`, `Tamil`, `Telugu`,
   `Gujarati`, `Gurmukhi`, `Kannada`, `Malayalam`, `Odia`,
   `Sinhala`).
2. Tracks the **first observed non-Latin script** and rejects
   any later keyword from a different script.
3. Narrows further when a pragma is declared: the declared
   dialect's script must match the keywords' script. Within
   Devanagari, a Sanskrit-vs-Hindi-vs-Marathi sub-gate enforces
   per-dialect spellings (Sanskrit accepts `कार्य` but not
   the Hindi-specific `फलन`).

The gate fires the moment a second script shows up among the
structure keywords, even without a pragma -- a Devanagari `fn`
keyword locks the whole file to Devanagari:

<img class="manas" src="../images/mascot/manas_mascot_error.png" title="this code does not compile!"/>

```vani
कार्य main() -> i64 {
  return 0;
}
```

`कार्य` (Devanagari `fn`) sets the file's script; `return` is a
Latin/English keyword, so the lexer rejects it with `language
mismatch: file already used a Devanagari structure keyword ...
can't switch to a English alias mid-file. Pick one script per
file.` -- caught before parsing even starts.

## The SOV statement shapes

Sanskrit (with optional support in Hindi/Marathi) lets you
write verb-at-end (SOV) forms. The shapes that ship today:

| SOV shape | Desugars to |
|---|---|
| `<expr> लिख;` | `print <expr>;` |
| `<expr> पुनरागम;` | `return <expr>;` |
| `<expr> सिद्धम्;` | `assert <expr>;` |
| `<expr> प्रमाण;` | `prove <expr>;` |
| `<name>: <type> = <init> माना;` | `let <name>: <type> = <init>;` |
| `<cond> यदि { ... } अन्यथा { ... }` | `if <cond> { ... } else { ... }` |
| `<cond> यावत् { ... }` | `while <cond> { ... }` |

`fn` / `struct` / `enum` / top-level decls are keyword-first
only in v1 -- no SOV path for those yet.

## What's automatic in a Devanagari-pragma file

These work without you doing anything special:

- **Integer print** emits Devanagari numerals (`०..९`) via the
  per-script helper (`intent_print_int_dev` for tree-C,
  `@intent_print_int_dev` in LLVM IR). Phase 1.1.
- **Error labels** render in the matching script (`त्रुटिः` for
  Sanskrit, `त्रुटि` for Hindi, `चूक` for Marathi, etc.).
- **Identifiers** can be Devanagari letters -- the LLVM
  backend mangles non-ASCII codepoints via `_uHHHH` because LLVM
  IR identifier grammar forbids non-ASCII; the C backend uses
  the bytes directly.

## The dialect roster

| Script | Dialects |
|---|---|
| Latin | English |
| Devanagari | Sanskrit, Hindi, Marathi, Nepali, Maithili, Konkani |
| Bengali | Bengali, Assamese (shares script) |
| Tamil | Tamil |
| Telugu | Telugu |
| Gujarati | Gujarati |
| Gurmukhi | Punjabi |
| Kannada | Kannada |
| Malayalam | Malayalam |
| Odia | Odia |
| Sinhala | Sinhala |
| Perso-Arabic (RTL) | Urdu, Sindhi, Punjabi-Shahmukhi, Persian, Pashto (Phase 12.x) |

**21 dialects across 11 scripts** as of Phase 12.5 (2026-06-07).
Within Perso-Arabic, two distinct numeral blocks are wired --
Eastern Arabic-Indic ٠..٩ (Urdu/Sindhi/Shahmukhi) and Persian
۰..۹ (Persian/Pashto).

## Natural-everyday vs. formal-tatsama keywords

A 2026-06-07 audit across all 10 Indic dialects layered
**natural everyday** spellings alongside the Sanskrit-rooted
tatsama forms. Both registers compile -- pick whichever reads
more naturally for your file:

| Dialect | True (formal / everyday) | False (formal / everyday) |
|---|---|---|
| Marathi | `सत्य` / `बरोबर`, `खरे` | `असत्य` / `खोटे`, `चूक` |
| Hindi | `सत्य`, `सही` / `सच` | `असत्य`, `अशुद्ध` / `झूठ`, `गलत` |
| Bengali | `সত্য` / `ঠিক` | `অসত্য` / `মিথ্যা`, `ভুল` |
| Kannada | `ಸತ್ಯ` / `ಸರಿ` | `ಸುಳ್ಳು` / `ತಪ್ಪು` |
| Malayalam | `സത്യം` / `ശരി` | `അസത്യം` / `തെറ്റ്` |
| Sinhala | `සත්‍ය` / `හරි` | `අසත්‍ය` / `වැරදි` |

Sanskrit gained a classical Match form: `मेलन` (melana, deverbal
noun) alongside the colloquial `मेल`.

**Marathi-specific note**: `सही` means "signature" (noun) in
Marathi and `अशुद्ध` strictly means "impure"; both are
Hindi-only as bool literals. Marathi's `बदल` is the noun
"change" -- the proper mutable adjective is `बदलणारा`. And
Marathi conjugates `print` from `लिह्-` (`लिहा` / `लिही` /
`लिहिया`), not the Hindi `लिख्-` root.

Bool literals stay outside the per-file purity gate by design,
so adding spellings never breaks old files.

## A worked Sanskrit example

```vani
// श्री।
// vani-lang: sanskrit
उद्देश्य "factorial demo";

कार्य factorial(n: i64) -> i64
अपेक्षित n >= 0;
अपेक्षित n <= 10;
{
  यदि n <= 1 {
    पुनरागम 1;
  } अन्यथा {
    पुनरागम n * factorial(n - 1);
  }
}

कार्य main() -> i64 {
  माना x: i64 = factorial(5);
  सिद्धम् x == १२०;
  लिख x;
  पुनरागम 0;
}
```

Prints `१२०`. Same backend output as the English-keyword
version, but the source reads like Sanskrit.

## Per-dialect error rendering

`src/diagnostic.rs` keeps a small prefix-translation table
per dialect. The Bengali table, for example:

```
"expected " -> "প্রত্যাশিত "
"unknown variable" -> "অজানা চলক (unknown variable)"
"type mismatch" -> "প্রকার অমিল (type mismatch)"
```

Add to these as users report rough patches. Sanskrit / Hindi /
Marathi / Bengali / Tamil / Telugu / Gujarati / Punjabi /
Kannada / Malayalam / Odia / Sinhala each have a starter
table; Nepali / Maithili / Konkani route through their nearest
Devanagari neighbor.

## Challenge

Read `examples/language/sanskrit/pure_devanagari.vani`.
Translate it by hand to Bengali (using the existing Bengali
keyword table). Verify both files produce identical output
on both backends.

---

**Previous**: [Sec.6 -- SMT trace debugging ->](06_smt_debug.md)
**Next**: [Sec.8 -- Cross-language translator extension ->](08_translator.md)
