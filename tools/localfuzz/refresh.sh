#!/bin/bash
# Keeps the localfuzz worktree's compiler binary from going stale.
#
# Why this exists: on 2026-08-04 the harness had been fuzzing a vanic
# binary built from a commit 2 days / 25 bugfixes behind main, so a large
# fraction of its "findings" (confirmed: at least BUG-76's and BUG-88's
# whole clusters, ~19 of 84) were bugs already fixed on main -- pure
# wasted cycles re-discovering dead bugs instead of finding new ones.
#
# Stops the harness (never rebuild/merge while it's mid-cycle against
# this worktree), merges main into local-fuzz-findings, rebuilds
# --release, and restarts. Safe to run unattended (systemd timer) or by
# hand. Every failure mode below leaves the PREVIOUS known-good binary
# in place rather than limping on with something broken or half-built.
set -u
# 2026-08-05: the systemd --user timer's environment doesn't source
# ~/.bashrc/~/.profile, so `cargo` (installed under ~/.cargo/bin via
# rustup) isn't on PATH when this script runs unattended -- the
# nightly refresh silently failed with "cargo: command not found",
# merged main into the worktree anyway (git doesn't need cargo), but
# left the OLD binary in place, so the harness kept fuzzing a stale
# build without any obvious signal beyond a line in refresh.log. This
# script's own docstring promises "safe to run unattended (systemd
# timer)" -- source cargo's env file here (its own PATH-prepend logic
# is idempotent, safe to source from either an interactive shell or a
# bare systemd unit) so that promise actually holds.
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
cd "$(dirname "${BASH_SOURCE[0]}")" || exit 1   # -> tools/localfuzz
LOCALFUZZ_DIR="$(pwd)"
WORKTREE_DIR="$(cd ../.. && pwd)"               # -> worktree root
LOG="$LOCALFUZZ_DIR/refresh.log"

log() { echo "[$(date -u +%Y-%m-%dT%H:%M:%SZ)] $*" | tee -a "$LOG"; }

log "=== refresh starting ==="

log "stopping harness + ollama"
"$LOCALFUZZ_DIR/stop.sh" >>"$LOG" 2>&1

cd "$WORKTREE_DIR" || { log "FATAL: cannot cd to worktree $WORKTREE_DIR"; exit 1; }

# allowed_paths.conf / allowed_readonly_paths.conf are PERMANENTLY locally
# modified by design (main tracks template content, this machine's real
# absolute paths are deliberately never committed -- see README's
# "Filesystem allowlist" section) -- excluded from the dirty-check, not a
# sign of unexpected work sitting in the tree.
DIRTY="$(git status --porcelain -- . \
    ':!tools/localfuzz/allowed_paths.conf' \
    ':!tools/localfuzz/allowed_readonly_paths.conf')"
if [ -n "$DIRTY" ]; then
    log "ABORT: worktree has uncommitted changes -- refusing to touch it unattended. Investigate by hand."
    log "$DIRTY"
    "$LOCALFUZZ_DIR/start.sh" >>"$LOG" 2>&1
    exit 1
fi

BEFORE_SHA="$(git rev-parse HEAD)"

export GIT_AUTHOR_NAME="localfuzz-refresh" GIT_AUTHOR_EMAIL="localfuzz@localhost"
export GIT_COMMITTER_NAME="localfuzz-refresh" GIT_COMMITTER_EMAIL="localfuzz@localhost"

log "fetching + checking main for new commits"
if ! git fetch origin main >>"$LOG" 2>&1; then
    log "WARN: fetch failed (offline?) -- merging local main as-is"
fi

BEHIND="$(git rev-list --count HEAD..main 2>/dev/null || echo 0)"
if [ "$BEHIND" = "0" ]; then
    log "already up to date with main -- nothing to merge"
else
    log "main is $BEHIND commit(s) ahead -- merging"
    if ! git merge main -m "refresh.sh: merge main ($BEHIND new commit(s)) to keep fuzzed binary current" >>"$LOG" 2>&1; then
        log "MERGE CONFLICT -- aborting merge, leaving worktree as it was. Needs a human pass (see tools/localfuzz/README.md's replicate/merge notes)."
        git merge --abort >>"$LOG" 2>&1
        "$LOCALFUZZ_DIR/start.sh" >>"$LOG" 2>&1
        exit 1
    fi
    log "merge OK, new HEAD $(git rev-parse HEAD)"
fi

log "building --release"
if cargo build --release >>"$LOG" 2>&1; then
    log "build OK"
else
    log "BUILD FAILED -- previous binary (if any) is left in place by cargo; NOT rolling back the merge, since main itself should build. Needs investigation before the next refresh."
fi

if [ ! -x "$WORKTREE_DIR/target/release/vanic" ]; then
    log "FATAL: no vanic binary at target/release/vanic after build -- cannot start harness against nothing. Leaving services stopped."
    exit 1
fi

log "starting harness + ollama"
"$LOCALFUZZ_DIR/start.sh" >>"$LOG" 2>&1

log "=== refresh done (was $BEFORE_SHA, now $(git rev-parse HEAD)) ==="
