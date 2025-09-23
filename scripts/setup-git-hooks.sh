#!/bin/bash
# ABOUTME: Script to install git hooks for code quality enforcement
# ABOUTME: Sets up pre-commit hook to block AI-generated commit messages and validate code quality

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

# Install pre-commit hook
echo "📝 Installing pre-commit hook..."
cp "$PROJECT_ROOT/.githooks/pre-commit" "$PROJECT_ROOT/.git/hooks/pre-commit"
chmod +x "$PROJECT_ROOT/.git/hooks/pre-commit"

# Set git hooks path (optional, for team consistency)
echo "⚙️  Configuring git hooks path..."
git config core.hooksPath .githooks

echo ""
echo "✅ Git hooks installed successfully!"
echo ""
echo "The pre-commit hook will now:"
echo "  - Block commits with AI-generated signatures (🤖, Claude references, etc.)"
echo "  - Enforce clean, human-written commit messages"
echo ""
echo "To bypass the hook in emergencies, use: git commit --no-verify"
echo "But please only use this for legitimate emergencies!"
echo ""
echo "Hook location: .git/hooks/pre-commit"
echo "Hook source: .githooks/pre-commit"