# vāṇī tools

Out-of-tree utilities that do not ship with the compiler binary.

- [`vani_translate.py`](#vani_translatepy) — keyword translation between 63 languages with optional LLM translation of comments, strings, and identifiers
- [`regen_vani_translate_keywords.py`](#regen_vani_translate_keywordspy) — regenerates `vani_translate.py`'s keyword tables from `src/lexer.rs`; run this after any lexer.rs keyword edit
- [`test_vani_translate.py`](#test_vani_translatepy) — regression suite for `vani_translate.py`, run in CI
- [`leak_sweep.py`](#leak_sweeppy) — ASan + LeakSanitizer + UBSan sweep over the example corpus, run in CI
- [`install-cross-qemu.sh`](#install-cross-qemush) — set up AArch64 / RISC-V 64 QEMU user-mode emulation + cross-compilers for local `--target=` testing
- [`llm_context/`](llm_context/README.md) — prompt-engineering bundle and MCP server for AI agents (Phase ML-2)

---

## `vani_translate.py`

**Current version**: B.1 v3  
**Status**: Production-ready keyword translation; LLM path requires an API key or local Ollama instance

Keyword data (`ALIASES`, `ALL_SYNONYMS`) is generated from `src/lexer.rs` by
[`regen_vani_translate_keywords.py`](#regen_vani_translate_keywordspy) — don't
hand-edit those tables directly, the same way `tools/regen_lsp_keywords.py`
keeps `src/lsp.rs`'s completion lists in sync with `lexer.rs`. (2026-08-12:
the tables had drifted badly enough from `lexer.rs` that translating several
real dialects produced output that didn't compile — 6 dialects were also
missing from `--to` entirely. Both fixed the same day;
[`test_vani_translate.py`](#test_vani_translatepy) is the permanent
regression guard, wired into CI.)

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

63 languages across 12 script families — the full set the compiler itself
accepts via `// vani-lang:` pragmas.

| Family | Languages |
|--------|-----------|
| Indo-Aryan (Devanagari) | English\*, Sanskrit, Hindi, Marathi, Nepali, Maithili, Konkani |
| Indo-Aryan (other scripts) | Bengali, Assamese, Odia, Gujarati, Punjabi, Sinhala |
| Dravidian | Tamil, Telugu, Kannada, Malayalam |
| East Asian | Mandarin, Japanese, Korean |
| Southeast Asian | Thai, Vietnamese, Khmer, Burmese, Lao, Malay, Indonesian, Filipino |
| Middle Eastern / RTL | Arabic, Hebrew, Persian, Urdu, Sindhi, Punjabi-Shahmukhi (`--to punjabi_shahmukhi`), Pashto |
| Cyrillic | Russian |
| European (non-Latin) | Greek |
| European (Latin) | Spanish, French, German, Portuguese, Italian, Dutch, Polish, Turkish, Swedish, Norwegian, Danish, Hungarian, Czech, Slovak, Finnish, Romanian, Catalan |
| Caucasian | Armenian, Georgian |
| African | Swahili, Yoruba, Hausa, Amharic |
| Other | Tibetan, Cherokee, Mongolian |

\* English is the canonical baseline; all other languages map to and from it.

Nepali, Maithili, Konkani, Assamese, Sindhi, and Punjabi-Shahmukhi are
pragma-only dialects that reuse an existing shared keyword table (same as
the compiler itself treats them — see `docs/language_manual.md`'s
Multilingual keywords section) rather than having their own translated
vocabulary; translating *into* one of them produces the same keyword
spellings as its parent (Hindi, Bengali, and Urdu respectively), just under
that dialect's own pragma tag.

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

1. **Keyword substitution** — a character-level scan replaces every keyword token using a reverse-lookup table seeded from `ALL_SYNONYMS` (every spelling `lexer.rs` actually accepts for a given TokenKind, across every dialect) and `ALIASES` (the single canonical spelling used for output). Multi-word fusions are handled by a one-token look-ahead.
2. **SOV rewriting** — for SOV target languages, verb-final statement lines are detected and reordered (SVO → SOV on output; SOV → SVO on input).
3. **LLM pass** (optional) — `//` comment text, quoted string content, and optionally identifiers are extracted and sent to the chosen LLM backend. On failure the original text is kept unchanged.

`_is_word_char()` recognises characters from all 27 supported Unicode script ranges so that non-ASCII keyword tokens are correctly delimited without a full Unicode segmenter.

---

## `regen_vani_translate_keywords.py`

**Status**: Maintenance tool — run after any keyword-table edit in `src/lexer.rs`

Regenerates `vani_translate.py`'s `ALIASES` and `ALL_SYNONYMS` tables directly
from `src/lexer.rs`'s real keyword-matching functions, instead of leaving
them hand-maintained (the shape of bug that caused BUG-173 in `src/lsp.rs`
and the 2026-08-12 `vani_translate.py` staleness this script exists to
prevent recurring).

```bash
# Report drift without writing (used by CI + test_vani_translate.py)
python3 tools/regen_vani_translate_keywords.py --check

# Fix ALIASES cells that no longer match lexer.rs, add any language
# missing from ALIASES entirely, and regenerate ALL_SYNONYMS
python3 tools/regen_vani_translate_keywords.py
```

It validates every `ALIASES[TokenKind][language]` cell against the real
`lexer.rs` function(s) that language's keywords come from (`LANG_TABLES`),
backfills any cell that's simply missing (a language never got added for
some TokenKind), and adds the 6 dialects that reuse an existing shared
table wholesale (`ALIAS_OF`) if they're absent. It does **not** invent a
translation for a TokenKind `lexer.rs` genuinely has no word for in some
language — those stay as documented gaps (2 known cases: Cherokee and
Mongolian have no native `as`-cast keyword yet).

---

## `test_vani_translate.py`

**Status**: Regression suite — run in CI

Guards against `vani_translate.py` regressing the way it did on
2026-08-12. Needs a built `vanic` binary to run its compile-check steps
(`cargo build --release --bin vanic` first); falls back to just the
table-staleness check otherwise.

```bash
python3 tools/test_vani_translate.py
```

Checks: `regen_vani_translate_keywords.py --check` passes; every dialect's
own real example file translates to English and compiles; English's
`basics.vani` translates into every dialect and compiles; `--verify`'s
round-trip (including its dual-hop `vanic check` compile-check) passes for
every dialect.

---

## `leak_sweep.py`

**Status**: Production; runs in CI on every push/PR to `main` (the `leak-sweep` job in `.github/workflows/ci.yml`)

Compiles every `.vani` file under `examples/` that passes `vanic check` to C, builds it with `gcc -fsanitize=address,leak,undefined -fno-sanitize-recover=all` (the same flags `vanic run --backend=c` itself uses), runs it, and classifies any AddressSanitizer / LeakSanitizer / UndefinedBehaviorSanitizer report. Added after round 8's bug-pattern audit (2026-08-09) found this kind of systematic sweep had never existed and turned up two real bugs (one a genuine heap-use-after-free) on its first pass.

```bash
# Build vanic first, then sweep the corpus against the checked-in baseline.
# Exits 0 if every finding matches the baseline exactly, 1 otherwise.
cargo build --release --bin vanic
python3 tools/leak_sweep.py
```

Some findings are already-triaged (a methodology false positive, or a real bug deliberately left open with a documented reason) rather than new regressions — those live in `tools/leak_sweep_baseline.json` with a `reason` field, so the sweep only fails CI on a genuinely NEW finding. If you fix one of the baselined bugs, remove its entry; if the sweep flags something new, read `tools/leak_sweep_baseline.json`'s existing entries first (particularly the BUG-157 async-cluster entries) before assuming it's the same known issue.

To regenerate the baseline file from scratch after a deliberate, reviewed change in what's expected to be flagged:

```bash
python3 tools/leak_sweep.py --update-baseline
# then edit tools/leak_sweep_baseline.json to fill in each entry's "reason"
```

Full methodology writeup, and the reasoning behind each currently-baselined finding, is in `docs/BUG_PATTERN_AUDIT_TODO_8.md`.

---

## `install-cross-qemu.sh`

**Status**: Dev-environment helper; installs the same packages `.github/workflows/ci.yml`'s `test-aarch64-qemu` / `test-riscv64-qemu` jobs use

Sets up local AArch64 and RISC-V 64-bit cross-compilation + QEMU
user-mode emulation on Debian/Ubuntu, so `vanic run --target=...`
and `cargo test --target ...` can be exercised without CI:
`qemu-user-static` (provides `qemu-aarch64-static` /
`qemu-riscv64-static` and kernel binfmt_misc registration, so
foreign-arch binaries run transparently), `gcc-aarch64-linux-gnu`,
and `gcc-riscv64-linux-gnu`.

```bash
./tools/install-cross-qemu.sh          # install + verify (needs sudo)
./tools/install-cross-qemu.sh --check  # verify only, no install/sudo
```

Prints ready-to-use `CARGO_TARGET_*_RUNNER` / `vanic run --target=`
invocations on success, including the `QEMU_RISCV64`/`--cpu=sifive-x280`
combo needed to exercise RVV (Vector extension) codegen and the
`QEMU_AARCH64`/`--sve2` combo for SVE. See
[Advanced 4b -- Cross-compilation](../tutorials/src/advanced/04b_cross_compile_primer.md)
and [`docs/qemu_testing.md`](../docs/qemu_testing.md) for the full
reference.

---

### llm_context/

See [`llm_context/README.md`](llm_context/README.md) for the AI-agent bundle and MCP server (`mcp_server.py`).
