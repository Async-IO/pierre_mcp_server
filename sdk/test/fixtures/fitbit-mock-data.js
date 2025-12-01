// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

/**
 * Mock Fitbit API v1 Response Data
 *
 * These fixtures are based on actual Fitbit Web API v1 responses
 * Reference: https://dev.fitbit.com/build/reference/web-api/activity/
 *
 * Used for testing the Pierre MCP SDK without requiring live Fitbit connections
 */

/**
 * Mock Fitbit Activity Log List Response
 * Based on Fitbit's Get Activity Log List endpoint
 * https://dev.fitbit.com/build/reference/web-api/activity/get-activity-log-list/
 */
const mockFitbitActivities = [
  {
    // EXACT Fitbit API v1 format - Running activity
    activityId: 987654321,
    activityName: "Morning Run",
    activityTypeId: 90009,  // Fitbit Running type ID
    startTime: "2025-01-10T06:30:00.000",  // Fitbit format without timezone
    duration: 2520000,  // milliseconds (42 minutes)
    distance: 8.0472,  // kilometers
    steps: 10500,
    calories: 485,
    elevationGain: 125.5,  // meters
    averageHeartRate: 152,
    heartRateZones: [
      {
        name: "Out of Range",
        min: 30,
        max: 94,
        minutes: 0
      },
      {
        name: "Fat Burn",
        min: 94,
        max: 132,
        minutes: 8
      },
      {
        name: "Cardio",
        min: 132,
        max: 160,
        minutes: 28
      },
      {
        name: "Peak",
        min: 160,
        max: 220,
        minutes: 6
      }
    ]
  },
  {
    // EXACT Fitbit API v1 format - Biking activity with detailed metrics
    activityId: 987654322,
    activityName: "Afternoon Bike Ride",
    activityTypeId: 1071,  // Fitbit Biking type ID
    startTime: "2025-01-09T14:15:00.000",
    duration: 4500000,  // milliseconds (75 minutes)
    distance: 32.1865,  // kilometers
    steps: null,  // Bikes don't have steps
    calories: 925,
    elevationGain: 45.0,  // meters
    averageHeartRate: 138,
    heartRateZones: [
      {
        name: "Out of Range",
        min: 30,
        max: 94,
        minutes: 0
      },
      {
        name: "Fat Burn",
        min: 94,
        max: 132,
        minutes: 25
      },
      {
        name: "Cardio",
        min: 132,
        max: 160,
        minutes: 45
      },
      {
        name: "Peak",
        min: 160,
        max: 220,
        minutes: 5
      }
    ]
  },
  {
    // EXACT Fitbit API v1 format - Swimming activity (no GPS)
    activityId: 987654323,
    activityName: "Pool Swim",
    activityTypeId: 90024,  // Fitbit Swimming type ID
    startTime: "2025-01-08T17:00:00.000",
    duration: 1920000,  // milliseconds (32 minutes)
    distance: 1.5,  // kilometers
    steps: null,  // Swimming doesn't have steps
    calories: 285,
    elevationGain: null,  // No elevation in pool
    averageHeartRate: 125,
    heartRateZones: [
      {
        name: "Out of Range",
        min: 30,
        max: 94,
        minutes: 0
      },
      {
        name: "Fat Burn",
        min: 94,
        max: 132,
        minutes: 28
      },
      {
        name: "Cardio",
        min: 132,
        max: 160,
        minutes: 4
      },
      {
        name: "Peak",
        min: 160,
        max: 220,
        minutes: 0
      }
    ]
  }
];

/**
 * Mock Fitbit User Profile
 * Based on Fitbit's Get Profile endpoint
 * https://dev.fitbit.com/build/reference/web-api/user/get-profile/
 */
const mockFitbitAthlete = {
  user: {
    encodedId: "ABC123",
    displayName: "Pierre Runner",
    firstName: "Pierre",
    lastName: "Runner",
    avatar: "https://static0.fitbit.com/images/profile/defaultProfile_100_male.gif"
  }
};

/**
 * Mock Fitbit Lifetime Stats
 * Based on Fitbit's Get Lifetime Stats endpoint
 * https://dev.fitbit.com/build/reference/web-api/activity/get-lifetime-stats/
 */
const mockFitbitStats = {
  lifetime: {
    total: {
      distance: 9256.8473,  // kilometers
      floors: 8542  // floors climbed
    }
  }
};

/**
 * Mock Pierre-Transformed Fitbit Activity
 * This is what the Pierre server returns after processing Fitbit data
 * Transformation: FitbitActivity (JSON) → Activity (Pierre model)
 */
const mockPierreFitbitActivity = {
  id: "987654321",
  name: "Morning Run",
  sport_type: "Run",  // Pierre maps activity_type_id 90009 → Run
  start_date: "2025-01-10T06:30:00Z",
  duration_seconds: 2520,  // Pierre converts ms to seconds (2520000 / 1000)
  distance_meters: 8047.2,  // Pierre converts km to meters (8.0472 * 1000)
  elevation_gain: 125.5,
  average_heart_rate: 152,  // Pierre uses average_heart_rate from averageHeartRate
  max_heart_rate: null,  // Fitbit doesn't provide max HR in activity log
  average_speed: 3.195,  // Pierre calculates from distance/duration
  max_speed: null,  // Fitbit doesn't provide in activity log
  calories: 485,
  steps: 10500,
  heart_rate_zones: [
    { name: "Out of Range", min: 30, max: 94, minutes: 0 },
    { name: "Fat Burn", min: 94, max: 132, minutes: 8 },
    { name: "Cardio", min: 132, max: 160, minutes: 28 },
    { name: "Peak", min: 160, max: 220, minutes: 6 }
  ],
  average_power: null,
  max_power: null,
  normalized_power: null,
  power_zones: null,
  ftp: null,
  average_cadence: null,  // Fitbit doesn't provide cadence in basic activity log
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
  start_latitude: null,  // Fitbit activity log doesn't include GPS coordinates
  start_longitude: null,
  city: null,
  region: null,
  country: null,
  trail_name: null,
  segment_efforts: null,
  provider: "fitbit"
};

/**
 * Mock Pierre-Transformed Fitbit Athlete Profile
 */
const mockPierreFitbitAthlete = {
  id: "ABC123",
  username: "Pierre Runner",
  firstname: "Pierre",
  lastname: "Runner",
  city: null,
  state: null,
  country: null,
  sex: null,
  weight: null,
  ftp: null,
  profile_picture: "https://static0.fitbit.com/images/profile/defaultProfile_100_male.gif",
  created_at: null,
  provider: "fitbit"
};

/**
 * Mock Pierre-Transformed Fitbit Stats
 */
const mockPierreFitbitStats = {
  recent_totals: null,  // Fitbit doesn't provide recent period in lifetime stats
  ytd_totals: null,  // Fitbit doesn't provide year-to-date in lifetime stats
  all_time_totals: {
    runs: null,
    rides: null,
    swims: null,
    distance: 9256847.3,  // Pierre converts km to meters (9256.8473 * 1000)
    elevation: null,  // Fitbit reports floors, not direct elevation
    floors_climbed: 8542
  },
  biggest_ride_distance: null,
  biggest_climb_elevation: null,
  provider: "fitbit"
};

/**
 * Mock MCP Tool Response for get_activities (Fitbit)
 */
const mockMcpFitbitActivitiesResponse = {
  content: [
    {
      type: "text",
      text: JSON.stringify({
        activities: [mockPierreFitbitActivity],
        count: 1,
        provider: "fitbit"
      }, null, 2)
    }
  ]
};

/**
 * Mock MCP Tool Response for get_athlete (Fitbit)
 */
const mockMcpFitbitAthleteResponse = {
  content: [
    {
      type: "text",
      text: JSON.stringify(mockPierreFitbitAthlete, null, 2)
    }
  ]
};

/**
 * Mock MCP Tool Response for get_stats (Fitbit)
 */
const mockMcpFitbitStatsResponse = {
  content: [
    {
      type: "text",
      text: JSON.stringify(mockPierreFitbitStats, null, 2)
    }
  ]
};

module.exports = {
  mockFitbitActivities,
  mockFitbitAthlete,
  mockFitbitStats,
  mockPierreFitbitActivity,
  mockPierreFitbitAthlete,
  mockPierreFitbitStats,
  mockMcpFitbitActivitiesResponse,
  mockMcpFitbitAthleteResponse,
  mockMcpFitbitStatsResponse
};
