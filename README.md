<div align="center">
  <img src="templates/pierre-logo.svg" width="150" height="150" alt="Pierre Fitness Platform Logo">
  <h1>Pierre Fitness Platform</h1>
</div>

[![Backend CI](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/ci.yml/badge.svg)](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/ci.yml)
[![Frontend Tests](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/frontend-tests.yml/badge.svg)](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/frontend-tests.yml)
[![MCP Compliance](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/mcp-compliance.yml/badge.svg)](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/mcp-compliance.yml)

**Development Status**: This project is under active development. APIs and features may change.

Pierre Fitness Platform connects AI assistants to fitness data from Strava, Garmin, and Fitbit. The platform implements the Model Context Protocol (MCP), A2A protocol, OAuth 2.0, and REST APIs for integration with Claude, ChatGPT, and other AI assistants.

### Intelligence System

The platform calculates fitness metrics using established sports science formulas:

- **Training Load**: TSS (Training Stress Score) from power or heart rate data, CTL (42-day fitness), ATL (7-day fatigue), TSB (form indicator)
- **Race Predictions**: VDOT-based predictions using Jack Daniels' VO2max formula, Riegel formula for distance scaling
- **Statistical Analysis**: Linear regression for performance trends, R² coefficient for fit quality, moving averages for smoothing
- **Pattern Detection**: Weekly training schedule consistency, hard/easy workout alternation, volume progression analysis
- **Physiological Validation**: Bounds checking for heart rate (100-220 bpm max), power (50-600W FTP), VO2 max (20-90 ml/kg/min)

See [Intelligence and Analytics Methodology](docs/intelligence-methodology.md) for formulas, implementation details, and scientific references.

## Features

- **MCP Protocol**: JSON-RPC over HTTP for AI assistant integration
- **OAuth 2.0 Server**: RFC 7591 dynamic client registration for MCP clients
- **RS256/JWKS**: Asymmetric JWT signing with public key distribution
- **A2A Protocol**: Agent-to-agent communication with capability discovery
- **Multi-Tenancy**: Isolated data and configuration per organization
- **Real-Time Updates**: Server-Sent Events for OAuth notifications
- **Plugin System**: Compile-time plugin architecture with lifecycle management
- **PII Redaction**: Middleware for sensitive data removal in logs and responses
- **Cursor Pagination**: Keyset pagination for consistent large dataset traversal
- **Intelligent Caching**: LRU cache with TTL for API response optimization
- **Atomic Operations**: TOCTOU prevention with database-level atomic token operations
- **Structured Error Handling**: Type-safe error propagation with AppError/DatabaseError/ProviderError

## Architecture

Pierre Fitness Platform runs as a single HTTP server on port 8081 (configurable). All protocols (MCP, OAuth 2.0, REST API) share the same port.

### mcp transport modes

```
┌──────────────────────────────────────────────────────────────────────────────────────┐
│                              MCP Client Integration                                  │
├──────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                      │
│  stdio transport (subprocess-based)                                                  │
│  ┌─────────────────┐    stdio     ┌─────────────────┐    HTTP+OAuth   ┌──────────┐   │
│  │   MCP Client    │ ◄─────────►  │ Pierre SDK      │ ◄─────────────► │ Pierre   │   │
│  │ (Claude Desktop)│              │ Bridge          │                 │ Fitness  │   │
│  └─────────────────┘              └─────────────────┘                 │ Platform │   │
│                                                                       │          │   │
│  streamable http transport (server-based)                              │          │   │
│  ┌─────────────────┐    MCP-over-HTTP+OAuth                           │          │   │
│  │   MCP Client    │ ◄──────────────────────────────────────────────► │          │   │
│  │ (HTTP-native)   │                                                  │          │   │
│  └─────────────────┘                                                  └──────────┘   │
│                                                                                      │
└──────────────────────────────────────────────────────────────────────────────────────┘
```

**stdio transport** (via `pierre-mcp-client` npm package)
- mcp clients spawn server as subprocess and communicate via stdin/stdout
- for mcp clients using stdio transport (claude desktop, chatgpt, most existing clients)
- sdk bridge handles oauth 2.0 flow and token management automatically
- configuration: add sdk command to mcp client config (see mcp client integration section)

**streamable http transport** (direct http connection)
- mcp clients connect directly to pierre's http endpoint
- for mcp clients with streamable http transport support
- direct mcp-over-http communication with oauth 2.0 authentication
- configuration: implement oauth 2.0 flow and connect to `http://localhost:8081/mcp`

## LLM Interaction

AI assistants query fitness data through natural language. The LLM determines which MCP tools to call and combines results.

### Example Interactions

| Natural Language Request | What Happens | Tools Used |
|--------------------------|--------------|------------|
| "Get my last 10 activities and propose a week-long meal plan with protein targets based on my training load" | Retrieves recent activities, analyzes intensity and duration, calculates caloric expenditure, generates nutrition recommendations with macro breakdowns | `get_activities`, `analyze_training_load`, `calculate_metrics` |
| "Compare my three longest runs this month and identify areas for improvement" | Fetches top runs by distance, analyzes pace consistency, heart rate zones, elevation patterns, provides feedback | `get_activities`, `compare_activities`, `analyze_performance_trends` |
| "Analyze my cycling data from the past 3 months and suggest realistic goals for next quarter" | Reviews historical performance, detects trends and patterns, evaluates fitness progression, recommends targets | `get_activities`, `analyze_performance_trends`, `suggest_goals`, `analyze_goal_feasibility` |
| "Check my training load for the last two weeks and tell me if I need a recovery day" | Calculates cumulative training stress, analyzes recovery metrics, provides rest recommendations | `analyze_training_load`, `get_activities`, `generate_recommendations` |
| "When's the best day this week for an outdoor run based on my typical schedule and weather conditions?" | Analyzes activity patterns, checks weather forecasts, recommends timing | `detect_patterns`, `get_activities`, weather integration |

## Quick Start

### Prerequisites

- Rust 1.70+
- SQLite (default) or PostgreSQL (production)

### Installation

```bash
git clone https://github.com/Async-IO/pierre_mcp_server.git
cd pierre_mcp_server
cargo build --release
```

### Configuration

#### Using `.envrc` (Recommended)

Pierre Fitness Platform includes a `.envrc` file for environment configuration. Use [direnv](https://direnv.net/) to automatically load environment variables:

```bash
# Install direnv (macOS)
brew install direnv

# Add to your shell profile (~/.zshrc or ~/.bashrc)
eval "$(direnv hook zsh)"  # or bash

# Allow direnv for this directory
cd pierre_mcp_server
direnv allow
```

The `.envrc` file includes all required configuration with development defaults. Edit `.envrc` to customize settings for your environment.

#### Manual Configuration

**Required Environment Variables**:
```bash
export DATABASE_URL="sqlite:./data/pierre.db"
export PIERRE_MASTER_ENCRYPTION_KEY="$(openssl rand -base64 32)"
```

**Optional Environment Variables**:
```bash
export HTTP_PORT=8081              # Server port (default: 8081)
export RUST_LOG=info               # Log level
export JWT_EXPIRY_HOURS=24         # JWT token expiry
export PIERRE_RSA_KEY_SIZE=4096    # RSA key size for JWT signing (default: 4096)

# Fitness provider OAuth (for data integration)
export STRAVA_CLIENT_ID=your_client_id
export STRAVA_CLIENT_SECRET=your_client_secret
export STRAVA_REDIRECT_URI=http://localhost:8081/oauth/callback/strava  # local dev only

# Garmin Connect OAuth (optional)
export GARMIN_CLIENT_ID=your_consumer_key
export GARMIN_CLIENT_SECRET=your_consumer_secret
export GARMIN_REDIRECT_URI=http://localhost:8081/oauth/callback/garmin  # local dev only

# Production: Use HTTPS for callback URLs
# export STRAVA_REDIRECT_URI=https://api.example.com/oauth/callback/strava
# export GARMIN_REDIRECT_URI=https://api.example.com/oauth/callback/garmin

# Weather data (optional)
export OPENWEATHER_API_KEY=your_api_key

# Cache configuration
export CACHE_MAX_ENTRIES=10000                    # Maximum cached entries (default: 10,000)
export CACHE_CLEANUP_INTERVAL_SECS=300            # Cleanup interval in seconds (default: 300)
# export REDIS_URL=redis://localhost:6379         # Redis cache (future support)
```

See `src/constants/mod.rs:32-173` for all environment variables.

### Starting the Server

```bash
cargo run --bin pierre-mcp-server
```

The server will start on port 8081 and display available endpoints.

### Initial Setup

Create an admin user via REST API:

```bash
curl -X POST http://localhost:8081/admin/setup \
  -H "Content-Type: application/json" \
  -d '{
    "email": "admin@example.com",
    "password": "SecurePass123!",
    "display_name": "Admin"
  }'
```

## MCP Client Integration

Pierre Fitness Platform includes an SDK bridge for direct integration with MCP clients. The SDK handles OAuth 2.0 authentication automatically.

### SDK Installation

**Option 1: Install from npm (Recommended)**

```bash
npm install pierre-mcp-client@next
```

The SDK is published as a pre-release package (`@next` tag) during v0.x development.

**Option 2: Build from source**

The SDK is included in the `sdk/` directory:

```bash
cd sdk
npm install
npm run build
```

### MCP Client Configuration

Add Pierre to your MCP client configuration. For Claude Desktop:

**Configuration File Location**:
- macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`
- Windows: `%APPDATA%\Claude\claude_desktop_config.json`
- Linux: `~/.config/claude/claude_desktop_config.json`

**Configuration (using npm package)**:
```json
{
  "mcpServers": {
    "pierre-fitness": {
      "command": "npx",
      "args": [
        "-y",
        "pierre-mcp-client@next",
        "--server",
        "http://localhost:8081"
      ]
    }
  }
}
```

**Alternative (using local installation)**:
```json
{
  "mcpServers": {
    "pierre-fitness": {
      "command": "node",
      "args": [
        "/absolute/path/to/pierre_mcp_server/sdk/dist/cli.js",
        "--server",
        "http://localhost:8081"
      ]
    }
  }
}
```

Replace `/absolute/path/to/` with your actual path for local installations.

### Authentication Flow

When the MCP client starts, the SDK will:

1. Register an OAuth 2.0 client with Pierre Fitness Platform (RFC 7591)
2. Open your browser for authentication
3. Handle the OAuth callback and token exchange
4. Use JWT tokens for all MCP requests

No manual token management required.

## streamable http transport

mcp clients with streamable http transport support can connect directly to pierre without the sdk bridge.

### http transport setup

mcp clients with streamable http transport connect directly to the mcp endpoint:

```
endpoint: http://localhost:8081/mcp (development)
endpoint: https://your-server.com/mcp (production)
```

### http transport authentication

streamable http connections use oauth 2.0 authorization code flow:

1. client discovers oauth configuration from `/.well-known/oauth-authorization-server`
2. client registers dynamically using rfc 7591 (`/oauth2/register`)
3. opens browser for user authentication (`/oauth2/authorize`)
4. exchanges authorization code for jwt token (`/oauth2/token`)
5. uses jwt token for all subsequent mcp requests

### choosing transport mode

| feature | stdio transport | streamable http transport |
|---------|----------------|---------------------------|
| **best for** | claude desktop, chatgpt, existing mcp clients | web services, distributed systems, custom tooling |
| **connection** | subprocess (stdin/stdout) | http server |
| **setup** | npm package (`pierre-mcp-client`) | http client implementation |
| **oauth flow** | automatic via sdk bridge | client implements oauth 2.0 flow |
| **performance** | low latency (local ipc) | network overhead (http) |
| **multi-client** | one client per process | multiple concurrent clients |
| **use cases** | desktop applications, ide integrations | remote deployments, multi-tenant systems |

## Available MCP Tools

Pierre Fitness Platform provides 25 tools through the MCP protocol. Tool definitions are in `src/protocols/universal/tool_registry.rs:12-45`.

### Core Fitness Data

| Tool | Description | Parameters |
|------|-------------|------------|
| `get_activities` | Get user activities from fitness providers | `provider` (optional), `limit` (optional) |
| `get_athlete` | Get athlete profile information | None |
| `get_stats` | Get athlete statistics and metrics | None |
| `analyze_activity` | Analyze a specific activity with detailed insights | `activity_id` (required) |
| `get_activity_intelligence` | Get AI-powered analysis for an activity | `activity_id` (required) |
| `get_connection_status` | Check OAuth connection status for providers | None |
| `disconnect_provider` | Disconnect from a fitness provider | `provider` (required) |

### Goals and Progress

| Tool | Description | Parameters |
|------|-------------|------------|
| `set_goal` | Set a new fitness goal | `goal_type`, `target_value` (required) |
| `suggest_goals` | Get AI-suggested goals based on activity history | None |
| `analyze_goal_feasibility` | Analyze if a goal is achievable | `goal_data` (required) |
| `track_progress` | Track progress toward goals | `goal_id` (required) |

### Performance Analysis

| Tool | Description | Parameters |
|------|-------------|------------|
| `calculate_metrics` | Calculate custom fitness metrics | `activity_id` (required) |
| `analyze_performance_trends` | Analyze performance trends over time | None |
| `compare_activities` | Compare activities for performance analysis | `activity_ids` (required) |
| `detect_patterns` | Detect patterns in activity data | None |
| `generate_recommendations` | Generate personalized training recommendations | None |
| `calculate_fitness_score` | Calculate overall fitness score | None |
| `predict_performance` | Predict future performance based on training | None |
| `analyze_training_load` | Analyze training load and recovery | None |

### Configuration Management

| Tool | Description | Parameters |
|------|-------------|------------|
| `get_configuration_catalog` | Get complete configuration catalog | None |
| `get_configuration_profiles` | Get available configuration profiles | None |
| `get_user_configuration` | Get current user configuration | None |
| `update_user_configuration` | Update user configuration parameters | `profile` or `parameters` (required) |
| `calculate_personalized_zones` | Calculate personalized training zones | None |
| `validate_configuration` | Validate configuration parameters | `parameters` (required) |

Tool descriptions from `src/protocols/universal/tool_registry.rs:114-162`.

## Authentication

Pierre Fitness Platform supports multiple authentication methods for different use cases.

### two oauth systems

pierre implements two separate oauth systems:

**oauth client** (pierre → fitness providers):
- pierre connects TO fitness providers (strava, garmin, fitbit) to fetch user activity data
- configure with provider credentials: `STRAVA_CLIENT_ID`, `FITBIT_CLIENT_ID`, `GARMIN_CLIENT_ID`
- handles user authorization and automatic token refresh
- implementation: `src/oauth2_client/`, `src/providers/`
- see [oauth client documentation](docs/oauth-client.md) for technical details

**oauth server** (mcp clients → pierre):
- mcp clients (claude desktop, custom integrations) connect TO pierre for authentication
- implements rfc 7591 (dynamic client registration) and rfc 7636 (pkce)
- issues jwt access tokens for mcp protocol requests
- implementation: `src/oauth2_server/`
- endpoints: `/oauth2/register`, `/oauth2/authorize`, `/oauth2/token`, `/oauth2/jwks`
- see [oauth2 server documentation](docs/oauth2-server.md) for technical details

**quick summary**: configure fitness provider credentials for data access. mcp clients use oauth2 endpoints for authentication.

### oauth 2.0 for mcp clients

the pierre sdk (`pierre-mcp-client` npm package) handles oauth 2.0 authentication automatically for stdio transport. for streamable http transport:

**discovery endpoint:**
```
GET /.well-known/oauth-authorization-server
```

**registration:**
```bash
POST /oauth2/register
{
  "redirect_uris": ["https://client.example.com/callback"],
  "client_name": "My MCP Client",
  "grant_types": ["authorization_code"]
}
```

**authorization and token exchange:**
1. redirect user to `/oauth2/authorize` with client_id and pkce challenge
2. exchange authorization code at `/oauth2/token` for jwt access token
3. use jwt token in `Authorization: Bearer <token>` header for all mcp requests

see [oauth2 server documentation](docs/oauth2-server.md) for complete flow, pkce requirements, and error handling.

### JWT Authentication

For direct REST API access:

```bash
# Register user
curl -X POST http://localhost:8081/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email": "user@example.com", "password": "password123", "display_name": "User"}'

# Login to get JWT token
curl -X POST http://localhost:8081/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email": "user@example.com", "password": "password123"}'

# Use JWT token
curl -H "Authorization: Bearer YOUR_JWT_TOKEN" http://localhost:8081/mcp
```

### API Key Authentication

For service-to-service integration:

```bash
# Create API key (requires admin or user JWT)
curl -X POST http://localhost:8081/api/keys \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name": "My Service", "tier": "professional"}'

# Use API key
curl -H "X-API-Key: YOUR_API_KEY" http://localhost:8081/api/endpoint
```

## A2A (Agent-to-Agent) Protocol

Pierre Fitness Platform supports agent-to-agent communication for autonomous AI systems. Implementation in `src/a2a/`.

**A2A Features**:
- Agent Cards for capability discovery (`src/a2a/agent_card.rs`)
- Cryptographic authentication between agents
- Asynchronous messaging protocol
- Protocol versioning (A2A 1.0.0)

**A2A Endpoints**:
- `GET /a2a/status` - Get A2A protocol status
- `GET /a2a/tools` - Get available A2A tools
- `POST /a2a/execute` - Execute A2A tool
- `GET /a2a/monitoring` - Get A2A monitoring information
- `GET /a2a/client/tools` - Get client-specific A2A tools
- `POST /a2a/client/execute` - Execute client A2A tool

**Example A2A Integration**:
```rust
use pierre_mcp_server::a2a::A2AClientManager;

#[tokio::main]
async fn main() -> Result<()> {
    let client = A2AClientManager::new("https://pierre-server.com/a2a").await?;

    let response = client.send_message(
        "fitness-analyzer-agent",
        serde_json::json!({
            "action": "analyze_performance",
            "user_id": "user-123",
            "timeframe": "last_30_days"
        })
    ).await?;

    println!("Analysis: {}", response);
    Ok(())
}
```

## Real-Time Notifications

Pierre Fitness Platform provides Server-Sent Events (SSE) for real-time updates. Implementation in `src/notifications/sse.rs` and `src/sse.rs`.

**SSE Endpoint**:
```
GET /notifications/sse?user_id={user_id}
```

**Notification Types**:
- OAuth authorization completion
- OAuth errors and failures
- System status updates
- A2A message notifications

**Example JavaScript Integration**:
```javascript
const eventSource = new EventSource('/notifications/sse?user_id=user-123');

eventSource.onmessage = function(event) {
    const notification = JSON.parse(event.data);
    console.log('Notification:', notification);

    if (notification.type === 'oauth_complete') {
        // Handle OAuth completion
        window.location.reload();
    }
};
```

## Testing

```bash
# Run all tests
cargo test

# Run specific test suite
cargo test --test mcp_protocol_comprehensive_test
cargo test --test mcp_multitenant_complete_test

# Run with output
cargo test -- --nocapture

# Lint and test (comprehensive validation)
./scripts/lint-and-test.sh
```

### RSA Key Size Configuration

Pierre Fitness Platform uses RS256 asymmetric signing for JWT tokens. Key size affects both security and performance:

**Production (4096-bit keys - default)**:
- Higher security with larger key size
- Slower key generation (~10 seconds)
- Use in production environments

**Testing (2048-bit keys)**:
- Faster key generation (~250ms)
- Suitable for development and testing
- Set via environment variable:

```bash
export PIERRE_RSA_KEY_SIZE=2048
```

### Test Performance Optimization

Pierre Fitness Platform includes a shared test JWKS manager to eliminate RSA key generation overhead:

**Shared Test JWKS Pattern** (implemented in `tests/common.rs:40-52`):
```rust
use pierre_mcp_server_integrations::common;

// Reuses shared JWKS manager across all tests (10x faster)
let jwks_manager = common::get_shared_test_jwks();
```

**Performance Impact**:
- **Without optimization**: 100ms+ RSA key generation per test
- **With shared JWKS**: One-time generation, instant reuse across test suite
- **Result**: 10x faster test execution

**E2E Tests**: The SDK test suite (`sdk/test/`) automatically uses 2048-bit keys via `PIERRE_RSA_KEY_SIZE=2048` in server startup configuration (`sdk/test/helpers/server.js:82`).

## Development Tools

### Automated Setup

```bash
# Clean database and fresh server start
./scripts/fresh-start.sh
cargo run --bin pierre-mcp-server &

# Complete workflow test (admin + user + tenant + login + MCP)
./scripts/complete-user-workflow.sh

# Load saved environment variables
source .workflow_test_env
echo "JWT Token: ${JWT_TOKEN:0:50}..."
```

### Management Dashboard

A web dashboard is available for monitoring:

```bash
cd frontend
npm install && npm run dev
```

Access at `http://localhost:5173` for:
- User management and approval
- API key monitoring
- Usage analytics
- Real-time request monitoring

See `frontend/README.md` for details.

## Documentation

Complete documentation is in the `docs/` directory:

- **[Getting Started](docs/getting-started.md)** - installation and quick start
- **[Architecture](docs/architecture.md)** - system design and components
- **[Protocols](docs/protocols.md)** - mcp, oauth2, a2a, rest protocols
- **[Authentication](docs/authentication.md)** - jwt, api keys, oauth2
- **[Configuration](docs/configuration.md)** - environment variables and settings
- **[Contributing](docs/contributing.md)** - development guidelines

Installation guide for MCP clients:
- **[MCP Client Installation](docs/installation-guides/install-mcp-client.md)** - claude desktop, chatgpt, and other mcp clients

## Code Quality

Pierre Fitness Platform uses validation scripts to maintain code quality and prevent common issues:

**Pre-commit Validation**:
- Pattern validation via `scripts/validation-patterns.toml`
- Clippy linting with strict warnings
- Test execution
- Format checking

**Run validation**:
```bash
./scripts/lint-and-test.sh
```

Install git hooks:
```bash
./scripts/install-hooks.sh
```

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/new-feature`)
3. Run validation (`./scripts/lint-and-test.sh`)
4. Commit changes (`git commit -m 'feat: add new feature'`)
5. Push to branch (`git push origin feature/new-feature`)
6. Open a Pull Request

## License

This project is dual-licensed:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

You may choose either license.
