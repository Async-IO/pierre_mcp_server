# Getting Started

Pierre MCP Server setup and configuration.

## Installation

```bash
git clone https://github.com/Async-IO/pierre_mcp_server.git
cd pierre_mcp_server
cargo build --release
```

## Database Setup

```bash
./scripts/fresh-start.sh
cargo run --bin pierre-mcp-server
```

Server runs on:
- Port 8080: MCP protocol endpoint
- Port 8081: HTTP REST API

## User Registration

```bash
curl -X POST http://localhost:8081/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "password123",
    "display_name": "User Name"
  }'
```

## Authentication

```bash
# Get JWT token
curl -X POST http://localhost:8081/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "password123"
  }'

# Create API key for MCP access
curl -X POST http://localhost:8081/api/keys \
  -H "Authorization: Bearer JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "MCP Client",
    "tier": "professional",
    "description": "API key for MCP protocol",
    "rate_limit_requests": 10000,
    "expires_in_days": 90
  }'
```

## OAuth Configuration

Configure OAuth credentials through the API after user approval:

```bash
# Store Strava OAuth credentials for the authenticated user
curl -X POST http://localhost:8081/oauth/credentials \
  -H "Authorization: Bearer USER_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "provider": "strava",
    "client_id": "your_strava_client_id",
    "client_secret": "your_strava_client_secret",
    "redirect_uri": "http://localhost:8081/auth/strava/callback"
  }'
```

For programmatic access, use the JavaScript SDK:

```javascript
const { PierreClientSDK } = require('../sdk/pierre-client-sdk');

const sdk = new PierreClientSDK('http://localhost:8081');
await sdk.login('user@example.com', 'password');

// Configure OAuth credentials
await sdk.setOAuthCredentials('strava', {
  clientId: 'your_strava_client_id',
  clientSecret: 'your_strava_client_secret',
  redirectUri: 'http://localhost:8081/auth/strava/callback'
});
```

## MCP Protocol Testing

Test MCP endpoints with API key authentication:

```bash
# List available tools
curl -X POST http://localhost:8080/mcp \
  -H "Authorization: API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/list",
    "params": {},
    "id": 1
  }'

# Call a tool
curl -X POST http://localhost:8080/mcp \
  -H "Authorization: API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "get_activities",
      "arguments": {"provider": "strava", "limit": 5}
    },
    "id": 2
  }'
```

**Important Notes:**
- **Protocol Version**: Use `2024-11-05` for MCP protocol version
- **Transport Ports**: stdio (same process), HTTP (port 8080)
- **Authentication**: API key required for all tool calls
- **Error Handling**: Follow JSON-RPC 2.0 error format
- **Rate Limiting**: Applied per user account, not per connection

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

## Configuration

### Environment Variables

Pierre supports multiple configuration methods in order of precedence:

1. **Command line arguments** (highest priority)
2. **Environment variables**
3. **Configuration files**
4. **Default values** (lowest priority)

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
# Example for reducing SQL query noise:
# RUST_LOG=info,sqlx::query=warn     # App logs at info, SQL queries only on warnings/errors
# RUST_LOG=debug,sqlx::query=trace   # Development: debug logs but SQL at trace level
LOG_FORMAT=json                      # Log format (json, text)
```

#### OAuth Provider Configuration

OAuth providers are configured per user in the database. Each user stores their own OAuth app credentials:

```python
import sqlite3
conn = sqlite3.connect("data/users.db")
conn.execute("""
    INSERT INTO user_oauth_app_credentials 
    (id, user_id, provider, client_id, client_secret, redirect_uri, created_at, updated_at)
    VALUES (?, ?, 'strava', ?, ?, ?, datetime('now'), datetime('now'))
""", [
    "unique_id", "your_user_id", "your_client_id", 
    "your_client_secret", "http://localhost:8081/auth/strava/callback"
])
conn.commit()
```

#### Weather Integration
```bash
# Weather API is still configured via environment variable
OPENWEATHER_API_KEY=your_openweather_api_key        # Required for weather analysis
```

## Authentication

### Overview

The Pierre MCP Server supports multiple authentication methods:

- **JWT Tokens**: For user authentication in web applications
- **API Keys**: For production integrations and B2B customers  
- **A2A Authentication**: For agent-to-agent communication
- **OAuth2 Flow**: For fitness provider connections (Strava, Fitbit, etc.)

### JWT Authentication

#### JWT Token Structure

JWT tokens include the following claims:

```json
{
  "sub": "user_12345",           // User ID (subject)
  "email": "user@example.com",   // User email
  "iat": 1705123456,             // Issued at (Unix timestamp)
  "exp": 1705209856,             // Expires at (Unix timestamp)
  "iss": "pierre-mcp-server",    // Issuer
  "aud": "pierre-api",           // Audience
  "permissions": [               // User permissions
    "read_activities",
    "write_goals",
    "admin_access"
  ]
}
```

#### Getting a JWT Token

**1. User Registration and Login**

```bash
# Register new user
curl -X POST http://localhost:8081/auth/register \\
  -H "Content-Type: application/json" \\
  -d '{
    "email": "user@example.com",
    "password": "secure_password123",
    "display_name": "John Doe"
  }'

# Login to get JWT token
curl -X POST http://localhost:8081/auth/login \\
  -H "Content-Type: application/json" \\
  -d '{
    "email": "user@example.com",
    "password": "secure_password123"
  }'
```

### API Key Authentication

API keys are recommended for production integrations and provide better rate limiting and monitoring capabilities.

#### Creating API Keys

```bash
# Create an API key (requires admin JWT token)
curl -X POST http://localhost:8081/api/admin/api-keys \\
  -H "Authorization: Bearer $ADMIN_JWT_TOKEN" \\
  -H "Content-Type: application/json" \\
  -d '{
    "name": "Production Integration",
    "description": "API key for production fitness app",
    "expires_in_days": 365
  }'
```

#### Using API Keys

```bash
# Use API key in requests
curl -X GET http://localhost:8081/api/activities \\
  -H "X-API-Key: pierre_12345678-abcd-efgh-ijkl-1234567890ab"
```

### OAuth2 Setup

#### Strava OAuth Setup

1. **Create Strava Application**:
   - Go to https://www.strava.com/settings/api
   - Create a new API application
   - Set redirect URI to: `http://localhost:8081/auth/strava/callback`

2. **Store OAuth Credentials in Database**:
   ```python
   import sqlite3
   conn = sqlite3.connect("data/users.db")
   conn.execute("""
       INSERT INTO user_oauth_app_credentials 
       (id, user_id, provider, client_id, client_secret, redirect_uri, created_at, updated_at)
       VALUES (?, ?, 'strava', ?, ?, ?, datetime('now'), datetime('now'))
   """, [
       "unique_id", "your_user_id", "your_strava_client_id", 
       "your_strava_client_secret", "http://localhost:8081/auth/strava/callback"
   ])
   conn.commit()
   ```

3. **Test OAuth Flow**:
   ```bash
   # Get OAuth authorization URL
   curl -X GET "http://localhost:8081/auth/strava?user_id=user_123"
   
   # Visit the returned URL in browser to authorize
   # User will be redirected back with authorization code
   ```

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
| `diagnose-weather-api` | Weather API diagnostics | Troubleshoot weather issues |
| `serve-docs` | Local documentation server | Documentation development |

### Running Binaries

All binaries are available via cargo:

```bash
# Core server
cargo run --bin pierre-mcp-server -- --help

# Setup utilities
cargo run --bin auth-setup -- --help
cargo run --bin admin-setup -- --help

# Testing utilities
cargo run --bin diagnose-weather-api

# Documentation server
cargo run --bin serve-docs
```

## Next Steps

1. **For MCP Integration**: See [API Reference](API_REFERENCE.md) for available tools and endpoints
2. **For Production Deployment**: Check [Deployment Guide](DEPLOYMENT_GUIDE.md)
3. **For Database Setup**: Review [Database Guide](DATABASE_GUIDE.md)

## Troubleshooting

### Common Issues

1. **Port conflicts**: Change `MCP_PORT` and `HTTP_PORT` in environment variables
2. **Database connection errors**: Verify `DATABASE_URL` and ensure database is accessible
3. **OAuth errors**: Check client IDs/secrets and redirect URIs match provider settings
4. **JWT token issues**: Ensure `JWT_SECRET` is at least 32 characters long

### Logging Configuration

**Standard Log Levels:**
```bash
RUST_LOG=error    # Only errors
RUST_LOG=warn     # Warnings and errors
RUST_LOG=info     # General information (recommended for production)
RUST_LOG=debug    # Detailed debugging information
RUST_LOG=trace    # Very verbose debugging
```

**Reducing SQL Query Noise:**
By default, sqlx logs all SQL queries at DEBUG level. To reduce log noise:

```bash
# Production: App info, SQL only on errors
RUST_LOG=info,sqlx::query=warn

# Development: App debug, SQL at trace level
RUST_LOG=debug,sqlx::query=trace

# Hide SQL queries completely
RUST_LOG=info,sqlx=warn
```

**Module-Specific Logging:**
```bash
# Fine-tune specific modules
RUST_LOG=info,pierre_mcp_server::mcp=debug,reqwest=warn
```

### Getting Help

- Check logs with appropriate `RUST_LOG` level for detailed error information
- Use health check endpoint: `http://localhost:8081/health`
- Run diagnostic utilities: `cargo run --bin diagnose-weather-api`