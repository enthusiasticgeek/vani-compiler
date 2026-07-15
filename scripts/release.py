#!/usr/bin/env python3
"""
scripts/release.py — one-command release automation for vāṇī.

Usage:
    python3 scripts/release.py <new-version>   # e.g. 0.6.0 or 0.5.1
    python3 scripts/release.py --patch         # auto-increment patch
    python3 scripts/release.py --minor         # auto-increment minor
    python3 scripts/release.py --major         # auto-increment major

What it does (mirrors RELEASING.md steps 2–9):
  1. Bumps Cargo.toml to <new-version>
  2. Runs scripts/sync_version.py to sync all doc version strings
  3. Scaffolds RELEASE_NOTES/v<new-version>.md if it doesn't exist
  4. Prepends a stub entry to CHANGELOG.md
  5. Commits: "chore: bump version to <new-version>"
  6. Tags:   git tag -a v<new-version> -m "Release <new-version>"
  7. Pushes the tag (triggers release.yml binary builds)
  8. Attempts cargo publish (skips gracefully if no token)
  9. Bumps Cargo.toml to <new-version-patch+1>-dev, syncs, commits, pushes main

Flags:
  --dry-run   Print every shell command without executing it.
  --no-push   Skip git push and cargo publish (for local testing).
  --no-publish Skip cargo publish only.
"""

import argparse
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).parent.parent


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def run(cmd: list[str], *, dry_run: bool = False, check: bool = True) -> str:
    """Run a command, print it, return stdout. Honour --dry-run."""
    print("  $", " ".join(cmd))
    if dry_run:
        return ""
    result = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)
    if check and result.returncode != 0:
        print(result.stdout)
        print(result.stderr)
        sys.exit(f"Command failed: {' '.join(cmd)}")
    return result.stdout.strip()


def cargo_version() -> str:
    text = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
    m = re.search(r'^version\s*=\s*"([^"]+)"', text, re.MULTILINE)
    if not m:
        sys.exit("ERROR: could not parse version from Cargo.toml")
    return m.group(1)


def set_cargo_version(ver: str) -> None:
    path = ROOT / "Cargo.toml"
    text = path.read_text(encoding="utf-8")
    new_text, n = re.subn(
        r'^(version\s*=\s*")[^"]+"',
        f'\\g<1>{ver}"',
        text,
        count=1,
        flags=re.MULTILINE,
    )
    if n == 0:
        sys.exit("ERROR: could not replace version in Cargo.toml")
    path.write_text(new_text, encoding="utf-8")
    print(f"  Cargo.toml → {ver}")


def latest_git_tag() -> str | None:
    out = run(["git", "tag", "--sort=-version:refname"], dry_run=False, check=False)
    for line in out.splitlines():
        t = line.strip().lstrip("v")
        if re.match(r"^\d+\.\d+\.\d+", t):
            return t
    return None


def bump_version(current: str, part: str) -> str:
    # Strip any pre-release suffix (e.g. "0.5.1-dev" → "0.5.1")
    base = current.split("-")[0]
    major, minor, patch = (int(x) for x in base.split("."))
    if part == "major":
        return f"{major + 1}.0.0"
    if part == "minor":
        return f"{major}.{minor + 1}.0"
    return f"{major}.{minor}.{patch + 1}"


def dev_version(release: str) -> str:
    major, minor, patch = (int(x) for x in release.split("."))
    return f"{major}.{minor}.{patch + 1}-dev"


def scaffold_release_notes(ver: str, dry_run: bool) -> Path:
    notes_dir = ROOT / "RELEASE_NOTES"
    notes_dir.mkdir(exist_ok=True)
    path = notes_dir / f"v{ver}.md"
    if path.exists():
        print(f"  RELEASE_NOTES/v{ver}.md already exists — skipping scaffold")
        return path
    template = f"""\
# vāṇī v{ver} — YYYY-MM-DD

<!-- Replace this file with a human-readable summary before pushing the tag. -->
<!-- The release workflow uses this via body_path; if absent it auto-generates. -->

## What's new

- TODO: summarise added features.

## Bug fixes

- TODO: summarise fixes.

## Upgrade notes

- No breaking changes.
"""
    if not dry_run:
        path.write_text(template, encoding="utf-8")
    print(f"  scaffolded RELEASE_NOTES/v{ver}.md")
    return path


def prepend_changelog(ver: str, dry_run: bool) -> None:
    path = ROOT / "CHANGELOG.md"
    if not path.exists():
        if not dry_run:
            path.write_text("# Changelog\n\n", encoding="utf-8")
    text = path.read_text(encoding="utf-8")
    stub = f"""\
## [v{ver}] — YYYY-MM-DD

### Added

- TODO

### Fixed

- TODO

---

"""
    # Only prepend if this version block isn't already there
    if f"## [v{ver}]" in text:
        print(f"  CHANGELOG.md already has v{ver} entry — skipping")
        return
    # Insert after the header block (first blank line after the title)
    insertion_re = re.compile(r"(# Changelog.*?\n\n)", re.DOTALL)
    new_text, n = insertion_re.subn(r"\1" + stub, text, count=1)
    if n == 0:
        new_text = stub + text
    if not dry_run:
        path.write_text(new_text, encoding="utf-8")
    print("  CHANGELOG.md — stub prepended")


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main() -> None:
    parser = argparse.ArgumentParser(description="Cut a vāṇī release.")
    group = parser.add_mutually_exclusive_group()
    group.add_argument("version", nargs="?", help="Explicit new version (e.g. 0.6.0)")
    group.add_argument("--patch", action="store_true")
    group.add_argument("--minor", action="store_true")
    group.add_argument("--major", action="store_true")
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--no-push", action="store_true")
    parser.add_argument("--no-publish", action="store_true")
    args = parser.parse_args()

    dry = args.dry_run

    # Determine new version
    current = cargo_version()
    latest_tag = latest_git_tag()
    base = latest_tag or current.split("-")[0]
    print(f"Current Cargo.toml: {current}   Latest tag: v{latest_tag or 'none'}")

    if args.version:
        new_ver = args.version.lstrip("v")
    elif args.patch:
        new_ver = bump_version(base, "patch")
    elif args.minor:
        new_ver = bump_version(base, "minor")
    elif args.major:
        new_ver = bump_version(base, "major")
    else:
        sys.exit("Specify a version: <x.y.z> | --patch | --minor | --major")

    print(f"\nReleasing v{new_ver}  (dry_run={dry})\n")

    # Step 1 — bump Cargo.toml
    print("[1/9] Bump Cargo.toml")
    if not dry:
        set_cargo_version(new_ver)

    # Step 2 — sync docs
    print("[2/9] Sync version strings")
    run(["python3", "scripts/sync_version.py"], dry_run=dry)

    # Step 3 — release notes
    print("[3/9] Scaffold release notes")
    notes_path = scaffold_release_notes(new_ver, dry)

    # Step 4 — changelog
    print("[4/9] Update CHANGELOG.md")
    prepend_changelog(new_ver, dry)

    # Step 5 — commit
    print("[5/9] Commit")
    changed = run(["git", "diff", "--name-only"], dry_run=False)
    untracked_notes = str(notes_path.relative_to(ROOT)).replace("\\", "/")
    files_to_add = [
        "Cargo.toml", "Cargo.lock", "RELEASING.md", "TODO.md",
        "CHANGELOG.md", untracked_notes,
    ]
    run(["git", "add"] + files_to_add, dry_run=dry)
    run(
        ["git", "commit", "-m", f"chore: bump version to {new_ver}\n\nCo-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"],
        dry_run=dry,
    )

    # Step 6 — tag
    print("[6/9] Tag")
    run(["git", "tag", "-a", f"v{new_ver}", "-m", f"Release {new_ver}"], dry_run=dry)

    # Step 7 — push tag
    print("[7/9] Push tag")
    if not args.no_push:
        run(["git", "push", "origin", f"v{new_ver}"], dry_run=dry)
    else:
        print("  --no-push: skipping git push tag")

    # Step 8 — cargo publish
    print("[8/9] cargo publish")
    if not args.no_push and not args.no_publish:
        result = subprocess.run(
            ["cargo", "publish"],
            cwd=ROOT,
            capture_output=True,
            text=True,
        )
        if result.returncode != 0:
            print(f"  WARNING: cargo publish failed (no token?). Run manually:\n"
                  f"    cargo login <TOKEN> && cargo publish")
        else:
            print("  Published to crates.io")
    else:
        print("  skipping cargo publish")

    # Step 9 — post-release dev bump
    print("[9/9] Post-release dev bump")
    dev_ver = dev_version(new_ver)
    if not dry:
        set_cargo_version(dev_ver)
    run(["python3", "scripts/sync_version.py"], dry_run=dry)
    dev_files = ["Cargo.toml", "Cargo.lock", "RELEASING.md", "TODO.md"]
    run(["git", "add"] + dev_files, dry_run=dry)
    run(
        ["git", "commit", "-m", f"chore: bump to {dev_ver}\n\nCo-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"],
        dry_run=dry,
    )
    if not args.no_push:
        run(["git", "push", "origin", "main"], dry_run=dry)
    else:
        print("  --no-push: skipping git push main")

    print(f"\nDone. v{new_ver} released; main is now at {dev_ver}.")
    if args.no_push or args.no_publish:
        print("Remember to: git push origin main  &&  cargo publish")


if __name__ == "__main__":
    main()
