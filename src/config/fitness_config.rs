// ABOUTME: Fitness-specific configuration for training zones, thresholds, and sport parameters
// ABOUTME: Manages physiological settings, training zones, and sport-specific configurations
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Fitness-specific configuration for sport types and intelligence parameters

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Main fitness configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FitnessConfig {
    pub sport_types: HashMap<String, String>,
    pub intelligence: IntelligenceConfig,
    pub weather_api: Option<WeatherApiConfig>,
}

/// Intelligence analysis configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IntelligenceConfig {
    pub effort_thresholds: EffortThresholds,
    pub zone_thresholds: ZoneThresholds,
    pub weather_mapping: WeatherMapping,
    pub personal_records: PersonalRecordConfig,
}

/// Effort level thresholds for categorizing workout intensity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffortThresholds {
    pub light_max: f32,
    pub moderate_max: f32,
    pub hard_max: f32,
    // > hard_max = very_high
}

/// Heart rate zone thresholds (as percentage of max HR)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneThresholds {
    pub recovery_max: f32,
    pub endurance_max: f32,
    pub tempo_max: f32,
    pub threshold_max: f32,
    // > threshold_max = vo2max
}

/// Weather detection and mapping configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherMapping {
    pub rain_keywords: Vec<String>,
    pub snow_keywords: Vec<String>,
    pub wind_threshold: f32,
}

/// Personal record detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalRecordConfig {
    pub pace_improvement_threshold: f32,
    pub distance_pr_types: Vec<String>,
    pub time_pr_types: Vec<String>,
}

/// Weather API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherApiConfig {
    pub provider: String,
    pub enabled: bool,
    pub cache_duration_hours: u64,
    pub request_timeout_seconds: u64,
    pub rate_limit_requests_per_minute: u64,
}

impl FitnessConfig {
    /// Load fitness configuration from file or use defaults
    ///
    /// # Errors
    ///
    /// Returns an error if the specified config file exists but cannot be read or parsed
    pub fn load(path: Option<String>) -> Result<Self> {
        let mut config = if let Some(config_path) = path {
            if Path::new(&config_path).exists() {
                Self::load_from_file(&config_path)?
            } else {
                Self::default()
            }
        } else if let Ok(env_path) = std::env::var("FITNESS_CONFIG_PATH") {
            if Path::new(&env_path).exists() {
                Self::load_from_file(&env_path)?
            } else {
                Self::default()
            }
        } else if Path::new("fitness_config.toml").exists() {
            Self::load_from_file("fitness_config.toml")?
        } else {
            Self::default()
        };

        // Apply environment variable overrides
        Self::apply_environment_overrides(&mut config);
        Ok(config)
    }

    /// Load configuration from a specific file
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or contains invalid TOML syntax
    pub fn load_from_file(path: &str) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read fitness config file: {path}"))?;

        let config: Self = toml::from_str(&content)
            .with_context(|| format!("Failed to parse fitness config file: {path}"))?;

        Ok(config)
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
        file_path: Option<String>,
    ) -> Result<Self> {
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

        // Fall back to file-based loading with environment overrides
        Self::load(file_path)
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
            enabled: true,
            cache_duration_hours: 24,
            request_timeout_seconds: 10,
            rate_limit_requests_per_minute: crate::constants::time::MINUTE_SECONDS as u64,
        }
    }
}
