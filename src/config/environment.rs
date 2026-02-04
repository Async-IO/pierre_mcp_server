// ABOUTME: Environment configuration management for deployment-specific settings
// ABOUTME: Orchestrates loading of all configuration modules from environment variables
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Environment-based configuration management for production deployment
//!
//! This module serves as the main orchestrator for configuration loading,
//! delegating to specialized sub-modules for each configuration domain.

use crate::config::IntelligenceConfig;
use crate::errors::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::env;
use tracing::{info, warn};

// Re-export types from sub-modules for convenience
// API providers
pub use crate::config::api_providers::{
    ExternalServicesConfig, FitbitApiConfig, GarminApiConfig, GeocodingServiceConfig,
    StravaApiConfig, WeatherServiceConfig,
};
// Cache and rate limiting
pub use crate::config::cache::{
    CacheConfig, CacheTtlConfig, RateLimitConfig, RedisConnectionConfig,
};
// Database
pub use crate::config::database::{
    BackupConfig, DatabaseConfig, DatabaseUrl, PostgresPoolConfig, SqlxConfig,
};
// Goal management
pub use crate::config::goal_management::GoalManagementConfig;
// Logging
pub use crate::config::logging::LoggingConfig;
// MCP
pub use crate::config::mcp::{AppBehaviorConfig, McpConfig, ProtocolConfig, TokioRuntimeConfig};
// Network
pub use crate::config::network::{
    CorsConfig, HttpClientConfig, RouteTimeoutConfig, SseBufferStrategy, SseConfig, TlsConfig,
};
// OAuth
pub use crate::config::oauth::{
    default_provider, get_oauth_config, load_provider_env_config, FirebaseConfig,
    OAuth2ServerConfig, OAuthConfig, OAuthProviderConfig, ProviderEnvConfig,
};
// Security
pub use crate::config::security::{
    AuthConfig, MonitoringConfig, SecurityConfig, SecurityHeadersConfig,
};
// Sleep tool params (operational parameters, distinct from intelligence sleep config)
pub use crate::config::sleep_tool_params::SleepToolParamsConfig;
// Training zones (now in intelligence/)
pub use crate::config::intelligence::TrainingZonesConfig;
// Core types
pub use crate::config::types::{Environment, LlmProviderType, LogLevel};

/// Server configuration for HTTP and MCP protocols
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerConfig {
    /// Server port (handles both MCP and HTTP)
    pub http_port: u16,
    /// OAuth callback port for bridge focus recovery
    pub oauth_callback_port: u16,
    /// Frontend URL for OAuth redirects (development uses separate port)
    pub frontend_url: Option<String>,
    /// Log level
    pub log_level: LogLevel,
    /// Logging and PII redaction configuration
    pub logging: LoggingConfig,
    /// Database configuration
    pub database: DatabaseConfig,
    /// Authentication configuration
    pub auth: AuthConfig,
    /// Firebase authentication configuration for social logins
    pub firebase: FirebaseConfig,
    /// OAuth provider configurations
    pub oauth: OAuthConfig,
    /// `OAuth2` authorization server configuration
    pub oauth2_server: OAuth2ServerConfig,
    /// Security settings
    pub security: SecurityConfig,
    /// External service configuration
    pub external_services: ExternalServicesConfig,
    /// USDA `FoodData` Central API key (optional, for nutrition features)
    pub usda_api_key: Option<String>,
    /// Application behavior settings
    pub app_behavior: AppBehaviorConfig,
    /// HTTP client timeout configuration
    pub http_client: HttpClientConfig,
    /// SSE connection management configuration
    pub sse: SseConfig,
    /// Per-route timeout configuration
    pub route_timeouts: RouteTimeoutConfig,
    /// Server host
    pub host: String,
    /// Base URL for OAuth and external URLs
    pub base_url: String,
    /// MCP server configuration
    pub mcp: McpConfig,
    /// CORS configuration
    pub cors: CorsConfig,
    /// Cache configuration
    pub cache: CacheConfig,
    /// Rate limiting configuration
    pub rate_limiting: RateLimitConfig,
    /// Sleep tool operational parameters (activity limits, trend thresholds)
    pub sleep_tool_params: SleepToolParamsConfig,
    /// Goal management and feasibility configuration
    pub goal_management: GoalManagementConfig,
    /// Training zone percentages configuration
    pub training_zones: TrainingZonesConfig,
    /// Tokio runtime configuration
    pub tokio_runtime: TokioRuntimeConfig,
    /// `SQLx` connection pool configuration
    pub sqlx: SqlxConfig,
    /// System monitoring configuration
    pub monitoring: MonitoringConfig,
}

impl ServerConfig {
    /// Load configuration from environment variables
    ///
    /// # Errors
    ///
    /// Returns an error if environment variables contain invalid values or required configuration is missing
    pub fn from_env() -> AppResult<Self> {
        Self::initialize_environment();

        let config = Self {
            http_port: env::var("HTTP_PORT")
                .or_else(|_| env::var("MCP_PORT"))
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(8081),
            oauth_callback_port: env::var("OAUTH_CALLBACK_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(35535),
            frontend_url: env::var("FRONTEND_URL").ok(),
            log_level: LogLevel::from_str_or_default(
                &env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_owned()),
            ),
            logging: LoggingConfig::from_env(),
            database: DatabaseConfig::from_env()?,
            auth: AuthConfig::from_env()?,
            firebase: FirebaseConfig::from_env(),
            oauth: OAuthConfig::from_env(),
            oauth2_server: OAuth2ServerConfig::from_env(),
            security: SecurityConfig::from_env()?,
            external_services: ExternalServicesConfig::from_env(),
            usda_api_key: env::var("USDA_API_KEY").ok(),
            app_behavior: AppBehaviorConfig::from_env()?,
            http_client: HttpClientConfig::from_env(),
            sse: SseConfig::from_env()?,
            route_timeouts: RouteTimeoutConfig::from_env(),
            host: env::var("HOST").unwrap_or_else(|_| "localhost".to_owned()),
            base_url: env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8081".to_owned()),
            mcp: McpConfig::from_env(),
            cors: CorsConfig::from_env(),
            cache: CacheConfig::from_env(),
            rate_limiting: RateLimitConfig::from_env(),
            sleep_tool_params: SleepToolParamsConfig::from_env(),
            goal_management: GoalManagementConfig::from_env(),
            training_zones: TrainingZonesConfig::from_env(),
            tokio_runtime: TokioRuntimeConfig::from_env(),
            sqlx: SqlxConfig::from_env(),
            monitoring: MonitoringConfig::from_env(),
        };

        config.validate()?;
        info!("Configuration loaded successfully");
        Ok(config)
    }

    /// Initialize environment by loading .env file and logging
    fn initialize_environment() {
        info!("Loading configuration from environment variables");

        // Load .env file if it exists
        if let Err(e) = dotenvy::dotenv() {
            warn!("No .env file found or failed to load: {}", e);
        }
    }

    /// Validate configuration values
    ///
    /// # Errors
    ///
    /// Returns an error if configuration values are invalid or conflicting
    pub fn validate(&self) -> AppResult<()> {
        self.validate_oauth_providers();
        self.validate_oauth2_issuer_url()?;
        self.security.validate_tls()?;
        Ok(())
    }

    /// Validate OAuth provider configurations
    fn validate_oauth_providers(&self) {
        if self.oauth.strava.enabled
            && (self.oauth.strava.client_id.is_none() || self.oauth.strava.client_secret.is_none())
        {
            warn!("Strava OAuth is enabled but missing client_id or client_secret");
        }

        if self.oauth.fitbit.enabled
            && (self.oauth.fitbit.client_id.is_none() || self.oauth.fitbit.client_secret.is_none())
        {
            warn!("Fitbit OAuth is enabled but missing client_id or client_secret");
        }
    }

    /// Validate `OAuth2` issuer URL according to RFC 8414 security requirements
    ///
    /// # Errors
    ///
    /// Returns an error if production issuer URL doesn't use HTTPS
    fn validate_oauth2_issuer_url(&self) -> AppResult<()> {
        // In production, issuer MUST use HTTPS to prevent token theft and MITM attacks
        if self.security.headers.environment.is_production() {
            if !self.oauth2_server.issuer_url.starts_with("https://") {
                return Err(AppError::invalid_input(format!(
                    "OAuth2 issuer URL must use HTTPS in production (RFC 8414 security requirement). Current: {}",
                    self.oauth2_server.issuer_url
                )));
            }
        } else if !self
            .oauth2_server
            .issuer_url
            .starts_with("http://localhost")
            && !self.oauth2_server.issuer_url.starts_with("https://")
        {
            // In development/testing, allow localhost HTTP but warn about non-localhost HTTP
            warn!(
                "OAuth2 issuer URL should use HTTPS or localhost in development: {}",
                self.oauth2_server.issuer_url
            );
        }
        Ok(())
    }

    /// Initialize all configurations including intelligence config
    ///
    /// # Errors
    ///
    /// Returns an error if intelligence configuration cannot be loaded or validated
    pub fn init_all_configs(&self) -> AppResult<()> {
        // Initialize intelligence configuration
        let intelligence_config = IntelligenceConfig::global();

        // Validate intelligence configuration is properly loaded by accessing a field
        info!(
            "Intelligence config initialized successfully (min duration: {}s)",
            intelligence_config
                .activity_analyzer
                .analysis
                .min_duration_seconds
        );

        info!("All configurations initialized successfully");
        Ok(())
    }

    /// Get a summary of the configuration for logging (without secrets)
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "Pierre MCP Server Configuration:\n\
             - Server Port: {}\n\
             - Log Level: {}\n\
             - Database: {}\n\
             - Strava OAuth: {}\n\
             - Strava Redirect URI: {}\n\
             - Fitbit OAuth: {}\n\
             - Weather Service: {}\n\
             - TLS: {}\n\
             - Rate Limiting: {}\n\
             - CI Mode: {}\n\
             - Protocol Version: {}",
            self.http_port,
            self.log_level,
            if self.database.url.is_sqlite() {
                "SQLite"
            } else {
                "PostgreSQL"
            },
            "API-Configured",
            self.oauth
                .strava
                .redirect_uri
                .as_ref()
                .map_or("Not configured", |s| s.as_str()),
            "API-Configured",
            if self.external_services.weather.enabled
                && self.external_services.weather.api_key.is_some()
            {
                "Enabled"
            } else {
                "Disabled"
            },
            if self.security.tls.enabled {
                "Enabled"
            } else {
                "Disabled"
            },
            "Configured",
            self.app_behavior.ci_mode,
            self.app_behavior.protocol.mcp_version
        )
    }

    /// Convenience methods for accessing commonly used values
    /// Get the `OpenWeather` API key if available
    #[must_use]
    pub fn openweather_api_key(&self) -> Option<&str> {
        self.external_services.weather.api_key.as_deref()
    }

    /// Get Strava API configuration
    #[must_use]
    pub const fn strava_api_config(&self) -> &StravaApiConfig {
        &self.external_services.strava_api
    }

    /// Get Fitbit API configuration
    #[must_use]
    pub const fn fitbit_api_config(&self) -> &FitbitApiConfig {
        &self.external_services.fitbit_api
    }

    /// Get Garmin API configuration
    #[must_use]
    pub const fn garmin_api_config(&self) -> &GarminApiConfig {
        &self.external_services.garmin_api
    }

    /// Check if CI mode is enabled
    #[must_use]
    pub const fn is_ci_mode(&self) -> bool {
        self.app_behavior.ci_mode
    }

    /// Get protocol information
    #[must_use]
    pub fn protocol_info(&self) -> (&str, &str, &str) {
        (
            &self.app_behavior.protocol.mcp_version,
            &self.app_behavior.protocol.server_name,
            &self.app_behavior.protocol.server_version,
        )
    }

    /// Get activity fetch limits
    #[must_use]
    pub const fn activity_limits(&self) -> (usize, usize) {
        (
            self.app_behavior.max_activities_fetch,
            self.app_behavior.default_activities_limit,
        )
    }
}
