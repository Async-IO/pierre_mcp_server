// ABOUTME: Auto-generated TypeScript type definitions for Pierre MCP tool parameters
// ABOUTME: Generated from server tool schemas - DO NOT EDIT MANUALLY
//
// Tool count: 12
// To regenerate: bun run generate (from packages/mcp-types)

/* eslint-disable @typescript-eslint/no-explicit-any */

// ============================================================================
// TOOL PARAMETER TYPES
// ============================================================================

// Note: connect_to_pierre removed - SDK bridge handles authentication locally via RFC 8414 discovery


/**
 * Analyze nutritional content of a meal from its ingredients
 */
export interface AnalyzeMealNutritionParams {

  /** Array of ingredients with fdc_id and amount_g fields */
  ingredients: any[];
}


/**
 * Calculate daily calorie and macronutrient needs based on biometrics and goals
 */
export interface CalculateDailyNutritionParams {

  /** Activity level: sedentary, lightly_active, moderately_active, very_active, extra_active */
  activity_level: string;

  /** Age in years */
  age: number;

  /** Gender: male or female */
  gender: string;

  /** Height in centimeters */
  height_cm: number;

  /** Training goal: maintenance, weight_loss, muscle_gain, endurance_performance */
  training_goal: string;

  /** Body weight in kilograms */
  weight_kg: number;
}


/**
 * Detect training patterns including hard/easy day balance, weekly schedule consistency, volume progression, and overtraining warning signs
 */
export interface DetectPatternsParams {

  /** Fitness provider to query. Defaults to configured provider. */
  provider?: string;

  /** Number of weeks to analyze for patterns. Default: 4. */
  weeks?: number;
}


/**
 * Retrieve user's fitness activities from connected providers with optional filtering by sport type, date range, and pagination support
 */
export interface GetActivitiesParams {

  /** Unix timestamp - return activities after this time. */
  after?: number;

  /** Unix timestamp - return activities before this time. */
  before?: number;

  /** Output format: 'json' (default) or 'toon' (token-efficient for LLMs). */
  format?: string;

  /** Maximum number of activities to return. Defaults to format-aware limit to prevent context overflow. */
  limit?: number;

  /** Output mode: 'summary' (default, minimal fields) or 'detailed' (full activity data). */
  mode?: string;

  /** Number of activities to skip for pagination. */
  offset?: number;

  /** Fitness provider to query (e.g., 'strava', 'fitbit'). Defaults to configured default provider. */
  provider?: string;

  /** Filter by sport type (e.g., 'run', 'ride', 'swim'). Case-insensitive. */
  sport_type?: string;
}


/**
 * Retrieve the user's athlete profile from connected fitness providers including personal details and preferences
 */
export interface GetAthleteParams {

  /** Output format: 'json' (default) or 'toon' (token-efficient for LLMs). */
  format?: string;

  /** Fitness provider to query (e.g., 'strava', 'fitbit'). Defaults to configured default provider. */
  provider?: string;
}


/**
 * Get the complete catalog of available configuration options
 */
export interface GetConfigurationCatalogParams {}


/**
 * Get available configuration profile templates
 */
export interface GetConfigurationProfilesParams {}


/**
 * Get detailed nutritional information for a specific food item
 */
export interface GetFoodDetailsParams {

  /** USDA FoodData Central ID of the food item */
  fdc_id: number;
}


/**
 * Retrieve aggregated activity statistics from connected fitness providers including totals, records, and year-to-date metrics
 */
export interface GetStatsParams {

  /** Output format: 'json' (default) or 'toon' (token-efficient for LLMs). */
  format?: string;

  /** Fitness provider to query (e.g., 'strava', 'fitbit'). Defaults to configured default provider. */
  provider?: string;
}


/**
 * Search USDA FoodData Central database for foods. Returns up to 10 results by default. Check the `has_more` field before requesting additional pages.
 */
export interface SearchFoodParams {

  /** Page number (1-indexed, default: 1). Only use if previous response had has_more=true */
  page_number?: number;

  /** Number of results per page (default: 10, max: 50) */
  page_size?: number;

  /** Search query for food items */
  query: string;
}


/**
 * Get AI-suggested fitness goals based on your activity history and fitness level
 */
export interface SuggestGoalsParams {

  /** Fitness provider to analyze. Defaults to configured provider. */
  provider?: string;
}


/**
 * Validate configuration parameters for physiological correctness
 */
export interface ValidateConfigurationParams {

  /** Configuration parameters to validate */
  parameters: Record<string, any>;
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
export type ToolName = "analyze_meal_nutrition" | "calculate_daily_nutrition" | "detect_patterns" | "get_activities" | "get_athlete" | "get_configuration_catalog" | "get_configuration_profiles" | "get_food_details" | "get_stats" | "search_food" | "suggest_goals" | "validate_configuration";

/**
 * Map of tool names to their parameter types
 */
export interface ToolParamsMap {
  "analyze_meal_nutrition": AnalyzeMealNutritionParams;
  "calculate_daily_nutrition": CalculateDailyNutritionParams;
  "detect_patterns": DetectPatternsParams;
  "get_activities": GetActivitiesParams;
  "get_athlete": GetAthleteParams;
  "get_configuration_catalog": GetConfigurationCatalogParams;
  "get_configuration_profiles": GetConfigurationProfilesParams;
  "get_food_details": GetFoodDetailsParams;
  "get_stats": GetStatsParams;
  "search_food": SearchFoodParams;
  "suggest_goals": SuggestGoalsParams;
  "validate_configuration": ValidateConfigurationParams;
}
