# architecture

Pierre Fitness Platform is a multi-protocol fitness data platform that connects AI assistants to strava, garmin, and fitbit. single binary, single port (8081), multiple protocols.

## system design

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
│   fitness providers                     │
│   • strava                              │
│   • garmin                              │
│   • fitbit                              │
└─────────────────────────────────────────┘
```

## core components

### protocols layer (`src/protocols/`)
- `universal/` - protocol-agnostic business logic
- shared by mcp and a2a protocols
- 25 fitness tools (activities, analysis, goals)

### mcp implementation (`src/mcp/`)
- json-rpc 2.0 over http
- sse transport for streaming
- tool registry and execution

### oauth2 server (`src/oauth2/`)
- rfc 7591 dynamic client registration
- rfc 7636 pkce support
- jwt access tokens for mcp clients

### providers (`src/providers/`)
- trait-based fitness provider abstraction
- strava, garmin, fitbit implementations
- unified oauth token management

### intelligence (`src/intelligence/`)
- activity analysis and insights
- performance trend detection
- training load calculation
- goal feasibility analysis

### database (`src/database_plugins/`)
- pluggable backend (sqlite, postgresql)
- encrypted token storage
- multi-tenant isolation

### authentication (`src/auth.rs`)
- jwt token generation/validation
- api key management
- rate limiting per tenant

## error handling

Pierre Fitness Platform uses structured error types for precise error handling and propagation.

### error type hierarchy

```
AppError (src/errors.rs)
├── Database(DatabaseError)
├── Provider(ProviderError)
├── Authentication
├── Authorization
├── Validation
└── Internal
```

### error types

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

### error propagation

all fallible operations return `Result<T, E>` types:
```rust
pub async fn get_user(db: &Database, user_id: &str) -> Result<User, DatabaseError>
pub async fn fetch_activities(provider: &Strava) -> Result<Vec<Activity>, ProviderError>
pub async fn process_request(req: Request) -> Result<Response, AppError>
```

errors propagate using `?` operator and convert automatically:
```rust
// DatabaseError converts to AppError
let user = db.get_user(user_id).await?;

// ProviderError converts to AppError
let activities = provider.fetch_activities().await?;
```

### error responses

structured json error responses:
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

http status mapping:
- `DatabaseError::NotFound` → 404
- `ProviderError::ApiError` → 502/503
- `AppError::Validation` → 400
- `AppError::Authentication` → 401
- `AppError::Authorization` → 403

implementation: `src/errors.rs`, `src/database/errors.rs`, `src/providers/errors.rs`

## request flow

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
    ├─ providers (strava/garmin/fitbit)
    ├─ intelligence (analysis)
    └─ configuration
    ↓
[database + cache]
    ↓
response
```

## multi-tenancy

every request operates within tenant context:
- isolated data per tenant
- tenant-specific encryption keys
- custom rate limits
- feature flags

## key design decisions

### single port architecture
all protocols share port 8081. simplified deployment, easier oauth2 callback handling, unified tls/security.

### dependency injection via serverresources
all components initialized once at startup, shared via `Arc<T>`. eliminates resource creation anti-patterns.

### protocol abstraction
business logic in `protocols::universal` works for both mcp and a2a. write once, use everywhere.

### pluggable architecture
- database: sqlite (dev) or postgresql (prod)
- cache: in-memory lru (current), redis (future)
- tools: compile-time plugin system via `linkme`

## file structure

```
src/
├── bin/
│   ├── pierre-mcp-server.rs  # main binary
│   └── admin-setup.rs         # admin cli tool
├── protocols/
│   └── universal/             # shared business logic
├── mcp/                       # mcp protocol
├── oauth2/                    # oauth2 authorization server
├── a2a/                       # a2a protocol
├── providers/                 # fitness integrations
├── intelligence/              # activity analysis
├── database_plugins/          # database backends
├── auth.rs                    # authentication
├── oauth/                     # provider oauth
├── tenant/                    # multi-tenancy
├── tools/                     # tool execution engine
├── cache/                     # caching layer
└── lib.rs                     # public api
```

## security layers

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

## scalability

### horizontal scaling
stateless server design. scale by adding instances behind load balancer. shared postgresql + redis for state.

### database sharding
- tenant-based sharding
- time-based partitioning for historical data
- provider-specific tables

### caching strategy
- health checks: 30s ttl
- mcp sessions: lru cache (10k entries)
- weather data: configurable ttl
- future: redis for distributed cache

## plugin lifecycle

compile-time plugin system using `linkme` crate for intelligence modules.

plugins stored in `src/intelligence/plugins/`:
- zone-based intensity analysis
- training recommendations
- performance trend detection
- goal feasibility analysis

lifecycle hooks:
- `init()` - plugin initialization
- `execute()` - tool execution
- `validate()` - parameter validation
- `cleanup()` - resource cleanup

plugins registered at compile time via `#[distributed_slice(PLUGINS)]` attribute.
no runtime loading, zero overhead plugin discovery.

implementation: `src/intelligence/plugins/mod.rs`, `src/lifecycle/`

## pii redaction

middleware layer removes sensitive data from logs and responses.

redacted fields:
- email addresses
- passwords
- tokens (jwt, oauth, api keys)
- user ids
- tenant ids

redaction patterns:
- email: `***@***.***`
- token: `[REDACTED-<type>]`
- uuid: `[REDACTED-UUID]`

enabled via `LOG_FORMAT=json` for structured logging.
implementation: `src/middleware/redaction.rs`

## cursor pagination

keyset pagination using composite cursor (`created_at`, `id`) for consistent ordering.

benefits:
- no duplicate results during data changes
- stable pagination across pages
- efficient for large datasets

cursor format: base64-encoded json with timestamp (milliseconds) + id.

example:
```
cursor: "eyJ0aW1lc3RhbXAiOjE3MDAwMDAwMDAsImlkIjoiYWJjMTIzIn0="
decoded: {"timestamp":1700000000,"id":"abc123"}
```

endpoints using cursor pagination:
- `GET /admin/users/pending?cursor=<cursor>&limit=20`
- `GET /admin/users/active?cursor=<cursor>&limit=20`

implementation: `src/pagination/`, `src/database/users.rs:668-737`, `src/database_plugins/postgres.rs:378-420`

## monitoring

health endpoint: `GET /health`
- database connectivity
- provider availability
- system uptime
- cache statistics

logs: structured json via tracing + opentelemetry
metrics: request latency, error rates, provider api usage
