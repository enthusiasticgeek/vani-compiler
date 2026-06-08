use crate::span::Span;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub span: Span,
    pub message: String,
    /// Optional secondary spans with short notes that point to related
    /// source locations (e.g., the original move site, the prior binding,
    /// the ensures clause violated by a return). These are rendered after
    /// the primary diagnostic with the same source-line + underline format.
    pub related: Vec<(Span, String)>,
}

/// SOV-S9b (2026-06-06): per-file diagnostic localization. When the
/// source has a `// vani-lang: <dialect>` pragma in the first ~10
/// lines, error and note labels render in the declared dialect.
/// The English body of each message is preserved so users can
/// search for it and reach the existing English-language docs +
/// issues; the localization layers a short native-language
/// "त्रुटिः" / "त्रुटि" / "चूक" prefix on top.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum DiagLang {
    Sanskrit,
    Hindi,
    Marathi,
    English,
    // Phase 2 (2026-06-07): Tier I Indo-Aryan dialect extensions.
    // v1 routes their localized labels + prefix tables through
    // the closest existing dialect (Nepali/Maithili → Hindi,
    // Konkani → Marathi); native-language tables can be added as
    // user requests come in.
    Nepali,
    Maithili,
    Konkani,
    // Phase 5b (2026-06-07): Bengali — first non-Devanagari
    // Brahmi script. Its own label table since collapsing to
    // Hindi/Marathi would render Devanagari script in a Bengali-
    // pragma file, which defeats the dialect-aware UX.
    Bengali,
    // Phase 6 (2026-06-07): Brahmi-derived batch. Each has its
    // own native-script label table so users see error labels
    // in the same script they're writing code in.
    Tamil,
    Telugu,
    Gujarati,
    Punjabi,
    // Phase 6 second half (2026-06-07).
    Kannada,
    Malayalam,
    Odia,
    Assamese,    // Bengali-script Indo-Aryan; routes through Bengali labels.
    Sinhala,
    // Phase 12 (2026-06-07): Perso-Arabic.
    Urdu,
    // Phase 12.2/12.3 (2026-06-07): also Perso-Arabic; routed
    // through Urdu's labels in localize_label / localize_message.
    Sindhi,
    PunjabiShahmukhi,
    // Phase 12.4/12.5 (2026-06-07): Persian + Pashto. Persian
    // has its own label table; Pashto routes through Persian.
    Persian,
    Pashto,
    // Phase 8b.2 (2026-06-07): Russian — first Cyrillic-script
    // dialect with its own label table.
    Russian,
    // Phase 8b.1 (2026-06-07): Spanish — first Latin-script
    // Tier II dialect. Latin error labels with Spanish words.
    Spanish,
    // Phase 8b.3 (2026-06-07): French — second Latin-with-
    // accents Tier II dialect.
    French,
    // Phase 9b (2026-06-07): Japanese — first three-script
    // dialect (Hiragana + Katakana + Kanji).
    Japanese,
    // Phase 10.1 (2026-06-07): German — third Latin-with-
    // accents Tier II dialect.
    German,
    // Phase 13.1 (2026-06-07): Korean — first Hangul-script
    // dialect.
    Korean,
    // Phase 13.2 (2026-06-08): Portuguese — fourth Latin-with-
    // accents Tier II dialect.
    Portuguese,
    // Phase 13.3 (2026-06-08): Indonesian — first basic-Latin
    // (no diacritics) Tier II dialect.
    Indonesian,
    // Phase 13.4 (2026-06-08): Greek — first Greek-script
    // dialect.
    Greek,
    // Phase 13.5 (2026-06-08): Hebrew — second RTL-script
    // dialect (after Perso-Arabic).
    Hebrew,
    // Phase 13.6 (2026-06-08): Italian — fifth Latin-with-
    // accents dialect (mostly pure ASCII surface).
    Italian,
    // Phase 13.7 (2026-06-08): Modern Standard Arabic on the
    // existing Script::Arabic infrastructure.
    Arabic,
    // Phase 13.8 (2026-06-08): Polish — first Slavic Latin
    // dialect.
    Polish,
    // Phase 13.9 (2026-06-08): Turkish — Turkic family.
    Turkish,
    // Phase 13.10 (2026-06-08): Malay — sibling of Indonesian.
    Malay,
    // Phase 13.11 (2026-06-08): Swahili — first African dialect.
    Swahili,
}

fn detect_diag_lang(source: &str) -> Option<DiagLang> {
    for (i, line) in source.lines().enumerate() {
        if i > 10 {
            break;
        }
        let trimmed = line.trim_start();
        if !trimmed.starts_with("//") {
            continue;
        }
        let body = trimmed.trim_start_matches("//").trim();
        let Some(rest) = body
            .strip_prefix("vani-lang:")
            .or_else(|| body.strip_prefix("vani-lang :"))
        else {
            continue;
        };
        let name = rest.trim().to_ascii_lowercase();
        return match name.as_str() {
            "sanskrit" | "saṁskṛta" | "sa" => Some(DiagLang::Sanskrit),
            "hindi" | "hindī" | "hi" => Some(DiagLang::Hindi),
            "marathi" | "marāṭhī" | "mr" => Some(DiagLang::Marathi),
            "english" | "en" => Some(DiagLang::English),
            // Phase 2 (2026-06-07): Tier I dialect extensions.
            "nepali" | "nepālī" | "ne" => Some(DiagLang::Nepali),
            "maithili" | "maithilī" | "mai" => Some(DiagLang::Maithili),
            "konkani" | "koṅkaṇī" | "kok" => Some(DiagLang::Konkani),
            "bengali" | "bangla" | "bāṅlā" | "bn" => Some(DiagLang::Bengali),
            "tamil" | "tamiḻ" | "ta" => Some(DiagLang::Tamil),
            "telugu" | "telugū" | "te" => Some(DiagLang::Telugu),
            "gujarati" | "gujarātī" | "gu" => Some(DiagLang::Gujarati),
            "punjabi" | "pañjābī" | "pa" => Some(DiagLang::Punjabi),
            // Phase 6 second half (2026-06-07).
            "kannada" | "kannaḍa" | "kn" => Some(DiagLang::Kannada),
            "malayalam" | "malayāḷam" | "ml" => Some(DiagLang::Malayalam),
            "odia" | "oṛiā" | "oriya" | "or" => Some(DiagLang::Odia),
            "assamese" | "ɔxɔmia" | "as" => Some(DiagLang::Assamese),
            "sinhala" | "siṁhala" | "si" => Some(DiagLang::Sinhala),
            "urdu" | "urdū" | "ur" => Some(DiagLang::Urdu),
            "sindhi" | "sindhī" | "sd" => Some(DiagLang::Sindhi),
            "punjabi-shahmukhi" | "shahmukhi" | "pnb"
                => Some(DiagLang::PunjabiShahmukhi),
            "persian" | "farsi" | "fārsī" | "fa" => Some(DiagLang::Persian),
            "pashto" | "paṣ́tō" | "ps" => Some(DiagLang::Pashto),
            // Phase 8b.2 (2026-06-07): first Cyrillic dialect.
            "russian" | "русский" | "ru" => Some(DiagLang::Russian),
            // Phase 8b.1 (2026-06-07): first Latin Tier II dialect.
            "spanish" | "español" | "castellano" | "es"
                => Some(DiagLang::Spanish),
            // Phase 8b.3 (2026-06-07): second Latin-with-accents.
            "french" | "français" | "francais" | "fr"
                => Some(DiagLang::French),
            // Phase 9b (2026-06-07): first three-script dialect.
            "japanese" | "日本語" | "nihongo" | "ja"
                => Some(DiagLang::Japanese),
            // Phase 10.1 (2026-06-07): third Latin-with-accents.
            "german" | "deutsch" | "de" => Some(DiagLang::German),
            // Phase 13.1 (2026-06-07): first Hangul-script dialect.
            "korean" | "한국어" | "hangugeo" | "ko"
                => Some(DiagLang::Korean),
            // Phase 13.2 (2026-06-08): fourth Latin-with-accents.
            "portuguese" | "português" | "portugues" | "pt"
                | "brasileiro" | "brasil"
                => Some(DiagLang::Portuguese),
            // Phase 13.3 (2026-06-08): first basic-Latin dialect.
            "indonesian" | "indonesia" | "bahasa" | "id"
                => Some(DiagLang::Indonesian),
            // Phase 13.4 (2026-06-08): first Greek-script dialect.
            "greek" | "ελληνικά" | "ellinika" | "el"
                => Some(DiagLang::Greek),
            // Phase 13.5 (2026-06-08): second RTL dialect.
            "hebrew" | "עברית" | "ivrit" | "he" | "iw"
                => Some(DiagLang::Hebrew),
            // Phase 13.6 (2026-06-08): fifth Latin-with-accents.
            "italian" | "italiano" | "it" => Some(DiagLang::Italian),
            // Phase 13.7 (2026-06-08): Modern Standard Arabic.
            "arabic" | "العربية" | "arabi" | "ar"
                => Some(DiagLang::Arabic),
            // Phase 13.8 (2026-06-08): Polish.
            "polish" | "polski" | "pl" => Some(DiagLang::Polish),
            // Phase 13.9 (2026-06-08): Turkish.
            "turkish" | "türkçe" | "turkce" | "tr"
                => Some(DiagLang::Turkish),
            // Phase 13.10 (2026-06-08): Malay.
            "malay" | "melayu" | "bahasa-melayu" | "ms"
                => Some(DiagLang::Malay),
            // Phase 13.11 (2026-06-08): Swahili.
            "swahili" | "kiswahili" | "sw"
                => Some(DiagLang::Swahili),
            _ => None,
        };
    }
    None
}

fn localize_label(level: &str, lang: Option<DiagLang>) -> String {
    // Phase 2 + 5b + 6: route Tier-I-dialect labels through
    // their nearest existing dialect. Assamese rides Bengali's
    // labels since it shares the script.
    let lang = lang.map(|l| match l {
        DiagLang::Nepali | DiagLang::Maithili => DiagLang::Hindi,
        DiagLang::Konkani => DiagLang::Marathi,
        DiagLang::Assamese => DiagLang::Bengali,
        // Phase 12.2/12.3: Sindhi + Shahmukhi route through
        // Urdu labels (same Perso-Arabic script).
        DiagLang::Sindhi | DiagLang::PunjabiShahmukhi => DiagLang::Urdu,
        // Phase 12.5: Pashto routes through Persian labels.
        DiagLang::Pashto => DiagLang::Persian,
        other => other,
    });
    match (level, lang) {
        ("error", Some(DiagLang::Sanskrit)) => "त्रुटिः (error)".to_string(),
        ("error", Some(DiagLang::Hindi)) => "त्रुटि (error)".to_string(),
        ("error", Some(DiagLang::Marathi)) => "चूक (error)".to_string(),
        ("error", Some(DiagLang::Bengali)) => "ত্রুটি (error)".to_string(),
        ("error", Some(DiagLang::Tamil)) => "பிழை (error)".to_string(),
        ("error", Some(DiagLang::Telugu)) => "లోపం (error)".to_string(),
        ("error", Some(DiagLang::Gujarati)) => "ભૂલ (error)".to_string(),
        ("error", Some(DiagLang::Punjabi)) => "ਗਲਤੀ (error)".to_string(),
        ("error", Some(DiagLang::Kannada)) => "ದೋಷ (error)".to_string(),
        ("error", Some(DiagLang::Malayalam)) => "പിശക് (error)".to_string(),
        ("error", Some(DiagLang::Odia)) => "ତ୍ରୁଟି (error)".to_string(),
        ("error", Some(DiagLang::Sinhala)) => "දෝෂය (error)".to_string(),
        ("error", Some(DiagLang::Urdu)) => "غلطی (error)".to_string(),
        ("error", Some(DiagLang::Persian)) => "خطا (error)".to_string(),
        ("error", Some(DiagLang::Russian)) => "ошибка (error)".to_string(),
        ("error", Some(DiagLang::Spanish)) => "error".to_string(),
        ("error", Some(DiagLang::French)) => "erreur (error)".to_string(),
        ("error", Some(DiagLang::Japanese)) => "エラー (error)".to_string(),
        ("error", Some(DiagLang::German)) => "Fehler (error)".to_string(),
        ("error", Some(DiagLang::Korean)) => "오류 (error)".to_string(),
        ("error", Some(DiagLang::Portuguese)) => "erro (error)".to_string(),
        ("error", Some(DiagLang::Indonesian)) => "kesalahan (error)".to_string(),
        ("error", Some(DiagLang::Greek)) => "σφάλμα (error)".to_string(),
        ("error", Some(DiagLang::Hebrew)) => "שגיאה (error)".to_string(),
        ("error", Some(DiagLang::Italian)) => "errore (error)".to_string(),
        ("error", Some(DiagLang::Arabic)) => "خطأ (error)".to_string(),
        ("error", Some(DiagLang::Polish)) => "błąd (error)".to_string(),
        ("error", Some(DiagLang::Turkish)) => "hata (error)".to_string(),
        ("error", Some(DiagLang::Malay)) => "ralat (error)".to_string(),
        ("error", Some(DiagLang::Swahili)) => "kosa (error)".to_string(),
        ("note", Some(DiagLang::Sanskrit)) => "टिप्पणी (note)".to_string(),
        ("note", Some(DiagLang::Hindi)) => "टिप्पणी (note)".to_string(),
        ("note", Some(DiagLang::Marathi)) => "टीप (note)".to_string(),
        ("note", Some(DiagLang::Bengali)) => "টীকা (note)".to_string(),
        ("note", Some(DiagLang::Tamil)) => "குறிப்பு (note)".to_string(),
        ("note", Some(DiagLang::Telugu)) => "గమనిక (note)".to_string(),
        ("note", Some(DiagLang::Gujarati)) => "નોંધ (note)".to_string(),
        ("note", Some(DiagLang::Punjabi)) => "ਨੋਟ (note)".to_string(),
        ("note", Some(DiagLang::Kannada)) => "ಟಿಪ್ಪಣಿ (note)".to_string(),
        ("note", Some(DiagLang::Malayalam)) => "കുറിപ്പ് (note)".to_string(),
        ("note", Some(DiagLang::Odia)) => "ଟିପ୍ପଣୀ (note)".to_string(),
        ("note", Some(DiagLang::Sinhala)) => "සටහන (note)".to_string(),
        ("note", Some(DiagLang::Urdu)) => "نوٹ (note)".to_string(),
        ("note", Some(DiagLang::Persian)) => "یادداشت (note)".to_string(),
        ("note", Some(DiagLang::Russian)) => "примечание (note)".to_string(),
        ("note", Some(DiagLang::Spanish)) => "nota (note)".to_string(),
        ("note", Some(DiagLang::French)) => "remarque (note)".to_string(),
        ("note", Some(DiagLang::Japanese)) => "注記 (note)".to_string(),
        ("note", Some(DiagLang::German)) => "Hinweis (note)".to_string(),
        ("note", Some(DiagLang::Korean)) => "참고 (note)".to_string(),
        ("note", Some(DiagLang::Portuguese)) => "nota (note)".to_string(),
        ("note", Some(DiagLang::Indonesian)) => "catatan (note)".to_string(),
        ("note", Some(DiagLang::Greek)) => "σημείωση (note)".to_string(),
        ("note", Some(DiagLang::Hebrew)) => "הערה (note)".to_string(),
        ("note", Some(DiagLang::Italian)) => "nota (note)".to_string(),
        ("note", Some(DiagLang::Arabic)) => "ملاحظة (note)".to_string(),
        ("note", Some(DiagLang::Polish)) => "uwaga (note)".to_string(),
        ("note", Some(DiagLang::Turkish)) => "not (note)".to_string(),
        ("note", Some(DiagLang::Malay)) => "nota (note)".to_string(),
        ("note", Some(DiagLang::Swahili)) => "kumbuka (note)".to_string(),
        _ => level.to_string(),
    }
}

/// Best-effort message-prefix localization for the highest-
/// frequency error families. The English body is appended after a
/// `—` so the user can match the existing wording in docs/issues.
/// Unknown prefixes pass through with no translation.
fn localize_message(message: &str, lang: Option<DiagLang>) -> String {
    let Some(lang) = lang else {
        return message.to_string();
    };
    if lang == DiagLang::English {
        return message.to_string();
    }
    // Phase 2 + 5b + 6: Nepali/Maithili share Hindi's prefix
    // table; Konkani shares Marathi's; Assamese shares Bengali's
    // (same script). Bengali / Tamil / Telugu / Gujarati /
    // Punjabi / Kannada / Malayalam / Odia / Sinhala have their
    // own. Collapse first so the match is one arm per script.
    let lang = match lang {
        DiagLang::Nepali | DiagLang::Maithili => DiagLang::Hindi,
        DiagLang::Konkani => DiagLang::Marathi,
        DiagLang::Assamese => DiagLang::Bengali,
        // Phase 12.2/12.3 (2026-06-07): Perso-Arabic siblings
        // share Urdu's prefix table.
        DiagLang::Sindhi | DiagLang::PunjabiShahmukhi => DiagLang::Urdu,
        // Phase 12.5: Pashto routes through Persian.
        DiagLang::Pashto => DiagLang::Persian,
        other => other,
    };
    let table = match lang {
        DiagLang::Sanskrit => &[
            ("expected ", "अपेक्षितम् "),
            ("unknown variable", "अज्ञातं चरम् (unknown variable)"),
            ("unknown function", "अज्ञातं कार्यम् (unknown function)"),
            ("unknown struct", "अज्ञाता संरचना (unknown struct)"),
            ("type mismatch", "प्रकारभेदः (type mismatch)"),
            ("cannot prove", "प्रमाणीकर्तुम् अशक्यम् (cannot prove)"),
            ("function ", "कार्यम् "),
            ("language mismatch", "भाषाभेदः (language mismatch)"),
            ("invalid", "अमान्यम् (invalid)"),
            ("integer literal", "पूर्णांकमूल्यम् (integer literal)"),
            ("float literal", "दशांशमूल्यम् (float literal)"),
        ][..],
        DiagLang::Hindi => &[
            ("expected ", "अपेक्षित "),
            ("unknown variable", "अज्ञात चर (unknown variable)"),
            ("unknown function", "अज्ञात फलन (unknown function)"),
            ("unknown struct", "अज्ञात संरचना (unknown struct)"),
            ("type mismatch", "प्रकार मेल नहीं (type mismatch)"),
            ("cannot prove", "प्रमाणित नहीं कर सकते (cannot prove)"),
            ("function ", "फलन "),
            ("language mismatch", "भाषा बेमेल (language mismatch)"),
            ("invalid", "अमान्य (invalid)"),
            ("integer literal", "पूर्णांक मान (integer literal)"),
            ("float literal", "दशांश मान (float literal)"),
        ][..],
        DiagLang::Marathi => &[
            ("expected ", "अपेक्षित "),
            ("unknown variable", "अज्ञात चल (unknown variable)"),
            ("unknown function", "अज्ञात कार्य (unknown function)"),
            ("unknown struct", "अज्ञात संरचना (unknown struct)"),
            ("type mismatch", "प्रकार जुळत नाही (type mismatch)"),
            ("cannot prove", "सिद्ध करता येत नाही (cannot prove)"),
            ("function ", "कार्य "),
            ("language mismatch", "भाषा जुळत नाही (language mismatch)"),
            ("invalid", "अवैध (invalid)"),
            ("integer literal", "पूर्णांक मूल्य (integer literal)"),
            ("float literal", "दशांश मूल्य (float literal)"),
        ][..],
        DiagLang::English => return message.to_string(),
        // Phase 5b (2026-06-07): Bengali prefix table — same
        // shape as the Devanagari ones but with Bengali-script
        // wording. v1 ships a starter set; expand as user
        // requests come in.
        DiagLang::Bengali => &[
            ("expected ", "প্রত্যাশিত "),
            ("unknown variable", "অজানা চলক (unknown variable)"),
            ("unknown function", "অজানা ফাংশন (unknown function)"),
            ("unknown struct", "অজানা গঠন (unknown struct)"),
            ("type mismatch", "প্রকার অমিল (type mismatch)"),
            ("cannot prove", "প্রমাণ করা যায় না (cannot prove)"),
            ("function ", "ফাংশন "),
            ("language mismatch", "ভাষা অমিল (language mismatch)"),
            ("script mismatch", "লিপি অমিল (script mismatch)"),
            ("invalid", "অবৈধ (invalid)"),
            ("integer literal", "পূর্ণসংখ্যা মান (integer literal)"),
            ("float literal", "দশমিক মান (float literal)"),
        ][..],
        // Phase 6 (2026-06-07): Tamil / Telugu / Gujarati /
        // Punjabi v1 starter tables. Shorter than Sanskrit/
        // Hindi/Marathi/Bengali because the prefix-translation
        // surface is still being calibrated for these scripts;
        // unknown prefixes pass through with no translation
        // (still useful — the error labels above already
        // localized "error" / "note").
        DiagLang::Tamil => &[
            ("expected ", "எதிர்பார்க்கப்பட்டது "),
            ("unknown variable", "தெரியாத மாறி (unknown variable)"),
            ("unknown function", "தெரியாத செயல்பாடு (unknown function)"),
            ("type mismatch", "வகை பொருந்தவில்லை (type mismatch)"),
            ("cannot prove", "நிரூபிக்க இயலவில்லை (cannot prove)"),
            ("language mismatch", "மொழி பொருந்தவில்லை (language mismatch)"),
            ("script mismatch", "எழுத்துப் பொருந்தவில்லை (script mismatch)"),
        ][..],
        DiagLang::Telugu => &[
            ("expected ", "ఆశించినది "),
            ("unknown variable", "తెలియని చరం (unknown variable)"),
            ("unknown function", "తెలియని ఫంక్షన్ (unknown function)"),
            ("type mismatch", "రకం సరిపోలడం లేదు (type mismatch)"),
            ("cannot prove", "నిరూపించలేము (cannot prove)"),
            ("language mismatch", "భాష సరిపోలడం లేదు (language mismatch)"),
            ("script mismatch", "లిపి సరిపోలడం లేదు (script mismatch)"),
        ][..],
        DiagLang::Gujarati => &[
            ("expected ", "અપેક્ષિત "),
            ("unknown variable", "અજાણ્યું ચલ (unknown variable)"),
            ("unknown function", "અજાણ્યું કાર્ય (unknown function)"),
            ("type mismatch", "પ્રકાર મેળ ખાતો નથી (type mismatch)"),
            ("cannot prove", "સાબિત કરી શકાતું નથી (cannot prove)"),
            ("language mismatch", "ભાષા મેળ ખાતી નથી (language mismatch)"),
            ("script mismatch", "લિપિ મેળ ખાતી નથી (script mismatch)"),
        ][..],
        DiagLang::Punjabi => &[
            ("expected ", "ਉਮੀਦ ਕੀਤੀ ਗਈ "),
            ("unknown variable", "ਅਣਜਾਣ ਚਲ (unknown variable)"),
            ("unknown function", "ਅਣਜਾਣ ਕਾਰਜ (unknown function)"),
            ("type mismatch", "ਕਿਸਮ ਮੇਲ ਨਹੀਂ ਖਾਂਦੀ (type mismatch)"),
            ("cannot prove", "ਸਾਬਤ ਨਹੀਂ ਹੋ ਸਕਦਾ (cannot prove)"),
            ("language mismatch", "ਭਾਸ਼ਾ ਮੇਲ ਨਹੀਂ ਖਾਂਦੀ (language mismatch)"),
            ("script mismatch", "ਲਿਪੀ ਮੇਲ ਨਹੀਂ ਖਾਂਦੀ (script mismatch)"),
        ][..],
        DiagLang::Kannada => &[
            ("expected ", "ನಿರೀಕ್ಷಿತ "),
            ("unknown variable", "ಅಪರಿಚಿತ ಚರ (unknown variable)"),
            ("unknown function", "ಅಪರಿಚಿತ ಕಾರ್ಯ (unknown function)"),
            ("type mismatch", "ಪ್ರಕಾರ ಹೊಂದಿಕೆಯಿಲ್ಲ (type mismatch)"),
            ("cannot prove", "ಸಾಬೀತುಪಡಿಸಲಾಗದು (cannot prove)"),
            ("language mismatch", "ಭಾಷೆ ಹೊಂದಿಕೆಯಿಲ್ಲ (language mismatch)"),
            ("script mismatch", "ಲಿಪಿ ಹೊಂದಿಕೆಯಿಲ್ಲ (script mismatch)"),
        ][..],
        DiagLang::Malayalam => &[
            ("expected ", "പ്രതീക്ഷിച്ച "),
            ("unknown variable", "അജ്ഞാത ചരം (unknown variable)"),
            ("unknown function", "അജ്ഞാത കാര്യം (unknown function)"),
            ("type mismatch", "തരം പൊരുത്തപ്പെടുന്നില്ല (type mismatch)"),
            ("cannot prove", "തെളിയിക്കാൻ കഴിയില്ല (cannot prove)"),
            ("language mismatch", "ഭാഷ പൊരുത്തപ്പെടുന്നില്ല (language mismatch)"),
            ("script mismatch", "ലിപി പൊരുത്തപ്പെടുന്നില്ല (script mismatch)"),
        ][..],
        DiagLang::Odia => &[
            ("expected ", "ଆଶା କରାଯାଇଥିଲା "),
            ("unknown variable", "ଅଜଣା ଚଳ (unknown variable)"),
            ("unknown function", "ଅଜଣା କାର୍ଯ୍ୟ (unknown function)"),
            ("type mismatch", "ପ୍ରକାର ମେଳ ନୁହେଁ (type mismatch)"),
            ("cannot prove", "ପ୍ରମାଣ କରିପାରିବ ନାହିଁ (cannot prove)"),
            ("language mismatch", "ଭାଷା ମେଳ ନୁହେଁ (language mismatch)"),
            ("script mismatch", "ଲିପି ମେଳ ନୁହେଁ (script mismatch)"),
        ][..],
        DiagLang::Sinhala => &[
            ("expected ", "අපේක්ෂිත "),
            ("unknown variable", "නොදන්නා විචල්‍ය (unknown variable)"),
            ("unknown function", "නොදන්නා කාර්යය (unknown function)"),
            ("type mismatch", "වර්ග නොගැලපීම (type mismatch)"),
            ("cannot prove", "ඔප්පු කළ නොහැක (cannot prove)"),
            ("language mismatch", "භාෂා නොගැලපීම (language mismatch)"),
            ("script mismatch", "අකුරු නොගැලපීම (script mismatch)"),
        ][..],
        DiagLang::Urdu => &[
            ("expected ", "متوقع "),
            ("unknown variable", "نامعلوم متغیر (unknown variable)"),
            ("unknown function", "نامعلوم فنکشن (unknown function)"),
            ("type mismatch", "قسم میں اختلاف (type mismatch)"),
            ("cannot prove", "ثابت نہیں کیا جا سکتا (cannot prove)"),
            ("language mismatch", "زبان میں اختلاف (language mismatch)"),
            ("script mismatch", "رسم الخط میں اختلاف (script mismatch)"),
        ][..],
        DiagLang::Persian => &[
            ("expected ", "مورد انتظار "),
            ("unknown variable", "متغیر ناشناخته (unknown variable)"),
            ("unknown function", "تابع ناشناخته (unknown function)"),
            ("type mismatch", "عدم تطابق نوع (type mismatch)"),
            ("cannot prove", "قابل اثبات نیست (cannot prove)"),
            ("language mismatch", "عدم تطابق زبان (language mismatch)"),
            ("script mismatch", "عدم تطابق خط (script mismatch)"),
        ][..],
        DiagLang::Russian => &[
            ("expected ", "ожидалось "),
            ("unknown variable", "неизвестная переменная (unknown variable)"),
            ("unknown function", "неизвестная функция (unknown function)"),
            ("unknown struct", "неизвестная структура (unknown struct)"),
            ("type mismatch", "несоответствие типов (type mismatch)"),
            ("cannot prove", "не удаётся доказать (cannot prove)"),
            ("language mismatch", "несоответствие языка (language mismatch)"),
            ("script mismatch", "несоответствие письменности (script mismatch)"),
            ("invalid", "недопустимо (invalid)"),
            ("integer literal", "целочисленный литерал (integer literal)"),
            ("float literal", "вещественный литерал (float literal)"),
        ][..],
        DiagLang::Spanish => &[
            ("expected ", "esperado "),
            ("unknown variable", "variable desconocida (unknown variable)"),
            ("unknown function", "función desconocida (unknown function)"),
            ("unknown struct", "estructura desconocida (unknown struct)"),
            ("type mismatch", "tipos incompatibles (type mismatch)"),
            ("cannot prove", "no se puede probar (cannot prove)"),
            ("function ", "función "),
            ("language mismatch", "idioma incompatible (language mismatch)"),
            ("invalid", "inválido (invalid)"),
            ("integer literal", "literal entero (integer literal)"),
            ("float literal", "literal decimal (float literal)"),
        ][..],
        DiagLang::French => &[
            ("expected ", "attendu "),
            ("unknown variable", "variable inconnue (unknown variable)"),
            ("unknown function", "fonction inconnue (unknown function)"),
            ("unknown struct", "structure inconnue (unknown struct)"),
            ("type mismatch", "types incompatibles (type mismatch)"),
            ("cannot prove", "impossible à prouver (cannot prove)"),
            ("function ", "fonction "),
            ("language mismatch", "langue incompatible (language mismatch)"),
            ("invalid", "invalide (invalid)"),
            ("integer literal", "littéral entier (integer literal)"),
            ("float literal", "littéral décimal (float literal)"),
        ][..],
        DiagLang::Japanese => &[
            ("expected ", "期待される "),
            ("unknown variable", "未定義の変数 (unknown variable)"),
            ("unknown function", "未定義の関数 (unknown function)"),
            ("unknown struct", "未定義の構造体 (unknown struct)"),
            ("type mismatch", "型が一致しません (type mismatch)"),
            ("cannot prove", "証明できません (cannot prove)"),
            ("function ", "関数 "),
            ("language mismatch", "言語が一致しません (language mismatch)"),
            ("script mismatch", "文字体系が一致しません (script mismatch)"),
            ("invalid", "無効 (invalid)"),
            ("integer literal", "整数リテラル (integer literal)"),
            ("float literal", "浮動小数点リテラル (float literal)"),
        ][..],
        DiagLang::German => &[
            ("expected ", "erwartet "),
            ("unknown variable", "unbekannte Variable (unknown variable)"),
            ("unknown function", "unbekannte Funktion (unknown function)"),
            ("unknown struct", "unbekannte Struktur (unknown struct)"),
            ("type mismatch", "Typenkonflikt (type mismatch)"),
            ("cannot prove", "kann nicht beweisen (cannot prove)"),
            ("function ", "Funktion "),
            ("language mismatch", "Sprachkonflikt (language mismatch)"),
            ("invalid", "ungültig (invalid)"),
            ("integer literal", "Ganzzahlliteral (integer literal)"),
            ("float literal", "Dezimalliteral (float literal)"),
        ][..],
        DiagLang::Korean => &[
            ("expected ", "예상 "),
            ("unknown variable", "알 수 없는 변수 (unknown variable)"),
            ("unknown function", "알 수 없는 함수 (unknown function)"),
            ("unknown struct", "알 수 없는 구조체 (unknown struct)"),
            ("type mismatch", "타입 불일치 (type mismatch)"),
            ("cannot prove", "증명할 수 없음 (cannot prove)"),
            ("function ", "함수 "),
            ("language mismatch", "언어 불일치 (language mismatch)"),
            ("script mismatch", "문자 체계 불일치 (script mismatch)"),
            ("invalid", "잘못됨 (invalid)"),
            ("integer literal", "정수 리터럴 (integer literal)"),
            ("float literal", "실수 리터럴 (float literal)"),
        ][..],
        DiagLang::Portuguese => &[
            ("expected ", "esperado "),
            ("unknown variable", "variável desconhecida (unknown variable)"),
            ("unknown function", "função desconhecida (unknown function)"),
            ("unknown struct", "estrutura desconhecida (unknown struct)"),
            ("type mismatch", "tipos incompatíveis (type mismatch)"),
            ("cannot prove", "não é possível provar (cannot prove)"),
            ("function ", "função "),
            ("language mismatch", "idioma incompatível (language mismatch)"),
            ("invalid", "inválido (invalid)"),
            ("integer literal", "literal inteiro (integer literal)"),
            ("float literal", "literal decimal (float literal)"),
        ][..],
        DiagLang::Indonesian => &[
            ("expected ", "diharapkan "),
            ("unknown variable", "variabel tidak dikenal (unknown variable)"),
            ("unknown function", "fungsi tidak dikenal (unknown function)"),
            ("unknown struct", "struktur tidak dikenal (unknown struct)"),
            ("type mismatch", "tipe tidak cocok (type mismatch)"),
            ("cannot prove", "tidak dapat membuktikan (cannot prove)"),
            ("function ", "fungsi "),
            ("language mismatch", "bahasa tidak cocok (language mismatch)"),
            ("invalid", "tidak valid (invalid)"),
            ("integer literal", "literal bilangan bulat (integer literal)"),
            ("float literal", "literal desimal (float literal)"),
        ][..],
        DiagLang::Greek => &[
            ("expected ", "αναμένεται "),
            ("unknown variable", "άγνωστη μεταβλητή (unknown variable)"),
            ("unknown function", "άγνωστη συνάρτηση (unknown function)"),
            ("unknown struct", "άγνωστη δομή (unknown struct)"),
            ("type mismatch", "ασυμβατότητα τύπων (type mismatch)"),
            ("cannot prove", "αδύνατη απόδειξη (cannot prove)"),
            ("function ", "συνάρτηση "),
            ("language mismatch", "ασυμβατότητα γλωσσών (language mismatch)"),
            ("script mismatch", "ασυμβατότητα γραφών (script mismatch)"),
            ("invalid", "άκυρο (invalid)"),
            ("integer literal", "ακέραιο σταθερό (integer literal)"),
            ("float literal", "δεκαδικό σταθερό (float literal)"),
        ][..],
        DiagLang::Hebrew => &[
            ("expected ", "צפוי "),
            ("unknown variable", "משתנה לא ידוע (unknown variable)"),
            ("unknown function", "פונקציה לא ידועה (unknown function)"),
            ("unknown struct", "מבנה לא ידוע (unknown struct)"),
            ("type mismatch", "חוסר התאמת טיפוס (type mismatch)"),
            ("cannot prove", "לא ניתן להוכיח (cannot prove)"),
            ("function ", "פונקציה "),
            ("language mismatch", "חוסר התאמת שפה (language mismatch)"),
            ("script mismatch", "חוסר התאמת כתב (script mismatch)"),
            ("invalid", "לא חוקי (invalid)"),
            ("integer literal", "מספר שלם (integer literal)"),
            ("float literal", "מספר עשרוני (float literal)"),
        ][..],
        DiagLang::Italian => &[
            ("expected ", "atteso "),
            ("unknown variable", "variabile sconosciuta (unknown variable)"),
            ("unknown function", "funzione sconosciuta (unknown function)"),
            ("unknown struct", "struttura sconosciuta (unknown struct)"),
            ("type mismatch", "tipi incompatibili (type mismatch)"),
            ("cannot prove", "non si può dimostrare (cannot prove)"),
            ("function ", "funzione "),
            ("language mismatch", "lingua incompatibile (language mismatch)"),
            ("invalid", "non valido (invalid)"),
            ("integer literal", "letterale intero (integer literal)"),
            ("float literal", "letterale decimale (float literal)"),
        ][..],
        DiagLang::Arabic => &[
            ("expected ", "متوقع "),
            ("unknown variable", "متغير غير معروف (unknown variable)"),
            ("unknown function", "دالة غير معروفة (unknown function)"),
            ("unknown struct", "بنية غير معروفة (unknown struct)"),
            ("type mismatch", "عدم تطابق النوع (type mismatch)"),
            ("cannot prove", "تعذر الإثبات (cannot prove)"),
            ("function ", "دالة "),
            ("language mismatch", "عدم تطابق اللغة (language mismatch)"),
            ("invalid", "غير صالح (invalid)"),
            ("integer literal", "عدد صحيح (integer literal)"),
            ("float literal", "عدد عشري (float literal)"),
        ][..],
        DiagLang::Polish => &[
            ("expected ", "oczekiwano "),
            ("unknown variable", "nieznana zmienna (unknown variable)"),
            ("unknown function", "nieznana funkcja (unknown function)"),
            ("type mismatch", "niezgodność typów (type mismatch)"),
            ("cannot prove", "nie można udowodnić (cannot prove)"),
            ("function ", "funkcja "),
            ("invalid", "nieprawidłowy (invalid)"),
            ("integer literal", "literał całkowity (integer literal)"),
            ("float literal", "literał dziesiętny (float literal)"),
        ][..],
        DiagLang::Turkish => &[
            ("expected ", "beklenen "),
            ("unknown variable", "bilinmeyen değişken (unknown variable)"),
            ("unknown function", "bilinmeyen işlev (unknown function)"),
            ("type mismatch", "tür uyuşmazlığı (type mismatch)"),
            ("cannot prove", "kanıtlanamadı (cannot prove)"),
            ("function ", "işlev "),
            ("invalid", "geçersiz (invalid)"),
            ("integer literal", "tam sayı sabiti (integer literal)"),
            ("float literal", "ondalık sabit (float literal)"),
        ][..],
        DiagLang::Malay => &[
            ("expected ", "dijangka "),
            ("unknown variable", "pemboleh ubah tidak diketahui (unknown variable)"),
            ("unknown function", "fungsi tidak diketahui (unknown function)"),
            ("type mismatch", "jenis tidak sepadan (type mismatch)"),
            ("cannot prove", "tidak dapat membuktikan (cannot prove)"),
            ("function ", "fungsi "),
            ("invalid", "tidak sah (invalid)"),
        ][..],
        DiagLang::Swahili => &[
            ("expected ", "ilitarajiwa "),
            ("unknown variable", "kibadala kisichojulikana (unknown variable)"),
            ("unknown function", "kazi isiyojulikana (unknown function)"),
            ("type mismatch", "kutolingana kwa aina (type mismatch)"),
            ("cannot prove", "haiwezi kuthibitishwa (cannot prove)"),
            ("function ", "kazi "),
            ("invalid", "batili (invalid)"),
        ][..],
        // Collapsed above to Hindi/Marathi/Bengali/Urdu; rustc
        // requires the arms to be syntactically exhaustive.
        DiagLang::Nepali
        | DiagLang::Maithili
        | DiagLang::Konkani
        | DiagLang::Assamese
        | DiagLang::Sindhi
        | DiagLang::PunjabiShahmukhi
        | DiagLang::Pashto => {
            unreachable!("collapsed dialects shouldn't reach this match")
        }
    };
    for (en_prefix, dev_prefix) in table {
        if let Some(rest) = message.strip_prefix(en_prefix) {
            return format!("{}{}", dev_prefix, rest);
        }
    }
    message.to_string()
}

impl Diagnostic {
    pub fn new(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
            related: Vec::new(),
        }
    }

    pub fn with_related(mut self, span: Span, note: impl Into<String>) -> Self {
        self.related.push((span, note.into()));
        self
    }
}

pub fn format_diagnostics(path: &str, source: &str, diagnostics: &[Diagnostic]) -> String {
    let mut output = String::new();
    let lang = detect_diag_lang(source);
    for diagnostic in diagnostics {
        let label = localize_label("error", lang);
        let msg = localize_message(&diagnostic.message, lang);
        render_one(&mut output, path, source, diagnostic.span, &label, &msg);
        for (span, note) in &diagnostic.related {
            let nlabel = localize_label("note", lang);
            let nmsg = localize_message(note, lang);
            render_one(&mut output, path, source, *span, &nlabel, &nmsg);
        }
    }

    output
}

fn render_one(output: &mut String, path: &str, source: &str, span: Span, level: &str, message: &str) {
    let (line_number, column_number, line_start, line_end) = line_info(source, span.start);
    let line = &source[line_start..line_end];
    let span_start_byte = span.start.min(line_end).max(line_start);
    let span_end_byte = span.end.min(line_end).max(span_start_byte);
    let underline_start = char_count(&source[line_start..span_start_byte]);
    let underline_width = char_count(&source[span_start_byte..span_end_byte]).max(1);

    output.push_str(&format!(
        "{}:{}:{}: {}: {}\n",
        path, line_number, column_number, level, message
    ));
    output.push_str(line);
    output.push('\n');
    output.push_str(&" ".repeat(underline_start));
    output.push_str(&"^".repeat(underline_width));
    output.push('\n');
}

fn line_info(source: &str, offset: usize) -> (usize, usize, usize, usize) {
    let clamped = offset.min(source.len());
    let mut line_number = 1;
    let mut line_start = 0;

    for (index, byte) in source.bytes().enumerate() {
        if index >= clamped {
            break;
        }
        if byte == b'\n' {
            line_number += 1;
            line_start = index + 1;
        }
    }

    let line_end = source[line_start..]
        .find('\n')
        .map(|relative| line_start + relative)
        .unwrap_or(source.len());
    let column_number = char_count(&source[line_start..clamped]) + 1;

    (line_number, column_number, line_start, line_end)
}

fn char_count(text: &str) -> usize {
    text.chars().count()
}

/// Tracks where each source file's contents live in a concatenated multi-file
/// build buffer, so a global span offset can be mapped back to the original
/// file + local offset for accurate diagnostics.
#[derive(Clone, Debug, Default)]
pub struct FileMap {
    entries: Vec<FileEntry>,
}

#[derive(Clone, Debug)]
pub struct FileEntry {
    pub path: String,
    pub source: String,
    /// Byte offset in the concatenated buffer where this file's content starts.
    pub start: usize,
}

impl FileMap {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    pub fn push(&mut self, path: String, source: String, start: usize) {
        self.entries.push(FileEntry { path, source, start });
    }

    /// Find the file entry containing the given global offset, plus the
    /// local offset within that file's source.
    pub fn lookup(&self, global_offset: usize) -> Option<(&FileEntry, usize)> {
        // Scan in reverse so later (deeper-pushed) files win on tie at
        // file-boundary offsets; in practice ranges don't overlap.
        for entry in self.entries.iter().rev() {
            if entry.start <= global_offset
                && global_offset <= entry.start + entry.source.len()
            {
                return Some((entry, global_offset - entry.start));
            }
        }
        None
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// One past the last byte covered by any entry. New entries
    /// pushed after this can use this as their `start` so spans
    /// stay unambiguous.
    pub fn end_offset(&self) -> usize {
        self.entries
            .iter()
            .map(|e| e.start + e.source.len())
            .max()
            .unwrap_or(0)
    }

    /// Append every entry from `other` to `self`, shifting each
    /// entry's `start` so it sits past `self`'s current end (plus
    /// a one-byte gap so no boundary lookup wins twice). Returns
    /// the shift amount the caller should add to any diagnostic
    /// span produced against `other` so they remain valid in the
    /// merged map.
    ///
    /// Used by `intentc check --json` across multiple files: each
    /// `compile_path` call returns its own FileMap (starting at 0)
    /// and a list of diagnostics whose spans are relative to that
    /// map; merging shifts both into a single global frame so the
    /// JSON formatter can emit a single `{"diagnostics": [...]}`
    /// object covering the whole run.
    pub fn extend_with(&mut self, other: &FileMap) -> usize {
        let shift = self.end_offset() + if self.entries.is_empty() { 0 } else { 1 };
        for entry in &other.entries {
            self.entries.push(FileEntry {
                path: entry.path.clone(),
                source: entry.source.clone(),
                start: entry.start + shift,
            });
        }
        shift
    }
}

pub fn format_diagnostics_with_files(map: &FileMap, diagnostics: &[Diagnostic]) -> String {
    let mut output = String::new();
    for d in diagnostics {
        // Look up the source for this diagnostic to detect its
        // per-file language pragma. The pragma applies to the
        // FILE the span originates from, not globally.
        let lang = map
            .lookup(d.span.start)
            .map(|(entry, _)| detect_diag_lang(&entry.source))
            .unwrap_or(None);
        let label = localize_label("error", lang);
        let msg = localize_message(&d.message, lang);
        render_with_filemap(&mut output, map, d.span, &label, &msg);
        for (span, note) in &d.related {
            let nlabel = localize_label("note", lang);
            let nmsg = localize_message(note, lang);
            render_with_filemap(&mut output, map, *span, &nlabel, &nmsg);
        }
    }
    output
}

fn render_with_filemap(
    output: &mut String,
    map: &FileMap,
    span: Span,
    level: &str,
    message: &str,
) {
    let Some((entry, local_start)) = map.lookup(span.start) else {
        // Fallback: print without file context if mapping fails.
        output.push_str(&format!("?:?:?: {}: {}\n", level, message));
        return;
    };
    let source = &entry.source;
    let local_end = span
        .end
        .saturating_sub(entry.start)
        .min(source.len());

    let (line_number, column_number, line_start, line_end) = line_info(source, local_start);
    let line = &source[line_start..line_end];
    let span_start_byte = local_start.min(line_end).max(line_start);
    let span_end_byte = local_end.min(line_end).max(span_start_byte);
    let underline_start = char_count(&source[line_start..span_start_byte]);
    let underline_width = char_count(&source[span_start_byte..span_end_byte]).max(1);

    output.push_str(&format!(
        "{}:{}:{}: {}: {}\n",
        entry.path, line_number, column_number, level, message
    ));
    output.push_str(line);
    output.push('\n');
    output.push_str(&" ".repeat(underline_start));
    output.push_str(&"^".repeat(underline_width));
    output.push('\n');
}

pub fn format_diagnostics_json_with_files(map: &FileMap, diagnostics: &[Diagnostic]) -> String {
    let mut out = String::from("{\"diagnostics\":[");
    for (i, d) in diagnostics.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        emit_diagnostic_json_with_map(&mut out, map, d);
    }
    out.push_str("]}\n");
    out
}

fn emit_diagnostic_json_with_map(out: &mut String, map: &FileMap, d: &Diagnostic) {
    out.push_str("{\"level\":\"error\",\"message\":");
    push_json_string(out, &d.message);
    out.push_str(",\"primary\":");
    emit_span_json_with_map(out, map, d.span);
    if !d.related.is_empty() {
        out.push_str(",\"related\":[");
        for (i, (span, note)) in d.related.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str("{\"message\":");
            push_json_string(out, note);
            out.push_str(",\"span\":");
            emit_span_json_with_map(out, map, *span);
            out.push('}');
        }
        out.push(']');
    }
    out.push('}');
}

fn emit_span_json_with_map(out: &mut String, map: &FileMap, span: Span) {
    if let Some((entry, local_start)) = map.lookup(span.start) {
        let (line_start, col_start, _, _) = line_info(&entry.source, local_start);
        let local_end = span.end.saturating_sub(entry.start).min(entry.source.len());
        let (line_end, col_end, _, _) = line_info(&entry.source, local_end);
        out.push('{');
        out.push_str("\"file\":");
        push_json_string(out, &entry.path);
        out.push_str(&format!(
            ",\"line\":{},\"col\":{},\"end_line\":{},\"end_col\":{}",
            line_start, col_start, line_end, col_end
        ));
        out.push('}');
    } else {
        out.push_str("{\"file\":null}");
    }
}

/// JSON serialization of a diagnostic list. Hand-rolled to keep zero
/// dependencies. Shape:
///
/// ```json
/// {
///   "diagnostics": [
///     {
///       "level": "error",
///       "message": "value 'xs' was moved; cannot use after move",
///       "primary": { "file": "f.vani", "line": 8, "col": 18,
///                    "end_line": 8, "end_col": 20 },
///       "related": [
///         { "message": "'xs' was moved here",
///           "span": { "file": "f.vani", "line": 7, "col": 21, ... } }
///       ]
///     }
///   ]
/// }
/// ```
///
/// Always ends with a single newline so consumers can read it line by line.
pub fn format_diagnostics_json(path: &str, source: &str, diagnostics: &[Diagnostic]) -> String {
    let mut out = String::from("{\"diagnostics\":[");
    for (i, d) in diagnostics.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        emit_diagnostic_json(&mut out, path, source, d);
    }
    out.push_str("]}\n");
    out
}

fn emit_diagnostic_json(out: &mut String, path: &str, source: &str, d: &Diagnostic) {
    out.push_str("{\"level\":\"error\",\"message\":");
    push_json_string(out, &d.message);
    out.push_str(",\"primary\":");
    emit_span_json(out, path, source, d.span);
    if !d.related.is_empty() {
        out.push_str(",\"related\":[");
        for (i, (span, note)) in d.related.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str("{\"message\":");
            push_json_string(out, note);
            out.push_str(",\"span\":");
            emit_span_json(out, path, source, *span);
            out.push('}');
        }
        out.push(']');
    }
    out.push('}');
}

fn emit_span_json(out: &mut String, path: &str, source: &str, span: Span) {
    let (line_start, col_start, _, _) = line_info(source, span.start);
    let (line_end, col_end, _, _) = line_info(source, span.end);
    out.push('{');
    out.push_str("\"file\":");
    push_json_string(out, path);
    out.push_str(&format!(
        ",\"line\":{},\"col\":{},\"end_line\":{},\"end_col\":{}",
        line_start, col_start, line_end, col_end
    ));
    out.push('}');
}

fn push_json_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}
