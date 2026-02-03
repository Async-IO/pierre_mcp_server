// ABOUTME: Core configuration type definitions for environment and logging settings
// ABOUTME: Contains LogLevel, Environment, and LlmProviderType enums used across config modules
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use serde::{Deserialize, Serialize};
use std::env;
use std::fmt::{Display, Formatter, Result as FmtResult};

/// Strongly typed log level configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// Error level - only critical errors
    Error,
    /// Warning level - potential issues
    Warn,
    /// Info level - normal operational messages (default)
    #[default]
    Info,
    /// Debug level - detailed debugging information
    Debug,
    /// Trace level - very verbose tracing
    Trace,
}

impl LogLevel {
    /// Convert to `tracing::Level`
    #[must_use]
    pub const fn to_tracing_level(&self) -> tracing::Level {
        match self {
            Self::Error => tracing::Level::ERROR,
            Self::Warn => tracing::Level::WARN,
            Self::Info => tracing::Level::INFO,
            Self::Debug => tracing::Level::DEBUG,
            Self::Trace => tracing::Level::TRACE,
        }
    }

    /// Parse from string with fallback
    #[must_use]
    pub fn from_str_or_default(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "error" => Self::Error,
            "warn" => Self::Warn,
            "debug" => Self::Debug,
            "trace" => Self::Trace,
            _ => Self::Info, // Default fallback (including "info")
        }
    }
}

impl Display for LogLevel {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Error => write!(f, "error"),
            Self::Warn => write!(f, "warn"),
            Self::Info => write!(f, "info"),
            Self::Debug => write!(f, "debug"),
            Self::Trace => write!(f, "trace"),
        }
    }
}

/// Environment type for security and other configurations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    /// Development environment (default)
    #[default]
    Development,
    /// Production environment with stricter security
    Production,
    /// Testing environment for automated tests
    Testing,
}

impl Environment {
    /// Parse from string with fallback
    #[must_use]
    pub fn from_str_or_default(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "production" | "prod" => Self::Production,
            "testing" | "test" => Self::Testing,
            _ => Self::Development, // Default fallback (including "development" | "dev")
        }
    }

    /// Check if this is a production environment
    #[must_use]
    pub const fn is_production(&self) -> bool {
        matches!(self, Self::Production)
    }

    /// Check if this is a development environment
    #[must_use]
    pub const fn is_development(&self) -> bool {
        matches!(self, Self::Development)
    }

    /// Check if this is a testing environment
    #[must_use]
    pub const fn is_testing(&self) -> bool {
        matches!(self, Self::Testing)
    }
}

impl Display for Environment {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Development => write!(f, "development"),
            Self::Production => write!(f, "production"),
            Self::Testing => write!(f, "testing"),
        }
    }
}

/// LLM provider selection for chat functionality
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum LlmProviderType {
    /// Groq provider - LPU-accelerated inference for Llama/Mixtral models (default)
    #[default]
    Groq,
    /// Google Gemini provider - full-featured with vision support
    Gemini,
    /// Local LLM provider - `OpenAI`-compatible endpoint (Ollama, vLLM, `LocalAI`)
    Local,
}

impl LlmProviderType {
    /// Environment variable name for LLM provider selection
    pub const ENV_VAR: &'static str = "PIERRE_LLM_PROVIDER";

    /// Parse from string with fallback to default
    #[must_use]
    pub fn from_str_or_default(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "gemini" | "google" => Self::Gemini,
            "local" | "ollama" | "vllm" | "localai" => Self::Local,
            _ => Self::Groq, // Default fallback (including "groq")
        }
    }

    /// Load from environment variable
    #[must_use]
    pub fn from_env() -> Self {
        env::var(Self::ENV_VAR)
            .map(|s| Self::from_str_or_default(&s))
            .unwrap_or_default()
    }
}

impl LlmProviderType {
    /// Environment variable for model/version selection
    pub const MODEL_ENV_VAR: &'static str = "PIERRE_LLM_MODEL";

    /// Environment variable for enabling fallback
    pub const FALLBACK_ENABLED_ENV_VAR: &'static str = "PIERRE_LLM_FALLBACK_ENABLED";

    /// Environment variable for fallback provider selection
    pub const FALLBACK_PROVIDER_ENV_VAR: &'static str = "PIERRE_LLM_FALLBACK_PROVIDER";

    /// Environment variable for fallback model selection
    pub const FALLBACK_MODEL_ENV_VAR: &'static str = "PIERRE_LLM_FALLBACK_MODEL";

    /// Environment variable for fallback wait time in seconds
    pub const FALLBACK_WAIT_SECS_ENV_VAR: &'static str = "PIERRE_LLM_FALLBACK_WAIT_SECS";

    /// Default wait time before attempting fallback (10 seconds, matches Gemini retry)
    pub const DEFAULT_FALLBACK_WAIT_SECS: u64 = 10;

    /// Get model from environment
    ///
    /// Reads `PIERRE_LLM_MODEL` - returns None if not set.
    /// Logs an error when not configured but allows server to continue.
    #[must_use]
    pub fn model_from_env() -> Option<String> {
        match env::var(Self::MODEL_ENV_VAR) {
            Ok(model) if !model.is_empty() => Some(model),
            _ => {
                tracing::error!(
                    "{} environment variable not set. Configure it in .envrc to specify the LLM model.",
                    Self::MODEL_ENV_VAR
                );
                None
            }
        }
    }

    /// Check if fallback is enabled from environment
    ///
    /// Reads `PIERRE_LLM_FALLBACK_ENABLED` - returns true only if set to "true" or "1".
    /// Disabled by default.
    #[must_use]
    pub fn is_fallback_enabled() -> bool {
        env::var(Self::FALLBACK_ENABLED_ENV_VAR)
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(false)
    }

    /// Get fallback provider from environment
    ///
    /// Reads `PIERRE_LLM_FALLBACK_PROVIDER` - returns None if not set.
    /// Must be explicitly configured.
    #[must_use]
    pub fn fallback_provider_from_env() -> Option<Self> {
        env::var(Self::FALLBACK_PROVIDER_ENV_VAR)
            .ok()
            .filter(|s| !s.is_empty())
            .map(|s| Self::from_str_or_default(&s))
    }

    /// Get fallback model from environment
    ///
    /// Reads `PIERRE_LLM_FALLBACK_MODEL` - returns None if not set.
    /// When fallback is triggered, uses this model or the default.
    #[must_use]
    pub fn fallback_model_from_env() -> Option<String> {
        env::var(Self::FALLBACK_MODEL_ENV_VAR)
            .ok()
            .filter(|s| !s.is_empty())
    }

    /// Get fallback wait time in seconds from environment
    ///
    /// Reads `PIERRE_LLM_FALLBACK_WAIT_SECS` - defaults to 10 seconds.
    #[must_use]
    pub fn fallback_wait_secs() -> u64 {
        env::var(Self::FALLBACK_WAIT_SECS_ENV_VAR)
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(Self::DEFAULT_FALLBACK_WAIT_SECS)
    }
}

impl Display for LlmProviderType {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Groq => write!(f, "groq"),
            Self::Gemini => write!(f, "gemini"),
            Self::Local => write!(f, "local"),
        }
    }
}
