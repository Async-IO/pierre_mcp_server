// ABOUTME: Security configuration types for authentication and monitoring
// ABOUTME: Handles JWT auth, security headers, and system monitoring settings
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::config::network::{parse_origins, TlsConfig};
use crate::config::types::Environment;
use crate::constants::system_monitoring;
use crate::errors::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::env;

/// Authentication configuration for JWT tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// JWT expiry time in hours
    pub jwt_expiry_hours: u64,
    /// Enable JWT refresh tokens
    pub enable_refresh_tokens: bool,
    /// Admin token cache TTL in seconds (default: 300 = 5 minutes)
    pub admin_token_cache_ttl_secs: u64,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwt_expiry_hours: 24,
            enable_refresh_tokens: false,
            admin_token_cache_ttl_secs: 300, // 5 minutes
        }
    }
}

impl AuthConfig {
    /// Load authentication configuration from environment
    ///
    /// # Errors
    ///
    /// Returns an error if auth environment variables are invalid
    pub fn from_env() -> AppResult<Self> {
        Ok(Self {
            jwt_expiry_hours: u64::try_from(
                env::var("JWT_EXPIRY_HOURS")
                    .ok()
                    .and_then(|s| s.parse::<i64>().ok())
                    .unwrap_or(24)
                    .max(0),
            )
            .unwrap_or(24),
            enable_refresh_tokens: env_var_or("ENABLE_REFRESH_TOKENS", "false")
                .parse()
                .map_err(|e| {
                    AppError::invalid_input(format!("Invalid ENABLE_REFRESH_TOKENS value: {e}"))
                })?,
            admin_token_cache_ttl_secs: env::var("ADMIN_TOKEN_CACHE_TTL_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(300),
        })
    }
}

/// Security configuration including CORS, rate limiting, and TLS
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecurityConfig {
    /// CORS allowed origins
    pub cors_origins: Vec<String>,
    /// TLS configuration
    pub tls: TlsConfig,
    /// Security headers configuration
    pub headers: SecurityHeadersConfig,
}

impl SecurityConfig {
    /// Load security configuration from environment
    ///
    /// # Errors
    ///
    /// Returns an error if security environment variables are invalid
    pub fn from_env() -> AppResult<Self> {
        Ok(Self {
            cors_origins: parse_origins(&env_var_or("CORS_ORIGINS", "*")),
            tls: TlsConfig::from_env(),
            headers: SecurityHeadersConfig::from_env(),
        })
    }

    /// Validate TLS configuration
    ///
    /// # Errors
    ///
    /// Returns an error if TLS is enabled but certificate or key path is missing
    pub fn validate_tls(&self) -> AppResult<()> {
        if self.tls.enabled && (self.tls.cert_path.is_none() || self.tls.key_path.is_none()) {
            return Err(AppError::invalid_input(
                "TLS is enabled but cert_path or key_path is missing",
            ));
        }
        Ok(())
    }
}

/// Security headers configuration based on environment
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecurityHeadersConfig {
    /// Environment type for security headers (development, production)
    pub environment: Environment,
}

impl SecurityHeadersConfig {
    /// Load security headers configuration from environment
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            environment: Environment::from_str_or_default(&env_var_or(
                "SECURITY_HEADERS_ENV",
                "development",
            )),
        }
    }
}

/// System monitoring configuration for health checks and alerts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Memory warning threshold percentage (0-100)
    pub memory_warning_threshold: f64,
    /// Disk warning threshold percentage (0-100)
    pub disk_warning_threshold: f64,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            memory_warning_threshold: system_monitoring::MEMORY_WARNING_THRESHOLD,
            disk_warning_threshold: system_monitoring::DISK_WARNING_THRESHOLD,
        }
    }
}

impl MonitoringConfig {
    /// Load monitoring configuration from environment
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            memory_warning_threshold: env::var("MONITORING_MEMORY_WARNING_THRESHOLD")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(system_monitoring::MEMORY_WARNING_THRESHOLD),
            disk_warning_threshold: env::var("MONITORING_DISK_WARNING_THRESHOLD")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(system_monitoring::DISK_WARNING_THRESHOLD),
        }
    }
}

/// Get environment variable or default value
fn env_var_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_owned())
}
