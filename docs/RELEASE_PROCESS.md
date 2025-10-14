# Release Process

This document describes the process for creating and publishing releases of Pierre MCP Server.

## Overview

Pierre uses GitHub Actions to automatically build and publish multi-platform binaries when a version tag is pushed. The release workflow builds binaries for:

- **Linux**: x86_64 (GNU and musl variants)
- **macOS**: x86_64 (Intel) and aarch64 (Apple Silicon)
- **Windows**: x86_64 (MSVC)

## Prerequisites

Before creating a release, ensure:

1. ✅ All tests pass: `./scripts/lint-and-test.sh`
2. ✅ CHANGELOG.md is updated with release notes
3. ✅ Version in `Cargo.toml` matches the release version
4. ✅ Documentation is up to date
5. ✅ All commits are pushed to `main` branch

## Release Steps

### 1. Update Version Information

#### Update Cargo.toml

```bash
# Edit Cargo.toml version field
vim Cargo.toml
```

Change version at line 3:
```toml
version = "0.1.0"  # Update to your target version
```

#### Update CHANGELOG.md

Add a new section for the release at the top of CHANGELOG.md:

```markdown
## [0.1.0] - 2025-10-14

### Added
- Feature 1
- Feature 2

### Changed
- Change 1

### Fixed
- Bug fix 1
```

Update the version links at the bottom:
```markdown
[0.1.0]: https://github.com/Async-IO/pierre_mcp_server/releases/tag/v0.1.0
```

### 2. Commit Version Bump

```bash
git add Cargo.toml CHANGELOG.md
git commit -m "chore: bump version to 0.1.0"
git push origin main
```

### 3. Create and Push Release Tag

Create an annotated tag:

```bash
git tag -a v0.1.0 -m "Release version 0.1.0"
git push origin v0.1.0
```

**Important**: The tag must follow the format `v*.*.*` (e.g., `v0.1.0`, `v1.2.3`) to trigger the release workflow.

### 4. Monitor Release Build

After pushing the tag, GitHub Actions will automatically:

1. Build binaries for all platforms (takes ~15-20 minutes)
2. Create compressed archives with checksums
3. Extract release notes from CHANGELOG.md
4. Create a GitHub Release with all binaries attached

Monitor the build progress:

```
https://github.com/Async-IO/pierre_mcp_server/actions/workflows/release.yml
```

### 5. Verify Release

Once the workflow completes:

1. **Check the release page**:
   ```
   https://github.com/Async-IO/pierre_mcp_server/releases/tag/v0.1.0
   ```

2. **Verify all binaries are present**:
   - `pierre-mcp-server-v0.1.0-linux-x86_64-gnu.tar.gz`
   - `pierre-mcp-server-v0.1.0-linux-x86_64-musl.tar.gz` ⭐ (recommended for Linux)
   - `pierre-mcp-server-v0.1.0-macos-x86_64.tar.gz`
   - `pierre-mcp-server-v0.1.0-macos-aarch64.tar.gz`
   - `pierre-mcp-server-v0.1.0-windows-x86_64.zip`
   - `SHA256SUMS.txt`

3. **Download and test** at least one binary:
   ```bash
   # Linux example
   wget https://github.com/Async-IO/pierre_mcp_server/releases/download/v0.1.0/pierre-mcp-server-v0.1.0-linux-x86_64-musl.tar.gz
   tar -xzf pierre-mcp-server-v0.1.0-linux-x86_64-musl.tar.gz
   cd pierre-mcp-server-v0.1.0
   ./bin/pierre-mcp-server --version
   ```

4. **Verify checksums**:
   ```bash
   sha256sum -c SHA256SUMS.txt
   ```

### 6. Announce Release

After verifying the release:

1. **Update README.md** (if needed) with download links
2. **Post to GitHub Discussions** (if applicable)
3. **Update project website** (if applicable)
4. **Announce on social media** (if applicable)

## Release Workflow Details

### Workflow Triggers

The release workflow (`.github/workflows/release.yml`) triggers on:

1. **Tag push**: Automatically when you push a tag matching `v*.*.*`
2. **Manual dispatch**: Via GitHub UI (Actions → Release Binaries → Run workflow)

### Build Matrix

The workflow uses a matrix strategy to build in parallel:

```yaml
matrix:
  include:
    - os: ubuntu-latest, target: x86_64-unknown-linux-gnu
    - os: ubuntu-latest, target: x86_64-unknown-linux-musl
    - os: macos-latest, target: x86_64-apple-darwin
    - os: macos-latest, target: aarch64-apple-darwin
    - os: windows-latest, target: x86_64-pc-windows-msvc
```

### Binary Contents

Each release archive contains:

```
pierre-mcp-server-v0.1.0/
├── bin/
│   ├── pierre-mcp-server     # Main server binary
│   ├── admin-setup            # Admin CLI tool
│   ├── diagnose-weather-api   # Diagnostic tool
│   └── serve-docs             # Documentation server
├── README.md                  # Project documentation
├── LICENSE-MIT               # MIT license
├── LICENSE-APACHE            # Apache 2.0 license
└── INSTALL.txt               # Installation instructions
```

### Optimization Settings

Binaries are built with release optimizations from `Cargo.toml`:

```toml
[profile.release]
lto = "thin"          # Link-time optimization
codegen-units = 1     # Better optimization
panic = "abort"       # Smaller binary
strip = true          # Remove debug symbols
```

## Troubleshooting

### Build Failure

If the build fails:

1. Check the GitHub Actions logs for the specific error
2. Fix the issue in the codebase
3. Delete the failed release (if created)
4. Delete the tag locally and remotely:
   ```bash
   git tag -d v0.1.0
   git push origin :refs/tags/v0.1.0
   ```
5. Fix the issue, commit, and retry from step 3

### Missing Binaries

If some binaries are missing from the release:

1. Check which build job failed in Actions logs
2. The issue is likely platform-specific (musl tools, cross-compilation, etc.)
3. Fix the issue and re-release

### Release Already Exists

If you need to update an existing release:

1. **Option A**: Delete the release and tag, then recreate
   ```bash
   # Via GitHub UI: Delete the release
   git tag -d v0.1.0
   git push origin :refs/tags/v0.1.0
   # Then recreate as per normal process
   ```

2. **Option B**: Manually upload missing binaries via GitHub UI
   - Go to the release page
   - Click "Edit release"
   - Drag and drop binaries into the assets section

## Manual Release (Emergency)

If the automated workflow fails and you need to create a release manually:

### Build Binaries Locally

**Linux (on Ubuntu/Debian)**:
```bash
# Install musl tools
sudo apt-get install musl-tools

# Add targets
rustup target add x86_64-unknown-linux-gnu x86_64-unknown-linux-musl

# Build
cargo build --release --target x86_64-unknown-linux-gnu
cargo build --release --target x86_64-unknown-linux-musl

# Strip
strip target/x86_64-unknown-linux-gnu/release/pierre-mcp-server
strip target/x86_64-unknown-linux-musl/release/pierre-mcp-server
```

**macOS**:
```bash
# Add targets
rustup target add x86_64-apple-darwin aarch64-apple-darwin

# Build
cargo build --release --target x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin

# Strip
strip target/x86_64-apple-darwin/release/pierre-mcp-server
strip target/aarch64-apple-darwin/release/pierre-mcp-server
```

**Windows**:
```powershell
# Add target
rustup target add x86_64-pc-windows-msvc

# Build
cargo build --release --target x86_64-pc-windows-msvc
```

### Package and Upload

```bash
# Create directory structure
mkdir -p pierre-mcp-server-v0.1.0/bin
cp target/release/pierre-mcp-server pierre-mcp-server-v0.1.0/bin/
cp README.md LICENSE-* pierre-mcp-server-v0.1.0/

# Create archive
tar -czf pierre-mcp-server-v0.1.0-linux-x86_64-musl.tar.gz pierre-mcp-server-v0.1.0

# Generate checksum
sha256sum pierre-mcp-server-*.tar.gz > SHA256SUMS.txt

# Upload via GitHub UI or gh CLI
gh release create v0.1.0 pierre-mcp-server-*.tar.gz SHA256SUMS.txt \
  --title "Pierre MCP Server v0.1.0" \
  --notes-file CHANGELOG.md
```

## Pre-Release Checklist

Before creating any release, verify:

- [ ] All CI checks pass on main branch
- [ ] Version updated in `Cargo.toml`
- [ ] CHANGELOG.md updated with release notes
- [ ] Documentation updated (if needed)
- [ ] No uncommitted changes
- [ ] No pending PRs that should be included
- [ ] Security audit clean: `cargo audit`
- [ ] Clippy warnings resolved: `cargo clippy -- -D warnings`
- [ ] Tests pass: `cargo test --all-features`
- [ ] Binary size acceptable: `cargo build --release && ls -lh target/release/pierre-mcp-server`

## Post-Release Tasks

After a successful release:

1. [ ] Update main branch if needed
2. [ ] Close GitHub milestone (if used)
3. [ ] Update project roadmap
4. [ ] Document any breaking changes
5. [ ] Notify users of the release
6. [ ] Update installation documentation
7. [ ] Monitor issue tracker for bug reports

## Versioning Policy

Pierre follows [Semantic Versioning](https://semver.org/):

- **MAJOR** version (1.0.0): Incompatible API changes
- **MINOR** version (0.1.0): New functionality, backward compatible
- **PATCH** version (0.0.1): Bug fixes, backward compatible

### Pre-1.0.0 Versioning

For versions before 1.0.0:
- Breaking changes may occur in MINOR versions (0.x.0)
- PATCH versions (0.0.x) should be backward compatible

## Release Schedule

- **Patch releases**: As needed for critical bug fixes
- **Minor releases**: Monthly or when significant features are ready
- **Major releases**: When introducing breaking changes (v1.0.0+)

## Emergency Hotfix Process

For critical security or bug fixes:

1. Create a hotfix branch from the release tag
2. Apply the minimal fix
3. Update CHANGELOG.md with patch notes
4. Bump PATCH version in Cargo.toml
5. Tag and release as normal
6. Merge hotfix back to main

```bash
git checkout -b hotfix/v0.1.1 v0.1.0
# Make fixes
git commit -m "fix: critical security issue"
git tag -a v0.1.1 -m "Hotfix release 0.1.1"
git push origin hotfix/v0.1.1 v0.1.1
git checkout main
git merge hotfix/v0.1.1
```

---

**Questions or Issues?**

If you encounter problems with the release process, please:
1. Check the [GitHub Actions workflow logs](https://github.com/Async-IO/pierre_mcp_server/actions)
2. Review this documentation for troubleshooting steps
3. Open an issue with the `release` label
