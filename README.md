# Pierre MCP Server

[![CI](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/ci.yml/badge.svg)](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/ci.yml)
[![Frontend Tests](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/frontend-tests.yml/badge.svg)](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/frontend-tests.yml)

**Development Status**: This project is under active development. APIs and features may change.

A Model Context Protocol (MCP) server that connects AI assistants to fitness data from providers like Strava. Built in Rust, it provides secure access to activity data, athlete profiles, and basic fitness analytics through the MCP protocol.

## Key Features

- **Multi-Protocol Support**: MCP, A2A (Agent-to-Agent), OAuth 2.0 Authorization Server, REST API
- **Enterprise Multi-Tenancy**: Isolated data and configuration per organization
- **Real-Time Notifications**: Server-Sent Events for OAuth status and system updates
- **Compile-Time Plugin System**: Zero-overhead extensible fitness analysis tools
- **High Performance**: Rust-based implementation with memory safety and fearless concurrency
- **Standards Compliance**: RFC 7591 OAuth 2.0 dynamic client registration, MCP 1.0 specification

## Use Cases

- **Fitness Data Analysis**: Access and analyze activities from Strava, Fitbit, and other providers
- **Performance Intelligence**: Generate insights from training data with weather and location context
- **AI Assistant Integration**: Enable Claude, ChatGPT, and other AI assistants to work with fitness data
- **Autonomous Agent Systems**: Build fitness-focused AI agents with A2A communication capabilities
- **Multi-tenant SaaS Applications**: Support multiple organizations with isolated data and billing
- **OAuth 2.0 Provider**: Act as authorization server for fitness applications using mcp-remote
- **Real-time Dashboards**: Stream live notifications for OAuth flows and system events

## Quick Start

### Prerequisites

- Rust 1.70+
- SQLite (default) or PostgreSQL (production)

### Installation

```bash
git clone https://github.com/Async-IO/pierre_mcp_server.git
cd pierre_mcp_server
cargo build --release
```

### Basic Setup

1. **Set required environment variables:**
```bash
export DATABASE_URL="sqlite:./data/pierre.db"
export PIERRE_MASTER_ENCRYPTION_KEY="$(openssl rand -base64 32)"
```

2. **Start the server:**
```bash
cargo run --bin pierre-mcp-server
```

3. **Create admin user:**
```bash
curl -X POST http://localhost:8081/admin/setup \
  -H "Content-Type: application/json" \
  -d '{
    "email": "admin@example.com",
    "password": "SecurePass123!",
    "display_name": "System Administrator"
  }'
```

### Automated Development Setup

For development and testing:

```bash
# Clean database and start fresh server
./scripts/fresh-start.sh
RUST_LOG=debug cargo run --bin pierre-mcp-server &

# Run complete setup (admin + user + tenant + login + MCP test)
./scripts/complete-user-workflow.sh

# Use saved environment variables
source .workflow_test_env
echo "JWT Token: ${JWT_TOKEN:0:50}..."
```

### Docker Installation

```bash
# Build and run with Docker
docker build -t pierre-mcp-server .
docker run -p 8080:8080 pierre-mcp-server
```

## MCP Client Configuration

### MCP Client Support

Pierre MCP Server supports any MCP-compliant client. However, `mcp-remote` is the recommended client for OAuth 2.0 authentication.

#### Using mcp-remote

Pierre MCP Server is designed to work with `mcp-remote`, which handles the OAuth 2.0 flow automatically.

#### Installation

mcp-remote can be used directly via npx (recommended) or installed globally:

```bash
# Option 1: Use via npx (recommended - no installation required)
npx mcp-remote http://localhost:8081/mcp --allow-http --debug --transport http-first --auth-timeout 120 --timeout 120

# Option 2: Install globally first, then use
npm install -g mcp-remote
mcp-remote http://localhost:8081/mcp --allow-http --debug --transport http-first --auth-timeout 120 --timeout 120
```

#### Claude Desktop Configuration

Add this to your Claude Desktop configuration file (`~/Library/Application Support/Claude/claude_desktop_config.json` on macOS):

**Option 1: Using npx (recommended)**
```json
{
  "mcpServers": {
    "pierre-fitness": {
      "command": "npx",
      "args": ["mcp-remote", "http://localhost:8081/mcp", "--allow-http", "--debug", "--transport", "http-first", "--auth-timeout", "120", "--timeout", "120"],
      "env": {}
    }
  }
}
```

**Option 2: Using globally installed mcp-remote**
```json
{
  "mcpServers": {
    "pierre-fitness": {
      "command": "mcp-remote",
      "args": ["http://localhost:8081/mcp", "--allow-http", "--debug", "--transport", "http-first", "--auth-timeout", "120", "--timeout", "120"],
      "env": {}
    }
  }
}
```

This will:
1. Register a new OAuth client automatically
2. Open browser for user login and authorization
3. Exchange authorization code for JWT token
4. Use JWT for subsequent MCP requests

### Claude Desktop Integration

**Note:** Direct Claude Desktop integration has not been reliably tested. The recommended approach is to use `mcp-remote` as your MCP client, which provides a stable interface to Pierre MCP Server's OAuth 2.0 authentication system.

For Claude Desktop users, use `mcp-remote` as an intermediary client to access Pierre's fitness tools through the command line.

## Available Tools

### Core Fitness Data Tools

| Tool | Description | Required Parameters |
|------|-------------|-------------------|
| `get_activities` | Get activities from fitness providers | `provider` (optional), `limit` (optional) |
| `get_athlete` | Get athlete information | None |
| `get_stats` | Get athlete statistics | None |
| `get_activity_intelligence` | Get AI intelligence for activity | `activity_id` |
| `get_connection_status` | Check provider connection status | None |
| `disconnect_provider` | Disconnect and remove stored tokens for a specific fitness provider | `provider` |

### Notification Management Tools

| Tool | Description | Required Parameters |
|------|-------------|-------------------|
| `get_notifications` | Get user notifications | None |
| `mark_notifications_read` | Mark notifications as read | None |
| `announce_oauth_success` | Announce OAuth flow completion | None |
| `check_oauth_notifications` | Check for OAuth notifications | None |

### Analytics & Performance Tools

| Tool | Description | Required Parameters |
|------|-------------|-------------------|
| `analyze_activity` | Analyze an activity | `activity_id` |
| `calculate_metrics` | Calculate advanced fitness metrics for an activity | `activity_id` |
| `analyze_performance_trends` | Analyze performance trends over time | None |
| `compare_activities` | Compare an activity against similar activities or personal bests | `activity_ids` |
| `detect_patterns` | Detect patterns in training data | None |
| `predict_performance` | Predict future performance capabilities | None |
| `calculate_fitness_score` | Calculate comprehensive fitness score | None |
| `analyze_training_load` | Analyze training load balance and recovery needs | None |
| `generate_recommendations` | Generate personalized training recommendations | None |

### Goal & Training Tools

| Tool | Description | Required Parameters |
|------|-------------|-------------------|
| `set_goal` | Set a fitness goal | `goal_type`, `target_value` |
| `track_progress` | Track progress toward a specific goal | `goal_id` |
| `suggest_goals` | Generate AI-powered goal suggestions | None |
| `analyze_goal_feasibility` | Assess whether a goal is realistic and achievable | `goal_data` |

### Configuration Tools

| Tool | Description | Required Parameters |
|------|-------------|-------------------|
| `get_configuration_catalog` | Get the complete configuration catalog with all available parameters | None |
| `get_configuration_profiles` | Get available configuration profiles (Research, Elite, Recreational, etc.) | None |
| `get_user_configuration` | Get current user's configuration settings and overrides | None |
| `update_user_configuration` | Update user's configuration parameters and session overrides | `profile` or `parameters` |
| `calculate_personalized_zones` | Calculate personalized training zones based on user's VO2 max and configuration | None |
| `validate_configuration` | Validate configuration parameters against safety rules and constraints | `parameters` |

### Fitness Configuration Tools

| Tool | Description | Required Parameters |
|------|-------------|-------------------|
| `get_fitness_config` | Get current fitness configuration | None |
| `set_fitness_config` | Set fitness configuration parameters | `config` |
| `list_fitness_configs` | List available fitness configurations | None |
| `delete_fitness_config` | Delete a fitness configuration | `config_id` |


## A2A (Agent-to-Agent) Protocol

Pierre supports Agent-to-Agent communication for building autonomous fitness agent networks:

**A2A Protocol Features:**
- **Agent Cards**: Self-describing agent capabilities and identity
- **Secure Communication**: Cryptographic authentication between agents
- **Async Messaging**: Non-blocking inter-agent communication
- **Protocol Versioning**: Forward-compatible A2A message format

**A2A Endpoints:**
- `GET /a2a/status` - Get A2A protocol status
- `GET /a2a/tools` - Get available A2A tools
- `POST /a2a/execute` - Execute A2A tool
- `GET /a2a/monitoring` - Get A2A monitoring information
- `GET /a2a/client/tools` - Get client-specific A2A tools
- `POST /a2a/client/execute` - Execute client A2A tool

**Example A2A Integration:**
```rust
use pierre_mcp_server::a2a::A2AClient;

#[tokio::main]
async fn main() -> Result<()> {
    let client = A2AClient::new("https://pierre-server.com/a2a").await?;

    let response = client.send_message(
        "fitness-analyzer-agent",
        serde_json::json!({
            "action": "analyze_performance",
            "user_id": "user-123",
            "timeframe": "last_30_days"
        })
    ).await?;

    println!("Analysis result: {}", response);
    Ok(())
}
```

## Real-Time Notifications

Pierre provides Server-Sent Events (SSE) for real-time updates:

**Notification Endpoints:**
- `GET /notifications/sse?user_id={user_id}` - Subscribe to user notifications

**Notification Types:**
- OAuth authorization completion
- OAuth errors and failures
- System status updates
- A2A message notifications

**Example SSE Integration:**
```javascript
const eventSource = new EventSource('/notifications/sse?user_id=user-123');

eventSource.onmessage = function(event) {
    const notification = JSON.parse(event.data);
    console.log('Received:', notification);

    if (notification.type === 'oauth_complete') {
        // Handle OAuth completion
        window.location.reload();
    }
};
```

## Authentication & Security

Pierre MCP Server implements dual authentication modes for maximum compatibility:

### OAuth 2.0 Authorization Server (RFC-Compliant)

Pierre acts as a standards-compliant OAuth 2.0 Authorization Server for mcp-remote compatibility:

**Available OAuth 2.0 Endpoints:**
- `GET /.well-known/oauth-authorization-server` - Server metadata discovery (RFC 8414)
- `POST /oauth2/register` - Dynamic client registration (RFC 7591)
- `GET /oauth2/authorize` - Authorization endpoint
- `POST /oauth2/token` - Token endpoint (issues JWT access tokens)
- `GET /oauth2/jwks` - JSON Web Key Set

**OAuth 2.0 Flow Example:**
```bash
# 1. Client registration (automatic via mcp-remote)
curl -X POST http://localhost:8080/oauth2/register \
  -H "Content-Type: application/json" \
  -d '{
    "redirect_uris": ["http://localhost:35535/oauth/callback"],
    "client_name": "mcp-remote",
    "grant_types": ["authorization_code"]
  }'

# 2. Use mcp-remote for full OAuth 2.0 flow
mcp-remote http://localhost:8080/mcp --allow-http
```

### JWT Token Authentication

1. **Create admin user** (using admin-setup binary):
```bash
# Create admin user with admin-setup binary
cargo run --bin admin-setup -- create-admin-user \
  --email admin@example.com \
  --password SecurePass123!
```

2. **User registration and login:**
```bash
# Register a new user
curl -X POST http://localhost:8080/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email": "user@example.com", "password": "pass123", "display_name": "User"}'

# Get JWT token for MCP integration
JWT_TOKEN=$(curl -s -X POST http://localhost:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email": "user@example.com", "password": "pass123"}' | jq -r '.jwt_token')
```

3. **Alternative: OAuth 2.0 flow** (recommended):
```bash
# Use mcp-remote for automatic OAuth 2.0 authentication
mcp-remote http://localhost:8080/mcp --allow-http
```

## Configuration

### OAuth Provider Integration

Pierre MCP Server supports multiple methods for providing OAuth credentials for fitness providers:

1. **Server-level credentials** (default): Environment variables shared across all users
2. **Client-specific credentials** (for full control): Environment variables in MCP client configuration  
3. **Tenant-specific credentials**: Isolated per organization via API

#### OAuth Credential Configuration

By default, Pierre MCP Server uses shared server-level OAuth credentials for all users. 

**Alternative: Client-Specific Credentials**

If you need full control over your OAuth application (custom rate limits, branding, etc.), you can optionally provide your own credentials in the MCP client configuration:
```json
{
  "mcpServers": {
    "pierre-fitness": {
      "url": "http://127.0.0.1:8080/mcp",
      "headers": {
        "Authorization": "Bearer YOUR_JWT_TOKEN"
      },
      "initializationOptions": {
        "oauthCredentials": {
          "strava": {
            "clientId": "your_client_id",
            "clientSecret": "your_client_secret"
          },
          "fitbit": {
            "clientId": "your_fitbit_client_id",
            "clientSecret": "your_fitbit_client_secret"
          }
        }
      }
    }
  }
}
```

The server will use these client-specific credentials instead of the shared server-level credentials for OAuth flows.

### Environment Variables

#### Required
```bash
# Core Configuration
DATABASE_URL=sqlite:./data/pierre.db
PIERRE_MASTER_ENCRYPTION_KEY=your_32_byte_base64_key  # Generate with: openssl rand -base64 32
```

#### Optional
```bash
# Server Configuration
HTTP_PORT=8081  # Default port for all protocols (MCP + OAuth 2.0 + REST API)
HOST=localhost

# Logging
RUST_LOG=info
LOG_FORMAT=json  # For structured logging

# Database (Production)
DATABASE_URL=postgresql://user:pass@localhost:5432/pierre

# OAuth Provider Configuration (for fitness data integration)
STRAVA_CLIENT_ID=your_strava_client_id
STRAVA_CLIENT_SECRET=your_strava_client_secret
STRAVA_REDIRECT_URI=http://localhost:8080/api/oauth/callback/strava

# JWT Configuration (Required for OAuth 2.0)
JWT_SECRET=your_jwt_secret_key  # Required for OAuth 2.0 authorization server
JWT_EXPIRY_HOURS=24

# OpenWeather API (for activity intelligence)
OPENWEATHER_API_KEY=your_openweather_api_key
```


## Architecture

Pierre MCP Server implements a multi-protocol architecture:

- **HTTP Server**: Single port (default 8081) for all protocols
- **MCP Protocol**: JSON-RPC over HTTP with JWT authentication for tool execution
- **OAuth 2.0 Authorization Server**: RFC-compliant server supporting dynamic client registration (RFC 7591)
- **REST API**: User management and fitness provider OAuth endpoints
- **Plugin System**: Compile-time plugin architecture for extensible fitness analysis
- **Multi-tenant Support**: Isolated user data and configuration
- **JWT Authentication**: Standard token-based authentication with configurable secrets

## Testing

```bash
# Run all tests
cargo test

# Run with coverage
cargo test --release

# Lint and test (comprehensive validation)
./scripts/lint-and-test.sh

# MCP protocol compliance tests
cargo test --test mcp_protocol_comprehensive_test
cargo test --test mcp_protocol_compliance_test

# Multi-tenant integration tests
cargo test --test mcp_multitenant_complete_test
```

## Management Dashboard

A web dashboard is available for monitoring and administration:

```bash
# Start the dashboard (requires server running)
cd frontend
npm install && npm run dev
```

Access at http://localhost:5173 for:
- User management and approval
- API key monitoring and rate limits
- Usage analytics and system metrics
- Real-time request monitoring

See [frontend/README.md](frontend/README.md) for detailed development information.

## Documentation

Complete documentation is available in the `docs/` directory:

- **[Getting Started](docs/developer-guide/15-getting-started.md)** - Setup guide
- **[Installation Guides](docs/installation-guides/)** - Platform-specific installation
- **[Developer Guide](docs/developer-guide/)** - Technical documentation
- **[Fitness Configuration](docs/developer-guide/20-fitness-configuration.md)** - Comprehensive fitness configuration guide
- **[Logging and Observability](docs/developer-guide/19-logging-and-observability.md)** - Logging, debugging, and monitoring
- **[Plugin System](docs/developer-guide/18-plugin-system.md)** - Plugin development guide
- **[API Reference](docs/developer-guide/14-api-reference.md)** - API documentation
- **[Security Guide](docs/developer-guide/17-security-guide.md)** - Security best practices

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/new-feature`)
3. Run tests and linting (`./scripts/lint-and-test.sh`)
4. Commit your changes (`git commit -m 'feat: add new feature'`)
5. Push to the branch (`git push origin feature/new-feature`)
6. Open a Pull Request

## License

This project is dual-licensed under:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

You may choose either license for your use.