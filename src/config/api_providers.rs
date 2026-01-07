// ABOUTME: External API provider configuration types for fitness platforms
// ABOUTME: Handles Strava, Fitbit, Garmin API settings and external service configurations
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::constants::api_provider_limits::{
    garmin, strava, FITBIT_RATE_LIMIT_DAILY, FITBIT_RATE_LIMIT_HOURLY, STRAVA_RATE_LIMIT_15MIN,
    STRAVA_RATE_LIMIT_DAILY,
};
use serde::{Deserialize, Serialize};
use std::env;

/// External API services configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExternalServicesConfig {
    /// Weather service configuration
    pub weather: WeatherServiceConfig,
    /// Geocoding service configuration
    pub geocoding: GeocodingServiceConfig,
    /// Strava API configuration
    pub strava_api: StravaApiConfig,
    /// Fitbit API configuration
    pub fitbit_api: FitbitApiConfig,
    /// Garmin API configuration
    pub garmin_api: GarminApiConfig,
}

impl ExternalServicesConfig {
    /// Load external services configuration from environment
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            weather: WeatherServiceConfig::from_env(),
            geocoding: GeocodingServiceConfig::from_env(),
            strava_api: StravaApiConfig::from_env(),
            fitbit_api: FitbitApiConfig::from_env(),
            garmin_api: GarminApiConfig::from_env(),
        }
    }
}

/// Weather API service configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WeatherServiceConfig {
    /// `OpenWeather` API key
    pub api_key: Option<String>,
    /// Weather service base URL
    pub base_url: String,
    /// Enable weather service
    pub enabled: bool,
}

impl WeatherServiceConfig {
    /// Load weather service configuration from environment
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            api_key: env::var("OPENWEATHER_API_KEY").ok(),
            base_url: env_var_or(
                "OPENWEATHER_BASE_URL",
                "https://api.openweathermap.org/data/2.5",
            ),
            enabled: env_var_or("WEATHER_SERVICE_ENABLED", "true")
                .parse()
                .unwrap_or(true),
        }
    }
}

/// Geocoding API service configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeocodingServiceConfig {
    /// Geocoding service base URL
    pub base_url: String,
    /// Enable geocoding service
    pub enabled: bool,
}

impl GeocodingServiceConfig {
    /// Load geocoding service configuration from environment
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            base_url: env_var_or("GEOCODING_BASE_URL", "https://nominatim.openstreetmap.org"),
            enabled: env_var_or("GEOCODING_SERVICE_ENABLED", "true")
                .parse()
                .unwrap_or(true),
        }
    }
}

/// Strava API configuration for OAuth and data fetching
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StravaApiConfig {
    /// Strava API base URL
    pub base_url: String,
    /// Strava auth URL
    pub auth_url: String,
    /// Strava token URL
    pub token_url: String,
    /// Strava deauthorize URL
    pub deauthorize_url: String,
    /// Default activities per page when fetching
    pub default_activities_per_page: usize,
    /// Maximum activities per API request
    pub max_activities_per_request: usize,
    /// Rate limit for 15-minute window
    pub rate_limit_15min: u32,
    /// Rate limit for daily window
    pub rate_limit_daily: u32,
}

impl StravaApiConfig {
    /// Load Strava API configuration from environment
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            base_url: env_var_or("STRAVA_API_BASE", "https://www.strava.com/api/v3"),
            auth_url: env_var_or("STRAVA_AUTH_URL", "https://www.strava.com/oauth/authorize"),
            token_url: env_var_or("STRAVA_TOKEN_URL", "https://www.strava.com/oauth/token"),
            deauthorize_url: env_var_or(
                "STRAVA_DEAUTHORIZE_URL",
                "https://www.strava.com/oauth/deauthorize",
            ),
            default_activities_per_page: env::var("STRAVA_DEFAULT_ACTIVITIES_PER_PAGE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(strava::DEFAULT_ACTIVITIES_PER_PAGE),
            max_activities_per_request: env::var("STRAVA_MAX_ACTIVITIES_PER_REQUEST")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(strava::MAX_ACTIVITIES_PER_REQUEST),
            rate_limit_15min: env::var("STRAVA_RATE_LIMIT_15MIN")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(STRAVA_RATE_LIMIT_15MIN),
            rate_limit_daily: env::var("STRAVA_RATE_LIMIT_DAILY")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(STRAVA_RATE_LIMIT_DAILY),
        }
    }
}

/// Fitbit API configuration for OAuth and data fetching
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FitbitApiConfig {
    /// Fitbit API base URL
    pub base_url: String,
    /// Fitbit auth URL
    pub auth_url: String,
    /// Fitbit token URL
    pub token_url: String,
    /// Fitbit revoke URL
    pub revoke_url: String,
    /// Rate limit for hourly window
    pub rate_limit_hourly: u32,
    /// Rate limit for daily window
    pub rate_limit_daily: u32,
}

impl FitbitApiConfig {
    /// Load Fitbit API configuration from environment
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            base_url: env_var_or("FITBIT_API_BASE", "https://api.fitbit.com"),
            auth_url: env_var_or("FITBIT_AUTH_URL", "https://www.fitbit.com/oauth2/authorize"),
            token_url: env_var_or("FITBIT_TOKEN_URL", "https://api.fitbit.com/oauth2/token"),
            revoke_url: env_var_or("FITBIT_REVOKE_URL", "https://api.fitbit.com/oauth2/revoke"),
            rate_limit_hourly: env::var("FITBIT_RATE_LIMIT_HOURLY")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(FITBIT_RATE_LIMIT_HOURLY),
            rate_limit_daily: env::var("FITBIT_RATE_LIMIT_DAILY")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(FITBIT_RATE_LIMIT_DAILY),
        }
    }
}

/// Garmin API configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GarminApiConfig {
    /// Garmin API base URL
    pub base_url: String,
    /// Garmin auth URL
    pub auth_url: String,
    /// Garmin token URL
    pub token_url: String,
    /// Garmin revoke URL
    pub revoke_url: String,
    /// Default activities per page when fetching
    pub default_activities_per_page: usize,
    /// Maximum activities per API request
    pub max_activities_per_request: usize,
    /// Recommended maximum requests per hour per user
    pub recommended_max_requests_per_hour: usize,
    /// Recommended minimum interval between login attempts (seconds)
    pub recommended_min_login_interval_secs: u64,
    /// Estimated rate limit block duration (seconds)
    pub estimated_rate_limit_block_duration_secs: u64,
}

impl GarminApiConfig {
    /// Load Garmin API configuration from environment
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            base_url: env_var_or(
                "GARMIN_API_BASE",
                "https://apis.garmin.com/wellness-api/rest",
            ),
            auth_url: env_var_or("GARMIN_AUTH_URL", "https://connect.garmin.com/oauthConfirm"),
            token_url: env_var_or(
                "GARMIN_TOKEN_URL",
                "https://connectapi.garmin.com/oauth-service/oauth/access_token",
            ),
            revoke_url: env_var_or(
                "GARMIN_REVOKE_URL",
                "https://connectapi.garmin.com/oauth-service/oauth/revoke",
            ),
            default_activities_per_page: env::var("GARMIN_DEFAULT_ACTIVITIES_PER_PAGE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(garmin::DEFAULT_ACTIVITIES_PER_PAGE),
            max_activities_per_request: env::var("GARMIN_MAX_ACTIVITIES_PER_REQUEST")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(garmin::MAX_ACTIVITIES_PER_REQUEST),
            recommended_max_requests_per_hour: env::var("GARMIN_MAX_REQUESTS_PER_HOUR")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(garmin::RECOMMENDED_MAX_REQUESTS_PER_HOUR),
            recommended_min_login_interval_secs: env::var("GARMIN_MIN_LOGIN_INTERVAL_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(garmin::RECOMMENDED_MIN_LOGIN_INTERVAL_SECS),
            estimated_rate_limit_block_duration_secs: env::var(
                "GARMIN_RATE_LIMIT_BLOCK_DURATION_SECS",
            )
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(garmin::ESTIMATED_RATE_LIMIT_BLOCK_DURATION_SECS),
        }
    }
}

/// Get environment variable or default value
fn env_var_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_owned())
}
