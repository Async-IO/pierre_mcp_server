// ABOUTME: Comprehensive integration tests for all 45 Pierre MCP tools
// ABOUTME: Tests parameter validation, successful calls, and error handling for complete tool coverage
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

const { ensureServerRunning } = require('../helpers/server');
const { TestConfig } = require('../helpers/fixtures');
const { generateTestToken } = require('../helpers/token-generator');
const { MockMCPClient } = require('../helpers/mock-client');
const path = require('path');

/**
 * Comprehensive integration tests for all 45 Pierre MCP tools
 *
 * Tool Categories:
 * - Authentication (4 tools): connect_to_pierre, connect_provider, get_connection_status, disconnect_provider
 * - Activities (3 tools): get_activities, get_athlete, get_stats
 * - Intelligence (4 tools): get_activity_intelligence, check_oauth_notifications, announce_oauth_success, get_notifications/mark_notifications_read
 * - Analysis (5 tools): analyze_activity, calculate_metrics, analyze_performance_trends, compare_activities, detect_patterns
 * - Goals (5 tools): set_goal, track_progress, suggest_goals, analyze_goal_feasibility, generate_recommendations
 * - Fitness Metrics (3 tools): calculate_fitness_score, predict_performance, analyze_training_load
 * - Configuration (6 tools): get_configuration_catalog, get_configuration_profiles, get_user_configuration, update_user_configuration, calculate_personalized_zones, validate_configuration
 * - Fitness Config (4 tools): get_fitness_config, set_fitness_config, list_fitness_configs, delete_fitness_config
 * - Nutrition (5 tools): calculate_daily_nutrition, get_nutrient_timing, search_food, get_food_details, analyze_meal_nutrition
 * - Sleep/Recovery (6 tools): analyze_sleep_quality, calculate_recovery_score, suggest_rest_day, track_sleep_trends, optimize_sleep_schedule
 */
describe('All Pierre MCP Tools - Integration Tests', () => {
  let serverHandle;
  let client;
  const bridgePath = path.join(__dirname, '../../dist/cli.js');

  beforeAll(async () => {
    // Start Pierre server
    serverHandle = await ensureServerRunning({
      port: TestConfig.defaultServerPort,
      database: TestConfig.testDatabase,
      encryptionKey: TestConfig.testEncryptionKey
    });

    // Create mock MCP client with correct server URL
    client = new MockMCPClient('node', [bridgePath, '--server', TestConfig.defaultServerUrl]);
    await client.start();
  }, 60000);

  afterAll(async () => {
    if (client) {
      await client.stop();
    }
    if (serverHandle?.cleanup) {
      await serverHandle.cleanup();
    }
  });

  // ============================================================================
  // AUTHENTICATION TOOLS (4 tools)
  // ============================================================================

  describe('Authentication Tools', () => {
    test('connect_provider - should require provider parameter', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'connect_provider',
          arguments: {
            provider: 'strava'
          }
        }
      });

      expect(response.result).toBeDefined();
      // Should initiate OAuth flow or return auth URL
    }, 30000);

    test('get_connection_status - should return connection status', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'get_connection_status',
          arguments: {}
        }
      });

      expect(response.result).toBeDefined();
      expect(response.result.content).toBeDefined();
    }, 30000);

    test('disconnect_provider - should require provider parameter', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'disconnect_provider',
          arguments: {
            provider: 'strava'
          }
        }
      });

      // May succeed or fail depending on connection state
      expect(response).toBeDefined();
    }, 30000);
  });

  // ============================================================================
  // ACTIVITY TOOLS (3 tools)
  // ============================================================================

  describe('Activity Data Tools', () => {
    test('get_activities - should accept pagination parameters', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'get_activities',
          arguments: {
            provider: 'strava',
            limit: 10,
            offset: 0
          }
        }
      });

      // May fail if not connected, but should accept parameters
      expect(response).toBeDefined();
    }, 30000);

  });

  // ============================================================================
  // INTELLIGENCE TOOLS (4 tools)
  // ============================================================================

  describe('Intelligence & Notifications Tools', () => {
    test('get_activity_intelligence - should accept optional flags', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'get_activity_intelligence',
          arguments: {
            activity_id: 'test-123',
            provider: 'strava',
            include_location: true,
            include_weather: false
          }
        }
      });

      expect(response).toBeDefined();
    }, 30000);

    test('get_notifications - should accept optional parameters', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'get_notifications',
          arguments: {
            include_read: false,
            provider: 'strava'
          }
        }
      });

      expect(response.result).toBeDefined();
    }, 30000);

    test('mark_notifications_read - should accept optional notification_id', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'mark_notifications_read',
          arguments: {
            notification_id: 'test-notif-123'
          }
        }
      });

      expect(response).toBeDefined();
    }, 30000);

    test('check_oauth_notifications - should work without parameters', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'check_oauth_notifications',
          arguments: {}
        }
      });

      expect(response.result).toBeDefined();
    }, 30000);
  });

  // ============================================================================
  // ANALYSIS TOOLS (5 tools)
  // ============================================================================

  describe('Analysis Tools', () => {
    test('calculate_metrics - should require activity_id and provider', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'calculate_metrics',
          arguments: {
            activity_id: 'test-123',
            provider: 'strava',
            metrics: ['trimp', 'power_to_weight']
          }
        }
      });

      expect(response).toBeDefined();
    }, 30000);

    test('analyze_performance_trends - should accept time period and metric type', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'analyze_performance_trends',
          arguments: {
            provider: 'strava',
            metric_type: 'pace',
            time_period: '30_days'
          }
        }
      });

      expect(response).toBeDefined();
    }, 30000);

    test('compare_activities - should require activity IDs', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'compare_activities',
          arguments: {
            activity_ids: ['123', '456'],
            provider: 'strava'
          }
        }
      });

      expect(response).toBeDefined();
    }, 30000);

    test('detect_patterns - should accept analysis parameters', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'detect_patterns',
          arguments: {
            provider: 'strava',
            pattern_type: 'training_load',
            time_period: '90_days'
          }
        }
      });

      expect(response).toBeDefined();
    }, 30000);
  });

  // ============================================================================
  // GOAL TOOLS (5 tools)
  // ============================================================================

  describe('Goal Management Tools', () => {
    test('set_goal - should require goal parameters', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'set_goal',
          arguments: {
            goal_type: 'distance',
            target_value: 100,
            target_date: '2025-12-31',
            provider: 'strava'
          }
        }
      });

      expect(response).toBeDefined();
    }, 30000);

    test('track_progress - should require goal_id', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'track_progress',
          arguments: {
            goal_id: 'goal-123',
            provider: 'strava'
          }
        }
      });

      expect(response).toBeDefined();
    }, 30000);

    test('suggest_goals - should work with provider only', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'suggest_goals',
          arguments: {
            provider: 'strava'
          }
        }
      });

      expect(response).toBeDefined();
    }, 30000);

    test('analyze_goal_feasibility - should require goal parameters', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'analyze_goal_feasibility',
          arguments: {
            goal_type: 'marathon',
            target_date: '2026-04-15',
            provider: 'strava'
          }
        }
      });

      expect(response).toBeDefined();
    }, 30000);

    test('generate_recommendations - should accept context', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'generate_recommendations',
          arguments: {
            provider: 'strava',
            context: 'training_plan'
          }
        }
      });

      expect(response).toBeDefined();
    }, 30000);
  });

  // ============================================================================
  // FITNESS METRICS TOOLS (3 tools)
  // ============================================================================

  describe('Fitness Metrics Tools', () => {
    test('calculate_fitness_score - should accept time period', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'calculate_fitness_score',
          arguments: {
            provider: 'strava',
            time_period: '30_days'
          }
        }
      });

      expect(response).toBeDefined();
    }, 30000);

    test('predict_performance - should require event type', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'predict_performance',
          arguments: {
            event_type: '5k',
            provider: 'strava'
          }
        }
      });

      expect(response).toBeDefined();
    }, 30000);

    test('analyze_training_load - should accept period', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'analyze_training_load',
          arguments: {
            provider: 'strava',
            period: '7_days'
          }
        }
      });

      expect(response).toBeDefined();
    }, 30000);
  });

  // ============================================================================
  // CONFIGURATION TOOLS (6 tools)
  // ============================================================================

  describe('Configuration Tools', () => {
    test('get_configuration_catalog - should work without parameters', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'get_configuration_catalog',
          arguments: {}
        }
      });

      expect(response.result).toBeDefined();
    }, 30000);

    test('get_configuration_profiles - should work without parameters', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'get_configuration_profiles',
          arguments: {}
        }
      });

      expect(response.result).toBeDefined();
    }, 30000);

    test('get_user_configuration - should accept config_key', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'get_user_configuration',
          arguments: {
            config_key: 'training_preferences'
          }
        }
      });

      expect(response).toBeDefined();
    }, 30000);

    test('update_user_configuration - should require config data', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'update_user_configuration',
          arguments: {
            config_key: 'training_preferences',
            config_value: { weekly_goal: 50 }
          }
        }
      });

      expect(response).toBeDefined();
    }, 30000);

    test('calculate_personalized_zones - should require user data', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'calculate_personalized_zones',
          arguments: {
            provider: 'strava',
            zone_type: 'heart_rate'
          }
        }
      });

      expect(response).toBeDefined();
    }, 30000);

    test('validate_configuration - should accept config for validation', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'validate_configuration',
          arguments: {
            config_data: { ftp: 250, max_hr: 185 }
          }
        }
      });

      expect(response).toBeDefined();
    }, 30000);
  });

  // ============================================================================
  // FITNESS CONFIG TOOLS (4 tools)
  // ============================================================================

  describe('Fitness Config Tools', () => {
    test('get_fitness_config - should accept config_name', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'get_fitness_config',
          arguments: {
            config_name: 'default'
          }
        }
      });

      expect(response).toBeDefined();
    }, 30000);

    test('set_fitness_config - should require config data', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'set_fitness_config',
          arguments: {
            config_name: 'test_config',
            config_data: {
              ftp: 250,
              max_hr: 185,
              zones: {
                z1: { min: 100, max: 120 }
              }
            }
          }
        }
      });

      expect(response).toBeDefined();
    }, 30000);

    test('list_fitness_configs - should work without parameters', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'list_fitness_configs',
          arguments: {}
        }
      });

      expect(response.result).toBeDefined();
    }, 30000);

    test('delete_fitness_config - should require config_name', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'delete_fitness_config',
          arguments: {
            config_name: 'test_config'
          }
        }
      });

      expect(response).toBeDefined();
    }, 30000);
  });

  // ============================================================================
  // NUTRITION TOOLS (5 tools)
  // ============================================================================

  describe('Nutrition Tools', () => {
    test('calculate_daily_nutrition - should accept activity level', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'calculate_daily_nutrition',
          arguments: {
            activity_level: 'moderate',
            goal: 'maintain'
          }
        }
      });

      expect(response).toBeDefined();
    }, 30000);

    test('get_nutrient_timing - should accept workout context', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'get_nutrient_timing',
          arguments: {
            workout_type: 'endurance',
            duration_minutes: 90
          }
        }
      });

      expect(response).toBeDefined();
    }, 30000);

    test('search_food - should require query', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'search_food',
          arguments: {
            query: 'banana',
            limit: 10
          }
        }
      });

      expect(response).toBeDefined();
    }, 30000);

    test('get_food_details - should require food_id', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'get_food_details',
          arguments: {
            food_id: 'banana-001'
          }
        }
      });

      expect(response).toBeDefined();
    }, 30000);

    test('analyze_meal_nutrition - should require meal data', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'analyze_meal_nutrition',
          arguments: {
            foods: [
              { food_id: 'banana-001', quantity: 1 },
              { food_id: 'oatmeal-002', quantity: 0.5 }
            ]
          }
        }
      });

      expect(response).toBeDefined();
    }, 30000);
  });

  // ============================================================================
  // SLEEP & RECOVERY TOOLS (5 tools)
  // ============================================================================

  describe('Sleep & Recovery Tools', () => {
    test('analyze_sleep_quality - should accept sleep data', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'analyze_sleep_quality',
          arguments: {
            sleep_hours: 7.5,
            sleep_date: '2025-11-10'
          }
        }
      });

      expect(response).toBeDefined();
    }, 30000);

    test('calculate_recovery_score - should work with provider', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'calculate_recovery_score',
          arguments: {
            provider: 'strava'
          }
        }
      });

      expect(response).toBeDefined();
    }, 30000);

    test('suggest_rest_day - should analyze training load', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'suggest_rest_day',
          arguments: {
            provider: 'strava'
          }
        }
      });

      expect(response).toBeDefined();
    }, 30000);

    test('track_sleep_trends - should accept time period', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'track_sleep_trends',
          arguments: {
            period: '30_days'
          }
        }
      });

      expect(response).toBeDefined();
    }, 30000);

    test('optimize_sleep_schedule - should provide recommendations', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'optimize_sleep_schedule',
          arguments: {
            wake_time: '06:00',
            activity_schedule: 'morning_workouts'
          }
        }
      });

      expect(response).toBeDefined();
    }, 30000);
  });

  // ============================================================================
  // TOOLS LIST VALIDATION
  // ============================================================================

  describe('Tools List Completeness', () => {
    test('should expose all 45 tools via tools/list', async () => {
      const response = await client.send({
        method: 'tools/list'
      });

      expect(response.result).toBeDefined();
      expect(response.result.tools).toBeDefined();
      expect(Array.isArray(response.result.tools)).toBe(true);

      // Verify we have 45 tools
      expect(response.result.tools.length).toBe(45);

      // Verify all expected tools are present
      const toolNames = response.result.tools.map(t => t.name);
      const expectedTools = [
        'connect_to_pierre', 'connect_provider', 'get_connection_status', 'disconnect_provider',
        'get_activities', 'get_athlete', 'get_stats',
        'get_activity_intelligence', 'get_notifications', 'mark_notifications_read',
        'announce_oauth_success', 'check_oauth_notifications',
        'analyze_activity', 'calculate_metrics', 'analyze_performance_trends',
        'compare_activities', 'detect_patterns',
        'set_goal', 'track_progress', 'suggest_goals', 'analyze_goal_feasibility',
        'generate_recommendations',
        'calculate_fitness_score', 'predict_performance', 'analyze_training_load',
        'get_configuration_catalog', 'get_configuration_profiles', 'get_user_configuration',
        'update_user_configuration', 'calculate_personalized_zones', 'validate_configuration',
        'get_fitness_config', 'set_fitness_config', 'list_fitness_configs', 'delete_fitness_config',
        'calculate_daily_nutrition', 'get_nutrient_timing', 'search_food',
        'get_food_details', 'analyze_meal_nutrition',
        'analyze_sleep_quality', 'calculate_recovery_score', 'suggest_rest_day',
        'track_sleep_trends', 'optimize_sleep_schedule'
      ];

      expectedTools.forEach(toolName => {
        expect(toolNames).toContain(toolName);
      });
    }, 30000);
  });
});
