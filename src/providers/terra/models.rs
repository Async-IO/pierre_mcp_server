// ABOUTME: Terra API data models representing webhook payload structures
// ABOUTME: Maps Terra's standardized JSON schemas to Rust types for deserialization
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! Terra API data models
//!
//! These structures represent the JSON payloads received from Terra webhooks.
//! Terra normalizes data from 150+ providers into consistent schemas.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Terra webhook payload wrapper
///
/// All webhook events from Terra follow this structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerraWebhookPayload {
    /// Event type: "activity", "sleep", "body", "daily", "nutrition", "auth"
    #[serde(rename = "type")]
    pub event_type: String,

    /// Terra user object containing provider and user identification
    pub user: Option<TerraUser>,

    /// Activity data (when type = "activity")
    pub data: Option<Vec<TerraDataWrapper>>,

    /// Status for auth events
    pub status: Option<String>,

    /// Message for auth events
    pub message: Option<String>,

    /// Old user ID for deauth events
    pub old_user: Option<TerraUser>,

    /// Reference ID for correlation
    pub reference_id: Option<String>,

    /// Widget session ID
    pub widget_session_id: Option<String>,
}

/// Terra user identification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerraUser {
    /// Terra's internal user ID
    pub user_id: String,

    /// Provider name (e.g., "GARMIN", "OURA", "FITBIT")
    pub provider: Option<String>,

    /// Last webhook update timestamp
    pub last_webhook_update: Option<String>,

    /// Reference ID for your system's user mapping
    pub reference_id: Option<String>,
}

/// Wrapper for different data types in webhook payloads
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TerraDataWrapper {
    /// Activity data
    Activity(Box<TerraActivity>),
    /// Sleep data
    Sleep(Box<TerraSleep>),
    /// Body metrics
    Body(Box<TerraBody>),
    /// Daily summary
    Daily(Box<TerraDaily>),
    /// Nutrition data
    Nutrition(Box<TerraNutrition>),
}

/// Terra activity/workout data
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TerraActivity {
    /// Activity metadata
    pub metadata: Option<TerraMetadata>,

    /// Active duration in seconds
    pub active_durations_data: Option<TerraActiveDurations>,

    /// Distance data
    pub distance_data: Option<TerraDistanceData>,

    /// Calories data
    pub calories_data: Option<TerraCaloriesData>,

    /// Heart rate data
    pub heart_rate_data: Option<TerraHeartRateData>,

    /// Movement data (cadence, speed)
    pub movement_data: Option<TerraMovementData>,

    /// MET data (metabolic equivalent)
    pub met_data: Option<TerraMetData>,

    /// Position/GPS data
    pub position_data: Option<TerraPositionData>,

    /// Device data
    pub device_data: Option<TerraDeviceData>,

    /// Oxygen data (`SpO2`)
    pub oxygen_data: Option<TerraOxygenData>,

    /// Strain/load data
    pub strain_data: Option<TerraStrainData>,

    /// Power data (cycling/running power)
    pub power_data: Option<TerraPowerData>,

    /// TSS (Training Stress Score) data
    pub tss_data: Option<TerraTssData>,

    /// Lap data for interval activities
    pub lap_data: Option<Vec<TerraLapData>>,
}

/// Common metadata for all Terra data types
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TerraMetadata {
    /// Start time of the activity/session
    pub start_time: Option<DateTime<Utc>>,

    /// End time of the activity/session
    pub end_time: Option<DateTime<Utc>>,

    /// Activity type (e.g., "RUNNING", "CYCLING", "SWIMMING")
    #[serde(rename = "type")]
    pub activity_type: Option<i32>,

    /// Human-readable activity name
    pub name: Option<String>,

    /// Summary ID from provider
    pub summary_id: Option<String>,

    /// City where activity took place
    pub city: Option<String>,

    /// Country where activity took place
    pub country: Option<String>,

    /// Upload type
    pub upload_type: Option<i32>,
}

/// Active duration metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TerraActiveDurations {
    /// Activity duration in seconds
    pub activity_seconds: Option<f64>,

    /// Rest duration in seconds
    pub rest_seconds: Option<f64>,

    /// Low intensity duration in seconds
    pub low_intensity_seconds: Option<f64>,

    /// Moderate intensity duration in seconds
    pub moderate_intensity_seconds: Option<f64>,

    /// High intensity duration in seconds
    pub vigorous_intensity_seconds: Option<f64>,

    /// Inactivity seconds
    pub inactivity_seconds: Option<f64>,
}

/// Distance metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TerraDistanceData {
    /// Total distance in meters
    pub distance_meters: Option<f64>,

    /// Steps taken
    pub steps: Option<i64>,

    /// Floors climbed
    pub floors_climbed: Option<i32>,

    /// Elevation gain in meters
    pub elevation_gain_metres: Option<f64>,

    /// Elevation loss in meters
    pub elevation_loss_metres: Option<f64>,

    /// Max elevation in meters
    pub max_elevation_metres: Option<f64>,

    /// Min elevation in meters
    pub min_elevation_metres: Option<f64>,

    /// Average elevation in meters
    pub avg_elevation_metres: Option<f64>,

    /// Swimming data
    pub swimming: Option<TerraSwimmingData>,
}

/// Swimming-specific metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TerraSwimmingData {
    /// Number of pool lengths
    pub num_laps: Option<i32>,

    /// Number of strokes
    pub num_strokes: Option<i32>,

    /// Pool length in meters
    pub pool_length_metres: Option<f64>,
}

/// Calories metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TerraCaloriesData {
    /// Total calories burned
    pub total_burned_calories: Option<f64>,

    /// Calories burned during activity
    pub activity_burned_calories: Option<f64>,

    /// BMR calories
    pub bmr_burned_calories: Option<f64>,

    /// Net intake calories
    pub net_intake_calories: Option<f64>,
}

/// Heart rate metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TerraHeartRateData {
    /// Average heart rate BPM
    pub avg_hr_bpm: Option<f64>,

    /// Maximum heart rate BPM
    pub max_hr_bpm: Option<f64>,

    /// Minimum heart rate BPM
    pub min_hr_bpm: Option<f64>,

    /// Resting heart rate BPM
    pub resting_hr_bpm: Option<f64>,

    /// Heart rate variability RMSSD
    pub avg_hrv_rmssd: Option<f64>,

    /// Heart rate variability SDNN
    pub avg_hrv_sdnn: Option<f64>,

    /// HR samples for time series
    pub hr_samples: Option<Vec<TerraSample>>,
}

/// Time-series sample data
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TerraSample {
    /// Timestamp of sample
    pub timestamp: Option<DateTime<Utc>>,

    /// Sample value
    pub value: Option<f64>,

    /// Timer start time (for lap/interval data)
    pub timer_start_time: Option<DateTime<Utc>>,

    /// Timer duration seconds
    pub timer_duration_seconds: Option<f64>,
}

/// Movement metrics (speed, cadence)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TerraMovementData {
    /// Average speed in m/s
    pub avg_speed_metres_per_second: Option<f64>,

    /// Max speed in m/s
    pub max_speed_metres_per_second: Option<f64>,

    /// Average cadence (steps/min or RPM)
    pub avg_cadence: Option<f64>,

    /// Max cadence
    pub max_cadence: Option<f64>,

    /// Average pace in seconds per meter
    pub avg_pace_minutes_per_kilometre: Option<f64>,

    /// Best pace
    pub max_pace_minutes_per_kilometre: Option<f64>,

    /// Normalized speed
    pub normalized_speed_metres_per_second: Option<f64>,
}

/// MET (Metabolic Equivalent) data
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TerraMetData {
    /// Average MET value
    pub avg_level: Option<f64>,

    /// Number of low MET minutes
    pub num_low_intensity_minutes: Option<f64>,

    /// Number of moderate MET minutes
    pub num_moderate_intensity_minutes: Option<f64>,

    /// Number of high MET minutes
    pub num_high_intensity_minutes: Option<f64>,
}

/// Position/GPS data
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TerraPositionData {
    /// Start position
    pub start_pos_lat_lng_deg: Option<[f64; 2]>,

    /// End position
    pub end_pos_lat_lng_deg: Option<[f64; 2]>,

    /// Centre position
    pub centre_pos_lat_lng_deg: Option<[f64; 2]>,

    /// GPS samples for route
    pub position_samples: Option<Vec<TerraPositionSample>>,
}

/// GPS position sample
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TerraPositionSample {
    /// Timestamp
    pub timestamp: Option<DateTime<Utc>>,

    /// Latitude
    pub lat: Option<f64>,

    /// Longitude
    pub lng: Option<f64>,

    /// Altitude in meters
    pub altitude_metres: Option<f64>,
}

/// Device information
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TerraDeviceData {
    /// Device name
    pub name: Option<String>,

    /// Device manufacturer
    pub manufacturer: Option<String>,

    /// Serial number
    pub serial_number: Option<String>,

    /// Software version
    pub software_version: Option<String>,

    /// Hardware version
    pub hardware_version: Option<String>,

    /// Activation timestamp
    pub activation_timestamp: Option<DateTime<Utc>>,
}

/// Oxygen saturation data
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TerraOxygenData {
    /// Average `SpO2` percentage
    pub avg_saturation_percentage: Option<f64>,

    /// VO2 max estimate
    pub vo2max_ml_per_min_per_kg: Option<f64>,

    /// `SpO2` samples
    pub saturation_samples: Option<Vec<TerraSample>>,
}

/// Strain/load data
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TerraStrainData {
    /// Strain level (0-21 for WHOOP)
    pub strain_level: Option<f64>,

    /// Activity strain
    pub activity_strain_level: Option<f64>,

    /// Average stress level
    pub avg_stress_level: Option<f64>,

    /// Max stress level
    pub max_stress_level: Option<f64>,
}

/// Power data for cycling/running
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TerraPowerData {
    /// Average power in watts
    pub avg_watts: Option<f64>,

    /// Max power in watts
    pub max_watts: Option<f64>,

    /// Normalized power
    pub normalized_watts: Option<f64>,

    /// Power samples
    pub power_samples: Option<Vec<TerraSample>>,
}

/// Training Stress Score data
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TerraTssData {
    /// Training Stress Score
    pub tss: Option<f64>,

    /// Intensity Factor
    pub intensity_factor: Option<f64>,

    /// Normalized Power / FTP
    pub normalized_power: Option<f64>,

    /// FTP used for calculation
    pub ftp: Option<f64>,
}

/// Lap/interval data
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TerraLapData {
    /// Lap number
    pub lap_index: Option<i32>,

    /// Start time
    pub start_time: Option<DateTime<Utc>>,

    /// End time
    pub end_time: Option<DateTime<Utc>>,

    /// Lap distance in meters
    pub distance_metres: Option<f64>,

    /// Lap duration in seconds
    pub total_seconds: Option<f64>,

    /// Average HR for lap
    pub avg_hr_bpm: Option<f64>,

    /// Max HR for lap
    pub max_hr_bpm: Option<f64>,

    /// Average speed for lap
    pub avg_speed_metres_per_second: Option<f64>,
}

/// Terra sleep data
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TerraSleep {
    /// Sleep metadata
    pub metadata: Option<TerraMetadata>,

    /// Sleep duration data
    pub sleep_durations_data: Option<TerraSleepDurations>,

    /// Heart rate during sleep
    pub heart_rate_data: Option<TerraHeartRateData>,

    /// Respiratory data during sleep
    pub respiration_data: Option<TerraRespirationData>,

    /// Temperature data during sleep
    pub temperature_data: Option<TerraTemperatureData>,

    /// Readiness/recovery score
    pub readiness_data: Option<TerraReadinessData>,
}

/// Sleep duration metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TerraSleepDurations {
    /// Time asleep in seconds
    pub asleep_seconds: Option<f64>,

    /// Time awake in seconds
    pub awake_seconds: Option<f64>,

    /// Time in light sleep
    pub light_sleep_seconds: Option<f64>,

    /// Time in deep sleep (SWS)
    pub deep_sleep_seconds: Option<f64>,

    /// Time in REM sleep
    pub rem_sleep_seconds: Option<f64>,

    /// Time in bed
    pub in_bed_seconds: Option<f64>,

    /// Time to fall asleep
    pub sleep_latency_seconds: Option<f64>,

    /// Number of awakenings
    pub num_awakenings: Option<i32>,

    /// Sleep efficiency percentage
    pub sleep_efficiency: Option<f64>,

    /// Sleep stages timeline
    pub sleep_stages: Option<Vec<TerraSleepStage>>,
}

/// Sleep stage sample
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TerraSleepStage {
    /// Start time of stage
    pub start_time: Option<DateTime<Utc>>,

    /// End time of stage
    pub end_time: Option<DateTime<Utc>>,

    /// Stage type: 0=unknown, 1=awake, 2=light, 3=deep, 4=rem
    pub stage: Option<i32>,
}

/// Respiratory data
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TerraRespirationData {
    /// Average breaths per minute
    pub avg_breaths_per_minute: Option<f64>,

    /// Maximum breaths per minute
    pub max_breaths_per_minute: Option<f64>,

    /// Minimum breaths per minute
    pub min_breaths_per_minute: Option<f64>,

    /// On-demand `SpO2` samples
    pub on_demand_reading: Option<f64>,
}

/// Temperature data
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TerraTemperatureData {
    /// Average temperature in Celsius
    pub avg_temperature_celsius: Option<f64>,

    /// Max temperature
    pub max_temperature_celsius: Option<f64>,

    /// Min temperature
    pub min_temperature_celsius: Option<f64>,

    /// Temperature deviation from baseline
    pub delta_temperature_celsius: Option<f64>,
}

/// Readiness/recovery metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TerraReadinessData {
    /// Readiness score (0-100)
    pub readiness_score: Option<f64>,

    /// Recovery score (0-100)
    pub recovery_score: Option<f64>,

    /// Activity balance score
    pub activity_balance: Option<f64>,

    /// Sleep balance score
    pub sleep_balance: Option<f64>,

    /// HRV balance score
    pub hrv_balance: Option<f64>,

    /// Temperature status
    pub temperature_deviation: Option<f64>,

    /// Resting HR status
    pub resting_hr_status: Option<f64>,
}

/// Terra body metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TerraBody {
    /// Body metadata
    pub metadata: Option<TerraMetadata>,

    /// Measurement data
    pub measurements_data: Option<TerraMeasurementsData>,
}

/// Body measurement metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TerraMeasurementsData {
    /// Weight in kg
    pub weight_kg: Option<f64>,

    /// Height in cm
    pub height_cm: Option<f64>,

    /// BMI
    pub bmi: Option<f64>,

    /// Body fat percentage
    pub body_fat_percentage: Option<f64>,

    /// Muscle mass in kg
    pub muscle_mass_kg: Option<f64>,

    /// Bone mass in kg
    pub bone_mass_kg: Option<f64>,

    /// Body water percentage
    pub body_water_percentage: Option<f64>,

    /// Lean mass in kg
    pub lean_mass_kg: Option<f64>,

    /// Basal metabolic rate
    pub bmr: Option<f64>,

    /// Blood pressure systolic
    pub blood_pressure_systolic: Option<f64>,

    /// Blood pressure diastolic
    pub blood_pressure_diastolic: Option<f64>,

    /// Blood glucose mg/dL
    pub blood_glucose_mg_per_dl: Option<f64>,
}

/// Terra daily summary
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TerraDaily {
    /// Daily metadata
    pub metadata: Option<TerraMetadata>,

    /// Distance data
    pub distance_data: Option<TerraDistanceData>,

    /// Active duration data
    pub active_durations_data: Option<TerraActiveDurations>,

    /// Calories data
    pub calories_data: Option<TerraCaloriesData>,

    /// Heart rate data
    pub heart_rate_data: Option<TerraHeartRateData>,

    /// Stress data
    pub stress_data: Option<TerraStressData>,

    /// Score data (activity score, etc.)
    pub scores: Option<TerraDailyScores>,
}

/// Stress metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TerraStressData {
    /// Average stress level
    pub avg_stress_level: Option<f64>,

    /// Max stress level
    pub max_stress_level: Option<f64>,

    /// Rest stress duration in seconds
    pub rest_stress_duration_seconds: Option<f64>,

    /// Low stress duration in seconds
    pub low_stress_duration_seconds: Option<f64>,

    /// Medium stress duration in seconds
    pub medium_stress_duration_seconds: Option<f64>,

    /// High stress duration in seconds
    pub high_stress_duration_seconds: Option<f64>,

    /// Stress qualifier
    pub stress_qualifier: Option<String>,
}

/// Daily score metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TerraDailyScores {
    /// Activity score (0-100)
    pub activity_score: Option<f64>,

    /// Recovery score (0-100)
    pub recovery_score: Option<f64>,

    /// Sleep score (0-100)
    pub sleep_score: Option<f64>,
}

/// Terra nutrition data
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TerraNutrition {
    /// Nutrition metadata
    pub metadata: Option<TerraMetadata>,

    /// Nutrition summary
    pub summary: Option<TerraNutritionSummary>,

    /// Individual meal entries
    pub meals: Option<Vec<TerraMeal>>,
}

/// Nutrition summary for the day
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TerraNutritionSummary {
    /// Total calories consumed
    pub calories: Option<f64>,

    /// Total protein in grams
    pub protein_g: Option<f64>,

    /// Total carbohydrates in grams
    pub carbohydrates_g: Option<f64>,

    /// Total fiber in grams
    pub fiber_g: Option<f64>,

    /// Total sugar in grams
    pub sugar_g: Option<f64>,

    /// Total fat in grams
    pub fat_g: Option<f64>,

    /// Saturated fat in grams
    pub saturated_fat_g: Option<f64>,

    /// Trans fat in grams
    pub trans_fat_g: Option<f64>,

    /// Cholesterol in mg
    pub cholesterol_mg: Option<f64>,

    /// Sodium in mg
    pub sodium_mg: Option<f64>,

    /// Potassium in mg
    pub potassium_mg: Option<f64>,

    /// Water in mL
    pub water_ml: Option<f64>,

    /// Caffeine in mg
    pub caffeine_mg: Option<f64>,

    /// Alcohol in grams
    pub alcohol_g: Option<f64>,
}

/// Individual meal entry
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TerraMeal {
    /// Meal name/type (breakfast, lunch, dinner, snack)
    pub name: Option<String>,

    /// Meal timestamp
    pub timestamp: Option<DateTime<Utc>>,

    /// Meal ID
    pub id: Option<String>,

    /// Macros for this meal
    pub macros: Option<TerraMealMacros>,

    /// Food items in meal
    pub food_items: Option<Vec<TerraFoodItem>>,
}

/// Macros for a meal
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TerraMealMacros {
    /// Calories
    pub calories: Option<f64>,

    /// Protein in grams
    pub protein_g: Option<f64>,

    /// Carbohydrates in grams
    pub carbohydrates_g: Option<f64>,

    /// Fat in grams
    pub fat_g: Option<f64>,

    /// Fiber in grams
    pub fiber_g: Option<f64>,

    /// Sugar in grams
    pub sugar_g: Option<f64>,
}

/// Individual food item
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TerraFoodItem {
    /// Food name
    pub name: Option<String>,

    /// Brand name
    pub brand: Option<String>,

    /// Serving size
    pub serving_size: Option<f64>,

    /// Serving unit
    pub serving_unit: Option<String>,

    /// Number of servings
    pub servings: Option<f64>,

    /// Calories per serving
    pub calories: Option<f64>,

    /// Macros per serving
    pub macros: Option<TerraMealMacros>,
}

/// Terra athlete/user profile
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TerraAthlete {
    /// Terra user ID
    pub user_id: String,

    /// Provider name (e.g., "GARMIN", "FITBIT")
    pub provider: String,

    /// Reference ID from your system
    pub reference_id: Option<String>,

    /// First name
    pub first_name: Option<String>,

    /// Last name
    pub last_name: Option<String>,

    /// Email
    pub email: Option<String>,

    /// Date of birth
    pub date_of_birth: Option<String>,

    /// Gender
    pub gender: Option<String>,

    /// City
    pub city: Option<String>,

    /// Country
    pub country: Option<String>,
}
