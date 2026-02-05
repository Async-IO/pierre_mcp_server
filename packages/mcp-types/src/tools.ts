// ABOUTME: Auto-generated TypeScript type definitions for Pierre MCP tool parameters
// ABOUTME: Generated from server tool schemas - DO NOT EDIT MANUALLY
//
// To regenerate: bun run generate (from packages/mcp-types)

/* eslint-disable @typescript-eslint/no-explicit-any */

// ============================================================================
// TOOL PARAMETER TYPES
// ============================================================================

// Note: connect_to_pierre removed - SDK bridge handles authentication locally via RFC 8414 discovery

/**
 * Connect to Fitness Provider - Unified authentication flow that connects you to both Pierre and a fitness provider (like Strava or Fitbit) in a single seamless process. This will open a browser window for secure authentication with both systems.
 */
export interface ConnectProviderParams {

  /** Fitness provider to connect to. Supported providers: 'strava', 'fitbit' */
  provider: string;
}


/**
 * Check which fitness providers are currently connected and authorized for the user. Returns connection status for all supported providers. Optionally accepts OAuth credentials to use custom apps instead of server defaults.
 */
export interface GetConnectionStatusParams {

  /** Optional: Your Fitbit OAuth client ID. If provided with client_secret, will be used instead of server defaults. */
  fitbit_client_id?: string;

  /** Optional: Your Fitbit OAuth client secret. Must be provided with client_id. */
  fitbit_client_secret?: string;

  /** Optional: Your Strava OAuth client ID. If provided with client_secret, will be used instead of server defaults. */
  strava_client_id?: string;

  /** Optional: Your Strava OAuth client secret. Must be provided with client_id. */
  strava_client_secret?: string;
}


/**
 * Disconnect and remove stored tokens for a specific fitness provider. This revokes access to the provider's data.
 */
export interface DisconnectProviderParams {

  /** Fitness provider to disconnect (e.g., 'strava', 'fitbit') */
  provider: string;
}


/**
 * Get fitness activities from a provider
 */
export interface GetActivitiesParams {

  /** Maximum number of activities to return */
  limit?: number;

  /** Number of activities to skip (for pagination) */
  offset?: number;

  /** Fitness provider name (e.g., 'strava', 'fitbit') */
  provider: string;
}


/**
 * Get athlete profile from a provider
 */
export interface GetAthleteParams {

  /** Fitness provider name (e.g., 'strava', 'fitbit') */
  provider: string;
}


/**
 * Get fitness statistics from a provider
 */
export interface GetStatsParams {

  /** Fitness provider name (e.g., 'strava', 'fitbit') */
  provider: string;
}


/**
 * Generate AI-powered insights and analysis for a specific activity
 */
export interface GetActivityIntelligenceParams {

  /** ID of the specific activity to analyze */
  activity_id: string;

  /** Whether to include location intelligence (default: true) */
  include_location?: boolean;

  /** Whether to include weather analysis (default: true) */
  include_weather?: boolean;

  /** Fitness provider name (e.g., 'strava', 'fitbit') */
  provider: string;
}


/**
 * Perform deep analysis of an individual activity including insights, metrics, and anomaly detection
 */
export interface AnalyzeActivityParams {

  /** ID of the activity to analyze */
  activity_id: string;

  /** Fitness provider name (e.g., 'strava', 'fitbit') */
  provider: string;
}


/**
 * Calculate advanced fitness metrics for an activity (TRIMP, power-to-weight ratio, efficiency scores, etc.)
 */
export interface CalculateMetricsParams {

  /** ID of the activity */
  activity_id: string;

  /** Specific metrics to calculate (e.g., ['trimp', 'power_to_weight', 'efficiency']) */
  metrics?: any[];

  /** Fitness provider name */
  provider: string;
}


/**
 * Analyze performance trends over time with statistical analysis and insights
 */
export interface AnalyzePerformanceTrendsParams {

  /** Metric to analyze trends for ('pace', 'heart_rate', 'power', 'distance', 'duration') */
  metric: string;

  /** Fitness provider name */
  provider: string;

  /** Filter by sport type (optional) */
  sport_type?: string;

  /** Time period for analysis ('week', 'month', 'quarter', 'sixmonths', 'year') */
  timeframe: string;
}


/**
 * Compare an activity against similar activities, personal bests, or historical averages
 */
export interface CompareActivitiesParams {

  /** Primary activity to compare */
  activity_id: string;

  /** Type of comparison ('similar_activities', 'personal_best', 'average', 'recent') */
  comparison_type: string;

  /** Fitness provider name */
  provider: string;
}


/**
 * Detect patterns in training data such as consistency trends, seasonal variations, or performance plateaus
 */
export interface DetectPatternsParams {

  /** Type of pattern to detect ('training_consistency', 'seasonal_trends', 'performance_plateaus', 'injury_risk') */
  pattern_type: string;

  /** Fitness provider name */
  provider: string;

  /** Time period for pattern analysis */
  timeframe?: string;
}


/**
 * Create and manage fitness goals with tracking and progress monitoring
 */
export interface SetGoalParams {

  /** Goal description */
  description?: string;

  /** Type of goal ('distance', 'time', 'frequency', 'performance', 'custom') */
  goal_type: string;

  /** Sport type for the goal */
  sport_type?: string;

  /** Target completion date (ISO format) */
  target_date: string;

  /** Target value to achieve */
  target_value: number;

  /** Goal title */
  title: string;
}


/**
 * Track progress toward a specific goal with milestone achievements and completion estimates
 */
export interface TrackProgressParams {

  /** ID of the goal to track */
  goal_id: string;
}


/**
 * Generate AI-powered goal suggestions based on user's activity history and fitness level
 */
export interface SuggestGoalsParams {

  /** Category of goals to suggest ('distance', 'performance', 'consistency', 'all') */
  goal_category?: string;

  /** Fitness provider name */
  provider: string;
}


/**
 * Assess whether a goal is realistic and achievable based on current performance and timeline
 */
export interface AnalyzeGoalFeasibilityParams {

  /** ID of the goal to analyze */
  goal_id: string;
}


/**
 * Generate personalized training recommendations based on activity data and user profile
 */
export interface GenerateRecommendationsParams {

  /** Specific activity to base recommendations on (optional) */
  activity_id?: string;

  /** Fitness provider name */
  provider: string;

  /** Type of recommendations ('training', 'recovery', 'nutrition', 'equipment', 'all') */
  recommendation_type?: string;
}


/**
 * Calculate comprehensive fitness score based on recent training load, consistency, and performance trends
 */
export interface CalculateFitnessScoreParams {

  /** Fitness provider name */
  provider: string;

  /** Time period for fitness assessment ('month', 'quarter', 'sixmonths') */
  timeframe?: string;
}


/**
 * Predict future performance capabilities based on current fitness trends and training history
 */
export interface PredictPerformanceParams {

  /** Fitness provider name */
  provider: string;

  /** Target date for prediction (ISO format) */
  target_date?: string;

  /** Target distance for performance prediction */
  target_distance: number;

  /** Target sport type for prediction */
  target_sport: string;
}


/**
 * Analyze training load balance, recovery needs, and load distribution over time
 */
export interface AnalyzeTrainingLoadParams {

  /** Fitness provider name */
  provider: string;

  /** Time period for load analysis ('week', 'month', 'quarter') */
  timeframe?: string;
}


/**
 * Get the complete configuration catalog with all available parameters and their metadata
 */
export interface GetConfigurationCatalogParams {}


/**
 * Get available configuration profiles (Research, Elite, Recreational, Beginner, Medical, etc.)
 */
export interface GetConfigurationProfilesParams {}


/**
 * Get current user's configuration including active profile and parameter overrides
 */
export interface GetUserConfigurationParams {}


/**
 * Update user's configuration by applying a profile and/or parameter overrides
 */
export interface UpdateUserConfigurationParams {

  /** Parameter overrides to apply (optional) */
  parameters?: Record<string, any>;

  /** Configuration profile to apply (optional) */
  profile?: string;
}


/**
 * Calculate personalized training zones (heart rate, pace, power) based on VO2 max and physiological parameters
 */
export interface CalculatePersonalizedZonesParams {

  /** Lactate threshold as percentage of VO2 max (optional, defaults to 0.85) */
  lactate_threshold?: number;

  /** Maximum heart rate in bpm (optional, defaults to 190) */
  max_hr?: number;

  /** Resting heart rate in bpm (optional, defaults to 60) */
  resting_hr?: number;

  /** Sport efficiency factor (optional, defaults to 1.0) */
  sport_efficiency?: number;

  /** VO2 max in ml/kg/min */
  vo2_max: number;
}


/**
 * Validate configuration parameters for physiological limits and scientific bounds
 */
export interface ValidateConfigurationParams {

  /** Configuration parameters to validate */
  parameters: Record<string, any>;
}


/**
 * Get fitness configuration settings including heart rate zones, power zones, and training parameters
 */
export interface GetFitnessConfigParams {

  /** Name of the fitness configuration to retrieve (defaults to 'default') */
  configuration_name?: string;
}


/**
 * Save fitness configuration settings for heart rate zones, power zones, and training parameters
 */
export interface SetFitnessConfigParams {

  /** Fitness configuration object containing zones, thresholds, and training parameters */
  configuration: Record<string, any>;

  /** Name of the fitness configuration to save (defaults to 'default') */
  configuration_name?: string;
}


/**
 * List all available fitness configuration names for the user
 */
export interface ListFitnessConfigsParams {}


/**
 * Delete a specific fitness configuration by name
 */
export interface DeleteFitnessConfigParams {

  /** Name of the fitness configuration to delete */
  configuration_name: string;
}


/**
 * Calculate daily calorie and macronutrient needs using Mifflin-St Jeor BMR formula. Returns BMR, TDEE, and macros (protein, carbs, fat) adjusted for training goal.
 */
export interface CalculateDailyNutritionParams {

  /** Activity level: 'sedentary', 'lightly_active', 'moderately_active', 'very_active', or 'extra_active' */
  activity_level: string;

  /** Age in years (max 150) */
  age: number;

  /** Gender: 'male' or 'female' */
  gender: string;

  /** Height in centimeters */
  height_cm: number;

  /** Training goal: 'maintenance', 'weight_loss', 'muscle_gain', or 'endurance_performance' */
  training_goal: string;

  /** Body weight in kilograms */
  weight_kg: number;
}


/**
 * Get optimal pre-workout and post-workout nutrition recommendations following ISSN (International Society of Sports Nutrition) guidelines. Returns timing windows, macros, and hydration targets.
 */
export interface GetNutrientTimingParams {

  /** Daily protein target in grams */
  daily_protein_g: number;

  /** Body weight in kilograms */
  weight_kg: number;

  /** Workout intensity: 'low', 'moderate', or 'high' */
  workout_intensity: string;
}


/**
 * Search USDA FoodData Central database for foods by name or description. Returns food ID, name, brand, and category for each match.
 */
export interface SearchFoodParams {

  /** Number of results to return (default: 10, max: 200) */
  page_size?: number;

  /** Food name or description to search for */
  query: string;
}


/**
 * Get detailed nutritional information for a specific food from USDA FoodData Central. Returns complete nutrient breakdown including calories, macros, vitamins, and minerals per 100g serving.
 */
export interface GetFoodDetailsParams {

  /** USDA FoodData Central ID for the food (from search_food results) */
  fdc_id: number;
}


/**
 * Analyze total calories and macronutrients for a meal composed of multiple foods. Each food requires USDA FoodData Central ID and portion size in grams. Returns aggregated nutrition totals.
 */
export interface AnalyzeMealNutritionParams {

  /** Array of food items with 'fdc_id' (number) and 'grams' (number) for each food */
  foods: any[];
}


/**
 * Analyze sleep quality using NSF/AASM guidelines. Returns overall score (0-100), stage breakdown (deep/REM/light), efficiency rating, and HRV trends if available. Provides recommendations for sleep optimization.
 */
export interface AnalyzeSleepQualityParams {

  /** Optional baseline HRV RMSSD value for comparison */
  baseline_hrv?: number;

  /** Optional array of recent HRV RMSSD values (numbers) for trend analysis */
  recent_hrv_values?: any[];

  /** Sleep data object with: date (string), duration_hours (number), efficiency_percent (number), deep_sleep_hours (number), rem_sleep_hours (number), light_sleep_hours (number), awakenings (number), hrv_rmssd_ms (number, optional) */
  sleep_data: Record<string, any>;
}


/**
 * Calculate comprehensive recovery score combining Training Stress Balance (TSB), sleep quality, and HRV metrics. Returns overall score (0-100), recovery category (optimal/adequate/compromised/poor), and training readiness recommendations.
 */
export interface CalculateRecoveryScoreParams {

  /** Fitness provider to fetch activities from (currently only 'strava' supported) */
  provider: string;

  /** Optional user configuration with: ftp (number), lthr (number), max_hr (number), resting_hr (number), weight_kg (number) */
  user_config?: Record<string, any>;
}


/**
 * AI-powered rest day recommendation based on training load analysis, recovery metrics, and fatigue indicators. Returns whether rest is recommended, urgency level, and reasoning based on TSB, recent intensity, and recovery status.
 */
export interface SuggestRestDayParams {

  /** Fitness provider to fetch activities from (currently only 'strava' supported) */
  provider: string;
}


/**
 * Track sleep patterns over time and identify trends. Requires at least 7 days of sleep data. Returns average metrics, trend direction (improving/stable/declining), consistency analysis, and recommendations for sleep optimization.
 */
export interface TrackSleepTrendsParams {

  /** Array of sleep data objects, each with: date (string), duration_hours (number), efficiency_percent (number, optional), deep_sleep_hours (number, optional), rem_sleep_hours (number, optional), light_sleep_hours (number, optional). Minimum 7 days required. */
  sleep_history: any[];
}


/**
 * Generate personalized sleep schedule recommendations based on training load, recovery needs, and upcoming workouts. Returns recommended sleep duration, optimal bedtime window, and sleep quality tips tailored to current training phase.
 */
export interface OptimizeSleepScheduleParams {

  /** Fitness provider to fetch activities from (currently only 'strava' supported) */
  provider: string;

  /** Intensity of upcoming workout: 'low', 'moderate', or 'high' (default: 'moderate') */
  upcoming_workout_intensity?: string;

  /** Optional user configuration with: ftp (number), lthr (number), max_hr (number), resting_hr (number), weight_kg (number) */
  user_config?: Record<string, any>;
}

// ============================================================================
// TOOL RESPONSE TYPES
// ============================================================================

/**
 * Generic MCP tool response wrapper
 */
export interface McpToolResponse {
  content?: Array<{
    type: string;
    text?: string;
    [key: string]: any;
  }>;
  isError?: boolean;
  [key: string]: any;
}

/**
 * MCP error response
 */
export interface McpErrorResponse {
  code: number;
  message: string;
  data?: any;
}


// ============================================================================
// TOOL NAME TYPES
// ============================================================================

/**
 * Union type of all available tool names
 */
export type ToolName = "connect_provider" | "get_connection_status" | "disconnect_provider" | "get_activities" | "get_athlete" | "get_stats" | "get_activity_intelligence" | "analyze_activity" | "calculate_metrics" | "analyze_performance_trends" | "compare_activities" | "detect_patterns" | "set_goal" | "track_progress" | "suggest_goals" | "analyze_goal_feasibility" | "generate_recommendations" | "calculate_fitness_score" | "predict_performance" | "analyze_training_load" | "get_configuration_catalog" | "get_configuration_profiles" | "get_user_configuration" | "update_user_configuration" | "calculate_personalized_zones" | "validate_configuration" | "get_fitness_config" | "set_fitness_config" | "list_fitness_configs" | "delete_fitness_config" | "calculate_daily_nutrition" | "get_nutrient_timing" | "search_food" | "get_food_details" | "analyze_meal_nutrition" | "analyze_sleep_quality" | "calculate_recovery_score" | "suggest_rest_day" | "track_sleep_trends" | "optimize_sleep_schedule";

/**
 * Map of tool names to their parameter types
 */
export interface ToolParamsMap {
  "connect_provider": ConnectProviderParams;
  "get_connection_status": GetConnectionStatusParams;
  "disconnect_provider": DisconnectProviderParams;
  "get_activities": GetActivitiesParams;
  "get_athlete": GetAthleteParams;
  "get_stats": GetStatsParams;
  "get_activity_intelligence": GetActivityIntelligenceParams;
  "analyze_activity": AnalyzeActivityParams;
  "calculate_metrics": CalculateMetricsParams;
  "analyze_performance_trends": AnalyzePerformanceTrendsParams;
  "compare_activities": CompareActivitiesParams;
  "detect_patterns": DetectPatternsParams;
  "set_goal": SetGoalParams;
  "track_progress": TrackProgressParams;
  "suggest_goals": SuggestGoalsParams;
  "analyze_goal_feasibility": AnalyzeGoalFeasibilityParams;
  "generate_recommendations": GenerateRecommendationsParams;
  "calculate_fitness_score": CalculateFitnessScoreParams;
  "predict_performance": PredictPerformanceParams;
  "analyze_training_load": AnalyzeTrainingLoadParams;
  "get_configuration_catalog": GetConfigurationCatalogParams;
  "get_configuration_profiles": GetConfigurationProfilesParams;
  "get_user_configuration": GetUserConfigurationParams;
  "update_user_configuration": UpdateUserConfigurationParams;
  "calculate_personalized_zones": CalculatePersonalizedZonesParams;
  "validate_configuration": ValidateConfigurationParams;
  "get_fitness_config": GetFitnessConfigParams;
  "set_fitness_config": SetFitnessConfigParams;
  "list_fitness_configs": ListFitnessConfigsParams;
  "delete_fitness_config": DeleteFitnessConfigParams;
  "calculate_daily_nutrition": CalculateDailyNutritionParams;
  "get_nutrient_timing": GetNutrientTimingParams;
  "search_food": SearchFoodParams;
  "get_food_details": GetFoodDetailsParams;
  "analyze_meal_nutrition": AnalyzeMealNutritionParams;
  "analyze_sleep_quality": AnalyzeSleepQualityParams;
  "calculate_recovery_score": CalculateRecoveryScoreParams;
  "suggest_rest_day": SuggestRestDayParams;
  "track_sleep_trends": TrackSleepTrendsParams;
  "optimize_sleep_schedule": OptimizeSleepScheduleParams;
}
