# Pierre MCP Server

[![CI](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/ci.yml/badge.svg)](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/ci.yml)
[![Frontend Tests](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/frontend-tests.yml/badge.svg)](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/frontend-tests.yml)

MCP server for fitness data access. Connects AI assistants to Strava and Fitbit APIs through MCP Protocol, A2A Protocol, and REST APIs with secure OAuth and encrypted storage.

## Technical Architecture

**Client-Server Architecture**: Clean separation between data processing and API access:

**Pierre MCP Server** (`pierre-mcp-server`) - Core daemon process
- Manages fitness data operations and OAuth credentials
- SQLite/PostgreSQL database with encrypted storage (AES-256-GCM)
- HTTP API server and MCP protocol handler
- Secure user isolation with JWT authentication

**Pierre MCP Client** (`pierre-mcp-client`) - Protocol adapter
- Translates MCP protocol to server HTTP API calls
- Stateless design with no local data storage
- Configured via environment variables for server access

**Data Flow**: Clients authenticate with JWT → Server validates and executes → Returns fitness data via MCP protocol

## Quick Setup Guide

### Prerequisites

- Rust toolchain 1.75+: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- Strava Developer App: Register at [developers.strava.com](https://developers.strava.com)
- Database: SQLite (development) or PostgreSQL (deployment)

### Local Development Setup

```bash
# Clone and build
git clone https://github.com/Async-IO/pierre_mcp_server.git
cd pierre_mcp_server
cargo build --release

# Start server (default: http://localhost:8081)
cargo run --bin pierre-mcp-server

# Create your account
curl -X POST http://localhost:8081/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "your@email.com",
    "password": "your_password", 
    "name": "Your Name"
  }'

# Configure OAuth credentials (authenticated request)
curl -X POST http://localhost:8081/api/oauth/configure \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
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

**Claude Desktop MCP Configuration** (`~/.claude/claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "pierre-fitness": {
      "command": "/path/to/pierre_mcp_server/target/release/pierre-mcp-client",
      "env": {
        "PIERRE_JWT_TOKEN": "YOUR_JWT_TOKEN",
        "PIERRE_SERVER_URL": "http://localhost:8081"
      }
    }
  }
}
```

**Generate JWT Token**:
```bash
curl -X POST http://localhost:8081/api/auth/token \
  -H "Content-Type: application/json" \
  -d '{
    "email": "your@email.com",
    "password": "your_password"
  }'
```

**Connect to Strava**: Ask Claude "Connect me to Strava" to complete OAuth flow.

**Usage**: Ask natural language fitness questions:
- "What was my longest run this month?"
- "Compare cycling vs running performance trends"
- "Analyze my heart rate zones for last week's activities"

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
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
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
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
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
PIERRE_JWT_TOKEN=your-jwt-token
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

## Available MCP Tools

Core fitness data access tools available via MCP protocol:

| Tool | Parameters | Description |
|------|------------|-------------|
| `get_activities` | `provider`, `limit`, `offset` | Fetch activities with pagination |
| `get_athlete` | `provider` | Complete athlete profile |
| `get_stats` | `provider` | Aggregated fitness statistics |
| `get_activity_intelligence` | `provider`, `activity_id`, `include_weather` | AI-powered activity analysis |
| `analyze_activity` | `provider`, `activity_id` | Detailed performance metrics |
| `calculate_metrics` | `provider`, `activity_id`, `metrics` | Scientific fitness calculations |
| `get_performance_trends` | `provider`, `timeframe` | Performance trend analysis |
| `get_training_recommendations` | `provider`, `analysis_period` | Personalized training suggestions |

**Complete API Reference**: See [docs/API_REFERENCE.md](docs/API_REFERENCE.md) for all 21 available tools.

## API Endpoints

### User Management

```bash
# Register user
POST /api/auth/register
{
  "email": "user@example.com",
  "password": "secure_password",
  "name": "User Name"
}

# Login (get JWT token)
POST /api/auth/token
{
  "email": "user@example.com",
  "password": "secure_password"
}

# Configure OAuth for your account
POST /api/oauth/configure
Authorization: Bearer YOUR_JWT_TOKEN
{
  "provider": "strava",
  "client_id": "your_client_id",
  "client_secret": "your_client_secret",
  "redirect_uri": "http://localhost:8081/oauth/callback",
  "scopes": ["read", "activity:read_all"]
}
```

### OAuth Flow

```bash
# Start OAuth authorization
GET /oauth/authorize/{provider}

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
# Ensure JWT token is valid
curl -X POST http://localhost:8081/api/auth/token \
  -d '{"email": "your@email.com", "password": "your_password"}'
```

**"Authentication failed" errors**
```bash
# Verify JWT token is valid and not expired
curl -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  http://localhost:8081/api/profile
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
cargo test test_user_creation

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

## Technical Implementation

**Data Isolation**: Strict user data isolation with secure OAuth credentials and encrypted storage

**Authentication**: JWT-based authentication with user-scoped permissions and API key support

**Database**: Plugin-based architecture supporting SQLite (development) and PostgreSQL (deployment) with AES-256-GCM encryption

**Protocols**: Full MCP 2024-11-05 specification support plus custom A2A protocol for enterprise integrations

**Testing**: Comprehensive test suite with >90% code coverage including integration and end-to-end tests

## License

This project is dual-licensed under either of:

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
* MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for details on how to get started.