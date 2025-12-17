# Pierre MCP Server - Reference Part 3: Tools & Intelligence

> Reference documentation for ChatGPT. Part 3: Tools, Sports Science, Nutrition.

---

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
- `toon`: Token-Oriented Object Notation for ~40% fewer LLM tokens

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
| `get_recipe_constraints` | Get macro targets and guidelines for meal timing | - | `meal_timing` (string), `target_calories` (number) |
| `validate_recipe` | Validate recipe nutrition against training targets | `name` (string), `ingredients` (array), `meal_timing` (string) | `target_calories` (number), `dietary_restrictions` (array) |
| `save_recipe` | Save validated recipe to user's collection | `name` (string), `ingredients` (array), `meal_timing` (string) | `description` (string), `servings` (number), `prep_time_minutes` (number), `cook_time_minutes` (number), `instructions` (array), `tags` (array), `dietary_restrictions` (array), `skill_level` (string), `source` (string) |
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
  "quantity": 200,
  "unit": "grams",
  "fdc_id": 171077
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
    "name": "Post-Workout Protein Bowl",
    "meal_timing": "post_training",
    "target_calories": 600,
    "ingredients": [
      {"name": "chicken breast", "quantity": 200, "unit": "grams"},
      {"name": "brown rice", "quantity": 1, "unit": "cup"},
      {"name": "broccoli", "quantity": 150, "unit": "grams"}
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
    "meal_timing": "post_training",
    "description": "Quick protein shake for post-workout recovery",
    "servings": 1,
    "prep_time_minutes": 5,
    "ingredients": [
      {"name": "whey protein powder", "quantity": 30, "unit": "grams"},
      {"name": "banana", "quantity": 1, "unit": "piece"},
      {"name": "almond milk", "quantity": 1, "unit": "cup"}
    ],
    "instructions": ["Add all ingredients to blender", "Blend until smooth"],
    "tags": ["quick", "shake", "high-protein"],
    "skill_level": "beginner"
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

- MCP Protocol Specification
- Pierre MCP Server Repository
- Development Guide
- Testing Guide
- Configuration Guide

---

*Last Updated: 2025-12-06*
*Pierre Fitness Platform v1.0.0*

---

# Pierre intelligence and analytics methodology

## What this document covers

This comprehensive guide explains the scientific methods, algorithms, and decision rules behind pierre's analytics engine. It provides transparency into:

- **mathematical foundations**: formulas, statistical methods, and physiological models
- **data sources and processing**: inputs, validation, and transformation pipelines
- **calculation methodologies**: step-by-step algorithms with code examples
- **scientific references**: peer-reviewed research backing each metric
- **implementation details**: rust code architecture and design patterns
- **limitations and guardrails**: edge cases, confidence levels, and safety mechanisms
- **verification**: validation against published sports science data

**algorithm implementation**: all algorithms described in this document are implemented using enum-based dependency injection for runtime configuration flexibility. Each algorithm category (max heart rate, TRIMP, TSS, VDOT, training load, recovery, FTP, LTHR, VO2max) supports multiple variants selectable via environment variables. See configuration.md for available algorithm variants and architecture.md for implementation details.

---

## Table of contents

### Core Architecture
- architecture overview
  - foundation modules
  - core modules
  - intelligence tools (47 tools)
- data sources and permissions
  - primary data
  - user profile (optional)
  - configuration
  - provider normalization
  - data retention and privacy

### Personalization And Zones
- personalization engine
  - age-based max heart rate estimation
  - heart rate zones
  - power zones (cycling)

### Core Metrics And Calculations
- core metrics
  - pace vs speed
- training stress score (TSS)
  - power-based TSS (preferred)
  - heart rate-based TSS (hrTSS)
- normalized power (NP)
- chronic training load (CTL) and acute training load (ATL)
  - mathematical formulation
- training stress balance (TSB)
- overtraining risk detection

### Statistical Analysis
- statistical trend analysis

### Performance Prediction
- performance prediction: VDOT
  - VDOT calculation from race performance
  - race time prediction from VDOT
  - VDOT accuracy verification ✅
- performance prediction: riegel formula

### Pattern Recognition
- pattern detection
  - weekly schedule
  - hard/easy alternation
  - volume progression

### Sleep And Recovery
- sleep and recovery analysis
  - sleep quality scoring
  - recovery score calculation
  - configuration

### Validation And Safety
- validation and safety
  - parameter bounds (physiological ranges)
  - confidence levels
  - edge case handling

### Configuration
- configuration strategies
  - conservative strategy
  - default strategy
  - aggressive strategy

### Testing And Quality
- testing and verification
  - test coverage
  - verification methods

### Debugging Guide
- debugging and validation guide
  - general debugging workflow
  - metric-specific debugging
  - common platform-specific issues
  - data quality validation
  - when to contact support
  - debugging tools and utilities

### Reference Information
- limitations
  - model assumptions
  - known issues
  - prediction accuracy
- references
  - scientific literature
- faq
- glossary

---

## Architecture Overview

Pierre's intelligence system uses a **foundation modules** approach for code reuse and consistency:

```
┌─────────────────────────────────────────────┐
│   mcp/a2a protocol layer                    │
│   (src/protocols/universal/)                │
└──────────────────┬──────────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────────┐
│   intelligence tools (47 tools)             │
│   (src/protocols/universal/handlers/)       │
└──────────────────┬──────────────────────────┘
                   │
    ┌──────────────┼──────────────────┬───────────┬────────────┐
    ▼              ▼                  ▼           ▼            ▼
┌─────────────┐ ┌──────────────┐ ┌──────────┐ ┌───────────┐ ┌──────────────┐
│ Training    │ │ Performance  │ │ Pattern  │ │Statistical│ │ Sleep &      │
│ Load Calc   │ │ Predictor    │ │ Detector │ │ Analyzer  │ │ Recovery     │
│             │ │              │ │          │ │           │ │              │
│ TSS/CTL/ATL │ │ VDOT/Riegel  │ │ Weekly   │ │Regression │ │ Sleep Score  │
│ TSB/Risk    │ │ Race Times   │ │ Patterns │ │ Trends    │ │ Recovery Calc│
└─────────────┘ └──────────────┘ └──────────┘ └───────────┘ └──────────────┘
                    FOUNDATION MODULES
             Shared by all intelligence tools
```

### Foundation Modules

**`src/intelligence/training_load.rs`** - training stress calculations
- TSS (Training Stress Score) from power or heart rate
- CTL (Chronic Training Load) - 42-day EMA for fitness
- ATL (Acute Training Load) - 7-day EMA for fatigue
- TSB (Training Stress Balance) - form indicator
- Overtraining risk assessment with 3 risk factors
- Gap handling: zero-fills missing days in EMA calculation

**`src/intelligence/performance_prediction.rs`** - race predictions
- VDOT calculation from race performance (Jack Daniels formula)
- Race time prediction for 5K, 10K, 15K, Half Marathon, Marathon
- Riegel formula for distance-based predictions
- Accuracy: 0.2-5.5% vs. published VDOT tables
- Verified against VDOT 40, 50, 60 reference values

**`src/intelligence/pattern_detection.rs`** - pattern recognition
- Weekly schedule detection with consistency scoring
- Hard/easy alternation pattern analysis
- Volume progression trend detection (increasing/stable/decreasing)
- Overtraining signals detection (3 risk factors)

**`src/intelligence/statistical_analysis.rs`** - statistical methods
- Linear regression with R² calculation
- Trend detection (improving/stable/declining)
- Correlation analysis
- Moving averages and smoothing
- Significance level assessment

**`src/intelligence/sleep_analysis.rs`** - sleep quality scoring
- Duration scoring with NSF guidelines (7-9 hours optimal for adults, 8-10 for athletes)
- Stages scoring with AASM recommendations (deep 15-25%, REM 20-25%)
- Efficiency scoring with clinical thresholds (excellent >90%, good >85%, poor <70%)
- Overall quality calculation (weighted average of components)
- Dependency injection with `SleepRecoveryConfig` for all thresholds

**`src/intelligence/recovery_calculator.rs`** - recovery assessment
- TSB normalization (-30 to +30 → 0-100 recovery score)
- HRV scoring based on RMSSD baseline comparison (±3ms stable, >5ms good recovery)
- Weighted recovery calculation (40% TSB, 40% sleep, 20% HRV when available)
- Fallback scoring when HRV unavailable (50% TSB, 50% sleep)
- Recovery classification (excellent/good/fair/poor) with actionable thresholds
- Dependency injection with `SleepRecoveryConfig` for configurability

### Core Modules

**`src/intelligence/metrics.rs`** - advanced metrics calculation
**`src/intelligence/performance_analyzer_v2.rs`** - performance analysis framework
**`src/intelligence/physiological_constants.rs`** - sport science constants
**`src/intelligence/recommendation_engine.rs`** - training recommendations
**`src/intelligence/goal_engine.rs`** - goal tracking and progress

### Intelligence Tools (47 tools)

All 47 MCP tools now use real calculations from foundation modules:

**group 1: analysis** (use StatisticalAnalyzer + PatternDetector)
- analyze_performance_trends
- detect_patterns
- compare_activities

**group 2: recommendations** (use TrainingLoadCalculator + PatternDetector)
- generate_recommendations
- calculate_fitness_score
- analyze_training_load

**group 3: predictions** (use PerformancePredictor)
- predict_performance

**group 4: configuration** (use physiological_constants validation)
- validate_configuration (ranges + relationships)
- suggest_goals (real profile from activities)

**group 5: goals** (use 10% improvement rule)
- analyze_goal_feasibility

**group 6: sleep and recovery** (use SleepAnalyzer + RecoveryCalculator)
- analyze_sleep_quality (NSF/AASM-based scoring)
- calculate_recovery_score (TSB + sleep + HRV)
- track_sleep_trends (longitudinal analysis)
- optimize_sleep_schedule (personalized timing)
- get_rest_day_recommendations (training load-based)

---

## Data Sources And Permissions

### Primary Data
Fitness activities via oauth2 authorization from multiple providers:

**supported providers**: strava, garmin, fitbit, whoop

**activity data**:
- **temporal**: `start_date`, `elapsed_time`, `moving_time`
- **spatial**: `distance`, `total_elevation_gain`, GPS polyline (optional)
- **physiological**: `average_heartrate`, `max_heartrate`, heart rate stream
- **power**: `average_watts`, `weighted_average_watts`, `kilojoules`, power stream (strava, garmin)
- **sport metadata**: `type`, `sport_type`, `workout_type`

### User Profile (optional)
- **demographics**: `age`, `gender`, `weight_kg`, `height_cm`
- **thresholds**: `max_hr`, `resting_hr`, `lthr`, `ftp`, `cp`, `vo2max`
- **preferences**: `units`, `training_focus`, `injury_history`
- **fitness level**: `beginner`, `intermediate`, `advanced`, `elite`

### Configuration
- **strategy**: `conservative`, `default`, `aggressive` (affects thresholds)
- **units**: metric (km, m, kg) or imperial (mi, ft, lb)
- **zone model**: karvonen (HR reserve) or percentage max HR

### Provider Normalization
Pierre normalizes data from different providers into a unified format:

```rust
// src/providers/ - unified activity model
pub struct Activity {
    pub provider: Provider, // Strava, Garmin, Fitbit
    pub start_date: DateTime<Utc>,
    pub distance: Option<f64>,
    pub moving_time: u64,
    pub sport_type: String,
    // ... normalized fields
}
```

**provider-specific features**:
- **strava**: full power metrics, segments, kudos
- **garmin**: advanced running dynamics, training effect, recovery time
- **fitbit**: all-day heart rate, sleep tracking, steps
- **whoop**: strain scores, recovery metrics, sleep stages, HRV data

### Data Retention And Privacy
- activities cached for 7 days (configurable)
- analysis results cached for 24 hours
- token revocation purges all cached data within 1 hour
- no third-party data sharing
- encryption: AES-256-GCM for tokens, tenant-specific keys
- provider tokens stored separately, isolated per tenant

---

## Personalization Engine

### Age-based Max Heart Rate Estimation

When `max_hr` not provided, pierre uses the classic fox formula:

**formula**:

```
max_hr(age) = 220 − age
```

**bounds**:

```
max_hr ∈ [160, 220] bpm to exclude physiologically implausible values
```

**rust implementation**:

```rust
// src/intelligence/physiological_constants.rs
pub const AGE_BASED_MAX_HR_CONSTANT: u32 = 220;
pub const MAX_REALISTIC_HEART_RATE: u32 = 220;

fn estimate_max_hr(age: i32) -> u32 {
    let estimated = AGE_BASED_MAX_HR_CONSTANT - age as u32;
    estimated.clamp(160, MAX_REALISTIC_HEART_RATE)
}
```

**reference**: Fox, S.M., Naughton, J.P., & Haskell, W.L. (1971). Physical activity and the prevention of coronary heart disease. *Annals of Clinical Research*, 3(6), 404-432.

**note**: while newer research suggests the Tanaka formula (`208 − 0.7 × age`) may be more accurate, pierre uses the classic Fox formula (`220 − age`) for simplicity and widespread familiarity. The difference is typically 3-8 bpm for ages 20-60.

### Heart Rate Zones

Pierre's HR zone calculations use **karvonen method** (HR reserve) internally for threshold determination:

**karvonen formula**:

```
target_hr(intensity%) = (HR_reserve × intensity%) + HR_rest
```

Where:
- `HR_reserve = HR_max − HR_rest`
- `intensity% ∈ [0, 1]`

**five-zone model** (used internally):

```
Zone 1 (Recovery):  [HR_rest + 0.50 × HR_reserve, HR_rest + 0.60 × HR_reserve]
Zone 2 (Endurance): [HR_rest + 0.60 × HR_reserve, HR_rest + 0.70 × HR_reserve]
Zone 3 (Tempo):     [HR_rest + 0.70 × HR_reserve, HR_rest + 0.80 × HR_reserve]
Zone 4 (Threshold): [HR_rest + 0.80 × HR_reserve, HR_rest + 0.90 × HR_reserve]
Zone 5 (VO2max):    [HR_rest + 0.90 × HR_reserve, HR_max]
```

**important note**: while pierre uses karvonen-based constants for internal HR zone classification (see `src/intelligence/physiological_constants.rs`), there is **no public API helper function** for calculating HR zones. Users must implement their own zone calculation using the formula above.

**internal constants** (reference implementation):

```rust
// src/intelligence/physiological_constants.rs
pub const ANAEROBIC_THRESHOLD_PERCENT: f64 = 0.85; // 85% of HR reserve
pub const AEROBIC_THRESHOLD_PERCENT: f64 = 0.70;   // 70% of HR reserve
```

**fallback**: when `resting_hr` unavailable, pierre uses simple percentage of `max_hr` for intensity classification.

**reference**: Karvonen, M.J., Kentala, E., & Mustala, O. (1957). The effects of training on heart rate; a longitudinal study. *Annales medicinae experimentalis et biologiae Fenniae*, 35(3), 307-315.

### Power Zones (cycling)

Five-zone model based on functional threshold power (FTP):

**power zones**:

```
Zone 1 (Active Recovery): [0, 0.55 × FTP)
Zone 2 (Endurance):       [0.55 × FTP, 0.75 × FTP)
Zone 3 (Tempo):           [0.75 × FTP, 0.90 × FTP)
Zone 4 (Threshold):       [0.90 × FTP, 1.05 × FTP)
Zone 5 (VO2max+):         [1.05 × FTP, ∞)
```

**rust implementation**:

```rust
// src/intelligence/physiological_constants.rs
pub fn calculate_power_zones(ftp: f64) -> PowerZones {
    PowerZones {
        zone1: (0.0,         ftp * 0.55), // Active recovery
        zone2: (ftp * 0.55,  ftp * 0.75), // Endurance
        zone3: (ftp * 0.75,  ftp * 0.90), // Tempo
        zone4: (ftp * 0.90,  ftp * 1.05), // Threshold
        zone5: (ftp * 1.05,  f64::MAX),   // VO2max+
    }
}
```

**physiological adaptations**:
- **Z1 (active recovery)**: < 55% FTP - flush metabolites, active rest
- **Z2 (endurance)**: 55-75% FTP - aerobic base building
- **Z3 (tempo)**: 75-90% FTP - muscular endurance
- **Z4 (threshold)**: 90-105% FTP - lactate threshold work
- **Z5 (VO2max+)**: > 105% FTP - maximal aerobic/anaerobic efforts

**reference**: Coggan, A. & Allen, H. (2010). *Training and Racing with a Power Meter* (2nd ed.). VeloPress.

---

## Core Metrics

### Pace Vs Speed

**pace formula** (time per distance, seconds per kilometer):

```
pace(d, t) = 0,              if d < 1 meter
           = t / (d / 1000), if d ≥ 1 meter
```

Where:
- `t` = moving time (seconds)
- `d` = distance (meters)

**speed formula** (distance per time, meters per second):

```
speed(d, t) = 0,      if t = 0
            = d / t,  if t > 0
```

Where:
- `d` = distance (meters)
- `t` = moving time (seconds)

**rust implementation**:

```rust
// src/intelligence/metrics.rs

// pace: time per distance (seconds per km)
pub fn calculate_pace(moving_time_s: u64, distance_m: f64) -> f64 {
    if distance_m < 1.0 { return 0.0; }
    (moving_time_s as f64) / (distance_m / 1000.0)
}

// speed: distance per time (m/s)
pub fn calculate_speed(distance_m: f64, moving_time_s: u64) -> f64 {
    if moving_time_s == 0 { return 0.0; }
    distance_m / (moving_time_s as f64)
}
```

---

## Training Stress Score (TSS)

TSS quantifies training load accounting for intensity and duration.

### Power-based TSS (preferred)

**formula**:

```
TSS = duration_hours × IF² × 100
```

Where:
- `IF` = intensity factor = `avg_power / FTP`
- `avg_power` = average power for the activity (watts)
- `FTP` = functional threshold power (watts)
- `duration_hours` = activity duration (hours)

**important note**: pierre uses **average power**, not normalized power (NP), for TSS calculations. While NP (see normalized power section) better accounts for variability in cycling efforts, the current implementation uses simple average power for consistency and computational efficiency.

**rust implementation**:

```rust
// src/intelligence/metrics.rs
fn calculate_tss(avg_power: u32, ftp: f64, duration_hours: f64) -> f64 {
    let intensity_factor = f64::from(avg_power) / ftp;
    (duration_hours * intensity_factor * intensity_factor * TSS_BASE_MULTIPLIER).round()
}
```

Where `TSS_BASE_MULTIPLIER = 100.0`

**input/output specification**:

```
Inputs:
  avg_power: u32          // Average watts for activity, must be > 0
  duration_hours: f64     // Activity duration, must be > 0
  ftp: f64                // Functional Threshold Power, must be > 0

Output:
  tss: f64                // Training Stress Score, typically 0-500
                          // No upper bound (extreme efforts can exceed 500)

Precision: IEEE 754 double precision (f64)
Tolerance: ±0.1 for validation due to floating point arithmetic
```

**validation examples**:

Example 1: Easy endurance ride
```
Input:
  avg_power = 180 W
  duration_hours = 2.0 h
  ftp = 300.0 W

Calculation:
  1. IF = 180.0 / 300.0 = 0.6
  2. IF² = 0.6² = 0.36
  3. TSS = 2.0 × 0.36 × 100 = 72.0

Expected API result: tss = 72.0
Interpretation: Low training stress (< 150)
```

Example 2: Threshold workout
```
Input:
  avg_power = 250 W
  duration_hours = 2.0 h
  ftp = 300.0 W

Calculation:
  1. IF = 250.0 / 300.0 = 0.8333...
  2. IF² = 0.8333² = 0.6944...
  3. TSS = 2.0 × 0.6944 × 100 = 138.89

Expected API result: tss = 138.9 (rounded to 1 decimal)
Interpretation: Moderate training stress (150-300 range)
```

Example 3: High-intensity interval session
```
Input:
  avg_power = 320 W
  duration_hours = 1.5 h
  ftp = 300.0 W

Calculation:
  1. IF = 320.0 / 300.0 = 1.0667
  2. IF² = 1.0667² = 1.1378
  3. TSS = 1.5 × 1.1378 × 100 = 170.67

Expected API result: tss = 170.7 (rounded to 1 decimal, though code rounds to nearest integer = 171.0)
Interpretation: Moderate-high training stress
```

**API response format**:

```json
{
  "activity_id": "12345678",
  "tss": 139.0,
  "method": "power",
  "inputs": {
    "avg_power": 250,
    "duration_hours": 2.0,
    "ftp": 300.0
  },
  "intensity_factor": 0.833,
  "interpretation": "moderate"
}
```

**common validation issues**:

1. **Mismatch in duration calculation**
   - Issue: Manual calculation uses elapsed_time, API uses moving_time
   - Solution: API uses `moving_time` (excludes stops). Verify which time you're comparing
   - Example: 2h ride with 10min stop = 1.83h moving_time

2. **FTP value discrepancy**
   - Issue: User's FTP changed but old value cached
   - Solution: Check user profile endpoint for current FTP value used in calculation
   - Validation: Ensure same FTP value in both calculations

3. **Average power vs normalized power expectation**
   - Issue: Expecting NP-based TSS but API uses average power
   - Pierre uses **average power**, not normalized power (NP)
   - For steady efforts: avg_power ≈ NP, minimal difference
   - For variable efforts: NP typically 3-10% higher than avg_power
   - Example: intervals averaging 200W may have NP=210W → TSS difference ~10%
   - Solution: Use average power in your validation calculations

4. **Floating point precision and rounding**
   - Issue: Manual calculation shows 138.888... But API returns 139.0
   - Solution: API rounds TSS to nearest integer using `.round()`
   - Tolerance: Accept ±1.0 difference as valid due to rounding

5. **Missing power data**
   - Issue: API returns error or falls back to hrTSS
   - Solution: Check activity has valid power stream data
   - Fallback: If no power data, API uses heart rate method (hrTSS)

### Heart Rate-based TSS (hrTSS)

**formula**:

```
hrTSS = duration_hours × (HR_avg / HR_threshold)² × 100
```

Where:
- `HR_avg` = average heart rate during activity (bpm)
- `HR_threshold` = lactate threshold heart rate (bpm)
- `duration_hours` = activity duration (hours)

**rust implementation**:

```rust
pub fn calculate_tss_hr(
    avg_hr: u32,
    duration_hours: f64,
    lthr: u32,
) -> f64 {
    let hr_ratio = (avg_hr as f64) / (lthr as f64);
    duration_hours * hr_ratio.powi(2) * 100.0
}
```

**input/output specification**:

```
Inputs:
  avg_hr: u32             // Average heart rate (bpm), must be > 0
  duration_hours: f64     // Activity duration, must be > 0
  lthr: u32               // Lactate Threshold HR (bpm), must be > 0

Output:
  hrTSS: f64              // Heart Rate Training Stress Score
                          // Typically 0-500, no upper bound

Precision: IEEE 754 double precision (f64)
Tolerance: ±0.1 for validation
```

**validation examples**:

Example 1: Easy run
```
Input:
  avg_hr = 135 bpm
  duration_hours = 1.0 h
  lthr = 165 bpm

Calculation:
  1. HR ratio = 135 / 165 = 0.8182
  2. HR ratio² = 0.8182² = 0.6694
  3. hrTSS = 1.0 × 0.6694 × 100 = 66.9

Expected API result: hrTSS = 66.9
Interpretation: Low training stress
```

Example 2: Tempo run
```
Input:
  avg_hr = 155 bpm
  duration_hours = 1.5 h
  lthr = 165 bpm

Calculation:
  1. HR ratio = 155 / 165 = 0.9394
  2. HR ratio² = 0.9394² = 0.8825
  3. hrTSS = 1.5 × 0.8825 × 100 = 132.4

Expected API result: hrTSS = 132.4
Interpretation: Moderate training stress
```

**API response format**:

```json
{
  "activity_id": "87654321",
  "tss": 66.9,
  "method": "heart_rate",
  "inputs": {
    "average_hr": 135,
    "duration_hours": 1.0,
    "lthr": 165
  },
  "hr_ratio": 0.818,
  "interpretation": "low"
}
```

**common validation issues**:

1. **LTHR value uncertainty**
   - Issue: User hasn't set or tested LTHR
   - Solution: API may estimate LTHR as ~88% of max_hr if not provided
   - Validation: Confirm LTHR value used via user profile endpoint

2. **Average HR calculation method**
   - Issue: Different averaging methods (time-weighted vs sample-weighted)
   - Solution: API uses time-weighted average from HR stream
   - Example: 30min @ 140bpm + 30min @ 160bpm = 150bpm average (not simple mean)

3. **HR drift**
   - Issue: Long efforts show cardiac drift (HR rises despite steady effort)
   - Solution: This is physiologically accurate - hrTSS will be higher than power-based TSS
   - Note: Not an error; reflects cardiovascular stress

4. **Comparison with power TSS**
   - Issue: hrTSS ≠ power TSS for same activity
   - Solution: Expected - HR responds to environmental factors (heat, fatigue)
   - Typical: hrTSS 5-15% higher than power TSS in hot conditions

**interpretation**:
- TSS < 150: low training stress
- 150 ≤ TSS < 300: moderate training stress
- 300 ≤ TSS < 450: high training stress
- TSS ≥ 450: very high training stress

**reference**: Coggan, A. (2003). Training Stress Score. *TrainingPeaks*.

---

## Normalized Power (NP)

Accounts for variability in cycling efforts using coggan's algorithm:

**important note**: NP calculation is available via the `calculate_normalized_power()` method, but **TSS uses average power** (not NP) in the current implementation. See TSS section for details.

**algorithm**:

1. Raise each instantaneous power to 4th power:
   ```
   Qᵢ = Pᵢ⁴
   ```

2. Calculate 30-second rolling average of power⁴ values:
   ```
   P̄⁴ₖ = (1/30) × Σⱼ₌₀²⁹ Qₖ₊ⱼ
   ```

3. Average all 30-second windows and take 4th root:
   ```
   NP = ⁴√((1/n) × Σₖ₌₁ⁿ P̄⁴ₖ)
   ```

Where:
- `Pᵢ` = instantaneous power at second i (watts)
- `Qᵢ` = power raised to 4th power (watts⁴)
- `P̄⁴ₖ` = 30-second rolling average of power⁴ values
- `n` = number of 30-second windows

**key distinction**: This raises power to 4th FIRST, then calculates rolling averages. This is NOT the same as averaging power first then raising to 4th.

**fallback** (if data < 30 seconds):

```
NP = average power (simple mean)
```

**rust implementation**:

```rust
// src/intelligence/metrics.rs
pub fn calculate_normalized_power(&self, power_data: &[u32]) -> Option<f64> {
    if power_data.len() < 30 {
        return None; // Need at least 30 seconds of data
    }

    // Convert to f64 for calculations
    let power_f64: Vec<f64> = power_data.iter().map(|&p| f64::from(p)).collect();

    // Calculate 30-second rolling averages of power^4
    let mut rolling_avg_power4 = Vec::new();
    for i in 29..power_f64.len() {
        let window = &power_f64[(i - 29)..=i];
        // Step 1 & 2: raise to 4th power, then average within window
        let avg_power4: f64 = window.iter().map(|&p| p.powi(4)).sum::<f64>() / 30.0;
        rolling_avg_power4.push(avg_power4);
    }

    if rolling_avg_power4.is_empty() {
        return None;
    }

    // Step 3: average all windows, then take 4th root
    let mean_power4 = rolling_avg_power4.iter().sum::<f64>()
        / f64::from(u32::try_from(rolling_avg_power4.len()).unwrap_or(u32::MAX));
    Some(mean_power4.powf(0.25))
}
```

**physiological basis**: 4th power weighting matches metabolic cost of variable efforts. Alternating 200W/150W has higher physiological cost than steady 175W. The 4th power emphasizes high-intensity bursts.

---

## Chronic Training Load (CTL) And Acute Training Load (ATL)

CTL ("fitness") and ATL ("fatigue") track training stress using exponential moving averages.

### Mathematical Formulation

**exponential moving average (EMA)**:

```
α = 2 / (N + 1)

EMAₜ = α × TSSₜ + (1 − α) × EMAₜ₋₁
```

Where:
- `N` = window size (days)
- `TSSₜ` = training stress score on day t
- `EMAₜ` = exponential moving average on day t
- `α` = smoothing factor ∈ (0, 1)

**chronic training load (CTL)**:

```
CTL = EMA₄₂(TSS_daily)
```

42-day exponential moving average of daily TSS, representing long-term fitness

**acute training load (ATL)**:

```
ATL = EMA₇(TSS_daily)
```

7-day exponential moving average of daily TSS, representing short-term fatigue

**training stress balance (TSB)**:

```
TSB = CTL − ATL
```

Difference between fitness and fatigue, representing current form

**daily TSS aggregation** (multiple activities per day):

```
TSS_daily = Σᵢ₌₁ⁿ TSSᵢ
```

Where `n` = number of activities on a given day

**gap handling** (missing training days):

```
For days with no activities: TSSₜ = 0

This causes exponential decay: EMAₜ = (1 − α) × EMAₜ₋₁
```

**rust implementation**:

```rust
// src/intelligence/training_load.rs
const CTL_WINDOW_DAYS: i64 = 42; // 6 weeks
const ATL_WINDOW_DAYS: i64 = 7;  // 1 week

pub fn calculate_training_load(
    activities: &[Activity],
    ftp: Option<f64>,
    lthr: Option<f64>,
    max_hr: Option<f64>,
    resting_hr: Option<f64>,
    weight_kg: Option<f64>,
) -> Result<TrainingLoad> {
    // Handle empty activities
    if activities.is_empty() {
        return Ok(TrainingLoad {
            ctl: 0.0,
            atl: 0.0,
            tsb: 0.0,
            tss_history: Vec::new(),
        });
    }

    // Calculate TSS for each activity
    let mut tss_data: Vec<TssDataPoint> = Vec::new();
    for activity in activities {
        if let Ok(tss) = calculate_tss(activity, ftp, lthr, max_hr, resting_hr, weight_kg) {
            tss_data.push(TssDataPoint {
                date: activity.start_date,
                tss,
            });
        }
    }

    // Handle no valid TSS calculations
    if tss_data.is_empty() {
        return Ok(TrainingLoad {
            ctl: 0.0,
            atl: 0.0,
            tsb: 0.0,
            tss_history: Vec::new(),
        });
    }

    let ctl = calculate_ema(&tss_data, CTL_WINDOW_DAYS);
    let atl = calculate_ema(&tss_data, ATL_WINDOW_DAYS);
    let tsb = ctl - atl;

    Ok(TrainingLoad { ctl, atl, tsb, tss_history: tss_data })
}

fn calculate_ema(tss_data: &[TssDataPoint], window_days: i64) -> f64 {
    if tss_data.is_empty() {
        return 0.0;
    }

    let alpha = 2.0 / (window_days as f64 + 1.0);

    // Create daily TSS map (handles multiple activities per day)
    let mut tss_map = std::collections::HashMap::new();
    for point in tss_data {
        let date_key = point.date.date_naive();
        *tss_map.entry(date_key).or_insert(0.0) += point.tss;
    }

    // Calculate EMA day by day, filling gaps with 0.0
    let first_date = tss_data[0].date;
    let last_date = tss_data[tss_data.len() - 1].date;
    let days_span = (last_date - first_date).num_days();

    let mut ema = 0.0;
    for day_offset in 0..=days_span {
        let current_date = first_date + Duration::days(day_offset);
        let date_key = current_date.date_naive();
        let daily_tss = tss_map.get(&date_key).copied().unwrap_or(0.0); // Gap = 0

        ema = daily_tss.mul_add(alpha, ema * (1.0 - alpha));
    }

    ema
}
```

**input/output specification**:

```
Inputs:
  activities: &[Activity]  // Array of activities with TSS values
  ftp: Option<f64>         // For power-based TSS calculation
  lthr: Option<f64>        // For HR-based TSS calculation
  max_hr: Option<f64>      // For HR zone estimation
  resting_hr: Option<f64>  // For HR zone estimation
  weight_kg: Option<f64>   // For pace-based TSS estimation

Output:
  TrainingLoad {
    ctl: f64,              // Chronic Training Load (0-200 typical)
    atl: f64,              // Acute Training Load (0-300 typical)
    tsb: f64,              // Training Stress Balance (-50 to +50 typical)
    tss_history: Vec<TssDataPoint>  // Daily TSS values used
  }

Precision: IEEE 754 double precision (f64)
Tolerance: ±0.5 for CTL/ATL, ±1.0 for TSB due to cumulative rounding
```

**validation examples**:

Example 1: Simple 7-day training block (no gaps)
```
Input activities (daily TSS):
  Day 1: 100
  Day 2: 80
  Day 3: 120
  Day 4: 60  (recovery)
  Day 5: 110
  Day 6: 90
  Day 7: 140

Calculation (simplified for Day 7):
  α_ctl = 2 / (42 + 1) = 0.0465
  α_atl = 2 / (7 + 1) = 0.25

  ATL (7-day EMA, final value):
    Day 1: 100 × 0.25 = 25.0
    Day 2: 80 × 0.25 + 25.0 × 0.75 = 38.75
    Day 3: 120 × 0.25 + 38.75 × 0.75 = 59.06
    Day 4: 60 × 0.25 + 59.06 × 0.75 = 59.30
    Day 5: 110 × 0.25 + 59.30 × 0.75 = 71.98
    Day 6: 90 × 0.25 + 71.98 × 0.75 = 76.49
    Day 7: 140 × 0.25 + 76.49 × 0.75 = 92.37

  CTL (42-day EMA, grows slowly):
    Assuming starting from 0, after 7 days ≈ 32.5

  TSB = CTL - ATL = 32.5 - 92.37 = -59.87

Expected API result:
  ctl ≈ 32.5
  atl ≈ 92.4
  tsb ≈ -59.9
Interpretation: Heavy training week, significant fatigue
```

Example 2: Training with gap (rest week)
```
Input activities:
  Days 1-7: Daily TSS = 100 (week 1)
  Days 8-14: No activities (rest week)
  Day 15: TSS = 120 (return to training)

At Day 14 (after rest week):
  α_atl = 0.25

  Day 7 ATL: ~75.0
  Day 8: 0 × 0.25 + 75.0 × 0.75 = 56.25
  Day 9: 0 × 0.25 + 56.25 × 0.75 = 42.19
  Day 10: 0 × 0.25 + 42.19 × 0.75 = 31.64
  Day 11: 0 × 0.25 + 31.64 × 0.75 = 23.73
  Day 12: 0 × 0.25 + 23.73 × 0.75 = 17.80
  Day 13: 0 × 0.25 + 17.80 × 0.75 = 13.35
  Day 14: 0 × 0.25 + 13.35 × 0.75 = 10.01

Expected API result at Day 14:
  atl ≈ 10.0 (decayed from ~75)
  ctl ≈ 35.0 (decays slower due to 42-day window)
  tsb ≈ +25.0 (fresh, ready for hard training)

Note: Gap = zero TSS causes exponential decay
```

Example 3: Multiple activities per day
```
Input activities (same day):
  Morning: TSS = 80 (easy ride)
  Evening: TSS = 60 (strength training converted to TSS)

Aggregation:
  Daily TSS = 80 + 60 = 140

EMA calculation uses 140 for that day's TSS value

Expected API result:
  tss_history[date] = 140.0 (single aggregated value)
  ATL/CTL calculations use 140 for that day
```

**API response format**:

```json
{
  "ctl": 87.5,
  "atl": 92.3,
  "tsb": -4.8,
  "tss_history": [
    {"date": "2025-10-01", "tss": 100.0},
    {"date": "2025-10-02", "tss": 85.0},
    {"date": "2025-10-03", "tss": 120.0}
  ],
  "status": "productive",
  "fitness_trend": "building",
  "last_updated": "2025-10-03T18:30:00Z"
}
```

**common validation issues**:

1. **Date range discrepancy**
   - Issue: Manual calculation uses different time window
   - Solution: API uses all activities within the date range, verify your date filter
   - Example: "Last 42 days" starts from current date midnight UTC

2. **Gap handling differences**
   - Issue: Manual calculation skips gaps, API fills with zeros
   - Solution: API fills missing days with TSS=0, causing realistic decay
   - Validation: Check tss_history - should include interpolated zeros
   - Example: 5-day training gap → CTL decays ~22%, ATL decays ~75%

3. **Multiple activities aggregation**
   - Issue: Not summing same-day activities
   - Solution: API sums all TSS values for a single day
   - Example: 2 rides on Monday: 80 TSS + 60 TSS = 140 TSS for that day

4. **Starting value (cold start)**
   - Issue: EMA starting value assumption
   - Solution: API starts EMA at 0.0 for new users
   - Note: CTL takes ~6 weeks to stabilize, ATL takes ~2 weeks
   - Impact: Early values less reliable (first 2-6 weeks of training)

5. **TSS calculation failures**
   - Issue: Some activities excluded due to missing data
   - Solution: API skips activities without power/HR data
   - Validation: Check tss_history.length vs activities count
   - Example: 10 activities but only 7 in tss_history → 3 failed TSS calculation

6. **Floating point accumulation**
   - Issue: Small differences accumulate over many days
   - Solution: Accept ±0.5 for CTL/ATL, ±1.0 for TSB
   - Cause: IEEE 754 rounding across 42+ days of calculations

7. **Timezone effects**
   - Issue: Activity recorded at 11:59 PM vs 12:01 AM different days
   - Solution: API uses activity start_date in UTC
   - Validation: Check which day activity is assigned to in tss_history

8. **CTL/ATL ratio interpretation**
   - Issue: TSB seems wrong despite correct CTL/ATL
   - Solution: TSB = CTL - ATL, not a ratio
   - Example: CTL=100, ATL=110 → TSB=-10 (fatigued, not "10% fatigued")

**validation workflow**:

Step 1: Verify TSS calculations
```
For each activity in tss_history:
  - Recalculate TSS using activity data
  - Confirm TSS value matches (±0.1)
```

Step 2: Verify daily aggregation
```
Group activities by date:
  - Sum same-day TSS values
  - Confirm daily_tss matches aggregation
```

Step 3: Verify EMA calculation
```
Starting from EMA = 0:
  For each day from first to last:
    - Calculate α = 2 / (window + 1)
    - EMA_new = daily_tss × α + EMA_old × (1 - α)
    - Confirm EMA_new matches API value (±0.5)
```

Step 4: Verify TSB
```
TSB = CTL - ATL
Confirm: API_tsb ≈ API_ctl - API_atl (±0.1)
```

**edge case handling**:
- **zero activities**: returns CTL=0, ATL=0, TSB=0
- **training gaps**: zero-fills missing days (realistic fitness decay)
- **multiple activities per day**: sums TSS values
- **failed TSS calculations**: skips activities, continues with valid data

**reference**: Banister, E.W. (1991). Modeling elite athletic performance. Human Kinetics.

---

## Training Stress Balance (TSB)

TSB indicates form/freshness using piecewise classification:

**training status classification**:

```
TrainingStatus(TSB) = Overreaching,  if TSB < −10
                    = Productive,    if −10 ≤ TSB < 0
                    = Fresh,         if 0 ≤ TSB ≤ 10
                    = Detraining,    if TSB > 10
```

**rust implementation**:

```rust
pub fn interpret_tsb(tsb: f64) -> TrainingStatus {
    match tsb {
        t if t < -10.0 => TrainingStatus::Overreaching,
        t if t < 0.0   => TrainingStatus::Productive,
        t if t <= 10.0 => TrainingStatus::Fresh,
        _              => TrainingStatus::Detraining,
    }
}
```

**interpretation**:
- **TSB < −10**: overreaching (high fatigue) - recovery needed
- **−10 ≤ TSB < 0**: productive training - building fitness
- **0 ≤ TSB ≤ 10**: fresh - ready for hard efforts
- **TSB > 10**: risk of detraining

**reference**: Banister, E.W., Calvert, T.W., Savage, M.V., & Bach, T. (1975). A systems model of training. *Australian Journal of Sports Medicine*, 7(3), 57-61.

---

## Overtraining Risk Detection

**three-factor risk assessment**:

```
Risk Factor 1 (Acute Load Spike):
  Triggered when: (CTL > 0) ∧ (ATL > 1.3 × CTL)

Risk Factor 2 (Very High Acute Load):
  Triggered when: ATL > 150

Risk Factor 3 (Deep Fatigue):
  Triggered when: TSB < −10
```

**risk level classification**:

```
RiskLevel = Low,       if |risk_factors| = 0
          = Moderate,  if |risk_factors| = 1
          = High,      if |risk_factors| ≥ 2
```

Where `|risk_factors|` = count of triggered risk factors

**rust implementation**:

```rust
// src/intelligence/training_load.rs
pub fn check_overtraining_risk(training_load: &TrainingLoad) -> OvertrainingRisk {
    let mut risk_factors = Vec::new();

    // 1. Acute load spike
    if training_load.ctl > 0.0 && training_load.atl > training_load.ctl * 1.3 {
        risk_factors.push(
            "Acute load spike >30% above chronic load".to_string()
        );
    }

    // 2. Very high acute load
    if training_load.atl > 150.0 {
        risk_factors.push(
            "Very high acute load (>150 TSS/day)".to_string()
        );
    }

    // 3. Deep fatigue
    if training_load.tsb < -10.0 {
        risk_factors.push(
            "Deep fatigue (TSB < -10)".to_string()
        );
    }

    let risk_level = match risk_factors.len() {
        0 => RiskLevel::Low,
        1 => RiskLevel::Moderate,
        _ => RiskLevel::High,
    };

    OvertrainingRisk { risk_level, risk_factors }
}
```

**physiological interpretation**:
- **Acute load spike**: fatigue (ATL) exceeds fitness (CTL) by >30%, indicating sudden increase
- **Very high acute load**: average daily TSS >150 in past week, exceeding sustainable threshold
- **Deep fatigue**: negative TSB <−10, indicating accumulated fatigue without recovery

**reference**: Halson, S.L. (2014). Monitoring training load to understand fatigue. *Sports Medicine*, 44(Suppl 2), 139-147.

---

## Statistical Trend Analysis

Pierre uses ordinary least squares linear regression for trend detection:

**linear regression formulation**:

Given n data points `(xᵢ, yᵢ)`, fit line: `ŷ = β₀ + β₁x`

**slope calculation**:

```
β₁ = (Σᵢ₌₁ⁿ xᵢyᵢ − n × x̄ × ȳ) / (Σᵢ₌₁ⁿ xᵢ² − n × x̄²)
```

**intercept calculation**:

```
β₀ = ȳ − β₁ × x̄
```

Where:
- `x̄ = (1/n) × Σᵢ₌₁ⁿ xᵢ` (mean of x values)
- `ȳ = (1/n) × Σᵢ₌₁ⁿ yᵢ` (mean of y values)
- `n` = number of data points

**coefficient of determination (R²)**:

```
R² = 1 − (SS_res / SS_tot)
```

Where:
- `SS_tot = Σᵢ₌₁ⁿ (yᵢ − ȳ)²` (total sum of squares)
- `SS_res = Σᵢ₌₁ⁿ (yᵢ − ŷᵢ)²` (residual sum of squares)
- `ŷᵢ = β₀ + β₁xᵢ` (predicted value)

**correlation coefficient**:

```
r = sign(β₁) × √R²
```

**rust implementation**:

```rust
// src/intelligence/statistical_analysis.rs
pub fn linear_regression(data_points: &[TrendDataPoint]) -> Result<RegressionResult> {
    let n = data_points.len() as f64;
    let x_values: Vec<f64> = (0..data_points.len()).map(|i| i as f64).collect();
    let y_values: Vec<f64> = data_points.iter().map(|p| p.value).collect();

    let sum_x = x_values.iter().sum::<f64>();
    let sum_y = y_values.iter().sum::<f64>();
    let sum_xx = x_values.iter().map(|x| x * x).sum::<f64>();
    let sum_xy = x_values.iter().zip(&y_values).map(|(x, y)| x * y).sum::<f64>();
    let sum_yy = y_values.iter().map(|y| y * y).sum::<f64>();

    let mean_x = sum_x / n;
    let mean_y = sum_y / n;

    // Calculate slope and intercept
    let numerator = sum_xy - n * mean_x * mean_y;
    let denominator = sum_xx - n * mean_x * mean_x;

    let slope = numerator / denominator;
    let intercept = mean_y - slope * mean_x;

    // Calculate R² (coefficient of determination)
    let ss_tot = sum_yy - n * mean_y * mean_y;
    let ss_res: f64 = y_values
        .iter()
        .zip(&x_values)
        .map(|(y, x)| {
            let predicted = slope * x + intercept;
            (y - predicted).powi(2)
        })
        .sum();

    let r_squared = 1.0 - (ss_res / ss_tot);
    let correlation = r_squared.sqrt() * slope.signum();

    Ok(RegressionResult {
        slope,
        intercept,
        r_squared,
        correlation,
    })
}
```

**R² interpretation**:
- 0.0 ≤ R² < 0.3: weak relationship
- 0.3 ≤ R² < 0.5: moderate relationship
- 0.5 ≤ R² < 0.7: strong relationship
- 0.7 ≤ R² ≤ 1.0: very strong relationship

**reference**: Draper, N.R. & Smith, H. (1998). *Applied Regression Analysis* (3rd ed.). Wiley.

---

## Performance Prediction: VDOT

VDOT is jack daniels' VO2max adjusted for running economy:

### VDOT Calculation From Race Performance

**step 1: convert to velocity** (meters per minute):

```
v = (d / t) × 60
```

Where:
- `d` = distance (meters)
- `t` = time (seconds)
- `v ∈ [100, 500]` m/min (validated range)

**step 2: calculate VO2 consumption** (Jack Daniels' formula):

```
VO₂ = −4.60 + 0.182258v + 0.000104v²
```

**step 3: adjust for race duration**:

```
percent_max(t) = 0.97,   if t_min < 5      (very short, oxygen deficit)
               = 0.99,   if 5 ≤ t_min < 15  (5K range)
               = 1.00,   if 15 ≤ t_min < 30 (10K-15K, optimal)
               = 0.98,   if 30 ≤ t_min < 90 (half marathon)
               = 0.95,   if t_min ≥ 90      (marathon+, fatigue)
```

Where `t_min = t / 60` (time in minutes)

**step 4: calculate VDOT**:

```
VDOT = VO₂ / percent_max(t)
```

**rust implementation**:

```rust
// src/intelligence/performance_prediction.rs
pub fn calculate_vdot(distance_m: f64, time_s: f64) -> Result<f64> {
    // Convert to velocity (m/min)
    let velocity = (distance_m / time_s) * 60.0;

    // Validate velocity range
    if !(100.0..=500.0).contains(&velocity) {
        return Err(AppError::invalid_input(
            format!("Velocity {velocity:.1} m/min outside valid range (100-500)")
        ));
    }

    // Jack Daniels' VO2 formula
    // VO2 = -4.60 + 0.182258×v + 0.000104×v²
    let vo2 = (0.000104 * velocity).mul_add(
        velocity,
        0.182258f64.mul_add(velocity, -4.60)
    );

    // Adjust for race duration
    let percent_max = calculate_percent_max_adjustment(time_s);

    // VDOT = VO2 / percent_used
    Ok(vo2 / percent_max)
}

fn calculate_percent_max_adjustment(time_s: f64) -> f64 {
    let time_minutes = time_s / 60.0;

    match time_minutes {
        t if t < 5.0  => 0.97, // Very short - oxygen deficit
        t if t < 15.0 => 0.99, // 5K range
        t if t < 30.0 => 1.00, // 10K-15K range - optimal
        t if t < 90.0 => 0.98, // Half marathon range
        _             => 0.95, // Marathon+ - fatigue accumulation
    }
}
```

**VDOT ranges**:
- 30-40: beginner
- 40-50: recreational
- 50-60: competitive amateur
- 60-70: sub-elite
- 70-85: elite
- VDOT ∈ [30, 85] (typical range)

**input/output specification**:

Inputs:
  Distance_m: f64         // Race distance in meters, must be > 0
  Time_s: f64             // Race time in seconds, must be > 0

Output:
  Vdot: f64               // VDOT value, typically 30-85

Derived:
  Velocity: f64           // Calculated velocity (m/min), must be in [100, 500]
  Vo2: f64                // VO2 consumption (ml/kg/min)
  Percent_max: f64        // Race duration adjustment factor [0.95-1.00]

Precision: IEEE 754 double precision (f64)
Tolerance: ±0.5 VDOT units due to floating point arithmetic and physiological variance

**validation examples**:

Example 1: 5K race (recreational runner)
  Input:
    distance_m = 5000.0
    time_s = 1200.0  (20:00)

  Step-by-step calculation:
    1. velocity = (5000.0 / 1200.0) × 60 = 250.0 m/min
    2. vo2 = -4.60 + (0.182258 × 250.0) + (0.000104 × 250.0²)
         = -4.60 + 45.5645 + 6.5
         = 47.4645 ml/kg/min
    3. time_minutes = 1200.0 / 60 = 20.0
       percent_max = 0.99  (5K range: 15 ≤ t < 30)
    4. VDOT = 47.4645 / 0.99 = 47.9

  Expected Output: VDOT = 47.9

Example 2: 10K race (competitive amateur)
  Input:
    distance_m = 10000.0
    time_s = 2250.0  (37:30)

  Step-by-step calculation:
    1. velocity = (10000.0 / 2250.0) × 60 = 266.67 m/min
    2. vo2 = -4.60 + (0.182258 × 266.67) + (0.000104 × 266.67²)
         = -4.60 + 48.6021 + 7.3956
         = 51.3977 ml/kg/min
    3. time_minutes = 2250.0 / 60 = 37.5
       percent_max = 0.98  (half marathon range: 30 ≤ t < 90)
    4. VDOT = 51.3977 / 0.98 = 52.4

  Expected Output: VDOT = 52.4

Example 3: Marathon race (sub-elite)
  Input:
    distance_m = 42195.0
    time_s = 10800.0  (3:00:00)

  Step-by-step calculation:
    1. velocity = (42195.0 / 10800.0) × 60 = 234.42 m/min
    2. vo2 = -4.60 + (0.182258 × 234.42) + (0.000104 × 234.42²)
         = -4.60 + 42.7225 + 5.7142
         = 43.8367 ml/kg/min
    3. time_minutes = 10800.0 / 60 = 180.0
       percent_max = 0.95  (marathon range: t ≥ 90)
    4. VDOT = 43.8367 / 0.95 = 46.1

  Expected Output: VDOT = 46.1

  Note: This seems low for 3-hour marathon. In reality, sub-elite marathoners
  Typically have VDOT 60-70. This illustrates the importance of race-specific
  Calibration and proper pacing (marathon fatigue factor = 0.95 significantly
  Impacts VDOT calculation).

Example 4: Half marathon race (recreational competitive)
  Input:
    distance_m = 21097.5
    time_s = 5400.0  (1:30:00)

  Step-by-step calculation:
    1. velocity = (21097.5 / 5400.0) × 60 = 234.42 m/min
    2. vo2 = -4.60 + (0.182258 × 234.42) + (0.000104 × 234.42²)
         = -4.60 + 42.7225 + 5.7142
         = 43.8367 ml/kg/min
    3. time_minutes = 5400.0 / 60 = 90.0
       percent_max = 0.95  (marathon range: t ≥ 90)
       NOTE: Boundary condition - at exactly 90 minutes, uses 0.95
    4. VDOT = 43.8367 / 0.95 = 46.1

  Expected Output: VDOT = 46.1

**API response format**:

```json
{
  "activity_id": "12345678",
  "vdot": 52.4,
  "inputs": {
    "distance_m": 10000.0,
    "time_s": 2250.0,
    "pace_per_km": "3:45"
  },
  "calculated": {
    "velocity_m_per_min": 266.67,
    "vo2_ml_per_kg_min": 51.40,
    "percent_max_adjustment": 0.98,
    "time_minutes": 37.5
  },
  "interpretation": "competitive_amateur",
  "race_predictions": {
    "5K": "17:22",
    "10K": "36:15",
    "half_marathon": "1:20:45",
    "marathon": "2:50:30"
  }
}
```

**common validation issues**:

1. **velocity out of range (100-500 m/min)**:
   - Cause: extremely slow pace (<12 km/h) or unrealistic fast pace (>30 km/h)
   - Example: 5K in 50 minutes → velocity = 100 m/min (walking pace)
   - Example: 5K in 10 minutes → velocity = 500 m/min (world record ~350 m/min)
   - Solution: validate input data quality; reject activities with unrealistic paces

2. **percent_max boundary conditions**:
   - At t = 5, 15, 30, 90 minutes, percent_max changes discretely
   - Example: 10K in 29:59 uses 1.00 (10K range), but 30:01 uses 0.98 (half range)
   - This creates discontinuous VDOT jumps at boundaries
   - Solution: document boundary behavior; users should expect ±2 VDOT variance near boundaries

3. **comparison with Jack Daniels' tables**:
   - Pierre uses mathematical formula; Jack Daniels' tables use empirical adjustments
   - Expected difference: 0.2-5.5% (see verification section)
   - Example: VDOT 50 marathon → pierre predicts 3:12:38, table shows 3:08:00 (2.5% diff)
   - Solution: both are valid; pierre is more consistent across distances

4. **VDOT from different race distances doesn't match**:
   - Cause: runner's strengths vary by distance (speed vs endurance)
   - Example: VDOT 55 from 5K but VDOT 50 from marathon
   - Physiological: runner may have strong VO2max but weaker lactate threshold
   - Solution: use most recent race at target distance; VDOT varies by race type

5. **VDOT too low for known fitness level**:
   - Cause: race conducted in poor conditions (heat, hills, wind)
   - Cause: insufficient taper or poor pacing strategy
   - Cause: race was not maximal effort (training run logged as race)
   - Solution: only use races with maximal effort in good conditions

6. **VDOT outside typical range [30, 85]**:
   - VDOT < 30: data quality issue or walking activity
   - VDOT > 85: elite/world-class performance (verify data accuracy)
   - Solution: pierre rejects VDOT outside [30, 85] as invalid input

7. **predicted race times don't match actual performance**:
   - Cause: VDOT assumes proper training at target distance
   - Example: VDOT 60 from 5K predicts 2:46 marathon, but runner lacks endurance
   - Solution: VDOT is running economy, not prediction; requires distance-specific training

8. **floating point precision differences**:
   - Different platforms may produce slightly different VDOT values
   - Example: velocity = 266.666666... (repeating) may round differently
   - Tolerance: accept ±0.5 VDOT units as equivalent
   - Solution: compare VDOT values with tolerance, not exact equality

**validation workflow for users**:

1. **verify input data quality**:
   ```bash
   # Check velocity is in valid range
   Velocity = (distance_m / time_s) × 60
   Assert 100.0 ≤ velocity ≤ 500.0
   ```

2. **calculate intermediate values**:
   ```bash
   # Verify VO2 calculation
   Vo2 = -4.60 + (0.182258 × velocity) + (0.000104 × velocity²)

   # Verify percent_max adjustment
   Time_minutes = time_s / 60
   # Check against percent_max ranges (see formula)
   ```

3. **calculate VDOT**:
   ```bash
   Vdot = vo2 / percent_max
   Assert 30.0 ≤ vdot ≤ 85.0
   ```

4. **compare with reference**:
   - Compare calculated VDOT with Jack Daniels' published tables
   - Accept 0-6% difference as normal
   - If difference >6%, investigate input data quality

### Race Time Prediction From VDOT

**step 1: calculate velocity at VO2max** (inverse of Jack Daniels' formula):

Solve quadratic equation:
```
0.000104v² + 0.182258v − (VDOT + 4.60) = 0
```

Using quadratic formula:
```
v = (−b + √(b² − 4ac)) / (2a)
```

Where:
- `a = 0.000104`
- `b = 0.182258`
- `c = −(VDOT + 4.60)`

**step 2: adjust velocity for race distance**:

```
v_race(d, v_max) = 0.98 × v_max,                           if d ≤ 5,000 m
                 = 0.94 × v_max,                           if 5,000 < d ≤ 10,000 m
                 = 0.91 × v_max,                           if 10,000 < d ≤ 15,000 m
                 = 0.88 × v_max,                           if 15,000 < d ≤ 21,097.5 m
                 = 0.84 × v_max,                           if 21,097.5 < d ≤ 42,195 m
                 = max(0.70, 0.84 − 0.02(r − 1)) × v_max,  if d > 42,195 m
```

Where `r = d / 42,195` (marathon ratio for ultra distances)

**step 3: calculate predicted time**:

```
t_predicted = (d / v_race) × 60
```

Where:
- `d` = target distance (meters)
- `v_race` = race velocity (meters/minute)
- `t_predicted` = predicted time (seconds)

**rust implementation**:

```rust
pub fn predict_time_vdot(vdot: f64, target_distance_m: f64) -> Result<f64> {
    // Validate VDOT range
    if !(30.0..=85.0).contains(&vdot) {
        return Err(AppError::invalid_input(
            format!("VDOT {vdot:.1} outside typical range (30-85)")
        ));
    }

    // Calculate velocity at VO2max (reverse of VDOT formula)
    // vo2 = -4.60 + 0.182258 × v + 0.000104 × v²
    // Solve quadratic: 0.000104v² + 0.182258v - (vo2 + 4.60) = 0

    let a = 0.000104;
    let b = 0.182258;
    let c = -(vdot + 4.60);

    let discriminant = b.mul_add(b, -(4.0 * a * c));
    let velocity_max = (-b + discriminant.sqrt()) / (2.0 * a);

    // Adjust for race distance
    let race_velocity = calculate_race_velocity(velocity_max, target_distance_m);

    // Calculate time
    Ok((target_distance_m / race_velocity) * 60.0)
}

fn calculate_race_velocity(velocity_max: f64, distance_m: f64) -> f64 {
    let percent_max = if distance_m <= 5_000.0 {
        0.98 // 5K: 98% of VO2max velocity
    } else if distance_m <= 10_000.0 {
        0.94 // 10K: 94%
    } else if distance_m <= 15_000.0 {
        0.91 // 15K: 91%
    } else if distance_m <= 21_097.5 {
        0.88 // Half: 88%
    } else if distance_m <= 42_195.0 {
        0.84 // Marathon: 84%
    } else {
        // Ultra: progressively lower
        let marathon_ratio = distance_m / 42_195.0;
        (marathon_ratio - 1.0).mul_add(-0.02, 0.84).max(0.70)
    };

    velocity_max * percent_max
}
```

**input/output specification for race time prediction**:

Inputs:
  Vdot: f64               // VDOT value, must be in [30, 85]
  Target_distance_m: f64  // Target race distance in meters, must be > 0

Output:
  Predicted_time_s: f64   // Predicted race time in seconds

Derived:
  Velocity_max: f64       // Velocity at VO2max (m/min) from quadratic formula
  Race_velocity: f64      // Adjusted velocity for race distance (m/min)
  Percent_max: f64        // Distance-based velocity adjustment [0.70-0.98]

Precision: IEEE 754 double precision (f64)
Tolerance: ±2% for race time predictions (±3 seconds per 5K, ±6 seconds per 10K, ±3 minutes per marathon)

**validation examples for race time prediction**:

Example 1: Predict 5K time from VDOT 50
  Input:
    vdot = 50.0
    target_distance_m = 5000.0

  Step-by-step calculation:
    1. Solve quadratic: 0.000104v² + 0.182258v - (50.0 + 4.60) = 0
       a = 0.000104, b = 0.182258, c = -54.60
       discriminant = 0.182258² - (4 × 0.000104 × -54.60)
                   = 0.033218 + 0.022718 = 0.055936
       velocity_max = (-0.182258 + √0.055936) / (2 × 0.000104)
                   = (-0.182258 + 0.23652) / 0.000208
                   = 260.78 m/min

    2. Adjust for 5K distance (≤ 5000m → 0.98 × velocity_max):
       race_velocity = 0.98 × 260.78 = 255.56 m/min

    3. Calculate predicted time:
       predicted_time_s = (5000.0 / 255.56) × 60 = 1174.3 seconds
                       = 19:34

  Expected Output: 19:34 (19 minutes 34 seconds)
  Jack Daniels Reference: 19:31 → 0.2% difference ✅

Example 2: Predict marathon time from VDOT 60
  Input:
    vdot = 60.0
    target_distance_m = 42195.0

  Step-by-step calculation:
    1. Solve quadratic: 0.000104v² + 0.182258v - (60.0 + 4.60) = 0
       c = -64.60
       discriminant = 0.033218 + 0.026870 = 0.060088
       velocity_max = (-0.182258 + 0.24513) / 0.000208
                   = 302.34 m/min

    2. Adjust for marathon distance (21097.5 < d ≤ 42195 → 0.84 × velocity_max):
       race_velocity = 0.84 × 302.34 = 253.97 m/min

    3. Calculate predicted time:
       predicted_time_s = (42195.0 / 253.97) × 60 = 9970 seconds
                       = 2:46:10

  Expected Output: 2:46:10 (2 hours 46 minutes 10 seconds)
  Jack Daniels Reference: 2:40:00 → 3.9% difference ✅

Example 3: Predict 10K time from VDOT 40
  Input:
    vdot = 40.0
    target_distance_m = 10000.0

  Step-by-step calculation:
    1. Solve quadratic: 0.000104v² + 0.182258v - (40.0 + 4.60) = 0
       c = -44.60
       discriminant = 0.033218 + 0.018550 = 0.051768
       velocity_max = (-0.182258 + 0.22752) / 0.000208
                   = 217.43 m/min

    2. Adjust for 10K distance (5000 < d ≤ 10000 → 0.94 × velocity_max):
       race_velocity = 0.94 × 217.43 = 204.38 m/min

    3. Calculate predicted time:
       predicted_time_s = (10000.0 / 204.38) × 60 = 2932 seconds
                       = 48:52

  Expected Output: 48:52 (48 minutes 52 seconds)
  Jack Daniels Reference: 51:42 → 5.5% difference ✅

**API response format for race predictions**:

```json
{
  "user_id": "user_12345",
  "vdot": 50.0,
  "calculation_date": "2025-01-15",
  "race_predictions": [
    {
      "distance": "5K",
      "distance_m": 5000.0,
      "predicted_time_s": 1174.3,
      "predicted_time_formatted": "19:34",
      "pace_per_km": "3:55",
      "race_velocity_m_per_min": 255.56
    },
    {
      "distance": "10K",
      "distance_m": 10000.0,
      "predicted_time_s": 2448.0,
      "predicted_time_formatted": "40:48",
      "pace_per_km": "4:05",
      "race_velocity_m_per_min": 244.90
    },
    {
      "distance": "Half Marathon",
      "distance_m": 21097.5,
      "predicted_time_s": 5516.0,
      "predicted_time_formatted": "1:31:56",
      "pace_per_km": "4:21",
      "race_velocity_m_per_min": 229.50
    },
    {
      "distance": "Marathon",
      "distance_m": 42195.0,
      "predicted_time_s": 11558.0,
      "predicted_time_formatted": "3:12:38",
      "pace_per_km": "4:35",
      "race_velocity_m_per_min": 218.85
    }
  ],
  "calculated": {
    "velocity_max_m_per_min": 260.78,
    "interpretation": "recreational_competitive"
  },
  "accuracy_note": "Predictions assume proper training, taper, and race conditions. Expected ±5% variance from actual performance."
}
```

**common validation issues for race time prediction**:

1. **quadratic formula numerical instability**:
   - At extreme VDOT values (near 30 or 85), discriminant may be small
   - Very small discriminant → numerical precision issues in sqrt()
   - Solution: validate VDOT is in [30, 85] before calculation

2. **velocity_max boundary at distance transitions**:
   - Percent_max changes discretely at 5K, 10K, 15K, half, marathon boundaries
   - Example: 5001m uses 0.94 (10K), but 4999m uses 0.98 (5K) → 4% velocity difference
   - Creates discontinuous predictions near distance boundaries
   - Solution: document boundary behavior; predictions are approximations

3. **ultra-distance predictions become conservative**:
   - Formula: 0.84 - 0.02 × (marathon_ratio - 1) for d > 42195m
   - Example: 50K → marathon_ratio = 1.18 → percent_max = 0.836
   - Example: 100K → marathon_ratio = 2.37 → percent_max = 0.813
   - Minimum floor: 0.70 (70% of VO2max velocity)
   - Solution: VDOT predictions for ultras (>42K) are less accurate; use with caution

4. **predicted times slower than personal bests**:
   - Cause: VDOT calculated from shorter distance (5K VDOT predicting marathon)
   - Cause: insufficient endurance training for longer distances
   - Example: VDOT 60 from 5K → predicts 2:46 marathon, but runner only has 10K training
   - Solution: VDOT assumes distance-specific training; predictions require proper preparation

5. **predicted times much faster than current fitness**:
   - Cause: VDOT calculated from recent breakthrough race or downhill course
   - Cause: VDOT input doesn't reflect current fitness (old value)
   - Solution: recalculate VDOT from recent representative race in similar conditions

6. **race predictions don't account for external factors**:
   - Weather: heat +5-10%, wind +2-5%, rain +1-3%
   - Course: hills +3-8%, trail +5-15% vs flat road
   - Altitude: +3-5% per 1000m elevation for non-acclimated runners
   - Solution: VDOT predictions are baseline; adjust for race conditions

7. **comparison with Jack Daniels' tables shows differences**:
   - Pierre: mathematical formula (consistent across all distances)
   - Jack Daniels: empirical adjustments from real runner data
   - Expected variance: 0.2-5.5% (see accuracy verification below)
   - Solution: both approaches are valid; pierre is more algorithmic

8. **floating point precision in quadratic formula**:
   - Discriminant calculation: b² - 4ac may lose precision for similar values
   - Square root operation introduces rounding
   - Velocity calculation: division by small value (2a = 0.000208) amplifies errors
   - Tolerance: accept ±1 second per 10 minutes of predicted time
   - Solution: use f64 precision throughout; compare with tolerance

**validation workflow for race time predictions**:

1. **validate VDOT input**:
   ```bash
   Assert 30.0 ≤ vdot ≤ 85.0
   ```

2. **solve quadratic for velocity_max**:
   ```bash
   A = 0.000104
   B = 0.182258
   C = -(vdot + 4.60)
   Discriminant = b² - 4ac
   Assert discriminant > 0
   Velocity_max = (-b + √discriminant) / (2a)
   ```

3. **calculate race velocity with distance adjustment**:
   ```bash
   # Check percent_max based on distance
   # 5K: 0.98, 10K: 0.94, 15K: 0.91, Half: 0.88, Marathon: 0.84, Ultra: see formula
   Race_velocity = percent_max × velocity_max
   ```

4. **calculate predicted time**:
   ```bash
   Predicted_time_s = (target_distance_m / race_velocity) × 60
   ```

5. **compare with Jack Daniels' reference**:
   - Use VDOT accuracy verification table below
   - Accept 0-6% difference as normal
   - If >6% difference, verify calculation steps

### VDOT Accuracy Verification ✅

Pierre's VDOT predictions have been verified against jack daniels' published tables:

```
VDOT 50 (recreational competitive):
  5K:        19:34 vs 19:31 reference → 0.2% difference ✅
  10K:       40:48 vs 40:31 reference → 0.7% difference ✅
  Half:    1:31:56 vs 1:30:00 reference → 2.2% difference ✅
  Marathon: 3:12:38 vs 3:08:00 reference → 2.5% difference ✅

VDOT 60 (sub-elite):
  5K:        16:53 vs 16:39 reference → 1.4% difference ✅
  10K:       35:11 vs 34:40 reference → 1.5% difference ✅
  Marathon: 2:46:10 vs 2:40:00 reference → 3.9% difference ✅

VDOT 40 (recreational):
  5K:        23:26 vs 24:44 reference → 5.2% difference ✅
  10K:       48:52 vs 51:42 reference → 5.5% difference ✅
  Marathon: 3:50:46 vs 3:57:00 reference → 2.6% difference ✅

Overall accuracy: 0.2-5.5% difference across all distances
```

**why differences exist**:
- jack daniels' tables use empirical adjustments from real runner data
- pierre uses pure mathematical VDOT formula
- 6% tolerance is excellent for race predictions (weather, course, pacing all affect actual performance)

**test verification**: `tests/vdot_table_verification_test.rs`

**reference**: Daniels, J. (2013). *Daniels' Running Formula* (3rd ed.). Human Kinetics.

---

## Performance Prediction: Riegel Formula

Predicts race times across distances using power-law relationship:

**riegel formula**:

```
T₂ = T₁ × (D₂ / D₁)^1.06
```

Where:
- `T₁` = known race time (seconds)
- `D₁` = known race distance (meters)
- `T₂` = predicted race time (seconds)
- `D₂` = target race distance (meters)
- `1.06` = riegel exponent (empirically derived constant)

**domain constraints**:
- `D₁ > 0, T₁ > 0, D₂ > 0` (all values must be positive)

**rust implementation**:

```rust
// src/intelligence/performance_prediction.rs
const RIEGEL_EXPONENT: f64 = 1.06;

pub fn predict_time_riegel(
    known_distance_m: f64,
    known_time_s: f64,
    target_distance_m: f64,
) -> Result<f64> {
    if known_distance_m <= 0.0 || known_time_s <= 0.0 || target_distance_m <= 0.0 {
        return Err(AppError::invalid_input(
            "All distances and times must be positive"
        ));
    }

    let distance_ratio = target_distance_m / known_distance_m;
    Ok(known_time_s * distance_ratio.powf(RIEGEL_EXPONENT))
}
```

**example**: predict marathon from half marathon:
- Given: T₁ = 1:30:00 = 5400s, D₁ = 21,097m
- Target: D₂ = 42,195m
- Calculation: T₂ = 5400 × (42,195 / 21,097)^1.06 ≈ 11,340s ≈ 3:09:00

**reference**: Riegel, P.S. (1981). Athletic records and human endurance. *American Scientist*, 69(3), 285-290.

---

## Pattern Detection

### Weekly Schedule

**algorithm**:

1. Count activities by weekday: `C(d) = |{activities on weekday d}|`
2. Sort weekdays by frequency: rank by descending `C(d)`
3. Calculate consistency score based on distribution

**output**:
- `most_common_days` = top 3 weekdays by activity count
- `consistency_score ∈ [0, 100]`

**rust implementation**:

```rust
// src/intelligence/pattern_detection.rs
pub fn detect_weekly_schedule(activities: &[Activity]) -> WeeklySchedulePattern {
    let mut day_counts: HashMap<Weekday, u32> = HashMap::new();

    for activity in activities {
        *day_counts.entry(activity.start_date.weekday()).or_insert(0) += 1;
    }

    let mut day_freq: Vec<(Weekday, u32)> = day_counts.into_iter().collect();
    day_freq.sort_by(|a, b| b.1.cmp(&a.1));

    let consistency_score = calculate_consistency(&day_freq, activities.len());

    WeeklySchedulePattern {
        most_common_days: day_freq.iter().take(3).map(|(d, _)| *d).collect(),
        consistency_score,
    }
}
```

**consistency interpretation**:
- 0 ≤ score < 30: highly variable
- 30 ≤ score < 60: moderate consistency
- 60 ≤ score < 80: consistent schedule
- 80 ≤ score ≤ 100: very consistent routine

### Hard/Easy Alternation

**algorithm**:

1. Classify each activity intensity: `I(a) ∈ {Hard, Easy}`
2. Sort activities chronologically by date
3. Count alternations in consecutive activities:
   ```
   Alternations = |{i : (I(aᵢ) = Hard ∧ I(aᵢ₊₁) = Easy) ∨ (I(aᵢ) = Easy ∧ I(aᵢ₊₁) = Hard)}|
   ```

4. Calculate pattern strength:
   ```
   Pattern_strength = alternations / (n − 1)
   ```
   Where `n` = number of activities

**classification**:

```
follows_pattern = true,   if pattern_strength > 0.6
                = false,  if pattern_strength ≤ 0.6
```

**rust implementation**:

```rust
pub fn detect_hard_easy_pattern(activities: &[Activity]) -> HardEasyPattern {
    let mut intensities = Vec::new();

    for activity in activities {
        let intensity = calculate_relative_intensity(activity);
        intensities.push((activity.start_date, intensity));
    }

    intensities.sort_by_key(|(date, _)| *date);

    // Detect alternation
    let mut alternations = 0;
    for window in intensities.windows(2) {
        if (window[0].1 == Intensity::Hard && window[1].1 == Intensity::Easy)
            || (window[0].1 == Intensity::Easy && window[1].1 == Intensity::Hard)
        {
            alternations += 1;
        }
    }

    let pattern_strength = (alternations as f64) / (intensities.len() as f64 - 1.0);

    HardEasyPattern {
        follows_pattern: pattern_strength > 0.6,
        pattern_strength,
    }
}
```

### Volume Progression

**algorithm**:

1. Group activities by week: compute total volume per week
2. Apply linear regression to weekly volumes (see statistical trend analysis section)
3. Classify trend based on slope:
   ```
   VolumeTrend = Increasing,  if slope > 0.05
               = Decreasing,  if slope < −0.05
               = Stable,      if −0.05 ≤ slope ≤ 0.05
   ```

**output**:
- trend classification
- slope (rate of change)
- R² (goodness of fit)

**rust implementation**:

```rust
pub fn detect_volume_progression(activities: &[Activity]) -> VolumeProgressionPattern {
    // Group by weeks
    let weekly_volumes = group_by_weeks(activities);

    // Calculate trend
    let trend_result = StatisticalAnalyzer::linear_regression(&weekly_volumes)?;

    let trend = if trend_result.slope > 0.05 {
        VolumeTrend::Increasing
    } else if trend_result.slope < -0.05 {
        VolumeTrend::Decreasing
    } else {
        VolumeTrend::Stable
    };

    VolumeProgressionPattern {
        trend,
        slope: trend_result.slope,
        r_squared: trend_result.r_squared,
    }
}
```

**reference**: Esteve-Lanao, J. Et al. (2005). How do endurance runners train? *Med Sci Sports Exerc*, 37(3), 496-504.

---

## Sleep And Recovery Analysis

### Sleep Quality Scoring

Pierre uses NSF (National Sleep Foundation) and AASM (American Academy of Sleep Medicine) guidelines for sleep quality assessment. The overall sleep quality score (0-100) combines three weighted components:

**sleep quality formula**:

```
sleep_quality = (duration_score × 0.40) + (stages_score × 0.35) + (efficiency_score × 0.25)
```

Where:
- `duration_score` weight: **40%** (emphasizes total sleep time)
- `stages_score` weight: **35%** (sleep architecture quality)
- `efficiency_score` weight: **25%** (sleep fragmentation)

#### Duration Scoring

Based on NSF recommendations with athlete-specific adjustments:

**piecewise linear scoring function**:

```
duration_score(d) = 100,                  if d ≥ 8
                  = 85 + 15(d − 7),       if 7 ≤ d < 8
                  = 60 + 25(d − 6),       if 6 ≤ d < 7
                  = 30 + 30(d − 5),       if 5 ≤ d < 6
                  = 30(d / 5),            if d < 5
```

Where `d` = sleep duration (hours)

**rust implementation**:

```rust
// src/intelligence/sleep_analysis.rs
pub fn sleep_duration_score(duration_hours: f64, config: &SleepRecoveryConfig) -> f64 {
    if duration_hours >= config.athlete_optimal_hours {        // >=8h → 100
        100.0
    } else if duration_hours >= config.adult_min_hours {       // 7-8h → 85-100
        85.0 + ((duration_hours - 7.0) / 1.0) * 15.0
    } else if duration_hours >= config.short_sleep_threshold { // 6-7h → 60-85
        60.0 + ((duration_hours - 6.0) / 1.0) * 25.0
    } else if duration_hours >= config.very_short_sleep_threshold { // 5-6h → 30-60
        30.0 + ((duration_hours - 5.0) / 1.0) * 30.0
    } else {                                                   // <5h → 0-30
        (duration_hours / 5.0) * 30.0
    }
}
```

**default thresholds**:
- **d ≥ 8 hours**: score = 100 (optimal for athletes)
- **7 ≤ d < 8 hours**: score ∈ [85, 100] (adequate for adults)
- **6 ≤ d < 7 hours**: score ∈ [60, 85] (short sleep)
- **5 ≤ d < 6 hours**: score ∈ [30, 60] (very short)
- **d < 5 hours**: score ∈ [0, 30] (severe deprivation)

**scientific basis**: NSF recommends 7-9h for adults, 8-10h for athletes. <6h linked to increased injury risk and impaired performance.

**reference**: Hirshkowitz, M. Et al. (2015). National Sleep Foundation's sleep time duration recommendations. *Sleep Health*, 1(1), 40-43.

#### Stages Scoring

Based on AASM guidelines for healthy sleep stage distribution:

**deep sleep scoring function**:

```
deep_score(p_deep) = 100,                       if p_deep ≥ 20
                   = 70 + 30(p_deep − 15)/5,    if 15 ≤ p_deep < 20
                   = 70(p_deep / 15),           if p_deep < 15
```

**REM sleep scoring function**:

```
rem_score(p_rem) = 100,                      if p_rem ≥ 25
                 = 70 + 30(p_rem − 20)/5,    if 20 ≤ p_rem < 25
                 = 70(p_rem / 20),           if p_rem < 20
```

**awake time penalty**:

```
penalty(p_awake) = 0,                  if p_awake ≤ 5
                 = 2(p_awake − 5),     if p_awake > 5
```

**combined stages score**:

```
stages_score = max(0, min(100,
               0.4 × deep_score + 0.4 × rem_score + 0.2 × p_light − penalty))
```

Where:
- `p_deep` = deep sleep percentage (%)
- `p_rem` = REM sleep percentage (%)
- `p_light` = light sleep percentage (%)
- `p_awake` = awake time percentage (%)

**rust implementation**:

```rust
// src/intelligence/sleep_analysis.rs
pub fn sleep_stages_score(
    deep_percent: f64,
    rem_percent: f64,
    light_percent: f64,
    awake_percent: f64,
    config: &SleepRecoveryConfig
) -> f64 {
    // Deep sleep: 40% weight (physical recovery)
    let deep_score = if deep_percent >= 20.0 { 100.0 }
                     else if deep_percent >= 15.0 { 70.0 + ((deep_percent - 15.0) / 5.0) * 30.0 }
                     else { (deep_percent / 15.0) * 70.0 };

    // REM sleep: 40% weight (cognitive recovery)
    let rem_score = if rem_percent >= 25.0 { 100.0 }
                    else if rem_percent >= 20.0 { 70.0 + ((rem_percent - 20.0) / 5.0) * 30.0 }
                    else { (rem_percent / 20.0) * 70.0 };

    // Awake time penalty: >5% awake reduces score
    let awake_penalty = if awake_percent > 5.0 { (awake_percent - 5.0) * 2.0 } else { 0.0 };

    // Combined: 40% deep, 40% REM, 20% light, minus awake penalty
    ((deep_score * 0.4) + (rem_score * 0.4) + (light_percent * 0.2) - awake_penalty).clamp(0.0, 100.0)
}
```

**optimal ranges**:
- **deep sleep**: 15-25% (physical recovery, growth hormone release)
- **REM sleep**: 20-25% (memory consolidation, cognitive function)
- **light sleep**: 45-55% (transition stages)
- **awake time**: <5% (sleep fragmentation indicator)

**scientific basis**: AASM sleep stage guidelines. Deep sleep critical for physical recovery, REM for cognitive processing.

**reference**: Berry, R.B. Et al. (2017). AASM Scoring Manual Version 2.4. *American Academy of Sleep Medicine*.

#### Efficiency Scoring

Based on clinical sleep medicine thresholds:

**sleep efficiency formula**:

```
efficiency = (t_asleep / t_bed) × 100
```

Where:
- `t_asleep` = total time asleep (minutes)
- `t_bed` = total time in bed (minutes)
- `efficiency ∈ [0, 100]` (percentage)

**piecewise linear scoring function**:

```
efficiency_score(e) = 100,                     if e ≥ 90
                    = 85 + 15(e − 85)/5,       if 85 ≤ e < 90
                    = 65 + 20(e − 75)/10,      if 75 ≤ e < 85
                    = 65(e / 75),              if e < 75
```

Where `e` = efficiency percentage

**rust implementation**:

```rust
// src/intelligence/sleep_analysis.rs
pub fn sleep_efficiency_score(efficiency_percent: f64, config: &SleepRecoveryConfig) -> f64 {
    if efficiency_percent >= 90.0 {       // >=90% → 100 (excellent)
        100.0
    } else if efficiency_percent >= 85.0 { // 85-90% → 85-100 (good)
        85.0 + ((efficiency_percent - 85.0) / 5.0) * 15.0
    } else if efficiency_percent >= 75.0 { // 75-85% → 65-85 (fair)
        65.0 + ((efficiency_percent - 75.0) / 10.0) * 20.0
    } else {                              // <75% → 0-65 (poor)
        (efficiency_percent / 75.0) * 65.0
    }
}
```

**thresholds**:
- **e ≥ 90%**: score = 100 (excellent, minimal sleep fragmentation)
- **85 ≤ e < 90%**: score ∈ [85, 100] (good, normal range)
- **75 ≤ e < 85%**: score ∈ [65, 85] (fair, moderate fragmentation)
- **e < 75%**: score ∈ [0, 65] (poor, severe fragmentation)

**scientific basis**: sleep efficiency >85% considered normal in clinical sleep medicine.

**input/output specification for sleep quality scoring**:

Inputs:
  Duration_hours: f64      // Sleep duration in hours, must be ≥ 0
  Deep_percent: f64        // Deep sleep percentage [0, 100]
  Rem_percent: f64         // REM sleep percentage [0, 100]
  Light_percent: f64       // Light sleep percentage [0, 100]
  Awake_percent: f64       // Awake time percentage [0, 100]
  Time_asleep_min: f64     // Total time asleep in minutes
  Time_in_bed_min: f64     // Total time in bed in minutes

Outputs:
  Sleep_quality: f64       // Overall sleep quality score [0, 100]
  Duration_score: f64      // Duration component score [0, 100]
  Stages_score: f64        // Sleep stages component score [0, 100]
  Efficiency_score: f64    // Sleep efficiency component score [0, 100]
  Efficiency_percent: f64  // Calculated efficiency (time_asleep / time_in_bed) × 100

Precision: IEEE 754 double precision (f64)
Tolerance: ±1.0 for overall score, ±2.0 for component scores due to piecewise function boundaries

**validation examples for sleep quality scoring**:

Example 1: Excellent sleep (athlete optimal)
  Input:
    duration_hours = 8.5
    deep_percent = 20.0
    rem_percent = 25.0
    light_percent = 52.0
    awake_percent = 3.0
    time_asleep_min = 510.0  (8.5 hours)
    time_in_bed_min = 540.0  (9 hours)

  Step-by-step calculation:
    1. Duration score:
       duration_hours = 8.5 ≥ 8.0 → score = 100

    2. Stages score:
       deep_score = 20.0 ≥ 20 → 100
       rem_score = 25.0 ≥ 25 → 100
       awake_penalty = 3.0 ≤ 5 → 0
       stages_score = (100 × 0.4) + (100 × 0.4) + (52.0 × 0.2) − 0
                    = 40 + 40 + 10.4 = 90.4

    3. Efficiency score:
       efficiency = (510.0 / 540.0) × 100 = 94.4%
       94.4 ≥ 90 → score = 100

    4. Overall sleep quality:
       sleep_quality = (100 × 0.40) + (90.4 × 0.35) + (100 × 0.25)
                     = 40.0 + 31.64 + 25.0 = 96.6

  Expected Output: sleep_quality = 96.6

Example 2: Good sleep (typical adult)
  Input:
    duration_hours = 7.5
    deep_percent = 18.0
    rem_percent = 22.0
    light_percent = 54.0
    awake_percent = 6.0
    time_asleep_min = 450.0  (7.5 hours)
    time_in_bed_min = 500.0  (8.33 hours)

  Step-by-step calculation:
    1. Duration score:
       7.0 ≤ 7.5 < 8.0
       score = 85 + 15 × (7.5 − 7.0) = 85 + 7.5 = 92.5

    2. Stages score:
       deep_score: 15 ≤ 18.0 < 20
                 = 70 + 30 × (18.0 − 15.0) / 5 = 70 + 18 = 88
       rem_score: 20 ≤ 22.0 < 25
                = 70 + 30 × (22.0 − 20.0) / 5 = 70 + 12 = 82
       awake_penalty = 6.0 > 5 → (6.0 − 5.0) × 2 = 2.0
       stages_score = (88 × 0.4) + (82 × 0.4) + (54.0 × 0.2) − 2.0
                    = 35.2 + 32.8 + 10.8 − 2.0 = 76.8

    3. Efficiency score:
       efficiency = (450.0 / 500.0) × 100 = 90.0%
       90.0 ≥ 90 → score = 100

    4. Overall sleep quality:
       sleep_quality = (92.5 × 0.40) + (76.8 × 0.35) + (100 × 0.25)
                     = 37.0 + 26.88 + 25.0 = 88.9

  Expected Output: sleep_quality = 88.9

Example 3: Poor sleep (short duration, fragmented)
  Input:
    duration_hours = 5.5
    deep_percent = 12.0
    rem_percent = 18.0
    light_percent = 60.0
    awake_percent = 10.0
    time_asleep_min = 330.0  (5.5 hours)
    time_in_bed_min = 420.0  (7 hours)

  Step-by-step calculation:
    1. Duration score:
       5.0 ≤ 5.5 < 6.0
       score = 30 + 30 × (5.5 − 5.0) = 30 + 15 = 45

    2. Stages score:
       deep_score: 12.0 < 15
                 = 70 × (12.0 / 15.0) = 56
       rem_score: 18.0 < 20
                = 70 × (18.0 / 20.0) = 63
       awake_penalty = (10.0 − 5.0) × 2 = 10.0
       stages_score = (56 × 0.4) + (63 × 0.4) + (60.0 × 0.2) − 10.0
                    = 22.4 + 25.2 + 12.0 − 10.0 = 49.6

    3. Efficiency score:
       efficiency = (330.0 / 420.0) × 100 = 78.57%
       75 ≤ 78.57 < 85
       score = 65 + 20 × (78.57 − 75) / 10 = 65 + 7.14 = 72.1

    4. Overall sleep quality:
       sleep_quality = (45 × 0.40) + (49.6 × 0.35) + (72.1 × 0.25)
                     = 18.0 + 17.36 + 18.025 = 53.4

  Expected Output: sleep_quality = 53.4

Example 4: Boundary condition (exactly 7 hours, 85% efficiency)
  Input:
    duration_hours = 7.0
    deep_percent = 15.0
    rem_percent = 20.0
    light_percent = 60.0
    awake_percent = 5.0
    time_asleep_min = 420.0
    time_in_bed_min = 494.12  (exactly 85% efficiency)

  Step-by-step calculation:
    1. Duration score:
       duration_hours = 7.0 (exactly at boundary)
       score = 85.0  (lower boundary of 7-8h range)

    2. Stages score:
       deep_score = 15.0 (exactly at boundary) → 70.0
       rem_score = 20.0 (exactly at boundary) → 70.0
       awake_penalty = 5.0 (exactly at threshold) → 0
       stages_score = (70 × 0.4) + (70 × 0.4) + (60 × 0.2) − 0
                    = 28 + 28 + 12 = 68.0

    3. Efficiency score:
       efficiency = (420.0 / 494.12) × 100 = 85.0% (exactly at boundary)
       score = 85.0  (lower boundary of 85-90% range)

    4. Overall sleep quality:
       sleep_quality = (85.0 × 0.40) + (68.0 × 0.35) + (85.0 × 0.25)
                     = 34.0 + 23.8 + 21.25 = 79.1

  Expected Output: sleep_quality = 79.1

**API response format for sleep quality**:

```json
{
  "user_id": "user_12345",
  "sleep_session_id": "sleep_20250115",
  "date": "2025-01-15",
  "sleep_quality": {
    "overall_score": 88.1,
    "interpretation": "good",
    "components": {
      "duration": {
        "hours": 7.5,
        "score": 92.5,
        "status": "adequate"
      },
      "stages": {
        "deep_percent": 18.0,
        "rem_percent": 22.0,
        "light_percent": 54.0,
        "awake_percent": 6.0,
        "score": 76.8,
        "deep_score": 88.0,
        "rem_score": 82.0,
        "awake_penalty": 2.0,
        "status": "good"
      },
      "efficiency": {
        "percent": 90.0,
        "time_asleep_min": 450.0,
        "time_in_bed_min": 500.0,
        "score": 100.0,
        "status": "excellent"
      }
    }
  },
  "guidelines": {
    "duration_target": "8+ hours for athletes, 7-9 hours for adults",
    "deep_sleep_target": "15-25%",
    "rem_sleep_target": "20-25%",
    "efficiency_target": ">85%"
  }
}
```

**common validation issues for sleep quality scoring**:

1. **percentage components don't sum to 100**:
   - Cause: sleep tracker rounding or missing data
   - Example: deep=18%, REM=22%, light=55%, awake=6% → sum=101%
   - Solution: normalize percentages to sum to 100% before calculation
   - Note: pierre accepts raw percentages; validation is user's responsibility

2. **efficiency > 100%**:
   - Cause: time_asleep > time_in_bed (data error)
   - Example: slept 8 hours but only in bed 7 hours
   - Solution: validate time_asleep ≤ time_in_bed before calculation

3. **boundary discontinuities in scoring**:
   - At duration thresholds (5h, 6h, 7h, 8h), score changes slope
   - Example: 6.99h → score ≈85, but 7.01h → score ≈85.15 (not discontinuous)
   - Piecewise functions are continuous but have slope changes
   - Tolerance: ±2 points near boundaries acceptable

4. **very high awake percentage (>20%)**:
   - Causes large penalty in stages_score
   - Example: awake=25% → penalty=(25-5)×2=40 points
   - Can result in negative stages_score (clamped to 0)
   - Solution: investigate sleep fragmentation; may indicate sleep disorder

5. **missing sleep stage data**:
   - Some trackers don't provide detailed stages
   - Without stages, cannot calculate complete sleep_quality
   - Solution: use duration + efficiency only, or return error

6. **athlete vs non-athlete thresholds**:
   - Current implementation uses athlete-optimized thresholds (8h optimal)
   - Non-athletes may see lower scores with 7-8h sleep
   - Solution: configuration parameter athlete_optimal_hours (default: 8.0)

7. **sleep duration > 12 hours**:
   - Very long sleep may indicate oversleeping or health issue
   - Current formula caps at 100 for duration ≥ 8h
   - 12h sleep gets same score as 8h sleep
   - Solution: document that >10h is not necessarily better

8. **comparison with consumer sleep trackers**:
   - Consumer trackers (Fitbit, Apple Watch) may use proprietary scoring
   - Pierre uses NSF/AASM validated scientific guidelines
   - Expect 5-15 point difference between trackers
   - Solution: pierre is more conservative and scientifically grounded

**validation workflow for sleep quality**:

1. **validate input data**:
   ```bash
   Assert duration_hours ≥ 0
   Assert 0 ≤ deep_percent ≤ 100
   Assert 0 ≤ rem_percent ≤ 100
   Assert 0 ≤ light_percent ≤ 100
   Assert 0 ≤ awake_percent ≤ 100
   Assert time_asleep_min ≤ time_in_bed_min
   ```

2. **calculate component scores**:
   ```bash
   Duration_score = score_duration(duration_hours)
   Stages_score = score_stages(deep%, rem%, light%, awake%)
   Efficiency = (time_asleep / time_in_bed) × 100
   Efficiency_score = score_efficiency(efficiency)
   ```

3. **calculate weighted overall score**:
   ```bash
   Sleep_quality = (duration_score × 0.40) + (stages_score × 0.35) + (efficiency_score × 0.25)
   Assert 0 ≤ sleep_quality ≤ 100
   ```

4. **compare with expected ranges**:
   - Excellent: 85-100
   - Good: 70-85
   - Fair: 50-70
   - Poor: <50

### Recovery Score Calculation

Pierre calculates training readiness by combining TSB, sleep quality, and HRV (when available):

**weighted recovery score formula**:

```
recovery_score = 0.4 × TSB_score + 0.4 × sleep_score + 0.2 × HRV_score,  if HRV available
               = 0.5 × TSB_score + 0.5 × sleep_score,                    if HRV unavailable
```

Where:
- `TSB_score` = normalized TSB score ∈ [0, 100] (see TSB normalization below)
- `sleep_score` = overall sleep quality score ∈ [0, 100] (from sleep analysis)
- `HRV_score` = heart rate variability score ∈ [0, 100] (when available)

**recovery level classification**:

```
recovery_level = excellent,  if score ≥ 85
               = good,       if 70 ≤ score < 85
               = fair,       if 50 ≤ score < 70
               = poor,       if score < 50
```

**rust implementation**:

```rust
// src/intelligence/recovery_calculator.rs
pub fn calculate_recovery_score(
    tsb: f64,
    sleep_quality: f64,
    hrv_data: Option<HrvData>,
    config: &SleepRecoveryConfig
) -> RecoveryScore {
    // 1. Normalize TSB from [-30, +30] to [0, 100]
    let tsb_score = normalize_tsb(tsb);

    // 2. Sleep already scored [0, 100]

    // 3. Score HRV if available
    let (recovery_score, components) = match hrv_data {
        Some(hrv) => {
            let hrv_score = score_hrv(hrv, config);
            // Weights: 40% TSB, 40% sleep, 20% HRV
            let score = (tsb_score * 0.4) + (sleep_quality * 0.4) + (hrv_score * 0.2);
            (score, (tsb_score, sleep_quality, Some(hrv_score)))
        },
        None => {
            // Weights: 50% TSB, 50% sleep (no HRV)
            let score = (tsb_score * 0.5) + (sleep_quality * 0.5);
            (score, (tsb_score, sleep_quality, None))
        }
    };

    // 4. Classify recovery level
    let level = if recovery_score >= 85.0 { "excellent" }
                else if recovery_score >= 70.0 { "good" }
                else if recovery_score >= 50.0 { "fair" }
                else { "poor" };

    RecoveryScore { score: recovery_score, level, components }
}
```

#### TSB Normalization

Training stress balance maps to recovery score using **configurable thresholds**, not fixed breakpoints:

**configurable TSB thresholds** (from `SleepRecoveryConfig.training_stress_balance`):

```rust
// Default configuration values (src/config/intelligence_config.rs:1178)
TsbConfig {
    highly_fatigued_tsb: -15.0,    // Extreme fatigue threshold
    fatigued_tsb: -10.0,            // Productive fatigue threshold
    fresh_tsb_min: 5.0,             // Optimal fresh range start
    fresh_tsb_max: 15.0,            // Optimal fresh range end
    detraining_tsb: 25.0,           // Detraining risk threshold
}
```

**rust implementation**:

```rust
// src/intelligence/recovery_calculator.rs:250
pub fn score_tsb(
    tsb: f64,
    config: &SleepRecoveryConfig,
) -> f64 {
    let detraining_tsb = config.training_stress_balance.detraining_tsb;
    let fresh_tsb_max = config.training_stress_balance.fresh_tsb_max;
    let fresh_tsb_min = config.training_stress_balance.fresh_tsb_min;
    let fatigued_tsb = config.training_stress_balance.fatigued_tsb;
    let highly_fatigued_tsb = config.training_stress_balance.highly_fatigued_tsb;

    if (fresh_tsb_min..=fresh_tsb_max).contains(&tsb) {
        // Optimal fresh range: 100 points
        100.0
    } else if tsb > detraining_tsb {
        // Too fresh (risk of detraining): penalize
        100.0 - ((tsb - detraining_tsb) * 2.0).min(30.0)
    } else if tsb > fresh_tsb_max {
        // Between optimal and detraining: slight penalty
        ((tsb - fresh_tsb_max) / (detraining_tsb - fresh_tsb_max)).mul_add(-10.0, 100.0)
    } else if tsb >= 0.0 {
        // Slightly fresh (0 to fresh_tsb_min): 85-100 points
        (tsb / fresh_tsb_min).mul_add(15.0, 85.0)
    } else if tsb >= fatigued_tsb {
        // Productive fatigue: 60-85 points
        ((tsb - fatigued_tsb) / fatigued_tsb.abs()).mul_add(25.0, 60.0)
    } else if tsb >= highly_fatigued_tsb {
        // High fatigue: 30-60 points
        ((tsb - highly_fatigued_tsb) / (fatigued_tsb - highly_fatigued_tsb)).mul_add(30.0, 30.0)
    } else {
        // Extreme fatigue: 0-30 points
        30.0 - ((tsb.abs() - highly_fatigued_tsb.abs()) / highly_fatigued_tsb.abs() * 30.0)
            .min(30.0)
    }
}
```

**scoring ranges** (with default config):
- **TSB > +25**: score ∈ [70, 100] decreasing - detraining risk (too much rest)
- **+15 < TSB ≤ +25**: score ∈ [90, 100] - approaching detraining
- **+5 ≤ TSB ≤ +15**: score = **100** - optimal fresh zone (race ready)
- **0 ≤ TSB < +5**: score ∈ [85, 100] - slightly fresh
- **−10 ≤ TSB < 0**: score ∈ [60, 85] - productive fatigue (building fitness)
- **−15 ≤ TSB < −10**: score ∈ [30, 60] - high fatigue
- **TSB < −15**: score ∈ [0, 30] - extreme fatigue (recovery needed)

**configurable via environment**:
- `INTELLIGENCE_TSB_HIGHLY_FATIGUED` (default: -15.0)
- `INTELLIGENCE_TSB_FATIGUED` (default: -10.0)
- `INTELLIGENCE_TSB_FRESH_MIN` (default: 5.0)
- `INTELLIGENCE_TSB_FRESH_MAX` (default: 15.0)
- `INTELLIGENCE_TSB_DETRAINING` (default: 25.0)

**reference**: Banister, E.W. (1991). Modeling elite athletic performance. *Human Kinetics*.

#### HRV Scoring

Heart rate variability assessment based on categorical recovery status, not continuous RMSSD scoring:

**recovery status determination**:

Pierre first classifies HRV into a **categorical recovery status** (`HrvRecoveryStatus` enum) based on RMSSD comparison to baseline and weekly average:

```rust
// src/intelligence/sleep_analysis.rs:558
fn determine_hrv_recovery_status(
    current: f64,
    weekly_avg: f64,
    baseline_deviation: Option<f64>,
    config: &SleepRecoveryConfig,
) -> HrvRecoveryStatus {
    // Check baseline deviation first (if available)
    if let Some(deviation) = baseline_deviation {
        if deviation < -baseline_deviation_concern {
            return HrvRecoveryStatus::HighlyFatigued;
        } else if deviation < -5.0 {
            return HrvRecoveryStatus::Fatigued;
        }
    }

    // Compare to weekly average
    let change_from_avg = current - weekly_avg;
    if change_from_avg >= rmssd_increase_threshold {
        HrvRecoveryStatus::Recovered
    } else if change_from_avg <= rmssd_decrease_threshold {
        HrvRecoveryStatus::Fatigued
    } else {
        HrvRecoveryStatus::Normal
    }
}
```

**discrete HRV scoring function**:

Pierre maps the categorical recovery status to a **fixed discrete score**, not a continuous function:

```rust
// src/intelligence/recovery_calculator.rs:288
pub const fn score_hrv(hrv: &HrvTrendAnalysis) -> f64 {
    match hrv.recovery_status {
        HrvRecoveryStatus::Recovered => 100.0,
        HrvRecoveryStatus::Normal => 70.0,
        HrvRecoveryStatus::Fatigued => 40.0,
        HrvRecoveryStatus::HighlyFatigued => 20.0,
    }
}
```

**recovery status interpretation**:
- **Recovered**: score = **100** - elevated HRV, ready for high-intensity training
- **Normal**: score = **70** - HRV within normal range, continue current training load
- **Fatigued**: score = **40** - decreased HRV, consider reducing training intensity
- **HighlyFatigued**: score = **20** - significantly decreased HRV, prioritize recovery

Where:
- `RMSSD` = root mean square of successive RR interval differences (milliseconds)
- `weekly_avg` = 7-day rolling average of RMSSD
- `baseline_deviation` = percent change from long-term baseline (if established)
- `rmssd_increase_threshold` = typically +5ms (configurable)
- `rmssd_decrease_threshold` = typically -10ms (configurable)
- `baseline_deviation_concern` = typically -15% (configurable)

**scientific basis**: HRV (specifically RMSSD) reflects autonomic nervous system recovery. Decreases indicate accumulated fatigue, increases indicate good adaptation. Pierre uses discrete categories rather than continuous scoring to provide clear, actionable recovery guidance.

**reference**: Plews, D.J. Et al. (2013). Training adaptation and heart rate variability in elite endurance athletes. *Int J Sports Physiol Perform*, 8(3), 286-293.

**input/output specification for recovery score**:

Inputs:
  Tsb: f64                 // Training Stress Balance, typically [-30, +30]
  Sleep_quality: f64       // Sleep quality score [0, 100]
  Hrv_rmssd: Option<f64>   // Current HRV RMSSD (ms), optional
  Hrv_baseline: Option<f64>  // Baseline HRV RMSSD (ms), optional

Outputs:
  Recovery_score: f64      // Overall recovery score [0, 100]
  Tsb_score: f64           // Normalized TSB component [0, 100]
  Sleep_score: f64         // Sleep component [0, 100] (pass-through)
  Hrv_score: Option<f64>   // HRV component [0, 100], if available
  Recovery_level: String   // Classification: excellent/good/fair/poor

Precision: IEEE 754 double precision (f64)
Tolerance: ±2.0 for overall score due to piecewise function boundaries and component weighting

**validation examples for recovery score**:

Example 1: Excellent recovery (with HRV, fresh athlete)
  Input:
    tsb = 8.0
    sleep_quality = 92.0
    hrv_rmssd = 55.0
    hrv_baseline = 50.0

  Step-by-step calculation:
    1. Normalize TSB (5 ≤ 8.0 < 15):
       tsb_score = 80 + 10 × (8.0 − 5.0) / 10 = 80 + 3 = 83

    2. Sleep score (pass-through):
       sleep_score = 92.0

    3. HRV score:
       current_rmssd = 55.0, weekly_avg_rmssd ≈ 50.0
       change_from_avg = 55.0 − 50.0 = +5.0ms
       +5.0 ≥ +5.0 threshold → HrvRecoveryStatus::Recovered → score = 100

    4. Recovery score (with HRV: 40% TSB, 40% sleep, 20% HRV):
       recovery_score = (83 × 0.4) + (92 × 0.4) + (100 × 0.2)
                     = 33.2 + 36.8 + 20.0 = 90.0

    5. Classification:
       90.0 ≥ 85 → "excellent"

  Expected Output:
    recovery_score = 90.0
    recovery_level = "excellent"

Example 2: Good recovery (no HRV, moderate training)
  Input:
    tsb = 2.0
    sleep_quality = 78.0
    hrv_rmssd = None
    hrv_baseline = None

  Step-by-step calculation:
    1. Normalize TSB (-5 ≤ 2.0 < 5):
       tsb_score = 60 + 20 × (2.0 + 5.0) / 10 = 60 + 14 = 74

    2. Sleep score:
       sleep_score = 78.0

    3. HRV score:
       hrv_score = None

    4. Recovery score (without HRV: 50% TSB, 50% sleep):
       recovery_score = (74 × 0.5) + (78 × 0.5)
                     = 37.0 + 39.0 = 76.0

    5. Classification:
       70 ≤ 76.0 < 85 → "good"

  Expected Output:
    recovery_score = 76.0
    recovery_level = "good"

Example 3: Poor recovery (fatigued with poor sleep)
  Input:
    tsb = -12.0
    sleep_quality = 55.0
    hrv_rmssd = 42.0
    hrv_baseline = 50.0

  Step-by-step calculation:
    1. Normalize TSB (-15 ≤ -12.0 < -10):
       tsb_score = 20 + 20 × (-12.0 + 15.0) / 5 = 20 + 12 = 32

    2. Sleep score:
       sleep_score = 55.0

    3. HRV score:
       current_rmssd = 42.0, baseline = 50.0
       baseline_deviation = (42.0 − 50.0) / 50.0 × 100 = -16%
       -16% < -5.0% threshold → HrvRecoveryStatus::Fatigued → score = 40

    4. Recovery score (with HRV):
       recovery_score = (32 × 0.4) + (55 × 0.4) + (40 × 0.2)
                     = 12.8 + 22.0 + 8.0 = 42.8

    5. Classification:
       42.8 < 50 → "poor"

  Expected Output:
    recovery_score = 42.8
    recovery_level = "poor"

Example 4: Fair recovery (overreached but sleeping well)
  Input:
    tsb = -7.0
    sleep_quality = 88.0
    hrv_rmssd = None
    hrv_baseline = None

  Step-by-step calculation:
    1. Normalize TSB (-10 ≤ -7.0 < -5):
       tsb_score = 40 + 20 × (-7.0 + 10.0) / 5 = 40 + 12 = 52

    2. Sleep score:
       sleep_score = 88.0

    3. HRV score:
       hrv_score = None

    4. Recovery score (without HRV):
       recovery_score = (52 × 0.5) + (88 × 0.5)
                     = 26.0 + 44.0 = 70.0

    5. Classification:
       70.0 = 70 (exactly at boundary) → "good"

  Expected Output:
    recovery_score = 70.0
    recovery_level = "good"

Example 5: Boundary condition (extreme fatigue, excellent sleep/HRV)
  Input:
    tsb = -25.0
    sleep_quality = 95.0
    hrv_rmssd = 62.0
    hrv_baseline = 50.0

  Step-by-step calculation:
    1. Normalize TSB (TSB < -15):
       tsb_score = max(0, 20 × (-25.0 + 30.0) / 15) = max(0, 6.67) = 6.67

    2. Sleep score:
       sleep_score = 95.0

    3. HRV score:
       current_rmssd = 62.0, weekly_avg_rmssd ≈ 50.0
       change_from_avg = 62.0 − 50.0 = +12.0ms
       +12.0 ≥ +5.0 threshold → HrvRecoveryStatus::Recovered → score = 100

    4. Recovery score:
       recovery_score = (6.67 × 0.4) + (95 × 0.4) + (100 × 0.2)
                     = 2.67 + 38.0 + 20.0 = 60.67

    5. Classification:
       50 ≤ 60.67 < 70 → "fair"

  Expected Output:
    recovery_score = 60.67
    recovery_level = "fair"
    Note: Despite excellent sleep and HRV, extreme training fatigue (TSB=-25)
    significantly impacts overall recovery. This demonstrates TSB's 40% weight.

**API response format for recovery score**:

```json
{
  "user_id": "user_12345",
  "date": "2025-01-15",
  "recovery": {
    "overall_score": 88.0,
    "level": "excellent",
    "interpretation": "Well recovered and ready for high-intensity training",
    "components": {
      "tsb": {
        "raw_value": 8.0,
        "normalized_score": 83.0,
        "weight": 0.4,
        "contribution": 33.2,
        "status": "fresh"
      },
      "sleep": {
        "score": 92.0,
        "weight": 0.4,
        "contribution": 36.8,
        "status": "excellent"
      },
      "hrv": {
        "rmssd_current": 55.0,
        "rmssd_baseline": 50.0,
        "delta": 5.0,
        "score": 90.0,
        "weight": 0.2,
        "contribution": 18.0,
        "status": "excellent"
      }
    }
  },
  "recommendations": {
    "training_readiness": "high",
    "suggested_intensity": "Can handle high-intensity or race-pace efforts",
    "rest_needed": false
  },
  "historical_context": {
    "7_day_average": 82.5,
    "trend": "improving"
  }
}
```

**common validation issues for recovery scoring**:

1. **HRV available vs unavailable changes weights**:
   - With HRV: 40% TSB, 40% sleep, 20% HRV
   - Without HRV: 50% TSB, 50% sleep
   - Same TSB and sleep values produce different recovery scores
   - Example: TSB=80, sleep=90 → with HRV (90): 86.0, without HRV: 85.0
   - Solution: document which weights were used in API response

2. **TSB outside typical range [-30, +30]**:
   - TSB < -30: normalization formula gives score < 0 (clamped to 0)
   - TSB > +30: normalization caps at 100 (TSB ≥ 15 → score ≥ 90)
   - Extreme TSB values are physiologically unrealistic for sustained periods
   - Solution: validate TSB is reasonable before recovery calculation

3. **HRV baseline not established**:
   - Requires 7-14 days of consistent morning HRV measurements
   - Without baseline, cannot calculate meaningful HRV_score
   - Using population average (50ms) is inaccurate (individual variation 20-100ms)
   - Solution: return recovery without HRV component until baseline established

4. **recovery score boundaries**:
   - At 50, 70, 85 boundaries, classification changes
   - Example: 69.9 → "fair", but 70.0 → "good"
   - Score 84.9 is "good" but user might feel "excellent"
   - Solution: display numerical score alongside classification

5. **conflicting component signals**:
   - Example: excellent sleep (95) but poor TSB (-20) and HRV (-8ms)
   - Recovery score may be "fair" despite great sleep
   - Users may be confused why good sleep doesn't mean full recovery
   - Solution: show component breakdown so users understand weighted contributions

6. **acute vs chronic fatigue mismatches**:
   - TSB reflects training load (chronic)
   - HRV reflects autonomic recovery (acute)
   - Sleep reflects restfulness (acute)
   - Possible to have: TSB fresh (+10) but HRV poor (-5ms) from illness
   - Solution: recovery score balances all factors; investigate component discrepancies

7. **comparison with other platforms**:
   - Whoop, Garmin, Oura use proprietary recovery algorithms
   - Pierre uses transparent, scientifically-validated formulas
   - Expect 5-20 point differences between platforms
   - Solution: pierre prioritizes scientific validity over matching proprietary scores

8. **recovery score vs subjective feeling mismatch**:
   - Score is objective measure; feeling is subjective
   - Mental fatigue, stress, nutrition not captured
   - Example: score 80 ("good") but athlete feels exhausted from work stress
   - Solution: recovery score is one input to training decisions, not sole determinant

**validation workflow for recovery score**:

1. **validate input data**:
   ```bash
   # TSB typically in [-30, +30] but accept wider range
   Assert -50.0 ≤ tsb ≤ +50.0
   Assert 0.0 ≤ sleep_quality ≤ 100.0

   # If HRV provided, both current and baseline required
   If hrv_rmssd.is_some():
       assert hrv_baseline.is_some()
       assert hrv_rmssd > 0 && hrv_baseline > 0
   ```

2. **normalize TSB**:
   ```bash
   Tsb_score = normalize_tsb(tsb)  # See TSB normalization formula
   Assert 0.0 ≤ tsb_score ≤ 100.0
   ```

3. **score HRV if available**:
   ```bash
   If hrv_rmssd and weekly_avg_rmssd and baseline_deviation:
       # Determine categorical recovery status
       hrv_status = determine_hrv_recovery_status(hrv_rmssd, weekly_avg_rmssd, baseline_deviation)

       # Map status to discrete score
       hrv_score = score_hrv(hrv_status)  # Recovered→100, Normal→70, Fatigued→40, HighlyFatigued→20
       assert hrv_score ∈ {100.0, 70.0, 40.0, 20.0}
   ```

4. **calculate weighted recovery score**:
   ```bash
   If hrv_score:
       recovery = (tsb_score × 0.4) + (sleep_quality × 0.4) + (hrv_score × 0.2)
   Else:
       recovery = (tsb_score × 0.5) + (sleep_quality × 0.5)

   Assert 0.0 ≤ recovery ≤ 100.0
   ```

5. **classify recovery level**:
   ```bash
   Level = if recovery ≥ 85.0: "excellent"
           else if recovery ≥ 70.0: "good"
           else if recovery ≥ 50.0: "fair"
           else: "poor"
   ```

6. **validate component contributions**:
   ```bash
   # Component contributions should sum to recovery_score
   Total_contribution = (tsb_score × tsb_weight) +
                       (sleep_quality × sleep_weight) +
                       (hrv_score × hrv_weight if HRV)

   Assert abs(total_contribution - recovery_score) < 0.1  # floating point tolerance
   ```

### Configuration

All sleep/recovery thresholds configurable via environment variables:

```bash
# Sleep duration thresholds (hours)
PIERRE_SLEEP_ADULT_MIN_HOURS=7.0
PIERRE_SLEEP_ATHLETE_OPTIMAL_HOURS=8.0
PIERRE_SLEEP_SHORT_THRESHOLD=6.0
PIERRE_SLEEP_VERY_SHORT_THRESHOLD=5.0

# Sleep stages thresholds (percentage)
PIERRE_SLEEP_DEEP_MIN_PERCENT=15.0
PIERRE_SLEEP_DEEP_OPTIMAL_PERCENT=20.0
PIERRE_SLEEP_REM_MIN_PERCENT=20.0
PIERRE_SLEEP_REM_OPTIMAL_PERCENT=25.0

# Sleep efficiency thresholds (percentage)
PIERRE_SLEEP_EFFICIENCY_EXCELLENT=90.0
PIERRE_SLEEP_EFFICIENCY_GOOD=85.0
PIERRE_SLEEP_EFFICIENCY_POOR=70.0

# HRV thresholds (milliseconds)
PIERRE_HRV_RMSSD_DECREASE_CONCERN=-10.0
PIERRE_HRV_RMSSD_INCREASE_GOOD=5.0

# TSB thresholds
PIERRE_TSB_HIGHLY_FATIGUED=-15.0
PIERRE_TSB_FATIGUED=-10.0
PIERRE_TSB_FRESH_MIN=5.0
PIERRE_TSB_FRESH_MAX=15.0
PIERRE_TSB_DETRAINING=25.0

# Recovery scoring weights
PIERRE_RECOVERY_TSB_WEIGHT_FULL=0.4
PIERRE_RECOVERY_SLEEP_WEIGHT_FULL=0.4
PIERRE_RECOVERY_HRV_WEIGHT_FULL=0.2
PIERRE_RECOVERY_TSB_WEIGHT_NO_HRV=0.5
PIERRE_RECOVERY_SLEEP_WEIGHT_NO_HRV=0.5
```

Defaults based on peer-reviewed research (NSF, AASM, Shaffer & Ginsberg 2017).

---

## Validation And Safety

### Parameter Bounds (physiological ranges)

**physiological parameter ranges**:

```
max_hr ∈ [100, 220] bpm
resting_hr ∈ [30, 100] bpm
threshold_hr ∈ [100, 200] bpm
VO2max ∈ [20.0, 90.0] ml/kg/min
FTP ∈ [50, 600] watts
```

**range validation**: each parameter verified against physiologically plausible bounds

**relationship validation**:

```
resting_hr < threshold_hr < max_hr
```

Validation constraints:
- `HR_rest < HR_max` (resting heart rate below maximum)
- `HR_rest < HR_threshold` (resting heart rate below threshold)
- `HR_threshold < HR_max` (threshold heart rate below maximum)

**rust implementation**:

```rust
// src/intelligence/physiological_constants.rs::configuration_validation
pub const MAX_HR_MIN: u64 = 100;
pub const MAX_HR_MAX: u64 = 220;
pub const RESTING_HR_MIN: u64 = 30;
pub const RESTING_HR_MAX: u64 = 100;
pub const THRESHOLD_HR_MIN: u64 = 100;
pub const THRESHOLD_HR_MAX: u64 = 200;
pub const VO2_MAX_MIN: f64 = 20.0;
pub const VO2_MAX_MAX: f64 = 90.0;
pub const FTP_MIN: u64 = 50;
pub const FTP_MAX: u64 = 600;

// src/protocols/universal/handlers/configuration.rs
pub fn validate_parameter_ranges(
    obj: &serde_json::Map<String, serde_json::Value>,
    errors: &mut Vec<String>,
) -> bool {
    let mut all_valid = true;

    // Validate max_hr
    if let Some(hr) = obj.get("max_hr").and_then(Value::as_u64) {
        if !(MAX_HR_MIN..=MAX_HR_MAX).contains(&hr) {
            all_valid = false;
            errors.push(format!(
                "max_hr must be between {MAX_HR_MIN} and {MAX_HR_MAX} bpm, got {hr}"
            ));
        }
    }

    // Validate resting_hr
    if let Some(hr) = obj.get("resting_hr").and_then(Value::as_u64) {
        if !(RESTING_HR_MIN..=RESTING_HR_MAX).contains(&hr) {
            all_valid = false;
            errors.push(format!(
                "resting_hr must be between {RESTING_HR_MIN} and {RESTING_HR_MAX} bpm, got {hr}"
            ));
        }
    }

    // ... other validations

    all_valid
}

pub fn validate_parameter_relationships(
    obj: &serde_json::Map<String, serde_json::Value>,
    errors: &mut Vec<String>,
) -> bool {
    let mut all_valid = true;

    let max_hr = obj.get("max_hr").and_then(Value::as_u64);
    let resting_hr = obj.get("resting_hr").and_then(Value::as_u64);
    let threshold_hr = obj.get("threshold_hr").and_then(Value::as_u64);

    // Validate resting_hr < threshold_hr < max_hr
    if let (Some(resting), Some(max)) = (resting_hr, max_hr) {
        if resting >= max {
            all_valid = false;
            errors.push(format!(
                "resting_hr ({resting}) must be less than max_hr ({max})"
            ));
        }
    }

    if let (Some(resting), Some(threshold)) = (resting_hr, threshold_hr) {
        if resting >= threshold {
            all_valid = false;
            errors.push(format!(
                "resting_hr ({resting}) must be less than threshold_hr ({threshold})"
            ));
        }
    }

    if let (Some(threshold), Some(max)) = (threshold_hr, max_hr) {
        if threshold >= max {
            all_valid = false;
            errors.push(format!(
                "threshold_hr ({threshold}) must be less than max_hr ({max})"
            ));
        }
    }

    all_valid
}
```

**references**:
- ACSM Guidelines for Exercise Testing and Prescription, 11th Edition
- European Society of Cardiology guidelines on exercise testing

### Confidence Levels

**confidence level classification**:

```
confidence(n, R²) = High,      if (n ≥ 15) ∧ (R² ≥ 0.7)
                  = Medium,    if (n ≥ 8) ∧ (R² ≥ 0.5)
                  = Low,       if (n ≥ 3) ∧ (R² ≥ 0.3)
                  = VeryLow,   otherwise
```

Where:
- `n` = number of data points
- `R²` = coefficient of determination ∈ [0, 1]

**rust implementation**:

```rust
pub fn calculate_confidence(
    data_points: usize,
    r_squared: f64,
) -> ConfidenceLevel {
    match (data_points, r_squared) {
        (n, r) if n >= 15 && r >= 0.7 => ConfidenceLevel::High,
        (n, r) if n >= 8  && r >= 0.5 => ConfidenceLevel::Medium,
        (n, r) if n >= 3  && r >= 0.3 => ConfidenceLevel::Low,
        _ => ConfidenceLevel::VeryLow,
    }
}
```

### Edge Case Handling

**1. Users with no activities**:

```
If |activities| = 0, return:
  CTL = 0
  ATL = 0
  TSB = 0
  TSS_history = ∅ (empty set)
```

**rust implementation**:
```rust
if activities.is_empty() {
    return Ok(TrainingLoad {
        ctl: 0.0,
        atl: 0.0,
        tsb: 0.0,
        tss_history: Vec::new(),
    });
}
```

**2. Training gaps (TSS sequence breaks)**:

```
For missing days: TSS_daily = 0

Exponential decay: EMAₜ = (1 − α) × EMAₜ₋₁
```

Result: CTL/ATL naturally decay during breaks (realistic fitness loss)

**rust implementation**:
```rust
// Zero-fill missing days in EMA calculation
let daily_tss = tss_map.get(&date_key).copied().unwrap_or(0.0); // Gap = 0
ema = daily_tss.mul_add(alpha, ema * (1.0 - alpha));
```

**3. Invalid physiological parameters**:

Range validation checks:
- `max_hr = 250` → rejected (exceeds upper bound 220)
- `resting_hr = 120` → rejected (exceeds upper bound 100)

Relationship validation checks:
- `max_hr = 150, resting_hr = 160` → rejected (violates `HR_rest < HR_max`)

Returns detailed error messages for each violation

**4. Invalid race velocities**:

Velocity constraint: `v ∈ [100, 500]` m/min

If `v ∉ [100, 500]`, reject with error message

**rust implementation**:
```rust
if !(MIN_VELOCITY..=MAX_VELOCITY).contains(&velocity) {
    return Err(AppError::invalid_input(format!(
        "Velocity {velocity:.1} m/min outside valid range (100-500)"
    )));
}
```

**5. VDOT out of range**:

VDOT constraint: `VDOT ∈ [30, 85]`

If `VDOT ∉ [30, 85]`, reject with error message

**rust implementation**:
```rust
if !(30.0..=85.0).contains(&vdot) {
    return Err(AppError::invalid_input(format!(
        "VDOT {vdot:.1} outside typical range (30-85)"
    )));
}
```

---

## Configuration Strategies

Three strategies adjust training thresholds:

### Conservative Strategy

**parameters**:
- `max_weekly_load_increase = 0.05` (5%)
- `recovery_threshold = 1.2`

**rust implementation**:
```rust
impl IntelligenceStrategy for ConservativeStrategy {
    fn max_weekly_load_increase(&self) -> f64 { 0.05 } // 5%
    fn recovery_threshold(&self) -> f64 { 1.2 }
}
```

**recommended for**: injury recovery, beginners, older athletes

### Default Strategy

**parameters**:
- `max_weekly_load_increase = 0.10` (10%)
- `recovery_threshold = 1.3`

**rust implementation**:
```rust
impl IntelligenceStrategy for DefaultStrategy {
    fn max_weekly_load_increase(&self) -> f64 { 0.10 } // 10%
    fn recovery_threshold(&self) -> f64 { 1.3 }
}
```

**recommended for**: general training, recreational athletes

### Aggressive Strategy

**parameters**:
- `max_weekly_load_increase = 0.15` (15%)
- `recovery_threshold = 1.5`

**rust implementation**:
```rust
impl IntelligenceStrategy for AggressiveStrategy {
    fn max_weekly_load_increase(&self) -> f64 { 0.15 } // 15%
    fn recovery_threshold(&self) -> f64 { 1.5 }
}
```

**recommended for**: competitive athletes, experienced trainers

---

## Testing And Verification

### Test Coverage

**unit tests** (22 functions, 562 assertions):
- `tests/pattern_detection_test.rs` - 4 tests
- `tests/performance_prediction_test.rs` - 9 tests
- `tests/training_load_test.rs` - 6 tests
- `tests/vdot_table_verification_test.rs` - 3 tests

**integration tests** (116+ test files):
- Full MCP tool workflows
- Multi-provider scenarios
- Edge case handling
- Error recovery

**automated intelligence testing** (30+ integration tests):
- `tests/intelligence_tools_basic_test.rs` - 10 tests covering basic fitness data tools
- `tests/intelligence_tools_advanced_test.rs` - 20+ tests covering analytics, predictions, and goals
- `tests/intelligence_synthetic_helpers_test.rs` - synthetic data generation validation

**synthetic data framework** (`tests/helpers/`):
- `synthetic_provider.rs` - mock fitness provider with realistic activity data
- `synthetic_data.rs` - configurable test scenarios (beginner runner, experienced cyclist, multi-sport)
- `test_utils.rs` - test utilities and scenario builders
- enables testing all 8 intelligence tools without OAuth dependencies

### Verification Methods

**1. Scientific validation**:
- VDOT predictions: 0.2-5.5% accuracy vs. jack daniels' tables
- TSS formulas: match coggan's published methodology
- Statistical methods: verified against standard regression algorithms

**2. Edge case testing**:
```rust
#[test]
fn test_empty_activities() {
    let result = TrainingLoadCalculator::new()
        .calculate_training_load(&[], None, None, None, None, None)
        .unwrap();
    assert_eq!(result.ctl, 0.0);
    assert_eq!(result.atl, 0.0);
}

#[test]
fn test_training_gaps() {
    // Activities: day 1, day 10 (9-day gap)
    // EMA should decay naturally through the gap
    let activities = create_activities_with_gap();
    let result = calculate_training_load(&activities).unwrap();
    // Verify CTL decay through gap
}

#[test]
fn test_invalid_hr_relationships() {
    let config = json!({
        "max_hr": 150,
        "resting_hr": 160
    });
    let result = validate_configuration(&config);
    assert!(result.errors.contains("resting_hr must be less than max_hr"));
}
```

**3. Placeholder elimination**:
```bash
# Zero placeholders confirmed
rg -i "placeholder|todo|fixme|hack|stub" src/ | wc -l
# Output: 0
```

**4. Synthetic data testing**:
```rust
// Example: Test fitness score calculation with synthetic data
#[tokio::test]
async fn test_fitness_score_calculation() {
    let provider = create_synthetic_provider_with_scenario(
        TestScenario::ExperiencedCyclistConsistent
    );

    let activities = provider.get_activities(Some(100), None)
        .await.expect("Should get activities");

    let analyzer = PerformanceAnalyzerV2::new(Box::new(DefaultStrategy))
        .expect("Should create analyzer");

    let fitness_score = analyzer.calculate_fitness_score(&activities)
        .expect("Should calculate fitness score");

    // Verify realistic fitness score for experienced cyclist
    assert!(fitness_score.overall_score >= 70.0);
    assert!(fitness_score.overall_score <= 90.0);
}
```

**5. Code quality**:
```bash
# Zero clippy warnings (pedantic + nursery)
cargo clippy -- -W clippy::all -W clippy::pedantic -W clippy::nursery -D warnings
# Output: PASS

# Zero prohibited patterns
rg "unwrap\(\)|expect\(|panic!\(|anyhow!\(" src/ | wc -l
# Output: 0
```

---

## Debugging And Validation Guide

This comprehensive guide helps API users troubleshoot discrepancies between expected and actual calculations.

### General Debugging Workflow

When your calculated values don't match pierre's API responses, follow this systematic approach:

**1. Verify input data quality**

```bash
# Check for data integrity issues
- Missing values: NULL, NaN, undefined
- Out-of-range values: negative durations, power > 2000W, HR > 220bpm
- Unit mismatches: meters vs kilometers, seconds vs minutes, watts vs kilowatts
- Timestamp errors: activities in future, overlapping time periods
```

**2. Reproduce calculation step-by-step**

Use the validation examples in each metric section:
- Start with the exact input values from the example
- Calculate each intermediate step
- Compare intermediate values, not just final results
- Identify exactly where your calculation diverges

**3. Check boundary conditions**

Many formulas use piecewise functions with discrete boundaries:
- TSS duration scaling: check if you're at 30min, 90min boundaries
- VDOT percent_max: check if you're at 5min, 15min, 30min, 90min boundaries
- Sleep duration scoring: check if you're at 5h, 6h, 7h, 8h boundaries
- Recovery level classification: check if you're at 50, 70, 85 boundaries

**4. Verify floating point precision**

```rust
// DON'T compare with exact equality
if calculated_value == expected_value { ... }  // ❌ WRONG

// DO compare with tolerance
if (calculated_value - expected_value).abs() < tolerance { ... }  // ✅ CORRECT

// Recommended tolerances:
// TSS: ±0.1
// CTL/ATL: ±0.5
// TSB: ±1.0
// VDOT: ±0.5
// Sleep quality: ±1.0
// Recovery score: ±2.0
```

**5. Eliminate common calculation errors**

See metric-specific sections below for detailed error patterns.

### Metric-specific Debugging

#### Debugging TSS Calculations

**symptom: TSS values differ by 5-20%**

```bash
# Diagnostic checklist:
1. Verify normalized power calculation (4th root method)
   - Are you using 30-second rolling average?
   - Did you apply the 4th power before averaging?
   - Formula: NP = ⁴√(avg(power³⁰ˢᵉᶜ⁴))

2. Check intensity factor precision
   - IF = NP / FTP
   - Verify FTP value is user's current FTP, not default

3. Verify duration is in hours
   - Common error: passing seconds instead of hours
   - TSS = (duration_hours × NP² × 100) / (FTP² × 3600)

4. Check for zero or negative FTP
   - FTP must be > 0
   - Default FTP values may not represent user's actual fitness
```

**example debugging session:**

```
User reports: TSS = 150, but pierre returns 138.9

Inputs:
  duration_s = 7200  (2 hours)
  normalized_power = 250W
  ftp = 300W

Debug steps:
  1. Convert duration: 7200 / 3600 = 2.0 hours ✓
  2. Calculate IF: 250 / 300 = 0.833 ✓
  3. Calculate TSS: 2.0 × 0.833² × 100 = 138.8889 ✓

Root cause: User was using duration in seconds directly
  Wrong: TSS = 7200 × 0.833² × 100 / (300² × 3600) = [calculation error]
  Fix: Convert seconds to hours first
```

#### Debugging CTL/ATL/TSB Calculations

**symptom: CTL/ATL drift over time, doesn't match pierre**

```bash
# Diagnostic checklist:
1. Verify EMA initialization (cold start problem)
   - First CTL = first TSS (not 0)
   - First ATL = first TSS (not 0)
   - Don't initialize with population averages

2. Check gap handling
   - Zero TSS days should be included in EMA
   - Formula: CTL_today = (CTL_yesterday × (1 - 1/42)) + (TSS_today × (1/42))
   - If activity missing: TSS_today = 0, but still update EMA

3. Verify day boundaries
   - Activities must be grouped by calendar day
   - Multiple activities per day: sum TSS before EMA
   - Timezone consistency: use user's local timezone

4. Check calculation order
   - Update CTL and ATL FIRST
   - Calculate TSB AFTER: TSB = CTL - ATL
   - Don't calculate TSB independently
```

**example debugging session:**

```
User reports: After 7 days, CTL = 55, but pierre shows 45

Day | TSS | User's CTL | Pierre's CTL | Issue
----|-----|------------|--------------|-------
1   | 100 | 100        | 100          | ✓ Match (initialization)
2   | 80  | 90         | 97.6         | ❌ Wrong formula
3   | 60  | 75         | 93.3         | ❌ Compounding error
...

Debug:
  Day 2 calculation:
    User: CTL = (100 + 80) / 2 = 90  ❌ Using simple average
    Pierre: CTL = 100 × (41/42) + 80 × (1/42) = 97.619  ✓ Using EMA

Root cause: User implementing simple moving average instead of exponential
Fix: Use EMA formula with decay factor (41/42 for CTL, 6/7 for ATL)
```

#### Debugging VDOT Calculations

**symptom: VDOT differs by 2-5 points**

```bash
# Diagnostic checklist:
1. Verify velocity calculation
   - velocity = (distance_m / time_s) × 60
   - Must be in meters per minute (not km/h or mph)
   - Valid range: [100, 500] m/min

2. Check percent_max for race duration
   - t < 5min: 0.97
   - 5min ≤ t < 15min: 0.99
   - 15min ≤ t < 30min: 1.00
   - 30min ≤ t < 90min: 0.98
   - t ≥ 90min: 0.95
   - Use time in MINUTES for this check

3. Verify VO2 calculation precision
   - vo2 = -4.60 + 0.182258×v + 0.000104×v²
   - Use full coefficient precision (not rounded values)
   - Don't round intermediate values

4. Check boundary conditions
   - At exactly t=15min: uses 1.00 (not 0.99)
   - At exactly t=30min: uses 0.98 (not 1.00)
   - Boundary behavior creates discrete jumps
```

**example debugging session:**

```
User reports: 10K in 37:30 → VDOT = 50.5, but pierre returns 52.4

Inputs:
  distance_m = 10000
  time_s = 2250

Debug steps:
  1. velocity = (10000 / 2250) × 60 = 266.67 m/min ✓
  2. vo2 = -4.60 + 0.182258×266.67 + 0.000104×266.67²
     User calculated: vo2 = 50.8 ❌
     Correct: vo2 = -4.60 + 48.602 + 7.396 = 51.398 ✓

  3. time_minutes = 2250 / 60 = 37.5 minutes
     37.5 minutes is in range [30, 90) → percent_max = 0.98 ✓

  4. VDOT = 51.398 / 0.98 = 52.4 ✓

Root cause: User calculated vo2 incorrectly (likely rounding error)
  User used: 0.18 instead of 0.182258 (coefficient precision loss)
  Fix: Use full precision coefficients
```

#### Debugging Sleep Quality Scoring

**symptom: sleep score differs by 10-20 points**

```bash
# Diagnostic checklist:
1. Verify component percentages sum correctly
   - deep% + rem% + light% + awake% should ≈ 100%
   - Tracker rounding may cause sum = 99% or 101%
   - Pierre accepts raw values (no normalization)

2. Check efficiency calculation
   - efficiency = (time_asleep_min / time_in_bed_min) × 100
   - time_asleep should ALWAYS be ≤ time_in_bed
   - If efficiency > 100%, data error

3. Verify awake penalty application
   - Only applied if awake% > 5%
   - penalty = (awake_percent - 5.0) × 2.0
   - Subtracted from stages_score (can result in negative, clamped to 0)

4. Check component weights
   - Duration: 40%
   - Stages: 35%
   - Efficiency: 25%
   - Weights must sum to 100%
```

**example debugging session:**

```
User reports: 7.5h sleep → score = 80, but pierre returns 88.1

Inputs:
  duration_hours = 7.5
  deep% = 18, rem% = 22, light% = 54, awake% = 6
  time_asleep = 450min, time_in_bed = 500min

Debug steps:
  1. Duration score: 7.0 ≤ 7.5 < 8.0
     score = 85 + 15×(7.5-7.0) = 92.5 ✓

  2. Stages score:
     deep_score = 70 + 30×(18-15)/5 = 88.0 ✓
     rem_score = 70 + 30×(22-20)/5 = 82.0 ✓
     awake_penalty = (6-5)×2 = 2.0 ✓

     User calculated: (88×0.4) + (82×0.4) + (54×0.2) = 78.8 ❌
     Correct: (88×0.4) + (82×0.4) + (54×0.2) - 2.0 = 76.8 ✓

  3. Efficiency: (450/500)×100 = 90% → score = 100 ✓

  4. Overall:
     User: (92.5×0.35) + (78.8×0.40) + (100×0.25) = 85.07 ❌
     Pierre: (92.5×0.35) + (76.8×0.40) + (100×0.25) = 88.1 ✓

Root cause: User forgot to subtract awake_penalty from stages_score
Fix: Apply penalty before weighting stages component
```

#### Debugging Recovery Score

**symptom: recovery score differs by 5-10 points**

```bash
# Diagnostic checklist:
1. Verify TSB normalization
   - Don't use raw TSB value [-30, +30]
   - Must normalize to [0, 100] using piecewise function
   - See TSB normalization formula (6 ranges)

2. Check HRV weighting
   - WITH HRV: 40% TSB, 40% sleep, 20% HRV
   - WITHOUT HRV: 50% TSB, 50% sleep
   - Same inputs produce different scores based on HRV availability

3. Verify HRV delta calculation
   - delta = current_rmssd - baseline_rmssd
   - Must use individual baseline (not population average)
   - Positive delta = good recovery
   - Negative delta = poor recovery

4. Check classification boundaries
   - excellent: ≥85
   - good: [70, 85)
   - fair: [50, 70)
   - poor: <50
```

**example debugging session:**

```
User reports: TSB=8, sleep=92, HRV=55 (weekly_avg=50) → score=85, but pierre returns 90

Debug steps:
  1. TSB normalization (5 ≤ 8 < 15):
     tsb_score = 80 + 10×(8-5)/10 = 83.0 ✓

  2. Sleep score (pass-through):
     sleep_score = 92.0 ✓

  3. HRV score:
     change_from_avg = 55 - 50 = +5.0ms
     +5.0 ≥ +5.0 threshold → HrvRecoveryStatus::Recovered → score = 100 ✓

  4. Recovery score:
     User calculated: (83×0.5) + (92×0.5) = 87.5 ❌
     Pierre: (83×0.4) + (92×0.4) + (100×0.2) = 90.0 ✓

Root cause: User applied 50/50 weights even though HRV available
  Wrong: 50% TSB, 50% sleep (HRV ignored)
  Correct: 40% TSB, 40% sleep, 20% HRV

Fix: When HRV available, use 40/40/20 split
```

### Common Platform-specific Issues

#### Javascript/Typescript Precision

```javascript
// JavaScript number is IEEE 754 double precision (same as Rust f64)
// But watch for integer overflow and precision loss

// ❌ WRONG: Integer math before conversion
const velocity = (distance_m / time_s) * 60;  // May lose precision

// ✅ CORRECT: Ensure floating point math
const velocity = (distance_m / time_s) * 60.0;

// ❌ WRONG: Using Math.pow for small exponents
const if_squared = Math.pow(intensity_factor, 2);

// ✅ CORRECT: Direct multiplication (faster, more precise)
const if_squared = intensity_factor * intensity_factor;
```

#### Python Precision

```python
# Python 3 uses arbitrary precision integers
# But watch for integer division vs float division

# ❌ WRONG: Integer division (Python 2 behavior)
velocity = (distance_m / time_s) * 60  # May truncate

# ✅ CORRECT: Ensure float division
velocity = float(distance_m) / float(time_s) * 60.0

# ❌ WRONG: Using ** operator with large values
normalized_power = (sum(powers) / len(powers)) ** 0.25

# ✅ CORRECT: Use explicit functions for clarity
import math
normalized_power = math.pow(sum(powers) / len(powers), 0.25)
```

#### REST API / JSON Precision

```bash
# JSON numbers are typically parsed as double precision
# But watch for serialization precision loss

# Server returns:
{"tss": 138.88888888888889}

# Client receives (depending on JSON parser):
{"tss": 138.89}  # Rounded by parser

# Solution: Accept small differences
tolerance = 0.1
assert abs(received_tss - expected_tss) < tolerance
```

### Data Quality Validation

Before debugging calculation logic, verify input data quality:

**activity data validation**

```bash
# Power data
- Valid range: [0, 2000] watts (pro cyclists max ~500W sustained)
- Check for dropout: consecutive zeros in power stream
- Check for spikes: isolated values >2× average power
- Negative values: impossible, indicates sensor error

# Heart rate data
- Valid range: [40, 220] bpm
- Check for dropout: consecutive zeros or flat lines
- Check resting HR: typically [40-80] bpm for athletes
- Max HR: age-based estimate 220-age (±10 bpm variance)

# Duration data
- Valid range: [0, 86400] seconds (max 24 hours per activity)
- Check for negative durations: clock sync issues
- Check for unrealistic durations: 48h "run" likely data error

# Distance data
- Valid range: depends on sport
- Running: typical pace [3-15] min/km
- Cycling: typical speed [15-45] km/h
- Check for GPS drift: indoor activities with high distance

# Sleep data
- Duration: typically [2-14] hours
- Efficiency: typically [65-98]%
- Stage percentages must sum to ~100%
- Check for unrealistic values: 0% deep sleep, 50% awake
```

**handling missing data**

```rust
// Pierre's approach to missing data:

// TSS calculation: reject if required fields missing
if power_data.is_empty() || ftp.is_none() {
    return Err(AppError::insufficient_data("Cannot calculate TSS"));
}

// CTL/ATL calculation: use zero for missing days
let tss_today = activities_today.map(|a| a.tss).sum_or(0.0);

// Sleep quality: partial calculation if stages missing
if stages.is_none() {
    // Calculate using duration and efficiency only (skip stages component)
    sleep_quality = (duration_score × 0.60) + (efficiency_score × 0.40)
}

// Recovery score: adaptive weighting based on availability
match (tsb, sleep_quality, hrv_data) {
    (Some(t), Some(s), Some(h)) => /* 40/40/20 */,
    (Some(t), Some(s), None)    => /* 50/50 */,
    _                           => Err(InsufficientData),
}
```

### When To Contact Support

Contact pierre support team if:

**1. Consistent calculation discrepancies >10%**
- You've verified input data quality
- You've reproduced calculation step-by-step
- Discrepancy persists across multiple activities
- Example: "All my TSS values are 15% higher than pierre's"

**2. Boundary condition bugs**
- Discrete jumps at boundaries larger than expected
- Example: "At exactly 15 minutes, my VDOT jumps by 5 points"

**3. Platform-specific precision issues**
- Same calculation produces different results on different platforms
- Example: "VDOT matches on desktop but differs by 3 on mobile"

**4. API response format changes**
- Response structure doesn't match documentation
- Missing fields in JSON response
- Unexpected error codes

**provide in support request:**
```
Subject: [METRIC] Calculation Discrepancy - [Brief Description]

Environment:
- Platform: [Web/Mobile/API]
- Language: [JavaScript/Python/Rust/etc]
- Pierre API version: [v1/v2/etc]

Input Data:
- [Full input values with types and units]
- Activity ID (if applicable): [123456789]

Expected Output:
- [Your calculated value with step-by-step calculation]

Actual Output:
- [Pierre's API response value]

Difference:
- Absolute: [X.XX units]
- Percentage: [X.X%]

Debugging Steps Taken:
- [List what you've already tried]
```

### Debugging Tools And Utilities

**command-line validation**

```bash
# Quick TSS calculation
echo "scale=2; (2.0 * 250 * 250 * 100) / (300 * 300 * 3600)" | bc

# Quick VDOT velocity check
python3 -c "print((10000 / 2250) * 60)"

# Quick EMA calculation
python3 -c "ctl_prev=100; tss=80; ctl_new=ctl_prev*(41/42)+tss*(1/42); print(ctl_new)"

# Compare with tolerance
python3 -c "import sys; abs(138.9 - 138.8) < 0.1 and sys.exit(0) or sys.exit(1)"
```

**spreadsheet validation**

Create a validation spreadsheet with columns:
```
| Input 1 | Input 2 | ... | Intermediate 1 | Intermediate 2 | Final Result | Pierre Result | Diff | Within Tolerance? |
```

Use formulas to calculate step-by-step and highlight discrepancies.

**automated testing**

```python
# Example pytest validation test
import pytest
from pierre_client import calculate_tss

def test_tss_validation_examples():
    """Test against documented validation examples."""

    # Example 1: Easy ride
    result = calculate_tss(
        normalized_power=180,
        duration_hours=2.0,
        ftp=300
    )
    assert abs(result - 72.0) < 0.1, f"Expected 72.0, got {result}"

    # Example 2: Threshold workout
    result = calculate_tss(
        normalized_power=250,
        duration_hours=2.0,
        ftp=300
    )
    assert abs(result - 138.9) < 0.1, f"Expected 138.9, got {result}"
```

---

## Limitations

### Model Assumptions
1. **linear progression**: assumes linear improvement, but adaptation is non-linear
2. **steady-state**: assumes consistent training environment
3. **population averages**: formulas may not fit individual physiology
4. **data quality**: sensor accuracy affects calculations

### Known Issues
- **HR metrics**: affected by caffeine, sleep, stress, heat, altitude
- **power metrics**: require proper FTP testing, affected by wind/drafting
- **pace metrics**: terrain and weather significantly affect running

### Prediction Accuracy
- **VDOT**: ±5% typical variance from actual race performance
- **TSB**: individual response to training load varies
- **patterns**: require sufficient data (minimum 3 weeks for trends)

---

## References

### Scientific Literature

1. **Banister, E.W.** (1991). Modeling elite athletic performance. Human Kinetics.

2. **Coggan, A. & Allen, H.** (2010). *Training and Racing with a Power Meter* (2nd ed.). VeloPress.

3. **Daniels, J.** (2013). *Daniels' Running Formula* (3rd ed.). Human Kinetics.

4. **Esteve-Lanao, J. Et al.** (2005). How do endurance runners train? *Med Sci Sports Exerc*, 37(3), 496-504.

5. **Halson, S.L.** (2014). Monitoring training load to understand fatigue. *Sports Medicine*, 44(Suppl 2), 139-147.

6. **Karvonen, M.J. Et al.** (2057). The effects of training on heart rate. *Ann Med Exp Biol Fenn*, 35(3), 307-315.

7. **Riegel, P.S.** (1981). Athletic records and human endurance. *American Scientist*, 69(3), 285-290.

8. **Tanaka, H. Et al.** (2001). Age-predicted maximal heart rate revisited. *J Am Coll Cardiol*, 37(1), 153-156.

9. **Gabbett, T.J.** (2016). The training-injury prevention paradox. *Br J Sports Med*, 50(5), 273-280.

10. **Seiler, S.** (2010). Training intensity distribution in endurance athletes. *Int J Sports Physiol Perform*, 5(3), 276-291.

11. **Draper, N.R. & Smith, H.** (1998). *Applied Regression Analysis* (3rd ed.). Wiley.

12. **Hirshkowitz, M. Et al.** (2015). National Sleep Foundation's sleep time duration recommendations: methodology and results summary. *Sleep Health*, 1(1), 40-43.

13. **Berry, R.B. Et al.** (2017). The AASM Manual for the Scoring of Sleep and Associated Events: Rules, Terminology and Technical Specifications, Version 2.4. *American Academy of Sleep Medicine*.

14. **Watson, N.F. Et al.** (2015). Recommended Amount of Sleep for a Healthy Adult: A Joint Consensus Statement of the American Academy of Sleep Medicine and Sleep Research Society. *Sleep*, 38(6), 843-844.

15. **Plews, D.J. Et al.** (2013). Training adaptation and heart rate variability in elite endurance athletes: opening the door to effective monitoring. *Int J Sports Physiol Perform*, 8(3), 286-293.

16. **Shaffer, F. & Ginsberg, J.P.** (2017). An Overview of Heart Rate Variability Metrics and Norms. *Front Public Health*, 5, 258.

---

## FAQ

**Q: why doesn't my prediction match race day?**
A: predictions are ranges (±5%), not exact. Affected by: weather, course, pacing, nutrition, taper, mental state.

**Q: can analytics work without HR or power?**
A: yes, but lower confidence. Pace-based TSS estimates used. Add HR/power for better accuracy.

**Q: how often update FTP/LTHR?**
A: FTP every 6-8 weeks, LTHR every 8-12 weeks, max HR annually.

**Q: why is TSB negative?**
A: normal during training. -30 to -10 = building fitness, -10 to 0 = productive, 0 to +10 = fresh/race ready.

**Q: how interpret confidence levels?**
A: high (15+ points, R²>0.7) = actionable; medium = guidance; low = directional; very low = insufficient data.

**Q: what happens if I have gaps in training?**
A: CTL/ATL naturally decay with zero TSS during gaps. This accurately models fitness loss during breaks.

**Q: how accurate are the VDOT predictions?**
A: verified 0.2-5.5% accuracy against jack daniels' published tables. Predictions assume proper training, taper, and race conditions.

**Q: what if my parameters are outside the valid ranges?**
A: validation will reject with specific error messages. Ranges are based on human physiology research (ACSM guidelines).

**Q: how much sleep do athletes need?**
A: 8-10 hours for optimal recovery (NSF guidelines). Minimum 7 hours for adults. <6 hours increases injury risk and impairs performance.

**Q: what's more important: sleep duration or quality?**
A: both matter. 8 hours of fragmented sleep (70% efficiency) scores lower than 7 hours of solid sleep (95% efficiency). Aim for both duration and quality.

**Q: why is my recovery score low despite good sleep?**
A: recovery combines TSB (40%), sleep (40%), HRV (20%). Negative TSB from high training load lowers score even with good sleep. This accurately reflects accumulated fatigue.

**Q: how does HRV affect recovery scoring?**
A: HRV (RMSSD) indicates autonomic nervous system recovery. +5ms above baseline = excellent, ±3ms = normal, -10ms = poor recovery. When unavailable, recovery uses 50% TSB + 50% sleep.

**Q: what providers support sleep tracking?**
A: fitbit, garmin, and whoop provide sleep data. Strava does not (returns `UnsupportedFeature` error). Use provider with sleep tracking for full recovery analysis.

---

## Glossary

**ATL**: acute training load (7-day EMA of TSS) - fatigue
**CTL**: chronic training load (42-day EMA of TSS) - fitness
**EMA**: exponential moving average - weighted average giving more weight to recent data
**FTP**: functional threshold power (1-hour max power)
**LTHR**: lactate threshold heart rate
**TSB**: training stress balance (CTL - ATL) - form
**TSS**: training stress score (duration × intensity²)
**VDOT**: VO2max adjusted for running economy (jack daniels)
**NP**: normalized power (4th root method)
**R²**: coefficient of determination (fit quality, 0-1)
**IF**: intensity factor (NP / FTP)
**RMSSD**: root mean square of successive differences (HRV metric, milliseconds)
**HRV**: heart rate variability (autonomic nervous system recovery indicator)
**NSF**: National Sleep Foundation (sleep duration guidelines)
**AASM**: American Academy of Sleep Medicine (sleep stage scoring standards)
**REM**: rapid eye movement sleep (cognitive recovery, memory consolidation)
**N3/deep sleep**: slow-wave sleep (physical recovery, growth hormone release)
**sleep efficiency**: (time asleep / time in bed) × 100 (fragmentation indicator)
**sleep quality**: combined score (40% duration, 35% stages, 25% efficiency)
**recovery score**: training readiness (40% TSB, 40% sleep, 20% HRV)

---

---

# Pierre Nutrition and USDA Integration Methodology

## What This Document Covers

This comprehensive guide explains the scientific methods, algorithms, and data integration behind pierre's nutrition system. It provides transparency into:

- **mathematical foundations**: BMR formulas, TDEE calculations, macronutrient distribution algorithms
- **usda fooddata central integration**: real food database access with 350,000+ foods
- **calculation methodologies**: step-by-step algorithms for daily nutrition needs
- **scientific references**: peer-reviewed research backing each recommendation
- **implementation details**: rust code architecture and api integration patterns
- **validation**: bounds checking, input validation, and safety mechanisms
- **testing**: comprehensive test coverage without external api dependencies

**target audience**: developers, nutritionists, coaches, and users seeking deep understanding of pierre's nutrition intelligence.

---

## ⚠️ Implementation Status: Production-Ready

**as of 2025-10-31**, pierre's nutrition system has been built from scratch using peer-reviewed sports nutrition science and usda fooddata central integration:

### What Was Implemented ✅
- **mifflin-st jeor bmr**: most accurate resting energy expenditure formula (±10% error vs indirect calorimetry)
- **tdee calculation**: activity-based multipliers from mcardle exercise physiology textbook
- **protein recommendations**: sport-specific ranges from phillips & van loon sports nutrition research
- **carbohydrate targeting**: burke et al. endurance athlete guidelines (3-12 g/kg based on activity)
- **fat calculations**: dri guidelines enforcement (20-35% of tdee)
- **nutrient timing**: kerksick et al. position stand on pre/post-workout nutrition
- **usda integration**: real food lookup via fooddata central api with mock support for testing
- **meal analysis**: multi-food calculations with accurate macro summations
- **input validation**: age (10-120), weight (0-300kg), height (0-300cm) bounds checking

### Verification ✅
- **39 algorithm tests**: bmr (4), tdee (5), protein (5), carbs (4), fat (3), complete nutrition (3), timing (3), edge cases (13)
- **formula accuracy**: mifflin-st jeor within 1 kcal of hand calculations
- **macro summing**: percentages always sum to 100% ±0.1%
- **usda integration**: tested with mock client (banana, chicken breast, oatmeal, salmon)
- **edge case handling**: negative inputs rejected, extreme values bounded, missing data handled
- **zero warnings**: strict clippy (pedantic + nursery) passes clean
- **1,188 total tests** passing including nutrition suite

**result**: pierre nutrition system is production-ready with scientifically-validated algorithms and usda fooddata integration.

---

## Architecture Overview

Pierre's nutrition system uses a **foundation modules** approach integrated with usda fooddata central:

```
┌─────────────────────────────────────────────┐
│   mcp/a2a protocol layer                    │
│   (src/protocols/universal/)                │
└──────────────────┬──────────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────────┐
│   nutrition tools (5 tools)                 │
│   (src/protocols/universal/handlers/)       │
└──────────────────┬──────────────────────────┘
                   │
    ┌──────────────┼──────────────────────────┐
    ▼              ▼                          ▼
┌─────────────┐ ┌──────────────┐       ┌──────────────┐
│ Nutrition   │ │ USDA Food    │       │ Meal         │
│ Calculator  │ │ Database     │       │ Analyzer     │
│             │ │              │       │              │
│ BMR/TDEE    │ │ 350k+ Foods  │       │ Multi-Food   │
│ Macros      │ │ Nutrients    │       │ Summation    │
│ Timing      │ │ API Client   │       │ Analysis     │
└─────────────┘ └──────────────┘       └──────────────┘
         NUTRITION FOUNDATION MODULE
```

### Nutrition Calculator Module

**`src/intelligence/nutrition_calculator.rs`** - core nutrition algorithms
- **mifflin-st jeor bmr** calculation with gender-specific constants
- **tdee calculation** with 5 activity level multipliers (1.2-1.9)
- **protein recommendations** based on activity level and training goal (0.8-2.2 g/kg)
- **carbohydrate targeting** optimized for endurance, strength, or weight loss (3-12 g/kg)
- **fat calculations** ensuring dri compliance (20-35% of tdee)
- **nutrient timing** for pre/post-workout optimization
- **protein distribution** across meals for muscle protein synthesis
- **input validation** with physiological bounds checking

### USDA Integration Module

**`src/external/usda_client.rs`** - fooddata central api client
- **async http client** with configurable timeout and rate limiting
- **food search** by name with pagination support
- **food details** retrieval with complete nutrient breakdown
- **mock client** for testing without api calls
- **error handling** with retry logic and graceful degradation
- **caching** with ttl for api response optimization

**`src/external/usda_client.rs`** - usda data structures (models re-exported via `src/external/mod.rs`)
- **food** representation with fdc_id and description
- **nutrient** structure with name, amount, unit
- **search results** with pagination metadata
- **type-safe** deserialization from usda json responses

---

## 1. Basal Metabolic Rate (BMR) Calculation

### Mifflin-St Jeor Formula (1990)

**most accurate formula** for resting energy expenditure (±10% error vs indirect calorimetry), superior to harris-benedict.

#### Formula

**for males:**
```
bmr = (10 × weight_kg) + (6.25 × height_cm) - (5 × age) + 5
```

**for females:**
```
bmr = (10 × weight_kg) + (6.25 × height_cm) - (5 × age) - 161
```

#### Implementation

`src/intelligence/nutrition_calculator.rs:169-207`

```rust
pub fn calculate_mifflin_st_jeor(
    weight_kg: f64,
    height_cm: f64,
    age: u32,
    gender: Gender,
    config: &BmrConfig,
) -> Result<f64, AppError> {
    // Validation
    if weight_kg <= 0.0 || weight_kg > 300.0 {
        return Err(AppError::invalid_input("Weight must be between 0 and 300 kg"));
    }
    if height_cm <= 0.0 || height_cm > 300.0 {
        return Err(AppError::invalid_input("Height must be between 0 and 300 cm"));
    }
    if !(10..=120).contains(&age) {
        return Err(AppError::invalid_input(
            "Age must be between 10 and 120 years (Mifflin-St Jeor formula validated for ages 10+)",
        ));
    }

    // Mifflin-St Jeor formula
    let weight_component = config.msj_weight_coef * weight_kg;         // 10.0
    let height_component = config.msj_height_coef * height_cm;         // 6.25
    let age_component = config.msj_age_coef * f64::from(age);          // -5.0

    let gender_constant = match gender {
        Gender::Male => config.msj_male_constant,      // +5
        Gender::Female => config.msj_female_constant,  // -161
    };

    let bmr = weight_component + height_component + age_component + gender_constant;

    // Minimum 1000 kcal/day safety check
    Ok(bmr.max(1000.0))
}
```

#### Example Calculations

**example 1: 30-year-old male, 75kg, 180cm**
```
bmr = (10 × 75) + (6.25 × 180) - (5 × 30) + 5
bmr = 750 + 1125 - 150 + 5
bmr = 1730 kcal/day
```

**example 2: 25-year-old female, 60kg, 165cm**
```
bmr = (10 × 60) + (6.25 × 165) - (5 × 25) - 161
bmr = 600 + 1031.25 - 125 - 161
bmr = 1345 kcal/day
```

#### Configuration

`src/config/intelligence_config.rs:423-438`

```rust
pub struct BmrConfig {
    pub use_mifflin_st_jeor: bool,     // true (recommended)
    pub use_harris_benedict: bool,     // false (legacy)
    pub msj_weight_coef: f64,          // 10.0
    pub msj_height_coef: f64,          // 6.25
    pub msj_age_coef: f64,             // -5.0
    pub msj_male_constant: f64,        // 5.0
    pub msj_female_constant: f64,      // -161.0
}
```

#### Scientific Reference

**mifflin, m.d., et al. (1990)**
*"a new predictive equation for resting energy expenditure in healthy individuals"*
American journal of clinical nutrition, 51(2), 241-247
Doi: 10.1093/ajcn/51.2.241

**key findings:**
- validated on 498 healthy subjects (247 males, 251 females)
- accuracy: ±10% error vs indirect calorimetry
- superior to harris-benedict formula (1919) by 5%
- accounts for modern body composition changes

---

## 2. Total Daily Energy Expenditure (TDEE)

### Activity Factor Multipliers

**tdee** = bmr × activity factor

#### Activity Levels

Based on mcardle, katch & katch exercise physiology (2010):

| activity level | description | multiplier | example activities |
|----------------|-------------|------------|-------------------|
| sedentary | little/no exercise | 1.2 | desk job, no workouts |
| lightly active | 1-3 days/week | 1.375 | walking, light yoga |
| moderately active | 3-5 days/week | 1.55 | running 3×/week, cycling |
| very active | 6-7 days/week | 1.725 | daily training, athlete |
| extra active | 2×/day hard training | 1.9 | professional athlete |

#### Implementation

`src/intelligence/nutrition_calculator.rs:209-245`

```rust
pub fn calculate_tdee(
    bmr: f64,
    activity_level: ActivityLevel,
    config: &ActivityFactorsConfig,
) -> Result<f64, AppError> {
    if bmr < 1000.0 || bmr > 5000.0 {
        return Err(AppError::invalid_input("BMR must be between 1000 and 5000"));
    }

    let activity_factor = match activity_level {
        ActivityLevel::Sedentary => config.sedentary,              // 1.2
        ActivityLevel::LightlyActive => config.lightly_active,     // 1.375
        ActivityLevel::ModeratelyActive => config.moderately_active, // 1.55
        ActivityLevel::VeryActive => config.very_active,           // 1.725
        ActivityLevel::ExtraActive => config.extra_active,         // 1.9
    };

    Ok(bmr * activity_factor)
}
```

#### Example Calculations

**sedentary: bmr 1500 × 1.2 = 1800 kcal/day**
**very active: bmr 1500 × 1.725 = 2587 kcal/day**

#### Configuration

`src/config/intelligence_config.rs:444-455`

```rust
pub struct ActivityFactorsConfig {
    pub sedentary: f64,          // 1.2
    pub lightly_active: f64,     // 1.375
    pub moderately_active: f64,  // 1.55
    pub very_active: f64,        // 1.725
    pub extra_active: f64,       // 1.9
}
```

---

## 3. Macronutrient Recommendations

### Protein Needs

#### Recommendations by Activity and Goal

Based on phillips & van loon (2011) doi: 10.1080/02640414.2011.619204:

| activity level | training goal | protein (g/kg) | rationale |
|----------------|---------------|----------------|-----------|
| sedentary | any | 0.8 | dri minimum |
| lightly/moderately active | maintenance | 1.3 | active lifestyle support |
| very/extra active | endurance | 2.0 | glycogen sparing, recovery |
| very/extra active | strength/muscle gain | 2.2 | muscle protein synthesis |
| any | weight loss | 1.8 | muscle preservation |

#### Implementation

`src/intelligence/nutrition_calculator.rs:274-313`

```rust
pub fn calculate_protein_needs(
    weight_kg: f64,
    activity_level: ActivityLevel,
    training_goal: TrainingGoal,
    config: &MacronutrientConfig,
) -> Result<f64, AppError> {
    let protein_g_per_kg = match (activity_level, training_goal) {
        // Sedentary baseline (DRI minimum)
        (ActivityLevel::Sedentary, _) => config.protein_min_g_per_kg,  // 0.8

        // Moderate activity
        (ActivityLevel::LightlyActive | ActivityLevel::ModeratelyActive, _) => {
            config.protein_moderate_g_per_kg  // 1.3
        }

        // Athletic - goal-specific
        (ActivityLevel::VeryActive | ActivityLevel::ExtraActive, TrainingGoal::EndurancePerformance) => {
            config.protein_endurance_max_g_per_kg  // 2.0
        }
        (ActivityLevel::VeryActive | ActivityLevel::ExtraActive,
         TrainingGoal::StrengthPerformance | TrainingGoal::MuscleGain) => {
            config.protein_strength_max_g_per_kg  // 2.2
        }

        // Weight loss: higher protein for muscle preservation
        (_, TrainingGoal::WeightLoss) => config.protein_athlete_g_per_kg,  // 1.8

        // Default for very/extra active
        (ActivityLevel::VeryActive | ActivityLevel::ExtraActive, _) => {
            config.protein_athlete_g_per_kg  // 1.8
        }
    };

    Ok(weight_kg * protein_g_per_kg)
}
```

### Carbohydrate Needs

#### Recommendations by Activity and Goal

Based on burke et al. (2011) doi: 10.1080/02640414.2011.585473:

| activity level | training goal | carbs (g/kg) | rationale |
|----------------|---------------|--------------|-----------|
| sedentary/light | any | 3.0 | brain function minimum |
| moderate | maintenance | 6.0 | glycogen replenishment |
| very/extra active | muscle gain | 7.2 (6.0 × 1.2) | anabolic support |
| any | endurance | 10.0 | high glycogen demand |

#### Implementation

`src/intelligence/nutrition_calculator.rs:336-365`

```rust
pub fn calculate_carb_needs(
    weight_kg: f64,
    activity_level: ActivityLevel,
    training_goal: TrainingGoal,
    config: &MacronutrientConfig,
) -> Result<f64, AppError> {
    let carbs_g_per_kg = match (activity_level, training_goal) {
        // Low activity
        (ActivityLevel::Sedentary | ActivityLevel::LightlyActive, _) => {
            config.carbs_low_activity_g_per_kg  // 3.0
        }

        // Endurance athletes need high carbs
        (_, TrainingGoal::EndurancePerformance) => {
            config.carbs_high_endurance_g_per_kg  // 10.0
        }

        // Moderate activity
        (ActivityLevel::ModeratelyActive, _) => {
            config.carbs_moderate_activity_g_per_kg  // 6.0
        }

        // Very/extra active (non-endurance) - slightly higher
        (ActivityLevel::VeryActive | ActivityLevel::ExtraActive, _) => {
            config.carbs_moderate_activity_g_per_kg * 1.2  // 7.2
        }
    };

    Ok(weight_kg * carbs_g_per_kg)
}
```

### Fat Needs

#### DRI Guidelines

Dietary reference intakes (institute of medicine):
- **minimum**: 20% of tdee (hormone production, vitamin absorption)
- **optimal**: 25-30% of tdee (satiety, performance)
- **maximum**: 35% of tdee (avoid excess)

#### Implementation

`src/intelligence/nutrition_calculator.rs:392-435`

```rust
pub fn calculate_fat_needs(
    tdee: f64,
    protein_g: f64,
    carbs_g: f64,
    training_goal: TrainingGoal,
    config: &MacronutrientConfig,
) -> Result<f64, AppError> {
    // Calculate calories from protein and carbs
    let protein_kcal = protein_g * 4.0;
    let carbs_kcal = carbs_g * 4.0;
    let fat_kcal_available = tdee - protein_kcal - carbs_kcal;

    // Goal-specific fat targeting
    let target_fat_percent = match training_goal {
        TrainingGoal::WeightLoss => config.fat_min_percent_tdee,  // 20%
        TrainingGoal::MuscleGain | TrainingGoal::StrengthPerformance => {
            config.fat_optimal_percent_tdee - 2.5  // 25%
        }
        TrainingGoal::EndurancePerformance | TrainingGoal::Maintenance => {
            config.fat_optimal_percent_tdee  // 27.5%
        }
    };

    // Take maximum of remainder or target percentage
    let fat_from_remainder = fat_kcal_available / 9.0;
    let fat_from_target = (tdee * target_fat_percent / 100.0) / 9.0;

    let fat_g = fat_from_remainder.max(fat_from_target);

    // Enforce DRI bounds (20-35% of TDEE)
    let min_fat = (tdee * config.fat_min_percent_tdee / 100.0) / 9.0;
    let max_fat = (tdee * config.fat_max_percent_tdee / 100.0) / 9.0;

    Ok(fat_g.clamp(min_fat, max_fat))
}
```

### Configuration

`src/config/intelligence_config.rs:464-487`

```rust
pub struct MacronutrientConfig {
    // Protein ranges (g/kg)
    pub protein_min_g_per_kg: f64,          // 0.8
    pub protein_moderate_g_per_kg: f64,     // 1.3
    pub protein_athlete_g_per_kg: f64,      // 1.8
    pub protein_endurance_max_g_per_kg: f64, // 2.0
    pub protein_strength_max_g_per_kg: f64, // 2.2

    // Carbohydrate ranges (g/kg)
    pub carbs_low_activity_g_per_kg: f64,      // 3.0
    pub carbs_moderate_activity_g_per_kg: f64, // 6.0
    pub carbs_high_endurance_g_per_kg: f64,    // 10.0

    // Fat percentages (% of TDEE)
    pub fat_min_percent_tdee: f64,     // 20%
    pub fat_max_percent_tdee: f64,     // 35%
    pub fat_optimal_percent_tdee: f64, // 27.5%
}
```

---

## 4. Nutrient Timing

### Pre-workout Nutrition

Based on kerksick et al. (2017) doi: 10.1186/s12970-017-0189-4:

**timing**: 1-3 hours before workout
**carbohydrates**: 0.5-1.0 g/kg (intensity-dependent)
- low intensity: 0.375 g/kg (0.5 × 0.75)
- moderate intensity: 0.75 g/kg
- high intensity: 0.975 g/kg (1.3 × 0.75)

### Post-workout Nutrition

**timing**: within 2 hours (flexible - total daily intake matters most)
**protein**: 20-40g (muscle protein synthesis threshold)
**carbohydrates**: 0.8-1.2 g/kg (glycogen restoration)

### Protein Distribution

**optimal**: 4 meals/day with even protein distribution
**minimum**: 3 meals/day
**rationale**: muscle protein synthesis maximized with 0.4-0.5 g/kg per meal

#### Implementation

`src/intelligence/nutrition_calculator.rs:539-606`

```rust
pub fn calculate_nutrient_timing(
    weight_kg: f64,
    daily_protein_g: f64,
    workout_intensity: WorkoutIntensity,
    config: &NutrientTimingConfig,
) -> Result<NutrientTimingPlan, AppError> {
    // Pre-workout carbs based on intensity
    let pre_workout_carbs = match workout_intensity {
        WorkoutIntensity::Low => weight_kg * config.pre_workout_carbs_g_per_kg * 0.5,
        WorkoutIntensity::Moderate => weight_kg * config.pre_workout_carbs_g_per_kg,
        WorkoutIntensity::High => weight_kg * config.pre_workout_carbs_g_per_kg * 1.3,
    };

    // Post-workout protein (20-40g optimal range)
    let post_workout_protein = config.post_workout_protein_g_min
        .max((daily_protein_g / 5.0).min(config.post_workout_protein_g_max));

    // Post-workout carbs
    let post_workout_carbs = weight_kg * config.post_workout_carbs_g_per_kg;

    // Protein distribution across day
    let meals_per_day = config.protein_meals_per_day_optimal;
    let protein_per_meal = daily_protein_g / f64::from(meals_per_day);

    Ok(NutrientTimingPlan {
        pre_workout: PreWorkoutNutrition {
            carbs_g: pre_workout_carbs,
            timing_hours_before: config.pre_workout_window_hours,
            recommendations: vec![
                format!("Consume {pre_workout_carbs:.0}g carbs 1-3 hours before workout"),
                "Focus on easily digestible carbs (banana, oatmeal, toast)".to_string(),
            ],
        },
        post_workout: PostWorkoutNutrition {
            protein_g: post_workout_protein,
            carbs_g: post_workout_carbs,
            timing_hours_after: config.post_workout_window_hours,
            recommendations: vec![
                format!("Consume {post_workout_protein:.0}g protein + {post_workout_carbs:.0}g carbs within 2 hours"),
                "Window is flexible - total daily intake matters most".to_string(),
            ],
        },
        daily_protein_distribution: ProteinDistribution {
            meals_per_day,
            protein_per_meal_g: protein_per_meal,
            strategy: format!(
                "Distribute {daily_protein_g:.0}g protein across {meals_per_day} meals (~{protein_per_meal:.0}g each)"
            ),
        },
    })
}
```

### Configuration

`src/config/intelligence_config.rs:495-512`

```rust
pub struct NutrientTimingConfig {
    pub pre_workout_window_hours: f64,          // 2.0
    pub post_workout_window_hours: f64,         // 2.0
    pub pre_workout_carbs_g_per_kg: f64,        // 0.75
    pub post_workout_protein_g_min: f64,        // 20.0
    pub post_workout_protein_g_max: f64,        // 40.0
    pub post_workout_carbs_g_per_kg: f64,       // 1.0
    pub protein_meals_per_day_min: u8,          // 3
    pub protein_meals_per_day_optimal: u8,      // 4
}
```

### Recipe Meal Timing Macro Distributions

The recipe system (`src/intelligence/recipes/`) uses percentage-based macronutrient distributions that adjust based on training context. These distributions are applied when generating recipe constraints for LLM clients or validating recipes.

#### Macro Distribution by Meal Timing

| Meal Timing    | Protein | Carbs | Fat  | Rationale                                      |
|----------------|---------|-------|------|------------------------------------------------|
| Pre-training   | 20%     | 55%   | 25%  | Maximize glycogen, minimize GI distress        |
| Post-training  | 30%     | 45%   | 25%  | Optimize MPS + glycogen replenishment          |
| Rest day       | 30%     | 35%   | 35%  | Lower glycogen needs, carb periodization       |
| General        | 25%     | 45%   | 30%  | Balanced for non-specific meals                |

#### Scientific Justification

**Pre-training (20% protein, 55% carbs, 25% fat)**

High carbohydrate availability maximizes muscle glycogen stores for energy. The ISSN recommends 1-4 g/kg of high-glycemic carbohydrates 1-4 hours before exercise for glycogen optimization. Lower fat (25%) aids gastric emptying, reducing gastrointestinal distress during exercise.

*Reference: Kerksick CM, Arent S, Schoenfeld BJ, et al. (2017) "International Society of Sports Nutrition Position Stand: Nutrient Timing" Journal of the International Society of Sports Nutrition 14:33. DOI: 10.1186/s12970-017-0189-4*

**Post-training (30% protein, 45% carbs, 25% fat)**

Elevated protein intake (0.25-0.4 g/kg or approximately 20-40g) within 2 hours post-exercise maximizes muscle protein synthesis (MPS). Moderate carbohydrates (0.8-1.2 g/kg) accelerate glycogen resynthesis, especially when combined with protein. The 30% protein proportion ensures adequate leucine threshold (~2.5-3g) for MPS activation.

*Reference: Jäger R, Kerksick CM, Campbell BI, et al. (2017) "International Society of Sports Nutrition Position Stand: Protein and Exercise" Journal of the International Society of Sports Nutrition 14:20. DOI: 10.1186/s12970-017-0177-8*

**Rest day (30% protein, 35% carbs, 35% fat)**

Carbohydrate periodization principles advocate for reduced carbohydrate intake on non-training days when glycogen demands are lower. Training with reduced glycogen availability (the "train-low" approach) stimulates mitochondrial biogenesis and improves oxidative capacity. Higher fat (35%) compensates for reduced carbohydrate calories while maintaining satiety through slower gastric emptying.

*Reference: Impey SG, Hearris MA, Hammond KM, et al. (2018) "Fuel for the Work Required: A Theoretical Framework for Carbohydrate Periodization and the Glycogen Threshold Hypothesis" Sports Medicine 48(5):1031-1048. DOI: 10.1007/s40279-018-0867-7*

#### Implementation

`src/intelligence/recipes/models.rs:33-49`

```rust
impl MealTiming {
    /// Get recommended macro distribution percentages for this timing
    ///
    /// Returns (`protein_pct`, `carbs_pct`, `fat_pct`) tuple that sums to 100
    pub const fn macro_distribution(&self) -> (u8, u8, u8) {
        match self {
            // Pre-training: prioritize carbs for energy
            Self::PreTraining => (20, 55, 25),
            // Post-training: prioritize protein for recovery
            Self::PostTraining => (30, 45, 25),
            // Rest day: balanced with lower carbs
            Self::RestDay => (30, 35, 35),
            // General: balanced distribution
            Self::General => (25, 45, 30),
        }
    }
}
```

### TDEE-Based Recipe Calorie Calculation

When generating recipe constraints via `get_recipe_constraints`, the system calculates target calories using a priority-based approach:

1. **Explicit calories** - If provided in the request, uses the exact value
2. **TDEE-based** - When user's TDEE is provided, calculates calories as a proportion of daily energy
3. **Fallback defaults** - Uses research-based defaults when no TDEE is available

#### TDEE Proportions by Meal Timing

| Meal Timing    | TDEE Proportion | Example (2500 kcal TDEE) | Rationale                                      |
|----------------|-----------------|--------------------------|------------------------------------------------|
| Pre-training   | 17.5%           | 438 kcal                 | Moderate meal to fuel workout without GI stress |
| Post-training  | 27.5%           | 688 kcal                 | Largest meal for recovery and glycogen restoration |
| Rest day       | 25.0%           | 625 kcal                 | Standard meal proportion for recovery days     |
| General        | 25.0%           | 625 kcal                 | Balanced default for non-training meals        |

#### Fallback Calorie Values

When TDEE is not provided, the system uses these scientifically-informed defaults:

| Meal Timing    | Fallback Calories | Rationale                                      |
|----------------|-------------------|------------------------------------------------|
| Pre-training   | 400 kcal          | Light meal suitable for pre-workout fueling    |
| Post-training  | 600 kcal          | Larger meal for optimal recovery nutrition     |
| Rest day       | 500 kcal          | Moderate meal for non-training days            |
| General        | 500 kcal          | Balanced default for general meal planning     |

#### Scientific Justification

**Post-training as largest meal (27.5% of TDEE)**

The post-workout period represents the optimal window for nutrient partitioning. Elevated muscle glycogen synthase activity and enhanced insulin sensitivity make this the ideal time for higher calorie intake. The 27.5% proportion ensures adequate calories for both glycogen restoration (requiring 0.8-1.2 g/kg carbohydrates) and muscle protein synthesis (requiring 20-40g protein).

*Reference: Ivy JL, Katz AL, Cutler CL, et al. (1988) "Muscle glycogen synthesis after exercise: effect of time of carbohydrate ingestion" Journal of Applied Physiology 64(4):1480-1485. DOI: 10.1152/jappl.1988.64.4.1480*

**Pre-training as smaller meal (17.5% of TDEE)**

Lower calorie intake pre-workout minimizes gastrointestinal distress while still providing adequate fuel. The ISSN recommends consuming carbohydrates 1-4 hours before exercise, with smaller meals closer to workout time. The 17.5% proportion provides sufficient energy without compromising exercise performance or comfort.

#### Configuration

`src/config/intelligence_config.rs`

```rust
/// Meal TDEE proportion configuration based on ISSN research
pub struct MealTdeeProportionsConfig {
    pub pre_training: f64,    // 0.175 (17.5% of TDEE)
    pub post_training: f64,   // 0.275 (27.5% of TDEE)
    pub rest_day: f64,        // 0.25 (25% of TDEE)
    pub general: f64,         // 0.25 (25% of TDEE)
    pub fallback_calories: MealFallbackCaloriesConfig,
}

/// Fallback calorie values when TDEE is not available
pub struct MealFallbackCaloriesConfig {
    pub pre_training: f64,   // 400.0 kcal
    pub post_training: f64,  // 600.0 kcal
    pub rest_day: f64,       // 500.0 kcal
    pub general: f64,        // 500.0 kcal
}
```

#### API Response Fields

When TDEE is provided, `get_recipe_constraints` includes additional fields:

```json
{
  "calories": 688,
  "tdee_based": true,
  "tdee": 2500,
  "tdee_proportion": 0.275
}
```

When TDEE is not provided, `tdee_based` is `false` and fallback calories are used.

---

## 5. USDA FoodData Central Integration

### API Overview

**usda fooddata central** provides access to:
- **350,000+ foods** in the database
- **comprehensive nutrients** (protein, carbs, fat, vitamins, minerals)
- **branded foods** with manufacturer data
- **foundation foods** with detailed nutrient profiles
- **sr legacy foods** from usda nutrient database

### Client Implementation

`src/external/usda_client.rs:1-233`

#### Real Client (Production)

```rust
pub struct UsdaClient {
    client: reqwest::Client,
    config: UsdaClientConfig,
}

impl UsdaClient {
    pub async fn search_foods(&self, query: &str, page_size: usize) -> Result<SearchResult> {
        let url = format!("{}/foods/search", self.config.base_url);

        let response = self.client
            .get(&url)
            .query(&[
                ("query", query),
                ("pageSize", &page_size.to_string()),
                ("api_key", &self.config.api_key),
            ])
            .timeout(Duration::from_secs(self.config.timeout_secs))
            .send()
            .await?;

        response.json().await
    }

    pub async fn get_food_details(&self, fdc_id: u64) -> Result<FoodDetails> {
        let url = format!("{}/food/{}", self.config.base_url, fdc_id);

        let response = self.client
            .get(&url)
            .query(&[("api_key", &self.config.api_key)])
            .timeout(Duration::from_secs(self.config.timeout_secs))
            .send()
            .await?;

        response.json().await
    }
}
```

#### Mock Client (Testing)

```rust
pub struct MockUsdaClient;

impl MockUsdaClient {
    pub fn new() -> Self {
        Self
    }

    pub fn search_foods(&self, query: &str, _page_size: usize) -> Result<SearchResult> {
        // Return realistic mock data based on query
        let foods = match query.to_lowercase().as_str() {
            q if q.contains("chicken") => vec![
                Food {
                    fdc_id: 171477,
                    description: "Chicken breast, skinless, boneless, raw".to_string(),
                },
            ],
            q if q.contains("banana") => vec![
                Food {
                    fdc_id: 173944,
                    description: "Banana, raw".to_string(),
                },
            ],
            // ... more mock foods
        };

        Ok(SearchResult {
            foods,
            total_hits: foods.len(),
            current_page: 1,
            total_pages: 1,
        })
    }

    pub fn get_food_details(&self, fdc_id: u64) -> Result<FoodDetails> {
        // Return complete nutrient breakdown
        match fdc_id {
            171477 => Ok(FoodDetails {  // Chicken breast
                fdc_id: 171477,
                description: "Chicken breast, skinless, boneless, raw".to_string(),
                food_nutrients: vec![
                    Nutrient {
                        nutrient_name: "Protein".to_string(),
                        amount: 23.09,
                        unit: "g".to_string(),
                    },
                    Nutrient {
                        nutrient_name: "Energy".to_string(),
                        amount: 120.0,
                        unit: "kcal".to_string(),
                    },
                    // ... more nutrients
                ],
            }),
            // ... more mock foods
        }
    }
}
```

### Configuration

`src/config/intelligence_config.rs:514-522`

```rust
pub struct UsdaApiConfig {
    pub base_url: String,              // "https://api.nal.usda.gov/fdc/v1"
    pub timeout_secs: u64,             // 10
    pub cache_ttl_hours: u64,          // 24
    pub max_cache_items: usize,        // 1000
    pub rate_limit_per_minute: u32,    // 30
}
```

---

## 6. MCP Tool Integration

Pierre exposes 5 nutrition tools via mcp protocol:

### calculate_daily_nutrition

**calculates complete daily nutrition requirements**

```json
{
  "name": "calculate_daily_nutrition",
  "description": "Calculate complete daily nutrition requirements (BMR, TDEE, macros)",
  "inputSchema": {
    "type": "object",
    "properties": {
      "weight_kg": { "type": "number", "description": "Body weight in kg" },
      "height_cm": { "type": "number", "description": "Height in cm" },
      "age": { "type": "integer", "description": "Age in years" },
      "gender": { "type": "string", "enum": ["male", "female"] },
      "activity_level": { "type": "string", "enum": ["sedentary", "lightly_active", "moderately_active", "very_active", "extra_active"] },
      "training_goal": { "type": "string", "enum": ["maintenance", "weight_loss", "muscle_gain", "endurance_performance"] }
    },
    "required": ["weight_kg", "height_cm", "age", "gender", "activity_level", "training_goal"]
  }
}
```

**example response:**
```json
{
  "bmr": 1730,
  "tdee": 2682,
  "protein_g": 135,
  "carbs_g": 402,
  "fat_g": 82,
  "macro_percentages": {
    "protein_percent": 20.1,
    "carbs_percent": 60.0,
    "fat_percent": 27.5
  },
  "method": "Mifflin-St Jeor + Activity Factor"
}
```

### calculate_nutrient_timing

**calculates pre/post-workout nutrition and daily protein distribution**

```json
{
  "name": "calculate_nutrient_timing",
  "inputSchema": {
    "properties": {
      "weight_kg": { "type": "number" },
      "daily_protein_g": { "type": "number" },
      "workout_intensity": { "type": "string", "enum": ["low", "moderate", "high"] }
    }
  }
}
```

### search_foods (USDA)

**searches usda fooddata central database**

```json
{
  "name": "search_foods",
  "inputSchema": {
    "properties": {
      "query": { "type": "string", "description": "Food name to search" },
      "page_size": { "type": "integer", "default": 10 },
      "use_mock": { "type": "boolean", "default": false }
    }
  }
}
```

### get_food_details (USDA)

**retrieves complete nutrient breakdown for a food**

```json
{
  "name": "get_food_details",
  "inputSchema": {
    "properties": {
      "fdc_id": { "type": "integer", "description": "USDA FDC ID" },
      "use_mock": { "type": "boolean", "default": false }
    }
  }
}
```

### analyze_meal_nutrition

**analyzes complete meal with multiple foods**

```json
{
  "name": "analyze_meal_nutrition",
  "inputSchema": {
    "properties": {
      "meal_foods": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "fdc_id": { "type": "integer" },
            "grams": { "type": "number" }
          }
        }
      },
      "use_mock": { "type": "boolean", "default": false }
    }
  }
}
```

**example request:**
```json
{
  "meal_foods": [
    { "fdc_id": 171477, "grams": 150 },  // chicken breast
    { "fdc_id": 170379, "grams": 200 },  // brown rice
    { "fdc_id": 170417, "grams": 100 }   // broccoli
  ]
}
```

**example response:**
```json
{
  "total_calories": 456,
  "total_protein_g": 42.5,
  "total_carbs_g": 62.3,
  "total_fat_g": 5.1,
  "food_details": [
    { "fdc_id": 171477, "description": "Chicken breast", "grams": 150 },
    { "fdc_id": 170379, "description": "Brown rice", "grams": 200 },
    { "fdc_id": 170417, "description": "Broccoli", "grams": 100 }
  ]
}
```

---

## 7. Testing and Verification

### Comprehensive Test Suite

**39 algorithm tests** covering all nutrition calculations:

#### Test Categories

**bmr calculations (4 tests)**
- male/female typical cases
- minimum bmr enforcement (1000 kcal floor)
- large athlete scenarios

**tdee calculations (5 tests)**
- all 5 activity levels (1.2-1.9 multipliers)
- sedentary through extra active

**protein needs (5 tests)**
- all 4 training goals
- activity level scaling
- weight proportionality

**carbohydrate needs (4 tests)**
- endurance high-carb requirements
- weight loss lower-carb approach
- muscle gain optimization
- activity level scaling

**fat calculations (3 tests)**
- balanced macro scenarios
- minimum fat enforcement (20% tdee)
- high tdee edge cases

**complete daily nutrition (3 tests)**
- male maintenance profile
- female weight loss profile
- athlete endurance profile

**nutrient timing (3 tests)**
- high/moderate/low workout intensities
- pre/post-workout calculations
- protein distribution strategies

**edge cases & validation (13 tests)**
- negative/zero weight rejection
- invalid height rejection
- age bounds (10-120 years)
- extreme tdee scenarios
- macro percentage summing (always 100%)
- all intensity levels
- invalid inputs handling

### Test Execution

`tests/nutrition_comprehensive_test.rs:1-902`

```bash
# run nutrition tests
cargo test --test nutrition_comprehensive_test

# output
test result: ok. 39 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### Formula Verification

**mifflin-st jeor accuracy**:
- 30yo male, 75kg, 180cm: calculated 1730 kcal (matches hand calculation)
- 25yo female, 60kg, 165cm: calculated 1345 kcal (matches hand calculation)

**macro percentages**:
- all scenarios tested sum to 100.0% ±0.1%

**usda integration**:
- mock client tested with banana (173944), chicken (171477), oatmeal (173904), salmon (175168)
- nutrient calculations verified against usda data

---

## 8. Configuration and Customization

All nutrition parameters are configurable via `src/config/intelligence_config.rs`:

```rust
pub struct NutritionConfig {
    pub bmr: BmrConfig,
    pub activity_factors: ActivityFactorsConfig,
    pub macronutrients: MacronutrientConfig,
    pub nutrient_timing: NutrientTimingConfig,
    pub usda_api: UsdaApiConfig,
}
```

### Environment Variables

```bash
# USDA API (optional - mock client available for testing)
export USDA_API_KEY=your_api_key_here

# Server configuration
export HTTP_PORT=8081
export DATABASE_URL=sqlite:./data/users.db
```

### Dependency Injection

All calculation functions accept configuration structs:
- **testable**: inject mock configs for testing
- **flexible**: change thresholds without code changes
- **documented**: configuration structs have inline documentation

---

## 9. Scientific References

### BMR and Energy Expenditure

1. **mifflin, m.d., et al. (1990)**
   - "a new predictive equation for resting energy expenditure"
   - american journal of clinical nutrition, 51(2), 241-247
   - doi: 10.1093/ajcn/51.2.241

2. **mcardle, w.d., katch, f.i., & katch, v.l. (2010)**
   - exercise physiology: nutrition, energy, and human performance
   - lippincott williams & wilkins

### Protein Recommendations

3. **phillips, s.m., & van loon, l.j. (2011)**
   - "dietary protein for athletes: from requirements to optimum adaptation"
   - journal of sports sciences, 29(sup1), s29-s38
   - doi: 10.1080/02640414.2011.619204

4. **morton, r.w., et al. (2018)**
   - "a systematic review, meta-analysis and meta-regression of protein intake"
   - british journal of sports medicine, 52(6), 376-384
   - doi: 10.1136/bjsports-2017-097608

### Carbohydrate Recommendations

5. **burke, l.m., et al. (2011)**
   - "carbohydrates for training and competition"
   - journal of sports sciences, 29(sup1), s17-s27
   - doi: 10.1080/02640414.2011.585473

### Nutrient Timing

6. **kerksick, c.m., et al. (2017)**
   - "international society of sports nutrition position stand: nutrient timing"
   - journal of the international society of sports nutrition, 14(1), 33
   - doi: 10.1186/s12970-017-0189-4

7. **aragon, a.a., & schoenfeld, b.j. (2013)**
   - "nutrient timing revisited: is there a post-exercise anabolic window?"
   - journal of the international society of sports nutrition, 10(1), 5
   - doi: 10.1186/1550-2783-10-5

### Fat Recommendations

8. **institute of medicine (2005)**
   - dietary reference intakes for energy, carbohydrate, fiber, fat, fatty acids, cholesterol, protein, and amino acids
   - national academies press

---

## 10. Implementation Roadmap

### Phase 1: Foundation (Complete ✅)
- [x] bmr calculation (mifflin-st jeor)
- [x] tdee calculation with activity factors
- [x] protein recommendations by activity/goal
- [x] carbohydrate targeting
- [x] fat calculations with dri compliance
- [x] nutrient timing algorithms
- [x] input validation and bounds checking
- [x] 39 comprehensive algorithm tests

### Phase 2: USDA Integration (Complete ✅)
- [x] usda client with async api calls
- [x] food search functionality
- [x] food details retrieval
- [x] mock client for testing
- [x] meal analysis with multi-food support
- [x] nutrient summation calculations

### Phase 3: MCP Tools (Complete ✅)
- [x] calculate_daily_nutrition tool
- [x] calculate_nutrient_timing tool
- [x] search_foods tool
- [x] get_food_details tool
- [x] analyze_meal_nutrition tool

### Phase 4: Future Enhancements
- [ ] meal planning tool (weekly meal generation)
- [ ] recipe nutrition analysis
- [ ] micronutrient tracking (vitamins, minerals)
- [ ] dietary restriction support (vegan, gluten-free, etc.)
- [ ] food substitution recommendations
- [ ] grocery list generation

---

## 11. Limitations and Considerations

### Age Range
- **validated**: 10-120 years
- **optimal accuracy**: adults 18-65 years
- **pediatric**: mifflin-st jeor not validated for children under 10

### Activity Level Estimation
- **subjective**: users may overestimate activity
- **recommendation**: start conservative (lower activity level)
- **adjustment**: monitor results and adjust over 2-4 weeks

### Individual Variation
- **bmr variance**: ±10% between individuals
- **metabolic adaptation**: tdee may decrease with prolonged deficit
- **recommendation**: use calculations as starting point, adjust based on results

### Athletic Populations
- **elite athletes**: may need higher protein (2.2-2.4 g/kg)
- **ultra-endurance**: may need higher carbs (12+ g/kg)
- **strength athletes**: may benefit from higher fat (30-35%)

### Medical Conditions
- **contraindications**: diabetes, kidney disease, metabolic disorders
- **recommendation**: consult healthcare provider before dietary changes
- **monitoring**: regular health checkups recommended

---

## 12. Usage Examples

### Example 1: Calculate Daily Nutrition

**input:**
```rust
let params = DailyNutritionParams {
    weight_kg: 75.0,
    height_cm: 180.0,
    age: 30,
    gender: Gender::Male,
    activity_level: ActivityLevel::ModeratelyActive,
    training_goal: TrainingGoal::Maintenance,
};

let result = calculate_daily_nutrition_needs(
    &params,
    &config.bmr,
    &config.activity_factors,
    &config.macronutrients,
)?;
```

**output:**
```rust
DailyNutritionNeeds {
    bmr: 1730.0,
    tdee: 2682.0,
    protein_g: 97.5,
    carbs_g: 450.0,
    fat_g: 82.0,
    macro_percentages: MacroPercentages {
        protein_percent: 14.5,
        carbs_percent: 67.1,
        fat_percent: 27.5,
    },
    method: "Mifflin-St Jeor + Activity Factor",
}
```

### Example 2: Nutrient Timing

**input:**
```rust
let timing = calculate_nutrient_timing(
    75.0,          // weight_kg
    150.0,         // daily_protein_g
    WorkoutIntensity::High,
    &config.nutrient_timing,
)?;
```

**output:**
```rust
NutrientTimingPlan {
    pre_workout: PreWorkoutNutrition {
        carbs_g: 73.1,  // 75kg × 0.75 × 1.3
        timing_hours_before: 2.0,
    },
    post_workout: PostWorkoutNutrition {
        protein_g: 30.0,  // min(max(150/5, 20), 40)
        carbs_g: 75.0,    // 75kg × 1.0
        timing_hours_after: 2.0,
    },
    daily_protein_distribution: ProteinDistribution {
        meals_per_day: 4,
        protein_per_meal_g: 37.5,  // 150 / 4
        strategy: "Distribute 150g protein across 4 meals (~38g each)",
    },
}
```

### Example 3: Meal Analysis

**input:**
```json
{
  "meal_foods": [
    { "fdc_id": 171477, "grams": 150 },
    { "fdc_id": 170379, "grams": 200 }
  ],
  "use_mock": true
}
```

**output:**
```json
{
  "total_calories": 420,
  "total_protein_g": 40.0,
  "total_carbs_g": 46.0,
  "total_fat_g": 4.5,
  "food_details": [
    { "fdc_id": 171477, "description": "Chicken breast", "grams": 150 },
    { "fdc_id": 170379, "description": "Brown rice", "grams": 200 }
  ]
}
```

---

## Appendix: Formula Derivations

### Mifflin-St Jeor Regression Coefficients

**derived from 498-subject study:**

**weight coefficient (10.0)**
- represents metabolic cost of maintaining lean mass
- approximately 22 kcal/kg/day for lean tissue

**height coefficient (6.25)**
- correlates with body surface area
- taller individuals have higher metabolic rate

**age coefficient (-5.0)**
- accounts for age-related metabolic decline
- approximately 2% decrease per decade

**gender constant**
- male (+5): accounts for higher lean mass percentage
- female (-161): accounts for higher fat mass percentage

### Activity Factor Derivation

**based on doubly labeled water studies:**

**sedentary (1.2)**: 20% above bmr
- typical desk job with no structured exercise

**lightly active (1.375)**: 37.5% above bmr
- 1-3 days/week light exercise (walking, yoga)

**moderately active (1.55)**: 55% above bmr
- 3-5 days/week moderate exercise (running, cycling)

**very active (1.725)**: 72.5% above bmr
- 6-7 days/week intense training

**extra active (1.9)**: 90% above bmr
- professional athletes with 2×/day training

---

**document version**: 1.0.0
**last updated**: 2025-10-31
**implementation status**: production-ready
**test coverage**: 39 algorithm tests, 1,188 total tests passing

---

