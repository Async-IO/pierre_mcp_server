# MCP Tools Reference

Pierre MCP Server exposes **21 comprehensive tools** organized into categories for complete fitness data analysis and management.

## 🏃 Core Data Access Tools

### `get_activities`
Fetch fitness activities with pagination support
- **Parameters**: 
  - `provider` (required): Fitness provider name (e.g., 'strava', 'fitbit')
  - `limit` (optional): Maximum number of activities to return
  - `offset` (optional): Number of activities to skip (for pagination)
- **Providers**: Strava (real-time API), Fitbit (date-based queries)
- **Returns**: Activity list with metrics, GPS data, heart rate, and timing

### `get_athlete`
Get complete athlete profile information  
- **Parameters**: `provider` (required)
- **Returns**: Name, avatar, stats, preferences, and account details

### `get_stats`
Get aggregated fitness statistics and lifetime metrics
- **Parameters**: `provider` (required)
- **Returns**: Total distance, activities, elevation, achievements

## 🧠 Activity Intelligence & Analysis

### `get_activity_intelligence`
AI-powered activity analysis with full context
- **Parameters**: 
  - `provider` (required): Fitness provider name
  - `activity_id` (required): ID of the specific activity to analyze
  - `include_weather` (optional): Whether to include weather analysis (default: true)
  - `include_location` (optional): Whether to include location intelligence (default: true)
- **Features**: Weather correlation, location intelligence, performance metrics
- **Returns**: Natural language insights, personal records, environmental analysis

### `analyze_activity`
Deep dive analysis of individual activities
- **Parameters**: `provider`, `activity_id`
- **Returns**: Detailed metrics, anomaly detection, performance insights

### `calculate_metrics`
Advanced fitness calculations (TRIMP, power ratios, efficiency)
- **Parameters**: 
  - `provider` (required): Fitness provider name
  - `activity_id` (required): ID of the activity
  - `metrics` (optional): Specific metrics to calculate (e.g., ['trimp', 'power_to_weight', 'efficiency'])
- **Returns**: Scientific fitness metrics and performance indicators

### `analyze_performance_trends`
Statistical performance analysis over time
- **Parameters**: 
  - `provider` (required): Fitness provider name
  - `timeframe` (required): Time period for analysis ('week', 'month', 'quarter', 'sixmonths', 'year')
  - `metric` (required): Metric to analyze trends for ('pace', 'heart_rate', 'power', 'distance', 'duration')
  - `sport_type` (optional): Filter by sport type
- **Returns**: Trend analysis, regression patterns, performance forecasts

### `compare_activities`
Compare activities against personal bests and averages
- **Parameters**: 
  - `provider` (required): Fitness provider name
  - `activity_id` (required): Primary activity to compare
  - `comparison_type` (required): Type of comparison ('similar_activities', 'personal_best', 'average', 'recent')
- **Returns**: Comparative analysis, rankings, improvement suggestions

### `detect_patterns`
AI pattern detection in training data
- **Parameters**: 
  - `provider` (required): Fitness provider name
  - `pattern_type` (required): Type of pattern to detect ('training_consistency', 'seasonal_trends', 'performance_plateaus', 'injury_risk')
  - `timeframe` (optional): Time period for pattern analysis
- **Returns**: Training consistency, seasonal trends, injury risk patterns

## 🔗 Connection Management Tools

### `connect_strava`
Generate Strava OAuth authorization URL
- **Parameters**: None (uses JWT context)
- **Returns**: Authorization URL with state management for secure OAuth flow

### `connect_fitbit`
Generate Fitbit OAuth authorization URL
- **Parameters**: None (uses JWT context)  
- **Returns**: Authorization URL with PKCE security for Fitbit connection

### `get_connection_status`
Check provider connection status
- **Parameters**: None (uses JWT context)
- **Returns**: Connected providers, token status, authorization expiry

### `disconnect_provider`
Safely disconnect and revoke provider access
- **Parameters**: `provider` (required): Fitness provider to disconnect (e.g., 'strava', 'fitbit')
- **Returns**: Confirmation of token removal and access revocation

## 🎯 Goal Management Tools

### `set_goal`
Create and configure fitness goals with tracking
- **Parameters**: 
  - `title` (required): Goal title
  - `goal_type` (required): Type of goal ('distance', 'time', 'frequency', 'performance', 'custom')
  - `target_value` (required): Target value to achieve
  - `target_date` (required): Target completion date (ISO format)
  - `description` (optional): Goal description
  - `sport_type` (optional): Sport type for the goal
- **Returns**: Goal ID, tracking setup, milestone configuration

### `track_progress`
Monitor progress toward specific goals
- **Parameters**: `goal_id` (required): ID of the goal to track
- **Returns**: Progress percentage, milestone achievements, completion estimates

### `suggest_goals`
AI-generated goal recommendations
- **Parameters**: 
  - `provider` (required): Fitness provider name
  - `goal_category` (optional): Category of goals to suggest ('distance', 'performance', 'consistency', 'all')
- **Returns**: Personalized goal suggestions based on activity history

### `analyze_goal_feasibility`
Assess goal achievability
- **Parameters**: `goal_id` (required): ID of the goal to analyze
- **Returns**: Feasibility analysis, timeline assessment, adjustment recommendations

## 📊 Advanced Analytics Tools

### `generate_recommendations`
Personalized training recommendations
- **Parameters**: 
  - `provider` (required): Fitness provider name
  - `recommendation_type` (optional): Type of recommendations ('training', 'recovery', 'nutrition', 'equipment', 'all')
  - `activity_id` (optional): Specific activity to base recommendations on
- **Returns**: Training, recovery, nutrition, and equipment recommendations

### `calculate_fitness_score`
Comprehensive fitness scoring
- **Parameters**: 
  - `provider` (required): Fitness provider name
  - `timeframe` (optional): Time period for fitness assessment ('month', 'quarter', 'sixmonths')
- **Returns**: Overall fitness score, component analysis, improvement areas

### `predict_performance`
Future performance predictions
- **Parameters**: 
  - `provider` (required): Fitness provider name
  - `target_sport` (required): Sport type for prediction
  - `target_distance` (required): Target distance for performance prediction
  - `target_date` (optional): Target date for prediction (ISO format)
- **Returns**: Performance predictions, confidence intervals, training requirements

### `analyze_training_load`
Training load analysis and balance
- **Parameters**: 
  - `provider` (required): Fitness provider name
  - `timeframe` (optional): Time period for load analysis ('week', 'month', 'quarter')
- **Returns**: Load distribution, recovery needs, training stress balance

## 🌟 Real-World Data Examples

**Live Strava Integration** (based on successful OAuth testing):
```json
// Real athlete data from connected account
{
  "firstname": "Jeanfrancois",
  "lastname": "Arcand", 
  "username": "cheffamille",
  "id": "32530060",
  "profile_picture": "https://dgalywyr863hv.cloudfront.net/pictures/athletes/32530060/22178752/5/large.jpg",
  "is_real_data": true
}

// Recent activities with real metrics
{
  "activities": [
    {
      "name": "Voisinage",
      "sport_type": "Ride",
      "distance_meters": 10393.7,
      "duration_seconds": 3625,
      "elevation_gain": 260.0,
      "average_heart_rate": 109,
      "max_heart_rate": 145,
      "start_date": "2025-06-17T12:08:49+00:00",
      "is_real_data": true
    }
  ]
}
```

## Example Tool Usage

### Real data access with authenticated user
```bash
curl -X POST http://localhost:8081/a2a/execute \
  -H "Authorization: Bearer JWT_TOKEN" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools.execute",
    "params": {
      "tool_name": "get_activities",
      "parameters": {"provider": "strava", "limit": 10}
    }
  }'
```

### Activity intelligence with full context
```bash
curl -X POST http://localhost:8081/a2a/execute \
  -H "Authorization: Bearer JWT_TOKEN" \
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

### Goal management
```bash
curl -X POST http://localhost:8081/a2a/execute \
  -H "Authorization: Bearer JWT_TOKEN" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools.execute",
    "params": {
      "tool_name": "set_goal",
      "parameters": {
        "title": "Run 1000km in 2025",
        "goal_type": "distance",
        "target_value": 1000000,
        "target_date": "2025-12-31",
        "sport_type": "Run"
      }
    }
  }'
```

## Implementation Notes

- **Universal Tool Executor**: All tools are implemented through a Universal Tool Executor that supports both MCP and A2A protocols.
- **Provider Support**: The server primarily supports Strava and Fitbit as fitness data providers.
- **Authentication**: Tools requiring provider data need the user to be authenticated via OAuth flow first using the connection tools.
- **Real Data Access**: Tools like `get_activities`, `get_athlete`, and `get_stats` can access real data from connected Strava accounts with valid OAuth tokens.