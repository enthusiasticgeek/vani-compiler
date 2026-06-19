#!/usr/bin/env python3
"""
vani_translate — translate a .vani source file's keywords between
                English, Sanskrit, Hindi, and Marathi.

B.1 v2 — token-level keyword substitution with round-trip
verification, auto-detection, and batch mode.

Usage:
    # Translate to Sanskrit (auto-detects source from pragma):
    python3 tools/vani_translate.py examples/language/english/basics.vani \
        --to sanskrit -o out.vani

    # Verify round-trip: english → hindi → english matches original:
    python3 tools/vani_translate.py basics.vani --to hindi --verify

    # Translate all .vani files in a directory tree:
    python3 tools/vani_translate.py examples/language/english/ \
        --to marathi --batch -o translated/

    # Edit in-place (saves backup as .vani.bak):
    python3 tools/vani_translate.py basics.vani --to sanskrit --inplace

    # Print all keyword aliases as a markdown table:
    python3 tools/vani_translate.py --list-keywords

    # Verify AST equivalence via vanic (requires vanic in PATH):
    diff <(vanic ast examples/language/english/basics.vani) \
         <(vanic ast out.vani)

What this v2 does NOT do (deferred to later phases):
  - SOV word-order reshape. The output keeps the source's word
    order; only keywords are substituted. A Sanskrit source with
    verb-final shapes won't be reshaped to keyword-first when
    translated to English.
  - Identifier translation. User-named functions, vars, and types
    stay in whatever language the author wrote them in.
  - Comment translation. Comments are preserved verbatim.
"""

import argparse
import io
import sys
from pathlib import Path
from typing import Dict, List, Optional, Tuple

# Ensure UTF-8 output on Windows (default console is cp1252).
if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8")
if hasattr(sys.stderr, "reconfigure"):
    sys.stderr.reconfigure(encoding="utf-8")


# Single-word keyword alias table. Each TokenKind maps to one
# canonical spelling per language. Source: src/lexer.rs
#
# When multiple aliases exist for a (TokenKind, language) pair, the
# table picks the most natural / most-common spelling.
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

# Multi-word forms that the lexer fuses post-tokenization.
MULTI_WORD_ALIASES: Dict[Tuple[str, ...], str] = {
    ("नहीं", "तो"):      "Else",
    ("के", "लिए"):        "For",
    ("सिद्ध", "करो"):     "Prove",
    ("सिद्ध", "करा"):     "Prove",
    ("समान्तर", "प्रति"): "Parallel",
}


def _is_word_char(c: str) -> bool:
    if c.isalnum() or c == "_":
        return True
    cp = ord(c)
    return (0x0900 <= cp <= 0x097F) or (0x0A8E0 <= cp <= 0x0A8FF)


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
            # Check multi-word.
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


def translate(source: str, target_lang: str) -> str:
    """
    Walk `source` substituting every keyword with the target_lang
    spelling. Rewrites `// vani-lang:` pragma to match target_lang.
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


def verify_roundtrip(source: str, target_lang: str, src_lang: Optional[str]) -> Tuple[bool, str]:
    """
    Translate source → target_lang → src_lang, then compare the
    keyword-token sequences of the original and the double-translated
    result. Returns (passed, message).
    """
    effective_src = src_lang or detect_pragma_lang(source) or "english"
    intermediate = translate(source, target_lang)
    back = translate(intermediate, effective_src)
    orig_tokens = extract_keyword_tokens(source)
    back_tokens = extract_keyword_tokens(back)
    if orig_tokens == back_tokens:
        return True, (
            f"round-trip ok: {effective_src} → {target_lang} → {effective_src} "
            f"({len(orig_tokens)} keyword tokens preserved)"
        )
    diffs = [
        f"  pos {i}: {a!r} → {b!r}"
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
        cells = " | ".join(mapping.get(l, "—") for l in langs)
        rows.append(f"| {kind:<12} | {cells} |")
    return "\n".join(rows)


def _translate_file(
    src_path: Path,
    target_lang: str,
    out_path: Optional[Path],
    inplace: bool,
    add_sri_header: bool,
    verify: bool,
    src_lang: Optional[str],
    verbose: bool,
) -> bool:
    source = src_path.read_text(encoding="utf-8")
    if verify:
        ok, msg = verify_roundtrip(source, target_lang, src_lang)
        prefix = src_path.name + ": " if verbose else ""
        print(f"{prefix}{msg}", file=sys.stderr if not ok else sys.stdout)
        if not ok:
            return False

    translated = translate(source, target_lang)

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
            print(f"  {src_path}  (backup → {backup.name})")
    elif out_path is not None:
        if out_path.is_dir():
            dest = out_path / src_path.name
        else:
            dest = out_path
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_text(translated, encoding="utf-8")
        if verbose:
            print(f"  {src_path} → {dest}")
    else:
        sys.stdout.write(translated)
    return True


def main() -> int:
    parser = argparse.ArgumentParser(
        description=(
            "Translate a .vani source file's keywords between "
            "English, Sanskrit, Hindi, Marathi, and Mandarin."
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
            "source language — optional; auto-detected from the "
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

    if args.batch:
        if not args.input.is_dir():
            parser.error("--batch requires INPUT to be a directory")
        files = list(args.input.rglob("*.vani"))
        if not files:
            print(f"no .vani files found under {args.input}", file=sys.stderr)
            return 1
        ok_count = 0
        for f in sorted(files):
            ok = _translate_file(
                f, args.target_lang, args.output, args.inplace,
                args.add_sri_header, args.verify, args.src_lang, verbose=True,
            )
            if ok:
                ok_count += 1
        print(f"{ok_count}/{len(files)} files translated successfully.", file=sys.stderr)
        return 0 if ok_count == len(files) else 1
    else:
        if not args.input.exists():
            print(f"input file not found: {args.input}", file=sys.stderr)
            return 1
        ok = _translate_file(
            args.input, args.target_lang, args.output, args.inplace,
            args.add_sri_header, args.verify, args.src_lang, verbose=args.verbose,
        )
        return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
