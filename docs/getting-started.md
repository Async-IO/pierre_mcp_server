# Getting Started with Pierre MCP Server

Complete setup guide to get Pierre MCP Server running from scratch. This guide covers installation, configuration, and first-time usage.

## Architecture Overview

Pierre MCP Server runs on two ports:
- **Port 8080**: MCP protocol server (for AI assistants like Claude)
- **Port 8081**: HTTP REST API server (for admin management, user authentication)

## Prerequisites

### Required
- **Rust 1.75+**: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`

### Optional (for full features)
- **PostgreSQL**: For multi-user deployments (SQLite auto-created for development)
- **Strava Developer App**: Create at [developers.strava.com](https://developers.strava.com) for real fitness data

## Quick Setup

### 1. Install and Build

```bash
# Clone and build
git clone https://github.com/Async-IO/pierre_mcp_server.git
cd pierre_mcp_server
cargo build --release

# Start server (auto-creates SQLite database)
cargo run --bin pierre-mcp-server
```

**Expected output:**
```
INFO Starting Pierre MCP Server...
INFO Database auto-created at ./data/users.db
WARN Generated MEK for development (save for deployment): PIERRE_MASTER_ENCRYPTION_KEY=<base64_key>
INFO MCP server listening on port 8080
INFO HTTP server listening on port 8081
INFO Server ready - admin setup available at POST /admin/setup
```

### 2. Create Admin User via Server API

```bash
# Create first admin user through server API
curl -X POST http://localhost:8081/admin/setup \
  -H "Content-Type: application/json" \
  -d '{
    "email": "admin@example.com",
    "password": "SecurePass123!",
    "display_name": "System Administrator"
  }'
```

**Expected output:**
```json
{
  "user_id": "550e8400-e29b-41d4-a716-446655440000",
  "admin_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "message": "Admin user admin@example.com created successfully with token"
}
```

### 3. Verify Server Health

```bash
curl http://localhost:8081/api/health
# Should return: {"status":"healthy"}
```

## User Management Workflow

### Register a New User

```bash
curl -X POST http://localhost:8081/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "UserPass123!",
    "display_name": "Test User"
  }'
```

### Admin Approval Required

New users are created with "pending" status. Admin must approve them:

```bash
# 1. Use the admin token from the setup step above

# 2. List pending users
curl -X GET http://localhost:8081/admin/pending-users \
  -H "Authorization: Bearer <ADMIN_TOKEN_FROM_SETUP>"

# 3. Approve user
curl -X POST http://localhost:8081/admin/approve-user/<USER_ID> \
  -H "Authorization: Bearer <ADMIN_TOKEN_FROM_SETUP>"
```

### User Login (After Approval)

```bash
curl -X POST http://localhost:8081/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "UserPass123!"
  }'
```

## Claude Desktop Integration

### 1. MCP Client Configuration

**macOS:** `~/Library/Application Support/Claude/claude_desktop_config.json`
**Windows:** `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "pierre-fitness": {
      "command": "node",
      "args": ["/path/to/pierre-mcp-client.js"],
      "env": {
        "PIERRE_API_URL": "http://localhost:8081",
        "PIERRE_AUTH_TOKEN": "USER_JWT_TOKEN_FROM_LOGIN"
      }
    }
  }
}
```

### 2. Connect Strava Account

Visit in browser: `http://localhost:8081/api/oauth/strava/auth` (requires user login)

### 3. Test in Claude Desktop

Ask Claude: "What were my recent activities?"

## Advanced Configuration

### Multi-User Deployment

```bash
# Use explicit encryption key (save the MEK from server logs)
export PIERRE_MASTER_ENCRYPTION_KEY="<base64_key_from_logs>"

# Configure PostgreSQL (optional)
export DATABASE_URL="postgresql://user:pass@localhost/pierre"

# Start server and create admin
cargo run --bin pierre-mcp-server &
ADMIN_TOKEN=$(curl -s -X POST http://localhost:8081/admin/setup \
  -H "Content-Type: application/json" \
  -d '{
    "email": "admin@example.com",
    "password": "SecurePass123!",
    "display_name": "System Administrator"
  }' | jq -r '.admin_token')

# Configure Strava OAuth
curl -X POST http://localhost:8081/api/tenants/<TENANT_UUID>/oauth \
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

### Clean Restart

```bash
# Reset everything for fresh start
./scripts/fresh-start.sh
```

## Testing Your Setup

### 1. Health Check

```bash
curl http://localhost:8081/api/health
# Expected: {"status":"healthy"}
```

### 2. Admin Access

```bash
# List users (use admin token from setup)
curl -X GET http://localhost:8081/admin/users \
  -H "Authorization: Bearer <ADMIN_TOKEN_FROM_SETUP>"
```

### 3. MCP Protocol Test

```bash
# Test MCP tools list
curl -X POST http://localhost:8080/mcp \
  -H "Authorization: Bearer <USER_JWT_TOKEN>" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/list",
    "params": {},
    "id": 1
  }'
```

## Troubleshooting

### Common Issues

**Server won't start:**
- Check port 8080/8081 aren't in use: `lsof -i :8080`
- Verify Rust installation: `rustc --version`

**Database errors:**
- Reset database: `./scripts/fresh-start.sh`
- Check SQLite file permissions in `./data/`

**User can't login:**
- Verify user status: Admin must approve new users
- Check password requirements: 8+ characters

**MCP client connection fails:**
- Verify JWT token is valid (expires after 24 hours)
- Check user has completed OAuth connection to Strava/Fitbit
- Ensure server is running on correct ports

### Debug Mode

```bash
RUST_LOG=debug cargo run --bin pierre-mcp-server
```

### Logs Location

- Server logs: Console output
- Database: `./data/users.db` (SQLite browser to inspect)

## Next Steps

- **API Integration**: See [API Reference](developer-guide/14-api-reference.md)
- **A2A Protocol**: See [A2A Quick Start](A2A_QUICK_START.md)
- **Architecture Deep Dive**: See [System Architecture](developer-guide/01-architecture.md)
- **Security Configuration**: See [Security Guide](developer-guide/17-security-guide.md)

## Development Workflows

### Running Tests

```bash
./scripts/lint-and-test.sh
```

### Code Contribution

```bash
# Format code
cargo fmt

# Check for issues
cargo clippy

# Run full test suite
cargo test
```