// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Main entry point for Pierre MCP Client TypeScript SDK
// ABOUTME: Re-exports MCP client and configuration for programmatic integration
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

/**
 * Pierre MCP Client SDK
 *
 * Export the main MCP client implementation for programmatic use
 */

export { PierreMcpClient, BridgeConfig } from './bridge';

/**
 * Export structured error types for typed error handling
 */
export { PierreError, PierreErrorCode } from './errors';

/**
 * Export all TypeScript type definitions for Pierre MCP tools
 *
 * These types are auto-generated from server tool schemas.
 * To regenerate: bun run generate-types
 */
export * from './types';

/**
 * Export Zod response schemas for runtime validation
 *
 * These schemas validate tool responses at runtime, ensuring type safety
 * between the SDK and server. Use these to validate responses manually
 * or enable automatic validation via BridgeConfig.responseValidation.
 *
 * Note: ToolName is exported from types.ts (input params), so we export
 * ResponseToolName from response-schemas for the output side.
 */
export {
  // Base schemas
  UuidSchema,
  TimestampSchema,
  QualityRatingSchema,
  ConnectionStatusSchema,
  GoalStatusSchema,
  ConfidenceLevelSchema,
  ScoreSchema,
  InsightsArraySchema,
  RecommendationsArraySchema,

  // Common patterns
  ResponseMetadataSchema,
  PaginationInfoSchema,
  ScoreBasedResponseSchema,
  ValidationResponseSchema,
  McpContentItemSchema,
  McpToolResponseBaseSchema,

  // All tool response schemas
  ConnectProviderResponseSchema,
  GetConnectionStatusResponseSchema,
  DisconnectProviderResponseSchema,
  GetActivitiesResponseSchema,
  GetAthleteResponseSchema,
  GetStatsResponseSchema,
  GetActivityIntelligenceResponseSchema,
  AnalyzeActivityResponseSchema,
  CalculateMetricsResponseSchema,
  AnalyzePerformanceTrendsResponseSchema,
  CompareActivitiesResponseSchema,
  DetectPatternsResponseSchema,
  GenerateRecommendationsResponseSchema,
  CalculateFitnessScoreResponseSchema,
  PredictPerformanceResponseSchema,
  AnalyzeTrainingLoadResponseSchema,
  SetGoalResponseSchema,
  TrackProgressResponseSchema,
  SuggestGoalsResponseSchema,
  AnalyzeGoalFeasibilityResponseSchema,
  GetConfigurationCatalogResponseSchema,
  GetConfigurationProfilesResponseSchema,
  GetUserConfigurationResponseSchema,
  UpdateUserConfigurationResponseSchema,
  CalculatePersonalizedZonesResponseSchema,
  ValidateConfigurationResponseSchema,
  GetFitnessConfigResponseSchema,
  SetFitnessConfigResponseSchema,
  ListFitnessConfigsResponseSchema,
  DeleteFitnessConfigResponseSchema,
  CalculateDailyNutritionResponseSchema,
  GetNutrientTimingResponseSchema,
  SearchFoodResponseSchema,
  GetFoodDetailsResponseSchema,
  AnalyzeMealNutritionResponseSchema,
  AnalyzeSleepQualityResponseSchema,
  CalculateRecoveryScoreResponseSchema,
  SuggestRestDayResponseSchema,
  TrackSleepTrendsResponseSchema,
  OptimizeSleepScheduleResponseSchema,

  // Schema map and utilities
  ToolResponseSchemaMap,
  validateToolResponse,
  validateToolResponseStrict,
  hasResponseSchema,
  getValidatedToolNames,

  // Response types (inferred from schemas)
  type ToolName as ResponseToolName,
  type AnyToolResponse,
  type ToolResponseMap,
  type ValidationResult,
} from './response-schemas';

/**
 * Export response validation utilities
 *
 * Use these to configure validation behavior, check validation stats,
 * or manually validate tool responses.
 */
export {
  validateMcpToolResponse,
  configureValidator,
  getValidatorConfig,
  createValidatedToolCall,
  isValidResponse,
  getValidationStats,
  resetValidationStats,
  validateWithStats,
  type ResponseValidatorConfig,
  type ValidatedToolResult,
  type ValidationStats,
} from './response-validator';

/**
 * Export secure storage utilities for token management
 *
 * These provide encrypted file-based storage for OAuth tokens and credentials.
 * The default storage location is ~/.pierre-mcp-tokens.enc
 */
export {
  createSecureStorage,
  EncryptedFileStorage,
  type SecureTokenStorage,
} from './secure-storage';