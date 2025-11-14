# Claude Code for Web: MCP Multi-Tenant + SDK E2E Tests Implementation

## Initial Setup

**CRITICAL: Before starting, fetch and follow my coding standards:**

```
Fetch my global Claude.md guidelines from:
https://gist.githubusercontent.com/jfarcand/82f32197bac97516261274edd818a4fc/raw/CLAUDE.md

Save this as reference and follow ALL rules throughout this implementation.
```

**Key Rules to Follow**:
- Address me as "ChefFamille"
- NEVER use `--no-verify` when committing
- Run validation BEFORE claiming completion: `cargo fmt`, `./scripts/architectural-validation.sh`, strict clippy, `cargo test`
- Use Result-based error handling (NO `unwrap()`, `expect()`, `panic!()` in production code)
- Tests can use `unwrap()` and `expect()` (allowed in test code)
- NO `anyhow!()` macro in production code (use structured error types)
- Document all `Arc<T>` usage with justification
- Work directly on current branch (ChefFamille will create feature branch if needed)

## Task Overview

Implement comprehensive **MCP Protocol HTTP + Multi-Tenant + SDK end-to-end tests** without duplicating existing test framework. Follow the plan documented in:

**`docs/mcp-multitenant-sdk-e2e-test-plan.md`**

Read this plan thoroughly before starting. It contains:
- Architecture analysis
- Test scenarios (6 scenarios)
- Implementation phases (5 weeks compressed to focused implementation)
- Infrastructure extensions
- Success metrics

## Implementation Instructions

### Phase 1: Foundation (First Priority)

**Goal**: Establish infrastructure for hybrid Rust + SDK testing

#### Task 1.1: Extend `tests/common.rs` with SDK Bridge Helpers

Add these helper functions to `tests/common.rs`:

```rust
/// Spawn SDK bridge process for testing
/// Returns handle that automatically cleans up on drop
pub async fn spawn_sdk_bridge(jwt_token: &str, server_port: u16) -> Result<SdkBridgeHandle> {
    // Implementation details in plan document
}

/// Send HTTP MCP request directly to server
pub async fn send_http_mcp_request(
    url: &str,
    method: &str,
    params: Value,
    jwt_token: &str,
) -> Result<Value> {
    // Implementation details in plan document
}

/// Create test tenant with user and token
pub async fn create_test_tenant(
    resources: &ServerResources,
    email: &str,
) -> Result<(User, String)> {
    // Reuse existing create_test_user_with_email + token generation
}
```

**Validation**:
- Functions compile without errors
- Functions have proper documentation
- No `unwrap()` or `expect()` in function bodies (use `?` operator)
- `SdkBridgeHandle` implements `Drop` for cleanup

**Commit**: `feat: add SDK bridge spawn helpers to test common utilities`

#### Task 1.2: Create SDK Multi-Tenant Helpers

Create new file: `sdk/test/helpers/multitenant-setup.js`

```javascript
/**
 * Setup multiple MCP clients for multi-tenant testing
 * @param {number} numTenants - Number of tenant clients to create
 * @returns {Promise<Array>} Array of client objects with {user, token, client}
 */
async function setupMultiTenantClients(numTenants = 2) {
    // Implementation from plan document
}

/**
 * Create tenant via Rust server HTTP endpoint
 * @param {string} email - Tenant email
 * @returns {Promise<{user, token}>} User and JWT token
 */
async function createTenantViaRustBridge(email) {
    // Implementation from plan document
}

module.exports = {
    setupMultiTenantClients,
    createTenantViaRustBridge
};
```

**Validation**:
- File follows existing SDK test helper patterns
- JSDoc comments for all functions
- Proper error handling

**Commit**: `feat: add SDK multi-tenant test setup helpers`

#### Task 1.3: Create Rust Server Bridge Helper

Create new file: `sdk/test/helpers/rust-server-bridge.js`

```javascript
/**
 * Bridge between SDK tests and Rust test server
 * Provides utilities for coordinating multi-tenant scenarios
 */

/**
 * Call Rust test endpoint to create tenant
 */
async function createTenantOnServer(serverUrl, email) {
    // Implementation
}

/**
 * Cleanup all test tenants via Rust endpoint
 */
async function cleanupTestTenants(serverUrl) {
    // Implementation
}

module.exports = {
    createTenantOnServer,
    cleanupTestTenants
};
```

**Validation**:
- Integrates with existing `sdk/test/helpers/server.js`
- Proper error handling and cleanup

**Commit**: `feat: add Rust server bridge for SDK multi-tenant coordination`

---

### Phase 2: Rust Coordinator Tests

**Goal**: Implement Rust-side multi-tenant MCP tests

#### Task 2.1: Create Main Test File

Create new file: `tests/mcp_multitenant_sdk_e2e_test.rs`

**File Header**:
```rust
// ABOUTME: End-to-end tests for MCP protocol with multi-tenant isolation via SDK and HTTP
// ABOUTME: Validates tenant isolation, transport parity, and concurrent access patterns
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use anyhow::Result;
use pierre_mcp_server::mcp::resources::ServerResources;
use serde_json::{json, Value};

mod common;

// Tests go here
```

**Validation**:
- File compiles
- Follows existing test file patterns
- Imports necessary dependencies

**Commit**: `feat: create mcp_multitenant_sdk_e2e_test file structure`

#### Task 2.2: Implement Scenario 1 - Concurrent Multi-Tenant Tool Calls

Add test to `tests/mcp_multitenant_sdk_e2e_test.rs`:

```rust
#[tokio::test]
async fn test_concurrent_multitenant_get_activities() -> Result<()> {
    // Setup: Create 3 tenants with separate users
    // Spawn 3 SDK bridges (one per tenant)
    // Simultaneously call get_activities for all tenants
    // Validate: No cross-tenant data leakage
    // Cleanup: All SDK processes and resources

    // Implementation details in plan document (Scenario 1)
}
```

**Validation**:
- Test passes: `cargo test test_concurrent_multitenant_get_activities`
- No resource leaks (check with `lsof` during test)
- Test completes in <10 seconds

**Commit**: `test: add concurrent multi-tenant tool calls test (Scenario 1)`

#### Task 2.3: Implement Scenario 2 - HTTP vs SDK Transport Parity

Add test:

```rust
#[tokio::test]
async fn test_http_vs_sdk_transport_parity() -> Result<()> {
    // Setup: Create 1 tenant
    // Call get_athlete via HTTP (direct server request)
    // Call get_athlete via SDK (stdio bridge)
    // Compare responses (should be identical)
    // Repeat for 10+ tools

    // Implementation details in plan document (Scenario 2)
}
```

**Validation**:
- Test passes
- Validates responses are identical for all tested tools
- Logs which tools were tested

**Commit**: `test: add HTTP vs SDK transport parity test (Scenario 2)`

#### Task 2.4: Implement Scenario 3 - Tenant Isolation at Protocol Level

Add test:

```rust
#[tokio::test]
async fn test_tenant_isolation_protocol_level() -> Result<()> {
    // Setup: Create 2 tenants (T1, T2)
    // T1 calls get_activities and gets activity IDs
    // T2 attempts to access T1's activities by ID
    // Validate: T2 receives 403 Forbidden or 404 Not Found

    // Implementation details in plan document (Scenario 3)
}
```

**Validation**:
- Test passes
- Validates proper HTTP status codes (403 or 404)
- Validates error messages are appropriate

**Commit**: `test: add tenant isolation protocol level test (Scenario 3)`

#### Task 2.5: Implement Scenario 5 - Rate Limiting Per Tenant

Add test:

```rust
#[tokio::test]
async fn test_rate_limiting_per_tenant_isolation() -> Result<()> {
    // Setup: Create 2 tenants with different tiers
    // T1 (free tier) makes requests until rate limited
    // T2 (professional tier) continues making requests
    // Validate: T1 rate limit does not affect T2

    // Implementation details in plan document (Scenario 5)
}
```

**Validation**:
- Test passes
- Validates 429 status code for rate-limited tenant
- Validates other tenant unaffected

**Commit**: `test: add per-tenant rate limiting isolation test (Scenario 5)`

---

### Phase 3: SDK Multi-Tenant Tests

**Goal**: Implement TypeScript SDK-side multi-tenant tests

#### Task 3.1: Create SDK E2E Multi-Tenant Directory

Create directory structure:
```
sdk/test/e2e-multitenant/
├── concurrent-tenants.test.js
├── tenant-isolation.test.js
└── type-consistency.test.js
```

**Commit**: `feat: create SDK e2e-multitenant test directory structure`

#### Task 3.2: Implement Concurrent Tenants Test

Create `sdk/test/e2e-multitenant/concurrent-tenants.test.js`:

```javascript
// ABOUTME: Tests concurrent access by multiple tenants via SDK bridge
// ABOUTME: Validates no cross-tenant data leakage during simultaneous tool calls

const { setupMultiTenantClients } = require('../helpers/multitenant-setup');
const { ensureServerRunning } = require('../helpers/server');

describe('Multi-Tenant Concurrent Access via SDK', () => {
    // Implementation from plan document (Scenario 1 - SDK perspective)

    test('Multiple tenants call get_activities simultaneously', async () => {
        // Test implementation
    });

    test('Multiple tenants call different tools concurrently', async () => {
        // Test implementation
    });
});
```

**Validation**:
- Tests pass: `npm run test:e2e`
- No flaky failures (run 5 times)
- Proper cleanup of all clients

**Commit**: `test: add SDK concurrent tenants e2e test`

#### Task 3.3: Implement Tenant Isolation Test

Create `sdk/test/e2e-multitenant/tenant-isolation.test.js`:

```javascript
// ABOUTME: Tests tenant isolation via SDK bridge
// ABOUTME: Validates cross-tenant access is properly forbidden

describe('Multi-Tenant Isolation via SDK', () => {
    // Implementation from plan document (Scenario 3 - SDK perspective)

    test('Tenant cannot access another tenant activities', async () => {
        // Test implementation
    });

    test('Tenant receives proper error codes for forbidden access', async () => {
        // Validate 403/404 responses
    });
});
```

**Validation**:
- Tests pass
- Validates error codes and messages
- Cleanup works properly

**Commit**: `test: add SDK tenant isolation e2e test`

#### Task 3.4: Implement Type Consistency Test

Create `sdk/test/e2e-multitenant/type-consistency.test.js`:

```javascript
// ABOUTME: Tests type schema consistency across multiple tenants
// ABOUTME: Validates tools/list returns identical schemas for all tenants

describe('Multi-Tenant Type Consistency', () => {
    // Implementation from plan document (Scenario 4)

    test('All tenants receive identical tool schemas', async () => {
        // Fetch schemas for 3 tenants
        // Compare schemas (should be identical)
    });

    test('Generated TypeScript types match all tenant schemas', async () => {
        // Validate type generation consistency
    });
});
```

**Validation**:
- Tests pass
- Schemas are compared correctly
- Type generation validation works

**Commit**: `test: add SDK type consistency e2e test`

---

### Phase 4: Type Generation Validation

**Goal**: Validate type generation for multi-tenant scenarios

#### Task 4.1: Create Type Generation Test

Create new file: `tests/mcp_type_generation_multitenant_test.rs`

```rust
// ABOUTME: Validates MCP type generation produces consistent schemas across tenants
// ABOUTME: Ensures auto-generated TypeScript types are tenant-agnostic

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use anyhow::Result;
use serde_json::Value;

mod common;

#[tokio::test]
async fn test_type_schemas_identical_across_tenants() -> Result<()> {
    // Implementation from plan document (Scenario 4)
    // Create 5 tenants with different configurations
    // Fetch tool schemas for each tenant
    // Validate all schemas are identical
}

#[tokio::test]
async fn test_generated_types_match_schemas() -> Result<()> {
    // Fetch schemas from server
    // Parse existing generated types from sdk/src/types.ts
    // Validate generated types match schemas
}
```

**Validation**:
- Tests pass
- Schemas comparison is accurate
- Type parsing logic is robust

**Commit**: `test: add type generation multi-tenant validation tests`

---

### Phase 5: Integration and Documentation

**Goal**: Integrate all tests and update documentation

#### Task 5.1: Update Test Documentation

Update `README.md` test section:

```markdown
## Testing

### Multi-Tenant End-to-End Tests

Comprehensive tests validating MCP protocol with multi-tenant isolation:

```bash
# Rust multi-tenant MCP tests
cargo test --test mcp_multitenant_sdk_e2e_test
cargo test --test mcp_type_generation_multitenant_test

# SDK multi-tenant tests
cd sdk
npm run test:e2e
```

Test coverage:
- ✅ Concurrent multi-tenant tool calls
- ✅ HTTP vs SDK transport parity
- ✅ Tenant isolation at protocol level
- ✅ Type generation consistency
- ✅ Rate limiting per tenant
```

**Commit**: `docs: update README with multi-tenant e2e test documentation`

#### Task 5.2: Run Full Validation Suite

**Before final commit, run ALL validation steps**:

```bash
# 1. Format code
cargo fmt

# 2. Architectural validation
./scripts/architectural-validation.sh

# 3. Strict clippy (zero tolerance)
cargo clippy --tests -- \
  -W clippy::all \
  -W clippy::pedantic \
  -W clippy::nursery \
  -D warnings

# 4. Run ALL Rust tests
cargo test

# 5. Run ALL SDK tests
cd sdk && npm test && cd ..

# 6. Run new multi-tenant tests specifically
cargo test --test mcp_multitenant_sdk_e2e_test -- --nocapture
cargo test --test mcp_type_generation_multitenant_test -- --nocapture
cd sdk && npm run test:e2e -- --testPathPattern=e2e-multitenant && cd ..
```

**If ANY validation fails**:
- Fix the issues immediately
- Do NOT commit until all validations pass
- Re-run full validation suite

**Only when ALL validations pass**:

```bash
# Create final commit
git add -A
git commit -m "feat: add comprehensive MCP multi-tenant + SDK e2e tests

- Add SDK bridge spawn helpers to tests/common.rs
- Add SDK multi-tenant test setup utilities
- Implement 6 test scenarios:
  1. Concurrent multi-tenant tool calls
  2. HTTP vs SDK transport parity
  3. Tenant isolation at protocol level
  4. Type generation consistency
  5. Rate limiting per tenant
  6. OAuth flow multi-tenant

Test coverage:
- Rust: +5 new test functions in 2 new test files
- SDK: +10 new test functions in 3 new test files
- Total: ~100-150 new test assertions

All tests pass:
- cargo test: PASSED
- cargo clippy --tests (strict): PASSED
- npm test (SDK): PASSED
- architectural validation: PASSED"
```

---

## Execution Guidelines for CCFW

### Working Style

1. **Read the plan first**: Thoroughly review `docs/mcp-multitenant-sdk-e2e-test-plan.md`

2. **Work incrementally**:
   - Implement one phase at a time
   - Commit after each completed task
   - Validate after each commit

3. **Follow existing patterns**:
   - Study existing test files in `tests/`
   - Study existing SDK tests in `sdk/test/`
   - Match the style and structure

4. **Ask for clarification** if:
   - Plan details are unclear
   - Existing test patterns are ambiguous
   - Infrastructure doesn't match plan expectations

5. **Report progress**:
   - After each phase completion
   - If you encounter blockers
   - When validation fails

### Success Criteria

**Phase 1 Complete When**:
- All helper functions compile and have tests
- SDK helper files exist and are valid JavaScript
- `cargo test` passes (no regressions)

**Phase 2 Complete When**:
- All 4 Rust tests pass individually
- All 4 Rust tests pass concurrently
- No clippy warnings in new code

**Phase 3 Complete When**:
- All 3 SDK test files pass
- No flaky tests (run each 5 times)
- Proper cleanup verified

**Phase 4 Complete When**:
- Type generation tests pass
- Schema validation is accurate
- No regressions in type generation

**Phase 5 Complete When**:
- Documentation updated
- Full validation suite passes
- Final commit created

### Validation Checklist (Before Final Commit)

```
☐ cargo fmt (no changes needed)
☐ ./scripts/architectural-validation.sh (PASSED)
☐ cargo clippy --tests (strict mode, zero warnings)
☐ cargo test (all tests pass)
☐ npm test (all SDK tests pass)
☐ New tests run successfully:
  ☐ cargo test --test mcp_multitenant_sdk_e2e_test
  ☐ cargo test --test mcp_type_generation_multitenant_test
  ☐ cd sdk && npm run test:e2e -- --testPathPattern=e2e-multitenant
☐ No resource leaks (verified with lsof during test runs)
☐ Documentation updated in README.md
☐ All commits follow conventional commit format
```

---

## Emergency Contacts / Questions

If you encounter issues:

1. **Review the plan**: `docs/mcp-multitenant-sdk-e2e-test-plan.md`
2. **Check existing tests**: Study similar patterns in `tests/` and `sdk/test/`
3. **Ask ChefFamille**: Report blockers clearly with context

## Ready to Start?

Confirm you:
1. ✅ Fetched and read https://gist.githubusercontent.com/jfarcand/82f32197bac97516261274edd818a4fc/raw/CLAUDE.md
2. ✅ Read the implementation plan: `docs/mcp-multitenant-sdk-e2e-test-plan.md`
3. ✅ Understand the validation requirements (fmt, clippy strict, tests)
4. ✅ Ready to work on current branch (ChefFamille manages branching)

Then proceed with **Phase 1, Task 1.1** and work through each phase sequentially.

Good luck! 🚀
