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
//! - **Environment**: Server configuration from environment variables
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

// Core configuration modules
/// Environment and server configuration
pub mod environment;
/// Fitness and training configuration parameters
pub mod fitness;
/// Intelligence module configuration and strategies
pub mod intelligence;

// Runtime configuration system (moved from src/configuration/)
/// Configuration parameter catalog and schema definitions
pub mod catalog;
/// User profile configurations and templates
pub mod profiles;
/// Runtime configuration management with session-scoped overrides
pub mod runtime;
/// Configuration validation and safety checks
pub mod validation;
/// VO2 max-based physiological calculations
pub mod vo2_max;

// HTTP routes for configuration management
/// HTTP routes for configuration management
pub mod routes;

/// Admin configuration management with runtime parameter overrides
pub mod admin;

// Re-export main configuration types from environment
pub use environment::{LlmProviderType, ServerConfig};

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

// Re-export VO2 max calculation types
pub use vo2_max::{
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
