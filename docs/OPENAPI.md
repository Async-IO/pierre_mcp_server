# Pierre MCP Fitness API Documentation

## 🚀 Interactive API Documentation

This directory contains the OpenAPI specification and documentation server for the Pierre MCP Fitness API.

### 📖 Quick Start

1. **Start the documentation server:**
   ```bash
   cargo run --bin serve-docs
   ```

2. **View interactive documentation:**
   - Open: http://localhost:3000
   - Swagger UI with all 21 fitness tools
   - Try API calls directly in browser

3. **Access OpenAPI spec:**
   - YAML: http://localhost:3000/openapi.yaml
   - JSON: http://localhost:3000/openapi.json

### 🛠️ What's Included

- **Complete OpenAPI 3.0.3 specification** for all MCP tools
- **Interactive Swagger UI** with custom Pierre MCP branding
- **21 documented tools** across 4 categories:
  - 🔧 **Core Tools** (8): Activities, athlete data, intelligence
  - 📊 **Analytics** (8): Performance analysis, patterns, predictions
  - 🎯 **Goals** (4): Goal setting, tracking, feasibility
  - 🔗 **Connections** (4): Provider OAuth and management

### 📚 Tool Categories

#### Core Tools
- `get_activities` - Retrieve fitness activities
- `get_athlete` - Get athlete profile
- `get_stats` - Fitness statistics
- `get_activity_intelligence` - AI-powered activity insights

#### Advanced Analytics
- `calculate_fitness_score` - Comprehensive fitness scoring
- `analyze_training_load` - Training load analysis
- `detect_patterns` - Pattern recognition in training
- `analyze_performance_trends` - Performance trend analysis
- `generate_recommendations` - Personalized training advice
- `predict_performance` - Future performance prediction

#### Goal Management
- `set_goal` - Create fitness goals
- `track_progress` - Monitor goal progress
- `suggest_goals` - AI-powered goal suggestions
- `analyze_goal_feasibility` - Goal feasibility assessment

#### Provider Connections
- `connect_strava` - Strava OAuth flow
- `connect_fitbit` - Fitbit OAuth flow
- `get_connection_status` - Check provider connections
- `disconnect_provider` - Revoke provider access

### 🔧 Development

#### Custom Documentation Port
```bash
DOCS_PORT=8080 cargo run --bin serve-docs
```

#### Update OpenAPI Spec
Edit `docs/openapi.yaml` and restart the server.

#### Add New Tools
1. Update `openapi.yaml` with new tool definitions
2. Add tool schemas to `x-mcp-tools` section
3. Include examples and parameter documentation

### 🌟 Features

- **MCP Protocol Documentation** - Complete MCP JSON-RPC format
- **Authentication Examples** - JWT bearer token usage
- **Request/Response Examples** - Real API call examples
- **Error Handling** - Comprehensive error code documentation
- **Provider Support** - Multi-provider fitness data access
- **AI-Ready Format** - Optimized for LLM consumption

### 📋 API Information

- **Base URL**: `ws://localhost:8080` (Local Development)
- **Protocol**: MCP (Model Context Protocol) over WebSocket
- **Authentication**: JWT Bearer tokens
- **Format**: JSON-RPC 2.0

### 📄 Example MCP Request

```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "id": 1,
  "auth": "Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "params": {
    "name": "get_activities",
    "arguments": {
      "provider": "strava",
      "limit": 20
    }
  }
}
```

### 📄 Example Response

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": [
    {
      "id": "123456789",
      "name": "Morning Run",
      "sport_type": "Run",
      "distance_meters": 5000,
      "duration_seconds": 1800,
      "start_date": "2024-01-15T07:00:00Z"
    }
  ]
}
```

---

**Built with ❤️ for the fitness and AI developer community**