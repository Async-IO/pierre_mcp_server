#!/bin/bash
# ABOUTME: Local Linux environment test script using Docker
# ABOUTME: Tests platform compatibility in Linux container for cross-platform validation
#
# Licensed under either of Apache License, Version 2.0 or MIT License at your option.
# Copyright ©2025 Async-IO.org

# Test Linux-specific code locally using Docker

set -e

echo "🐧 Testing Linux-specific code in Docker..."

# Use the same Rust version as CI
docker run --rm \
    -v "$(pwd)":/workspace \
    -w /workspace \
    rust:1.87.0 \
    bash -c "
        echo '📦 Installing clippy...'
        rustup component add clippy

        echo '🔍 Running clippy with Linux target...'
        cargo clippy --all-targets --all-features --quiet -- \
            -W clippy::all \
            -W clippy::pedantic \
            -W clippy::nursery \
            -D warnings

        echo '✅ Linux-specific clippy check complete!'
    "

echo "🎉 All Linux checks passed locally!"