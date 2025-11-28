<div align="center">
  <img src="templates/pierre-logo.svg" width="150" height="150" alt="Pierre Fitness Platform Logo">
  <h1>Pierre Fitness Platform</h1>
</div>

[![Backend CI](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/ci.yml/badge.svg)](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/ci.yml)
[![Frontend Tests](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/frontend-tests.yml/badge.svg)](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/frontend-tests.yml)
[![SDK Tests](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/sdk-tests.yml/badge.svg)](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/sdk-tests.yml)
[![MCP Compliance](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/mcp-compliance.yml/badge.svg)](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/mcp-compliance.yml)
[![Cognitive Complexity](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/cognitive-complexity.yml/badge.svg)](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/cognitive-complexity.yml)

**Development Status**: This project is under active development. APIs and features may change.

Pierre Fitness Platform connects AI assistants to fitness data from Strava, Garmin, and Fitbit. The platform implements the Model Context Protocol (MCP), A2A protocol, OAuth 2.0, and REST APIs for integration with Claude, ChatGPT, and other AI assistants.

### Intelligence System

The platform calculates fitness metrics using established sports science formulas:

- **Training Load**: TSS (Training Stress Score) from power or heart rate data, CTL (42-day fitness), ATL (7-day fatigue), TSB (form indicator)
- **Race Predictions**: VDOT-based predictions using Jack Daniels' VO2max formula, Riegel formula for distance scaling
- **Statistical Analysis**: Linear regression for performance trends, R² coefficient for fit quality, moving averages for smoothing
- **Pattern Detection**: Weekly training schedule consistency, hard/easy workout alternation, volume progression analysis
- **Sleep and Recovery**: NSF/AASM-based sleep quality scoring, TSB normalization, HRV-based recovery assessment, weighted recovery calculations
- **Nutrition and USDA Integration**: Mifflin-St Jeor BMR, TDEE with activity factors, sport-specific protein/carb/fat recommendations, nutrient timing, USDA FoodData Central integration (350,000+ foods)
- **Physiological Validation**: Bounds checking for heart rate (100-220 bpm max), power (50-600W FTP), VO2 max (20-90 ml/kg/min)
- **Configurable Algorithms**: All calculation algorithms support multiple variants (e.g., Daniels vs Riegel for VDOT, Bannister vs Edwards for TRIMP) selectable via environment variables for runtime customization

See [Intelligence and Analytics Methodology](docs/intelligence-methodology.md) and [Nutrition Methodology](docs/nutrition-methodology.md) for formulas, implementation details, and scientific references. Sleep and recovery analysis is documented in the Intelligence Methodology. See [Algorithm Configuration](docs/configuration.md#algorithm-configuration) for available algorithm variants and configuration options.

## Features

- **MCP Protocol**: JSON-RPC over HTTP for AI assistant integration
- **A2A Protocol**: Agent-to-agent communication with capability discovery
- **OAuth 2.0 Server**: RFC 7591 dynamic client registration for MCP clients
- **RS256/JWKS**: Asymmetric JWT signing with public key distribution
- **Plugin System**: Compile-time plugin architecture with lifecycle management
- **TypeScript SDK**: npm package `pierre-mcp-client` for stdio transport integration ([SDK documentation](./sdk/README.md))
- **Pluggable Providers**: Service Provider Interface (SPI) with feature flags for compile-time provider selection

### Provider Architecture

Pierre supports multiple fitness providers through a pluggable architecture:

| Provider | Feature Flag | Status | Capabilities |
|----------|-------------|--------|--------------|
| Strava | `provider-strava` | ✅ Full | Activities, Stats, Routes |
| Garmin | `provider-garmin` | ✅ Full | Activities, Sleep, Health |
| WHOOP | `provider-whoop` | ✅ Full | Sleep, Recovery, Activities, Health |
| Synthetic | `provider-synthetic` | ✅ Full | Development/Testing (no OAuth) |
| Fitbit | `provider-fitbit` | 🚧 Planned | Activities, Sleep, Health |

**Build with specific providers:**
```bash
# All providers (default)
cargo build --release

# Only Strava
cargo build --release --no-default-features --features "sqlite,provider-strava"

# Strava + Garmin (no synthetic)
cargo build --release --no-default-features --features "sqlite,provider-strava,provider-garmin"

# Strava + WHOOP (sleep, recovery, activities)
cargo build --release --no-default-features --features "sqlite,provider-strava,provider-whoop"
```

See [Chapter 17.5: Pluggable Provider Architecture](docs/tutorial/chapter-17.5-pluggable-providers.md) for detailed documentation on adding custom providers.

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
│  streamable http transport (server-based)                             │          │   │
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
| "Calculate my daily nutrition needs for marathon training and suggest pre-workout meals" | Calculates BMR/TDEE based on user profile, determines macros for endurance goal, calculates nutrient timing, searches USDA database for suitable pre-workout foods | `calculate_daily_nutrition`, `calculate_nutrient_timing`, `search_foods` |
| "Get my last 10 activities and propose a week-long meal plan with protein targets based on my training load" | Retrieves recent activities, analyzes intensity and duration, calculates caloric expenditure, generates nutrition recommendations with macro breakdowns | `get_activities`, `analyze_training_load`, `calculate_daily_nutrition` |
| "Compare my three longest runs this month and identify areas for improvement" | Fetches top runs by distance, analyzes pace consistency, heart rate zones, elevation patterns, provides feedback | `get_activities`, `compare_activities`, `analyze_performance_trends` |
| "Analyze this meal: 150g chicken breast, 200g rice, 100g broccoli" | Looks up each food in USDA database, retrieves complete nutrient breakdown, calculates total macros and calories for the meal | `analyze_meal_nutrition`, `get_food_details` |
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
export DATABASE_URL="sqlite:./data/users.db"
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
export STRAVA_REDIRECT_URI=http://localhost:8081/api/oauth/callback/strava  # local dev only

# Garmin Connect OAuth (optional)
export GARMIN_CLIENT_ID=your_consumer_key
export GARMIN_CLIENT_SECRET=your_consumer_secret
export GARMIN_REDIRECT_URI=http://localhost:8081/api/oauth/callback/garmin  # local dev only

# WHOOP OAuth (optional)
export WHOOP_CLIENT_ID=your_client_id
export WHOOP_CLIENT_SECRET=your_client_secret
export WHOOP_REDIRECT_URI=http://localhost:8081/api/oauth/callback/whoop  # local dev only

# Production: Use HTTPS for callback URLs
# export STRAVA_REDIRECT_URI=https://api.example.com/api/oauth/callback/strava
# export GARMIN_REDIRECT_URI=https://api.example.com/api/oauth/callback/garmin
# export WHOOP_REDIRECT_URI=https://api.example.com/api/oauth/callback/whoop

# Weather data (optional)
export OPENWEATHER_API_KEY=your_api_key

# Algorithm configuration (optional)
export PIERRE_MAXHR_ALGORITHM=tanaka           # Max heart rate: fox, tanaka, nes, gulati
export PIERRE_TRIMP_ALGORITHM=hybrid           # Training impulse: bannister_male, bannister_female, edwards_simplified, lucia_banded, hybrid
export PIERRE_TSS_ALGORITHM=avg_power          # Training stress score: avg_power, normalized_power, hybrid
export PIERRE_VDOT_ALGORITHM=daniels           # Running performance: daniels, riegel, hybrid
export PIERRE_TRAINING_LOAD_ALGORITHM=ema      # Training load: ema, sma, wma, kalman
export PIERRE_RECOVERY_ALGORITHM=weighted      # Recovery aggregation: weighted, additive, multiplicative, minmax, neural
export PIERRE_FTP_ALGORITHM=from_vo2max        # Functional threshold power: 20min_test, 8min_test, ramp_test, from_vo2max, hybrid
export PIERRE_LTHR_ALGORITHM=from_maxhr        # Lactate threshold HR: from_maxhr, from_30min, from_race, lab_test, hybrid
export PIERRE_VO2MAX_ALGORITHM=from_vdot       # VO2max estimation: from_vdot, cooper, rockport, astrand, bruce, hybrid
# See docs/configuration.md#algorithm-configuration for details

# Cache configuration
export CACHE_MAX_ENTRIES=10000                    # Maximum cached entries (default: 10,000)
export CACHE_CLEANUP_INTERVAL_SECS=300            # Cleanup interval in seconds (default: 300)
export REDIS_URL=redis://localhost:6379           # Redis cache (optional, uses in-memory if not set)
```

See `src/constants/mod.rs` for all environment variables and default values.

### Starting the Server

```bash
cargo run --bin pierre-mcp-server
```

The server will start on port 8081 and display available endpoints.

### Initial Setup

#### Required Environment Variables

Before starting the server, ensure these environment variables are set:

```bash
# Database connection
export DATABASE_URL="sqlite:./data/users.db"

# Encryption key (generate with: openssl rand -base64 32)
export PIERRE_MASTER_ENCRYPTION_KEY="$(openssl rand -base64 32)"

# Optional: Server port (default: 8081)
export HTTP_PORT=8081
```

For convenience, use the included `.envrc` file with [direnv](https://direnv.net/).

#### Create Database

Ensure the data directory exists:

```bash
mkdir -p ./data
```

#### Create Admin User

After starting the server, create an admin user:

```bash
curl -X POST http://localhost:8081/admin/setup \
  -H "Content-Type: application/json" \
  -d '{
    "email": "admin@example.com",
    "password": "SecurePass123!",
    "display_name": "Admin"
  }'
```

The response includes an admin token. Save this token for administrative operations.

#### Register and Approve Users

> **Security Note**: User registration requires admin authentication to prevent unauthorized account creation.

1. **Register a user** (requires admin token):

```bash
curl -X POST http://localhost:8081/api/auth/register \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer {admin_token}" \
  -d '{
    "email": "user@example.com",
    "password": "userpass123",
    "display_name": "Regular User"
  }'
```

Replace `{admin_token}` with the admin token from the setup response. The response includes a `user_id`. Save this for the approval step.

2. **Approve the user** (requires admin token):

```bash
curl -X POST http://localhost:8081/admin/approve-user/{user_id} \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer {admin_token}" \
  -d '{
    "reason": "User registration approved",
    "create_default_tenant": true,
    "tenant_name": "User Organization",
    "tenant_slug": "user-org"
  }'
```

Replace `{user_id}` with the user ID from step 1 and `{admin_token}` with the admin token from the setup response.

3. **User login** (get JWT token for MCP access):

```bash
curl -X POST http://localhost:8081/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "userpass123"
  }'
```

The response includes a `jwt_token`. Use this token for MCP protocol requests.

#### Automated Setup Script

For testing and development, use the automated workflow script:

```bash
# Clean database and start fresh
./scripts/fresh-start.sh
cargo run --bin pierre-mcp-server &

# Run complete setup (creates admin, user, tenant, and tests MCP)
./scripts/complete-user-workflow.sh

# Load generated environment variables for manual testing
source .workflow_test_env
echo "JWT Token: ${JWT_TOKEN:0:50}..."
```

The script creates:
- Admin user: `admin@pierre.mcp`
- Regular user: `user@example.com`
- Default tenant: `User Organization`
- JWT token for MCP access (saved in `.workflow_test_env`)

## MCP Client Integration

Pierre Fitness Platform includes an SDK bridge for direct integration with MCP clients that only support stdin/out. The SDK handles OAuth 2.0 authentication automatically.

### SDK Installation

**Requirements**:
- Node.js 18.0.0 or higher
- Pierre MCP Server running on port 8081

**Option 1: npx (No Installation Required)**

```bash
npx -y pierre-mcp-client@next --server http://localhost:8081
```

**Option 2: Install from npm**

```bash
npm install pierre-mcp-client@next
```

The SDK is published as `@next` during v0.x development.

**Option 3: Build from source**

```bash
cd sdk
npm install
npm run build
```

**What the SDK provides**:
- Automatic OAuth 2.0 registration and authentication
- Token management and refresh
- stdio transport for MCP clients (Claude Desktop, ChatGPT)
- Auto-generated TypeScript types for 45+ MCP tools

See [SDK README](sdk/README.md) for detailed documentation.

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

## Streamable http transport

Mcp clients with streamable http transport support can connect directly to pierre without the sdk bridge.

### Http transport setup

Mcp clients with streamable http transport connect directly to the mcp endpoint:

```
endpoint: http://localhost:8081/mcp (development)
endpoint: https://your-server.com/mcp (production)
```

### Http transport authentication

Streamable http connections use oauth 2.0 authorization code flow:

1. Client discovers oauth configuration from `/.well-known/oauth-authorization-server`
2. Client registers dynamically using rfc 7591 (`/oauth2/register`)
3. Opens browser for user authentication (`/oauth2/authorize`)
4. Exchanges authorization code for jwt token (`/oauth2/token`)
5. Uses jwt token for all subsequent mcp requests

## Available MCP Tools

Pierre Fitness Platform provides 46 tools through the MCP protocol. Tool definitions are in `src/protocols/universal/tool_registry.rs` (ToolId enum).

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

### Sleep and Recovery

| Tool | Description | Parameters |
|------|-------------|------------|
| `analyze_sleep_quality` | Analyze sleep quality with NSF/AASM scoring | `sleep_session` (required) |
| `calculate_recovery_score` | Calculate recovery readiness from TSB, sleep, HRV | `tsb`, `sleep_quality`, `hrv_data` (optional) |
| `track_sleep_trends` | Track sleep patterns and trends over time | `start_date`, `end_date` (required) |
| `optimize_sleep_schedule` | Get personalized sleep timing recommendations | `preferences` (optional) |
| `suggest_rest_day` | Get rest day recommendations based on recovery | `tsb`, `recent_load`, `sleep_quality` (optional) |

### Configuration Management

| Tool | Description | Parameters |
|------|-------------|------------|
| `get_configuration_catalog` | Get complete configuration catalog | None |
| `get_configuration_profiles` | Get available configuration profiles | None |
| `get_user_configuration` | Get current user configuration | None |
| `update_user_configuration` | Update user configuration parameters | `profile` or `parameters` (required) |
| `calculate_personalized_zones` | Calculate personalized training zones | None |
| `validate_configuration` | Validate configuration parameters | `parameters` (required) |

Tool descriptions from `src/protocols/universal/tool_registry.rs` (ToolId enum description method).

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

## Testing

```bash
# Run all tests
cargo test

# Lint and test
./scripts/lint-and-test.sh
```

For comprehensive testing documentation including multi-tenant tests, intelligence testing framework, RSA key configuration, and performance optimization, see [docs/testing.md](docs/testing.md).

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

A React + Vite web dashboard is available for monitoring and administration.

#### Quick Start

```bash
# Start backend server (terminal 1)
./bin/start-server.sh

# Start frontend dev server (terminal 2)
./bin/start-frontend.sh
```

The frontend script loads `.envrc` for `VITE_BACKEND_URL` configuration (defaults to `http://localhost:8081`).

#### Manual Setup

```bash
cd frontend
npm install
npm run dev
```

#### Environment Configuration

Add to `.envrc` for custom backend URL:
```bash
export VITE_BACKEND_URL="http://localhost:8081"
```

#### Frontend Features

Access at `http://localhost:5173` for:
- User management and approval
- API key monitoring
- Usage analytics
- Real-time request monitoring
- OAuth connection status

#### Available Scripts

| Script | Location | Description |
|--------|----------|-------------|
| `./bin/start-server.sh` | Project root | Start backend on port 8081 |
| `./bin/stop-server.sh` | Project root | Stop backend server |
| `./bin/start-frontend.sh` | Project root | Start Vite dev server |

See `frontend/README.md` for detailed frontend documentation.

## Documentation

Complete documentation is in the `docs/` directory:

- **[Getting Started](docs/getting-started.md)** - installation and quick start
- **[Architecture](docs/architecture.md)** - system design and components
- **[Protocols](docs/protocols.md)** - mcp, oauth2, a2a, rest protocols
- **[Authentication](docs/authentication.md)** - jwt, api keys, oauth2
- **[OAuth Client](docs/oauth-client.md)** - fitness provider oauth connections (strava, fitbit, garmin)
- **[OAuth2 Server](docs/oauth2-server.md)** - mcp client authentication
- **[Configuration](docs/configuration.md)** - environment variables and settings
- **[Contributing](docs/contributing.md)** - development guidelines

Component documentation:
- **[SDK Documentation](sdk/README.md)** - TypeScript SDK for MCP client integration
- **[Scripts Documentation](scripts/README.md)** - development, testing, and deployment scripts

Installation guide for MCP clients:
- **[MCP Client Installation](docs/installation-guides/install-mcp-client.md)** - claude desktop, chatgpt, and other mcp clients

## Code Quality

Pierre Fitness Platform uses validation scripts to maintain code quality and prevent common issues. See [Scripts Documentation](scripts/README.md) for the complete script inventory.

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
./scripts/setup-git-hooks.sh
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
