# Pierre MCP Server - Reference Part 4: Testing & CI

> Reference documentation for ChatGPT. Part 4: Testing, CI/CD, Contributing.

---

# Testing Guide

Pierre Fitness Platform includes comprehensive test coverage using synthetic data for intelligence tools.

## Running Tests

```bash
# Run all tests
cargo test

# Run specific test suites
cargo test --test mcp_protocol_comprehensive_test
cargo test --test mcp_multitenant_complete_test
cargo test --test intelligence_tools_basic_test
cargo test --test intelligence_tools_advanced_test

# Run with output
cargo test -- --nocapture

# Lint and test
./scripts/lint-and-test.sh
```

## Multi-Tenant Tests

Tests validating MCP protocol with multi-tenant isolation across HTTP and SDK transports:

```bash
# Rust multi-tenant MCP tests (4 test scenarios)
cargo test --test mcp_multitenant_sdk_e2e_test

# Type generation multi-tenant validation (3 test scenarios)
cargo test --test mcp_type_generation_multitenant_test

# SDK multi-tenant tests (11 test cases)
cd sdk
npm run test -- --testPathPattern=e2e-multitenant
cd ..
```

**Test Coverage**:
- Concurrent multi-tenant tool calls without data leakage
- HTTP and SDK transport parity
- Tenant isolation at protocol level (403/404 errors for unauthorized access)
- Type generation consistency across tenants
- Rate limiting per tenant
- SDK concurrent access by multiple tenants
- SDK tenant isolation verification
- Schema consistency across tiers

**Test Infrastructure** (`tests/common.rs` and `sdk/test/helpers/`):
- `spawn_sdk_bridge()`: Spawns SDK process with JWT token and automatic cleanup
- `send_http_mcp_request()`: Direct HTTP MCP requests for transport testing
- `create_test_tenant()`: Creates tenant with user and JWT token
- `multitenant-setup.js`: Multi-tenant client setup and isolation verification
- `rust-server-bridge.js`: Coordination between SDK tests and Rust server

## Intelligence Testing Framework

The platform includes 30+ integration tests covering all 8 intelligence tools without OAuth dependencies:

**Test Categories**:
- **Basic Tools**: `get_athlete`, `get_activities`, `get_stats`, `compare_activities`
- **Advanced Analytics**: `calculate_fitness_score`, `predict_performance`, `analyze_training_load`
- **Goal Management**: `suggest_goals`, `analyze_goal_feasibility`, `track_progress`

**Synthetic Data Scenarios**:
- Beginner runner improving over time
- Experienced cyclist with consistent training
- Multi-sport athlete (triathlete pattern)
- Training gaps and recovery periods

See `tests/intelligence_tools_basic_test.rs` and `tests/intelligence_tools_advanced_test.rs` for details.

## RSA Key Size Configuration

Pierre Fitness Platform uses RS256 asymmetric signing for JWT tokens. Key size affects both security and performance:

**Production (4096-bit keys - default)**:
- Higher security with larger key size
- Slower key generation (~10 seconds)
- Use in production environments

**Testing (2048-bit keys)**:
- Faster key generation (~250ms)
- Suitable for development and testing
- Set via environment variable:

```bash
export PIERRE_RSA_KEY_SIZE=2048
```

## Test Performance Optimization

Pierre Fitness Platform includes a shared test JWKS manager to eliminate RSA key generation overhead:

**Shared Test JWKS Pattern** (implemented in `tests/common.rs:40-52`):
```rust
use pierre_mcp_server_integrations::common;

// Reuses shared JWKS manager across all tests (10x faster)
let jwks_manager = common::get_shared_test_jwks();
```

**Performance Impact**:
- **Without optimization**: 100ms+ RSA key generation per test
- **With shared JWKS**: One-time generation, instant reuse across test suite
- **Result**: 10x faster test execution

**E2E Tests**: The SDK test suite (`sdk/test/`) automatically uses 2048-bit keys via `PIERRE_RSA_KEY_SIZE=2048` in server startup configuration (`sdk/test/helpers/server.js:82`).

---

# Testing Strategy

## Overview

This document outlines the multi-tier testing strategy for Pierre MCP Server. The strategy is designed to provide fast feedback during development while maintaining comprehensive test coverage in CI.

## Test Suite Statistics

- **Total test files:** 166
- **Total test code:** ~62,000 lines
- **E2E tests:** 11 files
- **Comprehensive tests:** 9 files
- **Integration tests:** 11 files
- **Unit/Component tests:** ~120 files

## Test Tiers

### Tier 1: Smoke Tests (2-3 minutes)

**When to use:** On every commit via git pre-commit hook

**Script:** `./scripts/smoke-test.sh`

**What it runs:**
- Format check (`cargo fmt --check`)
- Clippy on lib + bins only
- Unit tests (`cargo test --lib`)
- 1 critical integration test (health check)

**Purpose:** Catch obvious errors immediately with minimal time investment.

### Tier 2: Fast Tests (< 5 minutes)

**When to use:** During active development when you want quick feedback

**Script:** `./scripts/fast-tests.sh`

**What it runs:**
- All unit tests
- Fast integration tests (excludes slow patterns)

**What it excludes:**
- E2E tests (require full server startup)
- Comprehensive tests (extensive test scenarios)
- Large integration tests (OAuth flows, multi-tenant, etc.)

**Purpose:** Get rapid feedback on most code changes without waiting for slow tests.

### Tier 3: Pre-Push Tests (5-10 minutes)

**When to use:** Automatically before `git push` via pre-push hook

**Script:** `./scripts/pre-push-tests.sh`

**What it runs:** 20 critical path tests covering:
1. **Critical Infrastructure** (3 tests)
   - Health endpoints
   - Database basics
   - Encryption & crypto keys
2. **Security & Authentication** (5 tests)
   - Authentication
   - API key validation
   - JWT persistence
   - OAuth2 security
   - Security headers
3. **MCP Protocol** (3 tests)
   - MCP compliance
   - JSON-RPC protocol
   - MCP tools
4. **Core Functionality** (4 tests)
   - Error handling (AppResult validation)
   - Data models
   - Database plugins (SQLite/Postgres)
   - Basic integration
5. **Multi-tenancy** (2 tests)
   - Tenant isolation
   - Tenant context
6. **Protocols & Features** (3 tests)
   - A2A protocol basics
   - Algorithm correctness (sports science)
   - Rate limiting middleware

**Purpose:** Catch 80% of issues before pushing to remote, preventing CI failures.

### Tier 4: Category Tests

**When to use:** Testing specific subsystems

**Script:** `./scripts/category-test-runner.sh <category>`

**Available categories:**
- `mcp` - MCP server tests
- `admin` - Admin functionality tests
- `oauth` - OAuth2 tests
- `security` - Security tests
- `database` - Database tests
- `intelligence` - Intelligence/analytics tests
- `config` - Configuration tests
- `auth` - Authentication tests
- `integration` - Integration tests

**Purpose:** Run focused test suites when working on specific features.

### Tier 5: Safe Test Runner

**When to use:** Running the full test suite locally without OOM issues

**Script:** `./scripts/safe-test-runner.sh`

**What it does:**
- Runs ALL 151 test files
- Batches tests (5 tests per batch)
- Pauses between batches for memory cleanup
- Generates detailed logs

**Purpose:** Complete local test validation when needed.

### Tier 6: Full CI Suite (30-60 minutes)

**When to use:** Automatically in GitHub Actions on PRs and pushes

**What it runs:**
- Format check
- Clippy (all targets, all features)
- Security audit (cargo deny)
- Architectural validation
- Secret pattern validation
- All tests with coverage (SQLite + PostgreSQL)
- Frontend tests
- SDK builds

**Purpose:** Comprehensive validation before merging to main branch.

## Test File Naming Conventions

### Slow Tests (should be excluded from fast test runs)
- `*_e2e_test.rs` - End-to-end tests requiring full server
- `*_comprehensive_test.rs` - Extensive test scenarios
- `*_integration.rs` - Integration tests
- Large route tests: `routes_comprehensive_test.rs`, `routes_dashboard_test.rs`, etc.

### Fast Tests (included in fast test runs)
- `*_test.rs` - Standard unit/component tests
- Short route tests: `routes_test.rs`, `routes_health_http_test.rs`
- Module-specific tests

## Developer Workflow

### During Active Development

```bash
# Quick feedback loop (< 5 min)
./scripts/fast-tests.sh

# Or just smoke tests (2-3 min)
./scripts/smoke-test.sh

# Test specific feature
./scripts/category-test-runner.sh mcp
```

### Before Committing

```bash
# Automatic via pre-commit hook
git commit -m "Your message"
# Runs: ./scripts/smoke-test.sh
```

### Before Pushing

```bash
# Automatic via pre-push hook
git push
# Runs: ./scripts/pre-push-tests.sh (5-10 min)
```

### Manual Full Validation

```bash
# Run everything locally (matches CI closely)
./scripts/lint-and-test.sh

# Or just the test suite
./scripts/safe-test-runner.sh
```

## Setting Up Git Hooks

To enable automatic pre-commit and pre-push testing:

```bash
./scripts/setup-git-hooks.sh
```

This installs:
- **Pre-commit hook:** Runs smoke tests (2-3 min)
- **Commit-msg hook:** Enforces 1-2 line commit messages (instant)
- **Pre-push hook:** Runs critical path tests (5-10 min)

### Bypassing Hooks (Emergency Only)

```bash
# Skip pre-commit and commit-msg
git commit --no-verify

# Skip pre-push
git push --no-verify
```

**Warning:** Only bypass hooks for legitimate emergencies. Bypassing hooks increases the risk of CI failures and breaks the fast feedback loop.

## Performance Tips

### Speed Up Local Testing

1. **Use fast tests during development:**
   ```bash
   ./scripts/fast-tests.sh  # Skip slow tests
   ```

2. **Test specific categories:**
   ```bash
   ./scripts/category-test-runner.sh auth  # Just auth tests
   ```

3. **Test single files:**
   ```bash
   cargo test --test routes_health_http_test
   ```

4. **Use watch mode for tight loops:**
   ```bash
   cargo watch -x "test --lib"
   ```

### Optimize Test Execution

Current test execution uses `--test-threads=1` globally due to database contention. Future optimizations:

1. **Increase parallelism for isolated tests**
2. **Use in-memory databases for unit tests**
3. **Mock external dependencies**
4. **Split large test files into smaller, focused tests**

## Test Categories

### Critical Path Tests (Must Pass)
- Health checks
- Authentication
- MCP protocol compliance
- Security basics
- Tenant isolation

### Important Tests (Should Pass)
- All route handlers
- Data models
- Error handling
- Configuration validation

### Extended Tests (Nice to Have)
- Comprehensive edge cases
- Performance tests
- Integration with all providers

## CI Configuration

### SQLite Tests
- Runs on: Every PR, main branch push
- Database: In-memory SQLite
- Coverage: Enabled (codecov)

### PostgreSQL Tests
- Runs on: Every PR, main branch push
- Database: PostgreSQL 16 (GitHub Actions service)
- Coverage: Enabled (codecov)

### Frontend Tests
- Runs on: Every PR, main branch push
- Tools: npm test, ESLint, TypeScript
- Coverage: Enabled (codecov)

## Future Improvements

### Phase 2: Test Organization
- [ ] Add test speed markers/tags
- [ ] Reorganize tests by speed (fast/medium/slow directories)
- [ ] Create test discovery tools

### Phase 3: Test Optimization
- [ ] Split large comprehensive test files
- [ ] Increase parallelism where safe
- [ ] Add mock servers for E2E tests
- [ ] Optimize slow database tests

### Phase 4: Monitoring
- [ ] Add test timing metrics
- [ ] Set up alerts for slow tests
- [ ] Regular performance reviews
- [ ] Track test suite growth

## Troubleshooting

### Tests Timeout Locally

Use the safe test runner with batching:
```bash
./scripts/safe-test-runner.sh
```

### Pre-Push Tests Too Slow

You can adjust the tests in `scripts/pre-push-tests.sh` or temporarily bypass:
```bash
git push --no-verify  # Use sparingly!
```

### CI Fails But Local Tests Pass

1. Check if you're testing with the right database (SQLite vs PostgreSQL)
2. Run the full suite locally: `./scripts/lint-and-test.sh`
3. Check for environment-specific issues

### Out of Memory (OOM) Errors

1. Use batched test runner: `./scripts/safe-test-runner.sh`
2. Run category-specific tests: `./scripts/category-test-runner.sh <category>`
3. Test files individually: `cargo test --test <test_name>`

## Summary

| Tier | Time | When | Command |
|------|------|------|---------|
| Smoke | 2-3 min | Every commit | `./scripts/smoke-test.sh` |
| Fast | < 5 min | Active dev | `./scripts/fast-tests.sh` |
| Pre-push | 5-10 min | Before push | `./scripts/pre-push-tests.sh` |
| Category | Varies | Feature work | `./scripts/category-test-runner.sh <cat>` |
| Full | 15-25 min | Before PR | `./scripts/safe-test-runner.sh` |
| CI | 30-60 min | PR/merge | Automatic in GitHub Actions |

This tiered approach ensures fast feedback during development while maintaining comprehensive coverage in CI.

---

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
4. Architectural validation (`./scripts/architectural-validation.sh`)
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
5. Secret pattern validation (`./scripts/validate-no-secrets.sh`)
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
**Fix**: Run `./scripts/architectural-validation.sh` locally, fix violations

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
./scripts/architectural-validation.sh

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

- GitHub Actions Documentation
- Codecov Documentation
- cargo-deny Configuration
- cargo-llvm-cov Usage

---

# Contributing

## Development Setup

```bash
git clone https://github.com/Async-IO/pierre_mcp_server.git
cd pierre_mcp_server

# install direnv (optional but recommended)
brew install direnv
direnv allow

# build
cargo build

# run tests
cargo test

# run validation
./scripts/lint-and-test.sh
```

## Code Standards

### Rust Idiomatic Code

- prefer borrowing (`&T`) over cloning
- use `Result<T, E>` for all fallible operations
- never use `unwrap()` in production code (tests ok)
- document all public apis with `///` comments
- follow rust naming conventions (snake_case)

### Error Handling

Use structured error types (no anyhow!):
```rust
// bad - anyhow not allowed
use anyhow::Result;

// good - use AppResult and structured errors
use crate::errors::AppResult;

pub async fn my_function() -> AppResult<Value> {
    // errors automatically convert via From trait
    let user = db.users().get_by_id(id).await?;
    Ok(result)
}
```

No panics in production code:
```rust
// bad
let value = some_option.unwrap();

// good
let value = some_option.ok_or(MyError::NotFound)?;
```

**Important**: The codebase enforces zero-tolerance for `impl From<anyhow::Error>` via static analysis (commits b592b5e, 3219f07).

### Forbidden Patterns

- `unwrap()`, `expect()`, `panic!()` in src/ (except tests)
- `#[allow(clippy::...)]` attributes
- variables/functions starting with `_` (use meaningful names)
- hardcoded magic values
- `todo!()`, `unimplemented!()` placeholders

### Required Patterns

- all modules start with aboutme comments:
```rust
// ABOUTME: Brief description of what this module does
// ABOUTME: Second line of description if needed
```

- every `.clone()` must be justified with comment:
```rust
let db = database.clone(); // clone for tokio::spawn thread safety
```

## Testing

### Test Requirements

Every feature needs:
1. **unit tests**: test individual functions
2. **integration tests**: test component interactions
3. **e2e tests**: test complete workflows

No exceptions. If you think a test doesn't apply, ask first.

### Running Tests

```bash
# all tests
cargo test

# specific test
cargo test test_name

# integration tests
cargo test --test mcp_multitenant_complete_test

# with output
cargo test -- --nocapture

# quiet mode
cargo test --quiet
```

### Test Patterns

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_feature() {
        // arrange
        let input = setup_test_data();

        // act
        let result = function_under_test(input).await;

        // assert
        assert!(result.is_ok());
    }
}
```

### Test Location

- unit tests: in same file as code (`#[cfg(test)] mod tests`)
- integration tests: in `tests/` directory
- avoid `#[cfg(test)]` in src/ (tests only)

## Workflow

### Creating Features

1. Create feature branch:
```bash
git checkout -b feature/my-feature
```

2. Implement feature with tests
3. Run validation:
```bash
./scripts/lint-and-test.sh
```

4. Commit:
```bash
git add .
git commit -m "feat: add my feature"
```

5. Push and create pr:
```bash
git push origin feature/my-feature
```

### Fixing Bugs

Bug fixes go directly to main branch:
```bash
git checkout main
# fix bug
git commit -m "fix: correct issue with X"
git push origin main
```

### Commit Messages

Follow conventional commits:
- `feat:` - new feature
- `fix:` - bug fix
- `refactor:` - code refactoring
- `docs:` - documentation changes
- `test:` - test additions/changes
- `chore:` - maintenance tasks

No ai assistant references in commits (automated text removed).

## Validation

### Pre-commit Checks

```bash
./scripts/lint-and-test.sh
```

Runs:
1. Clippy with strict lints
2. Pattern validation (no unwrap, no placeholders)
3. All tests
4. Format check

### Clippy

```bash
cargo clippy -- -W clippy::all -W clippy::pedantic -W clippy::nursery -D warnings
```

Zero tolerance for warnings.

### Pattern Validation

Checks for banned patterns:
```bash
# no unwrap/expect/panic
rg "unwrap\(\)|expect\(|panic!\(" src/

# no placeholders
rg -i "placeholder|todo|fixme" src/

# no clippy allows
rg "#\[allow\(clippy::" src/

# no underscore prefixes
rg "fn _|let _[a-zA-Z]|struct _|enum _" src/
```

### Git Hooks

Install pre-commit hook:
```bash
./scripts/install-hooks.sh
```

Runs validation automatically before commits.

## Architecture Guidelines

### Dependency Injection

Use `Arc<T>` for shared resources:
```rust
pub struct ServerResources {
    pub database: Arc<Database>,
    pub auth_manager: Arc<AuthManager>,
    // ...
}
```

Pass resources to components, not global state.

### Protocol Abstraction

Business logic in `src/protocols/universal/`. Protocol handlers (mcp, a2a) just translate requests/responses.

```rust
// business logic - protocol agnostic
impl UniversalToolExecutor {
    pub async fn execute_tool(&self, tool: &str, params: Value) -> Result<Value> {
        // implementation
    }
}

// protocol handler - translation only
impl McpHandler {
    pub async fn handle_tool_call(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let result = self.executor.execute_tool(&request.tool, request.params).await;
        // translate to json-rpc response
    }
}
```

### Multi-tenant Isolation

Every request needs tenant context:
```rust
pub struct TenantContext {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub role: TenantRole,
}
```

Database queries filter by tenant_id.

### Error Handling

Use thiserror for custom errors:
```rust
#[derive(Debug, thiserror::Error)]
pub enum MyError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("database error")]
    Database(#[from] DatabaseError),
}
```

Propagate with `?` operator.

## Adding New Features

### New Fitness Provider

1. Implement `FitnessProvider` trait in `src/providers/`:
```rust
pub struct NewProvider {
    config: ProviderConfig,
    credentials: Option<OAuth2Credentials>,
}

#[async_trait]
impl FitnessProvider for NewProvider {
    fn name(&self) -> &'static str { "new_provider" }
    // ... implement other methods
}
```

2. Register in `src/providers/registry.rs`
3. Add oauth configuration in `src/oauth/`
4. Add tests

### New MCP Tool

1. Define tool in `src/protocols/universal/tool_registry.rs`:
```rust
pub const TOOL_MY_FEATURE: ToolDefinition = ToolDefinition {
    name: "my_feature",
    description: "Description of what it does",
    input_schema: ...,
};
```

2. Implement handler in `src/protocols/universal/handlers/`:
```rust
pub async fn handle_my_feature(
    context: &UniversalContext,
    params: Value,
) -> Result<Value> {
    // implementation
}
```

3. Register in tool executor
4. Add unit + integration tests
5. Regenerate SDK types:
```bash
# Ensure server is running
cargo run --bin pierre-mcp-server

# Generate TypeScript types
cd sdk
npm run generate-types
git add src/types.ts
```

**Why**: SDK type definitions are auto-generated from server tool schemas. This ensures TypeScript clients have up-to-date parameter types for the new tool.

### New Database Backend

1. Implement repository traits in `src/database_plugins/`:
```rust
use crate::database::repositories::*;

pub struct MyDbProvider { /* ... */ }

// Implement each repository trait for your backend
#[async_trait]
impl UserRepository for MyDbProvider {
    // implement user management methods
}

#[async_trait]
impl OAuthTokenRepository for MyDbProvider {
    // implement oauth token methods
}
// ... implement other 11 repository traits
```

2. Add to factory in `src/database_plugins/factory.rs`
3. Add migration support
4. Add comprehensive tests

**Note**: The codebase uses the repository pattern with 13 focused repository traits (commit 6f3efef). See `src/database/repositories/mod.rs` for the complete list.

## Documentation

### Code Documentation

All public items need doc comments:
```rust
/// Brief description of function
///
/// # Arguments
///
/// * `param` - Description of parameter
///
/// # Returns
///
/// Description of return value
///
/// # Errors
///
/// When this function errors
pub fn my_function(param: Type) -> Result<Type> {
    // implementation
}
```

### Updating Docs

After significant changes:
1. Update relevant docs in `docs/`
2. Keep docs concise and accurate
3. Remove outdated information
4. Test all code examples

## Getting Help

- check existing code for examples
- read rust documentation: https://doc.rust-lang.org/
- ask in github discussions
- open issue for bugs/questions

## Review Process

1. Automated checks must pass (ci) - see ci/cd documentation
2. Code review by maintainer
3. All feedback addressed
4. Tests added/updated
5. Documentation updated
6. Merge to main

### CI/CD Requirements

All GitHub Actions workflows must pass before merge:
- **Rust**: Core quality gate (formatting, clippy, tests)
- **Backend CI**: Multi-database validation (SQLite + PostgreSQL)
- **Cross-Platform**: OS compatibility (Linux, macOS, Windows)
- **SDK Tests**: TypeScript SDK bridge validation
- **MCP Compliance**: Protocol specification conformance

See ci/cd.md for detailed workflow documentation, troubleshooting, and local validation commands.

## Release Process

Handled by maintainers:
1. Version bump in `Cargo.toml`
2. Update changelog
3. Create git tag
4. Publish to crates.io
5. Publish sdk to npm

## Code of Conduct

- be respectful
- focus on technical merit
- welcome newcomers
- assume good intentions
- provide constructive feedback

---

