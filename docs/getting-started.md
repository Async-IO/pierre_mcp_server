# Getting Started with Pierre MCP Server

Complete guide to get Pierre MCP Server running from zero to production-ready deployment.

## What is Pierre MCP Server?

Pierre is a fitness data API platform that connects AI assistants (like Claude) to fitness providers (like Strava). It supports:
- **MCP protocol** for AI assistants
- **A2A protocol** for autonomous agents and enterprise integrations  
- **REST API** for web applications

## Architecture Overview

Pierre MCP Server runs on two ports:
- **Port 8080**: MCP protocol server (for AI assistants like Claude)
- **Port 8081**: HTTP REST API server (for admin management, user authentication)

## Choose Your Path

### Path 1: Quick Start
Perfect for contributors and developers who want to start coding immediately.

```bash
# Clone and build
git clone https://github.com/Async-IO/pierre_mcp_server.git
cd pierre_mcp_server
cargo build --release

# Automated setup - creates admin, user, tenant, tests MCP
./scripts/fresh-start.sh
source .envrc && cargo run --bin pierre-mcp-server &
./scripts/complete-user-workflow.sh

# Reuse generated tokens
source .workflow_test_env
curl http://localhost:8081/api/health  # Should return {"status":"healthy"}
```

**What the automated script creates:**
- ✅ Admin user (admin@pierre.mcp)
- ✅ Regular user (user@example.com) with approved status
- ✅ Default tenant with OAuth configuration
- ✅ JWT tokens saved to `.workflow_test_env`
- ✅ Validates 25 MCP tools work correctly

Ready to contribute? See [CONTRIBUTING.md](../CONTRIBUTING.md).

### Path 2: Production Setup
For production deployments with security and encryption properly configured.

## Prerequisites

### Required
- **Rust 1.75+**: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`

### Optional (for full features)
- **PostgreSQL**: For multi-user deployments (SQLite auto-created for development)
- **Strava Developer App**: Create at [developers.strava.com](https://developers.strava.com) for real fitness data

## Production Setup Process

### Step 1: Clean Start

Always start with a clean database:

```bash
./scripts/fresh-start.sh
```

This script:
- Stops any running Docker containers
- Removes SQLite databases in `./data/`
- Removes Docker volumes
- Provides a clean slate for setup

### Step 2: Configure Security (Production)

Pierre uses two-tier encryption (MEK/DEK) with automatic mode detection:

**Development Mode (Auto-generates keys with warnings):**
```bash
RUST_LOG=debug cargo run --bin pierre-mcp-server

# System will display:
# WARN Generated MEK (save for production): PIERRE_MASTER_ENCRYPTION_KEY=<base64_key>
```

**Production Mode (Explicit security keys):**
```bash
# Set Master Encryption Key (save the key from development logs)
export PIERRE_MASTER_ENCRYPTION_KEY="<base64_key_from_development_logs>"

# Or generate new production key
export PIERRE_MASTER_ENCRYPTION_KEY="$(openssl rand -base64 32)"

# Start with production security (no warnings)
RUST_LOG=info cargo run --bin pierre-mcp-server
```

### Step 3: Create Admin User

Create the first admin user through the server API:

```bash
curl -X POST http://localhost:8081/admin/setup \
  -H "Content-Type: application/json" \
  -d '{
    "email": "admin@yourcompany.com",
    "password": "SecurePass123!",
    "display_name": "System Administrator"
  }'
```

**Success Output:**
```json
{
  "user_id": "550e8400-e29b-41d4-a716-446655440000",
  "admin_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "message": "Admin user created successfully"
}
```

**Save the admin_token** - you'll need it for user management.

### Step 4: User Management Workflow

**Register Regular User:**
```bash
curl -X POST http://localhost:8081/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "UserPass123!",
    "display_name": "Test User"
  }'
```

**Admin Approval (Required):**
```bash
# List pending users
curl -X GET http://localhost:8081/admin/pending-users \
  -H "Authorization: Bearer <ADMIN_TOKEN>"

# Approve user
curl -X POST http://localhost:8081/admin/approve-user/<USER_ID> \
  -H "Authorization: Bearer <ADMIN_TOKEN>" \
  -H "Content-Type: application/json" \
  -d '{"reason": "Account verified"}'
```

**User Login:**
```bash
curl -X POST http://localhost:8081/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "UserPass123!"
  }'
```

Save the JWT token from login response for MCP integration.

## Claude Desktop Integration

### 1. MCP Client Configuration

**macOS:** `~/Library/Application Support/Claude/claude_desktop_config.json`
**Windows:** `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "pierre-fitness": {
      "command": "/path/to/pierre_mcp_server/scripts/mcp-client.sh",
      "env": {
        "PIERRE_JWT_TOKEN": "USER_JWT_TOKEN_FROM_LOGIN",
        "PIERRE_SERVER_URL": "http://127.0.0.1:8080/mcp"
      }
    }
  }
}
```

### 2. Connect Fitness Provider

Visit: `http://localhost:8081/api/oauth/strava/auth` (requires user login)

### 3. Test in Claude Desktop

Restart Claude Desktop and ask: "What were my recent activities?"

## Environment Variables

### Development
```bash
DATABASE_URL=sqlite:./data/pierre.db
RUST_LOG=debug
```

### Production
```bash
# Core Configuration
MCP_PORT=8080
HTTP_PORT=8081
DATABASE_URL=postgresql://user:pass@localhost:5432/pierre

# Security (Required)
PIERRE_MASTER_ENCRYPTION_KEY=your_32_byte_base64_key

# OAuth Providers
STRAVA_CLIENT_ID=your_strava_client_id
STRAVA_CLIENT_SECRET=your_strava_client_secret

# Logging
RUST_LOG=info
```

## Testing Your Setup

### Health Check
```bash
curl http://localhost:8081/api/health
# Expected: {"status":"healthy"}
```

### Admin Access
```bash
curl -X GET http://localhost:8081/admin/users \
  -H "Authorization: Bearer <ADMIN_TOKEN>"
```

### MCP Protocol Test
```bash
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

### Server Won't Start
- Check ports aren't in use: `lsof -i :8080 -i :8081`
- Verify Rust installation: `rustc --version`
- Reset database: `./scripts/fresh-start.sh`

### User Can't Login
- Verify user status: Admin must approve new users
- Check password requirements: 8+ characters

### MCP Client Connection Fails
- Verify JWT token is valid (expires after 24 hours)
- Check user completed OAuth connection to Strava/Fitbit
- Ensure server running on correct ports

### Database Issues
- Reset database: `./scripts/fresh-start.sh`
- Check SQLite file permissions in `./data/`
- For PostgreSQL: verify connection string and credentials

### Debug Mode
```bash
RUST_LOG=debug cargo run --bin pierre-mcp-server
```

## Next Steps

Once setup is complete:

### For Developers
- **API Integration**: See [API Reference](developer-guide/14-api-reference.md)
- **Architecture Deep Dive**: See [System Architecture](developer-guide/01-architecture.md)
- **Contributing**: See [CONTRIBUTING.md](../CONTRIBUTING.md)

### For Integrators
- **MCP Protocol**: See [MCP Protocol Guide](developer-guide/04-mcp-protocol.md)
- **A2A Protocol**: See [A2A Quick Start](A2A_QUICK_START.md)
- **REST API**: Complete API documentation in [API Reference](developer-guide/14-api-reference.md)

### For Production
- **Deployment**: See [Deployment Guide](DEPLOYMENT_GUIDE.md)
- **Database**: See [Database Guide](DATABASE_GUIDE.md)
- **Security**: See [Security Guide](developer-guide/17-security-guide.md)

## Docker Deployment

### Single Command Deployment
```bash
docker run -d \
  -p 8080:8080 -p 8081:8081 \
  -e STRAVA_CLIENT_ID=your_client_id \
  -e STRAVA_CLIENT_SECRET=your_client_secret \
  -e DATABASE_URL=sqlite:./data/pierre.db \
  -e PIERRE_MASTER_ENCRYPTION_KEY=your_key_here \
  --name pierre-fitness \
  --volume pierre-data:/app/data \
  pierre-mcp-server:latest
```

### Docker Compose
```yaml
version: '3.8'
services:
  pierre-server:
    image: pierre-mcp-server:latest
    ports:
      - "8080:8080"
      - "8081:8081"
    environment:
      - STRAVA_CLIENT_ID=${STRAVA_CLIENT_ID}
      - STRAVA_CLIENT_SECRET=${STRAVA_CLIENT_SECRET}
      - DATABASE_URL=postgresql://user:pass@db:5432/pierre
      - PIERRE_MASTER_ENCRYPTION_KEY=${PIERRE_MASTER_ENCRYPTION_KEY}
    depends_on:
      - db
    restart: unless-stopped

  db:
    image: postgres:16
    environment:
      - POSTGRES_DB=pierre
      - POSTGRES_USER=pierre_user
      - POSTGRES_PASSWORD=${DB_PASSWORD}
    volumes:
      - postgres_data:/var/lib/postgresql/data
    restart: unless-stopped

volumes:
  postgres_data:
```

This consolidated guide eliminates duplication while providing clear paths for different user types and use cases.