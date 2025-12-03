#!/bin/bash
# ABOUTME: Prepares docs for Starlight by copying from root docs/ and adding frontmatter
# ABOUTME: Extracts title from first H1 heading and adds YAML frontmatter

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WEBSITE_DIR="$(dirname "$SCRIPT_DIR")"
PROJECT_ROOT="$(dirname "$WEBSITE_DIR")"
DOCS_SRC="$PROJECT_ROOT/docs"
DOCS_DEST="$WEBSITE_DIR/src/content/docs"

echo "Preparing docs for Starlight..."
echo "Source: $DOCS_SRC"
echo "Destination: $DOCS_DEST"

# Clean docs destination (but keep config.ts)
find "$DOCS_DEST" -name "*.md" -type f -delete 2>/dev/null || true
rm -rf "$DOCS_DEST/installation-guides" "$DOCS_DEST/architecture" 2>/dev/null || true

# Function to extract title from markdown file
extract_title() {
    local file="$1"
    # Look for first H1 heading (# Title)
    local title=$(grep -m 1 "^# " "$file" | sed 's/^# //')
    if [ -z "$title" ]; then
        # Fallback to filename
        title=$(basename "$file" .md | sed 's/-/ /g' | sed 's/\b\(.\)/\u\1/g')
    fi
    echo "$title"
}

# Function to process a markdown file
process_file() {
    local src_file="$1"
    local dest_file="$2"
    local title=$(extract_title "$src_file")

    # Skip README files (use parent directory name instead)
    local basename=$(basename "$src_file")
    if [ "$basename" = "README.md" ]; then
        # Rename README.md to index.md for Starlight
        dest_file="${dest_file%README.md}index.md"
    fi

    # Create destination directory if needed
    mkdir -p "$(dirname "$dest_file")"

    # Check if file already has frontmatter
    if head -1 "$src_file" | grep -q "^---$"; then
        # Already has frontmatter, just copy
        cp "$src_file" "$dest_file"
    else
        # Add frontmatter
        {
            echo "---"
            echo "title: \"$title\""
            echo "---"
            echo ""
            # Skip license comments at the top
            sed '/^<!-- SPDX/d; /^<!-- Copyright/d' "$src_file"
        } > "$dest_file"
    fi

    echo "  Processed: $(basename "$src_file") -> $(basename "$dest_file")"
}

# Process all markdown files
find "$DOCS_SRC" -name "*.md" -type f | while read -r src_file; do
    # Get relative path from docs source
    rel_path="${src_file#$DOCS_SRC/}"
    dest_file="$DOCS_DEST/$rel_path"

    # Skip tutorial directory (too many files, keep on GitHub only)
    if [[ "$rel_path" == tutorial/* ]]; then
        continue
    fi

    process_file "$src_file" "$dest_file"
done

echo ""
echo "Docs preparation complete!"
echo "Total files: $(find "$DOCS_DEST" -name "*.md" | wc -l | tr -d ' ')"
