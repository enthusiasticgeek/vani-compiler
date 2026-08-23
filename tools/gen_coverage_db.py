#!/usr/bin/env python3
"""Generate the baked-in feature-combination coverage database
(coverage_fingerprints.json) that `vanic check --coverage` scores a
user's program against.

A fingerprint (see src/coverage.rs's doc comment for the exact
format) is only added to the database if it was extracted from an
example file that is BOTH:

  1. Accepted by `vanic check` (well-typed, passes the checker's own
     SMT verification), and
  2. Confirmed leak/bug-free by `tools/leak_sweep.py`'s own
     ASan+LeakSanitizer+UBSan sweep over the C backend -- the SAME
     sweep this repo already trusts for the leak-regression baseline
     (see leak_sweep_baseline.json). A file currently flagged by that
     sweep is excluded even if the finding is itself baselined
     (tracked-but-not-yet-fixed) -- baselined still means "known
     buggy", not "known good", and this database must only ever
     assert the latter.

This directly reuses leak_sweep.py's sweep() rather than
re-implementing ASan sweeping, so there is exactly one place that
decides what "verified clean" means for this repo's example corpus.

Usage:
    python3 tools/gen_coverage_db.py [--vanic PATH] [--out PATH]

The output is baked into the vanic binary at compile time via
include_str! (see src/coverage.rs) -- vanic itself never generates or
fetches this file at check-time, keeping the coverage feature fully
offline.
"""
import argparse
import glob
import json
import os
import subprocess
import sys
import time

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(REPO_ROOT, "tools"))
import leak_sweep  # noqa: E402  (reuses its sweep() + GCC_SANITIZE_FLAGS)

DEFAULT_OUT = os.path.join(REPO_ROOT, "coverage_fingerprints.json")


def relpath(p):
    return os.path.relpath(p, REPO_ROOT)


def extract_fingerprints(vanic, path, timeout=10):
    """Run `vanic check <path> --dump-fingerprints`, return a set of
    fingerprint strings, or None if the check itself failed (should
    not happen for a file that already passed the leak sweep's own
    `vanic check` gate, but handled defensively)."""
    try:
        p = subprocess.run(
            [vanic, "check", path, "--dump-fingerprints"],
            capture_output=True, timeout=timeout, text=True,
        )
    except subprocess.TimeoutExpired:
        return None
    if p.returncode != 0:
        return None
    lines = [ln.strip() for ln in p.stdout.splitlines() if ln.strip()]
    # Single-file --dump-fingerprints output is a flat, unindented
    # list (see main.rs) -- every non-empty line is one fingerprint.
    return set(lines)


def main():
    ap = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--vanic", default=os.path.join(REPO_ROOT, "target", "release", "vanic"),
                     help="path to the vanic binary (default: target/release/vanic)")
    ap.add_argument("--workdir", default=os.path.join(REPO_ROOT, "target", "coverage_gen"),
                     help="scratch directory for the leak sweep's emitted C + binaries")
    ap.add_argument("--out", default=DEFAULT_OUT,
                     help=f"output path for the coverage database (default: {relpath(DEFAULT_OUT)})")
    args = ap.parse_args()

    if not os.path.exists(args.vanic):
        print(f"error: vanic binary not found at {args.vanic} -- build it first "
              f"(cargo build --release --bin vanic)", file=sys.stderr)
        return 2

    print("Running leak_sweep over examples/ to determine the verified-clean file set...")
    start = time.time()
    findings = leak_sweep.sweep(args.vanic, args.workdir)
    print(f"leak_sweep finished in {time.time() - start:.0f}s, "
          f"{len(findings)} file(s) flagged (excluded from the coverage DB)")

    all_files = sorted(glob.glob(os.path.join(REPO_ROOT, "examples", "**", "*.vani"), recursive=True))
    clean_files = [f for f in all_files if relpath(f) not in findings]
    print(f"{len(clean_files)}/{len(all_files)} example files are verified clean; "
          f"extracting fingerprints from those...")

    fingerprints = set()
    n_extract_fail = 0
    for f in clean_files:
        fps = extract_fingerprints(args.vanic, f)
        if fps is None:
            n_extract_fail += 1
            continue
        fingerprints |= fps

    if n_extract_fail:
        print(f"warning: {n_extract_fail} clean file(s) failed re-check during "
              f"fingerprint extraction (unexpected -- investigate)", file=sys.stderr)

    db = {
        "format_version": 1,
        "generated_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "source_corpus": "examples/**/*.vani",
        "total_example_files": len(all_files),
        "verified_clean_files": len(clean_files),
        "fingerprint_count": len(fingerprints),
        "fingerprints": sorted(fingerprints),
    }
    with open(args.out, "w") as out:
        json.dump(db, out, indent=2)
        out.write("\n")
    print(f"Wrote {len(fingerprints)} fingerprints to {relpath(args.out)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
