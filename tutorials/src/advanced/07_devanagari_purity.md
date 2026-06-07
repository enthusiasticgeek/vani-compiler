# Advanced 7 — Devanagari purity arc

> **Learning goal**: understand the `// vani-lang:` pragma's
> purity gate, how the lexer enforces per-file dialect choice,
> and how the SOV verb-at-end statement shapes desugar.

## The pragma + the gate

Every `.vani` file optionally declares its dialect on the
first ~10 lines:

```rust
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
| `<cond> यदि { … } अन्यथा { … }` | `if <cond> { … } else { … }` |
| `<cond> यावत् { … }` | `while <cond> { … }` |

`fn` / `struct` / `enum` / top-level decls are keyword-first
only in v1 — no SOV path for those yet.

## What's automatic in a Devanagari-pragma file

These work without you doing anything special:

- **Integer print** emits Devanagari numerals (`०..९`) via the
  per-script helper (`intent_print_int_dev` for tree-C,
  `@intent_print_int_dev` in LLVM IR). Phase 1.1.
- **Error labels** render in the matching script (`त्रुटिः` for
  Sanskrit, `त्रुटि` for Hindi, `चूक` for Marathi, etc.).
- **Identifiers** can be Devanagari letters — the LLVM
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

**16 dialects across 10 scripts** as of Phase 6 (2026-06-07).

## A worked Sanskrit example

```rust
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
"expected " → "প্রত্যাশিত "
"unknown variable" → "অজানা চলক (unknown variable)"
"type mismatch" → "প্রকার অমিল (type mismatch)"
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

**Next**: [§8 — Cross-language translator extension →](08_translator.md)
