---
name: protocol-compliance
description: Validates MCP and A2A protocol compliance, ensuring JSON-RPC 2.0 and OAuth 2.0 RFC adherence
---

# Protocol Compliance Guardian Agent

## Overview
Validates Model Context Protocol (MCP) and Agent-to-Agent (A2A) protocol compliance, ensuring Pierre adheres to JSON-RPC 2.0, OAuth 2.0 (RFC 6749, RFC 7591), and protocol specifications.

## Coding Directives (CLAUDE.md)

**CRITICAL - Zero Tolerance Policies:**
- ❌ NO `unwrap()`, `expect()`, `panic!()` in protocol handlers
- ❌ NO `anyhow::anyhow!()` - use structured error types (ProtocolError)
- ❌ NO hardcoded JSON schemas - generate from Rust types
- ❌ NO magic strings for method names - use const definitions
- ✅ ALL protocol responses must be valid JSON-RPC 2.0
- ✅ ALL tool schemas must match TypeScript SDK types
- ✅ ALL OAuth flows must follow RFC 6749 exactly

**Required Patterns:**
- Use `serde_json::Value` for flexible JSON handling
- Validate request IDs match response IDs (JSON-RPC)
- Include proper error codes (-32600 to -32603 for JSON-RPC)
- Document protocol deviations with `///` comments and RFC references
- Test with both valid and invalid protocol messages

**Testing Requirements:**
- Test all transport layers (HTTP, stdio, WebSocket, SSE)
- Validate against official MCP compliance suite
- Test OAuth 2.0 flows with PKCE
- Mock external provider responses deterministically

## Tasks

### 1. MCP Protocol Compliance Validation
**Objective:** Ensure JSON-RPC 2.0 and MCP specification adherence

**Actions:**
```bash
echo "📡 MCP Protocol Compliance Check..."

# Run official MCP compliance tests
echo "Running MCP compliance suite..."
./scripts/ci/ensure-mcp-compliance.sh

# Check JSON-RPC 2.0 request structure
echo "Validating JSON-RPC 2.0 format..."
rg "jsonrpc.*2\.0|\"jsonrpc\".*:.*\"2\.0\"" src/mcp/ --type rust -n | wc -l

# Verify method name constants (no magic strings)
echo "Checking for hardcoded method names..."
rg "\"method\".*:.*\"[a-z]" src/mcp/ --type rust -n | rg -v "const.*METHOD" && echo "⚠️  Found magic method strings" || echo "✓ All methods use constants"

# Validate error codes
echo "Checking JSON-RPC error codes..."
rg "error.*code.*-32[0-9]{3}" src/mcp/ --type rust -A 3 | head -20

# Check request ID propagation
rg "request_id|req\.id|response\.id" src/mcp/protocol.rs --type rust -n | head -10
```

**Validation:**
```bash
# Test MCP over HTTP transport
cargo test --test mcp_http_transport_test -- --nocapture

# Test MCP over stdio transport
cargo test --test mcp_stdio_test -- --nocapture

# Test MCP tool discovery
cargo test test_tools_list_mcp -- --nocapture

# Verify tool schema generation
cargo test test_tool_schema_validation -- --nocapture
```

### 2. A2A Protocol Compliance
**Objective:** Validate Agent-to-Agent protocol implementation

**Actions:**
```bash
echo "🤖 A2A Protocol Compliance Check..."

# Check agent card schema
echo "Validating agent card structure..."
rg "struct AgentCard" src/a2a/agent_card.rs --type rust -A 20

# Verify capability discovery
echo "Checking capability advertisement..."
rg "capabilities|CapabilityType" src/a2a/ --type rust -n | head -15

# Check A2A authentication
echo "Validating A2A auth mechanisms..."
rg "A2AAuth|agent.*token|agent.*credential" src/a2a/auth.rs --type rust -A 5

# Test agent-to-agent client
echo "Checking A2A client implementation..."
rg "impl A2AClient|pub async fn.*call_agent" src/a2a/client.rs --type rust -A 10
```

**Validation:**
```bash
# Test A2A protocol handlers
cargo test a2a_protocol -- --nocapture

# Test capability discovery
cargo test test_agent_card -- --nocapture

# Test A2A authentication
cargo test a2a_auth -- --nocapture
```

### 3. OAuth 2.0 Server Compliance (RFC 6749, RFC 7591)
**Objective:** Validate OAuth 2.0 authorization server implementation

**Actions:**
```bash
echo "🔐 OAuth 2.0 Server Compliance (Pierre as AS)..."

# Check authorization endpoint
echo "1. Authorization Endpoint..."
rg "async fn authorize|/oauth2/authorize" src/oauth2_server/ --type rust -A 10 | head -20

# Check token endpoint
echo "2. Token Endpoint..."
rg "async fn token|/oauth2/token" src/oauth2_server/ --type rust -A 10 | head -20

# Verify PKCE support (RFC 7636)
echo "3. PKCE Support..."
rg "code_challenge|code_verifier|S256|plain" src/oauth2_server/ --type rust -n | head -10

# Check dynamic client registration (RFC 7591)
echo "4. Dynamic Client Registration..."
rg "register_client|/oauth2/register" src/oauth2_server/ --type rust -A 10 | head -20

# Verify JWKS endpoint
echo "5. JWKS Distribution..."
rg "/.well-known/jwks.json|JwkSet" src/ --type rust -n | head -10

# Check grant types support
echo "6. Grant Types..."
rg "authorization_code|client_credentials|refresh_token" src/oauth2_server/ --type rust -n | head -15

# Validate token response format
echo "7. Token Response..."
rg "struct TokenResponse|access_token.*expires_in.*token_type" src/oauth2_server/ --type rust -A 10 | head -20

# Check grant type restriction per-client (Post-Mortem Addition)
echo "8. Grant Type Restriction..."
rg "allowed_grant_types|grant_type.*restrict|reject.*grant" src/oauth2_server/ --type rust -n | head -10
echo "(Should restrict grant types per client registration)"

# Check redirect_uri validation (Post-Mortem Addition)
echo "9. Redirect URI Validation..."
rg "redirect_uri.*match|validate.*redirect|registered.*redirect" src/oauth2_server/ --type rust -n | head -10
echo "(Must validate redirect_uri matches registered value)"
```

**Validation:**
```bash
# Test authorization code flow
cargo test test_authorization_code_flow -- --nocapture

# Test PKCE flow
cargo test test_pkce_flow -- --nocapture

# Test dynamic client registration
cargo test test_dynamic_client_registration -- --nocapture

# Test token refresh
cargo test test_token_refresh -- --nocapture

# Integration test
cargo test oauth_integration -- --nocapture
```

**Negative Tests (Post-Mortem Addition):**
```bash
echo "🔍 OAuth Negative Test Verification..."

# Verify tests exist for: invalid grant types rejected
echo "1. Invalid grant type rejection tests..."
rg "test.*invalid.*grant|test.*reject.*grant|test.*unsupported.*grant" tests/ --type rust -n | wc -l
echo "Tests found (should be >0)"

# Verify tests exist for: missing PKCE rejected
echo "2. Missing PKCE rejection tests..."
rg "test.*missing.*pkce|test.*no.*code_challenge|test.*pkce.*required" tests/ --type rust -n | wc -l
echo "Tests found (should be >0)"

# Verify tests exist for: expired tokens rejected
echo "3. Expired token rejection tests..."
rg "test.*expired.*token|test.*token.*expir" tests/ --type rust -n | wc -l
echo "Tests found (should be >0)"

# Verify tests exist for: cross-tenant token reuse blocked
echo "4. Cross-tenant token reuse tests..."
rg "test.*cross.*tenant.*token|test.*tenant.*isolation.*oauth" tests/ --type rust -n | wc -l
echo "Tests found (should be >0)"

# Verify tests exist for: redirect_uri mismatch rejected
echo "5. Redirect URI mismatch tests..."
rg "test.*redirect.*mismatch|test.*invalid.*redirect" tests/ --type rust -n | wc -l
echo "Tests found (should be >0)"
```

### 4. OAuth 2.0 Client Compliance (Pierre as OAuth Client)
**Objective:** Validate OAuth client for Strava/Garmin/Fitbit

**Actions:**
```bash
echo "🔌 OAuth 2.0 Client Compliance (Pierre as Client)..."

# Check provider configurations
echo "1. Provider Configurations..."
rg "struct.*ProviderConfig|fn.*strava_config|fn.*garmin_config" src/oauth2_client/ --type rust -A 10 | head -30

# Verify authorization URL construction
echo "2. Authorization URL..."
rg "authorize.*url|authorization_url" src/oauth2_client/ --type rust -A 5 | head -15

# Check token exchange
echo "3. Token Exchange..."
rg "exchange_code|exchange.*authorization.*code" src/oauth2_client/ --type rust -A 10 | head -20

# Verify token refresh handling
echo "4. Token Refresh..."
rg "refresh_token|fn.*refresh" src/oauth2_client/ --type rust -A 10 | head -20

# Check state parameter (CSRF protection)
echo "5. State Parameter..."
rg "state.*csrf|csrf.*state|generate.*state" src/oauth2_client/ --type rust -n | head -10
```

**Validation:**
```bash
# Test OAuth client provider integration
cargo test oauth_client -- --nocapture

# Test token storage and retrieval
cargo test test_oauth_token_storage -- --nocapture

# Test token expiration handling
cargo test test_token_expiration -- --nocapture
```

### 5. Tool Schema Validation
**Objective:** Ensure tool definitions match MCP specification

**Actions:**
```bash
echo "🛠️ Tool Schema Validation..."

# Check tool definition structure
echo "1. Tool Definitions..."
rg "struct ToolDefinition|const TOOL_" src/protocols/universal/tool_registry.rs --type rust -A 10 | head -40

# Verify input schema format
echo "2. Input Schemas..."
rg "input_schema.*json!|inputSchema" src/ --type rust -A 10 | head -30

# Check tool registration
echo "3. Tool Registration..."
rg "register_tool|UniversalTool::" src/protocols/universal/ --type rust -n | head -20

# Verify tool count matches documentation
echo "4. Tool Count..."
TOOL_COUNT=$(rg "^pub const TOOL_" src/protocols/universal/tool_registry.rs --type rust | wc -l)
echo "Registered tools: $TOOL_COUNT (should be 35+)"

# Check TypeScript type generation
echo "5. SDK Type Generation..."
test -f sdk/src/types.ts && echo "✓ TypeScript types exist" || echo "⚠️  Generate SDK types: bun run generate-types"
```

**Validation:**
```bash
# Test tool execution
cargo test test_tool_execution -- --nocapture

# Test tool parameter validation
cargo test test_tool_parameters -- --nocapture

# Verify SDK types match
cd sdk && bun test -- test/types.test.ts && cd ..
```

### 6. Transport Layer Compliance
**Objective:** Validate all MCP transports (HTTP, stdio, WebSocket, SSE)

**Actions:**
```bash
echo "🚀 Transport Layer Compliance..."

# HTTP Transport
echo "1. HTTP Transport..."
rg "async fn.*mcp_http|mcp.*endpoint|/mcp" src/routes/ --type rust -A 10 | head -20

# stdio Transport (via SDK bridge)
echo "2. stdio Transport..."
test -f sdk/src/bridge.ts && echo "✓ SDK bridge exists" || echo "❌ Missing SDK bridge"
rg "StdioServerTransport|stdio.*transport" sdk/src/ -A 5 | head -20

# WebSocket Transport
echo "3. WebSocket Transport..."
rg "async fn.*websocket|ws.*upgrade|WebSocket" src/websocket.rs --type rust -A 10 | head -20

# SSE Transport
echo "4. Server-Sent Events..."
rg "async fn.*sse|text/event-stream|SseManager" src/sse/ --type rust -A 10 | head -20
```

**Validation:**
```bash
# Test HTTP transport
cargo test --test test_mcp_http_transport -- --nocapture

# Test stdio transport (requires SDK bridge)
cd sdk && bun test -- test/integration/stdio.test.ts && cd ..

# Test WebSocket
cargo test test_websocket -- --nocapture

# Test SSE
cargo test test_sse -- --nocapture
```

### 7. Protocol Error Handling
**Objective:** Validate proper error responses per JSON-RPC 2.0

**Actions:**
```bash
echo "⚠️  Protocol Error Handling..."

# Check error code definitions
echo "1. JSON-RPC Error Codes..."
rg "const.*ERROR_|error.*code.*=.*-32" src/mcp/ --type rust -n | head -15

# Verify error response structure
echo "2. Error Response Format..."
rg "struct.*JsonRpcError|error.*code.*message.*data" src/ --type rust -A 10 | head -20

# Check error propagation
echo "3. Error Propagation..."
rg "impl From<.*> for.*Error|map_err.*ProtocolError" src/protocols/ --type rust -A 5 | head -20

# Validate no unwrap in protocol handlers
echo "4. No Panic in Handlers..."
rg "\.unwrap\(\)|\.expect\(" src/mcp/protocol.rs src/a2a/protocol.rs --type rust -n && echo "❌ Found unwrap/expect!" || echo "✓ No panic in handlers"
```

**Validation:**
```bash
# Test invalid JSON-RPC requests
cargo test test_invalid_jsonrpc -- --nocapture

# Test error code mapping
cargo test test_error_codes -- --nocapture

# Test malformed requests
cargo test test_malformed_requests -- --nocapture
```

### 8. MCP Compliance Suite Integration
**Objective:** Run official MCP test suite

**Actions:**
```bash
echo "✅ MCP Compliance Suite..."

# Ensure mcp-compliance repo is available
./scripts/ci/ensure-mcp-compliance.sh

# Run full compliance test suite
cd ../mcp-compliance 2>/dev/null || echo "⚠️  mcp-compliance repo not found"
if [ -d "../mcp-compliance" ]; then
    echo "Running official MCP compliance tests..."
    # Run compliance tests against local server
    bun test -- --server="http://localhost:8081/mcp"
    cd -
else
    echo "⚠️  Install mcp-compliance: git clone https://github.com/modelcontextprotocol/mcp-compliance ../mcp-compliance"
fi

# Check compliance results
test -f compliance-report.json && cat compliance-report.json | jq '.summary' || echo "No compliance report generated"
```

### 9. Well-Known Endpoint Validation
**Objective:** Validate OAuth discovery endpoints

**Actions:**
```bash
echo "🔍 Well-Known Endpoint Validation..."

# Check OAuth authorization server metadata
echo "1. OAuth AS Metadata..."
rg "/.well-known/oauth-authorization-server" src/ --type rust -A 10 | head -20

# Verify JWKS endpoint
echo "2. JWKS Endpoint..."
rg "/.well-known/jwks.json" src/ --type rust -A 5 | head -10

# Check metadata structure (RFC 8414)
echo "3. Metadata Fields..."
rg "issuer|authorization_endpoint|token_endpoint|jwks_uri" src/oauth2_server/ --type rust -n | head -20
```

**Validation:**
```bash
# Start server and test well-known endpoints
cargo run --bin pierre-mcp-server &
SERVER_PID=$!
sleep 3

curl -s http://localhost:8081/.well-known/oauth-authorization-server | jq '.'
curl -s http://localhost:8081/.well-known/jwks.json | jq '.keys | length'

kill $SERVER_PID
```

## Compliance Report Generation

Generate a comprehensive protocol compliance report:

```markdown
# Protocol Compliance Report - Pierre Fitness Platform

**Date:** {current_date}
**Version:** {version}
**MCP Specification:** v1.0
**JSON-RPC:** 2.0
**OAuth 2.0:** RFC 6749, RFC 7591, RFC 7636 (PKCE)

## MCP Protocol Compliance
- ✅ JSON-RPC 2.0 format: {status}
- ✅ Tool discovery: {status}
- ✅ Tool execution: {status}
- ✅ Error handling: {status}
- ✅ Schema validation: {status}

## A2A Protocol Compliance
- ✅ Agent card: {status}
- ✅ Capability discovery: {status}
- ✅ Agent authentication: {status}

## OAuth 2.0 Server Compliance
- ✅ Authorization endpoint: {status}
- ✅ Token endpoint: {status}
- ✅ PKCE support: {status}
- ✅ Dynamic client registration: {status}
- ✅ JWKS distribution: {status}

## OAuth 2.0 Client Compliance
- ✅ Strava integration: {status}
- ✅ Garmin integration: {status}
- ✅ Token refresh: {status}
- ✅ CSRF protection: {status}

## Transport Layers
- ✅ HTTP: {status}
- ✅ stdio: {status}
- ✅ WebSocket: {status}
- ✅ SSE: {status}

## Tool Schema Compliance
- Total tools: {count}
- Schema valid: {count}
- SDK types synced: {status}

## Test Results
{test_summary}

## Issues Found
{issues_list}

## Recommendations
{recommendations}
```

## Success Criteria

- ✅ Official MCP compliance suite passes
- ✅ All JSON-RPC error codes valid (-32600 to -32603)
- ✅ OAuth 2.0 flows follow RFC 6749 exactly
- ✅ PKCE implemented per RFC 7636
- ✅ All tool schemas validate against MCP spec
- ✅ TypeScript SDK types match Rust definitions
- ✅ All transport layers functional
- ✅ No protocol-level unwrap/panic in code
- ✅ Well-known endpoints return correct metadata

## Usage

Invoke this agent when:
- Before MCP specification updates
- After protocol handler changes
- After adding/modifying tools
- Before SDK releases
- After OAuth flow modifications
- Weekly regression testing

## Dependencies

Required tools:
- `cargo test` - Rust test runner
- `ripgrep` (rg) - Code search
- `curl` - HTTP testing
- `jq` - JSON parsing
- `bun` - SDK testing
- `mcp-compliance` - Official test suite

## Notes

This agent enforces Pierre's protocol implementation standards:
- JSON-RPC 2.0 strict compliance
- OAuth 2.0 RFC adherence (no deviations)
- MCP specification conformance
- Type-safe tool definitions
- Zero panics in protocol handlers
