<div align="center">
  <img src="templates/pierre-logo.svg" width="150" height="150" alt="Pierre Fitness Platform Logo">
  <h1>Pierre Fitness Platform</h1>
</div>

[![Backend CI](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/ci.yml/badge.svg)](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/ci.yml)
[![Cross-Platform](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/cross-platform.yml/badge.svg)](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/cross-platform.yml)
[![Frontend Tests](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/frontend-tests.yml/badge.svg)](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/frontend-tests.yml)
[![SDK Tests](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/sdk-tests.yml/badge.svg)](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/sdk-tests.yml)
[![MCP Compliance](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/mcp-compliance.yml/badge.svg)](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/mcp-compliance.yml)

Pierre Fitness Platform connects AI assistants to fitness data from Strava, Garmin, Fitbit, and WHOOP. Implements Model Context Protocol (MCP), A2A protocol, OAuth 2.0, and REST APIs for Claude, ChatGPT, and other AI assistants.

## Intelligence System

Sports science-based fitness analysis:

- **Training Load**: TSS, CTL (42-day fitness), ATL (7-day fatigue), TSB (form)
- **Race Predictions**: VDOT (Jack Daniels), Riegel formula
- **Sleep & Recovery**: NSF/AASM scoring, HRV-based recovery, TSB normalization
- **Nutrition**: Mifflin-St Jeor BMR, TDEE, macros, USDA FoodData Central (350k+ foods)
- **Pattern Detection**: Training consistency, hard/easy alternation, volume progression
- **Configurable Algorithms**: Runtime selection via environment variables

See [Intelligence Methodology](docs/intelligence-methodology.md) and [Nutrition Methodology](docs/nutrition-methodology.md).

## Features

- **MCP Protocol**: JSON-RPC 2.0 for AI assistant integration
- **A2A Protocol**: Agent-to-agent communication
- **OAuth 2.0 Server**: RFC 7591 dynamic client registration
- **45 MCP Tools**: Activities, goals, analysis, sleep, recovery, nutrition, configuration
- **TypeScript SDK**: `pierre-mcp-client` npm package
- **Pluggable Providers**: Compile-time provider selection
- **TOON Format**: Token-Oriented Object Notation output for ~40% LLM token reduction ([spec](https://toonformat.dev))

## Provider Support

| Provider | Feature Flag | Capabilities |
|----------|-------------|--------------|
| Strava | `provider-strava` | Activities, Stats, Routes |
| Garmin | `provider-garmin` | Activities, Sleep, Health |
| WHOOP | `provider-whoop` | Sleep, Recovery, Strain |
| Fitbit | `provider-fitbit` | Activities, Sleep, Health |
| Synthetic | `provider-synthetic` | Development/Testing |

Build with specific providers:
```bash
cargo build --release                                                    # all providers
cargo build --release --no-default-features --features "sqlite,provider-strava"  # strava only
```

See [Pluggable Provider Architecture](docs/tutorial/chapter-17.5-pluggable-providers.md).

## LLM Interaction

AI assistants query fitness data through natural language:

| Request | Tools Used |
|---------|------------|
| "Calculate my daily nutrition needs for marathon training" | `calculate_daily_nutrition`, `calculate_nutrient_timing`, `search_foods` |
| "Get my last 10 activities and analyze training load" | `get_activities`, `analyze_training_load`, `calculate_daily_nutrition` |
| "Compare my three longest runs this month" | `get_activities`, `compare_activities`, `analyze_performance_trends` |
| "Analyze this meal: 150g chicken, 200g rice, 100g broccoli" | `analyze_meal_nutrition`, `get_food_details` |
| "Do I need a recovery day based on my training load?" | `analyze_training_load`, `get_activities`, `generate_recommendations` |

## Quick Start

```bash
git clone https://github.com/Async-IO/pierre_mcp_server.git
cd pierre_mcp_server
cp .envrc.example .envrc  # edit with your settings
direnv allow              # or: source .envrc
./bin/setup-and-start.sh  # complete setup: fresh DB, admin user, server start
```

Server starts on `http://localhost:8081`. See [Getting Started](docs/getting-started.md) for detailed setup.

## MCP Client Configuration

Add to Claude Desktop config (`~/Library/Application Support/Claude/claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "pierre-fitness": {
      "command": "npx",
      "args": ["-y", "pierre-mcp-client@next", "--server", "http://localhost:8081"]
    }
  }
}
```

The SDK handles OAuth 2.0 authentication automatically. See [SDK Documentation](sdk/README.md).

## Available MCP Tools

45 tools organized in 8 categories:

| Category | Tools | Description |
|----------|-------|-------------|
| **Core Fitness** | 9 | Activities, athlete profile, provider connections |
| **Goals** | 4 | Goal setting, suggestions, feasibility, progress |
| **Analysis** | 8 | Metrics, trends, patterns, recommendations |
| **Sleep & Recovery** | 5 | Sleep quality, recovery score, rest recommendations |
| **Nutrition** | 5 | BMR/TDEE, macros, USDA food search, meal analysis |
| **Configuration** | 7 | User settings, training zones, profiles |
| **Fitness Config** | 4 | Fitness parameters, thresholds |
| **OAuth** | 5 | Notifications, connection management |

Full tool reference: `src/protocols/universal/tool_registry.rs`

## Server Management

```bash
./bin/setup-and-start.sh  # complete setup: fresh DB, admin user, server start
./bin/start-server.sh     # start backend only (loads .envrc)
./bin/stop-server.sh      # stop backend
./bin/start-frontend.sh   # start dashboard (http://localhost:5173)
```

Options for `setup-and-start.sh`:
- `--skip-fresh-start` - preserve existing database
- `--run-tests` - run workflow tests after startup
- `--admin-email EMAIL` - custom admin email
- `--admin-password PWD` - custom admin password

## User Portal Dashboard

Web-based dashboard for users and administrators at `http://localhost:5173`.

### Features
- **Role-Based Access**: super_admin, admin, user roles with permission hierarchy
- **User Registration**: Self-registration with admin approval workflow
- **API Key Management**: Create, view, deactivate API keys
- **MCP Tokens**: Generate tokens for Claude Desktop and AI assistants
- **Usage Analytics**: Request patterns, tool usage charts
- **Super Admin Impersonation**: View dashboard as any user for support

### User Roles

| Role | Capabilities |
|------|--------------|
| **User** | Own API keys, MCP tokens, analytics |
| **Admin** | + User approval, all users analytics |
| **Super Admin** | + Impersonation, admin tokens, system config |

### First Admin Setup

```bash
cargo run --bin admin-setup -- create-admin-user \
  --email admin@example.com \
  --password SecurePassword123 \
  --super-admin
```

See [Frontend Documentation](frontend/README.md) for detailed dashboard documentation.

## Documentation

### Reference
- [Getting Started](docs/getting-started.md) - installation, configuration, first run
- [Architecture](docs/architecture.md) - system design, components, request flow
- [Protocols](docs/protocols.md) - MCP, OAuth2, A2A, REST
- [Authentication](docs/authentication.md) - JWT, API keys, OAuth2 flows
- [Configuration](docs/configuration.md) - environment variables, algorithms

### Development
- [Development Guide](docs/development.md) - workflow, dashboard, testing
- [Scripts Reference](scripts/README.md) - 30+ development scripts
- [CI/CD](docs/ci-cd.md) - GitHub Actions, pipelines
- [Contributing](docs/contributing.md) - code standards, PR workflow

### Learning
- [Tutorial (25 chapters)](docs/tutorial-table-of-contents.md) - comprehensive Rust learning path

### Components
- [SDK](sdk/README.md) - TypeScript client for MCP integration
- [Frontend](frontend/README.md) - React dashboard

### Methodology
- [Intelligence](docs/intelligence-methodology.md) - sports science formulas
- [Nutrition](docs/nutrition-methodology.md) - dietary calculations

## Testing

```bash
cargo test                        # all tests
./scripts/lint-and-test.sh        # full CI suite
./scripts/smoke-test.sh           # quick validation (~3 min)
```

See [Testing Documentation](docs/testing.md).

## Contributing

See [Contributing Guide](docs/contributing.md).

## License

Dual-licensed under [Apache 2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT).
