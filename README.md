# Pierre MCP Server - Developer Guide

[![CI](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/ci.yml/badge.svg)](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/ci.yml)
[![Frontend Tests](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/frontend-tests.yml/badge.svg)](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/frontend-tests.yml)

Multi-tenant MCP server providing AI assistants with secure access to fitness data (Strava, Fitbit). Supports MCP Protocol, A2A Protocol, and REST APIs with per-tenant OAuth isolation.

## Architecture Overview

**Two-Component Architecture**: This system has clear separation between server and client:

1. **Pierre MCP Server** (`pierre-mcp-server`) - Runs as daemon with database access
   - Handles all fitness data operations
   - Manages tenant OAuth credentials
   - Encrypts and stores sensitive data
   - Serves HTTP API and MCP endpoints

2. **Pierre MCP Client** (`pierre-mcp-client`) - Lightweight MCP client for Claude Desktop
   - No database access whatsoever
   - Connects to running server via HTTP
   - Translates MCP protocol to HTTP API calls
   - Stateless and secure

**Critical**: Clients never have database access. All data operations happen server-side.

## Quick Setup Guide

### Prerequisites

1. **Rust toolchain** (1.75+): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
2. **Strava app**: Create at [developers.strava.com](https://developers.strava.com)
3. **Database**: SQLite (default) or PostgreSQL

### Local Development Setup

```bash
# 1. Clone and build
git clone https://github.com/Async-IO/pierre_mcp_server.git
cd pierre_mcp_server
cargo build --release

# 2. Start the Pierre MCP Server (runs as daemon)
cargo run --bin pierre-mcp-server
# Server starts on http://localhost:8081

# 3. In another terminal, create your development tenant
curl -X POST http://localhost:8081/api/tenants \
  -H "Content-Type: application/json" \
  -d '{
    "name": "My Development Org",
    "slug": "dev-org",
    "domain": "localhost"
  }'
# Save the returned tenant_id

# 4. Configure tenant OAuth with your Strava app
curl -X POST http://localhost:8081/api/tenants/{TENANT_ID}/oauth \
  -H "Content-Type: application/json" \
  -d '{
    "provider": "strava",
    "client_id": "YOUR_STRAVA_CLIENT_ID",
    "client_secret": "YOUR_STRAVA_CLIENT_SECRET",
    "redirect_uri": "http://localhost:8081/oauth/callback",
    "scopes": ["read", "activity:read_all"]
  }'
```

## Claude Desktop Integration

### Step 1: Configure Claude Desktop

Add to your Claude Desktop config (`~/.claude/claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "pierre-fitness": {
      "command": "/path/to/pierre_mcp_server/target/release/pierre-mcp-client",
      "env": {
        "TENANT_ID": "YOUR_TENANT_ID_FROM_STEP_3",
        "TENANT_JWT_TOKEN": "generated_jwt_token_here"
      }
    }
  }
}
```

**Important**: Use `pierre-mcp-client` (the lightweight client), not `pierre-mcp-server` (the database server).

### Step 2: Generate Tenant JWT Token

```bash
# Generate a JWT token for your tenant
curl -X POST http://localhost:8081/api/tenants/{TENANT_ID}/jwt \
  -H "Content-Type: application/json" \
  -d '{"scopes": ["fitness:read", "activity:read"]}'
```

### Step 3: Connect to Strava

In Claude Desktop, ask: "Connect me to Strava". The server will:
1. Generate OAuth URL using your tenant's credentials
2. Open browser for Strava authorization
3. Store encrypted tokens in your tenant's secure storage

### Step 4: Start Analyzing

Now you can ask natural language questions:
- "What was my longest run this month?"
- "Compare my cycling vs running performance"
- "Show me my activity trends for the past year"

## Python Client Integration

### Installing the Client

```bash
pip install pierre-mcp-client
# Or from source:
pip install git+https://github.com/Async-IO/pierre_mcp_server.git#subdirectory=clients/python
```

### Basic Usage

```python
from pierre_mcp import PierreMCPClient
import asyncio

async def main():
    client = PierreMCPClient(
        server_url="http://localhost:8081",
        tenant_id="your-tenant-id",
        jwt_token="your-jwt-token"
    )
    
    # Connect to server
    await client.connect()
    
    # Get available tools
    tools = await client.list_tools()
    print(f"Available tools: {[tool.name for tool in tools]}")
    
    # Execute a tool
    result = await client.call_tool(
        "get_activities",
        {"provider": "strava", "limit": 5}
    )
    
    print(f"Recent activities: {result}")
    
    await client.close()

asyncio.run(main())
```

### Advanced Python Usage

```python
from pierre_mcp import PierreMCPClient
from datetime import datetime, timedelta

async def analyze_performance():
    client = PierreMCPClient(
        server_url="http://localhost:8081",
        tenant_id="dev-org",
        jwt_token="your-jwt-token"
    )
    
    await client.connect()
    
    # Get activities from last month
    end_date = datetime.now()
    start_date = end_date - timedelta(days=30)
    
    activities = await client.call_tool(
        "get_activities",
        {
            "provider": "strava",
            "after": start_date.isoformat(),
            "before": end_date.isoformat()
        }
    )
    
    # Analyze each activity
    for activity in activities:
        analysis = await client.call_tool(
            "analyze_activity",
            {"activity_id": activity["id"], "provider": "strava"}
        )
        print(f"Activity {activity['name']}: {analysis}")
    
    await client.close()

asyncio.run(analyze_performance())
```

## A2A Protocol Integration

### Register A2A Client

```bash
# Register your A2A client application
curl -X POST http://localhost:8081/a2a/clients \
  -H "Content-Type: application/json" \
  -H "X-Tenant-ID: YOUR_TENANT_ID" \
  -d '{
    "name": "Fitness AI Assistant",
    "description": "AI-powered fitness data analysis",
    "capabilities": ["fitness-analysis", "activity-tracking"],
    "contact_email": "developer@yourcompany.com"
  }'
```

### Execute A2A Tools

```bash
# Execute fitness analysis tool
curl -X POST http://localhost:8081/a2a/execute \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "X-Tenant-ID: YOUR_TENANT_ID" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools.execute",
    "params": {
      "tool_name": "get_activities",
      "parameters": {
        "provider": "strava",
        "limit": 10,
        "activity_type": "Run"
      }
    },
    "id": 1
  }'
```

## Environment Configuration

### Client Environment Variables

For MCP clients (Claude Desktop, Python client, etc.):

```bash
# Client connection settings
PIERRE_SERVER_URL=http://localhost:8081
TENANT_ID=your-tenant-id
TENANT_JWT_TOKEN=your-jwt-token
```

### Server Environment Variables (Server-Side Only)

These are only needed when running the Pierre MCP Server itself:

```bash
# Server configuration
PIERRE_PORT=8081
PIERRE_HOST=0.0.0.0
JWT_SECRET=your-jwt-secret-key

# Optional: External services
WEATHER_API_KEY=your-openweathermap-key
GOOGLE_MAPS_API_KEY=your-google-maps-key
```

**Note**: Database configuration is internal to the server and never exposed to clients.

## Available Tools

| Tool | Description | Parameters | Example |
|------|-------------|------------|----------|
| `get_activities` | Fetch activities from provider | `provider`, `limit`, `after`, `before` | Get last 10 runs |
| `get_activity_details` | Get detailed activity data | `activity_id`, `provider` | Analyze specific workout |
| `get_athlete_stats` | Get athlete statistics | `provider` | Overall performance metrics |
| `analyze_activity` | AI-powered activity analysis | `activity_id`, `provider` | Performance insights |
| `get_segments` | Get segment data | `activity_id`, `provider` | Route segment analysis |
| `search_activities` | Search activities by criteria | `query`, `provider` | Find specific workouts |

## API Endpoints

### Tenant Management

```bash
# Create tenant
POST /api/tenants
{
  "name": "Organization Name",
  "slug": "org-slug",
  "domain": "optional-domain.com"
}

# Configure tenant OAuth
POST /api/tenants/{tenant_id}/oauth
{
  "provider": "strava",
  "client_id": "your_client_id",
  "client_secret": "your_client_secret",
  "redirect_uri": "http://localhost:8081/oauth/callback",
  "scopes": ["read", "activity:read_all"]
}

# Generate JWT token
POST /api/tenants/{tenant_id}/jwt
{
  "scopes": ["fitness:read", "activity:read"]
}
```

### OAuth Flow

```bash
# Start OAuth authorization (tenant-aware)
GET /oauth/authorize/{provider}?tenant_id={tenant_id}

# OAuth callback (handles token exchange)
GET /oauth/callback?code=...&state=...
```

### Health Check

```bash
# Server health
GET /health

# Database health
GET /health/database
```

## Troubleshooting

### Common Issues

**"Permission denied" errors**
```bash
# Ensure JWT token has correct scopes
curl -X POST http://localhost:8081/api/tenants/{tenant_id}/jwt \
  -d '{"scopes": ["fitness:read", "activity:read"]}'
```

**"Tenant not found" errors**
```bash
# Check tenant exists and use correct ID
curl http://localhost:8081/api/tenants
```

**OAuth authorization failures**
```bash
# Verify Strava app settings:
# - Authorization Callback Domain: localhost (for dev)
# - Redirect URI: http://localhost:8081/oauth/callback
```

**Server connection errors**
```bash
# Check if Pierre MCP Server is running
curl http://localhost:8081/health
# Should return: {"status": "ok"}
```

### Debug Mode

```bash
# Run with debug logging
RUST_LOG=debug cargo run --bin pierre-mcp-server

# Or set in environment
echo "RUST_LOG=debug" >> .env
```

### Database Reset

```bash
# Restart Pierre MCP Server to reset state
pkill pierre-mcp-server
cargo run --bin pierre-mcp-server
```

## Production Deployment

### Docker Deployment

```bash
# Build Docker image
docker build -t pierre-mcp-server .

# Run server (database configuration is internal)
docker run -d \
  -p 8081:8081 \
  -e JWT_SECRET="your-production-jwt-secret" \
  --name pierre-mcp \
  pierre-mcp-server
```

### Docker Compose

```yaml
version: '3.8'
services:
  pierre-mcp:
    build: .
    ports:
      - "8081:8081"
    environment:
      - JWT_SECRET=your-production-jwt-secret
    volumes:
      - pierre_data:/app/data

volumes:
  pierre_data:
```

### Cloud Deployment (GCP)

```bash
# Build and push to GCR
gcloud builds submit --tag gcr.io/YOUR_PROJECT/pierre-mcp-server

# Deploy to Cloud Run
gcloud run deploy pierre-mcp-server \
  --image gcr.io/YOUR_PROJECT/pierre-mcp-server \
  --platform managed \
  --region us-central1 \
  --set-env-vars JWT_SECRET="your-production-secret" \
  --allow-unauthenticated
```

## Development

### Running Tests

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_tenant_creation

# Run linter and tests
./scripts/lint-and-test.sh
```

### Code Quality

```bash
# Format code
cargo fmt

# Check for issues
cargo clippy -- -D warnings

# Check documentation
cargo doc --no-deps --open
```

### Server Logs

```bash
# View server logs for debugging
RUST_LOG=debug cargo run --bin pierre-mcp-server

# Check server health
curl http://localhost:8081/health
```

## Architecture Notes

- **Multi-tenant only**: No single-tenant mode, all data isolated by tenant
- **OAuth per tenant**: Each tenant configures their own Strava/Fitbit apps
- **Encrypted storage**: All sensitive data encrypted with AES-256-GCM
- **JWT authentication**: Tenant-scoped tokens with configurable permissions
- **Database agnostic**: SQLite for development, PostgreSQL for production

## License

This project is dual-licensed under either of:

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
* MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for details on how to get started.