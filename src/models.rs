// ABOUTME: Core data models and types for the Pierre fitness API
// ABOUTME: Defines Activity, User, SportType and other fundamental data structures
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! # Data Models
//!
// NOTE: All `.clone()` calls in this file are Safe - they are necessary for:
// - HashMap key ownership for statistics aggregation (stage_type.clone())
// - Data structure ownership transfers across model boundaries
//!
//! This module contains the core data structures used throughout the Pierre MCP Server.
//! These models provide a unified representation of fitness data from various providers
//! like Strava and Fitbit.
//!
//! ## Design Principles
//!
//! - **Provider Agnostic**: Models abstract away provider-specific differences
//! - **Extensible**: Optional fields accommodate different provider capabilities
//! - **Serializable**: All models support JSON serialization for MCP protocol
//! - **Type Safe**: Strong typing prevents common data handling errors
//!
//! ## Core Models
//!
//! - `Activity`: Represents a single fitness activity (run, ride, etc.)
//! - `Athlete`: User profile information
//! - `Stats`: Aggregated fitness statistics
//! - `PersonalRecord`: Individual performance records
//! - `SportType`: Enumeration of supported activity types

use crate::constants::tiers;
use crate::errors::AppError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Heart rate zone data for an activity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartRateZone {
    /// Zone name (e.g., "Zone 1", "Fat Burn", "Cardio")
    pub name: String,
    /// Minimum heart rate for this zone in BPM
    pub min_hr: u32,
    /// Maximum heart rate for this zone in BPM
    pub max_hr: u32,
    /// Minutes spent in this zone during the activity
    pub minutes: u32,
}

/// Power zone data for cycling activities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerZone {
    /// Zone name (e.g., "Zone 1", "Active Recovery", "Threshold")
    pub name: String,
    /// Minimum power for this zone in watts
    pub min_power: u32,
    /// Maximum power for this zone in watts
    pub max_power: u32,
    /// Time spent in this zone in seconds
    pub time_in_zone: u32,
}

/// Sleep stage data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SleepStage {
    /// Stage type (awake, light, deep, rem)
    pub stage_type: SleepStageType,
    /// Start time of this stage
    pub start_time: DateTime<Utc>,
    /// Duration of this stage in minutes
    pub duration_minutes: u32,
}

/// Types of sleep stages
#[non_exhaustive]
#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SleepStageType {
    /// Awake stage - user is conscious and alert
    Awake,
    /// Light sleep stage - easy to wake from, body relaxing
    Light,
    /// Deep sleep stage - restorative, hard to wake from
    Deep,
    /// REM (Rapid Eye Movement) sleep stage - dreaming, memory consolidation
    Rem,
}

/// Time-series data for detailed activity analysis
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimeSeriesData {
    /// Time offsets from activity start in seconds
    pub timestamps: Vec<u32>,
    /// Heart rate measurements (BPM)
    pub heart_rate: Option<Vec<u32>>,
    /// Power measurements (watts)
    pub power: Option<Vec<u32>>,
    /// Cadence measurements (RPM or steps/min)
    pub cadence: Option<Vec<u32>>,
    /// Speed measurements (m/s)
    pub speed: Option<Vec<f32>>,
    /// Altitude measurements (meters)
    pub altitude: Option<Vec<f32>>,
    /// Temperature measurements (Celsius)
    pub temperature: Option<Vec<f32>>,
    /// GPS coordinates (lat, lon pairs)
    pub gps_coordinates: Option<Vec<(f64, f64)>>,
}

/// Segment effort within an activity (primarily from Strava)
/// Represents performance on a known route/segment during an activity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SegmentEffort {
    /// Unique identifier for the segment effort
    pub id: String,
    /// Name of the segment
    pub name: String,
    /// Elapsed time on segment in seconds
    pub elapsed_time: u64,
    /// Moving time on segment in seconds (excludes stopped time)
    pub moving_time: Option<u64>,
    /// When the segment effort started (UTC)
    pub start_date: DateTime<Utc>,
    /// Distance of the segment in meters
    pub distance: f64,
    /// Average heart rate during segment (BPM)
    pub average_heart_rate: Option<u32>,
    /// Max heart rate during segment (BPM)
    pub max_heart_rate: Option<u32>,
    /// Average cadence during segment
    pub average_cadence: Option<u32>,
    /// Average power during segment (watts)
    pub average_watts: Option<u32>,
    /// King of the Mountain (KOM) rank for this effort (1 = fastest ever)
    pub kom_rank: Option<u32>,
    /// Personal Record (PR) rank for this athlete (1 = athlete's best)
    pub pr_rank: Option<u32>,
    /// Segment climb category (HC, 1-4, or None)
    pub climb_category: Option<u32>,
    /// Average grade/gradient of the segment (percentage)
    pub average_grade: Option<f32>,
    /// Elevation gain on the segment in meters
    pub elevation_gain: Option<f64>,
}

/// User tier for rate limiting - same as `API` key tiers for consistency
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UserTier {
    /// Free tier with basic limits
    Starter,
    /// Professional tier with higher limits
    Professional,
    /// Enterprise tier with unlimited access
    Enterprise,
}

impl std::fmt::Display for UserTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Starter => write!(f, "Starter"),
            Self::Professional => write!(f, "Professional"),
            Self::Enterprise => write!(f, "Enterprise"),
        }
    }
}

/// User account status for admin approval workflow
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum UserStatus {
    /// Account pending admin approval (new registrations)
    #[default]
    Pending,
    /// Account approved and active
    Active,
    /// Account suspended by admin
    Suspended,
}

impl UserStatus {
    /// Check if user can login
    #[must_use]
    pub const fn can_login(&self) -> bool {
        matches!(self, Self::Active)
    }

    /// Get user-friendly status message
    #[must_use]
    pub const fn to_message(&self) -> &'static str {
        match self {
            Self::Pending => "Your account is pending admin approval",
            Self::Active => "Account is active",
            Self::Suspended => "Your account has been suspended",
        }
    }
}

impl std::fmt::Display for UserStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Active => write!(f, "active"),
            Self::Suspended => write!(f, "suspended"),
        }
    }
}

impl UserTier {
    /// Get monthly request limit for this tier
    #[must_use]
    pub const fn monthly_limit(&self) -> Option<u32> {
        match self {
            Self::Starter => Some(10_000),
            Self::Professional => Some(100_000),
            Self::Enterprise => None, // Unlimited
        }
    }

    /// Get display name for this tier
    #[must_use]
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Starter => "Starter",
            Self::Professional => "Professional",
            Self::Enterprise => "Enterprise",
        }
    }

    /// Convert to string for database storage
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Starter => tiers::STARTER,
            Self::Professional => tiers::PROFESSIONAL,
            Self::Enterprise => tiers::ENTERPRISE,
        }
    }
}

impl std::str::FromStr for UserTier {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            tiers::STARTER => Ok(Self::Starter),
            tiers::PROFESSIONAL => Ok(Self::Professional),
            tiers::ENTERPRISE => Ok(Self::Enterprise),
            _ => Err(AppError::invalid_input(format!("Invalid user tier: {s}")).into()),
        }
    }
}

/// Represents a single fitness activity from any provider
///
/// An activity contains all the essential information about a workout,
/// including timing, distance, performance metrics, and metadata.
///
/// # Examples
///
/// ```rust
/// use pierre_mcp_server::models::{Activity, SportType};
/// use chrono::Utc;
///
/// let activity = Activity {
///     id: "12345".into(),
///     name: "Morning Run".into(),
///     sport_type: SportType::Run,
///     start_date: Utc::now(),
///     duration_seconds: 1800, // 30 minutes
///     distance_meters: Some(5000.0), // 5km
///     elevation_gain: Some(100.0),
///     average_heart_rate: Some(150),
///     max_heart_rate: Some(175),
///     average_speed: Some(2.78), // m/s
///     max_speed: Some(4.17), // m/s
///     calories: Some(300),
///     steps: None,
///     heart_rate_zones: None,
///     average_power: None,
///     max_power: None,
///     normalized_power: None,
///     power_zones: None,
///     ftp: None,
///     average_cadence: None,
///     max_cadence: None,
///     hrv_score: None,
///     recovery_heart_rate: None,
///     temperature: None,
///     humidity: None,
///     average_altitude: None,
///     wind_speed: None,
///     ground_contact_time: None,
///     vertical_oscillation: None,
///     stride_length: None,
///     running_power: None,
///     breathing_rate: None,
///     spo2: None,
///     training_stress_score: None,
///     intensity_factor: None,
///     suffer_score: None,
///     time_series_data: None,
///     start_latitude: Some(45.5017), // Montreal
///     start_longitude: Some(-73.5673),
///     city: Some("Montreal".into()),
///     region: Some("Quebec".into()),
///     country: Some("Canada".into()),
///     trail_name: Some("Mount Royal Trail".into()),
///     provider: "strava".into(),
///     workout_type: None,
///     sport_type_detail: None,
///     segment_efforts: None,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    /// Unique identifier for the activity (provider-specific)
    pub id: String,
    /// Human-readable name/title of the activity
    pub name: String,
    /// Type of sport/activity (run, ride, swim, etc.)
    pub sport_type: SportType,
    /// When the activity started (UTC)
    pub start_date: DateTime<Utc>,
    /// Total duration of the activity in seconds
    pub duration_seconds: u64,
    /// Total distance covered in meters (if applicable)
    pub distance_meters: Option<f64>,
    /// Total elevation gained in meters (if available)
    pub elevation_gain: Option<f64>,
    /// Average heart rate during the activity (BPM)
    pub average_heart_rate: Option<u32>,
    /// Maximum heart rate reached during the activity (BPM)
    pub max_heart_rate: Option<u32>,
    /// Average speed in meters per second
    pub average_speed: Option<f64>,
    /// Maximum speed reached in meters per second
    pub max_speed: Option<f64>,
    /// Estimated calories burned during the activity
    pub calories: Option<u32>,
    /// Total steps taken during the activity (for walking/running activities)
    pub steps: Option<u32>,
    /// Heart rate zone data if available from the provider
    pub heart_rate_zones: Option<Vec<HeartRateZone>>,

    // Advanced Power Metrics
    /// Average power output in watts (cycling/rowing)
    pub average_power: Option<u32>,
    /// Maximum power output reached in watts
    pub max_power: Option<u32>,
    /// Normalized power (power adjusted for variability)
    pub normalized_power: Option<u32>,
    /// Power zone distribution
    pub power_zones: Option<Vec<PowerZone>>,
    /// Functional Threshold Power at time of activity
    pub ftp: Option<u32>,

    // Cadence Metrics
    /// Average cadence (RPM for cycling, steps/min for running)
    pub average_cadence: Option<u32>,
    /// Maximum cadence reached
    pub max_cadence: Option<u32>,

    // Advanced Heart Rate Metrics
    /// Heart Rate Variability score during activity
    pub hrv_score: Option<f64>,
    /// Heart rate recovery (drop in first minute after activity)
    pub recovery_heart_rate: Option<u32>,

    // Environmental Conditions
    /// Temperature during activity (Celsius)
    pub temperature: Option<f32>,
    /// Humidity percentage during activity
    pub humidity: Option<f32>,
    /// Average altitude during activity (meters)
    pub average_altitude: Option<f32>,
    /// Wind speed during activity (m/s)
    pub wind_speed: Option<f32>,

    // Biomechanical Metrics (Running)
    /// Ground contact time in milliseconds
    pub ground_contact_time: Option<u32>,
    /// Vertical oscillation in centimeters
    pub vertical_oscillation: Option<f32>,
    /// Average stride length in meters
    pub stride_length: Option<f32>,
    /// Running power (estimated or measured)
    pub running_power: Option<u32>,

    // Respiratory and Oxygen Metrics
    /// Average breathing rate (breaths per minute)
    pub breathing_rate: Option<u32>,
    /// Blood oxygen saturation percentage
    pub spo2: Option<f32>,

    // Training Load and Performance
    /// Training Stress Score for this activity
    pub training_stress_score: Option<f32>,
    /// Intensity Factor (normalized intensity vs threshold)
    pub intensity_factor: Option<f32>,
    /// Suffer score or relative effort rating
    pub suffer_score: Option<u32>,

    // Detailed Time-Series Data
    /// Time-series data for advanced analysis
    pub time_series_data: Option<TimeSeriesData>,
    /// Starting latitude coordinate (if available)
    pub start_latitude: Option<f64>,
    /// Starting longitude coordinate (if available)
    pub start_longitude: Option<f64>,
    /// Location information extracted from GPS coordinates
    pub city: Option<String>,
    /// Region/state/province where the activity took place
    pub region: Option<String>,
    /// Country where the activity took place
    pub country: Option<String>,
    /// Trail or route name if available (e.g., "Saint-Hippolyte trail")
    pub trail_name: Option<String>,

    // Activity Classification and Detail
    /// Workout type designation (e.g., Strava: 0=default, 1=race, 2=long run, 3=workout, 10=trail run, 11=road run)
    /// This helps distinguish trail vs road runs, race efforts, etc.
    pub workout_type: Option<u32>,
    /// Detailed sport type from provider (e.g., "`MountainBikeRide`", "`TrailRun`", "`VirtualRide`")
    /// More granular than `sport_type` enum
    pub sport_type_detail: Option<String>,

    // Segment Performance Data
    /// Segment efforts for this activity (primarily from Strava)
    /// Contains performance data for known segments/routes within the activity
    pub segment_efforts: Option<Vec<SegmentEffort>>,

    /// Source provider of this activity data
    pub provider: String,
}

/// Sleep session data for recovery analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SleepSession {
    /// Unique identifier for the sleep session
    pub id: String,
    /// When sleep started
    pub start_time: DateTime<Utc>,
    /// When sleep ended
    pub end_time: DateTime<Utc>,
    /// Total time spent in bed (minutes)
    pub time_in_bed: u32,
    /// Actual sleep time (minutes)
    pub total_sleep_time: u32,
    /// Sleep efficiency percentage (sleep time / time in bed)
    pub sleep_efficiency: f32,
    /// Sleep quality score (0-100)
    pub sleep_score: Option<f32>,
    /// Sleep stages breakdown
    pub stages: Vec<SleepStage>,
    /// Heart rate variability during sleep
    pub hrv_during_sleep: Option<f64>,
    /// Average respiratory rate during sleep
    pub respiratory_rate: Option<f32>,
    /// Temperature variation during sleep
    pub temperature_variation: Option<f32>,
    /// Number of times awakened
    pub wake_count: Option<u32>,
    /// Time to fall asleep (minutes)
    pub sleep_onset_latency: Option<u32>,
    /// Provider of this sleep data
    pub provider: String,
}

/// Daily recovery and readiness metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryMetrics {
    /// Date for these recovery metrics
    pub date: DateTime<Utc>,
    /// Overall recovery score (0-100)
    pub recovery_score: Option<f32>,
    /// Readiness score for training (0-100)
    pub readiness_score: Option<f32>,
    /// HRV status or trend
    pub hrv_status: Option<String>,
    /// Sleep contribution to recovery (0-100)
    pub sleep_score: Option<f32>,
    /// Stress level indicator (0-100, higher = more stress)
    pub stress_level: Option<f32>,
    /// Current training load
    pub training_load: Option<f32>,
    /// Resting heart rate for the day
    pub resting_heart_rate: Option<u32>,
    /// Body temperature deviation from baseline
    pub body_temperature: Option<f32>,
    /// Respiratory rate while resting
    pub resting_respiratory_rate: Option<f32>,
    /// Provider of this recovery data
    pub provider: String,
}

/// Health metrics for comprehensive wellness tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthMetrics {
    /// Date for these health metrics
    pub date: DateTime<Utc>,
    /// Weight in kilograms
    pub weight: Option<f64>,
    /// Body fat percentage
    pub body_fat_percentage: Option<f32>,
    /// Muscle mass in kilograms
    pub muscle_mass: Option<f64>,
    /// Bone mass in kilograms
    pub bone_mass: Option<f64>,
    /// Body water percentage
    pub body_water_percentage: Option<f32>,
    /// Basal metabolic rate (calories/day)
    pub bmr: Option<u32>,
    /// Blood pressure (systolic, diastolic)
    pub blood_pressure: Option<(u32, u32)>,
    /// Blood glucose level (mg/dL)
    pub blood_glucose: Option<f32>,
    /// VO2 max estimate (ml/kg/min)
    pub vo2_max: Option<f32>,
    /// Provider of this health data
    pub provider: String,
}

impl SleepSession {
    /// Calculate sleep stages summary
    #[must_use]
    pub fn stage_summary(&self) -> HashMap<SleepStageType, u32> {
        let mut summary = HashMap::new();
        for stage in &self.stages {
            *summary.entry(stage.stage_type).or_insert(0) += stage.duration_minutes;
        }
        summary
    }

    /// Get deep sleep percentage
    #[must_use]
    pub fn deep_sleep_percentage(&self) -> f32 {
        let deep_sleep_total = self
            .stages
            .iter()
            .filter(|s| matches!(s.stage_type, SleepStageType::Deep))
            .map(|s| s.duration_minutes)
            .sum::<u32>();
        let deep_sleep_minutes =
            f32::from(u16::try_from(deep_sleep_total.min(u32::from(u16::MAX))).unwrap_or(u16::MAX));

        if self.total_sleep_time > 0 {
            let total_sleep_f32 = f32::from(
                u16::try_from(self.total_sleep_time.min(u32::from(u16::MAX))).unwrap_or(u16::MAX),
            );
            (deep_sleep_minutes / total_sleep_f32) * 100.0
        } else {
            0.0
        }
    }

    /// Get REM sleep percentage
    #[must_use]
    pub fn rem_sleep_percentage(&self) -> f32 {
        let rem_sleep_total = self
            .stages
            .iter()
            .filter(|s| matches!(s.stage_type, SleepStageType::Rem))
            .map(|s| s.duration_minutes)
            .sum::<u32>();
        let rem_sleep_minutes =
            f32::from(u16::try_from(rem_sleep_total.min(u32::from(u16::MAX))).unwrap_or(u16::MAX));

        if self.total_sleep_time > 0 {
            let total_sleep_f32 = f32::from(
                u16::try_from(self.total_sleep_time.min(u32::from(u16::MAX))).unwrap_or(u16::MAX),
            );
            (rem_sleep_minutes / total_sleep_f32) * 100.0
        } else {
            0.0
        }
    }
}

impl RecoveryMetrics {
    /// Check if recovery metrics indicate good readiness for training
    #[must_use]
    pub fn is_ready_for_training(&self) -> bool {
        // Consider ready if recovery score > 70 and readiness score > 70
        match (self.recovery_score, self.readiness_score) {
            (Some(recovery), Some(readiness)) => recovery > 70.0 && readiness > 70.0,
            (Some(recovery), None) => recovery > 70.0,
            (None, Some(readiness)) => readiness > 70.0,
            (None, None) => false,
        }
    }

    /// Get overall wellness score combining all available metrics
    #[must_use]
    pub fn wellness_score(&self) -> Option<f32> {
        let mut total_score = 0.0;
        let mut factor_count = 0;

        if let Some(recovery) = self.recovery_score {
            total_score += recovery;
            factor_count += 1;
        }

        if let Some(sleep) = self.sleep_score {
            total_score += sleep;
            factor_count += 1;
        }

        // Invert stress level (lower stress = better wellness)
        if let Some(stress) = self.stress_level {
            total_score += 100.0 - stress;
            factor_count += 1;
        }

        if factor_count > 0 {
            Some(total_score / f32::from(u8::try_from(factor_count).unwrap_or(u8::MAX)))
        } else {
            None
        }
    }
}

impl Default for Activity {
    fn default() -> Self {
        Self {
            id: "test_id".into(),
            name: "Test Activity".into(),
            sport_type: SportType::Run,
            start_date: chrono::Utc::now(),
            duration_seconds: 1800,        // 30 minutes
            distance_meters: Some(5000.0), // 5km
            elevation_gain: Some(100.0),
            average_heart_rate: Some(150),
            max_heart_rate: Some(180),
            average_speed: Some(2.78), // ~10 km/h
            max_speed: Some(4.0),
            calories: Some(350),
            steps: None,
            heart_rate_zones: None,

            // Advanced metrics (all default to None)
            average_power: None,
            max_power: None,
            normalized_power: None,
            power_zones: None,
            ftp: None,
            average_cadence: None,
            max_cadence: None,
            hrv_score: None,
            recovery_heart_rate: None,
            temperature: None,
            humidity: None,
            average_altitude: None,
            wind_speed: None,
            ground_contact_time: None,
            vertical_oscillation: None,
            stride_length: None,
            running_power: None,
            breathing_rate: None,
            spo2: None,
            training_stress_score: None,
            intensity_factor: None,
            suffer_score: None,
            time_series_data: None,

            start_latitude: None,
            start_longitude: None,
            city: None,
            region: None,
            country: None,
            trail_name: None,

            // Detailed activity classification fields
            workout_type: None,
            sport_type_detail: None,
            segment_efforts: None,

            provider: "test".into(),
        }
    }
}

/// Enumeration of supported sport/activity types
///
/// This enum covers the most common fitness activities across all providers.
/// The `Other` variant handles provider-specific activity types that don't
/// map to the standard categories.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SportType {
    /// Running activity
    Run,
    /// Cycling/biking activity
    Ride,
    /// Swimming activity
    Swim,
    /// Walking activity
    Walk,
    /// Hiking activity
    Hike,

    // Virtual/Indoor activities
    /// Indoor/trainer cycling activity
    VirtualRide,
    /// Treadmill running activity
    VirtualRun,
    /// Generic workout/exercise activity
    Workout,
    /// Yoga practice
    Yoga,

    // E-bike and specialty cycling
    /// Electric bike ride
    EbikeRide,
    /// Mountain biking activity
    MountainBike,
    /// Gravel cycling activity
    GravelRide,

    // Winter sports
    /// Cross-country skiing
    CrossCountrySkiing,
    /// Alpine/downhill skiing
    AlpineSkiing,
    /// Snowboarding activity
    Snowboarding,
    /// Snowshoeing activity
    Snowshoe,
    /// Ice skating activity
    IceSkating,
    /// Backcountry skiing
    BackcountrySkiing,

    // Water sports
    /// Kayaking activity
    Kayaking,
    /// Canoeing activity
    Canoeing,
    /// Rowing activity
    Rowing,
    /// Stand-up paddleboarding
    Paddleboarding,
    /// Surfing activity
    Surfing,
    /// Kitesurfing activity
    Kitesurfing,

    // Strength and fitness
    /// Weight/strength training
    StrengthTraining,
    /// `CrossFit` workout
    Crossfit,
    /// Pilates session
    Pilates,

    // Climbing and adventure
    /// Rock climbing activity
    RockClimbing,
    /// Trail running
    TrailRunning,

    // Team and racquet sports
    /// Soccer/football
    Soccer,
    /// Basketball
    Basketball,
    /// Tennis
    Tennis,
    /// Golf
    Golf,

    // Alternative transport
    /// Skateboarding
    Skateboarding,
    /// Inline skating
    InlineSkating,

    /// Other activity type not covered by standard categories
    Other(String),
}

impl SportType {
    /// Create `SportType` from provider string using configuration mapping
    #[must_use]
    pub fn from_provider_string(
        provider_sport: &str,
        fitness_config: &crate::config::FitnessConfig,
    ) -> Self {
        // First check if we have a configured mapping
        if let Some(internal_name) = fitness_config.map_sport_type(provider_sport) {
            return Self::from_internal_string(internal_name);
        }

        // Direct string-to-enum mapping
        match provider_sport {
            "Run" => Self::Run,
            "Ride" => Self::Ride,
            "Swim" => Self::Swim,
            "Walk" => Self::Walk,
            "Hike" => Self::Hike,
            "VirtualRide" => Self::VirtualRide,
            "VirtualRun" => Self::VirtualRun,
            "Workout" => Self::Workout,
            "Yoga" => Self::Yoga,
            "EBikeRide" => Self::EbikeRide,
            "MountainBikeRide" => Self::MountainBike,
            "GravelRide" => Self::GravelRide,
            "CrossCountrySkiing" => Self::CrossCountrySkiing,
            "AlpineSkiing" => Self::AlpineSkiing,
            "Snowboarding" => Self::Snowboarding,
            "Snowshoe" => Self::Snowshoe,
            "IceSkate" => Self::IceSkating,
            "BackcountrySki" => Self::BackcountrySkiing,
            "Kayaking" => Self::Kayaking,
            "Canoeing" => Self::Canoeing,
            "Rowing" => Self::Rowing,
            "StandUpPaddling" => Self::Paddleboarding,
            "Surfing" => Self::Surfing,
            "Kitesurf" => Self::Kitesurfing,
            "WeightTraining" => Self::StrengthTraining,
            "Crossfit" => Self::Crossfit,
            "Pilates" => Self::Pilates,
            "RockClimbing" => Self::RockClimbing,
            "TrailRunning" => Self::TrailRunning,
            "Soccer" => Self::Soccer,
            "Basketball" => Self::Basketball,
            "Tennis" => Self::Tennis,
            "Golf" => Self::Golf,
            "Skateboard" => Self::Skateboarding,
            "InlineSkate" => Self::InlineSkating,
            other => Self::Other(other.to_owned()),
        }
    }

    /// Create `SportType` from internal configuration string
    #[must_use]
    pub fn from_internal_string(internal_name: &str) -> Self {
        match internal_name {
            "run" => Self::Run,
            "bike_ride" => Self::Ride,
            "swim" => Self::Swim,
            "walk" => Self::Walk,
            "hike" => Self::Hike,
            "virtual_ride" => Self::VirtualRide,
            "virtual_run" => Self::VirtualRun,
            "workout" => Self::Workout,
            "yoga" => Self::Yoga,
            "ebike_ride" => Self::EbikeRide,
            "mountain_bike" => Self::MountainBike,
            "gravel_ride" => Self::GravelRide,
            "cross_country_skiing" => Self::CrossCountrySkiing,
            "alpine_skiing" => Self::AlpineSkiing,
            "snowboarding" => Self::Snowboarding,
            "snowshoe" => Self::Snowshoe,
            "ice_skating" => Self::IceSkating,
            "backcountry_skiing" => Self::BackcountrySkiing,
            "kayaking" => Self::Kayaking,
            "canoeing" => Self::Canoeing,
            "rowing" => Self::Rowing,
            "paddleboarding" => Self::Paddleboarding,
            "surfing" => Self::Surfing,
            "kitesurfing" => Self::Kitesurfing,
            "strength_training" => Self::StrengthTraining,
            "crossfit" => Self::Crossfit,
            "pilates" => Self::Pilates,
            "rock_climbing" => Self::RockClimbing,
            "trail_running" => Self::TrailRunning,
            "soccer" => Self::Soccer,
            "basketball" => Self::Basketball,
            "tennis" => Self::Tennis,
            "golf" => Self::Golf,
            "skateboarding" => Self::Skateboarding,
            "inline_skating" => Self::InlineSkating,
            other => Self::Other(other.to_owned()),
        }
    }

    /// Get the human-readable name for this sport type
    #[must_use]
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Run => "run",
            Self::Ride => "bike ride",
            Self::Swim => "swim",
            Self::Walk => "walk",
            Self::Hike => "hike",
            Self::VirtualRide => "indoor bike ride",
            Self::VirtualRun => "treadmill run",
            Self::Workout => "workout",
            Self::Yoga => "yoga session",
            Self::EbikeRide => "e-bike ride",
            Self::MountainBike => "mountain bike ride",
            Self::GravelRide => "gravel ride",
            Self::CrossCountrySkiing => "cross-country ski",
            Self::AlpineSkiing => "alpine ski",
            Self::Snowboarding => "snowboard session",
            Self::Snowshoe => "snowshoe hike",
            Self::IceSkating => "ice skating session",
            Self::BackcountrySkiing => "backcountry ski",
            Self::Kayaking => "kayak session",
            Self::Canoeing => "canoe trip",
            Self::Rowing => "rowing session",
            Self::Paddleboarding => "paddleboard session",
            Self::Surfing => "surf session",
            Self::Kitesurfing => "kitesurf session",
            Self::StrengthTraining => "strength training",
            Self::Crossfit => "CrossFit workout",
            Self::Pilates => "Pilates session",
            Self::RockClimbing => "climbing session",
            Self::TrailRunning => "trail run",
            Self::Soccer => "soccer game",
            Self::Basketball => "basketball game",
            Self::Tennis => "tennis match",
            Self::Golf => "golf round",
            Self::Skateboarding => "skate session",
            Self::InlineSkating => "inline skating",
            Self::Other(_name) => "activity", // Could use name but keeping generic
        }
    }
}

/// Represents an athlete/user profile from any provider
///
/// Contains the essential profile information that's commonly available
/// across fitness platforms.
///
/// # Examples
///
/// ```rust
/// use pierre_mcp_server::models::Athlete;
///
/// let athlete = Athlete {
///     id: "12345".into(),
///     username: "runner123".into(),
///     firstname: Some("John".into()),
///     lastname: Some("Doe".into()),
///     profile_picture: Some("https://dgalywyr863hv.cloudfront.net/pictures/athletes/12345678/avatar/medium.jpg".into()),
///     provider: "strava".into(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Athlete {
    /// Unique identifier for the athlete (provider-specific)
    pub id: String,
    /// Public username/handle
    pub username: String,
    /// First name (may not be public on some providers)
    pub firstname: Option<String>,
    /// Last name (may not be public on some providers)
    pub lastname: Option<String>,
    /// `URL` to profile picture/avatar
    pub profile_picture: Option<String>,
    /// Source provider of this athlete data
    pub provider: String,
}

/// Aggregated fitness statistics for an athlete
///
/// Contains summarized statistics across all activities for a given time period.
/// Values are typically calculated from the athlete's activity history.
///
/// # Examples
///
/// ```rust
/// use pierre_mcp_server::models::Stats;
///
/// let stats = Stats {
///     total_activities: 150,
///     total_distance: 1500000.0, // 1500 km in meters
///     total_duration: 540000, // 150 hours in seconds
///     total_elevation_gain: 25000.0, // 25km of elevation
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stats {
    /// Total number of recorded activities
    pub total_activities: u64,
    /// Total distance covered across all activities (meters)
    pub total_distance: f64,
    /// Total time spent in activities (seconds)
    pub total_duration: u64,
    /// Total elevation gained across all activities (meters)
    pub total_elevation_gain: f64,
}

/// Represents a personal record achievement
///
/// Tracks the athlete's best performance in various metrics.
/// Links back to the specific activity where the record was achieved.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalRecord {
    /// `ID` of the activity where this record was achieved
    pub activity_id: String,
    /// Type of performance metric
    pub metric: PrMetric,
    /// Value of the record (units depend on metric type)
    pub value: f64,
    /// When the record was achieved
    pub date: DateTime<Utc>,
}

/// Types of personal record metrics tracked
///
/// Each metric represents a different aspect of athletic performance
/// that can be optimized and tracked over time.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PrMetric {
    /// Fastest pace achieved (seconds per meter)
    FastestPace,
    /// Longest distance covered in a single activity (meters)
    LongestDistance,
    /// Highest elevation gained in a single activity (meters)
    HighestElevation,
    /// Fastest completion time for a standard distance (seconds)
    FastestTime,
}

// ================================================================================================
// Multi-Tenant Models
// ================================================================================================

/// Represents a user in the multi-tenant system
///
/// Users are authenticated through `OAuth` providers and have encrypted tokens
/// stored securely for accessing their fitness data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// Unique user identifier
    pub id: Uuid,
    /// User email address (used for identification)
    pub email: String,
    /// Display name
    pub display_name: Option<String>,
    /// Hashed password for authentication
    pub password_hash: String,
    /// User tier for rate limiting
    pub tier: UserTier,
    /// Tenant this user belongs to for multi-tenant architecture
    pub tenant_id: Option<String>,
    /// Encrypted Strava tokens
    pub strava_token: Option<EncryptedToken>,
    /// Encrypted Fitbit tokens
    pub fitbit_token: Option<EncryptedToken>,
    /// When the user account was created
    pub created_at: DateTime<Utc>,
    /// Last time user accessed the system
    pub last_active: DateTime<Utc>,
    /// Whether the user account is active
    pub is_active: bool,
    /// User account status for admin approval workflow
    pub user_status: UserStatus,
    /// Whether this user has admin privileges
    pub is_admin: bool,
    /// Admin who approved this user (if approved)
    pub approved_by: Option<Uuid>,
    /// When the user was approved by admin
    pub approved_at: Option<DateTime<Utc>>,
}

/// User physiological profile for personalized analysis
///
/// Contains physiological data used for calculating personalized heart rate zones,
/// pace zones, and other performance thresholds based on individual fitness metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPhysiologicalProfile {
    /// User `ID` this profile belongs to
    pub user_id: Uuid,
    /// VO2 max in ml/kg/min (if measured or estimated)
    pub vo2_max: Option<f64>,
    /// Resting heart rate in bpm
    pub resting_hr: Option<u16>,
    /// Maximum heart rate in bpm
    pub max_hr: Option<u16>,
    /// Lactate threshold as percentage of VO2 max (0.65-0.95)
    pub lactate_threshold_percentage: Option<f64>,
    /// Age in years
    pub age: Option<u16>,
    /// Weight in kg
    pub weight: Option<f64>,
    /// Overall fitness level
    pub fitness_level: crate::configuration::profiles::FitnessLevel,
    /// Primary sport for specialized calculations
    pub primary_sport: SportType,
    /// Years of training experience
    pub training_experience_years: Option<u8>,
}

impl UserPhysiologicalProfile {
    /// Create a new physiological profile
    #[must_use]
    pub const fn new(user_id: Uuid, primary_sport: SportType) -> Self {
        Self {
            user_id,
            vo2_max: None,
            resting_hr: None,
            max_hr: None,
            lactate_threshold_percentage: None,
            age: None,
            weight: None,
            fitness_level: crate::configuration::profiles::FitnessLevel::Recreational,
            primary_sport,
            training_experience_years: None,
        }
    }

    /// Estimate max heart rate from age if not provided using Tanaka formula
    #[must_use]
    #[allow(clippy::cast_possible_truncation)] // Safe: HR is constrained to 0-220 range
    #[allow(clippy::cast_sign_loss)] // Safe: HR is always positive from algorithm
    pub fn estimated_max_hr(&self) -> Option<u16> {
        self.max_hr.or_else(|| {
            self.age.map(|age| {
                use crate::intelligence::algorithms::MaxHrAlgorithm;

                // Use Tanaka formula via enum (gold standard: 208 - 0.7xage)
                MaxHrAlgorithm::Tanaka
                    .estimate(u32::from(age), None)
                    .ok()
                    .map_or_else(|| 220_u16.saturating_sub(age), |hr| hr.round() as u16)
            })
        })
    }

    /// Check if profile has sufficient data for VO2 max calculations
    #[must_use]
    pub const fn has_vo2_max_data(&self) -> bool {
        self.vo2_max.is_some()
            && self.resting_hr.is_some()
            && (self.max_hr.is_some() || self.age.is_some())
    }

    /// Get fitness level from VO2 max if available
    #[must_use]
    pub fn fitness_level_from_vo2_max(&self) -> crate::configuration::profiles::FitnessLevel {
        self.vo2_max.map_or(self.fitness_level, |vo2_max| {
            crate::configuration::profiles::FitnessLevel::from_vo2_max(
                vo2_max, self.age, None, // Gender not stored in this profile
            )
        })
    }
}

/// Encrypted `OAuth` token storage
///
/// Tokens are encrypted at rest using AES-256-GCM encryption.
/// Only decrypted when needed for `API` calls.
/// Each encrypted token has its nonce prepended to the ciphertext.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedToken {
    /// Encrypted access token with prepended nonce (base64 encoded: \[12-byte nonce\]\[ciphertext\])
    pub access_token: String,
    /// Encrypted refresh token with prepended nonce (base64 encoded: \[12-byte nonce\]\[ciphertext\])
    pub refresh_token: String,
    /// When the access token expires
    pub expires_at: DateTime<Utc>,
    /// Token scope permissions
    pub scope: String,
}

/// Decrypted `OAuth` token for `API` calls
///
/// This is never stored - only exists in memory during `API` requests.
#[derive(Debug, Clone)]
pub struct DecryptedToken {
    /// Plain text access token
    pub access_token: String,
    /// Plain text refresh token
    pub refresh_token: String,
    /// When the access token expires
    pub expires_at: DateTime<Utc>,
    /// Token scope permissions
    pub scope: String,
}

/// User OAuth token for tenant-provider combination
///
/// Stores user's personal OAuth tokens for accessing fitness providers
/// within their tenant's application context. Each user can have one token
/// per tenant-provider combination (e.g., user's Strava token in tenant A).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserOAuthToken {
    /// Unique identifier for this token record
    pub id: String,
    /// User who owns this token
    pub user_id: Uuid,
    /// Tenant context for this token
    pub tenant_id: String,
    /// Provider name (strava, fitbit, etc.)
    pub provider: String,
    /// Encrypted OAuth access token
    pub access_token: String,
    /// Encrypted OAuth refresh token (optional for some providers)
    pub refresh_token: Option<String>,
    /// Token type (usually "Bearer")
    pub token_type: String,
    /// When the access token expires
    pub expires_at: Option<DateTime<Utc>>,
    /// Granted OAuth scopes
    pub scope: Option<String>,
    /// When this token was first stored
    pub created_at: DateTime<Utc>,
    /// When this token was last updated
    pub updated_at: DateTime<Utc>,
}

impl UserOAuthToken {
    /// Create a new user OAuth token
    #[must_use]
    pub fn new(
        user_id: Uuid,
        tenant_id: String,
        provider: String,
        access_token: String,
        refresh_token: Option<String>,
        expires_at: Option<DateTime<Utc>>,
        scope: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            user_id,
            tenant_id,
            provider,
            access_token,
            refresh_token,
            token_type: "Bearer".to_owned(),
            expires_at,
            scope,
            created_at: now,
            updated_at: now,
        }
    }

    /// Check if the access token is expired
    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .is_some_and(|expires_at| Utc::now() > expires_at)
    }

    /// Check if token needs refresh (expires within 5 minutes)
    #[must_use]
    pub fn needs_refresh(&self) -> bool {
        self.expires_at.is_some_and(|expires_at| {
            let refresh_threshold = Utc::now() + chrono::Duration::minutes(5);
            refresh_threshold >= expires_at
        })
    }

    /// Update token with new values
    pub fn update_token(
        &mut self,
        access_token: String,
        refresh_token: Option<String>,
        expires_at: Option<DateTime<Utc>>,
        scope: Option<String>,
    ) {
        self.access_token = access_token;
        self.refresh_token = refresh_token;
        self.expires_at = expires_at;
        self.scope = scope;
        self.updated_at = Utc::now();
    }
}

/// User OAuth app credentials for cloud deployment
///
/// Each user can configure their own OAuth application credentials
/// for each provider (Strava, Fitbit, etc.) to work in cloud deployments
/// where server-wide environment variables won't work.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserOAuthApp {
    /// Unique identifier for this OAuth app configuration
    pub id: String,
    /// User who owns this OAuth app configuration
    pub user_id: Uuid,
    /// OAuth provider name (strava, fitbit, etc.)
    pub provider: String,
    /// OAuth client ID from the provider
    pub client_id: String,
    /// OAuth client secret from the provider (encrypted)
    pub client_secret: String,
    /// OAuth redirect URI configured with the provider
    pub redirect_uri: String,
    /// When this configuration was created
    pub created_at: DateTime<Utc>,
    /// When this configuration was last updated
    pub updated_at: DateTime<Utc>,
}

impl UserOAuthApp {
    /// Create a new user OAuth app configuration
    #[must_use]
    pub fn new(
        user_id: Uuid,
        provider: String,
        client_id: String,
        client_secret: String,
        redirect_uri: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            user_id,
            provider,
            client_id,
            client_secret,
            redirect_uri,
            created_at: now,
            updated_at: now,
        }
    }
}

/// User session for `MCP` protocol authentication
///
/// Contains `JWT` token and user context for secure `MCP` communication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSession {
    /// User `ID` this session belongs to
    pub user_id: Uuid,
    /// `JWT` token for authentication
    pub jwt_token: String,
    /// When the session expires
    pub expires_at: DateTime<Utc>,
    /// User's email for display
    pub email: String,
    /// Available fitness providers for this user
    pub available_providers: Vec<String>,
}

/// Authentication request for `MCP` protocol
///
/// Clients send this to authenticate with the `MCP` server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthRequest {
    /// `JWT` token for authentication
    pub token: String,
}

/// Authentication response for `MCP` protocol
///
/// Server responds with user context and available capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    /// Whether authentication was successful
    pub authenticated: bool,
    /// User `ID` if authenticated
    pub user_id: Option<Uuid>,
    /// Error message if authentication failed
    pub error: Option<String>,
    /// Available fitness providers for this user
    pub available_providers: Vec<String>,
}

impl User {
    /// Create a new user with the given email and password hash
    #[must_use]
    pub fn new(email: String, password_hash: String, display_name: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            email,
            display_name,
            password_hash,
            tier: UserTier::Starter, // Default to starter tier
            tenant_id: None,         // No tenant assigned until approval
            strava_token: None,
            fitbit_token: None,
            created_at: now,
            last_active: now,
            is_active: true,
            user_status: UserStatus::Pending, // New users need admin approval
            is_admin: false,                  // Regular users are not admins by default
            approved_by: None,
            approved_at: None,
        }
    }

    /// Check if user has valid Strava token
    #[must_use]
    pub fn has_strava_access(&self) -> bool {
        self.strava_token
            .as_ref()
            .is_some_and(|token| token.expires_at > Utc::now())
    }

    /// Check if user has valid Fitbit token
    #[must_use]
    pub fn has_fitbit_access(&self) -> bool {
        self.fitbit_token
            .as_ref()
            .is_some_and(|token| token.expires_at > Utc::now())
    }

    /// Get list of available providers for this user
    #[must_use]
    pub fn available_providers(&self) -> Vec<String> {
        let mut providers = Vec::with_capacity(2); // Typically Strava and Fitbit
        if self.has_strava_access() {
            providers.push("strava".into());
        }
        if self.has_fitbit_access() {
            providers.push("fitbit".into());
        }
        providers
    }

    /// Update last active timestamp
    pub fn update_last_active(&mut self) {
        self.last_active = Utc::now();
    }
}

impl EncryptedToken {
    /// Create a new encrypted token
    ///
    /// Encrypts both access and refresh tokens with independent nonces.
    /// Each nonce is prepended to its corresponding ciphertext for cryptographic independence.
    ///
    /// # Errors
    ///
    /// Returns an error if encryption fails or if the encryption key is invalid
    pub fn new(
        access_token: &str,
        refresh_token: &str,
        expires_at: DateTime<Utc>,
        scope: String,
        encryption_key: &[u8],
    ) -> crate::errors::AppResult<Self> {
        use base64::{engine::general_purpose, Engine as _};
        use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
        use ring::rand::{SecureRandom, SystemRandom};

        let rng = SystemRandom::new();

        // Encrypt access token with its own nonce
        let mut access_nonce_bytes = [0u8; 12];
        rng.fill(&mut access_nonce_bytes)?;
        let access_nonce = Nonce::assume_unique_for_key(access_nonce_bytes);

        let unbound_key = UnboundKey::new(&AES_256_GCM, encryption_key)?;
        let key = LessSafeKey::new(unbound_key);

        let mut access_token_data = access_token.as_bytes().to_vec();
        key.seal_in_place_append_tag(access_nonce, Aad::empty(), &mut access_token_data)?;

        // Prepend nonce to ciphertext (modern pattern)
        let mut access_combined = access_nonce_bytes.to_vec();
        access_combined.extend(access_token_data);
        let encrypted_access = general_purpose::STANDARD.encode(access_combined);

        // Encrypt refresh token with its own independent nonce
        let mut refresh_nonce_bytes = [0u8; 12];
        rng.fill(&mut refresh_nonce_bytes)?;
        let refresh_nonce = Nonce::assume_unique_for_key(refresh_nonce_bytes);

        let unbound_key2 = UnboundKey::new(&AES_256_GCM, encryption_key)?;
        let key2 = LessSafeKey::new(unbound_key2);

        let mut refresh_token_data = refresh_token.as_bytes().to_vec();
        key2.seal_in_place_append_tag(refresh_nonce, Aad::empty(), &mut refresh_token_data)?;

        // Prepend nonce to ciphertext (modern pattern)
        let mut refresh_combined = refresh_nonce_bytes.to_vec();
        refresh_combined.extend(refresh_token_data);
        let encrypted_refresh = general_purpose::STANDARD.encode(refresh_combined);

        Ok(Self {
            access_token: encrypted_access,
            refresh_token: encrypted_refresh,
            expires_at,
            scope,
        })
    }

    /// Decrypt the token for use
    ///
    /// Extracts nonces from the prepended ciphertext and decrypts each token independently.
    ///
    /// # Errors
    ///
    /// Returns an error if decryption fails, nonce is invalid, or the encryption key is incorrect
    pub fn decrypt(&self, encryption_key: &[u8]) -> crate::errors::AppResult<DecryptedToken> {
        use base64::{engine::general_purpose, Engine as _};
        use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};

        // Decrypt access token: extract nonce from prepended data
        let access_combined = general_purpose::STANDARD.decode(&self.access_token)?;
        if access_combined.len() < 12 {
            return Err(AppError::invalid_input("Invalid access token: too short"));
        }

        let (access_nonce_bytes, access_ciphertext) = access_combined.split_at(12);
        let access_nonce = Nonce::assume_unique_for_key(access_nonce_bytes.try_into()?);

        let unbound_key = UnboundKey::new(&AES_256_GCM, encryption_key)?;
        let key = LessSafeKey::new(unbound_key);

        let mut access_data = access_ciphertext.to_vec();
        let access_plaintext = key.open_in_place(access_nonce, Aad::empty(), &mut access_data)?;
        let access_token = String::from_utf8(access_plaintext.to_vec())
            .map_err(|e| AppError::invalid_input(format!("Invalid UTF-8 in access token: {e}")))?;

        // Decrypt refresh token: extract nonce from prepended data
        let refresh_combined = general_purpose::STANDARD.decode(&self.refresh_token)?;
        if refresh_combined.len() < 12 {
            return Err(AppError::invalid_input("Invalid refresh token: too short"));
        }

        let (refresh_nonce_bytes, refresh_ciphertext) = refresh_combined.split_at(12);
        let refresh_nonce = Nonce::assume_unique_for_key(refresh_nonce_bytes.try_into()?);

        let unbound_key2 = UnboundKey::new(&AES_256_GCM, encryption_key)?;
        let key2 = LessSafeKey::new(unbound_key2);

        let mut refresh_data = refresh_ciphertext.to_vec();
        let refresh_plaintext =
            key2.open_in_place(refresh_nonce, Aad::empty(), &mut refresh_data)?;
        let refresh_token = String::from_utf8(refresh_plaintext.to_vec())
            .map_err(|e| AppError::invalid_input(format!("Invalid UTF-8 in refresh token: {e}")))?;

        Ok(DecryptedToken {
            access_token,
            refresh_token,
            expires_at: self.expires_at,
            scope: self.scope.clone(),
        })
    }
}

// Multi-Tenant Models

/// Tenant organization in multi-tenant setup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    /// Unique tenant identifier
    pub id: Uuid,
    /// Tenant organization name
    pub name: String,
    /// URL-safe slug for tenant
    pub slug: String,
    /// Custom domain for tenant (optional)
    pub domain: Option<String>,
    /// Subscription plan (basic, pro, enterprise)
    pub plan: String,
    /// User ID of the tenant owner
    pub owner_user_id: Uuid,
    /// When tenant was created
    pub created_at: DateTime<Utc>,
    /// When tenant was last updated
    pub updated_at: DateTime<Utc>,
}

impl Tenant {
    /// Creates a new tenant with the given details
    #[must_use]
    pub fn new(
        name: String,
        slug: String,
        domain: Option<String>,
        plan: String,
        owner_user_id: Uuid,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            slug,
            domain,
            plan,
            owner_user_id,
            created_at: now,
            updated_at: now,
        }
    }
}

/// OAuth application registration for MCP clients
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthApp {
    /// Unique app identifier
    pub id: Uuid,
    /// OAuth client ID
    pub client_id: String,
    /// OAuth client secret
    pub client_secret: String,
    /// Application name
    pub name: String,
    /// Application description
    pub description: Option<String>,
    /// Allowed redirect URIs
    pub redirect_uris: Vec<String>,
    /// Permitted scopes
    pub scopes: Vec<String>,
    /// Application type (desktop, web, mobile, server)
    pub app_type: String,
    /// User ID of the app owner
    pub owner_user_id: Uuid,
    /// When app was registered
    pub created_at: DateTime<Utc>,
    /// When app was last updated
    pub updated_at: DateTime<Utc>,
}

/// OAuth app creation parameters
pub struct OAuthAppParams {
    /// OAuth 2.0 client identifier
    pub client_id: String,
    /// OAuth 2.0 client secret for authentication
    pub client_secret: String,
    /// Human-readable name of the OAuth application
    pub name: String,
    /// Optional description of the application's purpose
    pub description: Option<String>,
    /// List of authorized redirect URIs for OAuth flow
    pub redirect_uris: Vec<String>,
    /// List of OAuth scopes the app can request
    pub scopes: Vec<String>,
    /// Type of OAuth application (e.g., "web", "native", "service")
    pub app_type: String,
    /// UUID of the user who owns this OAuth app
    pub owner_user_id: Uuid,
}

impl OAuthApp {
    /// Create new OAuth app from parameters
    #[must_use]
    pub fn new(params: OAuthAppParams) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            client_id: params.client_id,
            client_secret: params.client_secret,
            name: params.name,
            description: params.description,
            redirect_uris: params.redirect_uris,
            scopes: params.scopes,
            app_type: params.app_type,
            owner_user_id: params.owner_user_id,
            created_at: now,
            updated_at: now,
        }
    }
}

/// OAuth authorization code for token exchange
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationCode {
    /// The authorization code
    pub code: String,
    /// Client ID that requested the code
    pub client_id: String,
    /// Redirect URI used in the request
    pub redirect_uri: String,
    /// Requested scopes
    pub scope: String,
    /// User ID that authorized the request
    pub user_id: Option<Uuid>,
    /// When the code expires
    pub expires_at: DateTime<Utc>,
    /// When the code was created
    pub created_at: DateTime<Utc>,
    /// Whether the code has been used
    pub is_used: bool,
}

impl AuthorizationCode {
    /// Creates a new authorization code with 10-minute expiration
    #[must_use]
    pub fn new(
        code: String,
        client_id: String,
        redirect_uri: String,
        scope: String,
        user_id: Option<Uuid>,
    ) -> Self {
        let now = Utc::now();
        Self {
            code,
            client_id,
            redirect_uri,
            scope,
            user_id,
            expires_at: now + chrono::Duration::minutes(10), // 10 minute expiration
            created_at: now,
            is_used: false,
        }
    }
}
