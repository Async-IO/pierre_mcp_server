# Pierre MCP Server

[![CI](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/ci.yml/badge.svg)](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/ci.yml)
[![Frontend Tests](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/frontend-tests.yml/badge.svg)](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/frontend-tests.yml)

**Development Status**: This project is under active development. APIs and features may change.

A comprehensive MCP (Model Context Protocol) server implementation for fitness data access, analytics, and intelligence. Connects AI assistants and autonomous agents to fitness providers through secure OAuth integration with advanced data analysis capabilities.

## Use Cases

- Fitness Data Analysis: Access and analyze activities from Strava, Fitbit, and other providers
- Performance Intelligence: Generate insights from training data with weather and location context
- AI Assistant Integration: Enable AI assistants to work with fitness data
- Autonomous Agent Systems: Build fitness-focused AI agents with comprehensive data access
- Multi-tenant Applications: Support multiple users and organizations with isolated data access

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
docker run -p 8080:8080 -p 8081:8081 pierre-mcp-server
```

## MCP Client Configuration

Configure your MCP client to connect to Pierre MCP Server by adding the following to your client's configuration file:

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

Replace `YOUR_JWT_TOKEN` with the JWT token obtained from the authentication process described in the Authentication section below.

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

## Authentication & Security

### JWT Token Authentication

1. Create admin account and approve users:
```bash
# Create admin
ADMIN_RESPONSE=$(curl -s -X POST http://localhost:8081/admin/setup \
  -H "Content-Type: application/json" \
  -d '{"email": "admin@example.com", "password": "SecurePass123!", "display_name": "Admin"}')

ADMIN_TOKEN=$(echo $ADMIN_RESPONSE | jq -r '.admin_token')

# Register and approve user
USER_ID=$(curl -s -X POST http://localhost:8081/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email": "user@example.com", "password": "pass123", "display_name": "User"}' | jq -r '.user_id')

curl -s -X POST "http://localhost:8081/admin/approve-user/$USER_ID" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"reason": "Approved", "create_default_tenant": true}'
```

2. Get JWT token for MCP integration:
```bash
JWT_TOKEN=$(curl -s -X POST http://localhost:8081/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email": "user@example.com", "password": "pass123"}' | jq -r '.jwt_token')
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
# Server Ports
MCP_PORT=8080
HTTP_PORT=8081

# Logging
RUST_LOG=info

# Database (Production)
DATABASE_URL=postgresql://user:pass@localhost:5432/pierre

# OAuth Providers (shared across all users by default)
STRAVA_CLIENT_ID=your_strava_client_id
STRAVA_CLIENT_SECRET=your_strava_client_secret
FITBIT_CLIENT_ID=your_fitbit_client_id
FITBIT_CLIENT_SECRET=your_fitbit_client_secret
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

Pierre MCP Server implements a multi-protocol, multi-tenant architecture:

- **MCP Protocol**: JSON-RPC over stdio and HTTP transports (port 8080)
- **HTTP REST API**: Management and OAuth endpoints (port 8081)  
- **A2A Protocol**: Agent-to-Agent communication
- **Plugin System**: Extensible compile-time plugin architecture
- **Multi-tenant**: Isolated data access with tenant management
- **OAuth Integration**: Secure provider authentication

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

## Documentation

Comprehensive documentation is available in the `docs/` directory:

- **[Getting Started](docs/developer-guide/15-getting-started.md)** - Quick setup guide
- **[Installation Guides](docs/installation-guides/)** - Platform-specific installation
- **[Developer Guide](docs/developer-guide/)** - Complete technical documentation
- **[Plugin System](docs/developer-guide/18-plugin-system.md)** - Plugin development guide
- **[API Reference](docs/developer-guide/14-api-reference.md)** - Complete API documentation
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