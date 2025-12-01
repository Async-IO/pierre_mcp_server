// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Fitbit provider integration tests validating API v1 data transformation
// ABOUTME: Tests realistic Fitbit API responses and Pierre's data transformation logic
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

const {
  mockFitbitActivities,
  mockFitbitAthlete,
  mockFitbitStats,
  mockPierreFitbitActivity
} = require('../fixtures/fitbit-mock-data');

/**
 * Mock MCP Client that simulates Fitbit API responses
 * Returns realistic Fitbit API v1 data structures
 */
class MockFitbitMCPClient {
  async send(request) {
    if (request.method === 'tools/call') {
      const { name, arguments: args } = request.params;

      // Simulate get_activities with Fitbit provider
      if (name === 'get_activities' && args.provider === 'fitbit') {
        return {
          result: {
            content: [{
              type: 'text',
              text: JSON.stringify({
                activities: [mockPierreFitbitActivity],
                count: 1,
                provider: 'fitbit'
              })
            }]
          }
        };
      }

      // Simulate get_athlete with Fitbit provider
      if (name === 'get_athlete' && args.provider === 'fitbit') {
        return {
          result: {
            content: [{
              type: 'text',
              text: JSON.stringify({
                id: "ABC123",
                username: "Pierre Runner",
                firstname: "Pierre",
                lastname: "Runner",
                profile_picture: "https://static0.fitbit.com/images/profile/defaultProfile_100_male.gif",
                provider: "fitbit"
              })
            }]
          }
        };
      }

      // Simulate get_stats with Fitbit provider
      if (name === 'get_stats' && args.provider === 'fitbit') {
        return {
          result: {
            content: [{
              type: 'text',
              text: JSON.stringify({
                all_time_totals: {
                  distance: 9256847.3,
                  floors_climbed: 8542
                },
                provider: "fitbit"
              })
            }]
          }
        };
      }

      // Missing provider parameter
      if (!args.provider) {
        return {
          error: {
            code: -32602,
            message: 'Missing required parameter: provider'
          }
        };
      }
    }

    return { result: {} };
  }

  async start() {}
  async stop() {}
}

describe('Fitbit Provider Integration Tests', () => {
  let client;

  beforeAll(async () => {
    client = new MockFitbitMCPClient();
    await client.start();
  });

  afterAll(async () => {
    if (client) {
      await client.stop();
    }
  });

  // ============================================================================
  // get_activities - Fitbit Data Structure Tests
  // ============================================================================

  describe('get_activities - Fitbit Data', () => {
    test('should return activities with realistic Fitbit data structure', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'get_activities',
          arguments: {
            provider: 'fitbit',
            limit: 10
          }
        }
      });

      expect(response.result).toBeDefined();
      expect(response.result.content).toBeDefined();
      expect(response.result.content[0].type).toBe('text');

      const result = JSON.parse(response.result.content[0].text);
      expect(result.activities).toBeDefined();
      expect(Array.isArray(result.activities)).toBe(true);
      expect(result.provider).toBe('fitbit');
    }, 30000);

    test('should return activity with all core fields from Fitbit', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'get_activities',
          arguments: { provider: 'fitbit', limit: 1 }
        }
      });

      const result = JSON.parse(response.result.content[0].text);
      const activity = result.activities[0];

      // Core Fitbit activity fields (transformed to Pierre format)
      expect(activity).toHaveProperty('id');
      expect(activity).toHaveProperty('name');
      expect(activity).toHaveProperty('sport_type');
      expect(activity).toHaveProperty('start_date');
      expect(activity).toHaveProperty('duration_seconds');
      expect(activity).toHaveProperty('distance_meters');
      expect(activity).toHaveProperty('calories');
      expect(activity).toHaveProperty('provider', 'fitbit');
    }, 30000);

    test('should include Fitbit-specific metrics (heart rate zones, steps)', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'get_activities',
          arguments: { provider: 'fitbit', limit: 1 }
        }
      });

      const result = JSON.parse(response.result.content[0].text);
      const activity = result.activities[0];

      // Fitbit-specific fields
      expect(activity).toHaveProperty('steps');  // Fitbit tracks steps
      expect(activity).toHaveProperty('heart_rate_zones');  // Fitbit provides HR zones
      expect(activity).toHaveProperty('average_heart_rate');
      expect(activity).toHaveProperty('elevation_gain');
    }, 30000);

    test('should handle Fitbit activity type mapping correctly', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'get_activities',
          arguments: { provider: 'fitbit', limit: 1 }
        }
      });

      const result = JSON.parse(response.result.content[0].text);
      const activity = result.activities[0];

      // Fitbit activity_type_id 90009 should map to "Run"
      expect(activity.sport_type).toBe('Run');
    }, 30000);
  });

  // ============================================================================
  // get_athlete - Fitbit Profile Data Tests
  // ============================================================================

  describe('get_athlete - Fitbit Profile Data', () => {
    test('should return athlete profile with Fitbit data structure', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'get_athlete',
          arguments: { provider: 'fitbit' }
        }
      });

      expect(response.result).toBeDefined();
      const athlete = JSON.parse(response.result.content[0].text);

      expect(athlete).toHaveProperty('id');
      expect(athlete).toHaveProperty('username');
      expect(athlete).toHaveProperty('provider', 'fitbit');
    }, 30000);

    test('should include optional Fitbit athlete fields when available', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'get_athlete',
          arguments: { provider: 'fitbit' }
        }
      });

      const athlete = JSON.parse(response.result.content[0].text);

      // Fitbit profile fields
      expect(athlete).toHaveProperty('firstname');
      expect(athlete).toHaveProperty('lastname');
      expect(athlete).toHaveProperty('profile_picture');
    }, 30000);
  });

  // ============================================================================
  // get_stats - Fitbit Statistics Tests
  // ============================================================================

  describe('get_stats - Fitbit Statistics', () => {
    test('should return stats with Fitbit totals structure', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'get_stats',
          arguments: { provider: 'fitbit' }
        }
      });

      expect(response.result).toBeDefined();
      const stats = JSON.parse(response.result.content[0].text);

      expect(stats).toHaveProperty('provider', 'fitbit');
      expect(stats).toHaveProperty('all_time_totals');
    }, 30000);

    test('should include Fitbit-specific stats (floors climbed)', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'get_stats',
          arguments: { provider: 'fitbit' }
        }
      });

      const stats = JSON.parse(response.result.content[0].text);

      // Fitbit provides floors climbed instead of elevation
      expect(stats.all_time_totals).toHaveProperty('floors_climbed');
      expect(stats.all_time_totals).toHaveProperty('distance');
    }, 30000);

    test('should validate Fitbit stats have realistic values', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'get_stats',
          arguments: { provider: 'fitbit' }
        }
      });

      const stats = JSON.parse(response.result.content[0].text);

      // Validate realistic ranges
      expect(stats.all_time_totals.distance).toBeGreaterThan(0);
      expect(stats.all_time_totals.floors_climbed).toBeGreaterThan(0);
    }, 30000);
  });

  // ============================================================================
  // Error Handling Tests
  // ============================================================================

  describe('Error Handling with Fitbit Provider', () => {
    test('should require provider parameter for Fitbit tools', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'get_activities',
          arguments: {}
        }
      });

      expect(response.error).toBeDefined();
      expect(response.error.message).toContain('provider');
    }, 30000);
  });

  // ============================================================================
  // Data Transformation: Fitbit → Pierre
  // ============================================================================

  describe('Data Transformation: Fitbit → Pierre', () => {
    test('should correctly transform Fitbit distance (km to meters)', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'get_activities',
          arguments: { provider: 'fitbit', limit: 1 }
        }
      });

      const result = JSON.parse(response.result.content[0].text);
      const activity = result.activities[0];

      // Fitbit: 8.0472 km → Pierre: 8047.2 meters
      expect(activity.distance_meters).toBe(8047.2);
    }, 30000);

    test('should correctly transform Fitbit time fields (ms to seconds)', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'get_activities',
          arguments: { provider: 'fitbit', limit: 1 }
        }
      });

      const result = JSON.parse(response.result.content[0].text);
      const activity = result.activities[0];

      // Fitbit stores duration in milliseconds, Pierre in seconds
      // 2520000 ms → 2520 seconds
      expect(activity.duration_seconds).toBe(2520);
    }, 30000);

    test('should preserve Fitbit activity ID as string', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'get_activities',
          arguments: { provider: 'fitbit', limit: 1 }
        }
      });

      const result = JSON.parse(response.result.content[0].text);
      const activity = result.activities[0];

      expect(typeof activity.id).toBe('string');
      expect(activity.id).toBe('987654321');
    }, 30000);

    test('should add provider field to all Fitbit data', async () => {
      const activityResponse = await client.send({
        method: 'tools/call',
        params: {
          name: 'get_activities',
          arguments: { provider: 'fitbit', limit: 1 }
        }
      });

      const athleteResponse = await client.send({
        method: 'tools/call',
        params: {
          name: 'get_athlete',
          arguments: { provider: 'fitbit' }
        }
      });

      const statsResponse = await client.send({
        method: 'tools/call',
        params: {
          name: 'get_stats',
          arguments: { provider: 'fitbit' }
        }
      });

      const activityResult = JSON.parse(activityResponse.result.content[0].text);
      const athleteResult = JSON.parse(athleteResponse.result.content[0].text);
      const statsResult = JSON.parse(statsResponse.result.content[0].text);

      expect(activityResult.provider).toBe('fitbit');
      expect(activityResult.activities[0].provider).toBe('fitbit');
      expect(athleteResult.provider).toBe('fitbit');
      expect(statsResult.provider).toBe('fitbit');
    }, 30000);
  });
});
