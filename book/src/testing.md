<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Testing

This guide covers the testing strategy and practices for Pierre MCP Server.

## Test Suite Overview

| Metric | Value |
|--------|-------|
| Total test files | 195 |
| Total test code | ~62,000 lines |
| Full suite duration | ~13 minutes |
| Total tests | 647 tests across 163 files |

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

**Script:** `./scripts/ci/pre-push-validate.sh`

**What it does:**
1. Creates validation marker (valid for 15 minutes)
2. Runs tiered checks based on changed files:
   - Code formatting (`cargo fmt --check`)
   - Architectural validation
   - Schema validation
   - Smart test selection based on changed files
   - Frontend/SDK/Mobile tests (if those directories changed)

**Workflow:**
```bash
# 1. Run validation
./scripts/ci/pre-push-validate.sh

# 2. Push (hook verifies marker exists and is fresh)
git push
```

### Tier 2: Full CI Suite

**When to use:** Before PR/merge, or in GitHub Actions

**Script:** `./scripts/ci/lint-and-test.sh`

**What it runs:**
1. Static analysis & code quality validation
2. `cargo fmt --check`
3. `cargo clippy --all-targets` (zero tolerance)
4. `cargo deny check` (security audit)
5. SDK build
6. `cargo test --all-targets` (all tests)
7. Frontend validation (lint, types, unit, E2E, build)
8. MCP compliance validation
9. SDK TypeScript validation + integration tests

**Duration:** ~30-60 minutes

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
./scripts/ci/lint-and-test.sh
```

## Test File Naming Conventions

| Pattern | Description |
|---------|-------------|
| `*_test.rs` | Standard unit/component tests |
| `*_e2e_test.rs` | End-to-end tests requiring full server |
| `*_comprehensive_test.rs` | Extensive test scenarios |
| `*_integration.rs` | Integration tests |

## Multi-Tenant Tests

Tests validating MCP protocol with multi-tenant isolation:

```bash
# Rust multi-tenant MCP tests
cargo test --test mcp_multitenant_sdk_e2e_test

# Type generation multi-tenant validation
cargo test --test mcp_type_generation_multitenant_test

# SDK multi-tenant tests
cd sdk
bun run test -- --testPathPattern=e2e-multitenant
cd ..
```

**Test Coverage:**
- Concurrent multi-tenant tool calls without data leakage
- HTTP and SDK transport parity
- Tenant isolation at protocol level (403/404 errors for unauthorized access)
- Type generation consistency across tenants
- Rate limiting per tenant
- SDK concurrent access by multiple tenants

**Test Infrastructure** (`tests/common.rs` and `sdk/test/helpers/`):
- `spawn_sdk_bridge()`: Spawns SDK process with JWT token and automatic cleanup
- `send_http_mcp_request()`: Direct HTTP MCP requests for transport testing
- `create_test_tenant()`: Creates tenant with user and JWT token

## Intelligence Testing Framework

The platform includes 30+ integration tests covering all 8 intelligence tools without OAuth dependencies:

**Test Categories:**
- **Basic Tools:** `get_athlete`, `get_activities`, `get_stats`, `compare_activities`
- **Advanced Analytics:** `calculate_fitness_score`, `predict_performance`, `analyze_training_load`
- **Goal Management:** `suggest_goals`, `analyze_goal_feasibility`, `track_progress`

**Synthetic Data Scenarios:**
- Beginner runner improving over time
- Experienced cyclist with consistent training
- Multi-sport athlete (triathlete pattern)
- Training gaps and recovery periods

See `tests/intelligence_tools_basic_test.rs` and `tests/intelligence_tools_advanced_test.rs` for details.

## RSA Key Size Configuration

Pierre uses RS256 asymmetric signing for JWT tokens. Key size affects both security and performance:

**Production (4096-bit keys - default):**
- Higher security with larger key size
- Slower key generation (~10 seconds)

**Testing (2048-bit keys):**
- Faster key generation (~250ms)
- Set via environment variable:

```bash
export PIERRE_RSA_KEY_SIZE=2048
```

## Test Performance Optimization

### Shared Test JWKS

Pierre includes a shared test JWKS manager to eliminate RSA key generation overhead:

```rust
use pierre_mcp_server_integrations::common;

// Reuses shared JWKS manager across all tests (10x faster)
let jwks_manager = common::get_shared_test_jwks();
```

**Performance Impact:**
- **Without optimization:** 100ms+ RSA key generation per test
- **With shared JWKS:** One-time generation, instant reuse
- **Result:** 10x faster test execution

### Speed Tips

1. **Always use targeted tests:**
   ```bash
   # Slow - compiles all 163 test files
   cargo test test_browse_store

   # Fast - only compiles one test file
   cargo test --test store_routes_test test_browse_store
   ```

2. **Use watch mode for tight loops:**
   ```bash
   cargo watch -x "test --test <file> <pattern>"
   ```

## Specialized Testing

### PostgreSQL Integration

```bash
# Requires Docker
./scripts/testing/test-postgres.sh
```

### SDK/Bridge Tests

```bash
./scripts/testing/run-bridge-tests.sh
```

### MCP Protocol Compliance

```bash
./scripts/ci/ensure-mcp-compliance.sh
```

### Frontend Tests

```bash
# Web frontend
./scripts/ci/pre-push-frontend-tests.sh

# Mobile
./scripts/ci/pre-push-mobile-tests.sh
```

## Git Hooks Setup

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

## CI Configuration

| Environment | Trigger | Database | Coverage |
|-------------|---------|----------|----------|
| SQLite | Every PR, main push | In-memory SQLite | Enabled |
| PostgreSQL | Every PR, main push | PostgreSQL 16 | Enabled |
| Frontend | Every PR, main push | N/A | Enabled |

## Troubleshooting

### CI Fails But Local Tests Pass

1. Check if you're testing with the right database (SQLite vs PostgreSQL)
2. Run the full suite locally: `./scripts/ci/lint-and-test.sh`
3. Check for environment-specific issues

### Validation Marker Expired

```bash
# Re-run validation to create fresh marker
./scripts/ci/pre-push-validate.sh
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
| Pre-push | ~1-5 min | Before push | `./scripts/ci/pre-push-validate.sh` |
| Full CI | ~30-60 min | PR/merge | `./scripts/ci/lint-and-test.sh` |
