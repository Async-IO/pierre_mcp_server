<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Chapter 19: Comprehensive Tools Guide - All 45 MCP Tools

This chapter provides a complete reference to all 45 MCP tools Pierre offers for fitness data analysis. You'll learn tool categories, natural language prompt examples, and how AI assistants discover and use these tools.

## What You'll Learn

- Complete tool inventory (45 tools)
- Tool categorization (6 categories)
- Natural language prompt patterns
- Tool discovery via `tools/list`
- Parameter schemas and validation
- Connection vs analytics vs configuration tools
- Real-world usage examples
- Tool chaining patterns

## Tool Overview

Pierre provides 45 MCP tools organized in 6 functional categories:

```
┌────────────────────────────────────────────────────────────┐
│              Pierre MCP Tools (45 total)                   │
├────────────────────────────────────────────────────────────┤
│ 1. Connection Tools (4)                                    │
│    - OAuth flow management                                 │
│    - Provider connection status                            │
├────────────────────────────────────────────────────────────┤
│ 2. Data Access Tools (8)                                   │
│    - Activities, athlete profiles, stats                   │
│    - Notifications and OAuth completion                    │
├────────────────────────────────────────────────────────────┤
│ 3. Intelligence & Analytics (13)                           │
│    - Activity analysis, metrics calculation                │
│    - Performance trends, pattern detection                 │
│    - Goal management, predictions                          │
├────────────────────────────────────────────────────────────┤
│ 4. Configuration Management (10)                           │
│    - User profiles, training zones                         │
│    - Fitness configuration CRUD                            │
├────────────────────────────────────────────────────────────┤
│ 5. Nutrition Tools (5)                                     │
│    - Daily nutrition calculations                          │
│    - USDA food database search                             │
│    - Meal analysis                                         │
├────────────────────────────────────────────────────────────┤
│ 6. Sleep & Recovery (5)                                    │
│    - Sleep quality analysis                                │
│    - Recovery score calculation                            │
│    - Rest day suggestions                                  │
└────────────────────────────────────────────────────────────┘
```

**Tool registry**:

**Source**: src/mcp/schema.rs:499-559
```rust
pub fn get_tools() -> Vec<ToolSchema> {
    create_fitness_tools()
}

fn create_fitness_tools() -> Vec<ToolSchema> {
    vec![
        // Connection tools (4)
        create_connect_to_pierre_tool(),
        create_connect_provider_tool(),
        create_get_connection_status_tool(),
        create_disconnect_provider_tool(),
        // Data Access tools (8)
        create_get_activities_tool(),
        create_get_athlete_tool(),
        create_get_stats_tool(),
        create_get_activity_intelligence_tool(),
        create_get_notifications_tool(),
        create_mark_notifications_read_tool(),
        create_announce_oauth_success_tool(),
        create_check_oauth_notifications_tool(),
        // Intelligence & Analytics (13)
        create_analyze_activity_tool(),
        create_calculate_metrics_tool(),
        create_analyze_performance_trends_tool(),
        create_compare_activities_tool(),
        create_detect_patterns_tool(),
        create_set_goal_tool(),
        create_track_progress_tool(),
        create_suggest_goals_tool(),
        create_analyze_goal_feasibility_tool(),
        create_generate_recommendations_tool(),
        create_calculate_fitness_score_tool(),
        create_predict_performance_tool(),
        create_analyze_training_load_tool(),
        // Configuration Management (10)
        create_get_configuration_catalog_tool(),
        create_get_configuration_profiles_tool(),
        create_get_user_configuration_tool(),
        create_update_user_configuration_tool(),
        create_calculate_personalized_zones_tool(),
        create_validate_configuration_tool(),
        create_get_fitness_config_tool(),
        create_set_fitness_config_tool(),
        create_list_fitness_configs_tool(),
        create_delete_fitness_config_tool(),
        // Nutrition (5)
        create_calculate_daily_nutrition_tool(),
        create_get_nutrient_timing_tool(),
        create_search_food_tool(),
        create_get_food_details_tool(),
        create_analyze_meal_nutrition_tool(),
        // Sleep & Recovery (5)
        create_analyze_sleep_quality_tool(),
        calculate_recovery_score_tool(),
        create_suggest_rest_day_tool(),
        create_track_sleep_trends_tool(),
        create_optimize_sleep_schedule_tool(),
    ]
}
```

## 1. Connection Tools (4 Tools)

These tools manage OAuth connections to Pierre and fitness providers.

### Connect_to_pierre

**Description**: Authenticate with Pierre Fitness Server to access your fitness data.

**Natural language prompts**:
- "Connect to Pierre to access my fitness data"
- "I need to log in to Pierre"
- "Authenticate with Pierre server"

**Parameters**: None (opens browser for OAuth flow)

**Use case**: First-time setup or re-authentication after token expiration.

### Connect_provider

**Description**: Connect to a fitness provider (Strava, Fitbit) via unified OAuth flow.

**Parameters**:
```json
{
  "provider": "strava"  // Required: "strava" or "fitbit"
}
```

**Natural language prompts**:
- "Connect to Strava to get my activities"
- "I want to sync my Fitbit data"
- "Link my Garmin account"

**Use case**: Initial provider connection or adding additional providers.

### Get_connection_status

**Description**: Check which fitness providers are currently connected.

**Parameters**: Optional OAuth credentials for custom apps

**Natural language prompts**:
- "Which providers am I connected to?"
- "Show my connection status"
- "Am I still connected to Strava?"

**Use case**: Verify active connections before requesting data.

### Disconnect_provider

**Description**: Revoke access tokens for a specific fitness provider.

**Parameters**:
```json
{
  "provider": "strava"  // Required
}
```

**Natural language prompts**:
- "Disconnect from Strava"
- "Remove my Fitbit connection"
- "Revoke Pierre's access to my Garmin data"

**Use case**: Privacy management, switching accounts, troubleshooting.

## 2. Data Access Tools (8 Tools)

These tools fetch raw data from connected fitness providers.

### Get_activities

**Description**: Retrieve fitness activities from a provider.

**Parameters**:
```json
{
  "provider": "strava",  // Required
  "limit": 10,           // Optional: max activities (default: 10)
  "offset": 0            // Optional: pagination offset
}
```

**Natural language prompts**:
- "Show me my last 20 Strava runs"
- "Get my recent Fitbit activities"
- "Fetch all my workouts from this month"

**Use case**: Activity listing, data exploration, trend analysis preparation.

### Get_athlete

**Description**: Get athlete profile from a provider.

**Parameters**:
```json
{
  "provider": "strava"  // Required
}
```

**Natural language prompts**:
- "Show my Strava profile"
- "What's my FTP according to Strava?"
- "Get my athlete stats"

**Use case**: Profile information, baseline metrics (FTP, max HR, weight).

### Get_stats

**Description**: Get aggregate statistics from a provider.

**Parameters**:
```json
{
  "provider": "strava"  // Required
}
```

**Natural language prompts**:
- "Show my year-to-date running totals"
- "What are my all-time cycling stats?"
- "How much have I run this month?"

**Use case**: Summary statistics, progress tracking, milestone identification.

### Get_activity_intelligence

**Description**: AI-powered insights and analysis for a specific activity.

**Parameters**:
```json
{
  "activity_id": "12345678",      // Required
  "provider": "strava",            // Required
  "include_location": true,        // Optional: location intelligence
  "include_weather": true          // Optional: weather analysis
}
```

**Natural language prompts**:
- "Analyze my last run with weather and location insights"
- "What can you tell me about activity 12345678?"
- "Give me intelligent insights on my latest ride"

**Use case**: Deep activity analysis, performance insights, environmental factors.

### Get_notifications

**Description**: Get OAuth notifications for the user.

**Parameters**:
```json
{
  "provider": "strava",     // Optional: filter by provider
  "include_read": false     // Optional: include already read (default: false)
}
```

**Natural language prompts**:
- "Do I have any new notifications?"
- "Show unread OAuth notifications"
- "Check for Strava connection updates"

**Use case**: OAuth completion tracking, connection diagnostics.

### Mark_notifications_read

**Description**: Mark OAuth notifications as read.

**Parameters**:
```json
{
  "notification_id": "abc123"  // Optional: specific notification ID
}
```

**Natural language prompts**:
- "Mark all notifications as read"
- "Clear notification abc123"
- "Dismiss OAuth notifications"

**Use case**: Notification management, clearing completed OAuth flows.

### Announce_oauth_success

**Description**: Display OAuth connection success message in chat.

**Natural language prompts**: (Typically called internally by Pierre)

**Use case**: User feedback for successful OAuth flows.

### Check_oauth_notifications

**Description**: Check for pending OAuth notifications.

**Natural language prompts**:
- "Any pending OAuth completions?"
- "Check if OAuth finished"

**Use case**: Polling for OAuth completion in SDK.

## 3. Intelligence & Analytics Tools (13 Tools)

These tools provide AI-powered analysis and insights.

### Analyze_activity

**Description**: Comprehensive analysis of a single activity.

**Natural language prompts**:
- "Analyze my activity from yesterday"
- "What insights can you give me about my last ride?"
- "Deep dive into my marathon performance"

**Use case**: Post-workout analysis, identifying strengths/weaknesses.

### Calculate_metrics

**Description**: Calculate derived metrics from activity data.

**Natural language prompts**:
- "Calculate my TSS for last week"
- "What's my Normalized Power for this ride?"
- "Compute training load metrics"

**Use case**: Advanced metrics not provided by fitness providers.

### Analyze_performance_trends

**Description**: Identify performance trends over time.

**Natural language prompts**:
- "Am I getting faster at running?"
- "Show my cycling power trends over the last 3 months"
- "Is my fitness improving?"

**Use case**: Long-term progress tracking, plateau detection.

### Compare_activities

**Description**: Compare two or more activities.

**Natural language prompts**:
- "Compare my last two 5K runs"
- "How does today's ride compare to last week?"
- "Show differences between these activities"

**Use case**: Performance comparison, identifying improvements/regressions.

### Detect_patterns

**Description**: Detect patterns in training data.

**Natural language prompts**:
- "Find patterns in my running data"
- "Do I always run faster in the morning?"
- "What training patterns lead to my best performances?"

**Use case**: Optimization insights, habit identification.

### Set_goal

**Description**: Set a fitness goal with target metrics.

**Natural language prompts**:
- "Set a goal to run a sub-20 minute 5K by June"
- "I want to cycle 200km per week"
- "Target: Complete a marathon in under 4 hours"

**Use case**: Goal management, motivation tracking.

### Track_progress

**Description**: Track progress towards goals.

**Natural language prompts**:
- "How am I progressing towards my marathon goal?"
- "Show progress on my weekly cycling target"
- "Am I on track to hit my 5K goal?"

**Use case**: Goal monitoring, progress visualization.

### Suggest_goals

**Description**: AI-suggested goals based on current fitness level.

**Natural language prompts**:
- "What goals should I set?"
- "Suggest realistic running goals for me"
- "What's achievable in the next 3 months?"

**Use case**: Goal discovery, personalized recommendations.

### Analyze_goal_feasibility

**Description**: Analyze if a goal is realistic given current fitness.

**Natural language prompts**:
- "Can I realistically run a sub-3 hour marathon?"
- "Is a 100-mile week feasible for me?"
- "Evaluate my goal to bike 50km in under 2 hours"

**Use case**: Goal validation, expectation management.

### Generate_recommendations

**Description**: Generate training recommendations.

**Natural language prompts**:
- "What should I work on to improve my cycling?"
- "Give me recommendations for faster 10K times"
- "How can I improve my marathon performance?"

**Use case**: Training advice, weakness identification.

### Calculate_fitness_score

**Description**: Calculate current fitness score.

**Natural language prompts**:
- "What's my current fitness score?"
- "Calculate my fitness level"
- "How fit am I right now?"

**Use case**: Fitness tracking, periodization planning.

### Predict_performance

**Description**: Predict performance for upcoming events.

**Natural language prompts**:
- "Predict my marathon time"
- "What pace can I sustain for a half marathon?"
- "Estimate my 5K time based on current fitness"

**Use case**: Race planning, pacing strategy.

### Analyze_training_load

**Description**: Analyze training stress and recovery needs.

**Natural language prompts**:
- "Am I overtraining?"
- "What's my current training load?"
- "Do I need a rest day?"

**Use case**: Recovery planning, injury prevention.

## 4. Configuration Management Tools (10 Tools)

These tools manage user profiles and training zones.

### Get_configuration_catalog

**Description**: List all available configuration algorithms and profiles.

**Natural language prompts**:
- "What configuration profiles are available?"
- "Show me all training zone calculation methods"

**Use case**: Discovering configuration options.

### Get_user_configuration

**Description**: Retrieve user's current configuration.

**Natural language prompts**:
- "Show my current training zones"
- "What's my configuration?"

**Use case**: Viewing active settings.

### Update_user_configuration

**Description**: Update user profile (age, weight, FTP, max HR, etc.).

**Natural language prompts**:
- "Update my FTP to 250 watts"
- "Set my max heart rate to 185"
- "Change my weight to 70kg"

**Use case**: Profile updates after fitness tests.

### Calculate_personalized_zones

**Description**: Calculate personalized training zones.

**Natural language prompts**:
- "Calculate my heart rate zones"
- "What are my power zones?"
- "Determine my pace zones"

**Use case**: Training zone setup.

## 5. Nutrition Tools (5 Tools)

These tools provide nutrition analysis and planning.

### Calculate_daily_nutrition

**Description**: Calculate daily nutrition needs.

**Natural language prompts**:
- "How many calories should I eat?"
- "Calculate my daily protein needs"
- "What are my macros?"

**Use case**: Nutrition planning based on training load.

### Search_food

**Description**: Search USDA food database.

**Natural language prompts**:
- "Search for 'banana' in the food database"
- "Find nutrition info for oatmeal"

**Use case**: Food logging, meal planning.

### Get_food_details

**Description**: Get detailed nutrition info for a food.

**Natural language prompts**:
- "Show details for food ID 123456"
- "What nutrients are in this food?"

**Use case**: Detailed nutrition analysis.

### Analyze_meal_nutrition

**Description**: Analyze complete meal nutrition.

**Natural language prompts**:
- "Analyze this meal: 100g chicken, 200g rice, 50g broccoli"
- "What's the nutritional breakdown of my lunch?"

**Use case**: Meal logging, nutrition tracking.

## 6. Sleep & Recovery Tools (5 Tools)

These tools analyze sleep and recovery metrics.

### Analyze_sleep_quality

**Description**: Analyze sleep quality and duration.

**Natural language prompts**:
- "How was my sleep last night?"
- "Analyze my sleep quality"

**Use case**: Recovery monitoring.

### Calculate_recovery_score

**Description**: Calculate recovery score based on multiple factors.

**Natural language prompts**:
- "What's my recovery score?"
- "Am I recovered enough to train hard?"

**Use case**: Training intensity planning.

### Suggest_rest_day

**Description**: Suggest if a rest day is needed.

**Natural language prompts**:
- "Do I need a rest day?"
- "Should I take it easy today?"

**Use case**: Injury prevention, overtraining avoidance.

## Tool Chaining Patterns

AI assistants often chain multiple tools together:

**Pattern 1: Connect → Fetch → Analyze**
```
User: "Analyze my recent running performance"

AI chains:
1. get_connection_status()  // Check if connected
2. get_activities(provider="strava", limit=20)  // Fetch runs
3. analyze_performance_trends()  // Analyze trends
4. generate_recommendations()  // Suggest improvements
```

**Pattern 2: Configuration → Calculation → Recommendation**
```
User: "What should my training zones be?"

AI chains:
1. get_user_configuration()  // Get FTP, max HR
2. calculate_personalized_zones()  // Calculate zones
3. generate_recommendations()  // Training advice for each zone
```

**Pattern 3: Goal Setting → Tracking → Prediction**
```
User: "Set a goal and track my progress"

AI chains:
1. suggest_goals()  // Suggest realistic goal
2. set_goal()  // Create goal
3. track_progress()  // Monitor progress
4. predict_performance()  // Estimate completion
```

## Key Takeaways

1. **45 total tools**: Organized in 6 functional categories for comprehensive fitness analysis.

2. **Natural language**: AI assistants translate user prompts to tool calls automatically.

3. **Tool discovery**: `tools/list` provides all tool schemas for AI assistants.

4. **Connection-first**: Most workflows start with connection tools to establish OAuth.

5. **Intelligence layer**: 13 analytics tools provide AI-powered insights beyond raw data.

6. **Configuration-driven**: Personalized zones and recommendations based on user profile.

7. **Nutrition integration**: USDA food database + meal analysis for holistic health.

8. **Recovery focus**: Sleep and recovery tools prevent overtraining.

9. **Tool chaining**: Complex workflows combine multiple tools sequentially.

10. **JSON Schema**: Every tool has input schema for validation and type safety.

---

**Next Chapter**: [Chapter 20: Sports Science Algorithms & Intelligence](./chapter-20-sports-science.md) - Learn how Pierre implements sports science algorithms for TSS, CTL/ATL/TSB, VO2 max estimation, FTP detection, and performance predictions.
