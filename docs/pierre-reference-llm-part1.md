# Pierre MCP Server - Reference Part 1: Core

> Reference documentation for ChatGPT. Part 1: Overview, Architecture, Configuration.

---

# Documentation

Developer documentation for Pierre Fitness Platform.

## Quick Links

### For Users
- Getting Started - Install, configure, connect your AI assistant

### For Developers
1. Getting Started - Setup dev environment
2. Architecture - System design
3. Development Guide - Workflow, dashboard, testing
4. Contributing - Code standards, PR workflow

### For Integrators
- MCP clients: Protocols
- Web apps: Protocols
- Autonomous agents: Protocols

## Reference Documentation

### Core
- Getting Started - Installation and quick start
- Architecture - System design and components
- Protocols - MCP, OAuth2, A2A, REST protocols
- Authentication - JWT, API keys, OAuth2

### Configuration
- Configuration - Settings and algorithms
- Environment - .envrc variables reference

### OAuth
- OAuth Client - Fitness provider connections (Strava, Fitbit, Garmin, WHOOP, Terra)
- OAuth2 Server - MCP client authentication

### Development
- Development Guide - Workflow, dashboard, admin tools
- Build - Rust toolchain, cargo configuration
- CI/CD - GitHub Actions, pipelines
- Testing - Test framework, strategies
- Contributing - Development guidelines

### Methodology
- Intelligence Methodology - Sports science formulas
- Nutrition Methodology - Dietary calculations

## Scripts

Development, testing, and deployment scripts.

- Scripts Reference - 30+ scripts documented

Key scripts:
```bash
./bin/start-server.sh     # start backend
./bin/stop-server.sh      # stop backend
./bin/start-frontend.sh   # start dashboard
./scripts/fresh-start.sh  # clean database reset
./scripts/lint-and-test.sh # full CI suite
```

## Tutorial

Comprehensive Rust learning path using Pierre as the codebase.

- Tutorial Table of Contents - 25 chapters + appendices

### Learning Paths

**Quick Start** (core concepts):
1. Chapter 1 - Architecture
2. Chapter 2 - Error Handling
3. Chapter 9 - JSON-RPC
4. Chapter 10 - MCP Protocol
5. Chapter 19 - Tools Guide

**Security-Focused**:
1. Chapter 5 - Cryptographic Keys
2. Chapter 6 - JWT Authentication
3. Chapter 7 - Multi-Tenant Isolation
4. Chapter 15 - OAuth 2.0 Server

## Component Documentation

- SDK Documentation - TypeScript SDK for MCP clients
- Frontend Documentation - React dashboard
- Examples - Sample integrations

## Installation Guides

- MCP Client Installation - Claude Desktop, ChatGPT

## Additional Resources

- OpenAPI spec: `openapi.yaml`
- Main README: ../README.md

## Documentation Style

- **Concise**: Developers don't read walls of text
- **Accurate**: Verified against actual code
- **Practical**: Code examples that work
- **Capitalized**: Section headings start with capital letters

---

# Getting Started

## Prerequisites

- rust 1.91+ (matches `rust-toolchain`)
- sqlite3 (or postgresql for production)
- node 24+ (for sdk)

## Installation

```bash
git clone https://github.com/Async-IO/pierre_mcp_server.git
cd pierre_mcp_server
cargo build --release
```

Binary: `target/release/pierre-mcp-server`

## Configuration

### Using Direnv (Recommended)

```bash
brew install direnv
cd pierre_mcp_server
direnv allow
```

Edit `.envrc` for your environment. Development defaults included.

### Manual Setup

Required:
```bash
export DATABASE_URL="sqlite:./data/users.db"
export PIERRE_MASTER_ENCRYPTION_KEY="$(openssl rand -base64 32)"
```

Optional provider oauth (connect to strava/garmin/fitbit/whoop):
```bash
# local development only
export STRAVA_CLIENT_ID=your_id
export STRAVA_CLIENT_SECRET=your_secret
export STRAVA_REDIRECT_URI=http://localhost:8081/api/oauth/callback/strava  # local dev

export GARMIN_CLIENT_ID=your_key
export GARMIN_CLIENT_SECRET=your_secret
export GARMIN_REDIRECT_URI=http://localhost:8081/api/oauth/callback/garmin  # local dev

# production: use https for callback urls (required)
# export STRAVA_REDIRECT_URI=https://api.example.com/api/oauth/callback/strava
# export GARMIN_REDIRECT_URI=https://api.example.com/api/oauth/callback/garmin
```

**security**: http callback urls only for local development. Production must use https to protect authorization codes.

See environment.md for all environment variables.

## Running the Server

```bash
cargo run --bin pierre-mcp-server
```

Server starts on `http://localhost:8081`

Logs show available endpoints:
- `/health` - health check
- `/mcp` - mcp protocol endpoint
- `/oauth2/*` - oauth2 authorization server
- `/api/*` - rest api
- `/admin/*` - admin endpoints

## Create Admin User

```bash
curl -X POST http://localhost:8081/admin/setup \
  -H "Content-Type: application/json" \
  -d '{
    "email": "admin@example.com",
    "password": "SecurePass123!",
    "display_name": "Admin"
  }'
```

Response includes jwt token. Save it.

## Connect MCP Client

### Option 1: NPM Package (Recommended)

```bash
npm install -g pierre-mcp-client@next
```

Claude desktop config (`~/Library/Application Support/Claude/claude_desktop_config.json`):
```json
{
  "mcpServers": {
    "pierre": {
      "command": "npx",
      "args": ["-y", "pierre-mcp-client@next", "--server", "http://localhost:8081"]
    }
  }
}
```

### Option 2: Build From Source

```bash
cd sdk
npm install
npm run build
```

Claude desktop config:
```json
{
  "mcpServers": {
    "pierre": {
      "command": "node",
      "args": ["/absolute/path/to/sdk/dist/cli.js", "--server", "http://localhost:8081"]
    }
  }
}
```

Restart claude desktop.

## Authentication Flow

Sdk handles oauth2 automatically:
1. Registers oauth2 client with Pierre Fitness Platform (rfc 7591)
2. Opens browser for login
3. Handles callback and token exchange
4. Stores jwt token
5. Uses jwt for all mcp requests

No manual token management needed.

## Verify Connection

In claude desktop, ask:
- "connect to strava" - initiates oauth flow
- "get my last 5 activities" - fetches strava data
- "analyze my training load" - runs intelligence engine

## Available Tools

Pierre Fitness Platform exposes dozens of MCP tools:

**fitness data:**
- `get_activities` - fetch activities
- `get_athlete` - athlete profile
- `get_stats` - athlete statistics
- `analyze_activity` - detailed activity analysis

**goals:**
- `set_goal` - create fitness goal
- `suggest_goals` - ai-suggested goals
- `track_progress` - goal progress tracking
- `analyze_goal_feasibility` - feasibility analysis

**performance:**
- `calculate_metrics` - custom metrics
- `analyze_performance_trends` - trend detection
- `compare_activities` - activity comparison
- `detect_patterns` - pattern recognition
- `generate_recommendations` - training recommendations
- `analyze_training_load` - load analysis

**configuration:**
- `get_user_configuration` - current config
- `update_user_configuration` - update config
- `calculate_personalized_zones` - training zones

See tools-reference.md for complete tool documentation.

## Development Workflow

```bash
# clean start
./scripts/fresh-start.sh
cargo run --bin pierre-mcp-server &

# run complete workflow test
./scripts/complete-user-workflow.sh

# load saved credentials
source .workflow_test_env
echo $JWT_TOKEN
```

## Testing

```bash
# all tests
cargo test

# specific suite
cargo test --test mcp_multitenant_complete_test

# with output
cargo test -- --nocapture

# lint + test
./scripts/lint-and-test.sh
```

## Troubleshooting

### Server Won't Start

Check logs for:
- database connection errors → verify `DATABASE_URL`
- encryption key errors → verify `PIERRE_MASTER_ENCRYPTION_KEY`
- port conflicts → check port 8081 availability

### SDK Connection Fails

1. Verify server is running: `curl http://localhost:8081/health`
2. Check claude desktop logs: `~/Library/Logs/Claude/mcp*.log`
3. Test sdk directly: `npx pierre-mcp-client@next --server http://localhost:8081`

### OAuth2 Flow Fails

- verify redirect uri matches: server must be accessible at configured uri
- check browser console for errors
- verify provider credentials (strava_client_id, etc.)

## Next Steps

- architecture.md - system design
- protocols.md - protocol details
- authentication.md - auth guide
- configuration.md - configuration reference

---

# Architecture

Pierre Fitness Platform is a multi-protocol fitness data platform that connects AI assistants to strava, garmin, fitbit, whoop, and terra (150+ wearables). Single binary, single port (8081), multiple protocols.

## System Design

```
┌─────────────────┐
│   mcp clients   │ claude desktop, chatgpt, etc
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│   pierre sdk    │ typescript bridge (stdio → http)
│   (npm package) │
└────────┬────────┘
         │ http + oauth2
         ▼
┌─────────────────────────────────────────┐
│   Pierre Fitness Platform (rust)        │
│   port 8081 (all protocols)             │
│                                          │
│   • mcp protocol (json-rpc 2.0)        │
│   • oauth2 server (rfc 7591)           │
│   • a2a protocol (agent-to-agent)      │
│   • rest api                            │
│   • sse (real-time notifications)      │
└────────┬────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────┐
│   fitness providers (1 to x)            │
│   • strava                              │
│   • garmin                              │
│   • fitbit                              │
│   • synthetic (oauth-free dev/testing)  │
│   • custom providers (pluggable)        │
│                                          │
│   ProviderRegistry: runtime discovery   │
│   Environment config: PIERRE_*_*        │
└─────────────────────────────────────────┘
```

## Core Components

### Protocols Layer (`src/protocols/`)
- `universal/` - protocol-agnostic business logic
- shared by mcp and a2a protocols
- dozens of fitness tools (activities, analysis, goals, sleep, recovery, nutrition, configuration)

### MCP Implementation (`src/mcp/`)
- json-rpc 2.0 over http
- sse transport for streaming
- tool registry and execution

### OAuth2 Server (`src/oauth2_server/`)
- rfc 7591 dynamic client registration
- rfc 7636 pkce support
- jwt access tokens for mcp clients

### OAuth2 Client (`src/oauth2_client/`)
- pierre connects to fitness providers as oauth client
- pkce support for enhanced security
- automatic token refresh
- multi-tenant credential isolation

### Providers (`src/providers/`)
- **pluggable provider architecture**: factory pattern with runtime registration
- **feature flags**: compile-time provider selection (`provider-strava`, `provider-garmin`, `provider-fitbit`, `provider-whoop`, `provider-terra`, `provider-synthetic`)
- **service provider interface (spi)**: `ProviderDescriptor` trait for external provider registration
- **bitflags capabilities**: efficient `ProviderCapabilities` with combinators (`full_health()`, `full_fitness()`)
- **1 to x providers simultaneously**: supports strava + garmin + custom providers at once
- **provider registry**: `ProviderRegistry` manages all providers with dynamic discovery
- **environment-based config**: cloud-native configuration via `PIERRE_<PROVIDER>_*` env vars:
  - `PIERRE_STRAVA_CLIENT_ID`, `PIERRE_STRAVA_CLIENT_SECRET` (also: legacy `STRAVA_CLIENT_ID`)
  - `PIERRE_<PROVIDER>_AUTH_URL`, `PIERRE_<PROVIDER>_TOKEN_URL`, `PIERRE_<PROVIDER>_SCOPES`
  - Falls back to hardcoded defaults if env vars not set
- **shared `FitnessProvider` trait**: uniform interface for all providers
- **built-in providers**: strava, garmin, fitbit, whoop, terra (150+ wearables), synthetic (oauth-free dev/testing)
- **oauth parameters**: `OAuthParams` captures provider-specific oauth differences (scope separator, pkce)
- **dynamic discovery**: `supported_providers()` and `is_supported()` for runtime introspection
- **zero code changes**: add new providers without modifying tools or connection handlers
- **unified oauth token management**: per-provider credentials with automatic refresh

### Intelligence (`src/intelligence/`)
- activity analysis and insights
- performance trend detection
- training load calculation
- goal feasibility analysis

### Database (`src/database/`)
- **repository pattern**: 13 focused repositories following SOLID principles
- repository accessors: `db.users()`, `db.oauth_tokens()`, `db.api_keys()`, `db.profiles()`, etc.
- pluggable backend (sqlite, postgresql) via `src/database_plugins/`
- encrypted token storage
- multi-tenant isolation

#### Repository Architecture

The database layer implements the repository pattern with focused, cohesive repositories:

**13 focused repositories** (`src/database/repositories/`):
1. `UserRepository` - user account management
2. `OAuthTokenRepository` - oauth token storage (tenant-scoped)
3. `ApiKeyRepository` - api key management
4. `UsageRepository` - usage tracking and analytics
5. `A2ARepository` - agent-to-agent management
6. `ProfileRepository` - user profiles and goals
7. `InsightRepository` - ai-generated insights
8. `AdminRepository` - admin token management
9. `TenantRepository` - multi-tenant management
10. `OAuth2ServerRepository` - oauth 2.0 server functionality
11. `SecurityRepository` - key rotation and audit
12. `NotificationRepository` - oauth notifications
13. `FitnessConfigRepository` - fitness configuration management

**accessor pattern** (`src/database/mod.rs:139-245`):
```rust
let db = Database::new(database_url, encryption_key).await?;

// Access repositories via typed accessors
let user = db.users().get_by_id(user_id).await?;
let token = db.oauth_tokens().get(user_id, tenant_id, provider).await?;
let api_key = db.api_keys().get_by_key(key).await?;
```

**benefits**:
- **single responsibility**: each repository handles one domain
- **interface segregation**: consumers only depend on needed methods
- **testability**: mock individual repositories independently
- **maintainability**: changes isolated to specific repositories

### Authentication (`src/auth.rs`)
- jwt token generation/validation
- api key management
- rate limiting per tenant

## Error Handling

Pierre Fitness Platform uses structured error types for precise error handling and propagation. The codebase **does not use anyhow** - all errors are structured types using `thiserror`.

### Error Type Hierarchy

```
AppError (src/errors.rs)
├── Database(DatabaseError)
├── Provider(ProviderError)
├── Authentication
├── Authorization
├── Validation
└── Internal
```

### Error Types

**DatabaseError** (`src/database/errors.rs`):
- `NotFound`: entity not found (user, token, oauth client)
- `QueryFailed`: database query execution failure
- `ConstraintViolation`: unique constraint or foreign key violations
- `ConnectionFailed`: database connection issues
- `TransactionFailed`: transaction commit/rollback errors

**ProviderError** (`src/providers/errors.rs`):
- `ApiError`: fitness provider api errors (status code + message)
- `AuthenticationFailed`: oauth token invalid or expired
- `RateLimitExceeded`: provider rate limit hit
- `NetworkError`: network connectivity issues
- `Unavailable`: provider temporarily unavailable

**AppError** (`src/errors.rs`):
- application-level errors with error codes
- http status code mapping
- structured error responses with context

### Error Propagation

All fallible operations return `Result<T, E>` types with **structured error types only**:
```rust
pub async fn get_user(db: &Database, user_id: &str) -> Result<User, DatabaseError>
pub async fn fetch_activities(provider: &Strava) -> Result<Vec<Activity>, ProviderError>
pub async fn process_request(req: Request) -> Result<Response, AppError>
```

**AppResult type alias** (`src/errors.rs`):
```rust
pub type AppResult<T> = Result<T, AppError>;
```

Errors propagate using `?` operator with automatic conversion via `From` trait implementations:
```rust
// DatabaseError converts to AppError via From<DatabaseError>
let user = db.users().get_by_id(user_id).await?;

// ProviderError converts to AppError via From<ProviderError>
let activities = provider.fetch_activities().await?;
```

**no blanket anyhow conversions**: the codebase enforces zero-tolerance for `impl From<anyhow::Error>` via static analysis (`scripts/lint-and-test.sh`) to prevent loss of type information.

### Error Responses

Structured json error responses:
```json
{
  "error": {
    "code": "database_not_found",
    "message": "User not found: user-123",
    "details": {
      "entity_type": "user",
      "entity_id": "user-123"
    }
  }
}
```

Http status mapping:
- `DatabaseError::NotFound` → 404
- `ProviderError::ApiError` → 502/503
- `AppError::Validation` → 400
- `AppError::Authentication` → 401
- `AppError::Authorization` → 403

Implementation: `src/errors.rs`, `src/database/errors.rs`, `src/providers/errors.rs`

## Request Flow

```
client request
    ↓
[security middleware] → cors, headers, csrf
    ↓
[authentication] → jwt or api key
    ↓
[tenant context] → load user/tenant data
    ↓
[rate limiting] → check quotas
    ↓
[protocol router]
    ├─ mcp → universal protocol → tools
    ├─ a2a → universal protocol → tools
    └─ rest → direct handlers
    ↓
[tool execution]
    ├─ providers (strava/garmin/fitbit/whoop)
    ├─ intelligence (analysis)
    └─ configuration
    ↓
[database + cache]
    ↓
response
```

## Multi-Tenancy

Every request operates within tenant context:
- isolated data per tenant
- tenant-specific encryption keys
- custom rate limits
- feature flags

## Key Design Decisions

### Single Port Architecture
All protocols share port 8081. Simplified deployment, easier oauth2 callback handling, unified tls/security.

### Focused Context Dependency Injection

Replaces service locator anti-pattern with focused contexts providing type-safe DI with minimal coupling.

**context hierarchy** (`src/context/`):
```
ServerContext
├── AuthContext       (auth_manager, auth_middleware, admin_jwt_secret, jwks_manager)
├── DataContext       (database, provider_registry, activity_intelligence)
├── ConfigContext     (config, tenant_oauth_client, a2a_client_manager)
└── NotificationContext (websocket_manager, oauth_notification_sender)
```

**usage pattern**:
```rust
// Access specific contexts from ServerContext
let user = ctx.data().database().users().get_by_id(id).await?;
let token = ctx.auth().auth_manager().validate_token(jwt)?;
```

**benefits**:
- **single responsibility**: each context handles one domain
- **interface segregation**: handlers depend only on needed contexts
- **testability**: mock individual contexts independently
- **type safety**: compile-time verification of dependencies

**migration**: `ServerContext::from(&ServerResources)` provides gradual migration path.

### Protocol Abstraction
Business logic in `protocols::universal` works for both mcp and a2a. Write once, use everywhere.

### Pluggable Architecture
- database: sqlite (dev) or postgresql (prod)
- cache: in-memory lru or redis (distributed caching)
- tools: compile-time plugin system via `linkme`

### SDK Architecture

**TypeScript SDK** (`sdk/`): stdio→http bridge for MCP clients (Claude Desktop, ChatGPT).

```
MCP Client (Claude Desktop)
    ↓ stdio (json-rpc)
pierre-mcp-client (npm package)
    ↓ http (json-rpc)
Pierre MCP Server (rust)
```

**key features**:
- automatic oauth2 token management (browser-based auth flow)
- token refresh handling
- secure credential storage via system keychain
- npx deployment: `npx -y pierre-mcp-client@next --server http://localhost:8081`

Implementation: `sdk/src/bridge.ts`, `sdk/src/cli.ts`

### Type Mapping System

**rust→typescript type generation**: auto-generates TypeScript interfaces from server JSON schemas.

```
src/mcp/schema.rs (tool definitions)
    ↓ npm run generate-types
sdk/src/types.ts (47 parameter interfaces)
```

**type-safe json schemas** (`src/types/json_schemas.rs`):
- replaces dynamic `serde_json::Value` with typed structs
- compile-time validation via serde
- fail-fast error handling with clear error messages
- backwards compatibility via field aliases (`#[serde(alias = "type")]`)

**generated types include**:
- `ToolParamsMap` - maps tool names to parameter types
- `ToolName` - union type of all 47 tool names
- common data types: `Activity`, `Athlete`, `Stats`, `FitnessConfig`

Usage: `npm run generate-types` (requires running server on port 8081)

## File Structure

```
src/
├── bin/
│   ├── pierre-mcp-server.rs     # main binary
│   ├── admin_setup.rs           # admin cli tool (binary: admin-setup)
│   └── diagnose_weather_api.rs  # weather api diagnostic tool
├── protocols/
│   └── universal/             # shared business logic
├── mcp/                       # mcp protocol
├── oauth2_server/             # oauth2 authorization server (mcp clients → pierre)
├── oauth2_client/             # oauth2 client (pierre → fitness providers)
├── a2a/                       # a2a protocol
├── providers/                 # fitness integrations
├── intelligence/              # activity analysis
├── database/                  # repository pattern (13 focused repositories)
│   ├── repositories/          # repository trait definitions and implementations
│   └── ...                    # user, oauth token, api key management modules
├── database_plugins/          # database backends (sqlite, postgresql)
├── admin/                     # admin authentication
├── context/                   # focused di contexts (auth, data, config, notification)
├── auth.rs                    # authentication
├── tenant/                    # multi-tenancy
├── tools/                     # tool execution engine
├── cache/                     # caching layer
├── config/                    # configuration
├── constants/                 # constants and defaults
├── crypto/                    # encryption utilities
├── types/                     # type-safe json schemas
└── lib.rs                     # public api
sdk/                           # typescript mcp client
├── src/bridge.ts              # stdio→http bridge
├── src/types.ts               # auto-generated types
└── test/                      # integration tests
```

## Security Layers

1. **transport**: https/tls
2. **authentication**: jwt tokens, api keys
3. **authorization**: tenant-based rbac
4. **encryption**: two-tier key management
   - master key: encrypts tenant keys
   - tenant keys: encrypt user tokens
5. **rate limiting**: token bucket per tenant
6. **atomic operations**: toctou prevention
   - refresh token consumption: atomic check-and-revoke
   - prevents race conditions in token exchange
   - database-level atomicity guarantees

## Scalability

### Horizontal Scaling
Stateless server design. Scale by adding instances behind load balancer. Shared postgresql and optional redis for distributed cache.

### Database Sharding
- tenant-based sharding
- time-based partitioning for historical data
- provider-specific tables

### Caching Strategy
- health checks: 30s ttl
- mcp sessions: lru cache (10k entries)
- weather data: configurable ttl
- distributed cache: redis support for multi-instance deployments
- in-memory fallback: lru cache with automatic eviction

## Plugin Lifecycle

Compile-time plugin system using `linkme` crate for intelligence modules.

Plugins stored in `src/intelligence/plugins/`:
- zone-based intensity analysis
- training recommendations
- performance trend detection
- goal feasibility analysis

Lifecycle hooks:
- `init()` - plugin initialization
- `execute()` - tool execution
- `validate()` - parameter validation
- `cleanup()` - resource cleanup

Plugins registered at compile time via `#[distributed_slice(PLUGINS)]` attribute.
No runtime loading, zero overhead plugin discovery.

Implementation: `src/intelligence/plugins/mod.rs`, `src/lifecycle/`

## Algorithm Dependency Injection

Zero-overhead algorithm dispatch using rust enums instead of hardcoded formulas.

### Design Pattern

Fitness intelligence uses enum-based dependency injection for all calculation algorithms:

```rust
pub enum VdotAlgorithm {
    Daniels,                    // Jack Daniels' formula
    Riegel { exponent: f64 },   // Power-law model
    Hybrid,                     // Auto-select based on data
}

impl VdotAlgorithm {
    pub fn calculate_vdot(&self, distance: f64, time: f64) -> Result<f64, AppError> {
        match self {
            Self::Daniels => Self::calculate_daniels(distance, time),
            Self::Riegel { exponent } => Self::calculate_riegel(distance, time, *exponent),
            Self::Hybrid => Self::calculate_hybrid(distance, time),
        }
    }
}
```

### Benefits

**compile-time dispatch**: zero runtime overhead, inlined by llvm
**configuration flexibility**: runtime algorithm selection via environment variables
**defensive programming**: hybrid variants with automatic fallback
**testability**: each variant independently testable
**maintainability**: all algorithm logic in single enum file
**no magic strings**: type-safe algorithm selection

### Algorithm Types

Nine algorithm categories with multiple variants each:

1. **max heart rate** (`src/intelligence/algorithms/max_heart_rate.rs`)
   - fox, tanaka, nes, gulati
   - environment: `PIERRE_MAXHR_ALGORITHM`

2. **training impulse (trimp)** (`src/intelligence/algorithms/trimp.rs`)
   - bannister male/female, edwards, lucia, hybrid
   - environment: `PIERRE_TRIMP_ALGORITHM`

3. **training stress score (tss)** (`src/intelligence/algorithms/tss.rs`)
   - avg_power, normalized_power, hybrid
   - environment: `PIERRE_TSS_ALGORITHM`

4. **vdot** (`src/intelligence/algorithms/vdot.rs`)
   - daniels, riegel, hybrid
   - environment: `PIERRE_VDOT_ALGORITHM`

5. **training load** (`src/intelligence/algorithms/training_load.rs`)
   - ema, sma, wma, kalman filter
   - environment: `PIERRE_TRAINING_LOAD_ALGORITHM`

6. **recovery aggregation** (`src/intelligence/algorithms/recovery_aggregation.rs`)
   - weighted, additive, multiplicative, minmax, neural
   - environment: `PIERRE_RECOVERY_ALGORITHM`

7. **functional threshold power (ftp)** (`src/intelligence/algorithms/ftp.rs`)
   - 20min_test, 8min_test, ramp_test, from_vo2max, hybrid
   - environment: `PIERRE_FTP_ALGORITHM`

8. **lactate threshold heart rate (lthr)** (`src/intelligence/algorithms/lthr.rs`)
   - from_maxhr, from_30min, from_race, lab_test, hybrid
   - environment: `PIERRE_LTHR_ALGORITHM`

9. **vo2max estimation** (`src/intelligence/algorithms/vo2max_estimation.rs`)
   - from_vdot, cooper, rockport, astrand, bruce, hybrid
   - environment: `PIERRE_VO2MAX_ALGORITHM`

### Configuration Integration

Algorithms configured via `src/config/intelligence_config.rs`:

```rust
pub struct AlgorithmConfig {
    pub max_heart_rate: String,     // PIERRE_MAXHR_ALGORITHM
    pub trimp: String,               // PIERRE_TRIMP_ALGORITHM
    pub tss: String,                 // PIERRE_TSS_ALGORITHM
    pub vdot: String,                // PIERRE_VDOT_ALGORITHM
    pub training_load: String,       // PIERRE_TRAINING_LOAD_ALGORITHM
    pub recovery_aggregation: String, // PIERRE_RECOVERY_ALGORITHM
    pub ftp: String,                 // PIERRE_FTP_ALGORITHM
    pub lthr: String,                // PIERRE_LTHR_ALGORITHM
    pub vo2max: String,              // PIERRE_VO2MAX_ALGORITHM
}
```

Defaults optimized for balanced accuracy vs data requirements.

### Enforcement

Automated validation ensures no hardcoded algorithms bypass the enum system.

Validation script: `scripts/validate-algorithm-di.sh`
Patterns defined: `scripts/validation-patterns.toml`

Checks for:
- hardcoded formulas (e.g., `220 - age`)
- magic numbers (e.g., `0.182258` in non-algorithm files)
- algorithmic logic outside enum implementations

Exclusions documented in validation patterns (e.g., tests, algorithm enum files).

Ci pipeline fails on algorithm di violations (zero tolerance).

### Hybrid Algorithms

Special variant that provides defensive fallback logic:

```rust
pub enum TssAlgorithm {
    AvgPower,                // Simple, always works
    NormalizedPower { .. },  // Accurate, requires power stream
    Hybrid,                  // Try NP, fallback to avg_power
}

impl TssAlgorithm {
    fn calculate_hybrid(&self, activity: &Activity, ...) -> Result<f64, AppError> {
        Self::calculate_np_tss(activity, ...)
            .or_else(|_| Self::calculate_avg_power_tss(activity, ...))
    }
}
```

Hybrid algorithms maximize reliability while preferring accuracy when data available.

### Usage Pattern

All intelligence calculations use algorithm enums:

```rust
use crate::intelligence::algorithms::vdot::VdotAlgorithm;
use crate::config::intelligence_config::get_config;

let config = get_config();
let algorithm = VdotAlgorithm::from_str(&config.algorithms.vdot)?;
let vdot = algorithm.calculate_vdot(5000.0, 1200.0)?; // 5K in 20:00
```

No hardcoded formulas anywhere in intelligence layer.

Implementation: `src/intelligence/algorithms/`, `src/config/intelligence_config.rs`, `scripts/validate-algorithm-di.sh`

## PII Redaction

Middleware layer removes sensitive data from logs and responses.

Redacted fields:
- email addresses
- passwords
- tokens (jwt, oauth, api keys)
- user ids
- tenant ids

Redaction patterns:
- email: `***@***.***`
- token: `[REDACTED-<type>]`
- uuid: `[REDACTED-UUID]`

Enabled via `LOG_FORMAT=json` for structured logging.
Implementation: `src/middleware/redaction.rs`

## Cursor Pagination

Keyset pagination using composite cursor (`created_at`, `id`) for consistent ordering.

Benefits:
- no duplicate results during data changes
- stable pagination across pages
- efficient for large datasets

Cursor format: base64-encoded json with timestamp (milliseconds) + id.

Example:
```
cursor: "eyJ0aW1lc3RhbXAiOjE3MDAwMDAwMDAsImlkIjoiYWJjMTIzIn0="
decoded: {"timestamp":1700000000,"id":"abc123"}
```

Endpoints using cursor pagination:
- `GET /admin/users/pending?cursor=<cursor>&limit=20`
- `GET /admin/users/active?cursor=<cursor>&limit=20`

Implementation: `src/pagination/`, `src/database/users.rs:668-737`, `src/database_plugins/postgres.rs:378-420`

## Monitoring

Health endpoint: `GET /health`
- database connectivity
- provider availability
- system uptime
- cache statistics

Logs: structured json via tracing + opentelemetry
Metrics: request latency, error rates, provider api usage

---

# Build Configuration

Technical documentation for build system configuration, linting enforcement, and compilation settings.

## Rust Toolchain Management

**File**: `rust-toolchain`
**Current version**: `1.91.0`

### Version Pinning Strategy

The project pins the exact Rust version to ensure reproducible builds across development and CI/CD environments. This eliminates "works on my machine" issues and enforces consistent compiler behavior.

**Rationale for 1.91.0**:
- Stable rust 2021 edition support
- clippy lint groups fully stabilized
- sqlx compile-time query checking compatibility
- tokio 1.x runtime stability

### Updating Rust Version

Update process requires validation across:
1. clippy lint compatibility (all/pedantic/nursery groups)
2. sqlx macro compatibility (database query verification)
3. tokio runtime stability
4. dependency compatibility check via `cargo tree`

**Command**: Update `rust-toolchain` file and run full validation:
```bash
echo "1.XX.0" > rust-toolchain
./scripts/lint-and-test.sh
```

## Cargo.toml Linting Configuration

### Zero-Tolerance Enforcement Model

Lines 148-208 define compile-time error enforcement via `[lints.rust]` and `[lints.clippy]`.

**Design decision**: All clippy warnings are build errors via `level = "deny"`. This eliminates the "fix it later" anti-pattern and prevents technical debt accumulation.

### Clippy Lint Groups

```toml
[lints.clippy]
all = { level = "deny", priority = -1 }
pedantic = { level = "deny", priority = -1 }
nursery = { level = "deny", priority = -1 }
```

**Rationale**:
- `all`: Standard correctness lints (memory safety, logic errors)
- `pedantic`: Code quality lints (style, readability)
- `nursery`: Experimental lints (cutting-edge analysis)
- `priority = -1`: Apply base groups first, allow specific overrides

**Trade-off**: Nursery lints may change behavior between rust versions. Accepted for early detection of potential issues.

### Unsafe Code Policy

```toml
[lints.rust]
unsafe_code = "deny"
```

**Enforcement model**: deny-by-default with whitelist validation.

**Approved locations**:
- `src/health.rs`: Windows FFI for system health metrics (`GlobalMemoryStatusEx`, `GetDiskFreeSpaceExW`)

**Validation**: `scripts/architectural-validation.sh` fails build if unsafe code appears outside approved locations.

**Rationale**: Unsafe code eliminates rust's memory safety guarantees. Whitelist approach ensures:
1. All unsafe usage is justified and documented
2. Unsafe code is isolated to specific modules
3. Code review focuses on unsafe boundaries
4. FFI interactions are contained

### Error Handling Enforcement

```toml
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
```

**Acceptable contexts**:
- Test code with documented failure expectations
- Static data known valid at compile time (e.g., regex compilation in const context)
- Binary `main()` functions where failure should terminate process

**Production code requirements**:
- All fallible operations return `Result<T, E>`
- Error propagation via `?` operator
- Structured error types (AppError, DatabaseError, ProviderError)
- No string-based errors

**Rationale**: `unwrap()` causes panics on `None`/`Err`, crashing the server. Production services must handle errors gracefully and return structured error responses.

### Type Conversion Safety

```toml
cast_possible_truncation = "allow"
cast_sign_loss = "allow"
cast_precision_loss = "allow"
```

**Rationale**: Type conversions are validated at call sites via context analysis. Blanket denial creates false positives for:
- `u64` → `usize` (safe on 64-bit systems)
- `f64` → `f32` (acceptable precision loss for display)
- `i64` → `u64` (validated non-negative before cast)

**Requirement**: Casts must be documented with safety justification when non-obvious.

### Function Size Policy

```toml
too_many_lines = "allow"
```

**Policy**: Functions over 100 lines trigger manual review but don't fail build.

**Validation**: Scripts detect functions >100 lines and verify documentation comment explaining complexity. Functions >100 lines require:
- `// Long function:` comment with rationale, OR
- Decomposition into helper functions

**Rationale**: Some functions have legitimate complexity (e.g., protocol parsers, error handling dispatchers). Blanket 50-line limit creates artificial decomposition that reduces readability.

### Additional Quality Lints

```toml
clone_on_copy = "warn"      # Cloning Copy types is inefficient
redundant_clone = "warn"     # Unnecessary allocations
await_holding_lock = "warn"  # Deadlock prevention
str_to_string = "deny"       # Prefer .to_owned() for clarity
```

## Build Profiles

### Dev Profile

```toml
[profile.dev]
debug = 1            # line number information for backtraces
opt-level = 0        # no optimization, fastest compilation
overflow-checks = true   # catch integer overflow in debug builds
```

**Use case**: Development iteration speed. Prioritizes compilation time over runtime performance.

### Release Profile

```toml
[profile.release]
lto = "thin"         # link-time optimization (intra-crate)
codegen-units = 1    # single codegen unit for better optimization
panic = "abort"      # reduce binary size, no unwinding
strip = true         # remove debug symbols
```

**Binary size impact**: ~40% size reduction vs unoptimized
**Compilation time**: +30% vs dev profile
**Runtime performance**: 2-5x faster than dev builds

**Rationale**:
- `lto = "thin"`: Balance between compilation time and optimization
- `codegen-units = 1`: Maximum intra-crate optimization
- `panic = "abort"`: Production services should crash on panic (no recovery)
- `strip = true`: Debug symbols not needed in production

### Release-LTO Profile

```toml
[profile.release-lto]
inherits = "release"
lto = "fat"          # cross-crate optimization
```

**Binary size impact**: Additional 10-15% size reduction
**Compilation time**: 2-3x slower than thin LTO
**Runtime performance**: Marginal improvement (5-10%) over thin LTO

**Use case**: Distribution builds where binary size critical. Not used in CI/CD due to compilation time.

## Feature Flags

```toml
[features]
default = ["sqlite"]
sqlite = []
postgresql = ["sqlx/postgres"]
testing = []
telemetry = []
```

**Design decision**: Compile-time feature selection eliminates runtime configuration complexity.

**sqlite (default)**: Development and single-instance deployments
**postgresql**: Production multi-instance deployments with shared state
**testing**: Test utilities and mock implementations
**telemetry**: OpenTelemetry instrumentation (production observability)

**Binary size impact**:
- sqlite-only: ~45MB
- sqlite+postgresql: ~48MB
- All features: ~50MB

## Dependency Strategy

### Principle: Minimal Dependencies

Each dependency increases:
- Binary size (transitive dependencies)
- Compilation time
- Supply chain attack surface
- Maintenance burden (version conflicts)

**Review process**: New dependencies require justification:
1. What stdlib/existing dependency could solve this?
2. What's the binary size impact? (`cargo bloat`)
3. Is the crate maintained? (recent commits, issue response)
4. What's the transitive dependency count? (`cargo tree`)

### Pinned Dependencies

```toml
base64ct = "=1.6.0"
```

**Rationale**: base64ct 1.7.0+ requires rust edition 2024, incompatible with dependencies still on edition 2021. Pin eliminates upgrade-time breakage.

### Feature-Gated Dependencies

```toml
reqwest = { version = "0.12", features = ["json", "rustls-tls", "stream"], default-features = false }
sqlx = { version = "0.8", features = ["runtime-tokio-rustls", "sqlite", "postgres", ...], default-features = false }
```

**Rationale**: `default-features = false` eliminates unused functionality:
- reqwest: Exclude native-tls (prefer rustls for pure-rust stack)
- sqlx: Exclude mysql/mssql drivers

**Binary size savings**: ~5MB from feature pruning

## Validation Commands

### Pre-Commit Checks

```bash
# Linting (zero warnings)
cargo clippy --all-targets --all-features

# Type checking
cargo check --all-features

# Tests
cargo test --release

# Binary size
cargo build --release && ls -lh target/release/pierre-mcp-server

# Security audit
cargo deny check

# Full validation
./scripts/lint-and-test.sh
```

### CI/CD Validation

The project uses five GitHub Actions workflows for comprehensive validation:

1. **Rust** (`.github/workflows/rust.yml`): Core quality gate
   - clippy zero-warning check
   - Test suite execution with coverage
   - Security audit (cargo-deny)
   - Architecture validation (unsafe code, algorithm patterns)

2. **Backend CI** (`.github/workflows/ci.yml`): Multi-database validation
   - SQLite + PostgreSQL test execution
   - Frontend tests (Node.js/TypeScript)
   - Secret pattern validation
   - Separate coverage for each database

3. **Cross-Platform** (`.github/workflows/cross-platform.yml`): OS compatibility
   - Linux (PostgreSQL), macOS (SQLite), Windows (SQLite)
   - Platform-specific optimizations

4. **SDK Tests** (`.github/workflows/sdk-tests.yml`): TypeScript SDK bridge
   - Unit, integration, and E2E tests
   - SDK ↔ Rust server communication validation

5. **MCP Compliance** (`.github/workflows/mcp-compliance.yml`): Protocol specification
   - MCP protocol conformance testing
   - TypeScript type validation

**See ci/cd.md for comprehensive workflow documentation, troubleshooting guides, and local validation commands.**

## Cargo-Deny Configuration

**File**: `deny.toml`

### Security Advisory Scanning

```toml
[advisories]
ignore = [
    "RUSTSEC-2023-0071",  # Legacy ignore
    "RUSTSEC-2024-0384",  # instant crate unmaintained (no safe upgrade path)
    "RUSTSEC-2024-0387",  # opentelemetry_api merged (used by opentelemetry-stdout)
]
```

**Rationale**: Ignored advisories have no safe upgrade path or are false positives for our usage. Requires periodic review.

### License Compliance

```toml
[licenses]
allow = [
    "MIT", "Apache-2.0",        # Standard permissive licenses
    "BSD-3-Clause",             # Crypto libraries
    "ISC",                      # ring, untrusted
    "Unicode-3.0",              # ICU unicode data
    "CDLA-Permissive-2.0",      # TLS root certificates
    "MPL-2.0", "Zlib",          # Additional OSI-approved
]
```

**Policy**: Only OSI-approved permissive licenses allowed. Copyleft licenses (GPL, AGPL) prohibited due to distribution restrictions.

### Supply Chain Protection

```toml
[sources]
unknown-git = "deny"
unknown-registry = "deny"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]
```

**Rationale**: Only crates.io dependencies allowed. Prevents supply chain attacks via malicious git repositories or alternate registries.

## Compilation Optimization Notes

### LTO Trade-offs

**thin lto**: Optimizes within each crate, respects crate boundaries
- Compilation time: Moderate
- Optimization level: Good
- Incremental compilation: Partially supported

**fat lto**: Optimizes across all crate boundaries
- Compilation time: Slow (2-3x thin LTO)
- Optimization level: Best
- Incremental compilation: Not supported

**Decision**: Use thin LTO for CI/CD (balance), fat LTO for releases (when available).

### Codegen-Units

`codegen-units = 1` forces single-threaded LLVM optimization.

**Trade-off**:
- ❌ Slower compilation (no parallel codegen)
- ✅ Better optimization (more context for LLVM)
- ✅ Smaller binary size

**Rationale**: CI/CD runs in parallel on GitHub Actions. Single-codegen-unit optimization per build is acceptable.

### Panic Handling

`panic = "abort"` eliminates unwinding machinery.

**Binary size savings**: ~1-2MB
**Runtime impact**: Panics terminate process immediately (no `Drop` execution)

**Rationale**: Production services using structured error handling should never panic. If panic occurs, it's a bug requiring process restart.

## Historical Notes

### Removed Configurations

**toml dependency** (line 91): Removed in favor of environment-only configuration
- Rationale: Environment variables eliminate config file complexity
- No runtime config file parsing
- 12-factor app compliance

**auth-setup binary** (lines 19-21): Commented out, replaced by admin-setup
- Migration: Consolidated authentication setup into admin CLI tool
- Maintains backward compatibility via admin-setup commands

---

# Configuration

## Environment Variables

Pierre Fitness Platform configured entirely via environment variables. No config files.

### Required Variables

```bash
# database
DATABASE_URL="sqlite:./data/users.db"  # or postgresql://...

# encryption (generate: openssl rand -base64 32)
PIERRE_MASTER_ENCRYPTION_KEY="<base64_encoded_32_bytes>"
```

### Server Configuration

```bash
# network
HTTP_PORT=8081                    # server port (default: 8081)
HOST=127.0.0.1                    # bind address (default: 127.0.0.1)

# logging
RUST_LOG=info                     # log level (error, warn, info, debug, trace)
LOG_FORMAT=json                   # json or pretty (default: pretty)
LOG_INCLUDE_LOCATION=1            # include file/line numbers (production: auto-enabled)
LOG_INCLUDE_THREAD=1              # include thread information (production: auto-enabled)
LOG_INCLUDE_SPANS=1               # include tracing spans (production: auto-enabled)
```

### Logging and Observability

Pierre provides production-ready logging with structured output, request correlation, and performance monitoring.

#### HTTP Request Logging

Automatic HTTP request/response logging via tower-http TraceLayer:

**what gets logged**:
- request: method, URI, HTTP version
- response: status code, latency (milliseconds)
- request ID: unique UUID for correlation

**example output** (INFO level):
```
INFO request{method=GET uri=/health}: tower_http::trace::on_response status=200 latency=5ms
INFO request{method=POST uri=/auth/login}: tower_http::trace::on_response status=200 latency=45ms
INFO request{method=GET uri=/api/activities}: tower_http::trace::on_response status=200 latency=235ms
```

**verbosity control**:
- `RUST_LOG=tower_http=warn` - disable HTTP request logs
- `RUST_LOG=tower_http=info` - enable HTTP request logs (default)
- `RUST_LOG=tower_http=debug` - add request/response headers

#### Structured Logging (JSON Format)

JSON format recommended for production deployments:

```bash
LOG_FORMAT=json
RUST_LOG=info
```

**benefits**:
- machine-parseable for log aggregation (Elasticsearch, Splunk, etc.)
- automatic field extraction for querying
- preserves structured data (no string parsing needed)
- efficient storage and indexing

**fields included**:
- `timestamp`: ISO 8601 timestamp with milliseconds
- `level`: log level (ERROR, WARN, INFO, DEBUG, TRACE)
- `target`: rust module path (e.g., `pierre_mcp_server::routes::auth`)
- `message`: human-readable message
- `span`: tracing span context (operation, duration, fields)
- `fields`: structured key-value pairs

**example json output**:
```json
{"timestamp":"2025-01-13T10:23:45.123Z","level":"INFO","target":"pierre_mcp_server::routes::auth","fields":{"route":"login","email":"user@example.com"},"message":"User login attempt for email: user@example.com"}
{"timestamp":"2025-01-13T10:23:45.168Z","level":"INFO","target":"tower_http::trace::on_response","fields":{"method":"POST","uri":"/auth/login","status":200,"latency_ms":45},"message":"request completed"}
```

**pretty format** (development default):
```
2025-01-13T10:23:45.123Z  INFO pierre_mcp_server::routes::auth route=login email=user@example.com: User login attempt for email: user@example.com
2025-01-13T10:23:45.168Z  INFO tower_http::trace::on_response method=POST uri=/auth/login status=200 latency_ms=45: request completed
```

#### Request ID Correlation

Every HTTP request receives unique X-Request-ID header for distributed tracing:

**response header**:
```
HTTP/1.1 200 OK
X-Request-ID: 550e8400-e29b-41d4-a716-446655440000
Content-Type: application/json
```

**tracing through logs**:

Find all logs for specific request:
```bash
# json format
cat logs/pierre.log | jq 'select(.fields.request_id == "550e8400-e29b-41d4-a716-446655440000")'

# pretty format
grep "550e8400-e29b-41d4-a716-446655440000" logs/pierre.log
```

**benefits**:
- correlate logs across microservices
- debug user-reported issues via request ID
- trace request flow through database, APIs, external providers
- essential for production troubleshooting

#### Performance Monitoring

Automatic timing spans for critical operations:

**database operations**:
```rust
#[tracing::instrument(skip(self), fields(db_operation = "get_user"))]
async fn get_user(&self, user_id: Uuid) -> Result<Option<User>>
```

**provider api calls**:
```rust
#[tracing::instrument(skip(self), fields(provider = "strava", api_call = "get_activities"))]
async fn get_activities(&self, limit: Option<usize>) -> Result<Vec<Activity>>
```

**route handlers**:
```rust
#[tracing::instrument(skip(self, request), fields(route = "login", email = %request.email))]
pub async fn login(&self, request: LoginRequest) -> AppResult<LoginResponse>
```

**example performance logs**:
```
DEBUG pierre_mcp_server::database db_operation=get_user user_id=123e4567-e89b-12d3-a456-426614174000 duration_ms=12
INFO pierre_mcp_server::providers::strava provider=strava api_call=get_activities duration_ms=423
INFO pierre_mcp_server::routes::auth route=login email=user@example.com duration_ms=67
```

**analyzing performance**:
```bash
# find slow database queries (>100ms)
cat logs/pierre.log | jq 'select(.fields.db_operation and .fields.duration_ms > 100)'

# find slow API calls (>500ms)
cat logs/pierre.log | jq 'select(.fields.api_call and .fields.duration_ms > 500)'

# average response time per route
cat logs/pierre.log | jq -r 'select(.fields.route) | "\(.fields.route) \(.fields.duration_ms)"' | awk '{sum[$1]+=$2; count[$1]++} END {for (route in sum) print route, sum[route]/count[route]}'
```

#### Security and Privacy

**no sensitive data logged**:
- JWT secrets never logged (removed in production-ready improvements)
- passwords never logged (hashed before storage)
- OAuth tokens never logged (encrypted at rest)
- PII redacted by default (emails masked in non-auth logs)

**verified security**:
```bash
# verify no JWT secrets in logs
RUST_LOG=debug cargo run 2>&1 | grep -i "secret\|password\|token" | grep -v "access_token"
# should show: no JWT secret exposure, only generic "initialized successfully" messages
```

**safe to log**:
- user IDs (UUIDs, not emails)
- request IDs (correlation)
- operation types (login, get_activities, etc.)
- performance metrics (duration, status codes)
- error categories (not full stack traces with sensitive data)

### Authentication

```bash
# jwt tokens
JWT_EXPIRY_HOURS=24               # token lifetime (default: 24)
JWT_SECRET_PATH=/path/to/secret   # optional: load secret from file
PIERRE_RSA_KEY_SIZE=4096          # rsa key size for rs256 signing (default: 4096, test: 2048)

# oauth2 server
OAUTH2_ISSUER_URL=http://localhost:8081  # oauth2 discovery issuer url (default: http://localhost:8081)

# password hashing
PASSWORD_HASH_ALGORITHM=argon2    # argon2 or bcrypt (default: argon2)
```

### Fitness Providers

#### strava

```bash
STRAVA_CLIENT_ID=your_id
STRAVA_CLIENT_SECRET=your_secret
STRAVA_REDIRECT_URI=http://localhost:8081/api/oauth/callback/strava  # local development only
```

**security warning**: http callback urls only for local development. Production must use https:
```bash
STRAVA_REDIRECT_URI=https://api.example.com/api/oauth/callback/strava  # production
```

Get credentials: https://www.strava.com/settings/api

#### Garmin

```bash
GARMIN_CLIENT_ID=your_consumer_key
GARMIN_CLIENT_SECRET=your_consumer_secret
GARMIN_REDIRECT_URI=http://localhost:8081/api/oauth/callback/garmin  # local development only
```

**security warning**: http callback urls only for local development. Production must use https:
```bash
GARMIN_REDIRECT_URI=https://api.example.com/api/oauth/callback/garmin  # production
```

Get credentials: https://developer.garmin.com/

#### Whoop

```bash
WHOOP_CLIENT_ID=your_client_id
WHOOP_CLIENT_SECRET=your_client_secret
WHOOP_REDIRECT_URI=http://localhost:8081/api/oauth/callback/whoop  # local development only
```

**security warning**: http callback urls only for local development. Production must use https:
```bash
WHOOP_REDIRECT_URI=https://api.example.com/api/oauth/callback/whoop  # production
```

Get credentials: https://developer.whoop.com/

**whoop capabilities**:
- Sleep tracking (sleep sessions, sleep stages, sleep need)
- Recovery metrics (HRV, recovery score, strain)
- Workout activities (with heart rate zones, strain scores)
- Health metrics (SpO2, skin temperature, body measurements)

**whoop scopes**:
- `offline`: Required for refresh tokens
- `read:profile`: User profile information
- `read:body_measurement`: Height, weight, max heart rate
- `read:workout`: Workout/activity data
- `read:sleep`: Sleep tracking data
- `read:recovery`: Recovery scores
- `read:cycles`: Physiological cycle data

#### Terra

Terra provides unified access to 150+ wearable devices through a single API.

```bash
TERRA_API_KEY=your_api_key
TERRA_DEV_ID=your_dev_id
TERRA_WEBHOOK_SECRET=your_webhook_secret  # for webhook data ingestion
```

Get credentials: https://tryterra.co/

**terra capabilities**:
- Unified API for 150+ wearables (Garmin, Polar, WHOOP, Oura, etc.)
- Webhook-based data ingestion
- Activity, sleep, and health data aggregation

#### Fitbit

```bash
FITBIT_CLIENT_ID=your_id
FITBIT_CLIENT_SECRET=your_secret
FITBIT_REDIRECT_URI=http://localhost:8081/api/oauth/callback/fitbit  # local development only
```

**security warning**: http callback urls only for local development. Production must use https:
```bash
FITBIT_REDIRECT_URI=https://api.example.com/api/oauth/callback/fitbit  # production
```

Get credentials: https://dev.fitbit.com/apps

**callback url security**:
- **http**: local development only (`localhost` or `127.0.0.1`)
  - tokens transmitted unencrypted
  - vulnerable to mitm attacks
  - some providers reject http in production
- **https**: production deployments (required)
  - tls encryption protects tokens in transit
  - prevents credential interception
  - required by most oauth providers in production

#### OpenWeather (Optional)

For weather-based recommendations:
```bash
OPENWEATHER_API_KEY=your_api_key
```

Get key: https://openweathermap.org/api

### Algorithm Configuration

Fitness intelligence algorithms configurable via environment variables. Each algorithm has multiple variants with different accuracy, performance, and data requirements.

#### Max Heart Rate Estimation

```bash
PIERRE_MAXHR_ALGORITHM=tanaka  # default
```

**available algorithms**:
- `fox`: Classic 220 - age formula (simple, least accurate)
- `tanaka`: 208 - (0.7 × age) (default, validated in large studies)
- `nes`: 211 - (0.64 × age) (most accurate for fit individuals)
- `gulati`: 206 - (0.88 × age) (gender-specific for females)

#### Training Impulse (TRIMP)

```bash
PIERRE_TRIMP_ALGORITHM=hybrid  # default
```

**available algorithms**:
- `bannister_male`: Exponential formula for males (exp(1.92), requires resting HR)
- `bannister_female`: Exponential formula for females (exp(1.67), requires resting HR)
- `edwards_simplified`: Zone-based TRIMP (5 zones, linear weighting)
- `lucia_banded`: Sport-specific intensity bands (cycling, running)
- `hybrid`: Auto-select Bannister if data available, fallback to Edwards (default)

#### Training Stress Score (TSS)

```bash
PIERRE_TSS_ALGORITHM=avg_power  # default
```

**available algorithms**:
- `avg_power`: Fast calculation using average power (default, always works)
- `normalized_power`: Industry standard using 30s rolling window (requires power stream)
- `hybrid`: Try normalized power, fallback to average power if stream unavailable

#### VDOT (Running Performance)

```bash
PIERRE_VDOT_ALGORITHM=daniels  # default
```

**available algorithms**:
- `daniels`: Jack Daniels' formula (VO2 = -4.60 + 0.182258×v + 0.000104×v²) (default)
- `riegel`: Power-law model (T2 = T1 × (D2/D1)^1.06) (good for ultra distances)
- `hybrid`: Auto-select Daniels for 5K-Marathon, Riegel for ultra distances

#### Training Load (CTL/ATL/TSB)

```bash
PIERRE_TRAINING_LOAD_ALGORITHM=ema  # default
```

**available algorithms**:
- `ema`: Exponential Moving Average (TrainingPeaks standard, CTL=42d, ATL=7d) (default)
- `sma`: Simple Moving Average (equal weights, simpler but less responsive)
- `wma`: Weighted Moving Average (linear weights, compromise between EMA and SMA)
- `kalman`: Kalman Filter (optimal for noisy data, complex tuning)

#### Recovery Aggregation

```bash
PIERRE_RECOVERY_ALGORITHM=weighted  # default
```

**available algorithms**:
- `weighted`: Weighted average with physiological priorities (default)
- `additive`: Simple sum of recovery scores
- `multiplicative`: Product of normalized recovery factors
- `minmax`: Minimum score (conservative, limited by worst metric)
- `neural`: ML-based aggregation (requires training data)

#### Functional Threshold Power (FTP)

```bash
PIERRE_FTP_ALGORITHM=from_vo2max  # default
```

**available algorithms**:
- `20min_test`: 95% of 20-minute max average power (most common field test)
- `8min_test`: 90% of 8-minute max average power (shorter alternative)
- `ramp_test`: Protocol-specific extraction (Zwift, TrainerRoad formats)
- `60min_power`: 100% of 60-minute max average power (gold standard, very difficult)
- `critical_power`: 2-parameter model (requires multiple test durations)
- `from_vo2max`: Estimate from VO2max (FTP = VO2max × 13.5 × fitness_factor) (default)
- `hybrid`: Try best available method based on recent activity data

#### Lactate Threshold Heart Rate (LTHR)

```bash
PIERRE_LTHR_ALGORITHM=from_maxhr  # default
```

**available algorithms**:
- `from_maxhr`: 85-90% of max HR based on fitness level (default, simple)
- `from_30min`: 95-100% of 30-minute test average HR (field test)
- `from_race`: Extract from race efforts (10K-Half Marathon pace)
- `lab_test`: Direct lactate measurement (requires lab equipment)
- `hybrid`: Auto-select best method from available data

#### VO2max Estimation

```bash
PIERRE_VO2MAX_ALGORITHM=from_vdot  # default
```

**available algorithms**:
- `from_vdot`: Calculate from running VDOT (VO2max = VDOT in ml/kg/min) (default)
- `cooper`: 12-minute run test (VO2max = (distance_m - 504.9) / 44.73)
- `rockport`: 1-mile walk test (considers HR, age, gender, weight)
- `astrand`: Submaximal cycle test (requires HR response)
- `bruce`: Treadmill protocol (clinical setting, progressive grades)
- `hybrid`: Auto-select from available test data

**algorithm selection strategy**:
- **default algorithms**: balanced accuracy vs data requirements
- **hybrid algorithms**: defensive programming, fallback to simpler methods
- **specialized algorithms**: higher accuracy but more data/computation required

**configuration example** (.envrc):
```bash
# conservative setup (less data required)
export PIERRE_MAXHR_ALGORITHM=tanaka
export PIERRE_TRIMP_ALGORITHM=edwards_simplified
export PIERRE_TSS_ALGORITHM=avg_power
export PIERRE_RECOVERY_ALGORITHM=weighted

# performance setup (requires more data)
export PIERRE_TRIMP_ALGORITHM=bannister_male
export PIERRE_TSS_ALGORITHM=normalized_power
export PIERRE_TRAINING_LOAD_ALGORITHM=kalman
export PIERRE_RECOVERY_ALGORITHM=neural
```

### Database Configuration

#### sqlite (development)

```bash
DATABASE_URL="sqlite:./data/users.db"
```

Creates database file at path if not exists.

#### PostgreSQL (Production)

```bash
DATABASE_URL="postgresql://user:pass@localhost:5432/pierre"

# connection pool
POSTGRES_MAX_CONNECTIONS=20       # max pool size (default: 20)
POSTGRES_MIN_CONNECTIONS=2        # min pool size (default: 2)
POSTGRES_ACQUIRE_TIMEOUT=30       # connection timeout seconds (default: 30)
```

#### SQLx Pool Configuration

Fine-tune database connection pool behavior for production workloads:

```bash
# connection lifecycle
SQLX_IDLE_TIMEOUT_SECS=600        # close idle connections after (default: 600)
SQLX_MAX_LIFETIME_SECS=1800       # max connection lifetime (default: 1800)

# connection validation
SQLX_TEST_BEFORE_ACQUIRE=true     # validate before use (default: true)

# performance
SQLX_STATEMENT_CACHE_CAPACITY=100 # prepared statement cache (default: 100)
```

### Tokio Runtime Configuration

Configure async runtime for performance tuning:

```bash
# worker threads (default: number of CPU cores)
TOKIO_WORKER_THREADS=4

# thread stack size in bytes (default: OS default)
TOKIO_THREAD_STACK_SIZE=2097152   # 2MB

# worker thread name prefix (default: pierre-worker)
TOKIO_THREAD_NAME=pierre-worker
```

### Cache Configuration

```bash
# cache configuration (in-memory or redis)
CACHE_MAX_ENTRIES=10000           # max cached items for in-memory (default: 10,000)
CACHE_CLEANUP_INTERVAL_SECS=300   # cleanup interval in seconds (default: 300)

# redis cache (optional - uses in-memory if not set)
REDIS_URL=redis://localhost:6379  # redis connection url
```

### Rate Limiting

```bash
# burst limits per tier (requests in short window)
RATE_LIMIT_FREE_TIER_BURST=100        # default: 100
RATE_LIMIT_PROFESSIONAL_BURST=500     # default: 500
RATE_LIMIT_ENTERPRISE_BURST=2000      # default: 2000

# OAuth2 endpoint rate limits (requests per minute)
OAUTH_AUTHORIZE_RATE_LIMIT_RPM=60     # default: 60
OAUTH_TOKEN_RATE_LIMIT_RPM=30         # default: 30
OAUTH_REGISTER_RATE_LIMIT_RPM=10      # default: 10

# Admin-provisioned API key monthly limit (Starter tier default)
PIERRE_ADMIN_API_KEY_MONTHLY_LIMIT=10000
```

### Multi-Tenancy

```bash
# tenant isolation
TENANT_MAX_USERS=100              # max users per tenant
TENANT_MAX_PROVIDERS=5            # max connected providers per tenant

# default features per tenant
TENANT_DEFAULT_FEATURES="activity_analysis,goal_tracking"
```

### Security

```bash
# cors
CORS_ALLOWED_ORIGINS="http://localhost:3000,http://localhost:5173"
CORS_MAX_AGE=3600

# csrf protection
CSRF_TOKEN_EXPIRY=3600            # seconds

# tls (production)
TLS_CERT_PATH=/path/to/cert.pem
TLS_KEY_PATH=/path/to/key.pem
```

## Fitness Configuration

User-specific fitness parameters managed via mcp tools or rest api.

### Configuration Profiles

Predefined fitness profiles:

- `beginner`: conservative zones, longer recovery
- `intermediate`: standard zones, moderate training
- `advanced`: aggressive zones, high training load
- `elite`: performance-optimized zones
- `custom`: user-defined parameters

### Fitness Parameters

```json
{
  "profile": "advanced",
  "vo2_max": 55.0,
  "max_heart_rate": 185,
  "resting_heart_rate": 45,
  "threshold_heart_rate": 170,
  "threshold_power": 280,
  "threshold_pace": 240,
  "weight_kg": 70.0,
  "height_cm": 175
}
```

### Training Zones

Automatically calculated based on profile:

```json
{
  "heart_rate_zones": [
    {"zone": 1, "min_bpm": 93, "max_bpm": 111},
    {"zone": 2, "min_bpm": 111, "max_bpm": 130},
    {"zone": 3, "min_bpm": 130, "max_bpm": 148},
    {"zone": 4, "min_bpm": 148, "max_bpm": 167},
    {"zone": 5, "min_bpm": 167, "max_bpm": 185}
  ],
  "power_zones": [
    {"zone": 1, "min_watts": 0, "max_watts": 154},
    {"zone": 2, "min_watts": 154, "max_watts": 210},
    ...
  ]
}
```

### Updating Configuration

Via mcp tool:
```json
{
  "tool": "update_user_configuration",
  "parameters": {
    "profile": "elite",
    "vo2_max": 60.0,
    "threshold_power": 300
  }
}
```

Via rest api:
```bash
curl -X PUT http://localhost:8081/api/configuration/user \
  -H "Authorization: Bearer <jwt>" \
  -H "Content-Type: application/json" \
  -d '{
    "profile": "elite",
    "vo2_max": 60.0
  }'
```

### Configuration Catalog

Get all available parameters:
```bash
curl -H "Authorization: Bearer <jwt>" \
  http://localhost:8081/api/configuration/catalog
```

Response describes each parameter:
- type (number, boolean, enum)
- valid range
- default value
- description

## Using direnv

Recommended for local development.

### Setup

```bash
brew install direnv

# add to shell (~/.zshrc or ~/.bashrc)
eval "$(direnv hook zsh)"  # or bash

# in project directory
direnv allow
```

### .envrc File

Edit `.envrc` in project root:
```bash
# development overrides
export RUST_LOG=debug
export HTTP_PORT=8081
export DATABASE_URL=sqlite:./data/users.db

# provider credentials (dev)
export STRAVA_CLIENT_ID=dev_client_id
export STRAVA_CLIENT_SECRET=dev_secret
export STRAVA_REDIRECT_URI=http://localhost:8081/api/oauth/callback/strava

# load from file
if [ -f .env.local ]; then
  source .env.local
fi
```

Direnv automatically loads/unloads environment when entering/leaving directory.

### .env.local (Gitignored)

Store secrets in `.env.local`:
```bash
# never commit this file
export PIERRE_MASTER_ENCRYPTION_KEY="<generated_key>"
export STRAVA_CLIENT_SECRET="<real_secret>"
```

## Production Deployment

### environment file

Create `/etc/pierre/environment`:
```bash
DATABASE_URL=postgresql://pierre:pass@db.internal:5432/pierre
PIERRE_MASTER_ENCRYPTION_KEY=<strong_key>
HTTP_PORT=8081
HOST=0.0.0.0
LOG_FORMAT=json
RUST_LOG=info

# provider credentials from secrets manager
STRAVA_CLIENT_ID=prod_id
STRAVA_CLIENT_SECRET=prod_secret
STRAVA_REDIRECT_URI=https://api.example.com/api/oauth/callback/strava

# tls
TLS_CERT_PATH=/etc/pierre/tls/cert.pem
TLS_KEY_PATH=/etc/pierre/tls/key.pem

# postgres
POSTGRES_MAX_CONNECTIONS=50
POSTGRES_MIN_CONNECTIONS=5

# cache
CACHE_MAX_ENTRIES=50000

# rate limiting
RATE_LIMIT_REQUESTS_PER_MINUTE=120
```

### systemd Service

```ini
[Unit]
Description=Pierre MCP Server
After=network.target postgresql.service

[Service]
Type=simple
User=pierre
Group=pierre
WorkingDirectory=/opt/pierre
EnvironmentFile=/etc/pierre/environment
ExecStart=/opt/pierre/bin/pierre-mcp-server
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

### Docker

```dockerfile
FROM rust:1.70 as builder
WORKDIR /build
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/pierre-mcp-server /usr/local/bin/

ENV HTTP_PORT=8081
ENV DATABASE_URL=postgresql://pierre:pass@db:5432/pierre

EXPOSE 8081
CMD ["pierre-mcp-server"]
```

Run:
```bash
docker run -d \
  --name pierre \
  -p 8081:8081 \
  -e DATABASE_URL=postgresql://... \
  -e PIERRE_MASTER_ENCRYPTION_KEY=... \
  pierre:latest
```

## Validation

Check configuration at startup:
```bash
RUST_LOG=info cargo run --bin pierre-mcp-server
```

Logs show:
- loaded environment variables
- database connection status
- enabled features
- configured providers
- listening address

## Troubleshooting

### missing environment variables

Server fails to start. Check required variables set:
```bash
echo $DATABASE_URL
echo $PIERRE_MASTER_ENCRYPTION_KEY
```

### Invalid Database URL

- sqlite: ensure directory exists
- postgresql: verify connection string, credentials, database exists

### Provider OAuth Fails

- verify redirect uri exactly matches environment variable
- ensure uri accessible from browser (not `127.0.0.1` for remote)
- check provider console for correct credentials

### Port Conflicts

Change http_port:
```bash
export HTTP_PORT=8082
```

### Encryption Key Errors

Regenerate:
```bash
openssl rand -base64 32
```

Must be exactly 32 bytes (base64 encoded = 44 characters).

## References

All configuration constants: `src/constants/mod.rs`
Fitness profiles: `src/configuration/profiles.rs`
Database setup: `src/database_plugins/`

---

# Environment Configuration

Environment variables for Pierre Fitness Platform. Copy `.envrc.example` to `.envrc` and customize.

## Setup

```bash
cp .envrc.example .envrc
# edit .envrc with your settings
direnv allow  # or: source .envrc
```

## Required Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `DATABASE_URL` | Database connection string | `sqlite:./data/users.db` |
| `PIERRE_MASTER_ENCRYPTION_KEY` | Master encryption key (base64) | `openssl rand -base64 32` |

## Server Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `HTTP_PORT` | `8081` | Server port |
| `RUST_LOG` | `info` | Log level (debug, info, warn, error) |
| `JWT_EXPIRY_HOURS` | `24` | JWT token expiration |
| `PIERRE_RSA_KEY_SIZE` | `4096` | RSA key size (2048 for dev, 4096 for prod) |

## Database

### SQLite (Development)

```bash
export DATABASE_URL="sqlite:./data/users.db"
```

### PostgreSQL (Production)

```bash
export DATABASE_URL="postgresql://user:pass@localhost/pierre_db"
export POSTGRES_MAX_CONNECTIONS="10"
export POSTGRES_MIN_CONNECTIONS="0"
export POSTGRES_ACQUIRE_TIMEOUT="30"
```

### SQLx Pool Configuration

Fine-tune database connection pool behavior:

| Variable | Default | Description |
|----------|---------|-------------|
| `SQLX_IDLE_TIMEOUT_SECS` | `600` | Seconds before idle connections are closed |
| `SQLX_MAX_LIFETIME_SECS` | `1800` | Maximum connection lifetime in seconds |
| `SQLX_TEST_BEFORE_ACQUIRE` | `true` | Validate connections before use |
| `SQLX_STATEMENT_CACHE_CAPACITY` | `100` | Prepared statement cache size |

## Tokio Runtime Configuration

Configure the async runtime for performance tuning:

| Variable | Default | Description |
|----------|---------|-------------|
| `TOKIO_WORKER_THREADS` | CPU cores | Number of worker threads |
| `TOKIO_THREAD_STACK_SIZE` | OS default | Thread stack size in bytes |
| `TOKIO_THREAD_NAME` | `pierre-worker` | Worker thread name prefix |

## Provider Configuration

### Default Provider

```bash
export PIERRE_DEFAULT_PROVIDER=strava  # strava, garmin, synthetic
```

### Strava

```bash
# required for strava oauth
export PIERRE_STRAVA_CLIENT_ID=your-client-id
export PIERRE_STRAVA_CLIENT_SECRET=your-client-secret

# legacy variables (backward compatible)
export STRAVA_CLIENT_ID=your-client-id
export STRAVA_CLIENT_SECRET=your-client-secret
export STRAVA_REDIRECT_URI=http://localhost:8081/api/oauth/callback/strava
```

### Garmin

```bash
# required for garmin oauth
export PIERRE_GARMIN_CLIENT_ID=your-consumer-key
export PIERRE_GARMIN_CLIENT_SECRET=your-consumer-secret

# legacy variables (backward compatible)
export GARMIN_CLIENT_ID=your-consumer-key
export GARMIN_CLIENT_SECRET=your-consumer-secret
export GARMIN_REDIRECT_URI=http://localhost:8081/api/oauth/callback/garmin
```

### Fitbit

```bash
export FITBIT_CLIENT_ID=your-client-id
export FITBIT_CLIENT_SECRET=your-client-secret
export FITBIT_REDIRECT_URI=http://localhost:8081/api/oauth/callback/fitbit
```

### WHOOP

```bash
export WHOOP_CLIENT_ID=your-client-id
export WHOOP_CLIENT_SECRET=your-client-secret
export WHOOP_REDIRECT_URI=http://localhost:8081/api/oauth/callback/whoop
```

### Terra (150+ Wearables)

```bash
export TERRA_API_KEY=your-api-key
export TERRA_DEV_ID=your-dev-id
export TERRA_WEBHOOK_SECRET=your-webhook-secret
```

### Synthetic (No Credentials Needed)

```bash
export PIERRE_DEFAULT_PROVIDER=synthetic
# no oauth credentials required - works out of the box
```

## Algorithm Configuration

Configure fitness calculation algorithms via environment variables.

| Variable | Default | Options |
|----------|---------|---------|
| `PIERRE_MAXHR_ALGORITHM` | `tanaka` | fox, tanaka, nes, gulati |
| `PIERRE_TRIMP_ALGORITHM` | `hybrid` | bannister_male, bannister_female, edwards_simplified, lucia_banded, hybrid |
| `PIERRE_TSS_ALGORITHM` | `avg_power` | avg_power, normalized_power, hybrid |
| `PIERRE_VDOT_ALGORITHM` | `daniels` | daniels, riegel, hybrid |
| `PIERRE_TRAINING_LOAD_ALGORITHM` | `ema` | ema, sma, wma, kalman |
| `PIERRE_RECOVERY_ALGORITHM` | `weighted` | weighted, additive, multiplicative, minmax, neural |
| `PIERRE_FTP_ALGORITHM` | `from_vo2max` | 20min_test, 8min_test, ramp_test, from_vo2max, hybrid |
| `PIERRE_LTHR_ALGORITHM` | `from_maxhr` | from_maxhr, from_30min, from_race, lab_test, hybrid |
| `PIERRE_VO2MAX_ALGORITHM` | `from_vdot` | from_vdot, cooper, rockport, astrand, bruce, hybrid |

See configuration.md for algorithm details.

## Fitness Configuration

### Effort Thresholds (1-10 Scale)

```bash
export FITNESS_EFFORT_LIGHT_MAX="3.0"
export FITNESS_EFFORT_MODERATE_MAX="5.0"
export FITNESS_EFFORT_HARD_MAX="7.0"
# > 7.0 = very_high
```

### Heart Rate Zone Thresholds (% of Max HR)

```bash
export FITNESS_ZONE_RECOVERY_MAX="60.0"
export FITNESS_ZONE_ENDURANCE_MAX="70.0"
export FITNESS_ZONE_TEMPO_MAX="80.0"
export FITNESS_ZONE_THRESHOLD_MAX="90.0"
# > 90.0 = vo2max
```

### Personal Records

```bash
export FITNESS_PR_PACE_IMPROVEMENT_THRESHOLD="5.0"
```

## Weather Integration

```bash
export OPENWEATHER_API_KEY="your-api-key"
export FITNESS_WEATHER_ENABLED="true"
export FITNESS_WEATHER_WIND_THRESHOLD="15.0"
export FITNESS_WEATHER_CACHE_DURATION_HOURS="24"
export FITNESS_WEATHER_REQUEST_TIMEOUT_SECONDS="10"
export FITNESS_WEATHER_RATE_LIMIT_PER_MINUTE="60"
```

## Rate Limiting

```bash
export RATE_LIMIT_ENABLED="true"
export RATE_LIMIT_REQUESTS="100"
export RATE_LIMIT_WINDOW="60"  # seconds
```

## Cache Configuration

```bash
export CACHE_MAX_ENTRIES="10000"
export CACHE_CLEANUP_INTERVAL_SECS="300"
export REDIS_URL="redis://localhost:6379"  # optional, uses in-memory if not set
```

## Backup Configuration

```bash
export BACKUP_ENABLED="true"
export BACKUP_INTERVAL="21600"  # 6 hours in seconds
export BACKUP_RETENTION="7"      # days
export BACKUP_DIRECTORY="./backups"
```

## Activity Limits

```bash
export MAX_ACTIVITIES_FETCH="100"
export DEFAULT_ACTIVITIES_LIMIT="20"
```

## OAuth Callback

```bash
export OAUTH_CALLBACK_PORT="35535"  # bridge callback port for focus recovery
```

## Development Defaults

For dev/test only (leave empty in production):

```bash
# Regular user defaults (for OAuth login form)
export OAUTH_DEFAULT_EMAIL="user@example.com"
export OAUTH_DEFAULT_PASSWORD="userpass123"

# Admin user defaults (for setup scripts)
export ADMIN_EMAIL="admin@pierre.mcp"
export ADMIN_PASSWORD="adminpass123"
```

## Frontend Configuration

```bash
export VITE_BACKEND_URL="http://localhost:8081"
```

## Production vs Development

| Setting | Development | Production |
|---------|-------------|------------|
| `DATABASE_URL` | sqlite | postgresql |
| `PIERRE_RSA_KEY_SIZE` | 2048 | 4096 |
| `RUST_LOG` | debug | info |
| Redirect URIs | http://localhost:... | https://... |
| `OAUTH_DEFAULT_*` | set | empty |

## Security Notes

- Never commit `.envrc` (gitignored)
- Use HTTPS redirect URIs in production
- Generate unique `PIERRE_MASTER_ENCRYPTION_KEY` per environment
- Rotate provider credentials periodically

---

# Development

Development workflow, tools, and dashboard setup for Pierre Fitness Platform.

## Server Management

### Startup Scripts

```bash
./bin/start-server.sh     # start backend (loads .envrc, port 8081)
./bin/stop-server.sh      # stop backend (graceful shutdown)
./bin/start-frontend.sh   # start dashboard (port 5173)
```

### Manual Startup

```bash
# backend
cargo run --bin pierre-mcp-server

# frontend (separate terminal)
cd frontend && npm run dev
```

## Development Workflow

### Fresh Start

```bash
# clean database and start fresh
./scripts/fresh-start.sh
./bin/start-server.sh &

# run complete setup (admin + user + tenant + MCP test)
./scripts/complete-user-workflow.sh

# load saved credentials
source .workflow_test_env
echo "JWT Token: ${JWT_TOKEN:0:50}..."
```

### Automated Setup Script

`./scripts/complete-user-workflow.sh` creates:
- Admin user: `$ADMIN_EMAIL` (default: `admin@pierre.mcp`)
- Regular user: `$OAUTH_DEFAULT_EMAIL` (default: `user@example.com`)
- Default tenant: `User Organization`
- JWT token (saved in `.workflow_test_env`)

## Management Dashboard

React + Vite web dashboard for monitoring and administration.

### Quick Start

```bash
# terminal 1: backend
./bin/start-server.sh

# terminal 2: frontend
./bin/start-frontend.sh
```

Access at `http://localhost:5173`

### Features

- **Role-Based Access**: super_admin, admin, user roles with permission hierarchy
- **User Registration**: Self-registration with admin approval workflow
- **User Management**: Registration approval, tenant management
- **API Keys**: Generate API keys for Claude Desktop, AI assistants, and programmatic access
- **Usage Analytics**: Request patterns, tool usage charts (282 E2E tests)
- **Real-time Updates**: WebSocket-based live data
- **OAuth Status**: Provider connection monitoring
- **Super Admin Impersonation**: View dashboard as any user for support

### Manual Setup

```bash
cd frontend
npm install
npm run dev
```

### Environment

Add to `.envrc` for custom backend URL:
```bash
export VITE_BACKEND_URL="http://localhost:8081"
```

See frontend/README.md for detailed frontend documentation.

## Admin Tools

### admin-setup Binary

Manage admin users and API tokens:

```bash
# create admin user for frontend login
cargo run --bin admin-setup -- create-admin-user \
  --email admin@example.com \
  --password SecurePassword123

# generate API token for a service
cargo run --bin admin-setup -- generate-token \
  --service my_service \
  --expires-days 30

# generate super admin token (no expiry, all permissions)
cargo run --bin admin-setup -- generate-token \
  --service admin_console \
  --super-admin

# list all admin tokens
cargo run --bin admin-setup -- list-tokens --detailed

# revoke a token
cargo run --bin admin-setup -- revoke-token <token_id>
```

### curl-based Setup

```bash
# create admin (first run only)
curl -X POST http://localhost:8081/admin/setup \
  -H "Content-Type: application/json" \
  -d '{
    "email": "admin@example.com",
    "password": "SecurePass123!",
    "display_name": "Admin"
  }'

# register user (requires admin token)
curl -X POST http://localhost:8081/api/auth/register \
  -H "Authorization: Bearer {admin_token}" \
  -H "Content-Type: application/json" \
  -d "{
    \"email\": \"$OAUTH_DEFAULT_EMAIL\",
    \"password\": \"$OAUTH_DEFAULT_PASSWORD\",
    \"display_name\": \"User\"
  }"

# approve user (requires admin token)
curl -X POST http://localhost:8081/admin/approve-user/{user_id} \
  -H "Authorization: Bearer {admin_token}" \
  -H "Content-Type: application/json" \
  -d '{
    "reason": "Approved",
    "create_default_tenant": true,
    "tenant_name": "User Org",
    "tenant_slug": "user-org"
  }'
```

## Testing

### Quick Validation

```bash
./scripts/smoke-test.sh           # ~3 minutes
./scripts/fast-tests.sh           # ~5 minutes
./scripts/pre-push-tests.sh       # ~10 minutes
```

### Full Test Suite

```bash
cargo test                        # all tests (~13 min)
./scripts/lint-and-test.sh        # full CI suite
```

### Targeted Testing

```bash
cargo test test_training_load     # by test name
cargo test --test intelligence_test  # by test file
cargo test intelligence::         # by module path
cargo test <pattern> -- --nocapture  # with output
```

See testing.md for comprehensive testing documentation.

## Validation

### Pre-commit Checklist

```bash
cargo fmt                              # format code
./scripts/architectural-validation.sh  # architectural patterns
cargo clippy -- -D warnings            # linting
cargo test <relevant_tests>            # targeted tests
```

### CI Validation

```bash
./scripts/lint-and-test.sh        # runs everything CI runs
```

## Scripts Reference

30+ scripts in `scripts/` directory:

| Category | Scripts |
|----------|---------|
| **Development** | `dev-start.sh`, `fresh-start.sh` |
| **Testing** | `smoke-test.sh`, `fast-tests.sh`, `safe-test-runner.sh` |
| **Validation** | `architectural-validation.sh`, `lint-and-test.sh` |
| **Deployment** | `deploy.sh` |
| **SDK** | `generate-sdk-types.js`, `run_bridge_tests.sh` |

See scripts/README.md for complete documentation.

## Debugging

### Server Logs

```bash
# real-time logs
RUST_LOG=debug cargo run --bin pierre-mcp-server

# log to file
./bin/start-server.sh  # logs to server.log
```

### SDK Debugging

```bash
npx pierre-mcp-client@next --server http://localhost:8081 --verbose
```

### Health Check

```bash
curl http://localhost:8081/health
```

## Database

### SQLite (Development)

```bash
# location
./data/users.db

# reset
./scripts/fresh-start.sh
```

### PostgreSQL (Production)

```bash
# test postgresql integration
./scripts/test-postgres.sh
```

See configuration.md for database configuration.

---

