<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# CI/CD Pipeline

Comprehensive documentation for the GitHub Actions continuous integration and deployment workflows.

## Overview

The project uses five specialized GitHub Actions workflows that validate different aspects of the codebase:

| Workflow | Focus | Platforms | Database Support |
|----------|-------|-----------|------------------|
| **Rust** | Core Rust quality gate | Ubuntu | SQLite |
| **Backend CI** | Comprehensive backend + frontend | Ubuntu | SQLite + PostgreSQL |
| **Cross-Platform** | OS compatibility | Linux, macOS, Windows | Mixed |
| **SDK Tests** | TypeScript SDK bridge | Ubuntu | SQLite |
| **MCP Compliance** | Protocol specification | Ubuntu | SQLite |

All workflows run on pushes to `main`, `debug/*`, `feature/*`, `claude/*` branches and on pull requests to `main`.

## Workflow Details

### Rust Workflow

**File**: `.github/workflows/rust.yml`

**Purpose**: Fast quality gate for core Rust development

**When it runs**: All pushes and PRs

**What it validates**:
1. Code formatting (`cargo fmt --check`)
2. Clippy zero-tolerance linting
3. Security audit (`cargo deny check`)
4. Architectural validation (`./scripts/ci/architectural-validation.sh`)
5. Release build (`cargo build --release`)
6. Test coverage with `cargo-llvm-cov`
7. Codecov upload

**Database**: SQLite in-memory only

**Key characteristics**:
- Single Ubuntu runner
- Full quality checks
- ~8-10 minutes runtime
- Generates coverage report

**Environment variables**:
```bash
DATABASE_URL="sqlite::memory:"
ENCRYPTION_KEY="rEFe91l6lqLahoyl9OSzum9dKa40VvV5RYj8bHGNTeo="
PIERRE_MASTER_ENCRYPTION_KEY="rEFe91l6lqLahoyl9OSzum9dKa40VvV5RYj8bHGNTeo="
STRAVA_CLIENT_ID="test_client_id_ci"
STRAVA_CLIENT_SECRET="test_client_secret_ci"
STRAVA_REDIRECT_URI="http://localhost:8080/auth/strava/callback"
```

### Backend CI Workflow

**File**: `.github/workflows/ci.yml`

**Purpose**: Comprehensive backend and frontend validation with multi-database support

**When it runs**: All pushes and PRs

**What it validates**:

**Job 1: backend-tests (SQLite)**
1. Code formatting
2. Clippy zero-tolerance
3. Security audit
4. Architectural validation
5. Secret pattern validation (`./scripts/ci/validate-no-secrets.sh`)
6. All tests with SQLite coverage
7. Codecov upload (flag: `backend-sqlite`)

**Job 2: postgres-tests (PostgreSQL)**
1. PostgreSQL 16 service container startup
2. Connection verification
3. Database plugin tests (`--features postgresql`)
4. All tests with PostgreSQL coverage (30-minute timeout)
5. Codecov upload (flag: `backend-postgresql`)

**Job 3: frontend-tests**
1. Node.js 20 setup
2. npm lint (`npm run lint`)
3. TypeScript type checking (`npx tsc --noEmit`)
4. Frontend tests with coverage (`npm run test:coverage`)
5. Frontend build (`npm run build`)
6. Codecov upload (flag: `frontend`)

**Key characteristics**:
- Three parallel jobs
- Separate coverage for each database
- Frontend validation included
- ~15-35 minutes runtime (PostgreSQL job is longest)

**PostgreSQL configuration**:
```bash
POSTGRES_USER=pierre
POSTGRES_PASSWORD=ci_test_password
POSTGRES_DB=pierre_mcp_server
POSTGRES_MAX_CONNECTIONS=3
POSTGRES_MIN_CONNECTIONS=1
POSTGRES_ACQUIRE_TIMEOUT=20
```

### Cross-Platform Tests Workflow

**File**: `.github/workflows/cross-platform.yml`

**Purpose**: Verify code works across Linux, macOS, and Windows

**When it runs**: Pushes and PRs that modify:
- `src/**`
- `tests/**`
- `Cargo.toml` or `Cargo.lock`
- `.github/workflows/cross-platform.yml`

**What it validates**:

**Matrix strategy**: Runs on 3 platforms in parallel
- ubuntu-latest (with PostgreSQL)
- macos-latest (SQLite only)
- windows-latest (SQLite only)

**Platform-specific behavior**:

**Ubuntu**:
- PostgreSQL 16 service container
- All features enabled (`--all-features`)
- Clippy with all features
- Tests with `--test-threads=1`

**macOS**:
- SQLite in-memory
- Default features only
- Clippy without `--all-features`
- Standard test execution

**Windows**:
- SQLite in-memory
- Default features only
- Release mode tests (`--release`) for speed
- Clippy without `--all-features`

**Key characteristics**:
- Path filtering (only Rust code changes)
- No coverage reporting
- No architectural validation
- No security audit
- Lightweight, fast checks
- ~10-15 minutes per platform

**What it doesn't do**:
- Coverage generation (focused on compatibility)
- Heavy validation steps (delegated to other workflows)

### SDK Tests Workflow

**File**: `.github/workflows/sdk-tests.yml`

**Purpose**: TypeScript SDK bridge validation and integration with Rust server

**When it runs**: Pushes and PRs that modify:
- `sdk/**`
- `.github/workflows/sdk-tests.yml`

**What it validates**:
1. Node.js 20 + Rust 1.91.0 setup
2. SDK dependency installation (`npm ci --prefer-offline`)
3. SDK bridge build (`npm run build`)
4. SDK unit tests (`npm run test:unit`)
5. Rust server debug build (`cargo build`)
6. SDK integration tests (`npm run test:integration`)
7. SDK E2E tests (`npm run test:e2e`)
8. Test artifact upload (7-day retention)

**Key characteristics**:
- Path filtering (only SDK changes)
- Multi-language validation (TypeScript + Rust)
- Debug Rust build (faster for integration tests)
- `--forceExit` flag for clean Jest shutdown
- ~8-12 minutes runtime

**Test levels**:
- **Unit**: SDK-only tests (no Rust dependency)
- **Integration**: SDK ↔ Rust server communication
- **E2E**: Complete workflow testing

### MCP Compliance Workflow

**File**: `.github/workflows/mcp-compliance.yml`

**Purpose**: Validate MCP protocol specification compliance

**When it runs**: All pushes and PRs

**What it validates**:
1. Python 3.11 + Node.js 20 + Rust 1.91.0 setup
2. MCP Validator installation (cloned from `Janix-ai/mcp-validator`)
3. SDK dependency installation
4. SDK bridge build
5. SDK TypeScript types validation:
   - Checks `src/types.ts` exists
   - Rejects placeholder content
   - Requires pre-generated types in repository
6. MCP compliance validation (`./scripts/ensure_mcp_compliance.sh`)
7. Artifact cleanup

**Key characteristics**:
- Multi-language stack (Python + Node.js + Rust)
- External validation tool
- Strict type generation requirements
- Disk space management (aggressive cleanup)
- CI-specific flags (`CI=true`, `GITHUB_ACTIONS=true`)
- Security flags (`PIERRE_ALLOW_INTERACTIVE_OAUTH=false`)
- ~10-15 minutes runtime

**Environment variables**:
```bash
CI="true"
GITHUB_ACTIONS="true"
HTTP_PORT=8080
DATABASE_URL="sqlite::memory:"
PIERRE_MASTER_ENCRYPTION_KEY="rEFe91l6lqLahoyl9OSzum9dKa40VvV5RYj8bHGNTeo="
PIERRE_ALLOW_INTERACTIVE_OAUTH="false"
PIERRE_RSA_KEY_SIZE="2048"
```

## Workflow Triggers

### Push Triggers

All workflows run on these branches:
- `main`
- `debug/*`
- `feature/*`
- `claude/*`

### Pull Request Triggers

All workflows run on PRs to:
- `main`

### Path Filtering

Some workflows only run when specific files change:

**Cross-Platform Tests**:
- `src/**`
- `tests/**`
- `Cargo.toml`, `Cargo.lock`
- `.github/workflows/cross-platform.yml`

**SDK Tests**:
- `sdk/**`
- `.github/workflows/sdk-tests.yml`

**Optimization rationale**: Path filtering reduces CI resource usage by skipping irrelevant workflow runs. For example, changing only SDK code doesn't require cross-platform Rust validation.

## Understanding CI/CD Results

### Status Indicators

- ✅ **Green check**: All validations passed
- ⚠️ **Yellow circle**: Workflow in progress
- ❌ **Red X**: One or more checks failed

### Common Failure Patterns

#### Formatting Failure
```
error: left behind trailing whitespace
```
**Fix**: Run `cargo fmt` locally before committing

#### Clippy Failure
```
error: using `unwrap()` on a `Result` value
```
**Fix**: Use proper error handling with `?` operator or `ok_or_else()`

#### Test Failure
```
test result: FAILED. 1245 passed; 7 failed
```
**Fix**: Run `cargo test` locally to reproduce, fix failing tests

#### Security Audit Failure
```
error: 1 security advisory found
```
**Fix**: Run `cargo deny check` locally, update dependencies or add justified ignore

#### Architectural Validation Failure
```
ERROR: Found unwrap() usage in production code
```
**Fix**: Run `./scripts/ci/architectural-validation.sh` locally, fix violations

#### PostgreSQL Connection Failure
```
ERROR: PostgreSQL connection timeout
```
**Cause**: PostgreSQL service container not ready
**Fix**: Usually transient, re-run workflow

#### SDK Type Validation Failure
```
ERROR: src/types.ts contains placeholder content
```
**Fix**: Run `npm run generate-types` locally with running server, commit generated types

### Viewing Detailed Logs

1. Navigate to Actions tab in GitHub
2. Click on the workflow run
3. Click on the failing job
4. Expand the failing step
5. Review error output

## Local Validation Before Push

Run the same checks locally to catch issues before CI:

```bash
# 1. Format code
cargo fmt

# 2. Architectural validation
./scripts/ci/architectural-validation.sh

# 3. Zero-tolerance clippy
cargo clippy --tests -- \
  -W clippy::all \
  -W clippy::pedantic \
  -W clippy::nursery \
  -D warnings

# 4. Run all tests
cargo test

# 5. Security audit
cargo deny check

# 6. SDK tests (if SDK changed)
cd sdk
npm run test:unit
npm run test:integration
npm run test:e2e
cd ..

# 7. Frontend tests (if frontend changed)
cd frontend
npm run lint
npm run test:coverage
npm run build
cd ..
```

**Shortcut**: Use validation script
```bash
./scripts/lint-and-test.sh
```

## Debugging CI/CD Failures

### Reproducing Locally

Match CI environment exactly:

```bash
# Set CI environment variables
export DATABASE_URL="sqlite::memory:"
export ENCRYPTION_KEY="rEFe91l6lqLahoyl9OSzum9dKa40VvV5RYj8bHGNTeo="
export PIERRE_MASTER_ENCRYPTION_KEY="rEFe91l6lqLahoyl9OSzum9dKa40VvV5RYj8bHGNTeo="
export STRAVA_CLIENT_ID="test_client_id_ci"
export STRAVA_CLIENT_SECRET="test_client_secret_ci"
export STRAVA_REDIRECT_URI="http://localhost:8080/auth/strava/callback"

# Run tests matching CI configuration
cargo test --test-threads=1
```

### Platform-Specific Issues

**macOS vs Linux differences**:
- File system case sensitivity
- Line ending handling (CRLF vs LF)
- Path separator differences

**Windows-specific issues**:
- Longer compilation times (run release mode tests)
- Path length limitations
- File locking behavior

### PostgreSQL-Specific Debugging

Start local PostgreSQL matching CI:

```bash
docker run -d \
  --name postgres-ci \
  -e POSTGRES_USER=pierre \
  -e POSTGRES_PASSWORD=ci_test_password \
  -e POSTGRES_DB=pierre_mcp_server \
  -p 5432:5432 \
  postgres:16-alpine

# Wait for startup
sleep 5

# Run PostgreSQL tests
export DATABASE_URL="postgresql://pierre:ci_test_password@localhost:5432/pierre_mcp_server"
cargo test --features postgresql

# Cleanup
docker stop postgres-ci
docker rm postgres-ci
```

### SDK Integration Debugging

Run SDK tests with debug output:

```bash
cd sdk

# Build Rust server in debug mode
cd ..
cargo build
cd sdk

# Run tests with verbose output
npm run test:integration -- --verbose
npm run test:e2e -- --verbose
```

## Coverage Reporting

### Codecov Integration

Coverage reports are uploaded to Codecov with specific flags:

- `backend-sqlite`: SQLite test coverage
- `backend-postgresql`: PostgreSQL test coverage
- `frontend`: Frontend test coverage

### Viewing Coverage

1. Navigate to Codecov dashboard
2. Filter by flag to see database-specific coverage
3. Review coverage trends over time
4. Identify untested code paths

### Coverage Thresholds

No enforced thresholds (yet), but aim for:
- Core business logic: >80%
- Database plugins: >75%
- Protocol handlers: >70%

## Workflow Maintenance

### Updating Rust Version

When updating Rust toolchain:

1. Update `rust-toolchain` file
2. Update `.github/workflows/*.yml` (search for `dtolnay/rust-toolchain@`)
3. Test locally with new version
4. Commit and verify all workflows pass

### Updating Dependencies

When updating crate dependencies:

1. Run `cargo update`
2. Test locally
3. Check `cargo deny check` for new advisories
4. Update `deny.toml` if needed (with justification)
5. Commit and verify CI passes

### Adding New Workflow

When adding new validation:

1. Create workflow file in `.github/workflows/`
2. Test workflow on feature branch
3. Document in this file
4. Update summary table
5. Add to `contributing.md` review process

## Cost Optimization

### Cache Strategy

Workflows use `actions/cache@v4` for:
- Rust dependencies (`~/.cargo/`)
- Compiled artifacts (`target/`)
- Node.js dependencies (`node_modules/`)

**Cache keys** include:
- OS (`${{ runner.os }}`)
- Rust version
- `Cargo.lock` hash

### Disk Space Management

Ubuntu runners have limited disk space (~14GB usable).

**Free disk space steps**:
- Remove unused Android SDK
- Remove unused .NET frameworks
- Remove unused Docker images
- Clean Cargo cache

**Workflows using cleanup**:
- Rust workflow
- Backend CI workflow
- Cross-Platform Tests workflow
- MCP Compliance workflow

### Parallel Execution

Jobs run in parallel when independent:
- Backend CI: 3 jobs in parallel (SQLite, PostgreSQL, frontend)
- Cross-Platform: 3 jobs in parallel (Linux, macOS, Windows)

**Total CI time**: ~30-35 minutes (longest job determines duration)

## Troubleshooting Reference

### "failed to get `X` as a dependency"

**Cause**: Network timeout fetching crate
**Fix**: Re-run workflow (transient issue)

### "disk quota exceeded"

**Cause**: Insufficient disk space on runner
**Fix**: Workflow already includes cleanup; may need to reduce artifact size

### "database connection pool exhausted"

**Cause**: Tests creating too many connections
**Fix**: Tests use `--test-threads=1` to serialize execution

### "clippy warnings found"

**Cause**: New clippy version detected additional issues
**Fix**: Run `cargo clippy --fix` locally, review and commit

### "mcp validator not found"

**Cause**: Failed to clone mcp-validator repository
**Fix**: Re-run workflow (transient network issue)

### "sdk types contain placeholder"

**Cause**: Generated types not committed to repository
**Fix**: Run `npm run generate-types` locally with server running, commit result

## Best Practices

### Before Creating PR

1. Run `./scripts/lint-and-test.sh` locally
2. Verify all tests pass
3. Check clippy with zero warnings
4. Review architectural validation
5. If SDK changed, run SDK tests
6. If frontend changed, run frontend tests

### Reviewing PR CI Results

1. Wait for all workflows to complete
2. Review any failures immediately
3. Don't merge with failing workflows
4. Check coverage hasn't decreased significantly
5. Review security audit warnings

### Maintaining CI/CD Health

1. Monitor workflow run times (alert if >50% increase)
2. Review dependency updates monthly
3. Update Rust version quarterly
4. Keep workflows DRY (extract common steps to scripts)
5. Document any workflow changes in this file

## Future Improvements

Planned enhancements:

- Enforce coverage thresholds
- Add benchmark regression testing
- Add performance profiling workflow
- Add automated dependency updates (Dependabot)
- Add deployment workflow for releases
- Add E2E testing with real Strava API (secure credentials)

## Additional Resources

- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [Codecov Documentation](https://docs.codecov.com/)
- [cargo-deny Configuration](https://embarkstudios.github.io/cargo-deny/)
- [cargo-llvm-cov Usage](https://github.com/taiki-e/cargo-llvm-cov)
