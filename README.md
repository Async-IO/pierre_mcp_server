# Pierre MCP Server

MCP server for fitness data access. Provides HTTP and MCP protocol endpoints for Strava and Fitbit integration with encrypted OAuth credential storage.

## Architecture

- **Server**: HTTP API (port 8081) and MCP protocol (port 8080)
- **Database**: SQLite with AES-256-GCM encrypted OAuth credentials
- **Authentication**: JWT tokens and API keys
- **OAuth**: Per-user credential storage for cloud deployment

## Setup

### Prerequisites

- Rust 1.75+
- Strava Developer App (register at developers.strava.com)

### Installation

```bash
git clone https://github.com/Async-IO/pierre_mcp_server.git
cd pierre_mcp_server
cargo build --release
```

### Database Setup

```bash
# Clean database
./scripts/fresh-start.sh

# Start server
cargo run --bin pierre-mcp-server
```

### User Registration

```bash
curl -X POST http://localhost:8081/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "password123",
    "display_name": "User Name"
  }'
```

### Authentication

```bash
# Login to get JWT token
curl -X POST http://localhost:8081/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "password123"
  }'

# Create API key for MCP access
curl -X POST http://localhost:8081/api/keys \
  -H "Authorization: Bearer JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "MCP Client",
    "tier": "professional",
    "description": "API key for MCP protocol",
    "rate_limit_requests": 10000,
    "expires_in_days": 90
  }'
```

### OAuth Configuration

Store your Strava app credentials directly in database:

```python
import sqlite3
conn = sqlite3.connect("data/users.db")
conn.execute("""
    INSERT INTO user_oauth_app_credentials 
    (id, user_id, provider, client_id, client_secret, redirect_uri, created_at, updated_at)
    VALUES (?, ?, 'strava', ?, ?, ?, datetime('now'), datetime('now'))
""", [
    "unique_id", "your_user_id", "your_client_id", 
    "your_client_secret", "http://localhost:8081/auth/strava/callback"
])
conn.commit()
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
