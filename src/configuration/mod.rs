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

pub mod catalog;
pub mod profiles;
pub mod runtime;
pub mod validation;
pub mod vo2_max;

pub use catalog::{ConfigCatalog, ConfigCategory, ConfigModule, ConfigParameter};
pub use profiles::{ConfigProfile, FitnessLevel};
pub use runtime::{ConfigAware, ConfigValue, RuntimeConfig};
pub use validation::{ConfigValidator, ValidationResult};
pub use vo2_max::{PersonalizedHRZones, PersonalizedPaceZones, VO2MaxCalculator};

/// Re-export commonly used types
pub mod prelude {
    pub use super::{
        ConfigAware, ConfigProfile, ConfigValue, PersonalizedHRZones, RuntimeConfig,
        VO2MaxCalculator,
    };
}
