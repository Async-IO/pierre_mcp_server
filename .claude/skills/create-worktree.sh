#!/usr/bin/env bash
# ABOUTME: Creates a git worktree with all necessary environment files
# ABOUTME: Copies .envrc and runs direnv allow automatically

set -euo pipefail

usage() {
    echo "Usage: $0 <branch-name> [worktree-path]"
    echo ""
    echo "Creates a git worktree with proper environment setup."
    echo ""
    echo "Arguments:"
    echo "  branch-name    Name of the branch to create"
    echo "  worktree-path  Optional path for worktree (default: ../pierre_mcp_server-<branch>)"
    echo ""
    echo "Example:"
    echo "  $0 feature/new-api"
    echo "  $0 fix/bug-123 /tmp/bug-fix"
    exit 1
}

if [[ $# -lt 1 ]]; then
    usage
fi

BRANCH_NAME="$1"
MAIN_WORKTREE="$(git rev-parse --show-toplevel)"
WORKTREE_PATH="${2:-$(dirname "$MAIN_WORKTREE")/pierre_mcp_server-${BRANCH_NAME//\//-}}"

echo "Creating worktree for branch: $BRANCH_NAME"
echo "Worktree path: $WORKTREE_PATH"

# Create the worktree and branch
git worktree add -b "$BRANCH_NAME" "$WORKTREE_PATH"

# Copy environment files
echo "Copying environment files..."
cp "$MAIN_WORKTREE/.envrc" "$WORKTREE_PATH/.envrc"

# Copy .mcp.json if it exists
if [[ -f "$MAIN_WORKTREE/.mcp.json" ]]; then
    cp "$MAIN_WORKTREE/.mcp.json" "$WORKTREE_PATH/.mcp.json"
fi

# Run direnv allow if direnv is available
if command -v direnv &> /dev/null; then
    echo "Running direnv allow..."
    cd "$WORKTREE_PATH" && direnv allow
fi

echo ""
echo "Worktree created successfully!"
echo ""
echo "Next steps:"
echo "  cd $WORKTREE_PATH"
echo ""
