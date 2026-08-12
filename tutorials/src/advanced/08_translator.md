# Advanced 8 -- The cross-language translator

> **Learning goal**: understand how `vani_translate.py` works, use it to
> move a program between two of its 63 supported languages, add an LLM
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

63 languages across 12 script families -- the full set the compiler
itself accepts via `// vani-lang:` pragmas. Use the exact lowercase
name on the CLI and in pragmas.

```
english     sanskrit    hindi       marathi
nepali      maithili    konkani
bengali     assamese    odia        gujarati    punjabi     sinhala
tamil       telugu      kannada     malayalam
mandarin    japanese    korean
thai        vietnamese  khmer       burmese     lao
malay       indonesian  filipino
arabic      hebrew      persian     urdu        sindhi
punjabi_shahmukhi       pashto
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

Six of these (`nepali`, `maithili`, `konkani`, `assamese`, `sindhi`,
`punjabi_shahmukhi`) are pragma-only dialects that reuse an existing
shared keyword table -- same as the compiler itself treats them (see
`docs/languages.md`). Translating *into* one of them produces the same
keyword spellings as its parent (Hindi, Bengali, and Urdu
respectively), just under that dialect's own pragma tag.
`punjabi_shahmukhi` is the one exception to "CLI name == pragma tag":
the compiler's pragma parser expects the hyphenated
`// vani-lang: punjabi-shahmukhi`, which the translator writes
automatically -- you still pass `--to punjabi_shahmukhi` (underscore)
on the command line.

---

## Pass 1: keyword translation

Every keyword is substituted using a bidirectional lookup table
(`ALIASES` for output spellings, `ALL_SYNONYMS` for input recognition
-- see "Adding a keyword" below for why there are two). The `--from`
flag is optional — the translator auto-detects every known keyword
regardless of source language. Translation between any two of the 63
languages works directly, not only through English.

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
# Print every keyword alias as a markdown table
python3 tools/vani_translate.py --list-keywords
```

The table only has columns for English, Sanskrit, Hindi, Marathi, and
Mandarin -- `list_keywords()` in `tools/vani_translate.py` hardcodes
those five as a representative sample; it does NOT show all 63 (a full
63-column table would be unreadable as a terminal/markdown table). A
missing entry in one of those five columns prints as a bare `--`, not
`(missing)`. To check coverage for any of the other 58 languages, read
the `ALIASES` dict in `tools/vani_translate.py` directly, run
`python3 tools/regen_vani_translate_keywords.py --check` (reports every
cell that's wrong OR entirely missing, validated directly against
`src/lexer.rs` -- see "Adding a keyword" below), or translate a file
that exercises the keyword you care about and inspect the output by
eye.

---

## Pass 2: SOV word-order rewriting

Twenty-six languages in the translator use Subject-Object-Verb word
order. Verb-final statement shapes are rewritten automatically — no
flag needed.

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
for idx from 0 to n {          →    idx के लिए 0 से n तक {
```

(Loop variable spelled `idx`, not a short common-word-shaped name --
some short identifiers coincidentally collide with another language's
keyword spelling in the translator's global cross-language lookup
table and get silently mangled. See "Known limitations" below.)

Both directions work. Translating a Hindi-keyword file back to English
restores `return` / `print` at the start of the line and the `for … from … to`
shape.

SOV languages in the translator: Sanskrit, Hindi, Marathi, Nepali,
Maithili, Konkani, Bengali, Assamese, Odia, Gujarati, Punjabi, Sinhala,
Tamil, Telugu, Kannada, Malayalam, Japanese, Korean, Urdu, Sindhi,
Punjabi-Shahmukhi, Persian, Pashto, Turkish, Mongolian, Tibetan.

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
# → round-trip ok: english -> japanese -> english (21 keyword tokens
#   preserved, both hops compile clean)
# (exact count drifts as example files are edited -- verified against
# examples/language/english/basics.vani as of 2026-08-12; what matters
# is the "round-trip ok" message, not the specific number)
```

<img class="manas" src="../images/mascot/manas_mascot_caution.png" title="this code needs extra care"/>

The round-trip guarantee covers keywords only. Comments, strings, and
identifiers are not compared. When a `vanic` binary can be found (on
`PATH`, or this repo's own `target/release`/`target/debug`), `--verify`
*additionally* compiles both translation hops with `vanic check` and
folds that into the pass/fail verdict — a real syntax problem in
either hop fails verification even if the keyword-token sequence
happens to still match. (Token-sequence equality alone has a real
blind spot: an untranslated keyword that silently passes through
round-trips to itself unchanged, which used to make a genuinely broken
translation report "ok" — confirmed and fixed 2026-08-12.)

---

## Adding a keyword to the alias table

**Don't hand-edit `ALIASES`.** That used to be the documented workflow
here, and it's exactly how the table drifted badly enough (2026-08-12)
that translating several real dialects silently produced output that
didn't compile -- the same "hand-copied table quietly rots relative to
`src/lexer.rs`" shape as an earlier bug in the LSP's completion lists.
`src/lexer.rs` is the actual source of truth for every keyword spelling
the compiler accepts; `tools/vani_translate.py`'s tables are generated
from it by `tools/regen_vani_translate_keywords.py`.

If the language you want already has the word in `lexer.rs` (just
missing from `ALIASES`), fixing it is one command:

```bash
python3 tools/regen_vani_translate_keywords.py --check   # see what's stale/missing
python3 tools/regen_vani_translate_keywords.py           # fix it
```

If `lexer.rs` genuinely has no word for that TokenKind in that
language's dialect function yet, add it there first (find the right
`fn xxx_keyword(text: &str) -> Option<TokenKind>` function for the
language -- see [Section 9](09_new_dialect.md) for how the dialect
functions are organized), rebuild `vanic`, and re-run the regen script
above; it will pick up the new entry automatically.

Verify with the round-trip:

```bash
cat > /tmp/test_pure.vani << 'EOF'
// vani-lang: english
pure fn square(n: i64) -> i64 { return n * n; }
fn main() -> i64 { print square(3); return 0; }
EOF

python3 tools/vani_translate.py /tmp/test_pure.vani --to korean --verify
# → round-trip ok: english -> korean -> english (N keyword tokens preserved)
```

Or, better, run the whole regression suite (`tools/test_vani_translate.py`
-- the same one CI runs) before and after your change; it exercises
every dialect in both directions plus `--verify`, not just the one you
touched.

---

## Adding a new language to the translator

Every dialect `src/lexer.rs` accepts should also work in the
translator -- as of this writing all 63 do (verified by
`tools/test_vani_translate.py`, wired into CI). If you're adding a
language the **compiler** doesn't support yet, do that first (see
[Section 9 →](09_new_dialect.md)) -- once `vanic` itself can parse the
new dialect, wiring it into the translator is a small addition to
`tools/regen_vani_translate_keywords.py`, not `vani_translate.py`
directly:

1. **Add an entry to `LANG_TABLES`**: the new language's key, the
   `lexer.rs` function(s) supplying its keyword table (in preference
   order -- native-script form before its ASCII-only pragma-gated
   counterpart, if it has both), and the exact `// vani-lang: <tag>`
   pragma string the compiler's pragma parser expects (almost always
   identical to the language key -- see `punjabi_shahmukhi`'s entry
   for the one exception, a hyphen-vs-underscore mismatch handled by
   `_PRAGMA_TAG_OVERRIDES`).
2. If the new dialect is a pragma-only alias of an existing shared
   table (like Nepali/Maithili/Konkani reusing the Devanagari table,
   or Assamese reusing Bengali's), also add it to `ALIAS_OF` so its
   column gets filled in as a straight copy of the parent's
   already-curated words.
3. Run `python3 tools/regen_vani_translate_keywords.py` -- it adds the
   language to `SUPPORTED_LANGS`, `ALIASES`, and `ALL_SYNONYMS`
   automatically. Add the new language to `SOV_LANGS` by hand if it
   uses Subject-Object-Verb word order (the regen script doesn't infer
   grammar).
4. The Unicode character range for a genuinely new script also needs a
   line in `_is_word_char()` if `lexer.rs`'s own dialect functions
   introduced one `vani_translate.py` doesn't already recognize.

Run `python3 tools/test_vani_translate.py` afterward to confirm the new
language round-trips and compiles both directions.

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
| **User identifiers that happen to match another supported language's keyword spelling get silently mangled** — confirmed by testing (2026-08-12): a parameter named `com` collides with Portuguese's spelling of `With` and becomes `सह` under any target language, even though the file is English/Hindi and never mentions Portuguese. Pass 1's keyword matching is deliberately global across all 63 languages (that's what makes `--from` auto-detection work at all -- see "Pass 1" above), and now also matches every synonym in `ALL_SYNONYMS`, not just each language's one canonical spelling -- broader recognition on purpose (it's what fixed the "some real dialect spelling silently passes through untranslated" bug this table used to describe a symptom of), but it widens this specific collision surface too. | Avoid short, common-word-shaped identifiers (`com`, `de`, `is`, `to`, `as`, ...); `--verify` only checks the keyword token sequence, so it will NOT catch a mangled identifier -- inspect translated output by eye when it uses short names |

---

## Challenge

1. Pick any worked example from the Beginner track. Translate it to two
   languages from different script families (e.g. Japanese and Arabic)
   and run `--verify` on both. Inspect the output in a Unicode-capable
   editor.

2. Pick a `TokenKind` and a language, and use
   `tools/regen_vani_translate_keywords.py --check` to see whether its
   `ALIASES` cell is stale or missing. If none are (the table should
   be clean as of this writing), pick a keyword `lexer.rs` doesn't yet
   have a word for in some dialect, add one there, and run the regen
   script to pick it up. Run the round-trip test either way.

3. *(Advanced)* Run the translator with `--llm anthropic --translate-identifiers`
   on a file that has descriptive function names. Review whether the
   translated identifiers still reflect the original meaning.

---

**Previous**: [Sec.7 -- Devanagari purity arc ->](07_devanagari_purity.md)
**Next**: [Sec.9 -- Adding a new dialect (compiler-level) →](09_new_dialect.md)
