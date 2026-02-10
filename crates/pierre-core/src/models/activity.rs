// ABOUTME: Fitness activity models including Activity, ActivityBuilder, and related types
// ABOUTME: Heart rate zones, power zones, time series data, and segment efforts
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::SportType;

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
