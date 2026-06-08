# Grammar review queue — Sanskrit / Hindi / Marathi keyword picks

**Status**: SOV-S9 — best-effort initial picks shipped 2026-06-06.
Native-speaker linguists welcome to revise.

## Why this doc exists

vāṇī's lexer ships ~91 Devanagari keyword aliases covering all
46 structure keywords. Some picks are tatsama Sanskrit roots that
work uniformly across all three Indo-Aryan dialects (Sanskrit,
Hindi, Marathi); others are dialect-specific. The current picks
are **engineering-grade best-effort** from a non-native-speaker
implementer; this file is the queue for native-speaker
refinements.

Each entry is a candidate for revision. The format is:

```
| Keyword | Pick(s)          | Confidence | Notes / open question  |
```

**Confidence** ratings:
  - **High** — pick comes from a clear traditional spelling
    (saṁskṛta dictionary entry, official Hindi/Marathi
    technical-term gloss) with no ambiguity.
  - **Medium** — pick is plausible but a native speaker might
    prefer a different shade of meaning. Mark with the
    alternative when known.
  - **Low** — best-effort; explicitly flagged for review.

## How to suggest a revision

Open a PR that:
1. Updates the table below (move the entry to the **Revised**
   section).
2. Updates `src/lexer.rs:devanagari_keyword` (and SOV-S8
   `spelling_supports_dialect` if the new spelling has a
   different dialect tag set).
3. Adds a lib test in `src/lib.rs:tests::devanagari_*` to pin
   the new spelling.
4. Optionally regenerates Devanagari examples in
   `examples/language/{sanskrit,hindi,marathi}/` via the
   translator at `tools/vani_translate.py` (update the
   ALIASES table there too).

## Open queue — picks flagged for native-speaker review

### Sanskrit picks

| Keyword | Pick | Confidence | Notes |
|---|---|---|---|
| `fn` | `कार्य` *kārya* | High | Tatsama; classical "work/function" |
| `let` | `माना` *mānā* | High | "Assume / let" |
| `return` | `पुनरागम` *punarāgama* | Medium | Literal "going back"; alternative `प्रत्यागम` (pratyāgama) more common in classical |
| `if` | `यदि` *yadi* | High | Classical Sanskrit standard |
| `else` | `अन्यथा` *anyathā* | High | "Otherwise"; standard |
| `while` | `यावत्` *yāvat* | Medium | "As long as"; closes with `तर्हि` traditionally; SOV not yet supported for while so the verb alone may feel incomplete |
| `for` | `प्रति` *prati* | Medium | "For each"; range syntax is a vāṇी abstraction not directly Sanskrit-rooted |
| `match` | `मेल` *mela* | Medium | "Join / match"; alternative `संगति` (saṅgati) might be clearer |
| `assert` | `सिद्धम्` *siddham* | High | "Established; proven" |
| `prove` | `प्रमाण` *pramāṇa* | High | "Proof" |
| `print` | `लिख` *likh* | High | Imperative root for "write" |
| `intent` | `उद्देश्य` *uddeśya* | High | "Goal / intent"; tatsama |
| `type` | `प्रकार` *prakāra* | High | "Kind / type"; tatsama |
| `extern` | `बाह्य` *bāhya* | High | "External"; tatsama |
| `invariant` | `अपरिवर्तनीय` *aparivartanīya* | Medium | "Unchanging"; literal but long — alternative `नित्य` (nitya = "eternal/constant") might feel more idiomatic |
| `ref` | `दृष्ट्या` *dṛṣṭyā* | Low | "Via sight / by reference"; instrumental case; pure invention — needs review |
| `mut` | `परिवर्तनीय` *parivartanīya* | Medium | "Changeable"; long; alternative `चल` (cala) shorter but ambiguous |
| `interface` | `संकेत` *saṅket* | Medium | "Sign / protocol"; alternative `अंतरापृष्ठ` (antarāpṛṣṭha) is literal but unusual |

### Hindi picks

| Keyword | Pick | Confidence | Notes |
|---|---|---|---|
| `fn` | `फलन` *phalan* | High | Standard Hindi technical term |
| `let` | `माना` *mānā* | High | Shared with Sanskrit; alternative `मानो` (māno) imperative also accepted |
| `return` | `लौटाओ` *lauṭāo* | High | "Return!" imperative |
| `if` | `अगर` *agar* | High | Standard colloquial Hindi |
| `else` | `वरना` *varnā* | High | "Otherwise" |
| `while` | `जबतक` *jab tak* | High | "Until" |
| `for` | `के लिए` *ke liye* | High | "For the sake of"; multi-word fused |
| `match` | `मिलान` *milān* | High | Standard "matching" |
| `assert` | `सुनिश्चित` *sunishchit* | High | "Ensured" |
| `prove` | `सिद्ध करो` *siddha karo* | High | "Prove!" multi-word; or `प्रमाणित` (pramāṇita) single-word |
| `print` | `लिखो` *likho* | High | "Write!" imperative |
| `mut` | `परिवर्तनीय` *parivartanīya* | Medium | Same as Sanskrit; tatsama |
| `where` | `जहाँ` *jahām̐* | High | "Where" |
| `is` | `है` *hai* | High | "Is" |

### Marathi picks

| Keyword | Pick | Confidence | Notes |
|---|---|---|---|
| `fn` | `कार्य` *kārya* | High | Shared with Sanskrit (tatsama) |
| `let` | `मान` *māna* | High | "Assume" (Marathi imperative form) |
| `return` | `परत` *parat* | High | "Back" |
| `if` | `जर` *jar* | High | Standard Marathi |
| `else` | `नाहीतर` *nāhītar* | High | "Else" |
| `while` | `जोपर्यंत` *jopa­ryanta* | High | "Until" |
| `for` | `साठी` *sāṭhī* | High | "For" |
| `match` | `जुळवा` *juḷvā* | High | "Match!" |
| `assert` | `खात्री` *khātrī* | High | "Certainty" |
| `prove` | `सिद्ध करा` *siddha karā* | High | "Prove!" multi-word; or `दाखवा` (dākhvā) single-word |
| `print` | `लिखो` *likho* | Medium | Shared with Hindi (Marathi historically uses `लिहा` lihā but `लिखो` is widely understood) |
| `mut` | `बदल` *badla* | High | "Change" root |

## Revised picks (post-review)

*(none yet — first review pass pending)*

## Polish-arc 2026-06-08 additions — needs review

The 2026-06-08 multi-arc session shipped four sets of dialect
spellings that should be reviewed alongside the SOV-S9 picks
above:

### Async / await dialect lift

| Keyword | Pick | Dialects | Confidence | Notes |
|---|---|---|---|---|
| `async` | `अतुल्यकालिक` *atulyakālika* | Sanskrit / Hindi / Marathi | Low | Tatsama coinage = "non-synchronous". Used in some Sanskrit-academic CS writing but not native-speaker validated. Wired via parser-level `is_async_ident` helper (not the lexer's keyword table — `async` stays contextual). |
| `async` | `异步` *yìbù* | Mandarin | Medium | Widely-attested CS spelling (Rust/Go/Python translations). |
| `async` | `非同期` *hidouki* | Japanese | Medium | Widely-attested CS spelling. |
| `await` | `प्रतीक्षा` *pratīkṣā* | Sanskrit / Hindi / Marathi | Low | "Wait" — tatsama. |
| `await` | `等候` *děnghòu* | Mandarin | Medium | Chosen because `等待` already means `join` in this dialect. |
| `await` | `待機` *taiki* | Japanese | Medium | "Standby / wait" — common CS spelling. |

### Mandarin keyword table (Phase 10.2)

The full Mandarin keyword set is in `src/lexer.rs:mandarin_keyword`.
~55 spellings — most are direct CS-vocabulary calques widely
attested in Mandarin programming literature (函数 / 让 / 返回 /
如果 / 否则 / 引用 / 可变 / 任务 / 等待 / 等等). Confidence:
**Medium** overall; native-speaker validation queued.

The full list is mirrored in two places:
- `src/lexer.rs:mandarin_keyword` (lexer dispatch)
- `tools/vani_translate.py:ALIASES` (translator)
- `src/lsp.rs:MANDARIN_KEYWORDS` (LSP autocomplete)

A single-source-of-truth refactor is queued in TODO.md.

## Sources / references

- **Sanskrit**: Monier-Williams Sanskrit–English Dictionary
  (1899), Apte's Practical Sanskrit-English Dictionary.
- **Hindi**: McGregor's Oxford Hindi-English Dictionary,
  Hindi tech-term glossaries (CDAC, etc.).
- **Marathi**: Molesworth's Marathi-English Dictionary, modern
  Marathi technical-term references.

## Out-of-scope for SOV-S9

- **Word-order grammar** (SOV vs SVO). The SOV decisions are
  documented separately in [TODO.md §*Sanskrit-derived SOV
  completion*](../TODO.md). This review is for individual
  keyword spellings only.
- **Sanskrit-vs-Hindi-vs-Marathi dialect purity rules**. The
  per-spelling dialect tag table in
  `src/lexer.rs:spelling_supports_dialect` is similarly a
  best-effort encoding of which spellings are natively used in
  which dialect; revisions to that table should accompany any
  spelling changes here.
