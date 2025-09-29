# Pierre MCP Server Developer Guide

## Table of Contents

1. [Architecture Overview](./01-architecture.md)
2. [Core Components](./02-core-components.md)
3. [Server Implementation](./03-server-implementation.md)
4. [MCP Protocol](./04-mcp-protocol.md)
5. [A2A Protocol](./05-a2a-protocol.md)
6. [Authentication & Security](./06-authentication.md)
7. [Tenant Management](./07-tenant-management.md)
8. [Database Layer](./08-database.md)
9. [API Routes](./09-api-routes.md)
10. [Sequence Diagrams](./10-sequence-diagrams.md)
11. [Architecture Diagrams](./11-architecture-diagrams.md)
12. [Configuration System](./12-configuration.md)
13. [Rate Limiting](./13-rate-limiting.md)
14. [API Reference](./14-api-reference.md)
15. [Getting Started](./15-getting-started.md)
16. [Testing Strategy](./16-testing-strategy.md)
17. [Security Guide](./17-security-guide.md)
18. [Plugin System](./18-plugin-system.md)
19. [Logging and Observability](./19-logging-and-observability.md)
20. [OAuth 2.0 Server Implementation](./20-oauth2-server.md)
21. [Real-Time Notifications & SSE](./21-notifications-sse.md)
22. [A2A Integration Guide](./A2A-INTEGRATION-GUIDE.md)

## Overview

Pierre MCP Server is a high-performance multi-protocol fitness data API platform designed for LLMs and AI assistants. Built in Rust for enterprise-grade performance, the system provides fitness data aggregation, analysis, and intelligence capabilities through multiple protocols:

- **MCP (Model Context Protocol)**: Primary interface for AI assistants like Claude
- **A2A (Agent-to-Agent)**: Inter-agent communication with cryptographic authentication
- **OAuth 2.0 Authorization Server**: RFC-compliant server supporting MCP client integration
- **REST API**: Traditional HTTP endpoints for web applications and admin operations
- **Server-Sent Events**: Real-time notifications for OAuth flows and system events
- **WebSocket**: Real-time bidirectional communication support

## Setup

```bash
# Install dependencies
cargo build --release

# Set up database and authentication
cargo run --bin admin-setup -- create-admin-user

# Start the server
./target/release/pierre-mcp-server
```

## Architecture Principles

### 1. Multi-Tenancy
All users operate in isolated tenant contexts with their own data, configurations, and rate limits.

### 2. Protocol Agnostic Core
Business logic is separated from protocol handlers, allowing multiple interfaces to the same functionality.

### 3. Provider Abstraction
Fitness data providers (Strava, Fitbit, etc.) are abstracted behind a common interface.

### 4. Security First
- Two-tier key management system (MEK/DEK)
- JWT-based authentication with database-stored secrets
- AES-256-GCM encrypted token storage
- Per-tenant data isolation
- Multi-layer rate limiting
- Comprehensive audit logging

### 5. Intelligence Layer
Analytics and recommendations powered by physiological models and ML algorithms.

## Development Workflow

1. Understanding the Codebase: Start with the architecture overview
2. Setting Up Development: Follow the deployment guide for local setup
3. Adding Features: Review core components and relevant protocol documentation
4. Testing: Test coverage includes unit, integration, and e2e tests
5. Deployment: Container-based deployment with health checks and monitoring

## Key Technologies

- Rust: Core language for performance and safety
- Tokio: Async runtime for concurrent operations
- SQLite/PostgreSQL: Database backends with encrypted storage
- JWT: Authentication tokens
- MCP: Model Context Protocol for AI assistants
- WebSocket: Real-time communication
- Docker: Containerization for deployment

## Project Structure

```
pierre_mcp_server/
├── src/
│   ├── bin/              # Binary entry points (server, client, admin-setup)
│   ├── a2a/              # A2A (Agent-to-Agent) protocol implementation
│   ├── admin/            # Admin functionality and user management
│   ├── config/           # Configuration management and environment setup
│   ├── configuration/    # Runtime configuration and dynamic settings
│   ├── crypto/           # Cryptographic utilities and key management
│   ├── database/         # Database modules (users, tokens, OAuth, A2A, analytics)
│   ├── database_plugins/ # Database abstraction layer (SQLite/PostgreSQL backends)
│   ├── oauth2/           # OAuth 2.0 Authorization Server (RFC 7591 compliant)
│   ├── notifications/    # Real-time notifications and SSE implementation
│   ├── plugins/          # Compile-time plugin system with inventory registration
│   ├── middleware/       # HTTP middleware for tracing and tenant context
│   ├── intelligence/     # Analytics and recommendations
│   ├── key_management/   # Two-tier key management system
│   ├── mcp/             # MCP protocol implementation
│   ├── oauth/           # OAuth management
│   ├── protocols/       # Protocol converters and universal handlers
│   ├── providers/       # Fitness data providers
│   ├── security/        # Security components
│   ├── tenant/          # Tenant management
│   ├── tools/           # Tool implementations
│   └── utils/           # Utilities
├── tests/               # Integration and unit tests
└── docs/               # Documentation
```

## Contributing

Please review the [CONTRIBUTING.md](../../CONTRIBUTING.md) file for guidelines on contributing to this project.

## License

Licensed under either Apache License 2.0 or MIT License at your option.