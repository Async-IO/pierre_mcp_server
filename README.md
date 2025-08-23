# Pierre MCP Server - Developer Guide

[![CI](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/ci.yml/badge.svg)](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/ci.yml)
[![Frontend Tests](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/frontend-tests.yml/badge.svg)](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/frontend-tests.yml)

> ⚠️ **Development Status**: This project is under active development. APIs and features may change.

MCP server implementation for fitness data access from Strava and Fitbit providers. Supports MCP protocol, A2A protocol, and REST APIs with OAuth credential management and user authentication.

## Architecture Overview

**Two-Component Architecture**: This system has clear separation between server and client:

1. **Pierre MCP Server** (`pierre-mcp-server`) - Runs as daemon with database access
   - Handles all fitness data operations
   - Two-tier key management system (MEK/DEK)
   - Manages OAuth credentials with AES-256-GCM encryption
   - Enforces admin approval for new users
   - Serves HTTP API and MCP endpoints

2. **Pierre MCP Client** (`pierre-mcp-client`) - Lightweight MCP client for Claude Desktop
   - Connects to running server via HTTP
   - Translates MCP protocol to HTTP API calls
   - Stateless design

## Protocol Support

### MCP Protocol (Model Context Protocol)
- **Version**: Draft specification (2025-06-18)
- **Endpoint**: Port 8080
- **Authentication**: JWT tokens
- **Use Cases**: AI assistants (Claude, ChatGPT), LLM applications

### A2A Protocol (Agent-to-Agent)
- **Version**: v0.2.3
- **Endpoint**: Port 8081 `/a2a/*`
- **Authentication**: Client credentials + JWT
- **Use Cases**: Enterprise integrations, autonomous agents

### REST API
- **Endpoint**: Port 8081
- **Authentication**: JWT tokens
- **Use Cases**: Web applications, dashboards

## Documentation

### 📚 Developer Guide
**[Complete Developer Guide](docs/developer-guide/README.md)** - Comprehensive technical documentation with 17 sections covering architecture, protocols, API reference, testing, and more.

#### Quick Links
- [Getting Started](docs/developer-guide/15-getting-started.md) - Setup and development guide
- [API Reference](docs/developer-guide/14-api-reference.md) - Complete REST API documentation
- [System Architecture](docs/developer-guide/01-architecture.md) - Design patterns and structure
- [MCP Protocol](docs/developer-guide/04-mcp-protocol.md) - Model Context Protocol implementation
- [A2A Protocol](docs/developer-guide/05-a2a-protocol.md) - Agent-to-Agent protocol
- [A2A Integration Examples](docs/developer-guide/A2A-INTEGRATION-GUIDE.md) - Discord bots, IoT, analytics
- [Security Guide](docs/developer-guide/17-security-guide.md) - Two-tier key management, encryption, production security

### 🚀 Quick Start
- [A2A Quick Start](docs/A2A_QUICK_START.md) - 5-minute A2A setup
- [Deployment Guide](docs/DEPLOYMENT_GUIDE.md) - Docker & Kubernetes deployment
- [Database Guide](docs/DATABASE_GUIDE.md) - Database setup and migrations

## Quick Setup Guide

### Prerequisites

1. **Rust toolchain** (1.75+): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
2. **Strava app**: Create at [developers.strava.com](https://developers.strava.com)
3. **Database**: SQLite (default) or PostgreSQL
4. **Master Encryption Key**: Optional for development (auto-generated), required for production security

### Local Development Setup

```bash
# 1. Clone and build
git clone https://github.com/Async-IO/pierre_mcp_server.git
cd pierre_mcp_server
cargo build --release

# 2. (Optional) Set Master Encryption Key for production  
# For development, system auto-generates MEK with warning messages
# For production, set explicit MEK:
export PIERRE_MASTER_ENCRYPTION_KEY="$(openssl rand -base64 32)"
echo "Save this MEK securely: $PIERRE_MASTER_ENCRYPTION_KEY"

# 3. Start the Pierre MCP Server (runs as daemon)
cargo run --bin pierre-mcp-server
# Server starts on http://localhost:8081 (HTTP) and http://localhost:8080 (MCP)

# 4. Create the default tenant (required for user registration)
curl -X POST http://localhost:8081/api/tenants \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Default Organization", 
    "slug": "default-tenant",
    "plan": "starter"
  }'
# Save the tenant "id" (UUID) from the response - you'll need it for OAuth configuration

# 5. Configure OAuth credentials for the default tenant
curl -X POST http://localhost:8081/api/tenants/{TENANT_UUID}/oauth \
  -H "Content-Type: application/json" \
  -d '{
    "provider": "strava",
    "client_id": "YOUR_STRAVA_CLIENT_ID", 
    "client_secret": "YOUR_STRAVA_CLIENT_SECRET",
    "redirect_uri": "http://localhost:8081/api/oauth/callback/strava",
    "scopes": ["read", "activity:read_all"]
  }'
# Replace {TENANT_UUID} with the UUID returned from step 3
```

## MCP Client Integration

MCP server implementation compatible with MCP clients following the Model Context Protocol specification.

### MCP Client Compatibility

| Client | Platform | Configuration |
|--------|----------|---------------|
| **Claude Desktop** | Desktop app | JSON config file |
| **ChatGPT** | With MCP support | Custom integration |
| **Cursor** | IDE | MCP extension |
| **Continue.dev** | VS Code | MCP plugin |
| **Custom agents** | Any platform | Direct MCP protocol |

### MCP Configuration

Standard MCP client binary for MCP-compatible applications:

#### Option 1: Direct MCP Binary
```json
{
  "mcpServers": {
    "pierre-fitness": {
      "command": "/path/to/pierre_mcp_server/target/release/pierre-mcp-client",
      "args": ["--server-url", "http://localhost:8081"],
      "env": {
        "PIERRE_JWT_TOKEN": "YOUR_JWT_TOKEN"
      }
    }
  }
}
```

#### Option 2: HTTP MCP Transport
```bash
# For clients that support HTTP MCP transport
curl -X POST http://localhost:8080/mcp \
  -H "Authorization: Bearer JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/list",
    "params": {},
    "id": 1
  }'
```

### Client-Specific Examples

<details>
<summary><strong>Claude Desktop</strong></summary>

Add to `~/.claude/claude_desktop_config.json`:
```json
{
  "mcpServers": {
    "pierre-fitness": {
      "command": "/path/to/pierre_mcp_server/target/release/pierre-mcp-client",
      "args": ["--server-url", "http://localhost:8081"],
      "env": {
        "PIERRE_JWT_TOKEN": "your-jwt-token"
      }
    }
  }
}
```
</details>

<details>
<summary><strong>Custom MCP Clients</strong></summary>

Use the MCP protocol directly:
```python
import asyncio
import json
from mcp import ClientSession, StdioServerParameters

async def main():
    server_params = StdioServerParameters(
        command="/path/to/pierre-mcp-client",
        args=["--server-url", "http://localhost:8081"],
        env={"PIERRE_JWT_TOKEN": "your-jwt-token"}
    )
    
    async with ClientSession(server_params) as session:
        await session.initialize()
        tools = await session.list_tools()
        print(f"Available tools: {[tool.name for tool in tools]}")
```
</details>

### Get User JWT Token

After user registration and login, get JWT token for MCP client authentication:

```bash
# First register user (creates pending status)
curl -X POST http://localhost:8081/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "secure_password",
    "display_name": "User Name"
  }'

# Admin approves user (see User Management section)
# Then user can login to get JWT token
curl -X POST http://localhost:8081/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "secure_password"
  }'
# Returns jwt_token for MCP client configuration
```

### First Time Setup

1. Start the Pierre MCP Server
2. Create a tenant and configure OAuth credentials
3. Register a user account and get admin approval  
4. Configure your MCP client with the user's JWT token
5. Start your MCP-compatible application
6. Ask about fitness data: "What were my recent activities?"
7. Follow the OAuth URL to authorize Strava access
8. Your fitness data is now available through MCP protocol

## User Management with Admin Approval

### Register New Users (Creates "Pending" Status)

```bash
curl -X POST http://localhost:8081/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "secure_password",
    "display_name": "User Name"
  }'
```

### Admin Approval Required

New users are created with "pending" status and cannot access tools until approved:

```bash
# Admin approves the user
curl -X POST http://localhost:8081/api/admin/users/{user_id}/approve \
  -H "Authorization: Bearer ADMIN_JWT_TOKEN"
```

### User Login (After Approval)

```bash
curl -X POST http://localhost:8081/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "secure_password"
  }'
```

## Testing the Server

### Test MCP Protocol
```bash
# List available tools
curl -X POST http://localhost:8080/mcp \
  -H "Authorization: Bearer JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/list",
    "params": {},
    "id": 1
  }'
```

### Test A2A Protocol
```bash
# Register A2A client
curl -X POST http://localhost:8081/a2a/clients \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Test Client",
    "description": "Testing A2A integration",
    "capabilities": ["fitness-data-analysis"]
  }'
```

## JavaScript SDK Usage

For programmatic access, use the JavaScript SDK:

```javascript
const { PierreClientSDK } = require('./sdk/pierre-client-sdk');

const sdk = new PierreClientSDK('http://localhost:8081');

// Register and wait for admin approval
await sdk.register({
  email: 'user@example.com',
  password: 'secure_password',
  displayName: 'User Name'
});

// After admin approval, login
const session = await sdk.login('user@example.com', 'secure_password');

// Use the API
const activities = await sdk.getStravaActivities();
```

## Deployment Modes

| Mode | Use Case | Features |
|------|----------|----------|
| **Production** | Organizations, teams | User authentication, admin approval, encrypted OAuth storage |
| **Development** | Personal use, testing | Simplified setup, environment variable configuration |

## Features

| Category | Features |
|----------|----------|
| **Architecture** | User authentication • Admin approval system • Two-component design |
| **Security** | Two-tier key management (MEK/DEK) • AES-256-GCM encryption • JWT authentication • Secure OAuth credential storage |
| **Protocols** | MCP Protocol • A2A Protocol • REST APIs |
| **Integrations** | Strava • Fitbit • Claude Desktop • ChatGPT • Custom agents |
| **Intelligence** | Activity analysis • Location detection • Weather integration |

## Core API Endpoints

### User Management (Port 8081)
- `POST /api/auth/register` - User registration (creates "pending" status)
- `POST /api/auth/login` - User authentication (after admin approval)
- `POST /api/admin/users/{id}/approve` - Admin approves pending user

### MCP Protocol (Port 8080)
- `POST /mcp` - All MCP protocol communications (JSON-RPC 2.0)

### A2A Protocol (Port 8081)
- `POST /a2a/clients` - Register A2A client
- `POST /a2a/auth` - A2A authentication
- `POST /a2a/execute` - Execute tools via A2A

## License

This project is dual-licensed under either of:

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
* MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.