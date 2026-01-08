<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Documentation

Developer documentation for Pierre Fitness Platform.

## Quick Links

### For Users
- [Getting Started](getting-started.md) - Install, configure, connect your AI assistant

### For Developers
1. [Getting Started](getting-started.md) - Setup dev environment
2. [Architecture](architecture.md) - System design
3. [Development Guide](development.md) - Workflow, dashboard, testing
4. [Contributing](contributing.md) - Code standards, PR workflow

### For Integrators
- MCP clients: [Protocols](protocols.md#mcp-model-context-protocol)
- Web apps: [Protocols](protocols.md#rest-api)
- Autonomous agents: [Protocols](protocols.md#a2a-agent-to-agent-protocol)

## Reference Documentation

### Core
- [Getting Started](getting-started.md) - Installation and quick start
- [Architecture](architecture.md) - System design and components
- [Protocols](protocols.md) - MCP, OAuth2, A2A, REST protocols
- [Authentication](authentication.md) - JWT, API keys, OAuth2

### Configuration
- [Configuration](configuration.md) - Settings and algorithms
- [Environment](environment.md) - .envrc variables reference

### APIs
- [Prompts API](prompts-api.md) - REST API for managing AI chat prompts (admin)

### OAuth
- [OAuth Client](oauth-client.md) - Fitness provider connections (Strava, Fitbit, Garmin, WHOOP, COROS, Terra)
- [OAuth2 Server](oauth2-server.md) - MCP client authentication

### Development
- [Development Guide](development.md) - Workflow, dashboard, admin tools
- [Build](build.md) - Rust toolchain, cargo configuration
- [CI/CD](ci-cd.md) - GitHub Actions, pipelines
- [Testing](testing.md) - Test framework, strategies
- [Contributing](contributing.md) - Development guidelines

### Methodology
- [Intelligence Methodology](intelligence-methodology.md) - Sports science formulas
- [Nutrition Methodology](nutrition-methodology.md) - Dietary calculations

## Scripts

Development, testing, and deployment scripts.

- [Scripts Reference](../scripts/README.md) - 30+ scripts documented

Key scripts:
```bash
./bin/start-server.sh     # start backend
./bin/stop-server.sh      # stop backend
./bin/start-frontend.sh   # start dashboard
./scripts/fresh-start.sh  # clean database reset
./scripts/lint-and-test.sh # full CI suite
```

## Tutorial

Comprehensive Rust learning path using Pierre as the codebase.

- [Tutorial Table of Contents](tutorial-table-of-contents.md) - 25 chapters + appendices

### Learning Paths

**Quick Start** (core concepts):
1. Chapter 1 - Architecture
2. Chapter 2 - Error Handling
3. Chapter 9 - JSON-RPC
4. Chapter 10 - MCP Protocol
5. Chapter 19 - Tools Guide

**Security-Focused**:
1. Chapter 5 - Cryptographic Keys
2. Chapter 6 - JWT Authentication
3. Chapter 7 - Multi-Tenant Isolation
4. Chapter 15 - OAuth 2.0 Server

## Component Documentation

- [SDK Documentation](../sdk/README.md) - TypeScript SDK for MCP clients
- [Frontend Documentation](../frontend/README.md) - React dashboard
- [Examples](../examples/README.md) - Sample integrations

## Installation Guides

- [MCP Client Installation](installation-guides/install-mcp-client.md) - Claude Desktop, ChatGPT

## Additional Resources

- OpenAPI spec: `openapi.yaml`
- Main README: [../README.md](../README.md)

## Documentation Style

- **Concise**: Developers don't read walls of text
- **Accurate**: Verified against actual code
- **Practical**: Code examples that work
- **Capitalized**: Section headings start with capital letters
