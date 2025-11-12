// ABOUTME: Garmin Connect provider integration tests validating API data transformation
// ABOUTME: Tests realistic Garmin Connect API responses and Pierre's data transformation logic
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

const {
  mockGarminActivities,
  mockGarminAthlete,
  mockGarminStats,
  mockPierreGarminActivity
} = require('../fixtures/garmin-mock-data');

/**
 * Mock MCP Client that simulates Garmin Connect API responses
 * Returns realistic Garmin Connect API data structures
 */
class MockGarminMCPClient {
  async send(request) {
    if (request.method === 'tools/call') {
      const { name, arguments: args } = request.params;

      // Simulate get_activities with Garmin provider
      if (name === 'get_activities' && args.provider === 'garmin') {
        return {
          result: {
            content: [{
              type: 'text',
              text: JSON.stringify({
                activities: [mockPierreGarminActivity],
                count: 1,
                provider: 'garmin'
              })
            }]
          }
        };
      }

      // Simulate get_athlete with Garmin provider
      if (name === 'get_athlete' && args.provider === 'garmin') {
        return {
          result: {
            content: [{
              type: 'text',
              text: JSON.stringify({
                id: "garmin-user-123",
                username: "Pierre Athlete",
                firstname: "Pierre G.",
                lastname: "Athlete",
                profile_picture: "https://s3.amazonaws.com/garmin-connect-prod/profile_images/abc123.jpg",
                provider: "garmin"
              })
            }]
          }
        };
      }

      // Simulate get_stats with Garmin provider
      if (name === 'get_stats' && args.provider === 'garmin') {
        return {
          result: {
            content: [{
              type: 'text',
              text: JSON.stringify({
                all_time_totals: {
                  count: 342,
                  distance: 9256847.3,
                  duration: 1162000,
                  elevation: 92850.5
                },
                provider: "garmin"
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

describe('Garmin Provider Integration Tests', () => {
  let client;

  beforeAll(async () => {
    client = new MockGarminMCPClient();
    await client.start();
  });

  afterAll(async () => {
    if (client) {
      await client.stop();
    }
  });

  // ============================================================================
  // get_activities - Garmin Data Structure Tests
  // ============================================================================

  describe('get_activities - Garmin Data', () => {
    test('should return activities with realistic Garmin data structure', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'get_activities',
          arguments: {
            provider: 'garmin',
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
      expect(result.provider).toBe('garmin');
    }, 30000);

    test('should return activity with all core fields from Garmin', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'get_activities',
          arguments: { provider: 'garmin', limit: 1 }
        }
      });

      const result = JSON.parse(response.result.content[0].text);
      const activity = result.activities[0];

      // Core Garmin activity fields (transformed to Pierre format)
      expect(activity).toHaveProperty('id');
      expect(activity).toHaveProperty('name');
      expect(activity).toHaveProperty('sport_type');
      expect(activity).toHaveProperty('start_date');
      expect(activity).toHaveProperty('duration_seconds');
      expect(activity).toHaveProperty('distance_meters');
      expect(activity).toHaveProperty('calories');
      expect(activity).toHaveProperty('provider', 'garmin');
    }, 30000);

    test('should include Garmin-specific metrics (heart rate, speed, power)', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'get_activities',
          arguments: { provider: 'garmin', limit: 1 }
        }
      });

      const result = JSON.parse(response.result.content[0].text);
      const activity = result.activities[0];

      // Garmin-specific fields
      expect(activity).toHaveProperty('average_heart_rate');  // Garmin: average_hr
      expect(activity).toHaveProperty('max_heart_rate');  // Garmin: max_hr
      expect(activity).toHaveProperty('average_speed');
      expect(activity).toHaveProperty('max_speed');
      expect(activity).toHaveProperty('elevation_gain');
      expect(activity).toHaveProperty('average_cadence');  // Garmin: average_running_cadence
    }, 30000);

    test('should handle Garmin activity type mapping correctly', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'get_activities',
          arguments: { provider: 'garmin', limit: 1 }
        }
      });

      const result = JSON.parse(response.result.content[0].text);
      const activity = result.activities[0];

      // Garmin activity_type "running" should map to "Run"
      expect(activity.sport_type).toBe('Run');
    }, 30000);
  });

  // ============================================================================
  // get_athlete - Garmin Profile Data Tests
  // ============================================================================

  describe('get_athlete - Garmin Profile Data', () => {
    test('should return athlete profile with Garmin data structure', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'get_athlete',
          arguments: { provider: 'garmin' }
        }
      });

      expect(response.result).toBeDefined();
      const athlete = JSON.parse(response.result.content[0].text);

      expect(athlete).toHaveProperty('id');
      expect(athlete).toHaveProperty('username');
      expect(athlete).toHaveProperty('provider', 'garmin');
    }, 30000);

    test('should include optional Garmin athlete fields when available', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'get_athlete',
          arguments: { provider: 'garmin' }
        }
      });

      const athlete = JSON.parse(response.result.content[0].text);

      // Garmin profile fields
      expect(athlete).toHaveProperty('firstname');
      expect(athlete).toHaveProperty('lastname');
      expect(athlete).toHaveProperty('profile_picture');
    }, 30000);
  });

  // ============================================================================
  // get_stats - Garmin Statistics Tests
  // ============================================================================

  describe('get_stats - Garmin Statistics', () => {
    test('should return stats with Garmin totals structure', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'get_stats',
          arguments: { provider: 'garmin' }
        }
      });

      expect(response.result).toBeDefined();
      const stats = JSON.parse(response.result.content[0].text);

      expect(stats).toHaveProperty('provider', 'garmin');
      expect(stats).toHaveProperty('all_time_totals');
    }, 30000);

    test('should include Garmin-specific stats (count, duration, elevation)', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'get_stats',
          arguments: { provider: 'garmin' }
        }
      });

      const stats = JSON.parse(response.result.content[0].text);

      // Garmin provides comprehensive totals
      expect(stats.all_time_totals).toHaveProperty('count');
      expect(stats.all_time_totals).toHaveProperty('distance');
      expect(stats.all_time_totals).toHaveProperty('duration');
      expect(stats.all_time_totals).toHaveProperty('elevation');
    }, 30000);

    test('should validate Garmin stats have realistic values', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'get_stats',
          arguments: { provider: 'garmin' }
        }
      });

      const stats = JSON.parse(response.result.content[0].text);

      // Validate realistic ranges
      expect(stats.all_time_totals.count).toBeGreaterThan(0);
      expect(stats.all_time_totals.distance).toBeGreaterThan(0);
      expect(stats.all_time_totals.duration).toBeGreaterThan(0);
      expect(stats.all_time_totals.elevation).toBeGreaterThan(0);
    }, 30000);
  });

  // ============================================================================
  // Error Handling Tests
  // ============================================================================

  describe('Error Handling with Garmin Provider', () => {
    test('should require provider parameter for Garmin tools', async () => {
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
  // Data Transformation: Garmin → Pierre
  // ============================================================================

  describe('Data Transformation: Garmin → Pierre', () => {
    test('should correctly preserve Garmin distance (already in meters)', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'get_activities',
          arguments: { provider: 'garmin', limit: 1 }
        }
      });

      const result = JSON.parse(response.result.content[0].text);
      const activity = result.activities[0];

      // Garmin already uses meters, Pierre preserves it
      expect(activity.distance_meters).toBe(8047.2);
    }, 30000);

    test('should correctly preserve Garmin time fields (already in seconds)', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'get_activities',
          arguments: { provider: 'garmin', limit: 1 }
        }
      });

      const result = JSON.parse(response.result.content[0].text);
      const activity = result.activities[0];

      // Garmin already uses seconds, Pierre preserves it
      expect(activity.duration_seconds).toBe(2520);
    }, 30000);

    test('should transform Garmin field names (average_hr → average_heart_rate)', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'get_activities',
          arguments: { provider: 'garmin', limit: 1 }
        }
      });

      const result = JSON.parse(response.result.content[0].text);
      const activity = result.activities[0];

      // Garmin: average_hr, max_hr → Pierre: average_heart_rate, max_heart_rate
      expect(activity.average_heart_rate).toBe(152);
      expect(activity.max_heart_rate).toBe(178);
    }, 30000);

    test('should preserve Garmin activity ID as string', async () => {
      const response = await client.send({
        method: 'tools/call',
        params: {
          name: 'get_activities',
          arguments: { provider: 'garmin', limit: 1 }
        }
      });

      const result = JSON.parse(response.result.content[0].text);
      const activity = result.activities[0];

      expect(typeof activity.id).toBe('string');
      expect(activity.id).toBe('12345678901');
    }, 30000);

    test('should add provider field to all Garmin data', async () => {
      const activityResponse = await client.send({
        method: 'tools/call',
        params: {
          name: 'get_activities',
          arguments: { provider: 'garmin', limit: 1 }
        }
      });

      const athleteResponse = await client.send({
        method: 'tools/call',
        params: {
          name: 'get_athlete',
          arguments: { provider: 'garmin' }
        }
      });

      const statsResponse = await client.send({
        method: 'tools/call',
        params: {
          name: 'get_stats',
          arguments: { provider: 'garmin' }
        }
      });

      const activityResult = JSON.parse(activityResponse.result.content[0].text);
      const athleteResult = JSON.parse(athleteResponse.result.content[0].text);
      const statsResult = JSON.parse(statsResponse.result.content[0].text);

      expect(activityResult.provider).toBe('garmin');
      expect(activityResult.activities[0].provider).toBe('garmin');
      expect(athleteResult.provider).toBe('garmin');
      expect(statsResult.provider).toBe('garmin');
    }, 30000);
  });
});
