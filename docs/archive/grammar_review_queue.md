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
| `for` | `प्रति` *prati* | Medium | "For each"; range syntax is a vāṇī abstraction not directly Sanskrit-rooted |
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

## `downto` keyword-parity sweep (2026-08-13) — needs review

`downto` (descending counterpart of `to`, see `docs/v1_limitations.md`
L29) shipped English-only first, then was extended to all 62 dialects
in the same session as a keyword-parity sweep matching the BUG-170
precedent. Every pick below is a **new coinage** — none have been
native-speaker reviewed. The general pattern: take the dialect's
existing `to`/`until` word (already shipped, already reviewed at
whatever confidence it originally had) and compound it with that
dialect's word for "down"/"below"/"descend", following the same
fused-compound convention already used for `finoa` (Italian,
"until-to fused") and the `EPrint` coinages (`त्रुटिलिख` etc.).
Several picks turned out to already be real, attested words/phrases
in their language (marked **High**); most are engineering-grade
best-effort (**Medium**); a handful for scripts/languages with very
thin training coverage are flagged **Low** and most need a native
speaker before shipping with confidence.

| Dialect | Pick | Confidence | Notes |
|---|---|---|---|
| Sanskrit/Hindi/Marathi | `अधोतक` *adhotak* | Medium | adhas/adho- "below" + tak |
| Bengali | `নিম্নপর্যন্ত` | Medium | nimno "lower" + paryanta |
| Tamil | `கீழ்வரைக்கும்` | Medium | kīḻ "below" + varaikkum |
| Telugu | `దిగువవరకూ` | Medium | diguva "lower" + varakuu |
| Gujarati | `નીચેસુધી` | Medium | niche "down" + sudhee |
| Punjabi | `ਹੇਠਤੱਕ` | Medium | heth "under" + takk |
| Kannada | `ಕೆಳಗೆ` | Medium-High | kelage already natively means "down/below/downward" |
| Malayalam | `താഴെവരെക്കും` | Medium | thazhe "down" + varekkum |
| Odia | `ନିମ୍ନପର୍ଯ୍ୟନ୍ତ` | Medium | nimna "lower" + paryanta |
| Sinhala | `පහළදක්වා` | Medium | pahala "below" + dakvaa |
| Urdu | `نیچےتک` | Medium | neeche "down" + tak |
| Persian | `فروبه` | **Low** | speculative "foru-" (descend) + be |
| Pashto | `ښکتهته` | **Low** | speculative xkta (down) + ta |
| Khmer | `ក្រោមដល់` | **Low** | kraom "below" + dol |
| Burmese | `အောက်သို့` | Medium | auk-thou is a real natural phrase ("downward/to below") |
| Amharic | `ታችድረስ` | **Low** | tachi "down" + dress |
| Tibetan | `མར་བར་དུ` | **Low** | mar "down" + bar-du |
| Cherokee | `ᎡᎳᏗᎬᏛ` | **Low** | eladi (down, uncertain) + gvdv |
| Lao | `ລົງເຖິງ` | Medium-High | long "descend" + theung reads as a natural verb phrase |
| Mongolian (traditional script) | `ᠳᠣᠣᠷᠠᠬᠦᠷᠲᠡᠯᠡ` | **Low** | speculative doora (down) + kürtele |
| Slovak | `nadol` | Medium-High | real Slovak word "downward" |
| Finnish (ascii) | `alasasti` | Medium | alas "down" + asti |
| Catalan (ascii) | `finsavall` | Medium | fins + avall "down(river)" |
| Yoruba | `désílẹ̀` | Medium-Low | dé + sílẹ̀ "down" |
| Hausa (ascii) | `zuwakasa` | Medium | zuwa "toward" + kasa "ground/down" |
| Norwegian (ascii) | `nedtil` | Medium-High | ned "down" + til |
| Danish (ascii) | `nedtil` | Medium-High | ned "down" + til |
| Armenian | `ներքևմինչև` | Low-Medium | nerqev "down" + minchev |
| Georgian | `ქვემოთმდე` | Medium | kvemot "below" + mde |
| Hungarian | `lehatárig` / ascii `lehatarig` | Medium-High | le- (down, productive verbal prefix) + határig |
| Czech | `dolů` / ascii `dolu` | Medium-High | real Czech word "down/downward" |
| Swedish | `nertill` | High | real Swedish word ("at/toward the bottom") |
| Filipino (ascii) | `pababahanggang` | Medium-Low | pababa "downward" + hanggang |
| Vietnamese | `xuốngđến` | Medium | xuống "descend" + đến |
| Romanian | `pânăjos` / ascii `panajos` | Medium | până + jos "down" |
| Dutch (ascii) | `totbeneden` | Medium | tot + beneden "below" |
| Thai | `ลงถึง` | Medium-High | long "down" + thueng (Thai doesn't space-separate words anyway) |
| Polish (ascii) | `dodolu` | Medium | do + dolu "to the bottom" |
| Turkish | `aşağıkadar` / ascii `asagikadar` | Medium-High | aşağı "down" + kadar |
| Malay (ascii) | `hinggabawah` | Medium | hingga + bawah "below" |
| Swahili (ascii) | `hadichini` | Medium-High | hadi + chini "down/below" |
| Italian (ascii) | `finogiu` | Medium-High | fino + giù "down", same fused style as the existing `finoa` |
| Arabic | `إلىأسفل` | Low-Medium | ilā + asfal "below" |
| Greek | `μέχρικάτω` | Medium | méhri + káto "down" |
| Hebrew | `עדלמטה` | Low-Medium | ad + lemata "downward" |
| Indonesian (ascii) | `sampaibawah` / `hinggabawah` | Medium | sampai/hingga + bawah "below" |
| Portuguese | `atébaixo` / ascii `atebaixo` | High | real Portuguese phrase "até baixo" ("until down"), fused |
| Spanish (ascii) | `hastaabajo` | High | real Spanish phrase "hasta abajo" ("until down"), fused |
| French (ascii) | `versbas` | Medium-High | vers + bas "down" |
| German (ascii) | `bisrunter` | Medium-High | bis + runter "down" |
| Korean | `아래까지` | High | arae "down" + kkaji, natural Korean compound |
| Japanese | `下まで` | High | shita "down" + made, natural Japanese compound |
| Mandarin | `下到` | Medium-High | xià "down" + dào |
| Russian (Cyrillic) | `донизу` | High | real Russian word ("down to the bottom") |

**Please revise.** Same process as the SOV-S9 queue above: update the
table, update `src/lexer.rs` (the relevant `*_keyword` function, plus
`spelling_supports_dialect` for the Devanagari entry), add/adjust a
lib test, and regenerate `tools/vani_translate.py` via
`tools/regen_vani_translate_keywords.py`. The Low/Low-Medium entries
(Persian, Pashto, Khmer, Amharic, Tibetan, Cherokee, Mongolian,
Armenian, Arabic, Hebrew, Filipino, Yoruba) are the highest-priority
revision targets — same languages/scripts flagged for human review in
the BUG-171 native-speaker pass.

## `step` keyword-parity sweep (2026-08-23) — needs review

`step` (stride clause on a range-form `for`, see
`docs/v1_limitations.md` L29's remaining gap after `downto`) shipped
English-only first, then was extended to every dialect that already
has a `to`/`downto` spelling, in the same session, matching the
`downto` sweep above exactly. Unlike `downto` (a directional
preposition needing a fused compound), `step` is a plain noun ("a
step/stride/pace"), so most picks below are each dialect's own
existing, real word for a walking step/stage/level — not new
compound coinages — and confidence is correspondingly higher across
the board. A handful of scripts with thin training coverage are
still flagged **Low**, matching the same languages already flagged
Low in the `downto` table above.

| Dialect | Pick | Confidence | Notes |
|---|---|---|---|
| Sanskrit/Hindi/Marathi | `चरण` *caraṇ* | High | real word, "step/stage" |
| Bengali | `ধাপ` *dhap* | High | real word, "step/stage" |
| Tamil | `படி` *paḍi* | High | real word, "step/degree" |
| Telugu | `అడుగు` *aḍugu* | High | real word, "step/footstep" |
| Gujarati | `પગલું` *paglũ* | High | real word, "step" |
| Punjabi | `ਕਦਮ` *kadam* | High | real word, "step" |
| Kannada | `ಹಂತ` *hanta* | High | real word, "step/stage" |
| Malayalam | `ഘട്ടം` *ghaṭṭam* | High | real word, "step/stage" |
| Odia | `ଧାପ` *dhapa* | Medium-High | cognate of Bengali's real word |
| Sinhala | `පියවර` *piyawara* | High | real word, "step" |
| Urdu | `قدم` *qadam* | High | real word, "step" (cognate of Punjabi's) |
| Persian | `گام` *gām* | **Low** | plausible real word ("step/pace") but unreviewed, same confidence class as its `downto` entry |
| Pashto | `ګام` *gām* | **Low** | same caveat as Persian above |
| Khmer | `ជំហាន` *chomhan* | **Low** | plausible real word but unreviewed |
| Burmese | `အဆင့်` *a-sin* | Medium-High | real word, "step/stage/level" |
| Amharic | `ደረጃ` *dereja* | **Low** | plausible real word ("step/level") but unreviewed |
| Tibetan | `གོམ་པ` *gompa* | **Low** | plausible real word ("step/pace") but unreviewed |
| Cherokee | `ᎠᏍᎦᏂ` *asgani* | **Low** | speculative coinage, no confident source |
| Lao | `ຂັ້ນ` *khan* | Medium-High | real word, "step/level" (cognate of Thai's) |
| Mongolian (traditional script) | `ᠠᠯᠬᠤᠮ` *alkhum* | **Low** | speculative transliteration of Cyrillic Mongolian "алхам" into the traditional script |
| Slovak | `krok` | High | real Slovak word, "step" |
| Finnish (ascii) | `askel` | High | real Finnish word, "step" |
| Catalan (ascii) | `pas` | High | real Catalan word, "step" |
| Yoruba | `igbesẹ` | Medium | real word ("step/footstep") but unreviewed |
| Hausa (ascii) | `mataki` | High | real Hausa word, "step" (well-attested, e.g. "mataki-mataki" = step by step) |
| Norwegian (ascii) | `steg` | High | real Norwegian word, "step" |
| Danish (ascii) | `trin` | High | real Danish word, "step" |
| Armenian | `քայլ` *k'ayl* | Medium | real word ("step") but unreviewed |
| Georgian | `ნაბიჯი` *nabiji* | High | real Georgian word, "step" |
| Hungarian | `lépés` / ascii `lepes` | High | real Hungarian word, "step" |
| Czech | `krok` (both native+ascii) | High | real Czech word, "step" (cognate of Slovak's) |
| Swedish | `steg` (both native+ascii) | High | real Swedish word, "step" (cognate of Norwegian's) |
| Filipino (ascii) | `hakbang` | High | real Tagalog word, "step" (well-attested) |
| Vietnamese | `bước` | High | real Vietnamese word, "step" |
| Romanian | `pas` (both native+ascii) | High | real Romanian word, "step" |
| Dutch (ascii) | `stap` | High | real Dutch word, "step" |
| Thai | `ขั้น` *khan* | Medium-High | real word, "step/level" (cognate of Lao's) |
| Polish (ascii) | `krok` | High | real Polish word, "step" (cognate of Czech/Slovak) |
| Turkish | `adım` / ascii `adim` | High | real Turkish word, "step" |
| Malay (ascii) | `langkah` | High | real Malay word, "step" |
| Swahili (ascii) | `hatua` | High | real Swahili word, "step" (well-attested, e.g. "hatua kwa hatua") |
| Italian (ascii) | `passo` | High | real Italian word, "step" |
| Arabic | `خطوة` *khatwa* | Medium | real word ("step") but unreviewed |
| Greek | `βήμα` *vima* | High | real Greek word, "step" |
| Hebrew | `צעד` *tsa'ad* | Medium | real word ("step") but unreviewed |
| Indonesian (ascii) | `langkah` | High | real Indonesian word, "step" (cognate of Malay's) |
| Portuguese | `passo` (both native+ascii) | High | real Portuguese word, "step" (cognate of Italian/Spanish) |
| Spanish (ascii) | `paso` | High | real Spanish word, "step" |
| French (ascii) | `pas` | High | real French word, "step" (cognate of Catalan/Romanian) |
| German (ascii) | `Schritt` | High | real German word, "step" |
| Korean | `단계` *dan-gye* | High | real Korean word, "step/stage" |
| Japanese | `段階` *dankai* | High | real Japanese word, "step/stage" (shares kanji with Korean's) |
| Mandarin | `步长` *bùcháng* | High | the standard CS/math term for "step size/stride" -- literally "step-length" |
| Russian (Cyrillic) | `шаг` *shag* | High | real Russian word, "step" -- also the standard programming term for loop stride |

**Please revise.** Same process as the `downto` queue above: update
the table, update `src/lexer.rs` (the relevant `*_keyword` function),
add/adjust a lib test, and regenerate `tools/vani_translate.py` via
`tools/regen_vani_translate_keywords.py`. The Low entries (Persian,
Pashto, Khmer, Amharic, Tibetan, Cherokee, Mongolian) are the
highest-priority revision targets — the same languages/scripts
already flagged Low in the `downto` table above and in the BUG-171
native-speaker pass.

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
