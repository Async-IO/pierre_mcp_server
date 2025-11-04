# architecture

Pierre Fitness Platform is a multi-protocol fitness data platform that connects AI assistants to strava, garmin, and fitbit. Single binary, single port (8081), multiple protocols.

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
- 36 fitness tools (activities, analysis, goals, sleep, recovery, nutrition, configuration)

### mcp implementation (`src/mcp/`)
- json-rpc 2.0 over http
- sse transport for streaming
- tool registry and execution

### oauth2 server (`src/oauth2_server/`)
- rfc 7591 dynamic client registration
- rfc 7636 pkce support
- jwt access tokens for mcp clients

### oauth2 client (`src/oauth2_client/`)
- pierre connects to fitness providers as oauth client
- pkce support for enhanced security
- automatic token refresh
- multi-tenant credential isolation

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

All fallible operations return `Result<T, E>` types:
```rust
pub async fn get_user(db: &Database, user_id: &str) -> Result<User, DatabaseError>
pub async fn fetch_activities(provider: &Strava) -> Result<Vec<Activity>, ProviderError>
pub async fn process_request(req: Request) -> Result<Response, AppError>
```

Errors propagate using `?` operator and convert automatically:
```rust
// DatabaseError converts to AppError
let user = db.get_user(user_id).await?;

// ProviderError converts to AppError
let activities = provider.fetch_activities().await?;
```

### error responses

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

Every request operates within tenant context:
- isolated data per tenant
- tenant-specific encryption keys
- custom rate limits
- feature flags

## key design decisions

### single port architecture
All protocols share port 8081. Simplified deployment, easier oauth2 callback handling, unified tls/security.

### dependency injection via serverresources
All components initialized once at startup, shared via `Arc<T>`. Eliminates resource creation anti-patterns.

### protocol abstraction
Business logic in `protocols::universal` works for both mcp and a2a. Write once, use everywhere.

### pluggable architecture
- database: sqlite (dev) or postgresql (prod)
- cache: in-memory lru or redis (distributed caching)
- tools: compile-time plugin system via `linkme`

## file structure

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
├── database_plugins/          # database backends
├── admin/                     # admin authentication
├── auth.rs                    # authentication
├── tenant/                    # multi-tenancy
├── tools/                     # tool execution engine
├── cache/                     # caching layer
├── config/                    # configuration
├── constants/                 # constants and defaults
├── crypto/                    # encryption utilities
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
Stateless server design. Scale by adding instances behind load balancer. Shared postgresql and optional redis for distributed cache.

### database sharding
- tenant-based sharding
- time-based partitioning for historical data
- provider-specific tables

### caching strategy
- health checks: 30s ttl
- mcp sessions: lru cache (10k entries)
- weather data: configurable ttl
- distributed cache: redis support for multi-instance deployments
- in-memory fallback: lru cache with automatic eviction

## plugin lifecycle

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

## algorithm dependency injection

Zero-overhead algorithm dispatch using rust enums instead of hardcoded formulas.

### design pattern

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

### benefits

**compile-time dispatch**: zero runtime overhead, inlined by llvm
**configuration flexibility**: runtime algorithm selection via environment variables
**defensive programming**: hybrid variants with automatic fallback
**testability**: each variant independently testable
**maintainability**: all algorithm logic in single enum file
**no magic strings**: type-safe algorithm selection

### algorithm types

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

### configuration integration

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

### enforcement

Automated validation ensures no hardcoded algorithms bypass the enum system.

Validation script: `scripts/validate-algorithm-di.sh`
Patterns defined: `scripts/validation-patterns.toml`

Checks for:
- hardcoded formulas (e.g., `220 - age`)
- magic numbers (e.g., `0.182258` in non-algorithm files)
- algorithmic logic outside enum implementations

Exclusions documented in validation patterns (e.g., tests, algorithm enum files).

Ci pipeline fails on algorithm di violations (zero tolerance).

### hybrid algorithms

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

### usage pattern

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

## pii redaction

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

## cursor pagination

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

## monitoring

Health endpoint: `GET /health`
- database connectivity
- provider availability
- system uptime
- cache statistics

Logs: structured json via tracing + opentelemetry
Metrics: request latency, error rates, provider api usage
