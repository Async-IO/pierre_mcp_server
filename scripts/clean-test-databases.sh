#!/bin/bash
# ABOUTME: Clean up accumulated test database files from test_data directory
# ABOUTME: Removes old test databases while preserving directory structure

set -euo pipefail

echo "Cleaning up test database files..."

# Create test_data directory if it doesn't exist
mkdir -p test_data

# Count files before cleanup
BEFORE_COUNT=$(find test_data -name "*.db" | wc -l | tr -d ' ')
BEFORE_SIZE=$(du -sh test_data 2>/dev/null | cut -f1 || echo "0B")

echo "Before cleanup: ${BEFORE_COUNT} database files (${BEFORE_SIZE})"

# Remove all .db files in test_data directory
find test_data -name "*.db" -type f -delete

# Remove any .db-shm and .db-wal SQLite files as well
find test_data -name "*.db-shm" -type f -delete 2>/dev/null || true
find test_data -name "*.db-wal" -type f -delete 2>/dev/null || true

# Count files after cleanup
AFTER_COUNT=$(find test_data -name "*.db" | wc -l | tr -d ' ')
AFTER_SIZE=$(du -sh test_data 2>/dev/null | cut -f1 || echo "0B")

echo "After cleanup: ${AFTER_COUNT} database files (${AFTER_SIZE})"
echo "Removed $((BEFORE_COUNT - AFTER_COUNT)) database files"

# Keep the directory structure for future tests
touch test_data/.gitkeep

echo "Test database cleanup completed!"