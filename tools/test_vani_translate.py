#!/usr/bin/env python3
"""Regression suite for tools/vani_translate.py.

Added 2026-08-12 after confirming the tool was broken for a real chunk
of its claimed language coverage: `ALIASES` (its hand-maintained
keyword table) had drifted from src/lexer.rs in the exact same "silent
hand-copy staleness" shape as BUG-173 (src/lsp.rs's completion lists),
6 of the compiler's 63 dialects were missing from the tool's `--to`
list entirely, and the `_translate_keywords`/`build_reverse_lookup`
recognition logic only ever knew ALIASES's single curated "canonical"
spelling per (TokenKind, language) rather than every spelling
src/lexer.rs actually accepts -- so a real source file using any OTHER
valid synonym (e.g. Danish's ASCII "formaal" alongside native
"formål") silently passed through untranslated. All fixed the same
day; this script is the permanent guard against it recurring.

Checks, in order:
  1. `tools/regen_vani_translate_keywords.py --check` -- ALIASES /
     ALL_SYNONYMS must already match src/lexer.rs exactly.
  2. Every dialect under examples/language/ (one representative file
     each) translates cleanly to English and the result compiles via
     `vanic check`.
  3. `examples/language/english/basics.vani` translates cleanly INTO
     every supported non-English dialect and each result compiles.
  4. `--verify`'s round-trip check (source -> target -> source) passes
     for every dialect, including its `vanic check` compile-check of
     BOTH hops (not just token-sequence equality, which has its own
     blind spot -- see `verify_roundtrip`'s docstring).

Needs a built `vanic` binary (release or debug) to actually run;
skips steps 2-4 with a warning (not a failure) if none is found, since
`tools/regen_vani_translate_keywords.py --check` alone still catches
the most common regression (table drift) without a compiler build.

Usage:
    python3 tools/test_vani_translate.py
Exit code 0 = all checks passed, 1 = at least one failed.
"""
import importlib.util
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EXAMPLES = ROOT / "examples" / "language"
TRANSLATE_PATH = ROOT / "tools" / "vani_translate.py"
REGEN_PATH = ROOT / "tools" / "regen_vani_translate_keywords.py"


def find_vanic() -> str | None:
    for profile in ("release", "debug"):
        candidate = ROOT / "target" / profile / "vanic"
        if candidate.is_file():
            return str(candidate)
    return None


def load_translate_module():
    spec = importlib.util.spec_from_file_location("vani_translate", TRANSLATE_PATH)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def check_compiles(vanic: str, text: str) -> tuple[bool, str]:
    import tempfile

    with tempfile.NamedTemporaryFile(
        mode="w", suffix=".vani", delete=False, encoding="utf-8"
    ) as tmp:
        tmp.write(text)
        tmp_path = tmp.name
    try:
        result = subprocess.run(
            [vanic, "check", tmp_path], capture_output=True, text=True, timeout=30,
        )
        if result.returncode == 0:
            return True, ""
        lines = (result.stderr or result.stdout).strip().splitlines()
        return False, lines[0] if lines else "(no output)"
    finally:
        Path(tmp_path).unlink(missing_ok=True)


def main() -> int:
    failures: list[str] = []

    print("[1/4] tools/regen_vani_translate_keywords.py --check")
    result = subprocess.run(
        [sys.executable, str(REGEN_PATH), "--check"], capture_output=True, text=True,
    )
    if result.returncode != 0:
        failures.append(
            "ALIASES/ALL_SYNONYMS are stale relative to src/lexer.rs -- run "
            "tools/regen_vani_translate_keywords.py:\n" + result.stdout
        )
    else:
        print("  OK")

    vanic = find_vanic()
    if vanic is None:
        print("[2-4/4] SKIPPED -- no vanic binary found "
              "(build with `cargo build --release --bin vanic` first)")
        if failures:
            print("\n".join(failures))
            return 1
        print("Done (partial -- table-staleness check only).")
        return 0

    mod = load_translate_module()
    langs = [l for l in mod.SUPPORTED_LANGS if l != "english"]

    print(f"[2/4] {len(langs)} dialects -> english (compile check)")
    for lang in langs:
        lang_dir = EXAMPLES / lang
        files = sorted(lang_dir.glob("*.vani")) if lang_dir.is_dir() else []
        if not files:
            failures.append(f"  no example .vani file found under examples/language/{lang}/")
            continue
        source = files[0].read_text(encoding="utf-8")
        translated = mod.translate(source, "english", lang)
        ok, err = check_compiles(vanic, translated)
        if not ok:
            failures.append(f"  {lang} -> english ({files[0].name}): {err}")
    print("  done" if not failures else f"  {len(failures)} failure(s) so far")

    print(f"[3/4] english -> {len(langs)} dialects (compile check)")
    en_source = (EXAMPLES / "english" / "basics.vani").read_text(encoding="utf-8")
    before = len(failures)
    for lang in langs:
        translated = mod.translate(en_source, lang, "english")
        ok, err = check_compiles(vanic, translated)
        if not ok:
            failures.append(f"  english -> {lang}: {err}")
    print("  done" if len(failures) == before else f"  {len(failures) - before} new failure(s)")

    print(f"[4/4] --verify round-trip for {len(langs)} dialects")
    before = len(failures)
    for lang in langs:
        lang_dir = EXAMPLES / lang
        files = sorted(lang_dir.glob("*.vani")) if lang_dir.is_dir() else []
        if not files:
            continue
        source = files[0].read_text(encoding="utf-8")
        ok, msg = mod.verify_roundtrip(source, "english", lang)
        if not ok:
            failures.append(f"  --verify {lang} ({files[0].name}): {msg}")
    print("  done" if len(failures) == before else f"  {len(failures) - before} new failure(s)")

    if failures:
        print(f"\n{len(failures)} failure(s):")
        print("\n".join(failures))
        return 1
    print("\nAll checks passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
