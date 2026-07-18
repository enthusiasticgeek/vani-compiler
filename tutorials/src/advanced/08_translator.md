# Advanced 8 -- The cross-language translator

> **Learning goal**: understand how `vani_translate.py` works, use it to
> move a program between two of its 57 supported languages, add an LLM
> backend for natural-language content, and extend the keyword table with
> a new entry.

**Who this chapter is for**: anyone who wants to translate existing
`.vani` sources between human languages — for localisation, teaching, or
reading a teammate's code — and contributors who want to extend the
keyword table or plug in a new language. The translator is a standalone
Python script; you do not need to understand compiler internals.

---

## What the translator does

`tools/vani_translate.py` rewrites a `.vani` source file in three passes:

| Pass | What changes | Requires |
|------|-------------|----------|
| 1 — keyword substitution | All 48 reserved keywords, `true`/`false`, the `// vani-lang:` pragma | Always on |
| 2 — SOV word-order rewrite | Verb-final statements (`return`, `print`, `assert`, `prove`) and Hindi/Sanskrit/Marathi for-range loops | Automatic for SOV target languages |
| 3 — LLM translation | `// line comments`, `"string literals"`, and optionally user-defined identifiers | `--llm BACKEND` flag |

Everything else — operator tokens, numeric literals, whitespace, block
comments `/* */` — is forwarded unchanged.

---

## Supported languages

57 languages across 12 script families. Use the exact lowercase name on
the CLI and in `// vani-lang:` pragmas.

```
english     sanskrit    hindi       marathi
bengali     odia        gujarati    punjabi     sinhala
tamil       telugu      kannada     malayalam
mandarin    japanese    korean
thai        vietnamese  khmer       burmese     lao
malay       indonesian  filipino
arabic      hebrew      persian     urdu        pashto
russian
greek
spanish     french      german      portuguese  italian
dutch       polish      turkish     swedish     norwegian
danish      hungarian   czech       slovak      finnish
romanian    catalan
armenian    georgian
swahili     yoruba      hausa       amharic
tibetan     cherokee    mongolian
```

---

## Pass 1: keyword translation

Every keyword is substituted using a bidirectional lookup table
(`ALIASES` in `vani_translate.py`). The `--from` flag is optional —
the translator auto-detects every known keyword regardless of source
language. Translation between any two of the 57 languages works directly,
not only through English.

```bash
# English → Japanese
python3 tools/vani_translate.py basics.vani --to japanese

# Japanese → Spanish (no --from needed)
python3 tools/vani_translate.py basics_ja.vani --to spanish

# Any language → English → round-trip back
python3 tools/vani_translate.py hindi_src.vani --to english --verify
```

A minimal round-trip:

```vani
// vani-lang: english
fn add(a: i64, b: i64) -> i64 {
    return a + b;
}
```

After `--to russian`:

```vani
// vani-lang: russian
функция add(a: i64, b: i64) -> i64 {
    вернуть a + b;
}
```

After `--to arabic` from the Russian file:

```vani
// vani-lang: arabic
دالة add(a: i64, b: i64) -> i64 {
    أرجع a + b;
}
```

### Inspecting the alias table

```bash
# Print every keyword alias as a markdown table (all 57 languages)
python3 tools/vani_translate.py --list-keywords
```

---

## Pass 2: SOV word-order rewriting

Twenty languages in the translator use Subject-Object-Verb word order.
Verb-final statement shapes are rewritten automatically — no flag needed.

**Verb-final statements**:

```
English (SVO)           Hindi (SOV)
─────────────────────   ─────────────────────
return total;      →    total लौटाओ;
print x;           →    x लिखो;
assert b != 0;     →    b != 0 सुनिश्चित;
prove n >= 0;      →    n >= 0 सिद्ध करो;
```

**For-range loops** (Devanagari family):

```
English                              Hindi
──────────────────────────────────   ─────────────────────────────────
for i from 0 to n {            →    i के लिए 0 से n तक {
```

Both directions work. Translating a Hindi-keyword file back to English
restores `return` / `print` at the start of the line and the `for … from … to`
shape.

SOV languages in the translator: Sanskrit, Hindi, Marathi, Bengali, Odia,
Gujarati, Punjabi, Sinhala, Tamil, Telugu, Kannada, Malayalam, Japanese,
Korean, Urdu, Persian, Pashto, Turkish, Mongolian, Tibetan.

---

## Pass 3: LLM translation of comments, strings, and identifiers

Without `--llm`, comments and strings pass through unchanged:

```vani
// compute the factorial            ← stays as-is
print "hello";                      ← stays as-is
```

Add `--llm BACKEND` to translate natural-language content too.

### Anthropic

```bash
export ANTHROPIC_API_KEY=sk-ant-...
python3 tools/vani_translate.py basics.vani --to hindi \
    --llm anthropic --llm-model claude-haiku-4-5-20251001
```

Install: `pip install 'anthropic>=0.20'`

### OpenAI

```bash
export OPENAI_API_KEY=sk-...
python3 tools/vani_translate.py basics.vani --to hindi \
    --llm openai --llm-model gpt-4o-mini
```

Install: `pip install openai`

### Ollama (local, no API key)

```bash
ollama pull llama3.2

python3 tools/vani_translate.py basics.vani --to hindi \
    --llm ollama --llm-model llama3.2 --llm-timeout 120
```

### What LLM translation looks like

Source (English):

```vani
// vani-lang: english
// Compute the factorial of n recursively.
fn factorial(n: i64) -> i64 {
    if n <= 1 {
        return 1;
    }
    return n * factorial(n - 1);
}

fn main() -> i64 {
    print "result:";
    print factorial(5);
    return 0;
}
```

After `--to hindi --llm anthropic`:

```vani
// vani-lang: hindi
// n का गुणनखंड पुनरावर्ती रूप से गणना करें।
फलन factorial(n: i64) -> i64 {
    अगर n <= 1 {
        1 लौटाओ;
    }
    n * factorial(n - 1) लौटाओ;
}

फलन main() -> i64 {
    "परिणाम:" लिखो;
    factorial(5) लिखो;
    0 लौटाओ;
}
```

Keywords: substituted (pass 1). SOV word-order: rewritten (pass 2).
Comment and string: translated by the LLM (pass 3).

### Identifier translation (optional)

User-defined names like `factorial`, `safe_div`, `total_count` are not
changed by default. Add `--translate-identifiers` to translate them too:

```bash
python3 tools/vani_translate.py basics.vani --to hindi \
    --llm anthropic --translate-identifiers
```

All unique identifiers are batched into a single LLM call.
`camelCase` and `snake_case` names are split on word boundaries before
sending (`safe_div` → "safe div") and re-joined after translation.

---

## Batch mode and in-place editing

```bash
# Translate every .vani file in a directory tree
python3 tools/vani_translate.py examples/language/english/ \
    --to tamil --batch -o examples/language/tamil/

# Edit a file in-place (original saved as .bak)
python3 tools/vani_translate.py myfile.vani --to korean --inplace
```

---

## Round-trip verification

`--verify` runs the translation, then translates back and checks that the
keyword token sequence is identical to the original. Use it in CI to
catch coverage gaps.

```bash
python3 tools/vani_translate.py basics.vani --to japanese --verify
# → round-trip ok: english -> japanese -> english (12 keyword tokens preserved)
```

The round-trip guarantee covers keywords only. Comments, strings, and
identifiers are not compared.

---

## Adding a keyword to the alias table

Suppose you want to add a Korean alias for `pure` that the translator
currently falls back to English for. Open `tools/vani_translate.py`,
find the `Pure` entry, and add the `korean` key:

```python
"Pure": {
    "english": "pure",
    "sanskrit": "शुद्ध",  "hindi": "शुद्ध",   "marathi": "शुद्ध",
    "mandarin": "纯",      "japanese": "純粋",   "korean": "순수",    # ← added
    "russian":  "чистый",
    ...
},
```

Verify with the round-trip:

```bash
# Write a file that uses pure
cat > /tmp/test_pure.vani << 'EOF'
// vani-lang: english
pure fn square(n: i64) -> i64 { return n * n; }
fn main() -> i64 { print square(3); return 0; }
EOF

python3 tools/vani_translate.py /tmp/test_pure.vani --to korean --verify
# → round-trip ok: english -> korean -> english (N keyword tokens preserved)
```

That's it. The reverse-lookup table is rebuilt from `ALIASES` at startup,
so no other change is needed.

---

## Adding a new language to the translator

Adding a language to the **translator only** (not to the compiler's lexer)
is a three-step change to `tools/vani_translate.py`:

1. **Add entries** to every relevant row of `ALIASES`. At minimum cover
   the ~30 most-common token kinds (`Fn`, `Let`, `Return`, `If`, `Else`,
   `While`, `For`, `Break`, `Continue`, `Assert`, `Prove`, `True`,
   `False`, `Print`, `Match`, `Struct`, `Enum`, `Const`, `Pub`, `Module`,
   `Use`, `Interface`, `Implement`, `Try`, `Task`, `Join`, `Unsafe`,
   `RegionKw`, `Intent`, `Type`). Leave uncommon ones out — the
   translator falls back to English for any missing entry.

2. **Add the language name** to `SUPPORTED_LANGS`.

3. **Add to `SOV_LANGS`** if the language uses Subject-Object-Verb word
   order (the translator will then automatically rewrite verb-final
   statement shapes).

```python
# Step 2
SUPPORTED_LANGS = (
    ...,
    "assamese",    # ← new
)

# Step 3 (Assamese is SOV)
SOV_LANGS = frozenset({
    ...,
    "assamese",
})
```

The Unicode character range for the new script also needs a line in
`_is_word_char()` if it is not already covered (Assamese reuses the
Bengali block U+0980–U+09FF, which is already there).

To add a language to the **compiler itself** (so it can parse and compile
`.vani` files in that language), see [Section 9 →](09_new_dialect.md).

---

## Programmatic API

```python
from tools.vani_translate import translate, translate_with_llm, verify_roundtrip

# Keywords only
out = translate(source, target_lang="japanese")

# Keywords + LLM comments + strings
out = translate_with_llm(
    source,
    target_lang="hindi",
    src_lang="english",
    llm="anthropic",
    model="claude-haiku-4-5-20251001",
    translate_identifiers=False,
    llm_timeout=60,
)

# Verify round-trip
ok, msg = verify_roundtrip(source, target_lang="korean", src_lang="english")
print(msg)
```

---

## Known limitations

| Limitation | Workaround |
|---|---|
| Block comments `/* … */` are not LLM-translated | Use `//` comments |
| Multi-line string literals are not translated | Keep strings on one line |
| Nested for-range SOV patterns: only the outermost loop is rewritten | — |
| Ollama quality varies by model size | Use a 7B+ model; set `--llm-timeout 120` |

---

## Challenge

1. Pick any worked example from the Beginner track. Translate it to two
   languages from different script families (e.g. Japanese and Arabic)
   and run `--verify` on both. Inspect the output in a Unicode-capable
   editor.

2. Add one keyword entry to a language that is missing it (find gaps by
   running `--list-keywords` and looking for `(missing)` or English
   fallbacks). Run the round-trip test.

3. *(Advanced)* Run the translator with `--llm anthropic --translate-identifiers`
   on a file that has descriptive function names. Review whether the
   translated identifiers still reflect the original meaning.

---

**Previous**: [Sec.7 -- Devanagari purity arc ->](07_devanagari_purity.md)
**Next**: [Sec.9 -- Adding a new dialect (compiler-level) →](09_new_dialect.md)
