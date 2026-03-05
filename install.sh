#!/bin/bash
set -euo pipefail

REPO="alex454-byte/opencode-tokens"
BINARY="oct"
INSTALL_DIR="${OCT_INSTALL_DIR:-$HOME/.local/bin}"

# Detect platform
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$OS" in
  linux)  OS_TAG="unknown-linux-gnu" ;;
  darwin) OS_TAG="apple-darwin" ;;
  *)      echo "Unsupported OS: $OS"; exit 1 ;;
esac

case "$ARCH" in
  x86_64|amd64)  ARCH_TAG="x86_64" ;;
  arm64|aarch64) ARCH_TAG="aarch64" ;;
  *)             echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

TARGET="${ARCH_TAG}-${OS_TAG}"

# Get latest release tag
LATEST=$(curl -sL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | head -1 | cut -d'"' -f4)

if [ -z "$LATEST" ]; then
  echo "No releases found. Building from source..."
  if ! command -v cargo >/dev/null 2>&1; then
    echo "Rust not installed. Install from https://rustup.rs first."
    exit 1
  fi
  cargo install --git "https://github.com/${REPO}" --root "$HOME/.local"
  echo ""
  echo "Installed oct to $INSTALL_DIR"
  ensure_path
  exit 0
fi

URL="https://github.com/${REPO}/releases/download/${LATEST}/oct-${TARGET}.tar.gz"

echo "Installing oct ${LATEST} for ${TARGET}..."

# Create install dir
mkdir -p "$INSTALL_DIR"

# Download and extract
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

if curl -fsSL "$URL" -o "$TMP/oct.tar.gz"; then
  tar xzf "$TMP/oct.tar.gz" -C "$TMP"
  mv "$TMP/oct" "$INSTALL_DIR/oct"
  chmod +x "$INSTALL_DIR/oct"
else
  echo "Binary not available for ${TARGET}. Building from source..."
  if ! command -v cargo >/dev/null 2>&1; then
    echo "Rust not installed. Install from https://rustup.rs first."
    exit 1
  fi
  cargo install --git "https://github.com/${REPO}" --root "$HOME/.local"
fi

echo ""
echo "Installed oct to $INSTALL_DIR/oct"

# Check PATH
if ! echo "$PATH" | tr ':' '\n' | grep -qx "$INSTALL_DIR"; then
  echo ""
  echo "Add to your shell profile:"
  echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
fi

echo ""
echo "Get started:"
echo "  oct init          # set up for current project"
echo "  oct init --global # set up for all projects"
echo "  oct gain          # check token savings"
