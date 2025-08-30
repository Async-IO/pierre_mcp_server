# Pierre MCP Server

[![CI](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/ci.yml/badge.svg)](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/ci.yml)
[![Frontend Tests](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/frontend-tests.yml/badge.svg)](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/frontend-tests.yml)

> Development Status: This project is under active development. APIs and features may change.

MCP server implementation for fitness data access with unified provider architecture. Supports fitness data providers (Strava, Fitbit), MCP protocol, A2A protocol, and REST APIs with OAuth credential management and user authentication.

## Architecture Overview

**Server-Focused Architecture**: This system runs as a server that provides multiple interfaces:

1. Pierre MCP Server (`pierre-mcp-server`) - Main server daemon
   - Unified Provider Architecture: Trait-based system for fitness data providers
   - Provider Registry: Factory pattern for provider instantiation
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
| `auth-setup` | Auth configuration CLI | Configure authentication settings |
| `diagnose-weather-api` | Weather API diagnostic | Debug weather integration |
| `serve-docs` | Documentation server | Serve API documentation |

### API Endpoints
| Endpoint | Purpose | Method | Auth Required |
|----------|---------|--------|---------------|
| `/admin/setup` | Create first admin user | POST | None (first-time only) |
| `/admin/setup-status` | Check setup status | GET | None |

### Protocol Support
- **MCP Protocol**: Port 8080 - AI assistants (Claude, ChatGPT), LLM applications  
- **A2A Protocol**: Port 8081 `/a2a/*` - System integrations, autonomous agents
- **REST API**: Port 8081 `/api/*` - Web applications, dashboards

## Documentation

### [Getting Started Guide](docs/getting-started.md)
Complete setup, admin configuration, and first-time usage.

### [Documentation Index](docs/README.md)
Navigate all available documentation by topic and user type.

### Quick References
- [User Registration Guide](claude_docs/HOW_TO_REGISTER_A_USER.md) - Complete curl-based workflow for user setup
- [API Reference](docs/developer-guide/14-api-reference.md) - REST API, MCP protocol, A2A endpoints
- [System Architecture](docs/developer-guide/01-architecture.md) - Design patterns and structure  
- [A2A Quick Start](docs/A2A_QUICK_START.md) - Agent integration setup

### Development
- [Developer Guide](docs/developer-guide/README.md) - Architecture, protocols, testing
- [Contributing Guide](CONTRIBUTING.md) - Code standards and workflow
- [Security Guide](docs/developer-guide/17-security-guide.md) - Two-tier key management, deployment security

### Setup

```bash
# 1. Clone and build (2 minutes)
git clone https://github.com/Async-IO/pierre_mcp_server.git
cd pierre_mcp_server
cargo build --release

# 2. Start server (1 minute)
cargo run --bin pierre-mcp-server
# Server ready on ports 8080 (MCP) and 8081 (HTTP)
# Database auto-created at ./data/users.db
# MEK auto-generated for development

# 3. Create admin user via server API (30 seconds)
curl -X POST http://localhost:8081/admin/setup \
  -H "Content-Type: application/json" \
  -d '{
    "email": "admin@example.com",
    "password": "SecurePass123!",
    "display_name": "System Administrator"
  }'
# Returns: {"user_id": "...", "admin_token": "eyJ0eXAi...", "message": "Admin user created successfully"}

# 4. Verify it works (30 seconds)
curl http://localhost:8081/api/health
# Should return: {"status":"healthy"}

# 5. Run tests
./scripts/lint-and-test.sh
```

### Advanced Setup (Full Features)
<details>
<summary>Click for full setup with Strava integration</summary>

```bash
# Set deployment encryption key
export PIERRE_MASTER_ENCRYPTION_KEY="$(openssl rand -base64 32)"
echo "Save this MEK: $PIERRE_MASTER_ENCRYPTION_KEY"

# Create admin user and get admin token
ADMIN_TOKEN=$(curl -s -X POST http://localhost:8081/admin/setup \
  -H "Content-Type: application/json" \
  -d '{
    "email": "admin@example.com",
    "password": "SecurePass123!",
    "display_name": "System Administrator"
  }' | jq -r '.admin_token')

# Create default tenant using admin token
TENANT_RESPONSE=$(curl -s -X POST http://localhost:8081/api/tenants \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{"name": "My Organization", "slug": "default", "plan": "starter"}')

TENANT_ID=$(echo $TENANT_RESPONSE | jq -r '.tenant_id')

# Configure Strava OAuth (get credentials from developers.strava.com)
curl -X POST http://localhost:8081/api/tenants/$TENANT_ID/oauth \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{
    "provider": "strava",
    "client_id": "YOUR_STRAVA_CLIENT_ID",
    "client_secret": "YOUR_STRAVA_CLIENT_SECRET", 
    "redirect_uri": "http://localhost:8081/api/oauth/callback/strava",
    "scopes": ["read", "activity:read_all"]
  }'
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

### MCP Integration

The server provides an HTTP MCP endpoint at port 8080, path `/mcp`.

#### Direct HTTP MCP Testing
```bash
# Test MCP endpoint directly
curl -X POST http://localhost:8080/mcp \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/list", 
    "params": {},
    "id": 1
  }'
```

#### Claude Desktop Configuration

Add to `~/.claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "pierre-fitness": {
      "command": "/path/to/your/pierre_mcp_server/scripts/mcp-client.sh",
      "env": {
        "PIERRE_JWT_TOKEN": "YOUR_JWT_TOKEN_HERE",
        "PIERRE_SERVER_URL": "http://127.0.0.1:8080/mcp"
      }
    }
  }
}
```

Replace:
- `/path/to/your/pierre_mcp_server/scripts/mcp-client.sh` with the actual path to your installation
- `YOUR_JWT_TOKEN_HERE` with a JWT token from user login or admin token generation

## Troubleshooting

### User Registration and Approval

**Problem**: User cannot access MCP tools after registration.

**Cause**: Users are created in "pending" status and require admin approval.

**Solution**: Use the streamlined approve-user-with-tenant workflow:

```bash
# 1. User registers (creates "pending" status)
USER_RESPONSE=$(curl -s -X POST http://localhost:8081/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email": "user@example.com", "password": "pass123", "display_name": "User"}')

USER_ID=$(echo $USER_RESPONSE | jq -r '.user_id')

# 2. Admin approves user AND creates tenant in single transaction
curl -s -X POST "http://localhost:8081/admin/approve-user/$USER_ID" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_ADMIN_TOKEN" \
  -d '{
    "reason": "User approved",
    "create_default_tenant": true,
    "tenant_name": "User Organization", 
    "tenant_slug": "user-org"
  }'

# 3. User can now login and access MCP tools immediately
```

### OAuth Provider Setup (Optional)

**For Strava Integration**: Configure OAuth at the tenant level after user approval:

```bash
# Configure Strava OAuth for the user's tenant
curl -X POST "http://localhost:8081/api/tenants/$TENANT_ID/oauth" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_ADMIN_TOKEN" \
  -d '{
    "provider": "strava",
    "client_id": "YOUR_STRAVA_CLIENT_ID",
    "client_secret": "YOUR_STRAVA_CLIENT_SECRET",
    "redirect_uri": "http://localhost:8081/api/oauth/callback/strava", 
    "scopes": ["read", "activity:read_all"]
  }'
```

**Required**: Get Strava credentials from [developers.strava.com](https://developers.strava.com)

## Common Workflows

### New Contributor Workflow
```bash
git clone YOUR_FORK && cd pierre_mcp_server
cargo build && cargo run --bin pierre-mcp-server
curl http://localhost:8081/api/health
./scripts/lint-and-test.sh
# Make changes, test, submit PR
```

### User Management Workflow
```bash
# 1. Create admin user (first-time setup)
ADMIN_TOKEN=$(curl -s -X POST http://localhost:8081/admin/setup \
  -H "Content-Type: application/json" \
  -d '{"email":"admin@pierre.mcp", "password":"adminpass123", "display_name":"Admin"}' | jq -r '.admin_token')

# 2. Register user (creates "pending" status)
USER_ID=$(curl -s -X POST http://localhost:8081/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email":"user@example.com", "password":"pass123", "display_name":"User"}' | jq -r '.user_id')

# 3. Admin approval WITH tenant creation (single transaction)
curl -s -X POST "http://localhost:8081/admin/approve-user/$USER_ID" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"reason":"Approved", "create_default_tenant":true, "tenant_name":"User Org", "tenant_slug":"user-org"}'

# 4. User login (now works immediately)
JWT_TOKEN=$(curl -s -X POST http://localhost:8081/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"user@example.com", "password":"pass123"}' | jq -r '.jwt_token')

# 5. Ready for MCP client usage
echo "JWT Token: $JWT_TOKEN"
```

### First Time Setup

1. Start the Pierre MCP Server
2. Create admin user via `/admin/setup` API
3. Register user - creates "pending" status
4. Admin approves user WITH automatic tenant creation - single API call
5. User logs in - gets JWT token immediately  
6. Configure MCP client with JWT token (see Claude Desktop section)
7. Optional: Configure OAuth at tenant level for Strava/Fitbit access
8. All MCP tools available, no legacy OAuth errors

Key improvement: Tenant creation is now automatic during user approval, eliminating the "Legacy OAuth not supported" error completely.

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

### Admin Approval with Automatic Tenant Creation

New users are created with "pending" status and cannot access tools until approved. The approval process automatically creates a tenant:

```bash
# Admin approves user AND creates tenant in single transaction
curl -X POST http://localhost:8081/admin/approve-user/{user_id} \
  -H "Authorization: Bearer ADMIN_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "reason": "User approved for access",
    "create_default_tenant": true,
    "tenant_name": "User Organization", 
    "tenant_slug": "user-org"
  }'
```

Features:
- Single API call handles user approval and tenant setup
- Eliminates "Legacy OAuth not supported" errors 
- User can access MCP tools immediately after approval

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