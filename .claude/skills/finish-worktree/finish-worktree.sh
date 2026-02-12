#!/usr/bin/env bash
# ABOUTME: Prepares a feature branch for merge by rebasing onto main
# ABOUTME: Handles rebase, push, and provides CI monitoring instructions

set -euo pipefail

BRANCH_NAME=$(git branch --show-current)
MAIN_BRANCH="main"

if [[ "$BRANCH_NAME" == "$MAIN_BRANCH" ]]; then
    echo "Error: Already on $MAIN_BRANCH branch. Switch to your feature branch first."
    exit 1
fi

echo "Finishing branch: $BRANCH_NAME"
echo ""

# Fetch latest from origin
echo "Fetching latest from origin..."
git fetch origin "$MAIN_BRANCH"

# Check if rebase is needed
LOCAL_MAIN=$(git rev-parse "origin/$MAIN_BRANCH")
MERGE_BASE=$(git merge-base HEAD "origin/$MAIN_BRANCH")

if [[ "$LOCAL_MAIN" != "$MERGE_BASE" ]]; then
    echo "Rebasing onto origin/$MAIN_BRANCH..."
    git rebase "origin/$MAIN_BRANCH"
    echo "Rebase complete."
else
    echo "Branch is already up to date with $MAIN_BRANCH."
fi

echo ""
echo "Running local validation before push..."
echo ""

# Run validation
cargo fmt --check || { echo "Run 'cargo fmt' first"; exit 1; }
./scripts/ci/architectural-validation.sh
cargo clippy --all-targets

echo ""
echo "Local validation passed!"
echo ""

# Push
echo "Pushing branch to origin..."
git push --force-with-lease origin "$BRANCH_NAME"

# Save branch info for merge-and-cleanup.sh
WORKTREE_PATH="$(git rev-parse --show-toplevel)"
MAIN_WORKTREE="$(cd "$WORKTREE_PATH" && git worktree list --porcelain | grep -A1 "^worktree" | head -1 | sed 's/worktree //')"
echo "$BRANCH_NAME|$WORKTREE_PATH" > "$MAIN_WORKTREE/.claude/skills/.last-feature-branch"

echo ""
echo "Branch pushed successfully!"
echo ""
echo "=========================================="
echo "NEXT STEPS:"
echo "=========================================="
echo ""
echo "1. Monitor CI at:"
echo "   https://github.com/Async-IO/pierre_mcp_server/actions"
echo ""
echo "2. Once CI is GREEN, run from main worktree:"
echo "   cd <main-worktree>"
echo "   ./.claude/skills/merge-and-cleanup.sh"
echo ""
echo "   (Branch info saved - no args needed)"
echo ""
