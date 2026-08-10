#!/usr/bin/env python3
"""Systematic ASan + LeakSanitizer + UBSan sweep across the vāṇी
example corpus (C backend).

For every `.vani` file under `examples/` (recursively) that passes
`vanic check`: emit C, compile with sanitizers using the exact same
flags `vanic run --backend=c` itself uses, run with a short timeout,
and classify the result. A file is FLAGGED if AddressSanitizer,
LeakSanitizer, or UndefinedBehaviorSanitizer reports a real problem.

Some flagged files are known, already-triaged findings (a
methodology false positive, or a real bug deliberately left open
with a documented reason) rather than new regressions -- those are
listed in `tools/leak_sweep_baseline.json` so CI only fails on a
NEW finding, not on rediscovering an already-tracked one.

Usage:
    python3 tools/leak_sweep.py [--vanic PATH] [--fail-on-new]
                                 [--update-baseline]

Exit code: 0 if no un-baselined finding, 1 otherwise. With
`--fail-on-new` (the CI mode), also exits 1 if a PREVIOUSLY
baselined finding stops reproducing -- that means the bug it
documents may have been silently fixed (good news, but the baseline
entry should be removed) or the sweep methodology regressed (bad
news) -- either way it's worth a human looking, not a silent pass.
"""
import argparse
import glob
import json
import os
import subprocess
import sys
import time

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BASELINE_PATH = os.path.join(REPO_ROOT, "tools", "leak_sweep_baseline.json")

# Matches the real compile flags `vanic run --backend=c` uses
# (confirmed by reading main.rs's compile invocation directly), plus
# the sanitizer flags themselves.
GCC_SANITIZE_FLAGS = [
    "-fsanitize=address,leak,undefined",
    "-fno-sanitize-recover=all",
    "-O1", "-g", "-pthread", "-fopenmp",
]


def run(cmd, timeout, env=None):
    try:
        p = subprocess.run(cmd, capture_output=True, timeout=timeout, text=True, env=env)
        return p.returncode, p.stdout, p.stderr, False
    except subprocess.TimeoutExpired as e:
        return None, e.stdout or "", (e.stderr or ""), True


def classify_asan(stderr):
    if "LeakSanitizer" in stderr and "detected memory leaks" in stderr:
        return "LEAK"
    if "AddressSanitizer" in stderr:
        if "heap-use-after-free" in stderr:
            return "USE_AFTER_FREE"
        if "double-free" in stderr or "attempting double-free" in stderr:
            return "DOUBLE_FREE"
        if "heap-buffer-overflow" in stderr:
            return "HEAP_OVERFLOW"
        if "stack-buffer-overflow" in stderr:
            return "STACK_OVERFLOW"
        if "SEGV" in stderr:
            return "SEGV"
        return "ASAN_OTHER"
    if "UndefinedBehaviorSanitizer" in stderr and "runtime error:" in stderr:
        return "UBSAN"
    if "runtime error:" in stderr:
        return "UBSAN_MAYBE"
    return None


def load_baseline():
    if not os.path.exists(BASELINE_PATH):
        return {}
    with open(BASELINE_PATH) as f:
        entries = json.load(f)
    return {e["file"]: e for e in entries}


def relpath(p):
    return os.path.relpath(p, REPO_ROOT)


def sweep(vanic, workdir, timeout_check=10, timeout_emit=15, timeout_gcc=30, timeout_run=8):
    cbin_dir = os.path.join(workdir, "cbins")
    os.makedirs(cbin_dir, exist_ok=True)
    files = sorted(glob.glob(os.path.join(REPO_ROOT, "examples", "**", "*.vani"), recursive=True))
    print(f"Total candidate files: {len(files)}", flush=True)

    findings = {}
    n_checked_out = n_gcc_fail = n_compiled = n_run_clean = n_flagged = 0
    start = time.time()

    for i, path in enumerate(files):
        rc, _out, err, to = run([vanic, "check", path], timeout=timeout_check)
        if to or rc != 0:
            n_checked_out += 1
            continue

        base = os.path.join(cbin_dir, f"f{i}")
        c_path = base + ".c"
        rc, _out, err, to = run([vanic, "emit-c", path, "-o", c_path], timeout=timeout_emit)
        if to or rc != 0 or not os.path.exists(c_path):
            n_checked_out += 1
            continue

        bin_path = base + "_bin"
        gcc_cmd = ["gcc"] + GCC_SANITIZE_FLAGS + ["-o", bin_path, c_path, "-lm"]
        rc, _out, err, to = run(gcc_cmd, timeout=timeout_gcc)
        if to or rc != 0 or not os.path.exists(bin_path):
            n_gcc_fail += 1
            continue
        n_compiled += 1

        env = dict(os.environ)
        env["ASAN_OPTIONS"] = "detect_leaks=1:abort_on_error=0:exitcode=99"
        env["UBSAN_OPTIONS"] = "print_stacktrace=1"
        rc, _out, err, to = run([bin_path], timeout=timeout_run, env=env)
        if to:
            findings[relpath(path)] = {"klass": "TIMEOUT", "stderr": ""}
            n_flagged += 1
            continue

        klass = classify_asan(err)
        if klass:
            findings[relpath(path)] = {"klass": klass, "stderr": err[-4000:]}
            n_flagged += 1
        elif rc == 99:
            findings[relpath(path)] = {"klass": "ASAN_EXIT_99_UNCLASSIFIED", "stderr": err[-4000:]}
            n_flagged += 1
        else:
            n_run_clean += 1

        if (i + 1) % 100 == 0:
            elapsed = time.time() - start
            print(f"[{i+1}/{len(files)}] elapsed={elapsed:.0f}s "
                  f"checked_out={n_checked_out} gcc_fail={n_gcc_fail} "
                  f"compiled={n_compiled} clean={n_run_clean} "
                  f"flagged={n_flagged}", flush=True)

    print("=== DONE ===")
    print(f"Total: {len(files)}")
    print(f"Skipped (check/emit failed): {n_checked_out}")
    print(f"GCC compile failed: {n_gcc_fail}")
    print(f"Compiled+run clean: {n_run_clean}")
    print(f"FLAGGED: {n_flagged}")
    return findings


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--vanic", default=os.path.join(REPO_ROOT, "target", "release", "vanic"),
                     help="path to the vanic binary (default: target/release/vanic)")
    ap.add_argument("--workdir", default=os.path.join(REPO_ROOT, "target", "leak_sweep"),
                     help="scratch directory for emitted C + binaries")
    ap.add_argument("--update-baseline", action="store_true",
                     help="rewrite tools/leak_sweep_baseline.json from this run's findings, instead of comparing against it")
    args = ap.parse_args()

    if not os.path.exists(args.vanic):
        print(f"error: vanic binary not found at {args.vanic} -- build it first "
              f"(cargo build --release --bin vanic)", file=sys.stderr)
        return 2

    findings = sweep(args.vanic, args.workdir)

    if args.update_baseline:
        entries = [
            {"file": f, "klass": v["klass"], "reason": "TODO: fill in why this is known/expected"}
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
            print(f"  {f}: {v['klass']}")
            print("  " + v["stderr"].replace("\n", "\n  ")[:2000])

    if stale_baseline:
        print(f"\n=== {len(stale_baseline)} baseline entr{'y' if len(stale_baseline)==1 else 'ies'} did NOT reproduce ===")
        for f, e in sorted(stale_baseline.items()):
            print(f"  {f}: expected {e['klass']} ({e.get('reason', '')[:100]}) -- now clean")
        print("If this is because the underlying bug got fixed, remove the entry from "
              "tools/leak_sweep_baseline.json. If the sweep methodology changed (e.g. a "
              "file was moved/renamed), update the entry's path instead.")

    if new_findings or stale_baseline:
        return 1
    print("\nAll findings match the baseline exactly. Clean.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
