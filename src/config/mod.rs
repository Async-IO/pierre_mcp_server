// ABOUTME: Configuration management module for centralized server settings and parameters
// ABOUTME: Handles environment configs, fitness parameters, intelligence settings, and runtime options
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
//! Configuration module for Pierre MCP Server
//!
//! This module provides centralized configuration management for all components
//! of the Pierre MCP Server, including:
//!
//! - **Environment**: Server configuration orchestrator
//! - **Types**: Core configuration enums (LogLevel, Environment, LlmProviderType)
//! - **Database**: Database connection and pool configuration
//! - **OAuth**: OAuth provider and Firebase authentication configuration
//! - **API Providers**: External fitness API configurations (Strava, Fitbit, Garmin)
//! - **Network**: HTTP client, SSE, CORS, and TLS configuration
//! - **Cache**: Redis and rate limiting configuration
//! - **Security**: Authentication and monitoring configuration
//! - **Logging**: PII redaction and log sampling configuration
//! - **MCP**: Model Context Protocol server configuration
//! - **Fitness**: Sport types, training zones, and fitness parameters
//! - **Intelligence**: AI analysis strategies and recommendation engines
//! - **Catalog**: Parameter catalog and schema definitions
//! - **Profiles**: User and athlete profile configurations
//! - **Runtime**: Session-scoped configuration overrides
//! - **Validation**: Configuration validation and safety checks
//! - **VO2 Max**: Physiological calculations and training zones
//! - **Routes**: HTTP endpoints for configuration management

use std::error::Error;

use tracing::{debug, info};

// Core configuration type modules (extracted from environment.rs)
/// External API provider configuration (Strava, Fitbit, Garmin APIs)
pub mod api_providers;
/// Cache and rate limiting configuration (Redis, TTLs, rate limits)
pub mod cache;
/// Database configuration (`DatabaseUrl`, pools, backups, `SQLx`)
pub mod database;
/// Goal management configuration
pub mod goal_management;
/// Logging and PII redaction configuration
pub mod logging;
/// MCP protocol and runtime configuration
pub mod mcp;
/// Network configuration (HTTP client, SSE, CORS, TLS, timeouts)
pub mod network;
/// OAuth provider configuration (Strava, Fitbit, Garmin, Firebase)
pub mod oauth;
/// Security configuration (auth, headers, monitoring)
pub mod security;
/// Sleep tool operational parameters (activity limits, trend thresholds)
pub mod sleep_tool_params;
/// Tool selection configuration for per-tenant MCP tool filtering
pub mod tool_selection;
/// Core configuration type definitions (`LogLevel`, `Environment`, `LlmProviderType`)
pub mod types;

// Main orchestrator module
/// Environment and server configuration orchestrator
pub mod environment;

// Existing modules
/// Fitness and training configuration parameters
pub mod fitness;
/// Intelligence module configuration and strategies
pub mod intelligence;

// Runtime configuration system
/// Configuration parameter catalog and schema definitions
pub mod catalog;
/// User profile configurations and templates
pub mod profiles;
/// Runtime configuration management with session-scoped overrides
pub mod runtime;
/// Configuration validation and safety checks
pub mod validation;

// HTTP routes for configuration management
/// HTTP routes for configuration management
pub mod routes;

/// Admin configuration management with runtime parameter overrides
pub mod admin;

// Re-export main configuration types from environment
pub use environment::{LlmProviderType, ServerConfig};

// Re-export core types
pub use types::{Environment, LogLevel};

// Re-export database types
pub use database::{BackupConfig, DatabaseConfig, DatabaseUrl, PostgresPoolConfig, SqlxConfig};

// Re-export OAuth types
pub use oauth::{
    default_provider, get_oauth_config, load_provider_env_config, FirebaseConfig,
    OAuth2ServerConfig, OAuthConfig, OAuthProviderConfig, ProviderEnvConfig,
};

// Re-export API provider types
pub use api_providers::{
    ExternalServicesConfig, FitbitApiConfig, GarminApiConfig, GeocodingServiceConfig,
    StravaApiConfig, WeatherServiceConfig,
};

// Re-export network types
pub use network::{
    CorsConfig, HttpClientConfig, RouteTimeoutConfig, SseBufferStrategy, SseConfig, TlsConfig,
};

// Re-export cache types
pub use cache::{CacheConfig, CacheTtlConfig, RateLimitConfig, RedisConnectionConfig};

// Re-export security types
pub use security::{AuthConfig, MonitoringConfig, SecurityConfig, SecurityHeadersConfig};

// Re-export logging types
pub use logging::LoggingConfig;

// Re-export MCP types
pub use mcp::{AppBehaviorConfig, McpConfig, ProtocolConfig, TokioRuntimeConfig};

// Re-export fitness domain types
pub use goal_management::GoalManagementConfig;
pub use sleep_tool_params::SleepToolParamsConfig;
pub use tool_selection::ToolSelectionConfig;

// Re-export intelligence domain types (moved from top-level)
pub use intelligence::TrainingZonesConfig;

// Re-export fitness configuration types
pub use fitness::{FitnessConfig, WeatherApiConfig};

// Re-export intelligence configuration types and strategies
pub use intelligence::{
    AggressiveStrategy, ConfigError, ConservativeStrategy, DefaultStrategy, IntelligenceConfig,
    IntelligenceStrategy, MacroDistribution, MealTdeeProportionsConfig, MealTimingMacrosConfig,
};

// Re-export catalog types
pub use catalog::{CatalogBuilder, ConfigCatalog, ConfigParameter, ParameterType};

// Re-export profile types
pub use profiles::{ConfigProfile, FitnessLevel, ProfileTemplates, ZoneGranularity};

// Re-export runtime configuration types
pub use runtime::{ConfigAware, ConfigChange, ConfigExport, ConfigValue, RuntimeConfig};

// Re-export validation types
pub use validation::{ConfigValidator, ImpactAnalysis, RiskLevel, ValidationResult};

// Re-export VO2 max calculation types (now in intelligence/)
pub use intelligence::{
    PersonalizedHRZones, PersonalizedPaceZones, PersonalizedPowerZones, SportEfficiency,
    VO2MaxCalculator,
};

// Re-export route handlers
pub use routes::{ConfigurationRoutes, FitnessConfigurationRoutes};

/// Initialize all configurations
///
/// # Errors
///
/// Returns an error if configuration initialization fails
pub fn init_configs() -> Result<(), Box<dyn Error>> {
    // Initialize global intelligence config
    let intelligence_config = IntelligenceConfig::global();

    // Validate configuration is properly loaded by accessing a field
    debug!(
        "Intelligence config initialized successfully (min duration: {}s)",
        intelligence_config
            .activity_analyzer
            .analysis
            .min_duration_seconds
    );

    info!("All configurations initialized successfully");
    Ok(())
}
