# Pierre MCP Server - API Key Provisioning Guide

This document explains how to deploy, configure, and use the Pierre MCP Server for API key provisioning and MCP client integration.

## Architecture Overview

```
┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│                               PIERRE ECOSYSTEM                                                 │
├────────────────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                                │
│  ┌──────────────────┐    Admin JWT     ┌─────────────────┐    API Keys     ┌─────────────────┐ │
│  │ Admin Service    │ ───────────────► │   MCP Server    │ ──────────────► │   Claude        │ │
│  │ (Port 8082)      │                  │  (Port 8081)    │                 │   Desktop       │ │
│  │                  │                  │                 │                 │                 │ │
│  │ • Landing page   │                  │ • Multi-tenant  │                 │ • MCP client    │ │
│  │ • Approval flow  │                  │ • Admin APIs    │                 │ • Tool usage    │ │
│  │ • Email service  │                  │ • MCP protocol  │                 │ • Fitness data  │ │
│  │ • Admin dashboard│                  │ • Rate limiting │                 │ • AI assistance │ │
│  │ • Key requests   │                  │ • Tool execution│                 │                 │ │
│  └──────────────────┘                  └─────────────────┘                 └─────────────────┘ │
│           │                                     │                                     │        │
│           └──────────┬──────────────────────────┴─────────────────────────────────────┘        │
│                      │                                                                         │
│                ┌────────────────────┐                                                          │
│                │    Database        │                                                          │
│                │ (SQLite/Postgres)  │                                                          │
│                │                    │                                                          │
│                │ • Users            │                                                          │
│                │ • API Keys         │                                                          │
│                │ • Admin Tokens     │                                                          │
│                │ • Usage Stats      │                                                          │
│                └────────────────────┘                                                          │
└────────────────────────────────────────────────────────────────────────────────────────────────┘
```

## Deployment Scenarios

### Scenario 1: Single-Tenant (Personal Use)

**Use Case**: Personal Claude integration, no authentication required
**Target Users**: Individual developers, personal fitness tracking

```bash
# 1. Deploy server in single-tenant mode
cargo run --bin pierre-mcp-server -- --single-tenant --port 8080

# 2. Configure Claude Desktop
# ~/.claude/claude_desktop_config.json
{
  "mcpServers": {
    "pierre-fitness": {
      "command": "path/to/pierre-mcp-server",
      "args": ["--single-tenant", "--port", "8080"]
    }
  }
}
```

**No provisioning needed** - Claude can immediately use all fitness tools.

### Scenario 2: Multi-Tenant Deployment

**Use Case**: Multi-user environments, API service provider
**Target Users**: Organizations, developers building fitness applications

```bash
# 1. Deploy MCP server in multi-tenant mode
cargo run --bin pierre-mcp-server -- \
  --database-url "postgresql://user:pass@db:5432/pierre" \
  --port 8081

# 2. Deploy Admin Service (optional but recommended)
cd ../pierre_admin_service
./scripts/setup-admin-token.sh
cargo run --bin pierre_admin_service

# 3. Provision users via admin API
```

## API Key Provisioning Methods

### Method 1: Direct Admin Provisioning

**Best for**: Programmatic provisioning, enterprise accounts

```bash
# Step 1: Generate admin token
cargo run --bin admin-setup -- generate-token \
  --service "enterprise_provisioner" \
  --permissions "provision_keys,revoke_keys,list_keys" \
  --expires-in-days 365

# Step 2: Provision API key for user
curl -X POST http://localhost:8081/admin/provision-api-key \
  -H "Authorization: Bearer <admin_jwt_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "user_email": "user@example.com",
    "tier": "professional",
    "rate_limit_requests": 100000,
    "rate_limit_period": "month",
    "expires_in_days": 365,
    "name": "Production Claude Integration",
    "description": "API key for Claude desktop"
  }'

# Response includes the API key:
{
  "success": true,
  "api_key": "pk_live_abc123def456ghi789jkl012mno345pq",
  "api_key_id": "key_abc123",
  "user_id": "user_def456",
  "tier": "professional",
  "expires_at": "2026-06-20T12:00:00Z"
}
```

### Method 2: Admin Service with Approval Workflow

**Best for**: Self-service with approval, trial accounts

The `pierre_admin_service` provides a complete approval workflow:

```bash
# 1. User visits landing page
https://your-domain.com:8082

# 2. User fills out request form:
# - Email address
# - Company name
# - Use case description
# - Requested tier (starter, professional, enterprise)

# 3. Email verification required

# 4. Admin reviews and approves via dashboard
https://your-domain.com:8082/admin

# 5. API key automatically provisioned and emailed to user
```

## Client Integration Guide

### For Claude Desktop Users

Once a user receives their API key:

```json
// ~/.claude/claude_desktop_config.json
{
  "mcpServers": {
    "pierre-fitness": {
      "command": "mcp-client",
      "args": [
        "--url", "https://your-api-server.com:8081",
        "--auth-key", "pk_live_abc123def456ghi789jkl012mno345pq"
      ]
    }
  }
}
```

### For Other MCP Clients

Direct MCP protocol with authentication:

```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "get_activities",
    "arguments": {
      "provider": "strava",
      "limit": 10,
      "days_back": 30
    }
  },
  "auth": "pk_live_abc123def456ghi789jkl012mno345pq",
  "id": "1"
}
```

### For A2A Protocol (Agent-to-Agent)

Programmatic API access:

```python
import requests

# Authenticate and get activities
response = requests.post("https://your-api-server.com:8081/", json={
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
        "name": "get_activities",
        "arguments": {"provider": "strava", "limit": 5}
    },
    "auth": "pk_live_abc123def456ghi789jkl012mno345pq",
    "id": "1"
})

activities = response.json()["result"]
```

## Production Deployment

### Docker Deployment

```dockerfile
# docker-compose.prod.yml
version: '3.8'
services:
  pierre-mcp:
    build: .
    ports:
      - "8081:8081"
    environment:
      - DATABASE_URL=postgresql://user:pass@db:5432/pierre
      - JWT_SECRET=your-secure-jwt-secret
      - ENCRYPTION_KEY=your-encryption-key
    depends_on:
      - db

  pierre-admin:
    build: ../pierre_admin_service
    ports:
      - "8082:8082"
    environment:
      - MCP_SERVER_URL=http://pierre-mcp:8081
      - MCP_ADMIN_TOKEN=your-admin-jwt-token
      - SENDGRID_API_KEY=your-sendgrid-key
      - DATABASE_URL=postgresql://user:pass@db:5432/pierre

  db:
    image: postgres:15
    environment:
      - POSTGRES_DB=pierre
      - POSTGRES_USER=user
      - POSTGRES_PASSWORD=pass
    volumes:
      - postgres_data:/var/lib/postgresql/data

volumes:
  postgres_data:
```

### Environment Setup

```bash
# Production environment variables
export DATABASE_URL="postgresql://user:pass@db:5432/pierre"
export JWT_SECRET="your-256-bit-secret-here"
export ENCRYPTION_KEY="your-encryption-key-here"
export RUST_LOG="info"

# Admin service
export MCP_SERVER_URL="http://localhost:8081"
export MCP_ADMIN_TOKEN="your-admin-jwt-token"
export SENDGRID_API_KEY="your-sendgrid-api-key"
export BASE_APP_URL="https://api.yourdomain.com"
```

## Security and Authentication

### Admin Token Management

```bash
# Generate new admin token
cargo run --bin admin-setup -- generate-token \
  --service "user_provisioner" \
  --permissions "provision_keys,list_keys" \
  --expires-in-days 90

# Rotate existing token
cargo run --bin admin-setup -- rotate-token admin_token_123

# List active tokens
cargo run --bin admin-setup -- list-tokens
```

### API Key Management

```bash
# List user's API keys
curl -H "Authorization: Bearer <admin_token>" \
     http://localhost:8081/admin/api-keys?user_email=user@company.com

# Revoke API key
curl -X DELETE \
     -H "Authorization: Bearer <admin_token>" \
     http://localhost:8081/admin/api-keys/key_abc123

# Get usage statistics
curl -H "Authorization: Bearer <admin_token>" \
     "http://localhost:8081/admin/api-keys/key_abc123/usage?start_date=2024-01-01&end_date=2024-01-31"
```

## Monitoring and Analytics

### Real-time Monitoring

The MCP server provides comprehensive usage analytics:

- **Request volume** per API key
- **Tool usage** breakdown (get_activities, get_weather, etc.)
- **Error rates** and response times
- **Rate limiting** hits and warnings
- **Geographic usage** patterns

### Admin Dashboard

Access via the admin service at `https://admin.yourdomain.com`:

- View all user API keys
- Monitor usage in real-time
- Set up alerts for high usage
- Generate reports
- Manage user tiers

## Troubleshooting

### Common Issues

**API Key Authentication Fails**
```bash
# Verify API key format and permissions
curl -H "Authorization: Bearer YOUR_API_KEY" \
     http://localhost:8081/admin/api-keys/validate
```

**Rate Limiting Issues**
- Check current usage: GET `/admin/api-keys/{key_id}/usage`
- Adjust rate limits in configuration
- Implement exponential backoff in your client

**Connection Issues**
- Verify server is running: `curl http://localhost:8081/health`
- Check firewall settings for ports 8081-8082
- Validate environment variables are set

### Getting Help

- **Documentation**: Review [API_REFERENCE.md](API_REFERENCE.md)
- **Issues**: Report bugs via GitHub Issues
- **Community**: Join discussions on GitHub

For detailed setup instructions, see:
- [SETUP.md](SETUP.md) - Initial server setup
- [DEPLOYMENT.md](DEPLOYMENT.md) - Production deployment
- [API_REFERENCE.md](API_REFERENCE.md) - Complete API documentation