# Getting Started Guide

This guide covers setup, configuration, and development workflows for Pierre MCP Server. Follow these steps to establish a working development environment.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Quick Setup](#quick-setup)
3. [Development Environment](#development-environment)
4. [First Run](#first-run)
5. [Testing Your Setup](#testing-your-setup)
6. [Common Development Workflows](#common-development-workflows)
7. [Integration Examples](#integration-examples)
8. [Troubleshooting](#troubleshooting)
9. [Next Steps](#next-steps)

## Prerequisites

### Required Software

1. Rust (1.70+ recommended)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source ~/.cargo/env
   ```

2. Database (choose one):
   - SQLite (default, for development)
   - PostgreSQL (recommended for production)
   ```bash
   # PostgreSQL installation
   brew install postgresql  # macOS
   sudo apt-get install postgresql postgresql-contrib  # Ubuntu
   ```

3. Redis (optional, for production caching)
   ```bash
   # Optional: only needed for production deployments
   brew install redis  # macOS
   sudo apt-get install redis-server  # Ubuntu
   ```

4. Git
   ```bash
   git --version  # Should be 2.0+
   ```

### Optional Tools

- Docker and Docker Compose (for containerized development)
- Claude Desktop or other MCP client (for testing)
- Postman or curl (for API testing)

### External Services

You'll need accounts and API credentials for:
- Strava API (for fitness data integration)
- Fitbit API (optional, for additional data sources)

## Setup

### 1. Clone the Repository

```bash
git clone https://github.com/Async-IO/pierre_mcp_server.git
cd pierre_mcp_server
```

### 2. Environment Configuration

Create your environment file:

```bash
cp .env.example .env
```

Edit `.env` with your settings:

```bash
# Required Configuration
DATABASE_URL="sqlite:./data/pierre.db"  # For development
PIERRE_MASTER_ENCRYPTION_KEY="your_32_byte_base64_key"  # Generate with: openssl rand -base64 32

# Optional Configuration
HTTP_PORT=8081  # Single port for all protocols
HOST="127.0.0.1"
RUST_LOG=info
LOG_FORMAT=json

# Production Database
# DATABASE_URL="postgresql://user:pass@localhost/pierre"

# OAuth Credentials (get these from provider developer consoles)
STRAVA_CLIENT_ID="your_strava_client_id"
STRAVA_CLIENT_SECRET="your_strava_client_secret"
STRAVA_REDIRECT_URI="http://localhost:8081/api/oauth/callback/strava"

# JWT Configuration
JWT_EXPIRY_HOURS=24
JWT_SECRET_PATH=./data/jwt.secret

# OpenWeather API (for activity intelligence)
OPENWEATHER_API_KEY="your_openweather_api_key"

# Logging
RUST_LOG="info,pierre_mcp_server=debug"
```

### 3. Database Setup

For SQLite (default):
```bash
# Database will be created automatically on first run
cargo run --bin pierre-mcp-server
```

For PostgreSQL:
```bash
# Create database
createdb pierre
# Run migrations (handled automatically by the server)
```

### 4. Build and Run

```bash
# Development build
cargo build

# Run with development settings
cargo run --bin pierre-mcp-server
```

You should see:
```
2024-01-15T10:30:00.123Z  INFO pierre_mcp_server: Server starting on 127.0.0.1:3000
2024-01-15T10:30:00.124Z  INFO pierre_mcp_server: Database migrations completed
2024-01-15T10:30:00.125Z  INFO pierre_mcp_server: MCP protocol handler initialized
2024-01-15T10:30:00.126Z  INFO pierre_mcp_server: A2A protocol handler initialized
2024-01-15T10:30:00.127Z  INFO pierre_mcp_server: Server ready for connections
```

## Development Environment

### Project Structure Overview

```
pierre_mcp_server/
├── src/
│   ├── main.rs                 # Server entry point
│   ├── lib.rs                  # Core library
│   ├── api_key_routes.rs       # API key management
│   ├── auth.rs                 # Authentication logic
│   ├── database_plugins/       # Database abstraction
│   ├── protocols/              # MCP and A2A protocols
│   └── ...
├── tests/                      # Integration tests
├── docs/                       # Documentation
├── Cargo.toml                  # Dependencies and metadata
├── .env.example                # Environment template
└── README.md                   # Project overview
```

### Development Commands

```bash
# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run

# Check code without running
cargo check

# Format code
cargo fmt

# Lint code
cargo clippy

# Run complete validation
./scripts/lint-and-test.sh
```

### IDE Setup

VS Code Extensions:
- rust-analyzer
- Even Better TOML
- REST Client

Settings (`.vscode/settings.json`):
```json
{
    "rust-analyzer.cargo.features": "all",
    "rust-analyzer.checkOnSave.command": "clippy",
    "editor.formatOnSave": true
}
```

## First Run

### 1. Start the Server

```bash
cargo run --bin pierre-mcp-server
```

The server will start on port 8081 by default and display all available endpoints.

### 2. Initialize Admin User

The server is ready for admin setup via REST API. Create the initial admin user:

```bash
curl -X POST http://localhost:8081/admin/setup \
  -H "Content-Type: application/json" \
  -d '{
    "email": "admin@example.com",
    "password": "SecurePass123!",
    "display_name": "System Administrator"
  }'
```

This will return an admin token for administrative operations.

### 3. Register Your First User

```bash
curl -X POST http://localhost:8081/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "userpass123",
    "display_name": "Regular User"
  }'
```

**Response:**
```json
{
  "user_id": "550e8400-e29b-41d4-a716-446655440000",
  "message": "User registered successfully. Your account is pending admin approval."
}
```

### 4. Approve User (Admin Action)

New users require admin approval:

```bash
curl -X POST "http://localhost:8081/admin/approve-user/550e8400-e29b-41d4-a716-446655440000" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{
    "reason": "User registration approved",
    "create_default_tenant": true,
    "tenant_name": "User Organization",
    "tenant_slug": "user-org"
  }'
```

### 5. Login and Get JWT Token

After approval, users can login:

```bash
JWT_TOKEN=$(curl -s -X POST http://localhost:8081/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "userpass123"
  }' | jq -r '.jwt_token')

echo "JWT Token: $JWT_TOKEN"
```

Save the JWT token for API calls and MCP integration.

## Testing Your Setup

### 1. Test Health Check

```bash
curl -X GET http://localhost:8081/health
```

### 2. Test MCP Tools Listing

```bash
curl -X POST http://localhost:8081/mcp \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/list"
  }'
```

### 3. Test OAuth 2.0 Server

```bash
# Get OAuth 2.0 server metadata
curl -X GET http://localhost:8081/.well-known/oauth-authorization-server
```

### 4. Test MCP WebSocket Connection

Create a test file `test_mcp.js`:

```javascript
const WebSocket = require('ws');

const ws = new WebSocket('ws://localhost:8081/mcp/ws', {
  headers: {
    'Authorization': 'Bearer YOUR_JWT_TOKEN'
  }
});

ws.on('open', function open() {
  console.log('Connected to MCP server');
  
  // Send initialize request
  ws.send(JSON.stringify({
    jsonrpc: "2.0",
    method: "initialize",
    params: {
      protocolVersion: "2025-06-18",
      capabilities: {
        roots: { listChanged: true },
        sampling: {}
      },
      clientInfo: {
        name: "Test Client",
        version: "1.0.0"
      }
    },
    id: 1
  }));
});

ws.on('message', function message(data) {
  console.log('Received:', JSON.parse(data));
});
```

Run with: `node test_mcp.js`

### 3. Test A2A Protocol

```bash
# Register A2A client
curl -X POST http://localhost:8081/a2a/register \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Test Agent",
    "description": "Test A2A client",
    "capabilities": ["webhook"],
    "contact_email": "developer@example.com"
  }'

# Authenticate A2A client
curl -X POST http://localhost:8081/a2a/auth \
  -H "Content-Type: application/json" \
  -d '{
    "client_id": "YOUR_CLIENT_ID",
    "client_secret": "YOUR_CLIENT_SECRET",
    "scopes": ["read"]
  }'
```

## Common Development Workflows

### Adding a New MCP Tool

1. Define the tool in `src/protocols/mcp/tools/`:

```rust
// src/protocols/mcp/tools/my_new_tool.rs
use anyhow::Result;
use serde_json::Value;

pub async fn execute_my_new_tool(
    user_id: &str,
    params: Value,
) -> Result<Value> {
    // Tool implementation
    Ok(serde_json::json!({
        "result": "success",
        "data": params
    }))
}
```

2. Register the tool:

```rust
// In src/protocols/mcp/tools/mod.rs
pub mod my_new_tool;

// Add to tool registry
tools.insert("my_new_tool", Box::new(my_new_tool::execute_my_new_tool));
```

3. Add tests:

```rust
// tests/mcp_tools_test.rs
#[tokio::test]
async fn test_my_new_tool() {
    // Test implementation
}
```

4. Update documentation:

```markdown
<!-- In docs/developer-guide/04-mcp-protocol.md -->
### my_new_tool

Description of the tool...
```

### Adding a New REST Endpoint

1. Add route handler:

```rust
// In appropriate routes file
pub async fn my_new_endpoint(
    &self,
    auth_header: Option<&str>,
    request: MyRequest,
) -> Result<MyResponse> {
    // Implementation
}
```

2. Add to router configuration:

```rust
// In server setup
.route("/api/my-endpoint", post(my_new_endpoint))
```

3. Add request/response types:

```rust
#[derive(Deserialize)]
pub struct MyRequest {
    pub field: String,
}

#[derive(Serialize)]
pub struct MyResponse {
    pub result: String,
}
```

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test module
cargo test mcp_protocol

# Run with debug output
cargo test -- --nocapture

# Run integration tests only
cargo test --test '*'

# Run with coverage (requires cargo-tarpaulin)
cargo tarpaulin --out Html
```

### Database Migrations

When you add new database tables or columns:

1. Create migration script:

```sql
-- migrations/004_add_my_table.sql
CREATE TABLE my_table (
    id UUID PRIMARY KEY,
    name VARCHAR NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

2. Update database schema in code:

```rust
// src/database_plugins/models.rs
#[derive(sqlx::FromRow)]
pub struct MyTable {
    pub id: String,
    pub name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
```

3. Test migration:

```bash
# Migrations run automatically on server start
cargo run --bin pierre-mcp-server
```

## Integration Examples

### MCP Client Integration

#### Claude Desktop Configuration

**Configuration Path:**
- **macOS:** `~/Library/Application Support/Claude/claude_desktop_config.json`
- **Windows:** `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "pierre-fitness": {
      "url": "http://127.0.0.1:8081/mcp",
      "headers": {
        "Authorization": "Bearer USER_JWT_TOKEN_FROM_LOGIN"
      }
    }
  }
}
```

#### Other MCP Client Configuration

**Configuration Paths (varies by client):**
Different MCP clients use different configuration file locations. Consult your MCP client's documentation for the specific path.

```json
{
  "mcpServers": {
    "pierre-fitness": {
      "url": "http://127.0.0.1:8081/mcp",
      "headers": {
        "Authorization": "Bearer USER_JWT_TOKEN_FROM_LOGIN"
      }
    }
  }
}
```

### Python A2A Client

```python
import requests
import json

class PierreA2AClient:
    def __init__(self, base_url, client_id, client_secret):
        self.base_url = base_url
        self.client_id = client_id
        self.client_secret = client_secret
        self.session_token = None
    
    def authenticate(self):
        """Authenticate and get session token"""
        response = requests.post(
            f"{self.base_url}/a2a/auth",
            json={
                "client_id": self.client_id,
                "client_secret": self.client_secret,
                "scopes": ["read", "write"]
            }
        )
        data = response.json()
        self.session_token = data["session_token"]
        return self.session_token
    
    def execute_tool(self, tool_name, parameters):
        """Execute A2A tool"""
        headers = {
            "Authorization": f"Bearer {self.session_token}",
            "Content-Type": "application/json"
        }
        
        payload = {
            "jsonrpc": "2.0",
            "method": "tools.execute",
            "params": {
                "tool_name": tool_name,
                "parameters": parameters
            },
            "id": 1
        }
        
        response = requests.post(
            f"{self.base_url}/a2a/tools",
            json=payload,
            headers=headers
        )
        return response.json()

# Usage
client = PierreA2AClient(
    "http://localhost:3000",
    "your_client_id", 
    "your_client_secret"
)

client.authenticate()
result = client.execute_tool("get_activities", {"limit": 10})
print(result)
```

### Discord Bot Integration

```javascript
const { Client, GatewayIntentBits } = require('discord.js');
const axios = require('axios');

class PierreBot {
    constructor() {
        this.client = new Client({
            intents: [GatewayIntentBits.Guilds, GatewayIntentBits.GuildMessages]
        });
        this.pierreAPI = axios.create({
            baseURL: 'http://localhost:3000',
            headers: {
                'Authorization': 'Bearer YOUR_A2A_SESSION_TOKEN'
            }
        });
    }
    
    async getFitnessData(userId) {
        try {
            const response = await this.pierreAPI.post('/a2a/tools', {
                jsonrpc: "2.0",
                method: "tools.execute",
                params: {
                    tool_name: "get_activities",
                    parameters: { limit: 5 }
                },
                id: 1
            });
            
            return response.data.result;
        } catch (error) {
            console.error('Failed to get fitness data:', error);
            return null;
        }
    }
}

const bot = new PierreBot();
// Initialize bot...
```

## Troubleshooting

### Common Issues

#### 1. Database Connection Failed

Error: `Failed to connect to database`

Solutions:
- Check `DATABASE_URL` in `.env`
- Ensure PostgreSQL is running: `brew services start postgresql`
- Create database: `createdb pierre`
- Check permissions

#### 2. Redis Connection Failed

Error: `Failed to connect to Redis`

Solutions:
- Install Redis: `brew install redis`
- Start Redis: `redis-server`
- Check `REDIS_URL` in `.env`
- For development, you can disable Redis by commenting out `REDIS_URL`

#### 3. OAuth Configuration Issues

Error: `No OAuth credentials configured for tenant`

Solutions:
- Ensure OAuth credentials are set in `.env`
- Register your application with Strava/Fitbit
- Check redirect URIs match exactly
- Verify client ID and secret are correct

#### 4. MCP Connection Issues

Error: `WebSocket connection failed`

Solutions:
- Check API key is valid: `curl -X GET http://localhost:8081/api/keys/list -H "Authorization: Bearer JWT"`
- Verify WebSocket endpoint: `ws://localhost:8081/ws`
- Check server logs for authentication errors
- Ensure API key header format: `X-API-Key: your_key`

#### 5. Build Errors

Error: `Failed to compile`

Solutions:
- Update Rust: `rustup update`
- Clean build: `cargo clean && cargo build`
- Check Rust version: `rustc --version` (should be 1.70+)
- Resolve dependency conflicts: `cargo update`

### Debug Mode

Enable verbose logging:

```bash
RUST_LOG=debug,sqlx=info,reqwest=info cargo run
```

This will show:
- Database queries
- HTTP requests/responses
- Authentication attempts
- Protocol message handling

### Performance Issues

If the server is slow:

1. Check database performance:
   ```bash
   # Enable SQL query logging
   RUST_LOG=sqlx=debug cargo run
   ```

2. Monitor resource usage:
   ```bash
   # Check CPU/memory usage
   top -p $(pgrep pierre-mcp-server)
   ```

3. Optimize database:
   ```sql
   -- Add indexes for frequently queried columns
   CREATE INDEX idx_users_email ON users(email);
   CREATE INDEX idx_api_keys_user_id ON api_keys(user_id);
   ```

### Getting Help

1. Check the logs:
   ```bash
   tail -f logs/pierre.log
   ```

2. Enable debug mode:
   ```bash
   RUST_LOG=debug cargo run
   ```

3. Run tests to verify setup:
   ```bash
   cargo test --test integration_tests
   ```

4. Check GitHub Issues: Look for similar problems in the repository issues

5. Community Support: Join our Discord/Slack for real-time help

## Next Steps

### Explore the Codebase

1. Read the Architecture Guide: `docs/developer-guide/01-architecture.md`
2. Study Protocol Implementation: `docs/developer-guide/04-mcp-protocol.md`
3. Review API Documentation: `docs/developer-guide/14-api-reference.md`

### Development Tasks

1. Set up OAuth Integration:
   - Register Strava application
   - Test OAuth flow
   - Connect fitness accounts

2. Build Your First Tool:
   - Create custom MCP tool
   - Add business logic
   - Write comprehensive tests

3. Deploy to Production:
   - Set up PostgreSQL database
   - Configure environment variables
   - Deploy with Docker/Kubernetes

### Contribution Guidelines

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/my-new-feature`
3. Make changes and test: `./scripts/lint-and-test.sh`
4. Write tests: All new code must have tests
5. Update documentation: Keep docs in sync
6. Submit pull request: Follow the PR template

### Learning Resources

- Rust Book: https://doc.rust-lang.org/book/
- MCP Specification: https://spec.modelcontextprotocol.io/
- Strava API: https://developers.strava.com/
- JSON-RPC 2.0: https://www.jsonrpc.org/specification

You now have a working Pierre MCP Server development environment. You can begin building fitness AI integrations.