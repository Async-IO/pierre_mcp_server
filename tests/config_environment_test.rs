// ABOUTME: Unit tests for config environment functionality
// ABOUTME: Validates config environment behavior, edge cases, and error handling
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use pierre_mcp_server::config::environment::{
    BackupConfig, DatabaseConfig, DatabaseUrl, Environment, LogLevel, OAuthConfig,
    OAuthProviderConfig, SecurityConfig, ServerConfig, SqlxConfig, TokioRuntimeConfig,
};
use std::env;

// Tests for public configuration types

#[test]
fn test_log_level_parsing() {
    assert_eq!(LogLevel::from_str_or_default("error"), LogLevel::Error);
    assert_eq!(LogLevel::from_str_or_default("WARN"), LogLevel::Warn);
    assert_eq!(LogLevel::from_str_or_default("info"), LogLevel::Info);
    assert_eq!(LogLevel::from_str_or_default("Debug"), LogLevel::Debug);
    assert_eq!(LogLevel::from_str_or_default("trace"), LogLevel::Trace);
    assert_eq!(LogLevel::from_str_or_default("invalid"), LogLevel::Info); // Default fallback
}

#[test]
fn test_environment_parsing() {
    assert_eq!(
        Environment::from_str_or_default("production"),
        Environment::Production
    );
    assert_eq!(
        Environment::from_str_or_default("PROD"),
        Environment::Production
    );
    assert_eq!(
        Environment::from_str_or_default("development"),
        Environment::Development
    );
    assert_eq!(
        Environment::from_str_or_default("dev"),
        Environment::Development
    );
    assert_eq!(
        Environment::from_str_or_default("testing"),
        Environment::Testing
    );
    assert_eq!(
        Environment::from_str_or_default("test"),
        Environment::Testing
    );
    assert_eq!(
        Environment::from_str_or_default("invalid"),
        Environment::Development
    ); // Default fallback
}

#[test]
fn test_database_url_parsing() {
    // SQLite URLs
    let sqlite_url = DatabaseUrl::parse_url("sqlite:./test.db").unwrap();
    assert!(sqlite_url.is_sqlite());
    assert!(!sqlite_url.is_postgresql());
    assert_eq!(sqlite_url.to_connection_string(), "sqlite:./test.db");

    // Memory database
    let memory_url = DatabaseUrl::parse_url("sqlite::memory:").unwrap();
    assert!(memory_url.is_memory());
    assert!(memory_url.is_sqlite());

    // PostgreSQL URLs
    let pg_url = DatabaseUrl::parse_url("postgresql://user:pass@localhost/db").unwrap();
    assert!(pg_url.is_postgresql());
    assert!(!pg_url.is_sqlite());

    // Fallback to SQLite
    let fallback_url = DatabaseUrl::parse_url("./some/path.db").unwrap();
    assert!(fallback_url.is_sqlite());
}

/// Helper function to create a valid test `ServerConfig`
fn create_test_server_config() -> ServerConfig {
    ServerConfig {
        http_port: 3000,
        database: DatabaseConfig {
            url: DatabaseUrl::SQLite {
                path: "./test.db".into(),
            },
            backup: BackupConfig {
                directory: "./backups".into(),
                ..Default::default()
            },
            ..Default::default()
        },
        oauth: OAuthConfig {
            strava: OAuthProviderConfig {
                client_id: Some("test_id".into()),
                client_secret: Some("test_secret".into()),
                redirect_uri: Some("http://localhost/callback".into()),
                scopes: vec!["read".into()],
                enabled: true,
            },
            ..Default::default()
        },
        security: SecurityConfig::default(),
        ..Default::default()
    }
}

#[test]
fn test_config_validation() {
    // Test valid configuration with single-port architecture
    let config = create_test_server_config();

    // With single-port architecture, validation should pass
    assert!(config.validate().is_ok());
}

// Tests for TokioRuntimeConfig

#[test]
fn test_tokio_runtime_config_default() {
    let config = TokioRuntimeConfig::default();

    assert!(
        config.worker_threads.is_none(),
        "worker_threads should be None by default"
    );
    assert!(
        config.thread_stack_size.is_none(),
        "thread_stack_size should be None by default"
    );
    assert_eq!(
        config.thread_name, "pierre-worker",
        "thread_name should default to 'pierre-worker'"
    );
    assert!(config.enable_io, "enable_io should be true by default");
    assert!(config.enable_time, "enable_time should be true by default");
}

#[test]
fn test_tokio_runtime_config_from_env() {
    // Test without environment variables set (should use defaults)
    // Note: We can't set env vars in parallel tests safely, so we just test default behavior
    let config = TokioRuntimeConfig::from_env();

    // Should have defaults for unset variables
    assert_eq!(
        config.thread_name,
        env::var("TOKIO_THREAD_NAME").unwrap_or_else(|_| "pierre-worker".to_owned())
    );
    assert!(config.enable_io);
    assert!(config.enable_time);
}

#[test]
fn test_tokio_runtime_config_serialization() {
    let config = TokioRuntimeConfig {
        worker_threads: Some(4),
        thread_stack_size: Some(4 * 1024 * 1024),
        thread_name: "test-worker".to_owned(),
        enable_io: true,
        enable_time: true,
    };

    // Verify serialization works
    let json = serde_json::to_string(&config).expect("serialization should succeed");
    assert!(json.contains("worker_threads"));
    assert!(json.contains('4')); // worker_threads value

    // Verify deserialization works
    let deserialized: TokioRuntimeConfig =
        serde_json::from_str(&json).expect("deserialization should succeed");
    assert_eq!(deserialized.worker_threads, Some(4));
    assert_eq!(deserialized.thread_stack_size, Some(4 * 1024 * 1024));
    assert_eq!(deserialized.thread_name, "test-worker");
}

#[test]
fn test_tokio_runtime_config_in_server_config() {
    // Verify TokioRuntimeConfig is properly included in ServerConfig
    let mut config = create_test_server_config();

    // Modify tokio runtime settings
    config.tokio_runtime.worker_threads = Some(8);
    config.tokio_runtime.thread_stack_size = Some(8 * 1024 * 1024);

    assert_eq!(config.tokio_runtime.worker_threads, Some(8));
    assert_eq!(
        config.tokio_runtime.thread_stack_size,
        Some(8 * 1024 * 1024)
    );
}

#[test]
fn test_tokio_runtime_config_clone_and_debug() {
    let config = TokioRuntimeConfig {
        worker_threads: Some(2),
        thread_stack_size: Some(2 * 1024 * 1024),
        thread_name: "clone-test".to_owned(),
        enable_io: true,
        enable_time: false,
    };

    // Test Clone
    let cloned = config.clone();
    assert_eq!(cloned.worker_threads, config.worker_threads);
    assert_eq!(cloned.thread_name, config.thread_name);

    // Test Debug
    let debug_output = format!("{config:?}");
    assert!(debug_output.contains("TokioRuntimeConfig"));
    assert!(debug_output.contains("worker_threads"));
}

// Tests for SqlxConfig

#[test]
fn test_sqlx_config_default() {
    let config = SqlxConfig::default();

    assert!(
        config.idle_timeout_secs.is_none(),
        "idle_timeout_secs should be None by default"
    );
    assert!(
        config.max_lifetime_secs.is_none(),
        "max_lifetime_secs should be None by default"
    );
    assert!(
        config.test_before_acquire,
        "test_before_acquire should be true by default"
    );
    assert!(
        config.statement_cache_capacity.is_none(),
        "statement_cache_capacity should be None by default"
    );
}

#[test]
fn test_sqlx_config_from_env() {
    // Test without environment variables set (should use defaults)
    let config = SqlxConfig::from_env();

    // Should have defaults for unset variables
    assert!(config.test_before_acquire);
    // Other fields depend on environment
}

#[test]
fn test_sqlx_config_serialization() {
    let config = SqlxConfig {
        idle_timeout_secs: Some(300),
        max_lifetime_secs: Some(1800),
        test_before_acquire: true,
        statement_cache_capacity: Some(200),
    };

    // Verify serialization works
    let json = serde_json::to_string(&config).expect("serialization should succeed");
    assert!(json.contains("idle_timeout_secs"));
    assert!(json.contains("300"));

    // Verify deserialization works
    let deserialized: SqlxConfig =
        serde_json::from_str(&json).expect("deserialization should succeed");
    assert_eq!(deserialized.idle_timeout_secs, Some(300));
    assert_eq!(deserialized.max_lifetime_secs, Some(1800));
    assert!(deserialized.test_before_acquire);
    assert_eq!(deserialized.statement_cache_capacity, Some(200));
}

#[test]
fn test_sqlx_config_in_server_config() {
    // Verify SqlxConfig is properly included in ServerConfig
    let mut config = create_test_server_config();

    // Modify SQLx settings
    config.sqlx.idle_timeout_secs = Some(600);
    config.sqlx.max_lifetime_secs = Some(3600);

    assert_eq!(config.sqlx.idle_timeout_secs, Some(600));
    assert_eq!(config.sqlx.max_lifetime_secs, Some(3600));
}

#[test]
fn test_sqlx_config_clone_and_debug() {
    let config = SqlxConfig {
        idle_timeout_secs: Some(120),
        max_lifetime_secs: Some(600),
        test_before_acquire: false,
        statement_cache_capacity: Some(50),
    };

    // Test Clone
    let cloned = config.clone();
    assert_eq!(cloned.idle_timeout_secs, config.idle_timeout_secs);
    assert_eq!(cloned.test_before_acquire, config.test_before_acquire);

    // Test Debug
    let debug_output = format!("{config:?}");
    assert!(debug_output.contains("SqlxConfig"));
    assert!(debug_output.contains("idle_timeout_secs"));
}
