#!/bin/bash
set -e

echo "Running pre-commit formatting..."

# Ensure we are in the repo root
ROOT_DIR=$(git rev-parse --show-toplevel)
cd "$ROOT_DIR"

# Get staged rust files
STAGED_RS_FILES=$(git diff --name-only --cached --diff-filter=d | grep '\.rs$' || true)

if [ -n "$STAGED_RS_FILES" ]; then
    echo "Files to format: $STAGED_RS_FILES"
    
    # Run formatter on the whole workspace to be safe and consistent
    cargo fmt --all
    
    # Re-stage the files that were already staged
    # We use xargs to handle potential large list of files
    echo "$STAGED_RS_FILES" | xargs git add
    
    echo "Formatting complete and files re-staged."
else
    echo "No Rust files staged, skipping formatting."
fi
