#!/bin/sh
set -e

# --global here means the container's OWN root user gitconfig (isolated
# inside this ephemeral container filesystem) -- never touches the repo.
# Identity for commits comes from GIT_AUTHOR_*/GIT_COMMITTER_* env vars
# (set in docker-compose.yml), deliberately NOT `git config --local`:
# in a linked worktree, --local writes to the config shared with every
# other worktree of this repo, including the main checkout.
git config --global --add safe.directory /repo

exec python3 tools/localfuzz/harness.py "$@"
