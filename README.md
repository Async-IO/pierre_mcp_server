# Pierre MCP Server

[![CI](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/ci.yml/badge.svg)](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/ci.yml)
[![Frontend Tests](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/frontend-tests.yml/badge.svg)](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/frontend-tests.yml)

> ⚠️ **Development Status**: This project is under active development. APIs and features may change.

MCP server implementation for fitness data access from Strava and Fitbit providers. Supports MCP protocol, A2A protocol, and REST APIs with OAuth credential management and user authentication.

## Architecture Overview

**Server-Focused Architecture**: This system runs as a server that provides multiple interfaces:

1. **Pierre MCP Server** (`pierre-mcp-server`) - Main server daemon
   - Handles all fitness data operations
   - Two-tier key management system (MEK/DEK)  
   - Manages OAuth credentials with AES-256-GCM encryption
   - Enforces admin approval for new users
   - Serves MCP protocol directly via HTTP transport
   - Provides REST API and admin endpoints

## Quick Reference

### API Endpoints
| Purpose | Port | Endpoint | Auth Required | Example |
|---------|------|----------|---------------|----------|
| **MCP protocol** | 8080 | All MCP calls | API Key | Claude Desktop integration |
| **Health check** | 8081 | `GET /api/health` | None | `curl localhost:8081/api/health` |
| **User registration** | 8081 | `POST /api/auth/register` | None | User signup |
| **User login** | 8081 | `POST /api/auth/login` | None | Get JWT token |
| **Admin actions** | 8081 | `POST /admin/*` | Admin JWT | Approve users, etc. |
| **A2A protocol** | 8081 | `POST /a2a/*` | Client credentials | Agent-to-agent comms |

### Binaries
| Binary | Purpose | When to Use |
|--------|---------|-------------|
| `pierre-mcp-server` | Main server daemon | Always running (ports 8080 + 8081) |
| `pierre-mcp-client` | MCP client for Claude | Claude Desktop integration |
| `admin-setup` | Admin user management | Initial setup, user approval |

### Protocol Support
- **MCP Protocol**: Port 8080 - AI assistants (Claude, ChatGPT), LLM applications  
- **A2A Protocol**: Port 8081 `/a2a/*` - System integrations, autonomous agents
- **REST API**: Port 8081 `/api/*` - Web applications, dashboards

## Documentation

### **📖 [Getting Started Guide](docs/getting-started.md)**
Complete setup, admin configuration, and first-time usage.

### **📚 [Documentation Index](docs/README.md)**
Navigate all available documentation by topic and user type.

### **⚡ Quick References**
- [API Reference](docs/developer-guide/14-api-reference.md) - REST API, MCP protocol, A2A endpoints
- [System Architecture](docs/developer-guide/01-architecture.md) - Design patterns and structure  
- [A2A Quick Start](docs/A2A_QUICK_START.md) - 5-minute agent integration setup

### **🛠️ Development**
- [Developer Guide](docs/developer-guide/README.md) - Architecture, protocols, testing
- [Contributing Guide](CONTRIBUTING.md) - Code standards and workflow
- [Security Guide](docs/developer-guide/17-security-guide.md) - Two-tier key management, deployment security

## Quick Setup

### Prerequisites (Choose One)

**👥 New Contributor** (Recommended)
- Only **Rust 1.75+** required: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- SQLite auto-created, no external dependencies
- Perfect for: Contributing code, testing, learning

**🚀 Full Development**  
- **Rust 1.75+** + **Strava app** (create at [developers.strava.com](https://developers.strava.com))
- Optional: PostgreSQL, Redis
- Perfect for: Multi-user deployment, full feature testing

### Setup

```bash
# 1. Clone and build (2 minutes)
git clone https://github.com/Async-IO/pierre_mcp_server.git
cd pierre_mcp_server
cargo build --release

# 2. Start server (1 minute)
cargo run --bin pierre-mcp-server
# ✅ Server ready on ports 8080 (MCP) and 8081 (HTTP)
# ✅ Database auto-created at ./data/users.db
# ✅ MEK auto-generated for development

# 3. Verify it works (30 seconds)
curl http://localhost:8081/api/health
# Should return: {"status":"healthy"}

# 4. Ready! (30 seconds)
./scripts/lint-and-test.sh  # Run full test suite
# Ready to contribute
```

### Advanced Setup (Full Features)
<details>
<summary>Click for full setup with Strava integration</summary>

```bash
# Set deployment encryption key
export PIERRE_MASTER_ENCRYPTION_KEY="$(openssl rand -base64 32)"
echo "Save this MEK: $PIERRE_MASTER_ENCRYPTION_KEY"

# Create default tenant
curl -X POST http://localhost:8081/api/tenants \
  -H "Content-Type: application/json" \
  -d '{"name": "My Organization", "slug": "default", "plan": "starter"}'

# Configure Strava OAuth (get credentials from developers.strava.com)
curl -X POST http://localhost:8081/api/tenants/{TENANT_UUID}/oauth \
  -H "Content-Type: application/json" \
  -d '{"provider": "strava", "client_id": "YOUR_CLIENT_ID", "client_secret": "YOUR_CLIENT_SECRET", "redirect_uri": "http://localhost:8081/api/oauth/callback/strava", "scopes": ["read", "activity:read_all"]}'
```
</details>

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

## Common Workflows

### New Contributor Workflow
```bash
git clone YOUR_FORK && cd pierre_mcp_server
cargo build && cargo run --bin pierre-mcp-server
curl http://localhost:8081/api/health  # ✅ Should work
./scripts/lint-and-test.sh              # ✅ Should pass
# Make changes, test, submit PR
```

### User Management Workflow  
```bash
# 1. Register user
curl -X POST http://localhost:8081/api/auth/register \
  -d '{"email":"user@example.com", "password":"pass123", "display_name":"User"}'

# 2. Admin approval needed (admin user created via admin-setup binary)
# 3. User login
curl -X POST http://localhost:8081/api/auth/login \
  -d '{"email":"user@example.com", "password":"pass123"}'
# Returns JWT for MCP client
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
curl -X POST http://localhost:8081/admin/approve-user/{user_id} \
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
| **Multi-user** | Teams, organizations | User authentication, admin approval, encrypted OAuth storage |
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
- `POST /admin/approve-user/{id}` - Admin approves pending user

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