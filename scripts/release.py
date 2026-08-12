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
  5. Refuses to continue if either of the above still has an unfilled
     "TODO" / "YYYY-MM-DD" scaffold placeholder (see check_notes_not_
     stale) -- write real content for BOTH files BEFORE running this
     script (steps 3/4 skip scaffolding when the file/entry already
     exists) so this check passes cleanly. Override with
     --allow-stale-notes if you genuinely mean to ship placeholders.
  6. Commits: "chore: bump version to <new-version>"
  7. Tags:   git tag -a v<new-version> -m "Release <new-version>"
  8. Pushes the tag (triggers release.yml binary builds)
  9. Attempts cargo publish (skips gracefully if no token)
  10. Bumps Cargo.toml to <new-version-patch+1>-dev, syncs, commits, pushes main

Flags:
  --dry-run          Print every shell command without executing it.
  --no-push          Skip git push and cargo publish (for local testing).
  --allow-stale-notes
                      Don't abort when RELEASE_NOTES/CHANGELOG still have
                      unfilled scaffold placeholders (not recommended --
                      v0.9.2 shipped with placeholder docs precisely
                      because nothing enforced this).
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


def check_notes_not_stale(ver: str) -> list[str]:
    """Return human-readable problems if RELEASE_NOTES/v<ver>.md or
    CHANGELOG.md's '## [v<ver>]' entry still contain an unfilled scaffold
    placeholder ("- TODO" line / "YYYY-MM-DD" date). Empty list means
    both are ready to ship.

    Matches the exact scaffold bullet shape (a line starting with
    "- TODO"), not a bare "TODO" substring search -- real release-note
    prose legitimately mentions filenames like `docs/TODO_CURRENT.md`,
    which a naive substring check would misfire on.

    v0.9.2 (2026-08-11) was tagged and released with BOTH files still
    holding their auto-scaffolded stub content untouched -- nothing in
    this script (or the release workflow) ever checked, so the mistake
    went unnoticed until a later pass caught it and rewrote both files
    retroactively. This function exists so that can't happen silently
    again.
    """
    problems: list[str] = []
    todo_line_re = re.compile(r"(?m)^- TODO\b")

    def stale_hits(text: str) -> list[str]:
        hits = []
        if todo_line_re.search(text):
            hits.append("a '- TODO' scaffold bullet")
        if "YYYY-MM-DD" in text:
            hits.append("the 'YYYY-MM-DD' placeholder date")
        return hits

    notes_path = ROOT / "RELEASE_NOTES" / f"v{ver}.md"
    if notes_path.exists():
        text = notes_path.read_text(encoding="utf-8")
        hits = stale_hits(text)
        if hits:
            problems.append(
                f"RELEASE_NOTES/v{ver}.md still contains {' and '.join(hits)} "
                f"-- replace the scaffolded template with real content."
            )
    else:
        problems.append(f"RELEASE_NOTES/v{ver}.md does not exist.")

    changelog_path = ROOT / "CHANGELOG.md"
    text = changelog_path.read_text(encoding="utf-8") if changelog_path.exists() else ""
    m = re.search(rf"## \[v{re.escape(ver)}\].*?(?=\n## \[|\Z)", text, re.DOTALL)
    if not m:
        problems.append(f"CHANGELOG.md has no '## [v{ver}]' entry.")
    else:
        hits = stale_hits(m.group(0))
        if hits:
            problems.append(
                f"CHANGELOG.md's '## [v{ver}]' entry still contains "
                f"{' and '.join(hits)} -- replace the scaffolded "
                f"stub with real content."
            )
    return problems


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
    parser.add_argument(
        "--allow-stale-notes",
        action="store_true",
        help="Ship even if RELEASE_NOTES/CHANGELOG still have unfilled "
             "scaffold placeholders (not recommended).",
    )
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
    print("[1/10] Bump Cargo.toml")
    if not dry:
        set_cargo_version(new_ver)

    # Step 2 — sync docs
    print("[2/10] Sync version strings")
    run(["python3", "scripts/sync_version.py"], dry_run=dry)

    # Step 3 — release notes
    print("[3/10] Scaffold release notes")
    notes_path = scaffold_release_notes(new_ver, dry)

    # Step 4 — changelog
    print("[4/10] Update CHANGELOG.md")
    prepend_changelog(new_ver, dry)

    # Step 5 — refuse to ship placeholder docs
    print("[5/10] Verify release notes + changelog are filled in")
    if dry:
        print("  --dry-run: skipping (files not necessarily written)")
    else:
        problems = check_notes_not_stale(new_ver)
        if problems and not args.allow_stale_notes:
            print("\nERROR: refusing to cut a release with placeholder docs:")
            for p in problems:
                print(f"  - {p}")
            print(
                f"\nEdit RELEASE_NOTES/v{new_ver}.md and CHANGELOG.md's "
                f"'## [v{new_ver}]' entry with real content, then re-run "
                f"this script (steps 3/4 will skip scaffolding since both "
                f"already exist). Override with --allow-stale-notes if you "
                f"genuinely mean to ship placeholders."
            )
            sys.exit(1)
        elif problems:
            print("  WARNING: shipping with placeholder docs (--allow-stale-notes set):")
            for p in problems:
                print(f"    - {p}")
        else:
            print("  OK — no placeholder markers found")

    # Step 6 — commit
    print("[6/10] Commit")
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

    # Step 7 — tag
    print("[7/10] Tag")
    run(["git", "tag", "-a", f"v{new_ver}", "-m", f"Release {new_ver}"], dry_run=dry)

    # Step 8 — push tag
    print("[8/10] Push tag")
    if not args.no_push:
        run(["git", "push", "origin", f"v{new_ver}"], dry_run=dry)
    else:
        print("  --no-push: skipping git push tag")

    # Step 9 — cargo publish
    print("[9/10] cargo publish")
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

    # Step 10 — post-release dev bump
    print("[10/10] Post-release dev bump")
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
