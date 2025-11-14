# MCP Protocol + Multi-Tenant + SDK End-to-End Test Plan

**Author**: Senior Rust Engineer (ChefFamille)
**Date**: 2025-11-14
**Status**: Proposal

## Executive Summary

This plan proposes a comprehensive testing strategy to add **MCP Protocol HTTP + Multi-tenant + SDK integration tests** without duplicating the existing test framework. The plan leverages existing infrastructure (1400+ tests, type generation, shared utilities) and fills a critical gap in end-to-end testing coverage.

## Current State Analysis

### Existing Test Coverage (1400+ tests)

#### Rust Tests (~1436 tests in 140 files)
**Location**: `tests/*.rs`

**Coverage**:
- ✅ MCP Protocol compliance (HTTP endpoints, JSON-RPC 2.0)
- ✅ Multi-tenant database isolation
- ✅ OAuth 2.0 flows (PKCE, state validation, token refresh)
- ✅ Intelligence tools (synthetic data, no real OAuth)
- ✅ Database operations (SQLite + PostgreSQL)
- ✅ Rate limiting (per-tenant, per-user)

**Key Infrastructure** (`tests/common.rs`):
- Shared JWKS manager (10x faster test execution)
- `ServerResources` pattern for consistent setup
- Test database creation (memory + persistent)
- User/tenant creation helpers
- Mock USDA client for nutrition tests

#### SDK Tests (~3158 lines in `sdk/test/`)
**Location**: `sdk/test/{unit,integration,e2e}/`

**Coverage**:
- ✅ SDK stdio bridge (Claude Desktop simulation)
- ✅ OAuth 2.0 client registration (RFC 7591)
- ✅ Token management (storage, refresh, validation)
- ✅ Provider integrations (Strava, Garmin, Fitbit)
- ✅ MCP spec compliance (tools/list, initialize, call_tool)

**Key Infrastructure** (`sdk/test/helpers/`):
- Server startup/cleanup
- Mock MCP client
- Token generation
- Keychain cleanup
- Test fixtures for providers

#### Type Generation Framework
**Location**: `scripts/generate-sdk-types.js`

**Capabilities**:
- Fetches tool schemas from server (`tools/list`)
- Converts JSON schemas to TypeScript interfaces
- Generates ~450 lines of type definitions
- Output: `sdk/src/types.ts` (45+ tool interfaces)

**Usage**:
```bash
cargo run --bin pierre-mcp-server  # Start server
cd sdk && npm run generate-types   # Generate types
```

### Critical Gap Identified

**Missing Coverage**:
❌ **Combined MCP Protocol HTTP + Multi-tenant + SDK integration tests**

**Specific Gaps**:
1. No tests that exercise **both** HTTP protocol AND SDK bridge for multi-tenant scenarios
2. No validation that type generation works correctly across multiple tenants
3. No end-to-end tests simulating multiple MCP clients (different tenants) simultaneously
4. No tests validating tenant isolation at the MCP protocol level (not just database)

**Why This Matters**:
- MCP clients connect via HTTP OR stdio (SDK)
- Multi-tenancy must work correctly for BOTH transport modes
- Type generation must produce schemas that work for all tenants
- Tenant isolation must be enforced at protocol level, not just database

## Proposed Solution

### Architecture: Hybrid Test Module

Create a **new test module** that bridges Rust and SDK tests:

```
tests/
├── mcp_multitenant_sdk_e2e_test.rs  ← NEW: Rust coordinator
│   └── Spawns SDK processes, validates HTTP responses
└── common.rs  ← EXTENDED: Add SDK spawn helpers

sdk/test/
├── e2e-multitenant/  ← NEW: SDK multi-tenant scenarios
│   ├── concurrent-tenants.test.js
│   ├── tenant-isolation.test.js
│   └── type-consistency.test.js
└── helpers/
    ├── multitenant-setup.js  ← NEW: Multi-tenant helpers
    └── rust-server-bridge.js  ← NEW: Rust test coordination
```

### Test Strategy: Three Layers

#### Layer 1: Rust Coordinator (New Test File)
**File**: `tests/mcp_multitenant_sdk_e2e_test.rs`

**Responsibilities**:
- Start Pierre MCP server with test database
- Create multiple users/tenants (reuse `tests/common.rs`)
- Generate JWT tokens for each tenant
- Spawn SDK bridge processes (one per tenant)
- Send HTTP requests to server directly
- Validate responses from both HTTP and SDK
- Clean up all resources

**Example Test**:
```rust
#[tokio::test]
async fn test_multitenant_sdk_http_isolation() -> Result<()> {
    // Setup: Create server with two tenants
    let resources = common::create_test_server_resources().await?;
    let (tenant1_user_id, tenant1_user) =
        common::create_test_user_with_email(&resources.database, "tenant1@example.com").await?;
    let (tenant2_user_id, tenant2_user) =
        common::create_test_user_with_email(&resources.database, "tenant2@example.com").await?;

    // Generate tokens
    let tenant1_token = resources.auth_manager.generate_token(&tenant1_user, &resources.jwks_manager)?;
    let tenant2_token = resources.auth_manager.generate_token(&tenant2_user, &resources.jwks_manager)?;

    // Spawn SDK bridge for Tenant 1
    let sdk1 = spawn_sdk_bridge(&tenant1_token).await?;

    // Spawn SDK bridge for Tenant 2
    let sdk2 = spawn_sdk_bridge(&tenant2_token).await?;

    // Test 1: Tenant 1 calls get_activities via SDK
    sdk1.send_mcp_request("get_activities", json!({"limit": 10})).await?;

    // Test 2: Tenant 2 calls get_activities via HTTP
    let http_response = send_http_mcp_request(
        "http://localhost:8081/mcp",
        "get_activities",
        json!({"limit": 10}),
        &tenant2_token
    ).await?;

    // Test 3: Validate isolation (Tenant 1 cannot see Tenant 2 data)
    // ... validation logic ...

    Ok(())
}
```

**Key Features**:
- Reuses `tests/common.rs` helpers (no duplication)
- Tests both HTTP and SDK transports
- Validates multi-tenant isolation
- Uses existing `ServerResources` pattern

#### Layer 2: SDK Multi-Tenant Tests (New SDK Tests)
**Directory**: `sdk/test/e2e-multitenant/`

**Test Files**:

1. **`concurrent-tenants.test.js`**:
   - Multiple SDK clients connecting simultaneously
   - Different tenants calling same tools
   - Validates no cross-tenant data leakage

2. **`tenant-isolation.test.js`**:
   - Tenant A creates activity
   - Tenant B cannot access Tenant A's activity
   - Validates 403/404 responses for unauthorized access

3. **`type-consistency.test.js`**:
   - Fetch tool schemas for multiple tenants
   - Validate that schemas are identical
   - Ensure type generation works for all tenants

**Example Test** (`concurrent-tenants.test.js`):
```javascript
describe('Multi-Tenant Concurrent Access', () => {
  test('Multiple tenants call get_activities simultaneously', async () => {
    const { tenant1Client, tenant2Client } = await setupMultiTenantClients();

    // Both tenants call get_activities at the same time
    const [tenant1Response, tenant2Response] = await Promise.all([
      tenant1Client.callTool('get_activities', { limit: 10 }),
      tenant2Client.callTool('get_activities', { limit: 10 })
    ]);

    // Validate each tenant only sees their own activities
    expect(tenant1Response.content).toHaveLength(10);
    expect(tenant2Response.content).toHaveLength(10);

    // Validate no overlap in activity IDs
    const tenant1Ids = tenant1Response.content.map(a => a.id);
    const tenant2Ids = tenant2Response.content.map(a => a.id);
    expect(tenant1Ids).not.toEqual(expect.arrayContaining(tenant2Ids));
  });
});
```

**Key Features**:
- Reuses `sdk/test/helpers/` utilities
- Extends existing SDK test patterns
- Focuses on multi-tenant scenarios

#### Layer 3: Type Generation Validation (New Test)
**File**: `tests/mcp_type_generation_multitenant_test.rs`

**Purpose**: Validate that type generation produces consistent schemas across tenants

**Test Flow**:
1. Create 3 tenants with different configurations
2. For each tenant, call `tools/list` via HTTP
3. Extract tool schemas from responses
4. Compare schemas (should be identical)
5. Validate that generated TypeScript types are tenant-agnostic

**Example Test**:
```rust
#[tokio::test]
async fn test_type_schemas_identical_across_tenants() -> Result<()> {
    let resources = common::create_test_server_resources().await?;

    // Create 3 tenants
    let tenants = vec![
        create_test_tenant(&resources, "tenant1@example.com").await?,
        create_test_tenant(&resources, "tenant2@example.com").await?,
        create_test_tenant(&resources, "tenant3@example.com").await?,
    ];

    // Fetch tool schemas for each tenant
    let schemas: Vec<Value> = futures::future::try_join_all(
        tenants.iter().map(|(user, token)| {
            fetch_tools_list_schema(&resources, token)
        })
    ).await?;

    // All schemas should be identical
    assert_eq!(schemas[0], schemas[1]);
    assert_eq!(schemas[1], schemas[2]);

    // Validate schema structure matches type generation expectations
    validate_schema_structure(&schemas[0])?;

    Ok(())
}
```

### Test Scenarios

#### Scenario 1: Concurrent Multi-Tenant Tool Calls
**Objective**: Validate that multiple tenants can call MCP tools simultaneously without interference

**Test Steps**:
1. Create 3 tenants (T1, T2, T3)
2. Spawn 3 SDK clients (one per tenant)
3. Simultaneously call `get_activities` for all tenants
4. Validate:
   - Each tenant receives only their activities
   - No cross-tenant data leakage
   - Response times are consistent
   - No race conditions or deadlocks

**Expected Outcome**: All tenants receive correct, isolated data

#### Scenario 2: HTTP vs SDK Transport Parity
**Objective**: Validate that HTTP and SDK transports produce identical results

**Test Steps**:
1. Create 1 tenant
2. Call `get_athlete` via HTTP (direct server request)
3. Call `get_athlete` via SDK (stdio bridge)
4. Compare responses (should be identical)
5. Repeat for 10+ tools

**Expected Outcome**: HTTP and SDK responses are identical for all tools

#### Scenario 3: Tenant Isolation at Protocol Level
**Objective**: Validate that tenant isolation is enforced at MCP protocol level

**Test Steps**:
1. Create 2 tenants (T1, T2)
2. T1 creates activity via `create_activity` (future tool)
3. T1 retrieves activity via `get_activity` with activity ID
4. T2 attempts to retrieve T1's activity using same activity ID
5. Validate:
   - T1 succeeds (200 OK)
   - T2 fails (403 Forbidden or 404 Not Found)

**Expected Outcome**: T2 cannot access T1's activity

#### Scenario 4: Type Generation Consistency
**Objective**: Validate that type generation works correctly for all tenants

**Test Steps**:
1. Create 5 tenants with different configurations
2. For each tenant, call `tools/list` via HTTP
3. Extract tool schemas from responses
4. Compare schemas (should be identical)
5. Run type generation script
6. Validate generated TypeScript types match schemas

**Expected Outcome**: Schemas are identical, type generation succeeds

#### Scenario 5: Rate Limiting Per Tenant
**Objective**: Validate that rate limiting is enforced per tenant

**Test Steps**:
1. Create 2 tenants (T1=free tier, T2=professional tier)
2. T1 makes 100 requests/min (exceeds free tier limit)
3. T2 makes 100 requests/min (within professional tier limit)
4. Validate:
   - T1 gets rate limited (429 Too Many Requests)
   - T2 succeeds (200 OK)
   - T1's rate limit does not affect T2

**Expected Outcome**: Rate limiting is isolated per tenant

#### Scenario 6: OAuth Flow Multi-Tenant
**Objective**: Validate OAuth flows work correctly for multiple tenants

**Test Steps**:
1. Create 2 tenants (T1, T2)
2. T1 connects to Strava via SDK
3. T2 connects to Strava via SDK
4. Validate:
   - Each tenant has separate OAuth credentials
   - T1's tokens are isolated from T2's tokens
   - Token refresh works independently for each tenant

**Expected Outcome**: OAuth tokens are isolated per tenant

### Infrastructure Extensions

#### New Helpers in `tests/common.rs`

```rust
/// Spawn SDK bridge process for testing
pub async fn spawn_sdk_bridge(jwt_token: &str) -> Result<SdkBridgeHandle> {
    let sdk_path = Path::new("./sdk/dist/cli.js");
    let child = Command::new("node")
        .arg(sdk_path)
        .arg("--server")
        .arg("http://localhost:8081")
        .arg("--token")
        .arg(jwt_token)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    Ok(SdkBridgeHandle { child })
}

/// Send HTTP MCP request directly to server
pub async fn send_http_mcp_request(
    url: &str,
    method: &str,
    params: Value,
    jwt_token: &str,
) -> Result<Value> {
    let client = reqwest::Client::new();
    let response = client
        .post(url)
        .header("Authorization", format!("Bearer {}", jwt_token))
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params
        }))
        .send()
        .await?;

    let body: Value = response.json().await?;
    Ok(body)
}

/// Create test tenant with user and token
pub async fn create_test_tenant(
    resources: &ServerResources,
    email: &str,
) -> Result<(User, String)> {
    let (user_id, user) = create_test_user_with_email(&resources.database, email).await?;
    let token = resources.auth_manager.generate_token(&user, &resources.jwks_manager)?;
    Ok((user, token))
}
```

#### New Helpers in `sdk/test/helpers/multitenant-setup.js`

```javascript
/**
 * Setup multiple MCP clients for multi-tenant testing
 */
async function setupMultiTenantClients(numTenants = 2) {
  const clients = [];

  for (let i = 0; i < numTenants; i++) {
    const email = `tenant${i + 1}@example.com`;
    const { user, token } = await createTenantViaRustBridge(email);

    const client = new MockMCPClient('node', [
      BRIDGE_PATH,
      '--server',
      SERVER_URL,
      '--token',
      token
    ]);

    await client.start();
    await client.send(MCPMessages.initialize);

    clients.push({ user, token, client });
  }

  return clients;
}

/**
 * Create tenant via Rust server bridge
 */
async function createTenantViaRustBridge(email) {
  // Call Rust test server to create tenant
  // This ensures consistency with Rust test setup
  const response = await fetch('http://localhost:8081/test/create-tenant', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ email })
  });

  return await response.json();
}

module.exports = {
  setupMultiTenantClients,
  createTenantViaRustBridge
};
```

### Implementation Plan

#### Phase 1: Foundation (Week 1)
**Goal**: Establish infrastructure for hybrid testing

**Tasks**:
1. ✅ Analyze existing test framework
2. ⬜ Add `spawn_sdk_bridge` helper to `tests/common.rs`
3. ⬜ Add `send_http_mcp_request` helper to `tests/common.rs`
4. ⬜ Add `create_test_tenant` helper to `tests/common.rs`
5. ⬜ Create `sdk/test/helpers/multitenant-setup.js`
6. ⬜ Create `sdk/test/helpers/rust-server-bridge.js`

**Validation**:
- Helpers compile and pass basic smoke tests
- SDK bridge can be spawned from Rust tests
- HTTP requests work from Rust tests

#### Phase 2: Rust Coordinator Tests (Week 2)
**Goal**: Implement Rust-side multi-tenant tests

**Tasks**:
1. ⬜ Create `tests/mcp_multitenant_sdk_e2e_test.rs`
2. ⬜ Implement Scenario 1: Concurrent Multi-Tenant Tool Calls
3. ⬜ Implement Scenario 2: HTTP vs SDK Transport Parity
4. ⬜ Implement Scenario 3: Tenant Isolation at Protocol Level
5. ⬜ Implement Scenario 5: Rate Limiting Per Tenant

**Validation**:
- All tests pass in isolation
- All tests pass when run concurrently (`cargo test --test mcp_multitenant_sdk_e2e_test`)
- No resource leaks (check with `lsof` during test runs)

#### Phase 3: SDK Multi-Tenant Tests (Week 3)
**Goal**: Implement SDK-side multi-tenant tests

**Tasks**:
1. ⬜ Create `sdk/test/e2e-multitenant/` directory
2. ⬜ Implement `concurrent-tenants.test.js` (Scenario 1)
3. ⬜ Implement `tenant-isolation.test.js` (Scenario 3)
4. ⬜ Implement `type-consistency.test.js` (Scenario 4)
5. ⬜ Implement OAuth multi-tenant test (Scenario 6)

**Validation**:
- All SDK tests pass (`npm run test:e2e`)
- Tests work with both SQLite and PostgreSQL backends
- Tests clean up resources properly

#### Phase 4: Type Generation Validation (Week 4)
**Goal**: Validate type generation for multi-tenant scenarios

**Tasks**:
1. ⬜ Create `tests/mcp_type_generation_multitenant_test.rs`
2. ⬜ Implement schema consistency tests
3. ⬜ Integrate with CI/CD pipeline
4. ⬜ Add documentation for type generation workflow

**Validation**:
- Type generation produces identical schemas for all tenants
- Generated TypeScript types are valid
- CI/CD pipeline runs type generation checks

#### Phase 5: CI/CD Integration (Week 5)
**Goal**: Integrate new tests into CI/CD pipeline

**Tasks**:
1. ⬜ Add new test suite to `.github/workflows/ci.yml`
2. ⬜ Add new test suite to `.github/workflows/sdk-tests.yml`
3. ⬜ Configure test parallelization
4. ⬜ Add test coverage reporting

**Validation**:
- All tests pass in CI/CD
- Test execution time < 5 minutes
- Coverage reports show improvement

### Success Metrics

**Quantitative Metrics**:
- **Test Coverage**: Increase from 1400 to 1500+ tests
- **Multi-Tenant Coverage**: 100% of MCP tools tested in multi-tenant mode
- **Type Consistency**: 100% schema consistency across tenants
- **Execution Time**: New tests complete in < 2 minutes

**Qualitative Metrics**:
- **No Duplication**: Reuse >80% of existing test infrastructure
- **Maintainability**: New tests follow existing patterns
- **Documentation**: All new helpers have doc comments
- **CI/CD Integration**: Tests run automatically on every PR

### Risk Mitigation

**Risk 1: SDK Process Spawning Complexity**
- **Mitigation**: Use battle-tested process spawning libraries (`tokio::process`)
- **Fallback**: Implement timeout and cleanup mechanisms

**Risk 2: Test Flakiness (Timing Issues)**
- **Mitigation**: Use deterministic waits, not `sleep()`
- **Fallback**: Implement retry logic with exponential backoff

**Risk 3: Resource Leaks (Database/Processes)**
- **Mitigation**: Use RAII pattern for all resources
- **Fallback**: Add explicit cleanup in `Drop` implementations

**Risk 4: CI/CD Performance Impact**
- **Mitigation**: Run tests in parallel where possible
- **Fallback**: Use test sharding for large test suites

### Maintenance Plan

**Monthly**:
- Review test execution times
- Identify and fix flaky tests
- Update test data/fixtures

**Quarterly**:
- Review test coverage metrics
- Refactor redundant tests
- Update documentation

**Annually**:
- Audit entire test suite for duplication
- Update to latest testing best practices
- Review CI/CD pipeline efficiency

## Conclusion

This plan provides a comprehensive strategy for adding **MCP Protocol HTTP + Multi-tenant + SDK end-to-end tests** without duplicating the existing test framework. By leveraging existing infrastructure (shared helpers, type generation, `ServerResources` pattern) and creating targeted hybrid tests, we can achieve:

1. **100% multi-tenant coverage** for MCP protocol
2. **No duplication** of existing test infrastructure
3. **Fast execution** (<5 minutes in CI/CD)
4. **High maintainability** (follows existing patterns)

The phased implementation approach (5 weeks) ensures steady progress with validation at each milestone.

## Appendix: Test File Structure

```
pierre_mcp_server/
├── tests/
│   ├── common.rs                              # EXTENDED: Add SDK spawn helpers
│   ├── mcp_multitenant_sdk_e2e_test.rs       # NEW: Rust coordinator tests
│   ├── mcp_type_generation_multitenant_test.rs # NEW: Type validation tests
│   └── [140 existing test files]             # UNCHANGED
├── sdk/
│   ├── test/
│   │   ├── e2e-multitenant/                  # NEW: Multi-tenant SDK tests
│   │   │   ├── concurrent-tenants.test.js
│   │   │   ├── tenant-isolation.test.js
│   │   │   └── type-consistency.test.js
│   │   ├── helpers/
│   │   │   ├── multitenant-setup.js          # NEW: Multi-tenant helpers
│   │   │   ├── rust-server-bridge.js         # NEW: Rust coordination
│   │   │   └── [existing helpers]            # UNCHANGED
│   │   └── [existing tests]                  # UNCHANGED
│   └── src/types.ts                          # UNCHANGED (auto-generated)
├── scripts/
│   └── generate-sdk-types.js                 # UNCHANGED
└── .github/workflows/
    ├── ci.yml                                # UPDATED: Add new test suite
    └── sdk-tests.yml                         # UPDATED: Add multi-tenant tests
```

**Total New Files**: 8
**Total Modified Files**: 4
**Total Lines of New Test Code**: ~2000 lines
**Estimated Test Count Increase**: ~100-150 tests
