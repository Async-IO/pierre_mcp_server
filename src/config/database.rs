// ABOUTME: Database configuration types for SQLite and PostgreSQL connections
// ABOUTME: Handles connection pools, backups, and SQLx settings
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::constants::{database, defaults, limits};
use crate::errors::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::env;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::path::PathBuf;

/// Type-safe database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DatabaseUrl {
    /// `SQLite` database with file path
    SQLite {
        /// Path to `SQLite` database file
        path: PathBuf,
    },
    /// `PostgreSQL` connection
    PostgreSQL {
        /// `PostgreSQL` connection string
        connection_string: String,
    },
    /// In-memory `SQLite` (for testing)
    Memory,
}

impl DatabaseUrl {
    /// Parse from string with validation
    ///
    /// # Errors
    ///
    /// Returns an error if the database URL format is invalid or unsupported
    pub fn parse_url(s: &str) -> AppResult<Self> {
        if s.starts_with("sqlite:") {
            let path_str = s.strip_prefix("sqlite:").unwrap_or(s);
            if path_str == ":memory:" {
                Ok(Self::Memory)
            } else {
                Ok(Self::SQLite {
                    path: PathBuf::from(path_str),
                })
            }
        } else if s.starts_with("postgresql://") || s.starts_with("postgres://") {
            Ok(Self::PostgreSQL {
                connection_string: s.to_owned(),
            })
        } else {
            // Fallback: treat as SQLite file path
            Ok(Self::SQLite {
                path: PathBuf::from(s),
            })
        }
    }

    /// Convert to connection string
    #[must_use]
    pub fn to_connection_string(&self) -> String {
        match self {
            Self::SQLite { path } => format!("sqlite:{}", path.display()),
            Self::PostgreSQL { connection_string } => connection_string.clone(),
            Self::Memory => "sqlite::memory:".into(),
        }
    }

    /// Check if this is an in-memory database
    #[must_use]
    pub const fn is_memory(&self) -> bool {
        matches!(self, Self::Memory)
    }

    /// Check if this is a `SQLite` database
    #[must_use]
    pub const fn is_sqlite(&self) -> bool {
        matches!(self, Self::SQLite { .. } | Self::Memory)
    }

    /// Check if this is a `PostgreSQL` database
    #[must_use]
    pub const fn is_postgresql(&self) -> bool {
        matches!(self, Self::PostgreSQL { .. })
    }
}

impl Default for DatabaseUrl {
    fn default() -> Self {
        Self::SQLite {
            path: PathBuf::from("./data/users.db"),
        }
    }
}

impl Display for DatabaseUrl {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.to_connection_string())
    }
}

/// Database connection and management configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DatabaseConfig {
    /// Database URL (`SQLite` path or `PostgreSQL` connection string)
    pub url: DatabaseUrl,
    /// Enable database migrations on startup
    pub auto_migrate: bool,
    /// Database backup configuration
    pub backup: BackupConfig,
    /// `PostgreSQL` connection pool configuration
    pub postgres_pool: PostgresPoolConfig,
}

impl DatabaseConfig {
    /// Load database configuration from environment
    ///
    /// # Errors
    ///
    /// Returns an error if database environment variables are invalid
    pub fn from_env() -> AppResult<Self> {
        Ok(Self {
            url: DatabaseUrl::parse_url(
                &env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite::memory:".to_owned()),
            )
            .unwrap_or_default(),
            auto_migrate: env_var_or("AUTO_MIGRATE", "true")
                .parse()
                .map_err(|e| AppError::invalid_input(format!("Invalid AUTO_MIGRATE value: {e}")))?,
            backup: BackupConfig::from_env()?,
            postgres_pool: PostgresPoolConfig::from_env(),
        })
    }
}

/// `PostgreSQL` connection pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgresPoolConfig {
    /// Maximum number of connections in the pool
    pub max_connections: u32,
    /// Minimum number of connections in the pool
    pub min_connections: u32,
    /// Connection acquire timeout in seconds
    pub acquire_timeout_secs: u64,
    /// Number of connection retries on startup
    pub connection_retries: u32,
    /// Initial retry delay in milliseconds (doubles with exponential backoff)
    pub initial_retry_delay_ms: u64,
    /// Maximum retry delay in milliseconds
    pub max_retry_delay_ms: u64,
}

impl Default for PostgresPoolConfig {
    fn default() -> Self {
        // CI environment detection at config load time
        let is_ci = env::var("CI").is_ok();
        Self {
            max_connections: if is_ci { 3 } else { 10 },
            min_connections: if is_ci { 1 } else { 2 },
            acquire_timeout_secs: if is_ci { 20 } else { 30 },
            connection_retries: database::CONNECTION_RETRIES,
            initial_retry_delay_ms: database::INITIAL_RETRY_DELAY_MS,
            max_retry_delay_ms: database::MAX_RETRY_DELAY_MS,
        }
    }
}

impl PostgresPoolConfig {
    /// Load `PostgreSQL` pool configuration from environment (or defaults)
    #[must_use]
    pub fn from_env() -> Self {
        let is_ci = env::var("CI").is_ok();
        Self {
            max_connections: env::var("POSTGRES_MAX_CONNECTIONS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(if is_ci { 3 } else { 10 }),
            min_connections: env::var("POSTGRES_MIN_CONNECTIONS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(if is_ci { 1 } else { 2 }),
            acquire_timeout_secs: env::var("POSTGRES_ACQUIRE_TIMEOUT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(if is_ci { 20 } else { 30 }),
            connection_retries: env::var("POSTGRES_CONNECTION_RETRIES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(database::CONNECTION_RETRIES),
            initial_retry_delay_ms: env::var("POSTGRES_INITIAL_RETRY_DELAY_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(database::INITIAL_RETRY_DELAY_MS),
            max_retry_delay_ms: env::var("POSTGRES_MAX_RETRY_DELAY_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(database::MAX_RETRY_DELAY_MS),
        }
    }
}

/// Configuration for automatic database backups
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BackupConfig {
    /// Enable automatic backups
    pub enabled: bool,
    /// Backup interval in seconds
    pub interval_seconds: u64,
    /// Number of backups to retain
    pub retention_count: u32,
    /// Backup directory path
    pub directory: PathBuf,
}

impl BackupConfig {
    /// Load backup configuration from environment
    ///
    /// # Errors
    ///
    /// Returns an error if backup environment variables are invalid
    pub fn from_env() -> AppResult<Self> {
        Ok(Self {
            enabled: env_var_or("BACKUP_ENABLED", "true").parse().map_err(|e| {
                AppError::invalid_input(format!("Invalid BACKUP_ENABLED value: {e}"))
            })?,
            interval_seconds: env_var_or(
                "BACKUP_INTERVAL",
                &limits::DEFAULT_BACKUP_INTERVAL_SECS.to_string(),
            )
            .parse()
            .map_err(|e| AppError::invalid_input(format!("Invalid BACKUP_INTERVAL value: {e}")))?,
            retention_count: env_var_or(
                "BACKUP_RETENTION",
                &limits::DEFAULT_BACKUP_RETENTION_COUNT.to_string(),
            )
            .parse()
            .map_err(|e| AppError::invalid_input(format!("Invalid BACKUP_RETENTION value: {e}")))?,
            directory: PathBuf::from(env_var_or("BACKUP_DIRECTORY", defaults::DEFAULT_BACKUP_DIR)),
        })
    }
}

/// `SQLx` connection pool configuration for controlling database connections
///
/// These settings apply to both `SQLite` and `PostgreSQL` connection pools.
/// Values of `None` use `SQLx` defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlxConfig {
    /// Maximum time a connection can sit idle before being closed (seconds)
    /// Set via `SQLX_IDLE_TIMEOUT_SECS` environment variable
    /// None = use `SQLx` default (10 minutes for `PostgreSQL`, none for `SQLite`)
    pub idle_timeout_secs: Option<u64>,
    /// Maximum lifetime of a connection before it's closed (seconds)
    /// Set via `SQLX_MAX_LIFETIME_SECS` environment variable
    /// None = use `SQLx` default (30 minutes)
    pub max_lifetime_secs: Option<u64>,
    /// Whether to test connections before acquiring from pool
    /// Set via `SQLX_TEST_BEFORE_ACQUIRE` environment variable
    pub test_before_acquire: bool,
    /// Statement cache capacity per connection
    /// Set via `SQLX_STATEMENT_CACHE_CAPACITY` environment variable
    /// None = use `SQLx` default (100)
    pub statement_cache_capacity: Option<usize>,
}

impl Default for SqlxConfig {
    fn default() -> Self {
        Self {
            idle_timeout_secs: None,        // Use SQLx default
            max_lifetime_secs: None,        // Use SQLx default
            test_before_acquire: true,      // Enable by default for reliability
            statement_cache_capacity: None, // Use SQLx default (100)
        }
    }
}

impl SqlxConfig {
    /// Load from environment variables
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            idle_timeout_secs: env::var("SQLX_IDLE_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.parse().ok()),
            max_lifetime_secs: env::var("SQLX_MAX_LIFETIME_SECS")
                .ok()
                .and_then(|s| s.parse().ok()),
            test_before_acquire: env::var("SQLX_TEST_BEFORE_ACQUIRE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(true),
            statement_cache_capacity: env::var("SQLX_STATEMENT_CACHE_CAPACITY")
                .ok()
                .and_then(|s| s.parse().ok()),
        }
    }
}

/// Get environment variable or default value
fn env_var_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_owned())
}
