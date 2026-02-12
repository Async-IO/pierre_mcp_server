<div align="center">
  <img src="templates/pierre-logo.svg" width="150" height="150" alt="Pierre Fitness Platform Logo">
  <h1>Pierre Fitness Platform</h1>
</div>

[![Backend CI](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/ci.yml/badge.svg)](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/ci.yml)
[![Cross-Platform](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/cross-platform.yml/badge.svg)](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/cross-platform.yml)
[![Frontend Tests](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/frontend-tests.yml/badge.svg)](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/frontend-tests.yml)
[![SDK Tests](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/sdk-tests.yml/badge.svg)](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/sdk-tests.yml)
[![MCP Compliance](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/mcp-compliance.yml/badge.svg)](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/mcp-compliance.yml)
[![Mobile Tests](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/mobile-tests.yml/badge.svg)](https://github.com/Async-IO/pierre_mcp_server/actions/workflows/mobile-tests.yml)

Pierre Fitness Platform connects AI assistants to fitness data from Strava, Garmin, Fitbit, WHOOP, COROS, and Terra (150+ wearables). Implements Model Context Protocol (MCP), A2A protocol, OAuth 2.0, and REST APIs for Claude, ChatGPT, and other AI assistants.

## Intelligence System

Sports science-based fitness analysis including training load management, race predictions, sleep and recovery scoring, nutrition planning, and pattern detection.

See [Intelligence Methodology](https://async-io.github.io/pierre_mcp_server/intelligence-methodology.html), [Nutrition Methodology](https://async-io.github.io/pierre_mcp_server/nutrition-methodology.html), and [Mobility Methodology](https://async-io.github.io/pierre_mcp_server/mobility-methodology.html) for details.

## Features

- **MCP Protocol**: JSON-RPC 2.0 for AI assistant integration
- **A2A Protocol**: Agent-to-agent communication
- **OAuth 2.0 Server**: RFC 7591 dynamic client registration
- **53 MCP Tools**: Activities, goals, analysis, sleep, recovery, nutrition, recipes, mobility, configuration
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
| COROS | `provider-coros` | Activities, Sleep, Recovery |
| Terra | `provider-terra` | 150+ wearables, Activities, Sleep, Health |
| Synthetic | `provider-synthetic` | Development/Testing |

Build with specific providers:
```bash
cargo build --release                                                    # all providers
cargo build --release --no-default-features --features "sqlite,provider-strava"  # strava only
```

See [Build Configuration](https://async-io.github.io/pierre_mcp_server/build.html) for provider architecture details.

## Modular Architecture

Pierre uses compile-time feature flags for modular deployments. Build only what you need.

### Server Profiles

Pre-configured bundles for common deployment scenarios:

| Profile | Description | Binary Size |
|---------|-------------|-------------|
| `server-full` | All protocols, transports, clients (default) | ~50MB |
| `server-mcp-stdio` | MCP protocol + stdio transport (desktop clients) | ~35MB |
| `server-mcp-bridge` | MCP + A2A protocols, web transports | ~40MB |
| `server-mobile-backend` | REST + MCP, mobile client routes | ~42MB |
| `server-saas-full` | REST + MCP, web + admin clients | ~45MB |

```bash
# Build for desktop MCP clients (minimal)
cargo build --release --no-default-features --features "sqlite,server-mcp-stdio"

# Build for SaaS deployment
cargo build --release --no-default-features --features "postgresql,server-saas-full"
```

### Feature Categories

| Category | Features | Description |
|----------|----------|-------------|
| **Protocols** | `protocol-rest`, `protocol-mcp`, `protocol-a2a` | API protocols |
| **Transports** | `transport-http`, `transport-websocket`, `transport-sse`, `transport-stdio` | Communication layers |
| **Clients** | `client-web`, `client-admin`, `client-mobile` | Route groups |
| **Tools** | `tools-fitness-core`, `tools-wellness`, `tools-all` | MCP tool categories |

See [Build Configuration](https://async-io.github.io/pierre_mcp_server/build.html) for detailed feature documentation.

## What You Can Ask

- "Calculate my daily nutrition needs for marathon training"
- "Analyze my training load - do I need a recovery day?"
- "Compare my three longest runs this month"
- "Analyze this meal: 150g chicken, 200g rice, 100g broccoli"
- "What's my predicted marathon time based on recent runs?"

See [Tools Reference](https://async-io.github.io/pierre_mcp_server/tools-reference.html) for the 53 available MCP tools.

## Quick Start

```bash
git clone https://github.com/Async-IO/pierre_mcp_server.git
cd pierre_mcp_server
cp .envrc.example .envrc  # edit with your settings
direnv allow              # or: source .envrc

# Full dev environment: reset DB, seed data, start all 3 servers
./bin/setup-db-with-seeds-and-oauth-and-start-servers.sh
```

This single command:
- Resets database with fresh migrations
- Seeds admin, AI coaches, demo users, test data, mobility data
- Starts Pierre server (8081), web frontend (3000), Expo mobile (8082)
- Displays all credentials, tokens, and log file paths

See [Getting Started](https://async-io.github.io/pierre_mcp_server/getting-started.html) for detailed setup.

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

53 tools organized in 9 categories:

| Category | Tools | Description |
|----------|-------|-------------|
| **Core Fitness** | 6 | Activities, athlete profile, provider connections |
| **Goals** | 4 | Goal setting, suggestions, feasibility, progress |
| **Analysis** | 10 | Metrics, trends, patterns, predictions, recommendations |
| **Sleep & Recovery** | 5 | Sleep quality, recovery score, rest recommendations |
| **Nutrition** | 5 | BMR/TDEE, macros, USDA food search, meal analysis |
| **Recipes** | 7 | Training-aware meal planning and recipe storage |
| **Mobility** | 6 | Stretching exercises, yoga poses, recovery sequences |
| **Configuration** | 6 | User settings, training zones, profiles |
| **Fitness Config** | 4 | Fitness parameters, thresholds |

Full tool reference: [Tools Reference](https://async-io.github.io/pierre_mcp_server/tools-reference.html)

## Server Management

```bash
# Full development setup (recommended for first run or fresh start)
./bin/setup-db-with-seeds-and-oauth-and-start-servers.sh

# Individual services
./bin/start-server.sh     # start backend only (port 8081)
./bin/stop-server.sh      # stop backend
./bin/start-frontend.sh   # start web dashboard (port 3000)
```

The full setup script does everything:
1. Resets database with fresh migrations
2. Seeds admin user, AI coaches, demo users, test data, mobility data
3. Starts Pierre server, web frontend, and Expo mobile
4. Displays all credentials, tokens, and log file paths

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
cargo run --bin pierre-cli -- user create \
  --email admin@example.com \
  --password SecurePassword123 \
  --super-admin
```

See [Frontend Documentation](frontend/README.md) for detailed dashboard documentation.

## Mobile App

React Native mobile app for iOS and Android with conversational AI interface.

### Features
- **AI Chat Interface**: Conversational UI with markdown rendering and real-time streaming
- **Fitness Provider Integration**: Connect to Strava, Garmin, Fitbit, WHOOP, COROS via OAuth
- **Activity Tracking**: View and analyze your fitness activities
- **Training Insights**: Get AI-powered training recommendations

### Quick Start

```bash
cd frontend-mobile
bun install
bun start   # Start Expo development server
bun run ios # Run on iOS Simulator
```

See [Mobile App README](frontend-mobile/README.md) and [Mobile Development Guide](https://async-io.github.io/pierre_mcp_server/mobile-development.html).

## AI Coaches

Pierre includes an AI coaching system with 9 default coaching personas and support for user-created personalized coaches.

### Default Coaches

The system includes 9 AI coaching personas across 5 categories:

| Category | Icon | Coaches |
|----------|------|---------|
| **Training** | 🏃 | Endurance Coach, Speed Coach |
| **Nutrition** | 🥗 | Sports Nutritionist, Hydration Specialist |
| **Recovery** | 😴 | Recovery Specialist, Sleep Coach |
| **Recipes** | 👨‍🍳 | Performance Chef, Meal Prep Expert |
| **Analysis** | 📊 | Data Analyst |

Default coaches are seeded automatically by `./bin/setup-and-start.sh` and are visible to all users.

### Personalized Coaches

Users can create their own AI coaches with custom:
- Name and personality
- System prompts and behavior
- Category assignment
- Avatar customization

User-created coaches appear in a "Personalized" section above system coaches and are private to each user.

### Coach Seeder

To seed or refresh the default coaches:

```bash
cargo run --bin seed-coaches
```

This creates the 9 default AI coaching personas if they don't already exist.

## Documentation

### Reference
- [Getting Started](https://async-io.github.io/pierre_mcp_server/getting-started.html) - installation, configuration, first run
- [Architecture](https://async-io.github.io/pierre_mcp_server/architecture.html) - system design, components, request flow
- [Protocols](https://async-io.github.io/pierre_mcp_server/protocols.html) - MCP, OAuth2, A2A, REST
- [Authentication](https://async-io.github.io/pierre_mcp_server/authentication.html) - JWT, API keys, OAuth2 flows
- [Configuration](https://async-io.github.io/pierre_mcp_server/configuration.html) - environment variables, algorithms

### Development
- [Development Guide](https://async-io.github.io/pierre_mcp_server/development.html) - workflow, dashboard, testing
- [Scripts Reference](scripts/README.md) - 23 development scripts across 6 subdirectories
- [CI/CD](https://async-io.github.io/pierre_mcp_server/ci-cd.html) - GitHub Actions, pipelines
- [Release Guide](https://async-io.github.io/pierre_mcp_server/release_how_to.html) - releasing server and SDK to npm
- [Contributing](CONTRIBUTING.md) - code standards, PR workflow

### Components
- [SDK](sdk/README.md) - TypeScript client for MCP integration
- [Frontend](frontend/README.md) - React dashboard
- [Mobile](frontend-mobile/README.md) - React Native mobile app
- [Mobile Development](https://async-io.github.io/pierre_mcp_server/mobile-development.html) - mobile dev setup guide

### Methodology
- [Intelligence](https://async-io.github.io/pierre_mcp_server/intelligence-methodology.html) - sports science formulas
- [Nutrition](https://async-io.github.io/pierre_mcp_server/nutrition-methodology.html) - dietary calculations
- [Mobility](https://async-io.github.io/pierre_mcp_server/mobility-methodology.html) - stretching and yoga sequences

## Testing

```bash
cargo test                        # all tests
./scripts/ci/lint-and-test.sh     # full CI suite
./scripts/ci/pre-push-validate.sh # tiered validation before push
```

See [Testing Documentation](https://async-io.github.io/pierre_mcp_server/testing.html).

## Development Workflow

### Before Committing

```bash
# 1. Format code
cargo fmt

# 2. Architectural validation (includes security checks in CI)
./scripts/ci/architectural-validation.sh

# 3. Clippy (Cargo.toml defines all lint levels)
cargo clippy -p pierre_mcp_server --all-targets

# 4. Run relevant tests (always specify --test for speed)
cargo test --test <test_file> <test_pattern> -- --nocapture
```

### Security Skills (Before Pushing Security-Sensitive Changes)

Run these when modifying auth, OAuth, admin, database, or multi-tenant code:

| Skill | Command | What It Checks |
|-------|---------|----------------|
| **Security Review** | `./scripts/ci/security-review.sh` | Authorization boundaries, tenant isolation, logging hygiene, SQL injection, XSS |
| **Input Validation** | `./scripts/ci/check-input-validation.sh` | Division-by-zero, pagination bounds, cache key completeness, numeric ranges |
| **Architecture** | `./scripts/ci/architectural-validation.sh` | Placeholder detection, error handling, algorithm DI, unsafe code, secret patterns |

These scripts run automatically in CI via the `code-quality` job (gates all other jobs). Run them locally when:
- Adding/modifying API endpoints or MCP tools
- Changing database queries or cache operations
- Modifying OAuth flows or authentication logic
- Adding math operations on user-supplied input

### Before Pushing

```bash
# 1. Enable git hooks (once per clone)
git config core.hooksPath .githooks

# 2. Run validation (creates marker valid for 15 min)
./scripts/ci/pre-push-validate.sh

# 3. Push (hook checks for valid marker)
git push
```

The pre-push hook blocks pushes without a valid marker. This decouples test execution from the push to avoid SSH timeout issues.

## Contributing

See [Contributing Guide](CONTRIBUTING.md).

## License

Dual-licensed under [Apache 2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT).
