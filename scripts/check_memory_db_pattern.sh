#!/bin/bash

# Script to find and fix problematic SQLite memory database patterns
# These patterns create physical files instead of in-memory databases

echo "🔍 Checking for problematic SQLite memory database patterns..."

# Check for existing physical files that shouldn't exist
echo "1. Checking for physical :memory:test_ files..."
if ls :memory:test_* 2>/dev/null; then
    echo "❌ Found physical test database files that should be in-memory!"
    echo "   These files should be cleaned up:"
    ls -la :memory:test_*
    echo "   Removing these files..."
    rm -f :memory:test_*
    echo "   ✅ Files cleaned up"
else
    echo "   ✅ No problematic physical files found"
fi

# Search for problematic patterns in source code
echo ""
echo "2. Searching for problematic database URL patterns..."

# Look for sqlite::memory: with any suffix that would create physical files
if grep -r "sqlite::memory:[^\"']*[a-zA-Z0-9]" --include="*.rs" . 2>/dev/null; then
    echo "❌ Found SQLite memory URLs with suffixes that create physical files!"
    echo "   These should be fixed to use just 'sqlite::memory:'"
else
    echo "   ✅ No problematic memory database patterns found in source"
fi

# Look for format! patterns that might create test database URLs
echo ""
echo "3. Checking for format! usage with memory databases..."
if grep -r "format!.*sqlite.*memory.*test" --include="*.rs" . 2>/dev/null; then
    echo "❌ Found format! patterns that might create test database files!"
else
    echo "   ✅ No problematic format! patterns found"
fi

# Check for any test database isolation patterns
echo ""
echo "4. Checking for test isolation patterns..."
if grep -r "test_.*{.*}" --include="*.rs" . 2>/dev/null | grep -i "database\|sqlite"; then
    echo "⚠️  Found test patterns that might affect database creation"
else
    echo "   ✅ No problematic test isolation patterns found"
fi

echo ""
echo "🎯 Recommendations:"
echo "   - Always use 'sqlite::memory:' for in-memory databases"
echo "   - Never append suffixes to memory database URLs"
echo "   - Use temporary files with tempfile crate for file-based test databases"
echo "   - Ensure test isolation doesn't modify memory database URLs"

echo ""
echo "✅ Check complete!"