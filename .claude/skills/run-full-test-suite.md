---
name: run-full-test-suite
description: Executes comprehensive test suite across unit, integration, E2E, database, protocols, and intelligence algorithms
---

# Run Full Test Suite Skill

## Purpose
Executes comprehensive test suite across all categories: unit, integration, E2E, database, protocols, and intelligence algorithms.

## CLAUDE.md Compliance
- ✅ Runs all deterministic tests
- ✅ Uses synthetic data (no external dependencies)
- ✅ Tests both success and error paths
- ✅ Validates code quality

## Usage
Run this skill:
- Before committing code
- Before pull requests
- Before releases
- After major refactoring
- Daily CI validation

## Prerequisites
- Cargo and Rust toolchain
- Test dependencies installed

## Commands

### Full Test Suite
```bash
# Run ALL tests (unit + integration + doc tests)
cargo test --all-features
```

### Category-Based Testing
```bash
# Use the category test runner
./scripts/category-test-runner.sh all
```

### Specific Test Categories

#### Unit Tests
```bash
# All library unit tests
cargo test --lib -- --quiet
```

#### Integration Tests
```bash
# All integration tests
cargo test --test '*' -- --quiet

# Specific integration test
cargo test --test mcp_multitenant_complete_test -- --nocapture
```

#### Doc Tests
```bash
# Documentation example tests
cargo test --doc -- --quiet
```

#### Intelligence Tests
```bash
# Basic intelligence algorithms
cargo test --test intelligence_tools_basic_test -- --nocapture

# Advanced intelligence algorithms
cargo test --test intelligence_tools_advanced_test -- --nocapture
```

#### Protocol Tests
```bash
# MCP protocol tests
cargo test protocol -- --quiet

# OAuth tests
cargo test oauth -- --quiet

# Authentication tests
cargo test auth -- --quiet
```

#### Database Tests
```bash
# Database abstraction layer
cargo test database --lib -- --quiet

# Database plugins (SQLite)
cargo test --test database_plugins_comprehensive_test --features sqlite

# PostgreSQL (requires Docker)
./scripts/test-postgres.sh
```

### Performance Testing
```bash
# Run benchmarks (if configured)
cargo bench --bench '*' || echo "No benchmarks configured"
```

### Parallel vs Sequential
```bash
# Parallel execution (default, faster)
cargo test --all-features

# Sequential execution (for database tests with shared state)
cargo test --all-features -- --test-threads=1
```

## Test Output Modes

### Quiet Mode (Summary Only)
```bash
# Show only pass/fail summary
cargo test --all-features --quiet
```

### Verbose Mode (Show Output)
```bash
# Show println! and debug output
cargo test --all-features -- --nocapture
```

### Show Only Failures
```bash
# Run tests and show only failures
cargo test --all-features 2>&1 | grep -A 10 "FAILED"
```

## Test Filtering

### By Name
```bash
# Run specific test
cargo test test_vdot_calculation

# Run tests matching pattern
cargo test multitenant

# Run tests in specific module
cargo test intelligence::algorithms
```

### By Feature Flag
```bash
# Test with specific features
cargo test --features sqlite
cargo test --features postgresql
cargo test --features testing
```

### Exclude Tests
```bash
# Skip expensive tests
cargo test --all-features -- --skip test_expensive_operation

# Skip integration tests
cargo test --lib
```

## Expected Test Results

### Success Output
```
running 245 tests
test auth::tests::test_jwt_validation ... ok
test database::tests::test_tenant_scoping ... ok
test intelligence::algorithms::vdot::tests::test_daniels_formula ... ok
test protocols::mcp::tests::test_jsonrpc_format ... ok
...

test result: ok. 245 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### With Coverage Summary
```
   Doc-tests pierre_mcp_server

running 12 tests
test src/intelligence/algorithms/vdot.rs - intelligence::algorithms::vdot::calculate_vdot (line 45) ... ok
...

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Test Coverage by Area

### Core Functionality (~100 tests)
- Authentication & Authorization
- Multi-tenant isolation
- Database operations
- Cryptography

### Protocol Layer (~50 tests)
- MCP protocol (JSON-RPC 2.0)
- A2A protocol
- OAuth 2.0 server
- OAuth 2.0 client

### Intelligence (~60 tests)
- VDOT calculations
- TSS/CTL/ATL/TSB
- TRIMP calculations
- FTP estimation
- VO2max estimation
- Recovery scoring
- Sleep analysis
- Nutrition calculations

### Infrastructure (~35 tests)
- Transport layers (HTTP, stdio, WebSocket, SSE)
- Caching
- Rate limiting
- Middleware
- Health checks

## CI/CD Integration

### GitHub Actions Workflow
```yaml
# .github/workflows/rust.yml
- name: Run tests
  run: cargo test --all-features --verbose
```

### Pre-Commit Hook
```bash
# Run fast tests before commit
cargo test --lib --quiet || exit 1
```

## Troubleshooting

### Issue: Tests hang
```bash
# Identify hanging test
cargo test --all-features -- --nocapture --test-threads=1

# Common causes:
# - Deadlock in async code
# - Infinite loop
# - Waiting for external resource
```

### Issue: Flaky tests
```bash
# Run test multiple times to reproduce
for i in {1..10}; do
    cargo test test_flaky_test || echo "Failed on iteration $i"
done

# Common causes:
# - Race conditions
# - Non-deterministic RNG (use seeded RNG!)
# - Time-dependent logic
```

### Issue: Database locked
```bash
# Use serial_test for DB tests
#[serial_test::serial]
#[tokio::test]
async fn test_database_operation() { }

# Or run with single thread
cargo test --all-features -- --test-threads=1
```

### Issue: Out of memory
```bash
# Reduce parallel test execution
cargo test --all-features -- --test-threads=4

# Or run test categories separately
cargo test --lib
cargo test --test mcp_multitenant_complete_test
```

## Test Maintenance

### Finding Slow Tests
```bash
# Run with timing
cargo test --all-features -- --nocapture --test-threads=1 | grep -E "test.*ok in"

# Tests > 1 second should be reviewed
```

### Test Coverage Analysis
```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --out Html --output-dir coverage/

# Open coverage/index.html
```

### Unused Test Code
```bash
# Find unused test utilities
cargo test --all-features 2>&1 | grep "warning.*unused"
```

## Success Criteria
- ✅ All unit tests pass (>100 tests)
- ✅ All integration tests pass (>50 tests)
- ✅ All doc tests pass
- ✅ No flaky tests
- ✅ No ignored tests without explanation
- ✅ Test coverage > 80%
- ✅ No test failures in CI
- ✅ All tests complete in < 5 minutes

## Quick Test Commands Cheat Sheet

```bash
# Fast check (unit tests only)
cargo test --lib --quiet

# Full test suite
cargo test --all-features

# Specific test with output
cargo test test_name -- --nocapture

# Multi-tenant isolation
cargo test --test mcp_multitenant_complete_test

# Intelligence algorithms
cargo test --test intelligence_tools_basic_test

# Database tests
cargo test database

# Protocol compliance
cargo test protocol

# Authentication
cargo test auth oauth

# Everything in parallel
./scripts/category-test-runner.sh all
```

## Related Files
- `scripts/category-test-runner.sh` - Test orchestration
- `tests/` - Integration tests directory
- `tests/common.rs` - Shared test utilities

## Related Skills
- `test-multitenant-isolation.md` - Multi-tenant testing
- `test-intelligence-algorithms.md` - Algorithm validation
- `test-mcp-compliance.md` - Protocol compliance
- `test-orchestrator.md` (agent) - Comprehensive orchestration
