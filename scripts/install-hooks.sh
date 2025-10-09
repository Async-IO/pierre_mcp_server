#!/bin/bash
# ABOUTME: Installs git hooks from scripts/hooks/ to .git/hooks/
# ABOUTME: Run this script once after cloning the repository to enable project git hooks
#
# Licensed under either of Apache License, Version 2.0 or MIT License at your option.
# Copyright ©2025 Async-IO.org

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
HOOKS_DIR="$SCRIPT_DIR/hooks"
GIT_HOOKS_DIR="$SCRIPT_DIR/../.git/hooks"

if [ ! -d "$GIT_HOOKS_DIR" ]; then
    echo "❌ ERROR: .git/hooks directory not found. Are you in a git repository?"
    exit 1
fi

echo "📦 Installing git hooks..."

for hook in "$HOOKS_DIR"/*; do
    if [ -f "$hook" ]; then
        hook_name=$(basename "$hook")
        target="$GIT_HOOKS_DIR/$hook_name"

        cp "$hook" "$target"
        chmod +x "$target"
        echo "✅ Installed: $hook_name"
    fi
done

echo ""
echo "✨ All hooks installed successfully!"
echo "   Hooks are now active in .git/hooks/"