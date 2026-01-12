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
    /// Google Gemini provider via AI Studio API - full-featured with vision support
    Gemini,
    /// Google Vertex AI provider - GCP-native Gemini access with service account auth
    Vertex,
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
            "vertex" | "vertexai" | "vertex_ai" | "vertex-ai" => Self::Vertex,
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

impl Display for LlmProviderType {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Groq => write!(f, "groq"),
            Self::Gemini => write!(f, "gemini"),
            Self::Vertex => write!(f, "vertex"),
            Self::Local => write!(f, "local"),
        }
    }
}
