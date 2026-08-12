#!/usr/bin/env python3
"""Validate (and fix) tools/vani_translate.py's ALIASES table against the
real keyword tables in src/lexer.rs.

Why: ALIASES is ~1200 lines of hand-maintained Python, independent of
lexer.rs, with no mechanism keeping the two in sync. Confirmed drift
(2026-08-12): translating several dialects (Danish, Swahili, ...) to
English left specific keywords untouched, producing output that doesn't
compile -- the exact same "hand-copied table silently drifts from
lexer.rs" shape as BUG-173 (src/lsp.rs's completion lists), fixed the
same way there: treat lexer.rs as the single source of truth and
regenerate/validate against it mechanically instead of by hand.

This script:
  1. Extracts every `"word" => TokenKind::Xxx` pair from each dialect's
     lexer.rs keyword function(s).
  2. For each (TokenKind, language) cell in ALIASES, checks the word is
     actually recognized by lexer.rs for that TokenKind + language. Cells
     that are wrong get replaced with a real word extracted from
     lexer.rs; the script does NOT invent translations lexer.rs doesn't
     already have.
  3. Adds the 6 languages that were missing from ALIASES/--to entirely
     (assamese, sindhi, punjabi_shahmukhi, nepali, maithili, konkani) --
     each is a pragma-only alias of an existing shared keyword table
     (see LANG_TABLES below), so their column is a straight copy of the
     parent language's already-valid word.

Usage:
    python3 tools/regen_vani_translate_keywords.py --check   # report only
    python3 tools/regen_vani_translate_keywords.py           # fix in place
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LEXER_PATH = ROOT / "src" / "lexer.rs"
TRANSLATE_PATH = ROOT / "tools" / "vani_translate.py"

# ---------------------------------------------------------------------------
# Language -> lexer.rs function(s) supplying its keyword table, and the
# exact `// vani-lang: <tag>` pragma string lexer.rs's pragma parser
# accepts for it (see Lexer::detect_pragma_lang / the big match in
# lexer.rs around line 5860). Order in the list matters only for which
# function's spelling gets picked as canonical when a cell needs fixing
# (first function wins); native-script tables are listed before their
# ASCII-only pragma-gated counterpart so the always-recognized spelling
# is preferred.
# ---------------------------------------------------------------------------
LANG_TABLES: dict[str, tuple[list[str], str]] = {
    # multi_word_devanagari_keyword supplies the handful of legitimate
    # space-containing phrases (Hindi "के लिए" = For, "सिद्ध करो" = Prove,
    # Marathi "सिद्ध करा" = Prove, etc.) -- without it those cells look
    # like typos to a naive single-token check.
    "sanskrit": (["devanagari_keyword", "multi_word_devanagari_keyword"], "sanskrit"),
    "hindi": (["devanagari_keyword", "multi_word_devanagari_keyword"], "hindi"),
    "marathi": (["devanagari_keyword", "multi_word_devanagari_keyword"], "marathi"),
    # Phase 2 dialects: pragma-only aliases of the shared Devanagari table.
    "nepali": (["devanagari_keyword", "multi_word_devanagari_keyword"], "nepali"),
    "maithili": (["devanagari_keyword", "multi_word_devanagari_keyword"], "maithili"),
    "konkani": (["devanagari_keyword", "multi_word_devanagari_keyword"], "konkani"),
    "bengali": (["bengali_keyword"], "bengali"),
    # Assamese: pragma-only alias of the Bengali table (shares the
    # Bengali Unicode block -- see the DialectLang::Assamese doc comment).
    "assamese": (["bengali_keyword"], "assamese"),
    "tamil": (["tamil_keyword"], "tamil"),
    "telugu": (["telugu_keyword"], "telugu"),
    "gujarati": (["gujarati_keyword"], "gujarati"),
    "punjabi": (["punjabi_keyword"], "punjabi"),
    "kannada": (["kannada_keyword"], "kannada"),
    "malayalam": (["malayalam_keyword"], "malayalam"),
    "odia": (["odia_keyword"], "odia"),
    "sinhala": (["sinhala_keyword"], "sinhala"),
    "urdu": (["urdu_keyword"], "urdu"),
    # Sindhi: pragma-only alias of the Urdu table (v1 accepts the Urdu
    # keyword union -- see the DialectLang::Sindhi doc comment).
    "sindhi": (["urdu_keyword"], "sindhi"),
    # Punjabi-Shahmukhi: pragma-only alias of the Urdu table too (see
    # examples/language/punjabi_shahmukhi/basics.vani's own header
    # comment: "the dialect tag accepts the Urdu keyword union").
    # NOTE pragma tag uses a HYPHEN, not underscore.
    "punjabi_shahmukhi": (["urdu_keyword"], "punjabi-shahmukhi"),
    "persian": (["persian_keyword"], "persian"),
    "pashto": (["pashto_keyword"], "pashto"),
    "russian": (["cyrillic_keyword"], "russian"),
    "spanish": (["spanish_keyword", "spanish_ascii_keyword"], "spanish"),
    "french": (["french_keyword", "french_ascii_keyword"], "french"),
    "japanese": (["japanese_keyword"], "japanese"),
    "mandarin": (["mandarin_keyword"], "mandarin"),
    "korean": (["korean_keyword"], "korean"),
    "german": (["german_keyword", "german_ascii_keyword"], "german"),
    "portuguese": (["portuguese_keyword", "portuguese_ascii_keyword"], "portuguese"),
    "indonesian": (["indonesian_ascii_keyword"], "indonesian"),
    "greek": (["greek_keyword"], "greek"),
    "hebrew": (["hebrew_keyword"], "hebrew"),
    "italian": (["italian_ascii_keyword"], "italian"),
    "arabic": (["arabic_keyword"], "arabic"),
    "polish": (["polish_keyword", "polish_ascii_keyword"], "polish"),
    "turkish": (["turkish_keyword", "turkish_ascii_keyword"], "turkish"),
    "malay": (["malay_ascii_keyword"], "malay"),
    "swahili": (["swahili_ascii_keyword"], "swahili"),
    "vietnamese": (["vietnamese_keyword", "vietnamese_ascii_keyword"], "vietnamese"),
    "romanian": (["romanian_keyword", "romanian_ascii_keyword"], "romanian"),
    "dutch": (["dutch_ascii_keyword"], "dutch"),
    "thai": (["thai_keyword"], "thai"),
    "hungarian": (["hungarian_keyword", "hungarian_ascii_keyword"], "hungarian"),
    "czech": (["czech_keyword", "czech_ascii_keyword"], "czech"),
    "slovak": (["slovak_keyword", "slovak_ascii_keyword"], "slovak"),
    "finnish": (["finnish_keyword", "finnish_ascii_keyword"], "finnish"),
    "swedish": (["swedish_keyword", "swedish_ascii_keyword"], "swedish"),
    "filipino": (["filipino_ascii_keyword"], "filipino"),
    "norwegian": (["norwegian_keyword", "norwegian_ascii_keyword"], "norwegian"),
    "danish": (["danish_keyword", "danish_ascii_keyword"], "danish"),
    "armenian": (["armenian_keyword"], "armenian"),
    "georgian": (["georgian_keyword"], "georgian"),
    "catalan": (["catalan_keyword", "catalan_ascii_keyword"], "catalan"),
    "yoruba": (["yoruba_keyword"], "yoruba"),
    "hausa": (["hausa_keyword", "hausa_ascii_keyword"], "hausa"),
    "khmer": (["khmer_keyword"], "khmer"),
    "burmese": (["burmese_keyword"], "burmese"),
    "amharic": (["amharic_keyword"], "amharic"),
    "tibetan": (["tibetan_keyword"], "tibetan"),
    "cherokee": (["cherokee_keyword"], "cherokee"),
    "lao": (["lao_keyword"], "lao"),
    "mongolian": (["mongolian_keyword"], "mongolian"),
}

# Languages whose column is a straight copy of a parent language's
# already-curated word (shared lexer.rs table, see LANG_TABLES comments).
ALIAS_OF: dict[str, str] = {
    "nepali": "hindi",
    "maithili": "hindi",
    "konkani": "hindi",
    "assamese": "bengali",
    "sindhi": "urdu",
    "punjabi_shahmukhi": "urdu",
}

TOKENKIND_RE = re.compile(r'"([^"\\]+)"\s*=>\s*TokenKind::(\w+)')


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


def extract_pairs(lexer_src: str, fn_name: str) -> list[tuple[str, str]]:
    body = get_fn_body(lexer_src, fn_name)
    return TOKENKIND_RE.findall(body)


def build_lang_index(lexer_src: str) -> dict[str, dict[str, list[str]]]:
    """lang -> {TokenKind: [word, word, ...]} (first word per fn is
    listed first; a language's own list may have duplicates across its
    tables, order preserved, first occurrence wins as canonical)."""
    index: dict[str, dict[str, list[str]]] = {}
    cache: dict[str, list[tuple[str, str]]] = {}
    for lang, (fns, _pragma) in LANG_TABLES.items():
        by_kind: dict[str, list[str]] = {}
        for fn in fns:
            if fn not in cache:
                cache[fn] = extract_pairs(lexer_src, fn)
            for word, kind in cache[fn]:
                by_kind.setdefault(kind, [])
                if word not in by_kind[kind]:
                    by_kind[kind].append(word)
        index[lang] = by_kind
    return index


def extract_english_base_pairs(lexer_src: str) -> list[tuple[str, str]]:
    """The inline `match text { "fn" => TokenKind::Fn, ... }` block inside
    `lex_ident` -- English's canonical spelling AND every English-side
    alias (`give`/`give_back` for Return, `record` for Struct, `trait`
    for Interface, `impl` for Implement, `mod` for Module, `public` for
    Pub, `write` for Print, `yields` for Arrow, etc). Not a standalone
    `fn xxx_keyword`, so extracted by line range instead of brace-depth."""
    start_m = re.search(r"fn lex_ident\(&mut self, start: usize\) \{", lexer_src)
    if not start_m:
        raise SystemExit("lex_ident not found in lexer.rs")
    block_m = re.search(r"let kind = match text \{", lexer_src[start_m.end():])
    if not block_m:
        raise SystemExit("lex_ident's English match block not found")
    block_start = start_m.end() + block_m.end()
    end_m = re.search(r"_ if text\.bytes\(\)\.any", lexer_src[block_start:])
    if not end_m:
        raise SystemExit("end of lex_ident's English match block not found")
    body = lexer_src[block_start: block_start + end_m.start()]
    return TOKENKIND_RE.findall(body)


def build_all_synonyms(lexer_src: str, lang_index: dict[str, dict[str, list[str]]]) -> dict[str, list[str]]:
    """TokenKind -> every word ANY language (including English's own
    aliases) accepts for it, union across the whole compiler. Used to
    let the translate tool recognize source-language spellings that
    aren't ALIASES's single curated "canonical" pick for that language
    (e.g. Danish's ASCII "formaal" alongside native "formål" -- both are
    real lexer.rs Intent keywords, but ALIASES only ever names one)."""
    all_syn: dict[str, list[str]] = {}

    def add(word: str, kind: str) -> None:
        lst = all_syn.setdefault(kind, [])
        if word not in lst:
            lst.append(word)

    for word, kind in extract_english_base_pairs(lexer_src):
        add(word, kind)
    for by_kind in lang_index.values():
        for kind, words in by_kind.items():
            for w in words:
                add(w, kind)
    return all_syn


def load_aliases_module():
    import importlib.util

    spec = importlib.util.spec_from_file_location("vani_translate", TRANSLATE_PATH)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def main() -> int:
    check_only = "--check" in sys.argv
    lexer_src = LEXER_PATH.read_text(encoding="utf-8")
    lang_index = build_lang_index(lexer_src)

    mod = load_aliases_module()
    aliases = mod.ALIASES

    problems: list[str] = []
    fixes: dict[tuple[str, str], str] = {}  # (TokenKind, lang) -> new word

    # Pass 1: validate + fix existing cells for languages already in ALIASES.
    for kind, per_lang in aliases.items():
        for lang, word in per_lang.items():
            if lang == "english":
                continue
            table = lang_index.get(lang)
            if table is None:
                problems.append(f"UNKNOWN LANG in ALIASES: {lang!r} (TokenKind {kind})")
                continue
            valid_words = table.get(kind, [])
            if word not in valid_words:
                if valid_words:
                    replacement = valid_words[0]
                    problems.append(
                        f"WRONG  {kind:12s} {lang:20s} {word!r} -> not recognized; "
                        f"replacing with {replacement!r}"
                    )
                    fixes[(kind, lang)] = replacement
                else:
                    problems.append(
                        f"MISSING {kind:12s} {lang:20s} {word!r} -> lexer.rs has NO "
                        f"word for this TokenKind in this language's table at all "
                        f"(left as-is, needs a real lexer.rs addition first)"
                    )

    # Pass 2: add missing languages as straight copies of their parent.
    for lang, parent in ALIAS_OF.items():
        for kind, per_lang in aliases.items():
            if lang in per_lang:
                continue
            parent_word = per_lang.get(parent)
            if parent_word is None:
                continue
            table = lang_index.get(lang, {})
            valid_words = table.get(kind, [])
            if parent_word in valid_words:
                fixes[(kind, lang)] = parent_word
            elif valid_words:
                fixes[(kind, lang)] = valid_words[0]
                problems.append(
                    f"NEWLANG {kind:12s} {lang:20s} parent {parent!r}'s word "
                    f"{parent_word!r} not valid here; using {valid_words[0]!r} instead"
                )
            else:
                problems.append(
                    f"NEWLANG {kind:12s} {lang:20s} -- no word available at all "
                    f"(no entry added for this cell)"
                )

    # Pass 3: fill any OTHER missing cell (a language that already has
    # some entries in this TokenKind's dict, or none at all, but isn't
    # one of ALIAS_OF's straight-copy cases) directly from lang_index,
    # whenever lexer.rs actually has a word for it. This is the "table
    # was just never backfilled for this language" gap (confirmed via
    # e.g. Pashto's Assert/Prove/Intent -- all three are real, longstanding
    # entries in pashto_keyword, just never copied into ALIASES) --
    # different from Pass 1's "wrong spelling" case and Pass 2's "whole
    # new language" case. Cells with genuinely no lexer.rs word are left
    # alone (translate() falls back to English's spelling for those,
    # same as before this script existed -- a real lexer.rs gap, not a
    # Python-table gap, and out of scope for this script to invent).
    for lang in LANG_TABLES:
        if lang in ALIAS_OF:
            continue
        for kind, per_lang in aliases.items():
            if lang in per_lang or (kind, lang) in fixes:
                continue
            table = lang_index.get(lang, {})
            valid_words = table.get(kind, [])
            if valid_words:
                fixes[(kind, lang)] = valid_words[0]
                problems.append(
                    f"BACKFILL {kind:12s} {lang:20s} -- was entirely absent from "
                    f"ALIASES; adding {valid_words[0]!r} from lexer.rs"
                )

    print(f"{len(problems)} problem(s) found across {len(aliases)} TokenKinds x "
          f"{len(lang_index)} languages.")
    for p in problems[:600]:
        print(" ", p)
    if len(problems) > 600:
        print(f"  ... and {len(problems) - 600} more")

    all_syn = build_all_synonyms(lexer_src, lang_index)
    syn_stale = format_all_synonyms(all_syn) not in TRANSLATE_PATH.read_text(encoding="utf-8")

    if check_only:
        if syn_stale:
            print("ALL_SYNONYMS is STALE -- run without --check to regenerate")
        return 1 if (problems or syn_stale) else 0

    if fixes:
        apply_fixes(fixes)
        print(f"Applied {len(fixes)} fix(es)/addition(s) to {TRANSLATE_PATH}")
    else:
        print("ALIASES: nothing to fix.")

    changed = write_all_synonyms(all_syn)
    print("ALL_SYNONYMS regenerated." if changed else "ALL_SYNONYMS already up to date.")
    return 0


def apply_fixes(fixes: dict[tuple[str, str], str]) -> None:
    """Rewrite ALIASES entries in tools/vani_translate.py source text.
    Works block-by-block on each `"TokenKind": { ... },` dict literal so
    we don't need a full Python-AST round-trip (which would strip
    comments / reformat the whole 1200-line table)."""
    src = TRANSLATE_PATH.read_text(encoding="utf-8")

    by_kind: dict[str, dict[str, str]] = {}
    for (kind, lang), word in fixes.items():
        by_kind.setdefault(kind, {})[lang] = word

    for kind, lang_fixes in by_kind.items():
        block_re = re.compile(
            r'("' + re.escape(kind) + r'":\s*\{)(.*?)(\n    \},)', re.DOTALL
        )
        m = block_re.search(src)
        if not m:
            print(f"  WARNING: could not find ALIASES[{kind!r}] block, skipping", file=sys.stderr)
            continue
        body = m.group(2)
        for lang, word in lang_fixes.items():
            entry_re = re.compile(r'"' + re.escape(lang) + r'":\s*"[^"]*"')
            new_entry = f'"{lang}": "{word}"'
            if entry_re.search(body):
                body = entry_re.sub(new_entry, body, count=1)
            else:
                # New language column -- append before the closing brace,
                # on its own indented line.
                body = body.rstrip() + f'\n        "{lang}": "{word}",'
        src = src[: m.start()] + m.group(1) + body + m.group(3) + src[m.end():]

    TRANSLATE_PATH.write_text(src, encoding="utf-8")


_SYN_BEGIN = "# BEGIN ALL_SYNONYMS (auto-generated by tools/regen_vani_translate_keywords.py)"
_SYN_END = "# END ALL_SYNONYMS"


def format_all_synonyms(all_syn: dict[str, list[str]]) -> str:
    lines = [_SYN_BEGIN, "ALL_SYNONYMS: Dict[str, List[str]] = {"]
    for kind in sorted(all_syn):
        words = all_syn[kind]
        rendered = ", ".join(repr(w) for w in words)
        line = f'    "{kind}": [{rendered}],'
        if len(line) <= 100:
            lines.append(line)
        else:
            lines.append(f'    "{kind}": [')
            cur = "        "
            for w in words:
                tok = f"{w!r}, "
                if len(cur) + len(tok) > 96:
                    lines.append(cur.rstrip())
                    cur = "        "
                cur += tok
            if cur.strip():
                lines.append(cur.rstrip())
            lines.append("    ],")
    lines.append("}")
    lines.append(_SYN_END)
    return "\n".join(lines)


def write_all_synonyms(all_syn: dict[str, list[str]]) -> bool:
    """Insert or replace the ALL_SYNONYMS block right after MULTI_WORD_ALIASES
    in tools/vani_translate.py. Returns True if the file changed."""
    src = TRANSLATE_PATH.read_text(encoding="utf-8")
    block = format_all_synonyms(all_syn)

    block_re = re.compile(re.escape(_SYN_BEGIN) + r".*?" + re.escape(_SYN_END), re.DOTALL)
    if block_re.search(src):
        # Callable replacement: `block` may itself contain literal
        # backslashes (e.g. Mongolian codepoints whose repr() escapes to
        # "᠎"), which re.sub's string-replacement DSL would
        # otherwise try to interpret as backreferences/escapes.
        new_src = block_re.sub(lambda _m: block, src)
    else:
        anchor = "MULTI_WORD_ALIASES: Dict[Tuple[str, ...], str] = {"
        idx = src.find(anchor)
        if idx == -1:
            raise SystemExit("could not find MULTI_WORD_ALIASES anchor to insert ALL_SYNONYMS after")
        close_idx = src.find("\n}\n", idx)
        if close_idx == -1:
            raise SystemExit("could not find end of MULTI_WORD_ALIASES dict")
        insert_at = close_idx + len("\n}\n")
        new_src = src[:insert_at] + "\n" + block + "\n\n" + src[insert_at:]

    if new_src == src:
        return False
    TRANSLATE_PATH.write_text(new_src, encoding="utf-8")
    return True


if __name__ == "__main__":
    raise SystemExit(main())
