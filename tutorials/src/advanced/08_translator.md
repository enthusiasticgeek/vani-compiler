# Advanced 8 — Writing a cross-language translator extension

> **Learning goal**: extend `tools/vani_translate.py` to
> support a new keyword spelling or a new dialect, and
> understand the round-trip parity guarantee.

**Who this chapter is for**: contributors who want to add
vocabulary for a dialect that's close to one already shipped
(e.g. a regional spelling variant), or developers who want to
understand how the keyword-substitution pipeline works. The
translator is a Python script — not part of the Rust compiler —
so you don't need to understand compiler internals to use it.

## What the translator does

`tools/vani_translate.py` does token-level keyword substitution
between vāṇी's supported dialects: English ↔ Sanskrit ↔ Hindi
↔ Marathi (the other dialects accept the union of these as a
starting point).

```bash
# English → Sanskrit (with auspicious header)
python3 tools/vani_translate.py --to sanskrit \
    examples/language/english/basics.vani \
    -o /tmp/basics_sa.vani --add-sri-header

# Run the translation to verify identical behavior
vanic run /tmp/basics_sa.vani

# Translate any pair
python3 tools/vani_translate.py --to marathi /tmp/basics_sa.vani \
    -o /tmp/basics_mr.vani
```

The `--from` flag is advisory — the translator recognizes
keywords regardless of source dialect because the alias table
is bidirectional.

## Round-trip parity

The contract: `english → sanskrit → english` produces a file
that compiles to the same AST as the original (modulo
whitespace and which alias was picked when multiple existed).
This is the property the test ledger pins.

## The alias table

The full mapping lives in
`tools/vani_translate.py::ALIASES` — one Python dict, one
entry per `TokenKind`:

```python
ALIASES = {
    "Fn":         {"english": "fn",       "sanskrit": "कार्य",
                   "hindi":   "फलन",     "marathi":  "कार्य"},
    "Let":        {"english": "let",      "sanskrit": "माना",
                   "hindi":   "माना",    "marathi":  "मान"},
    ...
}
```

When multiple aliases exist for a `(kind, dialect)` pair (e.g.
`Fn` accepts both `कार्य` and `फलन` in Hindi), the table picks
the most natural / most-common spelling. A future enhancement
could preserve the source's specific choice; v1 normalizes.

## Adding a new keyword to the table

Suppose you want to add a Hindi alias for `pure`. The lexer
already accepts `शुद्ध` (Sanskrit/Hindi/Marathi tatsama); you
want the translator to emit it too.

1. Open `tools/vani_translate.py`.
2. Find the `Pure` entry. Confirm `hindi` is already populated.
   If it's missing or wrong, edit:
   ```python
   "Pure": {"english": "pure", "sanskrit": "शुद्ध",
            "hindi": "शुद्ध", "marathi": "शुद्ध"},
   ```
3. Run the translator round-trip on a file that uses `pure`:
   ```bash
   python3 tools/vani_translate.py --to hindi src/foo.vani -o /tmp/foo_hi.vani
   python3 tools/vani_translate.py --to english /tmp/foo_hi.vani -o /tmp/foo_back.vani
   diff src/foo.vani /tmp/foo_back.vani    # should be near-empty
   ```

## Adding a new dialect

The translator currently understands 4 dialects (English /
Sanskrit / Hindi / Marathi). To extend to a new dialect (say,
Nepali):

1. Add a `nepali` column to every row of `ALIASES`. Reuse the
   tatsama Sanskrit spelling where Nepali matches; pick a
   Nepali-specific verb otherwise.
2. Update `SUPPORTED_LANGS` to include the new dialect:
   ```python
   SUPPORTED_LANGS = ("english", "sanskrit", "hindi", "marathi", "nepali")
   ```
3. Add the `--add-sri-header` behavior if appropriate (Nepali
   uses Devanagari, so `// श्री।` still fits).
4. Update `tools/llm_context/bundle.py::emit_aliases` — it
   reads `ALIASES` directly, so the LLM context bundle picks
   up the new dialect automatically.
5. Add a test in `tools/test_translate.py` (TBD module) that
   round-trips a small program through the new dialect.

## What the translator doesn't do

- **It's not an SOV reshape**: source word order is preserved.
  Translating English `print x;` to Sanskrit yields
  `लिख x;`, not the SOV form `x लिख;`. Use `vanic fmt` for
  canonicalization.
- **Comments are preserved verbatim** — the user controls their
  language.
- **The four English-only keywords** (`extern`, `type`,
  `intent`, `invariant`) are now translated too (SOV-S7 expanded
  the alias table). Anything not in `ALIASES` passes through
  unchanged.

## When you'd reach for this

- You wrote a new Devanagari example in Sanskrit and want
  Hindi/Marathi clones for the parity sweep.
- You want to read a Hindi-language teammate's code in English
  without leaving your editor.
- You're contributing a new dialect alias and want the
  translator to emit the canonical spelling.

## Challenge

Pick one structure-keyword you use most often. Add a
hand-curated alias for it in a hypothetical dialect you'd want
to see supported (e.g. Odia, Sinhala, or your own first-
language). Run the round-trip and read the diff out loud.

---

**Next**: [§9 — Adding a new dialect →](09_new_dialect.md)
