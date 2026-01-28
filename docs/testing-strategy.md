<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Testing Strategy

## Overview

This document outlines the testing strategy for Pierre MCP Server. The strategy provides fast feedback during development through targeted testing while maintaining comprehensive coverage in CI.

## Test Suite Statistics

- **Total test files:** 195
- **Total test code:** ~62,000 lines
- **Full suite duration:** ~13 minutes (647 tests across 163 files)
- **Clippy full check:** ~2 minutes

## Testing Tiers

### Tier 0: Targeted Tests (During Development)

**When to use:** After every code change

**Command:**
```bash
cargo test --test <test_file> <test_pattern> -- --nocapture
```

**Why targeted tests:**
- Only compiles the specific test file (~5-10 seconds)
- Running without `--test` compiles ALL 163 test files (~2-3 minutes)
- Much faster feedback loop

**Examples:**
```bash
# Run specific test in a file
cargo test --test intelligence_test test_training_load -- --nocapture

# Run all tests in a specific file
cargo test --test store_routes_test -- --nocapture

# List tests in a file
cargo test --test routes_health_http_test -- --list
```

**Finding the right test file:**
```bash
# Find which file contains your test
rg "test_name" tests/ --files-with-matches
```

### Tier 1: Pre-Push Validation

**When to use:** Before `git push`

**Script:** `./scripts/pre-push-validate.sh`

**What it does:**
1. Creates validation marker (valid for 15 minutes)
2. Runs tiered checks based on changed files:
   - **Tier 0:** Code formatting (`cargo fmt --check`)
   - **Tier 1:** Architectural validation
   - **Tier 2:** Schema validation
   - **Tier 3:** Smart test selection based on changed files
   - **Tier 4-6:** Frontend/SDK/Mobile tests (if those directories changed)

**Workflow:**
```bash
# 1. Run validation
./scripts/pre-push-validate.sh

# 2. Push (hook verifies marker exists and is fresh)
git push
```

**Purpose:** Fast, focused validation that catches most issues before CI.

### Tier 2: Full CI Suite

**When to use:** Before PR/merge, or in GitHub Actions

**Script:** `./scripts/lint-and-test.sh`

**What it runs:**
1. Cleanup of generated files
2. Static analysis & code quality validation
3. `cargo fmt --check`
4. `cargo clippy --all-targets` (zero tolerance)
5. `cargo deny check` (security audit)
6. SDK build
7. `cargo test --all-targets` (all tests)
8. Frontend validation (lint, types, unit, E2E, build)
9. MCP compliance validation
10. SDK TypeScript validation + integration tests
11. Bridge test suite
12. Release build + documentation

**Duration:** ~30-60 minutes

**Purpose:** Comprehensive validation before merging to main.

## Test File Naming Conventions

| Pattern | Description |
|---------|-------------|
| `*_test.rs` | Standard unit/component tests |
| `*_e2e_test.rs` | End-to-end tests requiring full server |
| `*_comprehensive_test.rs` | Extensive test scenarios |
| `*_integration.rs` | Integration tests |

## Developer Workflow

### During Active Development

```bash
# Run targeted tests for the module you're changing
cargo test --test <test_file> <pattern> -- --nocapture

# Examples:
cargo test --test mcp_tools_unit test_activities -- --nocapture
cargo test --test auth_test -- --nocapture
cargo test --test intelligence_algorithms_test -- --nocapture
```

### Before Committing

```bash
# 1. Format code
cargo fmt

# 2. Architectural validation
./scripts/architectural-validation.sh

# 3. Clippy (strict mode)
cargo clippy --all-targets

# 4. Run targeted tests for changed modules
cargo test --test <test_file> <pattern> -- --nocapture
```

### Before Pushing

```bash
# 1. Run validation (creates marker valid for 15 min)
./scripts/pre-push-validate.sh

# 2. Push (hook verifies marker)
git push
```

### Manual Full Validation

```bash
# Run full CI suite locally
./scripts/lint-and-test.sh
```

## Setting Up Git Hooks

```bash
# One-time setup
git config core.hooksPath .githooks
```

The pre-push hook verifies:
- Validation marker exists
- Marker is fresh (< 15 minutes)
- Marker matches current commit

### Bypassing Hooks (Emergency Only)

```bash
git push --no-verify
```

**Warning:** Only bypass for legitimate emergencies. CI will still run.

## Specialized Testing

### PostgreSQL Integration

```bash
# Requires Docker
./scripts/test-postgres.sh
```

### SDK/Bridge Tests

```bash
./scripts/run_bridge_tests.sh
```

### MCP Protocol Compliance

```bash
./scripts/ensure_mcp_compliance.sh
```

### Frontend Tests

```bash
# Web frontend
./scripts/pre-push-frontend-tests.sh

# Mobile
./scripts/pre-push-mobile-tests.sh
```

## Performance Tips

### Speed Up Local Testing

1. **Always use targeted tests:**
   ```bash
   # ❌ Slow - compiles all 163 test files
   cargo test test_browse_store

   # ✅ Fast - only compiles one test file
   cargo test --test store_routes_test test_browse_store
   ```

2. **Use watch mode for tight loops:**
   ```bash
   cargo watch -x "test --test <file> <pattern>"
   ```

3. **Skip tests during clippy:**
   ```bash
   cargo clippy -p pierre_mcp_server --all-targets
   ```

### Test Targeting Patterns

| Scenario | Command |
|----------|---------|
| Run one test | `cargo test --test <file> <test_name>` |
| Run all tests in file | `cargo test --test <file>` |
| List tests in file | `cargo test --test <file> -- --list` |
| Run with output | `cargo test --test <file> <test> -- --nocapture` |

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

## Troubleshooting

### CI Fails But Local Tests Pass

1. Check if you're testing with the right database (SQLite vs PostgreSQL)
2. Run the full suite locally: `./scripts/lint-and-test.sh`
3. Check for environment-specific issues

### Validation Marker Expired

```bash
# Re-run validation to create fresh marker
./scripts/pre-push-validate.sh
```

### Finding Which Tests to Run

```bash
# Find test files for a module
rg "mod_name" tests/ --files-with-matches

# Find test files mentioning a function
rg "function_name" tests/ --files-with-matches
```

## Summary

| Tier | Time | When | Command |
|------|------|------|---------|
| Targeted | ~5-10s | Every change | `cargo test --test <file> <pattern>` |
| Pre-push | ~1-5 min | Before push | `./scripts/pre-push-validate.sh` |
| Full CI | ~30-60 min | PR/merge | `./scripts/lint-and-test.sh` |

This approach prioritizes fast feedback during development while ensuring comprehensive validation before code reaches main.
