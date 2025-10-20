# configuration

## environment variables

pierre configured entirely via environment variables. no config files.

### required variables

```bash
# database
DATABASE_URL="sqlite:./data/pierre.db"  # or postgresql://...

# encryption (generate: openssl rand -base64 32)
PIERRE_MASTER_ENCRYPTION_KEY="<base64_encoded_32_bytes>"
```

### server configuration

```bash
# network
HTTP_PORT=8081                    # server port (default: 8081)
HOST=127.0.0.1                    # bind address (default: 127.0.0.1)

# logging
RUST_LOG=info                     # log level (error, warn, info, debug, trace)
LOG_FORMAT=json                   # json or pretty (default: pretty)
```

### authentication

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

### fitness providers

#### strava

```bash
STRAVA_CLIENT_ID=your_id
STRAVA_CLIENT_SECRET=your_secret
STRAVA_REDIRECT_URI=http://localhost:8081/oauth/callback/strava
```

get credentials: https://www.strava.com/settings/api

#### garmin

```bash
GARMIN_CLIENT_ID=your_consumer_key
GARMIN_CLIENT_SECRET=your_consumer_secret
GARMIN_REDIRECT_URI=http://localhost:8081/oauth/callback/garmin
```

get credentials: https://developer.garmin.com/

#### fitbit

```bash
FITBIT_CLIENT_ID=your_id
FITBIT_CLIENT_SECRET=your_secret
FITBIT_REDIRECT_URI=http://localhost:8081/oauth/callback/fitbit
```

get credentials: https://dev.fitbit.com/apps

#### openweather (optional)

for weather-based recommendations:
```bash
OPENWEATHER_API_KEY=your_api_key
```

get key: https://openweathermap.org/api

### database configuration

#### sqlite (development)

```bash
DATABASE_URL="sqlite:./data/pierre.db"
```

creates database file at path if not exists.

#### postgresql (production)

```bash
DATABASE_URL="postgresql://user:pass@localhost:5432/pierre"

# connection pool
POSTGRES_MAX_CONNECTIONS=20       # max pool size (default: 20)
POSTGRES_MIN_CONNECTIONS=2        # min pool size (default: 2)
POSTGRES_ACQUIRE_TIMEOUT=30       # connection timeout seconds (default: 30)
```

### cache configuration

```bash
# in-memory lru cache
CACHE_MAX_ENTRIES=10000           # max cached items (default: 10,000)
CACHE_CLEANUP_INTERVAL_SECS=300   # cleanup interval (default: 300)

# redis cache (future support)
# REDIS_URL=redis://localhost:6379
```

### rate limiting

```bash
# global defaults
RATE_LIMIT_REQUESTS_PER_MINUTE=60
RATE_LIMIT_BURST=10

# per-tier overrides
API_TIER_FREE_LIMIT=100           # requests per day
API_TIER_PROFESSIONAL_LIMIT=10000
API_TIER_ENTERPRISE_LIMIT=0       # unlimited (0 = no limit)
```

### multi-tenancy

```bash
# tenant isolation
TENANT_MAX_USERS=100              # max users per tenant
TENANT_MAX_PROVIDERS=5            # max connected providers per tenant

# default features per tenant
TENANT_DEFAULT_FEATURES="activity_analysis,goal_tracking"
```

### security

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

## fitness configuration

user-specific fitness parameters managed via mcp tools or rest api.

### configuration profiles

predefined fitness profiles:

- `beginner`: conservative zones, longer recovery
- `intermediate`: standard zones, moderate training
- `advanced`: aggressive zones, high training load
- `elite`: performance-optimized zones
- `custom`: user-defined parameters

### fitness parameters

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

### training zones

automatically calculated based on profile:

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

### updating configuration

via mcp tool:
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

via rest api:
```bash
curl -X PUT http://localhost:8081/api/configuration/user \
  -H "Authorization: Bearer <jwt>" \
  -H "Content-Type: application/json" \
  -d '{
    "profile": "elite",
    "vo2_max": 60.0
  }'
```

### configuration catalog

get all available parameters:
```bash
curl -H "Authorization: Bearer <jwt>" \
  http://localhost:8081/api/configuration/catalog
```

response describes each parameter:
- type (number, boolean, enum)
- valid range
- default value
- description

## using direnv

recommended for local development.

### setup

```bash
brew install direnv

# add to shell (~/.zshrc or ~/.bashrc)
eval "$(direnv hook zsh)"  # or bash

# in project directory
direnv allow
```

### .envrc file

edit `.envrc` in project root:
```bash
# development overrides
export RUST_LOG=debug
export HTTP_PORT=8081
export DATABASE_URL=sqlite:./data/pierre.db

# provider credentials (dev)
export STRAVA_CLIENT_ID=dev_client_id
export STRAVA_CLIENT_SECRET=dev_secret
export STRAVA_REDIRECT_URI=http://localhost:8081/oauth/callback/strava

# load from file
if [ -f .env.local ]; then
  source .env.local
fi
```

direnv automatically loads/unloads environment when entering/leaving directory.

### .env.local (gitignored)

store secrets in `.env.local`:
```bash
# never commit this file
export PIERRE_MASTER_ENCRYPTION_KEY="<generated_key>"
export STRAVA_CLIENT_SECRET="<real_secret>"
```

## production deployment

### environment file

create `/etc/pierre/environment`:
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
STRAVA_REDIRECT_URI=https://api.example.com/oauth/callback/strava

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

### systemd service

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

### docker

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

run:
```bash
docker run -d \
  --name pierre \
  -p 8081:8081 \
  -e DATABASE_URL=postgresql://... \
  -e PIERRE_MASTER_ENCRYPTION_KEY=... \
  pierre:latest
```

## validation

check configuration at startup:
```bash
RUST_LOG=info cargo run --bin pierre-mcp-server
```

logs show:
- loaded environment variables
- database connection status
- enabled features
- configured providers
- listening address

## troubleshooting

### missing environment variables

server fails to start. check required variables set:
```bash
echo $DATABASE_URL
echo $PIERRE_MASTER_ENCRYPTION_KEY
```

### invalid database url

- sqlite: ensure directory exists
- postgresql: verify connection string, credentials, database exists

### provider oauth fails

- verify redirect uri exactly matches environment variable
- ensure uri accessible from browser (not `127.0.0.1` for remote)
- check provider console for correct credentials

### port conflicts

change http_port:
```bash
export HTTP_PORT=8082
```

### encryption key errors

regenerate:
```bash
openssl rand -base64 32
```

must be exactly 32 bytes (base64 encoded = 44 characters).

## references

all configuration constants: `src/constants/mod.rs`
fitness profiles: `src/configuration/profiles.rs`
database setup: `src/database_plugins/`
