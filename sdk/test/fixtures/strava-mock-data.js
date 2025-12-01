// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

/**
 * Mock Strava API Response Data
 *
 * These fixtures are based on actual Strava API v3 responses
 * Reference: https://developers.strava.com/docs/reference/
 *
 * Used for testing the Pierre MCP SDK without requiring live Strava connections
 */

/**
 * Mock Strava Athlete Profile
 * Based on Strava's getLoggedInAthlete endpoint
 */
const mockStravaAthlete = {
  id: 123456789,
  username: "pierre_test_athlete",
  resource_state: 3,
  firstname: "Pierre",
  lastname: "Testeur",
  city: "Montreal",
  state: "Quebec",
  country: "Canada",
  sex: "M",
  premium: true,
  created_at: "2023-01-15T10:30:00Z",
  updated_at: "2025-01-10T15:45:00Z",
  profile_medium: "https://example.cloudfront.net/pictures/athletes/123456789/medium.jpg",
  profile: "https://example.cloudfront.net/pictures/athletes/123456789/large.jpg",
  follower_count: 42,
  friend_count: 38,
  weight: 75.0,
  ftp: 250
};

/**
 * Mock Strava Activities
 * Based on Strava's getLoggedInAthleteActivities endpoint
 */
const mockStravaActivities = [
  {
    // EXACT Strava API v3 format for activities
    resource_state: 2,
    id: 10543210987654321,
    name: "Morning Run - Mount Royal",
    distance: 8047.2,  // meters (5 miles)
    moving_time: 2430,  // seconds (40.5 minutes)
    elapsed_time: 2520,  // seconds (42 minutes, includes stop time)
    total_elevation_gain: 125.5,  // meters
    type: "Run",  // Pierre parses this as activity_type
    sport_type: "Run",
    start_date: "2025-01-10T06:30:00Z",
    start_date_local: "2025-01-10T01:30:00Z",
    timezone: "(GMT-05:00) America/Montreal",
    start_latlng: [45.5017, -73.5673],  // Montreal [lat, lon] array
    end_latlng: [45.5045, -73.5650],
    location_city: "Montreal",  // Strava API field
    location_state: "Quebec",  // Strava API field
    location_country: "Canada",  // Strava API field
    kudos_count: 8,
    comment_count: 2,
    athlete_count: 1,
    average_speed: 3.31,  // m/s (~12 km/h, ~7:30/km pace)
    max_speed: 4.5,  // m/s
    average_heartrate: 152,  // Strava uses heartrate not heart_rate
    max_heartrate: 178,
    average_cadence: 85,  // steps/min (f32 in Pierre)
    elev_high: 245.0,
    elev_low: 120.0,
    calories: 485.0,  // f32 in Pierre's parser
    has_heartrate: true,
    heartrate_opt_out: false,
    display_hide_heartrate_option: true,
    workout_type: 1,  // Race
    suffer_score: 87  // Strava's relative effort
  },
  {
    // EXACT Strava API v3 format for ride with power data
    resource_state: 2,
    id: 10543210987654322,
    name: "Afternoon Ride - Lachine Canal",
    distance: 32186.5,  // meters (20 miles)
    moving_time: 4320,  // seconds (72 minutes)
    elapsed_time: 4500,  // seconds (75 minutes)
    total_elevation_gain: 45.0,  // meters
    type: "Ride",
    sport_type: "Ride",
    start_date: "2025-01-09T14:15:00Z",
    start_date_local: "2025-01-09T09:15:00Z",
    timezone: "(GMT-05:00) America/Montreal",
    start_latlng: [45.4642, -73.6084],  // Lachine [lat, lon] array
    end_latlng: [45.4650, -73.6090],
    location_city: "Lachine",  // Strava API field
    location_state: "Quebec",  // Strava API field
    location_country: "Canada",  // Strava API field
    kudos_count: 12,
    comment_count: 3,
    athlete_count: 1,
    average_speed: 7.45,  // m/s (~27 km/h)
    max_speed: 12.5,  // m/s
    average_heartrate: 138,  // Strava uses heartrate not heart_rate
    max_heartrate: 165,
    average_cadence: 82.0,  // RPM (f32 in Pierre)
    average_watts: 185.5,  // f32 in Pierre
    weighted_average_watts: 195.2,  // Normalized power (Strava specific)
    max_watts: 420,  // u32 in Pierre
    device_watts: true,  // Power from actual power meter
    kilojoules: 820.3,  // f32 in Pierre
    elev_high: 45.0,
    elev_low: 22.0,
    calories: 925.0,  // f32 in Pierre's parser
    has_heartrate: true,
    heartrate_opt_out: false,
    display_hide_heartrate_option: true,
    workout_type: 0,  // Default
    suffer_score: 112  // Strava's relative effort
  },
  {
    // EXACT Strava API v3 format for swim (pool - no GPS)
    resource_state: 2,
    id: 10543210987654323,
    name: "Easy Recovery Swim",
    distance: 1500.0,  // meters
    moving_time: 1800,  // seconds (30 minutes)
    elapsed_time: 1920,  // seconds (32 minutes)
    total_elevation_gain: 0,
    type: "Swim",
    sport_type: "Swim",
    start_date: "2025-01-08T17:00:00Z",
    start_date_local: "2025-01-08T12:00:00Z",
    timezone: "(GMT-05:00) America/Montreal",
    start_latlng: null,  // Pool swim - no GPS
    end_latlng: null,
    location_city: "Montreal",  // Strava API field
    location_state: "Quebec",  // Strava API field
    location_country: "Canada",  // Strava API field
    kudos_count: 5,
    comment_count: 1,
    athlete_count: 1,
    average_speed: 0.83,  // m/s (~2:00/100m pace)
    max_speed: 1.1,  // m/s
    calories: 285.0,  // f32 in Pierre's parser
    workout_type: 3  // Workout
  }
];

/**
 * Mock Strava Stats
 * Based on Strava's getStats endpoint
 */
const mockStravaStats = {
  biggest_ride_distance: 98234.5,  // meters
  biggest_climb_elevation_gain: 1234.5,  // meters
  recent_ride_totals: {
    count: 8,
    distance: 256847.3,  // meters
    moving_time: 32400,  // seconds (9 hours)
    elapsed_time: 34200,  // seconds
    elevation_gain: 1850.5,  // meters
    achievement_count: 15
  },
  recent_run_totals: {
    count: 12,
    distance: 96234.8,  // meters
    moving_time: 28800,  // seconds (8 hours)
    elapsed_time: 30000,  // seconds
    elevation_gain: 1245.0,  // meters
    achievement_count: 23
  },
  recent_swim_totals: {
    count: 5,
    distance: 7500.0,  // meters
    moving_time: 9000,  // seconds (2.5 hours)
    elapsed_time: 9600,  // seconds
    elevation_gain: 0,
    achievement_count: 3
  },
  ytd_ride_totals: {
    count: 42,
    distance: 1256847.3,  // meters
    moving_time: 162000,  // seconds (45 hours)
    elapsed_time: 171000,  // seconds
    elevation_gain: 12850.5,  // meters
    achievement_count: 87
  },
  ytd_run_totals: {
    count: 68,
    distance: 548234.8,  // meters
    moving_time: 165600,  // seconds (46 hours)
    elapsed_time: 172800,  // seconds
    elevation_gain: 6245.0,  // meters
    achievement_count: 142
  },
  ytd_swim_totals: {
    count: 18,
    distance: 27500.0,  // meters
    moving_time: 36000,  // seconds (10 hours)
    elapsed_time: 38400,  // seconds
    elevation_gain: 0,
    achievement_count: 12
  },
  all_ride_totals: {
    count: 342,
    distance: 9256847.3,  // meters
    moving_time: 1162000,  // seconds (322 hours)
    elapsed_time: 1224000,  // seconds
    elevation_gain: 92850.5,  // meters
    achievement_count: 687
  },
  all_run_totals: {
    count: 568,
    distance: 4548234.8,  // meters
    moving_time: 1365600,  // seconds (379 hours)
    elapsed_time: 1425600,  // seconds
    elevation_gain: 46245.0,  // meters
    achievement_count: 1142
  },
  all_swim_totals: {
    count: 98,
    distance: 147500.0,  // meters
    moving_time: 198000,  // seconds (55 hours)
    elapsed_time: 211200,  // seconds
    elevation_gain: 0,
    achievement_count: 62
  }
};

/**
 * Mock Pierre-Transformed Activity
 * This is what the Pierre server returns after processing Strava data
 * Transformation: StravaActivity (JSON) → Activity (Pierre model)
 */
const mockPierreActivity = {
  id: "10543210987654321",
  name: "Morning Run - Mount Royal",
  sport_type: "Run",
  start_date: "2025-01-10T06:30:00Z",
  duration_seconds: 2520,  // Pierre uses elapsed_time from Strava
  distance_meters: 8047.2,
  elevation_gain: 125.5,
  average_heart_rate: 152,  // Pierre converts average_heartrate → average_heart_rate
  max_heart_rate: 178,  // Pierre converts max_heartrate → max_heart_rate
  average_speed: 3.31,
  max_speed: 4.5,
  calories: 485,
  steps: null,
  heart_rate_zones: null,
  average_power: null,
  max_power: null,
  normalized_power: null,
  power_zones: null,
  ftp: null,
  average_cadence: 85,  // Pierre converts f32 to u32
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
  suffer_score: 87,  // Pierre includes Strava's suffer_score
  time_series_data: null,
  start_latitude: 45.5017,  // Pierre extracts from start_latlng[0]
  start_longitude: -73.5673,  // Pierre extracts from start_latlng[1]
  city: "Montreal",  // Pierre uses location_city
  region: "Quebec",  // Pierre uses location_state as region
  country: "Canada",  // Pierre uses location_country
  trail_name: null,
  segment_efforts: null,
  provider: "strava"
};

/**
 * Mock Pierre-Transformed Athlete Profile
 */
const mockPierreAthlete = {
  id: "123456789",
  username: "pierre_test_athlete",
  firstname: "Pierre",
  lastname: "Testeur",
  city: "Montreal",
  state: "Quebec",
  country: "Canada",
  sex: "M",
  weight: 75.0,
  ftp: 250,
  profile_picture: "https://example.cloudfront.net/pictures/athletes/123456789/large.jpg",
  created_at: "2023-01-15T10:30:00Z",
  provider: "strava"
};

/**
 * Mock Pierre-Transformed Stats
 */
const mockPierreStats = {
  recent_totals: {
    runs: {
      count: 12,
      distance: 96234.8,
      duration: 28800,
      elevation: 1245.0
    },
    rides: {
      count: 8,
      distance: 256847.3,
      duration: 32400,
      elevation: 1850.5
    },
    swims: {
      count: 5,
      distance: 7500.0,
      duration: 9000,
      elevation: 0
    }
  },
  ytd_totals: {
    runs: {
      count: 68,
      distance: 548234.8,
      duration: 165600,
      elevation: 6245.0
    },
    rides: {
      count: 42,
      distance: 1256847.3,
      duration: 162000,
      elevation: 12850.5
    },
    swims: {
      count: 18,
      distance: 27500.0,
      duration: 36000,
      elevation: 0
    }
  },
  all_time_totals: {
    runs: {
      count: 568,
      distance: 4548234.8,
      duration: 1365600,
      elevation: 46245.0
    },
    rides: {
      count: 342,
      distance: 9256847.3,
      duration: 1162000,
      elevation: 92850.5
    },
    swims: {
      count: 98,
      distance: 147500.0,
      duration: 198000,
      elevation: 0
    }
  },
  biggest_ride_distance: 98234.5,
  biggest_climb_elevation: 1234.5,
  provider: "strava"
};

/**
 * Mock MCP Tool Response for get_activities
 */
const mockMcpActivitiesResponse = {
  content: [
    {
      type: "text",
      text: JSON.stringify({
        activities: [mockPierreActivity],
        count: 1,
        provider: "strava"
      }, null, 2)
    }
  ]
};

/**
 * Mock MCP Tool Response for get_athlete
 */
const mockMcpAthleteResponse = {
  content: [
    {
      type: "text",
      text: JSON.stringify(mockPierreAthlete, null, 2)
    }
  ]
};

/**
 * Mock MCP Tool Response for get_stats
 */
const mockMcpStatsResponse = {
  content: [
    {
      type: "text",
      text: JSON.stringify(mockPierreStats, null, 2)
    }
  ]
};

module.exports = {
  mockStravaAthlete,
  mockStravaActivities,
  mockStravaStats,
  mockPierreActivity,
  mockPierreAthlete,
  mockPierreStats,
  mockMcpActivitiesResponse,
  mockMcpAthleteResponse,
  mockMcpStatsResponse
};
