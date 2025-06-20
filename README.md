# Pierre Fitness API

[![CI](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/ci.yml/badge.svg)](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/ci.yml)
[![Frontend Tests](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/frontend-tests.yml/badge.svg)](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/frontend-tests.yml)

A multi-protocol fitness data API providing secure access to fitness data from multiple providers (Strava, Fitbit) through the [Model Context Protocol](https://modelcontextprotocol.io/specification/draft) (MCP), [A2A](https://github.com/google-a2a/A2A) (Agent-to-Agent) Protocol, and REST APIs. Built for LLMs and AI applications, Pierre Fitness API features comprehensive API key management with tiered rate limiting, trial keys with automatic expiration, OAuth integration, real-time analytics, comprehensive activity intelligence, and multi-protocol support for AI agent communication. Architecture technical details availaibe on [DeepWiki](https://deepwiki.com/Async-IO/pierre_mcp_server) 

## 🤖 AI-Powered Fitness Analysis

Pierre connects your fitness data to AI assistants like Claude, ChatGPT, and any agents supporting the MCP/A2A protocol, providing intelligent analysis with location, weather, and performance context.

| Analysis Type | Example Queries | Key Features |
|---------------|----------------|--------------|
| **🏃 Running** | "What was my longest run this year and where?" | Pace trends, location context, terrain analysis |
| **🚴 Cross-Training** | "Compare my cycling vs running performance" | Multi-sport analysis, heart rate zones, consistency |
| **🗺️ Location Intelligence** | "Where do I perform best?" | GPS tracking, terrain impact, route optimization |
| **🌦️ Weather Impact** | "How does weather affect my performance?" | Temperature correlation, seasonal patterns |
| **📊 Activity Intelligence** | "Analyze my marathon with full context" | AI-powered insights, environmental factors |
| **🎯 Goal Tracking** | "How close am I to my 1000km goal?" | Progress monitoring, achievement analysis |
| **📈 Performance Trends** | "Find patterns in my training data" | Long-term analysis, optimization suggestions |

> 💡 **See detailed examples**: Check out our [comprehensive prompt guide](docs/PROMPT_EXAMPLES.md) with 50+ real-world queries.

## 🔌 Integration Methods

Pierre supports multiple integration patterns for different use cases:

| Integration Type | Best For | Setup Complexity | Authentication |
|------------------|----------|------------------|----------------|
| **🤖 MCP Protocol** | AI assistants (Claude, ChatGPT, Copilot) | Low | JWT Token |
| **🔗 A2A Protocol** | AI agents & applications | Medium | API Keys |
| **🌐 REST API** | Web apps & dashboards | Medium | OAuth2 + JWT |
| **🏠 Single-Tenant** | Personal local use | Minimal | Optional |

### Quick Setup Examples

<details>
<summary><strong>🤖 AI Assistant Integration (Claude, ChatGPT, etc.)</strong></summary>

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
<summary><strong>🔗 A2A Integration for Developers</strong></summary>

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

> 📖 **Detailed guides**: See our [setup documentation](docs/SETUP.md) for complete integration examples.

## 📚 Documentation

| Guide | Description | Key Topics |
|-------|-------------|------------|
| **📋 [Setup & Installation](docs/SETUP.md)** | Get started quickly | Local setup, OAuth config, Docker deployment |
| **🛠️ [MCP Tools Reference](docs/TOOLS.md)** | All 21 fitness tools | Data access, intelligence, goals, analytics |
| **🌦️ [Weather Integration](docs/WEATHER.md)** | Weather-enhanced analysis | OpenWeatherMap setup, mock data, intelligence |
| **🔑 [API Reference](docs/API_REFERENCE.md)** | Enterprise features | API keys, rate limiting, security, monitoring |
| **🏢 [Business Provisioning](docs/PROVISIONING.md)** | B2B deployment guide | API key provisioning, multi-tenant setup, admin workflows |
| **🚀 [Deployment Guide](docs/DEPLOYMENT.md)** | Production deployment | Docker, Kubernetes, cloud platforms |
| **🏗️ [Architecture Guide](docs/ARCHITECTURE.md)** | Technical deep-dive | System design, extensibility, contribution areas |

## ⭐ Key Features

| Category | Features |
|----------|----------|
| **🔗 Integrations** | Strava, Fitbit • MCP Protocol • A2A Protocol • REST APIs |
| **🛡️ Security** | OAuth2 + PKCE • JWT Authentication • Encrypted storage • Rate limiting |
| **🧠 Intelligence** | AI activity analysis • Location detection • Weather integration |
| **🏗️ Architecture** | Single & multi-tenant • Cloud-ready • Extensible • Deployment-ready |

## 🚀 Quick Start

| Method | Command | Use Case |
|--------|---------|----------|
| **🏠 Local** | `cargo run --bin pierre-mcp-server -- --single-tenant` | Personal use, development |
| **🐳 Docker** | `./docker-compose-with-envrc.sh up` | Easy deployment, cloud-ready |
| **🤖 AI Assistants** | Add to MCP client config | Claude, ChatGPT, agent integration |

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

## 🏗️ Architecture

| Mode | Best For | Features |
|------|----------|----------|
| **🏠 Single-Tenant** | Personal use | Local config, no auth required, simple setup |
| **☁️ Multi-Tenant** | Cloud deployment | JWT auth, encrypted storage, user isolation |

## 🌟 Community & Ecosystem

Pierre is an **open-source project** built by fitness enthusiasts, AI developers, and protocol engineers. We're building the foundation for intelligent fitness applications.

### 🎯 **Why Contribute?**
- **Learn cutting-edge tech**: MCP protocol, A2A communication, Rust async, AI integration
- **Real impact**: Used by developers building AI fitness applications
- **Growing ecosystem**: New fitness providers, client libraries, and integrations needed
- **Welcoming community**: All skill levels welcome, mentorship available

### 🚀 **Quick Ways to Contribute**
- 🐛 **Report bugs** or suggest features via GitHub Issues
- 📚 **Improve docs** - add examples, fix typos, clarify guides  
- 🔌 **Add providers** - Garmin, Polar, Suunto, local CSV imports
- 🐍 **Build clients** - Go, JavaScript, Ruby, PHP client libraries
- 🧪 **Write tests** - improve reliability and coverage

**[📖 See full contribution guide below ⬇️](#-contributing)**

## License

This project is dual-licensed under either of:

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
* MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

## 🤝 Contributing

We welcome contributions from developers of all skill levels! Pierre is built by the community, for the community.

### 🚀 Quick Start for Contributors

1. **Fork the repository** and clone locally
2. **Read the code** - architecture is documented in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
3. **Pick an area**:
   - 🦀 **Rust backend**: Core MCP/A2A protocols, fitness providers, database layers
   - ⚛️ **TypeScript frontend**: Admin dashboard, analytics UI, real-time monitoring
   - 🐍 **Python examples**: Client libraries, integration demos, data processing
   - 📚 **Documentation**: Guides, examples, API references

### 🎯 Contribution Areas

| Area | Good for | Examples |
|------|----------|----------|
| **New Fitness Providers** | API integration experience | Garmin, Polar, Suunto connectors |
| **MCP Protocol Extensions** | Protocol/networking knowledge | New tool types, enhanced schemas |
| **Analytics & Intelligence** | Data science/ML background | Advanced metrics, AI insights |
| **Client Libraries** | SDK development | Go, JavaScript, Ruby clients |
| **DevOps & Deployment** | Infrastructure expertise | Kubernetes, monitoring, scaling |

### 💡 Ideas & Discussions

- 💬 **GitHub Discussions**: Share ideas, ask questions, propose features
- 🐛 **Issues**: Bug reports, feature requests, technical discussions  
- 📖 **Documentation**: Improve guides, add examples, fix typos
- 🧪 **Testing**: Add test cases, improve CI/CD, performance testing

### 📋 Development Setup

```bash
# Clone and setup
git clone https://github.com/YOUR_USERNAME/pierre_mcp_server.git
cd pierre_mcp_server

# Backend development
cargo build
cargo test
./scripts/lint-and-test.sh

# Frontend development  
cd frontend && npm install && npm run dev

# Documentation
# Edit .md files and test with any markdown renderer
```

### 🏗️ Architecture Overview

Pierre is designed to be modular and extensible:

- **Core**: MCP server, A2A protocol, authentication
- **Providers**: Pluggable fitness data sources (Strava, Fitbit, etc.)
- **Intelligence**: AI-powered analysis and insights  
- **Clients**: Multiple language SDKs and examples

### 🌟 Recognition

Contributors are recognized in:
- Release notes and changelogs
- GitHub contributor graphs
- Documentation credits
- Community showcases

### 📞 Getting Help

- 📖 **Documentation**: Start with [docs/SETUP.md](docs/SETUP.md)
- 💬 **Discussions**: Ask questions in GitHub Discussions
- 🐛 **Issues**: Report bugs or request features
- 📧 **Maintainers**: Tag @maintainers for architectural questions

### 🔄 Contribution Process

1. **Discuss first** for large changes (GitHub Discussions/Issues)
2. **Fork & branch** from main
3. **Write tests** for new functionality  
4. **Run lint**: `./scripts/lint-and-test.sh`
5. **Submit PR** with clear description
6. **Iterate** based on review feedback

### 📜 License

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.