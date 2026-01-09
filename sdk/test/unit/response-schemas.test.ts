// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Unit tests for Zod response schema validation
// ABOUTME: Tests schema structure, validation logic, and type inference

import { describe, test, expect } from "bun:test";
import {
  // Base schemas
  UuidSchema,
  TimestampSchema,
  QualityRatingSchema,
  ScoreSchema,
  ConfidenceLevelSchema,

  // Tool response schemas
  GetActivitiesResponseSchema,
  GetAthleteResponseSchema,
  GetStatsResponseSchema,
  GetActivityIntelligenceResponseSchema,
  CalculateFitnessScoreResponseSchema,
  AnalyzeTrainingLoadResponseSchema,
  SetGoalResponseSchema,
  TrackProgressResponseSchema,
  GetConnectionStatusResponseSchema,
  DisconnectProviderResponseSchema,
  CalculateDailyNutritionResponseSchema,
  AnalyzeSleepQualityResponseSchema,
  CalculateRecoveryScoreResponseSchema,
  GetConfigurationCatalogResponseSchema,
  GetUserConfigurationResponseSchema,

  // Validation utilities
  validateToolResponse,
  validateToolResponseStrict,
  hasResponseSchema,
  getValidatedToolNames,
  ToolResponseSchemaMap,
} from "../../src/response-schemas";

describe("Base Schemas", () => {
  describe("UuidSchema", () => {
    test("accepts valid UUID", () => {
      const result = UuidSchema.safeParse("550e8400-e29b-41d4-a716-446655440000");
      expect(result.success).toBe(true);
    });

    test("rejects invalid UUID", () => {
      const result = UuidSchema.safeParse("not-a-uuid");
      expect(result.success).toBe(false);
    });
  });

  describe("TimestampSchema", () => {
    test("accepts RFC3339 timestamp", () => {
      const result = TimestampSchema.safeParse("2025-01-08T12:00:00Z");
      expect(result.success).toBe(true);
    });

    test("accepts timestamp with offset", () => {
      const result = TimestampSchema.safeParse("2025-01-08T12:00:00+05:30");
      expect(result.success).toBe(true);
    });
  });

  describe("QualityRatingSchema", () => {
    test("accepts valid ratings", () => {
      for (const rating of ["excellent", "good", "moderate", "fair", "poor"]) {
        const result = QualityRatingSchema.safeParse(rating);
        expect(result.success).toBe(true);
      }
    });

    test("rejects invalid rating", () => {
      const result = QualityRatingSchema.safeParse("amazing");
      expect(result.success).toBe(false);
    });
  });

  describe("ScoreSchema", () => {
    test("accepts scores 0-100", () => {
      expect(ScoreSchema.safeParse(0).success).toBe(true);
      expect(ScoreSchema.safeParse(50).success).toBe(true);
      expect(ScoreSchema.safeParse(100).success).toBe(true);
    });

    test("rejects scores outside range", () => {
      expect(ScoreSchema.safeParse(-1).success).toBe(false);
      expect(ScoreSchema.safeParse(101).success).toBe(false);
    });
  });

  describe("ConfidenceLevelSchema", () => {
    test("accepts values 0-1", () => {
      expect(ConfidenceLevelSchema.safeParse(0).success).toBe(true);
      expect(ConfidenceLevelSchema.safeParse(0.5).success).toBe(true);
      expect(ConfidenceLevelSchema.safeParse(1).success).toBe(true);
    });

    test("rejects values outside range", () => {
      expect(ConfidenceLevelSchema.safeParse(-0.1).success).toBe(false);
      expect(ConfidenceLevelSchema.safeParse(1.1).success).toBe(false);
    });
  });
});

describe("Tool Response Schemas", () => {
  describe("GetActivitiesResponseSchema", () => {
    test("accepts valid activities response", () => {
      const response = {
        activities: [
          {
            id: "12345",
            name: "Morning Run",
            type: "Run",
            distance: 5000,
            duration: 1800,
          },
        ],
        provider: "strava",
        count: 1,
        has_more: false,
      };
      const result = GetActivitiesResponseSchema.safeParse(response);
      expect(result.success).toBe(true);
    });

    test("accepts TOON format response", () => {
      const response = {
        activities_toon: "compressed_data_here",
        provider: "strava",
        format: "toon" as const,
        format_fallback: false,
      };
      const result = GetActivitiesResponseSchema.safeParse(response);
      expect(result.success).toBe(true);
    });

    test("allows extra fields (passthrough)", () => {
      const response = {
        activities: [],
        provider: "strava",
        extra_field: "should be allowed",
      };
      const result = GetActivitiesResponseSchema.safeParse(response);
      expect(result.success).toBe(true);
    });
  });

  describe("GetAthleteResponseSchema", () => {
    test("accepts valid athlete response", () => {
      const response = {
        athlete: {
          id: "123",
          username: "testuser",
          firstname: "Test",
          lastname: "User",
        },
        format: "json" as const,
      };
      const result = GetAthleteResponseSchema.safeParse(response);
      expect(result.success).toBe(true);
    });
  });

  describe("CalculateFitnessScoreResponseSchema", () => {
    test("accepts valid fitness score response", () => {
      const response = {
        fitness_score: 75,
        status: "good" as const,
        score_breakdowns: {
          consistency: 80,
          volume: 70,
          intensity: 75,
        },
        insights: ["Good training consistency", "Volume is adequate"],
        recommendations: ["Consider adding interval training"],
      };
      const result = CalculateFitnessScoreResponseSchema.safeParse(response);
      expect(result.success).toBe(true);
    });

    test("rejects score outside 0-100", () => {
      const response = {
        fitness_score: 150,
      };
      const result = CalculateFitnessScoreResponseSchema.safeParse(response);
      expect(result.success).toBe(false);
    });
  });

  describe("AnalyzeTrainingLoadResponseSchema", () => {
    test("accepts valid training load response", () => {
      const response = {
        ctl: 45.5,
        atl: 55.2,
        tsb: -9.7,
        status: "productive",
        insights: ["Training load is building"],
        recommendations: ["Maintain current load"],
        load_trend: "increasing" as const,
      };
      const result = AnalyzeTrainingLoadResponseSchema.safeParse(response);
      expect(result.success).toBe(true);
    });
  });

  describe("SetGoalResponseSchema", () => {
    test("accepts valid goal creation response", () => {
      const response = {
        goal_id: "550e8400-e29b-41d4-a716-446655440000",
        goal_type: "distance",
        target_value: 100,
        title: "Run 100km this month",
        status: "created",
        created_at: "2025-01-08T12:00:00Z",
      };
      const result = SetGoalResponseSchema.safeParse(response);
      expect(result.success).toBe(true);
    });
  });

  describe("TrackProgressResponseSchema", () => {
    test("accepts valid progress response", () => {
      const response = {
        goal_id: "550e8400-e29b-41d4-a716-446655440000",
        progress_percentage: 45.5,
        days_remaining: 15,
        status: "in_progress",
        on_track: true,
        insights: ["You are making good progress"],
      };
      const result = TrackProgressResponseSchema.safeParse(response);
      expect(result.success).toBe(true);
    });
  });

  describe("GetConnectionStatusResponseSchema", () => {
    test("accepts single provider response", () => {
      const response = {
        provider: "strava",
        status: "connected" as const,
        connected: true,
      };
      const result = GetConnectionStatusResponseSchema.safeParse(response);
      expect(result.success).toBe(true);
    });

    test("accepts multi-provider response", () => {
      const response = {
        providers: {
          strava: { connected: true, status: "connected" as const },
          fitbit: { connected: false, status: "disconnected" as const },
        },
      };
      const result = GetConnectionStatusResponseSchema.safeParse(response);
      expect(result.success).toBe(true);
    });
  });

  describe("CalculateDailyNutritionResponseSchema", () => {
    test("accepts valid nutrition response", () => {
      const response = {
        calories: 2500,
        protein_g: 150,
        carbs_g: 300,
        fat_g: 80,
        bmr: 1800,
        tdee: 2500,
      };
      const result = CalculateDailyNutritionResponseSchema.safeParse(response);
      expect(result.success).toBe(true);
    });
  });

  describe("AnalyzeSleepQualityResponseSchema", () => {
    test("accepts valid sleep quality response", () => {
      const response = {
        sleep_quality_score: 85,
        quality_rating: "good" as const,
        sleep_efficiency: 92,
        sleep_stages: {
          deep_sleep_hours: 1.5,
          rem_sleep_hours: 2.0,
          light_sleep_hours: 4.5,
        },
        insights: ["Good sleep duration"],
        recommendations: ["Try to sleep earlier"],
      };
      const result = AnalyzeSleepQualityResponseSchema.safeParse(response);
      expect(result.success).toBe(true);
    });
  });

  describe("CalculateRecoveryScoreResponseSchema", () => {
    test("accepts valid recovery score response", () => {
      const response = {
        recovery_score: 78,
        recovery_status: "good" as const,
        training_readiness: "ready for moderate training",
        recommendations: ["Consider light activity today"],
      };
      const result = CalculateRecoveryScoreResponseSchema.safeParse(response);
      expect(result.success).toBe(true);
    });
  });

  describe("GetConfigurationCatalogResponseSchema", () => {
    test("accepts valid catalog response", () => {
      const response = {
        catalog: {
          vo2_max: { type: "float", min: 20, max: 90 },
          ftp: { type: "integer", min: 50, max: 500 },
        },
        total_parameters: 2,
      };
      const result = GetConfigurationCatalogResponseSchema.safeParse(response);
      expect(result.success).toBe(true);
    });
  });

  describe("GetUserConfigurationResponseSchema", () => {
    test("accepts valid user config response", () => {
      const response = {
        user_id: "550e8400-e29b-41d4-a716-446655440000",
        active_profile: "recreational",
        configuration: {
          profile: { name: "recreational" },
          session_overrides: {},
        },
      };
      const result = GetUserConfigurationResponseSchema.safeParse(response);
      expect(result.success).toBe(true);
    });
  });
});

describe("Validation Utilities", () => {
  describe("validateToolResponse", () => {
    test("returns success for valid response", () => {
      const response = {
        activities: [],
        provider: "strava",
      };
      const result = validateToolResponse("get_activities", response);
      expect(result.success).toBe(true);
      expect(result.data).toBeDefined();
    });

    test("returns error for invalid response", () => {
      const response = {
        fitness_score: 200, // Invalid: > 100
      };
      const result = validateToolResponse("calculate_fitness_score", response);
      expect(result.success).toBe(false);
      expect(result.error).toBeDefined();
      expect(result.error?.issues.length).toBeGreaterThan(0);
    });

    test("returns error for unknown tool", () => {
      const result = validateToolResponse("unknown_tool" as any, {});
      expect(result.success).toBe(false);
      expect(result.error?.issues[0].message).toContain("No schema defined");
    });
  });

  describe("validateToolResponseStrict", () => {
    test("returns data for valid response", () => {
      const response = {
        provider: "strava",
        message: "Disconnected",
      };
      const data = validateToolResponseStrict("disconnect_provider", response);
      expect(data.provider).toBe("strava");
    });

    test("throws for invalid response", () => {
      const response = {
        fitness_score: -10, // Invalid
      };
      expect(() => {
        validateToolResponseStrict("calculate_fitness_score", response);
      }).toThrow();
    });
  });

  describe("hasResponseSchema", () => {
    test("returns true for known tools", () => {
      expect(hasResponseSchema("get_activities")).toBe(true);
      expect(hasResponseSchema("calculate_fitness_score")).toBe(true);
      expect(hasResponseSchema("set_goal")).toBe(true);
    });

    test("returns false for unknown tools", () => {
      expect(hasResponseSchema("unknown_tool")).toBe(false);
    });
  });

  describe("getValidatedToolNames", () => {
    test("returns all tool names with schemas", () => {
      const names = getValidatedToolNames();
      expect(names.length).toBeGreaterThan(30); // We have 40 tools
      expect(names).toContain("get_activities");
      expect(names).toContain("calculate_fitness_score");
    });
  });

  describe("ToolResponseSchemaMap", () => {
    test("contains schemas for all expected tools", () => {
      const expectedTools = [
        "connect_provider",
        "get_connection_status",
        "disconnect_provider",
        "get_activities",
        "get_athlete",
        "get_stats",
        "get_activity_intelligence",
        "analyze_activity",
        "calculate_metrics",
        "analyze_performance_trends",
        "compare_activities",
        "detect_patterns",
        "generate_recommendations",
        "calculate_fitness_score",
        "predict_performance",
        "analyze_training_load",
        "set_goal",
        "track_progress",
        "suggest_goals",
        "analyze_goal_feasibility",
        "calculate_daily_nutrition",
        "analyze_sleep_quality",
        "calculate_recovery_score",
      ];

      for (const tool of expectedTools) {
        expect(ToolResponseSchemaMap).toHaveProperty(tool);
      }
    });
  });
});

describe("Schema Coverage", () => {
  test("all schemas in ToolResponseSchemaMap are valid Zod schemas", () => {
    for (const [toolName, schema] of Object.entries(ToolResponseSchemaMap)) {
      expect(schema).toBeDefined();
      expect(typeof schema.safeParse).toBe("function");
      // Verify each schema can parse an empty object (with passthrough)
      // This tests that the schema structure is valid
      const result = schema.safeParse({});
      // We don't require success, just that parsing doesn't throw
      expect(result).toBeDefined();
    }
  });
});
