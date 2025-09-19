# Pierre MCP Server

[![CI](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/ci.yml/badge.svg)](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/ci.yml)
[![Frontend Tests](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/frontend-tests.yml/badge.svg)](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/frontend-tests.yml)

**Development Status**: This project is under active development. APIs and features may change.

A high-performance MCP (Model Context Protocol) server that connects AI assistants to fitness data platforms. Built in Rust for enterprise-grade performance, it provides secure multi-tenant access to fitness data from providers like Strava and Fitbit, with real-time notifications, OAuth 2.0 server capabilities, and extensible plugin architecture.

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

## Installation

### Setup

```bash
git clone https://github.com/Async-IO/pierre_mcp_server.git
cd pierre_mcp_server
cargo build --release

# Start the server
cargo run --bin pierre-mcp-server
```

### Automated Setup

For development and testing, use the automated workflow:

```bash
# Clean database and start fresh server
./scripts/fresh-start.sh
source .envrc && RUST_LOG=debug cargo run --bin pierre-mcp-server &

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

Pierre MCP Server supports multiple MCP client configurations:

### Option 1: Direct Connection (Requires JWT Token)
```json
{
  "mcpServers": {
    "pierre-fitness": {
      "url": "http://127.0.0.1:8080/mcp",
      "headers": {
        "Authorization": "Bearer YOUR_JWT_TOKEN"
      }
    }
  }
}
```

### Option 2: OAuth 2.0 with mcp-remote (Recommended)
```bash
# Use mcp-remote for automatic OAuth 2.0 authentication
mcp-remote http://localhost:8080/mcp --allow-http
```

The OAuth 2.0 flow will automatically:
1. Register as an OAuth 2.0 client with Pierre
2. Obtain authorization code via OAuth 2.0 flow
3. Exchange code for JWT access token
4. Use JWT for authenticated MCP requests

Replace `YOUR_JWT_TOKEN` with the JWT token obtained from the authentication process or use mcp-remote for automatic OAuth 2.0 authentication.

## Available Tools

<details>
<summary>Core Fitness Data Tools</summary>

| Tool | Description | Required Parameters |
|------|-------------|-------------------|
| `get_activities` | Get activities from fitness providers | `limit` (optional) |
| `get_athlete` | Get athlete information | None |
| `get_stats` | Get athlete statistics | None |
| `get_activity_intelligence` | Get AI intelligence for activity | `activity_id` |
| `get_connection_status` | Check provider connection status | None |
| `disconnect_provider` | Disconnect and remove stored tokens for a specific fitness provider | `provider` |

</details>

<details>
<summary>Analytics & Performance Tools</summary>

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

</details>

<details>
<summary>Goal & Training Tools</summary>

| Tool | Description | Required Parameters |
|------|-------------|-------------------|
| `set_goal` | Set a fitness goal | `goal_type`, `target_value` |
| `track_progress` | Track progress toward a specific goal | `goal_id` |
| `suggest_goals` | Generate AI-powered goal suggestions | None |
| `analyze_goal_feasibility` | Assess whether a goal is realistic and achievable | `goal_data` |

</details>

<details>
<summary>Configuration Tools</summary>

| Tool | Description | Required Parameters |
|------|-------------|-------------------|
| `get_configuration_catalog` | Get the complete configuration catalog with all available parameters | None |
| `get_configuration_profiles` | Get available configuration profiles (Research, Elite, Recreational, etc.) | None |
| `get_user_configuration` | Get current user's configuration settings and overrides | None |
| `update_user_configuration` | Update user's configuration parameters and session overrides | `profile` or `parameters` |
| `calculate_personalized_zones` | Calculate personalized training zones based on user's VO2 max and configuration | None |
| `validate_configuration` | Validate configuration parameters against safety rules and constraints | `parameters` |

</details>

## Plugin System

Pierre MCP Server features a compile-time plugin architecture for extensible functionality:

```rust
use pierre_mcp_server::plugins::core::{PluginCategory, PluginImplementation, PluginInfo, PluginToolStatic};
use pierre_mcp_server::plugins::PluginEnvironment;
use pierre_mcp_server::protocols::universal::{UniversalRequest, UniversalResponse};
use pierre_mcp_server::protocols::ProtocolError;
use pierre_mcp_server::{impl_static_plugin, plugin_info, register_plugin};
use async_trait::async_trait;

pub struct CustomAnalysisPlugin;

impl PluginToolStatic for CustomAnalysisPlugin {
    fn new() -> Self { Self }
    
    const INFO: PluginInfo = plugin_info!(
        name: "custom_analysis",
        description: "Custom fitness analysis",
        category: PluginCategory::Analytics,
        input_schema: r#"{"type": "object", "properties": {"activity_id": {"type": "string"}}}"#,
        credit_cost: 1,
        author: "Your Team",
        version: "1.0.0",
    );
}

#[async_trait]
impl PluginImplementation for CustomAnalysisPlugin {
    async fn execute_impl(
        &self,
        request: UniversalRequest,
        env: PluginEnvironment<'_>,
    ) -> Result<UniversalResponse, ProtocolError> {
        // Plugin implementation
        todo!("Implement custom analysis logic")
    }
}

// Use macro to implement required traits
impl_static_plugin!(CustomAnalysisPlugin);

// Register plugin for automatic discovery
register_plugin!(CustomAnalysisPlugin);
```

## A2A (Agent-to-Agent) Protocol

Pierre supports Agent-to-Agent communication for building autonomous fitness agent networks:

**A2A Protocol Features:**
- **Agent Cards**: Self-describing agent capabilities and identity
- **Secure Communication**: Cryptographic authentication between agents
- **Async Messaging**: Non-blocking inter-agent communication
- **Protocol Versioning**: Forward-compatible A2A message format

**A2A Endpoints:**
- `GET /a2a/agents` - Discover available agents
- `POST /a2a/register` - Register new agent
- `POST /a2a/message` - Send message to agent
- `GET /a2a/agent/{id}/capabilities` - Get agent capabilities

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
- `GET /notifications/sse/{user_id}` - Subscribe to user notifications
- `GET /oauth/notifications/sse/{user_id}` - Subscribe to OAuth flow updates

**Notification Types:**
- OAuth authorization completion
- OAuth errors and failures
- System status updates
- A2A message notifications

**Example SSE Integration:**
```javascript
const eventSource = new EventSource('/notifications/sse/user-123');

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

1. Create admin account and approve users:
```bash
# Create admin user (single server on port 8080)
ADMIN_RESPONSE=$(curl -s -X POST http://localhost:8080/admin/setup \
  -H "Content-Type: application/json" \
  -d '{"email": "admin@example.com", "password": "SecurePass123!", "display_name": "Admin"}')

ADMIN_TOKEN=$(echo $ADMIN_RESPONSE | jq -r '.admin_token')

# Register and approve user
USER_ID=$(curl -s -X POST http://localhost:8080/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email": "user@example.com", "password": "pass123", "display_name": "User"}' | jq -r '.user_id')

curl -s -X POST "http://localhost:8080/admin/approve-user/$USER_ID" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"reason": "Approved", "create_default_tenant": true}'
```

2. Get JWT token for MCP integration:
```bash
# Direct JWT token approach
JWT_TOKEN=$(curl -s -X POST http://localhost:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email": "user@example.com", "password": "pass123"}' | jq -r '.jwt_token')

# OR use OAuth 2.0 flow via mcp-remote (recommended)
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
PIERRE_MASTER_ENCRYPTION_KEY=your_32_byte_base64_key
```

#### Optional
```bash
# Server Configuration (Consolidated Architecture)
HTTP_PORT=8080  # Single port for all protocols (MCP + OAuth 2.0 + REST API)
HOST=localhost

# Logging
RUST_LOG=info

# Database (Production)
DATABASE_URL=postgresql://user:pass@localhost:5432/pierre

# OAuth 2.0 Configuration
STRAVA_CLIENT_ID=your_strava_client_id
STRAVA_CLIENT_SECRET=your_strava_client_secret
STRAVA_REDIRECT_URI=http://localhost:8080/api/oauth/callback/strava

# JWT Configuration
JWT_EXPIRY_HOURS=24
```

### Fitness Configuration

Pierre supports comprehensive fitness configuration through `fitness_config.toml`:

```toml
[zones.heart_rate]
zone_1_max = 142
zone_2_max = 152
zone_3_max = 162
zone_4_max = 172
zone_5_max = 182

[zones.power]
ftp = 250
zone_1_max = 144  # 58% of FTP
zone_2_max = 175  # 70% of FTP
zone_3_max = 205  # 82% of FTP
zone_4_max = 235  # 94% of FTP
zone_5_max = 325  # 130% of FTP

[athlete_profile]
weight_kg = 70.0
max_heart_rate = 190
resting_heart_rate = 45
vo2_max = 55.0
```

## Architecture

Pierre MCP Server implements a consolidated multi-protocol, multi-tenant architecture on a single port:

- **Single Server Port**: All protocols consolidated on port 8080 for simplicity
- **MCP Protocol**: JSON-RPC with conditional authentication (discovery methods = no auth, execution = JWT auth)
- **OAuth 2.0 Authorization Server**: RFC-compliant server for mcp-remote compatibility
- **HTTP REST API**: Management endpoints and legacy OAuth flows
- **A2A Protocol**: Agent-to-Agent communication
- **Plugin System**: Extensible compile-time plugin architecture
- **Multi-tenant**: Isolated data access with tenant management
- **JWT Authentication**: Standards-compliant token-based authentication

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