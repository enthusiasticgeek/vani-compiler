#!/usr/bin/env python3
"""Regenerate src/lsp.rs's XXX_KEYWORDS completion-popup arrays from the
real keyword tables in src/lexer.rs, instead of hand-copying them.

Why: src/lsp.rs used to hand-maintain a duplicate word list per dialect
for `textDocument/completion`. Every keyword-table edit in lexer.rs
(there have been many across BUG-166..171) silently left lsp.rs's copy
stale -- 263 stale entries were found this way and filed as BUG-173
(docs/BUG_PATTERN_AUDIT_TODO_11.md). This script extracts the actual
`"word" => TokenKind::Xxx` match keys from each dialect's
`xxx_keyword`/`xxx_ascii_keyword` function(s) in lexer.rs and rewrites
the corresponding `const XXX_KEYWORDS` array in lsp.rs verbatim, so the
two can never drift as long as this is re-run after a keyword edit.
`cargo test lsp_keyword_lists_match_lexer` fails CI if they do drift
without a re-run.

Usage:
    python3 tools/regen_lsp_keywords.py            # rewrite src/lsp.rs
    python3 tools/regen_lsp_keywords.py --check     # exit 1 if stale, no write
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LEXER_PATH = ROOT / "src" / "lexer.rs"
LSP_PATH = ROOT / "src" / "lsp.rs"

# LSP array name -> lexer.rs function(s) whose match-arm string keys
# form that dialect's keyword set (ascii + native tables are unioned
# when a dialect has both, matching how the lexer itself accepts both).
MAPPING = {
    "MANDARIN_KEYWORDS": ["mandarin_keyword"],
    "DEVANAGARI_KEYWORDS": ["devanagari_keyword"],
    "BENGALI_KEYWORDS": ["bengali_keyword"],
    "TAMIL_KEYWORDS": ["tamil_keyword"],
    "TELUGU_KEYWORDS": ["telugu_keyword"],
    "GUJARATI_KEYWORDS": ["gujarati_keyword"],
    "PUNJABI_KEYWORDS": ["punjabi_keyword"],
    "KANNADA_KEYWORDS": ["kannada_keyword"],
    "ODIA_KEYWORDS": ["odia_keyword"],
    "URDU_KEYWORDS": ["urdu_keyword"],
    "PERSIAN_KEYWORDS": ["persian_keyword"],
    "KOREAN_KEYWORDS": ["korean_keyword"],
    "JAPANESE_KEYWORDS": ["japanese_keyword"],
    "ARABIC_KEYWORDS": ["arabic_keyword"],
    "HEBREW_KEYWORDS": ["hebrew_keyword"],
    "RUSSIAN_KEYWORDS": ["cyrillic_keyword"],
    "SPANISH_KEYWORDS": ["spanish_keyword", "spanish_ascii_keyword"],
    "FRENCH_KEYWORDS": ["french_keyword", "french_ascii_keyword"],
    "GERMAN_KEYWORDS": ["german_keyword", "german_ascii_keyword"],
    "PORTUGUESE_KEYWORDS": ["portuguese_keyword", "portuguese_ascii_keyword"],
    "ITALIAN_KEYWORDS": ["italian_ascii_keyword"],
    "TURKISH_KEYWORDS": ["turkish_keyword", "turkish_ascii_keyword"],
    "POLISH_KEYWORDS": ["polish_keyword", "polish_ascii_keyword"],
    "INDONESIAN_KEYWORDS": ["indonesian_ascii_keyword"],
    "MALAY_KEYWORDS": ["malay_ascii_keyword"],
    "SWAHILI_KEYWORDS": ["swahili_ascii_keyword"],
    "DUTCH_KEYWORDS": ["dutch_ascii_keyword"],
    "THAI_KEYWORDS": ["thai_keyword"],
    "HUNGARIAN_KEYWORDS": ["hungarian_keyword", "hungarian_ascii_keyword"],
    "CZECH_KEYWORDS": ["czech_keyword", "czech_ascii_keyword"],
}


def get_fn_body(lexer_src: str, name: str) -> str:
    m = re.search(r"fn " + re.escape(name) + r"\(text: &str\) -> Option<TokenKind> \{", lexer_src)
    if not m:
        raise SystemExit(f"function {name} not found in lexer.rs")
    start = m.end()
    depth = 1
    i = start
    while depth > 0:
        if lexer_src[i] == "{":
            depth += 1
        elif lexer_src[i] == "}":
            depth -= 1
        i += 1
    return lexer_src[start:i]


def extract_words(lexer_src: str, name: str) -> list[str]:
    body = get_fn_body(lexer_src, name)
    return re.findall(r'"([^"\\]+)"\s*=>\s*TokenKind::', body)


def format_array(name: str, words: list[str]) -> str:
    lines = [f"const {name}: &[&str] = &["]
    cur = "    "
    for w in words:
        tok = f'"{w}", '
        if len(cur) + len(tok) > 76:
            lines.append(cur.rstrip())
            cur = "    "
        cur += tok
    if cur.strip():
        lines.append(cur.rstrip())
    lines.append("];")
    return "\n".join(lines)


def main() -> int:
    check_only = "--check" in sys.argv
    lexer_src = LEXER_PATH.read_text()
    lsp_src = LSP_PATH.read_text()
    original = lsp_src

    for arr, fns in MAPPING.items():
        seen: list[str] = []
        for fn in fns:
            for w in extract_words(lexer_src, fn):
                if w not in seen:
                    seen.append(w)
        new_block = format_array(arr, seen)
        pattern = re.compile(r"const " + re.escape(arr) + r": &\[&str\] = &\[.*?\];", re.DOTALL)
        m = pattern.search(lsp_src)
        if not m:
            raise SystemExit(f"const {arr} not found in lsp.rs")
        lsp_src = lsp_src[: m.start()] + new_block + lsp_src[m.end() :]

    if lsp_src == original:
        print("lsp.rs keyword lists already up to date")
        return 0
    if check_only:
        print("lsp.rs keyword lists are STALE -- run without --check to regenerate")
        return 1
    LSP_PATH.write_text(lsp_src)
    print("lsp.rs keyword lists regenerated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
