//! Configuration module for Pierre MCP Server
//!
//! This module provides centralized configuration management for all components
//! of the Pierre MCP Server, including environment settings, fitness configurations,
//! and intelligence module configurations.

pub mod environment;
pub mod fitness_config;
pub mod intelligence_config;

// Re-export main configuration types
pub use environment::ServerConfig;
pub use fitness_config::FitnessConfig;
pub use intelligence_config::{
    AggressiveStrategy, ConfigError, ConservativeStrategy, DefaultStrategy, IntelligenceConfig,
    IntelligenceStrategy,
};

/// Initialize all configurations
pub fn init_configs() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize global intelligence config
    let _intelligence_config = IntelligenceConfig::global();

    tracing::info!("All configurations initialized successfully");
    Ok(())
}
