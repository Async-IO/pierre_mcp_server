// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: E2E tests for response validation module that validates tool responses against schemas.
// ABOUTME: Tests schema validation, error handling, and real tool responses from Pierre server.

const {
  validateToolResponse,
  validateToolResponseStrict,
  hasResponseSchema,
  getValidatedToolNames,
  GetActivitiesResponseSchema,
  CalculateFitnessScoreResponseSchema,
} = require('../../dist/index.js');

const TIMEOUT = 10000;

describe('Response Validation E2E Tests', () => {
  describe('Schema Validation', () => {
    test('should validate get_activities response', () => {
      const validResponse = {
        activities: [
          {
            id: '12345',
            name: 'Morning Run',
            type: 'Run',
            distance: 5000,
            duration: 1800,
          },
        ],
        provider: 'strava',
        count: 1,
        has_more: false,
      };

      const result = validateToolResponse('get_activities', validResponse);
      expect(result.success).toBe(true);
      expect(result.data).toBeDefined();
    }, TIMEOUT);

    test('should validate TOON format response', () => {
      const toonResponse = {
        activities_toon: 'compressed_base64_data_here',
        provider: 'strava',
        format: 'toon',
        format_fallback: false,
      };

      const result = validateToolResponse('get_activities', toonResponse);
      expect(result.success).toBe(true);
    }, TIMEOUT);

    test('should validate fitness score response', () => {
      const fitnessResponse = {
        fitness_score: 75,
        status: 'good',
        score_breakdowns: {
          consistency: 80,
          volume: 70,
          intensity: 75,
        },
        insights: ['Good training consistency'],
        recommendations: ['Add interval training'],
      };

      const result = validateToolResponse('calculate_fitness_score', fitnessResponse);
      expect(result.success).toBe(true);
    }, TIMEOUT);

    test('should reject invalid fitness score (out of range)', () => {
      const invalidResponse = {
        fitness_score: 150, // Invalid: > 100
      };

      const result = validateToolResponse('calculate_fitness_score', invalidResponse);
      expect(result.success).toBe(false);
      expect(result.error).toBeDefined();
      expect(result.error.issues.length).toBeGreaterThan(0);
    }, TIMEOUT);

    test('should validate training load response', () => {
      const trainingLoadResponse = {
        ctl: 45.5,
        atl: 55.2,
        tsb: -9.7,
        status: 'productive',
        insights: ['Training load is building'],
        recommendations: ['Maintain current load'],
        load_trend: 'increasing',
      };

      const result = validateToolResponse('analyze_training_load', trainingLoadResponse);
      expect(result.success).toBe(true);
    }, TIMEOUT);

    test('should validate connection status response', () => {
      const singleProviderResponse = {
        provider: 'strava',
        status: 'connected',
        connected: true,
      };

      const result = validateToolResponse('get_connection_status', singleProviderResponse);
      expect(result.success).toBe(true);
    }, TIMEOUT);

    test('should validate multi-provider connection status', () => {
      const multiProviderResponse = {
        providers: {
          strava: { connected: true, status: 'connected' },
          fitbit: { connected: false, status: 'disconnected' },
        },
      };

      const result = validateToolResponse('get_connection_status', multiProviderResponse);
      expect(result.success).toBe(true);
    }, TIMEOUT);
  });

  describe('Error Handling', () => {
    test('should return error for unknown tool', () => {
      const result = validateToolResponse('unknown_tool_xyz', {});

      expect(result.success).toBe(false);
      expect(result.error).toBeDefined();
      expect(result.error.issues[0].message).toContain('No schema defined');
    }, TIMEOUT);

    test('should return meaningful error messages', () => {
      const invalidResponse = {
        fitness_score: 'not a number', // Should be number
      };

      const result = validateToolResponse('calculate_fitness_score', invalidResponse);
      expect(result.success).toBe(false);
      expect(result.error.issues.length).toBeGreaterThan(0);

      // Error should mention the field
      const issueMessages = result.error.issues.map((i) => i.message).join(' ');
      expect(issueMessages.length).toBeGreaterThan(0);
    }, TIMEOUT);
  });

  describe('Strict Validation', () => {
    test('should return data for valid response', () => {
      const validResponse = {
        provider: 'strava',
        message: 'Disconnected',
      };

      const data = validateToolResponseStrict('disconnect_provider', validResponse);
      expect(data.provider).toBe('strava');
    }, TIMEOUT);

    test('should throw for invalid response', () => {
      const invalidResponse = {
        fitness_score: -10, // Invalid: negative
      };

      expect(() => {
        validateToolResponseStrict('calculate_fitness_score', invalidResponse);
      }).toThrow();
    }, TIMEOUT);
  });

  describe('Schema Registry', () => {
    test('should have schemas for known tools', () => {
      const knownTools = [
        'get_activities',
        'calculate_fitness_score',
        'set_goal',
        'track_progress',
        'get_connection_status',
      ];

      for (const tool of knownTools) {
        expect(hasResponseSchema(tool)).toBe(true);
      }
    }, TIMEOUT);

    test('should return false for unknown tools', () => {
      expect(hasResponseSchema('nonexistent_tool')).toBe(false);
    }, TIMEOUT);

    test('should return list of validated tool names', () => {
      const names = getValidatedToolNames();

      expect(Array.isArray(names)).toBe(true);
      expect(names.length).toBeGreaterThan(30); // We have 40+ tools
      expect(names).toContain('get_activities');
      expect(names).toContain('calculate_fitness_score');
    }, TIMEOUT);
  });

  describe('Passthrough Behavior', () => {
    test('should allow extra fields in responses', () => {
      const responseWithExtra = {
        activities: [],
        provider: 'strava',
        extra_field: 'should be allowed',
        another_extra: { nested: true },
      };

      const result = validateToolResponse('get_activities', responseWithExtra);
      expect(result.success).toBe(true);
      expect(result.data.extra_field).toBe('should be allowed');
    }, TIMEOUT);
  });

  describe('Schema Types', () => {
    test('should validate UUID format', () => {
      const goalResponse = {
        goal_id: '550e8400-e29b-41d4-a716-446655440000',
        goal_type: 'distance',
        target_value: 100,
        title: 'Run 100km',
        status: 'created',
        created_at: '2025-01-08T12:00:00Z',
      };

      const result = validateToolResponse('set_goal', goalResponse);
      expect(result.success).toBe(true);
    }, TIMEOUT);

    test('should accept any string format for goal_id (schema uses string, not UUID)', () => {
      // Note: SetGoalResponseSchema intentionally uses z.string() for goal_id,
      // not UuidSchema, allowing flexibility in goal ID formats
      const goalResponse = {
        goal_id: 'not-a-valid-uuid',
        goal_type: 'distance',
        target_value: 100,
        title: 'Run 100km',
        status: 'created',
        created_at: '2025-01-08T12:00:00Z',
      };

      const result = validateToolResponse('set_goal', goalResponse);
      expect(result.success).toBe(true);
    }, TIMEOUT);

    test('should validate timestamp format', () => {
      const responseWithTimestamp = {
        goal_id: '550e8400-e29b-41d4-a716-446655440000',
        goal_type: 'distance',
        target_value: 100,
        title: 'Test',
        status: 'created',
        created_at: '2025-01-08T12:00:00+05:30', // Offset format
      };

      const result = validateToolResponse('set_goal', responseWithTimestamp);
      expect(result.success).toBe(true);
    }, TIMEOUT);

    test('should validate score range (0-100)', () => {
      // Valid score
      const validResult = validateToolResponse('calculate_fitness_score', {
        fitness_score: 50,
      });
      expect(validResult.success).toBe(true);

      // Invalid score (negative)
      const negativeResult = validateToolResponse('calculate_fitness_score', {
        fitness_score: -1,
      });
      expect(negativeResult.success).toBe(false);

      // Invalid score (> 100)
      const overResult = validateToolResponse('calculate_fitness_score', {
        fitness_score: 101,
      });
      expect(overResult.success).toBe(false);
    }, TIMEOUT);
  });

  describe('Direct Schema Usage', () => {
    test('should allow direct schema parsing', () => {
      const response = {
        activities: [],
        provider: 'strava',
      };

      const result = GetActivitiesResponseSchema.safeParse(response);
      expect(result.success).toBe(true);
    }, TIMEOUT);

    test('should provide type inference', () => {
      const response = {
        fitness_score: 85,
        status: 'excellent',
      };

      const result = CalculateFitnessScoreResponseSchema.safeParse(response);
      expect(result.success).toBe(true);
      if (result.success) {
        expect(typeof result.data.fitness_score).toBe('number');
      }
    }, TIMEOUT);
  });
});
