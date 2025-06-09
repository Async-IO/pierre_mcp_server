#!/bin/bash
# Setup script for Pierre MCP Server

echo "Setting up Pierre MCP Server..."

# Check if direnv is installed
if ! command -v direnv &> /dev/null; then
    echo "direnv is not installed. Please install it first:"
    echo "  macOS: brew install direnv"
    echo "  Linux: apt-get install direnv or similar"
    exit 1
fi

# Copy .envrc.example if .envrc doesn't exist
if [ ! -f .envrc ]; then
    echo "Creating .envrc from example..."
    cp .envrc.example .envrc
    echo "Please edit .envrc with your API credentials"
else
    echo ".envrc already exists"
fi

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "Rust is not installed. Installing via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    source $HOME/.cargo/env
fi

echo ""
echo "Setup complete! Next steps:"
echo "1. Edit .envrc with your API credentials"
echo "2. Run 'direnv allow' to load environment variables"
echo "3. Run 'cargo build' to build the project"
echo "4. Run 'cargo run' to start the server"