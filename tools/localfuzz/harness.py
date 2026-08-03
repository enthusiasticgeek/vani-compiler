#!/usr/bin/env python3
"""Continuous local differential-testing harness for vanic.

Launched by start.sh under its own `systemd-run --user` transient service
(hard MemoryMax/CPUQuota cap, no swap) so it can't starve an interactive
build or Claude Code session on the same host. Each cycle it either
mutates a corpus .vani file or (every HARNESS_GENERATE_EVERY cycles) asks
the local Ollama model (also running under its own capped user service)
to write a fresh one, then runs the candidate through `vanic check` and
both backends of `vanic run`. Anything that crashes, hangs, or diverges
between backends gets saved under tools/localfuzz/findings/ and drafted
into docs/TODO_LOCAL_STAGING.md.

This harness never touches `main` and never edits files outside
tools/localfuzz/findings/ and docs/TODO_LOCAL_STAGING.md -- it only ever
commits to the local-fuzz-findings branch it runs on. Findings are leads,
not confirmed bugs: they still need a human or frontier-model root-cause
pass before being promoted into docs/TODO_CURRENT.md.
"""
import argparse
import datetime
import hashlib
import json
import os
import random
import re
import signal
import subprocess
import time
import urllib.error
import urllib.request
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
VANIC = REPO / "target" / "release" / "vanic"
CORPUS_DIRS = [REPO / "examples"]
FINDINGS_DIR = REPO / "tools" / "localfuzz" / "findings"
STAGING_DOC = REPO / "docs" / "TODO_LOCAL_STAGING.md"
SCRATCH = Path("/tmp/localfuzz")

OLLAMA_URL = os.environ.get("OLLAMA_URL", "http://127.0.0.1:11434")
OLLAMA_MODEL = os.environ.get("OLLAMA_MODEL", "qwen2.5-coder:1.5b")
CHECK_TIMEOUT = int(os.environ.get("CHECK_TIMEOUT", "15"))
RUN_TIMEOUT = int(os.environ.get("RUN_TIMEOUT", "20"))
AUTOCOMMIT = os.environ.get("HARNESS_AUTOCOMMIT", "1") == "1"
GENERATE_EVERY = int(os.environ.get("HARNESS_GENERATE_EVERY", "0"))

CRASH_MARKERS = (
    "panicked at", "RUST_BACKTRACE", "memory allocation of",
    "Segmentation fault", "double free", "free(): invalid",
    "SIGSEGV", "SIGABRT", "internal compiler error",
    "unreachable!()", "not yet implemented", "stack overflow",
)

STAGING_HEADER = """# vani-compiler -- Local Fuzz Staging Log (auto-generated, NOT authoritative)

Candidate findings from the unattended local-model fuzz harness
(`tools/localfuzz/`), running on the `local-fuzz-findings` branch only.
Entries here are drafted by a small local model that has NOT read the
compiler source and has NOT verified root cause -- treat every entry as
an unverified lead, not a confirmed bug.

Promote a real finding into `docs/TODO_CURRENT.md` (on `main`, with a
proper BUG-N writeup) only after a human or frontier-model session has:
1. Reproduced it independently.
2. Root-caused it in the actual source.
3. Confirmed it isn't already-known/expected behavior.

---
"""

REPORT_SYSTEM_PROMPT = """You are drafting a CANDIDATE bug report for the vani-compiler \
project's local staging log. You are a small local model assisting a human/frontier-model \
review pipeline -- you have NOT read the compiler source. Do NOT claim a root cause. \
Only describe: what was run, the exact repro source, the observed symptom (crash/hang/ \
divergent output), and which backend(s) it affects. Be terse. If unsure of anything, say \
so explicitly rather than guessing. End with 'STATUS: needs human/frontier root-cause \
review.'"""


def log(msg):
    ts = datetime.datetime.utcnow().isoformat(timespec="seconds")
    print(f"[{ts}Z] {msg}", flush=True)


def ensure_vanic_built():
    if VANIC.exists():
        return
    log("vanic binary not found, building (cargo build --release)...")
    subprocess.run(["cargo", "build", "--release"], cwd=REPO, check=True)
    log("build complete.")


def load_corpus():
    files = [p for d in CORPUS_DIRS for p in d.rglob("*.vani")]
    if not files:
        raise SystemExit(f"no .vani corpus files found under {CORPUS_DIRS}")
    log(f"loaded {len(files)} corpus files")
    return files


# --- mutators: simple text/line-level edits, no parser needed -------------

def _stmt_lines(lines):
    return [i for i, l in enumerate(lines) if l.strip().endswith(";")]


def mut_numeric_boundary(lines, rng):
    lines = lines[:]
    idxs = [i for i, l in enumerate(lines) if re.search(r"(?<![\w.])\d+(?![\w.])", l)]
    if not idxs:
        return lines
    i = rng.choice(idxs)
    repl = rng.choice(["0", "-1", "1", "9223372036854775807", "-9223372036854775808"])
    lines[i] = re.sub(r"(?<![\w.])\d+(?![\w.])", repl, lines[i], count=1)
    return lines


def mut_duplicate_line(lines, rng):
    lines = lines[:]
    idxs = _stmt_lines(lines)
    if not idxs:
        return lines
    i = rng.choice(idxs)
    lines.insert(i, lines[i])
    return lines


def mut_delete_line(lines, rng):
    lines = lines[:]
    idxs = [i for i in _stmt_lines(lines) if "fn " not in lines[i]]
    if not idxs:
        return lines
    del lines[rng.choice(idxs)]
    return lines


def mut_swap_adjacent(lines, rng):
    lines = lines[:]
    idxs = set(_stmt_lines(lines))
    pairs = [i for i in idxs if (i + 1) in idxs]
    if not pairs:
        return lines
    i = rng.choice(pairs)
    lines[i], lines[i + 1] = lines[i + 1], lines[i]
    return lines


def mut_type_swap(lines, rng):
    lines = lines[:]
    types = ["i64", "u64", "i32", "u32", "f64"]
    idxs = [i for i, l in enumerate(lines) if any(f": {t}" in l for t in types)]
    if not idxs:
        return lines
    i = rng.choice(idxs)
    for t in types:
        if f": {t}" in lines[i]:
            other = rng.choice([x for x in types if x != t])
            lines[i] = lines[i].replace(f": {t}", f": {other}", 1)
            break
    return lines


def mut_wrap_redundant(lines, rng):
    lines = lines[:]
    idxs = [i for i, l in enumerate(lines) if "let " in l and l.rstrip().endswith(";")]
    if not idxs:
        return lines
    i = rng.choice(idxs)
    lines[i] = lines[i].rstrip()[:-1] + " + 0;"
    return lines


MUTATORS = [
    mut_numeric_boundary, mut_duplicate_line, mut_delete_line,
    mut_swap_adjacent, mut_type_swap, mut_wrap_redundant,
]


def mutate(src, rng, n=None):
    lines = src.split("\n")
    for _ in range(n or rng.randint(1, 2)):
        lines = rng.choice(MUTATORS)(lines, rng)
    return "\n".join(lines)


# --- running vanic ----------------------------------------------------------

def run_vanic(args, timeout):
    """Runs vanic in its own process group so a timeout can kill the whole
    group, not just the direct child. vanic itself shells out to `lli`/`cc`
    (see backend_llvm.rs/main.rs) -- a hung or infinite-looping test program
    under `lli` would otherwise survive `vanic`'s own death and leak inside
    this container across cycles, undermining the memory/cpu caps over a
    long unattended run.
    """
    proc = subprocess.Popen(
        [str(VANIC), *args], cwd=REPO,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
        start_new_session=True,
    )
    try:
        stdout, stderr = proc.communicate(timeout=timeout)
        return dict(rc=proc.returncode, stdout=stdout, stderr=stderr, timed_out=False)
    except subprocess.TimeoutExpired:
        try:
            os.killpg(os.getpgid(proc.pid), signal.SIGKILL)
        except ProcessLookupError:
            pass
        stdout, stderr = proc.communicate()
        return dict(rc=None, stdout=stdout or "", stderr=stderr or "", timed_out=True)


def is_crash(result):
    if result["timed_out"]:
        return True
    return any(m in result["stderr"] for m in CRASH_MARKERS)


def test_candidate(path):
    """Returns None if nothing interesting, else a dict describing the finding.

    Flag ranking (cheapest/most-certain first): a crash at `check` stage before
    any backend runs; a crash/hang under either backend's `run`; identical-exit
    but differing stdout between backends. Note: `--backend=c` deliberately
    comes AFTER the file path -- flags before the path have historically been
    silently swallowed by vanic's CLI parsing (see BUG-42 in docs/TODO_CURRENT.md).
    """
    chk = run_vanic(["check", str(path)], CHECK_TIMEOUT)
    if is_crash(chk):
        return {"kind": "check-crash", "check": chk}

    run_c = run_vanic(["run", str(path), "--backend=c"], RUN_TIMEOUT)
    run_l = run_vanic(["run", str(path)], RUN_TIMEOUT)

    if is_crash(run_c) or is_crash(run_l):
        return {"kind": "run-crash", "c": run_c, "llvm": run_l}

    if run_c["rc"] == 0 and run_l["rc"] == 0 and run_c["stdout"] != run_l["stdout"]:
        return {"kind": "backend-divergence", "c": run_c, "llvm": run_l}

    return None


# --- findings + reporting ---------------------------------------------------

def save_finding(src_text, finding, base_path):
    h = hashlib.sha1(src_text.encode()).hexdigest()[:10]
    ts = datetime.datetime.utcnow().strftime("%Y%m%d-%H%M%S")
    outdir = FINDINGS_DIR / f"{ts}-{finding['kind']}-{h}"
    outdir.mkdir(parents=True, exist_ok=True)
    (outdir / "repro.vani").write_text(src_text)
    (outdir / "finding.json").write_text(json.dumps({
        "base": str(base_path.relative_to(REPO)) if base_path else "llm-generated",
        **finding,
    }, indent=2, default=str))
    log(f"finding saved: {outdir}")
    return outdir


def ollama_generate(prompt, system=None, timeout=180):
    payload = {"model": OLLAMA_MODEL, "prompt": prompt, "stream": False}
    if system:
        payload["system"] = system
    req = urllib.request.Request(
        f"{OLLAMA_URL}/api/generate",
        data=json.dumps(payload).encode(),
        headers={"Content-Type": "application/json"},
    )
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            return json.loads(resp.read())["response"]
    except (urllib.error.URLError, TimeoutError, OSError) as e:
        log(f"ollama call failed: {e}")
        return None


def draft_report(src_text, finding, base_path):
    prompt = (
        f"Base corpus file: {base_path}\n\n"
        f"Mutant/generated source:\n```vani\n{src_text}\n```\n\n"
        f"Finding kind: {finding['kind']}\n"
        f"Raw result data:\n```json\n{json.dumps(finding, indent=2, default=str)[:4000]}\n```\n\n"
        "Draft the staging entry now."
    )
    return ollama_generate(prompt, system=REPORT_SYSTEM_PROMPT)


def append_staging_doc(outdir, report_text, finding):
    STAGING_DOC.parent.mkdir(parents=True, exist_ok=True)
    if not STAGING_DOC.exists():
        STAGING_DOC.write_text(STAGING_HEADER)
    entry = report_text or (
        "(ollama unavailable -- raw finding only)\n\n```json\n"
        + json.dumps(finding, indent=2, default=str) + "\n```\n"
    )
    with STAGING_DOC.open("a") as f:
        f.write(f"\n---\n\n### Candidate: {outdir.name}\n\n"
                 f"Repro: `{outdir.relative_to(REPO)}/repro.vani`\n\n{entry}\n")


def git_commit(paths, message):
    if not AUTOCOMMIT:
        return
    subprocess.run(["git", "add", *[str(p) for p in paths]], cwd=REPO, check=False)
    subprocess.run(["git", "commit", "-q", "-m", message], cwd=REPO, check=False)


def generate_novel_program(rng):
    # Disabled by default (HARNESS_GENERATE_EVERY=0): the bundle.py context
    # below is large enough that CPU-only prefill reliably exceeds even a
    # 240s timeout on a capped small model -- confirmed on this host with
    # qwen2.5-coder:1.5b. It's the prompt SIZE that's the bottleneck here,
    # not model size, so a bigger model won't help and a smaller CPU cap
    # would only make it worse. draft_report() (used for real findings) has
    # a much shorter prompt and works fine within the same timeout budget.
    bundle = subprocess.run(
        ["python3", "tools/llm_context/bundle.py", "--no-examples"],
        cwd=REPO, capture_output=True, text=True, timeout=30,
    ).stdout
    features = [
        "closures capturing by ref", "generics with a where-bound interface",
        "async/await", "requires/ensures SMT contracts", "parallel for with reduce",
        "match with guards and enum payloads", "Vec<Struct> mutation via clone_at",
        "Result/Option error propagation with try/?",
    ]
    pick = rng.sample(features, 2)
    prompt = (
        bundle + "\n\n---\n\nWrite ONE complete, self-contained .vani program "
        f"(with a real `main`) that exercises: {pick[0]} AND {pick[1]}. "
        "Output ONLY the vani source, no prose, no markdown fences."
    )
    out = ollama_generate(prompt, timeout=240)
    if not out:
        return None
    return re.sub(r"^```\w*\n|```$", "", out.strip(), flags=re.MULTILINE)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--once", action="store_true", help="run a single cycle and exit")
    ap.add_argument("--sleep", type=float,
                     default=float(os.environ.get("HARNESS_SLEEP", "20")))
    args = ap.parse_args()

    ensure_vanic_built()
    corpus = load_corpus()
    rng = random.Random()
    SCRATCH.mkdir(parents=True, exist_ok=True)
    cycle = 0

    while True:
        cycle += 1
        base_path = None
        src = None

        if GENERATE_EVERY and cycle % GENERATE_EVERY == 0:
            log("cycle: LLM-generated novel program")
            src = generate_novel_program(rng)
            if src is None:
                log("generation failed/unavailable, falling back to mutation")

        if src is None:
            base_path = rng.choice(corpus)
            src = mutate(base_path.read_text(), rng)

        tmp = SCRATCH / "candidate.vani"
        tmp.write_text(src)

        finding = test_candidate(tmp)
        if finding:
            log(f"FINDING: {finding['kind']} (base={base_path})")
            outdir = save_finding(src, finding, base_path)
            report = draft_report(src, finding, base_path)
            append_staging_doc(outdir, report, finding)
            git_commit([outdir, STAGING_DOC],
                       f"localfuzz: candidate {finding['kind']} ({outdir.name})")
        else:
            log(f"cycle {cycle}: clean (base={base_path})")

        if args.once:
            break
        time.sleep(args.sleep)


if __name__ == "__main__":
    main()
