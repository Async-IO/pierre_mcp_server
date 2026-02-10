// ABOUTME: Configuration error types for intelligence module validation
// ABOUTME: Defines error variants for invalid ranges, missing fields, and validation failures
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Configuration error types for intelligence module validation.

use std::env;
use thiserror::Error;

/// Configuration-related errors
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Value outside acceptable range (e.g., percentage not between 0-100)
    #[error("Invalid range: {0}")]
    InvalidRange(&'static str),

    /// Required configuration field is missing
    #[error("Missing required field: {0}")]
    MissingField(&'static str),

    /// Environment variable access or parse error
    #[error("Environment variable error: {0}")]
    EnvVar(#[from] env::VarError),

    /// Failed to parse configuration value
    #[error("Parse error: {0}")]
    Parse(String),

    /// Weights don't sum to required total (e.g., not 100%)
    #[error("Invalid weights: {0}")]
    InvalidWeights(&'static str),

    /// Numeric value outside valid range for parameter
    #[error("Value out of range: {0}")]
    ValueOutOfRange(&'static str),
}
