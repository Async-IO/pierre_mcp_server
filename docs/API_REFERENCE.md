# API Reference

Complete API documentation for the Pierre Fitness API platform, including MCP tools, HTTP endpoints, error handling, and integration examples.

## Table of Contents

1. [MCP Tools Reference](#mcp-tools-reference)
2. [HTTP API Endpoints](#http-api-endpoints)
3. [Error Handling](#error-handling)
4. [Integration Examples](#integration-examples)
5. [Weather Integration](#weather-integration)

## MCP Tools Reference

Pierre Fitness API exposes **21 comprehensive tools** organized into categories for complete fitness data analysis and management.

### 🏃 Core Data Access Tools

#### `get_activities`
Fetch fitness activities with pagination support
- **Parameters**: 
  - `provider` (required): Fitness provider name (e.g., 'strava', 'fitbit')
  - `limit` (optional): Maximum number of activities to return
  - `offset` (optional): Number of activities to skip (for pagination)
- **Providers**: Strava (real-time API), Fitbit (date-based queries)
- **Returns**: Activity list with metrics, GPS data, heart rate, and timing

#### `get_athlete`
Get complete athlete profile information  
- **Parameters**: `provider` (required)
- **Returns**: Name, avatar, stats, preferences, and account details

#### `get_stats`
Get aggregated fitness statistics and lifetime metrics
- **Parameters**: `provider` (required)
- **Returns**: Total distance, activities, elevation, achievements

### 🧠 Activity Intelligence & Analysis

#### `get_activity_intelligence`
AI-powered activity analysis with full context
- **Parameters**: 
  - `provider` (required): Fitness provider name
  - `activity_id` (required): ID of the specific activity to analyze
  - `include_weather` (optional): Whether to include weather analysis (default: true)
  - `include_location` (optional): Whether to include location intelligence (default: true)
- **Features**: Weather correlation, location intelligence, performance metrics
- **Returns**: Natural language insights, personal records, environmental analysis

#### `analyze_activity`
Deep dive analysis of individual activities
- **Parameters**: `provider`, `activity_id`
- **Returns**: Detailed metrics, anomaly detection, performance insights

#### `calculate_metrics`
Advanced fitness calculations (TRIMP, power ratios, efficiency)
- **Parameters**: 
  - `provider` (required): Fitness provider name
  - `activity_id` (required): ID of the activity
  - `metrics` (optional): Specific metrics to calculate (e.g., ['trimp', 'power_to_weight', 'efficiency'])
- **Returns**: Scientific fitness metrics and performance indicators

### 📊 Performance & Trend Analysis

#### `get_performance_trends`
Analyze performance trends over time periods
- **Parameters**: `provider`, `timeframe` (week/month/quarter/year)
- **Returns**: Trend analysis, improvement metrics, statistical insights

#### `compare_activities`
Compare two activities for performance differences
- **Parameters**: `provider`, `activity1_id`, `activity2_id`
- **Returns**: Side-by-side comparison, performance deltas, insights

#### `get_fitness_score`
Calculate comprehensive fitness score based on activities
- **Parameters**: `provider`, `timeframe` (optional)
- **Returns**: Fitness score (0-100), contributing factors, recommendations

### 🎯 Goal Management

#### `get_goals`
Retrieve and manage fitness goals
- **Parameters**: `status` (optional): active/completed/all
- **Returns**: Goal list with progress tracking

#### `create_goal`
Create new fitness goal with tracking
- **Parameters**: Goal details (type, target, deadline)
- **Returns**: Goal ID and tracking setup

#### `update_goal_progress`
Update progress on existing goal
- **Parameters**: `goal_id`, progress data
- **Returns**: Updated goal status and recommendations

### 🌍 Location & Environment

#### `get_location_intelligence`
Analyze activity locations and routes
- **Parameters**: `provider`, `activity_id`
- **Returns**: Location insights, route analysis, terrain information

#### `get_weather_analysis`
Get weather correlation with performance
- **Parameters**: `provider`, `activity_id`
- **Returns**: Weather data and performance impact analysis

### 📈 Advanced Analytics

#### `get_training_recommendations`
Get personalized training recommendations
- **Parameters**: `provider`, `analysis_period` (optional)
- **Returns**: Personalized training suggestions based on data

#### `detect_anomalies`
Identify unusual patterns in fitness data
- **Parameters**: `provider`, `timeframe`
- **Returns**: Anomaly detection results and explanations

#### `get_recovery_analysis`
Analyze recovery patterns and recommendations
- **Parameters**: `provider`, `recent_activities_count` (optional)
- **Returns**: Recovery status and recommendations

### 🔧 Utility Tools

#### `validate_oauth_connection`
Test OAuth connection status for providers
- **Parameters**: `provider`
- **Returns**: Connection status, token validity, refresh information

#### `get_supported_providers`
List all supported fitness providers
- **Parameters**: None
- **Returns**: Provider list with capabilities and status

#### `refresh_provider_data`
Force refresh of data from provider
- **Parameters**: `provider`
- **Returns**: Refresh status and updated data summary

#### `export_data`
Export fitness data in various formats
- **Parameters**: `provider`, `format` (json/csv/gpx), `timeframe`
- **Returns**: Exported data in requested format

## HTTP API Endpoints

### B2B API Platform Features

#### 🔑 API Key Management
- **Tiered Access**: Trial (1K/month), Starter (10K/month), Professional (100K/month), Enterprise (Unlimited)
- **Trial Keys**: 14-day auto-expiring trial keys with one-per-user limit
- **Rate Limiting**: Automatic monthly rate limiting with real-time tracking
- **Usage Analytics**: Detailed usage statistics per tool and time period
- **Secure Storage**: SHA-256 hashed keys with prefix-based identification

#### 📊 Developer Dashboard
- **Real-time Monitoring**: WebSocket-based live updates
- **Usage Analytics**: Tool-specific usage breakdown and trends
- **Rate Limit Status**: Visual indicators and warnings
- **API Key Management**: Create, list, and deactivate keys

#### 🔐 Enterprise Security
- **JWT Authentication**: 24-hour tokens with detailed error messages
- **API Key Authentication**: Production (`pk_live_`) and trial (`pk_trial_`) keys
- **Encrypted Storage**: AES-256-GCM for OAuth tokens at rest
- **CORS Support**: Full cross-origin resource sharing configuration
- **User Isolation**: Complete data separation between tenants

### Admin API Endpoints

| Endpoint | Method | Description | Required Permission |
|----------|--------|-------------|-------------------|
| `/admin/provision-api-key` | POST | Create new API key for user | ProvisionKeys |
| `/admin/revoke-api-key` | POST | Revoke existing API key | RevokeKeys |
| `/admin/list-api-keys` | GET | List API keys with filters | ListKeys |
| `/admin/token-info` | GET | Get admin token information | ManageAdminTokens |
| `/admin/setup-status` | GET | Check if admin setup required | None (public) |
| `/admin/health` | GET | Admin service health check | None (public) |

#### Admin Authentication

All admin endpoints (except `setup-status` and `health`) require an admin JWT token:

```bash
Authorization: Bearer <admin_jwt_token>
```

Admin tokens are generated using the `admin-setup` binary and include specific permissions.

#### List API Keys Endpoint

Get filtered list of API keys with pagination:

```bash
GET /admin/list-api-keys?user_email=user@example.com&active_only=true&limit=50&offset=0
```

**Query Parameters:**
- `user_email` (optional): Filter by user email
- `active_only` (optional): Only return active keys (default: true)
- `limit` (optional): Number of keys to return (1-100, default: unset)
- `offset` (optional): Number of keys to skip (default: 0)

**Response:**
```json
{
  "success": true,
  "message": "Found 3 API keys",
  "data": {
    "filters": {
      "user_email": "user@example.com",
      "active_only": true,
      "limit": 50,
      "offset": 0
    },
    "keys": [
      {
        "id": "key_abc123",
        "user_id": "user_def456",
        "name": "Production Key",
        "description": "Main API key",
        "tier": "professional",
        "rate_limit": {
          "requests": 100000,
          "window": 2592000
        },
        "is_active": true,
        "created_at": "2024-01-15T10:30:00Z",
        "last_used_at": "2024-06-20T14:22:00Z",
        "expires_at": "2025-01-15T10:30:00Z",
        "usage_count": 0
      }
    ],
    "count": 3
  }
}
```

## Error Handling

### Error Response Formats

#### MCP Protocol Errors

MCP errors follow JSON-RPC 2.0 specification:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32001,
    "message": "Authentication failed",
    "data": {
      "error_type": "INVALID_TOKEN",
      "suggestion": "Please login again to get a new token",
      "documentation_url": "https://docs.pierre-fitness.com/authentication"
    }
  }
}
```

#### HTTP API Errors

HTTP errors return JSON with standard structure:

```json
{
  "error": "Invalid API key",
  "message": "API key is invalid, expired, or rate limited",
  "details": {
    "key_prefix": "pk_trial_1234",
    "status": "expired",
    "expires_at": "2024-01-15T10:30:00Z"
  },
  "error_code": "API_KEY_EXPIRED",
  "timestamp": "2024-01-15T14:23:00Z"
}
```

#### A2A Protocol Errors

A2A errors follow JSON-RPC format with specific error codes:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32001,
    "message": "A2A authentication failed",
    "data": {
      "error_type": "INVALID_CLIENT_CREDENTIALS",
      "client_id": "a2a_client_12345",
      "suggestion": "Verify client credentials and try again"
    }
  }
}
```

### Common Error Codes

| Code | Type | Description | HTTP Status |
|------|------|-------------|-------------|
| -32600 | Invalid Request | Malformed JSON-RPC request | 400 |
| -32601 | Method Not Found | MCP tool does not exist | 404 |
| -32602 | Invalid Params | Invalid parameters for tool | 400 |
| -32603 | Internal Error | Server internal error | 500 |
| -32001 | Authentication Error | Authentication failed | 401 |
| -32002 | Permission Denied | Insufficient permissions | 403 |
| -32003 | Rate Limited | Too many requests | 429 |
| -32004 | Provider Error | Fitness provider API error | 502 |
| -32005 | Data Not Found | Requested data not found | 404 |

### Troubleshooting Guide

#### Authentication Issues

**Problem**: `Authentication failed` error
**Solution**: 
1. Check JWT token validity and expiration
2. Verify API key format and status
3. Ensure proper Authorization header format

**Problem**: `Permission denied` error
**Solution**:
1. Verify user has required permissions
2. Check API key tier limits
3. Confirm admin token permissions for admin endpoints

#### Rate Limiting

**Problem**: `Rate limit exceeded` error
**Solution**:
1. Check current usage via `/admin/list-api-keys`
2. Upgrade API key tier if needed
3. Implement backoff and retry logic

#### Provider Issues

**Problem**: Fitness provider connection errors
**Solution**:
1. Use `validate_oauth_connection` tool to check status
2. Re-authorize OAuth connection if expired
3. Check provider API status and limitations

## Integration Examples

### Basic MCP Integration

```python
import json
from mcp import Client

# Initialize MCP client
client = Client("stdio", command=["pierre-mcp-server"])

# Get activities from Strava
response = await client.call_tool("get_activities", {
    "provider": "strava",
    "limit": 10
})

activities = json.loads(response.content[0].text)
print(f"Found {len(activities)} activities")
```

### HTTP API Integration

```python
import requests

# Using API key authentication
headers = {
    "X-API-Key": "pk_live_1234567890abcdef",
    "Content-Type": "application/json"
}

# Get API key usage statistics
response = requests.get(
    "http://localhost:8081/admin/list-api-keys",
    headers=headers
)

if response.status_code == 200:
    data = response.json()
    print(f"Found {data['data']['count']} API keys")
```

### Advanced Activity Analysis

```python
# Get detailed activity intelligence
response = await client.call_tool("get_activity_intelligence", {
    "provider": "strava",
    "activity_id": "12345678",
    "include_weather": True,
    "include_location": True
})

intelligence = json.loads(response.content[0].text)
print(f"Activity summary: {intelligence['summary']}")
print(f"Key insights: {intelligence['key_insights']}")
```

### Goal Management Workflow

```python
# Create a new fitness goal
goal_response = await client.call_tool("create_goal", {
    "type": "distance",
    "target_value": 100,  # 100km
    "target_date": "2024-12-31",
    "sport": "running"
})

goal_id = json.loads(goal_response.content[0].text)["goal_id"]

# Update progress
progress_response = await client.call_tool("update_goal_progress", {
    "goal_id": goal_id,
    "current_value": 25,  # 25km completed
    "update_date": "2024-06-20"
})
```

## Weather Integration

### OpenWeatherMap Setup

Pierre integrates with OpenWeatherMap API for weather correlation analysis.

#### Configuration

```bash
# Set your OpenWeatherMap API key
export OPENWEATHER_API_KEY=your_api_key_here

# Optional: Configure API base URL (default shown)
export OPENWEATHER_API_BASE_URL=https://api.openweathermap.org
```

#### Getting an API Key

1. Sign up at [OpenWeatherMap](https://openweathermap.org/api)
2. Choose a plan (free tier available with 60 calls/minute)
3. Get your API key from the dashboard
4. Set the `OPENWEATHER_API_KEY` environment variable

#### Weather Analysis Features

- **Activity Weather Correlation**: Analyze how weather affected performance
- **Location-based Weather**: Get weather data for activity start locations
- **Historical Weather**: Retrieve weather data for past activities
- **Performance Impact**: Understand weather's effect on training

#### Example Weather Analysis

```python
# Get weather analysis for an activity
weather_response = await client.call_tool("get_weather_analysis", {
    "provider": "strava",
    "activity_id": "12345678"
})

weather_data = json.loads(weather_response.content[0].text)
print(f"Temperature: {weather_data['temperature']}°C")
print(f"Conditions: {weather_data['conditions']}")
print(f"Performance impact: {weather_data['performance_impact']}")
```

#### Weather Data Structure

```json
{
  "temperature": 22.5,
  "humidity": 65,
  "wind_speed": 12.3,
  "conditions": "partly cloudy",
  "pressure": 1013.25,
  "visibility": 10.0,
  "uv_index": 6,
  "performance_impact": {
    "temperature_effect": "optimal",
    "humidity_effect": "moderate",
    "wind_effect": "slight_headwind",
    "overall_rating": "good"
  }
}
```

### Testing Weather Integration

Use the diagnostic tools to test your weather setup:

```bash
# Test basic weather API connection
cargo run --bin test-weather-integration

# Test with real OpenWeatherMap API
cargo run --bin test-real-weather

# Diagnose weather API issues
cargo run --bin diagnose-weather-api
```

## Rate Limiting and Best Practices

### API Rate Limits

| Tier | Monthly Requests | Burst Limit | Recommended Use |
|------|------------------|-------------|-----------------|
| Trial | 1,000 | 10/minute | Testing and evaluation |
| Starter | 10,000 | 30/minute | Small applications |
| Professional | 100,000 | 100/minute | Production applications |
| Enterprise | Unlimited | 1000/minute | High-volume integrations |

### Best Practices

1. **Implement retry logic** with exponential backoff
2. **Cache responses** when appropriate to reduce API calls
3. **Use pagination** for large datasets
4. **Monitor usage** via admin endpoints
5. **Handle errors gracefully** with proper user feedback

### Example Retry Logic

```python
import asyncio
import random

async def call_tool_with_retry(client, tool_name, params, max_retries=3):
    for attempt in range(max_retries):
        try:
            response = await client.call_tool(tool_name, params)
            return response
        except RateLimitError:
            if attempt < max_retries - 1:
                # Exponential backoff with jitter
                delay = (2 ** attempt) + random.uniform(0, 1)
                await asyncio.sleep(delay)
            else:
                raise
        except Exception as e:
            if attempt == max_retries - 1:
                raise
            await asyncio.sleep(1)
```

This completes the comprehensive API reference documentation covering all aspects of the Pierre Fitness API platform.