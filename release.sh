#!/bin/bash
# Build and package the release binary for the current platform.
# Asset names carry no version, so install.sh works with any tag and with releases/latest.

set -e

RELEASE_DIR="./releases"

case "$(uname -s)" in
  Darwin) OS="macos" ;;
  Linux) OS="linux" ;;
  *) echo "Unsupported OS"; exit 1 ;;
esac
case "$(uname -m)" in
  x86_64) ARCH="x86_64" ;;
  aarch64|arm64) ARCH="arm64" ;;
  *) echo "Unsupported architecture"; exit 1 ;;
esac

cargo build --release

PACKAGE_NAME="sysconfig-${OS}-${ARCH}"
mkdir -p "$RELEASE_DIR"
cp ./target/release/sysconfig "$RELEASE_DIR/$PACKAGE_NAME"
chmod +x "$RELEASE_DIR/$PACKAGE_NAME"
(cd "$RELEASE_DIR" && shasum -a 256 "$PACKAGE_NAME" > "$PACKAGE_NAME.sha256")

echo "✓ Built: $RELEASE_DIR/$PACKAGE_NAME"
echo ""
echo "Upload to an existing release:"
echo "  gh release upload <tag> $RELEASE_DIR/$PACKAGE_NAME $RELEASE_DIR/$PACKAGE_NAME.sha256"
