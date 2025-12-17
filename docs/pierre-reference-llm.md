# Pierre MCP Server - Reference Documentation

> Comprehensive reference documentation optimized for LLM/ChatGPT consumption.
> For the tutorial, see pierre-tutorial-llm.md

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

# Authentication

Pierre supports multiple authentication methods for different use cases.

## Authentication Methods

| method | use case | header | endpoints |
|--------|----------|--------|-----------|
| jwt tokens | mcp clients, web apps | `Authorization: Bearer <token>` | all authenticated endpoints |
| api keys | a2a systems | `X-API-Key: <key>` | a2a endpoints |
| oauth2 | provider integration | varies | fitness provider apis |

## JWT Authentication

### Registration

```bash
curl -X POST http://localhost:8081/api/auth/register \
  -H "Authorization: Bearer <admin_jwt_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "SecurePass123!",
    "display_name": "User Name"
  }'
```

Response:
```json
{
  "user_id": "uuid",
  "email": "user@example.com",
  "token": "jwt_token",
  "expires_at": "2024-01-01T00:00:00Z"
}
```

### Login

Uses OAuth2 Resource Owner Password Credentials (ROPC) flow:

```bash
curl -X POST http://localhost:8081/oauth/token \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=password&username=user@example.com&password=SecurePass123!"
```

Response includes jwt_token. Store securely.

### Using JWT Tokens

Include in authorization header:
```bash
curl -H "Authorization: Bearer <jwt_token>" \
  http://localhost:8081/mcp
```

### Token Expiry

Default: 24 hours (configurable via `JWT_EXPIRY_HOURS`)

Refresh before expiry:
```bash
curl -X POST http://localhost:8081/api/auth/refresh \
  -H "Authorization: Bearer <current_token>"
```

## API Key Authentication

For a2a systems and service-to-service communication.

### Creating API Keys

Requires admin or user jwt:
```bash
curl -X POST http://localhost:8081/api/keys \
  -H "Authorization: Bearer <jwt_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "My A2A System",
    "tier": "professional"
  }'
```

Response:
```json
{
  "api_key": "generated_key",
  "name": "My A2A System",
  "tier": "professional",
  "created_at": "2024-01-01T00:00:00Z"
}
```

Save api key - cannot be retrieved later.

### Using API Keys

```bash
curl -H "X-API-Key: <api_key>" \
  http://localhost:8081/a2a/tools
```

### API Key Tiers

- `trial`: 1,000 requests/month (auto-expires after 14 days)
- `starter`: 10,000 requests/month
- `professional`: 100,000 requests/month
- `enterprise`: unlimited (no fixed monthly cap)

Rate limits are enforced per API key over a rolling 30-day window.

## OAuth2 (MCP Client Authentication)

Pierre acts as oauth2 authorization server for mcp clients.

### OAuth2 vs OAuth (Terminology)

Pierre implements two oauth systems:

1. **oauth2_server module** (`src/oauth2_server/`): pierre AS oauth2 server
   - mcp clients authenticate TO pierre
   - rfc 7591 dynamic client registration
   - issues jwt access tokens

2. **oauth2_client module** (`src/oauth2_client/`): pierre AS oauth2 client
   - pierre authenticates TO fitness providers (strava, garmin, fitbit, whoop)
   - manages provider tokens
   - handles token refresh

### OAuth2 Flow (MCP Clients)

Sdk handles automatically. Manual flow:

1. **register client**:
```bash
curl -X POST http://localhost:8081/oauth2/register \
  -H "Content-Type: application/json" \
  -d '{
    "redirect_uris": ["http://localhost:35535/oauth/callback"],
    "client_name": "My MCP Client",
    "grant_types": ["authorization_code"]
  }'
```

2. **authorization** (browser):
```
http://localhost:8081/oauth2/authorize?
  client_id=<client_id>&
  redirect_uri=<redirect_uri>&
  response_type=code&
  code_challenge=<sha256_base64url(verifier)>&
  code_challenge_method=S256
```

3. **token exchange**:
```bash
curl -X POST http://localhost:8081/oauth2/token \
  -d "grant_type=authorization_code&\
      code=<code>&\
      client_id=<client_id>&\
      client_secret=<client_secret>&\
      code_verifier=<verifier>"
```

Receives jwt access token.

### PKCE Enforcement

Pierre requires pkce (rfc 7636) for security:
- code verifier: 43-128 random characters
- code challenge: base64url(sha256(verifier))
- challenge method: S256 only

No plain text challenge methods allowed.

## MCP Client Integration (Claude Code, VS Code, etc.)

mcp clients (claude code, vs code with cline/continue, cursor, etc.) connect to pierre via http-based mcp protocol.

### Authentication Flow

1. **user registration and login**:
```bash
# create user account
curl -X POST http://localhost:8081/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "SecurePass123!"
  }'

# login to get jwt token (OAuth2 ROPC flow)
curl -X POST http://localhost:8081/oauth/token \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=password&username=user@example.com&password=SecurePass123!"
```

response includes jwt token:
```json
{
  "jwt_token": "eyJ0eXAiOiJKV1Qi...",
  "expires_at": "2025-11-05T18:00:00Z",
  "user": {
    "id": "75059e8b-1f56-4fcf-a14e-860966783c93",
    "email": "user@example.com"
  }
}
```

2. **configure mcp client**:

option a: **claude code** - using `/mcp` command (interactive):
```bash
# in claude code session
/mcp add pierre-production \
  --url http://localhost:8081/mcp \
  --transport http \
  --header "Authorization: Bearer eyJ0eXAiOiJKV1Qi..."
```

manual configuration (`~/.config/claude-code/mcp_config.json`):
```json
{
  "mcpServers": {
    "pierre-production": {
      "url": "http://localhost:8081/mcp",
      "transport": "http",
      "headers": {
        "Authorization": "Bearer eyJ0eXAiOiJKV1Qi..."
      }
    }
  }
}
```

option b: **vs code** (cline, continue, cursor) - edit settings:

for cline extension (`~/.vscode/settings.json` or workspace settings):
```json
{
  "cline.mcpServers": {
    "pierre-production": {
      "url": "http://localhost:8081/mcp",
      "transport": "http",
      "headers": {
        "Authorization": "Bearer eyJ0eXAiOiJKV1Qi..."
      }
    }
  }
}
```

for continue extension:
```json
{
  "continue.mcpServers": [{
    "url": "http://localhost:8081/mcp",
    "headers": {
      "Authorization": "Bearer eyJ0eXAiOiJKV1Qi..."
    }
  }]
}
```

3. **automatic authentication**:

mcp clients include jwt token in all mcp requests:
```http
POST /mcp HTTP/1.1
Host: localhost:8081
Authorization: Bearer eyJ0eXAiOiJKV1Qi...
Content-Type: application/json

{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "connect_provider",
    "arguments": {"provider": "strava"}
  }
}
```

pierre's mcp server validates jwt on every request:
- extracts user_id from token
- validates signature using jwks
- checks expiration
- enforces rate limits per tenant

### MCP Endpoint Authentication Requirements

| endpoint | auth required | notes |
|----------|---------------|-------|
| `POST /mcp` (initialize) | no | discovery only |
| `POST /mcp` (tools/list) | no | unauthenticated tool listing |
| `POST /mcp` (tools/call) | yes | requires valid jwt |
| `POST /mcp` (prompts/list) | no | discovery only |
| `POST /mcp` (resources/list) | no | discovery only |

implementation: `src/mcp/multitenant.rs:1726`

### Token Expiry and Refresh

jwt tokens expire after 24 hours (default, configurable via `JWT_EXPIRY_HOURS`).

when token expires, user must:
1. login again to get new jwt token
2. update claude code configuration with new token

automatic refresh not implemented in most mcp clients (requires manual re-login).

### Connecting to Fitness Providers

once authenticated to pierre, connect to fitness providers:

1. **using mcp tool** (recommended):
```
user: "connect to strava"
```

mcp client calls `connect_provider` tool with jwt authentication:
- pierre validates jwt, extracts user_id
- generates oauth authorization url for that user_id
- opens browser for strava authorization
- callback stores strava token for user_id
- **no pierre login required** - user already authenticated via jwt!

2. **via rest api**:
```bash
curl -H "Authorization: Bearer <jwt>" \
  http://localhost:8081/api/oauth/auth/strava/<user_id>
```

### Why No Pierre Login During Strava OAuth?

common question: "why don't i need to log into pierre when connecting to strava?"

**answer**: you're already authenticated!

sequence:
1. you logged into pierre (got jwt token)
2. configured your mcp client (claude code, vs code, cursor, etc.) with jwt token
3. mcp client includes jwt in every mcp request
4. when you say "connect to strava":
   - mcp client sends `tools/call` with jwt
   - pierre extracts user_id from jwt (e.g., `75059e8b-1f56-4fcf-a14e-860966783c93`)
   - generates oauth url: `http://localhost:8081/api/oauth/auth/strava/75059e8b-1f56-4fcf-a14e-860966783c93`
   - state parameter includes user_id: `75059e8b-1f56-4fcf-a14e-860966783c93:random_nonce`
5. browser opens strava authorization (you prove you own the strava account)
6. strava redirects to callback with code
7. pierre validates state, exchanges code for token
8. stores strava token for your user_id (from jwt)

**key insight**: jwt token proves your identity to pierre. strava oauth proves you own the fitness account. no duplicate login needed.

### Security Considerations

**jwt token storage**: mcp clients store jwt tokens in configuration files:
- claude code: `~/.config/claude-code/mcp_config.json`
- vs code extensions: `.vscode/settings.json` or user settings

these files should have restricted permissions (chmod 600 for config files).

**token exposure**: jwt tokens in config files are sensitive. treat like passwords:
- don't commit to version control
- don't share tokens
- rotate regularly (re-login to get new token)
- revoke if compromised

**oauth state validation**: pierre validates oauth state parameters to prevent:
- csrf attacks (random nonce verified)
- user_id spoofing (state must match authenticated user)
- replay attacks (state used once)

**implementation**: `src/routes/auth.rs`, `src/mcp/multitenant.rs`

### Troubleshooting

**"authentication required" error**:
- check jwt token in your mcp client's configuration file
  - claude code: `~/.config/claude-code/mcp_config.json`
  - vs code: `.vscode/settings.json`
- verify token not expired (24h default)
- confirm token format: `Bearer eyJ0eXAi...`

**"invalid token" error**:
- token may be expired - login again
- token signature invalid - check `PIERRE_MASTER_ENCRYPTION_KEY`
- user account may be disabled - check user status

**fitness provider connection fails**:
- check oauth credentials (client_id, client_secret) at server startup
- verify redirect_uri matches provider registration
- see oauth credential validation logs for fingerprint debugging

**oauth credential debugging**:

pierre validates oauth credentials at startup and logs fingerprints:
```
OAuth provider strava: enabled=true, client_id=163846,
  secret_length=40, secret_fingerprint=f3c0d77f
```

use fingerprints to compare secrets without exposing actual values:
```bash
# check correct secret
echo -n "0f2b184c076e60a35e8ced43db9c3c20c5fcf4f3" | \
  sha256sum | cut -c1-8
# output: f3c0d77f ← correct

# check wrong secret
echo -n "1dfc45ad0a1f6983b835e4495aa9473d111d03bc" | \
  sha256sum | cut -c1-8
# output: 79092abb ← wrong!
```

if fingerprints don't match, you're using wrong credentials.

## Provider OAuth (Fitness Data)

Pierre acts as oauth client to fitness providers.

### Supported Providers

- strava (oauth2)
- garmin (oauth1 + oauth2)
- fitbit (oauth2)

### Configuration

Set environment variables:
```bash
# strava (local development)
export STRAVA_CLIENT_ID=your_id
export STRAVA_CLIENT_SECRET=your_secret
export STRAVA_REDIRECT_URI=http://localhost:8081/api/oauth/callback/strava  # local dev only

# strava (production)
export STRAVA_REDIRECT_URI=https://api.example.com/api/oauth/callback/strava  # required

# garmin (local development)
export GARMIN_CLIENT_ID=your_key
export GARMIN_CLIENT_SECRET=your_secret
export GARMIN_REDIRECT_URI=http://localhost:8081/api/oauth/callback/garmin  # local dev only

# garmin (production)
export GARMIN_REDIRECT_URI=https://api.example.com/api/oauth/callback/garmin  # required
```

**callback url security requirements**:
- http urls: local development only (localhost/127.0.0.1)
- https urls: required for production deployments
- failure to use https in production:
  - authorization codes transmitted unencrypted
  - vulnerable to token interception
  - most providers reject http callbacks in production

### Connecting Providers

Via mcp tool:
```
user: "connect to strava"
```

Or via rest api:
```bash
curl -H "Authorization: Bearer <jwt>" \
  http://localhost:8081/oauth/connect/strava
```

Opens browser for provider authentication. After approval, redirected to callback:
```bash
# local development
http://localhost:8081/api/oauth/callback/strava?code=<auth_code>

# production (https required)
https://api.example.com/api/oauth/callback/strava?code=<auth_code>
```

Pierre exchanges code for access/refresh tokens, stores encrypted.

**security**: authorization codes in callback urls must be protected with tls in production. Http callbacks leak codes to network observers.

### Token Storage

Provider tokens stored encrypted in database:
- encryption key: tenant-specific key (derived from master key)
- algorithm: aes-256-gcm
- rotation: automatic refresh before expiry

### Checking Connection Status

```bash
curl -H "Authorization: Bearer <jwt>" \
  http://localhost:8081/oauth/status
```

Response:
```json
{
  "connected_providers": ["strava"],
  "strava": {
    "connected": true,
    "expires_at": "2024-01-01T00:00:00Z"
  },
  "garmin": {
    "connected": false
  }
}
```

## Web Application Security

### Cookie-Based Authentication (Production Web Apps)

Pierre implements secure cookie-based authentication for web applications using httpOnly cookies with CSRF protection.

#### Security Model

**httpOnly cookies** prevent JavaScript access to JWT tokens, eliminating XSS-based token theft:
```
Set-Cookie: auth_token=<jwt>; HttpOnly; Secure; SameSite=Strict; Max-Age=86400
```

**CSRF protection** uses double-submit cookie pattern with cryptographic tokens:
```
Set-Cookie: csrf_token=<token>; Secure; SameSite=Strict; Max-Age=1800
X-CSRF-Token: <token>  (sent in request header)
```

#### Cookie Security Flags

| flag | value | purpose |
|------|-------|---------|
| HttpOnly | true | prevents JavaScript access (XSS protection) |
| Secure | true | requires HTTPS (prevents sniffing) |
| SameSite | Strict | prevents cross-origin requests (CSRF mitigation) |
| Max-Age | 86400 (auth), 1800 (csrf) | automatic expiration |

#### Authentication Flow

**login** (`POST /oauth/token` - OAuth2 ROPC flow):
```bash
curl -X POST http://localhost:8081/oauth/token \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=password&username=user@example.com&password=SecurePass123!"
```

response sets two cookies and returns csrf token:
```json
{
  "jwt_token": "eyJ0eXAiOiJKV1Qi...",  // deprecated, for backward compatibility
  "csrf_token": "cryptographic_random_32bytes",
  "user": {"id": "uuid", "email": "user@example.com"},
  "expires_at": "2025-01-20T18:00:00Z"
}
```

cookies set automatically:
```
Set-Cookie: auth_token=eyJ0eXAiOiJKV1Qi...; HttpOnly; Secure; SameSite=Strict; Max-Age=86400
Set-Cookie: csrf_token=cryptographic_random_32bytes; Secure; SameSite=Strict; Max-Age=1800
```

**authenticated requests**:

browsers automatically include cookies. web apps must include csrf token header:
```bash
curl -X POST http://localhost:8081/api/something \
  -H "X-CSRF-Token: cryptographic_random_32bytes" \
  -H "Cookie: auth_token=...; csrf_token=..." \
  -d '{"data": "value"}'
```

server validates:
1. jwt token from `auth_token` cookie
2. csrf token from `csrf_token` cookie matches `X-CSRF-Token` header
3. csrf token is valid for authenticated user
4. csrf token not expired (30 minute lifetime)

**logout** (`POST /api/auth/logout`):
```bash
curl -X POST http://localhost:8081/api/auth/logout \
  -H "Cookie: auth_token=..."
```

server clears cookies:
```
Set-Cookie: auth_token=; Max-Age=0
Set-Cookie: csrf_token=; Max-Age=0
```

#### CSRF Protection Details

**token generation**:
- 256-bit (32 byte) cryptographic randomness
- user-scoped validation (token tied to specific user_id)
- 30-minute expiration
- stored in-memory (HashMap with automatic cleanup)

**validation requirements**:
- csrf validation required for: POST, PUT, DELETE, PATCH
- csrf validation skipped for: GET, HEAD, OPTIONS
- validation extracts:
  1. user_id from jwt token (auth_token cookie)
  2. csrf token from X-CSRF-Token header
  3. verifies token valid for that user_id
  4. verifies token not expired

**double-submit cookie pattern**:
```
1. server generates csrf token
2. server sets csrf_token cookie (JavaScript readable)
3. server returns csrf_token in JSON response
4. client stores csrf_token in memory
5. client includes X-CSRF-Token header in state-changing requests
6. server validates:
   - csrf_token cookie matches X-CSRF-Token header
   - token is valid for authenticated user_id
   - token not expired
```

**security benefits**:
- attacker cannot read csrf token (cross-origin restriction)
- attacker cannot forge valid csrf token (cryptographic randomness)
- attacker cannot reuse old token (user-scoped validation)
- attacker cannot use expired token (30-minute lifetime)

#### Frontend Integration (React/TypeScript)

**axios configuration**:
```typescript
// enable automatic cookie handling
axios.defaults.withCredentials = true;

// request interceptor for csrf token
axios.interceptors.request.use((config) => {
  if (['POST', 'PUT', 'DELETE', 'PATCH'].includes(config.method?.toUpperCase() || '')) {
    const csrfToken = apiService.getCsrfToken();
    if (csrfToken && config.headers) {
      config.headers['X-CSRF-Token'] = csrfToken;
    }
  }
  return config;
});

// response interceptor for 401 errors
axios.interceptors.response.use(
  (response) => response,
  async (error) => {
    if (error.response?.status === 401) {
      // clear csrf token and redirect to login
      apiService.clearCsrfToken();
      window.location.href = '/login';
    }
    return Promise.reject(error);
  }
);
```

**login flow** (OAuth2 ROPC):
```typescript
async function login(email: string, password: string) {
  const params = new URLSearchParams({
    grant_type: 'password',
    username: email,
    password: password
  });
  const response = await axios.post('/oauth/token', params, {
    headers: { 'Content-Type': 'application/x-www-form-urlencoded' }
  });

  // store csrf token in memory (cookies set automatically)
  apiService.setCsrfToken(response.data.csrf_token);

  // store user info in localStorage (not sensitive)
  localStorage.setItem('user', JSON.stringify(response.data.user));

  return response.data;
}
```

**logout flow**:
```typescript
async function logout() {
  try {
    // call backend to clear httpOnly cookies
    await axios.post('/api/auth/logout');
  } catch (error) {
    console.error('Logout failed:', error);
  } finally {
    // clear client-side state
    apiService.clearCsrfToken();
    localStorage.removeItem('user');
  }
}
```

#### Token Refresh

web apps can proactively refresh tokens using the refresh endpoint:

```typescript
async function refreshToken() {
  const response = await axios.post('/api/auth/refresh');

  // server sets new auth_token and csrf_token cookies
  apiService.setCsrfToken(response.data.csrf_token);

  return response.data;
}
```

refresh generates:
- new jwt token (24 hour expiry)
- new csrf token (30 minute expiry)
- both cookies updated automatically

**when to refresh**:
- proactively before jwt expires (24h default)
- after csrf token expires (30min default)
- after receiving 401 response with expired token

#### Implementation References

**backend**:
- csrf token manager: `src/security/csrf.rs`
- secure cookie utilities: `src/security/cookies.rs`
- csrf middleware: `src/middleware/csrf.rs`
- authentication middleware: `src/middleware/auth.rs` (cookie-aware)
- auth handlers: `src/routes/auth.rs` (login, refresh, logout)

**frontend**:
- api service: `frontend/src/services/api.ts`
- auth context: `frontend/src/contexts/AuthContext.tsx`

#### Backward Compatibility

pierre supports both cookie-based and bearer token authentication simultaneously:

1. **cookie-based** (web apps): jwt from httpOnly cookie
2. **bearer token** (api clients): `Authorization: Bearer <token>` header

middleware tries cookies first, falls back to authorization header.

### API Key Authentication (Service-to-Service)

for a2a systems and service-to-service communication, api keys provide simpler authentication without cookies or csrf.

## Security Features

### Password Hashing

- algorithm: argon2id (default) or bcrypt
- configurable work factor
- per-user salt

### Token Encryption

- jwt signing: rs256 asymmetric (rsa) or hs256 symmetric
  - rs256: 4096-bit rsa keys (production), 2048-bit (tests)
  - hs256: 64-byte secret (legacy)
- provider tokens: aes-256-gcm
- encryption keys: two-tier system
  - master key (env: `PIERRE_MASTER_ENCRYPTION_KEY`)
  - tenant keys (derived from master key)

### RS256/JWKS

Asymmetric signing for distributed token verification.

Public keys available at `/admin/jwks` (legacy) and `/oauth2/jwks` (oauth2 clients):
```bash
curl http://localhost:8081/oauth2/jwks
```

Response (rfc 7517 compliant):
```json
{
  "keys": [
    {
      "kty": "RSA",
      "use": "sig",
      "kid": "key_2024_01_01_123456",
      "n": "modulus_base64url",
      "e": "exponent_base64url"
    }
  ]
}
```

**cache-control headers**: jwks endpoint returns `Cache-Control: public, max-age=3600` allowing browsers to cache public keys for 1 hour.

Clients verify tokens using public key. Pierre signs with private key.

Benefits:
- private key never leaves server
- clients verify without shared secret
- supports key rotation with grace period
- browser caching reduces jwks endpoint load

**key rotation**: when keys are rotated, old keys are retained during grace period to allow existing tokens to validate. New tokens are signed with the current key.

### Rate Limiting

Token bucket algorithm per authentication method:
- jwt tokens: per-tenant limits
- api keys: per-tier limits (free: 100/day, professional: 10,000/day, enterprise: unlimited)
- oauth2 endpoints: per-ip limits
  - `/oauth2/authorize`: 60 requests/minute
  - `/oauth2/token`: 30 requests/minute
  - `/oauth2/register`: 10 requests/minute

Oauth2 rate limit responses include:
```
X-RateLimit-Limit: 60
X-RateLimit-Remaining: 59
X-RateLimit-Reset: 1704067200
Retry-After: 42
```

Implementation: `src/rate_limiting.rs`, `src/oauth2/rate_limiting.rs`

### CSRF Protection

pierre implements comprehensive csrf protection for web applications:

**web application requests**:
- double-submit cookie pattern (see "Web Application Security" section above)
- 256-bit cryptographic csrf tokens
- user-scoped validation
- 30-minute token expiration
- automatic header validation for POST/PUT/DELETE/PATCH

**oauth flows**:
- state parameter validation in oauth flows (prevents csrf in oauth redirects)
- pkce for oauth2 authorization (code challenge verification)
- origin validation for web requests

see "Web Application Security" section above for detailed csrf implementation.

### Atomic Token Operations

Pierre prevents toctou (time-of-check to time-of-use) race conditions in token operations.

**problem**: token reuse attacks

Standard token validation flow vulnerable to race conditions:
```
thread 1: check token valid → ✓ valid
thread 2: check token valid → ✓ valid
thread 1: revoke token → success
thread 2: revoke token → success (token used twice!)
```

**solution**: atomic check-and-revoke

Pierre uses database-level atomic operations:
```sql
-- single atomic transaction
UPDATE oauth2_refresh_tokens
SET revoked_at = NOW()
WHERE token = ? AND revoked_at IS NULL
RETURNING *
```

Benefits:
- **race condition elimination**: only one thread can consume token
- **database-level garantees**: transaction isolation prevents concurrent access
- **zero-trust security**: every token exchange verified atomically

**vulnerable endpoints protected**:
- `POST /oauth2/token` (refresh token grant)
- token refresh operations
- authorization code exchange

**implementation details**:

Atomic operations in database plugins (`src/database_plugins/`):
```rust
/// atomically consume oauth2 refresh token (check-and-revoke in single operation)
async fn consume_refresh_token(&self, token: &str) -> Result<RefreshToken, DatabaseError>
```

Sqlite implementation uses `RETURNING` clause:
```rust
UPDATE oauth2_refresh_tokens
SET revoked_at = datetime('now')
WHERE token = ? AND revoked_at IS NULL
RETURNING *
```

Postgresql implementation uses same pattern with `RETURNING`:
```rust
UPDATE oauth2_refresh_tokens
SET revoked_at = NOW()
WHERE token = $1 AND revoked_at IS NULL
RETURNING *
```

If query returns no rows, token either:
- doesn't exist
- already revoked (race condition detected)
- expired

All three cases result in authentication failure, preventing token reuse.

Security guarantees:
- **serializability**: database transactions prevent concurrent modifications
- **atomicity**: check and revoke happen in single operation
- **consistency**: no partial state changes possible
- **isolation**: concurrent requests see consistent view

Implementation: `src/database_plugins/sqlite.rs`, `src/database_plugins/postgres.rs`, `src/oauth2/endpoints.rs`

## Troubleshooting

### "Invalid Token" Errors

- check token expiry: jwt tokens expire after 24h (default)
- verify token format: must be `Bearer <token>`
- ensure token not revoked: check `/oauth/status`

### OAuth2 Flow Fails

- verify redirect uri exactly matches registration
- check pkce challenge/verifier match
- ensure code not expired (10 min lifetime)

### Provider OAuth Fails

- verify provider credentials (client_id, client_secret)
- check redirect uri accessible from browser
- ensure callback endpoint reachable

### API Key Rejected

- verify api key active: not deleted or expired
- check rate limits: may be throttled
- ensure correct header: `X-API-Key` (case-sensitive)

## Implementation References

- jwt authentication: `src/auth.rs`
- api key management: `src/api_keys.rs`
- oauth2 server: `src/oauth2_server/`
- provider oauth: `src/oauth2_client/`
- encryption: `src/crypto/`, `src/key_management.rs`
- rate limiting: `src/rate_limiting.rs`

---

# OAuth2 Server

Pierre includes a standards-compliant oauth2 authorization server for secure mcp client authentication.

## Features

- authorization code flow with pkce (s256 only)
- dynamic client registration (rfc 7591)
- server-side state validation for csrf protection
- argon2id client secret hashing
- multi-tenant isolation
- refresh token rotation
- jwt-based access tokens

## Quick Start

### 1. Register OAuth2 Client

```bash
curl -X POST http://localhost:8081/oauth2/register \
  -H "Content-Type: application/json" \
  -d '{
    "redirect_uris": ["https://example.com/callback"],
    "client_name": "My MCP Client",
    "grant_types": ["authorization_code"],
    "response_types": ["code"]
  }'
```

Response:
```json
{
  "client_id": "mcp_client_abc123",
  "client_secret": "secret_xyz789",
  "client_id_issued_at": 1640000000,
  "redirect_uris": ["https://example.com/callback"],
  "grant_types": ["authorization_code"],
  "response_types": ["code"]
}
```

**important:** save `client_secret` immediately. Cannot be retrieved later.

### 2. Generate PKCE Challenge

```python
import secrets
import hashlib
import base64

# generate code verifier (43-128 characters)
code_verifier = base64.urlsafe_b64encode(secrets.token_bytes(32)).decode('utf-8').rstrip('=')

# generate code challenge (s256)
code_challenge = base64.urlsafe_b64encode(
    hashlib.sha256(code_verifier.encode('utf-8')).digest()
).decode('utf-8').rstrip('=')

# generate state (csrf protection)
state = secrets.token_urlsafe(32)

# store code_verifier and state in session
session['code_verifier'] = code_verifier
session['oauth_state'] = state
```

### 3. Initiate Authorization

Redirect user to authorization endpoint:

```
https://pierre.example.com/oauth2/authorize?
  response_type=code&
  client_id=mcp_client_abc123&
  redirect_uri=https://example.com/callback&
  state=<random_state>&
  code_challenge=<pkce_challenge>&
  code_challenge_method=S256&
  scope=read:activities write:goals
```

User will authenticate and authorize. Pierre redirects to callback with authorization code:

```
https://example.com/callback?
  code=auth_code_xyz&
  state=<same_random_state>
```

### 4. Validate State and Exchange Code

```python
# validate state parameter (csrf protection)
received_state = request.args.get('state')
stored_state = session.pop('oauth_state', None)

if not received_state or received_state != stored_state:
    return "csrf attack detected", 400

# exchange authorization code for tokens
code = request.args.get('code')
code_verifier = session.pop('code_verifier')
```

```bash
curl -X POST http://localhost:8081/oauth2/token \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=authorization_code" \
  -d "code=auth_code_xyz" \
  -d "redirect_uri=https://example.com/callback" \
  -d "client_id=mcp_client_abc123" \
  -d "client_secret=secret_xyz789" \
  -d "code_verifier=<stored_code_verifier>"
```

Response:
```json
{
  "access_token": "jwt_access_token",
  "token_type": "Bearer",
  "expires_in": 3600,
  "refresh_token": "refresh_token_abc",
  "scope": "read:activities write:goals"
}
```

### 5. Use Access Token

```bash
curl -H "Authorization: Bearer jwt_access_token" \
  http://localhost:8081/mcp
```

## Client Registration

### Register New Client

Endpoint: `POST /oauth2/register`

Required fields:
- `redirect_uris` - array of callback urls (https required except localhost)

Optional fields:
- `client_name` - display name
- `client_uri` - client homepage url
- `grant_types` - defaults to `["authorization_code"]`
- `response_types` - defaults to `["code"]`
- `scope` - space-separated scope list

### Redirect URI Validation

Pierre enforces strict redirect uri validation:

**allowed:**
- `https://` urls (production)
- `http://localhost:*` (development)
- `http://127.0.0.1:*` (development)
- `urn:ietf:wg:oauth:2.0:oob` (out-of-band for native apps)

**rejected:**
- `http://` non-localhost urls
- urls with fragments (`#`)
- wildcard domains (`*.example.com`)
- malformed urls

### Example Registrations

**web application:**
```bash
curl -X POST http://localhost:8081/oauth2/register \
  -H "Content-Type: application/json" \
  -d '{
    "redirect_uris": ["https://app.example.com/auth/callback"],
    "client_name": "Example Web App",
    "client_uri": "https://app.example.com",
    "scope": "read:activities read:athlete"
  }'
```

**native application:**
```bash
curl -X POST http://localhost:8081/oauth2/register \
  -H "Content-Type: application/json" \
  -d '{
    "redirect_uris": ["http://localhost:8080/callback"],
    "client_name": "Example Desktop App",
    "scope": "read:activities write:goals"
  }'
```

## Authorization Flow

### Step 1: Authorization Request

Build authorization url with required parameters:

```python
from urllib.parse import urlencode

params = {
    'response_type': 'code',
    'client_id': client_id,
    'redirect_uri': redirect_uri,
    'state': state,                    # required for csrf protection
    'code_challenge': code_challenge,  # required for pkce
    'code_challenge_method': 'S256',   # only s256 supported
    'scope': 'read:activities write:goals'  # optional
}

auth_url = f"https://pierre.example.com/oauth2/authorize?{urlencode(params)}"
```

Redirect user to `auth_url`.

### Step 2: User Authentication

If user not logged in, pierre displays login form. After successful login, shows authorization consent screen.

### Step 3: Authorization Callback

Pierre redirects to your `redirect_uri` with authorization code:

```
https://example.com/callback?code=<auth_code>&state=<state>
```

Error response (if user denies):
```
https://example.com/callback?error=access_denied&error_description=User+denied+authorization
```

### Step 4: Token Exchange

Exchange authorization code for access token:

```bash
curl -X POST http://localhost:8081/oauth2/token \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=authorization_code" \
  -d "code=<auth_code>" \
  -d "redirect_uri=<same_redirect_uri>" \
  -d "client_id=<client_id>" \
  -d "client_secret=<client_secret>" \
  -d "code_verifier=<pkce_verifier>"
```

**important:** authorization codes expire in 10 minutes and are single-use.

## Token Management

### Access Tokens

Jwt-based tokens with 1-hour expiration (configurable).

Claims include:
- `sub` - user id
- `email` - user email
- `tenant_id` - tenant identifier
- `scope` - granted scopes
- `exp` - expiration timestamp

### Refresh Tokens

Use refresh token to obtain new access token without re-authentication:

```bash
curl -X POST http://localhost:8081/oauth2/token \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=refresh_token" \
  -d "refresh_token=<refresh_token>" \
  -d "client_id=<client_id>" \
  -d "client_secret=<client_secret>"
```

Response:
```json
{
  "access_token": "new_jwt_access_token",
  "token_type": "Bearer",
  "expires_in": 3600,
  "refresh_token": "new_refresh_token",
  "scope": "read:activities write:goals"
}
```

**refresh token rotation:** pierre issues new refresh token with each refresh request. Old refresh token is revoked.

### Token Validation

Validate access token and optionally refresh if expired:

```bash
curl -X POST http://localhost:8081/oauth2/validate \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "refresh_token": "optional_refresh_token"
  }'
```

Responses:

**valid token:**
```json
{
  "status": "valid",
  "expires_in": 1800
}
```

**token refreshed:**
```json
{
  "status": "refreshed",
  "access_token": "new_jwt_token",
  "refresh_token": "new_refresh_token",
  "token_type": "Bearer"
}
```

**invalid token:**
```json
{
  "status": "invalid",
  "reason": "token expired",
  "requires_full_reauth": true
}
```

## Security Features

### PKCE (Proof Key for Code Exchange)

Pierre requires pkce for all authorization code flows.

**supported methods:**
- `S256` (sha256) - required

**rejected methods:**
- `plain` - insecure, not supported

**implementation:**
1. Generate random `code_verifier` (43-128 characters)
2. Compute `code_challenge = base64url(sha256(code_verifier))`
3. Send `code_challenge` in authorization request
4. Send `code_verifier` in token exchange
5. Pierre validates `sha256(code_verifier) == code_challenge`

Prevents authorization code interception attacks.

### State Parameter Validation

Pierre implements defense-in-depth csrf protection with server-side state validation.

**client requirements:**
1. Generate cryptographically random state (≥128 bits entropy)
2. Store state in session before authorization request
3. Include state in authorization request
4. Validate state matches in callback

**server behavior:**
1. Stores state with 10-minute expiration
2. Binds state to client_id and user
3. Validates state on callback
4. Marks state as used (single-use)
5. Rejects expired, used, or mismatched states

**example implementation:**
```python
import secrets

# before authorization
state = secrets.token_urlsafe(32)
session['oauth_state'] = state

# in callback
received_state = request.args.get('state')
stored_state = session.pop('oauth_state', None)

if not received_state or received_state != stored_state:
    abort(400, "invalid state - possible csrf attack")
```

### Client Secret Hashing

Client secrets hashed with argon2id (memory-hard algorithm resistant to gpu attacks).

**verification:**
```bash
# validate client credentials
curl -X POST http://localhost:8081/oauth2/token \
  -d "client_id=<id>" \
  -d "client_secret=<secret>" \
  ...
```

Pierre verifies secret using constant-time comparison to prevent timing attacks.

### Multi-tenant Isolation

All oauth artifacts (codes, tokens, states) bound to tenant_id. Cross-tenant access prevented at database layer.

## Scopes

Pierre supports fine-grained permission control via oauth scopes.

### Available Scopes

**fitness data:**
- `read:activities` - read activity data
- `write:activities` - create/update activities
- `read:athlete` - read athlete profile
- `write:athlete` - update athlete profile

**goals and analytics:**
- `read:goals` - read fitness goals
- `write:goals` - create/update goals
- `read:analytics` - access analytics data

**administrative:**
- `admin:users` - manage users
- `admin:system` - system administration

### Requesting Scopes

Include in authorization request:

```
/oauth2/authorize?
  ...
  scope=read:activities read:athlete write:goals
```

### Scope Validation

Pierre validates requested scopes against client's registered scopes. Access tokens include granted scopes in jwt claims.

## Error Handling

### Authorization Errors

Returned as query parameters in redirect:

```
https://example.com/callback?
  error=invalid_request&
  error_description=missing+code_challenge&
  state=<state>
```

**common errors:**
- `invalid_request` - missing or invalid parameters
- `unauthorized_client` - client not authorized for this flow
- `access_denied` - user denied authorization
- `unsupported_response_type` - response_type not supported
- `invalid_scope` - requested scope invalid or not allowed
- `server_error` - internal server error

### Token Errors

Returned as json in response body:

```json
{
  "error": "invalid_grant",
  "error_description": "authorization code expired",
  "error_uri": "https://datatracker.ietf.org/doc/html/rfc6749#section-5.2"
}
```

**common errors:**
- `invalid_request` - malformed request
- `invalid_client` - client authentication failed
- `invalid_grant` - code expired, used, or invalid
- `unauthorized_client` - client not authorized
- `unsupported_grant_type` - grant type not supported

## Common Integration Patterns

### Web Application Flow

1. User clicks "connect with pierre"
2. App redirects to pierre authorization endpoint
3. User logs in (if needed) and approves
4. Pierre redirects back with authorization code
5. App exchanges code for tokens (server-side)
6. App stores tokens securely (encrypted database)
7. App uses access token for api requests
8. App refreshes token before expiration

### Native Application Flow

1. App opens system browser to authorization url
2. User authenticates and approves
3. Browser redirects to `http://localhost:port/callback`
4. App's local server receives callback
5. App exchanges code for tokens
6. App stores tokens securely (os keychain)

### Single Page Application (SPA) Flow

**recommended:** use authorization code flow with pkce:

1. Spa redirects to pierre authorization endpoint
2. Pierre redirects back with authorization code
3. Spa exchanges code for tokens via backend proxy
4. Backend stores refresh token
5. Backend returns short-lived access token to spa
6. Spa uses access token for api requests
7. Spa requests new access token via backend when expired

**not recommended:** implicit flow (deprecated)

## Troubleshooting

### Authorization Code Expired

**symptom:** `invalid_grant` error when exchanging code

**solution:** authorization codes expire in 10 minutes. Restart authorization flow.

### PKCE Validation Failed

**symptom:** `invalid_grant: pkce verification failed`

**solutions:**
- ensure `code_verifier` sent in token request matches original
- verify code_challenge computed as `base64url(sha256(code_verifier))`
- check no extra padding (`=`) in base64url encoding

### State Validation Failed

**symptom:** `invalid_grant: invalid state parameter`

**solutions:**
- ensure state sent in callback matches original request
- check state not expired (10-minute ttl)
- verify state not reused (single-use)
- confirm state stored in user session before authorization

### Redirect URI Mismatch

**symptom:** `invalid_request: redirect_uri mismatch`

**solutions:**
- redirect_uri in authorization request must exactly match registration
- redirect_uri in token request must match authorization request
- https required for non-localhost urls

### Client Authentication Failed

**symptom:** `invalid_client`

**solutions:**
- verify client_id correct
- verify client_secret correct (case-sensitive)
- ensure client_secret not expired
- check client not deleted

### Refresh Token Revoked

**symptom:** `invalid_grant: refresh token revoked or expired`

**solutions:**
- refresh tokens expire after 30 days of inactivity
- old refresh tokens revoked after successful refresh (rotation)
- restart authorization flow to obtain new tokens

## Configuration

### Token Lifetimes

Pierre currently uses fixed lifetimes for OAuth2 artifacts (configured in code, not via environment variables):

- Authorization codes: 10 minutes (single-use)
- Access tokens: 1 hour
- Refresh tokens: 30 days
- State parameters: 10 minutes

Changing these values requires a code change in the OAuth2 server configuration (see `src/oauth2_server/` and `src/constants/`).

## See Also

- authentication - jwt and api key authentication
- protocols - fitness provider integrations
- configuration - server configuration

---

# OAuth Client (Fitness Providers)

Pierre acts as an oauth 2.0 client to connect to fitness providers (strava, fitbit, garmin) on behalf of users.

## Overview

**oauth2_client module** (`src/oauth2_client/`):
- pierre connects TO fitness providers as oauth client
- handles user authorization and token management
- supports pkce for enhanced security
- multi-tenant credential isolation

**separate from oauth2_server**:
- oauth2_server: mcp clients connect TO pierre
- oauth2_client: pierre connects TO fitness providers

## Supported Providers

| provider | oauth version | pkce | status | scopes | implementation |
|----------|--------------|------|--------|--------|----------------|
| strava | oauth 2.0 | required | active | `activity:read_all` | `src/providers/strava.rs` |
| fitbit | oauth 2.0 | required | active | `activity`,`heartrate`,`location`,`nutrition`,`profile`,`settings`,`sleep`,`social`,`weight` | `src/providers/fitbit.rs` |
| garmin | oauth 2.0 | required | active | `wellness:read`,`activities:read` | `src/providers/garmin_provider.rs` |
| whoop | oauth 2.0 | required | active | `read:profile`,`read:body_measurement`,`read:workout`,`read:sleep`,`read:recovery`,`read:cycles` | `src/providers/whoop_provider.rs` |
| terra | oauth 2.0 | required | active | device-dependent (150+ wearables) | `src/providers/terra_provider.rs` |

**note**: providers require compile-time feature flags (`provider-strava`, `provider-fitbit`, `provider-whoop`, `provider-terra`, etc.).

Implementation: `src/oauth2_client/mod.rs`

## Configuration

### Environment Variables

**strava:**
```bash
export STRAVA_CLIENT_ID=your_client_id
export STRAVA_CLIENT_SECRET=your_client_secret
export STRAVA_REDIRECT_URI=http://localhost:8081/api/oauth/callback/strava  # dev
```

**fitbit:**
```bash
export FITBIT_CLIENT_ID=your_client_id
export FITBIT_CLIENT_SECRET=your_client_secret
export FITBIT_REDIRECT_URI=http://localhost:8081/api/oauth/callback/fitbit  # dev
```

**garmin:**
```bash
export GARMIN_CLIENT_ID=your_consumer_key
export GARMIN_CLIENT_SECRET=your_consumer_secret
export GARMIN_REDIRECT_URI=http://localhost:8081/api/oauth/callback/garmin  # dev
```

**whoop:**
```bash
export WHOOP_CLIENT_ID=your_client_id
export WHOOP_CLIENT_SECRET=your_client_secret
export WHOOP_REDIRECT_URI=http://localhost:8081/api/oauth/callback/whoop  # dev
```

**production:** use https redirect urls:
```bash
export STRAVA_REDIRECT_URI=https://api.example.com/api/oauth/callback/strava
export FITBIT_REDIRECT_URI=https://api.example.com/api/oauth/callback/fitbit
export GARMIN_REDIRECT_URI=https://api.example.com/api/oauth/callback/garmin
export WHOOP_REDIRECT_URI=https://api.example.com/api/oauth/callback/whoop
```

Constants: `src/constants/oauth/providers.rs`

## Multi-tenant Architecture

### Credential Hierarchy

Credentials resolved in priority order:
1. **tenant-specific credentials** (database, encrypted)
2. **server-level credentials** (environment variables)

Implementation: `src/oauth2_client/tenant_client.rs`

### Tenant OAuth Client

**`TenantOAuthClient`** (`src/oauth2_client/tenant_client.rs:36-49`):
```rust
pub struct TenantOAuthClient {
    pub oauth_manager: Arc<Mutex<TenantOAuthManager>>,
}
```

**features:**
- tenant-specific credential isolation
- rate limiting per tenant per provider
- automatic credential fallback to server config

### Storing Tenant Credentials

**via authorization request headers:**
```bash
curl -X GET "http://localhost:8081/api/oauth/auth/strava/uuid" \
  -H "x-strava-client-id: tenant_client_id" \
  -H "x-strava-client_secret: tenant_client_secret"
```

Credentials stored encrypted in database, bound to tenant.

**via api:**
```rust
tenant_oauth_client.store_credentials(
    tenant_id,
    "strava",
    StoreCredentialsRequest {
        client_id: "tenant_client_id".to_string(),
        client_secret: "tenant_client_secret".to_string(),
        redirect_uri: "https://tenant.example.com/callback/strava".to_string(),
        scopes: vec!["activity:read_all".to_string()],
        configured_by: user_id,
    }
).await?;
```

Implementation: `src/oauth2_client/tenant_client.rs:21-34`

### Rate Limiting

**default limits** (`src/tenant/oauth_manager.rs`):
- strava: 1000 requests/day per tenant
- fitbit: 150 requests/day per tenant
- garmin: 1000 requests/day per tenant
- whoop: 1000 requests/day per tenant

**rate limit enforcement:**
```rust
let (current_usage, daily_limit) = manager
    .check_rate_limit(tenant_id, provider)?;

if current_usage >= daily_limit {
    return Err(AppError::invalid_input(format!(
        "Tenant {} exceeded daily rate limit for {}: {}/{}",
        tenant_id, provider, current_usage, daily_limit
    )));
}
```

Implementation: `src/oauth2_client/tenant_client.rs:64-75`

## OAuth Flow

### Step 1: Initiate Authorization

**via mcp tool:**
```
user: "connect to strava"
```

**via rest api:**
```bash
curl -H "Authorization: Bearer <jwt>" \
  "http://localhost:8081/api/oauth/auth/strava/<user_id>"
```

**flow manager** (`src/oauth2_client/flow_manager.rs:29-105`):
1. Validates user_id and tenant_id
2. Processes optional tenant credentials from headers
3. Generates authorization redirect url
4. Returns http 302 redirect to provider

### Step 2: User Authorizes at Provider

Pierre generates authorization url with:
- **pkce s256 challenge** (128-character verifier)
- **state parameter** for csrf protection (`{user_id}:{random_uuid}`)
- **provider scopes** (activity read, heartrate, etc.)

**pkce generation** (`src/oauth2_client/client.rs:35-58`):
```rust
pub fn generate() -> PkceParams {
    // 128-character random verifier (43-128 allowed by RFC)
    let code_verifier: String = (0..128)
        .map(|_| CHARS[rng.gen_range(0..CHARS.len())] as char)
        .collect();

    // S256 challenge: base64url(sha256(code_verifier))
    let mut hasher = Sha256::new();
    hasher.update(code_verifier.as_bytes());
    let hash = hasher.finalize();
    let code_challenge = URL_SAFE_NO_PAD.encode(hash);

    PkceParams {
        code_verifier,
        code_challenge,
        code_challenge_method: "S256".into(),
    }
}
```

User authenticates with provider and grants permissions.

### Step 3: OAuth Callback

Provider redirects to pierre callback:
```
http://localhost:8081/api/oauth/callback/strava?
  code=authorization_code&
  state=user_id:random_uuid
```

**callback handling** (`src/routes/auth.rs`):
1. Validates state parameter (csrf protection)
2. Extracts user_id from state
3. Exchanges authorization code for access token
4. Encrypts tokens with aes-256-gcm
5. Stores in database (tenant-isolated)
6. Renders success page

### Step 4: Success Page

User sees branded html page:
- provider name and connection status
- user identifier
- pierre logo
- instructions to return to mcp client

Template: `templates/oauth_success.html`
Renderer: `src/oauth2_client/flow_manager.rs:350-393`

## Token Management

### OAuth2Token Structure

**`OAuth2Token`** (`src/oauth2_client/client.rs:61-82`):
```rust
pub struct OAuth2Token {
    pub access_token: String,
    pub token_type: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
}

impl OAuth2Token {
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .is_some_and(|expires_at| expires_at <= Utc::now())
    }

    pub fn will_expire_soon(&self) -> bool {
        self.expires_at
            .is_some_and(|expires_at| expires_at <= Utc::now() + Duration::minutes(5))
    }
}
```

### Storage

Tokens stored in `users` table with provider-specific columns:

```sql
-- strava example
strava_access_token     TEXT      -- encrypted
strava_refresh_token    TEXT      -- encrypted
strava_expires_at       TIMESTAMP
strava_scope            TEXT      -- comma-separated
```

**encryption:**
- algorithm: aes-256-gcm
- key: tenant-specific (derived from `PIERRE_MASTER_ENCRYPTION_KEY`)
- unique key per tenant ensures isolation

Implementation: `src/database/tokens.rs`, `src/crypto/`, `src/key_management.rs`

### Automatic Refresh

Pierre refreshes expired tokens before api requests:

**refresh criteria:**
- access token expired or expiring within 5 minutes
- refresh token available and valid

**refresh flow** (`src/oauth2_client/client.rs:272-302`):
```rust
pub async fn refresh_token(&self, refresh_token: &str) -> AppResult<OAuth2Token> {
    let params = [
        ("client_id", self.config.client_id.as_str()),
        ("client_secret", self.config.client_secret.as_str()),
        ("refresh_token", refresh_token),
        ("grant_type", "refresh_token"),
    ];

    let response: TokenResponse = self
        .client
        .post(&self.config.token_url)
        .form(&params)
        .send()
        .await?
        .json()
        .await?;

    Ok(Self::token_from_response(response))
}
```

Note: PKCE (`code_verifier`) is only used during authorization code exchange, not token refresh per RFC 7636.

### Manual Token Operations

**get token:**
```rust
let token = database.get_oauth_token(user_id, "strava").await?;
```

**update token:**
```rust
database.update_oauth_token(
    user_id,
    "strava",
    OAuthToken {
        access_token: "new_token".to_string(),
        refresh_token: Some("new_refresh".to_string()),
        expires_at: Utc::now() + Duration::hours(6),
        scope: "activity:read_all".to_string(),
    }
).await?;
```

**clear token (disconnect):**
```rust
database.clear_oauth_token(user_id, "strava").await?;
```

Implementation: `src/database/tokens.rs`

## Connection Status

**check connection:**
```bash
curl -H "Authorization: Bearer <jwt>" \
  http://localhost:8081/api/oauth/status
```

Response:
```json
{
  "connected_providers": ["strava", "fitbit"],
  "providers": {
    "strava": {
      "connected": true,
      "expires_at": "2024-01-01T12:00:00Z",
      "scope": "activity:read_all",
      "auto_refresh": true
    },
    "fitbit": {
      "connected": true,
      "expires_at": "2024-01-01T14:00:00Z",
      "scope": "activity heartrate location",
      "auto_refresh": true
    },
    "garmin": {
      "connected": false
    }
  }
}
```

**disconnect provider:**

Use the `disconnect_provider` MCP tool to revoke a provider connection; there is no standalone REST `DELETE /api/oauth/disconnect/{provider}` endpoint.

Implementation: `src/routes/auth.rs`

## Security Features

### PKCE (Proof Key for Code Exchange)

**implementation** (`src/oauth2_client/client.rs:27-59`):

All provider oauth flows use pkce (rfc 7636):

**code verifier:**
- 128 characters
- cryptographically random
- allowed characters: `A-Z a-z 0-9 - . _ ~`

**code challenge:**
- sha256 hash of code verifier
- base64url encoded (no padding)
- method: s256 only

Prevents authorization code interception attacks.

### State Parameter Validation

**state format:** `{user_id}:{random_uuid}`

**validation** (`src/oauth2_client/flow_manager.rs:162-215`):
1. Extract user_id from state
2. Verify user exists and belongs to tenant
3. Ensure state not reused (single-use)

Invalid state results in authorization rejection.

### Token Encryption

**encryption** (`src/crypto/`, `src/key_management.rs`):
- algorithm: aes-256-gcm
- key derivation:
  - master key: `PIERRE_MASTER_ENCRYPTION_KEY` (base64, 32 bytes)
  - tenant keys: derived from master key using tenant_id
  - unique key per tenant ensures isolation

**encrypted fields:**
- access_token
- refresh_token
- client_secret (for tenant credentials)

Decryption requires:
1. Correct master key
2. Correct tenant_id
3. Valid encryption nonce

### Tenant Isolation

Oauth artifacts never shared between tenants:
- credentials stored per tenant_id
- tokens bound to user and tenant
- rate limits enforced per tenant
- database queries include tenant_id filter

Cross-tenant access prevented at database layer.

Implementation: `src/tenant/oauth_manager.rs`

## Provider-specific Details

### Strava

**auth url:** `https://www.strava.com/oauth/authorize`
**token url:** `https://www.strava.com/oauth/token`
**api base:** `https://www.strava.com/api/v3`

**default scopes:** `activity:read_all`

**available scopes:**
- `read` - read public profile
- `activity:read` - read non-private activities
- `activity:read_all` - read all activities (public and private)
- `activity:write` - create and update activities

**rate limits:**
- 100 requests per 15 minutes per access token
- 1000 requests per day per application

**token lifetime:**
- access token: 6 hours
- refresh token: permanent (until revoked)

Implementation: `src/providers/strava.rs`, `src/providers/strava_provider.rs`

### Fitbit

**auth url:** `https://www.fitbit.com/oauth2/authorize`
**token url:** `https://api.fitbit.com/oauth2/token`
**api base:** `https://api.fitbit.com/1`

**default scopes:** `activity heartrate location nutrition profile settings sleep social weight`

**scope details:**
- `activity` - steps, distance, calories, floors
- `heartrate` - heart rate data
- `location` - gps data
- `nutrition` - food and water logs
- `profile` - personal information
- `settings` - user preferences
- `sleep` - sleep logs
- `social` - friends and leaderboards
- `weight` - weight and body measurements

**rate limits:**
- 150 requests per hour per user

**token lifetime:**
- access token: 8 hours
- refresh token: 1 year

Implementation: `src/providers/fitbit.rs`

### Garmin

**auth url:** `https://connect.garmin.com/oauthConfirm`
**token url:** `https://connectapi.garmin.com/oauth-service/oauth/access_token`
**api base:** `https://apis.garmin.com`

**default scopes:** `wellness:read activities:read`

**scope details:**
- `wellness:read` - health metrics (sleep, stress, hrv)
- `activities:read` - workout and activity data
- `wellness:write` - update health data
- `activities:write` - create activities

**rate limits:**
- varies by api endpoint
- typically 1000 requests per day

**token lifetime:**
- access token: 1 year
- refresh token: not provided (long-lived access token)

Implementation: `src/providers/garmin_provider.rs`

### WHOOP

**auth url:** `https://api.prod.whoop.com/oauth/oauth2/auth`
**token url:** `https://api.prod.whoop.com/oauth/oauth2/token`
**api base:** `https://api.prod.whoop.com/developer/v1`

**default scopes:** `offline read:profile read:body_measurement read:workout read:sleep read:recovery read:cycles`

**scope details:**
- `offline` - offline access for token refresh
- `read:profile` - user profile information
- `read:body_measurement` - body measurements (weight, height)
- `read:workout` - workout/activity data with strain scores
- `read:sleep` - sleep sessions and metrics
- `read:recovery` - daily recovery scores
- `read:cycles` - physiological cycle data

**rate limits:**
- varies by endpoint
- standard api rate limiting applies

**token lifetime:**
- access token: 1 hour
- refresh token: long-lived (requires `offline` scope)

Implementation: `src/providers/whoop_provider.rs`

## Error Handling

### Authorization Errors

Displayed on html error page (`templates/oauth_error.html`):

**common errors:**
- `access_denied` - user denied authorization
- `invalid_request` - missing or invalid parameters
- `invalid_scope` - requested scope not available
- `server_error` - provider api error

Renderer: `src/oauth2_client/flow_manager.rs:329-347`

### Callback Errors

Returned as query parameters:
```
http://localhost:8081/api/oauth/callback/strava?
  error=access_denied&
  error_description=User+declined+authorization
```

### Token Errors

**expired token:**
- automatically refreshed before api request
- no user action required

**invalid refresh token:**
- user must re-authorize
- connection status shows disconnected

**rate limit exceeded:**
```json
{
  "error": "rate_limit_exceeded",
  "provider": "strava",
  "retry_after_secs": 3600,
  "limit_type": "daily quota"
}
```

Implementation: `src/providers/errors.rs`

## Troubleshooting

### Authorization Fails

**symptom:** redirect to provider fails or returns error

**solutions:**
- verify provider credentials (client_id, client_secret)
- check redirect_uri matches provider configuration exactly
- ensure redirect_uri uses https in production
- confirm provider api credentials active and approved

### Callback Error: State Validation Failed

**symptom:** `invalid state parameter` error on callback

**solutions:**
- ensure user_id in authorization request matches authenticated user
- check user exists in database
- verify tenant association correct
- confirm no url encoding issues in state parameter

### Token Refresh Fails

**symptom:** api requests fail with authentication error

**solutions:**
- check refresh token not expired or revoked
- verify provider credentials still valid
- ensure network connectivity to provider api
- re-authorize user to obtain new tokens

### Rate Limit Exceeded

**symptom:** api requests rejected with rate limit error

**solutions:**
- check current usage via tenant_oauth_manager
- wait for daily reset (midnight utc)
- request rate limit increase from provider
- optimize api call patterns to reduce requests

### Encryption Key Mismatch

**symptom:** cannot decrypt stored tokens

**solutions:**
- verify `PIERRE_MASTER_ENCRYPTION_KEY` unchanged
- check key is valid base64 (32 bytes decoded)
- ensure key not rotated without token re-encryption
- re-authorize users if key changed

## Implementation References

- oauth2 client: `src/oauth2_client/client.rs`
- oauth flow manager: `src/oauth2_client/flow_manager.rs`
- tenant client: `src/oauth2_client/tenant_client.rs`
- tenant oauth manager: `src/tenant/oauth_manager.rs`
- provider implementations: `src/providers/`
- token storage: `src/database/tokens.rs`
- route handlers: `src/routes/auth.rs`
- templates: `templates/oauth_success.html`, `templates/oauth_error.html`

## See Also

- oauth2 server - mcp client authentication
- authentication - authentication methods and jwt tokens
- configuration - environment variables

---

# Protocols

Pierre implements three protocols on a single http port (8081).

## MCP (Model Context Protocol)

Json-rpc 2.0 protocol for ai assistant integration.

### Endpoints

- `POST /mcp` - main mcp endpoint
- `GET /mcp/sse/{session_id}` - sse transport for streaming (session-scoped)

### Transport

Pierre supports both http and sse transports:
- http: traditional request-response
- sse: server-sent events for streaming responses

Sdk handles transport negotiation automatically.

### Authentication

Mcp requests require jwt bearer token in authorization header:
```
Authorization: Bearer <jwt_token>
```

Obtained via oauth2 flow (sdk handles automatically).

### Request Format

```json
{
  "jsonrpc": "2.0",
  "id": "1",
  "method": "tools/call",
  "params": {
    "name": "get_activities",
    "arguments": {
      "limit": 5
    }
  }
}
```

### Response Format

```json
{
  "jsonrpc": "2.0",
  "id": "1",
  "result": {
    "content": [
      {
        "type": "text",
        "text": "[activity data...]"
      }
    ]
  }
}
```

### Output Format Parameter

Most data-returning tools support an optional `format` parameter for output serialization:

| Format | Description | Use Case |
|--------|-------------|----------|
| `json` | Standard JSON (default) | Universal compatibility |
| `toon` | Token-Oriented Object Notation | ~40% fewer LLM tokens |

Example with TOON format:
```json
{
  "jsonrpc": "2.0",
  "id": "1",
  "method": "tools/call",
  "params": {
    "name": "get_activities",
    "arguments": {
      "provider": "strava",
      "limit": 100,
      "format": "toon"
    }
  }
}
```

TOON format responses include `format: "toon"` and `content_type: "application/vnd.toon"` in the result. Use TOON for large datasets (year summaries, batch analysis) to reduce LLM context usage.

See TOON specification for format details.

### MCP Methods

- `initialize` - start session
- `tools/list` - list available tools
- `tools/call` - execute tool
- `resources/list` - list resources
- `prompts/list` - list prompts

Implementation: `src/mcp/protocol.rs`, `src/protocols/universal/`

## OAuth2 Authorization Server

Rfc 7591 (dynamic client registration) + rfc 7636 (pkce) compliant oauth2 server for mcp client authentication.

### Endpoints

- `GET /.well-known/oauth-authorization-server` - server metadata (rfc 8414)
- `POST /oauth2/register` - dynamic client registration
- `GET /oauth2/authorize` - authorization endpoint
- `POST /oauth2/token` - token endpoint
- `GET /oauth2/jwks` - json web key set
- `GET /.well-known/jwks.json` - jwks at standard oidc location
- `POST /oauth2/validate-and-refresh` - validate and refresh jwt tokens
- `POST /oauth2/token-validate` - validate jwt token

### Registration Flow

1. **client registration** (rfc 7591):
```bash
# local development (http allowed for localhost)
curl -X POST http://localhost:8081/oauth2/register \
  -H "Content-Type: application/json" \
  -d '{
    "redirect_uris": ["http://localhost:35535/oauth/callback"],
    "client_name": "My MCP Client (Dev)",
    "grant_types": ["authorization_code"]
  }'

# production (https required)
curl -X POST https://api.example.com/oauth2/register \
  -H "Content-Type: application/json" \
  -d '{
    "redirect_uris": ["https://client.example.com/oauth/callback"],
    "client_name": "My MCP Client",
    "grant_types": ["authorization_code"]
  }'
```

Response:
```json
{
  "client_id": "generated_client_id",
  "client_secret": "generated_secret",
  "redirect_uris": ["http://localhost:35535/oauth/callback"],
  "grant_types": ["authorization_code"]
}
```

**callback url security**: redirect_uris using http only permitted for localhost/127.0.0.1 in development. Production clients must use https to protect authorization codes from interception.

2. **authorization request**:
```
GET /oauth2/authorize?
  client_id=<client_id>&
  redirect_uri=<redirect_uri>&
  response_type=code&
  code_challenge=<pkce_challenge>&
  code_challenge_method=S256
```

User authenticates in browser, redirected to:
```
<redirect_uri>?code=<authorization_code>
```

3. **token exchange**:
```bash
curl -X POST http://localhost:8081/oauth2/token \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=authorization_code&\
      code=<authorization_code>&\
      client_id=<client_id>&\
      client_secret=<client_secret>&\
      redirect_uri=<redirect_uri>&\
      code_verifier=<pkce_verifier>"
```

Response:
```json
{
  "access_token": "jwt_token",
  "token_type": "Bearer",
  "expires_in": 86400
}
```

Jwt access token used for all mcp requests.

### PKCE Requirement

Pierre enforces pkce (rfc 7636) for all authorization code flows. Clients must:
- generate code verifier (43-128 characters)
- create code challenge: `base64url(sha256(verifier))`
- include challenge in authorization request
- include verifier in token request

### Server Discovery (RFC 8414)

Pierre provides oauth2 server metadata for automatic configuration:

```bash
curl http://localhost:8081/.well-known/oauth-authorization-server
```

Response includes:
```json
{
  "issuer": "http://localhost:8081",
  "authorization_endpoint": "http://localhost:8081/oauth2/authorize",
  "token_endpoint": "http://localhost:8081/oauth2/token",
  "jwks_uri": "http://localhost:8081/oauth2/jwks",
  "registration_endpoint": "http://localhost:8081/oauth2/register",
  "response_types_supported": ["code"],
  "grant_types_supported": ["authorization_code"],
  "code_challenge_methods_supported": ["S256"]
}
```

Issuer url configurable via `OAUTH2_ISSUER_URL` environment variable.

### JWKS Endpoint

Public keys for jwt token verification available at `/oauth2/jwks`:

```bash
curl http://localhost:8081/oauth2/jwks
```

Response (rfc 7517 compliant):
```json
{
  "keys": [
    {
      "kty": "RSA",
      "use": "sig",
      "kid": "key_2024_01_01",
      "n": "modulus_base64url",
      "e": "exponent_base64url"
    }
  ]
}
```

**cache-control headers**: jwks endpoint returns `Cache-Control: public, max-age=3600` allowing browsers to cache public keys for 1 hour.

### Key Rotation

Pierre supports rs256 key rotation with grace period:
- new keys generated with timestamp-based kid (e.g., `key_2024_01_01_123456`)
- old keys retained during grace period for existing token validation
- tokens issued with old keys remain valid until expiration
- new tokens signed with current key

Clients should:
1. Fetch jwks on startup
2. Cache public keys for 1 hour (respects cache-control header)
3. Refresh jwks if unknown kid encountered
4. Verify token signature using matching kid

### Rate Limiting

Oauth2 endpoints protected by per-ip token bucket rate limiting:

| endpoint | requests per minute |
|----------|---------------------|
| `/oauth2/authorize` | 60 (1/second) |
| `/oauth2/token` | 30 (1/2 seconds) |
| `/oauth2/register` | 10 (1/6 seconds) |

Rate limit headers included in all responses:
```
X-RateLimit-Limit: 60
X-RateLimit-Remaining: 59
X-RateLimit-Reset: 1704067200
```

429 response when limit exceeded:
```json
{
  "error": "rate_limit_exceeded",
  "error_description": "Rate limit exceeded. Retry after 42 seconds."
}
```

Headers:
```
Retry-After: 42
X-RateLimit-Limit: 60
X-RateLimit-Remaining: 0
X-RateLimit-Reset: 1704067200
```

Implementation: `src/oauth2_server/`, `src/oauth2_server/rate_limiting.rs`

## A2A (Agent-to-Agent Protocol)

Protocol for autonomous ai systems to communicate.

### Endpoints

- `GET /a2a/status` - protocol status
- `GET /a2a/tools` - available tools
- `POST /a2a/execute` - execute tool
- `GET /a2a/monitoring` - monitoring info

### Authentication

A2a uses api keys:
```
X-API-Key: <api_key>
```

Create api key via admin endpoint:
```bash
curl -X POST http://localhost:8081/api/keys \
  -H "Authorization: Bearer <admin_jwt>" \
  -H "Content-Type: application/json" \
  -d '{"name": "My A2A System", "tier": "professional"}'
```

### Agent Cards

Agents advertise capabilities via agent cards:
```json
{
  "agent_id": "fitness-analyzer",
  "name": "Fitness Analyzer Agent",
  "version": "1.0.0",
  "capabilities": [
    "activity_analysis",
    "performance_prediction",
    "goal_tracking"
  ],
  "endpoints": [
    {
      "path": "/a2a/execute",
      "method": "POST",
      "description": "Execute fitness analysis"
    }
  ]
}
```

### Request Format

```json
{
  "tool": "analyze_activity",
  "parameters": {
    "activity_id": "12345",
    "analysis_type": "comprehensive"
  }
}
```

### Response Format

```json
{
  "success": true,
  "result": {
    "analysis": {...},
    "recommendations": [...]
  }
}
```

Implementation: `src/a2a/`, `src/protocols/universal/`

## REST API

Traditional rest endpoints for web applications.

### Authentication Endpoints

- `POST /api/auth/register` - user registration (admin-provisioned)
- `POST /api/auth/login` - user login
- `POST /api/auth/logout` - logout
- `POST /api/auth/refresh` - refresh jwt token

### Provider OAuth Endpoints

- `GET /api/oauth/auth/{provider}/{user_id}` - initiate oauth (strava, garmin, fitbit, whoop)
- `GET /api/oauth/callback/{provider}` - oauth callback
- `GET /api/oauth/status` - connection status

### Admin Endpoints

- `POST /admin/setup` - create admin user
- `POST /admin/users` - manage users
- `GET /admin/analytics` - usage analytics

### Configuration Endpoints

- `GET /api/configuration/catalog` - config catalog
- `GET /api/configuration/profiles` - available profiles
- `GET /api/configuration/user` - user config
- `PUT /api/configuration/user` - update config

Implementation: `src/routes.rs`, `src/admin_routes.rs`, `src/configuration_routes.rs`

## SSE (Server-Sent Events)

Real-time notifications for oauth completions and system events.

### Endpoint

```
GET /notifications/sse?user_id=<user_id>
```

### Event Types

- `oauth_complete` - oauth flow completed
- `oauth_error` - oauth flow failed
- `system_status` - system status update

### Example

```javascript
const eventSource = new EventSource('/notifications/sse?user_id=user-123');

eventSource.onmessage = function(event) {
  const notification = JSON.parse(event.data);
  if (notification.type === 'oauth_complete') {
    console.log('OAuth completed for provider:', notification.provider);
  }
};
```

Implementation: `src/notifications/sse.rs`, `src/sse.rs`

## Protocol Comparison

| feature | mcp | oauth2 | a2a | rest |
|---------|-----|--------|-----|------|
| primary use | ai assistants | client auth | agent comms | web apps |
| auth method | jwt bearer | - | api key | jwt bearer |
| transport | http + sse | http | http | http |
| format | json-rpc 2.0 | oauth2 | json | json |
| implementation | `src/mcp/` | `src/oauth2_server/` | `src/a2a/` | `src/routes/` |

## Choosing a Protocol

- **ai assistant integration**: use mcp (claude, chatgpt)
- **web application**: use rest api
- **autonomous agents**: use a2a
- **client authentication**: use oauth2 (for mcp clients)

All protocols share the same business logic via `src/protocols/universal/`.

---

# provider registration guide

This guide shows how pierre's pluggable provider architecture supports **1 to x providers simultaneously** and how new providers are registered.

## provider registration flow

```
┌──────────────────────────────────────────────────────┐
│  Step 1: Application Startup                         │
│  ProviderRegistry::new() called                      │
└────────────┬─────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────┐
│  Step 2: Factory Registration (1 to x providers)     │
│                                                       │
│  registry.register_factory("strava", StravaFactory)  │
│  registry.register_factory("garmin", GarminFactory)  │
│  registry.register_factory("fitbit", FitbitFactory)  │
│  registry.register_factory("synthetic", SynthFactory)│
│  registry.register_factory("whoop", WhoopFactory)    │ <- built-in
│  registry.register_factory("terra", TerraFactory)    │ <- built-in
│  registry.register_factory("polar", PolarFactory)    │ <- custom example
│  ... unlimited providers ...                         │
└────────────┬─────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────┐
│  Step 3: Environment Configuration Loading           │
│                                                       │
│  For each registered provider:                       │
│    config = load_provider_env_config(                │
│      provider_name,                                  │
│      default_auth_url,                               │
│      default_token_url,                              │
│      default_api_base_url,                           │
│      default_revoke_url,                             │
│      default_scopes                                  │
│    )                                                 │
│    registry.set_default_config(provider, config)     │
└────────────┬─────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────┐
│  Step 4: Runtime Usage                               │
│                                                       │
│  // Check if provider is available                   │
│  if registry.is_supported("strava") { ... }          │
│                                                       │
│  // List all available providers                     │
│  let providers = registry.supported_providers();     │
│  // ["strava", "garmin", "fitbit", "synthetic",      │
│  //  "whoop", "polar", ...]                          │
│                                                       │
│  // Create provider instance                         │
│  let provider = registry.create_provider("strava");  │
│                                                       │
│  // Use provider through FitnessProvider trait       │
│  let activities = provider.get_activities(...).await;│
└──────────────────────────────────────────────────────┘
```

## how providers are registered

### example: registering strava (built-in)

**Location**: `src/providers/registry.rs:71-94`

```rust
impl ProviderRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            factories: HashMap::new(),
            default_configs: HashMap::new(),
        };

        // 1. Register factory
        registry.register_factory(
            oauth_providers::STRAVA,  // "strava"
            Box::new(StravaProviderFactory),
        );

        // 2. Load environment configuration
        let (_client_id, _client_secret, auth_url, token_url,
             api_base_url, revoke_url, scopes) =
            crate::config::environment::load_provider_env_config(
                oauth_providers::STRAVA,
                "https://www.strava.com/oauth/authorize",
                "https://www.strava.com/oauth/token",
                "https://www.strava.com/api/v3",
                Some("https://www.strava.com/oauth/deauthorize"),
                &[oauth_providers::STRAVA_DEFAULT_SCOPES.to_owned()],
            );

        // 3. Set default configuration
        registry.set_default_config(
            oauth_providers::STRAVA,
            ProviderConfig {
                name: oauth_providers::STRAVA.to_owned(),
                auth_url,
                token_url,
                api_base_url,
                revoke_url,
                default_scopes: scopes,
            },
        );

        // Repeat for Garmin, Fitbit, Synthetic, etc.
        // ...

        registry
    }
}
```

### example: registering custom provider (whoop)

**Location**: `src/providers/registry.rs` (add to `new()` method)

```rust
// Register Whoop provider
registry.register_factory(
    "whoop",
    Box::new(WhoopProviderFactory),
);

let (_, _, auth_url, token_url, api_base_url, revoke_url, scopes) =
    crate::config::environment::load_provider_env_config(
        "whoop",
        "https://api.prod.whoop.com/oauth/authorize",
        "https://api.prod.whoop.com/oauth/token",
        "https://api.prod.whoop.com/developer/v1",
        Some("https://api.prod.whoop.com/oauth/revoke"),
        &["read:workout".to_owned(), "read:profile".to_owned()],
    );

registry.set_default_config(
    "whoop",
    ProviderConfig {
        name: "whoop".to_owned(),
        auth_url,
        token_url,
        api_base_url,
        revoke_url,
        default_scopes: scopes,
    },
);
```

**That's it!** Whoop is now registered and available alongside Strava, Garmin, and others.

## environment variables for 1 to x providers

Pierre supports **unlimited providers simultaneously**. Just set environment variables for each:

```bash
# Default provider (required)
export PIERRE_DEFAULT_PROVIDER=strava

# Provider 1: Strava
export PIERRE_STRAVA_CLIENT_ID=abc123
export PIERRE_STRAVA_CLIENT_SECRET=secret123

# Provider 2: Garmin
export PIERRE_GARMIN_CLIENT_ID=xyz789
export PIERRE_GARMIN_CLIENT_SECRET=secret789

# Provider 3: Fitbit
export PIERRE_FITBIT_CLIENT_ID=fitbit123
export PIERRE_FITBIT_CLIENT_SECRET=fitbit_secret

# Provider 4: Synthetic (no credentials needed!)
# Automatically available - no env vars required

# Provider 5: Custom Whoop
export PIERRE_WHOOP_CLIENT_ID=whoop_client
export PIERRE_WHOOP_CLIENT_SECRET=whoop_secret

# Provider 6: Custom Polar
export PIERRE_POLAR_CLIENT_ID=polar_client
export PIERRE_POLAR_CLIENT_SECRET=polar_secret

# ... unlimited providers ...
```

## dynamic discovery at runtime

Tools automatically discover all registered providers:

### connection status for all providers

**Request**:
```json
{
  "method": "tools/call",
  "params": {
    "name": "get_connection_status"
  }
}
```

**Response** (discovers all 1 to x providers):
```json
{
  "success": true,
  "result": {
    "providers": {
      "strava": { "connected": true, "status": "connected" },
      "garmin": { "connected": true, "status": "connected" },
      "fitbit": { "connected": false, "status": "disconnected" },
      "synthetic": { "connected": true, "status": "connected" },
      "whoop": { "connected": true, "status": "connected" },
      "polar": { "connected": false, "status": "disconnected" }
    }
  }
}
```

**Implementation** (`src/protocols/universal/handlers/connections.rs:84-110`):
```rust
// Multi-provider mode - check all supported providers from registry
let providers_to_check = executor.resources.provider_registry.supported_providers();
let mut providers_status = serde_json::Map::new();

for provider in providers_to_check {
    let is_connected = matches!(
        executor
            .auth_service
            .get_valid_token(user_uuid, provider, request.tenant_id.as_deref())
            .await,
        Ok(Some(_))
    );

    providers_status.insert(
        provider.to_owned(),
        serde_json::json!({
            "connected": is_connected,
            "status": if is_connected { "connected" } else { "disconnected" }
        }),
    );
}
```

**Key benefit**: No hardcoded provider lists! Add/remove providers without changing tool code.

### dynamic error messages

**Request** (invalid provider):
```json
{
  "method": "tools/call",
  "params": {
    "name": "connect_provider",
    "arguments": {
      "provider": "unknown_provider"
    }
  }
}
```

**Response** (automatically lists all registered providers):
```json
{
  "success": false,
  "error": "Provider 'unknown_provider' is not supported. Supported providers: strava, garmin, fitbit, synthetic, whoop, polar"
}
```

**Implementation** (`src/protocols/universal/handlers/connections.rs:332-340`):
```rust
if !is_provider_supported(provider, &executor.resources.provider_registry) {
    let supported_providers = executor
        .resources
        .provider_registry
        .supported_providers()
        .join(", ");
    return Ok(connection_error(format!(
        "Provider '{provider}' is not supported. Supported providers: {supported_providers}"
    )));
}
```

## provider factory implementations

Each provider implements `ProviderFactory`:

### strava factory

```rust
struct StravaProviderFactory;

impl ProviderFactory for StravaProviderFactory {
    fn create(&self, config: ProviderConfig) -> Box<dyn FitnessProvider> {
        Box::new(StravaProvider::new(config))
    }

    fn supported_providers(&self) -> &'static [&'static str] {
        &["strava"]
    }
}
```

### synthetic factory (oauth-free!)

```rust
struct SyntheticProviderFactory;

impl ProviderFactory for SyntheticProviderFactory {
    fn create(&self, _config: ProviderConfig) -> Box<dyn FitnessProvider> {
        // Ignores config - generates synthetic data
        Box::new(SyntheticProvider::default())
    }

    fn supported_providers(&self) -> &'static [&'static str] {
        &["synthetic"]
    }
}
```

### custom whoop factory (example)

```rust
pub struct WhoopProviderFactory;

impl ProviderFactory for WhoopProviderFactory {
    fn create(&self, config: ProviderConfig) -> Box<dyn FitnessProvider> {
        Box::new(WhoopProvider::new(config))
    }

    fn supported_providers(&self) -> &'static [&'static str] {
        &["whoop"]
    }
}
```

## simultaneous multi-provider usage

Users can connect to **all providers simultaneously** and aggregate data:

### example: aggregating activities from all connected providers

```rust
pub async fn get_all_activities_from_all_providers(
    user_id: Uuid,
    tenant_id: Uuid,
    registry: &ProviderRegistry,
    auth_service: &AuthService,
) -> Vec<Activity> {
    let mut all_activities = Vec::new();

    // Iterate through all registered providers
    for provider_name in registry.supported_providers() {
        // Check if user is connected to this provider
        if let Ok(Some(credentials)) = auth_service
            .get_valid_token(user_id, &provider_name, Some(&tenant_id.to_string()))
            .await
        {
            // Create provider instance
            if let Some(provider) = registry.create_provider(&provider_name) {
                // Set credentials
                if provider.set_credentials(credentials).await.is_ok() {
                    // Fetch activities
                    if let Ok(activities) = provider.get_activities(Some(50), None).await {
                        all_activities.extend(activities);
                    }
                }
            }
        }
    }

    // Sort by date (most recent first)
    all_activities.sort_by(|a, b| b.start_date.cmp(&a.start_date));

    // Deduplicate if needed (same activity synced to multiple providers)
    all_activities
}
```

**Result**: Activities from Strava, Garmin, Fitbit, Whoop, Polar all in one unified list!

## configuration best practices

### development (single provider)
```bash
# Use synthetic provider - no OAuth needed
export PIERRE_DEFAULT_PROVIDER=synthetic
```

### production (multi-provider deployment)
```bash
# Default to strava
export PIERRE_DEFAULT_PROVIDER=strava

# Configure all active providers
export PIERRE_STRAVA_CLIENT_ID=${STRAVA_CLIENT_ID_SECRET}
export PIERRE_STRAVA_CLIENT_SECRET=${STRAVA_SECRET}

export PIERRE_GARMIN_CLIENT_ID=${GARMIN_KEY}
export PIERRE_GARMIN_CLIENT_SECRET=${GARMIN_SECRET}

export PIERRE_FITBIT_CLIENT_ID=${FITBIT_KEY}
export PIERRE_FITBIT_CLIENT_SECRET=${FITBIT_SECRET}
```

### testing (mix synthetic + real)
```bash
# Test with both synthetic and real provider
export PIERRE_DEFAULT_PROVIDER=synthetic
export PIERRE_STRAVA_CLIENT_ID=test_id
export PIERRE_STRAVA_CLIENT_SECRET=test_secret
```

## summary

**1 to x providers simultaneously**:
- ✅ Register unlimited providers via factory pattern
- ✅ Each provider independently configured via environment variables
- ✅ Runtime discovery via `supported_providers()` and `is_supported()`
- ✅ Zero code changes to add/remove providers
- ✅ Tools automatically adapt to available providers
- ✅ Users can connect to all providers at once
- ✅ Data aggregation across multiple providers
- ✅ Synthetic provider for OAuth-free development

**Key files**:
- `src/providers/registry.rs` - Central registry managing all providers
- `src/providers/core.rs` - `FitnessProvider` trait and `ProviderFactory` trait
- `src/config/environment.rs` - Environment-based configuration loading
- `src/protocols/universal/handlers/connections.rs` - Dynamic provider discovery

For detailed implementation guide, see Chapter 17.5: Pluggable Provider Architecture.

---

# LLM Provider Integration

This document describes Pierre's LLM (Large Language Model) provider abstraction layer, which enables pluggable AI model integration with streaming support for the chat functionality.

## Overview

The LLM module provides a trait-based abstraction that allows Pierre to integrate with multiple AI providers (Gemini, OpenAI, Ollama, etc.) through a unified interface. The design mirrors the fitness provider SPI pattern for consistency.

```
┌─────────────────────────────────────────────────────────────────┐
│                    LlmProviderRegistry                          │
│              Manages multiple LLM providers                     │
└────────────────────────────┬────────────────────────────────────┘
                             │
         ┌───────────────────┼───────────────────┐
         │                   │                   │
         ▼                   ▼                   ▼
   ┌───────────┐      ┌───────────┐      ┌───────────┐
   │  Gemini   │      │  OpenAI   │      │  Ollama   │
   │ Provider  │      │ Provider  │      │ Provider  │
   └─────┬─────┘      └─────┬─────┘      └─────┬─────┘
         │                  │                   │
         └──────────────────┴───────────────────┘
                           │
                           ▼
               ┌───────────────────────┐
               │   LlmProvider Trait   │
               │   (shared interface)  │
               └───────────────────────┘
```

## Configuration

### Environment Variables

| Variable | Description | Required |
|----------|-------------|----------|
| `GEMINI_API_KEY` | Google Gemini API key | Yes (for Gemini) |

### Supported Models

#### Gemini (Default Provider)

| Model | Description | Default |
|-------|-------------|---------|
| `gemini-2.0-flash-exp` | Latest experimental flash model | ✓ |
| `gemini-1.5-pro` | Production-ready pro model | |
| `gemini-1.5-flash` | Fast, efficient model | |
| `gemini-1.0-pro` | Legacy pro model | |

## Quick Start

### Basic Usage

```rust
use pierre_mcp_server::llm::{
    GeminiProvider, LlmProvider, ChatMessage, ChatRequest,
};

// Create provider from environment variable
let provider = GeminiProvider::from_env()?;

// Build a chat request
let request = ChatRequest::new(vec![
    ChatMessage::system("You are a helpful fitness assistant."),
    ChatMessage::user("What's a good warm-up routine?"),
])
.with_temperature(0.7)
.with_max_tokens(1000);

// Get a response
let response = provider.complete(&request).await?;
println!("{}", response.content);
```

### Streaming Responses

```rust
use futures_util::StreamExt;

let request = ChatRequest::new(vec![
    ChatMessage::user("Explain the benefits of interval training"),
])
.with_streaming();

let mut stream = provider.complete_stream(&request).await?;

while let Some(chunk) = stream.next().await {
    match chunk {
        Ok(chunk) => {
            print!("{}", chunk.delta);
            if chunk.is_final {
                println!("\n[Done]");
            }
        }
        Err(e) => eprintln!("Error: {e}"),
    }
}
```

## API Reference

### LlmCapabilities

Bitflags indicating provider features:

| Flag | Description |
|------|-------------|
| `STREAMING` | Supports streaming responses |
| `FUNCTION_CALLING` | Supports function/tool calling |
| `VISION` | Supports image input |
| `JSON_MODE` | Supports structured JSON output |
| `SYSTEM_MESSAGES` | Supports system role messages |

```rust
// Check capabilities
let caps = provider.capabilities();
if caps.supports_streaming() {
    // Use streaming API
}
```

### ChatMessage

Message structure for conversations:

```rust
// Constructor methods
let system = ChatMessage::system("You are helpful");
let user = ChatMessage::user("Hello!");
let assistant = ChatMessage::assistant("Hi there!");
```

### ChatRequest

Request configuration with builder pattern:

```rust
let request = ChatRequest::new(messages)
    .with_model("gemini-1.5-pro")    // Override default model
    .with_temperature(0.7)            // 0.0 to 1.0
    .with_max_tokens(2000)            // Max output tokens
    .with_streaming();                // Enable streaming
```

### ChatResponse

Response structure:

| Field | Type | Description |
|-------|------|-------------|
| `content` | `String` | Generated text |
| `model` | `String` | Model used |
| `usage` | `Option<TokenUsage>` | Token counts |
| `finish_reason` | `Option<String>` | Why generation stopped |

### StreamChunk

Streaming chunk structure:

| Field | Type | Description |
|-------|------|-------------|
| `delta` | `String` | Incremental text |
| `is_final` | `bool` | Whether this is the last chunk |
| `finish_reason` | `Option<String>` | Reason if final |

## Provider Registry

The `LlmProviderRegistry` manages multiple providers:

```rust
use pierre_mcp_server::llm::LlmProviderRegistry;

let mut registry = LlmProviderRegistry::new();

// Register providers
registry.register(Box::new(GeminiProvider::from_env()?));
// registry.register(Box::new(OpenAIProvider::from_env()?));

// Set default
registry.set_default("gemini")?;

// Get provider by name
let provider = registry.get("gemini");

// List all registered
let names: Vec<&str> = registry.list();
```

## Adding New Providers

To implement a new LLM provider:

1. **Implement the trait**:

```rust
use async_trait::async_trait;
use pierre_mcp_server::llm::{
    LlmProvider, LlmCapabilities, ChatRequest, ChatResponse,
    ChatStream, AppError,
};

pub struct MyProvider {
    api_key: String,
    // ...
}

#[async_trait]
impl LlmProvider for MyProvider {
    fn name(&self) -> &'static str {
        "myprovider"
    }

    fn display_name(&self) -> &'static str {
        "My Custom Provider"
    }

    fn capabilities(&self) -> LlmCapabilities {
        LlmCapabilities::STREAMING | LlmCapabilities::SYSTEM_MESSAGES
    }

    fn default_model(&self) -> &'static str {
        "my-model-v1"
    }

    fn available_models(&self) -> &'static [&'static str] {
        &["my-model-v1", "my-model-v2"]
    }

    async fn complete(&self, request: &ChatRequest) -> Result<ChatResponse, AppError> {
        // Implementation
    }

    async fn complete_stream(&self, request: &ChatRequest) -> Result<ChatStream, AppError> {
        // Implementation
    }

    async fn health_check(&self) -> Result<bool, AppError> {
        // Implementation
    }
}
```

2. **Register the provider**:

```rust
registry.register(Box::new(MyProvider::new(api_key)));
```

## Error Handling

All provider methods return `Result<T, AppError>`:

```rust
match provider.complete(&request).await {
    Ok(response) => println!("{}", response.content),
    Err(AppError { code, message, .. }) => {
        match code {
            ErrorCode::RateLimitExceeded => // Handle rate limit
            ErrorCode::AuthenticationFailed => // Handle auth error
            _ => // Handle other errors
        }
    }
}
```

## Testing

Run LLM-specific tests:

```bash
# Unit tests
cargo test --test llm_test

# With output
cargo test --test llm_test -- --nocapture
```

## See Also

- Chapter 26: LLM Provider Architecture
- Configuration Guide
- Error Reference

---

# MCP Tools Reference

Comprehensive reference for all 47 Model Context Protocol (MCP) tools provided by Pierre Fitness Platform. These tools enable AI assistants to access fitness data, analyze performance, manage configurations, and provide personalized recommendations.

## Overview

Pierre MCP Server provides tools organized into 8 functional categories:
- **Core Fitness Tools**: Activity data and provider connections
- **Goals & Planning**: Goal setting and progress tracking
- **Performance Analysis**: Activity insights and trend analysis
- **Configuration Management**: System-wide configuration
- **Fitness Configuration**: User fitness zones and thresholds
- **Sleep & Recovery**: Sleep analysis and recovery tracking
- **Nutrition**: Dietary calculations and USDA food database
- **Recipe Management**: Training-aware meal planning and recipe storage

### Output Format

Most data-returning tools support an optional `format` parameter:
- `json` (default): Standard JSON output
- `toon`: Token-Oriented Object Notation for ~40% fewer LLM tokens

Use `format: "toon"` when querying large datasets (year summaries, batch analysis) to reduce LLM context usage.

---

## Core Fitness Tools

Basic fitness data retrieval and provider connection management.

| Tool Name | Description | Required Parameters | Optional Parameters |
|-----------|-------------|---------------------|---------------------|
| `get_activities` | Get user's fitness activities with optional filtering | `provider` (string) | `limit`, `offset`, `before`, `after`, `sport_type`, `mode`, `format` |
| `get_athlete` | Get user's athlete profile and basic information | `provider` (string) | `format` |
| `get_stats` | Get user's performance statistics and metrics | `provider` (string) | `format` |
| `get_connection_status` | Check OAuth connection status for fitness providers | - | `strava_client_id` (string), `strava_client_secret` (string), `fitbit_client_id` (string), `fitbit_client_secret` (string) |
| `connect_provider` | Connect to a fitness data provider via OAuth | `provider` (string) | - |
| `disconnect_provider` | Disconnect user from a fitness data provider | `provider` (string) | - |

### Parameter Details

**Supported Providers**: `strava`, `garmin`, `fitbit`, `whoop`, `terra`

**`get_activities` Parameters**:
- `provider`: Fitness provider name (e.g., 'strava', 'garmin', 'fitbit', 'whoop', 'terra')
- `limit`: Maximum number of activities to return
- `offset`: Number of activities to skip (for pagination)

**`get_connection_status` Parameters**:
- `strava_client_id`: Your Strava OAuth client ID (uses server defaults if not provided)
- `strava_client_secret`: Your Strava OAuth client secret
- `fitbit_client_id`: Your Fitbit OAuth client ID (uses server defaults if not provided)
- `fitbit_client_secret`: Your Fitbit OAuth client secret

---

## Goals & Planning

Tools for setting fitness goals, tracking progress, and receiving AI-powered goal suggestions.

| Tool Name | Description | Required Parameters | Optional Parameters |
|-----------|-------------|---------------------|---------------------|
| `set_goal` | Create and manage fitness goals with tracking | `title` (string), `goal_type` (string), `target_value` (number), `target_date` (string) | `description` (string), `sport_type` (string) |
| `suggest_goals` | Get AI-suggested fitness goals based on activity history | `provider` (string) | `goal_category` (string) |
| `analyze_goal_feasibility` | Analyze whether a goal is achievable given current fitness level | `goal_id` (string) | - |
| `track_progress` | Track progress towards fitness goals | `goal_id` (string) | - |

### Parameter Details

**`set_goal` Parameters**:
- `goal_type`: Type of goal - `distance`, `time`, `frequency`, `performance`, or `custom`
- `target_date`: Target completion date in ISO format (e.g., "2025-12-31")

**`suggest_goals` Parameters**:
- `goal_category`: Category of goals - `distance`, `performance`, `consistency`, or `all`

---

## Performance Analysis

Advanced analytics tools for activity analysis, trend detection, and performance predictions.

| Tool Name | Description | Required Parameters | Optional Parameters |
|-----------|-------------|---------------------|---------------------|
| `analyze_activity` | Analyze a specific activity with detailed performance insights | `provider` (string), `activity_id` (string) | - |
| `get_activity_intelligence` | Get AI-powered intelligence analysis for an activity | `provider` (string), `activity_id` (string) | `include_weather` (boolean), `include_location` (boolean) |
| `calculate_metrics` | Calculate custom fitness metrics and performance indicators | `provider` (string), `activity_id` (string) | `metrics` (array) |
| `analyze_performance_trends` | Analyze performance trends over time | `provider` (string), `timeframe` (string), `metric` (string) | `sport_type` (string) |
| `compare_activities` | Compare two activities for performance analysis | `provider` (string), `activity_id` (string), `comparison_type` (string) | - |
| `detect_patterns` | Detect patterns and insights in activity data | `provider` (string), `pattern_type` (string) | `timeframe` (string) |
| `generate_recommendations` | Generate personalized training recommendations | `provider` (string) | `recommendation_type` (string), `activity_id` (string) |
| `calculate_fitness_score` | Calculate overall fitness score based on recent activities | `provider` (string) | `timeframe` (string), `sleep_provider` (string) |
| `predict_performance` | Predict future performance based on training patterns | `provider` (string), `target_sport` (string), `target_distance` (number) | `target_date` (string) |
| `analyze_training_load` | Analyze training load and recovery metrics | `provider` (string) | `timeframe` (string), `sleep_provider` (string) |

### Parameter Details

**`get_activity_intelligence` Parameters**:
- `include_weather`: Whether to include weather analysis (default: true)
- `include_location`: Whether to include location intelligence (default: true)

**`calculate_metrics` Parameters**:
- `metrics`: Array of specific metrics to calculate (e.g., `['trimp', 'power_to_weight', 'efficiency']`)

**`analyze_performance_trends` Parameters**:
- `timeframe`: Time period - `week`, `month`, `quarter`, `sixmonths`, or `year`
- `metric`: Metric to analyze - `pace`, `heart_rate`, `power`, `distance`, or `duration`

**`compare_activities` Parameters**:
- `comparison_type`: Type of comparison - `similar_activities`, `personal_best`, `average`, or `recent`

**`detect_patterns` Parameters**:
- `pattern_type`: Pattern to detect - `training_consistency`, `seasonal_trends`, `performance_plateaus`, or `injury_risk`

**`generate_recommendations` Parameters**:
- `recommendation_type`: Type of recommendations - `training`, `recovery`, `nutrition`, `equipment`, or `all`

**`calculate_fitness_score` Parameters** (Cross-Provider Support):
- `timeframe`: Analysis period - `month`, `last_90_days`, or `all_time`
- `sleep_provider`: Optional sleep/recovery provider for cross-provider analysis (e.g., `whoop`, `garmin`). When specified, recovery quality factors into the fitness score:
  - Excellent recovery (90-100): +5% fitness score bonus
  - Good recovery (70-89): No adjustment
  - Moderate recovery (50-69): -5% penalty
  - Poor recovery (<50): -10% penalty

**`analyze_training_load` Parameters** (Cross-Provider Support):
- `timeframe`: Analysis period - `week`, `month`, etc.
- `sleep_provider`: Optional sleep/recovery provider for cross-provider analysis. Adds recovery context to training load analysis including sleep quality score, HRV data, and recovery status.

---

## Configuration Management

System-wide configuration management tools for physiological parameters and training zones.

| Tool Name | Description | Required Parameters | Optional Parameters |
|-----------|-------------|---------------------|---------------------|
| `get_configuration_catalog` | Get complete configuration catalog with all available parameters | - | - |
| `get_configuration_profiles` | Get available configuration profiles (Research, Elite, Recreational, etc.) | - | - |
| `get_user_configuration` | Get current user's configuration settings and overrides | - | - |
| `update_user_configuration` | Update user's configuration parameters and session overrides | - | `profile` (string), `parameters` (object) |
| `calculate_personalized_zones` | Calculate personalized training zones based on VO2 max | `vo2_max` (number) | `resting_hr` (number), `max_hr` (number), `lactate_threshold` (number), `sport_efficiency` (number) |
| `validate_configuration` | Validate configuration parameters against safety rules | `parameters` (object) | - |

### Parameter Details

**`update_user_configuration` Parameters**:
- `profile`: Configuration profile to apply (e.g., 'Research', 'Elite', 'Recreational', 'Beginner', 'Medical')
- `parameters`: Parameter overrides as JSON object

**`calculate_personalized_zones` Parameters**:
- `vo2_max`: VO2 max in ml/kg/min
- `resting_hr`: Resting heart rate in bpm (default: 60)
- `max_hr`: Maximum heart rate in bpm (default: 190)
- `lactate_threshold`: Lactate threshold as percentage of VO2 max (default: 0.85)
- `sport_efficiency`: Sport efficiency factor (default: 1.0)

---

## Fitness Configuration

User-specific fitness configuration for heart rate zones, power zones, and training thresholds.

| Tool Name | Description | Required Parameters | Optional Parameters |
|-----------|-------------|---------------------|---------------------|
| `get_fitness_config` | Get user fitness configuration settings | - | `configuration_name` (string) |
| `set_fitness_config` | Save user fitness configuration settings | `configuration` (object) | `configuration_name` (string) |
| `list_fitness_configs` | List all fitness configuration names | - | - |
| `delete_fitness_config` | Delete a specific fitness configuration | `configuration_name` (string) | - |

### Parameter Details

**`get_fitness_config` / `set_fitness_config` Parameters**:
- `configuration_name`: Name of the configuration (defaults to 'default')
- `configuration`: Fitness configuration object containing zones, thresholds, and training parameters

**Configuration Object Structure**:
```json
{
  "heart_rate_zones": {
    "zone1": {"min": 100, "max": 120},
    "zone2": {"min": 120, "max": 140},
    "zone3": {"min": 140, "max": 160},
    "zone4": {"min": 160, "max": 180},
    "zone5": {"min": 180, "max": 200}
  },
  "power_zones": { /* similar structure */ },
  "ftp": 250,
  "lthr": 165,
  "max_hr": 190,
  "resting_hr": 50,
  "weight_kg": 70
}
```

---

## Sleep & Recovery

Sleep quality analysis and recovery monitoring tools using NSF/AASM guidelines. These tools support **cross-provider data fetching**, allowing you to use activities from one provider and sleep/recovery data from another.

| Tool Name | Description | Required Parameters | Optional Parameters |
|-----------|-------------|---------------------|---------------------|
| `analyze_sleep_quality` | Analyze sleep quality from provider data or manual input | Either `sleep_provider` OR `sleep_data` | `activity_provider`, `days_back`, `recent_hrv_values`, `baseline_hrv` |
| `calculate_recovery_score` | Calculate holistic recovery score combining TSB, sleep, and HRV | Either `activity_provider` OR `sleep_provider` | `sleep_provider`, `activity_provider`, `user_config` |
| `suggest_rest_day` | AI-powered rest day recommendation | Either `activity_provider` OR `sleep_data` | `activity_provider`, `sleep_provider`, `training_load`, `recovery_score` |
| `track_sleep_trends` | Track sleep patterns over time | Either `sleep_provider` OR `sleep_history` | `days_back` |
| `optimize_sleep_schedule` | Optimize sleep duration based on training load | Either `activity_provider` OR `sleep_history` | `activity_provider`, `sleep_provider`, `target_sleep_hours`, `training_schedule` |

### Cross-Provider Support

Sleep and recovery tools support fetching data from different providers for activities and sleep. This enables scenarios like:

- **Strava + WHOOP**: Activities from Strava, recovery/sleep data from WHOOP
- **Garmin + Fitbit**: Running data from Garmin, sleep tracking from Fitbit
- **Any activity provider + Any sleep provider**: Mix and match based on your device ecosystem

**Provider Priority (when auto-selecting)**:
- **Activity providers**: strava > garmin > fitbit > whoop > terra > synthetic
- **Sleep providers**: whoop > garmin > fitbit > terra > synthetic

**Example: Cross-Provider Recovery Score**:
```json
{
  "tool": "calculate_recovery_score",
  "parameters": {
    "activity_provider": "strava",
    "sleep_provider": "whoop"
  }
}
```

**Response includes providers used**:
```json
{
  "recovery_score": { ... },
  "providers_used": {
    "activity_provider": "strava",
    "sleep_provider": "whoop"
  }
}
```

### Parameter Details

**`analyze_sleep_quality` Sleep Data Object** (for manual input mode):
```json
{
  "date": "2025-11-28",
  "duration_hours": 7.5,
  "efficiency_percent": 85,
  "deep_sleep_hours": 1.5,
  "rem_sleep_hours": 2.0,
  "light_sleep_hours": 4.0,
  "awakenings": 2,
  "hrv_rmssd_ms": 45
}
```

**`calculate_recovery_score` / `optimize_sleep_schedule` User Config**:
```json
{
  "ftp": 250,
  "lthr": 165,
  "max_hr": 190,
  "resting_hr": 50,
  "weight_kg": 70
}
```

**`track_sleep_trends` Parameters**:
- `sleep_history`: Array of sleep data objects (minimum 7 days required)
- `sleep_provider`: Provider name to fetch sleep history from (alternative to `sleep_history`)
- `days_back`: Number of days to analyze (default: 14)

**`optimize_sleep_schedule` Parameters**:
- `activity_provider`: Provider for activity data
- `sleep_provider`: Provider for sleep data (optional, can be same as activity_provider)
- `target_sleep_hours`: Target sleep duration in hours (default: 8.0)
- `training_schedule`: Weekly training schedule object

---

## Nutrition

Nutrition calculation tools with USDA FoodData Central database integration.

| Tool Name | Description | Required Parameters | Optional Parameters |
|-----------|-------------|---------------------|---------------------|
| `calculate_daily_nutrition` | Calculate daily calorie and macronutrient needs (Mifflin-St Jeor) | `weight_kg` (number), `height_cm` (number), `age` (number), `gender` (string), `activity_level` (string), `training_goal` (string) | - |
| `get_nutrient_timing` | Get optimal pre/post-workout nutrition (ISSN guidelines) | `weight_kg` (number), `daily_protein_g` (number) | `workout_intensity` (string), `activity_provider` (string), `days_back` (number) |
| `search_food` | Search USDA FoodData Central database | `query` (string) | `page_size` (number) |
| `get_food_details` | Get detailed nutritional information for a food | `fdc_id` (number) | - |
| `analyze_meal_nutrition` | Analyze total calories and macros for a meal | `foods` (array) | - |

### Parameter Details

**`calculate_daily_nutrition` Parameters**:
- `gender`: Either `male` or `female`
- `activity_level`: `sedentary`, `lightly_active`, `moderately_active`, `very_active`, or `extra_active`
- `training_goal`: `maintenance`, `weight_loss`, `muscle_gain`, or `endurance_performance`
- `age`: Age in years (max 150)

**`get_nutrient_timing` Parameters**:
- `workout_intensity`: Workout intensity level - `low`, `moderate`, or `high` (required if `activity_provider` not specified)
- `activity_provider`: Fitness provider for activity data (e.g., `strava`, `garmin`). When specified, workout intensity is auto-inferred from recent training load
- `days_back`: Number of days of activity history to analyze for intensity inference (default: 7, max: 30)

**Cross-Provider Support**: When using `activity_provider`, the tool analyzes your recent training data to automatically determine workout intensity based on training volume and heart rate patterns:
- **High intensity**: >2 hours/day or average HR >150 bpm
- **Moderate intensity**: 1-2 hours/day or average HR 130-150 bpm
- **Low intensity**: <1 hour/day and average HR <130 bpm

**`search_food` Parameters**:
- `query`: Food name or description to search for
- `page_size`: Number of results to return (default: 10, max: 200)

**`get_food_details` Parameters**:
- `fdc_id`: USDA FoodData Central ID (obtained from `search_food` results)

**`analyze_meal_nutrition` Foods Array**:
```json
{
  "foods": [
    {"fdc_id": 171705, "grams": 100},
    {"fdc_id": 173424, "grams": 50}
  ]
}
```

---

## Usage Examples

### Connecting to a Provider
```json
{
  "tool": "connect_provider",
  "parameters": {
    "provider": "strava"
  }
}
```

### Getting Recent Activities
```json
{
  "tool": "get_activities",
  "parameters": {
    "provider": "strava",
    "limit": 10,
    "offset": 0
  }
}
```

### Analyzing Activity Intelligence
```json
{
  "tool": "get_activity_intelligence",
  "parameters": {
    "provider": "strava",
    "activity_id": "12345678",
    "include_weather": true,
    "include_location": true
  }
}
```

### Setting a Fitness Goal
```json
{
  "tool": "set_goal",
  "parameters": {
    "title": "Run 100km this month",
    "goal_type": "distance",
    "target_value": 100000,
    "target_date": "2025-12-31",
    "sport_type": "Run"
  }
}
```

### Calculating Daily Nutrition
```json
{
  "tool": "calculate_daily_nutrition",
  "parameters": {
    "weight_kg": 70,
    "height_cm": 175,
    "age": 30,
    "gender": "male",
    "activity_level": "very_active",
    "training_goal": "endurance_performance"
  }
}
```

### Analyzing Sleep Quality

**Using a sleep provider** (recommended):
```json
{
  "tool": "analyze_sleep_quality",
  "parameters": {
    "sleep_provider": "whoop",
    "days_back": 7
  }
}
```

**Cross-provider analysis** (activities from Strava, sleep from WHOOP):
```json
{
  "tool": "analyze_sleep_quality",
  "parameters": {
    "activity_provider": "strava",
    "sleep_provider": "whoop"
  }
}
```

**Manual sleep data input** (for providers without direct integration):
```json
{
  "tool": "analyze_sleep_quality",
  "parameters": {
    "sleep_data": {
      "date": "2025-11-28",
      "duration_hours": 7.5,
      "efficiency_percent": 85,
      "deep_sleep_hours": 1.5,
      "rem_sleep_hours": 2.0,
      "light_sleep_hours": 4.0,
      "awakenings": 2,
      "hrv_rmssd_ms": 45
    }
  }
}
```

---

## Recipe Management

Training-aware recipe management tools for meal planning aligned with workout schedules. Uses the "Combat des Chefs" architecture where LLM clients generate recipes and Pierre validates nutrition via USDA.

| Tool Name | Description | Required Parameters | Optional Parameters |
|-----------|-------------|---------------------|---------------------|
| `get_recipe_constraints` | Get macro targets and guidelines for meal timing | - | `meal_timing` (string), `target_calories` (number) |
| `validate_recipe` | Validate recipe nutrition against training targets | `name` (string), `ingredients` (array), `meal_timing` (string) | `target_calories` (number), `dietary_restrictions` (array) |
| `save_recipe` | Save validated recipe to user's collection | `name` (string), `ingredients` (array), `meal_timing` (string) | `description` (string), `servings` (number), `prep_time_minutes` (number), `cook_time_minutes` (number), `instructions` (array), `tags` (array), `dietary_restrictions` (array), `skill_level` (string), `source` (string) |
| `list_recipes` | List user's saved recipes | - | `meal_timing` (string), `tags` (array), `limit` (number), `offset` (number) |
| `get_recipe` | Get a specific recipe by ID | `recipe_id` (string) | - |
| `delete_recipe` | Delete a recipe from user's collection | `recipe_id` (string) | - |
| `search_recipes` | Search recipes by name, ingredients, or tags | `query` (string) | `meal_timing` (string), `limit` (number) |

### Parameter Details

**Meal Timing Values**:
- `pre_training`: High-carb focus (55% carbs, 20% protein, 25% fat)
- `post_training`: High-protein focus (45% carbs, 30% protein, 25% fat)
- `rest_day`: Lower carb, moderate protein (35% carbs, 30% protein, 35% fat)
- `general`: Balanced macros (45% carbs, 25% protein, 30% fat)

**Ingredient Object Structure**:
```json
{
  "name": "chicken breast",
  "quantity": 200,
  "unit": "grams",
  "fdc_id": 171077
}
```

**Supported Units** (auto-converted to grams):
- Weight: `grams`, `g`, `oz`, `ounces`, `lb`, `pounds`, `kg`
- Volume: `ml`, `milliliters`, `cups`, `cup`, `tbsp`, `tablespoon`, `tsp`, `teaspoon`
- Count: `pieces`, `piece`, `whole`

**Skill Level Values**: `beginner`, `intermediate`, `advanced`

**Dietary Restrictions**: `vegetarian`, `vegan`, `gluten_free`, `dairy_free`, `nut_free`, `keto`, `paleo`

**Example: Validate a Post-Workout Recipe**:
```json
{
  "tool": "validate_recipe",
  "parameters": {
    "name": "Post-Workout Protein Bowl",
    "meal_timing": "post_training",
    "target_calories": 600,
    "ingredients": [
      {"name": "chicken breast", "quantity": 200, "unit": "grams"},
      {"name": "brown rice", "quantity": 1, "unit": "cup"},
      {"name": "broccoli", "quantity": 150, "unit": "grams"}
    ]
  }
}
```

**Example: Save a Recipe**:
```json
{
  "tool": "save_recipe",
  "parameters": {
    "name": "Recovery Shake",
    "meal_timing": "post_training",
    "description": "Quick protein shake for post-workout recovery",
    "servings": 1,
    "prep_time_minutes": 5,
    "ingredients": [
      {"name": "whey protein powder", "quantity": 30, "unit": "grams"},
      {"name": "banana", "quantity": 1, "unit": "piece"},
      {"name": "almond milk", "quantity": 1, "unit": "cup"}
    ],
    "instructions": ["Add all ingredients to blender", "Blend until smooth"],
    "tags": ["quick", "shake", "high-protein"],
    "skill_level": "beginner"
  }
}
```

---

## Notes

- **Authentication**: Most tools require OAuth authentication with Pierre and the respective fitness provider
- **Provider Support**: Supports Strava, Garmin, Fitbit, WHOOP, and Terra (150+ wearables) providers
- **Rate Limits**: Subject to provider API rate limits (e.g., Strava: 100 requests per 15 minutes, 1000 per day)
- **Token Refresh**: OAuth tokens are automatically refreshed when expired
- **USDA Database**: Food search tools use free USDA FoodData Central API with 24-hour caching
- **Scientific Guidelines**:
  - Sleep analysis follows NSF (National Sleep Foundation) and AASM (American Academy of Sleep Medicine) guidelines
  - Nutrition recommendations follow ISSN (International Society of Sports Nutrition) guidelines
  - BMR calculations use validated Mifflin-St Jeor formula

---

## Tool Categories Summary

| Category | Tool Count | Description |
|----------|------------|-------------|
| Core Fitness | 6 | Activity data and provider connections |
| Goals & Planning | 4 | Goal management and progress tracking |
| Performance Analysis | 10 | Activity analytics and predictions |
| Configuration Management | 6 | System configuration and zones |
| Fitness Configuration | 4 | User fitness settings |
| Sleep & Recovery | 5 | Sleep analysis and recovery metrics |
| Nutrition | 5 | Dietary calculations and food database |
| Recipe Management | 7 | Training-aware meal planning and recipes |
| **Total** | **47** | **Complete MCP tool suite** |

---

## Additional Resources

- MCP Protocol Specification
- Pierre MCP Server Repository
- Development Guide
- Testing Guide
- Configuration Guide

---

*Last Updated: 2025-12-06*
*Pierre Fitness Platform v1.0.0*

---

# Pierre intelligence and analytics methodology

## What this document covers

This comprehensive guide explains the scientific methods, algorithms, and decision rules behind pierre's analytics engine. It provides transparency into:

- **mathematical foundations**: formulas, statistical methods, and physiological models
- **data sources and processing**: inputs, validation, and transformation pipelines
- **calculation methodologies**: step-by-step algorithms with code examples
- **scientific references**: peer-reviewed research backing each metric
- **implementation details**: rust code architecture and design patterns
- **limitations and guardrails**: edge cases, confidence levels, and safety mechanisms
- **verification**: validation against published sports science data

**algorithm implementation**: all algorithms described in this document are implemented using enum-based dependency injection for runtime configuration flexibility. Each algorithm category (max heart rate, TRIMP, TSS, VDOT, training load, recovery, FTP, LTHR, VO2max) supports multiple variants selectable via environment variables. See configuration.md for available algorithm variants and architecture.md for implementation details.

---

## Table of contents

### Core Architecture
- architecture overview
  - foundation modules
  - core modules
  - intelligence tools (47 tools)
- data sources and permissions
  - primary data
  - user profile (optional)
  - configuration
  - provider normalization
  - data retention and privacy

### Personalization And Zones
- personalization engine
  - age-based max heart rate estimation
  - heart rate zones
  - power zones (cycling)

### Core Metrics And Calculations
- core metrics
  - pace vs speed
- training stress score (TSS)
  - power-based TSS (preferred)
  - heart rate-based TSS (hrTSS)
- normalized power (NP)
- chronic training load (CTL) and acute training load (ATL)
  - mathematical formulation
- training stress balance (TSB)
- overtraining risk detection

### Statistical Analysis
- statistical trend analysis

### Performance Prediction
- performance prediction: VDOT
  - VDOT calculation from race performance
  - race time prediction from VDOT
  - VDOT accuracy verification ✅
- performance prediction: riegel formula

### Pattern Recognition
- pattern detection
  - weekly schedule
  - hard/easy alternation
  - volume progression

### Sleep And Recovery
- sleep and recovery analysis
  - sleep quality scoring
  - recovery score calculation
  - configuration

### Validation And Safety
- validation and safety
  - parameter bounds (physiological ranges)
  - confidence levels
  - edge case handling

### Configuration
- configuration strategies
  - conservative strategy
  - default strategy
  - aggressive strategy

### Testing And Quality
- testing and verification
  - test coverage
  - verification methods

### Debugging Guide
- debugging and validation guide
  - general debugging workflow
  - metric-specific debugging
  - common platform-specific issues
  - data quality validation
  - when to contact support
  - debugging tools and utilities

### Reference Information
- limitations
  - model assumptions
  - known issues
  - prediction accuracy
- references
  - scientific literature
- faq
- glossary

---

## Architecture Overview

Pierre's intelligence system uses a **foundation modules** approach for code reuse and consistency:

```
┌─────────────────────────────────────────────┐
│   mcp/a2a protocol layer                    │
│   (src/protocols/universal/)                │
└──────────────────┬──────────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────────┐
│   intelligence tools (47 tools)             │
│   (src/protocols/universal/handlers/)       │
└──────────────────┬──────────────────────────┘
                   │
    ┌──────────────┼──────────────────┬───────────┬────────────┐
    ▼              ▼                  ▼           ▼            ▼
┌─────────────┐ ┌──────────────┐ ┌──────────┐ ┌───────────┐ ┌──────────────┐
│ Training    │ │ Performance  │ │ Pattern  │ │Statistical│ │ Sleep &      │
│ Load Calc   │ │ Predictor    │ │ Detector │ │ Analyzer  │ │ Recovery     │
│             │ │              │ │          │ │           │ │              │
│ TSS/CTL/ATL │ │ VDOT/Riegel  │ │ Weekly   │ │Regression │ │ Sleep Score  │
│ TSB/Risk    │ │ Race Times   │ │ Patterns │ │ Trends    │ │ Recovery Calc│
└─────────────┘ └──────────────┘ └──────────┘ └───────────┘ └──────────────┘
                    FOUNDATION MODULES
             Shared by all intelligence tools
```

### Foundation Modules

**`src/intelligence/training_load.rs`** - training stress calculations
- TSS (Training Stress Score) from power or heart rate
- CTL (Chronic Training Load) - 42-day EMA for fitness
- ATL (Acute Training Load) - 7-day EMA for fatigue
- TSB (Training Stress Balance) - form indicator
- Overtraining risk assessment with 3 risk factors
- Gap handling: zero-fills missing days in EMA calculation

**`src/intelligence/performance_prediction.rs`** - race predictions
- VDOT calculation from race performance (Jack Daniels formula)
- Race time prediction for 5K, 10K, 15K, Half Marathon, Marathon
- Riegel formula for distance-based predictions
- Accuracy: 0.2-5.5% vs. published VDOT tables
- Verified against VDOT 40, 50, 60 reference values

**`src/intelligence/pattern_detection.rs`** - pattern recognition
- Weekly schedule detection with consistency scoring
- Hard/easy alternation pattern analysis
- Volume progression trend detection (increasing/stable/decreasing)
- Overtraining signals detection (3 risk factors)

**`src/intelligence/statistical_analysis.rs`** - statistical methods
- Linear regression with R² calculation
- Trend detection (improving/stable/declining)
- Correlation analysis
- Moving averages and smoothing
- Significance level assessment

**`src/intelligence/sleep_analysis.rs`** - sleep quality scoring
- Duration scoring with NSF guidelines (7-9 hours optimal for adults, 8-10 for athletes)
- Stages scoring with AASM recommendations (deep 15-25%, REM 20-25%)
- Efficiency scoring with clinical thresholds (excellent >90%, good >85%, poor <70%)
- Overall quality calculation (weighted average of components)
- Dependency injection with `SleepRecoveryConfig` for all thresholds

**`src/intelligence/recovery_calculator.rs`** - recovery assessment
- TSB normalization (-30 to +30 → 0-100 recovery score)
- HRV scoring based on RMSSD baseline comparison (±3ms stable, >5ms good recovery)
- Weighted recovery calculation (40% TSB, 40% sleep, 20% HRV when available)
- Fallback scoring when HRV unavailable (50% TSB, 50% sleep)
- Recovery classification (excellent/good/fair/poor) with actionable thresholds
- Dependency injection with `SleepRecoveryConfig` for configurability

### Core Modules

**`src/intelligence/metrics.rs`** - advanced metrics calculation
**`src/intelligence/performance_analyzer_v2.rs`** - performance analysis framework
**`src/intelligence/physiological_constants.rs`** - sport science constants
**`src/intelligence/recommendation_engine.rs`** - training recommendations
**`src/intelligence/goal_engine.rs`** - goal tracking and progress

### Intelligence Tools (47 tools)

All 47 MCP tools now use real calculations from foundation modules:

**group 1: analysis** (use StatisticalAnalyzer + PatternDetector)
- analyze_performance_trends
- detect_patterns
- compare_activities

**group 2: recommendations** (use TrainingLoadCalculator + PatternDetector)
- generate_recommendations
- calculate_fitness_score
- analyze_training_load

**group 3: predictions** (use PerformancePredictor)
- predict_performance

**group 4: configuration** (use physiological_constants validation)
- validate_configuration (ranges + relationships)
- suggest_goals (real profile from activities)

**group 5: goals** (use 10% improvement rule)
- analyze_goal_feasibility

**group 6: sleep and recovery** (use SleepAnalyzer + RecoveryCalculator)
- analyze_sleep_quality (NSF/AASM-based scoring)
- calculate_recovery_score (TSB + sleep + HRV)
- track_sleep_trends (longitudinal analysis)
- optimize_sleep_schedule (personalized timing)
- get_rest_day_recommendations (training load-based)

---

## Data Sources And Permissions

### Primary Data
Fitness activities via oauth2 authorization from multiple providers:

**supported providers**: strava, garmin, fitbit, whoop

**activity data**:
- **temporal**: `start_date`, `elapsed_time`, `moving_time`
- **spatial**: `distance`, `total_elevation_gain`, GPS polyline (optional)
- **physiological**: `average_heartrate`, `max_heartrate`, heart rate stream
- **power**: `average_watts`, `weighted_average_watts`, `kilojoules`, power stream (strava, garmin)
- **sport metadata**: `type`, `sport_type`, `workout_type`

### User Profile (optional)
- **demographics**: `age`, `gender`, `weight_kg`, `height_cm`
- **thresholds**: `max_hr`, `resting_hr`, `lthr`, `ftp`, `cp`, `vo2max`
- **preferences**: `units`, `training_focus`, `injury_history`
- **fitness level**: `beginner`, `intermediate`, `advanced`, `elite`

### Configuration
- **strategy**: `conservative`, `default`, `aggressive` (affects thresholds)
- **units**: metric (km, m, kg) or imperial (mi, ft, lb)
- **zone model**: karvonen (HR reserve) or percentage max HR

### Provider Normalization
Pierre normalizes data from different providers into a unified format:

```rust
// src/providers/ - unified activity model
pub struct Activity {
    pub provider: Provider, // Strava, Garmin, Fitbit
    pub start_date: DateTime<Utc>,
    pub distance: Option<f64>,
    pub moving_time: u64,
    pub sport_type: String,
    // ... normalized fields
}
```

**provider-specific features**:
- **strava**: full power metrics, segments, kudos
- **garmin**: advanced running dynamics, training effect, recovery time
- **fitbit**: all-day heart rate, sleep tracking, steps
- **whoop**: strain scores, recovery metrics, sleep stages, HRV data

### Data Retention And Privacy
- activities cached for 7 days (configurable)
- analysis results cached for 24 hours
- token revocation purges all cached data within 1 hour
- no third-party data sharing
- encryption: AES-256-GCM for tokens, tenant-specific keys
- provider tokens stored separately, isolated per tenant

---

## Personalization Engine

### Age-based Max Heart Rate Estimation

When `max_hr` not provided, pierre uses the classic fox formula:

**formula**:

```
max_hr(age) = 220 − age
```

**bounds**:

```
max_hr ∈ [160, 220] bpm to exclude physiologically implausible values
```

**rust implementation**:

```rust
// src/intelligence/physiological_constants.rs
pub const AGE_BASED_MAX_HR_CONSTANT: u32 = 220;
pub const MAX_REALISTIC_HEART_RATE: u32 = 220;

fn estimate_max_hr(age: i32) -> u32 {
    let estimated = AGE_BASED_MAX_HR_CONSTANT - age as u32;
    estimated.clamp(160, MAX_REALISTIC_HEART_RATE)
}
```

**reference**: Fox, S.M., Naughton, J.P., & Haskell, W.L. (1971). Physical activity and the prevention of coronary heart disease. *Annals of Clinical Research*, 3(6), 404-432.

**note**: while newer research suggests the Tanaka formula (`208 − 0.7 × age`) may be more accurate, pierre uses the classic Fox formula (`220 − age`) for simplicity and widespread familiarity. The difference is typically 3-8 bpm for ages 20-60.

### Heart Rate Zones

Pierre's HR zone calculations use **karvonen method** (HR reserve) internally for threshold determination:

**karvonen formula**:

```
target_hr(intensity%) = (HR_reserve × intensity%) + HR_rest
```

Where:
- `HR_reserve = HR_max − HR_rest`
- `intensity% ∈ [0, 1]`

**five-zone model** (used internally):

```
Zone 1 (Recovery):  [HR_rest + 0.50 × HR_reserve, HR_rest + 0.60 × HR_reserve]
Zone 2 (Endurance): [HR_rest + 0.60 × HR_reserve, HR_rest + 0.70 × HR_reserve]
Zone 3 (Tempo):     [HR_rest + 0.70 × HR_reserve, HR_rest + 0.80 × HR_reserve]
Zone 4 (Threshold): [HR_rest + 0.80 × HR_reserve, HR_rest + 0.90 × HR_reserve]
Zone 5 (VO2max):    [HR_rest + 0.90 × HR_reserve, HR_max]
```

**important note**: while pierre uses karvonen-based constants for internal HR zone classification (see `src/intelligence/physiological_constants.rs`), there is **no public API helper function** for calculating HR zones. Users must implement their own zone calculation using the formula above.

**internal constants** (reference implementation):

```rust
// src/intelligence/physiological_constants.rs
pub const ANAEROBIC_THRESHOLD_PERCENT: f64 = 0.85; // 85% of HR reserve
pub const AEROBIC_THRESHOLD_PERCENT: f64 = 0.70;   // 70% of HR reserve
```

**fallback**: when `resting_hr` unavailable, pierre uses simple percentage of `max_hr` for intensity classification.

**reference**: Karvonen, M.J., Kentala, E., & Mustala, O. (1957). The effects of training on heart rate; a longitudinal study. *Annales medicinae experimentalis et biologiae Fenniae*, 35(3), 307-315.

### Power Zones (cycling)

Five-zone model based on functional threshold power (FTP):

**power zones**:

```
Zone 1 (Active Recovery): [0, 0.55 × FTP)
Zone 2 (Endurance):       [0.55 × FTP, 0.75 × FTP)
Zone 3 (Tempo):           [0.75 × FTP, 0.90 × FTP)
Zone 4 (Threshold):       [0.90 × FTP, 1.05 × FTP)
Zone 5 (VO2max+):         [1.05 × FTP, ∞)
```

**rust implementation**:

```rust
// src/intelligence/physiological_constants.rs
pub fn calculate_power_zones(ftp: f64) -> PowerZones {
    PowerZones {
        zone1: (0.0,         ftp * 0.55), // Active recovery
        zone2: (ftp * 0.55,  ftp * 0.75), // Endurance
        zone3: (ftp * 0.75,  ftp * 0.90), // Tempo
        zone4: (ftp * 0.90,  ftp * 1.05), // Threshold
        zone5: (ftp * 1.05,  f64::MAX),   // VO2max+
    }
}
```

**physiological adaptations**:
- **Z1 (active recovery)**: < 55% FTP - flush metabolites, active rest
- **Z2 (endurance)**: 55-75% FTP - aerobic base building
- **Z3 (tempo)**: 75-90% FTP - muscular endurance
- **Z4 (threshold)**: 90-105% FTP - lactate threshold work
- **Z5 (VO2max+)**: > 105% FTP - maximal aerobic/anaerobic efforts

**reference**: Coggan, A. & Allen, H. (2010). *Training and Racing with a Power Meter* (2nd ed.). VeloPress.

---

## Core Metrics

### Pace Vs Speed

**pace formula** (time per distance, seconds per kilometer):

```
pace(d, t) = 0,              if d < 1 meter
           = t / (d / 1000), if d ≥ 1 meter
```

Where:
- `t` = moving time (seconds)
- `d` = distance (meters)

**speed formula** (distance per time, meters per second):

```
speed(d, t) = 0,      if t = 0
            = d / t,  if t > 0
```

Where:
- `d` = distance (meters)
- `t` = moving time (seconds)

**rust implementation**:

```rust
// src/intelligence/metrics.rs

// pace: time per distance (seconds per km)
pub fn calculate_pace(moving_time_s: u64, distance_m: f64) -> f64 {
    if distance_m < 1.0 { return 0.0; }
    (moving_time_s as f64) / (distance_m / 1000.0)
}

// speed: distance per time (m/s)
pub fn calculate_speed(distance_m: f64, moving_time_s: u64) -> f64 {
    if moving_time_s == 0 { return 0.0; }
    distance_m / (moving_time_s as f64)
}
```

---

## Training Stress Score (TSS)

TSS quantifies training load accounting for intensity and duration.

### Power-based TSS (preferred)

**formula**:

```
TSS = duration_hours × IF² × 100
```

Where:
- `IF` = intensity factor = `avg_power / FTP`
- `avg_power` = average power for the activity (watts)
- `FTP` = functional threshold power (watts)
- `duration_hours` = activity duration (hours)

**important note**: pierre uses **average power**, not normalized power (NP), for TSS calculations. While NP (see normalized power section) better accounts for variability in cycling efforts, the current implementation uses simple average power for consistency and computational efficiency.

**rust implementation**:

```rust
// src/intelligence/metrics.rs
fn calculate_tss(avg_power: u32, ftp: f64, duration_hours: f64) -> f64 {
    let intensity_factor = f64::from(avg_power) / ftp;
    (duration_hours * intensity_factor * intensity_factor * TSS_BASE_MULTIPLIER).round()
}
```

Where `TSS_BASE_MULTIPLIER = 100.0`

**input/output specification**:

```
Inputs:
  avg_power: u32          // Average watts for activity, must be > 0
  duration_hours: f64     // Activity duration, must be > 0
  ftp: f64                // Functional Threshold Power, must be > 0

Output:
  tss: f64                // Training Stress Score, typically 0-500
                          // No upper bound (extreme efforts can exceed 500)

Precision: IEEE 754 double precision (f64)
Tolerance: ±0.1 for validation due to floating point arithmetic
```

**validation examples**:

Example 1: Easy endurance ride
```
Input:
  avg_power = 180 W
  duration_hours = 2.0 h
  ftp = 300.0 W

Calculation:
  1. IF = 180.0 / 300.0 = 0.6
  2. IF² = 0.6² = 0.36
  3. TSS = 2.0 × 0.36 × 100 = 72.0

Expected API result: tss = 72.0
Interpretation: Low training stress (< 150)
```

Example 2: Threshold workout
```
Input:
  avg_power = 250 W
  duration_hours = 2.0 h
  ftp = 300.0 W

Calculation:
  1. IF = 250.0 / 300.0 = 0.8333...
  2. IF² = 0.8333² = 0.6944...
  3. TSS = 2.0 × 0.6944 × 100 = 138.89

Expected API result: tss = 138.9 (rounded to 1 decimal)
Interpretation: Moderate training stress (150-300 range)
```

Example 3: High-intensity interval session
```
Input:
  avg_power = 320 W
  duration_hours = 1.5 h
  ftp = 300.0 W

Calculation:
  1. IF = 320.0 / 300.0 = 1.0667
  2. IF² = 1.0667² = 1.1378
  3. TSS = 1.5 × 1.1378 × 100 = 170.67

Expected API result: tss = 170.7 (rounded to 1 decimal, though code rounds to nearest integer = 171.0)
Interpretation: Moderate-high training stress
```

**API response format**:

```json
{
  "activity_id": "12345678",
  "tss": 139.0,
  "method": "power",
  "inputs": {
    "avg_power": 250,
    "duration_hours": 2.0,
    "ftp": 300.0
  },
  "intensity_factor": 0.833,
  "interpretation": "moderate"
}
```

**common validation issues**:

1. **Mismatch in duration calculation**
   - Issue: Manual calculation uses elapsed_time, API uses moving_time
   - Solution: API uses `moving_time` (excludes stops). Verify which time you're comparing
   - Example: 2h ride with 10min stop = 1.83h moving_time

2. **FTP value discrepancy**
   - Issue: User's FTP changed but old value cached
   - Solution: Check user profile endpoint for current FTP value used in calculation
   - Validation: Ensure same FTP value in both calculations

3. **Average power vs normalized power expectation**
   - Issue: Expecting NP-based TSS but API uses average power
   - Pierre uses **average power**, not normalized power (NP)
   - For steady efforts: avg_power ≈ NP, minimal difference
   - For variable efforts: NP typically 3-10% higher than avg_power
   - Example: intervals averaging 200W may have NP=210W → TSS difference ~10%
   - Solution: Use average power in your validation calculations

4. **Floating point precision and rounding**
   - Issue: Manual calculation shows 138.888... But API returns 139.0
   - Solution: API rounds TSS to nearest integer using `.round()`
   - Tolerance: Accept ±1.0 difference as valid due to rounding

5. **Missing power data**
   - Issue: API returns error or falls back to hrTSS
   - Solution: Check activity has valid power stream data
   - Fallback: If no power data, API uses heart rate method (hrTSS)

### Heart Rate-based TSS (hrTSS)

**formula**:

```
hrTSS = duration_hours × (HR_avg / HR_threshold)² × 100
```

Where:
- `HR_avg` = average heart rate during activity (bpm)
- `HR_threshold` = lactate threshold heart rate (bpm)
- `duration_hours` = activity duration (hours)

**rust implementation**:

```rust
pub fn calculate_tss_hr(
    avg_hr: u32,
    duration_hours: f64,
    lthr: u32,
) -> f64 {
    let hr_ratio = (avg_hr as f64) / (lthr as f64);
    duration_hours * hr_ratio.powi(2) * 100.0
}
```

**input/output specification**:

```
Inputs:
  avg_hr: u32             // Average heart rate (bpm), must be > 0
  duration_hours: f64     // Activity duration, must be > 0
  lthr: u32               // Lactate Threshold HR (bpm), must be > 0

Output:
  hrTSS: f64              // Heart Rate Training Stress Score
                          // Typically 0-500, no upper bound

Precision: IEEE 754 double precision (f64)
Tolerance: ±0.1 for validation
```

**validation examples**:

Example 1: Easy run
```
Input:
  avg_hr = 135 bpm
  duration_hours = 1.0 h
  lthr = 165 bpm

Calculation:
  1. HR ratio = 135 / 165 = 0.8182
  2. HR ratio² = 0.8182² = 0.6694
  3. hrTSS = 1.0 × 0.6694 × 100 = 66.9

Expected API result: hrTSS = 66.9
Interpretation: Low training stress
```

Example 2: Tempo run
```
Input:
  avg_hr = 155 bpm
  duration_hours = 1.5 h
  lthr = 165 bpm

Calculation:
  1. HR ratio = 155 / 165 = 0.9394
  2. HR ratio² = 0.9394² = 0.8825
  3. hrTSS = 1.5 × 0.8825 × 100 = 132.4

Expected API result: hrTSS = 132.4
Interpretation: Moderate training stress
```

**API response format**:

```json
{
  "activity_id": "87654321",
  "tss": 66.9,
  "method": "heart_rate",
  "inputs": {
    "average_hr": 135,
    "duration_hours": 1.0,
    "lthr": 165
  },
  "hr_ratio": 0.818,
  "interpretation": "low"
}
```

**common validation issues**:

1. **LTHR value uncertainty**
   - Issue: User hasn't set or tested LTHR
   - Solution: API may estimate LTHR as ~88% of max_hr if not provided
   - Validation: Confirm LTHR value used via user profile endpoint

2. **Average HR calculation method**
   - Issue: Different averaging methods (time-weighted vs sample-weighted)
   - Solution: API uses time-weighted average from HR stream
   - Example: 30min @ 140bpm + 30min @ 160bpm = 150bpm average (not simple mean)

3. **HR drift**
   - Issue: Long efforts show cardiac drift (HR rises despite steady effort)
   - Solution: This is physiologically accurate - hrTSS will be higher than power-based TSS
   - Note: Not an error; reflects cardiovascular stress

4. **Comparison with power TSS**
   - Issue: hrTSS ≠ power TSS for same activity
   - Solution: Expected - HR responds to environmental factors (heat, fatigue)
   - Typical: hrTSS 5-15% higher than power TSS in hot conditions

**interpretation**:
- TSS < 150: low training stress
- 150 ≤ TSS < 300: moderate training stress
- 300 ≤ TSS < 450: high training stress
- TSS ≥ 450: very high training stress

**reference**: Coggan, A. (2003). Training Stress Score. *TrainingPeaks*.

---

## Normalized Power (NP)

Accounts for variability in cycling efforts using coggan's algorithm:

**important note**: NP calculation is available via the `calculate_normalized_power()` method, but **TSS uses average power** (not NP) in the current implementation. See TSS section for details.

**algorithm**:

1. Raise each instantaneous power to 4th power:
   ```
   Qᵢ = Pᵢ⁴
   ```

2. Calculate 30-second rolling average of power⁴ values:
   ```
   P̄⁴ₖ = (1/30) × Σⱼ₌₀²⁹ Qₖ₊ⱼ
   ```

3. Average all 30-second windows and take 4th root:
   ```
   NP = ⁴√((1/n) × Σₖ₌₁ⁿ P̄⁴ₖ)
   ```

Where:
- `Pᵢ` = instantaneous power at second i (watts)
- `Qᵢ` = power raised to 4th power (watts⁴)
- `P̄⁴ₖ` = 30-second rolling average of power⁴ values
- `n` = number of 30-second windows

**key distinction**: This raises power to 4th FIRST, then calculates rolling averages. This is NOT the same as averaging power first then raising to 4th.

**fallback** (if data < 30 seconds):

```
NP = average power (simple mean)
```

**rust implementation**:

```rust
// src/intelligence/metrics.rs
pub fn calculate_normalized_power(&self, power_data: &[u32]) -> Option<f64> {
    if power_data.len() < 30 {
        return None; // Need at least 30 seconds of data
    }

    // Convert to f64 for calculations
    let power_f64: Vec<f64> = power_data.iter().map(|&p| f64::from(p)).collect();

    // Calculate 30-second rolling averages of power^4
    let mut rolling_avg_power4 = Vec::new();
    for i in 29..power_f64.len() {
        let window = &power_f64[(i - 29)..=i];
        // Step 1 & 2: raise to 4th power, then average within window
        let avg_power4: f64 = window.iter().map(|&p| p.powi(4)).sum::<f64>() / 30.0;
        rolling_avg_power4.push(avg_power4);
    }

    if rolling_avg_power4.is_empty() {
        return None;
    }

    // Step 3: average all windows, then take 4th root
    let mean_power4 = rolling_avg_power4.iter().sum::<f64>()
        / f64::from(u32::try_from(rolling_avg_power4.len()).unwrap_or(u32::MAX));
    Some(mean_power4.powf(0.25))
}
```

**physiological basis**: 4th power weighting matches metabolic cost of variable efforts. Alternating 200W/150W has higher physiological cost than steady 175W. The 4th power emphasizes high-intensity bursts.

---

## Chronic Training Load (CTL) And Acute Training Load (ATL)

CTL ("fitness") and ATL ("fatigue") track training stress using exponential moving averages.

### Mathematical Formulation

**exponential moving average (EMA)**:

```
α = 2 / (N + 1)

EMAₜ = α × TSSₜ + (1 − α) × EMAₜ₋₁
```

Where:
- `N` = window size (days)
- `TSSₜ` = training stress score on day t
- `EMAₜ` = exponential moving average on day t
- `α` = smoothing factor ∈ (0, 1)

**chronic training load (CTL)**:

```
CTL = EMA₄₂(TSS_daily)
```

42-day exponential moving average of daily TSS, representing long-term fitness

**acute training load (ATL)**:

```
ATL = EMA₇(TSS_daily)
```

7-day exponential moving average of daily TSS, representing short-term fatigue

**training stress balance (TSB)**:

```
TSB = CTL − ATL
```

Difference between fitness and fatigue, representing current form

**daily TSS aggregation** (multiple activities per day):

```
TSS_daily = Σᵢ₌₁ⁿ TSSᵢ
```

Where `n` = number of activities on a given day

**gap handling** (missing training days):

```
For days with no activities: TSSₜ = 0

This causes exponential decay: EMAₜ = (1 − α) × EMAₜ₋₁
```

**rust implementation**:

```rust
// src/intelligence/training_load.rs
const CTL_WINDOW_DAYS: i64 = 42; // 6 weeks
const ATL_WINDOW_DAYS: i64 = 7;  // 1 week

pub fn calculate_training_load(
    activities: &[Activity],
    ftp: Option<f64>,
    lthr: Option<f64>,
    max_hr: Option<f64>,
    resting_hr: Option<f64>,
    weight_kg: Option<f64>,
) -> Result<TrainingLoad> {
    // Handle empty activities
    if activities.is_empty() {
        return Ok(TrainingLoad {
            ctl: 0.0,
            atl: 0.0,
            tsb: 0.0,
            tss_history: Vec::new(),
        });
    }

    // Calculate TSS for each activity
    let mut tss_data: Vec<TssDataPoint> = Vec::new();
    for activity in activities {
        if let Ok(tss) = calculate_tss(activity, ftp, lthr, max_hr, resting_hr, weight_kg) {
            tss_data.push(TssDataPoint {
                date: activity.start_date,
                tss,
            });
        }
    }

    // Handle no valid TSS calculations
    if tss_data.is_empty() {
        return Ok(TrainingLoad {
            ctl: 0.0,
            atl: 0.0,
            tsb: 0.0,
            tss_history: Vec::new(),
        });
    }

    let ctl = calculate_ema(&tss_data, CTL_WINDOW_DAYS);
    let atl = calculate_ema(&tss_data, ATL_WINDOW_DAYS);
    let tsb = ctl - atl;

    Ok(TrainingLoad { ctl, atl, tsb, tss_history: tss_data })
}

fn calculate_ema(tss_data: &[TssDataPoint], window_days: i64) -> f64 {
    if tss_data.is_empty() {
        return 0.0;
    }

    let alpha = 2.0 / (window_days as f64 + 1.0);

    // Create daily TSS map (handles multiple activities per day)
    let mut tss_map = std::collections::HashMap::new();
    for point in tss_data {
        let date_key = point.date.date_naive();
        *tss_map.entry(date_key).or_insert(0.0) += point.tss;
    }

    // Calculate EMA day by day, filling gaps with 0.0
    let first_date = tss_data[0].date;
    let last_date = tss_data[tss_data.len() - 1].date;
    let days_span = (last_date - first_date).num_days();

    let mut ema = 0.0;
    for day_offset in 0..=days_span {
        let current_date = first_date + Duration::days(day_offset);
        let date_key = current_date.date_naive();
        let daily_tss = tss_map.get(&date_key).copied().unwrap_or(0.0); // Gap = 0

        ema = daily_tss.mul_add(alpha, ema * (1.0 - alpha));
    }

    ema
}
```

**input/output specification**:

```
Inputs:
  activities: &[Activity]  // Array of activities with TSS values
  ftp: Option<f64>         // For power-based TSS calculation
  lthr: Option<f64>        // For HR-based TSS calculation
  max_hr: Option<f64>      // For HR zone estimation
  resting_hr: Option<f64>  // For HR zone estimation
  weight_kg: Option<f64>   // For pace-based TSS estimation

Output:
  TrainingLoad {
    ctl: f64,              // Chronic Training Load (0-200 typical)
    atl: f64,              // Acute Training Load (0-300 typical)
    tsb: f64,              // Training Stress Balance (-50 to +50 typical)
    tss_history: Vec<TssDataPoint>  // Daily TSS values used
  }

Precision: IEEE 754 double precision (f64)
Tolerance: ±0.5 for CTL/ATL, ±1.0 for TSB due to cumulative rounding
```

**validation examples**:

Example 1: Simple 7-day training block (no gaps)
```
Input activities (daily TSS):
  Day 1: 100
  Day 2: 80
  Day 3: 120
  Day 4: 60  (recovery)
  Day 5: 110
  Day 6: 90
  Day 7: 140

Calculation (simplified for Day 7):
  α_ctl = 2 / (42 + 1) = 0.0465
  α_atl = 2 / (7 + 1) = 0.25

  ATL (7-day EMA, final value):
    Day 1: 100 × 0.25 = 25.0
    Day 2: 80 × 0.25 + 25.0 × 0.75 = 38.75
    Day 3: 120 × 0.25 + 38.75 × 0.75 = 59.06
    Day 4: 60 × 0.25 + 59.06 × 0.75 = 59.30
    Day 5: 110 × 0.25 + 59.30 × 0.75 = 71.98
    Day 6: 90 × 0.25 + 71.98 × 0.75 = 76.49
    Day 7: 140 × 0.25 + 76.49 × 0.75 = 92.37

  CTL (42-day EMA, grows slowly):
    Assuming starting from 0, after 7 days ≈ 32.5

  TSB = CTL - ATL = 32.5 - 92.37 = -59.87

Expected API result:
  ctl ≈ 32.5
  atl ≈ 92.4
  tsb ≈ -59.9
Interpretation: Heavy training week, significant fatigue
```

Example 2: Training with gap (rest week)
```
Input activities:
  Days 1-7: Daily TSS = 100 (week 1)
  Days 8-14: No activities (rest week)
  Day 15: TSS = 120 (return to training)

At Day 14 (after rest week):
  α_atl = 0.25

  Day 7 ATL: ~75.0
  Day 8: 0 × 0.25 + 75.0 × 0.75 = 56.25
  Day 9: 0 × 0.25 + 56.25 × 0.75 = 42.19
  Day 10: 0 × 0.25 + 42.19 × 0.75 = 31.64
  Day 11: 0 × 0.25 + 31.64 × 0.75 = 23.73
  Day 12: 0 × 0.25 + 23.73 × 0.75 = 17.80
  Day 13: 0 × 0.25 + 17.80 × 0.75 = 13.35
  Day 14: 0 × 0.25 + 13.35 × 0.75 = 10.01

Expected API result at Day 14:
  atl ≈ 10.0 (decayed from ~75)
  ctl ≈ 35.0 (decays slower due to 42-day window)
  tsb ≈ +25.0 (fresh, ready for hard training)

Note: Gap = zero TSS causes exponential decay
```

Example 3: Multiple activities per day
```
Input activities (same day):
  Morning: TSS = 80 (easy ride)
  Evening: TSS = 60 (strength training converted to TSS)

Aggregation:
  Daily TSS = 80 + 60 = 140

EMA calculation uses 140 for that day's TSS value

Expected API result:
  tss_history[date] = 140.0 (single aggregated value)
  ATL/CTL calculations use 140 for that day
```

**API response format**:

```json
{
  "ctl": 87.5,
  "atl": 92.3,
  "tsb": -4.8,
  "tss_history": [
    {"date": "2025-10-01", "tss": 100.0},
    {"date": "2025-10-02", "tss": 85.0},
    {"date": "2025-10-03", "tss": 120.0}
  ],
  "status": "productive",
  "fitness_trend": "building",
  "last_updated": "2025-10-03T18:30:00Z"
}
```

**common validation issues**:

1. **Date range discrepancy**
   - Issue: Manual calculation uses different time window
   - Solution: API uses all activities within the date range, verify your date filter
   - Example: "Last 42 days" starts from current date midnight UTC

2. **Gap handling differences**
   - Issue: Manual calculation skips gaps, API fills with zeros
   - Solution: API fills missing days with TSS=0, causing realistic decay
   - Validation: Check tss_history - should include interpolated zeros
   - Example: 5-day training gap → CTL decays ~22%, ATL decays ~75%

3. **Multiple activities aggregation**
   - Issue: Not summing same-day activities
   - Solution: API sums all TSS values for a single day
   - Example: 2 rides on Monday: 80 TSS + 60 TSS = 140 TSS for that day

4. **Starting value (cold start)**
   - Issue: EMA starting value assumption
   - Solution: API starts EMA at 0.0 for new users
   - Note: CTL takes ~6 weeks to stabilize, ATL takes ~2 weeks
   - Impact: Early values less reliable (first 2-6 weeks of training)

5. **TSS calculation failures**
   - Issue: Some activities excluded due to missing data
   - Solution: API skips activities without power/HR data
   - Validation: Check tss_history.length vs activities count
   - Example: 10 activities but only 7 in tss_history → 3 failed TSS calculation

6. **Floating point accumulation**
   - Issue: Small differences accumulate over many days
   - Solution: Accept ±0.5 for CTL/ATL, ±1.0 for TSB
   - Cause: IEEE 754 rounding across 42+ days of calculations

7. **Timezone effects**
   - Issue: Activity recorded at 11:59 PM vs 12:01 AM different days
   - Solution: API uses activity start_date in UTC
   - Validation: Check which day activity is assigned to in tss_history

8. **CTL/ATL ratio interpretation**
   - Issue: TSB seems wrong despite correct CTL/ATL
   - Solution: TSB = CTL - ATL, not a ratio
   - Example: CTL=100, ATL=110 → TSB=-10 (fatigued, not "10% fatigued")

**validation workflow**:

Step 1: Verify TSS calculations
```
For each activity in tss_history:
  - Recalculate TSS using activity data
  - Confirm TSS value matches (±0.1)
```

Step 2: Verify daily aggregation
```
Group activities by date:
  - Sum same-day TSS values
  - Confirm daily_tss matches aggregation
```

Step 3: Verify EMA calculation
```
Starting from EMA = 0:
  For each day from first to last:
    - Calculate α = 2 / (window + 1)
    - EMA_new = daily_tss × α + EMA_old × (1 - α)
    - Confirm EMA_new matches API value (±0.5)
```

Step 4: Verify TSB
```
TSB = CTL - ATL
Confirm: API_tsb ≈ API_ctl - API_atl (±0.1)
```

**edge case handling**:
- **zero activities**: returns CTL=0, ATL=0, TSB=0
- **training gaps**: zero-fills missing days (realistic fitness decay)
- **multiple activities per day**: sums TSS values
- **failed TSS calculations**: skips activities, continues with valid data

**reference**: Banister, E.W. (1991). Modeling elite athletic performance. Human Kinetics.

---

## Training Stress Balance (TSB)

TSB indicates form/freshness using piecewise classification:

**training status classification**:

```
TrainingStatus(TSB) = Overreaching,  if TSB < −10
                    = Productive,    if −10 ≤ TSB < 0
                    = Fresh,         if 0 ≤ TSB ≤ 10
                    = Detraining,    if TSB > 10
```

**rust implementation**:

```rust
pub fn interpret_tsb(tsb: f64) -> TrainingStatus {
    match tsb {
        t if t < -10.0 => TrainingStatus::Overreaching,
        t if t < 0.0   => TrainingStatus::Productive,
        t if t <= 10.0 => TrainingStatus::Fresh,
        _              => TrainingStatus::Detraining,
    }
}
```

**interpretation**:
- **TSB < −10**: overreaching (high fatigue) - recovery needed
- **−10 ≤ TSB < 0**: productive training - building fitness
- **0 ≤ TSB ≤ 10**: fresh - ready for hard efforts
- **TSB > 10**: risk of detraining

**reference**: Banister, E.W., Calvert, T.W., Savage, M.V., & Bach, T. (1975). A systems model of training. *Australian Journal of Sports Medicine*, 7(3), 57-61.

---

## Overtraining Risk Detection

**three-factor risk assessment**:

```
Risk Factor 1 (Acute Load Spike):
  Triggered when: (CTL > 0) ∧ (ATL > 1.3 × CTL)

Risk Factor 2 (Very High Acute Load):
  Triggered when: ATL > 150

Risk Factor 3 (Deep Fatigue):
  Triggered when: TSB < −10
```

**risk level classification**:

```
RiskLevel = Low,       if |risk_factors| = 0
          = Moderate,  if |risk_factors| = 1
          = High,      if |risk_factors| ≥ 2
```

Where `|risk_factors|` = count of triggered risk factors

**rust implementation**:

```rust
// src/intelligence/training_load.rs
pub fn check_overtraining_risk(training_load: &TrainingLoad) -> OvertrainingRisk {
    let mut risk_factors = Vec::new();

    // 1. Acute load spike
    if training_load.ctl > 0.0 && training_load.atl > training_load.ctl * 1.3 {
        risk_factors.push(
            "Acute load spike >30% above chronic load".to_string()
        );
    }

    // 2. Very high acute load
    if training_load.atl > 150.0 {
        risk_factors.push(
            "Very high acute load (>150 TSS/day)".to_string()
        );
    }

    // 3. Deep fatigue
    if training_load.tsb < -10.0 {
        risk_factors.push(
            "Deep fatigue (TSB < -10)".to_string()
        );
    }

    let risk_level = match risk_factors.len() {
        0 => RiskLevel::Low,
        1 => RiskLevel::Moderate,
        _ => RiskLevel::High,
    };

    OvertrainingRisk { risk_level, risk_factors }
}
```

**physiological interpretation**:
- **Acute load spike**: fatigue (ATL) exceeds fitness (CTL) by >30%, indicating sudden increase
- **Very high acute load**: average daily TSS >150 in past week, exceeding sustainable threshold
- **Deep fatigue**: negative TSB <−10, indicating accumulated fatigue without recovery

**reference**: Halson, S.L. (2014). Monitoring training load to understand fatigue. *Sports Medicine*, 44(Suppl 2), 139-147.

---

## Statistical Trend Analysis

Pierre uses ordinary least squares linear regression for trend detection:

**linear regression formulation**:

Given n data points `(xᵢ, yᵢ)`, fit line: `ŷ = β₀ + β₁x`

**slope calculation**:

```
β₁ = (Σᵢ₌₁ⁿ xᵢyᵢ − n × x̄ × ȳ) / (Σᵢ₌₁ⁿ xᵢ² − n × x̄²)
```

**intercept calculation**:

```
β₀ = ȳ − β₁ × x̄
```

Where:
- `x̄ = (1/n) × Σᵢ₌₁ⁿ xᵢ` (mean of x values)
- `ȳ = (1/n) × Σᵢ₌₁ⁿ yᵢ` (mean of y values)
- `n` = number of data points

**coefficient of determination (R²)**:

```
R² = 1 − (SS_res / SS_tot)
```

Where:
- `SS_tot = Σᵢ₌₁ⁿ (yᵢ − ȳ)²` (total sum of squares)
- `SS_res = Σᵢ₌₁ⁿ (yᵢ − ŷᵢ)²` (residual sum of squares)
- `ŷᵢ = β₀ + β₁xᵢ` (predicted value)

**correlation coefficient**:

```
r = sign(β₁) × √R²
```

**rust implementation**:

```rust
// src/intelligence/statistical_analysis.rs
pub fn linear_regression(data_points: &[TrendDataPoint]) -> Result<RegressionResult> {
    let n = data_points.len() as f64;
    let x_values: Vec<f64> = (0..data_points.len()).map(|i| i as f64).collect();
    let y_values: Vec<f64> = data_points.iter().map(|p| p.value).collect();

    let sum_x = x_values.iter().sum::<f64>();
    let sum_y = y_values.iter().sum::<f64>();
    let sum_xx = x_values.iter().map(|x| x * x).sum::<f64>();
    let sum_xy = x_values.iter().zip(&y_values).map(|(x, y)| x * y).sum::<f64>();
    let sum_yy = y_values.iter().map(|y| y * y).sum::<f64>();

    let mean_x = sum_x / n;
    let mean_y = sum_y / n;

    // Calculate slope and intercept
    let numerator = sum_xy - n * mean_x * mean_y;
    let denominator = sum_xx - n * mean_x * mean_x;

    let slope = numerator / denominator;
    let intercept = mean_y - slope * mean_x;

    // Calculate R² (coefficient of determination)
    let ss_tot = sum_yy - n * mean_y * mean_y;
    let ss_res: f64 = y_values
        .iter()
        .zip(&x_values)
        .map(|(y, x)| {
            let predicted = slope * x + intercept;
            (y - predicted).powi(2)
        })
        .sum();

    let r_squared = 1.0 - (ss_res / ss_tot);
    let correlation = r_squared.sqrt() * slope.signum();

    Ok(RegressionResult {
        slope,
        intercept,
        r_squared,
        correlation,
    })
}
```

**R² interpretation**:
- 0.0 ≤ R² < 0.3: weak relationship
- 0.3 ≤ R² < 0.5: moderate relationship
- 0.5 ≤ R² < 0.7: strong relationship
- 0.7 ≤ R² ≤ 1.0: very strong relationship

**reference**: Draper, N.R. & Smith, H. (1998). *Applied Regression Analysis* (3rd ed.). Wiley.

---

## Performance Prediction: VDOT

VDOT is jack daniels' VO2max adjusted for running economy:

### VDOT Calculation From Race Performance

**step 1: convert to velocity** (meters per minute):

```
v = (d / t) × 60
```

Where:
- `d` = distance (meters)
- `t` = time (seconds)
- `v ∈ [100, 500]` m/min (validated range)

**step 2: calculate VO2 consumption** (Jack Daniels' formula):

```
VO₂ = −4.60 + 0.182258v + 0.000104v²
```

**step 3: adjust for race duration**:

```
percent_max(t) = 0.97,   if t_min < 5      (very short, oxygen deficit)
               = 0.99,   if 5 ≤ t_min < 15  (5K range)
               = 1.00,   if 15 ≤ t_min < 30 (10K-15K, optimal)
               = 0.98,   if 30 ≤ t_min < 90 (half marathon)
               = 0.95,   if t_min ≥ 90      (marathon+, fatigue)
```

Where `t_min = t / 60` (time in minutes)

**step 4: calculate VDOT**:

```
VDOT = VO₂ / percent_max(t)
```

**rust implementation**:

```rust
// src/intelligence/performance_prediction.rs
pub fn calculate_vdot(distance_m: f64, time_s: f64) -> Result<f64> {
    // Convert to velocity (m/min)
    let velocity = (distance_m / time_s) * 60.0;

    // Validate velocity range
    if !(100.0..=500.0).contains(&velocity) {
        return Err(AppError::invalid_input(
            format!("Velocity {velocity:.1} m/min outside valid range (100-500)")
        ));
    }

    // Jack Daniels' VO2 formula
    // VO2 = -4.60 + 0.182258×v + 0.000104×v²
    let vo2 = (0.000104 * velocity).mul_add(
        velocity,
        0.182258f64.mul_add(velocity, -4.60)
    );

    // Adjust for race duration
    let percent_max = calculate_percent_max_adjustment(time_s);

    // VDOT = VO2 / percent_used
    Ok(vo2 / percent_max)
}

fn calculate_percent_max_adjustment(time_s: f64) -> f64 {
    let time_minutes = time_s / 60.0;

    match time_minutes {
        t if t < 5.0  => 0.97, // Very short - oxygen deficit
        t if t < 15.0 => 0.99, // 5K range
        t if t < 30.0 => 1.00, // 10K-15K range - optimal
        t if t < 90.0 => 0.98, // Half marathon range
        _             => 0.95, // Marathon+ - fatigue accumulation
    }
}
```

**VDOT ranges**:
- 30-40: beginner
- 40-50: recreational
- 50-60: competitive amateur
- 60-70: sub-elite
- 70-85: elite
- VDOT ∈ [30, 85] (typical range)

**input/output specification**:

Inputs:
  Distance_m: f64         // Race distance in meters, must be > 0
  Time_s: f64             // Race time in seconds, must be > 0

Output:
  Vdot: f64               // VDOT value, typically 30-85

Derived:
  Velocity: f64           // Calculated velocity (m/min), must be in [100, 500]
  Vo2: f64                // VO2 consumption (ml/kg/min)
  Percent_max: f64        // Race duration adjustment factor [0.95-1.00]

Precision: IEEE 754 double precision (f64)
Tolerance: ±0.5 VDOT units due to floating point arithmetic and physiological variance

**validation examples**:

Example 1: 5K race (recreational runner)
  Input:
    distance_m = 5000.0
    time_s = 1200.0  (20:00)

  Step-by-step calculation:
    1. velocity = (5000.0 / 1200.0) × 60 = 250.0 m/min
    2. vo2 = -4.60 + (0.182258 × 250.0) + (0.000104 × 250.0²)
         = -4.60 + 45.5645 + 6.5
         = 47.4645 ml/kg/min
    3. time_minutes = 1200.0 / 60 = 20.0
       percent_max = 0.99  (5K range: 15 ≤ t < 30)
    4. VDOT = 47.4645 / 0.99 = 47.9

  Expected Output: VDOT = 47.9

Example 2: 10K race (competitive amateur)
  Input:
    distance_m = 10000.0
    time_s = 2250.0  (37:30)

  Step-by-step calculation:
    1. velocity = (10000.0 / 2250.0) × 60 = 266.67 m/min
    2. vo2 = -4.60 + (0.182258 × 266.67) + (0.000104 × 266.67²)
         = -4.60 + 48.6021 + 7.3956
         = 51.3977 ml/kg/min
    3. time_minutes = 2250.0 / 60 = 37.5
       percent_max = 0.98  (half marathon range: 30 ≤ t < 90)
    4. VDOT = 51.3977 / 0.98 = 52.4

  Expected Output: VDOT = 52.4

Example 3: Marathon race (sub-elite)
  Input:
    distance_m = 42195.0
    time_s = 10800.0  (3:00:00)

  Step-by-step calculation:
    1. velocity = (42195.0 / 10800.0) × 60 = 234.42 m/min
    2. vo2 = -4.60 + (0.182258 × 234.42) + (0.000104 × 234.42²)
         = -4.60 + 42.7225 + 5.7142
         = 43.8367 ml/kg/min
    3. time_minutes = 10800.0 / 60 = 180.0
       percent_max = 0.95  (marathon range: t ≥ 90)
    4. VDOT = 43.8367 / 0.95 = 46.1

  Expected Output: VDOT = 46.1

  Note: This seems low for 3-hour marathon. In reality, sub-elite marathoners
  Typically have VDOT 60-70. This illustrates the importance of race-specific
  Calibration and proper pacing (marathon fatigue factor = 0.95 significantly
  Impacts VDOT calculation).

Example 4: Half marathon race (recreational competitive)
  Input:
    distance_m = 21097.5
    time_s = 5400.0  (1:30:00)

  Step-by-step calculation:
    1. velocity = (21097.5 / 5400.0) × 60 = 234.42 m/min
    2. vo2 = -4.60 + (0.182258 × 234.42) + (0.000104 × 234.42²)
         = -4.60 + 42.7225 + 5.7142
         = 43.8367 ml/kg/min
    3. time_minutes = 5400.0 / 60 = 90.0
       percent_max = 0.95  (marathon range: t ≥ 90)
       NOTE: Boundary condition - at exactly 90 minutes, uses 0.95
    4. VDOT = 43.8367 / 0.95 = 46.1

  Expected Output: VDOT = 46.1

**API response format**:

```json
{
  "activity_id": "12345678",
  "vdot": 52.4,
  "inputs": {
    "distance_m": 10000.0,
    "time_s": 2250.0,
    "pace_per_km": "3:45"
  },
  "calculated": {
    "velocity_m_per_min": 266.67,
    "vo2_ml_per_kg_min": 51.40,
    "percent_max_adjustment": 0.98,
    "time_minutes": 37.5
  },
  "interpretation": "competitive_amateur",
  "race_predictions": {
    "5K": "17:22",
    "10K": "36:15",
    "half_marathon": "1:20:45",
    "marathon": "2:50:30"
  }
}
```

**common validation issues**:

1. **velocity out of range (100-500 m/min)**:
   - Cause: extremely slow pace (<12 km/h) or unrealistic fast pace (>30 km/h)
   - Example: 5K in 50 minutes → velocity = 100 m/min (walking pace)
   - Example: 5K in 10 minutes → velocity = 500 m/min (world record ~350 m/min)
   - Solution: validate input data quality; reject activities with unrealistic paces

2. **percent_max boundary conditions**:
   - At t = 5, 15, 30, 90 minutes, percent_max changes discretely
   - Example: 10K in 29:59 uses 1.00 (10K range), but 30:01 uses 0.98 (half range)
   - This creates discontinuous VDOT jumps at boundaries
   - Solution: document boundary behavior; users should expect ±2 VDOT variance near boundaries

3. **comparison with Jack Daniels' tables**:
   - Pierre uses mathematical formula; Jack Daniels' tables use empirical adjustments
   - Expected difference: 0.2-5.5% (see verification section)
   - Example: VDOT 50 marathon → pierre predicts 3:12:38, table shows 3:08:00 (2.5% diff)
   - Solution: both are valid; pierre is more consistent across distances

4. **VDOT from different race distances doesn't match**:
   - Cause: runner's strengths vary by distance (speed vs endurance)
   - Example: VDOT 55 from 5K but VDOT 50 from marathon
   - Physiological: runner may have strong VO2max but weaker lactate threshold
   - Solution: use most recent race at target distance; VDOT varies by race type

5. **VDOT too low for known fitness level**:
   - Cause: race conducted in poor conditions (heat, hills, wind)
   - Cause: insufficient taper or poor pacing strategy
   - Cause: race was not maximal effort (training run logged as race)
   - Solution: only use races with maximal effort in good conditions

6. **VDOT outside typical range [30, 85]**:
   - VDOT < 30: data quality issue or walking activity
   - VDOT > 85: elite/world-class performance (verify data accuracy)
   - Solution: pierre rejects VDOT outside [30, 85] as invalid input

7. **predicted race times don't match actual performance**:
   - Cause: VDOT assumes proper training at target distance
   - Example: VDOT 60 from 5K predicts 2:46 marathon, but runner lacks endurance
   - Solution: VDOT is running economy, not prediction; requires distance-specific training

8. **floating point precision differences**:
   - Different platforms may produce slightly different VDOT values
   - Example: velocity = 266.666666... (repeating) may round differently
   - Tolerance: accept ±0.5 VDOT units as equivalent
   - Solution: compare VDOT values with tolerance, not exact equality

**validation workflow for users**:

1. **verify input data quality**:
   ```bash
   # Check velocity is in valid range
   Velocity = (distance_m / time_s) × 60
   Assert 100.0 ≤ velocity ≤ 500.0
   ```

2. **calculate intermediate values**:
   ```bash
   # Verify VO2 calculation
   Vo2 = -4.60 + (0.182258 × velocity) + (0.000104 × velocity²)

   # Verify percent_max adjustment
   Time_minutes = time_s / 60
   # Check against percent_max ranges (see formula)
   ```

3. **calculate VDOT**:
   ```bash
   Vdot = vo2 / percent_max
   Assert 30.0 ≤ vdot ≤ 85.0
   ```

4. **compare with reference**:
   - Compare calculated VDOT with Jack Daniels' published tables
   - Accept 0-6% difference as normal
   - If difference >6%, investigate input data quality

### Race Time Prediction From VDOT

**step 1: calculate velocity at VO2max** (inverse of Jack Daniels' formula):

Solve quadratic equation:
```
0.000104v² + 0.182258v − (VDOT + 4.60) = 0
```

Using quadratic formula:
```
v = (−b + √(b² − 4ac)) / (2a)
```

Where:
- `a = 0.000104`
- `b = 0.182258`
- `c = −(VDOT + 4.60)`

**step 2: adjust velocity for race distance**:

```
v_race(d, v_max) = 0.98 × v_max,                           if d ≤ 5,000 m
                 = 0.94 × v_max,                           if 5,000 < d ≤ 10,000 m
                 = 0.91 × v_max,                           if 10,000 < d ≤ 15,000 m
                 = 0.88 × v_max,                           if 15,000 < d ≤ 21,097.5 m
                 = 0.84 × v_max,                           if 21,097.5 < d ≤ 42,195 m
                 = max(0.70, 0.84 − 0.02(r − 1)) × v_max,  if d > 42,195 m
```

Where `r = d / 42,195` (marathon ratio for ultra distances)

**step 3: calculate predicted time**:

```
t_predicted = (d / v_race) × 60
```

Where:
- `d` = target distance (meters)
- `v_race` = race velocity (meters/minute)
- `t_predicted` = predicted time (seconds)

**rust implementation**:

```rust
pub fn predict_time_vdot(vdot: f64, target_distance_m: f64) -> Result<f64> {
    // Validate VDOT range
    if !(30.0..=85.0).contains(&vdot) {
        return Err(AppError::invalid_input(
            format!("VDOT {vdot:.1} outside typical range (30-85)")
        ));
    }

    // Calculate velocity at VO2max (reverse of VDOT formula)
    // vo2 = -4.60 + 0.182258 × v + 0.000104 × v²
    // Solve quadratic: 0.000104v² + 0.182258v - (vo2 + 4.60) = 0

    let a = 0.000104;
    let b = 0.182258;
    let c = -(vdot + 4.60);

    let discriminant = b.mul_add(b, -(4.0 * a * c));
    let velocity_max = (-b + discriminant.sqrt()) / (2.0 * a);

    // Adjust for race distance
    let race_velocity = calculate_race_velocity(velocity_max, target_distance_m);

    // Calculate time
    Ok((target_distance_m / race_velocity) * 60.0)
}

fn calculate_race_velocity(velocity_max: f64, distance_m: f64) -> f64 {
    let percent_max = if distance_m <= 5_000.0 {
        0.98 // 5K: 98% of VO2max velocity
    } else if distance_m <= 10_000.0 {
        0.94 // 10K: 94%
    } else if distance_m <= 15_000.0 {
        0.91 // 15K: 91%
    } else if distance_m <= 21_097.5 {
        0.88 // Half: 88%
    } else if distance_m <= 42_195.0 {
        0.84 // Marathon: 84%
    } else {
        // Ultra: progressively lower
        let marathon_ratio = distance_m / 42_195.0;
        (marathon_ratio - 1.0).mul_add(-0.02, 0.84).max(0.70)
    };

    velocity_max * percent_max
}
```

**input/output specification for race time prediction**:

Inputs:
  Vdot: f64               // VDOT value, must be in [30, 85]
  Target_distance_m: f64  // Target race distance in meters, must be > 0

Output:
  Predicted_time_s: f64   // Predicted race time in seconds

Derived:
  Velocity_max: f64       // Velocity at VO2max (m/min) from quadratic formula
  Race_velocity: f64      // Adjusted velocity for race distance (m/min)
  Percent_max: f64        // Distance-based velocity adjustment [0.70-0.98]

Precision: IEEE 754 double precision (f64)
Tolerance: ±2% for race time predictions (±3 seconds per 5K, ±6 seconds per 10K, ±3 minutes per marathon)

**validation examples for race time prediction**:

Example 1: Predict 5K time from VDOT 50
  Input:
    vdot = 50.0
    target_distance_m = 5000.0

  Step-by-step calculation:
    1. Solve quadratic: 0.000104v² + 0.182258v - (50.0 + 4.60) = 0
       a = 0.000104, b = 0.182258, c = -54.60
       discriminant = 0.182258² - (4 × 0.000104 × -54.60)
                   = 0.033218 + 0.022718 = 0.055936
       velocity_max = (-0.182258 + √0.055936) / (2 × 0.000104)
                   = (-0.182258 + 0.23652) / 0.000208
                   = 260.78 m/min

    2. Adjust for 5K distance (≤ 5000m → 0.98 × velocity_max):
       race_velocity = 0.98 × 260.78 = 255.56 m/min

    3. Calculate predicted time:
       predicted_time_s = (5000.0 / 255.56) × 60 = 1174.3 seconds
                       = 19:34

  Expected Output: 19:34 (19 minutes 34 seconds)
  Jack Daniels Reference: 19:31 → 0.2% difference ✅

Example 2: Predict marathon time from VDOT 60
  Input:
    vdot = 60.0
    target_distance_m = 42195.0

  Step-by-step calculation:
    1. Solve quadratic: 0.000104v² + 0.182258v - (60.0 + 4.60) = 0
       c = -64.60
       discriminant = 0.033218 + 0.026870 = 0.060088
       velocity_max = (-0.182258 + 0.24513) / 0.000208
                   = 302.34 m/min

    2. Adjust for marathon distance (21097.5 < d ≤ 42195 → 0.84 × velocity_max):
       race_velocity = 0.84 × 302.34 = 253.97 m/min

    3. Calculate predicted time:
       predicted_time_s = (42195.0 / 253.97) × 60 = 9970 seconds
                       = 2:46:10

  Expected Output: 2:46:10 (2 hours 46 minutes 10 seconds)
  Jack Daniels Reference: 2:40:00 → 3.9% difference ✅

Example 3: Predict 10K time from VDOT 40
  Input:
    vdot = 40.0
    target_distance_m = 10000.0

  Step-by-step calculation:
    1. Solve quadratic: 0.000104v² + 0.182258v - (40.0 + 4.60) = 0
       c = -44.60
       discriminant = 0.033218 + 0.018550 = 0.051768
       velocity_max = (-0.182258 + 0.22752) / 0.000208
                   = 217.43 m/min

    2. Adjust for 10K distance (5000 < d ≤ 10000 → 0.94 × velocity_max):
       race_velocity = 0.94 × 217.43 = 204.38 m/min

    3. Calculate predicted time:
       predicted_time_s = (10000.0 / 204.38) × 60 = 2932 seconds
                       = 48:52

  Expected Output: 48:52 (48 minutes 52 seconds)
  Jack Daniels Reference: 51:42 → 5.5% difference ✅

**API response format for race predictions**:

```json
{
  "user_id": "user_12345",
  "vdot": 50.0,
  "calculation_date": "2025-01-15",
  "race_predictions": [
    {
      "distance": "5K",
      "distance_m": 5000.0,
      "predicted_time_s": 1174.3,
      "predicted_time_formatted": "19:34",
      "pace_per_km": "3:55",
      "race_velocity_m_per_min": 255.56
    },
    {
      "distance": "10K",
      "distance_m": 10000.0,
      "predicted_time_s": 2448.0,
      "predicted_time_formatted": "40:48",
      "pace_per_km": "4:05",
      "race_velocity_m_per_min": 244.90
    },
    {
      "distance": "Half Marathon",
      "distance_m": 21097.5,
      "predicted_time_s": 5516.0,
      "predicted_time_formatted": "1:31:56",
      "pace_per_km": "4:21",
      "race_velocity_m_per_min": 229.50
    },
    {
      "distance": "Marathon",
      "distance_m": 42195.0,
      "predicted_time_s": 11558.0,
      "predicted_time_formatted": "3:12:38",
      "pace_per_km": "4:35",
      "race_velocity_m_per_min": 218.85
    }
  ],
  "calculated": {
    "velocity_max_m_per_min": 260.78,
    "interpretation": "recreational_competitive"
  },
  "accuracy_note": "Predictions assume proper training, taper, and race conditions. Expected ±5% variance from actual performance."
}
```

**common validation issues for race time prediction**:

1. **quadratic formula numerical instability**:
   - At extreme VDOT values (near 30 or 85), discriminant may be small
   - Very small discriminant → numerical precision issues in sqrt()
   - Solution: validate VDOT is in [30, 85] before calculation

2. **velocity_max boundary at distance transitions**:
   - Percent_max changes discretely at 5K, 10K, 15K, half, marathon boundaries
   - Example: 5001m uses 0.94 (10K), but 4999m uses 0.98 (5K) → 4% velocity difference
   - Creates discontinuous predictions near distance boundaries
   - Solution: document boundary behavior; predictions are approximations

3. **ultra-distance predictions become conservative**:
   - Formula: 0.84 - 0.02 × (marathon_ratio - 1) for d > 42195m
   - Example: 50K → marathon_ratio = 1.18 → percent_max = 0.836
   - Example: 100K → marathon_ratio = 2.37 → percent_max = 0.813
   - Minimum floor: 0.70 (70% of VO2max velocity)
   - Solution: VDOT predictions for ultras (>42K) are less accurate; use with caution

4. **predicted times slower than personal bests**:
   - Cause: VDOT calculated from shorter distance (5K VDOT predicting marathon)
   - Cause: insufficient endurance training for longer distances
   - Example: VDOT 60 from 5K → predicts 2:46 marathon, but runner only has 10K training
   - Solution: VDOT assumes distance-specific training; predictions require proper preparation

5. **predicted times much faster than current fitness**:
   - Cause: VDOT calculated from recent breakthrough race or downhill course
   - Cause: VDOT input doesn't reflect current fitness (old value)
   - Solution: recalculate VDOT from recent representative race in similar conditions

6. **race predictions don't account for external factors**:
   - Weather: heat +5-10%, wind +2-5%, rain +1-3%
   - Course: hills +3-8%, trail +5-15% vs flat road
   - Altitude: +3-5% per 1000m elevation for non-acclimated runners
   - Solution: VDOT predictions are baseline; adjust for race conditions

7. **comparison with Jack Daniels' tables shows differences**:
   - Pierre: mathematical formula (consistent across all distances)
   - Jack Daniels: empirical adjustments from real runner data
   - Expected variance: 0.2-5.5% (see accuracy verification below)
   - Solution: both approaches are valid; pierre is more algorithmic

8. **floating point precision in quadratic formula**:
   - Discriminant calculation: b² - 4ac may lose precision for similar values
   - Square root operation introduces rounding
   - Velocity calculation: division by small value (2a = 0.000208) amplifies errors
   - Tolerance: accept ±1 second per 10 minutes of predicted time
   - Solution: use f64 precision throughout; compare with tolerance

**validation workflow for race time predictions**:

1. **validate VDOT input**:
   ```bash
   Assert 30.0 ≤ vdot ≤ 85.0
   ```

2. **solve quadratic for velocity_max**:
   ```bash
   A = 0.000104
   B = 0.182258
   C = -(vdot + 4.60)
   Discriminant = b² - 4ac
   Assert discriminant > 0
   Velocity_max = (-b + √discriminant) / (2a)
   ```

3. **calculate race velocity with distance adjustment**:
   ```bash
   # Check percent_max based on distance
   # 5K: 0.98, 10K: 0.94, 15K: 0.91, Half: 0.88, Marathon: 0.84, Ultra: see formula
   Race_velocity = percent_max × velocity_max
   ```

4. **calculate predicted time**:
   ```bash
   Predicted_time_s = (target_distance_m / race_velocity) × 60
   ```

5. **compare with Jack Daniels' reference**:
   - Use VDOT accuracy verification table below
   - Accept 0-6% difference as normal
   - If >6% difference, verify calculation steps

### VDOT Accuracy Verification ✅

Pierre's VDOT predictions have been verified against jack daniels' published tables:

```
VDOT 50 (recreational competitive):
  5K:        19:34 vs 19:31 reference → 0.2% difference ✅
  10K:       40:48 vs 40:31 reference → 0.7% difference ✅
  Half:    1:31:56 vs 1:30:00 reference → 2.2% difference ✅
  Marathon: 3:12:38 vs 3:08:00 reference → 2.5% difference ✅

VDOT 60 (sub-elite):
  5K:        16:53 vs 16:39 reference → 1.4% difference ✅
  10K:       35:11 vs 34:40 reference → 1.5% difference ✅
  Marathon: 2:46:10 vs 2:40:00 reference → 3.9% difference ✅

VDOT 40 (recreational):
  5K:        23:26 vs 24:44 reference → 5.2% difference ✅
  10K:       48:52 vs 51:42 reference → 5.5% difference ✅
  Marathon: 3:50:46 vs 3:57:00 reference → 2.6% difference ✅

Overall accuracy: 0.2-5.5% difference across all distances
```

**why differences exist**:
- jack daniels' tables use empirical adjustments from real runner data
- pierre uses pure mathematical VDOT formula
- 6% tolerance is excellent for race predictions (weather, course, pacing all affect actual performance)

**test verification**: `tests/vdot_table_verification_test.rs`

**reference**: Daniels, J. (2013). *Daniels' Running Formula* (3rd ed.). Human Kinetics.

---

## Performance Prediction: Riegel Formula

Predicts race times across distances using power-law relationship:

**riegel formula**:

```
T₂ = T₁ × (D₂ / D₁)^1.06
```

Where:
- `T₁` = known race time (seconds)
- `D₁` = known race distance (meters)
- `T₂` = predicted race time (seconds)
- `D₂` = target race distance (meters)
- `1.06` = riegel exponent (empirically derived constant)

**domain constraints**:
- `D₁ > 0, T₁ > 0, D₂ > 0` (all values must be positive)

**rust implementation**:

```rust
// src/intelligence/performance_prediction.rs
const RIEGEL_EXPONENT: f64 = 1.06;

pub fn predict_time_riegel(
    known_distance_m: f64,
    known_time_s: f64,
    target_distance_m: f64,
) -> Result<f64> {
    if known_distance_m <= 0.0 || known_time_s <= 0.0 || target_distance_m <= 0.0 {
        return Err(AppError::invalid_input(
            "All distances and times must be positive"
        ));
    }

    let distance_ratio = target_distance_m / known_distance_m;
    Ok(known_time_s * distance_ratio.powf(RIEGEL_EXPONENT))
}
```

**example**: predict marathon from half marathon:
- Given: T₁ = 1:30:00 = 5400s, D₁ = 21,097m
- Target: D₂ = 42,195m
- Calculation: T₂ = 5400 × (42,195 / 21,097)^1.06 ≈ 11,340s ≈ 3:09:00

**reference**: Riegel, P.S. (1981). Athletic records and human endurance. *American Scientist*, 69(3), 285-290.

---

## Pattern Detection

### Weekly Schedule

**algorithm**:

1. Count activities by weekday: `C(d) = |{activities on weekday d}|`
2. Sort weekdays by frequency: rank by descending `C(d)`
3. Calculate consistency score based on distribution

**output**:
- `most_common_days` = top 3 weekdays by activity count
- `consistency_score ∈ [0, 100]`

**rust implementation**:

```rust
// src/intelligence/pattern_detection.rs
pub fn detect_weekly_schedule(activities: &[Activity]) -> WeeklySchedulePattern {
    let mut day_counts: HashMap<Weekday, u32> = HashMap::new();

    for activity in activities {
        *day_counts.entry(activity.start_date.weekday()).or_insert(0) += 1;
    }

    let mut day_freq: Vec<(Weekday, u32)> = day_counts.into_iter().collect();
    day_freq.sort_by(|a, b| b.1.cmp(&a.1));

    let consistency_score = calculate_consistency(&day_freq, activities.len());

    WeeklySchedulePattern {
        most_common_days: day_freq.iter().take(3).map(|(d, _)| *d).collect(),
        consistency_score,
    }
}
```

**consistency interpretation**:
- 0 ≤ score < 30: highly variable
- 30 ≤ score < 60: moderate consistency
- 60 ≤ score < 80: consistent schedule
- 80 ≤ score ≤ 100: very consistent routine

### Hard/Easy Alternation

**algorithm**:

1. Classify each activity intensity: `I(a) ∈ {Hard, Easy}`
2. Sort activities chronologically by date
3. Count alternations in consecutive activities:
   ```
   Alternations = |{i : (I(aᵢ) = Hard ∧ I(aᵢ₊₁) = Easy) ∨ (I(aᵢ) = Easy ∧ I(aᵢ₊₁) = Hard)}|
   ```

4. Calculate pattern strength:
   ```
   Pattern_strength = alternations / (n − 1)
   ```
   Where `n` = number of activities

**classification**:

```
follows_pattern = true,   if pattern_strength > 0.6
                = false,  if pattern_strength ≤ 0.6
```

**rust implementation**:

```rust
pub fn detect_hard_easy_pattern(activities: &[Activity]) -> HardEasyPattern {
    let mut intensities = Vec::new();

    for activity in activities {
        let intensity = calculate_relative_intensity(activity);
        intensities.push((activity.start_date, intensity));
    }

    intensities.sort_by_key(|(date, _)| *date);

    // Detect alternation
    let mut alternations = 0;
    for window in intensities.windows(2) {
        if (window[0].1 == Intensity::Hard && window[1].1 == Intensity::Easy)
            || (window[0].1 == Intensity::Easy && window[1].1 == Intensity::Hard)
        {
            alternations += 1;
        }
    }

    let pattern_strength = (alternations as f64) / (intensities.len() as f64 - 1.0);

    HardEasyPattern {
        follows_pattern: pattern_strength > 0.6,
        pattern_strength,
    }
}
```

### Volume Progression

**algorithm**:

1. Group activities by week: compute total volume per week
2. Apply linear regression to weekly volumes (see statistical trend analysis section)
3. Classify trend based on slope:
   ```
   VolumeTrend = Increasing,  if slope > 0.05
               = Decreasing,  if slope < −0.05
               = Stable,      if −0.05 ≤ slope ≤ 0.05
   ```

**output**:
- trend classification
- slope (rate of change)
- R² (goodness of fit)

**rust implementation**:

```rust
pub fn detect_volume_progression(activities: &[Activity]) -> VolumeProgressionPattern {
    // Group by weeks
    let weekly_volumes = group_by_weeks(activities);

    // Calculate trend
    let trend_result = StatisticalAnalyzer::linear_regression(&weekly_volumes)?;

    let trend = if trend_result.slope > 0.05 {
        VolumeTrend::Increasing
    } else if trend_result.slope < -0.05 {
        VolumeTrend::Decreasing
    } else {
        VolumeTrend::Stable
    };

    VolumeProgressionPattern {
        trend,
        slope: trend_result.slope,
        r_squared: trend_result.r_squared,
    }
}
```

**reference**: Esteve-Lanao, J. Et al. (2005). How do endurance runners train? *Med Sci Sports Exerc*, 37(3), 496-504.

---

## Sleep And Recovery Analysis

### Sleep Quality Scoring

Pierre uses NSF (National Sleep Foundation) and AASM (American Academy of Sleep Medicine) guidelines for sleep quality assessment. The overall sleep quality score (0-100) combines three weighted components:

**sleep quality formula**:

```
sleep_quality = (duration_score × 0.40) + (stages_score × 0.35) + (efficiency_score × 0.25)
```

Where:
- `duration_score` weight: **40%** (emphasizes total sleep time)
- `stages_score` weight: **35%** (sleep architecture quality)
- `efficiency_score` weight: **25%** (sleep fragmentation)

#### Duration Scoring

Based on NSF recommendations with athlete-specific adjustments:

**piecewise linear scoring function**:

```
duration_score(d) = 100,                  if d ≥ 8
                  = 85 + 15(d − 7),       if 7 ≤ d < 8
                  = 60 + 25(d − 6),       if 6 ≤ d < 7
                  = 30 + 30(d − 5),       if 5 ≤ d < 6
                  = 30(d / 5),            if d < 5
```

Where `d` = sleep duration (hours)

**rust implementation**:

```rust
// src/intelligence/sleep_analysis.rs
pub fn sleep_duration_score(duration_hours: f64, config: &SleepRecoveryConfig) -> f64 {
    if duration_hours >= config.athlete_optimal_hours {        // >=8h → 100
        100.0
    } else if duration_hours >= config.adult_min_hours {       // 7-8h → 85-100
        85.0 + ((duration_hours - 7.0) / 1.0) * 15.0
    } else if duration_hours >= config.short_sleep_threshold { // 6-7h → 60-85
        60.0 + ((duration_hours - 6.0) / 1.0) * 25.0
    } else if duration_hours >= config.very_short_sleep_threshold { // 5-6h → 30-60
        30.0 + ((duration_hours - 5.0) / 1.0) * 30.0
    } else {                                                   // <5h → 0-30
        (duration_hours / 5.0) * 30.0
    }
}
```

**default thresholds**:
- **d ≥ 8 hours**: score = 100 (optimal for athletes)
- **7 ≤ d < 8 hours**: score ∈ [85, 100] (adequate for adults)
- **6 ≤ d < 7 hours**: score ∈ [60, 85] (short sleep)
- **5 ≤ d < 6 hours**: score ∈ [30, 60] (very short)
- **d < 5 hours**: score ∈ [0, 30] (severe deprivation)

**scientific basis**: NSF recommends 7-9h for adults, 8-10h for athletes. <6h linked to increased injury risk and impaired performance.

**reference**: Hirshkowitz, M. Et al. (2015). National Sleep Foundation's sleep time duration recommendations. *Sleep Health*, 1(1), 40-43.

#### Stages Scoring

Based on AASM guidelines for healthy sleep stage distribution:

**deep sleep scoring function**:

```
deep_score(p_deep) = 100,                       if p_deep ≥ 20
                   = 70 + 30(p_deep − 15)/5,    if 15 ≤ p_deep < 20
                   = 70(p_deep / 15),           if p_deep < 15
```

**REM sleep scoring function**:

```
rem_score(p_rem) = 100,                      if p_rem ≥ 25
                 = 70 + 30(p_rem − 20)/5,    if 20 ≤ p_rem < 25
                 = 70(p_rem / 20),           if p_rem < 20
```

**awake time penalty**:

```
penalty(p_awake) = 0,                  if p_awake ≤ 5
                 = 2(p_awake − 5),     if p_awake > 5
```

**combined stages score**:

```
stages_score = max(0, min(100,
               0.4 × deep_score + 0.4 × rem_score + 0.2 × p_light − penalty))
```

Where:
- `p_deep` = deep sleep percentage (%)
- `p_rem` = REM sleep percentage (%)
- `p_light` = light sleep percentage (%)
- `p_awake` = awake time percentage (%)

**rust implementation**:

```rust
// src/intelligence/sleep_analysis.rs
pub fn sleep_stages_score(
    deep_percent: f64,
    rem_percent: f64,
    light_percent: f64,
    awake_percent: f64,
    config: &SleepRecoveryConfig
) -> f64 {
    // Deep sleep: 40% weight (physical recovery)
    let deep_score = if deep_percent >= 20.0 { 100.0 }
                     else if deep_percent >= 15.0 { 70.0 + ((deep_percent - 15.0) / 5.0) * 30.0 }
                     else { (deep_percent / 15.0) * 70.0 };

    // REM sleep: 40% weight (cognitive recovery)
    let rem_score = if rem_percent >= 25.0 { 100.0 }
                    else if rem_percent >= 20.0 { 70.0 + ((rem_percent - 20.0) / 5.0) * 30.0 }
                    else { (rem_percent / 20.0) * 70.0 };

    // Awake time penalty: >5% awake reduces score
    let awake_penalty = if awake_percent > 5.0 { (awake_percent - 5.0) * 2.0 } else { 0.0 };

    // Combined: 40% deep, 40% REM, 20% light, minus awake penalty
    ((deep_score * 0.4) + (rem_score * 0.4) + (light_percent * 0.2) - awake_penalty).clamp(0.0, 100.0)
}
```

**optimal ranges**:
- **deep sleep**: 15-25% (physical recovery, growth hormone release)
- **REM sleep**: 20-25% (memory consolidation, cognitive function)
- **light sleep**: 45-55% (transition stages)
- **awake time**: <5% (sleep fragmentation indicator)

**scientific basis**: AASM sleep stage guidelines. Deep sleep critical for physical recovery, REM for cognitive processing.

**reference**: Berry, R.B. Et al. (2017). AASM Scoring Manual Version 2.4. *American Academy of Sleep Medicine*.

#### Efficiency Scoring

Based on clinical sleep medicine thresholds:

**sleep efficiency formula**:

```
efficiency = (t_asleep / t_bed) × 100
```

Where:
- `t_asleep` = total time asleep (minutes)
- `t_bed` = total time in bed (minutes)
- `efficiency ∈ [0, 100]` (percentage)

**piecewise linear scoring function**:

```
efficiency_score(e) = 100,                     if e ≥ 90
                    = 85 + 15(e − 85)/5,       if 85 ≤ e < 90
                    = 65 + 20(e − 75)/10,      if 75 ≤ e < 85
                    = 65(e / 75),              if e < 75
```

Where `e` = efficiency percentage

**rust implementation**:

```rust
// src/intelligence/sleep_analysis.rs
pub fn sleep_efficiency_score(efficiency_percent: f64, config: &SleepRecoveryConfig) -> f64 {
    if efficiency_percent >= 90.0 {       // >=90% → 100 (excellent)
        100.0
    } else if efficiency_percent >= 85.0 { // 85-90% → 85-100 (good)
        85.0 + ((efficiency_percent - 85.0) / 5.0) * 15.0
    } else if efficiency_percent >= 75.0 { // 75-85% → 65-85 (fair)
        65.0 + ((efficiency_percent - 75.0) / 10.0) * 20.0
    } else {                              // <75% → 0-65 (poor)
        (efficiency_percent / 75.0) * 65.0
    }
}
```

**thresholds**:
- **e ≥ 90%**: score = 100 (excellent, minimal sleep fragmentation)
- **85 ≤ e < 90%**: score ∈ [85, 100] (good, normal range)
- **75 ≤ e < 85%**: score ∈ [65, 85] (fair, moderate fragmentation)
- **e < 75%**: score ∈ [0, 65] (poor, severe fragmentation)

**scientific basis**: sleep efficiency >85% considered normal in clinical sleep medicine.

**input/output specification for sleep quality scoring**:

Inputs:
  Duration_hours: f64      // Sleep duration in hours, must be ≥ 0
  Deep_percent: f64        // Deep sleep percentage [0, 100]
  Rem_percent: f64         // REM sleep percentage [0, 100]
  Light_percent: f64       // Light sleep percentage [0, 100]
  Awake_percent: f64       // Awake time percentage [0, 100]
  Time_asleep_min: f64     // Total time asleep in minutes
  Time_in_bed_min: f64     // Total time in bed in minutes

Outputs:
  Sleep_quality: f64       // Overall sleep quality score [0, 100]
  Duration_score: f64      // Duration component score [0, 100]
  Stages_score: f64        // Sleep stages component score [0, 100]
  Efficiency_score: f64    // Sleep efficiency component score [0, 100]
  Efficiency_percent: f64  // Calculated efficiency (time_asleep / time_in_bed) × 100

Precision: IEEE 754 double precision (f64)
Tolerance: ±1.0 for overall score, ±2.0 for component scores due to piecewise function boundaries

**validation examples for sleep quality scoring**:

Example 1: Excellent sleep (athlete optimal)
  Input:
    duration_hours = 8.5
    deep_percent = 20.0
    rem_percent = 25.0
    light_percent = 52.0
    awake_percent = 3.0
    time_asleep_min = 510.0  (8.5 hours)
    time_in_bed_min = 540.0  (9 hours)

  Step-by-step calculation:
    1. Duration score:
       duration_hours = 8.5 ≥ 8.0 → score = 100

    2. Stages score:
       deep_score = 20.0 ≥ 20 → 100
       rem_score = 25.0 ≥ 25 → 100
       awake_penalty = 3.0 ≤ 5 → 0
       stages_score = (100 × 0.4) + (100 × 0.4) + (52.0 × 0.2) − 0
                    = 40 + 40 + 10.4 = 90.4

    3. Efficiency score:
       efficiency = (510.0 / 540.0) × 100 = 94.4%
       94.4 ≥ 90 → score = 100

    4. Overall sleep quality:
       sleep_quality = (100 × 0.40) + (90.4 × 0.35) + (100 × 0.25)
                     = 40.0 + 31.64 + 25.0 = 96.6

  Expected Output: sleep_quality = 96.6

Example 2: Good sleep (typical adult)
  Input:
    duration_hours = 7.5
    deep_percent = 18.0
    rem_percent = 22.0
    light_percent = 54.0
    awake_percent = 6.0
    time_asleep_min = 450.0  (7.5 hours)
    time_in_bed_min = 500.0  (8.33 hours)

  Step-by-step calculation:
    1. Duration score:
       7.0 ≤ 7.5 < 8.0
       score = 85 + 15 × (7.5 − 7.0) = 85 + 7.5 = 92.5

    2. Stages score:
       deep_score: 15 ≤ 18.0 < 20
                 = 70 + 30 × (18.0 − 15.0) / 5 = 70 + 18 = 88
       rem_score: 20 ≤ 22.0 < 25
                = 70 + 30 × (22.0 − 20.0) / 5 = 70 + 12 = 82
       awake_penalty = 6.0 > 5 → (6.0 − 5.0) × 2 = 2.0
       stages_score = (88 × 0.4) + (82 × 0.4) + (54.0 × 0.2) − 2.0
                    = 35.2 + 32.8 + 10.8 − 2.0 = 76.8

    3. Efficiency score:
       efficiency = (450.0 / 500.0) × 100 = 90.0%
       90.0 ≥ 90 → score = 100

    4. Overall sleep quality:
       sleep_quality = (92.5 × 0.40) + (76.8 × 0.35) + (100 × 0.25)
                     = 37.0 + 26.88 + 25.0 = 88.9

  Expected Output: sleep_quality = 88.9

Example 3: Poor sleep (short duration, fragmented)
  Input:
    duration_hours = 5.5
    deep_percent = 12.0
    rem_percent = 18.0
    light_percent = 60.0
    awake_percent = 10.0
    time_asleep_min = 330.0  (5.5 hours)
    time_in_bed_min = 420.0  (7 hours)

  Step-by-step calculation:
    1. Duration score:
       5.0 ≤ 5.5 < 6.0
       score = 30 + 30 × (5.5 − 5.0) = 30 + 15 = 45

    2. Stages score:
       deep_score: 12.0 < 15
                 = 70 × (12.0 / 15.0) = 56
       rem_score: 18.0 < 20
                = 70 × (18.0 / 20.0) = 63
       awake_penalty = (10.0 − 5.0) × 2 = 10.0
       stages_score = (56 × 0.4) + (63 × 0.4) + (60.0 × 0.2) − 10.0
                    = 22.4 + 25.2 + 12.0 − 10.0 = 49.6

    3. Efficiency score:
       efficiency = (330.0 / 420.0) × 100 = 78.57%
       75 ≤ 78.57 < 85
       score = 65 + 20 × (78.57 − 75) / 10 = 65 + 7.14 = 72.1

    4. Overall sleep quality:
       sleep_quality = (45 × 0.40) + (49.6 × 0.35) + (72.1 × 0.25)
                     = 18.0 + 17.36 + 18.025 = 53.4

  Expected Output: sleep_quality = 53.4

Example 4: Boundary condition (exactly 7 hours, 85% efficiency)
  Input:
    duration_hours = 7.0
    deep_percent = 15.0
    rem_percent = 20.0
    light_percent = 60.0
    awake_percent = 5.0
    time_asleep_min = 420.0
    time_in_bed_min = 494.12  (exactly 85% efficiency)

  Step-by-step calculation:
    1. Duration score:
       duration_hours = 7.0 (exactly at boundary)
       score = 85.0  (lower boundary of 7-8h range)

    2. Stages score:
       deep_score = 15.0 (exactly at boundary) → 70.0
       rem_score = 20.0 (exactly at boundary) → 70.0
       awake_penalty = 5.0 (exactly at threshold) → 0
       stages_score = (70 × 0.4) + (70 × 0.4) + (60 × 0.2) − 0
                    = 28 + 28 + 12 = 68.0

    3. Efficiency score:
       efficiency = (420.0 / 494.12) × 100 = 85.0% (exactly at boundary)
       score = 85.0  (lower boundary of 85-90% range)

    4. Overall sleep quality:
       sleep_quality = (85.0 × 0.40) + (68.0 × 0.35) + (85.0 × 0.25)
                     = 34.0 + 23.8 + 21.25 = 79.1

  Expected Output: sleep_quality = 79.1

**API response format for sleep quality**:

```json
{
  "user_id": "user_12345",
  "sleep_session_id": "sleep_20250115",
  "date": "2025-01-15",
  "sleep_quality": {
    "overall_score": 88.1,
    "interpretation": "good",
    "components": {
      "duration": {
        "hours": 7.5,
        "score": 92.5,
        "status": "adequate"
      },
      "stages": {
        "deep_percent": 18.0,
        "rem_percent": 22.0,
        "light_percent": 54.0,
        "awake_percent": 6.0,
        "score": 76.8,
        "deep_score": 88.0,
        "rem_score": 82.0,
        "awake_penalty": 2.0,
        "status": "good"
      },
      "efficiency": {
        "percent": 90.0,
        "time_asleep_min": 450.0,
        "time_in_bed_min": 500.0,
        "score": 100.0,
        "status": "excellent"
      }
    }
  },
  "guidelines": {
    "duration_target": "8+ hours for athletes, 7-9 hours for adults",
    "deep_sleep_target": "15-25%",
    "rem_sleep_target": "20-25%",
    "efficiency_target": ">85%"
  }
}
```

**common validation issues for sleep quality scoring**:

1. **percentage components don't sum to 100**:
   - Cause: sleep tracker rounding or missing data
   - Example: deep=18%, REM=22%, light=55%, awake=6% → sum=101%
   - Solution: normalize percentages to sum to 100% before calculation
   - Note: pierre accepts raw percentages; validation is user's responsibility

2. **efficiency > 100%**:
   - Cause: time_asleep > time_in_bed (data error)
   - Example: slept 8 hours but only in bed 7 hours
   - Solution: validate time_asleep ≤ time_in_bed before calculation

3. **boundary discontinuities in scoring**:
   - At duration thresholds (5h, 6h, 7h, 8h), score changes slope
   - Example: 6.99h → score ≈85, but 7.01h → score ≈85.15 (not discontinuous)
   - Piecewise functions are continuous but have slope changes
   - Tolerance: ±2 points near boundaries acceptable

4. **very high awake percentage (>20%)**:
   - Causes large penalty in stages_score
   - Example: awake=25% → penalty=(25-5)×2=40 points
   - Can result in negative stages_score (clamped to 0)
   - Solution: investigate sleep fragmentation; may indicate sleep disorder

5. **missing sleep stage data**:
   - Some trackers don't provide detailed stages
   - Without stages, cannot calculate complete sleep_quality
   - Solution: use duration + efficiency only, or return error

6. **athlete vs non-athlete thresholds**:
   - Current implementation uses athlete-optimized thresholds (8h optimal)
   - Non-athletes may see lower scores with 7-8h sleep
   - Solution: configuration parameter athlete_optimal_hours (default: 8.0)

7. **sleep duration > 12 hours**:
   - Very long sleep may indicate oversleeping or health issue
   - Current formula caps at 100 for duration ≥ 8h
   - 12h sleep gets same score as 8h sleep
   - Solution: document that >10h is not necessarily better

8. **comparison with consumer sleep trackers**:
   - Consumer trackers (Fitbit, Apple Watch) may use proprietary scoring
   - Pierre uses NSF/AASM validated scientific guidelines
   - Expect 5-15 point difference between trackers
   - Solution: pierre is more conservative and scientifically grounded

**validation workflow for sleep quality**:

1. **validate input data**:
   ```bash
   Assert duration_hours ≥ 0
   Assert 0 ≤ deep_percent ≤ 100
   Assert 0 ≤ rem_percent ≤ 100
   Assert 0 ≤ light_percent ≤ 100
   Assert 0 ≤ awake_percent ≤ 100
   Assert time_asleep_min ≤ time_in_bed_min
   ```

2. **calculate component scores**:
   ```bash
   Duration_score = score_duration(duration_hours)
   Stages_score = score_stages(deep%, rem%, light%, awake%)
   Efficiency = (time_asleep / time_in_bed) × 100
   Efficiency_score = score_efficiency(efficiency)
   ```

3. **calculate weighted overall score**:
   ```bash
   Sleep_quality = (duration_score × 0.40) + (stages_score × 0.35) + (efficiency_score × 0.25)
   Assert 0 ≤ sleep_quality ≤ 100
   ```

4. **compare with expected ranges**:
   - Excellent: 85-100
   - Good: 70-85
   - Fair: 50-70
   - Poor: <50

### Recovery Score Calculation

Pierre calculates training readiness by combining TSB, sleep quality, and HRV (when available):

**weighted recovery score formula**:

```
recovery_score = 0.4 × TSB_score + 0.4 × sleep_score + 0.2 × HRV_score,  if HRV available
               = 0.5 × TSB_score + 0.5 × sleep_score,                    if HRV unavailable
```

Where:
- `TSB_score` = normalized TSB score ∈ [0, 100] (see TSB normalization below)
- `sleep_score` = overall sleep quality score ∈ [0, 100] (from sleep analysis)
- `HRV_score` = heart rate variability score ∈ [0, 100] (when available)

**recovery level classification**:

```
recovery_level = excellent,  if score ≥ 85
               = good,       if 70 ≤ score < 85
               = fair,       if 50 ≤ score < 70
               = poor,       if score < 50
```

**rust implementation**:

```rust
// src/intelligence/recovery_calculator.rs
pub fn calculate_recovery_score(
    tsb: f64,
    sleep_quality: f64,
    hrv_data: Option<HrvData>,
    config: &SleepRecoveryConfig
) -> RecoveryScore {
    // 1. Normalize TSB from [-30, +30] to [0, 100]
    let tsb_score = normalize_tsb(tsb);

    // 2. Sleep already scored [0, 100]

    // 3. Score HRV if available
    let (recovery_score, components) = match hrv_data {
        Some(hrv) => {
            let hrv_score = score_hrv(hrv, config);
            // Weights: 40% TSB, 40% sleep, 20% HRV
            let score = (tsb_score * 0.4) + (sleep_quality * 0.4) + (hrv_score * 0.2);
            (score, (tsb_score, sleep_quality, Some(hrv_score)))
        },
        None => {
            // Weights: 50% TSB, 50% sleep (no HRV)
            let score = (tsb_score * 0.5) + (sleep_quality * 0.5);
            (score, (tsb_score, sleep_quality, None))
        }
    };

    // 4. Classify recovery level
    let level = if recovery_score >= 85.0 { "excellent" }
                else if recovery_score >= 70.0 { "good" }
                else if recovery_score >= 50.0 { "fair" }
                else { "poor" };

    RecoveryScore { score: recovery_score, level, components }
}
```

#### TSB Normalization

Training stress balance maps to recovery score using **configurable thresholds**, not fixed breakpoints:

**configurable TSB thresholds** (from `SleepRecoveryConfig.training_stress_balance`):

```rust
// Default configuration values (src/config/intelligence_config.rs:1178)
TsbConfig {
    highly_fatigued_tsb: -15.0,    // Extreme fatigue threshold
    fatigued_tsb: -10.0,            // Productive fatigue threshold
    fresh_tsb_min: 5.0,             // Optimal fresh range start
    fresh_tsb_max: 15.0,            // Optimal fresh range end
    detraining_tsb: 25.0,           // Detraining risk threshold
}
```

**rust implementation**:

```rust
// src/intelligence/recovery_calculator.rs:250
pub fn score_tsb(
    tsb: f64,
    config: &SleepRecoveryConfig,
) -> f64 {
    let detraining_tsb = config.training_stress_balance.detraining_tsb;
    let fresh_tsb_max = config.training_stress_balance.fresh_tsb_max;
    let fresh_tsb_min = config.training_stress_balance.fresh_tsb_min;
    let fatigued_tsb = config.training_stress_balance.fatigued_tsb;
    let highly_fatigued_tsb = config.training_stress_balance.highly_fatigued_tsb;

    if (fresh_tsb_min..=fresh_tsb_max).contains(&tsb) {
        // Optimal fresh range: 100 points
        100.0
    } else if tsb > detraining_tsb {
        // Too fresh (risk of detraining): penalize
        100.0 - ((tsb - detraining_tsb) * 2.0).min(30.0)
    } else if tsb > fresh_tsb_max {
        // Between optimal and detraining: slight penalty
        ((tsb - fresh_tsb_max) / (detraining_tsb - fresh_tsb_max)).mul_add(-10.0, 100.0)
    } else if tsb >= 0.0 {
        // Slightly fresh (0 to fresh_tsb_min): 85-100 points
        (tsb / fresh_tsb_min).mul_add(15.0, 85.0)
    } else if tsb >= fatigued_tsb {
        // Productive fatigue: 60-85 points
        ((tsb - fatigued_tsb) / fatigued_tsb.abs()).mul_add(25.0, 60.0)
    } else if tsb >= highly_fatigued_tsb {
        // High fatigue: 30-60 points
        ((tsb - highly_fatigued_tsb) / (fatigued_tsb - highly_fatigued_tsb)).mul_add(30.0, 30.0)
    } else {
        // Extreme fatigue: 0-30 points
        30.0 - ((tsb.abs() - highly_fatigued_tsb.abs()) / highly_fatigued_tsb.abs() * 30.0)
            .min(30.0)
    }
}
```

**scoring ranges** (with default config):
- **TSB > +25**: score ∈ [70, 100] decreasing - detraining risk (too much rest)
- **+15 < TSB ≤ +25**: score ∈ [90, 100] - approaching detraining
- **+5 ≤ TSB ≤ +15**: score = **100** - optimal fresh zone (race ready)
- **0 ≤ TSB < +5**: score ∈ [85, 100] - slightly fresh
- **−10 ≤ TSB < 0**: score ∈ [60, 85] - productive fatigue (building fitness)
- **−15 ≤ TSB < −10**: score ∈ [30, 60] - high fatigue
- **TSB < −15**: score ∈ [0, 30] - extreme fatigue (recovery needed)

**configurable via environment**:
- `INTELLIGENCE_TSB_HIGHLY_FATIGUED` (default: -15.0)
- `INTELLIGENCE_TSB_FATIGUED` (default: -10.0)
- `INTELLIGENCE_TSB_FRESH_MIN` (default: 5.0)
- `INTELLIGENCE_TSB_FRESH_MAX` (default: 15.0)
- `INTELLIGENCE_TSB_DETRAINING` (default: 25.0)

**reference**: Banister, E.W. (1991). Modeling elite athletic performance. *Human Kinetics*.

#### HRV Scoring

Heart rate variability assessment based on categorical recovery status, not continuous RMSSD scoring:

**recovery status determination**:

Pierre first classifies HRV into a **categorical recovery status** (`HrvRecoveryStatus` enum) based on RMSSD comparison to baseline and weekly average:

```rust
// src/intelligence/sleep_analysis.rs:558
fn determine_hrv_recovery_status(
    current: f64,
    weekly_avg: f64,
    baseline_deviation: Option<f64>,
    config: &SleepRecoveryConfig,
) -> HrvRecoveryStatus {
    // Check baseline deviation first (if available)
    if let Some(deviation) = baseline_deviation {
        if deviation < -baseline_deviation_concern {
            return HrvRecoveryStatus::HighlyFatigued;
        } else if deviation < -5.0 {
            return HrvRecoveryStatus::Fatigued;
        }
    }

    // Compare to weekly average
    let change_from_avg = current - weekly_avg;
    if change_from_avg >= rmssd_increase_threshold {
        HrvRecoveryStatus::Recovered
    } else if change_from_avg <= rmssd_decrease_threshold {
        HrvRecoveryStatus::Fatigued
    } else {
        HrvRecoveryStatus::Normal
    }
}
```

**discrete HRV scoring function**:

Pierre maps the categorical recovery status to a **fixed discrete score**, not a continuous function:

```rust
// src/intelligence/recovery_calculator.rs:288
pub const fn score_hrv(hrv: &HrvTrendAnalysis) -> f64 {
    match hrv.recovery_status {
        HrvRecoveryStatus::Recovered => 100.0,
        HrvRecoveryStatus::Normal => 70.0,
        HrvRecoveryStatus::Fatigued => 40.0,
        HrvRecoveryStatus::HighlyFatigued => 20.0,
    }
}
```

**recovery status interpretation**:
- **Recovered**: score = **100** - elevated HRV, ready for high-intensity training
- **Normal**: score = **70** - HRV within normal range, continue current training load
- **Fatigued**: score = **40** - decreased HRV, consider reducing training intensity
- **HighlyFatigued**: score = **20** - significantly decreased HRV, prioritize recovery

Where:
- `RMSSD` = root mean square of successive RR interval differences (milliseconds)
- `weekly_avg` = 7-day rolling average of RMSSD
- `baseline_deviation` = percent change from long-term baseline (if established)
- `rmssd_increase_threshold` = typically +5ms (configurable)
- `rmssd_decrease_threshold` = typically -10ms (configurable)
- `baseline_deviation_concern` = typically -15% (configurable)

**scientific basis**: HRV (specifically RMSSD) reflects autonomic nervous system recovery. Decreases indicate accumulated fatigue, increases indicate good adaptation. Pierre uses discrete categories rather than continuous scoring to provide clear, actionable recovery guidance.

**reference**: Plews, D.J. Et al. (2013). Training adaptation and heart rate variability in elite endurance athletes. *Int J Sports Physiol Perform*, 8(3), 286-293.

**input/output specification for recovery score**:

Inputs:
  Tsb: f64                 // Training Stress Balance, typically [-30, +30]
  Sleep_quality: f64       // Sleep quality score [0, 100]
  Hrv_rmssd: Option<f64>   // Current HRV RMSSD (ms), optional
  Hrv_baseline: Option<f64>  // Baseline HRV RMSSD (ms), optional

Outputs:
  Recovery_score: f64      // Overall recovery score [0, 100]
  Tsb_score: f64           // Normalized TSB component [0, 100]
  Sleep_score: f64         // Sleep component [0, 100] (pass-through)
  Hrv_score: Option<f64>   // HRV component [0, 100], if available
  Recovery_level: String   // Classification: excellent/good/fair/poor

Precision: IEEE 754 double precision (f64)
Tolerance: ±2.0 for overall score due to piecewise function boundaries and component weighting

**validation examples for recovery score**:

Example 1: Excellent recovery (with HRV, fresh athlete)
  Input:
    tsb = 8.0
    sleep_quality = 92.0
    hrv_rmssd = 55.0
    hrv_baseline = 50.0

  Step-by-step calculation:
    1. Normalize TSB (5 ≤ 8.0 < 15):
       tsb_score = 80 + 10 × (8.0 − 5.0) / 10 = 80 + 3 = 83

    2. Sleep score (pass-through):
       sleep_score = 92.0

    3. HRV score:
       current_rmssd = 55.0, weekly_avg_rmssd ≈ 50.0
       change_from_avg = 55.0 − 50.0 = +5.0ms
       +5.0 ≥ +5.0 threshold → HrvRecoveryStatus::Recovered → score = 100

    4. Recovery score (with HRV: 40% TSB, 40% sleep, 20% HRV):
       recovery_score = (83 × 0.4) + (92 × 0.4) + (100 × 0.2)
                     = 33.2 + 36.8 + 20.0 = 90.0

    5. Classification:
       90.0 ≥ 85 → "excellent"

  Expected Output:
    recovery_score = 90.0
    recovery_level = "excellent"

Example 2: Good recovery (no HRV, moderate training)
  Input:
    tsb = 2.0
    sleep_quality = 78.0
    hrv_rmssd = None
    hrv_baseline = None

  Step-by-step calculation:
    1. Normalize TSB (-5 ≤ 2.0 < 5):
       tsb_score = 60 + 20 × (2.0 + 5.0) / 10 = 60 + 14 = 74

    2. Sleep score:
       sleep_score = 78.0

    3. HRV score:
       hrv_score = None

    4. Recovery score (without HRV: 50% TSB, 50% sleep):
       recovery_score = (74 × 0.5) + (78 × 0.5)
                     = 37.0 + 39.0 = 76.0

    5. Classification:
       70 ≤ 76.0 < 85 → "good"

  Expected Output:
    recovery_score = 76.0
    recovery_level = "good"

Example 3: Poor recovery (fatigued with poor sleep)
  Input:
    tsb = -12.0
    sleep_quality = 55.0
    hrv_rmssd = 42.0
    hrv_baseline = 50.0

  Step-by-step calculation:
    1. Normalize TSB (-15 ≤ -12.0 < -10):
       tsb_score = 20 + 20 × (-12.0 + 15.0) / 5 = 20 + 12 = 32

    2. Sleep score:
       sleep_score = 55.0

    3. HRV score:
       current_rmssd = 42.0, baseline = 50.0
       baseline_deviation = (42.0 − 50.0) / 50.0 × 100 = -16%
       -16% < -5.0% threshold → HrvRecoveryStatus::Fatigued → score = 40

    4. Recovery score (with HRV):
       recovery_score = (32 × 0.4) + (55 × 0.4) + (40 × 0.2)
                     = 12.8 + 22.0 + 8.0 = 42.8

    5. Classification:
       42.8 < 50 → "poor"

  Expected Output:
    recovery_score = 42.8
    recovery_level = "poor"

Example 4: Fair recovery (overreached but sleeping well)
  Input:
    tsb = -7.0
    sleep_quality = 88.0
    hrv_rmssd = None
    hrv_baseline = None

  Step-by-step calculation:
    1. Normalize TSB (-10 ≤ -7.0 < -5):
       tsb_score = 40 + 20 × (-7.0 + 10.0) / 5 = 40 + 12 = 52

    2. Sleep score:
       sleep_score = 88.0

    3. HRV score:
       hrv_score = None

    4. Recovery score (without HRV):
       recovery_score = (52 × 0.5) + (88 × 0.5)
                     = 26.0 + 44.0 = 70.0

    5. Classification:
       70.0 = 70 (exactly at boundary) → "good"

  Expected Output:
    recovery_score = 70.0
    recovery_level = "good"

Example 5: Boundary condition (extreme fatigue, excellent sleep/HRV)
  Input:
    tsb = -25.0
    sleep_quality = 95.0
    hrv_rmssd = 62.0
    hrv_baseline = 50.0

  Step-by-step calculation:
    1. Normalize TSB (TSB < -15):
       tsb_score = max(0, 20 × (-25.0 + 30.0) / 15) = max(0, 6.67) = 6.67

    2. Sleep score:
       sleep_score = 95.0

    3. HRV score:
       current_rmssd = 62.0, weekly_avg_rmssd ≈ 50.0
       change_from_avg = 62.0 − 50.0 = +12.0ms
       +12.0 ≥ +5.0 threshold → HrvRecoveryStatus::Recovered → score = 100

    4. Recovery score:
       recovery_score = (6.67 × 0.4) + (95 × 0.4) + (100 × 0.2)
                     = 2.67 + 38.0 + 20.0 = 60.67

    5. Classification:
       50 ≤ 60.67 < 70 → "fair"

  Expected Output:
    recovery_score = 60.67
    recovery_level = "fair"
    Note: Despite excellent sleep and HRV, extreme training fatigue (TSB=-25)
    significantly impacts overall recovery. This demonstrates TSB's 40% weight.

**API response format for recovery score**:

```json
{
  "user_id": "user_12345",
  "date": "2025-01-15",
  "recovery": {
    "overall_score": 88.0,
    "level": "excellent",
    "interpretation": "Well recovered and ready for high-intensity training",
    "components": {
      "tsb": {
        "raw_value": 8.0,
        "normalized_score": 83.0,
        "weight": 0.4,
        "contribution": 33.2,
        "status": "fresh"
      },
      "sleep": {
        "score": 92.0,
        "weight": 0.4,
        "contribution": 36.8,
        "status": "excellent"
      },
      "hrv": {
        "rmssd_current": 55.0,
        "rmssd_baseline": 50.0,
        "delta": 5.0,
        "score": 90.0,
        "weight": 0.2,
        "contribution": 18.0,
        "status": "excellent"
      }
    }
  },
  "recommendations": {
    "training_readiness": "high",
    "suggested_intensity": "Can handle high-intensity or race-pace efforts",
    "rest_needed": false
  },
  "historical_context": {
    "7_day_average": 82.5,
    "trend": "improving"
  }
}
```

**common validation issues for recovery scoring**:

1. **HRV available vs unavailable changes weights**:
   - With HRV: 40% TSB, 40% sleep, 20% HRV
   - Without HRV: 50% TSB, 50% sleep
   - Same TSB and sleep values produce different recovery scores
   - Example: TSB=80, sleep=90 → with HRV (90): 86.0, without HRV: 85.0
   - Solution: document which weights were used in API response

2. **TSB outside typical range [-30, +30]**:
   - TSB < -30: normalization formula gives score < 0 (clamped to 0)
   - TSB > +30: normalization caps at 100 (TSB ≥ 15 → score ≥ 90)
   - Extreme TSB values are physiologically unrealistic for sustained periods
   - Solution: validate TSB is reasonable before recovery calculation

3. **HRV baseline not established**:
   - Requires 7-14 days of consistent morning HRV measurements
   - Without baseline, cannot calculate meaningful HRV_score
   - Using population average (50ms) is inaccurate (individual variation 20-100ms)
   - Solution: return recovery without HRV component until baseline established

4. **recovery score boundaries**:
   - At 50, 70, 85 boundaries, classification changes
   - Example: 69.9 → "fair", but 70.0 → "good"
   - Score 84.9 is "good" but user might feel "excellent"
   - Solution: display numerical score alongside classification

5. **conflicting component signals**:
   - Example: excellent sleep (95) but poor TSB (-20) and HRV (-8ms)
   - Recovery score may be "fair" despite great sleep
   - Users may be confused why good sleep doesn't mean full recovery
   - Solution: show component breakdown so users understand weighted contributions

6. **acute vs chronic fatigue mismatches**:
   - TSB reflects training load (chronic)
   - HRV reflects autonomic recovery (acute)
   - Sleep reflects restfulness (acute)
   - Possible to have: TSB fresh (+10) but HRV poor (-5ms) from illness
   - Solution: recovery score balances all factors; investigate component discrepancies

7. **comparison with other platforms**:
   - Whoop, Garmin, Oura use proprietary recovery algorithms
   - Pierre uses transparent, scientifically-validated formulas
   - Expect 5-20 point differences between platforms
   - Solution: pierre prioritizes scientific validity over matching proprietary scores

8. **recovery score vs subjective feeling mismatch**:
   - Score is objective measure; feeling is subjective
   - Mental fatigue, stress, nutrition not captured
   - Example: score 80 ("good") but athlete feels exhausted from work stress
   - Solution: recovery score is one input to training decisions, not sole determinant

**validation workflow for recovery score**:

1. **validate input data**:
   ```bash
   # TSB typically in [-30, +30] but accept wider range
   Assert -50.0 ≤ tsb ≤ +50.0
   Assert 0.0 ≤ sleep_quality ≤ 100.0

   # If HRV provided, both current and baseline required
   If hrv_rmssd.is_some():
       assert hrv_baseline.is_some()
       assert hrv_rmssd > 0 && hrv_baseline > 0
   ```

2. **normalize TSB**:
   ```bash
   Tsb_score = normalize_tsb(tsb)  # See TSB normalization formula
   Assert 0.0 ≤ tsb_score ≤ 100.0
   ```

3. **score HRV if available**:
   ```bash
   If hrv_rmssd and weekly_avg_rmssd and baseline_deviation:
       # Determine categorical recovery status
       hrv_status = determine_hrv_recovery_status(hrv_rmssd, weekly_avg_rmssd, baseline_deviation)

       # Map status to discrete score
       hrv_score = score_hrv(hrv_status)  # Recovered→100, Normal→70, Fatigued→40, HighlyFatigued→20
       assert hrv_score ∈ {100.0, 70.0, 40.0, 20.0}
   ```

4. **calculate weighted recovery score**:
   ```bash
   If hrv_score:
       recovery = (tsb_score × 0.4) + (sleep_quality × 0.4) + (hrv_score × 0.2)
   Else:
       recovery = (tsb_score × 0.5) + (sleep_quality × 0.5)

   Assert 0.0 ≤ recovery ≤ 100.0
   ```

5. **classify recovery level**:
   ```bash
   Level = if recovery ≥ 85.0: "excellent"
           else if recovery ≥ 70.0: "good"
           else if recovery ≥ 50.0: "fair"
           else: "poor"
   ```

6. **validate component contributions**:
   ```bash
   # Component contributions should sum to recovery_score
   Total_contribution = (tsb_score × tsb_weight) +
                       (sleep_quality × sleep_weight) +
                       (hrv_score × hrv_weight if HRV)

   Assert abs(total_contribution - recovery_score) < 0.1  # floating point tolerance
   ```

### Configuration

All sleep/recovery thresholds configurable via environment variables:

```bash
# Sleep duration thresholds (hours)
PIERRE_SLEEP_ADULT_MIN_HOURS=7.0
PIERRE_SLEEP_ATHLETE_OPTIMAL_HOURS=8.0
PIERRE_SLEEP_SHORT_THRESHOLD=6.0
PIERRE_SLEEP_VERY_SHORT_THRESHOLD=5.0

# Sleep stages thresholds (percentage)
PIERRE_SLEEP_DEEP_MIN_PERCENT=15.0
PIERRE_SLEEP_DEEP_OPTIMAL_PERCENT=20.0
PIERRE_SLEEP_REM_MIN_PERCENT=20.0
PIERRE_SLEEP_REM_OPTIMAL_PERCENT=25.0

# Sleep efficiency thresholds (percentage)
PIERRE_SLEEP_EFFICIENCY_EXCELLENT=90.0
PIERRE_SLEEP_EFFICIENCY_GOOD=85.0
PIERRE_SLEEP_EFFICIENCY_POOR=70.0

# HRV thresholds (milliseconds)
PIERRE_HRV_RMSSD_DECREASE_CONCERN=-10.0
PIERRE_HRV_RMSSD_INCREASE_GOOD=5.0

# TSB thresholds
PIERRE_TSB_HIGHLY_FATIGUED=-15.0
PIERRE_TSB_FATIGUED=-10.0
PIERRE_TSB_FRESH_MIN=5.0
PIERRE_TSB_FRESH_MAX=15.0
PIERRE_TSB_DETRAINING=25.0

# Recovery scoring weights
PIERRE_RECOVERY_TSB_WEIGHT_FULL=0.4
PIERRE_RECOVERY_SLEEP_WEIGHT_FULL=0.4
PIERRE_RECOVERY_HRV_WEIGHT_FULL=0.2
PIERRE_RECOVERY_TSB_WEIGHT_NO_HRV=0.5
PIERRE_RECOVERY_SLEEP_WEIGHT_NO_HRV=0.5
```

Defaults based on peer-reviewed research (NSF, AASM, Shaffer & Ginsberg 2017).

---

## Validation And Safety

### Parameter Bounds (physiological ranges)

**physiological parameter ranges**:

```
max_hr ∈ [100, 220] bpm
resting_hr ∈ [30, 100] bpm
threshold_hr ∈ [100, 200] bpm
VO2max ∈ [20.0, 90.0] ml/kg/min
FTP ∈ [50, 600] watts
```

**range validation**: each parameter verified against physiologically plausible bounds

**relationship validation**:

```
resting_hr < threshold_hr < max_hr
```

Validation constraints:
- `HR_rest < HR_max` (resting heart rate below maximum)
- `HR_rest < HR_threshold` (resting heart rate below threshold)
- `HR_threshold < HR_max` (threshold heart rate below maximum)

**rust implementation**:

```rust
// src/intelligence/physiological_constants.rs::configuration_validation
pub const MAX_HR_MIN: u64 = 100;
pub const MAX_HR_MAX: u64 = 220;
pub const RESTING_HR_MIN: u64 = 30;
pub const RESTING_HR_MAX: u64 = 100;
pub const THRESHOLD_HR_MIN: u64 = 100;
pub const THRESHOLD_HR_MAX: u64 = 200;
pub const VO2_MAX_MIN: f64 = 20.0;
pub const VO2_MAX_MAX: f64 = 90.0;
pub const FTP_MIN: u64 = 50;
pub const FTP_MAX: u64 = 600;

// src/protocols/universal/handlers/configuration.rs
pub fn validate_parameter_ranges(
    obj: &serde_json::Map<String, serde_json::Value>,
    errors: &mut Vec<String>,
) -> bool {
    let mut all_valid = true;

    // Validate max_hr
    if let Some(hr) = obj.get("max_hr").and_then(Value::as_u64) {
        if !(MAX_HR_MIN..=MAX_HR_MAX).contains(&hr) {
            all_valid = false;
            errors.push(format!(
                "max_hr must be between {MAX_HR_MIN} and {MAX_HR_MAX} bpm, got {hr}"
            ));
        }
    }

    // Validate resting_hr
    if let Some(hr) = obj.get("resting_hr").and_then(Value::as_u64) {
        if !(RESTING_HR_MIN..=RESTING_HR_MAX).contains(&hr) {
            all_valid = false;
            errors.push(format!(
                "resting_hr must be between {RESTING_HR_MIN} and {RESTING_HR_MAX} bpm, got {hr}"
            ));
        }
    }

    // ... other validations

    all_valid
}

pub fn validate_parameter_relationships(
    obj: &serde_json::Map<String, serde_json::Value>,
    errors: &mut Vec<String>,
) -> bool {
    let mut all_valid = true;

    let max_hr = obj.get("max_hr").and_then(Value::as_u64);
    let resting_hr = obj.get("resting_hr").and_then(Value::as_u64);
    let threshold_hr = obj.get("threshold_hr").and_then(Value::as_u64);

    // Validate resting_hr < threshold_hr < max_hr
    if let (Some(resting), Some(max)) = (resting_hr, max_hr) {
        if resting >= max {
            all_valid = false;
            errors.push(format!(
                "resting_hr ({resting}) must be less than max_hr ({max})"
            ));
        }
    }

    if let (Some(resting), Some(threshold)) = (resting_hr, threshold_hr) {
        if resting >= threshold {
            all_valid = false;
            errors.push(format!(
                "resting_hr ({resting}) must be less than threshold_hr ({threshold})"
            ));
        }
    }

    if let (Some(threshold), Some(max)) = (threshold_hr, max_hr) {
        if threshold >= max {
            all_valid = false;
            errors.push(format!(
                "threshold_hr ({threshold}) must be less than max_hr ({max})"
            ));
        }
    }

    all_valid
}
```

**references**:
- ACSM Guidelines for Exercise Testing and Prescription, 11th Edition
- European Society of Cardiology guidelines on exercise testing

### Confidence Levels

**confidence level classification**:

```
confidence(n, R²) = High,      if (n ≥ 15) ∧ (R² ≥ 0.7)
                  = Medium,    if (n ≥ 8) ∧ (R² ≥ 0.5)
                  = Low,       if (n ≥ 3) ∧ (R² ≥ 0.3)
                  = VeryLow,   otherwise
```

Where:
- `n` = number of data points
- `R²` = coefficient of determination ∈ [0, 1]

**rust implementation**:

```rust
pub fn calculate_confidence(
    data_points: usize,
    r_squared: f64,
) -> ConfidenceLevel {
    match (data_points, r_squared) {
        (n, r) if n >= 15 && r >= 0.7 => ConfidenceLevel::High,
        (n, r) if n >= 8  && r >= 0.5 => ConfidenceLevel::Medium,
        (n, r) if n >= 3  && r >= 0.3 => ConfidenceLevel::Low,
        _ => ConfidenceLevel::VeryLow,
    }
}
```

### Edge Case Handling

**1. Users with no activities**:

```
If |activities| = 0, return:
  CTL = 0
  ATL = 0
  TSB = 0
  TSS_history = ∅ (empty set)
```

**rust implementation**:
```rust
if activities.is_empty() {
    return Ok(TrainingLoad {
        ctl: 0.0,
        atl: 0.0,
        tsb: 0.0,
        tss_history: Vec::new(),
    });
}
```

**2. Training gaps (TSS sequence breaks)**:

```
For missing days: TSS_daily = 0

Exponential decay: EMAₜ = (1 − α) × EMAₜ₋₁
```

Result: CTL/ATL naturally decay during breaks (realistic fitness loss)

**rust implementation**:
```rust
// Zero-fill missing days in EMA calculation
let daily_tss = tss_map.get(&date_key).copied().unwrap_or(0.0); // Gap = 0
ema = daily_tss.mul_add(alpha, ema * (1.0 - alpha));
```

**3. Invalid physiological parameters**:

Range validation checks:
- `max_hr = 250` → rejected (exceeds upper bound 220)
- `resting_hr = 120` → rejected (exceeds upper bound 100)

Relationship validation checks:
- `max_hr = 150, resting_hr = 160` → rejected (violates `HR_rest < HR_max`)

Returns detailed error messages for each violation

**4. Invalid race velocities**:

Velocity constraint: `v ∈ [100, 500]` m/min

If `v ∉ [100, 500]`, reject with error message

**rust implementation**:
```rust
if !(MIN_VELOCITY..=MAX_VELOCITY).contains(&velocity) {
    return Err(AppError::invalid_input(format!(
        "Velocity {velocity:.1} m/min outside valid range (100-500)"
    )));
}
```

**5. VDOT out of range**:

VDOT constraint: `VDOT ∈ [30, 85]`

If `VDOT ∉ [30, 85]`, reject with error message

**rust implementation**:
```rust
if !(30.0..=85.0).contains(&vdot) {
    return Err(AppError::invalid_input(format!(
        "VDOT {vdot:.1} outside typical range (30-85)"
    )));
}
```

---

## Configuration Strategies

Three strategies adjust training thresholds:

### Conservative Strategy

**parameters**:
- `max_weekly_load_increase = 0.05` (5%)
- `recovery_threshold = 1.2`

**rust implementation**:
```rust
impl IntelligenceStrategy for ConservativeStrategy {
    fn max_weekly_load_increase(&self) -> f64 { 0.05 } // 5%
    fn recovery_threshold(&self) -> f64 { 1.2 }
}
```

**recommended for**: injury recovery, beginners, older athletes

### Default Strategy

**parameters**:
- `max_weekly_load_increase = 0.10` (10%)
- `recovery_threshold = 1.3`

**rust implementation**:
```rust
impl IntelligenceStrategy for DefaultStrategy {
    fn max_weekly_load_increase(&self) -> f64 { 0.10 } // 10%
    fn recovery_threshold(&self) -> f64 { 1.3 }
}
```

**recommended for**: general training, recreational athletes

### Aggressive Strategy

**parameters**:
- `max_weekly_load_increase = 0.15` (15%)
- `recovery_threshold = 1.5`

**rust implementation**:
```rust
impl IntelligenceStrategy for AggressiveStrategy {
    fn max_weekly_load_increase(&self) -> f64 { 0.15 } // 15%
    fn recovery_threshold(&self) -> f64 { 1.5 }
}
```

**recommended for**: competitive athletes, experienced trainers

---

## Testing And Verification

### Test Coverage

**unit tests** (22 functions, 562 assertions):
- `tests/pattern_detection_test.rs` - 4 tests
- `tests/performance_prediction_test.rs` - 9 tests
- `tests/training_load_test.rs` - 6 tests
- `tests/vdot_table_verification_test.rs` - 3 tests

**integration tests** (116+ test files):
- Full MCP tool workflows
- Multi-provider scenarios
- Edge case handling
- Error recovery

**automated intelligence testing** (30+ integration tests):
- `tests/intelligence_tools_basic_test.rs` - 10 tests covering basic fitness data tools
- `tests/intelligence_tools_advanced_test.rs` - 20+ tests covering analytics, predictions, and goals
- `tests/intelligence_synthetic_helpers_test.rs` - synthetic data generation validation

**synthetic data framework** (`tests/helpers/`):
- `synthetic_provider.rs` - mock fitness provider with realistic activity data
- `synthetic_data.rs` - configurable test scenarios (beginner runner, experienced cyclist, multi-sport)
- `test_utils.rs` - test utilities and scenario builders
- enables testing all 8 intelligence tools without OAuth dependencies

### Verification Methods

**1. Scientific validation**:
- VDOT predictions: 0.2-5.5% accuracy vs. jack daniels' tables
- TSS formulas: match coggan's published methodology
- Statistical methods: verified against standard regression algorithms

**2. Edge case testing**:
```rust
#[test]
fn test_empty_activities() {
    let result = TrainingLoadCalculator::new()
        .calculate_training_load(&[], None, None, None, None, None)
        .unwrap();
    assert_eq!(result.ctl, 0.0);
    assert_eq!(result.atl, 0.0);
}

#[test]
fn test_training_gaps() {
    // Activities: day 1, day 10 (9-day gap)
    // EMA should decay naturally through the gap
    let activities = create_activities_with_gap();
    let result = calculate_training_load(&activities).unwrap();
    // Verify CTL decay through gap
}

#[test]
fn test_invalid_hr_relationships() {
    let config = json!({
        "max_hr": 150,
        "resting_hr": 160
    });
    let result = validate_configuration(&config);
    assert!(result.errors.contains("resting_hr must be less than max_hr"));
}
```

**3. Placeholder elimination**:
```bash
# Zero placeholders confirmed
rg -i "placeholder|todo|fixme|hack|stub" src/ | wc -l
# Output: 0
```

**4. Synthetic data testing**:
```rust
// Example: Test fitness score calculation with synthetic data
#[tokio::test]
async fn test_fitness_score_calculation() {
    let provider = create_synthetic_provider_with_scenario(
        TestScenario::ExperiencedCyclistConsistent
    );

    let activities = provider.get_activities(Some(100), None)
        .await.expect("Should get activities");

    let analyzer = PerformanceAnalyzerV2::new(Box::new(DefaultStrategy))
        .expect("Should create analyzer");

    let fitness_score = analyzer.calculate_fitness_score(&activities)
        .expect("Should calculate fitness score");

    // Verify realistic fitness score for experienced cyclist
    assert!(fitness_score.overall_score >= 70.0);
    assert!(fitness_score.overall_score <= 90.0);
}
```

**5. Code quality**:
```bash
# Zero clippy warnings (pedantic + nursery)
cargo clippy -- -W clippy::all -W clippy::pedantic -W clippy::nursery -D warnings
# Output: PASS

# Zero prohibited patterns
rg "unwrap\(\)|expect\(|panic!\(|anyhow!\(" src/ | wc -l
# Output: 0
```

---

## Debugging And Validation Guide

This comprehensive guide helps API users troubleshoot discrepancies between expected and actual calculations.

### General Debugging Workflow

When your calculated values don't match pierre's API responses, follow this systematic approach:

**1. Verify input data quality**

```bash
# Check for data integrity issues
- Missing values: NULL, NaN, undefined
- Out-of-range values: negative durations, power > 2000W, HR > 220bpm
- Unit mismatches: meters vs kilometers, seconds vs minutes, watts vs kilowatts
- Timestamp errors: activities in future, overlapping time periods
```

**2. Reproduce calculation step-by-step**

Use the validation examples in each metric section:
- Start with the exact input values from the example
- Calculate each intermediate step
- Compare intermediate values, not just final results
- Identify exactly where your calculation diverges

**3. Check boundary conditions**

Many formulas use piecewise functions with discrete boundaries:
- TSS duration scaling: check if you're at 30min, 90min boundaries
- VDOT percent_max: check if you're at 5min, 15min, 30min, 90min boundaries
- Sleep duration scoring: check if you're at 5h, 6h, 7h, 8h boundaries
- Recovery level classification: check if you're at 50, 70, 85 boundaries

**4. Verify floating point precision**

```rust
// DON'T compare with exact equality
if calculated_value == expected_value { ... }  // ❌ WRONG

// DO compare with tolerance
if (calculated_value - expected_value).abs() < tolerance { ... }  // ✅ CORRECT

// Recommended tolerances:
// TSS: ±0.1
// CTL/ATL: ±0.5
// TSB: ±1.0
// VDOT: ±0.5
// Sleep quality: ±1.0
// Recovery score: ±2.0
```

**5. Eliminate common calculation errors**

See metric-specific sections below for detailed error patterns.

### Metric-specific Debugging

#### Debugging TSS Calculations

**symptom: TSS values differ by 5-20%**

```bash
# Diagnostic checklist:
1. Verify normalized power calculation (4th root method)
   - Are you using 30-second rolling average?
   - Did you apply the 4th power before averaging?
   - Formula: NP = ⁴√(avg(power³⁰ˢᵉᶜ⁴))

2. Check intensity factor precision
   - IF = NP / FTP
   - Verify FTP value is user's current FTP, not default

3. Verify duration is in hours
   - Common error: passing seconds instead of hours
   - TSS = (duration_hours × NP² × 100) / (FTP² × 3600)

4. Check for zero or negative FTP
   - FTP must be > 0
   - Default FTP values may not represent user's actual fitness
```

**example debugging session:**

```
User reports: TSS = 150, but pierre returns 138.9

Inputs:
  duration_s = 7200  (2 hours)
  normalized_power = 250W
  ftp = 300W

Debug steps:
  1. Convert duration: 7200 / 3600 = 2.0 hours ✓
  2. Calculate IF: 250 / 300 = 0.833 ✓
  3. Calculate TSS: 2.0 × 0.833² × 100 = 138.8889 ✓

Root cause: User was using duration in seconds directly
  Wrong: TSS = 7200 × 0.833² × 100 / (300² × 3600) = [calculation error]
  Fix: Convert seconds to hours first
```

#### Debugging CTL/ATL/TSB Calculations

**symptom: CTL/ATL drift over time, doesn't match pierre**

```bash
# Diagnostic checklist:
1. Verify EMA initialization (cold start problem)
   - First CTL = first TSS (not 0)
   - First ATL = first TSS (not 0)
   - Don't initialize with population averages

2. Check gap handling
   - Zero TSS days should be included in EMA
   - Formula: CTL_today = (CTL_yesterday × (1 - 1/42)) + (TSS_today × (1/42))
   - If activity missing: TSS_today = 0, but still update EMA

3. Verify day boundaries
   - Activities must be grouped by calendar day
   - Multiple activities per day: sum TSS before EMA
   - Timezone consistency: use user's local timezone

4. Check calculation order
   - Update CTL and ATL FIRST
   - Calculate TSB AFTER: TSB = CTL - ATL
   - Don't calculate TSB independently
```

**example debugging session:**

```
User reports: After 7 days, CTL = 55, but pierre shows 45

Day | TSS | User's CTL | Pierre's CTL | Issue
----|-----|------------|--------------|-------
1   | 100 | 100        | 100          | ✓ Match (initialization)
2   | 80  | 90         | 97.6         | ❌ Wrong formula
3   | 60  | 75         | 93.3         | ❌ Compounding error
...

Debug:
  Day 2 calculation:
    User: CTL = (100 + 80) / 2 = 90  ❌ Using simple average
    Pierre: CTL = 100 × (41/42) + 80 × (1/42) = 97.619  ✓ Using EMA

Root cause: User implementing simple moving average instead of exponential
Fix: Use EMA formula with decay factor (41/42 for CTL, 6/7 for ATL)
```

#### Debugging VDOT Calculations

**symptom: VDOT differs by 2-5 points**

```bash
# Diagnostic checklist:
1. Verify velocity calculation
   - velocity = (distance_m / time_s) × 60
   - Must be in meters per minute (not km/h or mph)
   - Valid range: [100, 500] m/min

2. Check percent_max for race duration
   - t < 5min: 0.97
   - 5min ≤ t < 15min: 0.99
   - 15min ≤ t < 30min: 1.00
   - 30min ≤ t < 90min: 0.98
   - t ≥ 90min: 0.95
   - Use time in MINUTES for this check

3. Verify VO2 calculation precision
   - vo2 = -4.60 + 0.182258×v + 0.000104×v²
   - Use full coefficient precision (not rounded values)
   - Don't round intermediate values

4. Check boundary conditions
   - At exactly t=15min: uses 1.00 (not 0.99)
   - At exactly t=30min: uses 0.98 (not 1.00)
   - Boundary behavior creates discrete jumps
```

**example debugging session:**

```
User reports: 10K in 37:30 → VDOT = 50.5, but pierre returns 52.4

Inputs:
  distance_m = 10000
  time_s = 2250

Debug steps:
  1. velocity = (10000 / 2250) × 60 = 266.67 m/min ✓
  2. vo2 = -4.60 + 0.182258×266.67 + 0.000104×266.67²
     User calculated: vo2 = 50.8 ❌
     Correct: vo2 = -4.60 + 48.602 + 7.396 = 51.398 ✓

  3. time_minutes = 2250 / 60 = 37.5 minutes
     37.5 minutes is in range [30, 90) → percent_max = 0.98 ✓

  4. VDOT = 51.398 / 0.98 = 52.4 ✓

Root cause: User calculated vo2 incorrectly (likely rounding error)
  User used: 0.18 instead of 0.182258 (coefficient precision loss)
  Fix: Use full precision coefficients
```

#### Debugging Sleep Quality Scoring

**symptom: sleep score differs by 10-20 points**

```bash
# Diagnostic checklist:
1. Verify component percentages sum correctly
   - deep% + rem% + light% + awake% should ≈ 100%
   - Tracker rounding may cause sum = 99% or 101%
   - Pierre accepts raw values (no normalization)

2. Check efficiency calculation
   - efficiency = (time_asleep_min / time_in_bed_min) × 100
   - time_asleep should ALWAYS be ≤ time_in_bed
   - If efficiency > 100%, data error

3. Verify awake penalty application
   - Only applied if awake% > 5%
   - penalty = (awake_percent - 5.0) × 2.0
   - Subtracted from stages_score (can result in negative, clamped to 0)

4. Check component weights
   - Duration: 40%
   - Stages: 35%
   - Efficiency: 25%
   - Weights must sum to 100%
```

**example debugging session:**

```
User reports: 7.5h sleep → score = 80, but pierre returns 88.1

Inputs:
  duration_hours = 7.5
  deep% = 18, rem% = 22, light% = 54, awake% = 6
  time_asleep = 450min, time_in_bed = 500min

Debug steps:
  1. Duration score: 7.0 ≤ 7.5 < 8.0
     score = 85 + 15×(7.5-7.0) = 92.5 ✓

  2. Stages score:
     deep_score = 70 + 30×(18-15)/5 = 88.0 ✓
     rem_score = 70 + 30×(22-20)/5 = 82.0 ✓
     awake_penalty = (6-5)×2 = 2.0 ✓

     User calculated: (88×0.4) + (82×0.4) + (54×0.2) = 78.8 ❌
     Correct: (88×0.4) + (82×0.4) + (54×0.2) - 2.0 = 76.8 ✓

  3. Efficiency: (450/500)×100 = 90% → score = 100 ✓

  4. Overall:
     User: (92.5×0.35) + (78.8×0.40) + (100×0.25) = 85.07 ❌
     Pierre: (92.5×0.35) + (76.8×0.40) + (100×0.25) = 88.1 ✓

Root cause: User forgot to subtract awake_penalty from stages_score
Fix: Apply penalty before weighting stages component
```

#### Debugging Recovery Score

**symptom: recovery score differs by 5-10 points**

```bash
# Diagnostic checklist:
1. Verify TSB normalization
   - Don't use raw TSB value [-30, +30]
   - Must normalize to [0, 100] using piecewise function
   - See TSB normalization formula (6 ranges)

2. Check HRV weighting
   - WITH HRV: 40% TSB, 40% sleep, 20% HRV
   - WITHOUT HRV: 50% TSB, 50% sleep
   - Same inputs produce different scores based on HRV availability

3. Verify HRV delta calculation
   - delta = current_rmssd - baseline_rmssd
   - Must use individual baseline (not population average)
   - Positive delta = good recovery
   - Negative delta = poor recovery

4. Check classification boundaries
   - excellent: ≥85
   - good: [70, 85)
   - fair: [50, 70)
   - poor: <50
```

**example debugging session:**

```
User reports: TSB=8, sleep=92, HRV=55 (weekly_avg=50) → score=85, but pierre returns 90

Debug steps:
  1. TSB normalization (5 ≤ 8 < 15):
     tsb_score = 80 + 10×(8-5)/10 = 83.0 ✓

  2. Sleep score (pass-through):
     sleep_score = 92.0 ✓

  3. HRV score:
     change_from_avg = 55 - 50 = +5.0ms
     +5.0 ≥ +5.0 threshold → HrvRecoveryStatus::Recovered → score = 100 ✓

  4. Recovery score:
     User calculated: (83×0.5) + (92×0.5) = 87.5 ❌
     Pierre: (83×0.4) + (92×0.4) + (100×0.2) = 90.0 ✓

Root cause: User applied 50/50 weights even though HRV available
  Wrong: 50% TSB, 50% sleep (HRV ignored)
  Correct: 40% TSB, 40% sleep, 20% HRV

Fix: When HRV available, use 40/40/20 split
```

### Common Platform-specific Issues

#### Javascript/Typescript Precision

```javascript
// JavaScript number is IEEE 754 double precision (same as Rust f64)
// But watch for integer overflow and precision loss

// ❌ WRONG: Integer math before conversion
const velocity = (distance_m / time_s) * 60;  // May lose precision

// ✅ CORRECT: Ensure floating point math
const velocity = (distance_m / time_s) * 60.0;

// ❌ WRONG: Using Math.pow for small exponents
const if_squared = Math.pow(intensity_factor, 2);

// ✅ CORRECT: Direct multiplication (faster, more precise)
const if_squared = intensity_factor * intensity_factor;
```

#### Python Precision

```python
# Python 3 uses arbitrary precision integers
# But watch for integer division vs float division

# ❌ WRONG: Integer division (Python 2 behavior)
velocity = (distance_m / time_s) * 60  # May truncate

# ✅ CORRECT: Ensure float division
velocity = float(distance_m) / float(time_s) * 60.0

# ❌ WRONG: Using ** operator with large values
normalized_power = (sum(powers) / len(powers)) ** 0.25

# ✅ CORRECT: Use explicit functions for clarity
import math
normalized_power = math.pow(sum(powers) / len(powers), 0.25)
```

#### REST API / JSON Precision

```bash
# JSON numbers are typically parsed as double precision
# But watch for serialization precision loss

# Server returns:
{"tss": 138.88888888888889}

# Client receives (depending on JSON parser):
{"tss": 138.89}  # Rounded by parser

# Solution: Accept small differences
tolerance = 0.1
assert abs(received_tss - expected_tss) < tolerance
```

### Data Quality Validation

Before debugging calculation logic, verify input data quality:

**activity data validation**

```bash
# Power data
- Valid range: [0, 2000] watts (pro cyclists max ~500W sustained)
- Check for dropout: consecutive zeros in power stream
- Check for spikes: isolated values >2× average power
- Negative values: impossible, indicates sensor error

# Heart rate data
- Valid range: [40, 220] bpm
- Check for dropout: consecutive zeros or flat lines
- Check resting HR: typically [40-80] bpm for athletes
- Max HR: age-based estimate 220-age (±10 bpm variance)

# Duration data
- Valid range: [0, 86400] seconds (max 24 hours per activity)
- Check for negative durations: clock sync issues
- Check for unrealistic durations: 48h "run" likely data error

# Distance data
- Valid range: depends on sport
- Running: typical pace [3-15] min/km
- Cycling: typical speed [15-45] km/h
- Check for GPS drift: indoor activities with high distance

# Sleep data
- Duration: typically [2-14] hours
- Efficiency: typically [65-98]%
- Stage percentages must sum to ~100%
- Check for unrealistic values: 0% deep sleep, 50% awake
```

**handling missing data**

```rust
// Pierre's approach to missing data:

// TSS calculation: reject if required fields missing
if power_data.is_empty() || ftp.is_none() {
    return Err(AppError::insufficient_data("Cannot calculate TSS"));
}

// CTL/ATL calculation: use zero for missing days
let tss_today = activities_today.map(|a| a.tss).sum_or(0.0);

// Sleep quality: partial calculation if stages missing
if stages.is_none() {
    // Calculate using duration and efficiency only (skip stages component)
    sleep_quality = (duration_score × 0.60) + (efficiency_score × 0.40)
}

// Recovery score: adaptive weighting based on availability
match (tsb, sleep_quality, hrv_data) {
    (Some(t), Some(s), Some(h)) => /* 40/40/20 */,
    (Some(t), Some(s), None)    => /* 50/50 */,
    _                           => Err(InsufficientData),
}
```

### When To Contact Support

Contact pierre support team if:

**1. Consistent calculation discrepancies >10%**
- You've verified input data quality
- You've reproduced calculation step-by-step
- Discrepancy persists across multiple activities
- Example: "All my TSS values are 15% higher than pierre's"

**2. Boundary condition bugs**
- Discrete jumps at boundaries larger than expected
- Example: "At exactly 15 minutes, my VDOT jumps by 5 points"

**3. Platform-specific precision issues**
- Same calculation produces different results on different platforms
- Example: "VDOT matches on desktop but differs by 3 on mobile"

**4. API response format changes**
- Response structure doesn't match documentation
- Missing fields in JSON response
- Unexpected error codes

**provide in support request:**
```
Subject: [METRIC] Calculation Discrepancy - [Brief Description]

Environment:
- Platform: [Web/Mobile/API]
- Language: [JavaScript/Python/Rust/etc]
- Pierre API version: [v1/v2/etc]

Input Data:
- [Full input values with types and units]
- Activity ID (if applicable): [123456789]

Expected Output:
- [Your calculated value with step-by-step calculation]

Actual Output:
- [Pierre's API response value]

Difference:
- Absolute: [X.XX units]
- Percentage: [X.X%]

Debugging Steps Taken:
- [List what you've already tried]
```

### Debugging Tools And Utilities

**command-line validation**

```bash
# Quick TSS calculation
echo "scale=2; (2.0 * 250 * 250 * 100) / (300 * 300 * 3600)" | bc

# Quick VDOT velocity check
python3 -c "print((10000 / 2250) * 60)"

# Quick EMA calculation
python3 -c "ctl_prev=100; tss=80; ctl_new=ctl_prev*(41/42)+tss*(1/42); print(ctl_new)"

# Compare with tolerance
python3 -c "import sys; abs(138.9 - 138.8) < 0.1 and sys.exit(0) or sys.exit(1)"
```

**spreadsheet validation**

Create a validation spreadsheet with columns:
```
| Input 1 | Input 2 | ... | Intermediate 1 | Intermediate 2 | Final Result | Pierre Result | Diff | Within Tolerance? |
```

Use formulas to calculate step-by-step and highlight discrepancies.

**automated testing**

```python
# Example pytest validation test
import pytest
from pierre_client import calculate_tss

def test_tss_validation_examples():
    """Test against documented validation examples."""

    # Example 1: Easy ride
    result = calculate_tss(
        normalized_power=180,
        duration_hours=2.0,
        ftp=300
    )
    assert abs(result - 72.0) < 0.1, f"Expected 72.0, got {result}"

    # Example 2: Threshold workout
    result = calculate_tss(
        normalized_power=250,
        duration_hours=2.0,
        ftp=300
    )
    assert abs(result - 138.9) < 0.1, f"Expected 138.9, got {result}"
```

---

## Limitations

### Model Assumptions
1. **linear progression**: assumes linear improvement, but adaptation is non-linear
2. **steady-state**: assumes consistent training environment
3. **population averages**: formulas may not fit individual physiology
4. **data quality**: sensor accuracy affects calculations

### Known Issues
- **HR metrics**: affected by caffeine, sleep, stress, heat, altitude
- **power metrics**: require proper FTP testing, affected by wind/drafting
- **pace metrics**: terrain and weather significantly affect running

### Prediction Accuracy
- **VDOT**: ±5% typical variance from actual race performance
- **TSB**: individual response to training load varies
- **patterns**: require sufficient data (minimum 3 weeks for trends)

---

## References

### Scientific Literature

1. **Banister, E.W.** (1991). Modeling elite athletic performance. Human Kinetics.

2. **Coggan, A. & Allen, H.** (2010). *Training and Racing with a Power Meter* (2nd ed.). VeloPress.

3. **Daniels, J.** (2013). *Daniels' Running Formula* (3rd ed.). Human Kinetics.

4. **Esteve-Lanao, J. Et al.** (2005). How do endurance runners train? *Med Sci Sports Exerc*, 37(3), 496-504.

5. **Halson, S.L.** (2014). Monitoring training load to understand fatigue. *Sports Medicine*, 44(Suppl 2), 139-147.

6. **Karvonen, M.J. Et al.** (2057). The effects of training on heart rate. *Ann Med Exp Biol Fenn*, 35(3), 307-315.

7. **Riegel, P.S.** (1981). Athletic records and human endurance. *American Scientist*, 69(3), 285-290.

8. **Tanaka, H. Et al.** (2001). Age-predicted maximal heart rate revisited. *J Am Coll Cardiol*, 37(1), 153-156.

9. **Gabbett, T.J.** (2016). The training-injury prevention paradox. *Br J Sports Med*, 50(5), 273-280.

10. **Seiler, S.** (2010). Training intensity distribution in endurance athletes. *Int J Sports Physiol Perform*, 5(3), 276-291.

11. **Draper, N.R. & Smith, H.** (1998). *Applied Regression Analysis* (3rd ed.). Wiley.

12. **Hirshkowitz, M. Et al.** (2015). National Sleep Foundation's sleep time duration recommendations: methodology and results summary. *Sleep Health*, 1(1), 40-43.

13. **Berry, R.B. Et al.** (2017). The AASM Manual for the Scoring of Sleep and Associated Events: Rules, Terminology and Technical Specifications, Version 2.4. *American Academy of Sleep Medicine*.

14. **Watson, N.F. Et al.** (2015). Recommended Amount of Sleep for a Healthy Adult: A Joint Consensus Statement of the American Academy of Sleep Medicine and Sleep Research Society. *Sleep*, 38(6), 843-844.

15. **Plews, D.J. Et al.** (2013). Training adaptation and heart rate variability in elite endurance athletes: opening the door to effective monitoring. *Int J Sports Physiol Perform*, 8(3), 286-293.

16. **Shaffer, F. & Ginsberg, J.P.** (2017). An Overview of Heart Rate Variability Metrics and Norms. *Front Public Health*, 5, 258.

---

## FAQ

**Q: why doesn't my prediction match race day?**
A: predictions are ranges (±5%), not exact. Affected by: weather, course, pacing, nutrition, taper, mental state.

**Q: can analytics work without HR or power?**
A: yes, but lower confidence. Pace-based TSS estimates used. Add HR/power for better accuracy.

**Q: how often update FTP/LTHR?**
A: FTP every 6-8 weeks, LTHR every 8-12 weeks, max HR annually.

**Q: why is TSB negative?**
A: normal during training. -30 to -10 = building fitness, -10 to 0 = productive, 0 to +10 = fresh/race ready.

**Q: how interpret confidence levels?**
A: high (15+ points, R²>0.7) = actionable; medium = guidance; low = directional; very low = insufficient data.

**Q: what happens if I have gaps in training?**
A: CTL/ATL naturally decay with zero TSS during gaps. This accurately models fitness loss during breaks.

**Q: how accurate are the VDOT predictions?**
A: verified 0.2-5.5% accuracy against jack daniels' published tables. Predictions assume proper training, taper, and race conditions.

**Q: what if my parameters are outside the valid ranges?**
A: validation will reject with specific error messages. Ranges are based on human physiology research (ACSM guidelines).

**Q: how much sleep do athletes need?**
A: 8-10 hours for optimal recovery (NSF guidelines). Minimum 7 hours for adults. <6 hours increases injury risk and impairs performance.

**Q: what's more important: sleep duration or quality?**
A: both matter. 8 hours of fragmented sleep (70% efficiency) scores lower than 7 hours of solid sleep (95% efficiency). Aim for both duration and quality.

**Q: why is my recovery score low despite good sleep?**
A: recovery combines TSB (40%), sleep (40%), HRV (20%). Negative TSB from high training load lowers score even with good sleep. This accurately reflects accumulated fatigue.

**Q: how does HRV affect recovery scoring?**
A: HRV (RMSSD) indicates autonomic nervous system recovery. +5ms above baseline = excellent, ±3ms = normal, -10ms = poor recovery. When unavailable, recovery uses 50% TSB + 50% sleep.

**Q: what providers support sleep tracking?**
A: fitbit, garmin, and whoop provide sleep data. Strava does not (returns `UnsupportedFeature` error). Use provider with sleep tracking for full recovery analysis.

---

## Glossary

**ATL**: acute training load (7-day EMA of TSS) - fatigue
**CTL**: chronic training load (42-day EMA of TSS) - fitness
**EMA**: exponential moving average - weighted average giving more weight to recent data
**FTP**: functional threshold power (1-hour max power)
**LTHR**: lactate threshold heart rate
**TSB**: training stress balance (CTL - ATL) - form
**TSS**: training stress score (duration × intensity²)
**VDOT**: VO2max adjusted for running economy (jack daniels)
**NP**: normalized power (4th root method)
**R²**: coefficient of determination (fit quality, 0-1)
**IF**: intensity factor (NP / FTP)
**RMSSD**: root mean square of successive differences (HRV metric, milliseconds)
**HRV**: heart rate variability (autonomic nervous system recovery indicator)
**NSF**: National Sleep Foundation (sleep duration guidelines)
**AASM**: American Academy of Sleep Medicine (sleep stage scoring standards)
**REM**: rapid eye movement sleep (cognitive recovery, memory consolidation)
**N3/deep sleep**: slow-wave sleep (physical recovery, growth hormone release)
**sleep efficiency**: (time asleep / time in bed) × 100 (fragmentation indicator)
**sleep quality**: combined score (40% duration, 35% stages, 25% efficiency)
**recovery score**: training readiness (40% TSB, 40% sleep, 20% HRV)

---

---

# Pierre Nutrition and USDA Integration Methodology

## What This Document Covers

This comprehensive guide explains the scientific methods, algorithms, and data integration behind pierre's nutrition system. It provides transparency into:

- **mathematical foundations**: BMR formulas, TDEE calculations, macronutrient distribution algorithms
- **usda fooddata central integration**: real food database access with 350,000+ foods
- **calculation methodologies**: step-by-step algorithms for daily nutrition needs
- **scientific references**: peer-reviewed research backing each recommendation
- **implementation details**: rust code architecture and api integration patterns
- **validation**: bounds checking, input validation, and safety mechanisms
- **testing**: comprehensive test coverage without external api dependencies

**target audience**: developers, nutritionists, coaches, and users seeking deep understanding of pierre's nutrition intelligence.

---

## ⚠️ Implementation Status: Production-Ready

**as of 2025-10-31**, pierre's nutrition system has been built from scratch using peer-reviewed sports nutrition science and usda fooddata central integration:

### What Was Implemented ✅
- **mifflin-st jeor bmr**: most accurate resting energy expenditure formula (±10% error vs indirect calorimetry)
- **tdee calculation**: activity-based multipliers from mcardle exercise physiology textbook
- **protein recommendations**: sport-specific ranges from phillips & van loon sports nutrition research
- **carbohydrate targeting**: burke et al. endurance athlete guidelines (3-12 g/kg based on activity)
- **fat calculations**: dri guidelines enforcement (20-35% of tdee)
- **nutrient timing**: kerksick et al. position stand on pre/post-workout nutrition
- **usda integration**: real food lookup via fooddata central api with mock support for testing
- **meal analysis**: multi-food calculations with accurate macro summations
- **input validation**: age (10-120), weight (0-300kg), height (0-300cm) bounds checking

### Verification ✅
- **39 algorithm tests**: bmr (4), tdee (5), protein (5), carbs (4), fat (3), complete nutrition (3), timing (3), edge cases (13)
- **formula accuracy**: mifflin-st jeor within 1 kcal of hand calculations
- **macro summing**: percentages always sum to 100% ±0.1%
- **usda integration**: tested with mock client (banana, chicken breast, oatmeal, salmon)
- **edge case handling**: negative inputs rejected, extreme values bounded, missing data handled
- **zero warnings**: strict clippy (pedantic + nursery) passes clean
- **1,188 total tests** passing including nutrition suite

**result**: pierre nutrition system is production-ready with scientifically-validated algorithms and usda fooddata integration.

---

## Architecture Overview

Pierre's nutrition system uses a **foundation modules** approach integrated with usda fooddata central:

```
┌─────────────────────────────────────────────┐
│   mcp/a2a protocol layer                    │
│   (src/protocols/universal/)                │
└──────────────────┬──────────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────────┐
│   nutrition tools (5 tools)                 │
│   (src/protocols/universal/handlers/)       │
└──────────────────┬──────────────────────────┘
                   │
    ┌──────────────┼──────────────────────────┐
    ▼              ▼                          ▼
┌─────────────┐ ┌──────────────┐       ┌──────────────┐
│ Nutrition   │ │ USDA Food    │       │ Meal         │
│ Calculator  │ │ Database     │       │ Analyzer     │
│             │ │              │       │              │
│ BMR/TDEE    │ │ 350k+ Foods  │       │ Multi-Food   │
│ Macros      │ │ Nutrients    │       │ Summation    │
│ Timing      │ │ API Client   │       │ Analysis     │
└─────────────┘ └──────────────┘       └──────────────┘
         NUTRITION FOUNDATION MODULE
```

### Nutrition Calculator Module

**`src/intelligence/nutrition_calculator.rs`** - core nutrition algorithms
- **mifflin-st jeor bmr** calculation with gender-specific constants
- **tdee calculation** with 5 activity level multipliers (1.2-1.9)
- **protein recommendations** based on activity level and training goal (0.8-2.2 g/kg)
- **carbohydrate targeting** optimized for endurance, strength, or weight loss (3-12 g/kg)
- **fat calculations** ensuring dri compliance (20-35% of tdee)
- **nutrient timing** for pre/post-workout optimization
- **protein distribution** across meals for muscle protein synthesis
- **input validation** with physiological bounds checking

### USDA Integration Module

**`src/external/usda_client.rs`** - fooddata central api client
- **async http client** with configurable timeout and rate limiting
- **food search** by name with pagination support
- **food details** retrieval with complete nutrient breakdown
- **mock client** for testing without api calls
- **error handling** with retry logic and graceful degradation
- **caching** with ttl for api response optimization

**`src/external/usda_client.rs`** - usda data structures (models re-exported via `src/external/mod.rs`)
- **food** representation with fdc_id and description
- **nutrient** structure with name, amount, unit
- **search results** with pagination metadata
- **type-safe** deserialization from usda json responses

---

## 1. Basal Metabolic Rate (BMR) Calculation

### Mifflin-St Jeor Formula (1990)

**most accurate formula** for resting energy expenditure (±10% error vs indirect calorimetry), superior to harris-benedict.

#### Formula

**for males:**
```
bmr = (10 × weight_kg) + (6.25 × height_cm) - (5 × age) + 5
```

**for females:**
```
bmr = (10 × weight_kg) + (6.25 × height_cm) - (5 × age) - 161
```

#### Implementation

`src/intelligence/nutrition_calculator.rs:169-207`

```rust
pub fn calculate_mifflin_st_jeor(
    weight_kg: f64,
    height_cm: f64,
    age: u32,
    gender: Gender,
    config: &BmrConfig,
) -> Result<f64, AppError> {
    // Validation
    if weight_kg <= 0.0 || weight_kg > 300.0 {
        return Err(AppError::invalid_input("Weight must be between 0 and 300 kg"));
    }
    if height_cm <= 0.0 || height_cm > 300.0 {
        return Err(AppError::invalid_input("Height must be between 0 and 300 cm"));
    }
    if !(10..=120).contains(&age) {
        return Err(AppError::invalid_input(
            "Age must be between 10 and 120 years (Mifflin-St Jeor formula validated for ages 10+)",
        ));
    }

    // Mifflin-St Jeor formula
    let weight_component = config.msj_weight_coef * weight_kg;         // 10.0
    let height_component = config.msj_height_coef * height_cm;         // 6.25
    let age_component = config.msj_age_coef * f64::from(age);          // -5.0

    let gender_constant = match gender {
        Gender::Male => config.msj_male_constant,      // +5
        Gender::Female => config.msj_female_constant,  // -161
    };

    let bmr = weight_component + height_component + age_component + gender_constant;

    // Minimum 1000 kcal/day safety check
    Ok(bmr.max(1000.0))
}
```

#### Example Calculations

**example 1: 30-year-old male, 75kg, 180cm**
```
bmr = (10 × 75) + (6.25 × 180) - (5 × 30) + 5
bmr = 750 + 1125 - 150 + 5
bmr = 1730 kcal/day
```

**example 2: 25-year-old female, 60kg, 165cm**
```
bmr = (10 × 60) + (6.25 × 165) - (5 × 25) - 161
bmr = 600 + 1031.25 - 125 - 161
bmr = 1345 kcal/day
```

#### Configuration

`src/config/intelligence_config.rs:423-438`

```rust
pub struct BmrConfig {
    pub use_mifflin_st_jeor: bool,     // true (recommended)
    pub use_harris_benedict: bool,     // false (legacy)
    pub msj_weight_coef: f64,          // 10.0
    pub msj_height_coef: f64,          // 6.25
    pub msj_age_coef: f64,             // -5.0
    pub msj_male_constant: f64,        // 5.0
    pub msj_female_constant: f64,      // -161.0
}
```

#### Scientific Reference

**mifflin, m.d., et al. (1990)**
*"a new predictive equation for resting energy expenditure in healthy individuals"*
American journal of clinical nutrition, 51(2), 241-247
Doi: 10.1093/ajcn/51.2.241

**key findings:**
- validated on 498 healthy subjects (247 males, 251 females)
- accuracy: ±10% error vs indirect calorimetry
- superior to harris-benedict formula (1919) by 5%
- accounts for modern body composition changes

---

## 2. Total Daily Energy Expenditure (TDEE)

### Activity Factor Multipliers

**tdee** = bmr × activity factor

#### Activity Levels

Based on mcardle, katch & katch exercise physiology (2010):

| activity level | description | multiplier | example activities |
|----------------|-------------|------------|-------------------|
| sedentary | little/no exercise | 1.2 | desk job, no workouts |
| lightly active | 1-3 days/week | 1.375 | walking, light yoga |
| moderately active | 3-5 days/week | 1.55 | running 3×/week, cycling |
| very active | 6-7 days/week | 1.725 | daily training, athlete |
| extra active | 2×/day hard training | 1.9 | professional athlete |

#### Implementation

`src/intelligence/nutrition_calculator.rs:209-245`

```rust
pub fn calculate_tdee(
    bmr: f64,
    activity_level: ActivityLevel,
    config: &ActivityFactorsConfig,
) -> Result<f64, AppError> {
    if bmr < 1000.0 || bmr > 5000.0 {
        return Err(AppError::invalid_input("BMR must be between 1000 and 5000"));
    }

    let activity_factor = match activity_level {
        ActivityLevel::Sedentary => config.sedentary,              // 1.2
        ActivityLevel::LightlyActive => config.lightly_active,     // 1.375
        ActivityLevel::ModeratelyActive => config.moderately_active, // 1.55
        ActivityLevel::VeryActive => config.very_active,           // 1.725
        ActivityLevel::ExtraActive => config.extra_active,         // 1.9
    };

    Ok(bmr * activity_factor)
}
```

#### Example Calculations

**sedentary: bmr 1500 × 1.2 = 1800 kcal/day**
**very active: bmr 1500 × 1.725 = 2587 kcal/day**

#### Configuration

`src/config/intelligence_config.rs:444-455`

```rust
pub struct ActivityFactorsConfig {
    pub sedentary: f64,          // 1.2
    pub lightly_active: f64,     // 1.375
    pub moderately_active: f64,  // 1.55
    pub very_active: f64,        // 1.725
    pub extra_active: f64,       // 1.9
}
```

---

## 3. Macronutrient Recommendations

### Protein Needs

#### Recommendations by Activity and Goal

Based on phillips & van loon (2011) doi: 10.1080/02640414.2011.619204:

| activity level | training goal | protein (g/kg) | rationale |
|----------------|---------------|----------------|-----------|
| sedentary | any | 0.8 | dri minimum |
| lightly/moderately active | maintenance | 1.3 | active lifestyle support |
| very/extra active | endurance | 2.0 | glycogen sparing, recovery |
| very/extra active | strength/muscle gain | 2.2 | muscle protein synthesis |
| any | weight loss | 1.8 | muscle preservation |

#### Implementation

`src/intelligence/nutrition_calculator.rs:274-313`

```rust
pub fn calculate_protein_needs(
    weight_kg: f64,
    activity_level: ActivityLevel,
    training_goal: TrainingGoal,
    config: &MacronutrientConfig,
) -> Result<f64, AppError> {
    let protein_g_per_kg = match (activity_level, training_goal) {
        // Sedentary baseline (DRI minimum)
        (ActivityLevel::Sedentary, _) => config.protein_min_g_per_kg,  // 0.8

        // Moderate activity
        (ActivityLevel::LightlyActive | ActivityLevel::ModeratelyActive, _) => {
            config.protein_moderate_g_per_kg  // 1.3
        }

        // Athletic - goal-specific
        (ActivityLevel::VeryActive | ActivityLevel::ExtraActive, TrainingGoal::EndurancePerformance) => {
            config.protein_endurance_max_g_per_kg  // 2.0
        }
        (ActivityLevel::VeryActive | ActivityLevel::ExtraActive,
         TrainingGoal::StrengthPerformance | TrainingGoal::MuscleGain) => {
            config.protein_strength_max_g_per_kg  // 2.2
        }

        // Weight loss: higher protein for muscle preservation
        (_, TrainingGoal::WeightLoss) => config.protein_athlete_g_per_kg,  // 1.8

        // Default for very/extra active
        (ActivityLevel::VeryActive | ActivityLevel::ExtraActive, _) => {
            config.protein_athlete_g_per_kg  // 1.8
        }
    };

    Ok(weight_kg * protein_g_per_kg)
}
```

### Carbohydrate Needs

#### Recommendations by Activity and Goal

Based on burke et al. (2011) doi: 10.1080/02640414.2011.585473:

| activity level | training goal | carbs (g/kg) | rationale |
|----------------|---------------|--------------|-----------|
| sedentary/light | any | 3.0 | brain function minimum |
| moderate | maintenance | 6.0 | glycogen replenishment |
| very/extra active | muscle gain | 7.2 (6.0 × 1.2) | anabolic support |
| any | endurance | 10.0 | high glycogen demand |

#### Implementation

`src/intelligence/nutrition_calculator.rs:336-365`

```rust
pub fn calculate_carb_needs(
    weight_kg: f64,
    activity_level: ActivityLevel,
    training_goal: TrainingGoal,
    config: &MacronutrientConfig,
) -> Result<f64, AppError> {
    let carbs_g_per_kg = match (activity_level, training_goal) {
        // Low activity
        (ActivityLevel::Sedentary | ActivityLevel::LightlyActive, _) => {
            config.carbs_low_activity_g_per_kg  // 3.0
        }

        // Endurance athletes need high carbs
        (_, TrainingGoal::EndurancePerformance) => {
            config.carbs_high_endurance_g_per_kg  // 10.0
        }

        // Moderate activity
        (ActivityLevel::ModeratelyActive, _) => {
            config.carbs_moderate_activity_g_per_kg  // 6.0
        }

        // Very/extra active (non-endurance) - slightly higher
        (ActivityLevel::VeryActive | ActivityLevel::ExtraActive, _) => {
            config.carbs_moderate_activity_g_per_kg * 1.2  // 7.2
        }
    };

    Ok(weight_kg * carbs_g_per_kg)
}
```

### Fat Needs

#### DRI Guidelines

Dietary reference intakes (institute of medicine):
- **minimum**: 20% of tdee (hormone production, vitamin absorption)
- **optimal**: 25-30% of tdee (satiety, performance)
- **maximum**: 35% of tdee (avoid excess)

#### Implementation

`src/intelligence/nutrition_calculator.rs:392-435`

```rust
pub fn calculate_fat_needs(
    tdee: f64,
    protein_g: f64,
    carbs_g: f64,
    training_goal: TrainingGoal,
    config: &MacronutrientConfig,
) -> Result<f64, AppError> {
    // Calculate calories from protein and carbs
    let protein_kcal = protein_g * 4.0;
    let carbs_kcal = carbs_g * 4.0;
    let fat_kcal_available = tdee - protein_kcal - carbs_kcal;

    // Goal-specific fat targeting
    let target_fat_percent = match training_goal {
        TrainingGoal::WeightLoss => config.fat_min_percent_tdee,  // 20%
        TrainingGoal::MuscleGain | TrainingGoal::StrengthPerformance => {
            config.fat_optimal_percent_tdee - 2.5  // 25%
        }
        TrainingGoal::EndurancePerformance | TrainingGoal::Maintenance => {
            config.fat_optimal_percent_tdee  // 27.5%
        }
    };

    // Take maximum of remainder or target percentage
    let fat_from_remainder = fat_kcal_available / 9.0;
    let fat_from_target = (tdee * target_fat_percent / 100.0) / 9.0;

    let fat_g = fat_from_remainder.max(fat_from_target);

    // Enforce DRI bounds (20-35% of TDEE)
    let min_fat = (tdee * config.fat_min_percent_tdee / 100.0) / 9.0;
    let max_fat = (tdee * config.fat_max_percent_tdee / 100.0) / 9.0;

    Ok(fat_g.clamp(min_fat, max_fat))
}
```

### Configuration

`src/config/intelligence_config.rs:464-487`

```rust
pub struct MacronutrientConfig {
    // Protein ranges (g/kg)
    pub protein_min_g_per_kg: f64,          // 0.8
    pub protein_moderate_g_per_kg: f64,     // 1.3
    pub protein_athlete_g_per_kg: f64,      // 1.8
    pub protein_endurance_max_g_per_kg: f64, // 2.0
    pub protein_strength_max_g_per_kg: f64, // 2.2

    // Carbohydrate ranges (g/kg)
    pub carbs_low_activity_g_per_kg: f64,      // 3.0
    pub carbs_moderate_activity_g_per_kg: f64, // 6.0
    pub carbs_high_endurance_g_per_kg: f64,    // 10.0

    // Fat percentages (% of TDEE)
    pub fat_min_percent_tdee: f64,     // 20%
    pub fat_max_percent_tdee: f64,     // 35%
    pub fat_optimal_percent_tdee: f64, // 27.5%
}
```

---

## 4. Nutrient Timing

### Pre-workout Nutrition

Based on kerksick et al. (2017) doi: 10.1186/s12970-017-0189-4:

**timing**: 1-3 hours before workout
**carbohydrates**: 0.5-1.0 g/kg (intensity-dependent)
- low intensity: 0.375 g/kg (0.5 × 0.75)
- moderate intensity: 0.75 g/kg
- high intensity: 0.975 g/kg (1.3 × 0.75)

### Post-workout Nutrition

**timing**: within 2 hours (flexible - total daily intake matters most)
**protein**: 20-40g (muscle protein synthesis threshold)
**carbohydrates**: 0.8-1.2 g/kg (glycogen restoration)

### Protein Distribution

**optimal**: 4 meals/day with even protein distribution
**minimum**: 3 meals/day
**rationale**: muscle protein synthesis maximized with 0.4-0.5 g/kg per meal

#### Implementation

`src/intelligence/nutrition_calculator.rs:539-606`

```rust
pub fn calculate_nutrient_timing(
    weight_kg: f64,
    daily_protein_g: f64,
    workout_intensity: WorkoutIntensity,
    config: &NutrientTimingConfig,
) -> Result<NutrientTimingPlan, AppError> {
    // Pre-workout carbs based on intensity
    let pre_workout_carbs = match workout_intensity {
        WorkoutIntensity::Low => weight_kg * config.pre_workout_carbs_g_per_kg * 0.5,
        WorkoutIntensity::Moderate => weight_kg * config.pre_workout_carbs_g_per_kg,
        WorkoutIntensity::High => weight_kg * config.pre_workout_carbs_g_per_kg * 1.3,
    };

    // Post-workout protein (20-40g optimal range)
    let post_workout_protein = config.post_workout_protein_g_min
        .max((daily_protein_g / 5.0).min(config.post_workout_protein_g_max));

    // Post-workout carbs
    let post_workout_carbs = weight_kg * config.post_workout_carbs_g_per_kg;

    // Protein distribution across day
    let meals_per_day = config.protein_meals_per_day_optimal;
    let protein_per_meal = daily_protein_g / f64::from(meals_per_day);

    Ok(NutrientTimingPlan {
        pre_workout: PreWorkoutNutrition {
            carbs_g: pre_workout_carbs,
            timing_hours_before: config.pre_workout_window_hours,
            recommendations: vec![
                format!("Consume {pre_workout_carbs:.0}g carbs 1-3 hours before workout"),
                "Focus on easily digestible carbs (banana, oatmeal, toast)".to_string(),
            ],
        },
        post_workout: PostWorkoutNutrition {
            protein_g: post_workout_protein,
            carbs_g: post_workout_carbs,
            timing_hours_after: config.post_workout_window_hours,
            recommendations: vec![
                format!("Consume {post_workout_protein:.0}g protein + {post_workout_carbs:.0}g carbs within 2 hours"),
                "Window is flexible - total daily intake matters most".to_string(),
            ],
        },
        daily_protein_distribution: ProteinDistribution {
            meals_per_day,
            protein_per_meal_g: protein_per_meal,
            strategy: format!(
                "Distribute {daily_protein_g:.0}g protein across {meals_per_day} meals (~{protein_per_meal:.0}g each)"
            ),
        },
    })
}
```

### Configuration

`src/config/intelligence_config.rs:495-512`

```rust
pub struct NutrientTimingConfig {
    pub pre_workout_window_hours: f64,          // 2.0
    pub post_workout_window_hours: f64,         // 2.0
    pub pre_workout_carbs_g_per_kg: f64,        // 0.75
    pub post_workout_protein_g_min: f64,        // 20.0
    pub post_workout_protein_g_max: f64,        // 40.0
    pub post_workout_carbs_g_per_kg: f64,       // 1.0
    pub protein_meals_per_day_min: u8,          // 3
    pub protein_meals_per_day_optimal: u8,      // 4
}
```

### Recipe Meal Timing Macro Distributions

The recipe system (`src/intelligence/recipes/`) uses percentage-based macronutrient distributions that adjust based on training context. These distributions are applied when generating recipe constraints for LLM clients or validating recipes.

#### Macro Distribution by Meal Timing

| Meal Timing    | Protein | Carbs | Fat  | Rationale                                      |
|----------------|---------|-------|------|------------------------------------------------|
| Pre-training   | 20%     | 55%   | 25%  | Maximize glycogen, minimize GI distress        |
| Post-training  | 30%     | 45%   | 25%  | Optimize MPS + glycogen replenishment          |
| Rest day       | 30%     | 35%   | 35%  | Lower glycogen needs, carb periodization       |
| General        | 25%     | 45%   | 30%  | Balanced for non-specific meals                |

#### Scientific Justification

**Pre-training (20% protein, 55% carbs, 25% fat)**

High carbohydrate availability maximizes muscle glycogen stores for energy. The ISSN recommends 1-4 g/kg of high-glycemic carbohydrates 1-4 hours before exercise for glycogen optimization. Lower fat (25%) aids gastric emptying, reducing gastrointestinal distress during exercise.

*Reference: Kerksick CM, Arent S, Schoenfeld BJ, et al. (2017) "International Society of Sports Nutrition Position Stand: Nutrient Timing" Journal of the International Society of Sports Nutrition 14:33. DOI: 10.1186/s12970-017-0189-4*

**Post-training (30% protein, 45% carbs, 25% fat)**

Elevated protein intake (0.25-0.4 g/kg or approximately 20-40g) within 2 hours post-exercise maximizes muscle protein synthesis (MPS). Moderate carbohydrates (0.8-1.2 g/kg) accelerate glycogen resynthesis, especially when combined with protein. The 30% protein proportion ensures adequate leucine threshold (~2.5-3g) for MPS activation.

*Reference: Jäger R, Kerksick CM, Campbell BI, et al. (2017) "International Society of Sports Nutrition Position Stand: Protein and Exercise" Journal of the International Society of Sports Nutrition 14:20. DOI: 10.1186/s12970-017-0177-8*

**Rest day (30% protein, 35% carbs, 35% fat)**

Carbohydrate periodization principles advocate for reduced carbohydrate intake on non-training days when glycogen demands are lower. Training with reduced glycogen availability (the "train-low" approach) stimulates mitochondrial biogenesis and improves oxidative capacity. Higher fat (35%) compensates for reduced carbohydrate calories while maintaining satiety through slower gastric emptying.

*Reference: Impey SG, Hearris MA, Hammond KM, et al. (2018) "Fuel for the Work Required: A Theoretical Framework for Carbohydrate Periodization and the Glycogen Threshold Hypothesis" Sports Medicine 48(5):1031-1048. DOI: 10.1007/s40279-018-0867-7*

#### Implementation

`src/intelligence/recipes/models.rs:33-49`

```rust
impl MealTiming {
    /// Get recommended macro distribution percentages for this timing
    ///
    /// Returns (`protein_pct`, `carbs_pct`, `fat_pct`) tuple that sums to 100
    pub const fn macro_distribution(&self) -> (u8, u8, u8) {
        match self {
            // Pre-training: prioritize carbs for energy
            Self::PreTraining => (20, 55, 25),
            // Post-training: prioritize protein for recovery
            Self::PostTraining => (30, 45, 25),
            // Rest day: balanced with lower carbs
            Self::RestDay => (30, 35, 35),
            // General: balanced distribution
            Self::General => (25, 45, 30),
        }
    }
}
```

### TDEE-Based Recipe Calorie Calculation

When generating recipe constraints via `get_recipe_constraints`, the system calculates target calories using a priority-based approach:

1. **Explicit calories** - If provided in the request, uses the exact value
2. **TDEE-based** - When user's TDEE is provided, calculates calories as a proportion of daily energy
3. **Fallback defaults** - Uses research-based defaults when no TDEE is available

#### TDEE Proportions by Meal Timing

| Meal Timing    | TDEE Proportion | Example (2500 kcal TDEE) | Rationale                                      |
|----------------|-----------------|--------------------------|------------------------------------------------|
| Pre-training   | 17.5%           | 438 kcal                 | Moderate meal to fuel workout without GI stress |
| Post-training  | 27.5%           | 688 kcal                 | Largest meal for recovery and glycogen restoration |
| Rest day       | 25.0%           | 625 kcal                 | Standard meal proportion for recovery days     |
| General        | 25.0%           | 625 kcal                 | Balanced default for non-training meals        |

#### Fallback Calorie Values

When TDEE is not provided, the system uses these scientifically-informed defaults:

| Meal Timing    | Fallback Calories | Rationale                                      |
|----------------|-------------------|------------------------------------------------|
| Pre-training   | 400 kcal          | Light meal suitable for pre-workout fueling    |
| Post-training  | 600 kcal          | Larger meal for optimal recovery nutrition     |
| Rest day       | 500 kcal          | Moderate meal for non-training days            |
| General        | 500 kcal          | Balanced default for general meal planning     |

#### Scientific Justification

**Post-training as largest meal (27.5% of TDEE)**

The post-workout period represents the optimal window for nutrient partitioning. Elevated muscle glycogen synthase activity and enhanced insulin sensitivity make this the ideal time for higher calorie intake. The 27.5% proportion ensures adequate calories for both glycogen restoration (requiring 0.8-1.2 g/kg carbohydrates) and muscle protein synthesis (requiring 20-40g protein).

*Reference: Ivy JL, Katz AL, Cutler CL, et al. (1988) "Muscle glycogen synthesis after exercise: effect of time of carbohydrate ingestion" Journal of Applied Physiology 64(4):1480-1485. DOI: 10.1152/jappl.1988.64.4.1480*

**Pre-training as smaller meal (17.5% of TDEE)**

Lower calorie intake pre-workout minimizes gastrointestinal distress while still providing adequate fuel. The ISSN recommends consuming carbohydrates 1-4 hours before exercise, with smaller meals closer to workout time. The 17.5% proportion provides sufficient energy without compromising exercise performance or comfort.

#### Configuration

`src/config/intelligence_config.rs`

```rust
/// Meal TDEE proportion configuration based on ISSN research
pub struct MealTdeeProportionsConfig {
    pub pre_training: f64,    // 0.175 (17.5% of TDEE)
    pub post_training: f64,   // 0.275 (27.5% of TDEE)
    pub rest_day: f64,        // 0.25 (25% of TDEE)
    pub general: f64,         // 0.25 (25% of TDEE)
    pub fallback_calories: MealFallbackCaloriesConfig,
}

/// Fallback calorie values when TDEE is not available
pub struct MealFallbackCaloriesConfig {
    pub pre_training: f64,   // 400.0 kcal
    pub post_training: f64,  // 600.0 kcal
    pub rest_day: f64,       // 500.0 kcal
    pub general: f64,        // 500.0 kcal
}
```

#### API Response Fields

When TDEE is provided, `get_recipe_constraints` includes additional fields:

```json
{
  "calories": 688,
  "tdee_based": true,
  "tdee": 2500,
  "tdee_proportion": 0.275
}
```

When TDEE is not provided, `tdee_based` is `false` and fallback calories are used.

---

## 5. USDA FoodData Central Integration

### API Overview

**usda fooddata central** provides access to:
- **350,000+ foods** in the database
- **comprehensive nutrients** (protein, carbs, fat, vitamins, minerals)
- **branded foods** with manufacturer data
- **foundation foods** with detailed nutrient profiles
- **sr legacy foods** from usda nutrient database

### Client Implementation

`src/external/usda_client.rs:1-233`

#### Real Client (Production)

```rust
pub struct UsdaClient {
    client: reqwest::Client,
    config: UsdaClientConfig,
}

impl UsdaClient {
    pub async fn search_foods(&self, query: &str, page_size: usize) -> Result<SearchResult> {
        let url = format!("{}/foods/search", self.config.base_url);

        let response = self.client
            .get(&url)
            .query(&[
                ("query", query),
                ("pageSize", &page_size.to_string()),
                ("api_key", &self.config.api_key),
            ])
            .timeout(Duration::from_secs(self.config.timeout_secs))
            .send()
            .await?;

        response.json().await
    }

    pub async fn get_food_details(&self, fdc_id: u64) -> Result<FoodDetails> {
        let url = format!("{}/food/{}", self.config.base_url, fdc_id);

        let response = self.client
            .get(&url)
            .query(&[("api_key", &self.config.api_key)])
            .timeout(Duration::from_secs(self.config.timeout_secs))
            .send()
            .await?;

        response.json().await
    }
}
```

#### Mock Client (Testing)

```rust
pub struct MockUsdaClient;

impl MockUsdaClient {
    pub fn new() -> Self {
        Self
    }

    pub fn search_foods(&self, query: &str, _page_size: usize) -> Result<SearchResult> {
        // Return realistic mock data based on query
        let foods = match query.to_lowercase().as_str() {
            q if q.contains("chicken") => vec![
                Food {
                    fdc_id: 171477,
                    description: "Chicken breast, skinless, boneless, raw".to_string(),
                },
            ],
            q if q.contains("banana") => vec![
                Food {
                    fdc_id: 173944,
                    description: "Banana, raw".to_string(),
                },
            ],
            // ... more mock foods
        };

        Ok(SearchResult {
            foods,
            total_hits: foods.len(),
            current_page: 1,
            total_pages: 1,
        })
    }

    pub fn get_food_details(&self, fdc_id: u64) -> Result<FoodDetails> {
        // Return complete nutrient breakdown
        match fdc_id {
            171477 => Ok(FoodDetails {  // Chicken breast
                fdc_id: 171477,
                description: "Chicken breast, skinless, boneless, raw".to_string(),
                food_nutrients: vec![
                    Nutrient {
                        nutrient_name: "Protein".to_string(),
                        amount: 23.09,
                        unit: "g".to_string(),
                    },
                    Nutrient {
                        nutrient_name: "Energy".to_string(),
                        amount: 120.0,
                        unit: "kcal".to_string(),
                    },
                    // ... more nutrients
                ],
            }),
            // ... more mock foods
        }
    }
}
```

### Configuration

`src/config/intelligence_config.rs:514-522`

```rust
pub struct UsdaApiConfig {
    pub base_url: String,              // "https://api.nal.usda.gov/fdc/v1"
    pub timeout_secs: u64,             // 10
    pub cache_ttl_hours: u64,          // 24
    pub max_cache_items: usize,        // 1000
    pub rate_limit_per_minute: u32,    // 30
}
```

---

## 6. MCP Tool Integration

Pierre exposes 5 nutrition tools via mcp protocol:

### calculate_daily_nutrition

**calculates complete daily nutrition requirements**

```json
{
  "name": "calculate_daily_nutrition",
  "description": "Calculate complete daily nutrition requirements (BMR, TDEE, macros)",
  "inputSchema": {
    "type": "object",
    "properties": {
      "weight_kg": { "type": "number", "description": "Body weight in kg" },
      "height_cm": { "type": "number", "description": "Height in cm" },
      "age": { "type": "integer", "description": "Age in years" },
      "gender": { "type": "string", "enum": ["male", "female"] },
      "activity_level": { "type": "string", "enum": ["sedentary", "lightly_active", "moderately_active", "very_active", "extra_active"] },
      "training_goal": { "type": "string", "enum": ["maintenance", "weight_loss", "muscle_gain", "endurance_performance"] }
    },
    "required": ["weight_kg", "height_cm", "age", "gender", "activity_level", "training_goal"]
  }
}
```

**example response:**
```json
{
  "bmr": 1730,
  "tdee": 2682,
  "protein_g": 135,
  "carbs_g": 402,
  "fat_g": 82,
  "macro_percentages": {
    "protein_percent": 20.1,
    "carbs_percent": 60.0,
    "fat_percent": 27.5
  },
  "method": "Mifflin-St Jeor + Activity Factor"
}
```

### calculate_nutrient_timing

**calculates pre/post-workout nutrition and daily protein distribution**

```json
{
  "name": "calculate_nutrient_timing",
  "inputSchema": {
    "properties": {
      "weight_kg": { "type": "number" },
      "daily_protein_g": { "type": "number" },
      "workout_intensity": { "type": "string", "enum": ["low", "moderate", "high"] }
    }
  }
}
```

### search_foods (USDA)

**searches usda fooddata central database**

```json
{
  "name": "search_foods",
  "inputSchema": {
    "properties": {
      "query": { "type": "string", "description": "Food name to search" },
      "page_size": { "type": "integer", "default": 10 },
      "use_mock": { "type": "boolean", "default": false }
    }
  }
}
```

### get_food_details (USDA)

**retrieves complete nutrient breakdown for a food**

```json
{
  "name": "get_food_details",
  "inputSchema": {
    "properties": {
      "fdc_id": { "type": "integer", "description": "USDA FDC ID" },
      "use_mock": { "type": "boolean", "default": false }
    }
  }
}
```

### analyze_meal_nutrition

**analyzes complete meal with multiple foods**

```json
{
  "name": "analyze_meal_nutrition",
  "inputSchema": {
    "properties": {
      "meal_foods": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "fdc_id": { "type": "integer" },
            "grams": { "type": "number" }
          }
        }
      },
      "use_mock": { "type": "boolean", "default": false }
    }
  }
}
```

**example request:**
```json
{
  "meal_foods": [
    { "fdc_id": 171477, "grams": 150 },  // chicken breast
    { "fdc_id": 170379, "grams": 200 },  // brown rice
    { "fdc_id": 170417, "grams": 100 }   // broccoli
  ]
}
```

**example response:**
```json
{
  "total_calories": 456,
  "total_protein_g": 42.5,
  "total_carbs_g": 62.3,
  "total_fat_g": 5.1,
  "food_details": [
    { "fdc_id": 171477, "description": "Chicken breast", "grams": 150 },
    { "fdc_id": 170379, "description": "Brown rice", "grams": 200 },
    { "fdc_id": 170417, "description": "Broccoli", "grams": 100 }
  ]
}
```

---

## 7. Testing and Verification

### Comprehensive Test Suite

**39 algorithm tests** covering all nutrition calculations:

#### Test Categories

**bmr calculations (4 tests)**
- male/female typical cases
- minimum bmr enforcement (1000 kcal floor)
- large athlete scenarios

**tdee calculations (5 tests)**
- all 5 activity levels (1.2-1.9 multipliers)
- sedentary through extra active

**protein needs (5 tests)**
- all 4 training goals
- activity level scaling
- weight proportionality

**carbohydrate needs (4 tests)**
- endurance high-carb requirements
- weight loss lower-carb approach
- muscle gain optimization
- activity level scaling

**fat calculations (3 tests)**
- balanced macro scenarios
- minimum fat enforcement (20% tdee)
- high tdee edge cases

**complete daily nutrition (3 tests)**
- male maintenance profile
- female weight loss profile
- athlete endurance profile

**nutrient timing (3 tests)**
- high/moderate/low workout intensities
- pre/post-workout calculations
- protein distribution strategies

**edge cases & validation (13 tests)**
- negative/zero weight rejection
- invalid height rejection
- age bounds (10-120 years)
- extreme tdee scenarios
- macro percentage summing (always 100%)
- all intensity levels
- invalid inputs handling

### Test Execution

`tests/nutrition_comprehensive_test.rs:1-902`

```bash
# run nutrition tests
cargo test --test nutrition_comprehensive_test

# output
test result: ok. 39 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### Formula Verification

**mifflin-st jeor accuracy**:
- 30yo male, 75kg, 180cm: calculated 1730 kcal (matches hand calculation)
- 25yo female, 60kg, 165cm: calculated 1345 kcal (matches hand calculation)

**macro percentages**:
- all scenarios tested sum to 100.0% ±0.1%

**usda integration**:
- mock client tested with banana (173944), chicken (171477), oatmeal (173904), salmon (175168)
- nutrient calculations verified against usda data

---

## 8. Configuration and Customization

All nutrition parameters are configurable via `src/config/intelligence_config.rs`:

```rust
pub struct NutritionConfig {
    pub bmr: BmrConfig,
    pub activity_factors: ActivityFactorsConfig,
    pub macronutrients: MacronutrientConfig,
    pub nutrient_timing: NutrientTimingConfig,
    pub usda_api: UsdaApiConfig,
}
```

### Environment Variables

```bash
# USDA API (optional - mock client available for testing)
export USDA_API_KEY=your_api_key_here

# Server configuration
export HTTP_PORT=8081
export DATABASE_URL=sqlite:./data/users.db
```

### Dependency Injection

All calculation functions accept configuration structs:
- **testable**: inject mock configs for testing
- **flexible**: change thresholds without code changes
- **documented**: configuration structs have inline documentation

---

## 9. Scientific References

### BMR and Energy Expenditure

1. **mifflin, m.d., et al. (1990)**
   - "a new predictive equation for resting energy expenditure"
   - american journal of clinical nutrition, 51(2), 241-247
   - doi: 10.1093/ajcn/51.2.241

2. **mcardle, w.d., katch, f.i., & katch, v.l. (2010)**
   - exercise physiology: nutrition, energy, and human performance
   - lippincott williams & wilkins

### Protein Recommendations

3. **phillips, s.m., & van loon, l.j. (2011)**
   - "dietary protein for athletes: from requirements to optimum adaptation"
   - journal of sports sciences, 29(sup1), s29-s38
   - doi: 10.1080/02640414.2011.619204

4. **morton, r.w., et al. (2018)**
   - "a systematic review, meta-analysis and meta-regression of protein intake"
   - british journal of sports medicine, 52(6), 376-384
   - doi: 10.1136/bjsports-2017-097608

### Carbohydrate Recommendations

5. **burke, l.m., et al. (2011)**
   - "carbohydrates for training and competition"
   - journal of sports sciences, 29(sup1), s17-s27
   - doi: 10.1080/02640414.2011.585473

### Nutrient Timing

6. **kerksick, c.m., et al. (2017)**
   - "international society of sports nutrition position stand: nutrient timing"
   - journal of the international society of sports nutrition, 14(1), 33
   - doi: 10.1186/s12970-017-0189-4

7. **aragon, a.a., & schoenfeld, b.j. (2013)**
   - "nutrient timing revisited: is there a post-exercise anabolic window?"
   - journal of the international society of sports nutrition, 10(1), 5
   - doi: 10.1186/1550-2783-10-5

### Fat Recommendations

8. **institute of medicine (2005)**
   - dietary reference intakes for energy, carbohydrate, fiber, fat, fatty acids, cholesterol, protein, and amino acids
   - national academies press

---

## 10. Implementation Roadmap

### Phase 1: Foundation (Complete ✅)
- [x] bmr calculation (mifflin-st jeor)
- [x] tdee calculation with activity factors
- [x] protein recommendations by activity/goal
- [x] carbohydrate targeting
- [x] fat calculations with dri compliance
- [x] nutrient timing algorithms
- [x] input validation and bounds checking
- [x] 39 comprehensive algorithm tests

### Phase 2: USDA Integration (Complete ✅)
- [x] usda client with async api calls
- [x] food search functionality
- [x] food details retrieval
- [x] mock client for testing
- [x] meal analysis with multi-food support
- [x] nutrient summation calculations

### Phase 3: MCP Tools (Complete ✅)
- [x] calculate_daily_nutrition tool
- [x] calculate_nutrient_timing tool
- [x] search_foods tool
- [x] get_food_details tool
- [x] analyze_meal_nutrition tool

### Phase 4: Future Enhancements
- [ ] meal planning tool (weekly meal generation)
- [ ] recipe nutrition analysis
- [ ] micronutrient tracking (vitamins, minerals)
- [ ] dietary restriction support (vegan, gluten-free, etc.)
- [ ] food substitution recommendations
- [ ] grocery list generation

---

## 11. Limitations and Considerations

### Age Range
- **validated**: 10-120 years
- **optimal accuracy**: adults 18-65 years
- **pediatric**: mifflin-st jeor not validated for children under 10

### Activity Level Estimation
- **subjective**: users may overestimate activity
- **recommendation**: start conservative (lower activity level)
- **adjustment**: monitor results and adjust over 2-4 weeks

### Individual Variation
- **bmr variance**: ±10% between individuals
- **metabolic adaptation**: tdee may decrease with prolonged deficit
- **recommendation**: use calculations as starting point, adjust based on results

### Athletic Populations
- **elite athletes**: may need higher protein (2.2-2.4 g/kg)
- **ultra-endurance**: may need higher carbs (12+ g/kg)
- **strength athletes**: may benefit from higher fat (30-35%)

### Medical Conditions
- **contraindications**: diabetes, kidney disease, metabolic disorders
- **recommendation**: consult healthcare provider before dietary changes
- **monitoring**: regular health checkups recommended

---

## 12. Usage Examples

### Example 1: Calculate Daily Nutrition

**input:**
```rust
let params = DailyNutritionParams {
    weight_kg: 75.0,
    height_cm: 180.0,
    age: 30,
    gender: Gender::Male,
    activity_level: ActivityLevel::ModeratelyActive,
    training_goal: TrainingGoal::Maintenance,
};

let result = calculate_daily_nutrition_needs(
    &params,
    &config.bmr,
    &config.activity_factors,
    &config.macronutrients,
)?;
```

**output:**
```rust
DailyNutritionNeeds {
    bmr: 1730.0,
    tdee: 2682.0,
    protein_g: 97.5,
    carbs_g: 450.0,
    fat_g: 82.0,
    macro_percentages: MacroPercentages {
        protein_percent: 14.5,
        carbs_percent: 67.1,
        fat_percent: 27.5,
    },
    method: "Mifflin-St Jeor + Activity Factor",
}
```

### Example 2: Nutrient Timing

**input:**
```rust
let timing = calculate_nutrient_timing(
    75.0,          // weight_kg
    150.0,         // daily_protein_g
    WorkoutIntensity::High,
    &config.nutrient_timing,
)?;
```

**output:**
```rust
NutrientTimingPlan {
    pre_workout: PreWorkoutNutrition {
        carbs_g: 73.1,  // 75kg × 0.75 × 1.3
        timing_hours_before: 2.0,
    },
    post_workout: PostWorkoutNutrition {
        protein_g: 30.0,  // min(max(150/5, 20), 40)
        carbs_g: 75.0,    // 75kg × 1.0
        timing_hours_after: 2.0,
    },
    daily_protein_distribution: ProteinDistribution {
        meals_per_day: 4,
        protein_per_meal_g: 37.5,  // 150 / 4
        strategy: "Distribute 150g protein across 4 meals (~38g each)",
    },
}
```

### Example 3: Meal Analysis

**input:**
```json
{
  "meal_foods": [
    { "fdc_id": 171477, "grams": 150 },
    { "fdc_id": 170379, "grams": 200 }
  ],
  "use_mock": true
}
```

**output:**
```json
{
  "total_calories": 420,
  "total_protein_g": 40.0,
  "total_carbs_g": 46.0,
  "total_fat_g": 4.5,
  "food_details": [
    { "fdc_id": 171477, "description": "Chicken breast", "grams": 150 },
    { "fdc_id": 170379, "description": "Brown rice", "grams": 200 }
  ]
}
```

---

## Appendix: Formula Derivations

### Mifflin-St Jeor Regression Coefficients

**derived from 498-subject study:**

**weight coefficient (10.0)**
- represents metabolic cost of maintaining lean mass
- approximately 22 kcal/kg/day for lean tissue

**height coefficient (6.25)**
- correlates with body surface area
- taller individuals have higher metabolic rate

**age coefficient (-5.0)**
- accounts for age-related metabolic decline
- approximately 2% decrease per decade

**gender constant**
- male (+5): accounts for higher lean mass percentage
- female (-161): accounts for higher fat mass percentage

### Activity Factor Derivation

**based on doubly labeled water studies:**

**sedentary (1.2)**: 20% above bmr
- typical desk job with no structured exercise

**lightly active (1.375)**: 37.5% above bmr
- 1-3 days/week light exercise (walking, yoga)

**moderately active (1.55)**: 55% above bmr
- 3-5 days/week moderate exercise (running, cycling)

**very active (1.725)**: 72.5% above bmr
- 6-7 days/week intense training

**extra active (1.9)**: 90% above bmr
- professional athletes with 2×/day training

---

**document version**: 1.0.0
**last updated**: 2025-10-31
**implementation status**: production-ready
**test coverage**: 39 algorithm tests, 1,188 total tests passing

---

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

---

# Testing Strategy

## Overview

This document outlines the multi-tier testing strategy for Pierre MCP Server. The strategy is designed to provide fast feedback during development while maintaining comprehensive test coverage in CI.

## Test Suite Statistics

- **Total test files:** 166
- **Total test code:** ~62,000 lines
- **E2E tests:** 11 files
- **Comprehensive tests:** 9 files
- **Integration tests:** 11 files
- **Unit/Component tests:** ~120 files

## Test Tiers

### Tier 1: Smoke Tests (2-3 minutes)

**When to use:** On every commit via git pre-commit hook

**Script:** `./scripts/smoke-test.sh`

**What it runs:**
- Format check (`cargo fmt --check`)
- Clippy on lib + bins only
- Unit tests (`cargo test --lib`)
- 1 critical integration test (health check)

**Purpose:** Catch obvious errors immediately with minimal time investment.

### Tier 2: Fast Tests (< 5 minutes)

**When to use:** During active development when you want quick feedback

**Script:** `./scripts/fast-tests.sh`

**What it runs:**
- All unit tests
- Fast integration tests (excludes slow patterns)

**What it excludes:**
- E2E tests (require full server startup)
- Comprehensive tests (extensive test scenarios)
- Large integration tests (OAuth flows, multi-tenant, etc.)

**Purpose:** Get rapid feedback on most code changes without waiting for slow tests.

### Tier 3: Pre-Push Tests (5-10 minutes)

**When to use:** Automatically before `git push` via pre-push hook

**Script:** `./scripts/pre-push-tests.sh`

**What it runs:** 20 critical path tests covering:
1. **Critical Infrastructure** (3 tests)
   - Health endpoints
   - Database basics
   - Encryption & crypto keys
2. **Security & Authentication** (5 tests)
   - Authentication
   - API key validation
   - JWT persistence
   - OAuth2 security
   - Security headers
3. **MCP Protocol** (3 tests)
   - MCP compliance
   - JSON-RPC protocol
   - MCP tools
4. **Core Functionality** (4 tests)
   - Error handling (AppResult validation)
   - Data models
   - Database plugins (SQLite/Postgres)
   - Basic integration
5. **Multi-tenancy** (2 tests)
   - Tenant isolation
   - Tenant context
6. **Protocols & Features** (3 tests)
   - A2A protocol basics
   - Algorithm correctness (sports science)
   - Rate limiting middleware

**Purpose:** Catch 80% of issues before pushing to remote, preventing CI failures.

### Tier 4: Category Tests

**When to use:** Testing specific subsystems

**Script:** `./scripts/category-test-runner.sh <category>`

**Available categories:**
- `mcp` - MCP server tests
- `admin` - Admin functionality tests
- `oauth` - OAuth2 tests
- `security` - Security tests
- `database` - Database tests
- `intelligence` - Intelligence/analytics tests
- `config` - Configuration tests
- `auth` - Authentication tests
- `integration` - Integration tests

**Purpose:** Run focused test suites when working on specific features.

### Tier 5: Safe Test Runner

**When to use:** Running the full test suite locally without OOM issues

**Script:** `./scripts/safe-test-runner.sh`

**What it does:**
- Runs ALL 151 test files
- Batches tests (5 tests per batch)
- Pauses between batches for memory cleanup
- Generates detailed logs

**Purpose:** Complete local test validation when needed.

### Tier 6: Full CI Suite (30-60 minutes)

**When to use:** Automatically in GitHub Actions on PRs and pushes

**What it runs:**
- Format check
- Clippy (all targets, all features)
- Security audit (cargo deny)
- Architectural validation
- Secret pattern validation
- All tests with coverage (SQLite + PostgreSQL)
- Frontend tests
- SDK builds

**Purpose:** Comprehensive validation before merging to main branch.

## Test File Naming Conventions

### Slow Tests (should be excluded from fast test runs)
- `*_e2e_test.rs` - End-to-end tests requiring full server
- `*_comprehensive_test.rs` - Extensive test scenarios
- `*_integration.rs` - Integration tests
- Large route tests: `routes_comprehensive_test.rs`, `routes_dashboard_test.rs`, etc.

### Fast Tests (included in fast test runs)
- `*_test.rs` - Standard unit/component tests
- Short route tests: `routes_test.rs`, `routes_health_http_test.rs`
- Module-specific tests

## Developer Workflow

### During Active Development

```bash
# Quick feedback loop (< 5 min)
./scripts/fast-tests.sh

# Or just smoke tests (2-3 min)
./scripts/smoke-test.sh

# Test specific feature
./scripts/category-test-runner.sh mcp
```

### Before Committing

```bash
# Automatic via pre-commit hook
git commit -m "Your message"
# Runs: ./scripts/smoke-test.sh
```

### Before Pushing

```bash
# Automatic via pre-push hook
git push
# Runs: ./scripts/pre-push-tests.sh (5-10 min)
```

### Manual Full Validation

```bash
# Run everything locally (matches CI closely)
./scripts/lint-and-test.sh

# Or just the test suite
./scripts/safe-test-runner.sh
```

## Setting Up Git Hooks

To enable automatic pre-commit and pre-push testing:

```bash
./scripts/setup-git-hooks.sh
```

This installs:
- **Pre-commit hook:** Runs smoke tests (2-3 min)
- **Commit-msg hook:** Enforces 1-2 line commit messages (instant)
- **Pre-push hook:** Runs critical path tests (5-10 min)

### Bypassing Hooks (Emergency Only)

```bash
# Skip pre-commit and commit-msg
git commit --no-verify

# Skip pre-push
git push --no-verify
```

**Warning:** Only bypass hooks for legitimate emergencies. Bypassing hooks increases the risk of CI failures and breaks the fast feedback loop.

## Performance Tips

### Speed Up Local Testing

1. **Use fast tests during development:**
   ```bash
   ./scripts/fast-tests.sh  # Skip slow tests
   ```

2. **Test specific categories:**
   ```bash
   ./scripts/category-test-runner.sh auth  # Just auth tests
   ```

3. **Test single files:**
   ```bash
   cargo test --test routes_health_http_test
   ```

4. **Use watch mode for tight loops:**
   ```bash
   cargo watch -x "test --lib"
   ```

### Optimize Test Execution

Current test execution uses `--test-threads=1` globally due to database contention. Future optimizations:

1. **Increase parallelism for isolated tests**
2. **Use in-memory databases for unit tests**
3. **Mock external dependencies**
4. **Split large test files into smaller, focused tests**

## Test Categories

### Critical Path Tests (Must Pass)
- Health checks
- Authentication
- MCP protocol compliance
- Security basics
- Tenant isolation

### Important Tests (Should Pass)
- All route handlers
- Data models
- Error handling
- Configuration validation

### Extended Tests (Nice to Have)
- Comprehensive edge cases
- Performance tests
- Integration with all providers

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

## Future Improvements

### Phase 2: Test Organization
- [ ] Add test speed markers/tags
- [ ] Reorganize tests by speed (fast/medium/slow directories)
- [ ] Create test discovery tools

### Phase 3: Test Optimization
- [ ] Split large comprehensive test files
- [ ] Increase parallelism where safe
- [ ] Add mock servers for E2E tests
- [ ] Optimize slow database tests

### Phase 4: Monitoring
- [ ] Add test timing metrics
- [ ] Set up alerts for slow tests
- [ ] Regular performance reviews
- [ ] Track test suite growth

## Troubleshooting

### Tests Timeout Locally

Use the safe test runner with batching:
```bash
./scripts/safe-test-runner.sh
```

### Pre-Push Tests Too Slow

You can adjust the tests in `scripts/pre-push-tests.sh` or temporarily bypass:
```bash
git push --no-verify  # Use sparingly!
```

### CI Fails But Local Tests Pass

1. Check if you're testing with the right database (SQLite vs PostgreSQL)
2. Run the full suite locally: `./scripts/lint-and-test.sh`
3. Check for environment-specific issues

### Out of Memory (OOM) Errors

1. Use batched test runner: `./scripts/safe-test-runner.sh`
2. Run category-specific tests: `./scripts/category-test-runner.sh <category>`
3. Test files individually: `cargo test --test <test_name>`

## Summary

| Tier | Time | When | Command |
|------|------|------|---------|
| Smoke | 2-3 min | Every commit | `./scripts/smoke-test.sh` |
| Fast | < 5 min | Active dev | `./scripts/fast-tests.sh` |
| Pre-push | 5-10 min | Before push | `./scripts/pre-push-tests.sh` |
| Category | Varies | Feature work | `./scripts/category-test-runner.sh <cat>` |
| Full | 15-25 min | Before PR | `./scripts/safe-test-runner.sh` |
| CI | 30-60 min | PR/merge | Automatic in GitHub Actions |

This tiered approach ensures fast feedback during development while maintaining comprehensive coverage in CI.

---

# CI/CD Pipeline

Comprehensive documentation for the GitHub Actions continuous integration and deployment workflows.

## Overview

The project uses five specialized GitHub Actions workflows that validate different aspects of the codebase:

| Workflow | Focus | Platforms | Database Support |
|----------|-------|-----------|------------------|
| **Rust** | Core Rust quality gate | Ubuntu | SQLite |
| **Backend CI** | Comprehensive backend + frontend | Ubuntu | SQLite + PostgreSQL |
| **Cross-Platform** | OS compatibility | Linux, macOS, Windows | Mixed |
| **SDK Tests** | TypeScript SDK bridge | Ubuntu | SQLite |
| **MCP Compliance** | Protocol specification | Ubuntu | SQLite |

All workflows run on pushes to `main`, `debug/*`, `feature/*`, `claude/*` branches and on pull requests to `main`.

## Workflow Details

### Rust Workflow

**File**: `.github/workflows/rust.yml`

**Purpose**: Fast quality gate for core Rust development

**When it runs**: All pushes and PRs

**What it validates**:
1. Code formatting (`cargo fmt --check`)
2. Clippy zero-tolerance linting
3. Security audit (`cargo deny check`)
4. Architectural validation (`./scripts/architectural-validation.sh`)
5. Release build (`cargo build --release`)
6. Test coverage with `cargo-llvm-cov`
7. Codecov upload

**Database**: SQLite in-memory only

**Key characteristics**:
- Single Ubuntu runner
- Full quality checks
- ~8-10 minutes runtime
- Generates coverage report

**Environment variables**:
```bash
DATABASE_URL="sqlite::memory:"
ENCRYPTION_KEY="rEFe91l6lqLahoyl9OSzum9dKa40VvV5RYj8bHGNTeo="
PIERRE_MASTER_ENCRYPTION_KEY="rEFe91l6lqLahoyl9OSzum9dKa40VvV5RYj8bHGNTeo="
STRAVA_CLIENT_ID="test_client_id_ci"
STRAVA_CLIENT_SECRET="test_client_secret_ci"
STRAVA_REDIRECT_URI="http://localhost:8080/auth/strava/callback"
```

### Backend CI Workflow

**File**: `.github/workflows/ci.yml`

**Purpose**: Comprehensive backend and frontend validation with multi-database support

**When it runs**: All pushes and PRs

**What it validates**:

**Job 1: backend-tests (SQLite)**
1. Code formatting
2. Clippy zero-tolerance
3. Security audit
4. Architectural validation
5. Secret pattern validation (`./scripts/validate-no-secrets.sh`)
6. All tests with SQLite coverage
7. Codecov upload (flag: `backend-sqlite`)

**Job 2: postgres-tests (PostgreSQL)**
1. PostgreSQL 16 service container startup
2. Connection verification
3. Database plugin tests (`--features postgresql`)
4. All tests with PostgreSQL coverage (30-minute timeout)
5. Codecov upload (flag: `backend-postgresql`)

**Job 3: frontend-tests**
1. Node.js 20 setup
2. npm lint (`npm run lint`)
3. TypeScript type checking (`npx tsc --noEmit`)
4. Frontend tests with coverage (`npm run test:coverage`)
5. Frontend build (`npm run build`)
6. Codecov upload (flag: `frontend`)

**Key characteristics**:
- Three parallel jobs
- Separate coverage for each database
- Frontend validation included
- ~15-35 minutes runtime (PostgreSQL job is longest)

**PostgreSQL configuration**:
```bash
POSTGRES_USER=pierre
POSTGRES_PASSWORD=ci_test_password
POSTGRES_DB=pierre_mcp_server
POSTGRES_MAX_CONNECTIONS=3
POSTGRES_MIN_CONNECTIONS=1
POSTGRES_ACQUIRE_TIMEOUT=20
```

### Cross-Platform Tests Workflow

**File**: `.github/workflows/cross-platform.yml`

**Purpose**: Verify code works across Linux, macOS, and Windows

**When it runs**: Pushes and PRs that modify:
- `src/**`
- `tests/**`
- `Cargo.toml` or `Cargo.lock`
- `.github/workflows/cross-platform.yml`

**What it validates**:

**Matrix strategy**: Runs on 3 platforms in parallel
- ubuntu-latest (with PostgreSQL)
- macos-latest (SQLite only)
- windows-latest (SQLite only)

**Platform-specific behavior**:

**Ubuntu**:
- PostgreSQL 16 service container
- All features enabled (`--all-features`)
- Clippy with all features
- Tests with `--test-threads=1`

**macOS**:
- SQLite in-memory
- Default features only
- Clippy without `--all-features`
- Standard test execution

**Windows**:
- SQLite in-memory
- Default features only
- Release mode tests (`--release`) for speed
- Clippy without `--all-features`

**Key characteristics**:
- Path filtering (only Rust code changes)
- No coverage reporting
- No architectural validation
- No security audit
- Lightweight, fast checks
- ~10-15 minutes per platform

**What it doesn't do**:
- Coverage generation (focused on compatibility)
- Heavy validation steps (delegated to other workflows)

### SDK Tests Workflow

**File**: `.github/workflows/sdk-tests.yml`

**Purpose**: TypeScript SDK bridge validation and integration with Rust server

**When it runs**: Pushes and PRs that modify:
- `sdk/**`
- `.github/workflows/sdk-tests.yml`

**What it validates**:
1. Node.js 20 + Rust 1.91.0 setup
2. SDK dependency installation (`npm ci --prefer-offline`)
3. SDK bridge build (`npm run build`)
4. SDK unit tests (`npm run test:unit`)
5. Rust server debug build (`cargo build`)
6. SDK integration tests (`npm run test:integration`)
7. SDK E2E tests (`npm run test:e2e`)
8. Test artifact upload (7-day retention)

**Key characteristics**:
- Path filtering (only SDK changes)
- Multi-language validation (TypeScript + Rust)
- Debug Rust build (faster for integration tests)
- `--forceExit` flag for clean Jest shutdown
- ~8-12 minutes runtime

**Test levels**:
- **Unit**: SDK-only tests (no Rust dependency)
- **Integration**: SDK ↔ Rust server communication
- **E2E**: Complete workflow testing

### MCP Compliance Workflow

**File**: `.github/workflows/mcp-compliance.yml`

**Purpose**: Validate MCP protocol specification compliance

**When it runs**: All pushes and PRs

**What it validates**:
1. Python 3.11 + Node.js 20 + Rust 1.91.0 setup
2. MCP Validator installation (cloned from `Janix-ai/mcp-validator`)
3. SDK dependency installation
4. SDK bridge build
5. SDK TypeScript types validation:
   - Checks `src/types.ts` exists
   - Rejects placeholder content
   - Requires pre-generated types in repository
6. MCP compliance validation (`./scripts/ensure_mcp_compliance.sh`)
7. Artifact cleanup

**Key characteristics**:
- Multi-language stack (Python + Node.js + Rust)
- External validation tool
- Strict type generation requirements
- Disk space management (aggressive cleanup)
- CI-specific flags (`CI=true`, `GITHUB_ACTIONS=true`)
- Security flags (`PIERRE_ALLOW_INTERACTIVE_OAUTH=false`)
- ~10-15 minutes runtime

**Environment variables**:
```bash
CI="true"
GITHUB_ACTIONS="true"
HTTP_PORT=8080
DATABASE_URL="sqlite::memory:"
PIERRE_MASTER_ENCRYPTION_KEY="rEFe91l6lqLahoyl9OSzum9dKa40VvV5RYj8bHGNTeo="
PIERRE_ALLOW_INTERACTIVE_OAUTH="false"
PIERRE_RSA_KEY_SIZE="2048"
```

## Workflow Triggers

### Push Triggers

All workflows run on these branches:
- `main`
- `debug/*`
- `feature/*`
- `claude/*`

### Pull Request Triggers

All workflows run on PRs to:
- `main`

### Path Filtering

Some workflows only run when specific files change:

**Cross-Platform Tests**:
- `src/**`
- `tests/**`
- `Cargo.toml`, `Cargo.lock`
- `.github/workflows/cross-platform.yml`

**SDK Tests**:
- `sdk/**`
- `.github/workflows/sdk-tests.yml`

**Optimization rationale**: Path filtering reduces CI resource usage by skipping irrelevant workflow runs. For example, changing only SDK code doesn't require cross-platform Rust validation.

## Understanding CI/CD Results

### Status Indicators

- ✅ **Green check**: All validations passed
- ⚠️ **Yellow circle**: Workflow in progress
- ❌ **Red X**: One or more checks failed

### Common Failure Patterns

#### Formatting Failure
```
error: left behind trailing whitespace
```
**Fix**: Run `cargo fmt` locally before committing

#### Clippy Failure
```
error: using `unwrap()` on a `Result` value
```
**Fix**: Use proper error handling with `?` operator or `ok_or_else()`

#### Test Failure
```
test result: FAILED. 1245 passed; 7 failed
```
**Fix**: Run `cargo test` locally to reproduce, fix failing tests

#### Security Audit Failure
```
error: 1 security advisory found
```
**Fix**: Run `cargo deny check` locally, update dependencies or add justified ignore

#### Architectural Validation Failure
```
ERROR: Found unwrap() usage in production code
```
**Fix**: Run `./scripts/architectural-validation.sh` locally, fix violations

#### PostgreSQL Connection Failure
```
ERROR: PostgreSQL connection timeout
```
**Cause**: PostgreSQL service container not ready
**Fix**: Usually transient, re-run workflow

#### SDK Type Validation Failure
```
ERROR: src/types.ts contains placeholder content
```
**Fix**: Run `npm run generate-types` locally with running server, commit generated types

### Viewing Detailed Logs

1. Navigate to Actions tab in GitHub
2. Click on the workflow run
3. Click on the failing job
4. Expand the failing step
5. Review error output

## Local Validation Before Push

Run the same checks locally to catch issues before CI:

```bash
# 1. Format code
cargo fmt

# 2. Architectural validation
./scripts/architectural-validation.sh

# 3. Zero-tolerance clippy
cargo clippy --tests -- \
  -W clippy::all \
  -W clippy::pedantic \
  -W clippy::nursery \
  -D warnings

# 4. Run all tests
cargo test

# 5. Security audit
cargo deny check

# 6. SDK tests (if SDK changed)
cd sdk
npm run test:unit
npm run test:integration
npm run test:e2e
cd ..

# 7. Frontend tests (if frontend changed)
cd frontend
npm run lint
npm run test:coverage
npm run build
cd ..
```

**Shortcut**: Use validation script
```bash
./scripts/lint-and-test.sh
```

## Debugging CI/CD Failures

### Reproducing Locally

Match CI environment exactly:

```bash
# Set CI environment variables
export DATABASE_URL="sqlite::memory:"
export ENCRYPTION_KEY="rEFe91l6lqLahoyl9OSzum9dKa40VvV5RYj8bHGNTeo="
export PIERRE_MASTER_ENCRYPTION_KEY="rEFe91l6lqLahoyl9OSzum9dKa40VvV5RYj8bHGNTeo="
export STRAVA_CLIENT_ID="test_client_id_ci"
export STRAVA_CLIENT_SECRET="test_client_secret_ci"
export STRAVA_REDIRECT_URI="http://localhost:8080/auth/strava/callback"

# Run tests matching CI configuration
cargo test --test-threads=1
```

### Platform-Specific Issues

**macOS vs Linux differences**:
- File system case sensitivity
- Line ending handling (CRLF vs LF)
- Path separator differences

**Windows-specific issues**:
- Longer compilation times (run release mode tests)
- Path length limitations
- File locking behavior

### PostgreSQL-Specific Debugging

Start local PostgreSQL matching CI:

```bash
docker run -d \
  --name postgres-ci \
  -e POSTGRES_USER=pierre \
  -e POSTGRES_PASSWORD=ci_test_password \
  -e POSTGRES_DB=pierre_mcp_server \
  -p 5432:5432 \
  postgres:16-alpine

# Wait for startup
sleep 5

# Run PostgreSQL tests
export DATABASE_URL="postgresql://pierre:ci_test_password@localhost:5432/pierre_mcp_server"
cargo test --features postgresql

# Cleanup
docker stop postgres-ci
docker rm postgres-ci
```

### SDK Integration Debugging

Run SDK tests with debug output:

```bash
cd sdk

# Build Rust server in debug mode
cd ..
cargo build
cd sdk

# Run tests with verbose output
npm run test:integration -- --verbose
npm run test:e2e -- --verbose
```

## Coverage Reporting

### Codecov Integration

Coverage reports are uploaded to Codecov with specific flags:

- `backend-sqlite`: SQLite test coverage
- `backend-postgresql`: PostgreSQL test coverage
- `frontend`: Frontend test coverage

### Viewing Coverage

1. Navigate to Codecov dashboard
2. Filter by flag to see database-specific coverage
3. Review coverage trends over time
4. Identify untested code paths

### Coverage Thresholds

No enforced thresholds (yet), but aim for:
- Core business logic: >80%
- Database plugins: >75%
- Protocol handlers: >70%

## Workflow Maintenance

### Updating Rust Version

When updating Rust toolchain:

1. Update `rust-toolchain` file
2. Update `.github/workflows/*.yml` (search for `dtolnay/rust-toolchain@`)
3. Test locally with new version
4. Commit and verify all workflows pass

### Updating Dependencies

When updating crate dependencies:

1. Run `cargo update`
2. Test locally
3. Check `cargo deny check` for new advisories
4. Update `deny.toml` if needed (with justification)
5. Commit and verify CI passes

### Adding New Workflow

When adding new validation:

1. Create workflow file in `.github/workflows/`
2. Test workflow on feature branch
3. Document in this file
4. Update summary table
5. Add to `contributing.md` review process

## Cost Optimization

### Cache Strategy

Workflows use `actions/cache@v4` for:
- Rust dependencies (`~/.cargo/`)
- Compiled artifacts (`target/`)
- Node.js dependencies (`node_modules/`)

**Cache keys** include:
- OS (`${{ runner.os }}`)
- Rust version
- `Cargo.lock` hash

### Disk Space Management

Ubuntu runners have limited disk space (~14GB usable).

**Free disk space steps**:
- Remove unused Android SDK
- Remove unused .NET frameworks
- Remove unused Docker images
- Clean Cargo cache

**Workflows using cleanup**:
- Rust workflow
- Backend CI workflow
- Cross-Platform Tests workflow
- MCP Compliance workflow

### Parallel Execution

Jobs run in parallel when independent:
- Backend CI: 3 jobs in parallel (SQLite, PostgreSQL, frontend)
- Cross-Platform: 3 jobs in parallel (Linux, macOS, Windows)

**Total CI time**: ~30-35 minutes (longest job determines duration)

## Troubleshooting Reference

### "failed to get `X` as a dependency"

**Cause**: Network timeout fetching crate
**Fix**: Re-run workflow (transient issue)

### "disk quota exceeded"

**Cause**: Insufficient disk space on runner
**Fix**: Workflow already includes cleanup; may need to reduce artifact size

### "database connection pool exhausted"

**Cause**: Tests creating too many connections
**Fix**: Tests use `--test-threads=1` to serialize execution

### "clippy warnings found"

**Cause**: New clippy version detected additional issues
**Fix**: Run `cargo clippy --fix` locally, review and commit

### "mcp validator not found"

**Cause**: Failed to clone mcp-validator repository
**Fix**: Re-run workflow (transient network issue)

### "sdk types contain placeholder"

**Cause**: Generated types not committed to repository
**Fix**: Run `npm run generate-types` locally with server running, commit result

## Best Practices

### Before Creating PR

1. Run `./scripts/lint-and-test.sh` locally
2. Verify all tests pass
3. Check clippy with zero warnings
4. Review architectural validation
5. If SDK changed, run SDK tests
6. If frontend changed, run frontend tests

### Reviewing PR CI Results

1. Wait for all workflows to complete
2. Review any failures immediately
3. Don't merge with failing workflows
4. Check coverage hasn't decreased significantly
5. Review security audit warnings

### Maintaining CI/CD Health

1. Monitor workflow run times (alert if >50% increase)
2. Review dependency updates monthly
3. Update Rust version quarterly
4. Keep workflows DRY (extract common steps to scripts)
5. Document any workflow changes in this file

## Future Improvements

Planned enhancements:

- Enforce coverage thresholds
- Add benchmark regression testing
- Add performance profiling workflow
- Add automated dependency updates (Dependabot)
- Add deployment workflow for releases
- Add E2E testing with real Strava API (secure credentials)

## Additional Resources

- GitHub Actions Documentation
- Codecov Documentation
- cargo-deny Configuration
- cargo-llvm-cov Usage

---

# Contributing

## Development Setup

```bash
git clone https://github.com/Async-IO/pierre_mcp_server.git
cd pierre_mcp_server

# install direnv (optional but recommended)
brew install direnv
direnv allow

# build
cargo build

# run tests
cargo test

# run validation
./scripts/lint-and-test.sh
```

## Code Standards

### Rust Idiomatic Code

- prefer borrowing (`&T`) over cloning
- use `Result<T, E>` for all fallible operations
- never use `unwrap()` in production code (tests ok)
- document all public apis with `///` comments
- follow rust naming conventions (snake_case)

### Error Handling

Use structured error types (no anyhow!):
```rust
// bad - anyhow not allowed
use anyhow::Result;

// good - use AppResult and structured errors
use crate::errors::AppResult;

pub async fn my_function() -> AppResult<Value> {
    // errors automatically convert via From trait
    let user = db.users().get_by_id(id).await?;
    Ok(result)
}
```

No panics in production code:
```rust
// bad
let value = some_option.unwrap();

// good
let value = some_option.ok_or(MyError::NotFound)?;
```

**Important**: The codebase enforces zero-tolerance for `impl From<anyhow::Error>` via static analysis (commits b592b5e, 3219f07).

### Forbidden Patterns

- `unwrap()`, `expect()`, `panic!()` in src/ (except tests)
- `#[allow(clippy::...)]` attributes
- variables/functions starting with `_` (use meaningful names)
- hardcoded magic values
- `todo!()`, `unimplemented!()` placeholders

### Required Patterns

- all modules start with aboutme comments:
```rust
// ABOUTME: Brief description of what this module does
// ABOUTME: Second line of description if needed
```

- every `.clone()` must be justified with comment:
```rust
let db = database.clone(); // clone for tokio::spawn thread safety
```

## Testing

### Test Requirements

Every feature needs:
1. **unit tests**: test individual functions
2. **integration tests**: test component interactions
3. **e2e tests**: test complete workflows

No exceptions. If you think a test doesn't apply, ask first.

### Running Tests

```bash
# all tests
cargo test

# specific test
cargo test test_name

# integration tests
cargo test --test mcp_multitenant_complete_test

# with output
cargo test -- --nocapture

# quiet mode
cargo test --quiet
```

### Test Patterns

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_feature() {
        // arrange
        let input = setup_test_data();

        // act
        let result = function_under_test(input).await;

        // assert
        assert!(result.is_ok());
    }
}
```

### Test Location

- unit tests: in same file as code (`#[cfg(test)] mod tests`)
- integration tests: in `tests/` directory
- avoid `#[cfg(test)]` in src/ (tests only)

## Workflow

### Creating Features

1. Create feature branch:
```bash
git checkout -b feature/my-feature
```

2. Implement feature with tests
3. Run validation:
```bash
./scripts/lint-and-test.sh
```

4. Commit:
```bash
git add .
git commit -m "feat: add my feature"
```

5. Push and create pr:
```bash
git push origin feature/my-feature
```

### Fixing Bugs

Bug fixes go directly to main branch:
```bash
git checkout main
# fix bug
git commit -m "fix: correct issue with X"
git push origin main
```

### Commit Messages

Follow conventional commits:
- `feat:` - new feature
- `fix:` - bug fix
- `refactor:` - code refactoring
- `docs:` - documentation changes
- `test:` - test additions/changes
- `chore:` - maintenance tasks

No ai assistant references in commits (automated text removed).

## Validation

### Pre-commit Checks

```bash
./scripts/lint-and-test.sh
```

Runs:
1. Clippy with strict lints
2. Pattern validation (no unwrap, no placeholders)
3. All tests
4. Format check

### Clippy

```bash
cargo clippy -- -W clippy::all -W clippy::pedantic -W clippy::nursery -D warnings
```

Zero tolerance for warnings.

### Pattern Validation

Checks for banned patterns:
```bash
# no unwrap/expect/panic
rg "unwrap\(\)|expect\(|panic!\(" src/

# no placeholders
rg -i "placeholder|todo|fixme" src/

# no clippy allows
rg "#\[allow\(clippy::" src/

# no underscore prefixes
rg "fn _|let _[a-zA-Z]|struct _|enum _" src/
```

### Git Hooks

Install pre-commit hook:
```bash
./scripts/install-hooks.sh
```

Runs validation automatically before commits.

## Architecture Guidelines

### Dependency Injection

Use `Arc<T>` for shared resources:
```rust
pub struct ServerResources {
    pub database: Arc<Database>,
    pub auth_manager: Arc<AuthManager>,
    // ...
}
```

Pass resources to components, not global state.

### Protocol Abstraction

Business logic in `src/protocols/universal/`. Protocol handlers (mcp, a2a) just translate requests/responses.

```rust
// business logic - protocol agnostic
impl UniversalToolExecutor {
    pub async fn execute_tool(&self, tool: &str, params: Value) -> Result<Value> {
        // implementation
    }
}

// protocol handler - translation only
impl McpHandler {
    pub async fn handle_tool_call(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let result = self.executor.execute_tool(&request.tool, request.params).await;
        // translate to json-rpc response
    }
}
```

### Multi-tenant Isolation

Every request needs tenant context:
```rust
pub struct TenantContext {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub role: TenantRole,
}
```

Database queries filter by tenant_id.

### Error Handling

Use thiserror for custom errors:
```rust
#[derive(Debug, thiserror::Error)]
pub enum MyError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("database error")]
    Database(#[from] DatabaseError),
}
```

Propagate with `?` operator.

## Adding New Features

### New Fitness Provider

1. Implement `FitnessProvider` trait in `src/providers/`:
```rust
pub struct NewProvider {
    config: ProviderConfig,
    credentials: Option<OAuth2Credentials>,
}

#[async_trait]
impl FitnessProvider for NewProvider {
    fn name(&self) -> &'static str { "new_provider" }
    // ... implement other methods
}
```

2. Register in `src/providers/registry.rs`
3. Add oauth configuration in `src/oauth/`
4. Add tests

### New MCP Tool

1. Define tool in `src/protocols/universal/tool_registry.rs`:
```rust
pub const TOOL_MY_FEATURE: ToolDefinition = ToolDefinition {
    name: "my_feature",
    description: "Description of what it does",
    input_schema: ...,
};
```

2. Implement handler in `src/protocols/universal/handlers/`:
```rust
pub async fn handle_my_feature(
    context: &UniversalContext,
    params: Value,
) -> Result<Value> {
    // implementation
}
```

3. Register in tool executor
4. Add unit + integration tests
5. Regenerate SDK types:
```bash
# Ensure server is running
cargo run --bin pierre-mcp-server

# Generate TypeScript types
cd sdk
npm run generate-types
git add src/types.ts
```

**Why**: SDK type definitions are auto-generated from server tool schemas. This ensures TypeScript clients have up-to-date parameter types for the new tool.

### New Database Backend

1. Implement repository traits in `src/database_plugins/`:
```rust
use crate::database::repositories::*;

pub struct MyDbProvider { /* ... */ }

// Implement each repository trait for your backend
#[async_trait]
impl UserRepository for MyDbProvider {
    // implement user management methods
}

#[async_trait]
impl OAuthTokenRepository for MyDbProvider {
    // implement oauth token methods
}
// ... implement other 11 repository traits
```

2. Add to factory in `src/database_plugins/factory.rs`
3. Add migration support
4. Add comprehensive tests

**Note**: The codebase uses the repository pattern with 13 focused repository traits (commit 6f3efef). See `src/database/repositories/mod.rs` for the complete list.

## Documentation

### Code Documentation

All public items need doc comments:
```rust
/// Brief description of function
///
/// # Arguments
///
/// * `param` - Description of parameter
///
/// # Returns
///
/// Description of return value
///
/// # Errors
///
/// When this function errors
pub fn my_function(param: Type) -> Result<Type> {
    // implementation
}
```

### Updating Docs

After significant changes:
1. Update relevant docs in `docs/`
2. Keep docs concise and accurate
3. Remove outdated information
4. Test all code examples

## Getting Help

- check existing code for examples
- read rust documentation: https://doc.rust-lang.org/
- ask in github discussions
- open issue for bugs/questions

## Review Process

1. Automated checks must pass (ci) - see ci/cd documentation
2. Code review by maintainer
3. All feedback addressed
4. Tests added/updated
5. Documentation updated
6. Merge to main

### CI/CD Requirements

All GitHub Actions workflows must pass before merge:
- **Rust**: Core quality gate (formatting, clippy, tests)
- **Backend CI**: Multi-database validation (SQLite + PostgreSQL)
- **Cross-Platform**: OS compatibility (Linux, macOS, Windows)
- **SDK Tests**: TypeScript SDK bridge validation
- **MCP Compliance**: Protocol specification conformance

See ci/cd.md for detailed workflow documentation, troubleshooting, and local validation commands.

## Release Process

Handled by maintainers:
1. Version bump in `Cargo.toml`
2. Update changelog
3. Create git tag
4. Publish to crates.io
5. Publish sdk to npm

## Code of Conduct

- be respectful
- focus on technical merit
- welcome newcomers
- assume good intentions
- provide constructive feedback

---
