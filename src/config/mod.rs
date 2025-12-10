// ABOUTME: Configuration management module for centralized server settings and parameters
// ABOUTME: Handles environment configs, fitness parameters, intelligence settings, and runtime options
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
//! Configuration module for Pierre MCP Server
//!
//! This module provides centralized configuration management for all components
//! of the Pierre MCP Server, including environment settings, fitness configurations,
//! and intelligence module configurations.

/// Environment and server configuration
pub mod environment;
/// Fitness and training configuration parameters
pub mod fitness_config;
/// Intelligence module configuration and strategies
pub mod intelligence_config;

// Re-export main configuration types
pub use environment::ServerConfig;
pub use fitness_config::{FitnessConfig, WeatherApiConfig};
/// Re-export intelligence configuration types and strategies
pub use intelligence_config::{
    AggressiveStrategy, ConfigError, ConservativeStrategy, DefaultStrategy, IntelligenceConfig,
    IntelligenceStrategy, MacroDistribution, MealTdeeProportionsConfig, MealTimingMacrosConfig,
};

/// Initialize all configurations
///
/// # Errors
///
/// Returns an error if configuration initialization fails
pub fn init_configs() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize global intelligence config
    let intelligence_config = IntelligenceConfig::global();

    // Validate configuration is properly loaded by accessing a field
    tracing::debug!(
        "Intelligence config initialized successfully (min duration: {}s)",
        intelligence_config
            .activity_analyzer
            .analysis
            .min_duration_seconds
    );

    tracing::info!("All configurations initialized successfully");
    Ok(())
}
