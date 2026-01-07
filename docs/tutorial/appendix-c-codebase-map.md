<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Appendix C: Pierre Codebase Map

Quick reference for navigating the Pierre codebase.

## Core Modules

- **src/lib.rs**: Module declarations (45 modules)
- **src/bin/pierre-mcp-server.rs**: Binary entry point (server startup)
- **src/config/**: Environment configuration
- **src/errors.rs**: Error types with `thiserror`

## Authentication & Security

- **src/auth.rs**: JWT authentication and validation
- **src/key_management.rs**: MEK/DEK two-tier key management
- **src/admin/jwks.rs**: JWKS manager for RSA keys
- **src/crypto/keys.rs**: Ed25519 key generation for A2A
- **src/middleware/auth.rs**: MCP authentication middleware

## Dependency Injection

- **src/context/**: Focused context DI system
  - **server.rs**: ServerContext composing all contexts
  - **auth.rs**: AuthContext (auth_manager, JWT, JWKS)
  - **data.rs**: DataContext (database, provider_registry)
  - **config.rs**: ConfigContext (config, tenant OAuth)
  - **notification.rs**: NotificationContext (websocket, OAuth notifications)

## Database

- **src/database_plugins/**: Database abstraction layer
  - **factory.rs**: Database trait and factory pattern
  - **sqlite.rs**: SQLite implementation
  - **postgres.rs**: PostgreSQL implementation

## MCP Protocol

- **src/jsonrpc/**: JSON-RPC 2.0 foundation
- **src/mcp/protocol.rs**: MCP request handlers
- **src/mcp/schema.rs**: Tool schemas (47 tools)
- **src/mcp/tool_handlers.rs**: Tool execution logic
- **src/mcp/transport_manager.rs**: Transport layer coordination

## OAuth & Providers

- **src/oauth2_server/**: OAuth 2.0 server (RFC 7591)
- **src/oauth2_client/**: OAuth 2.0 client for fitness providers
- **src/providers/core.rs**: FitnessProvider trait
- **src/providers/strava.rs**: Strava API integration
- **src/providers/garmin_provider.rs**: Garmin API integration
- **src/providers/fitbit.rs**: Fitbit API integration
- **src/providers/whoop_provider.rs**: WHOOP API integration
- **src/providers/terra_provider.rs**: Terra API integration (150+ wearables)

## Intelligence Algorithms

- **src/intelligence/algorithms/tss.rs**: Training Stress Score
- **src/intelligence/algorithms/training_load.rs**: CTL/ATL/TSB
- **src/intelligence/algorithms/vo2max.rs**: VO2 max estimation
- **src/intelligence/algorithms/ftp.rs**: FTP detection
- **src/intelligence/performance_analyzer.rs**: Activity analysis

## A2A Protocol

- **src/a2a/protocol.rs**: A2A message handling
- **src/a2a/auth.rs**: A2A authentication
- **src/a2a/agent_card.rs**: Capability discovery
- **src/a2a/client.rs**: A2A client implementation
- **src/a2a_routes.rs**: HTTP endpoints for A2A protocol

## Output Formatters

- **src/formatters/mod.rs**: Output format abstraction layer
  - **OutputFormat**: Enum for JSON (default) or TOON format selection
  - **format_output()**: Serialize data to selected format
  - **TOON**: Token-Oriented Object Notation (~40% token reduction for LLMs)

## API Key Routes

- **src/api_key_routes.rs**: HTTP endpoints for API key management
  - Trial key requests
  - API key status and listing
  - User self-service key operations

## SDK (TypeScript)

- **sdk/src/bridge.ts**: SDK bridge (stdio ↔ HTTP)
- **sdk/src/types.ts**: Generated tool types (47 interfaces)
- **sdk/src/secure-storage.ts**: OS keychain integration
- **sdk/src/cli.ts**: CLI wrapper for MCP hosts

## Frontend Admin Dashboard (React/TypeScript)

- **frontend/src/App.tsx**: Main application component
- **frontend/src/services/api.ts**: Axios API client with CSRF handling
- **frontend/src/contexts/**: React contexts
  - **AuthContext.tsx**: Authentication state management
  - **WebSocketContext.ts**: WebSocket connection context
  - **WebSocketProvider.tsx**: Real-time updates provider
- **frontend/src/hooks/**: Custom React hooks
  - **useAuth.ts**: Authentication hook
  - **useWebSocket.ts**: WebSocket connection hook
- **frontend/src/components/**: UI components (20+)
  - **Dashboard.tsx**: Main dashboard view
  - **UserManagement.tsx**: User approval and management
  - **A2AManagement.tsx**: Agent-to-Agent monitoring
  - **ApiKeyList.tsx**: API key management
  - **UsageAnalytics.tsx**: Request patterns and metrics
  - **RequestMonitor.tsx**: Real-time request monitoring
  - **ToolUsageBreakdown.tsx**: Tool usage visualization

## Templates (OAuth HTML)

- **templates/oauth_success.html**: OAuth success page
- **templates/oauth_error.html**: OAuth error page
- **templates/oauth_login.html**: OAuth login page
- **templates/pierre-logo.svg**: Brand assets

## Testing

- **tests/helpers/synthetic_data.rs**: Deterministic test data
- **tests/helpers/synthetic_provider.rs**: In-memory provider
- **tests/integration/**: Integration tests
- **tests/e2e/**: End-to-end tests

## Scripts

See [scripts/README.md](../../scripts/README.md) for comprehensive documentation.

**Key scripts by category**:

### Development
- **scripts/dev-start.sh**: Start development environment (backend + frontend)
- **scripts/fresh-start.sh**: Clean database reset
- **scripts/setup-git-hooks.sh**: Install pre-commit, commit-msg, pre-push hooks

### Validation & Testing
- **scripts/architectural-validation.sh**: Custom pattern validation (anyhow!, DI, etc.)
- **scripts/lint-and-test.sh**: Full CI validation suite
- **scripts/pre-push-tests.sh**: Critical path tests (5-10 minutes)
- **scripts/smoke-test.sh**: Quick validation (2-3 minutes)
- **scripts/category-test-runner.sh**: Run tests by category (mcp, oauth, security)

### SDK & Type Generation
- **scripts/generate-sdk-types.js**: Auto-generate TypeScript types from server schemas

### Deployment
- **scripts/deploy.sh**: Docker Compose deployment (dev/prod)

### Configuration
- **scripts/validation-patterns.toml**: Architectural validation rules

## Key File Locations

| Feature | File Path |
|---------|-----------|
| Tool registry | src/mcp/schema.rs:499 |
| JWT auth | src/auth.rs |
| OAuth server | src/oauth2_server/endpoints.rs |
| Provider trait | src/providers/core.rs:52 |
| TSS calculation | src/intelligence/algorithms/tss.rs |
| Synthetic data | tests/helpers/synthetic_data.rs |
| SDK bridge | sdk/src/bridge.ts |
| Frontend dashboard | frontend/src/components/Dashboard.tsx |
| OAuth templates | templates/oauth_success.html |
| Architectural validation | scripts/architectural-validation.sh |
| Full CI suite | scripts/lint-and-test.sh |

## Key Takeaways

1. **Module organization**: 45 modules in src/lib.rs.
2. **Database abstraction**: Factory pattern with SQLite/PostgreSQL implementations.
3. **MCP protocol**: JSON-RPC foundation + MCP-specific handlers.
4. **OAuth dual role**: Server (for MCP clients) + client (for fitness providers).
5. **Intelligence**: Algorithm modules in src/intelligence/algorithms/.
6. **Testing**: Synthetic data for deterministic tests.
7. **SDK bridge**: TypeScript SDK bridges MCP hosts to Pierre server (stdio ↔ HTTP).
8. **Admin dashboard**: React/TypeScript frontend for server management.
9. **Templates**: HTML templates for OAuth flows with brand styling.
10. **Scripts**: Comprehensive tooling for validation, testing, and deployment.
