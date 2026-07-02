#!/usr/bin/env python3
"""
vani_translate — translate a .vani source file's keywords between
                English, Sanskrit, Hindi, and Marathi.

B.1 v3 — adds SOV <-> SVO word-order reordering for verb-final
statements and Hindi for-range loops; adds optional LLM-based
translation of comments, string literals, and identifiers.

Usage:
    # Translate to Sanskrit (auto-detects source from pragma):
    python3 tools/vani_translate.py examples/language/english/basics.vani \\
        --to sanskrit -o out.vani

    # SOV word-order is reordered automatically:
    #   hindi:    n पुनरागम;            -> english: return n;
    #   english:  return n;             -> hindi:   n लौटाओ;
    #   hindi:    i के लिए 0 से 5 तक { -> english: for i from 0 to 5 {
    #   english:  for i from 0 to 5 {  -> hindi:   i के लिए 0 से 5 तक {

    # LLM translation of comments + strings (Anthropic):
    python3 tools/vani_translate.py basics.vani --to hindi \\
        --llm anthropic --llm-model claude-haiku-4-5-20251001

    # LLM translation via local Ollama:
    python3 tools/vani_translate.py basics.vani --to hindi \\
        --llm ollama --llm-model llama3.2

    # LLM translation via OpenAI:
    python3 tools/vani_translate.py basics.vani --to hindi \\
        --llm openai --llm-model gpt-4o-mini

    # Also translate identifiers (camelCase/snake_case split and translated):
    python3 tools/vani_translate.py basics.vani --to hindi \\
        --llm anthropic --translate-identifiers

    # Verify round-trip:
    python3 tools/vani_translate.py basics.vani --to hindi --verify

    # Print all keyword aliases as a markdown table:
    python3 tools/vani_translate.py --list-keywords

What this does NOT do:
  - Translate block comments /* ... */ (only line comments // ... are translated).
  - Translate multi-line string literals spanning more than one line.
  - Handle nested for-range SOV patterns (only the outermost level is reordered).
"""

import argparse
import io
import re
import sys
from pathlib import Path
from typing import Dict, List, Optional, Tuple

# Ensure UTF-8 output on Windows (default console is cp1252).
if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8")
if hasattr(sys.stderr, "reconfigure"):
    sys.stderr.reconfigure(encoding="utf-8")


# ---------------------------------------------------------------------------
# Keyword alias table.  Source of truth: src/lexer.rs
# ---------------------------------------------------------------------------

ALIASES: Dict[str, Dict[str, str]] = {
    # ── Declarations ─────────────────────────────────────────────────────────
    "Fn": {
        "english": "fn",
        "sanskrit": "कार्य",        "hindi": "फलन",          "marathi": "कार्य",
        "bengali": "ফাংশন",         "odia": "କାର୍ଯ୍ୟ",
        "tamil": "செயல்பாடு",       "telugu": "పని",          "kannada": "ಕಾರ್ಯ",
        "malayalam": "കാര്യം",
        "gujarati": "કાર્ય",        "punjabi": "ਕਾਰਜ",
        "sinhala": "කාර්යය",
        "mandarin": "函数",          "japanese": "関数",       "korean": "함수",
        "thai": "ฟังก์ชัน",         "vietnamese": "hàm",     "khmer": "មុខងារ",
        "burmese": "လုပ်ဆောင်ချက်", "lao": "ໜ້າທີ່",
        "arabic": "دالة",           "hebrew": "פונקציה",      "persian": "تابع",
        "urdu": "فنکشن",            "pashto": "فنکشن",
        "russian": "функция",
        "greek": "συνάρτηση",
        "spanish": "funcion",       "french": "fonction",     "german": "funktion",
        "portuguese": "funcao",     "italian": "funzione",    "dutch": "functie",
        "polish": "funkcja",        "turkish": "işlev",       "swedish": "funktion",
        "norwegian": "funksjon",    "danish": "funktion",     "hungarian": "függvény",
        "czech": "funkce",          "slovak": "funkcia",      "finnish": "funktio",
        "romanian": "funcție",      "catalan": "funcio",
        "armenian": "ֆունկցիա",     "georgian": "ფუნქცია",
        "swahili": "kazi",          "yoruba": "iṣẹ́",         "hausa": "aiki",
        "amharic": "ተግባር",
        "tibetan": "ལས་ཀ",
        "cherokee": "ᏗᎦᏬᏂᎯᏍᏗ",   "mongolian": "ᠴᠠᠭ",
        "malay": "fungsi",          "indonesian": "fungsi",   "filipino": "gawain",
    },
    "Let": {
        "english": "let",
        "sanskrit": "माना",          "hindi": "माना",          "marathi": "मान",
        "bengali": "মান",            "odia": "ମନେକର",
        "tamil": "கொள்",            "telugu": "అనుకో",        "kannada": "ಊಹಿಸಿ",
        "malayalam": "കരുതുക",
        "gujarati": "માનો",          "punjabi": "ਮੰਨੋ",
        "sinhala": "අනුමානය",
        "mandarin": "让",            "japanese": "代入",        "korean": "정의",
        "thai": "ให้",               "vietnamese": "đặt",     "khmer": "អោយ",
        "burmese": "ထား",            "lao": "ໃຫ້",
        "arabic": "ليكن",            "hebrew": "יהי",          "persian": "فرض",
        "urdu": "مانیں",             "pashto": "ووایه",
        "russian": "пусть",
        "greek": "έστω",
        "spanish": "sea",            "french": "soit",         "german": "sei",
        "portuguese": "seja",        "italian": "sia",         "dutch": "laat",
        "polish": "niech",           "turkish": "olsun",       "swedish": "låt",
        "norwegian": "la",           "danish": "lad",          "hungarian": "legyen",
        "czech": "nechť",            "slovak": "nech",         "finnish": "olkoon",
        "romanian": "fie",           "catalan": "sigui",
        "armenian": "թող",           "georgian": "მიეცი",
        "swahili": "acha",           "yoruba": "jẹ́",          "hausa": "bari",
        "amharic": "ይሁን",
        "tibetan": "ཡོད་པར་ཤོག",
        "cherokee": "ᎠᏁᎳ",
        "malay": "biarkan",          "indonesian": "biarkan",  "filipino": "hayaan",
    },
    "Struct": {
        "english": "struct",
        "sanskrit": "संरचना",        "hindi": "संरचना",        "marathi": "संरचना",
        "bengali": "গঠন",            "odia": "ଗଠନ",
        "tamil": "கட்டமைப்பு",       "telugu": "నిర్మాణం",     "kannada": "ರಚನೆ",
        "malayalam": "ഘടന",
        "gujarati": "રચના",          "punjabi": "ਰਚਨਾ",
        "sinhala": "ව්‍යුහය",
        "mandarin": "结构",           "japanese": "構造体",      "korean": "구조체",
        "thai": "โครงสร้าง",         "vietnamese": "cấu_trúc", "khmer": "រចនាសម្ព័ន្ធ",
        "burmese": "ဖွဲ့စည်းပုံ",    "lao": "ໂຄງສ້າງ",
        "arabic": "بنية",            "hebrew": "מבנה",          "persian": "ساختار",
        "urdu": "ساخت",              "pashto": "جوړښت",
        "russian": "структура",
        "greek": "δομή",
        "spanish": "estructura",     "french": "structure",    "german": "struktur",
        "portuguese": "estrutura",   "italian": "struttura",   "dutch": "structuur",
        "polish": "struktura",       "turkish": "yapı",        "swedish": "struktur",
        "norwegian": "struktur",     "danish": "struktur",     "hungarian": "szerkezet",
        "czech": "struktura",        "slovak": "štruktúra",    "finnish": "rakenne",
        "romanian": "structură",     "catalan": "estructura",
        "armenian": "կառուցվածք",    "georgian": "სტრუქტურა",
        "swahili": "muundo",         "hausa": "tsari",
        "amharic": "መዋቅር",
        "cherokee": "ᎠᏙᏢᏍᎩ",
        "malay": "struktur",         "indonesian": "struktur", "filipino": "istraktura",
    },
    "Enum": {
        "english": "enum",
        "sanskrit": "विकल्प",        "hindi": "गणन",           "marathi": "गणन",
        "bengali": "গণনা",           "odia": "ଗଣନା",
        "tamil": "எண்ணுப்பெயர்",    "telugu": "గణన",          "kannada": "ಎಣಿಕೆ",
        "malayalam": "എണ്ണൽ",
        "gujarati": "ગણના",          "punjabi": "ਗਣਨਾ",
        "sinhala": "ගණනය",
        "mandarin": "枚举",           "japanese": "列挙",        "korean": "열거",
        "thai": "การแจงนับ",         "vietnamese": "liệt_kê",  "khmer": "ការរាប់បញ្ចូល",
        "burmese": "စာရင်း",         "lao": "ການນັບ",
        "arabic": "تعداد",           "hebrew": "ספירה",         "persian": "شمارش",
        "urdu": "شمار",              "pashto": "شمېرل",
        "russian": "перечисление",
        "greek": "απαρίθμηση",
        "spanish": "enumeracion",    "french": "énumération",  "german": "aufzählung",
        "portuguese": "enumeracao",  "italian": "enumerazione","dutch": "opsomming",
        "polish": "wyliczenie",      "turkish": "sıralama",    "swedish": "uppräkning",
        "norwegian": "oppregning",   "danish": "optælling",    "hungarian": "felsorolás",
        "czech": "výčet",            "slovak": "vypocet",      "finnish": "luettelointi",
        "romanian": "enumerare",     "catalan": "enumeracio",
        "armenian": "թվարկում",      "georgian": "ჩამოთვლა",
        "swahili": "orodha",         "hausa": "lissafi",
        "amharic": "ቆጠራ",
        "tibetan": "རྩིས",
        "malay": "penghitungan",     "indonesian": "enumerasi","filipino": "pagbilang",
    },
    "Const": {
        "english": "const",
        "sanskrit": "स्थिर",         "hindi": "स्थिर",         "marathi": "स्थिर",
        "bengali": "স্থির",          "odia": "ସ୍ଥିର",
        "tamil": "மாறா",             "telugu": "స్థిరం",       "kannada": "ಸ್ಥಿರ",
        "malayalam": "സ്ഥിരം",
        "gujarati": "સ્થિર",         "punjabi": "ਸਥਿਰ",
        "sinhala": "ස්ථිර",
        "mandarin": "常量",           "japanese": "定数",        "korean": "상수",
        "thai": "คงที่",             "vietnamese": "hằng",    "khmer": "ថេរ",
        "burmese": "ပုံသေ",          "lao": "ຄົງທີ່",
        "arabic": "قيمة_ثابتة",      "hebrew": "קבוע",          "persian": "ثابت",
        "urdu": "ثابت",              "pashto": "ثابت",
        "russian": "постоянная",
        "greek": "σταθερά",
        "spanish": "constante",      "french": "constante",    "german": "konstante",
        "portuguese": "constante",   "italian": "costante",    "dutch": "constante",
        "polish": "stała",           "turkish": "sabit",       "swedish": "konstant",
        "norwegian": "konstant",     "danish": "konstant",     "hungarian": "állandó",
        "czech": "konstanta",        "slovak": "konstanta",    "finnish": "vakio",
        "romanian": "constantă",     "catalan": "constant",
        "armenian": "հաստատուն",     "georgian": "მუდმივი",
        "swahili": "thabiti",        "hausa": "tabbas",
        "amharic": "ቋሚ",
        "tibetan": "རྟག",
        "malay": "pemalar",          "indonesian": "tetap",    "filipino": "pirme",
    },
    "Type": {
        "english": "type",
        "sanskrit": "प्रकार",        "hindi": "प्रकार",         "marathi": "प्रकार",
        "bengali": "প্রকার",         "odia": "ପ୍ରକାର",
        "tamil": "வகை",              "telugu": "రకం",          "kannada": "ಪ್ರಕಾರ",
        "malayalam": "തരം",
        "gujarati": "પ્રકાર",        "punjabi": "ਕਿਸਮ",
        "sinhala": "වර්ගය",
        "mandarin": "类型",           "japanese": "型",          "korean": "타입",
        "thai": "ชนิด",              "vietnamese": "kiểu",    "khmer": "ប្រភេទ",
        "burmese": "အမျိုးအစား",     "lao": "ປະເພດ",
        "arabic": "نوع",             "hebrew": "סוג",           "persian": "نوع",
        "urdu": "قسم",               "pashto": "ډول",
        "russian": "тип",
        "greek": "τύπος",
        "spanish": "tipo",           "french": "type",          "german": "typ",
        "portuguese": "tipo",        "italian": "tipo",         "dutch": "type",
        "polish": "typ",             "turkish": "tür",          "swedish": "typ",
        "norwegian": "type",         "danish": "type",          "hungarian": "típus",
        "czech": "typ",              "slovak": "typ",           "finnish": "tyyppi",
        "romanian": "tip",           "catalan": "tipus",
        "armenian": "տեսակ",         "georgian": "ტიპი",
        "swahili": "aina",           "hausa": "irin",
        "amharic": "አይነት",
        "tibetan": "རིགས",
        "malay": "jenis",            "indonesian": "tipe",     "filipino": "uri",
    },
    "Extern": {
        "english": "extern",
        "sanskrit": "बाह्य",         "hindi": "बाह्य",          "marathi": "बाह्य",
        "bengali": "বাহ্যিক",        "odia": "ବାହ୍ୟ",
        "mandarin": "外部",           "japanese": "外部",         "korean": "외부",
        "thai": "ภายนอก",            "vietnamese": "bên_ngoài",
        "arabic": "خارجي",           "hebrew": "חיצוני",         "persian": "خارجی",
        "urdu": "بیرونی",
        "russian": "внешний",
        "greek": "εξωτερικό",
        "spanish": "externo",        "french": "externe",       "german": "extern",
        "portuguese": "externo",     "italian": "esterno",      "dutch": "extern",
        "polish": "zewnętrzny",      "turkish": "dış",          "swedish": "extern",
        "norwegian": "ekstern",      "danish": "extern",        "hungarian": "külső",
        "czech": "vnější",           "slovak": "vonkajší",      "finnish": "ulkoinen",
        "romanian": "extern",        "catalan": "extern",
        "armenian": "արտաքին",       "georgian": "გარე",
        "amharic": "ውጫዊ",
        "tibetan": "ཕྱི",
        "malay": "luaran",           "indonesian": "eksternal",
    },
    "Intent": {
        "english": "intent",
        "sanskrit": "उद्देश्य",      "hindi": "उद्देश्य",       "marathi": "उद्देश्य",
        "bengali": "উদ্দেশ্য",       "odia": "ଉଦ୍ଦେଶ୍ୟ",
        "tamil": "நோக்கம்",          "telugu": "ఉద్దేశం",      "kannada": "ಉದ್ದೇಶ",
        "malayalam": "ഉദ്ദേശ്യം",
        "gujarati": "ઉદ્દેશ",        "punjabi": "ਉਦੇਸ਼",
        "sinhala": "අරමුණ",
        "mandarin": "目的",           "japanese": "目的",         "korean": "목적",
        "thai": "จุดประสงค์",         "vietnamese": "mục_đích",  "khmer": "គោលបំណង",
        "burmese": "ရည်ရွယ်ချက်",    "lao": "ຈຸດປະສົງ",
        "arabic": "هدف",             "hebrew": "מטרה",           "persian": "هدف",
        "urdu": "مقصد",
        "russian": "цель",
        "greek": "σκοπός",
        "spanish": "intencion",      "french": "but",           "german": "absicht",
        "portuguese": "intencao",    "italian": "scopo",        "dutch": "doel",
        "polish": "cel",             "turkish": "amaç",         "swedish": "syfte",
        "norwegian": "hensikt",      "danish": "formål",        "hungarian": "cél",
        "czech": "záměr",            "slovak": "účel",          "finnish": "tarkoitus",
        "romanian": "scop",          "catalan": "proposit",
        "armenian": "նպատակ",        "georgian": "მიზანი",
        "swahili": "kusudi",         "yoruba": "ìpinnu",        "hausa": "nufin",
        "amharic": "ዓላማ",
        "tibetan": "དམིགས་ཡུལ",
        "mongolian": "ᠵᠣᠷᠢᠯᠭ᠎ᠠ",
        "malay": "niat",             "indonesian": "tujuan",    "filipino": "layunin",
    },
    "Invariant": {
        "english": "invariant",
        "sanskrit": "अपरिवर्तनीय",   "hindi": "अपरिवर्तनीय",   "marathi": "अपरिवर्तनीय",
        "bengali": "অপরিবর্তনীয়",   "odia": "ଅଚଳ",
        "tamil": "மாறிலா",           "telugu": "మారని",         "kannada": "ಅಚಲ",
        "malayalam": "അചലം",
        "gujarati": "અચળ",           "punjabi": "ਅਟੱਲ",
        "sinhala": "නිශ්චල",
        "mandarin": "不变量",          "japanese": "不変",         "korean": "불변",
        "thai": "ไม่เปลี่ยน",         "vietnamese": "bất_biến",
        "arabic": "مستقر",           "hebrew": "בלתי_משתנה",
        "russian": "инвариант",
        "greek": "αμετάβλητο",
        "spanish": "invariante",     "french": "invariant",     "german": "unveraenderlich",
        "portuguese": "invariante",  "italian": "invariante",   "dutch": "invariant",
        "polish": "niezmienny",      "turkish": "değişmez",     "swedish": "oföränderlig",
        "norwegian": "uforanderlig", "danish": "uforanderlig",  "hungarian": "változatlan",
        "czech": "neměnný",          "slovak": "nemenný",       "finnish": "muuttumaton",
        "romanian": "invariant",     "catalan": "invariant",
        "armenian": "անփոփոխ",       "georgian": "უცვლელი",
        "amharic": "የማይለወጥ",
        "malay": "invarian",         "indonesian": "invarian",
    },

    # ── Visibility / modules / imports ───────────────────────────────────────
    "Pub": {
        "english": "pub",
        "sanskrit": "सार्वजनिक",     "hindi": "सार्वजनिक",      "marathi": "सार्वजनिक",
        "bengali": "সর্বজনীন",       "odia": "ସର୍ବସାଧାରଣ",
        "tamil": "பொது",             "telugu": "ప్రజా",         "kannada": "ಸಾರ್ವಜನಿಕ",
        "malayalam": "പൊതു",
        "gujarati": "જાહેર",         "punjabi": "ਜਨਤਕ",
        "sinhala": "පොදු",
        "mandarin": "公开",           "japanese": "公開",         "korean": "공개",
        "thai": "สาธารณะ",           "vietnamese": "công_khai", "khmer": "សាធារណៈ",
        "burmese": "အများပြည်သူ",    "lao": "ສາທາລະນະ",
        "arabic": "عام",             "hebrew": "ציבורי",         "persian": "عمومی",
        "urdu": "عوامی",             "pashto": "عمومي",
        "russian": "публичный",
        "greek": "δημόσιο",
        "spanish": "publico",        "french": "public",        "german": "öffentlich",
        "portuguese": "publico",     "italian": "pubblico",     "dutch": "openbaar",
        "polish": "publiczny",       "turkish": "genel",        "swedish": "offentlig",
        "norwegian": "offentlig",    "danish": "offentlig",     "hungarian": "nyilvános",
        "czech": "veřejný",          "slovak": "verejný",       "finnish": "julkinen",
        "romanian": "public",        "catalan": "public",
        "armenian": "հանրային",      "georgian": "საჯარო",
        "swahili": "umma",           "hausa": "gama_gari",
        "amharic": "ሕዝባዊ",
        "tibetan": "སྤྱི",
        "malay": "awam",             "indonesian": "publik",    "filipino": "pampubliko",
    },
    "Module": {
        "english": "module",
        "sanskrit": "खण्ड",          "hindi": "मॉड्यूल",        "marathi": "मॉड्यूल",
        "bengali": "খণ্ড",           "odia": "ଖଣ୍ଡ",
        "tamil": "தொகுதி",           "telugu": "మాడ్యూల్",      "kannada": "ಖಂಡ",
        "malayalam": "ഖണ്ഡം",
        "gujarati": "ખંડ",           "punjabi": "ਖੰਡ",
        "sinhala": "මොඩියුලය",
        "mandarin": "模块",           "japanese": "モジュール",    "korean": "모듈",
        "thai": "โมดูล",             "vietnamese": "mô_đun",    "khmer": "ម៉ូឌុល",
        "burmese": "ယူနစ်",          "lao": "ໂມດູນ",
        "arabic": "وحدة",            "hebrew": "מודול",          "persian": "بخش",
        "urdu": "ماڈیول",            "pashto": "برخه",
        "russian": "модуль",
        "greek": "ενότητα",
        "spanish": "modulo",         "french": "module",        "german": "modul",
        "portuguese": "modulo",      "italian": "modulo",       "dutch": "module",
        "polish": "moduł",           "turkish": "modül",        "swedish": "modul",
        "norwegian": "modul",        "danish": "modul",         "hungarian": "modul",
        "czech": "modul",            "slovak": "modul",         "finnish": "moduuli",
        "romanian": "modul",         "catalan": "modul",
        "armenian": "մոդուլ",        "georgian": "მოდული",
        "swahili": "moduli",         "hausa": "sashe",
        "amharic": "ሞዱል",
        "tibetan": "ཚན",
        "malay": "modul",            "indonesian": "modul",     "filipino": "modyul",
    },
    "Use": {
        "english": "use",
        "sanskrit": "उपयोग",         "hindi": "उपयोग",          "marathi": "उपयोग",
        "bengali": "ব্যবহার",         "odia": "ବ୍ୟବହାର",
        "tamil": "பயன்படுத்து",      "telugu": "ఉపయోగించు",     "kannada": "ಬಳಸಿ",
        "malayalam": "ഉപയോഗിക്കുക",
        "gujarati": "વાપરો",          "punjabi": "ਵਰਤੋ",
        "sinhala": "භාවිතා",
        "mandarin": "使用",           "japanese": "使用",         "korean": "사용",
        "thai": "ใช้",               "vietnamese": "sử_dụng",  "khmer": "ប្រើ",
        "burmese": "သုံး",            "lao": "ໃຊ້",
        "arabic": "استخدم",          "hebrew": "השתמש",          "persian": "استفاده",
        "urdu": "استعمال",           "pashto": "وکاروه",
        "russian": "использовать",
        "greek": "χρήση",
        "spanish": "usar",           "french": "utiliser",      "german": "verwenden",
        "portuguese": "usar",        "italian": "usare",        "dutch": "gebruik",
        "polish": "użyj",            "turkish": "kullan",       "swedish": "använd",
        "norwegian": "bruk",         "danish": "brug",          "hungarian": "használd",
        "czech": "použij",           "slovak": "použi",         "finnish": "käytä",
        "romanian": "folosește",     "catalan": "usa",
        "armenian": "օգտագործել",    "georgian": "გამოყენება",
        "swahili": "tumia",          "hausa": "amfani",
        "amharic": "ተጠቀም",
        "tibetan": "བཀོལ",
        "cherokee": "ᎬᏙᏗ",          "mongolian": "ᠬᠡᠷᠡᠭᠯᠡ",
        "malay": "guna",             "indonesian": "pakai",     "filipino": "gamitin",
    },
    "As": {
        "english": "as",
        "sanskrit": "यथा",           "hindi": "यथा",            "marathi": "यथा",
        "bengali": "হিসাবে",          "odia": "ଭାବେ",
        "tamil": "ஆக",               "telugu": "గా",           "kannada": "ಆಗಿ",
        "malayalam": "ആയി",
        "gujarati": "તરીકે",          "punjabi": "ਵਜੋਂ",
        "sinhala": "ලෙස",
        "mandarin": "作为",           "japanese": "として",       "korean": "로서",
        "thai": "เป็น",              "vietnamese": "như",       "khmer": "ជា",
        "burmese": "အဖြစ်",          "lao": "ເປັນ",
        "arabic": "بوصفه",           "persian": "بعنوان",
        "urdu": "بطور",
        "russian": "как",
        "greek": "ως",
        "spanish": "como",           "french": "comme",         "german": "als",
        "portuguese": "como",        "italian": "come",         "dutch": "als",
        "polish": "jako",            "turkish": "olarak",       "swedish": "som",
        "norwegian": "som",          "danish": "som",           "hungarian": "mint",
        "czech": "jako",             "slovak": "ako",           "finnish": "kuten",
        "romanian": "ca",            "catalan": "com",
        "armenian": "որպես",         "georgian": "როგორც",
        "swahili": "kama",           "hausa": "kamar",
        "tibetan": "དུ",
        "malay": "sebagai",          "indonesian": "sebagai",   "filipino": "bilang",
    },

    # ── Control flow ─────────────────────────────────────────────────────────
    "Return": {
        "english": "return",
        "sanskrit": "पुनरागम",       "hindi": "लौटाओ",          "marathi": "परत",
        "bengali": "ফেরত",           "odia": "ଫେରନ୍ତୁ",
        "tamil": "திருப்பு",         "telugu": "తిరిగి",        "kannada": "ಹಿಂದಿರುಗಿ",
        "malayalam": "തിരികെ",
        "gujarati": "પાછા",          "punjabi": "ਮੁੜੋ",
        "sinhala": "ආපසු",
        "mandarin": "返回",           "japanese": "戻る",         "korean": "반환",
        "thai": "คืน",               "vietnamese": "trả_về",   "khmer": "ត្រលប់",
        "burmese": "ပြန်",           "lao": "ກັບຄືນ",
        "arabic": "أرجع",            "hebrew": "החזר",           "persian": "بازگشت",
        "urdu": "واپس",              "pashto": "بېرته",
        "russian": "вернуть",
        "greek": "επιστροφή",
        "spanish": "regresar",       "french": "retourner",     "german": "zurück",
        "portuguese": "retornar",    "italian": "ritornare",    "dutch": "terug",
        "polish": "zwróć",           "turkish": "dön",          "swedish": "återvänd",
        "norwegian": "returner",     "danish": "returner",      "hungarian": "visszatér",
        "czech": "vrať",             "slovak": "vráť",          "finnish": "palaa",
        "romanian": "întoarce",      "catalan": "retorna",
        "armenian": "վերադարձ",      "georgian": "დაბრუნება",
        "swahili": "rudi",           "hausa": "koma",
        "amharic": "መልስ",
        "tibetan": "ལོག",
        "cherokee": "ᏗᎬᏎᏗ",         "mongolian": "ᠪᠤᠴᠠ",
        "malay": "kembali",          "indonesian": "kembali",   "filipino": "ibalik",
    },
    "If": {
        "english": "if",
        "sanskrit": "यदि",           "hindi": "अगर",            "marathi": "जर",
        "bengali": "যদি",            "odia": "ଯଦି",
        "tamil": "என்றால்",          "telugu": "అయితే",         "kannada": "ಆದರೆ",
        "malayalam": "എങ്കിൽ",
        "gujarati": "જો",            "punjabi": "ਜੇ",
        "sinhala": "නම්",
        "mandarin": "如果",           "japanese": "もし",         "korean": "만약",
        "thai": "ถ้า",               "vietnamese": "nếu",      "khmer": "បើ",
        "burmese": "ဆိုလျှင်",       "lao": "ຖ້າ",
        "arabic": "إذا",             "hebrew": "אם",             "persian": "اگر",
        "urdu": "اگر",               "pashto": "که",
        "russian": "если",
        "greek": "αν",
        "spanish": "si",             "french": "si",            "german": "wenn",
        "portuguese": "se",          "italian": "se",           "dutch": "indien",
        "polish": "jeśli",           "turkish": "eğer",         "swedish": "om",
        "norwegian": "hvis",         "danish": "hvis",          "hungarian": "ha",
        "czech": "pokud",            "slovak": "ak",            "finnish": "jos",
        "romanian": "dacă",          "catalan": "si",
        "armenian": "եթե",           "georgian": "თუ",
        "swahili": "ikiwa",          "yoruba": "bí",            "hausa": "idan",
        "amharic": "ከ",
        "tibetan": "གལ་ཏེ",
        "cherokee": "ᎢᏳᏃ",          "mongolian": "ᠬᠡᠷᠪᠡ",
        "malay": "jika",             "indonesian": "jika",      "filipino": "kung",
    },
    "Else": {
        "english": "else",
        "sanskrit": "अन्यथा",        "hindi": "वरना",           "marathi": "नाहीतर",
        "bengali": "নাহলে",          "odia": "ଅନ୍ୟଥା",
        "tamil": "இல்லாவிட்டால்",    "telugu": "లేకపోతే",       "kannada": "ಇಲ್ಲದಿದ್ದರೆ",
        "malayalam": "അല്ലെങ്കിൽ",
        "gujarati": "નહીંતર",        "punjabi": "ਵਰਨਾ",
        "sinhala": "නැතිනම්",
        "mandarin": "否则",           "japanese": "そうでなければ","korean": "아니면",
        "thai": "ไม่เช่นนั้น",        "vietnamese": "ngược_lại", "khmer": "ផ្សេង",
        "burmese": "မဟုတ်ပါက",       "lao": "ບໍ່ດັ່ງນັ້ນ",
        "arabic": "وإلا",            "hebrew": "אחרת",           "persian": "وگرنه",
        "urdu": "ورنہ",
        "russian": "иначе",
        "greek": "αλλιώς",
        "spanish": "sino",           "french": "sinon",         "german": "sonst",
        "portuguese": "senao",       "italian": "altrimenti",   "dutch": "anders",
        "polish": "inaczej",         "turkish": "yoksa",        "swedish": "annars",
        "norwegian": "ellers",       "danish": "ellers",        "hungarian": "különben",
        "czech": "jinak",            "slovak": "inak",          "finnish": "muuten",
        "romanian": "altfel",        "catalan": "altrament",
        "armenian": "այլապես",       "georgian": "სხვა",
        "swahili": "vinginevyo",     "hausa": "ko_kuwa",
        "amharic": "ካልሆነ",
        "tibetan": "གཞན",
        "cherokee": "ᎪᎯ",           "mongolian": "ᠡᠰᠡᠪᠡᠯ",
        "malay": "selainnya",        "indonesian": "selainnya", "filipino": "kundi",
    },
    "While": {
        "english": "while",
        "sanskrit": "यावत्",          "hindi": "जबतक",           "marathi": "जोपर्यंत",
        "bengali": "যতক্ষণ",         "odia": "ଯେତେବେଳେ",
        "tamil": "வரை",              "telugu": "వరకు",          "kannada": "ತನಕ",
        "malayalam": "വരെ",
        "gujarati": "જ્યારે",        "punjabi": "ਜਦੋਂ",
        "sinhala": "තෙක්",
        "mandarin": "当",            "japanese": "間",           "korean": "동안",
        "thai": "ขณะที่",            "vietnamese": "trong_khi", "khmer": "ខណៈ",
        "burmese": "နေစဉ်",          "lao": "ໃນຂະນະທີ່",
        "arabic": "بينما",           "hebrew": "כאשר",           "persian": "تا",
        "russian": "пока",
        "greek": "όσο",
        "spanish": "mientras",       "french": "tandis",        "german": "während",
        "portuguese": "enquanto",    "italian": "mentre",       "dutch": "zolang",
        "polish": "dopóki",          "turkish": "iken",         "swedish": "medan",
        "norwegian": "mens",         "danish": "mens",          "hungarian": "amíg",
        "czech": "dokud",            "slovak": "pokiaľ",        "finnish": "kun",
        "romanian": "cât_timp",      "catalan": "mentre",
        "armenian": "քանի",          "georgian": "სანამ",
        "swahili": "wakati",         "hausa": "yayin",
        "amharic": "ሲ",
        "tibetan": "བར",
        "mongolian": "ᠶᠠᠭ᠎ᠠ",
        "malay": "selama",           "indonesian": "selama",    "filipino": "habang",
    },
    "For": {
        "english": "for",
        "sanskrit": "प्रति",          "hindi": "के लिए",         "marathi": "साठी",
        "bengali": "প্রতি",           "odia": "ପ୍ରତି",
        "tamil": "ஒவ்வொரு",         "telugu": "ప్రతి",         "kannada": "ಪ್ರತಿ",
        "malayalam": "ഓരോ",
        "gujarati": "પ્રતિ",         "punjabi": "ਹਰ",
        "sinhala": "සෑම",
        "mandarin": "对于",           "japanese": "対象",         "korean": "각각",
        "thai": "สำหรับ",            "vietnamese": "với_mỗi",  "khmer": "សម្រាប់",
        "burmese": "အတွက်",          "lao": "ສຳລັບ",
        "arabic": "لكل",             "hebrew": "עבור",           "persian": "هر",
        "urdu": "ہر",                "pashto": "هر",
        "russian": "для",
        "greek": "για",
        "spanish": "para",           "french": "pour",          "german": "für",
        "portuguese": "para",        "italian": "per",          "dutch": "voor",
        "polish": "dla",             "turkish": "için",         "swedish": "för",
        "norwegian": "for",          "danish": "for",           "hungarian": "minden",
        "czech": "pro",              "slovak": "pre",           "finnish": "jokaiselle",
        "romanian": "pentru",        "catalan": "per",
        "armenian": "ամեն",          "georgian": "თითოეული",
        "swahili": "kwa",            "yoruba": "fún",           "hausa": "ga",
        "amharic": "ለ",
        "tibetan": "ལ",
        "cherokee": "ᏌᏊ",           "mongolian": "ᠬᠠᠷᠠᠭᠠᠯᠵᠠᠯ",
        "malay": "untuk",            "indonesian": "untuk",     "filipino": "para",
    },
    "In": {
        "english": "in",
        "sanskrit": "में",           "hindi": "में",             "marathi": "में",
        "bengali": "মধ্যে",          "odia": "ରେ",
        "tamil": "உள்",              "telugu": "లో",            "kannada": "ರಲ್ಲಿ",
        "malayalam": "ഇൽ",
        "gujarati": "માં",           "punjabi": "ਵਿੱਚ",
        "sinhala": "තුළ",
        "mandarin": "in",            "japanese": "に",           "korean": "안에",
        "thai": "ใน",                "khmer": "ក្នុង",
        "burmese": "ထဲမှာ",          "lao": "ໃນ",
        "arabic": "في",              "hebrew": "בתוך",           "persian": "در",
        "urdu": "میں",
        "russian": "в",
        "greek": "σε",
        "spanish": "en",             "french": "dans",          "german": "in",
        "portuguese": "em",          "dutch": "in",
        "polish": "w",               "turkish": "içinde",       "swedish": "i",
        "finnish": "sisalla",
        "armenian": "մեջ",           "georgian": "ში",
        "swahili": "ndani",          "hausa": "cikin",
        "amharic": "ውስጥ",
        "tibetan": "ནང",
        "cherokee": "ᎭᏫᎾ",
        "malay": "dalam",            "indonesian": "dalam",     "filipino": "sa",
    },
    "From": {
        "english": "from",
        "sanskrit": "से",            "hindi": "से",              "marathi": "से",
        "bengali": "থেকে",           "odia": "ରୁ",
        "tamil": "இருந்து",          "telugu": "నుండి",          "kannada": "ಇಂದ",
        "malayalam": "നിന്ന്",
        "gujarati": "થી",            "punjabi": "ਤੋਂ",
        "sinhala": "සිට",
        "mandarin": "从",            "japanese": "から",          "korean": "에서",
        "thai": "จาก",               "vietnamese": "từ",        "khmer": "ពី",
        "burmese": "မှ",             "lao": "ຈາກ",
        "arabic": "من",              "hebrew": "מתוך",           "persian": "از",
        "russian": "от",
        "greek": "από",
        "spanish": "desde",          "french": "depuis",        "german": "von",
        "portuguese": "desde",       "italian": "da",           "dutch": "van",
        "polish": "od",              "turkish": "den",          "swedish": "från",
        "norwegian": "fra",          "danish": "fra",
        "czech": "od",               "slovak": "od",            "finnish": "lähtien",
        "romanian": "de_la",         "catalan": "des",
        "armenian": "ից",            "georgian": "დან",
        "swahili": "kutoka",         "yoruba": "láti",          "hausa": "daga",
        "amharic": "ከ",
        "tibetan": "ནས",
        "mongolian": "ᠠᠴᠠ",
        "malay": "dari",             "indonesian": "dari",      "filipino": "mula",
    },
    "To": {
        "english": "to",
        "sanskrit": "तक",            "hindi": "तक",              "marathi": "तक",
        "bengali": "পর্যন্ত",        "odia": "ପର୍ଯ୍ୟନ୍ତ",
        "tamil": "வரைக்கும்",        "telugu": "వరకూ",           "kannada": "ಗೆ",
        "malayalam": "വരെക്കും",
        "gujarati": "સુધી",          "punjabi": "ਤੱਕ",
        "sinhala": "දක්වා",
        "mandarin": "到",            "japanese": "まで",          "korean": "까지",
        "thai": "ถึง",               "vietnamese": "đến",       "khmer": "ដល់",
        "burmese": "သို့",           "lao": "ເຖິງ",
        "arabic": "إلى",             "hebrew": "עד",             "persian": "تا",
        "urdu": "تک",
        "russian": "до",
        "greek": "μέχρι",
        "spanish": "hasta",          "french": "vers",          "german": "bis",
        "portuguese": "ate",         "italian": "fino",         "dutch": "tot",
        "polish": "do",              "turkish": "kadar",        "swedish": "till",
        "norwegian": "til",          "danish": "til",
        "czech": "do",               "slovak": "do",            "finnish": "asti",
        "romanian": "până",          "catalan": "fins",
        "armenian": "մինչև",         "georgian": "მდე",
        "swahili": "hadi",           "yoruba": "dé",            "hausa": "zuwa",
        "amharic": "ወደ",
        "tibetan": "བར་དུ",
        "mongolian": "ᠬᠦᠷᠲᠡᠯᠡ",
        "malay": "hingga",           "indonesian": "sampai",    "filipino": "hanggang",
    },
    "Break": {
        "english": "break",
        "sanskrit": "विराम",         "hindi": "रुको",            "marathi": "थांब",
        "bengali": "বিরাম",          "odia": "ବନ୍ଦ",
        "tamil": "நிறுத்து",         "telugu": "ఆపు",           "kannada": "ನಿಲ್ಲಿ",
        "malayalam": "നിർത്തുക",
        "gujarati": "વિરામ",         "punjabi": "ਵਿਰਾਮ",
        "sinhala": "නවත්වන්න",
        "mandarin": "中断",           "japanese": "中断",         "korean": "중단",
        "thai": "หยุด",              "vietnamese": "ngắt",     "khmer": "បំបាក់",
        "burmese": "ရပ်",            "lao": "ຢຸດ",
        "arabic": "كسر",             "hebrew": "שבור",           "persian": "بشکن",
        "urdu": "بند",
        "russian": "прервать",
        "greek": "διακοπή",
        "spanish": "romper",         "french": "interrompre",   "german": "brechen",
        "portuguese": "parar",       "italian": "rompere",      "dutch": "stop",
        "polish": "przerwij",        "turkish": "kır",          "swedish": "bryt",
        "norwegian": "bryt",         "danish": "bryd",          "hungarian": "törj",
        "czech": "přeruš",           "slovak": "preruš",        "finnish": "katkaise",
        "romanian": "rupe",          "catalan": "trenca",
        "armenian": "ընդհատել",      "georgian": "შეჩერება",
        "swahili": "vunja",          "hausa": "dakatar",
        "amharic": "ስብር",
        "tibetan": "འགོག",
        "cherokee": "ᎤᎵᏍᎩᏗ",      "mongolian": "ᠵᠣᠭᠰᠣ",
        "malay": "berhenti",         "indonesian": "berhenti",  "filipino": "tumigil",
    },
    "Continue": {
        "english": "continue",
        "sanskrit": "अग्रे",         "hindi": "आगे",             "marathi": "पुढे",
        "bengali": "এগিয়ে",          "odia": "ଜାରି",
        "tamil": "தொடர்",            "telugu": "కొనసాగించు",     "kannada": "ಮುಂದುವರಿಸಿ",
        "malayalam": "തുടരുക",
        "gujarati": "ચાલુ",          "punjabi": "ਜਾਰੀ",
        "sinhala": "ඉදිරියට",
        "mandarin": "继续",           "japanese": "続行",         "korean": "계속",
        "thai": "ดำเนินต่อ",          "vietnamese": "tiếp_tục",  "khmer": "បន្ត",
        "burmese": "ဆက်လုပ်",        "lao": "ສືບຕໍ່",
        "arabic": "استمر",           "hebrew": "המשך",           "persian": "ادامه",
        "urdu": "جاری",
        "russian": "продолжить",
        "greek": "συνέχεια",
        "spanish": "continuar",      "french": "continuer",     "german": "weiter",
        "portuguese": "continuar",   "italian": "continuare",   "dutch": "verder",
        "polish": "kontynuuj",       "turkish": "devam",        "swedish": "fortsätt",
        "norwegian": "fortsett",     "danish": "fortsæt",       "hungarian": "folytasd",
        "czech": "pokračuj",         "slovak": "pokračuj",      "finnish": "jatka",
        "romanian": "continuă",      "catalan": "continua",
        "armenian": "շարունակել",    "georgian": "გაგრძელება",
        "swahili": "endelea",        "hausa": "ci_gaba",
        "amharic": "ቀጥል",
        "tibetan": "མུ་མཐུད",
        "cherokee": "ᏗᎧᎵᏍᏗ",      "mongolian": "ᠦᠷᠭᠦᠯᠵᠢᠯᠡ",
        "malay": "teruskan",         "indonesian": "lanjutkan", "filipino": "magpatuloy",
    },
    "Then": {
        "english": "then",
        "sanskrit": "तदा",           "hindi": "तो",              "marathi": "तर",
        "bengali": "তবে",            "odia": "ତାହେଲେ",
        "tamil": "அப்போது",          "telugu": "అప్పుడు",        "kannada": "ನಂತರ",
        "malayalam": "പിന്നെ",
        "gujarati": "પછી",           "punjabi": "ਤਦ",
        "sinhala": "පසු",
        "mandarin": "那么",           "japanese": "ならば",        "korean": "그러면",
        "thai": "แล้ว",              "vietnamese": "thì",       "khmer": "បន្ទាប់មក",
        "burmese": "ထို့နောက်",       "lao": "ແລ້ວ",
        "arabic": "ثم",              "hebrew": "אז",             "persian": "سپس",
        "urdu": "تب",
        "russian": "тогда",
        "greek": "τότε",
        "spanish": "entonces",       "french": "alors",         "german": "dann",
        "portuguese": "entao",       "italian": "allora",       "dutch": "dan",
        "polish": "wtedy",           "turkish": "o_zaman",      "swedish": "så",
        "norwegian": "da",           "danish": "så",            "hungarian": "akkor",
        "czech": "pak",              "slovak": "potom",         "finnish": "sitten",
        "romanian": "atunci",        "catalan": "aleshores",
        "armenian": "ապա",           "georgian": "მაშინ",
        "swahili": "kisha",          "yoruba": "nígbànáà",      "hausa": "sannan",
        "amharic": "ከዚያ",
        "tibetan": "དེ་ནས",
        "mongolian": "ᠳᠠᠷᠠᠭ᠎ᠠ",
        "malay": "maka",             "indonesian": "maka",      "filipino": "kung_gayon",
    },

    # ── References ───────────────────────────────────────────────────────────
    "Ref": {
        "english": "ref",
        "sanskrit": "दृष्ट्या",      "hindi": "देखो",            "marathi": "पहा",
        "bengali": "দেখ",            "odia": "ଦେଖନ୍ତୁ",
        "tamil": "பார்",             "telugu": "చూడు",          "kannada": "ನೋಡಿ",
        "malayalam": "നോക്കുക",
        "gujarati": "જુઓ",           "punjabi": "ਵੇਖੋ",
        "sinhala": "බලන්න",
        "mandarin": "引用",           "japanese": "参照",         "korean": "참조",
        "thai": "ดู",                "vietnamese": "tham_chiếu","khmer": "មើល",
        "burmese": "ကြည့်",           "lao": "ເບິ່ງ",
        "arabic": "مرجع",            "hebrew": "הפנייה",         "persian": "ببین",
        "russian": "смотри",
        "greek": "αναφορά",
        "spanish": "ver",            "french": "référence",     "german": "sehen",
        "portuguese": "ver",         "italian": "vedere",       "dutch": "zie",
        "polish": "zobacz",          "turkish": "gör",          "swedish": "se",
        "norwegian": "se",           "danish": "se",            "hungarian": "nézd",
        "czech": "viz",              "slovak": "pozri",         "finnish": "katso",
        "romanian": "vezi",          "catalan": "veure",
        "armenian": "տեսնել",        "georgian": "ნახე",
        "swahili": "tazama",         "hausa": "duba",
        "tibetan": "ལྟ",
        "cherokee": "ᎯᎪᎲᎢ",         "mongolian": "ᠦᠵᠡ",
        "malay": "lihat",            "indonesian": "lihat",
    },
    "Mut": {
        "english": "mut",
        "sanskrit": "परिवर्तनीय",    "hindi": "परिवर्तनीय",      "marathi": "बदल",
        "bengali": "পরিবর্তনীয়",    "odia": "ପରିବର୍ତ୍ତନୀୟ",
        "tamil": "மாறக்கூடிய",      "telugu": "మార్చదగిన",      "kannada": "ಪರಿವರ್ತನೀಯ",
        "malayalam": "മാറ്റാവുന്ന",
        "gujarati": "પરિવર્તનીય",    "punjabi": "ਬਦਲਣਯੋਗ",
        "sinhala": "පරිවර්තනීය",
        "mandarin": "可变",           "japanese": "可変",         "korean": "가변",
        "thai": "เปลี่ยนแปลงได้",    "vietnamese": "có_thể_thay_đổi",
        "arabic": "متغير",           "hebrew": "משתנה",          "persian": "تغییرپذیر",
        "urdu": "بدلنا",
        "russian": "изменяемый",
        "greek": "μεταβλητό",
        "spanish": "mutable",        "french": "muable",        "german": "veränderlich",
        "portuguese": "mutavel",     "italian": "mutevole",     "dutch": "veranderlijk",
        "polish": "zmienny",         "turkish": "değişken",     "swedish": "föränderlig",
        "norwegian": "foranderlig",  "danish": "foranderlig",   "hungarian": "változó",
        "czech": "proměnný",         "slovak": "meniteľný",     "finnish": "muuttuva",
        "romanian": "schimbabil",    "catalan": "mutable",
        "armenian": "փոփոխական",     "georgian": "ცვალებადი",
        "swahili": "badilika",       "hausa": "canzawa",
        "cherokee": "ᏚᎵᎮᎵᎬᎢ",
        "malay": "berubah",          "indonesian": "dapatberubah",
    },

    # ── Matching ─────────────────────────────────────────────────────────────
    "Match": {
        "english": "match",
        "sanskrit": "मेल",           "hindi": "मिलान",           "marathi": "जुळवा",
        "bengali": "মেলে",           "odia": "ମେଳ",
        "tamil": "பொருந்து",         "telugu": "సరిపోలు",        "kannada": "ಹೊಂದಾಣಿಕೆ",
        "malayalam": "പൊരുത്തപ്പെടുത്തുക",
        "gujarati": "મેળવો",         "punjabi": "ਮੇਲ",
        "sinhala": "ගැලපීම",
        "mandarin": "匹配",           "japanese": "一致",         "korean": "일치",
        "thai": "ตรงกัน",            "vietnamese": "khớp",     "khmer": "ផ្គូផ្គង",
        "burmese": "ကိုက်ညီ",        "lao": "ກົງກັນ",
        "arabic": "طابق",            "hebrew": "התאם",           "persian": "تطبیق",
        "russian": "совпадение",
        "greek": "αντιστοιχία",
        "spanish": "coincidir",      "french": "correspondre",  "german": "übereinstimmen",
        "portuguese": "combinar",    "italian": "corrispondere","dutch": "vergelijk",
        "polish": "dopasuj",         "turkish": "eşle",         "swedish": "matcha",
        "norwegian": "sammenlign",   "danish": "match",         "hungarian": "egyezzen",
        "czech": "odpovídej",        "slovak": "porovnaj",      "finnish": "vastaa",
        "romanian": "potrivește",    "catalan": "coincideix",
        "armenian": "համապատասխանեցնել","georgian": "შესაბამისობა",
        "swahili": "linganisha",     "hausa": "dace",
        "tibetan": "མཐུན",
        "malay": "padan",            "indonesian": "cocokkan",
    },

    # ── Verification ─────────────────────────────────────────────────────────
    "Assert": {
        "english": "assert",
        "sanskrit": "सिद्धम्",       "hindi": "सुनिश्चित",       "marathi": "खात्री",
        "bengali": "নিশ্চিত",        "odia": "ନିଶ୍ଚିତ",
        "tamil": "உறுதி",            "telugu": "నిర్ధారించు",     "kannada": "ಖಚಿತಪಡಿಸಿ",
        "malayalam": "ഉറപ്പിക്കുക",
        "gujarati": "નિશ્ચિત",       "punjabi": "ਨਿਸ਼ਚਿਤ",
        "sinhala": "තහවුරු",
        "mandarin": "断言",           "japanese": "確認",         "korean": "확인",
        "thai": "ยืนยัน",            "vietnamese": "khẳng_định","khmer": "បញ្ជាក់",
        "burmese": "သေချာ",          "lao": "ຢືນຢັນ",
        "arabic": "تأكد",            "hebrew": "ודא",            "persian": "ادعا",
        "russian": "утверждать",
        "greek": "επιβεβαίωση",
        "spanish": "afirmar",        "french": "affirmer",      "german": "überprüfen",
        "portuguese": "afirmar",     "italian": "affermare",    "dutch": "bevestig",
        "polish": "potwierdź",       "turkish": "doğrula",      "swedish": "påstå",
        "norwegian": "påstå",        "danish": "påstå",         "hungarian": "állítsd",
        "czech": "tvrď",             "slovak": "potvrď",        "finnish": "vahvista",
        "romanian": "afirmă",        "catalan": "afirma",
        "armenian": "հաստատել",      "georgian": "დაამოწმე",
        "swahili": "thibitisha",     "yoruba": "jẹ́risí",       "hausa": "tabbatar",
        "amharic": "አረጋግጥ",
        "tibetan": "ངེས",
        "mongolian": "ᠪᠠᠲᠤᠯ",
        "malay": "pastikan",         "indonesian": "pastikan",  "filipino": "patunayan",
    },
    "Prove": {
        "english": "prove",
        "sanskrit": "प्रमाण",        "hindi": "सिद्ध करो",       "marathi": "सिद्ध करा",
        "bengali": "প্রমাণ",         "odia": "ପ୍ରମାଣ",
        "tamil": "நிரூபி",           "telugu": "నిరూపించు",      "kannada": "ಸಾಬೀತುಪಡಿಸಿ",
        "malayalam": "തെളിയിക്കുക",
        "gujarati": "પ્રમાણ",        "punjabi": "ਪ੍ਰਮਾਣ",
        "sinhala": "ඔප්පු",
        "mandarin": "证明",           "japanese": "証明",         "korean": "증명",
        "thai": "พิสูจน์",           "vietnamese": "chứng_minh","khmer": "បង្ហាញ",
        "burmese": "သက်သေပြ",        "lao": "ພິສູດ",
        "arabic": "أثبت",            "hebrew": "הוכח",           "persian": "اثبات",
        "russian": "доказать",
        "greek": "απόδειξη",
        "spanish": "demostrar",      "french": "démontrer",     "german": "beweisen",
        "portuguese": "provar",      "italian": "dimostrare",   "dutch": "bewijs",
        "polish": "udowodnij",       "turkish": "kanıtla",      "swedish": "bevisa",
        "norwegian": "bevis",        "danish": "bevis",         "hungarian": "bizonyítsd",
        "czech": "dokaž",            "slovak": "dokáž",         "finnish": "todista",
        "romanian": "dovedește",     "catalan": "demostra",
        "armenian": "ապացուցել",     "georgian": "დაამტკიცე",
        "swahili": "thibitisha_ki",  "hausa": "nuna",
        "amharic": "አስረዳ",
        "tibetan": "བསྒྲུབས",
        "mongolian": "ᠨᠣᠲᠠᠯᠠ",
        "malay": "buktikan",         "indonesian": "buktikan",  "filipino": "ipakita",
    },
    "Requires": {
        "english": "requires",
        "sanskrit": "अपेक्षित",      "hindi": "चाहिए",           "marathi": "पाहिजे",
        "bengali": "প্রয়োজনীয়",    "odia": "ଆବଶ୍ୟକ",
        "tamil": "தேவை",             "telugu": "అవసరం",          "kannada": "ಅಗತ್ಯ",
        "malayalam": "ആവശ്യം",
        "gujarati": "જરૂરી",         "punjabi": "ਲੋੜੀਂਦਾ",
        "sinhala": "අවශ්‍ය",
        "mandarin": "要求",           "japanese": "前提",         "korean": "필요",
        "thai": "ต้องการ",            "vietnamese": "yêu_cầu",  "khmer": "ត្រូវការ",
        "burmese": "လို",            "lao": "ຕ້ອງການ",
        "arabic": "يتطلب",           "hebrew": "דורש",           "persian": "نیاز",
        "russian": "требует",
        "greek": "απαιτεί",
        "spanish": "requiere",       "french": "exige",         "german": "benoetigt",
        "portuguese": "requer",      "italian": "richiede",     "dutch": "vereist",
        "polish": "wymaga",          "turkish": "gerektirir",   "swedish": "kräver",
        "norwegian": "krever",       "danish": "kræver",        "hungarian": "igényel",
        "czech": "vyžaduje",         "slovak": "vyžaduje",      "finnish": "vaatii",
        "romanian": "necesită",      "catalan": "requereix",
        "armenian": "պահանջում",     "georgian": "მოითხოვს",
        "swahili": "hitaji",         "hausa": "bukata",
        "amharic": "ይፈልጋል",
        "tibetan": "དགོས",
        "malay": "memerlukan",       "indonesian": "perlu",     "filipino": "kailangan",
    },
    "Ensures": {
        "english": "ensures",
        "sanskrit": "सुनिश्चयित",    "hindi": "निश्चित",         "marathi": "निश्चित",
        "bengali": "সুনিশ্চিত",      "odia": "ସୁନିଶ୍ଚିତ",
        "tamil": "உறுதிப்படுத்து",   "telugu": "నిశ్చయం",        "kannada": "ಖಚಿತ",
        "malayalam": "ഉറപ്പ്",
        "gujarati": "ખાતરી",         "punjabi": "ਯਕੀਨੀ",
        "sinhala": "සහතික",
        "mandarin": "保证",           "japanese": "保証",         "korean": "보장",
        "thai": "รับประกัน",          "vietnamese": "đảm_bảo",  "khmer": "ធានា",
        "burmese": "ဆောင်ရွက်",      "lao": "ຮັບປະກັນ",
        "arabic": "يضمن",            "hebrew": "מבטיח",          "persian": "تضمین",
        "russian": "гарантирует",
        "greek": "εγγυάται",
        "spanish": "garantiza",      "french": "garantit",      "german": "garantiert",
        "portuguese": "garante",     "italian": "garantisce",   "dutch": "verzekert",
        "polish": "gwarantuje",      "turkish": "sağlar",       "swedish": "säkerställer",
        "norwegian": "garanterer",   "danish": "garanterer",    "hungarian": "garantál",
        "czech": "zajišťuje",        "slovak": "zaručuje",      "finnish": "takaa",
        "romanian": "garantează",    "catalan": "garanteix",
        "armenian": "երաշխավորում",  "georgian": "უზრუნველყოფს",
        "swahili": "hakikisha",      "hausa": "tabbace",
        "amharic": "ያረጋግጣል",
        "tibetan": "ཁག",
        "malay": "menjamin",         "indonesian": "jamin",     "filipino": "tiyakin",
    },

    # ── Bool / print ─────────────────────────────────────────────────────────
    "True": {
        "english": "true",
        "sanskrit": "सत्य",          "hindi": "सत्य",            "marathi": "सत्य",
        "bengali": "সত্য",           "odia": "ସତ୍ୟ",
        "tamil": "மெய்",             "telugu": "నిజం",           "kannada": "ಸತ್ಯ",
        "malayalam": "സത്യം",
        "gujarati": "સાચું",         "punjabi": "ਸੱਚ",
        "sinhala": "සත්‍ය",
        "mandarin": "真",            "japanese": "真",           "korean": "참",
        "thai": "จริง",              "vietnamese": "đúng",     "khmer": "ពិត",
        "burmese": "မှန်",           "lao": "ຈິງ",
        "arabic": "صحيح",            "hebrew": "אמת",            "persian": "درست",
        "urdu": "سچ",                "pashto": "سم",
        "russian": "истина",
        "greek": "αληθές",
        "spanish": "verdadero",      "french": "vrai",          "german": "wahr",
        "portuguese": "verdadeiro",  "italian": "vero",         "dutch": "waar",
        "polish": "prawda",          "turkish": "doğru",        "swedish": "sant",
        "norwegian": "sant",         "danish": "sandt",         "hungarian": "igaz",
        "czech": "pravda",           "slovak": "pravda",        "finnish": "tosi",
        "romanian": "adevărat",      "catalan": "cert",
        "armenian": "ճշմարիտ",       "georgian": "ჭეშმარიტი",
        "swahili": "kweli",          "yoruba": "òótọ́",         "hausa": "gaskiya",
        "amharic": "እውነት",
        "tibetan": "བདེན",
        "cherokee": "ᎤᏙᎯᏳ",         "mongolian": "ᠦᠨᠡᠨ",
        "malay": "benar",            "indonesian": "benar",     "filipino": "totoo",
    },
    "False": {
        "english": "false",
        "sanskrit": "असत्य",         "hindi": "असत्य",           "marathi": "असत्य",
        "bengali": "অসত্য",          "odia": "ମିଥ୍ୟା",
        "tamil": "பொய்",             "telugu": "అబద్ధం",        "kannada": "ಸುಳ್ಳು",
        "malayalam": "അസത്യം",
        "gujarati": "ખોટું",         "punjabi": "ਝੂਠ",
        "sinhala": "අසත්‍ය",
        "mandarin": "假",            "japanese": "偽",           "korean": "거짓",
        "thai": "เท็จ",              "vietnamese": "sai",      "khmer": "មិនពិត",
        "burmese": "မှား",           "lao": "ບໍ່ຈິງ",
        "arabic": "خطأ",             "hebrew": "שקר",            "persian": "نادرست",
        "urdu": "جھوٹ",              "pashto": "ناسم",
        "russian": "ложь",
        "greek": "ψευδές",
        "spanish": "falso",          "french": "faux",          "german": "falsch",
        "portuguese": "falso",       "italian": "falso",        "dutch": "onwaar",
        "polish": "fałsz",           "turkish": "yanlış",       "swedish": "falskt",
        "norwegian": "usant",        "danish": "falsk",         "hungarian": "hamis",
        "czech": "nepravda",         "slovak": "nepravda",      "finnish": "epatosi",
        "romanian": "fals",          "catalan": "fals",
        "armenian": "կեղծ",          "georgian": "მცდარი",
        "swahili": "uongo",          "yoruba": "irọ́",          "hausa": "ƙarya",
        "amharic": "ሐሰት",
        "tibetan": "རྫུན",
        "cherokee": "ᎤᏝ",           "mongolian": "ᠬᠤᠳᠠᠯ",
        "malay": "palsu",            "indonesian": "salah",     "filipino": "mali",
    },
    "Print": {
        "english": "print",
        "sanskrit": "लिख",           "hindi": "लिखो",            "marathi": "लिहा",
        "bengali": "লেখ",            "odia": "ଲେଖ",
        "tamil": "எழுது",            "telugu": "రాయి",          "kannada": "ಬರೆ",
        "malayalam": "എഴുതുക",
        "gujarati": "લખો",           "punjabi": "ਲਿਖੋ",
        "sinhala": "ලියන්න",
        "mandarin": "打印",           "japanese": "表示",         "korean": "출력",
        "thai": "พิมพ์",             "vietnamese": "in_ra",    "khmer": "បោះពុម្ព",
        "burmese": "ပုံနှိပ်",        "lao": "ພິມ",
        "arabic": "اطبع",            "hebrew": "הדפס",           "persian": "چاپ",
        "urdu": "لکھو",              "pashto": "ولیکه",
        "russian": "печатать",
        "greek": "εκτύπωση",
        "spanish": "imprimir",       "french": "imprimer",      "german": "drucken",
        "portuguese": "imprimir",    "italian": "stampare",     "dutch": "druk",
        "polish": "drukuj",          "turkish": "yazdır",       "swedish": "skriv",
        "norwegian": "skriv",        "danish": "udskriv",       "hungarian": "nyomtass",
        "czech": "vypiš",            "slovak": "vytlač",        "finnish": "tulosta",
        "romanian": "tipărește",     "catalan": "imprimeix",
        "armenian": "տպել",          "georgian": "ბეჭდვა",
        "swahili": "chapisha",       "yoruba": "tẹ̀",          "hausa": "rubuta",
        "amharic": "ህትመት",
        "tibetan": "པར",
        "mongolian": "ᠬᠡᠪᠯᠡ",
        "malay": "cetak",            "indonesian": "cetak",     "filipino": "isulat",
    },

    # ── Purity / parallelism ─────────────────────────────────────────────────
    "Pure": {
        "english": "pure",
        "sanskrit": "शुद्ध",         "hindi": "शुद्ध",           "marathi": "शुद्ध",
        "mandarin": "纯",             "japanese": "純粋",          "korean": "순수",
        "thai": "บริสุทธิ์",         "vietnamese": "thuần_túy",
        "arabic": "نقي",             "hebrew": "טהור",
        "russian": "чистый",
        "greek": "καθαρό",
        "spanish": "puro",           "french": "pur",           "german": "rein",
        "portuguese": "puro",        "italian": "puro",         "dutch": "zuiver",
        "polish": "czysty",          "turkish": "saf",          "swedish": "ren",
        "norwegian": "ren",          "danish": "ren",           "hungarian": "tiszta",
        "czech": "čistý",            "slovak": "čistý",         "finnish": "puhdas",
        "romanian": "pur",           "catalan": "pur",
        "swahili": "safi",
        "amharic": "ንጹህ",
        "tibetan": "གཙང",
        "mongolian": "ᠴᠡᠪᠡᠷ",
        "malay": "tulen",            "indonesian": "murni",     "filipino": "dalisay",
    },
    "Parallel": {
        "english": "parallel",
        "sanskrit": "समानांतर",      "hindi": "समानांतर",        "marathi": "समानांतर",
        "mandarin": "并行",           "japanese": "並列",          "korean": "병렬",
        "thai": "ขนาน",              "vietnamese": "song_song",
        "arabic": "متوازي",          "hebrew": "מקבילי",
        "russian": "параллельный",
        "greek": "παράλληλο",
        "spanish": "paralelo",       "french": "parallèle",     "german": "parallel",
        "portuguese": "paralelo",    "italian": "parallelo",    "dutch": "parallel",
        "polish": "równoległy",      "turkish": "paralel",      "swedish": "parallell",
        "norwegian": "parallell",    "danish": "parallel",      "hungarian": "párhuzamos",
        "czech": "paralelní",        "slovak": "paralelný",     "finnish": "rinnakkainen",
        "romanian": "paralel",       "catalan": "paral_lel",
        "swahili": "sambamba",
        "amharic": "ትይዩ",
        "malay": "selari",           "indonesian": "paralel",   "filipino": "magkatulad",
    },
    "Reduce": {
        "english": "reduce",
        "sanskrit": "संक्षेप",       "hindi": "संक्षेप",          "marathi": "संक्षेप",
        "mandarin": "reduce",
    },
    "With": {
        "english": "with",
        "sanskrit": "सह",            "hindi": "सह",              "marathi": "सह",
        "mandarin": "with",
    },

    # ── Interfaces / methods ─────────────────────────────────────────────────
    "Interface": {
        "english": "interface",
        "sanskrit": "संकेत",         "hindi": "संकेत",           "marathi": "संकेत",
        "mandarin": "接口",           "japanese": "インターフェース","korean": "인터페이스",
        "thai": "อินเทอร์เฟซ",       "vietnamese": "giao_diện",
        "arabic": "واجهة",           "hebrew": "ממשק",            "persian": "رابط",
        "urdu": "رابطہ",
        "russian": "интерфейс",
        "greek": "διεπαφή",
        "spanish": "interfaz",       "french": "interface",     "german": "schnittstelle",
        "portuguese": "interface",   "italian": "interfaccia",  "dutch": "interface",
        "polish": "interfejs",       "turkish": "arayüz",       "swedish": "gränssnitt",
        "norwegian": "grensesnitt",  "danish": "grænseflade",   "hungarian": "felület",
        "czech": "rozhraní",         "slovak": "rozhranie",     "finnish": "rajapinta",
        "romanian": "interfață",     "catalan": "interface",
        "armenian": "միջերես",       "georgian": "ინტერფეისი",
        "swahili": "kiolesura",      "amharic": "በይነገጽ",
        "malay": "antara_muka",      "indonesian": "antarmuka",
    },
    "Implement": {
        "english": "implement",
        "sanskrit": "कार्यान्वित",   "hindi": "कार्यान्वित",     "marathi": "कार्यान्वित",
        "mandarin": "实现",           "japanese": "実装",          "korean": "구현",
        "thai": "ดำเนินการ",         "vietnamese": "triển_khai",
        "arabic": "نفذ",             "hebrew": "ממש",
        "russian": "реализовать",
        "greek": "υλοποίηση",
        "spanish": "implementar",    "french": "implémenter",   "german": "implementieren",
        "portuguese": "implementar", "italian": "implementare", "dutch": "implementeer",
        "polish": "zaimplementuj",   "turkish": "uygula",       "swedish": "implementera",
        "norwegian": "implementer",  "danish": "implementer",   "hungarian": "valositsd_meg",
        "czech": "implementuj",      "slovak": "implementuj",   "finnish": "toteuta",
        "romanian": "implementează", "catalan": "implementa",
        "armenian": "իրականացնել",   "georgian": "განხორციელება",
        "amharic": "ተግብር",
        "malay": "laksanakan",       "indonesian": "terapkan",  "filipino": "ipatupad",
    },
    "Methods": {
        "english": "methods",
        "sanskrit": "विधि",           "hindi": "विधि",            "marathi": "विधि",
        "mandarin": "方法",            "japanese": "メソッド",      "korean": "메서드",
        "thai": "วิธีการ",            "vietnamese": "phương_thức",
        "arabic": "طرق",             "hebrew": "שיטות",
        "russian": "методы",
        "greek": "μέθοδοι",
        "spanish": "metodos",        "french": "méthodes",      "german": "methoden",
        "portuguese": "metodos",     "italian": "metodi",       "dutch": "methoden",
        "polish": "metody",          "turkish": "metodlar",     "swedish": "metoder",
        "norwegian": "metoder",      "danish": "metoder",       "hungarian": "metódusok",
        "czech": "metody",           "slovak": "metody",        "finnish": "menetelmat",
        "romanian": "metode",        "catalan": "metodes",
        "armenian": "մեթոդներ",      "georgian": "მეთოდები",
        "amharic": "ዘዴዎች",
        "malay": "kaedah",           "indonesian": "metode",    "filipino": "pamamaraan",
    },

    # ── Bounds ───────────────────────────────────────────────────────────────
    "Where": {
        "english": "where",
        "sanskrit": "यत्र",           "hindi": "जहाँ",            "marathi": "जिथे",
        "mandarin": "其中",            "japanese": "ここで",        "korean": "여기서",
        "thai": "ที่ไหน",             "vietnamese": "ở_đâu",
        "arabic": "حيث",             "hebrew": "איפה",
        "russian": "где",
        "greek": "όπου",
        "spanish": "donde",          "french": "où",            "german": "wo",
        "portuguese": "onde",        "italian": "dove",         "dutch": "waar_is",
        "polish": "gdzie",           "turkish": "nerede",       "swedish": "där",
        "norwegian": "hvor",         "danish": "hvor",          "hungarian": "ahol",
        "czech": "kde",              "slovak": "kde",           "finnish": "missä",
        "romanian": "unde",          "catalan": "on",
        "armenian": "որտեղ",         "georgian": "სად",
        "swahili": "wapi",           "hausa": "ina",
        "amharic": "የት",
        "malay": "di_mana",          "indonesian": "dimana",    "filipino": "saan",
    },
    "Is": {
        "english": "is",
        "sanskrit": "अस्ति",          "hindi": "है",              "marathi": "आहे",
        "mandarin": "is",             "japanese": "は",            "korean": "이다",
        "thai": "คือ",               "vietnamese": "là",
        "arabic": "هو",              "hebrew": "הוא",
        "russian": "есть",
        "greek": "είναι",
        "spanish": "es",             "french": "est",           "german": "ist",
        "portuguese": "eh",          "italian": "è",            "dutch": "is",
        "polish": "jest",            "turkish": "dır",          "swedish": "är",
        "norwegian": "er",           "danish": "er",            "hungarian": "van",
        "czech": "je",               "slovak": "je",            "finnish": "on",
        "romanian": "este",          "catalan": "es",
        "armenian": "է",             "georgian": "არის",
        "hausa": "ne",
        "amharic": "ነው",
        "malay": "adalah",           "indonesian": "adalah",    "filipino": "ay",
    },

    # ── Concurrency ──────────────────────────────────────────────────────────
    "Try": {
        "english": "try",
        "sanskrit": "प्रयास",         "hindi": "प्रयास",          "marathi": "प्रयास",
        "mandarin": "尝试",            "japanese": "試行",          "korean": "시도",
        "thai": "ลอง",               "vietnamese": "thử",
        "arabic": "حاول",            "hebrew": "נסה",
        "russian": "попытка",
        "greek": "δοκιμή",
        "spanish": "intentar",       "french": "essayer",       "german": "versuchen",
        "portuguese": "tentar",      "italian": "tentare",      "dutch": "probeer",
        "polish": "spróbuj",         "turkish": "dene",         "swedish": "försök",
        "norwegian": "prøv",         "danish": "prøv",          "hungarian": "próbáld",
        "czech": "zkus",             "slovak": "skus",          "finnish": "kokeile",
        "romanian": "încearcă",      "catalan": "prova",
        "armenian": "փորձել",        "georgian": "სცადე",
        "swahili": "jaribu",         "hausa": "gwadawa",
        "amharic": "ሞክር",
        "malay": "cuba",             "indonesian": "coba",      "filipino": "subukan",
    },
    "Task": {
        "english": "task",
        "sanskrit": "नियोग",          "hindi": "नियोग",           "marathi": "नियोग",
        "bengali": "নিয়োগ",
        "mandarin": "任务",            "japanese": "タスク",        "korean": "작업",
        "thai": "งาน",               "vietnamese": "công_việc",
        "arabic": "مهمة",            "hebrew": "משימה",           "persian": "وظیفه",
        "urdu": "کام",
        "russian": "задача",
        "greek": "εργασία",
        "spanish": "tarea",          "french": "tâche",         "german": "aufgabe",
        "portuguese": "tarefa",      "italian": "compito",      "dutch": "taak",
        "polish": "zadanie",         "turkish": "görev",        "swedish": "uppgift",
        "norwegian": "oppgave",      "danish": "opgave",        "hungarian": "feladat",
        "czech": "úloha",            "slovak": "úloha",         "finnish": "tehtava",
        "romanian": "sarcină",       "catalan": "tasca",
        "armenian": "խնդիր",         "georgian": "დავალება",
        "swahili": "kazi_ndogo",     "hausa": "aiki_kadan",
        "amharic": "ስራ",
        "mongolian": "ᠡᠭᠦᠷᠭᠡ",
        "malay": "tugasan",          "indonesian": "tugas",     "filipino": "tungkulin",
    },
    "Join": {
        "english": "join",
        "sanskrit": "संयोजन",         "hindi": "संयोजन",          "marathi": "संयोजन",
        "mandarin": "等待",            "japanese": "結合",          "korean": "결합",
        "thai": "รวม",               "vietnamese": "kết_hợp",
        "arabic": "اربط",            "hebrew": "חיבור",
        "russian": "соединить",
        "greek": "ένωση",
        "spanish": "unir",           "french": "joindre",       "german": "verbinden",
        "portuguese": "juntar",      "italian": "unire",        "dutch": "verbind",
        "polish": "połącz",          "turkish": "birleştir",    "swedish": "förena",
        "norwegian": "forene",       "danish": "forén",         "hungarian": "egyesít",
        "czech": "spoj",             "slovak": "spoj",          "finnish": "yhdista",
        "romanian": "unește",        "catalan": "uneix",
        "armenian": "միանալ",        "georgian": "შეერთება",
        "swahili": "unganisha",      "hausa": "hadawa",
        "amharic": "ቀላቀል",
        "tibetan": "མཐུན་སྦྱོར",
        "malay": "bergabung",        "indonesian": "gabungkan", "filipino": "pagsama",
    },

    # ── Embedded ─────────────────────────────────────────────────────────────
    "Unsafe": {
        "english": "unsafe",
        "sanskrit": "असुरक्षित",      "hindi": "असुरक्षित",       "marathi": "असुरक्षित",
        "mandarin": "不安全",          "japanese": "危険",          "korean": "위험",
        "thai": "ไม่ปลอดภัย",         "vietnamese": "không_an_toàn",
        "arabic": "غير_آمن",         "hebrew": "מסוכן",
        "russian": "небезопасно",
        "greek": "επικίνδυνο",
        "spanish": "inseguro",       "french": "dangereux",     "german": "unsicher",
        "portuguese": "inseguro",    "italian": "insicuro",     "dutch": "onveilig",
        "polish": "niebezpieczny",   "turkish": "güvensiz",     "swedish": "osäker",
        "norwegian": "usikker",      "danish": "usikker",       "hungarian": "veszélyes",
        "czech": "nebezpečný",       "slovak": "nebezpečný",    "finnish": "vaarallinen",
        "romanian": "nesigur",       "catalan": "insegur",
        "armenian": "անապահով",      "georgian": "სახიფათო",
        "amharic": "አደገኛ",
        "tibetan": "ཉེན་ཁ",
        "malay": "tidak_selamat",    "indonesian": "bahaya",
    },
    "RegionKw": {
        "english": "region",
        "sanskrit": "क्षेत्र",         "hindi": "क्षेत्र",         "marathi": "क्षेत्र",
        "mandarin": "区域",             "japanese": "領域",          "korean": "영역",
        "thai": "พื้นที่",             "vietnamese": "vùng",
        "arabic": "منطقة",            "hebrew": "אזור",
        "russian": "область",
        "greek": "περιοχή",
        "spanish": "region",         "french": "région",        "german": "bereich",
        "portuguese": "regiao",      "italian": "regione",      "dutch": "gebied",
        "polish": "obszar",          "turkish": "bölge",        "swedish": "region",
        "norwegian": "område",       "danish": "område",        "hungarian": "tartomány",
        "czech": "oblast",           "slovak": "oblasť",        "finnish": "alue",
        "romanian": "regiune",       "catalan": "regio",
        "armenian": "տարածք",        "georgian": "რეგიონი",
        "amharic": "ክልል",
        "tibetan": "ཁུལ",
        "malay": "kawasan",          "indonesian": "wilayah",   "filipino": "rehiyon",
    },
}

SUPPORTED_LANGS = (
    "english", "sanskrit", "hindi", "marathi", "mandarin",
    # South Asian
    "bengali", "odia", "gujarati", "punjabi", "sinhala",
    # Dravidian
    "tamil", "telugu", "kannada", "malayalam",
    # East Asian
    "japanese", "korean",
    # Southeast Asian
    "thai", "vietnamese", "khmer", "burmese", "lao",
    "malay", "indonesian", "filipino",
    # Middle Eastern / RTL
    "arabic", "hebrew", "persian", "urdu", "pashto",
    # Cyrillic
    "russian",
    # European (non-Latin-script)
    "greek",
    # European (Latin-script)
    "spanish", "french", "german", "portuguese", "italian", "dutch",
    "polish", "turkish", "swedish", "norwegian", "danish",
    "hungarian", "czech", "slovak", "finnish", "romanian", "catalan",
    # Caucasian
    "armenian", "georgian",
    # African
    "swahili", "yoruba", "hausa", "amharic",
    # Other scripts
    "tibetan", "cherokee", "mongolian",
)

# Devanagari Indo-Aryan targets that get the श्री। header.
_IA_DEVANAGARI = frozenset(("sanskrit", "hindi", "marathi", "nepali", "maithili", "konkani"))

# Languages with SOV (Subject-Object-Verb) word order for certain constructs.
SOV_LANGS = frozenset({
    "sanskrit", "hindi", "marathi",
    "bengali", "odia", "gujarati", "punjabi", "sinhala",
    "tamil", "telugu", "kannada", "malayalam",
    "japanese", "korean",
    "urdu", "persian", "pashto",
    "turkish", "mongolian", "tibetan",
})

# Multi-word forms that the lexer fuses post-tokenization.
MULTI_WORD_ALIASES: Dict[Tuple[str, ...], str] = {
    ("नहीं", "तो"):      "Else",
    ("के", "लिए"):        "For",
    ("सिद्ध", "करो"):     "Prove",
    ("सिद्ध", "करा"):     "Prove",
    ("समान्तर", "प्रति"): "Parallel",
}

# ---------------------------------------------------------------------------
# SOV word-order helpers
# ---------------------------------------------------------------------------

# Verb-final token kinds: these appear at the END of the statement in SOV langs.
_SOV_VERB_FINAL_KINDS = frozenset({"Return", "Print", "Assert", "Prove"})

# Build: spelling -> kind, for every non-English SOV verb-final keyword.
# Single-word forms only here (multi-word forms handled separately below).
_VERB_FINAL_SPELLINGS: Dict[str, str] = {}
for _kind in _SOV_VERB_FINAL_KINDS:
    for _lang, _spelling in ALIASES[_kind].items():
        if _lang != "english" and " " not in _spelling:
            _VERB_FINAL_SPELLINGS[_spelling] = _kind

# Multi-word verb-final spellings: "WORD1 WORD2" -> kind
_MULTI_WORD_VERB_FINALS: Dict[str, str] = {
    " ".join(pair): kind
    for pair, kind in MULTI_WORD_ALIASES.items()
    if kind in _SOV_VERB_FINAL_KINDS
}


def _is_word_char(c: str) -> bool:
    if c == "_" or c.isalnum():
        return True
    cp = ord(c)
    return (
        0x0530 <= cp <= 0x058F or   # Armenian
        0x0590 <= cp <= 0x05FF or   # Hebrew
        0x0600 <= cp <= 0x06FF or   # Arabic / Urdu / Persian / Pashto
        0x0900 <= cp <= 0x097F or   # Devanagari
        0x0980 <= cp <= 0x09FF or   # Bengali
        0x0A00 <= cp <= 0x0A7F or   # Gurmukhi (Punjabi)
        0x0A80 <= cp <= 0x0AFF or   # Gujarati
        0x0B00 <= cp <= 0x0B7F or   # Odia
        0x0B80 <= cp <= 0x0BFF or   # Tamil
        0x0C00 <= cp <= 0x0C7F or   # Telugu
        0x0C80 <= cp <= 0x0CFF or   # Kannada
        0x0D00 <= cp <= 0x0D7F or   # Malayalam
        0x0D80 <= cp <= 0x0DFF or   # Sinhala
        0x0E00 <= cp <= 0x0E7F or   # Thai
        0x0E80 <= cp <= 0x0EFF or   # Lao
        0x0F00 <= cp <= 0x0FFF or   # Tibetan
        0x1000 <= cp <= 0x109F or   # Burmese / Myanmar
        0x10A0 <= cp <= 0x10FF or   # Georgian
        0x1100 <= cp <= 0x11FF or   # Korean Jamo
        0x1200 <= cp <= 0x137F or   # Ethiopic (Amharic)
        0x13A0 <= cp <= 0x13FF or   # Cherokee
        0x1780 <= cp <= 0x17FF or   # Khmer
        0x1800 <= cp <= 0x18AF or   # Mongolian
        0x0370 <= cp <= 0x03FF or   # Greek
        0x0400 <= cp <= 0x04FF or   # Cyrillic (Russian)
        0x4E00 <= cp <= 0x9FFF or   # CJK (Mandarin)
        0x3040 <= cp <= 0x30FF or   # Hiragana / Katakana (Japanese)
        0xA8E0 <= cp <= 0xA8FF or   # Devanagari Extended
        0xAC00 <= cp <= 0xD7AF      # Korean Hangul Syllables
    )


def _last_word(s: str) -> Tuple[str, int]:
    """Return (word, start_index) for the last contiguous word in s."""
    end = len(s)
    while end > 0 and not _is_word_char(s[end - 1]):
        end -= 1
    start = end
    while start > 0 and _is_word_char(s[start - 1]):
        start -= 1
    return s[start:end], start


def _try_normalize_verbfinal_line(line: str) -> str:
    """
    If `line` (in a SOV language) ends with a verb-final keyword followed by ;,
    reorder it to English SVO: put the English verb first.

    'n पुनरागम;'      -> 'return n;'
    '  x लिखो;'       -> '  print x;'
    '  x सिद्ध करो;'  -> '  prove x;'   (multi-word Prove)
    """
    stripped = line.rstrip()  # strip all trailing whitespace/newlines
    trailing = line[len(stripped):]  # re-append after reorder (e.g. "\n")

    if not stripped.endswith(";"):
        return line

    # Capture indent; work on body (indent-free "expr VERB" string)
    leading = stripped[: len(stripped) - len(stripped.lstrip())]
    before_semi = stripped[:-1].rstrip()          # "  expr VERB"
    body = before_semi[len(leading):].rstrip()    # "expr VERB" (no indent)

    if not body:
        return line

    # --- single-word verb-final ---
    verb, verb_start = _last_word(body)
    if verb and verb in _VERB_FINAL_SPELLINGS:
        kind = _VERB_FINAL_SPELLINGS[verb]
        expr = body[:verb_start].rstrip()
        english_verb = ALIASES[kind]["english"]
        if expr:
            return f"{leading}{english_verb} {expr};{trailing}"
        return f"{leading}{english_verb};{trailing}"

    # --- two-word verb-final (e.g. "सिद्ध करो", "सिद्ध करा") ---
    before_last = body[:verb_start].rstrip()
    if before_last:
        word2 = verb
        word1, word1_start = _last_word(before_last)
        two_word = f"{word1} {word2}"
        if two_word in _MULTI_WORD_VERB_FINALS:
            kind = _MULTI_WORD_VERB_FINALS[two_word]
            expr = before_last[:word1_start].rstrip()
            english_verb = ALIASES[kind]["english"]
            if expr:
                return f"{leading}{english_verb} {expr};{trailing}"
            return f"{leading}{english_verb};{trailing}"

    return line


def _normalize_sov_to_svo(source: str, src_lang: str) -> str:
    """
    Pre-processing: if source is in a SOV language, reorder verb-final
    statements to SVO (English word order) so that keyword substitution
    produces the correct target output.

    Handles:
      - Verb-final return/print/assert/prove statements (line-level).
      - Hindi for-range:  VAR के लिए START से END तक {  →  for VAR from START to END {
    """
    if src_lang not in SOV_LANGS:
        return source

    # 1. Line-level verb-final reorder.
    trailing_nl = source.endswith("\n")
    source = "\n".join(
        _try_normalize_verbfinal_line(ln)
        for ln in source.splitlines(keepends=False)
    )
    if trailing_nl:
        source += "\n"

    # 2. Hindi for-range: VAR के लिए START से END तक {
    if src_lang == "hindi":
        # Multi-word के लिए = For.  Regex: IDENT (whitespace) के लिए EXPR से EXPR तक (ws) {
        pat = re.compile(
            r'([ \t]*)(\w+)([ \t]+)के\s+लिए([ \t]+)(\S+)([ \t]+)से([ \t]+)(\S+)([ \t]+)तक([ \t]*)\{'
        )
        def _fix_for(m: re.Match) -> str:
            indent, var, _, _, start, _, _, end, _, _, = m.groups()
            return f"{indent}for {var} from {start} to {end} {{"
        source = pat.sub(_fix_for, source)

    return source


def _convert_svo_to_sov(source: str, target_lang: str) -> str:
    """
    Post-processing: if target is a SOV language, reorder SVO verb-initial
    statements to verb-final SOV.

    Handles:
      - Verb-initial return/print/assert/prove statements (line-level).
      - Hindi for-range:  for VAR from START to END {  →  VAR के लिए START से END तक {
    """
    if target_lang not in SOV_LANGS:
        return source

    # Build lookup: english_verb -> target spelling
    target_verb: Dict[str, str] = {
        ALIASES[k]["english"]: ALIASES[k].get(target_lang, ALIASES[k]["english"])
        for k in _SOV_VERB_FINAL_KINDS
    }

    result_lines = []
    for line in source.splitlines(keepends=False):
        stripped = line.rstrip()
        if not stripped.endswith(";"):
            result_lines.append(line)
            continue
        leading = stripped[: len(stripped) - len(stripped.lstrip())]
        body = stripped.lstrip()

        # Check if the line starts with one of the target verbs (as already
        # substituted by translate()) or their English originals.
        matched = False
        for en_verb, sov_verb in target_verb.items():
            # The translate() step will have already replaced 'return' with
            # e.g. 'लौटाओ'.  So we look for either form.
            for look_for in (sov_verb, en_verb):
                if body.startswith(look_for + " ") or body.startswith(look_for + "\t"):
                    expr = body[len(look_for):].strip().rstrip(";")
                    # Skip if there's no expression (e.g. bare `return;`)
                    if expr:
                        result_lines.append(f"{leading}{expr} {sov_verb};")
                    else:
                        result_lines.append(line)
                    matched = True
                    break
            if matched:
                break
        if not matched:
            result_lines.append(line)

    trailing_nl = source.endswith("\n")
    source = "\n".join(result_lines)
    if trailing_nl:
        source += "\n"

    # Hindi for-range:  for VAR from START to END {  →  VAR के लिए START से END तक {
    if target_lang == "hindi":
        for_kw  = ALIASES["For"]["hindi"]   # "के लिए"
        from_kw = ALIASES["From"]["hindi"]  # "से"
        to_kw   = ALIASES["To"]["hindi"]    # "तक"
        # At this point translate() has already substituted: for→के लिए, from→से, to→तक
        # So the (wrong) text looks like: "के लिए VAR से START तक END {"
        pat = re.compile(
            re.escape(for_kw) + r'([ \t]+)(\w+)([ \t]+)' +
            re.escape(from_kw) + r'([ \t]+)(\S+)([ \t]+)' +
            re.escape(to_kw) + r'([ \t]+)(\S+)([ \t]*)\{'
        )
        def _fix_for_sov(m: re.Match) -> str:
            _, var, _, _, start, _, _, end, _ = m.groups()
            return f"{var} {for_kw} {start} {from_kw} {end} {to_kw} {{"
        source = pat.sub(_fix_for_sov, source)

    return source


# ---------------------------------------------------------------------------
# Core keyword translator
# ---------------------------------------------------------------------------

def build_reverse_lookup() -> Dict[str, Tuple[str, str]]:
    rev: Dict[str, Tuple[str, str]] = {}
    for kind, langs in ALIASES.items():
        for lang, spelling in langs.items():
            rev[spelling] = (kind, lang)
    return rev


def detect_pragma_lang(source: str) -> Optional[str]:
    """Return the language declared in the first `// vani-lang: <name>` pragma, or None."""
    for line in source.splitlines():
        stripped = line.lstrip("/").strip()
        for prefix in ("vani-lang:", "vani-lang :"):
            if stripped.startswith(prefix):
                lang = stripped[len(prefix):].strip().lower()
                if lang in SUPPORTED_LANGS:
                    return lang
    return None


def extract_keyword_tokens(source: str) -> List[str]:
    """Return the ordered sequence of TokenKind names found in source."""
    rev = build_reverse_lookup()
    tokens: List[str] = []
    i = 0
    n = len(source)
    while i < n:
        c = source[i]
        if c == "/" and i + 1 < n and source[i + 1] == "/":
            j = source.find("\n", i)
            i = n if j == -1 else j
            continue
        if c == '"':
            i += 1
            while i < n and source[i] != '"':
                if source[i] == "\\" and i + 1 < n:
                    i += 2
                    continue
                i += 1
            if i < n:
                i += 1
            continue
        if _is_word_char(c):
            j = i
            while j < n and _is_word_char(source[j]):
                j += 1
            word = source[i:j]
            k = j
            while k < n and source[k] in (" ", "\t"):
                k += 1
            second = None
            second_end = k
            if k < n and _is_word_char(source[k]):
                m = k
                while m < n and _is_word_char(source[m]):
                    m += 1
                second = source[k:m]
                second_end = m
            if second is not None and (word, second) in MULTI_WORD_ALIASES:
                tokens.append(MULTI_WORD_ALIASES[(word, second)])
                i = second_end
                continue
            if word in rev:
                tokens.append(rev[word][0])
            i = j
            continue
        i += 1
    return tokens


def _translate_keywords(source: str, target_lang: str) -> str:
    """
    Pure keyword substitution (no word-order changes).
    Rewrites the `// vani-lang:` pragma to target_lang.
    """
    assert target_lang in SUPPORTED_LANGS, f"unknown target {target_lang!r}"
    rev = build_reverse_lookup()
    out: List[str] = []
    i = 0
    n = len(source)
    while i < n:
        c = source[i]
        # Line comment — pass through, rewriting vani-lang pragma.
        if c == "/" and i + 1 < n and source[i + 1] == "/":
            j = source.find("\n", i)
            if j == -1:
                j = n
            line = source[i:j]
            stripped = line.lstrip("/").strip()
            if stripped.startswith("vani-lang:") or stripped.startswith("vani-lang :"):
                leading = line[: len(line) - len(line.lstrip("/ \t"))]
                out.append(f"{leading}vani-lang: {target_lang}")
            else:
                out.append(line)
            i = j
            continue
        # String literal — copy through, handling escapes.
        if c == '"':
            out.append(c)
            i += 1
            while i < n and source[i] != '"':
                if source[i] == "\\" and i + 1 < n:
                    out.append(source[i:i + 2])
                    i += 2
                    continue
                out.append(source[i])
                i += 1
            if i < n:
                out.append(source[i])
                i += 1
            continue
        # Word token.
        if _is_word_char(c):
            j = i
            while j < n and _is_word_char(source[j]):
                j += 1
            word = source[i:j]
            # Multi-word lookahead.
            k = j
            while k < n and source[k] in (" ", "\t"):
                k += 1
            second = None
            second_end = k
            if k < n and _is_word_char(source[k]):
                m = k
                while m < n and _is_word_char(source[m]):
                    m += 1
                second = source[k:m]
                second_end = m
            if second is not None:
                key = (word, second)
                if key in MULTI_WORD_ALIASES:
                    kind = MULTI_WORD_ALIASES[key]
                    if kind in ALIASES:
                        out.append(ALIASES[kind].get(target_lang, ALIASES[kind]["english"]))
                        i = second_end
                        continue
            if word in rev:
                kind, _ = rev[word]
                if kind in ALIASES:
                    out.append(ALIASES[kind].get(target_lang, ALIASES[kind]["english"]))
                    i = j
                    continue
            out.append(word)
            i = j
            continue
        out.append(c)
        i += 1
    return "".join(out)


def translate(source: str, target_lang: str, src_lang: Optional[str] = None) -> str:
    """
    Translate source to target_lang.

    Steps:
      1. Detect source language (from pragma or argument).
      2. If source is SOV, normalize verb-final statements to SVO.
      3. Substitute keywords to target_lang spellings.
      4. If target is SOV, convert SVO verb-initial statements to SOV.
      5. Rewrite pragma.
    """
    effective_src = src_lang or detect_pragma_lang(source) or "english"
    text = _normalize_sov_to_svo(source, effective_src)
    text = _translate_keywords(text, target_lang)
    text = _convert_svo_to_sov(text, target_lang)
    return text


def verify_roundtrip(source: str, target_lang: str, src_lang: Optional[str]) -> Tuple[bool, str]:
    """
    Translate source → target_lang → src_lang, then compare the
    keyword-token sequences of the original and the double-translated
    result. Returns (passed, message).
    """
    effective_src = src_lang or detect_pragma_lang(source) or "english"
    intermediate = translate(source, target_lang, effective_src)
    back = translate(intermediate, effective_src, target_lang)
    orig_tokens = extract_keyword_tokens(source)
    back_tokens = extract_keyword_tokens(back)
    if orig_tokens == back_tokens:
        return True, (
            f"round-trip ok: {effective_src} -> {target_lang} -> {effective_src} "
            f"({len(orig_tokens)} keyword tokens preserved)"
        )
    diffs = [
        f"  pos {i}: {a!r} -> {b!r}"
        for i, (a, b) in enumerate(zip(orig_tokens, back_tokens))
        if a != b
    ]
    if len(orig_tokens) != len(back_tokens):
        diffs.append(
            f"  token count: {len(orig_tokens)} original vs {len(back_tokens)} after round-trip"
        )
    return False, "round-trip FAILED:\n" + "\n".join(diffs)


def list_keywords() -> str:
    """Return a markdown table of all keyword aliases."""
    langs = ["english", "sanskrit", "hindi", "marathi", "mandarin"]
    header = "| TokenKind | " + " | ".join(l.capitalize() for l in langs) + " |"
    sep    = "|-----------|" + "|".join("-" * (len(l) + 2) for l in langs) + "|"
    rows = [header, sep]
    for kind, mapping in sorted(ALIASES.items()):
        cells = " | ".join(mapping.get(l, "--") for l in langs)
        rows.append(f"| {kind:<12} | {cells} |")
    return "\n".join(rows)


# ---------------------------------------------------------------------------
# LLM translation for comments, strings, and identifiers
# ---------------------------------------------------------------------------

_LANG_NAMES = {
    "english":  "English",
    "sanskrit": "Sanskrit",
    "hindi":    "Hindi",
    "marathi":  "Marathi",
    "mandarin": "Mandarin Chinese",
}


def _llm_prompt(text: str, src_lang: str, target_lang: str, content_type: str) -> str:
    src_name = _LANG_NAMES.get(src_lang, src_lang)
    tgt_name = _LANG_NAMES.get(target_lang, target_lang)
    if content_type == "comment text":
        # Explicit framing prevents models from generating code instead of translating.
        return (
            f"Translate this source code comment from {src_name} to {tgt_name}.\n"
            f"The input is natural language text written as a comment inside a computer program.\n"
            f"Output ONLY the translated natural language sentence -- no code, no quotes, "
            f"no surrounding punctuation.\n"
            f"Keep any technical terms, variable names, numbers, and identifiers unchanged.\n\n"
            f"Comment: {text.strip()}"
        )
    return (
        f"Translate the following {content_type} from {src_name} to {tgt_name}.\n"
        f"Rules:\n"
        f"- Translate only the natural language content.\n"
        f"- Preserve all technical terms, variable names, code references, "
        f"and special characters exactly as-is.\n"
        f"- Output ONLY the translated text, nothing else.\n\n"
        f"Text:\n{text}"
    )


def _call_anthropic(text: str, src_lang: str, target_lang: str,
                    content_type: str, model: str) -> str:
    try:
        import anthropic as _anthropic
    except ImportError:
        raise RuntimeError(
            "anthropic package not installed. Run: pip install 'anthropic>=0.20'"
        )
    prompt = _llm_prompt(text, src_lang, target_lang, content_type)

    # Modern SDK (v0.20+): has Anthropic class with messages API.
    if hasattr(_anthropic, "Anthropic"):
        client = _anthropic.Anthropic()
        msg = client.messages.create(
            model=model,
            max_tokens=4096,
            messages=[{"role": "user", "content": prompt}],
        )
        return msg.content[0].text.strip()

    # Legacy SDK (v0.2.x): uses Client + completion() + HUMAN_PROMPT sentinel.
    if hasattr(_anthropic, "Client") and hasattr(_anthropic, "HUMAN_PROMPT"):
        import os
        api_key = os.environ.get("ANTHROPIC_API_KEY")
        if not api_key:
            raise RuntimeError(
                "ANTHROPIC_API_KEY environment variable not set. "
                "Set it or upgrade: pip install 'anthropic>=0.20'"
            )
        client = _anthropic.Client(api_key=api_key)
        full_prompt = (
            f"{_anthropic.HUMAN_PROMPT} {prompt}{_anthropic.AI_PROMPT}"
        )
        resp = client.completion(
            prompt=full_prompt,
            model=model if model.startswith("claude-v") else "claude-v1",
            max_tokens_to_sample=4096,
        )
        return resp["completion"].strip()

    raise RuntimeError(
        "Unrecognized anthropic package. Run: pip install 'anthropic>=0.20'"
    )


def _call_openai(text: str, src_lang: str, target_lang: str,
                 content_type: str, model: str) -> str:
    try:
        import openai as _openai
    except ImportError:
        raise RuntimeError(
            "openai package not installed. Run: pip install openai"
        )
    client = _openai.OpenAI()
    resp = client.chat.completions.create(
        model=model,
        messages=[{"role": "user", "content": _llm_prompt(text, src_lang, target_lang, content_type)}],
    )
    return resp.choices[0].message.content.strip()


def _call_ollama(text: str, src_lang: str, target_lang: str,
                 content_type: str, model: str,
                 host: str = "http://localhost:11434",
                 timeout: int = 60) -> str:
    try:
        import requests as _requests
    except ImportError:
        raise RuntimeError(
            "requests package not installed. Run: pip install requests"
        )
    payload = {
        "model": model,
        "prompt": _llm_prompt(text, src_lang, target_lang, content_type),
        "stream": False,
    }
    resp = _requests.post(f"{host}/api/generate", json=payload, timeout=timeout)
    resp.raise_for_status()
    return resp.json()["response"].strip()


def _llm_translate_chunk(text: str, src_lang: str, target_lang: str,
                          content_type: str,
                          llm: str, model: str,
                          ollama_host: str = "http://localhost:11434",
                          llm_timeout: int = 60) -> str:
    """Call the chosen LLM backend to translate a natural language chunk."""
    if not text.strip():
        return text
    if llm == "anthropic":
        return _call_anthropic(text, src_lang, target_lang, content_type, model)
    if llm == "openai":
        return _call_openai(text, src_lang, target_lang, content_type, model)
    if llm == "ollama":
        return _call_ollama(text, src_lang, target_lang, content_type, model,
                            ollama_host, llm_timeout)
    raise ValueError(f"unknown LLM backend: {llm!r}")


def _split_identifier(ident: str) -> List[str]:
    """
    Split a camelCase or snake_case identifier into constituent words.
    E.g. 'safeDiv' -> ['safe', 'Div'], 'safe_div' -> ['safe', 'div'].
    """
    # Split on underscores first
    parts = ident.split("_")
    words: List[str] = []
    for part in parts:
        if not part:
            continue
        # Split camelCase
        sub = re.sub(r"([A-Z]+)([A-Z][a-z])", r"\1_\2", part)
        sub = re.sub(r"([a-z\d])([A-Z])", r"\1_\2", sub)
        words.extend(sub.split("_"))
    return words


def translate_with_llm(
    source: str,
    target_lang: str,
    src_lang: Optional[str],
    llm: str,
    model: str,
    translate_identifiers: bool = False,
    ollama_host: str = "http://localhost:11434",
    llm_timeout: int = 60,
) -> str:
    """
    Translate a vani source file using both keyword substitution and LLM
    translation for natural language content.

    Translates:
    - Keywords: via the keyword substitution table (always).
    - Line comments (// ...): via LLM.
    - String literals ("..."): via LLM.
    - Identifiers (user-defined names): via LLM when translate_identifiers=True.
    """
    effective_src = src_lang or detect_pragma_lang(source) or "english"

    # Step 1: keyword translation (with SOV reordering).
    result = translate(source, target_lang, effective_src)

    # Step 2: translate line comments.
    def _translate_comment(m: re.Match) -> str:
        prefix = m.group(1)   # // or //
        content = m.group(2)  # the text after //

        # Skip pragma lines.
        stripped = content.strip()
        if stripped.startswith("vani-lang:") or stripped.startswith("श्री।"):
            return m.group(0)

        try:
            translated = _llm_translate_chunk(
                content, effective_src, target_lang, "comment text",
                llm, model, ollama_host, llm_timeout
            )
            return prefix + " " + translated.lstrip()
        except Exception as e:
            print(f"  [llm] comment translation failed: {e}", file=sys.stderr)
            return m.group(0)

    result = re.sub(r"(//)(.*)", _translate_comment, result)

    # Step 3: translate string literals.
    def _translate_string(m: re.Match) -> str:
        content = m.group(1)  # content inside quotes
        try:
            translated = _llm_translate_chunk(
                content, effective_src, target_lang, "string literal",
                llm, model, ollama_host, llm_timeout
            )
            # Ensure no unescaped quotes sneak in.
            translated = translated.replace('"', '\\"')
            return f'"{translated}"'
        except Exception as e:
            print(f"  [llm] string translation failed: {e}", file=sys.stderr)
            return m.group(0)

    # Match string literals but not escaped quotes inside them.
    result = re.sub(r'"((?:[^"\\]|\\.)*)"', _translate_string, result)

    # Step 4: translate identifiers (optional).
    if translate_identifiers:
        rev = build_reverse_lookup()
        # Collect all unique user-defined identifiers (not keywords, not all-caps consts).
        idents = set(re.findall(r'\b([a-zA-Z_][a-zA-Z0-9_]*)\b', result))
        # Filter out keywords and trivial names.
        idents = {
            w for w in idents
            if w not in rev
            and w not in ALIASES
            and len(w) >= 3
            and not w.isupper()
        }

        ident_map: Dict[str, str] = {}
        if idents:
            # Batch all identifiers into one LLM call to save API round-trips.
            batch = "\n".join(sorted(idents))
            try:
                translated_batch = _llm_translate_chunk(
                    batch, effective_src, target_lang,
                    "list of programming identifiers (one per line -- translate each separately, preserve the same line count)",
                    llm, model, ollama_host, llm_timeout
                )
                translated_lines = translated_batch.splitlines()
                sorted_idents = sorted(idents)
                for orig, xlat in zip(sorted_idents, translated_lines):
                    # Sanitize: identifiers must be word-chars only.
                    clean = re.sub(r"[^\w]", "_", xlat.strip())
                    if clean and clean != orig:
                        ident_map[orig] = clean
            except Exception as e:
                print(f"  [llm] identifier translation failed: {e}", file=sys.stderr)

        if ident_map:
            def _replace_ident(m: re.Match) -> str:
                return ident_map.get(m.group(0), m.group(0))
            # Sort by length desc so longer identifiers match before shorter subsets.
            pattern = r'\b(' + "|".join(
                re.escape(k) for k in sorted(ident_map, key=len, reverse=True)
            ) + r')\b'
            result = re.sub(pattern, _replace_ident, result)

    return result


# ---------------------------------------------------------------------------
# File-level translation
# ---------------------------------------------------------------------------

def _translate_file(
    src_path: Path,
    target_lang: str,
    out_path: Optional[Path],
    inplace: bool,
    add_sri_header: bool,
    verify: bool,
    src_lang: Optional[str],
    verbose: bool,
    llm: Optional[str] = None,
    llm_model: str = "claude-haiku-4-5-20251001",
    translate_identifiers: bool = False,
    ollama_host: str = "http://localhost:11434",
    llm_timeout: int = 60,
) -> bool:
    source = src_path.read_text(encoding="utf-8")
    if verify:
        ok, msg = verify_roundtrip(source, target_lang, src_lang)
        prefix = src_path.name + ": " if verbose else ""
        print(f"{prefix}{msg}", file=sys.stderr if not ok else sys.stdout)
        if not ok:
            return False

    if llm:
        translated = translate_with_llm(
            source, target_lang, src_lang, llm, llm_model,
            translate_identifiers, ollama_host, llm_timeout
        )
    else:
        translated = translate(source, target_lang, src_lang)

    if add_sri_header and target_lang in _IA_DEVANAGARI:
        if not translated.lstrip().startswith("// श्री।"):
            translated = (
                f"// श्री।\n"
                f"// vani-lang: {target_lang}\n"
                f"//\n"
                + translated
            )

    if inplace:
        backup = src_path.with_suffix(src_path.suffix + ".bak")
        backup.write_text(source, encoding="utf-8")
        src_path.write_text(translated, encoding="utf-8")
        if verbose:
            print(f"  {src_path}  (backup -> {backup.name})")
    elif out_path is not None:
        if out_path.is_dir():
            dest = out_path / src_path.name
        else:
            dest = out_path
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_text(translated, encoding="utf-8")
        if verbose:
            print(f"  {src_path} -> {dest}")
    else:
        sys.stdout.write(translated)
    return True


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def main() -> int:
    parser = argparse.ArgumentParser(
        description=(
            "Translate a .vani source file's keywords between "
            "English, Sanskrit, Hindi, Marathi, and Mandarin. "
            "SOV word-order (verb-final statements and Hindi for-range) "
            "is reordered automatically."
        )
    )
    parser.add_argument(
        "input",
        type=Path,
        nargs="?",
        help="source .vani file or directory (with --batch)",
    )
    parser.add_argument(
        "--from",
        dest="src_lang",
        choices=SUPPORTED_LANGS,
        default=None,
        help=(
            "source language -- optional; auto-detected from the "
            "`// vani-lang:` pragma if not provided"
        ),
    )
    parser.add_argument(
        "--to",
        dest="target_lang",
        choices=SUPPORTED_LANGS,
        default=None,
        help="target language (required unless --list-keywords)",
    )
    parser.add_argument(
        "-o", "--output",
        type=Path,
        default=None,
        help="output file or directory (default: stdout; directory used with --batch)",
    )
    parser.add_argument(
        "--inplace", "-i",
        action="store_true",
        help="translate file in-place, saving original as <file>.bak",
    )
    parser.add_argument(
        "--batch",
        action="store_true",
        help="translate all .vani files under INPUT directory tree",
    )
    parser.add_argument(
        "--verify",
        action="store_true",
        help=(
            "after translation, translate back to source language and "
            "verify the keyword token sequence is preserved"
        ),
    )
    parser.add_argument(
        "--list-keywords",
        action="store_true",
        help="print all keyword aliases as a markdown table and exit",
    )
    parser.add_argument(
        "--add-sri-header",
        action="store_true",
        help=(
            "prepend `// श्री।` and `// vani-lang: <lang>` when "
            "translating to an Indo-Aryan Devanagari language"
        ),
    )

    # LLM options
    llm_group = parser.add_argument_group("LLM translation (comments, strings, identifiers)")
    llm_group.add_argument(
        "--llm",
        choices=("anthropic", "openai", "ollama"),
        default=None,
        metavar="BACKEND",
        help=(
            "Enable LLM translation for comments and string literals. "
            "Choices: anthropic, openai, ollama. "
            "Requires the corresponding Python package and API credentials."
        ),
    )
    llm_group.add_argument(
        "--llm-model",
        default=None,
        metavar="MODEL",
        help=(
            "Model name to use with --llm. "
            "Defaults: anthropic=claude-haiku-4-5-20251001, "
            "openai=gpt-4o-mini, ollama=llama3.2"
        ),
    )
    llm_group.add_argument(
        "--translate-identifiers",
        action="store_true",
        help=(
            "Also translate user-defined identifiers via LLM (requires --llm). "
            "All unique identifiers are batched into one API call."
        ),
    )
    llm_group.add_argument(
        "--ollama-host",
        default="http://localhost:11434",
        metavar="URL",
        help="Ollama server URL (default: http://localhost:11434)",
    )
    llm_group.add_argument(
        "--llm-timeout",
        type=int,
        default=60,
        metavar="SECONDS",
        help=(
            "HTTP timeout for LLM requests in seconds (default: 60). "
            "Increase for slow CPU-only Ollama models."
        ),
    )

    parser.add_argument(
        "--verbose", "-v",
        action="store_true",
        help="print per-file progress to stderr",
    )
    args = parser.parse_args()

    if args.list_keywords:
        print(list_keywords())
        return 0

    if args.input is None:
        parser.error("INPUT is required unless --list-keywords is set")
    if args.target_lang is None:
        parser.error("--to is required")
    if args.inplace and args.output is not None:
        parser.error("--inplace and --output are mutually exclusive")
    if args.translate_identifiers and not args.llm:
        parser.error("--translate-identifiers requires --llm")

    # Default model per backend
    llm_model = args.llm_model
    if args.llm and not llm_model:
        llm_model = {
            "anthropic": "claude-haiku-4-5-20251001",
            "openai":    "gpt-4o-mini",
            "ollama":    "llama3.2",
        }[args.llm]

    common = dict(
        target_lang=args.target_lang,
        out_path=args.output,
        inplace=args.inplace,
        add_sri_header=args.add_sri_header,
        verify=args.verify,
        src_lang=args.src_lang,
        verbose=args.verbose,
        llm=args.llm,
        llm_model=llm_model,
        translate_identifiers=args.translate_identifiers,
        ollama_host=args.ollama_host,
        llm_timeout=args.llm_timeout,
    )

    if args.batch:
        if not args.input.is_dir():
            parser.error("--batch requires INPUT to be a directory")
        files = list(args.input.rglob("*.vani"))
        if not files:
            print(f"no .vani files found under {args.input}", file=sys.stderr)
            return 1
        ok_count = 0
        for f in sorted(files):
            ok = _translate_file(f, **common)
            if ok:
                ok_count += 1
        print(f"{ok_count}/{len(files)} files translated successfully.", file=sys.stderr)
        return 0 if ok_count == len(files) else 1
    else:
        if not args.input.exists():
            print(f"input file not found: {args.input}", file=sys.stderr)
            return 1
        ok = _translate_file(args.input, **common)
        return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
