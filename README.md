# Pierre Fitness API

[![CI](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/ci.yml/badge.svg)](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/ci.yml)
[![Frontend Tests](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/frontend-tests.yml/badge.svg)](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/frontend-tests.yml)

**Active Development** - This project is under active development and not yet ready for production use. We welcome feedback, contributions, and early testing from the community. Please report issues and share your experiences.

An open source multi-protocol fitness data API providing secure access to fitness data from multiple providers (Strava, Fitbit) through the [Model Context Protocol](https://modelcontextprotocol.io/specification/draft) (MCP), [A2A](https://github.com/google-a2a/A2A) (Agent-to-Agent) Protocol, and REST APIs. Built for LLMs and AI applications with comprehensive API key management, rate limiting, OAuth integration, and real-time analytics.

## Fitness Data Analysis

Pierre connects fitness data to AI assistants like Claude, ChatGPT, and any agents supporting the MCP/A2A protocol, providing intelligent analysis with location, weather, and performance context.

| Analysis Type | Example Queries | Key Features |
|---------------|----------------|--------------|
| **Running** | "What was my longest run this year and where?" | Pace trends, location context, terrain analysis |
| **Cross-Training** | "Compare my cycling vs running performance" | Multi-sport analysis, heart rate zones, consistency |
| **Location Intelligence** | "Where do I perform best?" | GPS tracking, terrain impact, route optimization |
| **Weather Impact** | "How does weather affect my performance?" | Temperature correlation, seasonal patterns |
| **Activity Intelligence** | "Analyze my marathon with full context" | Insights with environmental factors |
| **Goal Tracking** | "How close am I to my 1000km goal?" | Progress monitoring, achievement analysis |
| **Performance Trends** | "Find patterns in my training data" | Long-term analysis, optimization suggestions |

> **See detailed examples**: Check out our [comprehensive prompt guide](docs/PROMPT_EXAMPLES.md) with 50+ real-world queries.

## Integration Methods

Pierre supports multiple integration patterns for different use cases:

| Integration Type | Best For | Setup Complexity | Authentication |
|------------------|----------|------------------|----------------|
| **MCP Protocol**<br/>(2025-06-18) | AI assistants ([Claude](https://claude.ai), [ChatGPT](https://chatgpt.com), any MCP compliant client) | Low | JWT Token |
| **A2A Protocol**<br/>(v0.2.3) | AI agents & applications | Medium | API Keys |
| **REST API** | Web apps & dashboards | Medium | OAuth2 + JWT |
| **Single-Tenant** | Personal local use | Minimal | Optional |

### Quick Setup Examples

<details>
<summary><strong>AI Assistant Integration (Claude, ChatGPT, etc.)</strong></summary>

1. **Configure MCP Server**
   ```json
   // For Claude Desktop (~/.claude/claude_desktop_config.json)
   {
     "mcpServers": {
       "pierre-fitness": {
         "command": "path/to/pierre-mcp-server",
         "args": ["--single-tenant", "--port", "8080"]
       }
     }
   }
   
   // For ChatGPT or other MCP-compatible clients
   // Use the same MCP protocol with your client's configuration
   ```

2. **Connect to Strava**
   - Visit the OAuth URL provided by Pierre
   - Authorize access to your Strava data
   - Start asking questions in natural language

3. **Works with any MCP/A2A compatible agent**
   - Claude Desktop, ChatGPT with MCP support
   - Custom AI agents, GitHub Copilot extensions
   - Any application supporting MCP or A2A protocols

</details>

<details>
<summary><strong>A2A Integration for Developers</strong></summary>

```bash
# Register your application
curl -X POST https://your-pierre-server.com/a2a/clients \
  -H "Authorization: Bearer JWT_TOKEN" \
  -d '{"name": "My Fitness App", "description": "AI fitness coach"}'

# Execute fitness tools
curl -X POST https://your-pierre-server.com/a2a/execute \
  -H "Authorization: Bearer API_KEY" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools.execute",
    "params": {
      "tool_name": "get_activities",
      "parameters": {"provider": "strava", "limit": 10}
    }
  }'
```

</details>

> **Detailed guides**: See our [setup documentation](docs/SETUP.md) for complete integration examples.

## Documentation

| Guide | Description | Key Topics |
|-------|-------------|------------|
| **[Documentation Index](docs/README.md)** | **Complete doc guide** | **All documentation organized by use case** |
| **[Getting Started](docs/GETTING_STARTED.md)** | Setup and configuration | Local setup, OAuth config, Docker deployment |
| **[API Reference](docs/API_REFERENCE.md)** | **Complete API documentation** | **MCP tools, HTTP endpoints, error handling** |
| **[OpenAPI Specification](docs/openapi.yaml)** | **Interactive API reference** | **Complete API spec with examples** |
| **[Database Guide](docs/DATABASE_GUIDE.md)** | Database setup | SQLite/PostgreSQL setup and database plugins |
| **[Deployment Guide](docs/DEPLOYMENT_GUIDE.md)** | Production deployment | Docker, Kubernetes, cloud platforms |

## Key Features

| Category | Features |
|----------|----------|
| **Integrations** | Strava, Fitbit • MCP Protocol • A2A Protocol • REST APIs |
| **Security** | OAuth2 + PKCE • JWT Authentication • Encrypted storage • Rate limiting |
| **Intelligence** | Activity analysis • Location detection • Weather integration |
| **Architecture** | Single & multi-tenant • Cloud-ready • Extensible • Deployment-ready |

## Quick Start

| Method | Command | Use Case |
|--------|---------|----------|
| **Local** | `cargo run --bin pierre-mcp-server -- --single-tenant` | Personal use, development |
| **Docker** | `./docker-compose-with-envrc.sh up` | Easy deployment, cloud-ready |
| **AI Assistants** | Add to MCP client config | Claude, ChatGPT, agent integration |

### One-Minute Setup
```bash
# 1. Clone and build
git clone https://github.com/Async-IO/pierre_mcp_server.git
cd pierre_mcp_server && cargo build --release

# 2. Run locally (single-tenant mode)
cargo run --bin pierre-mcp-server -- --single-tenant

# 3. Configure AI Assistant (Claude, ChatGPT, etc.)
echo '{
  "mcpServers": {
    "pierre-fitness": {
      "command": "'$(pwd)'/target/release/pierre-mcp-server",
      "args": ["--single-tenant", "--port", "8080"]
    }
  }
}' > ~/.claude/claude_desktop_config.json
```

> **Fresh Start**: Need to clean your database? See our [Database Cleanup Guide](docs/DATABASE_CLEANUP.md).

## Architecture

| Mode | Best For | Features |
|------|----------|----------|
| **Single-Tenant** | Personal use | Local config, no auth required, simple setup |
| **Multi-Tenant** | Cloud deployment | JWT auth, encrypted storage, user isolation |

## License

This project is dual-licensed under either of:

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
* MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for details on how to get started.