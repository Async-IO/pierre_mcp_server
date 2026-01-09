// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Zod runtime validation schemas for all MCP tool responses
// ABOUTME: Provides type-safe response validation with detailed error messages

import { z } from "zod";

// ============================================================================
// BASE SCHEMAS - Reusable building blocks
// ============================================================================

/**
 * UUID string format validator
 */
export const UuidSchema = z.string().uuid();

/**
 * RFC3339 timestamp string
 */
export const TimestampSchema = z.string().datetime({ offset: true }).or(z.string().regex(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}/));

/**
 * Quality/Status rating enum used across multiple tools
 */
export const QualityRatingSchema = z.enum(["excellent", "good", "moderate", "fair", "poor"]);

/**
 * Connection status enum
 */
export const ConnectionStatusSchema = z.enum(["connected", "disconnected"]);

/**
 * Goal status enum
 */
export const GoalStatusSchema = z.enum(["created", "in_progress", "completed", "abandoned"]);

/**
 * Confidence level (0-1 float)
 */
export const ConfidenceLevelSchema = z.number().min(0).max(1);

/**
 * Score (0-100 integer or float)
 */
export const ScoreSchema = z.number().min(0).max(100);

/**
 * Insights array - array of observation strings
 */
export const InsightsArraySchema = z.array(z.string());

/**
 * Recommendations array - array of suggestion strings
 */
export const RecommendationsArraySchema = z.array(z.string());

// ============================================================================
// COMMON RESPONSE PATTERNS
// ============================================================================

/**
 * Standard metadata included in most responses
 */
export const ResponseMetadataSchema = z.object({
  user_id: z.string().optional(),
  tenant_id: z.string().nullable().optional(),
  cached: z.boolean().optional(),
  timestamp: TimestampSchema.optional(),
  analysis_type: z.string().optional(),
  analysis_source: z.enum(["mcp_sampling", "static", "database", "calculated"]).optional(),
  format: z.enum(["json", "toon"]).optional(),
  returned_count: z.number().int().optional(),
}).passthrough();

/**
 * Pagination info for list responses
 */
export const PaginationInfoSchema = z.object({
  offset: z.number().int().min(0),
  limit: z.number().int().min(1),
  returned_count: z.number().int().min(0),
  has_more: z.boolean(),
  total_count: z.number().int().min(0).optional(),
});

/**
 * Score-based response pattern (fitness score, recovery score, etc.)
 */
export const ScoreBasedResponseSchema = z.object({
  score: ScoreSchema,
  status: QualityRatingSchema.optional(),
  confidence_level: ConfidenceLevelSchema.optional(),
  insights: InsightsArraySchema.optional(),
  recommendations: RecommendationsArraySchema.optional(),
}).passthrough();

/**
 * Validation response pattern
 */
export const ValidationResponseSchema = z.object({
  valid: z.boolean(),
  errors: z.array(z.string()).optional(),
  warnings: z.array(z.string()).optional(),
  suggestions: z.array(z.string()).optional(),
}).passthrough();

// ============================================================================
// MCP TOOL RESPONSE WRAPPER
// ============================================================================

/**
 * MCP content item (text or other types)
 */
export const McpContentItemSchema = z.object({
  type: z.string(),
  text: z.string().optional(),
}).passthrough();

/**
 * Base MCP tool response wrapper
 */
export const McpToolResponseBaseSchema = z.object({
  content: z.array(McpContentItemSchema).optional(),
  isError: z.boolean().optional(),
}).passthrough();

// ============================================================================
// TOOL-SPECIFIC RESPONSE SCHEMAS
// ============================================================================

// --------------------------------------------------------------------------
// Connection Management Tools
// --------------------------------------------------------------------------

export const ConnectProviderResponseSchema = z.object({
  provider: z.string(),
  status: z.string(),
  message: z.string(),
  authorization_url: z.string().url().optional(),
  requires_browser: z.boolean().optional(),
}).passthrough();

export const GetConnectionStatusResponseSchema = z.union([
  // Single provider response
  z.object({
    provider: z.string(),
    status: ConnectionStatusSchema,
    connected: z.boolean(),
    expires_at: TimestampSchema.optional(),
    scopes: z.array(z.string()).optional(),
  }).passthrough(),
  // Multi-provider response
  z.object({
    providers: z.record(z.string(), z.object({
      connected: z.boolean(),
      status: ConnectionStatusSchema,
      expires_at: TimestampSchema.optional(),
    })),
  }).passthrough(),
]);

export const DisconnectProviderResponseSchema = z.object({
  provider: z.string(),
  message: z.string(),
  disconnected_at: TimestampSchema.optional(),
  success: z.boolean().optional(),
}).passthrough();

// --------------------------------------------------------------------------
// Fitness Data Tools
// --------------------------------------------------------------------------

/**
 * Activity map structure
 */
export const ActivityMapSchema = z.object({
  id: z.string().optional(),
  summary_polyline: z.string().optional(),
  polyline: z.string().optional(),
}).passthrough();

/**
 * Activity structure
 */
export const ActivitySchema = z.object({
  id: z.union([z.string(), z.number()]),
  name: z.string(),
  type: z.string(),
  distance: z.number().optional(),
  duration: z.number().optional(),
  moving_time: z.number().optional(),
  elapsed_time: z.number().optional(),
  total_elevation_gain: z.number().optional(),
  start_date: z.string().optional(),
  start_date_local: z.string().optional(),
  timezone: z.string().optional(),
  average_speed: z.number().optional(),
  max_speed: z.number().optional(),
  average_cadence: z.number().optional(),
  average_heartrate: z.number().optional(),
  max_heartrate: z.number().optional(),
  average_watts: z.number().optional(),
  kilojoules: z.number().optional(),
  device_watts: z.boolean().optional(),
  has_heartrate: z.boolean().optional(),
  calories: z.number().optional(),
  map: ActivityMapSchema.optional(),
}).passthrough();

export const GetActivitiesResponseSchema = z.object({
  activities: z.array(ActivitySchema).optional(),
  activities_toon: z.string().optional(),
  provider: z.string(),
  count: z.number().int().optional(),
  mode: z.enum(["summary", "detailed"]).optional(),
  format: z.enum(["json", "toon"]).optional(),
  format_fallback: z.boolean().optional(),
  offset: z.number().int().optional(),
  limit: z.number().int().optional(),
  has_more: z.boolean().optional(),
  returned_count: z.number().int().optional(),
}).passthrough();

/**
 * Athlete structure
 */
export const AthleteSchema = z.object({
  id: z.union([z.string(), z.number()]),
  username: z.string().optional(),
  resource_state: z.number().optional(),
  firstname: z.string().optional(),
  lastname: z.string().optional(),
  bio: z.string().optional(),
  city: z.string().optional(),
  state: z.string().optional(),
  country: z.string().optional(),
  sex: z.string().optional(),
  premium: z.boolean().optional(),
  summit: z.boolean().optional(),
  created_at: z.string().optional(),
  updated_at: z.string().optional(),
  weight: z.number().optional(),
  ftp: z.number().optional(),
}).passthrough();

export const GetAthleteResponseSchema = z.object({
  athlete: AthleteSchema.optional(),
  athlete_toon: z.string().optional(),
  format: z.enum(["json", "toon"]).optional(),
  format_fallback: z.boolean().optional(),
}).passthrough();

/**
 * Activity totals for statistics
 */
export const ActivityTotalsSchema = z.object({
  count: z.number().optional(),
  distance: z.number().optional(),
  moving_time: z.number().optional(),
  elapsed_time: z.number().optional(),
  elevation_gain: z.number().optional(),
  achievement_count: z.number().optional(),
}).passthrough();

export const GetStatsResponseSchema = z.object({
  stats: z.object({
    biggest_ride_distance: z.number().optional(),
    biggest_climb_elevation_gain: z.number().optional(),
    recent_ride_totals: ActivityTotalsSchema.optional(),
    recent_run_totals: ActivityTotalsSchema.optional(),
    recent_swim_totals: ActivityTotalsSchema.optional(),
    ytd_ride_totals: ActivityTotalsSchema.optional(),
    ytd_run_totals: ActivityTotalsSchema.optional(),
    ytd_swim_totals: ActivityTotalsSchema.optional(),
    all_ride_totals: ActivityTotalsSchema.optional(),
    all_run_totals: ActivityTotalsSchema.optional(),
    all_swim_totals: ActivityTotalsSchema.optional(),
  }).passthrough().optional(),
  stats_toon: z.string().optional(),
  format: z.enum(["json", "toon"]).optional(),
  format_fallback: z.boolean().optional(),
}).passthrough();

// --------------------------------------------------------------------------
// Intelligence Analysis Tools
// --------------------------------------------------------------------------

export const GetActivityIntelligenceResponseSchema = z.object({
  activity_id: z.string(),
  activity_type: z.string().optional(),
  timestamp: TimestampSchema.optional(),
  intelligence: z.object({
    summary: z.string().optional(),
    insights: InsightsArraySchema.optional(),
    recommendations: RecommendationsArraySchema.optional(),
    performance_metrics: z.record(z.string(), z.unknown()).optional(),
  }).passthrough().optional(),
  analysis_source: z.enum(["mcp_sampling", "static"]).optional(),
}).passthrough();

export const AnalyzeActivityResponseSchema = GetActivityIntelligenceResponseSchema;

export const CalculateMetricsResponseSchema = z.object({
  power_output: z.number().optional(),
  efficiency_score: z.number().optional(),
  vo2_max: z.number().optional(),
  anaerobic_threshold: z.number().optional(),
  peak_heart_rate: z.number().int().optional(),
  average_heart_rate: z.number().optional(),
  relative_intensity: z.number().optional(),
  training_effect: z.object({
    aerobic: z.number().optional(),
    anaerobic: z.number().optional(),
  }).passthrough().optional(),
  trimp: z.number().optional(),
  tss: z.number().optional(),
}).passthrough();

export const AnalyzePerformanceTrendsResponseSchema = z.object({
  trend_direction: z.enum(["improving", "stable", "declining"]).optional(),
  trend_slope: z.number().optional(),
  r_squared: z.number().optional(),
  performance_change: z.number().optional(),
  insights: InsightsArraySchema.optional(),
  recommendations: RecommendationsArraySchema.optional(),
  data_points: z.number().int().optional(),
  timeframe: z.string().optional(),
  metric: z.string().optional(),
}).passthrough();

export const CompareActivitiesResponseSchema = z.object({
  comparison: z.record(z.string(), z.unknown()).optional(),
  performance_delta: z.number().optional(),
  insights: InsightsArraySchema.optional(),
  recommendations: RecommendationsArraySchema.optional(),
  primary_activity: z.record(z.string(), z.unknown()).optional(),
  comparison_activity: z.record(z.string(), z.unknown()).optional(),
}).passthrough();

export const DetectPatternsResponseSchema = z.object({
  patterns_detected: z.array(z.string()).optional(),
  pattern_details: z.record(z.string(), z.unknown()).optional(),
  confidence_scores: z.record(z.string(), z.number()).optional(),
  recommended_actions: RecommendationsArraySchema.optional(),
  pattern_type: z.string().optional(),
}).passthrough();

export const GenerateRecommendationsResponseSchema = z.object({
  recommendations: z.array(z.object({
    type: z.string().optional(),
    title: z.string().optional(),
    description: z.string(),
    priority: z.enum(["high", "medium", "low"]).optional(),
    reasoning: z.string().optional(),
  }).passthrough()).optional(),
  reasoning: z.record(z.string(), z.unknown()).optional(),
  confidence_scores: z.record(z.string(), z.number()).optional(),
  personalized_context: z.record(z.string(), z.unknown()).optional(),
}).passthrough();

export const CalculateFitnessScoreResponseSchema = z.object({
  fitness_score: ScoreSchema,
  score_breakdowns: z.record(z.string(), z.number()).optional(),
  recovery_adjustment_info: z.record(z.string(), z.unknown()).optional(),
  unadjusted_score: z.number().optional(),
  status: QualityRatingSchema.optional(),
  insights: InsightsArraySchema.optional(),
  recommendations: RecommendationsArraySchema.optional(),
}).passthrough();

export const PredictPerformanceResponseSchema = z.object({
  predicted_time: z.number().optional(),
  predicted_pace: z.string().optional(),
  confidence_level: ConfidenceLevelSchema.optional(),
  reasoning: z.array(z.string()).optional(),
  assumptions: z.array(z.string()).optional(),
  target_sport: z.string().optional(),
  target_distance: z.number().optional(),
}).passthrough();

export const AnalyzeTrainingLoadResponseSchema = z.object({
  ctl: z.number().optional(), // Chronic Training Load
  atl: z.number().optional(), // Acute Training Load
  tsb: z.number().optional(), // Training Stress Balance
  status: z.string().optional(),
  insights: InsightsArraySchema.optional(),
  recommendations: RecommendationsArraySchema.optional(),
  recovery_context: z.record(z.string(), z.unknown()).optional(),
  load_trend: z.enum(["increasing", "stable", "decreasing"]).optional(),
}).passthrough();

// --------------------------------------------------------------------------
// Goals Tools
// --------------------------------------------------------------------------

export const SetGoalResponseSchema = z.object({
  goal_id: z.string(),
  goal_type: z.string(),
  target_value: z.number(),
  timeframe: z.union([z.number(), z.string()]).optional(),
  title: z.string().optional(),
  created_at: TimestampSchema.optional(),
  status: z.string().optional(),
  message: z.string().optional(),
}).passthrough();

export const TrackProgressResponseSchema = z.object({
  goal_id: z.string(),
  current_progress: z.number().optional(),
  progress_percentage: z.number().min(0).max(100).optional(),
  days_remaining: z.number().int().optional(),
  status: z.string().optional(),
  insights: InsightsArraySchema.optional(),
  recommendations: RecommendationsArraySchema.optional(),
  pace_assessment: z.string().optional(),
  on_track: z.boolean().optional(),
}).passthrough();

export const SuggestGoalsResponseSchema = z.object({
  suggestions: z.array(z.object({
    goal_type: z.string(),
    target_value: z.number(),
    timeframe: z.union([z.number(), z.string()]).optional(),
    confidence_level: ConfidenceLevelSchema.optional(),
    reasoning: z.string().optional(),
  }).passthrough()).optional(),
  analysis: z.record(z.string(), z.unknown()).optional(),
}).passthrough();

export const AnalyzeGoalFeasibilityResponseSchema = z.object({
  feasible: z.boolean(),
  feasibility_score: ScoreSchema.optional(),
  confidence_level: ConfidenceLevelSchema.optional(),
  risk_factors: z.array(z.string()).optional(),
  recommendations: RecommendationsArraySchema.optional(),
  adjusted_target: z.number().optional(),
  adjusted_timeframe: z.number().optional(),
  analysis: z.object({
    current_level: z.number().optional(),
    target_value: z.number().optional(),
    improvement_required_percent: z.number().optional(),
    safe_improvement_capacity_percent: z.number().optional(),
    timeframe_months: z.number().optional(),
  }).passthrough().optional(),
  historical_context: z.record(z.string(), z.unknown()).optional(),
}).passthrough();

// --------------------------------------------------------------------------
// Configuration Tools
// --------------------------------------------------------------------------

export const GetConfigurationCatalogResponseSchema = z.object({
  catalog: z.record(z.string(), z.unknown()),
  total_parameters: z.number().int().optional(),
  categories: z.array(z.string()).optional(),
}).passthrough();

export const GetConfigurationProfilesResponseSchema = z.object({
  profiles: z.array(z.object({
    name: z.string(),
    profile: z.record(z.string(), z.unknown()).optional(),
    description: z.string().optional(),
  }).passthrough()),
  total_count: z.number().int().optional(),
}).passthrough();

export const GetUserConfigurationResponseSchema = z.object({
  user_id: z.string().optional(),
  active_profile: z.string().optional(),
  configuration: z.object({
    profile: z.record(z.string(), z.unknown()).optional(),
    session_overrides: z.record(z.string(), z.unknown()).optional(),
    last_modified: TimestampSchema.optional(),
  }).passthrough().optional(),
  available_parameters: z.number().int().optional(),
}).passthrough();

export const UpdateUserConfigurationResponseSchema = z.object({
  user_id: z.string().optional(),
  configuration: z.record(z.string(), z.unknown()).optional(),
  message: z.string(),
  updated_at: TimestampSchema.optional(),
  updated: z.boolean().optional(),
}).passthrough();

export const CalculatePersonalizedZonesResponseSchema = z.object({
  zones: z.object({
    heart_rate: z.array(z.object({
      zone: z.number().int(),
      name: z.string(),
      min: z.number(),
      max: z.number(),
      description: z.string().optional(),
    })).optional(),
    power: z.array(z.object({
      zone: z.number().int(),
      name: z.string(),
      min: z.number(),
      max: z.number(),
      description: z.string().optional(),
    })).optional(),
    pace: z.array(z.object({
      zone: z.number().int(),
      name: z.string(),
      min: z.string(),
      max: z.string(),
      description: z.string().optional(),
    })).optional(),
  }).passthrough().optional(),
  vo2_max: z.number().optional(),
  lactate_threshold: z.number().optional(),
}).passthrough();

export const ValidateConfigurationResponseSchema = ValidationResponseSchema;

export const GetFitnessConfigResponseSchema = z.object({
  configuration_name: z.string().optional(),
  configuration: z.record(z.string(), z.unknown()).optional(),
  exists: z.boolean().optional(),
}).passthrough();

export const SetFitnessConfigResponseSchema = z.object({
  configuration_name: z.string(),
  message: z.string(),
  saved_at: TimestampSchema.optional(),
  success: z.boolean().optional(),
}).passthrough();

export const ListFitnessConfigsResponseSchema = z.object({
  configurations: z.array(z.string()),
  count: z.number().int().optional(),
}).passthrough();

export const DeleteFitnessConfigResponseSchema = z.object({
  configuration_name: z.string(),
  message: z.string(),
  deleted_at: TimestampSchema.optional(),
  success: z.boolean().optional(),
}).passthrough();

// --------------------------------------------------------------------------
// Nutrition Tools
// --------------------------------------------------------------------------

export const CalculateDailyNutritionResponseSchema = z.object({
  calories: z.number(),
  protein_g: z.number(),
  carbs_g: z.number(),
  fat_g: z.number(),
  bmr: z.number().optional(),
  tdee: z.number().optional(),
  micronutrients: z.record(z.string(), z.number()).optional(),
  meal_timing_suggestions: z.array(z.string()).optional(),
}).passthrough();

export const GetNutrientTimingResponseSchema = z.object({
  pre_workout: z.object({
    timing_minutes: z.number().optional(),
    nutrients: z.record(z.string(), z.unknown()).optional(),
    recommendations: z.array(z.string()).optional(),
  }).passthrough().optional(),
  during_workout: z.object({
    nutrients: z.record(z.string(), z.unknown()).optional(),
    recommendations: z.array(z.string()).optional(),
  }).passthrough().optional(),
  post_workout: z.object({
    timing_minutes: z.number().optional(),
    nutrients: z.record(z.string(), z.unknown()).optional(),
    recommendations: z.array(z.string()).optional(),
  }).passthrough().optional(),
  recommendations: RecommendationsArraySchema.optional(),
}).passthrough();

export const SearchFoodResponseSchema = z.object({
  results: z.array(z.object({
    fdc_id: z.number().int(),
    description: z.string(),
    calories: z.number().optional(),
    brand: z.string().optional(),
    category: z.string().optional(),
  }).passthrough()),
  total_results: z.number().int().optional(),
  search_query: z.string().optional(),
}).passthrough();

export const GetFoodDetailsResponseSchema = z.object({
  fdc_id: z.number().int(),
  description: z.string(),
  calories: z.number().optional(),
  protein: z.number().optional(),
  carbs: z.number().optional(),
  fat: z.number().optional(),
  fiber: z.number().optional(),
  micronutrients: z.record(z.string(), z.number()).optional(),
  serving_info: z.object({
    serving_size: z.number().optional(),
    serving_unit: z.string().optional(),
  }).passthrough().optional(),
}).passthrough();

export const AnalyzeMealNutritionResponseSchema = z.object({
  meal_name: z.string().optional(),
  total_calories: z.number(),
  macronutrient_breakdown: z.object({
    protein_g: z.number(),
    carbs_g: z.number(),
    fat_g: z.number(),
    fiber_g: z.number().optional(),
  }).passthrough(),
  micronutrients: z.record(z.string(), z.number()).optional(),
  analysis: InsightsArraySchema.optional(),
  timing_suitability: z.record(z.string(), z.unknown()).optional(),
}).passthrough();

// --------------------------------------------------------------------------
// Sleep & Recovery Tools
// --------------------------------------------------------------------------

export const AnalyzeSleepQualityResponseSchema = z.object({
  sleep_quality_score: ScoreSchema,
  quality_rating: QualityRatingSchema.optional(),
  sleep_stages: z.object({
    deep_sleep_hours: z.number().optional(),
    rem_sleep_hours: z.number().optional(),
    light_sleep_hours: z.number().optional(),
    awake_hours: z.number().optional(),
  }).passthrough().optional(),
  sleep_efficiency: z.number().min(0).max(100).optional(),
  insights: InsightsArraySchema.optional(),
  recommendations: RecommendationsArraySchema.optional(),
}).passthrough();

export const CalculateRecoveryScoreResponseSchema = z.object({
  recovery_score: ScoreSchema,
  recovery_status: QualityRatingSchema.optional(),
  hrv_score: z.number().optional(),
  sleep_contribution: z.number().optional(),
  training_readiness: z.string().optional(),
  recommendations: RecommendationsArraySchema.optional(),
  components: z.object({
    sleep: z.number().optional(),
    hrv: z.number().optional(),
    training_load: z.number().optional(),
  }).passthrough().optional(),
}).passthrough();

export const SuggestRestDayResponseSchema = z.object({
  suggested_rest_day: z.boolean(),
  reasoning: z.array(z.string()).optional(),
  fatigue_level: z.string().optional(),
  recovery_needs: z.record(z.string(), z.unknown()).optional(),
  recommendations: RecommendationsArraySchema.optional(),
  urgency: z.enum(["high", "medium", "low"]).optional(),
}).passthrough();

export const TrackSleepTrendsResponseSchema = z.object({
  trend_direction: z.enum(["improving", "stable", "declining"]).optional(),
  average_sleep_duration: z.number().optional(),
  sleep_quality_trend: z.number().optional(),
  insights: InsightsArraySchema.optional(),
  recommendations: RecommendationsArraySchema.optional(),
  data_points: z.number().int().optional(),
}).passthrough();

export const OptimizeSleepScheduleResponseSchema = z.object({
  optimal_bedtime: z.string().optional(),
  optimal_wake_time: z.string().optional(),
  recommended_duration: z.number().optional(),
  sleep_cycles: z.number().int().optional(),
  factors_considered: z.array(z.string()).optional(),
  recommendations: RecommendationsArraySchema.optional(),
}).passthrough();

// ============================================================================
// TOOL RESPONSE SCHEMA MAP
// ============================================================================

/**
 * Map of tool names to their Zod response schemas.
 * This enables type-safe runtime validation of all tool responses.
 */
export const ToolResponseSchemaMap = {
  // Connection Management
  connect_provider: ConnectProviderResponseSchema,
  get_connection_status: GetConnectionStatusResponseSchema,
  disconnect_provider: DisconnectProviderResponseSchema,

  // Fitness Data
  get_activities: GetActivitiesResponseSchema,
  get_athlete: GetAthleteResponseSchema,
  get_stats: GetStatsResponseSchema,

  // Intelligence Analysis
  get_activity_intelligence: GetActivityIntelligenceResponseSchema,
  analyze_activity: AnalyzeActivityResponseSchema,
  calculate_metrics: CalculateMetricsResponseSchema,
  analyze_performance_trends: AnalyzePerformanceTrendsResponseSchema,
  compare_activities: CompareActivitiesResponseSchema,
  detect_patterns: DetectPatternsResponseSchema,
  generate_recommendations: GenerateRecommendationsResponseSchema,
  calculate_fitness_score: CalculateFitnessScoreResponseSchema,
  predict_performance: PredictPerformanceResponseSchema,
  analyze_training_load: AnalyzeTrainingLoadResponseSchema,

  // Goals
  set_goal: SetGoalResponseSchema,
  track_progress: TrackProgressResponseSchema,
  suggest_goals: SuggestGoalsResponseSchema,
  analyze_goal_feasibility: AnalyzeGoalFeasibilityResponseSchema,

  // Configuration
  get_configuration_catalog: GetConfigurationCatalogResponseSchema,
  get_configuration_profiles: GetConfigurationProfilesResponseSchema,
  get_user_configuration: GetUserConfigurationResponseSchema,
  update_user_configuration: UpdateUserConfigurationResponseSchema,
  calculate_personalized_zones: CalculatePersonalizedZonesResponseSchema,
  validate_configuration: ValidateConfigurationResponseSchema,
  get_fitness_config: GetFitnessConfigResponseSchema,
  set_fitness_config: SetFitnessConfigResponseSchema,
  list_fitness_configs: ListFitnessConfigsResponseSchema,
  delete_fitness_config: DeleteFitnessConfigResponseSchema,

  // Nutrition
  calculate_daily_nutrition: CalculateDailyNutritionResponseSchema,
  get_nutrient_timing: GetNutrientTimingResponseSchema,
  search_food: SearchFoodResponseSchema,
  get_food_details: GetFoodDetailsResponseSchema,
  analyze_meal_nutrition: AnalyzeMealNutritionResponseSchema,

  // Sleep & Recovery
  analyze_sleep_quality: AnalyzeSleepQualityResponseSchema,
  calculate_recovery_score: CalculateRecoveryScoreResponseSchema,
  suggest_rest_day: SuggestRestDayResponseSchema,
  track_sleep_trends: TrackSleepTrendsResponseSchema,
  optimize_sleep_schedule: OptimizeSleepScheduleResponseSchema,
} as const;

export type ToolName = keyof typeof ToolResponseSchemaMap;

// ============================================================================
// VALIDATION UTILITIES
// ============================================================================

/**
 * Result of validating a tool response
 */
export interface ValidationResult<T> {
  success: boolean;
  data?: T;
  error?: {
    tool: string;
    issues: z.ZodIssue[];
    rawData: unknown;
  };
}

/**
 * Validate a tool response against its schema.
 *
 * @param toolName - The name of the tool
 * @param response - The raw response data to validate
 * @returns ValidationResult with parsed data or error details
 */
export function validateToolResponse<T extends ToolName>(
  toolName: T,
  response: unknown
): ValidationResult<z.infer<typeof ToolResponseSchemaMap[T]>> {
  const schema = ToolResponseSchemaMap[toolName];

  if (!schema) {
    return {
      success: false,
      error: {
        tool: toolName,
        issues: [{
          code: "custom",
          path: [],
          message: `No schema defined for tool: ${toolName}`,
        }],
        rawData: response,
      },
    };
  }

  const result = schema.safeParse(response);

  if (result.success) {
    return {
      success: true,
      data: result.data,
    };
  }

  return {
    success: false,
    error: {
      tool: toolName,
      issues: result.error.issues,
      rawData: response,
    },
  };
}

/**
 * Validate and throw on error (for strict mode)
 */
export function validateToolResponseStrict<T extends ToolName>(
  toolName: T,
  response: unknown
): z.infer<typeof ToolResponseSchemaMap[T]> {
  const result = validateToolResponse(toolName, response);

  if (!result.success) {
    const issueMessages = result.error!.issues
      .map(i => `  - ${i.path.join('.')}: ${i.message}`)
      .join('\n');
    throw new Error(
      `Response validation failed for tool "${toolName}":\n${issueMessages}`
    );
  }

  return result.data!;
}

/**
 * Check if a tool has a defined response schema
 */
export function hasResponseSchema(toolName: string): toolName is ToolName {
  return toolName in ToolResponseSchemaMap;
}

/**
 * Get all tool names that have response schemas
 */
export function getValidatedToolNames(): ToolName[] {
  return Object.keys(ToolResponseSchemaMap) as ToolName[];
}

// ============================================================================
// TYPE EXPORTS (inferred from Zod schemas)
// ============================================================================

export type ConnectProviderResponse = z.infer<typeof ConnectProviderResponseSchema>;
export type GetConnectionStatusResponse = z.infer<typeof GetConnectionStatusResponseSchema>;
export type DisconnectProviderResponse = z.infer<typeof DisconnectProviderResponseSchema>;
export type GetActivitiesResponse = z.infer<typeof GetActivitiesResponseSchema>;
export type GetAthleteResponse = z.infer<typeof GetAthleteResponseSchema>;
export type GetStatsResponse = z.infer<typeof GetStatsResponseSchema>;
export type GetActivityIntelligenceResponse = z.infer<typeof GetActivityIntelligenceResponseSchema>;
export type AnalyzeActivityResponse = z.infer<typeof AnalyzeActivityResponseSchema>;
export type CalculateMetricsResponse = z.infer<typeof CalculateMetricsResponseSchema>;
export type AnalyzePerformanceTrendsResponse = z.infer<typeof AnalyzePerformanceTrendsResponseSchema>;
export type CompareActivitiesResponse = z.infer<typeof CompareActivitiesResponseSchema>;
export type DetectPatternsResponse = z.infer<typeof DetectPatternsResponseSchema>;
export type GenerateRecommendationsResponse = z.infer<typeof GenerateRecommendationsResponseSchema>;
export type CalculateFitnessScoreResponse = z.infer<typeof CalculateFitnessScoreResponseSchema>;
export type PredictPerformanceResponse = z.infer<typeof PredictPerformanceResponseSchema>;
export type AnalyzeTrainingLoadResponse = z.infer<typeof AnalyzeTrainingLoadResponseSchema>;
export type SetGoalResponse = z.infer<typeof SetGoalResponseSchema>;
export type TrackProgressResponse = z.infer<typeof TrackProgressResponseSchema>;
export type SuggestGoalsResponse = z.infer<typeof SuggestGoalsResponseSchema>;
export type AnalyzeGoalFeasibilityResponse = z.infer<typeof AnalyzeGoalFeasibilityResponseSchema>;
export type GetConfigurationCatalogResponse = z.infer<typeof GetConfigurationCatalogResponseSchema>;
export type GetConfigurationProfilesResponse = z.infer<typeof GetConfigurationProfilesResponseSchema>;
export type GetUserConfigurationResponse = z.infer<typeof GetUserConfigurationResponseSchema>;
export type UpdateUserConfigurationResponse = z.infer<typeof UpdateUserConfigurationResponseSchema>;
export type CalculatePersonalizedZonesResponse = z.infer<typeof CalculatePersonalizedZonesResponseSchema>;
export type ValidateConfigurationResponse = z.infer<typeof ValidateConfigurationResponseSchema>;
export type GetFitnessConfigResponse = z.infer<typeof GetFitnessConfigResponseSchema>;
export type SetFitnessConfigResponse = z.infer<typeof SetFitnessConfigResponseSchema>;
export type ListFitnessConfigsResponse = z.infer<typeof ListFitnessConfigsResponseSchema>;
export type DeleteFitnessConfigResponse = z.infer<typeof DeleteFitnessConfigResponseSchema>;
export type CalculateDailyNutritionResponse = z.infer<typeof CalculateDailyNutritionResponseSchema>;
export type GetNutrientTimingResponse = z.infer<typeof GetNutrientTimingResponseSchema>;
export type SearchFoodResponse = z.infer<typeof SearchFoodResponseSchema>;
export type GetFoodDetailsResponse = z.infer<typeof GetFoodDetailsResponseSchema>;
export type AnalyzeMealNutritionResponse = z.infer<typeof AnalyzeMealNutritionResponseSchema>;
export type AnalyzeSleepQualityResponse = z.infer<typeof AnalyzeSleepQualityResponseSchema>;
export type CalculateRecoveryScoreResponse = z.infer<typeof CalculateRecoveryScoreResponseSchema>;
export type SuggestRestDayResponse = z.infer<typeof SuggestRestDayResponseSchema>;
export type TrackSleepTrendsResponse = z.infer<typeof TrackSleepTrendsResponseSchema>;
export type OptimizeSleepScheduleResponse = z.infer<typeof OptimizeSleepScheduleResponseSchema>;

/**
 * Union type of all validated response types
 */
export type AnyToolResponse = z.infer<typeof ToolResponseSchemaMap[ToolName]>;

/**
 * Map of tool names to their response types
 */
export type ToolResponseMap = {
  [K in ToolName]: z.infer<typeof ToolResponseSchemaMap[K]>;
};
