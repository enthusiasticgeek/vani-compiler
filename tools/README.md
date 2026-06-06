# vāṇी tools

Out-of-tree utilities that don't ship with the compiler binary.

## `vani_translate.py` — cross-language `.vani` source translator

**Status**: B.1 v1 — shipped 2026-06-06 (commit pending).

Token-level keyword substitution between vāṇी's supported natural
languages: English, Sanskrit, Hindi, Marathi.

### Usage

```bash
# English → Sanskrit (with auspicious-beginning header)
python3 tools/vani_translate.py --to sanskrit \
    examples/language/english/basics.vani \
    -o /tmp/basics_sa.vani --add-sri-header

# Run the translation to verify it compiles + behaves identically
vanic run /tmp/basics_sa.vani --backend=c

# Translate any pair — `--from` is advisory; the translator
# recognizes keywords regardless of source language
python3 tools/vani_translate.py --to marathi /tmp/basics_sa.vani -o /tmp/basics_mr.vani
python3 tools/vani_translate.py --to hindi   /tmp/basics_mr.vani -o /tmp/basics_hi.vani
python3 tools/vani_translate.py --to english /tmp/basics_hi.vani -o /tmp/basics_back.vani
```

### What v1 does

- Substitutes 49 keyword token-kinds across all 4 languages, including
  the four newly-shipped SOV-S7 aliases (`intent`, `type`, `extern`,
  `invariant` → Sanskrit-root tatsama forms).
- Detects 5 multi-word Devanagari fusions (`नहीं तो` = else,
  `के लिए` = for, `सिद्ध करो` / `सिद्ध करा` = prove,
  `समान्तर प्रति` = parallel) as a unit, not two separate words.
- Preserves identifiers, comments, strings, whitespace, operators,
  numbers verbatim.
- Optional `--add-sri-header` prepends the Sanskrit `// श्री।`
  invocation comment when targeting a Devanagari language.

### What v1 does NOT do (deferred)

- **SOV word-order reshape.** The output keeps the source's word
  order; only keywords are substituted. A pure-Sanskrit source with
  verb-final shapes won't be reshaped to English keyword-first when
  translated (or vice versa). SOV reshape is Tier C work.
- **Identifier translation.** User-named functions, vars, types
  stay in whatever language the author wrote them in. Mixing is
  explicitly allowed by design.
- **Comment translation.** Comments are preserved verbatim — the
  user controls their language.

### Round-trip parity

Verified for 8 representative example shapes (basics, integers,
control_flow, early_exit, for_loops, vec_invariants, scopes,
verified) — English → Sanskrit → English produces source that
compiles to the same AST (modulo whitespace + chosen alias when
multiple existed). Run the embedded test:

```bash
python3 << 'EOF'
import re, subprocess
from pathlib import Path

EXAMPLES = [
    "basics.vani", "integers.vani", "control_flow.vani",
    "early_exit.vani", "for_loops.vani", "vec_invariants.vani",
    "scopes.vani", "verified.vani",
]

def normalize_ast(p):
    out = subprocess.run(['vanic', 'ast', str(p)],
                         capture_output=True, text=True).stdout
    return re.sub(r'span: Span \{[^}]*\}', '', out)

for name in EXAMPLES:
    src = Path('examples/language/english') / name
    sa = Path('/tmp') / f'rt_{name}.sa.vani'
    en2 = Path('/tmp') / f'rt_{name}.en2.vani'
    subprocess.run(['python3', 'tools/vani_translate.py',
                    '--to', 'sanskrit', str(src), '-o', str(sa),
                    '--add-sri-header'], check=True)
    subprocess.run(['python3', 'tools/vani_translate.py',
                    '--to', 'english', str(sa), '-o', str(en2)],
                   check=True)
    ok = normalize_ast(src) == normalize_ast(en2)
    print(f"  {'✅' if ok else '❌'} {name}")
EOF
```

### Next steps

- **B.1.1**: package as a `vanic translate` subcommand in Rust
  (Python is the prototype). Drop the JSON alias table; read
  directly from the lexer's keyword table at compile time.
- **B.1.2**: SOV reshape flag (`--reshape sov` / `--reshape
  keyword-first`). Pairs with Tier C (Sanskrit-derived SOV
  completion).
- **B.1.3**: extend to global-language surface (Tier E) — Spanish,
  Mandarin, Arabic, Japanese, etc. The alias table grows; the
  translator logic stays the same.
