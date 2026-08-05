#!/bin/bash
# Starts Ollama and the fuzz harness, each as its own capped, filesystem-
# sandboxed, unprivileged `systemd --user` service. Can be run on demand,
# or automatically at boot via the (not-tracked-in-this-repo)
# ~/.config/systemd/user/vani-localfuzz-autostart.service, which just
# wraps this script (`systemctl --user enable vani-localfuzz-autostart`
# to turn auto-start on/off; requires `loginctl enable-linger $USER` for
# it to fire without an interactive login). stop.sh stops both services
# either way.
#
# No sudo, no containers -- see README.md for why (this host's kernel
# lacks CONFIG_CGROUP_BPF, which container runtimes need but plain cgroup
# v2 resource control and namespace sandboxing do not).
#
# Filesystem sandboxing: see sandbox_lib.sh for exactly what this
# enforces. In short -- Ollama (the model server) gets access to NOTHING
# but its own binary and model storage, not even the vani-compiler repo
# (it never needs to touch it directly; the harness mediates everything
# over the local HTTP API). The harness gets access only to what's listed
# in allowed_paths.conf/allowed_readonly_paths.conf (by default: the
# vani-compiler-localfuzz worktree + toolchain, nothing else on the PC).
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$HERE/../.." && pwd)"
OLLAMA_DIST="${OLLAMA_DIST:-$HOME/.local/share/vani-localfuzz/ollama-dist}"
OLLAMA_BIN="$OLLAMA_DIST/bin/ollama"
OLLAMA_DATA="$HOME/.local/share/vani-localfuzz/ollama-models"
# Ollama generates an SSH-style identity keypair (id_ed25519) under
# $HOME/.ollama on first use of some code paths, even for plain `serve`
# on some builds. ProtectHome=tmpfs replaces $HOME with an empty,
# per-invocation tmpfs, so that mkdir fails -- confirmed crash-looping
# ("could not create directory mkdir /home/.../.ollama: read-only file
# system") on a from-scratch sandboxed start (e.g. after a host reboot,
# where nothing has ever populated this path inside the tmpfs before).
# Give it a real, persistent, already-allowlisted-tree home for that
# instead of touching the user's actual $HOME/.ollama.
OLLAMA_HOME="$HOME/.local/share/vani-localfuzz/ollama-home"

if [ ! -x "$OLLAMA_BIN" ]; then
  echo "Ollama not found at $OLLAMA_BIN -- see README.md 'First-time setup'." >&2
  exit 1
fi

mkdir -p "$OLLAMA_DATA" "$OLLAMA_HOME/.ollama"
source "$HERE/sandbox_lib.sh"

if systemctl --user is-active --quiet vani-localfuzz-ollama.service 2>/dev/null; then
  echo "vani-localfuzz-ollama already running."
else
  systemd-run --user --unit=vani-localfuzz-ollama \
    -p MemoryMax=2G -p MemorySwapMax=0 -p CPUQuota=150% \
    -p Restart=on-failure -p RestartSec=15 \
    -p ProtectSystem=strict -p ProtectHome=tmpfs \
    -p "BindPaths=$OLLAMA_DATA" -p "BindReadOnlyPaths=$OLLAMA_DIST" \
    -p "BindPaths=$OLLAMA_HOME/.ollama:$HOME/.ollama" \
    -p PrivateTmp=yes -p NoNewPrivileges=yes \
    -E OLLAMA_MODELS="$OLLAMA_DATA" \
    -E OLLAMA_HOST=127.0.0.1:11434 \
    -E LD_LIBRARY_PATH="$OLLAMA_DIST/lib/ollama" \
    -- "$OLLAMA_BIN" serve
  echo -n "waiting for ollama to come up"
  for _ in $(seq 1 30); do
    curl -sf http://127.0.0.1:11434/ >/dev/null 2>&1 && { echo " ok"; break; }
    echo -n "."
    sleep 1
  done
fi

if systemctl --user is-active --quiet vani-localfuzz-harness.service 2>/dev/null; then
  echo "vani-localfuzz-harness already running -- run stop.sh first to restart." >&2
  exit 1
fi

build_sandbox_args   # -> $SANDBOX_ARGS, from allowed_paths.conf / allowed_readonly_paths.conf

systemd-run --user --unit=vani-localfuzz-harness \
  -p MemoryMax=1G -p MemorySwapMax=0 -p CPUQuota=100% \
  -p Restart=on-failure -p RestartSec=15 \
  "${SANDBOX_ARGS[@]}" \
  -E OLLAMA_URL=http://127.0.0.1:11434 \
  -E OLLAMA_MODEL="${OLLAMA_MODEL:-qwen2.5-coder:1.5b}" \
  -E HARNESS_SLEEP="${HARNESS_SLEEP:-20}" \
  -E HARNESS_AUTOCOMMIT="${HARNESS_AUTOCOMMIT:-1}" \
  -E HARNESS_GENERATE_EVERY="${HARNESS_GENERATE_EVERY:-10}" \
  -E GIT_AUTHOR_NAME=vani-localfuzz-bot \
  -E GIT_AUTHOR_EMAIL=localfuzz@vani.local \
  -E GIT_COMMITTER_NAME=vani-localfuzz-bot \
  -E GIT_COMMITTER_EMAIL=localfuzz@vani.local \
  -- python3 "$REPO/tools/localfuzz/harness.py"

cat <<EOF
Started. Logs:
  journalctl --user -u vani-localfuzz-ollama -f
  journalctl --user -u vani-localfuzz-harness -f
Stop with: tools/localfuzz/stop.sh
EOF
