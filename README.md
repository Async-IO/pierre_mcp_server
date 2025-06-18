# Pierre MCP Server

[![CI](https://github.com/jfarcand/pierre_mcp_server/actions/workflows/ci.yml/badge.svg)](https://github.com/jfarcand/pierre_mcp_server/actions/workflows/ci.yml)
[![Frontend Tests](https://github.com/jfarcand/pierre_mcp_server/actions/workflows/frontend-tests.yml/badge.svg)](https://github.com/jfarcand/pierre_mcp_server/actions/workflows/frontend-tests.yml)

A multi-tenant fitness data platform providing secure B2B API access to fitness data from multiple providers (Strava, Fitbit) through the Model Context Protocol (MCP), A2A (Agent-to-Agent) Protocol, and REST APIs. Built for developers and AI assistants, Pierre MCP Server features enterprise-grade API key management with tiered rate limiting, trial keys with automatic expiration, OAuth integration, real-time analytics, comprehensive activity intelligence, and support for AI agent communication.

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

## Real-World Connection Flows

Pierre MCP Server provides multiple ways for users and AI agents to connect and access fitness data, all built on secure OAuth2 flows with real-time data access.

### 🤖 AI Assistant Integration (MCP Protocol)

**For users connecting AI assistants like Claude Desktop or GitHub Copilot:**

#### Complete User Journey

1. **Account Setup**
   ```bash
   # User registers on your Pierre instance
   curl -X POST https://your-pierre-server.com/auth/register \
     -H "Content-Type: application/json" \
     -d '{
       "email": "user@example.com",
       "password": "secure_password",
       "display_name": "John Runner"
     }'
   ```

2. **Strava Connection**
   ```bash
   # User connects their Strava account via OAuth
   curl -X GET "https://your-pierre-server.com/oauth/auth/strava/{user_id}" \
     -H "Authorization: Bearer USER_JWT_TOKEN"
   # Returns: Real Strava OAuth URL for browser authorization
   ```

3. **AI Assistant Configuration**
   ```json
   // ~/.claude/claude_desktop_config.json
   {
     "mcpServers": {
       "pierre-fitness": {
         "command": "mcp-client",
         "args": ["--server", "wss://your-pierre-server.com:8080"],
         "env": {
           "JWT_TOKEN": "your_jwt_token_here"
         }
       }
     }
   }
   ```

4. **Natural Language Queries**
   ```
   User → Claude: "What was my longest run this year and where did I run it?"
   Claude → Pierre: get_activities + get_activity_intelligence
   Pierre → Strava: Fetch real activity data with location
   Claude → User: "Your longest run was 21.5km in Saint-Hippolyte, Québec on March 15th..."
   ```

### 🔗 Developer Integration (A2A Protocol)

**For applications and AI agents connecting programmatically:**

#### A2A Client Registration
```bash
# Developer registers their application
curl -X POST https://your-pierre-server.com/a2a/clients \
  -H "Authorization: Bearer USER_JWT_TOKEN" \
  -d '{
    "name": "FitnessCoach AI",
    "description": "AI-powered fitness coaching application",
    "capabilities": ["fitness-analysis", "goal-tracking", "performance-prediction"],
    "redirect_uris": ["https://myapp.com/oauth/callback"],
    "contact_email": "developer@myapp.com"
  }'
# Returns: client_id, client_secret, api_key
```

#### Real-Time Data Access
```bash
# Application accesses user's fitness data
curl -X POST https://your-pierre-server.com/a2a/execute \
  -H "Authorization: Bearer A2A_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools.execute",
    "id": 1,
    "params": {
      "tool_name": "get_activities",
      "parameters": {
        "provider": "strava",
        "limit": 20
      }
    }
  }'
```

#### Activity Intelligence with Context
```bash
# Get AI-powered insights with weather and location
curl -X POST https://your-pierre-server.com/a2a/execute \
  -H "Authorization: Bearer A2A_API_KEY" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools.execute",
    "params": {
      "tool_name": "get_activity_intelligence",
      "parameters": {
        "provider": "strava",
        "activity_id": "14816735354",
        "include_weather": true,
        "include_location": true
      }
    }
  }'
```

### 🔄 Real OAuth Flow Example

**Complete end-to-end example with actual Strava data:**

```bash
# 1. User registration
curl -X POST http://localhost:8081/auth/register \
  -d '{"email": "athlete@example.com", "password": "secure123"}'
# Returns: {"user_id": "ca33ad77-728b-4e6d-83c5-d2878a69a9dc"}

# 2. Login for JWT token
curl -X POST http://localhost:8081/auth/login \
  -d '{"email": "athlete@example.com", "password": "secure123"}'
# Returns: {"jwt_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9..."}

# 3. Generate Strava OAuth URL
curl -X GET "http://localhost:8081/oauth/auth/strava/ca33ad77-728b-4e6d-83c5-d2878a69a9dc" \
  -H "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9..."
# Returns: {"authorization_url": "https://www.strava.com/oauth/authorize?client_id=163846&..."}

# 4. User visits URL in browser → Strava authorization → automatic callback processing

# 5. Access real fitness data
curl -X POST http://localhost:8081/a2a/execute \
  -H "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9..." \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools.execute",
    "params": {
      "tool_name": "get_athlete",
      "parameters": {"provider": "strava"}
    }
  }'
# Returns: Real Strava athlete data with profile, stats, and activity access
```

## 📚 Documentation

Complete documentation is organized in the `docs/` directory:

### 📋 [Setup & Installation](docs/SETUP.md)
- Local development setup
- OAuth2 configuration for Strava and Fitbit
- Environment variables and configuration files
- Docker deployment options

### 🛠️ [MCP Tools Reference](docs/TOOLS.md)
- Complete reference for all 21 fitness analysis tools
- Tool categories: Data Access, Intelligence, Connections, Goals, Analytics
- Real-world examples with live Strava data
- API usage patterns and best practices

### 🌦️ [Weather Integration](docs/WEATHER.md)
- OpenWeatherMap API integration
- Mock weather system for development
- Weather-enhanced activity intelligence
- Setup and configuration guide

### 🔑 [API Reference](docs/API_REFERENCE.md)
- B2B API platform features
- API key management and tiers
- Enterprise security features
- Usage analytics and monitoring

### 🚀 [Deployment Guide](docs/DEPLOYMENT.md)
- Production deployment scenarios
- Docker and Kubernetes configurations
- Cloud deployment (AWS, GCP, Azure)
- Monitoring and observability

## Features

- **Multi-Provider Support**: Strava and Fitbit integration with unified API
- **Enhanced Security**: OAuth2 authentication with PKCE (Proof Key for Code Exchange)
- **Comprehensive Data Access**: Activities, athlete profiles, and aggregated statistics
- **🗺️ Location Intelligence**: GPS-based location detection with trail and region identification
- **🌦️ Intelligent Weather Integration**: Real-time and historical weather analysis with contextual insights
- **🧠 Activity Intelligence**: AI-powered activity analysis with performance metrics, location, and weather context
- **MCP Protocol Compliance**: Works seamlessly with Claude and GitHub Copilot
- **🤖 A2A (Agent-to-Agent) Protocol**: Open protocol for AI agent communication
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

## Quick Start

### Local Development
```bash
# Clone and build
git clone https://github.com/jfarcand/pierre_mcp_server.git
cd pierre_mcp_server
cargo build --release

# Run in single-tenant mode
cargo run --bin pierre-mcp-server -- --single-tenant
```

### Docker Deployment
```bash
# Setup environment
cp .env.example .envrc
# Edit .envrc with your OAuth credentials

# Run with Docker Compose
./docker-compose-with-envrc.sh up
```

### Adding to Claude Desktop
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

## License

This project is dual-licensed under either of:

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
* MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.