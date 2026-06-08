#!/usr/bin/env python3
"""
vani_translate — translate a .vani source file's keywords between
                English, Sanskrit, Hindi, and Marathi.

B.1 v1 — token-level keyword substitution (the simplest correct
shape). Preserves identifiers, comments, strings, whitespace,
operators, and numbers verbatim. Only the keywords change.

Usage:
    python3 tools/vani_translate.py --from english --to sanskrit \
        examples/language/english/basics.vani -o out.vani

    # Then verify the translation compiles to the same AST:
    diff <(vanic ast examples/language/english/basics.vani) \
         <(vanic ast out.vani)

What this v1 does NOT do (deferred to later phases):
  - SOV word-order reshape. The output keeps the source's word
    order; only keywords are substituted. A pure-Sanskrit source
    with verb-final shapes won't be reshaped to keyword-first when
    translated to English.
  - Identifier translation. User-named functions, vars, and types
    stay in whatever language the author wrote them in. Mixing is
    explicitly allowed.
  - Comment translation. Comments are preserved verbatim — the
    user controls their language.
  - The 4 still-English-only keywords (`extern`, `type`, `intent`,
    `invariant`) pass through unchanged in any direction.

Round-trip parity: english → sanskrit → english should produce a
file that compiles to the same AST as the original (modulo
whitespace and which alias was picked when multiple existed).
"""

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Dict, List, Optional, Tuple


# Single-word keyword alias table. Each TokenKind maps to one
# canonical spelling per language. Source: src/lexer.rs:222-365
# (devanagari_keyword) + the README alias table.
#
# When multiple aliases exist for a (TokenKind, language) pair, the
# table picks the most natural / most-common spelling. The translator
# can be configured later to preserve the source's specific choice
# (e.g. कार्य vs फलन for "fn" in Hindi).
ALIASES: Dict[str, Dict[str, str]] = {
    # Declarations
    "Fn":         {"english": "fn",       "sanskrit": "कार्य",       "hindi": "फलन",       "marathi": "कार्य",        "mandarin": "函数"},
    "Let":        {"english": "let",      "sanskrit": "माना",       "hindi": "माना",       "marathi": "मान",          "mandarin": "让"},
    "Struct":     {"english": "struct",   "sanskrit": "संरचना",      "hindi": "संरचना",     "marathi": "संरचना",      "mandarin": "结构"},
    "Enum":       {"english": "enum",     "sanskrit": "विकल्प",      "hindi": "गणन",        "marathi": "गणन",          "mandarin": "枚举"},
    "Const":      {"english": "const",    "sanskrit": "स्थिर",        "hindi": "स्थिर",      "marathi": "स्थिर",        "mandarin": "常量"},

    # Visibility / modules / imports
    "Pub":        {"english": "pub",      "sanskrit": "सार्वजनिक",   "hindi": "सार्वजनिक",  "marathi": "सार्वजनिक",  "mandarin": "公开"},
    "Module":     {"english": "module",   "sanskrit": "खण्ड",        "hindi": "मॉड्यूल",    "marathi": "मॉड्यूल",     "mandarin": "模块"},
    "Use":        {"english": "use",      "sanskrit": "उपयोग",       "hindi": "उपयोग",      "marathi": "उपयोग",        "mandarin": "使用"},
    "As":         {"english": "as",       "sanskrit": "यथा",         "hindi": "यथा",        "marathi": "यथा",          "mandarin": "作为"},

    # Control flow
    "Return":     {"english": "return",   "sanskrit": "पुनरागम",     "hindi": "लौटाओ",      "marathi": "परत",          "mandarin": "返回"},
    "If":         {"english": "if",       "sanskrit": "यदि",         "hindi": "अगर",        "marathi": "जर",           "mandarin": "如果"},
    "Else":       {"english": "else",     "sanskrit": "अन्यथा",      "hindi": "वरना",       "marathi": "नाहीतर",       "mandarin": "否则"},
    "While":      {"english": "while",    "sanskrit": "यावत्",        "hindi": "जबतक",       "marathi": "जोपर्यंत",     "mandarin": "当"},
    "For":        {"english": "for",      "sanskrit": "प्रति",        "hindi": "के लिए",     "marathi": "साठी",         "mandarin": "对于"},
    "In":         {"english": "in",       "sanskrit": "में",          "hindi": "में",         "marathi": "में",          "mandarin": "in"},
    "From":       {"english": "from",     "sanskrit": "से",          "hindi": "से",          "marathi": "से",           "mandarin": "从"},
    "To":         {"english": "to",       "sanskrit": "तक",          "hindi": "तक",          "marathi": "तक",           "mandarin": "到"},
    "Break":      {"english": "break",    "sanskrit": "विराम",       "hindi": "रुको",        "marathi": "थांब",         "mandarin": "中断"},
    "Continue":   {"english": "continue", "sanskrit": "अग्रे",       "hindi": "आगे",         "marathi": "पुढे",         "mandarin": "继续"},
    "Then":       {"english": "then",     "sanskrit": "तदा",         "hindi": "तो",          "marathi": "तर",           "mandarin": "那么"},

    # References
    "Ref":        {"english": "ref",      "sanskrit": "दृष्ट्या",    "hindi": "देखो",        "marathi": "पहा",          "mandarin": "引用"},
    "Mut":        {"english": "mut",      "sanskrit": "परिवर्तनीय",  "hindi": "परिवर्तनीय",  "marathi": "बदल",          "mandarin": "可变"},

    # Matching
    "Match":      {"english": "match",    "sanskrit": "मेल",         "hindi": "मिलान",       "marathi": "जुळवा",        "mandarin": "匹配"},

    # Verification
    "Assert":     {"english": "assert",   "sanskrit": "सिद्धम्",      "hindi": "सुनिश्चित",  "marathi": "खात्री",       "mandarin": "断言"},
    "Prove":      {"english": "prove",    "sanskrit": "प्रमाण",      "hindi": "सिद्ध करो",   "marathi": "सिद्ध करा",    "mandarin": "证明"},
    "Requires":   {"english": "requires", "sanskrit": "अपेक्षित",    "hindi": "चाहिए",      "marathi": "पाहिजे",        "mandarin": "要求"},
    "Ensures":    {"english": "ensures",  "sanskrit": "सुनिश्चयित",  "hindi": "निश्चित",     "marathi": "निश्चित",      "mandarin": "保证"},

    # Bool / print
    "True":       {"english": "true",     "sanskrit": "सत्य",        "hindi": "सत्य",        "marathi": "सत्य",         "mandarin": "真"},
    "False":      {"english": "false",    "sanskrit": "असत्य",       "hindi": "असत्य",      "marathi": "असत्य",        "mandarin": "假"},
    "Print":      {"english": "print",    "sanskrit": "लिख",         "hindi": "लिखो",        "marathi": "लिहा",         "mandarin": "打印"},

    # Purity / parallelism
    "Pure":       {"english": "pure",     "sanskrit": "शुद्ध",       "hindi": "शुद्ध",       "marathi": "शुद्ध",        "mandarin": "纯"},
    "Parallel":   {"english": "parallel", "sanskrit": "समानांतर",    "hindi": "समानांतर",   "marathi": "समानांतर",     "mandarin": "并行"},
    "Reduce":     {"english": "reduce",   "sanskrit": "संक्षेप",     "hindi": "संक्षेप",     "marathi": "संक्षेप",      "mandarin": "reduce"},
    "With":       {"english": "with",     "sanskrit": "सह",          "hindi": "सह",          "marathi": "सह",           "mandarin": "with"},

    # Interfaces / methods
    "Interface":  {"english": "interface", "sanskrit": "संकेत",      "hindi": "संकेत",       "marathi": "संकेत",       "mandarin": "接口"},
    "Implement":  {"english": "implement", "sanskrit": "कार्यान्वित","hindi": "कार्यान्वित","marathi": "कार्यान्वित", "mandarin": "实现"},
    "Methods":    {"english": "methods",   "sanskrit": "विधि",        "hindi": "विधि",        "marathi": "विधि",        "mandarin": "方法"},

    # Bounds
    "Where":      {"english": "where",    "sanskrit": "यत्र",         "hindi": "जहाँ",        "marathi": "जिथे",         "mandarin": "其中"},
    "Is":         {"english": "is",       "sanskrit": "अस्ति",        "hindi": "है",          "marathi": "आहे",          "mandarin": "is"},

    # Concurrency
    "Try":        {"english": "try",      "sanskrit": "प्रयास",       "hindi": "प्रयास",     "marathi": "प्रयास",      "mandarin": "尝试"},
    "Task":       {"english": "task",     "sanskrit": "नियोग",        "hindi": "नियोग",      "marathi": "नियोग",        "mandarin": "任务"},
    "Join":       {"english": "join",     "sanskrit": "संयोजन",       "hindi": "संयोजन",     "marathi": "संयोजन",       "mandarin": "等待"},

    # Embedded — Layer 1.1 + 5
    "Unsafe":     {"english": "unsafe",   "sanskrit": "असुरक्षित",    "hindi": "असुरक्षित",  "marathi": "असुरक्षित",   "mandarin": "不安全"},
    "RegionKw":   {"english": "region",   "sanskrit": "क्षेत्र",       "hindi": "क्षेत्र",     "marathi": "क्षेत्र",       "mandarin": "区域"},

    # SOV-S7 (2026-06-06): four newly-Devanagari-aliased
    # keywords. All tatsama Sanskrit roots shared across the
    # three Indo-Aryan dialects.
    "Intent":     {"english": "intent",   "sanskrit": "उद्देश्य",     "hindi": "उद्देश्य",   "marathi": "उद्देश्य",     "mandarin": "目的"},
    "Type":       {"english": "type",     "sanskrit": "प्रकार",       "hindi": "प्रकार",      "marathi": "प्रकार",       "mandarin": "类型"},
    "Extern":     {"english": "extern",   "sanskrit": "बाह्य",        "hindi": "बाह्य",      "marathi": "बाह्य",        "mandarin": "外部"},
    "Invariant":  {"english": "invariant","sanskrit": "अपरिवर्तनीय",  "hindi": "अपरिवर्तनीय","marathi": "अपरिवर्तनीय",  "mandarin": "不变量"},
}

SUPPORTED_LANGS = ("english", "sanskrit", "hindi", "marathi", "mandarin")


def build_reverse_lookup() -> Dict[str, Tuple[str, str]]:
    """
    Build (spelling) → (TokenKind, language) lookup. When the same
    spelling appears in multiple languages (e.g. कार्य = fn in both
    Sanskrit and Marathi), the LAST entry wins; the translator
    treats them as interchangeable for token-kind identification.
    """
    rev: Dict[str, Tuple[str, str]] = {}
    for kind, langs in ALIASES.items():
        for lang, spelling in langs.items():
            rev[spelling] = (kind, lang)
    return rev


# Multi-word forms that the lexer fuses post-tokenization. The
# translator must detect them as a unit so a Hindi "के लिए" doesn't
# accidentally translate as two separate words. Source:
# src/lexer.rs:611-621.
MULTI_WORD_ALIASES: Dict[Tuple[str, ...], str] = {
    ("नहीं", "तो"):     "Else",
    ("के", "लिए"):       "For",
    ("सिद्ध", "करो"):    "Prove",
    ("सिद्ध", "करा"):    "Prove",
    ("समान्तर", "प्रति"): "Parallel",
}


# Word-char classifier: ASCII alphanumerics + underscore + the
# Devanagari Unicode block (U+0900–U+097F) including the supplement
# (U+A8E0–U+A8FF). Same predicate the lexer uses internally.
def _is_word_char(c: str) -> bool:
    if c.isalnum() or c == "_":
        return True
    cp = ord(c)
    if 0x0900 <= cp <= 0x097F:
        return True
    if 0x0A8E0 <= cp <= 0x0A8FF:
        return True
    return False


def translate(source: str, target_lang: str) -> str:
    """
    Walk `source` character-by-character, substituting any keyword
    token with the target_lang's spelling. Preserves everything
    else verbatim — comments, strings, whitespace, identifiers,
    operators, numbers. The `// vani-lang: <name>` pragma comment
    is rewritten to match `target_lang` so the output passes the
    per-file dialect-purity gate (fixed 2026-06-07: prior versions
    left the source pragma intact, which caused
    marathi→hindi/hindi→marathi translations to fail purity
    narrowing).
    """
    assert target_lang in SUPPORTED_LANGS, f"unknown target {target_lang}"
    rev = build_reverse_lookup()
    out: List[str] = []
    i = 0
    n = len(source)
    while i < n:
        c = source[i]
        # Line comment — copy through end of line, but rewrite a
        # `// vani-lang: <name>` pragma to match target_lang so
        # the dialect-purity gate in the output file accepts the
        # translated keywords.
        if c == "/" and i + 1 < n and source[i + 1] == "/":
            j = source.find("\n", i)
            if j == -1:
                j = n
            line = source[i:j]
            # Match e.g. `// vani-lang: marathi` (optional spaces).
            stripped = line.lstrip("/").strip()
            if stripped.startswith("vani-lang:") or stripped.startswith("vani-lang :"):
                # Preserve leading slashes/whitespace, replace the
                # body so the pragma now declares target_lang.
                leading = line[: len(line) - len(line.lstrip("/ \t"))]
                out.append(f"{leading}vani-lang: {target_lang}")
            else:
                out.append(line)
            i = j
            continue
        # String literal — copy through closing quote, handling
        # backslash escapes.
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
                out.append(source[i])  # closing quote
                i += 1
            continue
        # Word: collect until non-word char.
        if _is_word_char(c):
            j = i
            while j < n and _is_word_char(source[j]):
                j += 1
            word = source[i:j]
            # Multi-word lookahead: if word + next non-space + word2
            # matches a multi-word alias, translate as a unit.
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

            replaced_multi = False
            if second is not None:
                key = (word, second)
                if key in MULTI_WORD_ALIASES:
                    kind = MULTI_WORD_ALIASES[key]
                    if kind in ALIASES:
                        out.append(ALIASES[kind][target_lang])
                        i = second_end
                        replaced_multi = True
            if replaced_multi:
                continue

            # Single-word: look up.
            if word in rev:
                kind, _src_lang = rev[word]
                if kind in ALIASES:
                    out.append(ALIASES[kind][target_lang])
                    i = j
                    continue
            # English keyword fallback — the lexer's hardcoded
            # English-keyword set isn't in `rev` (we only put
            # Devanagari/English aliases from ALIASES there, which
            # does include the English forms via .english slot).
            # If the word matches an English form, treat as keyword.
            # (Already handled above since ALIASES["X"]["english"]
            # → "X" populated in `rev`.)
            out.append(word)
            i = j
            continue
        # Anything else (whitespace, punctuation, operators):
        # pass through.
        out.append(c)
        i += 1
    return "".join(out)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Translate a .vani source file's keywords between supported languages."
    )
    parser.add_argument("input", type=Path, help="source .vani file")
    parser.add_argument(
        "--from",
        dest="src_lang",
        choices=SUPPORTED_LANGS,
        default=None,
        help="source language (currently advisory only; the translator "
             "detects keywords regardless of source language)",
    )
    parser.add_argument(
        "--to",
        dest="target_lang",
        choices=SUPPORTED_LANGS,
        required=True,
        help="target language",
    )
    parser.add_argument(
        "-o",
        "--output",
        type=Path,
        default=None,
        help="output file (default: stdout)",
    )
    parser.add_argument(
        "--add-sri-header",
        action="store_true",
        help="prepend the Sanskrit `// श्री।` invocation header AND "
             "the SOV-S8 `// vani-lang: <lang>` purity pragma when "
             "translating to a Devanagari-script language (sanskrit / hindi / marathi)",
    )
    args = parser.parse_args()
    if not args.input.exists():
        print(f"input file not found: {args.input}", file=sys.stderr)
        return 1
    source = args.input.read_text(encoding="utf-8")
    translated = translate(source, args.target_lang)
    if args.add_sri_header and args.target_lang in ("sanskrit", "hindi", "marathi"):
        if not translated.lstrip().startswith("// श्री।"):
            translated = (
                f"// श्री।\n"
                f"// vani-lang: {args.target_lang}\n"
                f"//\n"
                + translated
            )
    if args.output:
        args.output.write_text(translated, encoding="utf-8")
    else:
        sys.stdout.write(translated)
    return 0


if __name__ == "__main__":
    sys.exit(main())
