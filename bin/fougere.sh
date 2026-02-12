#!/usr/bin/env bash
# ABOUTME: Wrapper script that delegates a task to GitHub Copilot CLI.
# ABOUTME: Handles bootstrap (install + auth) and forwards the prompt to copilot -p.
set -euo pipefail

PROMPT="${1:-}"
if [ -z "$PROMPT" ]; then
  echo "Usage: fougere.sh \"<task prompt>\""
  echo "Example: fougere.sh \"What is my name based on git log?\""
  exit 1
fi

STDOUT_FILE="/tmp/fougere-stdout.txt"
STDERR_FILE="/tmp/fougere-stderr.txt"
AUTH_MARKER="$HOME/.copilot/.fougere-authed"

# --- Bootstrap: install copilot CLI if missing ---
if ! command -v copilot &>/dev/null; then
  echo "[fougere] copilot CLI not found, installing..."
  mkdir -p ~/.copilot
  echo '{"store_token_plaintext": true}' > ~/.copilot/config.json

  if ! command -v npm &>/dev/null; then
    echo "[fougere] npm not found, installing Node.js..."
    if command -v apt-get &>/dev/null; then
      apt-get update -qq && apt-get install -y -qq nodejs npm 2>&1
    elif command -v brew &>/dev/null; then
      brew install node 2>&1
    else
      curl -fsSL https://deb.nodesource.com/setup_22.x | bash - && apt-get install -y nodejs 2>&1
    fi
  fi

  npm install -g @github/copilot 2>&1
  hash -r
fi

if ! command -v copilot &>/dev/null; then
  echo "[fougere] FATAL: copilot CLI could not be installed."
  exit 2
fi

echo "[fougere] copilot CLI: $(copilot --version 2>&1)"

# --- Auth check: test once per session ---
if [ ! -f "$AUTH_MARKER" ]; then
  echo "[fougere] Testing authentication..."
  AUTH_OUTPUT=$(copilot -p "Reply with exactly AUTH_OK" --model claude-opus-4.6 --yolo --no-ask-user 2>&1 || true)

  if echo "$AUTH_OUTPUT" | grep -q "AUTH_OK"; then
    echo "[fougere] Authentication OK."
    touch "$AUTH_MARKER"
  elif echo "$AUTH_OUTPUT" | grep -qi "device.*code\|github.com/login/device"; then
    echo ""
    echo "============================================"
    echo "[fougere] COPILOT NEEDS GITHUB LOGIN"
    echo ""
    echo "$AUTH_OUTPUT" | grep -iE "code|http|device|url"
    echo ""
    echo "Go to the URL above, enter the device code,"
    echo "then re-run this script."
    echo "============================================"
    exit 3
  else
    echo "[fougere] AUTH FAILED. Raw output:"
    echo "$AUTH_OUTPUT"
    exit 4
  fi
else
  echo "[fougere] Auth marker found, skipping auth test."
fi

# --- Forward the task to copilot ---
PROJECT_ROOT=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
echo "[fougere] Project root: $PROJECT_ROOT"
echo "[fougere] Forwarding task to copilot..."
echo "[fougere] Prompt: $PROMPT"
echo ""

cd "$PROJECT_ROOT"
set +e
copilot -p "$PROMPT" \
  --model claude-opus-4.6 \
  --yolo \
  --no-ask-user \
  --add-dir . \
  1>"$STDOUT_FILE" 2>"$STDERR_FILE"
EXIT_CODE=$?
set -e

echo ""
echo "============================================"
echo "[fougere] Copilot exit code: $EXIT_CODE"
echo "============================================"
echo ""
echo "--- STDOUT ---"
cat "$STDOUT_FILE"
echo ""
echo "--- STDERR ---"
cat "$STDERR_FILE"
echo ""
echo "[fougere] Done."
exit $EXIT_CODE
