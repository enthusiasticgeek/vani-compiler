# localfuzz -- continuous local differential-testing harness

Unattended, token-free bug-hunting for vani-compiler using a small local
model (via Ollama) and a deterministic differential-testing harness. Runs
entirely inside capped Docker containers, isolated from your normal
interactive `vani-compiler` checkout and any Claude Code session working
there at the same time.

## Isolation guarantees

- **Filesystem/git**: this whole directory lives in a dedicated git
  worktree (`vani-compiler-localfuzz`, sibling to the real `vani-compiler`
  checkout), on its own branch (`local-fuzz-findings`). Your main checkout
  is never read or written by this pipeline.
- **Resources**: both containers get a hard ceiling in `docker-compose.yml`
  via `cpus`/`mem_limit` (+ `memswap_limit` pinned equal to `mem_limit`, so
  neither container can spill into swap and thrash the whole host). Default
  caps: `ollama` 1.5 CPU / 3GB, `vani-fuzz` 1 CPU / 1GB -- together that's
  2.5 of your 4 cores and 4 of your 11GB, always leaving the rest free for
  an interactive `cargo build` / Claude Code session. This is a real
  ceiling on absolute consumption, not just a priority hint -- note it uses
  the plain `cpus`/`mem_limit` keys, not `deploy.resources.limits` (that
  key is Swarm-only and is silently ignored by plain `docker compose up`).
  `cpu_shares` is also set, but treat it as a minor secondary aid, not the
  guarantee -- it only affects relative priority among cgroups that are
  true siblings in the host's tree, which a container vs. a bare host
  process often isn't.
- **Writes**: the harness only ever creates files under
  `tools/localfuzz/findings/` and appends to `docs/TODO_LOCAL_STAGING.md`,
  and only ever commits to the `local-fuzz-findings` branch. It never
  pushes, never touches `main`, never edits `docs/TODO_CURRENT.md`.
- **Ownership caveat**: the container runs as root (standard Docker
  default), so files it creates in the worktree will be root-owned on the
  host. Harmless (this worktree isn't your working checkout), but if it
  bothers you: `sudo chown -R $USER:$USER ../../` from this directory.

## What it actually does

Not an LLM guessing at bugs -- an LLM assisting a deterministic harness.
Each cycle:

1. Picks a file from `examples/**/*.vani` (1000+ files already in the
   repo, including the `examples/edge_cases/` adversarial corpus) and
   applies 1-2 small text-level mutations (numeric boundary values,
   statement duplication/deletion/reordering, primitive-type swaps).
   Every 5th cycle (`HARNESS_GENERATE_EVERY`), instead asks the local
   model to write a fresh program combining two random language features,
   primed with the project's own `tools/llm_context/bundle.py` context
   (keyword tables, examples, `docs/v1_limitations.md`) so it's not
   guessing at syntax from scratch.
2. Runs the candidate through `vanic check`, then `vanic run` on both
   backends (`--backend=c` and default LLVM), each under a timeout.
3. Flags it if: `check` itself crashes/hangs, either backend
   crashes/hangs, or both backends exit 0 with different stdout
   (backend divergence -- the bug class that found most of this
   project's real bugs historically).
4. On a flag: saves the repro + raw results under
   `tools/localfuzz/findings/<timestamp>-<kind>-<hash>/`, asks the local
   model to draft a terse, honesty-gated staging entry (explicitly
   forbidden from claiming a root cause it hasn't verified), appends it
   to `docs/TODO_LOCAL_STAGING.md`, and commits both to
   `local-fuzz-findings`.

This mirrors the mechanical compile-and-diff method that found nearly
every real bug in this project's history (see `project_vani_compiler_status`
memory) -- the harness does the finding, the local model does the volume
generation and the first-pass writeup.

## First-time setup

Requires Docker + Docker Compose v2 (`docker compose`, not legacy
`docker-compose`).

```bash
cd tools/localfuzz
docker compose build
docker compose up -d ollama
docker compose exec ollama ollama pull qwen2.5-coder:7b-instruct-q4_K_M
```

## Running

```bash
cd tools/localfuzz
docker compose up -d          # starts both services, runs forever
docker compose logs -f vani-fuzz   # watch cycles/findings live
docker compose down           # stop everything
```

One-shot smoke test (single cycle, no loop, useful before trusting it to
run unattended):

```bash
docker compose run --rm vani-fuzz --once
```

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
merged into `main` as-is.

## Fixing findings (manual, not automated)

Deliberately NOT wired into the continuous loop -- auto-editing compiler
source unattended is a different risk tier than generating/flagging test
programs. If you want the local model to attempt a fix for something
staged here, do it as an explicit, separate, manual step, e.g. with
[Aider](https://aider.chat) against the same capped Ollama model:

Requires Aider installed on the host (`pip install aider-chat`), and note
the `ollama_chat/` prefix, not `ollama/` -- that's the prefix Aider/litellm
actually expect for Ollama's chat API:

```bash
cd ../vani-compiler-localfuzz
export OLLAMA_API_BASE=http://localhost:11434
aider --model ollama_chat/qwen2.5-coder:7b-instruct-q4_K_M \
      src/checker.rs   # or whichever file the finding points at
```

Before accepting anything it proposes: run `cargo test --workspace`
(all targets under `tests/*.rs`, not just `--lib`) inside this worktree,
and only commit to `local-fuzz-findings` -- never `main`. Keep anything
touching the SMT/affine checker or the parallel backend-dispatch
functions (`c_type_name`/`format_declarator`/`c_element_storage`,
`llvm_type`/`llvm_byte_size`, etc.) off-limits for the local model --
this project's own history shows that class of change needs the deeper
reasoning a frontier model gives, not a 7B local one.

## Tuning

- `docker-compose.yml`: `cpus`/`memory`/`cpu_shares` per service.
- `OLLAMA_MODEL` env var: swap in a larger model (e.g. a 14B) if you
  raise the memory cap; smaller/faster if you want tighter caps.
- `HARNESS_SLEEP`: seconds between cycles.
- `HARNESS_GENERATE_EVERY`: how often to use LLM-generation vs. plain
  mutation (mutation is nearly free; generation costs a model call).
- `HARNESS_AUTOCOMMIT=0`: disable auto-commit, review findings manually
  before committing anything to `local-fuzz-findings`.
