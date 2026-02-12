<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Release How-To Guide

This guide explains how to release Pierre MCP Server (Rust binaries) and the SDK (npm package).

## Prerequisites

- Write access to the repository
- `NPM_TOKEN` secret configured in GitHub (already set up)
- Clean working directory (no uncommitted changes)

## Quick Release

```bash
# 1. Prepare the release (runs validation, bumps versions, creates commit + tag)
./scripts/release/prepare-release.sh 0.3.0

# 2. Push commit and tag
git push origin main
git push origin v0.3.0

# 3. Create GitHub Release from the tag (triggers npm publish)
```

## Detailed Steps

### Step 1: Prepare the Release

The `prepare-release.sh` script handles everything:

1. **Runs validation automatically** (via `validate-release.sh`)
2. Updates version in `Cargo.toml` and `sdk/package.json`
3. Updates `Cargo.lock`
4. Creates git commit and tag

Validation checks:
- Version consistency between `Cargo.toml` and `sdk/package.json`
- npm version availability (not already published)
- Git status (no uncommitted changes)
- Rust build and clippy
- SDK build, lint, and type-check
- CHANGELOG entry exists

Usage:

```bash
# Stable release
./scripts/release/prepare-release.sh 0.3.0

# Pre-release (beta)
./scripts/release/prepare-release.sh 0.3.0 --pre-release beta

# Pre-release (release candidate)
./scripts/release/prepare-release.sh 0.4.0 --pre-release rc.1

# Dry run (preview changes without modifying files)
./scripts/release/prepare-release.sh 0.3.0 --dry-run

# Update files but don't create git commit/tag
./scripts/release/prepare-release.sh 0.3.0 --no-commit

# Skip validation (not recommended, for emergencies only)
./scripts/release/prepare-release.sh 0.3.0 --skip-validation
```

### Step 2: Push to GitHub

```bash
# Push the commit
git push origin main

# Push the tag (triggers release workflow)
git push origin v0.3.0
```

### Step 3: Create GitHub Release

1. Go to https://github.com/Async-IO/pierre_mcp_server/releases
2. Click "Draft a new release"
3. Select the tag you just pushed (`v0.3.0`)
4. Add release notes (or let it auto-generate from CHANGELOG)
5. Click "Publish release"

Publishing the release triggers:
- `release.yml` - Builds Rust binaries for all platforms
- `sdk-release.yml` - Publishes SDK to npm

## What Gets Released

### Rust Binaries (`release.yml`)

Triggered by: pushing a tag `v*.*.*`

Builds for:
- Linux x86_64 (GNU and musl)
- macOS x86_64 (Intel)
- macOS aarch64 (Apple Silicon)
- Windows x86_64

Assets uploaded to GitHub Release:
- `pierre-mcp-server-v0.3.0-linux-x86_64-musl.tar.gz`
- `pierre-mcp-server-v0.3.0-macos-aarch64.tar.gz`
- etc.

### npm Package (`sdk-release.yml`)

Triggered by: publishing a GitHub Release

Publishes to: https://www.npmjs.com/package/pierre-mcp-client

npm tags:
- `latest` - stable releases (e.g., `0.3.0`)
- `beta` - beta releases (e.g., `0.3.0-beta`)
- `alpha` - alpha releases (e.g., `0.3.0-alpha`)
- `rc` - release candidates (e.g., `0.3.0-rc.1`)

## Pre-Release Workflow

For testing before a stable release:

```bash
# 1. Create beta release
./scripts/release/prepare-release.sh 0.3.0 --pre-release beta
git push origin main
git push origin v0.3.0-beta

# 2. Create GitHub Release (mark as pre-release)
# The SDK will be published with `npm install pierre-mcp-client@beta`

# 3. After testing, create stable release
./scripts/release/prepare-release.sh 0.3.0
git push origin main
git push origin v0.3.0
# Create GitHub Release (not marked as pre-release)
```

## Manual npm Publish (Emergency)

If the automated workflow fails, you can publish manually:

```bash
cd sdk
npm run build
npm run test:unit
npm publish --access public --tag latest
```

## Troubleshooting

### "Version already exists on npm"

The version has already been published. You must increment the version number.

### "SDK version does not match release"

The `sdk/package.json` version doesn't match the git tag. Use `prepare-release.sh` to ensure consistency.

### "Tag already exists"

Delete the existing tag if it was created in error:
```bash
git tag -d v0.3.0
git push origin :refs/tags/v0.3.0
```

### Workflow failed after partial publish

If the npm publish succeeded but the workflow shows failed:
1. Check https://www.npmjs.com/package/pierre-mcp-client to confirm
2. If published, the release is complete despite the workflow status
3. If not published, re-run the failed workflow from GitHub Actions

## Version Numbering

Follow [Semantic Versioning](https://semver.org/):

- **MAJOR** (1.0.0): Breaking API changes
- **MINOR** (0.1.0): New features, backward compatible
- **PATCH** (0.0.1): Bug fixes, backward compatible

Pre-release suffixes:
- `-alpha` - Early development, unstable
- `-beta` - Feature complete, testing
- `-rc.N` - Release candidate N, final testing
