#!/usr/bin/env bash
set -euo pipefail

REPO="iceberghbs/cmdplz"
BIN_NAME="cmd-engine"
INSTALL_DIR="$HOME/.local/bin"

# ── detect OS / arch ──────────────────────────────────────────
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"

case "$ARCH" in
  x86_64|amd64)  ARCH="x86_64" ;;
  aarch64|arm64) ARCH="aarch64" ;;
  *)
    echo "Unsupported architecture: $ARCH"
    exit 1
    ;;
esac

case "$OS" in
  linux|darwin) ;;
  *)
    echo "Unsupported OS: $OS"
    exit 1
    ;;
esac

ARCHIVE="cmd-engine-${OS}-${ARCH}.tar.gz"
DOWNLOAD_URL="https://github.com/${REPO}/releases/latest/download/${ARCHIVE}"

# ── download ──────────────────────────────────────────────────
echo ""
echo "╔══════════════════════════════════════════╗"
echo "║          cmdplz installer                ║"
echo "╚══════════════════════════════════════════╝"
echo ""
echo "  OS/arch: ${OS}-${ARCH}"
echo "  Downloading ${ARCHIVE}..."
echo ""

mkdir -p "$INSTALL_DIR"

TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

curl -fsSL "$DOWNLOAD_URL" -o "$TMPDIR/${ARCHIVE}"

# ── extract ───────────────────────────────────────────────────
tar -xzf "$TMPDIR/${ARCHIVE}" -C "$TMPDIR"
chmod +x "$TMPDIR/${BIN_NAME}"
mv "$TMPDIR/${BIN_NAME}" "$INSTALL_DIR/${BIN_NAME}"

echo "  ✓ Installed to ${INSTALL_DIR}/${BIN_NAME}"

# ── PATH ──────────────────────────────────────────────────────
ensure_path() {
  local rc="$1"
  [ ! -f "$rc" ] && return
  if ! grep -q "${INSTALL_DIR}" "$rc" 2>/dev/null; then
    echo "export PATH=\"${INSTALL_DIR}:\$PATH\"" >> "$rc"
    echo "  ✓ Added ${INSTALL_DIR} to PATH in $(basename "$rc")"
  fi
}

ensure_path "$HOME/.bashrc"
ensure_path "$HOME/.zshrc"
ensure_path "$HOME/.config/fish/config.fish"

# ── launch setup wizard ───────────────────────────────────────
echo ""
echo "  ──────────────────────────────────"
echo "  Launching setup wizard..."
echo ""

exec "${INSTALL_DIR}/${BIN_NAME}" setup