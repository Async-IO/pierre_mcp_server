# Pierre Fitness Platform - Complete Technical Reference

> This document is optimized for LLM consumption. Use it to answer questions about Pierre's architecture, code patterns, and implementation details.

---

## OVERVIEW

**Pierre** is a production Rust fitness API platform implementing:
- **MCP (Model Context Protocol)** - JSON-RPC 2.0 protocol for AI assistant tool execution
- **A2A (Agent-to-Agent)** - Inter-agent communication protocol
- **REST API** - Traditional HTTP endpoints for admin dashboard
- **OAuth 2.0** - RFC 7591 server for MCP clients + OAuth client for fitness providers

**Tech Stack**: Rust 1.91+, Tokio async runtime, Axum web framework, SQLx (SQLite/PostgreSQL), TypeScript SDK, React frontend

**Codebase Stats**: 287 source files, 190 test files, 47 MCP tools, 45 modules

---

## SYSTEM ARCHITECTURE

### High-Level Design
```
┌─────────────────┐
│   MCP Clients   │ Claude Desktop, ChatGPT, etc.
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│   Pierre SDK    │ TypeScript bridge (stdio → HTTP)
│   (npm package) │
└────────┬────────┘
         │ HTTP + OAuth2
         ▼
┌─────────────────────────────────────────┐
│   Pierre Fitness Platform (Rust)        │
│   Port 8081 (all protocols)             │
│                                          │
│   • MCP protocol (JSON-RPC 2.0)        │
│   • OAuth2 server (RFC 7591)           │
│   • A2A protocol (agent-to-agent)      │
│   • REST API                            │
│   • SSE (real-time notifications)      │
└────────┬────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────┐
│   Fitness Providers (1 to x)            │
│   • Strava                              │
│   • Garmin                              │
│   • Fitbit                              │
│   • WHOOP                               │
│   • Terra (150+ wearables)              │
│   • Synthetic (OAuth-free dev/testing)  │
│                                          │
│   ProviderRegistry: runtime discovery   │
│   Environment config: PIERRE_*_*        │
└─────────────────────────────────────────┘
```

### Request Flow
```
Client Request
    ↓
[Security Middleware] → CORS, headers, CSRF
    ↓
[Authentication] → JWT or API key
    ↓
[Tenant Context] → Load user/tenant data
    ↓
[Rate Limiting] → Check quotas
    ↓
[Protocol Router]
    ├─ MCP → Universal Protocol → Tools
    ├─ A2A → Universal Protocol → Tools
    └─ REST → Direct handlers
    ↓
[Tool Execution]
    ├─ Providers (Strava/Garmin/Fitbit/WHOOP)
    ├─ Intelligence (analysis)
    └─ Configuration
    ↓
[Database + Cache]
    ↓
Response
```

### Single Port Architecture
All protocols share port 8081 for simplified deployment, easier OAuth callback handling, and unified TLS/security

---

## PROJECT STRUCTURE

```
pierre_mcp_server/
├── src/                          # Rust source (library + binaries)
│   ├── lib.rs                    # Library root - 45 module declarations
│   ├── bin/                      # Binary entry points
│   │   ├── pierre-mcp-server.rs  # Main server
│   │   └── admin_setup.rs        # Admin CLI
│   ├── mcp/                      # MCP protocol (10 files)
│   ├── a2a/                      # A2A protocol
│   ├── protocols/universal/      # Shared protocol layer
│   ├── providers/                # Fitness providers (Strava, Garmin, Fitbit, WHOOP, Terra)
│   ├── intelligence/             # Sports science algorithms
│   ├── database/                 # Repository traits (13 focused traits)
│   ├── database_plugins/         # SQLite/PostgreSQL implementations
│   └── [35+ other modules]
├── sdk/                          # TypeScript SDK for stdio transport
├── frontend/                     # React admin dashboard
├── tests/                        # 190 integration/e2e tests
└── templates/                    # OAuth HTML templates
```

---

## ERROR HANDLING

### Policy
- **NEVER use `anyhow::anyhow!()`** in production code
- Use structured error types: `AppError`, `DatabaseError`, `ProviderError`
- All functions return `AppResult<T>` (alias for `Result<T, AppError>`)

### Error Hierarchy
```
AppError (src/errors.rs)           ← HTTP-level errors
    ├── DatabaseError              ← Database operations
    ├── ProviderError              ← External API calls
    └── ProtocolError              ← Protocol-specific errors
```

### Error Type Example
```rust
#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Entity not found: {entity_type} with id '{entity_id}'")]
    NotFound { entity_type: &'static str, entity_id: String },

    #[error("Tenant isolation violation: {entity_type} '{entity_id}'")]
    TenantIsolationViolation {
        entity_type: &'static str,
        entity_id: String,
        requested_tenant: String,
        actual_tenant: String,
    },
}
```

### HTTP Status Code Mapping
| ErrorCode | HTTP Status | Description |
|-----------|-------------|-------------|
| AuthRequired | 401 | Authentication needed |
| AuthInvalid | 401 | Invalid credentials |
| PermissionDenied | 403 | Not authorized |
| ResourceNotFound | 404 | Entity not found |
| RateLimitExceeded | 429 | Too many requests |
| InternalError | 500 | Server error |

---

## CONFIGURATION

### Environment Variables
```bash
# Database
DATABASE_URL="sqlite:./data/users.db"
PIERRE_MASTER_ENCRYPTION_KEY="$(openssl rand -base64 32)"

# Server
HTTP_PORT=8081
RUST_LOG=info
JWT_EXPIRY_HOURS=24

# OAuth Providers
STRAVA_CLIENT_ID=your_client_id
STRAVA_CLIENT_SECRET=your_client_secret

# Algorithm Selection
PIERRE_MAXHR_ALGORITHM=tanaka      # fox, tanaka, nes, gulati
PIERRE_TSS_ALGORITHM=avg_power     # avg_power, normalized_power, hybrid
PIERRE_VDOT_ALGORITHM=daniels
```

### Tokio Runtime Configuration
```bash
TOKIO_WORKER_THREADS=4             # Worker thread count
TOKIO_THREAD_STACK_SIZE=2097152    # Stack size (2MB)
TOKIO_THREAD_NAME=pierre-worker    # Thread name prefix
```

### SQLx Pool Configuration
```bash
SQLX_IDLE_TIMEOUT_SECS=600         # Idle connection timeout
SQLX_MAX_LIFETIME_SECS=1800        # Max connection lifetime
SQLX_TEST_BEFORE_ACQUIRE=true      # Validate before use
SQLX_STATEMENT_CACHE_CAPACITY=100  # Prepared statement cache
```

---

## DEPENDENCY INJECTION

### ServerResources (Central DI Container)
```rust
#[derive(Clone)]
pub struct ServerResources {
    pub database: Arc<Database>,
    pub auth_manager: Arc<AuthManager>,
    pub jwks_manager: Arc<JwksManager>,
    pub provider_registry: Arc<ProviderRegistry>,
    pub cache: Arc<Cache>,
    pub config: Arc<ServerConfig>,
    // ... 15+ more dependencies
}
```

### Focused Contexts (Preferred Pattern)
```rust
pub struct AuthContext {
    pub auth_manager: Arc<AuthManager>,
    pub jwks_manager: Arc<JwksManager>,
}

pub struct DataContext {
    pub database: Arc<Database>,
    pub provider_registry: Arc<ProviderRegistry>,
}
```

### Arc Usage
- Clone Arc (cheap) - just increments atomic counter
- Create resources once at startup, wrap in Arc, share via cloning
- Use `Arc<T>` for thread-safe shared ownership in async contexts

---

## CRYPTOGRAPHIC KEYS

### Two-Tier Key Management (MEK + DEK)
```
┌─────────────────────────────────────────────┐
│         Master Encryption Key (MEK)          │
│  PIERRE_MASTER_ENCRYPTION_KEY env var        │
│  Base64-encoded 32 bytes (256 bits)          │
└─────────────────┬───────────────────────────┘
                  │ encrypts
                  ▼
┌─────────────────────────────────────────────┐
│        Data Encryption Keys (DEKs)           │
│  Generated per-tenant, stored encrypted      │
│  in database encryption_keys table           │
└─────────────────────────────────────────────┘
```

### JWKS Manager (RS256 JWT Signing)
- 2048-bit RSA keys for JWT signing
- Keys stored encrypted in database
- JWKS endpoint: `/.well-known/jwks.json`

---

## JWT AUTHENTICATION

### Token Structure
```json
{
  "sub": "user_uuid",
  "tenant_id": "tenant_uuid",
  "email": "user@example.com",
  "role": "user",
  "exp": 1234567890,
  "iat": 1234567890
}
```

### Authentication Flow
1. User authenticates via OAuth or login
2. Server generates RS256-signed JWT
3. Client includes JWT in `Authorization: Bearer <token>` header
4. Middleware validates signature and extracts claims
5. Request proceeds with authenticated user context

### Token Sources
- **HTTP**: `Authorization` header
- **WebSocket**: Initial connection params
- **stdio**: `auth` field in JSON-RPC params

---

## MULTI-TENANT ISOLATION

### Tenant Model
```rust
pub struct Tenant {
    pub id: Uuid,
    pub name: String,
    pub slug: String,           // URL-friendly identifier
    pub created_at: DateTime<Utc>,
}
```

### Isolation Pattern
- Every database query includes `WHERE tenant_id = ?`
- Users belong to exactly one tenant
- Cross-tenant access returns `TenantIsolationViolation` error
- Tenant context extracted from JWT `tenant_id` claim

---

## MCP PROTOCOL

### JSON-RPC 2.0 Foundation
```json
// Request
{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {...}}

// Response
{"jsonrpc": "2.0", "id": 1, "result": {...}}

// Error
{"jsonrpc": "2.0", "id": 1, "error": {"code": -32600, "message": "..."}}
```

### MCP Methods
| Method | Auth Required | Description |
|--------|---------------|-------------|
| `initialize` | No | Protocol negotiation |
| `tools/list` | No | List available tools |
| `tools/call` | Yes | Execute a tool |
| `resources/list` | No | List resources |

### Request Flow
```
Client → Transport → JSON Parse → Validate → Route → Auth → Tenant → Execute → Serialize → Response
```

---

## MCP TOOLS (47 Total)

### Strava API Tools
- `get_athlete` - Get authenticated athlete profile
- `get_activities` - List activities with pagination
- `get_activity_details` - Single activity with streams
- `get_activity_zones` - Heart rate/power zones
- `get_activity_laps` - Lap data
- `get_activity_streams` - Time series data
- `get_stats` - Athlete statistics

### Intelligence Tools
- `analyze_activity` - AI-powered activity analysis
- `get_training_load` - CTL/ATL/TSB metrics
- `get_fitness_trends` - Long-term trends
- `calculate_zones` - HR/power zone calculation
- `estimate_ftp` - FTP estimation
- `estimate_vo2max` - VO2max estimation

### Goal Tools
- `create_goal` - Create fitness goal
- `get_goals` - List goals
- `update_goal_progress` - Update progress

### Configuration Tools
- `get_user_preferences` - User settings
- `update_user_preferences` - Update settings
- `get_algorithm_config` - Algorithm selection

### Sleep & Recovery Tools
- `log_sleep` - Log sleep data
- `get_sleep_history` - Sleep trends
- `get_recovery_status` - Recovery recommendations

### Nutrition Tools
- `search_foods` - USDA food search
- `log_meal` - Log food intake
- `get_nutrition_summary` - Daily nutrition

### Connection Tools
- `list_connections` - Provider connections
- `connect_provider` - Initiate OAuth
- `disconnect_provider` - Remove connection

---

## OUTPUT FORMATTERS

### Supported Formats
```rust
pub enum OutputFormat {
    Json,  // Default - universal compatibility
    Toon,  // Token-Oriented Object Notation (~40% token reduction)
}
```

### TOON Benefits
- 40% fewer tokens for LLM consumption
- Eliminates JSON syntax overhead (quotes, colons, commas)
- Ideal for large datasets (year of activities)

### Usage
```rust
let format = OutputFormat::from_str_param("toon");
let output = format_output(&activities, format)?;
// output.data = serialized string
// output.content_type = "application/vnd.toon"
```

---

## TRANSPORT LAYERS

### HTTP Transport
- Primary transport for web clients
- Endpoint: `POST /mcp`
- Auth via `Authorization: Bearer <token>`

### WebSocket Transport
- Bidirectional real-time communication
- Endpoint: `GET /ws`
- Persistent connection with heartbeat

### stdio Transport (SDK)
- For MCP hosts (Claude Desktop, etc.)
- SDK bridges stdio ↔ HTTP
- Manages OAuth flow and token storage

### SSE Transport
- Server-Sent Events for streaming
- Endpoint: `GET /sse`
- Progress updates during long operations

---

## OAUTH 2.0

### Pierre as OAuth Server (RFC 7591)
- Issues tokens to MCP clients
- Dynamic client registration
- Authorization code flow with PKCE

### Pierre as OAuth Client
- Connects to fitness providers (Strava, Garmin, etc.)
- Stores tokens encrypted per-user
- Automatic token refresh

### OAuth Flow
```
1. Client redirects to /oauth/authorize
2. User authenticates with provider
3. Provider redirects to /oauth/callback
4. Pierre exchanges code for tokens
5. Tokens stored encrypted in database
6. Client receives success notification
```

---

## FITNESS PROVIDERS

### Pluggable Provider Architecture
- **Factory pattern**: Runtime registration of providers
- **Feature flags**: Compile-time provider selection (`provider-strava`, `provider-garmin`, etc.)
- **Service Provider Interface (SPI)**: `ProviderDescriptor` trait for external providers
- **1 to x providers simultaneously**: Use Strava + Garmin + custom providers at once
- **Zero code changes**: Add new providers without modifying tools or handlers

### Supported Providers
| Provider | Features |
|----------|----------|
| Strava | Activities, athlete, zones, streams |
| Garmin | Activities, sleep, body composition |
| Fitbit | Activities, sleep, heart rate |
| WHOOP | Recovery, strain, sleep |
| Terra | 150+ wearables via unified API |
| Synthetic | OAuth-free dev/testing |

### Provider Configuration
Environment-based config via `PIERRE_<PROVIDER>_*`:
```bash
PIERRE_STRAVA_CLIENT_ID=your_client_id
PIERRE_STRAVA_CLIENT_SECRET=your_client_secret
PIERRE_<PROVIDER>_AUTH_URL=...
PIERRE_<PROVIDER>_TOKEN_URL=...
PIERRE_<PROVIDER>_SCOPES=...
```

### Provider Trait
```rust
pub trait FitnessProvider: Send + Sync {
    async fn get_activities(&self, user_id: &str, params: &ActivityParams)
        -> ProviderResult<Vec<Activity>>;
    async fn get_athlete(&self, user_id: &str)
        -> ProviderResult<Athlete>;
    // ... more methods
}
```

### Provider Registry
```rust
// Runtime discovery
let providers = provider_registry.supported_providers();
let is_supported = provider_registry.is_supported("strava");
let provider = provider_registry.get_provider("strava")?;
```

---

## SPORTS SCIENCE ALGORITHMS

### Training Stress Score (TSS)
```
TSS = (duration × NP × IF) / (FTP × 3600) × 100
where:
  NP = Normalized Power
  IF = Intensity Factor (NP/FTP)
  FTP = Functional Threshold Power
```

### Training Load (CTL/ATL/TSB)
```
CTL (Chronic Training Load) = 42-day exponential moving average of TSS
ATL (Acute Training Load) = 7-day exponential moving average of TSS
TSB (Training Stress Balance) = CTL - ATL
```

### Max HR Algorithms
| Algorithm | Formula |
|-----------|---------|
| Fox | 220 - age |
| Tanaka | 208 - (0.7 × age) |
| Nes | 211 - (0.64 × age) |
| Gulati (women) | 206 - (0.88 × age) |

### VO2max Estimation
- Jack Daniels VDOT tables
- Cooper test formula
- From race performances

### Algorithm Dependency Injection
Zero-overhead algorithm dispatch using Rust enums instead of hardcoded formulas.

**Nine Algorithm Categories** (each with multiple variants):
| Category | Environment Variable | Example Variants |
|----------|---------------------|------------------|
| Max Heart Rate | `PIERRE_MAXHR_ALGORITHM` | fox, tanaka, nes, gulati |
| TRIMP | `PIERRE_TRIMP_ALGORITHM` | bannister, edwards, lucia |
| TSS | `PIERRE_TSS_ALGORITHM` | avg_power, normalized_power, hybrid |
| VDOT | `PIERRE_VDOT_ALGORITHM` | daniels, riegel, hybrid |
| Training Load | `PIERRE_TRAINING_LOAD_ALGORITHM` | ema, sma, wma, kalman |
| Recovery | `PIERRE_RECOVERY_ALGORITHM` | weighted, additive, multiplicative |
| FTP | `PIERRE_FTP_ALGORITHM` | 20min_test, 8min_test, ramp_test |
| LTHR | `PIERRE_LTHR_ALGORITHM` | from_maxhr, from_30min, from_race |
| VO2max | `PIERRE_VO2MAX_ALGORITHM` | from_vdot, cooper, rockport, bruce |

**Hybrid Algorithms**: Try accurate method first, fallback to simpler method if data unavailable.

**Implementation Pattern**:
```rust
pub enum TssAlgorithm {
    AvgPower,                // Simple, always works
    NormalizedPower { .. },  // Accurate, requires power stream
    Hybrid,                  // Try NP, fallback to avg_power
}

let algorithm = TssAlgorithm::from_str(&config.algorithms.tss)?;
let tss = algorithm.calculate_tss(activity)?;
```

---

## A2A PROTOCOL

### Agent Card (Capability Discovery)
```json
{
  "name": "Pierre Fitness Agent",
  "description": "Fitness data analysis agent",
  "capabilities": ["fitness_analysis", "training_recommendations"],
  "endpoint": "https://pierre.example.com/a2a"
}
```

### A2A Authentication
- Ed25519 key pairs per agent
- Request signing with timestamps
- Mutual authentication

---

## DATABASE

### Abstraction Layer
```rust
pub enum Database {
    Sqlite(SqliteDatabase),
    Postgres(PostgresDatabase),
}
```

### Repository Pattern (13 Focused Repositories)
| Repository | Purpose |
|------------|---------|
| `UserRepository` | User account management |
| `OAuthTokenRepository` | OAuth token storage (tenant-scoped) |
| `ApiKeyRepository` | API key management |
| `UsageRepository` | Usage tracking and analytics |
| `A2ARepository` | Agent-to-agent management |
| `ProfileRepository` | User profiles and goals |
| `InsightRepository` | AI-generated insights |
| `AdminRepository` | Admin token management |
| `TenantRepository` | Multi-tenant management |
| `OAuth2ServerRepository` | OAuth 2.0 server functionality |
| `SecurityRepository` | Key rotation and audit |
| `NotificationRepository` | OAuth notifications |
| `FitnessConfigRepository` | Fitness configuration management |

### Accessor Pattern
```rust
let db = Database::new(database_url, encryption_key).await?;

// Access repositories via typed accessors
let user = db.users().get_by_id(user_id).await?;
let token = db.oauth_tokens().get(user_id, tenant_id, provider).await?;
let api_key = db.api_keys().get_by_key(key).await?;
```

### Connection String Detection
```rust
if url.starts_with("sqlite:") → SQLite
if url.starts_with("postgres://") → PostgreSQL
```

### Cursor Pagination
Keyset pagination using composite cursor (`created_at`, `id`) for consistent ordering:
- No duplicate results during data changes
- Stable pagination across pages
- Efficient for large datasets
- Cursor format: base64-encoded JSON with timestamp + id

---

## API KEYS & RATE LIMITING

### API Key Tiers
| Tier | Monthly Limit | Use Case |
|------|---------------|----------|
| Trial | 1,000 | 14-day evaluation |
| Starter | 10,000 | Small projects |
| Professional | 100,000 | Production apps |
| Enterprise | Unlimited | High-volume |

### Rate Limiting
- Token bucket algorithm
- Per-key and per-user limits
- 429 response with `Retry-After` header

---

## SDK (TypeScript)

### Architecture
```
MCP Client (Claude Desktop)
    ↓ stdio (JSON-RPC)
pierre-mcp-client (npm package)
    ↓ HTTP (JSON-RPC)
Pierre MCP Server (Rust)
```

### Installation & Usage
```bash
# Via npx (recommended)
npx -y pierre-mcp-client@next --server http://localhost:8081
```

### Key Features
- Automatic OAuth2 token management (browser-based auth flow)
- Token refresh handling
- Secure credential storage via system keychain
- stdio ↔ HTTP protocol bridge

### Type Generation
- 47 tool parameter interfaces auto-generated from Rust schemas
- `ToolParamsMap` - maps tool names to parameter types
- `ToolName` - union type of all 47 tool names
- Run: `npm run generate-types` (requires server on port 8081)

### MCP Protocol Compliance

**Supported Versions**: 2025-06-18 (primary), 2025-03-26, 2024-11-05

**Core Features**:
- ✅ Structured tool output
- ✅ OAuth 2.1 authentication
- ✅ Elicitation support
- ✅ Enhanced security (CORS, Origin validation)
- ✅ Bearer token validation
- ✅ PKCE flow

**Advanced MCP Features**:

| Feature | Description |
|---------|-------------|
| **Sampling** | Bidirectional LLM requests via `SamplingPeer` |
| **Completion** | Argument auto-completion for tools |
| **Progress Reporting** | `ProgressTracker` with notification channels |
| **Cancellation** | `CancellationToken` for async operations |

### Sampling Integration
Two high-value intelligence tools use MCP sampling:
- `get_activity_intelligence` - AI-powered activity analysis
- `generate_recommendations` - Personalized coaching advice

### SDK Files
| File | Purpose |
|------|---------|
| `sdk/src/bridge.ts` | stdio → HTTP bridge |
| `sdk/src/cli.ts` | CLI wrapper for MCP hosts |
| `sdk/src/types.ts` | Auto-generated tool types |
| `sdk/src/secure-storage.ts` | OS keychain integration |

---

## FRONTEND DASHBOARD

### Overview
React/TypeScript admin dashboard for managing Pierre MCP Server.

### Features
- **Dashboard Overview**: API key usage and system metrics
- **User Management**: User approval, registration, tenant management
- **Connections**: A2A clients and API Keys management
- **MCP Tokens**: Token generation and management
- **Rate Limiting**: Monitor and configure API rate limits
- **A2A Monitoring**: Agent-to-Agent communication tracking
- **Real-time Updates**: WebSocket-based live data
- **Usage Analytics**: Request patterns and tool usage breakdown
- **Role-based Access**: Admin impersonation and permissions

### User Roles

| Feature | Regular User | Admin |
|---------|--------------|-------|
| Dashboard Overview | Own data | Platform-wide |
| API Keys | Own keys | + System-wide keys |
| Connected Apps (A2A) | Own clients | All clients |
| MCP Tokens | Own tokens | Own tokens |
| Analytics | Own usage | All users |
| User Management | Hidden | Full access |
| Impersonation | No | Yes |

### User Onboarding Flow
```
Register → Pending Approval → Admin Approves → Active User
                 ↓
         Admin Rejects → Rejected (End)
```

### Tech Stack
| Technology | Purpose |
|------------|---------|
| React 19.1 | UI framework |
| TypeScript 5.8 | Type safety |
| Vite 6.4 | Build tooling |
| TailwindCSS 3.4 | Styling |
| @tanstack/react-query | Server state |
| Chart.js | Analytics charts |
| Vitest | Unit testing |
| Playwright | E2E testing (282 tests) |

### Frontend Project Structure
```
frontend/
├── src/
│   ├── components/     # React components (20+)
│   ├── contexts/       # React context providers
│   ├── services/       # API service layer
│   └── types/          # TypeScript types
├── e2e/                # Playwright tests
└── dist/               # Production build
```

### API Integration
- Base URL: `http://localhost:8081`
- Authentication: JWT in localStorage
- Key endpoints: `/api/auth/*`, `/api/keys/*`, `/api/admin/*`, `/api/dashboard/*`

### Brand Identity
Three-pillar color system:
| Pillar | Color | Hex | Usage |
|--------|-------|-----|-------|
| Activity | Emerald | `#10B981` | Movement, fitness |
| Nutrition | Amber | `#F59E0B` | Food, fuel |
| Recovery | Indigo | `#6366F1` | Rest, sleep |

Primary colors: Pierre Violet (`#7C3AED`), Pierre Cyan (`#06B6D4`)

---

## TESTING

### Synthetic Data
- Deterministic test data generation
- Seeded random for reproducibility
- Realistic fitness data patterns

### Test Categories
```bash
cargo test --test mcp_protocol_test      # MCP protocol
cargo test --test oauth_test             # OAuth flows
cargo test --test security_test          # Security
cargo test --test intelligence_test      # Algorithms
```

### Synthetic Provider
- In-memory provider for tests
- No external API calls
- Configurable responses

---

## PERFORMANCE

### Binary Size
- Target: <50MB for pierre-mcp-server
- Feature flags minimize unused code

### Connection Pooling
- SQLite: Single connection with WAL mode
- PostgreSQL: Configurable pool (min/max connections)

### Caching
- LRU cache for frequently accessed data
- Redis support for distributed deployments
- Cache invalidation on writes

---

## ADDING NEW TOOLS

### 1. Define Tool Schema
```rust
// src/protocols/universal/tool_registry.rs
ToolId::MyNewTool => ToolDefinition {
    name: "my_new_tool",
    description: "Does something useful",
    input_schema: json!({
        "type": "object",
        "properties": {
            "param1": {"type": "string", "description": "First param"}
        },
        "required": ["param1"]
    }),
}
```

### 2. Implement Handler
```rust
// src/protocols/universal/handlers/my_handler.rs
pub async fn handle_my_new_tool(
    ctx: &HandlerContext,
    params: MyNewToolParams,
) -> AppResult<serde_json::Value> {
    // Implementation
    Ok(json!({"result": "success"}))
}
```

### 3. Register in Executor
```rust
// src/protocols/universal/executor.rs
ToolId::MyNewTool => handle_my_new_tool(ctx, params).await,
```

### 4. Generate SDK Types
```bash
node scripts/generate-sdk-types.js
```

---

## KEY FILES REFERENCE

| Feature | File Path |
|---------|-----------|
| Library root | src/lib.rs |
| Main binary | src/bin/pierre-mcp-server.rs |
| Error types | src/errors.rs |
| MCP protocol | src/mcp/protocol.rs |
| Tool registry | src/protocols/universal/tool_registry.rs |
| Tool handlers | src/protocols/universal/handlers/ |
| Database factory | src/database_plugins/factory.rs |
| Auth manager | src/auth.rs |
| JWT handling | src/admin/jwks.rs |
| OAuth server | src/oauth2_server/endpoints.rs |
| OAuth client | src/oauth2_client/ |
| Providers | src/providers/ |
| Intelligence | src/intelligence/ |
| SDK bridge | sdk/src/bridge.ts |
| Type generation | scripts/generate-sdk-types.js |

---

## COMMON PATTERNS

### Arc Clone for Async Handlers
```rust
let db = Arc::clone(&resources.database);
tokio::spawn(async move {
    db.query(...).await
});
```

### Error Propagation
```rust
async fn fetch_user(id: &str) -> AppResult<User> {
    let user = database.get_user(id).await?;  // Auto-converts DatabaseError to AppError
    Ok(user)
}
```

### Feature Flags
```rust
#[cfg(feature = "postgresql")]
pub fn postgres_specific() { ... }

#[cfg(feature = "sqlite")]
pub fn sqlite_specific() { ... }
```

### Conditional Compilation
```toml
[features]
default = ["sqlite"]
sqlite = []
postgresql = ["sqlx/postgres"]
testing = []
```

---

## SECURITY

### Security Layers
1. **Transport**: HTTPS/TLS 1.3
2. **Authentication**: JWT tokens, API keys
3. **Authorization**: Tenant-based RBAC
4. **Encryption**: Two-tier key management (MEK encrypts DEKs, DEKs encrypt user data)
5. **Rate limiting**: Token bucket per tenant

### Atomic Operations (TOCTOU Prevention)
- Refresh token consumption: atomic check-and-revoke
- Prevents race conditions in token exchange
- Database-level atomicity guarantees

### PII Redaction
Middleware removes sensitive data from logs and responses:
| Field | Redacted As |
|-------|-------------|
| Email | `***@***.***` |
| Token | `[REDACTED-<type>]` |
| UUID | `[REDACTED-UUID]` |

Enabled via `LOG_FORMAT=json` for structured logging.

### Authentication
- RS256 JWT tokens (24h expiry)
- CSRF protection for web endpoints
- Rate limiting on auth endpoints

### Multi-Tenancy
- Strict tenant isolation at database level
- No cross-tenant data access possible
- Tenant ID in every query

---

## DEPLOYMENT

### Docker
```bash
docker build -t pierre-mcp-server .
docker run -p 8081:8081 -e DATABASE_URL=... pierre-mcp-server
```

### Environment Detection
```rust
pub enum Environment {
    Development,  // Relaxed CORS, debug logging
    Production,   // Strict security, minimal logging
    Testing,      // Test utilities enabled
}
```

### Health Check
```bash
curl http://localhost:8081/health
# {"status": "healthy", "version": "0.2.0"}
```

---

## TROUBLESHOOTING

### Common Errors

**"Entity not found"**
- Check entity ID exists
- Verify tenant context matches

**"Tenant isolation violation"**
- User trying to access another tenant's data
- Check JWT tenant_id claim

**"Rate limit exceeded"**
- Reduce request frequency
- Upgrade API key tier
- Check `Retry-After` header

**"Authentication required"**
- Include `Authorization: Bearer <token>` header
- Check token not expired
- Verify token signature

---

## VERSION INFO

- **Current Version**: 0.2.x
- **Rust Version**: 1.91.0+
- **Node.js Version**: 24.0.0+ (SDK)
- **Supported Databases**: SQLite, PostgreSQL
