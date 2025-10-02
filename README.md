# Pierre MCP Server

[![CI](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/ci.yml/badge.svg)](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/ci.yml)
[![Frontend Tests](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/frontend-tests.yml/badge.svg)](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/frontend-tests.yml)
[![MCP Compliance](https://img.shields.io/badge/MCP_Compliance-69.8%25-yellow)](sdk/MCP_COMPLIANCE.md)

**Development Status**: This project is under active development. APIs and features may change.

Pierre MCP Server connects AI assistants to fitness data from Strava and Fitbit. The server implements the Model Context Protocol (MCP) for integration with Claude, ChatGPT, and other AI assistants.

## Features

- **MCP Protocol**: JSON-RPC over HTTP for AI assistant integration
- **OAuth 2.0 Server**: RFC 7591 dynamic client registration for MCP clients
- **A2A Protocol**: Agent-to-agent communication with capability discovery
- **Multi-Tenancy**: Isolated data and configuration per organization
- **Real-Time Updates**: Server-Sent Events for OAuth notifications
- **Plugin System**: Compile-time plugin architecture for fitness analysis

## Architecture

Pierre runs as a single HTTP server on port 8081 (configurable). All protocols (MCP, OAuth 2.0, REST API) share the same port.

```
┌─────────────────┐    stdio     ┌─────────────────┐    HTTP+OAuth   ┌─────────────────┐
│   MCP Client    │ ◄─────────► │ Pierre SDK      │ ◄─────────────► │ Pierre MCP      │
│                 │              │ Bridge          │                 │ Server          │
└─────────────────┘              └─────────────────┘                 └─────────────────┘
```

**Core Components** (from `src/lib.rs:58-182`):
- `mcp`: MCP protocol server implementation
- `oauth2`: OAuth 2.0 authorization server (RFC 7591)
- `a2a`: Agent-to-agent protocol with agent cards
- `providers`: Fitness provider integrations (Strava, Fitbit)
- `intelligence`: Activity analysis and insights
- `database_plugins`: SQLite and PostgreSQL support
- `auth`: JWT token authentication
- `crypto`: Two-tier key management system

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

### Configuration

**Required Environment Variables**:
```bash
export DATABASE_URL="sqlite:./data/pierre.db"
export PIERRE_MASTER_ENCRYPTION_KEY="$(openssl rand -base64 32)"
```

**Optional Environment Variables**:
```bash
export HTTP_PORT=8081              # Server port (default: 8081)
export RUST_LOG=info               # Log level
export JWT_EXPIRY_HOURS=24         # JWT token expiry

# Fitness provider OAuth (for data integration)
export STRAVA_CLIENT_ID=your_client_id
export STRAVA_CLIENT_SECRET=your_client_secret
export STRAVA_REDIRECT_URI=http://localhost:8081/oauth/callback/strava

# Weather data (optional)
export OPENWEATHER_API_KEY=your_api_key
```

See `src/constants/mod.rs:32-173` for all environment variables.

### Starting the Server

```bash
cargo run --bin pierre-mcp-server
```

The server will start on port 8081 and display available endpoints.

### Initial Setup

Create an admin user via REST API:

```bash
curl -X POST http://localhost:8081/admin/setup \
  -H "Content-Type: application/json" \
  -d '{
    "email": "admin@example.com",
    "password": "SecurePass123!",
    "display_name": "Admin"
  }'
```

## MCP Client Integration

Pierre includes an SDK bridge for direct integration with MCP clients. The SDK handles OAuth 2.0 authentication automatically.

### SDK Installation

The SDK is included in the `sdk/` directory:

```bash
cd sdk
npm install
npm run build
```

### MCP Client Configuration

Add Pierre to your MCP client configuration. For Claude Desktop:

**Configuration File Location**:
- macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`
- Windows: `%APPDATA%\Claude\claude_desktop_config.json`
- Linux: `~/.config/claude/claude_desktop_config.json`

**Configuration**:
```json
{
  "mcpServers": {
    "pierre-fitness": {
      "command": "node",
      "args": [
        "/absolute/path/to/pierre_mcp_server/sdk/dist/cli.js",
        "--server",
        "http://localhost:8081"
      ]
    }
  }
}
```

Replace `/absolute/path/to/` with your actual path.

### Authentication Flow

When the MCP client starts, the SDK will:

1. Register an OAuth 2.0 client with Pierre (RFC 7591)
2. Open your browser for authentication
3. Handle the OAuth callback and token exchange
4. Use JWT tokens for all MCP requests

No manual token management required.

## Available MCP Tools

Pierre provides 26 tools through the MCP protocol. Tool definitions are in `src/protocols/universal/tool_registry.rs:12-45`.

### Core Fitness Data

| Tool | Description | Parameters |
|------|-------------|------------|
| `get_activities` | Get user activities from fitness providers | `provider` (optional), `limit` (optional) |
| `get_athlete` | Get athlete profile information | None |
| `get_stats` | Get athlete statistics and metrics | None |
| `analyze_activity` | Analyze a specific activity with detailed insights | `activity_id` (required) |
| `get_activity_intelligence` | Get AI-powered analysis for an activity | `activity_id` (required) |
| `get_connection_status` | Check OAuth connection status for providers | None |
| `disconnect_provider` | Disconnect from a fitness provider | `provider` (required) |

### Goals and Progress

| Tool | Description | Parameters |
|------|-------------|------------|
| `set_goal` | Set a new fitness goal | `goal_type`, `target_value` (required) |
| `suggest_goals` | Get AI-suggested goals based on activity history | None |
| `analyze_goal_feasibility` | Analyze if a goal is achievable | `goal_data` (required) |
| `track_progress` | Track progress toward goals | `goal_id` (required) |

### Performance Analysis

| Tool | Description | Parameters |
|------|-------------|------------|
| `calculate_metrics` | Calculate custom fitness metrics | `activity_id` (required) |
| `analyze_performance_trends` | Analyze performance trends over time | None |
| `compare_activities` | Compare activities for performance analysis | `activity_ids` (required) |
| `detect_patterns` | Detect patterns in activity data | None |
| `generate_recommendations` | Generate personalized training recommendations | None |
| `calculate_fitness_score` | Calculate overall fitness score | None |
| `predict_performance` | Predict future performance based on training | None |
| `analyze_training_load` | Analyze training load and recovery | None |

### Configuration Management

| Tool | Description | Parameters |
|------|-------------|------------|
| `get_configuration_catalog` | Get complete configuration catalog | None |
| `get_configuration_profiles` | Get available configuration profiles | None |
| `get_user_configuration` | Get current user configuration | None |
| `update_user_configuration` | Update user configuration parameters | `profile` or `parameters` (required) |
| `calculate_personalized_zones` | Calculate personalized training zones | None |
| `validate_configuration` | Validate configuration parameters | `parameters` (required) |

Tool descriptions from `src/protocols/universal/tool_registry.rs:114-162`.

## Authentication

Pierre supports multiple authentication methods for different use cases.

### OAuth 2.0 Authorization Server

Pierre implements an OAuth 2.0 Authorization Server for MCP client authentication. Implementation in `src/oauth2/`.

**OAuth 2.0 Endpoints**:
- `GET /.well-known/oauth-authorization-server` - Server metadata (RFC 8414)
- `POST /oauth2/register` - Dynamic client registration (RFC 7591)
- `GET /oauth2/authorize` - Authorization endpoint
- `POST /oauth2/token` - Token endpoint (issues JWT access tokens)
- `GET /oauth2/jwks` - JSON Web Key Set

**OAuth 2.0 Flow**:

The Pierre SDK handles this automatically. For manual integration:

```bash
# 1. Register OAuth 2.0 client
curl -X POST http://localhost:8081/oauth2/register \
  -H "Content-Type: application/json" \
  -d '{
    "redirect_uris": ["http://localhost:35535/oauth/callback"],
    "client_name": "My MCP Client",
    "grant_types": ["authorization_code"]
  }'

# Response includes client_id and client_secret

# 2. Browser authorization
# User opens: http://localhost:8081/oauth2/authorize?client_id=...&redirect_uri=...&response_type=code

# 3. Token exchange
curl -X POST http://localhost:8081/oauth2/token \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=authorization_code&code=...&client_id=...&client_secret=..."

# Response includes JWT access token
```

Implementation in `src/oauth2/client_registration.rs:27-50`.

### JWT Authentication

For direct REST API access:

```bash
# Register user
curl -X POST http://localhost:8081/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email": "user@example.com", "password": "password123", "display_name": "User"}'

# Login to get JWT token
curl -X POST http://localhost:8081/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email": "user@example.com", "password": "password123"}'

# Use JWT token
curl -H "Authorization: Bearer YOUR_JWT_TOKEN" http://localhost:8081/mcp
```

### API Key Authentication

For service-to-service integration:

```bash
# Create API key (requires admin or user JWT)
curl -X POST http://localhost:8081/api/keys \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name": "My Service", "tier": "professional"}'

# Use API key
curl -H "X-API-Key: YOUR_API_KEY" http://localhost:8081/api/endpoint
```

## A2A (Agent-to-Agent) Protocol

Pierre supports agent-to-agent communication for autonomous AI systems. Implementation in `src/a2a/`.

**A2A Features**:
- Agent Cards for capability discovery (`src/a2a/agent_card.rs`)
- Cryptographic authentication between agents
- Asynchronous messaging protocol
- Protocol versioning (A2A 1.0.0)

**A2A Endpoints**:
- `GET /a2a/status` - Get A2A protocol status
- `GET /a2a/tools` - Get available A2A tools
- `POST /a2a/execute` - Execute A2A tool
- `GET /a2a/monitoring` - Get A2A monitoring information
- `GET /a2a/client/tools` - Get client-specific A2A tools
- `POST /a2a/client/execute` - Execute client A2A tool

**Example A2A Integration**:
```rust
use pierre_mcp_server::a2a::A2AClientManager;

#[tokio::main]
async fn main() -> Result<()> {
    let client = A2AClientManager::new("https://pierre-server.com/a2a").await?;

    let response = client.send_message(
        "fitness-analyzer-agent",
        serde_json::json!({
            "action": "analyze_performance",
            "user_id": "user-123",
            "timeframe": "last_30_days"
        })
    ).await?;

    println!("Analysis: {}", response);
    Ok(())
}
```

## Real-Time Notifications

Pierre provides Server-Sent Events (SSE) for real-time updates. Implementation in `src/notifications/sse.rs` and `src/sse.rs`.

**SSE Endpoint**:
```
GET /notifications/sse?user_id={user_id}
```

**Notification Types**:
- OAuth authorization completion
- OAuth errors and failures
- System status updates
- A2A message notifications

**Example JavaScript Integration**:
```javascript
const eventSource = new EventSource('/notifications/sse?user_id=user-123');

eventSource.onmessage = function(event) {
    const notification = JSON.parse(event.data);
    console.log('Notification:', notification);

    if (notification.type === 'oauth_complete') {
        // Handle OAuth completion
        window.location.reload();
    }
};
```

## Testing

```bash
# Run all tests
cargo test

# Run specific test suite
cargo test --test mcp_protocol_comprehensive_test
cargo test --test mcp_multitenant_complete_test

# Run with output
cargo test -- --nocapture

# Lint and test (comprehensive validation)
./scripts/lint-and-test.sh
```

## Development Tools

### Automated Setup

```bash
# Clean database and fresh server start
./scripts/fresh-start.sh
cargo run --bin pierre-mcp-server &

# Complete workflow test (admin + user + tenant + login + MCP)
./scripts/complete-user-workflow.sh

# Load saved environment variables
source .workflow_test_env
echo "JWT Token: ${JWT_TOKEN:0:50}..."
```

### Management Dashboard

A web dashboard is available for monitoring:

```bash
cd frontend
npm install && npm run dev
```

Access at `http://localhost:5173` for:
- User management and approval
- API key monitoring
- Usage analytics
- Real-time request monitoring

See `frontend/README.md` for details.

## Documentation

Complete documentation is in the `docs/` directory:

- **[Getting Started](docs/developer-guide/15-getting-started.md)** - Setup guide
- **[Architecture](docs/developer-guide/01-architecture.md)** - System design
- **[MCP Protocol](docs/developer-guide/04-mcp-protocol.md)** - MCP implementation details
- **[A2A Protocol](docs/developer-guide/05-a2a-protocol.md)** - Agent-to-agent communication
- **[Authentication](docs/developer-guide/06-authentication.md)** - OAuth 2.0 and JWT
- **[Database](docs/developer-guide/08-database.md)** - Database schema and migrations
- **[Configuration](docs/developer-guide/12-configuration.md)** - Configuration management
- **[API Reference](docs/developer-guide/14-api-reference.md)** - REST API documentation
- **[Security](docs/developer-guide/17-security-guide.md)** - Security best practices
- **[Plugin System](docs/developer-guide/18-plugin-system.md)** - Plugin development
- **[Logging](docs/developer-guide/19-logging-and-observability.md)** - Logging and monitoring

Installation guides for specific platforms:
- **[Claude Desktop](docs/installation-guides/install-claude.md)**
- **[ChatGPT](docs/installation-guides/install-chatgpt.md)**

## Code Quality

Pierre uses validation scripts to maintain code quality and prevent common issues:

**Pre-commit Validation**:
- Pattern validation via `scripts/validation-patterns.toml`
- Clippy linting with strict warnings
- Test execution
- Format checking

**Run validation**:
```bash
./scripts/lint-and-test.sh
```

Install git hooks:
```bash
./scripts/install-hooks.sh
```

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/new-feature`)
3. Run validation (`./scripts/lint-and-test.sh`)
4. Commit changes (`git commit -m 'feat: add new feature'`)
5. Push to branch (`git push origin feature/new-feature`)
6. Open a Pull Request

## License

This project is dual-licensed:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

You may choose either license.
