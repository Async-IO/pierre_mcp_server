// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

/**
 * Mock Garmin Connect API Response Data
 *
 * These fixtures are based on Garmin Connect API responses
 * Reference: https://developer.garmin.com/gc-developer-program/activity-api/
 *
 * Used for testing the Pierre MCP SDK without requiring live Garmin connections
 */

/**
 * Mock Garmin Activity Response
 * Based on Garmin Connect API activity format
 * Garmin uses snake_case for most field names
 */
const mockGarminActivities = [
  {
    // EXACT Garmin API format - Running activity
    activity_id: 12345678901,
    activity_name: "Morning Trail Run",
    activity_type: "running",
    start_time_gmt: "2025-01-10T06:30:00.000Z",  // Garmin uses ISO 8601 with GMT
    distance: 8047.2,  // meters
    duration: 2520.0,  // seconds (floating point)
    elevation_gain: 125.5,  // meters
    average_speed: 3.195,  // m/s
    max_speed: 4.5,  // m/s
    average_hr: 152,  // Garmin uses 'hr' not 'heart_rate'
    max_hr: 178,
    average_running_cadence: 85.0,  // steps per minute (Garmin specific)
    average_power: null,  // Running power (optional)
    max_power: null,
    calories: 485.0
  },
  {
    // EXACT Garmin API format - Cycling activity with power data
    activity_id: 12345678902,
    activity_name: "Afternoon Bike Ride",
    activity_type: "cycling",
    start_time_gmt: "2025-01-09T14:15:00.000Z",
    distance: 32186.5,  // meters
    duration: 4500.0,  // seconds
    elevation_gain: 45.0,  // meters
    average_speed: 7.15,  // m/s (~25.7 km/h)
    max_speed: 12.5,  // m/s
    average_hr: 138,
    max_hr: 165,
    average_running_cadence: 82.0,  // RPM for cycling (Garmin uses same field)
    average_power: 185.5,  // watts
    max_power: 420.0,  // watts
    calories: 925.0
  },
  {
    // EXACT Garmin API format - Swimming activity
    activity_id: 12345678903,
    activity_name: "Recovery Swim",
    activity_type: "swimming",
    start_time_gmt: "2025-01-08T17:00:00.000Z",
    distance: 1500.0,  // meters
    duration: 1920.0,  // seconds
    elevation_gain: 0.0,  // No elevation in pool
    average_speed: 0.78,  // m/s (~2:08/100m pace)
    max_speed: 1.1,  // m/s
    average_hr: 125,
    max_hr: 145,
    average_running_cadence: null,  // Not applicable for swimming
    average_power: null,
    max_power: null,
    calories: 285.0
  }
];

/**
 * Mock Garmin Athlete Profile
 * Based on Garmin Connect API user profile format
 */
const mockGarminAthlete = {
  user_id: "garmin-user-123",
  display_name: "Pierre Athlete",
  full_name: "Pierre G. Athlete",
  profile_image_url: "https://s3.amazonaws.com/garmin-connect-prod/profile_images/abc123.jpg"
};

/**
 * Mock Garmin Stats Response
 * Based on Garmin Connect API stats format
 * Garmin uses camelCase for stats fields
 */
const mockGarminStats = {
  totalActivities: 342,
  totalDistance: 9256847.3,  // meters
  totalDuration: 1162000.0,  // seconds
  totalElevationGain: 92850.5  // meters
};

/**
 * Mock Pierre-Transformed Garmin Activity
 * This is what the Pierre server returns after processing Garmin data
 * Transformation: GarminActivityResponse (JSON) → Activity (Pierre model)
 */
const mockPierreGarminActivity = {
  id: "12345678901",
  name: "Morning Trail Run",
  sport_type: "Run",  // Pierre maps activity_type "running" → Run
  start_date: "2025-01-10T06:30:00Z",
  duration_seconds: 2520,  // Pierre uses Garmin's duration directly (already in seconds)
  distance_meters: 8047.2,  // Pierre uses Garmin's distance directly (already in meters)
  elevation_gain: 125.5,
  average_heart_rate: 152,  // Pierre converts average_hr → average_heart_rate
  max_heart_rate: 178,  // Pierre converts max_hr → max_heart_rate
  average_speed: 3.195,
  max_speed: 4.5,
  calories: 485,
  steps: null,  // Garmin activity API doesn't include steps in this format
  heart_rate_zones: null,  // Garmin activity summary doesn't include HR zones
  average_power: null,  // This run didn't have power data
  max_power: null,
  normalized_power: null,
  power_zones: null,
  ftp: null,
  average_cadence: 85,  // Pierre converts average_running_cadence → average_cadence
  max_cadence: null,
  hrv_score: null,
  recovery_heart_rate: null,
  temperature: null,
  humidity: null,
  average_altitude: null,
  wind_speed: null,
  ground_contact_time: null,
  vertical_oscillation: null,
  stride_length: null,
  running_power: null,
  breathing_rate: null,
  spo2: null,
  training_stress_score: null,
  intensity_factor: null,
  suffer_score: null,
  time_series_data: null,
  start_latitude: null,  // Garmin activity summary doesn't include GPS coordinates
  start_longitude: null,
  city: null,
  region: null,
  country: null,
  trail_name: null,
  segment_efforts: null,
  provider: "garmin"
};

/**
 * Mock Pierre-Transformed Garmin Athlete Profile
 */
const mockPierreGarminAthlete = {
  id: "garmin-user-123",
  username: "Pierre Athlete",
  firstname: "Pierre G.",
  lastname: "Athlete",
  city: null,
  state: null,
  country: null,
  sex: null,
  weight: null,
  ftp: null,
  profile_picture: "https://s3.amazonaws.com/garmin-connect-prod/profile_images/abc123.jpg",
  created_at: null,
  provider: "garmin"
};

/**
 * Mock Pierre-Transformed Garmin Stats
 */
const mockPierreGarminStats = {
  recent_totals: null,  // Garmin summary doesn't provide recent period breakdown
  ytd_totals: null,  // Garmin summary doesn't provide year-to-date breakdown
  all_time_totals: {
    runs: null,
    rides: null,
    swims: null,
    count: 342,
    distance: 9256847.3,  // meters (already in meters from Garmin)
    duration: 1162000,  // seconds
    elevation: 92850.5  // meters
  },
  biggest_ride_distance: null,
  biggest_climb_elevation: null,
  provider: "garmin"
};

/**
 * Mock MCP Tool Response for get_activities (Garmin)
 */
const mockMcpGarminActivitiesResponse = {
  content: [
    {
      type: "text",
      text: JSON.stringify({
        activities: [mockPierreGarminActivity],
        count: 1,
        provider: "garmin"
      }, null, 2)
    }
  ]
};

/**
 * Mock MCP Tool Response for get_athlete (Garmin)
 */
const mockMcpGarminAthleteResponse = {
  content: [
    {
      type: "text",
      text: JSON.stringify(mockPierreGarminAthlete, null, 2)
    }
  ]
};

/**
 * Mock MCP Tool Response for get_stats (Garmin)
 */
const mockMcpGarminStatsResponse = {
  content: [
    {
      type: "text",
      text: JSON.stringify(mockPierreGarminStats, null, 2)
    }
  ]
};

module.exports = {
  mockGarminActivities,
  mockGarminAthlete,
  mockGarminStats,
  mockPierreGarminActivity,
  mockPierreGarminAthlete,
  mockPierreGarminStats,
  mockMcpGarminActivitiesResponse,
  mockMcpGarminAthleteResponse,
  mockMcpGarminStatsResponse
};
