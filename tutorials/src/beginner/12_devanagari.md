# Beginner 12 — Devanagari surface (optional intro)

> **Learning goal**: see vāṇी's same program in Sanskrit, get a
> feel for verb-at-end (SOV) shape, and decide whether the
> Devanagari dialect surface is for you. Skip this lesson if
> you only plan to write English-keyword vāṇी; nothing later in
> the tracks depends on it.

## The program

Save this in `~/lesson12.vani`:

```vani
// श्री।
// vani-lang: sanskrit
उद्देश्य "Lesson 12 — first program with a Sanskrit pragma.";

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
`i64` type stay ASCII — Devanagari identifiers are supported
but they're a stylistic choice, not a requirement.

## Why it works that way

- **`// vani-lang: <dialect>`** is the *purity pragma*. Inside
  this file, every structure keyword must be a valid
  spelling for the declared dialect (Sanskrit / Hindi /
  Marathi / Nepali / Maithili / Konkani — the last three were
  added in Phase 2). Mixing English keywords mid-file is a
  compile error: "language mismatch."
- **The `// श्री।` header** is decorative — a conventional
  Sanskrit *auspicious-beginning* mark. The compiler ignores it.
  It's a recognizable cue that the file uses the dialect surface.
- **`प्रति` / `यदि` / `यावत्`** (and their Hindi/Marathi
  cousins) take care of `for` / `if` / `while`. The complete
  alias table for all 46 structure keywords is in the
  [README](https://github.com/anthropics/claude-code/blob/main/README.md)
  under "Language targeting & queued work".
- **SOV (verb-at-end) shapes** like `x लिख;` (= `print x;`) are
  available for `print`, `return`, `assert`, `prove`, `let`,
  `if`, and `while`. They're documented in
  Advanced §7 — *Devanagari purity arc*.
- **The translator works both ways**. If you want to convert
  an English file to Sanskrit (or vice versa) without writing
  by hand, use the tool documented in
  [`tools/README.md`](https://github.com/anthropics/claude-code/blob/main/tools/README.md):
  ```bash
  python3 tools/vani_translate.py --to sanskrit \
      ~/lesson1.vani -o ~/lesson1_sa.vani --add-sri-header
  ```

## Should I write vāṇी in Devanagari?

It's a stylistic choice. The Devanagari surface exists for two
reasons:

1. **Code as you speak.** If your mental language for thinking
   about programs is Sanskrit / Hindi / Marathi, writing keywords
   in those languages removes a context-switch.
2. **Pedagogy in Indian-language CS curricula.** A vāṇी
   classroom can introduce programming without forcing students
   to learn English keywords first.

If neither applies to you, **stay in English** — every example
elsewhere in this tutorial uses the English surface, and the
language is fully expressive there.

## Challenge

Pick any one of your solutions from Lessons 2–11 and translate
it to a Sanskrit-pragma file *by hand* (no tooling). Run it
and confirm the output is byte-identical (except for the
Devanagari numerals). This is a great way to internalize the
keyword table.

---

**Congratulations — you've completed the Beginner track!**

Next steps:
- **Intermediate** — [§1 — Structs and methods →](../intermediate/01_struct_methods.md)
  starts the next track. You'll add custom types, generics,
  dynamic dispatch, and a deep dive on the SMT verifier.
- **Browse the examples**. With the Beginner track behind you,
  `examples/language/english/` should be navigable. Start with
  the ones in the alphabetical first quarter — they're the
  simplest.
- **Try a [design pattern](../intermediate/11_design_patterns.md)**.
  The 22 GoF patterns in
  [`examples/language/english/design_patterns/`](https://github.com/anthropics/claude-code/tree/main/examples/language/english/design_patterns)
  each show a v1 idiom (tagged-struct Composite, int-disc
  Bridge, …) that's worth knowing even if you're not writing
  enterprise code.
