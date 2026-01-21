#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025 Pierre Fitness Intelligence
# ABOUTME: Checks if gh CLI is installed and authenticated
# ABOUTME: Outputs guidance for Claude Code to install/authenticate if needed

# Check if gh is installed
if ! command -v gh &> /dev/null; then
    echo "⚠️  GH CLI NOT INSTALLED - Run: brew install gh (macOS) or apt install gh (Linux)"
    echo "   Then authenticate: gh auth login --with-token"
    exit 0
fi

# Check if gh is authenticated
if ! gh auth status &> /dev/null; then
    echo "⚠️  GH CLI NOT AUTHENTICATED"
    echo "   Ask user for GitHub Personal Access Token (needs 'repo' and 'workflow' scopes)"
    echo "   Then run: echo '<token>' | gh auth login --with-token"
    exit 0
fi

echo "✅ gh CLI ready - can monitor workflows"
