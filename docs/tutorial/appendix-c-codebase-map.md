# appendix C: pierre codebase map

Quick reference for navigating the Pierre codebase.

## core modules

- **src/lib.rs**: Module declarations (40+ modules)
- **src/main.rs**: Binary entry point (server startup)
- **src/config/**: Environment configuration
- **src/errors.rs**: Error types with `thiserror`

## authentication & security

- **src/auth.rs**: JWT authentication and validation
- **src/key_management.rs**: MEK/DEK two-tier key management
- **src/admin/jwks.rs**: JWKS manager for RSA keys
- **src/crypto/keys.rs**: Ed25519 key generation for A2A
- **src/middleware/auth.rs**: MCP authentication middleware

## database

- **src/database_plugins/**: Database abstraction layer
  - **factory.rs**: Database trait and factory pattern
  - **sqlite.rs**: SQLite implementation
  - **postgres.rs**: PostgreSQL implementation

## MCP protocol

- **src/jsonrpc/**: JSON-RPC 2.0 foundation
- **src/mcp/protocol.rs**: MCP request handlers
- **src/mcp/schema.rs**: Tool schemas (45 tools)
- **src/mcp/tool_handlers.rs**: Tool execution logic
- **src/mcp/transport_manager.rs**: Transport layer coordination

## OAuth & providers

- **src/oauth2_server/**: OAuth 2.0 server (RFC 7591)
- **src/oauth2_client/**: OAuth 2.0 client for fitness providers
- **src/providers/core.rs**: FitnessProvider trait
- **src/providers/strava.rs**: Strava API integration
- **src/providers/fitbit.rs**: Fitbit API integration

## intelligence algorithms

- **src/intelligence/algorithms/tss.rs**: Training Stress Score
- **src/intelligence/algorithms/training_load.rs**: CTL/ATL/TSB
- **src/intelligence/algorithms/vo2max.rs**: VO2 max estimation
- **src/intelligence/algorithms/ftp.rs**: FTP detection
- **src/intelligence/performance_analyzer.rs**: Activity analysis

## A2A protocol

- **src/a2a/protocol.rs**: A2A message handling
- **src/a2a/auth.rs**: A2A authentication
- **src/a2a/agent_card.rs**: Capability discovery
- **src/a2a/client.rs**: A2A client implementation

## SDK (TypeScript)

- **sdk/src/bridge.ts**: SDK bridge (stdio ↔ HTTP)
- **sdk/src/types.ts**: Generated tool types
- **sdk/src/secure-storage.ts**: OS keychain integration

## testing

- **tests/helpers/synthetic_data.rs**: Deterministic test data
- **tests/helpers/synthetic_provider.rs**: In-memory provider
- **tests/integration/**: Integration tests
- **tests/e2e/**: End-to-end tests

## scripts

- **scripts/generate-sdk-types.js**: Tools-to-types generator

## key file locations

| Feature | File Path |
|---------|-----------|
| Tool registry | src/mcp/schema.rs:499 |
| JWT auth | src/auth.rs |
| OAuth server | src/oauth2_server/endpoints.rs |
| Provider trait | src/providers/core.rs:52 |
| TSS calculation | src/intelligence/algorithms/tss.rs |
| Synthetic data | tests/helpers/synthetic_data.rs |

## key takeaways

1. **Module organization**: 40+ modules in src/lib.rs.
2. **Database abstraction**: Factory pattern with SQLite/PostgreSQL implementations.
3. **MCP protocol**: JSON-RPC foundation + MCP-specific handlers.
4. **OAuth dual role**: Server (for MCP clients) + client (for fitness providers).
5. **Intelligence**: Algorithm modules in src/intelligence/algorithms/.
6. **Testing**: Synthetic data for deterministic tests.
