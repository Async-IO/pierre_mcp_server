# Setup Guide

This guide covers installation, OAuth configuration, and initial setup of Pierre Fitness API.

## Quick Start

### Local Development

```bash
cargo build --release
```

### Docker Deployment

The server supports Docker deployment with direnv (.envrc) integration:

1. **Setup Environment Variables**:
   ```bash
   # Copy the example to .envrc
   cp .env.example .envrc
   # Edit .envrc with your OAuth credentials
   # If using direnv: direnv allow
   ```

2. **Using Docker Compose with direnv**:
   ```bash
   # Use the helper script that loads .envrc
   ./docker-compose-with-envrc.sh up
   
   # Or manually export variables and run docker-compose
   eval $(cat .envrc | grep export) && docker-compose up
   ```

3. **Production Deployment**:
   ```bash
   # Build and run in production mode
   docker-compose -f docker-compose.prod.yml up -d
   ```

4. **Health Checks**: Available at `http://localhost:8081/health`

**Note**: The Docker setup includes automatic health checks, backup services, and optional SQLite web interface for development.

## Available Binaries

Pierre includes several utility binaries for setup, testing, and administration:

### Core Binaries

| Binary | Purpose | Usage |
|--------|---------|-------|
| `pierre-mcp-server` | Main server binary | Production deployment |
| `auth-setup` | OAuth credential setup | Initial provider configuration |
| `admin-setup` | Admin token management | Generate/manage admin tokens |

### Testing & Utility Binaries

| Binary | Purpose | Usage |
|--------|---------|-------|
| `test-with-data` | Test with real fitness data | Development/debugging |
| `test-weather-integration` | Test weather API integration | Weather setup validation |
| `test-real-weather` | Test with OpenWeatherMap API | Real weather API testing |
| `diagnose-weather-api` | Weather API diagnostics | Troubleshoot weather issues |
| `test-intelligence-for-longest-run` | Test activity intelligence | AI analysis validation |
| `test-location-intelligence` | Test location detection | GPS/location testing |
| `test-oauth-callback` | Test OAuth callback flow | OAuth integration testing |
| `serve-docs` | Local documentation server | Documentation development |

### Activity Analysis Utilities

| Binary | Purpose | Usage |
|--------|---------|-------|
| `find-2024-longest-run` | Find longest run in 2024 | Data analysis |
| `find-2025-longest-run` | Find longest run in 2025 | Data analysis |
| `find-consecutive-10k-runs` | Find consecutive 10k+ runs | Training pattern analysis |
| `check-longest-run-gps` | Check GPS data for longest run | GPS validation |

### Running Binaries

All binaries are available via cargo:

```bash
# Core server
cargo run --bin pierre-mcp-server -- --help

# Setup utilities
cargo run --bin auth-setup -- --help
cargo run --bin admin-setup -- --help

# Testing utilities
cargo run --bin test-weather-integration
cargo run --bin diagnose-weather-api

# Activity analysis
cargo run --bin find-2024-longest-run
```

### Binary Documentation

**pierre-mcp-server**
```bash
cargo run --bin pierre-mcp-server -- --help

# Single-tenant mode (personal use)
cargo run --bin pierre-mcp-server -- --single-tenant --port 8080

# Multi-tenant mode (cloud deployment)
cargo run --bin pierre-mcp-server -- --database-url sqlite:./users.db --port 8081
```

**auth-setup**
```bash
# Setup Strava OAuth
cargo run --bin auth-setup -- strava --client-id CLIENT_ID --client-secret CLIENT_SECRET

# Setup Fitbit OAuth  
cargo run --bin auth-setup -- fitbit --client-id CLIENT_ID --client-secret CLIENT_SECRET
```

**admin-setup**
```bash
# Generate admin token
cargo run --bin admin-setup -- generate-token \
  --service "admin_service" \
  --permissions "provision_keys,revoke_keys" \
  --expires-in-days 90

# List admin tokens
cargo run --bin admin-setup -- list-tokens

# Rotate admin token
cargo run --bin admin-setup -- rotate-token TOKEN_ID
```

## OAuth2 Setup

### Strava

1. Create a Strava application at https://www.strava.com/settings/api
2. Note your Client ID and Client Secret
3. Run the auth setup tool:

```bash
cargo run --bin auth-setup -- strava \
  --client-id YOUR_CLIENT_ID \
  --client-secret YOUR_CLIENT_SECRET
```

4. Follow the browser prompts to authorize the application
5. The tool will save your tokens to the config file

### Fitbit

1. Create a Fitbit application at https://dev.fitbit.com/apps/new
   - **Application Type**: Personal
   - **OAuth 2.0 Application Type**: Confidential
   - **Redirect URL**: `http://localhost:8080/callback` (or your callback URL)
   - **Default Access Type**: Read Only
2. Note your Client ID and Client Secret
3. Run the auth setup tool:

```bash
cargo run --bin auth-setup -- fitbit \
  --client-id YOUR_CLIENT_ID \
  --client-secret YOUR_CLIENT_SECRET
```

4. Follow the browser prompts to authorize the application
5. The tool will save your tokens to the config file

**Note**: Fitbit requires explicit scopes. The server requests `activity`, `profile`, and `sleep` permissions.

## Configuration

The server supports multiple configuration methods:

### Using direnv (.envrc):
```bash
# Copy the example file
cp .envrc.example .envrc

# Edit with your credentials
vim .envrc

# Allow direnv to load the file
direnv allow
```

### Using .env file:
```env
# Strava Configuration
STRAVA_CLIENT_ID=your_strava_client_id
STRAVA_CLIENT_SECRET=your_strava_client_secret
STRAVA_ACCESS_TOKEN=your_strava_access_token
STRAVA_REFRESH_TOKEN=your_strava_refresh_token

# Fitbit Configuration
FITBIT_CLIENT_ID=your_fitbit_client_id
FITBIT_CLIENT_SECRET=your_fitbit_client_secret
FITBIT_ACCESS_TOKEN=your_fitbit_access_token
FITBIT_REFRESH_TOKEN=your_fitbit_refresh_token

# Weather Configuration (optional)
OPENWEATHER_API_KEY=your_openweather_api_key
```

### Using config.toml:
```toml
[providers.strava]
auth_type = "oauth2"
client_id = "your_strava_client_id"
client_secret = "your_strava_client_secret"
access_token = "your_strava_access_token"
refresh_token = "your_strava_refresh_token"

[providers.fitbit]
auth_type = "oauth2"
client_id = "your_fitbit_client_id"
client_secret = "your_fitbit_client_secret"
access_token = "your_fitbit_access_token"
refresh_token = "your_fitbit_refresh_token"
```

## Usage

### Single-Tenant Mode (Personal Use)

```bash
# Run in single-tenant mode (default, backwards compatible)
cargo run --bin pierre-mcp-server -- --single-tenant

# Run with custom port
cargo run --bin pierre-mcp-server -- --single-tenant --port 9000

# Run with custom config file
cargo run --bin pierre-mcp-server -- --single-tenant --config /path/to/config.toml
```

### Multi-Tenant Mode (Cloud Deployment)

```bash
# Run in multi-tenant mode with authentication
cargo run --bin pierre-mcp-server

# Specify database and authentication settings
cargo run --bin pierre-mcp-server -- \
  --database-url "sqlite:./users.db" \
  --token-expiry-hours 24 \
  --port 8080

# Use custom encryption and JWT secret files
cargo run --bin pierre-mcp-server -- \
  --encryption-key-file ./custom-encryption.key \
  --jwt-secret-file ./custom-jwt.secret
```

### Multi-Tenant Authentication Flow

1. **User Registration/Login**
   ```bash
   # Register new user
   curl -X POST http://localhost:8081/auth/register \
     -H "Content-Type: application/json" \
     -d '{"email": "user@example.com", "password": "secure_password"}'

   # Login to get JWT token
   curl -X POST http://localhost:8081/auth/login \
     -H "Content-Type: application/json" \
     -d '{"email": "user@example.com", "password": "secure_password"}'
   ```

2. **Use JWT Token in MCP calls**
   ```json
   {
     "method": "authenticate",
     "params": {
       "jwt_token": "your_jwt_token_here"
     }
   }
   ```

## Adding to Claude or GitHub Copilot

### Single-Tenant Mode Configuration

Add to your MCP configuration:

```json
{
  "mcpServers": {
    "pierre-fitness": {
      "command": "path/to/pierre-mcp-server",
      "args": ["--single-tenant", "--port", "8080"]
    }
  }
}
```

Or for development:

```json
{
  "mcpServers": {
    "pierre-fitness-dev": {
      "command": "cargo",
      "args": ["run", "--bin", "pierre-mcp-server", "--", "--single-tenant", "--port", "8080"],
      "cwd": "/path/to/pierre_mcp_server"
    }
  }
}
```

### Multi-Tenant Mode Configuration

For cloud deployments, connect to your hosted multi-tenant server:

```json
{
  "mcpServers": {
    "pierre-fitness-cloud": {
      "command": "mcp-client",
      "args": ["--url", "https://your-cloud-server.com:8080", "--auth-type", "jwt"]
    }
  }
}
```