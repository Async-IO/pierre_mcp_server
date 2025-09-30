#!/bin/bash

# Script to debug race condition in Linux Docker environment

echo "🐧 Running race condition test in Linux Docker..."

docker run --rm -v $(pwd):/app -w /app rust:1.87 bash -c "
    echo 'Building and running test in Linux...'
    cargo test test_complete_multitenant_workflow --test mcp_multitenant_complete_test -- --nocapture 2>&1 | head -50
"