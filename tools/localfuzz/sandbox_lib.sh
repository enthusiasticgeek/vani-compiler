# Shared helper: builds systemd-run --user filesystem-sandboxing arguments
# from the allowed_paths.conf / allowed_readonly_paths.conf allowlists.
# Sourced by start.sh and run-sandboxed.sh -- not meant to be run directly.
#
# Security model (verified empirically on this host, systemd 257,
# unprivileged --user units):
#   - ProtectSystem=strict makes the whole filesystem read-only except
#     /dev, /proc, /sys and whatever is explicitly bound in.
#   - ProtectHome=tmpfs replaces /home, /root, and /run/user/$UID with an
#     empty, per-unit, non-persistent tmpfs -- nothing under /home is
#     visible AT ALL (not even read-only) except paths explicitly bound
#     back in below. Confirmed: a sandboxed unit sees only the allowed
#     entries under /home/virgo/source, nothing else -- other projects,
#     personal files, etc. are invisible, not just unwritable.
#   - Paths are bound back in with BindPaths=/BindReadOnlyPaths=, NOT
#     ReadWritePaths=/ReadOnlyPaths= -- the latter silently fail
#     ("No such file or directory") when combined with ProtectHome=tmpfs
#     on this systemd version; BindPaths=/BindReadOnlyPaths= is the
#     directive that actually works for punching a real path through a
#     tmpfs-replaced parent.
#   - PrivateTmp=yes gives the unit its own isolated /tmp (harness.py's
#     scratch dir lives there, no extra config needed).
#   - NoNewPrivileges=yes blocks privilege escalation via setuid binaries.
#
# NOT covered: network egress. IPAddressDeny=/IPAddressAllow= was tested
# and requires root to actually take effect -- systemd logs "unit
# configures an IP firewall, but not running as root" and silently lets
# all traffic through for --user units on this host. Not included here to
# avoid claiming a restriction that doesn't actually apply. The practical
# mitigation in place is that Ollama binds to 127.0.0.1 only (no inbound
# exposure); true egress confinement would need a system-level (root)
# service or a separate network-namespace tool, neither set up here.

_localfuzz_here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

_read_path_list() {
  local file="$1"
  [ -f "$file" ] || return 0
  grep -vE '^[[:space:]]*(#|$)' "$file"
}

# Populates the SANDBOX_ARGS array (for use as "${SANDBOX_ARGS[@]}" in a
# systemd-run invocation) from allowed_paths.conf (read-write) and
# allowed_readonly_paths.conf (read-only), both in this directory.
build_sandbox_args() {
  SANDBOX_ARGS=(
    -p ProtectSystem=strict
    -p ProtectHome=tmpfs
    -p PrivateTmp=yes
    -p NoNewPrivileges=yes
  )
  local p
  while IFS= read -r p; do
    [ -n "$p" ] && SANDBOX_ARGS+=(-p "BindPaths=$p")
  done < <(_read_path_list "$_localfuzz_here/allowed_paths.conf")
  while IFS= read -r p; do
    [ -n "$p" ] && SANDBOX_ARGS+=(-p "BindReadOnlyPaths=$p")
  done < <(_read_path_list "$_localfuzz_here/allowed_readonly_paths.conf")
}
