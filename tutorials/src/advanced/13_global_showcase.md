# Advanced 13 -- A world tour: vāṇी in your language

> **Learning goal**: see one small, checked-correct vāṇी program run
> correctly after being translated into 13 more human languages spread
> across Asia, Europe, Africa, and a few rare/endangered ones -- and
> understand that this isn't a demo or a mockup. Every file below was
> actually compiled and actually run, on both backends, as part of
> writing this chapter.

Sec.7-9 covered the mechanics: how dialect purity checking works, how
the translator tool converts keywords between languages, and how to
add a brand-new dialect. This chapter is the payoff -- a quick,
concrete look at what that machinery buys a newcomer deciding whether
vāṇी is worth learning.

## The program

Every version below is the *same* program: sum the numbers 1 through
10, double-check the answer is 55, print it. Nothing clever --
readable at a glance in any language, so the only thing changing
between versions is the keywords, not the logic.

```vani
// examples/language/english/global_showcase.vani

intent "sum 1..10, checked correct, then printed";

fn sum_to(n: i64) -> i64 {
  let total: i64 = 0;
  for i from 1 to n + 1 {
    total = total + i;
  }
  return total;
}

fn main() -> i64 {
  let result: i64 = sum_to(10);
  assert result == 55;
  print result;
  return 0;
}
```

`assert result == 55;` is the detail worth pausing on: that's not a
comment or a convention, it's a real compile-time check. The compiler
calls an automated theorem prover (Sec.6, "SMT trace debugging") to
confirm the claim before it ever agrees to produce a binary. Every
translated copy below keeps that same guarantee -- localizing the
*words* never touches the *meaning*.

## The same program, five more ways

A sample across the requested regions -- Asian, European, African, and
one rare/endangered language -- to show how differently the *surface*
can read while the program underneath stays identical.

**Mandarin** (`examples/language/mandarin/global_showcase.vani`):
```vani
函数 sum_to(n: i64) -> i64 {
  让 total: i64 = 0;
  对于 i 从 1 到 n + 1 {
    total = total + i;
  }
  返回 total;
}
```

**Japanese** (`examples/language/japanese/global_showcase.vani`) --
notice the verb moves to the end of the line (`total 戻る;`, "total
return"), because the translator also reorders grammar, not just
words, for languages that put the verb last:
```vani
関数 sum_to(n: i64) -> i64 {
  代入 total: i64 = 0;
  対象 i から 1 まで n + 1 {
    total = total + i;
  }
  total 戻る;
}
```

**Russian** (`examples/language/russian/global_showcase.vani`):
```vani
функция sum_to(n: i64) -> i64 {
  пусть total: i64 = 0;
  для i от 1 до n + 1 {
    total = total + i;
  }
  вернуть total;
}
```

**Swahili** (`examples/language/swahili/global_showcase.vani`):
```vani
kazi sum_to(n: i64) -> i64 {
  acha total: i64 = 0;
  kwa i kutoka 1 hadi n + 1 {
    total = total + i;
  }
  rudi total;
}
```

**Cherokee** (`examples/language/cherokee/global_showcase.vani`) --
written in the Cherokee syllabary, invented by Sequoyah in 1821 and
still actively taught today:
```vani
ᏗᎦᏬᏂᎯᏍᏗ sum_to(n: i64) -> i64 {
  ᎠᏁᎳ total: i64 = 0;
  ᏌᏊ i ᏓᏓᎴᏂᏍᎬ 1 ᎬᏛ n + 1 {
    total = total + i;
  }
  ᏗᎬᏎᏗ total;
}
```

## All 13, and what was checked

Every file below was translated from the English source with
`tools/vani_translate.py`, round-trip-verified (translate out, then
back, confirm the keyword sequence survives), then **actually compiled
and run on both backends** (`vanic run ...` and
`vanic run ... --backend=c`) -- not just eyeballed. All 13 print `55`
on both.

| Region | Language | File |
|---|---|---|
| Asia | Mandarin Chinese | [`examples/language/mandarin/global_showcase.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/mandarin/global_showcase.vani) |
| Asia | Japanese | [`examples/language/japanese/global_showcase.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/japanese/global_showcase.vani) |
| Asia | Korean | [`examples/language/korean/global_showcase.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/korean/global_showcase.vani) |
| Asia | Thai | [`examples/language/thai/global_showcase.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/thai/global_showcase.vani) |
| Europe | German | [`examples/language/german/global_showcase.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/german/global_showcase.vani) |
| Europe | French | [`examples/language/french/global_showcase.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/french/global_showcase.vani) |
| Europe | Spanish | [`examples/language/spanish/global_showcase.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/spanish/global_showcase.vani) |
| Europe | Russian | [`examples/language/russian/global_showcase.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/russian/global_showcase.vani) |
| Africa | Amharic | [`examples/language/amharic/global_showcase.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/amharic/global_showcase.vani) |
| Africa | Swahili | [`examples/language/swahili/global_showcase.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/swahili/global_showcase.vani) |
| Rare / endangered | Cherokee (ᏣᎳᎩ) | [`examples/language/cherokee/global_showcase.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/cherokee/global_showcase.vani) |
| Rare / endangered | Tibetan | [`examples/language/tibetan/global_showcase.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/tibetan/global_showcase.vani) |
| Rare / endangered | Mongolian | [`examples/language/mongolian/global_showcase.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/mongolian/global_showcase.vani) |

That's 13 translations plus the English original -- 14 working
programs, one program's worth of logic, spanning 8 different scripts
(Han, Japanese kana/kanji, Hangul, Thai, Cyrillic, Latin, Ge'ez, and
the Cherokee syllabary). These 13 are a sample, not the full catalog:
vāṇी ships **62 dialects across 26 scripts** in total (see
`README.md`'s Tier-I/II tables) -- everything from this chapter's
picks to Hindi, Arabic, Hebrew, Vietnamese, Yoruba, and dozens more.

## Try it yourself

Pick any row above and run it exactly as shipped:

```bash
vanic run examples/language/mandarin/global_showcase.vani
vanic run examples/language/mandarin/global_showcase.vani --backend=c
```

Both print `55`. Swap in any other file from the table -- or your own
native language, if it's one of the 62 -- and the result is identical.
To see the full keyword mapping for a language, or translate a program
of your own, see Sec.8 ("Writing a cross-language translator
extension"):

```bash
python3 tools/vani_translate.py --from english --to japanese \
  path/to/your_program.vani
```

## What this actually demonstrates

Not that translation is hard (word-substitution isn't), but that vāṇी
treats "which human language do I write code in" as a solved,
first-class, *checked* setting rather than an English-only assumption
bolted on top. The compiler enforces one language per file (no mixing
Devanagari identifiers into a Japanese-pragma file -- Sec.7 covers
that check), the same SMT-backed correctness guarantees apply
regardless of which dialect the source is written in, and both
backends (LLVM and C) produce identical, correct output no matter the
source language. If a reason to try vāṇी was "cool, but does it
actually work outside English?" -- this chapter is the answer.

---

**Congratulations -- you've completed the Advanced track!**

That's the whole tutorial set (Beginner + Intermediate +
Advanced -- 97 lessons). The next-best thing is to:

- Read `examples/language/english/` end to end. With all three
  tracks behind you, every file should be navigable.
- File issues for rough patches you hit. The compiler's most
  honest design feedback is from real programs.
- Contribute a fix or a dialect. The execution plan in
  [`TODO.md`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/TODO.md)
  has phase-by-phase queued work; pick whatever calls to you.

---

**Previous**: [Sec.12 -- Safety-critical standards ->](12_safety_standards.md)
