#!/usr/bin/env python3
"""
vani_translate — translate a .vani source file's keywords between
                English, Sanskrit, Hindi, and Marathi.

B.1 v3 — adds SOV <-> SVO word-order reordering for verb-final
statements and Hindi for-range loops; adds optional LLM-based
translation of comments, string literals, and identifiers.

Usage:
    # Translate to Sanskrit (auto-detects source from pragma):
    python3 tools/vani_translate.py examples/language/english/basics.vani \\
        --to sanskrit -o out.vani

    # SOV word-order is reordered automatically:
    #   hindi:    n पुनरागम;            -> english: return n;
    #   english:  return n;             -> hindi:   n लौटाओ;
    #   hindi:    i के लिए 0 से 5 तक { -> english: for i from 0 to 5 {
    #   english:  for i from 0 to 5 {  -> hindi:   i के लिए 0 से 5 तक {

    # LLM translation of comments + strings (Anthropic):
    python3 tools/vani_translate.py basics.vani --to hindi \\
        --llm anthropic --llm-model claude-haiku-4-5-20251001

    # LLM translation via local Ollama:
    python3 tools/vani_translate.py basics.vani --to hindi \\
        --llm ollama --llm-model llama3.2

    # LLM translation via OpenAI:
    python3 tools/vani_translate.py basics.vani --to hindi \\
        --llm openai --llm-model gpt-4o-mini

    # Also translate identifiers (camelCase/snake_case split and translated):
    python3 tools/vani_translate.py basics.vani --to hindi \\
        --llm anthropic --translate-identifiers

    # Verify round-trip:
    python3 tools/vani_translate.py basics.vani --to hindi --verify

    # Print all keyword aliases as a markdown table:
    python3 tools/vani_translate.py --list-keywords

What this does NOT do:
  - Translate block comments /* ... */ (only line comments // ... are translated).
  - Translate multi-line string literals spanning more than one line.
  - Handle nested for-range SOV patterns (only the outermost level is reordered).
"""

import argparse
import io
import re
import sys
from pathlib import Path
from typing import Dict, List, Optional, Tuple

# Ensure UTF-8 output on Windows (default console is cp1252).
if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8")
if hasattr(sys.stderr, "reconfigure"):
    sys.stderr.reconfigure(encoding="utf-8")


# ---------------------------------------------------------------------------
# Keyword alias table.  Source of truth: src/lexer.rs
# ---------------------------------------------------------------------------

ALIASES: Dict[str, Dict[str, str]] = {
    # Declarations
    "Fn":         {"english": "fn",        "sanskrit": "कार्य",        "hindi": "फलन",        "marathi": "कार्य",       "mandarin": "函数"},
    "Let":        {"english": "let",       "sanskrit": "माना",        "hindi": "माना",        "marathi": "मान",          "mandarin": "让"},
    "Struct":     {"english": "struct",    "sanskrit": "संरचना",       "hindi": "संरचना",      "marathi": "संरचना",      "mandarin": "结构"},
    "Enum":       {"english": "enum",      "sanskrit": "विकल्प",       "hindi": "गणन",         "marathi": "गणन",          "mandarin": "枚举"},
    "Const":      {"english": "const",     "sanskrit": "स्थिर",         "hindi": "स्थिर",       "marathi": "स्थिर",        "mandarin": "常量"},
    "Type":       {"english": "type",      "sanskrit": "प्रकार",        "hindi": "प्रकार",       "marathi": "प्रकार",       "mandarin": "类型"},
    "Extern":     {"english": "extern",    "sanskrit": "बाह्य",         "hindi": "बाह्य",       "marathi": "बाह्य",        "mandarin": "外部"},
    "Intent":     {"english": "intent",    "sanskrit": "उद्देश्य",      "hindi": "उद्देश्य",    "marathi": "उद्देश्य",     "mandarin": "目的"},
    "Invariant":  {"english": "invariant", "sanskrit": "अपरिवर्तनीय",   "hindi": "अपरिवर्तनीय", "marathi": "अपरिवर्तनीय",  "mandarin": "不变量"},

    # Visibility / modules / imports
    "Pub":        {"english": "pub",       "sanskrit": "सार्वजनिक",    "hindi": "सार्वजनिक",   "marathi": "सार्वजनिक",   "mandarin": "公开"},
    "Module":     {"english": "module",    "sanskrit": "खण्ड",         "hindi": "मॉड्यूल",     "marathi": "मॉड्यूल",      "mandarin": "模块"},
    "Use":        {"english": "use",       "sanskrit": "उपयोग",        "hindi": "उपयोग",       "marathi": "उपयोग",        "mandarin": "使用"},
    "As":         {"english": "as",        "sanskrit": "यथा",          "hindi": "यथा",         "marathi": "यथा",          "mandarin": "作为"},

    # Control flow
    "Return":     {"english": "return",    "sanskrit": "पुनरागम",      "hindi": "लौटाओ",       "marathi": "परत",          "mandarin": "返回"},
    "If":         {"english": "if",        "sanskrit": "यदि",          "hindi": "अगर",         "marathi": "जर",           "mandarin": "如果"},
    "Else":       {"english": "else",      "sanskrit": "अन्यथा",       "hindi": "वरना",        "marathi": "नाहीतर",       "mandarin": "否则"},
    "While":      {"english": "while",     "sanskrit": "यावत्",         "hindi": "जबतक",        "marathi": "जोपर्यंत",     "mandarin": "当"},
    "For":        {"english": "for",       "sanskrit": "प्रति",         "hindi": "के लिए",      "marathi": "साठी",         "mandarin": "对于"},
    "In":         {"english": "in",        "sanskrit": "में",           "hindi": "में",          "marathi": "में",          "mandarin": "in"},
    "From":       {"english": "from",      "sanskrit": "से",           "hindi": "से",           "marathi": "से",           "mandarin": "从"},
    "To":         {"english": "to",        "sanskrit": "तक",           "hindi": "तक",           "marathi": "तक",           "mandarin": "到"},
    "Break":      {"english": "break",     "sanskrit": "विराम",        "hindi": "रुको",         "marathi": "थांब",         "mandarin": "中断"},
    "Continue":   {"english": "continue",  "sanskrit": "अग्रे",        "hindi": "आगे",          "marathi": "पुढे",         "mandarin": "继续"},
    "Then":       {"english": "then",      "sanskrit": "तदा",          "hindi": "तो",           "marathi": "तर",           "mandarin": "那么"},

    # References
    "Ref":        {"english": "ref",       "sanskrit": "दृष्ट्या",     "hindi": "देखो",         "marathi": "पहा",          "mandarin": "引用"},
    "Mut":        {"english": "mut",       "sanskrit": "परिवर्तनीय",   "hindi": "परिवर्तनीय",   "marathi": "बदल",          "mandarin": "可变"},

    # Matching
    "Match":      {"english": "match",     "sanskrit": "मेल",          "hindi": "मिलान",        "marathi": "जुळवा",        "mandarin": "匹配"},

    # Verification
    "Assert":     {"english": "assert",    "sanskrit": "सिद्धम्",       "hindi": "सुनिश्चित",   "marathi": "खात्री",       "mandarin": "断言"},
    "Prove":      {"english": "prove",     "sanskrit": "प्रमाण",       "hindi": "सिद्ध करो",    "marathi": "सिद्ध करा",    "mandarin": "证明"},
    "Requires":   {"english": "requires",  "sanskrit": "अपेक्षित",     "hindi": "चाहिए",       "marathi": "पाहिजे",        "mandarin": "要求"},
    "Ensures":    {"english": "ensures",   "sanskrit": "सुनिश्चयित",   "hindi": "निश्चित",      "marathi": "निश्चित",      "mandarin": "保证"},

    # Bool / print
    "True":       {"english": "true",      "sanskrit": "सत्य",         "hindi": "सत्य",         "marathi": "सत्य",         "mandarin": "真"},
    "False":      {"english": "false",     "sanskrit": "असत्य",        "hindi": "असत्य",       "marathi": "असत्य",        "mandarin": "假"},
    "Print":      {"english": "print",     "sanskrit": "लिख",          "hindi": "लिखो",         "marathi": "लिहा",         "mandarin": "打印"},

    # Purity / parallelism
    "Pure":       {"english": "pure",      "sanskrit": "शुद्ध",        "hindi": "शुद्ध",        "marathi": "शुद्ध",        "mandarin": "纯"},
    "Parallel":   {"english": "parallel",  "sanskrit": "समानांतर",     "hindi": "समानांतर",    "marathi": "समानांतर",     "mandarin": "并行"},
    "Reduce":     {"english": "reduce",    "sanskrit": "संक्षेप",      "hindi": "संक्षेप",      "marathi": "संक्षेप",      "mandarin": "reduce"},
    "With":       {"english": "with",      "sanskrit": "सह",           "hindi": "सह",           "marathi": "सह",           "mandarin": "with"},

    # Interfaces / methods
    "Interface":  {"english": "interface", "sanskrit": "संकेत",        "hindi": "संकेत",        "marathi": "संकेत",       "mandarin": "接口"},
    "Implement":  {"english": "implement", "sanskrit": "कार्यान्वित",  "hindi": "कार्यान्वित",  "marathi": "कार्यान्वित", "mandarin": "实现"},
    "Methods":    {"english": "methods",   "sanskrit": "विधि",          "hindi": "विधि",         "marathi": "विधि",        "mandarin": "方法"},

    # Bounds
    "Where":      {"english": "where",     "sanskrit": "यत्र",          "hindi": "जहाँ",         "marathi": "जिथे",         "mandarin": "其中"},
    "Is":         {"english": "is",        "sanskrit": "अस्ति",         "hindi": "है",           "marathi": "आहे",          "mandarin": "is"},

    # Concurrency
    "Try":        {"english": "try",       "sanskrit": "प्रयास",        "hindi": "प्रयास",      "marathi": "प्रयास",      "mandarin": "尝试"},
    "Task":       {"english": "task",      "sanskrit": "नियोग",         "hindi": "नियोग",       "marathi": "नियोग",        "mandarin": "任务"},
    "Join":       {"english": "join",      "sanskrit": "संयोजन",        "hindi": "संयोजन",      "marathi": "संयोजन",       "mandarin": "等待"},

    # Embedded
    "Unsafe":     {"english": "unsafe",    "sanskrit": "असुरक्षित",     "hindi": "असुरक्षित",   "marathi": "असुरक्षित",   "mandarin": "不安全"},
    "RegionKw":   {"english": "region",    "sanskrit": "क्षेत्र",        "hindi": "क्षेत्र",      "marathi": "क्षेत्र",       "mandarin": "区域"},
}

SUPPORTED_LANGS = ("english", "sanskrit", "hindi", "marathi", "mandarin")

# Devanagari Indo-Aryan targets that get the श्री। header.
_IA_DEVANAGARI = frozenset(("sanskrit", "hindi", "marathi", "nepali", "maithili", "konkani"))

# Languages with SOV (Subject-Object-Verb) word order for certain constructs.
SOV_LANGS = frozenset({"sanskrit", "hindi", "marathi"})

# Multi-word forms that the lexer fuses post-tokenization.
MULTI_WORD_ALIASES: Dict[Tuple[str, ...], str] = {
    ("नहीं", "तो"):      "Else",
    ("के", "लिए"):        "For",
    ("सिद्ध", "करो"):     "Prove",
    ("सिद्ध", "करा"):     "Prove",
    ("समान्तर", "प्रति"): "Parallel",
}

# ---------------------------------------------------------------------------
# SOV word-order helpers
# ---------------------------------------------------------------------------

# Verb-final token kinds: these appear at the END of the statement in SOV langs.
_SOV_VERB_FINAL_KINDS = frozenset({"Return", "Print", "Assert", "Prove"})

# Build: spelling -> kind, for every non-English SOV verb-final keyword.
# Single-word forms only here (multi-word forms handled separately below).
_VERB_FINAL_SPELLINGS: Dict[str, str] = {}
for _kind in _SOV_VERB_FINAL_KINDS:
    for _lang, _spelling in ALIASES[_kind].items():
        if _lang != "english" and " " not in _spelling:
            _VERB_FINAL_SPELLINGS[_spelling] = _kind

# Multi-word verb-final spellings: "WORD1 WORD2" -> kind
_MULTI_WORD_VERB_FINALS: Dict[str, str] = {
    " ".join(pair): kind
    for pair, kind in MULTI_WORD_ALIASES.items()
    if kind in _SOV_VERB_FINAL_KINDS
}


def _is_word_char(c: str) -> bool:
    if c.isalnum() or c == "_":
        return True
    cp = ord(c)
    return (0x0900 <= cp <= 0x097F) or (0x0A8E0 <= cp <= 0x0A8FF)


def _last_word(s: str) -> Tuple[str, int]:
    """Return (word, start_index) for the last contiguous word in s."""
    end = len(s)
    while end > 0 and not _is_word_char(s[end - 1]):
        end -= 1
    start = end
    while start > 0 and _is_word_char(s[start - 1]):
        start -= 1
    return s[start:end], start


def _try_normalize_verbfinal_line(line: str) -> str:
    """
    If `line` (in a SOV language) ends with a verb-final keyword followed by ;,
    reorder it to English SVO: put the English verb first.

    'n पुनरागम;'      -> 'return n;'
    '  x लिखो;'       -> '  print x;'
    '  x सिद्ध करो;'  -> '  prove x;'   (multi-word Prove)
    """
    stripped = line.rstrip()  # strip all trailing whitespace/newlines
    trailing = line[len(stripped):]  # re-append after reorder (e.g. "\n")

    if not stripped.endswith(";"):
        return line

    # Capture indent; work on body (indent-free "expr VERB" string)
    leading = stripped[: len(stripped) - len(stripped.lstrip())]
    before_semi = stripped[:-1].rstrip()          # "  expr VERB"
    body = before_semi[len(leading):].rstrip()    # "expr VERB" (no indent)

    if not body:
        return line

    # --- single-word verb-final ---
    verb, verb_start = _last_word(body)
    if verb and verb in _VERB_FINAL_SPELLINGS:
        kind = _VERB_FINAL_SPELLINGS[verb]
        expr = body[:verb_start].rstrip()
        english_verb = ALIASES[kind]["english"]
        if expr:
            return f"{leading}{english_verb} {expr};{trailing}"
        return f"{leading}{english_verb};{trailing}"

    # --- two-word verb-final (e.g. "सिद्ध करो", "सिद्ध करा") ---
    before_last = body[:verb_start].rstrip()
    if before_last:
        word2 = verb
        word1, word1_start = _last_word(before_last)
        two_word = f"{word1} {word2}"
        if two_word in _MULTI_WORD_VERB_FINALS:
            kind = _MULTI_WORD_VERB_FINALS[two_word]
            expr = before_last[:word1_start].rstrip()
            english_verb = ALIASES[kind]["english"]
            if expr:
                return f"{leading}{english_verb} {expr};{trailing}"
            return f"{leading}{english_verb};{trailing}"

    return line


def _normalize_sov_to_svo(source: str, src_lang: str) -> str:
    """
    Pre-processing: if source is in a SOV language, reorder verb-final
    statements to SVO (English word order) so that keyword substitution
    produces the correct target output.

    Handles:
      - Verb-final return/print/assert/prove statements (line-level).
      - Hindi for-range:  VAR के लिए START से END तक {  →  for VAR from START to END {
    """
    if src_lang not in SOV_LANGS:
        return source

    # 1. Line-level verb-final reorder.
    trailing_nl = source.endswith("\n")
    source = "\n".join(
        _try_normalize_verbfinal_line(ln)
        for ln in source.splitlines(keepends=False)
    )
    if trailing_nl:
        source += "\n"

    # 2. Hindi for-range: VAR के लिए START से END तक {
    if src_lang == "hindi":
        # Multi-word के लिए = For.  Regex: IDENT (whitespace) के लिए EXPR से EXPR तक (ws) {
        pat = re.compile(
            r'([ \t]*)(\w+)([ \t]+)के\s+लिए([ \t]+)(\S+)([ \t]+)से([ \t]+)(\S+)([ \t]+)तक([ \t]*)\{'
        )
        def _fix_for(m: re.Match) -> str:
            indent, var, _, _, start, _, _, end, _, _, = m.groups()
            return f"{indent}for {var} from {start} to {end} {{"
        source = pat.sub(_fix_for, source)

    return source


def _convert_svo_to_sov(source: str, target_lang: str) -> str:
    """
    Post-processing: if target is a SOV language, reorder SVO verb-initial
    statements to verb-final SOV.

    Handles:
      - Verb-initial return/print/assert/prove statements (line-level).
      - Hindi for-range:  for VAR from START to END {  →  VAR के लिए START से END तक {
    """
    if target_lang not in SOV_LANGS:
        return source

    # Build lookup: english_verb -> target spelling
    target_verb: Dict[str, str] = {
        ALIASES[k]["english"]: ALIASES[k][target_lang]
        for k in _SOV_VERB_FINAL_KINDS
        if target_lang in ALIASES[k]
    }

    result_lines = []
    for line in source.splitlines(keepends=False):
        stripped = line.rstrip()
        if not stripped.endswith(";"):
            result_lines.append(line)
            continue
        leading = stripped[: len(stripped) - len(stripped.lstrip())]
        body = stripped.lstrip()

        # Check if the line starts with one of the target verbs (as already
        # substituted by translate()) or their English originals.
        matched = False
        for en_verb, sov_verb in target_verb.items():
            # The translate() step will have already replaced 'return' with
            # e.g. 'लौटाओ'.  So we look for either form.
            for look_for in (sov_verb, en_verb):
                if body.startswith(look_for + " ") or body.startswith(look_for + "\t"):
                    expr = body[len(look_for):].strip().rstrip(";")
                    # Skip if there's no expression (e.g. bare `return;`)
                    if expr:
                        result_lines.append(f"{leading}{expr} {sov_verb};")
                    else:
                        result_lines.append(line)
                    matched = True
                    break
            if matched:
                break
        if not matched:
            result_lines.append(line)

    trailing_nl = source.endswith("\n")
    source = "\n".join(result_lines)
    if trailing_nl:
        source += "\n"

    # Hindi for-range:  for VAR from START to END {  →  VAR के लिए START से END तक {
    if target_lang == "hindi":
        for_kw  = ALIASES["For"]["hindi"]   # "के लिए"
        from_kw = ALIASES["From"]["hindi"]  # "से"
        to_kw   = ALIASES["To"]["hindi"]    # "तक"
        # At this point translate() has already substituted: for→के लिए, from→से, to→तक
        # So the (wrong) text looks like: "के लिए VAR से START तक END {"
        pat = re.compile(
            re.escape(for_kw) + r'([ \t]+)(\w+)([ \t]+)' +
            re.escape(from_kw) + r'([ \t]+)(\S+)([ \t]+)' +
            re.escape(to_kw) + r'([ \t]+)(\S+)([ \t]*)\{'
        )
        def _fix_for_sov(m: re.Match) -> str:
            _, var, _, _, start, _, _, end, _ = m.groups()
            return f"{var} {for_kw} {start} {from_kw} {end} {to_kw} {{"
        source = pat.sub(_fix_for_sov, source)

    return source


# ---------------------------------------------------------------------------
# Core keyword translator
# ---------------------------------------------------------------------------

def build_reverse_lookup() -> Dict[str, Tuple[str, str]]:
    rev: Dict[str, Tuple[str, str]] = {}
    for kind, langs in ALIASES.items():
        for lang, spelling in langs.items():
            rev[spelling] = (kind, lang)
    return rev


def detect_pragma_lang(source: str) -> Optional[str]:
    """Return the language declared in the first `// vani-lang: <name>` pragma, or None."""
    for line in source.splitlines():
        stripped = line.lstrip("/").strip()
        for prefix in ("vani-lang:", "vani-lang :"):
            if stripped.startswith(prefix):
                lang = stripped[len(prefix):].strip().lower()
                if lang in SUPPORTED_LANGS:
                    return lang
    return None


def extract_keyword_tokens(source: str) -> List[str]:
    """Return the ordered sequence of TokenKind names found in source."""
    rev = build_reverse_lookup()
    tokens: List[str] = []
    i = 0
    n = len(source)
    while i < n:
        c = source[i]
        if c == "/" and i + 1 < n and source[i + 1] == "/":
            j = source.find("\n", i)
            i = n if j == -1 else j
            continue
        if c == '"':
            i += 1
            while i < n and source[i] != '"':
                if source[i] == "\\" and i + 1 < n:
                    i += 2
                    continue
                i += 1
            if i < n:
                i += 1
            continue
        if _is_word_char(c):
            j = i
            while j < n and _is_word_char(source[j]):
                j += 1
            word = source[i:j]
            k = j
            while k < n and source[k] in (" ", "\t"):
                k += 1
            second = None
            second_end = k
            if k < n and _is_word_char(source[k]):
                m = k
                while m < n and _is_word_char(source[m]):
                    m += 1
                second = source[k:m]
                second_end = m
            if second is not None and (word, second) in MULTI_WORD_ALIASES:
                tokens.append(MULTI_WORD_ALIASES[(word, second)])
                i = second_end
                continue
            if word in rev:
                tokens.append(rev[word][0])
            i = j
            continue
        i += 1
    return tokens


def _translate_keywords(source: str, target_lang: str) -> str:
    """
    Pure keyword substitution (no word-order changes).
    Rewrites the `// vani-lang:` pragma to target_lang.
    """
    assert target_lang in SUPPORTED_LANGS, f"unknown target {target_lang!r}"
    rev = build_reverse_lookup()
    out: List[str] = []
    i = 0
    n = len(source)
    while i < n:
        c = source[i]
        # Line comment — pass through, rewriting vani-lang pragma.
        if c == "/" and i + 1 < n and source[i + 1] == "/":
            j = source.find("\n", i)
            if j == -1:
                j = n
            line = source[i:j]
            stripped = line.lstrip("/").strip()
            if stripped.startswith("vani-lang:") or stripped.startswith("vani-lang :"):
                leading = line[: len(line) - len(line.lstrip("/ \t"))]
                out.append(f"{leading}vani-lang: {target_lang}")
            else:
                out.append(line)
            i = j
            continue
        # String literal — copy through, handling escapes.
        if c == '"':
            out.append(c)
            i += 1
            while i < n and source[i] != '"':
                if source[i] == "\\" and i + 1 < n:
                    out.append(source[i:i + 2])
                    i += 2
                    continue
                out.append(source[i])
                i += 1
            if i < n:
                out.append(source[i])
                i += 1
            continue
        # Word token.
        if _is_word_char(c):
            j = i
            while j < n and _is_word_char(source[j]):
                j += 1
            word = source[i:j]
            # Multi-word lookahead.
            k = j
            while k < n and source[k] in (" ", "\t"):
                k += 1
            second = None
            second_end = k
            if k < n and _is_word_char(source[k]):
                m = k
                while m < n and _is_word_char(source[m]):
                    m += 1
                second = source[k:m]
                second_end = m
            if second is not None:
                key = (word, second)
                if key in MULTI_WORD_ALIASES:
                    kind = MULTI_WORD_ALIASES[key]
                    if kind in ALIASES:
                        out.append(ALIASES[kind][target_lang])
                        i = second_end
                        continue
            if word in rev:
                kind, _ = rev[word]
                if kind in ALIASES:
                    out.append(ALIASES[kind][target_lang])
                    i = j
                    continue
            out.append(word)
            i = j
            continue
        out.append(c)
        i += 1
    return "".join(out)


def translate(source: str, target_lang: str, src_lang: Optional[str] = None) -> str:
    """
    Translate source to target_lang.

    Steps:
      1. Detect source language (from pragma or argument).
      2. If source is SOV, normalize verb-final statements to SVO.
      3. Substitute keywords to target_lang spellings.
      4. If target is SOV, convert SVO verb-initial statements to SOV.
      5. Rewrite pragma.
    """
    effective_src = src_lang or detect_pragma_lang(source) or "english"
    text = _normalize_sov_to_svo(source, effective_src)
    text = _translate_keywords(text, target_lang)
    text = _convert_svo_to_sov(text, target_lang)
    return text


def verify_roundtrip(source: str, target_lang: str, src_lang: Optional[str]) -> Tuple[bool, str]:
    """
    Translate source → target_lang → src_lang, then compare the
    keyword-token sequences of the original and the double-translated
    result. Returns (passed, message).
    """
    effective_src = src_lang or detect_pragma_lang(source) or "english"
    intermediate = translate(source, target_lang, effective_src)
    back = translate(intermediate, effective_src, target_lang)
    orig_tokens = extract_keyword_tokens(source)
    back_tokens = extract_keyword_tokens(back)
    if orig_tokens == back_tokens:
        return True, (
            f"round-trip ok: {effective_src} -> {target_lang} -> {effective_src} "
            f"({len(orig_tokens)} keyword tokens preserved)"
        )
    diffs = [
        f"  pos {i}: {a!r} -> {b!r}"
        for i, (a, b) in enumerate(zip(orig_tokens, back_tokens))
        if a != b
    ]
    if len(orig_tokens) != len(back_tokens):
        diffs.append(
            f"  token count: {len(orig_tokens)} original vs {len(back_tokens)} after round-trip"
        )
    return False, "round-trip FAILED:\n" + "\n".join(diffs)


def list_keywords() -> str:
    """Return a markdown table of all keyword aliases."""
    langs = ["english", "sanskrit", "hindi", "marathi", "mandarin"]
    header = "| TokenKind | " + " | ".join(l.capitalize() for l in langs) + " |"
    sep    = "|-----------|" + "|".join("-" * (len(l) + 2) for l in langs) + "|"
    rows = [header, sep]
    for kind, mapping in sorted(ALIASES.items()):
        cells = " | ".join(mapping.get(l, "--") for l in langs)
        rows.append(f"| {kind:<12} | {cells} |")
    return "\n".join(rows)


# ---------------------------------------------------------------------------
# LLM translation for comments, strings, and identifiers
# ---------------------------------------------------------------------------

_LANG_NAMES = {
    "english":  "English",
    "sanskrit": "Sanskrit",
    "hindi":    "Hindi",
    "marathi":  "Marathi",
    "mandarin": "Mandarin Chinese",
}


def _llm_prompt(text: str, src_lang: str, target_lang: str, content_type: str) -> str:
    src_name = _LANG_NAMES.get(src_lang, src_lang)
    tgt_name = _LANG_NAMES.get(target_lang, target_lang)
    if content_type == "comment text":
        # Explicit framing prevents models from generating code instead of translating.
        return (
            f"Translate this source code comment from {src_name} to {tgt_name}.\n"
            f"The input is natural language text written as a comment inside a computer program.\n"
            f"Output ONLY the translated natural language sentence -- no code, no quotes, "
            f"no surrounding punctuation.\n"
            f"Keep any technical terms, variable names, numbers, and identifiers unchanged.\n\n"
            f"Comment: {text.strip()}"
        )
    return (
        f"Translate the following {content_type} from {src_name} to {tgt_name}.\n"
        f"Rules:\n"
        f"- Translate only the natural language content.\n"
        f"- Preserve all technical terms, variable names, code references, "
        f"and special characters exactly as-is.\n"
        f"- Output ONLY the translated text, nothing else.\n\n"
        f"Text:\n{text}"
    )


def _call_anthropic(text: str, src_lang: str, target_lang: str,
                    content_type: str, model: str) -> str:
    try:
        import anthropic as _anthropic
    except ImportError:
        raise RuntimeError(
            "anthropic package not installed. Run: pip install 'anthropic>=0.20'"
        )
    prompt = _llm_prompt(text, src_lang, target_lang, content_type)

    # Modern SDK (v0.20+): has Anthropic class with messages API.
    if hasattr(_anthropic, "Anthropic"):
        client = _anthropic.Anthropic()
        msg = client.messages.create(
            model=model,
            max_tokens=4096,
            messages=[{"role": "user", "content": prompt}],
        )
        return msg.content[0].text.strip()

    # Legacy SDK (v0.2.x): uses Client + completion() + HUMAN_PROMPT sentinel.
    if hasattr(_anthropic, "Client") and hasattr(_anthropic, "HUMAN_PROMPT"):
        import os
        api_key = os.environ.get("ANTHROPIC_API_KEY")
        if not api_key:
            raise RuntimeError(
                "ANTHROPIC_API_KEY environment variable not set. "
                "Set it or upgrade: pip install 'anthropic>=0.20'"
            )
        client = _anthropic.Client(api_key=api_key)
        full_prompt = (
            f"{_anthropic.HUMAN_PROMPT} {prompt}{_anthropic.AI_PROMPT}"
        )
        resp = client.completion(
            prompt=full_prompt,
            model=model if model.startswith("claude-v") else "claude-v1",
            max_tokens_to_sample=4096,
        )
        return resp["completion"].strip()

    raise RuntimeError(
        "Unrecognized anthropic package. Run: pip install 'anthropic>=0.20'"
    )


def _call_openai(text: str, src_lang: str, target_lang: str,
                 content_type: str, model: str) -> str:
    try:
        import openai as _openai
    except ImportError:
        raise RuntimeError(
            "openai package not installed. Run: pip install openai"
        )
    client = _openai.OpenAI()
    resp = client.chat.completions.create(
        model=model,
        messages=[{"role": "user", "content": _llm_prompt(text, src_lang, target_lang, content_type)}],
    )
    return resp.choices[0].message.content.strip()


def _call_ollama(text: str, src_lang: str, target_lang: str,
                 content_type: str, model: str,
                 host: str = "http://localhost:11434",
                 timeout: int = 60) -> str:
    try:
        import requests as _requests
    except ImportError:
        raise RuntimeError(
            "requests package not installed. Run: pip install requests"
        )
    payload = {
        "model": model,
        "prompt": _llm_prompt(text, src_lang, target_lang, content_type),
        "stream": False,
    }
    resp = _requests.post(f"{host}/api/generate", json=payload, timeout=timeout)
    resp.raise_for_status()
    return resp.json()["response"].strip()


def _llm_translate_chunk(text: str, src_lang: str, target_lang: str,
                          content_type: str,
                          llm: str, model: str,
                          ollama_host: str = "http://localhost:11434",
                          llm_timeout: int = 60) -> str:
    """Call the chosen LLM backend to translate a natural language chunk."""
    if not text.strip():
        return text
    if llm == "anthropic":
        return _call_anthropic(text, src_lang, target_lang, content_type, model)
    if llm == "openai":
        return _call_openai(text, src_lang, target_lang, content_type, model)
    if llm == "ollama":
        return _call_ollama(text, src_lang, target_lang, content_type, model,
                            ollama_host, llm_timeout)
    raise ValueError(f"unknown LLM backend: {llm!r}")


def _split_identifier(ident: str) -> List[str]:
    """
    Split a camelCase or snake_case identifier into constituent words.
    E.g. 'safeDiv' -> ['safe', 'Div'], 'safe_div' -> ['safe', 'div'].
    """
    # Split on underscores first
    parts = ident.split("_")
    words: List[str] = []
    for part in parts:
        if not part:
            continue
        # Split camelCase
        sub = re.sub(r"([A-Z]+)([A-Z][a-z])", r"\1_\2", part)
        sub = re.sub(r"([a-z\d])([A-Z])", r"\1_\2", sub)
        words.extend(sub.split("_"))
    return words


def translate_with_llm(
    source: str,
    target_lang: str,
    src_lang: Optional[str],
    llm: str,
    model: str,
    translate_identifiers: bool = False,
    ollama_host: str = "http://localhost:11434",
    llm_timeout: int = 60,
) -> str:
    """
    Translate a vani source file using both keyword substitution and LLM
    translation for natural language content.

    Translates:
    - Keywords: via the keyword substitution table (always).
    - Line comments (// ...): via LLM.
    - String literals ("..."): via LLM.
    - Identifiers (user-defined names): via LLM when translate_identifiers=True.
    """
    effective_src = src_lang or detect_pragma_lang(source) or "english"

    # Step 1: keyword translation (with SOV reordering).
    result = translate(source, target_lang, effective_src)

    # Step 2: translate line comments.
    def _translate_comment(m: re.Match) -> str:
        prefix = m.group(1)   # // or //
        content = m.group(2)  # the text after //

        # Skip pragma lines.
        stripped = content.strip()
        if stripped.startswith("vani-lang:") or stripped.startswith("श्री।"):
            return m.group(0)

        try:
            translated = _llm_translate_chunk(
                content, effective_src, target_lang, "comment text",
                llm, model, ollama_host, llm_timeout
            )
            return prefix + " " + translated.lstrip()
        except Exception as e:
            print(f"  [llm] comment translation failed: {e}", file=sys.stderr)
            return m.group(0)

    result = re.sub(r"(//)(.*)", _translate_comment, result)

    # Step 3: translate string literals.
    def _translate_string(m: re.Match) -> str:
        content = m.group(1)  # content inside quotes
        try:
            translated = _llm_translate_chunk(
                content, effective_src, target_lang, "string literal",
                llm, model, ollama_host, llm_timeout
            )
            # Ensure no unescaped quotes sneak in.
            translated = translated.replace('"', '\\"')
            return f'"{translated}"'
        except Exception as e:
            print(f"  [llm] string translation failed: {e}", file=sys.stderr)
            return m.group(0)

    # Match string literals but not escaped quotes inside them.
    result = re.sub(r'"((?:[^"\\]|\\.)*)"', _translate_string, result)

    # Step 4: translate identifiers (optional).
    if translate_identifiers:
        rev = build_reverse_lookup()
        # Collect all unique user-defined identifiers (not keywords, not all-caps consts).
        idents = set(re.findall(r'\b([a-zA-Z_][a-zA-Z0-9_]*)\b', result))
        # Filter out keywords and trivial names.
        idents = {
            w for w in idents
            if w not in rev
            and w not in ALIASES
            and len(w) >= 3
            and not w.isupper()
        }

        ident_map: Dict[str, str] = {}
        if idents:
            # Batch all identifiers into one LLM call to save API round-trips.
            batch = "\n".join(sorted(idents))
            try:
                translated_batch = _llm_translate_chunk(
                    batch, effective_src, target_lang,
                    "list of programming identifiers (one per line -- translate each separately, preserve the same line count)",
                    llm, model, ollama_host, llm_timeout
                )
                translated_lines = translated_batch.splitlines()
                sorted_idents = sorted(idents)
                for orig, xlat in zip(sorted_idents, translated_lines):
                    # Sanitize: identifiers must be word-chars only.
                    clean = re.sub(r"[^\w]", "_", xlat.strip())
                    if clean and clean != orig:
                        ident_map[orig] = clean
            except Exception as e:
                print(f"  [llm] identifier translation failed: {e}", file=sys.stderr)

        if ident_map:
            def _replace_ident(m: re.Match) -> str:
                return ident_map.get(m.group(0), m.group(0))
            # Sort by length desc so longer identifiers match before shorter subsets.
            pattern = r'\b(' + "|".join(
                re.escape(k) for k in sorted(ident_map, key=len, reverse=True)
            ) + r')\b'
            result = re.sub(pattern, _replace_ident, result)

    return result


# ---------------------------------------------------------------------------
# File-level translation
# ---------------------------------------------------------------------------

def _translate_file(
    src_path: Path,
    target_lang: str,
    out_path: Optional[Path],
    inplace: bool,
    add_sri_header: bool,
    verify: bool,
    src_lang: Optional[str],
    verbose: bool,
    llm: Optional[str] = None,
    llm_model: str = "claude-haiku-4-5-20251001",
    translate_identifiers: bool = False,
    ollama_host: str = "http://localhost:11434",
    llm_timeout: int = 60,
) -> bool:
    source = src_path.read_text(encoding="utf-8")
    if verify:
        ok, msg = verify_roundtrip(source, target_lang, src_lang)
        prefix = src_path.name + ": " if verbose else ""
        print(f"{prefix}{msg}", file=sys.stderr if not ok else sys.stdout)
        if not ok:
            return False

    if llm:
        translated = translate_with_llm(
            source, target_lang, src_lang, llm, llm_model,
            translate_identifiers, ollama_host, llm_timeout
        )
    else:
        translated = translate(source, target_lang, src_lang)

    if add_sri_header and target_lang in _IA_DEVANAGARI:
        if not translated.lstrip().startswith("// श्री।"):
            translated = (
                f"// श्री।\n"
                f"// vani-lang: {target_lang}\n"
                f"//\n"
                + translated
            )

    if inplace:
        backup = src_path.with_suffix(src_path.suffix + ".bak")
        backup.write_text(source, encoding="utf-8")
        src_path.write_text(translated, encoding="utf-8")
        if verbose:
            print(f"  {src_path}  (backup -> {backup.name})")
    elif out_path is not None:
        if out_path.is_dir():
            dest = out_path / src_path.name
        else:
            dest = out_path
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_text(translated, encoding="utf-8")
        if verbose:
            print(f"  {src_path} -> {dest}")
    else:
        sys.stdout.write(translated)
    return True


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def main() -> int:
    parser = argparse.ArgumentParser(
        description=(
            "Translate a .vani source file's keywords between "
            "English, Sanskrit, Hindi, Marathi, and Mandarin. "
            "SOV word-order (verb-final statements and Hindi for-range) "
            "is reordered automatically."
        )
    )
    parser.add_argument(
        "input",
        type=Path,
        nargs="?",
        help="source .vani file or directory (with --batch)",
    )
    parser.add_argument(
        "--from",
        dest="src_lang",
        choices=SUPPORTED_LANGS,
        default=None,
        help=(
            "source language -- optional; auto-detected from the "
            "`// vani-lang:` pragma if not provided"
        ),
    )
    parser.add_argument(
        "--to",
        dest="target_lang",
        choices=SUPPORTED_LANGS,
        default=None,
        help="target language (required unless --list-keywords)",
    )
    parser.add_argument(
        "-o", "--output",
        type=Path,
        default=None,
        help="output file or directory (default: stdout; directory used with --batch)",
    )
    parser.add_argument(
        "--inplace", "-i",
        action="store_true",
        help="translate file in-place, saving original as <file>.bak",
    )
    parser.add_argument(
        "--batch",
        action="store_true",
        help="translate all .vani files under INPUT directory tree",
    )
    parser.add_argument(
        "--verify",
        action="store_true",
        help=(
            "after translation, translate back to source language and "
            "verify the keyword token sequence is preserved"
        ),
    )
    parser.add_argument(
        "--list-keywords",
        action="store_true",
        help="print all keyword aliases as a markdown table and exit",
    )
    parser.add_argument(
        "--add-sri-header",
        action="store_true",
        help=(
            "prepend `// श्री।` and `// vani-lang: <lang>` when "
            "translating to an Indo-Aryan Devanagari language"
        ),
    )

    # LLM options
    llm_group = parser.add_argument_group("LLM translation (comments, strings, identifiers)")
    llm_group.add_argument(
        "--llm",
        choices=("anthropic", "openai", "ollama"),
        default=None,
        metavar="BACKEND",
        help=(
            "Enable LLM translation for comments and string literals. "
            "Choices: anthropic, openai, ollama. "
            "Requires the corresponding Python package and API credentials."
        ),
    )
    llm_group.add_argument(
        "--llm-model",
        default=None,
        metavar="MODEL",
        help=(
            "Model name to use with --llm. "
            "Defaults: anthropic=claude-haiku-4-5-20251001, "
            "openai=gpt-4o-mini, ollama=llama3.2"
        ),
    )
    llm_group.add_argument(
        "--translate-identifiers",
        action="store_true",
        help=(
            "Also translate user-defined identifiers via LLM (requires --llm). "
            "All unique identifiers are batched into one API call."
        ),
    )
    llm_group.add_argument(
        "--ollama-host",
        default="http://localhost:11434",
        metavar="URL",
        help="Ollama server URL (default: http://localhost:11434)",
    )
    llm_group.add_argument(
        "--llm-timeout",
        type=int,
        default=60,
        metavar="SECONDS",
        help=(
            "HTTP timeout for LLM requests in seconds (default: 60). "
            "Increase for slow CPU-only Ollama models."
        ),
    )

    parser.add_argument(
        "--verbose", "-v",
        action="store_true",
        help="print per-file progress to stderr",
    )
    args = parser.parse_args()

    if args.list_keywords:
        print(list_keywords())
        return 0

    if args.input is None:
        parser.error("INPUT is required unless --list-keywords is set")
    if args.target_lang is None:
        parser.error("--to is required")
    if args.inplace and args.output is not None:
        parser.error("--inplace and --output are mutually exclusive")
    if args.translate_identifiers and not args.llm:
        parser.error("--translate-identifiers requires --llm")

    # Default model per backend
    llm_model = args.llm_model
    if args.llm and not llm_model:
        llm_model = {
            "anthropic": "claude-haiku-4-5-20251001",
            "openai":    "gpt-4o-mini",
            "ollama":    "llama3.2",
        }[args.llm]

    common = dict(
        target_lang=args.target_lang,
        out_path=args.output,
        inplace=args.inplace,
        add_sri_header=args.add_sri_header,
        verify=args.verify,
        src_lang=args.src_lang,
        verbose=args.verbose,
        llm=args.llm,
        llm_model=llm_model,
        translate_identifiers=args.translate_identifiers,
        ollama_host=args.ollama_host,
        llm_timeout=args.llm_timeout,
    )

    if args.batch:
        if not args.input.is_dir():
            parser.error("--batch requires INPUT to be a directory")
        files = list(args.input.rglob("*.vani"))
        if not files:
            print(f"no .vani files found under {args.input}", file=sys.stderr)
            return 1
        ok_count = 0
        for f in sorted(files):
            ok = _translate_file(f, **common)
            if ok:
                ok_count += 1
        print(f"{ok_count}/{len(files)} files translated successfully.", file=sys.stderr)
        return 0 if ok_count == len(files) else 1
    else:
        if not args.input.exists():
            print(f"input file not found: {args.input}", file=sys.stderr)
            return 1
        ok = _translate_file(args.input, **common)
        return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
