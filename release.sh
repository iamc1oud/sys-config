#!/bin/bash
# Build and package release binaries for GitHub releases

set -e

VERSION="${1:-0.1.0}"
BUILD_DIR="./target/release"
RELEASE_DIR="./releases"

echo "Building sysconfig v${VERSION}..."
cargo build --release

mkdir -p "$RELEASE_DIR"

# Current platform (macOS arm64)
BIN="$BUILD_DIR/sys-config"
if [ ! -f "$BIN" ]; then
  echo "Error: Binary not found at $BIN"
  exit 1
fi

OS="macos"
ARCH="arm64"
PACKAGE_NAME="sys-config-${VERSION}-${OS}-${ARCH}"

echo "Packaging $PACKAGE_NAME..."
cp "$BIN" "$RELEASE_DIR/$PACKAGE_NAME"
chmod +x "$RELEASE_DIR/$PACKAGE_NAME"

# Create checksum
cd "$RELEASE_DIR"
shasum -a 256 "$PACKAGE_NAME" > "$PACKAGE_NAME.sha256"
cd - > /dev/null

echo "✓ Built: $RELEASE_DIR/$PACKAGE_NAME"
echo "✓ Checksum: $RELEASE_DIR/$PACKAGE_NAME.sha256"
echo ""
echo "Next: Upload to GitHub Releases"
echo "  gh release create v${VERSION} releases/*"
