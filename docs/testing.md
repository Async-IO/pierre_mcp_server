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
