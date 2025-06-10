# Pierre MCP Server

A comprehensive MCP (Model Context Protocol) server for fitness data analysis. Provides secure access to fitness data from multiple providers (Strava, Fitbit) through Claude and other AI assistants.

## LLM Prompt Examples

Once connected to Claude or another AI assistant, you can use natural language prompts to analyze your fitness data:

### 🏃 Running Analysis
```
What was my longest run this year?

Analyze my running pace trends over the last 3 months.

How many miles did I run in total last month?

What's my average weekly running distance?

Find my fastest 5K time this year.
```

### 🚴 Cross-Training Analysis
```
Compare my cycling vs running activities this month.

What's my most active day of the week?

Show me my heart rate zones during my last 5 workouts.

How has my fitness improved over the last 6 months?

What's my longest consecutive streak of workouts?
```

### 📊 Fitness Insights
```
Create a summary of my fitness goals progress.

What activities burn the most calories for me?

Analyze my workout patterns and suggest improvements.

How does my Strava data compare to my Fitbit data?

What's my average elevation gain per week?

Analyze how weather conditions affect my running performance.

Show me activities where I performed well despite bad weather.
```

### 🎯 Goal Tracking
```
How close am I to running 1000 miles this year?

Track my progress toward my weekly activity goals.

What's my personal best for each activity type?

Show me days where I exceeded 10,000 steps.

Find patterns in my rest days vs active days.
```

### 📈 Advanced Analysis
```
Correlate my workout intensity with my recovery time.

What's the optimal workout frequency based on my data?

Analyze seasonal patterns in my activity levels.

Compare my performance before and after equipment changes.

Identify my most and least consistent months for training.

How do weather conditions correlate with my workout performance?

Find patterns between temperature and my running pace.
```

## Features

- **Multi-Provider Support**: Strava and Fitbit integration with unified API
- **Enhanced Security**: OAuth2 authentication with PKCE (Proof Key for Code Exchange)
- **Comprehensive Data Access**: Activities, athlete profiles, and aggregated statistics
- **Intelligent Weather Integration**: Real-time and historical weather analysis with contextual insights
- **Activity Intelligence**: AI-powered activity analysis with performance metrics and trends
- **MCP Protocol Compliance**: Works seamlessly with Claude and GitHub Copilot
- **Extensible Design**: Easy to add new fitness providers in the future
- **Production Ready**: Comprehensive testing and clean error handling

## Installation

```bash
cargo build --release
```

## OAuth2 Setup

### Strava

1. Create a Strava application at https://www.strava.com/settings/api
2. Note your Client ID and Client Secret
3. Run the auth setup tool:

```bash
cargo run --bin auth-setup -- strava \
  --client-id YOUR_CLIENT_ID \
  --client-secret YOUR_CLIENT_SECRET
```

4. Follow the browser prompts to authorize the application
5. The tool will save your tokens to the config file

### Fitbit

1. Create a Fitbit application at https://dev.fitbit.com/apps/new
   - **Application Type**: Personal
   - **OAuth 2.0 Application Type**: Confidential
   - **Redirect URL**: `http://localhost:8080/callback` (or your callback URL)
   - **Default Access Type**: Read Only
2. Note your Client ID and Client Secret
3. Run the auth setup tool:

```bash
cargo run --bin auth-setup -- fitbit \
  --client-id YOUR_CLIENT_ID \
  --client-secret YOUR_CLIENT_SECRET
```

4. Follow the browser prompts to authorize the application
5. The tool will save your tokens to the config file

**Note**: Fitbit requires explicit scopes. The server requests `activity`, `profile`, and `sleep` permissions.

## Weather Integration

The server includes comprehensive weather integration that automatically enhances activity analysis with contextual weather data.

### Features

- ✅ **Real-time Weather**: Current weather data from OpenWeatherMap
- ✅ **Historical Weather**: Historical weather data for past activities (with subscription)
- ✅ **GPS-Based**: Extracts coordinates from activity start locations
- ✅ **Smart Fallback**: Intelligent mock weather when API unavailable
- ✅ **Activity Intelligence**: Weather context in activity summaries
- ✅ **Impact Analysis**: Weather difficulty and performance adjustments

### Setup (Optional)

Weather integration works out-of-the-box with realistic mock weather patterns. For real weather data:

1. **Get OpenWeatherMap API Key** (free tier available)
   - Visit https://openweathermap.org/api
   - Sign up for free account
   - Copy your API key

2. **Set Environment Variable**
   ```bash
   export OPENWEATHER_API_KEY="your_api_key_here"
   ```

3. **Configure Settings** (optional)
   Edit `fitness_config.toml`:
   ```toml
   [weather_api]
   provider = "openweathermap"
   enabled = true
   cache_duration_hours = 24
   fallback_to_mock = true
   ```

### Weather Intelligence Examples

With weather integration, activity analysis includes contextual insights:

```json
{
  "summary": "Morning run in the rain with moderate intensity",
  "contextual_factors": {
    "weather": {
      "temperature_celsius": 15.2,
      "humidity_percentage": 85.0,
      "wind_speed_kmh": 12.5,
      "conditions": "rain"
    },
    "time_of_day": "morning"
  }
}
```

### Weather Features

| Feature | Free Tier | Paid Tier |
|---------|-----------|-----------|
| **Mock Weather** | ✅ Realistic patterns | ✅ Available |
| **Current Weather** | ✅ Real-time data | ✅ Real-time data |
| **Historical Weather** | 🎭 Mock fallback | ✅ Real historical data |
| **API Calls** | 1,000/day free | Unlimited with subscription |
| **Production Ready** | ✅ Zero costs | ✅ Precise data |

### Testing Weather Integration

```bash
# Test weather system
cargo run --bin test-weather-integration

# Diagnose API setup
cargo run --bin diagnose-weather-api
```

## Configuration

The server supports multiple configuration methods:

### Using direnv (.envrc):
```bash
# Copy the example file
cp .envrc.example .envrc

# Edit with your credentials
vim .envrc

# Allow direnv to load the file
direnv allow
```

### Using .env file:
```env
# Strava Configuration
STRAVA_CLIENT_ID=your_strava_client_id
STRAVA_CLIENT_SECRET=your_strava_client_secret
STRAVA_ACCESS_TOKEN=your_strava_access_token
STRAVA_REFRESH_TOKEN=your_strava_refresh_token

# Fitbit Configuration
FITBIT_CLIENT_ID=your_fitbit_client_id
FITBIT_CLIENT_SECRET=your_fitbit_client_secret
FITBIT_ACCESS_TOKEN=your_fitbit_access_token
FITBIT_REFRESH_TOKEN=your_fitbit_refresh_token

# Weather Configuration (optional)
OPENWEATHER_API_KEY=your_openweather_api_key
```

### Using config.toml:
```toml
[providers.strava]
auth_type = "oauth2"
client_id = "your_strava_client_id"
client_secret = "your_strava_client_secret"
access_token = "your_strava_access_token"
refresh_token = "your_strava_refresh_token"

[providers.fitbit]
auth_type = "oauth2"
client_id = "your_fitbit_client_id"
client_secret = "your_fitbit_client_secret"
access_token = "your_fitbit_access_token"
refresh_token = "your_fitbit_refresh_token"
```

## Usage

```bash
# Run with default settings
cargo run

# Run with custom port
cargo run -- --port 9000

# Run with custom config file
cargo run -- --config /path/to/config.toml
```

## MCP Tools

The server exposes the following tools for all supported providers:

- `get_activities`: Fetch fitness activities from a provider (supports pagination with limit/offset)
  - **Providers**: `strava`, `fitbit`
  - **Strava**: Uses activity list API with pagination
  - **Fitbit**: Uses date-based activity queries (last 30 days by default)
- `get_athlete`: Get athlete profile information
  - **Strava**: Returns detailed athlete profile with avatar
  - **Fitbit**: Returns user profile with display name and avatar
- `get_stats`: Get aggregated statistics
  - **Strava**: Uses athlete stats API with activity-based fallback
  - **Fitbit**: Uses lifetime stats API with floor-to-elevation conversion
- `get_activity_intelligence`: Generate AI-powered activity analysis with weather context
  - **Features**: Performance metrics, zone distribution, weather impact, personal records
  - **Weather**: Automatic GPS-based weather retrieval with intelligent fallback
  - **Intelligence**: Natural language summaries with contextual insights

### Example Usage

```bash
# Test the server with example queries
cargo run --bin find-2025-longest-run
cargo run --bin find-2024-longest-run
cargo run --bin find-consecutive-10k-runs

# Example MCP tool calls:
# {"method": "tools/call", "params": {"name": "get_activities", "arguments": {"provider": "strava", "limit": 10}}}
# {"method": "tools/call", "params": {"name": "get_activities", "arguments": {"provider": "fitbit", "limit": 20}}}
# {"method": "tools/call", "params": {"name": "get_athlete", "arguments": {"provider": "strava"}}}
# {"method": "tools/call", "params": {"name": "get_activity_intelligence", "arguments": {"provider": "strava", "activity_id": "12345", "include_weather": true}}}
```

## Adding to Claude or GitHub Copilot

Add to your MCP configuration:

```json
{
  "mcpServers": {
    "pierre-fitness": {
      "command": "path/to/pierre-mcp-server",
      "args": ["--port", "8080"]
    }
  }
}
```

Or for development:

```json
{
  "mcpServers": {
    "pierre-fitness-dev": {
      "command": "cargo",
      "args": ["run", "--", "--port", "8080"],
      "cwd": "/path/to/pierre_mcp_server"
    }
  }
}
```

## Development Status

### 🎯 Recent Testing (June 2025)
- ✅ Successfully connected to live MCP server
- ✅ Retrieved 500+ activities with pagination
- ✅ Found 68 runs in 2025, identified longest: 12.59km trail run
- ✅ All 47 tests passing with clean compilation
- ✅ Strava OAuth2 integration fully operational

### ✅ Completed
- [x] Core MCP server implementation with JSON-RPC over TCP
- [x] Strava provider with full OAuth2 authentication and PKCE security
- [x] Fitbit provider with full OAuth2 authentication and PKCE security
- [x] Multi-provider architecture with unified API
- [x] Configuration management (file-based and environment variables)
- [x] Comprehensive data models (Activity, Athlete, Stats, PersonalRecord)
- [x] **Weather Integration**: Real-time and historical weather analysis
- [x] **Activity Intelligence**: AI-powered activity analysis with performance metrics
- [x] **Smart Configuration**: Externalized sport type configuration (35+ activities)
- [x] Unit tests for all core modules (21 tests)
- [x] Integration tests for MCP server and providers (18 tests)
- [x] End-to-end workflow tests (5 tests)
- [x] Weather integration tests and diagnostics
- [x] Example client implementations (find-2024-longest-run, find-2025-longest-run, find-consecutive-10k-runs)
- [x] Comprehensive test coverage (50+ tests passing)
- [x] Clean compilation with no warnings
- [x] Dual MIT/Apache 2.0 licensing
- [x] Complete API documentation
- [x] OAuth2 setup tooling with web callback
- [x] PKCE implementation for enhanced OAuth2 security
- [x] Type-safe MCP schema definitions (no hardcoded JSON)
- [x] Complete JSON externalization for protocol compliance

### 🚀 Roadmap

Track our progress and upcoming features on [GitHub Issues](https://github.com/jfarcand/pierre_mcp_server/issues).

Key areas of development:
- **Additional Providers**: Polar Flow, Wahoo, TrainingPeaks, and more
- **Enhanced Features**: Webhooks, GPS tracks, heart rate analysis
- **Performance**: Connection pooling, caching, rate limiting
- **Developer Experience**: Docker support, CI/CD, comprehensive docs
- **Security**: Token encryption, audit logging, secure rotation

**Note**: Garmin Connect and RunKeeper providers were removed due to API accessibility issues. The infrastructure remains ready for future providers.

## Contributing

We welcome contributions! Please see our [contribution guidelines](CONTRIBUTING.md) for details.

### Quick Start for Contributors

1. **Fork and clone the repository**
   ```bash
   git clone https://github.com/jfarcand/pierre_mcp_server.git
   cd pierre_mcp_server
   ```

2. **Set up development environment**
   ```bash
   # Install Rust (if not already installed)
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

3. **Run tests**
   ```bash
   # Run all tests
   cargo test
   
   # Run tests with output
   cargo test -- --nocapture
   ```

4. **Development workflow**
   ```bash
   # Format code
   cargo fmt
   
   # Lint code
   cargo clippy
   ```

### Adding a New Provider

1. Create a new file in `src/providers/your_provider.rs`
2. Implement the `FitnessProvider` trait
3. Add OAuth2 or API key authentication
4. Update the provider factory in `src/providers/mod.rs`
5. Add comprehensive tests in `tests/provider_integration.rs`
6. Update configuration examples in README

### Code Style

- Follow standard formatting (`cargo fmt`)
- Use clippy for linting (`cargo clippy`)
- Write comprehensive tests for new features
- Document public APIs with comments
- Follow the existing error handling patterns

### Commit Guidelines

- Use conventional commit format: `feat:`, `fix:`, `docs:`, etc.
- Write clear, descriptive commit messages
- Keep commits focused and atomic
- Reference issues in commit messages when applicable

## License

This project is dual-licensed under either of:

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
* MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
