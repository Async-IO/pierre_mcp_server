// ABOUTME: Configuration module organizing all configuration-related components
// ABOUTME: Exports configuration management, validation, and runtime config handling
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! Configuration exposure system for dynamic constant management
//!
//! This module provides a runtime configuration system that allows
//! MCP and A2A clients to discover, read, and modify physiological
//! constants and system parameters at runtime.

/// Configuration catalog for discovering available parameters
pub mod catalog;
/// Configuration profiles for different fitness levels
pub mod profiles;
/// Runtime configuration value management
pub mod runtime;
/// Configuration validation and constraint checking
pub mod validation;
/// VO2max calculation and personalized zone generation
pub mod vo2_max;

pub use catalog::{ConfigCatalog, ConfigCategory, ConfigModule, ConfigParameter};
pub use profiles::{ConfigProfile, FitnessLevel};
pub use runtime::{ConfigAware, ConfigValue, RuntimeConfig};
/// Re-export validation types
pub use validation::{ConfigValidator, ValidationResult};
/// Re-export VO2 max calculation types
pub use vo2_max::{PersonalizedHRZones, PersonalizedPaceZones, VO2MaxCalculator};

/// Re-export commonly used types
pub mod prelude {
    pub use super::{
        ConfigAware, ConfigProfile, ConfigValue, PersonalizedHRZones, RuntimeConfig,
        VO2MaxCalculator,
    };
}
