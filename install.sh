#!/bin/sh
# Zen installer — https://github.com/vistralis/zen
# Usage: curl -sSf https://raw.githubusercontent.com/vistralis/zen/main/install.sh | sh
set -e

REPO="vistralis/zen"
INSTALL_DIR="${ZEN_INSTALL_DIR:-$HOME/.local/bin}"

# Detect architecture
ARCH=$(uname -m)
case "$ARCH" in
    x86_64|amd64)   ARCH_TAG="x86_64" ;;
    aarch64|arm64)   ARCH_TAG="aarch64" ;;
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

# Detect libc: use musl build if glibc is too old or absent
LIBC_SUFFIX=""
GLIBC_MIN_MAJOR=2
GLIBC_MIN_MINOR=39

if command -v ldd >/dev/null 2>&1; then
    GLIBC_VER=$(ldd --version 2>&1 | head -1 | grep -oE '[0-9]+\.[0-9]+' | tail -1)
    if [ -n "$GLIBC_VER" ]; then
        GLIBC_MAJOR=$(echo "$GLIBC_VER" | cut -d. -f1)
        GLIBC_MINOR=$(echo "$GLIBC_VER" | cut -d. -f2)
        if [ "$GLIBC_MAJOR" -lt "$GLIBC_MIN_MAJOR" ] 2>/dev/null || \
           { [ "$GLIBC_MAJOR" -eq "$GLIBC_MIN_MAJOR" ] && [ "$GLIBC_MINOR" -lt "$GLIBC_MIN_MINOR" ]; } 2>/dev/null; then
            LIBC_SUFFIX="-musl"
            echo "Detected glibc $GLIBC_VER (< $GLIBC_MIN_MAJOR.$GLIBC_MIN_MINOR), using static (musl) binary."
        fi
    else
        LIBC_SUFFIX="-musl"
        echo "Could not detect glibc version, using static (musl) binary."
    fi
else
    LIBC_SUFFIX="-musl"
    echo "No glibc detected (musl/Alpine system?), using static (musl) binary."
fi

ARTIFACT="zen-${ARCH_TAG}-linux${LIBC_SUFFIX}"

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
    *":${INSTALL_DIR}:"*) echo "✓ ${INSTALL_DIR} is already in PATH" ;;
    *) echo "⚠ ${INSTALL_DIR} is not in PATH" ;;
esac

echo ""
echo "Add these lines to ~/.bashrc (or ~/.zshrc), in this order:"
echo ""
echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
echo "  eval \"\$(zen hook bash)\"    # or: eval \"\$(zen hook zsh)\""
echo ""
echo "Then restart your shell or run: source ~/.bashrc"
echo ""
echo "Run 'zen --version' to verify."
