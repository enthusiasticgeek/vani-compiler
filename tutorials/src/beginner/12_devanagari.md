# Beginner 12 â€” Devanagari surface (optional intro)

> **Learning goal**: see vÄá¹‡à¥€'s same program in Sanskrit, get a
> feel for verb-at-end (SOV) shape, and decide whether the
> Devanagari dialect surface is for you. Skip this lesson if
> you only plan to write English-keyword vÄá¹‡à¥€; nothing later in
> the tracks depends on it.

Most programming languages use English keywords because they
were invented by English speakers. vÄá¹‡à¥€ treats that as a
historical accident, not a rule. The same compiler that reads
`fn main()` also reads `à¤®à¥à¤–à¥à¤¯ à¤«à¤²à¤¨`, because the Devanagari
keyword `à¤«à¤²à¤¨` maps to `fn` in the same way `funciÃ³n` maps to
"function" in Spanish. You add one line to your file â€”
`// vani-lang: sanskrit` â€” and the entire keyword vocabulary
switches. The program still runs identically; only the words
you type change. This lesson shows you what that looks like in
practice.

## The program

Save this in `~/lesson12.vani`:

```vani
// à¤¶à¥à¤°à¥€à¥¤
// vani-lang: sanskrit
à¤‰à¤¦à¥à¤¦à¥‡à¤¶à¥à¤¯ "Lesson 12 â€” first program with a Sanskrit pragma.";

à¤•à¤¾à¤°à¥à¤¯ add(a: i64, b: i64) -> i64
à¤…à¤ªà¥‡à¤•à¥à¤·à¤¿à¤¤ a >= 0;
à¤…à¤ªà¥‡à¤•à¥à¤·à¤¿à¤¤ b >= 0;
{
  à¤ªà¥à¤¨à¤°à¤¾à¤—à¤® a + b;
}

à¤•à¤¾à¤°à¥à¤¯ main() -> i64 {
  à¤®à¤¾à¤¨à¤¾ x: i64 = 5;
  à¤®à¤¾à¤¨à¤¾ y: i64 = 7;
  à¤®à¤¾à¤¨à¤¾ sum: i64 = add(x, y);
  à¤¸à¤¿à¤¦à¥à¤§à¤®à¥ sum == 12;
  à¤²à¤¿à¤– sum;
  à¤ªà¥à¤¨à¤°à¤¾à¤—à¤® 0;
}
```

## Compile + run

```bash
vanic run ~/lesson12.vani
```

Expected output:

```
à¥§à¥¨
```

Note the **Devanagari digits**: `à¥§à¥¨` is `12`. vÄá¹‡à¥€'s runtime
PRINT helper detects the file's `// vani-lang:` pragma and
converts integer output to Devanagari numerals (`à¥¦..à¥¯`)
automatically (Phase 1.1).

## What changed vs. the English version

Side-by-side, mapping by row:

| English | Sanskrit (`vani-lang: sanskrit`) |
|---|---|
| `intent "...";` | `à¤‰à¤¦à¥à¤¦à¥‡à¤¶à¥à¤¯ "...";` |
| `fn add(...)` | `à¤•à¤¾à¤°à¥à¤¯ add(...)` |
| `requires a >= 0;` | `à¤…à¤ªà¥‡à¤•à¥à¤·à¤¿à¤¤ a >= 0;` |
| `return a + b;` | `à¤ªà¥à¤¨à¤°à¤¾à¤—à¤® a + b;` |
| `let x: i64 = 5;` | `à¤®à¤¾à¤¨à¤¾ x: i64 = 5;` |
| `assert sum == 12;` | `à¤¸à¤¿à¤¦à¥à¤§à¤®à¥ sum == 12;` |
| `print sum;` | `à¤²à¤¿à¤– sum;` |

Identifier names (`add`, `x`, `y`, `sum`, `main`) and the
`i64` type stay ASCII â€” Devanagari identifiers are supported
but they're a stylistic choice, not a requirement.

## Why it works that way

- **`// vani-lang: <dialect>`** is the *purity pragma*. Inside
  this file, every structure keyword must be a valid
  spelling for the declared dialect (Sanskrit / Hindi /
  Marathi / Nepali / Maithili / Konkani â€” the last three were
  added in Phase 2). Mixing English keywords mid-file is a
  compile error: "language mismatch."
- **The `// à¤¶à¥à¤°à¥€à¥¤` header** is decorative â€” a conventional
  Sanskrit *auspicious-beginning* mark. The compiler ignores it.
  It's a recognizable cue that the file uses the dialect surface.
- **`à¤ªà¥à¤°à¤¤à¤¿` / `à¤¯à¤¦à¤¿` / `à¤¯à¤¾à¤µà¤¤à¥`** (and their Hindi/Marathi
  cousins) take care of `for` / `if` / `while`. The complete
  alias table for all 46 structure keywords is in the
  [README](https://github.com/enthusiasticgeek/vani-compiler/blob/main/README.md)
  under "Language targeting & queued work".
- **SOV (verb-at-end) shapes** like `x à¤²à¤¿à¤–;` (= `print x;`) are
  available for `print`, `return`, `assert`, `prove`, `let`,
  `if`, and `while`. They're documented in
  Advanced Â§7 â€” *Devanagari purity arc*.
- **The translator works both ways**. If you want to convert
  an English file to Sanskrit (or vice versa) without writing
  by hand, use the tool documented in
  [`tools/README.md`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/tools/README.md):
  ```bash
  python3 tools/vani_translate.py --to sanskrit \
      ~/lesson1.vani -o ~/lesson1_sa.vani --add-sri-header
  ```

## Should I write vÄá¹‡à¥€ in Devanagari?

It's a stylistic choice. The Devanagari surface exists for two
reasons:

1. **Code as you speak.** If your mental language for thinking
   about programs is Sanskrit / Hindi / Marathi, writing keywords
   in those languages removes a context-switch.
2. **Pedagogy in Indian-language CS curricula.** A vÄá¹‡à¥€
   classroom can introduce programming without forcing students
   to learn English keywords first.

If neither applies to you, **stay in English** â€” every example
elsewhere in this tutorial uses the English surface, and the
language is fully expressive there.

## Challenge

Pick any one of your solutions from Lessons 2â€“11 and translate
it to a Sanskrit-pragma file *by hand* (no tooling). Run it
and confirm the output is byte-identical (except for the
Devanagari numerals). This is a great way to internalize the
keyword table.

---

**Congratulations â€” you've completed the Beginner track!**

Next steps:
- **Intermediate** â€” [Â§1 â€” Structs and methods â†’](../intermediate/01_struct_methods.md)
  starts the next track. You'll add custom types, generics,
  dynamic dispatch, and a deep dive on the SMT verifier.
- **Browse the examples**. With the Beginner track behind you,
  `examples/language/english/` should be navigable. Start with
  the ones in the alphabetical first quarter â€” they're the
  simplest.
- **Try a [design pattern](../intermediate/11_design_patterns.md)**.
  The 22 GoF patterns in
  [`examples/language/english/design_patterns/`](https://github.com/enthusiasticgeek/vani-compiler/tree/main/examples/language/english/design_patterns)
  each show a v1 idiom (tagged-struct Composite, int-disc
  Bridge, â€¦) that's worth knowing even if you're not writing
  enterprise code.
