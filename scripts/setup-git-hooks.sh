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

# Install pre-push hook from .githooks/
echo "📝 Installing pre-push hook..."
cp "$PROJECT_ROOT/.githooks/pre-push" "$PROJECT_ROOT/.git/hooks/pre-push"
chmod +x "$PROJECT_ROOT/.git/hooks/pre-push"

# Install commit-msg hook from .githooks/
echo "📝 Installing commit-msg hook..."
cp "$PROJECT_ROOT/.githooks/commit-msg" "$PROJECT_ROOT/.git/hooks/commit-msg"
chmod +x "$PROJECT_ROOT/.git/hooks/commit-msg"

echo ""
echo "✅ Git hooks installed successfully!"
echo ""
echo "Pre-commit hook will now:"
echo "  ✓ Block commits with AI-generated signatures (🤖, Claude references, etc.)"
echo "  ✓ Block claude_docs/ files (AI working notes)"
echo "  ✓ Block root *.md files (except README.md, CHANGELOG.md, CONTRIBUTING.md)"
echo "  ✓ Block suspicious files (.bak, _old, .tmp, etc.)"
echo "  ✓ Enforce clean, human-written commit messages"
echo ""
echo "Commit-msg hook will now:"
echo "  ✓ Enforce 1-2 line commit messages (no novels!)"
echo "  ✓ Block AI-generated commit signatures"
echo "  ✓ Validate first line length (max 100 chars)"
echo "  ✓ Encourage conventional commit format"
echo ""
echo "Pre-push hook will now:"
echo "  ✓ Run critical path tests (~5-10 minutes)"
echo "  ✓ Catch 80% of issues before CI runs"
echo "  ✓ Prevent pushing code that fails essential tests"
echo ""
echo "To bypass hooks in emergencies:"
echo "  git commit --no-verify  (skip pre-commit and commit-msg)"
echo "  git push --no-verify    (skip pre-push)"
echo "But please only use this for legitimate emergencies!"
echo ""
echo "Hook locations:"
echo "  Pre-commit: .git/hooks/pre-commit (source: .githooks/pre-commit)"
echo "  Commit-msg: .git/hooks/commit-msg (source: .githooks/commit-msg)"
echo "  Pre-push:   .git/hooks/pre-push (source: .githooks/pre-push)"