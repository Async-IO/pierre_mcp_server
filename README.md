# Pierre MCP Server

[![CI](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/ci.yml/badge.svg)](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/ci.yml)
[![Frontend Tests](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/frontend-tests.yml/badge.svg)](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/frontend-tests.yml)

> **Development Status**: This project is under active development. APIs and features may change.

**MCP Server** and **A2A Protocol** implementation for fitness data access. Connects AI assistants (Claude, ChatGPT) and autonomous agents to fitness providers (Strava, Fitbit) with OAuth credential management.

## Quick Start

```bash
git clone https://github.com/Async-IO/pierre_mcp_server.git
cd pierre_mcp_server
cargo build --release

# Start server
cargo run --bin pierre-mcp-server
```

### Automated Setup Script

For development and testing, use the complete user workflow script:

```bash
# Clean database and start fresh server
./scripts/fresh-start.sh
source .envrc && RUST_LOG=debug cargo run --bin pierre-mcp-server &

# Run complete 5-step workflow (admin + user + tenant + login + MCP test)
./scripts/complete-user-workflow.sh

# Use saved environment variables
source .workflow_test_env
echo "Ready! JWT Token: ${JWT_TOKEN:0:50}..."
```

This automated script performs all 5 setup steps and saves tokens to `.workflow_test_env` for easy reuse.

## MCP Protocol Integration

### Claude Desktop Setup

1. **Create admin and user accounts:**
```bash
# Create admin
ADMIN_RESPONSE=$(curl -s -X POST http://localhost:8081/admin/setup \
  -H "Content-Type: application/json" \
  -d '{"email": "admin@example.com", "password": "SecurePass123!", "display_name": "Admin"}')

ADMIN_TOKEN=$(echo $ADMIN_RESPONSE | jq -r '.admin_token')

# Register user  
USER_ID=$(curl -s -X POST http://localhost:8081/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email": "user@example.com", "password": "pass123", "display_name": "User"}' | jq -r '.user_id')

# Approve user with tenant
curl -s -X POST "http://localhost:8081/admin/approve-user/$USER_ID" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"reason": "Approved", "create_default_tenant": true, "tenant_name": "User Org", "tenant_slug": "user-org"}'

# Get JWT token for MCP
JWT_TOKEN=$(curl -s -X POST http://localhost:8081/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email": "user@example.com", "password": "pass123"}' | jq -r '.jwt_token')
```

2. **Configure Claude Desktop** (`~/.claude/claude_desktop_config.json`):
```json
{
  "mcpServers": {
    "pierre-fitness": {
      "command": "/path/to/pierre_mcp_server/scripts/mcp-client.sh",
      "env": {
        "PIERRE_JWT_TOKEN": "YOUR_JWT_TOKEN_FROM_ABOVE",
        "PIERRE_SERVER_URL": "http://127.0.0.1:8080/mcp"
      }
    }
  }
}
```

## Supported MCP Tools

### Core Fitness Data
- `get_activities` - Fetch activities from fitness providers
- `get_athlete` - Get athlete profile information  
- `get_stats` - Get fitness statistics and metrics
- `get_activity_intelligence` - Detailed activity analysis with weather/location
- `get_connection_status` - Check provider connection status
- `disconnect_provider` - Disconnect from fitness provider

### Analytics & Performance
- `analyze_activity` - Deep dive analysis of specific activities
- `calculate_metrics` - Compute custom performance metrics
- `analyze_performance_trends` - Track performance over time
- `compare_activities` - Compare multiple activities
- `detect_patterns` - Find patterns in training data
- `predict_performance` - Predict future performance
- `generate_recommendations` - Generate personalized training recommendations

### Goal Management
- `create_goal` - Create fitness goals
- `get_goals` - Get all user goals
- `suggest_goals` - AI-suggested goals based on history

### Provider Management
- `connect_provider` - Connect to a fitness data provider (Strava, Fitbit)

### Weather & Context
- `get_weather_for_activity` - Get weather conditions for activities

### Example MCP Client Questions

**Activity Analysis:**
- "Show me my last 5 runs and analyze my pace trends"
- "What was my best cycling activity this month?"
- "Compare my morning runs vs evening runs this week"

**Performance Insights:**
- "Analyze my training load over the past month"
- "What patterns do you see in my workout data?"  
- "Predict my 5K time based on my recent training"

**Goal Setting:**
- "Help me create a realistic marathon training goal"
- "Show me all my current fitness goals"
- "What goals should I focus on based on my fitness level?"

**Weather & Context:**
- "What were the weather conditions during my last run?"
- "Show me how weather affected my cycling performance"

## A2A Protocol Integration

The A2A (Agent-to-Agent) protocol enables autonomous agents to access fitness data programmatically.

### A2A Client Registration
```bash
# Register A2A client
curl -X POST http://localhost:8081/a2a/clients \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{
    "name": "My Agent",
    "description": "Fitness data analysis agent", 
    "capabilities": ["fitness-data-analysis"]
  }'
```

### A2A Authentication Flow
```bash
# Authenticate A2A client
curl -X POST http://localhost:8081/a2a/auth \
  -H "Content-Type: application/json" \
  -d '{
    "client_id": "YOUR_CLIENT_ID",
    "client_secret": "YOUR_CLIENT_SECRET"
  }'
```

## Architecture

- **Port 8080**: MCP Protocol server (JSON-RPC over HTTP)
- **Port 8081**: HTTP API (A2A protocol, admin setup, authentication)

## Documentation

[Complete Documentation](docs/README.md)

**Integration Guides:**
- [MCP Protocol Guide](docs/developer-guide/04-mcp-protocol.md) - MCP implementation details
- [A2A Protocol Guide](docs/developer-guide/05-a2a-protocol.md) - Agent integration
- [A2A Quick Start](docs/A2A_QUICK_START.md) - A2A setup guide
- [Getting Started](docs/getting-started.md) - Server setup and configuration

**Developer Resources:**
- [API Reference](docs/developer-guide/14-api-reference.md) - REST API and tool documentation
- [Architecture Guide](docs/developer-guide/01-architecture.md) - System design
- [Agent Examples](examples/agents/) - Autonomous agent implementations

## License

This project is dual-licensed under either of:

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
* MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.