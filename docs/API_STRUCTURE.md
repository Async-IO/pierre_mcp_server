# Pierre MCP Server API Structure

This document clarifies the complete API structure for Pierre MCP Server.

## Port Structure

| Port | Protocol | Purpose |
|------|----------|---------|
| **8080** | MCP (JSON-RPC over WebSocket) | AI assistant integration (Claude Desktop, etc.) |
| **8081** | HTTP REST | Web APIs, admin management, A2A protocol |

## HTTP API Structure (Port 8081)

### Regular API Routes
**Pattern**: `/api/*`

| Purpose | Endpoint Pattern | Example |
|---------|------------------|---------|
| User authentication | `/api/auth/*` | `/api/auth/login`, `/api/auth/register`, `/api/auth/refresh` |
| API key management | `/api/keys/*` | `/api/keys` (POST/GET), `/api/keys/{id}` (DELETE), `/api/keys/{id}/usage` |
| Tenant management | `/api/tenants/*` | `/api/tenants` (POST/GET), `/api/tenants/{id}/oauth` |
| Configuration | `/api/configuration/*` | `/api/configuration/catalog`, `/api/configuration/profiles`, `/api/configuration/user`, `/api/configuration/zones` |
| Dashboard analytics | `/api/dashboard/*` | `/api/dashboard/overview`, `/api/dashboard/analytics`, `/api/dashboard/rate-limits`, `/api/dashboard/request-logs`, `/api/dashboard/request-stats`, `/api/dashboard/tool-usage` |
| OAuth flows | `/api/oauth/*` | `/api/oauth/auth/{provider}/{user_id}`, `/api/oauth/callback/{provider}` |
| Health checks | `/health` | `/health` (Note: no `/api/` prefix) |

### MCP HTTP Routes
**Pattern**: `/mcp` (Note: NO `/api/` prefix)

| Purpose | Endpoint | Description |
|---------|----------|-------------|
| MCP HTTP interface | `POST /mcp` | HTTP-based MCP protocol endpoint |

### Admin Routes
**Pattern**: `/admin/*` (Note: NO `/api/` prefix)

| Purpose | Endpoint | Description |
|---------|----------|-------------|
| Setup status | `GET /admin/setup/status` | Check if admin setup needed |
| Initial setup | `POST /admin/setup` | Create first admin user |
| List users | `GET /admin/users` | List all users |
| Pending users | `GET /admin/pending-users` | List users awaiting approval |
| Approve user | `POST /admin/approve-user/{id}` | Approve pending user |
| Suspend user | `POST /admin/suspend-user/{id}` | Suspend user account |
| Manage tokens | `GET /admin/tokens` | List admin tokens |
| Rate limiting | `GET /admin/rate-limits/*` | Manage user rate limits |

### A2A Protocol Routes
**Pattern**: `/a2a/*` (Note: NO `/api/` prefix)

| Purpose | Endpoint | Description |
|---------|----------|-------------|
| Agent card | `GET /a2a/agent-card` | Agent discovery card |
| Client registration | `POST /a2a/clients` | Register A2A client |
| Authentication | `POST /a2a/auth` | Get session token |
| Tool execution | `POST /a2a/execute` | Execute A2A tools |
| Dashboard overview | `GET /a2a/dashboard/overview` | A2A dashboard |
| Client management | `GET /a2a/clients` | List A2A clients |
| Client usage | `GET /a2a/clients/{id}/usage` | Client usage stats |
| Client rate limit | `GET /a2a/clients/{id}/rate-limit` | Client rate limit info |

## Why This Structure?

### `/api/*` for Complete REST API
- **Unified REST design** for all web application functionality
- **Version-ready** (can add `/api/v2/` later)  
- **Standard practice** for modern APIs
- **Complete feature set**: auth, resources, analytics, OAuth flows
- **Single authentication pattern** with consistent JWT handling

### `/mcp` for MCP HTTP Protocol
- **Single endpoint** for HTTP-based MCP communication
- **Alternative to WebSocket** for certain integrations
- **Protocol compliance** with MCP specification

### `/admin/*` for System Administration
- **System administration** separate from regular API
- **Legacy compatibility** - existing admin tools expect this
- **Administrative operations** with elevated permissions

### `/a2a/*` for Agent-to-Agent Protocol
- **Protocol identifier** in the path
- **Distinct from regular REST API** - this is RPC-style
- **Agent discovery** standards expect this pattern

## Authentication Summary

| Route Pattern | Auth Method | Token Type |
|---------------|-------------|------------|
| `/api/*` (all routes) | `Authorization: Bearer <JWT>` | User JWT token |
| `/mcp` | `Authorization: Bearer <JWT>` | User JWT token |
| `/admin/*` | `Authorization: Bearer <JWT>` | Admin JWT token |  
| `/a2a/*` | `Authorization: Bearer <token>` | A2A session token |
| Port 8080 (MCP) | `Authorization: Bearer <JWT>` | User JWT token |

## Common Patterns

### User Registration & Approval Flow
```bash
# 1. User registers
POST /api/auth/register

# 2. Admin approves (note: /admin not /api/admin)
POST /admin/approve-user/{user_id}

# 3. User logs in
POST /api/auth/login
```

### A2A Integration Flow
```bash
# 1. Register A2A client
POST /a2a/clients

# 2. Get session token  
POST /a2a/auth

# 3. Execute tools
POST /a2a/execute
```

This structure achieves **complete REST API consistency** with all regular functionality under `/api/*`, while maintaining **specialized protocol endpoints** (`/mcp`, `/a2a/*`) and **administrative functions** (`/admin/*`) with **clear functional separation**.