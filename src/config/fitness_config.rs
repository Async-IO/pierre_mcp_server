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
        // Try explicit path first
        if let Some(config_path) = path {
            if Path::new(&config_path).exists() {
                return Self::load_from_file(&config_path);
            }
            // If explicit path doesn't exist, fall back to defaults
            return Ok(Self::default());
        }

        // Try default fitness config file
        if Path::new("fitness_config.toml").exists() {
            return Self::load_from_file("fitness_config.toml");
        }

        // Fall back to embedded defaults
        Ok(Self::default())
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
            rate_limit_requests_per_minute: 60,
        }
    }
}
