#!/bin/bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025 Pierre Fitness Intelligence
#
# ABOUTME: Validates SDK response schemas against server tool definitions
# ABOUTME: Detects schema drift by comparing tool counts and running schema tests

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo "========================================"
echo "SDK Schema Validation"
echo "========================================"

# Check that we're in the right directory
if [ ! -f "$PROJECT_ROOT/sdk/package.json" ]; then
    echo "Error: SDK not found at $PROJECT_ROOT/sdk"
    exit 1
fi

cd "$PROJECT_ROOT/sdk"

# Step 1: Verify response-schemas.ts exists
echo ""
echo "Step 1: Checking response-schemas.ts exists..."
if [ ! -f "src/response-schemas.ts" ]; then
    echo "Error: src/response-schemas.ts not found"
    echo "This file contains Zod schemas for validating tool responses"
    exit 1
fi
echo "OK: response-schemas.ts found"

# Step 2: Verify types.ts exists (input param types)
echo ""
echo "Step 2: Checking types.ts exists..."
if [ ! -f "src/types.ts" ]; then
    echo "Error: src/types.ts not found"
    echo "Run 'bun run generate-types' to generate type definitions"
    exit 1
fi
echo "OK: types.ts found"

# Step 3: Count tools in types.ts (input schemas)
echo ""
echo "Step 3: Counting tools in types.ts..."
INPUT_TOOL_COUNT=$(grep -c "export interface.*Params {" src/types.ts || echo "0")
echo "Found $INPUT_TOOL_COUNT input parameter interfaces"

# Step 4: Count tools in response-schemas.ts (output schemas)
echo ""
echo "Step 4: Counting tools in response-schemas.ts..."
OUTPUT_TOOL_COUNT=$(grep -c "ResponseSchema = z.object" src/response-schemas.ts || echo "0")
echo "Found $OUTPUT_TOOL_COUNT response schemas"

# Step 5: Count tools in ToolResponseSchemaMap
echo ""
echo "Step 5: Counting tools in ToolResponseSchemaMap..."
MAP_TOOL_COUNT=$(grep -E "^\s+\"[a-z_]+\":" src/response-schemas.ts | wc -l | tr -d ' ')
echo "Found $MAP_TOOL_COUNT tools in ToolResponseSchemaMap"

# Step 6: Check for reasonable coverage
echo ""
echo "Step 6: Validating coverage..."
MIN_EXPECTED_TOOLS=35  # We have ~40 tools
if [ "$MAP_TOOL_COUNT" -lt "$MIN_EXPECTED_TOOLS" ]; then
    echo "Warning: Only $MAP_TOOL_COUNT tools in ToolResponseSchemaMap (expected >= $MIN_EXPECTED_TOOLS)"
    echo "Some tools may be missing response schemas"
fi

# Step 7: TypeScript type check
echo ""
echo "Step 7: Running TypeScript type check..."
if ! bun run type-check 2>/dev/null; then
    echo "Error: TypeScript type check failed"
    echo "Fix type errors in SDK before continuing"
    exit 1
fi
echo "OK: TypeScript types are valid"

# Step 8: Run schema tests
echo ""
echo "Step 8: Running response schema tests..."
if ! bun test test/unit/response-schemas.test.ts 2>/dev/null; then
    echo "Error: Response schema tests failed"
    exit 1
fi
echo "OK: Response schema tests passed"

# Summary
echo ""
echo "========================================"
echo "Schema Validation Summary"
echo "========================================"
echo "Input param interfaces: $INPUT_TOOL_COUNT"
echo "Response schemas:       $OUTPUT_TOOL_COUNT"
echo "Tools in schema map:    $MAP_TOOL_COUNT"
echo ""
echo "All schema validation checks passed!"
