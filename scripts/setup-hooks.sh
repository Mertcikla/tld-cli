#!/bin/bash
set -e

HOOK_SOURCE="scripts/pre-commit.sh"
HOOK_DEST=".git/hooks/pre-commit"

if [ ! -d ".git" ]; then
    echo "Error: .git directory not found. Are you in the repository root?"
    exit 1
fi

echo "Installing pre-commit hook..."
cp "$HOOK_SOURCE" "$HOOK_DEST"
chmod +x "$HOOK_DEST"

echo "Pre-commit hook installed successfully at $HOOK_DEST"
