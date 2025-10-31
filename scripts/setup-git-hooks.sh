#!/bin/bash
# ABOUTME: Script to install git hooks for code quality enforcement
# ABOUTME: Sets up unified pre-commit hook with all validations (AI messages, suspicious files, etc.)
#
# Licensed under either of Apache License, Version 2.0 or MIT License at your option.
# Copyright ©2025 Async-IO.org

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "🔧 Setting up git hooks for pierre_mcp_server..."

# Check if we're in a git repository
if [ ! -d "$PROJECT_ROOT/.git" ]; then
    echo "❌ Error: Not in a git repository"
    exit 1
fi

# Create git hooks directory if it doesn't exist
mkdir -p "$PROJECT_ROOT/.git/hooks"

# Install unified pre-commit hook from .githooks/
echo "📝 Installing unified pre-commit hook..."
cp "$PROJECT_ROOT/.githooks/pre-commit" "$PROJECT_ROOT/.git/hooks/pre-commit"
chmod +x "$PROJECT_ROOT/.git/hooks/pre-commit"

echo ""
echo "✅ Git hooks installed successfully!"
echo ""
echo "The unified pre-commit hook will now:"
echo "  ✓ Block commits with AI-generated signatures (🤖, Claude references, etc.)"
echo "  ✓ Block claude_docs/ files (AI working notes)"
echo "  ✓ Block root *.md files (except README.md, CHANGELOG.md, CONTRIBUTING.md)"
echo "  ✓ Block suspicious files (.bak, _old, .tmp, etc.)"
echo "  ✓ Enforce clean, human-written commit messages"
echo ""
echo "To bypass the hook in emergencies, use: git commit --no-verify"
echo "But please only use this for legitimate emergencies!"
echo ""
echo "Hook location: .git/hooks/pre-commit"
echo "Hook source: .githooks/pre-commit"