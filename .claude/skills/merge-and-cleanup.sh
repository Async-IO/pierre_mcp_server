#!/usr/bin/env bash
# ABOUTME: Squash merges a feature branch to main and cleans up worktree
# ABOUTME: Run from main worktree after CI passes on feature branch

set -euo pipefail

MAIN_WORKTREE="$(git rev-parse --show-toplevel)"
LAST_BRANCH_FILE="$MAIN_WORKTREE/.claude/skills/.last-feature-branch"

usage() {
    echo "Usage: $0 [branch-name] [worktree-path]"
    echo ""
    echo "Squash merges a feature branch to main and cleans up."
    echo "Run this from the main worktree after CI passes."
    echo ""
    echo "If no arguments provided, uses the last branch from finish-worktree.sh"
    echo ""
    echo "Arguments:"
    echo "  branch-name    Name of the feature branch to merge (optional)"
    echo "  worktree-path  Optional worktree path"
    echo ""
    echo "Example:"
    echo "  $0                    # uses last feature branch"
    echo "  $0 feature/new-api"
    exit 1
}

# Get branch info from argument or saved file
if [[ $# -ge 1 ]]; then
    BRANCH_NAME="$1"
    WORKTREE_PATH="${2:-$(dirname "$MAIN_WORKTREE")/pierre_mcp_server-${BRANCH_NAME//\//-}}"
elif [[ -f "$LAST_BRANCH_FILE" ]]; then
    SAVED_INFO=$(cat "$LAST_BRANCH_FILE")
    BRANCH_NAME="${SAVED_INFO%%|*}"
    WORKTREE_PATH="${SAVED_INFO##*|}"
    echo "Using saved branch: $BRANCH_NAME"
    echo "Worktree: $WORKTREE_PATH"
    echo ""
else
    echo "Error: No branch specified and no saved branch found."
    echo "Run finish-worktree.sh first, or specify branch name."
    echo ""
    usage
fi

CURRENT_BRANCH=$(git branch --show-current)

# Verify we're on main
if [[ "$CURRENT_BRANCH" != "main" ]]; then
    echo "Error: Must be on main branch. Currently on: $CURRENT_BRANCH"
    echo "Run: git checkout main"
    exit 1
fi

# Verify the branch exists
if ! git rev-parse --verify "$BRANCH_NAME" &>/dev/null; then
    echo "Error: Branch '$BRANCH_NAME' does not exist"
    exit 1
fi

echo "Merging branch: $BRANCH_NAME"
echo "Worktree path: $WORKTREE_PATH"
echo ""

# Pull latest main
echo "Pulling latest main..."
git pull origin main

# Squash merge
echo "Squash merging $BRANCH_NAME..."
git merge --squash "$BRANCH_NAME"

echo ""
echo "Squash merge staged. Please review and commit:"
echo ""
git status --short
echo ""

# Prompt for commit message
read -r -p "Enter commit message (or press Enter for editor): " COMMIT_MSG

if [[ -n "$COMMIT_MSG" ]]; then
    git commit -m "$COMMIT_MSG

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
else
    git commit
fi

# Push main
echo ""
echo "Pushing main..."
git push origin main

# Remove worktree if it exists
if [[ -d "$WORKTREE_PATH" ]]; then
    echo ""
    echo "Removing worktree at $WORKTREE_PATH..."
    git worktree remove "$WORKTREE_PATH"
else
    echo ""
    echo "Worktree not found at $WORKTREE_PATH (may have been removed manually)"
fi

# Delete branch
echo "Deleting branch $BRANCH_NAME..."
git branch -d "$BRANCH_NAME"

# Clean up saved branch file
rm -f "$LAST_BRANCH_FILE"

echo ""
echo "Done! Branch $BRANCH_NAME has been merged and cleaned up."
echo ""
