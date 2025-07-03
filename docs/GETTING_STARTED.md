# Getting Started with Pierre Fitness API

This comprehensive guide covers installation, configuration, and authentication setup for the Pierre Fitness API platform.

## Quick Start

### Local Development

```bash
# Build the project
cargo build --release

# Run the server
cargo run --bin pierre-mcp-server
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

# Weather Integration
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
    "tier": "professional",
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
| `test-with-data` | Test with real fitness data | Development/debugging |
| `test-weather-integration` | Test weather API integration | Weather setup validation |
| `test-real-weather` | Test with OpenWeatherMap API | Real weather API testing |
| `diagnose-weather-api` | Weather API diagnostics | Troubleshoot weather issues |
| `test-intelligence-for-longest-run` | Test activity intelligence | AI analysis validation |
| `test-location-intelligence` | Test location detection | GPS/location testing |
| `test-oauth-callback` | Test OAuth callback flow | OAuth integration testing |
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
cargo run --bin test-weather-integration
cargo run --bin diagnose-weather-api

# Activity analysis
cargo run --bin find-2024-longest-run
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

### Getting Help

- Check logs with `RUST_LOG=debug` for detailed error information
- Use health check endpoint: `http://localhost:8081/health`
- Run diagnostic utilities: `cargo run --bin diagnose-weather-api`