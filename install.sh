#!/bin/bash
set -e

VERSION="${1:-latest}"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
OS="$(uname -s)"
ARCH="$(uname -m)"

# Map arch names
case "$ARCH" in
  x86_64) ARCH="x86_64" ;;
  aarch64|arm64) ARCH="arm64" ;;
  *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

# Map OS names
case "$OS" in
  Darwin) OS="macos" ;;
  Linux) OS="linux" ;;
  *) echo "Unsupported OS: $OS"; exit 1 ;;
esac

BINARY_NAME="sys-config-${VERSION}-${OS}-${ARCH}"
GITHUB_REPO="ajay-singh-questions/sys-config"
DOWNLOAD_URL="https://github.com/${GITHUB_REPO}/releases/download/v${VERSION}/${BINARY_NAME}"

echo "Installing sysconfig ${VERSION} for ${OS}/${ARCH}..."
mkdir -p "$INSTALL_DIR"

# Download
echo "Downloading from ${DOWNLOAD_URL}..."
if ! curl -fsSL "$DOWNLOAD_URL" -o "$INSTALL_DIR/sysconfig"; then
  echo "Error: Failed to download. Check version and platform compatibility."
  exit 1
fi

chmod +x "$INSTALL_DIR/sysconfig"

# Verify
if ! "$INSTALL_DIR/sysconfig" --version > /dev/null; then
  echo "Error: Binary verification failed."
  exit 1
fi

echo "✓ Installed to $INSTALL_DIR/sysconfig"

# Check if in PATH
if [[ ":$PATH:" == *":$INSTALL_DIR:"* ]]; then
  echo "✓ $INSTALL_DIR is in PATH"
else
  echo ""
  echo "⚠ Add to PATH by adding this to ~/.bashrc or ~/.zshrc:"
  echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
fi

echo ""
echo "Ready to use: sysconfig init ~/dotfiles"
