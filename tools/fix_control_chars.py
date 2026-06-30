#!/usr/bin/env python3
"""
Fix residual C1 control characters (U+0081, U+008D) and stray Ä (U+00C4)
left over from the cp1252 round-trip of garbled UTF-8.

Also replace remaining box-drawing characters with ASCII.
"""
import os, re

TUTORIAL_DIR = os.path.join(os.path.dirname(__file__), '..', 'tutorials', 'src')

# U+0081 / U+008D are C1 control characters from garbled bytes 0x81 / 0x8D.
# Remove them, then fix any Ä that was left behind as the first byte of
# a garbled ā (U+0101 = C4 81 in UTF-8).
#
# Pattern: Ä followed by one or more control chars, followed by anything
# => the Ä should be ā when it is directly adjacent to ṇ or followed by
#    Devanagari continuation.

BOX_FIXES = [
    ('┐', '+'),
    ('┘', '+'),
    ('┌', '+'),
    ('┤', '+'),
    ('┬', '+'),
    ('┼', '+'),
]

MISC_FIXES = [
    # Remove C1 control characters
    ('', ''),
    ('', ''),
    ('', ''),
    ('', ''),
    ('', ''),
    # Fix stray Ä that should be ā (Latin small a with macron)
    # These appear in vāṇī = v + Ä[lost 0x81] + ṇ + ī
    # After removing U+0081, we have vÄṇī -> replace Äṇ with āṇ
    ('Äṇ', 'āṇ'),
    # Any remaining Ä at start of a word where a macron-a is expected
    # (conservative: only if next char is also non-ASCII, i.e., part of vāṇī)
    # Subscript / superscript that slipped through
    ('ᵢ', 'i'),   # LATIN SUBSCRIPT SMALL LETTER I ᵢ
    ('²', '^2'),  # SUPERSCRIPT TWO ²
    ('³', '^3'),  # SUPERSCRIPT THREE ³
    ('ⁿ', '^n'),  # SUPERSCRIPT LATIN SMALL LETTER N ⁿ
    ('₂', '_2'),  # SUBSCRIPT TWO ₂
    ('≡', '==='), # IDENTICAL TO ≡
    ('✘', '[ ]'), # HEAVY BALLOT X ✘
    ('❌', '[x]'), # CROSS MARK emoji ❌
    # Box drawing (any remaining after first pass)
    ('┘', '+'), ('┐', '+'), ('┌', '+'), ('┤', '+'),
    ('┬', '+'), ('┼', '+'), ('─', '-'), ('│', '|'),
    # Stray small chars
    ('ṇī', 'ṇī'),  # protect correct ṇī sequence
]

changed = []
for dirpath, _, filenames in os.walk(TUTORIAL_DIR):
    for fname in sorted(filenames):
        if not fname.endswith('.md'):
            continue
        fpath = os.path.join(dirpath, fname)
        with open(fpath, 'r', encoding='utf-8') as f:
            text = f.read()
        original = text
        for src, dst in BOX_FIXES + MISC_FIXES:
            text = text.replace(src, dst)
        if text != original:
            with open(fpath, 'w', encoding='utf-8', newline='\n') as f:
                f.write(text)
            rel = os.path.relpath(fpath, TUTORIAL_DIR).replace('\\', '/')
            changed.append(rel)
            print(f'  fixed: {rel}')

print(f'\n{len(changed)} files changed.')
