# Pierre MCP Server

> ⚠️ **Development Status**: This project is under active development. APIs and features may change.

MCP server for fitness data access with multi-tenant support, admin approval system, and per-user OAuth credential storage.

## Architecture

- **Multi-tenant**: Complete tenant isolation with per-user OAuth credentials
- **Admin Approval**: New users require admin approval before accessing tools
- **Security**: AES-256-GCM encryption for OAuth credentials, JWT authentication
- **Protocols**: HTTP REST API (port 8081) and MCP protocol (port 8080)
- **Database**: SQLite/PostgreSQL with encrypted credential storage

## Documentation

- [Getting Started](docs/GETTING_STARTED.md) - Installation and setup
- [API Reference](docs/API_REFERENCE.md) - Complete API documentation
- [Deployment Guide](docs/DEPLOYMENT_GUIDE.md) - Production deployment
- [Database Guide](docs/DATABASE_GUIDE.md) - Database architecture
- [A2A Protocol](docs/A2A_REFERENCE.md) - Agent-to-Agent communication

## Quick Start

### Prerequisites

- Rust 1.75+
- Node.js 18+ (for JavaScript SDK)
- Strava Developer App (register at [developers.strava.com](https://developers.strava.com))

### Installation

```bash
git clone https://github.com/Async-IO/pierre_mcp_server.git
cd pierre_mcp_server
cargo build --release
```

### Basic Setup

```bash
# Initialize database
./scripts/fresh-start.sh

# Start server
cargo run --bin pierre-mcp-server
```

### User Onboarding Flow

1. **Register User** (creates user with "pending" status):
```bash
curl -X POST http://localhost:8081/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "secure_password",
    "display_name": "User Name"
  }'
```

2. **Admin Approval** (required before user can access tools):
```bash
# Admin approves the user
curl -X POST http://localhost:8081/admin/users/{user_id}/approve \
  -H "Authorization: Bearer ADMIN_JWT_TOKEN"
```

3. **User Login** (after approval):
```bash
curl -X POST http://localhost:8081/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "secure_password"
  }'
```

### OAuth Configuration

After user approval, configure OAuth credentials through the API:

```bash
# Store Strava OAuth credentials for the user
curl -X POST http://localhost:8081/oauth/credentials \
  -H "Authorization: Bearer USER_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "provider": "strava",
    "client_id": "your_strava_client_id",
    "client_secret": "your_strava_client_secret",
    "redirect_uri": "http://localhost:8081/auth/strava/callback"
  }'
```

### JavaScript SDK Usage

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

// Configure OAuth
await sdk.setOAuthCredentials('strava', {
  clientId: 'your_strava_client_id',
  clientSecret: 'your_strava_client_secret'
});

// Use the API
const activities = await sdk.getStravaActivities();
```

## MCP Protocol Usage

### Test MCP Endpoint

```bash
curl -X POST http://localhost:8080/mcp \
  -H "Authorization: API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/list",
    "params": {},
    "id": 1
  }'
```

### Claude Desktop Configuration

Add to `~/.claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "pierre-fitness": {
      "command": "node",
      "args": ["-e", "
        const http = require('http');
        const API_KEY = 'pk_live_YOUR_API_KEY';
        const SERVER_URL = 'http://localhost:8080';
        
        process.stdin.on('data', async (data) => {
          try {
            const request = JSON.parse(data.toString());
            const response = await fetch(SERVER_URL + '/mcp', {
              method: 'POST',
              headers: {
                'Authorization': API_KEY,
                'Content-Type': 'application/json'
              },
              body: JSON.stringify(request)
            });
            const result = await response.json();
            process.stdout.write(JSON.stringify(result) + '\\n');
          } catch (e) {
            process.stdout.write(JSON.stringify({
              jsonrpc: '2.0',
              id: request?.id || null,
              error: { code: -1, message: e.message }
            }) + '\\n');
          }
        });
      "]
    }
  }
}
```

## API Endpoints

### HTTP REST API (Port 8081)

- `POST /auth/register` - User registration
- `POST /auth/login` - User authentication (returns JWT)
- `POST /api/keys` - Create API key (requires JWT)
- `GET /dashboard/overview` - User dashboard (requires JWT)

### MCP Protocol (Port 8080)

- `POST /mcp` - MCP JSON-RPC endpoint (requires API key)

### OAuth Flow

- `GET /auth/strava` - Initiate Strava OAuth
- `GET /auth/strava/callback` - OAuth callback handler

## Database Schema

### User OAuth App Credentials

```sql
CREATE TABLE user_oauth_app_credentials (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    provider TEXT NOT NULL,
    client_id TEXT NOT NULL,
    client_secret TEXT NOT NULL,  -- Encrypted
    redirect_uri TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, provider)
);
```
