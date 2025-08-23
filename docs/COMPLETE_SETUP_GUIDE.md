# Pierre MCP Server - Complete Setup Guide

This guide provides step-by-step instructions for setting up Pierre MCP Server from scratch, including common pitfalls and solutions.

## Overview

Pierre MCP Server runs on two ports:
- **Port 8080**: MCP protocol server (for Claude Desktop integration)  
- **Port 8081**: HTTP REST API server (for user registration, admin management, etc.)

## Prerequisites

- Rust toolchain installed
- SQLite (default) or PostgreSQL
- Environment variables properly configured

## Complete Setup Process

### Step 1: Clean Start

Always start with a clean database when setting up from zero:

```bash
./scripts/fresh-start.sh
```

This script:
- Stops any running Docker containers
- Removes SQLite databases in `./data/`
- Removes Docker volumes
- Provides a clean slate for setup

### Step 2: Configure Two-Tier Key Management

Pierre uses an automatic two-tier key management system with mode detection:

**For Development (Automatic MEK generation):**
```bash
# No environment variable needed - system auto-generates MEK with warnings
RUST_LOG=debug cargo run --bin pierre-mcp-server

# System will display warnings:
# WARN PIERRE_MASTER_ENCRYPTION_KEY not found in environment
# WARN Generating temporary MEK for development - NOT SECURE FOR PRODUCTION  
# WARN Generated MEK (save for production): PIERRE_MASTER_ENCRYPTION_KEY=<base64_key>
```

**For Production (Explicit MEK):**
```bash
# Set Master Encryption Key from development logs or generate new one
export PIERRE_MASTER_ENCRYPTION_KEY="<base64_key_from_development_logs>"
# OR generate new one:
# export PIERRE_MASTER_ENCRYPTION_KEY="$(openssl rand -base64 32)"

# Start server with production MEK (no warnings)
RUST_LOG=debug cargo run --bin pierre-mcp-server
```

**Key Management Process:**
1. **Bootstrap**: MEK loaded from environment (or auto-generated), temporary DEK created
2. **Database Init**: Database initialized with temporary DEK  
3. **Complete Init**: Existing encrypted DEK loaded from database or current DEK stored encrypted

**Important Notes:**
- **MEK (Master Encryption Key)**: From `PIERRE_MASTER_ENCRYPTION_KEY` environment variable
- **DEK (Database Encryption Key)**: Auto-generated, stored encrypted with MEK in database
- Development to production: Copy MEK from development logs - no data migration needed
- Server runs on ports 8080 (MCP) and 8081 (HTTP)

### Step 3: Start the Server

Start the Pierre MCP server with the configured key management:

```bash
RUST_LOG=debug cargo run --bin pierre-mcp-server
```

The server will display two-tier key management initialization:

**Development Mode (MEK auto-generated):**
```
WARN PIERRE_MASTER_ENCRYPTION_KEY not found in environment
WARN Generating temporary MEK for development - NOT SECURE FOR PRODUCTION
WARN Generated MEK (save for production): PIERRE_MASTER_ENCRYPTION_KEY=base64_key_here
Two-tier key management system bootstrapped
Database initialized successfully: SQLite (./data/users.db)
Two-tier key management system fully initialized
```

**Production Mode (MEK from environment):**
```
Loading Master Encryption Key from environment variable
Two-tier key management system bootstrapped  
Database initialized successfully: SQLite (./data/users.db)
Two-tier key management system fully initialized
```

### Step 4: Create Admin User

Create an admin user for the web interface:

```bash
RUST_LOG=debug cargo run --bin admin-setup -- --verbose create-admin-user --email admin@yourcompany.com --password yourpassword123
```

**Success Output:**
```
Admin User Created Successfully!
================================================
USER DETAILS:
   Email: admin@yourcompany.com
   Name: Pierre Admin
   Tier: Enterprise (Full access)
   Status: Active

LOGIN CREDENTIALS:
================================================
   Email: admin@yourcompany.com
   Password: yourpassword123
```

### Step 5: Generate Super Admin Token

Generate a super admin token for API access:

```bash
RUST_LOG=debug cargo run --bin admin-setup -- --verbose generate-token --service your_service_name --super-admin --expires-days 365
```

**Success Output:**
```
Admin Token Generated Successfully!
==============================================================
YOUR JWT TOKEN (SAVE THIS NOW):
==============================================================
eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...
==============================================================

CRITICAL SECURITY NOTES:
- This token is shown ONLY ONCE - save it now!
- Store it securely in your environment:
  export PIERRE_MCP_ADMIN_TOKEN="eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9..."
```

**IMPORTANT:** Copy and save this JWT token immediately - it's only shown once!

### Step 6: Register Test User

Register a test user account via the HTTP API (port 8081):

```bash
curl -X POST http://localhost:8081/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "testuser@example.com",
    "password": "testpass123",
    "display_name": "Test User"
  }'
```

**Success Response:**
```json
{
  "user_id": "e13aeeb8-b7cb-4498-aeb7-b211284c1d37",
  "message": "User registered successfully. Your account is pending admin approval."
}
```

### Step 7: Approve User (Optional)

Use the admin token to approve the registered user:

```bash
curl -X PUT http://localhost:8081/admin/users/e13aeeb8-b7cb-4498-aeb7-b211284c1d37/approve \
  -H "Authorization: Bearer YOUR_JWT_TOKEN_HERE" \
  -H "Content-Type: application/json"
```

### Step 8: Configure Claude Desktop

Create or update your Claude Desktop configuration file:

**macOS:** `~/Library/Application Support/Claude/claude_desktop_config.json`
**Windows:** `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "pierre-fitness": {
      "command": "node",
      "args": ["/path/to/pierre_mcp_server/clients/javascript/pierre-mcp-bridge.js"],
      "env": {
        "PIERRE_MCP_SERVER_URL": "http://localhost:8080",
        "PIERRE_USER_EMAIL": "testuser@example.com",
        "PIERRE_USER_PASSWORD": "testpass123"
      }
    }
  }
}
```

### Step 9: Test MCP Integration

1. Restart Claude Desktop
2. Start a new conversation
3. Try fitness-related commands:
   - "Show me my connection status for fitness providers"
   - "Help me connect to Strava"
   - "What fitness tools are available?"

## Common Issues & Solutions

### Issue 1: Connection Refused on Port 8080

**Problem:** Claude Desktop can't connect to MCP server
**Solution:** Ensure server is running and listening on port 8080

```bash
# Check if server is running
lsof -i :8080
# Should show pierre-mcp-server process
```

### Issue 2: 404 Not Found on Registration

**Problem:** Getting 404 when trying to register users
**Solution:** Use port 8081 for HTTP API, not 8080

```bash
# Correct (HTTP API)
curl -X POST http://localhost:8081/auth/register ...

# Incorrect (MCP protocol port)
curl -X POST http://localhost:8080/auth/register ...
```

### Issue 3: Two-Tier Key Management Issues

**Problem:** Admin tokens not working or encryption key mismatches
**Solution:** Ensure consistent Master Encryption Key (MEK)

1. **Development**: Use the same generated MEK consistently:
   ```bash
   # Save the MEK from server startup logs
   export PIERRE_MASTER_ENCRYPTION_KEY="generated_key_from_logs"
   ```

2. **Production**: Set MEK explicitly:
   ```bash
   export PIERRE_MASTER_ENCRYPTION_KEY="$(openssl rand -base64 32)"
   ```

3. **Troubleshooting**: Check key management initialization in logs:
   ```
   Two-tier key management system bootstrapped
   Two-tier key management system fully initialized
   ```

### Issue 4: User Approval Fails

**Problem:** Cannot approve users with admin token
**Solution:** Verify admin token has correct permissions

```bash
# List admin tokens to verify
RUST_LOG=debug cargo run --bin admin-setup -- --verbose list-tokens --detailed
```

### Issue 5: Database Inconsistencies

**Problem:** Database corruption or inconsistent state
**Solution:** Clean restart

```bash
./scripts/fresh-start.sh
# Then repeat setup process
```

## Production Deployment Notes

### Environment Variables

Set these for production:

```bash
export PIERRE_MASTER_ENCRYPTION_KEY="your_base64_key_here"
export DATABASE_URL="your_production_database_url"
export RUST_LOG="info"  # Don't use debug in production
```

### Security Considerations

1. **Master Encryption Key (MEK) Security**:
   - Store MEK in secure key management system (HSM, Kubernetes secrets)
   - Never commit MEK to version control
   - Backup MEK separately from database backups
   - Rotate MEK annually or when compromised

2. **Admin Token Security**:
   - Never commit admin tokens to version control
   - Rotate admin tokens regularly in production
   - Monitor admin token usage via audit logs

3. **General Security**:
   - Use HTTPS in production deployments
   - Set strong passwords for admin users
   - Enable audit logging for all administrative actions

### Database Management

- Use PostgreSQL for production (better performance, concurrent access)
- Set up proper backups
- Monitor database growth and performance
- Use proper connection pooling

## Architecture Overview

```
┌─────────────────┐    ┌─────────────────┐
│   Claude Desktop │────│  MCP Port 8080  │
└─────────────────┘    │                 │
                       │  Pierre Server  │
┌─────────────────┐    │                 │
│   Web Interface │────│ HTTP Port 8081  │
└─────────────────┘    └─────────────────┘
                              │
                       ┌─────────────────┐
                       │   Database      │
                       │   (SQLite/PG)   │
                       └─────────────────┘
```

## Testing the Setup

### 1. Basic Server Health

```bash
curl http://localhost:8081/health
# Should return: {"status": "healthy"}
```

### 2. MCP Protocol Test

```bash
curl -X POST http://localhost:8080 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}'
```

### 3. Admin API Test

```bash
curl -X GET http://localhost:8081/admin/tokens \
  -H "Authorization: Bearer YOUR_JWT_TOKEN"
```

## Quick Reference

| Component | Port | Purpose |
|-----------|------|---------|
| MCP Server | 8080 | Claude Desktop integration |
| HTTP API | 8081 | User registration, admin interface |
| Database | - | SQLite (dev) / PostgreSQL (prod) |

| Command | Purpose |
|---------|---------|
| `./scripts/fresh-start.sh` | Clean database restart |
| `cargo run --bin pierre-mcp-server` | Start main server |
| `cargo run --bin admin-setup` | Admin management |
| `./scripts/lint-and-test.sh` | Run tests and linting |

## Support

For issues:
1. Check server logs for detailed error messages
2. Verify all ports are accessible
3. Ensure database connectivity
4. Check admin token permissions
5. Review this guide for common solutions

---

*This guide is maintained to ensure consistent setup experiences. Update it when processes change.*