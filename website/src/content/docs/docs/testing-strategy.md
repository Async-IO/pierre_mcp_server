---
title: "Testing Strategy"
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
