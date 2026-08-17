#!/usr/bin/env python3
"""
vani_translate — translate a .vani source file's keywords between any
                of the 63 dialects the compiler itself supports (see
                SUPPORTED_LANGS below for the full list -- English plus
                62 others spanning Indo-Aryan, Dravidian, CJK, Southeast
                Asian, Middle Eastern/RTL, Cyrillic, European, Caucasian,
                and African language families).

B.1 v3 — adds SOV <-> SVO word-order reordering for verb-final
statements and Hindi for-range loops; adds optional LLM-based
translation of comments, string literals, and identifiers.

Keyword data (ALIASES, ALL_SYNONYMS) is generated from src/lexer.rs by
tools/regen_vani_translate_keywords.py -- run that after any lexer.rs
keyword-table edit instead of hand-editing this file's tables directly,
the same way tools/regen_lsp_keywords.py keeps src/lsp.rs in sync (see
that script's own docstring for why: a hand-maintained duplicate table
drifts from the real one silently otherwise).

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
        "nepali": "फलन",
        "maithili": "फलन",
        "konkani": "फलन",
        "assamese": "ফাংশন",
        "sindhi": "فنکشن",
        "punjabi_shahmukhi": "فنکشن",
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
        "nepali": "माना",
        "maithili": "माना",
        "konkani": "माना",
        "assamese": "মান",
        "sindhi": "مانیں",
        "punjabi_shahmukhi": "مانیں",
        "mongolian": "ᠶᠠᠪᠤᠭᠤᠯ",
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
        "nepali": "संरचना",
        "maithili": "संरचना",
        "konkani": "संरचना",
        "assamese": "গঠন",
        "sindhi": "ساخت",
        "punjabi_shahmukhi": "ساخت",
        "yoruba": "ọ̀nà",
        "tibetan": "སྒྲིག་གཞི",
        "mongolian": "ᠪᠦᠳᠦᠭᠴᠡ",
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
        "nepali": "गणन",
        "maithili": "गणन",
        "konkani": "गणन",
        "assamese": "গণনা",
        "sindhi": "شمار",
        "punjabi_shahmukhi": "شمار",
        "yoruba": "àkọsílẹ̀",
        "cherokee": "ᎢᎦᏙᎯ",
        "mongolian": "ᠲᠣᠭᠠᠯᠠᠯ",
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
        "nepali": "स्थिर",
        "maithili": "स्थिर",
        "konkani": "स्थिर",
        "assamese": "স্থির",
        "sindhi": "ثابت",
        "punjabi_shahmukhi": "ثابت",
        "yoruba": "àlàfo",
        "cherokee": "ᎠᏢᏓᏅᎯ",
        "mongolian": "ᠲᠣᠭᠲᠠᠮᠠᠯ",
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
        "polish": "typ",             "turkish": "tip",          "swedish": "typ",
        "norwegian": "type",         "danish": "type",          "hungarian": "típus",
        "czech": "typ",              "slovak": "typ",           "finnish": "tyyppi",
        "romanian": "tip",           "catalan": "tipus",
        "armenian": "տեսակ",         "georgian": "ტიპი",
        "swahili": "aina",           "hausa": "nau'i",
        "amharic": "አይነት",
        "tibetan": "རིགས",
        "malay": "jenis",            "indonesian": "tipe",     "filipino": "uri",
        "nepali": "प्रकार",
        "maithili": "प्रकार",
        "konkani": "प्रकार",
        "assamese": "প্রকার",
        "sindhi": "قسم",
        "punjabi_shahmukhi": "قسم",
        "yoruba": "irú",
        "cherokee": "ᎢᏳᏓᎴᎩ",
        "mongolian": "ᠬᠡᠯᠪᠡᠷᠢ",
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
        "nepali": "बाह्य",
        "maithili": "बाह्य",
        "konkani": "बाह्य",
        "assamese": "বাহ্যিক",
        "sindhi": "بیرونی",
        "punjabi_shahmukhi": "بیرونی",
        "tamil": "வெளி",
        "telugu": "బాహ్య",
        "gujarati": "બાહ્ય",
        "punjabi": "ਬਾਹਰੀ",
        "kannada": "ಬಾಹ್ಯ",
        "malayalam": "ബാഹ്യം",
        "sinhala": "බාහිර",
        "pashto": "بهرنی",
        "swahili": "nje",
        "filipino": "panlabas",
        "yoruba": "ìta",
        "hausa": "waje",
        "khmer": "ខាងក្រៅ",
        "burmese": "အပြင်",
        "cherokee": "ᏙᏱᏗᏢ",
        "lao": "ພາຍນອກ",
        "mongolian": "ᠭᠠᠳᠠᠨ᠎ᠠ",
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
        "swahili": "lengo",         "yoruba": "ìpinnu",        "hausa": "nufin",
        "amharic": "ዓላማ",
        "tibetan": "དམིགས་ཡུལ",
        "mongolian": "ᠵᠣᠷᠢᠯᠭ᠎ᠠ",
        "malay": "tujuan",             "indonesian": "tujuan",    "filipino": "layunin",
        "nepali": "उद्देश्य",
        "maithili": "उद्देश्य",
        "konkani": "उद्देश्य",
        "assamese": "উদ্দেশ্য",
        "sindhi": "مقصد",
        "punjabi_shahmukhi": "مقصد",
        "pashto": "موخه",
        "cherokee": "ᎤᎲᏍᏛ",
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
        "malay": "tetap",         "indonesian": "invarian",
        "nepali": "अपरिवर्तनीय",
        "maithili": "अपरिवर्तनीय",
        "konkani": "अपरिवर्तनीय",
        "assamese": "অপরিবর্তনীয়",
        "urdu": "غیرمتغیر",
        "persian": "تغییرناپذیر",
        "pashto": "دايمي",
        "swahili": "isiyobadilika",
        "filipino": "walangpalit",
        "yoruba": "àìyípadà",
        "hausa": "a_canzawa",
        "khmer": "មិនប្រែប្រួល",
        "burmese": "မပြောင်းလဲ",
        "tibetan": "མི་འགྱུར",
        "cherokee": "ᏂᎦᎳᏛᎾ",
        "lao": "ບໍ່ປ່ຽນ",
        "mongolian": "ᠲᠣᠭᠲᠠᠪᠤᠷᠢᠲᠠᠢ",
        "sindhi": "غیرمتغیر",
        "punjabi_shahmukhi": "غیرمتغیر",
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
        "nepali": "सार्वजनिक",
        "maithili": "सार्वजनिक",
        "konkani": "सार्वजनिक",
        "assamese": "সর্বজনীন",
        "sindhi": "عوامی",
        "punjabi_shahmukhi": "عوامی",
        "yoruba": "gbangba",
        "cherokee": "ᏂᎦᏓ",
        "mongolian": "ᠨᠡᠶᠢᠲᠡ",
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
        "nepali": "मॉड्यूल",
        "maithili": "मॉड्यूल",
        "konkani": "मॉड्यूल",
        "assamese": "খণ্ড",
        "sindhi": "ماڈیول",
        "punjabi_shahmukhi": "ماڈیول",
        "yoruba": "ìṣù",
        "cherokee": "ᎠᏯᏙᎸ",
        "mongolian": "ᠨᠢᠭᠡᠴᠡ",
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
        "nepali": "उपयोग",
        "maithili": "उपयोग",
        "konkani": "उपयोग",
        "assamese": "ব্যবহার",
        "sindhi": "استعمال",
        "punjabi_shahmukhi": "استعمال",
        "yoruba": "lò",
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
        "arabic": "بصفة",           "persian": "بعنوان",
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
        "nepali": "यथा",
        "maithili": "यथा",
        "konkani": "यथा",
        "assamese": "হিসাবে",
        "sindhi": "بطور",
        "punjabi_shahmukhi": "بطور",
        "pashto": "لکه",
        "hebrew": "בתור",
        "yoruba": "bí_ti",
        "amharic": "እንደ",
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
        "nepali": "लौटाओ",
        "maithili": "लौटाओ",
        "konkani": "लौटाओ",
        "assamese": "ফেরত",
        "sindhi": "واپس",
        "punjabi_shahmukhi": "واپس",
        "yoruba": "padà",
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
        "nepali": "अगर",
        "maithili": "अगर",
        "konkani": "अगर",
        "assamese": "যদি",
        "sindhi": "اگر",
        "punjabi_shahmukhi": "اگر",
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
        "nepali": "वरना",
        "maithili": "वरना",
        "konkani": "वरना",
        "assamese": "নাহলে",
        "sindhi": "ورنہ",
        "punjabi_shahmukhi": "ورنہ",
        "pashto": "کهنه",
        "yoruba": "yàtọ̀",
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
        "spanish": "mientras",       "french": "tantque",        "german": "während",
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
        "nepali": "जबतक",
        "maithili": "जबतक",
        "konkani": "जबतक",
        "assamese": "যতক্ষণ",
        "urdu": "دوران",
        "pashto": "ترڅو",
        "yoruba": "nígbà",
        "cherokee": "ᏰᎵᏊ",
        "sindhi": "دوران",
        "punjabi_shahmukhi": "دوران",
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
        "arabic": "لكل",             "hebrew": "לכל",           "persian": "هر",
        "urdu": "ہر",                "pashto": "هریو",
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
        "nepali": "के लिए",
        "maithili": "के लिए",
        "konkani": "के लिए",
        "assamese": "প্রতি",
        "sindhi": "ہر",
        "punjabi_shahmukhi": "ہر",
    },
    "In": {
        "english": "in",
        "sanskrit": "में",           "hindi": "में",             "marathi": "में",
        "bengali": "মধ্যে",          "odia": "ରେ",
        "tamil": "உள்",              "telugu": "లో",            "kannada": "ರಲ್ಲಿ",
        "malayalam": "ഇൽ",
        "gujarati": "માં",           "punjabi": "ਵਿੱਚ",
        "sinhala": "තුළ",
        "mandarin": "在",            "japanese": "中",           "korean": "안에",
        "thai": "ใน",                "khmer": "ក្នុង",
        "burmese": "ထဲမှာ",          "lao": "ໃນ",
        "arabic": "في",              "hebrew": "בתוך",           "persian": "در",
        "urdu": "میں",
        "russian": "в",
        "greek": "σε",
        "spanish": "en",             "french": "dans",          "german": "in",
        "portuguese": "em",          "dutch": "in",
        "polish": "w",               "turkish": "içinde",       "swedish": "inuti",
        "finnish": "sisalla",
        "armenian": "մեջ",           "georgian": "ში",
        "swahili": "ndani",          "hausa": "cikin",
        "amharic": "ውስጥ",
        "tibetan": "ནང",
        "cherokee": "ᎭᏫᎾ",
        "malay": "dalam",            "indonesian": "dalam",     "filipino": "sa",
        "nepali": "में",
        "maithili": "में",
        "konkani": "में",
        "assamese": "মধ্যে",
        "sindhi": "میں",
        "punjabi_shahmukhi": "میں",
        "pashto": "په",
        "italian": "in",
        "vietnamese": "trong",
        "romanian": "în",
        "hungarian": "belül",
        "czech": "uvnitř",
        "slovak": "vnútri",
        "norwegian": "inni",
        "danish": "indeni",
        "catalan": "en",
        "yoruba": "nínú",
        "mongolian": "ᠳᠣᠲᠣᠷ᠎ᠠ",
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
        "romanian": "din",         "catalan": "des",
        "armenian": "ից",            "georgian": "დან",
        "swahili": "kutoka",         "yoruba": "láti",          "hausa": "daga",
        "amharic": "ጀምሮ",
        "tibetan": "ནས",
        "mongolian": "ᠠᠴᠠ",
        "malay": "dari",             "indonesian": "dari",      "filipino": "mula",
        "nepali": "से",
        "maithili": "से",
        "konkani": "से",
        "assamese": "থেকে",
        "urdu": "سے",
        "pashto": "له",
        "hungarian": "kezdve",
        "cherokee": "ᏓᏓᎴᏂᏍᎬ",
        "sindhi": "سے",
        "punjabi_shahmukhi": "سے",
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
        "arabic": "إلى",             "hebrew": "עד",             "persian": "به",
        "urdu": "تک",
        "russian": "до",
        "greek": "μέχρι",
        "spanish": "hasta",          "french": "vers",          "german": "bis",
        "portuguese": "ate",         "italian": "finoa",         "dutch": "tot",
        "polish": "do",              "turkish": "kadar",        "swedish": "till",
        "norwegian": "til",          "danish": "til",
        "czech": "do",               "slovak": "do",            "finnish": "asti",
        "romanian": "până",          "catalan": "fins",
        "armenian": "մինչև",         "georgian": "მდე",
        "swahili": "hadi",           "yoruba": "dé",            "hausa": "zuwa",
        "amharic": "ድረስ",
        "tibetan": "བར་དུ",
        "mongolian": "ᠬᠦᠷᠲᠡᠯᠡ",
        "malay": "hingga",           "indonesian": "sampai",    "filipino": "hanggang",
        "nepali": "तक",
        "maithili": "तक",
        "konkani": "तक",
        "assamese": "পর্যন্ত",
        "sindhi": "تک",
        "punjabi_shahmukhi": "تک",
        "pashto": "ته",
        "hungarian": "határig",
        "cherokee": "ᎬᏛ",
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
        "nepali": "रुको",
        "maithili": "रुको",
        "konkani": "रुको",
        "assamese": "বিরাম",
        "sindhi": "بند",
        "punjabi_shahmukhi": "بند",
        "pashto": "ودروه",
        "yoruba": "dáwọ́dúró",
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
        "nepali": "आगे",
        "maithili": "आगे",
        "konkani": "आगे",
        "assamese": "এগিয়ে",
        "sindhi": "جاری",
        "punjabi_shahmukhi": "جاری",
        "pashto": "دوام",
        "yoruba": "tẹ̀síwájú",
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
        "polish": "wtedy",           "turkish": "sonra",      "swedish": "så",
        "norwegian": "da",           "danish": "så",            "hungarian": "akkor",
        "czech": "pak",              "slovak": "potom",         "finnish": "sitten",
        "romanian": "atunci",        "catalan": "aleshores",
        "armenian": "ապա",           "georgian": "მაშინ",
        "swahili": "kisha",          "yoruba": "nígbànáà",      "hausa": "sannan",
        "amharic": "ከዚያ",
        "tibetan": "དེ་ནས",
        "mongolian": "ᠳᠠᠷᠠᠭ᠎ᠠ",
        "malay": "maka",             "indonesian": "maka",      "filipino": "saka",
        "nepali": "तो",
        "maithili": "तो",
        "konkani": "तो",
        "assamese": "তবে",
        "sindhi": "تب",
        "punjabi_shahmukhi": "تب",
        "pashto": "بیا",
        "cherokee": "ᎣᏂ",
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
        "nepali": "देखो",
        "maithili": "देखो",
        "konkani": "देखो",
        "assamese": "দেখ",
        "urdu": "دیکھیں",
        "pashto": "وګوره",
        "filipino": "tingnan",
        "yoruba": "wò",
        "amharic": "ይመልከት",
        "sindhi": "دیکھیں",
        "punjabi_shahmukhi": "دیکھیں",
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
        "nepali": "परिवर्तनीय",
        "maithili": "परिवर्तनीय",
        "konkani": "परिवर्तनीय",
        "assamese": "পরিবর্তনীয়",
        "sindhi": "بدلنا",
        "punjabi_shahmukhi": "بدلنا",
        "pashto": "بدلېدونکی",
        "filipino": "nababago",
        "yoruba": "àyípadà",
        "khmer": "អាចផ្លាស់ប្តូរ",
        "burmese": "ပြောင်းလဲနိုင်",
        "amharic": "ሊቀየር",
        "tibetan": "འགྱུར",
        "lao": "ປ່ຽນແປງໄດ້",
        "mongolian": "ᠬᠤᠪᠢᠷᠠᠮᠲᠠᠭᠠᠢ",
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
        "nepali": "मिलान",
        "maithili": "मिलान",
        "konkani": "मिलान",
        "assamese": "মেলে",
        "urdu": "ملان",
        "pashto": "سمون",
        "filipino": "tugmain",
        "yoruba": "bámu",
        "amharic": "ተዛመደ",
        "cherokee": "ᎠᏍᏓᏩᏛᏍᎩ",
        "mongolian": "ᠲᠣᠬᠢᠷᠠ",
        "sindhi": "ملان",
        "punjabi_shahmukhi": "ملان",
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
        "nepali": "सुनिश्चित",
        "maithili": "सुनिश्चित",
        "konkani": "सुनिश्चित",
        "assamese": "নিশ্চিত",
        "urdu": "یقینی",
        "pashto": "تایید",
        "cherokee": "ᎯᏍᏗᏎᏍᏗ",
        "sindhi": "یقینی",
        "punjabi_shahmukhi": "یقینی",
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
        "swahili": "thibitisha_kuwa",  "hausa": "nuna",
        "amharic": "አስረዳ",
        "tibetan": "བསྒྲུབས",
        "mongolian": "ᠨᠣᠲᠠᠯᠠ",
        "malay": "buktikan",         "indonesian": "buktikan",  "filipino": "ipakita",
        "nepali": "सिद्ध करो",
        "maithili": "सिद्ध करो",
        "konkani": "सिद्ध करो",
        "assamese": "প্রমাণ",
        "urdu": "ثبوت",
        "pashto": "ثبوت",
        "yoruba": "fihàn",
        "cherokee": "ᎠᎩᏠᏯᏍᏗ",
        "sindhi": "ثبوت",
        "punjabi_shahmukhi": "ثبوت",
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
        "polish": "wymaga",          "turkish": "gerek",   "swedish": "kräver",
        "norwegian": "krever",       "danish": "kræver",        "hungarian": "igényel",
        "czech": "vyžaduje",         "slovak": "vyžaduje",      "finnish": "vaatii",
        "romanian": "necesită",      "catalan": "requereix",
        "armenian": "պահանջում",     "georgian": "მოითხოვს",
        "swahili": "hitaji",         "hausa": "bukata",
        "amharic": "ይፈልጋል",
        "tibetan": "དགོས",
        "malay": "memerlukan",       "indonesian": "perlu",     "filipino": "kailangan",
        "nepali": "चाहिए",
        "maithili": "चाहिए",
        "konkani": "चाहिए",
        "assamese": "প্রয়োজনীয়",
        "urdu": "درکار",
        "pashto": "اړتیا",
        "yoruba": "nílò",
        "cherokee": "ᎠᏎᏗ",
        "mongolian": "ᠱᠠᠭᠠᠷᠳᠠ",
        "sindhi": "درکار",
        "punjabi_shahmukhi": "درکار",
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
        "polish": "gwarantuje",      "turkish": "garanti",       "swedish": "säkerställer",
        "norwegian": "garanterer",   "danish": "garanterer",    "hungarian": "garantál",
        "czech": "zajišťuje",        "slovak": "zaručuje",      "finnish": "takaa",
        "romanian": "garantează",    "catalan": "garanteix",
        "armenian": "երաշխավորում",  "georgian": "უზრუნველყოფს",
        "swahili": "hakikisha",      "hausa": "tabbace",
        "amharic": "ያረጋግጣል",
        "tibetan": "ཁག",
        "malay": "menjamin",         "indonesian": "jamin",     "filipino": "tiyakin",
        "nepali": "निश्चित",
        "maithili": "निश्चित",
        "konkani": "निश्चित",
        "assamese": "সুনিশ্চিত",
        "urdu": "ضمانت",
        "pashto": "ډاډ",
        "yoruba": "dájú",
        "cherokee": "ᎤᏙᎯᏳᏫᏍᏗ",
        "mongolian": "ᠪᠠᠲᠤᠯᠠᠭ᠎ᠠ",
        "sindhi": "ضمانت",
        "punjabi_shahmukhi": "ضمانت",
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
        "nepali": "सत्य",
        "maithili": "सत्य",
        "konkani": "सत्य",
        "assamese": "সত্য",
        "sindhi": "سچ",
        "punjabi_shahmukhi": "سچ",
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
        "nepali": "असत्य",
        "maithili": "असत्य",
        "konkani": "असत्य",
        "assamese": "অসত্য",
        "sindhi": "جھوٹ",
        "punjabi_shahmukhi": "جھوٹ",
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
        "nepali": "लिखो",
        "maithili": "लिखो",
        "konkani": "लिखो",
        "assamese": "লেখ",
        "sindhi": "لکھو",
        "punjabi_shahmukhi": "لکھو",
        "cherokee": "ᎠᎴᏂᏍᎬᎢ",
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
        "nepali": "शुद्ध",
        "maithili": "शुद्ध",
        "konkani": "शुद्ध",
        "bengali": "শুদ্ধ",
        "tamil": "தூய",
        "telugu": "శుద్ధ",
        "gujarati": "શુદ્ધ",
        "punjabi": "ਸ਼ੁੱਧ",
        "kannada": "ಶುದ್ಧ",
        "malayalam": "ശുദ്ധം",
        "odia": "ଶୁଦ୍ଧ",
        "sinhala": "ශුද්ධ",
        "urdu": "خالص",
        "persian": "خالص",
        "pashto": "خالص",
        "armenian": "մաքուր",
        "georgian": "სუფთა",
        "yoruba": "mímọ́",
        "hausa": "tsabta",
        "khmer": "បរិសុទ្ធ",
        "burmese": "သန့်ရှင်း",
        "cherokee": "ᎦᏅᎯᏛ",
        "lao": "ບໍລິສຸດ",
        "assamese": "শুদ্ধ",
        "sindhi": "خالص",
        "punjabi_shahmukhi": "خالص",
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
        "romanian": "paralel",       "catalan": "parallel",
        "swahili": "sambamba",
        "amharic": "ትይዩ",
        "malay": "selari",           "indonesian": "paralel",   "filipino": "magkatulad",
        "nepali": "समानांतर",
        "maithili": "समानांतर",
        "konkani": "समानांतर",
        "bengali": "সমান্তরাল",
        "tamil": "இணை",
        "telugu": "సమాంతర",
        "gujarati": "સમાંતર",
        "punjabi": "ਸਮਾਂਤਰ",
        "kannada": "ಸಮಾನಾಂತರ",
        "malayalam": "സമാന്തരം",
        "odia": "ସମାନ୍ତର",
        "sinhala": "සමාන්තර",
        "urdu": "متوازی",
        "persian": "موازی",
        "pashto": "موازي",
        "armenian": "զուգահեռ",
        "georgian": "პარალელური",
        "yoruba": "akáṣe",
        "hausa": "madaidaici",
        "khmer": "ស្របគ្នា",
        "burmese": "ပြိုင်တူ",
        "tibetan": "མཉམ",
        "cherokee": "ᎾᏍᎩᏯ",
        "lao": "ຂະໜານ",
        "mongolian": "ᠵᠡᠷᠭᠡ",
        "assamese": "সমান্তরাল",
        "sindhi": "متوازی",
        "punjabi_shahmukhi": "متوازی",
    },
    "Reduce": {
        "english": "reduce",
        "sanskrit": "संक्षेप",       "hindi": "संक्षेप",          "marathi": "संक्षेप",
        "mandarin": "减少",
        "nepali": "संक्षेप",
        "maithili": "संक्षेप",
        "konkani": "संक्षेप",
        "bengali": "সংক্ষেপ",
        "tamil": "குறை",
        "telugu": "సంక్షేప",
        "gujarati": "સંક્ષેપ",
        "punjabi": "ਸੰਖੇਪ",
        "kannada": "ಸಂಕ್ಷೇಪ",
        "malayalam": "സംക്ഷേപം",
        "odia": "ସଂକ୍ଷେପ",
        "sinhala": "සංක්ෂේප",
        "urdu": "تخفیف",
        "persian": "کاهش",
        "pashto": "کمښت",
        "russian": "сократить",
        "spanish": "reducir",
        "french": "reduire",
        "japanese": "削減",
        "korean": "축소",
        "german": "reduzieren",
        "portuguese": "reduzir",
        "indonesian": "kurangi",
        "greek": "μείωση",
        "hebrew": "הפחתה",
        "italian": "ridurre",
        "arabic": "تقليل",
        "polish": "zmniejsz",
        "turkish": "azalt",
        "malay": "kurangkan",
        "swahili": "punguza",
        "vietnamese": "giảm",
        "romanian": "reduce",
        "dutch": "verminder",
        "thai": "ลด",
        "hungarian": "csökkent",
        "czech": "zmenši",
        "slovak": "zmenši",
        "finnish": "vahenna",
        "swedish": "reducera",
        "filipino": "bawasan",
        "norwegian": "reduser",
        "danish": "reducer",
        "armenian": "կրճատում",
        "georgian": "შემცირება",
        "catalan": "redueix",
        "yoruba": "dínku",
        "hausa": "rage",
        "khmer": "កាត់បន្ថយ",
        "burmese": "လျှော့ချ",
        "amharic": "ቀንስ",
        "tibetan": "ཉུང་དུ",
        "cherokee": "ᎤᏍᏗᎪᏗ",
        "lao": "ຫຼຸດ",
        "mongolian": "ᠪᠠᠭᠠᠰᠬᠠ",
        "assamese": "সংক্ষেপ",
        "sindhi": "تخفیف",
        "punjabi_shahmukhi": "تخفیف",
    },
    "With": {
        "english": "with",
        "sanskrit": "सह",            "hindi": "सह",              "marathi": "सह",
        "mandarin": "与",
        "nepali": "सह",
        "maithili": "सह",
        "konkani": "सह",
        "bengali": "সহ",
        "tamil": "உடன்",
        "telugu": "తో",
        "gujarati": "સાથે",
        "punjabi": "ਨਾਲ",
        "kannada": "ಜೊತೆ",
        "malayalam": "കൂടെ",
        "odia": "ସହିତ",
        "sinhala": "සමඟ",
        "urdu": "ساتھ",
        "persian": "با",
        "pashto": "سره",
        "russian": "совместно",
        "spanish": "con",
        "french": "avec",
        "japanese": "と",
        "korean": "함께",
        "german": "mit",
        "portuguese": "com",
        "indonesian": "dengan",
        "greek": "με",
        "hebrew": "עם",
        "italian": "con",
        "arabic": "مع",
        "polish": "razem",
        "turkish": "ile",
        "malay": "dengan",
        "swahili": "na",
        "vietnamese": "với",
        "romanian": "cu",
        "dutch": "met",
        "thai": "กับ",
        "hungarian": "együtt",
        "czech": "spolu",
        "slovak": "spolu",
        "finnish": "kanssa",
        "swedish": "med",
        "filipino": "kasama",
        "norwegian": "med",
        "danish": "med",
        "armenian": "հետ",
        "georgian": "თან",
        "catalan": "amb",
        "yoruba": "pẹ̀lú",
        "hausa": "tare",
        "khmer": "ជាមួយ",
        "burmese": "နှင့်အတူ",
        "amharic": "ጋር",
        "tibetan": "དང",
        "cherokee": "ᎠᎴ",
        "lao": "ກັບ",
        "mongolian": "ᠬᠠᠮᠲᠤ",
        "assamese": "সহ",
        "sindhi": "ساتھ",
        "punjabi_shahmukhi": "ساتھ",
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
        "malay": "antaramuka",      "indonesian": "antarmuka",
        "nepali": "संकेत",
        "maithili": "संकेत",
        "konkani": "संकेत",
        "sindhi": "رابطہ",
        "punjabi_shahmukhi": "رابطہ",
        "bengali": "সংকেত",
        "tamil": "இடைமுகம்",
        "telugu": "సంకేతం",
        "gujarati": "સંકેત",
        "punjabi": "ਸੰਕੇਤ",
        "kannada": "ಸಂಕೇತ",
        "malayalam": "സങ്കേതം",
        "odia": "ସଙ୍କେତ",
        "sinhala": "සංකේතය",
        "pashto": "اړیکه",
        "filipino": "ugnayan",
        "yoruba": "ifaramọ",
        "hausa": "hannu",
        "khmer": "ចំណុចប្រទាក់",
        "burmese": "မျက်နှာပြင်",
        "tibetan": "འབྲེལ་མཐུད",
        "cherokee": "ᎠᏓᏛᏗ",
        "lao": "ສ່ວນຕິດຕໍ່",
        "mongolian": "ᠵᠠᠯᠭᠠᠭᠤᠷ",
        "assamese": "সংকেত",
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
        "amharic": "ተግባራዊ",
        "malay": "laksanakan",       "indonesian": "terapkan",  "filipino": "ipatupad",
        "nepali": "कार्यान्वित",
        "maithili": "कार्यान्वित",
        "konkani": "कार्यान्वित",
        "bengali": "কার্যান্বিত",
        "tamil": "செயல்படுத்து",
        "telugu": "అమలు",
        "gujarati": "અમલ",
        "punjabi": "ਅਮਲ",
        "kannada": "ಜಾರಿ",
        "malayalam": "നടപ്പിലാക്കുക",
        "odia": "ପ୍ରୟୋଗ",
        "sinhala": "ක්‍රියාත්මක",
        "urdu": "نافذ",
        "persian": "اجرا",
        "pashto": "پلي",
        "swahili": "tekeleza",
        "yoruba": "muṣẹ",
        "hausa": "aiwatar",
        "khmer": "អនុវត្ត",
        "burmese": "အကောင်အထည်ဖော်",
        "tibetan": "ལག་བསྟར",
        "cherokee": "ᎬᏔᏂᏙᎲ",
        "lao": "ປະຕິບັດ",
        "mongolian": "ᠬᠡᠷᠡᠭᠵᠢᠭᠦᠯ",
        "assamese": "কার্যান্বিত",
        "sindhi": "نافذ",
        "punjabi_shahmukhi": "نافذ",
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
        "polish": "metody",          "turkish": "metotlar",     "swedish": "metoder",
        "norwegian": "metoder",      "danish": "metoder",       "hungarian": "metódusok",
        "czech": "metody",           "slovak": "metody",        "finnish": "menetelmat",
        "romanian": "metode",        "catalan": "metodes",
        "armenian": "մեթոդներ",      "georgian": "მეთოდები",
        "amharic": "ዘዴዎች",
        "malay": "kaedah",           "indonesian": "metode",    "filipino": "pamamaraan",
        "nepali": "विधि",
        "maithili": "विधि",
        "konkani": "विधि",
        "bengali": "বিধি",
        "tamil": "முறைகள்",
        "telugu": "పద్ధతులు",
        "gujarati": "પદ્ધતિઓ",
        "punjabi": "ਢੰਗ",
        "kannada": "ವಿಧಾನಗಳು",
        "malayalam": "രീതികൾ",
        "odia": "ପଦ୍ଧତି",
        "sinhala": "ක්‍රම",
        "urdu": "طریقے",
        "persian": "روش",
        "pashto": "طریقې",
        "swahili": "njia",
        "yoruba": "ipa",
        "hausa": "hanyoyi",
        "khmer": "វិធីសាស្ត្រ",
        "burmese": "နည်းလမ်း",
        "tibetan": "ཐབས་ལམ",
        "cherokee": "ᏗᏄᎪᏗ",
        "lao": "ວິທີການ",
        "mongolian": "ᠠᠷᠭ᠎ᠠ",
        "assamese": "বিধি",
        "sindhi": "طریقے",
        "punjabi_shahmukhi": "طریقے",
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
        "malay": "tempat",          "indonesian": "dimana",    "filipino": "saan",
        "nepali": "जहाँ",
        "maithili": "जहाँ",
        "konkani": "जहाँ",
        "bengali": "যেখানে",
        "tamil": "எங்கே",
        "telugu": "ఎక్కడ",
        "gujarati": "જ્યાં",
        "punjabi": "ਜਿੱਥੇ",
        "kannada": "ಎಲ್ಲಿ",
        "malayalam": "എവിടെ",
        "odia": "କେଉଁଠାରେ",
        "sinhala": "කොහෙද",
        "urdu": "جہاں",
        "persian": "کجا",
        "pashto": "چیرته",
        "yoruba": "ibo",
        "khmer": "ណា",
        "burmese": "ဘယ်မှာ",
        "tibetan": "གང",
        "cherokee": "ᎭᏢ",
        "lao": "ບ່ອນທີ່",
        "mongolian": "ᠬᠠᠮᠢᠭ᠎ᠠ",
        "assamese": "যেখানে",
        "sindhi": "جہاں",
        "punjabi_shahmukhi": "جہاں",
    },
    "Is": {
        "english": "is",
        "sanskrit": "अस्ति",          "hindi": "है",              "marathi": "आहे",
        "mandarin": "是",             "japanese": "は",            "korean": "이다",
        "thai": "คือ",               "vietnamese": "là",
        "arabic": "هو",              "hebrew": "הוא",
        "russian": "есть",
        "greek": "είναι",
        "spanish": "es",             "french": "est",           "german": "ist",
        "portuguese": "eh",          "italian": "risulta",            "dutch": "is",
        "polish": "jest",            "turkish": "olur",          "swedish": "är",
        "norwegian": "er",           "danish": "er",            "hungarian": "van",
        "czech": "je",               "slovak": "je",            "finnish": "on",
        "romanian": "este",          "catalan": "es",
        "armenian": "է",             "georgian": "არის",
        "hausa": "ne",
        "amharic": "ነው",
        "malay": "adalah",           "indonesian": "adalah",    "filipino": "ay",
        "nepali": "है",
        "maithili": "है",
        "konkani": "है",
        "bengali": "হয়",
        "tamil": "ஆகும்",
        "telugu": "ఉంది",
        "gujarati": "છે",
        "punjabi": "ਹੈ",
        "kannada": "ಇದೆ",
        "malayalam": "ആണ്",
        "odia": "ଅଟେ",
        "sinhala": "වේ",
        "urdu": "ہے",
        "persian": "است",
        "pashto": "دی",
        "swahili": "ni",
        "yoruba": "ni",
        "khmer": "គឺ",
        "burmese": "ဖြစ်သည်",
        "tibetan": "ཡིན",
        "cherokee": "ᎨᏒ",
        "lao": "ແມ່ນ",
        "mongolian": "ᠪᠣᠯᠤᠨ᠎ᠠ",
        "assamese": "হয়",
        "sindhi": "ہے",
        "punjabi_shahmukhi": "ہے",
    },

    # ── Concurrency ──────────────────────────────────────────────────────────
    "Try": {
        "english": "try",
        "sanskrit": "प्रयास",         "hindi": "प्रयास",          "marathi": "प्रयास",
        "mandarin": "尝试",            "japanese": "試行",          "korean": "시도",
        "thai": "ลอง",               "vietnamese": "thử",
        "arabic": "حاول",            "hebrew": "נסה",
        "russian": "попробуй",
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
        "nepali": "प्रयास",
        "maithili": "प्रयास",
        "konkani": "प्रयास",
        "bengali": "চেষ্টা",
        "tamil": "முயற்சி",
        "telugu": "ప్రయత్నించు",
        "gujarati": "પ્રયાસ",
        "punjabi": "ਕੋਸ਼ਿਸ਼",
        "kannada": "ಪ್ರಯತ್ನ",
        "malayalam": "ശ്രമിക്കുക",
        "odia": "ପ୍ରୟାସ",
        "sinhala": "උත්සාහ",
        "urdu": "کوشش",
        "persian": "تلاش",
        "pashto": "هڅه",
        "yoruba": "gbiyanju",
        "khmer": "ព្យាយាម",
        "burmese": "ကြိုးစား",
        "tibetan": "འབད",
        "cherokee": "ᎠᏓᎫᏓᏛᏍᎩ",
        "lao": "ລອງ",
        "mongolian": "ᠣᠷᠣᠯᠳᠤ",
        "assamese": "চেষ্টা",
        "sindhi": "کوشش",
        "punjabi_shahmukhi": "کوشش",
    },
    "Task": {
        "english": "task",
        "sanskrit": "नियोग",          "hindi": "नियोग",           "marathi": "नियोग",
        "bengali": "নিয়োগ",
        "mandarin": "任务",            "japanese": "タスク",        "korean": "작업",
        "thai": "งาน",               "vietnamese": "công_việc",
        "arabic": "مهمة",            "hebrew": "משימה",           "persian": "وظیفه",
        "urdu": "ٹاسک",
        "russian": "задача",
        "greek": "εργασία",
        "spanish": "tarea",          "french": "tâche",         "german": "aufgabe",
        "portuguese": "tarefa",      "italian": "compito",      "dutch": "taak",
        "polish": "zadanie",         "turkish": "görev",        "swedish": "uppgift",
        "norwegian": "oppgave",      "danish": "opgave",        "hungarian": "feladat",
        "czech": "úloha",            "slovak": "úloha",         "finnish": "tehtava",
        "romanian": "sarcină",       "catalan": "tasca",
        "armenian": "խնդիր",         "georgian": "დავალება",
        "swahili": "jukumu",     "hausa": "hidima",
        "amharic": "ስራ",
        "mongolian": "ᠡᠭᠦᠷᠭᠡ",
        "malay": "tugasan",          "indonesian": "tugas",     "filipino": "tungkulin",
        "nepali": "नियोग",
        "maithili": "नियोग",
        "konkani": "नियोग",
        "assamese": "নিয়োগ",
        "sindhi": "ٹاسک",
        "punjabi_shahmukhi": "ٹاسک",
        "tamil": "பணி",
        "telugu": "కార్యం",
        "gujarati": "નિયોગ",
        "punjabi": "ਨਿਯੋਗ",
        "kannada": "ನಿಯೋಗ",
        "malayalam": "നിയോഗം",
        "odia": "ନିଯୋଗ",
        "sinhala": "නියෝගය",
        "pashto": "دنده",
        "yoruba": "ojúṣe",
        "khmer": "ភារកិច្ច",
        "burmese": "တာဝန်",
        "tibetan": "ལས",
        "cherokee": "ᏗᎦᎸᏫᏍᏓᏁᏗ",
        "lao": "ວຽກງານ",
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
        "malay": "gabung",        "indonesian": "gabungkan", "filipino": "pagsama",
        "nepali": "संयोजन",
        "maithili": "संयोजन",
        "konkani": "संयोजन",
        "bengali": "যোগ",
        "tamil": "சேர்",
        "telugu": "సంయోగం",
        "gujarati": "સંયોજન",
        "punjabi": "ਸੰਯੋਜਨ",
        "kannada": "ಸಂಯೋಜನೆ",
        "malayalam": "സംയോജനം",
        "odia": "ସଂଯୋଜନ",
        "sinhala": "සංයෝජනය",
        "urdu": "ملاپ",
        "persian": "پیوستن",
        "pashto": "نښلول",
        "yoruba": "darapọ",
        "khmer": "ភ្ជាប់",
        "burmese": "ပူးပေါင်း",
        "cherokee": "ᏓᏂᎳᏫᏍᎦ",
        "lao": "ເຊື່ອມ",
        "mongolian": "ᠨᠡᠶᠢᠯᠡ",
        "assamese": "যোগ",
        "sindhi": "ملاپ",
        "punjabi_shahmukhi": "ملاپ",
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
        "malay": "tidakselamat",    "indonesian": "bahaya",
        "nepali": "असुरक्षित",
        "maithili": "असुरक्षित",
        "konkani": "असुरक्षित",
        "bengali": "অসুরক্ষিত",
        "tamil": "பாதுகாப்பற்ற",
        "telugu": "అసురక్షిత",
        "gujarati": "અસુરક્ષિત",
        "punjabi": "ਅਸੁਰੱਖਿਅਤ",
        "kannada": "ಅಸುರಕ್ಷಿತ",
        "malayalam": "അസുരക്ഷിതം",
        "odia": "ଅସୁରକ୍ଷିତ",
        "sinhala": "අනාරක්ෂිත",
        "urdu": "غیرمحفوظ",
        "persian": "ناامن",
        "pashto": "ناامن",
        "swahili": "hatari",
        "filipino": "mapanganib",
        "yoruba": "àìláàbò",
        "hausa": "kasada",
        "khmer": "មិនមានសុវត្ថិភាព",
        "burmese": "ဘေးကင်းမှု",
        "cherokee": "ᎠᏂᏍᎦᏂᎩᏛ",
        "lao": "ບໍ່ປອດໄພ",
        "mongolian": "ᠠᠶᠤᠯᠲᠠᠢ",
        "assamese": "অসুরক্ষিত",
        "sindhi": "غیرمحفوظ",
        "punjabi_shahmukhi": "غیرمحفوظ",
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
        "polish": "obszar",          "turkish": "bölge",        "swedish": "område",
        "norwegian": "omrade",       "danish": "område",        "hungarian": "tartomány",
        "czech": "oblast",           "slovak": "oblasť",        "finnish": "alue",
        "romanian": "regiune",       "catalan": "regio",
        "armenian": "տարածք",        "georgian": "რეგიონი",
        "amharic": "ክልል",
        "tibetan": "ཁུལ",
        "malay": "kawasan",          "indonesian": "wilayah",   "filipino": "rehiyon",
        "nepali": "क्षेत्र",
        "maithili": "क्षेत्र",
        "konkani": "क्षेत्र",
        "bengali": "ক্ষেত্র",
        "tamil": "பகுதி",
        "telugu": "ప్రాంతం",
        "gujarati": "ક્ષેત્ર",
        "punjabi": "ਖੇਤਰ",
        "kannada": "ಪ್ರದೇಶ",
        "malayalam": "പ്രദേശം",
        "odia": "କ୍ଷେତ୍ର",
        "sinhala": "ප්‍රදේශය",
        "urdu": "علاقہ",
        "persian": "منطقه",
        "pashto": "سیمه",
        "swahili": "eneo",
        "yoruba": "agbègbè",
        "hausa": "yanki",
        "khmer": "តំបន់",
        "burmese": "ဒေသ",
        "cherokee": "ᎦᏙᎯ",
        "lao": "ພູມພາກ",
        "mongolian": "ᠪᠥᠰᠡ",
        "assamese": "ক্ষেত্র",
        "sindhi": "علاقہ",
        "punjabi_shahmukhi": "علاقہ",
    },
}

SUPPORTED_LANGS = (
    "english", "sanskrit", "hindi", "marathi", "mandarin",
    # South Asian
    "bengali", "odia", "gujarati", "punjabi", "sinhala",
    # Phase 2 dialects (2026-08-12): pragma-only aliases of an existing
    # shared keyword table -- see tools/regen_vani_translate_keywords.py's
    # LANG_TABLES/ALIAS_OF for exactly which table each reuses.
    "nepali", "maithili", "konkani", "assamese", "sindhi",
    "punjabi_shahmukhi",
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
    "sanskrit", "hindi", "marathi", "nepali", "maithili", "konkani",
    "bengali", "odia", "gujarati", "punjabi", "sinhala", "assamese",
    "tamil", "telugu", "kannada", "malayalam",
    "japanese", "korean",
    "urdu", "persian", "pashto", "sindhi", "punjabi_shahmukhi",
    "turkish", "mongolian", "tibetan",
})

# lexer.rs's `// vani-lang:` pragma parser expects specific tag spellings
# (src/lexer.rs's DialectLang pragma match, ~line 5860) that don't always
# match this tool's internal SUPPORTED_LANGS key -- currently just
# punjabi_shahmukhi (underscore, matching examples/language/punjabi_shahmukhi/
# and every other plain-lowercase-word key here) vs lexer.rs's
# "punjabi-shahmukhi" (hyphen, the only spelling its pragma match accepts
# besides "shahmukhi" / "pnb"). Used when WRITING a pragma; reading is
# handled by checking both spellings in detect_pragma_lang.
_PRAGMA_TAG_OVERRIDES: Dict[str, str] = {
    "punjabi_shahmukhi": "punjabi-shahmukhi",
}


def pragma_tag(lang: str) -> str:
    """The exact `// vani-lang: <tag>` string lexer.rs's pragma parser
    expects for `lang`. Almost always `lang` itself."""
    return _PRAGMA_TAG_OVERRIDES.get(lang, lang)

# Multi-word forms that the lexer fuses post-tokenization.
MULTI_WORD_ALIASES: Dict[Tuple[str, ...], str] = {
    ("नहीं", "तो"):      "Else",
    ("के", "लिए"):        "For",
    ("सिद्ध", "करो"):     "Prove",
    ("सिद्ध", "करा"):     "Prove",
    ("समान्तर", "प्रति"): "Parallel",
}

# BEGIN ALL_SYNONYMS (auto-generated by tools/regen_vani_translate_keywords.py)
ALL_SYNONYMS: Dict[str, List[str]] = {
    "Arrow": ['yields'],
    "As": [
        'as', 'यथा', 'হিসাবে', 'ஆக', 'గా', 'તરીકે', 'ਵਜੋਂ', 'ಆಗಿ', 'ആയി', 'ଭାବେ', 'ලෙස',
        'بطور', 'بعنوان', 'لکه', 'как', 'como', 'comme', 'として', '作为', '로서', 'als', 'sebagai',
        'ως', 'בתור', 'come', 'بصفة', 'jako', 'olarak', 'kama', 'như', 'ca', 'เป็น', 'mint',
        'ako', 'kuten', 'som', 'bilang', 'որպես', 'როგორც', 'com', 'bí_ti', 'kamar', 'ជា',
        'အဖြစ်', 'እንደ', 'དུ', 'ເປັນ',
    ],
    "Assert": [
        'assert', 'खात्री', 'सुनिश्चित', 'सिद्धम्', 'নিশ্চিত', 'உறுதி', 'నిర్ధారించు',
        'નિશ્ચિત', 'ਨਿਸ਼ਚਿਤ', 'ಖಚಿತಪಡಿಸಿ', 'ഉറപ്പിക്കുക', 'ନିଶ୍ଚିତ', 'තහවුරු', 'یقینی', 'ادعا',
        'تایید', 'утверждать', 'afirmar', 'vérifier', 'vérifie', 'affirmer', '確認', '断言', '확인',
        'überprüfen', 'überprüfe', 'prüfen', 'prüfe', 'behaupten', 'pastikan', 'επιβεβαίωση',
        'ודא', 'affermare', 'تأكد', 'potwierdź', 'potwierdz', 'doğrula', 'dogrula',
        'thibitisha', 'khẳng_định', 'afirmă', 'afirma', 'bevestig', 'ยืนยัน', 'állítsd',
        'allitsd', 'tvrď', 'tvrd', 'potvrď', 'potvrd', 'vahvista', 'påstå', 'bekrafta',
        'patunayan', 'bekreft', 'bekraeft', 'paastaa', 'հաստատել', 'დაამოწმე', 'jẹ́risí',
        'tabbatar', 'បញ្ជាក់', 'သေချာ', 'አረጋግጥ', 'ངེས', 'ᎯᏍᏗᏎᏍᏗ', 'ຢືນຢັນ', 'ᠪᠠᠲᠤᠯ',
    ],
    "Bool": ['bool', 'तर्क', 'बूल'],
    "Break": [
        'break', 'विराम', 'रुको', 'थांब', 'বিরাম', 'நிறுத்து', 'ఆపు', 'વિરામ', 'ਵਿਰਾਮ',
        'ನಿಲ್ಲಿ', 'നിർത്തുക', 'ବନ୍ଦ', 'නවත්වන්න', 'بند', 'بشکن', 'ودروه', 'прервать', 'romper',
        'interrompre', '中断', '중단', 'brechen', 'parar', 'interromper', 'berhenti', 'διακοπή',
        'שבור', 'הפסק', 'rompere', 'interrompere', 'كسر', 'przerwij', 'kır', 'kir', 'vunja',
        'ngắt', 'rupe', 'stop', 'หยุด', 'törj', 'torj', 'přeruš', 'prerus', 'preruš', 'perus',
        'katkaise', 'bryt', 'tumigil', 'bryd', 'ընդհատել', 'შეჩერება', 'trenca', 'dáwọ́dúró',
        'dakatar', 'បំបាក់', 'ရပ်', 'ስብር', 'འགོག', 'ᎤᎵᏍᎩᏗ', 'ຢຸດ', 'ᠵᠣᠭᠰᠣ',
    ],
    "Cancel": ['cancel', 'निरसन'],
    "Const": [
        'const', 'स्थिर', 'नियत', 'স্থির', 'மாறா', 'స్థిరం', 'સ્થિર', 'ਸਥਿਰ', 'ಸ್ಥಿರ',
        'സ്ഥിരം', 'ସ୍ଥିର', 'ස්ථිර', 'ثابت', 'постоянная', 'constante', '定数', '常量', '상수',
        'konstante', 'tetap', 'σταθερά', 'קבוע', 'costante', 'قيمة_ثابتة', 'stała', 'stala',
        'sabit', 'pemalar', 'thabiti', 'hằng', 'constantă', 'constanta', 'คงที่', 'állandó',
        'allando', 'konstanta', 'konštanta', 'vakio', 'konstant', 'pirme', 'հաստատուն',
        'მუდმივი', 'constant', 'àlàfo', 'tabbas', 'ថេរ', 'ပုံသေ', 'ቋሚ', 'རྟག', 'ᎠᏢᏓᏅᎯ',
        'ຄົງທີ່', 'ᠲᠣᠭᠲᠠᠮᠠᠯ',
    ],
    "Continue": [
        'continue', 'अग्रे', 'पुढे', 'आगे', 'এগিয়ে', 'தொடர்', 'కొనసాగించు', 'ચાલુ', 'ਜਾਰੀ',
        'ಮುಂದುವರಿಸಿ', 'തുടരുക', 'ଜାରି', 'ඉදිරියට', 'جاری', 'ادامه', 'دوام', 'продолжить',
        'continuar', 'continuer', '続行', '继续', '계속', 'weiter', 'lanjutkan', 'συνέχεια', 'המשך',
        'continuare', 'استمر', 'kontynuuj', 'devam', 'teruskan', 'endelea', 'tiếp_tục',
        'continuă', 'continua', 'verder', 'ดำเนินต่อ', 'folytat', 'folytasd', 'pokračuj',
        'pokracuj', 'jatka', 'fortsätt', 'fortsatt', 'magpatuloy', 'fortsett', 'fortsæt',
        'fortsaet', 'շարունակել', 'გაგრძელება', 'tẹ̀síwájú', 'ci_gaba', 'បន្ត', 'ဆက်လုပ်',
        'ቀጥል', 'མུ་མཐུད', 'ᏗᎧᎵᏍᏗ', 'ສືບຕໍ່', 'ᠦᠷᠭᠦᠯᠵᠢᠯᠡ',
    ],
    "Detach": ['detach', 'वियोजन'],
    "DownTo": [
        'downto', 'अधोतक', 'নিম্নপর্যন্ত', 'கீழ்வரைக்கும்', 'దిగువవరకూ', 'નીચેસુધી', 'ਹੇਠਤੱਕ',
        'ಕೆಳಗೆ', 'താഴെവരെക്കും', 'ନିମ୍ନପର୍ଯ୍ୟନ୍ତ', 'පහළදක්වා', 'نیچےتک', 'فروبه', 'ښکتهته',
        'донизу', 'hastaabajo', 'versbas', '下まで', '下到', '아래까지', 'bisrunter', 'atébaixo',
        'atebaixo', 'sampaibawah', 'hinggabawah', 'μέχρικάτω', 'עדלמטה', 'finogiu', 'إلىأسفل',
        'dodolu', 'aşağıkadar', 'asagikadar', 'hadichini', 'xuốngđến', 'pânăjos', 'panajos',
        'totbeneden', 'ลงถึง', 'lehatárig', 'lehatarig', 'dolů', 'dolu', 'nadol', 'alasasti',
        'nertill', 'pababahanggang', 'nedtil', 'ներքևմինչև', 'ქვემოთმდე', 'finsavall',
        'désílẹ̀', 'zuwakasa', 'ក្រោមដល់', 'အောက်သို့', 'ታችድረስ', 'མར་བར་དུ', 'ᎡᎳᏗᎬᏛ',
        'ລົງເຖິງ', 'ᠳᠣᠣᠷᠠᠬᠦᠷᠲᠡᠯᠡ',
    ],
    "EPrint": [
        'eprint', 'त्रुटिलिख', 'त्रुटिलिखो', 'दोषलिहा', 'ত্রুটিলেখ', 'பிழைஎழுது', 'లోపంరాయి',
        'ભૂલલખો', 'ਗਲਤੀਲਿਖੋ', 'ದೋಷಬರೆ', 'പിശക്എഴുതുക', 'ତ୍ରୁଟିଲେଖ', 'දෝෂයලියන්න', 'غلطیلکھو',
        'خطاچاپ', 'خطاولیکه', 'ошибкапечатать', 'errorimprimir', 'erreurimprimer', 'エラー表示',
        '错误打印', '오류출력', 'fehlerdrucken', 'erroimprimir', 'kesalahancetak', 'σφάλμαεκτύπωση',
        'שגיאההדפס', 'errorestampare', 'خطأاطبع', 'bladdrukuj', 'hatayazdır', 'hatayazdir',
        'ralatcetak', 'kosachapisha', 'lỗi_in', 'eroaretipărește', 'eroaretipareste',
        'foutdruk', 'ข้อผิดพลาดพิมพ์', 'hibanyomtat', 'hibanyomtass', 'chybavypiš',
        'chybatiskni', 'chybavytlač', 'chybavytlac', 'virhetulosta', 'felskriv',
        'pagkakamaliisulat', 'feilskriv', 'fejludskriv', 'սխալիտպել', 'შეცდომაბეჭდვა',
        'errorimprimeix', 'àṣìṣetẹ̀', 'kuskurerubuta', 'កំហុសបោះពុម្ព', 'အမှားပုံနှိပ်',
        'ስህተትህትመት', 'ནོར་འཁྲུལ་པར', 'ᎤᎴᏗᎠᎴᏂᏍᎬᎢ', 'ຂໍ້ຜິດພາດພິມ', 'ᠠᠯᠳᠠᠭᠬᠡᠪᠯᠡ',
    ],
    "Else": [
        'else', 'अन्यथा', 'वरना', 'नाहीतर', 'नहीं तो', 'নাহলে', 'অন্যথা', 'இல்லாவிட்டால்',
        'లేకపోతే', 'નહીંતર', 'ਵਰਨਾ', 'ಇಲ್ಲದಿದ್ದರೆ', 'അല്ലെങ്കിൽ', 'ଅନ୍ୟଥା', 'නැතිනම්', 'ورنہ',
        'وگرنه', 'کهنه', 'иначе', 'sino', 'sinon', 'そうでなければ', '否则', '아니면', 'sonst', 'senão',
        'senao', 'selainnya', 'lainnya', 'αλλιώς', 'אחרת', 'altrimenti', 'وإلا', 'inaczej',
        'yoksa', 'vinginevyo', 'ngược_lại', 'altfel', 'anders', 'ไม่เช่นนั้น', 'különben',
        'kulonben', 'jinak', 'inak', 'muuten', 'annars', 'kundi', 'ellers', 'այլապես', 'სხვა',
        'altrament', 'yàtọ̀', 'ko_kuwa', 'ផ្សេង', 'မဟုတ်ပါက', 'ካልሆነ', 'གཞན', 'ᎪᎯ',
        'ບໍ່ດັ່ງນັ້ນ', 'ᠡᠰᠡᠪᠡᠯ',
    ],
    "Ensures": [
        'ensures', 'निश्चित', 'सुनिश्चयित', 'সুনিশ্চিত', 'உறுதிப்படுத்து', 'నిశ్చయం', 'ખાતરી',
        'ਯਕੀਨੀ', 'ಖಚಿತ', 'ഉറപ്പ്', 'ସୁନିଶ୍ଚିତ', 'සහතික', 'ضمانت', 'تضمین', 'ډاډ',
        'гарантирует', 'garantiza', 'garantit', '保証', '保证', '보장', 'garantiert', 'garante',
        'jamin', 'εγγυάται', 'מבטיח', 'garantisce', 'يضمن', 'gwarantuje', 'garanti',
        'menjamin', 'hakikisha', 'đảm_bảo', 'garantează', 'garanteaza', 'verzekert',
        'รับประกัน', 'garantál', 'garantal', 'zajišťuje', 'zajistuje', 'zaručuje', 'zarucuje',
        'takaa', 'säkerställer', 'garanterar', 'tiyakin', 'garanterer', 'երաշխավորում',
        'უზრუნველყოფს', 'garanteix', 'dájú', 'tabbace', 'ធានា', 'ဆောင်ရွက်', 'ያረጋግጣል', 'ཁག',
        'ᎤᏙᎯᏳᏫᏍᏗ', 'ຮັບປະກັນ', 'ᠪᠠᠲᠤᠯᠠᠭ\u180eᠠ',
    ],
    "Enum": [
        'enum', 'विकल्प', 'गणन', 'গণনা', 'எண்ணுப்பெயர்', 'గణన', 'ગણના', 'ਗਣਨਾ', 'ಎಣಿಕೆ',
        'എണ്ണൽ', 'ଗଣନା', 'ගණනය', 'شمار', 'شمارش', 'شمېرل', 'перечисление', 'enumeración',
        'enumeracion', 'énumération', 'enumeration', '列挙', '枚举', '열거', 'aufzählung',
        'aufzaehlung', 'enumeração', 'enumeracao', 'enumerasi', 'απαρίθμηση', 'ספירה',
        'enumerazione', 'تعداد', 'wyliczenie', 'sıralama', 'siralama', 'penghitungan',
        'orodha', 'liệt_kê', 'enumerare', 'opsomming', 'การแจงนับ', 'felsorolás', 'felsorolas',
        'výčet', 'vycet', 'výpočet', 'vypocet', 'luettelointi', 'uppräkning', 'uppraekning',
        'pagbilang', 'oppregning', 'optælling', 'optaelling', 'թվարկում', 'ჩამოთვლა',
        'enumeració', 'enumeracio', 'àkọsílẹ̀', 'lissafi', 'ការរាប់បញ្ចូល', 'စာရင်း', 'ቆጠራ',
        'རྩིས', 'ᎢᎦᏙᎯ', 'ການນັບ', 'ᠲᠣᠭᠠᠯᠠᠯ',
    ],
    "Extern": [
        'extern', 'बाह्य', 'বাহ্যিক', 'வெளி', 'బాహ్య', 'બાહ્ય', 'ਬਾਹਰੀ', 'ಬಾಹ್ಯ', 'ബാഹ്യം',
        'ବାହ୍ୟ', 'බාහිර', 'بیرونی', 'خارجی', 'بهرنی', 'внешний', 'externo', 'étranger',
        'externe', '外部', '외부', 'äußere', 'äußerer', 'eksternal', 'εξωτερικό', 'חיצוני',
        'esterno', 'خارجي', 'zewnętrzny', 'zewnetrzny', 'dış', 'dis', 'luaran', 'nje',
        'bên_ngoài', 'ภายนอก', 'külső', 'kulso', 'vnější', 'vnejsi', 'vonkajší', 'vonkajsi',
        'ulkoinen', 'panlabas', 'ekstern', 'արտաքին', 'გარე', 'ìta', 'waje', 'ខាងក្រៅ',
        'အပြင်', 'ውጫዊ', 'ཕྱི', 'ᏙᏱᏗᏢ', 'ພາຍນອກ', 'ᠭᠠᠳᠠᠨ\u180eᠠ',
    ],
    "F32": ['f32', 'दशांश३२'],
    "F64": ['f64', 'दशांश', 'दशांश६४'],
    "False": [
        'false', 'असत्य', 'अशुद्ध', 'झूठ', 'गलत', 'खोटे', 'चूक', 'অসত্য', 'মিথ্যা', 'ভুল',
        'பொய்', 'అబద్ధం', 'ખોટું', 'ਝੂਠ', 'ಸುಳ್ಳು', 'ತಪ್ಪು', 'അസത്യം', 'തെറ്റ്', 'ମିଥ୍ୟା',
        'අසත්\u200dය', 'වැරදි', 'جھوٹ', 'نادرست', 'ناسم', 'ложь', 'неверно', 'falso', 'faux',
        '偽', '假', '거짓', 'falsch', 'salah', 'ψευδές', 'שקר', 'خطأ', 'fałsz', 'falsz', 'yanlış',
        'yanlis', 'palsu', 'uongo', 'sai', 'fals', 'onwaar', 'เท็จ', 'hamis', 'nepravda',
        'epatosi', 'falskt', 'mali', 'usant', 'falsk', 'կեղծ', 'მცდარი', 'irọ́', 'ƙarya',
        'មិនពិត', 'မှား', 'ሐሰት', 'རྫུན', 'ᎤᏝ', 'ບໍ່ຈິງ', 'ᠬᠤᠳᠠᠯ',
    ],
    "Fn": [
        'fn', 'फलन', 'कार्य', 'ফাংশন', 'কাজ', 'செயல்பாடு', 'சார்பு', 'ఫంక్షన్', 'పని', 'કાર્ય',
        'ફંકશન', 'ਕਾਰਜ', 'ਫੰਕਸ਼ਨ', 'ಕಾರ್ಯ', 'ಫಂಕ್ಷನ್', 'കാര്യം', 'ഫംഗ്ഷൻ', 'କାର୍ଯ୍ୟ',
        'ଫଙ୍କସନ୍', 'කාර්යය', 'ශ්\u200dරිතය', 'فنکشن', 'کام', 'تابع', 'فانکشن', 'کار',
        'функция', 'función', 'funcion', 'fonction', '関数', '函数', '함수', 'funktion', 'função',
        'funcao', 'fungsi', 'συνάρτηση', 'פונקציה', 'פעולה', 'funzione', 'دالة', 'funkcja',
        'işlev', 'fonksiyon', 'kazi', 'hàm', 'funcție', 'functie', 'ฟังก์ชัน', 'függvény',
        'fuggveny', 'funkce', 'funkcia', 'funktio', 'gawain', 'funksjon', 'ֆունկցիա',
        'ფუნქცია', 'funció', 'funcio', 'iṣẹ́', 'aiki', 'មុខងារ', 'လုပ်ဆောင်ချက်', 'ተግባር',
        'ལས་ཀ', 'ᏗᎦᏬᏂᎯᏍᏗ', 'ໜ້າທີ່', 'ᠴᠠᠭ',
    ],
    "For": [
        'for', 'प्रति', 'साठी', 'के लिए', 'প্রতি', 'ஒவ்வொரு', 'ప్రతి', 'પ્રતિ', 'ਹਰ', 'ಪ್ರತಿ',
        'ഓരോ', 'ପ୍ରତି', 'සෑම', 'ہر', 'هر', 'هریو', 'для', 'para', 'pour', '対象', '对于', '각각',
        'für', 'jede', 'untuk', 'για', 'לכל', 'per', 'لكل', 'dla', 'için', 'icin', 'kwa',
        'với_mỗi', 'pentru', 'voor', 'สำหรับ', 'minden', 'pro', 'pre', 'jokaiselle', 'för',
        'ամեն', 'თითოეული', 'fún', 'ga', 'សម្រាប់', 'အတွက်', 'ለ', 'ལ', 'ᏌᏊ', 'ສຳລັບ',
        'ᠬᠠᠷᠠᠭᠠᠯᠵᠠᠯ',
    ],
    "Forall": ['forall'],
    "From": [
        'from', 'से', 'থেকে', 'இருந்து', 'నుండి', 'થી', 'ਤੋਂ', 'ಇಂದ', 'നിന്ന്', 'ରୁ', 'සිට',
        'سے', 'از', 'له', 'от', 'desde', 'depuis', 'から', '从', '에서', 'von', 'dari', 'από',
        'מתוך', 'da', 'من', 'od', 'den', 'kutoka', 'từ', 'din', 'van', 'จาก', 'kezdve',
        'lähtien', 'alkaen', 'från', 'fran', 'mula', 'fra', 'ից', 'დან', 'des', 'láti', 'daga',
        'ពី', 'မှ', 'ጀምሮ', 'ནས', 'ᏓᏓᎴᏂᏍᎬ', 'ຈາກ', 'ᠠᠴᠠ',
    ],
    "I16": ['i16', 'पूर्णांक१६'],
    "I32": ['i32', 'पूर्णांक३२'],
    "I64": ['i64', 'पूर्णांक', 'पूर्णांक६४'],
    "I8": ['i8', 'पूर्णांक८'],
    "If": [
        'if', 'यदि', 'अगर', 'जर', 'যদি', 'என்றால்', 'எனில்', 'అయితే', 'જો', 'ਜੇ', 'ಆದರೆ',
        'എങ്കിൽ', 'ଯଦି', 'නම්', 'اگر', 'که', 'если', 'si', 'もし', '如果', '만약', '만일', 'wenn',
        'se', 'jika', 'αν', 'אם', 'إذا', 'jeśli', 'jesli', 'eğer', 'eger', 'kama_ni', 'ikiwa',
        'nếu', 'dacă', 'daca', 'indien', 'ถ้า', 'ha', 'pokud', 'jestli', 'ak', 'kedy', 'jos',
        'om', 'kung', 'hvis', 'եթե', 'თუ', 'bí', 'idan', 'បើ', 'ဆိုလျှင်', 'ከ', 'གལ་ཏེ', 'ᎢᏳᏃ',
        'ຖ້າ', 'ᠬᠡᠷᠪᠡ',
    ],
    "Implement": [
        'impl', 'कार्यान्वित', 'কার্যান্বিত', 'செயல்படுத்து', 'అమలు', 'અમલ', 'ਅਮਲ', 'ಜಾರಿ',
        'നടപ്പിലാക്കുക', 'ପ୍ରୟୋଗ', 'ක්\u200dරියාත්මක', 'نافذ', 'اجرا', 'پلي', 'реализовать',
        'implementar', 'implémenter', 'implementer', '実装', '实现', '구현', 'implementieren',
        'terapkan', 'implementasi', 'υλοποίηση', 'ממש', 'implementare', 'نفذ', 'zaimplementuj',
        'uygula', 'laksanakan', 'tekeleza', 'triển_khai', 'implementează', 'implementeaza',
        'implementeer', 'ดำเนินการ', 'megvalósít', 'valositsd_meg', 'implementuj', 'toteuta',
        'implementera', 'ipatupad', 'implementér', 'իրականացնել', 'განხორციელება',
        'implementa', 'muṣẹ', 'aiwatar', 'អនុវត្ត', 'အကောင်အထည်ဖော်', 'ተግባራዊ', 'ལག་བསྟར',
        'ᎬᏔᏂᏙᎲ', 'ປະຕິບັດ', 'ᠬᠡᠷᠡᠭᠵᠢᠭᠦᠯ',
    ],
    "In": [
        'in', 'में', 'মধ্যে', 'உள்', 'లో', 'માં', 'ਵਿੱਚ', 'ರಲ್ಲಿ', 'ഇൽ', 'ରେ', 'තුළ', 'میں',
        'در', 'په', 'в', 'en', 'dans', '中', '在', '안에', 'em', 'dalam', 'σε', 'בתוך', 'في', 'w',
        'içinde', 'icinde', 'ndani', 'trong', 'în', 'ใน', 'belül', 'belul', 'uvnitř', 'uvnitr',
        'vnútri', 'vnutri', 'sisalla', 'inuti', 'sa', 'inni', 'indeni', 'մեջ', 'ში', 'nínú',
        'cikin', 'ក្នុង', 'ထဲမှာ', 'ውስጥ', 'ནང', 'ᎭᏫᎾ', 'ໃນ', 'ᠳᠣᠲᠣᠷ\u180eᠠ',
    ],
    "Intent": [
        'intent', 'उद्देश्य', 'উদ্দেশ্য', 'நோக்கம்', 'ఉద్దేశం', 'ઉદ્દેશ', 'ਉਦੇਸ਼', 'ಉದ್ದೇಶ',
        'ഉദ്ദേശ്യം', 'ଉଦ୍ଦେଶ୍ୟ', 'අරමුණ', 'مقصد', 'هدف', 'موخه', 'цель', 'intención',
        'propósito', 'intencion', 'but', 'objectif', '目的', '意図', '意图', '목적', 'absicht',
        'intenção', 'intencao', 'proposito', 'objetivo', 'tujuan', 'niat', 'σκοπός', 'מטרה',
        'scopo', 'intenzione', 'obiettivo', 'cel', 'intencja', 'amaç', 'amac', 'lengo',
        'mục_đích', 'scop', 'doel', 'จุดประสงค์', 'cél', 'záměr', 'zamer', 'účel', 'ucel',
        'tarkoitus', 'syfte', 'layunin', 'formål', 'hensikt', 'formaal', 'նպատակ', 'მიზანი',
        'propòsit', 'proposit', 'objectiu', 'ìpinnu', 'nufin', 'គោលបំណង', 'ရည်ရွယ်ချက်', 'ዓላማ',
        'དམིགས་ཡུལ', 'ᎤᎲᏍᏛ', 'ຈຸດປະສົງ', 'ᠵᠣᠷᠢᠯᠭ\u180eᠠ',
    ],
    "Interface": [
        'trait', 'संकेत', 'अंतरापृष्ठ', 'সংকেত', 'இடைமுகம்', 'సంకేతం', 'સંકેત', 'ਸੰਕੇਤ',
        'ಸಂಕೇತ', 'സങ്കേതം', 'ସଙ୍କେତ', 'සංකේතය', 'رابطہ', 'رابط', 'اړیکه', 'интерфейс',
        'interfaz', 'interface', 'インターフェース', '接口', '인터페이스', 'schnittstelle', 'antarmuka',
        'διεπαφή', 'ממשק', 'interfaccia', 'واجهة', 'interfejs', 'arayüz', 'arayuz',
        'antaramuka', 'kiolesura', 'giao_diện', 'interfață', 'interfata', 'อินเทอร์เฟซ',
        'felület', 'felulet', 'rozhraní', 'rozhrani', 'rozhranie', 'rajapinta', 'gränssnitt',
        'granssnitt', 'ugnayan', 'grensesnitt', 'grænseflade', 'graenseflade', 'միջերես',
        'ინტერფეისი', 'interfície', 'ifaramọ', 'hannu', 'ចំណុចប្រទាក់', 'မျက်နှာပြင်', 'በይነገጽ',
        'འབྲེལ་མཐུད', 'ᎠᏓᏛᏗ', 'ສ່ວນຕິດຕໍ່', 'ᠵᠠᠯᠭᠠᠭᠤᠷ',
    ],
    "Invariant": [
        'invariant', 'अपरिवर्तनीय', 'অপরিবর্তনীয়', 'மாறிலா', 'మారని', 'અચળ', 'ਅਟੱਲ', 'ಅಚಲ',
        'അചലം', 'ଅଚଳ', 'නිශ්චල', 'غیرمتغیر', 'تغییرناپذیر', 'دايمي', 'инвариант', 'invariante',
        '不変', '不变量', '불변', 'unveraenderlich', 'invarian', 'αμετάβλητο', 'בלתי_משתנה', 'مستقر',
        'niezmienny', 'değişmez', 'degismez', 'tetap', 'isiyobadilika', 'bất_biến',
        'ไม่เปลี่ยน', 'változatlan', 'valtozatlan', 'neměnný', 'nemenny', 'nemenný',
        'muuttumaton', 'oföränderlig', 'oforanderlig', 'walangpalit', 'uforanderlig',
        'անփոփոխ', 'უცვლელი', 'àìyípadà', 'a_canzawa', 'មិនប្រែប្រួល', 'မပြောင်းလဲ', 'የማይለወጥ',
        'མི་འགྱུར', 'ᏂᎦᎳᏛᎾ', 'ບໍ່ປ່ຽນ', 'ᠲᠣᠭᠲᠠᠪᠤᠷᠢᠲᠠᠢ',
    ],
    "Is": [
        'is', 'है', 'अस्ति', 'आहे', 'হয়', 'ஆகும்', 'ఉంది', 'છે', 'ਹੈ', 'ಇದೆ', 'ആണ്', 'ଅଟେ',
        'වේ', 'ہے', 'است', 'دی', 'есть', 'es', 'est', 'は', '是', '이다', 'ist', 'eh', 'adalah',
        'είναι', 'הוא', 'risulta', 'هو', 'jest', 'olur', 'ni', 'là', 'este', 'คือ', 'van',
        'je', 'on', 'är', 'ar', 'ay', 'er', 'է', 'არის', 'està', 'és', 'ne', 'គឺ', 'ဖြစ်သည်',
        'ነው', 'ཡིན', 'ᎨᏒ', 'ແມ່ນ', 'ᠪᠣᠯᠤᠨ\u180eᠠ',
    ],
    "Join": [
        'join', 'संयोजन', 'যোগ', 'சேர்', 'సంయోగం', 'સંયોજન', 'ਸੰਯੋਜਨ', 'ಸಂಯೋಜನೆ', 'സംയോജനം',
        'ସଂଯୋଜନ', 'සංයෝජනය', 'ملاپ', 'پیوستن', 'نښلول', 'соединить', 'unir', 'joindre', '結合',
        '等待', '合并', '결합', 'verbinden', 'juntar', 'gabungkan', 'ένωση', 'חיבור', 'unire',
        'اربط', 'połącz', 'polacz', 'birleştir', 'birlestir', 'gabung', 'unganisha', 'kết_hợp',
        'unește', 'uneste', 'verbind', 'รวม', 'egyesít', 'egyesit', 'spoj', 'yhdista',
        'förena', 'forena', 'pagsama', 'forene', 'forén', 'foren', 'միանալ', 'შეერთება',
        'uneix', 'darapọ', 'hadawa', 'ភ្ជាប់', 'ပူးပေါင်း', 'ቀላቀል', 'མཐུན་སྦྱོར', 'ᏓᏂᎳᏫᏍᎦ',
        'ເຊື່ອມ', 'ᠨᠡᠶᠢᠯᠡ',
    ],
    "Len": ['len'],
    "Let": [
        'assign', 'मान', 'माना', 'মান', 'ধরো', 'கொள்', 'అనుకో', 'માનો', 'ધારો', 'ਮੰਨੋ',
        'ಊಹಿಸಿ', 'ಮಾನ್ಯ', 'കരുതുക', 'ମନେକର', 'අනුමානය', 'مانیں', 'فرض', 'بگذار', 'ووایه',
        'пусть', 'sea', 'soit', '代入', '让', '정의', 'sei', 'seja', 'biarkan', 'misalkan', 'έστω',
        'יהי', 'sia', 'ليكن', 'niech', 'olsun', 'acha', 'đặt', 'fie', 'laat', 'ให้', 'legyen',
        'nechť', 'nechtt', 'nech', 'olkoon', 'låt', 'lat', 'hayaan', 'la', 'lad', 'թող',
        'მიეცი', 'sigui', 'jẹ́', 'bari', 'អោយ', 'ထား', 'ይሁን', 'ཡོད་པར་ཤོག', 'ᎠᏁᎳ', 'ໃຫ້',
        'ᠶᠠᠪᠤᠭᠤᠯ',
    ],
    "Match": [
        'match', 'जुळवा', 'मिलान', 'मेल', 'मेलन', 'মেলে', 'মিলান', 'பொருந்து', 'సరిపోలు',
        'મેળવો', 'ਮੇਲ', 'ಹೊಂದಾಣಿಕೆ', 'പൊരുത്തപ്പെടുത്തുക', 'ମେଳ', 'ගැලපීම', 'ملان', 'مماثلت',
        'تطبیق', 'سمون', 'совпадение', 'coincidir', 'correspondre', '一致', 'マッチ', '匹配', '일치',
        'übereinstimmen', 'passend', 'combinar', 'corresponder', 'cocokkan', 'padanan',
        'αντιστοιχία', 'התאם', 'corrispondere', 'combaciare', 'طابق', 'dopasuj', 'eşle',
        'esle', 'padan', 'linganisha', 'khớp', 'potrivește', 'potriveste', 'vergelijk',
        'ตรงกัน', 'illeszkedik', 'egyezzen', 'odpovídej', 'odpovidej', 'porovnaj', 'vastaa',
        'matcha', 'tugmain', 'sammenlign', 'համապատասխանեցնել', 'შესაბამისობა', 'coincideix',
        'bámu', 'dace', 'ផ្គូផ្គង', 'ကိုက်ညီ', 'ተዛመደ', 'མཐུན', 'ᎠᏍᏓᏩᏛᏍᎩ', 'ກົງກັນ', 'ᠲᠣᠬᠢᠷᠠ',
    ],
    "Methods": [
        'methods', 'विधि', 'বিধি', 'முறைகள்', 'పద్ధతులు', 'પદ્ધતિઓ', 'ਢੰਗ', 'ವಿಧಾನಗಳು',
        'രീതികൾ', 'ପଦ୍ଧତି', 'ක්\u200dරම', 'طریقے', 'روش', 'طریقې', 'методы', 'métodos',
        'metodos', 'méthodes', 'methodes', 'メソッド', '方法', '메서드', 'methoden', 'metode',
        'μέθοδοι', 'שיטות', 'metodi', 'طرق', 'metody', 'metotlar', 'kaedah', 'njia',
        'phương_thức', 'วิธีการ', 'metódusok', 'metodusok', 'metódy', 'menetelmat', 'metoder',
        'pamamaraan', 'մեթոդներ', 'მეთოდები', 'mètodes', 'metodes', 'ipa', 'hanyoyi',
        'វិធីសាស្ត្រ', 'နည်းလမ်း', 'ዘዴዎች', 'ཐབས་ལམ', 'ᏗᏄᎪᏗ', 'ວິທີການ', 'ᠠᠷᠭ\u180eᠠ',
    ],
    "Module": [
        'mod', 'खण्ड', 'मॉड्यूल', 'খণ্ড', 'தொகுதி', 'మాడ్యూల్', 'ખંડ', 'ਖੰਡ', 'ಖಂಡ', 'ഖണ്ഡം',
        'ଖଣ୍ଡ', 'මොඩියුලය', 'ماڈیول', 'حصہ', 'بخش', 'برخه', 'модуль', 'módulo', 'modulo',
        'module', 'モジュール', '単位', '模块', '모듈', 'modul', 'ενότητα', 'άρθρωμα', 'מודול', 'מודולים',
        'وحدة', 'moduł', 'modül', 'moduli', 'mô_đun', 'โมดูล', 'moduuli', 'modyul', 'մոդուլ',
        'მოდული', 'mòdul', 'ìṣù', 'sashe', 'ម៉ូឌុល', 'ယူနစ်', 'ሞዱል', 'ཚན', 'ᎠᏯᏙᎸ', 'ໂມດູນ',
        'ᠨᠢᠭᠡᠴᠡ',
    ],
    "Mut": [
        'mut', 'बदल', 'बदलणारा', 'परिवर्तनीय', 'পরিবর্তনীয়', 'மாறக்கூடிய', 'మార్చదగిన',
        'પરિવર્તનીય', 'ਬਦਲਣਯੋਗ', 'ಪರಿವರ್ತನೀಯ', 'മാറ്റാവുന്ന', 'ପରିବର୍ତ୍ତନୀୟ', 'පරිවර්තනීය',
        'بدلنا', 'تغییرپذیر', 'بدلېدونکی', 'изменяемый', 'mutable', 'muable', '可変', '可变', '가변',
        'veränderlich', 'veränderbar', 'wandelbar', 'mutável', 'mutavel', 'dapatberubah',
        'berubah', 'μεταβλητό', 'משתנה', 'mutevole', 'متغير', 'zmienny', 'değişken',
        'degisken', 'badilika', 'có_thể_thay_đổi', 'schimbabil', 'veranderlijk',
        'เปลี่ยนแปลงได้', 'változó', 'valtozo', 'proměnný', 'promenny', 'meniteľný',
        'menitelny', 'muuttuva', 'föränderlig', 'foranderlig', 'nababago', 'endrelig',
        'mutérbar', 'փոփոխական', 'ცვალებადი', 'canviable', 'àyípadà', 'canzawa',
        'អាចផ្លាស់ប្តូរ', 'ပြောင်းလဲနိုင်', 'ሊቀየር', 'འགྱུར', 'ᏚᎵᎮᎵᎬᎢ', 'ປ່ຽນແປງໄດ້',
        'ᠬᠤᠪᠢᠷᠠᠮᠲᠠᠭᠠᠢ',
    ],
    "Parallel": [
        'parallel', 'समानांतर', 'समान्तर प्रति', 'সমান্তরাল', 'இணை', 'సమాంతర', 'સમાંતર',
        'ਸਮਾਂਤਰ', 'ಸಮಾನಾಂತರ', 'സമാന്തരം', 'ସମାନ୍ତର', 'සමාන්තර', 'متوازی', 'موازی', 'موازي',
        'параллельный', 'paralelo', 'parallèle', 'parallele', '並列', '并行', '병렬', 'paralel',
        'παράλληλο', 'מקבילי', 'parallelo', 'متوازي', 'równoległy', 'rownolegly', 'selari',
        'sambamba', 'song_song', 'ขนาน', 'párhuzamos', 'parhuzamos', 'paralelní', 'paralelni',
        'paralelný', 'paralelny', 'rinnakkainen', 'parallell', 'magkatulad', 'զուգահեռ',
        'პარალელური', 'akáṣe', 'madaidaici', 'ស្របគ្នា', 'ပြိုင်တူ', 'ትይዩ', 'མཉམ', 'ᎾᏍᎩᏯ',
        'ຂະໜານ', 'ᠵᠡᠷᠭᠡ',
    ],
    "Print": [
        'write', 'लिख', 'लिखो', 'लिहा', 'लिही', 'लिहिया', 'লেখ', 'লিখো', 'எழுது', 'அச்சிடு',
        'రాయి', 'ముద్రించు', 'લખો', 'છાપો', 'ਲਿਖੋ', 'ਛਾਪੋ', 'ಬರೆ', 'ಮುದ್ರಿಸಿ', 'എഴുതുക',
        'അച്ചടിക്കുക', 'ଲେଖ', 'ଛାପନ୍ତୁ', 'ලියන්න', 'මුද්\u200dරණය', 'لکھو', 'چھاپو', 'چاپ',
        'بنویس', 'ولیکه', 'печатать', 'писать', 'imprimir', 'escribir', 'écrire', 'écris',
        'imprimer', 'imprime', 'afficher', '表示', '書く', '打印', '输出', '출력', '쓰기', 'drucken',
        'schreiben', 'escrever', 'cetak', 'tulis', 'εκτύπωση', 'γράψε', 'הדפס', 'כתוב',
        'stampare', 'scrivere', 'اطبع', 'drukuj', 'wypisz', 'yazdır', 'yazdir', 'chapisha',
        'andika', 'in', 'in_ra', 'tipărește', 'scrie', 'tipareste', 'druk', 'schrijf', 'พิมพ์',
        'nyomtat', 'nyomtass', 'vypiš', 'tiskni', 'vypis', 'vytlač', 'píš', 'vytlac', 'pis',
        'tulosta', 'skriv', 'isulat', 'udskriv', 'տպել', 'ბეჭდვა', 'imprimeix', 'tẹ̀',
        'rubuta', 'បោះពុម្ព', 'ပုံနှိပ်', 'ህትመት', 'པར', 'ᎠᎴᏂᏍᎬᎢ', 'ພິມ', 'ᠬᠡᠪᠯᠡ',
    ],
    "Prove": [
        'prove', 'सिद्ध', 'प्रमाण', 'प्रमाणित', 'दर्शाओ', 'दाखवा', 'सिद्ध करो', 'सिद्ध करा',
        'প্রমাণ', 'நிரூபி', 'నిరూపించు', 'પ્રમાણ', 'ਪ੍ਰਮਾਣ', 'ಸಾಬೀತುಪಡಿಸಿ', 'തെളിയിക്കുക',
        'ପ୍ରମାଣ', 'ඔප්පු', 'ثبوت', 'اثبات', 'доказать', 'demostrar', 'démontrer', 'démontre',
        'prouver', '証明', '证明', '증명', 'beweisen', 'provar', 'demonstrar', 'buktikan',
        'απόδειξη', 'הוכח', 'dimostrare', 'أثبت', 'udowodnij', 'kanıtla', 'kanitla',
        'thibitisha_kuwa', 'thibitisha_kabisa', 'chứng_minh', 'dovedește', 'dovedeste',
        'bewijs', 'พิสูจน์', 'bizonyítsd', 'bizonyitsd', 'dokaž', 'dokaz', 'dokáž', 'todista',
        'bevisa', 'ipakita', 'bevis', 'ապացուցել', 'დაამტკიცე', 'demostra', 'fihàn', 'nuna',
        'បង្ហាញ', 'သက်သေပြ', 'አስረዳ', 'བསྒྲུབས', 'ᎠᎩᏠᏯᏍᏗ', 'ພິສູດ', 'ᠨᠣᠲᠠᠯᠠ',
    ],
    "Pub": [
        'public', 'सार्वजनिक', 'সর্বজনীন', 'பொது', 'ప్రజా', 'જાહેર', 'ਜਨਤਕ', 'ಸಾರ್ವಜನಿಕ',
        'പൊതു', 'ସର୍ବସାଧାରଣ', 'පොදු', 'عوامی', 'عمومی', 'عمومي', 'публичный', 'общий',
        'público', 'publico', '公開', '公开', '공개', 'öffentlich', 'oeffentlich', 'publik', 'umum',
        'δημόσιο', 'ציבורי', 'pubblico', 'عام', 'publiczny', 'genel', 'awam', 'umma',
        'công_khai', 'openbaar', 'สาธารณะ', 'nyilvános', 'nyilvanos', 'veřejný', 'verejny',
        'verejný', 'julkinen', 'offentlig', 'pampubliko', 'հանրային', 'საჯარო', 'públic',
        'gbangba', 'gama_gari', 'សាធារណៈ', 'အများပြည်သူ', 'ሕዝባዊ', 'སྤྱི', 'ᏂᎦᏓ', 'ສາທາລະນະ',
        'ᠨᠡᠶᠢᠲᠡ',
    ],
    "Pure": [
        'pure', 'शुद्ध', 'শুদ্ধ', 'தூய', 'శుద్ధ', 'શુદ્ધ', 'ਸ਼ੁੱਧ', 'ಶುದ್ಧ', 'ശുദ്ധം', 'ଶୁଦ୍ଧ',
        'ශුද්ධ', 'خالص', 'чистый', 'puro', 'pur', '純粋', '纯', '纯粹', '순수', 'rein', 'murni',
        'καθαρό', 'טהור', 'نقي', 'czysty', 'saf', 'tulen', 'safi', 'thuần_túy', 'zuiver',
        'บริสุทธิ์', 'tiszta', 'čistý', 'cisty', 'puhdas', 'ren', 'dalisay', 'մաքուր', 'სუფთა',
        'mímọ́', 'tsabta', 'បរិសុទ្ធ', 'သန့်ရှင်း', 'ንጹህ', 'གཙང', 'ᎦᏅᎯᏛ', 'ບໍລິສຸດ', 'ᠴᠡᠪᠡᠷ',
    ],
    "Reduce": [
        'reduce', 'संक्षेप', 'সংক্ষেপ', 'குறை', 'సంక్షేప', 'સંક્ષેપ', 'ਸੰਖੇਪ', 'ಸಂಕ್ಷೇಪ',
        'സംക്ഷേപം', 'ସଂକ୍ଷେପ', 'සංක්ෂේප', 'تخفیف', 'کاهش', 'کمښت', 'сократить', 'reducir',
        'reduire', '削減', '减少', '축소', 'reduzieren', 'reduzir', 'kurangi', 'μείωση', 'הפחתה',
        'ridurre', 'تقليل', 'zmniejsz', 'azalt', 'kurangkan', 'punguza', 'giảm', 'verminder',
        'ลด', 'csökkent', 'csokkent', 'zmenši', 'zmensi', 'znizit', 'vahenna', 'reducera',
        'bawasan', 'reduser', 'reducer', 'կրճատում', 'შემცირება', 'redueix', 'dínku', 'rage',
        'កាត់បន្ថយ', 'လျှော့ချ', 'ቀንስ', 'ཉུང་དུ', 'ᎤᏍᏗᎪᏗ', 'ຫຼຸດ', 'ᠪᠠᠭᠠᠰᠬᠠ',
    ],
    "Ref": [
        'ref', 'पहा', 'देखो', 'दृष्ट्या', 'দেখ', 'பார்', 'చూడు', 'જુઓ', 'ਵੇਖੋ', 'ನೋಡಿ',
        'നോക്കുക', 'ଦେଖନ୍ତୁ', 'බලන්න', 'دیکھیں', 'ببین', 'وګوره', 'смотри', 'ver', 'référence',
        'voir', '参照', '引用', '참조', 'sehen', 'referência', 'lihat', 'αναφορά', 'הפנייה',
        'vedere', 'مرجع', 'zobacz', 'gör', 'bak', 'tazama', 'tham_chiếu', 'vezi', 'zie', 'ดู',
        'nézd', 'nezd', 'viz', 'pozri', 'katso', 'se', 'tingnan', 'տեսնել', 'ნახე', 'veure',
        'wò', 'duba', 'មើល', 'ကြည့်', 'ይመልከት', 'ལྟ', 'ᎯᎪᎲᎢ', 'ເບິ່ງ', 'ᠦᠵᠡ',
    ],
    "RegionKw": [
        'region', 'क्षेत्र', 'ক্ষেত্র', 'பகுதி', 'ప్రాంతం', 'ક્ષેત્ર', 'ਖੇਤਰ', 'ಪ್ರದೇಶ',
        'പ്രദേശം', 'କ୍ଷେତ୍ର', 'ප්\u200dරදේශය', 'علاقہ', 'منطقه', 'سیمه', 'область', 'región',
        'région', '領域', '区域', '영역', 'bereich', 'região', 'regiao', 'wilayah', 'περιοχή',
        'אזור', 'regione', 'منطقة', 'obszar', 'bölge', 'bolge', 'kawasan', 'eneo', 'vùng',
        'regiune', 'gebied', 'พื้นที่', 'tartomány', 'tartomany', 'oblast', 'oblasť', 'alue',
        'område', 'omrade', 'rehiyon', 'omraade', 'տարածք', 'რეგიონი', 'regió', 'regio',
        'agbègbè', 'yanki', 'តំបន់', 'ဒေသ', 'ክልል', 'ཁུལ', 'ᎦᏙᎯ', 'ພູມພາກ', 'ᠪᠥᠰᠡ',
    ],
    "Requires": [
        'requires', 'अपेक्षित', 'चाहिए', 'पाहिजे', 'প্রয়োজনীয়', 'தேவை', 'అవసరం', 'જરૂરી',
        'ਲੋੜੀਂਦਾ', 'ಅಗತ್ಯ', 'ആവശ്യം', 'ଆବଶ୍ୟକ', 'අවශ්\u200dය', 'درکار', 'نیاز', 'اړتیا',
        'требует', 'requiere', 'exige', '前提', '要求', '필요', 'benoetigt', 'requer', 'perlu',
        'απαιτεί', 'דורש', 'richiede', 'يتطلب', 'wymaga', 'gerek', 'memerlukan', 'hitaji',
        'yêu_cầu', 'necesită', 'necesita', 'vereist', 'ต้องการ', 'igényel', 'igenyel',
        'vyžaduje', 'vyzaduje', 'vaatii', 'kräver', 'krever', 'kailangan', 'kræver', 'kraever',
        'պահանջում', 'მოითხოვს', 'requereix', 'nílò', 'bukata', 'ត្រូវការ', 'လို', 'ይፈልጋል',
        'དགོས', 'ᎠᏎᏗ', 'ຕ້ອງການ', 'ᠱᠠᠭᠠᠷᠳᠠ',
    ],
    "Return": [
        'give_back', 'परत', 'लौटाओ', 'पुनरागम', 'ফেরত', 'প্রত্যাবর্তন', 'திருப்பு', 'తిరిగి',
        'પાછા', 'ਮੁੜੋ', 'ಹಿಂದಿರುಗಿ', 'ಮರಳಿ', 'തിരികെ', 'ଫେରନ୍ତୁ', 'ආපසු', 'واپس', 'لوٹاؤ',
        'بازگشت', 'بېرته', 'вернуть', 'верни', 'regresar', 'retornar', 'volver', 'retourner',
        'retourne', '戻る', '返す', '返回', '반환', '돌려주기', 'zurück', 'zurueck', 'retorne', 'kembali',
        'kembalikan', 'επιστροφή', 'החזר', 'חזרה', 'ritornare', 'ritorna', 'أرجع', 'إرجاع',
        'wróć', 'zwróć', 'zwroc', 'wroc', 'dön', 'döndür', 'geri', 'don', 'rudi', 'trả_về',
        'întoarce', 'intoarce', 'terug', 'คืน', 'visszatér', 'visszater', 'vrať', 'vrat',
        'vráť', 'palaa', 'återvänd', 'atervand', 'ibalik', 'returner', 'tilbake', 'vend',
        'վերադարձ', 'დაბრუნება', 'retorna', 'padà', 'koma', 'ត្រលប់', 'ပြန်', 'መልስ', 'ལོག',
        'ᏗᎬᏎᏗ', 'ກັບຄືນ', 'ᠪᠤᠴᠠ',
    ],
    "Struct": [
        'record', 'संरचना', 'গঠন', 'கட்டமைப்பு', 'నిర్మాణం', 'રચના', 'ਰਚਨਾ', 'ರಚನೆ', 'ഘടന',
        'ଗଠନ', 'ව්\u200dයුහය', 'ساخت', 'ساختار', 'جوړښت', 'структура', 'estructura',
        'structure', '構造体', '结构', '结构体', '구조체', 'struktur', 'estrutura', 'δομή', 'מבנה',
        'struttura', 'بنية', 'struktura', 'yapı', 'yapi', 'muundo', 'cấu_trúc', 'structură',
        'structura', 'structuur', 'โครงสร้าง', 'szerkezet', 'štruktúra', 'rakenne',
        'istraktura', 'կառուցվածք', 'სტრუქტურა', 'ọ̀nà', 'tsari', 'រចនាសម្ព័ន្ធ',
        'ဖွဲ့စည်းပုံ', 'መዋቅር', 'སྒྲིག་གཞི', 'ᎠᏙᏢᏍᎩ', 'ໂຄງສ້າງ', 'ᠪᠦᠳᠦᠭᠴᠡ',
    ],
    "Task": [
        'task', 'नियोग', 'নিয়োগ', 'பணி', 'కార్యం', 'નિયોગ', 'ਨਿਯੋਗ', 'ನಿಯೋಗ', 'നിയോഗം',
        'ନିଯୋଗ', 'නියෝගය', 'ٹاسک', 'وظیفه', 'دنده', 'задача', 'tarea', 'tâche', 'travail',
        'タスク', '任务', '작업', 'ausführbar', 'aufgabe', 'tarefa', 'tugas', 'εργασία', 'משימה',
        'compito', 'مهمة', 'zadanie', 'görev', 'gorev', 'tugasan', 'jukumu', 'công_việc',
        'sarcină', 'sarcina', 'taak', 'งาน', 'feladat', 'úloha', 'uloha', 'tehtävä', 'tehtava',
        'uppgift', 'tungkulin', 'oppgave', 'opgave', 'խնդիր', 'დავალება', 'tasca', 'ojúṣe',
        'hidima', 'ភារកិច្ច', 'တာဝန်', 'ስራ', 'ལས', 'ᏗᎦᎸᏫᏍᏓᏁᏗ', 'ວຽກງານ', 'ᠡᠭᠦᠷᠭᠡ',
    ],
    "Then": [
        'then', 'तदा', 'तो', 'तर', 'তবে', 'அப்போது', 'అప్పుడు', 'પછી', 'ਤਦ', 'ನಂತರ', 'പിന്നെ',
        'ତାହେଲେ', 'පසු', 'تب', 'سپس', 'بیا', 'тогда', 'entonces', 'alors', 'ならば', '那么', '그러면',
        'dann', 'então', 'entao', 'maka', 'τότε', 'אז', 'allora', 'ثم', 'wtedy', 'sonra',
        'kisha', 'thì', 'atunci', 'dan', 'แล้ว', 'akkor', 'pak', 'potom', 'sitten', 'så', 'sa',
        'saka', 'da', 'saa', 'ապա', 'მაშინ', 'aleshores', 'nígbànáà', 'sannan', 'បន្ទាប់មក',
        'ထို့နောက်', 'ከዚያ', 'དེ་ནས', 'ᎣᏂ', 'ແລ້ວ', 'ᠳᠠᠷᠠᠭ\u180eᠠ',
    ],
    "To": [
        'to', 'तक', 'পর্যন্ত', 'வரைக்கும்', 'వరకూ', 'સુધી', 'ਤੱਕ', 'ಗೆ', 'വരെക്കും',
        'ପର୍ଯ୍ୟନ୍ତ', 'දක්වා', 'تک', 'به', 'ته', 'до', 'hasta', 'vers', 'まで', '到', '까지', 'bis',
        'até', 'ate', 'sampai', 'hingga', 'μέχρι', 'עד', 'finoa', 'إلى', 'do', 'kadar', 'hadi',
        'đến', 'până', 'pana', 'tot', 'ถึง', 'határig', 'hatarig', 'asti', 'till', 'hanggang',
        'til', 'մինչև', 'მდე', 'fins', 'dé', 'zuwa', 'ដល់', 'သို့', 'ድረስ', 'བར་དུ', 'ᎬᏛ',
        'ເຖິງ', 'ᠬᠦᠷᠲᠡᠯᠡ',
    ],
    "True": [
        'true', 'सत्य', 'सही', 'सच', 'बरोबर', 'खरे', 'সত্য', 'ঠিক', 'மெய்', 'నిజం', 'સાચું',
        'ਸੱਚ', 'ಸತ್ಯ', 'ಸರಿ', 'സത്യം', 'ശരി', 'ସତ୍ୟ', 'සත්\u200dය', 'හරි', 'سچ', 'درست', 'سم',
        'истина', 'верно', 'verdadero', 'vérité', 'vrai', '真', '참', 'wahr', 'verdadeiro',
        'benar', 'αληθές', 'אמת', 'vero', 'صحيح', 'prawda', 'doğru', 'dogru', 'kweli', 'đúng',
        'adevărat', 'adevarat', 'waar', 'จริง', 'igaz', 'pravda', 'tosi', 'sant', 'totoo',
        'sandt', 'ճշմարիտ', 'ჭეშმარიტი', 'cert', 'veritable', 'òótọ́', 'gaskiya', 'ពិត',
        'မှန်', 'እውነት', 'བདེན', 'ᎤᏙᎯᏳ', 'ຈິງ', 'ᠦᠨᠡᠨ',
    ],
    "Try": [
        'try', 'प्रयास', 'চেষ্টা', 'முயற்சி', 'ప్రయత్నించు', 'પ્રયાસ', 'ਕੋਸ਼ਿਸ਼', 'ಪ್ರಯತ್ನ',
        'ശ്രമിക്കുക', 'ପ୍ରୟାସ', 'උත්සාහ', 'کوشش', 'تلاش', 'هڅه', 'попробуй', 'intentar',
        'essayer', '試行', '尝试', '시도', 'versuchen', 'tentar', 'coba', 'δοκιμή', 'נסה', 'tentare',
        'حاول', 'spróbuj', 'sprobuj', 'dene', 'cuba', 'jaribu', 'thử', 'încearcă', 'incearca',
        'probeer', 'ลอง', 'próbáld', 'probald', 'zkus', 'skús', 'skus', 'kokeile', 'försök',
        'forsok', 'subukan', 'prøv', 'prov', 'proev', 'փորձել', 'სცადე', 'prova', 'gbiyanju',
        'gwadawa', 'ព្យាយាម', 'ကြိုးစား', 'ሞክር', 'འབད', 'ᎠᏓᎫᏓᏛᏍᎩ', 'ລອງ', 'ᠣᠷᠣᠯᠳᠤ',
    ],
    "Type": [
        'type', 'प्रकार', 'প্রকার', 'வகை', 'రకం', 'પ્રકાર', 'ਕਿਸਮ', 'ಪ್ರಕಾರ', 'തരം', 'ପ୍ରକାର',
        'වර්ගය', 'قسم', 'نوع', 'ډول', 'тип', 'tipo', '型', '类型', '타입', 'typ', 'tipe', 'jenis',
        'τύπος', 'סוג', 'טיפוס', 'tip', 'aina', 'kiểu', 'ชนิด', 'típus', 'tipus', 'tyyppi',
        'uri', 'տեսակ', 'ტიპი', 'irú', "nau'i", 'iri', 'ប្រភេទ', 'အမျိုးအစား', 'አይነት', 'རིགས',
        'ᎢᏳᏓᎴᎩ', 'ປະເພດ', 'ᠬᠡᠯᠪᠡᠷᠢ',
    ],
    "U16": ['u16', 'अहस्ताक्षरित१६'],
    "U32": ['u32', 'अहस्ताक्षरित३२'],
    "U64": ['u64', 'अहस्ताक्षरित६४'],
    "U8": ['u8', 'अहस्ताक्षरित८'],
    "Unsafe": [
        'unsafe', 'असुरक्षित', 'অসুরক্ষিত', 'பாதுகாப்பற்ற', 'అసురక్షిత', 'અસુરક્ષિત',
        'ਅਸੁਰੱਖਿਅਤ', 'ಅಸುರಕ್ಷಿತ', 'അസുരക്ഷിതം', 'ଅସୁରକ୍ଷିତ', 'අනාරක්ෂිත', 'غیرمحفوظ', 'ناامن',
        'небезопасно', 'inseguro', 'dangereux', '危険', '不安全', '위험', 'unsicher', 'bahaya',
        'επικίνδυνο', 'מסוכן', 'insicuro', 'غير_آمن', 'niebezpieczny', 'güvensiz', 'guvensiz',
        'tidakselamat', 'hatari', 'không_an_toàn', 'nesigur', 'onveilig', 'ไม่ปลอดภัย',
        'veszélyes', 'veszelyes', 'nebezpečný', 'nebezpecny', 'vaarallinen', 'osäker',
        'osaker', 'mapanganib', 'usikker', 'անապահով', 'სახიფათო', 'insegur', 'àìláàbò',
        'kasada', 'មិនមានសុវត្ថិភាព', 'ဘေးကင်းမှု', 'አደገኛ', 'ཉེན་ཁ', 'ᎠᏂᏍᎦᏂᎩᏛ', 'ບໍ່ປອດໄພ',
        'ᠠᠶᠤᠯᠲᠠᠢ',
    ],
    "Use": [
        'use', 'उपयोग', 'ব্যবহার', 'பயன்படுத்து', 'ఉపయోగించు', 'વાપરો', 'ਵਰਤੋ', 'ಬಳಸಿ',
        'ഉപയോഗിക്കുക', 'ବ୍ୟବହାର', 'භාවිතා', 'استعمال', 'استفاده', 'وکاروه', 'использовать',
        'usar', 'utiliser', '使用', '사용', 'verwenden', 'pakai', 'χρήση', 'השתמש', 'usare',
        'استخدم', 'użyj', 'uzyj', 'kullan', 'guna', 'tumia', 'sử_dụng', 'folosește',
        'foloseste', 'gebruik', 'ใช้', 'használd', 'hasznald', 'použij', 'pouzij', 'použi',
        'pouzi', 'käytä', 'kayta', 'använd', 'anvand', 'gamitin', 'bruk', 'brug', 'օգտագործել',
        'გამოყენება', 'usa', 'lò', 'amfani', 'ប្រើ', 'သုံး', 'ተጠቀም', 'བཀོལ', 'ᎬᏙᏗ', 'ໃຊ້',
        'ᠬᠡᠷᠡᠭᠯᠡ',
    ],
    "Vec": ['Vec', 'सूची'],
    "Vec128": ['vec128'],
    "Vec256": ['vec256'],
    "Vec512": ['vec512'],
    "Where": [
        'where', 'जहाँ', 'यत्र', 'जिथे', 'যেখানে', 'எங்கே', 'ఎక్కడ', 'જ્યાં', 'ਜਿੱਥੇ', 'ಎಲ್ಲಿ',
        'എവിടെ', 'କେଉଁଠାରେ', 'කොහෙද', 'جہاں', 'کجا', 'چیرته', 'где', 'donde', 'où', 'ou',
        'ここで', '其中', '여기서', 'wo', 'onde', 'dimana', 'όπου', 'איפה', 'dove', 'حيث', 'gdzie',
        'nerede', 'tempat', 'wapi', 'ở_đâu', 'unde', 'waar_is', 'ที่ไหน', 'ahol', 'kde',
        'missä', 'missa', 'där', 'der', 'saan', 'hvor', 'որտեղ', 'სად', 'on', 'ibo', 'ina',
        'ណា', 'ဘယ်မှာ', 'የት', 'གང', 'ᎭᏢ', 'ບ່ອນທີ່', 'ᠬᠠᠮᠢᠭ\u180eᠠ',
    ],
    "While": [
        'while', 'यावत्', 'जबतक', 'जोपर्यंत', 'যতক্ষণ', 'வரை', 'వరకు', 'જ્યારે', 'ਜਦੋਂ', 'ತನಕ',
        'വരെ', 'ଯେତେବେଳେ', 'තෙක්', 'دوران', 'تا', 'ترڅو', 'пока', 'mientras', 'tantque', 'の間',
        '間', '当', '동안', 'während', 'solange', 'enquanto', 'selama', 'όσο', 'כאשר', 'mentre',
        'بينما', 'dopóki', 'dopoki', 'iken', 'wakati', 'trong_khi', 'cât_timp', 'cat_timp',
        'zolang', 'ขณะที่', 'amíg', 'amig', 'dokud', 'pokiaľ', 'kým', 'pokial', 'kun', 'medan',
        'habang', 'mens', 'քանի', 'სანამ', 'nígbà', 'yayin', 'ខណៈ', 'နေစဉ်', 'ሲ', 'བར', 'ᏰᎵᏊ',
        'ໃນຂະນະທີ່', 'ᠶᠠᠭ\u180eᠠ',
    ],
    "With": [
        'with', 'सह', 'সহ', 'உடன்', 'తో', 'સાથે', 'ਨਾਲ', 'ಜೊತೆ', 'കൂടെ', 'ସହିତ', 'සමඟ', 'ساتھ',
        'با', 'سره', 'совместно', 'con', 'avec', 'と', '与', '함께', 'mit', 'com', 'dengan', 'με',
        'עם', 'مع', 'razem', 'ile', 'na', 'với', 'cu', 'met', 'กับ', 'együtt', 'egyutt',
        'spolu', 'kanssa', 'med', 'kasama', 'հետ', 'თან', 'amb', 'pẹ̀lú', 'tare', 'ជាមួយ',
        'နှင့်အတူ', 'ጋር', 'དང', 'ᎠᎴ', 'ກັບ', 'ᠬᠠᠮᠲᠤ',
    ],
}
# END ALL_SYNONYMS


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
    """Every spelling recognized as a keyword, mapped to (TokenKind, lang).
    Seeded from ALL_SYNONYMS first (every word lexer.rs actually accepts
    for a TokenKind, across every dialect AND English's own aliases like
    `give`/`give_back`/`record`/`trait`/`impl`) so that translating FROM a
    source file isn't limited to recognizing only ALIASES's single
    curated "canonical" spelling per language -- a real file may
    legitimately use any lexer.rs-valid synonym (e.g. Danish's ASCII
    "formaal" alongside native "formål"; both are real Intent keywords,
    but ALIASES only ever names one as canonical). ALIASES entries are
    layered on top only to guarantee every canonical spelling is present
    even for a TokenKind ALL_SYNONYMS might not have (there shouldn't be
    any, but this keeps the invariant explicit rather than assumed). The
    `lang` half of each tuple is unused by every caller -- only `[0]`
    (the TokenKind) is ever read -- so an ALL_SYNONYMS-seeded entry's
    lang field is a placeholder, not the true source language."""
    rev: Dict[str, Tuple[str, str]] = {}
    for kind, words in ALL_SYNONYMS.items():
        for word in words:
            rev[word] = (kind, "")
    for kind, langs in ALIASES.items():
        for lang, spelling in langs.items():
            rev[spelling] = (kind, lang)
    return rev


_PRAGMA_TAG_TO_LANG: Dict[str, str] = {v: k for k, v in _PRAGMA_TAG_OVERRIDES.items()}


def detect_pragma_lang(source: str) -> Optional[str]:
    """Return the language declared in the first `// vani-lang: <name>` pragma, or None."""
    for line in source.splitlines():
        stripped = line.lstrip("/").strip()
        for prefix in ("vani-lang:", "vani-lang :"):
            if stripped.startswith(prefix):
                lang = stripped[len(prefix):].strip().lower()
                if lang in SUPPORTED_LANGS:
                    return lang
                if lang in _PRAGMA_TAG_TO_LANG:
                    return _PRAGMA_TAG_TO_LANG[lang]
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
    pragma_written = False
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
                out.append(f"{leading}vani-lang: {pragma_tag(target_lang)}")
                pragma_written = True
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
    result = "".join(out)
    # Regression (2026-08-12): a source file with no `// vani-lang:`
    # pragma at all (the common case for English -- it's the implicit
    # default, so example files never bother declaring it) previously
    # translated every keyword correctly but left the OUTPUT with no
    # pragma either. For any pragma-gated ASCII-only target dialect
    # (Swahili, Spanish, French, Malay, ...) that's fatal: the compiler
    # never activates that dialect's keyword table without the pragma,
    # so every translated keyword comes back as an unrecognized
    # identifier. Native-script targets recognize their keywords
    # unconditionally so this omission was harmless for them, which is
    # why the gap went unnoticed for a while. Prepend one whenever the
    # source had none and the target isn't English (which still needs
    # no pragma, matching every existing pragma-less English example).
    if not pragma_written and target_lang != "english":
        result = f"// vani-lang: {pragma_tag(target_lang)}\n" + result
    return result


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


def _find_vanic_binary() -> Optional[str]:
    """Locate a `vanic` binary for the optional --verify compile check:
    PATH first, then this repo's own target/{release,debug}/vanic (dev
    workflow -- tools/ lives at the repo root)."""
    import shutil

    found = shutil.which("vanic")
    if found:
        return found
    repo_root = Path(__file__).resolve().parent.parent
    for profile in ("release", "debug"):
        candidate = repo_root / "target" / profile / "vanic"
        if candidate.is_file():
            return str(candidate)
    return None


def _compile_check(vanic: str, text: str) -> Tuple[bool, str]:
    """Run `vanic check` on `text`. Returns (ok, first-line-of-error)."""
    import subprocess
    import tempfile

    with tempfile.NamedTemporaryFile(
        mode="w", suffix=".vani", delete=False, encoding="utf-8"
    ) as tmp:
        tmp.write(text)
        tmp_path = tmp.name
    try:
        result = subprocess.run(
            [vanic, "check", tmp_path],
            capture_output=True, text=True, timeout=30,
        )
        if result.returncode == 0:
            return True, ""
        first_line = (result.stderr or result.stdout).strip().splitlines()
        return False, first_line[0] if first_line else "(no output)"
    finally:
        Path(tmp_path).unlink(missing_ok=True)


def verify_roundtrip(source: str, target_lang: str, src_lang: Optional[str]) -> Tuple[bool, str]:
    """
    Translate source → target_lang → src_lang, then compare the
    keyword-token sequences of the original and the double-translated
    result. Returns (passed, message).

    Token-sequence equality alone has a real blind spot: it can't tell
    "every keyword translated correctly" from "some keyword silently
    stayed untranslated and round-tripped back to itself unchanged" --
    confirmed in practice (2026-08-12) on a build where an untranslated
    Swahili keyword produced non-compiling output while still reporting
    "round-trip ok" (token count/order matched -- the intermediate step
    just never touched that one word, so translating back to Swahili
    trivially matched the original). When a `vanic` binary can be found
    (PATH, or this repo's own target/release or target/debug), this
    function ALSO actually compiles the intermediate (target_lang)
    translation via `vanic check` and folds that result into the
    pass/fail verdict -- a real syntax/dialect-purity problem in the
    intermediate step fails verification even if the token sequence
    happens to still match after round-tripping back.
    """
    effective_src = src_lang or detect_pragma_lang(source) or "english"
    intermediate = translate(source, target_lang, effective_src)
    back = translate(intermediate, effective_src, target_lang)
    orig_tokens = extract_keyword_tokens(source)
    back_tokens = extract_keyword_tokens(back)
    tokens_match = orig_tokens == back_tokens

    vanic = _find_vanic_binary()
    compile_msgs: List[str] = []
    compile_ok = True
    if vanic is not None:
        # Check BOTH hops -- a corrupted OUTPUT-side spelling for the
        # return leg (target_lang -> effective_src) can produce a token
        # sequence that still matches (a wrong-but-still-recognized
        # spelling extracts as the same TokenKind) while the actual text
        # doesn't compile; checking only the forward hop would miss
        # exactly that case.
        for label, text in ((target_lang, intermediate), (f"{effective_src} (back)", back)):
            ok, msg = _compile_check(vanic, text)
            if not ok:
                compile_ok = False
                compile_msgs.append(f"  {label} failed `vanic check`: {msg}")
    else:
        compile_msgs.append("  (vanic binary not found -- skipped the compile check)")

    if tokens_match and compile_ok:
        suffix = "" if vanic is None else ", both hops compile clean"
        return True, (
            f"round-trip ok: {effective_src} -> {target_lang} -> {effective_src} "
            f"({len(orig_tokens)} keyword tokens preserved{suffix})"
        )

    diffs = list(compile_msgs)
    diffs += [
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
                f"// vani-lang: {pragma_tag(target_lang)}\n"
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
