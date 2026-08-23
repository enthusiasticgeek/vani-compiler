# localfuzz -- continuous local differential-testing harness

Unattended, token-free bug-hunting for vani-compiler using a small local
model (via Ollama) and a deterministic differential-testing harness.
Isolated from your normal interactive `vani-compiler` checkout and any
Claude Code session working there at the same time.

## Why no Docker

The original design used Docker for isolation. This host's kernel
(`5.4.28avl2-lowlatency`, a custom real-time build tuned for audio work)
was built without `CONFIG_CGROUP_BPF`, which container runtimes require
even to start a container at all -- confirmed by `docker run
debian:bookworm-slim echo hi` failing identically to anything more
elaborate. Rebuilding/reconfiguring that kernel is out of scope here (real
risk to whatever real-time audio setup it's tuned for) and wasn't asked
for. Plain cgroup v2 resource control -- the actual thing this harness
needs -- works fine on this host independent of that BPF gap, so the
isolation is built directly on `systemd-run --user` instead.

## Isolation guarantees

- **Filesystem/git**: this whole directory lives in a dedicated git
  worktree (`vani-compiler-localfuzz`, sibling to the real `vani-compiler`
  checkout), on its own branch (`local-fuzz-findings`).
- **Filesystem sandboxing (real, OS-enforced, not just script discipline)**:
  every process that talks to the local model -- Ollama itself, and the
  harness -- runs under `ProtectSystem=strict` + `ProtectHome=tmpfs`,
  which replaces the ENTIRE filesystem outside a few system paths with
  either read-only or an empty, non-persistent tmpfs. Specific real
  directories are then punched back through via `BindPaths=`
  (read-write) / `BindReadOnlyPaths=` (read-only), driven by
  `allowed_paths.conf` / `allowed_readonly_paths.conf` (see
  "Filesystem allowlist" below). This is verified, not just configured
  -- inspecting a running unit's actual mount namespace via
  `/proc/<pid>/root` confirms: the harness sees only the
  `vani-compiler-localfuzz` worktree, the main checkout's `.git` dir
  (needed for worktree git plumbing, not a scope violation -- see the
  comment in `allowed_paths.conf`), and the Rust toolchain (read-only);
  every one of the dozens of *other* project directories on this
  machine, and the main checkout's actual working-tree files, are
  invisible, not just unwritable. Ollama itself (the actual model
  process) is sandboxed even tighter: it has no access to
  `vani-compiler` at all, only its own binary and model-storage
  directory -- it never needs repo access, since the harness mediates
  everything over the local HTTP API.
- **Not network-isolated**: `IPAddressDeny=`/`IPAddressAllow=` were
  tried and don't work here -- systemd logs "unit configures an IP
  firewall, but not running as root" and silently allows all traffic
  for `--user` units on this host. Not claimed or configured. The
  practical mitigation in place is that Ollama binds to `127.0.0.1`
  only (no inbound exposure from other machines); outbound egress from
  either process is not restricted at the sandboxing layer.
- **Resources**: Ollama and the harness each run as their own transient
  `systemd --user` service (`systemd-run --unit=... -p MemoryMax=... -p
  CPUQuota=... -p MemorySwapMax=0`), which is a real, kernel-enforced
  cgroup v2 ceiling -- confirmed working unprivileged on this host
  (`systemctl --user show ... -p MemoryMax` reflects the cap immediately).
  `MemorySwapMax=0` means neither service can ever push the box into swap.
  Defaults: `ollama` 1.5 CPU / 3GB, `harness` 1 CPU / 1GB -- together 2.5
  of your cores and 4 of your 11GB, leaving the rest free for an
  interactive `cargo build` / Claude Code session. Tune via env vars or by
  editing `start.sh` directly.
- **Writes**: the harness only ever commits to the `local-fuzz-findings`
  branch it runs on (git identity set via `GIT_AUTHOR_*`/`GIT_COMMITTER_*`
  env vars passed to the transient service -- deliberately not `git config
  --local`, which in a linked worktree would write to the config *shared*
  with your main checkout). It never pushes, never touches `main`, never
  edits `docs/TODO_CURRENT.md`.
- **Not running by default**: nothing here auto-starts at login or boot.
  `start.sh` / `stop.sh` are explicit. (If you later want it to survive
  logout, `loginctl enable-linger $USER` keeps your user's systemd
  instance -- and anything running under it -- alive without an active
  session; not enabled here.)

## Filesystem allowlist

`allowed_paths.conf` (read-write) and `allowed_readonly_paths.conf`
(read-only) are the actual security policy -- one absolute path per line,
enforced as described above. Defaults to just the vani-compiler worktree
(+ the main checkout's `.git` dir, structurally required) and the Rust
toolchain. To let this same sandboxing cover another project later, add
its path to `allowed_paths.conf` -- nothing else needs to change.
`sandbox_lib.sh` (sourced by `start.sh` and `run-sandboxed.sh`) is what
turns these files into `systemd-run` arguments.

## What it actually does

Not an LLM guessing at bugs -- an LLM assisting a deterministic harness.
Each cycle:

1. **Pick a candidate**, in priority order:
   - Every `HARNESS_GAP_EVERY`th cycle (default 7, 2026-08-23): `vanic
     coverage-gaps` mines the compiler's own baked-in coverage
     database for `{shape}#{operation}` fingerprints with NO
     regression-test record anywhere in `examples/` at all (see
     `docs/TODO_CURRENT.md`'s "vanic coverage-gaps" entry on `main`).
     `generate_gap_targeted_program` picks one, finds a real English
     example exercising the operation, finds a second real example
     showing how the target element type gets constructed (if it's a
     real type rather than a leaf category like "a Copy struct"), and
     asks qwen to combine them into the exact untested shape. A
     mechanical, corpus-independent way to bias the search toward the
     kind of combination that produced BUG-216/217/218, rather than
     hoping random mutation stumbles into one.
   - Otherwise every `HARNESS_GENERATE_EVERY`th cycle (default 10):
     qwen writes a fresh program combining two real language features.
     It's grounded in two REAL example snippets pulled from
     `examples/language/english/` by keyword match against the
     `FEATURES` list in `harness.py` (NOT the full
     `tools/llm_context/bundle.py` dump -- that's tens of thousands of
     tokens and reliably timed out regardless of model size; a couple
     of concrete examples plus the keyword-alias table is enough
     grounding and actually fits in the timeout budget). This is qwen
     "learning" the feature set in the loosest sense: it never sees
     the compiler source, only real usage examples, each cycle.
   - Otherwise: a file from `examples/**/*.vani` (1000+ files already
     in the repo, including the `examples/edge_cases/` adversarial
     corpus), with 1-2 small text-level mutations applied (numeric
     boundary values, statement duplication/deletion/reordering,
     primitive-type swaps).
2. Runs the candidate through `vanic check`, then (only if `check`
   accepts it) `vanic run` on both backends (`--backend=c` and default
   LLVM), each under a timeout (with the whole process group killed on
   timeout, so a test program stuck in an infinite loop under `lli`
   can't leak past its deadline).
3. Classifies the result: `check` itself crashes/hangs -> finding.
   `check` cleanly rejects it (a normal diagnostic, common for
   qwen-generated candidates that got the syntax wrong) -> discarded,
   not a finding. Either backend crashes/hangs, or both exit with a
   different code or stdout (backend divergence -- the bug class that
   found most of this project's real bugs historically) -> finding.
   Both backends agree and complete cleanly -> success.
4. **On a finding**: saves the repro + raw results under
   `tools/localfuzz/findings/<timestamp>-<kind>-<hash>/`, qwen drafts a
   terse, honesty-gated staging entry (explicitly forbidden from
   claiming a root cause it hasn't verified) into
   `docs/TODO_LOCAL_STAGING.md`, and -- if `HARNESS_ATTEMPT_FIXES=1`,
   the default -- qwen gets one bounded shot at a fix; see "Fix
   attempts" below. All committed to `local-fuzz-findings`. Never
   blocks waiting for anyone to look at it. Any candidate that got
   past `check` also gets a `vanic check --coverage` score attached
   (`finding.json`'s `coverage_score` key) regardless of which of the
   three generation paths produced it -- cheap, offline, and lets a
   later review see at a glance whether findings actually cluster
   around low-coverage territory.
5. **On a clean success that was qwen-generated** (not a mutation):
   saved under `tools/localfuzz/candidate_regressions/` -- a candidate
   for promotion into `examples/` or `tests/run_end_to_end.rs` as a
   permanent regression test, since it demonstrates a feature
   combination that compiled and ran consistently on both backends.
   Its header comment records which generation path produced it (a
   feature pair, or a targeted gap fingerprint) and its coverage
   score. Also unreviewed until a human/frontier-model looks at it.
6. **On a clean success from mutation**: discarded, just logged --
   it's a minor variation of an example that already exists in the
   corpus, not new coverage.

This mirrors the mechanical compile-and-diff method that found nearly
every real bug in this project's history (see `project_vani_compiler_status`
memory) -- the harness does the finding, qwen does the volume generation,
first-pass writeup, and (rarely successful, but attempted) fix drafting.

### Fix attempts

On every finding (if `HARNESS_ATTEMPT_FIXES=1`), qwen gets ONE call: the
repro, the symptom, and a *heuristic* (not confirmed) guess at which
source file is likely involved, based on which stage/backend failed
(`guess_likely_area()` in `harness.py`). It's asked for a short
hypothesis, and *only if confident*, an optional unified diff.

- If qwen doesn't produce anything diff-shaped (the common case, given
  this model's demonstrated capability -- see "Hardware-driven tuning
  notes"): the hypothesis (or an honest "no hypothesis") is saved to
  `fix_attempt.md` next to the finding. This is the expected fallback --
  a documented starting point for a human or a frontier model, not a fix.
- If it DOES produce something diff-shaped: `git apply --check` first
  (a nearly-free dry run against HEAD -- almost certain to fail, since
  qwen has never seen the actual file content it's patching blind). Only
  if that unexpectedly succeeds does it go further: apply for real in a
  disposable, throwaway `git worktree` (never the live one this harness
  runs from), `cargo build --release`, and confirm the specific repro no
  longer crashes.
- **Even a fully-validated patch is NEVER auto-applied or auto-committed**
  -- it's saved as `proposed_fix.patch`, clearly marked as needing real
  review. "Builds and doesn't crash on this one repro" is not the same
  claim as "correct," and this project's own history (e.g. BUG-31's
  follow-up regression) has examples of exactly that gap.

## First-time setup

Requires `cargo`/`z3`/`clang`/`lli`/`python3` on `PATH` (already true on
this host) and `systemd` >= 245ish with cgroup v2 delegation to user
sessions (confirmed working here, systemd 257).

Ollama itself is installed as a plain user-space tarball, not the
official install script -- the script also auto-enables an *uncapped*
system-wide `ollama.service`, which would fight this setup for port 11434
and defeat the whole point of capping it. Ollama's releases ship as
`.tar.zst`, and this host has no system `zstd` binary (`tar --zstd` fails
with "Cannot exec: No such file or directory") and can't get one without
`sudo apt install zstd` -- decompress with Python's `zstandard` package
instead (installed to user site-packages; the `--break-system-packages`
flag is required by Debian's PEP 668 guard but this only touches
`~/.local/lib/python3.13/site-packages`, not any apt-managed package):

```bash
python3 -m pip install --user --break-system-packages zstandard

curl -L -o /tmp/ollama.tar.zst \
  https://github.com/ollama/ollama/releases/latest/download/ollama-linux-amd64.tar.zst
mkdir -p ~/.local/share/vani-localfuzz/ollama-dist
python3 -c "
import tarfile, zstandard, os
dctx = zstandard.ZstdDecompressor()
with open('/tmp/ollama.tar.zst', 'rb') as fh, dctx.stream_reader(fh) as reader:
    with tarfile.open(fileobj=reader, mode='r|') as tf:
        tf.extractall(os.path.expanduser('~/.local/share/vani-localfuzz/ollama-dist'), filter='data')
"
rm /tmp/ollama.tar.zst
```

Then pull the model (starts Ollama first, since `ollama pull` needs the
server running):

```bash
cd tools/localfuzz
./start.sh
OLLAMA_HOST=127.0.0.1:11434 ~/.local/share/vani-localfuzz/ollama-dist/bin/ollama \
  pull qwen2.5-coder:7b-instruct-q4_K_M
./stop.sh
```

## Running

```bash
cd tools/localfuzz
./start.sh                     # starts both services, runs forever
journalctl --user -u vani-localfuzz-harness -f   # watch cycles/findings live
./stop.sh                      # stop everything
```

One-shot smoke test (single cycle, no caps, run in the foreground --
useful before trusting it to run unattended). Ollama must already be
running (`./start.sh` first, or `ollama serve &` manually):

```bash
python3 tools/localfuzz/harness.py --once
```

## Staying current with `main` (`refresh.sh`)

This worktree's `vanic` binary does NOT update itself just because
`main` moves -- and `main` moves on its own even without you touching
it, since a separate automated process also lands bugfixes there (see
`docs/TODO_CURRENT.md`'s BUG-NN history and CHANGELOG.md). Confirmed
2026-08-04: this worktree's binary was 2 days / 25 bugfix commits
stale, and ~19 of 84 accumulated findings turned out to be bugs
already fixed on `main` (BUG-76, BUG-88) -- the harness was silently
re-discovering dead bugs instead of finding new ones.

`refresh.sh` fixes this: stops the harness, merges `main` into
`local-fuzz-findings`, `cargo build --release`s, restarts. Safe to run
unattended -- refuses to touch a dirty worktree (except the two
permanently-locally-modified allowlist configs), aborts and restores
the prior state on a merge conflict or if no binary results, never
force-pushes or touches `main`.

```bash
tools/localfuzz/refresh.sh                          # by hand, anytime
journalctl --user -u vani-localfuzz-refresh -f       # if run via the timer below
```

Wired to a nightly `systemd --user` timer
(`~/.config/systemd/user/vani-localfuzz-refresh.timer`, 03:00 by
default) plus a second timer for the digest below
(`vani-localfuzz-digest.timer`, 06:00 -- offset 3h later so the
harness has had time to generate findings against the FRESH binary
before they're summarized). Both are `Persistent=true` (catches up if
the machine was off at the scheduled time) but only fire while this
user's systemd instance is running -- if you log out entirely between
runs, either accept that gap or `loginctl enable-linger $USER` so your
user services (including these) keep running unattended. Enable with:

```bash
systemctl --user enable --now vani-localfuzz-refresh.timer vani-localfuzz-digest.timer
systemctl --user list-timers 'vani-localfuzz*'   # confirm next-run times
```

## Deduped digest (`digest.py`)

84 raw `findings/*/finding.json` files don't scale for a human or a
model to read one at a time -- most cluster into a handful of distinct
root causes. `digest.py` groups findings by a mechanical signature
(exit codes, timeout flags, coarse stderr classification), cross-checks
each cluster's stderr keywords against `main`'s current
`docs/TODO_CURRENT.md`/`CHANGELOG.md` (via `git show main:<path>`,
read-only through the shared `.git` dir, no working-tree access needed)
to flag "possible match: BUG-N" clusters, and writes a compact
`DIGEST_LATEST.md`. Tracks what it's already reported in
`.digest_state.json`, so a bare re-run only surfaces genuinely NEW
findings since the last digest:

```bash
python3 tools/localfuzz/digest.py          # only new findings since last run
python3 tools/localfuzz/digest.py --all    # full re-scan (doesn't reset the "seen" marker)
```

Auto-commits its own output to `local-fuzz-findings`, same convention
as the harness's own finding commits.

## Handoff to a frontier model (nightly pipeline, human-gated)

The full pipeline, in order: **fuzz continuously (harness.py) -> nightly
refresh (main merge + rebuild, 03:00) -> nightly digest (deduped +
already-fixed-checked, 06:00) -> human/Claude review, on demand.**

The first three stages are safe to run fully unattended (mechanical,
bounded, never touch `main`, never apply anything unreviewed). The last
stage is deliberately NOT automated further -- e.g. an autonomous nightly
agent that reproduces, root-causes, fixes, tests, AND commits/pushes to
`main` with no human in the loop is a different risk tier than anything
else in this tool. This project's own history has examples of a
validated-looking fix later needing a follow-up regression fix (see
BUG-31's history) -- auto-merging compiler changes unsupervised risks
silently breaking the compiler for everyone. When you (or a Claude Code
session) have time/tokens available:

```bash
cd ../vani-compiler-localfuzz
cat tools/localfuzz/DIGEST_LATEST.md
```

For each cluster NOT flagged "possible match: BUG-N": reproduce against
a freshly-refreshed `main` build (not this worktree's binary) yourself,
root-cause it properly, and only then write a real `BUG-N` entry into
`docs/TODO_CURRENT.md` on `main` -- same rigor as every other bug in
this project. If you want MORE automation than "read the digest by
hand," the `schedule` skill (Claude Code) can run a bounded nightly
agent that reads `DIGEST_LATEST.md` and drafts (never commits) a
root-cause writeup for unmatched clusters -- worth setting up once this
digest has run for a few days and you've seen what its signal-to-noise
actually looks like.

## Reviewing / promoting findings

Nothing here is trusted automatically. Periodically:

```bash
cd ../vani-compiler-localfuzz     # this worktree
git log --oneline local-fuzz-findings
cat docs/TODO_LOCAL_STAGING.md
```

For anything that looks real: reproduce it yourself (or hand it to a
frontier-model session) against the real `vani-compiler` checkout,
root-cause it properly, and only then write a real `BUG-N` entry into
`docs/TODO_CURRENT.md` on `main` -- the same rigor every other bug in
this project has gotten. Nothing from `local-fuzz-findings` should be
merged into `main` as-is. The same applies to
`tools/localfuzz/candidate_regressions/*.vani` -- verify each one
actually demonstrates something not already covered before copying it
into `examples/` or adding a `tests/run_end_to_end.rs` case for it.

## Fixing findings, deeper manual pass (Aider)

The continuous loop already gives every finding one bounded, automatic
fix attempt (see "Fix attempts" above) -- `fix_attempt.md` next to each
finding tells you whether that produced anything. This section is for
going deeper on a specific finding by hand, when you want to actually
sit with it -- still deliberately kept OUT of the unattended loop, since
open-ended multi-turn editing is a different risk tier than one bounded
generate-and-validate call. E.g. with [Aider](https://aider.chat)
against the same Ollama instance (start it first via `./start.sh` if it
isn't already running):

Run it through `run-sandboxed.sh` so it gets the same filesystem
confinement as everything else here:

```bash
pip install --user --break-system-packages aider-chat   # one-time
cd tools/localfuzz
export OLLAMA_API_BASE=http://127.0.0.1:11434
./run-sandboxed.sh -- aider --model ollama_chat/qwen2.5-coder:1.5b \
      src/checker.rs   # or whichever file the finding points at
```

Note: `run-sandboxed.sh`'s `--wait`/`--pty` flags need a real interactive
terminal (a controlling TTY) to work -- the filesystem-sandboxing and
env-passing logic were verified independently, but `--wait`/`--pty`
themselves couldn't be fully exercised from the non-interactive session
that built this. Try `./run-sandboxed.sh -- echo hello` first to confirm
it end-to-end before trusting it with Aider.

Before accepting anything it proposes: run `cargo test --workspace`
(all targets under `tests/*.rs`, not just `--lib`) inside this worktree,
and only commit to `local-fuzz-findings` -- never `main`. Keep anything
touching the SMT/affine checker or the parallel backend-dispatch
functions (`c_type_name`/`format_declarator`/`c_element_storage`,
`llvm_type`/`llvm_byte_size`, etc.) off-limits for the local model --
this project's own history shows that class of change needs the deeper
reasoning a frontier model gives, not a 7B local one.

## Tuning

- `start.sh`: `-p MemoryMax=`/`-p CPUQuota=` per service (env-overridable
  for the model name; edit the script directly for the resource numbers).
- `OLLAMA_MODEL` env var: swap in a larger model (e.g. a 14B) if you
  raise the memory cap; smaller/faster if you want tighter caps.
- `HARNESS_SLEEP`: seconds between cycles.
- `HARNESS_GENERATE_EVERY`: how often to use qwen-generation vs. plain
  mutation (mutation is nearly free, ~20s/cycle; generation is a real
  model call, observed ~1-5 min/cycle depending on load -- see
  "Hardware-driven tuning notes" below). Default `10`.
- `HARNESS_GAP_EVERY`: how often to use `vanic coverage-gaps`-targeted
  generation instead (takes priority over `HARNESS_GENERATE_EVERY` on
  a cycle where both would fire). Default `7`.
- `HARNESS_ATTEMPT_FIXES=0`: disable the one-shot fix-attempt call on
  findings (still logs/stages the finding itself, just skips
  `fix_attempt.md`).
- `HARNESS_AUTOCOMMIT=0`: disable auto-commit, review findings manually
  before committing anything to `local-fuzz-findings`.
- `FEATURES` in `harness.py`: the feature-name/keyword pairs used for
  generation grounding. Add an entry for any language feature you want
  qwen combining more often; `find_example()` just needs a keyword that
  matches an existing filename under `examples/language/english/`.

## Hardware-driven tuning notes (from running this on a modest CPU-only box)

Started with `qwen2.5-coder:7b-instruct-q4_K_M`; switched to
`qwen2.5-coder:1.5b` (the current default) after a cold load of the 7B
model didn't complete even after 15 minutes under normal desktop
contention (this machine has no GPU, 4 cores, and was concurrently
running a browser, an audio synth daemon, and another Claude Code
session -- not an idle benchmark box). The 1.5B model cold-loads in
~20s and answers a short prompt in ~22s total under the same conditions.
If you have more headroom (more cores, less contention, or don't mind
waiting), a bigger model will produce better-quality generated programs
and reports -- this default is chosen for reliability on a typical loaded
desktop, not peak quality.

The FIRST version of `generate_novel_program()` primed qwen with the
FULL `tools/llm_context/bundle.py` context (tens of thousands of
tokens) and reliably timed out even at 240s regardless of model size --
the bottleneck was prompt *length*, not model size. Fixed by grounding
it in just two real example snippets pulled by keyword from
`examples/language/english/` (see `FEATURES`/`find_example()`) plus the
small `bundle.py --section aliases` table -- a few KB instead of tens of
KB. That version completes, but is still genuinely slow on this
CPU-only, contended box: observed ~29 tokens/sec prompt processing and
noticeably slower token generation, so a full generation call
(including a bounded `num_predict` cap to stop it rambling) has taken
40s-4min+ depending on load, and can queue up behind another in-flight
call since Ollama serves one request at a time (`-np 1`) by default.
`draft_report()` (used for real findings, shorter prompt) is the fast
path at ~40s-180s. None of this blocks the loop -- cycles just take
however long the model calls inside them take; `HARNESS_GENERATE_EVERY`
(default `10`) is the knob for how often you pay that cost.

**Both services have `Restart=on-failure`/`RestartSec=15`** -- confirmed
necessary, not just defensive: a mutated test candidate can trigger a
large allocation in the *compiled test binary*, which counts against the
harness's own memory cap (it's a child process in the same cgroup) and
can OOM-kill the harness itself, not just that one test. Without
auto-restart this was observed to silently kill the pipeline, which then
sat dead for over an hour before anyone noticed. With it, systemd brings
the service back automatically (verified by killing the process directly
and confirming a fresh PID appears within `RestartSec`).

## Replicating on another PC

This tooling is committed to `main` so it can be cloned and set up
anywhere -- but `allowed_paths.conf`/`allowed_readonly_paths.conf` ship
as commented-out **templates**, not live config: the real paths are
specific to one machine/user and can't be guessed. First step on any new
machine is always to edit those two files.

### 0. Use a dedicated worktree + branch, not your main checkout

The isolation model throughout this doc assumes the harness runs against
a *separate* checkout from whatever you're actively working in
interactively (with Claude Code or otherwise) -- that's what makes it
safe to run continuously without fear of colliding with your own edits.
From your main `vani-compiler` clone:

```bash
git worktree add -b local-fuzz-findings ../vani-compiler-localfuzz main
```

Then point `allowed_paths.conf` at `../vani-compiler-localfuzz` (its
absolute path) and run everything from `<that worktree>/tools/localfuzz/`
-- not from `tools/localfuzz/` in your main checkout, even though the
files are identical (they're the same tracked files, just checked out
twice via the worktree).

### 1. Pick an isolation mode

Try the simplest possible container run first:

```bash
docker run --rm debian:bookworm-slim echo hi
```

- **Works** -> your kernel has what container runtimes need
  (`CONFIG_CGROUP_BPF`). You can use Docker Compose for isolation instead
  of `systemd-run` -- two services (ollama + harness), hard per-service
  caps via the plain `cpus`/`mem_limit`/`memswap_limit` compose keys (NOT
  `deploy.resources.limits`, see quirks below), bind-mount a dedicated
  worktree read-write and nothing else. Not included as ready-made files
  here (this host couldn't use them, see below), but that's the shape.
- **Fails** with something like `bpf_prog_query(BPF_CGROUP_DEVICE)
  failed: invalid argument` or any other OCI runtime error -> use the
  `systemd-run --user` scripts in this directory as-is. Needs Linux +
  systemd (>= ~245) with cgroup v2 delegated to user sessions -- true on
  most current distros. Check with:
  ```bash
  systemd-run --user --scope -p MemoryMax=100M -- true && echo "works"
  ```

### 2. OS support matrix

| Platform | What works |
|---|---|
| Linux + systemd + working Docker | Either mode; Docker gives real image-level dependency isolation on top of resource caps |
| Linux + systemd, no working Docker (this host) | `systemd-run --user` scripts here, as-is |
| Linux without systemd (Alpine/OpenRC, some embedded distros) | Neither -- would need a different sandboxing primitive (e.g. `bwrap`/`firejail`), not implemented here |
| Windows + WSL2 | Run inside the WSL2 distro. Docker Desktop's WSL2 backend uses a standard Microsoft-maintained kernel (has `CONFIG_CGROUP_BPF`), so the Docker path should just work. `systemd-run` also works if the distro has `systemd=true` set in `/etc/wsl.conf` (supported on modern WSL2/Windows 11). |
| Windows, no WSL | Not supported -- this is bash + systemd + a POSIX toolchain throughout. Use WSL2. |

### 3. Toolchain prerequisites (either isolation mode)

Same as building vani-compiler itself -- see the repo's own `INSTALL.md`.
Debian/Ubuntu: `sudo apt install build-essential z3 llvm clang`. Rust via
[rustup](https://rustup.rs). `python3` for the harness itself (stdlib
only, no pip deps required for the harness -- `zstandard`/`aider-chat`
below are only needed for their specific one-off setup steps).

## Quirks & gotchas encountered building this (useful when replicating)

- **Docker/runc `bpf_prog_query(BPF_CGROUP_DEVICE) failed: invalid
  argument`** = the kernel's `CONFIG_CGROUP_BPF` isn't set. Not fixable
  via any Docker/runc flag or config -- needs a kernel with that
  compiled in. Check: `grep CONFIG_CGROUP_BPF /boot/config-$(uname -r)`.
- **`deploy.resources.limits` in `docker-compose.yml` is Swarm-only** and
  is silently ignored by plain `docker compose up` unless you pass
  `--compatibility`. Use the plain top-level `cpus`/`mem_limit`/
  `memswap_limit` service keys instead -- those apply directly, no flag
  needed. (Set `memswap_limit` equal to `mem_limit`, or Docker's default
  lets a container swap up to 2x its memory cap.)
- **`git config --local` inside a linked git worktree writes to the
  config *shared* with every other worktree of that repo** (including a
  main checkout) -- it is NOT worktree-scoped by default. Don't use it to
  set automation-only identity. Use `GIT_AUTHOR_NAME`/`GIT_AUTHOR_EMAIL`/
  `GIT_COMMITTER_NAME`/`GIT_COMMITTER_EMAIL` env vars on the
  process/service instead -- they don't touch any config file at all.
- **`ReadWritePaths=`/`ReadOnlyPaths=` silently fail** ("No such file or
  directory") when combined with `ProtectHome=tmpfs` under systemd
  (confirmed on systemd 257) -- use `BindPaths=`/`BindReadOnlyPaths=`
  instead; those correctly bind a real path through a tmpfs-replaced
  parent.
- **`IPAddressDeny=`/`IPAddressAllow=` (systemd's cgroup-eBPF network
  filter) require root.** For `--user` (unprivileged) units, systemd logs
  "unit configures an IP firewall, but not running as root" and silently
  allows all traffic through anyway. Don't rely on it for network
  confinement in a rootless setup -- there isn't a rootless equivalent
  wired up here.
- **`systemd-run --user ... --wait` (and `--pty`) need a real controlling
  TTY.** They fail immediately with a generic "Failed to start transient
  service unit: Process org.freedesktop.systemd1 exited with status 1"
  when invoked from a session without one (some automation/tool-calling
  contexts, for instance). Works fine from a normal interactive terminal
  -- if you hit this error, check `tty` returns something other than
  "not a tty" before assuming the sandboxing itself is broken.
- **Ollama's Linux release asset changed from `.tgz` to `.tar.zst`**
  (zstd, not gzip) -- the old `.../download/ollama-linux-amd64.tgz` URL
  now 404s. `tar --zstd` shells out to a system `zstd` binary; if the
  host doesn't have one and you can't `apt install zstd` (no sudo),
  decompress with Python's `zstandard` package instead (see "First-time
  setup" above).
- **Debian 12+'s `pip install --user` refuses with
  "externally-managed-environment"** (PEP 668). `pip install --user
  --break-system-packages <pkg>` still installs to user site-packages
  only (`~/.local/lib/...`), not anything apt-managed -- a safe,
  reversible way around the guard for this kind of tooling.
- **The official Ollama install script auto-enables an *uncapped*
  system-wide `ollama.service`** and needs sudo. Skip it if you want a
  capped, sandboxed instance -- install the plain tarball to a user
  directory instead (see "First-time setup").
- **Aider expects the `ollama_chat/<model>` prefix, not `ollama/<model>`,
  and the `OLLAMA_API_BASE` env var**, not an `--openai-api-base` flag.
- **vanic's own CLI: always put `--backend=c` AFTER the file path**
  (`vanic run file.vani --backend=c`), not before -- historically
  (BUG-42) flags before the path were silently swallowed. `run_vanic()`
  in `harness.py` follows this defensively regardless of whether that's
  still true upstream.
- **Debian 13 (trixie) isn't in vani-compiler's own verified `INSTALL.md`
  compatibility table** (only up to Debian 12/Ubuntu 24.04) -- if you
  build a Docker image for the Option-A path above, pin
  `debian:bookworm-slim`, not the host's own newer Debian release, to
  match what's actually been verified upstream.
