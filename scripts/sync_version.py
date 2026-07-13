#!/usr/bin/env python3
"""
scripts/sync_version.py — single source of truth for the project version.

Reads the version from Cargo.toml and updates every doc that embeds it.
Run this whenever Cargo.toml version changes, or let the pre-commit hook
do it automatically.

Usage:
    python3 scripts/sync_version.py            # sync + print what changed
    python3 scripts/sync_version.py --check    # exit 1 if any doc is stale
    python3 scripts/sync_version.py --verify   # also warn if Cargo.toml
                                               # is behind the latest git tag
"""

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).parent.parent


def cargo_version() -> str:
    text = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
    m = re.search(r'^version\s*=\s*"([^"]+)"', text, re.MULTILINE)
    if not m:
        sys.exit("ERROR: could not parse version from Cargo.toml")
    return m.group(1)


def latest_git_tag() -> str | None:
    try:
        out = subprocess.check_output(
            ["git", "tag", "--sort=-version:refname"],
            cwd=ROOT, text=True, stderr=subprocess.DEVNULL
        ).strip()
        for line in out.splitlines():
            t = line.strip().lstrip("v")
            if re.match(r"^\d+\.\d+\.\d+", t):
                return t
    except subprocess.CalledProcessError:
        pass
    return None


def patch(path: Path, pattern: str, replacement: str, label: str) -> bool:
    """Replace first match of `pattern` with `replacement`. Returns True if changed."""
    text = path.read_text(encoding="utf-8")
    new_text, n = re.subn(pattern, replacement, text, count=1)
    if n == 0:
        print(f"  WARN  {label}: pattern not found in {path.relative_to(ROOT)}")
        return False
    if new_text == text:
        return False  # already correct
    path.write_text(new_text, encoding="utf-8")
    print(f"  FIXED {label}: {path.relative_to(ROOT)}")
    return True


def check(path: Path, pattern: str, label: str) -> bool:
    """Return True if the pattern matches (doc is up to date)."""
    text = path.read_text(encoding="utf-8")
    return bool(re.search(pattern, text))


def main() -> None:
    check_only = "--check" in sys.argv
    verify_tags = "--verify" in sys.argv or "--check" in sys.argv

    ver = cargo_version()

    # Optional: warn when Cargo.toml lags behind the latest git tag
    if verify_tags:
        latest = latest_git_tag()
        if latest and latest != ver:
            from packaging.version import Version  # type: ignore
            try:
                if Version(latest) > Version(ver):
                    print(
                        f"WARNING: Cargo.toml is at {ver} but latest git tag is "
                        f"v{latest}. Run:\n"
                        f"  git tag --sort=-version:refname | head -3\n"
                        f"and bump Cargo.toml to the correct next version before "
                        f"releasing."
                    )
            except Exception:
                pass  # packaging not installed; skip comparison

    # --- Targets -----------------------------------------------------------
    # Each entry: (file, search-regex, replacement, label)
    # Use a lambda so `ver` is substituted at call time.
    targets = [
        (
            ROOT / "RELEASING.md",
            r"\*\*Current version: `[^`]+`\*\*",
            f"**Current version: `{ver}`**",
            "RELEASING.md current-version line",
        ),
        (
            ROOT / "TODO.md",
            r"- \*\*Version\*\*: `[^`]+`[^\n]*",
            f"- **Version**: `{ver}` (tagged v0.1.0 through v{ver}; see RELEASING.md for full history).",
            "TODO.md version line",
        ),
    ]

    stale = []
    for path, pattern, replacement, label in targets:
        if not path.exists():
            print(f"  SKIP  {label}: file not found")
            continue
        if check_only:
            if not check(path, re.escape(replacement).replace(r"\ ", " "), label):
                stale.append(label)
                print(f"  STALE {label}: {path.relative_to(ROOT)}")
            else:
                print(f"  OK    {label}")
        else:
            patch(path, pattern, replacement, label)

    if check_only and stale:
        print(
            f"\n{len(stale)} doc(s) have a stale version string.\n"
            "Run:  python3 scripts/sync_version.py\n"
            "then git add the changed files and retry your commit."
        )
        sys.exit(1)

    if not check_only:
        print(f"\nAll version references synced to {ver}.")


if __name__ == "__main__":
    main()
