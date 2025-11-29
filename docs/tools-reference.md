# MCP Tools Reference

Comprehensive reference for all 45 Model Context Protocol (MCP) tools provided by Pierre Fitness Platform. These tools enable AI assistants to access fitness data, analyze performance, manage configurations, and provide personalized recommendations.

## Overview

Pierre MCP Server provides tools organized into 8 functional categories:
- **Core Fitness Tools**: Activity data and provider connections
- **OAuth & Notifications**: Authentication status and notifications
- **Goals & Planning**: Goal setting and progress tracking
- **Performance Analysis**: Activity insights and trend analysis
- **Configuration Management**: System-wide configuration
- **Fitness Configuration**: User fitness zones and thresholds
- **Sleep & Recovery**: Sleep analysis and recovery tracking
- **Nutrition**: Dietary calculations and USDA food database

---

## Core Fitness Tools

Basic fitness data retrieval and provider connection management.

| Tool Name | Description | Required Parameters | Optional Parameters |
|-----------|-------------|---------------------|---------------------|
| `get_activities` | Get user's fitness activities with optional filtering | `provider` (string) | `limit` (number), `offset` (number) |
| `get_athlete` | Get user's athlete profile and basic information | `provider` (string) | - |
| `get_stats` | Get user's performance statistics and metrics | `provider` (string) | - |
| `get_connection_status` | Check OAuth connection status for fitness providers | - | `strava_client_id` (string), `strava_client_secret` (string), `fitbit_client_id` (string), `fitbit_client_secret` (string) |
| `connect_to_pierre` | Connect to Pierre MCP server and trigger OAuth authentication flow | - | - |
| `connect_provider` | Connect to a fitness data provider via OAuth | `provider` (string) | - |
| `disconnect_provider` | Disconnect user from a fitness data provider | `provider` (string) | - |

### Parameter Details

**Supported Providers**: `strava`, `garmin`, `fitbit`, `whoop`, `terra`

**`get_activities` Parameters**:
- `provider`: Fitness provider name (e.g., 'strava', 'garmin', 'fitbit', 'whoop', 'terra')
- `limit`: Maximum number of activities to return
- `offset`: Number of activities to skip (for pagination)

**`get_connection_status` Parameters**:
- `strava_client_id`: Your Strava OAuth client ID (uses server defaults if not provided)
- `strava_client_secret`: Your Strava OAuth client secret
- `fitbit_client_id`: Your Fitbit OAuth client ID (uses server defaults if not provided)
- `fitbit_client_secret`: Your Fitbit OAuth client secret

---

## OAuth & Notifications

Tools for managing OAuth authentication notifications and status updates.

| Tool Name | Description | Required Parameters | Optional Parameters |
|-----------|-------------|---------------------|---------------------|
| `announce_oauth_success` | Announce OAuth connection success directly in chat | `provider` (string), `message` (string), `notification_id` (string) | - |
| `check_oauth_notifications` | Check for new OAuth completion notifications | - | - |
| `get_notifications` | Get list of OAuth notifications for the user | - | `include_read` (boolean), `provider` (string) |
| `mark_notifications_read` | Mark OAuth notifications as read | - | `notification_id` (string) |

### Parameter Details

**`get_notifications` Parameters**:
- `include_read`: Whether to include already read notifications (default: false)
- `provider`: Filter notifications by provider (e.g., 'strava', 'fitbit')

**`mark_notifications_read` Parameters**:
- `notification_id`: ID of specific notification to mark as read. If omitted, marks all unread notifications as read.

---

## Goals & Planning

Tools for setting fitness goals, tracking progress, and receiving AI-powered goal suggestions.

| Tool Name | Description | Required Parameters | Optional Parameters |
|-----------|-------------|---------------------|---------------------|
| `set_goal` | Create and manage fitness goals with tracking | `title` (string), `goal_type` (string), `target_value` (number), `target_date` (string) | `description` (string), `sport_type` (string) |
| `suggest_goals` | Get AI-suggested fitness goals based on activity history | `provider` (string) | `goal_category` (string) |
| `analyze_goal_feasibility` | Analyze whether a goal is achievable given current fitness level | `goal_id` (string) | - |
| `track_progress` | Track progress towards fitness goals | `goal_id` (string) | - |

### Parameter Details

**`set_goal` Parameters**:
- `goal_type`: Type of goal - `distance`, `time`, `frequency`, `performance`, or `custom`
- `target_date`: Target completion date in ISO format (e.g., "2025-12-31")

**`suggest_goals` Parameters**:
- `goal_category`: Category of goals - `distance`, `performance`, `consistency`, or `all`

---

## Performance Analysis

Advanced analytics tools for activity analysis, trend detection, and performance predictions.

| Tool Name | Description | Required Parameters | Optional Parameters |
|-----------|-------------|---------------------|---------------------|
| `analyze_activity` | Analyze a specific activity with detailed performance insights | `provider` (string), `activity_id` (string) | - |
| `get_activity_intelligence` | Get AI-powered intelligence analysis for an activity | `provider` (string), `activity_id` (string) | `include_weather` (boolean), `include_location` (boolean) |
| `calculate_metrics` | Calculate custom fitness metrics and performance indicators | `provider` (string), `activity_id` (string) | `metrics` (array) |
| `analyze_performance_trends` | Analyze performance trends over time | `provider` (string), `timeframe` (string), `metric` (string) | `sport_type` (string) |
| `compare_activities` | Compare two activities for performance analysis | `provider` (string), `activity_id` (string), `comparison_type` (string) | - |
| `detect_patterns` | Detect patterns and insights in activity data | `provider` (string), `pattern_type` (string) | `timeframe` (string) |
| `generate_recommendations` | Generate personalized training recommendations | `provider` (string) | `recommendation_type` (string), `activity_id` (string) |
| `calculate_fitness_score` | Calculate overall fitness score based on recent activities | `provider` (string) | `timeframe` (string) |
| `predict_performance` | Predict future performance based on training patterns | `provider` (string), `target_sport` (string), `target_distance` (number) | `target_date` (string) |
| `analyze_training_load` | Analyze training load and recovery metrics | `provider` (string) | `timeframe` (string) |

### Parameter Details

**`get_activity_intelligence` Parameters**:
- `include_weather`: Whether to include weather analysis (default: true)
- `include_location`: Whether to include location intelligence (default: true)

**`calculate_metrics` Parameters**:
- `metrics`: Array of specific metrics to calculate (e.g., `['trimp', 'power_to_weight', 'efficiency']`)

**`analyze_performance_trends` Parameters**:
- `timeframe`: Time period - `week`, `month`, `quarter`, `sixmonths`, or `year`
- `metric`: Metric to analyze - `pace`, `heart_rate`, `power`, `distance`, or `duration`

**`compare_activities` Parameters**:
- `comparison_type`: Type of comparison - `similar_activities`, `personal_best`, `average`, or `recent`

**`detect_patterns` Parameters**:
- `pattern_type`: Pattern to detect - `training_consistency`, `seasonal_trends`, `performance_plateaus`, or `injury_risk`

**`generate_recommendations` Parameters**:
- `recommendation_type`: Type of recommendations - `training`, `recovery`, `nutrition`, `equipment`, or `all`

---

## Configuration Management

System-wide configuration management tools for physiological parameters and training zones.

| Tool Name | Description | Required Parameters | Optional Parameters |
|-----------|-------------|---------------------|---------------------|
| `get_configuration_catalog` | Get complete configuration catalog with all available parameters | - | - |
| `get_configuration_profiles` | Get available configuration profiles (Research, Elite, Recreational, etc.) | - | - |
| `get_user_configuration` | Get current user's configuration settings and overrides | - | - |
| `update_user_configuration` | Update user's configuration parameters and session overrides | - | `profile` (string), `parameters` (object) |
| `calculate_personalized_zones` | Calculate personalized training zones based on VO2 max | `vo2_max` (number) | `resting_hr` (number), `max_hr` (number), `lactate_threshold` (number), `sport_efficiency` (number) |
| `validate_configuration` | Validate configuration parameters against safety rules | `parameters` (object) | - |

### Parameter Details

**`update_user_configuration` Parameters**:
- `profile`: Configuration profile to apply (e.g., 'Research', 'Elite', 'Recreational', 'Beginner', 'Medical')
- `parameters`: Parameter overrides as JSON object

**`calculate_personalized_zones` Parameters**:
- `vo2_max`: VO2 max in ml/kg/min
- `resting_hr`: Resting heart rate in bpm (default: 60)
- `max_hr`: Maximum heart rate in bpm (default: 190)
- `lactate_threshold`: Lactate threshold as percentage of VO2 max (default: 0.85)
- `sport_efficiency`: Sport efficiency factor (default: 1.0)

---

## Fitness Configuration

User-specific fitness configuration for heart rate zones, power zones, and training thresholds.

| Tool Name | Description | Required Parameters | Optional Parameters |
|-----------|-------------|---------------------|---------------------|
| `get_fitness_config` | Get user fitness configuration settings | - | `configuration_name` (string) |
| `set_fitness_config` | Save user fitness configuration settings | `configuration` (object) | `configuration_name` (string) |
| `list_fitness_configs` | List all fitness configuration names | - | - |
| `delete_fitness_config` | Delete a specific fitness configuration | `configuration_name` (string) | - |

### Parameter Details

**`get_fitness_config` / `set_fitness_config` Parameters**:
- `configuration_name`: Name of the configuration (defaults to 'default')
- `configuration`: Fitness configuration object containing zones, thresholds, and training parameters

**Configuration Object Structure**:
```json
{
  "heart_rate_zones": {
    "zone1": {"min": 100, "max": 120},
    "zone2": {"min": 120, "max": 140},
    "zone3": {"min": 140, "max": 160},
    "zone4": {"min": 160, "max": 180},
    "zone5": {"min": 180, "max": 200}
  },
  "power_zones": { /* similar structure */ },
  "ftp": 250,
  "lthr": 165,
  "max_hr": 190,
  "resting_hr": 50,
  "weight_kg": 70
}
```

---

## Sleep & Recovery

Sleep quality analysis and recovery monitoring tools using NSF/AASM guidelines.

| Tool Name | Description | Required Parameters | Optional Parameters |
|-----------|-------------|---------------------|---------------------|
| `analyze_sleep_quality` | Analyze sleep quality from Fitbit/Garmin data | `sleep_data` (object) | `recent_hrv_values` (array), `baseline_hrv` (number) |
| `calculate_recovery_score` | Calculate holistic recovery score combining TSB, sleep, and HRV | `provider` (string) | `user_config` (object) |
| `suggest_rest_day` | AI-powered rest day recommendation | `provider` (string) | - |
| `track_sleep_trends` | Track sleep patterns over time | `sleep_history` (array) | - |
| `optimize_sleep_schedule` | Optimize sleep duration based on training load | `provider` (string) | `user_config` (object), `upcoming_workout_intensity` (string) |

### Parameter Details

**`analyze_sleep_quality` Sleep Data Object**:
```json
{
  "date": "2025-11-28",
  "duration_hours": 7.5,
  "efficiency_percent": 85,
  "deep_sleep_hours": 1.5,
  "rem_sleep_hours": 2.0,
  "light_sleep_hours": 4.0,
  "awakenings": 2,
  "hrv_rmssd_ms": 45
}
```

**`calculate_recovery_score` / `optimize_sleep_schedule` User Config**:
```json
{
  "ftp": 250,
  "lthr": 165,
  "max_hr": 190,
  "resting_hr": 50,
  "weight_kg": 70
}
```

**`track_sleep_trends` Parameters**:
- `sleep_history`: Array of sleep data objects (minimum 7 days required)

**`optimize_sleep_schedule` Parameters**:
- `upcoming_workout_intensity`: Intensity level - `low`, `moderate`, or `high` (default: 'moderate')

---

## Nutrition

Nutrition calculation tools with USDA FoodData Central database integration.

| Tool Name | Description | Required Parameters | Optional Parameters |
|-----------|-------------|---------------------|---------------------|
| `calculate_daily_nutrition` | Calculate daily calorie and macronutrient needs (Mifflin-St Jeor) | `weight_kg` (number), `height_cm` (number), `age` (number), `gender` (string), `activity_level` (string), `training_goal` (string) | - |
| `get_nutrient_timing` | Get optimal pre/post-workout nutrition (ISSN guidelines) | `weight_kg` (number), `daily_protein_g` (number), `workout_intensity` (string) | - |
| `search_food` | Search USDA FoodData Central database | `query` (string) | `page_size` (number) |
| `get_food_details` | Get detailed nutritional information for a food | `fdc_id` (number) | - |
| `analyze_meal_nutrition` | Analyze total calories and macros for a meal | `foods` (array) | - |

### Parameter Details

**`calculate_daily_nutrition` Parameters**:
- `gender`: Either `male` or `female`
- `activity_level`: `sedentary`, `lightly_active`, `moderately_active`, `very_active`, or `extra_active`
- `training_goal`: `maintenance`, `weight_loss`, `muscle_gain`, or `endurance_performance`
- `age`: Age in years (max 150)

**`get_nutrient_timing` Parameters**:
- `workout_intensity`: Workout intensity level - `low`, `moderate`, or `high`

**`search_food` Parameters**:
- `query`: Food name or description to search for
- `page_size`: Number of results to return (default: 10, max: 200)

**`get_food_details` Parameters**:
- `fdc_id`: USDA FoodData Central ID (obtained from `search_food` results)

**`analyze_meal_nutrition` Foods Array**:
```json
{
  "foods": [
    {"fdc_id": 171705, "grams": 100},
    {"fdc_id": 173424, "grams": 50}
  ]
}
```

---

## Usage Examples

### Connecting to a Provider
```json
{
  "tool": "connect_provider",
  "parameters": {
    "provider": "strava"
  }
}
```

### Getting Recent Activities
```json
{
  "tool": "get_activities",
  "parameters": {
    "provider": "strava",
    "limit": 10,
    "offset": 0
  }
}
```

### Analyzing Activity Intelligence
```json
{
  "tool": "get_activity_intelligence",
  "parameters": {
    "provider": "strava",
    "activity_id": "12345678",
    "include_weather": true,
    "include_location": true
  }
}
```

### Setting a Fitness Goal
```json
{
  "tool": "set_goal",
  "parameters": {
    "title": "Run 100km this month",
    "goal_type": "distance",
    "target_value": 100000,
    "target_date": "2025-12-31",
    "sport_type": "Run"
  }
}
```

### Calculating Daily Nutrition
```json
{
  "tool": "calculate_daily_nutrition",
  "parameters": {
    "weight_kg": 70,
    "height_cm": 175,
    "age": 30,
    "gender": "male",
    "activity_level": "very_active",
    "training_goal": "endurance_performance"
  }
}
```

### Analyzing Sleep Quality
```json
{
  "tool": "analyze_sleep_quality",
  "parameters": {
    "sleep_data": {
      "date": "2025-11-28",
      "duration_hours": 7.5,
      "efficiency_percent": 85,
      "deep_sleep_hours": 1.5,
      "rem_sleep_hours": 2.0,
      "light_sleep_hours": 4.0,
      "awakenings": 2,
      "hrv_rmssd_ms": 45
    }
  }
}
```

---

## Notes

- **Authentication**: Most tools require OAuth authentication with Pierre and the respective fitness provider
- **Provider Support**: Supports Strava, Garmin, Fitbit, WHOOP, and Terra (150+ wearables) providers
- **Rate Limits**: Subject to provider API rate limits (e.g., Strava: 100 requests per 15 minutes, 1000 per day)
- **Token Refresh**: OAuth tokens are automatically refreshed when expired
- **USDA Database**: Food search tools use free USDA FoodData Central API with 24-hour caching
- **Scientific Guidelines**:
  - Sleep analysis follows NSF (National Sleep Foundation) and AASM (American Academy of Sleep Medicine) guidelines
  - Nutrition recommendations follow ISSN (International Society of Sports Nutrition) guidelines
  - BMR calculations use validated Mifflin-St Jeor formula

---

## Tool Categories Summary

| Category | Tool Count | Description |
|----------|------------|-------------|
| Core Fitness | 7 | Activity data and provider connections |
| OAuth & Notifications | 4 | Authentication status and notifications |
| Goals & Planning | 4 | Goal management and progress tracking |
| Performance Analysis | 10 | Activity analytics and predictions |
| Configuration Management | 6 | System configuration and zones |
| Fitness Configuration | 4 | User fitness settings |
| Sleep & Recovery | 5 | Sleep analysis and recovery metrics |
| Nutrition | 5 | Dietary calculations and food database |
| **Total** | **45** | **Complete MCP tool suite** |

---

## Additional Resources

- [MCP Protocol Specification](https://github.com/anthropics/mcp)
- [Pierre MCP Server Repository](https://github.com/yourusername/pierre_mcp_server)
- [Development Guide](./development.md)
- [Testing Guide](./testing.md)
- [Configuration Guide](./configuration.md)

---

*Last Updated: 2025-11-28*
*Pierre Fitness Platform v1.0.0*
