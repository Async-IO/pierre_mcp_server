// ABOUTME: Fitness-specific configuration for training zones, thresholds, and sport parameters
// ABOUTME: Manages physiological settings, training zones, and sport-specific configurations
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! Fitness-specific configuration for sport types and intelligence parameters

use crate::errors::AppResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Main fitness configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FitnessConfig {
    /// Map of sport type IDs to display names
    pub sport_types: HashMap<String, String>,
    /// Intelligence analysis configuration
    pub intelligence: IntelligenceConfig,
    /// Optional weather API configuration
    pub weather_api: Option<WeatherApiConfig>,
}

/// Intelligence analysis configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IntelligenceConfig {
    /// Effort level threshold values
    pub effort_thresholds: EffortThresholds,
    /// Heart rate zone threshold percentages
    pub zone_thresholds: ZoneThresholds,
    /// Weather detection keyword mappings
    pub weather_mapping: WeatherMapping,
    /// Personal record detection settings
    pub personal_records: PersonalRecordConfig,
}

/// Effort level thresholds for categorizing workout intensity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffortThresholds {
    /// Maximum heart rate percentage for light effort (0-1.0)
    pub light_max: f32,
    /// Maximum heart rate percentage for moderate effort (0-1.0)
    pub moderate_max: f32,
    /// Maximum heart rate percentage for hard effort (0-1.0, above this is very high)
    pub hard_max: f32,
    // > hard_max = very_high
}

/// Heart rate zone thresholds (as percentage of max HR)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneThresholds {
    /// Maximum percentage for recovery zone (Zone 1)
    pub recovery_max: f32,
    /// Maximum percentage for endurance zone (Zone 2)
    pub endurance_max: f32,
    /// Maximum percentage for tempo zone (Zone 3)
    pub tempo_max: f32,
    /// Maximum percentage for threshold zone (Zone 4, above this is VO2 max)
    pub threshold_max: f32,
    // > threshold_max = vo2max
}

/// Weather detection and mapping configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherMapping {
    /// Keywords for detecting rain in weather descriptions
    pub rain_keywords: Vec<String>,
    /// Keywords for detecting snow in weather descriptions
    pub snow_keywords: Vec<String>,
    /// Wind speed threshold for flagging windy conditions (m/s)
    pub wind_threshold: f32,
}

/// Personal record detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalRecordConfig {
    /// Minimum pace improvement percentage to count as PR (0-1.0)
    pub pace_improvement_threshold: f32,
    /// Distance-based PR types to track (e.g., "5k", "10k", "`half_marathon`")
    pub distance_pr_types: Vec<String>,
    /// Time-based PR types to track (e.g., "1h", "2h")
    pub time_pr_types: Vec<String>,
}

/// Weather API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherApiConfig {
    /// Weather provider name (e.g., "openweathermap")
    pub provider: String,
    /// Base URL for the weather API
    pub base_url: String,
    /// Whether weather API integration is enabled
    pub enabled: bool,
    /// How long to cache weather data (hours)
    pub cache_duration_hours: u64,
    /// Request timeout (seconds)
    pub request_timeout_seconds: u64,
    /// Rate limit for API requests (requests per minute)
    pub rate_limit_requests_per_minute: u64,
}

impl FitnessConfig {
    /// Load fitness configuration from environment variables with built-in defaults
    ///
    /// Cloud-native approach: All configuration via environment variables
    /// for easy deployment to any cloud platform
    ///
    /// # Errors
    ///
    /// Returns an error if environment variable parsing fails
    pub fn load() -> AppResult<Self> {
        let mut config = Self::default();

        // Load sport type mappings from environment variables
        Self::load_sport_types_from_env(&mut config.sport_types);

        // Load intelligence configuration from environment variables
        Self::apply_environment_overrides(&mut config);

        Ok(config)
    }

    /// Load sport type mappings from environment variables
    /// Environment variables follow pattern: `SPORT_TYPE_{PROVIDER_NAME}={internal_name}`
    fn load_sport_types_from_env(sport_types: &mut HashMap<String, String>) {
        // Standard sport type mappings from environment
        Self::load_env_sport_type(sport_types, "RUN", "run");
        Self::load_env_sport_type(sport_types, "RIDE", "bike_ride");
        Self::load_env_sport_type(sport_types, "SWIM", "swim");
        Self::load_env_sport_type(sport_types, "WALK", "walk");
        Self::load_env_sport_type(sport_types, "HIKE", "hike");
        Self::load_env_sport_type(sport_types, "VIRTUALRIDE", "virtual_ride");
        Self::load_env_sport_type(sport_types, "VIRTUALRUN", "virtual_run");
        Self::load_env_sport_type(sport_types, "WORKOUT", "workout");
        Self::load_env_sport_type(sport_types, "YOGA", "yoga");
        Self::load_env_sport_type(sport_types, "EBIKERIDE", "ebike_ride");
        Self::load_env_sport_type(sport_types, "MOUNTAINBIKERIDE", "mountain_bike");
        Self::load_env_sport_type(sport_types, "GRAVELRIDE", "gravel_ride");
        // Add more sport types as needed from environment variables
    }

    /// Load a single sport type mapping from environment variable
    fn load_env_sport_type(
        sport_types: &mut HashMap<String, String>,
        sport_key: &str,
        default_value: &str,
    ) {
        let env_key = format!("SPORT_TYPE_{sport_key}");
        // Convert sport_key back to proper case (e.g., "RUN" -> "Run")
        let proper_key = Self::sport_key_to_proper_case(sport_key);

        if let Ok(value) = std::env::var(&env_key) {
            sport_types.insert(proper_key, value);
        } else {
            // Use default mapping if env var not set
            sport_types.insert(proper_key, default_value.to_owned());
        }
    }

    /// Convert uppercase sport key to proper case (e.g., "RUN" -> "Run")
    fn sport_key_to_proper_case(key: &str) -> String {
        match key {
            "RUN" => "Run".to_owned(),
            "RIDE" => "Ride".to_owned(),
            "SWIM" => "Swim".to_owned(),
            "WALK" => "Walk".to_owned(),
            "HIKE" => "Hike".to_owned(),
            "VIRTUALRIDE" => "VirtualRide".to_owned(),
            "VIRTUALRUN" => "VirtualRun".to_owned(),
            "WORKOUT" => "Workout".to_owned(),
            "YOGA" => "Yoga".to_owned(),
            "EBIKERIDE" => "EBikeRide".to_owned(),
            "MOUNTAINBIKERIDE" => "MountainBikeRide".to_owned(),
            "GRAVELRIDE" => "GravelRide".to_owned(),
            _ => key.to_owned(), // fallback to original
        }
    }

    /// Get the internal sport type name for a provider sport type
    #[must_use]
    pub fn map_sport_type(&self, provider_sport: &str) -> Option<&str> {
        self.sport_types
            .get(provider_sport)
            .map(std::string::String::as_str)
    }

    /// Get all configured sport type mappings
    #[must_use]
    pub const fn get_sport_mappings(&self) -> &HashMap<String, String> {
        &self.sport_types
    }

    /// Load fitness configuration with database-first approach
    ///
    /// This method follows a hierarchical loading pattern:
    /// 1. Database (tenant + user-specific configuration) - highest priority
    /// 2. Database (tenant default configuration)
    /// 3. Environment variables (override file/default values)
    /// 4. File configuration
    /// 5. Built-in defaults - lowest priority
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails or configuration parsing fails
    pub async fn load_for_user(
        db_manager: Option<&crate::database::fitness_configurations::FitnessConfigurationManager>,
        tenant_id: Option<&str>,
        user_id: Option<&str>,
        configuration_name: Option<&str>,
    ) -> AppResult<Self> {
        let config_name = configuration_name.unwrap_or("default");

        // Try database first if available
        if let (Some(db), Some(tenant)) = (db_manager, tenant_id) {
            // Try user-specific config first, then tenant default
            let db_config = if let Some(uid) = user_id {
                db.get_user_config(tenant, uid, config_name).await?
            } else {
                db.get_tenant_config(tenant, config_name).await?
            };

            if let Some(mut config) = db_config {
                // Apply environment variable overrides even for database configs
                Self::apply_environment_overrides(&mut config);
                return Ok(config);
            }
        }

        // Fall back to environment-based loading (no file dependencies)
        Self::load()
    }

    /// Apply environment variable overrides to the configuration
    fn apply_environment_overrides(config: &mut Self) {
        Self::apply_effort_threshold_overrides(&mut config.intelligence.effort_thresholds);
        Self::apply_zone_threshold_overrides(&mut config.intelligence.zone_thresholds);
        Self::apply_weather_mapping_overrides(&mut config.intelligence.weather_mapping);
        Self::apply_personal_record_overrides(&mut config.intelligence.personal_records);
        Self::apply_weather_api_overrides(&mut config.weather_api);
    }

    /// Apply environment variable overrides for effort thresholds
    fn apply_effort_threshold_overrides(effort_thresholds: &mut EffortThresholds) {
        Self::parse_env_f32("FITNESS_EFFORT_LIGHT_MAX", &mut effort_thresholds.light_max);
        Self::parse_env_f32(
            "FITNESS_EFFORT_MODERATE_MAX",
            &mut effort_thresholds.moderate_max,
        );
        Self::parse_env_f32("FITNESS_EFFORT_HARD_MAX", &mut effort_thresholds.hard_max);
    }

    /// Apply environment variable overrides for zone thresholds
    fn apply_zone_threshold_overrides(zone_thresholds: &mut ZoneThresholds) {
        Self::parse_env_f32(
            "FITNESS_ZONE_RECOVERY_MAX",
            &mut zone_thresholds.recovery_max,
        );
        Self::parse_env_f32(
            "FITNESS_ZONE_ENDURANCE_MAX",
            &mut zone_thresholds.endurance_max,
        );
        Self::parse_env_f32("FITNESS_ZONE_TEMPO_MAX", &mut zone_thresholds.tempo_max);
        Self::parse_env_f32(
            "FITNESS_ZONE_THRESHOLD_MAX",
            &mut zone_thresholds.threshold_max,
        );
    }

    /// Apply environment variable overrides for weather mapping
    fn apply_weather_mapping_overrides(weather_mapping: &mut WeatherMapping) {
        Self::parse_env_f32(
            "FITNESS_WEATHER_WIND_THRESHOLD",
            &mut weather_mapping.wind_threshold,
        );
    }

    /// Apply environment variable overrides for personal record configuration
    fn apply_personal_record_overrides(personal_records: &mut PersonalRecordConfig) {
        Self::parse_env_f32(
            "FITNESS_PR_PACE_IMPROVEMENT_THRESHOLD",
            &mut personal_records.pace_improvement_threshold,
        );
    }

    /// Apply environment variable overrides for weather API configuration
    fn apply_weather_api_overrides(weather_api: &mut Option<WeatherApiConfig>) {
        if let Some(ref mut api_config) = weather_api {
            Self::parse_env_bool("FITNESS_WEATHER_ENABLED", &mut api_config.enabled);
            Self::parse_env_u64(
                "FITNESS_WEATHER_CACHE_DURATION_HOURS",
                &mut api_config.cache_duration_hours,
            );
            Self::parse_env_u64(
                "FITNESS_WEATHER_REQUEST_TIMEOUT_SECONDS",
                &mut api_config.request_timeout_seconds,
            );
            Self::parse_env_u64(
                "FITNESS_WEATHER_RATE_LIMIT_PER_MINUTE",
                &mut api_config.rate_limit_requests_per_minute,
            );
        }
    }

    /// Parse environment variable as f32 and update target if valid
    fn parse_env_f32(env_var: &str, target: &mut f32) {
        if let Ok(value) = std::env::var(env_var) {
            if let Ok(parsed) = value.parse::<f32>() {
                *target = parsed;
            }
        }
    }

    /// Parse environment variable as bool and update target if valid
    fn parse_env_bool(env_var: &str, target: &mut bool) {
        if let Ok(value) = std::env::var(env_var) {
            if let Ok(parsed) = value.parse::<bool>() {
                *target = parsed;
            }
        }
    }

    /// Parse environment variable as u64 and update target if valid
    fn parse_env_u64(env_var: &str, target: &mut u64) {
        if let Ok(value) = std::env::var(env_var) {
            if let Ok(parsed) = value.parse::<u64>() {
                *target = parsed;
            }
        }
    }
}

impl Default for FitnessConfig {
    fn default() -> Self {
        let mut sport_types = HashMap::new();

        // Standard activities
        sport_types.insert("Run".into(), "run".into());
        sport_types.insert("Ride".into(), "bike_ride".into());
        sport_types.insert("Swim".into(), "swim".into());
        sport_types.insert("Walk".into(), "walk".into());
        sport_types.insert("Hike".into(), "hike".into());

        // Virtual/Indoor activities
        sport_types.insert("VirtualRide".into(), "virtual_ride".into());
        sport_types.insert("VirtualRun".into(), "virtual_run".into());
        sport_types.insert("Workout".into(), "workout".into());
        sport_types.insert("Yoga".into(), "yoga".into());

        // E-bike and specialty cycling
        sport_types.insert("EBikeRide".into(), "ebike_ride".into());
        sport_types.insert("MountainBikeRide".into(), "mountain_bike".into());
        sport_types.insert("GravelRide".into(), "gravel_ride".into());

        // Winter sports
        sport_types.insert("CrossCountrySkiing".into(), "cross_country_skiing".into());
        sport_types.insert("AlpineSkiing".into(), "alpine_skiing".into());
        sport_types.insert("Snowboarding".into(), "snowboarding".into());
        sport_types.insert("Snowshoe".into(), "snowshoe".into());
        sport_types.insert("IceSkate".into(), "ice_skating".into());
        sport_types.insert("BackcountrySki".into(), "backcountry_skiing".into());

        // Water sports
        sport_types.insert("Kayaking".into(), "kayaking".into());
        sport_types.insert("Canoeing".into(), "canoeing".into());
        sport_types.insert("Rowing".into(), "rowing".into());
        sport_types.insert("StandUpPaddling".into(), "paddleboarding".into());
        sport_types.insert("Surfing".into(), "surfing".into());
        sport_types.insert("Kitesurf".into(), "kitesurfing".into());

        // Strength and fitness
        sport_types.insert("WeightTraining".into(), "strength_training".into());
        sport_types.insert("Crossfit".into(), "crossfit".into());
        sport_types.insert("Pilates".into(), "pilates".into());

        // Climbing and adventure
        sport_types.insert("RockClimbing".into(), "rock_climbing".into());
        sport_types.insert("TrailRunning".into(), "trail_running".into());

        // Team and racquet sports
        sport_types.insert("Soccer".into(), "soccer".into());
        sport_types.insert("Basketball".into(), "basketball".into());
        sport_types.insert("Tennis".into(), "tennis".into());
        sport_types.insert("Golf".into(), "golf".into());

        // Alternative transport
        sport_types.insert("Skateboard".into(), "skateboarding".into());
        sport_types.insert("InlineSkate".into(), "inline_skating".into());

        Self {
            sport_types,
            intelligence: IntelligenceConfig::default(),
            weather_api: Some(WeatherApiConfig::default()),
        }
    }
}

impl Default for EffortThresholds {
    fn default() -> Self {
        Self {
            light_max: 3.0,
            moderate_max: 5.0,
            hard_max: 7.0,
        }
    }
}

impl Default for ZoneThresholds {
    fn default() -> Self {
        Self {
            recovery_max: 60.0,
            endurance_max: 70.0,
            tempo_max: 80.0,
            threshold_max: 90.0,
        }
    }
}

impl Default for WeatherMapping {
    fn default() -> Self {
        Self {
            rain_keywords: vec![
                "rain".into(),
                "shower".into(),
                "storm".into(),
                "thunderstorm".into(),
                "drizzle".into(),
            ],
            snow_keywords: vec![
                "snow".into(),
                "blizzard".into(),
                "sleet".into(),
                "flurry".into(),
            ],
            wind_threshold: 15.0,
        }
    }
}

impl Default for PersonalRecordConfig {
    fn default() -> Self {
        Self {
            pace_improvement_threshold: 5.0,
            distance_pr_types: vec![
                "longest_run".into(),
                "longest_ride".into(),
                "longest_ski".into(),
            ],
            time_pr_types: vec![
                "fastest_5k".into(),
                "fastest_10k".into(),
                "fastest_marathon".into(),
            ],
        }
    }
}

impl Default for WeatherApiConfig {
    fn default() -> Self {
        Self {
            provider: "openweathermap".into(),
            base_url: "https://api.openweathermap.org".into(),
            enabled: true,
            cache_duration_hours: 24,
            request_timeout_seconds: 10,
            rate_limit_requests_per_minute: crate::constants::time::MINUTE_SECONDS as u64,
        }
    }
}
