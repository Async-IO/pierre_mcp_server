# Getting Started with Pierre Fitness API

This comprehensive guide covers installation, configuration, and authentication setup for the Pierre Fitness API platform.

## Quick Start

### Local Development

```bash
# Build the project
cargo build --release

# Run the server (requires environment configuration)
cargo run --bin pierre-mcp-server
```

### Production Setup

Pierre MCP Server provides user authentication, OAuth integration, and MCP protocol compliance with both stdio and HTTP transports.

**Step 1: Fresh Database Setup**
```bash
# Clean database and start fresh
./scripts/fresh-start.sh
```

**Step 2: Generate Admin Token**
```bash
# Generate admin token for API key management
cargo run --bin admin-setup generate-token --service "my-service"
# Save the JWT token from output - shown only once!
```

**Step 3: Start Production Server**
```bash
# Server runs on ports 8080 (MCP) and 8081 (HTTP)
cargo run --bin pierre-mcp-server
```

**Step 4: Create User Account**
```bash
# Register new user
curl -X POST http://localhost:8081/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email":"user@example.com","password":"password123","display_name":"Test User"}'

# Login to get JWT token
curl -X POST http://localhost:8081/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"user@example.com","password":"password123"}'
```

**Step 5: Setup OAuth (Optional)**
```bash
# Generate Strava OAuth URL (replace USER_ID with actual user ID)
curl -X GET "http://localhost:8081/oauth/auth/strava/YOUR_USER_ID"
# Visit the returned URL to authorize Strava access
```

### MCP Protocol Usage

The multi-tenant server supports both MCP-compliant transports as specified in the MCP 2024-11-05 specification:

#### MCP stdio Transport (Primary)
The stdio transport is the primary MCP transport for local AI assistant connections:

```bash
# Example: pipe MCP requests to server
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{"roots":{"listChanged":true}},"clientInfo":{"name":"test-client","version":"1.0.0"}}}
{"jsonrpc":"2.0","id":2,"method":"tools/call","auth":"Bearer YOUR_JWT_TOKEN","params":{"name":"get_connection_status","arguments":{}}}' | cargo run --bin pierre-mcp-server
```

#### MCP Streamable HTTP Transport
The HTTP transport enables remote MCP connections:

```bash
# Initialize MCP connection
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -H "Origin: http://localhost" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{"roots":{"listChanged":true}},"clientInfo":{"name":"test-client","version":"1.0.0"}}}'

# List available tools
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -H "Origin: http://localhost" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/list","auth":"Bearer YOUR_JWT_TOKEN"}'

# Call a tool (get connection status)
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -H "Origin: http://localhost" \
  -d '{"jsonrpc":"2.0","id":3,"method":"tools/call","auth":"Bearer YOUR_JWT_TOKEN","params":{"name":"get_connection_status","arguments":{}}}'

# Get athlete profile
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -H "Origin: http://localhost" \
  -d '{"jsonrpc":"2.0","id":4,"method":"tools/call","auth":"Bearer YOUR_JWT_TOKEN","params":{"name":"get_athlete","arguments":{"provider":"strava"}}}'

# Get recent activities
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -H "Origin: http://localhost" \
  -d '{"jsonrpc":"2.0","id":5,"method":"tools/call","auth":"Bearer YOUR_JWT_TOKEN","params":{"name":"get_activities","arguments":{"provider":"strava","limit":5}}}'
```

#### MCP Authentication
Multi-tenant mode requires JWT authentication in the `auth` field:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "auth": "Bearer YOUR_JWT_TOKEN",
  "params": {
    "name": "get_activities",
    "arguments": {"provider": "strava", "limit": 5}
  }
}
```

**Important Notes:**
- **Protocol Version**: Use `2024-11-05` for MCP protocol version
- **Transport Ports**: stdio (same process), HTTP (port 8080)
- **Authentication**: JWT token required for all tool calls
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

OAuth providers are configured through the admin API, not environment variables:

```bash
# Generate admin token first
cargo run --bin admin-setup generate-token --service "my-service"

# Configure OAuth providers via admin API
curl -X POST http://localhost:8081/admin/oauth/providers/strava \
  -H "Authorization: Bearer ADMIN_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "client_id": "your_strava_client_id",
    "client_secret": "your_strava_client_secret",
    "redirect_uri": "http://localhost:8081/oauth/callback/strava"
  }'

# Configure Fitbit provider
curl -X POST http://localhost:8081/admin/oauth/providers/fitbit \
  -H "Authorization: Bearer ADMIN_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "client_id": "your_fitbit_client_id",
    "client_secret": "your_fitbit_client_secret",
    "redirect_uri": "http://localhost:8081/oauth/callback/fitbit"
  }'
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
   - Set redirect URI to: `http://localhost:8081/oauth/callback/strava`

2. **Configure Environment Variables**:
   ```bash
   STRAVA_CLIENT_ID=your_strava_client_id
   STRAVA_CLIENT_SECRET=your_strava_client_secret
   ```

3. **Test OAuth Flow**:
   ```bash
   # Get OAuth authorization URL
   curl -X GET "http://localhost:8081/oauth/auth/strava?user_id=user_123"
   
   # Visit the returned URL in browser to authorize
   # User will be redirected back with authorization code
   ```

#### Fitbit OAuth Setup

1. **Create Fitbit Application**:
   - Go to https://dev.fitbit.com/apps
   - Create a new application
   - Set redirect URI to: `http://localhost:8081/oauth/callback/fitbit`

2. **Configure Environment Variables**:
   ```bash
   FITBIT_CLIENT_ID=your_fitbit_client_id
   FITBIT_CLIENT_SECRET=your_fitbit_client_secret
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