#!/usr/bin/env python3
"""
bundle — assemble a vāṇी context bundle for prompt-engineering
        an off-the-shelf LLM (Claude, GPT-4-class, Llama-3-class)
        as a vāṇी programmer.

Pipes the bundle to stdout so you can pass it as a system / dev
prompt without spending tokens on iterating through individual
files. Reads only repo sources of truth so the bundle stays
in sync without manual maintenance.

Sections (in order):

  1. System prompt — orienting the model as a vāṇी programmer.
  2. Keyword alias table — the English ↔ Sanskrit ↔ Hindi ↔
     Marathi mapping for every TokenKind. Source of truth:
     `tools/vani_translate.py::ALIASES`.
  3. SOV statement-shape table — how Sanskrit verb-at-end
     statements desugar to keyword-first.
  4. Design-pattern catalog — the 22 GoF examples from
     `examples/language/english/design_patterns/`, one line of
     intent per pattern + a pointer to the source file.
  5. English-keyword example corpus — for each example file,
     the `intent "…"` line + every fn signature, so the model
     knows what shape of code already exists.
  6. Dialect-aware error prefixes — the Devanagari error labels
     the compiler emits, so the model can match them when
     debugging Devanagari programs.
  7. v1 limitations catalog — the full `docs/v1_limitations.md`
     verbatim so the model doesn't suggest unsupported textbook
     constructs.

Usage:

    python3 tools/llm_context/bundle.py | pbcopy           # macOS
    python3 tools/llm_context/bundle.py | xclip -sel clip  # X11
    python3 tools/llm_context/bundle.py > /tmp/vani_ctx.md
    # then paste into Claude / GPT / Llama as a system message

Flags:
    --section <name>     emit only one section (system | aliases |
                         sov | patterns | examples | errors | limits)
    --no-examples        skip section 5 (cuts ~30K tokens)
    --no-limits          skip section 7
"""

import argparse
import importlib.util
import re
import sys
from pathlib import Path
from typing import Dict, List


REPO_ROOT = Path(__file__).resolve().parent.parent.parent
TRANSLATE_PY = REPO_ROOT / "tools" / "vani_translate.py"
EXAMPLES_EN = REPO_ROOT / "examples" / "language" / "english"
PATTERNS_DIR = EXAMPLES_EN / "design_patterns"
PATTERNS_README = PATTERNS_DIR / "README.md"
LIMITATIONS = REPO_ROOT / "docs" / "v1_limitations.md"
DIAGNOSTIC_RS = REPO_ROOT / "src" / "diagnostic.rs"


def load_aliases() -> Dict[str, Dict[str, str]]:
    """Import ALIASES from vani_translate.py without running its CLI."""
    spec = importlib.util.spec_from_file_location("vani_translate", TRANSLATE_PY)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod.ALIASES


def emit_system_prompt(out: List[str]) -> None:
    out.append("# vāṇी context bundle")
    out.append("")
    out.append(
        "You are a vāṇी (वाणी) programmer. vāṇी is a Rust-flavored language "
        "with a C and an LLVM backend, an SMT-backed verifier, and per-file "
        "dialect support for English / Sanskrit / Hindi / Marathi keywords."
    )
    out.append("")
    out.append("Ground rules when emitting vāṇी source:")
    out.append("")
    out.append("- Use **English keywords by default** (`fn`, `let`, `if`, `while`, …).")
    out.append("- If the file declares `// vani-lang: sanskrit` (or `hindi` / `marathi`), "
               "use the matching Devanagari keywords from §2 EXCLUSIVELY for that file.")
    out.append("- Honor the v1 limitations in §7 — do not suggest textbook constructs "
               "marked there as unsupported (enum-destructure, `let mut`, `Box<T>`, …). "
               "Adapt the design pattern to the documented workaround.")
    out.append("- Prefer SMT-verifiable shapes when possible: integer/bool arithmetic "
               "with `requires` / `ensures` clauses verifies at compile time.")
    out.append("- For UI / output, integer `print x` will emit Devanagari numerals "
               "automatically inside a Devanagari-pragma file (Phase 1.1).")
    out.append("")


def emit_aliases(out: List[str], aliases: Dict[str, Dict[str, str]]) -> None:
    out.append("## 2. Keyword alias table")
    out.append("")
    out.append("Every TokenKind and its canonical spelling per dialect. "
               "Multiple aliases exist for some kinds in the lexer; this table "
               "picks the most natural one. **Source of truth**: "
               "`tools/vani_translate.py::ALIASES`.")
    out.append("")
    out.append("| TokenKind | english | sanskrit | hindi | marathi |")
    out.append("|---|---|---|---|---|")
    for kind, row in aliases.items():
        out.append(
            f"| `{kind}` | `{row['english']}` | `{row['sanskrit']}` | "
            f"`{row['hindi']}` | `{row['marathi']}` |"
        )
    out.append("")


def emit_sov_table(out: List[str]) -> None:
    out.append("## 3. SOV statement-shape table (Sanskrit verb-at-end)")
    out.append("")
    out.append("In a `// vani-lang: sanskrit` file, these verb-at-end shapes "
               "desugar 1:1 to the canonical keyword-first form. Both shapes "
               "compile.")
    out.append("")
    out.append("| SOV shape | Desugars to |")
    out.append("|---|---|")
    out.append("| `<expr> लिख;`                       | `print <expr>;`              |")
    out.append("| `<expr> पुनरागम;`                    | `return <expr>;`             |")
    out.append("| `<expr> सिद्धम्;`                     | `assert <expr>;`             |")
    out.append("| `<expr> प्रमाण;`                     | `prove <expr>;`              |")
    out.append("| `<name>: <type> = <init> माना;`     | `let <name>: <type> = <init>;` |")
    out.append("| `<cond> यदि { … } अन्यथा { … }`     | `if <cond> { … } else { … }` |")
    out.append("| `<cond> यावत् { … }`                 | `while <cond> { … }`         |")
    out.append("")
    out.append("Other constructs (fn / struct / enum / top-level decls) are "
               "keyword-first only in v1 — no SOV path yet.")
    out.append("")


def emit_patterns(out: List[str]) -> None:
    out.append("## 4. GoF design-pattern catalog (English)")
    out.append("")
    out.append("The 22 GoF patterns each have a self-contained example. Each "
               "file cites the refactoring.guru URL, the textbook intent, and "
               "the vāṇी-specific deviation (cross-references the v1 "
               "limitations in §7). Refer to these when asked to implement a "
               "pattern — they show the v1 idioms (tagged-struct Composite, "
               "int-discriminator Bridge, free-fn Observer, etc.).")
    out.append("")
    for category in sorted(PATTERNS_DIR.iterdir()):
        if not category.is_dir():
            continue
        out.append(f"### {category.name.capitalize()}")
        out.append("")
        for vfile in sorted(category.glob("*.vani")):
            rel = vfile.relative_to(REPO_ROOT)
            intent = ""
            for line in vfile.read_text().splitlines():
                m = re.match(r'^\s*intent\s+"([^"]+)"', line)
                if m:
                    intent = m.group(1)
                    break
            out.append(f"- **{vfile.stem}** — {intent}. `{rel}`")
        out.append("")


def emit_examples(out: List[str]) -> None:
    out.append("## 5. English-keyword example corpus")
    out.append("")
    out.append("For each example, the `intent` declaration plus the function "
               "signatures. Body and tests omitted to keep the bundle "
               "context-window-efficient. Cite the path when generating "
               "code that follows the same shape.")
    out.append("")
    files = sorted(p for p in EXAMPLES_EN.glob("*.vani"))
    for vfile in files:
        rel = vfile.relative_to(REPO_ROOT)
        intent = ""
        sigs: List[str] = []
        for line in vfile.read_text().splitlines():
            if not intent:
                m = re.match(r'^\s*intent\s+"([^"]+)"', line)
                if m:
                    intent = m.group(1)
                    continue
            stripped = line.strip()
            if stripped.startswith("fn ") and "{" in stripped:
                sigs.append(stripped.split("{", 1)[0].rstrip())
            elif stripped.startswith("fn ") and stripped.endswith(";"):
                sigs.append(stripped)
        out.append(f"### `{rel}`")
        if intent:
            out.append(f"*Intent*: {intent}")
        if sigs:
            out.append("")
            out.append("```vani")
            for s in sigs:
                out.append(s)
            out.append("```")
        out.append("")


def emit_error_prefixes(out: List[str]) -> None:
    out.append("## 6. Dialect-aware error prefixes")
    out.append("")
    out.append("When a file declares `// vani-lang: sanskrit | hindi | marathi`, "
               "the compiler renders error labels and a curated prefix table "
               "in Devanagari. If the user shows you a Devanagari error trace, "
               "these are the shapes to recognize.")
    out.append("")
    src = DIAGNOSTIC_RS.read_text()
    out.append("```rust")
    in_table = False
    line_count = 0
    for line in src.splitlines():
        if "localize_message" in line and "fn " in line:
            in_table = True
        if in_table:
            out.append(line)
            line_count += 1
            if line_count > 120:
                out.append("// (truncated — see src/diagnostic.rs for the full table)")
                break
            if line.strip() == "}" and line_count > 10:
                break
    out.append("```")
    out.append("")


def emit_limitations(out: List[str]) -> None:
    out.append("## 7. v1 limitations catalog")
    out.append("")
    out.append(
        "Every textbook construct that's not supported in vāṇी v1, with "
        "the documented workaround. **Honor these** — suggesting a "
        "workaround-required pattern without applying the workaround is "
        "the most common failure mode."
    )
    out.append("")
    out.append(LIMITATIONS.read_text())
    out.append("")


SECTIONS = {
    "system":   emit_system_prompt,
    "aliases":  lambda out: emit_aliases(out, load_aliases()),
    "sov":      emit_sov_table,
    "patterns": emit_patterns,
    "examples": emit_examples,
    "errors":   emit_error_prefixes,
    "limits":   emit_limitations,
}


def main() -> int:
    ap = argparse.ArgumentParser(
        description="Assemble a vāṇी context bundle for LLM prompts.",
    )
    ap.add_argument("--section", choices=list(SECTIONS), default=None,
                    help="emit only this section (default: all)")
    ap.add_argument("--no-examples", action="store_true",
                    help="skip §5 (cuts ~30K tokens)")
    ap.add_argument("--no-limits", action="store_true",
                    help="skip §7")
    args = ap.parse_args()

    out: List[str] = []
    if args.section is not None:
        SECTIONS[args.section](out)
    else:
        emit_system_prompt(out)
        emit_aliases(out, load_aliases())
        emit_sov_table(out)
        emit_patterns(out)
        if not args.no_examples:
            emit_examples(out)
        emit_error_prefixes(out)
        if not args.no_limits:
            emit_limitations(out)

    sys.stdout.write("\n".join(out))
    return 0


if __name__ == "__main__":
    sys.exit(main())
