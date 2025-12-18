// ABOUTME: Core data models and types for the Pierre fitness API
// ABOUTME: Defines Activity, User, SportType and other fundamental data structures
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

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

use std::collections::HashMap;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::profiles::FitnessLevel;
use crate::config::FitnessConfig;
use crate::constants::tiers;
use crate::errors::{AppError, AppResult};
use crate::intelligence::algorithms::MaxHrAlgorithm;
use crate::permissions::UserRole;

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heart_rate: Option<Vec<u32>>,
    /// Power measurements (watts)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub power: Option<Vec<u32>>,
    /// Cadence measurements (RPM or steps/min)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cadence: Option<Vec<u32>>,
    /// Speed measurements (m/s)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<Vec<f32>>,
    /// Altitude measurements (meters)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub altitude: Option<Vec<f32>>,
    /// Temperature measurements (Celsius)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<Vec<f32>>,
    /// GPS coordinates (lat, lon pairs)
    #[serde(skip_serializing_if = "Option::is_none")]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub moving_time: Option<u64>,
    /// When the segment effort started (UTC)
    pub start_date: DateTime<Utc>,
    /// Distance of the segment in meters
    pub distance: f64,
    /// Average heart rate during segment (BPM)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub average_heart_rate: Option<u32>,
    /// Max heart rate during segment (BPM)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_heart_rate: Option<u32>,
    /// Average cadence during segment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub average_cadence: Option<u32>,
    /// Average power during segment (watts)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub average_watts: Option<u32>,
    /// King of the Mountain (KOM) rank for this effort (1 = fastest ever)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kom_rank: Option<u32>,
    /// Personal Record (PR) rank for this athlete (1 = athlete's best)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pr_rank: Option<u32>,
    /// Segment climb category (HC, 1-4, or None)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub climb_category: Option<u32>,
    /// Average grade/gradient of the segment (percentage)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub average_grade: Option<f32>,
    /// Elevation gain on the segment in meters
    #[serde(skip_serializing_if = "Option::is_none")]
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

impl Display for UserTier {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
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

impl Display for UserStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
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

impl FromStr for UserTier {
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
/// Fields are private to ensure data integrity - use accessor methods to read
/// and `ActivityBuilder` to construct new instances.
///
/// # Examples
///
/// ```rust
/// use pierre_mcp_server::models::{Activity, ActivityBuilder, SportType};
/// use chrono::Utc;
///
/// let activity = ActivityBuilder::new(
///     "12345",
///     "Morning Run",
///     SportType::Run,
///     Utc::now(),
///     1800,
///     "strava",
/// )
/// .distance_meters(5000.0)
/// .elevation_gain(100.0)
/// .average_heart_rate(150)
/// .max_heart_rate(175)
/// .city("Montreal".to_owned())
/// .build();
///
/// assert_eq!(activity.id(), "12345");
/// assert_eq!(activity.name(), "Morning Run");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    /// Unique identifier for the activity (provider-specific)
    id: String,
    /// Human-readable name/title of the activity
    name: String,
    /// Type of sport/activity (run, ride, swim, etc.)
    sport_type: SportType,
    /// When the activity started (UTC)
    start_date: DateTime<Utc>,
    /// Total duration of the activity in seconds
    duration_seconds: u64,
    /// Total distance covered in meters (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    distance_meters: Option<f64>,
    /// Total elevation gained in meters (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    elevation_gain: Option<f64>,
    /// Average heart rate during the activity (BPM)
    #[serde(skip_serializing_if = "Option::is_none")]
    average_heart_rate: Option<u32>,
    /// Maximum heart rate reached during the activity (BPM)
    #[serde(skip_serializing_if = "Option::is_none")]
    max_heart_rate: Option<u32>,
    /// Average speed in meters per second
    #[serde(skip_serializing_if = "Option::is_none")]
    average_speed: Option<f64>,
    /// Maximum speed reached in meters per second
    #[serde(skip_serializing_if = "Option::is_none")]
    max_speed: Option<f64>,
    /// Estimated calories burned during the activity
    #[serde(skip_serializing_if = "Option::is_none")]
    calories: Option<u32>,
    /// Total steps taken during the activity (for walking/running activities)
    #[serde(skip_serializing_if = "Option::is_none")]
    steps: Option<u32>,
    /// Heart rate zone data if available from the provider
    #[serde(skip_serializing_if = "Option::is_none")]
    heart_rate_zones: Option<Vec<HeartRateZone>>,

    // Advanced Power Metrics
    /// Average power output in watts (cycling/rowing)
    #[serde(skip_serializing_if = "Option::is_none")]
    average_power: Option<u32>,
    /// Maximum power output reached in watts
    #[serde(skip_serializing_if = "Option::is_none")]
    max_power: Option<u32>,
    /// Normalized power (power adjusted for variability)
    #[serde(skip_serializing_if = "Option::is_none")]
    normalized_power: Option<u32>,
    /// Power zone distribution
    #[serde(skip_serializing_if = "Option::is_none")]
    power_zones: Option<Vec<PowerZone>>,
    /// Functional Threshold Power at time of activity
    #[serde(skip_serializing_if = "Option::is_none")]
    ftp: Option<u32>,

    // Cadence Metrics
    /// Average cadence (RPM for cycling, steps/min for running)
    #[serde(skip_serializing_if = "Option::is_none")]
    average_cadence: Option<u32>,
    /// Maximum cadence reached
    #[serde(skip_serializing_if = "Option::is_none")]
    max_cadence: Option<u32>,

    // Advanced Heart Rate Metrics
    /// Heart Rate Variability score during activity
    #[serde(skip_serializing_if = "Option::is_none")]
    hrv_score: Option<f64>,
    /// Heart rate recovery (drop in first minute after activity)
    #[serde(skip_serializing_if = "Option::is_none")]
    recovery_heart_rate: Option<u32>,

    // Environmental Conditions
    /// Temperature during activity (Celsius)
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    /// Humidity percentage during activity
    #[serde(skip_serializing_if = "Option::is_none")]
    humidity: Option<f32>,
    /// Average altitude during activity (meters)
    #[serde(skip_serializing_if = "Option::is_none")]
    average_altitude: Option<f32>,
    /// Wind speed during activity (m/s)
    #[serde(skip_serializing_if = "Option::is_none")]
    wind_speed: Option<f32>,

    // Biomechanical Metrics (Running)
    /// Ground contact time in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    ground_contact_time: Option<u32>,
    /// Vertical oscillation in centimeters
    #[serde(skip_serializing_if = "Option::is_none")]
    vertical_oscillation: Option<f32>,
    /// Average stride length in meters
    #[serde(skip_serializing_if = "Option::is_none")]
    stride_length: Option<f32>,
    /// Running power (estimated or measured)
    #[serde(skip_serializing_if = "Option::is_none")]
    running_power: Option<u32>,

    // Respiratory and Oxygen Metrics
    /// Average breathing rate (breaths per minute)
    #[serde(skip_serializing_if = "Option::is_none")]
    breathing_rate: Option<u32>,
    /// Blood oxygen saturation percentage
    #[serde(skip_serializing_if = "Option::is_none")]
    spo2: Option<f32>,

    // Training Load and Performance
    /// Training Stress Score for this activity
    #[serde(skip_serializing_if = "Option::is_none")]
    training_stress_score: Option<f32>,
    /// Intensity Factor (normalized intensity vs threshold)
    #[serde(skip_serializing_if = "Option::is_none")]
    intensity_factor: Option<f32>,
    /// Suffer score or relative effort rating
    #[serde(skip_serializing_if = "Option::is_none")]
    suffer_score: Option<u32>,

    // Detailed Time-Series Data
    /// Time-series data for advanced analysis
    #[serde(skip_serializing_if = "Option::is_none")]
    time_series_data: Option<TimeSeriesData>,
    /// Starting latitude coordinate (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    start_latitude: Option<f64>,
    /// Starting longitude coordinate (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    start_longitude: Option<f64>,
    /// Location information extracted from GPS coordinates
    #[serde(skip_serializing_if = "Option::is_none")]
    city: Option<String>,
    /// Region/state/province where the activity took place
    #[serde(skip_serializing_if = "Option::is_none")]
    region: Option<String>,
    /// Country where the activity took place
    #[serde(skip_serializing_if = "Option::is_none")]
    country: Option<String>,
    /// Trail or route name if available (e.g., "Saint-Hippolyte trail")
    #[serde(skip_serializing_if = "Option::is_none")]
    trail_name: Option<String>,

    // Activity Classification and Detail
    /// Workout type designation (e.g., Strava: 0=default, 1=race, 2=long run, 3=workout, 10=trail run, 11=road run)
    /// This helps distinguish trail vs road runs, race efforts, etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    workout_type: Option<u32>,
    /// Detailed sport type from provider (e.g., "`MountainBikeRide`", "`TrailRun`", "`VirtualRide`")
    /// More granular than `sport_type` enum
    #[serde(skip_serializing_if = "Option::is_none")]
    sport_type_detail: Option<String>,

    // Segment Performance Data
    /// Segment efforts for this activity (primarily from Strava)
    /// Contains performance data for known segments/routes within the activity
    #[serde(skip_serializing_if = "Option::is_none")]
    segment_efforts: Option<Vec<SegmentEffort>>,

    /// Source provider of this activity data
    provider: String,
}

/// Accessor methods for Activity fields
impl Activity {
    /// Returns the unique identifier for the activity
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the human-readable name/title of the activity
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the type of sport/activity
    #[must_use]
    pub const fn sport_type(&self) -> &SportType {
        &self.sport_type
    }

    /// Returns when the activity started (UTC)
    #[must_use]
    pub const fn start_date(&self) -> DateTime<Utc> {
        self.start_date
    }

    /// Returns the total duration of the activity in seconds
    #[must_use]
    pub const fn duration_seconds(&self) -> u64 {
        self.duration_seconds
    }

    /// Returns the total distance covered in meters (if applicable)
    #[must_use]
    pub const fn distance_meters(&self) -> Option<f64> {
        self.distance_meters
    }

    /// Returns the total elevation gained in meters (if available)
    #[must_use]
    pub const fn elevation_gain(&self) -> Option<f64> {
        self.elevation_gain
    }

    /// Returns the average heart rate during the activity (BPM)
    #[must_use]
    pub const fn average_heart_rate(&self) -> Option<u32> {
        self.average_heart_rate
    }

    /// Returns the maximum heart rate reached during the activity (BPM)
    #[must_use]
    pub const fn max_heart_rate(&self) -> Option<u32> {
        self.max_heart_rate
    }

    /// Returns the average speed in meters per second
    #[must_use]
    pub const fn average_speed(&self) -> Option<f64> {
        self.average_speed
    }

    /// Returns the maximum speed reached in meters per second
    #[must_use]
    pub const fn max_speed(&self) -> Option<f64> {
        self.max_speed
    }

    /// Returns the estimated calories burned during the activity
    #[must_use]
    pub const fn calories(&self) -> Option<u32> {
        self.calories
    }

    /// Returns the total steps taken during the activity
    #[must_use]
    pub const fn steps(&self) -> Option<u32> {
        self.steps
    }

    /// Returns the heart rate zone data if available
    #[must_use]
    pub const fn heart_rate_zones(&self) -> Option<&Vec<HeartRateZone>> {
        self.heart_rate_zones.as_ref()
    }

    /// Returns the average power output in watts
    #[must_use]
    pub const fn average_power(&self) -> Option<u32> {
        self.average_power
    }

    /// Returns the maximum power output reached in watts
    #[must_use]
    pub const fn max_power(&self) -> Option<u32> {
        self.max_power
    }

    /// Returns the normalized power (power adjusted for variability)
    #[must_use]
    pub const fn normalized_power(&self) -> Option<u32> {
        self.normalized_power
    }

    /// Returns the power zone distribution
    #[must_use]
    pub const fn power_zones(&self) -> Option<&Vec<PowerZone>> {
        self.power_zones.as_ref()
    }

    /// Returns the Functional Threshold Power at time of activity
    #[must_use]
    pub const fn ftp(&self) -> Option<u32> {
        self.ftp
    }

    /// Returns the average cadence
    #[must_use]
    pub const fn average_cadence(&self) -> Option<u32> {
        self.average_cadence
    }

    /// Returns the maximum cadence reached
    #[must_use]
    pub const fn max_cadence(&self) -> Option<u32> {
        self.max_cadence
    }

    /// Returns the Heart Rate Variability score during activity
    #[must_use]
    pub const fn hrv_score(&self) -> Option<f64> {
        self.hrv_score
    }

    /// Returns the heart rate recovery value
    #[must_use]
    pub const fn recovery_heart_rate(&self) -> Option<u32> {
        self.recovery_heart_rate
    }

    /// Returns the temperature during activity (Celsius)
    #[must_use]
    pub const fn temperature(&self) -> Option<f32> {
        self.temperature
    }

    /// Returns the humidity percentage during activity
    #[must_use]
    pub const fn humidity(&self) -> Option<f32> {
        self.humidity
    }

    /// Returns the average altitude during activity (meters)
    #[must_use]
    pub const fn average_altitude(&self) -> Option<f32> {
        self.average_altitude
    }

    /// Returns the wind speed during activity (m/s)
    #[must_use]
    pub const fn wind_speed(&self) -> Option<f32> {
        self.wind_speed
    }

    /// Returns the ground contact time in milliseconds
    #[must_use]
    pub const fn ground_contact_time(&self) -> Option<u32> {
        self.ground_contact_time
    }

    /// Returns the vertical oscillation in centimeters
    #[must_use]
    pub const fn vertical_oscillation(&self) -> Option<f32> {
        self.vertical_oscillation
    }

    /// Returns the average stride length in meters
    #[must_use]
    pub const fn stride_length(&self) -> Option<f32> {
        self.stride_length
    }

    /// Returns the running power (estimated or measured)
    #[must_use]
    pub const fn running_power(&self) -> Option<u32> {
        self.running_power
    }

    /// Returns the average breathing rate (breaths per minute)
    #[must_use]
    pub const fn breathing_rate(&self) -> Option<u32> {
        self.breathing_rate
    }

    /// Returns the blood oxygen saturation percentage
    #[must_use]
    pub const fn spo2(&self) -> Option<f32> {
        self.spo2
    }

    /// Returns the Training Stress Score for this activity
    #[must_use]
    pub const fn training_stress_score(&self) -> Option<f32> {
        self.training_stress_score
    }

    /// Returns the Intensity Factor
    #[must_use]
    pub const fn intensity_factor(&self) -> Option<f32> {
        self.intensity_factor
    }

    /// Returns the suffer score or relative effort rating
    #[must_use]
    pub const fn suffer_score(&self) -> Option<u32> {
        self.suffer_score
    }

    /// Returns the time-series data for advanced analysis
    #[must_use]
    pub const fn time_series_data(&self) -> Option<&TimeSeriesData> {
        self.time_series_data.as_ref()
    }

    /// Returns the starting latitude coordinate
    #[must_use]
    pub const fn start_latitude(&self) -> Option<f64> {
        self.start_latitude
    }

    /// Returns the starting longitude coordinate
    #[must_use]
    pub const fn start_longitude(&self) -> Option<f64> {
        self.start_longitude
    }

    /// Returns the city where the activity took place
    #[must_use]
    pub fn city(&self) -> Option<&str> {
        self.city.as_deref()
    }

    /// Returns the region/state/province where the activity took place
    #[must_use]
    pub fn region(&self) -> Option<&str> {
        self.region.as_deref()
    }

    /// Returns the country where the activity took place
    #[must_use]
    pub fn country(&self) -> Option<&str> {
        self.country.as_deref()
    }

    /// Returns the trail or route name
    #[must_use]
    pub fn trail_name(&self) -> Option<&str> {
        self.trail_name.as_deref()
    }

    /// Returns the workout type designation
    #[must_use]
    pub const fn workout_type(&self) -> Option<u32> {
        self.workout_type
    }

    /// Returns the detailed sport type from provider
    #[must_use]
    pub fn sport_type_detail(&self) -> Option<&str> {
        self.sport_type_detail.as_deref()
    }

    /// Returns the segment efforts for this activity
    #[must_use]
    pub const fn segment_efforts(&self) -> Option<&Vec<SegmentEffort>> {
        self.segment_efforts.as_ref()
    }

    /// Returns the source provider of this activity data
    #[must_use]
    pub fn provider(&self) -> &str {
        &self.provider
    }
}

/// Builder for constructing Activity instances
///
/// Since Activity fields are private, use this builder to create new instances.
/// Required fields are set in `new()`, optional fields can be set using builder methods.
///
/// # Examples
///
/// ```rust
/// use pierre_mcp_server::models::{ActivityBuilder, SportType};
/// use chrono::Utc;
///
/// let activity = ActivityBuilder::new(
///     "12345",
///     "Morning Run",
///     SportType::Run,
///     Utc::now(),
///     1800,
///     "strava",
/// )
/// .distance_meters(5000.0)
/// .average_heart_rate(150)
/// .build();
/// ```
#[derive(Debug, Clone)]
pub struct ActivityBuilder {
    activity: Activity,
}

impl ActivityBuilder {
    /// Creates a new `ActivityBuilder` with required fields
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        sport_type: SportType,
        start_date: DateTime<Utc>,
        duration_seconds: u64,
        provider: impl Into<String>,
    ) -> Self {
        Self {
            activity: Activity {
                id: id.into(),
                name: name.into(),
                sport_type,
                start_date,
                duration_seconds,
                provider: provider.into(),
                distance_meters: None,
                elevation_gain: None,
                average_heart_rate: None,
                max_heart_rate: None,
                average_speed: None,
                max_speed: None,
                calories: None,
                steps: None,
                heart_rate_zones: None,
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
                workout_type: None,
                sport_type_detail: None,
                segment_efforts: None,
            },
        }
    }

    /// Sets the distance in meters
    #[must_use]
    pub const fn distance_meters(mut self, value: f64) -> Self {
        self.activity.distance_meters = Some(value);
        self
    }

    /// Sets the distance in meters (optional)
    #[must_use]
    pub const fn distance_meters_opt(mut self, value: Option<f64>) -> Self {
        self.activity.distance_meters = value;
        self
    }

    /// Sets the elevation gain in meters
    #[must_use]
    pub const fn elevation_gain(mut self, value: f64) -> Self {
        self.activity.elevation_gain = Some(value);
        self
    }

    /// Sets the elevation gain in meters (optional)
    #[must_use]
    pub const fn elevation_gain_opt(mut self, value: Option<f64>) -> Self {
        self.activity.elevation_gain = value;
        self
    }

    /// Sets the average heart rate
    #[must_use]
    pub const fn average_heart_rate(mut self, value: u32) -> Self {
        self.activity.average_heart_rate = Some(value);
        self
    }

    /// Sets the average heart rate (optional)
    #[must_use]
    pub const fn average_heart_rate_opt(mut self, value: Option<u32>) -> Self {
        self.activity.average_heart_rate = value;
        self
    }

    /// Sets the maximum heart rate
    #[must_use]
    pub const fn max_heart_rate(mut self, value: u32) -> Self {
        self.activity.max_heart_rate = Some(value);
        self
    }

    /// Sets the maximum heart rate (optional)
    #[must_use]
    pub const fn max_heart_rate_opt(mut self, value: Option<u32>) -> Self {
        self.activity.max_heart_rate = value;
        self
    }

    /// Sets the average speed in meters per second
    #[must_use]
    pub const fn average_speed(mut self, value: f64) -> Self {
        self.activity.average_speed = Some(value);
        self
    }

    /// Sets the average speed (optional)
    #[must_use]
    pub const fn average_speed_opt(mut self, value: Option<f64>) -> Self {
        self.activity.average_speed = value;
        self
    }

    /// Sets the maximum speed in meters per second
    #[must_use]
    pub const fn max_speed(mut self, value: f64) -> Self {
        self.activity.max_speed = Some(value);
        self
    }

    /// Sets the maximum speed (optional)
    #[must_use]
    pub const fn max_speed_opt(mut self, value: Option<f64>) -> Self {
        self.activity.max_speed = value;
        self
    }

    /// Sets the calories burned
    #[must_use]
    pub const fn calories(mut self, value: u32) -> Self {
        self.activity.calories = Some(value);
        self
    }

    /// Sets the calories (optional)
    #[must_use]
    pub const fn calories_opt(mut self, value: Option<u32>) -> Self {
        self.activity.calories = value;
        self
    }

    /// Sets the steps taken
    #[must_use]
    pub const fn steps(mut self, value: u32) -> Self {
        self.activity.steps = Some(value);
        self
    }

    /// Sets the steps (optional)
    #[must_use]
    pub const fn steps_opt(mut self, value: Option<u32>) -> Self {
        self.activity.steps = value;
        self
    }

    /// Sets the heart rate zones
    #[must_use]
    pub fn heart_rate_zones(mut self, value: Vec<HeartRateZone>) -> Self {
        self.activity.heart_rate_zones = Some(value);
        self
    }

    /// Sets the heart rate zones (optional)
    #[must_use]
    pub fn heart_rate_zones_opt(mut self, value: Option<Vec<HeartRateZone>>) -> Self {
        self.activity.heart_rate_zones = value;
        self
    }

    /// Sets the average power in watts
    #[must_use]
    pub const fn average_power(mut self, value: u32) -> Self {
        self.activity.average_power = Some(value);
        self
    }

    /// Sets the average power (optional)
    #[must_use]
    pub const fn average_power_opt(mut self, value: Option<u32>) -> Self {
        self.activity.average_power = value;
        self
    }

    /// Sets the maximum power in watts
    #[must_use]
    pub const fn max_power(mut self, value: u32) -> Self {
        self.activity.max_power = Some(value);
        self
    }

    /// Sets the maximum power (optional)
    #[must_use]
    pub const fn max_power_opt(mut self, value: Option<u32>) -> Self {
        self.activity.max_power = value;
        self
    }

    /// Sets the normalized power
    #[must_use]
    pub const fn normalized_power(mut self, value: u32) -> Self {
        self.activity.normalized_power = Some(value);
        self
    }

    /// Sets the normalized power (optional)
    #[must_use]
    pub const fn normalized_power_opt(mut self, value: Option<u32>) -> Self {
        self.activity.normalized_power = value;
        self
    }

    /// Sets the power zones
    #[must_use]
    pub fn power_zones(mut self, value: Vec<PowerZone>) -> Self {
        self.activity.power_zones = Some(value);
        self
    }

    /// Sets the power zones (optional)
    #[must_use]
    pub fn power_zones_opt(mut self, value: Option<Vec<PowerZone>>) -> Self {
        self.activity.power_zones = value;
        self
    }

    /// Sets the FTP
    #[must_use]
    pub const fn ftp(mut self, value: u32) -> Self {
        self.activity.ftp = Some(value);
        self
    }

    /// Sets the FTP (optional)
    #[must_use]
    pub const fn ftp_opt(mut self, value: Option<u32>) -> Self {
        self.activity.ftp = value;
        self
    }

    /// Sets the average cadence
    #[must_use]
    pub const fn average_cadence(mut self, value: u32) -> Self {
        self.activity.average_cadence = Some(value);
        self
    }

    /// Sets the average cadence (optional)
    #[must_use]
    pub const fn average_cadence_opt(mut self, value: Option<u32>) -> Self {
        self.activity.average_cadence = value;
        self
    }

    /// Sets the maximum cadence
    #[must_use]
    pub const fn max_cadence(mut self, value: u32) -> Self {
        self.activity.max_cadence = Some(value);
        self
    }

    /// Sets the maximum cadence (optional)
    #[must_use]
    pub const fn max_cadence_opt(mut self, value: Option<u32>) -> Self {
        self.activity.max_cadence = value;
        self
    }

    /// Sets the HRV score
    #[must_use]
    pub const fn hrv_score(mut self, value: f64) -> Self {
        self.activity.hrv_score = Some(value);
        self
    }

    /// Sets the HRV score (optional)
    #[must_use]
    pub const fn hrv_score_opt(mut self, value: Option<f64>) -> Self {
        self.activity.hrv_score = value;
        self
    }

    /// Sets the recovery heart rate
    #[must_use]
    pub const fn recovery_heart_rate(mut self, value: u32) -> Self {
        self.activity.recovery_heart_rate = Some(value);
        self
    }

    /// Sets the recovery heart rate (optional)
    #[must_use]
    pub const fn recovery_heart_rate_opt(mut self, value: Option<u32>) -> Self {
        self.activity.recovery_heart_rate = value;
        self
    }

    /// Sets the temperature in Celsius
    #[must_use]
    pub const fn temperature(mut self, value: f32) -> Self {
        self.activity.temperature = Some(value);
        self
    }

    /// Sets the temperature (optional)
    #[must_use]
    pub const fn temperature_opt(mut self, value: Option<f32>) -> Self {
        self.activity.temperature = value;
        self
    }

    /// Sets the humidity percentage
    #[must_use]
    pub const fn humidity(mut self, value: f32) -> Self {
        self.activity.humidity = Some(value);
        self
    }

    /// Sets the humidity (optional)
    #[must_use]
    pub const fn humidity_opt(mut self, value: Option<f32>) -> Self {
        self.activity.humidity = value;
        self
    }

    /// Sets the average altitude in meters
    #[must_use]
    pub const fn average_altitude(mut self, value: f32) -> Self {
        self.activity.average_altitude = Some(value);
        self
    }

    /// Sets the average altitude (optional)
    #[must_use]
    pub const fn average_altitude_opt(mut self, value: Option<f32>) -> Self {
        self.activity.average_altitude = value;
        self
    }

    /// Sets the wind speed in m/s
    #[must_use]
    pub const fn wind_speed(mut self, value: f32) -> Self {
        self.activity.wind_speed = Some(value);
        self
    }

    /// Sets the wind speed (optional)
    #[must_use]
    pub const fn wind_speed_opt(mut self, value: Option<f32>) -> Self {
        self.activity.wind_speed = value;
        self
    }

    /// Sets the ground contact time in milliseconds
    #[must_use]
    pub const fn ground_contact_time(mut self, value: u32) -> Self {
        self.activity.ground_contact_time = Some(value);
        self
    }

    /// Sets the ground contact time (optional)
    #[must_use]
    pub const fn ground_contact_time_opt(mut self, value: Option<u32>) -> Self {
        self.activity.ground_contact_time = value;
        self
    }

    /// Sets the vertical oscillation in centimeters
    #[must_use]
    pub const fn vertical_oscillation(mut self, value: f32) -> Self {
        self.activity.vertical_oscillation = Some(value);
        self
    }

    /// Sets the vertical oscillation (optional)
    #[must_use]
    pub const fn vertical_oscillation_opt(mut self, value: Option<f32>) -> Self {
        self.activity.vertical_oscillation = value;
        self
    }

    /// Sets the stride length in meters
    #[must_use]
    pub const fn stride_length(mut self, value: f32) -> Self {
        self.activity.stride_length = Some(value);
        self
    }

    /// Sets the stride length (optional)
    #[must_use]
    pub const fn stride_length_opt(mut self, value: Option<f32>) -> Self {
        self.activity.stride_length = value;
        self
    }

    /// Sets the running power
    #[must_use]
    pub const fn running_power(mut self, value: u32) -> Self {
        self.activity.running_power = Some(value);
        self
    }

    /// Sets the running power (optional)
    #[must_use]
    pub const fn running_power_opt(mut self, value: Option<u32>) -> Self {
        self.activity.running_power = value;
        self
    }

    /// Sets the breathing rate
    #[must_use]
    pub const fn breathing_rate(mut self, value: u32) -> Self {
        self.activity.breathing_rate = Some(value);
        self
    }

    /// Sets the breathing rate (optional)
    #[must_use]
    pub const fn breathing_rate_opt(mut self, value: Option<u32>) -> Self {
        self.activity.breathing_rate = value;
        self
    }

    /// Sets the `SpO2` percentage
    #[must_use]
    pub const fn spo2(mut self, value: f32) -> Self {
        self.activity.spo2 = Some(value);
        self
    }

    /// Sets the `SpO2` (optional)
    #[must_use]
    pub const fn spo2_opt(mut self, value: Option<f32>) -> Self {
        self.activity.spo2 = value;
        self
    }

    /// Sets the training stress score
    #[must_use]
    pub const fn training_stress_score(mut self, value: f32) -> Self {
        self.activity.training_stress_score = Some(value);
        self
    }

    /// Sets the training stress score (optional)
    #[must_use]
    pub const fn training_stress_score_opt(mut self, value: Option<f32>) -> Self {
        self.activity.training_stress_score = value;
        self
    }

    /// Sets the intensity factor
    #[must_use]
    pub const fn intensity_factor(mut self, value: f32) -> Self {
        self.activity.intensity_factor = Some(value);
        self
    }

    /// Sets the intensity factor (optional)
    #[must_use]
    pub const fn intensity_factor_opt(mut self, value: Option<f32>) -> Self {
        self.activity.intensity_factor = value;
        self
    }

    /// Sets the suffer score
    #[must_use]
    pub const fn suffer_score(mut self, value: u32) -> Self {
        self.activity.suffer_score = Some(value);
        self
    }

    /// Sets the suffer score (optional)
    #[must_use]
    pub const fn suffer_score_opt(mut self, value: Option<u32>) -> Self {
        self.activity.suffer_score = value;
        self
    }

    /// Sets the time series data
    #[must_use]
    pub fn time_series_data(mut self, value: TimeSeriesData) -> Self {
        self.activity.time_series_data = Some(value);
        self
    }

    /// Sets the time series data (optional)
    #[must_use]
    pub fn time_series_data_opt(mut self, value: Option<TimeSeriesData>) -> Self {
        self.activity.time_series_data = value;
        self
    }

    /// Sets the start latitude
    #[must_use]
    pub const fn start_latitude(mut self, value: f64) -> Self {
        self.activity.start_latitude = Some(value);
        self
    }

    /// Sets the start latitude (optional)
    #[must_use]
    pub const fn start_latitude_opt(mut self, value: Option<f64>) -> Self {
        self.activity.start_latitude = value;
        self
    }

    /// Sets the start longitude
    #[must_use]
    pub const fn start_longitude(mut self, value: f64) -> Self {
        self.activity.start_longitude = Some(value);
        self
    }

    /// Sets the start longitude (optional)
    #[must_use]
    pub const fn start_longitude_opt(mut self, value: Option<f64>) -> Self {
        self.activity.start_longitude = value;
        self
    }

    /// Sets the city
    #[must_use]
    pub fn city(mut self, value: String) -> Self {
        self.activity.city = Some(value);
        self
    }

    /// Sets the city (optional)
    #[must_use]
    pub fn city_opt(mut self, value: Option<String>) -> Self {
        self.activity.city = value;
        self
    }

    /// Sets the region
    #[must_use]
    pub fn region(mut self, value: String) -> Self {
        self.activity.region = Some(value);
        self
    }

    /// Sets the region (optional)
    #[must_use]
    pub fn region_opt(mut self, value: Option<String>) -> Self {
        self.activity.region = value;
        self
    }

    /// Sets the country
    #[must_use]
    pub fn country(mut self, value: String) -> Self {
        self.activity.country = Some(value);
        self
    }

    /// Sets the country (optional)
    #[must_use]
    pub fn country_opt(mut self, value: Option<String>) -> Self {
        self.activity.country = value;
        self
    }

    /// Sets the trail name
    #[must_use]
    pub fn trail_name(mut self, value: String) -> Self {
        self.activity.trail_name = Some(value);
        self
    }

    /// Sets the trail name (optional)
    #[must_use]
    pub fn trail_name_opt(mut self, value: Option<String>) -> Self {
        self.activity.trail_name = value;
        self
    }

    /// Sets the workout type
    #[must_use]
    pub const fn workout_type(mut self, value: u32) -> Self {
        self.activity.workout_type = Some(value);
        self
    }

    /// Sets the workout type (optional)
    #[must_use]
    pub const fn workout_type_opt(mut self, value: Option<u32>) -> Self {
        self.activity.workout_type = value;
        self
    }

    /// Sets the sport type detail
    #[must_use]
    pub fn sport_type_detail(mut self, value: String) -> Self {
        self.activity.sport_type_detail = Some(value);
        self
    }

    /// Sets the sport type detail (optional)
    #[must_use]
    pub fn sport_type_detail_opt(mut self, value: Option<String>) -> Self {
        self.activity.sport_type_detail = value;
        self
    }

    /// Sets the segment efforts
    #[must_use]
    pub fn segment_efforts(mut self, value: Vec<SegmentEffort>) -> Self {
        self.activity.segment_efforts = Some(value);
        self
    }

    /// Sets the segment efforts (optional)
    #[must_use]
    pub fn segment_efforts_opt(mut self, value: Option<Vec<SegmentEffort>>) -> Self {
        self.activity.segment_efforts = value;
        self
    }

    /// Builds the Activity instance
    #[must_use]
    pub fn build(self) -> Activity {
        self.activity
    }
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sleep_score: Option<f32>,
    /// Sleep stages breakdown
    pub stages: Vec<SleepStage>,
    /// Heart rate variability during sleep
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hrv_during_sleep: Option<f64>,
    /// Average respiratory rate during sleep
    #[serde(skip_serializing_if = "Option::is_none")]
    pub respiratory_rate: Option<f32>,
    /// Temperature variation during sleep
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature_variation: Option<f32>,
    /// Number of times awakened
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wake_count: Option<u32>,
    /// Time to fall asleep (minutes)
    #[serde(skip_serializing_if = "Option::is_none")]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery_score: Option<f32>,
    /// Readiness score for training (0-100)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readiness_score: Option<f32>,
    /// HRV status or trend
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hrv_status: Option<String>,
    /// Sleep contribution to recovery (0-100)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sleep_score: Option<f32>,
    /// Stress level indicator (0-100, higher = more stress)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stress_level: Option<f32>,
    /// Current training load
    #[serde(skip_serializing_if = "Option::is_none")]
    pub training_load: Option<f32>,
    /// Resting heart rate for the day
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resting_heart_rate: Option<u32>,
    /// Body temperature deviation from baseline
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_temperature: Option<f32>,
    /// Respiratory rate while resting
    #[serde(skip_serializing_if = "Option::is_none")]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
    /// Body fat percentage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_fat_percentage: Option<f32>,
    /// Muscle mass in kilograms
    #[serde(skip_serializing_if = "Option::is_none")]
    pub muscle_mass: Option<f64>,
    /// Bone mass in kilograms
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bone_mass: Option<f64>,
    /// Body water percentage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_water_percentage: Option<f32>,
    /// Basal metabolic rate (calories/day)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bmr: Option<u32>,
    /// Blood pressure (systolic, diastolic)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blood_pressure: Option<(u32, u32)>,
    /// Blood glucose level (mg/dL)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blood_glucose: Option<f32>,
    /// VO2 max estimate (ml/kg/min)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vo2_max: Option<f32>,
    /// Provider of this health data
    pub provider: String,
}

/// Nutrition log entry for tracking food intake
///
/// Represents daily or per-meal nutrition data from wearable integrations
/// like `MyFitnessPal` (via Terra) or other nutrition tracking apps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NutritionLog {
    /// Unique identifier for this nutrition log entry
    pub id: String,
    /// Date of the nutrition log
    pub date: DateTime<Utc>,
    /// Total calories consumed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_calories: Option<f64>,
    /// Total protein in grams
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protein_g: Option<f64>,
    /// Total carbohydrates in grams
    #[serde(skip_serializing_if = "Option::is_none")]
    pub carbohydrates_g: Option<f64>,
    /// Total fat in grams
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fat_g: Option<f64>,
    /// Fiber in grams
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fiber_g: Option<f64>,
    /// Sugar in grams
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sugar_g: Option<f64>,
    /// Sodium in mg
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sodium_mg: Option<f64>,
    /// Water intake in mL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub water_ml: Option<f64>,
    /// Individual meals/entries
    pub meals: Vec<MealEntry>,
    /// Provider of this nutrition data
    pub provider: String,
}

/// Individual meal entry within a nutrition log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MealEntry {
    /// Meal name/type (breakfast, lunch, dinner, snack)
    pub meal_type: MealType,
    /// Timestamp when meal was logged
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<DateTime<Utc>>,
    /// Meal description or name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Calories for this meal
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calories: Option<f64>,
    /// Protein in grams
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protein_g: Option<f64>,
    /// Carbohydrates in grams
    #[serde(skip_serializing_if = "Option::is_none")]
    pub carbohydrates_g: Option<f64>,
    /// Fat in grams
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fat_g: Option<f64>,
    /// Individual food items (if available)
    pub food_items: Vec<FoodItem>,
}

/// Type of meal
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum MealType {
    /// Breakfast meal
    Breakfast,
    /// Lunch meal
    Lunch,
    /// Dinner meal
    Dinner,
    /// Snack between meals
    Snack,
    /// Unspecified or other meal type
    Other,
}

impl MealType {
    /// Parse meal type from string
    #[must_use]
    pub fn from_str_lossy(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "breakfast" => Self::Breakfast,
            "lunch" => Self::Lunch,
            "dinner" => Self::Dinner,
            "snack" => Self::Snack,
            _ => Self::Other,
        }
    }
}

/// Individual food item within a meal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoodItem {
    /// Food name
    pub name: String,
    /// Brand name (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brand: Option<String>,
    /// Serving size amount
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serving_size: Option<f64>,
    /// Serving unit (g, oz, cup, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serving_unit: Option<String>,
    /// Number of servings consumed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub servings: Option<f64>,
    /// Calories per serving
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calories: Option<f64>,
    /// Protein per serving (grams)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protein_g: Option<f64>,
    /// Carbohydrates per serving (grams)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub carbohydrates_g: Option<f64>,
    /// Fat per serving (grams)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fat_g: Option<f64>,
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
    pub fn from_provider_string(provider_sport: &str, fitness_config: &FitnessConfig) -> Self {
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub firstname: Option<String>,
    /// Last name (may not be public on some providers)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lastname: Option<String>,
    /// `URL` to profile picture/avatar
    #[serde(skip_serializing_if = "Option::is_none")]
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
    /// Whether this user has admin privileges (legacy - use role instead)
    pub is_admin: bool,
    /// User role for permission system (`super_admin`, `admin`, `user`)
    pub role: UserRole,
    /// Admin who approved this user (if approved)
    pub approved_by: Option<Uuid>,
    /// When the user was approved by admin
    pub approved_at: Option<DateTime<Utc>>,
    /// Firebase UID if user authenticated via Firebase (Google, Apple, etc.)
    pub firebase_uid: Option<String>,
    /// Authentication provider: "email", "google.com", "apple.com", "github.com"
    pub auth_provider: String,
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
    pub fitness_level: FitnessLevel,
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
            fitness_level: FitnessLevel::Recreational,
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
    pub fn fitness_level_from_vo2_max(&self) -> FitnessLevel {
        self.vo2_max.map_or(self.fitness_level, |vo2_max| {
            FitnessLevel::from_vo2_max(
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
            role: UserRole::User,             // Default to regular user
            approved_by: None,
            approved_at: None,
            firebase_uid: None, // No Firebase UID for email/password users
            auth_provider: "email".to_owned(), // Default to email provider
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
    ) -> AppResult<Self> {
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
    pub fn decrypt(&self, encryption_key: &[u8]) -> AppResult<DecryptedToken> {
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
