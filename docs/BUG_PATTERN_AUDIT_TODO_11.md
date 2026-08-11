# BUG_PATTERN_AUDIT_TODO_11.md

Found 2026-08-10/11 while answering "any LSP server updates needed?" as a
wrap-up to the BUG-171 native-speaker review series. Filed as BUG-173.

## The problem

`src/lsp.rs` maintains ~30 hand-written `const XXX_KEYWORDS: &[&str]`
arrays (starting around line 1138, `dialect_keywords_for`) that duplicate
a subset of each dialect's keyword table from `src/lexer.rs`, used to
populate the `textDocument/completion` popup. These are explicitly
commented as "compact... not exhaustive" and were clearly hand-typed once
and never mechanically kept in sync with `lexer.rs` afterward.

A script comparing every string literal in every `XXX_KEYWORDS` array
against whether that exact string appears anywhere in `src/lexer.rs`
found **263 stale entries** (words that no longer exist in the lexer at
all) spread across essentially every dialect the LSP covers — not
isolated to any one round of edits. Five of these were caused by this
session's BUG-171 fixes (Tamil `இருக்க`, French `tache`, Russian
`попробовать`, Hebrew `עבור`, Italian `fino`) and have already been fixed
directly in `src/lsp.rs` in this session's commit. The other ~258 predate
this session by a wide margin — e.g. German's LSP list has always said
`"abbrechen"` for Break when the real keyword has apparently always been
`"brechen"`, and this has nothing to do with any BUG-16x/17x fix. Every
`XXX_KEYWORDS` array has multiple stale entries; this is not a few
outliers.

Impact is low-severity but real: a user typing in the LSP-integrated
editor sees a keyword suggested in the completion popup that, if
accepted, does not actually work as that keyword when the file is
compiled — a confusing papercut, not a compile/runtime bug (`vanic` never
consults these lists; they're purely `src/lsp.rs`'s own completion
feature).

## Why not just hand-fix all 263 now

Each stale word needs to be replaced with whatever the *current* correct
word actually is in `lexer.rs` for that (dialect, TokenKind) slot — that
requires looking up ~258 individual entries across ~30 dialects' native
and ASCII tables, the same kind of work as another full BUG-171-style
pass. Doing it fast and blind risks introducing new mistakes rather than
fixing old ones, so it wasn't attempted in this session; only the 5
entries this session's own edits broke were fixed.

## Recommended fix: eliminate the duplication, don't re-sync it

The `XXX_KEYWORDS` arrays are hand-copies of match-arm keys that already
exist verbatim in `src/lexer.rs`'s `xxx_keyword`/`xxx_ascii_keyword`
functions. Every future keyword-table edit (and there have been many —
BUG-166 through BUG-171 alone touched dozens of dialects) will silently
re-break this list again unless the duplication itself is removed.
Concretely:

1. Refactor the `xxx_keyword` functions in `lexer.rs` to be generated
   from (or to also expose) a `&[(&str, TokenKind)]` table instead of a
   bare `match` — many already read almost like a data table with a
   trailing `_ => return None`. A `phf`-style const table or a simple
   `&[(&str, TokenKind)]` slice that the `match`-based lookup is built
   from at compile time would let `src/lsp.rs` iterate the *same* table
   `lexer.rs` uses to lex, instead of a separate hand-copy.
2. `dialect_keywords_for` in `lsp.rs` would then just filter that shared
   table's keys instead of returning a hand-maintained `&'static [&str]`.
3. This is a bigger refactor than a word-list re-sync, but it's the only
   fix that doesn't silently rot again on the next localization pass.

If the full refactor isn't wanted, a cheaper interim step: add a `cargo
test` (mirroring the `bug170_global_dialects_structure_keyword_parity`
style already used in `src/lib.rs` — parse `lexer.rs`'s source text at
test time) that asserts every string in every `src/lsp.rs` `XXX_KEYWORDS`
array appears somewhere in the corresponding dialect's lexer function(s).
That at least turns future staleness into a CI failure instead of a
silent drift, without requiring the full table-sharing refactor.

## Scope reference

Re-run this to regenerate the current stale-entry list before starting a
fix pass (file sets drift):

```python
import re
lsp_src = open('src/lsp.rs').read()
lexer_src = open('src/lexer.rs').read()
for name, body in re.findall(r'const (\w+_KEYWORDS): &\[&str\] = &\[(.*?)\];', lsp_src, re.DOTALL):
    words = re.findall(r'"([^"]+)"', body)
    missing = [w for w in words if f'"{w}"' not in lexer_src]
    if missing:
        print(f"{name} ({len(missing)} of {len(words)} stale): {missing}")
```

## Update 2026-08-11: BUG-173 CLOSED

Took the "cheaper interim step" this doc flagged, but made it also do
the actual re-sync instead of just detecting drift: `tools/
regen_lsp_keywords.py` mechanically extracts every `"word" =>
TokenKind::...` match key from each dialect's `xxx_keyword`/
`xxx_ascii_keyword` function(s) in `src/lexer.rs` and rewrites the
corresponding `const XXX_KEYWORDS` array in `src/lsp.rs` from that
authoritative source -- no hand-guessing at what the "current" correct
word is, which is what made a manual fix of 263 entries risky. Re-ran
it once to fix all 263 stale entries across all ~30 covered dialects in
one mechanical pass (verified via the script's own `--check` mode:
before the regen it reported drift, after it reported "already up to
date").

Also added the drift-detection regression test recommended above --
`lsp_keyword_lists_match_lexer` in `src/lsp.rs`'s test module -- so a
FUTURE keyword-table edit in `lexer.rs` that isn't followed by
re-running `tools/regen_lsp_keywords.py` now fails CI instead of
silently rotting again the way the original 263 entries did.

The full architectural fix (sharing one literal table between the
lexer's `match` and the LSP's completion list, eliminating the
duplication entirely) is still not done -- that remains a larger,
separate refactor if ever wanted. The regen script + test combination
closes the actual bug (stale suggestions in the completion popup)
without it.

Did NOT expand LSP dialect coverage beyond the ~30 already-covered
dialects (BUG-173 was about existing entries going stale, not about
adding new dialect support to the LSP) -- that would be a feature
request, filed separately if wanted.
