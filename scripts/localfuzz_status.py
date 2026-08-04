#!/usr/bin/env python3
"""On-demand daily briefing for the tools/localfuzz pipeline.

Run from the main vani-compiler checkout:

    python3 scripts/localfuzz_status.py

Meant to be the FIRST thing a Claude Code session runs at the start of a
bug-fixing pass on localfuzz output. It does not fix anything or write to
`main` -- it locates the localfuzz worktree (via `git worktree list`, no
hardcoded path), refreshes the dedup digest (cheap -- digest.py itself only
processes findings newer than its own state marker), and prints a single
report: pipeline health, whether last night's refresh succeeded, and the
digest split into "needs a real look" vs "already flagged as fixed on
main" clusters, each with the exact file to open next.

Nothing here mutates `main`. The one write is digest.py's own (it commits
its output to the local-fuzz-findings branch in the OTHER worktree, same
as it does every night via the timer -- see tools/localfuzz/README.md).
Pass --no-digest to skip even that and just report the last cached state.
"""
import subprocess
import sys
import re
import datetime
from pathlib import Path

MAIN_REPO = Path(__file__).resolve().parent.parent


def sh(cmd, cwd=None, timeout=30):
    try:
        r = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True, timeout=timeout)
        return r.returncode, r.stdout, r.stderr
    except Exception as e:
        return 1, "", str(e)


def find_localfuzz_worktree() -> Path | None:
    rc, out, _ = sh(["git", "worktree", "list", "--porcelain"], cwd=MAIN_REPO)
    if rc != 0:
        return None
    blocks = out.strip().split("\n\n")
    for block in blocks:
        lines = dict(l.split(" ", 1) for l in block.splitlines() if " " in l)
        if lines.get("branch", "").endswith("local-fuzz-findings"):
            return Path(block.splitlines()[0].split(" ", 1)[1])
    return None


def service_status(name: str) -> str:
    rc, out, _ = sh(["systemctl", "--user", "show", name,
                      "-p", "ActiveState", "-p", "SubState"])
    if rc != 0 or not out.strip():
        return "not found"
    d = dict(l.split("=", 1) for l in out.strip().splitlines() if "=" in l)
    return f"{d.get('ActiveState', '?')}/{d.get('SubState', '?')}"


def timer_next_run(name: str) -> str:
    rc, out, _ = sh(["systemctl", "--user", "list-timers", name, "--no-pager"])
    lines = [l for l in out.splitlines() if name in l]
    return lines[0].split(name)[0].strip() if lines else "not scheduled"


def highest_bug_n(text: str) -> int:
    nums = [int(m) for m in re.findall(r"BUG-(\d+)", text)]
    return max(nums) if nums else 0


def main():
    print("=" * 70)
    print(f"localfuzz daily briefing -- {datetime.datetime.now().isoformat(timespec='seconds')}")
    print("=" * 70)

    wt = find_localfuzz_worktree()
    if wt is None:
        print("\nNo local-fuzz-findings worktree found (git worktree list). "
              "Nothing to report -- see tools/localfuzz/README.md's setup section.")
        sys.exit(1)
    print(f"\nWorktree: {wt}")

    lf = wt / "tools" / "localfuzz"

    # --- service health ---
    print("\n--- pipeline health ---")
    for svc in ("vani-localfuzz-harness", "vani-localfuzz-ollama"):
        print(f"  {svc}: {service_status(svc)}")
    for timer in ("vani-localfuzz-refresh.timer", "vani-localfuzz-digest.timer"):
        print(f"  {timer}: next run {timer_next_run(timer)}")

    # --- last refresh outcome ---
    print("\n--- last refresh.sh run ---")
    refresh_log = lf / "refresh.log"
    if refresh_log.exists():
        tail = refresh_log.read_text().strip().splitlines()[-12:]
        last_block_start = next(
            (i for i in range(len(tail) - 1, -1, -1) if "refresh starting" in tail[i]), 0
        )
        block = tail[last_block_start:]
        for line in block:
            print(f"  {line}")
        joined = "\n".join(block)
        if "ABORT" in joined or "BUILD FAILED" in joined or "FATAL" in joined:
            print("  *** last refresh did NOT complete cleanly -- fix this before trusting "
                  "anything below, the harness may be running a stale or missing binary ***")
    else:
        print("  no refresh.log yet -- refresh.sh has never run in this worktree")

    # --- refresh the digest (cheap: only processes findings newer than its marker) ---
    print("\n--- digest ---")
    if "--no-digest" not in sys.argv:
        rc, out, err = sh(["python3", "digest.py"], cwd=lf, timeout=60)
        print(f"  {out.strip() or err.strip()}")
    else:
        print("  (--no-digest passed, using cached DIGEST_LATEST.md as-is)")

    digest_path = lf / "DIGEST_LATEST.md"
    if not digest_path.exists():
        print("  no DIGEST_LATEST.md yet")
    else:
        text = digest_path.read_text()
        clusters = text.split("\n## ")
        needs_review, already_flagged = [], []
        for c in clusters[1:]:
            c = "## " + c
            (already_flagged if "possible match: BUG-" in c else needs_review).append(c)

        print(f"\n  {len(needs_review)} cluster(s) need a real look, "
              f"{len(already_flagged)} auto-flagged as possibly already fixed on main.")

        if needs_review:
            print("\n--- START HERE: unmatched clusters ---")
            for c in needs_review:
                lines = c.strip().splitlines()
                print(f"\n  {lines[0]}")
                for l in lines[1:]:
                    if l.startswith("- example:") or l.startswith("- C stderr") \
                            or l.startswith("- LLVM stderr"):
                        print(f"    {l}")

        if already_flagged:
            print("\n--- lower priority: already flagged, spot-check only ---")
            for c in already_flagged:
                lines = c.strip().splitlines()
                flag = next((l for l in lines if "possible match" in l), "")
                print(f"  {lines[0]}  |  {flag.strip('- ')}")

    # --- BUG-N numbering, fresh ---
    print("\n--- BUG-N numbering on main (fetch fresh before writing a new entry) ---")
    sh(["git", "fetch", "origin", "main"], cwd=MAIN_REPO, timeout=30)
    behind = sh(["git", "rev-list", "--count", "HEAD..origin/main"], cwd=MAIN_REPO)[1].strip()
    if behind and behind != "0":
        print(f"  *** local main is {behind} commit(s) behind origin/main -- pull before "
              f"picking a BUG-N or you may collide with the concurrent process ***")
    todo = sh(["git", "show", "origin/main:docs/TODO_CURRENT.md"], cwd=MAIN_REPO)[1]
    changelog = sh(["git", "show", "origin/main:CHANGELOG.md"], cwd=MAIN_REPO)[1]
    highest = max(highest_bug_n(todo), highest_bug_n(changelog))
    print(f"  highest BUG-N on origin/main: BUG-{highest} (next free: BUG-{highest + 1}, "
          f"re-check before committing -- see feedback_vani_concurrent_localfuzz_process memory)")

    print("\n" + "=" * 70)
    print("Next: for each unmatched cluster above, cd into the worktree, refresh.sh if the "
          "log looked stale, reproduce findings/<name>/repro.vani, root-cause it against "
          "the REAL main checkout, and only then write BUG-N into docs/TODO_CURRENT.md.")
    print("=" * 70)


if __name__ == "__main__":
    main()
