#!/bin/bash
# Runs an arbitrary command filesystem-sandboxed to the same allowlist as
# the harness (allowed_paths.conf / allowed_readonly_paths.conf) -- for
# manual, supervised tools that talk to the local model, e.g. Aider.
#
# Usage:
#   ./run-sandboxed.sh -- aider --model ollama_chat/qwen2.5-coder:7b-instruct-q4_K_M src/checker.rs
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$HERE/../.." && pwd)"
source "$HERE/sandbox_lib.sh"

if [ "${1:-}" = "--" ]; then shift; fi
[ $# -ge 1 ] || { echo "usage: $0 [--] <command> [args...]" >&2; exit 1; }

build_sandbox_args

exec systemd-run --user --pty --wait --collect --same-dir \
  "${SANDBOX_ARGS[@]}" \
  --setenv=PATH \
  --setenv=HOME \
  --setenv=OLLAMA_API_BASE \
  -- "$@"
