#!/usr/bin/env python3
"""Corpus-wide differential test: does every `.vani` example produce
the same exit code on both backends `vanic run` can target?

For every `.vani` file under `examples/` (recursively) that passes
`vanic check`: run it once via `vanic run <path>` (LLVM, the default
backend) and once via `vanic run <path> --backend=c` (the tree-C
backend), each under a timeout, and compare exit codes. A file is
FLAGGED if the two backends disagree.

This targets the exact class of bug BUG-192 (2026-08-14) turned out
to be: two silent LLVM-only trap bugs found only by a human manually
diffing SSA-vs-tree-backend IR side by side. The existing curated
cross-backend tests (tests/ssa_backend_c_crosscheck.rs,
tests/ssa_backend_llvm_crosscheck.rs) cover ~15-20 hand-written
snippets; this sweeps the entire real corpus (1000+ files) the same
way tools/leak_sweep.py already does for its own (single-backend,
ASan-focused) check.

Exit codes, not stdout, are compared -- HashMap iteration order, RNG,
concurrency interleaving, and wall-clock timestamps make full stdout
comparison unsafe across a corpus this broad. Exit-code parity is the
same bar tests/ssa_backend_c_crosscheck.rs itself already uses for its
curated set.

Some flagged files are known, already-triaged findings (e.g. a
program whose behavior is legitimately backend-sensitive, or a real
bug deliberately left open with a documented reason) rather than new
regressions -- those are listed in
`tools/backend_crosscheck_baseline.json` so CI only fails on a NEW
finding, not on rediscovering an already-tracked one. Mirrors
tools/leak_sweep.py's baseline workflow exactly.

Usage:
    python3 tools/backend_crosscheck.py [--vanic PATH] [--jobs N]
                                         [--update-baseline]

Exit code: 0 if no un-baselined finding, 1 otherwise. Also exits 1 if
a PREVIOUSLY baselined finding stops reproducing -- that means the
divergence it documents may have been silently fixed (good news, but
the baseline entry should be removed) or the sweep methodology
regressed (bad news) -- either way it's worth a human looking, not a
silent pass.
"""
import argparse
import glob
import json
import os
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor, as_completed

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BASELINE_PATH = os.path.join(REPO_ROOT, "tools", "backend_crosscheck_baseline.json")


def run(cmd, timeout):
    try:
        p = subprocess.run(cmd, capture_output=True, timeout=timeout, text=True)
        return p.returncode, p.stdout, p.stderr, False
    except subprocess.TimeoutExpired:
        return None, "", "", True


def load_baseline():
    if not os.path.exists(BASELINE_PATH):
        return {}
    with open(BASELINE_PATH) as f:
        entries = json.load(f)
    return {e["file"]: e for e in entries}


def relpath(p):
    return os.path.relpath(p, REPO_ROOT)


def check_one(vanic, path, timeout_check, timeout_run):
    """Returns (relpath, status, detail) where status is one of
    'skipped', 'clean', or a finding dict, or None for a skip."""
    rc, _out, _err, to = run([vanic, "check", path], timeout=timeout_check)
    if to or rc != 0:
        return relpath(path), "skipped", None

    rc_llvm, _out, _err, to_llvm = run([vanic, "run", path], timeout=timeout_run)
    rc_c, _out, _err, to_c = run(
        [vanic, "run", path, "--backend=c"], timeout=timeout_run
    )
    if to_llvm or to_c:
        # Interactive/stdin-driven or genuinely slow example on
        # (at least) one side -- not a backend-divergence signal,
        # skip rather than flag.
        return relpath(path), "skipped", None

    if rc_llvm != rc_c:
        return (
            relpath(path),
            "finding",
            {"exit_llvm": rc_llvm, "exit_c": rc_c},
        )
    return relpath(path), "clean", None


def sweep(vanic, jobs, timeout_check=10, timeout_run=30):
    files = sorted(
        glob.glob(os.path.join(REPO_ROOT, "examples", "**", "*.vani"), recursive=True)
    )
    print(f"Total candidate files: {len(files)}  (jobs={jobs})", flush=True)

    findings = {}
    n_skipped = n_clean = 0
    start = time.time()
    done = 0

    with ThreadPoolExecutor(max_workers=jobs) as pool:
        futures = {
            pool.submit(check_one, vanic, path, timeout_check, timeout_run): path
            for path in files
        }
        for fut in as_completed(futures):
            rel, status, detail = fut.result()
            done += 1
            if status == "skipped":
                n_skipped += 1
            elif status == "finding":
                findings[rel] = detail
            else:
                n_clean += 1
            if done % 100 == 0:
                elapsed = time.time() - start
                print(
                    f"[{done}/{len(files)}] elapsed={elapsed:.0f}s "
                    f"skipped={n_skipped} clean={n_clean} flagged={len(findings)}",
                    flush=True,
                )

    print("=== DONE ===")
    print(f"Total: {len(files)}")
    print(f"Skipped (check failed, or interactive/timeout on either backend): {n_skipped}")
    print(f"Clean (exit codes agree): {n_clean}")
    print(f"FLAGGED (exit codes disagree): {len(findings)}")
    return findings


def main():
    ap = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    ap.add_argument(
        "--vanic",
        default=os.path.join(REPO_ROOT, "target", "release", "vanic"),
        help="path to the vanic binary (default: target/release/vanic)",
    )
    ap.add_argument(
        "--jobs",
        type=int,
        default=os.cpu_count() or 4,
        help="parallel worker count (default: os.cpu_count())",
    )
    ap.add_argument(
        "--update-baseline",
        action="store_true",
        help="rewrite tools/backend_crosscheck_baseline.json from this run's "
        "findings, instead of comparing against it",
    )
    args = ap.parse_args()

    if not os.path.exists(args.vanic):
        print(
            f"error: vanic binary not found at {args.vanic} -- build it first "
            f"(cargo build --release --bin vanic)",
            file=sys.stderr,
        )
        return 2

    findings = sweep(args.vanic, args.jobs)

    if args.update_baseline:
        entries = [
            {
                "file": f,
                "exit_llvm": v["exit_llvm"],
                "exit_c": v["exit_c"],
                "reason": "TODO: fill in why this divergence is known/expected",
            }
            for f, v in sorted(findings.items())
        ]
        with open(BASELINE_PATH, "w") as out:
            json.dump(entries, out, indent=2)
            out.write("\n")
        print(f"Wrote {len(entries)} entries to {relpath(BASELINE_PATH)}")
        return 0

    baseline = load_baseline()
    new_findings = {f: v for f, v in findings.items() if f not in baseline}
    stale_baseline = {f: e for f, e in baseline.items() if f not in findings}

    if new_findings:
        print(f"\n=== {len(new_findings)} NEW finding(s) not in the baseline ===")
        for f, v in sorted(new_findings.items()):
            print(f"  {f}: llvm exit={v['exit_llvm']}  c exit={v['exit_c']}")

    if stale_baseline:
        print(
            f"\n=== {len(stale_baseline)} baseline entr"
            f"{'y' if len(stale_baseline) == 1 else 'ies'} did NOT reproduce ==="
        )
        for f, e in sorted(stale_baseline.items()):
            print(
                f"  {f}: expected llvm={e['exit_llvm']} c={e['exit_c']} "
                f"({e.get('reason', '')[:100]}) -- now agrees"
            )
        print(
            "If this is because the underlying divergence got fixed, remove the "
            "entry from tools/backend_crosscheck_baseline.json. If the sweep "
            "methodology changed (e.g. a file was moved/renamed), update the "
            "entry's path instead."
        )

    if new_findings or stale_baseline:
        return 1
    print("\nAll findings match the baseline exactly. Clean.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
