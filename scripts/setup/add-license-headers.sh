#!/bin/bash
# ABOUTME: Script to add SPDX license headers to source files that are missing them
# ABOUTME: Supports Rust (.rs), TypeScript/JavaScript (.ts/.tsx/.js/.jsx), and HTML files

set -euo pipefail

LICENSE_SPDX="SPDX-License-Identifier: MIT OR Apache-2.0"
LICENSE_COPYRIGHT="Copyright (c) 2025 Pierre Fitness Intelligence"

# Counters (using files to work around subshell issues)
MODIFIED_FILE=$(mktemp)
SKIPPED_FILE=$(mktemp)
echo "0" > "$MODIFIED_FILE"
echo "0" > "$SKIPPED_FILE"

increment_modified() {
    local count=$(cat "$MODIFIED_FILE")
    echo $((count + 1)) > "$MODIFIED_FILE"
}

increment_skipped() {
    local count=$(cat "$SKIPPED_FILE")
    echo $((count + 1)) > "$SKIPPED_FILE"
}

add_header_rust() {
    local file="$1"

    # Check if file already has SPDX license header
    if grep -q "SPDX-License-Identifier" "$file" 2>/dev/null; then
        increment_skipped
        return 0
    fi

    # Check for old-style license header (both // and //! formats) and replace it
    if grep -q "Licensed under either of Apache License" "$file" 2>/dev/null; then
        sed -i.bak \
            -e 's|// Licensed under either of Apache License, Version 2.0 or MIT License at your option.|// SPDX-License-Identifier: MIT OR Apache-2.0|' \
            -e 's|//! Licensed under either of Apache License, Version 2.0 or MIT License at your option.|// SPDX-License-Identifier: MIT OR Apache-2.0|' \
            -e 's|// Copyright .*Async-IO.org|// Copyright (c) 2025 Pierre Fitness Intelligence|' \
            -e 's|//! Copyright .*Async-IO.org|// Copyright (c) 2025 Pierre Fitness Intelligence|' \
            -e 's|// Copyright .*Pierre Fitness Intelligence|// Copyright (c) 2025 Pierre Fitness Intelligence|' \
            "$file"
        rm -f "${file}.bak"
        echo "  ↻ $file (updated old format)"
        increment_modified
        return 0
    fi

    # Create temp file for new header insertion
    local tmpfile=$(mktemp)
    local first_line=$(head -1 "$file")

    if [[ "$first_line" == "// ABOUTME:"* ]]; then
        # Find where ABOUTME comments end
        local aboutme_end=0
        local line_num=0
        while IFS= read -r line; do
            line_num=$((line_num + 1))
            if [[ "$line" == "// ABOUTME:"* ]]; then
                aboutme_end=$line_num
            elif [[ "$line" != "//" && -n "$line" ]]; then
                break
            fi
        done < "$file"

        head -n "$aboutme_end" "$file" > "$tmpfile"
        echo "//" >> "$tmpfile"
        echo "// $LICENSE_SPDX" >> "$tmpfile"
        echo "// $LICENSE_COPYRIGHT" >> "$tmpfile"

        local rest_start=$((aboutme_end + 1))
        local next_line=$(sed -n "${rest_start}p" "$file")
        if [[ "$next_line" == "//" ]]; then
            rest_start=$((rest_start + 1))
        fi
        tail -n "+${rest_start}" "$file" >> "$tmpfile"
    else
        echo "// $LICENSE_SPDX" > "$tmpfile"
        echo "// $LICENSE_COPYRIGHT" >> "$tmpfile"
        echo "" >> "$tmpfile"
        cat "$file" >> "$tmpfile"
    fi

    mv "$tmpfile" "$file"
    echo "  ✓ $file"
    increment_modified
}

add_header_js() {
    local file="$1"

    # Check if file already has SPDX license header
    if grep -q "SPDX-License-Identifier" "$file" 2>/dev/null; then
        increment_skipped
        return 0
    fi

    local tmpfile=$(mktemp)
    local first_line=$(head -1 "$file")

    # Check if file starts with shebang
    if [[ "$first_line" == "#!"* ]]; then
        echo "$first_line" > "$tmpfile"
        echo "// $LICENSE_SPDX" >> "$tmpfile"
        echo "// $LICENSE_COPYRIGHT" >> "$tmpfile"
        echo "" >> "$tmpfile"
        tail -n +2 "$file" >> "$tmpfile"
    else
        echo "// $LICENSE_SPDX" > "$tmpfile"
        echo "// $LICENSE_COPYRIGHT" >> "$tmpfile"
        echo "" >> "$tmpfile"
        cat "$file" >> "$tmpfile"
    fi

    mv "$tmpfile" "$file"
    echo "  ✓ $file"
    increment_modified
}

add_header_html() {
    local file="$1"

    # Check if file already has SPDX license header
    if grep -q "SPDX-License-Identifier" "$file" 2>/dev/null; then
        increment_skipped
        return 0
    fi

    local tmpfile=$(mktemp)
    local first_line=$(head -1 "$file")

    # Check if file starts with DOCTYPE or xml declaration
    if [[ "$first_line" == "<!DOCTYPE"* ]] || [[ "$first_line" == "<?xml"* ]]; then
        echo "$first_line" > "$tmpfile"
        echo "<!-- $LICENSE_SPDX -->" >> "$tmpfile"
        echo "<!-- $LICENSE_COPYRIGHT -->" >> "$tmpfile"
        tail -n +2 "$file" >> "$tmpfile"
    else
        echo "<!-- $LICENSE_SPDX -->" > "$tmpfile"
        echo "<!-- $LICENSE_COPYRIGHT -->" >> "$tmpfile"
        cat "$file" >> "$tmpfile"
    fi

    mv "$tmpfile" "$file"
    echo "  ✓ $file"
    increment_modified
}

add_header_shell() {
    local file="$1"

    # Check if file already has SPDX license header
    if grep -q "SPDX-License-Identifier" "$file" 2>/dev/null; then
        increment_skipped
        return 0
    fi

    local tmpfile=$(mktemp)
    local first_line=$(head -1 "$file")

    # Check if file starts with shebang
    if [[ "$first_line" == "#!"* ]]; then
        echo "$first_line" > "$tmpfile"
        echo "# $LICENSE_SPDX" >> "$tmpfile"
        echo "# $LICENSE_COPYRIGHT" >> "$tmpfile"
        tail -n +2 "$file" >> "$tmpfile"
    else
        echo "# $LICENSE_SPDX" > "$tmpfile"
        echo "# $LICENSE_COPYRIGHT" >> "$tmpfile"
        cat "$file" >> "$tmpfile"
    fi

    mv "$tmpfile" "$file"
    echo "  ✓ $file"
    increment_modified
}

add_header_python() {
    local file="$1"

    # Check if file already has SPDX license header
    if grep -q "SPDX-License-Identifier" "$file" 2>/dev/null; then
        increment_skipped
        return 0
    fi

    local tmpfile=$(mktemp)
    local first_line=$(head -1 "$file")

    # Check if file starts with shebang or encoding declaration
    if [[ "$first_line" == "#!"* ]] || [[ "$first_line" == "# -*-"* ]] || [[ "$first_line" == "# coding"* ]]; then
        echo "$first_line" > "$tmpfile"
        echo "# $LICENSE_SPDX" >> "$tmpfile"
        echo "# $LICENSE_COPYRIGHT" >> "$tmpfile"
        echo "" >> "$tmpfile"
        tail -n +2 "$file" >> "$tmpfile"
    else
        echo "# $LICENSE_SPDX" > "$tmpfile"
        echo "# $LICENSE_COPYRIGHT" >> "$tmpfile"
        echo "" >> "$tmpfile"
        cat "$file" >> "$tmpfile"
    fi

    mv "$tmpfile" "$file"
    echo "  ✓ $file"
    increment_modified
}

add_header_markdown() {
    local file="$1"

    # Check if file already has SPDX license header
    if grep -q "SPDX-License-Identifier" "$file" 2>/dev/null; then
        increment_skipped
        return 0
    fi

    local tmpfile=$(mktemp)

    # Markdown files use HTML comment format for license headers
    echo "<!-- $LICENSE_SPDX -->" > "$tmpfile"
    echo "<!-- $LICENSE_COPYRIGHT -->" >> "$tmpfile"
    echo "" >> "$tmpfile"
    cat "$file" >> "$tmpfile"

    mv "$tmpfile" "$file"
    echo "  ✓ $file"
    increment_modified
}

echo "Adding SPDX license headers to source files..."
echo ""
echo "Format: $LICENSE_SPDX"
echo "        $LICENSE_COPYRIGHT"
echo ""

# Process Rust files
for dir in src tests; do
    if [ -d "$dir" ]; then
        echo "Processing $dir/ (Rust)..."
        while IFS= read -r -d '' file; do
            add_header_rust "$file"
        done < <(find "$dir" -name "*.rs" -type f -print0)
        echo ""
    fi
done

# Process SDK TypeScript/JavaScript files
if [ -d "sdk" ]; then
    echo "Processing sdk/ (TypeScript/JavaScript)..."
    while IFS= read -r -d '' file; do
        add_header_js "$file"
    done < <(find sdk \( -name "*.ts" -o -name "*.tsx" -o -name "*.js" -o -name "*.jsx" \) ! -path "*/node_modules/*" -type f -print0)
    echo ""
fi

# Process Frontend TypeScript/JavaScript files
if [ -d "frontend" ]; then
    echo "Processing frontend/ (TypeScript/JavaScript)..."
    while IFS= read -r -d '' file; do
        add_header_js "$file"
    done < <(find frontend \( -name "*.ts" -o -name "*.tsx" -o -name "*.js" -o -name "*.jsx" \) ! -path "*/node_modules/*" -type f -print0)
    echo ""
fi

# Process HTML templates
if [ -d "templates" ]; then
    echo "Processing templates/ (HTML)..."
    while IFS= read -r -d '' file; do
        add_header_html "$file"
    done < <(find templates -name "*.html" -type f -print0)
    echo ""
fi

# Process shell scripts in scripts/
if [ -d "scripts" ]; then
    echo "Processing scripts/ (Shell)..."
    while IFS= read -r -d '' file; do
        add_header_shell "$file"
    done < <(find scripts -name "*.sh" -type f -print0)
    echo ""

    echo "Processing scripts/ (Python)..."
    while IFS= read -r -d '' file; do
        add_header_python "$file"
    done < <(find scripts -name "*.py" -type f -print0)
    echo ""

    echo "Processing scripts/ (JavaScript)..."
    while IFS= read -r -d '' file; do
        add_header_js "$file"
    done < <(find scripts -name "*.js" -type f -print0)
    echo ""
fi

# Process examples/ directory
if [ -d "examples" ]; then
    echo "Processing examples/ (Rust)..."
    while IFS= read -r -d '' file; do
        add_header_rust "$file"
    done < <(find examples -name "*.rs" ! -path "*/target/*" -type f -print0)
    echo ""

    echo "Processing examples/ (Python)..."
    while IFS= read -r -d '' file; do
        add_header_python "$file"
    done < <(find examples -name "*.py" -type f -print0)
    echo ""

    echo "Processing examples/ (JavaScript/TypeScript)..."
    while IFS= read -r -d '' file; do
        add_header_js "$file"
    done < <(find examples \( -name "*.js" -o -name "*.ts" \) ! -path "*/node_modules/*" -type f -print0)
    echo ""
fi

# Process documentation markdown files
if [ -d "docs" ]; then
    echo "Processing docs/ (Markdown)..."
    while IFS= read -r -d '' file; do
        add_header_markdown "$file"
    done < <(find docs -name "*.md" -type f -print0)
    echo ""
fi

modified_count=$(cat "$MODIFIED_FILE")
skipped_count=$(cat "$SKIPPED_FILE")
rm -f "$MODIFIED_FILE" "$SKIPPED_FILE"

echo "Done!"
echo "  Modified: $modified_count files"
echo "  Skipped (already had header): $skipped_count files"
