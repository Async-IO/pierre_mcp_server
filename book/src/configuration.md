<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

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

### MCP Tool Configuration

Control which MCP tools are available to tenants via environment variables and admin API.

#### Global Tool Disabling

```bash
# Comma-separated list of tool names to globally disable
PIERRE_DISABLED_TOOLS=analyze_sleep_quality,suggest_rest_day,track_sleep_trends,optimize_sleep_schedule
```

**Use cases**:
- Disable tools requiring premium provider integrations (e.g., sleep tools need WHOOP/Garmin)
- Temporarily disable tools during maintenance or outages
- Restrict tools based on deployment environment (dev vs production)

**precedence** (highest to lowest):
1. Global disabled (`PIERRE_DISABLED_TOOLS`) - overrides everything
2. Plan restrictions - subscription tier limits
3. Tenant overrides - per-tenant admin configuration
4. Tool catalog defaults - tool's built-in enabled state

#### Per-Tenant Tool Overrides

Admin API endpoints for managing tool availability per tenant:

```bash
# List tool catalog
GET /admin/tools/catalog

# Get effective tools for tenant
GET /admin/tools/tenant/{tenant_id}

# Enable/disable tool for tenant
POST /admin/tools/tenant/{tenant_id}/override
{
  "tool_name": "analyze_sleep_quality",
  "is_enabled": false,
  "reason": "Provider not configured"
}

# Remove override (revert to default)
DELETE /admin/tools/tenant/{tenant_id}/override/{tool_name}

# Get availability summary
GET /admin/tools/tenant/{tenant_id}/summary

# List globally disabled tools
GET /admin/tools/global-disabled
```

**required permissions**:
- `view_configuration`: read-only access to catalog and tenant tools
- `manage_configuration`: create/delete tool overrides

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
Fitness profiles: `src/config/profiles.rs`
Database setup: `src/database_plugins/`
