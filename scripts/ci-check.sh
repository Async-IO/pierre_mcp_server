#!/bin/bash
# ABOUTME: Continuous integration check script for CI/CD pipeline
# ABOUTME: Runs formatting, linting, and test checks for both Rust and frontend code
#
# Licensed under either of Apache License, Version 2.0 or MIT License at your option.
# Copyright ©2025 Async-IO.org

# Pierre MCP Server - CI Check Script
# Simplified version for CI environments

set -e

echo "🔍 Running CI checks for Pierre MCP Server..."

# Rust checks
echo "📦 Checking Rust formatting..."
cargo fmt --all -- --check

echo "🔍 Running Rust linter..."
cargo clippy --all-targets --all-features -- -D warnings

echo "🧪 Running Rust tests..."
cargo test --all --all-features

# Frontend checks (if exists)
if [ -d "frontend" ] && [ -f "frontend/package.json" ]; then
    echo "🌐 Checking frontend..."
    cd frontend
    
    # Install dependencies if needed
    if [ ! -d "node_modules" ]; then
        npm ci
    fi
    
    # Lint and type check
    npm run lint
    npx tsc --noEmit
    
    # Build check
    npm run build
    
    cd ..
fi

echo "✅ All CI checks passed!"