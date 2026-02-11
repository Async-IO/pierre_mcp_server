// ABOUTME: Common data types for Pierre MCP tools (Activity, Athlete, Stats, etc.)
// ABOUTME: Generated from server tool schemas - DO NOT EDIT MANUALLY
//
// Generated: 2026-02-11T21:11:49.166Z

/* eslint-disable @typescript-eslint/no-explicit-any */

// ============================================================================
// COMMON DATA TYPES
// ============================================================================

/**
 * Fitness activity data structure
 */
export interface Activity {
  id: string;
  name: string;
  type: string;
  distance?: number;
  duration?: number;
  moving_time?: number;
  elapsed_time?: number;
  total_elevation_gain?: number;
  start_date?: string;
  start_date_local?: string;
  timezone?: string;
  average_speed?: number;
  max_speed?: number;
  average_cadence?: number;
  average_heartrate?: number;
  max_heartrate?: number;
  average_watts?: number;
  kilojoules?: number;
  device_watts?: boolean;
  has_heartrate?: boolean;
  calories?: number;
  description?: string;
  trainer?: boolean;
  commute?: boolean;
  manual?: boolean;
  private?: boolean;
  visibility?: string;
  flagged?: boolean;
  gear_id?: string;
  from_accepted_tag?: boolean;
  upload_id?: number;
  external_id?: string;
  achievement_count?: number;
  kudos_count?: number;
  comment_count?: number;
  athlete_count?: number;
  photo_count?: number;
  map?: {
    id?: string;
    summary_polyline?: string;
    polyline?: string;
  };
  [key: string]: any;
}

/**
 * Athlete profile data structure
 */
export interface Athlete {
  id: string;
  username?: string;
  resource_state?: number;
  firstname?: string;
  lastname?: string;
  bio?: string;
  city?: string;
  state?: string;
  country?: string;
  sex?: string;
  premium?: boolean;
  summit?: boolean;
  created_at?: string;
  updated_at?: string;
  badge_type_id?: number;
  weight?: number;
  profile_medium?: string;
  profile?: string;
  friend?: any;
  follower?: any;
  ftp?: number;
  [key: string]: any;
}

/**
 * Athlete statistics data structure
 */
export interface Stats {
  biggest_ride_distance?: number;
  biggest_climb_elevation_gain?: number;
  recent_ride_totals?: ActivityTotals;
  recent_run_totals?: ActivityTotals;
  recent_swim_totals?: ActivityTotals;
  ytd_ride_totals?: ActivityTotals;
  ytd_run_totals?: ActivityTotals;
  ytd_swim_totals?: ActivityTotals;
  all_ride_totals?: ActivityTotals;
  all_run_totals?: ActivityTotals;
  all_swim_totals?: ActivityTotals;
  [key: string]: any;
}

/**
 * Activity totals for statistics
 */
export interface ActivityTotals {
  count?: number;
  distance?: number;
  moving_time?: number;
  elapsed_time?: number;
  elevation_gain?: number;
  achievement_count?: number;
}

/**
 * Fitness configuration profile
 */
export interface FitnessConfig {
  athlete_info?: {
    age?: number;
    weight?: number;
    height?: number;
    sex?: string;
    ftp?: number;
    max_heart_rate?: number;
    resting_heart_rate?: number;
    vo2_max?: number;
  };
  training_zones?: {
    heart_rate?: Zone[];
    power?: Zone[];
    pace?: Zone[];
  };
  goals?: Goal[];
  preferences?: {
    distance_unit?: string;
    weight_unit?: string;
    [key: string]: any;
  };
  [key: string]: any;
}

/**
 * Training zone definition
 */
export interface Zone {
  zone: number;
  name: string;
  min: number;
  max: number;
  description?: string;
}

/**
 * Fitness goal definition
 */
export interface Goal {
  id?: string;
  type: string;
  target_value: number;
  target_date: string;
  activity_type?: string;
  description?: string;
  progress?: number;
  status?: string;
  created_at?: string;
  updated_at?: string;
}

/**
 * Provider connection status
 */
export interface ConnectionStatus {
  provider: string;
  connected: boolean;
  last_sync?: string;
  expires_at?: string;
  scopes?: string[];
  [key: string]: any;
}

/**
 * Notification data structure
 */
export interface Notification {
  id: string;
  type: string;
  message: string;
  provider?: string;
  success?: boolean;
  created_at: string;
  read: boolean;
  [key: string]: any;
}
