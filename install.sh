#!/usr/bin/env sh
# install.sh — install vanic on Linux / macOS
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/enthusiasticgeek/vani-compiler/main/install.sh | sh
#   sh install.sh [--prefix /usr/local]

set -e

REPO="enthusiasticgeek/vani-compiler"
BIN="vanic"
PREFIX="${INSTALL_PREFIX:-/usr/local}"

# ── parse args ────────────────────────────────────────────────────────────────
while [ $# -gt 0 ]; do
  case "$1" in
    --prefix) PREFIX="$2"; shift 2 ;;
    *)        echo "unknown option: $1" >&2; exit 1 ;;
  esac
done

DEST="$PREFIX/bin"

# ── detect platform ──────────────────────────────────────────────────────────
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux)
    case "$ARCH" in
      x86_64)         ARCHIVE="vanic-linux-x86_64.tar.gz" ;;
      aarch64|arm64)  ARCHIVE="vanic-linux-aarch64.tar.gz" ;;
      *)              echo "Unsupported Linux arch: $ARCH" >&2; exit 1 ;;
    esac
    ;;
  Darwin)
    case "$ARCH" in
      x86_64)         ARCHIVE="vanic-macos-x86_64.tar.gz" ;;
      arm64)          ARCHIVE="vanic-macos-aarch64.tar.gz" ;;
      *)              echo "Unsupported macOS arch: $ARCH" >&2; exit 1 ;;
    esac
    ;;
  *)
    echo "Unsupported OS: $OS" >&2
    exit 1
    ;;
esac

# ── fetch latest tag ─────────────────────────────────────────────────────────
if command -v curl >/dev/null 2>&1; then
  FETCH="curl -fsSL"
elif command -v wget >/dev/null 2>&1; then
  FETCH="wget -qO-"
else
  echo "curl or wget required" >&2; exit 1
fi

TAG="$($FETCH "https://api.github.com/repos/$REPO/releases/latest" \
     | grep '"tag_name"' | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')"

if [ -z "$TAG" ]; then
  echo "Could not determine latest release tag." >&2
  exit 1
fi

URL="https://github.com/$REPO/releases/download/$TAG/$ARCHIVE"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

echo "Downloading vanic $TAG ($ARCHIVE)…"
$FETCH "$URL" > "$TMPDIR/$ARCHIVE"
tar xzf "$TMPDIR/$ARCHIVE" -C "$TMPDIR"

# ── install ──────────────────────────────────────────────────────────────────
mkdir -p "$DEST"
install -m 755 "$TMPDIR/$BIN" "$DEST/$BIN"

echo "Installed $BIN $TAG → $DEST/$BIN"
echo ""
echo "Make sure $DEST is in your PATH, then run:"
echo "  vanic --version"
