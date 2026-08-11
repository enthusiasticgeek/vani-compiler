# Beginner 12 -- Devanagari surface (optional intro)

> **Learning goal**: see vāṇी's same program in Sanskrit, get a
> feel for verb-at-end (SOV) shape, and decide whether the
> Devanagari dialect surface is for you. Skip this lesson if
> you only plan to write English-keyword vāṇी; nothing later in
> the tracks depends on it.

Most programming languages use English keywords because they
were invented by English speakers. vāṇी treats that as a
historical accident, not a rule. The same compiler that reads
`fn main()` also reads `फलन main()`, because the Devanagari
keyword `फलन` maps to `fn` in the same way `funcion` maps to
"function" in Spanish. You add one line to your file --
`// vani-lang: sanskrit` -- and the entire keyword vocabulary
switches. The program still runs identically; only the words
you type change.

The compiler natively supports Sanskrit, Hindi, Marathi, and several
other script families. The translator tool (`tools/vani_translate.py`)
extends this to **57 languages** — from Russian and Arabic to Japanese
and Swahili — so you can convert existing files into any of those
languages even if the compiler does not yet parse them natively.
This lesson shows you what the Devanagari surface looks like in
practice.

## The program

Save this in `~/lesson12.vani`:

```vani
// श्री।
// vani-lang: sanskrit
उद्देश्य "Lesson 12 -- first program with a Sanskrit pragma.";

कार्य add(a: i64, b: i64) -> i64
अपेक्षित a >= 0;
अपेक्षित b >= 0;
{
  पुनरागम a + b;
}

कार्य main() -> i64 {
  माना x: i64 = 5;
  माना y: i64 = 7;
  माना sum: i64 = add(x, y);
  सिद्धम् sum == 12;
  लिख sum;
  पुनरागम 0;
}
```

## Compile + run

```bash
vanic run ~/lesson12.vani
```

Expected output:

```
१२
```

Note the **Devanagari digits**: `१२` is `12`. vāṇी's runtime
PRINT helper detects the file's `// vani-lang:` pragma and
converts integer output to Devanagari numerals (`०..९`)
automatically (Phase 1.1).

## What changed vs. the English version

Side-by-side, mapping by row:

| English | Sanskrit (`vani-lang: sanskrit`) |
|---|---|
| `intent "...";` | `उद्देश्य "...";` |
| `fn add(...)` | `कार्य add(...)` |
| `requires a >= 0;` | `अपेक्षित a >= 0;` |
| `return a + b;` | `पुनरागम a + b;` |
| `let x: i64 = 5;` | `माना x: i64 = 5;` |
| `assert sum == 12;` | `सिद्धम् sum == 12;` |
| `print sum;` | `लिख sum;` |

Identifier names (`add`, `x`, `y`, `sum`, `main`) and the
`i64` type stay ASCII -- Devanagari identifiers are supported
but they're a stylistic choice, not a requirement. The one
exception is the entry point itself: `मुख्य`, `प्रमुख`, and
`प्रधान` are all canonicalized to the same entry point as plain
`main`, regardless of the file's dialect -- so a fully-Devanagari
file can spell it `मुख्य()` instead of `main()` if you'd rather
not have one lone Latin word in an otherwise-Devanagari program.

## Why it works that way

- **`// vani-lang: <dialect>`** is the *purity pragma*. Inside
  this file, every structure keyword must be a valid
  spelling for the declared dialect (Sanskrit / Hindi /
  Marathi / Nepali / Maithili / Konkani -- the last three were
  added in Phase 2). Mixing English keywords mid-file is a
  compile error: "language mismatch."
- **The `// श्री।` header** is decorative -- a conventional
  Sanskrit *auspicious-beginning* mark. The compiler ignores it.
  It's a recognizable cue that the file uses the dialect surface.
- **`प्रति` / `यदि` / `यावत्`** (and their Hindi/Marathi
  cousins) take care of `for` / `if` / `while`. The keyword alias
  tables (Sanskrit / Hindi / Marathi, by category) are in the
  [Language Manual](https://github.com/enthusiasticgeek/vani-compiler/blob/main/docs/language_manual.md#multilingual-keywords)'s
  "Multilingual keywords" section.
- **SOV (verb-at-end) shapes** like `x लिख;` (= `print x;`) are
  available for `print`, `return`, `assert`, `prove`, `let`,
  `if`, and `while`. They're documented in
  Advanced Sec.7 -- *Devanagari purity arc*.
- **The translator works both ways and across 57 languages**.
  Convert any `.vani` file between human languages without
  rewriting by hand:
  ```bash
  # English → Sanskrit
  python3 tools/vani_translate.py --to sanskrit \
      ~/lesson1.vani -o ~/lesson1_sa.vani --add-sri-header

  # Sanskrit → Japanese (or any other of the 57 supported languages)
  python3 tools/vani_translate.py --to japanese \
      ~/lesson1_sa.vani -o ~/lesson1_ja.vani

  # Add --llm anthropic to also translate comments and strings
  python3 tools/vani_translate.py --to hindi ~/lesson1.vani \
      --llm anthropic --llm-model claude-haiku-4-5-20251001
  ```
  See [`tools/README.md`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/tools/README.md)
  for the full reference and the LLM backend options.

### Mixing keywords from two scripts is a compile error

The "language mismatch" rule isn't just a warning -- once a
file has committed to a dialect (via a Devanagari structure
keyword), an English keyword later in the same file is
rejected:

<img class="manas" src="../images/mascot/manas_mascot_error.png" title="this code does not compile!"/>

```vani
// vani-lang: sanskrit
उद्देश्य "Mixing an English keyword into a Sanskrit-pragma file.";

fn add(a: i64, b: i64) -> i64 {
  पुनरागम a + b;
}
```

The compiler stops at `fn` with "language mismatch: file
already used a Devanagari structure keyword ... can't switch
to a English alias mid-file. Pick one script per file." Every
structure keyword in a `// vani-lang: sanskrit` file must be
`कार्य`, not `fn`.

## Should I write vāṇी in Devanagari?

It's a stylistic choice. The Devanagari surface exists for two
reasons:

1. **Code as you speak.** If your mental language for thinking
   about programs is Sanskrit / Hindi / Marathi, writing keywords
   in those languages removes a context-switch.
2. **Pedagogy in Indian-language CS curricula.** A vāṇी
   classroom can introduce programming without forcing students
   to learn English keywords first.

If neither applies to you, **stay in English** -- every example
elsewhere in this tutorial uses the English surface, and the
language is fully expressive there.

## Challenge

1. Pick any solution from Lessons 2–11 and translate it to a
   Sanskrit-pragma file *by hand* (no tooling). Run it and confirm
   the output is identical (except for the Devanagari numerals).
   This internalises the keyword table better than reading it.

2. *(Bonus)* Run the same file through the translator to Japanese or
   Arabic and compare the result with your hand-translated Sanskrit
   version. Notice what the keyword substitution does and doesn't
   change.

---

**Congratulations -- you've completed the Beginner track!**

Next steps:
- **Intermediate** -- [Sec.1 -- Structs and methods ->](../intermediate/01_struct_methods.md)
  starts the next track. You'll add custom types, generics,
  dynamic dispatch, and a deep dive on the SMT verifier.
- **Browse the examples**. With the Beginner track behind you,
  `examples/language/english/` should be navigable. Start with
  the ones in the alphabetical first quarter -- they're the
  simplest.
- **Try a [design pattern](../intermediate/11_design_patterns.md)**.
  The 22 GoF patterns in
  [`examples/language/english/design_patterns/`](https://github.com/enthusiasticgeek/vani-compiler/tree/main/examples/language/english/design_patterns)
  each show a v1 idiom (tagged-struct Composite, int-disc
  Bridge, ...) that's worth knowing even if you're not writing
  enterprise code.


---

**Previous**: [Sec.11 -- Challenges ->](11_challenges.md)
**Next**: [Sec.13a -- Big-O notation primer ->](13a_big_o_primer.md)

