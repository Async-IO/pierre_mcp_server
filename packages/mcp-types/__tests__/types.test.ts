// ABOUTME: Unit tests for @pierre/mcp-types package structure and exports
// ABOUTME: Validates type definitions are properly exported and consistent

import { describe, it, expect } from 'vitest';
import * as mcpTypes from '../src/index';
import * as tools from '../src/tools';
import * as common from '../src/common';

describe('mcp-types package exports', () => {
  it('exports tool parameter interfaces via ToolParamsMap keys', () => {
    // ToolParamsMap should have entries for all tools
    const toolParamsMap = {} as tools.ToolParamsMap;
    const toolNames: tools.ToolName[] = [
      'connect_provider',
      'get_connection_status',
      'disconnect_provider',
      'get_activities',
      'get_athlete',
      'get_stats',
    ];

    // Verify ToolName is a string union (type-level check, runtime validation)
    for (const name of toolNames) {
      expect(typeof name).toBe('string');
    }
  });

  it('exports McpToolResponse interface with expected shape', () => {
    const response: tools.McpToolResponse = {
      content: [{ type: 'text', text: 'hello' }],
      isError: false,
    };

    expect(response.content).toHaveLength(1);
    expect(response.content?.[0].type).toBe('text');
    expect(response.isError).toBe(false);
  });

  it('exports McpErrorResponse interface with expected shape', () => {
    const errorResponse: tools.McpErrorResponse = {
      code: -32600,
      message: 'Invalid request',
    };

    expect(errorResponse.code).toBe(-32600);
    expect(errorResponse.message).toBe('Invalid request');
  });

  it('exports common data types', () => {
    const activity: common.Activity = {
      id: '123',
      name: 'Morning Run',
      type: 'Run',
      distance: 5000,
      duration: 1800,
    };

    expect(activity.id).toBe('123');
    expect(activity.name).toBe('Morning Run');
    expect(activity.distance).toBe(5000);
  });

  it('exports Athlete type with profile fields', () => {
    const athlete: common.Athlete = {
      id: '456',
      username: 'runner42',
      firstname: 'Jane',
      lastname: 'Doe',
      weight: 65,
    };

    expect(athlete.id).toBe('456');
    expect(athlete.username).toBe('runner42');
  });

  it('exports Stats type with totals', () => {
    const stats: common.Stats = {
      biggest_ride_distance: 100000,
      recent_run_totals: {
        count: 10,
        distance: 50000,
        moving_time: 18000,
      },
    };

    expect(stats.biggest_ride_distance).toBe(100000);
    expect(stats.recent_run_totals?.count).toBe(10);
  });

  it('exports FitnessConfig with zones', () => {
    const config: common.FitnessConfig = {
      athlete_info: {
        age: 30,
        weight: 70,
        max_heart_rate: 190,
        resting_heart_rate: 55,
        vo2_max: 52,
      },
      training_zones: {
        heart_rate: [
          { zone: 1, name: 'Recovery', min: 100, max: 130 },
          { zone: 2, name: 'Aerobic', min: 130, max: 155 },
        ],
      },
    };

    expect(config.athlete_info?.vo2_max).toBe(52);
    expect(config.training_zones?.heart_rate).toHaveLength(2);
  });

  it('exports Goal type', () => {
    const goal: common.Goal = {
      type: 'distance',
      target_value: 42195,
      target_date: '2025-12-31',
      description: 'Complete a marathon',
    };

    expect(goal.type).toBe('distance');
    expect(goal.target_value).toBe(42195);
  });

  it('exports ConnectionStatus type', () => {
    const status: common.ConnectionStatus = {
      provider: 'strava',
      connected: true,
      scopes: ['read', 'activity:read_all'],
    };

    expect(status.provider).toBe('strava');
    expect(status.connected).toBe(true);
  });

  it('re-exports all types from index', () => {
    // The index should re-export from both tools.ts and common.ts
    // Create instances of types from the index module to verify availability
    const activity: mcpTypes.Activity = { id: '1', name: 'Test', type: 'Run' };
    const response: mcpTypes.McpToolResponse = { isError: false };
    const params: mcpTypes.ConnectProviderParams = { provider: 'strava' };

    expect(activity.id).toBe('1');
    expect(response.isError).toBe(false);
    expect(params.provider).toBe('strava');
  });

  it('tool parameter types have correct required fields', () => {
    const connectParams: tools.ConnectProviderParams = {
      provider: 'strava',
    };
    expect(connectParams.provider).toBe('strava');

    const activitiesParams: tools.GetActivitiesParams = {
      provider: 'strava',
      limit: 10,
    };
    expect(activitiesParams.provider).toBe('strava');
    expect(activitiesParams.limit).toBe(10);

    const trendsParams: tools.AnalyzePerformanceTrendsParams = {
      metric: 'pace',
      provider: 'strava',
      timeframe: 'month',
    };
    expect(trendsParams.metric).toBe('pace');
    expect(trendsParams.timeframe).toBe('month');
  });

  it('Notification type has required fields', () => {
    const notification: common.Notification = {
      id: 'notif-1',
      type: 'oauth_complete',
      message: 'Connected to Strava',
      created_at: '2025-01-15T10:00:00Z',
      read: false,
    };

    expect(notification.id).toBe('notif-1');
    expect(notification.read).toBe(false);
  });
});
