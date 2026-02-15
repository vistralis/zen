#!/bin/sh
# Zen installer — https://github.com/vistralis/zen
# Usage: curl -sSf https://raw.githubusercontent.com/vistralis/zen/main/install.sh | sh
set -e

REPO="vistralis/zen"
INSTALL_DIR="${ZEN_INSTALL_DIR:-$HOME/.local/bin}"

# Detect architecture
ARCH=$(uname -m)
case "$ARCH" in
    x86_64|amd64)   ARTIFACT="zen-x86_64-linux" ;;
    aarch64|arm64)   ARTIFACT="zen-aarch64-linux" ;;
    *)
        echo "Error: Unsupported architecture: $ARCH"
        echo "Zen currently supports x86_64 and aarch64 Linux."
        exit 1
        ;;
esac

# Detect OS
OS=$(uname -s)
case "$OS" in
    Linux) ;;
    *)
        echo "Error: Unsupported OS: $OS"
        echo "Zen currently supports Linux only."
        exit 1
        ;;
esac

# Get latest release tag
echo "Fetching latest release..."
LATEST=$(curl -sSf "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/')

if [ -z "$LATEST" ]; then
    echo "Error: Could not determine latest release."
    echo "Check https://github.com/${REPO}/releases"
    exit 1
fi

echo "Latest release: $LATEST"

# Download binary
URL="https://github.com/${REPO}/releases/download/${LATEST}/${ARTIFACT}"
echo "Downloading $ARTIFACT..."

mkdir -p "$INSTALL_DIR"
TMP_FILE=$(mktemp "${INSTALL_DIR}/zen.XXXXXX")
curl -sSfL "$URL" -o "$TMP_FILE"
chmod +x "$TMP_FILE"
mv -f "$TMP_FILE" "${INSTALL_DIR}/zen"

echo ""
echo "✓ Zen installed to ${INSTALL_DIR}/zen"
echo ""

# Check if install dir is in PATH
case ":$PATH:" in
    *":${INSTALL_DIR}:"*) ;;
    *)
        echo "Add this to your shell profile (~/.bashrc or ~/.zshrc):"
        echo ""
        echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
        echo ""
        ;;
esac

echo "Run 'zen --version' to verify."
