# Configuration Guide

This document provides comprehensive configuration options for Pierre MCP Server.

## Configuration Methods

Pierre supports multiple configuration methods in order of precedence:

1. **Command line arguments** (highest priority)
2. **Environment variables**
3. **Configuration files**
4. **Default values** (lowest priority)

## Environment Variables

### Complete Environment Variables Reference

#### Core Server Configuration
```bash
# Server Ports
MCP_PORT=8080                    # MCP protocol port
HTTP_PORT=8081                   # HTTP API port
HOST=127.0.0.1                   # Bind address

# Database
DATABASE_URL=sqlite:./data/users.db  # Database connection string
# DATABASE_URL=postgresql://user:pass@localhost:5432/pierre  # PostgreSQL alternative

# Security
JWT_SECRET=your-jwt-secret-here      # JWT signing secret (min 32 chars)
ENCRYPTION_KEY=your-32-byte-key      # AES-256 encryption key
TOKEN_EXPIRY_HOURS=24                # JWT token expiry (default: 24)

# Logging
RUST_LOG=info                        # Log level (error, warn, info, debug, trace)
LOG_FORMAT=json                      # Log format (json, text)
```

#### OAuth Provider Configuration
```bash
# Strava OAuth
STRAVA_CLIENT_ID=your_strava_client_id
STRAVA_CLIENT_SECRET=your_strava_client_secret
STRAVA_ACCESS_TOKEN=your_strava_access_token        # Optional: pre-configured token
STRAVA_REFRESH_TOKEN=your_strava_refresh_token      # Optional: pre-configured token

# Fitbit OAuth
FITBIT_CLIENT_ID=your_fitbit_client_id
FITBIT_CLIENT_SECRET=your_fitbit_client_secret
FITBIT_ACCESS_TOKEN=your_fitbit_access_token        # Optional: pre-configured token
FITBIT_REFRESH_TOKEN=your_fitbit_refresh_token      # Optional: pre-configured token
```

#### Weather Integration
```bash
# OpenWeatherMap API
OPENWEATHER_API_KEY=your_openweather_api_key        # Optional: for real weather data
WEATHER_CACHE_HOURS=24                              # Weather data cache duration
WEATHER_MOCK_FALLBACK=true                          # Use mock weather if API fails
```

#### CORS and Security
```bash
# CORS Configuration
CORS_ORIGINS=https://yourdomain.com,https://app.yourdomain.com
CORS_MAX_AGE=3600                                   # Preflight cache duration

# Rate Limiting
RATE_LIMIT_REQUESTS=1000                            # Default rate limit
RATE_LIMIT_WINDOW=3600                              # Rate limit window (seconds)

# Security Headers
ALLOWED_HOSTS=yourdomain.com,api.yourdomain.com
```

#### Email Configuration (for admin service)
```bash
# SMTP Configuration
SMTP_HOST=smtp.sendgrid.net
SMTP_PORT=587
SMTP_USERNAME=apikey
SMTP_PASSWORD=your_sendgrid_api_key
FROM_EMAIL=noreply@yourdomain.com
FROM_NAME="Pierre Fitness API"
```

#### Development and Testing
```bash
# Development Settings
DEV_MODE=true                                       # Enable development features
AUTO_APPROVE_KEYS=false                             # Auto-approve API key requests
MOCK_PROVIDERS=false                                # Use mock fitness providers

# Testing
TEST_USER_EMAIL=test@example.com                    # Default test user
SKIP_EMAIL_VERIFICATION=false                       # Skip email verification in tests
```

## Configuration Files

### fitness_config.toml

Complete configuration file structure:

```toml
# Server Configuration
[server]
mcp_port = 8080
http_port = 8081
host = "127.0.0.1"
single_tenant = false
database_url = "sqlite:./data/users.db"

# Security Configuration
[security]
jwt_secret = "your-jwt-secret-here"
encryption_key = "your-32-byte-encryption-key"
token_expiry_hours = 24
cors_origins = ["https://yourdomain.com"]
allowed_hosts = ["yourdomain.com"]

# Logging Configuration
[logging]
level = "info"
format = "json"
features = {
  location = true,
  spans = true,
  thread = true,
  target = false
}

# Rate Limiting Configuration
[rate_limiting]
default_requests = 1000
default_window = 3600
trial_requests = 1000
trial_window = 2592000  # 30 days

# OAuth Provider Configuration
[providers.strava]
auth_type = "oauth2"
client_id = "your_strava_client_id"
client_secret = "your_strava_client_secret"
access_token = "your_strava_access_token"      # Optional
refresh_token = "your_strava_refresh_token"    # Optional
redirect_uri = "http://localhost:8081/oauth/callback/strava"
scopes = ["read", "activity:read"]

[providers.fitbit]
auth_type = "oauth2"
client_id = "your_fitbit_client_id"
client_secret = "your_fitbit_client_secret"
access_token = "your_fitbit_access_token"      # Optional
refresh_token = "your_fitbit_refresh_token"    # Optional
redirect_uri = "http://localhost:8081/oauth/callback/fitbit"
scopes = ["activity", "profile", "sleep"]

# Weather API Configuration
[weather_api]
provider = "openweathermap"
api_key = "your_openweather_api_key"           # Optional
enabled = true
cache_duration_hours = 24
fallback_to_mock = true
mock_patterns = {
  realistic = true,
  seasonal = true,
  geographic = true
}

# Admin Service Configuration
[admin]
auto_approve_trial_keys = false
max_trial_keys_per_user = 1
trial_key_expiry_days = 14
email_verification_required = true
admin_dashboard_enabled = true

# Email Configuration
[email]
smtp_host = "smtp.sendgrid.net"
smtp_port = 587
smtp_username = "apikey"
smtp_password = "your_sendgrid_api_key"
from_email = "noreply@yourdomain.com"
from_name = "Pierre Fitness API"
email_templates_dir = "./templates"

# Development Configuration
[development]
dev_mode = false
mock_providers = false
auto_seed_data = false
debug_oauth = false
skip_ssl_verification = false
```

### Configuration File Locations

Pierre searches for configuration files in this order:

1. **Command line specified**: `--config /path/to/config.toml`
2. **Current directory**: `./fitness_config.toml`
3. **Home directory**: `~/.config/pierre/fitness_config.toml`
4. **System directory**: `/etc/pierre/fitness_config.toml`

### Environment-Specific Configurations

#### Development Configuration
```toml
[server]
single_tenant = true
host = "127.0.0.1"
database_url = "sqlite:./dev.db"

[development]
dev_mode = true
debug_oauth = true
auto_seed_data = true

[logging]
level = "debug"
format = "text"
```

#### Production Configuration
```toml
[server]
single_tenant = false
host = "0.0.0.0"
database_url = "postgresql://user:pass@db:5432/pierre"

[security]
cors_origins = ["https://yourdomain.com"]
allowed_hosts = ["yourdomain.com"]

[logging]
level = "info"
format = "json"

[rate_limiting]
default_requests = 10000
default_window = 3600
```

#### Testing Configuration
```toml
[server]
database_url = "sqlite::memory:"

[development]
mock_providers = true
skip_email_verification = true
auto_approve_keys = true

[admin]
auto_approve_trial_keys = true
```

## Command Line Arguments

### pierre-mcp-server Arguments

```bash
pierre-mcp-server [OPTIONS]

OPTIONS:
    --single-tenant                    Run in single-tenant mode
    --port <PORT>                      HTTP API port (default: 8081)
    --mcp-port <PORT>                  MCP protocol port (default: 8080)
    --database-url <URL>               Database connection string
    --config <PATH>                    Configuration file path
    --host <HOST>                      Bind address (default: 127.0.0.1)
    --jwt-secret <SECRET>              JWT signing secret
    --encryption-key <KEY>             AES encryption key
    --token-expiry-hours <HOURS>       JWT token expiry (default: 24)
    --log-level <LEVEL>                Log level (error, warn, info, debug, trace)
    --help                             Print help information
    --version                          Print version information
```

### Example Commands

```bash
# Development mode
pierre-mcp-server --single-tenant --log-level debug --port 8080

# Production mode
pierre-mcp-server \
  --database-url "postgresql://user:pass@db:5432/pierre" \
  --jwt-secret "production-secret-key" \
  --host "0.0.0.0" \
  --log-level info

# Custom configuration
pierre-mcp-server --config /etc/pierre/production.toml

# Docker deployment
pierre-mcp-server \
  --database-url "${DATABASE_URL}" \
  --jwt-secret "${JWT_SECRET}" \
  --host "0.0.0.0"
```

## Configuration Validation

### Required Settings

**Single-Tenant Mode:**
- Fitness provider OAuth credentials (Strava or Fitbit)

**Multi-Tenant Mode:**
- Database connection (`DATABASE_URL`)
- JWT secret (`JWT_SECRET`)
- Encryption key (`ENCRYPTION_KEY`)

### Optional but Recommended

- Weather API key for real weather data
- Email configuration for admin notifications
- CORS origins for web client security
- Rate limiting configuration for production

### Validation Commands

```bash
# Validate configuration
pierre-mcp-server --config fitness_config.toml --validate

# Test database connection
pierre-mcp-server --database-url "${DATABASE_URL}" --test-db

# Verify OAuth configuration
cargo run --bin auth-setup -- validate --provider strava
```

## Security Best Practices

### Production Security Checklist

- [ ] Use strong JWT secrets (minimum 32 characters)
- [ ] Set up proper CORS origins (avoid wildcards)
- [ ] Use encrypted database connections
- [ ] Configure rate limiting appropriately
- [ ] Set allowed hosts list
- [ ] Use environment variables for secrets (not config files)
- [ ] Rotate JWT secrets regularly
- [ ] Monitor configuration changes

### Secret Management

**Development:**
```bash
# Use .envrc with direnv
export JWT_SECRET="dev-secret-key-32-chars-minimum"
export ENCRYPTION_KEY="dev-encryption-key-32-bytes-min"
```

**Production:**
```bash
# Use proper secret management
export JWT_SECRET="$(cat /run/secrets/jwt_secret)"
export ENCRYPTION_KEY="$(cat /run/secrets/encryption_key)"

# Or use cloud secret managers
export JWT_SECRET="$(aws secretsmanager get-secret-value --secret-id pierre/jwt-secret --query SecretString --output text)"
```

## Troubleshooting Configuration

### Common Issues

**Configuration not found:**
```bash
# Check file permissions
ls -la ./fitness_config.toml

# Verify search paths
pierre-mcp-server --help  # Shows config search order
```

**Database connection fails:**
```bash
# Test database URL
pierre-mcp-server --database-url "${DATABASE_URL}" --test-db

# Check database permissions
sqlite3 ./data/users.db ".schema"  # For SQLite
psql "${DATABASE_URL}" -c "\dt"    # For PostgreSQL
```

**OAuth setup issues:**
```bash
# Validate OAuth configuration
cargo run --bin auth-setup -- validate

# Test OAuth callback
curl -i "http://localhost:8081/oauth/callback/strava?code=test&state=test"
```

### Configuration Debugging

Enable debug logging to troubleshoot configuration issues:

```bash
RUST_LOG=debug pierre-mcp-server --config fitness_config.toml
```

This will show:
- Configuration file loading order
- Environment variable resolution
- Database connection attempts
- OAuth provider initialization
- Rate limiting setup
- CORS configuration