#!/bin/bash
# Stops both capped user services started by start.sh.
set -uo pipefail
systemctl --user stop vani-localfuzz-harness.service 2>/dev/null
systemctl --user stop vani-localfuzz-ollama.service 2>/dev/null
echo "Stopped (harness + ollama)."
