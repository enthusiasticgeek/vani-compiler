#!/usr/bin/env python3
"""Dedup digest generator for tools/localfuzz/findings/.

The harness produces one raw finding.json + repro.vani per crash/divergence,
which is deliberately unopinionated (see harness.py) -- across a couple of
days that's 80+ directories, but they cluster into a handful of distinct
root-cause *signatures*. Reading all of them raw doesn't scale, for a human
or for a model. This script:

  1. Groups findings by a mechanical signature (kind, exit codes, timeout
     flags, and a coarse classification of each backend's stderr) -- the
     same grouping done by hand the first time this was needed.
  2. Cross-checks each signature against `main`'s current
     docs/TODO_CURRENT.md and CHANGELOG.md (via `git show main:<path>`,
     which only needs the shared .git dir -- already readable under the
     sandbox, no working-tree access to the main checkout required) to
     flag clusters that look like an already-fixed bug the stale binary
     is still tripping over. This is a heuristic keyword match, not proof
     -- it tells a reviewer where to look first, nothing more.
  3. Tracks which finding directories were already reported in a prior
     digest (.digest_state.json) so re-running only surfaces genuinely
     NEW findings since the last digest, not the same 80 every time.
  4. Writes a timestamped digest under tools/localfuzz/digests/ and
     refreshes DIGEST_LATEST.md to point at the newest one.

Nothing here is trusted automatically -- same rule as everything else in
this tool. A cluster flagged "not matched to a known fix" is a lead for a
human or frontier-model session, not a confirmed new bug.
"""
import json
import re
import subprocess
import sys
import datetime
import collections
from pathlib import Path

HERE = Path(__file__).resolve().parent
FINDINGS_DIR = HERE / "findings"
DIGESTS_DIR = HERE / "digests"
STATE_FILE = HERE / ".digest_state.json"
LATEST_LINK = HERE / "DIGEST_LATEST.md"


def classify_stderr(err: str) -> str:
    err = (err or "").strip()
    if not err:
        return ""
    if "PLEASE submit a bug report" in err:
        return "LLVM-INTERNAL-CRASH"
    if err.startswith("lli:") and "error:" in err:
        m = re.search(r"error: (.*)", err)
        return "LLI-PARSE-ERROR: " + (m.group(1)[:70] if m else "")
    if "Assertion" in err and "fn_main" in err:
        return "C-ASSERT-FAIL"
    if "cc failed" in err or (".c:" in err and "error:" in err):
        return "C-COMPILE-FAIL"
    if "index out of bounds" in err:
        return "RUST-PANIC: index out of bounds"
    if "overflow" in err.lower():
        return "OVERFLOW-PANIC"
    return err[:70]


def signature(finding: dict):
    kind = finding.get("kind", "?")
    c = finding.get("c", {}) or {}
    l = finding.get("llvm", {}) or {}
    return (
        kind,
        c.get("rc"),
        bool(c.get("timed_out")),
        l.get("rc"),
        bool(l.get("timed_out")),
        classify_stderr(c.get("stderr", "")),
        classify_stderr(l.get("stderr", "")),
    )


def sig_keywords(sig) -> list:
    """Pull a couple of grep-able keywords out of a signature for the
    already-fixed cross-check. Deliberately short and specific -- long
    phrases rarely match doc prose verbatim."""
    kws = []
    for part in (sig[5], sig[6]):
        if not part:
            continue
        if part.startswith("LLI-PARSE-ERROR: "):
            kws.append(part.split("LLI-PARSE-ERROR: ", 1)[1])
        elif part in ("C-ASSERT-FAIL", "C-COMPILE-FAIL", "LLVM-INTERNAL-CRASH"):
            pass  # too generic to grep on their own
        else:
            kws.append(part)
    return kws


def git_show(ref_path: str) -> str:
    try:
        out = subprocess.run(
            ["git", "show", ref_path],
            cwd=HERE, capture_output=True, text=True, timeout=15,
        )
        return out.stdout if out.returncode == 0 else ""
    except Exception:
        return ""


def load_main_docs() -> str:
    return git_show("main:docs/TODO_CURRENT.md") + "\n" + git_show("main:CHANGELOG.md")


def check_already_fixed(keywords, main_docs: str) -> str:
    if not keywords or not main_docs:
        return ""
    for kw in keywords:
        if len(kw) < 8:
            continue
        if kw in main_docs:
            # find which BUG-N section mentions it, best-effort
            idx = main_docs.find(kw)
            window = main_docs[max(0, idx - 800):idx]
            m = re.findall(r"BUG-(\d+)", window)
            bug_ref = f"BUG-{m[-1]}" if m else "(unknown BUG-N)"
            return f"possible match: {bug_ref} -- verbatim keyword {kw!r} found in main's docs"
    return ""


def load_state() -> dict:
    if STATE_FILE.exists():
        try:
            return json.loads(STATE_FILE.read_text())
        except Exception:
            pass
    return {"seen": []}


def save_state(state: dict):
    STATE_FILE.write_text(json.dumps(state, indent=2))


def main():
    only_new = "--all" not in sys.argv
    state = load_state()
    seen = set(state.get("seen", []))

    all_dirs = sorted(p for p in FINDINGS_DIR.iterdir() if p.is_dir())
    dirs = [d for d in all_dirs if not only_new or d.name not in seen]

    if not dirs:
        print("No new findings since last digest (use --all to force a full re-scan).")
        return

    main_docs = load_main_docs()

    clusters = collections.defaultdict(list)
    for d in dirs:
        fj = d / "finding.json"
        if not fj.exists():
            continue
        try:
            finding = json.loads(fj.read_text())
        except Exception:
            continue
        sig = signature(finding)
        clusters[sig].append(d.name)

    ranked = sorted(clusters.items(), key=lambda kv: -len(kv[1]))

    ts = datetime.datetime.now(datetime.timezone.utc).strftime("%Y%m%d-%H%M%S")
    DIGESTS_DIR.mkdir(exist_ok=True)
    out_path = DIGESTS_DIR / f"{ts}.md"

    lines = []
    lines.append(f"# localfuzz digest -- {ts}Z")
    lines.append("")
    lines.append(f"{len(dirs)} finding(s) in this digest "
                  f"({'new since last digest' if only_new else 'full re-scan, --all'}), "
                  f"collapsing to {len(clusters)} distinct signature(s).")
    lines.append("")
    lines.append("Signatures are mechanical (exit codes + timeout flags + coarse stderr "
                  "classification) -- same root cause can occasionally split across two "
                  "signatures, or two unrelated bugs can share one. Treat clusters as a "
                  "starting point, not ground truth.")
    lines.append("")

    for sig, members in ranked:
        kind, crc, cto, lrc, lto, cerr, lerr = sig
        kws = sig_keywords(sig)
        fixed_hint = check_already_fixed(kws, main_docs)
        lines.append(f"## [{len(members)}x] {kind} -- c.rc={crc} c.timeout={cto} "
                      f"llvm.rc={lrc} llvm.timeout={lto}")
        if cerr:
            lines.append(f"- C stderr class: `{cerr}`")
        if lerr:
            lines.append(f"- LLVM stderr class: `{lerr}`")
        if fixed_hint:
            lines.append(f"- **⚠ {fixed_hint} -- rebuild against latest main and re-verify "
                          f"before treating this as new**")
        lines.append(f"- example: `findings/{members[0]}/`")
        if len(members) > 1:
            lines.append(f"- all: {', '.join(members)}")
        lines.append("")

    out_path.write_text("\n".join(lines))
    LATEST_LINK.write_text("\n".join(lines))

    seen.update(d.name for d in dirs)
    state["seen"] = sorted(seen)
    state["last_run"] = ts
    save_state(state)

    print(f"Wrote {out_path}")
    print(f"Updated {LATEST_LINK}")
    print(f"{len(clusters)} distinct signatures from {len(dirs)} findings.")

    _commit(out_path, LATEST_LINK, STATE_FILE)


def _commit(*paths):
    """Auto-commit digest output to local-fuzz-findings, same convention as
    harness.py's own findings commits -- never touches main."""
    import os
    env = dict(os.environ)
    env.setdefault("GIT_AUTHOR_NAME", "localfuzz-digest")
    env.setdefault("GIT_AUTHOR_EMAIL", "localfuzz@localhost")
    env.setdefault("GIT_COMMITTER_NAME", "localfuzz-digest")
    env.setdefault("GIT_COMMITTER_EMAIL", "localfuzz@localhost")
    repo_root = HERE.parent.parent
    rel = [str(p.relative_to(repo_root)) for p in paths]
    subprocess.run(["git", "add"] + rel, cwd=repo_root, env=env, check=False)
    diff = subprocess.run(["git", "diff", "--cached", "--quiet"], cwd=repo_root, env=env)
    if diff.returncode == 0:
        return  # nothing changed (e.g. --all re-run with no new findings)
    subprocess.run(
        ["git", "commit", "-m", f"localfuzz: digest run {paths[0].stem}"],
        cwd=repo_root, env=env, check=False,
    )


if __name__ == "__main__":
    main()
