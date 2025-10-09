#!/bin/bash
# ABOUTME: Debugging helper for Linux Docker environment race conditions
# ABOUTME: Runs multitenant workflow test in Docker to reproduce Linux-specific issues
#
# Licensed under either of Apache License, Version 2.0 or MIT License at your option.
# Copyright ©2025 Async-IO.org

# Script to debug race condition in Linux Docker environment

echo "🐧 Running race condition test in Linux Docker..."

docker run --rm -v $(pwd):/app -w /app rust:1.87 bash -c "
    echo 'Building and running test in Linux...'
    cargo test test_complete_multitenant_workflow --test mcp_multitenant_complete_test -- --nocapture 2>&1 | head -50
"