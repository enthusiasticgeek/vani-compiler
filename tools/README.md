# vāṇī tools

Out-of-tree utilities that do not ship with the compiler binary.

- [`vani_translate.py`](#vani_translatepy) — keyword translation between 57 languages with optional LLM translation of comments, strings, and identifiers
- [`llm_context/`](llm_context/README.md) — prompt-engineering bundle and MCP server for AI agents (Phase ML-2)

---

## `vani_translate.py`

**Current version**: B.1 v3  
**Status**: Production-ready keyword translation; LLM path requires an API key or local Ollama instance

A source-level translator for `.vani` files. It substitutes keywords, rewrites SOV word-order where needed, and optionally delegates natural-language content (comments, strings, identifiers) to an LLM.

---

### Quick start

```bash
# English → Hindi (keywords only, stdout)
python3 tools/vani_translate.py examples/language/english/basics.vani --to hindi

# English → Sanskrit, write to file, add auspicious header
python3 tools/vani_translate.py examples/language/english/basics.vani \
    --to sanskrit -o out.vani --add-sri-header

# Translate a whole directory tree in-place
python3 tools/vani_translate.py examples/language/english/ \
    --to marathi --batch -o examples/language/marathi/

# Translate and verify the round-trip is lossless
python3 tools/vani_translate.py basics.vani --to tamil --verify

# Print every keyword alias as a markdown table
python3 tools/vani_translate.py --list-keywords
```

---

### Supported languages

57 languages across 12 script families.

| Family | Languages |
|--------|-----------|
| Indo-Aryan (Devanagari) | English\*, Sanskrit, Hindi, Marathi |
| Indo-Aryan (other scripts) | Bengali, Odia, Gujarati, Punjabi, Sinhala |
| Dravidian | Tamil, Telugu, Kannada, Malayalam |
| East Asian | Mandarin, Japanese, Korean |
| Southeast Asian | Thai, Vietnamese, Khmer, Burmese, Lao, Malay, Indonesian, Filipino |
| Middle Eastern / RTL | Arabic, Hebrew, Persian, Urdu, Pashto |
| Cyrillic | Russian |
| European (non-Latin) | Greek |
| European (Latin) | Spanish, French, German, Portuguese, Italian, Dutch, Polish, Turkish, Swedish, Norwegian, Danish, Hungarian, Czech, Slovak, Finnish, Romanian, Catalan |
| Caucasian | Armenian, Georgian |
| African | Swahili, Yoruba, Hausa, Amharic |
| Other | Tibetan, Cherokee, Mongolian |

\* English is the canonical baseline; all other languages map to and from it.

Language names are used as-is on the CLI (`--to hindi`, `--to japanese`, etc.) and in the source pragma (`// vani-lang: hindi`).

---

### What is translated

#### Always (no LLM required)

| What | Example |
|------|---------|
| Reserved keywords | `fn` → `फलन` (Hindi), `return` → `返回` (Mandarin) |
| Boolean literals | `true` / `false` → script equivalents |
| Multi-word fusions | `के लिए` (Hindi for) → `for` |
| `// vani-lang:` pragma | updated to reflect the target language |

48 keyword token-kinds are covered: declarations (`fn`, `let`, `struct`, `enum`, `const`, `type`, `extern`, `intent`, `invariant`), visibility (`pub`, `module`, `use`, `as`), control flow (`return`, `if`, `else`, `while`, `for`, `in`, `from`, `to`, `break`, `continue`, `then`), references (`ref`, `mut`), matching (`match`), verification (`assert`, `prove`, `requires`, `ensures`), booleans and print (`true`, `false`, `print`), purity (`pure`, `parallel`, `reduce`, `with`), interfaces (`interface`, `implement`, `methods`), bounds (`where`, `is`), concurrency (`try`, `task`, `join`), and embedded (`unsafe`, `region`).

#### With `--llm` (requires API key or Ollama)

| What | Example |
|------|---------|
| Line comments `// …` | `// compute the sum` → `// योगफल की गणना करें` |
| String literals `"…"` | `"hello"` → `"नमस्ते"` |
| Identifiers (opt-in) | `safe_div` → `सुरक्षित_भाग` (with `--translate-identifiers`) |

Block comments `/* … */` and multi-line strings are not translated.

---

### SOV word-order rewriting

Languages with Subject-Object-Verb word order get verb-final statement shapes rewritten automatically — no flag needed. The translator works in both directions.

**Verb-final statements** (return, print, assert, prove):

```
English (SVO):   return total;        →  Hindi (SOV): total लौटाओ;
Hindi (SOV):     total लौटाओ;         →  English:     return total;
```

**For-range loops** (Hindi/Sanskrit/Marathi):

```
English:   for i from 0 to 10 {       →  Hindi:   i के लिए 0 से 10 तक {
Hindi:     i के लिए 0 से 10 तक {      →  English: for i from 0 to 10 {
```

SOV languages: Sanskrit, Hindi, Marathi, Bengali, Odia, Gujarati, Punjabi, Sinhala, Tamil, Telugu, Kannada, Malayalam, Japanese, Korean, Urdu, Persian, Pashto, Turkish, Mongolian, Tibetan.

---

### LLM backends

Pass `--llm BACKEND` to enable comment and string translation.

#### Anthropic

```bash
export ANTHROPIC_API_KEY=sk-ant-...
python3 tools/vani_translate.py file.vani --to hindi \
    --llm anthropic --llm-model claude-haiku-4-5-20251001
```

Requires `pip install 'anthropic>=0.20'`. Compatible with both the modern (`Anthropic`) and legacy (`Client`) SDK APIs.

#### OpenAI

```bash
export OPENAI_API_KEY=sk-...
python3 tools/vani_translate.py file.vani --to hindi \
    --llm openai --llm-model gpt-4o-mini
```

Requires `pip install openai`.

#### Ollama (local, no API key)

```bash
# Start Ollama with a model first:
ollama pull llama3.2

python3 tools/vani_translate.py file.vani --to hindi \
    --llm ollama --llm-model llama3.2 \
    --llm-timeout 120
```

Default host: `http://localhost:11434`. Override with `--ollama-host`.

#### Default models

| Backend | Default model |
|---------|---------------|
| `anthropic` | `claude-haiku-4-5-20251001` |
| `openai` | `gpt-4o-mini` |
| `ollama` | `llama3.2` |

Override any default with `--llm-model MODEL`.

---

### Identifier translation

When `--translate-identifiers` is given (requires `--llm`), user-defined identifiers are extracted, batched into a single LLM call, and substituted throughout the file.

```bash
python3 tools/vani_translate.py file.vani --to hindi \
    --llm anthropic --translate-identifiers
```

`camelCase` and `snake_case` names are split on word boundaries before sending to the LLM (`safe_div` → `"safe div"`) and re-joined in the target script's preferred style after translation.

---

### All CLI flags

```
positional:
  input                 source .vani file or directory (with --batch)

options:
  --from LANG           source language (auto-detected from pragma if omitted)
  --to LANG             target language (required)
  -o / --output PATH    output file or directory (default: stdout)
  --inplace / -i        translate in-place; saves original as <file>.bak
  --batch               translate all .vani files under INPUT directory tree
  --verify              translate back and check keyword tokens are preserved
  --list-keywords       print all keyword aliases as a markdown table and exit
  --add-sri-header      prepend // श्री। when targeting a Devanagari language

LLM options:
  --llm BACKEND         enable LLM translation (anthropic | openai | ollama)
  --llm-model MODEL     model name (see defaults above)
  --translate-identifiers  also translate user-defined identifiers via LLM
  --ollama-host URL     Ollama server URL (default: http://localhost:11434)
  --llm-timeout SECS    per-call timeout in seconds (default: 60)
```

---

### Source pragma

Every `.vani` file should declare its language in the first line so the translator can auto-detect the source:

```vani
// vani-lang: hindi
फलन main() -> i64 {
    0 लौटाओ;
}
```

Without a pragma, English is assumed. The translated file's pragma is updated automatically.

---

### Round-trip verification

`--verify` translates the file to the target language and then back, checking that the keyword token sequence is identical to the original. Identifiers, comments, strings, and whitespace are not compared — only structural tokens.

```bash
python3 tools/vani_translate.py basics.vani --to japanese --verify
# → round-trip ok: english -> japanese -> english (12 keyword tokens preserved)
```

Use this in CI to catch accidental keyword coverage gaps.

---

### Programmatic API

```python
from tools.vani_translate import translate, translate_with_llm, verify_roundtrip

# Keyword-only translation
hindi_src = translate(english_src, target_lang="hindi")

# With LLM (comments + strings)
hindi_src = translate_with_llm(
    english_src,
    target_lang="hindi",
    src_lang="english",
    llm="anthropic",
    model="claude-haiku-4-5-20251001",
    translate_identifiers=False,
    llm_timeout=60,
)

# Verify round-trip
ok, message = verify_roundtrip(source, target_lang="hindi", src_lang="english")
```

---

### Known limitations

| Limitation | Workaround |
|------------|------------|
| Block comments `/* … */` are not translated | Use `//` line comments |
| Multi-line string literals spanning >1 line are not translated | Keep string content on one line |
| Nested for-range SOV patterns only rewrite the outermost level | — |
| Identifier translation may mis-split domain-specific abbreviations | Review and fix after translation |
| Ollama models vary in quality; small models may produce garbled output | Use a 7B+ model; set `--llm-timeout 120` |

---

### Architecture notes

The translator is a pure-Python single-file tool (`tools/vani_translate.py`). It operates in three passes:

1. **Keyword substitution** — a character-level scan replaces every keyword token using a reverse-lookup table built from `ALIASES`. Multi-word fusions are handled by a one-token look-ahead.
2. **SOV rewriting** — for SOV target languages, verb-final statement lines are detected and reordered (SVO → SOV on output; SOV → SVO on input).
3. **LLM pass** (optional) — `//` comment text, quoted string content, and optionally identifiers are extracted and sent to the chosen LLM backend. On failure the original text is kept unchanged.

`_is_word_char()` recognises characters from all 27 supported Unicode script ranges so that non-ASCII keyword tokens are correctly delimited without a full Unicode segmenter.

---

### llm_context/

See [`llm_context/README.md`](llm_context/README.md) for the AI-agent bundle and MCP server (`mcp_server.py`).
