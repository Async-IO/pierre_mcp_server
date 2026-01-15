<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# MCP Tools Reference

Comprehensive reference for all 47 Model Context Protocol (MCP) tools provided by Pierre Fitness Platform. These tools enable AI assistants to access fitness data, analyze performance, manage configurations, and provide personalized recommendations.

## Overview

Pierre MCP Server provides tools organized into 8 functional categories:
- **Core Fitness Tools**: Activity data and provider connections
- **Goals & Planning**: Goal setting and progress tracking
- **Performance Analysis**: Activity insights and trend analysis
- **Configuration Management**: System-wide configuration
- **Fitness Configuration**: User fitness zones and thresholds
- **Sleep & Recovery**: Sleep analysis and recovery tracking
- **Nutrition**: Dietary calculations and USDA food database
- **Recipe Management**: Training-aware meal planning and recipe storage

### Output Format

Most data-returning tools support an optional `format` parameter:
- `json` (default): Standard JSON output
- `toon`: [Token-Oriented Object Notation](https://toonformat.dev) for ~40% fewer LLM tokens

Use `format: "toon"` when querying large datasets (year summaries, batch analysis) to reduce LLM context usage.

---

## Core Fitness Tools

Basic fitness data retrieval and provider connection management.

| Tool Name | Description | Required Parameters | Optional Parameters |
|-----------|-------------|---------------------|---------------------|
| `get_activities` | Get user's fitness activities with optional filtering | `provider` (string) | `limit`, `offset`, `before`, `after`, `sport_type`, `mode`, `format` |
| `get_athlete` | Get user's athlete profile and basic information | `provider` (string) | `format` |
| `get_stats` | Get user's performance statistics and metrics | `provider` (string) | `format` |
| `get_connection_status` | Check OAuth connection status for fitness providers | - | `strava_client_id` (string), `strava_client_secret` (string), `fitbit_client_id` (string), `fitbit_client_secret` (string) |
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
| `calculate_fitness_score` | Calculate overall fitness score based on recent activities | `provider` (string) | `timeframe` (string), `sleep_provider` (string) |
| `predict_performance` | Predict future performance based on training patterns | `provider` (string), `target_sport` (string), `target_distance` (number) | `target_date` (string) |
| `analyze_training_load` | Analyze training load and recovery metrics | `provider` (string) | `timeframe` (string), `sleep_provider` (string) |

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

**`calculate_fitness_score` Parameters** (Cross-Provider Support):
- `timeframe`: Analysis period - `month`, `last_90_days`, or `all_time`
- `sleep_provider`: Optional sleep/recovery provider for cross-provider analysis (e.g., `whoop`, `garmin`). When specified, recovery quality factors into the fitness score:
  - Excellent recovery (90-100): +5% fitness score bonus
  - Good recovery (70-89): No adjustment
  - Moderate recovery (50-69): -5% penalty
  - Poor recovery (<50): -10% penalty

**`analyze_training_load` Parameters** (Cross-Provider Support):
- `timeframe`: Analysis period - `week`, `month`, etc.
- `sleep_provider`: Optional sleep/recovery provider for cross-provider analysis. Adds recovery context to training load analysis including sleep quality score, HRV data, and recovery status.

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

Sleep quality analysis and recovery monitoring tools using NSF/AASM guidelines. These tools support **cross-provider data fetching**, allowing you to use activities from one provider and sleep/recovery data from another.

| Tool Name | Description | Required Parameters | Optional Parameters |
|-----------|-------------|---------------------|---------------------|
| `analyze_sleep_quality` | Analyze sleep quality from provider data or manual input | Either `sleep_provider` OR `sleep_data` | `activity_provider`, `days_back`, `recent_hrv_values`, `baseline_hrv` |
| `calculate_recovery_score` | Calculate holistic recovery score combining TSB, sleep, and HRV | Either `activity_provider` OR `sleep_provider` | `sleep_provider`, `activity_provider`, `user_config` |
| `suggest_rest_day` | AI-powered rest day recommendation | Either `activity_provider` OR `sleep_data` | `activity_provider`, `sleep_provider`, `training_load`, `recovery_score` |
| `track_sleep_trends` | Track sleep patterns over time | Either `sleep_provider` OR `sleep_history` | `days_back` |
| `optimize_sleep_schedule` | Optimize sleep duration based on training load | Either `activity_provider` OR `sleep_history` | `activity_provider`, `sleep_provider`, `target_sleep_hours`, `training_schedule` |

### Cross-Provider Support

Sleep and recovery tools support fetching data from different providers for activities and sleep. This enables scenarios like:

- **Strava + WHOOP**: Activities from Strava, recovery/sleep data from WHOOP
- **Garmin + Fitbit**: Running data from Garmin, sleep tracking from Fitbit
- **Any activity provider + Any sleep provider**: Mix and match based on your device ecosystem

**Provider Priority (when auto-selecting)**:
- **Activity providers**: strava > garmin > fitbit > whoop > terra > synthetic
- **Sleep providers**: whoop > garmin > fitbit > terra > synthetic

**Example: Cross-Provider Recovery Score**:
```json
{
  "tool": "calculate_recovery_score",
  "parameters": {
    "activity_provider": "strava",
    "sleep_provider": "whoop"
  }
}
```

**Response includes providers used**:
```json
{
  "recovery_score": { ... },
  "providers_used": {
    "activity_provider": "strava",
    "sleep_provider": "whoop"
  }
}
```

### Parameter Details

**`analyze_sleep_quality` Sleep Data Object** (for manual input mode):
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
- `sleep_provider`: Provider name to fetch sleep history from (alternative to `sleep_history`)
- `days_back`: Number of days to analyze (default: 14)

**`optimize_sleep_schedule` Parameters**:
- `activity_provider`: Provider for activity data
- `sleep_provider`: Provider for sleep data (optional, can be same as activity_provider)
- `target_sleep_hours`: Target sleep duration in hours (default: 8.0)
- `training_schedule`: Weekly training schedule object

---

## Nutrition

Nutrition calculation tools with USDA FoodData Central database integration.

| Tool Name | Description | Required Parameters | Optional Parameters |
|-----------|-------------|---------------------|---------------------|
| `calculate_daily_nutrition` | Calculate daily calorie and macronutrient needs (Mifflin-St Jeor) | `weight_kg` (number), `height_cm` (number), `age` (number), `gender` (string), `activity_level` (string), `training_goal` (string) | - |
| `get_nutrient_timing` | Get optimal pre/post-workout nutrition (ISSN guidelines) | `weight_kg` (number), `daily_protein_g` (number) | `workout_intensity` (string), `activity_provider` (string), `days_back` (number) |
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
- `workout_intensity`: Workout intensity level - `low`, `moderate`, or `high` (required if `activity_provider` not specified)
- `activity_provider`: Fitness provider for activity data (e.g., `strava`, `garmin`). When specified, workout intensity is auto-inferred from recent training load
- `days_back`: Number of days of activity history to analyze for intensity inference (default: 7, max: 30)

**Cross-Provider Support**: When using `activity_provider`, the tool analyzes your recent training data to automatically determine workout intensity based on training volume and heart rate patterns:
- **High intensity**: >2 hours/day or average HR >150 bpm
- **Moderate intensity**: 1-2 hours/day or average HR 130-150 bpm
- **Low intensity**: <1 hour/day and average HR <130 bpm

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

**Using a sleep provider** (recommended):
```json
{
  "tool": "analyze_sleep_quality",
  "parameters": {
    "sleep_provider": "whoop",
    "days_back": 7
  }
}
```

**Cross-provider analysis** (activities from Strava, sleep from WHOOP):
```json
{
  "tool": "analyze_sleep_quality",
  "parameters": {
    "activity_provider": "strava",
    "sleep_provider": "whoop"
  }
}
```

**Manual sleep data input** (for providers without direct integration):
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

## Recipe Management

Training-aware recipe management tools for meal planning aligned with workout schedules. Uses the "Combat des Chefs" architecture where LLM clients generate recipes and Pierre validates nutrition via USDA.

| Tool Name | Description | Required Parameters | Optional Parameters |
|-----------|-------------|---------------------|---------------------|
| `get_recipe_constraints` | Get macro targets and guidelines for meal timing | - | `calories` (number), `tdee` (number), `meal_timing` (string), `dietary_restrictions` (array), `max_prep_time_mins` (number), `max_cook_time_mins` (number) |
| `validate_recipe` | Validate recipe nutrition against training targets | `servings` (number), `ingredients` (array) | `meal_timing` (string), `target_calories` (number), `dietary_restrictions` (array) |
| `save_recipe` | Save validated recipe to user's collection | `name` (string), `servings` (number), `instructions` (array), `ingredients` (array) | `description` (string), `prep_time_mins` (number), `cook_time_mins` (number), `tags` (array), `meal_timing` (string) |
| `list_recipes` | List user's saved recipes | - | `meal_timing` (string), `tags` (array), `limit` (number), `offset` (number) |
| `get_recipe` | Get a specific recipe by ID | `recipe_id` (string) | - |
| `delete_recipe` | Delete a recipe from user's collection | `recipe_id` (string) | - |
| `search_recipes` | Search recipes by name, ingredients, or tags | `query` (string) | `meal_timing` (string), `limit` (number) |

### Parameter Details

**Meal Timing Values**:
- `pre_training`: High-carb focus (55% carbs, 20% protein, 25% fat)
- `post_training`: High-protein focus (45% carbs, 30% protein, 25% fat)
- `rest_day`: Lower carb, moderate protein (35% carbs, 30% protein, 35% fat)
- `general`: Balanced macros (45% carbs, 25% protein, 30% fat)

**Ingredient Object Structure**:
```json
{
  "name": "chicken breast",
  "amount": 200,
  "unit": "grams",
  "preparation": "grilled"
}
```

**Supported Units** (auto-converted to grams):
- Weight: `grams`, `g`, `oz`, `ounces`, `lb`, `pounds`, `kg`
- Volume: `ml`, `milliliters`, `cups`, `cup`, `tbsp`, `tablespoon`, `tsp`, `teaspoon`
- Count: `pieces`, `piece`, `whole`

**Skill Level Values**: `beginner`, `intermediate`, `advanced`

**Dietary Restrictions**: `vegetarian`, `vegan`, `gluten_free`, `dairy_free`, `nut_free`, `keto`, `paleo`

**Example: Validate a Post-Workout Recipe**:
```json
{
  "tool": "validate_recipe",
  "parameters": {
    "servings": 1,
    "meal_timing": "post_training",
    "target_calories": 600,
    "ingredients": [
      {"name": "chicken breast", "amount": 200, "unit": "grams"},
      {"name": "brown rice", "amount": 1, "unit": "cup"},
      {"name": "broccoli", "amount": 150, "unit": "grams"}
    ]
  }
}
```

**Example: Save a Recipe**:
```json
{
  "tool": "save_recipe",
  "parameters": {
    "name": "Recovery Shake",
    "servings": 1,
    "meal_timing": "post_training",
    "description": "Quick protein shake for post-workout recovery",
    "prep_time_mins": 5,
    "ingredients": [
      {"name": "whey protein powder", "amount": 30, "unit": "grams"},
      {"name": "banana", "amount": 1, "unit": "piece"},
      {"name": "almond milk", "amount": 1, "unit": "cup"}
    ],
    "instructions": ["Add all ingredients to blender", "Blend until smooth"],
    "tags": ["quick", "shake", "high-protein"]
  }
}
```

---

## Tool Selection & Administration

Pierre MCP Server supports per-tenant tool selection, allowing administrators to control which tools are available to each tenant based on subscription plans and custom overrides.

### Overview

Tools are organized into 8 categories and gated by subscription plan tiers:

| Plan | Available Tools | Description |
|------|-----------------|-------------|
| `starter` | Core Fitness, Configuration, Connections | Basic data access and setup |
| `professional` | Starter + Analysis, Goals, Nutrition, Sleep, Recipes | Advanced analytics and planning |
| `enterprise` | All 47 tools | Complete platform access |

### How Tool Selection Works

When an MCP client calls a tool, the server checks:

1. **Plan Restriction** - Does tenant's plan meet the tool's `min_plan` requirement?
2. **Tenant Override** - Is there a custom override for this tenant/tool?
3. **Catalog Default** - Fall back to the tool's default enablement

If a tool is disabled, the server returns:
```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32601,
    "message": "Tool 'X' is not available for your tenant. Contact your administrator to enable it."
  }
}
```

### Tool Categories by Plan Tier

**Starter Plan (Default)**:
- Core Fitness: `get_activities`, `get_athlete`, `get_stats`, `connect_provider`, `disconnect_provider`, `get_connection_status`
- Configuration: `get_user_profile`, `set_preferences`, `get_system_config`
- Connections: OAuth management tools

**Professional Plan**:
- All Starter tools, plus:
- Performance Analysis: `analyze_activity`, `analyze_performance_trends`, `calculate_training_load`
- Goals: `set_goal`, `suggest_goals`, `track_progress`
- Nutrition: `calculate_nutrition`, `search_usda_foods`
- Sleep: `analyze_sleep`, `get_sleep_metrics`
- Recipes: `create_recipe`, `validate_recipe`

**Enterprise Plan**:
- All Professional tools, plus:
- Advanced AI: `get_activity_intelligence`, `predict_performance`
- Premium Analytics: `calculate_fitness_score`, `detect_patterns`

### Tenant Overrides

Administrators can customize tool availability per tenant:

- **Enable** tools above plan level (e.g., beta testing)
- **Disable** specific tools for compliance/policy reasons
- Each override records the admin who made the change and optional reason

Overrides are stored in `tenant_tool_overrides` table and cached for performance (5-minute TTL).

### Integration Notes

- **Unknown Tools**: Tools not in the catalog (plugins) pass through by default
- **Caching**: Effective tool lists are cached per tenant (LRU, 1000 tenants max)
- **Cache Invalidation**: Automatic on override changes

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
| Core Fitness | 6 | Activity data and provider connections |
| Goals & Planning | 4 | Goal management and progress tracking |
| Performance Analysis | 10 | Activity analytics and predictions |
| Configuration Management | 6 | System configuration and zones |
| Fitness Configuration | 4 | User fitness settings |
| Sleep & Recovery | 5 | Sleep analysis and recovery metrics |
| Nutrition | 5 | Dietary calculations and food database |
| Recipe Management | 7 | Training-aware meal planning and recipes |
| **Total** | **47** | **Complete MCP tool suite** |

---

## Additional Resources

- [MCP Protocol Specification](https://github.com/anthropics/mcp)
- [Pierre MCP Server Repository](https://github.com/Async-IO/pierre_mcp_server)
- [Development Guide](./development.md)
- [Testing Guide](./testing.md)
- [Configuration Guide](./configuration.md)

---

*Last Updated: 2025-12-06*
*Pierre Fitness Platform v1.0.0*
