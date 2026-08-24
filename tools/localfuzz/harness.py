#!/usr/bin/env python3
"""Continuous local differential-testing harness for vanic.

Launched by start.sh under its own `systemd-run --user` transient service
(hard MemoryMax/CPUQuota cap, no swap) so it can't starve an interactive
build or Claude Code session on the same host. Each cycle picks one of
three ways to get a candidate, in priority order:
  1. Every HARNESS_GAP_EVERY cycles: `vanic coverage-gaps` (2026-08-23)
     mines the baked-in coverage database for {shape}#{operation}
     fingerprints with NO regression-test record anywhere in examples/
     -- generate_gap_targeted_program picks one and grounds qwen in a
     real example of the operation (plus, if the element type is a
     real type name rather than a leaf category, a second real example
     showing how that type gets constructed), then asks it to combine
     them into the exact untested shape. A mechanical, corpus-
     independent way to bias the search toward the kind of combination
     that produced BUG-216/217/218, instead of relying on chance.
  2. Otherwise every HARNESS_GENERATE_EVERY cycles: the local Ollama
     model (qwen -- also running under its own capped user service)
     writes a fresh program combining two real vani-compiler features,
     grounded in real example snippets pulled from the corpus (not a
     huge context dump -- see generate_novel_program).
  3. Otherwise: mutate a random corpus .vani file (see MUTATORS).
The candidate is run through `vanic check` and both backends of `vanic
run`; every candidate that passes `check` also gets a `vanic check
--coverage` score attached to its finding/candidate-regression record,
regardless of which of the three paths produced it -- a cheap, offline
signal for whether findings cluster around already-low-coverage
territory (validating or not the bias in path 1 above).

Three outcomes:
  - Crashes/hangs/diverges -> saved under tools/localfuzz/findings/,
    qwen drafts a staging entry (docs/TODO_LOCAL_STAGING.md), and (if
    HARNESS_ATTEMPT_FIXES=1, the default) qwen gets ONE shot at a fix
    hypothesis -- see attempt_fix(). A fix is NEVER auto-applied or
    auto-committed, even if it locally validates (builds + the specific
    repro no longer crashes) -- "passes locally" isn't "correct," and
    this project's own history has examples of that gap. It's saved as
    a plain .patch file for a human/frontier-model to review and decide
    whether to apply. The much more common case -- qwen can't produce
    anything applicable -- still leaves a documented hypothesis (or an
    honest "no hypothesis") for that same later review. Nothing here
    ever blocks waiting for that review; the loop moves on immediately.
  - Compiles/runs cleanly AND was qwen-generated (not a mutation) ->
    saved under tools/localfuzz/candidate_regressions/ as a candidate
    for later promotion into examples/ or tests/run_end_to_end.rs --
    this is qwen's proposed test-suite growth, also unreviewed until a
    human/frontier-model looks at it.
  - Compiles/runs cleanly and was a mutation -> discarded, just logged.

This harness never touches `main` and never edits files outside
tools/localfuzz/findings/, tools/localfuzz/candidate_regressions/, and
docs/TODO_LOCAL_STAGING.md -- it only ever commits to the
local-fuzz-findings branch it runs on.
"""
import argparse
import datetime
import hashlib
import json
import os
import random
import re
import shutil
import signal
import subprocess
import time
import urllib.error
import urllib.request
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
VANIC = REPO / "target" / "release" / "vanic"
CORPUS_DIRS = [REPO / "examples"]
EXAMPLES_ENGLISH = REPO / "examples" / "language" / "english"
FINDINGS_DIR = REPO / "tools" / "localfuzz" / "findings"
CANDIDATE_REGRESSIONS_DIR = REPO / "tools" / "localfuzz" / "candidate_regressions"
STAGING_DOC = REPO / "docs" / "TODO_LOCAL_STAGING.md"
SCRATCH = Path("/tmp/localfuzz")

OLLAMA_URL = os.environ.get("OLLAMA_URL", "http://127.0.0.1:11434")
OLLAMA_MODEL = os.environ.get("OLLAMA_MODEL", "qwen2.5-coder:1.5b")
CHECK_TIMEOUT = int(os.environ.get("CHECK_TIMEOUT", "15"))
RUN_TIMEOUT = int(os.environ.get("RUN_TIMEOUT", "20"))
AUTOCOMMIT = os.environ.get("HARNESS_AUTOCOMMIT", "1") == "1"
GENERATE_EVERY = int(os.environ.get("HARNESS_GENERATE_EVERY", "10"))
ATTEMPT_FIXES = os.environ.get("HARNESS_ATTEMPT_FIXES", "1") == "1"
# 2026-08-23: `vanic coverage-gaps` mines the baked-in coverage
# database and prints candidate {shape}#{operation} fingerprints it
# has NO regression-tested record of at all -- see
# docs/TODO_CURRENT.md's "vanic coverage-gaps" entry on main. Every
# GAP_EVERY-th cycle, instead of a random mutation or a random
# feature-pair combo, the harness targets one of these gap
# fingerprints directly (generate_gap_targeted_program) -- a
# mechanical, corpus-independent way to bias the search toward the
# exact shape of combination that produced BUG-216/217/218, rather
# than relying on chance to stumble into one via random mutation.
GAP_EVERY = int(os.environ.get("HARNESS_GAP_EVERY", "7"))
# The 7 leaf shapes `canonical_shape` produces that are category
# names, not real vāṇी type identifiers (`Copy-Struct` isn't
# something you can write as a type) -- these get a plain-English
# instruction in the gap-targeted prompt instead of a second grounding
# example, since qwen already sees how to write an `i64` or a struct
# literal in the FIRST grounding example.
LEAF_FILLER_HINTS = {
    "Scalar": "a plain i64",
    "Str": "a Str (borrowed string literal)",
    "OwnedStr": "an OwnedStr (e.g. `s + \"\"` to copy a Str literal into one)",
    "Copy-Struct": "a small struct where every field is Copy (e.g. all i64/bool fields)",
    "NonCopy-Struct": "a struct with at least one non-Copy field (e.g. a Vec<i64> or OwnedStr field)",
    "Copy-Enum": "an enum with no payload (or only Copy payloads)",
    "NonCopy-Enum": "an enum with at least one non-Copy payload variant",
}

CRASH_MARKERS = (
    "panicked at",
    # BUG-pattern-audit-round-2 category F (2026-08-08): the bare
    # identifier "RUST_BACKTRACE" false-positived once already -- a
    # qwen-generated candidate happened to contain that literal string
    # in its OWN source text, and vanic's normal (non-crashing) parse-
    # error diagnostic echoed the offending source line back verbatim,
    # tripping this marker on a clean rejection. Anchored to the full
    # phrase Rust's panic runtime actually prints (the standard
    # backtrace-hint suffix line every real panic gets) instead of the
    # bare env-var name -- a fuzzer coincidentally generating this exact
    # multi-word phrase as data is astronomically less likely than the
    # single word alone.
    "run with `RUST_BACKTRACE=1`",
    "memory allocation of",
    "Segmentation fault", "double free", "free(): invalid",
    "SIGSEGV", "SIGABRT", "internal compiler error",
    "unreachable!()", "not yet implemented", "stack overflow",
)

# (human-readable feature name, filename-substring keyword used to find a
# real grounding example under examples/language/english/*.vani). Keep
# names/keywords in sync with what's actually in that directory --
# find_example() returns None on a miss and generate_novel_program()
# skips that cycle rather than guessing.
FEATURES = [
    ("closures capturing by ref", "closure"),
    ("async/await", "async"),
    ("requires/ensures SMT contracts", "contract"),
    ("bounded generics with an interface bound", "bounded_generic"),
    ("match with guards over enum payloads", "match_guard"),
    ("clone_at on a Vec<Struct> element", "clone_at"),
    ("try/? propagation over Option/Result", "try_"),
    ("parallel for with a reduce", "parallel"),
    ("interface/dyn dispatch", "interface"),
    ("Task<R> spawn and join", "task"),
    ("RwLock/Mutex shared state", "rwlock"),
    ("atomics", "atomic"),
    ("Box<dyn Iface>", "box_dyn"),
    ("enum payload variants", "enum"),
    ("array/Vec bounds and iteration", "array"),
]

STAGING_HEADER = """# vani-compiler -- Local Fuzz Staging Log (auto-generated, NOT authoritative)

Candidate findings from the unattended local-model fuzz harness
(`tools/localfuzz/`), running on the `local-fuzz-findings` branch only.
Entries here are drafted by a small local model (qwen) that has NOT read
the compiler source and has NOT verified root cause -- treat every entry
as an unverified lead, not a confirmed bug. Some entries have a sibling
`fix_attempt.md` (qwen's one-shot fix hypothesis, and a `proposed_fix.patch`
if -- rare -- it happened to produce something that locally validates).
Patches are NEVER auto-applied; they need the same review as everything
else here.

Promote a real finding into `docs/TODO_CURRENT.md` (on `main`, with a
proper BUG-N writeup) only after a human or frontier-model session has:
1. Reproduced it independently.
2. Root-caused it in the actual source.
3. Confirmed it isn't already-known/expected behavior.

See also `tools/localfuzz/candidate_regressions/` -- qwen-generated
programs that compiled/ran cleanly and combine two real features in a
way not obviously already covered; candidates for promotion into
`examples/` or `tests/run_end_to_end.rs`, also unreviewed.

---
"""

REPORT_SYSTEM_PROMPT = """You are drafting a CANDIDATE bug report for the vani-compiler \
project's local staging log. You are a small local model assisting a human/frontier-model \
review pipeline -- you have NOT read the compiler source. Do NOT claim a root cause. \
Only describe: what was run, the exact repro source, the observed symptom (crash/hang/ \
divergent output), and which backend(s) it affects. Be terse. If unsure of anything, say \
so explicitly rather than guessing. End with 'STATUS: needs human/frontier root-cause \
review.'"""

GEN_SYSTEM_PROMPT = """You are qwen, writing test programs for the vani-compiler project (a \
statically-typed, SOV-syntax, Devanagari-optional systems language implemented in Rust). You \
are shown two short real example programs, each demonstrating one existing language feature. \
Write ONE new, complete, self-contained .vani program (with a real `main` function returning \
i64) that combines BOTH features in a genuinely new way -- not a copy-paste of the examples. \
Follow the exact syntax shown (keywords, types, statement forms) -- do not invent syntax you \
have not seen in the examples. Output ONLY the vani source code: no prose, no markdown fences."""

FIX_ATTEMPT_SYSTEM_PROMPT = """You are looking at a compiler bug finding for the vani-compiler \
Rust project. You do NOT have access to the source files -- only the failing .vani program, the \
observed symptom, and a heuristic guess (not confirmed) at which source file is likely involved. \
Write a SHORT plain-text hypothesis: what likely goes wrong and roughly why. Do NOT invent file \
contents you have not seen. If, and only if, you are confident enough to propose an EXACT source \
change to a real file under src/, you may include a unified diff in a ```diff fenced block \
(--- a/src/... / +++ b/src/... headers) -- otherwise omit any diff and give only the hypothesis. \
End with 'CONFIDENCE: low/medium/high', or if you have no useful hypothesis at all, end with \
'NO HYPOTHESIS -- needs frontier-model source review.'"""


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


def _is_slow_prone_position(line):
    """True when `line` looks like a `sleep_ms(...)` call or a loop-bound
    (`for ... from/to/downto`, `while ...`) header -- the two shapes that
    made an i64::MIN/MAX boundary literal produce a "correctly asking for
    ~9.2 quintillion iterations / a multi-million-year sleep" program
    instead of a real overflow/bounds-trap repro (BUG_PATTERN_AUDIT_TODO_13.md
    Priority 4, 2026-08-14 -- 83/341 findings in that round, ~24%, were
    this exact shape). Text-level heuristic, not a real parse (this
    mutator has never needed one) -- false positives just mean a few
    extra lines get the smaller-magnitude pool below, which is harmless;
    false negatives just mean the occasional slow-timeout finding still
    gets through, no worse than before this fix.
    """
    return (
        "sleep_ms(" in line
        or ("for " in line and (" from " in line) and (" to " in line or " downto " in line))
        or "while " in line
    )


def mut_numeric_boundary(lines, rng):
    lines = lines[:]
    idxs = [i for i, l in enumerate(lines) if re.search(r"(?<![\w.])\d+(?![\w.])", l)]
    if not idxs:
        return lines
    i = rng.choice(idxs)
    # BUG_PATTERN_AUDIT_TODO_13.md Priority 4: an i64::MIN/MAX literal
    # landing in a sleep_ms(...) argument or a loop bound doesn't stress
    # an overflow/bounds trap the way it does everywhere else -- it just
    # asks for a correctly-specified, deterministically enormous amount
    # of real wall-clock work, which RUN_TIMEOUT can't distinguish from
    # a genuine hang. Keep the extreme literals for every OTHER position
    # (arithmetic/comparison, where they're exactly the interesting case)
    # and fall back to a smaller-magnitude pool only for these two
    # shapes -- still varied, still exercises 0/negative/off-by-one
    # edges, just not ones that turn the whole program into a multi-
    # million-year sleep or a 2^63-iteration loop.
    if _is_slow_prone_position(lines[i]):
        pool = ["0", "-1", "1", "2", "-2", "100", "-100"]
    else:
        pool = ["0", "-1", "1", "9223372036854775807", "-9223372036854775808"]
    repl = rng.choice(pool)
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

def run_vanic(args, timeout, cwd=None, binary=None):
    """Runs vanic in its own process group so a timeout can kill the whole
    group, not just the direct child. vanic itself shells out to `lli`/`cc`
    (see backend_llvm.rs/main.rs) -- a hung or infinite-looping test program
    under `lli` would otherwise survive `vanic`'s own death and leak inside
    this container across cycles, undermining the memory/cpu caps over a
    long unattended run.
    """
    proc = subprocess.Popen(
        [str(binary or VANIC), *args], cwd=cwd or REPO,
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
    # BUG-pattern-audit-round-2 category F (2026-08-08): on POSIX,
    # subprocess.Popen sets returncode to the NEGATIVE signal number
    # when the child was killed by a signal (e.g. -11 for SIGSEGV, -6
    # for SIGABRT) -- a reliable, text-content-independent crash signal
    # that was sitting unused in `result["rc"]` the whole time. `vanic`
    # itself normally translates a crashing CHILD process's (lli / a
    # cc-compiled binary) signal death into an ordinary positive exit
    # code (see BUG-130, "vanic run exit-code masking"), so this mostly
    # catches vanic's OWN process dying by signal -- e.g. a genuine
    # stack-overflow SIGSEGV in the compiler itself, which the Rust
    # panic runtime can't always turn into a clean unwind. Kept as an
    # ADDITIONAL signal alongside the text markers below, not a
    # replacement: an ordinary Rust panic reaching main() exits with a
    # normal positive code (101) under this project's default unwind
    # panic strategy, not a signal death, so text-matching is still the
    # only way to catch that class of crash.
    if result["rc"] is not None and result["rc"] < 0:
        return True
    return any(m in result["stderr"] for m in CRASH_MARKERS)


def test_candidate(path):
    """Always returns a dict with a "kind": one of "check-crash", "run-crash",
    "backend-divergence" (all real findings), "clean-reject" (vanic correctly
    rejected an invalid program -- expected and common for qwen-generated
    candidates, NOT a finding), or "clean-success" (compiled and ran
    consistently on both backends -- the only kind eligible to become a
    candidate regression).

    Flag ranking (cheapest/most-certain first): a crash at `check` stage before
    any backend runs; a clean `check` rejection (not a finding -- just an
    invalid program); a crash/hang under either backend's `run`; a differing
    exit code or stdout between backends. Note: `--backend=c` deliberately
    comes AFTER the file path -- flags before the path have historically been
    silently swallowed by vanic's CLI parsing (see BUG-42 in docs/TODO_CURRENT.md).
    """
    chk = run_vanic(["check", str(path)], CHECK_TIMEOUT)
    if is_crash(chk):
        return {"kind": "check-crash", "check": chk}
    if chk["rc"] != 0:
        return {"kind": "clean-reject", "check": chk}

    run_c = run_vanic(["run", str(path), "--backend=c"], RUN_TIMEOUT)
    run_l = run_vanic(["run", str(path)], RUN_TIMEOUT)

    if is_crash(run_c) or is_crash(run_l):
        return {"kind": "run-crash", "c": run_c, "llvm": run_l}

    if run_c["rc"] != run_l["rc"] or run_c["stdout"] != run_l["stdout"]:
        return {"kind": "backend-divergence", "c": run_c, "llvm": run_l}

    return {"kind": "clean-success", "c": run_c, "llvm": run_l}


# --- findings + reporting ---------------------------------------------------

def save_finding(src_text, finding, base_path):
    h = hashlib.sha1(src_text.encode()).hexdigest()[:10]
    ts = datetime.datetime.utcnow().strftime("%Y%m%d-%H%M%S")
    outdir = FINDINGS_DIR / f"{ts}-{finding['kind']}-{h}"
    outdir.mkdir(parents=True, exist_ok=True)
    (outdir / "repro.vani").write_text(src_text)
    (outdir / "finding.json").write_text(json.dumps({
        "base": str(base_path.relative_to(REPO)) if base_path else "qwen-generated",
        **finding,
    }, indent=2, default=str))
    log(f"finding saved: {outdir}")
    return outdir


def ollama_generate(prompt, system=None, timeout=180, num_predict=512):
    # num_predict bounds completion length -- without it a small model can
    # ramble/repeat indefinitely, burning the whole timeout for no reason.
    payload = {"model": OLLAMA_MODEL, "prompt": prompt, "stream": False,
               "options": {"num_predict": num_predict}}
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
    return ollama_generate(prompt, system=REPORT_SYSTEM_PROMPT, timeout=180, num_predict=400)


def append_staging_doc(outdir, report_text, finding):
    STAGING_DOC.parent.mkdir(parents=True, exist_ok=True)
    if not STAGING_DOC.exists():
        STAGING_DOC.write_text(STAGING_HEADER)
    entry = report_text or (
        "(ollama unavailable -- raw finding only)\n\n```json\n"
        + json.dumps(finding, indent=2, default=str) + "\n```\n"
    )
    fix_note = ""
    if (outdir / "fix_attempt.md").exists():
        fix_note = f"\nFix attempt: `{(outdir / 'fix_attempt.md').relative_to(REPO)}`"
        if (outdir / "proposed_fix.patch").exists():
            fix_note += f" (**validated patch**: `{(outdir / 'proposed_fix.patch').relative_to(REPO)}`)"
    with STAGING_DOC.open("a") as f:
        f.write(f"\n---\n\n### Candidate: {outdir.name}\n\n"
                 f"Repro: `{outdir.relative_to(REPO)}/repro.vani`{fix_note}\n\n{entry}\n")


def git_commit(paths, message):
    if not AUTOCOMMIT:
        return
    subprocess.run(["git", "add", *[str(p) for p in paths]], cwd=REPO, check=False)
    subprocess.run(["git", "commit", "-q", "-m", message], cwd=REPO, check=False)


# --- qwen-driven feature-combination generation -----------------------------

def find_example(keyword):
    matches = sorted(p for p in EXAMPLES_ENGLISH.glob("*.vani")
                      if keyword.lower() in p.stem.lower())
    return matches[0] if matches else None


def find_example_by_content(keyword, corpus_dirs=None):
    """Like find_example, but searches file CONTENT rather than just
    filenames -- needed for gap-targeted generation, where the keyword
    is a builtin operation or type name (`push`, `barrier_new`,
    `Barrier`) that's unlikely to appear in a filename but will appear
    in a file body. Defaults to EXAMPLES_ENGLISH ONLY, same as
    find_example/generate_novel_program -- a non-English dialect file
    uses different native-script STATEMENT keywords (fn/let/while
    etc. are localized per dialect; only builtin identifiers like
    `push` stay constant), and qwen grounded in one would likely
    produce garbage syntax, the same reason generate_novel_program
    never draws from outside EXAMPLES_ENGLISH either. Returns the
    SHORTEST matching file (cheapest possible grounding snippet), or
    None.
    """
    dirs = corpus_dirs or [EXAMPLES_ENGLISH]
    best = None
    for d in dirs:
        for p in d.rglob("*.vani"):
            try:
                text = p.read_text()
            except (UnicodeDecodeError, OSError):
                continue
            if keyword in text and (best is None or len(text) < len(best[1])):
                best = (p, text)
    return best[0] if best else None


def get_coverage_score(path):
    """Runs `vanic check <path> --coverage` and parses the leading
    "coverage: N/100" out of stdout. Returns None if the check itself
    failed (not well-typed -- score is meaningless) or the output
    couldn't be parsed. Cheap and offline (no network, no candidate
    re-run) -- attached to every finding/candidate regression so a
    human reviewing them later can see at a glance whether it clusters
    around already-low-coverage territory.
    """
    result = run_vanic(["check", str(path), "--coverage"], CHECK_TIMEOUT)
    m = re.search(r"coverage:\s*(\d+)/100", result["stdout"])
    return int(m.group(1)) if m else None


def generate_novel_program(rng):
    """Grounds qwen in two REAL example snippets (a few KB each) instead of
    the full tools/llm_context/bundle.py dump (tens of thousands of tokens,
    which reliably timed out regardless of model size -- see git history /
    README "Hardware-driven tuning notes"). Returns (source_or_None,
    feature_pick) -- feature_pick is returned even on failure so callers can
    log what was attempted.
    """
    pick = rng.sample(FEATURES, 2)
    snippets = []
    for name, keyword in pick:
        ex = find_example(keyword)
        if ex is None:
            log(f"no grounding example found for feature '{name}' (keyword '{keyword}')")
            continue
        text = ex.read_text()
        if len(text) > 3000:
            text = text[:3000] + "\n// (truncated)\n"
        snippets.append(f"# Feature: {name}\n# Example file: {ex.name}\n```vani\n{text}\n```")
    if len(snippets) < 2:
        return None, pick

    aliases = subprocess.run(
        ["python3", "tools/llm_context/bundle.py", "--section", "aliases"],
        cwd=REPO, capture_output=True, text=True, timeout=30,
    ).stdout
    prompt = (
        aliases + "\n\n" + "\n\n".join(snippets)
        + f"\n\n---\n\nWrite ONE new program combining: {pick[0][0]} AND {pick[1][0]}."
    )
    out = ollama_generate(prompt, system=GEN_SYSTEM_PROMPT, timeout=300, num_predict=700)
    if not out:
        return None, pick
    cleaned = re.sub(r"^```\w*\n|```$", "", out.strip(), flags=re.MULTILINE)
    return cleaned, pick


GAP_FP_RE = re.compile(r"^([A-Za-z0-9_]+)<([A-Za-z0-9_-]+)>#(.+)$")


def fetch_coverage_gaps():
    """Runs `vanic coverage-gaps --json` and returns the (family,
    filler, op, full_fingerprint) tuples for every gap fingerprint of
    the `Family<Filler>#op` shape -- the only shape this two-grounding
    generation scheme knows how to target (see coverage.rs's own doc
    comment for why depth-1 single-type-param families are the
    scoped-in case; atomic/multi-param shapes are skipped here too).
    Returns [] on any failure (binary too old, no gaps, etc.) so
    callers fall back cleanly.
    """
    out = run_vanic(["coverage-gaps", "--json"], 15)
    if out["rc"] != 0 or not out["stdout"]:
        return []
    try:
        gaps = json.loads(out["stdout"])["gaps"]
    except (json.JSONDecodeError, KeyError, TypeError):
        return []
    parsed = []
    for g in gaps:
        m = GAP_FP_RE.match(g)
        if m:
            parsed.append((m.group(1), m.group(2), m.group(3), g))
    return parsed


def generate_gap_targeted_program(rng, tried):
    """Picks one `Family<Filler>#op` gap fingerprint (preferring one
    this process hasn't already tried this run, per `tried`) and
    grounds qwen in a REAL example exercising `op` plus, if `Filler`
    is a real type name rather than a leaf category, a second real
    example showing how a `Filler` value gets built -- the exact same
    two-grounding-snippet shape as generate_novel_program, just with
    the two snippets chosen from the gap fingerprint instead of the
    static FEATURES table. Returns (source_or_None, fingerprint_or_None).
    """
    candidates = fetch_coverage_gaps()
    if not candidates:
        return None, None
    untried = [c for c in candidates if c[3] not in tried]
    family, filler, op, fp = rng.choice(untried or candidates)
    tried.add(fp)

    op_example = find_example_by_content(op)
    if op_example is None:
        return None, fp
    op_text = op_example.read_text()
    if len(op_text) > 2500:
        op_text = op_text[:2500] + "\n// (truncated)\n"
    snippets = [f"# Example showing the `{op}` operation:\n"
                f"# Example file: {op_example.name}\n```vani\n{op_text}\n```"]

    if filler in LEAF_FILLER_HINTS:
        filler_desc = LEAF_FILLER_HINTS[filler]
    else:
        filler_desc = f"a `{filler}` value"
        filler_example = find_example_by_content(filler)
        if filler_example and filler_example != op_example:
            ftext = filler_example.read_text()
            if len(ftext) > 1500:
                ftext = ftext[:1500] + "\n// (truncated)\n"
            snippets.append(
                f"# Example showing how `{filler}` values are built/used:\n"
                f"# Example file: {filler_example.name}\n```vani\n{ftext}\n```"
            )

    prompt = (
        "\n\n".join(snippets)
        + f"\n\n---\n\nWrite ONE new, complete .vani program that creates a "
        f"`{family}<{filler}>` (a {family} whose element type is {filler_desc}) "
        f"and calls `{op}` on it, following the exact syntax shown above. This "
        f"EXACT combination ({family}<{filler}>, {op}) has never been tried in "
        f"this project before -- the goal is only to exercise it, not to do "
        f"anything meaningful with the result."
    )
    out = ollama_generate(prompt, system=GEN_SYSTEM_PROMPT, timeout=300, num_predict=700)
    if not out:
        return None, fp
    cleaned = re.sub(r"^```\w*\n|```$", "", out.strip(), flags=re.MULTILINE)
    return cleaned, fp


def save_candidate_regression(src_text, feature_pick=None, gap_target=None, coverage_score=None):
    h = hashlib.sha1(src_text.encode()).hexdigest()[:10]
    CANDIDATE_REGRESSIONS_DIR.mkdir(parents=True, exist_ok=True)
    if list(CANDIDATE_REGRESSIONS_DIR.glob(f"*-{h}.vani")):
        return None  # identical content already staged
    ts = datetime.datetime.utcnow().strftime("%Y%m%d-%H%M%S")
    if gap_target:
        slug = re.sub(r"[^a-z0-9]+", "-", gap_target.lower()).strip("-")[:60]
        origin_line = (
            f"// targeted a coverage-gap fingerprint (`vanic coverage-gaps`): {gap_target}\n"
            f"// this exact (shape, operation) combination had no regression-test record\n"
            f"// anywhere in examples/ before this candidate.\n"
        )
    else:
        slug = re.sub(r"[^a-z0-9]+", "-", "-".join(f[1] for f in feature_pick).lower()).strip("-")[:60]
        origin_line = f"// features: {', '.join(f[0] for f in feature_pick)}\n"
    outpath = CANDIDATE_REGRESSIONS_DIR / f"{ts}-{slug}-{h}.vani"
    score_line = f"// coverage score: {coverage_score}/100\n" if coverage_score is not None else ""
    header = (
        f"// candidate regression, qwen-generated ({OLLAMA_MODEL}), UNREVIEWED\n"
        f"{origin_line}"
        f"{score_line}"
        f"// compiled and ran cleanly on both backends -- candidate for examples/ or\n"
        f"// tests/run_end_to_end.rs after human/frontier-model review.\n\n"
    )
    outpath.write_text(header + src_text)
    log(f"candidate regression saved: {outpath.name}")
    return outpath


# --- gated, non-blocking fix attempts ---------------------------------------

def guess_likely_area(finding):
    kind = finding.get("kind")
    if kind == "check-crash":
        return "src/checker.rs, or src/parser.rs if it looks like a parse-stage issue"
    if kind == "run-crash":
        c_bad = is_crash(finding.get("c", {"timed_out": False, "stderr": ""}))
        l_bad = is_crash(finding.get("llvm", {"timed_out": False, "stderr": ""}))
        if c_bad and not l_bad:
            return "src/backend_c.rs (only the C backend crashed; LLVM backend was fine)"
        if l_bad and not c_bad:
            return "src/backend_llvm.rs (only the LLVM backend crashed; C backend was fine)"
        return "src/checker.rs, or both src/backend_c.rs and src/backend_llvm.rs (both crashed)"
    if kind == "backend-divergence":
        return ("src/backend_c.rs and src/backend_llvm.rs -- compare their codegen for the "
                "construct involved, one of them is wrong")
    return "unclear -- src/checker.rs, src/backend_c.rs, or src/backend_llvm.rs"


def try_validate_patch(diff_text, outdir):
    """Best-effort: does this diff even apply against HEAD? Almost always
    "no" for a model with zero file access -- that's expected, not a bug.
    Only on the rare/unexpected case that it DOES apply do we go further:
    apply for real in a disposable throwaway git worktree (never the live
    one this harness runs from), build, and confirm the specific repro no
    longer crashes, before ever calling it "validated".
    """
    diff_text = diff_text.strip()
    if not diff_text:
        return False
    patch_file = outdir / "attempted.patch"
    patch_file.write_text(diff_text + "\n")

    check = subprocess.run(["git", "apply", "--check", str(patch_file)],
                            cwd=REPO, capture_output=True, text=True)
    if check.returncode != 0:
        return False

    scratch_repo = SCRATCH / "fix_validate"
    if scratch_repo.exists():
        shutil.rmtree(scratch_repo, ignore_errors=True)
    subprocess.run(["git", "worktree", "add", "--detach", "-q", str(scratch_repo), "HEAD"],
                    cwd=REPO, check=False, capture_output=True)
    try:
        if not (scratch_repo / ".git").exists():
            return False
        applied = subprocess.run(["git", "apply", str(patch_file.resolve())],
                                  cwd=scratch_repo, capture_output=True, text=True)
        if applied.returncode != 0:
            return False
        built = subprocess.run(["cargo", "build", "--release"], cwd=scratch_repo,
                                capture_output=True, text=True, timeout=600)
        if built.returncode != 0:
            return False
        candidate_bin = scratch_repo / "target" / "release" / "vanic"
        result = run_vanic(["check", str(outdir / "repro.vani")], CHECK_TIMEOUT,
                            cwd=scratch_repo, binary=candidate_bin)
        if is_crash(result):
            return False
        patch_file.rename(outdir / "proposed_fix.patch")
        return True
    finally:
        subprocess.run(["git", "worktree", "remove", "--force", str(scratch_repo)],
                        cwd=REPO, check=False, capture_output=True)


def attempt_fix(src_text, finding, outdir):
    """One qwen call, bounded timeout, never blocks the loop waiting on
    anything external. Almost always ends in an honest "no hypothesis" or
    an unvalidated hypothesis -- expected given the model's demonstrated
    capability, and exactly the documented fallback: a frontier model gets
    a clearly-labeled starting point instead of nothing.
    """
    area = guess_likely_area(finding)
    prompt = (
        f"Failing vani-compiler program:\n```vani\n{src_text}\n```\n\n"
        f"Finding kind: {finding['kind']}\n"
        f"Symptom data:\n```json\n{json.dumps(finding, indent=2, default=str)[:3000]}\n```\n\n"
        f"Heuristic (unconfirmed) likely area: {area}\n\n"
        "Give your hypothesis now."
    )
    out = ollama_generate(prompt, system=FIX_ATTEMPT_SYSTEM_PROMPT, timeout=180, num_predict=450)
    if not out:
        (outdir / "fix_attempt.md").write_text(
            "# Fix attempt\n\nollama call failed/unavailable -- no hypothesis generated.\n"
        )
        return

    diff_match = re.search(r"```diff\n(.*?)```", out, re.DOTALL)
    validated = try_validate_patch(diff_match.group(1), outdir) if diff_match else False

    if validated:
        outcome = ("**A candidate patch was extracted AND locally validated** (applies "
                   "cleanly against HEAD, `cargo build --release` succeeds, and this repro "
                   "no longer crashes) -- see `proposed_fix.patch`. This is still NOT a "
                   "green light to apply it: local validation confirms it builds and doesn't "
                   "crash on THIS ONE repro, not that it's correct or complete. Needs real "
                   "review before applying.")
    elif diff_match:
        outcome = ("A diff was attempted but did not apply/build/fix the repro -- discarded. "
                   "Needs frontier-model or human review from scratch.")
    else:
        outcome = "No patch attempted -- needs frontier-model or human review from scratch."

    (outdir / "fix_attempt.md").write_text(
        f"# Fix attempt (drafted by {OLLAMA_MODEL}, UNVERIFIED unless marked otherwise)\n\n"
        f"Heuristic likely-area hint given to the model: {area}\n\n"
        f"## qwen's response\n\n{out}\n\n"
        f"## Outcome\n\n{outcome}\n"
    )


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
    # Gap fingerprints already targeted THIS process lifetime -- steers
    # generate_gap_targeted_program away from immediate repeats without
    # needing any persistent state across restarts (the gap list itself
    # is deterministic per binary, so avoiding exact repeats within one
    # run is what actually matters for search diversity).
    tried_gaps = set()

    while True:
        cycle += 1
        base_path = None
        feature_pick = None
        gap_target = None
        src = None

        if GAP_EVERY and cycle % GAP_EVERY == 0:
            log("cycle: qwen-generated coverage-gap-targeted program")
            src, gap_target = generate_gap_targeted_program(rng, tried_gaps)
            if src is None:
                log(f"gap-targeted generation failed/unavailable (target={gap_target}), "
                    "falling back")

        if src is None and GENERATE_EVERY and cycle % GENERATE_EVERY == 0:
            log("cycle: qwen-generated feature-combination program")
            src, feature_pick = generate_novel_program(rng)
            if src is None:
                log(f"generation failed/unavailable (features tried: "
                    f"{[f[0] for f in feature_pick] if feature_pick else None}), "
                    "falling back to mutation")

        if src is None:
            base_path = rng.choice(corpus)
            src = mutate(base_path.read_text(), rng)
            feature_pick = None  # mutation output, not a qwen-authored candidate
            gap_target = None

        tmp = SCRATCH / "candidate.vani"
        tmp.write_text(src)

        finding = test_candidate(tmp)
        kind = finding["kind"]
        feature_names = [f[0] for f in feature_pick] if feature_pick else None
        # Skip re-checking a candidate whose `check` stage already
        # crashed/hung once -- re-running it just to score coverage
        # risks re-triggering the same crash/hang for no benefit.
        coverage_score = get_coverage_score(tmp) if kind != "check-crash" else None

        if kind in ("check-crash", "run-crash", "backend-divergence"):
            if feature_pick:
                finding["features"] = feature_names
            if gap_target:
                finding["gap_target"] = gap_target
            if coverage_score is not None:
                finding["coverage_score"] = coverage_score
            log(f"FINDING: {kind} (base={base_path}, features={feature_names}, "
                f"gap_target={gap_target}, coverage_score={coverage_score})")
            outdir = save_finding(src, finding, base_path)
            report = draft_report(src, finding, base_path)
            if ATTEMPT_FIXES:
                attempt_fix(src, finding, outdir)
            append_staging_doc(outdir, report, finding)
            git_commit([outdir, STAGING_DOC],
                       f"localfuzz: candidate {kind} ({outdir.name})")
        elif kind == "clean-success" and (feature_pick or gap_target):
            outpath = save_candidate_regression(
                src, feature_pick=feature_pick, gap_target=gap_target,
                coverage_score=coverage_score,
            )
            if outpath:
                git_commit([outpath], f"localfuzz: candidate regression ({outpath.name})")
            log(f"cycle {cycle}: clean-success, qwen-generated (features={feature_names}, "
                f"gap_target={gap_target}, coverage_score={coverage_score})")
        elif kind == "clean-reject" and (feature_pick or gap_target):
            log(f"cycle {cycle}: qwen-generated candidate rejected by checker "
                f"(features={feature_names}, gap_target={gap_target}) -- "
                "not staged, not a finding")
        else:
            log(f"cycle {cycle}: clean (base={base_path})")

        if args.once:
            break
        time.sleep(args.sleep)


if __name__ == "__main__":
    main()
