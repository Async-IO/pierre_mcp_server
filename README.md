# Pierre MCP Server

A comprehensive MCP (Model Context Protocol) server for fitness data analysis. Provides secure access to fitness data from multiple providers (Strava, Fitbit) through Claude and other AI assistants.

## LLM Prompt Examples

Once connected to Claude or another AI assistant, you can use natural language prompts to analyze your fitness data with comprehensive intelligence including location, weather, and performance context:

### 🏃 Running Analysis
```
What was my longest run this year and where did I run it?

Analyze my running pace trends over the last 3 months with location context.

How many kms did I run in total last month?

Find my fastest 5K time this year and the conditions when I achieved it.

Show me all my runs in Saint-Hippolyte and analyze the terrain impact.

Compare my performance on trails vs road running.

What's my average pace when running in different cities or regions?
```

### 🚴 Cross-Training Analysis
```
Compare my cycling vs running activities this month with location data.

What's my most active day of the week and where do I typically train?

Show me my heart rate zones during my last 5 workouts with weather context.

How has my fitness improved over the last 6 months?

What's my longest consecutive streak of workouts?

Analyze my performance in different locations - where do I perform best?

Find patterns between workout locations and my energy levels.
```

### 🗺️ Location Intelligence
```
Generate Activity Intelligence for my longest run in 2025 with full location context.

Where do I run most frequently and how does location affect my performance?

Analyze my trail running vs road running performance patterns.

Show me activities in Quebec and compare them to other regions.

Find all my runs on mountain trails and analyze elevation impact.

What cities or regions have I trained in this year?

Compare my performance in urban vs rural training locations.

Identify my favorite training routes and analyze why they work well for me.

Show me how different terrains (forest, mountain, city) affect my pace.
```

### 🌦️ Weather & Environmental Impact
```
Analyze how weather conditions affect my running performance.

Show me activities where I performed well despite challenging weather.

Find patterns between temperature and my running pace.

What's my best performance in cold weather vs hot weather?

Analyze how rain, wind, and humidity impact my training.

Show me my most challenging weather conditions and how I adapted.

Compare my performance in different seasons with weather context.

Find correlations between weather patterns and my training consistency.
```

### 📊 Comprehensive Activity Intelligence
```
Generate full Activity Intelligence for my most recent marathon with weather and location.

Analyze my longest bike ride with complete environmental context.

Show me my best performances with weather, location, and heart rate analysis.

Create a detailed analysis of my training in mountainous regions.

Compare my performance in different trail systems or parks.

Analyze how elevation gain correlates with my effort levels across locations.

Show me my most efficient training sessions with full environmental context.

Find patterns between location, weather, and my personal records.
```

### 🎯 Goal Tracking & Performance
```
How close am I to running 1000 miles this year and where have I run them?

Track my progress toward weekly goals with location diversity analysis.

What's my personal best for each activity type and where did I achieve them?

Show me days where I exceeded targets despite challenging conditions.

Find patterns in my rest days vs active days across different locations.

Analyze my consistency across different training environments.

Compare my goal achievement rates in different locations or weather conditions.
```

### 📈 Advanced Intelligence Analysis
```
Correlate workout intensity with recovery time across different locations.

What's the optimal workout frequency based on my data and environmental factors?

Analyze seasonal patterns in my activity levels with location context.

Compare my performance before and after training in new locations.

Identify my most and least consistent training environments.

Show me how location changes affect my adaptation and performance.

Find optimal training conditions based on my historical performance data.

Analyze the relationship between trail difficulty and my fitness improvements.

Create a comprehensive training analysis with weather, location, and performance metrics.
```

### 🧠 AI-Powered Insights
```
Generate intelligent summaries for my recent activities with full context.

Analyze my training patterns and suggest location-based improvements.

Show me how environmental factors influence my training decisions.

Create personalized insights about my optimal training conditions.

Find hidden patterns in my performance across different environments.

Suggest new training locations based on my performance preferences.

Analyze my adaptation to different training environments over time.
```

## Features

- **Multi-Provider Support**: Strava and Fitbit integration with unified API
- **Enhanced Security**: OAuth2 authentication with PKCE (Proof Key for Code Exchange)
- **Comprehensive Data Access**: Activities, athlete profiles, and aggregated statistics
- **🗺️ Location Intelligence**: GPS-based location detection with trail and region identification
  - **Reverse Geocoding**: GPS coordinates → "Saint-Hippolyte, Québec, Canada"
  - **Trail Detection**: Automatic recognition of trails, paths, and routes
  - **Regional Context**: City, region, and country identification for training analysis
  - **Location-Aware Summaries**: "Run in the rain **in Saint-Hippolyte, Québec**"
- **🌦️ Intelligent Weather Integration**: Real-time and historical weather analysis with contextual insights
- **🧠 Activity Intelligence**: AI-powered activity analysis with performance metrics, location, and weather context
  - **Performance Metrics**: Heart rate zones, effort levels, efficiency scores
  - **Environmental Context**: Weather conditions, location impact, terrain analysis
  - **Natural Language Summaries**: Human-readable insights with full context
  - **Personal Records**: Automatic detection with location and weather correlation
- **MCP Protocol Compliance**: Works seamlessly with Claude and GitHub Copilot
- **Extensible Design**: Easy to add new fitness providers in the future
- **Production Ready**: Comprehensive testing and clean error handling

## Architecture

Pierre MCP Server supports two deployment modes:

### 🏠 Single-Tenant Mode (Personal Use)
- **Perfect for individual users** who want to run the server locally
- No authentication required - direct access to your fitness data
- Simple configuration with local config files or environment variables
- Backwards compatible with existing setups

### ☁️ Multi-Tenant Mode (Cloud Deployment)
- **Enterprise-ready** for serving multiple users
- **JWT Authentication** with secure user sessions
- **Encrypted Token Storage** using AES-256-GCM for OAuth tokens at rest
- **SQLite Database** for user management and token storage
- **User Isolation** ensuring data privacy between users
- **Cloud-Ready** for deployment on any cloud provider

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

### Single-Tenant Mode (Personal Use)

```bash
# Run in single-tenant mode (default, backwards compatible)
cargo run --bin pierre-mcp-server -- --single-tenant

# Run with custom port
cargo run --bin pierre-mcp-server -- --single-tenant --port 9000

# Run with custom config file
cargo run --bin pierre-mcp-server -- --single-tenant --config /path/to/config.toml
```

### Multi-Tenant Mode (Cloud Deployment)

```bash
# Run in multi-tenant mode with authentication
cargo run --bin pierre-mcp-server

# Specify database and authentication settings
cargo run --bin pierre-mcp-server -- \
  --database-url "sqlite:./users.db" \
  --token-expiry-hours 24 \
  --port 8080

# Use custom encryption and JWT secret files
cargo run --bin pierre-mcp-server -- \
  --encryption-key-file ./custom-encryption.key \
  --jwt-secret-file ./custom-jwt.secret
```

### Multi-Tenant Authentication Flow

1. **User Registration/Login** (Phase 2 - Coming Soon)
   ```bash
   # Register new user
   curl -X POST http://localhost:8080/auth/register \
     -H "Content-Type: application/json" \
     -d '{"email": "user@example.com", "password": "secure_password"}'

   # Login to get JWT token
   curl -X POST http://localhost:8080/auth/login \
     -H "Content-Type: application/json" \
     -d '{"email": "user@example.com", "password": "secure_password"}'
   ```

2. **Use JWT Token in MCP calls**
   ```json
   {
     "method": "authenticate",
     "params": {
       "jwt_token": "your_jwt_token_here"
     }
   }
   ```

### Security Features

- **Encryption at Rest**: All OAuth tokens encrypted with AES-256-GCM
- **JWT Authentication**: Stateless authentication with configurable expiry
- **User Isolation**: Complete data separation between users
- **Secure Defaults**: Encryption keys auto-generated if not provided
- **No Shared State**: Each user's data completely isolated

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
- `get_activity_intelligence`: Generate AI-powered activity analysis with weather and location context
  - **Parameters**: `include_weather` (bool), `include_location` (bool)
  - **Performance Metrics**: Heart rate zones, effort levels, efficiency scores, personal records
  - **🌦️ Weather Context**: Automatic GPS-based weather retrieval with intelligent fallback
  - **🗺️ Location Intelligence**: Reverse geocoding, trail detection, regional context
  - **Environmental Analysis**: Weather impact, terrain difficulty, location-specific insights
  - **Natural Language**: Comprehensive summaries with full environmental context
  - **Example**: "Run in the rain in Saint-Hippolyte, Québec with very high intensity"

### Example Usage

```bash
# Test the server with example queries
cargo run --bin find-2025-longest-run
cargo run --bin find-2024-longest-run
cargo run --bin find-consecutive-10k-runs

# Test location intelligence features
cargo run --bin test-location-intelligence
cargo run --bin test-intelligence-for-longest-run
cargo run --bin check-longest-run-gps

# Example MCP tool calls:
# {"method": "tools/call", "params": {"name": "get_activities", "arguments": {"provider": "strava", "limit": 10}}}
# {"method": "tools/call", "params": {"name": "get_activities", "arguments": {"provider": "fitbit", "limit": 20}}}
# {"method": "tools/call", "params": {"name": "get_athlete", "arguments": {"provider": "strava"}}}

# Activity Intelligence with full context
# {"method": "tools/call", "params": {"name": "get_activity_intelligence", "arguments": {"provider": "strava", "activity_id": "12345", "include_weather": true, "include_location": true}}}

# Weather-only analysis
# {"method": "tools/call", "params": {"name": "get_activity_intelligence", "arguments": {"provider": "strava", "activity_id": "12345", "include_weather": true, "include_location": false}}}

# Location-only analysis  
# {"method": "tools/call", "params": {"name": "get_activity_intelligence", "arguments": {"provider": "strava", "activity_id": "12345", "include_weather": false, "include_location": true}}}
```

## Adding to Claude or GitHub Copilot

### Single-Tenant Mode Configuration

Add to your MCP configuration:

```json
{
  "mcpServers": {
    "pierre-fitness": {
      "command": "path/to/pierre-mcp-server",
      "args": ["--single-tenant", "--port", "8080"]
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
      "args": ["run", "--bin", "pierre-mcp-server", "--", "--single-tenant", "--port", "8080"],
      "cwd": "/path/to/pierre_mcp_server"
    }
  }
}
```

### Multi-Tenant Mode Configuration

For cloud deployments, connect to your hosted multi-tenant server:

```json
{
  "mcpServers": {
    "pierre-fitness-cloud": {
      "command": "mcp-client",
      "args": ["--url", "https://your-cloud-server.com:8080", "--auth-type", "jwt"]
    }
  }
}
```

## Development Roadmap

### ✅ Phase 1: Multi-Tenant Architecture (Completed)
- ✅ Multi-tenant server with JWT authentication
- ✅ Encrypted token storage with AES-256-GCM
- ✅ User isolation and database management
- ✅ Unified server supporting both single and multi-tenant modes
- ✅ Backwards compatibility for existing users

### 🚧 Phase 2: OAuth Integration & User Onboarding (In Progress)
- 🔄 User registration and login endpoints
- 🔄 OAuth2 flow integration for Strava/Fitbit in multi-tenant mode
- 🔄 Web interface for user onboarding
- 🔄 Token refresh automation
- 🔄 User management dashboard

### 📋 Phase 3: Cloud Deployment Infrastructure
- ⏳ Docker containerization
- ⏳ Kubernetes deployment manifests
- ⏳ Cloud provider templates (AWS, GCP, Azure)
- ⏳ Load balancing and scaling configuration
- ⏳ Monitoring and observability setup

### 📋 Phase 4: Advanced Features
- ⏳ Rate limiting and API quotas
- ⏳ Advanced analytics and reporting
- ⏳ WebSocket support for real-time updates
- ⏳ Plugin system for custom providers
- ⏳ GraphQL API support

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
