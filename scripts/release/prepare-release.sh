#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025 Pierre Fitness Intelligence
# ABOUTME: Prepares a release by bumping versions in Cargo.toml and sdk/package.json.
# ABOUTME: Creates a git commit and tag for the release.

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_usage() {
    echo "Usage: $0 <version> [--dry-run] [--no-commit] [--pre-release <tag>] [--skip-validation]"
    echo ""
    echo "Arguments:"
    echo "  version           Version number (e.g., 0.3.0 or v0.3.0)"
    echo ""
    echo "Options:"
    echo "  --dry-run         Show what would be done without making changes"
    echo "  --no-commit       Update files but don't create git commit/tag"
    echo "  --pre-release     Add pre-release tag (e.g., beta, alpha, rc.1)"
    echo "  --skip-validation Skip running validate-release.sh"
    echo ""
    echo "Examples:"
    echo "  $0 0.3.0                    # Release v0.3.0"
    echo "  $0 0.3.0 --pre-release beta # Release v0.3.0-beta"
    echo "  $0 0.4.0 --dry-run          # Preview changes for v0.4.0"
}

# Parse arguments
VERSION=""
DRY_RUN=false
NO_COMMIT=false
PRE_RELEASE=""
SKIP_VALIDATION=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --no-commit)
            NO_COMMIT=true
            shift
            ;;
        --pre-release)
            PRE_RELEASE="$2"
            shift 2
            ;;
        --skip-validation)
            SKIP_VALIDATION=true
            shift
            ;;
        --help|-h)
            print_usage
            exit 0
            ;;
        -*)
            echo -e "${RED}Error: Unknown option $1${NC}"
            print_usage
            exit 1
            ;;
        *)
            if [[ -z "$VERSION" ]]; then
                VERSION="$1"
            else
                echo -e "${RED}Error: Unexpected argument $1${NC}"
                print_usage
                exit 1
            fi
            shift
            ;;
    esac
done

if [[ -z "$VERSION" ]]; then
    echo -e "${RED}Error: Version is required${NC}"
    print_usage
    exit 1
fi

# Strip leading 'v' if present
VERSION="${VERSION#v}"

# Validate version format (semver)
if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo -e "${RED}Error: Invalid version format '$VERSION'. Expected semver (e.g., 0.3.0)${NC}"
    exit 1
fi

# Build full version string
FULL_VERSION="$VERSION"
if [[ -n "$PRE_RELEASE" ]]; then
    FULL_VERSION="${VERSION}-${PRE_RELEASE}"
fi

TAG_NAME="v${FULL_VERSION}"

echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}  Pierre Release Preparation${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "  Version:     ${GREEN}${FULL_VERSION}${NC}"
echo -e "  Git Tag:     ${GREEN}${TAG_NAME}${NC}"
echo -e "  Dry Run:     ${DRY_RUN}"
echo -e "  No Commit:   ${NO_COMMIT}"
echo ""

# Change to repo root
cd "$(dirname "$0")/.."
REPO_ROOT=$(pwd)

# Check we're in a git repo
if ! git rev-parse --git-dir > /dev/null 2>&1; then
    echo -e "${RED}Error: Not in a git repository${NC}"
    exit 1
fi

# Run validation first (unless skipped or dry-run)
if [[ "$SKIP_VALIDATION" == false && "$DRY_RUN" == false ]]; then
    echo -e "${BLUE}Running pre-release validation...${NC}"
    echo ""
    if ! ./scripts/release/validate-release.sh; then
        echo ""
        echo -e "${RED}Validation failed. Fix the issues above before releasing.${NC}"
        echo -e "${YELLOW}Tip: Use --skip-validation to bypass (not recommended)${NC}"
        exit 1
    fi
    echo ""
elif [[ "$DRY_RUN" == true ]]; then
    echo -e "${BLUE}[Dry run] Would run: ./scripts/release/validate-release.sh${NC}"
    echo ""
fi

# Check for uncommitted changes
if ! git diff-index --quiet HEAD -- 2>/dev/null; then
    echo -e "${YELLOW}Warning: You have uncommitted changes${NC}"
    if [[ "$DRY_RUN" == false && "$NO_COMMIT" == false ]]; then
        read -p "Continue anyway? (y/N) " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            exit 1
        fi
    fi
fi

# Check if tag already exists
if git rev-parse "$TAG_NAME" > /dev/null 2>&1; then
    echo -e "${RED}Error: Tag $TAG_NAME already exists${NC}"
    exit 1
fi

# Get current versions
CURRENT_CARGO_VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
CURRENT_SDK_VERSION=$(grep '"version"' sdk/package.json | head -1 | sed 's/.*"version": "\(.*\)".*/\1/')

echo -e "${BLUE}Current Versions:${NC}"
echo -e "  Cargo.toml:       ${CURRENT_CARGO_VERSION}"
echo -e "  sdk/package.json: ${CURRENT_SDK_VERSION}"
echo ""

if [[ "$CURRENT_CARGO_VERSION" == "$FULL_VERSION" && "$CURRENT_SDK_VERSION" == "$FULL_VERSION" ]]; then
    echo -e "${YELLOW}Versions already set to ${FULL_VERSION}${NC}"
fi

# Update Cargo.toml
update_cargo_toml() {
    echo -e "${BLUE}Updating Cargo.toml...${NC}"
    if [[ "$DRY_RUN" == true ]]; then
        echo "  Would change: version = \"$CURRENT_CARGO_VERSION\" → version = \"$FULL_VERSION\""
    else
        # Use sed to update the version (first occurrence only)
        if [[ "$OSTYPE" == "darwin"* ]]; then
            sed -i '' "0,/^version = \".*\"/s//version = \"$FULL_VERSION\"/" Cargo.toml
        else
            sed -i "0,/^version = \".*\"/s//version = \"$FULL_VERSION\"/" Cargo.toml
        fi
        echo -e "  ${GREEN}✓${NC} Updated to version = \"$FULL_VERSION\""
    fi
}

# Update sdk/package.json
update_sdk_package() {
    echo -e "${BLUE}Updating sdk/package.json...${NC}"
    if [[ "$DRY_RUN" == true ]]; then
        echo "  Would change: \"version\": \"$CURRENT_SDK_VERSION\" → \"version\": \"$FULL_VERSION\""
    else
        # Use sed to update the version
        if [[ "$OSTYPE" == "darwin"* ]]; then
            sed -i '' "s/\"version\": \".*\"/\"version\": \"$FULL_VERSION\"/" sdk/package.json
        else
            sed -i "s/\"version\": \".*\"/\"version\": \"$FULL_VERSION\"/" sdk/package.json
        fi
        echo -e "  ${GREEN}✓${NC} Updated to \"version\": \"$FULL_VERSION\""
    fi
}

# Update Cargo.lock
update_cargo_lock() {
    echo -e "${BLUE}Updating Cargo.lock...${NC}"
    if [[ "$DRY_RUN" == true ]]; then
        echo "  Would run: cargo update --workspace"
    else
        cargo update --workspace --quiet
        echo -e "  ${GREEN}✓${NC} Cargo.lock updated"
    fi
}

# Run validation
run_validation() {
    echo -e "${BLUE}Running validation...${NC}"
    if [[ "$DRY_RUN" == true ]]; then
        echo "  Would run: cargo check --quiet"
        echo "  Would run: cd sdk && npm run type-check"
    else
        echo "  Checking Rust compilation..."
        cargo check --quiet
        echo -e "  ${GREEN}✓${NC} Rust compilation check passed"

        echo "  Checking TypeScript compilation..."
        (cd sdk && npm run type-check --silent)
        echo -e "  ${GREEN}✓${NC} TypeScript type check passed"
    fi
}

# Create git commit and tag
create_git_commit() {
    if [[ "$NO_COMMIT" == true ]]; then
        echo -e "${YELLOW}Skipping git commit/tag (--no-commit)${NC}"
        return
    fi

    echo -e "${BLUE}Creating git commit and tag...${NC}"
    if [[ "$DRY_RUN" == true ]]; then
        echo "  Would run: git add Cargo.toml Cargo.lock sdk/package.json"
        echo "  Would run: git commit -m \"chore: release ${TAG_NAME}\""
        echo "  Would run: git tag -a ${TAG_NAME} -m \"Release ${TAG_NAME}\""
    else
        git add Cargo.toml Cargo.lock sdk/package.json
        git commit -m "chore: release ${TAG_NAME}"
        echo -e "  ${GREEN}✓${NC} Created commit"

        git tag -a "${TAG_NAME}" -m "Release ${TAG_NAME}"
        echo -e "  ${GREEN}✓${NC} Created tag ${TAG_NAME}"
    fi
}

# Execute updates
update_cargo_toml
update_sdk_package
update_cargo_lock
run_validation
create_git_commit

echo ""
echo -e "${GREEN}═══════════════════════════════════════════════════════════${NC}"
if [[ "$DRY_RUN" == true ]]; then
    echo -e "${GREEN}  Dry run complete! No changes were made.${NC}"
else
    echo -e "${GREEN}  Release ${TAG_NAME} prepared successfully!${NC}"
fi
echo -e "${GREEN}═══════════════════════════════════════════════════════════${NC}"
echo ""

if [[ "$DRY_RUN" == false && "$NO_COMMIT" == false ]]; then
    echo -e "${BLUE}Next steps:${NC}"
    echo "  1. Review the changes: git show HEAD"
    echo "  2. Push the commit and tag:"
    echo "     git push origin main"
    echo "     git push origin ${TAG_NAME}"
    echo ""
    echo "  This will trigger:"
    echo "    - release.yml    → Build and release Rust binaries"
    echo "    - sdk-release.yml → Publish SDK to npm (after release is published)"
fi
