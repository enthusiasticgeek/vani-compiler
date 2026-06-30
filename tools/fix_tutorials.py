#!/usr/bin/env python3
"""
fix_tutorials.py — Repair tutorial Markdown files.

Two-pass approach:
  Pass 1: For the 13 files with double-encoded characters, recover the
          original Unicode by re-encoding as cp1252 and decoding as UTF-8.
  Pass 2: Replace all Unicode typography with ASCII equivalents across
          every file so mdBook renders cleanly on all browsers/charsets.
  Also: strip UTF-8 BOMs.
"""

import os
import sys

TUTORIAL_DIR = os.path.join(os.path.dirname(__file__), '..', 'tutorials', 'src')

# Files with double-encoded (garbled) content — UTF-8 bytes were
# misread as cp1252 and re-saved, doubling the encoding.
GARBLED = {
    'advanced/04_embedded.md',
    'advanced/05_vtables.md',
    'advanced/10_internals.md',
    'beginner/05_loops.md',
    'beginner/09_smt_intro.md',
    'beginner/12_devanagari.md',
    'glossary.md',
    'intermediate/02_enums_payloads.md',
    'intermediate/03_affine.md',
    'intermediate/05_dyn.md',
    'intermediate/10_result_try.md',
    'intermediate/11_design_patterns.md',
    'intermediate/12_smt_deepdive.md',
}

def fix_garbled(text):
    """Recover text that was UTF-8 decoded as cp1252 then re-encoded."""
    try:
        return text.encode('cp1252').decode('utf-8')
    except (UnicodeEncodeError, UnicodeDecodeError):
        # Fall back chunk-by-chunk when isolated chars can't round-trip
        result = []
        i = 0
        while i < len(text):
            # Try progressively larger windows (handles multi-byte sequences)
            fixed = False
            for n in (4, 3, 2, 1):
                chunk = text[i:i + n]
                try:
                    result.append(chunk.encode('cp1252').decode('utf-8'))
                    i += n
                    fixed = True
                    break
                except (UnicodeEncodeError, UnicodeDecodeError):
                    continue
            if not fixed:
                result.append(text[i])
                i += 1
        return ''.join(result)

# Ordered replacements: longer strings first to avoid partial matches.
TYPOGRAPHY = [
    # Box drawing (must come before single-char replacements)
    ('├──', '+--'),
    ('└──', '+--'),
    ('├─►', '+->'),
    ('└─►', '+->'),
    ('├─▶', '+->'),
    ('└─▶', '+->'),
    ('├── ', '+-- '),
    ('└── ', '+-- '),
    ('──', '--'),
    ('═══', '==='),
    ('══', '=='),
    ('║', '|'),
    ('╔', '+'),
    ('╗', '+'),
    ('╚', '+'),
    ('╝', '+'),
    # Arrows
    ('→', '->'),   # →
    ('←', '<-'),   # ←
    ('↔', '<->'),  # ↔
    ('↓', 'v'),    # ↓
    ('↑', '^'),    # ↑
    ('▼', 'v'),    # ▼
    ('▶', '>'),    # ▶
    # Dashes
    ('—', '--'),   # em dash —
    ('–', '-'),    # en dash –
    # Ellipsis
    ('…', '...'),  # …
    # Section sign
    ('§', 'Sec.'), # §
    # Middle dot / bullet
    ('·', '*'),    # ·
    ('•', '*'),    # bullet •
    # Superscripts
    ('²', '^2'),   # ²
    ('³', '^3'),   # ³
    ('½', '1/2'),  # ½
    # Box drawing singles
    ('│', '|'),    # │
    ('├', '+'),    # ├
    ('└', '+'),    # └
    ('─', '-'),    # ─
    # Math
    ('≈', '~='),   # ≈
    ('≤', '<='),   # ≤
    ('≥', '>='),   # ≥
    ('≠', '!='),   # ≠
    ('×', 'x'),    # × (multiplication)
    ('÷', '/'),    # ÷
    ('µ', 'u'),    # µ (micro)
    # Greek letters (appear in complexity/math notation)
    ('π', 'pi'),
    ('Γ', 'Gamma'),
    ('Σ', 'Sigma'),
    ('α', 'alpha'),
    ('β', 'beta'),
    ('σ', 'sigma'),
    ('φ', 'phi'),
    ('μ', 'mu'),
    ('τ', 'tau'),
    ('θ', 'theta'),
    ('λ', 'lambda'),
    ('ε', 'epsilon'),
    ('δ', 'delta'),
    # Smart / curly quotes -> straight
    ('“', '"'),    # "
    ('”', '"'),    # "
    ('‘', "'"),    # '
    ('’', "'"),    # '
    # Check / cross marks
    ('✓', '[x]'),  # ✓
    ('✗', '[ ]'),  # ✗
    ('✘', '[ ]'),  # ✘
    # Currency (only if they appear outside Devanagari contexts)
    ('£', 'GBP'),  # £
    # Misc
    ('ó', 'o'),    # ó  (in Ó(n) complexity notation — keep lowercase)
    ('Ó', 'O'),    # Ó  (in Ó(n) big-O notation)
]

BOM = '﻿'


def process_file(rel_path, abs_path):
    with open(abs_path, 'r', encoding='utf-8', errors='replace') as f:
        text = f.read()

    original = text

    # Strip BOM
    if text.startswith(BOM):
        text = text[len(BOM):]

    # Pass 1: fix double-encoded content
    if rel_path in GARBLED:
        text = fix_garbled(text)

    # Pass 2: replace Unicode typography with ASCII
    for src, dst in TYPOGRAPHY:
        text = text.replace(src, dst)

    if text != original:
        with open(abs_path, 'w', encoding='utf-8', newline='\n') as f:
            f.write(text)
        return True
    return False


def main():
    changed = []
    errors = []
    for dirpath, _, filenames in os.walk(TUTORIAL_DIR):
        for fname in sorted(filenames):
            if not fname.endswith('.md'):
                continue
            abs_path = os.path.join(dirpath, fname)
            rel_path = os.path.relpath(abs_path, TUTORIAL_DIR).replace('\\', '/')
            try:
                if process_file(rel_path, abs_path):
                    changed.append(rel_path)
                    print(f'  fixed: {rel_path}')
            except Exception as e:
                errors.append((rel_path, str(e)))
                print(f'  ERROR: {rel_path}: {e}', file=sys.stderr)

    print(f'\n{len(changed)} files changed, {len(errors)} errors.')
    if errors:
        sys.exit(1)


if __name__ == '__main__':
    main()
