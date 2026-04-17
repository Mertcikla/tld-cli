#!/bin/sh
set -e

BINARY="tld"
REPO="Mertcikla/tld-cli"
INSTALL_DIR="/usr/local/bin"

# Detect OS and Architecture
OS_UNAME=$(uname -s)
ARCH_UNAME=$(uname -m)

case "$OS_UNAME" in
    Darwin) OS="Darwin" ;;
    Linux)  OS="Linux" ;;
    *) echo "Unsupported OS: $OS_UNAME"; exit 1 ;;
esac

case "$ARCH_UNAME" in
    x86_64) ARCH="x86_64" ;;
    arm64|aarch64) ARCH="arm64" ;;
    *) echo "Unsupported architecture: $ARCH_UNAME"; exit 1 ;;
esac

# Resolve the latest release and matching asset
RELEASE_JSON=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest")
VERSION=$(printf '%s\n' "$RELEASE_JSON" | sed -n 's/.*"tag_name": "\([^"]*\)".*/\1/p' | head -n 1)

if [ -z "$VERSION" ]; then
    echo "Could not find latest version for $REPO"
    exit 1
fi

FILENAME="tld_${OS}_${ARCH}.tar.gz"
URL=$(printf '%s\n' "$RELEASE_JSON" | sed -n 's/.*"browser_download_url": "\([^"]*\)".*/\1/p' | grep "/$FILENAME$" | head -n 1)

if [ -z "$URL" ]; then
    echo "Could not find release asset $FILENAME in $REPO $VERSION"
    exit 1
fi

echo "Downloading $BINARY $VERSION for $OS/$ARCH..."

# Download and Install
TMP_DIR=$(mktemp -d)
curl -LsSf "$URL" -o "$TMP_DIR/$FILENAME"
tar -xzf "$TMP_DIR/$FILENAME" -C "$TMP_DIR"

echo "Installing to $INSTALL_DIR (may require sudo)..."
if [ -w "$INSTALL_DIR" ]; then
    mv "$TMP_DIR/$BINARY" "$INSTALL_DIR/$BINARY"
else
    sudo mv "$TMP_DIR/$BINARY" "$INSTALL_DIR/$BINARY"
fi

# Cleanup and Verify
rm -rf "$TMP_DIR"
chmod +x "$INSTALL_DIR/$BINARY"

echo "--------------------------------------------------"
echo "Successfully installed! Run 'tld' to get started."
