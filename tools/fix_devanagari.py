#!/usr/bin/env python3
"""Fix residual garbled Devanagari and vāṇī brand name in tutorial files."""
import os

TUTORIAL_DIR = os.path.join(os.path.dirname(__file__), '..', 'tutorials', 'src')

FIXES = [
    # vāṇī brand name (ā = U+0101 was losing its continuation byte 0x81)
    ('vÄṇी', 'vāṇī'),
    ('VÄṇी', 'Vāṇī'),
    ('vÄṇī', 'vāṇī'),
    ('Äṇी', 'āṇी'),
    ('Äṇī', 'āṇī'),
    # Sanskrit keywords in 12_devanagari.md
    ('मà¥खà¥य', 'मुख्य'),       # mukhya = main
    ('शà¥री।', 'श्री।'),         # shree header
    ('उदà¥देशà¥य', 'उद्देश्य'),  # uddeshy = intent
    ('कारà¥य', 'कार्य'),         # karya = fn
    ('अपेकà¥षित', 'अपेक्षित'),  # apekshit = requires
    ('पà¥नरागम', 'पुनरागम'),     # punarAgam = return
    ('सिदà¥धमà¥', 'सिद्धम्'),    # siddham = assert
    ('पà¥रति', 'प्रति'),         # prati = for
    ('यावतà¥', 'यावत्'),         # yAvat = while
    # Stray continuation markers (fallback -- only if above didn't catch them)
    ('à¥', ''),
    ('à¤', ''),
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
        for garbled, correct in FIXES:
            text = text.replace(garbled, correct)
        if text != original:
            with open(fpath, 'w', encoding='utf-8', newline='\n') as f:
                f.write(text)
            rel = os.path.relpath(fpath, TUTORIAL_DIR).replace('\\', '/')
            changed.append(rel)
            print(f'  fixed: {rel}')

print(f'\n{len(changed)} files changed.')
