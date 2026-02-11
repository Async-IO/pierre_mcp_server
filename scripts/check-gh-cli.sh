#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025 Pierre Fitness Intelligence
# ABOUTME: Checks if gh CLI is installed and authenticated
# ABOUTME: Auto-installs gh via curl for CCFW compatibility

GH_INSTALL_DIR="$HOME/.local/bin"

# Function to install gh CLI via curl
install_gh() {
    echo "📦 Installing gh CLI..."
    
    # Create install directory
    mkdir -p "$GH_INSTALL_DIR"
    
    # Detect architecture
    ARCH=$(uname -m)
    case "$ARCH" in
        x86_64) ARCH="amd64" ;;
        aarch64|arm64) ARCH="arm64" ;;
        *) echo "❌ Unsupported architecture: $ARCH"; exit 0 ;;
    esac
    
    # Detect OS
    OS=$(uname -s | tr '[:upper:]' '[:lower:]')
    
    # Get latest version
    VERSION=$(curl -s https://api.github.com/repos/cli/cli/releases/latest | grep '"tag_name":' | sed -E 's/.*"v([^"]+)".*/\1/')
    
    if [ -z "$VERSION" ]; then
        echo "❌ Could not determine latest gh version"
        exit 0
    fi
    
    # Download and extract
    TARBALL="gh_${VERSION}_${OS}_${ARCH}.tar.gz"
    URL="https://github.com/cli/cli/releases/download/v${VERSION}/${TARBALL}"
    
    echo "   Downloading gh v${VERSION} for ${OS}/${ARCH}..."
    
    if curl -sL "$URL" | tar xz -C /tmp; then
        mv "/tmp/gh_${VERSION}_${OS}_${ARCH}/bin/gh" "$GH_INSTALL_DIR/gh"
        chmod +x "$GH_INSTALL_DIR/gh"
        rm -rf "/tmp/gh_${VERSION}_${OS}_${ARCH}"
        echo "✅ gh CLI installed to $GH_INSTALL_DIR/gh"
    else
        echo "❌ Failed to download gh CLI"
        exit 0
    fi
}

# Add local bin to PATH if not already there
if [[ ":$PATH:" != *":$GH_INSTALL_DIR:"* ]]; then
    export PATH="$GH_INSTALL_DIR:$PATH"
fi

# Check if gh is installed, install if not
if ! command -v gh &> /dev/null; then
    install_gh
    # Refresh shell's command hash after installation
    hash -r 2>/dev/null || true
fi

# Verify installation using direct file check (command -v may have stale cache)
if [ ! -x "$GH_INSTALL_DIR/gh" ] && ! command -v gh &> /dev/null; then
    echo "❌ gh CLI installation failed"
    exit 0
fi

# Check if gh is authenticated
if ! gh auth status &> /dev/null; then
    echo "⚠️  GH CLI NOT AUTHENTICATED - GitHub token required"
    echo "   Please provide a GitHub Personal Access Token with 'repo' and 'workflow' scopes."
    echo "   Once provided, run: echo '<token>' | gh auth login --with-token"
    exit 0
fi

echo "✅ gh CLI ready - can monitor workflows"
